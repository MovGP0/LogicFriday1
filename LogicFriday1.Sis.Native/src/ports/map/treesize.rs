//! Native Rust tree-size accounting for `sis/map/treesize.c`.
//!
//! The original SIS routine walks a mapped network from primary outputs,
//! splits trees at primary inputs and multi-fanout nodes, then prints a size
//! distribution. This port keeps the same accounting rule over the owned
//! `MapperTree` model: leaves and shared nodes form fragment frontiers, gate
//! nodes are counted once, and distribution entries are deterministic. Direct
//! `network_t` report integration remains an explicit dependency gap.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use super::tree::{MapperTree, MapperTreeError, MapperTreeNode, MapperTreeNodeId};
use super::two_level::PortDependency;

pub const REQUIRED_NETWORK_REPORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.271",
        source_file: "LogicSynthesis/sis/map/tree.c",
        note: "native mapper tree construction from SIS network nodes",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "native primary-output traversal and node fanout data",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "native node type, names, fanins, and fanout accounting",
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeSizeReport {
    pub root: MapperTreeNodeId,
    pub fragments: Vec<TreeSizeFragment>,
    pub distribution: Vec<TreeSizeDistributionEntry>,
}

impl TreeSizeReport {
    pub fn fragment(&self, root: MapperTreeNodeId) -> Option<&TreeSizeFragment> {
        self.fragments.iter().find(|fragment| fragment.root == root)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeSizeFragment {
    pub root: MapperTreeNodeId,
    pub internal_nodes: usize,
    pub leaf_count: usize,
    pub support: BTreeSet<MapperTreeNodeId>,
    pub depth: usize,
}

impl TreeSizeFragment {
    pub fn total_size(&self) -> usize {
        self.internal_nodes + self.leaf_count
    }

    pub fn support_size(&self) -> usize {
        self.support.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeSizeDistributionEntry {
    pub total_size: usize,
    pub leaf_count: usize,
    pub support_size: usize,
    pub depth: usize,
    pub frequency: usize,
    pub representative_root: MapperTreeNodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreeSizeError {
    Tree(MapperTreeError),
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for TreeSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tree(error) => write!(f, "{error}"),
            Self::MissingSisPorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} native SIS prerequisite ports",
                dependencies.len()
            ),
        }
    }
}

impl Error for TreeSizeError {}

impl From<MapperTreeError> for TreeSizeError {
    fn from(value: MapperTreeError) -> Self {
        Self::Tree(value)
    }
}

pub fn required_network_report_beads() -> &'static [PortDependency] {
    REQUIRED_NETWORK_REPORT_BEADS
}

pub fn sis_network_tree_size_report_unavailable() -> Result<TreeSizeReport, TreeSizeError> {
    Err(TreeSizeError::MissingSisPorts {
        operation: "map_print_tree_size over SIS network_t",
        dependencies: REQUIRED_NETWORK_REPORT_BEADS,
    })
}

pub fn analyze_mapper_tree(tree: &MapperTree) -> Result<TreeSizeReport, TreeSizeError> {
    tree.validate()?;

    let fanout_counts = reachable_fanout_counts(tree)?;
    let mut builder = TreeSizeReportBuilder::new(tree.root(), fanout_counts);
    builder.visit(tree, tree.root(), tree.root(), 0)?;

    Ok(builder.finish())
}

pub fn format_tree_size_distribution(report: &TreeSizeReport) -> String {
    let mut output = String::from("Distribution of Tree Sizes\n--------------------------\n");

    for entry in &report.distribution {
        output.push_str(&format!(
            "    nodes={:3}   leaf={:3}  support={:3}  depth={:3}  freq={:3}  (root node = {})\n",
            entry.total_size,
            entry.leaf_count,
            entry.support_size,
            entry.depth,
            entry.frequency,
            entry.representative_root.index()
        ));
    }

    output
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct DistributionKey {
    total_size: usize,
    leaf_count: usize,
    support_size: usize,
    depth: usize,
}

#[derive(Debug)]
struct TreeSizeReportBuilder {
    root: MapperTreeNodeId,
    fanout_counts: BTreeMap<MapperTreeNodeId, usize>,
    visited: BTreeSet<MapperTreeNodeId>,
    fragments: BTreeMap<MapperTreeNodeId, TreeSizeFragment>,
}

impl TreeSizeReportBuilder {
    fn new(root: MapperTreeNodeId, fanout_counts: BTreeMap<MapperTreeNodeId, usize>) -> Self {
        Self {
            root,
            fanout_counts,
            visited: BTreeSet::new(),
            fragments: BTreeMap::new(),
        }
    }

    fn finish(self) -> TreeSizeReport {
        let fragments = self.fragments.into_values().collect::<Vec<_>>();
        let distribution = build_distribution(&fragments);

        TreeSizeReport {
            root: self.root,
            fragments,
            distribution,
        }
    }

    fn visit(
        &mut self,
        tree: &MapperTree,
        node: MapperTreeNodeId,
        mut fragment_root: MapperTreeNodeId,
        mut depth: usize,
    ) -> Result<(), TreeSizeError> {
        let is_boundary = self.is_boundary(tree, node)?;
        let tree_node = tree
            .node(node)
            .ok_or(MapperTreeError::MissingNode { node })?;

        if is_boundary && node != fragment_root {
            self.add_leaf(fragment_root, node, depth);
            if matches!(tree_node, MapperTreeNode::Leaf { .. }) {
                return Ok(());
            }

            fragment_root = node;
            depth = 0;
        }

        if !self.visited.insert(node) {
            return Ok(());
        }

        match tree_node {
            MapperTreeNode::Leaf { .. } => {
                self.add_leaf(fragment_root, node, depth);
            }
            MapperTreeNode::Gate { fanins, .. } => {
                self.add_internal_node(fragment_root, depth);
                if fanins.is_empty() {
                    return Ok(());
                }

                for fanin in fanins {
                    self.visit(tree, fanin.node, fragment_root, depth + 1)?;
                }
            }
        }

        Ok(())
    }

    fn is_boundary(
        &self,
        tree: &MapperTree,
        node: MapperTreeNodeId,
    ) -> Result<bool, MapperTreeError> {
        let tree_node = tree
            .node(node)
            .ok_or(MapperTreeError::MissingNode { node })?;

        Ok(matches!(tree_node, MapperTreeNode::Leaf { .. })
            || self.fanout_counts.get(&node).copied().unwrap_or_default() > 1)
    }

    fn add_internal_node(&mut self, root: MapperTreeNodeId, depth: usize) {
        let fragment = self.fragment_mut(root);
        fragment.internal_nodes += 1;
        fragment.depth = fragment.depth.max(depth);
    }

    fn add_leaf(&mut self, root: MapperTreeNodeId, leaf: MapperTreeNodeId, depth: usize) {
        let fragment = self.fragment_mut(root);
        fragment.leaf_count += 1;
        fragment.support.insert(leaf);
        fragment.depth = fragment.depth.max(depth);
    }

    fn fragment_mut(&mut self, root: MapperTreeNodeId) -> &mut TreeSizeFragment {
        self.fragments
            .entry(root)
            .or_insert_with(|| TreeSizeFragment {
                root,
                internal_nodes: 0,
                leaf_count: 0,
                support: BTreeSet::new(),
                depth: 0,
            })
    }
}

fn reachable_fanout_counts(
    tree: &MapperTree,
) -> Result<BTreeMap<MapperTreeNodeId, usize>, MapperTreeError> {
    let mut counts = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![tree.root()];

    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        for fanin in tree
            .node(node)
            .ok_or(MapperTreeError::MissingNode { node })?
            .fanins()
        {
            *counts.entry(fanin.node).or_default() += 1;
            stack.push(fanin.node);
        }
    }

    Ok(counts)
}

fn build_distribution(fragments: &[TreeSizeFragment]) -> Vec<TreeSizeDistributionEntry> {
    let mut entries = BTreeMap::<DistributionKey, TreeSizeDistributionEntry>::new();

    for fragment in fragments {
        let key = DistributionKey {
            total_size: fragment.total_size(),
            leaf_count: fragment.leaf_count,
            support_size: fragment.support_size(),
            depth: fragment.depth,
        };
        let entry = entries
            .entry(key)
            .or_insert_with(|| TreeSizeDistributionEntry {
                total_size: key.total_size,
                leaf_count: key.leaf_count,
                support_size: key.support_size,
                depth: key.depth,
                frequency: 0,
                representative_root: fragment.root,
            });
        entry.frequency += 1;
        entry.representative_root = entry.representative_root.min(fragment.root);
    }

    let mut distribution = entries.into_values().collect::<Vec<_>>();
    distribution.sort_by(|left, right| {
        right
            .total_size
            .cmp(&left.total_size)
            .then_with(|| right.leaf_count.cmp(&left.leaf_count))
            .then_with(|| right.support_size.cmp(&left.support_size))
            .then_with(|| right.depth.cmp(&left.depth))
            .then_with(|| left.representative_root.cmp(&right.representative_root))
    });
    distribution
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::tree::{MapperTreeFanin, PrimitiveGateKind};

    fn simple_tree() -> (
        MapperTree,
        MapperTreeNodeId,
        MapperTreeNodeId,
        MapperTreeNodeId,
    ) {
        let mut tree = MapperTree::empty();
        let a = tree.add_leaf("a");
        let b = tree.add_leaf("b");
        let root = tree.add_gate(
            PrimitiveGateKind::And,
            vec![MapperTreeFanin::new(a), MapperTreeFanin::new(b)],
        );
        tree.set_root(root);
        tree.validate().unwrap();
        (tree, root, a, b)
    }

    #[test]
    fn counts_single_mapper_tree_fragment() {
        let (tree, root, a, b) = simple_tree();

        let report = analyze_mapper_tree(&tree).unwrap();
        let fragment = report.fragment(root).unwrap();

        assert_eq!(fragment.internal_nodes, 1);
        assert_eq!(fragment.leaf_count, 2);
        assert_eq!(fragment.support, BTreeSet::from([a, b]));
        assert_eq!(fragment.depth, 1);
        assert_eq!(fragment.total_size(), 3);
        assert_eq!(
            report.distribution,
            vec![TreeSizeDistributionEntry {
                total_size: 3,
                leaf_count: 2,
                support_size: 2,
                depth: 1,
                frequency: 1,
                representative_root: root,
            }]
        );
    }

    #[test]
    fn cuts_fragments_at_shared_nodes_and_counts_support_once() {
        let mut tree = MapperTree::empty();
        let a = tree.add_leaf("a");
        let b = tree.add_leaf("b");
        let shared = tree.add_gate(
            PrimitiveGateKind::And,
            vec![MapperTreeFanin::new(a), MapperTreeFanin::new(b)],
        );
        let root = tree.add_gate(
            PrimitiveGateKind::Or,
            vec![MapperTreeFanin::new(shared), MapperTreeFanin::new(shared)],
        );
        tree.set_root(root);
        tree.validate().unwrap();

        let report = analyze_mapper_tree(&tree).unwrap();
        let root_fragment = report.fragment(root).unwrap();
        let shared_fragment = report.fragment(shared).unwrap();

        assert_eq!(root_fragment.internal_nodes, 1);
        assert_eq!(root_fragment.leaf_count, 2);
        assert_eq!(root_fragment.support, BTreeSet::from([shared]));
        assert_eq!(root_fragment.depth, 1);

        assert_eq!(shared_fragment.internal_nodes, 1);
        assert_eq!(shared_fragment.leaf_count, 2);
        assert_eq!(shared_fragment.support, BTreeSet::from([a, b]));
        assert_eq!(shared_fragment.depth, 1);
        assert_eq!(report.distribution.len(), 2);
        assert!(report.distribution.iter().any(|entry| {
            entry.total_size == 3
                && entry.leaf_count == 2
                && entry.support_size == 1
                && entry.frequency == 1
        }));
        assert!(report.distribution.iter().any(|entry| {
            entry.total_size == 3
                && entry.leaf_count == 2
                && entry.support_size == 2
                && entry.frequency == 1
        }));
    }

    #[test]
    fn reports_leaf_root_as_one_leaf_fragment() {
        let mut tree = MapperTree::empty();
        let root = tree.add_leaf("input");
        tree.set_root(root);
        tree.validate().unwrap();

        let report = analyze_mapper_tree(&tree).unwrap();
        let fragment = report.fragment(root).unwrap();

        assert_eq!(fragment.internal_nodes, 0);
        assert_eq!(fragment.leaf_count, 1);
        assert_eq!(fragment.support, BTreeSet::from([root]));
        assert_eq!(fragment.depth, 0);
    }

    #[test]
    fn propagates_mapper_tree_validation_errors() {
        let tree = MapperTree::empty();

        assert_eq!(
            analyze_mapper_tree(&tree).unwrap_err(),
            TreeSizeError::Tree(MapperTreeError::EmptyTree)
        );
    }

    #[test]
    fn exposes_typed_dependency_gap_for_network_report() {
        assert_eq!(
            required_network_report_beads(),
            REQUIRED_NETWORK_REPORT_BEADS
        );
        assert_eq!(
            sis_network_tree_size_report_unavailable().unwrap_err(),
            TreeSizeError::MissingSisPorts {
                operation: "map_print_tree_size over SIS network_t",
                dependencies: REQUIRED_NETWORK_REPORT_BEADS,
            }
        );
    }

    #[test]
    fn formats_distribution_with_c_style_header() {
        let (tree, root, _, _) = simple_tree();

        let report = analyze_mapper_tree(&tree).unwrap();
        let text = format_tree_size_distribution(&report);

        assert!(text.starts_with("Distribution of Tree Sizes\n"));
        assert!(text.contains("nodes=  3"));
        assert!(text.contains("leaf=  2"));
        assert!(text.contains(&format!("root node = {}", root.index())));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("treesize.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
