//! Native Rust fanout-tree model for `sis/map/fanout_tree.c`.
//!
//! SIS stores fanout trees as prefix-ordered `array_t` entries and then patches
//! child pointers into that array before checking required times, area, loads,
//! and fanout leaves. This module keeps the useful mapper data as an owned Rust
//! forest. It intentionally exposes no legacy per-file C ABI entry points.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use super::virtual_net::DelayTime;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FanoutTreeNodeId(usize);

impl FanoutTreeNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutTreeNodeKind {
    Sink,
    Buffer,
    Source,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutPolarity {
    Positive,
    Negative,
    Unknown,
}

impl FanoutPolarity {
    pub fn inverted(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
            Self::Unknown => Self::Unknown,
        }
    }

    pub fn propagate_through(self, gate: BufferPolarity) -> Self {
        match gate {
            BufferPolarity::NonInverting => self,
            BufferPolarity::Inverting => self.inverted(),
            BufferPolarity::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferPolarity {
    NonInverting,
    Inverting,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSink {
    pub name: String,
    pub pin: usize,
    pub polarity: FanoutPolarity,
    pub load: f64,
    pub required: DelayTime,
    pub saved_fanout_index: Option<usize>,
}

impl FanoutSink {
    pub fn new(
        name: impl Into<String>,
        pin: usize,
        polarity: FanoutPolarity,
        load: f64,
        required: DelayTime,
    ) -> Self {
        Self {
            name: name.into(),
            pin,
            polarity,
            load,
            required,
            saved_fanout_index: None,
        }
    }

    pub fn with_saved_fanout_index(mut self, fanout_index: usize) -> Self {
        self.saved_fanout_index = Some(fanout_index);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FanoutTreeNode {
    Sink(FanoutSink),
    Gate {
        kind: FanoutTreeNodeKind,
        gate_index: usize,
        arity: usize,
        children: Vec<FanoutTreeNodeId>,
        polarity: FanoutPolarity,
        load: f64,
        required: DelayTime,
        arrival: DelayTime,
        area: f64,
    },
}

impl FanoutTreeNode {
    pub fn sink(sink: FanoutSink) -> Self {
        Self::Sink(sink)
    }

    pub fn buffer(gate_index: usize, arity: usize) -> Self {
        Self::gate(FanoutTreeNodeKind::Buffer, gate_index, arity)
    }

    pub fn source(gate_index: usize, arity: usize) -> Self {
        Self::gate(FanoutTreeNodeKind::Source, gate_index, arity)
    }

    pub fn kind(&self) -> FanoutTreeNodeKind {
        match self {
            Self::Sink(_) => FanoutTreeNodeKind::Sink,
            Self::Gate { kind, .. } => *kind,
        }
    }

    pub fn children(&self) -> &[FanoutTreeNodeId] {
        match self {
            Self::Sink(_) => &[],
            Self::Gate { children, .. } => children,
        }
    }

    pub fn fanout_load(&self) -> f64 {
        match self {
            Self::Sink(sink) => sink.load,
            Self::Gate { load, .. } => *load,
        }
    }

    pub fn required(&self) -> DelayTime {
        match self {
            Self::Sink(sink) => sink.required,
            Self::Gate { required, .. } => *required,
        }
    }

    pub fn arrival(&self) -> DelayTime {
        match self {
            Self::Sink(_) => DelayTime::new(f64::INFINITY, f64::INFINITY),
            Self::Gate { arrival, .. } => *arrival,
        }
    }

    fn gate(kind: FanoutTreeNodeKind, gate_index: usize, arity: usize) -> Self {
        Self::Gate {
            kind,
            gate_index,
            arity,
            children: Vec::new(),
            polarity: FanoutPolarity::Unknown,
            load: 0.0,
            required: DelayTime::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            arrival: DelayTime::new(f64::INFINITY, f64::INFINITY),
            area: f64::INFINITY,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutTreeForest {
    nodes: Vec<FanoutTreeNode>,
    roots: Vec<FanoutTreeNodeId>,
}

impl FanoutTreeForest {
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            roots: Vec::new(),
        }
    }

    pub fn from_prefix(nodes: Vec<FanoutTreeNode>) -> Result<Self, FanoutTreeError> {
        let mut forest = Self {
            nodes,
            roots: Vec::new(),
        };
        forest.add_edges_from_prefix()?;
        forest.validate()?;
        Ok(forest)
    }

    pub fn insert_sink(&mut self, sink: FanoutSink) -> FanoutTreeNodeId {
        self.push(FanoutTreeNode::sink(sink))
    }

    pub fn insert_gate(
        &mut self,
        gate_index: usize,
        arity: usize,
    ) -> Result<FanoutTreeNodeId, FanoutTreeError> {
        if arity == 0 {
            return Err(FanoutTreeError::InvalidArity {
                node: FanoutTreeNodeId(self.nodes.len()),
                arity,
            });
        }

        Ok(self.push(FanoutTreeNode::buffer(gate_index, arity)))
    }

    pub fn add_edges_from_prefix(&mut self) -> Result<(), FanoutTreeError> {
        self.roots.clear();
        let mut next = 0;
        while next < self.nodes.len() {
            let root = FanoutTreeNodeId(next);
            self.convert_root(root)?;
            self.roots.push(root);
            next = self.add_edges_rec(root, next + 1)?;
        }
        Ok(())
    }

    pub fn roots(&self) -> &[FanoutTreeNodeId] {
        &self.roots
    }

    pub fn nodes(&self) -> &[FanoutTreeNode] {
        &self.nodes
    }

    pub fn node(&self, id: FanoutTreeNodeId) -> Option<&FanoutTreeNode> {
        self.nodes.get(id.index())
    }

    pub fn root_node(&self) -> Result<FanoutTreeNodeId, FanoutTreeError> {
        match self.roots.as_slice() {
            [root] => Ok(*root),
            [] => Err(FanoutTreeError::EmptyForest),
            roots => Err(FanoutTreeError::ExpectedSingleRoot {
                actual: roots.len(),
            }),
        }
    }

    pub fn validate(&self) -> Result<(), FanoutTreeError> {
        if self.nodes.is_empty() {
            return Err(FanoutTreeError::EmptyForest);
        }

        let roots: BTreeSet<_> = self.roots.iter().copied().collect();
        for root in &self.roots {
            self.require_node(*root)?;
            if self.require_node(*root)?.kind() != FanoutTreeNodeKind::Source {
                return Err(FanoutTreeError::ExpectedSourceRoot { node: *root });
            }
        }

        for (index, node) in self.nodes.iter().enumerate() {
            let id = FanoutTreeNodeId(index);
            match node {
                FanoutTreeNode::Sink(sink) => {
                    if sink.name.is_empty() {
                        return Err(FanoutTreeError::EmptySinkName { node: id });
                    }
                    validate_metric(id, "sink.load", sink.load, true)?;
                    validate_delay(id, "sink.required", sink.required)?;
                }
                FanoutTreeNode::Gate {
                    kind,
                    arity,
                    children,
                    load,
                    required,
                    arrival,
                    area,
                    ..
                } => {
                    if *arity == 0 {
                        return Err(FanoutTreeError::InvalidArity {
                            node: id,
                            arity: *arity,
                        });
                    }
                    if *arity != children.len() {
                        return Err(FanoutTreeError::ChildCountMismatch {
                            node: id,
                            arity: *arity,
                            actual: children.len(),
                        });
                    }
                    if *kind == FanoutTreeNodeKind::Source && !roots.contains(&id) {
                        return Err(FanoutTreeError::UnrootedSource { node: id });
                    }
                    if *kind == FanoutTreeNodeKind::Buffer && roots.contains(&id) {
                        return Err(FanoutTreeError::ExpectedSourceRoot { node: id });
                    }
                    for child in children {
                        self.require_node(*child)?;
                    }
                    validate_metric(id, "gate.load", *load, true)?;
                    validate_delay(id, "gate.required", *required)?;
                    validate_delay(id, "gate.arrival", *arrival)?;
                    validate_metric(id, "gate.area", *area, true)?;
                }
            }
        }

        self.bottom_up_order()?;
        Ok(())
    }

    pub fn preorder(&self) -> Result<Vec<FanoutTreeNodeId>, FanoutTreeError> {
        let mut state = vec![VisitState::Unvisited; self.nodes.len()];
        let mut order = Vec::new();
        for root in &self.roots {
            self.preorder_rec(*root, &mut state, &mut order)?;
        }
        Ok(order)
    }

    pub fn bottom_up_order(&self) -> Result<Vec<FanoutTreeNodeId>, FanoutTreeError> {
        let mut state = vec![VisitState::Unvisited; self.nodes.len()];
        let mut order = Vec::new();
        for root in &self.roots {
            self.bottom_up_rec(*root, &mut state, &mut order)?;
        }
        Ok(order)
    }

    pub fn sink_ids(&self) -> Result<Vec<FanoutTreeNodeId>, FanoutTreeError> {
        let mut sinks = BTreeSet::new();
        for id in self.preorder()? {
            if self.require_node(id)?.kind() == FanoutTreeNodeKind::Sink {
                sinks.insert(id);
            }
        }
        Ok(sinks.into_iter().collect())
    }

    pub fn source_load(&self, timing: &impl FanoutDelayModel) -> Result<f64, FanoutTreeError> {
        self.node_load(self.root_node()?, timing)
    }

    pub fn node_load(
        &self,
        id: FanoutTreeNodeId,
        timing: &impl FanoutDelayModel,
    ) -> Result<f64, FanoutTreeError> {
        let node = self.require_node(id)?;
        let FanoutTreeNode::Gate { children, .. } = node else {
            return Err(FanoutTreeError::ExpectedGate { node: id });
        };

        let mut load = 0.0;
        for child in children {
            match self.require_node(*child)? {
                FanoutTreeNode::Sink(sink) => {
                    load += sink.load;
                }
                FanoutTreeNode::Gate { gate_index, .. } => {
                    load += timing.buffer_load(*gate_index)?;
                }
            }
        }
        load += timing.wire_load(children.len())?;
        validate_metric(id, "computed.load", load, true)?;
        Ok(load)
    }

    pub fn compute_required_times(
        &mut self,
        timing: &impl FanoutDelayModel,
    ) -> Result<DelayTime, FanoutTreeError> {
        let roots = self.roots.clone();
        let mut required = DelayTime::new(f64::INFINITY, f64::INFINITY);
        for root in roots {
            let root_required = self.required_rec(root, timing)?;
            required = required.min(root_required);
        }
        validate_delay(FanoutTreeNodeId(0), "forest.required", required)?;
        Ok(required)
    }

    pub fn compute_arrival_times(
        &mut self,
        timing: &impl FanoutDelayModel,
    ) -> Result<(), FanoutTreeError> {
        let roots = self.roots.clone();
        for root in roots {
            self.set_arrival(root, DelayTime::new(0.0, 0.0))?;
            let load = self.node_load(root, timing)?;
            self.arrival_rec(root, load, timing)?;
        }
        Ok(())
    }

    pub fn total_gate_area(&self, timing: &impl FanoutDelayModel) -> Result<f64, FanoutTreeError> {
        let mut area = 0.0;
        for id in self.bottom_up_order()? {
            if let FanoutTreeNode::Gate { gate_index, .. } = self.require_node(id)? {
                area += timing.gate_area(*gate_index)?;
            }
        }
        validate_metric(FanoutTreeNodeId(0), "forest.area", area, true)?;
        Ok(area)
    }

    pub fn check_summary(
        &mut self,
        expected: FanoutCost,
        timing: &impl FanoutDelayModel,
    ) -> Result<FanoutCost, FanoutTreeError> {
        let required = self.compute_required_times(timing)?;
        let area = self.total_gate_area(timing)?;
        let summary = FanoutCost {
            slack: required,
            area,
        };

        if !delay_equal(summary.slack, expected.slack) {
            return Err(FanoutTreeError::CostMismatch {
                metric: "slack",
                expected: expected.slack,
                actual: summary.slack,
            });
        }
        if !float_equal(summary.area, expected.area) {
            return Err(FanoutTreeError::AreaMismatch {
                expected: expected.area,
                actual: summary.area,
            });
        }

        Ok(summary)
    }

    fn push(&mut self, node: FanoutTreeNode) -> FanoutTreeNodeId {
        let id = FanoutTreeNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    fn require_node(&self, id: FanoutTreeNodeId) -> Result<&FanoutTreeNode, FanoutTreeError> {
        self.node(id)
            .ok_or(FanoutTreeError::MissingNode { node: id })
    }

    fn require_node_mut(
        &mut self,
        id: FanoutTreeNodeId,
    ) -> Result<&mut FanoutTreeNode, FanoutTreeError> {
        self.nodes
            .get_mut(id.index())
            .ok_or(FanoutTreeError::MissingNode { node: id })
    }

    fn convert_root(&mut self, root: FanoutTreeNodeId) -> Result<(), FanoutTreeError> {
        match self.require_node_mut(root)? {
            FanoutTreeNode::Gate { kind, .. } if *kind == FanoutTreeNodeKind::Buffer => {
                *kind = FanoutTreeNodeKind::Source;
                Ok(())
            }
            FanoutTreeNode::Gate { kind, .. } if *kind == FanoutTreeNodeKind::Source => Ok(()),
            _ => Err(FanoutTreeError::ExpectedSourceRoot { node: root }),
        }
    }

    fn add_edges_rec(
        &mut self,
        parent: FanoutTreeNodeId,
        mut next: usize,
    ) -> Result<usize, FanoutTreeError> {
        let arity = match self.require_node(parent)? {
            FanoutTreeNode::Sink(_) => return Ok(next),
            FanoutTreeNode::Gate { arity, .. } => *arity,
        };

        let mut children = Vec::with_capacity(arity);
        for _ in 0..arity {
            let child = FanoutTreeNodeId(next);
            self.require_node(child)?;
            children.push(child);
            next = match self.require_node(child)? {
                FanoutTreeNode::Sink(_) => next + 1,
                FanoutTreeNode::Gate { .. } => self.add_edges_rec(child, next + 1)?,
            };
        }

        if let FanoutTreeNode::Gate {
            children: current, ..
        } = self.require_node_mut(parent)?
        {
            *current = children;
        }

        Ok(next)
    }

    fn preorder_rec(
        &self,
        id: FanoutTreeNodeId,
        state: &mut [VisitState],
        order: &mut Vec<FanoutTreeNodeId>,
    ) -> Result<(), FanoutTreeError> {
        enter_node(id, state)?;
        order.push(id);
        for child in self.require_node(id)?.children() {
            self.preorder_rec(*child, state, order)?;
        }
        leave_node(id, state);
        Ok(())
    }

    fn bottom_up_rec(
        &self,
        id: FanoutTreeNodeId,
        state: &mut [VisitState],
        order: &mut Vec<FanoutTreeNodeId>,
    ) -> Result<(), FanoutTreeError> {
        enter_node(id, state)?;
        for child in self.require_node(id)?.children() {
            self.bottom_up_rec(*child, state, order)?;
        }
        order.push(id);
        leave_node(id, state);
        Ok(())
    }

    fn required_rec(
        &mut self,
        id: FanoutTreeNodeId,
        timing: &impl FanoutDelayModel,
    ) -> Result<DelayTime, FanoutTreeError> {
        let children = self.require_node(id)?.children().to_vec();
        let gate_index = match self.require_node(id)? {
            FanoutTreeNode::Sink(sink) => return Ok(sink.required),
            FanoutTreeNode::Gate { gate_index, .. } => *gate_index,
        };

        let mut required = DelayTime::new(f64::INFINITY, f64::INFINITY);
        let mut load = 0.0;
        for child in &children {
            match self.require_node(*child)? {
                FanoutTreeNode::Sink(sink) => {
                    required = required.min(sink.required);
                    load += sink.load;
                }
                FanoutTreeNode::Gate { gate_index, .. } => {
                    let child_gate_index = *gate_index;
                    let child_required = self.required_rec(*child, timing)?;
                    required =
                        required.min(timing.backward_intrinsic(child_required, child_gate_index)?);
                    load += timing.buffer_load(child_gate_index)?;
                }
            }
        }

        load += timing.wire_load(children.len())?;
        let required = timing.backward_load_dependent(required, gate_index, load)?;
        validate_delay(id, "computed.required", required)?;
        validate_metric(id, "computed.load", load, true)?;
        self.set_required_and_load(id, required, load)?;
        Ok(required)
    }

    fn arrival_rec(
        &mut self,
        id: FanoutTreeNodeId,
        source_load: f64,
        timing: &impl FanoutDelayModel,
    ) -> Result<(), FanoutTreeError> {
        let (arrival, gate_index, children) = match self.require_node(id)? {
            FanoutTreeNode::Sink(_) => return Ok(()),
            FanoutTreeNode::Gate {
                arrival,
                gate_index,
                children,
                ..
            } => (*arrival, *gate_index, children.clone()),
        };

        let local_arrival = timing.forward_load_dependent(arrival, gate_index, source_load)?;
        validate_delay(id, "computed.local_arrival", local_arrival)?;
        for child in children {
            match self.require_node(child)? {
                FanoutTreeNode::Sink(_) => {
                    self.set_arrival(child, local_arrival)?;
                }
                FanoutTreeNode::Gate {
                    gate_index: child_gate_index,
                    ..
                } => {
                    let child_arrival =
                        timing.forward_intrinsic(local_arrival, *child_gate_index)?;
                    self.set_arrival(child, child_arrival)?;
                    let child_load = self.node_load(child, timing)?;
                    self.arrival_rec(child, child_load, timing)?;
                }
            }
        }

        Ok(())
    }

    fn set_required_and_load(
        &mut self,
        id: FanoutTreeNodeId,
        required: DelayTime,
        load: f64,
    ) -> Result<(), FanoutTreeError> {
        let FanoutTreeNode::Gate {
            required: node_required,
            load: node_load,
            ..
        } = self.require_node_mut(id)?
        else {
            return Err(FanoutTreeError::ExpectedGate { node: id });
        };
        *node_required = required;
        *node_load = load;
        Ok(())
    }

    fn set_arrival(
        &mut self,
        id: FanoutTreeNodeId,
        arrival: DelayTime,
    ) -> Result<(), FanoutTreeError> {
        match self.require_node_mut(id)? {
            FanoutTreeNode::Sink(_) => Ok(()),
            FanoutTreeNode::Gate {
                arrival: node_arrival,
                ..
            } => {
                *node_arrival = arrival;
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutCost {
    pub slack: DelayTime,
    pub area: f64,
}

pub trait FanoutDelayModel {
    fn buffer_load(&self, gate_index: usize) -> Result<f64, FanoutTreeError>;

    fn gate_area(&self, gate_index: usize) -> Result<f64, FanoutTreeError>;

    fn wire_load(&self, fanout_count: usize) -> Result<f64, FanoutTreeError>;

    fn backward_intrinsic(
        &self,
        required: DelayTime,
        gate_index: usize,
    ) -> Result<DelayTime, FanoutTreeError>;

    fn backward_load_dependent(
        &self,
        required: DelayTime,
        gate_index: usize,
        load: f64,
    ) -> Result<DelayTime, FanoutTreeError>;

    fn forward_intrinsic(
        &self,
        arrival: DelayTime,
        gate_index: usize,
    ) -> Result<DelayTime, FanoutTreeError>;

    fn forward_load_dependent(
        &self,
        arrival: DelayTime,
        gate_index: usize,
        load: f64,
    ) -> Result<DelayTime, FanoutTreeError>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum FanoutTreeError {
    EmptyForest,
    MissingNode {
        node: FanoutTreeNodeId,
    },
    EmptySinkName {
        node: FanoutTreeNodeId,
    },
    InvalidArity {
        node: FanoutTreeNodeId,
        arity: usize,
    },
    ChildCountMismatch {
        node: FanoutTreeNodeId,
        arity: usize,
        actual: usize,
    },
    ExpectedSourceRoot {
        node: FanoutTreeNodeId,
    },
    ExpectedSingleRoot {
        actual: usize,
    },
    ExpectedGate {
        node: FanoutTreeNodeId,
    },
    UnrootedSource {
        node: FanoutTreeNodeId,
    },
    CycleDetected {
        node: FanoutTreeNodeId,
    },
    InvalidMetric {
        node: FanoutTreeNodeId,
        metric: &'static str,
        value: f64,
    },
    InvalidDelay {
        node: FanoutTreeNodeId,
        metric: &'static str,
        value: DelayTime,
    },
    MissingTimingGate {
        gate_index: usize,
    },
    CostMismatch {
        metric: &'static str,
        expected: DelayTime,
        actual: DelayTime,
    },
    AreaMismatch {
        expected: f64,
        actual: f64,
    },
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for FanoutTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyForest => write!(f, "fanout tree forest has no nodes"),
            Self::MissingNode { node } => write!(f, "missing fanout tree node {}", node.index()),
            Self::EmptySinkName { node } => {
                write!(f, "fanout sink node {} has an empty name", node.index())
            }
            Self::InvalidArity { node, arity } => {
                write!(
                    f,
                    "fanout gate node {} has invalid arity {arity}",
                    node.index()
                )
            }
            Self::ChildCountMismatch {
                node,
                arity,
                actual,
            } => write!(
                f,
                "fanout gate node {} has {actual} children but declares arity {arity}",
                node.index()
            ),
            Self::ExpectedSourceRoot { node } => {
                write!(f, "fanout tree root {} is not a source node", node.index())
            }
            Self::ExpectedSingleRoot { actual } => {
                write!(f, "expected one fanout tree root but found {actual}")
            }
            Self::ExpectedGate { node } => {
                write!(f, "fanout tree node {} is not a gate", node.index())
            }
            Self::UnrootedSource { node } => {
                write!(
                    f,
                    "fanout source node {} is not listed as a root",
                    node.index()
                )
            }
            Self::CycleDetected { node } => {
                write!(f, "fanout tree contains a cycle at node {}", node.index())
            }
            Self::InvalidMetric {
                node,
                metric,
                value,
            } => write!(
                f,
                "fanout tree node {} has invalid {metric} {value}",
                node.index()
            ),
            Self::InvalidDelay {
                node,
                metric,
                value,
            } => write!(
                f,
                "fanout tree node {} has invalid {metric} ({}, {})",
                node.index(),
                value.rise,
                value.fall
            ),
            Self::MissingTimingGate { gate_index } => {
                write!(f, "missing fanout timing data for gate index {gate_index}")
            }
            Self::CostMismatch {
                metric,
                expected,
                actual,
            } => write!(
                f,
                "fanout tree {metric} mismatch: expected ({}, {}), actual ({}, {})",
                expected.rise, expected.fall, actual.rise, actual.fall
            ),
            Self::AreaMismatch { expected, actual } => {
                write!(
                    f,
                    "fanout tree area mismatch: expected {expected}, actual {actual}"
                )
            }
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for FanoutTreeError {}

pub fn materialize_full_sis_fanout_tree_unavailable() -> Result<(), FanoutTreeError> {
    Err(FanoutTreeError::MissingSisPorts {
        operation: "fanout_tree full SIS network materialization",
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn enter_node(id: FanoutTreeNodeId, state: &mut [VisitState]) -> Result<(), FanoutTreeError> {
    let Some(slot) = state.get_mut(id.index()) else {
        return Err(FanoutTreeError::MissingNode { node: id });
    };

    match *slot {
        VisitState::Unvisited => {
            *slot = VisitState::Visiting;
            Ok(())
        }
        VisitState::Visiting => Err(FanoutTreeError::CycleDetected { node: id }),
        VisitState::Visited => Ok(()),
    }
}

fn leave_node(id: FanoutTreeNodeId, state: &mut [VisitState]) {
    if let Some(slot) = state.get_mut(id.index()) {
        *slot = VisitState::Visited;
    }
}

fn validate_metric(
    node: FanoutTreeNodeId,
    metric: &'static str,
    value: f64,
    allow_infinite: bool,
) -> Result<(), FanoutTreeError> {
    if value.is_nan() || (!allow_infinite && !value.is_finite()) || value < 0.0 {
        return Err(FanoutTreeError::InvalidMetric {
            node,
            metric,
            value,
        });
    }
    Ok(())
}

fn validate_delay(
    node: FanoutTreeNodeId,
    metric: &'static str,
    value: DelayTime,
) -> Result<(), FanoutTreeError> {
    if value.rise.is_nan() || value.fall.is_nan() {
        return Err(FanoutTreeError::InvalidDelay {
            node,
            metric,
            value,
        });
    }
    Ok(())
}

fn delay_equal(left: DelayTime, right: DelayTime) -> bool {
    float_equal(left.rise, right.rise) && float_equal(left.fall, right.fall)
}

fn float_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1.0e-9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct GateTiming {
        load: f64,
        area: f64,
        intrinsic: f64,
        load_factor: f64,
    }

    struct TestTiming {
        gates: Vec<GateTiming>,
        wire_factor: f64,
    }

    impl FanoutDelayModel for TestTiming {
        fn buffer_load(&self, gate_index: usize) -> Result<f64, FanoutTreeError> {
            Ok(self.gate(gate_index)?.load)
        }

        fn gate_area(&self, gate_index: usize) -> Result<f64, FanoutTreeError> {
            Ok(self.gate(gate_index)?.area)
        }

        fn wire_load(&self, fanout_count: usize) -> Result<f64, FanoutTreeError> {
            Ok(fanout_count as f64 * self.wire_factor)
        }

        fn backward_intrinsic(
            &self,
            required: DelayTime,
            gate_index: usize,
        ) -> Result<DelayTime, FanoutTreeError> {
            let gate = self.gate(gate_index)?;
            Ok(DelayTime::new(
                required.rise - gate.intrinsic,
                required.fall - gate.intrinsic,
            ))
        }

        fn backward_load_dependent(
            &self,
            required: DelayTime,
            gate_index: usize,
            load: f64,
        ) -> Result<DelayTime, FanoutTreeError> {
            let gate = self.gate(gate_index)?;
            let delay = load * gate.load_factor;
            Ok(DelayTime::new(required.rise - delay, required.fall - delay))
        }

        fn forward_intrinsic(
            &self,
            arrival: DelayTime,
            gate_index: usize,
        ) -> Result<DelayTime, FanoutTreeError> {
            let gate = self.gate(gate_index)?;
            Ok(DelayTime::new(
                arrival.rise + gate.intrinsic,
                arrival.fall + gate.intrinsic,
            ))
        }

        fn forward_load_dependent(
            &self,
            arrival: DelayTime,
            gate_index: usize,
            load: f64,
        ) -> Result<DelayTime, FanoutTreeError> {
            let gate = self.gate(gate_index)?;
            let delay = load * gate.load_factor;
            Ok(DelayTime::new(arrival.rise + delay, arrival.fall + delay))
        }
    }

    impl TestTiming {
        fn gate(&self, gate_index: usize) -> Result<GateTiming, FanoutTreeError> {
            self.gates
                .get(gate_index)
                .copied()
                .ok_or(FanoutTreeError::MissingTimingGate { gate_index })
        }
    }

    fn timing() -> TestTiming {
        TestTiming {
            gates: vec![
                GateTiming {
                    load: 2.0,
                    area: 5.0,
                    intrinsic: 1.0,
                    load_factor: 0.5,
                },
                GateTiming {
                    load: 1.5,
                    area: 3.0,
                    intrinsic: 0.5,
                    load_factor: 0.25,
                },
            ],
            wire_factor: 0.1,
        }
    }

    fn sample_forest() -> FanoutTreeForest {
        FanoutTreeForest::from_prefix(vec![
            FanoutTreeNode::buffer(0, 2),
            FanoutTreeNode::sink(FanoutSink::new(
                "fast",
                0,
                FanoutPolarity::Positive,
                1.0,
                DelayTime::new(10.0, 9.0),
            )),
            FanoutTreeNode::buffer(1, 1),
            FanoutTreeNode::sink(FanoutSink::new(
                "slow",
                1,
                FanoutPolarity::Positive,
                0.5,
                DelayTime::new(8.0, 7.0),
            )),
        ])
        .unwrap()
    }

    #[test]
    fn builds_edges_from_sis_prefix_order_and_traverses_deterministically() {
        let forest = sample_forest();

        assert_eq!(forest.roots(), &[FanoutTreeNodeId(0)]);
        assert_eq!(
            forest.preorder().unwrap(),
            vec![
                FanoutTreeNodeId(0),
                FanoutTreeNodeId(1),
                FanoutTreeNodeId(2),
                FanoutTreeNodeId(3),
            ]
        );
        assert_eq!(
            forest.bottom_up_order().unwrap(),
            vec![
                FanoutTreeNodeId(1),
                FanoutTreeNodeId(3),
                FanoutTreeNodeId(2),
                FanoutTreeNodeId(0),
            ]
        );
    }

    #[test]
    fn computes_source_load_required_times_arrivals_and_area() {
        let mut forest = sample_forest();
        let timing = timing();

        assert_eq!(forest.source_load(&timing).unwrap(), 2.7);
        assert_eq!(
            forest.compute_required_times(&timing).unwrap(),
            DelayTime::new(6.0, 5.0)
        );
        forest.compute_arrival_times(&timing).unwrap();

        assert_eq!(forest.total_gate_area(&timing).unwrap(), 8.0);
        assert_eq!(
            forest.require_node(FanoutTreeNodeId(2)).unwrap().arrival(),
            DelayTime::new(1.85, 1.85)
        );
        assert_eq!(
            forest
                .check_summary(
                    FanoutCost {
                        slack: DelayTime::new(6.0, 5.0),
                        area: 8.0,
                    },
                    &timing,
                )
                .unwrap()
                .area,
            8.0
        );
    }

    #[test]
    fn rejects_bad_prefix_roots_arity_missing_timing_and_empty_sink_names() {
        assert_eq!(
            FanoutTreeForest::from_prefix(vec![FanoutTreeNode::sink(FanoutSink::new(
                "sink",
                0,
                FanoutPolarity::Positive,
                1.0,
                DelayTime::new(1.0, 1.0),
            ))])
            .unwrap_err(),
            FanoutTreeError::ExpectedSourceRoot {
                node: FanoutTreeNodeId(0),
            }
        );

        assert_eq!(
            FanoutTreeForest::empty().insert_gate(0, 0).unwrap_err(),
            FanoutTreeError::InvalidArity {
                node: FanoutTreeNodeId(0),
                arity: 0,
            }
        );

        assert_eq!(
            FanoutTreeForest::from_prefix(vec![
                FanoutTreeNode::buffer(0, 1),
                FanoutTreeNode::sink(FanoutSink::new(
                    "",
                    0,
                    FanoutPolarity::Positive,
                    1.0,
                    DelayTime::new(1.0, 1.0),
                )),
            ])
            .unwrap_err(),
            FanoutTreeError::EmptySinkName {
                node: FanoutTreeNodeId(1),
            }
        );

        let forest = FanoutTreeForest::from_prefix(vec![
            FanoutTreeNode::buffer(0, 1),
            FanoutTreeNode::buffer(99, 1),
            FanoutTreeNode::sink(FanoutSink::new(
                "sink",
                0,
                FanoutPolarity::Positive,
                1.0,
                DelayTime::new(1.0, 1.0),
            )),
        ])
        .unwrap();
        assert_eq!(
            forest.source_load(&timing()).unwrap_err(),
            FanoutTreeError::MissingTimingGate { gate_index: 99 }
        );
    }

    #[test]
    fn preserves_saved_sink_metadata_for_fanout_est_integration() {
        let sink = FanoutSink::new(
            "po",
            0,
            FanoutPolarity::Negative,
            2.0,
            DelayTime::new(4.0, 4.0),
        )
        .with_saved_fanout_index(7);

        assert_eq!(sink.saved_fanout_index, Some(7));
        assert_eq!(sink.polarity.inverted(), FanoutPolarity::Positive);
        assert_eq!(
            FanoutPolarity::Positive.propagate_through(BufferPolarity::Inverting),
            FanoutPolarity::Negative
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("fanout_tree.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
