//! Native network depth-first traversals.
//!
//! The legacy SIS implementation returns node arrays ordered by depth-first
//! postorder and aborts when it sees a cycle.  This Rust version keeps the same
//! traversal semantics on an owned graph model and reports invalid input through
//! `Result` values.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const INFINITY_LEVEL: usize = usize::MAX;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub is_control_output: bool,
}

impl NetworkNode {
    pub fn new(id: usize, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind,
            fanins: Vec::new(),
            is_control_output: false,
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_control_output(mut self, is_control_output: bool) -> Self {
        self.is_control_output = is_control_output;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Network {
    nodes: Vec<NetworkNode>,
    positions: HashMap<NodeId, usize>,
    fanouts: HashMap<NodeId, Vec<NodeId>>,
}

impl Network {
    pub fn new(nodes: Vec<NetworkNode>) -> Result<Self, NetworkDfsError> {
        let mut positions = HashMap::with_capacity(nodes.len());
        for (position, node) in nodes.iter().enumerate() {
            if positions.insert(node.id, position).is_some() {
                return Err(NetworkDfsError::DuplicateNode { node_id: node.id });
            }
        }

        let mut fanouts: HashMap<NodeId, Vec<NodeId>> =
            nodes.iter().map(|node| (node.id, Vec::new())).collect();

        for node in &nodes {
            for fanin in &node.fanins {
                if !positions.contains_key(fanin) {
                    return Err(NetworkDfsError::MissingFanin {
                        node_id: node.id,
                        fanin_id: *fanin,
                    });
                }

                fanouts.entry(*fanin).or_default().push(node.id);
            }
        }

        Ok(Self {
            nodes,
            positions,
            fanouts,
        })
    }

    pub fn nodes(&self) -> &[NetworkNode] {
        &self.nodes
    }

    pub fn node(&self, node_id: NodeId) -> Result<&NetworkNode, NetworkDfsError> {
        self.positions
            .get(&node_id)
            .map(|position| &self.nodes[*position])
            .ok_or(NetworkDfsError::MissingNode { node_id })
    }

    pub fn fanouts(&self, node_id: NodeId) -> Result<&[NodeId], NetworkDfsError> {
        self.node(node_id)?;
        Ok(self
            .fanouts
            .get(&node_id)
            .map(Vec::as_slice)
            .unwrap_or_default())
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = &NetworkNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryInput)
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = &NetworkNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryOutput)
    }

    fn root_outputs(&self) -> Vec<NodeId> {
        let mut roots: Vec<NodeId> = self.primary_outputs().map(|node| node.id).collect();
        self.append_floating_output_roots(&mut roots);
        roots
    }

    fn special_root_outputs(&self) -> Vec<NodeId> {
        let mut roots: Vec<NodeId> = self
            .primary_outputs()
            .filter(|node| node.is_control_output)
            .map(|node| node.id)
            .collect();

        roots.extend(
            self.primary_outputs()
                .filter(|node| !node.is_control_output)
                .map(|node| node.id),
        );
        self.append_floating_output_roots(&mut roots);
        roots
    }

    fn root_inputs(&self) -> Vec<NodeId> {
        let mut roots: Vec<NodeId> = self.primary_inputs().map(|node| node.id).collect();
        roots.extend(
            self.nodes
                .iter()
                .filter(|node| node.fanins.is_empty() && node.kind != NodeKind::PrimaryInput)
                .map(|node| node.id),
        );
        roots
    }

    fn append_floating_output_roots(&self, roots: &mut Vec<NodeId>) {
        roots.extend(
            self.nodes
                .iter()
                .filter(|node| {
                    self.fanouts.get(&node.id).is_none_or(Vec::is_empty)
                        && node.kind != NodeKind::PrimaryOutput
                })
                .map(|node| node.id),
        );
    }
}

pub fn network_dfs(network: &Network) -> Result<Vec<NodeId>, NetworkDfsError> {
    dfs_from_roots(
        network,
        network.root_outputs(),
        Direction::Inputs,
        INFINITY_LEVEL,
    )
}

pub fn network_special_dfs(network: &Network) -> Result<Vec<NodeId>, NetworkDfsError> {
    dfs_from_roots(
        network,
        network.special_root_outputs(),
        Direction::Inputs,
        INFINITY_LEVEL,
    )
}

pub fn network_dfs_from_input(network: &Network) -> Result<Vec<NodeId>, NetworkDfsError> {
    dfs_from_roots(
        network,
        network.root_inputs(),
        Direction::Outputs,
        INFINITY_LEVEL,
    )
}

pub fn network_tfi(
    network: &Network,
    node_id: NodeId,
    level: usize,
) -> Result<Vec<NodeId>, NetworkDfsError> {
    let roots = network.node(node_id)?.fanins.clone();
    dfs_from_roots(network, roots, Direction::Inputs, level)
}

pub fn network_tfo(
    network: &Network,
    node_id: NodeId,
    level: usize,
) -> Result<Vec<NodeId>, NetworkDfsError> {
    let roots = network.fanouts(node_id)?.to_vec();
    dfs_from_roots(network, roots, Direction::Outputs, level)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Inputs,
    Outputs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Active,
    Done,
}

fn dfs_from_roots(
    network: &Network,
    roots: Vec<NodeId>,
    direction: Direction,
    level: usize,
) -> Result<Vec<NodeId>, NetworkDfsError> {
    let mut visited = HashMap::new();
    let mut order = Vec::new();

    for root in roots {
        dfs_recur(network, root, &mut order, &mut visited, direction, level)?;
    }

    Ok(order)
}

fn dfs_recur(
    network: &Network,
    node_id: NodeId,
    order: &mut Vec<NodeId>,
    visited: &mut HashMap<NodeId, VisitState>,
    direction: Direction,
    level: usize,
) -> Result<(), NetworkDfsError> {
    if level == 0 {
        return Ok(());
    }

    if let Some(state) = visited.get(&node_id) {
        return match state {
            VisitState::Active => Err(NetworkDfsError::CycleDetected { node_id }),
            VisitState::Done => Ok(()),
        };
    }

    visited.insert(node_id, VisitState::Active);

    if level > 1 {
        let fanins;
        let fanouts;
        let neighbors = match direction {
            Direction::Inputs => {
                fanins = network.node(node_id)?.fanins.as_slice();
                fanins
            }
            Direction::Outputs => {
                fanouts = network.fanouts(node_id)?;
                fanouts
            }
        };

        for neighbor in neighbors {
            dfs_recur(
                network,
                *neighbor,
                order,
                visited,
                direction,
                level.saturating_sub(1),
            )?;
        }
    } else {
        network.node(node_id)?;
    }

    visited.insert(node_id, VisitState::Done);
    order.push(node_id);
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkDfsError {
    DuplicateNode { node_id: NodeId },
    MissingNode { node_id: NodeId },
    MissingFanin { node_id: NodeId, fanin_id: NodeId },
    CycleDetected { node_id: NodeId },
}

impl fmt::Display for NetworkDfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode { node_id } => {
                write!(f, "duplicate network node {}", node_id.0)
            }
            Self::MissingNode { node_id } => {
                write!(f, "missing network node {}", node_id.0)
            }
            Self::MissingFanin { node_id, fanin_id } => {
                write!(
                    f,
                    "network node {} references missing fanin {}",
                    node_id.0, fanin_id.0
                )
            }
            Self::CycleDetected { node_id } => {
                write!(f, "network contains a cycle through node {}", node_id.0)
            }
        }
    }
}

impl Error for NetworkDfsError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_ids(nodes: &[usize]) -> Vec<NodeId> {
        nodes.iter().copied().map(NodeId).collect()
    }

    fn sample_network() -> Network {
        Network::new(vec![
            NetworkNode::new(1, "a", NodeKind::PrimaryInput),
            NetworkNode::new(2, "b", NodeKind::PrimaryInput),
            NetworkNode::new(3, "n1", NodeKind::Internal).with_fanins(node_ids(&[1, 2])),
            NetworkNode::new(4, "n2", NodeKind::Internal).with_fanins(node_ids(&[3])),
            NetworkNode::new(5, "out", NodeKind::PrimaryOutput).with_fanins(node_ids(&[4])),
        ])
        .unwrap()
    }

    #[test]
    fn dfs_from_outputs_places_fanins_before_each_node() {
        let network = sample_network();

        assert_eq!(network_dfs(&network).unwrap(), node_ids(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn dfs_from_inputs_places_fanouts_before_each_node() {
        let network = sample_network();

        assert_eq!(
            network_dfs_from_input(&network).unwrap(),
            node_ids(&[5, 4, 3, 1, 2])
        );
    }

    #[test]
    fn dfs_includes_floating_nodes_as_roots() {
        let network = Network::new(vec![
            NetworkNode::new(1, "a", NodeKind::PrimaryInput),
            NetworkNode::new(2, "floating", NodeKind::Internal).with_fanins(node_ids(&[1])),
        ])
        .unwrap();

        assert_eq!(network_dfs(&network).unwrap(), node_ids(&[1, 2]));
        assert_eq!(network_dfs_from_input(&network).unwrap(), node_ids(&[2, 1]));
    }

    #[test]
    fn special_dfs_visits_control_outputs_before_other_outputs() {
        let network = Network::new(vec![
            NetworkNode::new(1, "a", NodeKind::PrimaryInput),
            NetworkNode::new(2, "data", NodeKind::PrimaryInput),
            NetworkNode::new(3, "control", NodeKind::PrimaryOutput)
                .with_fanins(node_ids(&[1]))
                .with_control_output(true),
            NetworkNode::new(4, "out", NodeKind::PrimaryOutput).with_fanins(node_ids(&[2])),
        ])
        .unwrap();

        assert_eq!(
            network_special_dfs(&network).unwrap(),
            node_ids(&[1, 3, 2, 4])
        );
    }

    #[test]
    fn transitive_fanin_and_fanout_respect_level_limit() {
        let network = sample_network();

        assert_eq!(network_tfi(&network, NodeId(5), 1).unwrap(), node_ids(&[4]));
        assert_eq!(
            network_tfi(&network, NodeId(5), 2).unwrap(),
            node_ids(&[3, 4])
        );
        assert_eq!(network_tfo(&network, NodeId(1), 1).unwrap(), node_ids(&[3]));
        assert_eq!(
            network_tfo(&network, NodeId(1), 2).unwrap(),
            node_ids(&[4, 3])
        );
        assert_eq!(network_tfo(&network, NodeId(1), 0).unwrap(), Vec::new());
    }

    #[test]
    fn network_construction_validates_duplicate_and_missing_nodes() {
        assert!(matches!(
            Network::new(vec![
                NetworkNode::new(1, "a", NodeKind::PrimaryInput),
                NetworkNode::new(1, "again", NodeKind::Internal),
            ]),
            Err(NetworkDfsError::DuplicateNode { .. })
        ));

        assert!(matches!(
            Network::new(vec![
                NetworkNode::new(1, "n", NodeKind::Internal).with_fanins(node_ids(&[2])),
            ]),
            Err(NetworkDfsError::MissingFanin { .. })
        ));
    }

    #[test]
    fn dfs_reports_cycles() {
        let network = Network::new(vec![
            NetworkNode::new(1, "a", NodeKind::Internal).with_fanins(node_ids(&[2])),
            NetworkNode::new(2, "b", NodeKind::Internal).with_fanins(node_ids(&[1])),
            NetworkNode::new(3, "out", NodeKind::PrimaryOutput).with_fanins(node_ids(&[1])),
        ])
        .unwrap();

        assert!(matches!(
            network_dfs(&network),
            Err(NetworkDfsError::CycleDetected { .. })
        ));
        assert!(
            format!("{}", network_dfs(&network).unwrap_err()).contains("network contains a cycle")
        );
    }
}
