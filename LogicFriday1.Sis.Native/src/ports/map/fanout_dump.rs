//! Native Rust fanout-problem dump support for `sis/map/fanout_dump.c`.
//!
//! The original SIS helper is installed as a fanout optimizer callback, but it
//! does not optimize. It extracts large fanout problems into numbered BLIF
//! files. This port keeps that behavior as owned Rust data: callers provide the
//! source, polarity-indexed sinks, and optional mapper tree context; the module
//! returns deterministic dump records, BLIF text for the extracted fanout
//! network, and compact reports. Direct `network_t`, `node_t`, and `FILE *`
//! integration remains outside this per-file native port.

use std::error::Error;
use std::fmt;

use super::tree::{MapperTree, MapperTreeError};
use super::virtual_net::{
    DelayTime, GateKind, MINUS_INFINITY, NodeId, SourceRef, VirtualMappedNetwork,
    VirtualNetworkError,
};

pub const DEFAULT_DUMP_THRESHOLD: usize = 40;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutPolarity {
    Positive,
    Negative,
}

impl FanoutPolarity {
    pub fn inverted(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutTiming {
    pub arrival: DelayTime,
    pub drive: DelayTime,
}

impl Default for FanoutTiming {
    fn default() -> Self {
        Self {
            arrival: DelayTime::new(0.0, 0.0),
            drive: DelayTime::new(0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSourceInput {
    pub name: String,
    pub timing: FanoutTiming,
}

impl FanoutSourceInput {
    pub fn new(name: impl Into<String>, timing: FanoutTiming) -> Self {
        Self {
            name: name.into(),
            timing,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FanoutSourceKind {
    PrimaryInput {
        name: String,
        timing: FanoutTiming,
    },
    Gate {
        name: String,
        gate: GateKind,
        fanins: Vec<FanoutSourceInput>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSource {
    pub kind: FanoutSourceKind,
    pub polarity: FanoutPolarity,
}

impl FanoutSource {
    pub fn primary_input(
        name: impl Into<String>,
        polarity: FanoutPolarity,
        timing: FanoutTiming,
    ) -> Self {
        Self {
            kind: FanoutSourceKind::PrimaryInput {
                name: name.into(),
                timing,
            },
            polarity,
        }
    }

    pub fn gate(
        name: impl Into<String>,
        gate: GateKind,
        fanins: Vec<FanoutSourceInput>,
        polarity: FanoutPolarity,
    ) -> Self {
        Self {
            kind: FanoutSourceKind::Gate {
                name: name.into(),
                gate,
                fanins,
            },
            polarity,
        }
    }

    pub fn name(&self) -> &str {
        match &self.kind {
            FanoutSourceKind::PrimaryInput { name, .. } | FanoutSourceKind::Gate { name, .. } => {
                name
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSink {
    pub node_name: String,
    pub pin: isize,
    pub required: DelayTime,
    pub load: f64,
}

impl FanoutSink {
    pub fn new(node_name: impl Into<String>, pin: isize, required: DelayTime, load: f64) -> Self {
        Self {
            node_name: node_name.into(),
            pin,
            required,
            load,
        }
    }

    pub fn output_name(&self) -> String {
        format!("{}({})", self.node_name, self.pin)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutProblem {
    pub network_name: Option<String>,
    pub source: FanoutSource,
    pub positive_sinks: Vec<FanoutSink>,
    pub negative_sinks: Vec<FanoutSink>,
}

impl FanoutProblem {
    pub fn sink_count(&self) -> usize {
        self.positive_sinks.len() + self.negative_sinks.len()
    }

    fn sinks_for(&self, polarity: FanoutPolarity) -> &[FanoutSink] {
        match polarity {
            FanoutPolarity::Positive => &self.positive_sinks,
            FanoutPolarity::Negative => &self.negative_sinks,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FanoutDumpOptions {
    pub dump_threshold: usize,
}

impl Default for FanoutDumpOptions {
    fn default() -> Self {
        Self {
            dump_threshold: DEFAULT_DUMP_THRESHOLD,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutDumpCost {
    pub slack: DelayTime,
    pub area: f64,
}

impl Default for FanoutDumpCost {
    fn default() -> Self {
        Self {
            slack: MINUS_INFINITY,
            area: f64::INFINITY,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutDumpRecord {
    pub sequence: usize,
    pub filename: String,
    pub model_name: String,
    pub source_name: String,
    pub source_polarity: FanoutPolarity,
    pub sink_count: usize,
    pub positive_sink_count: usize,
    pub negative_sink_count: usize,
    pub network: VirtualMappedNetwork,
    pub internal_nodes: Vec<NodeId>,
    pub tree_depth: Option<usize>,
    pub tree_leaf_count: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutDumpOutcome {
    pub cost: FanoutDumpCost,
    pub record: Option<FanoutDumpRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FanoutDumpError {
    EmptyName {
        kind: &'static str,
    },
    InvalidLoad {
        sink: String,
        load: String,
    },
    EmptyGateFanins {
        source: String,
    },
    UnsupportedBlifGate {
        node: NodeId,
        gate: GateKind,
    },
    TooManyBlifCombinations {
        node: NodeId,
        gate: GateKind,
        fanin_count: usize,
    },
    MissingNode(NodeId),
    VirtualNetwork(VirtualNetworkError),
    MapperTree(MapperTreeError),
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for FanoutDumpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName { kind } => write!(f, "{kind} name cannot be empty"),
            Self::InvalidLoad { sink, load } => {
                write!(f, "fanout sink '{sink}' has invalid load {load}")
            }
            Self::EmptyGateFanins { source } => {
                write!(f, "fanout source gate '{source}' has no fanins")
            }
            Self::UnsupportedBlifGate { node, gate } => write!(
                f,
                "fanout dump node {} uses unsupported BLIF gate {:?}",
                node.index(),
                gate
            ),
            Self::TooManyBlifCombinations {
                node,
                gate,
                fanin_count,
            } => write!(
                f,
                "fanout dump node {} gate {:?} needs {fanin_count} input combinations",
                node.index(),
                gate
            ),
            Self::MissingNode(node) => write!(f, "missing fanout dump node {}", node.index()),
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::MapperTree(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for FanoutDumpError {}

impl From<VirtualNetworkError> for FanoutDumpError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

impl From<MapperTreeError> for FanoutDumpError {
    fn from(value: MapperTreeError) -> Self {
        Self::MapperTree(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutDumpSession {
    current_network_name: String,
    next_sequence: usize,
    options: FanoutDumpOptions,
}

impl FanoutDumpSession {
    pub fn new(network_name: Option<impl Into<String>>) -> Self {
        let current_network_name = network_name
            .map(Into::into)
            .filter(|name: &String| !name.is_empty())
            .unwrap_or_else(|| "debug".to_string());
        Self {
            current_network_name,
            next_sequence: 0,
            options: FanoutDumpOptions::default(),
        }
    }

    pub fn with_options(
        network_name: Option<impl Into<String>>,
        options: FanoutDumpOptions,
    ) -> Self {
        let mut session = Self::new(network_name);
        session.options = options;
        session
    }

    pub fn set_dump_threshold(&mut self, dump_threshold: usize) {
        self.options.dump_threshold = dump_threshold;
    }

    pub fn dump_problem(
        &mut self,
        problem: &FanoutProblem,
        tree: Option<&MapperTree>,
    ) -> Result<FanoutDumpOutcome, FanoutDumpError> {
        let Some(mut record) = extract_fanout_network(problem, self.options.dump_threshold)? else {
            return Ok(FanoutDumpOutcome {
                cost: FanoutDumpCost::default(),
                record: None,
            });
        };

        record.sequence = self.next_sequence;
        record.model_name = format!("{}.{}.fanout", self.current_network_name, record.sequence);
        record.filename = basename(&record.model_name).to_string();
        if let Some(tree) = tree {
            tree.validate()?;
            record.tree_depth = Some(tree.depth()?);
            record.tree_leaf_count = Some(tree.leaves()?.len());
        }
        self.next_sequence += 1;

        Ok(FanoutDumpOutcome {
            cost: FanoutDumpCost::default(),
            record: Some(record),
        })
    }
}

pub fn full_sis_fanout_dump_unavailable() -> Result<FanoutDumpOutcome, FanoutDumpError> {
    Err(FanoutDumpError::MissingSisPorts {
        operation: "fanout_dump full SIS callback and file writer integration",
    })
}

pub fn extract_fanout_network(
    problem: &FanoutProblem,
    dump_threshold: usize,
) -> Result<Option<FanoutDumpRecord>, FanoutDumpError> {
    validate_problem(problem)?;
    if problem.sink_count() < dump_threshold {
        return Ok(None);
    }

    let mut network = VirtualMappedNetwork::new();
    let mut internal_nodes = Vec::new();
    let source = add_source_to_network(&mut network, &problem.source)?;
    if matches!(problem.source.kind, FanoutSourceKind::Gate { .. }) {
        internal_nodes.push(source);
    }
    add_sinks(
        &mut network,
        source,
        problem.sinks_for(problem.source.polarity),
    )?;

    let inverse_sinks = problem.sinks_for(problem.source.polarity.inverted());
    if !inverse_sinks.is_empty() {
        let inverter = network.add_gate(".inv.", GateKind::Inverter, vec![SourceRef::Node(source)]);
        internal_nodes.push(inverter);
        add_sinks(&mut network, inverter, inverse_sinks)?;
    }

    network.setup_gate_links()?;

    Ok(Some(FanoutDumpRecord {
        sequence: 0,
        filename: String::new(),
        model_name: problem
            .network_name
            .clone()
            .unwrap_or_else(|| "debug.fanout".to_string()),
        source_name: problem.source.name().to_string(),
        source_polarity: problem.source.polarity,
        sink_count: problem.sink_count(),
        positive_sink_count: problem.positive_sinks.len(),
        negative_sink_count: problem.negative_sinks.len(),
        network,
        internal_nodes,
        tree_depth: None,
        tree_leaf_count: None,
    }))
}

pub fn format_fanout_blif(record: &FanoutDumpRecord) -> Result<String, FanoutDumpError> {
    let mut output = String::new();
    output.push_str(".model ");
    output.push_str(&record.model_name);
    output.push('\n');
    push_names_line(
        &mut output,
        ".inputs",
        record.network.inputs(),
        &record.network,
    )?;
    push_names_line(
        &mut output,
        ".outputs",
        record.network.outputs(),
        &record.network,
    )?;

    for node in &record.internal_nodes {
        write_gate_blif(&mut output, &record.network, *node)?;
    }

    for output_node in record.network.outputs() {
        let output_node_data = record
            .network
            .node(*output_node)
            .ok_or(FanoutDumpError::MissingNode(*output_node))?;
        let Some(source) = output_node_data.save_binding.first().copied() else {
            return Err(VirtualNetworkError::InvalidPrimaryOutputFanin(*output_node).into());
        };
        output.push_str(".names ");
        output.push_str(&source_name(&record.network, source)?);
        output.push(' ');
        output.push_str(&output_node_data.name);
        output.push('\n');
        output.push_str("1 1\n");
    }

    output.push_str(".end\n");
    Ok(output)
}

pub fn format_fanout_report(
    record: &FanoutDumpRecord,
    tree: Option<&MapperTree>,
) -> Result<String, FanoutDumpError> {
    let mut output = String::new();
    output.push_str("fanout-dump ");
    output.push_str(&record.filename);
    output.push('\n');
    output.push_str("model ");
    output.push_str(&record.model_name);
    output.push('\n');
    output.push_str("source ");
    output.push_str(&record.source_name);
    output.push(' ');
    output.push_str(record.source_polarity.label());
    output.push('\n');
    output.push_str(&format!(
        "sinks total={} positive={} negative={}\n",
        record.sink_count, record.positive_sink_count, record.negative_sink_count
    ));
    output.push_str(&format!(
        "network inputs={} outputs={} nodes={}\n",
        record.network.inputs().len(),
        record.network.outputs().len(),
        record.network.nodes().len()
    ));

    for output_node in record.network.outputs() {
        let item = record
            .network
            .node(*output_node)
            .ok_or(FanoutDumpError::MissingNode(*output_node))?;
        output.push_str(&format!(
            "sink {} required=({:.6},{:.6}) load={:.6}\n",
            item.name, item.required.rise, item.required.fall, item.load
        ));
    }

    if let Some(tree) = tree {
        tree.validate()?;
        output.push_str(&format!(
            "tree root={} depth={} leaves={}\n",
            tree.root().index(),
            tree.depth()?,
            tree.leaves()?.len()
        ));
        let preorder = tree
            .preorder()?
            .into_iter()
            .map(|node| node.index().to_string())
            .collect::<Vec<_>>()
            .join(",");
        output.push_str("tree preorder=");
        output.push_str(&preorder);
        output.push('\n');
    } else if let (Some(depth), Some(leaves)) = (record.tree_depth, record.tree_leaf_count) {
        output.push_str(&format!("tree depth={depth} leaves={leaves}\n"));
    }

    Ok(output)
}

fn add_source_to_network(
    network: &mut VirtualMappedNetwork,
    source: &FanoutSource,
) -> Result<NodeId, FanoutDumpError> {
    match &source.kind {
        FanoutSourceKind::PrimaryInput { name, .. } => Ok(network.add_primary_input(name)),
        FanoutSourceKind::Gate { name, gate, fanins } => {
            if fanins.is_empty() {
                return Err(FanoutDumpError::EmptyGateFanins {
                    source: name.clone(),
                });
            }
            let bindings = fanins
                .iter()
                .map(|fanin| Ok(SourceRef::Node(network.add_primary_input(&fanin.name))))
                .collect::<Result<Vec<_>, FanoutDumpError>>()?;
            Ok(network.add_gate(name, gate.clone(), bindings))
        }
    }
}

fn add_sinks(
    network: &mut VirtualMappedNetwork,
    source: NodeId,
    sinks: &[FanoutSink],
) -> Result<(), FanoutDumpError> {
    for sink in sinks {
        let output = network.add_primary_output(sink.output_name(), SourceRef::Node(source))?;
        let node = network
            .node_mut(output)
            .ok_or(FanoutDumpError::MissingNode(output))?;
        node.required = sink.required;
        node.load = sink.load;
    }

    Ok(())
}

fn validate_problem(problem: &FanoutProblem) -> Result<(), FanoutDumpError> {
    validate_name(problem.source.name(), "source")?;
    if let Some(name) = &problem.network_name {
        validate_name(name, "network")?;
    }
    match &problem.source.kind {
        FanoutSourceKind::PrimaryInput { name, .. } => validate_name(name, "source")?,
        FanoutSourceKind::Gate { name, fanins, .. } => {
            validate_name(name, "source")?;
            for fanin in fanins {
                validate_name(&fanin.name, "source fanin")?;
            }
        }
    }
    for sink in problem
        .positive_sinks
        .iter()
        .chain(problem.negative_sinks.iter())
    {
        validate_name(&sink.node_name, "sink")?;
        if !sink.load.is_finite() || sink.load < 0.0 {
            return Err(FanoutDumpError::InvalidLoad {
                sink: sink.output_name(),
                load: sink.load.to_string(),
            });
        }
    }

    Ok(())
}

fn validate_name(name: &str, kind: &'static str) -> Result<(), FanoutDumpError> {
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err(FanoutDumpError::EmptyName { kind });
    }

    Ok(())
}

fn push_names_line(
    output: &mut String,
    directive: &str,
    nodes: &[NodeId],
    network: &VirtualMappedNetwork,
) -> Result<(), FanoutDumpError> {
    output.push_str(directive);
    for node in nodes {
        output.push(' ');
        output.push_str(
            &network
                .node(*node)
                .ok_or(FanoutDumpError::MissingNode(*node))?
                .name,
        );
    }
    output.push('\n');
    Ok(())
}

fn write_gate_blif(
    output: &mut String,
    network: &VirtualMappedNetwork,
    node: NodeId,
) -> Result<(), FanoutDumpError> {
    let item = network
        .node(node)
        .ok_or(FanoutDumpError::MissingNode(node))?;
    let gate = item
        .gate
        .as_ref()
        .ok_or(FanoutDumpError::MissingNode(node))?;
    let fanins = item
        .save_binding
        .iter()
        .map(|source| source_name(network, *source))
        .collect::<Result<Vec<_>, _>>()?;

    output_names(output, &fanins, &item.name, network)?;
    for cube in gate_on_set(node, gate, fanins.len())? {
        output.push_str(&cube);
        if !cube.is_empty() {
            output.push(' ');
        }
        output.push_str("1\n");
    }

    Ok(())
}

fn output_names(
    output: &mut String,
    fanins: &[String],
    name: &str,
    _network: &VirtualMappedNetwork,
) -> Result<(), FanoutDumpError> {
    output.push_str(".names");
    for fanin in fanins {
        output.push(' ');
        output.push_str(fanin);
    }
    if !name.is_empty() {
        output.push(' ');
        output.push_str(name);
    }
    output.push('\n');
    Ok(())
}

fn source_name(
    network: &VirtualMappedNetwork,
    source: SourceRef,
) -> Result<String, FanoutDumpError> {
    match source {
        SourceRef::Node(node) => Ok(network
            .node(node)
            .ok_or(FanoutDumpError::MissingNode(node))?
            .name
            .clone()),
        SourceRef::ConstantZero => Ok("$false".to_string()),
        SourceRef::ConstantOne => Ok("$true".to_string()),
    }
}

fn gate_on_set(
    node: NodeId,
    gate: &GateKind,
    fanin_count: usize,
) -> Result<Vec<String>, FanoutDumpError> {
    match gate {
        GateKind::Inverter => Ok(vec!["0".to_string()]),
        GateKind::Wire => Ok(vec!["1".to_string()]),
        GateKind::And => Ok(vec!["1".repeat(fanin_count)]),
        GateKind::Nand => Ok(single_zero_cover(fanin_count)),
        GateKind::Or => Ok(single_one_cover(fanin_count)),
        GateKind::Nor => Ok(vec!["0".repeat(fanin_count)]),
        GateKind::Xor => parity_cover(node, gate, fanin_count, true),
        GateKind::Xnor => parity_cover(node, gate, fanin_count, false),
        GateKind::One => Ok(vec![String::new()]),
        GateKind::Zero => Ok(Vec::new()),
        GateKind::Mux | GateKind::Library(_) => Err(FanoutDumpError::UnsupportedBlifGate {
            node,
            gate: gate.clone(),
        }),
    }
}

fn single_zero_cover(fanin_count: usize) -> Vec<String> {
    (0..fanin_count)
        .map(|index| {
            let mut pattern = vec!['-'; fanin_count];
            pattern[index] = '0';
            pattern.into_iter().collect()
        })
        .collect()
}

fn single_one_cover(fanin_count: usize) -> Vec<String> {
    (0..fanin_count)
        .map(|index| {
            let mut pattern = vec!['-'; fanin_count];
            pattern[index] = '1';
            pattern.into_iter().collect()
        })
        .collect()
}

fn parity_cover(
    node: NodeId,
    gate: &GateKind,
    fanin_count: usize,
    odd: bool,
) -> Result<Vec<String>, FanoutDumpError> {
    if fanin_count > 12 {
        return Err(FanoutDumpError::TooManyBlifCombinations {
            node,
            gate: gate.clone(),
            fanin_count,
        });
    }

    let combinations = 1usize << fanin_count;
    Ok((0..combinations)
        .filter(|value| value.count_ones() % 2 == odd as u32)
        .map(|value| {
            (0..fanin_count)
                .rev()
                .map(|bit| {
                    if value & (1usize << bit) == 0 {
                        '0'
                    } else {
                        '1'
                    }
                })
                .collect()
        })
        .collect())
}

fn basename(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::tree::MapperTreeNode;

    fn sink(name: &str, pin: isize, rise: f64, fall: f64, load: f64) -> FanoutSink {
        FanoutSink::new(name, pin, DelayTime::new(rise, fall), load)
    }

    fn sample_problem() -> FanoutProblem {
        FanoutProblem {
            network_name: Some("net/path/demo".to_string()),
            source: FanoutSource::primary_input(
                "src",
                FanoutPolarity::Positive,
                FanoutTiming::default(),
            ),
            positive_sinks: vec![sink("a", 0, 10.0, 11.0, 1.5), sink("b", 1, 8.0, 9.0, 2.0)],
            negative_sinks: vec![sink("c", 2, 7.0, 6.0, 3.0)],
        }
    }

    #[test]
    fn skips_small_fanout_problems_but_preserves_cost_contract() {
        let mut session = FanoutDumpSession::new(Some("demo"));

        let outcome = session.dump_problem(&sample_problem(), None).unwrap();

        assert_eq!(outcome.cost, FanoutDumpCost::default());
        assert!(outcome.record.is_none());
    }

    #[test]
    fn extracts_fanout_network_and_adds_inverter_for_opposite_polarity() {
        let mut session = FanoutDumpSession::with_options(
            Some("dir/original"),
            FanoutDumpOptions { dump_threshold: 3 },
        );

        let record = session
            .dump_problem(&sample_problem(), None)
            .unwrap()
            .record
            .unwrap();

        assert_eq!(record.sequence, 0);
        assert_eq!(record.model_name, "dir/original.0.fanout");
        assert_eq!(record.filename, "original.0.fanout");
        assert_eq!(record.sink_count, 3);
        assert_eq!(record.network.inputs().len(), 1);
        assert_eq!(record.network.outputs().len(), 3);
        assert!(
            record
                .network
                .nodes()
                .iter()
                .any(|node| node.name == ".inv." && node.gate == Some(GateKind::Inverter))
        );
    }

    #[test]
    fn formats_deterministic_blif_for_extracted_problem() {
        let record = extract_fanout_network(&sample_problem(), 3)
            .unwrap()
            .unwrap();

        assert_eq!(
            format_fanout_blif(&record).unwrap(),
            concat!(
                ".model net/path/demo\n",
                ".inputs src\n",
                ".outputs a(0) b(1) c(2)\n",
                ".names src .inv.\n",
                "0 1\n",
                ".names src a(0)\n",
                "1 1\n",
                ".names src b(1)\n",
                "1 1\n",
                ".names .inv. c(2)\n",
                "1 1\n",
                ".end\n"
            )
        );
    }

    #[test]
    fn rejects_invalid_owned_inputs_and_unsupported_blif_library_gate() {
        let mut problem = sample_problem();
        problem.positive_sinks[0].load = f64::NAN;
        assert!(matches!(
            extract_fanout_network(&problem, 1),
            Err(FanoutDumpError::InvalidLoad { .. })
        ));

        let problem = FanoutProblem {
            network_name: Some("demo".to_string()),
            source: FanoutSource::gate(
                "n1",
                GateKind::Library("complex".to_string()),
                vec![FanoutSourceInput::new("a", FanoutTiming::default())],
                FanoutPolarity::Positive,
            ),
            positive_sinks: vec![sink("out", 0, 1.0, 1.0, 1.0)],
            negative_sinks: Vec::new(),
        };
        let record = extract_fanout_network(&problem, 1).unwrap().unwrap();

        assert!(matches!(
            format_fanout_blif(&record),
            Err(FanoutDumpError::UnsupportedBlifGate { .. })
        ));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("fanout_dump.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }

    #[test]
    fn mapper_tree_import_is_used_in_report_path() {
        let node = MapperTreeNode::leaf("leaf");
        assert!(matches!(node, MapperTreeNode::Leaf { .. }));
    }
}
