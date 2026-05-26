//! Native file-opening support for the SIS command package.
//!
//! The legacy routine returned `stdin` or `stdout` for `"-"`, searched
//! `open_path` plus the SIS library path for reads, then fell back to tilde
//! expansion before opening the file. This module exposes the same policy as an
//! owned Rust result without adding a per-file C ABI entry point.

use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OpenFileContext
{
    pub open_path: Option<String>,
    pub library_path: Option<String>,
}

impl OpenFileContext
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn with_open_path(mut self, open_path: impl Into<String>) -> Self
    {
        self.open_path = Some(open_path.into());
        self
    }

    pub fn with_library_path(mut self, library_path: impl Into<String>) -> Self
    {
        self.library_path = Some(library_path.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenFileRequest
{
    pub filename: String,
    pub mode: String,
    pub context: OpenFileContext,
    pub silent: bool,
}

impl OpenFileRequest
{
    pub fn new(filename: impl Into<String>, mode: impl Into<String>) -> Self
    {
        Self {
            filename: filename.into(),
            mode: mode.into(),
            context: OpenFileContext::new(),
            silent: false,
        }
    }

    pub fn with_context(mut self, context: OpenFileContext) -> Self
    {
        self.context = context;
        self
    }

    pub fn silent(mut self, silent: bool) -> Self
    {
        self.silent = silent;
        self
    }
}

#[derive(Debug)]
pub struct OpenedFile
{
    pub real_filename: String,
    pub target: OpenFileTarget,
}

impl OpenedFile
{
    pub fn is_stdin(&self) -> bool
    {
        matches!(self.target, OpenFileTarget::Stdin)
    }

    pub fn is_stdout(&self) -> bool
    {
        matches!(self.target, OpenFileTarget::Stdout)
    }

    pub fn is_file(&self) -> bool
    {
        matches!(self.target, OpenFileTarget::File(_))
    }
}

#[derive(Debug)]
pub enum OpenFileTarget
{
    Stdin,
    Stdout,
    File(File),
}

#[derive(Debug)]
pub struct OpenFileError
{
    pub real_filename: String,
    pub source: io::Error,
    pub diagnostic: Option<String>,
}

impl fmt::Display for OpenFileError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!(f, "{}: {}", self.real_filename, self.source)
    }
}

impl Error for OpenFileError
{
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        Some(&self.source)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParsedMode
{
    Read,
    Write,
    Append,
    ReadUpdate,
    WriteUpdate,
    AppendUpdate,
}

impl ParsedMode
{
    fn parse(mode: &str) -> io::Result<Self>
    {
        let mut chars = mode.chars();
        let Some(base) = chars.next() else
        {
            return Err(invalid_mode(mode));
        };

        let mut update = false;
        for suffix in chars
        {
            match suffix
            {
                '+' => update = true,
                'b' => {}
                _ => return Err(invalid_mode(mode)),
            }
        }

        match (base, update)
        {
            ('r', false) => Ok(Self::Read),
            ('r', true) => Ok(Self::ReadUpdate),
            ('w', false) => Ok(Self::Write),
            ('w', true) => Ok(Self::WriteUpdate),
            ('a', false) => Ok(Self::Append),
            ('a', true) => Ok(Self::AppendUpdate),
            _ => Err(invalid_mode(mode)),
        }
    }

    fn open_options(self) -> OpenOptions
    {
        let mut options = OpenOptions::new();
        match self
        {
            Self::Read =>
            {
                options.read(true);
            }
            Self::ReadUpdate =>
            {
                options.read(true).write(true);
            }
            Self::Write =>
            {
                options.write(true).create(true).truncate(true);
            }
            Self::WriteUpdate =>
            {
                options.read(true).write(true).create(true).truncate(true);
            }
            Self::Append =>
            {
                options.append(true).create(true);
            }
            Self::AppendUpdate =>
            {
                options.read(true).append(true).create(true);
            }
        }

        options
    }
}

pub fn open_file(request: OpenFileRequest) -> Result<OpenedFile, OpenFileError>
{
    if request.filename == "-"
    {
        return Ok(open_standard_stream(&request.mode));
    }

    let mode = ParsedMode::parse(&request.mode).map_err(|source| OpenFileError {
        real_filename: request.filename.clone(),
        diagnostic: diagnostic(&request.filename, &source, request.silent),
        source,
    })?;

    let real_filename = resolve_filename(&request.filename, &request.context, request.mode == "r");
    match mode.open_options().open(&real_filename)
    {
        Ok(file) => Ok(OpenedFile {
            real_filename,
            target: OpenFileTarget::File(file),
        }),
        Err(source) =>
        {
            let diagnostic = diagnostic(&real_filename, &source, request.silent);
            Err(OpenFileError {
                real_filename,
                source,
                diagnostic,
            })
        }
    }
}

fn open_standard_stream(mode: &str) -> OpenedFile
{
    if mode == "w"
    {
        OpenedFile {
            real_filename: "stdout".to_owned(),
            target: OpenFileTarget::Stdout,
        }
    }
    else
    {
        OpenedFile {
            real_filename: "stdin".to_owned(),
            target: OpenFileTarget::Stdin,
        }
    }
}

fn resolve_filename(filename: &str, context: &OpenFileContext, search_open_path: bool) -> String
{
    if search_open_path
    {
        if let Some(user_path) = context.open_path.as_deref()
        {
            let search_path = search_path_with_library(user_path, context.library_path.as_deref());
            if let Some(real_filename) = file_search(filename, &search_path)
            {
                return real_filename;
            }
        }
    }

    tilde_expand(filename)
}

fn search_path_with_library(user_path: &str, library_path: Option<&str>) -> String
{
    match library_path
    {
        Some(library_path) => format!("{user_path}:{library_path}"),
        None => user_path.to_owned(),
    }
}

fn file_search(filename: &str, search_path: &str) -> Option<String>
{
    let path = if search_path.is_empty()
    {
        "."
    }
    else
    {
        search_path
    };

    for directory in split_search_path(path)
    {
        let candidate = if directory == "."
        {
            filename.to_owned()
        }
        else
        {
            format!("{directory}/{filename}")
        };
        let expanded = tilde_expand(&candidate);

        if is_readable_file(&expanded)
        {
            return Some(expanded);
        }
    }

    None
}

fn split_search_path(path: &str) -> Vec<&str>
{
    let mut entries = Vec::new();
    let mut start = 0;

    for (index, value) in path.char_indices()
    {
        if value != ':'
        {
            continue;
        }

        if is_windows_drive_colon(path, index)
        {
            continue;
        }

        entries.push(&path[start..index]);
        start = index + value.len_utf8();
    }

    entries.push(&path[start..]);
    entries
}

fn is_windows_drive_colon(path: &str, index: usize) -> bool
{
    if index == 0
    {
        return false;
    }

    let prefix = &path[..index];
    let Some(drive) = prefix.chars().last() else
    {
        return false;
    };

    if !drive.is_ascii_alphabetic()
    {
        return false;
    }

    let before_drive = &prefix[..prefix.len() - drive.len_utf8()];
    if !before_drive.is_empty() && !before_drive.ends_with(':')
    {
        return false;
    }

    path[index + 1..]
        .chars()
        .next()
        .is_some_and(|next| next == '\\' || next == '/')
}

fn is_readable_file(filename: &str) -> bool
{
    fs::metadata(filename).is_ok_and(|metadata| metadata.is_file())
}

fn tilde_expand(filename: &str) -> String
{
    let Some(rest) = filename.strip_prefix('~') else
    {
        return filename.to_owned();
    };

    let (username, suffix) = match rest.find('/')
    {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };

    let directory = if username.is_empty()
    {
        current_user_home()
    }
    else if username == "octtools"
    {
        env_string("OCTTOOLS")
    }
    else
    {
        user_home(username)
    };

    match directory
    {
        Some(directory) => format!("{directory}{suffix}"),
        None => filename.to_owned(),
    }
}

fn current_user_home() -> Option<String>
{
    env_string("HOME")
        .or_else(|| env_string("USERPROFILE"))
        .or_else(|| match (env_string("HOMEDRIVE"), env_string("HOMEPATH"))
        {
            (Some(drive), Some(path)) => Some(format!("{drive}{path}")),
            _ => None,
        })
}

fn env_string(name: &str) -> Option<String>
{
    env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

#[cfg(unix)]
fn user_home(username: &str) -> Option<String>
{
    let passwd = fs::read_to_string("/etc/passwd").ok()?;

    passwd.lines().find_map(|line|
    {
        let mut fields = line.split(':');
        let name = fields.next()?;
        if name != username
        {
            return None;
        }

        fields.nth(4).map(str::to_owned)
    })
}

#[cfg(not(unix))]
fn user_home(_username: &str) -> Option<String>
{
    None
}

fn invalid_mode(mode: &str) -> io::Error
{
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("unsupported fopen mode '{mode}'"),
    )
}

fn diagnostic(filename: &str, source: &io::Error, silent: bool) -> Option<String>
{
    if silent
    {
        None
    }
    else
    {
        Some(format!("{filename}: {source}"))
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::io::{Read, Write};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_dir(name: &str) -> std::path::PathBuf
    {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("logicfriday1_open_file_{name}_{unique}"));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn dash_maps_write_mode_to_stdout()
    {
        let opened = open_file(OpenFileRequest::new("-", "w")).unwrap();

        assert_eq!(opened.real_filename, "stdout");
        assert!(opened.is_stdout());
    }

    #[test]
    fn dash_maps_non_write_modes_to_stdin()
    {
        let opened = open_file(OpenFileRequest::new("-", "r")).unwrap();

        assert_eq!(opened.real_filename, "stdin");
        assert!(opened.is_stdin());
    }

    #[test]
    fn read_mode_searches_open_path_before_fallback()
    {
        let root = temp_dir("search");
        let first = root.join("first");
        let library = root.join("library");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&library).unwrap();
        fs::write(library.join("target.blif"), "library copy").unwrap();

        let context = OpenFileContext::new()
            .with_open_path(first.to_string_lossy())
            .with_library_path(library.to_string_lossy());

        let opened = open_file(OpenFileRequest::new("target.blif", "r").with_context(context))
            .unwrap();

        assert_eq!(
            fs::canonicalize(&opened.real_filename).unwrap(),
            fs::canonicalize(library.join("target.blif")).unwrap()
        );
        assert!(opened.is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn read_mode_preserves_legacy_search_for_explicit_relative_path()
    {
        let root = temp_dir("explicit_relative");
        let library = root.join("library");
        fs::create_dir(&library).unwrap();
        fs::create_dir(library.join("relative")).unwrap();
        fs::write(library.join("relative/name.blif"), "found").unwrap();

        let context = OpenFileContext::new()
            .with_open_path(root.join("missing").to_string_lossy())
            .with_library_path(library.to_string_lossy());

        let opened = open_file(
            OpenFileRequest::new("relative/name.blif", "r").with_context(context),
        )
        .unwrap();

        assert_eq!(
            fs::canonicalize(&opened.real_filename).unwrap(),
            fs::canonicalize(library.join("relative/name.blif")).unwrap()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn write_mode_does_not_search_open_path()
    {
        let root = temp_dir("write");
        let search = root.join("search");
        fs::create_dir(&search).unwrap();
        fs::write(search.join("output.txt"), "existing").unwrap();

        let output = root.join("output.txt");
        let context = OpenFileContext::new().with_open_path(search.to_string_lossy());
        let opened = open_file(
            OpenFileRequest::new(output.to_string_lossy(), "w").with_context(context),
        )
        .unwrap();

        assert_eq!(
            fs::canonicalize(&opened.real_filename).unwrap(),
            fs::canonicalize(&output).unwrap()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn silent_suppresses_failure_diagnostic()
    {
        let root = temp_dir("silent");
        let missing = root.join("missing.blif");

        let error = open_file(OpenFileRequest::new(missing.to_string_lossy(), "r").silent(true))
            .unwrap_err();

        assert_eq!(error.real_filename, missing.to_string_lossy());
        assert_eq!(error.diagnostic, None);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn non_silent_failure_keeps_perror_style_diagnostic()
    {
        let root = temp_dir("diagnostic");
        let missing = root.join("missing.blif");

        let error = open_file(OpenFileRequest::new(missing.to_string_lossy(), "r")).unwrap_err();

        assert_eq!(error.real_filename, missing.to_string_lossy());
        assert!(error
            .diagnostic
            .as_ref()
            .unwrap()
            .starts_with(&format!("{}: ", missing.to_string_lossy())));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tilde_fallback_expands_current_user_home()
    {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_dir("home");
        fs::write(root.join("input.blif"), "home copy").unwrap();

        unsafe
        {
            env::set_var("HOME", &root);
        }

        let mut opened = open_file(OpenFileRequest::new("~/input.blif", "r")).unwrap();
        let mut contents = String::new();
        match &mut opened.target
        {
            OpenFileTarget::File(file) =>
            {
                file.read_to_string(&mut contents).unwrap();
            }
            _ => panic!("expected file target"),
        }

        assert_eq!(contents, "home copy");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn write_mode_truncates_existing_file()
    {
        let root = temp_dir("truncate");
        let target = root.join("output.txt");
        fs::write(&target, "old contents").unwrap();

        let mut opened = open_file(OpenFileRequest::new(target.to_string_lossy(), "w")).unwrap();
        match &mut opened.target
        {
            OpenFileTarget::File(file) =>
            {
                file.write_all(b"new").unwrap();
            }
            _ => panic!("expected file target"),
        }
        drop(opened);

        assert_eq!(fs::read_to_string(&target).unwrap(), "new");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_mode_reports_invalid_input()
    {
        let error = open_file(OpenFileRequest::new("file.blif", "q")).unwrap_err();

        assert_eq!(error.source.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.real_filename, "file.blif");
        assert!(error.diagnostic.unwrap().contains("unsupported fopen mode"));
    }

    #[test]
    fn append_update_mode_reads_and_appends()
    {
        let root = temp_dir("append_update");
        let target = root.join("log.txt");
        fs::write(&target, "old").unwrap();

        let mut opened = open_file(OpenFileRequest::new(target.to_string_lossy(), "a+")).unwrap();
        match &mut opened.target
        {
            OpenFileTarget::File(file) =>
            {
                file.write_all(b"new").unwrap();
            }
            _ => panic!("expected file target"),
        }
        drop(opened);

        assert_eq!(fs::read_to_string(&target).unwrap(), "oldnew");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_path_is_appended_after_user_open_path()
    {
        assert_eq!(
            search_path_with_library("first:second", Some("library")),
            "first:second:library"
        );
    }

    #[test]
    fn search_path_split_preserves_windows_drive_letters()
    {
        assert_eq!(
            split_search_path("C:\\first:D:\\second:relative"),
            ["C:\\first", "D:\\second", "relative"]
        );
    }

    #[test]
    fn empty_search_path_checks_current_directory()
    {
        let root = temp_dir("empty_path");
        let original = env::current_dir().unwrap();
        fs::write(root.join("local.blif"), "local").unwrap();
        env::set_current_dir(&root).unwrap();

        let found = file_search("local.blif", "").unwrap();

        env::set_current_dir(original).unwrap();
        assert_eq!(found, "local.blif");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn non_existing_search_target_falls_back_to_expanded_filename()
    {
        let root = temp_dir("fallback");
        let context = OpenFileContext::new().with_open_path(root.to_string_lossy());

        let request = OpenFileRequest::new("missing.blif", "r").with_context(context);
        let error = open_file(request).unwrap_err();

        assert_eq!(error.real_filename, "missing.blif");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn octtools_tilde_uses_environment()
    {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_dir("octtools");
        fs::write(root.join("library.blif"), "octtools").unwrap();

        unsafe
        {
            env::set_var("OCTTOOLS", &root);
        }

        let opened = open_file(OpenFileRequest::new("~octtools/library.blif", "r")).unwrap();

        assert_eq!(
            fs::canonicalize(&opened.real_filename).unwrap(),
            fs::canonicalize(root.join("library.blif")).unwrap()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unknown_user_tilde_is_left_unchanged()
    {
        assert_eq!(
            tilde_expand("~logicfriday_user_that_should_not_exist/file"),
            "~logicfriday_user_that_should_not_exist/file"
        );
    }

    #[test]
    fn readability_check_rejects_directories()
    {
        let root = temp_dir("directory");

        assert!(!is_readable_file(root.to_string_lossy().as_ref()));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mode_with_binary_suffix_is_accepted()
    {
        let root = temp_dir("binary");
        let target = root.join("input.bin");
        fs::write(&target, "binary").unwrap();

        let opened = open_file(OpenFileRequest::new(target.to_string_lossy(), "rb")).unwrap();

        assert!(opened.is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn binary_read_mode_does_not_search_open_path()
    {
        let root = temp_dir("binary_search");
        let search = root.join("search");
        fs::create_dir(&search).unwrap();
        fs::write(search.join("input.bin"), "binary").unwrap();

        let context = OpenFileContext::new().with_open_path(search.to_string_lossy());
        let error =
            open_file(OpenFileRequest::new("input.bin", "rb").with_context(context)).unwrap_err();

        assert_eq!(error.real_filename, "input.bin");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mode_with_update_before_binary_suffix_is_accepted()
    {
        let root = temp_dir("binary_update");
        let target = root.join("input.bin");
        fs::write(&target, "binary").unwrap();

        let opened = open_file(OpenFileRequest::new(target.to_string_lossy(), "r+b")).unwrap();

        assert!(opened.is_file());

        fs::remove_dir_all(root).unwrap();
    }
}
