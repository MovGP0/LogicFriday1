//! Native Rust model for `LogicSynthesis/sis/pld/xln_collapse.c`.
//!
//! The C file is an area-oriented partial-collapse driver for Xilinx mapping:
//! it computes mapped costs for internal nodes, greedily collapses a node into
//! all fanouts only when the mapped-area gain is positive, reschedules affected
//! fanouts/fanins, remaps remaining infeasible nodes, and evaluates an optional
//! network-level collapse before cofactor/Roth-Karp decomposition. Direct SIS
//! `network_t`, `node_t`, `array_t`, and `st_table` mutation remains gated by
//! explicit dependency errors until those ports are available.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoodOrFast {
    Good,
    Fast,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveFaninsOptions {
    pub move_fanins: bool,
    pub max_fanins: usize,
}

impl Default for MoveFaninsOptions {
    fn default() -> Self {
        Self {
            move_fanins: false,
            max_fanins: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XlnCollapseOptions {
    pub support: usize,
    pub cost_limit: usize,
    pub collapse_input_limit: usize,
    pub flag_decomp_good: u8,
    pub good_or_fast: GoodOrFast,
    pub move_fanins: MoveFaninsOptions,
}

impl XlnCollapseOptions {
    pub const fn new(support: usize) -> Self {
        Self {
            support,
            cost_limit: usize::MAX,
            collapse_input_limit: usize::MAX,
            flag_decomp_good: 0,
            good_or_fast: GoodOrFast::Good,
            move_fanins: MoveFaninsOptions {
                move_fanins: false,
                max_fanins: 0,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollapseNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    deleted: bool,
}

impl CollapseNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            deleted: false,
        }
    }

    pub fn internal(name: impl Into<String>, fanins: Vec<NodeId>) -> Self {
        Self::new(name, NodeKind::Internal).with_fanins(fanins)
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CollapseNetwork {
    nodes: Vec<CollapseNode>,
}

impl CollapseNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: CollapseNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&CollapseNode, XlnCollapseError> {
        self.nodes
            .get(id.0)
            .ok_or(XlnCollapseError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[CollapseNode] {
        &self.nodes
    }

    pub fn active_node_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (!node.deleted).then_some(NodeId(index)))
            .collect()
    }

    pub fn dfs_node_ids(&self) -> Vec<NodeId> {
        self.active_node_ids()
    }

    pub fn internal_node_ids(&self) -> Vec<NodeId> {
        self.active_node_ids()
            .into_iter()
            .filter(|id| self.nodes[id.0].kind == NodeKind::Internal)
            .collect()
    }

    pub fn network_num_internal(&self) -> usize {
        self.internal_node_ids().len()
    }

    pub fn network_num_pi(&self) -> usize {
        self.active_node_ids()
            .into_iter()
            .filter(|id| self.nodes[id.0].kind == NodeKind::PrimaryInput)
            .count()
    }

    pub fn fanouts(&self, node: NodeId) -> Result<Vec<NodeId>, XlnCollapseError> {
        self.node(node)?;
        Ok(self
            .active_node_ids()
            .into_iter()
            .filter(|candidate| self.nodes[candidate.0].fanins.contains(&node))
            .collect())
    }

    pub fn has_primary_output_fanout(&self, node: NodeId) -> Result<bool, XlnCollapseError> {
        Ok(self
            .fanouts(node)?
            .into_iter()
            .any(|fanout| self.nodes[fanout.0].kind == NodeKind::PrimaryOutput))
    }

    pub fn delete_node(&mut self, node: NodeId) -> Result<(), XlnCollapseError> {
        let target = self
            .nodes
            .get_mut(node.0)
            .ok_or(XlnCollapseError::UnknownNode(node))?;
        target.deleted = true;
        Ok(())
    }

    pub fn collapse_fanin_into_fanout(
        &mut self,
        fanout: NodeId,
        fanin: NodeId,
    ) -> Result<(), XlnCollapseError> {
        let replacement_fanins = self.node(fanin)?.fanins.clone();
        let fanout_node = self
            .nodes
            .get_mut(fanout.0)
            .ok_or(XlnCollapseError::UnknownNode(fanout))?;
        if fanout_node.deleted {
            return Err(XlnCollapseError::DeletedNode(fanout));
        }
        if !fanout_node.fanins.contains(&fanin) {
            return Err(XlnCollapseError::NotAFanin { fanout, fanin });
        }

        let mut revised = Vec::new();
        for current in fanout_node.fanins.iter().copied() {
            if current == fanin {
                for replacement in &replacement_fanins {
                    if *replacement != fanout && !revised.contains(replacement) {
                        revised.push(*replacement);
                    }
                }
            } else if !revised.contains(&current) {
                revised.push(current);
            }
        }
        fanout_node.fanins = revised;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MappedNetwork {
    pub internal_count: usize,
}

impl MappedNetwork {
    pub const fn new(internal_count: usize) -> Self {
        Self { internal_count }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialCollapseReport {
    pub collapsed_nodes: Vec<CollapsedNodeReport>,
    pub total_gain: isize,
    pub remaining_costs: HashMap<NodeId, MappedNetwork>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollapsedNodeReport {
    pub node: NodeId,
    pub fanouts: Vec<NodeId>,
    pub gain: isize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollapseAttempt {
    pub node: NodeId,
    pub fanouts: Vec<NodeId>,
    pub gain: isize,
    pub accepted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollapseIteration {
    pub collapsed: Vec<CollapsedNodeReport>,
    pub affected_nodes: Vec<NodeId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemapAction {
    KeepCurrent,
    TryOtherMappingOptions,
    ToggleDecompGood,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemapDecision {
    pub action: RemapAction,
    pub selected: MappedNetwork,
    pub restored_flag_decomp_good: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AreaCollapseCandidate {
    pub internal_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AreaCollapseChoice {
    NotAttempted(AreaCollapseSkipReason),
    CofactorOnly(AreaCollapseCandidate),
    RothKarp(AreaCollapseCandidate),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AreaCollapseSkipReason {
    SupportIsTwo,
    TooManyPrimaryInputs {
        primary_inputs: usize,
        collapse_input_limit: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollapseOperation {
    PartialCollapse,
    PartialCollapseNode,
    CollapseRemap,
    CollapseCheckArea,
    CheckNetworkForCollapsingArea,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnCollapseError {
    UnknownNode(NodeId),
    DeletedNode(NodeId),
    NotAFanin { fanout: NodeId, fanin: NodeId },
    MissingCost(NodeId),
    InvalidSupport { support: usize },
    MissingNativePorts { operation: CollapseOperation },
}

impl fmt::Display for XlnCollapseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_collapse node {:?}", node),
            Self::DeletedNode(node) => write!(f, "xln_collapse node {:?} was deleted", node),
            Self::NotAFanin { fanout, fanin } => {
                write!(f, "node {:?} is not a fanin of {:?}", fanin, fanout)
            }
            Self::MissingCost(node) => write!(f, "missing mapped cost for node {:?}", node),
            Self::InvalidSupport { support } => {
                write!(f, "PLD support must be positive, got {support}")
            }
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation:?} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for XlnCollapseError {}

pub fn partial_collapse_blocked<Network>(
    _network: &mut Network,
    _options: XlnCollapseOptions,
) -> Result<(), XlnCollapseError> {
    missing_native_ports(CollapseOperation::PartialCollapse)
}

pub fn partial_collapse_node_blocked<Network, Node>(
    _network: &mut Network,
    _node: &Node,
    _options: XlnCollapseOptions,
) -> Result<(), XlnCollapseError> {
    missing_native_ports(CollapseOperation::PartialCollapseNode)
}

pub fn collapse_remap_blocked<Node>(
    _node: &Node,
    _options: XlnCollapseOptions,
) -> Result<(), XlnCollapseError> {
    missing_native_ports(CollapseOperation::CollapseRemap)
}

pub fn collapse_check_area_blocked<Network>(
    _network: &mut Network,
    _options: XlnCollapseOptions,
    _roth_karp: bool,
) -> Result<(), XlnCollapseError> {
    missing_native_ports(CollapseOperation::CollapseCheckArea)
}

fn missing_native_ports<T>(operation: CollapseOperation) -> Result<T, XlnCollapseError> {
    Err(XlnCollapseError::MissingNativePorts { operation })
}

pub fn xln_cost_of_node(
    node: NodeId,
    cost_table: &HashMap<NodeId, MappedNetwork>,
) -> Result<usize, XlnCollapseError> {
    Ok(cost_table
        .get(&node)
        .ok_or(XlnCollapseError::MissingCost(node))?
        .internal_count)
}

pub fn build_initial_cost_table<F>(
    network: &CollapseNetwork,
    mut map_node: F,
) -> Result<HashMap<NodeId, MappedNetwork>, XlnCollapseError>
where
    F: FnMut(&CollapseNetwork, NodeId) -> Result<MappedNetwork, XlnCollapseError>,
{
    let mut cost_table = HashMap::new();
    for node in network.dfs_node_ids() {
        if network.node(node)?.kind == NodeKind::Internal {
            cost_table.insert(node, map_node(network, node)?);
        }
    }
    Ok(cost_table)
}

pub fn partial_collapse_node_with_costs<F>(
    network: &mut CollapseNetwork,
    node: NodeId,
    cost_table: &mut HashMap<NodeId, MappedNetwork>,
    mut map_collapsed_fanout: F,
) -> Result<CollapseAttempt, XlnCollapseError>
where
    F: FnMut(&CollapseNetwork, NodeId, NodeId) -> Result<MappedNetwork, XlnCollapseError>,
{
    let fanouts = network.fanouts(node)?;
    let cost_node = xln_cost_of_node(node, cost_table)?;
    let mut gain = cost_node as isize;
    let mut new_fanout_costs = Vec::with_capacity(fanouts.len());

    for fanout in &fanouts {
        let new_cost = map_collapsed_fanout(network, *fanout, node)?;
        let old_cost = xln_cost_of_node(*fanout, cost_table)?;
        gain += old_cost as isize - new_cost.internal_count as isize;
        new_fanout_costs.push((*fanout, new_cost));
    }

    if gain <= 0 {
        return Ok(CollapseAttempt {
            node,
            fanouts,
            gain,
            accepted: false,
        });
    }

    for (fanout, cost) in new_fanout_costs {
        network.collapse_fanin_into_fanout(fanout, node)?;
        cost_table.insert(fanout, cost);
    }
    cost_table
        .remove(&node)
        .ok_or(XlnCollapseError::MissingCost(node))?;

    Ok(CollapseAttempt {
        node,
        fanouts,
        gain,
        accepted: true,
    })
}

pub fn partial_collapse_iteration<F>(
    network: &mut CollapseNetwork,
    candidates: &[NodeId],
    cost_table: &mut HashMap<NodeId, MappedNetwork>,
    options: &XlnCollapseOptions,
    mut map_collapsed_fanout: F,
) -> Result<CollapseIteration, XlnCollapseError>
where
    F: FnMut(&CollapseNetwork, NodeId, NodeId) -> Result<MappedNetwork, XlnCollapseError>,
{
    let mut affected = HashSet::new();
    let mut collapsed = Vec::new();

    for node in candidates {
        affected.remove(node);
        let node_ref = network.node(*node)?;
        if node_ref.kind != NodeKind::Internal || network.has_primary_output_fanout(*node)? {
            continue;
        }
        if xln_cost_of_node(*node, cost_table)? > options.cost_limit {
            continue;
        }

        let original_fanouts = network.fanouts(*node)?;
        let original_fanins = node_ref.fanins.clone();
        let attempt = partial_collapse_node_with_costs(
            network,
            *node,
            cost_table,
            &mut map_collapsed_fanout,
        )?;
        if !attempt.accepted {
            continue;
        }

        for fanout in &original_fanouts {
            affected.insert(*fanout);
            for fanin in &network.node(*fanout)?.fanins {
                if *fanin != *node {
                    affected.insert(*fanin);
                }
            }
        }
        for fanin in original_fanins {
            affected.insert(fanin);
        }
        network.delete_node(*node)?;
        collapsed.push(CollapsedNodeReport {
            node: *node,
            fanouts: original_fanouts,
            gain: attempt.gain,
        });
    }

    let mut affected_nodes = affected
        .into_iter()
        .filter(|node| {
            network
                .node(*node)
                .is_ok_and(|node_ref| node_ref.kind == NodeKind::Internal && !node_ref.deleted)
        })
        .collect::<Vec<_>>();
    affected_nodes.sort();

    Ok(CollapseIteration {
        collapsed,
        affected_nodes,
    })
}

pub fn partial_collapse_network<F, G>(
    network: &mut CollapseNetwork,
    options: XlnCollapseOptions,
    map_node: F,
    mut map_collapsed_fanout: G,
) -> Result<PartialCollapseReport, XlnCollapseError>
where
    F: FnMut(&CollapseNetwork, NodeId) -> Result<MappedNetwork, XlnCollapseError>,
    G: FnMut(&CollapseNetwork, NodeId, NodeId) -> Result<MappedNetwork, XlnCollapseError>,
{
    if options.support == 0 {
        return Err(XlnCollapseError::InvalidSupport {
            support: options.support,
        });
    }

    let mut cost_table = build_initial_cost_table(network, map_node)?;
    let mut candidates = network.dfs_node_ids();
    let mut collapsed_nodes = Vec::new();
    let mut total_gain = 0;

    while !candidates.is_empty() {
        let iteration = partial_collapse_iteration(
            network,
            &candidates,
            &mut cost_table,
            &options,
            &mut map_collapsed_fanout,
        )?;
        total_gain += iteration
            .collapsed
            .iter()
            .map(|entry| entry.gain)
            .sum::<isize>();
        collapsed_nodes.extend(iteration.collapsed);
        candidates = iteration.affected_nodes;
    }

    Ok(PartialCollapseReport {
        collapsed_nodes,
        total_gain,
        remaining_costs: cost_table,
    })
}

pub fn collapse_remap_choice(
    current: MappedNetwork,
    options: &XlnCollapseOptions,
    alternate_from_toggled_flag: MappedNetwork,
    alternate_from_other_options: MappedNetwork,
) -> RemapDecision {
    if options.good_or_fast == GoodOrFast::Good {
        return RemapDecision {
            action: RemapAction::KeepCurrent,
            selected: current,
            restored_flag_decomp_good: options.flag_decomp_good,
        };
    }

    if options.flag_decomp_good == 2 {
        return RemapDecision {
            action: RemapAction::TryOtherMappingOptions,
            selected: alternate_from_other_options,
            restored_flag_decomp_good: options.flag_decomp_good,
        };
    }

    let selected = if alternate_from_toggled_flag.internal_count < current.internal_count {
        alternate_from_toggled_flag
    } else {
        current
    };
    RemapDecision {
        action: RemapAction::ToggleDecompGood,
        selected,
        restored_flag_decomp_good: options.flag_decomp_good,
    }
}

pub fn check_network_for_collapsing_area(
    network: &CollapseNetwork,
    options: &XlnCollapseOptions,
    roth_karp: bool,
    cofactor_candidate: AreaCollapseCandidate,
    roth_karp_candidate: Option<AreaCollapseCandidate>,
) -> Result<AreaCollapseChoice, XlnCollapseError> {
    if options.support == 0 {
        return Err(XlnCollapseError::InvalidSupport {
            support: options.support,
        });
    }
    if options.support == 2 {
        return Ok(AreaCollapseChoice::NotAttempted(
            AreaCollapseSkipReason::SupportIsTwo,
        ));
    }

    let primary_inputs = network.network_num_pi();
    if primary_inputs > options.collapse_input_limit {
        return Ok(AreaCollapseChoice::NotAttempted(
            AreaCollapseSkipReason::TooManyPrimaryInputs {
                primary_inputs,
                collapse_input_limit: options.collapse_input_limit,
            },
        ));
    }

    if roth_karp {
        if let Some(karp) = roth_karp_candidate {
            if cofactor_candidate.internal_count < karp.internal_count {
                return Ok(AreaCollapseChoice::CofactorOnly(cofactor_candidate));
            }
            return Ok(AreaCollapseChoice::RothKarp(karp));
        }
    }

    Ok(AreaCollapseChoice::CofactorOnly(cofactor_candidate))
}

pub fn collapse_check_area_replacement(
    current: MappedNetwork,
    collapsed_candidate: Option<AreaCollapseCandidate>,
) -> MappedNetwork {
    match collapsed_candidate {
        Some(candidate) if current.internal_count > candidate.internal_count => {
            MappedNetwork::new(candidate.internal_count)
        }
        _ => current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options() -> XlnCollapseOptions {
        XlnCollapseOptions {
            support: 5,
            cost_limit: 10,
            collapse_input_limit: 6,
            flag_decomp_good: 0,
            good_or_fast: GoodOrFast::Fast,
            move_fanins: MoveFaninsOptions::default(),
        }
    }

    fn sample_network() -> (CollapseNetwork, NodeId, NodeId, NodeId, NodeId) {
        let mut network = CollapseNetwork::new();
        let a = network.add_node(CollapseNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(CollapseNode::new("b", NodeKind::PrimaryInput));
        let x = network.add_node(CollapseNode::internal("x", vec![a, b]));
        let y = network.add_node(CollapseNode::internal("y", vec![x, b]));
        let z = network.add_node(CollapseNode::internal("z", vec![x, y]));
        (network, x, y, z, b)
    }

    #[test]
    fn cost_of_node_returns_mapped_internal_count() {
        let table = HashMap::from([(NodeId(1), MappedNetwork::new(3))]);

        assert_eq!(xln_cost_of_node(NodeId(1), &table), Ok(3));
        assert_eq!(
            xln_cost_of_node(NodeId(2), &table),
            Err(XlnCollapseError::MissingCost(NodeId(2)))
        );
    }

    #[test]
    fn partial_collapse_node_rejects_non_positive_gain_without_mutation() {
        let (mut network, x, y, z, _) = sample_network();
        let mut costs = HashMap::from([
            (x, MappedNetwork::new(1)),
            (y, MappedNetwork::new(2)),
            (z, MappedNetwork::new(2)),
        ]);

        let attempt = partial_collapse_node_with_costs(&mut network, x, &mut costs, |_, _, _| {
            Ok(MappedNetwork::new(3))
        })
        .unwrap();

        assert_eq!(
            attempt,
            CollapseAttempt {
                node: x,
                fanouts: vec![y, z],
                gain: -1,
                accepted: false,
            }
        );
        assert_eq!(network.node(y).unwrap().fanins, vec![x, NodeId(1)]);
        assert!(costs.contains_key(&x));
    }

    #[test]
    fn partial_collapse_node_accepts_positive_gain_updates_fanouts_and_costs() {
        let (mut network, x, y, z, b) = sample_network();
        let mut costs = HashMap::from([
            (x, MappedNetwork::new(2)),
            (y, MappedNetwork::new(3)),
            (z, MappedNetwork::new(4)),
        ]);

        let attempt =
            partial_collapse_node_with_costs(&mut network, x, &mut costs, |_, fanout, _| {
                Ok(if fanout == y {
                    MappedNetwork::new(1)
                } else {
                    MappedNetwork::new(2)
                })
            })
            .unwrap();

        assert!(attempt.accepted);
        assert_eq!(attempt.gain, 6);
        assert_eq!(network.node(y).unwrap().fanins, vec![NodeId(0), b]);
        assert_eq!(network.node(z).unwrap().fanins, vec![NodeId(0), b, y]);
        assert!(!costs.contains_key(&x));
        assert_eq!(costs[&y], MappedNetwork::new(1));
        assert_eq!(costs[&z], MappedNetwork::new(2));
    }

    #[test]
    fn iteration_reschedules_collapsed_fanouts_and_fanins() {
        let (mut network, x, y, z, b) = sample_network();
        let mut costs = HashMap::from([
            (x, MappedNetwork::new(2)),
            (y, MappedNetwork::new(3)),
            (z, MappedNetwork::new(4)),
        ]);

        let iteration = partial_collapse_iteration(
            &mut network,
            &[x],
            &mut costs,
            &default_options(),
            |_, _, _| Ok(MappedNetwork::new(1)),
        )
        .unwrap();

        assert_eq!(iteration.collapsed.len(), 1);
        assert_eq!(iteration.collapsed[0].node, x);
        assert!(network.node(x).unwrap().is_deleted());
        assert_eq!(iteration.affected_nodes, vec![y, z]);
        assert_eq!(network.node(y).unwrap().fanins, vec![NodeId(0), b]);
    }

    #[test]
    fn primary_output_fanout_and_cost_limit_prevent_collapse_attempts() {
        let mut network = CollapseNetwork::new();
        let a = network.add_node(CollapseNode::new("a", NodeKind::PrimaryInput));
        let x = network.add_node(CollapseNode::internal("x", vec![a]));
        network.add_node(CollapseNode::new("out", NodeKind::PrimaryOutput).with_fanins(vec![x]));
        let mut costs = HashMap::from([(x, MappedNetwork::new(1))]);
        let mut called = false;

        let iteration = partial_collapse_iteration(
            &mut network,
            &[x],
            &mut costs,
            &default_options(),
            |_, _, _| {
                called = true;
                Ok(MappedNetwork::new(0))
            },
        )
        .unwrap();

        assert!(iteration.collapsed.is_empty());
        assert!(!called);
        assert!(!network.node(x).unwrap().is_deleted());
    }

    #[test]
    fn network_partial_collapse_repeats_on_affected_internal_nodes() {
        let (mut network, x, y, z, _) = sample_network();

        let report = partial_collapse_network(
            &mut network,
            default_options(),
            |_, node| {
                Ok(match node {
                    id if id == x => MappedNetwork::new(2),
                    id if id == y => MappedNetwork::new(3),
                    id if id == z => MappedNetwork::new(3),
                    _ => MappedNetwork::new(1),
                })
            },
            |_, fanout, collapsed| {
                Ok(if collapsed == x || fanout == z {
                    MappedNetwork::new(1)
                } else {
                    MappedNetwork::new(3)
                })
            },
        )
        .unwrap();

        assert_eq!(
            report
                .collapsed_nodes
                .iter()
                .map(|entry| entry.node)
                .collect::<Vec<_>>(),
            vec![x, y, z]
        );
        assert_eq!(report.total_gain, 8);
        assert!(network.node(x).unwrap().is_deleted());
        assert!(network.node(y).unwrap().is_deleted());
        assert!(network.node(z).unwrap().is_deleted());
    }

    #[test]
    fn collapse_remap_matches_c_good_fast_and_flag_two_branches() {
        let current = MappedNetwork::new(5);
        let toggled = MappedNetwork::new(3);
        let other = MappedNetwork::new(2);
        let mut options = default_options();

        assert_eq!(
            collapse_remap_choice(current, &options, toggled, other),
            RemapDecision {
                action: RemapAction::ToggleDecompGood,
                selected: toggled,
                restored_flag_decomp_good: 0,
            }
        );

        options.flag_decomp_good = 2;
        assert_eq!(
            collapse_remap_choice(current, &options, toggled, other).action,
            RemapAction::TryOtherMappingOptions
        );

        options.good_or_fast = GoodOrFast::Good;
        assert_eq!(
            collapse_remap_choice(current, &options, toggled, other).selected,
            current
        );
    }

    #[test]
    fn area_collapse_skips_support_two_and_input_limit() {
        let mut network = CollapseNetwork::new();
        network.add_node(CollapseNode::new("a", NodeKind::PrimaryInput));
        network.add_node(CollapseNode::new("b", NodeKind::PrimaryInput));
        let mut options = default_options();
        options.support = 2;

        assert_eq!(
            check_network_for_collapsing_area(
                &network,
                &options,
                true,
                AreaCollapseCandidate { internal_count: 1 },
                Some(AreaCollapseCandidate { internal_count: 2 })
            )
            .unwrap(),
            AreaCollapseChoice::NotAttempted(AreaCollapseSkipReason::SupportIsTwo)
        );

        options.support = 5;
        options.collapse_input_limit = 1;
        assert_eq!(
            check_network_for_collapsing_area(
                &network,
                &options,
                false,
                AreaCollapseCandidate { internal_count: 1 },
                None
            )
            .unwrap(),
            AreaCollapseChoice::NotAttempted(AreaCollapseSkipReason::TooManyPrimaryInputs {
                primary_inputs: 2,
                collapse_input_limit: 1,
            })
        );
    }

    #[test]
    fn area_collapse_selects_smaller_cofactor_or_roth_karp_candidate() {
        let network = CollapseNetwork::new();
        let options = default_options();

        assert_eq!(
            check_network_for_collapsing_area(
                &network,
                &options,
                true,
                AreaCollapseCandidate { internal_count: 3 },
                Some(AreaCollapseCandidate { internal_count: 5 })
            )
            .unwrap(),
            AreaCollapseChoice::CofactorOnly(AreaCollapseCandidate { internal_count: 3 })
        );
        assert_eq!(
            check_network_for_collapsing_area(
                &network,
                &options,
                true,
                AreaCollapseCandidate { internal_count: 5 },
                Some(AreaCollapseCandidate { internal_count: 3 })
            )
            .unwrap(),
            AreaCollapseChoice::RothKarp(AreaCollapseCandidate { internal_count: 3 })
        );
    }

    #[test]
    fn collapse_check_area_replaces_only_with_strictly_smaller_candidate() {
        assert_eq!(
            collapse_check_area_replacement(
                MappedNetwork::new(4),
                Some(AreaCollapseCandidate { internal_count: 3 })
            ),
            MappedNetwork::new(3)
        );
        assert_eq!(
            collapse_check_area_replacement(
                MappedNetwork::new(4),
                Some(AreaCollapseCandidate { internal_count: 4 })
            ),
            MappedNetwork::new(4)
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_collapse.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
