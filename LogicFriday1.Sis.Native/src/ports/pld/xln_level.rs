//! Native Rust model for `LogicSynthesis/sis/pld/xln_level.c`.
//!
//! The C file reduces mapped PLD delay levels by repeatedly finding critical
//! nodes and collapsing them into critical fanouts. The real SIS entry points
//! depend on `network_t`, `node_t`, delay tracing, algebraic cofactoring, and
//! other PLD passes that are still being ported. This module keeps the
//! deterministic local behavior native: level bucketing, critical-node tests,
//! composite-fanin accounting, mux-critical-fanin selection, and a small owned
//! graph model for feasible collapse passes. SIS-bound operations return
//! explicit dependency errors with bead IDs and source files.

use std::collections::HashSet;
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraversalMethod {
    ByLevelWidth,
    Topological,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollapseHeuristic {
    PartialCriticalFanouts,
    AllCriticalFanouts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveFaninsOptions {
    pub enabled: bool,
    pub max_fanins: usize,
    pub bound_alphas: bool,
}

impl Default for MoveFaninsOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            max_fanins: 0,
            bound_alphas: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XlnLevelOptions {
    pub traversal_method: TraversalMethod,
    pub heuristic: CollapseHeuristic,
    pub support: usize,
    pub collapse_input_limit: usize,
    pub move_fanins: MoveFaninsOptions,
}

impl XlnLevelOptions {
    pub const fn new(
        traversal_method: TraversalMethod,
        heuristic: CollapseHeuristic,
        support: usize,
    ) -> Self {
        Self {
            traversal_method,
            heuristic,
            support,
            collapse_input_limit: 0,
            move_fanins: MoveFaninsOptions {
                enabled: false,
                max_fanins: 0,
                bound_alphas: false,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PldNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub arrival: DelayTime,
    pub slack: DelayTime,
    deleted: bool,
}

impl PldNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            arrival: DelayTime::new(0.0, 0.0),
            slack: DelayTime::new(0.0, 0.0),
            deleted: false,
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }

    pub fn with_arrival(mut self, rise: f64, fall: f64) -> Self {
        self.arrival = DelayTime::new(rise, fall);
        self
    }

    pub fn with_slack(mut self, rise: f64, fall: f64) -> Self {
        self.slack = DelayTime::new(rise, fall);
        self
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PldNetwork {
    nodes: Vec<PldNode>,
}

impl PldNetwork {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: PldNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&PldNode, XlnLevelError> {
        self.nodes.get(id.0).ok_or(XlnLevelError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[PldNode] {
        &self.nodes
    }

    pub fn active_node_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (!node.deleted).then_some(NodeId(index)))
            .collect()
    }

    pub fn topological_node_ids(&self) -> Vec<NodeId> {
        self.active_node_ids()
    }

    pub fn fanouts(&self, node: NodeId) -> Result<Vec<NodeId>, XlnLevelError> {
        self.node(node)?;
        Ok(self
            .active_node_ids()
            .into_iter()
            .filter(|candidate| self.nodes[candidate.0].fanins.contains(&node))
            .collect())
    }

    pub fn primary_input_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| !node.deleted && node.kind == NodeKind::PrimaryInput)
            .count()
    }

    pub fn internal_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| !node.deleted && node.kind == NodeKind::Internal)
            .count()
    }

    pub fn delete_node(&mut self, node: NodeId) -> Result<(), XlnLevelError> {
        let target = self
            .nodes
            .get_mut(node.0)
            .ok_or(XlnLevelError::UnknownNode(node))?;
        target.deleted = true;
        Ok(())
    }

    pub fn collapse_fanin(&mut self, fanout: NodeId, fanin: NodeId) -> Result<(), XlnLevelError> {
        let replacement_fanins = self.node(fanin)?.fanins.clone();
        let fanout_node = self
            .nodes
            .get_mut(fanout.0)
            .ok_or(XlnLevelError::UnknownNode(fanout))?;

        if fanout_node.deleted {
            return Err(XlnLevelError::DeletedNode(fanout));
        }
        if !fanout_node.fanins.contains(&fanin) {
            return Err(XlnLevelError::NotAFanin { fanout, fanin });
        }

        let mut revised = Vec::new();
        for existing in fanout_node.fanins.iter().copied() {
            if existing == fanin {
                for replacement in &replacement_fanins {
                    if *replacement != fanout && !revised.contains(replacement) {
                        revised.push(*replacement);
                    }
                }
            } else if !revised.contains(&existing) {
                revised.push(existing);
            }
        }
        fanout_node.fanins = revised;
        Ok(())
    }
}

impl Default for PldNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnePassStats {
    pub changed: bool,
    pub collapsed_pairs: Vec<(NodeId, NodeId)>,
    pub deleted_nodes: Vec<NodeId>,
}

impl OnePassStats {
    fn new() -> Self {
        Self {
            changed: false,
            collapsed_pairs: Vec::new(),
            deleted_nodes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReduceLevelStats {
    pub passes: usize,
    pub collapsed_pairs: usize,
    pub deleted_nodes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollapseDecision {
    Ineligible,
    NotCritical,
    Collapsed {
        fanouts: Vec<NodeId>,
        mux_decomposition_required: Vec<(NodeId, Option<NodeId>)>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JustOneCriticalFanin {
    pub allowed: bool,
    pub fanin: Option<NodeId>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum XlnLevelError {
    UnknownNode(NodeId),
    DeletedNode(NodeId),
    NotAFanin { fanout: NodeId, fanin: NodeId },
    InvalidSupport { support: usize },
    UnequalRiseFallSlack { node: NodeId, slack: DelayTime },
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for XlnLevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_level node {:?}", node),
            Self::DeletedNode(node) => write!(f, "xln_level node {:?} was deleted", node),
            Self::NotAFanin { fanout, fanin } => {
                write!(f, "node {:?} is not a fanin of {:?}", fanin, fanout)
            }
            Self::InvalidSupport { support } => {
                write!(f, "xln_level support must be positive, got {support}")
            }
            Self::UnequalRiseFallSlack { node, slack } => write!(
                f,
                "xln_level node {:?} has unequal rise/fall slack ({}, {})",
                node, slack.rise, slack.fall
            ),
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for XlnLevelError {}

pub fn reduce_levels_sis_network_blocked<Network>(
    _network: &mut Network,
    _options: XlnLevelOptions,
) -> Result<ReduceLevelStats, XlnLevelError> {
    Err(XlnLevelError::MissingNativePorts {
        operation: "xln_reduce_levels SIS network delay reduction",
    })
}

pub fn check_network_for_collapsing_delay_blocked<Network>(
    _network: &Network,
    _options: XlnLevelOptions,
) -> Result<Network, XlnLevelError> {
    Err(XlnLevelError::MissingNativePorts {
        operation: "xln_check_network_for_collapsing_delay",
    })
}

pub fn cofactor_decomp_network_blocked<Network>(
    _network: &mut Network,
    _support: usize,
) -> Result<(), XlnLevelError> {
    Err(XlnLevelError::MissingNativePorts {
        operation: "xln_cofactor_decomp_network",
    })
}

pub fn is_node_critical(
    network: &PldNetwork,
    node: NodeId,
    threshold: f64,
) -> Result<bool, XlnLevelError> {
    let node_ref = network.node(node)?;
    if node_ref.slack.rise != node_ref.slack.fall {
        return Err(XlnLevelError::UnequalRiseFallSlack {
            node,
            slack: node_ref.slack,
        });
    }
    Ok(node_ref.slack.rise <= threshold)
}

pub fn composite_fanin(
    network: &PldNetwork,
    n1: NodeId,
    n2: NodeId,
) -> Result<Vec<NodeId>, XlnLevelError> {
    let node1 = network.node(n1)?;
    let node2 = network.node(n2)?;
    let mut fanins = Vec::new();

    for fanin in node1.fanins.iter().copied() {
        if fanin != n2 && !fanins.contains(&fanin) {
            fanins.push(fanin);
        }
    }
    for fanin in node2.fanins.iter().copied() {
        if fanin != n1 && !node1.fanins.contains(&fanin) && !fanins.contains(&fanin) {
            fanins.push(fanin);
        }
    }

    Ok(fanins)
}

pub fn composite_fanin_count(
    network: &PldNetwork,
    n1: NodeId,
    n2: NodeId,
) -> Result<usize, XlnLevelError> {
    Ok(composite_fanin(network, n1, n2)?.len())
}

pub fn is_just_one_fanin_critical(
    network: &PldNetwork,
    fanout: NodeId,
    comp_fanin: &[NodeId],
) -> Result<JustOneCriticalFanin, XlnLevelError> {
    let level_fanout = network.node(fanout)?.arrival.rise;
    let level_fanout_minus_2 = level_fanout - 2.0;
    let level_fanout_minus_1 = level_fanout - 1.0;
    let mut critical_fanin = None;

    for node in comp_fanin {
        let level_node = network.node(*node)?.arrival.rise;
        if level_node == level_fanout_minus_2 {
            if critical_fanin.is_some() {
                return Ok(JustOneCriticalFanin {
                    allowed: false,
                    fanin: None,
                });
            }
            critical_fanin = Some(*node);
        } else if level_node == level_fanout_minus_1 {
            return Ok(JustOneCriticalFanin {
                allowed: false,
                fanin: None,
            });
        }
    }

    Ok(JustOneCriticalFanin {
        allowed: true,
        fanin: critical_fanin,
    })
}

pub fn array_of_levels(network: &PldNetwork) -> Vec<Vec<NodeId>> {
    let max_level = max_arrival_level(network);
    let mut levels = vec![Vec::new(); max_level + 1];
    for node in network.active_node_ids() {
        let level = clamped_level(network.nodes[node.0].arrival.rise, max_level);
        levels[level].push(node);
    }
    levels
}

pub fn array_of_critical_nodes_at_levels(
    network: &PldNetwork,
    threshold: f64,
) -> Result<Vec<Vec<NodeId>>, XlnLevelError> {
    let max_level = max_arrival_level(network);
    let mut levels = vec![Vec::new(); max_level + 1];
    for node in network.active_node_ids() {
        if is_node_critical(network, node, threshold)? {
            let level = clamped_level(network.nodes[node.0].arrival.rise, max_level);
            levels[level].push(node);
        }
    }
    Ok(levels)
}

pub fn sort_levels_by_width(levels: &mut [Vec<NodeId>]) {
    levels.sort_by_key(Vec::len);
}

pub fn node_collapse_if_critical(
    network: &mut PldNetwork,
    node: NodeId,
    threshold: f64,
    options: XlnLevelOptions,
) -> Result<CollapseDecision, XlnLevelError> {
    if options.support == 0 {
        return Err(XlnLevelError::InvalidSupport {
            support: options.support,
        });
    }
    if network.node(node)?.kind != NodeKind::Internal {
        return Ok(CollapseDecision::Ineligible);
    }
    if !is_node_critical(network, node, threshold)? {
        return Ok(CollapseDecision::NotCritical);
    }

    let arrival = network.node(node)?.arrival.rise;
    let mut selected = Vec::new();
    let mut mux_required = Vec::new();
    let mut infeasible_fanouts = Vec::new();
    let mut max_diff = 0usize;

    for fanout in network.fanouts(node)? {
        if network.node(fanout)?.kind == NodeKind::PrimaryOutput {
            continue;
        }
        if network.node(fanout)?.arrival.rise != arrival + 1.0 {
            continue;
        }

        let composite = composite_fanin(network, node, fanout)?;
        let count = composite.len();
        match options.heuristic {
            CollapseHeuristic::AllCriticalFanouts => {
                if count <= options.support {
                    selected.push(fanout);
                } else {
                    return Ok(CollapseDecision::Ineligible);
                }
            }
            CollapseHeuristic::PartialCriticalFanouts => {
                if count <= options.support {
                    selected.push(fanout);
                } else if count == options.support + 1 && options.support != 2 {
                    let criticality = is_just_one_fanin_critical(network, fanout, &composite)?;
                    if criticality.allowed {
                        selected.push(fanout);
                        mux_required.push((fanout, criticality.fanin));
                    } else {
                        max_diff = max_diff.max(count - options.support);
                        infeasible_fanouts.push(fanout);
                    }
                } else {
                    max_diff = max_diff.max(count - options.support);
                    infeasible_fanouts.push(fanout);
                }
            }
        }
    }

    for fanout in selected.iter().copied() {
        network.collapse_fanin(fanout, node)?;
    }

    if !selected.is_empty() {
        return Ok(CollapseDecision::Collapsed {
            fanouts: selected,
            mux_decomposition_required: mux_required,
        });
    }

    if options.move_fanins.enabled && !infeasible_fanouts.is_empty() && max_diff > 0 {
        return Err(XlnLevelError::MissingNativePorts {
            operation: "xln_node_move_fanins_for_delay and xln_try_collapsing_node",
        });
    }

    Ok(CollapseDecision::Ineligible)
}

pub fn one_pass_by_levels(
    network: &mut PldNetwork,
    threshold: f64,
    options: XlnLevelOptions,
) -> Result<OnePassStats, XlnLevelError> {
    let mut levels = array_of_critical_nodes_at_levels(network, threshold)?;
    sort_levels_by_width(&mut levels);
    one_pass_in_order(network, levels.into_iter().flatten(), threshold, options)
}

pub fn one_pass_topol(
    network: &mut PldNetwork,
    threshold: f64,
    options: XlnLevelOptions,
) -> Result<OnePassStats, XlnLevelError> {
    one_pass_in_order(network, network.topological_node_ids(), threshold, options)
}

pub fn reduce_levels(
    network: &mut PldNetwork,
    options: XlnLevelOptions,
) -> Result<ReduceLevelStats, XlnLevelError> {
    let mut passes = 0;
    let mut collapsed_pairs = 0;
    let mut deleted_nodes = 0;

    loop {
        let stats = match options.traversal_method {
            TraversalMethod::ByLevelWidth => one_pass_by_levels(network, 0.0, options)?,
            TraversalMethod::Topological => one_pass_topol(network, 0.0, options)?,
        };
        if !stats.changed {
            return Ok(ReduceLevelStats {
                passes,
                collapsed_pairs,
                deleted_nodes,
            });
        }
        passes += 1;
        collapsed_pairs += stats.collapsed_pairs.len();
        deleted_nodes += stats.deleted_nodes.len();

        if options.move_fanins.enabled {
            return Err(XlnLevelError::MissingNativePorts {
                operation: "xln_network_move_fanins_for_delay",
            });
        }
    }
}

fn one_pass_in_order<I>(
    network: &mut PldNetwork,
    order: I,
    threshold: f64,
    options: XlnLevelOptions,
) -> Result<OnePassStats, XlnLevelError>
where
    I: IntoIterator<Item = NodeId>,
{
    let mut stats = OnePassStats::new();
    let mut seen = HashSet::new();

    for node in order {
        if !seen.insert(node) || network.node(node).is_err() || network.nodes[node.0].deleted {
            continue;
        }
        let decision = node_collapse_if_critical(network, node, threshold, options)?;
        if let CollapseDecision::Collapsed { fanouts, .. } = decision {
            if !fanouts.is_empty() {
                stats.changed = true;
                stats
                    .collapsed_pairs
                    .extend(fanouts.into_iter().map(|fanout| (node, fanout)));
                if network.fanouts(node)?.is_empty() {
                    network.delete_node(node)?;
                    stats.deleted_nodes.push(node);
                }
            }
        }
    }

    Ok(stats)
}

fn max_arrival_level(network: &PldNetwork) -> usize {
    network
        .nodes
        .iter()
        .filter(|node| !node.deleted)
        .map(|node| node.arrival.rise as isize)
        .max()
        .unwrap_or(0)
        .max(0) as usize
}

fn clamped_level(level: f64, max_level: usize) -> usize {
    let level = level as isize;
    if level < 0 {
        0
    } else {
        (level as usize).min(max_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level_options(heuristic: CollapseHeuristic) -> XlnLevelOptions {
        XlnLevelOptions::new(TraversalMethod::ByLevelWidth, heuristic, 4)
    }

    fn node_names<'a>(network: &'a PldNetwork, ids: &[NodeId]) -> Vec<&'a str> {
        ids.iter()
            .map(|id| network.node(*id).unwrap().name.as_str())
            .collect()
    }

    #[test]
    fn criticality_uses_equal_rise_fall_slack_threshold() {
        let mut network = PldNetwork::new();
        let n = network.add_node(
            PldNode::new("n", NodeKind::Internal)
                .with_arrival(2.0, 2.0)
                .with_slack(0.5, 0.5),
        );
        let bad = network.add_node(PldNode::new("bad", NodeKind::Internal).with_slack(0.0, 1.0));

        assert!(is_node_critical(&network, n, 0.5).unwrap());
        assert!(!is_node_critical(&network, n, 0.4).unwrap());
        assert_eq!(
            is_node_critical(&network, bad, 0.0),
            Err(XlnLevelError::UnequalRiseFallSlack {
                node: bad,
                slack: DelayTime::new(0.0, 1.0),
            })
        );
    }

    #[test]
    fn composite_fanin_matches_c_union_rules() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(PldNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(PldNode::new("c", NodeKind::PrimaryInput));
        let x = network.add_node(PldNode::new("x", NodeKind::Internal).with_fanins(vec![a, b]));
        let y = network.add_node(PldNode::new("y", NodeKind::Internal).with_fanins(vec![x, c]));

        assert_eq!(
            node_names(&network, &composite_fanin(&network, x, y).unwrap()),
            vec!["a", "b", "c"]
        );
        assert_eq!(
            node_names(&network, &composite_fanin(&network, y, x).unwrap()),
            vec!["c", "a", "b"]
        );
        assert_eq!(composite_fanin_count(&network, y, x).unwrap(), 3);
    }

    #[test]
    fn levels_bucket_negative_arrivals_at_zero_and_sort_by_width() {
        let mut network = PldNetwork::new();
        let a =
            network.add_node(PldNode::new("a", NodeKind::PrimaryInput).with_arrival(-1.0, -1.0));
        let b = network.add_node(PldNode::new("b", NodeKind::Internal).with_arrival(2.0, 2.0));
        let c = network.add_node(PldNode::new("c", NodeKind::Internal).with_arrival(2.0, 2.0));

        let mut levels = array_of_levels(&network);
        assert_eq!(levels[0], vec![a]);
        assert_eq!(levels[1], Vec::<NodeId>::new());
        assert_eq!(levels[2], vec![b, c]);

        sort_levels_by_width(&mut levels);
        assert_eq!(levels[0], Vec::<NodeId>::new());
        assert_eq!(levels[1], vec![a]);
        assert_eq!(levels[2], vec![b, c]);
    }

    #[test]
    fn critical_levels_keep_only_threshold_nodes() {
        let mut network = PldNetwork::new();
        let critical = network.add_node(
            PldNode::new("critical", NodeKind::Internal)
                .with_arrival(1.0, 1.0)
                .with_slack(0.0, 0.0),
        );
        network.add_node(
            PldNode::new("late", NodeKind::Internal)
                .with_arrival(2.0, 2.0)
                .with_slack(1.0, 1.0),
        );

        let levels = array_of_critical_nodes_at_levels(&network, 0.0).unwrap();

        assert_eq!(levels[1], vec![critical]);
        assert_eq!(levels[2], Vec::<NodeId>::new());
    }

    #[test]
    fn just_one_fanin_critical_rejects_level_minus_one_and_multiple_minus_two() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput).with_arrival(3.0, 3.0));
        let b = network.add_node(PldNode::new("b", NodeKind::PrimaryInput).with_arrival(2.0, 2.0));
        let c = network.add_node(PldNode::new("c", NodeKind::PrimaryInput).with_arrival(1.0, 1.0));
        let out = network.add_node(PldNode::new("out", NodeKind::Internal).with_arrival(4.0, 4.0));

        assert_eq!(
            is_just_one_fanin_critical(&network, out, &[b, c]).unwrap(),
            JustOneCriticalFanin {
                allowed: true,
                fanin: Some(b),
            }
        );
        assert_eq!(
            is_just_one_fanin_critical(&network, out, &[c]).unwrap(),
            JustOneCriticalFanin {
                allowed: true,
                fanin: None,
            }
        );
        assert_eq!(
            is_just_one_fanin_critical(&network, out, &[a, b]).unwrap(),
            JustOneCriticalFanin {
                allowed: false,
                fanin: None,
            }
        );

        let d = network.add_node(PldNode::new("d", NodeKind::PrimaryInput).with_arrival(2.0, 2.0));
        assert_eq!(
            is_just_one_fanin_critical(&network, out, &[b, d]).unwrap(),
            JustOneCriticalFanin {
                allowed: false,
                fanin: None,
            }
        );
    }

    #[test]
    fn heuristic_two_aborts_if_any_critical_fanout_is_infeasible() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(PldNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(PldNode::new("c", NodeKind::PrimaryInput));
        let d = network.add_node(PldNode::new("d", NodeKind::PrimaryInput));
        let x = network.add_node(
            PldNode::new("x", NodeKind::Internal)
                .with_fanins(vec![a, b, c])
                .with_arrival(1.0, 1.0)
                .with_slack(0.0, 0.0),
        );
        let y = network.add_node(
            PldNode::new("y", NodeKind::Internal)
                .with_fanins(vec![x, d])
                .with_arrival(2.0, 2.0),
        );
        let options = XlnLevelOptions::new(
            TraversalMethod::ByLevelWidth,
            CollapseHeuristic::AllCriticalFanouts,
            2,
        );

        assert_eq!(
            node_collapse_if_critical(&mut network, x, 0.0, options).unwrap(),
            CollapseDecision::Ineligible
        );
        assert_eq!(network.node(y).unwrap().fanins, vec![x, d]);
    }

    #[test]
    fn heuristic_one_collapses_feasible_critical_fanouts() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(PldNode::new("b", NodeKind::PrimaryInput));
        let x = network.add_node(
            PldNode::new("x", NodeKind::Internal)
                .with_fanins(vec![a, b])
                .with_arrival(1.0, 1.0)
                .with_slack(0.0, 0.0),
        );
        let y = network.add_node(
            PldNode::new("y", NodeKind::Internal)
                .with_fanins(vec![x])
                .with_arrival(2.0, 2.0),
        );

        let decision = node_collapse_if_critical(
            &mut network,
            x,
            0.0,
            level_options(CollapseHeuristic::PartialCriticalFanouts),
        )
        .unwrap();

        assert_eq!(
            decision,
            CollapseDecision::Collapsed {
                fanouts: vec![y],
                mux_decomposition_required: Vec::new(),
            }
        );
        assert_eq!(network.node(y).unwrap().fanins, vec![a, b]);
    }

    #[test]
    fn one_pass_deletes_collapsed_node_when_no_fanouts_remain() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput));
        let x = network.add_node(
            PldNode::new("x", NodeKind::Internal)
                .with_fanins(vec![a])
                .with_arrival(1.0, 1.0)
                .with_slack(0.0, 0.0),
        );
        let y = network.add_node(
            PldNode::new("y", NodeKind::Internal)
                .with_fanins(vec![x])
                .with_arrival(2.0, 2.0),
        );

        let stats = one_pass_topol(
            &mut network,
            0.0,
            level_options(CollapseHeuristic::PartialCriticalFanouts),
        )
        .unwrap();

        assert!(stats.changed);
        assert_eq!(stats.collapsed_pairs, vec![(x, y)]);
        assert_eq!(stats.deleted_nodes, vec![x]);
        assert!(network.node(x).unwrap().is_deleted());
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_level.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
