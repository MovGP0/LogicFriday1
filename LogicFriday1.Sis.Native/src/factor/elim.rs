//! Native Rust model for SIS selective factor elimination.
//!
//! The C implementation repeatedly orders nodes, estimates whether collapsing a
//! candidate into each fanout would exceed a cover-size limit, and then performs
//! accepted collapses. This module keeps that behavior over owned Rust data so
//! higher-level integration can adapt real network/node ports later.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElimNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase {
    PositiveUnate,
    NegativeUnate,
    Binate,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorTree {
    Zero,
    One,
    Leaf(usize),
    Inverter(Box<FactorTree>),
    And(Vec<FactorTree>),
    Or(Vec<FactorTree>),
}

impl FactorTree {
    pub fn leaf(index: usize) -> Self {
        Self::Leaf(index)
    }

    pub fn inverter(child: FactorTree) -> Self {
        Self::Inverter(Box::new(child))
    }

    pub fn and(children: impl IntoIterator<Item = FactorTree>) -> Self {
        Self::And(children.into_iter().collect())
    }

    pub fn or(children: impl IntoIterator<Item = FactorTree>) -> Self {
        Self::Or(children.into_iter().collect())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElimNode {
    pub id: NodeId,
    pub name: String,
    pub kind: ElimNodeKind,
    pub fanins: Vec<NodeId>,
    pub fanouts: Vec<NodeId>,
    pub cube_count: usize,
    pub value: i32,
    pub order_level: i32,
    pub factor: Option<FactorTree>,
    removed: bool,
}

impl ElimNode {
    pub fn primary_input(id: usize, name: impl Into<String>) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind: ElimNodeKind::PrimaryInput,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            cube_count: 0,
            value: i32::MAX,
            order_level: 0,
            factor: None,
            removed: false,
        }
    }

    pub fn primary_output(id: usize, name: impl Into<String>, fanin: NodeId) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind: ElimNodeKind::PrimaryOutput,
            fanins: vec![fanin],
            fanouts: Vec::new(),
            cube_count: 0,
            value: i32::MAX,
            order_level: 0,
            factor: None,
            removed: false,
        }
    }

    pub fn internal(
        id: usize,
        name: impl Into<String>,
        fanins: impl Into<Vec<NodeId>>,
        cube_count: usize,
        value: i32,
        factor: FactorTree,
    ) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind: ElimNodeKind::Internal,
            fanins: fanins.into(),
            fanouts: Vec::new(),
            cube_count,
            value,
            order_level: 0,
            factor: Some(factor),
            removed: false,
        }
    }

    pub fn with_order_level(mut self, order_level: i32) -> Self {
        self.order_level = order_level;
        self
    }

    pub fn with_fanouts(mut self, fanouts: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanouts = fanouts.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EliminateReport {
    pub collapsed_edges: Vec<(NodeId, NodeId)>,
    pub rejected_by_primary_output: Vec<NodeId>,
    pub rejected_by_limit: Vec<(NodeId, NodeId, usize)>,
    pub passes: usize,
    pub effective_limit: usize,
}

impl EliminateReport {
    pub fn collapsed_nodes(&self) -> Vec<NodeId> {
        let mut nodes = Vec::new();
        for (_, eliminated) in &self.collapsed_edges {
            if !nodes.contains(eliminated) {
                nodes.push(*eliminated);
            }
        }
        nodes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EliminateError {
    UnknownNode(NodeId),
    MissingFactor(NodeId),
    MissingInverterChild,
    InvalidLeafIndex { node: NodeId, index: usize },
}

impl fmt::Display for EliminateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown eliminate node {}", node.0),
            Self::MissingFactor(node) => write!(f, "node {} has no factor tree", node.0),
            Self::MissingInverterChild => write!(f, "inverter factor has no child"),
            Self::InvalidLeafIndex { node, index } => {
                write!(f, "factor leaf index {index} is outside node {}", node.0)
            }
        }
    }
}

impl Error for EliminateError {}

pub type EliminateResult<T> = Result<T, EliminateError>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ElimNetwork {
    nodes: Vec<ElimNode>,
}

impl ElimNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, mut node: ElimNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.id = id;
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> EliminateResult<&ElimNode> {
        self.nodes.get(id.0).ok_or(EliminateError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> EliminateResult<&mut ElimNode> {
        self.nodes
            .get_mut(id.0)
            .ok_or(EliminateError::UnknownNode(id))
    }

    pub fn connect(&mut self, fanin: NodeId, fanout: NodeId) -> EliminateResult<()> {
        self.node(fanin)?;
        self.node(fanout)?;

        if !self.nodes[fanout.0].fanins.contains(&fanin) {
            self.nodes[fanout.0].fanins.push(fanin);
        }

        if !self.nodes[fanin.0].fanouts.contains(&fanout) {
            self.nodes[fanin.0].fanouts.push(fanout);
        }

        Ok(())
    }

    pub fn active_node_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| !node.removed)
            .map(|node| node.id)
            .collect()
    }

    pub fn eliminate(&mut self, threshold: i32, limit: usize) -> EliminateResult<EliminateReport> {
        eliminate(self, threshold, limit)
    }
}

pub fn eliminate(
    network: &mut ElimNetwork,
    threshold: i32,
    mut limit: usize,
) -> EliminateResult<EliminateReport> {
    let mut report = EliminateReport {
        effective_limit: limit,
        ..EliminateReport::default()
    };
    let mut dont_elim = HashSet::new();
    let mut eliminating = true;

    while eliminating {
        cleanup(network);
        eliminating = false;
        report.passes += 1;

        let nodevec = order_nodes_elim(network);
        let new_limit = nodevec
            .iter()
            .map(|node| network.nodes[node.0].cube_count.saturating_mul(2))
            .max()
            .unwrap_or(0);
        if limit > new_limit {
            limit = new_limit;
        }
        report.effective_limit = limit;

        for candidate in nodevec {
            let candidate_node = network.node(candidate)?;
            if candidate_node.kind == ElimNodeKind::PrimaryInput
                || candidate_node.value > threshold
                || dont_elim.contains(&candidate)
            {
                continue;
            }

            let fanouts = candidate_node.fanouts.clone();
            let mut elimok = true;
            for fanout in &fanouts {
                let fanout_node = network.node(*fanout)?;
                if fanout_node.kind == ElimNodeKind::PrimaryOutput {
                    elimok = false;
                    dont_elim.insert(candidate);
                    report.rejected_by_primary_output.push(candidate);
                    break;
                }

                let estimate = collapse_estimate(network, *fanout, candidate)?;
                if estimate > limit {
                    elimok = false;
                    dont_elim.insert(candidate);
                    report
                        .rejected_by_limit
                        .push((candidate, *fanout, estimate));
                    break;
                }
            }

            if elimok {
                for fanout in fanouts {
                    if collapse_node(network, fanout, candidate)? {
                        eliminating = true;
                        report.collapsed_edges.push((fanout, candidate));
                        let fanins = network.node(fanout)?.fanins.clone();
                        for fanin in fanins {
                            dont_elim.remove(&fanin);
                        }
                        dont_elim.remove(&fanout);
                    }
                }
            }
        }
    }

    Ok(report)
}

pub fn order_nodes_elim(network: &ElimNetwork) -> Vec<NodeId> {
    let mut entries = network
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| !node.removed && node.kind != ElimNodeKind::PrimaryOutput)
        .map(|(original_order, node)| (node.id, node.order_level, original_order))
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| right.2.cmp(&left.2)));
    entries.into_iter().map(|(node, _, _)| node).collect()
}

pub fn collapse_estimate(
    network: &ElimNetwork,
    fanout: NodeId,
    candidate: NodeId,
) -> EliminateResult<usize> {
    let fanout_node = network.node(fanout)?;
    let phase = input_phase(fanout_node, candidate);
    if phase == InputPhase::Unknown {
        return Ok(fanout_node.cube_count);
    }

    let complement_size = match phase {
        InputPhase::NegativeUnate | InputPhase::Binate => {
            let candidate_node = network.node(candidate)?;
            complement_estimate(
                candidate_node
                    .factor
                    .as_ref()
                    .ok_or(EliminateError::MissingFactor(candidate))?,
            )
        }
        InputPhase::PositiveUnate | InputPhase::Unknown => 0,
    };

    cover_estimate(
        network,
        fanout,
        candidate,
        fanout_node
            .factor
            .as_ref()
            .ok_or(EliminateError::MissingFactor(fanout))?,
        complement_size,
    )
}

pub fn complement_estimate(factor: &FactorTree) -> usize {
    match factor {
        FactorTree::One => 0,
        FactorTree::Zero | FactorTree::Leaf(_) | FactorTree::Inverter(_) => 1,
        FactorTree::And(children) => children.iter().map(complement_estimate).sum(),
        FactorTree::Or(children) => children.iter().map(complement_estimate).product::<usize>(),
    }
}

fn cover_estimate(
    network: &ElimNetwork,
    fanout: NodeId,
    candidate: NodeId,
    root: &FactorTree,
    complement_size: usize,
) -> EliminateResult<usize> {
    match root {
        FactorTree::Leaf(index) => cov_leaf_est(network, fanout, candidate, *index),
        FactorTree::Inverter(child) => {
            cov_inv_est(network, fanout, candidate, child, complement_size)
        }
        FactorTree::And(children) => {
            let mut size = 1usize;
            for child in children {
                size = size.saturating_mul(cover_estimate(
                    network,
                    fanout,
                    candidate,
                    child,
                    complement_size,
                )?);
            }
            Ok(size)
        }
        FactorTree::Or(children) => {
            let mut size = 0usize;
            for child in children {
                size = size.saturating_add(cover_estimate(
                    network,
                    fanout,
                    candidate,
                    child,
                    complement_size,
                )?);
            }
            Ok(size)
        }
        FactorTree::Zero | FactorTree::One => Ok(0),
    }
}

fn cov_leaf_est(
    network: &ElimNetwork,
    fanout: NodeId,
    candidate: NodeId,
    index: usize,
) -> EliminateResult<usize> {
    let fanout_node = network.node(fanout)?;
    let fanin = fanout_node
        .fanins
        .get(index)
        .ok_or(EliminateError::InvalidLeafIndex {
            node: fanout,
            index,
        })?;

    if *fanin == candidate {
        Ok(network.node(candidate)?.cube_count)
    } else {
        Ok(1)
    }
}

fn cov_inv_est(
    network: &ElimNetwork,
    fanout: NodeId,
    candidate: NodeId,
    child: &FactorTree,
    complement_size: usize,
) -> EliminateResult<usize> {
    let FactorTree::Leaf(index) = child else {
        return cover_estimate(network, fanout, candidate, child, complement_size);
    };
    let fanout_node = network.node(fanout)?;
    let fanin = fanout_node
        .fanins
        .get(*index)
        .ok_or(EliminateError::InvalidLeafIndex {
            node: fanout,
            index: *index,
        })?;

    if *fanin == candidate {
        Ok(complement_size)
    } else {
        Ok(1)
    }
}

fn input_phase(fanout: &ElimNode, candidate: NodeId) -> InputPhase {
    let mut positive = false;
    let mut negative = false;

    fn walk(
        factor: &FactorTree,
        inverted: bool,
        candidate_index: usize,
        positive: &mut bool,
        negative: &mut bool,
    ) {
        match factor {
            FactorTree::Leaf(index) if *index == candidate_index => {
                if inverted {
                    *negative = true;
                } else {
                    *positive = true;
                }
            }
            FactorTree::Inverter(child) => {
                walk(child, !inverted, candidate_index, positive, negative);
            }
            FactorTree::And(children) | FactorTree::Or(children) => {
                for child in children {
                    walk(child, inverted, candidate_index, positive, negative);
                }
            }
            FactorTree::Zero | FactorTree::One | FactorTree::Leaf(_) => {}
        }
    }

    let Some(candidate_index) = fanout.fanins.iter().position(|fanin| *fanin == candidate) else {
        return InputPhase::Unknown;
    };
    let Some(factor) = fanout.factor.as_ref() else {
        return InputPhase::Unknown;
    };

    walk(factor, false, candidate_index, &mut positive, &mut negative);
    match (positive, negative) {
        (true, true) => InputPhase::Binate,
        (true, false) => InputPhase::PositiveUnate,
        (false, true) => InputPhase::NegativeUnate,
        (false, false) => InputPhase::Unknown,
    }
}

fn collapse_node(
    network: &mut ElimNetwork,
    fanout: NodeId,
    candidate: NodeId,
) -> EliminateResult<bool> {
    if network.node(fanout)?.kind != ElimNodeKind::Internal
        || network.node(candidate)?.kind != ElimNodeKind::Internal
    {
        return Ok(false);
    }

    if !network.node(fanout)?.fanins.contains(&candidate) {
        return Ok(false);
    }

    let replacement_fanins = network.node(candidate)?.fanins.clone();
    {
        let fanout_node = network.node_mut(fanout)?;
        fanout_node.fanins.retain(|fanin| *fanin != candidate);
        for fanin in replacement_fanins {
            if !fanout_node.fanins.contains(&fanin) {
                fanout_node.fanins.push(fanin);
            }
        }
    }

    network
        .node_mut(candidate)?
        .fanouts
        .retain(|node| *node != fanout);
    for node in &mut network.nodes {
        node.fanouts.retain(|fanout_id| *fanout_id != fanout);
    }

    let fanins = network.node(fanout)?.fanins.clone();
    for fanin in fanins {
        if !network.node(fanin)?.fanouts.contains(&fanout) {
            network.node_mut(fanin)?.fanouts.push(fanout);
        }
    }

    Ok(true)
}

fn cleanup(network: &mut ElimNetwork) {
    let removable = network
        .nodes
        .iter()
        .filter(|node| {
            !node.removed && node.kind == ElimNodeKind::Internal && node.fanouts.is_empty()
        })
        .map(|node| node.id)
        .collect::<Vec<_>>();

    for node in removable {
        network.nodes[node.0].removed = true;
        let fanins = network.nodes[node.0].fanins.clone();
        for fanin in fanins {
            if let Some(fanin_node) = network.nodes.get_mut(fanin.0) {
                fanin_node.fanouts.retain(|fanout| *fanout != node);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> (ElimNetwork, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let mut network = ElimNetwork::new();
        let a = network.add_node(ElimNode::primary_input(0, "a"));
        let b = network.add_node(ElimNode::primary_input(1, "b"));
        let n = network.add_node(
            ElimNode::internal(
                2,
                "n",
                [a, b],
                3,
                -1,
                FactorTree::or([FactorTree::leaf(0), FactorTree::leaf(1)]),
            )
            .with_order_level(1),
        );
        let f = network.add_node(
            ElimNode::internal(
                3,
                "f",
                [n, b],
                2,
                4,
                FactorTree::and([FactorTree::leaf(0), FactorTree::leaf(1)]),
            )
            .with_order_level(2),
        );
        let y = network.add_node(ElimNode::primary_output(4, "y", f));

        network.connect(a, n).unwrap();
        network.connect(b, n).unwrap();
        network.connect(n, f).unwrap();
        network.connect(b, f).unwrap();
        network.connect(f, y).unwrap();

        (network, a, b, n, f, y)
    }

    #[test]
    fn complement_estimate_matches_factor_tree_rules() {
        let factor = FactorTree::or([
            FactorTree::and([FactorTree::leaf(0), FactorTree::leaf(1)]),
            FactorTree::inverter(FactorTree::leaf(2)),
        ]);

        assert_eq!(complement_estimate(&FactorTree::One), 0);
        assert_eq!(complement_estimate(&FactorTree::Zero), 1);
        assert_eq!(complement_estimate(&factor), 2);
    }

    #[test]
    fn collapse_estimate_uses_candidate_cover_for_positive_leaf() {
        let (network, _a, _b, n, f, _y) = sample_network();

        assert_eq!(collapse_estimate(&network, f, n).unwrap(), 3);
    }

    #[test]
    fn collapse_estimate_uses_complement_for_negative_or_binate_input() {
        let mut network = ElimNetwork::new();
        let a = network.add_node(ElimNode::primary_input(0, "a"));
        let n = network.add_node(ElimNode::internal(1, "n", [a], 4, 0, FactorTree::leaf(0)));
        let f = network.add_node(ElimNode::internal(
            2,
            "f",
            [n],
            1,
            0,
            FactorTree::inverter(FactorTree::leaf(0)),
        ));
        network.connect(a, n).unwrap();
        network.connect(n, f).unwrap();

        assert_eq!(collapse_estimate(&network, f, n).unwrap(), 1);
    }

    #[test]
    fn order_nodes_elim_excludes_primary_outputs_and_reverses_original_order_on_ties() {
        let (network, _a, _b, n, f, _y) = sample_network();

        let ordered = order_nodes_elim(&network);

        assert_eq!(ordered, vec![NodeId(1), NodeId(0), n, f]);
    }

    #[test]
    fn eliminate_collapses_low_value_candidate_and_cleans_dead_node() {
        let (mut network, _a, _b, n, f, _y) = sample_network();

        let report = network.eliminate(0, 20).unwrap();

        assert_eq!(report.collapsed_edges, vec![(f, n)]);
        assert!(!network.active_node_ids().contains(&n));
        assert!(!network.node(f).unwrap().fanins.contains(&n));
    }

    #[test]
    fn eliminate_vetoes_candidates_that_fan_out_to_primary_outputs() {
        let (mut network, _a, _b, _n, f, _y) = sample_network();
        network.node_mut(f).unwrap().value = -1;

        let report = network.eliminate(0, 20).unwrap();

        assert_eq!(report.rejected_by_primary_output, vec![f]);
        assert!(network.active_node_ids().contains(&f));
    }

    #[test]
    fn eliminate_vetoes_candidates_when_collapse_estimate_exceeds_limit() {
        let (mut network, _a, _b, n, _f, _y) = sample_network();

        let report = network.eliminate(0, 2).unwrap();

        assert_eq!(report.rejected_by_limit, vec![(n, NodeId(3), 3)]);
        assert!(network.active_node_ids().contains(&n));
    }

    #[test]
    fn no_legacy_c_abi_or_dependency_metadata_tokens_are_present() {
        let source = include_str!("elim.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }
}
