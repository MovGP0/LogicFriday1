//! Native Rust command model for `LogicSynthesis/sis/maxflow/com_max.c`.
//!
//! The legacy C file registers debug/test/run commands around the maxflow
//! package. This module keeps the command parsing, sample graph construction,
//! and formatted min-cut report native. Direct SIS command registration remains
//! a higher-level integration concern.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const RUN_MAXFLOW_USAGE: &str = "Usage: _run_maxflow input_file \n";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaxflowCommand {
    ToggleDebug,
    TestGraph,
    RunFile { input_file: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaxflowCommandError {
    MissingInputFile,
    ExtraArguments,
    ImproperInputFormat,
    UnknownNode(String),
    MissingSource,
    MissingSink,
    NegativeCapacity(i32),
    SelfLoop(String),
}

impl fmt::Display for MaxflowCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInputFile => write!(f, "{RUN_MAXFLOW_USAGE}"),
            Self::ExtraArguments => write!(f, "too many arguments for _run_maxflow"),
            Self::ImproperInputFormat => write!(f, "Improper Input format"),
            Self::UnknownNode(name) => write!(f, "unknown maxflow node {name}"),
            Self::MissingSource => write!(f, "maxflow graph has no source node"),
            Self::MissingSink => write!(f, "maxflow graph has no sink node"),
            Self::NegativeCapacity(capacity) => {
                write!(f, "negative maxflow edge capacity {capacity}")
            }
            Self::SelfLoop(name) => write!(f, "self-loop is not allowed for node {name}"),
        }
    }
}

impl Error for MaxflowCommandError {}

pub fn parse_run_maxflow_args<I, S>(args: I) -> Result<MaxflowCommand, MaxflowCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    let Some(input_file) = args.next() else {
        return Err(MaxflowCommandError::MissingInputFile);
    };
    if args.next().is_some() {
        return Err(MaxflowCommandError::ExtraArguments);
    }

    Ok(MaxflowCommand::RunFile {
        input_file: input_file.as_ref().to_owned(),
    })
}

pub fn toggle_debug(debug: &mut bool) {
    *debug = !*debug;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaxflowNodeKind {
    Internal,
    Source,
    Sink,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxflowEdge {
    pub from: String,
    pub to: String,
    pub capacity: i32,
    pub flow: i32,
    pub on_min_cut: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaxflowGraph {
    nodes: HashMap<String, MaxflowNodeKind>,
    edges: Vec<MaxflowEdge>,
}

impl MaxflowGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_node(&mut self, name: impl Into<String>, kind: MaxflowNodeKind) {
        let name = name.into();
        if kind == MaxflowNodeKind::Source || kind == MaxflowNodeKind::Sink {
            for existing in self.nodes.values_mut() {
                if *existing == kind {
                    *existing = MaxflowNodeKind::Internal;
                }
            }
        }
        self.nodes.insert(name, kind);
    }

    pub fn read_edge(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        capacity: i32,
    ) -> Result<(), MaxflowCommandError> {
        let from = from.into();
        let to = to.into();
        self.validate_edge(&from, &to, capacity)?;
        self.edges.push(MaxflowEdge {
            from,
            to,
            capacity,
            flow: 0,
            on_min_cut: false,
        });
        Ok(())
    }

    pub fn reread_edge(
        &mut self,
        from: &str,
        to: &str,
        capacity: i32,
    ) -> Result<(), MaxflowCommandError> {
        self.validate_edge(from, to, capacity)?;
        if let Some(edge) = self
            .edges
            .iter_mut()
            .find(|edge| edge.from == from && edge.to == to)
        {
            edge.capacity = capacity;
            edge.flow = edge.flow.clamp(0, capacity);
            return Ok(());
        }

        self.read_edge(from, to, capacity)
    }

    pub fn remove_node(&mut self, name: &str) -> bool {
        let removed = self.nodes.remove(name).is_some();
        if removed {
            self.edges
                .retain(|edge| edge.from != name && edge.to != name);
        }
        removed
    }

    pub fn change_node_type(
        &mut self,
        name: &str,
        kind: MaxflowNodeKind,
    ) -> Result<(), MaxflowCommandError> {
        if !self.nodes.contains_key(name) {
            return Err(MaxflowCommandError::UnknownNode(name.to_owned()));
        }
        self.read_node(name, kind);
        Ok(())
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&str, MaxflowNodeKind)> {
        self.nodes.iter().map(|(name, kind)| (name.as_str(), *kind))
    }

    pub fn edges(&self) -> &[MaxflowEdge] {
        &self.edges
    }

    pub fn source_name(&self) -> Option<&str> {
        self.nodes
            .iter()
            .find_map(|(name, kind)| (*kind == MaxflowNodeKind::Source).then_some(name.as_str()))
    }

    pub fn sink_name(&self) -> Option<&str> {
        self.nodes
            .iter()
            .find_map(|(name, kind)| (*kind == MaxflowNodeKind::Sink).then_some(name.as_str()))
    }

    fn validate_edge(
        &self,
        from: &str,
        to: &str,
        capacity: i32,
    ) -> Result<(), MaxflowCommandError> {
        if capacity < 0 {
            return Err(MaxflowCommandError::NegativeCapacity(capacity));
        }
        if from == to {
            return Err(MaxflowCommandError::SelfLoop(from.to_owned()));
        }
        if !self.nodes.contains_key(from) {
            return Err(MaxflowCommandError::UnknownNode(from.to_owned()));
        }
        if !self.nodes.contains_key(to) {
            return Err(MaxflowCommandError::UnknownNode(to.to_owned()));
        }
        Ok(())
    }

    fn reset_flow(&mut self) {
        for edge in &mut self.edges {
            edge.flow = 0;
            edge.on_min_cut = false;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CutsetArc {
    pub from: String,
    pub to: String,
    pub flow: i32,
    pub capacity: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxflowReport {
    pub cutset: Vec<CutsetArc>,
    pub value: i32,
}

pub fn build_test_graph() -> Result<MaxflowGraph, MaxflowCommandError> {
    let mut graph = MaxflowGraph::new();
    graph.read_node("mf_source", MaxflowNodeKind::Source);
    graph.read_node("mf_sink", MaxflowNodeKind::Sink);
    for name in ["a", "b", "c", "d", "e", "f", "g"] {
        graph.read_node(name, MaxflowNodeKind::Internal);
    }

    for (from, to, capacity) in [
        ("mf_source", "a", 2),
        ("mf_source", "b", 2),
        ("a", "c", 3),
        ("b", "c", 1),
        ("b", "d", 2),
        ("c", "f", 4),
        ("d", "e", 1),
        ("e", "mf_sink", 2),
        ("f", "e", 2),
        ("f", "g", 3),
        ("g", "mf_sink", 1),
    ] {
        graph.read_edge(from, to, capacity)?;
    }

    Ok(graph)
}

pub fn parse_run_maxflow_input(input: &str) -> Result<MaxflowGraph, MaxflowCommandError> {
    let mut lines = input.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Err(MaxflowCommandError::ImproperInputFormat);
    };
    let mut header_parts = header.split_whitespace();
    let source = parse_labeled_token(header_parts.next(), "Source:")?;
    let sink = parse_labeled_token(header_parts.next(), "Sink:")?;
    if header_parts.next().is_some() {
        return Err(MaxflowCommandError::ImproperInputFormat);
    }

    let mut graph = MaxflowGraph::new();
    graph.read_node(source, MaxflowNodeKind::Source);
    graph.read_node(sink, MaxflowNodeKind::Sink);

    let _ = lines.next();
    for line in lines {
        let mut parts = line.split_whitespace();
        let Some(from) = parts.next() else {
            continue;
        };
        let Some(to) = parts.next() else {
            return Err(MaxflowCommandError::ImproperInputFormat);
        };
        let Some(capacity) = parts.next() else {
            return Err(MaxflowCommandError::ImproperInputFormat);
        };
        if parts.next().is_some() {
            return Err(MaxflowCommandError::ImproperInputFormat);
        }

        let capacity = capacity
            .parse()
            .map_err(|_| MaxflowCommandError::ImproperInputFormat)?;
        if !graph.nodes.contains_key(from) {
            graph.read_node(from, MaxflowNodeKind::Internal);
        }
        if !graph.nodes.contains_key(to) {
            graph.read_node(to, MaxflowNodeKind::Internal);
        }
        graph.read_edge(from, to, capacity)?;
    }

    Ok(graph)
}

pub fn run_maxflow(graph: &mut MaxflowGraph) -> Result<MaxflowReport, MaxflowCommandError> {
    let source = graph
        .source_name()
        .ok_or(MaxflowCommandError::MissingSource)?
        .to_owned();
    let sink = graph
        .sink_name()
        .ok_or(MaxflowCommandError::MissingSink)?
        .to_owned();
    graph.reset_flow();

    while let Some(path) = find_augmenting_path(graph, &source, &sink) {
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

    let reachable = residual_reachable(graph, &source);
    let mut cutset = Vec::new();
    let mut value = 0_i32;
    for edge in &mut graph.edges {
        edge.on_min_cut =
            reachable.contains(edge.from.as_str()) && !reachable.contains(edge.to.as_str());
        if edge.on_min_cut {
            value += edge.flow;
            cutset.push(CutsetArc {
                from: edge.from.clone(),
                to: edge.to.clone(),
                flow: edge.flow,
                capacity: edge.capacity,
            });
        }
    }

    Ok(MaxflowReport { cutset, value })
}

pub fn format_cutset_report(report: &MaxflowReport) -> String {
    let mut output = String::new();
    output.push_str("Following maxflow/mincut found:\n");
    output.push_str("    From       to       flow\n");
    for arc in &report.cutset {
        output.push_str(&format!(
            " {:<10}{:<10}  {:>3}\n",
            arc.from, arc.to, arc.flow
        ));
    }
    output
}

pub fn format_test_cutset(report: &MaxflowReport) -> String {
    let mut output = String::new();
    output.push_str("    From       to       flow\n");
    for arc in &report.cutset {
        output.push_str(&format!(
            " {:<10}{:<10}  {:>3}\n",
            arc.from, arc.to, arc.flow
        ));
    }
    output.push_str(&format!("Cutset has {} arcs\n", report.cutset.len()));
    for arc in &report.cutset {
        output.push_str(&format!(
            "{} -> {} : Cap = {}, flow = {}\n",
            arc.from, arc.to, arc.capacity, arc.flow
        ));
    }
    output
}

fn parse_labeled_token<'a>(
    token: Option<&'a str>,
    label: &str,
) -> Result<&'a str, MaxflowCommandError> {
    let Some(token) = token else {
        return Err(MaxflowCommandError::ImproperInputFormat);
    };
    token
        .strip_prefix(label)
        .filter(|value| !value.is_empty())
        .ok_or(MaxflowCommandError::ImproperInputFormat)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResidualStep {
    edge_index: usize,
    forward: bool,
}

fn find_augmenting_path(
    graph: &MaxflowGraph,
    source: &str,
    sink: &str,
) -> Option<Vec<ResidualStep>> {
    let mut parents: HashMap<&str, (ResidualStep, &str)> = HashMap::new();
    let mut queue = VecDeque::from([source]);

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
    let mut node = sink;
    while node != source {
        let (step, parent) = parents[node];
        path.push(step);
        node = parent;
    }
    path.reverse();
    Some(path)
}

fn residual_reachable(graph: &MaxflowGraph, source: &str) -> HashSet<String> {
    let mut reachable = HashSet::from([source.to_owned()]);
    let mut queue = VecDeque::from([source.to_owned()]);

    while let Some(node) = queue.pop_front() {
        for edge in &graph.edges {
            if edge.from == node.as_str()
                && edge.capacity > edge.flow
                && !reachable.contains(edge.to.as_str())
            {
                reachable.insert(edge.to.clone());
                queue.push_back(edge.to.clone());
            }
            if edge.to == node.as_str() && edge.flow > 0 && !reachable.contains(edge.from.as_str())
            {
                reachable.insert(edge.from.clone());
                queue.push_back(edge.from.clone());
            }
        }
    }

    reachable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_debug_flag_like_command() {
        let mut debug = false;
        toggle_debug(&mut debug);
        assert!(debug);
        toggle_debug(&mut debug);
        assert!(!debug);
    }

    #[test]
    fn parses_run_maxflow_single_input_file() {
        assert_eq!(
            parse_run_maxflow_args(["input.mf"]),
            Ok(MaxflowCommand::RunFile {
                input_file: "input.mf".to_owned()
            })
        );
        assert_eq!(
            parse_run_maxflow_args(std::iter::empty::<&str>()),
            Err(MaxflowCommandError::MissingInputFile)
        );
    }

    #[test]
    fn parses_c_style_input_and_computes_cutset() {
        let input = "\
Source:s Sink:t
ignored header
s a 3
s b 2
a t 2
b t 4
";
        let mut graph = parse_run_maxflow_input(input).unwrap();
        let report = run_maxflow(&mut graph).unwrap();

        assert_eq!(report.value, 4);
        assert_eq!(report.cutset.len(), 2);
        assert!(format_cutset_report(&report).contains("Following maxflow/mincut found"));
    }

    #[test]
    fn sample_graph_matches_test_command_flow_value() {
        let mut graph = build_test_graph().unwrap();
        let report = run_maxflow(&mut graph).unwrap();

        assert_eq!(report.value, 3);
        assert!(format_test_cutset(&report).contains("Cutset has"));
    }

    #[test]
    fn graph_mutation_matches_test_command_sequence() {
        let mut graph = build_test_graph().unwrap();
        assert!(graph.remove_node("d"));
        graph.reread_edge("g", "mf_sink", 3).unwrap();
        graph
            .change_node_type("mf_source", MaxflowNodeKind::Internal)
            .unwrap();
        graph
            .change_node_type("a", MaxflowNodeKind::Source)
            .unwrap();
        let report = run_maxflow(&mut graph).unwrap();

        assert_eq!(graph.source_name(), Some("a"));
        assert_eq!(report.value, 3);
    }

    #[test]
    fn rejects_bad_input_and_invalid_edges() {
        assert_eq!(
            parse_run_maxflow_input("Source:s\n"),
            Err(MaxflowCommandError::ImproperInputFormat)
        );
        let mut graph = MaxflowGraph::new();
        graph.read_node("s", MaxflowNodeKind::Source);
        graph.read_node("t", MaxflowNodeKind::Sink);
        assert_eq!(
            graph.read_edge("s", "t", -1),
            Err(MaxflowCommandError::NegativeCapacity(-1))
        );
        assert_eq!(
            graph.read_edge("s", "s", 1),
            Err(MaxflowCommandError::SelfLoop("s".to_owned()))
        );
    }
}
