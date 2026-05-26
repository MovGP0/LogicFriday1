//! Native Rust port for `sis/speed/absorb.c`.
//!
//! The C implementation finds critical nodes in the transitive fanin of a root,
//! collapses those nodes into the root until none remain as direct fanins,
//! simplifies the root, and removes original fanin-cone nodes that become
//! fanoutless. This module keeps that control flow native and delegates graph
//! mutation to an idiomatic Rust backend trait.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    Internal,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbsorbParameters {
    pub distance: usize,
    pub debug: bool,
}

impl Default for AbsorbParameters {
    fn default() -> Self {
        Self {
            distance: 3,
            debug: false,
        }
    }
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
pub struct AbsorbSelection<N: Eq + Hash = NodeId> {
    pub collapse_order: Vec<N>,
    pub cache_depths: HashMap<N, usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollapseAttempt<N> {
    pub root: N,
    pub source: N,
    pub changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbsorbWarning<N> {
    CollapsingNodeWithManyFanins {
        root: N,
        source: N,
        fanin_count: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbsorbReport<N: Eq + Hash> {
    pub selection: AbsorbSelection<N>,
    pub collapse_attempts: Vec<CollapseAttempt<N>>,
    pub warnings: Vec<AbsorbWarning<N>>,
    pub deleted_nodes: Vec<N>,
}

pub trait AbsorbBackend {
    type NodeId: Clone + Eq + Hash + ToString;

    fn node_kind(&self, node: &Self::NodeId) -> Result<NodeKind, AbsorbError>;

    fn fanins(&self, node: &Self::NodeId) -> Result<Vec<Self::NodeId>, AbsorbError>;

    fn fanout_count(&self, node: &Self::NodeId) -> Result<usize, AbsorbError>;

    fn is_critical(
        &self,
        node: &Self::NodeId,
        params: &AbsorbParameters,
    ) -> Result<bool, AbsorbError>;

    fn collapse(&mut self, root: &Self::NodeId, source: &Self::NodeId)
    -> Result<bool, AbsorbError>;

    fn simplify_replace(&mut self, root: &Self::NodeId) -> Result<(), AbsorbError>;

    fn delete_node(&mut self, node: &Self::NodeId) -> Result<(), AbsorbError>;
}

pub fn speed_absorb<Network>(
    network: &mut Network,
    root: Network::NodeId,
    params: &AbsorbParameters,
) -> Result<AbsorbReport<Network::NodeId>, AbsorbError>
where
    Network: AbsorbBackend,
{
    if network.node_kind(&root)? != NodeKind::Internal {
        return Err(AbsorbError::RootIsNotInternal(root.to_string()));
    }

    let selection = select_absorb_nodes_from_backend(network, root.clone(), params)?;
    speed_absorb_array(network, root, params, selection)
}

pub fn speed_absorb_array<Network>(
    network: &mut Network,
    root: Network::NodeId,
    params: &AbsorbParameters,
    selection: AbsorbSelection<Network::NodeId>,
) -> Result<AbsorbReport<Network::NodeId>, AbsorbError>
where
    Network: AbsorbBackend,
{
    let original_fanins = network.fanins(&root)?;
    let mut collapse_attempts = Vec::new();
    let mut warnings = Vec::new();

    loop {
        for source in selection.collapse_order.iter().skip(1) {
            let fanin_count = network.fanins(source)?.len();
            if params.debug && fanin_count > 2 {
                warnings.push(AbsorbWarning::CollapsingNodeWithManyFanins {
                    root: root.clone(),
                    source: source.clone(),
                    fanin_count,
                });
            }

            let changed = network.collapse(&root, source)?;
            collapse_attempts.push(CollapseAttempt {
                root: root.clone(),
                source: source.clone(),
                changed,
            });
        }

        let mut more_to_come = false;
        for fanin in network.fanins(&root)? {
            if network.node_kind(&fanin)? != NodeKind::PrimaryInput
                && selection.cache_depths.contains_key(&fanin)
            {
                more_to_come = true;
                break;
            }
        }

        if !more_to_come {
            break;
        }
    }

    network.simplify_replace(&root)?;
    let deleted_nodes = delete_fanoutless_cone(network, original_fanins)?;

    Ok(AbsorbReport {
        selection,
        collapse_attempts,
        warnings,
        deleted_nodes,
    })
}

pub fn select_absorb_nodes(
    root: NodeId,
    nodes: &HashMap<NodeId, AbsorbNode>,
    distance: usize,
) -> Result<AbsorbSelection, AbsorbError> {
    let root_node = nodes
        .get(&root)
        .ok_or_else(|| AbsorbError::UnknownNode(format!("{root:?}")))?;
    if root_node.kind != NodeKind::Internal {
        return Err(AbsorbError::RootIsNotInternal(format!("{root:?}")));
    }

    let params = AbsorbParameters {
        distance,
        debug: false,
    };
    let backend = AbsorbSnapshot { nodes };
    select_absorb_nodes_from_backend(&backend, root, &params)
}

pub fn deletion_candidates_after_absorb(
    root_initial_fanins: &[NodeId],
    nodes: &HashMap<NodeId, AbsorbNode>,
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
            .ok_or_else(|| AbsorbError::UnknownNode(format!("{node_id:?}")))?;
        if node.fanout_count == 0 {
            deletion.push(node_id);
            queue.extend(node.fanins.iter().copied());
        }
    }

    Ok(deletion)
}

fn select_absorb_nodes_from_backend<Network>(
    network: &Network,
    root: Network::NodeId,
    params: &AbsorbParameters,
) -> Result<AbsorbSelection<Network::NodeId>, AbsorbError>
where
    Network: AbsorbBackend,
{
    let mut collapse_order = vec![root.clone()];
    let mut cache_depths = HashMap::from([(root.clone(), params.distance + 1)]);
    let mut first = 0;
    let mut remaining = params.distance;

    while remaining > 0 && first < collapse_order.len() {
        let last = collapse_order.len();
        for node in collapse_order[first..last].to_vec() {
            for fanin in network.fanins(&node)? {
                match network.node_kind(&fanin)? {
                    NodeKind::PrimaryInput => {
                        cache_depths.insert(fanin, 0);
                    }
                    _ if !cache_depths.contains_key(&fanin)
                        && network.is_critical(&fanin, params)? =>
                    {
                        cache_depths.insert(fanin.clone(), remaining);
                        collapse_order.push(fanin);
                    }
                    _ => {}
                }
            }
        }
        first = last;
        remaining -= 1;
    }

    Ok(AbsorbSelection {
        collapse_order,
        cache_depths,
    })
}

fn delete_fanoutless_cone<Network>(
    network: &mut Network,
    initial_nodes: Vec<Network::NodeId>,
) -> Result<Vec<Network::NodeId>, AbsorbError>
where
    Network: AbsorbBackend,
{
    let mut deleted = Vec::new();
    let mut processed = HashSet::new();
    let mut queue = VecDeque::from(initial_nodes);

    while let Some(node) = queue.pop_front() {
        if !processed.insert(node.clone()) {
            continue;
        }

        if network.fanout_count(&node)? != 0 {
            continue;
        }

        queue.extend(network.fanins(&node)?);
        network.delete_node(&node)?;
        deleted.push(node);
    }

    Ok(deleted)
}

struct AbsorbSnapshot<'a> {
    nodes: &'a HashMap<NodeId, AbsorbNode>,
}

impl AbsorbBackend for AbsorbSnapshot<'_> {
    type NodeId = NodeId;

    fn node_kind(&self, node: &Self::NodeId) -> Result<NodeKind, AbsorbError> {
        Ok(self
            .nodes
            .get(node)
            .ok_or_else(|| AbsorbError::UnknownNode(format!("{node:?}")))?
            .kind)
    }

    fn fanins(&self, node: &Self::NodeId) -> Result<Vec<Self::NodeId>, AbsorbError> {
        Ok(self
            .nodes
            .get(node)
            .ok_or_else(|| AbsorbError::UnknownNode(format!("{node:?}")))?
            .fanins
            .clone())
    }

    fn fanout_count(&self, node: &Self::NodeId) -> Result<usize, AbsorbError> {
        Ok(self
            .nodes
            .get(node)
            .ok_or_else(|| AbsorbError::UnknownNode(format!("{node:?}")))?
            .fanout_count)
    }

    fn is_critical(
        &self,
        node: &Self::NodeId,
        _params: &AbsorbParameters,
    ) -> Result<bool, AbsorbError> {
        Ok(self
            .nodes
            .get(node)
            .ok_or_else(|| AbsorbError::UnknownNode(format!("{node:?}")))?
            .critical)
    }

    fn collapse(
        &mut self,
        _root: &Self::NodeId,
        _source: &Self::NodeId,
    ) -> Result<bool, AbsorbError> {
        unreachable!("snapshot backend is read-only")
    }

    fn simplify_replace(&mut self, _root: &Self::NodeId) -> Result<(), AbsorbError> {
        unreachable!("snapshot backend is read-only")
    }

    fn delete_node(&mut self, _node: &Self::NodeId) -> Result<(), AbsorbError> {
        unreachable!("snapshot backend is read-only")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbsorbError {
    UnknownNode(String),
    RootIsNotInternal(String),
    Backend(String),
}

impl fmt::Display for AbsorbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown absorb node {node}"),
            Self::RootIsNotInternal(node) => {
                write!(f, "can only absorb internal nodes; {node} is not internal")
            }
            Self::Backend(message) => f.write_str(message),
        }
    }
}

impl Error for AbsorbError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestNode {
        kind: NodeKind,
        fanins: Vec<NodeId>,
        fanouts: Vec<NodeId>,
        critical: bool,
        simplified: bool,
    }

    #[derive(Clone, Debug, Default)]
    struct TestNetwork {
        nodes: HashMap<NodeId, TestNode>,
    }

    impl TestNetwork {
        fn add(
            &mut self,
            id: usize,
            kind: NodeKind,
            fanins: &[usize],
            fanouts: &[usize],
            critical: bool,
        ) {
            self.nodes.insert(
                NodeId(id),
                TestNode {
                    kind,
                    fanins: fanins.iter().copied().map(NodeId).collect(),
                    fanouts: fanouts.iter().copied().map(NodeId).collect(),
                    critical,
                    simplified: false,
                },
            );
        }

        fn node(&self, node: &NodeId) -> Result<&TestNode, AbsorbError> {
            self.nodes
                .get(node)
                .ok_or_else(|| AbsorbError::UnknownNode(format!("{node:?}")))
        }

        fn node_mut(&mut self, node: &NodeId) -> Result<&mut TestNode, AbsorbError> {
            self.nodes
                .get_mut(node)
                .ok_or_else(|| AbsorbError::UnknownNode(format!("{node:?}")))
        }
    }

    impl AbsorbBackend for TestNetwork {
        type NodeId = NodeId;

        fn node_kind(&self, node: &Self::NodeId) -> Result<NodeKind, AbsorbError> {
            Ok(self.node(node)?.kind)
        }

        fn fanins(&self, node: &Self::NodeId) -> Result<Vec<Self::NodeId>, AbsorbError> {
            Ok(self.node(node)?.fanins.clone())
        }

        fn fanout_count(&self, node: &Self::NodeId) -> Result<usize, AbsorbError> {
            Ok(self.node(node)?.fanouts.len())
        }

        fn is_critical(
            &self,
            node: &Self::NodeId,
            _params: &AbsorbParameters,
        ) -> Result<bool, AbsorbError> {
            Ok(self.node(node)?.critical)
        }

        fn collapse(
            &mut self,
            root: &Self::NodeId,
            source: &Self::NodeId,
        ) -> Result<bool, AbsorbError> {
            let source_fanins = self.node(source)?.fanins.clone();
            let root_fanins = self.node(root)?.fanins.clone();
            let mut changed = false;
            let mut replacement = Vec::new();

            for fanin in root_fanins {
                if &fanin == source {
                    changed = true;
                    replacement.extend(source_fanins.iter().copied());
                } else {
                    replacement.push(fanin);
                }
            }

            if !changed {
                return Ok(false);
            }

            self.node_mut(root)?.fanins = replacement;
            self.node_mut(source)?
                .fanouts
                .retain(|fanout| fanout != root);

            for fanin in source_fanins {
                let fanin_node = self.node_mut(&fanin)?;
                if !fanin_node.fanouts.contains(root) {
                    fanin_node.fanouts.push(*root);
                }
            }

            Ok(true)
        }

        fn simplify_replace(&mut self, root: &Self::NodeId) -> Result<(), AbsorbError> {
            self.node_mut(root)?.simplified = true;
            Ok(())
        }

        fn delete_node(&mut self, node: &Self::NodeId) -> Result<(), AbsorbError> {
            let removed = self
                .nodes
                .remove(node)
                .ok_or_else(|| AbsorbError::UnknownNode(format!("{node:?}")))?;
            for fanin in removed.fanins {
                if let Some(fanin_node) = self.nodes.get_mut(&fanin) {
                    fanin_node.fanouts.retain(|fanout| fanout != node);
                }
            }
            Ok(())
        }
    }

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
            Err(AbsorbError::RootIsNotInternal("NodeId(1)".to_string()))
        );
    }

    #[test]
    fn deletion_candidates_walk_fanin_tree_for_fanoutless_nodes() {
        let nodes = HashMap::from([
            (NodeId(2), node(2, NodeKind::Internal, &[4], 0, true)),
            (NodeId(3), node(3, NodeKind::Internal, &[], 2, true)),
            (NodeId(4), node(4, NodeKind::PrimaryInput, &[], 0, false)),
        ]);

        assert_eq!(
            deletion_candidates_after_absorb(&[NodeId(2), NodeId(3)], &nodes).unwrap(),
            vec![NodeId(2), NodeId(4)]
        );
    }

    #[test]
    fn speed_absorb_collapses_simplifies_and_deletes_fanoutless_cone() {
        let mut network = TestNetwork::default();
        network.add(1, NodeKind::Internal, &[2, 3], &[], true);
        network.add(2, NodeKind::Internal, &[4], &[1], true);
        network.add(3, NodeKind::Internal, &[], &[1], false);
        network.add(4, NodeKind::PrimaryInput, &[], &[2], false);

        let report = speed_absorb(
            &mut network,
            NodeId(1),
            &AbsorbParameters {
                distance: 2,
                debug: false,
            },
        )
        .unwrap();

        assert_eq!(report.selection.collapse_order, vec![NodeId(1), NodeId(2)]);
        assert_eq!(
            report.collapse_attempts,
            vec![CollapseAttempt {
                root: NodeId(1),
                source: NodeId(2),
                changed: true,
            }]
        );
        assert!(network.node(&NodeId(1)).unwrap().simplified);
        assert_eq!(
            network.node(&NodeId(1)).unwrap().fanins,
            vec![NodeId(4), NodeId(3)]
        );
        assert_eq!(report.deleted_nodes, vec![NodeId(2)]);
        assert!(!network.nodes.contains_key(&NodeId(2)));
    }

    #[test]
    fn speed_absorb_array_repeats_until_selected_fanins_leave_root() {
        let mut network = TestNetwork::default();
        network.add(1, NodeKind::Internal, &[2], &[], true);
        network.add(2, NodeKind::Internal, &[3], &[1], true);
        network.add(3, NodeKind::Internal, &[4], &[2], true);
        network.add(4, NodeKind::PrimaryInput, &[], &[3], false);

        let selection = AbsorbSelection {
            collapse_order: vec![NodeId(1), NodeId(2), NodeId(3)],
            cache_depths: HashMap::from([(NodeId(1), 3), (NodeId(2), 2), (NodeId(3), 1)]),
        };

        let report = speed_absorb_array(
            &mut network,
            NodeId(1),
            &AbsorbParameters::default(),
            selection,
        )
        .unwrap();

        assert_eq!(
            report
                .collapse_attempts
                .iter()
                .map(|attempt| (attempt.source, attempt.changed))
                .collect::<Vec<_>>(),
            vec![(NodeId(2), true), (NodeId(3), true)]
        );
        assert_eq!(network.node(&NodeId(1)).unwrap().fanins, vec![NodeId(4)]);
        assert_eq!(report.deleted_nodes, vec![NodeId(2), NodeId(3)]);
    }

    #[test]
    fn debug_mode_reports_large_collapse_source_warning() {
        let mut network = TestNetwork::default();
        network.add(1, NodeKind::Internal, &[2], &[], true);
        network.add(2, NodeKind::Internal, &[3, 4, 5], &[1], true);
        network.add(3, NodeKind::PrimaryInput, &[], &[2], false);
        network.add(4, NodeKind::PrimaryInput, &[], &[2], false);
        network.add(5, NodeKind::PrimaryInput, &[], &[2], false);

        let report = speed_absorb(
            &mut network,
            NodeId(1),
            &AbsorbParameters {
                distance: 1,
                debug: true,
            },
        )
        .unwrap();

        assert_eq!(
            report.warnings,
            vec![AbsorbWarning::CollapsingNodeWithManyFanins {
                root: NodeId(1),
                source: NodeId(2),
                fanin_count: 3,
            }]
        );
    }
}
