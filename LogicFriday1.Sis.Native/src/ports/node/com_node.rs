//! Native command model for the SIS node commands.
//!
//! This module keeps the legacy command surface as typed Rust data and small
//! execution helpers over owned nodes. Commands whose underlying algebra has
//! not been ported yet return explicit dependency errors instead of exporting
//! per-file ABI entry points.

#[cfg(test)]
#[path = "node.rs"]
mod native_node;

#[cfg(test)]
use native_node::{
    Cover, Cube, Node, NodeError, NodeFunction, SimplifyMode, node_function, node_not, node_scc,
    node_simplify_replace,
};

#[cfg(not(test))]
use super::node::{
    Cover, Cube, Node, NodeError, NodeFunction, SimplifyMode, node_function, node_not, node_scc,
    node_simplify_replace,
};

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub const NODE_COMMANDS: &[NodeCommandRegistration] = &[
    NodeCommandRegistration::new(NodeCommandKind::WeakDivision, "wd", true),
    NodeCommandRegistration::new(NodeCommandKind::Invert, "invert", true),
    NodeCommandRegistration::new(NodeCommandKind::SingleCubeContainment, "_scc", true),
    NodeCommandRegistration::new(NodeCommandKind::Simplify, "_sim", false),
    NodeCommandRegistration::new(NodeCommandKind::Divide, "_div", false),
    NodeCommandRegistration::new(NodeCommandKind::Cofactor, "_cof", false),
];

pub const DEFAULT_CUBE_SIZE: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeCommandKind {
    WeakDivision,
    Invert,
    SingleCubeContainment,
    Simplify,
    Divide,
    Cofactor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeCommandRegistration {
    pub kind: NodeCommandKind,
    pub name: &'static str,
    pub changes_network: bool,
}

impl NodeCommandRegistration {
    pub const fn new(kind: NodeCommandKind, name: &'static str, changes_network: bool) -> Self {
        Self {
            kind,
            name,
            changes_network,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeCommand {
    WeakDivision(WeakDivisionOptions),
    Invert(NodeSelection),
    SingleCubeContainment(NodeSelection),
    Simplify(SimplifyOptions),
    Divide(NodePair),
    Cofactor(NodePair),
}

impl NodeCommand {
    pub fn kind(&self) -> NodeCommandKind {
        match self {
            Self::WeakDivision(_) => NodeCommandKind::WeakDivision,
            Self::Invert(_) => NodeCommandKind::Invert,
            Self::SingleCubeContainment(_) => NodeCommandKind::SingleCubeContainment,
            Self::Simplify(_) => NodeCommandKind::Simplify,
            Self::Divide(_) => NodeCommandKind::Divide,
            Self::Cofactor(_) => NodeCommandKind::Cofactor,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeSelection {
    pub names: Vec<String>,
}

impl NodeSelection {
    pub fn new(names: Vec<String>) -> Self {
        Self { names }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodePair {
    pub first: String,
    pub second: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeakDivisionOptions {
    pub pair: NodePair,
    pub use_complement: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplifyOptions {
    pub selection: NodeSelection,
    pub mode: SimplifyMode,
}

impl Default for SimplifyOptions {
    fn default() -> Self {
        Self {
            selection: NodeSelection::default(),
            mode: SimplifyMode::Espresso,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodePackageInitialization {
    pub commands: Vec<NodeCommandRegistration>,
    pub cube_size: usize,
    pub espresso_flags: bool,
    pub long_names: bool,
}

impl Default for NodePackageInitialization {
    fn default() -> Self {
        Self {
            commands: NODE_COMMANDS.to_vec(),
            cube_size: DEFAULT_CUBE_SIZE,
            espresso_flags: true,
            long_names: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeCommandOutput {
    None,
    Division { quotient: Node, remainder: Node },
    Cofactor(Node),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeCommandError {
    UnknownCommand(String),
    MissingOptionValue(char),
    InvalidSimplifyMode(String),
    UnsupportedOption(String),
    WrongArity {
        command: NodeCommandKind,
        expected: usize,
        actual: usize,
    },
    EmptyNodeList(NodeCommandKind),
    MissingNode(String),
    Node(NodeError),
    MissingNativeDependency {
        command: NodeCommandKind,
        operation: &'static str,
    },
    InvalidCofactorCondition,
}

impl fmt::Display for NodeCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(f, "unknown node command {command}"),
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::InvalidSimplifyMode(mode) => write!(f, "unknown simplify mode {mode}"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::WrongArity {
                command,
                expected,
                actual,
            } => write!(
                f,
                "{command:?} expects {expected} node operands, got {actual}"
            ),
            Self::EmptyNodeList(command) => write!(f, "{command:?} expects at least one node"),
            Self::MissingNode(name) => write!(f, "node {name} was not found"),
            Self::Node(error) => write!(f, "{error}"),
            Self::MissingNativeDependency { command, operation } => {
                write!(f, "{command:?} requires unavailable native {operation}")
            }
            Self::InvalidCofactorCondition => write!(f, "cofactor condition must be one cube"),
        }
    }
}

impl Error for NodeCommandError {}

impl From<NodeError> for NodeCommandError {
    fn from(value: NodeError) -> Self {
        Self::Node(value)
    }
}

pub fn node_command_registrations() -> &'static [NodeCommandRegistration] {
    NODE_COMMANDS
}

pub fn node_package_initialization() -> NodePackageInitialization {
    NodePackageInitialization::default()
}

pub fn node_package_shutdown_discards_daemons_and_cube_size() -> bool {
    true
}

pub fn parse_node_command<I, S>(name: &str, args: I) -> Result<NodeCommand, NodeCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();

    match name {
        "wd" => parse_weak_division_args(args).map(NodeCommand::WeakDivision),
        "invert" => {
            parse_non_empty_selection(NodeCommandKind::Invert, args).map(NodeCommand::Invert)
        }
        "_scc" => parse_non_empty_selection(NodeCommandKind::SingleCubeContainment, args)
            .map(NodeCommand::SingleCubeContainment),
        "_sim" => parse_simplify_args(args).map(NodeCommand::Simplify),
        "_div" => parse_pair(NodeCommandKind::Divide, args).map(NodeCommand::Divide),
        "_cof" => parse_pair(NodeCommandKind::Cofactor, args).map(NodeCommand::Cofactor),
        _ => Err(NodeCommandError::UnknownCommand(name.to_owned())),
    }
}

pub fn execute_node_command(
    nodes: &mut BTreeMap<String, Node>,
    command: &NodeCommand,
) -> Result<NodeCommandOutput, NodeCommandError> {
    match command {
        NodeCommand::WeakDivision(_) => Err(NodeCommandError::MissingNativeDependency {
            command: NodeCommandKind::WeakDivision,
            operation: "substitution",
        }),
        NodeCommand::Invert(selection) => {
            for name in &selection.names {
                let replacement = {
                    let node = get_node(nodes, name)?;
                    node_not(node)?
                };
                let node = get_node_mut(nodes, name)?;
                *node = replacement;
            }
            Ok(NodeCommandOutput::None)
        }
        NodeCommand::SingleCubeContainment(selection) => {
            for name in &selection.names {
                node_scc(get_node_mut(nodes, name)?);
            }
            Ok(NodeCommandOutput::None)
        }
        NodeCommand::Simplify(options) => {
            for name in &options.selection.names {
                let node = get_node_mut(nodes, name)?;
                if matches!(
                    node_function(node)?,
                    NodeFunction::PrimaryInput | NodeFunction::PrimaryOutput
                ) {
                    continue;
                }
                node_simplify_replace(node, None, options.mode)?;
            }
            Ok(NodeCommandOutput::None)
        }
        NodeCommand::Divide(_) => Err(NodeCommandError::MissingNativeDependency {
            command: NodeCommandKind::Divide,
            operation: "division",
        }),
        NodeCommand::Cofactor(pair) => {
            let node = get_node(nodes, &pair.first)?;
            let condition = get_node(nodes, &pair.second)?;
            Ok(NodeCommandOutput::Cofactor(cofactor_node(node, condition)?))
        }
    }
}

pub fn cofactor_node(node: &Node, condition: &Node) -> Result<Node, NodeCommandError> {
    let function = node.function().ok_or(NodeError::MissingFunction {
        operation: "cofactor",
    })?;
    let condition_function = condition.function().ok_or(NodeError::MissingFunction {
        operation: "cofactor",
    })?;

    if condition_function.is_zero() {
        return Err(NodeCommandError::InvalidCofactorCondition);
    }

    if condition_function.is_one() {
        return Ok(node.clone());
    }

    if condition_function.cube_count() != 1 {
        return Err(NodeCommandError::InvalidCofactorCondition);
    }

    let mut fanins = node.fanins.clone();
    for fanin in &condition.fanins {
        if !fanins.contains(fanin) {
            fanins.push(fanin.clone());
        }
    }

    let condition_cube = condition_function.cubes()[0].inputs();
    let mut cubes = Vec::new();
    for cube in function.cubes() {
        let mut aligned = align_inputs(cube.inputs(), &node.fanins, &fanins);
        if is_compatible(&aligned, condition_cube, &condition.fanins, &fanins) {
            for (index, condition_fanin) in condition.fanins.iter().enumerate() {
                if condition_cube[index].is_some() {
                    let target = fanins
                        .iter()
                        .position(|fanin| fanin == condition_fanin)
                        .expect("condition fanin was aligned");
                    aligned[target] = None;
                }
            }
            cubes.push(Cube::new(aligned));
        }
    }

    Ok(Node::new(Cover::new(fanins.len(), cubes)?, fanins))
}

fn parse_weak_division_args(args: Vec<String>) -> Result<WeakDivisionOptions, NodeCommandError> {
    let mut use_complement = false;
    let mut operands = Vec::new();

    for arg in args {
        if arg == "-c" {
            use_complement = true;
        } else if arg.starts_with('-') {
            return Err(NodeCommandError::UnsupportedOption(arg));
        } else {
            operands.push(arg);
        }
    }

    let pair = parse_pair(NodeCommandKind::WeakDivision, operands)?;
    Ok(WeakDivisionOptions {
        pair,
        use_complement,
    })
}

fn parse_simplify_args(args: Vec<String>) -> Result<SimplifyOptions, NodeCommandError> {
    let mut options = SimplifyOptions::default();
    let mut operands = Vec::new();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        if arg == "-m" {
            index += 1;
            let mode = args
                .get(index)
                .ok_or(NodeCommandError::MissingOptionValue('m'))?;
            options.mode = parse_simplify_mode(mode)?;
        } else if let Some(mode) = arg.strip_prefix("-m") {
            if mode.is_empty() {
                return Err(NodeCommandError::MissingOptionValue('m'));
            }
            options.mode = parse_simplify_mode(mode)?;
        } else if arg.starts_with('-') {
            return Err(NodeCommandError::UnsupportedOption(arg.clone()));
        } else {
            operands.push(arg.clone());
        }

        index += 1;
    }

    options.selection = NodeSelection::new(operands);
    Ok(options)
}

fn parse_simplify_mode(mode: &str) -> Result<SimplifyMode, NodeCommandError> {
    match mode {
        "simpcomp" => Ok(SimplifyMode::SimpleComplement),
        "espresso" => Ok(SimplifyMode::Espresso),
        "exact" => Ok(SimplifyMode::Exact),
        "exact-lits" => Ok(SimplifyMode::ExactLiterals),
        _ => Err(NodeCommandError::InvalidSimplifyMode(mode.to_owned())),
    }
}

fn parse_non_empty_selection(
    command: NodeCommandKind,
    args: Vec<String>,
) -> Result<NodeSelection, NodeCommandError> {
    if args.is_empty() {
        return Err(NodeCommandError::EmptyNodeList(command));
    }

    Ok(NodeSelection::new(args))
}

fn parse_pair(command: NodeCommandKind, args: Vec<String>) -> Result<NodePair, NodeCommandError> {
    if args.len() != 2 {
        return Err(NodeCommandError::WrongArity {
            command,
            expected: 2,
            actual: args.len(),
        });
    }

    Ok(NodePair {
        first: args[0].clone(),
        second: args[1].clone(),
    })
}

fn get_node<'a>(
    nodes: &'a BTreeMap<String, Node>,
    name: &str,
) -> Result<&'a Node, NodeCommandError> {
    nodes
        .get(name)
        .ok_or_else(|| NodeCommandError::MissingNode(name.to_owned()))
}

fn get_node_mut<'a>(
    nodes: &'a mut BTreeMap<String, Node>,
    name: &str,
) -> Result<&'a mut Node, NodeCommandError> {
    nodes
        .get_mut(name)
        .ok_or_else(|| NodeCommandError::MissingNode(name.to_owned()))
}

fn align_inputs(
    inputs: &[Option<bool>],
    old_fanins: &[String],
    fanins: &[String],
) -> Vec<Option<bool>> {
    let mut aligned = vec![None; fanins.len()];
    for (old_index, old_fanin) in old_fanins.iter().enumerate() {
        if let Some(new_index) = fanins.iter().position(|fanin| fanin == old_fanin) {
            aligned[new_index] = inputs[old_index];
        }
    }
    aligned
}

fn is_compatible(
    cube: &[Option<bool>],
    condition_cube: &[Option<bool>],
    condition_fanins: &[String],
    fanins: &[String],
) -> bool {
    condition_cube
        .iter()
        .zip(condition_fanins)
        .all(|(condition_phase, condition_fanin)| {
            let Some(condition_phase) = condition_phase else {
                return true;
            };
            let Some(index) = fanins.iter().position(|fanin| fanin == condition_fanin) else {
                return true;
            };
            cube[index].map_or(true, |phase| phase == *condition_phase)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use native_node::{node_and, node_constant, node_equal_by_name, node_literal, node_num_cube};

    fn lit(name: &str, phase: i32) -> Node {
        node_literal(name, phase).unwrap()
    }

    fn sample_nodes() -> BTreeMap<String, Node> {
        let a = lit("a", 1);
        let b = lit("b", 1);
        let ab = node_and(&a, &b).unwrap();
        BTreeMap::from([
            ("a".to_owned(), a),
            ("b".to_owned(), b),
            ("ab".to_owned(), ab),
        ])
    }

    #[test]
    fn command_registrations_match_legacy_node_commands() {
        let registrations = node_command_registrations();

        assert_eq!(registrations.len(), 6);
        assert_eq!(registrations[0].name, "wd");
        assert_eq!(registrations[1].name, "invert");
        assert!(registrations[0].changes_network);
        assert!(!registrations[3].changes_network);

        let initialization = node_package_initialization();
        assert_eq!(initialization.cube_size, DEFAULT_CUBE_SIZE);
        assert!(initialization.espresso_flags);
        assert!(initialization.long_names);
        assert!(node_package_shutdown_discards_daemons_and_cube_size());
    }

    #[test]
    fn parses_simplify_modes_and_node_lists() {
        let command = parse_node_command("_sim", ["-m", "simpcomp", "n1", "n2"]).unwrap();

        assert_eq!(
            command,
            NodeCommand::Simplify(SimplifyOptions {
                selection: NodeSelection::new(vec!["n1".to_owned(), "n2".to_owned()]),
                mode: SimplifyMode::SimpleComplement,
            })
        );
        assert_eq!(
            parse_node_command("_sim", ["-mexact-lits"]).unwrap(),
            NodeCommand::Simplify(SimplifyOptions {
                selection: NodeSelection::default(),
                mode: SimplifyMode::ExactLiterals,
            })
        );
        assert_eq!(
            parse_node_command("_sim", ["-m", "fast"]).unwrap_err(),
            NodeCommandError::InvalidSimplifyMode("fast".to_owned())
        );
    }

    #[test]
    fn parses_pair_commands_and_rejects_wrong_arity() {
        assert_eq!(
            parse_node_command("wd", ["-c", "n1", "n2"]).unwrap(),
            NodeCommand::WeakDivision(WeakDivisionOptions {
                pair: NodePair {
                    first: "n1".to_owned(),
                    second: "n2".to_owned(),
                },
                use_complement: true,
            })
        );
        assert_eq!(
            parse_node_command("_cof", ["n1"]).unwrap_err(),
            NodeCommandError::WrongArity {
                command: NodeCommandKind::Cofactor,
                expected: 2,
                actual: 1,
            }
        );
        assert_eq!(
            parse_node_command("invert", Vec::<String>::new()).unwrap_err(),
            NodeCommandError::EmptyNodeList(NodeCommandKind::Invert)
        );
    }

    #[test]
    fn invert_and_scc_execute_over_named_nodes() {
        let mut nodes = sample_nodes();

        execute_node_command(
            &mut nodes,
            &NodeCommand::Invert(NodeSelection::new(vec!["a".to_owned()])),
        )
        .unwrap();
        assert_eq!(
            node_function(nodes.get("a").unwrap()).unwrap(),
            NodeFunction::Inverter
        );

        execute_node_command(
            &mut nodes,
            &NodeCommand::SingleCubeContainment(NodeSelection::new(vec!["ab".to_owned()])),
        )
        .unwrap();
        assert!(!nodes.get("ab").unwrap().is_dup_free);
    }

    #[test]
    fn simplify_skips_boundary_nodes_and_reports_unavailable_modes() {
        let mut nodes = sample_nodes();
        nodes.insert("pi".to_owned(), Node::primary_input("pi"));

        execute_node_command(
            &mut nodes,
            &NodeCommand::Simplify(SimplifyOptions {
                selection: NodeSelection::new(vec!["pi".to_owned(), "ab".to_owned()]),
                mode: SimplifyMode::SimpleComplement,
            }),
        )
        .unwrap();

        let error = execute_node_command(
            &mut nodes,
            &NodeCommand::Simplify(SimplifyOptions {
                selection: NodeSelection::new(vec!["ab".to_owned()]),
                mode: SimplifyMode::Espresso,
            }),
        )
        .unwrap_err();
        assert_eq!(
            error,
            NodeCommandError::Node(NodeError::NativeSupportUnavailable {
                operation: "cover minimization",
            })
        );
    }

    #[test]
    fn cofactor_removes_condition_literals_from_matching_cubes() {
        let mut nodes = sample_nodes();
        let command = NodeCommand::Cofactor(NodePair {
            first: "ab".to_owned(),
            second: "a".to_owned(),
        });

        let output = execute_node_command(&mut nodes, &command).unwrap();
        let NodeCommandOutput::Cofactor(cofactor) = output else {
            panic!("expected cofactor output");
        };

        assert_eq!(cofactor.fanins, vec!["b"]);
        assert_eq!(node_function(&cofactor).unwrap(), NodeFunction::Buffer);
        assert!(node_equal_by_name(nodes.get("b").unwrap(), &cofactor).unwrap());
    }

    #[test]
    fn cofactor_rejects_zero_and_multi_cube_conditions() {
        let a = lit("a", 1);
        let b = lit("b", 1);
        let ab = node_and(&a, &b).unwrap();
        let zero = node_constant(0).unwrap();
        let multi = native_node::node_or(&a, &b).unwrap();

        assert_eq!(
            cofactor_node(&ab, &zero).unwrap_err(),
            NodeCommandError::InvalidCofactorCondition
        );
        assert_eq!(
            cofactor_node(&ab, &multi).unwrap_err(),
            NodeCommandError::InvalidCofactorCondition
        );
    }

    #[test]
    fn divide_and_weak_division_report_pending_native_operations() {
        let mut nodes = sample_nodes();

        assert_eq!(
            execute_node_command(
                &mut nodes,
                &NodeCommand::Divide(NodePair {
                    first: "ab".to_owned(),
                    second: "a".to_owned(),
                }),
            )
            .unwrap_err(),
            NodeCommandError::MissingNativeDependency {
                command: NodeCommandKind::Divide,
                operation: "division",
            }
        );
        assert_eq!(
            execute_node_command(
                &mut nodes,
                &NodeCommand::WeakDivision(WeakDivisionOptions {
                    pair: NodePair {
                        first: "ab".to_owned(),
                        second: "a".to_owned(),
                    },
                    use_complement: false,
                }),
            )
            .unwrap_err(),
            NodeCommandError::MissingNativeDependency {
                command: NodeCommandKind::WeakDivision,
                operation: "substitution",
            }
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let source = include_str!("com_node.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-")));
        assert_eq!(node_num_cube(&lit("a", 1)).unwrap(), 1);
    }
}
