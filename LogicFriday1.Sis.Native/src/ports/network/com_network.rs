//! Native command model for `LogicSynthesis/sis/network/com_network.c`.
//!
//! The C file registers command entry points and delegates most real network
//! rewrites to lower-level SIS routines.  This port keeps the command parsing,
//! registration metadata, and local consistency checks in Rust while exposing
//! the rewrite and file-loading operations through an injectable backend.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use super::network_util::{Network, NetworkUtilError, NodeId, NodeKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkCommandKind {
    Check,
    Ripup,
    Collapse,
    Espresso,
    Sweep,
    Verify,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: NetworkCommandKind,
    pub changes_network: bool,
}

pub const NETWORK_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "_check",
        kind: NetworkCommandKind::Check,
        changes_network: false,
    },
    CommandRegistration {
        name: "_ripup",
        kind: NetworkCommandKind::Ripup,
        changes_network: true,
    },
    CommandRegistration {
        name: "collapse",
        kind: NetworkCommandKind::Collapse,
        changes_network: true,
    },
    CommandRegistration {
        name: "espresso",
        kind: NetworkCommandKind::Espresso,
        changes_network: true,
    },
    CommandRegistration {
        name: "sweep",
        kind: NetworkCommandKind::Sweep,
        changes_network: true,
    },
    CommandRegistration {
        name: "verify",
        kind: NetworkCommandKind::Verify,
        changes_network: false,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeSelection {
    All,
    Named(String),
    FaninsOf(String),
    FanoutsOf(String),
    PrimaryInputs,
    PrimaryOutputDrivers,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationMethod {
    Collapse,
    Bdd,
    Partitioned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationTarget {
    Original,
    CurrentAgainstFile(String),
    FileAgainstFile { left: String, right: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkCommand {
    Check {
        verbose: bool,
    },
    Ripup {
        nodes: Vec<NodeSelection>,
    },
    Collapse {
        nodes: Option<Vec<NodeSelection>>,
    },
    Espresso,
    Sweep,
    Verify {
        method: VerificationMethod,
        verbose: bool,
        target: VerificationTarget,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkCommandOutput {
    pub status: i32,
    pub messages: Vec<String>,
}

impl NetworkCommandOutput {
    pub fn success() -> Self {
        Self {
            status: 0,
            messages: Vec::new(),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            status: 1,
            messages: vec![message.into()],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkCommandError {
    UnknownCommand(String),
    Usage {
        command: NetworkCommandKind,
        usage: &'static str,
    },
    UnsupportedOption(String),
    MissingOptionValue(char),
    MissingNode(String),
    InvalidPrimaryOutput(NodeId),
    NetworkUtil(NetworkUtilError),
    SequentialNetwork,
    MissingOriginalNetwork,
    OperationUnavailable(NetworkCommandKind),
}

impl fmt::Display for NetworkCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(formatter, "unknown network command {command}"),
            Self::Usage { usage, .. } => write!(formatter, "{usage}"),
            Self::UnsupportedOption(option) => write!(formatter, "unsupported option {option}"),
            Self::MissingOptionValue(option) => write!(formatter, "missing value for -{option}"),
            Self::MissingNode(name) => write!(formatter, "network node '{name}' was not found"),
            Self::InvalidPrimaryOutput(node) => write!(
                formatter,
                "primary output {} must have exactly one fanin",
                node.index()
            ),
            Self::NetworkUtil(error) => write!(formatter, "{error}"),
            Self::SequentialNetwork => write!(
                formatter,
                "Use the verify_fsm command to verify sequential circuits."
            ),
            Self::MissingOriginalNetwork => write!(formatter, "error -- no original network"),
            Self::OperationUnavailable(command) => {
                write!(
                    formatter,
                    "native network command operation {command:?} is unavailable"
                )
            }
        }
    }
}

impl Error for NetworkCommandError {}

impl From<NetworkUtilError> for NetworkCommandError {
    fn from(value: NetworkUtilError) -> Self {
        Self::NetworkUtil(value)
    }
}

pub trait NetworkCommandBackend {
    fn network(&self) -> &Network;

    fn latch_count(&self) -> usize {
        0
    }

    fn has_original_network(&self) -> bool {
        self.network().original().is_some()
    }

    fn ripup_nodes(&mut self, _nodes: &[NodeId]) -> Result<(), NetworkCommandError> {
        Err(NetworkCommandError::OperationUnavailable(
            NetworkCommandKind::Ripup,
        ))
    }

    fn collapse_network(&mut self) -> Result<(), NetworkCommandError> {
        Err(NetworkCommandError::OperationUnavailable(
            NetworkCommandKind::Collapse,
        ))
    }

    fn collapse_single_node(&mut self, _node: NodeId) -> Result<(), NetworkCommandError> {
        Err(NetworkCommandError::OperationUnavailable(
            NetworkCommandKind::Collapse,
        ))
    }

    fn collapse_node_pair(
        &mut self,
        _left: NodeId,
        _right: NodeId,
    ) -> Result<(), NetworkCommandError> {
        Err(NetworkCommandError::OperationUnavailable(
            NetworkCommandKind::Collapse,
        ))
    }

    fn sweep_network(&mut self) -> Result<(), NetworkCommandError> {
        Err(NetworkCommandError::OperationUnavailable(
            NetworkCommandKind::Sweep,
        ))
    }

    fn espresso_network(&mut self) -> Result<bool, NetworkCommandError> {
        Err(NetworkCommandError::OperationUnavailable(
            NetworkCommandKind::Espresso,
        ))
    }

    fn verify_with_dc(
        &mut self,
        _method: VerificationMethod,
        _target: &VerificationTarget,
        _verbose: bool,
    ) -> Result<i32, NetworkCommandError> {
        Err(NetworkCommandError::OperationUnavailable(
            NetworkCommandKind::Verify,
        ))
    }
}

pub fn network_command_registrations() -> &'static [CommandRegistration] {
    NETWORK_COMMANDS
}

pub fn parse_network_command<I, S>(
    command_name: &str,
    args: I,
) -> Result<NetworkCommand, NetworkCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    match command_name {
        "_check" => parse_check_args(&args),
        "_ripup" => Ok(NetworkCommand::Ripup {
            nodes: parse_node_selections(&args)?,
        }),
        "collapse" => parse_collapse_args(&args),
        "espresso" => parse_zero_arg_command(&args, NetworkCommandKind::Espresso)
            .map(|()| NetworkCommand::Espresso),
        "sweep" => {
            parse_zero_arg_command(&args, NetworkCommandKind::Sweep).map(|()| NetworkCommand::Sweep)
        }
        "verify" => parse_verify_args(&args),
        _ => Err(NetworkCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn dispatch_network_command<B>(
    backend: &mut B,
    command: &NetworkCommand,
) -> Result<NetworkCommandOutput, NetworkCommandError>
where
    B: NetworkCommandBackend,
{
    match command {
        NetworkCommand::Check { verbose } => command_check(backend.network(), *verbose),
        NetworkCommand::Ripup { nodes } => {
            let selected = resolve_node_selections(backend.network(), nodes)?;
            backend.ripup_nodes(&selected)?;
            Ok(NetworkCommandOutput::success())
        }
        NetworkCommand::Collapse { nodes } => {
            command_collapse(backend, nodes.as_deref())?;
            Ok(NetworkCommandOutput::success())
        }
        NetworkCommand::Espresso => {
            backend.espresso_network()?;
            Ok(NetworkCommandOutput::success())
        }
        NetworkCommand::Sweep => {
            backend.sweep_network()?;
            Ok(NetworkCommandOutput::success())
        }
        NetworkCommand::Verify {
            method,
            verbose,
            target,
        } => command_verify(backend, *method, *verbose, target),
    }
}

pub fn command_check(
    network: &Network,
    verbose: bool,
) -> Result<NetworkCommandOutput, NetworkCommandError> {
    let diagnostics = check_network(network)?;
    if diagnostics.is_empty() {
        let mut output = NetworkCommandOutput::success();
        if verbose {
            output
                .messages
                .push("check: network passes consistency check".to_owned());
        }
        return Ok(output);
    }

    let mut output = NetworkCommandOutput::failure("check: problem detected with network");
    output.messages.extend(diagnostics);
    Ok(output)
}

pub fn check_network(network: &Network) -> Result<Vec<String>, NetworkCommandError> {
    let mut diagnostics = Vec::new();
    let mut present = BTreeSet::new();

    for (node, _) in network.nodes() {
        present.insert(node);
    }

    for (node, data) in network.nodes() {
        if data.kind == NodeKind::PrimaryOutput && data.fanins.len() != 1 {
            diagnostics.push(format!(
                "network_check: primary output {} has {} fanins",
                data.name,
                data.fanins.len()
            ));
        }

        for fanin in &data.fanins {
            if !present.contains(fanin) {
                diagnostics.push(format!(
                    "network_check: node {} references missing fanin {}",
                    data.name,
                    fanin.index()
                ));
                continue;
            }

            let fanin_node = network.node(*fanin)?;
            if !fanin_node.fanouts.contains(&node) {
                diagnostics.push(format!(
                    "network_check: node {} fanin {} is missing back-reference",
                    data.name, fanin_node.name
                ));
            }
        }

        for fanout in &data.fanouts {
            if !present.contains(fanout) {
                diagnostics.push(format!(
                    "network_check: node {} references missing fanout {}",
                    data.name,
                    fanout.index()
                ));
                continue;
            }

            let fanout_node = network.node(*fanout)?;
            if !fanout_node.fanins.contains(&node) {
                diagnostics.push(format!(
                    "network_check: node {} fanout {} is missing back-reference",
                    data.name, fanout_node.name
                ));
            }
        }
    }

    if is_acyclic(network)? {
        return Ok(diagnostics);
    }

    diagnostics.push("network_check: network contains a cycle".to_owned());
    Ok(diagnostics)
}

pub fn resolve_node_selections(
    network: &Network,
    selections: &[NodeSelection],
) -> Result<Vec<NodeId>, NetworkCommandError> {
    let selections = if selections.is_empty() {
        &[NodeSelection::All][..]
    } else {
        selections
    };
    let mut nodes = Vec::new();

    for selection in selections {
        match selection {
            NodeSelection::All => {
                nodes.extend(network.nodes().map(|(node, _)| node));
            }
            NodeSelection::Named(name) => {
                nodes.push(resolve_named_node(network, name)?);
            }
            NodeSelection::FaninsOf(name) => {
                let node = resolve_named_node(network, name)?;
                nodes.extend(network.node(node)?.fanins.iter().copied());
            }
            NodeSelection::FanoutsOf(name) => {
                let node = resolve_named_node(network, name)?;
                nodes.extend(network.node(node)?.fanouts.iter().copied());
            }
            NodeSelection::PrimaryInputs => {
                nodes.extend(network.primary_inputs().iter().copied());
            }
            NodeSelection::PrimaryOutputDrivers => {
                for output in network.primary_outputs() {
                    let output_node = network.node(*output)?;
                    if output_node.fanins.len() != 1 {
                        return Err(NetworkCommandError::InvalidPrimaryOutput(*output));
                    }
                    nodes.push(output_node.fanins[0]);
                }
            }
        }
    }

    Ok(nodes)
}

fn command_collapse<B>(
    backend: &mut B,
    selections: Option<&[NodeSelection]>,
) -> Result<(), NetworkCommandError>
where
    B: NetworkCommandBackend,
{
    let Some(selections) = selections else {
        return backend.collapse_network();
    };

    let nodes = resolve_node_selections(backend.network(), selections)?;
    match nodes.as_slice() {
        [node] => backend.collapse_single_node(*node),
        [left, right] => backend.collapse_node_pair(*left, *right),
        _ => Err(NetworkCommandError::Usage {
            command: NetworkCommandKind::Collapse,
            usage: "usage: clp [n1] [n2]",
        }),
    }
}

fn command_verify<B>(
    backend: &mut B,
    method: VerificationMethod,
    verbose: bool,
    target: &VerificationTarget,
) -> Result<NetworkCommandOutput, NetworkCommandError>
where
    B: NetworkCommandBackend,
{
    if backend.latch_count() != 0 {
        return Err(NetworkCommandError::SequentialNetwork);
    }

    if matches!(target, VerificationTarget::Original) && !backend.has_original_network() {
        return Err(NetworkCommandError::MissingOriginalNetwork);
    }

    let status = backend.verify_with_dc(method, target, verbose)?;
    Ok(NetworkCommandOutput {
        status,
        messages: Vec::new(),
    })
}

fn parse_check_args(args: &[String]) -> Result<NetworkCommand, NetworkCommandError> {
    let mut verbose = false;
    for arg in args {
        match arg.as_str() {
            "-v" => verbose = true,
            _ if arg.starts_with('-') => {
                return Err(NetworkCommandError::UnsupportedOption(arg.clone()));
            }
            _ => {
                return Err(NetworkCommandError::Usage {
                    command: NetworkCommandKind::Check,
                    usage: "usage: _check [-v]",
                });
            }
        }
    }

    Ok(NetworkCommand::Check { verbose })
}

fn parse_collapse_args(args: &[String]) -> Result<NetworkCommand, NetworkCommandError> {
    if args.is_empty() {
        return Ok(NetworkCommand::Collapse { nodes: None });
    }

    Ok(NetworkCommand::Collapse {
        nodes: Some(parse_node_selections(args)?),
    })
}

fn parse_verify_args(args: &[String]) -> Result<NetworkCommand, NetworkCommandError> {
    let mut method = VerificationMethod::Collapse;
    let mut verbose = false;
    let mut operands = Vec::new();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "-v" => {
                verbose = true;
                index += 1;
            }
            "-m" => {
                let value = args
                    .get(index + 1)
                    .ok_or(NetworkCommandError::MissingOptionValue('m'))?;
                method = parse_verification_method(value)?;
                index += 2;
            }
            _ if arg.starts_with('-') => {
                return Err(NetworkCommandError::UnsupportedOption(arg.clone()));
            }
            _ => {
                operands.push(arg.clone());
                index += 1;
            }
        }
    }

    let target = match operands.as_slice() {
        [] => VerificationTarget::Original,
        [path] => VerificationTarget::CurrentAgainstFile(path.clone()),
        [left, right] => VerificationTarget::FileAgainstFile {
            left: left.clone(),
            right: right.clone(),
        },
        _ => {
            return Err(NetworkCommandError::Usage {
                command: NetworkCommandKind::Verify,
                usage: "usage: verify [-m] [[net1.blif] [net2.blif]]",
            });
        }
    };

    Ok(NetworkCommand::Verify {
        method,
        verbose,
        target,
    })
}

fn parse_zero_arg_command(
    args: &[String],
    command: NetworkCommandKind,
) -> Result<(), NetworkCommandError> {
    if args.is_empty() {
        return Ok(());
    }

    Err(NetworkCommandError::Usage {
        command,
        usage: match command {
            NetworkCommandKind::Espresso => "usage: espresso",
            NetworkCommandKind::Sweep => "usage: sweep",
            _ => "usage: network-command",
        },
    })
}

fn parse_node_selections(args: &[String]) -> Result<Vec<NodeSelection>, NetworkCommandError> {
    if args.is_empty() {
        return Ok(Vec::new());
    }

    args.iter().map(|arg| parse_node_selection(arg)).collect()
}

fn parse_node_selection(arg: &str) -> Result<NodeSelection, NetworkCommandError> {
    if arg == "*" {
        return Ok(NodeSelection::All);
    }

    if arg == "i()" {
        return Ok(NodeSelection::PrimaryInputs);
    }

    if arg == "o()" {
        return Ok(NodeSelection::PrimaryOutputDrivers);
    }

    if let Some(name) = parenthesized_arg(arg, "i(") {
        return Ok(NodeSelection::FaninsOf(name.to_owned()));
    }

    if let Some(name) = parenthesized_arg(arg, "o(") {
        return Ok(NodeSelection::FanoutsOf(name.to_owned()));
    }

    Ok(NodeSelection::Named(arg.to_owned()))
}

fn parse_verification_method(value: &str) -> Result<VerificationMethod, NetworkCommandError> {
    match value {
        "clp" => Ok(VerificationMethod::Collapse),
        "bdd" => Ok(VerificationMethod::Bdd),
        "par" => Ok(VerificationMethod::Partitioned),
        _ => Err(NetworkCommandError::Usage {
            command: NetworkCommandKind::Verify,
            usage: "usage: verify [-m] [[net1.blif] [net2.blif]]",
        }),
    }
}

fn parenthesized_arg<'a>(arg: &'a str, prefix: &str) -> Option<&'a str> {
    arg.strip_prefix(prefix)?.strip_suffix(')')
}

fn resolve_named_node(network: &Network, name: &str) -> Result<NodeId, NetworkCommandError> {
    let node = network
        .find_node(name)
        .ok_or_else(|| NetworkCommandError::MissingNode(name.to_owned()))?;
    let data = network.node(node)?;

    if data.kind != NodeKind::PrimaryOutput {
        return Ok(node);
    }

    if data.fanins.len() != 1 {
        return Err(NetworkCommandError::InvalidPrimaryOutput(node));
    }

    Ok(data.fanins[0])
}

fn is_acyclic(network: &Network) -> Result<bool, NetworkCommandError> {
    let mut states = BTreeMap::new();
    for (node, _) in network.nodes() {
        if !visit_for_cycle(network, node, &mut states)? {
            return Ok(false);
        }
    }

    Ok(true)
}

fn visit_for_cycle(
    network: &Network,
    node: NodeId,
    states: &mut BTreeMap<NodeId, VisitState>,
) -> Result<bool, NetworkCommandError> {
    match states.get(&node) {
        Some(VisitState::Active) => return Ok(false),
        Some(VisitState::Done) => return Ok(true),
        None => {}
    }

    states.insert(node, VisitState::Active);
    for fanin in &network.node(node)?.fanins {
        if !visit_for_cycle(network, *fanin, states)? {
            return Ok(false);
        }
    }
    states.insert(node, VisitState::Done);
    Ok(true)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Active,
    Done,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::network::network_util::{CoverValue, Cube, NetworkNode, SopCover};

    #[derive(Default)]
    struct RecordingBackend {
        network: Network,
        latches: usize,
        calls: Vec<String>,
    }

    impl RecordingBackend {
        fn with_network(network: Network) -> Self {
            Self {
                network,
                latches: 0,
                calls: Vec::new(),
            }
        }
    }

    impl NetworkCommandBackend for RecordingBackend {
        fn network(&self) -> &Network {
            &self.network
        }

        fn latch_count(&self) -> usize {
            self.latches
        }

        fn ripup_nodes(&mut self, nodes: &[NodeId]) -> Result<(), NetworkCommandError> {
            self.calls.push(format!("ripup:{:?}", indexes(nodes)));
            Ok(())
        }

        fn collapse_network(&mut self) -> Result<(), NetworkCommandError> {
            self.calls.push("collapse:network".to_owned());
            Ok(())
        }

        fn collapse_single_node(&mut self, node: NodeId) -> Result<(), NetworkCommandError> {
            self.calls.push(format!("collapse:single:{}", node.index()));
            Ok(())
        }

        fn collapse_node_pair(
            &mut self,
            left: NodeId,
            right: NodeId,
        ) -> Result<(), NetworkCommandError> {
            self.calls
                .push(format!("collapse:pair:{}:{}", left.index(), right.index()));
            Ok(())
        }

        fn sweep_network(&mut self) -> Result<(), NetworkCommandError> {
            self.calls.push("sweep".to_owned());
            Ok(())
        }

        fn espresso_network(&mut self) -> Result<bool, NetworkCommandError> {
            self.calls.push("espresso".to_owned());
            Ok(true)
        }

        fn verify_with_dc(
            &mut self,
            method: VerificationMethod,
            target: &VerificationTarget,
            verbose: bool,
        ) -> Result<i32, NetworkCommandError> {
            self.calls
                .push(format!("verify:{method:?}:{target:?}:{verbose}"));
            Ok(0)
        }
    }

    fn sample_network() -> Network {
        let mut network = Network::new();
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let n = network
            .add_internal(
                "n",
                [a, b],
                SopCover::new([Cube::new([CoverValue::One, CoverValue::One])]),
            )
            .unwrap();
        let y = network.add_primary_output(n).unwrap();
        network.change_node_name(y, "y").unwrap();
        network
    }

    fn indexes(nodes: &[NodeId]) -> Vec<usize> {
        nodes.iter().map(|node| node.index()).collect()
    }

    #[test]
    fn registrations_match_original_command_names() {
        assert_eq!(
            network_command_registrations(),
            &[
                CommandRegistration {
                    name: "_check",
                    kind: NetworkCommandKind::Check,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "_ripup",
                    kind: NetworkCommandKind::Ripup,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "collapse",
                    kind: NetworkCommandKind::Collapse,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "espresso",
                    kind: NetworkCommandKind::Espresso,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "sweep",
                    kind: NetworkCommandKind::Sweep,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "verify",
                    kind: NetworkCommandKind::Verify,
                    changes_network: false,
                },
            ]
        );
    }

    #[test]
    fn parses_node_selection_syntax_used_by_com_get_nodes() {
        assert_eq!(
            parse_node_selections(&[
                "*".to_owned(),
                "i()".to_owned(),
                "o()".to_owned(),
                "i(n)".to_owned(),
                "o(n)".to_owned(),
                "n".to_owned(),
            ])
            .unwrap(),
            vec![
                NodeSelection::All,
                NodeSelection::PrimaryInputs,
                NodeSelection::PrimaryOutputDrivers,
                NodeSelection::FaninsOf("n".to_owned()),
                NodeSelection::FanoutsOf("n".to_owned()),
                NodeSelection::Named("n".to_owned()),
            ]
        );
    }

    #[test]
    fn resolves_primary_output_names_to_their_driver_like_c_helper() {
        let network = sample_network();
        let y_driver = network
            .node(network.find_node("y").unwrap())
            .unwrap()
            .fanins[0];

        assert_eq!(
            resolve_node_selections(&network, &[NodeSelection::Named("y".to_owned())]).unwrap(),
            vec![y_driver]
        );
        assert_eq!(
            resolve_node_selections(&network, &[NodeSelection::PrimaryOutputDrivers]).unwrap(),
            vec![y_driver]
        );
    }

    #[test]
    fn check_reports_success_message_only_when_verbose() {
        let network = sample_network();

        assert_eq!(
            command_check(&network, false).unwrap().messages,
            Vec::<String>::new()
        );
        assert_eq!(
            command_check(&network, true).unwrap().messages,
            vec!["check: network passes consistency check"]
        );
    }

    #[test]
    fn collapse_dispatch_preserves_original_arity_rules() {
        let network = sample_network();
        let mut backend = RecordingBackend::with_network(network);

        let command = parse_network_command("collapse", ["y", "a"]).unwrap();
        dispatch_network_command(&mut backend, &command).unwrap();

        assert_eq!(backend.calls, vec!["collapse:pair:2:0"]);
    }

    #[test]
    fn collapse_rejects_more_than_two_resolved_nodes() {
        let network = sample_network();
        let mut backend = RecordingBackend::with_network(network);
        let command = parse_network_command("collapse", ["*"]).unwrap();

        assert!(matches!(
            dispatch_network_command(&mut backend, &command),
            Err(NetworkCommandError::Usage {
                command: NetworkCommandKind::Collapse,
                ..
            })
        ));
    }

    #[test]
    fn verify_parses_method_verbose_and_targets() {
        assert_eq!(
            parse_network_command("verify", ["-m", "bdd", "-v", "left.blif", "right.blif"])
                .unwrap(),
            NetworkCommand::Verify {
                method: VerificationMethod::Bdd,
                verbose: true,
                target: VerificationTarget::FileAgainstFile {
                    left: "left.blif".to_owned(),
                    right: "right.blif".to_owned(),
                },
            }
        );
    }

    #[test]
    fn verify_rejects_sequential_networks_before_backend_dispatch() {
        let mut backend = RecordingBackend::with_network(sample_network());
        backend.latches = 1;
        let command = parse_network_command("verify", ["other.blif"]).unwrap();

        assert_eq!(
            dispatch_network_command(&mut backend, &command).unwrap_err(),
            NetworkCommandError::SequentialNetwork
        );
        assert!(backend.calls.is_empty());
    }

    #[test]
    fn zero_argument_commands_reject_extra_operands() {
        assert!(matches!(
            parse_network_command("sweep", ["x"]),
            Err(NetworkCommandError::Usage {
                command: NetworkCommandKind::Sweep,
                ..
            })
        ));
        assert!(matches!(
            parse_network_command("espresso", ["x"]),
            Err(NetworkCommandError::Usage {
                command: NetworkCommandKind::Espresso,
                ..
            })
        ));
    }

    #[test]
    fn source_contains_no_legacy_c_abi_export() {
        let source = include_str!("com_network.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
