//! Native Rust model for `LogicSynthesis/sis/retime/re_minreg.c`.
//!
//! The C unit minimizes registers by building a retiming linear program,
//! solving it with `re_simplx`, and then applying the chosen retiming to
//! internal graph vertices. This Rust port implements the file-local graph
//! preparation and tableau construction with owned indices. The top-level
//! optimizer reports explicit missing dependency errors until the sibling
//! `re_computeWD.c` and `re_simplx.c` ports are available as native Rust APIs.

use std::error::Error;
use std::fmt;

pub const SCALE: f64 = 1000.0;
pub const RETIME_TEST_NOT_SET: f64 = -99_999.0;
pub const POS_LARGE: i32 = 10_000;
pub const NEG_LARGE: i32 = -10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EdgeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeType {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNode {
    pub id: NodeId,
    pub name: Option<String>,
    pub kind: RetimeNodeType,
    pub lp_index: Option<usize>,
    pub fanins: Vec<EdgeId>,
    pub fanouts: Vec<EdgeId>,
    pub final_delay: f64,
    pub final_area: f64,
    pub user_time: f64,
    pub scaled_delay: i32,
    pub scaled_user_time: i32,
}

impl RetimeNode {
    pub fn new(kind: RetimeNodeType, name: impl Into<Option<String>>) -> Self {
        Self {
            id: NodeId(usize::MAX),
            name: name.into(),
            kind,
            lp_index: None,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            final_delay: 0.0,
            final_area: 0.0,
            user_time: RETIME_TEST_NOT_SET,
            scaled_delay: 0,
            scaled_user_time: 0,
        }
    }

    pub fn with_delay(mut self, final_delay: f64) -> Self {
        self.final_delay = final_delay;
        self
    }

    pub fn with_user_time(mut self, user_time: f64) -> Self {
        self.user_time = user_time;
        self
    }

    fn is_variable_vertex(&self) -> bool {
        matches!(self.kind, RetimeNodeType::Internal | RetimeNodeType::Ignore)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeEdge {
    pub id: EdgeId,
    pub source: NodeId,
    pub sink: NodeId,
    pub sink_fanin_id: usize,
    pub weight: i32,
    pub breadth: f64,
    pub temp_breadth: f64,
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

    pub fn add_node(&mut self, mut node: RetimeNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.id = id;
        self.nodes.push(node);
        id
    }

    pub fn add_edge(
        &mut self,
        source: NodeId,
        sink: NodeId,
        sink_fanin_id: usize,
        weight: i32,
        breadth: f64,
    ) -> Result<EdgeId, RetimeMinRegError> {
        self.require_node(source)?;
        self.require_node(sink)?;

        let id = EdgeId(self.edges.len());
        self.edges.push(RetimeEdge {
            id,
            source,
            sink,
            sink_fanin_id,
            weight,
            breadth,
            temp_breadth: breadth,
        });
        self.nodes[source.0].fanouts.push(id);
        self.nodes[sink.0].fanins.push(id);
        Ok(id)
    }

    pub fn edge_is_ignored(&self, edge: EdgeId) -> Result<bool, RetimeMinRegError> {
        let edge = self
            .edges
            .get(edge.0)
            .ok_or(RetimeMinRegError::MissingEdge(edge))?;
        Ok(self.nodes[edge.source.0].kind == RetimeNodeType::Ignore
            || self.nodes[edge.sink.0].kind == RetimeNodeType::Ignore)
    }

    pub fn apply_internal_retiming(&mut self, retiming: &[i32]) -> Result<(), RetimeMinRegError> {
        if retiming.len() < self.nodes.len() {
            return Err(RetimeMinRegError::RetimingVectorTooSmall {
                expected: self.nodes.len(),
                actual: retiming.len(),
            });
        }

        for node_id in 0..self.nodes.len() {
            if self.nodes[node_id].kind != RetimeNodeType::Internal {
                continue;
            }

            let lag = retiming[node_id];
            if lag == 0 {
                continue;
            }

            let fanins = self.nodes[node_id].fanins.clone();
            for edge_id in fanins {
                if !self.edge_is_ignored(edge_id)? {
                    self.edges[edge_id.0].weight += lag;
                }
            }

            let fanouts = self.nodes[node_id].fanouts.clone();
            for edge_id in fanouts {
                if !self.edge_is_ignored(edge_id)? {
                    self.edges[edge_id.0].weight -= lag;
                }
            }
        }

        Ok(())
    }

    fn require_node(&self, node: NodeId) -> Result<(), RetimeMinRegError> {
        match self.nodes.get(node.0) {
            Some(existing) if existing.id == node => Ok(()),
            _ => Err(RetimeMinRegError::MissingNode(node)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WdEntry {
    pub w: i32,
    pub d: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LpTableau {
    /// C-compatible tableau layout. Row 0 and column 0 are intentionally unused.
    pub a: Vec<Vec<f64>>,
    pub m: usize,
    pub m1: usize,
    pub m2: usize,
    pub m3: usize,
    pub translation: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinRegisterSetup {
    pub lp_variable_count: usize,
    pub node_for_lp_index: Vec<NodeId>,
    pub tableau: LpTableau,
}

pub fn retime_min_register(
    _graph: &mut RetimeGraph,
    _cycle_time: f64,
    _retiming: &mut [i32],
) -> Result<bool, RetimeMinRegError> {
    Err(RetimeMinRegError::MissingNativePorts {
        operation: "retime_min_register",
    })
}

pub fn prepare_min_register_problem(
    graph: &mut RetimeGraph,
    cycle_time: f64,
    wd: &[Vec<WdEntry>],
) -> Result<MinRegisterSetup, RetimeMinRegError> {
    add_register_sharing_nodes(graph)?;
    let node_for_lp_index = assign_lp_indices_and_scale_delays(graph);
    adjust_user_delays(graph)?;
    let lp_variable_count = node_for_lp_index.len() + 2;
    validate_wd_dimensions(wd, lp_variable_count)?;
    let scaled_cycle_time = ceil_scaled(cycle_time);
    let tableau = setup_lp_tableau(graph, lp_variable_count, wd, scaled_cycle_time)?;

    Ok(MinRegisterSetup {
        lp_variable_count,
        node_for_lp_index,
        tableau,
    })
}

pub fn add_register_sharing_nodes(
    graph: &mut RetimeGraph,
) -> Result<Vec<NodeId>, RetimeMinRegError> {
    let multi_fanout_nodes: Vec<NodeId> = graph
        .nodes
        .iter()
        .filter(|node| node.fanouts.len() > 1)
        .map(|node| node.id)
        .collect();
    let mut added = Vec::new();

    for node_id in multi_fanout_nodes {
        let active_fanouts: Vec<EdgeId> = graph.nodes[node_id.0]
            .fanouts
            .iter()
            .copied()
            .filter(|edge| !graph.edge_is_ignored(*edge).unwrap_or(true))
            .collect();

        if active_fanouts.is_empty() {
            continue;
        }

        let num_fan = active_fanouts.len() as f64;
        let max_weight = active_fanouts
            .iter()
            .map(|edge| graph.edges[edge.0].weight)
            .max()
            .unwrap_or(NEG_LARGE);

        let dummy = graph.add_node(RetimeNode::new(RetimeNodeType::Ignore, None));
        for edge_id in active_fanouts {
            let sink = graph.edges[edge_id.0].sink;
            let sink_fanin_id = graph.nodes[dummy.0].fanins.len();
            let weight = max_weight - graph.edges[edge_id.0].weight;

            graph.edges[edge_id.0].temp_breadth = graph.edges[edge_id.0].breadth / num_fan;
            let new_edge = graph.add_edge(sink, dummy, sink_fanin_id, weight, 1.0)?;
            graph.edges[new_edge.0].temp_breadth = graph.edges[new_edge.0].breadth / num_fan;
        }

        added.push(dummy);
    }

    Ok(added)
}

pub fn assign_lp_indices_and_scale_delays(graph: &mut RetimeGraph) -> Vec<NodeId> {
    let mut node_for_lp_index = Vec::new();

    for node in &mut graph.nodes {
        if node.is_variable_vertex() {
            let index = node_for_lp_index.len();
            node.lp_index = Some(index);
            node_for_lp_index.push(node.id);
        } else if node.user_time > RETIME_TEST_NOT_SET {
            node.scaled_user_time = ceil_scaled(node.user_time);
        }
        node.scaled_delay = ceil_scaled(node.final_delay);
    }

    let host_source_index = node_for_lp_index.len();
    let host_sink_index = host_source_index + 1;
    for node in &mut graph.nodes {
        match node.kind {
            RetimeNodeType::PrimaryInput => node.lp_index = Some(host_source_index),
            RetimeNodeType::PrimaryOutput => node.lp_index = Some(host_sink_index),
            RetimeNodeType::Internal | RetimeNodeType::Ignore => {}
        }
    }

    node_for_lp_index
}

pub fn adjust_user_delays(graph: &mut RetimeGraph) -> Result<(), RetimeMinRegError> {
    let primary_outputs: Vec<NodeId> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == RetimeNodeType::PrimaryOutput)
        .map(|node| node.id)
        .collect();

    for output in primary_outputs {
        if graph.nodes[output.0].user_time <= RETIME_TEST_NOT_SET {
            continue;
        }

        let scaled_user_time = graph.nodes[output.0].scaled_user_time;
        let fanins = graph.nodes[output.0].fanins.clone();
        for edge_id in fanins {
            let source = graph
                .edges
                .get(edge_id.0)
                .ok_or(RetimeMinRegError::MissingEdge(edge_id))?
                .source;
            graph.nodes[source.0].scaled_delay -= scaled_user_time;
        }
    }

    Ok(())
}

pub fn setup_lp_tableau(
    graph: &RetimeGraph,
    n: usize,
    wd: &[Vec<WdEntry>],
    scaled_cycle_time: i32,
) -> Result<LpTableau, RetimeMinRegError> {
    validate_wd_dimensions(wd, n)?;

    let num_edges = graph.edges.len();
    let mut m1 = num_edges;
    let mut m2 = 0;
    let m3 = 2;

    for row in wd.iter().take(n) {
        for entry in row.iter().take(n) {
            if entry.w == POS_LARGE {
                continue;
            }
            if entry.d > scaled_cycle_time {
                if entry.w < 1 {
                    m2 += 1;
                } else {
                    m1 += 1;
                }
            }
        }
    }

    let m = m1 + m2 + m3;
    let mut a = vec![vec![0.0; n + 2]; m + 3];
    let mut cur_m1 = 2;
    let mut cur_m2 = m1 + 2;

    for edge in &graph.edges {
        let i = graph.nodes[edge.source.0]
            .lp_index
            .ok_or(RetimeMinRegError::MissingLpIndex(edge.source))?;
        let j = graph.nodes[edge.sink.0]
            .lp_index
            .ok_or(RetimeMinRegError::MissingLpIndex(edge.sink))?;

        a[cur_m1][i + 2] -= 1.0;
        a[cur_m1][j + 2] += 1.0;
        a[cur_m1][1] = f64::from(edge.weight);
        cur_m1 += 1;
    }

    for i in 0..n {
        for j in 0..n {
            let entry = wd[i][j];
            if entry.w == POS_LARGE || entry.d <= scaled_cycle_time {
                continue;
            }

            let t = entry.w - 1;
            if t < 0 {
                a[cur_m2][i + 2] += 1.0;
                a[cur_m2][j + 2] -= 1.0;
                a[cur_m2][1] -= f64::from(t);
                cur_m2 += 1;
            } else {
                a[cur_m1][i + 2] -= 1.0;
                a[cur_m1][j + 2] += 1.0;
                a[cur_m1][1] += f64::from(t);
                cur_m1 += 1;
            }
        }
    }

    let translation = num_edges as i32 * sum_of_edge_weights(graph);
    a[m][1] = f64::from(translation);
    a[m][n] = -1.0;
    a[m + 1][1] = f64::from(translation);
    a[m + 1][n + 1] = -1.0;

    for node in &graph.nodes {
        let i = node
            .lp_index
            .ok_or(RetimeMinRegError::MissingLpIndex(node.id))?;
        for edge_id in &node.fanouts {
            a[1][i + 2] += graph.edges[edge_id.0].temp_breadth;
        }
        for edge_id in &node.fanins {
            a[1][i + 2] -= graph.edges[edge_id.0].temp_breadth;
        }
    }

    Ok(LpTableau {
        a,
        m,
        m1,
        m2,
        m3,
        translation,
    })
}

fn validate_wd_dimensions(wd: &[Vec<WdEntry>], n: usize) -> Result<(), RetimeMinRegError> {
    if wd.len() != n || wd.iter().any(|row| row.len() != n) {
        return Err(RetimeMinRegError::InvalidWdDimensions {
            expected: n,
            rows: wd.len(),
        });
    }
    Ok(())
}

fn sum_of_edge_weights(graph: &RetimeGraph) -> i32 {
    graph.edges.iter().map(|edge| edge.weight).sum()
}

fn ceil_scaled(value: f64) -> i32 {
    (value * SCALE).ceil() as i32
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetimeMinRegError {
    MissingNativePorts { operation: &'static str },
    MissingNode(NodeId),
    MissingEdge(EdgeId),
    MissingLpIndex(NodeId),
    InvalidWdDimensions { expected: usize, rows: usize },
    RetimingVectorTooSmall { expected: usize, actual: usize },
}

impl fmt::Display for RetimeMinRegError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native prerequisite ports")
            }
            Self::MissingNode(node) => write!(f, "retime graph references missing node {}", node.0),
            Self::MissingEdge(edge) => write!(f, "retime graph references missing edge {}", edge.0),
            Self::MissingLpIndex(node) => write!(f, "retime node {} has no LP index", node.0),
            Self::InvalidWdDimensions { expected, rows } => write!(
                f,
                "WD matrix must be {expected}x{expected}, got {rows} rows"
            ),
            Self::RetimingVectorTooSmall { expected, actual } => write!(
                f,
                "retiming vector has {actual} entries but graph requires {expected}"
            ),
        }
    }
}

impl Error for RetimeMinRegError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> RetimeGraph {
        let mut graph = RetimeGraph::new();
        let pi = graph.add_node(
            RetimeNode::new(RetimeNodeType::PrimaryInput, Some("pi".to_owned()))
                .with_user_time(0.25),
        );
        let n1 = graph.add_node(
            RetimeNode::new(RetimeNodeType::Internal, Some("n1".to_owned())).with_delay(1.2),
        );
        let n2 = graph.add_node(
            RetimeNode::new(RetimeNodeType::Internal, Some("n2".to_owned())).with_delay(2.0),
        );
        let po = graph.add_node(
            RetimeNode::new(RetimeNodeType::PrimaryOutput, Some("po".to_owned()))
                .with_user_time(0.4),
        );

        graph.add_edge(pi, n1, 0, 1, 6.0).unwrap();
        graph.add_edge(n1, n2, 0, 2, 2.0).unwrap();
        graph.add_edge(n1, po, 0, 0, 4.0).unwrap();
        graph
    }

    fn wd(n: usize) -> Vec<Vec<WdEntry>> {
        let mut wd = vec![vec![WdEntry { w: POS_LARGE, d: 0 }; n]; n];
        for (i, row) in wd.iter_mut().enumerate() {
            row[i] = WdEntry { w: 0, d: 0 };
        }
        wd
    }

    #[test]
    fn sharing_nodes_match_c_multi_fanout_model() {
        let mut graph = sample_graph();

        let added = add_register_sharing_nodes(&mut graph).unwrap();

        assert_eq!(added, vec![NodeId(4)]);
        assert_eq!(graph.nodes[4].kind, RetimeNodeType::Ignore);
        assert_eq!(graph.nodes[4].final_delay, 0.0);
        assert_eq!(graph.edges[1].temp_breadth, 1.0);
        assert_eq!(graph.edges[2].temp_breadth, 2.0);
        assert_eq!(graph.edges[3].source, NodeId(2));
        assert_eq!(graph.edges[3].sink, NodeId(4));
        assert_eq!(graph.edges[3].weight, 0);
        assert_eq!(graph.edges[3].temp_breadth, 0.5);
        assert_eq!(graph.edges[4].source, NodeId(3));
        assert_eq!(graph.edges[4].weight, 2);
    }

    #[test]
    fn lp_indices_scale_delays_and_po_user_delay_adjustment_follow_c_rules() {
        let mut graph = sample_graph();
        add_register_sharing_nodes(&mut graph).unwrap();

        let mapping = assign_lp_indices_and_scale_delays(&mut graph);
        adjust_user_delays(&mut graph).unwrap();

        assert_eq!(mapping, vec![NodeId(1), NodeId(2), NodeId(4)]);
        assert_eq!(graph.nodes[1].lp_index, Some(0));
        assert_eq!(graph.nodes[2].lp_index, Some(1));
        assert_eq!(graph.nodes[4].lp_index, Some(2));
        assert_eq!(graph.nodes[0].lp_index, Some(3));
        assert_eq!(graph.nodes[3].lp_index, Some(4));
        assert_eq!(graph.nodes[0].scaled_user_time, 250);
        assert_eq!(graph.nodes[3].scaled_user_time, 400);
        assert_eq!(graph.nodes[1].scaled_delay, 800);
        assert_eq!(graph.nodes[2].scaled_delay, 2000);
    }

    #[test]
    fn tableau_counts_edges_wd_constraints_hosts_and_objective() {
        let mut graph = sample_graph();
        add_register_sharing_nodes(&mut graph).unwrap();
        assign_lp_indices_and_scale_delays(&mut graph);
        adjust_user_delays(&mut graph).unwrap();

        let n = 5;
        let mut wd = wd(n);
        wd[0][1] = WdEntry { w: 2, d: 3000 };
        wd[1][0] = WdEntry { w: 0, d: 3000 };

        let tableau = setup_lp_tableau(&graph, n, &wd, 1500).unwrap();

        assert_eq!(tableau.m1, 6);
        assert_eq!(tableau.m2, 1);
        assert_eq!(tableau.m3, 2);
        assert_eq!(tableau.m, 9);
        assert_eq!(tableau.translation, 25);
        assert_eq!(tableau.a[2][5], -1.0);
        assert_eq!(tableau.a[2][2], 1.0);
        assert_eq!(tableau.a[2][1], 1.0);
        assert_eq!(tableau.a[7][3], 1.0);
        assert_eq!(tableau.a[7][2], -1.0);
        assert_eq!(tableau.a[7][1], 1.0);
        assert_eq!(tableau.a[9][5], -1.0);
        assert_eq!(tableau.a[10][6], -1.0);
        assert_eq!(tableau.a[1][2], 1.0 + 2.0 - 6.0);
    }

    #[test]
    fn prepare_min_register_problem_combines_setup_steps() {
        let mut graph = sample_graph();
        let mut wd = wd(5);
        wd[0][1] = WdEntry { w: 2, d: 3000 };

        let setup = prepare_min_register_problem(&mut graph, 1.5, &wd).unwrap();

        assert_eq!(setup.lp_variable_count, 5);
        assert_eq!(
            setup.node_for_lp_index,
            vec![NodeId(1), NodeId(2), NodeId(4)]
        );
        assert_eq!(setup.tableau.m1, 6);
        assert_eq!(graph.nodes.len(), 5);
    }

    #[test]
    fn apply_internal_retiming_updates_only_internal_incident_edges() {
        let mut graph = sample_graph();

        graph.apply_internal_retiming(&[0, 2, -1, 99]).unwrap();

        assert_eq!(graph.edges[0].weight, 3);
        assert_eq!(graph.edges[1].weight, -1);
        assert_eq!(graph.edges[2].weight, -2);
    }
    #[test]
    fn invalid_wd_dimensions_are_rejected() {
        let mut graph = sample_graph();
        add_register_sharing_nodes(&mut graph).unwrap();
        assign_lp_indices_and_scale_delays(&mut graph);

        assert_eq!(
            setup_lp_tableau(&graph, 5, &wd(4), 1000),
            Err(RetimeMinRegError::InvalidWdDimensions {
                expected: 5,
                rows: 4
            })
        );
    }
}
