//! Native Rust model for `LogicSynthesis/sis/pld/xln_feasible.c`.
//!
//! The C file reduces PLD infeasibility by moving one fanin of an oversized
//! node into another feasible fanin with a vacant input slot. The actual SIS
//! implementation proves each move by collapsing the candidate fanin,
//! selecting a Roth-Karp bound set, and accepting only a two-node
//! decomposition. This port keeps the traversal, candidate filtering, bound-set
//! construction, and modeled graph rewrite native. Direct mutation of SIS
//! `network_t`/`node_t` remains blocked behind explicit dependency errors.

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeasibleNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub critical: bool,
}

impl FeasibleNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            critical: false,
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }

    pub fn with_critical(mut self, critical: bool) -> Self {
        self.critical = critical;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeasibleNetwork {
    nodes: Vec<FeasibleNode>,
}

impl FeasibleNetwork {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: FeasibleNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> XlnFeasibleResult<&FeasibleNode> {
        self.nodes
            .get(id.0)
            .ok_or(XlnFeasibleError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[FeasibleNode] {
        &self.nodes
    }

    pub fn dfs_order(&self) -> Vec<NodeId> {
        (0..self.nodes.len()).map(NodeId).collect()
    }

    pub fn fanout_count(&self, id: NodeId) -> XlnFeasibleResult<usize> {
        self.node(id)?;
        Ok(self
            .nodes
            .iter()
            .filter(|node| node.fanins.contains(&id))
            .count())
    }

    pub fn fanin_index(&self, node: NodeId, fanin: NodeId) -> XlnFeasibleResult<Option<usize>> {
        Ok(self
            .node(node)?
            .fanins
            .iter()
            .position(|candidate| *candidate == fanin))
    }

    pub fn collapsed_fanins(&self, node: NodeId, g: NodeId) -> XlnFeasibleResult<Vec<NodeId>> {
        let mut result = Vec::new();
        let g_fanins = self.node(g)?.fanins.clone();

        for fanin in self.node(node)?.fanins.iter().copied() {
            if fanin == g {
                push_unique_all(&mut result, &g_fanins);
            } else {
                push_unique(&mut result, fanin);
            }
        }

        Ok(result)
    }

    pub fn move_fanin_into_target(
        &mut self,
        node: NodeId,
        fanin: NodeId,
        target: NodeId,
    ) -> XlnFeasibleResult<()> {
        self.node(fanin)?;
        {
            let target_node = self
                .nodes
                .get_mut(target.0)
                .ok_or(XlnFeasibleError::UnknownNode(target))?;
            push_unique(&mut target_node.fanins, fanin);
        }

        let node_ref = self
            .nodes
            .get_mut(node.0)
            .ok_or(XlnFeasibleError::UnknownNode(node))?;
        let before = node_ref.fanins.len();
        node_ref.fanins.retain(|candidate| *candidate != fanin);
        if node_ref.fanins.len() == before {
            return Err(XlnFeasibleError::NotAFanin { node, fanin });
        }
        Ok(())
    }
}

impl Default for FeasibleNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoveMode {
    Area,
    Delay,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoveFaninAttempt {
    pub node: NodeId,
    pub fanin_to_move: NodeId,
    pub target_fanin: NodeId,
    pub bound_set: Vec<NodeId>,
    pub lambda_indices: Vec<usize>,
    pub bound_alphas: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoveFaninReport {
    pub moved: bool,
    pub attempts: Vec<MoveFaninAttempt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReduceReport {
    pub visited_nodes: usize,
    pub improvements: usize,
}

pub trait DecompositionBackend {
    fn accepts(&mut self, attempt: &MoveFaninAttempt) -> XlnFeasibleResult<bool>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptingBackend {
    accepted: HashSet<(NodeId, NodeId, NodeId)>,
}

impl AcceptingBackend {
    pub fn new(accepted: impl IntoIterator<Item = (NodeId, NodeId, NodeId)>) -> Self {
        Self {
            accepted: accepted.into_iter().collect(),
        }
    }
}

impl DecompositionBackend for AcceptingBackend {
    fn accepts(&mut self, attempt: &MoveFaninAttempt) -> XlnFeasibleResult<bool> {
        Ok(self
            .accepted
            .remove(&(attempt.node, attempt.fanin_to_move, attempt.target_fanin)))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnFeasibleError {
    UnknownNode(NodeId),
    NotAFanin { node: NodeId, fanin: NodeId },
    InvalidBoundAlphas { bound_alphas: usize },
    MissingBoundSetNode { bound_node: NodeId },
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for XlnFeasibleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_feasible node {:?}", node),
            Self::NotAFanin { node, fanin } => {
                write!(f, "node {:?} is not a fanin of {:?}", fanin, node)
            }
            Self::InvalidBoundAlphas { bound_alphas } => {
                write!(
                    f,
                    "xln_node_move_fanin requires bound_alphas == 1, got {bound_alphas}"
                )
            }
            Self::MissingBoundSetNode { bound_node } => {
                write!(
                    f,
                    "bound-set node {:?} is not present in collapsed fanins",
                    bound_node
                )
            }
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for XlnFeasibleError {}

pub type XlnFeasibleResult<T> = Result<T, XlnFeasibleError>;

pub fn reduce_infeasibility_by_moving_fanins<B>(
    network: &mut FeasibleNetwork,
    support: usize,
    max_fanins: usize,
    backend: &mut B,
) -> XlnFeasibleResult<ReduceReport>
where
    B: DecompositionBackend,
{
    let order = network.dfs_order();
    let mut improvements = 0;
    for node in order.iter().copied() {
        improvements += node_move_fanins(network, node, support, max_fanins, 0, backend)?;
    }

    Ok(ReduceReport {
        visited_nodes: order.len(),
        improvements,
    })
}

pub fn node_move_fanins<B>(
    network: &mut FeasibleNetwork,
    node: NodeId,
    support: usize,
    max_fanins: usize,
    diff: usize,
    backend: &mut B,
) -> XlnFeasibleResult<usize>
where
    B: DecompositionBackend,
{
    let node_ref = network.node(node)?;
    if node_ref.kind != NodeKind::Internal {
        return Ok(0);
    }

    let num_fanin = node_ref.fanins.len();
    if num_fanin > max_fanins {
        return Ok(0);
    }

    let infeasibility = if diff == 0 {
        match num_fanin.checked_sub(support) {
            Some(infeasibility) if infeasibility > 0 => infeasibility,
            _ => return Ok(0),
        }
    } else {
        diff
    };

    let fanin_snapshot = node_ref.fanins.clone();
    let mut improvement = 0;
    for fanin in fanin_snapshot {
        if node_move_fanin(network, node, fanin, support, 1, MoveMode::Area, backend)?.moved {
            improvement += 1;
        }
        if improvement == infeasibility {
            break;
        }
    }

    Ok(improvement)
}

pub fn node_move_fanin<B>(
    network: &mut FeasibleNetwork,
    node: NodeId,
    fanin_to_move: NodeId,
    support: usize,
    bound_alphas: usize,
    mode: MoveMode,
    backend: &mut B,
) -> XlnFeasibleResult<MoveFaninReport>
where
    B: DecompositionBackend,
{
    if bound_alphas != 1 {
        return Err(XlnFeasibleError::InvalidBoundAlphas { bound_alphas });
    }
    if network.fanin_index(node, fanin_to_move)?.is_none() {
        return Err(XlnFeasibleError::NotAFanin {
            node,
            fanin: fanin_to_move,
        });
    }

    let mut attempts = Vec::new();
    for target_fanin in feasible_target_fanins(network, node, fanin_to_move, support, mode)? {
        let collapsed_fanins = network.collapsed_fanins(node, target_fanin)?;
        let bound_set = get_bound_set(
            network,
            node,
            fanin_to_move,
            target_fanin,
            &collapsed_fanins,
        )?;
        if bound_set.len() <= 1 {
            continue;
        }
        let lambda_indices = array_to_indices(&bound_set, &collapsed_fanins)?;
        let attempt = MoveFaninAttempt {
            node,
            fanin_to_move,
            target_fanin,
            bound_set,
            lambda_indices,
            bound_alphas,
        };
        let accepted = backend.accepts(&attempt)?;
        attempts.push(attempt);
        if accepted {
            network.move_fanin_into_target(node, fanin_to_move, target_fanin)?;
            return Ok(MoveFaninReport {
                moved: true,
                attempts,
            });
        }
    }

    Ok(MoveFaninReport {
        moved: false,
        attempts,
    })
}

pub fn feasible_target_fanins(
    network: &FeasibleNetwork,
    node: NodeId,
    fanin_to_move: NodeId,
    support: usize,
    mode: MoveMode,
) -> XlnFeasibleResult<Vec<NodeId>> {
    let mut result = Vec::new();
    for candidate in network.node(node)?.fanins.iter().copied() {
        if candidate == fanin_to_move {
            continue;
        }
        let candidate_ref = network.node(candidate)?;
        if candidate_ref.kind == NodeKind::PrimaryInput {
            continue;
        }
        if network.fanout_count(candidate)? != 1 {
            continue;
        }
        if candidate_ref.fanins.len() >= support {
            continue;
        }
        if mode == MoveMode::Delay && candidate_ref.critical {
            continue;
        }
        result.push(candidate);
    }
    Ok(result)
}

pub fn get_bound_set(
    network: &FeasibleNetwork,
    node: NodeId,
    fanin_to_move: NodeId,
    target_fanin: NodeId,
    collapsed_fanins: &[NodeId],
) -> XlnFeasibleResult<Vec<NodeId>> {
    network.node(node)?;
    network.node(fanin_to_move)?;
    let mut result = vec![fanin_to_move];
    for fanin in network.node(target_fanin)?.fanins.iter().copied() {
        if network.fanin_index(node, fanin)?.is_none() && collapsed_fanins.contains(&fanin) {
            result.push(fanin);
        }
    }
    Ok(result)
}

pub fn array_to_indices(
    bound_set: &[NodeId],
    collapsed_fanins: &[NodeId],
) -> XlnFeasibleResult<Vec<usize>> {
    bound_set
        .iter()
        .copied()
        .map(|bound_node| {
            collapsed_fanins
                .iter()
                .position(|candidate| *candidate == bound_node)
                .ok_or(XlnFeasibleError::MissingBoundSetNode { bound_node })
        })
        .collect()
}

pub fn reduce_infeasibility_by_moving_fanins_blocked<Network>(
    _network: &mut Network,
    _support: usize,
    _max_fanins: usize,
) -> XlnFeasibleResult<ReduceReport> {
    Err(missing_native_ports(
        "xln_network_reduce_infeasibility_by_moving_fanins SIS integration",
    ))
}

pub fn node_move_fanins_blocked<Node>(
    _node: &mut Node,
    _support: usize,
    _max_fanins: usize,
    _diff: usize,
) -> XlnFeasibleResult<usize> {
    Err(missing_native_ports("xln_node_move_fanins SIS integration"))
}

pub fn node_move_fanin_blocked<Node>(
    _node: &mut Node,
    _fanin_to_move: &mut Node,
    _support: usize,
    _bound_alphas: usize,
    _mode: MoveMode,
) -> XlnFeasibleResult<MoveFaninReport> {
    Err(missing_native_ports("xln_node_move_fanin SIS integration"))
}

fn missing_native_ports(operation: &'static str) -> XlnFeasibleError {
    XlnFeasibleError::MissingNativePorts { operation }
}

fn push_unique(values: &mut Vec<NodeId>, value: NodeId) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn push_unique_all(values: &mut Vec<NodeId>, additional: &[NodeId]) {
    for value in additional.iter().copied() {
        push_unique(values, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> (FeasibleNetwork, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let mut network = FeasibleNetwork::new();
        let a = network.add_node(FeasibleNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(FeasibleNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(FeasibleNode::new("c", NodeKind::PrimaryInput));
        let g =
            network.add_node(FeasibleNode::new("g", NodeKind::Internal).with_fanins(vec![b, c]));
        let n =
            network.add_node(FeasibleNode::new("n", NodeKind::Internal).with_fanins(vec![a, g]));
        (network, a, b, c, g, n)
    }

    #[test]
    fn target_filter_matches_c_vacancy_and_delay_rules() {
        let (mut network, a, _, _, g, n) = sample_network();
        network.nodes[g.0].critical = true;

        assert_eq!(
            feasible_target_fanins(&network, n, a, 3, MoveMode::Area).unwrap(),
            vec![g]
        );
        assert_eq!(
            feasible_target_fanins(&network, n, a, 3, MoveMode::Delay).unwrap(),
            Vec::<NodeId>::new()
        );
        assert_eq!(
            feasible_target_fanins(&network, n, a, 2, MoveMode::Area).unwrap(),
            Vec::<NodeId>::new()
        );
    }

    #[test]
    fn bound_set_includes_moved_fanin_and_new_target_fanins_only() {
        let (network, a, b, c, g, n) = sample_network();
        let collapsed = network.collapsed_fanins(n, g).unwrap();

        assert_eq!(collapsed, vec![a, b, c]);
        assert_eq!(
            get_bound_set(&network, n, a, g, &collapsed).unwrap(),
            vec![a, b, c]
        );
        assert_eq!(array_to_indices(&[a, c], &collapsed).unwrap(), vec![0, 2]);
    }

    #[test]
    fn node_move_fanin_requires_two_node_decomposition_acceptance() {
        let (mut network, a, _, _, g, n) = sample_network();
        let mut backend = AcceptingBackend::new([(n, a, g)]);

        let report =
            node_move_fanin(&mut network, n, a, 3, 1, MoveMode::Area, &mut backend).unwrap();

        assert!(report.moved);
        assert_eq!(report.attempts.len(), 1);
        assert_eq!(report.attempts[0].bound_set.len(), 3);
        assert_eq!(network.node(n).unwrap().fanins, vec![g]);
        assert_eq!(network.node(g).unwrap().fanins, vec![b_id(), c_id(), a]);
    }

    #[test]
    fn node_move_fanins_stops_when_diff_zero_becomes_feasible() {
        let mut network = FeasibleNetwork::new();
        let a = network.add_node(FeasibleNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(FeasibleNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(FeasibleNode::new("c", NodeKind::PrimaryInput));
        let g1 = network.add_node(FeasibleNode::new("g1", NodeKind::Internal).with_fanins(vec![b]));
        let g2 = network.add_node(FeasibleNode::new("g2", NodeKind::Internal).with_fanins(vec![c]));
        let n = network
            .add_node(FeasibleNode::new("n", NodeKind::Internal).with_fanins(vec![a, g1, g2]));
        let mut backend = AcceptingBackend::new([(n, a, g1), (n, g2, g1)]);

        let moved = node_move_fanins(&mut network, n, 2, 8, 0, &mut backend).unwrap();

        assert_eq!(moved, 1);
        assert_eq!(network.node(n).unwrap().fanins, vec![g1, g2]);
    }

    #[test]
    fn network_pass_visits_dfs_snapshot_and_accumulates_improvements() {
        let mut network = FeasibleNetwork::new();
        let a = network.add_node(FeasibleNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(FeasibleNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(FeasibleNode::new("c", NodeKind::PrimaryInput));
        let g = network.add_node(FeasibleNode::new("g", NodeKind::Internal).with_fanins(vec![c]));
        let n =
            network.add_node(FeasibleNode::new("n", NodeKind::Internal).with_fanins(vec![a, b, g]));
        let mut backend = AcceptingBackend::new([(n, a, g)]);

        let report =
            reduce_infeasibility_by_moving_fanins(&mut network, 2, 8, &mut backend).unwrap();

        assert_eq!(
            report,
            ReduceReport {
                visited_nodes: 5,
                improvements: 1,
            }
        );
        assert_eq!(network.node(n).unwrap().fanins, vec![b, g]);
    }

    #[test]
    fn ineligible_nodes_and_oversized_nodes_do_not_move() {
        let (mut network, a, _, _, _, n) = sample_network();
        let mut backend = AcceptingBackend::new([]);

        assert_eq!(
            node_move_fanins(&mut network, a, 1, 8, 0, &mut backend).unwrap(),
            0
        );
        assert_eq!(
            node_move_fanins(&mut network, n, 1, 1, 0, &mut backend).unwrap(),
            0
        );
    }

    #[test]
    fn invalid_bound_alpha_matches_c_assertion_as_error() {
        let (mut network, a, _, _, _, n) = sample_network();
        let mut backend = AcceptingBackend::new([]);

        assert_eq!(
            node_move_fanin(&mut network, n, a, 3, 2, MoveMode::Area, &mut backend).unwrap_err(),
            XlnFeasibleError::InvalidBoundAlphas { bound_alphas: 2 }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_feasible.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }

    fn b_id() -> NodeId {
        NodeId(1)
    }

    fn c_id() -> NodeId {
        NodeId(2)
    }
}
