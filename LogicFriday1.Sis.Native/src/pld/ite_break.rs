//! Native Rust model for the SIS ITE/ACT break pass.
//!
//! This module ports the deterministic graph expansion behavior to owned Rust
//! data. Direct mutation of legacy SIS `network_t`/`node_t` storage is modeled
//! as safe node replacement in `BreakNetwork`; integration with the full SIS
//! graph remains a higher-level wiring concern.

use std::error::Error;
use std::fmt;

pub type IteBreakResult<T> = Result<T, IteBreakError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteBreakError {
    MissingNativePorts {
        operation: &'static str,
    },
    MissingNode(NodeId),
    MissingIteVertex(IteVertexId),
    MissingActVertex(ActVertexId),
    MissingChild {
        vertex: VertexRef,
        child: &'static str,
    },
    MissingCostSlot(NodeId),
    MissingGraph(NodeId),
    MissingCachedNode(VertexRef),
    InvalidPattern {
        vertex: VertexRef,
        pattern_num: i32,
    },
    ExpectedSimpleMappedNode(NodeId),
    ExpectedLiteral(IteVertexId),
}

impl fmt::Display for IteBreakError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
            Self::MissingNode(node) => write!(f, "missing node {}", node.0),
            Self::MissingIteVertex(vertex) => write!(f, "missing ITE vertex {}", vertex.0),
            Self::MissingActVertex(vertex) => write!(f, "missing ACT vertex {}", vertex.0),
            Self::MissingChild { vertex, child } => {
                write!(f, "{vertex} is missing its {child} child")
            }
            Self::MissingCostSlot(node) => write!(f, "node {} has no ACT/ITE cost slot", node.0),
            Self::MissingGraph(node) => write!(f, "node {} has no break graph", node.0),
            Self::MissingCachedNode(vertex) => {
                write!(
                    f,
                    "{vertex} was revisited without a cached multiple-fanout node"
                )
            }
            Self::InvalidPattern {
                vertex,
                pattern_num,
            } => write!(f, "{vertex} has unsupported break pattern {pattern_num}"),
            Self::ExpectedSimpleMappedNode(node) => {
                write!(
                    f,
                    "node {} must be a constant or buffer when no mapped network exists",
                    node.0
                )
            }
            Self::ExpectedLiteral(vertex) => write!(f, "ITE vertex {} is not a literal", vertex.0),
        }
    }
}

impl Error for IteBreakError {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IteVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexRef {
    Ite(IteVertexId),
    Act(ActVertexId),
}

impl fmt::Display for VertexRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ite(vertex) => write!(f, "ITE vertex {}", vertex.0),
            Self::Act(vertex) => write!(f, "ACT vertex {}", vertex.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapMethod {
    Old,
    New,
    WithIter,
    WithJustDecomp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BreakParams {
    pub map_method: MapMethod,
    pub use_or_patterns: bool,
}

impl Default for BreakParams {
    fn default() -> Self {
        Self {
            map_method: MapMethod::Old,
            use_or_patterns: true,
        }
    }
}

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

impl NodeFunction {
    fn is_simple(self) -> bool {
        matches!(self, Self::Zero | Self::One | Self::Buffer)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LogicExpr {
    Constant(bool),
    Literal {
        node: NodeId,
        phase: bool,
    },
    Mux {
        condition: Box<LogicExpr>,
        when_false: Box<LogicExpr>,
        when_true: Box<LogicExpr>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BreakNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub expression: LogicExpr,
    pub cost_slot: Option<BreakCostSlot>,
    pub freed: bool,
}

impl BreakNode {
    pub fn internal(id: usize, name: impl Into<String>, function: NodeFunction) -> Self {
        let node_id = NodeId(id);
        Self {
            id: node_id,
            name: name.into(),
            kind: NodeKind::Internal,
            function,
            expression: LogicExpr::Literal {
                node: node_id,
                phase: true,
            },
            cost_slot: None,
            freed: false,
        }
    }

    pub fn primary_input(id: usize, name: impl Into<String>) -> Self {
        let node_id = NodeId(id);
        Self {
            id: node_id,
            name: name.into(),
            kind: NodeKind::PrimaryInput,
            function: NodeFunction::Buffer,
            expression: LogicExpr::Literal {
                node: node_id,
                phase: true,
            },
            cost_slot: None,
            freed: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BreakCostSlot {
    pub cost: i32,
    pub ite: Option<IteGraph>,
    pub ite_root: Option<IteVertexId>,
    pub act: Option<ActGraph>,
    pub act_root: Option<ActVertexId>,
    pub mapped_network: Option<Box<BreakNetwork>>,
}

impl BreakCostSlot {
    pub fn new(cost: i32) -> Self {
        Self {
            cost,
            ite: None,
            ite_root: None,
            act: None,
            act_root: None,
            mapped_network: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BreakNetwork {
    nodes: Vec<BreakNode>,
    pub freed: bool,
}

impl BreakNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_dfs_order(nodes: impl IntoIterator<Item = BreakNode>) -> Self {
        Self {
            nodes: nodes.into_iter().collect(),
            freed: false,
        }
    }

    pub fn nodes(&self) -> &[BreakNode] {
        &self.nodes
    }

    pub fn node(&self, node: NodeId) -> IteBreakResult<&BreakNode> {
        self.nodes
            .iter()
            .find(|candidate| candidate.id == node)
            .ok_or(IteBreakError::MissingNode(node))
    }

    pub fn node_mut(&mut self, node: NodeId) -> IteBreakResult<&mut BreakNode> {
        self.nodes
            .iter_mut()
            .find(|candidate| candidate.id == node)
            .ok_or(IteBreakError::MissingNode(node))
    }

    pub fn add_node(&mut self, mut node: BreakNode) -> NodeId {
        let id = NodeId(self.nodes.iter().map(|item| item.id.0).max().unwrap_or(0) + 1);
        node.id = id;
        self.nodes.push(node);
        id
    }

    fn add_aux_mux(&mut self, expression: LogicExpr) -> NodeId {
        let id = NodeId(self.nodes.iter().map(|node| node.id.0).max().unwrap_or(0) + 1);
        self.nodes.push(BreakNode {
            id,
            name: format!("ite_break_aux_{}", id.0),
            kind: NodeKind::Internal,
            function: NodeFunction::Other,
            expression,
            cost_slot: None,
            freed: false,
        });
        id
    }

    fn replace_node_expression(
        &mut self,
        node: NodeId,
        expression: LogicExpr,
    ) -> IteBreakResult<()> {
        self.node_mut(node)?.expression = expression;
        Ok(())
    }

    fn free_node(&mut self, node: NodeId) -> IteBreakResult<()> {
        self.node_mut(node)?.freed = true;
        Ok(())
    }

    fn free(&mut self) {
        self.freed = true;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BreakNetworkReport {
    pub changed_nodes: Vec<NodeId>,
    pub or_pattern_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BreakNodeReport {
    pub changed: bool,
    pub added_nodes: usize,
    pub freed_multiple_fanout_nodes: usize,
    pub replaced_from_mapped_network: bool,
    pub or_pattern_count: usize,
}

pub fn sis_bound_operation_unavailable(operation: &'static str) -> IteBreakResult<()> {
    Err(IteBreakError::MissingNativePorts { operation })
}

pub fn act_ite_break_network(
    network: &mut BreakNetwork,
    params: &BreakParams,
) -> IteBreakResult<BreakNetworkReport> {
    let node_ids = network
        .nodes()
        .iter()
        .filter(|node| node.kind == NodeKind::Internal)
        .map(|node| node.id)
        .collect::<Vec<_>>();
    let mut changed_nodes = Vec::new();
    let mut or_pattern_count = 0;

    for node in node_ids {
        let report = act_ite_break_node(network, node, params)?;
        if report.changed {
            changed_nodes.push(node);
        }
        or_pattern_count += report.or_pattern_count;
    }

    Ok(BreakNetworkReport {
        changed_nodes,
        or_pattern_count,
    })
}

pub fn act_ite_break_node(
    network: &mut BreakNetwork,
    node: NodeId,
    params: &BreakParams,
) -> IteBreakResult<BreakNodeReport> {
    let (kind, function) = {
        let node_ref = network.node(node)?;
        (node_ref.kind, node_ref.function)
    };
    if kind != NodeKind::Internal {
        return Ok(BreakNodeReport::unchanged());
    }

    if matches!(
        params.map_method,
        MapMethod::WithIter | MapMethod::WithJustDecomp
    ) {
        return break_iter_mapped_node(network, node);
    }

    let slot = network
        .node_mut(node)?
        .cost_slot
        .take()
        .ok_or(IteBreakError::MissingCostSlot(node))?;

    if slot.cost <= 1 {
        network.node_mut(node)?.cost_slot = Some(slot);
        return Ok(BreakNodeReport::unchanged());
    }

    if let (Some(graph), Some(root)) = (slot.ite, slot.ite_root) {
        return break_ite_node(network, node, graph, root, slot.cost, params);
    }

    if let (Some(graph), Some(root)) = (slot.act, slot.act_root) {
        return break_act_node(network, node, graph, root, slot.cost, params);
    }

    let mut restored_slot = BreakCostSlot::new(slot.cost);
    restored_slot.mapped_network = slot.mapped_network;
    network.node_mut(node)?.cost_slot = Some(restored_slot);
    if function.is_simple() {
        Ok(BreakNodeReport::unchanged())
    } else {
        Err(IteBreakError::MissingGraph(node))
    }
}

fn break_iter_mapped_node(
    network: &mut BreakNetwork,
    node: NodeId,
) -> IteBreakResult<BreakNodeReport> {
    let slot = network
        .node_mut(node)?
        .cost_slot
        .as_mut()
        .ok_or(IteBreakError::MissingCostSlot(node))?;

    if let Some(mut mapped_network) = slot.mapped_network.take() {
        let replaced = slot.cost != 1;
        if replaced {
            let expression = mapped_network
                .nodes()
                .iter()
                .rev()
                .find(|mapped_node| mapped_node.kind == NodeKind::Internal)
                .map(|mapped_node| mapped_node.expression.clone())
                .unwrap_or(LogicExpr::Constant(false));
            network.replace_node_expression(node, expression)?;
        }
        mapped_network.free();
        return Ok(BreakNodeReport {
            changed: true,
            added_nodes: 0,
            freed_multiple_fanout_nodes: 0,
            replaced_from_mapped_network: replaced,
            or_pattern_count: 0,
        });
    }

    if network.node(node)?.function.is_simple() {
        Ok(BreakNodeReport::unchanged())
    } else {
        Err(IteBreakError::ExpectedSimpleMappedNode(node))
    }
}

fn break_ite_node(
    network: &mut BreakNetwork,
    node: NodeId,
    mut graph: IteGraph,
    root: IteVertexId,
    cost: i32,
    params: &BreakParams,
) -> IteBreakResult<BreakNodeReport> {
    let added_before = network.nodes().len();
    graph.reset_marks(root)?;
    let mark_value = graph.vertex(root)?.mark;
    let complement = complement_mark(mark_value);
    let mut context = IteExpandContext {
        present_root: VertexRef::Ite(root),
        complement_mark: complement,
        use_or_patterns: params.use_or_patterns,
        or_pattern_count: 0,
    };
    let expression = graph.expand(root, network, &mut context)?;
    let freed = graph.free_nodes_in_multiple_fo(root, network, complement_mark(complement))?;
    network.replace_node_expression(node, expression)?;
    network.node_mut(node)?.cost_slot = Some(BreakCostSlot::new(cost));
    Ok(BreakNodeReport {
        changed: true,
        added_nodes: network.nodes().len() - added_before,
        freed_multiple_fanout_nodes: freed,
        replaced_from_mapped_network: false,
        or_pattern_count: context.or_pattern_count,
    })
}

fn break_act_node(
    network: &mut BreakNetwork,
    node: NodeId,
    mut graph: ActGraph,
    root: ActVertexId,
    cost: i32,
    params: &BreakParams,
) -> IteBreakResult<BreakNodeReport> {
    let added_before = network.nodes().len();
    graph.set_mark(root)?;
    let mark_value = graph.vertex(root)?.mark;
    let complement = complement_mark(mark_value);
    let mut context = ActExpandContext {
        present_root: VertexRef::Act(root),
        complement_mark: complement,
        use_or_patterns: params.use_or_patterns,
        or_pattern_count: 0,
    };
    let expression = graph.expand(root, network, &mut context)?;
    graph.set_mark(root)?;
    let freed = graph.free_nodes_in_multiple_fo(root, network, complement)?;
    network.replace_node_expression(node, expression)?;
    network.node_mut(node)?.cost_slot = Some(BreakCostSlot::new(cost));
    Ok(BreakNodeReport {
        changed: true,
        added_nodes: network.nodes().len() - added_before,
        freed_multiple_fanout_nodes: freed,
        replaced_from_mapped_network: false,
        or_pattern_count: context.or_pattern_count,
    })
}

impl BreakNodeReport {
    fn unchanged() -> Self {
        Self {
            changed: false,
            added_nodes: 0,
            freed_multiple_fanout_nodes: 0,
            replaced_from_mapped_network: false,
            or_pattern_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IteValue {
    Zero,
    One,
    Literal,
    IfThenElse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteVertex {
    pub value: IteValue,
    pub if_child: Option<IteVertexId>,
    pub then_child: Option<IteVertexId>,
    pub else_child: Option<IteVertexId>,
    pub fanin: Option<NodeId>,
    pub node: Option<NodeId>,
    pub mark: i32,
    pub multiple_fo: bool,
    pub pattern_num: i32,
    pub phase: bool,
}

impl IteVertex {
    pub fn terminal(value: bool) -> Self {
        Self {
            value: if value { IteValue::One } else { IteValue::Zero },
            if_child: None,
            then_child: None,
            else_child: None,
            fanin: None,
            node: None,
            mark: 0,
            multiple_fo: false,
            pattern_num: 0,
            phase: true,
        }
    }

    pub fn literal(fanin: NodeId, phase: bool) -> Self {
        Self {
            value: IteValue::Literal,
            fanin: Some(fanin),
            phase,
            ..Self::terminal(false)
        }
    }

    pub fn ite(if_child: IteVertexId, then_child: IteVertexId, else_child: IteVertexId) -> Self {
        Self {
            value: IteValue::IfThenElse,
            if_child: Some(if_child),
            then_child: Some(then_child),
            else_child: Some(else_child),
            pattern_num: 0,
            ..Self::terminal(false)
        }
    }

    pub fn with_pattern(mut self, pattern_num: i32) -> Self {
        self.pattern_num = pattern_num;
        self
    }

    pub fn with_multiple_fanout_node(mut self, node: NodeId) -> Self {
        self.multiple_fo = true;
        self.node = Some(node);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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

    pub fn vertex(&self, vertex: IteVertexId) -> IteBreakResult<&IteVertex> {
        self.vertices
            .get(vertex.0)
            .ok_or(IteBreakError::MissingIteVertex(vertex))
    }

    pub fn vertex_mut(&mut self, vertex: IteVertexId) -> IteBreakResult<&mut IteVertex> {
        self.vertices
            .get_mut(vertex.0)
            .ok_or(IteBreakError::MissingIteVertex(vertex))
    }

    pub fn reset_marks(&mut self, root: IteVertexId) -> IteBreakResult<()> {
        let mut seen = Vec::new();
        self.collect_ite(root, &mut seen)?;
        for vertex in seen {
            self.vertex_mut(vertex)?.mark = 0;
        }
        Ok(())
    }

    fn expand(
        &mut self,
        vertex: IteVertexId,
        network: &mut BreakNetwork,
        context: &mut IteExpandContext,
    ) -> IteBreakResult<LogicExpr> {
        let snapshot = self.vertex(vertex)?.clone();
        if snapshot.value == IteValue::Zero {
            self.vertex_mut(vertex)?.mark = context.complement_mark;
            return Ok(LogicExpr::Constant(false));
        }
        if snapshot.value == IteValue::One {
            self.vertex_mut(vertex)?.mark = context.complement_mark;
            return Ok(LogicExpr::Constant(true));
        }
        if snapshot.mark == context.complement_mark {
            return snapshot
                .node
                .map(|node| LogicExpr::Literal { node, phase: true })
                .ok_or(IteBreakError::MissingCachedNode(VertexRef::Ite(vertex)));
        }

        if !context.use_or_patterns && snapshot.pattern_num >= 4 {
            return Err(IteBreakError::InvalidPattern {
                vertex: VertexRef::Ite(vertex),
                pattern_num: snapshot.pattern_num,
            });
        }

        self.vertex_mut(vertex)?.mark = context.complement_mark;

        if snapshot.value == IteValue::Literal {
            let expression = self.literal_expression(vertex, snapshot.phase)?;
            if snapshot.multiple_fo {
                let node = cache_expression_node(snapshot.node, network, expression.clone());
                self.vertex_mut(vertex)?.node = Some(node);
            }
            return Ok(expression);
        }

        if self.is_positive_input_mux(vertex)? {
            let if_child = self.required_child(vertex, "IF", snapshot.if_child)?;
            self.vertex_mut(if_child)?.mark = context.complement_mark;
            self.vertex_mut(self.required_child(vertex, "THEN", snapshot.then_child)?)?
                .mark = context.complement_mark;
            self.vertex_mut(self.required_child(vertex, "ELSE", snapshot.else_child)?)?
                .mark = context.complement_mark;
            let expression = self.literal_expression(if_child, true)?;
            if snapshot.multiple_fo {
                let node = cache_expression_node(snapshot.node, network, expression.clone());
                self.vertex_mut(vertex)?.node = Some(node);
            }
            return Ok(expression);
        }

        if snapshot.pattern_num > 3 {
            context.or_pattern_count += 1;
        }

        let expression = self.expand_pattern(vertex, network, context)?;
        let result = self.finish_mux_vertex(
            vertex,
            snapshot.multiple_fo,
            expression,
            network,
            context.present_root,
        );
        Ok(result)
    }

    fn expand_pattern(
        &mut self,
        vertex: IteVertexId,
        network: &mut BreakNetwork,
        context: &mut IteExpandContext,
    ) -> IteBreakResult<LogicExpr> {
        match self.vertex(vertex)?.pattern_num {
            0 => {
                let condition = self.expand_child(vertex, "IF", network, context)?;
                let when_true = self.expand_child(vertex, "THEN", network, context)?;
                let when_false = self.expand_child(vertex, "ELSE", network, context)?;
                Ok(mux(condition, when_false, when_true))
            }
            1 => {
                let then_child = self.required_branch(vertex, "THEN")?;
                self.vertex_mut(then_child)?.mark = context.complement_mark;
                let expanded_then = self.expand_embedded_ite(then_child, network, context)?;
                let condition = self.expand_child(vertex, "IF", network, context)?;
                let when_false = self.expand_child(vertex, "ELSE", network, context)?;
                Ok(mux(condition, when_false, expanded_then))
            }
            2 => {
                let else_child = self.required_branch(vertex, "ELSE")?;
                self.vertex_mut(else_child)?.mark = context.complement_mark;
                let expanded_else = self.expand_embedded_ite(else_child, network, context)?;
                let condition = self.expand_child(vertex, "IF", network, context)?;
                let when_true = self.expand_child(vertex, "THEN", network, context)?;
                Ok(mux(condition, expanded_else, when_true))
            }
            3 => {
                let then_child = self.required_branch(vertex, "THEN")?;
                let else_child = self.required_branch(vertex, "ELSE")?;
                self.vertex_mut(then_child)?.mark = context.complement_mark;
                self.vertex_mut(else_child)?.mark = context.complement_mark;
                let expanded_then = self.expand_embedded_ite(then_child, network, context)?;
                let expanded_else = self.expand_embedded_ite(else_child, network, context)?;
                let condition = self.expand_child(vertex, "IF", network, context)?;
                Ok(mux(condition, expanded_else, expanded_then))
            }
            4 => {
                let if_child = self.required_branch(vertex, "IF")?;
                self.vertex_mut(if_child)?.mark = context.complement_mark;
                let condition = self.expand_embedded_ite(if_child, network, context)?;
                let when_true = self.expand_child(vertex, "THEN", network, context)?;
                let when_false = self.expand_child(vertex, "ELSE", network, context)?;
                Ok(mux(condition, when_false, when_true))
            }
            5 => {
                let if_child = self.required_branch(vertex, "IF")?;
                let then_child = self.required_branch(vertex, "THEN")?;
                self.vertex_mut(if_child)?.mark = context.complement_mark;
                self.vertex_mut(then_child)?.mark = context.complement_mark;
                let condition = self.expand_embedded_ite(if_child, network, context)?;
                let when_true = self.expand_embedded_ite(then_child, network, context)?;
                let when_false = self.expand_child(vertex, "ELSE", network, context)?;
                Ok(mux(condition, when_false, when_true))
            }
            6 => {
                let if_child = self.required_branch(vertex, "IF")?;
                let else_child = self.required_branch(vertex, "ELSE")?;
                self.vertex_mut(if_child)?.mark = context.complement_mark;
                self.vertex_mut(else_child)?.mark = context.complement_mark;
                let condition = self.expand_embedded_ite(if_child, network, context)?;
                let when_true = self.expand_child(vertex, "THEN", network, context)?;
                let when_false = self.expand_embedded_ite(else_child, network, context)?;
                Ok(mux(condition, when_false, when_true))
            }
            7 => {
                let if_child = self.required_branch(vertex, "IF")?;
                let then_child = self.required_branch(vertex, "THEN")?;
                let else_child = self.required_branch(vertex, "ELSE")?;
                self.vertex_mut(if_child)?.mark = context.complement_mark;
                self.vertex_mut(then_child)?.mark = context.complement_mark;
                self.vertex_mut(else_child)?.mark = context.complement_mark;
                let condition = self.expand_embedded_ite(if_child, network, context)?;
                let when_true = self.expand_embedded_ite(then_child, network, context)?;
                let when_false = self.expand_embedded_ite(else_child, network, context)?;
                Ok(mux(condition, when_false, when_true))
            }
            8 => {
                let else_child = self.required_branch(vertex, "ELSE")?;
                self.vertex_mut(else_child)?.mark = context.complement_mark;
                let condition = self.expand_child(vertex, "IF", network, context)?;
                let when_true = self.expand_child(vertex, "THEN", network, context)?;
                let else_condition = self.expand_grandchild(else_child, "IF", network, context)?;
                let else_when_false =
                    self.expand_grandchild(else_child, "ELSE", network, context)?;
                let when_false = mux(else_condition, else_when_false, when_true.clone());
                Ok(mux(condition, when_false, when_true))
            }
            9 => {
                let else_child = self.required_branch(vertex, "ELSE")?;
                let else_else = self.required_branch(else_child, "ELSE")?;
                self.vertex_mut(else_child)?.mark = context.complement_mark;
                self.vertex_mut(else_else)?.mark = context.complement_mark;
                let condition = self.expand_child(vertex, "IF", network, context)?;
                let when_true = self.expand_child(vertex, "THEN", network, context)?;
                let nested_else = self.expand_embedded_ite(else_else, network, context)?;
                let else_condition = self.expand_grandchild(else_child, "IF", network, context)?;
                let when_false = mux(else_condition, nested_else, when_true.clone());
                Ok(mux(condition, when_false, when_true))
            }
            pattern_num => Err(IteBreakError::InvalidPattern {
                vertex: VertexRef::Ite(vertex),
                pattern_num,
            }),
        }
    }

    fn expand_child(
        &mut self,
        vertex: IteVertexId,
        child: &'static str,
        network: &mut BreakNetwork,
        context: &mut IteExpandContext,
    ) -> IteBreakResult<LogicExpr> {
        let child = self.required_branch(vertex, child)?;
        self.expand(child, network, context)
    }

    fn expand_grandchild(
        &mut self,
        vertex: IteVertexId,
        child: &'static str,
        network: &mut BreakNetwork,
        context: &mut IteExpandContext,
    ) -> IteBreakResult<LogicExpr> {
        let child = self.required_branch(vertex, child)?;
        self.expand(child, network, context)
    }

    fn expand_embedded_ite(
        &mut self,
        vertex: IteVertexId,
        network: &mut BreakNetwork,
        context: &mut IteExpandContext,
    ) -> IteBreakResult<LogicExpr> {
        let condition = self.expand_child(vertex, "IF", network, context)?;
        let when_true = self.expand_child(vertex, "THEN", network, context)?;
        let when_false = self.expand_child(vertex, "ELSE", network, context)?;
        Ok(mux(condition, when_false, when_true))
    }

    fn finish_mux_vertex(
        &mut self,
        vertex: IteVertexId,
        multiple_fo: bool,
        expression: LogicExpr,
        network: &mut BreakNetwork,
        present_root: VertexRef,
    ) -> LogicExpr {
        if VertexRef::Ite(vertex) == present_root {
            return expression;
        }
        let node = network.add_aux_mux(expression);
        let literal = LogicExpr::Literal { node, phase: true };
        if multiple_fo {
            self.vertices[vertex.0].node = Some(node);
        }
        literal
    }

    fn literal_expression(&self, vertex: IteVertexId, phase: bool) -> IteBreakResult<LogicExpr> {
        let fanin = self
            .vertex(vertex)?
            .fanin
            .ok_or(IteBreakError::ExpectedLiteral(vertex))?;
        Ok(LogicExpr::Literal { node: fanin, phase })
    }

    fn is_positive_input_mux(&self, vertex: IteVertexId) -> IteBreakResult<bool> {
        let snapshot = self.vertex(vertex)?;
        if snapshot.value != IteValue::IfThenElse {
            return Ok(false);
        }
        let if_child = self.required_child(vertex, "IF", snapshot.if_child)?;
        let then_child = self.required_child(vertex, "THEN", snapshot.then_child)?;
        let else_child = self.required_child(vertex, "ELSE", snapshot.else_child)?;
        let if_vertex = self.vertex(if_child)?;
        Ok(if_vertex.value == IteValue::Literal
            && if_vertex.phase
            && self.vertex(then_child)?.value == IteValue::One
            && self.vertex(else_child)?.value == IteValue::Zero)
    }

    fn required_branch(
        &self,
        vertex: IteVertexId,
        child: &'static str,
    ) -> IteBreakResult<IteVertexId> {
        let snapshot = self.vertex(vertex)?;
        let id = match child {
            "IF" => snapshot.if_child,
            "THEN" => snapshot.then_child,
            "ELSE" => snapshot.else_child,
            _ => None,
        };
        self.required_child(vertex, child, id)
    }

    fn required_child(
        &self,
        vertex: IteVertexId,
        child: &'static str,
        id: Option<IteVertexId>,
    ) -> IteBreakResult<IteVertexId> {
        id.ok_or(IteBreakError::MissingChild {
            vertex: VertexRef::Ite(vertex),
            child,
        })
    }

    fn collect_ite(&self, root: IteVertexId, seen: &mut Vec<IteVertexId>) -> IteBreakResult<()> {
        if seen.contains(&root) {
            return Ok(());
        }
        seen.push(root);
        let snapshot = self.vertex(root)?;
        if snapshot.value == IteValue::IfThenElse {
            for child in [snapshot.if_child, snapshot.then_child, snapshot.else_child]
                .into_iter()
                .flatten()
            {
                self.collect_ite(child, seen)?;
            }
        }
        Ok(())
    }

    fn free_nodes_in_multiple_fo(
        &mut self,
        root: IteVertexId,
        network: &mut BreakNetwork,
        complement_mark: i32,
    ) -> IteBreakResult<usize> {
        let mut freed = 0;
        self.free_multiple_fo_from(root, network, complement_mark, &mut freed)?;
        Ok(freed)
    }

    fn free_multiple_fo_from(
        &mut self,
        vertex: IteVertexId,
        network: &mut BreakNetwork,
        complement_mark: i32,
        freed: &mut usize,
    ) -> IteBreakResult<()> {
        let snapshot = self.vertex(vertex)?.clone();
        if snapshot.mark == complement_mark {
            return Ok(());
        }
        self.vertex_mut(vertex)?.mark = complement_mark;
        if snapshot.value == IteValue::Zero || snapshot.value == IteValue::One {
            return Ok(());
        }
        if snapshot.multiple_fo {
            if let Some(node) = snapshot.node {
                network.free_node(node)?;
                *freed += 1;
            }
        }
        if snapshot.value == IteValue::IfThenElse {
            for child in [snapshot.if_child, snapshot.then_child, snapshot.else_child]
                .into_iter()
                .flatten()
            {
                self.free_multiple_fo_from(child, network, complement_mark, freed)?;
            }
        }
        Ok(())
    }
}

struct IteExpandContext {
    present_root: VertexRef,
    complement_mark: i32,
    use_or_patterns: bool,
    or_pattern_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActVertex {
    pub value: i32,
    pub low: Option<ActVertexId>,
    pub high: Option<ActVertexId>,
    pub fanin: Option<NodeId>,
    pub node: Option<NodeId>,
    pub mark: i32,
    pub multiple_fo: bool,
    pub pattern_num: i32,
}

impl ActVertex {
    pub fn terminal(value: bool) -> Self {
        Self {
            value: if value { 1 } else { 0 },
            low: None,
            high: None,
            fanin: None,
            node: None,
            mark: 0,
            multiple_fo: false,
            pattern_num: 0,
        }
    }

    pub fn literal(fanin: NodeId) -> Self {
        Self {
            value: 2,
            fanin: Some(fanin),
            ..Self::terminal(false)
        }
    }

    pub fn branch(fanin: NodeId, low: ActVertexId, high: ActVertexId) -> Self {
        Self {
            value: 3,
            low: Some(low),
            high: Some(high),
            fanin: Some(fanin),
            pattern_num: 0,
            ..Self::terminal(false)
        }
    }

    pub fn with_pattern(mut self, pattern_num: i32) -> Self {
        self.pattern_num = pattern_num;
        self
    }

    pub fn with_multiple_fanout_node(mut self, node: NodeId) -> Self {
        self.multiple_fo = true;
        self.node = Some(node);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActGraph {
    vertices: Vec<ActVertex>,
}

impl ActGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vertex(&mut self, vertex: ActVertex) -> ActVertexId {
        let id = ActVertexId(self.vertices.len());
        self.vertices.push(vertex);
        id
    }

    pub fn vertex(&self, vertex: ActVertexId) -> IteBreakResult<&ActVertex> {
        self.vertices
            .get(vertex.0)
            .ok_or(IteBreakError::MissingActVertex(vertex))
    }

    pub fn vertex_mut(&mut self, vertex: ActVertexId) -> IteBreakResult<&mut ActVertex> {
        self.vertices
            .get_mut(vertex.0)
            .ok_or(IteBreakError::MissingActVertex(vertex))
    }

    pub fn set_mark(&mut self, root: ActVertexId) -> IteBreakResult<()> {
        let mut seen = Vec::new();
        self.collect_act(root, &mut seen)?;
        for vertex in seen {
            self.vertex_mut(vertex)?.mark = 0;
        }
        Ok(())
    }

    fn expand(
        &mut self,
        vertex: ActVertexId,
        network: &mut BreakNetwork,
        context: &mut ActExpandContext,
    ) -> IteBreakResult<LogicExpr> {
        let snapshot = self.vertex(vertex)?.clone();
        if snapshot.value == 0 {
            self.vertex_mut(vertex)?.mark = context.complement_mark;
            return Ok(LogicExpr::Constant(false));
        }
        if snapshot.value == 1 {
            self.vertex_mut(vertex)?.mark = context.complement_mark;
            return Ok(LogicExpr::Constant(true));
        }
        if snapshot.mark == context.complement_mark {
            return snapshot
                .node
                .map(|node| LogicExpr::Literal { node, phase: true })
                .ok_or(IteBreakError::MissingCachedNode(VertexRef::Act(vertex)));
        }
        if !context.use_or_patterns && snapshot.pattern_num >= 4 {
            return Err(IteBreakError::InvalidPattern {
                vertex: VertexRef::Act(vertex),
                pattern_num: snapshot.pattern_num,
            });
        }

        self.vertex_mut(vertex)?.mark = context.complement_mark;

        if self.is_positive_input_branch(vertex)? {
            let expression = self.literal_expression(vertex)?;
            if snapshot.multiple_fo {
                let node = cache_expression_node(snapshot.node, network, expression.clone());
                self.vertex_mut(vertex)?.node = Some(node);
            }
            return Ok(expression);
        }

        if snapshot.pattern_num > 3 {
            context.or_pattern_count += 1;
        }

        let expression = self.expand_pattern(vertex, network, context)?;
        let node = if context.present_root == VertexRef::Act(vertex) {
            return Ok(expression);
        } else {
            network.add_aux_mux(expression)
        };
        if snapshot.multiple_fo {
            self.vertex_mut(vertex)?.node = Some(node);
        }
        Ok(LogicExpr::Literal { node, phase: true })
    }

    fn expand_pattern(
        &mut self,
        vertex: ActVertexId,
        network: &mut BreakNetwork,
        context: &mut ActExpandContext,
    ) -> IteBreakResult<LogicExpr> {
        match self.vertex(vertex)?.pattern_num {
            0 => {
                let when_false = self.expand_child(vertex, "low", network, context)?;
                let when_true = self.expand_child(vertex, "high", network, context)?;
                let condition = self.literal_expression(vertex)?;
                Ok(mux(condition, when_false, when_true))
            }
            1 => {
                let low = self.required_branch(vertex, "low")?;
                self.vertex_mut(low)?.mark = context.complement_mark;
                let when_false = self.expand_embedded_act(low, network, context)?;
                let condition = self.literal_expression(vertex)?;
                let when_true = self.expand_child(vertex, "high", network, context)?;
                Ok(mux(condition, when_false, when_true))
            }
            2 => {
                let high = self.required_branch(vertex, "high")?;
                self.vertex_mut(high)?.mark = context.complement_mark;
                let when_true = self.expand_embedded_act(high, network, context)?;
                let condition = self.literal_expression(vertex)?;
                let when_false = self.expand_child(vertex, "low", network, context)?;
                Ok(mux(condition, when_false, when_true))
            }
            3 => {
                let low = self.required_branch(vertex, "low")?;
                let high = self.required_branch(vertex, "high")?;
                self.vertex_mut(low)?.mark = context.complement_mark;
                self.vertex_mut(high)?.mark = context.complement_mark;
                let when_false = self.expand_embedded_act(low, network, context)?;
                let when_true = self.expand_embedded_act(high, network, context)?;
                let condition = self.literal_expression(vertex)?;
                Ok(mux(condition, when_false, when_true))
            }
            4 | 7 => {
                let low = self.required_branch(vertex, "low")?;
                self.vertex_mut(low)?.mark = context.complement_mark;
                let low_low = self.required_branch(low, "low")?;
                self.vertex_mut(low_low)?.mark = context.complement_mark;
                let nested = self.expand_embedded_act(low_low, network, context)?;
                let shared_high = self.expand_child(vertex, "high", network, context)?;
                let low_condition = self.literal_expression(low)?;
                let when_false = mux(low_condition, nested, shared_high.clone());
                let condition = self.literal_expression(vertex)?;
                Ok(mux(condition, when_false, shared_high))
            }
            5 | 6 => {
                let high = self.required_branch(vertex, "high")?;
                self.vertex_mut(high)?.mark = context.complement_mark;
                let high_low = self.required_branch(high, "low")?;
                self.vertex_mut(high_low)?.mark = context.complement_mark;
                let nested = self.expand_embedded_act(high_low, network, context)?;
                let high_condition = self.literal_expression(high)?;
                let high_high = self.expand_child(high, "high", network, context)?;
                let when_true = mux(high_condition, nested, high_high);
                let when_false = self.expand_child(vertex, "low", network, context)?;
                let condition = self.literal_expression(vertex)?;
                Ok(mux(condition, when_false, when_true))
            }
            pattern_num => Err(IteBreakError::InvalidPattern {
                vertex: VertexRef::Act(vertex),
                pattern_num,
            }),
        }
    }

    fn expand_child(
        &mut self,
        vertex: ActVertexId,
        child: &'static str,
        network: &mut BreakNetwork,
        context: &mut ActExpandContext,
    ) -> IteBreakResult<LogicExpr> {
        let child = self.required_branch(vertex, child)?;
        self.expand(child, network, context)
    }

    fn expand_embedded_act(
        &mut self,
        vertex: ActVertexId,
        network: &mut BreakNetwork,
        context: &mut ActExpandContext,
    ) -> IteBreakResult<LogicExpr> {
        let condition = self.literal_expression(vertex)?;
        let when_false = self.expand_child(vertex, "low", network, context)?;
        let when_true = self.expand_child(vertex, "high", network, context)?;
        Ok(mux(condition, when_false, when_true))
    }

    fn is_positive_input_branch(&self, vertex: ActVertexId) -> IteBreakResult<bool> {
        let snapshot = self.vertex(vertex)?;
        if snapshot.value <= 1 {
            return Ok(false);
        }
        let low = self.required_branch(vertex, "low")?;
        let high = self.required_branch(vertex, "high")?;
        Ok(self.vertex(low)?.value == 0 && self.vertex(high)?.value == 1)
    }

    fn literal_expression(&self, vertex: ActVertexId) -> IteBreakResult<LogicExpr> {
        let fanin = self
            .vertex(vertex)?
            .fanin
            .ok_or(IteBreakError::MissingCachedNode(VertexRef::Act(vertex)))?;
        Ok(LogicExpr::Literal {
            node: fanin,
            phase: true,
        })
    }

    fn required_branch(
        &self,
        vertex: ActVertexId,
        child: &'static str,
    ) -> IteBreakResult<ActVertexId> {
        let snapshot = self.vertex(vertex)?;
        let id = match child {
            "low" => snapshot.low,
            "high" => snapshot.high,
            _ => None,
        };
        id.ok_or(IteBreakError::MissingChild {
            vertex: VertexRef::Act(vertex),
            child,
        })
    }

    fn collect_act(&self, root: ActVertexId, seen: &mut Vec<ActVertexId>) -> IteBreakResult<()> {
        if seen.contains(&root) {
            return Ok(());
        }
        seen.push(root);
        let snapshot = self.vertex(root)?;
        if snapshot.value > 1 {
            for child in [snapshot.low, snapshot.high].into_iter().flatten() {
                self.collect_act(child, seen)?;
            }
        }
        Ok(())
    }

    fn free_nodes_in_multiple_fo(
        &mut self,
        root: ActVertexId,
        network: &mut BreakNetwork,
        complement_mark: i32,
    ) -> IteBreakResult<usize> {
        let mut freed = 0;
        self.free_multiple_fo_from(root, network, complement_mark, &mut freed)?;
        Ok(freed)
    }

    fn free_multiple_fo_from(
        &mut self,
        vertex: ActVertexId,
        network: &mut BreakNetwork,
        complement_mark: i32,
        freed: &mut usize,
    ) -> IteBreakResult<()> {
        let snapshot = self.vertex(vertex)?.clone();
        if snapshot.mark == complement_mark {
            return Ok(());
        }
        self.vertex_mut(vertex)?.mark = complement_mark;
        if snapshot.value <= 1 {
            return Ok(());
        }
        if snapshot.multiple_fo {
            if let Some(node) = snapshot.node {
                network.free_node(node)?;
                *freed += 1;
            }
        }
        for child in [snapshot.low, snapshot.high].into_iter().flatten() {
            self.free_multiple_fo_from(child, network, complement_mark, freed)?;
        }
        Ok(())
    }
}

struct ActExpandContext {
    present_root: VertexRef,
    complement_mark: i32,
    use_or_patterns: bool,
    or_pattern_count: usize,
}

fn mux(condition: LogicExpr, when_false: LogicExpr, when_true: LogicExpr) -> LogicExpr {
    LogicExpr::Mux {
        condition: Box::new(condition),
        when_false: Box::new(when_false),
        when_true: Box::new(when_true),
    }
}

fn complement_mark(mark: i32) -> i32 {
    if mark == 0 { 1 } else { 0 }
}

fn cache_expression_node(
    existing_node: Option<NodeId>,
    network: &mut BreakNetwork,
    expression: LogicExpr,
) -> NodeId {
    existing_node.unwrap_or_else(|| network.add_aux_mux(expression))
}

pub fn pld_replace_node_by_network_blocked() -> IteBreakResult<()> {
    sis_bound_operation_unavailable("pld_replace_node_by_network")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network_with_node(node: BreakNode) -> BreakNetwork {
        BreakNetwork::from_dfs_order([node])
    }

    fn node_with_slot(slot: BreakCostSlot) -> BreakNode {
        let mut node = BreakNode::internal(10, "f", NodeFunction::Other);
        node.cost_slot = Some(slot);
        node
    }

    #[test]
    fn network_break_skips_primary_inputs_and_changes_internal_nodes() {
        let mut graph = IteGraph::new();
        let literal = graph.add_vertex(IteVertex::literal(NodeId(1), false));
        let mut slot = BreakCostSlot::new(2);
        slot.ite = Some(graph);
        slot.ite_root = Some(literal);

        let mut network =
            BreakNetwork::from_dfs_order([BreakNode::primary_input(1, "a"), node_with_slot(slot)]);

        let report = act_ite_break_network(&mut network, &BreakParams::default()).unwrap();

        assert_eq!(report.changed_nodes, vec![NodeId(10)]);
        assert_eq!(
            network.node(NodeId(10)).unwrap().expression,
            LogicExpr::Literal {
                node: NodeId(1),
                phase: false,
            }
        );
    }

    #[test]
    fn low_cost_node_is_left_unchanged() {
        let mut node = BreakNode::internal(10, "f", NodeFunction::Other);
        node.cost_slot = Some(BreakCostSlot::new(1));
        let mut network = network_with_node(node);

        let report = act_ite_break_node(&mut network, NodeId(10), &BreakParams::default()).unwrap();

        assert!(!report.changed);
        assert!(network.node(NodeId(10)).unwrap().cost_slot.is_some());
    }

    #[test]
    fn iter_mapped_node_replaces_from_owned_network_when_cost_is_not_one() {
        let mut mapped = BreakNetwork::from_dfs_order([BreakNode {
            expression: LogicExpr::Constant(true),
            ..BreakNode::internal(22, "mapped", NodeFunction::Other)
        }]);
        mapped.freed = false;
        let mut slot = BreakCostSlot::new(2);
        slot.mapped_network = Some(Box::new(mapped));
        let mut network = network_with_node(node_with_slot(slot));

        let report = act_ite_break_node(
            &mut network,
            NodeId(10),
            &BreakParams {
                map_method: MapMethod::WithIter,
                use_or_patterns: true,
            },
        )
        .unwrap();

        assert!(report.changed);
        assert!(report.replaced_from_mapped_network);
        assert_eq!(
            network.node(NodeId(10)).unwrap().expression,
            LogicExpr::Constant(true)
        );
    }

    #[test]
    fn ite_positive_variable_mux_reduces_to_literal() {
        let mut graph = IteGraph::new();
        let x = graph.add_vertex(IteVertex::literal(NodeId(1), true));
        let one = graph.add_vertex(IteVertex::terminal(true));
        let zero = graph.add_vertex(IteVertex::terminal(false));
        let root = graph.add_vertex(IteVertex::ite(x, one, zero));
        let mut slot = BreakCostSlot::new(2);
        slot.ite = Some(graph);
        slot.ite_root = Some(root);
        let mut network = network_with_node(node_with_slot(slot));

        act_ite_break_node(&mut network, NodeId(10), &BreakParams::default()).unwrap();

        assert_eq!(
            network.node(NodeId(10)).unwrap().expression,
            LogicExpr::Literal {
                node: NodeId(1),
                phase: true,
            }
        );
    }

    #[test]
    fn ite_pattern_zero_adds_aux_node_for_nested_mux() {
        let mut graph = IteGraph::new();
        let x = graph.add_vertex(IteVertex::literal(NodeId(1), true));
        let root_x = graph.add_vertex(IteVertex::literal(NodeId(1), true));
        let y = graph.add_vertex(IteVertex::literal(NodeId(2), false));
        let zero = graph.add_vertex(IteVertex::terminal(false));
        let child = graph.add_vertex(IteVertex::ite(x, y, zero));
        let root = graph.add_vertex(IteVertex::ite(root_x, child, zero));
        let mut slot = BreakCostSlot::new(2);
        slot.ite = Some(graph);
        slot.ite_root = Some(root);
        let mut network = network_with_node(node_with_slot(slot));

        let report = act_ite_break_node(&mut network, NodeId(10), &BreakParams::default()).unwrap();

        assert!(report.changed);
        assert_eq!(report.added_nodes, 1);
        assert!(matches!(
            network.node(NodeId(10)).unwrap().expression,
            LogicExpr::Mux { .. }
        ));
    }

    #[test]
    fn ite_or_pattern_is_rejected_when_or_patterns_are_disabled() {
        let mut graph = IteGraph::new();
        let x = graph.add_vertex(IteVertex::literal(NodeId(1), true));
        let one = graph.add_vertex(IteVertex::terminal(true));
        let zero = graph.add_vertex(IteVertex::terminal(false));
        let root = graph.add_vertex(IteVertex::ite(x, one, zero).with_pattern(8));
        let mut slot = BreakCostSlot::new(2);
        slot.ite = Some(graph);
        slot.ite_root = Some(root);
        let mut network = network_with_node(node_with_slot(slot));

        let error = act_ite_break_node(
            &mut network,
            NodeId(10),
            &BreakParams {
                map_method: MapMethod::Old,
                use_or_patterns: false,
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            IteBreakError::InvalidPattern { pattern_num: 8, .. }
        ));
    }

    #[test]
    fn revisiting_non_multiple_fanout_vertex_reports_diagnostic() {
        let mut graph = IteGraph::new();
        let x = graph.add_vertex(IteVertex::literal(NodeId(1), false));
        let root = graph.add_vertex(IteVertex::ite(x, x, x));
        let mut slot = BreakCostSlot::new(2);
        slot.ite = Some(graph);
        slot.ite_root = Some(root);
        let mut network = network_with_node(node_with_slot(slot));

        let error =
            act_ite_break_node(&mut network, NodeId(10), &BreakParams::default()).unwrap_err();

        assert!(matches!(
            error,
            IteBreakError::MissingCachedNode(VertexRef::Ite(_))
        ));
    }

    #[test]
    fn act_positive_branch_reduces_to_vertex_literal() {
        let mut graph = ActGraph::new();
        let zero = graph.add_vertex(ActVertex::terminal(false));
        let one = graph.add_vertex(ActVertex::terminal(true));
        let root = graph.add_vertex(ActVertex::branch(NodeId(1), zero, one));
        let mut slot = BreakCostSlot::new(2);
        slot.act = Some(graph);
        slot.act_root = Some(root);
        let mut network = network_with_node(node_with_slot(slot));

        act_ite_break_node(&mut network, NodeId(10), &BreakParams::default()).unwrap();

        assert_eq!(
            network.node(NodeId(10)).unwrap().expression,
            LogicExpr::Literal {
                node: NodeId(1),
                phase: true,
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("ite_break.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
    }
}
