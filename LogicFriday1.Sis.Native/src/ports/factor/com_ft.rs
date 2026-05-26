//! Native command model for the SIS factor command package.
//!
//! The legacy command file registers the factor commands, parses their small
//! option sets, delegates factoring/elimination to lower-level routines, and
//! formats factored-form diagnostics. This port keeps those command semantics
//! over owned Rust data with backend hooks for the algebraic transformations.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateFunction {
    And,
    Or,
    Buffer,
    Inverter,
    Zero,
    One,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateNode {
    pub name: String,
    pub function: GateFunction,
    pub fanins: Vec<String>,
}

impl GateNode {
    pub fn new(
        name: impl Into<String>,
        function: GateFunction,
        fanins: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            function,
            fanins: fanins.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorTree {
    Empty,
    Zero,
    One,
    Leaf(String),
    Inverter(Box<FactorTree>),
    And(Vec<FactorTree>),
    Or(Vec<FactorTree>),
}

impl FactorTree {
    pub fn leaf(name: impl Into<String>) -> Self {
        Self::Leaf(name.into())
    }

    pub fn and(children: impl IntoIterator<Item = FactorTree>) -> Self {
        Self::And(children.into_iter().collect())
    }

    pub fn or(children: impl IntoIterator<Item = FactorTree>) -> Self {
        Self::Or(children.into_iter().collect())
    }

    pub fn inverter(child: FactorTree) -> Self {
        Self::Inverter(Box::new(child))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorNode {
    pub id: NodeId,
    pub name: String,
    pub short_name: Option<String>,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub factor: Option<FactorTree>,
    pub value: Option<i32>,
    pub converted_gates: Vec<GateNode>,
}

impl FactorNode {
    pub fn new(id: usize, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            short_name: None,
            kind,
            fanins: Vec::new(),
            factor: None,
            value: None,
            converted_gates: Vec::new(),
        }
    }

    pub fn with_short_name(mut self, short_name: impl Into<String>) -> Self {
        self.short_name = Some(short_name.into());
        self
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_factor(mut self, factor: FactorTree) -> Self {
        self.factor = Some(factor);
        self
    }

    pub fn with_value(mut self, value: i32) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_converted_gates(mut self, gates: impl IntoIterator<Item = GateNode>) -> Self {
        self.converted_gates = gates.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FactorCommandNetwork {
    nodes: BTreeMap<NodeId, FactorNode>,
    names: BTreeMap<String, NodeId>,
}

impl FactorCommandNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: FactorNode) -> Result<(), FactorCommandError> {
        if self.nodes.contains_key(&node.id) {
            return Err(FactorCommandError::DuplicateNodeId(node.id));
        }

        if self.names.contains_key(&node.name) {
            return Err(FactorCommandError::DuplicateNodeName(node.name));
        }

        self.names.insert(node.name.clone(), node.id);
        self.nodes.insert(node.id, node);
        Ok(())
    }

    pub fn node(&self, node: NodeId) -> Result<&FactorNode, FactorCommandError> {
        self.nodes
            .get(&node)
            .ok_or(FactorCommandError::UnknownNodeId(node))
    }

    pub fn node_mut(&mut self, node: NodeId) -> Result<&mut FactorNode, FactorCommandError> {
        self.nodes
            .get_mut(&node)
            .ok_or(FactorCommandError::UnknownNodeId(node))
    }

    pub fn id_by_name(&self, name: &str) -> Result<NodeId, FactorCommandError> {
        self.names
            .get(name)
            .copied()
            .ok_or_else(|| FactorCommandError::UnknownNodeName(name.to_owned()))
    }

    pub fn select_nodes<I, S>(&self, names: I) -> Result<Vec<NodeId>, FactorCommandError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        names
            .into_iter()
            .map(|name| self.id_by_name(name.as_ref()))
            .collect()
    }

    pub fn all_nodes(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactorCommandKind {
    PrintValue,
    PrintFactor,
    Factor,
    Eliminate,
    PrintFactorTree,
    ConvertFactor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: FactorCommandKind,
    pub changes_network: bool,
}

pub const FACTOR_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "print_value",
        kind: FactorCommandKind::PrintValue,
        changes_network: false,
    },
    CommandRegistration {
        name: "print_factor",
        kind: FactorCommandKind::PrintFactor,
        changes_network: false,
    },
    CommandRegistration {
        name: "factor",
        kind: FactorCommandKind::Factor,
        changes_network: true,
    },
    CommandRegistration {
        name: "eliminate",
        kind: FactorCommandKind::Eliminate,
        changes_network: true,
    },
    CommandRegistration {
        name: "_pft",
        kind: FactorCommandKind::PrintFactorTree,
        changes_network: false,
    },
    CommandRegistration {
        name: "_conv",
        kind: FactorCommandKind::ConvertFactor,
        changes_network: false,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactorMethod {
    Quick,
    Good,
    Boolean,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueOrder {
    Any,
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameMode {
    Long,
    Short,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorCommand {
    Factor {
        method: FactorMethod,
        nodes: Vec<String>,
    },
    PrintFactor {
        nodes: Vec<String>,
    },
    PrintValue {
        order: ValueOrder,
        limit: usize,
        nodes: Vec<String>,
    },
    PrintFactorTree {
        nodes: Vec<String>,
    },
    ConvertFactor {
        nodes: Vec<String>,
        name_mode: NameMode,
    },
    Eliminate {
        value: i32,
        limit: i32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorCommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl FactorCommandOutput {
    pub fn success(stdout: String) -> Self {
        Self {
            status: 0,
            stdout,
            stderr: String::new(),
        }
    }

    pub fn failure(stderr: impl Into<String>) -> Self {
        Self {
            status: 1,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorCommandError {
    UnknownCommand(String),
    UnsupportedOption(String),
    MissingOptionValue(char),
    Usage(FactorCommandKind),
    InvalidInteger(String),
    DuplicateNodeId(NodeId),
    DuplicateNodeName(String),
    UnknownNodeId(NodeId),
    UnknownNodeName(String),
    MissingFanin { node: NodeId },
    Backend(String),
}

impl fmt::Display for FactorCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(formatter, "unknown factor command {command}"),
            Self::UnsupportedOption(option) => write!(formatter, "unsupported option {option}"),
            Self::MissingOptionValue(option) => write!(formatter, "missing value for -{option}"),
            Self::Usage(command) => write!(formatter, "{}", usage(*command)),
            Self::InvalidInteger(value) => write!(formatter, "invalid integer value {value}"),
            Self::DuplicateNodeId(node) => write!(formatter, "duplicate factor node id {}", node.0),
            Self::DuplicateNodeName(name) => write!(formatter, "duplicate factor node name {name}"),
            Self::UnknownNodeId(node) => write!(formatter, "unknown factor node id {}", node.0),
            Self::UnknownNodeName(name) => write!(formatter, "unknown factor node {name}"),
            Self::MissingFanin { node } => {
                write!(formatter, "primary output node {} has no fanin", node.0)
            }
            Self::Backend(message) => formatter.write_str(message),
        }
    }
}

impl Error for FactorCommandError {}

pub trait FactorCommandBackend {
    fn factor_node(
        &mut self,
        network: &mut FactorCommandNetwork,
        node: NodeId,
        method: FactorMethod,
    ) -> Result<(), FactorCommandError>;

    fn eliminate(
        &mut self,
        network: &mut FactorCommandNetwork,
        value: i32,
        limit: i32,
    ) -> Result<(), FactorCommandError>;
}

pub struct NoopFactorBackend;

impl FactorCommandBackend for NoopFactorBackend {
    fn factor_node(
        &mut self,
        _network: &mut FactorCommandNetwork,
        _node: NodeId,
        _method: FactorMethod,
    ) -> Result<(), FactorCommandError> {
        Err(FactorCommandError::Backend(
            "native factor backend is not connected".to_owned(),
        ))
    }

    fn eliminate(
        &mut self,
        _network: &mut FactorCommandNetwork,
        _value: i32,
        _limit: i32,
    ) -> Result<(), FactorCommandError> {
        Err(FactorCommandError::Backend(
            "native eliminate backend is not connected".to_owned(),
        ))
    }
}

pub fn factor_command_registrations() -> &'static [CommandRegistration] {
    FACTOR_COMMANDS
}

pub fn parse_factor_command<I, S>(
    command_name: &str,
    args: I,
) -> Result<FactorCommand, FactorCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    match command_name {
        "factor" => parse_factor_args(&args),
        "print_factor" => Ok(FactorCommand::PrintFactor { nodes: args }),
        "print_value" => parse_print_value_args(&args),
        "_pft" => Ok(FactorCommand::PrintFactorTree { nodes: args }),
        "_conv" => Ok(FactorCommand::ConvertFactor {
            nodes: args,
            name_mode: NameMode::Long,
        }),
        "eliminate" => parse_eliminate_args(&args),
        _ => Err(FactorCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn dispatch_factor_command<B>(
    backend: &mut B,
    network: &mut FactorCommandNetwork,
    command: &FactorCommand,
) -> Result<FactorCommandOutput, FactorCommandError>
where
    B: FactorCommandBackend,
{
    match command {
        FactorCommand::Factor { method, nodes } => run_factor(backend, network, *method, nodes),
        FactorCommand::PrintFactor { nodes } => {
            Ok(FactorCommandOutput::success(print_factor(network, nodes)?))
        }
        FactorCommand::PrintValue {
            order,
            limit,
            nodes,
        } => Ok(FactorCommandOutput::success(print_values(
            network, nodes, *order, *limit,
        )?)),
        FactorCommand::PrintFactorTree { nodes } => Ok(FactorCommandOutput::success(
            print_factor_trees(network, nodes)?,
        )),
        FactorCommand::ConvertFactor { nodes, name_mode } => Ok(FactorCommandOutput::success(
            convert_factor(network, nodes, *name_mode)?,
        )),
        FactorCommand::Eliminate { value, limit } => {
            backend.eliminate(network, *value, *limit)?;
            Ok(FactorCommandOutput::success(String::new()))
        }
    }
}

pub fn factor_usage_text() -> &'static str {
    "usage: factor [-qgb] node-list\n    -q\t\tQuick factoring (default)\n    -g\t\tGood factoring\n    -b\t\tBoolean factoring\n"
}

pub fn usage(command: FactorCommandKind) -> &'static str {
    match command {
        FactorCommandKind::Factor => factor_usage_text(),
        FactorCommandKind::Eliminate => "usage: eliminate [-l limit] value\n",
        FactorCommandKind::PrintValue => {
            "usage: print_value [-p n] [-a|-d] [node-list]\n    -p\tn\tOnly print top 'n' values\n    -d\t\tPrint values in descending order\n    -a\t\tPrint values in ascending order\n"
        }
        FactorCommandKind::PrintFactor
        | FactorCommandKind::PrintFactorTree
        | FactorCommandKind::ConvertFactor => "usage: factor command [node-list]\n",
    }
}

fn parse_factor_args(args: &[String]) -> Result<FactorCommand, FactorCommandError> {
    let mut method = FactorMethod::Quick;
    let mut index = 0;

    while let Some(arg) = args.get(index) {
        if !arg.starts_with('-') || arg == "-" {
            break;
        }

        for option in arg[1..].chars() {
            method = match option {
                'q' => FactorMethod::Quick,
                'g' => FactorMethod::Good,
                'b' => FactorMethod::Boolean,
                _ => return Err(FactorCommandError::UnsupportedOption(format!("-{option}"))),
            };
        }

        index += 1;
    }

    if index == args.len() {
        return Err(FactorCommandError::Usage(FactorCommandKind::Factor));
    }

    Ok(FactorCommand::Factor {
        method,
        nodes: args[index..].to_vec(),
    })
}

fn parse_print_value_args(args: &[String]) -> Result<FactorCommand, FactorCommandError> {
    let mut order = ValueOrder::Any;
    let mut limit = usize::MAX;
    let mut index = 0;

    while let Some(arg) = args.get(index) {
        if !arg.starts_with('-') || arg == "-" {
            break;
        }

        match arg.as_str() {
            "-a" => {
                order = ValueOrder::Ascending;
                index += 1;
            }
            "-d" => {
                order = ValueOrder::Descending;
                index += 1;
            }
            "-p" => {
                let value = args
                    .get(index + 1)
                    .ok_or(FactorCommandError::MissingOptionValue('p'))?;
                limit = parse_usize(value)?;
                index += 2;
            }
            _ if arg.starts_with("-p") && arg.len() > 2 => {
                limit = parse_usize(&arg[2..])?;
                index += 1;
            }
            _ => return Err(FactorCommandError::UnsupportedOption(arg.clone())),
        }
    }

    Ok(FactorCommand::PrintValue {
        order,
        limit,
        nodes: args[index..].to_vec(),
    })
}

fn parse_eliminate_args(args: &[String]) -> Result<FactorCommand, FactorCommandError> {
    match args {
        [value] => Ok(FactorCommand::Eliminate {
            value: parse_i32(value)?,
            limit: 1000,
        }),
        [flag, limit, value] if flag == "-l" => Ok(FactorCommand::Eliminate {
            value: parse_i32(value)?,
            limit: parse_i32(limit)?,
        }),
        _ => Err(FactorCommandError::Usage(FactorCommandKind::Eliminate)),
    }
}

fn parse_i32(value: &str) -> Result<i32, FactorCommandError> {
    if value.is_empty() {
        return Err(FactorCommandError::InvalidInteger(value.to_owned()));
    }

    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty() || !digits.chars().all(|character| character.is_ascii_digit()) {
        return Err(FactorCommandError::InvalidInteger(value.to_owned()));
    }

    value
        .parse()
        .map_err(|_| FactorCommandError::InvalidInteger(value.to_owned()))
}

fn parse_usize(value: &str) -> Result<usize, FactorCommandError> {
    if value.is_empty() || !value.chars().all(|character| character.is_ascii_digit()) {
        return Err(FactorCommandError::InvalidInteger(value.to_owned()));
    }

    value
        .parse()
        .map_err(|_| FactorCommandError::InvalidInteger(value.to_owned()))
}

fn run_factor<B>(
    backend: &mut B,
    network: &mut FactorCommandNetwork,
    method: FactorMethod,
    node_names: &[String],
) -> Result<FactorCommandOutput, FactorCommandError>
where
    B: FactorCommandBackend,
{
    if method == FactorMethod::Boolean {
        return Ok(FactorCommandOutput::failure(
            "boolean factoring is not available\n",
        ));
    }

    let nodes = network.select_nodes(node_names)?;
    for node in nodes {
        if network.node(node)?.kind == NodeKind::Internal {
            backend.factor_node(network, node, method)?;
        }
    }

    Ok(FactorCommandOutput::success(String::new()))
}

fn selected_or_all(
    network: &FactorCommandNetwork,
    node_names: &[String],
) -> Result<Vec<NodeId>, FactorCommandError> {
    if node_names.is_empty() {
        Ok(network.all_nodes())
    } else {
        network.select_nodes(node_names)
    }
}

pub fn print_factor(
    network: &FactorCommandNetwork,
    node_names: &[String],
) -> Result<String, FactorCommandError> {
    let mut output = String::new();
    for node_id in selected_or_all(network, node_names)? {
        let node = network.node(node_id)?;
        match node.kind {
            NodeKind::PrimaryInput => {}
            NodeKind::PrimaryOutput => {
                if let Some(fanin) = node.fanins.first() {
                    let fanin = network.node(*fanin)?;
                    if fanin.kind == NodeKind::PrimaryInput {
                        output.push_str(&format!("    {} = {}\n", node.name, fanin.name));
                    }
                } else {
                    return Err(FactorCommandError::MissingFanin { node: node_id });
                }
            }
            NodeKind::Internal => {
                output.push_str(&format!(
                    "    {} = {}\n",
                    node.name,
                    factor_expression(node)
                ));
            }
        }
    }

    Ok(output)
}

pub fn print_values(
    network: &FactorCommandNetwork,
    node_names: &[String],
    order: ValueOrder,
    limit: usize,
) -> Result<String, FactorCommandError> {
    let mut nodes = selected_or_all(network, node_names)?;
    match order {
        ValueOrder::Any => {}
        ValueOrder::Ascending => {
            nodes.sort_by(|left, right| compare_node_value(network, *left, *right))
        }
        ValueOrder::Descending => {
            nodes.sort_by(|left, right| compare_node_value(network, *right, *left))
        }
    }

    let mut output = String::new();
    let mut printed = 0usize;
    for node_id in nodes {
        if printed >= limit {
            break;
        }

        let node = network.node(node_id)?;
        if matches!(node.kind, NodeKind::PrimaryInput | NodeKind::PrimaryOutput) {
            continue;
        }

        output.push_str(&format!(
            "{}:\t{}\n",
            node.name,
            node.value
                .map(|value| value.to_string())
                .unwrap_or_else(|| "(inf)".to_owned())
        ));
        printed += 1;
    }

    Ok(output)
}

pub fn print_factor_trees(
    network: &FactorCommandNetwork,
    node_names: &[String],
) -> Result<String, FactorCommandError> {
    let mut output = String::new();
    for node_id in selected_or_all(network, node_names)? {
        let node = network.node(node_id)?;
        if matches!(node.kind, NodeKind::PrimaryInput | NodeKind::PrimaryOutput) {
            continue;
        }

        output.push_str(&format!("--- {} ---\n", node.name));
        if let Some(factor) = &node.factor {
            append_factor_tree_lines(factor, 0, &mut output);
        } else {
            output.push_str("empty.\n");
        }
    }

    Ok(output)
}

pub fn convert_factor(
    network: &FactorCommandNetwork,
    node_names: &[String],
    name_mode: NameMode,
) -> Result<String, FactorCommandError> {
    let mut output = String::new();
    for node_id in selected_or_all(network, node_names)? {
        let node = network.node(node_id)?;
        if node.kind != NodeKind::Internal {
            continue;
        }

        output.push_str(&format!(
            "    {} = {}\n",
            node.name,
            factor_expression(node)
        ));
        for gate in &node.converted_gates {
            output.push_str(&format_gate(gate, name_mode));
        }
        output.push('\n');
    }

    Ok(output)
}

fn factor_expression(node: &FactorNode) -> String {
    node.factor
        .as_ref()
        .map(format_factor)
        .unwrap_or_else(|| "empty".to_owned())
}

fn format_factor(factor: &FactorTree) -> String {
    match factor {
        FactorTree::Empty => "empty".to_owned(),
        FactorTree::Zero => "0".to_owned(),
        FactorTree::One => "1".to_owned(),
        FactorTree::Leaf(name) => name.clone(),
        FactorTree::Inverter(child) => format!("!{}", format_factor_operand(child)),
        FactorTree::And(children) => join_children(children, " * "),
        FactorTree::Or(children) => join_children(children, " + "),
    }
}

fn format_factor_operand(factor: &FactorTree) -> String {
    match factor {
        FactorTree::Leaf(_) | FactorTree::Zero | FactorTree::One => format_factor(factor),
        FactorTree::Inverter(child)
            if matches!(
                child.as_ref(),
                FactorTree::Leaf(_) | FactorTree::Zero | FactorTree::One
            ) =>
        {
            format_factor(factor)
        }
        _ => format!("({})", format_factor(factor)),
    }
}

fn join_children(children: &[FactorTree], separator: &str) -> String {
    children
        .iter()
        .map(format_factor_operand)
        .collect::<Vec<_>>()
        .join(separator)
}

fn append_factor_tree_lines(factor: &FactorTree, level: usize, output: &mut String) {
    output.push_str(&format!("{}: {}\t", level, factor_tree_name(factor)));
    for child in factor_children(factor) {
        output.push_str(&format!("\t{}", factor_tree_name(child)));
    }
    output.push('\n');

    for child in factor_children(factor) {
        append_factor_tree_lines(child, level + 1, output);
    }
}

fn factor_children(factor: &FactorTree) -> &[FactorTree] {
    match factor {
        FactorTree::Inverter(child) => std::slice::from_ref(child),
        FactorTree::And(children) | FactorTree::Or(children) => children,
        FactorTree::Empty | FactorTree::Zero | FactorTree::One | FactorTree::Leaf(_) => &[],
    }
}

fn factor_tree_name(factor: &FactorTree) -> String {
    match factor {
        FactorTree::Empty => "EMPTY".to_owned(),
        FactorTree::Zero => "ZERO".to_owned(),
        FactorTree::One => "ONE".to_owned(),
        FactorTree::Leaf(name) => name.clone(),
        FactorTree::Inverter(_) => "INV".to_owned(),
        FactorTree::And(_) => "AND".to_owned(),
        FactorTree::Or(_) => "OR".to_owned(),
    }
}

fn format_gate(gate: &GateNode, _name_mode: NameMode) -> String {
    match gate.function {
        GateFunction::And | GateFunction::Or => format!(
            "\t{} = {}({})\n",
            gate.name,
            gate_name(gate.function),
            format_gate_fanins(&gate.fanins)
        ),
        GateFunction::Buffer | GateFunction::Inverter => {
            let fanin = gate.fanins.first().cloned().unwrap_or_default();
            format!(
                "\t{} = {}({})\n",
                gate.name,
                gate_name(gate.function),
                fanin
            )
        }
        GateFunction::Zero | GateFunction::One => {
            format!("\t{} = {}\n", gate.name, gate_name(gate.function))
        }
    }
}

fn format_gate_fanins(fanins: &[String]) -> String {
    if fanins.is_empty() {
        String::new()
    } else {
        format!(" {}", fanins.join(" "))
    }
}

fn gate_name(function: GateFunction) -> &'static str {
    match function {
        GateFunction::And => "AND",
        GateFunction::Or => "OR",
        GateFunction::Buffer => "BUF",
        GateFunction::Inverter => "INV",
        GateFunction::Zero => "0",
        GateFunction::One => "1",
    }
}

fn compare_node_value(network: &FactorCommandNetwork, left: NodeId, right: NodeId) -> Ordering {
    let left_value = network.node(left).ok().and_then(|node| node.value);
    let right_value = network.node(right).ok().and_then(|node| node.value);

    match (left_value, right_value) {
        (Some(left_value), Some(right_value)) => left_value.cmp(&right_value),
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (None, None) => left.cmp(&right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        factored: Vec<(NodeId, FactorMethod)>,
        eliminated: Vec<(i32, i32)>,
    }

    impl FactorCommandBackend for RecordingBackend {
        fn factor_node(
            &mut self,
            network: &mut FactorCommandNetwork,
            node: NodeId,
            method: FactorMethod,
        ) -> Result<(), FactorCommandError> {
            self.factored.push((node, method));
            network.node_mut(node)?.factor = Some(FactorTree::leaf("factored"));
            Ok(())
        }

        fn eliminate(
            &mut self,
            _network: &mut FactorCommandNetwork,
            value: i32,
            limit: i32,
        ) -> Result<(), FactorCommandError> {
            self.eliminated.push((value, limit));
            Ok(())
        }
    }

    fn sample_network() -> FactorCommandNetwork {
        let a = FactorNode::new(0, "a", NodeKind::PrimaryInput);
        let b = FactorNode::new(1, "b", NodeKind::PrimaryInput);
        let f = FactorNode::new(2, "f", NodeKind::Internal)
            .with_fanins([NodeId(0), NodeId(1)])
            .with_factor(FactorTree::or([
                FactorTree::and([FactorTree::leaf("a"), FactorTree::leaf("b")]),
                FactorTree::inverter(FactorTree::leaf("b")),
            ]))
            .with_value(7)
            .with_converted_gates([
                GateNode::new("n1", GateFunction::And, ["a", "b"]),
                GateNode::new("n2", GateFunction::Inverter, ["b"]),
                GateNode::new("f", GateFunction::Or, ["n1", "n2"]),
            ]);
        let g = FactorNode::new(3, "g", NodeKind::Internal).with_value(2);
        let y = FactorNode::new(4, "y", NodeKind::PrimaryOutput).with_fanins([NodeId(0)]);

        let mut network = FactorCommandNetwork::new();
        network.add_node(a).unwrap();
        network.add_node(b).unwrap();
        network.add_node(f).unwrap();
        network.add_node(g).unwrap();
        network.add_node(y).unwrap();
        network
    }

    #[test]
    fn registers_legacy_factor_commands_with_mutation_flags() {
        assert_eq!(
            factor_command_registrations(),
            &[
                CommandRegistration {
                    name: "print_value",
                    kind: FactorCommandKind::PrintValue,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "print_factor",
                    kind: FactorCommandKind::PrintFactor,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "factor",
                    kind: FactorCommandKind::Factor,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "eliminate",
                    kind: FactorCommandKind::Eliminate,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "_pft",
                    kind: FactorCommandKind::PrintFactorTree,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "_conv",
                    kind: FactorCommandKind::ConvertFactor,
                    changes_network: false,
                },
            ]
        );
    }

    #[test]
    fn parses_factor_options_and_requires_nodes() {
        assert_eq!(
            parse_factor_command("factor", ["-g", "f"]).unwrap(),
            FactorCommand::Factor {
                method: FactorMethod::Good,
                nodes: vec!["f".to_owned()],
            }
        );
        assert_eq!(
            parse_factor_command("factor", ["-qb", "f"]).unwrap(),
            FactorCommand::Factor {
                method: FactorMethod::Boolean,
                nodes: vec!["f".to_owned()],
            }
        );
        assert!(matches!(
            parse_factor_command("factor", ["-q"]),
            Err(FactorCommandError::Usage(FactorCommandKind::Factor))
        ));
    }

    #[test]
    fn boolean_factor_reports_legacy_unavailable_message() {
        let mut network = sample_network();
        let mut backend = RecordingBackend::default();
        let command = FactorCommand::Factor {
            method: FactorMethod::Boolean,
            nodes: vec!["f".to_owned()],
        };

        let output = dispatch_factor_command(&mut backend, &mut network, &command).unwrap();

        assert_eq!(output.status, 1);
        assert_eq!(output.stderr, "boolean factoring is not available\n");
        assert!(backend.factored.is_empty());
    }

    #[test]
    fn factor_dispatches_only_internal_nodes_to_backend() {
        let mut network = sample_network();
        let mut backend = RecordingBackend::default();
        let command = FactorCommand::Factor {
            method: FactorMethod::Quick,
            nodes: vec!["a".to_owned(), "f".to_owned(), "y".to_owned()],
        };

        let output = dispatch_factor_command(&mut backend, &mut network, &command).unwrap();

        assert_eq!(output.status, 0);
        assert_eq!(backend.factored, vec![(NodeId(2), FactorMethod::Quick)]);
    }

    #[test]
    fn print_factor_skips_primary_inputs_and_prints_po_buffer_case() {
        let network = sample_network();

        let output = print_factor(&network, &[]).unwrap();

        assert!(output.contains("    y = a\n"));
        assert!(output.contains("    f = (a * b) + !b\n"));
        assert!(!output.contains("    a ="));
    }

    #[test]
    fn print_value_orders_and_limits_internal_nodes() {
        let network = sample_network();

        let output = print_values(&network, &[], ValueOrder::Ascending, 1).unwrap();

        assert_eq!(output, "g:\t2\n");
    }

    #[test]
    fn print_factor_tree_reports_empty_or_nested_tree() {
        let network = sample_network();

        let output = print_factor_trees(&network, &["f".to_owned(), "g".to_owned()]).unwrap();

        assert!(output.contains("--- f ---\n0: OR\t\tAND\tINV\n"));
        assert!(output.contains("1: AND\t\ta\tb\n"));
        assert!(output.contains("--- g ---\nempty.\n"));
    }

    #[test]
    fn convert_factor_prints_source_factor_and_gate_nodes() {
        let network = sample_network();

        let output = convert_factor(&network, &["f".to_owned()], NameMode::Long).unwrap();

        assert_eq!(
            output,
            "    f = (a * b) + !b\n\tn1 = AND( a b)\n\tn2 = INV(b)\n\tf = OR( n1 n2)\n\n"
        );
    }

    #[test]
    fn parses_eliminate_legacy_forms_and_rejects_bad_numbers() {
        assert_eq!(
            parse_factor_command("eliminate", ["-l", "25", "-3"]).unwrap(),
            FactorCommand::Eliminate {
                value: -3,
                limit: 25,
            }
        );
        assert_eq!(
            parse_factor_command("eliminate", ["4"]).unwrap(),
            FactorCommand::Eliminate {
                value: 4,
                limit: 1000,
            }
        );
        assert!(matches!(
            parse_factor_command("eliminate", ["-l", "x", "3"]),
            Err(FactorCommandError::InvalidInteger(_))
        ));
    }

    #[test]
    fn eliminate_dispatches_to_backend() {
        let mut network = sample_network();
        let mut backend = RecordingBackend::default();
        let command = FactorCommand::Eliminate {
            value: 4,
            limit: 100,
        };

        dispatch_factor_command(&mut backend, &mut network, &command).unwrap();

        assert_eq!(backend.eliminated, vec![(4, 100)]);
    }

    #[test]
    fn source_contains_no_dependency_metadata_or_c_abi_tokens() {
        let source = include_str!("com_ft.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("be", "ad", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1", "-", "8", "j", "8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
