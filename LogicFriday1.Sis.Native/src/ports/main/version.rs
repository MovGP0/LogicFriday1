//! Native Rust version and library path helpers for SIS startup code.

use std::env;
use std::fmt;

const DEFAULT_COMPILE_DATE: &str = "<compile date not supplied>";
const DEFAULT_PACKAGE_NAME: &str = "SIS";
const DEFAULT_PACKAGE_VERSION: &str = "1.4";
const DEFAULT_LIBRARY: &str = "/projects/sis/sis/common/sis_lib";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VersionFlavor {
    Sis,
    Mis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionConfig {
    pub flavor: VersionFlavor,
    pub package_name: String,
    pub package_version: String,
    pub compile_date: String,
    pub library_path: String,
}

impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            flavor: VersionFlavor::Sis,
            package_name: option_env!("CARGO_PKG_NAME")
                .unwrap_or(DEFAULT_PACKAGE_NAME)
                .to_string(),
            package_version: option_env!("CARGO_PKG_VERSION")
                .unwrap_or(DEFAULT_PACKAGE_VERSION)
                .to_string(),
            compile_date: option_env!("LOGICFRIDAY_SIS_COMPILE_DATE")
                .unwrap_or(DEFAULT_COMPILE_DATE)
                .to_string(),
            library_path: option_env!("LOGICFRIDAY_SIS_LIBRARY")
                .unwrap_or(DEFAULT_LIBRARY)
                .to_string(),
        }
    }
}

impl VersionConfig {
    pub fn sis(package_name: impl Into<String>, package_version: impl Into<String>) -> Self {
        Self {
            flavor: VersionFlavor::Sis,
            package_name: package_name.into(),
            package_version: package_version.into(),
            ..Self::default()
        }
    }

    pub fn mis() -> Self {
        Self {
            flavor: VersionFlavor::Mis,
            ..Self::default()
        }
    }

    pub fn current_version(&self) -> CurrentVersion {
        match self.flavor {
            VersionFlavor::Sis => CurrentVersion::Sis {
                package_name: self.package_name.clone(),
                package_version: self.package_version.clone(),
            },
            VersionFlavor::Mis => CurrentVersion::Mis,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CurrentVersion {
    Sis {
        package_name: String,
        package_version: String,
    },
    Mis,
}

impl fmt::Display for CurrentVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sis {
                package_name,
                package_version,
            } => write!(
                f,
                "UC Berkeley & University of Verona, {package_name} {package_version}"
            ),
            Self::Mis => f.write_str("UC Berkeley, MIS Release 2.2"),
        }
    }
}

pub fn proc_date(date: &str) -> String {
    let fields: Vec<&str> = date.split_whitespace().collect();
    if fields.len() < 6 {
        return date.to_string();
    }

    let month = fields[1];
    let Ok(day_of_month) = fields[2].parse::<u32>() else {
        return date.to_string();
    };
    let Some((hour, minute, _second)) = parse_time(fields[3]) else {
        return date.to_string();
    };
    let Ok(year) = fields[5].parse::<u32>() else {
        return date.to_string();
    };

    let (display_hour, meridian) = if hour >= 12 {
        let display_hour = if hour >= 13 { hour - 12 } else { hour };
        (display_hour, "PM")
    } else {
        let display_hour = if hour == 0 { 12 } else { hour };
        (display_hour, "AM")
    };

    format!(
        "{day_of_month}-{month:>3}-{:02} at {display_hour}:{minute:02} {meridian}",
        year % 100
    )
}

pub fn sis_version() -> String {
    sis_version_with_config(&VersionConfig::default())
}

pub fn sis_version_with_config(config: &VersionConfig) -> String {
    format!(
        "{} (compiled {})",
        config.current_version(),
        proc_date(&config.compile_date)
    )
}

pub fn sis_library() -> String {
    sis_library_with_config(&VersionConfig::default())
}

pub fn sis_library_with_config(config: &VersionConfig) -> String {
    tilde_expand(&config.library_path)
}

fn parse_time(value: &str) -> Option<(u32, u32, u32)> {
    let mut parts = value.split(':');
    let hour = parts.next()?.parse::<u32>().ok()?;
    let minute = parts.next()?.parse::<u32>().ok()?;
    let second = parts.next()?.parse::<u32>().ok()?;

    if parts.next().is_some() || hour > 23 || minute > 59 || second > 59 {
        return None;
    }

    Some((hour, minute, second))
}

fn tilde_expand(path: &str) -> String {
    let Some(rest) = path.strip_prefix('~') else {
        return path.to_string();
    };

    let (user, suffix) = match rest.find(['/', '\\']) {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };

    let home = if user.is_empty() {
        current_user_home()
    } else {
        named_user_home(user)
    };

    match home {
        Some(home) => format!("{home}{suffix}"),
        None => path.to_string(),
    }
}

fn current_user_home() -> Option<String> {
    env_value("HOME")
        .or_else(|| env_value("USERPROFILE"))
        .or_else(|| match (env_value("HOMEDRIVE"), env_value("HOMEPATH")) {
            (Some(drive), Some(path)) => Some(format!("{drive}{path}")),
            _ => None,
        })
}

fn named_user_home(user: &str) -> Option<String> {
    if user == "octtools" {
        return env_value("OCTTOOLS");
    }

    None
}

fn env_value(name: &str) -> Option<String> {
    env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn proc_date_formats_afternoon_date_output() {
        assert_eq!(
            proc_date("Mon Nov 29 14:39:07 PST 1993"),
            "29-Nov-93 at 2:39 PM"
        );
    }

    #[test]
    fn proc_date_formats_midnight_as_am() {
        assert_eq!(
            proc_date("Tue Jan 02 00:05:59 UTC 2024"),
            "2-Jan-24 at 12:05 AM"
        );
    }

    #[test]
    fn proc_date_preserves_unrecognized_input() {
        assert_eq!(proc_date("not a date"), "not a date");
        assert_eq!(
            proc_date("Tue Jan 02 25:05:59 UTC 2024"),
            "Tue Jan 02 25:05:59 UTC 2024"
        );
    }

    #[test]
    fn sis_version_uses_sis_package_string() {
        let config = VersionConfig {
            flavor: VersionFlavor::Sis,
            package_name: "SIS".to_string(),
            package_version: "1.4".to_string(),
            compile_date: "Mon Nov 29 14:39:07 PST 1993".to_string(),
            library_path: DEFAULT_LIBRARY.to_string(),
        };

        assert_eq!(
            sis_version_with_config(&config),
            "UC Berkeley & University of Verona, SIS 1.4 (compiled 29-Nov-93 at 2:39 PM)"
        );
    }

    #[test]
    fn sis_version_uses_mis_release_string() {
        let config = VersionConfig {
            flavor: VersionFlavor::Mis,
            package_name: "ignored".to_string(),
            package_version: "ignored".to_string(),
            compile_date: DEFAULT_COMPILE_DATE.to_string(),
            library_path: DEFAULT_LIBRARY.to_string(),
        };

        assert_eq!(
            sis_version_with_config(&config),
            "UC Berkeley, MIS Release 2.2 (compiled <compile date not supplied>)"
        );
    }

    #[test]
    fn sis_library_expands_current_user_tilde() {
        let _guard = ENV_LOCK.lock().unwrap();

        unsafe {
            env::set_var("HOME", "/tmp/logicfriday-home");
        }

        let config = VersionConfig {
            library_path: "~/sis_lib".to_string(),
            ..VersionConfig::default()
        };

        assert_eq!(
            sis_library_with_config(&config),
            "/tmp/logicfriday-home/sis_lib"
        );
    }

    #[test]
    fn sis_library_expands_octtools_tilde() {
        let _guard = ENV_LOCK.lock().unwrap();

        unsafe {
            env::set_var("OCTTOOLS", "/opt/octtools");
        }

        let config = VersionConfig {
            library_path: "~octtools/sis_lib".to_string(),
            ..VersionConfig::default()
        };

        assert_eq!(sis_library_with_config(&config), "/opt/octtools/sis_lib");
    }

    #[test]
    fn sis_library_preserves_unknown_named_user() {
        let config = VersionConfig {
            library_path: "~missing-user/sis_lib".to_string(),
            ..VersionConfig::default()
        };

        assert_eq!(sis_library_with_config(&config), "~missing-user/sis_lib");
    }
}
