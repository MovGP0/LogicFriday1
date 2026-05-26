//! Native Rust model for `LogicSynthesis/sis/pld/xln_merge.c`.
//!
//! The C file builds merge candidates for pairs of internal PLD nodes, solves a
//! maximum-cardinality matching either with LINDO or a greedy sparse-matrix
//! heuristic, prints the selected pairs, and then opportunistically collapses
//! remaining internal nodes. This port keeps those deterministic parts native on
//! an owned graph model. Direct mutation of SIS `network_t`, `node_t`,
//! `array_t`, `sm_matrix`, and `st_table` is reported as explicit dependency
//! errors until the prerequisite ports are available.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        reason: "merge_node stores candidate nodes, fanin vectors, and match arrays in array_t",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.228",
        source_file: "LogicSynthesis/sis/io/write_util.c",
        reason: "merge output substitutes a single primary-output fanout via io_po_fanout_count",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "network node iteration, network_num_internal, and network_delete_node are required",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "fanin and fanout traversal provide merge candidates and collapse checks",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.317",
        source_file: "LogicSynthesis/sis/node/names.c",
        reason: "node_long_name is used in merge diagnostics",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node type and fanin count are needed to filter merge candidates",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.376",
        source_file: "LogicSynthesis/sis/pld/pld_util.c",
        reason: "post-merge collapse uses pld_insert_intermediate_nodes_in_table and sparse row/column deletion helpers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.388",
        source_file: "LogicSynthesis/sis/pld/xln_lindo.c",
        reason: "the exact LINDO matching path calls formulate_Lindo and get_Lindo_result",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.392",
        source_file: "LogicSynthesis/sis/pld/xln_new_part.c",
        reason: "post-merge cleanup calls xln_do_trivial_collapse_node_without_moving",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.457",
        source_file: "LogicSynthesis/sis/sparse/matrix.c",
        reason: "merge_node represents candidate pairs in sm_matrix",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.458",
        source_file: "LogicSynthesis/sis/sparse/rows.c",
        reason: "sm_shortest_row and row element traversal drive the greedy heuristic",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.456",
        source_file: "LogicSynthesis/sis/sparse/cols.c",
        reason: "candidate columns are removed after a selected row is matched",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "post-merge cleanup tracks remaining internal nodes in st_table",
    },
];

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    deleted: bool,
}

impl MergeNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            deleted: false,
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MergeNetwork {
    nodes: Vec<MergeNode>,
}

impl MergeNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: MergeNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&MergeNode, XlnMergeError> {
        self.nodes.get(id.0).ok_or(XlnMergeError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[MergeNode] {
        &self.nodes
    }

    pub fn active_node_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (!node.deleted).then_some(NodeId(index)))
            .collect()
    }

    pub fn internal_node_ids(&self) -> Vec<NodeId> {
        self.active_node_ids()
            .into_iter()
            .filter(|id| self.nodes[id.0].kind == NodeKind::Internal)
            .collect()
    }

    pub fn network_num_internal(&self) -> usize {
        self.internal_node_ids().len()
    }

    pub fn fanouts(&self, node: NodeId) -> Result<Vec<NodeId>, XlnMergeError> {
        self.node(node)?;
        Ok(self
            .active_node_ids()
            .into_iter()
            .filter(|candidate| self.nodes[candidate.0].fanins.contains(&node))
            .collect())
    }

    pub fn delete_node(&mut self, node: NodeId) -> Result<(), XlnMergeError> {
        let target = self
            .nodes
            .get_mut(node.0)
            .ok_or(XlnMergeError::UnknownNode(node))?;
        target.deleted = true;
        Ok(())
    }

    pub fn collapse_fanin(&mut self, out: NodeId, input: NodeId) -> Result<(), XlnMergeError> {
        let replacement_fanins = self.node(input)?.fanins.clone();
        let out_node = self
            .nodes
            .get_mut(out.0)
            .ok_or(XlnMergeError::UnknownNode(out))?;
        if out_node.deleted {
            return Err(XlnMergeError::DeletedNode(out));
        }
        if !out_node.fanins.contains(&input) {
            return Err(XlnMergeError::NotAFanin { out, input });
        }

        let mut revised = Vec::new();
        for fanin in out_node.fanins.iter().copied() {
            if fanin == input {
                for replacement in &replacement_fanins {
                    if *replacement != out && !revised.contains(replacement) {
                        revised.push(*replacement);
                    }
                }
            } else if !revised.contains(&fanin) {
                revised.push(fanin);
            }
        }
        out_node.fanins = revised;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MergeOptions {
    pub max_fanin: usize,
    pub max_common_fanin: usize,
    pub max_union_fanin: usize,
    pub use_lindo: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MergeCandidate {
    pub left: NodeId,
    pub right: NodeId,
    pub common_fanins: usize,
    pub union_fanins: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MergeMatch {
    pub left: NodeId,
    pub right: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeReport {
    pub matches: Vec<MergeMatch>,
    pub lines: Vec<String>,
    pub final_clbs: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollapseAfterMergeReport {
    pub deleted_nodes: Vec<NodeId>,
    pub remaining_table: HashSet<NodeId>,
    pub final_clbs: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnMergeError {
    UnknownNode(NodeId),
    DeletedNode(NodeId),
    NotAFanin {
        out: NodeId,
        input: NodeId,
    },
    LindoUnavailable {
        dependencies: &'static [PortDependency],
    },
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for XlnMergeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_merge node {:?}", node),
            Self::DeletedNode(node) => write!(f, "xln_merge node {:?} was deleted", node),
            Self::NotAFanin { out, input } => {
                write!(f, "node {:?} is not a fanin of {:?}", input, out)
            }
            Self::LindoUnavailable { dependencies } => write!(
                f,
                "LINDO matching is blocked by {} unported SIS C-file dependencies",
                dependencies.len()
            ),
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS C-file dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for XlnMergeError {}

pub fn merge_sis_network_blocked<Network>(
    _network: &mut Network,
    _options: MergeOptions,
) -> Result<MergeReport, XlnMergeError> {
    Err(XlnMergeError::MissingNativePorts {
        operation: "merge_node against SIS network_t",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn merge_network(
    network: &MergeNetwork,
    options: MergeOptions,
) -> Result<MergeReport, XlnMergeError> {
    if options.use_lindo {
        return Err(XlnMergeError::LindoUnavailable {
            dependencies: REQUIRED_PORT_DEPENDENCIES,
        });
    }

    let candidates = collect_merge_candidates(network, options)?;
    let matches = greedy_merge_nodes_without_lindo(&candidates);
    let lines = format_merge_lines(network, &matches)?;
    let final_clbs = network.network_num_internal().saturating_sub(matches.len());
    Ok(MergeReport {
        matches,
        lines,
        final_clbs,
    })
}

pub fn collect_merge_candidates(
    network: &MergeNetwork,
    options: MergeOptions,
) -> Result<Vec<MergeCandidate>, XlnMergeError> {
    let candidate_nodes: Vec<NodeId> = network
        .internal_node_ids()
        .into_iter()
        .filter(|node| {
            network
                .node(*node)
                .map(|node| node.fanins.len() <= options.max_fanin)
                .unwrap_or(false)
        })
        .collect();

    let mut fanin_sets = Vec::with_capacity(candidate_nodes.len());
    for node in &candidate_nodes {
        let mut fanins = network.node(*node)?.fanins.clone();
        fanins.sort();
        fanins.dedup();
        fanin_sets.push(fanins);
    }

    let mut candidates = Vec::new();
    for i in 0..candidate_nodes.len() {
        for j in (i + 1)..candidate_nodes.len() {
            let (common_fanins, union_fanins) =
                count_intersection_union(&fanin_sets[i], &fanin_sets[j]);
            if common_fanins <= options.max_common_fanin && union_fanins <= options.max_union_fanin
            {
                candidates.push(MergeCandidate {
                    left: candidate_nodes[i],
                    right: candidate_nodes[j],
                    common_fanins,
                    union_fanins,
                });
            }
        }
    }
    Ok(candidates)
}

pub fn count_intersection_union(left: &[NodeId], right: &[NodeId]) -> (usize, usize) {
    let mut i = 0;
    let mut j = 0;
    let mut intersection = 0;
    let mut union = 0;

    while i < left.len() || j < right.len() {
        if i == left.len() {
            union += right.len() - j;
            break;
        }
        if j == right.len() {
            union += left.len() - i;
            break;
        }

        match left[i].cmp(&right[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                intersection += 1;
                i += 1;
                j += 1;
            }
        }
        union += 1;
    }

    (intersection, union)
}

pub fn greedy_merge_nodes_without_lindo(candidates: &[MergeCandidate]) -> Vec<MergeMatch> {
    let mut active_edges: Vec<(NodeId, NodeId)> = candidates
        .iter()
        .map(|candidate| ordered_pair(candidate.left, candidate.right))
        .collect();
    let mut matches = Vec::new();

    while let Some(row1) = shortest_row(&active_edges) {
        let Some(row2) = neighbor_with_minimum_neighbors(row1, &active_edges) else {
            break;
        };
        matches.push(MergeMatch {
            left: row1,
            right: row2,
        });
        active_edges.retain(|(left, right)| {
            *left != row1 && *right != row1 && *left != row2 && *right != row2
        });
    }

    matches
}

pub fn shortest_row(edges: &[(NodeId, NodeId)]) -> Option<NodeId> {
    let mut rows = rows_in_scan_order(edges);
    rows.sort_by_key(|row| (degree(*row, edges), *row));
    rows.into_iter().next()
}

pub fn neighbor_with_minimum_neighbors(row: NodeId, edges: &[(NodeId, NodeId)]) -> Option<NodeId> {
    neighbors(row, edges)
        .into_iter()
        .min_by_key(|neighbor| (degree(*neighbor, edges), *neighbor))
}

pub fn format_merge_lines(
    network: &MergeNetwork,
    matches: &[MergeMatch],
) -> Result<Vec<String>, XlnMergeError> {
    let mut lines = vec!["Merging two CLB's into one CLB".to_owned()];
    for matched in matches {
        let left = display_node_after_single_po_substitution(network, matched.left)?;
        let right = display_node_after_single_po_substitution(network, matched.right)?;
        lines.push(format!("Merge node {left} and {right}"));
    }
    lines.push(format!(
        "# of CLB's = {}",
        network.network_num_internal().saturating_sub(matches.len())
    ));
    Ok(lines)
}

pub fn collapse_nodes_after_merge(
    network: &mut MergeNetwork,
    matches: &[MergeMatch],
    support: usize,
) -> Result<CollapseAfterMergeReport, XlnMergeError> {
    let mut table: HashSet<NodeId> = network.internal_node_ids().into_iter().collect();
    for matched in matches {
        table.remove(&matched.left);
        table.remove(&matched.right);
    }

    let mut deleted_nodes = Vec::new();
    for node in network.internal_node_ids() {
        if !table.contains(&node) {
            continue;
        }
        if are_fanouts_in_table(network, &table, node)?
            && trivial_collapse_without_moving(network, node, support)?
        {
            deleted_nodes.push(node);
        }
    }

    for node in &deleted_nodes {
        table.remove(node);
        network.delete_node(*node)?;
    }

    Ok(CollapseAfterMergeReport {
        final_clbs: network.network_num_internal().saturating_sub(matches.len()),
        deleted_nodes,
        remaining_table: table,
    })
}

pub fn are_fanouts_in_table(
    network: &MergeNetwork,
    table: &HashSet<NodeId>,
    node: NodeId,
) -> Result<bool, XlnMergeError> {
    if network.node(node)?.kind != NodeKind::Internal {
        return Ok(false);
    }
    Ok(network
        .fanouts(node)?
        .into_iter()
        .all(|fanout| table.contains(&fanout)))
}

fn trivial_collapse_without_moving(
    network: &mut MergeNetwork,
    node: NodeId,
    support: usize,
) -> Result<bool, XlnMergeError> {
    let fanouts = network.fanouts(node)?;
    if fanouts.is_empty() {
        return Ok(false);
    }
    if fanouts.iter().any(|fanout| {
        network
            .node(*fanout)
            .is_ok_and(|node| node.kind == NodeKind::PrimaryOutput)
    }) {
        return Ok(false);
    }
    if fanouts.iter().any(|fanout| {
        composite_fanin_count(network, *fanout, node).is_ok_and(|count| count > support)
    }) {
        return Ok(false);
    }
    for fanout in fanouts {
        network.collapse_fanin(fanout, node)?;
    }
    Ok(true)
}

fn composite_fanin_count(
    network: &MergeNetwork,
    out: NodeId,
    input: NodeId,
) -> Result<usize, XlnMergeError> {
    let out_node = network.node(out)?;
    let input_node = network.node(input)?;
    let mut fanins: HashSet<NodeId> = out_node
        .fanins
        .iter()
        .copied()
        .filter(|fanin| *fanin != input)
        .collect();
    for fanin in &input_node.fanins {
        fanins.insert(*fanin);
    }
    Ok(fanins.len())
}

fn display_node_after_single_po_substitution(
    network: &MergeNetwork,
    node: NodeId,
) -> Result<String, XlnMergeError> {
    let po_fanouts: Vec<NodeId> = network
        .fanouts(node)?
        .into_iter()
        .filter(|fanout| network.nodes[fanout.0].kind == NodeKind::PrimaryOutput)
        .collect();
    let display_node = if po_fanouts.len() == 1 {
        po_fanouts[0]
    } else {
        node
    };
    Ok(network.node(display_node)?.name.clone())
}

fn rows_in_scan_order(edges: &[(NodeId, NodeId)]) -> Vec<NodeId> {
    let mut rows = Vec::new();
    for (left, right) in edges {
        if !rows.contains(left) {
            rows.push(*left);
        }
        if !rows.contains(right) {
            rows.push(*right);
        }
    }
    rows
}

fn neighbors(row: NodeId, edges: &[(NodeId, NodeId)]) -> Vec<NodeId> {
    let mut result = Vec::new();
    for (left, right) in edges {
        if *left == row && !result.contains(right) {
            result.push(*right);
        } else if *right == row && !result.contains(left) {
            result.push(*left);
        }
    }
    result
}

fn degree(row: NodeId, edges: &[(NodeId, NodeId)]) -> usize {
    neighbors(row, edges).len()
}

fn ordered_pair(left: NodeId, right: NodeId) -> (NodeId, NodeId) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options() -> MergeOptions {
        MergeOptions {
            max_fanin: 4,
            max_common_fanin: 2,
            max_union_fanin: 4,
            use_lindo: false,
        }
    }

    #[test]
    fn count_intersection_union_matches_sorted_fanin_merge() {
        let left = [NodeId(0), NodeId(2), NodeId(4)];
        let right = [NodeId(1), NodeId(2), NodeId(4), NodeId(5)];

        assert_eq!(count_intersection_union(&left, &right), (2, 5));
    }

    #[test]
    fn collect_merge_candidates_filters_internal_nodes_by_c_thresholds() {
        let mut network = MergeNetwork::new();
        let a = network.add_node(MergeNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(MergeNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(MergeNode::new("c", NodeKind::PrimaryInput));
        let d = network.add_node(MergeNode::new("d", NodeKind::PrimaryInput));
        let e = network.add_node(MergeNode::new("e", NodeKind::PrimaryInput));
        let n1 = network.add_node(MergeNode::new("n1", NodeKind::Internal).with_fanins(vec![a, b]));
        let n2 = network.add_node(MergeNode::new("n2", NodeKind::Internal).with_fanins(vec![b, c]));
        network.add_node(MergeNode::new("n3", NodeKind::Internal).with_fanins(vec![c, d, e]));
        network
            .add_node(MergeNode::new("wide", NodeKind::Internal).with_fanins(vec![a, b, c, d, e]));

        let candidates = collect_merge_candidates(&network, default_options()).unwrap();

        assert_eq!(
            candidates,
            vec![
                MergeCandidate {
                    left: n1,
                    right: n2,
                    common_fanins: 1,
                    union_fanins: 3,
                },
                MergeCandidate {
                    left: n1,
                    right: NodeId(7),
                    common_fanins: 0,
                    union_fanins: 5,
                },
                MergeCandidate {
                    left: n2,
                    right: NodeId(7),
                    common_fanins: 1,
                    union_fanins: 4,
                },
            ]
            .into_iter()
            .filter(|candidate| candidate.union_fanins <= default_options().max_union_fanin)
            .collect::<Vec<_>>()
        );
    }

    #[test]
    fn greedy_matching_picks_shortest_row_then_neighbor_with_minimum_degree() {
        let candidates = vec![
            MergeCandidate {
                left: NodeId(0),
                right: NodeId(1),
                common_fanins: 0,
                union_fanins: 2,
            },
            MergeCandidate {
                left: NodeId(1),
                right: NodeId(2),
                common_fanins: 0,
                union_fanins: 2,
            },
            MergeCandidate {
                left: NodeId(1),
                right: NodeId(3),
                common_fanins: 0,
                union_fanins: 2,
            },
        ];

        assert_eq!(
            greedy_merge_nodes_without_lindo(&candidates),
            vec![MergeMatch {
                left: NodeId(0),
                right: NodeId(1),
            }]
        );
    }

    #[test]
    fn merge_report_formats_single_po_fanout_names_and_final_clbs() {
        let mut network = MergeNetwork::new();
        let a = network.add_node(MergeNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(MergeNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(MergeNode::new("c", NodeKind::PrimaryInput));
        let n1 = network.add_node(MergeNode::new("n1", NodeKind::Internal).with_fanins(vec![a, b]));
        let n2 = network.add_node(MergeNode::new("n2", NodeKind::Internal).with_fanins(vec![b, c]));
        network.add_node(MergeNode::new("po_n1", NodeKind::PrimaryOutput).with_fanins(vec![n1]));

        let report = merge_network(&network, default_options()).unwrap();

        assert_eq!(
            report.matches,
            vec![MergeMatch {
                left: n1,
                right: n2,
            }]
        );
        assert_eq!(
            report.lines,
            vec![
                "Merging two CLB's into one CLB",
                "Merge node po_n1 and n2",
                "# of CLB's = 1",
            ]
        );
        assert_eq!(report.final_clbs, 1);
    }

    #[test]
    fn collapse_after_merge_removes_remaining_nodes_only_when_all_fanouts_stay_in_table() {
        let mut network = MergeNetwork::new();
        let a = network.add_node(MergeNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(MergeNode::new("b", NodeKind::PrimaryInput));
        let x = network.add_node(MergeNode::new("x", NodeKind::Internal).with_fanins(vec![a, b]));
        let y = network.add_node(MergeNode::new("y", NodeKind::Internal).with_fanins(vec![x]));
        let z = network.add_node(MergeNode::new("z", NodeKind::Internal).with_fanins(vec![y, b]));
        let m1 = network.add_node(MergeNode::new("m1", NodeKind::Internal).with_fanins(vec![a]));
        let m2 = network.add_node(MergeNode::new("m2", NodeKind::Internal).with_fanins(vec![b]));
        let matches = [MergeMatch {
            left: m1,
            right: m2,
        }];

        let report = collapse_nodes_after_merge(&mut network, &matches, 3).unwrap();

        assert_eq!(report.deleted_nodes, vec![x, y]);
        assert!(network.node(x).unwrap().is_deleted());
        assert_eq!(network.node(z).unwrap().fanins, vec![a, b]);
        assert_eq!(report.final_clbs, 2);
    }

    #[test]
    fn primary_output_fanout_prevents_post_merge_collapse() {
        let mut network = MergeNetwork::new();
        let a = network.add_node(MergeNode::new("a", NodeKind::PrimaryInput));
        let x = network.add_node(MergeNode::new("x", NodeKind::Internal).with_fanins(vec![a]));
        network.add_node(MergeNode::new("out", NodeKind::PrimaryOutput).with_fanins(vec![x]));
        let table = HashSet::from([x]);

        assert!(!are_fanouts_in_table(&network, &table, x).unwrap());
        assert_eq!(
            collapse_nodes_after_merge(&mut network, &[], 2)
                .unwrap()
                .deleted_nodes,
            Vec::<NodeId>::new()
        );
    }

    #[test]
    fn lindo_path_reports_explicit_dependency_bead_and_source_file() {
        let network = MergeNetwork::new();
        let mut options = default_options();
        options.use_lindo = true;

        let error = merge_network(&network, options).unwrap_err();

        assert_eq!(
            error,
            XlnMergeError::LindoUnavailable {
                dependencies: REQUIRED_PORT_DEPENDENCIES,
            }
        );
        assert!(required_port_dependencies().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.388"
                && dependency.source_file == "LogicSynthesis/sis/pld/xln_lindo.c"
        }));
    }

    #[test]
    fn sis_bound_entry_reports_dependency_beads_and_sources() {
        let mut network = ();
        let error = merge_sis_network_blocked(&mut network, default_options()).unwrap_err();

        assert_eq!(
            error,
            XlnMergeError::MissingNativePorts {
                operation: "merge_node against SIS network_t",
                dependencies: REQUIRED_PORT_DEPENDENCIES,
            }
        );
        assert!(
            error
                .to_string()
                .contains("unported SIS C-file dependencies")
        );
        assert!(required_port_dependencies().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.376"
                && dependency.source_file == "LogicSynthesis/sis/pld/pld_util.c"
        }));
        assert!(required_port_dependencies().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.457"
                && dependency.source_file == "LogicSynthesis/sis/sparse/matrix.c"
        }));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_merge.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
