//! Native Rust DFS leaf ordering for SIS.
//!
//! The legacy routine orders a selected leaf set by traversing the transitive
//! fanin of one or more roots. This port keeps the same traversal heuristics on
//! owned graph data and deliberately exposes no per-file C ABI entry points.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

pub const UNASSIGNED_ORDER: isize = -1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DfsOrderNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DfsOrderFunction {
    ConstantZero,
    ConstantOne,
    Other,
}

impl DfsOrderFunction {
    fn is_constant(self) -> bool {
        matches!(self, Self::ConstantZero | Self::ConstantOne)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DfsOrderNode<N> {
    pub id: N,
    pub kind: DfsOrderNodeKind,
    pub function: DfsOrderFunction,
    pub fanins: Vec<N>,
    pub factored_leaf_depths: Vec<(N, usize)>,
}

impl<N> DfsOrderNode<N> {
    pub fn new(id: N, kind: DfsOrderNodeKind, fanins: impl Into<Vec<N>>) -> Self {
        Self {
            id,
            kind,
            function: DfsOrderFunction::Other,
            fanins: fanins.into(),
            factored_leaf_depths: Vec::new(),
        }
    }

    pub fn constant(id: N, function: DfsOrderFunction) -> Self {
        debug_assert!(function.is_constant());
        Self {
            id,
            kind: DfsOrderNodeKind::Internal,
            function,
            fanins: Vec::new(),
            factored_leaf_depths: Vec::new(),
        }
    }

    pub fn with_factored_leaf_depths(mut self, depths: impl Into<Vec<(N, usize)>>) -> Self {
        self.factored_leaf_depths = depths.into();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DfsOrderLeaf<N> {
    pub leaf: N,
    pub order: isize,
}

impl<N> DfsOrderLeaf<N> {
    pub fn new(leaf: N) -> Self {
        Self {
            leaf,
            order: UNASSIGNED_ORDER,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DfsOrderError<N> {
    EmptyRootsWithLeaves { leaf_count: usize },
    DuplicateNode(N),
    DuplicateLeaf(N),
    MissingNode(N),
    LeafAlreadyOrdered { index: usize, order: isize },
    PrimaryOutputRootWithoutFanin(N),
    PrimaryInputReachedOutsideLeaves(N),
    NonConstantNodeWithoutFanins(N),
    MissingTransitiveFaninInfo(N),
    CycleDetected(N),
    FactoredLeafDepthIsZero { node: N, leaf: N },
}

impl<N: fmt::Debug> fmt::Display for DfsOrderError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRootsWithLeaves { leaf_count } => {
                write!(f, "empty root set cannot order {leaf_count} leaves")
            }
            Self::DuplicateNode(node) => write!(f, "duplicate node id {node:?}"),
            Self::DuplicateLeaf(leaf) => write!(f, "duplicate leaf id {leaf:?}"),
            Self::MissingNode(node) => write!(f, "node {node:?} is not present in the graph"),
            Self::LeafAlreadyOrdered { index, order } => {
                write!(f, "leaf {index} already has assigned order {order}")
            }
            Self::PrimaryOutputRootWithoutFanin(node) => {
                write!(f, "primary-output root {node:?} has no driving fanin")
            }
            Self::PrimaryInputReachedOutsideLeaves(node) => {
                write!(f, "primary input {node:?} was reached outside the leaf set")
            }
            Self::NonConstantNodeWithoutFanins(node) => {
                write!(f, "non-constant node {node:?} has no fanins")
            }
            Self::MissingTransitiveFaninInfo(node) => {
                write!(f, "missing transitive-fanin info for node {node:?}")
            }
            Self::CycleDetected(node) => write!(f, "cycle detected at node {node:?}"),
            Self::FactoredLeafDepthIsZero { node, leaf } => {
                write!(f, "factored leaf {leaf:?} of node {node:?} has zero depth")
            }
        }
    }
}

impl<N: fmt::Debug> Error for DfsOrderError<N> {}

pub type DfsOrderResult<T, N> = Result<T, DfsOrderError<N>>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct DfsInfo<N> {
    node: N,
    level: usize,
    max_leaf: usize,
}

pub fn order_dfs<N>(
    nodes: &[DfsOrderNode<N>],
    roots: &[N],
    leaves: &mut [DfsOrderLeaf<N>],
    fixed_root_order: bool,
) -> DfsOrderResult<Vec<N>, N>
where
    N: Clone + Eq + Hash,
{
    validate_unassigned_leaves(leaves)?;

    if roots.is_empty() {
        return if leaves.is_empty() {
            Ok(Vec::new())
        } else {
            Err(DfsOrderError::EmptyRootsWithLeaves {
                leaf_count: leaves.len(),
            })
        };
    }

    let index = build_node_index(nodes)?;
    let leaf_index = build_leaf_index(leaves)?;
    let fanouts = build_fanouts(nodes, &index)?;
    let internal_roots = replace_primary_output_roots(nodes, roots, &index)?;
    let (tfi_inputs_first, tfi_outputs_first) =
        extract_transitive_fanin(nodes, &index, &fanouts, &internal_roots, &leaf_index)?;

    let mut info_table = tfi_inputs_first
        .iter()
        .cloned()
        .map(|node| {
            (
                node.clone(),
                DfsInfo {
                    node,
                    level: 0,
                    max_leaf: 0,
                },
            )
        })
        .collect::<HashMap<_, _>>();

    for node_id in &tfi_outputs_first {
        if leaf_index.contains_key(node_id) {
            continue;
        }
        propagate_level_to_fanins(nodes, &index, node_id, &mut info_table)?;
    }

    for node_id in &tfi_inputs_first {
        if leaf_index.contains_key(node_id) {
            let Some(info) = info_table.get_mut(node_id) else {
                return Err(DfsOrderError::MissingTransitiveFaninInfo(node_id.clone()));
            };
            info.max_leaf = info.level;
            continue;
        }

        let max_leaf = propagate_max_leaf_from_fanins(nodes, &index, node_id, &info_table)?;
        let Some(info) = info_table.get_mut(node_id) else {
            return Err(DfsOrderError::MissingTransitiveFaninInfo(node_id.clone()));
        };
        info.max_leaf = max_leaf;
    }

    if fixed_root_order {
        for (root_order, node_id) in internal_roots.iter().enumerate() {
            let Some(info) = info_table.get_mut(node_id) else {
                return Err(DfsOrderError::MissingTransitiveFaninInfo(node_id.clone()));
            };
            info.max_leaf = 0;
            info.level = root_order;
        }
    }

    let mut visited = HashSet::new();
    let mut order_list = Vec::new();
    let mut order_count = 0;
    order_nodes_rec(
        nodes,
        &index,
        &internal_roots,
        &info_table,
        &mut visited,
        leaves,
        &leaf_index,
        &mut order_list,
        &mut order_count,
    )?;

    debug_assert!(order_count as usize <= leaves.len());
    Ok(order_list)
}

pub fn bdd_tfi_inputs_first<N>(
    nodes: &[DfsOrderNode<N>],
    roots: &[N],
    leaves: &[DfsOrderLeaf<N>],
) -> DfsOrderResult<Vec<N>, N>
where
    N: Clone + Eq + Hash,
{
    let index = build_node_index(nodes)?;
    let leaf_index = build_leaf_index(leaves)?;
    let mut tfi = HashSet::new();
    let mut visiting = HashSet::new();
    let mut result = Vec::new();

    for root in roots {
        visit_inputs_first(
            nodes,
            &index,
            root,
            &leaf_index,
            &mut tfi,
            &mut visiting,
            &mut result,
        )?;
    }

    Ok(result)
}

fn validate_unassigned_leaves<N>(leaves: &[DfsOrderLeaf<N>]) -> DfsOrderResult<(), N>
where
    N: Clone,
{
    for (index, leaf) in leaves.iter().enumerate() {
        if leaf.order != UNASSIGNED_ORDER {
            return Err(DfsOrderError::LeafAlreadyOrdered {
                index,
                order: leaf.order,
            });
        }
    }

    Ok(())
}

fn build_node_index<N>(nodes: &[DfsOrderNode<N>]) -> DfsOrderResult<HashMap<N, usize>, N>
where
    N: Clone + Eq + Hash,
{
    let mut index = HashMap::with_capacity(nodes.len());
    for (position, node) in nodes.iter().enumerate() {
        if index.insert(node.id.clone(), position).is_some() {
            return Err(DfsOrderError::DuplicateNode(node.id.clone()));
        }
    }

    Ok(index)
}

fn build_leaf_index<N>(leaves: &[DfsOrderLeaf<N>]) -> DfsOrderResult<HashMap<N, usize>, N>
where
    N: Clone + Eq + Hash,
{
    let mut index = HashMap::with_capacity(leaves.len());
    for (position, leaf) in leaves.iter().enumerate() {
        if index.insert(leaf.leaf.clone(), position).is_some() {
            return Err(DfsOrderError::DuplicateLeaf(leaf.leaf.clone()));
        }
    }

    Ok(index)
}

fn build_fanouts<N>(
    nodes: &[DfsOrderNode<N>],
    index: &HashMap<N, usize>,
) -> DfsOrderResult<HashMap<N, Vec<N>>, N>
where
    N: Clone + Eq + Hash,
{
    let mut fanouts = nodes
        .iter()
        .map(|node| (node.id.clone(), Vec::new()))
        .collect::<HashMap<_, _>>();

    for node in nodes {
        for fanin in &node.fanins {
            if !index.contains_key(fanin) {
                return Err(DfsOrderError::MissingNode(fanin.clone()));
            }
            let Some(entries) = fanouts.get_mut(fanin) else {
                return Err(DfsOrderError::MissingNode(fanin.clone()));
            };
            entries.push(node.id.clone());
        }
    }

    Ok(fanouts)
}

fn replace_primary_output_roots<N>(
    nodes: &[DfsOrderNode<N>],
    roots: &[N],
    index: &HashMap<N, usize>,
) -> DfsOrderResult<Vec<N>, N>
where
    N: Clone + Eq + Hash,
{
    let mut internal_roots = Vec::with_capacity(roots.len());

    for root in roots {
        let node = node_by_id(nodes, index, root)?;
        if node.kind == DfsOrderNodeKind::PrimaryOutput {
            let Some(fanin) = node.fanins.first() else {
                return Err(DfsOrderError::PrimaryOutputRootWithoutFanin(root.clone()));
            };
            internal_roots.push(fanin.clone());
        } else {
            internal_roots.push(root.clone());
        }
    }

    Ok(internal_roots)
}

fn extract_transitive_fanin<N>(
    nodes: &[DfsOrderNode<N>],
    index: &HashMap<N, usize>,
    fanouts: &HashMap<N, Vec<N>>,
    roots: &[N],
    leaves: &HashMap<N, usize>,
) -> DfsOrderResult<(Vec<N>, Vec<N>), N>
where
    N: Clone + Eq + Hash,
{
    let mut tfi = HashSet::new();
    let mut visiting = HashSet::new();
    let mut tfi_inputs_first = Vec::new();
    let mut tfi_outputs_first = Vec::new();

    for root in roots {
        visit_inputs_first(
            nodes,
            index,
            root,
            leaves,
            &mut tfi,
            &mut visiting,
            &mut tfi_inputs_first,
        )?;
    }

    let mut visited = HashSet::new();
    for leaf in leaves.keys() {
        visit_outputs_first(fanouts, leaf, &tfi, &mut visited, &mut tfi_outputs_first)?;
    }

    Ok((tfi_inputs_first, tfi_outputs_first))
}

fn visit_inputs_first<N>(
    nodes: &[DfsOrderNode<N>],
    index: &HashMap<N, usize>,
    node_id: &N,
    leaves: &HashMap<N, usize>,
    tfi: &mut HashSet<N>,
    visiting: &mut HashSet<N>,
    tfi_inputs_first: &mut Vec<N>,
) -> DfsOrderResult<(), N>
where
    N: Clone + Eq + Hash,
{
    if tfi.contains(node_id) {
        return Ok(());
    }
    if !visiting.insert(node_id.clone()) {
        return Err(DfsOrderError::CycleDetected(node_id.clone()));
    }

    let node = node_by_id(nodes, index, node_id)?;
    if !leaves.contains_key(node_id) {
        if node.kind == DfsOrderNodeKind::PrimaryInput {
            return Err(DfsOrderError::PrimaryInputReachedOutsideLeaves(
                node_id.clone(),
            ));
        }
        if !node.function.is_constant() {
            if node.fanins.is_empty() {
                return Err(DfsOrderError::NonConstantNodeWithoutFanins(node_id.clone()));
            }
            for fanin in &node.fanins {
                visit_inputs_first(nodes, index, fanin, leaves, tfi, visiting, tfi_inputs_first)?;
            }
        }
    }

    visiting.remove(node_id);
    tfi_inputs_first.push(node_id.clone());
    tfi.insert(node_id.clone());
    Ok(())
}

fn visit_outputs_first<N>(
    fanouts: &HashMap<N, Vec<N>>,
    node_id: &N,
    tfi: &HashSet<N>,
    visited: &mut HashSet<N>,
    tfi_outputs_first: &mut Vec<N>,
) -> DfsOrderResult<(), N>
where
    N: Clone + Eq + Hash,
{
    if !tfi.contains(node_id) || visited.contains(node_id) {
        return Ok(());
    }

    let Some(fanout_nodes) = fanouts.get(node_id) else {
        return Err(DfsOrderError::MissingNode(node_id.clone()));
    };

    for fanout in fanout_nodes {
        visit_outputs_first(fanouts, fanout, tfi, visited, tfi_outputs_first)?;
    }

    tfi_outputs_first.push(node_id.clone());
    visited.insert(node_id.clone());
    Ok(())
}

fn propagate_level_to_fanins<N>(
    nodes: &[DfsOrderNode<N>],
    index: &HashMap<N, usize>,
    node_id: &N,
    info_table: &mut HashMap<N, DfsInfo<N>>,
) -> DfsOrderResult<(), N>
where
    N: Clone + Eq + Hash,
{
    let node = node_by_id(nodes, index, node_id)?;
    let Some(info) = info_table.get(node_id) else {
        return Err(DfsOrderError::MissingTransitiveFaninInfo(node_id.clone()));
    };
    let level = info.level;

    if node.factored_leaf_depths.is_empty() {
        for fanin in &node.fanins {
            update_fanin_level(fanin, level + 1, info_table)?;
        }
        return Ok(());
    }

    for (fanin, depth) in &node.factored_leaf_depths {
        if *depth == 0 {
            return Err(DfsOrderError::FactoredLeafDepthIsZero {
                node: node_id.clone(),
                leaf: fanin.clone(),
            });
        }
        update_fanin_level(fanin, level + depth, info_table)?;
    }

    Ok(())
}

fn update_fanin_level<N>(
    fanin: &N,
    level: usize,
    info_table: &mut HashMap<N, DfsInfo<N>>,
) -> DfsOrderResult<(), N>
where
    N: Clone + Eq + Hash,
{
    let Some(info) = info_table.get_mut(fanin) else {
        return Err(DfsOrderError::MissingTransitiveFaninInfo(fanin.clone()));
    };
    info.level = info.level.max(level);
    Ok(())
}

fn propagate_max_leaf_from_fanins<N>(
    nodes: &[DfsOrderNode<N>],
    index: &HashMap<N, usize>,
    node_id: &N,
    info_table: &HashMap<N, DfsInfo<N>>,
) -> DfsOrderResult<usize, N>
where
    N: Clone + Eq + Hash,
{
    let node = node_by_id(nodes, index, node_id)?;
    let mut max_leaf = 0;

    for fanin in &node.fanins {
        let Some(info) = info_table.get(fanin) else {
            return Err(DfsOrderError::MissingTransitiveFaninInfo(fanin.clone()));
        };
        max_leaf = max_leaf.max(info.max_leaf);
    }

    Ok(max_leaf)
}

#[allow(clippy::too_many_arguments)]
fn order_nodes_rec<N>(
    nodes: &[DfsOrderNode<N>],
    index: &HashMap<N, usize>,
    roots: &[N],
    info_table: &HashMap<N, DfsInfo<N>>,
    visited: &mut HashSet<N>,
    leaves: &mut [DfsOrderLeaf<N>],
    leaf_index: &HashMap<N, usize>,
    node_list: &mut Vec<N>,
    order_count: &mut isize,
) -> DfsOrderResult<(), N>
where
    N: Clone + Eq + Hash,
{
    let mut info_array = Vec::new();
    for (position, node_id) in roots.iter().enumerate() {
        if visited.contains(node_id) {
            continue;
        }
        let Some(info) = info_table.get(node_id) else {
            return Err(DfsOrderError::MissingTransitiveFaninInfo(node_id.clone()));
        };
        info_array.push((position, info));
    }

    info_array.sort_by(|(left_position, left), (right_position, right)| {
        right
            .max_leaf
            .cmp(&left.max_leaf)
            .then_with(|| left.level.cmp(&right.level))
            .then_with(|| left_position.cmp(right_position))
    });

    for (_, info) in info_array {
        let node_id = &info.node;
        if visited.contains(node_id) {
            continue;
        }

        if let Some(leaf_position) = leaf_index.get(node_id) {
            leaves[*leaf_position].order = *order_count;
            *order_count += 1;
        } else {
            let fanins = node_by_id(nodes, index, node_id)?.fanins.clone();
            order_nodes_rec(
                nodes,
                index,
                &fanins,
                info_table,
                visited,
                leaves,
                leaf_index,
                node_list,
                order_count,
            )?;
        }

        node_list.push(node_id.clone());
        visited.insert(node_id.clone());
    }

    Ok(())
}

fn node_by_id<'a, N>(
    nodes: &'a [DfsOrderNode<N>],
    index: &HashMap<N, usize>,
    node_id: &N,
) -> DfsOrderResult<&'a DfsOrderNode<N>, N>
where
    N: Clone + Eq + Hash,
{
    let Some(position) = index.get(node_id) else {
        return Err(DfsOrderError::MissingNode(node_id.clone()));
    };
    Ok(&nodes[*position])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pi(id: &'static str) -> DfsOrderNode<&'static str> {
        DfsOrderNode::new(id, DfsOrderNodeKind::PrimaryInput, [])
    }

    fn internal(id: &'static str, fanins: &[&'static str]) -> DfsOrderNode<&'static str> {
        DfsOrderNode::new(id, DfsOrderNodeKind::Internal, fanins.to_vec())
    }

    fn po(id: &'static str, fanin: &'static str) -> DfsOrderNode<&'static str> {
        DfsOrderNode::new(id, DfsOrderNodeKind::PrimaryOutput, [fanin])
    }

    fn leaf_orders(leaves: &[DfsOrderLeaf<&'static str>]) -> Vec<(&'static str, isize)> {
        leaves.iter().map(|leaf| (leaf.leaf, leaf.order)).collect()
    }

    #[test]
    fn empty_root_set_is_a_noop_when_leaf_set_is_empty() {
        let mut leaves = Vec::<DfsOrderLeaf<&str>>::new();

        assert_eq!(order_dfs(&[], &[], &mut leaves, false), Ok(Vec::new()));
    }

    #[test]
    fn orders_leaves_and_returns_inputs_first_node_list() {
        let nodes = vec![
            pi("a"),
            pi("b"),
            pi("c"),
            internal("x", &["a", "b"]),
            internal("y", &["c"]),
            internal("r", &["x", "y"]),
        ];
        let mut leaves = vec![
            DfsOrderLeaf::new("a"),
            DfsOrderLeaf::new("b"),
            DfsOrderLeaf::new("c"),
        ];

        let order = order_dfs(&nodes, &["r"], &mut leaves, false).unwrap();

        assert_eq!(order, vec!["a", "b", "x", "c", "y", "r"]);
        assert_eq!(leaf_orders(&leaves), vec![("a", 0), ("b", 1), ("c", 2)]);
    }

    #[test]
    fn deeper_max_leaf_roots_are_visited_first_unless_root_order_is_fixed() {
        let nodes = vec![
            pi("a"),
            pi("b"),
            internal("mid", &["b"]),
            internal("short_root", &["a"]),
            internal("deep_root", &["mid"]),
        ];

        let mut flexible_leaves = vec![DfsOrderLeaf::new("a"), DfsOrderLeaf::new("b")];
        let flexible_order = order_dfs(
            &nodes,
            &["short_root", "deep_root"],
            &mut flexible_leaves,
            false,
        )
        .unwrap();

        let mut fixed_leaves = vec![DfsOrderLeaf::new("a"), DfsOrderLeaf::new("b")];
        let fixed_order = order_dfs(
            &nodes,
            &["short_root", "deep_root"],
            &mut fixed_leaves,
            true,
        )
        .unwrap();

        assert_eq!(
            flexible_order,
            vec!["b", "mid", "deep_root", "a", "short_root"]
        );
        assert_eq!(leaf_orders(&flexible_leaves), vec![("a", 1), ("b", 0)]);
        assert_eq!(
            fixed_order,
            vec!["a", "short_root", "b", "mid", "deep_root"]
        );
        assert_eq!(leaf_orders(&fixed_leaves), vec![("a", 0), ("b", 1)]);
    }

    #[test]
    fn primary_output_roots_are_replaced_by_their_driver() {
        let nodes = vec![pi("a"), internal("r", &["a"]), po("out", "r")];
        let mut leaves = vec![DfsOrderLeaf::new("a")];

        let order = order_dfs(&nodes, &["out"], &mut leaves, false).unwrap();

        assert_eq!(order, vec!["a", "r"]);
        assert_eq!(leaf_orders(&leaves), vec![("a", 0)]);
    }

    #[test]
    fn factored_leaf_depths_take_part_in_level_tie_breaks() {
        let nodes = vec![
            pi("a"),
            pi("b"),
            DfsOrderNode::new("r", DfsOrderNodeKind::Internal, ["b", "a"])
                .with_factored_leaf_depths([("a", 3), ("b", 1)]),
        ];
        let mut leaves = vec![DfsOrderLeaf::new("a"), DfsOrderLeaf::new("b")];

        let order = order_dfs(&nodes, &["r"], &mut leaves, false).unwrap();

        assert_eq!(order, vec!["a", "b", "r"]);
        assert_eq!(leaf_orders(&leaves), vec![("a", 0), ("b", 1)]);
    }

    #[test]
    fn reports_primary_inputs_that_are_not_cut_by_leaves() {
        let nodes = vec![pi("a"), internal("r", &["a"])];
        let mut leaves = Vec::<DfsOrderLeaf<&str>>::new();

        assert_eq!(
            order_dfs(&nodes, &["r"], &mut leaves, false),
            Err(DfsOrderError::PrimaryInputReachedOutsideLeaves("a"))
        );
    }

    #[test]
    fn bdd_tfi_inputs_first_omits_leaf_fanins_but_keeps_roots() {
        let nodes = vec![pi("a"), pi("b"), internal("r", &["a", "b"])];
        let leaves = vec![DfsOrderLeaf::new("a")];

        let result = bdd_tfi_inputs_first(&nodes, &["r"], &leaves);

        assert_eq!(
            result,
            Err(DfsOrderError::PrimaryInputReachedOutsideLeaves("b"))
        );
    }
}
