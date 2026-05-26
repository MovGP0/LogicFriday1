//! Native Rust model for `LogicSynthesis/sis/pld/xln_ULM_util.c`.
//!
//! The C file is a small utility layer over SIS `maxflow`, `network`,
//! `node`, `array`, `st`, and `nodeindex` APIs. This module ports the
//! independent behavior to Rust data structures and reports the canonical
//! SIS-backed entry points as explicit missing dependency errors until those
//! ports are available.

use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt;

pub const SIS_MAXINT: i32 = 1 << 30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.279",
        source_file: "LogicSynthesis/sis/maxflow/maxflow.c",
        reason: "maxflow(), mf_get_node(), source-node access, and edge flow/head traversal",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.280",
        source_file: "LogicSynthesis/sis/maxflow/mf_input.c",
        reason: "mf_reread_edge() capacity updates used by change_edge_capacity",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "network_find_node() lookup used by graph2network_node",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node_long_name() and node identity/name storage",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "foreach_fanin traversal used by print_fanin",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.320",
        source_file: "LogicSynthesis/sis/node/nodeindex.c",
        reason: "nodeindex_indexof() used by print_array",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        reason: "array_t sorted pointer arrays used by count_intsec_union and print_array",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "st_foreach() debug table traversal used by print_table",
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnUlmUtilError {
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    UnknownNode(String),
    UnknownEdge {
        from: String,
        to: String,
    },
    MissingSourceNode,
    NegativeCapacity(i32),
    SelfLoop(String),
}

impl fmt::Display for XlnUlmUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} native SIS prerequisite ports",
                dependencies.len()
            ),
            Self::UnknownNode(name) => write!(f, "unknown maxflow node {name}"),
            Self::UnknownEdge { from, to } => write!(f, "unknown maxflow edge {from} -> {to}"),
            Self::MissingSourceNode => write!(f, "maxflow graph has no source node"),
            Self::NegativeCapacity(capacity) => {
                write!(f, "negative maxflow edge capacity {capacity}")
            }
            Self::SelfLoop(node) => write!(f, "self-loop is not allowed for node {node}"),
        }
    }
}

impl Error for XlnUlmUtilError {}

pub fn required_port_beads() -> &'static [PortDependency] {
    REQUIRED_PORT_BEADS
}

pub fn sis_bound_operation_unavailable(operation: &'static str) -> Result<(), XlnUlmUtilError> {
    Err(XlnUlmUtilError::MissingSisPorts {
        operation,
        dependencies: REQUIRED_PORT_BEADS,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowNodeKind {
    Internal,
    Source,
    Sink,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    pub capacity: i32,
    pub flow: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FlowGraph {
    nodes: HashMap<String, FlowNodeKind>,
    edges: Vec<FlowEdge>,
}

impl FlowGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, name: impl Into<String>, kind: FlowNodeKind) {
        let name = name.into();
        if kind == FlowNodeKind::Source {
            for existing in self.nodes.values_mut() {
                if *existing == FlowNodeKind::Source {
                    *existing = FlowNodeKind::Internal;
                }
            }
        }
        self.nodes.insert(name, kind);
    }

    pub fn add_edge(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        capacity: i32,
    ) -> Result<usize, XlnUlmUtilError> {
        let from = from.into();
        let to = to.into();
        self.validate_edge_endpoints(&from, &to, capacity)?;
        let index = self.edges.len();
        self.edges.push(FlowEdge {
            from,
            to,
            capacity,
            flow: 0,
        });
        Ok(index)
    }

    pub fn reread_edge(
        &mut self,
        from: &str,
        to: &str,
        capacity: i32,
    ) -> Result<usize, XlnUlmUtilError> {
        self.validate_edge_endpoints(from, to, capacity)?;
        if let Some(index) = self.edge_index(from, to) {
            let edge = &mut self.edges[index];
            edge.capacity = capacity;
            edge.flow = edge.flow.clamp(0, capacity);
            return Ok(index);
        }

        let index = self.edges.len();
        self.edges.push(FlowEdge {
            from: from.to_owned(),
            to: to.to_owned(),
            capacity,
            flow: 0,
        });
        Ok(index)
    }

    pub fn edge(&self, index: usize) -> Option<&FlowEdge> {
        self.edges.get(index)
    }

    pub fn edge_by_names(&self, from: &str, to: &str) -> Option<&FlowEdge> {
        self.edge_index(from, to)
            .and_then(|index| self.edges.get(index))
    }

    pub fn set_edge_flow(
        &mut self,
        from: &str,
        to: &str,
        flow: i32,
    ) -> Result<(), XlnUlmUtilError> {
        let index = self
            .edge_index(from, to)
            .ok_or_else(|| XlnUlmUtilError::UnknownEdge {
                from: from.to_owned(),
                to: to.to_owned(),
            })?;
        self.edges[index].flow = flow;
        Ok(())
    }

    pub fn source_name(&self) -> Option<&str> {
        self.nodes
            .iter()
            .find_map(|(name, kind)| (*kind == FlowNodeKind::Source).then_some(name.as_str()))
    }

    pub fn fanout_edges<'a>(&'a self, node: &'a str) -> impl Iterator<Item = &'a FlowEdge> + 'a {
        self.edges.iter().filter(move |edge| edge.from == node)
    }

    fn edge_index(&self, from: &str, to: &str) -> Option<usize> {
        self.edges
            .iter()
            .position(|edge| edge.from == from && edge.to == to)
    }

    fn validate_edge_endpoints(
        &self,
        from: &str,
        to: &str,
        capacity: i32,
    ) -> Result<(), XlnUlmUtilError> {
        if capacity < 0 {
            return Err(XlnUlmUtilError::NegativeCapacity(capacity));
        }
        if from == to {
            return Err(XlnUlmUtilError::SelfLoop(from.to_owned()));
        }
        if !self.nodes.contains_key(from) {
            return Err(XlnUlmUtilError::UnknownNode(from.to_owned()));
        }
        if !self.nodes.contains_key(to) {
            return Err(XlnUlmUtilError::UnknownNode(to.to_owned()));
        }
        Ok(())
    }

    fn reset_flows(&mut self) {
        for edge in &mut self.edges {
            edge.flow = 0;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub name: String,
    pub fanins: Vec<String>,
}

impl NetworkNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fanins: Vec::new(),
        }
    }

    pub fn with_fanins(name: impl Into<String>, fanins: impl IntoIterator<Item = String>) -> Self {
        Self {
            name: name.into(),
            fanins: fanins.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Network {
    nodes: HashMap<String, NetworkNode>,
}

impl Network {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: NetworkNode) {
        self.nodes.insert(node.name.clone(), node);
    }

    pub fn find_node(&self, name: &str) -> Option<&NetworkNode> {
        self.nodes.get(name)
    }
}

pub fn change_edge_capacity(
    graph: &mut FlowGraph,
    node_name: &str,
    capacity: i32,
) -> Result<usize, XlnUlmUtilError> {
    let top = format!("{node_name}_top");
    let bottom = format!("{node_name}_bottom");
    graph.reread_edge(&bottom, &top, capacity)
}

pub fn get_maxflow(graph: &mut FlowGraph) -> Result<i32, XlnUlmUtilError> {
    run_maxflow(graph)?;
    get_source_fanout_flow(graph)
}

pub fn get_source_fanout_flow(graph: &FlowGraph) -> Result<i32, XlnUlmUtilError> {
    let source = graph
        .source_name()
        .ok_or(XlnUlmUtilError::MissingSourceNode)?;
    let mut flow = 0_i32;
    for edge in graph.fanout_edges(source) {
        let save_flow = flow;
        flow = flow.saturating_add(edge.flow);
        if flow < save_flow || flow == i32::MAX {
            return Ok(SIS_MAXINT);
        }
    }
    Ok(flow)
}

pub fn get_maxflow_edge<'a>(
    graph: &'a FlowGraph,
    name1: &str,
    name2: &str,
) -> Option<&'a FlowEdge> {
    graph.edge_by_names(name1, name2)
}

pub fn graph2network_node<'a>(
    network: &'a Network,
    graph_node_name: &str,
) -> Option<&'a NetworkNode> {
    network.find_node(strip_graph_node_suffix(graph_node_name))
}

pub fn strip_graph_node_suffix(graph_node_name: &str) -> &str {
    graph_node_name
        .rsplit_once('_')
        .map_or(graph_node_name, |(base, _)| base)
}

pub fn format_fanin(node: &NetworkNode) -> String {
    let mut output = format!("Fanins of node {} = ", node.name);
    for fanin in &node.fanins {
        output.push_str(fanin);
        output.push(' ');
    }
    output.push('\n');
    output
}

pub fn comp_ptr<T>(obj1: *const T, obj2: *const T) -> Ordering {
    (obj1 as usize).cmp(&(obj2 as usize))
}

pub fn comp_ptr_c<T>(obj1: *const T, obj2: *const T) -> i32 {
    match comp_ptr(obj1, obj2) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

pub fn count_intsec_union<T>(array1: &[*const T], array2: &[*const T]) -> (usize, usize) {
    let mut i = 0;
    let mut j = 0;
    let mut num_intsec = 0;
    let mut num_union = 0;

    loop {
        if i > array1.len().saturating_sub(1) {
            num_union += array2.len() - j;
            break;
        }
        if j > array2.len().saturating_sub(1) {
            num_union += array1.len() - i;
            break;
        }

        match comp_ptr(array1[i], array2[j]) {
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                num_intsec += 1;
                i += 1;
                j += 1;
            }
        }
        num_union += 1;
    }

    (num_intsec, num_union)
}

pub fn format_table_entry(key: &str, value: usize, arg: Option<&str>) -> String {
    format!(
        "key = {key}, value = {value:x} arg = {}\n",
        arg.unwrap_or("")
    )
}

pub fn format_array(nodes: &[NetworkNode], indexes: &HashMap<String, usize>) -> String {
    let mut output = String::new();
    for node in nodes {
        let index = indexes.get(&node.name).copied().unwrap_or(usize::MAX);
        output.push_str(&format!("{}({index}) ", node.name));
    }
    output.push('\n');
    output
}

fn run_maxflow(graph: &mut FlowGraph) -> Result<(), XlnUlmUtilError> {
    let source = graph
        .source_name()
        .ok_or(XlnUlmUtilError::MissingSourceNode)?
        .to_owned();
    graph.reset_flows();

    while let Some(path) = find_augmenting_path(graph, &source) {
        let residual = path
            .iter()
            .map(|step| {
                let edge = &graph.edges[step.edge_index];
                if step.forward {
                    edge.capacity - edge.flow
                } else {
                    edge.flow
                }
            })
            .min()
            .unwrap_or(0);

        if residual <= 0 {
            break;
        }

        for step in path {
            if step.forward {
                graph.edges[step.edge_index].flow += residual;
            } else {
                graph.edges[step.edge_index].flow -= residual;
            }
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResidualStep {
    edge_index: usize,
    forward: bool,
}

fn find_augmenting_path(graph: &FlowGraph, source: &str) -> Option<Vec<ResidualStep>> {
    let mut parents: HashMap<&str, (ResidualStep, &str)> = HashMap::new();
    let mut queue = VecDeque::from([source]);
    let sink = graph
        .nodes
        .iter()
        .find_map(|(name, kind)| (*kind == FlowNodeKind::Sink).then_some(name.as_str()))?;

    while let Some(node) = queue.pop_front() {
        if node == sink {
            break;
        }

        for (edge_index, edge) in graph.edges.iter().enumerate() {
            if edge.from == node
                && edge.capacity > edge.flow
                && edge.to != source
                && !parents.contains_key(edge.to.as_str())
            {
                parents.insert(
                    edge.to.as_str(),
                    (
                        ResidualStep {
                            edge_index,
                            forward: true,
                        },
                        node,
                    ),
                );
                queue.push_back(edge.to.as_str());
            }

            if edge.to == node
                && edge.flow > 0
                && edge.from != source
                && !parents.contains_key(edge.from.as_str())
            {
                parents.insert(
                    edge.from.as_str(),
                    (
                        ResidualStep {
                            edge_index,
                            forward: false,
                        },
                        node,
                    ),
                );
                queue.push_back(edge.from.as_str());
            }
        }
    }

    if !parents.contains_key(sink) {
        return None;
    }

    let mut path = Vec::new();
    let mut current = sink;
    while current != source {
        let (step, previous) = parents[current];
        path.push(step);
        current = previous;
    }
    path.reverse();
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_edge_capacity_updates_bottom_to_top_edge() {
        let mut graph = FlowGraph::new();
        graph.add_node("n_bottom", FlowNodeKind::Internal);
        graph.add_node("n_top", FlowNodeKind::Internal);
        graph.add_edge("n_bottom", "n_top", 3).unwrap();

        let index = change_edge_capacity(&mut graph, "n", 9).unwrap();

        assert_eq!(graph.edge(index).unwrap().capacity, 9);
        assert_eq!(
            get_maxflow_edge(&graph, "n_bottom", "n_top").map(|edge| edge.capacity),
            Some(9)
        );
    }

    #[test]
    fn get_maxflow_computes_and_sums_source_fanout_flows() {
        let mut graph = FlowGraph::new();
        for (name, kind) in [
            ("mf_source", FlowNodeKind::Source),
            ("a", FlowNodeKind::Internal),
            ("b", FlowNodeKind::Internal),
            ("mf_sink", FlowNodeKind::Sink),
        ] {
            graph.add_node(name, kind);
        }
        graph.add_edge("mf_source", "a", 2).unwrap();
        graph.add_edge("mf_source", "b", 1).unwrap();
        graph.add_edge("a", "mf_sink", 1).unwrap();
        graph.add_edge("b", "mf_sink", 2).unwrap();

        assert_eq!(get_maxflow(&mut graph), Ok(2));
        assert_eq!(
            graph.edge_by_names("mf_source", "a").map(|edge| edge.flow),
            Some(1)
        );
        assert_eq!(
            graph.edge_by_names("mf_source", "b").map(|edge| edge.flow),
            Some(1)
        );
    }

    #[test]
    fn get_source_fanout_flow_saturates_like_c_overflow_guard() {
        let mut graph = FlowGraph::new();
        graph.add_node("source", FlowNodeKind::Source);
        graph.add_node("a", FlowNodeKind::Internal);
        graph.add_node("b", FlowNodeKind::Internal);
        graph.add_edge("source", "a", i32::MAX).unwrap();
        graph.add_edge("source", "b", i32::MAX).unwrap();
        graph.set_edge_flow("source", "a", i32::MAX - 1).unwrap();
        graph.set_edge_flow("source", "b", 10).unwrap();

        assert_eq!(get_source_fanout_flow(&graph), Ok(SIS_MAXINT));
    }

    #[test]
    fn augmenting_path_can_use_residual_back_edges() {
        let mut graph = FlowGraph::new();
        for (name, kind) in [
            ("source", FlowNodeKind::Source),
            ("a", FlowNodeKind::Internal),
            ("b", FlowNodeKind::Internal),
            ("sink", FlowNodeKind::Sink),
        ] {
            graph.add_node(name, kind);
        }
        graph.add_edge("source", "a", 1).unwrap();
        graph.add_edge("a", "b", 1).unwrap();
        graph.add_edge("b", "sink", 1).unwrap();
        graph.add_edge("source", "b", 1).unwrap();
        graph.add_edge("a", "sink", 1).unwrap();
        graph.set_edge_flow("source", "a", 1).unwrap();
        graph.set_edge_flow("a", "b", 1).unwrap();
        graph.set_edge_flow("b", "sink", 1).unwrap();

        let path = find_augmenting_path(&graph, "source").unwrap();

        assert_eq!(
            path,
            vec![
                ResidualStep {
                    edge_index: 3,
                    forward: true
                },
                ResidualStep {
                    edge_index: 1,
                    forward: false
                },
                ResidualStep {
                    edge_index: 4,
                    forward: true
                },
            ]
        );
    }

    #[test]
    fn graph2network_node_strips_final_modifier_suffix() {
        let mut network = Network::new();
        network.add_node(NetworkNode::new("abc_def"));

        assert_eq!(
            graph2network_node(&network, "abc_def_top").map(|node| node.name.as_str()),
            Some("abc_def")
        );
        assert!(graph2network_node(&network, "missing_bottom").is_none());
    }

    #[test]
    fn format_helpers_match_c_debug_shape() {
        let node = NetworkNode::with_fanins("out", ["a".to_owned(), "b".to_owned()]);
        assert_eq!(format_fanin(&node), "Fanins of node out = a b \n");
        assert_eq!(
            format_table_entry("k", 0x12ab, Some("ctx")),
            "key = k, value = 12ab arg = ctx\n"
        );
    }

    #[test]
    fn count_intsec_union_matches_sorted_pointer_merge() {
        let values = [10_i32, 20, 30, 40];
        let mut left = vec![&values[0] as *const i32, &values[1], &values[3]];
        let mut right = vec![&values[1] as *const i32, &values[2], &values[3]];
        left.sort_by(|a, b| comp_ptr(*a, *b));
        right.sort_by(|a, b| comp_ptr(*a, *b));

        assert_eq!(count_intsec_union(&left, &right), (2, 4));
    }

    #[test]
    fn sis_bound_scaffold_reports_dependency_beads_and_sources() {
        let Err(XlnUlmUtilError::MissingSisPorts {
            operation,
            dependencies,
        }) = sis_bound_operation_unavailable("graph2network_node")
        else {
            panic!("expected missing SIS ports");
        };

        assert_eq!(operation, "graph2network_node");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.279"
                && dependency.source_file == "LogicSynthesis/sis/maxflow/maxflow.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.305"
                && dependency.source_file == "LogicSynthesis/sis/network/network_util.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.485"
                && dependency.source_file == "LogicSynthesis/sis/st/st.c"
        }));
    }
}
