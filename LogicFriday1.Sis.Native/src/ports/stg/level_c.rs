//! Native Rust model for `LogicSynthesis/sis/stg/level_c.c`.
//!
//! The original C code levelizes the global `copy` network for STG
//! enumeration, stores per-node `ndata`, records latch present/next-state
//! nodes, initializes constant primary outputs, and then reorders gate fanins
//! by level while keeping the packed cube literal bits aligned.
//!
//! This module ports the independent data and algorithmic behavior into a
//! small Rust graph model. Binding directly to SIS `network_t`, `node_t`, and
//! `latch_t` remains blocked until those native ports exist, so the legacy
//! entry points are represented as explicit errors instead of C ABI exports.

use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const MAX_ELENGTH: usize = 36;
pub const MARKED: u8 = 4;
pub const BARRAY_LEN: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LevelPortDisposition {
    BlockedByUnportedNetworkNodeAndLatchApis,
}

pub fn level_port_disposition() -> LevelPortDisposition {
    LevelPortDisposition::BlockedByUnportedNetworkNodeAndLatchApis
}

pub fn level_port_is_blocked() -> bool {
    level_port_disposition() == LevelPortDisposition::BlockedByUnportedNetworkNodeAndLatchApis
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LevelDependency {
    NetworkPort,
    NodePort,
    LatchPort,
    SenumMainPort,
    EnumeratePort,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LevelError {
    MissingDependency(LevelDependency),
    UnknownNode { node: usize },
    UnknownFanin { node: usize, fanin: usize },
    NonInputLatchEndpoint { node: usize, latch_end: usize },
    UnlevelizableCircuit { stalled_level: usize },
    TooManyFaninsForBarray { node: usize, fanins: usize },
}

impl LevelError {
    pub const fn missing(dependency: LevelDependency) -> Self {
        Self::MissingDependency(dependency)
    }
}

impl fmt::Display for LevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDependency(LevelDependency::NetworkPort) => {
                write!(f, "SIS network APIs are not ported to Rust yet")
            }
            Self::MissingDependency(LevelDependency::NodePort) => {
                write!(f, "SIS node APIs are not ported to Rust yet")
            }
            Self::MissingDependency(LevelDependency::LatchPort) => {
                write!(f, "SIS latch APIs are not ported to Rust yet")
            }
            Self::MissingDependency(LevelDependency::SenumMainPort) => {
                write!(f, "SIS STG enumeration globals are not ported to Rust yet")
            }
            Self::MissingDependency(LevelDependency::EnumeratePort) => {
                write!(f, "SIS STG enumerate helpers are not ported to Rust yet")
            }
            Self::UnknownNode { node } => write!(f, "unknown node id {node}"),
            Self::UnknownFanin { node, fanin } => {
                write!(f, "node {node} has unknown fanin {fanin}")
            }
            Self::NonInputLatchEndpoint { node, latch_end } => write!(
                f,
                "primary input {node} has latch endpoint {latch_end} without a next-state fanin"
            ),
            Self::UnlevelizableCircuit { stalled_level } => write!(
                f,
                "circuit could not be levelized after level {stalled_level}"
            ),
            Self::TooManyFaninsForBarray { node, fanins } => write!(
                f,
                "node {node} has {fanins} fanins, exceeding level_c.c barray capacity"
            ),
        }
    }
}

impl Error for LevelError {}

pub fn level_circuit() -> Result<(), LevelError> {
    Err(LevelError::missing(LevelDependency::NetworkPort))
}

pub fn rearrange_gate_inputs() -> Result<(), LevelError> {
    Err(LevelError::missing(LevelDependency::NodePort))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    pub const fn is_one(self) -> bool {
        matches!(self, Self::One)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LevelNode {
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub fanins: Vec<usize>,
    pub fanouts: Vec<usize>,
    pub cube_literals: Option<Vec<Literal>>,
    pub latch_end: Option<LatchEnd>,
}

impl LevelNode {
    pub fn primary_input() -> Self {
        Self {
            kind: NodeKind::PrimaryInput,
            function: NodeFunction::Other,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            cube_literals: None,
            latch_end: None,
        }
    }

    pub fn primary_output(fanin: usize) -> Self {
        Self {
            kind: NodeKind::PrimaryOutput,
            function: NodeFunction::Other,
            fanins: vec![fanin],
            fanouts: Vec::new(),
            cube_literals: None,
            latch_end: None,
        }
    }

    pub fn constant(function: NodeFunction) -> Self {
        Self {
            kind: NodeKind::Internal,
            function,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            cube_literals: None,
            latch_end: None,
        }
    }

    pub fn internal(fanins: Vec<usize>, cube_literals: Vec<Literal>) -> Self {
        Self {
            kind: NodeKind::Internal,
            function: NodeFunction::Other,
            fanins,
            fanouts: Vec::new(),
            cube_literals: Some(cube_literals),
            latch_end: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LatchEnd {
    pub endpoint_node: usize,
    pub initial_value: i32,
    pub next_state_node: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeData {
    pub cube: u64,
    pub value: [u8; MAX_ELENGTH],
    pub jflag: [u8; MAX_ELENGTH],
    pub level: usize,
}

impl NodeData {
    pub fn new(cube: u64) -> Self {
        Self {
            cube,
            value: [0; MAX_ELENGTH],
            jflag: [0; MAX_ELENGTH],
            level: 0,
        }
    }

    pub fn mark(&mut self) {
        self.jflag[0] |= MARKED;
    }

    pub fn unmark(&mut self) {
        self.jflag[0] = 0;
    }

    pub fn is_marked(&self) -> bool {
        self.jflag[0] & MARKED != 0
    }

    pub fn set_constant_value(&mut self, value: bool) {
        self.value.fill(u8::from(value));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LevelCircuit {
    pub nodes: Vec<LevelNode>,
    pub primary_inputs: Vec<usize>,
    pub primary_outputs: Vec<usize>,
}

impl LevelCircuit {
    pub fn new(
        nodes: Vec<LevelNode>,
        primary_inputs: Vec<usize>,
        primary_outputs: Vec<usize>,
    ) -> Self {
        Self {
            nodes,
            primary_inputs,
            primary_outputs,
        }
    }

    pub fn with_derived_fanouts(mut self) -> Result<Self, LevelError> {
        for node in &mut self.nodes {
            node.fanouts.clear();
        }

        for node_id in 0..self.nodes.len() {
            let fanins = self.nodes[node_id].fanins.clone();
            for fanin in fanins {
                let Some(source) = self.nodes.get_mut(fanin) else {
                    return Err(LevelError::UnknownFanin {
                        node: node_id,
                        fanin,
                    });
                };
                source.fanouts.push(node_id);
            }
        }

        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Levelization {
    pub data: Vec<NodeData>,
    pub present_state: Vec<usize>,
    pub initial_state: Vec<i32>,
    pub next_state: Vec<usize>,
    pub real_primary_outputs: Vec<usize>,
    pub varying_nodes: Vec<usize>,
}

pub fn packed_cube_from_literals(literals: &[Literal]) -> u64 {
    let mut packed = 0;
    for literal in literals.iter().rev() {
        packed <<= 1;
        packed += u64::from(literal.is_one());
    }
    packed
}

pub fn initial_cube_for_node(node: &LevelNode) -> u64 {
    node.cube_literals
        .as_deref()
        .filter(|literals| !literals.is_empty())
        .map(packed_cube_from_literals)
        .unwrap_or(1)
}

pub fn levelize_circuit(circuit: &LevelCircuit) -> Result<Levelization, LevelError> {
    validate_node_refs(circuit)?;

    let mut data: Vec<NodeData> = circuit
        .nodes
        .iter()
        .map(|node| NodeData::new(initial_cube_for_node(node)))
        .collect();

    let mut present_state = Vec::new();
    let mut initial_state = Vec::new();
    let mut next_state = Vec::new();
    let mut varying_nodes = Vec::new();
    let mut real_primary_outputs = Vec::new();
    let mut queue = VecDeque::new();
    let mut queued = HashSet::new();

    for &node_id in &circuit.primary_inputs {
        let node = circuit
            .nodes
            .get(node_id)
            .ok_or(LevelError::UnknownNode { node: node_id })?;

        if let Some(latch_end) = node.latch_end {
            if !circuit
                .nodes
                .get(latch_end.endpoint_node)
                .is_some_and(|endpoint| endpoint.fanins.contains(&latch_end.next_state_node))
            {
                return Err(LevelError::NonInputLatchEndpoint {
                    node: node_id,
                    latch_end: latch_end.endpoint_node,
                });
            }
            present_state.push(node_id);
            initial_state.push(latch_end.initial_value);
            next_state.push(latch_end.next_state_node);
        } else {
            varying_nodes.push(node_id);
        }

        data[node_id].level = 0;
        data[node_id].mark();
        push_unqueued_fanouts(circuit, node_id, &mut queue, &mut queued);
    }

    for &node_id in &circuit.primary_outputs {
        let node = circuit
            .nodes
            .get(node_id)
            .ok_or(LevelError::UnknownNode { node: node_id })?;
        let Some(&fanin) = node.fanins.first() else {
            continue;
        };

        if let Some(source) = circuit.nodes.get(fanin) {
            if source.kind == NodeKind::Internal && source.fanins.is_empty() {
                data[fanin].level = 0;
                data[fanin].mark();
                data[fanin].set_constant_value(source.function == NodeFunction::One);
            }
        }

        if node.latch_end.is_none() {
            real_primary_outputs.push(fanin);
        }
    }

    let mut level = 0;
    while !queue.is_empty() {
        let level_count = queue.len();
        let mut scheduled = Vec::new();
        let mut deferred = Vec::new();

        for _ in 0..level_count {
            let current = queue.pop_front().expect("level queue length was captured");
            queued.remove(&current);

            if circuit.nodes[current]
                .fanins
                .iter()
                .all(|&fanin| data[fanin].is_marked())
            {
                scheduled.push(current);
            } else {
                deferred.push(current);
            }
        }

        if scheduled.is_empty() {
            return Err(LevelError::UnlevelizableCircuit {
                stalled_level: level + 1,
            });
        }

        level += 1;

        for node_id in scheduled {
            if circuit.nodes[node_id].kind == NodeKind::Internal {
                varying_nodes.push(node_id);
            }
            data[node_id].level = level;
            data[node_id].mark();
            push_unqueued_fanouts(circuit, node_id, &mut queue, &mut queued);
        }

        for node_id in deferred {
            if queued.insert(node_id) {
                queue.push_back(node_id);
            }
        }
    }

    for node in &mut data {
        node.unmark();
    }

    Ok(Levelization {
        data,
        present_state,
        initial_state,
        next_state,
        real_primary_outputs,
        varying_nodes,
    })
}

pub fn rearrange_gate_inputs_in_model(
    circuit: &mut LevelCircuit,
    data: &mut [NodeData],
) -> Result<(), LevelError> {
    validate_node_refs(circuit)?;

    for node_id in 0..circuit.nodes.len() {
        if circuit.nodes[node_id].kind != NodeKind::Internal {
            continue;
        }

        let fanin_count = circuit.nodes[node_id].fanins.len();
        if fanin_count <= 1 {
            continue;
        }
        if fanin_count > BARRAY_LEN {
            return Err(LevelError::TooManyFaninsForBarray {
                node: node_id,
                fanins: fanin_count,
            });
        }

        for j in 0..fanin_count - 1 {
            for k in j + 1..fanin_count {
                let left = circuit.nodes[node_id].fanins[j];
                let right = circuit.nodes[node_id].fanins[k];
                if data[left].level > data[right].level {
                    circuit.nodes[node_id].fanins.swap(j, k);
                    swap_cube_bits(&mut data[node_id].cube, j, k);
                }
            }
        }
    }

    Ok(())
}

pub fn swap_cube_bits(cube: &mut u64, left: usize, right: usize) {
    if left == right {
        return;
    }

    let left_mask = 1_u64 << left;
    let right_mask = 1_u64 << right;
    let left_set = *cube & left_mask != 0;
    let right_set = *cube & right_mask != 0;

    if right_set {
        *cube |= left_mask;
    } else {
        *cube &= !left_mask;
    }

    if left_set {
        *cube |= right_mask;
    } else {
        *cube &= !right_mask;
    }
}

fn validate_node_refs(circuit: &LevelCircuit) -> Result<(), LevelError> {
    for &node in circuit
        .primary_inputs
        .iter()
        .chain(circuit.primary_outputs.iter())
    {
        if node >= circuit.nodes.len() {
            return Err(LevelError::UnknownNode { node });
        }
    }

    for (node_id, node) in circuit.nodes.iter().enumerate() {
        for &fanin in &node.fanins {
            if fanin >= circuit.nodes.len() {
                return Err(LevelError::UnknownFanin {
                    node: node_id,
                    fanin,
                });
            }
        }
        for &fanout in &node.fanouts {
            if fanout >= circuit.nodes.len() {
                return Err(LevelError::UnknownNode { node: fanout });
            }
        }
    }

    Ok(())
}

fn push_unqueued_fanouts(
    circuit: &LevelCircuit,
    node_id: usize,
    queue: &mut VecDeque<usize>,
    queued: &mut HashSet<usize>,
) {
    for &fanout in &circuit.nodes[node_id].fanouts {
        if queued.insert(fanout) {
            queue.push_back(fanout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_cube_literals_with_fanin_zero_in_low_bit() {
        let packed = packed_cube_from_literals(&[
            Literal::One,
            Literal::Zero,
            Literal::One,
            Literal::DontCare,
        ]);

        assert_eq!(packed, 0b0101);
        assert_eq!(initial_cube_for_node(&LevelNode::primary_input()), 1);
    }

    #[test]
    fn initializes_constant_primary_output_values() {
        let nodes = vec![
            LevelNode::constant(NodeFunction::One),
            LevelNode::primary_output(0),
        ];
        let circuit = LevelCircuit::new(nodes, vec![], vec![1])
            .with_derived_fanouts()
            .unwrap();

        let result = levelize_circuit(&circuit).unwrap();

        assert_eq!(result.data[0].level, 0);
        assert!(result.data[0].value.iter().all(|&value| value == 1));
        assert_eq!(result.real_primary_outputs, vec![0]);
    }

    #[test]
    fn levelizes_inputs_internal_nodes_and_state_edges() {
        let mut pi = LevelNode::primary_input();
        pi.latch_end = Some(LatchEnd {
            endpoint_node: 3,
            initial_value: 1,
            next_state_node: 2,
        });

        let nodes = vec![
            pi,
            LevelNode::primary_input(),
            LevelNode::internal(vec![0, 1], vec![Literal::One, Literal::Zero]),
            LevelNode::primary_output(2),
        ];
        let circuit = LevelCircuit::new(nodes, vec![0, 1], vec![3])
            .with_derived_fanouts()
            .unwrap();

        let result = levelize_circuit(&circuit).unwrap();

        assert_eq!(result.present_state, vec![0]);
        assert_eq!(result.initial_state, vec![1]);
        assert_eq!(result.next_state, vec![2]);
        assert_eq!(result.varying_nodes, vec![1, 2]);
        assert_eq!(result.real_primary_outputs, vec![2]);
        assert_eq!(result.data[2].level, 1);
        assert!(result.data.iter().all(|node| !node.is_marked()));
    }

    #[test]
    fn reports_unlevelizable_cycles() {
        let nodes = vec![
            LevelNode {
                fanouts: vec![1],
                ..LevelNode::primary_input()
            },
            LevelNode {
                fanouts: vec![1],
                ..LevelNode::internal(vec![0, 1], vec![Literal::One, Literal::One])
            },
        ];
        let circuit = LevelCircuit::new(nodes, vec![0], vec![]);

        assert_eq!(
            levelize_circuit(&circuit),
            Err(LevelError::UnlevelizableCircuit { stalled_level: 1 })
        );
    }

    #[test]
    fn rearranges_gate_inputs_and_swaps_cube_bits() {
        let mut circuit = LevelCircuit::new(
            vec![
                LevelNode::primary_input(),
                LevelNode::primary_input(),
                LevelNode::primary_input(),
                LevelNode::internal(
                    vec![0, 1, 2],
                    vec![Literal::One, Literal::Zero, Literal::One],
                ),
            ],
            vec![0, 1, 2],
            vec![],
        );
        let mut data = vec![
            NodeData {
                level: 3,
                ..NodeData::new(1)
            },
            NodeData {
                level: 1,
                ..NodeData::new(1)
            },
            NodeData {
                level: 2,
                ..NodeData::new(1)
            },
            NodeData::new(0b101),
        ];

        rearrange_gate_inputs_in_model(&mut circuit, &mut data).unwrap();

        assert_eq!(circuit.nodes[3].fanins, vec![1, 2, 0]);
        assert_eq!(data[3].cube, 0b110);
    }

    #[test]
    fn scaffold_reports_missing_native_prerequisites() {
        assert!(level_port_is_blocked());
        assert_eq!(
            level_circuit(),
            Err(LevelError::MissingDependency(LevelDependency::NetworkPort))
        );
        assert_eq!(
            rearrange_gate_inputs(),
            Err(LevelError::MissingDependency(LevelDependency::NodePort))
        );
    }
}
