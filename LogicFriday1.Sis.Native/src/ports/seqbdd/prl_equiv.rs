//! Native Rust model for `LogicSynthesis/sis/seqbdd/prl_equiv.c`.
//!
//! The C implementation computes BDD signatures for every non-PO net, groups
//! nets whose care-restricted functions are equal or complementary, picks the
//! shallowest and then cheapest representative in each non-trivial class, and
//! rewrites fanouts through that representative or an inserted inverter. This
//! module ports the deterministic equivalence-class and representative-selection
//! behavior onto owned Rust records. Direct SIS `network_t`, `node_t`,
//! `array_t`, `st_table`, and BDD manager integration remains blocked on the
//! missing native SIS ports.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub fn is_prl_equiv_sis_integration_blocked() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FunctionSignature {
    id: u64,
    inverted: bool,
}

impl FunctionSignature {
    pub const fn new(id: u64) -> Self {
        Self {
            id,
            inverted: false,
        }
    }

    pub const fn complemented(id: u64) -> Self {
        Self { id, inverted: true }
    }

    pub const fn not(self) -> Self {
        Self {
            id: self.id,
            inverted: !self.inverted,
        }
    }

    pub const fn is_complement_of(self, other: Self) -> bool {
        self.id == other.id && self.inverted != other.inverted
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquivNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub fanout_count: usize,
    pub literal_count: usize,
    pub function: FunctionSignature,
}

impl EquivNode {
    pub fn new(
        id: NodeId,
        name: impl Into<String>,
        kind: NodeKind,
        function: FunctionSignature,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanout_count: 0,
            literal_count: 0,
            function,
        }
    }

    pub fn with_fanins(mut self, fanins: impl Into<Vec<NodeId>>) -> Self {
        self.fanins = fanins.into();
        self
    }

    pub fn with_fanout_count(mut self, fanout_count: usize) -> Self {
        self.fanout_count = fanout_count;
        self
    }

    pub fn with_literal_count(mut self, literal_count: usize) -> Self {
        self.literal_count = literal_count;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrlEquivError {
    MissingNativePorts { operation: &'static str },
    DuplicateNodeId(NodeId),
    UnknownFanin { node: NodeId, fanin: NodeId },
    RecursiveFaninCycle { node: NodeId },
}

impl fmt::Display for PrlEquivError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by missing native SIS ports")
            }
            Self::DuplicateNodeId(node) => write!(f, "duplicate prl_equiv node id {:?}", node),
            Self::UnknownFanin { node, fanin } => {
                write!(f, "node {:?} references unknown fanin {:?}", node, fanin)
            }
            Self::RecursiveFaninCycle { node } => {
                write!(f, "recursive fanin cycle reaches {:?}", node)
            }
        }
    }
}

impl Error for PrlEquivError {}

pub fn prl_equiv_nets_from_sis() -> Result<EquivCollapsePlan, PrlEquivError> {
    Err(PrlEquivError::MissingNativePorts {
        operation: "Prl_EquivNets SIS network/BDD entry",
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetCost {
    pub node: NodeId,
    pub depth: usize,
    pub cost: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquivalenceClass {
    pub root: NodeId,
    pub members: Vec<NodeId>,
    pub best_member: Option<NodeId>,
    pub member_costs: Vec<NetCost>,
}

impl EquivalenceClass {
    pub fn is_trivial(&self) -> bool {
        self.members.len() <= 1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplacementTarget {
    Node(NodeId),
    InverterOf(NodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutMove {
    pub from: NodeId,
    pub target: ReplacementTarget,
    pub inverted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassCollapse {
    pub root: NodeId,
    pub best_member: NodeId,
    pub inserted_inverter_for: NodeId,
    pub moves: Vec<FanoutMove>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquivCollapsePlan {
    pub classes: Vec<EquivalenceClass>,
    pub collapses: Vec<ClassCollapse>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquivalenceStats {
    pub total_classes: usize,
    pub classes_by_size: BTreeMap<usize, usize>,
}

pub fn plan_equivalent_net_collapses(
    nodes: &[EquivNode],
) -> Result<EquivCollapsePlan, PrlEquivError> {
    let node_map = node_map(nodes)?;
    let classes = compute_equivalence_classes(nodes, &node_map)?;
    let collapses = classes
        .iter()
        .filter(|class| !class.is_trivial())
        .map(|class| collapse_equivalence_class(class, &node_map))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(EquivCollapsePlan { classes, collapses })
}

pub fn equivalence_stats(classes: &[EquivalenceClass]) -> EquivalenceStats {
    let mut classes_by_size = BTreeMap::new();
    for class in classes {
        *classes_by_size.entry(class.members.len()).or_insert(0) += 1;
    }

    EquivalenceStats {
        total_classes: classes.len(),
        classes_by_size,
    }
}

fn compute_equivalence_classes(
    nodes: &[EquivNode],
    node_map: &BTreeMap<NodeId, &EquivNode>,
) -> Result<Vec<EquivalenceClass>, PrlEquivError> {
    let candidate_nodes = nodes
        .iter()
        .filter(|node| node.kind != NodeKind::PrimaryOutput)
        .collect::<Vec<_>>();
    let mut root_index_by_position = (0..candidate_nodes.len()).collect::<Vec<_>>();
    let mut class_member_positions = (0..candidate_nodes.len())
        .map(|index| vec![index])
        .collect::<Vec<_>>();

    for i in 0..candidate_nodes.len() {
        if root_index_by_position[i] < i {
            continue;
        }
        for j in (i + 1)..candidate_nodes.len() {
            if root_index_by_position[j] < j {
                continue;
            }
            let left = candidate_nodes[i].function;
            let right = candidate_nodes[j].function;
            if left == right || left.is_complement_of(right) {
                class_member_positions[i].push(j);
                root_index_by_position[j] = i;
            }
        }
    }

    let mut classes = Vec::new();
    for i in 0..candidate_nodes.len() {
        if root_index_by_position[i] < i {
            continue;
        }
        let members = class_member_positions[i]
            .iter()
            .map(|position| candidate_nodes[*position].id)
            .collect::<Vec<_>>();
        let member_costs = members
            .iter()
            .map(|member| compute_node_cost(*member, node_map))
            .collect::<Result<Vec<_>, _>>()?;
        let best_member = choose_best_member(&member_costs);
        classes.push(EquivalenceClass {
            root: candidate_nodes[i].id,
            members,
            best_member,
            member_costs,
        });
    }

    Ok(classes)
}

fn collapse_equivalence_class(
    class: &EquivalenceClass,
    node_map: &BTreeMap<NodeId, &EquivNode>,
) -> Result<ClassCollapse, PrlEquivError> {
    let best_member = class
        .best_member
        .expect("non-trivial equivalence classes always have a best member");
    let best_function = node_map
        .get(&best_member)
        .expect("best member comes from validated class members")
        .function;
    let mut moves = Vec::new();

    for member in &class.members {
        let member_function = node_map
            .get(member)
            .expect("class members come from validated node map")
            .function;
        let inverted = member_function != best_function;
        let target = if inverted {
            ReplacementTarget::InverterOf(best_member)
        } else {
            ReplacementTarget::Node(best_member)
        };
        moves.push(FanoutMove {
            from: *member,
            target,
            inverted,
        });
    }

    Ok(ClassCollapse {
        root: class.root,
        best_member,
        inserted_inverter_for: best_member,
        moves,
    })
}

fn choose_best_member(costs: &[NetCost]) -> Option<NodeId> {
    costs
        .iter()
        .min_by_key(|cost| (cost.depth, cost.cost))
        .map(|cost| cost.node)
}

fn compute_node_cost(
    node: NodeId,
    node_map: &BTreeMap<NodeId, &EquivNode>,
) -> Result<NetCost, PrlEquivError> {
    let mut memo = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    let (depth, cost) = compute_node_cost_rec(node, node_map, &mut memo, &mut visiting)?;
    Ok(NetCost { node, depth, cost })
}

fn compute_node_cost_rec(
    node: NodeId,
    node_map: &BTreeMap<NodeId, &EquivNode>,
    memo: &mut BTreeMap<NodeId, (usize, usize)>,
    visiting: &mut BTreeSet<NodeId>,
) -> Result<(usize, usize), PrlEquivError> {
    if let Some(value) = memo.get(&node) {
        return Ok(*value);
    }
    if !visiting.insert(node) {
        return Err(PrlEquivError::RecursiveFaninCycle { node });
    }

    let net = node_map
        .get(&node)
        .copied()
        .ok_or(PrlEquivError::UnknownFanin { node, fanin: node })?;
    let mut cost = if net.fanout_count == 1 {
        net.literal_count
    } else {
        0
    };
    let mut depth = 0;

    for fanin in &net.fanins {
        if !node_map.contains_key(fanin) {
            return Err(PrlEquivError::UnknownFanin {
                node,
                fanin: *fanin,
            });
        }
        let (fanin_depth, fanin_cost) = compute_node_cost_rec(*fanin, node_map, memo, visiting)?;
        cost += fanin_cost;
        depth = depth.max(fanin_depth + 1);
    }

    visiting.remove(&node);
    memo.insert(node, (depth, cost));
    Ok((depth, cost))
}

fn node_map(nodes: &[EquivNode]) -> Result<BTreeMap<NodeId, &EquivNode>, PrlEquivError> {
    let mut result = BTreeMap::new();
    for node in nodes {
        if result.insert(node.id, node).is_some() {
            return Err(PrlEquivError::DuplicateNodeId(node.id));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(id: u64) -> FunctionSignature {
        FunctionSignature::new(id)
    }

    fn inv(id: u64) -> FunctionSignature {
        FunctionSignature::complemented(id)
    }

    #[test]
    fn groups_equal_and_complementary_non_output_nets() {
        let nodes = vec![
            EquivNode::new(NodeId(0), "a", NodeKind::PrimaryInput, sig(1)),
            EquivNode::new(NodeId(1), "b", NodeKind::PrimaryInput, sig(2)),
            EquivNode::new(NodeId(2), "n1", NodeKind::Internal, sig(9)),
            EquivNode::new(NodeId(3), "n2", NodeKind::Internal, sig(9)),
            EquivNode::new(NodeId(4), "n3", NodeKind::Internal, inv(9)),
            EquivNode::new(NodeId(5), "po", NodeKind::PrimaryOutput, sig(9)),
        ];

        let plan = plan_equivalent_net_collapses(&nodes).unwrap();
        let stats = equivalence_stats(&plan.classes);

        assert_eq!(stats.total_classes, 3);
        assert_eq!(stats.classes_by_size.get(&1), Some(&2));
        assert_eq!(stats.classes_by_size.get(&3), Some(&1));
        assert_eq!(plan.collapses.len(), 1);
        assert_eq!(plan.collapses[0].root, NodeId(2));
        assert_eq!(plan.collapses[0].moves.len(), 3);
        assert!(plan.collapses[0].moves.iter().any(|move_| {
            move_.from == NodeId(4)
                && move_.target == ReplacementTarget::InverterOf(plan.collapses[0].best_member)
                && move_.inverted
        }));
    }

    #[test]
    fn chooses_lowest_depth_before_lowest_cost() {
        let nodes = vec![
            EquivNode::new(NodeId(0), "pi", NodeKind::PrimaryInput, sig(1))
                .with_fanout_count(1)
                .with_literal_count(8),
            EquivNode::new(NodeId(1), "deep", NodeKind::Internal, sig(7))
                .with_fanins(vec![NodeId(0)])
                .with_fanout_count(2)
                .with_literal_count(1),
            EquivNode::new(NodeId(2), "shallow", NodeKind::Internal, sig(7))
                .with_fanout_count(1)
                .with_literal_count(20),
        ];

        let plan = plan_equivalent_net_collapses(&nodes).unwrap();
        let collapse = plan
            .collapses
            .iter()
            .find(|collapse| collapse.root == NodeId(1))
            .unwrap();

        assert_eq!(collapse.best_member, NodeId(2));
        let class = plan
            .classes
            .iter()
            .find(|class| class.root == NodeId(1))
            .unwrap();
        assert!(class.member_costs.contains(&NetCost {
            node: NodeId(1),
            depth: 1,
            cost: 8,
        }));
        assert!(class.member_costs.contains(&NetCost {
            node: NodeId(2),
            depth: 0,
            cost: 20,
        }));
    }

    #[test]
    fn chooses_lowest_cost_when_depths_tie() {
        let nodes = vec![
            EquivNode::new(NodeId(0), "expensive", NodeKind::Internal, sig(4))
                .with_fanout_count(1)
                .with_literal_count(9),
            EquivNode::new(NodeId(1), "cheap", NodeKind::Internal, inv(4))
                .with_fanout_count(1)
                .with_literal_count(3),
        ];

        let plan = plan_equivalent_net_collapses(&nodes).unwrap();

        assert_eq!(plan.collapses[0].best_member, NodeId(1));
        assert_eq!(
            plan.collapses[0]
                .moves
                .iter()
                .find(|move_| move_.from == NodeId(0))
                .unwrap()
                .target,
            ReplacementTarget::InverterOf(NodeId(1))
        );
    }

    #[test]
    fn reports_unknown_fanin_and_cycles() {
        let unknown = vec![
            EquivNode::new(NodeId(0), "n", NodeKind::Internal, sig(1))
                .with_fanins(vec![NodeId(99)]),
        ];

        assert!(matches!(
            plan_equivalent_net_collapses(&unknown),
            Err(PrlEquivError::UnknownFanin {
                node: NodeId(0),
                fanin: NodeId(99)
            })
        ));

        let cycle = vec![
            EquivNode::new(NodeId(0), "a", NodeKind::Internal, sig(1)).with_fanins(vec![NodeId(1)]),
            EquivNode::new(NodeId(1), "b", NodeKind::Internal, sig(2)).with_fanins(vec![NodeId(0)]),
        ];

        assert!(matches!(
            plan_equivalent_net_collapses(&cycle),
            Err(PrlEquivError::RecursiveFaninCycle { node: NodeId(0) })
                | Err(PrlEquivError::RecursiveFaninCycle { node: NodeId(1) })
        ));
    }
}
