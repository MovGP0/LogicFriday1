//! Native Rust mapper tree model for `sis/map/tree.c`.
//!
//! The original mapper tree is an internal representation used before
//! library tree matching. This module keeps that data as an owned Rust graph:
//! nodes are either named leaves or primitive gates with ordered fanins, and
//! callers can validate and traverse the tree deterministically. It intentionally
//! exposes no legacy C ABI entry points.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MapperTreeNodeId(usize);

impl MapperTreeNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveGateKind {
    Buffer,
    Inverter,
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Xnor,
    One,
    Zero,
}

impl PrimitiveGateKind {
    pub fn is_constant(self) -> bool {
        matches!(self, Self::One | Self::Zero)
    }

    fn arity_rule(self) -> ArityRule {
        match self {
            Self::One | Self::Zero => ArityRule::Exact(0),
            Self::Buffer | Self::Inverter => ArityRule::Exact(1),
            Self::And | Self::Nand | Self::Or | Self::Nor | Self::Xor | Self::Xnor => {
                ArityRule::AtLeast(2)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapperTreeFanin {
    pub node: MapperTreeNodeId,
    pub inverted: bool,
}

impl MapperTreeFanin {
    pub fn new(node: MapperTreeNodeId) -> Self {
        Self {
            node,
            inverted: false,
        }
    }

    pub fn inverted(node: MapperTreeNodeId) -> Self {
        Self {
            node,
            inverted: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapperTreeNode {
    Leaf {
        name: String,
    },
    Gate {
        kind: PrimitiveGateKind,
        fanins: Vec<MapperTreeFanin>,
    },
}

impl MapperTreeNode {
    pub fn leaf(name: impl Into<String>) -> Self {
        Self::Leaf { name: name.into() }
    }

    pub fn gate(kind: PrimitiveGateKind, fanins: Vec<MapperTreeFanin>) -> Self {
        Self::Gate { kind, fanins }
    }

    pub fn fanins(&self) -> &[MapperTreeFanin] {
        match self {
            Self::Leaf { .. } => &[],
            Self::Gate { fanins, .. } => fanins,
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapperTree {
    nodes: Vec<MapperTreeNode>,
    root: MapperTreeNodeId,
}

impl MapperTree {
    pub fn new(
        root: MapperTreeNodeId,
        nodes: Vec<MapperTreeNode>,
    ) -> Result<Self, MapperTreeError> {
        let tree = Self { nodes, root };
        tree.validate()?;
        Ok(tree)
    }

    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            root: MapperTreeNodeId(0),
        }
    }

    pub fn add_leaf(&mut self, name: impl Into<String>) -> MapperTreeNodeId {
        self.push_node(MapperTreeNode::leaf(name))
    }

    pub fn add_gate(
        &mut self,
        kind: PrimitiveGateKind,
        fanins: Vec<MapperTreeFanin>,
    ) -> MapperTreeNodeId {
        self.push_node(MapperTreeNode::gate(kind, fanins))
    }

    pub fn set_root(&mut self, root: MapperTreeNodeId) {
        self.root = root;
    }

    pub fn root(&self) -> MapperTreeNodeId {
        self.root
    }

    pub fn node(&self, id: MapperTreeNodeId) -> Option<&MapperTreeNode> {
        self.nodes.get(id.index())
    }

    pub fn nodes(&self) -> &[MapperTreeNode] {
        &self.nodes
    }

    pub fn validate(&self) -> Result<(), MapperTreeError> {
        if self.nodes.is_empty() {
            return Err(MapperTreeError::EmptyTree);
        }
        self.require_node(self.root)?;

        for (index, node) in self.nodes.iter().enumerate() {
            let id = MapperTreeNodeId(index);
            match node {
                MapperTreeNode::Leaf { name } if name.is_empty() => {
                    return Err(MapperTreeError::EmptyLeafName { node: id });
                }
                MapperTreeNode::Leaf { .. } => {}
                MapperTreeNode::Gate { kind, fanins } => {
                    validate_arity(id, *kind, fanins.len())?;
                    for fanin in fanins {
                        self.require_node(fanin.node)?;
                    }
                }
            }
        }

        self.preorder()?;
        Ok(())
    }

    pub fn depth(&self) -> Result<usize, MapperTreeError> {
        let mut state = vec![VisitState::Unvisited; self.nodes.len()];
        self.depth_from(self.root, &mut state)
    }

    pub fn leaf_ids(&self) -> Result<Vec<MapperTreeNodeId>, MapperTreeError> {
        let mut leaves = BTreeSet::new();
        self.collect_leaves(self.root, &mut Vec::new(), &mut leaves)?;
        Ok(leaves.into_iter().collect())
    }

    pub fn leaves(&self) -> Result<Vec<(MapperTreeNodeId, &str)>, MapperTreeError> {
        self.leaf_ids()?
            .into_iter()
            .map(|id| match self.require_node(id)? {
                MapperTreeNode::Leaf { name } => Ok((id, name.as_str())),
                MapperTreeNode::Gate { .. } => Err(MapperTreeError::ExpectedLeaf { node: id }),
            })
            .collect()
    }

    pub fn preorder(&self) -> Result<Vec<MapperTreeNodeId>, MapperTreeError> {
        let mut state = vec![VisitState::Unvisited; self.nodes.len()];
        let mut order = Vec::new();
        self.preorder_from(self.root, &mut state, &mut order)?;
        Ok(order)
    }

    pub fn postorder(&self) -> Result<Vec<MapperTreeNodeId>, MapperTreeError> {
        let mut state = vec![VisitState::Unvisited; self.nodes.len()];
        let mut order = Vec::new();
        self.postorder_from(self.root, &mut state, &mut order)?;
        Ok(order)
    }

    fn push_node(&mut self, node: MapperTreeNode) -> MapperTreeNodeId {
        let id = MapperTreeNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    fn require_node(&self, id: MapperTreeNodeId) -> Result<&MapperTreeNode, MapperTreeError> {
        self.node(id)
            .ok_or(MapperTreeError::MissingNode { node: id })
    }

    fn depth_from(
        &self,
        id: MapperTreeNodeId,
        state: &mut [VisitState],
    ) -> Result<usize, MapperTreeError> {
        enter_node(id, state)?;
        let depth = match self.require_node(id)? {
            MapperTreeNode::Leaf { .. } => 0,
            MapperTreeNode::Gate { fanins, .. } => {
                let mut depth = 0;
                for fanin in fanins {
                    depth = depth.max(self.depth_from(fanin.node, state)? + 1);
                }
                depth
            }
        };
        leave_node(id, state);
        Ok(depth)
    }

    fn collect_leaves(
        &self,
        id: MapperTreeNodeId,
        stack: &mut Vec<MapperTreeNodeId>,
        leaves: &mut BTreeSet<MapperTreeNodeId>,
    ) -> Result<(), MapperTreeError> {
        if stack.contains(&id) {
            return Err(MapperTreeError::CycleDetected { node: id });
        }

        stack.push(id);
        match self.require_node(id)? {
            MapperTreeNode::Leaf { .. } => {
                leaves.insert(id);
            }
            MapperTreeNode::Gate { fanins, .. } => {
                for fanin in fanins {
                    self.collect_leaves(fanin.node, stack, leaves)?;
                }
            }
        }
        stack.pop();

        Ok(())
    }

    fn preorder_from(
        &self,
        id: MapperTreeNodeId,
        state: &mut [VisitState],
        order: &mut Vec<MapperTreeNodeId>,
    ) -> Result<(), MapperTreeError> {
        enter_node(id, state)?;
        order.push(id);
        for fanin in self.require_node(id)?.fanins() {
            self.preorder_from(fanin.node, state, order)?;
        }
        leave_node(id, state);
        Ok(())
    }

    fn postorder_from(
        &self,
        id: MapperTreeNodeId,
        state: &mut [VisitState],
        order: &mut Vec<MapperTreeNodeId>,
    ) -> Result<(), MapperTreeError> {
        enter_node(id, state)?;
        for fanin in self.require_node(id)?.fanins() {
            self.postorder_from(fanin.node, state, order)?;
        }
        order.push(id);
        leave_node(id, state);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapperTreeError {
    EmptyTree,
    MissingNode {
        node: MapperTreeNodeId,
    },
    EmptyLeafName {
        node: MapperTreeNodeId,
    },
    InvalidArity {
        node: MapperTreeNodeId,
        kind: PrimitiveGateKind,
        expected: &'static str,
        actual: usize,
    },
    CycleDetected {
        node: MapperTreeNodeId,
    },
    ExpectedLeaf {
        node: MapperTreeNodeId,
    },
}

impl fmt::Display for MapperTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTree => write!(f, "mapper tree has no nodes"),
            Self::MissingNode { node } => write!(f, "missing mapper tree node {}", node.index()),
            Self::EmptyLeafName { node } => {
                write!(f, "mapper tree leaf {} has an empty name", node.index())
            }
            Self::InvalidArity {
                node,
                kind,
                expected,
                actual,
            } => write!(
                f,
                "mapper tree node {} has invalid {kind:?} arity {actual}; expected {expected}",
                node.index()
            ),
            Self::CycleDetected { node } => {
                write!(f, "mapper tree contains a cycle at node {}", node.index())
            }
            Self::ExpectedLeaf { node } => {
                write!(f, "mapper tree node {} is not a leaf", node.index())
            }
        }
    }
}

impl Error for MapperTreeError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArityRule {
    Exact(usize),
    AtLeast(usize),
}

impl ArityRule {
    fn accepts(self, value: usize) -> bool {
        match self {
            Self::Exact(expected) => value == expected,
            Self::AtLeast(expected) => value >= expected,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Exact(0) => "0",
            Self::Exact(1) => "1",
            Self::Exact(_) => "the exact primitive arity",
            Self::AtLeast(2) => "at least 2",
            Self::AtLeast(_) => "at least the primitive minimum arity",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn validate_arity(
    node: MapperTreeNodeId,
    kind: PrimitiveGateKind,
    actual: usize,
) -> Result<(), MapperTreeError> {
    let rule = kind.arity_rule();
    if rule.accepts(actual) {
        return Ok(());
    }

    Err(MapperTreeError::InvalidArity {
        node,
        kind,
        expected: rule.label(),
        actual,
    })
}

fn enter_node(id: MapperTreeNodeId, state: &mut [VisitState]) -> Result<(), MapperTreeError> {
    let Some(slot) = state.get_mut(id.index()) else {
        return Err(MapperTreeError::MissingNode { node: id });
    };

    match *slot {
        VisitState::Unvisited => {
            *slot = VisitState::Visiting;
            Ok(())
        }
        VisitState::Visiting => Err(MapperTreeError::CycleDetected { node: id }),
        VisitState::Visited => Ok(()),
    }
}

fn leave_node(id: MapperTreeNodeId, state: &mut [VisitState]) {
    if let Some(slot) = state.get_mut(id.index()) {
        *slot = VisitState::Visited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> MapperTree {
        let mut tree = MapperTree::empty();
        let a = tree.add_leaf("a");
        let b = tree.add_leaf("b");
        let c = tree.add_leaf("c");
        let and = tree.add_gate(
            PrimitiveGateKind::And,
            vec![MapperTreeFanin::new(a), MapperTreeFanin::inverted(b)],
        );
        let root = tree.add_gate(
            PrimitiveGateKind::Or,
            vec![MapperTreeFanin::new(and), MapperTreeFanin::new(c)],
        );
        tree.set_root(root);
        tree.validate().unwrap();
        tree
    }

    #[test]
    fn traverses_from_root_in_deterministic_fanin_order() {
        let tree = sample_tree();

        assert_eq!(
            tree.preorder().unwrap(),
            vec![
                MapperTreeNodeId(4),
                MapperTreeNodeId(3),
                MapperTreeNodeId(0),
                MapperTreeNodeId(1),
                MapperTreeNodeId(2),
            ]
        );
        assert_eq!(
            tree.postorder().unwrap(),
            vec![
                MapperTreeNodeId(0),
                MapperTreeNodeId(1),
                MapperTreeNodeId(3),
                MapperTreeNodeId(2),
                MapperTreeNodeId(4),
            ]
        );
    }

    #[test]
    fn preserves_fanin_inversion_metadata() {
        let tree = sample_tree();
        let MapperTreeNode::Gate { fanins, .. } = tree.node(MapperTreeNodeId(3)).unwrap() else {
            panic!("expected gate node");
        };

        assert_eq!(fanins[0], MapperTreeFanin::new(MapperTreeNodeId(0)));
        assert_eq!(fanins[1], MapperTreeFanin::inverted(MapperTreeNodeId(1)));
    }

    #[test]
    fn rejects_missing_nodes_bad_arity_and_empty_leaf_names() {
        assert_eq!(
            MapperTree::new(MapperTreeNodeId(0), vec![MapperTreeNode::leaf("")]).unwrap_err(),
            MapperTreeError::EmptyLeafName {
                node: MapperTreeNodeId(0),
            }
        );

        assert_eq!(
            MapperTree::new(
                MapperTreeNodeId(0),
                vec![MapperTreeNode::gate(
                    PrimitiveGateKind::Inverter,
                    Vec::new(),
                )],
            )
            .unwrap_err(),
            MapperTreeError::InvalidArity {
                node: MapperTreeNodeId(0),
                kind: PrimitiveGateKind::Inverter,
                expected: "1",
                actual: 0,
            }
        );

        assert_eq!(
            MapperTree::new(
                MapperTreeNodeId(0),
                vec![MapperTreeNode::gate(
                    PrimitiveGateKind::Buffer,
                    vec![MapperTreeFanin::new(MapperTreeNodeId(99))],
                )],
            )
            .unwrap_err(),
            MapperTreeError::MissingNode {
                node: MapperTreeNodeId(99),
            }
        );
    }

    #[test]
    fn rejects_cycles() {
        let tree = MapperTree {
            nodes: vec![MapperTreeNode::gate(
                PrimitiveGateKind::Buffer,
                vec![MapperTreeFanin::new(MapperTreeNodeId(0))],
            )],
            root: MapperTreeNodeId(0),
        };

        assert_eq!(
            tree.validate().unwrap_err(),
            MapperTreeError::CycleDetected {
                node: MapperTreeNodeId(0),
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("tree.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
