//! Native Rust model for `LogicSynthesis/sis/power/power_sim.c`.
//!
//! The original C routine builds a symbolic network that exposes one XOR
//! primary output for each possible gate transition. This port keeps that
//! timing/snapshot behavior in an owned Rust graph. Direct integration with
//! SIS `network_t`, `node_t`, `array_t`, and `st_table` remains blocked until
//! the dependency ports listed below are available.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SymbolicNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerNode {
    pub name: String,
    pub kind: PowerNodeKind,
    pub fanins: Vec<NodeId>,
}

impl PowerNode {
    pub fn new(name: impl Into<String>, kind: PowerNodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerNetwork {
    name: String,
    nodes: Vec<PowerNode>,
    dfs_order: Vec<NodeId>,
}

impl PowerNetwork {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            dfs_order: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn nodes(&self) -> &[PowerNode] {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> Result<&PowerNode, PowerSimError> {
        self.nodes.get(id.0).ok_or(PowerSimError::UnknownNode(id))
    }

    pub fn add_node(&mut self, node: PowerNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        self.dfs_order.push(id);
        id
    }

    pub fn set_dfs_order(&mut self, order: Vec<NodeId>) -> Result<(), PowerSimError> {
        for id in &order {
            self.node(*id)?;
        }
        self.dfs_order = order;
        Ok(())
    }

    pub fn dfs_order(&self) -> &[NodeId] {
        &self.dfs_order
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeInfo {
    pub delay: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SymbolicNodeKind {
    Snapshot,
    Xor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicFaninPatch {
    pub original_fanin: NodeId,
    pub symbolic_fanin: SymbolicNodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicNode {
    pub name: String,
    pub kind: SymbolicNodeKind,
    pub original: NodeId,
    pub fanins: Vec<SymbolicFaninPatch>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicPowerNetwork {
    name: String,
    nodes: Vec<SymbolicNode>,
    primary_outputs: Vec<SymbolicNodeId>,
    max_normalized_delay: i32,
}

impl SymbolicPowerNetwork {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn nodes(&self) -> &[SymbolicNode] {
        &self.nodes
    }

    pub fn primary_outputs(&self) -> &[SymbolicNodeId] {
        &self.primary_outputs
    }

    pub fn max_normalized_delay(&self) -> i32 {
        self.max_normalized_delay
    }

    pub fn node(&self, id: SymbolicNodeId) -> Result<&SymbolicNode, PowerSimError> {
        self.nodes
            .get(id.0)
            .ok_or(PowerSimError::UnknownSymbolicNode(id))
    }

    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            primary_outputs: Vec::new(),
            max_normalized_delay: 0,
        }
    }

    fn add_snapshot(
        &mut self,
        name: impl Into<String>,
        original: NodeId,
        fanins: Vec<SymbolicFaninPatch>,
    ) -> SymbolicNodeId {
        self.add_node(SymbolicNode {
            name: name.into(),
            kind: SymbolicNodeKind::Snapshot,
            original,
            fanins,
        })
    }

    fn add_xor(
        &mut self,
        name: impl Into<String>,
        original: NodeId,
        before: SymbolicNodeId,
        after: SymbolicNodeId,
    ) -> SymbolicNodeId {
        let id = self.add_node(SymbolicNode {
            name: name.into(),
            kind: SymbolicNodeKind::Xor,
            original,
            fanins: vec![
                SymbolicFaninPatch {
                    original_fanin: original,
                    symbolic_fanin: before,
                },
                SymbolicFaninPatch {
                    original_fanin: original,
                    symbolic_fanin: after,
                },
            ],
        });
        self.primary_outputs.push(id);
        id
    }

    fn add_node(&mut self, node: SymbolicNode) -> SymbolicNodeId {
        let id = SymbolicNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerSimError {
    UnknownNode(NodeId),
    UnknownSymbolicNode(SymbolicNodeId),
    MissingNodeInfo { node: NodeId },
    MissingFaninDelayInfo { node: NodeId, fanin: NodeId },
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for PowerSimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown power-sim node {:?}", node),
            Self::UnknownSymbolicNode(node) => {
                write!(f, "unknown symbolic power-sim node {:?}", node)
            }
            Self::MissingNodeInfo { node, .. } => write!(
                f,
                "node {:?} is missing power node_info_t delay data from the SIS info table",
                node
            ),
            Self::MissingFaninDelayInfo { node, fanin, .. } => write!(
                f,
                "node {:?} depends on fanin {:?}, but no symbolic delay_info_t exists for it",
                node, fanin
            ),
            Self::MissingNativePorts { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
        }
    }
}

impl Error for PowerSimError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DelayInfo {
    before_switching: Vec<SymbolicNodeId>,
    after_switching: Vec<SymbolicNodeId>,
    switching_times: Vec<i32>,
}

impl DelayInfo {
    fn new() -> Self {
        Self {
            before_switching: Vec::new(),
            after_switching: Vec::new(),
            switching_times: Vec::new(),
        }
    }

    fn push_transition(&mut self, time: i32, before: SymbolicNodeId, after: SymbolicNodeId) {
        self.switching_times.push(time);
        self.before_switching.push(before);
        self.after_switching.push(after);
    }

    fn first_time(&self) -> i32 {
        self.switching_times[0]
    }

    fn last_time(&self) -> i32 {
        self.switching_times[self.switching_times.len() - 1]
    }

    fn after_at_time(&self, time: i32) -> Option<SymbolicNodeId> {
        self.switching_times
            .iter()
            .position(|frame| *frame == time)
            .map(|index| self.after_switching[index])
    }
}

pub fn power_symbolic_simulate(
    network: &PowerNetwork,
    info_table: &HashMap<NodeId, NodeInfo>,
) -> Result<SymbolicPowerNetwork, PowerSimError> {
    let mut symbolic = SymbolicPowerNetwork::new(format!("{}_symbolic", network.name()));
    let mut internal: HashMap<NodeId, DelayInfo> = HashMap::new();
    let mut max_num_transition = i32::MIN;

    for actual_id in network.dfs_order() {
        let actual = network.node(*actual_id)?;

        match actual.kind {
            PowerNodeKind::PrimaryInput => {
                let before =
                    symbolic.add_snapshot(format!("{}_000", actual.name), *actual_id, Vec::new());
                let after =
                    symbolic.add_snapshot(format!("{}_ttt", actual.name), *actual_id, Vec::new());

                let mut delay_info = DelayInfo::new();
                delay_info.push_transition(0, before, after);
                internal.insert(*actual_id, delay_info);

                symbolic.add_xor(format!("{}_xor_0", actual.name), *actual_id, before, after);
            }
            PowerNodeKind::PrimaryOutput => {}
            PowerNodeKind::Internal if actual.fanins.is_empty() => {
                let snapshot = symbolic.add_snapshot(actual.name.clone(), *actual_id, Vec::new());
                let mut delay_info = DelayInfo::new();
                delay_info.push_transition(0, snapshot, snapshot);
                internal.insert(*actual_id, delay_info);
            }
            PowerNodeKind::Internal => {
                let gate_delay = info_table
                    .get(actual_id)
                    .ok_or(PowerSimError::MissingNodeInfo { node: *actual_id })?
                    .delay;

                let mut min = i32::MAX;
                let mut max = i32::MIN;
                for fanin in &actual.fanins {
                    let delay_info =
                        internal
                            .get(fanin)
                            .ok_or(PowerSimError::MissingFaninDelayInfo {
                                node: *actual_id,
                                fanin: *fanin,
                            })?;
                    min = min.min(delay_info.first_time());
                    max = max.max(delay_info.last_time());
                }

                max_num_transition = max_num_transition.max(max + gate_delay);

                let frame_count = (max - min + 1) as usize;
                let mut switched = vec![vec![false; actual.fanins.len()]; frame_count];
                for (fanin_index, fanin) in actual.fanins.iter().enumerate() {
                    let delay_info =
                        internal
                            .get(fanin)
                            .ok_or(PowerSimError::MissingFaninDelayInfo {
                                node: *actual_id,
                                fanin: *fanin,
                            })?;
                    for frame in &delay_info.switching_times {
                        switched[(frame - min) as usize][fanin_index] = true;
                    }
                }

                let mut old_to_gate = Vec::with_capacity(actual.fanins.len());
                let mut new_to_gate = Vec::with_capacity(actual.fanins.len());
                for fanin in &actual.fanins {
                    let delay_info =
                        internal
                            .get(fanin)
                            .ok_or(PowerSimError::MissingFaninDelayInfo {
                                node: *actual_id,
                                fanin: *fanin,
                            })?;
                    let old = delay_info.before_switching[0];
                    old_to_gate.push(old);
                    if delay_info.first_time() == min {
                        new_to_gate.push(delay_info.after_switching[0]);
                    } else {
                        new_to_gate.push(old);
                    }
                }

                let mut delay_info = DelayInfo::new();
                for frame_index in 0..frame_count {
                    if switched[frame_index].iter().any(|value| *value) {
                        let before_fanins = patched_fanins(actual, &old_to_gate);
                        let after_fanins = patched_fanins(actual, &new_to_gate);
                        let before = symbolic.add_snapshot(
                            format!("{}_1_{}", actual.name, frame_index),
                            *actual_id,
                            before_fanins,
                        );
                        let after = symbolic.add_snapshot(
                            format!("{}_2_{}", actual.name, frame_index),
                            *actual_id,
                            after_fanins,
                        );
                        let transition_time = frame_index as i32 + min + gate_delay;
                        delay_info.push_transition(transition_time, before, after);
                        symbolic.add_xor(
                            format!("{}_xor_{}", actual.name, transition_time),
                            *actual_id,
                            before,
                            after,
                        );

                        old_to_gate.clone_from(&new_to_gate);
                    }

                    let next = frame_index + 1;
                    if next < frame_count {
                        update_new_to_gate(
                            network,
                            &internal,
                            actual,
                            *actual_id,
                            &switched[next],
                            min + next as i32,
                            &mut new_to_gate,
                        )?;
                    }
                }

                internal.insert(*actual_id, delay_info);
            }
        }
    }

    symbolic.max_normalized_delay = if max_num_transition == i32::MIN {
        0
    } else {
        max_num_transition
    };
    Ok(symbolic)
}

pub fn power_symbolic_simulate_from_sis_network<Network, InfoTable>(
    _network: &Network,
    _info_table: &InfoTable,
) -> Result<SymbolicPowerNetwork, PowerSimError> {
    Err(PowerSimError::MissingNativePorts {
        operation: "power_symbolic_simulate",
    })
}

fn patched_fanins(actual: &PowerNode, symbols: &[SymbolicNodeId]) -> Vec<SymbolicFaninPatch> {
    actual
        .fanins
        .iter()
        .copied()
        .zip(symbols.iter().copied())
        .map(|(original_fanin, symbolic_fanin)| SymbolicFaninPatch {
            original_fanin,
            symbolic_fanin,
        })
        .collect()
}

fn update_new_to_gate(
    network: &PowerNetwork,
    internal: &HashMap<NodeId, DelayInfo>,
    actual: &PowerNode,
    actual_id: NodeId,
    next_switched: &[bool],
    frame: i32,
    new_to_gate: &mut [SymbolicNodeId],
) -> Result<(), PowerSimError> {
    for (fanin_index, did_switch) in next_switched.iter().copied().enumerate() {
        if !did_switch {
            continue;
        }

        let fanin = actual.fanins[fanin_index];
        network.node(fanin)?;
        let delay_info = internal
            .get(&fanin)
            .ok_or(PowerSimError::MissingFaninDelayInfo {
                node: actual_id,
                fanin,
            })?;
        if let Some(symbolic_fanin) = delay_info.after_at_time(frame) {
            new_to_gate[fanin_index] = symbolic_fanin;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(network: &SymbolicPowerNetwork) -> Vec<&str> {
        network
            .nodes()
            .iter()
            .map(|node| node.name.as_str())
            .collect()
    }

    #[test]
    fn primary_input_creates_before_after_and_xor_output() {
        let mut network = PowerNetwork::new("toy");
        network.add_node(PowerNode::new("a", PowerNodeKind::PrimaryInput));

        let symbolic = power_symbolic_simulate(&network, &HashMap::new()).unwrap();

        assert_eq!(symbolic.name(), "toy_symbolic");
        assert_eq!(names(&symbolic), vec!["a_000", "a_ttt", "a_xor_0"]);
        assert_eq!(symbolic.primary_outputs(), &[SymbolicNodeId(2)]);
        assert_eq!(symbolic.max_normalized_delay(), 0);
        assert_eq!(
            symbolic.node(SymbolicNodeId(2)).unwrap().original,
            NodeId(0)
        );
    }

    #[test]
    fn internal_gate_with_simultaneous_fanin_switches_gets_one_delayed_xor() {
        let mut network = PowerNetwork::new("comb");
        let a = network.add_node(PowerNode::new("a", PowerNodeKind::PrimaryInput));
        let b = network.add_node(PowerNode::new("b", PowerNodeKind::PrimaryInput));
        let n =
            network.add_node(PowerNode::new("n", PowerNodeKind::Internal).with_fanins(vec![a, b]));
        let info = HashMap::from([(n, NodeInfo { delay: 2 })]);

        let symbolic = power_symbolic_simulate(&network, &info).unwrap();

        assert!(names(&symbolic).contains(&"n_1_0"));
        assert!(names(&symbolic).contains(&"n_2_0"));
        assert!(names(&symbolic).contains(&"n_xor_2"));
        assert_eq!(symbolic.max_normalized_delay(), 2);

        let before = symbolic
            .nodes()
            .iter()
            .find(|node| node.name == "n_1_0")
            .unwrap();
        assert_eq!(
            before.fanins,
            vec![
                SymbolicFaninPatch {
                    original_fanin: a,
                    symbolic_fanin: SymbolicNodeId(0),
                },
                SymbolicFaninPatch {
                    original_fanin: b,
                    symbolic_fanin: SymbolicNodeId(3),
                },
            ]
        );
    }

    #[test]
    fn staggered_fanin_switches_create_multiple_transition_frames() {
        let mut network = PowerNetwork::new("staggered");
        let a = network.add_node(PowerNode::new("a", PowerNodeKind::PrimaryInput));
        let b = network.add_node(PowerNode::new("b", PowerNodeKind::PrimaryInput));
        let x = network.add_node(PowerNode::new("x", PowerNodeKind::Internal).with_fanins(vec![a]));
        let y = network.add_node(PowerNode::new("y", PowerNodeKind::Internal).with_fanins(vec![b]));
        let n =
            network.add_node(PowerNode::new("n", PowerNodeKind::Internal).with_fanins(vec![x, y]));
        let info = HashMap::from([
            (x, NodeInfo { delay: 2 }),
            (y, NodeInfo { delay: 4 }),
            (n, NodeInfo { delay: 1 }),
        ]);

        let symbolic = power_symbolic_simulate(&network, &info).unwrap();
        let symbolic_names = names(&symbolic);

        assert!(symbolic_names.contains(&"x_xor_2"));
        assert!(symbolic_names.contains(&"y_xor_4"));
        assert!(symbolic_names.contains(&"n_xor_3"));
        assert!(symbolic_names.contains(&"n_xor_5"));
        assert_eq!(symbolic.max_normalized_delay(), 5);
    }

    #[test]
    fn primary_outputs_are_skipped_like_c_dummy_nodes() {
        let mut network = PowerNetwork::new("with_po");
        let a = network.add_node(PowerNode::new("a", PowerNodeKind::PrimaryInput));
        network.add_node(PowerNode::new("out", PowerNodeKind::PrimaryOutput).with_fanins(vec![a]));

        let symbolic = power_symbolic_simulate(&network, &HashMap::new()).unwrap();

        assert_eq!(names(&symbolic), vec!["a_000", "a_ttt", "a_xor_0"]);
    }

    #[test]
    fn sis_bound_entry_reports_explicit_missing_dependencies() {
        let error = power_symbolic_simulate_from_sis_network(&(), &()).unwrap_err();

        assert_eq!(
            error,
            PowerSimError::MissingNativePorts {
                operation: "power_symbolic_simulate",
            }
        );
        assert!(
            error
                .to_string()
                .contains("requires native SIS prerequisite ports")
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("power_sim.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
