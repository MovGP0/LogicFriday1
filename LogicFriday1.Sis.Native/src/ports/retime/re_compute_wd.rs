//! Native Rust model for `LogicSynthesis/sis/retime/re_computeWD.c`.
//!
//! The C unit computes the retiming W/D matrix from an already prepared
//! retime graph. This port keeps that computation native and owned: callers
//! provide nodes with LP indices plus edges with weights, and receive the
//! all-pairs `(w, d)` matrix used by `re_minreg.c`. Direct SIS table/debug
//! plumbing remains an explicit integration dependency.

use std::error::Error;
use std::fmt;

pub const RETIME_TEST_NOT_SET: f64 = -50_000.0;
pub const POS_LARGE: i32 = 10_000;
pub const IS_POS_LARGE: i32 = 5_000;

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
    pub scaled_delay: i32,
    pub user_time: f64,
    pub scaled_user_time: i32,
}

impl RetimeNode {
    pub fn new(kind: RetimeNodeType, name: impl Into<Option<String>>) -> Self {
        Self {
            id: NodeId(usize::MAX),
            name: name.into(),
            kind,
            lp_index: None,
            scaled_delay: 0,
            user_time: RETIME_TEST_NOT_SET,
            scaled_user_time: 0,
        }
    }

    pub fn with_lp_index(mut self, lp_index: usize) -> Self {
        self.lp_index = Some(lp_index);
        self
    }

    pub fn with_scaled_delay(mut self, scaled_delay: i32) -> Self {
        self.scaled_delay = scaled_delay;
        self
    }

    pub fn with_user_time(mut self, user_time: f64, scaled_user_time: i32) -> Self {
        self.user_time = user_time;
        self.scaled_user_time = scaled_user_time;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeEdge {
    pub id: EdgeId,
    pub source: NodeId,
    pub sink: NodeId,
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
        weight: i32,
    ) -> Result<EdgeId, ComputeWdError> {
        self.require_node(source)?;
        self.require_node(sink)?;

        let id = EdgeId(self.edges.len());
        self.edges.push(RetimeEdge {
            id,
            source,
            sink,
            weight,
        });
        Ok(id)
    }

    fn require_node(&self, node: NodeId) -> Result<(), ComputeWdError> {
        match self.nodes.get(node.0) {
            Some(existing) if existing.id == node => Ok(()),
            _ => Err(ComputeWdError::MissingNode(node)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WdEntry {
    pub w: i32,
    pub d: i32,
}

pub type WdMatrix = Vec<Vec<WdEntry>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_INTEGRATION_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.418",
        source_file: "LogicSynthesis/sis/retime/re_minreg.c",
        reason: "native callers need the min-register setup to pass prepared LP node-index data",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.422",
        source_file: "LogicSynthesis/sis/retime/re_util.c",
        reason: "SIS re_graph construction and node metadata are ported separately",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.423",
        source_file: "LogicSynthesis/sis/retime/retime_util.c",
        reason: "legacy retime graph allocation/accessor helpers are outside this W/D computation",
    },
];

pub fn required_integration_dependencies() -> &'static [PortDependency] {
    REQUIRED_INTEGRATION_DEPENDENCIES
}

pub fn sis_re_compute_wd_blocked() -> Result<(), ComputeWdError> {
    Err(ComputeWdError::MissingIntegrationDependencies {
        operation: "SIS re_computeWD graph/table integration",
        dependencies: REQUIRED_INTEGRATION_DEPENDENCIES,
    })
}

pub fn compute_wd(
    graph: &RetimeGraph,
    n: usize,
    node_for_lp_index: &[NodeId],
) -> Result<WdMatrix, ComputeWdError> {
    validate_index_mapping(graph, n, node_for_lp_index)?;

    let mut wd = vec![vec![WdEntry { w: POS_LARGE, d: 0 }; n]; n];

    for edge in &graph.edges {
        let source = graph
            .nodes
            .get(edge.source.0)
            .ok_or(ComputeWdError::MissingNode(edge.source))?;
        let sink = graph
            .nodes
            .get(edge.sink.0)
            .ok_or(ComputeWdError::MissingNode(edge.sink))?;
        let i = source
            .lp_index
            .ok_or(ComputeWdError::MissingLpIndex(edge.source))?;
        let j = sink
            .lp_index
            .ok_or(ComputeWdError::MissingLpIndex(edge.sink))?;

        if i >= n || j >= n {
            return Err(ComputeWdError::LpIndexOutOfRange {
                node: if i >= n { edge.source } else { edge.sink },
                lp_index: if i >= n { i } else { j },
                n,
            });
        }

        wd[i][j].w = if wd[i][j].w != POS_LARGE {
            wd[i][j].w.min(edge.weight)
        } else {
            edge.weight
        };

        let mut offset = 0;
        if source.kind == RetimeNodeType::PrimaryInput && source.user_time > RETIME_TEST_NOT_SET {
            offset = source.scaled_user_time;
            wd[i][j].d = wd[i][j].d.min(offset);
        }
        let delay = -(source.scaled_delay + offset);
        wd[i][j].d = wd[i][j].d.min(delay);
    }

    for k in (0..n).rev() {
        for i in (0..n).rev() {
            for j in (0..n).rev() {
                if is_greater(wd[i][j], wd[i][k], wd[k][j]) {
                    wd[i][j] = sum(wd[i][k], wd[k][j]);
                }
            }
        }
    }

    for i in (0..n).rev() {
        for j in (0..n).rev() {
            let sink_delay = if j >= n - 2 {
                0
            } else {
                graph.nodes[node_for_lp_index[j].0].scaled_delay
            };
            wd[i][j].d = sink_delay - wd[i][j].d;
        }
    }

    Ok(wd)
}

pub fn format_wd_debug(wd: &[Vec<WdEntry>]) -> String {
    let mut output = String::from("Data on the WD values\n");
    for i in (0..wd.len()).rev() {
        output.push_str(&format!("{i}::"));
        for entry in wd[i].iter().rev() {
            if entry.w > IS_POS_LARGE {
                output.push_str(" INFIN");
            } else {
                output.push_str(&format!(" {}-{:<4}", entry.w, entry.d));
            }
        }
        output.push('\n');
    }
    output
}

fn validate_index_mapping(
    graph: &RetimeGraph,
    n: usize,
    node_for_lp_index: &[NodeId],
) -> Result<(), ComputeWdError> {
    if n < 2 {
        return Err(ComputeWdError::InvalidHostVertexCount { n });
    }
    if node_for_lp_index.len() != n - 2 {
        return Err(ComputeWdError::InvalidIndexMapping {
            expected: n - 2,
            actual: node_for_lp_index.len(),
        });
    }
    for (index, node) in node_for_lp_index.iter().copied().enumerate() {
        graph.require_node(node)?;
        if graph.nodes[node.0].lp_index != Some(index) {
            return Err(ComputeWdError::IndexMappingMismatch {
                index,
                node,
                node_lp_index: graph.nodes[node.0].lp_index,
            });
        }
    }
    Ok(())
}

fn is_greater(current: WdEntry, left: WdEntry, right: WdEntry) -> bool {
    let candidate_w = left.w + right.w;
    current.w > candidate_w || current.w == candidate_w && current.d > left.d + right.d
}

fn sum(left: WdEntry, right: WdEntry) -> WdEntry {
    WdEntry {
        w: left.w + right.w,
        d: left.d + right.d,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ComputeWdError {
    MissingIntegrationDependencies {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    MissingNode(NodeId),
    MissingLpIndex(NodeId),
    LpIndexOutOfRange {
        node: NodeId,
        lp_index: usize,
        n: usize,
    },
    InvalidHostVertexCount {
        n: usize,
    },
    InvalidIndexMapping {
        expected: usize,
        actual: usize,
    },
    IndexMappingMismatch {
        index: usize,
        node: NodeId,
        node_lp_index: Option<usize>,
    },
}

impl fmt::Display for ComputeWdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIntegrationDependencies {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS retime dependencies",
                dependencies.len()
            ),
            Self::MissingNode(node) => write!(f, "retime graph references missing node {}", node.0),
            Self::MissingLpIndex(node) => write!(f, "retime node {} has no LP index", node.0),
            Self::LpIndexOutOfRange { node, lp_index, n } => write!(
                f,
                "retime node {} has LP index {lp_index}, outside WD dimension {n}",
                node.0
            ),
            Self::InvalidHostVertexCount { n } => {
                write!(f, "WD dimension must include two host vertices, got {n}")
            }
            Self::InvalidIndexMapping { expected, actual } => write!(
                f,
                "LP index mapping must contain {expected} non-host nodes, got {actual}"
            ),
            Self::IndexMappingMismatch {
                index,
                node,
                node_lp_index,
            } => write!(
                f,
                "LP index mapping entry {index} points to node {} with LP index {node_lp_index:?}",
                node.0
            ),
        }
    }
}

impl Error for ComputeWdError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> (RetimeGraph, Vec<NodeId>) {
        let mut graph = RetimeGraph::new();
        let a = graph.add_node(
            RetimeNode::new(RetimeNodeType::Internal, Some("a".to_owned()))
                .with_lp_index(0)
                .with_scaled_delay(120),
        );
        let b = graph.add_node(
            RetimeNode::new(RetimeNodeType::Internal, Some("b".to_owned()))
                .with_lp_index(1)
                .with_scaled_delay(230),
        );
        let pi = graph.add_node(
            RetimeNode::new(RetimeNodeType::PrimaryInput, Some("pi".to_owned()))
                .with_lp_index(2)
                .with_scaled_delay(40)
                .with_user_time(0.8, 80),
        );
        let po = graph.add_node(
            RetimeNode::new(RetimeNodeType::PrimaryOutput, Some("po".to_owned())).with_lp_index(3),
        );

        graph.add_edge(a, b, 2).unwrap();
        graph.add_edge(b, po, 1).unwrap();
        graph.add_edge(pi, a, 0).unwrap();
        (graph, vec![a, b])
    }

    #[test]
    fn direct_edges_apply_min_weight_and_source_delay_rules() {
        let (mut graph, mapping) = sample_graph();
        let a = mapping[0];
        let b = mapping[1];
        graph.add_edge(a, b, 5).unwrap();
        graph.add_edge(a, b, 1).unwrap();

        let wd = compute_wd(&graph, 4, &mapping).unwrap();

        assert_eq!(wd[0][1], WdEntry { w: 1, d: 350 });
        assert_eq!(wd[2][0], WdEntry { w: 0, d: 240 });
        assert_eq!(wd[1][3], WdEntry { w: 1, d: 230 });
    }

    #[test]
    fn path_relaxation_prefers_lower_weight_then_lower_delay() {
        let (mut graph, mapping) = sample_graph();
        graph.add_edge(mapping[0], NodeId(3), 3).unwrap();

        let wd = compute_wd(&graph, 4, &mapping).unwrap();

        assert_eq!(wd[0][3], WdEntry { w: 3, d: 350 });
        assert_eq!(wd[2][3], WdEntry { w: 3, d: 470 });
    }

    #[test]
    fn equal_weight_paths_keep_more_negative_preconverted_delay() {
        let mut graph = RetimeGraph::new();
        let a = graph.add_node(
            RetimeNode::new(RetimeNodeType::Internal, Some("a".to_owned()))
                .with_lp_index(0)
                .with_scaled_delay(100),
        );
        let slow = graph.add_node(
            RetimeNode::new(RetimeNodeType::Internal, Some("slow".to_owned()))
                .with_lp_index(1)
                .with_scaled_delay(900),
        );
        let fast = graph.add_node(
            RetimeNode::new(RetimeNodeType::Internal, Some("fast".to_owned()))
                .with_lp_index(2)
                .with_scaled_delay(200),
        );
        let po = graph.add_node(
            RetimeNode::new(RetimeNodeType::PrimaryOutput, Some("po".to_owned())).with_lp_index(4),
        );
        graph.add_edge(a, slow, 1).unwrap();
        graph.add_edge(slow, po, 1).unwrap();
        graph.add_edge(a, fast, 1).unwrap();
        graph.add_edge(fast, po, 1).unwrap();

        let wd = compute_wd(&graph, 5, &[a, slow, fast]).unwrap();

        assert_eq!(wd[0][4], WdEntry { w: 2, d: 1000 });
    }

    #[test]
    fn host_vertices_have_zero_sink_delay() {
        let (graph, mapping) = sample_graph();

        let wd = compute_wd(&graph, 4, &mapping).unwrap();

        assert_eq!(wd[0][3], WdEntry { w: 3, d: 350 });
        assert_eq!(wd[1][2], WdEntry { w: POS_LARGE, d: 0 });
    }

    #[test]
    fn invalid_mapping_is_reported_before_computation() {
        let (graph, mapping) = sample_graph();

        assert_eq!(
            compute_wd(&graph, 5, &mapping),
            Err(ComputeWdError::InvalidIndexMapping {
                expected: 3,
                actual: 2
            })
        );
    }

    #[test]
    fn missing_integration_dependencies_report_beads_and_sources() {
        let error =
            sis_re_compute_wd_blocked().expect_err("SIS integration is intentionally gated");
        let ComputeWdError::MissingIntegrationDependencies {
            operation,
            dependencies,
        } = error
        else {
            panic!("expected dependency error");
        };

        assert_eq!(operation, "SIS re_computeWD graph/table integration");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.418"
                && dependency.source_file == "LogicSynthesis/sis/retime/re_minreg.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.422"
                && dependency.source_file == "LogicSynthesis/sis/retime/re_util.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.423"
                && dependency.source_file == "LogicSynthesis/sis/retime/retime_util.c"
        }));
    }

    #[test]
    fn debug_dump_matches_c_infinite_threshold_ordering() {
        let wd = vec![
            vec![WdEntry { w: 1, d: 20 }, WdEntry { w: POS_LARGE, d: 0 }],
            vec![WdEntry { w: 0, d: 0 }, WdEntry { w: 2, d: 30 }],
        ];

        let dump = format_wd_debug(&wd);

        assert!(dump.contains("1:: 2-30   0-0"));
        assert!(dump.contains("0:: INFIN 1-20"));
    }
}
