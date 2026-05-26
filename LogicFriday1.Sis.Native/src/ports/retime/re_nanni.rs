//! Native Rust model for `LogicSynthesis/sis/retime/re_nanni.c`.
//!
//! The C unit implements Nanni/Saxe feasibility retiming over an existing
//! `re_graph`. This port keeps the behavior in owned Rust data structures and
//! exposes explicit dependency errors for SIS graph integration that still
//! belongs to adjacent native ports. It intentionally does not expose legacy C ABI
//! entry points.

use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const RETIME_TEST_NOT_SET: f64 = -50_000.0;

pub fn sis_re_nanni_integration_blocked() -> Result<(), NanniError> {
    Err(NanniError::MissingSisDependencies {
        operation: "retime_nanni_routine SIS graph integration",
    })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EdgeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeType {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNode {
    pub id: NodeId,
    pub node_type: RetimeNodeType,
    pub fanins: Vec<EdgeId>,
    pub fanouts: Vec<EdgeId>,
    pub final_delay: f64,
    pub user_time: f64,
}

impl RetimeNode {
    pub fn new(node_type: RetimeNodeType, final_delay: f64) -> Self {
        Self {
            id: NodeId(usize::MAX),
            node_type,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            final_delay,
            user_time: RETIME_TEST_NOT_SET,
        }
    }

    pub fn with_user_time(mut self, user_time: f64) -> Self {
        self.user_time = user_time;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeEdge {
    pub id: EdgeId,
    pub source: NodeId,
    pub sink: NodeId,
    pub weight: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RetimeGraph {
    pub nodes: Vec<RetimeNode>,
    pub edges: Vec<RetimeEdge>,
    pub primary_inputs: Vec<NodeId>,
    pub primary_outputs: Vec<NodeId>,
}

impl RetimeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, mut node: RetimeNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.id = id;
        match node.node_type {
            RetimeNodeType::PrimaryInput => self.primary_inputs.push(id),
            RetimeNodeType::PrimaryOutput => self.primary_outputs.push(id),
            RetimeNodeType::Internal | RetimeNodeType::Ignore => {}
        }
        self.nodes.push(node);
        id
    }

    pub fn add_edge(
        &mut self,
        source: NodeId,
        sink: NodeId,
        weight: i32,
    ) -> Result<EdgeId, NanniError> {
        self.require_node(source)?;
        self.require_node(sink)?;
        let id = EdgeId(self.edges.len());
        self.edges.push(RetimeEdge {
            id,
            source,
            sink,
            weight,
        });
        self.nodes[source.0].fanouts.push(id);
        self.nodes[sink.0].fanins.push(id);
        Ok(id)
    }

    pub fn retime_single_node(&mut self, node: NodeId, lag: i32) -> Result<(), NanniError> {
        self.require_node(node)?;
        if lag == 0 {
            return Ok(());
        }

        let fanouts = self.nodes[node.0].fanouts.clone();
        for edge_id in fanouts {
            if !self.ignore_edge(edge_id)? {
                self.edges[edge_id.0].weight -= lag;
            }
        }

        let fanins = self.nodes[node.0].fanins.clone();
        for edge_id in fanins {
            if !self.ignore_edge(edge_id)? {
                self.edges[edge_id.0].weight += lag;
            }
        }

        Ok(())
    }

    fn delay_table(&self) -> Result<Vec<f64>, NanniError> {
        let mut valid = vec![false; self.nodes.len()];
        let mut delays = vec![0.0; self.nodes.len()];

        for node in &self.nodes {
            if node.fanins.is_empty() {
                valid[node.id.0] = true;
                if node.user_time > RETIME_TEST_NOT_SET {
                    delays[node.id.0] = node.user_time;
                }
            }
        }

        for index in (0..self.nodes.len()).rev() {
            if !valid[index] {
                self.evaluate_delay(NodeId(index), &mut valid, &mut delays, &mut Vec::new())?;
            }
        }

        Ok(delays)
    }

    fn evaluate_delay(
        &self,
        node: NodeId,
        valid: &mut [bool],
        delays: &mut [f64],
        active: &mut Vec<NodeId>,
    ) -> Result<(), NanniError> {
        self.require_node(node)?;
        if valid[node.0] {
            return Ok(());
        }
        if active.contains(&node) {
            return Err(NanniError::ZeroWeightDelayCycle { node });
        }

        active.push(node);
        for edge_id in &self.nodes[node.0].fanins {
            if self.ignore_edge(*edge_id)? {
                continue;
            }
            let edge = &self.edges[edge_id.0];
            if edge.weight == 0 && !valid[edge.source.0] {
                self.evaluate_delay(edge.source, valid, delays, active)?;
            }
        }

        let mut max_fanin_delay = 0.0;
        for edge_id in &self.nodes[node.0].fanins {
            if self.ignore_edge(*edge_id)? {
                continue;
            }
            let edge = &self.edges[edge_id.0];
            if edge.weight == 0 {
                max_fanin_delay = f64::max(max_fanin_delay, delays[edge.source.0]);
            }
        }

        active.pop();
        valid[node.0] = true;
        delays[node.0] = max_fanin_delay + self.nodes[node.0].final_delay;
        Ok(())
    }

    fn slow_nodes(&self, clk: f64) -> Result<Vec<NodeId>, NanniError> {
        let delays = self.delay_table()?;
        let mut slow = Vec::new();
        for index in (0..self.nodes.len()).rev() {
            let node = &self.nodes[index];
            let offset = if node.node_type == RetimeNodeType::PrimaryOutput
                && node.user_time > RETIME_TEST_NOT_SET
            {
                node.user_time
            } else {
                0.0
            };

            if delays[index] - offset > clk {
                slow.push(node.id);
            }
        }
        Ok(slow)
    }

    fn input_reachable(&self) -> Result<Vec<NodeId>, NanniError> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        let mut queue = VecDeque::new();

        for input in &self.primary_inputs {
            self.require_node(*input)?;
            if seen.insert(*input) {
                result.push(*input);
                queue.push_back(*input);
            }
        }

        while let Some(node) = queue.pop_front() {
            for edge_id in &self.nodes[node.0].fanouts {
                if self.ignore_edge(*edge_id)? {
                    continue;
                }
                let edge = &self.edges[edge_id.0];
                if edge.weight == 0 && seen.insert(edge.sink) {
                    result.push(edge.sink);
                    queue.push_back(edge.sink);
                }
            }
        }

        Ok(result)
    }

    fn output_table_from_nodes(&self, nodes: &[NodeId]) -> HashSet<NodeId> {
        nodes
            .iter()
            .copied()
            .filter(|node| self.nodes[node.0].node_type == RetimeNodeType::PrimaryOutput)
            .collect()
    }

    fn set_outputs(&mut self, slow: &[NodeId], retiming: &mut [i32]) -> Result<bool, NanniError> {
        let slow_outputs = self.output_table_from_nodes(slow);
        if slow_outputs.is_empty() {
            return Ok(true);
        }

        let primary_outputs = self.primary_outputs.clone();
        for output in primary_outputs {
            if !slow_outputs.contains(&output) {
                self.retime_single_node(output, 1)?;
                retiming[output.0] += 1;
            }
        }

        let reachable = self.input_reachable()?;
        for node in reachable {
            if self.nodes[node.0].node_type == RetimeNodeType::PrimaryOutput
                && slow_outputs.contains(&node)
            {
                return Ok(false);
            }
            self.retime_single_node(node, 1)?;
            retiming[node.0] += 1;
        }

        Ok(true)
    }

    fn translate_retiming_vector(&mut self, retiming: &mut [i32]) -> Result<(), NanniError> {
        let max_pi = self
            .primary_inputs
            .iter()
            .map(|node| retiming[node.0])
            .max()
            .unwrap_or(0);
        let max_po = self
            .primary_outputs
            .iter()
            .map(|node| retiming[node.0])
            .max()
            .unwrap_or(0);

        if max_pi != max_po {
            return Err(NanniError::UnbalancedIoRetiming { max_pi, max_po });
        }

        for index in (0..self.nodes.len()).rev() {
            retiming[index] -= max_pi;
            self.retime_single_node(NodeId(index), -max_pi)?;
        }

        Ok(())
    }

    fn ignore_edge(&self, edge: EdgeId) -> Result<bool, NanniError> {
        let edge = self
            .edges
            .get(edge.0)
            .ok_or(NanniError::MissingEdge(edge))?;
        Ok(
            self.nodes[edge.source.0].node_type == RetimeNodeType::Ignore
                || self.nodes[edge.sink.0].node_type == RetimeNodeType::Ignore,
        )
    }

    fn require_node(&self, node: NodeId) -> Result<(), NanniError> {
        if node.0 < self.nodes.len() {
            Ok(())
        } else {
            Err(NanniError::MissingNode(node))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NanniError {
    MissingSisDependencies { operation: &'static str },
    MissingNode(NodeId),
    MissingEdge(EdgeId),
    ZeroWeightDelayCycle { node: NodeId },
    UnbalancedIoRetiming { max_pi: i32, max_po: i32 },
}

impl fmt::Display for NanniError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => {
                write!(f, "{operation} requires native prerequisite ports")
            }
            Self::MissingNode(node) => write!(f, "retime graph references missing node {}", node.0),
            Self::MissingEdge(edge) => write!(f, "retime graph references missing edge {}", edge.0),
            Self::ZeroWeightDelayCycle { node } => {
                write!(f, "zero-weight delay cycle reaches node {}", node.0)
            }
            Self::UnbalancedIoRetiming { max_pi, max_po } => write!(
                f,
                "translated retiming vector expected equal max PI/PO retiming, got {max_pi}/{max_po}"
            ),
        }
    }
}

impl Error for NanniError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NanniResult {
    pub feasible: bool,
    pub retiming: Vec<i32>,
}

pub fn retime_nanni_routine(
    graph: &mut RetimeGraph,
    d_clk: f64,
) -> Result<NanniResult, NanniError> {
    let n = graph.nodes.len();
    let mut retiming = vec![0; n];

    for _ in (0..n).rev() {
        let slow = graph.slow_nodes(d_clk)?;
        if slow.is_empty() {
            graph.translate_retiming_vector(&mut retiming)?;
            return Ok(NanniResult {
                feasible: true,
                retiming,
            });
        }

        for node in &slow {
            graph.retime_single_node(*node, 1)?;
            retiming[node.0] += 1;
        }

        if !graph.set_outputs(&slow, &mut retiming)? {
            return Ok(NanniResult {
                feasible: false,
                retiming,
            });
        }
    }

    Ok(NanniResult {
        feasible: false,
        retiming,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain_graph() -> RetimeGraph {
        let mut graph = RetimeGraph::new();
        let pi = graph.add_node(RetimeNode::new(RetimeNodeType::PrimaryInput, 0.0));
        let a = graph.add_node(RetimeNode::new(RetimeNodeType::Internal, 2.0));
        let b = graph.add_node(RetimeNode::new(RetimeNodeType::Internal, 3.0));
        let po = graph.add_node(RetimeNode::new(RetimeNodeType::PrimaryOutput, 0.0));
        graph.add_edge(pi, a, 0).unwrap();
        graph.add_edge(a, b, 0).unwrap();
        graph.add_edge(b, po, 0).unwrap();
        graph
    }

    #[test]
    fn delay_table_matches_zero_weight_arrival_recursion() {
        let graph = chain_graph();
        assert_eq!(graph.delay_table().unwrap(), vec![0.0, 2.0, 5.0, 5.0]);
        assert_eq!(graph.slow_nodes(4.0).unwrap(), vec![NodeId(3), NodeId(2)]);
    }

    #[test]
    fn primary_output_user_time_offsets_slow_node_test() {
        let mut graph = chain_graph();
        graph.nodes[3].user_time = 2.0;
        assert_eq!(graph.slow_nodes(4.0).unwrap(), vec![NodeId(2)]);
    }

    #[test]
    fn input_reachable_walks_only_zero_weight_fanouts() {
        let mut graph = chain_graph();
        graph.edges[1].weight = 1;
        assert_eq!(graph.input_reachable().unwrap(), vec![NodeId(0), NodeId(1)]);
    }

    #[test]
    fn set_outputs_ports_c_conflict_case() {
        let mut graph = chain_graph();
        let mut retiming = vec![0; graph.nodes.len()];
        let feasible = graph.set_outputs(&[NodeId(3)], &mut retiming).unwrap();
        assert!(!feasible);
        assert_eq!(retiming, vec![1, 1, 1, 0]);
    }

    #[test]
    fn nanni_routine_succeeds_without_retiming_when_clock_is_feasible() {
        let mut graph = chain_graph();
        let result = retime_nanni_routine(&mut graph, 5.0).unwrap();
        assert!(result.feasible);
        assert_eq!(result.retiming, vec![0, 0, 0, 0]);
        assert_eq!(
            graph
                .edges
                .iter()
                .map(|edge| edge.weight)
                .collect::<Vec<_>>(),
            vec![0, 0, 0]
        );
    }

    #[test]
    fn nanni_routine_reports_infeasible_after_bounded_iterations() {
        let mut graph = chain_graph();
        let result = retime_nanni_routine(&mut graph, 1.0).unwrap();
        assert!(!result.feasible);
        assert_eq!(result.retiming.len(), graph.nodes.len());
    }
}
