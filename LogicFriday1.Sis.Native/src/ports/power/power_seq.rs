//! Native Rust scaffold for `LogicSynthesis/sis/power/power_seq.c`.
//!
//! The C file builds a symbolic sequential power network, appends next-state
//! logic, orders present-state inputs before normal inputs for exact state
//! probabilities, and accumulates `cap_factor * probability * CAPACITANCE *
//! 250.0` into the total power. This module keeps those data-flow rules in
//! native Rust types. The SIS network, BDD, st_table, and companion power
//! routines are still separate porting beads, so SIS-bound entry points return
//! explicit dependency errors instead of exposing legacy symbols.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const CAPACITANCE: f64 = 0.01;
pub const POWER_SCALE: f64 = 250.0;
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PsLineOption {
    Approximation,
    Exact,
    StateLine,
    Uniform,
}

impl PsLineOption {
    pub fn from_c_value(value: i32) -> Option<Self> {
        match value {
            20 => Some(Self::Approximation),
            21 => Some(Self::Exact),
            22 => Some(Self::StateLine),
            23 => Some(Self::Uniform),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolicInput {
    pub node: NodeId,
    pub original_node: NodeId,
    pub probability_one: f64,
}

impl SymbolicInput {
    pub fn new(node: NodeId, original_node: NodeId, probability_one: f64) -> Self {
        Self {
            node,
            original_node,
            probability_one,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolicOutput {
    pub node: NodeId,
    pub original_node: NodeId,
    pub cap_factor: f64,
    pub probability_one: f64,
}

impl SymbolicOutput {
    pub fn new(node: NodeId, original_node: NodeId, cap_factor: f64, probability_one: f64) -> Self {
        Self {
            node,
            original_node,
            cap_factor,
            probability_one,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PiPowerInfo {
    pub node: NodeId,
    pub probability_one: f64,
    pub ps_line_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodePowerContribution {
    pub node: NodeId,
    pub original_node: NodeId,
    pub probability_one: f64,
    pub scaled_power: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequentialPowerReport {
    pub ordered_inputs: Vec<PiPowerInfo>,
    pub contributions: Vec<NodePowerContribution>,
    pub total_power: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequentialPowerModel {
    pub inputs: Vec<SymbolicInput>,
    pub outputs: Vec<SymbolicOutput>,
    pub state_line_index: BTreeMap<NodeId, usize>,
    pub ps_order: Vec<NodeId>,
    pub set_size: usize,
}

impl SequentialPowerModel {
    pub fn new(inputs: Vec<SymbolicInput>, outputs: Vec<SymbolicOutput>) -> Self {
        Self {
            inputs,
            outputs,
            state_line_index: BTreeMap::new(),
            ps_order: Vec::new(),
            set_size: 1,
        }
    }

    pub fn evaluate(&self, option: PsLineOption) -> Result<SequentialPowerReport, PowerSeqError> {
        if self.set_size == 0 {
            return Err(PowerSeqError::InvalidSetSize(0));
        }

        let mut ordered_inputs = self.inputs.clone();
        match option {
            PsLineOption::Exact => {
                place_ps_lines_first(&mut ordered_inputs, &self.state_line_index)
            }
            PsLineOption::Approximation if self.set_size != 1 => {
                place_present_state_inputs_first(&mut ordered_inputs, &self.ps_order)
            }
            PsLineOption::Approximation | PsLineOption::StateLine | PsLineOption::Uniform => {}
        }

        let ordered_inputs = ordered_inputs
            .into_iter()
            .map(|input| PiPowerInfo {
                ps_line_index: self.state_line_index.get(&input.original_node).copied(),
                node: input.node,
                probability_one: input.probability_one,
            })
            .collect();

        let contributions: Vec<_> = self
            .outputs
            .iter()
            .map(|output| NodePowerContribution {
                node: output.node,
                original_node: output.original_node,
                probability_one: output.probability_one,
                scaled_power: output.cap_factor
                    * output.probability_one
                    * CAPACITANCE
                    * POWER_SCALE,
            })
            .collect();
        let total_power = contributions
            .iter()
            .map(|contribution| contribution.scaled_power)
            .sum();

        Ok(SequentialPowerReport {
            ordered_inputs,
            contributions,
            total_power,
        })
    }
}

pub fn place_ps_lines_first(
    pi_order: &mut Vec<SymbolicInput>,
    state_line_index: &BTreeMap<NodeId, usize>,
) {
    pi_order.sort_by_key(|input| {
        (
            !state_line_index.contains_key(&input.original_node),
            state_line_index
                .get(&input.original_node)
                .copied()
                .unwrap_or(usize::MAX),
        )
    });
}

pub fn place_present_state_inputs_first(pi_order: &mut Vec<SymbolicInput>, ps_order: &[NodeId]) {
    let ps_rank: BTreeMap<_, _> = ps_order
        .iter()
        .copied()
        .enumerate()
        .map(|(rank, node)| (node, rank))
        .collect();
    pi_order.sort_by_key(|input| {
        (
            !ps_rank.contains_key(&input.original_node),
            ps_rank
                .get(&input.original_node)
                .copied()
                .unwrap_or(usize::MAX),
        )
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub copy_of: Option<NodeId>,
}

impl NetworkNode {
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            fanins: Vec::new(),
            copy_of: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PowerNetwork {
    pub nodes: BTreeMap<NodeId, NetworkNode>,
}

impl PowerNetwork {
    pub fn insert(&mut self, node: NetworkNode) {
        self.nodes.insert(node.id, node);
    }

    pub fn node(&self, id: NodeId) -> Result<&NetworkNode, PowerSeqError> {
        self.nodes.get(&id).ok_or(PowerSeqError::MissingNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut NetworkNode, PowerSeqError> {
        self.nodes
            .get_mut(&id)
            .ok_or(PowerSeqError::MissingNode(id))
    }

    pub fn fanouts_of(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter_map(|(candidate, node)| node.fanins.contains(&id).then_some(*candidate))
            .collect()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConcatenateLinks {
    pub po_links: BTreeMap<NodeId, NodeId>,
    pub ps_links: BTreeMap<NodeId, NodeId>,
    pub pi_links: BTreeMap<NodeId, NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcatenateReport {
    pub copied_nodes: BTreeMap<NodeId, NodeId>,
    pub deleted_symbolic_inputs: BTreeSet<NodeId>,
    pub deleted_ns_outputs: BTreeSet<NodeId>,
}

pub fn concatenate_networks(
    symbolic: &mut PowerNetwork,
    ns_logic: &PowerNetwork,
    links: &ConcatenateLinks,
) -> Result<ConcatenateReport, PowerSeqError> {
    let next_id = symbolic
        .nodes
        .keys()
        .map(|id| id.0)
        .max()
        .map(|id| id + 1)
        .unwrap_or(0);
    let mut copied_nodes = BTreeMap::new();

    for (offset, node) in ns_logic.nodes.values().enumerate() {
        let copied_id = NodeId(next_id + offset);
        let mut copied = node.clone();
        copied.id = copied_id;
        copied.name = format!("{}_nsl", node.name);
        copied.fanins.clear();
        copied_nodes.insert(node.id, copied_id);
        symbolic.insert(copied);
    }

    for node in ns_logic.nodes.values() {
        let copied_id = copied_nodes[&node.id];
        let rewritten_fanins: Result<Vec<_>, _> = node
            .fanins
            .iter()
            .map(|fanin| {
                copied_nodes
                    .get(fanin)
                    .copied()
                    .ok_or(PowerSeqError::MissingNode(*fanin))
            })
            .collect();
        symbolic.node_mut(copied_id)?.fanins = rewritten_fanins?;
    }

    let mut report = ConcatenateReport {
        copied_nodes,
        deleted_symbolic_inputs: BTreeSet::new(),
        deleted_ns_outputs: BTreeSet::new(),
    };

    for (ns_po, symbolic_pi) in &links.po_links {
        let copied_po = report
            .copied_nodes
            .get(ns_po)
            .copied()
            .ok_or(PowerSeqError::MissingNode(*ns_po))?;
        require_kind(symbolic, *symbolic_pi, NodeKind::PrimaryInput)?;
        require_kind(symbolic, copied_po, NodeKind::PrimaryOutput)?;
        let replacement = *symbolic
            .node(copied_po)?
            .fanins
            .first()
            .ok_or(PowerSeqError::PrimaryOutputWithoutFanin(copied_po))?;
        replace_fanin(symbolic, *symbolic_pi, replacement);
        symbolic.nodes.remove(symbolic_pi);
        symbolic.nodes.remove(&copied_po);
        report.deleted_symbolic_inputs.insert(*symbolic_pi);
        report.deleted_ns_outputs.insert(copied_po);
    }

    for (ns_pi, symbolic_pi) in links.ps_links.iter().chain(&links.pi_links) {
        let copied_pi = report
            .copied_nodes
            .get(ns_pi)
            .copied()
            .ok_or(PowerSeqError::MissingNode(*ns_pi))?;
        require_kind(symbolic, *symbolic_pi, NodeKind::PrimaryInput)?;
        require_kind(symbolic, copied_pi, NodeKind::PrimaryInput)?;
        let original_copy = symbolic.node(*symbolic_pi)?.copy_of;
        replace_fanin(symbolic, *symbolic_pi, copied_pi);
        symbolic.node_mut(copied_pi)?.copy_of = original_copy;
        symbolic.nodes.remove(symbolic_pi);
        report.deleted_symbolic_inputs.insert(*symbolic_pi);
    }

    Ok(report)
}

fn require_kind(
    network: &PowerNetwork,
    node: NodeId,
    expected: NodeKind,
) -> Result<(), PowerSeqError> {
    let actual = network.node(node)?.kind;
    if actual == expected {
        Ok(())
    } else {
        Err(PowerSeqError::UnexpectedNodeKind {
            node,
            expected,
            actual,
        })
    }
}

fn replace_fanin(network: &mut PowerNetwork, old_fanin: NodeId, new_fanin: NodeId) {
    for node in network.nodes.values_mut() {
        for fanin in &mut node.fanins {
            if *fanin == old_fanin {
                *fanin = new_fanin;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PowerSeqError {
    InvalidSetSize(usize),
    MissingNode(NodeId),
    PrimaryOutputWithoutFanin(NodeId),
    UnexpectedNodeKind {
        node: NodeId,
        expected: NodeKind,
        actual: NodeKind,
    },
    MissingNativePorts {
        operation: &'static str,
    },
}

impl fmt::Display for PowerSeqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSetSize(set_size) => {
                write!(f, "power_setSize must be at least 1, got {set_size}")
            }
            Self::MissingNode(node) => write!(f, "power network is missing node {:?}", node),
            Self::PrimaryOutputWithoutFanin(node) => {
                write!(f, "primary output {:?} has no fanin to link", node)
            }
            Self::UnexpectedNodeKind {
                node,
                expected,
                actual,
            } => write!(
                f,
                "node {:?} has kind {:?}, expected {:?}",
                node, actual, expected
            ),
            Self::MissingNativePorts { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
        }
    }
}

impl Error for PowerSeqError {}

pub fn evaluate_sis_sequential_power<Network>(
    _network: &Network,
    _option: PsLineOption,
) -> Result<SequentialPowerReport, PowerSeqError> {
    Err(PowerSeqError::MissingNativePorts {
        operation: "evaluate_sis_sequential_power",
    })
}

pub fn add_fsm_state_logic_to_sis_network<Network>(
    _network: &mut Network,
) -> Result<(), PowerSeqError> {
    Err(PowerSeqError::MissingNativePorts {
        operation: "add_fsm_state_logic_to_sis_network",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(id: usize, original: usize, probability_one: f64) -> SymbolicInput {
        SymbolicInput::new(NodeId(id), NodeId(original), probability_one)
    }

    fn output(id: usize, original: usize, cap_factor: f64, probability_one: f64) -> SymbolicOutput {
        SymbolicOutput::new(NodeId(id), NodeId(original), cap_factor, probability_one)
    }

    #[test]
    fn c_constants_and_options_are_preserved() {
        assert_eq!(CAPACITANCE, 0.01);
        assert_eq!(POWER_SCALE, 250.0);
        assert_eq!(
            PsLineOption::from_c_value(20),
            Some(PsLineOption::Approximation)
        );
        assert_eq!(PsLineOption::from_c_value(21), Some(PsLineOption::Exact));
        assert_eq!(
            PsLineOption::from_c_value(22),
            Some(PsLineOption::StateLine)
        );
        assert_eq!(PsLineOption::from_c_value(23), Some(PsLineOption::Uniform));
        assert_eq!(PsLineOption::from_c_value(99), None);
    }

    #[test]
    fn exact_option_places_present_state_lines_first_and_records_indexes() {
        let mut model = SequentialPowerModel::new(
            vec![
                input(10, 100, 0.1),
                input(11, 101, 0.2),
                input(12, 102, 0.3),
            ],
            vec![output(20, 200, 2.0, 0.25)],
        );
        model.state_line_index = BTreeMap::from([(NodeId(102), 0), (NodeId(100), 1)]);

        let report = model.evaluate(PsLineOption::Exact).unwrap();

        assert_eq!(
            report.ordered_inputs,
            vec![
                PiPowerInfo {
                    node: NodeId(12),
                    probability_one: 0.3,
                    ps_line_index: Some(0),
                },
                PiPowerInfo {
                    node: NodeId(10),
                    probability_one: 0.1,
                    ps_line_index: Some(1),
                },
                PiPowerInfo {
                    node: NodeId(11),
                    probability_one: 0.2,
                    ps_line_index: None,
                },
            ]
        );
    }

    #[test]
    fn approximation_with_sets_places_ps_inputs_before_plain_inputs() {
        let mut model = SequentialPowerModel::new(
            vec![
                input(10, 100, 0.1),
                input(11, 101, 0.2),
                input(12, 102, 0.3),
            ],
            Vec::new(),
        );
        model.set_size = 2;
        model.ps_order = vec![NodeId(101), NodeId(100)];

        let report = model.evaluate(PsLineOption::Approximation).unwrap();

        assert_eq!(
            report
                .ordered_inputs
                .iter()
                .map(|input| input.node)
                .collect::<Vec<_>>(),
            vec![NodeId(11), NodeId(10), NodeId(12)]
        );
    }

    #[test]
    fn total_power_matches_c_accumulation_and_final_scale() {
        let model = SequentialPowerModel::new(
            Vec::new(),
            vec![output(20, 200, 2.0, 0.25), output(21, 201, 4.0, 0.5)],
        );

        let report = model.evaluate(PsLineOption::Uniform).unwrap();

        assert_eq!(report.contributions[0].scaled_power, 1.25);
        assert_eq!(report.contributions[1].scaled_power, 5.0);
        assert_eq!(report.total_power, 6.25);
    }

    #[test]
    fn concatenate_copies_ns_logic_and_rewires_link_tables() {
        let mut symbolic = PowerNetwork::default();
        symbolic.insert(NetworkNode::new(
            NodeId(1),
            "sym_pi",
            NodeKind::PrimaryInput,
        ));
        symbolic.insert(NetworkNode {
            id: NodeId(2),
            name: "sym_internal".to_string(),
            kind: NodeKind::Internal,
            fanins: vec![NodeId(1)],
            copy_of: None,
        });

        let mut ns_logic = PowerNetwork::default();
        ns_logic.insert(NetworkNode::new(
            NodeId(10),
            "state",
            NodeKind::PrimaryInput,
        ));
        ns_logic.insert(NetworkNode {
            id: NodeId(11),
            name: "next".to_string(),
            kind: NodeKind::Internal,
            fanins: vec![NodeId(10)],
            copy_of: None,
        });

        let links = ConcatenateLinks {
            ps_links: BTreeMap::from([(NodeId(10), NodeId(1))]),
            ..ConcatenateLinks::default()
        };

        let report = concatenate_networks(&mut symbolic, &ns_logic, &links).unwrap();
        let copied_state = report.copied_nodes[&NodeId(10)];
        let copied_next = report.copied_nodes[&NodeId(11)];

        assert_eq!(symbolic.node(NodeId(2)).unwrap().fanins, vec![copied_state]);
        assert_eq!(
            symbolic.node(copied_next).unwrap().fanins,
            vec![copied_state]
        );
        assert!(symbolic.node(NodeId(1)).is_err());
    }

    #[test]
    fn no_legacy_abi_tokens_are_present_in_this_port() {
        let source = include_str!("power_seq.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
