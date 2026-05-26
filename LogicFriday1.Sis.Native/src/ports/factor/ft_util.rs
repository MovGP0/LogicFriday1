//! Native utilities for SIS factor trees.
//!
//! This module keeps the legacy first-child / next-sibling tree shape because
//! several factoring routines depend on that traversal order. The public API
//! uses owned Rust values and small graph adapters instead of raw node storage.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraversalOrder {
    InOrder,
    PostOrder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactorKind {
    Zero,
    One,
    And,
    Or,
    Inverter,
    Leaf,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorTree {
    pub kind: FactorKind,
    pub index: isize,
    pub len: usize,
    pub same_level: Option<Box<FactorTree>>,
    pub next_level: Option<Box<FactorTree>>,
}

impl FactorTree {
    pub fn new(kind: FactorKind, index: isize, len: usize) -> Self {
        Self {
            kind,
            index,
            len,
            same_level: None,
            next_level: None,
        }
    }

    pub fn leaf(index: usize) -> Self {
        Self::new(FactorKind::Leaf, index as isize, 0)
    }

    pub fn constant(value: bool) -> Self {
        Self::new(
            if value {
                FactorKind::One
            } else {
                FactorKind::Zero
            },
            -1,
            0,
        )
    }

    pub fn with_same_level(mut self, same_level: FactorTree) -> Self {
        self.same_level = Some(Box::new(same_level));
        self
    }

    pub fn with_next_level(mut self, next_level: FactorTree) -> Self {
        self.next_level = Some(Box::new(next_level));
        self
    }

    fn append_same_level(&mut self, sibling: FactorTree) {
        let mut cursor = &mut self.same_level;
        while let Some(next) = cursor {
            cursor = &mut next.same_level;
        }
        *cursor = Some(Box::new(sibling));
    }

    fn from_children(kind: FactorKind, children: Vec<FactorTree>) -> FactorResult<Self> {
        let mut iter = children.into_iter();
        let mut first = iter.next().ok_or(FactorError::EmptyFactorChildren(kind))?;
        for child in iter {
            first.append_same_level(child);
        }

        Ok(Self::new(kind, -1, 0).with_next_level(first))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Buffer,
    SumOfProducts,
    Input,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CubeLiteral {
    Zero,
    One,
    DontCare,
}

impl CubeLiteral {
    pub fn phase(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            Self::DontCare => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorNode {
    pub function: NodeFunction,
    fanins: Vec<NodeId>,
    cubes: Vec<Vec<CubeLiteral>>,
    factored: Option<FactorTree>,
}

impl FactorNode {
    pub fn input() -> Self {
        Self {
            function: NodeFunction::Input,
            fanins: Vec::new(),
            cubes: Vec::new(),
            factored: None,
        }
    }

    pub fn constant(value: bool) -> Self {
        Self {
            function: if value {
                NodeFunction::One
            } else {
                NodeFunction::Zero
            },
            fanins: Vec::new(),
            cubes: Vec::new(),
            factored: None,
        }
    }

    pub fn buffer(fanin: NodeId) -> Self {
        Self {
            function: NodeFunction::Buffer,
            fanins: vec![fanin],
            cubes: Vec::new(),
            factored: None,
        }
    }

    pub fn sum_of_products(
        fanins: impl Into<Vec<NodeId>>,
        cubes: impl Into<Vec<Vec<CubeLiteral>>>,
    ) -> Self {
        Self {
            function: NodeFunction::SumOfProducts,
            fanins: fanins.into(),
            cubes: cubes.into(),
            factored: None,
        }
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }

    pub fn cubes(&self) -> &[Vec<CubeLiteral>] {
        &self.cubes
    }

    pub fn factored(&self) -> Option<&FactorTree> {
        self.factored.as_ref()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FactorNetwork {
    nodes: Vec<FactorNode>,
}

impl FactorNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: FactorNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> FactorResult<&FactorNode> {
        self.nodes.get(id.0).ok_or(FactorError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> FactorResult<&mut FactorNode> {
        self.nodes.get_mut(id.0).ok_or(FactorError::UnknownNode(id))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FactorLiteral {
    pub node: NodeId,
    pub phase: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorExpr {
    Constant(bool),
    Input(NodeId),
    And(usize, usize),
    Or(usize, usize),
    Not(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeExpansion {
    pub root: usize,
    pub nodes: Vec<FactorExpr>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorError {
    UnknownNode(NodeId),
    FewerThanTwoFactorChildren(FactorKind),
    EmptyFactorChildren(FactorKind),
    EmptyProductTerm(NodeId),
    EmptyCover(NodeId),
    InvalidCubeWidth {
        node: NodeId,
        cube: usize,
        expected: usize,
        actual: usize,
    },
    MissingFanin {
        node: NodeId,
        fanin: NodeId,
    },
    InvalidLeafIndex(isize),
    MissingChild(FactorKind),
    UnknownFactorKind,
    MissingNativeIntegration {
        operation: &'static str,
    },
}

impl fmt::Display for FactorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown factor node {:?}", node),
            Self::FewerThanTwoFactorChildren(kind) => {
                write!(f, "factor {:?} requires at least two children", kind)
            }
            Self::EmptyFactorChildren(kind) => write!(f, "factor {:?} has no children", kind),
            Self::EmptyProductTerm(node) => write!(f, "node {:?} has an empty product term", node),
            Self::EmptyCover(node) => write!(f, "node {:?} has an empty cover", node),
            Self::InvalidCubeWidth {
                node,
                cube,
                expected,
                actual,
            } => write!(
                f,
                "node {:?} cube {} has width {}, expected {}",
                node, cube, actual, expected
            ),
            Self::MissingFanin { node, fanin } => {
                write!(f, "node {:?} is not a fanin of {:?}", fanin, node)
            }
            Self::InvalidLeafIndex(index) => write!(f, "invalid factor leaf index {}", index),
            Self::MissingChild(kind) => write!(f, "factor {:?} is missing its child", kind),
            Self::UnknownFactorKind => write!(f, "unknown factor kind"),
            Self::MissingNativeIntegration { operation } => {
                write!(f, "{operation} requires native factor integration")
            }
        }
    }
}

impl Error for FactorError {}

pub type FactorResult<T> = Result<T, FactorError>;

pub fn factor_alloc(node: &mut FactorNode) {
    node.factored = None;
}

pub fn factor_free(node: &mut FactorNode) {
    node.factored = None;
}

pub fn factor_invalid(node: &mut FactorNode) {
    factor_free(node);
}

pub fn factor_dup(old: &FactorNode, new: &mut FactorNode) {
    factor_free(new);
    new.factored = old.factored.clone();
}

pub fn factor_set(node: &mut FactorNode, tree: FactorTree) {
    node.factored = Some(tree);
}

pub fn factor_traverse<F>(
    node: NodeId,
    root: &FactorTree,
    order: TraversalOrder,
    mut visit: F,
) -> bool
where
    F: FnMut(NodeId, &FactorTree) -> bool,
{
    ft_traverse_recur(node, root, order, &mut visit)
}

pub fn ft_traverse_recur<F>(
    node: NodeId,
    root: &FactorTree,
    order: TraversalOrder,
    visit: &mut F,
) -> bool
where
    F: FnMut(NodeId, &FactorTree) -> bool,
{
    if order == TraversalOrder::InOrder && visit(node, root) {
        return true;
    }

    if let Some(child) = root.next_level.as_deref() {
        if ft_traverse_recur(node, child, order, visit) {
            return true;
        }
    }

    if let Some(sibling) = root.same_level.as_deref() {
        if ft_traverse_recur(node, sibling, order, visit) {
            return true;
        }
    }

    order == TraversalOrder::PostOrder && visit(node, root)
}

pub fn factor_nt_to_ft(
    network: &FactorNetwork,
    original: NodeId,
    root: NodeId,
) -> FactorResult<FactorTree> {
    match network.node(root)?.function {
        NodeFunction::Zero => Ok(FactorTree::constant(false)),
        NodeFunction::One => Ok(FactorTree::constant(true)),
        _ => node_to_ft(network, original, root),
    }
}

pub fn factor_nt_free(network: &mut FactorNetwork, root: NodeId) -> FactorResult<()> {
    let node = network.node_mut(root)?;
    node.factored = None;
    Ok(())
}

pub fn factor_best_literal(
    network: &FactorNetwork,
    f: NodeId,
    c: NodeId,
) -> FactorResult<Option<FactorLiteral>> {
    let f_node = network.node(f)?;
    let c_node = network.node(c)?;
    let f_count = node_literal_count(f_node)?;
    let c_count = node_literal_count(c_node)?;
    let mut best = None;
    let mut best_count = 0usize;

    for (fi, f_fanin) in f_node.fanins().iter().enumerate() {
        for (ci, c_fanin) in c_node.fanins().iter().enumerate() {
            if f_fanin != c_fanin {
                continue;
            }

            if c_count[ci].1 > 0 && f_count[fi].1 > best_count {
                best = Some(FactorLiteral {
                    node: *c_fanin,
                    phase: true,
                });
                best_count = f_count[fi].1;
            }

            if c_count[ci].0 > 0 && f_count[fi].0 > best_count {
                best = Some(FactorLiteral {
                    node: *c_fanin,
                    phase: false,
                });
                best_count = f_count[fi].0;
            }
        }
    }

    Ok(best)
}

pub fn factor_quick_kernel(_network: &FactorNetwork, _node: NodeId) -> FactorResult<NodeId> {
    Err(FactorError::MissingNativeIntegration {
        operation: "quick kernel extraction",
    })
}

pub fn factor_best_kernel(_network: &FactorNetwork, _node: NodeId) -> FactorResult<NodeId> {
    Err(FactorError::MissingNativeIntegration {
        operation: "best kernel extraction",
    })
}

pub fn factor_to_nodes(
    network: &FactorNetwork,
    node: NodeId,
    root: &FactorTree,
) -> FactorResult<NodeExpansion> {
    if network.node(node)?.function == NodeFunction::Buffer {
        let fanin = *network
            .node(node)?
            .fanins()
            .first()
            .ok_or(FactorError::MissingChild(FactorKind::Leaf))?;

        return Ok(NodeExpansion {
            root: 0,
            nodes: vec![FactorExpr::Input(fanin)],
        });
    }

    let mut nodes = Vec::new();
    let root = ft_conv(network, node, root, &mut nodes)?;
    Ok(NodeExpansion { root, nodes })
}

fn node_to_ft(network: &FactorNetwork, original: NodeId, node: NodeId) -> FactorResult<FactorTree> {
    let node_data = network.node(node)?;
    if node_data.function != NodeFunction::SumOfProducts {
        let index = network
            .node(original)?
            .fanins()
            .iter()
            .position(|fanin| *fanin == node)
            .ok_or(FactorError::MissingFanin {
                node: original,
                fanin: node,
            })?;

        return Ok(FactorTree::leaf(index));
    }

    if node_data.cubes().is_empty() {
        return Err(FactorError::EmptyCover(node));
    }

    let mut or_children = Vec::new();
    for (cube_index, cube) in node_data.cubes().iter().enumerate().rev() {
        if cube.len() != node_data.fanins().len() {
            return Err(FactorError::InvalidCubeWidth {
                node,
                cube: cube_index,
                expected: node_data.fanins().len(),
                actual: cube.len(),
            });
        }

        let mut and_children = Vec::new();
        for (literal, fanin) in cube.iter().zip(node_data.fanins()) {
            match literal {
                CubeLiteral::Zero => {
                    let child = node_to_ft(network, original, *fanin)?;
                    and_children
                        .push(FactorTree::new(FactorKind::Inverter, -1, 0).with_next_level(child));
                }
                CubeLiteral::One => and_children.push(node_to_ft(network, original, *fanin)?),
                CubeLiteral::DontCare => {}
            }
        }

        if and_children.is_empty() {
            return Err(FactorError::EmptyProductTerm(node));
        }

        let and = if and_children.len() == 1 {
            and_children.remove(0)
        } else {
            FactorTree::from_children(FactorKind::And, and_children)?
        };

        or_children.push(and);
    }

    if or_children.len() == 1 {
        Ok(or_children.remove(0))
    } else {
        FactorTree::from_children(FactorKind::Or, or_children)
    }
}

fn node_literal_count(node: &FactorNode) -> FactorResult<Vec<(usize, usize)>> {
    let mut counts = vec![(0usize, 0usize); node.fanins().len()];

    for (cube_index, cube) in node.cubes().iter().enumerate() {
        if cube.len() != node.fanins().len() {
            return Err(FactorError::InvalidCubeWidth {
                node: NodeId(usize::MAX),
                cube: cube_index,
                expected: node.fanins().len(),
                actual: cube.len(),
            });
        }

        for (index, literal) in cube.iter().enumerate() {
            match literal {
                CubeLiteral::Zero => counts[index].0 += 1,
                CubeLiteral::One => counts[index].1 += 1,
                CubeLiteral::DontCare => {}
            }
        }
    }

    Ok(counts)
}

fn ft_conv(
    network: &FactorNetwork,
    owner: NodeId,
    factor: &FactorTree,
    nodes: &mut Vec<FactorExpr>,
) -> FactorResult<usize> {
    match factor.kind {
        FactorKind::Zero => Ok(push_expr(nodes, FactorExpr::Constant(false))),
        FactorKind::One => Ok(push_expr(nodes, FactorExpr::Constant(true))),
        FactorKind::And => ft_nary_conv(network, owner, factor, nodes, true),
        FactorKind::Or => ft_nary_conv(network, owner, factor, nodes, false),
        FactorKind::Inverter => {
            let child = factor
                .next_level
                .as_deref()
                .ok_or(FactorError::MissingChild(FactorKind::Inverter))?;
            let child = ft_conv(network, owner, child, nodes)?;
            Ok(push_expr(nodes, FactorExpr::Not(child)))
        }
        FactorKind::Leaf => {
            if factor.index < 0 {
                return Err(FactorError::InvalidLeafIndex(factor.index));
            }

            let index = factor.index as usize;
            let fanin = *network
                .node(owner)?
                .fanins()
                .get(index)
                .ok_or(FactorError::InvalidLeafIndex(factor.index))?;
            Ok(push_expr(nodes, FactorExpr::Input(fanin)))
        }
        FactorKind::Unknown => Err(FactorError::UnknownFactorKind),
    }
}

fn ft_nary_conv(
    network: &FactorNetwork,
    owner: NodeId,
    factor: &FactorTree,
    nodes: &mut Vec<FactorExpr>,
    is_and: bool,
) -> FactorResult<usize> {
    let mut child = factor
        .next_level
        .as_deref()
        .ok_or(FactorError::MissingChild(factor.kind))?;
    let left = ft_conv(network, owner, child, nodes)?;
    child = child
        .same_level
        .as_deref()
        .ok_or(FactorError::FewerThanTwoFactorChildren(factor.kind))?;
    let right = ft_conv(network, owner, child, nodes)?;
    let mut accumulator = combine(nodes, is_and, left, right);

    while let Some(next) = child.same_level.as_deref() {
        child = next;
        let right = ft_conv(network, owner, child, nodes)?;
        accumulator = combine(nodes, is_and, accumulator, right);
    }

    Ok(accumulator)
}

fn combine(nodes: &mut Vec<FactorExpr>, is_and: bool, left: usize, right: usize) -> usize {
    if is_and {
        push_expr(nodes, FactorExpr::And(left, right))
    } else {
        push_expr(nodes, FactorExpr::Or(left, right))
    }
}

fn push_expr(nodes: &mut Vec<FactorExpr>, expr: FactorExpr) -> usize {
    let id = nodes.len();
    nodes.push(expr);
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abc_network() -> (FactorNetwork, NodeId, NodeId, NodeId, NodeId) {
        let mut network = FactorNetwork::new();
        let a = network.add_node(FactorNode::input());
        let b = network.add_node(FactorNode::input());
        let c = network.add_node(FactorNode::input());
        let f = network.add_node(FactorNode::sum_of_products(
            vec![a, b, c],
            vec![
                vec![CubeLiteral::One, CubeLiteral::Zero, CubeLiteral::DontCare],
                vec![CubeLiteral::DontCare, CubeLiteral::One, CubeLiteral::One],
            ],
        ));
        (network, a, b, c, f)
    }

    #[test]
    fn factor_state_can_be_allocated_invalidated_and_duplicated() {
        let mut old = FactorNode::input();
        let mut new = FactorNode::input();
        factor_alloc(&mut old);
        factor_set(&mut old, FactorTree::leaf(2));

        factor_dup(&old, &mut new);
        assert_eq!(new.factored(), old.factored());

        factor_invalid(&mut new);
        assert_eq!(new.factored(), None);
    }

    #[test]
    fn traversal_visits_next_level_before_same_level() {
        let tree = FactorTree::from_children(
            FactorKind::And,
            vec![
                FactorTree::leaf(0),
                FactorTree::new(FactorKind::Inverter, -1, 0).with_next_level(FactorTree::leaf(1)),
            ],
        )
        .unwrap();
        let mut kinds = Vec::new();

        let stopped = factor_traverse(NodeId(9), &tree, TraversalOrder::InOrder, |_, factor| {
            kinds.push(factor.kind);
            false
        });

        assert!(!stopped);
        assert_eq!(
            kinds,
            vec![
                FactorKind::And,
                FactorKind::Leaf,
                FactorKind::Inverter,
                FactorKind::Leaf
            ]
        );
    }

    #[test]
    fn sop_tree_conversion_preserves_cube_reverse_order_and_literal_phases() {
        let (network, _a, _b, _c, f) = abc_network();

        let tree = factor_nt_to_ft(&network, f, f).unwrap();

        assert_eq!(tree.kind, FactorKind::Or);
        let first_cube = tree.next_level.as_deref().unwrap();
        assert_eq!(first_cube.kind, FactorKind::And);
        assert_eq!(first_cube.next_level.as_deref().unwrap().index, 1);
        assert_eq!(
            first_cube
                .next_level
                .as_deref()
                .unwrap()
                .same_level
                .as_deref()
                .unwrap()
                .index,
            2
        );
        let second_cube = first_cube.same_level.as_deref().unwrap();
        assert_eq!(second_cube.kind, FactorKind::And);
        assert_eq!(second_cube.next_level.as_deref().unwrap().index, 0);
        assert_eq!(
            second_cube
                .next_level
                .as_deref()
                .unwrap()
                .same_level
                .as_deref()
                .unwrap()
                .kind,
            FactorKind::Inverter
        );
    }

    #[test]
    fn factor_to_nodes_expands_factor_tree_to_expression_nodes() {
        let (network, a, b, _c, f) = abc_network();
        let tree = FactorTree::from_children(
            FactorKind::And,
            vec![
                FactorTree::leaf(0),
                FactorTree::new(FactorKind::Inverter, -1, 0).with_next_level(FactorTree::leaf(1)),
            ],
        )
        .unwrap();

        let expansion = factor_to_nodes(&network, f, &tree).unwrap();

        assert_eq!(
            expansion.nodes,
            vec![
                FactorExpr::Input(a),
                FactorExpr::Input(b),
                FactorExpr::Not(1),
                FactorExpr::And(0, 2)
            ]
        );
        assert_eq!(expansion.root, 3);
    }

    #[test]
    fn best_literal_chooses_shared_candidate_phase_with_highest_source_count() {
        let (mut network, a, b, _c, f) = abc_network();
        let c = network.add_node(FactorNode::sum_of_products(
            vec![a, b],
            vec![vec![CubeLiteral::One, CubeLiteral::DontCare]],
        ));

        let best = factor_best_literal(&network, f, c).unwrap();

        assert_eq!(
            best,
            Some(FactorLiteral {
                node: a,
                phase: true
            })
        );
    }

    #[test]
    fn quick_kernel_reports_missing_native_integration() {
        let (network, _a, _b, _c, f) = abc_network();

        assert_eq!(
            factor_quick_kernel(&network, f),
            Err(FactorError::MissingNativeIntegration {
                operation: "quick kernel extraction"
            })
        );
    }

    #[test]
    fn no_legacy_tokens_or_issue_metadata_are_present_in_this_port() {
        let text = include_str!("ft_util.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }
}
