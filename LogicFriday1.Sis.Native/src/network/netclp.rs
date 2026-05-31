//! Native Rust network collapse.
//!
//! SIS collapses a network by visiting nodes in depth-first order, repeatedly
//! collapsing each internal node's internal fanins into the node, and finally
//! cleaning up internal nodes that no longer drive anything. This module keeps
//! that behavior in an owned Rust graph model without exposing a C ABI layer.

use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NetclpNodeId(pub usize);

impl NetclpNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetclpNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetclpLiteral {
    Zero,
    One,
    DontCare,
}

impl NetclpLiteral {
    fn from_value(value: bool) -> Self {
        if value {
            Self::One
        } else {
            Self::Zero
        }
    }

    fn matches(self, value: bool) -> bool {
        match self {
            Self::Zero => !value,
            Self::One => value,
            Self::DontCare => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetclpCube {
    literals: Vec<NetclpLiteral>,
}

impl NetclpCube {
    pub fn new(literals: impl Into<Vec<NetclpLiteral>>) -> Self {
        Self {
            literals: literals.into(),
        }
    }

    pub fn one(input_count: usize) -> Self {
        Self {
            literals: vec![NetclpLiteral::DontCare; input_count],
        }
    }

    pub fn literals(&self) -> &[NetclpLiteral] {
        &self.literals
    }

    fn matches_assignment(&self, assignment: &[bool]) -> bool {
        self.literals
            .iter()
            .zip(assignment)
            .all(|(literal, value)| literal.matches(*value))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetclpNode {
    pub name: String,
    pub kind: NetclpNodeKind,
    pub fanins: Vec<NetclpNodeId>,
    pub fanouts: BTreeSet<NetclpNodeId>,
    pub cover: Vec<NetclpCube>,
}

impl NetclpNode {
    pub fn new(name: impl Into<String>, kind: NetclpNodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: BTreeSet::new(),
            cover: Vec::new(),
        }
    }

    pub fn with_cover(
        mut self,
        fanins: impl Into<Vec<NetclpNodeId>>,
        cover: impl Into<Vec<NetclpCube>>,
    ) -> Self {
        self.fanins = fanins.into();
        self.cover = cover.into();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetclpError {
    MissingNode(NetclpNodeId),
    MissingFanin {
        node: NetclpNodeId,
        fanin: NetclpNodeId,
    },
    DuplicateNode(NetclpNodeId),
    InvalidPrimaryOutput(NetclpNodeId),
    CoverArityMismatch {
        node: NetclpNodeId,
        fanins: usize,
        literals: usize,
    },
    CycleDetected(NetclpNodeId),
    TooManyFanins(usize),
}

impl fmt::Display for NetclpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "missing network node {}", node.index()),
            Self::MissingFanin { node, fanin } => write!(
                formatter,
                "network node {} references missing fanin {}",
                node.index(),
                fanin.index()
            ),
            Self::DuplicateNode(node) => {
                write!(formatter, "duplicate network node {}", node.index())
            }
            Self::InvalidPrimaryOutput(node) => write!(
                formatter,
                "primary output {} must have exactly one fanin",
                node.index()
            ),
            Self::CoverArityMismatch {
                node,
                fanins,
                literals,
            } => write!(
                formatter,
                "node {} has {fanins} fanins but a cube with {literals} literals",
                node.index()
            ),
            Self::CycleDetected(node) => {
                write!(
                    formatter,
                    "network contains a cycle through node {}",
                    node.index()
                )
            }
            Self::TooManyFanins(count) => {
                write!(
                    formatter,
                    "cannot enumerate collapse cover for {count} fanins"
                )
            }
        }
    }
}

impl Error for NetclpError {}

pub type NetclpResult<T> = Result<T, NetclpError>;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct NetclpNetwork {
    nodes: Vec<Option<NetclpNode>>,
    order: Vec<NetclpNodeId>,
}

impl NetclpNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, mut node: NetclpNode) -> NetclpResult<NetclpNodeId> {
        let id = NetclpNodeId(self.nodes.len());
        validate_node_shape(id, &node)?;
        for fanin in &node.fanins {
            self.node(*fanin).map_err(|_| NetclpError::MissingFanin {
                node: id,
                fanin: *fanin,
            })?;
        }

        node.fanouts.clear();
        self.nodes.push(Some(node));
        self.order.push(id);
        self.rebuild_fanouts()?;
        Ok(id)
    }

    pub fn node(&self, node: NetclpNodeId) -> NetclpResult<&NetclpNode> {
        self.nodes
            .get(node.index())
            .and_then(Option::as_ref)
            .ok_or(NetclpError::MissingNode(node))
    }

    pub fn node_mut(&mut self, node: NetclpNodeId) -> NetclpResult<&mut NetclpNode> {
        self.nodes
            .get_mut(node.index())
            .and_then(Option::as_mut)
            .ok_or(NetclpError::MissingNode(node))
    }

    pub fn nodes(&self) -> impl Iterator<Item = (NetclpNodeId, &NetclpNode)> {
        self.order.iter().filter_map(|id| {
            self.nodes
                .get(id.index())
                .and_then(Option::as_ref)
                .map(|node| (*id, node))
        })
    }

    pub fn network_collapse(&mut self) -> NetclpResult<bool> {
        let mut changed = false;
        let order = self.depth_first_order()?;

        for node in order {
            if self.node(node)?.kind == NetclpNodeKind::Internal
                && self.network_collapse_single(node)?
            {
                changed = true;
            }
        }

        if self.network_cleanup()? {
            changed = true;
        }

        Ok(changed)
    }

    pub fn network_collapse_single(&mut self, node: NetclpNodeId) -> NetclpResult<bool> {
        if self.node(node)?.kind != NetclpNodeKind::Internal {
            return Ok(false);
        }

        let mut changed = false;
        loop {
            let fanins = self.node(node)?.fanins.clone();
            let mut collapsed = false;
            for fanin in fanins {
                if self.collapse_fanin(node, fanin)? {
                    changed = true;
                    collapsed = true;
                    break;
                }
            }

            if !collapsed {
                return Ok(changed);
            }
        }
    }

    pub fn network_cleanup(&mut self) -> NetclpResult<bool> {
        let mut changed = false;
        loop {
            let removable = self
                .nodes()
                .find(|(_, node)| node.kind == NetclpNodeKind::Internal && node.fanouts.is_empty())
                .map(|(id, _)| id);

            let Some(node) = removable else {
                return Ok(changed);
            };

            self.nodes[node.index()] = None;
            self.order.retain(|candidate| *candidate != node);
            self.rebuild_fanouts()?;
            changed = true;
        }
    }

    pub fn evaluate_node(
        &self,
        node: NetclpNodeId,
        values: &HashMap<NetclpNodeId, bool>,
    ) -> NetclpResult<bool> {
        let mut memo = HashMap::new();
        self.evaluate_node_inner(node, values, &mut memo, &mut BTreeSet::new())
    }

    fn collapse_fanin(&mut self, node: NetclpNodeId, fanin: NetclpNodeId) -> NetclpResult<bool> {
        let target = self.node(node)?.clone();
        let collapsed = self.node(fanin)?.clone();
        if target.kind != NetclpNodeKind::Internal
            || collapsed.kind != NetclpNodeKind::Internal
            || !target.fanins.contains(&fanin)
        {
            return Ok(false);
        }

        let replacement_fanins = replacement_fanin_base(&target, &collapsed, fanin);
        if replacement_fanins.len() >= usize::BITS as usize {
            return Err(NetclpError::TooManyFanins(replacement_fanins.len()));
        }

        let cover = collapsed_cover(&target, &collapsed, fanin, &replacement_fanins)?;
        let target = self.node_mut(node)?;
        target.fanins = replacement_fanins;
        target.cover = cover;
        self.rebuild_fanouts()?;
        Ok(true)
    }

    fn depth_first_order(&self) -> NetclpResult<Vec<NetclpNodeId>> {
        let mut roots = self
            .nodes()
            .filter(|(_, node)| node.kind == NetclpNodeKind::PrimaryOutput)
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        roots.extend(
            self.nodes()
                .filter(|(_, node)| {
                    node.kind != NetclpNodeKind::PrimaryOutput && node.fanouts.is_empty()
                })
                .map(|(id, _)| id),
        );

        let mut visited = HashMap::new();
        let mut order = Vec::new();
        for root in roots {
            self.depth_first_visit(root, &mut visited, &mut order)?;
        }
        for (node, _) in self.nodes() {
            if !visited.contains_key(&node) {
                self.depth_first_visit(node, &mut visited, &mut order)?;
            }
        }
        Ok(order)
    }

    fn depth_first_visit(
        &self,
        node: NetclpNodeId,
        visited: &mut HashMap<NetclpNodeId, VisitState>,
        order: &mut Vec<NetclpNodeId>,
    ) -> NetclpResult<()> {
        if let Some(state) = visited.get(&node) {
            return match state {
                VisitState::Active => Err(NetclpError::CycleDetected(node)),
                VisitState::Done => Ok(()),
            };
        }

        visited.insert(node, VisitState::Active);
        for fanin in self.node(node)?.fanins.clone() {
            self.depth_first_visit(fanin, visited, order)?;
        }
        visited.insert(node, VisitState::Done);
        order.push(node);
        Ok(())
    }

    fn rebuild_fanouts(&mut self) -> NetclpResult<()> {
        for node in self.nodes.iter_mut().flatten() {
            node.fanouts.clear();
        }

        let edges = self
            .nodes()
            .flat_map(|(node, data)| data.fanins.iter().copied().map(move |fanin| (node, fanin)))
            .collect::<Vec<_>>();

        for (node, fanin) in edges {
            let fanin_node = self
                .nodes
                .get_mut(fanin.index())
                .and_then(Option::as_mut)
                .ok_or(NetclpError::MissingFanin { node, fanin })?;
            fanin_node.fanouts.insert(node);
        }

        Ok(())
    }

    fn evaluate_node_inner(
        &self,
        node: NetclpNodeId,
        values: &HashMap<NetclpNodeId, bool>,
        memo: &mut HashMap<NetclpNodeId, bool>,
        active: &mut BTreeSet<NetclpNodeId>,
    ) -> NetclpResult<bool> {
        if let Some(value) = memo.get(&node) {
            return Ok(*value);
        }

        if !active.insert(node) {
            return Err(NetclpError::CycleDetected(node));
        }

        let node_data = self.node(node)?;
        let value = match node_data.kind {
            NetclpNodeKind::PrimaryInput => values
                .get(&node)
                .copied()
                .ok_or(NetclpError::MissingFanin { node, fanin: node })?,
            NetclpNodeKind::PrimaryOutput => {
                if node_data.fanins.len() != 1 {
                    return Err(NetclpError::InvalidPrimaryOutput(node));
                }
                self.evaluate_node_inner(node_data.fanins[0], values, memo, active)?
            }
            NetclpNodeKind::Internal => {
                let assignment = node_data
                    .fanins
                    .iter()
                    .map(|fanin| self.evaluate_node_inner(*fanin, values, memo, active))
                    .collect::<NetclpResult<Vec<_>>>()?;
                evaluate_cover(&node_data.cover, &assignment)
            }
        };

        active.remove(&node);
        memo.insert(node, value);
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Active,
    Done,
}

fn validate_node_shape(node: NetclpNodeId, data: &NetclpNode) -> NetclpResult<()> {
    if data.kind == NetclpNodeKind::PrimaryOutput && data.fanins.len() != 1 {
        return Err(NetclpError::InvalidPrimaryOutput(node));
    }

    for cube in &data.cover {
        if cube.literals().len() != data.fanins.len() {
            return Err(NetclpError::CoverArityMismatch {
                node,
                fanins: data.fanins.len(),
                literals: cube.literals().len(),
            });
        }
    }

    Ok(())
}

fn replacement_fanin_base(
    target: &NetclpNode,
    collapsed: &NetclpNode,
    collapsed_id: NetclpNodeId,
) -> Vec<NetclpNodeId> {
    let mut fanins = Vec::new();
    for fanin in &target.fanins {
        if *fanin == collapsed_id {
            for replacement in &collapsed.fanins {
                push_unique(&mut fanins, *replacement);
            }
        } else {
            push_unique(&mut fanins, *fanin);
        }
    }
    fanins
}

fn push_unique(fanins: &mut Vec<NetclpNodeId>, fanin: NetclpNodeId) {
    if !fanins.contains(&fanin) {
        fanins.push(fanin);
    }
}

fn collapsed_cover(
    target: &NetclpNode,
    collapsed: &NetclpNode,
    collapsed_id: NetclpNodeId,
    fanins: &[NetclpNodeId],
) -> NetclpResult<Vec<NetclpCube>> {
    let assignment_count = 1_usize << fanins.len();
    let mut cover = Vec::new();

    for mask in 0..assignment_count {
        let assignment = (0..fanins.len())
            .map(|index| (mask & (1_usize << index)) != 0)
            .collect::<Vec<_>>();

        if evaluates_collapsed(target, collapsed, collapsed_id, fanins, &assignment)? {
            cover.push(NetclpCube::new(
                assignment
                    .into_iter()
                    .map(NetclpLiteral::from_value)
                    .collect::<Vec<_>>(),
            ));
        }
    }

    Ok(cover)
}

fn evaluates_collapsed(
    target: &NetclpNode,
    collapsed: &NetclpNode,
    collapsed_id: NetclpNodeId,
    fanins: &[NetclpNodeId],
    assignment: &[bool],
) -> NetclpResult<bool> {
    let collapsed_value = evaluate_node_cover(collapsed, fanins, assignment)?;
    let target_assignment = target
        .fanins
        .iter()
        .map(|fanin| {
            if *fanin == collapsed_id {
                Ok(collapsed_value)
            } else {
                value_for_fanin(*fanin, fanins, assignment)
            }
        })
        .collect::<NetclpResult<Vec<_>>>()?;

    Ok(evaluate_cover(&target.cover, &target_assignment))
}

fn evaluate_node_cover(
    node: &NetclpNode,
    fanins: &[NetclpNodeId],
    assignment: &[bool],
) -> NetclpResult<bool> {
    let projected = node
        .fanins
        .iter()
        .map(|fanin| value_for_fanin(*fanin, fanins, assignment))
        .collect::<NetclpResult<Vec<_>>>()?;
    Ok(evaluate_cover(&node.cover, &projected))
}

fn value_for_fanin(
    fanin: NetclpNodeId,
    fanins: &[NetclpNodeId],
    assignment: &[bool],
) -> NetclpResult<bool> {
    fanins
        .iter()
        .position(|candidate| *candidate == fanin)
        .map(|index| assignment[index])
        .ok_or(NetclpError::MissingNode(fanin))
}

fn evaluate_cover(cover: &[NetclpCube], assignment: &[bool]) -> bool {
    cover.iter().any(|cube| cube.matches_assignment(assignment))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(literals: &[NetclpLiteral]) -> NetclpCube {
        NetclpCube::new(literals.to_vec())
    }

    fn values(values: &[(NetclpNodeId, bool)]) -> HashMap<NetclpNodeId, bool> {
        values.iter().copied().collect()
    }

    #[test]
    fn collapse_single_repeats_until_internal_fanins_are_removed() {
        let mut network = NetclpNetwork::new();
        let a = network
            .add_node(NetclpNode::new("a", NetclpNodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_node(NetclpNode::new("b", NetclpNodeKind::PrimaryInput))
            .unwrap();
        let c = network
            .add_node(NetclpNode::new("c", NetclpNodeKind::PrimaryInput))
            .unwrap();
        let ab = network
            .add_node(NetclpNode::new("ab", NetclpNodeKind::Internal).with_cover(
                vec![a, b],
                vec![cube(&[NetclpLiteral::One, NetclpLiteral::One])],
            ))
            .unwrap();
        let abc = network
            .add_node(NetclpNode::new("abc", NetclpNodeKind::Internal).with_cover(
                vec![ab, c],
                vec![cube(&[NetclpLiteral::One, NetclpLiteral::Zero])],
            ))
            .unwrap();

        assert!(network.network_collapse_single(abc).unwrap());

        assert_eq!(network.node(abc).unwrap().fanins, vec![a, b, c]);
        assert!(network
            .evaluate_node(abc, &values(&[(a, true), (b, true), (c, false)]))
            .unwrap());
        assert!(!network
            .evaluate_node(abc, &values(&[(a, true), (b, false), (c, false)]))
            .unwrap());
        assert!(!network
            .evaluate_node(abc, &values(&[(a, true), (b, true), (c, true)]))
            .unwrap());
    }

    #[test]
    fn network_collapse_uses_dfs_order_and_cleans_unused_internal_nodes() {
        let mut network = NetclpNetwork::new();
        let a = network
            .add_node(NetclpNode::new("a", NetclpNodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_node(NetclpNode::new("b", NetclpNodeKind::PrimaryInput))
            .unwrap();
        let ab = network
            .add_node(NetclpNode::new("ab", NetclpNodeKind::Internal).with_cover(
                vec![a, b],
                vec![cube(&[NetclpLiteral::One, NetclpLiteral::One])],
            ))
            .unwrap();
        let inv = network
            .add_node(
                NetclpNode::new("inv", NetclpNodeKind::Internal)
                    .with_cover(vec![ab], vec![cube(&[NetclpLiteral::Zero])]),
            )
            .unwrap();
        let output = network
            .add_node(
                NetclpNode::new("y", NetclpNodeKind::PrimaryOutput)
                    .with_cover(vec![inv], Vec::new()),
            )
            .unwrap();

        assert!(network.network_collapse().unwrap());

        assert_eq!(network.node(output).unwrap().fanins, vec![inv]);
        assert_eq!(network.node(inv).unwrap().fanins, vec![a, b]);
        assert!(matches!(network.node(ab), Err(NetclpError::MissingNode(_))));
        assert!(!network
            .evaluate_node(output, &values(&[(a, true), (b, true)]))
            .unwrap());
        assert!(network
            .evaluate_node(output, &values(&[(a, false), (b, true)]))
            .unwrap());
    }

    #[test]
    fn non_internal_nodes_do_not_collapse() {
        let mut network = NetclpNetwork::new();
        let input = network
            .add_node(NetclpNode::new("a", NetclpNodeKind::PrimaryInput))
            .unwrap();

        assert!(!network.network_collapse_single(input).unwrap());
        assert_eq!(
            network.node(input).unwrap().kind,
            NetclpNodeKind::PrimaryInput
        );
    }

    #[test]
    fn collapse_reports_cycles_before_mutating() {
        let mut network = NetclpNetwork::new();
        let first = network
            .add_node(NetclpNode::new("first", NetclpNodeKind::Internal))
            .unwrap();
        let second = network
            .add_node(
                NetclpNode::new("second", NetclpNodeKind::Internal)
                    .with_cover(vec![first], vec![cube(&[NetclpLiteral::One])]),
            )
            .unwrap();
        network.node_mut(first).unwrap().fanins.push(second);
        network
            .node_mut(first)
            .unwrap()
            .cover
            .push(cube(&[NetclpLiteral::One]));
        network.rebuild_fanouts().unwrap();

        assert!(matches!(
            network.network_collapse(),
            Err(NetclpError::CycleDetected(_))
        ));
    }
}
