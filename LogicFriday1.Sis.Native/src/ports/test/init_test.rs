//! Native Rust scaffold for `LogicSynthesis/sis/test/init_test.c`.
//!
//! The C startup hook registers the private `_test` command with `com_test`
//! and marks the command as network-mutating. The native Rust command registry
//! and the `com_test` command body are ported separately, so this file exposes
//! the registration intent as typed metadata instead of recreating a per-file C
//! ABI shim.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub handler: CommandHandler,
    pub changes_network: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandHandler {
    pub c_symbol: &'static str,
    pub source_file: &'static str,
}

pub const TEST_COMMAND_HANDLER: CommandHandler = CommandHandler {
    c_symbol: "com_test",
    source_file: "LogicSynthesis/sis/test/example.c",
};

pub const TEST_COMMAND_REGISTRATION: CommandRegistration = CommandRegistration {
    name: "_test",
    handler: TEST_COMMAND_HANDLER,
    changes_network: true,
};

pub const INIT_TEST_BLOCKER: &str =
    "Registration can be wired into the native command registry after com_test is ported.";

pub fn init_test_registration() -> CommandRegistration {
    TEST_COMMAND_REGISTRATION
}

pub fn init_test_is_scaffolded() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_c_registration_metadata() {
        let registration = init_test_registration();

        assert_eq!(registration.name, "_test");
        assert_eq!(registration.handler.c_symbol, "com_test");
        assert!(registration.changes_network);
    }

    #[test]
    fn documents_unported_handler_dependency() {
        assert!(init_test_is_scaffolded());
        assert!(TEST_COMMAND_HANDLER.source_file.ends_with("test/example.c"));
        assert!(INIT_TEST_BLOCKER.contains("com_test"));
    }
}
