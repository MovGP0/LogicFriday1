//! Native cutset construction over the maxflow graph implementation.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use super::com_max::{
    MaxflowCommandError, MaxflowGraph, MaxflowNodeKind, MaxflowReport, run_maxflow,
};

pub const DEFAULT_CUT_EDGE_WEIGHT: i32 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CutsetNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CutsetNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CutsetNode {
    pub id: CutsetNodeId,
    pub name: String,
    pub kind: CutsetNodeKind,
    pub fanins: Vec<CutsetNodeId>,
    pub fanouts: Vec<CutsetNodeId>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CutsetNetwork {
    nodes: Vec<CutsetNode>,
    index: HashMap<CutsetNodeId, usize>,
}

impl CutsetNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(
        &mut self,
        id: CutsetNodeId,
        name: impl Into<String>,
        kind: CutsetNodeKind,
    ) -> Result<(), CutsetError> {
        if self.index.contains_key(&id) {
            return Err(CutsetError::DuplicateNode(id));
        }

        self.index.insert(id, self.nodes.len());
        self.nodes.push(CutsetNode {
            id,
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
        });
        Ok(())
    }

    pub fn add_edge(&mut self, from: CutsetNodeId, to: CutsetNodeId) -> Result<(), CutsetError> {
        let from_index = self.index_of(from)?;
        let to_index = self.index_of(to)?;

        if !self.nodes[from_index].fanouts.contains(&to) {
            self.nodes[from_index].fanouts.push(to);
        }
        if !self.nodes[to_index].fanins.contains(&from) {
            self.nodes[to_index].fanins.push(from);
        }

        Ok(())
    }

    pub fn nodes(&self) -> &[CutsetNode] {
        &self.nodes
    }

    pub fn get(&self, id: CutsetNodeId) -> Option<&CutsetNode> {
        self.index.get(&id).map(|index| &self.nodes[*index])
    }

    fn index_of(&self, id: CutsetNodeId) -> Result<usize, CutsetError> {
        self.index
            .get(&id)
            .copied()
            .ok_or(CutsetError::UnknownNode(id))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowNetwork {
    pub graph: MaxflowGraph,
    pub name_to_node: HashMap<String, CutsetNodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CutsetResult {
    pub nodes: Vec<CutsetNodeId>,
    pub report: MaxflowReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CutsetError {
    DuplicateNode(CutsetNodeId),
    UnknownNode(CutsetNodeId),
    MissingCapacity(CutsetNodeId),
    NegativeCapacity(CutsetNodeId, i32),
    InvalidFlowGraph(MaxflowCommandError),
}

impl fmt::Display for CutsetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode(id) => write!(f, "duplicate cutset node {}", id.0),
            Self::UnknownNode(id) => write!(f, "unknown cutset node {}", id.0),
            Self::MissingCapacity(id) => write!(f, "missing cutset capacity for node {}", id.0),
            Self::NegativeCapacity(id, capacity) => {
                write!(f, "negative cutset capacity {capacity} for node {}", id.0)
            }
            Self::InvalidFlowGraph(error) => write!(f, "{error}"),
        }
    }
}

impl Error for CutsetError {}

impl From<MaxflowCommandError> for CutsetError {
    fn from(value: MaxflowCommandError) -> Self {
        Self::InvalidFlowGraph(value)
    }
}

pub fn cutset(
    network: &CutsetNetwork,
    capacities: &HashMap<CutsetNodeId, i32>,
) -> Result<CutsetResult, CutsetError> {
    cutset_with_edge_weight(network, capacities, DEFAULT_CUT_EDGE_WEIGHT)
}

pub fn cutset_with_edge_weight(
    network: &CutsetNetwork,
    capacities: &HashMap<CutsetNodeId, i32>,
    edge_weight: i32,
) -> Result<CutsetResult, CutsetError> {
    if edge_weight < 0 {
        return Err(CutsetError::InvalidFlowGraph(
            MaxflowCommandError::NegativeCapacity(edge_weight),
        ));
    }

    let FlowNetwork {
        mut graph,
        name_to_node,
    } = create_flow_network(network, capacities, edge_weight)?;
    let report = run_maxflow(&mut graph)?;
    let nodes = build_node_cutset(&report, &name_to_node);

    Ok(CutsetResult { nodes, report })
}

pub fn create_flow_network(
    network: &CutsetNetwork,
    capacities: &HashMap<CutsetNodeId, i32>,
    edge_weight: i32,
) -> Result<FlowNetwork, CutsetError> {
    if edge_weight < 0 {
        return Err(CutsetError::InvalidFlowGraph(
            MaxflowCommandError::NegativeCapacity(edge_weight),
        ));
    }

    let selected = validate_capacities(network, capacities)?;
    let mut graph = MaxflowGraph::new();
    let mut name_to_node = HashMap::new();

    graph.read_node("maxflow_source", MaxflowNodeKind::Source);
    graph.read_node("maxflow_sink", MaxflowNodeKind::Sink);

    for node in network.nodes() {
        if selected.contains(&node.id) {
            graph.read_node(node.name.clone(), MaxflowNodeKind::Internal);
            graph.read_node(duplicate_name(&node.name), MaxflowNodeKind::Internal);
        }
    }

    for node in network.nodes() {
        if node.kind != CutsetNodeKind::Internal || !selected.contains(&node.id) {
            continue;
        }

        let capacity = capacities[&node.id];
        let node_name = node.name.clone();
        let duplicate_node_name = duplicate_name(&node.name);

        graph.read_edge(node_name.clone(), duplicate_node_name.clone(), capacity)?;
        name_to_node.insert(node_name.clone(), node.id);

        let mut has_selected_fanin = false;
        for fanin_id in &node.fanins {
            if selected.contains(fanin_id) {
                let fanin = network
                    .get(*fanin_id)
                    .ok_or(CutsetError::UnknownNode(*fanin_id))?;
                graph.read_edge(duplicate_name(&fanin.name), node_name.clone(), edge_weight)?;
                has_selected_fanin = true;
            }
        }
        if !has_selected_fanin {
            graph.read_edge("maxflow_source", node_name.clone(), edge_weight)?;
        }

        let mut has_selected_fanout = false;
        for fanout_id in &node.fanouts {
            if selected.contains(fanout_id) {
                has_selected_fanout = true;
                break;
            }
        }
        if !has_selected_fanout {
            graph.read_edge(duplicate_node_name, "maxflow_sink", edge_weight)?;
        }
    }

    Ok(FlowNetwork {
        graph,
        name_to_node,
    })
}

pub fn build_node_cutset(
    report: &MaxflowReport,
    name_to_node: &HashMap<String, CutsetNodeId>,
) -> Vec<CutsetNodeId> {
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();

    for arc in &report.cutset {
        if let Some(node) = name_to_node.get(&arc.from) {
            if seen.insert(*node) {
                nodes.push(*node);
            }
        }
    }

    nodes
}

fn validate_capacities(
    network: &CutsetNetwork,
    capacities: &HashMap<CutsetNodeId, i32>,
) -> Result<HashSet<CutsetNodeId>, CutsetError> {
    let mut selected = HashSet::new();

    for (id, capacity) in capacities {
        let node = network.get(*id).ok_or(CutsetError::UnknownNode(*id))?;
        if *capacity < 0 {
            return Err(CutsetError::NegativeCapacity(*id, *capacity));
        }
        if node.kind == CutsetNodeKind::Internal {
            selected.insert(*id);
        }
    }

    for node in network.nodes() {
        if node.kind == CutsetNodeKind::Internal && !capacities.contains_key(&node.id) {
            return Err(CutsetError::MissingCapacity(node.id));
        }
    }

    Ok(selected)
}

fn duplicate_name(name: &str) -> String {
    format!("{name}_dup")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_split_flow_network_for_selected_internal_nodes() {
        let network = sample_network().unwrap();
        let capacities = HashMap::from([
            (CutsetNodeId(1), 4),
            (CutsetNodeId(2), 2),
            (CutsetNodeId(3), 3),
        ]);

        let flow = create_flow_network(&network, &capacities, 9).unwrap();
        let edges = flow.graph.edges();

        assert!(flow.name_to_node.contains_key("a"));
        assert!(
            edges
                .iter()
                .any(|edge| edge.from == "a" && edge.to == "a_dup" && edge.capacity == 4)
        );
        assert!(
            edges
                .iter()
                .any(|edge| edge.from == "a_dup" && edge.to == "b" && edge.capacity == 9)
        );
        assert!(
            edges
                .iter()
                .any(|edge| edge.from == "maxflow_source" && edge.to == "a" && edge.capacity == 9)
        );
        assert!(
            edges.iter().any(|edge| edge.from == "c_dup"
                && edge.to == "maxflow_sink"
                && edge.capacity == 9)
        );
    }

    #[test]
    fn computes_cutset_nodes_from_flow_cut_edges() {
        let network = sample_network().unwrap();
        let capacities = HashMap::from([
            (CutsetNodeId(1), 4),
            (CutsetNodeId(2), 2),
            (CutsetNodeId(3), 3),
        ]);

        let result = cutset_with_edge_weight(&network, &capacities, 9).unwrap();

        assert_eq!(result.report.value, 2);
        assert_eq!(result.nodes, vec![CutsetNodeId(2)]);
    }

    #[test]
    fn ignores_primary_io_nodes_in_capacity_table() {
        let network = sample_network().unwrap();
        let capacities = HashMap::from([
            (CutsetNodeId(0), 99),
            (CutsetNodeId(1), 1),
            (CutsetNodeId(2), 5),
            (CutsetNodeId(3), 5),
            (CutsetNodeId(4), 99),
        ]);

        let result = cutset_with_edge_weight(&network, &capacities, 9).unwrap();

        assert_eq!(result.nodes, vec![CutsetNodeId(1)]);
    }

    #[test]
    fn rejects_missing_and_invalid_capacities() {
        let network = sample_network().unwrap();

        assert_eq!(
            cutset(&network, &HashMap::from([(CutsetNodeId(1), 1)])),
            Err(CutsetError::MissingCapacity(CutsetNodeId(2)))
        );
        assert_eq!(
            cutset(
                &network,
                &HashMap::from([
                    (CutsetNodeId(1), 1),
                    (CutsetNodeId(2), -1),
                    (CutsetNodeId(3), 1),
                ]),
            ),
            Err(CutsetError::NegativeCapacity(CutsetNodeId(2), -1))
        );
    }

    #[test]
    fn rejects_duplicate_and_unknown_network_nodes() {
        let mut network = CutsetNetwork::new();
        network
            .add_node(CutsetNodeId(1), "a", CutsetNodeKind::Internal)
            .unwrap();

        assert_eq!(
            network.add_node(CutsetNodeId(1), "again", CutsetNodeKind::Internal),
            Err(CutsetError::DuplicateNode(CutsetNodeId(1)))
        );
        assert_eq!(
            network.add_edge(CutsetNodeId(1), CutsetNodeId(2)),
            Err(CutsetError::UnknownNode(CutsetNodeId(2)))
        );
    }

    fn sample_network() -> Result<CutsetNetwork, CutsetError> {
        let mut network = CutsetNetwork::new();
        network.add_node(CutsetNodeId(0), "pi", CutsetNodeKind::PrimaryInput)?;
        network.add_node(CutsetNodeId(1), "a", CutsetNodeKind::Internal)?;
        network.add_node(CutsetNodeId(2), "b", CutsetNodeKind::Internal)?;
        network.add_node(CutsetNodeId(3), "c", CutsetNodeKind::Internal)?;
        network.add_node(CutsetNodeId(4), "po", CutsetNodeKind::PrimaryOutput)?;
        network.add_edge(CutsetNodeId(0), CutsetNodeId(1))?;
        network.add_edge(CutsetNodeId(1), CutsetNodeId(2))?;
        network.add_edge(CutsetNodeId(2), CutsetNodeId(3))?;
        network.add_edge(CutsetNodeId(3), CutsetNodeId(4))?;
        Ok(network)
    }
}
