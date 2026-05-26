//! Bounded native Rust matcher for `sis/map/match.c`.
//!
//! The original SIS implementation recursively binds `node_t` objects to
//! primitive graph nodes and invokes a callback for every complete match. This
//! port keeps the owned-data part available to the mapper ports: validate
//! primitive match patterns, match them against `MapperTree` nodes with
//! polarity and arity constraints, derive simple candidates from genlib gates,
//! return deterministic, cost-ordered tree results, and backtrack across owned
//! primitive/network graph nodes without preserving C ABI entry points.

use std::error::Error;
use std::fmt;

use super::library::{GenlibGate, GenlibLibrary};
use super::tree::{
    MapperTree, MapperTreeError, MapperTreeNode, MapperTreeNodeId, PrimitiveGateKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MatchLimits {
    pub max_patterns: usize,
    pub max_pattern_nodes: usize,
    pub max_name_length: usize,
    pub max_matches: usize,
}

impl Default for MatchLimits {
    fn default() -> Self {
        Self {
            max_patterns: 16_384,
            max_pattern_nodes: 256,
            max_name_length: 256,
            max_matches: 65_536,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchPolarity {
    NonInverting,
    Inverting,
    Any,
}

impl MatchPolarity {
    pub fn accepts(self, inverted: bool) -> bool {
        match self {
            Self::NonInverting => !inverted,
            Self::Inverting => inverted,
            Self::Any => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchArity {
    Exact(usize),
    AtLeast(usize),
    Any,
}

impl MatchArity {
    pub fn accepts(self, value: usize) -> bool {
        match self {
            Self::Exact(expected) => value == expected,
            Self::AtLeast(expected) => value >= expected,
            Self::Any => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchEdge {
    pub polarity: MatchPolarity,
    pub pattern: Box<MatchPattern>,
}

impl MatchEdge {
    pub fn new(polarity: MatchPolarity, pattern: MatchPattern) -> Self {
        Self {
            polarity,
            pattern: Box::new(pattern),
        }
    }

    pub fn non_inverting(pattern: MatchPattern) -> Self {
        Self::new(MatchPolarity::NonInverting, pattern)
    }

    pub fn inverting(pattern: MatchPattern) -> Self {
        Self::new(MatchPolarity::Inverting, pattern)
    }

    pub fn any(pattern: MatchPattern) -> Self {
        Self::new(MatchPolarity::Any, pattern)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchPattern {
    Boundary {
        name: String,
    },
    Gate {
        kind: PrimitiveGateKind,
        arity: MatchArity,
        fanins: Vec<MatchEdge>,
    },
}

impl MatchPattern {
    pub fn boundary(name: impl Into<String>) -> Self {
        Self::Boundary { name: name.into() }
    }

    pub fn gate(kind: PrimitiveGateKind, fanins: Vec<MatchEdge>) -> Self {
        Self::Gate {
            kind,
            arity: MatchArity::Exact(fanins.len()),
            fanins,
        }
    }

    pub fn gate_with_arity(
        kind: PrimitiveGateKind,
        arity: MatchArity,
        fanins: Vec<MatchEdge>,
    ) -> Self {
        Self::Gate {
            kind,
            arity,
            fanins,
        }
    }

    fn node_count(&self) -> usize {
        match self {
            Self::Boundary { .. } => 1,
            Self::Gate { fanins, .. } => {
                1 + fanins
                    .iter()
                    .map(|edge| edge.pattern.node_count())
                    .sum::<usize>()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchGate {
    pub name: String,
    pub pattern: MatchPattern,
    pub cost: f64,
}

impl MatchGate {
    pub fn new(
        name: impl Into<String>,
        pattern: MatchPattern,
        cost: f64,
    ) -> Result<Self, MatchError> {
        let gate = Self {
            name: name.into(),
            pattern,
            cost,
        };
        gate.validate(MatchLimits::default())?;
        Ok(gate)
    }

    fn validate(&self, limits: MatchLimits) -> Result<(), MatchError> {
        validate_name(&self.name, "match gate", limits.max_name_length)?;
        if !self.cost.is_finite() || self.cost < 0.0 {
            return Err(MatchError::InvalidCost {
                gate: self.name.clone(),
                cost: self.cost,
            });
        }
        if self.pattern.node_count() > limits.max_pattern_nodes {
            return Err(MatchError::PatternTooLarge {
                gate: self.name.clone(),
                max: limits.max_pattern_nodes,
            });
        }
        validate_pattern(&self.name, &self.pattern, limits)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MatchLibrary {
    gates: Vec<MatchGate>,
}

impl MatchLibrary {
    pub fn new(gates: Vec<MatchGate>) -> Result<Self, MatchError> {
        Self::with_limits(gates, MatchLimits::default())
    }

    pub fn with_limits(gates: Vec<MatchGate>, limits: MatchLimits) -> Result<Self, MatchError> {
        if gates.len() > limits.max_patterns {
            return Err(MatchError::TooManyPatterns {
                max: limits.max_patterns,
            });
        }
        for gate in &gates {
            gate.validate(limits)?;
        }
        Ok(Self { gates })
    }

    pub fn from_genlib(library: &GenlibLibrary) -> Result<Self, MatchError> {
        let gates = library
            .gates
            .iter()
            .filter_map(match_gate_from_genlib)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(gates)
    }

    pub fn gates(&self) -> &[MatchGate] {
        &self.gates
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeMatch {
    pub gate: String,
    pub root: MapperTreeNodeId,
    pub frontier: Vec<MapperTreeNodeId>,
    pub cost: f64,
    pub pattern_nodes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NetMatchNodeId(usize);

impl NetMatchNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PrimitiveMatchNodeId(usize);

impl PrimitiveMatchNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchDirection {
    In,
    Out,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InternalFanoutPolicy {
    Exact,
    UpToLimit(usize),
    ExactWhenNextPrimitiveInternal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphMatchOptions {
    pub internal_fanout_policy: InternalFanoutPolicy,
}

impl Default for GraphMatchOptions {
    fn default() -> Self {
        Self {
            internal_fanout_policy: InternalFanoutPolicy::Exact,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetMatchNode {
    pub name: String,
    pub kind: MatchNodeKind,
    pub fanins: Vec<NetMatchNodeId>,
    pub fanouts: Vec<NetMatchNodeId>,
}

impl NetMatchNode {
    pub fn new(name: impl Into<String>, kind: MatchNodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NetMatchGraph {
    nodes: Vec<NetMatchNode>,
}

impl NetMatchGraph {
    pub fn new(nodes: Vec<NetMatchNode>) -> Result<Self, MatchError> {
        let graph = Self { nodes };
        graph.validate()?;
        Ok(graph)
    }

    pub fn empty() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, name: impl Into<String>, kind: MatchNodeKind) -> NetMatchNodeId {
        let id = NetMatchNodeId(self.nodes.len());
        self.nodes.push(NetMatchNode::new(name, kind));
        id
    }

    pub fn add_edge(
        &mut self,
        fanin: NetMatchNodeId,
        fanout: NetMatchNodeId,
    ) -> Result<(), MatchError> {
        self.require_node(fanin)?;
        self.require_node(fanout)?;
        if !self.nodes[fanout.index()].fanins.contains(&fanin) {
            self.nodes[fanout.index()].fanins.push(fanin);
        }
        if !self.nodes[fanin.index()].fanouts.contains(&fanout) {
            self.nodes[fanin.index()].fanouts.push(fanout);
        }
        Ok(())
    }

    pub fn node(&self, id: NetMatchNodeId) -> Option<&NetMatchNode> {
        self.nodes.get(id.index())
    }

    pub fn nodes(&self) -> &[NetMatchNode] {
        &self.nodes
    }

    pub fn validate(&self) -> Result<(), MatchError> {
        for (index, node) in self.nodes.iter().enumerate() {
            validate_name(
                &node.name,
                "network match node",
                MatchLimits::default().max_name_length,
            )?;
            let id = NetMatchNodeId(index);
            for fanin in &node.fanins {
                self.require_node(*fanin)?;
                if !self.nodes[fanin.index()].fanouts.contains(&id) {
                    return Err(MatchError::InconsistentGraphEdge);
                }
            }
            for fanout in &node.fanouts {
                self.require_node(*fanout)?;
                if !self.nodes[fanout.index()].fanins.contains(&id) {
                    return Err(MatchError::InconsistentGraphEdge);
                }
            }
        }
        Ok(())
    }

    fn require_node(&self, id: NetMatchNodeId) -> Result<&NetMatchNode, MatchError> {
        self.node(id)
            .ok_or(MatchError::MissingNetworkNode { node: id })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveMatchNode {
    pub name: String,
    pub kind: MatchNodeKind,
    pub isomorphic_sons: bool,
    pub fanin_count: usize,
    pub fanout_count: usize,
}

impl PrimitiveMatchNode {
    pub fn new(name: impl Into<String>, kind: MatchNodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            isomorphic_sons: false,
            fanin_count: 0,
            fanout_count: 0,
        }
    }

    pub fn with_isomorphic_sons(mut self) -> Self {
        self.isomorphic_sons = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveMatchEdge {
    pub this_node: PrimitiveMatchNodeId,
    pub connected_node: Option<PrimitiveMatchNodeId>,
    pub direction: MatchDirection,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PrimitiveMatchGraph {
    nodes: Vec<PrimitiveMatchNode>,
    edges: Vec<PrimitiveMatchEdge>,
}

impl PrimitiveMatchGraph {
    pub fn new(
        nodes: Vec<PrimitiveMatchNode>,
        edges: Vec<PrimitiveMatchEdge>,
    ) -> Result<Self, MatchError> {
        let graph = Self { nodes, edges };
        graph.validate()?;
        Ok(graph)
    }

    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        kind: MatchNodeKind,
    ) -> PrimitiveMatchNodeId {
        let id = PrimitiveMatchNodeId(self.nodes.len());
        self.nodes.push(PrimitiveMatchNode::new(name, kind));
        id
    }

    pub fn set_isomorphic_sons(
        &mut self,
        node: PrimitiveMatchNodeId,
        value: bool,
    ) -> Result<(), MatchError> {
        self.require_node(node)?;
        self.nodes[node.index()].isomorphic_sons = value;
        Ok(())
    }

    pub fn add_edge(
        &mut self,
        this_node: PrimitiveMatchNodeId,
        connected_node: Option<PrimitiveMatchNodeId>,
        direction: MatchDirection,
    ) -> Result<(), MatchError> {
        self.require_node(this_node)?;
        if let Some(connected_node) = connected_node {
            self.require_node(connected_node)?;
            match direction {
                MatchDirection::In => {
                    self.nodes[this_node.index()].fanout_count += 1;
                    self.nodes[connected_node.index()].fanin_count += 1;
                }
                MatchDirection::Out => {
                    self.nodes[this_node.index()].fanin_count += 1;
                    self.nodes[connected_node.index()].fanout_count += 1;
                }
            }
        }
        self.edges.push(PrimitiveMatchEdge {
            this_node,
            connected_node,
            direction,
        });
        Ok(())
    }

    pub fn node(&self, id: PrimitiveMatchNodeId) -> Option<&PrimitiveMatchNode> {
        self.nodes.get(id.index())
    }

    pub fn nodes(&self) -> &[PrimitiveMatchNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[PrimitiveMatchEdge] {
        &self.edges
    }

    pub fn validate(&self) -> Result<(), MatchError> {
        if self.edges.is_empty() {
            return Err(MatchError::EmptyPrimitiveEdges);
        }
        for node in &self.nodes {
            validate_name(
                &node.name,
                "primitive match node",
                MatchLimits::default().max_name_length,
            )?;
        }
        for (index, edge) in self.edges.iter().enumerate() {
            self.require_node(edge.this_node)?;
            if index == 0 && edge.connected_node.is_some() {
                return Err(MatchError::InvalidPrimitiveEdge {
                    reason: "first primitive edge cannot have a connected node",
                });
            }
            if index > 0 && edge.connected_node.is_none() {
                return Err(MatchError::InvalidPrimitiveEdge {
                    reason: "non-root primitive edges must have a connected node",
                });
            }
            if let Some(connected_node) = edge.connected_node {
                self.require_node(connected_node)?;
            }
        }
        Ok(())
    }

    fn require_node(&self, id: PrimitiveMatchNodeId) -> Result<&PrimitiveMatchNode, MatchError> {
        self.node(id)
            .ok_or(MatchError::MissingPrimitiveNode { node: id })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphMatchBinding {
    pub primitive: PrimitiveMatchNodeId,
    pub network: NetMatchNodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphMatch {
    pub bindings: Vec<GraphMatchBinding>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatchError {
    Tree(MapperTreeError),
    TooManyPatterns {
        max: usize,
    },
    TooManyMatches {
        max: usize,
    },
    PatternTooLarge {
        gate: String,
        max: usize,
    },
    EmptyName {
        kind: &'static str,
    },
    NameTooLong {
        kind: &'static str,
        name: String,
        max_name_length: usize,
    },
    InvalidArity {
        gate: String,
        reason: &'static str,
    },
    InvalidPattern {
        gate: String,
        reason: &'static str,
    },
    InvalidCost {
        gate: String,
        cost: f64,
    },
    MissingNetworkNode {
        node: NetMatchNodeId,
    },
    MissingPrimitiveNode {
        node: PrimitiveMatchNodeId,
    },
    EmptyPrimitiveEdges,
    InvalidPrimitiveEdge {
        reason: &'static str,
    },
    InconsistentGraphEdge,
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tree(error) => write!(f, "{error}"),
            Self::TooManyPatterns { max } => write!(f, "too many match patterns; max is {max}"),
            Self::TooManyMatches { max } => write!(f, "too many tree matches; max is {max}"),
            Self::PatternTooLarge { gate, max } => {
                write!(f, "match gate '{gate}' pattern exceeds {max} nodes")
            }
            Self::EmptyName { kind } => write!(f, "{kind} name cannot be empty"),
            Self::NameTooLong {
                kind,
                name,
                max_name_length,
            } => write!(
                f,
                "{kind} name '{name}' exceeds {max_name_length} characters"
            ),
            Self::InvalidArity { gate, reason } => {
                write!(f, "match gate '{gate}' has invalid arity: {reason}")
            }
            Self::InvalidPattern { gate, reason } => {
                write!(f, "match gate '{gate}' has invalid pattern: {reason}")
            }
            Self::InvalidCost { gate, cost } => {
                write!(f, "match gate '{gate}' has invalid cost {cost}")
            }
            Self::MissingNetworkNode { node } => {
                write!(f, "missing network match node {}", node.index())
            }
            Self::MissingPrimitiveNode { node } => {
                write!(f, "missing primitive match node {}", node.index())
            }
            Self::EmptyPrimitiveEdges => write!(f, "primitive match graph has no edges"),
            Self::InvalidPrimitiveEdge { reason } => {
                write!(f, "invalid primitive match edge: {reason}")
            }
            Self::InconsistentGraphEdge => write!(f, "network graph edge lists are inconsistent"),
        }
    }
}

impl Error for MatchError {}

impl From<MapperTreeError> for MatchError {
    fn from(value: MapperTreeError) -> Self {
        Self::Tree(value)
    }
}

pub fn enumerate_tree_matches(
    tree: &MapperTree,
    library: &MatchLibrary,
    limits: MatchLimits,
) -> Result<Vec<TreeMatch>, MatchError> {
    tree.validate()?;
    MatchLibrary::with_limits(library.gates.clone(), limits)?;

    let mut matches = Vec::new();
    for root in tree.preorder()? {
        for gate in library.gates() {
            let mut frontier = Vec::new();
            if !match_at(tree, root, &gate.pattern, &mut frontier)? {
                continue;
            }
            if matches.len() >= limits.max_matches {
                return Err(MatchError::TooManyMatches {
                    max: limits.max_matches,
                });
            }
            frontier.sort();
            frontier.dedup();
            matches.push(TreeMatch {
                gate: gate.name.clone(),
                root,
                frontier,
                cost: gate.cost,
                pattern_nodes: gate.pattern.node_count(),
            });
        }
    }

    matches.sort_by(compare_tree_matches);
    Ok(matches)
}

pub fn enumerate_graph_matches(
    network: &NetMatchGraph,
    start: NetMatchNodeId,
    primitive: &PrimitiveMatchGraph,
    options: GraphMatchOptions,
    limits: MatchLimits,
) -> Result<Vec<GraphMatch>, MatchError> {
    network.validate()?;
    network.require_node(start)?;
    primitive.validate()?;

    let mut state = GraphMatchState {
        network_bindings: vec![None; network.nodes().len()],
        primitive_bindings: vec![None; primitive.nodes().len()],
        matches: Vec::new(),
        max_matches: limits.max_matches,
    };

    match_graph_edge(network, primitive, options, start, 0, &mut state)?;
    state.matches.sort_by(compare_graph_matches);
    Ok(state.matches)
}

pub fn matches_at_root(
    tree: &MapperTree,
    library: &MatchLibrary,
    root: MapperTreeNodeId,
    limits: MatchLimits,
) -> Result<Vec<TreeMatch>, MatchError> {
    tree.validate()?;
    MatchLibrary::with_limits(library.gates.clone(), limits)?;

    let mut matches = Vec::new();
    for gate in library.gates() {
        let mut frontier = Vec::new();
        if !match_at(tree, root, &gate.pattern, &mut frontier)? {
            continue;
        }
        if matches.len() >= limits.max_matches {
            return Err(MatchError::TooManyMatches {
                max: limits.max_matches,
            });
        }
        frontier.sort();
        frontier.dedup();
        matches.push(TreeMatch {
            gate: gate.name.clone(),
            root,
            frontier,
            cost: gate.cost,
            pattern_nodes: gate.pattern.node_count(),
        });
    }

    matches.sort_by(compare_tree_matches);
    Ok(matches)
}

struct GraphMatchState {
    network_bindings: Vec<Option<PrimitiveMatchNodeId>>,
    primitive_bindings: Vec<Option<NetMatchNodeId>>,
    matches: Vec<GraphMatch>,
    max_matches: usize,
}

fn match_graph_edge(
    network: &NetMatchGraph,
    primitive: &PrimitiveMatchGraph,
    options: GraphMatchOptions,
    net_node: NetMatchNodeId,
    edge_index: usize,
    state: &mut GraphMatchState,
) -> Result<bool, MatchError> {
    let edge = primitive
        .edges()
        .get(edge_index)
        .ok_or(MatchError::InvalidPrimitiveEdge {
            reason: "primitive edge index is out of range",
        })?;
    let prim_node = edge.this_node;
    let next_edge = primitive.edges().get(edge_index + 1);

    let already_bound = match state.network_bindings[net_node.index()] {
        Some(bound_primitive) if bound_primitive != prim_node => {
            return Ok(true);
        }
        Some(_) => true,
        None => {
            if state.primitive_bindings[prim_node.index()].is_some() {
                return Ok(true);
            }
            if !node_can_bind(network, primitive, options, net_node, prim_node, next_edge)? {
                return Ok(true);
            }
            false
        }
    };

    if !already_bound {
        state.network_bindings[net_node.index()] = Some(prim_node);
        state.primitive_bindings[prim_node.index()] = Some(net_node);
    }

    let should_continue = if edge_index + 1 == primitive.edges().len() {
        push_graph_match(state)?;
        true
    } else {
        match_next_graph_edge(network, primitive, options, edge_index + 1, state)?
    };

    if !already_bound {
        state.network_bindings[net_node.index()] = None;
        state.primitive_bindings[prim_node.index()] = None;
    }

    Ok(should_continue)
}

fn match_next_graph_edge(
    network: &NetMatchGraph,
    primitive: &PrimitiveMatchGraph,
    options: GraphMatchOptions,
    edge_index: usize,
    state: &mut GraphMatchState,
) -> Result<bool, MatchError> {
    let edge = &primitive.edges()[edge_index];
    let connected_node = edge
        .connected_node
        .ok_or(MatchError::InvalidPrimitiveEdge {
            reason: "non-root primitive edge must have a connected node",
        })?;
    let matching_net_node = state.primitive_bindings[connected_node.index()].ok_or(
        MatchError::InvalidPrimitiveEdge {
            reason: "connected primitive node is not bound",
        },
    )?;
    let connected_primitive = primitive.require_node(connected_node)?;

    match edge.direction {
        MatchDirection::In if connected_primitive.isomorphic_sons => {
            let Some(candidate) = network
                .require_node(matching_net_node)?
                .fanins
                .iter()
                .copied()
                .find(|fanin| state.network_bindings[fanin.index()].is_none())
            else {
                return Ok(true);
            };
            match_graph_edge(network, primitive, options, candidate, edge_index, state)
        }
        MatchDirection::In => {
            let fanins = network.require_node(matching_net_node)?.fanins.clone();
            for candidate in fanins {
                if !match_graph_edge(network, primitive, options, candidate, edge_index, state)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        MatchDirection::Out => {
            let fanouts = network.require_node(matching_net_node)?.fanouts.clone();
            for candidate in fanouts {
                if !match_graph_edge(network, primitive, options, candidate, edge_index, state)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

fn node_can_bind(
    network: &NetMatchGraph,
    primitive: &PrimitiveMatchGraph,
    options: GraphMatchOptions,
    net_node: NetMatchNodeId,
    prim_node: PrimitiveMatchNodeId,
    next_edge: Option<&PrimitiveMatchEdge>,
) -> Result<bool, MatchError> {
    let net_node = network.require_node(net_node)?;
    let prim_node = primitive.require_node(prim_node)?;

    if prim_node.kind != MatchNodeKind::PrimaryInput
        && net_node.fanins.len() != prim_node.fanin_count
    {
        return Ok(false);
    }

    if prim_node.kind != MatchNodeKind::Internal {
        return Ok(true);
    }

    let accepts_fanout = match options.internal_fanout_policy {
        InternalFanoutPolicy::Exact => net_node.fanouts.len() == prim_node.fanout_count,
        InternalFanoutPolicy::UpToLimit(limit) => net_node.fanouts.len() <= limit,
        InternalFanoutPolicy::ExactWhenNextPrimitiveInternal => next_edge
            .and_then(|edge| primitive.node(edge.this_node))
            .is_none_or(|next_node| {
                next_node.kind != MatchNodeKind::Internal
                    || net_node.fanouts.len() == prim_node.fanout_count
            }),
    };

    Ok(accepts_fanout)
}

fn push_graph_match(state: &mut GraphMatchState) -> Result<(), MatchError> {
    if state.matches.len() >= state.max_matches {
        return Err(MatchError::TooManyMatches {
            max: state.max_matches,
        });
    }

    let bindings = state
        .primitive_bindings
        .iter()
        .enumerate()
        .filter_map(|(index, network)| {
            network.map(|network| GraphMatchBinding {
                primitive: PrimitiveMatchNodeId(index),
                network,
            })
        })
        .collect::<Vec<_>>();

    state.matches.push(GraphMatch { bindings });
    Ok(())
}

fn match_at(
    tree: &MapperTree,
    root: MapperTreeNodeId,
    pattern: &MatchPattern,
    frontier: &mut Vec<MapperTreeNodeId>,
) -> Result<bool, MatchError> {
    let node = tree
        .node(root)
        .ok_or(MapperTreeError::MissingNode { node: root })?;
    match pattern {
        MatchPattern::Boundary { .. } => {
            frontier.push(root);
            Ok(true)
        }
        MatchPattern::Gate {
            kind,
            arity,
            fanins,
        } => {
            let MapperTreeNode::Gate {
                kind: node_kind,
                fanins: node_fanins,
            } = node
            else {
                return Ok(false);
            };
            if node_kind != kind || !arity.accepts(node_fanins.len()) {
                return Ok(false);
            }
            if fanins.is_empty() {
                frontier.extend(node_fanins.iter().map(|fanin| fanin.node));
                return Ok(true);
            }
            if fanins.len() != node_fanins.len() {
                return Ok(false);
            }
            for (node_fanin, pattern_fanin) in node_fanins.iter().zip(fanins) {
                if !pattern_fanin.polarity.accepts(node_fanin.inverted) {
                    return Ok(false);
                }
                if !match_at(tree, node_fanin.node, &pattern_fanin.pattern, frontier)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

fn compare_tree_matches(left: &TreeMatch, right: &TreeMatch) -> std::cmp::Ordering {
    left.cost
        .total_cmp(&right.cost)
        .then_with(|| right.pattern_nodes.cmp(&left.pattern_nodes))
        .then_with(|| left.gate.cmp(&right.gate))
        .then_with(|| left.root.index().cmp(&right.root.index()))
        .then_with(|| {
            left.frontier
                .iter()
                .map(|node| node.index())
                .cmp(right.frontier.iter().map(|node| node.index()))
        })
}

fn compare_graph_matches(left: &GraphMatch, right: &GraphMatch) -> std::cmp::Ordering {
    left.bindings
        .iter()
        .map(|binding| (binding.primitive.index(), binding.network.index()))
        .cmp(
            right
                .bindings
                .iter()
                .map(|binding| (binding.primitive.index(), binding.network.index())),
        )
}

fn validate_pattern(
    gate: &str,
    pattern: &MatchPattern,
    limits: MatchLimits,
) -> Result<(), MatchError> {
    match pattern {
        MatchPattern::Boundary { name } => {
            validate_name(name, "match boundary", limits.max_name_length)
        }
        MatchPattern::Gate {
            kind,
            arity,
            fanins,
        } => {
            validate_gate_arity(gate, *kind, *arity, fanins)?;
            for fanin in fanins {
                validate_pattern(gate, &fanin.pattern, limits)?;
            }
            Ok(())
        }
    }
}

fn validate_gate_arity(
    gate: &str,
    kind: PrimitiveGateKind,
    arity: MatchArity,
    fanins: &[MatchEdge],
) -> Result<(), MatchError> {
    if matches!(kind, PrimitiveGateKind::One | PrimitiveGateKind::Zero) && !fanins.is_empty() {
        return Err(MatchError::InvalidPattern {
            gate: gate.to_string(),
            reason: "constant match patterns cannot have fanins",
        });
    }
    match arity {
        MatchArity::Exact(value) if !primitive_accepts_arity(kind, value) => {
            Err(MatchError::InvalidArity {
                gate: gate.to_string(),
                reason: "exact arity is not valid for primitive kind",
            })
        }
        MatchArity::AtLeast(value)
            if matches!(
                kind,
                PrimitiveGateKind::Buffer
                    | PrimitiveGateKind::Inverter
                    | PrimitiveGateKind::One
                    | PrimitiveGateKind::Zero
            ) && value != primitive_min_arity(kind) =>
        {
            Err(MatchError::InvalidArity {
                gate: gate.to_string(),
                reason: "minimum arity is not valid for primitive kind",
            })
        }
        MatchArity::Any if !fanins.is_empty() => Err(MatchError::InvalidArity {
            gate: gate.to_string(),
            reason: "wildcard arity cannot also provide explicit fanin patterns",
        }),
        MatchArity::Exact(value) if !fanins.is_empty() && value != fanins.len() => {
            Err(MatchError::InvalidArity {
                gate: gate.to_string(),
                reason: "exact arity must match explicit fanin pattern count",
            })
        }
        MatchArity::AtLeast(value) if !fanins.is_empty() && fanins.len() < value => {
            Err(MatchError::InvalidArity {
                gate: gate.to_string(),
                reason: "explicit fanin pattern count is below minimum arity",
            })
        }
        _ => Ok(()),
    }
}

fn primitive_accepts_arity(kind: PrimitiveGateKind, value: usize) -> bool {
    match kind {
        PrimitiveGateKind::One | PrimitiveGateKind::Zero => value == 0,
        PrimitiveGateKind::Buffer | PrimitiveGateKind::Inverter => value == 1,
        PrimitiveGateKind::And
        | PrimitiveGateKind::Nand
        | PrimitiveGateKind::Or
        | PrimitiveGateKind::Nor
        | PrimitiveGateKind::Xor
        | PrimitiveGateKind::Xnor => value >= 2,
    }
}

fn primitive_min_arity(kind: PrimitiveGateKind) -> usize {
    match kind {
        PrimitiveGateKind::One | PrimitiveGateKind::Zero => 0,
        PrimitiveGateKind::Buffer | PrimitiveGateKind::Inverter => 1,
        PrimitiveGateKind::And
        | PrimitiveGateKind::Nand
        | PrimitiveGateKind::Or
        | PrimitiveGateKind::Nor
        | PrimitiveGateKind::Xor
        | PrimitiveGateKind::Xnor => 2,
    }
}

fn validate_name(name: &str, kind: &'static str, max_name_length: usize) -> Result<(), MatchError> {
    if name.is_empty() {
        return Err(MatchError::EmptyName { kind });
    }
    if name.len() > max_name_length {
        return Err(MatchError::NameTooLong {
            kind,
            name: name.to_string(),
            max_name_length,
        });
    }
    Ok(())
}

fn match_gate_from_genlib(gate: &GenlibGate) -> Option<Result<MatchGate, MatchError>> {
    let expression = gate
        .output
        .expression
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != ';')
        .collect::<String>();
    let pattern = parse_genlib_pattern(&expression, &gate.pins)?;
    Some(MatchGate::new(gate.name.clone(), pattern, gate.area))
}

fn parse_genlib_pattern(
    expression: &str,
    pins: &[super::library::GenlibPin],
) -> Option<MatchPattern> {
    let expression = strip_wrapping_parentheses(expression);
    if expression == "CONST0" || expression == "0" {
        return Some(MatchPattern::gate(PrimitiveGateKind::Zero, Vec::new()));
    }
    if expression == "CONST1" || expression == "1" {
        return Some(MatchPattern::gate(PrimitiveGateKind::One, Vec::new()));
    }

    if pins.len() == 1 {
        let pin = pins[0].declared_name.as_str();
        if expression == pin {
            return Some(MatchPattern::gate(
                PrimitiveGateKind::Buffer,
                vec![MatchEdge::non_inverting(MatchPattern::boundary(pin))],
            ));
        }
        if expression == format!("!{pin}") {
            return Some(MatchPattern::gate(
                PrimitiveGateKind::Inverter,
                vec![MatchEdge::non_inverting(MatchPattern::boundary(pin))],
            ));
        }
    }

    parse_binary_genlib_pattern(expression, pins, '*', PrimitiveGateKind::And)
        .or_else(|| parse_binary_genlib_pattern(expression, pins, '+', PrimitiveGateKind::Or))
}

fn parse_binary_genlib_pattern(
    expression: &str,
    pins: &[super::library::GenlibPin],
    operator: char,
    kind: PrimitiveGateKind,
) -> Option<MatchPattern> {
    if pins.len() != 2 {
        return None;
    }
    let (left, right) = expression.split_once(operator)?;
    if left.contains(other_binary_operator(operator))
        || right.contains(other_binary_operator(operator))
    {
        return None;
    }

    let left = parse_pin_term(left, pins)?;
    let right = parse_pin_term(right, pins)?;
    Some(MatchPattern::gate(
        kind,
        vec![
            MatchEdge::new(left.0, MatchPattern::boundary(left.1)),
            MatchEdge::new(right.0, MatchPattern::boundary(right.1)),
        ],
    ))
}

fn other_binary_operator(operator: char) -> char {
    if operator == '*' { '+' } else { '*' }
}

fn parse_pin_term<'a>(
    term: &'a str,
    pins: &'a [super::library::GenlibPin],
) -> Option<(MatchPolarity, String)> {
    let (polarity, name) = term
        .strip_prefix('!')
        .map(|name| (MatchPolarity::Inverting, name))
        .unwrap_or((MatchPolarity::NonInverting, term));
    if pins.iter().any(|pin| pin.declared_name == name) {
        Some((polarity, name.to_string()))
    } else {
        None
    }
}

fn strip_wrapping_parentheses(value: &str) -> &str {
    let mut current = value;
    while current.starts_with('(') && current.ends_with(')') && current.len() >= 2 {
        current = &current[1..current.len() - 1];
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::tree::{MapperTreeFanin, PrimitiveGateKind};

    fn leaf(name: &str) -> MatchPattern {
        MatchPattern::boundary(name)
    }

    fn sample_tree() -> MapperTree {
        let mut tree = MapperTree::empty();
        let a = tree.add_leaf("a");
        let b = tree.add_leaf("b");
        let c = tree.add_leaf("c");
        let and = tree.add_gate(
            PrimitiveGateKind::And,
            vec![MapperTreeFanin::new(a), MapperTreeFanin::inverted(b)],
        );
        let root = tree.add_gate(
            PrimitiveGateKind::Or,
            vec![MapperTreeFanin::new(and), MapperTreeFanin::new(c)],
        );
        tree.set_root(root);
        tree.validate().unwrap();
        tree
    }

    fn sample_net_graph() -> (NetMatchGraph, NetMatchNodeId) {
        let mut graph = NetMatchGraph::empty();
        let a = graph.add_node("a", MatchNodeKind::PrimaryInput);
        let b = graph.add_node("b", MatchNodeKind::PrimaryInput);
        let and = graph.add_node("and", MatchNodeKind::Internal);
        let out = graph.add_node("out", MatchNodeKind::PrimaryOutput);
        graph.add_edge(a, and).unwrap();
        graph.add_edge(b, and).unwrap();
        graph.add_edge(and, out).unwrap();
        graph.validate().unwrap();
        (graph, and)
    }

    fn and2_primitive(isomorphic_sons: bool) -> PrimitiveMatchGraph {
        let mut graph = PrimitiveMatchGraph::empty();
        let root = graph.add_node("g", MatchNodeKind::Internal);
        let a = graph.add_node("a", MatchNodeKind::PrimaryInput);
        let b = graph.add_node("b", MatchNodeKind::PrimaryInput);
        let out = graph.add_node("out", MatchNodeKind::PrimaryOutput);
        graph.set_isomorphic_sons(root, isomorphic_sons).unwrap();
        graph.add_edge(root, None, MatchDirection::In).unwrap();
        graph.add_edge(a, Some(root), MatchDirection::In).unwrap();
        graph.add_edge(b, Some(root), MatchDirection::In).unwrap();
        graph
            .add_edge(out, Some(root), MatchDirection::Out)
            .unwrap();
        graph.validate().unwrap();
        graph
    }

    #[test]
    fn matches_polarity_and_arity_against_mapper_tree() {
        let library = MatchLibrary::new(vec![
            MatchGate::new(
                "or2",
                MatchPattern::gate(
                    PrimitiveGateKind::Or,
                    vec![
                        MatchEdge::non_inverting(MatchPattern::gate(
                            PrimitiveGateKind::And,
                            vec![
                                MatchEdge::non_inverting(leaf("a")),
                                MatchEdge::inverting(leaf("b")),
                            ],
                        )),
                        MatchEdge::non_inverting(leaf("c")),
                    ],
                ),
                2.0,
            )
            .unwrap(),
            MatchGate::new(
                "wrong_polarity",
                MatchPattern::gate(
                    PrimitiveGateKind::And,
                    vec![
                        MatchEdge::non_inverting(leaf("a")),
                        MatchEdge::non_inverting(leaf("b")),
                    ],
                ),
                1.0,
            )
            .unwrap(),
        ])
        .unwrap();

        let tree = sample_tree();
        let matches =
            matches_at_root(&tree, &library, tree.root(), MatchLimits::default()).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].gate, "or2");
        assert_eq!(
            matches[0]
                .frontier
                .iter()
                .map(|node| node.index())
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn orders_matches_by_cost_specificity_name_and_root() {
        let library = MatchLibrary::new(vec![
            MatchGate::new(
                "z_and",
                MatchPattern::gate(
                    PrimitiveGateKind::And,
                    vec![MatchEdge::any(leaf("a")), MatchEdge::any(leaf("b"))],
                ),
                1.0,
            )
            .unwrap(),
            MatchGate::new(
                "a_any_and",
                MatchPattern::gate_with_arity(
                    PrimitiveGateKind::And,
                    MatchArity::Exact(2),
                    Vec::new(),
                ),
                1.0,
            )
            .unwrap(),
            MatchGate::new(
                "cheap_or",
                MatchPattern::gate_with_arity(
                    PrimitiveGateKind::Or,
                    MatchArity::AtLeast(2),
                    Vec::new(),
                ),
                0.5,
            )
            .unwrap(),
        ])
        .unwrap();

        let matches = enumerate_tree_matches(&sample_tree(), &library, MatchLimits::default())
            .unwrap()
            .into_iter()
            .map(|item| item.gate)
            .collect::<Vec<_>>();

        assert_eq!(matches, vec!["cheap_or", "z_and", "a_any_and"]);
    }

    #[test]
    fn derives_simple_match_patterns_from_genlib() {
        let genlib = crate::ports::map::library::parse_genlib(concat!(
            "GATE and_inv 3 O=a*!b;\n",
            "PIN a NONINV 1 999 1 .2 1 .2\n",
            "PIN b INV 1 999 1 .2 1 .2\n",
            "GATE complex 9 O=a*b+c;\n",
            "PIN a NONINV 1 999 1 .2 1 .2\n",
            "PIN b NONINV 1 999 1 .2 1 .2\n",
            "PIN c NONINV 1 999 1 .2 1 .2\n",
        ))
        .unwrap();

        let library = MatchLibrary::from_genlib(&genlib).unwrap();

        assert_eq!(library.gates().len(), 1);
        assert_eq!(library.gates()[0].name, "and_inv");
        let matches =
            enumerate_tree_matches(&sample_tree(), &library, MatchLimits::default()).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].gate, "and_inv");
        assert_eq!(matches[0].root.index(), 3);
    }

    #[test]
    fn enumerates_complete_primitive_graph_bindings() {
        let (network, start) = sample_net_graph();
        let primitive = and2_primitive(false);

        let matches = enumerate_graph_matches(
            &network,
            start,
            &primitive,
            GraphMatchOptions::default(),
            MatchLimits::default(),
        )
        .unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches[0]
                .bindings
                .iter()
                .map(|binding| (binding.primitive.index(), binding.network.index()))
                .collect::<Vec<_>>(),
            vec![(0, 2), (1, 0), (2, 1), (3, 3)]
        );
        assert_eq!(
            matches[1]
                .bindings
                .iter()
                .map(|binding| (binding.primitive.index(), binding.network.index()))
                .collect::<Vec<_>>(),
            vec![(0, 2), (1, 1), (2, 0), (3, 3)]
        );
    }

    #[test]
    fn isomorphic_sons_choose_first_unbound_fanin() {
        let (network, start) = sample_net_graph();
        let primitive = and2_primitive(true);

        let matches = enumerate_graph_matches(
            &network,
            start,
            &primitive,
            GraphMatchOptions::default(),
            MatchLimits::default(),
        )
        .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0]
                .bindings
                .iter()
                .map(|binding| (binding.primitive.index(), binding.network.index()))
                .collect::<Vec<_>>(),
            vec![(0, 2), (1, 0), (2, 1), (3, 3)]
        );
    }

    #[test]
    fn applies_internal_fanout_policy() {
        let (mut network, start) = sample_net_graph();
        let extra = network.add_node("extra", MatchNodeKind::PrimaryOutput);
        network.add_edge(start, extra).unwrap();
        let primitive = and2_primitive(true);

        let exact = enumerate_graph_matches(
            &network,
            start,
            &primitive,
            GraphMatchOptions::default(),
            MatchLimits::default(),
        )
        .unwrap();
        let limited = enumerate_graph_matches(
            &network,
            start,
            &primitive,
            GraphMatchOptions {
                internal_fanout_policy: InternalFanoutPolicy::UpToLimit(2),
            },
            MatchLimits::default(),
        )
        .unwrap();

        assert!(exact.is_empty());
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("match.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
