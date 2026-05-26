//! Native Rust model for `LogicSynthesis/sis/pld/act_collapse.c`.
//!
//! The original file greedily scores internal nodes, estimates the gain from
//! collapsing a node into all of its fanouts, accepts only positive-gain
//! collapses, and refreshes affected scores. Direct SIS graph mutation and ACT
//! mapper integration are represented by explicit dependency diagnostics; the
//! core scheduling and gain logic is available over owned Rust data.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActInitParam {
    pub fanin_collapse: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActCost {
    pub cost: i32,
}

impl ActCost {
    pub const fn new(cost: i32) -> Self {
        Self { cost }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActCollapseNode {
    pub name: String,
    pub kind: ActNodeKind,
    pub fanins: Vec<ActNodeId>,
    deleted: bool,
}

impl ActCollapseNode {
    pub fn new(name: impl Into<String>, kind: ActNodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            deleted: false,
        }
    }

    pub fn internal(name: impl Into<String>, fanins: Vec<ActNodeId>) -> Self {
        Self::new(name, ActNodeKind::Internal).with_fanins(fanins)
    }

    pub fn with_fanins(mut self, fanins: Vec<ActNodeId>) -> Self {
        self.fanins = fanins;
        self
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActCollapseNetwork {
    nodes: Vec<ActCollapseNode>,
}

impl ActCollapseNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: ActCollapseNode) -> ActNodeId {
        let id = ActNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: ActNodeId) -> ActCollapseResult<&ActCollapseNode> {
        self.nodes
            .get(id.0)
            .ok_or(ActCollapseError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[ActCollapseNode] {
        &self.nodes
    }

    pub fn active_node_ids(&self) -> Vec<ActNodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (!node.deleted).then_some(ActNodeId(index)))
            .collect()
    }

    pub fn dfs_node_ids(&self) -> Vec<ActNodeId> {
        self.active_node_ids()
    }

    pub fn internal_node_ids(&self) -> Vec<ActNodeId> {
        self.active_node_ids()
            .into_iter()
            .filter(|id| self.nodes[id.0].kind == ActNodeKind::Internal)
            .collect()
    }

    pub fn fanouts(&self, node: ActNodeId) -> ActCollapseResult<Vec<ActNodeId>> {
        self.node(node)?;
        Ok(self
            .active_node_ids()
            .into_iter()
            .filter(|candidate| self.nodes[candidate.0].fanins.contains(&node))
            .collect())
    }

    pub fn has_primary_output_fanout(&self, node: ActNodeId) -> ActCollapseResult<bool> {
        Ok(self
            .fanouts(node)?
            .into_iter()
            .any(|fanout| self.nodes[fanout.0].kind == ActNodeKind::PrimaryOutput))
    }

    pub fn delete_node(&mut self, node: ActNodeId) -> ActCollapseResult<()> {
        let target = self
            .nodes
            .get_mut(node.0)
            .ok_or(ActCollapseError::UnknownNode(node))?;
        target.deleted = true;
        Ok(())
    }

    pub fn collapse_node_into_fanout(
        &mut self,
        node: ActNodeId,
        fanout: ActNodeId,
    ) -> ActCollapseResult<()> {
        let replacement_fanins = self.node(node)?.fanins.clone();
        let fanout_node = self
            .nodes
            .get_mut(fanout.0)
            .ok_or(ActCollapseError::UnknownNode(fanout))?;
        if fanout_node.deleted {
            return Err(ActCollapseError::DeletedNode(fanout));
        }
        if !fanout_node.fanins.contains(&node) {
            return Err(ActCollapseError::NotAFanin {
                fanout,
                fanin: node,
            });
        }

        let mut revised = Vec::new();
        for current in fanout_node.fanins.iter().copied() {
            if current == node {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialCollapseReport {
    pub total_gain: i32,
    pub collapsed_nodes: Vec<CollapseAttempt>,
    pub remaining_costs: HashMap<ActNodeId, ActCost>,
    pub remaining_scores: HashMap<ActNodeId, i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollapseAttempt {
    pub node: ActNodeId,
    pub fanouts: Vec<ActNodeId>,
    pub gain: i32,
    pub accepted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActCollapseOperation {
    PartialCollapseWithoutLindo,
    PartialCollapseNode,
    EvaluateCollapsedFanout,
    UpdateActFields,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActCollapseError {
    UnknownNode(ActNodeId),
    DeletedNode(ActNodeId),
    NotAFanin { fanout: ActNodeId, fanin: ActNodeId },
    MissingCost(ActNodeId),
    MissingScore(ActNodeId),
    NegativeCost { node: ActNodeId, cost: i32 },
    MissingNativePorts { operation: ActCollapseOperation },
}

impl fmt::Display for ActCollapseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown ACT collapse node {:?}", node),
            Self::DeletedNode(node) => write!(f, "ACT collapse node {:?} was deleted", node),
            Self::NotAFanin { fanout, fanin } => {
                write!(f, "node {:?} is not a fanin of {:?}", fanin, fanout)
            }
            Self::MissingCost(node) => write!(f, "missing ACT collapse cost for node {:?}", node),
            Self::MissingScore(node) => write!(f, "missing ACT collapse score for node {:?}", node),
            Self::NegativeCost { node, cost } => {
                write!(f, "negative ACT collapse cost {cost} for node {:?}", node)
            }
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation:?} requires native Rust ports for SIS dependencies"
            ),
        }
    }
}

impl Error for ActCollapseError {}

pub type ActCollapseResult<T> = Result<T, ActCollapseError>;

pub fn act_partial_collapse_without_lindo_blocked<Network, CostTable>(
    _network: &mut Network,
    _cost_table: &mut CostTable,
    _init_param: &ActInitParam,
) -> ActCollapseResult<i32> {
    missing_native_ports(ActCollapseOperation::PartialCollapseWithoutLindo)
}

pub fn act_partial_collapse_node_blocked<Network, Node, CostTable, ScoreTable>(
    _network: &mut Network,
    _node: &Node,
    _cost_table: &mut CostTable,
    _init_param: &ActInitParam,
    _score_table: &mut ScoreTable,
) -> ActCollapseResult<i32> {
    missing_native_ports(ActCollapseOperation::PartialCollapseNode)
}

pub fn cost_of_node(
    node: ActNodeId,
    cost_table: &HashMap<ActNodeId, ActCost>,
) -> ActCollapseResult<i32> {
    let cost = cost_table
        .get(&node)
        .ok_or(ActCollapseError::MissingCost(node))?
        .cost;
    if cost < 0 {
        return Err(ActCollapseError::NegativeCost { node, cost });
    }
    Ok(cost)
}

pub fn act_partial_collapse_assign_score_node(
    network: &ActCollapseNetwork,
    node: ActNodeId,
    score_table: &mut HashMap<ActNodeId, i32>,
    cost_table: &HashMap<ActNodeId, ActCost>,
    init_param: &ActInitParam,
) -> ActCollapseResult<Option<i32>> {
    let node_ref = network.node(node)?;
    if node_ref.kind != ActNodeKind::Internal || node_ref.deleted {
        return Ok(None);
    }

    if node_ref.fanins.len() > init_param.fanin_collapse
        || network.has_primary_output_fanout(node)?
    {
        score_table.insert(node, 0);
        return Ok(Some(0));
    }

    let mut cost = cost_of_node(node, cost_table)?;
    if cost > 3 {
        score_table.insert(node, 0);
        return Ok(Some(0));
    }
    if cost == 0 {
        cost = 1;
    }

    let fanout_cost_sum = network
        .fanouts(node)?
        .into_iter()
        .try_fold(0_i32, |total, fanout| {
            Ok(total + cost_of_node(fanout, cost_table)?)
        })?;
    let score = fanout_cost_sum / cost;
    score_table.insert(node, score);
    Ok(Some(score))
}

pub fn act_partial_collapse_assign_score_network(
    network: &ActCollapseNetwork,
    score_table: &mut HashMap<ActNodeId, i32>,
    cost_table: &HashMap<ActNodeId, ActCost>,
    init_param: &ActInitParam,
) -> ActCollapseResult<()> {
    for node in network.dfs_node_ids() {
        act_partial_collapse_assign_score_node(network, node, score_table, cost_table, init_param)?;
    }
    Ok(())
}

pub fn act_partial_collapse_find_max_score(
    network: &ActCollapseNetwork,
    score_table: &HashMap<ActNodeId, i32>,
) -> ActCollapseResult<Option<(ActNodeId, i32)>> {
    let mut max_node = None;
    let mut max_score = -1;

    for node in network.internal_node_ids() {
        let score = *score_table
            .get(&node)
            .ok_or(ActCollapseError::MissingScore(node))?;
        if score > max_score {
            max_score = score;
            max_node = Some(node);
        }
    }

    Ok(max_node.map(|node| (node, max_score)))
}

pub fn act_partial_collapse_node_with_costs<F>(
    network: &mut ActCollapseNetwork,
    node: ActNodeId,
    cost_table: &mut HashMap<ActNodeId, ActCost>,
    score_table: &mut HashMap<ActNodeId, i32>,
    init_param: &ActInitParam,
    mut evaluate_collapsed_fanout: F,
) -> ActCollapseResult<CollapseAttempt>
where
    F: FnMut(&ActCollapseNetwork, ActNodeId, ActNodeId) -> ActCollapseResult<ActCost>,
{
    let fanouts = network.fanouts(node)?;
    let mut gain = cost_of_node(node, cost_table)?;
    let mut new_fanout_costs = Vec::with_capacity(fanouts.len());

    for fanout in &fanouts {
        let new_cost = evaluate_collapsed_fanout(network, *fanout, node)?;
        let old_cost = cost_of_node(*fanout, cost_table)?;
        gain += old_cost - new_cost.cost;
        new_fanout_costs.push((*fanout, new_cost));
    }

    if gain <= 0 {
        score_table.insert(node, 0);
        return Ok(CollapseAttempt {
            node,
            fanouts,
            gain,
            accepted: false,
        });
    }

    for (fanout, cost) in new_fanout_costs {
        network.collapse_node_into_fanout(node, fanout)?;
        cost_table.insert(fanout, cost);
        act_partial_collapse_assign_score_node(
            network,
            fanout,
            score_table,
            cost_table,
            init_param,
        )?;
    }

    cost_table
        .remove(&node)
        .ok_or(ActCollapseError::MissingCost(node))?;
    score_table
        .remove(&node)
        .ok_or(ActCollapseError::MissingScore(node))?;
    network.delete_node(node)?;

    Ok(CollapseAttempt {
        node,
        fanouts,
        gain,
        accepted: true,
    })
}

pub fn act_partial_collapse_without_lindo_with_costs<F>(
    network: &mut ActCollapseNetwork,
    cost_table: &mut HashMap<ActNodeId, ActCost>,
    init_param: &ActInitParam,
    mut evaluate_collapsed_fanout: F,
) -> ActCollapseResult<PartialCollapseReport>
where
    F: FnMut(&ActCollapseNetwork, ActNodeId, ActNodeId) -> ActCollapseResult<ActCost>,
{
    let mut score_table = HashMap::new();
    act_partial_collapse_assign_score_network(network, &mut score_table, cost_table, init_param)?;

    let mut total_gain = 0;
    let mut collapsed_nodes = Vec::new();
    loop {
        let Some((node, score)) = act_partial_collapse_find_max_score(network, &score_table)?
        else {
            break;
        };
        if score == 0 {
            break;
        }

        let attempt = act_partial_collapse_node_with_costs(
            network,
            node,
            cost_table,
            &mut score_table,
            init_param,
            &mut evaluate_collapsed_fanout,
        )?;
        if attempt.accepted {
            total_gain += attempt.gain;
            collapsed_nodes.push(attempt);
        }
    }

    Ok(PartialCollapseReport {
        total_gain,
        collapsed_nodes,
        remaining_costs: cost_table.clone(),
        remaining_scores: score_table,
    })
}

pub fn act_partial_collapse_update_act_fields_blocked<Node, Cost>(
    _node: &mut Node,
    _cost: &Cost,
) -> ActCollapseResult<()> {
    missing_native_ports(ActCollapseOperation::UpdateActFields)
}

fn missing_native_ports<T>(operation: ActCollapseOperation) -> ActCollapseResult<T> {
    Err(ActCollapseError::MissingNativePorts { operation })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ActInitParam {
        ActInitParam { fanin_collapse: 3 }
    }

    fn sample_network() -> (
        ActCollapseNetwork,
        ActNodeId,
        ActNodeId,
        ActNodeId,
        ActNodeId,
    ) {
        let mut network = ActCollapseNetwork::new();
        let a = network.add_node(ActCollapseNode::new("a", ActNodeKind::PrimaryInput));
        let b = network.add_node(ActCollapseNode::new("b", ActNodeKind::PrimaryInput));
        let x = network.add_node(ActCollapseNode::internal("x", vec![a, b]));
        let y = network.add_node(ActCollapseNode::internal("y", vec![x, b]));
        let z = network.add_node(ActCollapseNode::internal("z", vec![x, y]));
        (network, x, y, z, b)
    }

    #[test]
    fn score_node_matches_c_thresholds_and_integer_ratio() {
        let (mut network, x, y, _, _) = sample_network();
        let mut costs = HashMap::from([
            (x, ActCost::new(2)),
            (y, ActCost::new(5)),
            (ActNodeId(4), ActCost::new(3)),
        ]);
        let mut scores = HashMap::new();

        assert_eq!(
            act_partial_collapse_assign_score_node(&network, x, &mut scores, &costs, &params()),
            Ok(Some(4))
        );
        assert_eq!(scores[&x], 4);

        costs.insert(x, ActCost::new(4));
        assert_eq!(
            act_partial_collapse_assign_score_node(&network, x, &mut scores, &costs, &params()),
            Ok(Some(0))
        );

        let out = network.add_node(ActCollapseNode::new("out", ActNodeKind::PrimaryOutput));
        network.nodes[out.0].fanins = vec![y];
        assert_eq!(
            act_partial_collapse_assign_score_node(&network, y, &mut scores, &costs, &params()),
            Ok(Some(0))
        );
    }

    #[test]
    fn assign_score_network_skips_non_internal_nodes() {
        let (network, x, y, z, _) = sample_network();
        let costs = HashMap::from([
            (x, ActCost::new(2)),
            (y, ActCost::new(3)),
            (z, ActCost::new(1)),
        ]);
        let mut scores = HashMap::new();

        act_partial_collapse_assign_score_network(&network, &mut scores, &costs, &params())
            .unwrap();

        assert_eq!(scores.len(), 3);
        assert!(scores.contains_key(&x));
        assert!(scores.contains_key(&y));
        assert!(scores.contains_key(&z));
    }

    #[test]
    fn find_max_score_uses_network_order_to_break_ties() {
        let (network, x, y, z, _) = sample_network();
        let scores = HashMap::from([(x, 3), (y, 7), (z, 7)]);

        assert_eq!(
            act_partial_collapse_find_max_score(&network, &scores),
            Ok(Some((y, 7)))
        );
    }

    #[test]
    fn rejected_collapse_sets_score_zero_and_preserves_network_and_costs() {
        let (mut network, x, y, z, _) = sample_network();
        let mut costs = HashMap::from([
            (x, ActCost::new(1)),
            (y, ActCost::new(2)),
            (z, ActCost::new(2)),
        ]);
        let original = network.clone();
        let mut scores = HashMap::from([(x, 5), (y, 1), (z, 0)]);

        let attempt = act_partial_collapse_node_with_costs(
            &mut network,
            x,
            &mut costs,
            &mut scores,
            &params(),
            |_, _, _| Ok(ActCost::new(3)),
        )
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
        assert_eq!(network, original);
        assert!(costs.contains_key(&x));
        assert_eq!(scores[&x], 0);
    }

    #[test]
    fn accepted_collapse_updates_fanouts_costs_scores_and_deletes_node() {
        let (mut network, x, y, z, b) = sample_network();
        let mut costs = HashMap::from([
            (x, ActCost::new(2)),
            (y, ActCost::new(3)),
            (z, ActCost::new(4)),
        ]);
        let mut scores = HashMap::from([(x, 4), (y, 1), (z, 0)]);

        let attempt = act_partial_collapse_node_with_costs(
            &mut network,
            x,
            &mut costs,
            &mut scores,
            &params(),
            |_, fanout, _| {
                Ok(if fanout == y {
                    ActCost::new(1)
                } else {
                    ActCost::new(2)
                })
            },
        )
        .unwrap();

        assert!(attempt.accepted);
        assert_eq!(attempt.gain, 6);
        assert_eq!(network.node(y).unwrap().fanins, vec![ActNodeId(0), b]);
        assert_eq!(network.node(z).unwrap().fanins, vec![ActNodeId(0), b, y]);
        assert!(network.node(x).unwrap().is_deleted());
        assert!(!costs.contains_key(&x));
        assert_eq!(costs[&y], ActCost::new(1));
        assert_eq!(costs[&z], ActCost::new(2));
        assert!(!scores.contains_key(&x));
    }

    #[test]
    fn partial_collapse_without_lindo_repeats_until_no_positive_scores() {
        let (mut network, x, y, z, _) = sample_network();
        let mut costs = HashMap::from([
            (x, ActCost::new(2)),
            (y, ActCost::new(3)),
            (z, ActCost::new(3)),
        ]);

        let report = act_partial_collapse_without_lindo_with_costs(
            &mut network,
            &mut costs,
            &params(),
            |_, fanout, collapsed| {
                Ok(if collapsed == x || fanout == z {
                    ActCost::new(1)
                } else {
                    ActCost::new(3)
                })
            },
        )
        .unwrap();

        assert_eq!(
            report
                .collapsed_nodes
                .iter()
                .map(|attempt| attempt.node)
                .collect::<Vec<_>>(),
            vec![x, y]
        );
        assert_eq!(report.total_gain, 7);
        assert!(network.node(x).unwrap().is_deleted());
        assert!(network.node(y).unwrap().is_deleted());
        assert!(!network.node(z).unwrap().is_deleted());
    }

    #[test]
    fn blocked_sis_bound_entry_points_return_generic_dependency_error() {
        let result = act_partial_collapse_without_lindo_blocked(&mut (), &mut (), &params());

        assert_eq!(
            result,
            Err(ActCollapseError::MissingNativePorts {
                operation: ActCollapseOperation::PartialCollapseWithoutLindo
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_or_beads_metadata_tokens_are_present_in_this_port() {
        let source = include_str!("act_collapse.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
