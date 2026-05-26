//! Native Rust model for `LogicSynthesis/sis/pld/ite_mux_net.c`.
//!
//! The C file is a small orchestration pass over a SIS network plus one
//! recursive ITE initializer. This port keeps those deterministic decisions in
//! owned Rust structures. Direct calls into SIS `network_t`, `node_t`,
//! `array_t`, factor, ITE-building, ITE-breaking, and cleanup routines remain
//! explicit dependency errors until the prerequisite native ports are present.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        reason: "network_dfs returns the C node traversal as array_t",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.297",
        source_file: "LogicSynthesis/sis/network/dfs.c",
        reason: "act_ite_mux_network walks internal nodes in network_dfs order",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node type, node_function, and node_num_literal drive the mux pass",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.198",
        source_file: "LogicSynthesis/sis/factor/ft_value.c",
        reason: "factor_num_literal selects factored-form ITE construction",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.365",
        source_file: "LogicSynthesis/sis/pld/ite_factor.c",
        reason: "act_ite_create_from_factored_form builds the factored-form ITE",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.372",
        source_file: "LogicSynthesis/sis/pld/ite_new_urp.c",
        reason: "act_ite_intermediate_new_make_ite builds the SOP-derived ITE",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.363",
        source_file: "LogicSynthesis/sis/pld/ite_break.c",
        reason: "act_ite_break_node maps eligible initialized ITE nodes",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.375",
        source_file: "LogicSynthesis/sis/pld/ite_util.c",
        reason: "act_free_ite_network releases temporary ACT/ITE/match state",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "the C initializer uses an st_table as a pointer visited set",
    },
];

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn sis_bound_operation_unavailable(operation: &'static str) -> Result<(), IteMuxNetError> {
    Err(IteMuxNetError::MissingNativePorts {
        operation,
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum IteMuxNetError {
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    MissingIteVertex(IteVertexId),
    MissingChild {
        parent: IteVertexId,
        branch: IteBranch,
    },
}

impl fmt::Display for IteMuxNetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} native SIS prerequisite ports",
                dependencies.len()
            ),
            Self::MissingIteVertex(vertex) => write!(f, "missing ITE vertex {}", vertex.0),
            Self::MissingChild { parent, branch } => {
                write!(f, "ITE vertex {} is missing its {branch:?} child", parent.0)
            }
        }
    }
}

impl Error for IteMuxNetError {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Buffer,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IteBuildAction {
    FactoredForm,
    SopIntermediate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IteMuxNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub sop_literals: usize,
    pub factored_literals: usize,
    pub ite_root: Option<IteVertexId>,
    pub slot_cost: i32,
}

impl IteMuxNode {
    pub fn new(id: usize, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind,
            function: NodeFunction::Other,
            sop_literals: 0,
            factored_literals: 0,
            ite_root: None,
            slot_cost: 0,
        }
    }

    pub fn with_function(mut self, function: NodeFunction) -> Self {
        self.function = function;
        self
    }

    pub fn with_literals(mut self, sop_literals: usize, factored_literals: usize) -> Self {
        self.sop_literals = sop_literals;
        self.factored_literals = factored_literals;
        self
    }

    pub fn with_ite_root(mut self, root: IteVertexId) -> Self {
        self.ite_root = Some(root);
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IteMuxNetwork {
    nodes: Vec<IteMuxNode>,
    pub ite_network_freed: bool,
}

impl IteMuxNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_dfs_order(nodes: impl IntoIterator<Item = IteMuxNode>) -> Self {
        Self {
            nodes: nodes.into_iter().collect(),
            ite_network_freed: false,
        }
    }

    pub fn nodes(&self) -> &[IteMuxNode] {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut [IteMuxNode] {
        &mut self.nodes
    }

    pub fn push_node(&mut self, node: IteMuxNode) {
        self.nodes.push(node);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IteMuxNodeReport {
    pub node: NodeId,
    pub action: IteBuildAction,
    pub initialized_ite: bool,
    pub broke_node: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteMuxNetworkReport {
    pub nodes: Vec<IteMuxNodeReport>,
    pub freed_ite_network: bool,
}

pub fn select_ite_build_action(
    sop_literals: usize,
    factored_literals: usize,
    fac_to_sop_ratio: f32,
) -> IteBuildAction {
    if sop_literals > 4 && (factored_literals as f32) < fac_to_sop_ratio * (sop_literals as f32) {
        IteBuildAction::FactoredForm
    } else {
        IteBuildAction::SopIntermediate
    }
}

pub fn should_break_after_initialization(function: NodeFunction) -> bool {
    !matches!(
        function,
        NodeFunction::Zero | NodeFunction::One | NodeFunction::Buffer
    )
}

pub fn act_ite_mux_network_model(
    network: &mut IteMuxNetwork,
    graph: &mut IteGraph,
    fac_to_sop_ratio: f32,
) -> Result<IteMuxNetworkReport, IteMuxNetError> {
    let mut reports = Vec::new();

    for node in network.nodes_mut() {
        if node.kind != NodeKind::Internal {
            continue;
        }

        let action =
            select_ite_build_action(node.sop_literals, node.factored_literals, fac_to_sop_ratio);

        if let Some(root) = node.ite_root {
            act_ite_initialize_ite_area_pattern0(graph, root)?;
        }

        let broke_node = should_break_after_initialization(node.function);
        if broke_node {
            node.slot_cost = 2;
        }

        reports.push(IteMuxNodeReport {
            node: node.id,
            action,
            initialized_ite: node.ite_root.is_some(),
            broke_node,
        });
    }

    network.ite_network_freed = true;
    Ok(IteMuxNetworkReport {
        nodes: reports,
        freed_ite_network: true,
    })
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IteVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IteValue {
    Zero,
    One,
    Literal,
    IfThenElse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IteBranch {
    If,
    Then,
    Else,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IteVertex {
    pub value: IteValue,
    pub if_child: Option<IteVertexId>,
    pub then_child: Option<IteVertexId>,
    pub else_child: Option<IteVertexId>,
    pub pattern_num: i32,
    pub cost: i32,
    pub mark: i32,
    pub mapped: bool,
    pub arrival_time: f64,
    pub multiple_fo: usize,
    pub multiple_fo_for_mapping: usize,
}

impl IteVertex {
    pub fn new(value: IteValue) -> Self {
        Self {
            value,
            if_child: None,
            then_child: None,
            else_child: None,
            pattern_num: -1,
            cost: -1,
            mark: -1,
            mapped: false,
            arrival_time: -1.0,
            multiple_fo: 0,
            multiple_fo_for_mapping: 0,
        }
    }

    pub fn with_children(
        mut self,
        if_child: IteVertexId,
        then_child: IteVertexId,
        else_child: IteVertexId,
    ) -> Self {
        self.if_child = Some(if_child);
        self.then_child = Some(then_child);
        self.else_child = Some(else_child);
        self
    }

    fn reset_area_pattern0(&mut self) {
        self.pattern_num = 0;
        self.cost = 0;
        self.mark = 0;
        self.mapped = true;
        self.arrival_time = 0.0;
        self.multiple_fo = 0;
        self.multiple_fo_for_mapping = 0;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IteGraph {
    vertices: Vec<IteVertex>,
}

impl IteGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vertex(&mut self, vertex: IteVertex) -> IteVertexId {
        let id = IteVertexId(self.vertices.len());
        self.vertices.push(vertex);
        id
    }

    pub fn vertex(&self, vertex: IteVertexId) -> Result<&IteVertex, IteMuxNetError> {
        self.vertices
            .get(vertex.0)
            .ok_or(IteMuxNetError::MissingIteVertex(vertex))
    }

    pub fn vertex_mut(&mut self, vertex: IteVertexId) -> Result<&mut IteVertex, IteMuxNetError> {
        self.vertices
            .get_mut(vertex.0)
            .ok_or(IteMuxNetError::MissingIteVertex(vertex))
    }
}

pub fn act_ite_initialize_ite_area_pattern0(
    graph: &mut IteGraph,
    root: IteVertexId,
) -> Result<(), IteMuxNetError> {
    let mut visited = HashSet::new();
    act_ite_insert_table_area_pattern0(graph, root, &mut visited)
}

pub fn act_ite_insert_table_area_pattern0(
    graph: &mut IteGraph,
    vertex: IteVertexId,
    visited: &mut HashSet<IteVertexId>,
) -> Result<(), IteMuxNetError> {
    if !visited.insert(vertex) {
        graph.vertex_mut(vertex)?.multiple_fo += 1;
        return Ok(());
    }

    let value = {
        let vertex_ref = graph.vertex_mut(vertex)?;
        vertex_ref.reset_area_pattern0();
        vertex_ref.value
    };

    if value != IteValue::IfThenElse {
        return Ok(());
    }

    let (if_child, then_child, else_child) = {
        let vertex_ref = graph.vertex(vertex)?;
        (
            vertex_ref.if_child.ok_or(IteMuxNetError::MissingChild {
                parent: vertex,
                branch: IteBranch::If,
            })?,
            vertex_ref.then_child.ok_or(IteMuxNetError::MissingChild {
                parent: vertex,
                branch: IteBranch::Then,
            })?,
            vertex_ref.else_child.ok_or(IteMuxNetError::MissingChild {
                parent: vertex,
                branch: IteBranch::Else,
            })?,
        )
    };

    act_ite_insert_table_area_pattern0(graph, if_child, visited)?;
    act_ite_insert_table_area_pattern0(graph, then_child, visited)?;
    act_ite_insert_table_area_pattern0(graph, else_child, visited)
}

pub fn act_ite_mux_network_blocked() -> Result<(), IteMuxNetError> {
    sis_bound_operation_unavailable("act_ite_mux_network")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_action_matches_c_ratio_gate() {
        assert_eq!(
            select_ite_build_action(5, 3, 0.75),
            IteBuildAction::FactoredForm
        );
        assert_eq!(
            select_ite_build_action(4, 1, 0.75),
            IteBuildAction::SopIntermediate
        );
        assert_eq!(
            select_ite_build_action(8, 6, 0.75),
            IteBuildAction::SopIntermediate
        );
    }

    #[test]
    fn initializer_resets_vertices_and_counts_repeated_fanout() {
        let mut graph = IteGraph::new();
        let shared = graph.add_vertex(IteVertex {
            multiple_fo: 9,
            mapped: false,
            ..IteVertex::new(IteValue::Literal)
        });
        let one = graph.add_vertex(IteVertex::new(IteValue::One));
        let root = graph
            .add_vertex(IteVertex::new(IteValue::IfThenElse).with_children(shared, one, shared));

        act_ite_initialize_ite_area_pattern0(&mut graph, root).unwrap();

        let root_vertex = graph.vertex(root).unwrap();
        assert_eq!(root_vertex.pattern_num, 0);
        assert_eq!(root_vertex.cost, 0);
        assert!(root_vertex.mapped);
        assert_eq!(root_vertex.arrival_time, 0.0);

        let shared_vertex = graph.vertex(shared).unwrap();
        assert_eq!(shared_vertex.multiple_fo, 1);
        assert_eq!(shared_vertex.multiple_fo_for_mapping, 0);
        assert!(shared_vertex.mapped);
    }

    #[test]
    fn mux_network_model_skips_non_internal_and_sets_break_cost() {
        let mut graph = IteGraph::new();
        let terminal = graph.add_vertex(IteVertex::new(IteValue::One));
        let root = graph.add_vertex(
            IteVertex::new(IteValue::IfThenElse).with_children(terminal, terminal, terminal),
        );
        let mut network = IteMuxNetwork::from_dfs_order([
            IteMuxNode::new(1, "pi", NodeKind::PrimaryInput),
            IteMuxNode::new(2, "logic", NodeKind::Internal)
                .with_literals(6, 2)
                .with_ite_root(root),
            IteMuxNode::new(3, "buf", NodeKind::Internal)
                .with_function(NodeFunction::Buffer)
                .with_literals(2, 2)
                .with_ite_root(terminal),
        ]);

        let report = act_ite_mux_network_model(&mut network, &mut graph, 0.5).unwrap();

        assert_eq!(report.nodes.len(), 2);
        assert_eq!(report.nodes[0].action, IteBuildAction::FactoredForm);
        assert!(report.nodes[0].broke_node);
        assert_eq!(network.nodes()[1].slot_cost, 2);
        assert_eq!(report.nodes[1].action, IteBuildAction::SopIntermediate);
        assert!(!report.nodes[1].broke_node);
        assert_eq!(network.nodes()[2].slot_cost, 0);
        assert!(network.ite_network_freed);
    }

    #[test]
    fn constants_and_buffers_are_not_broken() {
        assert!(!should_break_after_initialization(NodeFunction::Zero));
        assert!(!should_break_after_initialization(NodeFunction::One));
        assert!(!should_break_after_initialization(NodeFunction::Buffer));
        assert!(should_break_after_initialization(NodeFunction::Other));
    }

    #[test]
    fn missing_dependency_error_lists_blocking_beads_and_sources() {
        let Err(IteMuxNetError::MissingNativePorts {
            operation,
            dependencies,
        }) = act_ite_mux_network_blocked()
        else {
            panic!("expected missing dependency error");
        };

        assert_eq!(operation, "act_ite_mux_network");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.365"
                && dependency.source_file == "LogicSynthesis/sis/pld/ite_factor.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.372"
                && dependency.source_file == "LogicSynthesis/sis/pld/ite_new_urp.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.363"
                && dependency.source_file == "LogicSynthesis/sis/pld/ite_break.c"
        }));
    }
}
