//! Native Rust maximum-flow algorithm for `LogicSynthesis/sis/maxflow/maxflow.c`.
//!
//! This module ports the package-level maxflow behavior over the owned graph
//! model from `mf_input.rs`: initialize edge state, repeatedly augment through
//! residual paths, mark the source-side min-cut, build a cutset report, and
//! optionally verify cutset minimality.

use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::fmt;

use super::mf_input::MfGraph;

pub const MAX_FLOW: i32 = i32::MAX / 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CutsetArc {
    pub from: String,
    pub to: String,
    pub flow: i32,
    pub capacity: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxflowResult {
    pub value: i32,
    pub cutset: Vec<CutsetArc>,
    pub augmentations: usize,
}

impl MaxflowResult {
    pub fn is_empty(&self) -> bool {
        self.cutset.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaxflowError {
    MissingSource,
    MissingSink,
    InvalidEdgeIndex(usize),
    NegativeFlow(usize, i32),
    FlowExceedsCapacity(usize, i32, i32),
    ConservationViolation(String),
    CutsetEdgeNotSaturated(String, String),
    CutsetDoesNotSeparate,
    CutsetNotMinimum(String, String),
}

impl fmt::Display for MaxflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSource => write!(f, "source node unspecified"),
            Self::MissingSink => write!(f, "sink node unspecified"),
            Self::InvalidEdgeIndex(index) => write!(f, "invalid maxflow edge index {index}"),
            Self::NegativeFlow(index, flow) => {
                write!(f, "edge {index} has negative flow {flow}")
            }
            Self::FlowExceedsCapacity(index, flow, capacity) => {
                write!(f, "edge {index} flow {flow} exceeds capacity {capacity}")
            }
            Self::ConservationViolation(name) => {
                write!(f, "node {name} violates conservation of flow")
            }
            Self::CutsetEdgeNotSaturated(from, to) => {
                write!(f, "cutset edge {from}->{to} is not saturated")
            }
            Self::CutsetDoesNotSeparate => {
                write!(f, "cutset does not separate source from sink")
            }
            Self::CutsetNotMinimum(from, to) => {
                write!(f, "cutset is not minimum without edge {from}->{to}")
            }
        }
    }
}

impl Error for MaxflowError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResidualStep {
    edge_index: usize,
    forward: bool,
}

pub fn maxflow(
    graph: &mut MfGraph,
    verify_minimum_cut: bool,
) -> Result<MaxflowResult, MaxflowError> {
    let source = source_index(graph)?;
    let sink = sink_index(graph)?;
    maxflow_init(graph)?;

    let mut augmentations = 0;
    while let Some(path) = find_augmenting_path(graph, source, sink)? {
        let increment = path_increment(graph, &path)?;
        if increment <= 0 {
            break;
        }

        augment(graph, &path, increment)?;
        augmentations += 1;
    }

    let reachable = residual_reachable(graph, source)?;
    construct_cutset(graph, &reachable)?;
    check(graph, source, sink)?;
    if verify_minimum_cut {
        min_cutset_check(graph, source, sink)?;
    }

    Ok(MaxflowResult {
        value: maxflow_value(graph, source)?,
        cutset: get_cutset(graph)?,
        augmentations,
    })
}

pub fn maxflow_init(graph: &mut MfGraph) -> Result<(), MaxflowError> {
    source_index(graph)?;
    sink_index(graph)?;

    for edge_index in active_edge_indices(graph) {
        let edge = graph
            .edge_mut(edge_index)
            .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
        edge.flow = 0;
        edge.on_min_cut = false;
    }

    Ok(())
}

pub fn get_cutset(graph: &MfGraph) -> Result<Vec<CutsetArc>, MaxflowError> {
    let mut arcs = Vec::new();
    for edge_index in active_edge_indices(graph) {
        let edge = graph
            .edge(edge_index)
            .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
        if !edge.on_min_cut {
            continue;
        }

        let from = graph
            .tail_of_edge(edge)
            .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
        let to = graph
            .head_of_edge(edge)
            .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
        arcs.push(CutsetArc {
            from: from.name.clone(),
            to: to.name.clone(),
            flow: edge.flow,
            capacity: edge.capacity,
        });
    }

    Ok(arcs)
}

pub fn cutset_size(graph: &MfGraph) -> usize {
    graph.edges().filter(|edge| edge.on_min_cut).count()
}

pub fn check(graph: &MfGraph, source: usize, sink: usize) -> Result<(), MaxflowError> {
    for edge_index in active_edge_indices(graph) {
        let edge = graph
            .edge(edge_index)
            .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
        if edge.flow < 0 {
            return Err(MaxflowError::NegativeFlow(edge_index, edge.flow));
        }
        if edge.flow > edge.capacity {
            return Err(MaxflowError::FlowExceedsCapacity(
                edge_index,
                edge.flow,
                edge.capacity,
            ));
        }
        if edge.on_min_cut && edge.flow != edge.capacity {
            let tail = graph
                .tail_of_edge(edge)
                .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
            let head = graph
                .head_of_edge(edge)
                .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
            return Err(MaxflowError::CutsetEdgeNotSaturated(
                tail.name.clone(),
                head.name.clone(),
            ));
        }
    }

    for (node_index, node) in graph.nodes().iter().enumerate() {
        if node_index == source || node_index == sink {
            continue;
        }

        let inflow = node.fanin_edges.iter().try_fold(0, |sum, edge_index| {
            edge_flow(graph, *edge_index).map(|flow| sum + flow)
        })?;
        let outflow = node.fanout_edges.iter().try_fold(0, |sum, edge_index| {
            edge_flow(graph, *edge_index).map(|flow| sum + flow)
        })?;
        if inflow != outflow {
            return Err(MaxflowError::ConservationViolation(node.name.clone()));
        }
    }

    if trace_path_without_cutset(graph, source, sink, None)? {
        return Err(MaxflowError::CutsetDoesNotSeparate);
    }

    Ok(())
}

pub fn min_cutset_check(graph: &MfGraph, source: usize, sink: usize) -> Result<(), MaxflowError> {
    for edge_index in active_edge_indices(graph) {
        let Some(edge) = graph.edge(edge_index) else {
            return Err(MaxflowError::InvalidEdgeIndex(edge_index));
        };
        if !edge.on_min_cut {
            continue;
        }

        if !trace_path_without_cutset(graph, source, sink, Some(edge_index))? {
            let tail = graph
                .tail_of_edge(edge)
                .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
            let head = graph
                .head_of_edge(edge)
                .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
            return Err(MaxflowError::CutsetNotMinimum(
                tail.name.clone(),
                head.name.clone(),
            ));
        }
    }

    Ok(())
}

fn find_augmenting_path(
    graph: &MfGraph,
    source: usize,
    sink: usize,
) -> Result<Option<Vec<ResidualStep>>, MaxflowError> {
    let mut parent: Vec<Option<(usize, ResidualStep)>> = vec![None; graph.num_nodes()];
    let mut visited = vec![false; graph.num_nodes()];
    let mut queue = VecDeque::from([source]);
    visited[source] = true;

    while let Some(node_index) = queue.pop_front() {
        if node_index == sink {
            break;
        }

        let node = &graph.nodes()[node_index];
        for edge_index in &node.fanout_edges {
            let edge = graph
                .edge(*edge_index)
                .ok_or(MaxflowError::InvalidEdgeIndex(*edge_index))?;
            if edge.flow < edge.capacity && !visited[edge.head] {
                visited[edge.head] = true;
                parent[edge.head] = Some((
                    node_index,
                    ResidualStep {
                        edge_index: *edge_index,
                        forward: true,
                    },
                ));
                queue.push_back(edge.head);
            }
        }

        for edge_index in &node.fanin_edges {
            let edge = graph
                .edge(*edge_index)
                .ok_or(MaxflowError::InvalidEdgeIndex(*edge_index))?;
            if edge.flow > 0 && !visited[edge.tail] {
                visited[edge.tail] = true;
                parent[edge.tail] = Some((
                    node_index,
                    ResidualStep {
                        edge_index: *edge_index,
                        forward: false,
                    },
                ));
                queue.push_back(edge.tail);
            }
        }
    }

    if !visited[sink] {
        return Ok(None);
    }

    let mut path = Vec::new();
    let mut node = sink;
    while node != source {
        let (previous, step) = parent[node].ok_or(MaxflowError::InvalidEdgeIndex(usize::MAX))?;
        path.push(step);
        node = previous;
    }
    path.reverse();
    Ok(Some(path))
}

fn path_increment(graph: &MfGraph, path: &[ResidualStep]) -> Result<i32, MaxflowError> {
    path.iter()
        .map(|step| {
            let edge = graph
                .edge(step.edge_index)
                .ok_or(MaxflowError::InvalidEdgeIndex(step.edge_index))?;
            if step.forward {
                Ok(edge.capacity - edge.flow)
            } else {
                Ok(edge.flow)
            }
        })
        .try_fold(MAX_FLOW, |minimum, residual| {
            residual.map(|residual| minimum.min(residual))
        })
}

fn augment(graph: &mut MfGraph, path: &[ResidualStep], increment: i32) -> Result<(), MaxflowError> {
    for step in path {
        let edge = graph
            .edge_mut(step.edge_index)
            .ok_or(MaxflowError::InvalidEdgeIndex(step.edge_index))?;
        if step.forward {
            edge.flow += increment;
        } else {
            edge.flow -= increment;
        }
    }

    Ok(())
}

fn residual_reachable(graph: &MfGraph, source: usize) -> Result<HashSet<usize>, MaxflowError> {
    let mut reachable = HashSet::from([source]);
    let mut queue = VecDeque::from([source]);

    while let Some(node_index) = queue.pop_front() {
        let node = &graph.nodes()[node_index];
        for edge_index in &node.fanout_edges {
            let edge = graph
                .edge(*edge_index)
                .ok_or(MaxflowError::InvalidEdgeIndex(*edge_index))?;
            if edge.flow < edge.capacity && reachable.insert(edge.head) {
                queue.push_back(edge.head);
            }
        }

        for edge_index in &node.fanin_edges {
            let edge = graph
                .edge(*edge_index)
                .ok_or(MaxflowError::InvalidEdgeIndex(*edge_index))?;
            if edge.flow > 0 && reachable.insert(edge.tail) {
                queue.push_back(edge.tail);
            }
        }
    }

    Ok(reachable)
}

fn construct_cutset(graph: &mut MfGraph, reachable: &HashSet<usize>) -> Result<(), MaxflowError> {
    for edge_index in active_edge_indices(graph) {
        let edge = graph
            .edge_mut(edge_index)
            .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))?;
        edge.on_min_cut = reachable.contains(&edge.tail) && !reachable.contains(&edge.head);
    }

    Ok(())
}

fn trace_path_without_cutset(
    graph: &MfGraph,
    source: usize,
    sink: usize,
    ignored_cutset_edge: Option<usize>,
) -> Result<bool, MaxflowError> {
    let mut visited = HashSet::from([source]);
    let mut queue = VecDeque::from([source]);

    while let Some(node_index) = queue.pop_front() {
        if node_index == sink {
            return Ok(true);
        }

        for edge_index in &graph.nodes()[node_index].fanout_edges {
            let edge = graph
                .edge(*edge_index)
                .ok_or(MaxflowError::InvalidEdgeIndex(*edge_index))?;
            if edge.on_min_cut && Some(*edge_index) != ignored_cutset_edge {
                continue;
            }
            if visited.insert(edge.head) {
                queue.push_back(edge.head);
            }
        }
    }

    Ok(false)
}

fn maxflow_value(graph: &MfGraph, source: usize) -> Result<i32, MaxflowError> {
    graph.nodes()[source]
        .fanout_edges
        .iter()
        .try_fold(0, |sum, edge_index| {
            edge_flow(graph, *edge_index).map(|flow| sum + flow)
        })
}

fn edge_flow(graph: &MfGraph, edge_index: usize) -> Result<i32, MaxflowError> {
    graph
        .edge(edge_index)
        .map(|edge| edge.flow)
        .ok_or(MaxflowError::InvalidEdgeIndex(edge_index))
}

fn active_edge_indices(graph: &MfGraph) -> Vec<usize> {
    graph
        .nodes()
        .iter()
        .flat_map(|node| node.fanout_edges.iter().copied())
        .collect()
}

fn source_index(graph: &MfGraph) -> Result<usize, MaxflowError> {
    let source_name = graph
        .source_node()
        .ok_or(MaxflowError::MissingSource)?
        .name
        .clone();
    graph
        .node_index(&source_name)
        .ok_or(MaxflowError::MissingSource)
}

fn sink_index(graph: &MfGraph) -> Result<usize, MaxflowError> {
    let sink_name = graph
        .sink_node()
        .ok_or(MaxflowError::MissingSink)?
        .name
        .clone();
    graph
        .node_index(&sink_name)
        .ok_or(MaxflowError::MissingSink)
}

#[cfg(test)]
mod tests {
    use super::super::mf_input::MfNodeKind;
    use super::*;

    #[test]
    fn computes_maxflow_and_cutset() {
        let mut graph = sample_graph();
        let result = maxflow(&mut graph, true).unwrap();

        assert_eq!(result.value, 5);
        assert_eq!(result.augmentations, 2);
        assert_eq!(cutset_size(&graph), 2);
        assert_eq!(
            result
                .cutset
                .iter()
                .map(|arc| (arc.from.as_str(), arc.to.as_str(), arc.flow))
                .collect::<Vec<_>>(),
            vec![("s", "a", 3), ("s", "b", 2)]
        );
    }

    #[test]
    fn resets_previous_flow_before_running() {
        let mut graph = sample_graph();
        let result = maxflow(&mut graph, false).unwrap();
        assert_eq!(result.value, 5);

        for edge_index in active_edge_indices(&graph) {
            graph.edge_mut(edge_index).unwrap().flow = 0;
        }

        let second = maxflow(&mut graph, false).unwrap();
        assert_eq!(second.value, 5);
        assert!(graph.edges().any(|edge| edge.on_min_cut));
    }

    #[test]
    fn uses_reverse_residual_edges_when_needed() {
        let mut graph = MfGraph::new();
        graph.read_node("s", MfNodeKind::Source).unwrap();
        graph.read_node("t", MfNodeKind::Sink).unwrap();
        graph.read_node("a", MfNodeKind::Internal).unwrap();
        graph.read_node("b", MfNodeKind::Internal).unwrap();
        graph.read_edge("s", "a", 1).unwrap();
        graph.read_edge("s", "b", 1).unwrap();
        graph.read_edge("a", "b", 1).unwrap();
        graph.read_edge("a", "t", 1).unwrap();
        graph.read_edge("b", "t", 1).unwrap();

        let result = maxflow(&mut graph, true).unwrap();

        assert_eq!(result.value, 2);
        assert_eq!(
            graph
                .edges()
                .filter(|edge| edge.tail == graph.node_index("s").unwrap())
                .map(|edge| edge.flow)
                .sum::<i32>(),
            2
        );
    }

    #[test]
    fn handles_cycles_without_source_rewriting() {
        let mut graph = sample_graph();
        graph.read_edge("b", "a", 1).unwrap();
        graph.read_edge("a", "c", 1).unwrap();
        graph.read_edge("c", "b", 1).unwrap();

        let result = maxflow(&mut graph, true).unwrap();

        assert_eq!(result.value, 5);
        assert_eq!(
            check(
                &graph,
                source_index(&graph).unwrap(),
                sink_index(&graph).unwrap()
            ),
            Ok(())
        );
    }

    #[test]
    fn reports_missing_source_and_sink() {
        let mut graph = MfGraph::new();
        graph.read_node("s", MfNodeKind::Internal).unwrap();

        assert_eq!(maxflow(&mut graph, false), Err(MaxflowError::MissingSource));

        graph.change_node_type("s", MfNodeKind::Source).unwrap();
        assert_eq!(maxflow(&mut graph, false), Err(MaxflowError::MissingSink));
    }

    #[test]
    fn check_rejects_invalid_flow_state() {
        let mut graph = sample_graph();
        let edge_index = graph.read_edge("a", "b", 1).unwrap();
        graph.edge_mut(edge_index).unwrap().flow = 2;

        assert_eq!(
            check(
                &graph,
                source_index(&graph).unwrap(),
                sink_index(&graph).unwrap()
            ),
            Err(MaxflowError::FlowExceedsCapacity(edge_index, 2, 1))
        );
    }

    #[test]
    fn minimum_cut_verification_rejects_redundant_cut_edges() {
        let mut graph = sample_graph();
        maxflow(&mut graph, false).unwrap();
        let edge_index = graph.read_edge("a", "t", 0).unwrap();
        graph.edge_mut(edge_index).unwrap().on_min_cut = true;

        assert_eq!(
            min_cutset_check(
                &graph,
                source_index(&graph).unwrap(),
                sink_index(&graph).unwrap()
            ),
            Err(MaxflowError::CutsetNotMinimum(
                "a".to_owned(),
                "t".to_owned()
            ))
        );
    }

    fn sample_graph() -> MfGraph {
        let mut graph = MfGraph::new();
        graph.read_node("s", MfNodeKind::Source).unwrap();
        graph.read_node("t", MfNodeKind::Sink).unwrap();
        graph.read_node("a", MfNodeKind::Internal).unwrap();
        graph.read_node("b", MfNodeKind::Internal).unwrap();
        graph.read_node("c", MfNodeKind::Internal).unwrap();
        graph.read_edge("s", "a", 3).unwrap();
        graph.read_edge("s", "b", 2).unwrap();
        graph.read_edge("a", "b", 1).unwrap();
        graph.read_edge("a", "t", 3).unwrap();
        graph.read_edge("b", "c", 2).unwrap();
        graph.read_edge("c", "t", 2).unwrap();
        graph
    }
}
