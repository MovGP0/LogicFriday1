//! Native command-test harness for the SIS graph package.
//!
//! The C module registers three private `_graph_*` commands that exercise the
//! generic graph, static-slot graph, and DFS helpers. This Rust port keeps those
//! diagnostics available as structured command registrations and deterministic
//! report builders without adding legacy C ABI entry points.

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::fmt;

use super::graph::{EdgeId, Graph, GraphError, VertexId};

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "March", "april", "may", "June", "july", "Aug", "Sept", "Oct", "nov", "Dec",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub changes_network: bool,
    pub command: GraphCommand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphCommand {
    GraphTest,
    GraphStaticTest,
    GraphDfsTest,
}

pub const GRAPH_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "_graph_test",
        changes_network: false,
        command: GraphCommand::GraphTest,
    },
    CommandRegistration {
        name: "_graph_static_test",
        changes_network: false,
        command: GraphCommand::GraphStaticTest,
    },
    CommandRegistration {
        name: "_graph_dfs_test",
        changes_network: false,
        command: GraphCommand::GraphDfsTest,
    },
];

pub fn graph_command_registrations() -> &'static [CommandRegistration] {
    GRAPH_COMMANDS
}

pub fn init_graph() -> &'static [CommandRegistration] {
    GRAPH_COMMANDS
}

pub fn end_graph() {}

pub fn execute_graph_command(
    command: GraphCommand,
) -> Result<GraphCommandReport, GraphCommandError> {
    match command {
        GraphCommand::GraphTest => graph_test().map(GraphCommandReport::GraphTest),
        GraphCommand::GraphStaticTest => {
            graph_static_test().map(GraphCommandReport::GraphStaticTest)
        }
        GraphCommand::GraphDfsTest => graph_dfs_test().map(GraphCommandReport::GraphDfsTest),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphCommandReport {
    GraphTest(GraphTestReport),
    GraphStaticTest(GraphStaticTestReport),
    GraphDfsTest(GraphDfsTestReport),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphTestReport {
    pub duplicate_adjacency: Vec<AdjacencyReport<i32>>,
    pub copied_months: Vec<String>,
    pub source_warning_count: usize,
    pub duplicate_warning_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdjacencyReport<T> {
    pub vertex: T,
    pub goes_to: Vec<T>,
    pub comes_from: Vec<T>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphStaticTestReport {
    pub lines: Vec<StaticEdgeLine>,
    pub copied_vertex_slots: Vec<Option<StaticValue>>,
    pub graph_slot: Option<StaticValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticEdgeLine {
    pub index: i32,
    pub label: String,
    pub source: i32,
    pub destination: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphDfsTestReport {
    pub depth_first_sort: Vec<i32>,
    pub reverse_sort: Vec<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaticValue {
    Int(i32),
    Text(String),
    Char(char),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphCommandError {
    Graph(GraphError),
    CycleDetected,
    MissingSlot { item: &'static str, slot: usize },
}

impl fmt::Display for GraphCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(error) => write!(f, "{error}"),
            Self::CycleDetected => write!(f, "graph DFS command test graph contains a cycle"),
            Self::MissingSlot { item, slot } => {
                write!(f, "{item} is missing static slot {slot}")
            }
        }
    }
}

impl Error for GraphCommandError {}

impl From<GraphError> for GraphCommandError {
    fn from(value: GraphError) -> Self {
        Self::Graph(value)
    }
}

pub fn graph_test() -> Result<GraphTestReport, GraphCommandError> {
    let mut graph = Graph::<(), i32, ()>::new(());
    let mut vertices = Vec::new();

    for value in 0..10 {
        vertices.push(graph.add_vertex(value));
    }

    for source in 0..9 {
        for destination in (source + 1)..10 {
            graph.add_edge(vertices[source], vertices[destination], ())?;
        }
    }

    graph.add_edge(vertices[5], vertices[5], ())?;
    let first_out_edge = *graph
        .vertex(vertices[4])?
        .outgoing_edges()
        .first()
        .ok_or(GraphCommandError::Graph(GraphError::ListItemMissing))?;
    graph.delete_edge(first_out_edge)?;
    graph.delete_vertex(vertices[8])?;
    graph.add_vertex(10);

    let duplicate = graph.duplicate_cloned()?;
    let duplicate_adjacency = adjacency_report(&duplicate)?;
    let source_warning_count = graph.check()?.warnings.len();
    let duplicate_warning_count = duplicate.check()?.warnings.len();

    let mut month_graph = Graph::<(), &'static str, ()>::new(());
    for month in MONTHS {
        month_graph.add_vertex(month);
    }

    let copied_months = month_graph
        .duplicate(|data| *data, |month| (*month).to_string(), |edge| *edge)?
        .vertices()
        .map(|vertex| vertex.data.clone())
        .collect();

    Ok(GraphTestReport {
        duplicate_adjacency,
        copied_months,
        source_warning_count,
        duplicate_warning_count,
    })
}

pub fn graph_static_test() -> Result<GraphStaticTestReport, GraphCommandError> {
    let mut graph = StaticGraph::new(3, 2, 4);
    let mut vertices = Vec::new();

    for index in 0..10 {
        let vertex = graph.add_vertex();
        graph.set_vertex_slot(vertex, 0, StaticValue::Int(index))?;
        graph.set_vertex_slot(vertex, 1, StaticValue::Int(2 * index))?;
        vertices.push(vertex);
    }

    let mut edge_index = 0;
    for source in 0..9 {
        for destination in (source + 1)..10 {
            let edge = graph.add_edge(vertices[source], vertices[destination])?;
            graph.set_edge_slot(edge, 2, StaticValue::Text(MONTHS[source].to_string()))?;
            graph.set_edge_slot(edge, 1, StaticValue::Int(edge_index))?;
            edge_index += 1;
        }
    }

    graph.delete_vertex(vertices[3])?;
    let last_out_edge = graph
        .outgoing_edges(vertices[6])?
        .last()
        .copied()
        .ok_or(GraphCommandError::Graph(GraphError::ListItemMissing))?;
    graph.delete_edge(last_out_edge)?;
    graph.set_graph_slot(1, StaticValue::Char('f'))?;

    let mut duplicate = graph.duplicate()?;
    let copied_vertex = duplicate.add_vertex();
    duplicate.copy_vertex_slots(vertices[2], copied_vertex, &graph)?;

    Ok(GraphStaticTestReport {
        lines: duplicate.edge_lines()?,
        copied_vertex_slots: duplicate.vertex_slots(copied_vertex)?.to_vec(),
        graph_slot: duplicate.graph_slot(1).cloned(),
    })
}

pub fn graph_dfs_test() -> Result<GraphDfsTestReport, GraphCommandError> {
    let graph = build_dfs_test_graph()?;
    let depth_first_sort = depth_first_sort(&graph)?
        .into_iter()
        .map(|id| graph.vertex(id).map(|vertex| vertex.data))
        .collect::<Result<Vec<_>, _>>()?;
    let reverse_sort = graph
        .sorted_vertices(|left, right| right.data.cmp(&left.data))
        .into_iter()
        .map(|id| graph.vertex(id).map(|vertex| vertex.data))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(GraphDfsTestReport {
        depth_first_sort,
        reverse_sort,
    })
}

pub fn depth_first_sort<G, V, E>(
    graph: &Graph<G, V, E>,
) -> Result<Vec<VertexId>, GraphCommandError> {
    let mut permanent = HashSet::new();
    let mut temporary = HashSet::new();
    let mut sorted = Vec::new();

    for vertex in graph.vertices() {
        visit_vertex(
            graph,
            vertex.id,
            &mut permanent,
            &mut temporary,
            &mut sorted,
        )?;
    }

    Ok(sorted)
}

fn visit_vertex<G, V, E>(
    graph: &Graph<G, V, E>,
    vertex: VertexId,
    permanent: &mut HashSet<VertexId>,
    temporary: &mut HashSet<VertexId>,
    sorted: &mut Vec<VertexId>,
) -> Result<(), GraphCommandError> {
    if permanent.contains(&vertex) {
        return Ok(());
    }

    if !temporary.insert(vertex) {
        return Err(GraphCommandError::CycleDetected);
    }

    for edge_id in graph.vertex(vertex)?.incoming_edges() {
        let edge = graph.edge(*edge_id)?;
        visit_vertex(graph, edge.source, permanent, temporary, sorted)?;
    }

    temporary.remove(&vertex);
    permanent.insert(vertex);
    sorted.push(vertex);
    Ok(())
}

fn adjacency_report(
    graph: &Graph<(), i32, ()>,
) -> Result<Vec<AdjacencyReport<i32>>, GraphCommandError> {
    graph
        .vertices()
        .map(|vertex| {
            let goes_to = vertex
                .outgoing_edges()
                .iter()
                .map(|edge_id| {
                    graph
                        .edge(*edge_id)
                        .and_then(|edge| graph.vertex(edge.destination))
                })
                .map(|result| result.map(|vertex| vertex.data))
                .collect::<Result<Vec<_>, _>>()?;
            let comes_from = vertex
                .incoming_edges()
                .iter()
                .map(|edge_id| {
                    graph
                        .edge(*edge_id)
                        .and_then(|edge| graph.vertex(edge.source))
                })
                .map(|result| result.map(|vertex| vertex.data))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(AdjacencyReport {
                vertex: vertex.data,
                goes_to,
                comes_from,
            })
        })
        .collect()
}

fn build_dfs_test_graph() -> Result<Graph<(), i32, ()>, GraphCommandError> {
    let mut graph = Graph::<(), i32, ()>::new(());
    let mut vertices = Vec::new();

    for value in 0..10 {
        vertices.push(graph.add_vertex(value));
    }

    for (source, destination) in [
        (3, 4),
        (0, 3),
        (0, 6),
        (0, 2),
        (1, 3),
        (6, 3),
        (2, 5),
        (2, 3),
        (3, 5),
        (6, 2),
        (7, 8),
        (9, 7),
        (9, 8),
    ] {
        graph.add_edge(vertices[source], vertices[destination], ())?;
    }

    Ok(graph)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StaticGraph {
    graph: Graph<Vec<Option<StaticValue>>, Vec<Option<StaticValue>>, Vec<Option<StaticValue>>>,
    vertex_slots: usize,
    edge_slots: usize,
}

impl StaticGraph {
    fn new(graph_slots: usize, vertex_slots: usize, edge_slots: usize) -> Self {
        Self {
            graph: Graph::new(vec![None; graph_slots]),
            vertex_slots,
            edge_slots,
        }
    }

    fn add_vertex(&mut self) -> VertexId {
        self.graph.add_vertex(vec![None; self.vertex_slots])
    }

    fn add_edge(
        &mut self,
        source: VertexId,
        destination: VertexId,
    ) -> Result<EdgeId, GraphCommandError> {
        Ok(self
            .graph
            .add_edge(source, destination, vec![None; self.edge_slots])?)
    }

    fn set_graph_slot(&mut self, slot: usize, value: StaticValue) -> Result<(), GraphCommandError> {
        set_slot(&mut self.graph.data, slot, value, "graph")
    }

    fn graph_slot(&self, slot: usize) -> Option<&StaticValue> {
        self.graph.data.get(slot).and_then(Option::as_ref)
    }

    fn set_vertex_slot(
        &mut self,
        vertex: VertexId,
        slot: usize,
        value: StaticValue,
    ) -> Result<(), GraphCommandError> {
        let slots = &mut self.graph.vertex_mut(vertex)?.data;
        set_slot(slots, slot, value, "vertex")
    }

    fn vertex_slots(&self, vertex: VertexId) -> Result<&[Option<StaticValue>], GraphCommandError> {
        Ok(&self.graph.vertex(vertex)?.data)
    }

    fn copy_vertex_slots(
        &mut self,
        source: VertexId,
        destination: VertexId,
        source_graph: &Self,
    ) -> Result<(), GraphCommandError> {
        let source_slots = source_graph.graph.vertex(source)?.data.clone();
        self.graph.vertex_mut(destination)?.data = source_slots;
        Ok(())
    }

    fn set_edge_slot(
        &mut self,
        edge: EdgeId,
        slot: usize,
        value: StaticValue,
    ) -> Result<(), GraphCommandError> {
        let slots = &mut self.graph.edge_mut(edge)?.data;
        set_slot(slots, slot, value, "edge")
    }

    fn outgoing_edges(&self, vertex: VertexId) -> Result<&[EdgeId], GraphCommandError> {
        Ok(self.graph.vertex(vertex)?.outgoing_edges())
    }

    fn delete_edge(&mut self, edge: EdgeId) -> Result<(), GraphCommandError> {
        self.graph.delete_edge(edge)?;
        Ok(())
    }

    fn delete_vertex(&mut self, vertex: VertexId) -> Result<(), GraphCommandError> {
        self.graph.delete_vertex(vertex)?;
        Ok(())
    }

    fn duplicate(&self) -> Result<Self, GraphCommandError> {
        Ok(Self {
            graph: self.graph.duplicate_cloned()?,
            vertex_slots: self.vertex_slots,
            edge_slots: self.edge_slots,
        })
    }

    fn edge_lines(&self) -> Result<Vec<StaticEdgeLine>, GraphCommandError> {
        self.graph
            .edges()
            .map(|edge| {
                let source =
                    static_int_slot(&self.graph.vertex(edge.source)?.data, 0, "source vertex")?;
                let destination = static_int_slot(
                    &self.graph.vertex(edge.destination)?.data,
                    0,
                    "destination vertex",
                )?;
                let index = static_int_slot(&edge.data, 1, "edge")?;
                let label = static_text_slot(&edge.data, 2, "edge")?;

                Ok(StaticEdgeLine {
                    index,
                    label,
                    source,
                    destination,
                })
            })
            .collect()
    }
}

fn set_slot(
    slots: &mut [Option<StaticValue>],
    slot: usize,
    value: StaticValue,
    item: &'static str,
) -> Result<(), GraphCommandError> {
    let target = slots
        .get_mut(slot)
        .ok_or(GraphCommandError::MissingSlot { item, slot })?;
    *target = Some(value);
    Ok(())
}

fn static_int_slot(
    slots: &[Option<StaticValue>],
    slot: usize,
    item: &'static str,
) -> Result<i32, GraphCommandError> {
    match slots.get(slot).and_then(Option::as_ref) {
        Some(StaticValue::Int(value)) => Ok(*value),
        _ => Err(GraphCommandError::MissingSlot { item, slot }),
    }
}

fn static_text_slot(
    slots: &[Option<StaticValue>],
    slot: usize,
    item: &'static str,
) -> Result<String, GraphCommandError> {
    match slots.get(slot).and_then(Option::as_ref) {
        Some(StaticValue::Text(value)) => Ok(value.clone()),
        _ => Err(GraphCommandError::MissingSlot { item, slot }),
    }
}

pub fn reverse_vertex_data<T>(left: &T, right: &T) -> Ordering
where
    T: Ord,
{
    right.cmp(left)
}

pub fn verify_topological_order<G, V, E>(
    graph: &Graph<G, V, E>,
    order: &[VertexId],
) -> Result<bool, GraphCommandError> {
    let order_set = order.iter().copied().collect::<BTreeSet<_>>();
    if order_set.len() != graph.vertex_count() {
        return Ok(false);
    }

    for vertex in graph.vertices() {
        if !order_set.contains(&vertex.id) {
            return Ok(false);
        }
    }

    for edge in graph.edges() {
        let source_position = order
            .iter()
            .position(|vertex| *vertex == edge.source)
            .ok_or(GraphCommandError::Graph(GraphError::UnknownVertex(
                edge.source,
            )))?;
        let destination_position = order
            .iter()
            .position(|vertex| *vertex == edge.destination)
            .ok_or(GraphCommandError::Graph(GraphError::UnknownVertex(
                edge.destination,
            )))?;
        if source_position > destination_position {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_graph_registers_legacy_private_commands() {
        assert_eq!(
            init_graph(),
            &[
                CommandRegistration {
                    name: "_graph_test",
                    changes_network: false,
                    command: GraphCommand::GraphTest,
                },
                CommandRegistration {
                    name: "_graph_static_test",
                    changes_network: false,
                    command: GraphCommand::GraphStaticTest,
                },
                CommandRegistration {
                    name: "_graph_dfs_test",
                    changes_network: false,
                    command: GraphCommand::GraphDfsTest,
                },
            ]
        );
    }

    #[test]
    fn graph_test_duplicates_generic_graph_and_month_strings() {
        let report = graph_test().unwrap();

        assert_eq!(report.copied_months, MONTHS);
        assert_eq!(report.source_warning_count, 1);
        assert_eq!(report.duplicate_warning_count, 1);
        assert_eq!(report.duplicate_adjacency.len(), 10);
        assert_eq!(report.duplicate_adjacency[4].vertex, 4);
        assert_eq!(report.duplicate_adjacency[4].goes_to, vec![6, 7, 9]);
        assert_eq!(report.duplicate_adjacency[8].vertex, 9);
        assert_eq!(report.duplicate_adjacency[9].vertex, 10);
        assert!(report.duplicate_adjacency[9].goes_to.is_empty());
        assert!(report.duplicate_adjacency[9].comes_from.is_empty());
    }

    #[test]
    fn graph_static_test_preserves_slots_and_copies_edge_labels() {
        let report = graph_static_test().unwrap();

        assert_eq!(report.graph_slot, Some(StaticValue::Char('f')));
        assert_eq!(
            report.copied_vertex_slots,
            vec![Some(StaticValue::Int(2)), Some(StaticValue::Int(4))]
        );
        assert!(report.lines.iter().all(|line| line.source != 3));
        assert!(report.lines.iter().all(|line| line.destination != 3));
        assert!(
            !report
                .lines
                .iter()
                .any(|line| line.source == 6 && line.destination == 9)
        );
        assert!(report.lines.iter().any(|line| StaticEdgeLine {
            index: 0,
            label: "Jan".to_string(),
            source: 0,
            destination: 1,
        } == *line));
    }

    #[test]
    fn graph_dfs_test_returns_topological_and_reverse_sorts() {
        let graph = build_dfs_test_graph().unwrap();
        let report = graph_dfs_test().unwrap();
        let dfs_ids = report
            .depth_first_sort
            .iter()
            .map(|value| {
                graph
                    .vertices()
                    .find(|vertex| vertex.data == *value)
                    .unwrap()
                    .id
            })
            .collect::<Vec<_>>();

        assert!(verify_topological_order(&graph, &dfs_ids).unwrap());
        assert_eq!(report.depth_first_sort.len(), 10);
        assert_eq!(report.reverse_sort, vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    }

    #[test]
    fn depth_first_sort_rejects_cycles() {
        let mut graph = Graph::<(), i32, ()>::new(());
        let a = graph.add_vertex(0);
        let b = graph.add_vertex(1);
        graph.add_edge(a, b, ()).unwrap();
        graph.add_edge(b, a, ()).unwrap();

        assert_eq!(
            depth_first_sort(&graph),
            Err(GraphCommandError::CycleDetected)
        );
    }

    #[test]
    fn execute_graph_command_dispatches_reports() {
        assert!(matches!(
            execute_graph_command(GraphCommand::GraphDfsTest).unwrap(),
            GraphCommandReport::GraphDfsTest(_)
        ));
    }
}
