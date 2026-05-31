//! Native Rust phase-assignment core for SIS phase optimization.
//!
//! This module ports the phase bookkeeping from `sis/phase/phase.c` into an
//! owned Rust model. The original code stores node rows and fanout columns in a
//! sparse matrix and updates inverter-saving counts incrementally when a node
//! is inverted. This port preserves those row/column semantics without exposing
//! legacy C entry points.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PhaseNodeId(pub usize);

impl PhaseNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Inverter,
    Buffer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgePhase {
    Negative,
    Positive,
    Both,
}

impl EdgePhase {
    pub fn inverted(self) -> Self {
        match self {
            Self::Negative => Self::Positive,
            Self::Positive => Self::Negative,
            Self::Both => Self::Both,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseNode {
    pub name: String,
    pub kind: PhaseNodeKind,
    pub invertible: bool,
    pub inverted: bool,
    pub marked: bool,
    pub reaches_primary_output: bool,
    pub area: f64,
    pub dual_area: f64,
}

impl PhaseNode {
    pub fn new(name: impl Into<String>, kind: PhaseNodeKind) -> Self {
        let invertible = matches!(kind, PhaseNodeKind::Internal);

        Self {
            name: name.into(),
            kind,
            invertible,
            inverted: false,
            marked: false,
            reaches_primary_output: false,
            area: 0.0,
            dual_area: 0.0,
        }
    }

    pub fn with_area(mut self, area: f64, dual_area: f64) -> Self {
        self.area = area;
        self.dual_area = dual_area;
        self
    }

    pub fn with_invertible(mut self, invertible: bool) -> Self {
        self.invertible = invertible;
        self
    }

    pub fn with_primary_output_reach(mut self, reaches_primary_output: bool) -> Self {
        self.reaches_primary_output = reaches_primary_output;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhaseEdge {
    pub source: PhaseNodeId,
    pub target: PhaseNodeId,
    pub phase: EdgePhase,
}

impl PhaseEdge {
    pub fn new(source: PhaseNodeId, target: PhaseNodeId, phase: EdgePhase) -> Self {
        Self {
            source,
            target,
            phase,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseNodeState {
    node: PhaseNode,
    pos_used: i32,
    neg_used: i32,
    inv_save: i32,
}

impl PhaseNodeState {
    pub fn node(&self) -> &PhaseNode {
        &self.node
    }

    pub fn pos_used(&self) -> i32 {
        self.pos_used
    }

    pub fn neg_used(&self) -> i32 {
        self.neg_used
    }

    pub fn inv_save(&self) -> i32 {
        self.inv_save
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseNetwork {
    nodes: Vec<PhaseNodeState>,
    edges: BTreeMap<(PhaseNodeId, PhaseNodeId), EdgePhase>,
    cost: f64,
    inverter_area: f64,
}

impl PhaseNetwork {
    pub fn new(nodes: Vec<PhaseNode>, edges: Vec<PhaseEdge>) -> Result<Self, PhaseError> {
        Self::with_inverter_area(nodes, edges, 1.0)
    }

    pub fn with_inverter_area(
        nodes: Vec<PhaseNode>,
        edges: Vec<PhaseEdge>,
        inverter_area: f64,
    ) -> Result<Self, PhaseError> {
        if !inverter_area.is_finite() || inverter_area < 0.0 {
            return Err(PhaseError::InvalidArea {
                area: inverter_area,
            });
        }

        let mut network = Self {
            nodes: nodes
                .into_iter()
                .map(|node| PhaseNodeState {
                    node,
                    pos_used: 0,
                    neg_used: 0,
                    inv_save: 0,
                })
                .collect(),
            edges: BTreeMap::new(),
            cost: 0.0,
            inverter_area,
        };

        for edge in edges {
            network.ensure_node(edge.source)?;
            network.ensure_node(edge.target)?;
            if network
                .edges
                .insert((edge.source, edge.target), edge.phase)
                .is_some()
            {
                return Err(PhaseError::DuplicateEdge {
                    source: edge.source,
                    target: edge.target,
                });
            }
        }

        network.recompute();
        Ok(network)
    }

    pub fn nodes(&self) -> &[PhaseNodeState] {
        &self.nodes
    }

    pub fn node(&self, node: PhaseNodeId) -> Option<&PhaseNodeState> {
        self.nodes.get(node.index())
    }

    pub fn edge_phase(&self, source: PhaseNodeId, target: PhaseNodeId) -> Option<EdgePhase> {
        self.edges.get(&(source, target)).copied()
    }

    pub fn edges(&self) -> Vec<PhaseEdge> {
        self.edges
            .iter()
            .map(|((source, target), phase)| PhaseEdge::new(*source, *target, *phase))
            .collect()
    }

    pub fn cost(&self) -> f64 {
        self.cost
    }

    pub fn computed_cost(&self) -> f64 {
        self.nodes
            .iter()
            .map(|state| {
                let gate_cost = if state.node.inverted {
                    state.node.dual_area
                } else {
                    state.node.area
                };
                let output_inverter_cost = if state.pos_used != 0 {
                    self.inverter_area
                } else {
                    0.0
                };

                gate_cost + output_inverter_cost
            })
            .sum()
    }

    pub fn phase_value(&self, node: PhaseNodeId) -> Result<f64, PhaseError> {
        let state = self.node(node).ok_or(PhaseError::MissingNode { node })?;
        let area_delta = if state.node.inverted {
            state.node.dual_area - state.node.area
        } else {
            state.node.area - state.node.dual_area
        };

        Ok(area_delta + f64::from(state.inv_save) * self.inverter_area)
    }

    pub fn best_node(&self) -> Option<PhaseNodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, state)| state.node.invertible && !state.node.marked)
            .map(|(index, _)| PhaseNodeId(index))
            .fold(None, |best, node| {
                let value = self.phase_value(node).unwrap_or(f64::NEG_INFINITY);
                match best {
                    Some((best_node, best_value)) if best_value >= value => {
                        Some((best_node, best_value))
                    }
                    _ => Some((node, value)),
                }
            })
            .map(|(node, _)| node)
    }

    pub fn mark(&mut self, node: PhaseNodeId) -> Result<(), PhaseError> {
        self.ensure_node(node)?;
        self.nodes[node.index()].node.marked = true;
        Ok(())
    }

    pub fn unmark_all(&mut self) {
        for state in &mut self.nodes {
            state.node.marked = false;
        }
    }

    pub fn invert(&mut self, node: PhaseNodeId) -> Result<PhaseInvertReport, PhaseError> {
        self.ensure_node(node)?;
        if !self.nodes[node.index()].node.invertible {
            return Err(PhaseError::NodeNotInvertible { node });
        }

        let before = self.cost;
        self.cost -= self.phase_value(node)?;

        self.phase_dec(node)?;
        for fanin in self.fanins(node) {
            self.phase_dec(fanin)?;
        }

        self.do_invert(node)?;

        self.phase_inc(node)?;
        for fanin in self.fanins(node) {
            self.phase_inc(fanin)?;
        }

        self.check_consistency()?;

        Ok(PhaseInvertReport {
            node,
            cost_before: before,
            cost_after: self.cost,
        })
    }

    pub fn random_assign_with<F>(
        &mut self,
        mut next_bool: F,
    ) -> Result<Vec<PhaseNodeId>, PhaseError>
    where
        F: FnMut() -> bool,
    {
        let mut inverted = Vec::new();
        let nodes = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, state)| state.node.invertible.then_some(PhaseNodeId(index)))
            .collect::<Vec<_>>();

        for node in nodes {
            if next_bool() {
                self.invert(node)?;
                inverted.push(node);
            }
        }

        Ok(inverted)
    }

    pub fn check_consistency(&self) -> Result<(), PhaseError> {
        const THRESHOLD: f64 = 0.00001;

        let computed_cost = self.computed_cost();
        if (self.cost - computed_cost).abs() > THRESHOLD {
            return Err(PhaseError::InconsistentCost {
                recorded: self.cost,
                computed: computed_cost,
            });
        }

        for (index, state) in self.nodes.iter().enumerate() {
            let node = PhaseNodeId(index);
            if state.node.invertible && state.node.kind != PhaseNodeKind::Internal {
                return Err(PhaseError::InvalidInvertibleKind {
                    node,
                    kind: state.node.kind,
                });
            }

            let computed = self.compute_inv_save(node)?;
            if state.inv_save != computed {
                return Err(PhaseError::InconsistentSaving {
                    node,
                    recorded: state.inv_save,
                    computed,
                });
            }
        }

        Ok(())
    }

    fn recompute(&mut self) {
        for state in &mut self.nodes {
            state.pos_used = 0;
            state.neg_used = 0;
            state.inv_save = 0;
        }

        for ((source, _), phase) in &self.edges {
            let state = &mut self.nodes[source.index()];
            match phase {
                EdgePhase::Negative => state.neg_used += 1,
                EdgePhase::Positive => state.pos_used += 1,
                EdgePhase::Both => {
                    state.pos_used += 1;
                    state.neg_used += 1;
                }
            }
        }

        for index in 0..self.nodes.len() {
            let node = PhaseNodeId(index);
            self.nodes[index].inv_save = self
                .compute_inv_save(node)
                .expect("recompute uses already validated node identifiers");
        }

        self.cost = self.computed_cost();
    }

    fn compute_inv_save(&self, node: PhaseNodeId) -> Result<i32, PhaseError> {
        self.ensure_node(node)?;

        let mut count = inv_save_output(&self.nodes[node.index()]);
        for fanin in self.fanins(node) {
            let phase = self
                .edge_phase(fanin, node)
                .ok_or(PhaseError::MissingEdge {
                    source: fanin,
                    target: node,
                })?;
            count += inv_save_input(&self.nodes[fanin.index()], phase);
        }

        Ok(count)
    }

    fn phase_dec(&mut self, node: PhaseNodeId) -> Result<(), PhaseError> {
        self.ensure_node(node)?;
        let output_save = inv_save_output(&self.nodes[node.index()]);
        self.nodes[node.index()].inv_save -= output_save;

        for fanout in self.fanouts(node) {
            let phase = self
                .edge_phase(node, fanout)
                .ok_or(PhaseError::MissingEdge {
                    source: node,
                    target: fanout,
                })?;
            let input_save = inv_save_input(&self.nodes[node.index()], phase);
            self.nodes[fanout.index()].inv_save -= input_save;
        }

        Ok(())
    }

    fn phase_inc(&mut self, node: PhaseNodeId) -> Result<(), PhaseError> {
        self.ensure_node(node)?;
        let output_save = inv_save_output(&self.nodes[node.index()]);
        self.nodes[node.index()].inv_save += output_save;

        for fanout in self.fanouts(node) {
            let phase = self
                .edge_phase(node, fanout)
                .ok_or(PhaseError::MissingEdge {
                    source: node,
                    target: fanout,
                })?;
            let input_save = inv_save_input(&self.nodes[node.index()], phase);
            self.nodes[fanout.index()].inv_save += input_save;
        }

        Ok(())
    }

    fn do_invert(&mut self, node: PhaseNodeId) -> Result<(), PhaseError> {
        self.ensure_node(node)?;

        {
            let state = &mut self.nodes[node.index()];
            state.node.inverted = !state.node.inverted;
            std::mem::swap(&mut state.pos_used, &mut state.neg_used);
        }

        for fanout in self.fanouts(node) {
            self.flip_edge_phase(node, fanout)?;
        }

        for fanin in self.fanins(node) {
            let old_phase = self
                .edge_phase(fanin, node)
                .ok_or(PhaseError::MissingEdge {
                    source: fanin,
                    target: node,
                })?;
            let new_phase = old_phase.inverted();
            if old_phase != new_phase {
                let fanin_state = &mut self.nodes[fanin.index()];
                match old_phase {
                    EdgePhase::Negative => {
                        fanin_state.pos_used += 1;
                        fanin_state.neg_used -= 1;
                    }
                    EdgePhase::Positive => {
                        fanin_state.pos_used -= 1;
                        fanin_state.neg_used += 1;
                    }
                    EdgePhase::Both => {}
                }
                self.edges.insert((fanin, node), new_phase);
            }
        }

        Ok(())
    }

    fn flip_edge_phase(
        &mut self,
        source: PhaseNodeId,
        target: PhaseNodeId,
    ) -> Result<(), PhaseError> {
        let phase = self
            .edge_phase(source, target)
            .ok_or(PhaseError::MissingEdge { source, target })?;
        self.edges.insert((source, target), phase.inverted());
        Ok(())
    }

    fn fanouts(&self, node: PhaseNodeId) -> Vec<PhaseNodeId> {
        self.edges
            .keys()
            .filter_map(|(source, target)| (*source == node).then_some(*target))
            .collect()
    }

    fn fanins(&self, node: PhaseNodeId) -> Vec<PhaseNodeId> {
        self.edges
            .keys()
            .filter_map(|(source, target)| (*target == node).then_some(*source))
            .collect()
    }

    fn ensure_node(&self, node: PhaseNodeId) -> Result<(), PhaseError> {
        self.nodes
            .get(node.index())
            .map(|_| ())
            .ok_or(PhaseError::MissingNode { node })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhaseInvertReport {
    pub node: PhaseNodeId,
    pub cost_before: f64,
    pub cost_after: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PhaseError {
    MissingNode {
        node: PhaseNodeId,
    },
    MissingEdge {
        source: PhaseNodeId,
        target: PhaseNodeId,
    },
    DuplicateEdge {
        source: PhaseNodeId,
        target: PhaseNodeId,
    },
    NodeNotInvertible {
        node: PhaseNodeId,
    },
    InvalidInvertibleKind {
        node: PhaseNodeId,
        kind: PhaseNodeKind,
    },
    InvalidArea {
        area: f64,
    },
    InconsistentCost {
        recorded: f64,
        computed: f64,
    },
    InconsistentSaving {
        node: PhaseNodeId,
        recorded: i32,
        computed: i32,
    },
}

impl fmt::Display for PhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode { node } => {
                write!(f, "phase node {} was not found", node.index())
            }
            Self::MissingEdge { source, target } => write!(
                f,
                "phase edge {} -> {} was not found",
                source.index(),
                target.index()
            ),
            Self::DuplicateEdge { source, target } => write!(
                f,
                "duplicate phase edge {} -> {}",
                source.index(),
                target.index()
            ),
            Self::NodeNotInvertible { node } => {
                write!(f, "phase node {} is not invertible", node.index())
            }
            Self::InvalidInvertibleKind { node, kind } => write!(
                f,
                "phase node {} has incompatible invertible kind {:?}",
                node.index(),
                kind
            ),
            Self::InvalidArea { area } => write!(f, "invalid phase area {area}"),
            Self::InconsistentCost { recorded, computed } => write!(
                f,
                "phase cost is inconsistent: recorded {recorded}, computed {computed}"
            ),
            Self::InconsistentSaving {
                node,
                recorded,
                computed,
            } => write!(
                f,
                "phase saving for node {} is inconsistent: recorded {recorded}, computed {computed}",
                node.index()
            ),
        }
    }
}

impl Error for PhaseError {}

fn inv_save_output(state: &PhaseNodeState) -> i32 {
    if state.pos_used == 0 {
        -1
    } else if state.neg_used == 0 {
        1
    } else {
        0
    }
}

fn inv_save_input(state: &PhaseNodeState, phase: EdgePhase) -> i32 {
    match phase {
        EdgePhase::Negative if state.pos_used == 0 => -1,
        EdgePhase::Positive if state.pos_used == 1 => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(index: usize) -> PhaseNodeId {
        PhaseNodeId(index)
    }

    fn internal(name: &str) -> PhaseNode {
        PhaseNode::new(name, PhaseNodeKind::Internal)
    }

    #[test]
    fn setup_counts_row_use_and_initial_savings() {
        let network = PhaseNetwork::new(
            vec![internal("a"), internal("b"), internal("c")],
            vec![
                PhaseEdge::new(id(0), id(1), EdgePhase::Positive),
                PhaseEdge::new(id(0), id(2), EdgePhase::Negative),
                PhaseEdge::new(id(1), id(2), EdgePhase::Both),
            ],
        )
        .unwrap();

        assert_eq!(network.node(id(0)).unwrap().pos_used(), 1);
        assert_eq!(network.node(id(0)).unwrap().neg_used(), 1);
        assert_eq!(network.node(id(0)).unwrap().inv_save(), 0);
        assert_eq!(network.node(id(1)).unwrap().pos_used(), 1);
        assert_eq!(network.node(id(1)).unwrap().neg_used(), 1);
        assert_eq!(network.node(id(2)).unwrap().pos_used(), 0);
        assert_eq!(network.node(id(2)).unwrap().neg_used(), 0);
        assert_eq!(network.cost(), 2.0);
        assert_eq!(network.best_node(), Some(id(1)));
        network.check_consistency().unwrap();
    }

    #[test]
    fn invert_toggles_outgoing_and_incoming_phases_incrementally() {
        let mut network = PhaseNetwork::new(
            vec![internal("a"), internal("b"), internal("c")],
            vec![
                PhaseEdge::new(id(0), id(1), EdgePhase::Positive),
                PhaseEdge::new(id(1), id(2), EdgePhase::Negative),
            ],
        )
        .unwrap();

        let report = network.invert(id(1)).unwrap();

        assert_eq!(report.cost_before, 1.0);
        assert_eq!(report.cost_after, 1.0);
        assert_eq!(network.edge_phase(id(0), id(1)), Some(EdgePhase::Negative));
        assert_eq!(network.edge_phase(id(1), id(2)), Some(EdgePhase::Positive));
        assert!(network.node(id(1)).unwrap().node().inverted);
        assert_eq!(network.node(id(0)).unwrap().pos_used(), 0);
        assert_eq!(network.node(id(0)).unwrap().neg_used(), 1);
        assert_eq!(network.node(id(1)).unwrap().pos_used(), 1);
        assert_eq!(network.node(id(1)).unwrap().neg_used(), 0);
        network.check_consistency().unwrap();
    }

    #[test]
    fn area_mode_uses_dual_gate_delta_and_inverter_area() {
        let mut network = PhaseNetwork::with_inverter_area(
            vec![
                internal("a").with_area(3.0, 5.0),
                internal("b").with_area(7.0, 2.0),
                PhaseNode::new("po", PhaseNodeKind::PrimaryOutput),
            ],
            vec![
                PhaseEdge::new(id(0), id(1), EdgePhase::Positive),
                PhaseEdge::new(id(1), id(2), EdgePhase::Negative),
            ],
            0.5,
        )
        .unwrap();

        assert_eq!(network.cost(), 10.5);
        assert_eq!(network.phase_value(id(1)).unwrap(), 5.0);

        network.invert(id(1)).unwrap();

        assert_eq!(network.cost(), 5.5);
        assert_eq!(network.computed_cost(), 5.5);
    }

    #[test]
    fn marking_excludes_nodes_from_best_selection() {
        let mut network = PhaseNetwork::new(
            vec![internal("a"), internal("b")],
            vec![PhaseEdge::new(id(0), id(1), EdgePhase::Positive)],
        )
        .unwrap();

        assert_eq!(network.best_node(), Some(id(0)));
        network.mark(id(0)).unwrap();
        assert_eq!(network.best_node(), Some(id(1)));
        network.unmark_all();
        assert_eq!(network.best_node(), Some(id(0)));
    }

    #[test]
    fn rejects_noninvertible_node_inversion() {
        let mut network = PhaseNetwork::new(
            vec![
                PhaseNode::new("pi", PhaseNodeKind::PrimaryInput),
                internal("n"),
            ],
            vec![PhaseEdge::new(id(0), id(1), EdgePhase::Positive)],
        )
        .unwrap();

        assert_eq!(
            network.invert(id(0)),
            Err(PhaseError::NodeNotInvertible { node: id(0) })
        );
    }

    #[test]
    fn random_assign_uses_c_style_invertible_scan_order() {
        let mut network = PhaseNetwork::new(
            vec![internal("a"), internal("b"), internal("c")],
            vec![
                PhaseEdge::new(id(0), id(1), EdgePhase::Positive),
                PhaseEdge::new(id(1), id(2), EdgePhase::Positive),
            ],
        )
        .unwrap();
        let choices = [false, true, false];
        let mut index = 0;

        let inverted = network
            .random_assign_with(|| {
                let result = choices[index];
                index += 1;
                result
            })
            .unwrap();

        assert_eq!(inverted, vec![id(1)]);
        assert!(!network.node(id(0)).unwrap().node().inverted);
        assert!(network.node(id(1)).unwrap().node().inverted);
        assert!(!network.node(id(2)).unwrap().node().inverted);
        network.check_consistency().unwrap();
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("phase.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
