//! Port of `LogicSynthesis/sis/util/tmpfile.c`.
//!
//! The C source exposes `util_tempnam` and `util_tmpfile` as allocation and
//! `FILE *` helpers. This native Rust port returns owned `PathBuf` values and a
//! small file wrapper that cleans up its backing path on drop when the platform
//! cannot unlink an open file.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

static UNIQUE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub struct SisTempFile {
    file: File,
    cleanup_path: Option<PathBuf>,
}

impl SisTempFile {
    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    pub fn cleanup_path(&self) -> Option<&Path> {
        self.cleanup_path.as_deref()
    }
}

impl Drop for SisTempFile {
    fn drop(&mut self) {
        if let Some(path) = self.cleanup_path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

pub fn tempnam(dir: Option<&Path>, prefix: Option<&str>) -> PathBuf {
    let directory = select_temp_dir(dir);
    let filename = format!(
        "{}{}a{:05}",
        prefix.unwrap_or(""),
        next_unique_letters(),
        process::id()
    );

    directory.join(filename)
}

pub fn tmpfile() -> io::Result<SisTempFile> {
    for _ in 0..128 {
        let path = tempnam(None, Some("SIS"));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => {
                let cleanup_path = match fs::remove_file(&path) {
                    Ok(()) => None,
                    Err(_) => Some(path),
                };

                return Ok(SisTempFile { file, cleanup_path });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique SIS temporary file name",
    ))
}

fn select_temp_dir(dir: Option<&Path>) -> PathBuf {
    if let Some(path) = env::var_os("TMPDIR").map(PathBuf::from) {
        if check_directory(&path) {
            return path;
        }
    }

    if let Some(path) = dir {
        if check_directory(path) {
            return path.to_path_buf();
        }
    }

    let std_tmp = env::temp_dir();
    if check_directory(&std_tmp) {
        return std_tmp;
    }

    PathBuf::from("/tmp")
}

fn check_directory(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    metadata.is_dir() && is_writable_searchable_directory(path, &metadata)
}

#[cfg(unix)]
fn is_writable_searchable_directory(_path: &Path, metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o222 != 0 && metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_writable_searchable_directory(_path: &Path, metadata: &fs::Metadata) -> bool {
    !metadata.permissions().readonly()
}

fn next_unique_letters() -> String {
    let value = UNIQUE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let first = letter_for(value);
    let second = letter_for(value / 26);
    let third = letter_for(value / (26 * 26));

    format!("{first}{second}{third}")
}

fn letter_for(value: usize) -> char {
    (b'A' + (value % 26) as u8) as char
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn scratch_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("logicfriday1_tmpfile_{name}_{unique}"));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn tempnam_prefers_tmpdir_over_explicit_directory() {
        let _guard = TEST_LOCK.lock().unwrap();
        let original_tmpdir = env::var_os("TMPDIR");
        let tmpdir = scratch_dir("env");
        let explicit = scratch_dir("explicit");

        unsafe {
            env::set_var("TMPDIR", &tmpdir);
        }

        let generated = tempnam(Some(&explicit), Some("SIS"));

        assert!(generated.starts_with(&tmpdir));
        assert!(
            generated
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("SIS")
        );

        restore_tmpdir(original_tmpdir);
        fs::remove_dir_all(tmpdir).unwrap();
        fs::remove_dir_all(explicit).unwrap();
    }

    #[test]
    fn tempnam_uses_explicit_directory_when_tmpdir_is_invalid() {
        let _guard = TEST_LOCK.lock().unwrap();
        let original_tmpdir = env::var_os("TMPDIR");
        let explicit = scratch_dir("fallback");

        unsafe {
            env::set_var("TMPDIR", explicit.join("missing"));
        }

        let generated = tempnam(Some(&explicit), None);

        assert!(generated.starts_with(&explicit));

        restore_tmpdir(original_tmpdir);
        fs::remove_dir_all(explicit).unwrap();
    }

    #[test]
    fn tmpfile_returns_read_write_file() {
        let _guard = TEST_LOCK.lock().unwrap();
        let original_tmpdir = env::var_os("TMPDIR");
        let tmpdir = scratch_dir("file");

        unsafe {
            env::set_var("TMPDIR", &tmpdir);
        }

        let mut temporary = tmpfile().unwrap();
        temporary.file_mut().write_all(b"sis").unwrap();
        temporary.file_mut().seek(SeekFrom::Start(0)).unwrap();

        let mut contents = String::new();
        temporary.file_mut().read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "sis");

        drop(temporary);
        restore_tmpdir(original_tmpdir);
        fs::remove_dir_all(tmpdir).unwrap();
    }

    fn restore_tmpdir(original: Option<std::ffi::OsString>) {
        unsafe {
            match original {
                Some(value) => env::set_var("TMPDIR", value),
                None => env::remove_var("TMPDIR"),
            }
        }
    }
}
