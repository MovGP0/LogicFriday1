//! Native Rust command scaffold for `LogicSynthesis/sis/power/com_power.c`.
//!
//! The C file registers three SIS commands and provides small command handlers:
//! `power_estimate` forwards to `power_command_line_interface`, while
//! `power_free_info` and `power_print` enforce zero user arguments before
//! calling the power utility routines. This module keeps that command behavior
//! in Rust without exposing legacy C ABI symbols. Live registration against the
//! SIS command table and execution against SIS `network_t` remain explicit
//! dependency errors until those native ports are wired together.

use std::error::Error;
use std::fmt;

pub const POWER_FREE_INFO_USAGE: &str = "Too many arguments. Usage: power_free_info";
pub const POWER_PRINT_USAGE: &str = "Too many arguments. Usage: power_print";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerCommandKind {
    Estimate,
    FreeInfo,
    Print,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: PowerCommandKind,
    pub changes_network: bool,
}

pub const POWER_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "power_estimate",
        kind: PowerCommandKind::Estimate,
        changes_network: true,
    },
    CommandRegistration {
        name: "power_free_info",
        kind: PowerCommandKind::FreeInfo,
        changes_network: true,
    },
    CommandRegistration {
        name: "power_print",
        kind: PowerCommandKind::Print,
        changes_network: false,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerCommandOperation {
    RegisterCommands,
    Estimate,
    FreeInfo,
    Print,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerCommandInvocation {
    pub kind: PowerCommandKind,
    pub argv: Vec<String>,
}

impl PowerCommandInvocation {
    pub fn new(kind: PowerCommandKind, argv: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            kind,
            argv: argv.into_iter().map(Into::into).collect(),
        }
    }

    pub fn argc(&self) -> usize {
        self.argv.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerCommandError {
    MissingNativePorts {
        operation: PowerCommandOperation,
    },
    TooManyArguments {
        command: PowerCommandKind,
        usage: &'static str,
    },
    Backend(String),
}

impl fmt::Display for PowerCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
            Self::TooManyArguments { usage, .. } => f.write_str(usage),
            Self::Backend(message) => f.write_str(message),
        }
    }
}

impl Error for PowerCommandError {}

pub trait PowerCommandBackend {
    fn power_command_line_interface(&mut self, argv: &[String]) -> Result<i32, PowerCommandError>;

    fn free_power_info(&mut self) -> Result<i32, PowerCommandError>;

    fn print_power_info(&mut self) -> Result<i32, PowerCommandError>;
}

#[derive(Default)]
pub struct MissingPowerCommandBackend;

impl PowerCommandBackend for MissingPowerCommandBackend {
    fn power_command_line_interface(&mut self, _argv: &[String]) -> Result<i32, PowerCommandError> {
        Err(missing(PowerCommandOperation::Estimate))
    }

    fn free_power_info(&mut self) -> Result<i32, PowerCommandError> {
        Err(missing(PowerCommandOperation::FreeInfo))
    }

    fn print_power_info(&mut self) -> Result<i32, PowerCommandError> {
        Err(missing(PowerCommandOperation::Print))
    }
}

pub fn power_command_registrations() -> &'static [CommandRegistration] {
    POWER_COMMANDS
}

pub fn register_power_commands() -> Result<&'static [CommandRegistration], PowerCommandError> {
    Err(missing(PowerCommandOperation::RegisterCommands))
}

pub fn dispatch_power_command<B>(
    backend: &mut B,
    invocation: &PowerCommandInvocation,
) -> Result<i32, PowerCommandError>
where
    B: PowerCommandBackend,
{
    match invocation.kind {
        PowerCommandKind::Estimate => backend.power_command_line_interface(&invocation.argv),
        PowerCommandKind::FreeInfo => {
            require_no_user_arguments(invocation, POWER_FREE_INFO_USAGE)?;
            backend.free_power_info()
        }
        PowerCommandKind::Print => {
            require_no_user_arguments(invocation, POWER_PRINT_USAGE)?;
            backend.print_power_info()
        }
    }
}

pub fn execute_with_missing_dependencies(
    invocation: &PowerCommandInvocation,
) -> Result<i32, PowerCommandError> {
    dispatch_power_command(&mut MissingPowerCommandBackend, invocation)
}

fn require_no_user_arguments(
    invocation: &PowerCommandInvocation,
    usage: &'static str,
) -> Result<(), PowerCommandError> {
    if invocation.argc() != 1 {
        return Err(PowerCommandError::TooManyArguments {
            command: invocation.kind,
            usage,
        });
    }
    Ok(())
}

fn missing(operation: PowerCommandOperation) -> PowerCommandError {
    PowerCommandError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        calls: Vec<String>,
    }

    impl PowerCommandBackend for RecordingBackend {
        fn power_command_line_interface(
            &mut self,
            argv: &[String],
        ) -> Result<i32, PowerCommandError> {
            self.calls.push(format!("estimate:{argv:?}"));
            Ok(0)
        }

        fn free_power_info(&mut self) -> Result<i32, PowerCommandError> {
            self.calls.push("free".to_owned());
            Ok(0)
        }

        fn print_power_info(&mut self) -> Result<i32, PowerCommandError> {
            self.calls.push("print".to_owned());
            Ok(0)
        }
    }

    #[test]
    fn command_registrations_match_init_power_table() {
        assert_eq!(
            power_command_registrations(),
            &[
                CommandRegistration {
                    name: "power_estimate",
                    kind: PowerCommandKind::Estimate,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "power_free_info",
                    kind: PowerCommandKind::FreeInfo,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "power_print",
                    kind: PowerCommandKind::Print,
                    changes_network: false,
                },
            ]
        );
    }

    #[test]
    fn estimate_forwards_full_argv_to_power_command_line_interface() {
        let mut backend = RecordingBackend::default();
        let invocation = PowerCommandInvocation::new(
            PowerCommandKind::Estimate,
            ["power_estimate", "-m", "sampling"],
        );

        assert_eq!(dispatch_power_command(&mut backend, &invocation), Ok(0));
        assert_eq!(
            backend.calls,
            vec!["estimate:[\"power_estimate\", \"-m\", \"sampling\"]"]
        );
    }

    #[test]
    fn utility_commands_accept_only_command_name_argument() {
        let mut backend = RecordingBackend::default();

        assert_eq!(
            dispatch_power_command(
                &mut backend,
                &PowerCommandInvocation::new(PowerCommandKind::FreeInfo, ["power_free_info"]),
            ),
            Ok(0)
        );
        assert_eq!(
            dispatch_power_command(
                &mut backend,
                &PowerCommandInvocation::new(PowerCommandKind::Print, ["power_print"]),
            ),
            Ok(0)
        );
        assert_eq!(backend.calls, vec!["free", "print"]);
    }

    #[test]
    fn utility_commands_report_legacy_usage_on_extra_arguments() {
        assert_eq!(
            dispatch_power_command(
                &mut RecordingBackend::default(),
                &PowerCommandInvocation::new(PowerCommandKind::FreeInfo, ["power_free_info", "x"]),
            ),
            Err(PowerCommandError::TooManyArguments {
                command: PowerCommandKind::FreeInfo,
                usage: POWER_FREE_INFO_USAGE,
            })
        );
        assert_eq!(
            dispatch_power_command(
                &mut RecordingBackend::default(),
                &PowerCommandInvocation::new(PowerCommandKind::Print, ["power_print", "x"]),
            )
            .unwrap_err()
            .to_string(),
            POWER_PRINT_USAGE
        );
    }

    #[test]
    fn command_registration_is_blocked_on_command_registry_port() {
        assert_eq!(
            register_power_commands(),
            Err(PowerCommandError::MissingNativePorts {
                operation: PowerCommandOperation::RegisterCommands,
            })
        );
    }
}
