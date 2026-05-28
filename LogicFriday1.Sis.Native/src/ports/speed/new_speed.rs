//! Native Rust model for feasible behavior in `sis/speed/new_speed.c`.
//!
//! The original file is the recursive new-speed optimizer. Its top-level flow
//! still depends on SIS `network_t` mutation, delay tracing, local transform
//! callbacks, and cut selection from `new_wght_util.c`. This module ports the
//! independent decision rules and graph algorithms into owned Rust data, and
//! reports explicit missing dependencies for the full SIS-bound optimizer.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const NSP_EPSILON: f64 = 1.0e-6;
pub const NSP_INPUT_SEPARATOR: char = '#';
pub const POS_LARGE: f64 = 10_000.0;
pub const NEG_LARGE: f64 = -10_000.0;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub const fn neg_large() -> Self {
        Self {
            rise: NEG_LARGE,
            fall: NEG_LARGE,
        }
    }

    pub fn min_edge(self) -> f64 {
        self.rise.min(self.fall)
    }

    pub fn max_edge(self) -> f64 {
        self.rise.max(self.fall)
    }

    pub fn scale(self, load: f64) -> Self {
        Self {
            rise: self.rise * load,
            fall: self.fall * load,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayPin {
    pub drive: DelayTime,
    pub load: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    Library,
    UnitFanout,
    Mapped,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedRegion {
    AlongCriticalPath,
    TransitiveFanin,
    Compromise,
    OnlyTree,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformType {
    CriticalPath,
    Fanout,
    Dual,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformWeight {
    pub improvement: f64,
    pub area_cost: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewSpeedOptions {
    pub dist: i32,
    pub max_recur: i32,
    pub region: SpeedRegion,
    pub model: DelayModel,
    pub new_mode: bool,
    pub threshold: f64,
}

impl Default for NewSpeedOptions {
    fn default() -> Self {
        Self {
            dist: 3,
            max_recur: 1,
            region: SpeedRegion::AlongCriticalPath,
            model: DelayModel::Unit,
            new_mode: true,
            threshold: 0.5,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewSpeedNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub fanouts: Vec<NodeId>,
    pub critical: bool,
    pub arrival: DelayTime,
    pub slack: DelayTime,
    pub pins: Vec<DelayPin>,
}

impl NewSpeedNode {
    pub fn internal(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            kind: NodeKind::Internal,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            critical: false,
            arrival: DelayTime::new(0.0, 0.0),
            slack: DelayTime::new(0.0, 0.0),
            pins: Vec::new(),
        }
    }

    pub fn primary_input(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::PrimaryInput,
            ..Self::internal(id, name)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NewSpeedGraph {
    nodes: HashMap<NodeId, NewSpeedNode>,
}

impl NewSpeedGraph {
    pub fn new(nodes: Vec<NewSpeedNode>) -> Self {
        Self {
            nodes: nodes.into_iter().map(|node| (node.id, node)).collect(),
        }
    }

    pub fn node(&self, id: NodeId) -> Option<&NewSpeedNode> {
        self.nodes.get(&id)
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NewSpeedError {
    SisGraphDependency {
        operation: &'static str,
        source: &'static str,
    },
    UnknownNode(NodeId),
    MissingWeight(NodeId),
    MismatchedDeltaInputs {
        original_slacks: usize,
        updated_slacks: usize,
    },
}

impl fmt::Display for NewSpeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SisGraphDependency { operation, source } => write!(
                f,
                "{operation} requires SIS graph optimization from {source}"
            ),
            Self::UnknownNode(node) => write!(f, "unknown new_speed node {:?}", node),
            Self::MissingWeight(node) => write!(f, "missing transform weight for {:?}", node),
            Self::MismatchedDeltaInputs {
                original_slacks,
                updated_slacks,
            } => write!(
                f,
                "delta slack arrays differ in length: {original_slacks} != {updated_slacks}"
            ),
        }
    }
}

impl Error for NewSpeedError {}

#[derive(Clone, Debug, PartialEq)]
pub struct ExpandedSelection {
    pub nodes: Vec<NodeId>,
    pub added_area_savers: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollapseRegion {
    pub nodes: Vec<NodeId>,
    pub distance_by_node: HashMap<NodeId, i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeltaComputation {
    pub deltas: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecursionConstraint {
    pub target: NodeId,
    pub rise: f64,
    pub fall: f64,
}

pub fn new_speed_network_bound<Network>(
    _network: &mut Network,
    _options: &NewSpeedOptions,
) -> Result<bool, NewSpeedError> {
    Err(NewSpeedError::SisGraphDependency {
        operation: "new_speed",
        source: "LogicSynthesis/sis/speed/new_speed.c:30",
    })
}

pub fn adaptive_initial_threshold(
    saved_threshold: f64,
    new_mode: bool,
    first_slack_difference: Option<f64>,
) -> f64 {
    if new_mode {
        first_slack_difference
            .map(|slack_diff| 2.0 * slack_diff)
            .unwrap_or(saved_threshold)
    } else {
        saved_threshold
    }
}

pub fn should_recur(level: i32, max_recur: i32) -> bool {
    level + 1 < max_recur
}

pub fn recursion_limit_hit(level: i32, max_recur: i32) -> bool {
    level >= max_recur
}

pub fn delay_improved(transform_type: TransformType, old: DelayTime, new: DelayTime) -> bool {
    if transform_type == TransformType::CriticalPath {
        new.rise < old.rise - NSP_EPSILON && new.fall < old.fall - NSP_EPSILON
    } else {
        new.rise > old.rise + NSP_EPSILON && new.fall > old.fall + NSP_EPSILON
    }
}

pub fn expand_selection(
    mapped: bool,
    mincut: &[NodeId],
    weights: &HashMap<NodeId, TransformWeight>,
) -> ExpandedSelection {
    let mut selected = mincut.to_vec();
    let mut selected_set = selected.iter().copied().collect::<HashSet<_>>();
    let original_len = selected.len();

    if !mapped {
        let mut candidates = weights
            .iter()
            .filter_map(|(node, weight)| {
                (!selected_set.contains(node)
                    && weight.improvement > NSP_EPSILON
                    && weight.area_cost < 0.0)
                    .then_some(*node)
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|node| node.0);

        for node in candidates {
            selected.push(node);
            selected_set.insert(node);
        }
    }

    ExpandedSelection {
        added_area_savers: selected.len() - original_len,
        nodes: selected,
    }
}

pub fn collapse_bfs(
    graph: &NewSpeedGraph,
    root: NodeId,
    options: &NewSpeedOptions,
) -> Result<CollapseRegion, NewSpeedError> {
    let root_node = graph.node(root).ok_or(NewSpeedError::UnknownNode(root))?;
    if root_node.kind != NodeKind::Internal {
        return Ok(CollapseRegion {
            nodes: Vec::new(),
            distance_by_node: HashMap::new(),
        });
    }

    let mut nodes = vec![root];
    let mut distance_by_node = HashMap::from([(
        root,
        if options.region == SpeedRegion::OnlyTree {
            options.dist
        } else {
            options.dist + 1
        },
    )]);
    let mut first = 0;

    while first < nodes.len() {
        let last = nodes.len();
        for index in first..last {
            let temp = nodes[index];
            let cur_dist = distance_by_node[&temp];
            if options.region != SpeedRegion::OnlyTree && cur_dist == 1 {
                continue;
            }
            if options.region == SpeedRegion::OnlyTree && cur_dist < 0 {
                continue;
            }

            for fanin in graph
                .node(temp)
                .ok_or(NewSpeedError::UnknownNode(temp))?
                .fanins
                .clone()
            {
                let Some(fanin_node) = graph.node(fanin) else {
                    return Err(NewSpeedError::UnknownNode(fanin));
                };
                if fanin_node.kind != NodeKind::Internal || distance_by_node.contains_key(&fanin) {
                    continue;
                }

                add_collapse_fanin(
                    graph,
                    options.region,
                    fanin,
                    cur_dist,
                    &mut nodes,
                    &mut distance_by_node,
                )?;
            }
        }
        first = last;
    }

    if options.region == SpeedRegion::Compromise {
        add_compromise_nodes(graph, &mut nodes, &mut distance_by_node)?;
    }

    Ok(CollapseRegion {
        nodes,
        distance_by_node,
    })
}

pub fn filter_cutset(
    graph: &NewSpeedGraph,
    mincut: &[NodeId],
    options: &NewSpeedOptions,
    weights: &HashMap<NodeId, TransformWeight>,
) -> Result<Vec<NodeId>, NewSpeedError> {
    let mut deleted = HashSet::new();

    for (root_index, root) in mincut.iter().copied().enumerate().rev() {
        let root_weight = weights
            .get(&root)
            .ok_or(NewSpeedError::MissingWeight(root))?;
        if root_weight.improvement < NSP_EPSILON {
            deleted.insert(root);
            continue;
        }

        let region = collapse_bfs(graph, root, options)?;
        let region_set = region.nodes.iter().copied().collect::<HashSet<_>>();

        for (node_index, node) in mincut.iter().copied().enumerate().rev() {
            if root_index == node_index || !region_set.contains(&node) {
                continue;
            }
            weights
                .get(&node)
                .ok_or(NewSpeedError::MissingWeight(node))?;

            if !path_from_node_to_root_has_external_fanout(graph, node, root, &region_set)? {
                deleted.insert(node);
            }
        }
    }

    Ok(mincut
        .iter()
        .rev()
        .copied()
        .filter(|node| !deleted.contains(node))
        .collect())
}

pub fn critical_path_delta(old: DelayTime, new: DelayTime) -> f64 {
    (new.rise - old.rise).min(new.fall - old.fall)
}

pub fn fanout_delta(old: DelayTime, new: DelayTime) -> f64 {
    (old.rise - new.rise).min(old.fall - new.fall)
}

pub fn delta_from_slacks(
    first_delta: f64,
    original_slacks: &[f64],
    updated_slacks: &[f64],
) -> Result<DeltaComputation, NewSpeedError> {
    if original_slacks.len() != updated_slacks.len() {
        return Err(NewSpeedError::MismatchedDeltaInputs {
            original_slacks: original_slacks.len(),
            updated_slacks: updated_slacks.len(),
        });
    }

    let mut deltas = Vec::with_capacity(original_slacks.len() + 1);
    deltas.push(first_delta);
    deltas.extend(
        original_slacks
            .iter()
            .zip(updated_slacks)
            .map(|(old_slack, new_slack)| old_slack - new_slack),
    );
    Ok(DeltaComputation { deltas })
}

pub fn load_adjusted_min_slack(slack: DelayTime, input_drive: DelayTime, load: f64) -> f64 {
    let load_delay = input_drive.scale(load);
    (slack.rise - load_delay.rise).min(slack.fall - load_delay.fall)
}

pub fn apply_critical_path_constraints(
    targets: &[NodeId],
    arrivals: &[DelayTime],
    deltas_after_output: &[f64],
) -> Vec<RecursionConstraint> {
    targets
        .iter()
        .zip(arrivals)
        .zip(deltas_after_output)
        .map(|((target, arrival), delta)| {
            let delta = delta.max(0.0);
            RecursionConstraint {
                target: *target,
                rise: arrival.rise - delta,
                fall: arrival.fall - delta,
            }
        })
        .collect()
}

pub fn apply_fanout_constraints(
    targets: &[NodeId],
    required_times: &[DelayTime],
    deltas_after_input: &[f64],
) -> Vec<RecursionConstraint> {
    targets
        .iter()
        .zip(required_times)
        .zip(deltas_after_input)
        .map(|((target, required), delta)| {
            let delta = delta.max(0.0);
            RecursionConstraint {
                target: *target,
                rise: required.rise + delta,
                fall: required.fall + delta,
            }
        })
        .collect()
}

pub fn input_drive(node: &NewSpeedNode) -> DelayTime {
    if node.kind == NodeKind::PrimaryInput {
        return node
            .pins
            .first()
            .map(|pin| pin.drive)
            .unwrap_or_else(DelayTime::neg_large);
    }

    node.pins
        .iter()
        .fold(DelayTime::neg_large(), |drive, pin| DelayTime {
            rise: drive.rise.max(pin.drive.rise),
            fall: drive.fall.max(pin.drive.fall),
        })
}

pub fn synthetic_input_name(original_name: &str, fanin_index: usize) -> String {
    format!("{original_name}{NSP_INPUT_SEPARATOR}{fanin_index}")
}

fn add_collapse_fanin(
    graph: &NewSpeedGraph,
    region: SpeedRegion,
    fanin: NodeId,
    cur_dist: i32,
    nodes: &mut Vec<NodeId>,
    distance_by_node: &mut HashMap<NodeId, i32>,
) -> Result<(), NewSpeedError> {
    let fanin_node = graph.node(fanin).ok_or(NewSpeedError::UnknownNode(fanin))?;
    let mut should_add = true;
    let mut new_dist = cur_dist;

    if region != SpeedRegion::TransitiveFanin {
        if !fanin_node.critical {
            if region != SpeedRegion::OnlyTree || fanin_node.fanouts.len() > 1 {
                should_add = false;
            } else {
                new_dist = -1;
            }
        } else if region == SpeedRegion::OnlyTree
            && cur_dist <= 0
            && fanin_node.fanouts.len() > 1
            && fanin_node.fanins.len() > 1
        {
            should_add = false;
        }
    }

    if !should_add {
        return Ok(());
    }

    if region != SpeedRegion::OnlyTree {
        distance_by_node.insert(fanin, new_dist - 1);
        nodes.push(fanin);
        return Ok(());
    }

    if fanin_node.fanouts.len() == 1 {
        distance_by_node.insert(fanin, new_dist);
        nodes.push(fanin);
        return Ok(());
    }

    if cur_dist < 0 {
        return Ok(());
    }

    let mut cursor = fanin;
    let mut need_to_add = true;
    while graph
        .node(cursor)
        .ok_or(NewSpeedError::UnknownNode(cursor))?
        .fanins
        .len()
        == 1
    {
        if distance_by_node.contains_key(&cursor) {
            need_to_add = false;
            break;
        }
        distance_by_node.insert(cursor, new_dist - 1);
        nodes.push(cursor);
        cursor = graph.node(cursor).unwrap().fanins[0];
    }

    let cursor_node = graph
        .node(cursor)
        .ok_or(NewSpeedError::UnknownNode(cursor))?;
    if cursor_node.kind == NodeKind::PrimaryInput
        || distance_by_node.contains_key(&cursor)
        || cur_dist == 0
    {
        need_to_add = false;
    }

    if need_to_add {
        distance_by_node.insert(cursor, new_dist - 1);
        nodes.push(cursor);
    }
    Ok(())
}

fn add_compromise_nodes(
    graph: &NewSpeedGraph,
    nodes: &mut Vec<NodeId>,
    distance_by_node: &mut HashMap<NodeId, i32>,
) -> Result<(), NewSpeedError> {
    let mut min_arrival = POS_LARGE;
    for node in nodes.iter().copied() {
        for fanin in &graph
            .node(node)
            .ok_or(NewSpeedError::UnknownNode(node))?
            .fanins
        {
            if !distance_by_node.contains_key(fanin) {
                let arrival = graph
                    .node(*fanin)
                    .ok_or(NewSpeedError::UnknownNode(*fanin))?
                    .arrival;
                min_arrival = min_arrival.min(arrival.min_edge());
            }
        }
    }

    let mut first = 0;
    while first < nodes.len() {
        let last = nodes.len();
        for index in first..last {
            let temp = nodes[index];
            let cur_dist = distance_by_node[&temp];
            let fanins = graph
                .node(temp)
                .ok_or(NewSpeedError::UnknownNode(temp))?
                .fanins
                .clone();
            for fanin in fanins {
                if distance_by_node.contains_key(&fanin) || cur_dist <= 1 {
                    continue;
                }

                let should_add = graph
                    .node(fanin)
                    .ok_or(NewSpeedError::UnknownNode(fanin))?
                    .fanins
                    .iter()
                    .try_fold(false, |found, new_fanin| {
                        let arrival = graph
                            .node(*new_fanin)
                            .ok_or(NewSpeedError::UnknownNode(*new_fanin))?
                            .arrival;
                        Ok::<_, NewSpeedError>(
                            found || (arrival.rise >= min_arrival && arrival.fall >= min_arrival),
                        )
                    })?;

                if should_add {
                    distance_by_node.insert(fanin, cur_dist - 1);
                    nodes.push(fanin);
                }
            }
        }
        first = last;
    }
    Ok(())
}

fn path_from_node_to_root_has_external_fanout(
    graph: &NewSpeedGraph,
    node: NodeId,
    root: NodeId,
    region_set: &HashSet<NodeId>,
) -> Result<bool, NewSpeedError> {
    let mut queue = VecDeque::from([node]);
    let mut visited = HashSet::new();

    while let Some(temp) = queue.pop_front() {
        if !visited.insert(temp) || temp == root {
            continue;
        }
        for fanout in &graph
            .node(temp)
            .ok_or(NewSpeedError::UnknownNode(temp))?
            .fanouts
        {
            if region_set.contains(fanout) {
                queue.push_back(*fanout);
            } else {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize, fanins: &[usize], fanouts: &[usize], critical: bool) -> NewSpeedNode {
        let mut node = NewSpeedNode::internal(NodeId(id), format!("n{id}"));
        node.fanins = fanins.iter().map(|id| NodeId(*id)).collect();
        node.fanouts = fanouts.iter().map(|id| NodeId(*id)).collect();
        node.critical = critical;
        node
    }

    #[test]
    fn adaptive_threshold_and_recursion_limits_match_c_flow() {
        assert_eq!(adaptive_initial_threshold(0.5, true, Some(0.25)), 0.5);
        assert_eq!(adaptive_initial_threshold(0.7, true, Some(0.25)), 0.5);
        assert_eq!(adaptive_initial_threshold(0.7, true, None), 0.7);
        assert_eq!(adaptive_initial_threshold(0.7, false, Some(0.25)), 0.7);
        assert!(should_recur(0, 2));
        assert!(!should_recur(1, 2));
        assert!(recursion_limit_hit(2, 2));
    }

    #[test]
    fn delay_improvement_uses_arrival_for_clp_and_required_for_fanout() {
        let old = DelayTime::new(10.0, 11.0);
        assert!(delay_improved(
            TransformType::CriticalPath,
            old,
            DelayTime::new(9.0, 10.0)
        ));
        assert!(!delay_improved(
            TransformType::CriticalPath,
            old,
            DelayTime::new(9.0, 11.0)
        ));
        assert!(delay_improved(
            TransformType::Fanout,
            old,
            DelayTime::new(11.0, 12.0)
        ));
        assert!(!delay_improved(
            TransformType::Fanout,
            old,
            DelayTime::new(11.0, 10.0)
        ));
    }

    #[test]
    fn expand_selection_adds_unmapped_area_savers_only() {
        let selected = [NodeId(1)];
        let weights = HashMap::from([
            (
                NodeId(1),
                TransformWeight {
                    improvement: 5.0,
                    area_cost: 4.0,
                },
            ),
            (
                NodeId(2),
                TransformWeight {
                    improvement: 0.2,
                    area_cost: -1.0,
                },
            ),
            (
                NodeId(3),
                TransformWeight {
                    improvement: 0.0,
                    area_cost: -2.0,
                },
            ),
        ]);

        assert_eq!(
            expand_selection(false, &selected, &weights),
            ExpandedSelection {
                nodes: vec![NodeId(1), NodeId(2)],
                added_area_savers: 1,
            }
        );
        assert_eq!(
            expand_selection(true, &selected, &weights),
            ExpandedSelection {
                nodes: vec![NodeId(1)],
                added_area_savers: 0,
            }
        );
    }

    #[test]
    fn collapse_bfs_respects_critical_and_transitive_regions() {
        let graph = NewSpeedGraph::new(vec![
            node(0, &[1, 2], &[], true),
            node(1, &[3], &[0], true),
            node(2, &[4], &[0], false),
            node(3, &[], &[1], true),
            node(4, &[], &[2], true),
        ]);

        let critical = collapse_bfs(
            &graph,
            NodeId(0),
            &NewSpeedOptions {
                dist: 2,
                region: SpeedRegion::AlongCriticalPath,
                ..NewSpeedOptions::default()
            },
        )
        .unwrap();
        assert_eq!(critical.nodes, vec![NodeId(0), NodeId(1), NodeId(3)]);

        let transitive = collapse_bfs(
            &graph,
            NodeId(0),
            &NewSpeedOptions {
                dist: 2,
                region: SpeedRegion::TransitiveFanin,
                ..NewSpeedOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            transitive.nodes,
            vec![NodeId(0), NodeId(1), NodeId(2), NodeId(3), NodeId(4)]
        );
    }

    #[test]
    fn collapse_bfs_only_tree_continues_single_fanout_noncritical_tree() {
        let graph = NewSpeedGraph::new(vec![
            node(0, &[1], &[], true),
            node(1, &[2], &[0], false),
            node(2, &[], &[1], false),
        ]);

        let region = collapse_bfs(
            &graph,
            NodeId(0),
            &NewSpeedOptions {
                dist: 1,
                region: SpeedRegion::OnlyTree,
                ..NewSpeedOptions::default()
            },
        )
        .unwrap();

        assert_eq!(region.nodes, vec![NodeId(0), NodeId(1)]);
        assert_eq!(region.distance_by_node[&NodeId(1)], -1);
    }

    #[test]
    fn compromise_region_adds_inputs_with_arrival_inside_range() {
        let mut n2 = node(2, &[4], &[0], false);
        let mut n4 = node(4, &[], &[2], true);
        n4.arrival = DelayTime::new(8.0, 8.5);
        n2.arrival = DelayTime::new(1.0, 1.0);
        let graph = NewSpeedGraph::new(vec![
            node(0, &[1, 2], &[], true),
            node(1, &[3], &[0], true),
            n2,
            node(3, &[], &[1], true),
            n4,
        ]);

        let region = collapse_bfs(
            &graph,
            NodeId(0),
            &NewSpeedOptions {
                dist: 3,
                region: SpeedRegion::Compromise,
                ..NewSpeedOptions::default()
            },
        )
        .unwrap();

        assert!(region.nodes.contains(&NodeId(2)));
    }

    #[test]
    fn filter_cutset_removes_zero_gain_and_nodes_hidden_inside_another_region() {
        let graph = NewSpeedGraph::new(vec![
            node(0, &[1], &[], true),
            node(1, &[2], &[0], true),
            node(2, &[], &[1], true),
            node(3, &[], &[], true),
        ]);
        let weights = HashMap::from([
            (
                NodeId(0),
                TransformWeight {
                    improvement: 1.0,
                    area_cost: 0.0,
                },
            ),
            (
                NodeId(1),
                TransformWeight {
                    improvement: 1.0,
                    area_cost: 0.0,
                },
            ),
            (
                NodeId(3),
                TransformWeight {
                    improvement: 0.0,
                    area_cost: 0.0,
                },
            ),
        ]);

        let filtered = filter_cutset(
            &graph,
            &[NodeId(0), NodeId(1), NodeId(3)],
            &NewSpeedOptions {
                dist: 3,
                region: SpeedRegion::AlongCriticalPath,
                ..NewSpeedOptions::default()
            },
            &weights,
        )
        .unwrap();

        assert_eq!(filtered, vec![NodeId(0)]);
    }

    #[test]
    fn delta_and_constraints_match_c_arithmetic() {
        assert_eq!(
            critical_path_delta(DelayTime::new(5.0, 7.0), DelayTime::new(8.0, 8.5)),
            1.5
        );
        assert_eq!(
            fanout_delta(DelayTime::new(10.0, 9.0), DelayTime::new(8.0, 7.5)),
            1.5
        );
        assert_eq!(
            delta_from_slacks(1.5, &[4.0, 3.0], &[2.5, 3.25])
                .unwrap()
                .deltas,
            vec![1.5, 1.5, -0.25]
        );
        assert_eq!(
            load_adjusted_min_slack(DelayTime::new(5.0, 6.0), DelayTime::new(0.5, 1.0), 2.0),
            4.0
        );

        assert_eq!(
            apply_critical_path_constraints(
                &[NodeId(1), NodeId(2)],
                &[DelayTime::new(10.0, 11.0), DelayTime::new(7.0, 8.0)],
                &[2.0, -1.0],
            ),
            vec![
                RecursionConstraint {
                    target: NodeId(1),
                    rise: 8.0,
                    fall: 9.0,
                },
                RecursionConstraint {
                    target: NodeId(2),
                    rise: 7.0,
                    fall: 8.0,
                },
            ]
        );
        assert_eq!(
            apply_fanout_constraints(&[NodeId(3)], &[DelayTime::new(4.0, 5.0)], &[1.25],),
            vec![RecursionConstraint {
                target: NodeId(3),
                rise: 5.25,
                fall: 6.25,
            }]
        );
    }

    #[test]
    fn input_drive_uses_pi_first_pin_or_internal_worst_drive() {
        let mut pi = NewSpeedNode::primary_input(NodeId(0), "pi");
        pi.pins = vec![
            DelayPin {
                drive: DelayTime::new(1.0, 2.0),
                load: 0.0,
            },
            DelayPin {
                drive: DelayTime::new(9.0, 9.0),
                load: 0.0,
            },
        ];
        assert_eq!(input_drive(&pi), DelayTime::new(1.0, 2.0));

        let mut internal = NewSpeedNode::internal(NodeId(1), "n1");
        internal.pins = vec![
            DelayPin {
                drive: DelayTime::new(1.0, 3.0),
                load: 0.0,
            },
            DelayPin {
                drive: DelayTime::new(2.0, 2.5),
                load: 0.0,
            },
        ];
        assert_eq!(input_drive(&internal), DelayTime::new(2.0, 3.0));
        assert_eq!(synthetic_input_name("a", 4), "a#4");
    }

    #[test]
    fn network_bound_entry_reports_missing_dependencies() {
        let mut network = ();
        assert_eq!(
            new_speed_network_bound(&mut network, &NewSpeedOptions::default()),
            Err(NewSpeedError::SisGraphDependency {
                operation: "new_speed",
                source: "LogicSynthesis/sis/speed/new_speed.c:30",
            })
        );
    }
}
