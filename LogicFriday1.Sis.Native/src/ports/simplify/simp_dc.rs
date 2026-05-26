//! Native Rust model for feasible behavior in `sis/simplify/simp_dc.c`.
//!
//! The C implementation builds several don't-care approximations from SIS
//! `node_t` objects, `network_tfi`/`network_tfo` cone walks, ST tables, arrays,
//! and Boolean node operations. This module ports those algorithms onto a small
//! owned graph and expression model. Direct SIS pointer integration remains an
//! explicit missing-dependency error until the required native ports exist.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const INFINITY_LEVEL: usize = usize::MAX;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SimpNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimpNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoolExpr {
    Const(bool),
    Literal { node: SimpNodeId, positive: bool },
    NodeFunction(SimpNodeId),
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
    Xor(Box<BoolExpr>, Box<BoolExpr>),
    Xnor(Box<BoolExpr>, Box<BoolExpr>),
}

impl BoolExpr {
    pub const fn zero() -> Self {
        Self::Const(false)
    }

    pub const fn one() -> Self {
        Self::Const(true)
    }

    pub const fn node_function(node: SimpNodeId) -> Self {
        Self::NodeFunction(node)
    }

    pub const fn positive_literal(node: SimpNodeId) -> Self {
        Self::Literal {
            node,
            positive: true,
        }
    }

    pub fn and(left: BoolExpr, right: BoolExpr) -> Self {
        match (left, right) {
            (Self::Const(false), _) | (_, Self::Const(false)) => Self::Const(false),
            (Self::Const(true), expr) | (expr, Self::Const(true)) => expr,
            (Self::And(mut terms), Self::And(other)) => {
                terms.extend(other);
                Self::And(terms)
            }
            (Self::And(mut terms), expr) | (expr, Self::And(mut terms)) => {
                terms.push(expr);
                Self::And(terms)
            }
            (left, right) => Self::And(vec![left, right]),
        }
    }

    pub fn or(left: BoolExpr, right: BoolExpr) -> Self {
        match (left, right) {
            (Self::Const(true), _) | (_, Self::Const(true)) => Self::Const(true),
            (Self::Const(false), expr) | (expr, Self::Const(false)) => expr,
            (Self::Or(mut terms), Self::Or(other)) => {
                terms.extend(other);
                Self::Or(terms)
            }
            (Self::Or(mut terms), expr) | (expr, Self::Or(mut terms)) => {
                terms.push(expr);
                Self::Or(terms)
            }
            (left, right) => Self::Or(vec![left, right]),
        }
    }

    pub fn xor(left: BoolExpr, right: BoolExpr) -> Self {
        match (left, right) {
            (Self::Const(left), Self::Const(right)) => Self::Const(left ^ right),
            (Self::Const(false), expr) | (expr, Self::Const(false)) => expr,
            (Self::Const(true), expr) | (expr, Self::Const(true)) => {
                Self::Xor(Box::new(Self::Const(true)), Box::new(expr))
            }
            (left, right) if left == right => Self::Const(false),
            (left, right) => Self::Xor(Box::new(left), Box::new(right)),
        }
    }

    pub fn xnor(left: BoolExpr, right: BoolExpr) -> Self {
        match (left, right) {
            (Self::Const(left), Self::Const(right)) => Self::Const(left == right),
            (left, right) if left == right => Self::Const(true),
            (left, right) => Self::Xnor(Box::new(left), Box::new(right)),
        }
    }

    pub fn cofactor(&self, variable: SimpNodeId, value: bool) -> Self {
        match self {
            Self::Const(value) => Self::Const(*value),
            Self::Literal { node, positive } if *node == variable => {
                Self::Const(if *positive { value } else { !value })
            }
            Self::Literal { node, positive } => Self::Literal {
                node: *node,
                positive: *positive,
            },
            Self::NodeFunction(node) => Self::NodeFunction(*node),
            Self::And(terms) => terms
                .iter()
                .map(|term| term.cofactor(variable, value))
                .fold(Self::one(), Self::and),
            Self::Or(terms) => terms
                .iter()
                .map(|term| term.cofactor(variable, value))
                .fold(Self::zero(), Self::or),
            Self::Xor(left, right) => Self::xor(
                left.cofactor(variable, value),
                right.cofactor(variable, value),
            ),
            Self::Xnor(left, right) => Self::xnor(
                left.cofactor(variable, value),
                right.cofactor(variable, value),
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimpNode {
    pub name: String,
    pub kind: SimpNodeKind,
    pub fanins: Vec<SimpNodeId>,
    pub fanouts: Vec<SimpNodeId>,
    pub function: BoolExpr,
    pub cube_count: usize,
    pub literal_count: usize,
    pub level: Option<i32>,
}

impl SimpNode {
    pub fn new(name: impl Into<String>, kind: SimpNodeKind, function: BoolExpr) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            function,
            cube_count: 1,
            literal_count: 1,
            level: None,
        }
    }

    pub fn with_cost(mut self, cube_count: usize, literal_count: usize) -> Self {
        self.cube_count = cube_count;
        self.literal_count = literal_count;
        self
    }

    pub fn with_level(mut self, level: i32) -> Self {
        self.level = Some(level);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimpNetwork {
    nodes: Vec<SimpNode>,
}

impl SimpNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: SimpNode) -> SimpNodeId {
        let id = SimpNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn connect(&mut self, fanin: SimpNodeId, fanout: SimpNodeId) -> Result<(), SimpDcError> {
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

    pub fn node(&self, id: SimpNodeId) -> Result<&SimpNode, SimpDcError> {
        self.nodes.get(id.0).ok_or(SimpDcError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: SimpNodeId) -> Result<&mut SimpNode, SimpDcError> {
        self.nodes.get_mut(id.0).ok_or(SimpDcError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[SimpNode] {
        &self.nodes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeFanout {
    pub node: SimpNodeId,
    pub fanout: SimpNodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimpDcError {
    UnknownNode(SimpNodeId),
    MissingLevel(SimpNodeId),
    MissingSisPorts { operation: &'static str },
}

impl fmt::Display for SimpDcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown simplify node {:?}", node),
            Self::MissingLevel(node) => write!(f, "missing simplify level for {:?}", node),
            Self::MissingSisPorts { operation } => write!(
                f,
                "{operation} requires native Rust SIS ports that are not available yet"
            ),
        }
    }
}

impl Error for SimpDcError {}

pub fn sis_node_bound_dont_care(operation: &'static str) -> Result<BoolExpr, SimpDcError> {
    Err(SimpDcError::MissingSisPorts { operation })
}

pub fn simp_di(node: SimpNodeId) -> BoolExpr {
    BoolExpr::xor(
        BoolExpr::node_function(node),
        BoolExpr::positive_literal(node),
    )
}

pub fn unused_simp_fanin_dc(
    network: &SimpNetwork,
    node: SimpNodeId,
) -> Result<BoolExpr, SimpDcError> {
    let mut dc = BoolExpr::zero();
    for useful in shared_fanin_fanouts(network, node)? {
        if useful != node {
            dc = BoolExpr::or(dc, simp_di(useful));
        }
    }
    Ok(dc)
}

pub fn simp_fanout_dc(network: &SimpNetwork, node: SimpNodeId) -> Result<BoolExpr, SimpDcError> {
    let mut dc = BoolExpr::one();
    for fanout in network.node(node)?.fanouts.iter().copied() {
        let fanout_node = network.node(fanout)?;
        if fanout_node.kind == SimpNodeKind::PrimaryOutput {
            return Ok(BoolExpr::zero());
        }

        let positive = fanout_node.function.cofactor(node, true);
        let negative = fanout_node.function.cofactor(node, false);
        dc = BoolExpr::and(dc, BoolExpr::xnor(positive, negative));
        if dc == BoolExpr::zero() {
            break;
        }
    }
    Ok(dc)
}

pub fn simp_inout_dc(
    network: &SimpNetwork,
    node: SimpNodeId,
    input_level: usize,
    output_level: usize,
) -> Result<BoolExpr, SimpDcError> {
    Ok(BoolExpr::or(
        simp_tfanin_dc(network, node, input_level, output_level)?,
        simp_fanout_dc(network, node)?,
    ))
}

pub fn simp_tfanin_dc(
    network: &SimpNetwork,
    node: SimpNodeId,
    input_level: usize,
    output_level: usize,
) -> Result<BoolExpr, SimpDcError> {
    network.node(node)?;
    let mut fanout_cone: HashSet<SimpNodeId> = transitive_fanout(network, node, INFINITY_LEVEL)?
        .into_iter()
        .collect();
    fanout_cone.insert(node);

    let mut dc_nodes = HashSet::new();
    for fanin in transitive_fanin(network, node, input_level)? {
        dc_nodes.insert(fanin);
        for np in transitive_fanout(network, fanin, output_level)? {
            if !fanout_cone.contains(&np) {
                dc_nodes.insert(np);
            }
        }
    }

    let mut ordered: Vec<_> = dc_nodes.into_iter().collect();
    ordered.sort();

    let mut dc = BoolExpr::zero();
    for candidate in ordered {
        if network.node(candidate)?.kind == SimpNodeKind::Internal {
            dc = BoolExpr::or(dc, simp_di(candidate));
        }
    }
    Ok(dc)
}

pub fn simp_all_dc(network: &SimpNetwork, node: SimpNodeId) -> Result<BoolExpr, SimpDcError> {
    Ok(BoolExpr::or(
        simp_tfanin_dc(network, node, INFINITY_LEVEL, INFINITY_LEVEL)?,
        simp_fanout_dc(network, node)?,
    ))
}

pub fn simp_sub_fanin_dc(network: &SimpNetwork, node: SimpNodeId) -> Result<BoolExpr, SimpDcError> {
    let selected = filtered_sub_fanin_nodes(network, node, |_| Ok(true))?;
    Ok(or_discrepancy_terms(selected.into_iter()))
}

pub fn simp_level_dc(network: &SimpNetwork, node: SimpNodeId) -> Result<BoolExpr, SimpDcError> {
    let node_level = network
        .node(node)?
        .level
        .ok_or(SimpDcError::MissingLevel(node))?;
    let selected = filtered_sub_fanin_nodes(network, node, |candidate| {
        let candidate_level = network
            .node(candidate)?
            .level
            .ok_or(SimpDcError::MissingLevel(candidate))?;
        Ok(candidate_level < node_level)
    })?;
    Ok(or_discrepancy_terms(selected.into_iter()))
}

pub fn sub_fanin_candidates(
    network: &SimpNetwork,
    node: SimpNodeId,
) -> Result<Vec<SimpNodeId>, SimpDcError> {
    let target = network.node(node)?;
    let support: HashSet<_> = target.fanins.iter().copied().collect();
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    for fanin in target.fanins.iter().copied() {
        push_unique(&mut candidates, &mut seen, fanin);
        for fanout in transitive_fanout(network, fanin, INFINITY_LEVEL)? {
            if network.node(fanout)?.kind == SimpNodeKind::Internal {
                push_unique(&mut candidates, &mut seen, fanout);
            }
        }
    }

    let mut retained = Vec::new();
    for candidate in candidates {
        if candidate == node {
            continue;
        }
        let candidate_node = network.node(candidate)?;
        if candidate_node
            .fanins
            .iter()
            .all(|fanin| support.contains(fanin))
        {
            retained.push(candidate);
        }
    }
    Ok(retained)
}

pub fn shared_fanin_fanouts(
    network: &SimpNetwork,
    node: SimpNodeId,
) -> Result<Vec<SimpNodeId>, SimpDcError> {
    let mut nodefanouts = simp_nodefanout_vec(network, node)?;
    for fanin in network.node(node)?.fanins.iter().copied() {
        nodefanouts.extend(simp_nodefanout_vec(network, fanin)?);
    }
    nodefanouts.sort_by_key(|entry| (entry.node, entry.fanout));

    let mut table = HashSet::new();
    for pair in nodefanouts.windows(2) {
        let current = pair[0];
        let next = pair[1];
        if current.node == next.node {
            table.insert(current.fanout);
            table.insert(next.fanout);
        }
    }

    let mut fanouts: Vec<_> = table.into_iter().collect();
    fanouts.sort();
    Ok(fanouts)
}

pub fn simp_nodefanout_vec(
    network: &SimpNetwork,
    node: SimpNodeId,
) -> Result<Vec<NodeFanout>, SimpDcError> {
    Ok(network
        .node(node)?
        .fanins
        .iter()
        .copied()
        .map(|fanin| NodeFanout {
            node: fanin,
            fanout: node,
        })
        .collect())
}

pub fn transitive_fanin(
    network: &SimpNetwork,
    node: SimpNodeId,
    limit: usize,
) -> Result<Vec<SimpNodeId>, SimpDcError> {
    walk_cone(network, node, limit, ConeDirection::Fanin)
}

pub fn transitive_fanout(
    network: &SimpNetwork,
    node: SimpNodeId,
    limit: usize,
) -> Result<Vec<SimpNodeId>, SimpDcError> {
    walk_cone(network, node, limit, ConeDirection::Fanout)
}

fn filtered_sub_fanin_nodes<F>(
    network: &SimpNetwork,
    node: SimpNodeId,
    keep: F,
) -> Result<Vec<SimpNodeId>, SimpDcError>
where
    F: Fn(SimpNodeId) -> Result<bool, SimpDcError>,
{
    let target = network.node(node)?;
    let fanin_count = target.fanins.len();
    let target_cube_count = target.cube_count;
    let mut dc_list = sub_fanin_candidates(network, node)?;
    dc_list.sort_by_key(|id| {
        network
            .node(*id)
            .map(|node| node.cube_count)
            .unwrap_or(usize::MAX)
    });

    let mut selected = Vec::new();
    let mut accumulated_literals = 0;
    for candidate in dc_list {
        let candidate_node = network.node(candidate)?;
        if candidate_node.literal_count == 1
            || candidate_node.cube_count > 100
            || candidate_node.cube_count > 2 * target_cube_count
        {
            continue;
        }
        if fanin_count > 15 && accumulated_literals * fanin_count > 6000 {
            break;
        }
        if fanin_count >= 50 && accumulated_literals * fanin_count > 3000 {
            break;
        }
        if candidate_node.kind == SimpNodeKind::Internal && keep(candidate)? {
            selected.push(candidate);
            accumulated_literals += candidate_node.literal_count;
        }
    }
    Ok(selected)
}

fn or_discrepancy_terms(nodes: impl Iterator<Item = SimpNodeId>) -> BoolExpr {
    nodes.map(simp_di).fold(BoolExpr::zero(), BoolExpr::or)
}

fn push_unique(list: &mut Vec<SimpNodeId>, seen: &mut HashSet<SimpNodeId>, node: SimpNodeId) {
    if seen.insert(node) {
        list.push(node);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConeDirection {
    Fanin,
    Fanout,
}

fn walk_cone(
    network: &SimpNetwork,
    node: SimpNodeId,
    limit: usize,
    direction: ConeDirection,
) -> Result<Vec<SimpNodeId>, SimpDcError> {
    network.node(node)?;
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back((node, 0usize));

    while let Some((current, depth)) = queue.pop_front() {
        if depth == limit {
            continue;
        }
        let neighbors = match direction {
            ConeDirection::Fanin => &network.node(current)?.fanins,
            ConeDirection::Fanout => &network.node(current)?.fanouts,
        };
        for neighbor in neighbors.iter().copied() {
            network.node(neighbor)?;
            if seen.insert(neighbor) {
                ordered.push(neighbor);
                queue.push_back((neighbor, depth.saturating_add(1)));
            }
        }
    }

    Ok(ordered)
}

pub fn level_table_from_nodes(network: &SimpNetwork) -> HashMap<SimpNodeId, i32> {
    network
        .nodes()
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.level.map(|level| (SimpNodeId(index), level)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(id: SimpNodeId) -> BoolExpr {
        BoolExpr::positive_literal(id)
    }

    fn internal(name: &str, expr: BoolExpr, cubes: usize, literals: usize) -> SimpNode {
        SimpNode::new(name, SimpNodeKind::Internal, expr).with_cost(cubes, literals)
    }

    #[test]
    fn simp_di_xors_node_function_with_positive_literal() {
        let node = SimpNodeId(3);

        assert_eq!(
            simp_di(node),
            BoolExpr::Xor(
                Box::new(BoolExpr::NodeFunction(node)),
                Box::new(BoolExpr::Literal {
                    node,
                    positive: true
                })
            )
        );
    }

    #[test]
    fn shared_fanin_fanouts_match_simp_table_duplicate_rule() {
        let mut network = SimpNetwork::new();
        let a = network.add_node(SimpNode::new(
            "a",
            SimpNodeKind::PrimaryInput,
            BoolExpr::one(),
        ));
        let b = network.add_node(SimpNode::new(
            "b",
            SimpNodeKind::PrimaryInput,
            BoolExpr::one(),
        ));
        let x = network.add_node(internal("x", lit(a), 1, 2));
        let y = network.add_node(internal("y", lit(a), 1, 2));
        let f = network.add_node(internal("f", BoolExpr::and(lit(x), lit(y)), 2, 4));
        network.connect(a, x).unwrap();
        network.connect(b, x).unwrap();
        network.connect(a, y).unwrap();
        network.connect(x, f).unwrap();
        network.connect(y, f).unwrap();

        assert_eq!(shared_fanin_fanouts(&network, f).unwrap(), vec![x, y]);
        assert_eq!(
            unused_simp_fanin_dc(&network, f).unwrap(),
            BoolExpr::or(simp_di(x), simp_di(y))
        );
    }

    #[test]
    fn fanout_dc_is_zero_when_node_feeds_primary_output() {
        let mut network = SimpNetwork::new();
        let f = network.add_node(internal("f", BoolExpr::one(), 1, 1));
        let out = network.add_node(SimpNode::new("out", SimpNodeKind::PrimaryOutput, lit(f)));
        network.connect(f, out).unwrap();

        assert_eq!(simp_fanout_dc(&network, f).unwrap(), BoolExpr::zero());
    }

    #[test]
    fn fanout_dc_conjoins_xnor_of_positive_and_negative_cofactors() {
        let mut network = SimpNetwork::new();
        let f = network.add_node(internal("f", BoolExpr::one(), 1, 1));
        let a = network.add_node(SimpNode::new(
            "a",
            SimpNodeKind::PrimaryInput,
            BoolExpr::one(),
        ));
        let g1 = network.add_node(internal("g1", BoolExpr::and(lit(f), lit(a)), 1, 2));
        let g2 = network.add_node(internal("g2", BoolExpr::or(lit(f), lit(a)), 1, 2));
        network.connect(f, g1).unwrap();
        network.connect(a, g1).unwrap();
        network.connect(f, g2).unwrap();
        network.connect(a, g2).unwrap();

        assert_eq!(
            simp_fanout_dc(&network, f).unwrap(),
            BoolExpr::and(
                BoolExpr::xnor(lit(a), BoolExpr::zero()),
                BoolExpr::xnor(BoolExpr::one(), lit(a))
            )
        );
    }

    #[test]
    fn tfanin_dc_adds_internal_nodes_from_fanin_and_selected_fanout_cones() {
        let mut network = SimpNetwork::new();
        let a = network.add_node(SimpNode::new(
            "a",
            SimpNodeKind::PrimaryInput,
            BoolExpr::one(),
        ));
        let b = network.add_node(SimpNode::new(
            "b",
            SimpNodeKind::PrimaryInput,
            BoolExpr::one(),
        ));
        let x = network.add_node(internal("x", lit(a), 2, 3));
        let y = network.add_node(internal("y", lit(a), 2, 3));
        let f = network.add_node(internal("f", BoolExpr::and(lit(x), lit(b)), 3, 5));
        let z = network.add_node(internal("z", lit(a), 2, 3));
        network.connect(a, x).unwrap();
        network.connect(x, f).unwrap();
        network.connect(a, y).unwrap();
        network.connect(y, f).unwrap();
        network.connect(a, z).unwrap();
        network.connect(b, f).unwrap();

        assert_eq!(
            simp_tfanin_dc(&network, f, 1, INFINITY_LEVEL).unwrap(),
            BoolExpr::or(simp_di(x), simp_di(y))
        );
        assert!(
            !format!(
                "{:?}",
                simp_tfanin_dc(&network, f, 1, INFINITY_LEVEL).unwrap()
            )
            .contains(&format!("{:?}", z))
        );
    }

    #[test]
    fn sub_fanin_candidates_keep_same_or_subset_support() {
        let mut network = SimpNetwork::new();
        let a = network.add_node(SimpNode::new(
            "a",
            SimpNodeKind::PrimaryInput,
            BoolExpr::one(),
        ));
        let b = network.add_node(SimpNode::new(
            "b",
            SimpNodeKind::PrimaryInput,
            BoolExpr::one(),
        ));
        let c = network.add_node(SimpNode::new(
            "c",
            SimpNodeKind::PrimaryInput,
            BoolExpr::one(),
        ));
        let x = network.add_node(internal("x", lit(a), 2, 3));
        let y = network.add_node(internal("y", BoolExpr::and(lit(a), lit(b)), 2, 4));
        let outside = network.add_node(internal("outside", lit(c), 2, 4));
        let f = network.add_node(internal("f", BoolExpr::and(lit(a), lit(b)), 5, 9));
        network.connect(a, x).unwrap();
        network.connect(a, y).unwrap();
        network.connect(b, y).unwrap();
        network.connect(c, outside).unwrap();
        network.connect(a, f).unwrap();
        network.connect(b, f).unwrap();
        network.connect(x, outside).unwrap();

        assert_eq!(sub_fanin_candidates(&network, f).unwrap(), vec![a, x, y, b]);
        assert_eq!(
            simp_sub_fanin_dc(&network, f).unwrap(),
            BoolExpr::or(simp_di(x), simp_di(y))
        );
    }

    #[test]
    fn sub_fanin_dc_applies_cost_limits_and_literal_budget() {
        let mut network = SimpNetwork::new();
        let inputs: Vec<_> = (0..16)
            .map(|i| {
                network.add_node(SimpNode::new(
                    format!("i{i}"),
                    SimpNodeKind::PrimaryInput,
                    BoolExpr::one(),
                ))
            })
            .collect();
        let f = network.add_node(internal("f", BoolExpr::one(), 60, 20));
        for input in &inputs {
            network.connect(*input, f).unwrap();
        }
        let small = network.add_node(internal("small", lit(inputs[0]), 2, 400));
        let too_many_cubes = network.add_node(internal("huge", lit(inputs[1]), 101, 300));
        let too_expensive = network.add_node(internal("expensive", lit(inputs[2]), 121, 300));
        let over_budget = network.add_node(internal("over_budget", lit(inputs[3]), 3, 200));
        for node in [small, too_many_cubes, too_expensive, over_budget] {
            network.connect(inputs[0], node).unwrap();
        }

        assert_eq!(simp_sub_fanin_dc(&network, f).unwrap(), simp_di(small));
    }

    #[test]
    fn level_dc_keeps_only_lower_level_candidates() {
        let mut network = SimpNetwork::new();
        let a = network.add_node(SimpNode::new(
            "a",
            SimpNodeKind::PrimaryInput,
            BoolExpr::one(),
        ));
        let lower = network.add_node(internal("lower", lit(a), 2, 3).with_level(1));
        let same = network.add_node(internal("same", lit(a), 2, 3).with_level(3));
        let f = network.add_node(internal("f", lit(a), 4, 8).with_level(3));
        network.connect(a, lower).unwrap();
        network.connect(a, same).unwrap();
        network.connect(a, f).unwrap();

        assert_eq!(simp_level_dc(&network, f).unwrap(), simp_di(lower));
    }

    #[test]
    fn missing_level_and_sis_dependency_errors_are_explicit() {
        let mut network = SimpNetwork::new();
        let f = network.add_node(internal("f", BoolExpr::one(), 1, 2));

        assert_eq!(
            simp_level_dc(&network, f),
            Err(SimpDcError::MissingLevel(f))
        );
        assert_eq!(
            sis_node_bound_dont_care("simp_tfanin_dc"),
            Err(SimpDcError::MissingSisPorts {
                operation: "simp_tfanin_dc",
            })
        );
    }
}
