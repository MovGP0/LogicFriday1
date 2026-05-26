//! Native Rust model for `LogicSynthesis/sis/power/power_pipe.c`.
//!
//! The C file evaluates static pipelined circuits by building the symbolic
//! transition network from `power_sim.c`, splicing in two copies of zero-delay
//! latch-bypass logic (`000` and `ttt`), then accumulating BDD-derived output
//! probabilities into `power_info_t`.  This port keeps the graph rewrite and
//! final accumulation behavior on owned Rust data structures.  Direct SIS
//! `network_t`, `latch_t`, `st_table`, and BDD integration remains blocked by
//! the dependency beads listed below.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const CAPACITANCE: f64 = 0.01;
pub const POWER_SCALE: f64 = 250.0;
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipeNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub copy_of: Option<NodeId>,
    pub is_real_po: bool,
}

impl PipeNode {
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            fanins: Vec::new(),
            copy_of: None,
            is_real_po: kind == NodeKind::PrimaryOutput,
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_copy_of(mut self, copy_of: NodeId) -> Self {
        self.copy_of = Some(copy_of);
        self
    }

    pub fn real_po(mut self, is_real_po: bool) -> Self {
        self.is_real_po = is_real_po;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PipeLatch {
    pub input: NodeId,
    pub output: NodeId,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PipeNetwork {
    pub nodes: BTreeMap<NodeId, PipeNode>,
    pub latches: Vec<PipeLatch>,
}

impl PipeNetwork {
    pub fn insert(&mut self, node: PipeNode) -> Result<(), PowerPipeError> {
        if self.nodes.insert(node.id, node).is_some() {
            return Err(PowerPipeError::DuplicateNode);
        }
        Ok(())
    }

    pub fn add_latch(&mut self, input: NodeId, output: NodeId) -> Result<(), PowerPipeError> {
        self.node(input)?;
        self.node(output)?;
        self.latches.push(PipeLatch { input, output });
        Ok(())
    }

    pub fn node(&self, id: NodeId) -> Result<&PipeNode, PowerPipeError> {
        self.nodes.get(&id).ok_or(PowerPipeError::MissingNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut PipeNode, PowerPipeError> {
        self.nodes
            .get_mut(&id)
            .ok_or(PowerPipeError::MissingNode(id))
    }

    pub fn find_by_name(&self, name: &str) -> Option<NodeId> {
        self.nodes
            .iter()
            .find_map(|(id, node)| (node.name == name).then_some(*id))
    }

    pub fn fanouts_of(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter_map(|(candidate, node)| node.fanins.contains(&id).then_some(*candidate))
            .collect()
    }

    pub fn primary_inputs(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| (node.kind == NodeKind::PrimaryInput).then_some(*id))
            .collect()
    }

    pub fn primary_outputs(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| (node.kind == NodeKind::PrimaryOutput).then_some(*id))
            .collect()
    }

    pub fn next_node_id(&self) -> NodeId {
        NodeId(
            self.nodes
                .keys()
                .map(|id| id.0)
                .max()
                .map_or(0, |id| id + 1),
        )
    }

    pub fn validate(&self) -> Result<(), PowerPipeError> {
        for node in self.nodes.values() {
            for fanin in &node.fanins {
                if !self.nodes.contains_key(fanin) {
                    return Err(PowerPipeError::MissingFanin {
                        node: node.id,
                        fanin: *fanin,
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PipeInstant {
    Zero,
    Transition,
}

impl PipeInstant {
    fn c_suffix(self) -> &'static str {
        match self {
            Self::Zero => "000",
            Self::Transition => "ttt",
        }
    }

    fn copy_prefix(self) -> &'static str {
        match self {
            Self::Zero => "n0_",
            Self::Transition => "nT_",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcatenateReport {
    pub instant: PipeInstant,
    pub copied_nodes: BTreeMap<NodeId, NodeId>,
    pub removed_prefixed_inputs: BTreeSet<NodeId>,
    pub removed_prefixed_outputs: BTreeSet<NodeId>,
    pub removed_symbolic_inputs: BTreeSet<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipelineLogicReport {
    pub deleted_real_outputs: Vec<NodeId>,
    pub shorted_latches: Vec<PipeLatch>,
    pub swept_nodes: Vec<NodeId>,
    pub concatenations: Vec<ConcatenateReport>,
    pub deleted_dangling_symbolic_inputs: Vec<NodeId>,
}

pub fn add_pipeline_logic(
    original: &PipeNetwork,
    symbolic: &mut PipeNetwork,
) -> Result<PipelineLogicReport, PowerPipeError> {
    let mut zero_delay = original.clone();

    let deleted_real_outputs: Vec<_> = zero_delay
        .primary_outputs()
        .into_iter()
        .filter(|id| zero_delay.node(*id).is_ok_and(|node| node.is_real_po))
        .collect();
    for node in &deleted_real_outputs {
        zero_delay.nodes.remove(node);
    }

    let shorted_latches = zero_delay.latches.clone();
    for latch in &shorted_latches {
        let driver = zero_delay
            .node(latch.input)?
            .fanins
            .first()
            .copied()
            .unwrap_or(latch.input);
        let output = zero_delay.node_mut(latch.output)?;
        output.kind = NodeKind::PrimaryOutput;
        output.fanins = vec![driver];
        replace_fanin(&mut zero_delay, latch.output, driver);
    }
    zero_delay.latches.clear();

    let swept_nodes = sweep_unreachable(&mut zero_delay);
    let concatenations = vec![
        concatenate_pipeline_network(symbolic, &zero_delay, PipeInstant::Zero)?,
        concatenate_pipeline_network(symbolic, &zero_delay, PipeInstant::Transition)?,
    ];

    let deleted_dangling_symbolic_inputs: Vec<_> = symbolic
        .primary_inputs()
        .into_iter()
        .filter(|node| symbolic.fanouts_of(*node).is_empty())
        .collect();
    for node in &deleted_dangling_symbolic_inputs {
        symbolic.nodes.remove(node);
    }

    symbolic.validate()?;
    Ok(PipelineLogicReport {
        deleted_real_outputs,
        shorted_latches,
        swept_nodes,
        concatenations,
        deleted_dangling_symbolic_inputs,
    })
}

pub fn concatenate_pipeline_network(
    symbolic: &mut PipeNetwork,
    zero_delay: &PipeNetwork,
    instant: PipeInstant,
) -> Result<ConcatenateReport, PowerPipeError> {
    let base = symbolic.next_node_id().0;
    let mut copied_nodes = BTreeMap::new();
    let mut copied_inputs = Vec::new();
    let mut copied_outputs = Vec::new();

    for (offset, source) in zero_delay.nodes.values().enumerate() {
        let copied_id = NodeId(base + offset);
        let mut copied = source.clone();
        copied.id = copied_id;
        copied.name = format!("{}{}", instant.copy_prefix(), source.name);
        copied.fanins.clear();
        copied_nodes.insert(source.id, copied_id);
        if copied.kind == NodeKind::PrimaryInput {
            copied_inputs.push(copied_id);
        }
        if copied.kind == NodeKind::PrimaryOutput {
            copied_outputs.push(copied_id);
        }
        symbolic.insert(copied)?;
    }

    for source in zero_delay.nodes.values() {
        let copied_id = copied_nodes[&source.id];
        let rewritten = source
            .fanins
            .iter()
            .map(|fanin| {
                copied_nodes
                    .get(fanin)
                    .copied()
                    .ok_or(PowerPipeError::MissingFanin {
                        node: source.id,
                        fanin: *fanin,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        symbolic.node_mut(copied_id)?.fanins = rewritten;
    }

    let mut report = ConcatenateReport {
        instant,
        copied_nodes,
        removed_prefixed_inputs: BTreeSet::new(),
        removed_prefixed_outputs: BTreeSet::new(),
        removed_symbolic_inputs: BTreeSet::new(),
    };

    for copied_input in copied_inputs {
        let target_name =
            symbolic_target_name(symbolic.node(copied_input)?.name.as_str(), instant)?;
        let target = symbolic
            .find_by_name(&target_name)
            .ok_or(PowerPipeError::MissingSymbolicNode(target_name))?;
        replace_fanin(symbolic, copied_input, target);
        symbolic.nodes.remove(&copied_input);
        report.removed_prefixed_inputs.insert(copied_input);
    }

    for copied_output in copied_outputs {
        let target_name =
            symbolic_target_name(symbolic.node(copied_output)?.name.as_str(), instant)?;
        let target = symbolic
            .find_by_name(&target_name)
            .ok_or(PowerPipeError::MissingSymbolicNode(target_name))?;
        let replacement = *symbolic
            .node(copied_output)?
            .fanins
            .first()
            .ok_or(PowerPipeError::PrimaryOutputWithoutFanin(copied_output))?;
        replace_fanin(symbolic, target, replacement);
        symbolic.nodes.remove(&copied_output);
        symbolic.nodes.remove(&target);
        report.removed_prefixed_outputs.insert(copied_output);
        report.removed_symbolic_inputs.insert(target);
    }

    symbolic.validate()?;
    Ok(report)
}

fn symbolic_target_name(
    prefixed_name: &str,
    instant: PipeInstant,
) -> Result<String, PowerPipeError> {
    let unprefixed = prefixed_name
        .strip_prefix(instant.copy_prefix())
        .ok_or(PowerPipeError::InvalidPrefixedName)?;
    Ok(format!("{}_{}", unprefixed, instant.c_suffix()))
}

fn replace_fanin(network: &mut PipeNetwork, old_fanin: NodeId, new_fanin: NodeId) {
    for node in network.nodes.values_mut() {
        for fanin in &mut node.fanins {
            if *fanin == old_fanin {
                *fanin = new_fanin;
            }
        }
    }
}

fn sweep_unreachable(network: &mut PipeNetwork) -> Vec<NodeId> {
    let mut reached = BTreeSet::new();
    let outputs = network.primary_outputs();
    for output in outputs {
        mark_reached(network, output, &mut reached);
    }

    let removable: Vec<_> = network
        .nodes
        .keys()
        .copied()
        .filter(|node| !reached.contains(node))
        .collect();
    for node in &removable {
        network.nodes.remove(node);
    }
    removable
}

fn mark_reached(network: &PipeNetwork, node: NodeId, reached: &mut BTreeSet<NodeId>) {
    if !reached.insert(node) {
        return;
    }
    if let Some(node) = network.nodes.get(&node) {
        for fanin in &node.fanins {
            mark_reached(network, *fanin, reached);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerInfo {
    pub cap_factor: f64,
    pub switching_prob: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SymbolicOutputProbability {
    pub symbolic_output: NodeId,
    pub original_node: NodeId,
    pub probability_one: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipelinePowerContribution {
    pub symbolic_output: NodeId,
    pub original_node: NodeId,
    pub probability_one: f64,
    pub scaled_power: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipelinePowerReport {
    pub contributions: Vec<PipelinePowerContribution>,
    pub total_power: f64,
}

pub fn accumulate_arbitrary_pipeline_power(
    power_info: &mut BTreeMap<NodeId, PowerInfo>,
    output_probabilities: &[SymbolicOutputProbability],
) -> Result<PipelinePowerReport, PowerPipeError> {
    let mut contributions = Vec::with_capacity(output_probabilities.len());
    let mut total_power = 0.0;

    for output in output_probabilities {
        if !(0.0..=1.0).contains(&output.probability_one) {
            return Err(PowerPipeError::ProbabilityOutOfRange(
                output.probability_one,
            ));
        }

        let info = power_info
            .get_mut(&output.original_node)
            .ok_or(PowerPipeError::MissingPowerInfo(output.original_node))?;
        let scaled_power = info.cap_factor * output.probability_one * CAPACITANCE * POWER_SCALE;
        info.switching_prob += output.probability_one;
        total_power += scaled_power;
        contributions.push(PipelinePowerContribution {
            symbolic_output: output.symbolic_output,
            original_node: output.original_node,
            probability_one: output.probability_one,
            scaled_power,
        });
    }

    Ok(PipelinePowerReport {
        contributions,
        total_power,
    })
}

pub fn evaluate_sis_pipeline_power<Network, InfoTable>(
    _network: &Network,
    _info_table: &InfoTable,
) -> Result<PipelinePowerReport, PowerPipeError> {
    Err(PowerPipeError::MissingSisDependencies {
        operation: "power_pipe_arbit",
    })
}

pub fn add_pipeline_logic_to_sis_network<Network>(
    _network: &Network,
    _symbolic: &mut Network,
) -> Result<PipelineLogicReport, PowerPipeError> {
    Err(PowerPipeError::MissingSisDependencies {
        operation: "power_add_pipeline_logic",
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum PowerPipeError {
    MissingSisDependencies { operation: &'static str },
    DuplicateNode,
    MissingNode(NodeId),
    MissingFanin { node: NodeId, fanin: NodeId },
    MissingSymbolicNode(String),
    MissingPowerInfo(NodeId),
    PrimaryOutputWithoutFanin(NodeId),
    ProbabilityOutOfRange(f64),
    InvalidPrefixedName,
}

impl fmt::Display for PowerPipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
            Self::DuplicateNode => write!(f, "pipeline network contains a duplicate node id"),
            Self::MissingNode(node) => write!(f, "pipeline network is missing node {:?}", node),
            Self::MissingFanin { node, fanin } => {
                write!(
                    f,
                    "pipeline node {:?} references missing fanin {:?}",
                    node, fanin
                )
            }
            Self::MissingSymbolicNode(name) => {
                write!(f, "symbolic pipeline network is missing node {name}")
            }
            Self::MissingPowerInfo(node) => {
                write!(
                    f,
                    "missing power_info_t equivalent for original node {:?}",
                    node
                )
            }
            Self::PrimaryOutputWithoutFanin(node) => {
                write!(f, "primary output {:?} has no fanin to splice", node)
            }
            Self::ProbabilityOutOfRange(probability) => {
                write!(f, "probability {probability} is outside 0.0..=1.0")
            }
            Self::InvalidPrefixedName => write!(f, "copied pipeline node name is missing prefix"),
        }
    }
}

impl Error for PowerPipeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize, name: &str, kind: NodeKind) -> PipeNode {
        PipeNode::new(NodeId(id), name, kind)
    }

    fn insert_all(network: &mut PipeNetwork, nodes: Vec<PipeNode>) {
        for node in nodes {
            network.insert(node).unwrap();
        }
    }

    fn zero_delay_network() -> PipeNetwork {
        let mut network = PipeNetwork::default();
        insert_all(
            &mut network,
            vec![
                node(1, "a", NodeKind::PrimaryInput),
                node(2, "state_out", NodeKind::PrimaryInput),
                node(3, "n", NodeKind::Internal).with_fanins([NodeId(1)]),
                node(4, "state_in", NodeKind::PrimaryOutput)
                    .with_fanins([NodeId(3)])
                    .real_po(false),
                node(5, "real_out", NodeKind::PrimaryOutput)
                    .with_fanins([NodeId(3)])
                    .real_po(true),
                node(6, "dead", NodeKind::Internal),
            ],
        );
        network.add_latch(NodeId(4), NodeId(2)).unwrap();
        network
    }

    fn symbolic_network() -> PipeNetwork {
        let mut symbolic = PipeNetwork::default();
        insert_all(
            &mut symbolic,
            vec![
                node(10, "a_000", NodeKind::PrimaryInput),
                node(11, "a_ttt", NodeKind::PrimaryInput),
                node(12, "state_out_000", NodeKind::PrimaryInput),
                node(13, "state_out_ttt", NodeKind::PrimaryInput),
                node(14, "state_in_000", NodeKind::PrimaryInput),
                node(15, "state_in_ttt", NodeKind::PrimaryInput),
                node(16, "x0", NodeKind::Internal).with_fanins([NodeId(12)]),
                node(17, "x1", NodeKind::Internal).with_fanins([NodeId(13)]),
            ],
        );
        symbolic
    }

    #[test]
    fn c_constants_are_preserved() {
        assert_eq!(CAPACITANCE, 0.01);
        assert_eq!(POWER_SCALE, 250.0);
    }

    #[test]
    fn concatenate_prefixes_nodes_and_replaces_symbolic_time_inputs() {
        let mut zero_delay = zero_delay_network();
        zero_delay.nodes.remove(&NodeId(5));
        let mut symbolic = symbolic_network();

        let report =
            concatenate_pipeline_network(&mut symbolic, &zero_delay, PipeInstant::Zero).unwrap();

        assert!(symbolic.find_by_name("n0_a").is_none());
        assert!(symbolic.find_by_name("n0_n").is_some());
        assert_eq!(symbolic.node(NodeId(16)).unwrap().fanins, vec![NodeId(12)]);
        assert!(report.removed_prefixed_inputs.len() >= 2);
    }

    #[test]
    fn add_pipeline_logic_deletes_real_outputs_shorts_latches_and_splices_both_instants() {
        let original = zero_delay_network();
        let mut symbolic = symbolic_network();

        let report = add_pipeline_logic(&original, &mut symbolic).unwrap();

        assert_eq!(report.deleted_real_outputs, vec![NodeId(5)]);
        assert_eq!(
            report.shorted_latches,
            vec![PipeLatch {
                input: NodeId(4),
                output: NodeId(2),
            }]
        );
        assert_eq!(report.swept_nodes, vec![NodeId(6)]);
        assert_eq!(report.concatenations.len(), 2);
        assert!(symbolic.find_by_name("state_in_000").is_none());
        assert!(symbolic.find_by_name("state_in_ttt").is_none());
        assert_eq!(symbolic.node(NodeId(16)).unwrap().fanins.len(), 1);
        assert_eq!(symbolic.node(NodeId(17)).unwrap().fanins.len(), 1);
        assert!(symbolic.validate().is_ok());
    }

    #[test]
    fn arbitrary_pipeline_power_matches_c_accumulation_and_switching_update() {
        let mut power_info = BTreeMap::from([
            (
                NodeId(100),
                PowerInfo {
                    cap_factor: 2.0,
                    switching_prob: 0.25,
                },
            ),
            (
                NodeId(101),
                PowerInfo {
                    cap_factor: 4.0,
                    switching_prob: 0.0,
                },
            ),
        ]);

        let report = accumulate_arbitrary_pipeline_power(
            &mut power_info,
            &[
                SymbolicOutputProbability {
                    symbolic_output: NodeId(10),
                    original_node: NodeId(100),
                    probability_one: 0.5,
                },
                SymbolicOutputProbability {
                    symbolic_output: NodeId(11),
                    original_node: NodeId(101),
                    probability_one: 0.25,
                },
            ],
        )
        .unwrap();

        assert_eq!(report.total_power, 5.0);
        assert_eq!(report.contributions[0].scaled_power, 2.5);
        assert_eq!(power_info[&NodeId(100)].switching_prob, 0.75);
        assert_eq!(power_info[&NodeId(101)].switching_prob, 0.25);
    }

    #[test]
    fn no_legacy_abi_tokens_are_present_in_this_port() {
        let source = include_str!("power_pipe.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
