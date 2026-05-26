//! Native Rust port scaffold for `sis/speed/absorb.c`.
//!
//! The C code finds critical transitive fanin nodes within a distance, then
//! repeatedly collapses them into a root and deletes fanin nodes that lose all
//! fanout. This module ports the BFS selection and cleanup planning. Actual SIS
//! node collapse, simplification, and network deletion are represented as
//! blocked operations until the node/network ports exist.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    Internal,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbsorbNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub fanout_count: usize,
    pub critical: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbsorbSelection {
    pub collapse_order: Vec<NodeId>,
    pub cache_depths: HashMap<NodeId, usize>,
}

pub fn select_absorb_nodes(
    root: NodeId,
    nodes: &HashMap<NodeId, AbsorbNode>,
    distance: usize,
) -> Result<AbsorbSelection, AbsorbError> {
    let root_node = nodes.get(&root).ok_or(AbsorbError::UnknownNode(root))?;
    if root_node.kind != NodeKind::Internal {
        return Err(AbsorbError::RootIsNotInternal(root));
    }

    let mut collapse_order = vec![root];
    let mut cache_depths = HashMap::from([(root, distance + 1)]);
    let mut frontier = VecDeque::from([(root, distance)]);

    while let Some((node_id, remaining)) = frontier.pop_front() {
        if remaining == 0 {
            continue;
        }
        let node = nodes
            .get(&node_id)
            .ok_or(AbsorbError::UnknownNode(node_id))?;
        for fanin in &node.fanins {
            let fanin_node = nodes.get(fanin).ok_or(AbsorbError::UnknownNode(*fanin))?;
            if fanin_node.kind == NodeKind::PrimaryInput {
                cache_depths.insert(*fanin, 0);
            } else if !cache_depths.contains_key(fanin) && fanin_node.critical {
                cache_depths.insert(*fanin, remaining);
                collapse_order.push(*fanin);
                frontier.push_back((*fanin, remaining - 1));
            }
        }
    }

    Ok(AbsorbSelection {
        collapse_order,
        cache_depths,
    })
}

pub fn deletion_candidates_after_absorb(
    root_initial_fanins: &[NodeId],
    nodes: &HashMap<NodeId, AbsorbNode>,
    retained: &HashSet<NodeId>,
) -> Result<Vec<NodeId>, AbsorbError> {
    let mut deletion = Vec::new();
    let mut seen = HashSet::new();
    let mut queue: VecDeque<NodeId> = root_initial_fanins.iter().copied().collect();

    while let Some(node_id) = queue.pop_front() {
        if !seen.insert(node_id) {
            continue;
        }
        let node = nodes
            .get(&node_id)
            .ok_or(AbsorbError::UnknownNode(node_id))?;
        if node.fanout_count == 0 && !retained.contains(&node_id) {
            deletion.push(node_id);
            for fanin in &node.fanins {
                queue.push_back(*fanin);
            }
        }
    }

    Ok(deletion)
}

pub fn speed_absorb_bound() -> Result<(), AbsorbError> {
    Err(AbsorbError::MissingDependency(
        "speed_absorb requires native node collapse/simplify/fanin/fanout APIs, network deletion, and speed criticality ports",
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbsorbError {
    UnknownNode(NodeId),
    RootIsNotInternal(NodeId),
    MissingDependency(&'static str),
}

impl fmt::Display for AbsorbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown absorb node {:?}", node),
            Self::RootIsNotInternal(node) => {
                write!(
                    f,
                    "can only absorb internal nodes; {:?} is not internal",
                    node
                )
            }
            Self::MissingDependency(message) => write!(f, "{message}"),
        }
    }
}

impl Error for AbsorbError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(
        id: usize,
        kind: NodeKind,
        fanins: &[usize],
        fanout_count: usize,
        critical: bool,
    ) -> AbsorbNode {
        AbsorbNode {
            id: NodeId(id),
            kind,
            fanins: fanins.iter().copied().map(NodeId).collect(),
            fanout_count,
            critical,
        }
    }

    #[test]
    fn selection_follows_critical_fanins_until_distance_expires() {
        let nodes = HashMap::from([
            (NodeId(1), node(1, NodeKind::Internal, &[2, 3], 1, true)),
            (NodeId(2), node(2, NodeKind::Internal, &[4], 1, true)),
            (NodeId(3), node(3, NodeKind::Internal, &[5], 1, false)),
            (NodeId(4), node(4, NodeKind::PrimaryInput, &[], 1, false)),
            (NodeId(5), node(5, NodeKind::PrimaryInput, &[], 1, false)),
        ]);

        let selection = select_absorb_nodes(NodeId(1), &nodes, 2).unwrap();

        assert_eq!(selection.collapse_order, vec![NodeId(1), NodeId(2)]);
        assert_eq!(selection.cache_depths[&NodeId(1)], 3);
        assert_eq!(selection.cache_depths[&NodeId(2)], 2);
        assert_eq!(selection.cache_depths[&NodeId(4)], 0);
        assert!(!selection.cache_depths.contains_key(&NodeId(3)));
    }

    #[test]
    fn selection_rejects_non_internal_roots() {
        let nodes = HashMap::from([(NodeId(1), node(1, NodeKind::PrimaryInput, &[], 1, false))]);

        assert_eq!(
            select_absorb_nodes(NodeId(1), &nodes, 2),
            Err(AbsorbError::RootIsNotInternal(NodeId(1)))
        );
    }

    #[test]
    fn deletion_candidates_walk_fanin_tree_for_fanoutless_nodes() {
        let nodes = HashMap::from([
            (NodeId(2), node(2, NodeKind::Internal, &[4], 0, true)),
            (NodeId(3), node(3, NodeKind::Internal, &[], 2, true)),
            (NodeId(4), node(4, NodeKind::PrimaryInput, &[], 0, false)),
        ]);
        let retained = HashSet::from([NodeId(3)]);

        assert_eq!(
            deletion_candidates_after_absorb(&[NodeId(2), NodeId(3)], &nodes, &retained).unwrap(),
            vec![NodeId(2), NodeId(4)]
        );
    }

    #[test]
    fn network_bound_entry_reports_missing_dependencies() {
        assert_eq!(
            speed_absorb_bound(),
            Err(AbsorbError::MissingDependency(
                "speed_absorb requires native node collapse/simplify/fanin/fanout APIs, network deletion, and speed criticality ports",
            ))
        );
    }
}
