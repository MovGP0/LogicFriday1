//! Native Rust model for writing bounded-wire-delay networks in Burch format.
//!
//! The legacy implementation writes either a specification expression or a
//! mapped implementation expression. This port keeps the writer independent of
//! the old SIS pointer graph: callers provide a lightweight network snapshot
//! whose node values, latch bindings, covers, gates, and delays have already
//! been computed by native graph code.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fmt::Write;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BurchNodeKind
{
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BurchSignalKind
{
    Input,
    Output,
    Dummy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CubeInput
{
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurchDelay
{
    pub rise: f64,
    pub fall: f64,
}

impl BurchDelay
{
    pub fn new(rise: f64, fall: f64) -> Self
    {
        Self
        {
            rise,
            fall,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BurchSignal
{
    pub name: String,
    pub kind: BurchSignalKind,
}

impl BurchSignal
{
    pub fn input(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            kind: BurchSignalKind::Input,
        }
    }

    pub fn output(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            kind: BurchSignalKind::Output,
        }
    }

    pub fn dummy(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            kind: BurchSignalKind::Dummy,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BurchNode
{
    pub name: String,
    pub kind: BurchNodeKind,
    pub fanins: Vec<usize>,
    pub fanouts: Vec<usize>,
    pub latch_output: Option<usize>,
    pub simulation: Option<i32>,
    pub library_gate_name: Option<String>,
    pub pin_delays: Vec<BurchDelay>,
    pub cover: Vec<Vec<CubeInput>>,
}

impl BurchNode
{
    pub fn primary_input(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            kind: BurchNodeKind::PrimaryInput,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            latch_output: None,
            simulation: None,
            library_gate_name: None,
            pin_delays: Vec::new(),
            cover: Vec::new(),
        }
    }

    pub fn primary_output(name: impl Into<String>, fanin: usize) -> Self
    {
        Self
        {
            name: name.into(),
            kind: BurchNodeKind::PrimaryOutput,
            fanins: vec![fanin],
            fanouts: Vec::new(),
            latch_output: None,
            simulation: None,
            library_gate_name: None,
            pin_delays: Vec::new(),
            cover: Vec::new(),
        }
    }

    pub fn internal(name: impl Into<String>, gate_name: impl Into<String>, fanins: Vec<usize>) -> Self
    {
        Self
        {
            name: name.into(),
            kind: BurchNodeKind::Internal,
            fanins,
            fanouts: Vec::new(),
            latch_output: None,
            simulation: None,
            library_gate_name: Some(gate_name.into()),
            pin_delays: Vec::new(),
            cover: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BurchNetwork
{
    pub name: String,
    pub mapped: bool,
    pub primary_inputs: Vec<usize>,
    pub primary_outputs: Vec<usize>,
    pub nodes: Vec<BurchNode>,
    pub signals: Vec<BurchSignal>,
}

impl BurchNetwork
{
    pub fn new(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            mapped: true,
            primary_inputs: Vec::new(),
            primary_outputs: Vec::new(),
            nodes: Vec::new(),
            signals: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: BurchNode) -> usize
    {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    pub fn add_fanout(&mut self, node: usize, fanout: usize) -> Result<(), BurchWriteError>
    {
        self.node(node)?;
        self.node(fanout)?;
        self.nodes[node].fanouts.push(fanout);
        Ok(())
    }

    pub fn add_latch_binding(&mut self, latch_input_po: usize, latch_output_pi: usize) -> Result<(), BurchWriteError>
    {
        if self.node(latch_input_po)?.kind != BurchNodeKind::PrimaryOutput
        {
            return Err(BurchWriteError::InvalidLatchInput(latch_input_po));
        }
        if self.node(latch_output_pi)?.kind != BurchNodeKind::PrimaryInput
        {
            return Err(BurchWriteError::InvalidLatchOutput(latch_output_pi));
        }
        self.nodes[latch_input_po].latch_output = Some(latch_output_pi);
        Ok(())
    }

    fn node(&self, id: usize) -> Result<&BurchNode, BurchWriteError>
    {
        self.nodes.get(id).ok_or(BurchWriteError::MissingNode(id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurchWriteOptions
{
    pub min_delay_factor: f64,
    pub write_specification: bool,
}

impl Default for BurchWriteOptions
{
    fn default() -> Self
    {
        Self
        {
            min_delay_factor: 1.0,
            write_specification: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BurchWriteError
{
    NetworkNotMapped,
    MissingNode(usize),
    InvalidLatchInput(usize),
    InvalidLatchOutput(usize),
    MissingGateName(String),
    UnsupportedGateName(String),
    EmptyGate(String),
    PinDelayCount
    {
        node: String,
        expected: usize,
        actual: usize,
    },
    MalformedCover
    {
        node: String,
        expected: usize,
        actual: usize,
    },
    NameTooLong(String),
    Fmt,
}

impl fmt::Display for BurchWriteError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::NetworkNotMapped => formatter.write_str("network not mapped, cannot write netlist"),
            Self::MissingNode(id) => write!(formatter, "missing node {id}"),
            Self::InvalidLatchInput(id) => write!(formatter, "node {id} is not a latch input primary output"),
            Self::InvalidLatchOutput(id) => write!(formatter, "node {id} is not a latch output primary input"),
            Self::MissingGateName(node) => write!(formatter, "node {node} has no library gate"),
            Self::UnsupportedGateName(name) => write!(formatter, "unsupported Burch gate {name}"),
            Self::EmptyGate(node) => write!(formatter, "constant node function {node}"),
            Self::PinDelayCount { node, expected, actual } => {
                write!(formatter, "node {node} has {actual} pin delays, expected {expected}")
            }
            Self::MalformedCover { node, expected, actual } => {
                write!(formatter, "node {node} has cover cube width {actual}, expected {expected}")
            }
            Self::NameTooLong(name) => write!(formatter, "name too long: {name}"),
            Self::Fmt => formatter.write_str("failed to format Burch output"),
        }
    }
}

impl Error for BurchWriteError {}

impl From<fmt::Error> for BurchWriteError
{
    fn from(_: fmt::Error) -> Self
    {
        Self::Fmt
    }
}

pub fn write_burch(network: &BurchNetwork, options: BurchWriteOptions) -> Result<String, BurchWriteError>
{
    if !network.mapped
    {
        return Err(BurchWriteError::NetworkNotMapped);
    }

    let mut writer = BurchWriter::new(network, options);
    if options.write_specification
    {
        writer.write_specification()?;
    }
    else
    {
        writer.write_implementation()?;
    }

    Ok(writer.output)
}

pub fn make_legal_name(name: &str) -> Result<String, BurchWriteError>
{
    if name.len() >= 512
    {
        return Err(BurchWriteError::NameTooLong(name.to_owned()));
    }

    Ok(name
        .chars()
        .map(|value| if value.is_ascii_alphanumeric() { value } else { '_' })
        .collect())
}

pub fn burch_gate_name(library_gate_name: &str) -> Option<&'static str>
{
    if library_gate_name.starts_with("NAND")
    {
        Some("orgate")
    }
    else if library_gate_name.starts_with("NOR")
    {
        Some("andgate")
    }
    else if library_gate_name.starts_with("INV")
    {
        Some("inverter")
    }
    else if library_gate_name.starts_with("DELAY")
    {
        Some("buffer")
    }
    else
    {
        None
    }
}

struct BurchWriter<'a>
{
    network: &'a BurchNetwork,
    options: BurchWriteOptions,
    inputs: HashSet<String>,
    output: String,
}

impl<'a> BurchWriter<'a>
{
    fn new(network: &'a BurchNetwork, options: BurchWriteOptions) -> Self
    {
        let inputs = network
            .signals
            .iter()
            .filter(|signal| signal.kind == BurchSignalKind::Input)
            .map(|signal| make_legal_name(signal.name.as_str()).unwrap_or_else(|_| signal.name.clone()))
            .collect();

        Self
        {
            network,
            options,
            inputs,
            output: String::new(),
        }
    }

    fn write_specification(&mut self) -> Result<(), BurchWriteError>
    {
        writeln!(self.output, "\n(setq {}-spec", make_legal_name(self.network.name.as_str())?)?;
        writeln!(self.output, " (teval")?;
        writeln!(self.output, "   (compose")?;

        for node_id in 0..self.network.nodes.len()
        {
            let node = self.network.node(node_id)?;
            if self.is_garbage_node(node)?
            {
                continue;
            }
            self.write_specification_node(node_id)?;
        }

        writeln!(self.output, ")))")?;
        Ok(())
    }

    fn write_specification_node(&mut self, node_id: usize) -> Result<(), BurchWriteError>
    {
        let node = self.network.node(node_id)?;
        let fanin_count = node.fanins.len();
        if fanin_count == 0
        {
            return Err(BurchWriteError::EmptyGate(node.name.clone()));
        }
        self.validate_cover(node)?;

        let named_node = self.named_node(node_id)?;
        let named_node_name = named_node.name.as_str();
        if node.kind == BurchNodeKind::Internal
        {
            write!(self.output, "    (no-hazard gate ( ")?;
        }
        else
        {
            write!(self.output, "    (gate ( ")?;
        }

        for fanin_id in &node.fanins
        {
            let fanin = self.named_node(*fanin_id)?;
            if fanin.name == named_node_name
            {
                continue;
            }
            self.write_signal_reference(fanin, "", " ")?;
        }

        write!(self.output, ")")?;
        let named_node = self.named_node(node_id)?;
        self.write_signal_reference(named_node, "", "\n")?;
        writeln!(self.output, ";       insert here min del for {}", named_node.name)?;
        writeln!(self.output, "       (0 infty)")?;
        self.write_cover_expression(node)?;
        writeln!(self.output, "    )")?;
        Ok(())
    }

    fn write_implementation(&mut self) -> Result<(), BurchWriteError>
    {
        writeln!(self.output, "(setq {}-impl", make_legal_name(self.network.name.as_str())?)?;
        write!(self.output, " (project '(")?;
        for input in &self.network.primary_inputs
        {
            let node = self.network.node(*input)?;
            write!(self.output, " {}", make_legal_name(node.name.as_str())?)?;
        }
        writeln!(self.output, " Phi)")?;
        writeln!(self.output, "   (compose")?;

        for node_id in 0..self.network.nodes.len()
        {
            self.write_implementation_node(node_id)?;
        }

        writeln!(self.output, ")))")?;
        Ok(())
    }

    fn write_implementation_node(&mut self, node_id: usize) -> Result<(), BurchWriteError>
    {
        let node = self.network.node(node_id)?;
        if node.kind != BurchNodeKind::Internal
        {
            return Ok(());
        }

        let library_gate_name = node
            .library_gate_name
            .as_deref()
            .ok_or_else(|| BurchWriteError::MissingGateName(node.name.clone()))?;
        let gate_name = match burch_gate_name(library_gate_name)
        {
            Some(value) => value,
            None => return Ok(()),
        };

        let fanin_count = node.fanins.len();
        if fanin_count == 0
        {
            return Err(BurchWriteError::EmptyGate(node.name.clone()));
        }
        if node.pin_delays.len() != fanin_count
        {
            return Err(BurchWriteError::PinDelayCount
            {
                node: node.name.clone(),
                expected: fanin_count,
                actual: node.pin_delays.len(),
            });
        }

        let (min_delay, max_delay) = gate_delay_range(node, self.options.min_delay_factor);
        write!(self.output, "    (chaos {gate_name} ")?;

        if fanin_count == 1
        {
            let fanin = self.named_node(node.fanins[0])?;
            self.write_signal_reference(fanin, "", "")?;
        }
        else
        {
            write!(self.output, "( ")?;
            for (index, fanin_id) in node.fanins.iter().enumerate()
            {
                let fanin = self.named_node(*fanin_id)?;
                if self.has_later_duplicate_named_fanin(node, index, fanin.name.as_str())?
                {
                    continue;
                }
                self.write_signal_reference(fanin, "-", " ")?;
            }
            write!(self.output, ")")?;
        }

        let named_node = self.named_node(node_id)?;
        let output_name = make_legal_name(named_node.name.as_str())?;
        let _max_hazard = if self.inputs.contains(output_name.as_str())
        {
            None
        }
        else
        {
            Some(max_delay)
        };

        if named_node.simulation.is_some()
        {
            write!(self.output, " ")?;
            self.write_signal_reference(named_node, "", "")?;
            writeln!(self.output, " ({min_delay} {max_delay}))")?;
        }
        else
        {
            writeln!(self.output, " {output_name} ({min_delay} {max_delay}))")?;
        }

        Ok(())
    }

    fn write_signal_reference(
        &mut self,
        node: &BurchNode,
        prefix: &str,
        suffix: &str,
    ) -> Result<(), BurchWriteError>
    {
        let name = make_legal_name(node.name.as_str())?;
        match node.simulation
        {
            Some(value) => write!(self.output, "({prefix}{name} {value}){suffix}")?,
            None => write!(self.output, "{prefix}{name}{suffix}")?,
        }
        Ok(())
    }

    fn write_cover_expression(&mut self, node: &BurchNode) -> Result<(), BurchWriteError>
    {
        if node.cover.len() > 1
        {
            writeln!(self.output, "         (logior")?;
        }

        for cube in &node.cover
        {
            let cube_is_sparse = cube
                .iter()
                .filter(|input| **input != CubeInput::DontCare)
                .count() < 2 * node.fanins.len() - 1;

            if cube_is_sparse
            {
                write!(self.output, "         (logand ")?;
            }
            else
            {
                write!(self.output, "                 ")?;
            }

            for (index, input) in cube.iter().enumerate()
            {
                let fanin = self.network.node(node.fanins[index])?;
                match input
                {
                    CubeInput::Zero => write!(self.output, " (lognot {})", fanin.name)?,
                    CubeInput::One => write!(self.output, " {}", fanin.name)?,
                    CubeInput::DontCare => {}
                }
            }

            if cube_is_sparse
            {
                writeln!(self.output, ")")?;
            }
            else
            {
                writeln!(self.output)?;
            }
        }

        if node.cover.len() > 1
        {
            writeln!(self.output, "         )")?;
        }

        Ok(())
    }

    fn validate_cover(&self, node: &BurchNode) -> Result<(), BurchWriteError>
    {
        for cube in &node.cover
        {
            if cube.len() != node.fanins.len()
            {
                return Err(BurchWriteError::MalformedCover
                {
                    node: node.name.clone(),
                    expected: node.fanins.len(),
                    actual: cube.len(),
                });
            }
        }

        Ok(())
    }

    fn is_garbage_node(&self, node: &BurchNode) -> Result<bool, BurchWriteError>
    {
        Ok(self.first_level_latch_output(node)?.is_none())
    }

    fn named_node(&self, node_id: usize) -> Result<&'a BurchNode, BurchWriteError>
    {
        let node = self.network.node(node_id)?;
        match self.first_level_latch_output(node)?
        {
            Some(latch_output) => self.network.node(latch_output),
            None => Ok(node),
        }
    }

    fn first_level_latch_output(&self, node: &BurchNode) -> Result<Option<usize>, BurchWriteError>
    {
        for fanout_id in &node.fanouts
        {
            let fanout = self.network.node(*fanout_id)?;
            if fanout.kind == BurchNodeKind::PrimaryOutput
            {
                if let Some(latch_output) = fanout.latch_output
                {
                    if self.network.node(latch_output)?.kind == BurchNodeKind::PrimaryInput
                    {
                        return Ok(Some(latch_output));
                    }
                }
            }
        }

        Ok(None)
    }

    fn has_later_duplicate_named_fanin(
        &self,
        node: &BurchNode,
        index: usize,
        name: &str,
    ) -> Result<bool, BurchWriteError>
    {
        for fanin_id in node.fanins.iter().skip(index + 1)
        {
            if self.named_node(*fanin_id)?.name == name
            {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

fn gate_delay_range(node: &BurchNode, min_delay_factor: f64) -> (i32, i32)
{
    let mut min_delay = f64::INFINITY;
    let mut max_delay = 0.0_f64;
    for delay in &node.pin_delays
    {
        min_delay = min_delay.min(delay.rise).min(delay.fall);
        max_delay = max_delay.max(delay.rise).max(delay.fall);
    }

    ((min_delay * min_delay_factor) as i32, max_delay as i32)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn make_legal_name_replaces_non_alphanumeric_bytes()
    {
        assert_eq!(make_legal_name("a-b/c.1").unwrap(), "a_b_c_1");
    }

    #[test]
    fn burch_gate_name_maps_legacy_library_prefixes()
    {
        assert_eq!(burch_gate_name("NAND2"), Some("orgate"));
        assert_eq!(burch_gate_name("NOR3"), Some("andgate"));
        assert_eq!(burch_gate_name("INV"), Some("inverter"));
        assert_eq!(burch_gate_name("DELAY"), Some("buffer"));
        assert_eq!(burch_gate_name("AOI21"), None);
    }

    #[test]
    fn write_implementation_uses_latch_output_name_and_deduplicates_fanins()
    {
        let mut network = BurchNetwork::new("demo-net");
        network.signals.push(BurchSignal::input("a"));
        network.signals.push(BurchSignal::output("q"));

        let a = network.add_node(BurchNode::primary_input("a"));
        let internal = network.add_node(BurchNode::internal("n$1", "NAND2", vec![a, a]));
        let latch_input = network.add_node(BurchNode::primary_output("q_in", internal));
        let q = network.add_node(BurchNode::primary_input("q"));

        network.primary_inputs.push(a);
        network.primary_outputs.push(latch_input);
        network.nodes[internal].pin_delays = vec![BurchDelay::new(2.0, 4.0), BurchDelay::new(3.0, 5.0)];
        network.add_fanout(internal, latch_input).unwrap();
        network.add_latch_binding(latch_input, q).unwrap();
        network.nodes[q].simulation = Some(1);

        let output = write_burch(
            &network,
            BurchWriteOptions
            {
                min_delay_factor: 0.5,
                write_specification: false,
            },
        )
        .unwrap();

        assert!(output.contains("(setq demo_net-impl"));
        assert!(output.contains("(project '( a Phi)"));
        assert!(output.contains("    (chaos orgate ( -a ) (q 1) (1 5))"));
    }

    #[test]
    fn write_specification_skips_nodes_without_latch_fanout()
    {
        let mut network = BurchNetwork::new("spec");
        let a = network.add_node(BurchNode::primary_input("a"));
        let live = network.add_node(BurchNode::internal("live", "NAND2", vec![a]));
        let garbage = network.add_node(BurchNode::internal("garbage", "NAND2", vec![a]));
        let latch_input = network.add_node(BurchNode::primary_output("live_out", live));
        let q = network.add_node(BurchNode::primary_input("q"));

        network.nodes[live].cover = vec![vec![CubeInput::One]];
        network.nodes[garbage].cover = vec![vec![CubeInput::One]];
        network.add_fanout(live, latch_input).unwrap();
        network.add_latch_binding(latch_input, q).unwrap();

        let output = write_burch(
            &network,
            BurchWriteOptions
            {
                min_delay_factor: 1.0,
                write_specification: true,
            },
        )
        .unwrap();

        assert!(output.contains("(setq spec-spec"));
        assert!(output.contains("q\n;       insert here min del for q"));
        assert!(output.contains("                 a"));
        assert!(!output.contains("garbage"));
    }

    #[test]
    fn write_implementation_rejects_unmapped_networks()
    {
        let mut network = BurchNetwork::new("unmapped");
        network.mapped = false;

        assert_eq!(
            write_burch(&network, BurchWriteOptions::default()).unwrap_err(),
            BurchWriteError::NetworkNotMapped
        );
    }
}
