//! Native Rust model for `LogicSynthesis/sis/retime/re_milp.c`.
//!
//! The C file builds a Li/Leiserson mixed-integer retiming constraint graph,
//! solves it with the paper's shortest-path style relaxation, and then applies
//! the resulting lags with `retime_single_node`. This module ports that
//! algorithm over owned Rust data. Direct execution against SIS `re_graph`,
//! `re_node`, `re_edge`, `st_table`, and `avl_tree` objects is represented by
//! explicit dependency errors until those sibling ports are native.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const RETIME_TEST_NOT_SET: f64 = -50_000.0;
const SCALE: f64 = 10_000.0;
const EPSILON: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        reason: "legacy retime graphs store nodes, edges, fanins, and fanouts in array_t",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "native fanin and fanout traversal for SIS-backed retime graphs",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "native node identity and names used when reporting and applying retiming",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.415",
        source_file: "LogicSynthesis/sis/retime/re_graph.c",
        reason: "retime command flow and graph-level retiming orchestration",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.422",
        source_file: "LogicSynthesis/sis/retime/re_util.c",
        reason: "retime_single_node and native mutation of edge weights after solving",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.423",
        source_file: "LogicSynthesis/sis/retime/retime_util.c",
        reason: "native retime graph allocation, node indexing, and edge accessor helpers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "legacy re_milp.c maps re_node pointers to integer MILP indices with st_table",
    },
];

pub fn required_port_beads() -> &'static [PortDependency] {
    REQUIRED_PORT_BEADS
}

pub fn retime_lies_routine_for_sis_graph() -> Result<RetimeSolution, RetimeMilpError> {
    Err(RetimeMilpError::MissingNativeDependencies {
        operation: "retime_lies_routine over SIS re_graph",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNode {
    pub name: String,
    pub kind: RetimeNodeKind,
    pub final_delay: f64,
    pub user_time: f64,
}

impl RetimeNode {
    pub fn new(name: impl Into<String>, kind: RetimeNodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            final_delay: 0.0,
            user_time: RETIME_TEST_NOT_SET - 1.0,
        }
    }

    pub fn internal(name: impl Into<String>, final_delay: f64) -> Self {
        Self {
            name: name.into(),
            kind: RetimeNodeKind::Internal,
            final_delay,
            user_time: RETIME_TEST_NOT_SET - 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeEdge {
    pub source: usize,
    pub sink: usize,
    pub weight: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RetimeGraph {
    pub nodes: Vec<RetimeNode>,
    pub edges: Vec<RetimeEdge>,
}

impl RetimeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: RetimeNode) -> usize {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    pub fn add_edge(
        &mut self,
        source: usize,
        sink: usize,
        weight: i32,
    ) -> Result<usize, RetimeMilpError> {
        if source >= self.nodes.len() {
            return Err(RetimeMilpError::MissingNode { node_id: source });
        }
        if sink >= self.nodes.len() {
            return Err(RetimeMilpError::MissingNode { node_id: sink });
        }
        let id = self.edges.len();
        self.edges.push(RetimeEdge {
            source,
            sink,
            weight,
        });
        Ok(id)
    }

    fn internal_index_map(&self) -> HashMap<usize, usize> {
        let mut next = 1;
        let mut indices = HashMap::new();
        for (id, node) in self.nodes.iter().enumerate() {
            if node.kind == RetimeNodeKind::Internal {
                indices.insert(id, next);
                next += 1;
            }
        }
        indices
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeSolution {
    pub feasible: bool,
    pub retiming: Vec<i32>,
    pub integer_lags: Vec<f64>,
    pub arrivals: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetimeMilpError {
    MissingNativeDependencies {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    MissingNode {
        node_id: usize,
    },
    NonFiniteCycleTime {
        cycle_time: f64,
    },
    EmptyGraph,
    EdgeEndpointMustBeInternalOrIo {
        edge_id: usize,
    },
}

impl fmt::Display for RetimeMilpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativeDependencies {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} native SIS prerequisite ports",
                dependencies.len()
            ),
            Self::MissingNode { node_id } => {
                write!(f, "retime graph references missing node {node_id}")
            }
            Self::NonFiniteCycleTime { cycle_time } => {
                write!(
                    f,
                    "retime MILP cycle time must be finite and positive, got {cycle_time}"
                )
            }
            Self::EmptyGraph => write!(f, "retime MILP graph must contain at least one node"),
            Self::EdgeEndpointMustBeInternalOrIo { edge_id } => write!(
                f,
                "retime MILP edge {edge_id} is incident on an ignored node"
            ),
        }
    }
}

impl Error for RetimeMilpError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConstraintKind {
    Real,
    Integer,
}

#[derive(Clone, Debug, PartialEq)]
struct ConstraintEdge {
    from: usize,
    to: usize,
    weight_a: f64,
    weight_b: f64,
    kind: ConstraintKind,
}

#[derive(Clone, Debug, PartialEq)]
struct ConstraintNode {
    fanin: Vec<usize>,
    r: f64,
    y: f64,
    x: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeMilpGraph {
    nodes: Vec<ConstraintNode>,
    edges: Vec<ConstraintEdge>,
    num_int: usize,
}

impl RetimeMilpGraph {
    pub fn num_integer_nodes(&self) -> usize {
        self.num_int
    }

    pub fn num_total_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn num_constraints(&self) -> usize {
        self.edges.len()
    }
}

pub fn build_lies_constraint_graph(
    graph: &RetimeGraph,
    cycle_time: f64,
) -> Result<RetimeMilpGraph, RetimeMilpError> {
    validate_problem(graph, cycle_time)?;

    let internal_indices = graph.internal_index_map();
    let num_int = internal_indices.len() + 1;
    let num_total = 2 * num_int;
    let mut milp = RetimeMilpGraph {
        nodes: (0..num_total)
            .map(|_| ConstraintNode {
                fanin: Vec::new(),
                r: 0.0,
                y: 0.0,
                x: 0.0,
            })
            .collect(),
        edges: Vec::new(),
        num_int,
    };

    add_vertex_constraints(&mut milp, None, 0, cycle_time);
    for (node_id, node) in graph.nodes.iter().enumerate() {
        match node.kind {
            RetimeNodeKind::PrimaryOutput => {
                for edge in graph.edges.iter().filter(|edge| edge.sink == node_id) {
                    let from = node_to_index(&internal_indices, graph, edge.source)?;
                    add_edge_constraint(&mut milp, from, 0, edge, graph, cycle_time);
                }
            }
            RetimeNodeKind::Internal => {
                let to = node_to_index(&internal_indices, graph, node_id)?;
                add_vertex_constraints(&mut milp, Some(node), to, cycle_time);
                for edge in graph.edges.iter().filter(|edge| edge.sink == node_id) {
                    let from = node_to_index(&internal_indices, graph, edge.source)?;
                    add_edge_constraint(&mut milp, from, to, edge, graph, cycle_time);
                }
            }
            RetimeNodeKind::PrimaryInput | RetimeNodeKind::Ignore => {}
        }

        add_user_time_constraints(&mut milp, graph, &internal_indices, node_id, cycle_time)?;
    }

    Ok(milp)
}

pub fn retime_lies_routine(
    graph: &mut RetimeGraph,
    cycle_time: f64,
) -> Result<RetimeSolution, RetimeMilpError> {
    let mut milp = build_lies_constraint_graph(graph, cycle_time)?;
    if !solve_lies_constraint_graph(&mut milp) {
        return Ok(RetimeSolution {
            feasible: false,
            retiming: vec![0; graph.nodes.len()],
            integer_lags: Vec::new(),
            arrivals: Vec::new(),
        });
    }

    let internal_indices = graph.internal_index_map();
    let mut retiming = vec![0; graph.nodes.len()];
    let mut integer_lags = vec![0.0; graph.nodes.len()];
    let mut arrivals = vec![0.0; graph.nodes.len()];
    for (node_id, node) in graph.nodes.iter().enumerate() {
        if node.kind != RetimeNodeKind::Internal {
            continue;
        }
        let from = node_to_index(&internal_indices, graph, node_id)?;
        let lag = milp.nodes[from].x as i32;
        retiming[node_id] = lag;
        integer_lags[node_id] = milp.nodes[from].x;
        arrivals[node_id] = cycle_time * (milp.nodes[from + milp.num_int].x - milp.nodes[from].x);
    }

    apply_retiming(graph, &retiming)?;
    Ok(RetimeSolution {
        feasible: true,
        retiming,
        integer_lags,
        arrivals,
    })
}

fn solve_lies_constraint_graph(milp: &mut RetimeMilpGraph) -> bool {
    let num_int = milp.num_int;
    let num_total = milp.nodes.len();

    for _ in num_int..num_total {
        for j in num_int..num_total {
            let fanins = milp.nodes[j].fanin.clone();
            for edge_id in fanins {
                let edge = &milp.edges[edge_id];
                let candidate = milp.nodes[edge.from].r + edge.weight_a;
                milp.nodes[j].r = milp.nodes[j].r.min(candidate);
            }
        }
    }

    for j in num_int..num_total {
        for edge_id in milp.nodes[j].fanin.clone() {
            let edge = &milp.edges[edge_id];
            if greater(milp.nodes[j].r, milp.nodes[edge.from].r + edge.weight_a) {
                return false;
            }
        }
    }

    for edge in &mut milp.edges {
        edge.weight_b = edge.weight_a + milp.nodes[edge.from].r - milp.nodes[edge.to].r;
        if edge.kind == ConstraintKind::Real && edge.weight_b < -EPSILON {
            return false;
        }
    }

    for _ in 0..num_int {
        for i in 0..num_int {
            for edge_id in milp.nodes[i].fanin.clone() {
                let edge = &milp.edges[edge_id];
                let candidate = (milp.nodes[edge.from].y + edge.weight_b).floor();
                milp.nodes[edge.to].y = milp.nodes[edge.to].y.min(candidate);
            }
        }

        let mut ordered: Vec<_> = (0..num_total)
            .map(|i| (((SCALE * milp.nodes[i].y).floor() as i64), i))
            .collect();
        ordered.sort_unstable();
        for (_, i) in ordered {
            for j in num_int..num_total {
                for edge_id in milp.nodes[j].fanin.clone() {
                    let edge = &milp.edges[edge_id];
                    if edge.from == i {
                        let candidate = milp.nodes[i].y + edge.weight_b;
                        milp.nodes[edge.to].y = milp.nodes[edge.to].y.min(candidate);
                    }
                }
            }
        }
    }

    for i in 0..num_int {
        for edge_id in milp.nodes[i].fanin.clone() {
            let edge = &milp.edges[edge_id];
            if greater(
                milp.nodes[edge.to].y,
                milp.nodes[edge.from].y + edge.weight_b,
            ) {
                return false;
            }
        }
    }

    milp.nodes[0].x = milp.nodes[0].y + milp.nodes[0].r;
    let host_x = milp.nodes[0].x;
    for i in 1..num_int {
        milp.nodes[i].x = milp.nodes[i].y + milp.nodes[i].r - host_x;
    }
    for i in num_int..num_total {
        milp.nodes[i].x = milp.nodes[i].y + milp.nodes[i].r - host_x;
    }

    true
}

fn apply_retiming(graph: &mut RetimeGraph, retiming: &[i32]) -> Result<(), RetimeMilpError> {
    for node_id in 0..graph.nodes.len() {
        let lag = retiming[node_id];
        if graph.nodes[node_id].kind != RetimeNodeKind::Internal || lag == 0 {
            continue;
        }
        for edge in &mut graph.edges {
            if edge.source == node_id {
                edge.weight -= lag;
            }
            if edge.sink == node_id {
                edge.weight += lag;
            }
        }
    }
    Ok(())
}

fn add_vertex_constraints(
    graph: &mut RetimeMilpGraph,
    node: Option<&RetimeNode>,
    to: usize,
    cycle_time: f64,
) {
    let node_delay = if to == 0 {
        -cycle_time
    } else {
        node.expect("internal node required for non-host vertex constraint")
            .final_delay
    };
    let num_int = graph.num_int;
    add_constraint(
        graph,
        to + num_int,
        to,
        -(node_delay / cycle_time),
        ConstraintKind::Integer,
    );
    add_constraint(graph, to, to + num_int, 1.0, ConstraintKind::Real);
}

fn add_edge_constraint(
    graph: &mut RetimeMilpGraph,
    from: usize,
    to: usize,
    edge: &RetimeEdge,
    retime_graph: &RetimeGraph,
    cycle_time: f64,
) {
    let node_delay = if to == 0 {
        -cycle_time
    } else {
        retime_graph.nodes[edge.sink].final_delay
    };
    let num_int = graph.num_int;
    add_constraint(
        graph,
        to,
        from,
        f64::from(edge.weight),
        ConstraintKind::Integer,
    );
    add_constraint(
        graph,
        to + num_int,
        from + num_int,
        f64::from(edge.weight) - node_delay / cycle_time,
        ConstraintKind::Real,
    );
}

fn add_user_time_constraints(
    milp: &mut RetimeMilpGraph,
    graph: &RetimeGraph,
    internal_indices: &HashMap<usize, usize>,
    node_id: usize,
    cycle_time: f64,
) -> Result<(), RetimeMilpError> {
    let node = &graph.nodes[node_id];
    if node.kind == RetimeNodeKind::Internal || node.user_time < RETIME_TEST_NOT_SET {
        return Ok(());
    }

    let num_int = milp.num_int;
    if node.kind == RetimeNodeKind::PrimaryInput {
        for edge in graph.edges.iter().filter(|edge| edge.source == node_id) {
            let index = node_to_index(internal_indices, graph, edge.sink)?;
            let sink_delay = graph.nodes[edge.sink].final_delay;
            add_constraint(
                milp,
                index + num_int,
                0,
                f64::from(edge.weight) - (node.user_time + sink_delay) / cycle_time,
                ConstraintKind::Integer,
            );
        }
    } else if node.kind == RetimeNodeKind::PrimaryOutput {
        for edge in graph.edges.iter().filter(|edge| edge.sink == node_id) {
            let index = node_to_index(internal_indices, graph, edge.source)?;
            let user_time = cycle_time + node.user_time;
            add_constraint(
                milp,
                0,
                index + num_int,
                f64::from(edge.weight) + user_time / cycle_time,
                ConstraintKind::Real,
            );
        }
    }

    Ok(())
}

fn add_constraint(
    graph: &mut RetimeMilpGraph,
    from: usize,
    to: usize,
    weight_a: f64,
    kind: ConstraintKind,
) {
    let edge_id = graph.edges.len();
    graph.edges.push(ConstraintEdge {
        from,
        to,
        weight_a,
        weight_b: 0.0,
        kind,
    });
    graph.nodes[to].fanin.push(edge_id);
}

fn node_to_index(
    internal_indices: &HashMap<usize, usize>,
    graph: &RetimeGraph,
    node_id: usize,
) -> Result<usize, RetimeMilpError> {
    let node = graph
        .nodes
        .get(node_id)
        .ok_or(RetimeMilpError::MissingNode { node_id })?;
    match node.kind {
        RetimeNodeKind::Internal => internal_indices
            .get(&node_id)
            .copied()
            .ok_or(RetimeMilpError::MissingNode { node_id }),
        RetimeNodeKind::PrimaryInput | RetimeNodeKind::PrimaryOutput => Ok(0),
        RetimeNodeKind::Ignore => {
            Err(RetimeMilpError::EdgeEndpointMustBeInternalOrIo { edge_id: node_id })
        }
    }
}

fn validate_problem(graph: &RetimeGraph, cycle_time: f64) -> Result<(), RetimeMilpError> {
    if !cycle_time.is_finite() || cycle_time <= 0.0 {
        return Err(RetimeMilpError::NonFiniteCycleTime { cycle_time });
    }
    if graph.nodes.is_empty() {
        return Err(RetimeMilpError::EmptyGraph);
    }
    for (edge_id, edge) in graph.edges.iter().enumerate() {
        let source = graph
            .nodes
            .get(edge.source)
            .ok_or(RetimeMilpError::MissingNode {
                node_id: edge.source,
            })?;
        let sink = graph
            .nodes
            .get(edge.sink)
            .ok_or(RetimeMilpError::MissingNode { node_id: edge.sink })?;
        if source.kind == RetimeNodeKind::Ignore || sink.kind == RetimeNodeKind::Ignore {
            return Err(RetimeMilpError::EdgeEndpointMustBeInternalOrIo { edge_id });
        }
    }
    Ok(())
}

fn greater(left: f64, right: f64) -> bool {
    left > right + EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_graph() -> RetimeGraph {
        let mut graph = RetimeGraph::new();
        let pi = graph.add_node(RetimeNode::new("a", RetimeNodeKind::PrimaryInput));
        let n1 = graph.add_node(RetimeNode::internal("n1", 2.0));
        let po = graph.add_node(RetimeNode::new("z", RetimeNodeKind::PrimaryOutput));
        graph.add_edge(pi, n1, 0).unwrap();
        graph.add_edge(n1, po, 0).unwrap();
        graph
    }

    #[test]
    fn builds_host_internal_and_real_constraint_nodes() {
        let graph = simple_graph();
        let milp = build_lies_constraint_graph(&graph, 5.0).unwrap();

        assert_eq!(milp.num_integer_nodes(), 2);
        assert_eq!(milp.num_total_nodes(), 4);
        assert_eq!(milp.num_constraints(), 8);
        assert!(milp.edges.iter().any(|edge| {
            edge.from == 3 && edge.to == 1 && edge.kind == ConstraintKind::Integer
        }));
        assert!(milp.edges.iter().any(|edge| {
            edge.from == 2 && edge.to == 0 && edge.kind == ConstraintKind::Integer
        }));
    }

    #[test]
    fn user_time_constraints_match_primary_input_and_output_cases() {
        let mut graph = simple_graph();
        graph.nodes[0].user_time = 1.5;
        graph.nodes[2].user_time = -0.5;

        let milp = build_lies_constraint_graph(&graph, 5.0).unwrap();

        assert!(milp.edges.iter().any(|edge| {
            edge.from == 3
                && edge.to == 0
                && edge.kind == ConstraintKind::Integer
                && (edge.weight_a + 0.7).abs() < EPSILON
        }));
        assert!(milp.edges.iter().any(|edge| {
            edge.from == 0
                && edge.to == 3
                && edge.kind == ConstraintKind::Real
                && (edge.weight_a - 0.9).abs() < EPSILON
        }));
    }

    #[test]
    fn feasible_solution_returns_and_applies_zero_lag_for_simple_graph() {
        let mut graph = simple_graph();

        let solution = retime_lies_routine(&mut graph, 5.0).unwrap();

        assert!(solution.feasible);
        assert_eq!(solution.retiming, vec![0, 0, 0]);
        assert_eq!(graph.edges[0].weight, 0);
        assert_eq!(graph.edges[1].weight, 0);
        assert_eq!(solution.arrivals[1], 5.0);
    }

    #[test]
    fn applies_nonzero_lag_like_retime_single_node() {
        let mut graph = RetimeGraph::new();
        let pi = graph.add_node(RetimeNode::new("a", RetimeNodeKind::PrimaryInput));
        let n1 = graph.add_node(RetimeNode::internal("n1", 1.0));
        let n2 = graph.add_node(RetimeNode::internal("n2", 1.0));
        let po = graph.add_node(RetimeNode::new("z", RetimeNodeKind::PrimaryOutput));
        graph.add_edge(pi, n1, 1).unwrap();
        graph.add_edge(n1, n2, 0).unwrap();
        graph.add_edge(n2, po, 1).unwrap();

        let solution = retime_lies_routine(&mut graph, 1.5).unwrap();

        assert!(solution.feasible);
        assert_eq!(solution.retiming[1], -1);
        assert_eq!(graph.edges[0].weight, 0);
        assert_eq!(graph.edges[1].weight, 1);
    }

    #[test]
    fn sis_bound_entry_reports_dependency_beads_and_sources() {
        let error = retime_lies_routine_for_sis_graph().unwrap_err();
        let RetimeMilpError::MissingNativeDependencies { dependencies, .. } = error else {
            panic!("expected missing native dependency error");
        };

        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.422"
                && dependency.source_file == "LogicSynthesis/sis/retime/re_util.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.485"
                && dependency.source_file == "LogicSynthesis/sis/st/st.c"
        }));
        assert!(
            format!("{}", retime_lies_routine_for_sis_graph().unwrap_err()).contains("requires")
        );
    }

    #[test]
    fn validation_rejects_bad_cycle_time_and_ignored_edges() {
        let graph = simple_graph();
        assert!(matches!(
            build_lies_constraint_graph(&graph, 0.0),
            Err(RetimeMilpError::NonFiniteCycleTime { .. })
        ));

        let mut graph = RetimeGraph::new();
        let ignored = graph.add_node(RetimeNode::new("dead", RetimeNodeKind::Ignore));
        let po = graph.add_node(RetimeNode::new("z", RetimeNodeKind::PrimaryOutput));
        graph.edges.push(RetimeEdge {
            source: ignored,
            sink: po,
            weight: 0,
        });

        assert!(matches!(
            build_lies_constraint_graph(&graph, 1.0),
            Err(RetimeMilpError::EdgeEndpointMustBeInternalOrIo { edge_id: 0 })
        ));
    }
}
