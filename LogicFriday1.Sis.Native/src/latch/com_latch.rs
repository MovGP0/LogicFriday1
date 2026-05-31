//! Native command model for `LogicSynthesis/sis/latch/com_latch.c`.
//!
//! The legacy command only registers `print_latch` and prints latch endpoint,
//! value, type, and control-node information. This port keeps the command
//! parsing and formatting in safe Rust and represents the surrounding SIS
//! network/latch state with a small owned model for later integration.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(usize);

impl NodeId
{
    pub fn index(self) -> usize
    {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatchNode
{
    pub name: String,
}

impl LatchNode
{
    pub fn new(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchSynchType
{
    ActiveHigh,
    ActiveLow,
    RisingEdge,
    FallingEdge,
    Combinational,
    Asynch,
    Unknown,
}

impl LatchSynchType
{
    pub fn legacy_code(self) -> &'static str
    {
        match self
        {
            Self::ActiveHigh => "ah",
            Self::ActiveLow => "al",
            Self::RisingEdge => "re",
            Self::FallingEdge => "fe",
            Self::Combinational => "co",
            Self::Asynch => "as",
            Self::Unknown => "un",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Latch
{
    pub input: NodeId,
    pub output: NodeId,
    pub initial_value: i32,
    pub current_value: i32,
    pub synch_type: LatchSynchType,
    pub control: Option<NodeId>,
}

impl Latch
{
    pub fn new(input: NodeId, output: NodeId) -> Self
    {
        Self
        {
            input,
            output,
            initial_value: 3,
            current_value: 3,
            synch_type: LatchSynchType::Unknown,
            control: None,
        }
    }

    pub fn with_values(mut self, initial_value: i32, current_value: i32) -> Self
    {
        self.initial_value = initial_value;
        self.current_value = current_value;
        self
    }

    pub fn with_type(mut self, synch_type: LatchSynchType) -> Self
    {
        self.synch_type = synch_type;
        self
    }

    pub fn with_control(mut self, control: NodeId) -> Self
    {
        self.control = Some(control);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LatchNetwork
{
    nodes: Vec<LatchNode>,
    name_table: BTreeMap<String, NodeId>,
    latches: Vec<Latch>,
    latch_by_node: BTreeMap<NodeId, usize>,
}

impl LatchNetwork
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn add_node(&mut self, name: impl Into<String>) -> Result<NodeId, LatchCommandError>
    {
        let name = name.into();
        if self.name_table.contains_key(&name)
        {
            return Err(LatchCommandError::DuplicateNode(name));
        }

        let node = NodeId(self.nodes.len());
        self.nodes.push(LatchNode::new(name.clone()));
        self.name_table.insert(name, node);
        Ok(node)
    }

    pub fn add_latch(&mut self, latch: Latch) -> Result<usize, LatchCommandError>
    {
        self.node(latch.input)?;
        self.node(latch.output)?;
        if let Some(control) = latch.control
        {
            self.node(control)?;
        }
        if self.latch_by_node.contains_key(&latch.input)
        {
            return Err(LatchCommandError::NodeAlreadyLatched(latch.input));
        }
        if self.latch_by_node.contains_key(&latch.output)
        {
            return Err(LatchCommandError::NodeAlreadyLatched(latch.output));
        }

        let index = self.latches.len();
        self.latch_by_node.insert(latch.input, index);
        self.latch_by_node.insert(latch.output, index);
        self.latches.push(latch);
        Ok(index)
    }

    pub fn node(&self, node: NodeId) -> Result<&LatchNode, LatchCommandError>
    {
        self.nodes
            .get(node.index())
            .ok_or(LatchCommandError::MissingNodeById(node))
    }

    pub fn find_node(&self, name: &str) -> Option<NodeId>
    {
        self.name_table.get(name).copied()
    }

    pub fn latches(&self) -> &[Latch]
    {
        &self.latches
    }

    pub fn latch_from_node(&self, node: NodeId) -> Result<Option<&Latch>, LatchCommandError>
    {
        self.node(node)?;
        Ok(self
            .latch_by_node
            .get(&node)
            .and_then(|index| self.latches.get(*index)))
    }

    fn node_name(&self, node: NodeId) -> Result<&str, LatchCommandError>
    {
        Ok(self.node(node)?.name.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchCommandKind
{
    PrintLatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration
{
    pub name: &'static str,
    pub kind: LatchCommandKind,
    pub changes_network: bool,
}

pub const LATCH_COMMANDS: &[CommandRegistration] = &[CommandRegistration
{
    name: "print_latch",
    kind: LatchCommandKind::PrintLatch,
    changes_network: false,
}];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LatchCommand
{
    PrintLatch
    {
        summary: bool,
        nodes: Vec<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatchCommandOutput
{
    pub status: i32,
    pub lines: Vec<String>,
}

impl LatchCommandOutput
{
    pub fn success(lines: Vec<String>) -> Self
    {
        Self
        {
            status: 0,
            lines,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LatchCommandError
{
    UnknownCommand(String),
    Usage
    {
        command: LatchCommandKind,
        usage: &'static str,
    },
    UnsupportedOption(String),
    MissingNode(String),
    MissingNodeById(NodeId),
    DuplicateNode(String),
    NodeAlreadyLatched(NodeId),
}

impl fmt::Display for LatchCommandError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::UnknownCommand(command) => write!(formatter, "unknown latch command {command}"),
            Self::Usage
            {
                usage,
                ..
            } => write!(formatter, "{usage}"),
            Self::UnsupportedOption(option) => write!(formatter, "unsupported option {option}"),
            Self::MissingNode(name) => write!(formatter, "node {name} was not found"),
            Self::MissingNodeById(node) => write!(formatter, "node {} was not found", node.index()),
            Self::DuplicateNode(name) => write!(formatter, "duplicate node {name}"),
            Self::NodeAlreadyLatched(node) => write!(formatter, "node {} is already latched", node.index()),
        }
    }
}

impl Error for LatchCommandError {}

pub fn latch_command_registrations() -> &'static [CommandRegistration]
{
    LATCH_COMMANDS
}

pub fn parse_latch_command<I, S>(
    command_name: &str,
    args: I,
) -> Result<LatchCommand, LatchCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    match command_name
    {
        "print_latch" => parse_print_latch_args(&args),
        _ => Err(LatchCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn dispatch_latch_command(
    network: &LatchNetwork,
    command: &LatchCommand,
) -> Result<LatchCommandOutput, LatchCommandError>
{
    match command
    {
        LatchCommand::PrintLatch
        {
            summary,
            nodes,
        } => print_latch(network, *summary, nodes),
    }
}

pub fn print_latch(
    network: &LatchNetwork,
    summary: bool,
    node_names: &[String],
) -> Result<LatchCommandOutput, LatchCommandError>
{
    let mut lines = Vec::new();

    if node_names.is_empty()
    {
        for latch in network.latches()
        {
            lines.push(format_latch(network, latch, summary)?);
        }

        return Ok(LatchCommandOutput::success(lines));
    }

    for name in node_names
    {
        let node = network
            .find_node(name)
            .ok_or_else(|| LatchCommandError::MissingNode(name.clone()))?;
        match network.latch_from_node(node)?
        {
            Some(latch) => lines.push(format_latch(network, latch, summary)?),
            None => lines.push(format!("\tNode {} is not a latch input or output", name)),
        }
    }

    Ok(LatchCommandOutput::success(lines))
}

fn parse_print_latch_args(args: &[String]) -> Result<LatchCommand, LatchCommandError>
{
    let mut summary = false;
    let mut nodes = Vec::new();

    for arg in args
    {
        match arg.as_str()
        {
            "-s" if nodes.is_empty() => summary = true,
            _ if arg.starts_with('-') && nodes.is_empty() =>
            {
                return Err(LatchCommandError::Usage
                {
                    command: LatchCommandKind::PrintLatch,
                    usage: "usage: print_latch [-s] n1 n2 ...",
                });
            }
            _ => nodes.push(arg.clone()),
        }
    }

    Ok(LatchCommand::PrintLatch
    {
        summary,
        nodes,
    })
}

fn format_latch(
    network: &LatchNetwork,
    latch: &Latch,
    summary: bool,
) -> Result<String, LatchCommandError>
{
    let mut line = format!(
        "input: {} output: {} init val: {} cur val: {}",
        network.node_name(latch.input)?,
        network.node_name(latch.output)?,
        latch.initial_value,
        latch.current_value
    );

    if !summary
    {
        line.push_str(" type: ");
        line.push_str(latch.synch_type.legacy_code());
        line.push_str(" control: ");
        match latch.control
        {
            Some(control) => line.push_str(network.node_name(control)?),
            None => line.push_str("none"),
        }
    }

    Ok(line)
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn sample_network() -> LatchNetwork
    {
        let mut network = LatchNetwork::new();
        let d0 = network.add_node("d0").unwrap();
        let q0 = network.add_node("q0").unwrap();
        let clock = network.add_node("clock").unwrap();
        let other = network.add_node("other").unwrap();
        let d1 = network.add_node("d1").unwrap();
        let q1 = network.add_node("q1").unwrap();

        network
            .add_latch(
                Latch::new(d0, q0)
                    .with_values(0, 1)
                    .with_type(LatchSynchType::RisingEdge)
                    .with_control(clock),
            )
            .unwrap();
        network
            .add_latch(
                Latch::new(d1, q1)
                    .with_values(3, 3)
                    .with_type(LatchSynchType::Unknown),
            )
            .unwrap();

        assert!(network.latch_from_node(other).unwrap().is_none());
        network
    }

    #[test]
    fn registrations_match_original_command_name()
    {
        assert_eq!(
            latch_command_registrations(),
            &[CommandRegistration
            {
                name: "print_latch",
                kind: LatchCommandKind::PrintLatch,
                changes_network: false,
            }]
        );
    }

    #[test]
    fn parses_summary_option_and_node_operands()
    {
        assert_eq!(
            parse_latch_command("print_latch", ["-s", "q0", "d1"]).unwrap(),
            LatchCommand::PrintLatch
            {
                summary: true,
                nodes: vec!["q0".to_owned(), "d1".to_owned()],
            }
        );
    }

    #[test]
    fn rejects_unknown_options_with_legacy_usage()
    {
        assert_eq!(
            parse_latch_command("print_latch", ["-x"]).unwrap_err(),
            LatchCommandError::Usage
            {
                command: LatchCommandKind::PrintLatch,
                usage: "usage: print_latch [-s] n1 n2 ...",
            }
        );
    }

    #[test]
    fn prints_all_latches_with_control_when_not_summary()
    {
        let network = sample_network();
        let command = parse_latch_command("print_latch", Vec::<String>::new()).unwrap();
        let output = dispatch_latch_command(&network, &command).unwrap();

        assert_eq!(output.status, 0);
        assert_eq!(
            output.lines,
            vec![
                "input: d0 output: q0 init val: 0 cur val: 1 type: re control: clock",
                "input: d1 output: q1 init val: 3 cur val: 3 type: un control: none",
            ]
        );
    }

    #[test]
    fn summary_suppresses_type_and_control_fields()
    {
        let network = sample_network();
        let command = parse_latch_command("print_latch", ["-s"]).unwrap();
        let output = dispatch_latch_command(&network, &command).unwrap();

        assert_eq!(
            output.lines,
            vec![
                "input: d0 output: q0 init val: 0 cur val: 1",
                "input: d1 output: q1 init val: 3 cur val: 3",
            ]
        );
    }

    #[test]
    fn selected_latch_output_resolves_to_owning_latch()
    {
        let network = sample_network();
        let command = parse_latch_command("print_latch", ["q0"]).unwrap();
        let output = dispatch_latch_command(&network, &command).unwrap();

        assert_eq!(
            output.lines,
            vec!["input: d0 output: q0 init val: 0 cur val: 1 type: re control: clock"]
        );
    }

    #[test]
    fn selected_non_latch_node_matches_legacy_diagnostic()
    {
        let network = sample_network();
        let command = parse_latch_command("print_latch", ["other"]).unwrap();
        let output = dispatch_latch_command(&network, &command).unwrap();

        assert_eq!(
            output.lines,
            vec!["\tNode other is not a latch input or output"]
        );
    }

    #[test]
    fn latch_type_codes_match_legacy_print_codes()
    {
        assert_eq!(LatchSynchType::ActiveHigh.legacy_code(), "ah");
        assert_eq!(LatchSynchType::ActiveLow.legacy_code(), "al");
        assert_eq!(LatchSynchType::RisingEdge.legacy_code(), "re");
        assert_eq!(LatchSynchType::FallingEdge.legacy_code(), "fe");
        assert_eq!(LatchSynchType::Combinational.legacy_code(), "co");
        assert_eq!(LatchSynchType::Asynch.legacy_code(), "as");
        assert_eq!(LatchSynchType::Unknown.legacy_code(), "un");
    }

    #[test]
    fn source_contains_no_legacy_c_abi_export()
    {
        let source = include_str!("com_latch.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
