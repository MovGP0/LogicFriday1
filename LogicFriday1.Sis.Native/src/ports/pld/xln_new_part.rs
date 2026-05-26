//! Native Rust model for `LogicSynthesis/sis/pld/xln_new_part.c`.
//!
//! The C routine repeatedly collapses feasible PLD fanin nodes. This port keeps
//! the deterministic parts native: composite-fanin accounting, candidate
//! scoring, disjoint candidate selection, trivial-collapse feasibility, and a
//! small owned graph mutation model. Direct execution against SIS `network_t`
//! and `node_t` remains blocked by the dependency beads listed below.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
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
        reason: "xln_new_part.c stores candidate and cover sets in array_t",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.297",
        source_file: "LogicSynthesis/sis/network/dfs.c",
        reason: "trivial collapse iterates network_dfs order",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "network internal-node iteration, delete-node, and sweep operations",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.309",
        source_file: "LogicSynthesis/sis/node/collapse.c",
        reason: "node_collapse changes fanout logic during candidate and trivial collapses",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "fanin/fanout traversal and fanin-index lookup",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.317",
        source_file: "LogicSynthesis/sis/node/names.c",
        reason: "diagnostic output uses node_long_name",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node type/function classification and fanin counts",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.321",
        source_file: "LogicSynthesis/sis/node/nodemisc.c",
        reason: "node_replace applies simplified fanout functions after trivial collapse",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.391",
        source_file: "LogicSynthesis/sis/pld/xln_move_d.c",
        reason: "MOVE_FANINS calls xln_node_move_fanins before testing trivial collapse",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.455",
        source_file: "LogicSynthesis/sis/simplify/simp.c",
        reason: "node_simplify refreshes collapsed fanout functions",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "array_disjoint uses st_table to map node pointers to dense indexes",
    },
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PldNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    deleted: bool,
}

impl PldNode {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PldNetwork {
    nodes: Vec<PldNode>,
}

impl PldNetwork {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: PldNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&PldNode, XlnNewPartError> {
        self.nodes.get(id.0).ok_or(XlnNewPartError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[PldNode] {
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

    pub fn fanouts(&self, node: NodeId) -> Result<Vec<NodeId>, XlnNewPartError> {
        self.node(node)?;
        Ok(self
            .active_node_ids()
            .into_iter()
            .filter(|candidate| self.nodes[candidate.0].fanins.contains(&node))
            .collect())
    }

    pub fn collapse_fanin(&mut self, out: NodeId, input: NodeId) -> Result<(), XlnNewPartError> {
        let replacement_fanins = self.node(input)?.fanins.clone();
        let out_node = self
            .nodes
            .get_mut(out.0)
            .ok_or(XlnNewPartError::UnknownNode(out))?;
        if out_node.deleted {
            return Err(XlnNewPartError::DeletedNode(out));
        }
        if !out_node.fanins.contains(&input) {
            return Err(XlnNewPartError::NotAFanin { out, input });
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

    pub fn delete_node(&mut self, node: NodeId) -> Result<(), XlnNewPartError> {
        let target = self
            .nodes
            .get_mut(node.0)
            .ok_or(XlnNewPartError::UnknownNode(node))?;
        target.deleted = true;
        Ok(())
    }

    pub fn sweep_deleted_fanins(&mut self) {
        let deleted: HashSet<NodeId> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| node.deleted.then_some(NodeId(index)))
            .collect();
        for node in &mut self.nodes {
            node.fanins.retain(|fanin| !deleted.contains(fanin));
        }
    }
}

impl Default for PldNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XlnOptions {
    pub size: usize,
    pub move_fanins: bool,
    pub max_fanins: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollapseCandidate {
    pub value: usize,
    pub input: NodeId,
    pub out: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartitionStats {
    pub trivial_collapses: usize,
    pub selected_collapses: usize,
    pub iterations: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrivialCollapseResult {
    Collapsed,
    Ineligible,
    MissedFeasibility { diff: isize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnNewPartError {
    UnknownNode(NodeId),
    DeletedNode(NodeId),
    NotAFanin {
        out: NodeId,
        input: NodeId,
    },
    PrimaryOutputAsCollapseTarget(NodeId),
    CompositeFaninExceeded {
        out: NodeId,
        input: NodeId,
        composite_fanin: usize,
        size: usize,
    },
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for XlnNewPartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_new_part node {:?}", node),
            Self::DeletedNode(node) => write!(f, "xln_new_part node {:?} was deleted", node),
            Self::NotAFanin { out, input } => {
                write!(f, "node {:?} is not a fanin of {:?}", input, out)
            }
            Self::PrimaryOutputAsCollapseTarget(node) => {
                write!(f, "cannot collapse into primary-output node {:?}", node)
            }
            Self::CompositeFaninExceeded {
                out,
                input,
                composite_fanin,
                size,
            } => write!(
                f,
                "composite fanin of ({:?}, {:?}) is {composite_fanin}, exceeding size {size}",
                input, out
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

impl Error for XlnNewPartError {}

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn composite_fanin_count(
    network: &PldNetwork,
    node1: NodeId,
    node2: NodeId,
) -> Result<usize, XlnNewPartError> {
    let node1_ref = network.node(node1)?;
    let node2_ref = network.node(node2)?;
    if node1_ref.fanins.is_empty() {
        return Ok(node2_ref.fanins.len());
    }

    let existing: HashSet<NodeId> = node1_ref
        .fanins
        .iter()
        .copied()
        .filter(|fanin| *fanin != node2)
        .collect();
    let additional = node2_ref
        .fanins
        .iter()
        .filter(|fanin| !existing.contains(fanin))
        .count();
    Ok(existing.len() + additional)
}

pub fn cost_fanin(
    network: &PldNetwork,
    po: NodeId,
    fanin: NodeId,
) -> Result<usize, XlnNewPartError> {
    let po_node = network.node(po)?;
    let fanin_node = network.node(fanin)?;
    let shared = po_node
        .fanins
        .iter()
        .filter(|po_fanin| fanin_node.fanins.contains(po_fanin))
        .count();
    Ok(fanin_node.fanins.len().saturating_sub(shared))
}

pub fn collect_collapse_candidates(
    network: &PldNetwork,
    size: usize,
) -> Result<Vec<CollapseCandidate>, XlnNewPartError> {
    let mut candidates = Vec::new();
    for out in network.internal_node_ids() {
        let out_node = network.node(out)?;
        for input in out_node.fanins.iter().copied() {
            if network.node(input)?.kind == NodeKind::PrimaryInput {
                continue;
            }
            if composite_fanin_count(network, out, input)? <= size {
                let value = network
                    .fanouts(input)?
                    .into_iter()
                    .map(|fanout| cost_fanin(network, fanout, input))
                    .try_fold(0usize, |sum, cost| cost.map(|cost| sum + cost))?;
                candidates.push(CollapseCandidate { value, input, out });
            }
        }
    }
    candidates.sort_by(compare_candidates);
    Ok(candidates)
}

pub fn compare_candidates(left: &CollapseCandidate, right: &CollapseCandidate) -> Ordering {
    left.value
        .cmp(&right.value)
        .then_with(|| left.input.cmp(&right.input))
        .then_with(|| left.out.cmp(&right.out))
}

pub fn select_disjoint_candidates(
    candidates: Vec<CollapseCandidate>,
    internal_order: &[NodeId],
) -> Vec<CollapseCandidate> {
    let index: HashMap<NodeId, usize> = internal_order
        .iter()
        .copied()
        .enumerate()
        .map(|(position, node)| (node, position))
        .collect();
    let mut used_outputs = vec![false; internal_order.len()];
    let mut selected = Vec::new();

    for candidate in candidates {
        let Some(&input_index) = index.get(&candidate.input) else {
            continue;
        };
        let Some(&out_index) = index.get(&candidate.out) else {
            continue;
        };
        if !used_outputs[input_index] && !used_outputs[out_index] {
            used_outputs[out_index] = true;
            selected.push(candidate);
        }
    }

    selected
}

pub fn trivial_collapse_node_without_moving(
    network: &mut PldNetwork,
    node: NodeId,
    size: usize,
) -> Result<TrivialCollapseResult, XlnNewPartError> {
    if network.node(node)?.kind != NodeKind::Internal {
        return Ok(TrivialCollapseResult::Ineligible);
    }

    let fanouts = network.fanouts(node)?;
    if fanouts.is_empty() {
        return Ok(TrivialCollapseResult::Ineligible);
    }

    let mut missed_by = isize::MAX;
    for fanout in &fanouts {
        if network.node(*fanout)?.kind == NodeKind::PrimaryOutput {
            return Ok(TrivialCollapseResult::Ineligible);
        }
        let composite = composite_fanin_count(network, *fanout, node)?;
        if composite > size {
            missed_by = missed_by.min(size as isize - composite as isize);
        }
    }

    if missed_by != isize::MAX {
        return Ok(TrivialCollapseResult::MissedFeasibility { diff: missed_by });
    }

    for fanout in fanouts {
        network.collapse_fanin(fanout, node)?;
    }
    Ok(TrivialCollapseResult::Collapsed)
}

pub fn trivial_collapse_node(
    network: &mut PldNetwork,
    node: NodeId,
    options: XlnOptions,
) -> Result<TrivialCollapseResult, XlnNewPartError> {
    if options.move_fanins {
        return Err(XlnNewPartError::MissingNativePorts {
            operation: "xln_node_move_fanins",
            dependencies: REQUIRED_PORT_DEPENDENCIES,
        });
    }
    trivial_collapse_node_without_moving(network, node, options.size)
}

pub fn trivial_collapse_network_one_iter(
    network: &mut PldNetwork,
    options: XlnOptions,
) -> Result<usize, XlnNewPartError> {
    let mut num_collapsed = 0;
    for node in network.internal_node_ids() {
        if trivial_collapse_node(network, node, options)? == TrivialCollapseResult::Collapsed {
            network.delete_node(node)?;
            num_collapsed += 1;
        }
    }
    Ok(num_collapsed)
}

pub fn trivial_collapse_network(
    network: &mut PldNetwork,
    options: XlnOptions,
) -> Result<usize, XlnNewPartError> {
    let mut total = 0;
    loop {
        let collapsed = trivial_collapse_network_one_iter(network, options)?;
        if collapsed == 0 {
            return Ok(total);
        }
        total += collapsed;
        network.sweep_deleted_fanins();
    }
}

pub fn imp_part_network(
    network: &mut PldNetwork,
    options: XlnOptions,
) -> Result<PartitionStats, XlnNewPartError> {
    let mut stats = PartitionStats {
        trivial_collapses: 0,
        selected_collapses: 0,
        iterations: 0,
    };

    loop {
        let mut changed = false;
        let trivial = trivial_collapse_network(network, options)?;
        stats.trivial_collapses += trivial;
        changed |= trivial > 0;

        let internal_order = network.internal_node_ids();
        let candidates = collect_collapse_candidates(network, options.size)?;
        let selected = select_disjoint_candidates(candidates, &internal_order);

        for candidate in selected {
            let composite = composite_fanin_count(network, candidate.out, candidate.input)?;
            if composite > options.size {
                return Err(XlnNewPartError::CompositeFaninExceeded {
                    out: candidate.out,
                    input: candidate.input,
                    composite_fanin: composite,
                    size: options.size,
                });
            }
            network.collapse_fanin(candidate.out, candidate.input)?;
            stats.selected_collapses += 1;
            changed = true;
        }

        if !changed {
            return Ok(stats);
        }
        stats.iterations += 1;
        network.sweep_deleted_fanins();
    }
}

pub fn imp_part_sis_network_blocked<Network>(
    _network: &mut Network,
    _options: XlnOptions,
) -> Result<PartitionStats, XlnNewPartError> {
    Err(XlnNewPartError::MissingNativePorts {
        operation: "imp_part_network",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_names<'a>(network: &'a PldNetwork, ids: &[NodeId]) -> Vec<&'a str> {
        ids.iter()
            .map(|id| network.node(*id).unwrap().name.as_str())
            .collect()
    }

    #[test]
    fn composite_fanin_replaces_shared_input_with_distinct_fanins() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(PldNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(PldNode::new("c", NodeKind::PrimaryInput));
        let x = network.add_node(PldNode::new("x", NodeKind::Internal).with_fanins(vec![a, b]));
        let y = network.add_node(PldNode::new("y", NodeKind::Internal).with_fanins(vec![x, c]));

        assert_eq!(composite_fanin_count(&network, y, x).unwrap(), 3);
        assert_eq!(cost_fanin(&network, y, x).unwrap(), 2);
    }

    #[test]
    fn candidates_are_sorted_by_c_cost_and_skip_primary_inputs() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(PldNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(PldNode::new("c", NodeKind::PrimaryInput));
        let x = network.add_node(PldNode::new("x", NodeKind::Internal).with_fanins(vec![a, b]));
        let y = network.add_node(PldNode::new("y", NodeKind::Internal).with_fanins(vec![b, c]));
        network.add_node(PldNode::new("n1", NodeKind::Internal).with_fanins(vec![x, y]));
        network.add_node(PldNode::new("n2", NodeKind::Internal).with_fanins(vec![x, c]));

        let candidates = collect_collapse_candidates(&network, 4).unwrap();

        assert_eq!(
            candidates,
            vec![
                CollapseCandidate {
                    value: 2,
                    input: y,
                    out: NodeId(5),
                },
                CollapseCandidate {
                    value: 4,
                    input: x,
                    out: NodeId(5),
                },
                CollapseCandidate {
                    value: 4,
                    input: x,
                    out: NodeId(6),
                },
            ]
        );
    }

    #[test]
    fn disjoint_selection_marks_outputs_like_array_disjoint() {
        let candidates = vec![
            CollapseCandidate {
                value: 1,
                input: NodeId(0),
                out: NodeId(1),
            },
            CollapseCandidate {
                value: 2,
                input: NodeId(1),
                out: NodeId(2),
            },
            CollapseCandidate {
                value: 3,
                input: NodeId(0),
                out: NodeId(2),
            },
        ];

        assert_eq!(
            select_disjoint_candidates(candidates, &[NodeId(0), NodeId(1), NodeId(2)]),
            vec![
                CollapseCandidate {
                    value: 1,
                    input: NodeId(0),
                    out: NodeId(1),
                },
                CollapseCandidate {
                    value: 3,
                    input: NodeId(0),
                    out: NodeId(2),
                },
            ]
        );
    }

    #[test]
    fn trivial_collapse_rewrites_all_internal_fanouts_and_deletes_node() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(PldNode::new("b", NodeKind::PrimaryInput));
        let x = network.add_node(PldNode::new("x", NodeKind::Internal).with_fanins(vec![a, b]));
        let n1 = network.add_node(PldNode::new("n1", NodeKind::Internal).with_fanins(vec![x]));
        let n2 = network.add_node(PldNode::new("n2", NodeKind::Internal).with_fanins(vec![x, b]));

        let collapsed = trivial_collapse_network(
            &mut network,
            XlnOptions {
                size: 3,
                move_fanins: false,
                max_fanins: 8,
            },
        )
        .unwrap();

        assert_eq!(collapsed, 1);
        assert!(network.node(x).unwrap().is_deleted());
        assert_eq!(network.node(n1).unwrap().fanins, vec![a, b]);
        assert_eq!(network.node(n2).unwrap().fanins, vec![a, b]);
    }

    #[test]
    fn primary_output_fanout_blocks_trivial_collapse() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput));
        let x = network.add_node(PldNode::new("x", NodeKind::Internal).with_fanins(vec![a]));
        network.add_node(PldNode::new("out", NodeKind::PrimaryOutput).with_fanins(vec![x]));

        assert_eq!(
            trivial_collapse_node_without_moving(&mut network, x, 2).unwrap(),
            TrivialCollapseResult::Ineligible
        );
        assert!(!network.node(x).unwrap().is_deleted());
    }

    #[test]
    fn partition_network_applies_ranked_nontrivial_candidates() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(PldNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(PldNode::new("c", NodeKind::PrimaryInput));
        let x = network.add_node(PldNode::new("x", NodeKind::Internal).with_fanins(vec![a, b]));
        let y = network.add_node(PldNode::new("y", NodeKind::Internal).with_fanins(vec![x, c]));
        network.add_node(PldNode::new("out", NodeKind::PrimaryOutput).with_fanins(vec![y]));
        network.add_node(PldNode::new("x_out", NodeKind::PrimaryOutput).with_fanins(vec![x]));

        let stats = imp_part_network(
            &mut network,
            XlnOptions {
                size: 3,
                move_fanins: false,
                max_fanins: 8,
            },
        )
        .unwrap();

        assert_eq!(stats.selected_collapses, 1);
        assert_eq!(
            node_names(&network, &network.node(y).unwrap().fanins),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn move_fanins_reports_explicit_dependency() {
        let mut network = PldNetwork::new();
        let node = network.add_node(PldNode::new("n", NodeKind::Internal));

        let error = trivial_collapse_node(
            &mut network,
            node,
            XlnOptions {
                size: 4,
                move_fanins: true,
                max_fanins: 8,
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            XlnNewPartError::MissingNativePorts {
                operation: "xln_node_move_fanins",
                dependencies: REQUIRED_PORT_DEPENDENCIES,
            }
        );
        assert!(required_port_dependencies().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.391"
                && dependency.source_file == "LogicSynthesis/sis/pld/xln_move_d.c"
        }));
    }

    #[test]
    fn sis_bound_entry_reports_dependency_beads_and_sources() {
        let mut network = ();
        let error = imp_part_sis_network_blocked(
            &mut network,
            XlnOptions {
                size: 4,
                move_fanins: false,
                max_fanins: 8,
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            XlnNewPartError::MissingNativePorts {
                operation: "imp_part_network",
                dependencies: REQUIRED_PORT_DEPENDENCIES,
            }
        );
        assert!(
            error
                .to_string()
                .contains("unported SIS C-file dependencies")
        );
        assert!(required_port_dependencies().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.309"
                && dependency.source_file == "LogicSynthesis/sis/node/collapse.c"
        }));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_new_part.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
