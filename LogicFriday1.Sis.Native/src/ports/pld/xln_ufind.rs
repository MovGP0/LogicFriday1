//! Native Rust model for `LogicSynthesis/sis/pld/xln_ufind.c`.
//!
//! The C file combines a small union-find helper used by `xln_k_decomp.c`
//! with PLD network entry points that call broader SIS network/decomposition
//! routines. This module ports the union-find behavior to owned Rust state and
//! exposes the network operations through traits so native callers can provide
//! the missing SIS backends without reviving per-file C ABI shims.

use std::error::Error;
use std::fmt;

pub const REQUIRED_NETWORK_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.129",
        source_file: "LogicSynthesis/sis/decomp/dec_tech.c",
        reason: "provides decomp_tech_network used by estimate_clb_no and and_or_map",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.392",
        source_file: "LogicSynthesis/sis/pld/xln_new_part.c",
        reason: "provides imp_part_network used after technology decomposition",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "provides network_dup, network_free, and network_num_internal",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.297",
        source_file: "LogicSynthesis/sis/network/dfs.c",
        reason: "provides network_dfs traversal for estimate_net_no",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "provides node_function for primary-input/output filtering",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "provides node_num_fanin for estimate_net_no",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub fn required_network_dependencies() -> &'static [PortDependency] {
    REQUIRED_NETWORK_DEPENDENCIES
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XlnUfindOperation {
    EstimateClbNo,
    AndOrMap,
    EstimateNetNo,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnUfindError {
    MissingNode {
        index: usize,
    },
    NodeHasParent {
        index: usize,
        parent: usize,
    },
    MissingNativePorts {
        operation: XlnUfindOperation,
        dependencies: &'static [PortDependency],
    },
    Backend(String),
}

impl fmt::Display for XlnUfindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode { index } => write!(f, "tree node {index} is not present"),
            Self::NodeHasParent { index, parent } => {
                write!(f, "Node {index} has a parent {parent}")
            }
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => {
                write!(
                    f,
                    "{operation:?} requires native Rust ports for SIS dependencies: "
                )?;
                for (position, dependency) in dependencies.iter().enumerate() {
                    if position > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} ({})", dependency.bead_id, dependency.source_file)?;
                }
                Ok(())
            }
            Self::Backend(message) => f.write_str(message),
        }
    }
}

impl Error for XlnUfindError {}

pub type XlnUfindResult<T> = Result<T, XlnUfindError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeNode {
    pub index: usize,
    pub parent: usize,
    pub num_child: usize,
    pub class_num: i32,
}

impl TreeNode {
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            parent: index,
            num_child: 0,
            class_num: -1,
        }
    }

    pub const fn with_num_child(index: usize, num_child: usize) -> Self {
        Self {
            index,
            parent: index,
            num_child,
            class_num: -1,
        }
    }

    pub const fn is_root(&self) -> bool {
        self.parent == self.index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnionFindForest {
    nodes: Vec<TreeNode>,
}

impl UnionFindForest {
    pub fn new_singletons(count: usize) -> Self {
        let nodes = (0..count).map(TreeNode::new).collect();
        Self { nodes }
    }

    pub fn from_nodes(nodes: Vec<TreeNode>) -> Self {
        Self { nodes }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, index: usize) -> XlnUfindResult<&TreeNode> {
        self.nodes
            .get(index)
            .ok_or(XlnUfindError::MissingNode { index })
    }

    pub fn node_mut(&mut self, index: usize) -> XlnUfindResult<&mut TreeNode> {
        self.nodes
            .get_mut(index)
            .ok_or(XlnUfindError::MissingNode { index })
    }

    pub fn make_son(&mut self, child: usize, parent: usize) -> XlnUfindResult<()> {
        let child_num = self.node(child)?.num_child;
        self.node_mut(child)?.parent = parent;
        self.node_mut(parent)?.num_child += child_num;
        Ok(())
    }

    pub fn union_roots(&mut self, first: usize, second: usize) -> XlnUfindResult<usize> {
        self.require_root(first)?;
        self.require_root(second)?;

        if self.node(first)?.num_child < self.node(second)?.num_child {
            self.make_son(first, second)?;
            Ok(second)
        } else {
            self.make_son(second, first)?;
            Ok(first)
        }
    }

    pub fn find_tree(&mut self, index: usize) -> XlnUfindResult<usize> {
        let mut root = index;
        loop {
            let parent = self.node(root)?.parent;
            if parent == root {
                break;
            }
            root = parent;
        }

        let mut node_on_path = index;
        while node_on_path != root {
            let next = self.node(node_on_path)?.parent;
            self.node_mut(node_on_path)?.parent = root;
            node_on_path = next;
        }

        Ok(root)
    }

    pub fn assign_class_numbers(&mut self) -> XlnUfindResult<usize> {
        let mut root_classes: Vec<(usize, i32)> = Vec::new();
        for index in 0..self.nodes.len() {
            let root = self.find_tree(index)?;
            let class_num = match root_classes
                .iter()
                .find(|(known_root, _)| *known_root == root)
            {
                Some((_, class_num)) => *class_num,
                None => {
                    let class_num = root_classes.len() as i32;
                    root_classes.push((root, class_num));
                    class_num
                }
            };
            self.node_mut(index)?.class_num = class_num;
        }
        Ok(root_classes.len())
    }

    pub fn nodes(&self) -> &[TreeNode] {
        &self.nodes
    }

    fn require_root(&self, index: usize) -> XlnUfindResult<()> {
        let node = self.node(index)?;
        if node.parent != index {
            return Err(XlnUfindError::NodeHasParent {
                index,
                parent: node.parent,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XlnNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XlnNodeSummary {
    pub kind: XlnNodeKind,
    pub fanin_count: usize,
}

impl XlnNodeSummary {
    pub const fn new(kind: XlnNodeKind, fanin_count: usize) -> Self {
        Self { kind, fanin_count }
    }
}

pub fn estimate_net_no_from_nodes(nodes_in_dfs_order: &[XlnNodeSummary]) -> usize {
    nodes_in_dfs_order
        .iter()
        .filter(|node| node.kind != XlnNodeKind::PrimaryInput)
        .filter(|node| node.kind != XlnNodeKind::PrimaryOutput)
        .map(|node| node.fanin_count)
        .sum()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EstimateClbReport {
    pub upper_bound: usize,
}

pub trait XlnNetworkBackend {
    type Network;
    type Node;

    fn duplicate_network(&mut self, network: &Self::Network) -> XlnUfindResult<Self::Network>;

    fn decomp_tech_network(
        &mut self,
        network: &mut Self::Network,
        and_limit: usize,
        or_limit: usize,
    ) -> XlnUfindResult<()>;

    fn imp_part_network(
        &mut self,
        network: &mut Self::Network,
        size: usize,
        move_fanins: usize,
        max_fanins: usize,
    ) -> XlnUfindResult<()>;

    fn network_num_internal(&mut self, network: &Self::Network) -> XlnUfindResult<usize>;

    fn network_dfs(&mut self, network: &Self::Network) -> XlnUfindResult<Vec<Self::Node>>;

    fn node_kind(&mut self, node: &Self::Node) -> XlnUfindResult<XlnNodeKind>;

    fn node_num_fanin(&mut self, node: &Self::Node) -> XlnUfindResult<usize>;
}

pub fn estimate_clb_no<B>(
    backend: &mut B,
    network: &B::Network,
    size: usize,
) -> XlnUfindResult<EstimateClbReport>
where
    B: XlnNetworkBackend,
{
    let mut duplicate = backend.duplicate_network(network)?;
    backend.decomp_tech_network(&mut duplicate, 2, 2)?;
    backend.imp_part_network(&mut duplicate, size, 0, 0)?;
    let upper_bound = backend.network_num_internal(&duplicate)?;
    Ok(EstimateClbReport { upper_bound })
}

pub fn format_clb_upper_bound(report: EstimateClbReport) -> String {
    format!("The upper bound on CLBs is {}\n", report.upper_bound)
}

pub fn and_or_map<B>(backend: &mut B, network: &mut B::Network, size: usize) -> XlnUfindResult<()>
where
    B: XlnNetworkBackend,
{
    backend.decomp_tech_network(network, 2, 2)?;
    backend.imp_part_network(network, size, 0, 0)
}

pub fn estimate_net_no<B>(backend: &mut B, network: &B::Network) -> XlnUfindResult<usize>
where
    B: XlnNetworkBackend,
{
    let order = backend.network_dfs(network)?;
    let mut value = 0;

    for node in order {
        let kind = backend.node_kind(&node)?;
        if kind != XlnNodeKind::PrimaryInput && kind != XlnNodeKind::PrimaryOutput {
            value += backend.node_num_fanin(&node)?;
        }
    }

    Ok(value)
}

#[derive(Default)]
pub struct MissingXlnNetworkBackend;

impl XlnNetworkBackend for MissingXlnNetworkBackend {
    type Network = ();
    type Node = ();

    fn duplicate_network(&mut self, _network: &Self::Network) -> XlnUfindResult<Self::Network> {
        Err(missing(XlnUfindOperation::EstimateClbNo))
    }

    fn decomp_tech_network(
        &mut self,
        _network: &mut Self::Network,
        _and_limit: usize,
        _or_limit: usize,
    ) -> XlnUfindResult<()> {
        Err(missing(XlnUfindOperation::AndOrMap))
    }

    fn imp_part_network(
        &mut self,
        _network: &mut Self::Network,
        _size: usize,
        _move_fanins: usize,
        _max_fanins: usize,
    ) -> XlnUfindResult<()> {
        Err(missing(XlnUfindOperation::AndOrMap))
    }

    fn network_num_internal(&mut self, _network: &Self::Network) -> XlnUfindResult<usize> {
        Err(missing(XlnUfindOperation::EstimateClbNo))
    }

    fn network_dfs(&mut self, _network: &Self::Network) -> XlnUfindResult<Vec<Self::Node>> {
        Err(missing(XlnUfindOperation::EstimateNetNo))
    }

    fn node_kind(&mut self, _node: &Self::Node) -> XlnUfindResult<XlnNodeKind> {
        Err(missing(XlnUfindOperation::EstimateNetNo))
    }

    fn node_num_fanin(&mut self, _node: &Self::Node) -> XlnUfindResult<usize> {
        Err(missing(XlnUfindOperation::EstimateNetNo))
    }
}

pub fn estimate_clb_no_with_missing_dependencies(size: usize) -> XlnUfindResult<EstimateClbReport> {
    let mut backend = MissingXlnNetworkBackend;
    estimate_clb_no(&mut backend, &(), size)
}

pub fn and_or_map_with_missing_dependencies(size: usize) -> XlnUfindResult<()> {
    let mut backend = MissingXlnNetworkBackend;
    let mut network = ();
    and_or_map(&mut backend, &mut network, size)
}

pub fn estimate_net_no_with_missing_dependencies() -> XlnUfindResult<usize> {
    let mut backend = MissingXlnNetworkBackend;
    estimate_net_no(&mut backend, &())
}

fn missing(operation: XlnUfindOperation) -> XlnUfindError {
    XlnUfindError::MissingNativePorts {
        operation,
        dependencies: REQUIRED_NETWORK_DEPENDENCIES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_roots_matches_c_tie_break_and_parent_check() {
        let mut forest = UnionFindForest::new_singletons(3);

        assert_eq!(forest.union_roots(0, 1).unwrap(), 0);
        assert_eq!(forest.node(1).unwrap().parent, 0);
        assert_eq!(forest.node(0).unwrap().num_child, 0);

        assert_eq!(
            forest.union_roots(1, 2),
            Err(XlnUfindError::NodeHasParent {
                index: 1,
                parent: 0,
            })
        );
    }

    #[test]
    fn union_roots_attaches_smaller_child_count_under_larger_root() {
        let mut forest = UnionFindForest::from_nodes(vec![
            TreeNode::with_num_child(0, 1),
            TreeNode::with_num_child(1, 3),
        ]);

        assert_eq!(forest.union_roots(0, 1).unwrap(), 1);
        assert_eq!(forest.node(0).unwrap().parent, 1);
        assert_eq!(forest.node(1).unwrap().num_child, 4);
    }

    #[test]
    fn find_tree_compresses_path_to_root() {
        let mut forest = UnionFindForest::new_singletons(4);
        forest.node_mut(1).unwrap().parent = 0;
        forest.node_mut(2).unwrap().parent = 1;
        forest.node_mut(3).unwrap().parent = 2;

        assert_eq!(forest.find_tree(3).unwrap(), 0);
        assert_eq!(forest.node(3).unwrap().parent, 0);
        assert_eq!(forest.node(2).unwrap().parent, 0);
    }

    #[test]
    fn assign_class_numbers_numbers_roots_in_scan_order() {
        let mut forest = UnionFindForest::new_singletons(5);
        forest.union_roots(0, 2).unwrap();
        forest.union_roots(3, 4).unwrap();

        assert_eq!(forest.assign_class_numbers().unwrap(), 3);
        let classes: Vec<i32> = forest.nodes().iter().map(|node| node.class_num).collect();
        assert_eq!(classes, vec![0, 1, 0, 2, 2]);
    }

    #[test]
    fn estimate_net_no_counts_only_internal_fanins() {
        let nodes = [
            XlnNodeSummary::new(XlnNodeKind::PrimaryInput, 0),
            XlnNodeSummary::new(XlnNodeKind::Internal, 2),
            XlnNodeSummary::new(XlnNodeKind::PrimaryOutput, 1),
            XlnNodeSummary::new(XlnNodeKind::Internal, 3),
        ];

        assert_eq!(estimate_net_no_from_nodes(&nodes), 5);
    }

    #[derive(Default)]
    struct RecordingBackend {
        actions: Vec<String>,
        nodes: Vec<XlnNodeSummary>,
    }

    impl XlnNetworkBackend for RecordingBackend {
        type Network = usize;
        type Node = XlnNodeSummary;

        fn duplicate_network(&mut self, network: &Self::Network) -> XlnUfindResult<Self::Network> {
            self.actions.push(format!("dup {network}"));
            Ok(*network + 10)
        }

        fn decomp_tech_network(
            &mut self,
            network: &mut Self::Network,
            and_limit: usize,
            or_limit: usize,
        ) -> XlnUfindResult<()> {
            self.actions
                .push(format!("decomp {network} {and_limit} {or_limit}"));
            *network += 1;
            Ok(())
        }

        fn imp_part_network(
            &mut self,
            network: &mut Self::Network,
            size: usize,
            move_fanins: usize,
            max_fanins: usize,
        ) -> XlnUfindResult<()> {
            self.actions
                .push(format!("imp {network} {size} {move_fanins} {max_fanins}"));
            *network += 1;
            Ok(())
        }

        fn network_num_internal(&mut self, network: &Self::Network) -> XlnUfindResult<usize> {
            self.actions.push(format!("internal {network}"));
            Ok(*network)
        }

        fn network_dfs(&mut self, _network: &Self::Network) -> XlnUfindResult<Vec<Self::Node>> {
            Ok(self.nodes.clone())
        }

        fn node_kind(&mut self, node: &Self::Node) -> XlnUfindResult<XlnNodeKind> {
            Ok(node.kind)
        }

        fn node_num_fanin(&mut self, node: &Self::Node) -> XlnUfindResult<usize> {
            Ok(node.fanin_count)
        }
    }

    #[test]
    fn estimate_clb_no_duplicates_decomposes_partitions_and_counts_internal_nodes() {
        let mut backend = RecordingBackend::default();

        let report = estimate_clb_no(&mut backend, &7, 5).unwrap();

        assert_eq!(report.upper_bound, 19);
        assert_eq!(
            backend.actions,
            vec!["dup 7", "decomp 17 2 2", "imp 18 5 0 0", "internal 19"]
        );
        assert_eq!(
            format_clb_upper_bound(report),
            "The upper bound on CLBs is 19\n"
        );
    }

    #[test]
    fn and_or_map_decomposes_and_partitions_original_network() {
        let mut backend = RecordingBackend::default();
        let mut network = 3;

        and_or_map(&mut backend, &mut network, 4).unwrap();

        assert_eq!(network, 5);
        assert_eq!(backend.actions, vec!["decomp 3 2 2", "imp 4 4 0 0"]);
    }

    #[test]
    fn estimate_net_no_uses_backend_dfs_order_and_node_accessors() {
        let mut backend = RecordingBackend {
            nodes: vec![
                XlnNodeSummary::new(XlnNodeKind::PrimaryInput, 0),
                XlnNodeSummary::new(XlnNodeKind::Internal, 4),
                XlnNodeSummary::new(XlnNodeKind::PrimaryOutput, 1),
                XlnNodeSummary::new(XlnNodeKind::Internal, 2),
            ],
            ..RecordingBackend::default()
        };

        assert_eq!(estimate_net_no(&mut backend, &0).unwrap(), 6);
    }

    #[test]
    fn missing_dependency_errors_include_blocker_beads_and_source_files() {
        let Err(XlnUfindError::MissingNativePorts {
            operation,
            dependencies,
        }) = estimate_net_no_with_missing_dependencies()
        else {
            panic!("expected missing native ports");
        };

        assert_eq!(operation, XlnUfindOperation::EstimateNetNo);
        assert!(dependencies.iter().any(|dependency| dependency.bead_id
            == "LogicFriday1-8j8.2.6.297"
            && dependency.source_file == "LogicSynthesis/sis/network/dfs.c"));
        assert!(dependencies.iter().any(|dependency| dependency.bead_id
            == "LogicFriday1-8j8.2.6.392"
            && dependency.source_file == "LogicSynthesis/sis/pld/xln_new_part.c"));

        let message = estimate_clb_no_with_missing_dependencies(4)
            .unwrap_err()
            .to_string();
        assert!(message.contains("LogicFriday1-8j8.2.6.129"));
        assert!(message.contains("LogicSynthesis/sis/decomp/dec_tech.c"));
    }
}
