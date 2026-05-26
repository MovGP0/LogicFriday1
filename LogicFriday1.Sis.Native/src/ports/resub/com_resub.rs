//! Native Rust command model for `LogicSynthesis/sis/resub/com_resub.c`.
//!
//! The C file registers the `resub` SIS command, parses `-a`, `-b`, and `-d`,
//! and dispatches either whole-network resubstitution or node-list
//! resubstitution. The actual SIS graph mutations still depend on native ports
//! for the command registry, node-list resolver, and algebraic/boolean resub
//! engines, so this module exposes the command behavior through Rust data
//! structures and reports unavailable native support explicitly.

use std::error::Error;
use std::fmt;

pub const USAGE: &str = concat!(
    "usage: resub [-abd] [node-list]\n",
    "    -a\t\tAlgebraic resubstitution (default).\n",
    "    -b\t\tBoolean resubstitution.\n",
    "    -d\t\tDon't use complement (in algebraic resubstitution).\n",
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub changes_network: bool,
}

pub const RESUB_COMMAND: CommandRegistration = CommandRegistration {
    name: "resub",
    changes_network: true,
};

pub fn resub_command_registration() -> CommandRegistration {
    RESUB_COMMAND
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResubMethod {
    Algebraic,
    Boolean,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResubTarget {
    WholeNetwork,
    NodeList(Vec<String>),
}

impl ResubTarget {
    pub fn is_whole_network(&self) -> bool {
        matches!(self, Self::WholeNetwork)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResubCommand {
    pub method: ResubMethod,
    pub use_complement: bool,
    pub target: ResubTarget,
}

impl Default for ResubCommand {
    fn default() -> Self {
        Self {
            method: ResubMethod::Algebraic,
            use_complement: true,
            target: ResubTarget::WholeNetwork,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResubError {
    UnsupportedOption(String),
    MissingNativePorts { operation: ResubOperation },
    Backend(String),
}

impl fmt::Display for ResubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOption(option) => {
                write!(f, "unsupported resub option {option}\n{USAGE}")
            }
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation:?} requires unavailable native Rust SIS support"
            ),
            Self::Backend(message) => f.write_str(message),
        }
    }
}

impl Error for ResubError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResubOperation {
    RegisterCommand,
    ResolveNodeList,
    AlgebraicNetwork,
    AlgebraicNode,
    BooleanNetwork,
    BooleanNode,
}

pub fn parse_resub_args<I, S>(args: I) -> Result<ResubCommand, ResubError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut command = ResubCommand::default();
    let mut operands = Vec::new();
    let mut parsing_options = true;

    for arg in args {
        let arg = arg.as_ref();
        if parsing_options && arg == "--" {
            parsing_options = false;
            continue;
        }

        if parsing_options && arg.starts_with('-') && arg.len() > 1 {
            for flag in arg[1..].chars() {
                match flag {
                    'a' => command.method = ResubMethod::Algebraic,
                    'b' => command.method = ResubMethod::Boolean,
                    'd' => command.use_complement = false,
                    _ => return Err(ResubError::UnsupportedOption(format!("-{flag}"))),
                }
            }
        } else {
            parsing_options = false;
            operands.push(arg.to_owned());
        }
    }

    if !operands.is_empty() {
        command.target = ResubTarget::NodeList(operands);
    }

    Ok(command)
}

pub trait ResubBackend {
    type Node: Clone;

    fn resolve_nodes(&mut self, names: &[String]) -> Result<Vec<Self::Node>, ResubError>;

    fn resub_algebraic_network(&mut self, use_complement: bool) -> Result<(), ResubError>;

    fn resub_algebraic_node(
        &mut self,
        node: Self::Node,
        use_complement: bool,
    ) -> Result<(), ResubError>;

    fn resub_boolean_network(&mut self) -> Result<(), ResubError>;

    fn resub_boolean_node(&mut self, node: Self::Node) -> Result<(), ResubError>;
}

pub fn dispatch_resub_command<B>(backend: &mut B, command: &ResubCommand) -> Result<(), ResubError>
where
    B: ResubBackend,
{
    match (&command.method, &command.target) {
        (ResubMethod::Algebraic, ResubTarget::WholeNetwork) => {
            backend.resub_algebraic_network(command.use_complement)
        }
        (ResubMethod::Boolean, ResubTarget::WholeNetwork) => backend.resub_boolean_network(),
        (ResubMethod::Algebraic, ResubTarget::NodeList(names)) => {
            for node in backend.resolve_nodes(names)? {
                backend.resub_algebraic_node(node, command.use_complement)?;
            }
            Ok(())
        }
        (ResubMethod::Boolean, ResubTarget::NodeList(names)) => {
            for node in backend.resolve_nodes(names)? {
                backend.resub_boolean_node(node)?;
            }
            Ok(())
        }
    }
}

#[derive(Default)]
pub struct MissingResubBackend;

impl ResubBackend for MissingResubBackend {
    type Node = String;

    fn resolve_nodes(&mut self, _names: &[String]) -> Result<Vec<Self::Node>, ResubError> {
        Err(missing(ResubOperation::ResolveNodeList))
    }

    fn resub_algebraic_network(&mut self, _use_complement: bool) -> Result<(), ResubError> {
        Err(missing(ResubOperation::AlgebraicNetwork))
    }

    fn resub_algebraic_node(
        &mut self,
        _node: Self::Node,
        _use_complement: bool,
    ) -> Result<(), ResubError> {
        Err(missing(ResubOperation::AlgebraicNode))
    }

    fn resub_boolean_network(&mut self) -> Result<(), ResubError> {
        Err(missing(ResubOperation::BooleanNetwork))
    }

    fn resub_boolean_node(&mut self, _node: Self::Node) -> Result<(), ResubError> {
        Err(missing(ResubOperation::BooleanNode))
    }
}

pub fn register_resub_command() -> Result<CommandRegistration, ResubError> {
    Err(missing(ResubOperation::RegisterCommand))
}

pub fn execute_with_missing_dependencies(command: &ResubCommand) -> Result<(), ResubError> {
    dispatch_resub_command(&mut MissingResubBackend, command)
}

fn missing(operation: ResubOperation) -> ResubError {
    ResubError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        calls: Vec<String>,
    }

    impl ResubBackend for RecordingBackend {
        type Node = String;

        fn resolve_nodes(&mut self, names: &[String]) -> Result<Vec<Self::Node>, ResubError> {
            self.calls.push(format!("resolve:{names:?}"));
            Ok(names.to_vec())
        }

        fn resub_algebraic_network(&mut self, use_complement: bool) -> Result<(), ResubError> {
            self.calls
                .push(format!("algebraic_network:{use_complement}"));
            Ok(())
        }

        fn resub_algebraic_node(
            &mut self,
            node: Self::Node,
            use_complement: bool,
        ) -> Result<(), ResubError> {
            self.calls
                .push(format!("algebraic_node:{node}:{use_complement}"));
            Ok(())
        }

        fn resub_boolean_network(&mut self) -> Result<(), ResubError> {
            self.calls.push("boolean_network".to_owned());
            Ok(())
        }

        fn resub_boolean_node(&mut self, node: Self::Node) -> Result<(), ResubError> {
            self.calls.push(format!("boolean_node:{node}"));
            Ok(())
        }
    }

    #[test]
    fn default_command_is_algebraic_whole_network_with_complement() {
        assert_eq!(
            parse_resub_args(std::iter::empty::<&str>()),
            Ok(ResubCommand::default())
        );
    }

    #[test]
    fn parses_method_and_complement_options_with_last_method_winning() {
        assert_eq!(
            parse_resub_args(["-bd", "-a"]).unwrap(),
            ResubCommand {
                method: ResubMethod::Algebraic,
                use_complement: false,
                target: ResubTarget::WholeNetwork,
            }
        );
        assert_eq!(
            parse_resub_args(["-ab"]).unwrap().method,
            ResubMethod::Boolean
        );
    }

    #[test]
    fn preserves_resub_star_as_node_list_operand() {
        assert_eq!(
            parse_resub_args(["*"]).unwrap().target,
            ResubTarget::NodeList(vec!["*".to_owned()])
        );
    }

    #[test]
    fn rejects_unknown_options() {
        assert_eq!(
            parse_resub_args(["-x"]),
            Err(ResubError::UnsupportedOption("-x".to_owned()))
        );
    }

    #[test]
    fn dispatches_whole_network_modes() {
        let mut backend = RecordingBackend::default();
        dispatch_resub_command(&mut backend, &parse_resub_args(["-d"]).unwrap()).unwrap();
        dispatch_resub_command(&mut backend, &parse_resub_args(["-b"]).unwrap()).unwrap();

        assert_eq!(
            backend.calls,
            vec!["algebraic_network:false", "boolean_network"]
        );
    }

    #[test]
    fn dispatches_node_list_modes_after_resolution() {
        let mut backend = RecordingBackend::default();
        dispatch_resub_command(&mut backend, &parse_resub_args(["-d", "n1", "n2"]).unwrap())
            .unwrap();
        dispatch_resub_command(&mut backend, &parse_resub_args(["-b", "n3"]).unwrap()).unwrap();

        assert_eq!(
            backend.calls,
            vec![
                "resolve:[\"n1\", \"n2\"]",
                "algebraic_node:n1:false",
                "algebraic_node:n2:false",
                "resolve:[\"n3\"]",
                "boolean_node:n3",
            ]
        );
    }

    #[test]
    fn missing_backend_reports_failed_operation() {
        let Err(ResubError::MissingNativePorts { operation }) =
            execute_with_missing_dependencies(&parse_resub_args(["-b"]).unwrap())
        else {
            panic!("expected missing native port error");
        };

        assert_eq!(operation, ResubOperation::BooleanNetwork);
        assert_eq!(
            execute_with_missing_dependencies(&parse_resub_args(["-b"]).unwrap())
                .unwrap_err()
                .to_string(),
            "BooleanNetwork requires unavailable native Rust SIS support"
        );
    }

    #[test]
    fn command_registration_records_legacy_changes_network_flag() {
        assert_eq!(
            resub_command_registration(),
            CommandRegistration {
                name: "resub",
                changes_network: true,
            }
        );
        assert!(matches!(
            register_resub_command(),
            Err(ResubError::MissingNativePorts {
                operation: ResubOperation::RegisterCommand,
            })
        ));
    }
}
