//! Port of `LogicSynthesis/sis/util/texpand.c`.
//!
//! The C routine returns a newly allocated copy of its input, expanding a
//! leading tilde when the platform build enables BSD-style user lookup. This
//! native port exposes the behavior as an owned `String`.

use std::env;

pub fn tilde_expand(filename: &str) -> String {
    let Some(rest) = filename.strip_prefix('~') else {
        return filename.to_string();
    };

    let (username, suffix) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };

    let directory = if username.is_empty() {
        current_user_home()
    } else if username == "octtools" {
        env_string("OCTTOOLS")
    } else {
        user_home(username)
    };

    match directory {
        Some(directory) => format!("{directory}{suffix}"),
        None => filename.to_string(),
    }
}

fn current_user_home() -> Option<String> {
    env_string("HOME")
        .or_else(|| env_string("USERPROFILE"))
        .or_else(|| match (env_string("HOMEDRIVE"), env_string("HOMEPATH")) {
            (Some(drive), Some(path)) => Some(format!("{drive}{path}")),
            _ => None,
        })
}

fn env_string(name: &str) -> Option<String> {
    env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

#[cfg(unix)]
fn user_home(username: &str) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;

    passwd.lines().find_map(|line| {
        let mut fields = line.split(':');
        let name = fields.next()?;
        if name != username {
            return None;
        }

        fields.nth(4).map(str::to_string)
    })
}

#[cfg(not(unix))]
fn user_home(_username: &str) -> Option<String> {
    None
}

// TODO: expose this module through the crate module tree once translated Rust
// callers need it.

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn leaves_names_without_tilde_unchanged() {
        assert_eq!(tilde_expand("network.blif"), "network.blif");
    }

    #[test]
    fn leaves_unknown_user_tilde_unchanged() {
        assert_eq!(
            tilde_expand("~logicfriday_user_that_should_not_exist/file"),
            "~logicfriday_user_that_should_not_exist/file"
        );
    }

    #[test]
    fn expands_current_user_tilde_from_home() {
        let _guard = ENV_LOCK.lock().unwrap();

        unsafe {
            env::set_var("HOME", "/tmp/logicfriday-home");
        }

        assert_eq!(
            tilde_expand("~/library/sis"),
            "/tmp/logicfriday-home/library/sis"
        );
    }

    #[test]
    fn expands_octtools_from_environment() {
        let _guard = ENV_LOCK.lock().unwrap();

        unsafe {
            env::set_var("OCTTOOLS", "/opt/octtools");
        }

        assert_eq!(tilde_expand("~octtools/bin"), "/opt/octtools/bin");
    }
}
