//! Port of `LogicSynthesis/sis/util/pathsearch.c`.
//!
//! The C source returns newly allocated strings and searches colon-separated
//! paths, expanding `~` before probing each candidate. This module keeps those
//! contracts for the exported SIS symbols while also exposing Rust helpers for
//! tests and future native callers.

use std::env;
use std::fs;
use std::path::Path;

pub fn path_search(program: &str) -> Option<String> {
    #[cfg(unix)]
    {
        let path = env::var("PATH").unwrap_or_else(|_| String::from("."));
        file_search(program, Some(path.as_str()), "x")
    }

    #[cfg(not(unix))]
    {
        file_search(program, None, "x")
    }
}

pub fn file_search(file: &str, path: Option<&str>, mode: &str) -> Option<String> {
    let path = match path {
        Some(path) if !path.is_empty() => path,
        _ => ".",
    };

    for directory in path.split(':') {
        let candidate = if directory == "." {
            file.to_string()
        } else {
            format!("{directory}/{file}")
        };
        let expanded = tilde_expand(&candidate);

        if check_file(&expanded, mode.as_ref()) {
            return Some(expanded);
        }
    }

    None
}

fn check_file(filename: &str, mode: &str) -> bool {
    let path = Path::new(filename);
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    match mode.as_bytes().first().copied() {
        Some(b'w') => !metadata.permissions().readonly(),
        Some(b'x') => is_executable(path, &metadata),
        _ => true,
    }
}

#[cfg(unix)]
fn is_executable(_path: &Path, metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_path: &Path, _metadata: &fs::Metadata) -> bool {
    // The non-UNIX C branch maps mode "x" to an fopen read probe.
    true
}

fn tilde_expand(filename: &str) -> String {
    let Some(rest) = filename.strip_prefix('~') else {
        return filename.to_string();
    };

    let (username, suffix) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };

    let directory = if username.is_empty() {
        current_home_dir()
    } else if username == "octtools" {
        env::var("OCTTOOLS").ok()
    } else {
        None
    };

    match directory {
        Some(directory) => format!("{directory}{suffix}"),
        None => filename.to_string(),
    }
}

fn current_home_dir() -> Option<String> {
    env::var("HOME")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("USERPROFILE")
                .ok()
                .filter(|value| !value.is_empty())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("logicfriday1_pathsearch_{name}_{unique}"));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn searches_colon_separated_path_in_order() {
        let _guard = TEST_LOCK.lock().unwrap();
        let original = env::current_dir().unwrap();
        let root = temp_dir("path");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        let target = second.join("target.txt");
        File::create(&target).unwrap();

        env::set_current_dir(&root).unwrap();
        let path = "first:second";
        let found = file_search("target.txt", Some(&path), "r").unwrap();
        let found = fs::canonicalize(found).unwrap();
        env::set_current_dir(original).unwrap();

        assert_eq!(found, fs::canonicalize(&target).unwrap());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn null_or_empty_path_searches_current_directory() {
        let _guard = TEST_LOCK.lock().unwrap();
        let original = env::current_dir().unwrap();
        let directory = temp_dir("current");
        let target = directory.join("local.txt");
        File::create(&target).unwrap();

        env::set_current_dir(&directory).unwrap();
        let found = file_search("local.txt", None, "r").unwrap();
        env::set_current_dir(original).unwrap();

        assert_eq!(found, "local.txt");

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn expands_current_user_tilde() {
        let _guard = TEST_LOCK.lock().unwrap();
        let home = temp_dir("home");
        let target = home.join("tilde.txt");
        File::create(&target).unwrap();

        unsafe {
            env::set_var("HOME", &home);
        }

        let found = file_search("tilde.txt", Some("~/"), "r").unwrap();

        assert_eq!(
            fs::canonicalize(found).unwrap(),
            fs::canonicalize(&target).unwrap()
        );

        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn writable_mode_rejects_readonly_files() {
        let directory = temp_dir("readonly");
        let target = directory.join("readonly.txt");
        let mut file = File::create(&target).unwrap();
        writeln!(file, "readonly").unwrap();

        let mut permissions = fs::metadata(&target).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&target, permissions).unwrap();

        assert!(file_search("readonly.txt", Some(directory.to_str().unwrap()), "w").is_none());

        let mut permissions = fs::metadata(&target).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&target, permissions).unwrap();
        fs::remove_dir_all(directory).unwrap();
    }
}
