use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OctIoOptions {
    pub oct_enabled: bool,
}

impl Default for OctIoOptions {
    fn default() -> Self {
        Self { oct_enabled: false }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OctIoCommand {
    WriteOct,
    ReadOct,
}

impl OctIoCommand {
    pub fn name(self) -> &'static str {
        match self {
            Self::WriteOct => "write_oct",
            Self::ReadOct => "read_oct",
        }
    }

    pub fn changes_network(self) -> bool {
        match self {
            Self::WriteOct => false,
            Self::ReadOct => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OctIoCommandDescriptor {
    pub command: OctIoCommand,
    pub name: &'static str,
    pub changes_network: bool,
}

impl OctIoCommandDescriptor {
    pub const fn new(command: OctIoCommand, name: &'static str, changes_network: bool) -> Self {
        Self {
            command,
            name,
            changes_network,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OctIoError {
    RegistrationFailed {
        command: OctIoCommand,
        message: String,
    },
}

impl fmt::Display for OctIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistrationFailed { command, message } => {
                write!(
                    f,
                    "failed to register OCT command '{}': {message}",
                    command.name()
                )
            }
        }
    }
}

impl Error for OctIoError {}

pub trait OctIoCommandRegistrar {
    type Error: Error;

    fn register_octio_command(
        &mut self,
        descriptor: OctIoCommandDescriptor,
    ) -> Result<(), Self::Error>;
}

pub const OCTIO_COMMANDS: [OctIoCommandDescriptor; 2] = [
    OctIoCommandDescriptor::new(OctIoCommand::WriteOct, "write_oct", false),
    OctIoCommandDescriptor::new(OctIoCommand::ReadOct, "read_oct", true),
];

pub fn octio_command_descriptors(options: OctIoOptions) -> &'static [OctIoCommandDescriptor] {
    if options.oct_enabled {
        &OCTIO_COMMANDS
    } else {
        &[]
    }
}

pub fn init_octio<R>(
    registrar: &mut R,
    options: OctIoOptions,
) -> Result<Vec<OctIoCommandDescriptor>, OctIoError>
where
    R: OctIoCommandRegistrar,
{
    let mut registered = Vec::new();

    for descriptor in octio_command_descriptors(options) {
        registrar
            .register_octio_command(*descriptor)
            .map_err(|error| OctIoError::RegistrationFailed {
                command: descriptor.command,
                message: error.to_string(),
            })?;
        registered.push(*descriptor);
    }

    Ok(registered)
}

pub fn end_octio() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestRegistrationError(String);

    impl fmt::Display for TestRegistrationError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl Error for TestRegistrationError {}

    #[derive(Default)]
    struct RecordingRegistrar {
        commands: Vec<OctIoCommandDescriptor>,
        fail_at: Option<usize>,
    }

    impl OctIoCommandRegistrar for RecordingRegistrar {
        type Error = TestRegistrationError;

        fn register_octio_command(
            &mut self,
            descriptor: OctIoCommandDescriptor,
        ) -> Result<(), Self::Error> {
            if self.fail_at == Some(self.commands.len()) {
                return Err(TestRegistrationError(format!(
                    "{} is unavailable",
                    descriptor.name
                )));
            }

            self.commands.push(descriptor);
            Ok(())
        }
    }

    #[test]
    fn command_metadata_matches_legacy_registration() {
        assert_eq!(OctIoCommand::WriteOct.name(), "write_oct");
        assert!(!OctIoCommand::WriteOct.changes_network());

        assert_eq!(OctIoCommand::ReadOct.name(), "read_oct");
        assert!(OctIoCommand::ReadOct.changes_network());

        assert_eq!(
            OCTIO_COMMANDS,
            [
                OctIoCommandDescriptor {
                    command: OctIoCommand::WriteOct,
                    name: "write_oct",
                    changes_network: false
                },
                OctIoCommandDescriptor {
                    command: OctIoCommand::ReadOct,
                    name: "read_oct",
                    changes_network: true
                }
            ]
        );
    }

    #[test]
    fn disabled_oct_registers_no_commands() {
        let mut registrar = RecordingRegistrar::default();

        let registered = init_octio(&mut registrar, OctIoOptions::default()).unwrap();

        assert!(registered.is_empty());
        assert!(registrar.commands.is_empty());
    }

    #[test]
    fn enabled_oct_registers_write_then_read() {
        let mut registrar = RecordingRegistrar::default();
        let options = OctIoOptions { oct_enabled: true };

        let registered = init_octio(&mut registrar, options).unwrap();

        assert_eq!(registered, OCTIO_COMMANDS);
        assert_eq!(registrar.commands, OCTIO_COMMANDS);
    }

    #[test]
    fn registration_failure_reports_command_name() {
        let mut registrar = RecordingRegistrar {
            commands: Vec::new(),
            fail_at: Some(1),
        };
        let options = OctIoOptions { oct_enabled: true };

        let error = init_octio(&mut registrar, options).unwrap_err();

        assert_eq!(
            error,
            OctIoError::RegistrationFailed {
                command: OctIoCommand::ReadOct,
                message: "read_oct is unavailable".to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            "failed to register OCT command 'read_oct': read_oct is unavailable"
        );
    }

    #[test]
    fn shutdown_is_a_noop() {
        end_octio();
    }
}
