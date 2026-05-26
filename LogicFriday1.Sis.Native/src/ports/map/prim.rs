//! Native Rust primitive-gate support for `sis/map/prim.c`.
//!
//! The original file built the mapper's `prim_t` graph from SIS `network_t`
//! objects. The native port keeps the behavior needed by the Rust mapper as
//! owned primitive descriptors: classify genlib functions, construct virtual
//! gates, and keep explicit fanin arity validation. It intentionally exposes no
//! legacy C ABI entry points.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use super::library::GenlibGate;
use super::virtual_net::{GateKind, NodeId, NodeKind, SourceRef, VirtualMappedNetwork};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PrimitiveNodeId(usize);

impl PrimitiveNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveEdgeDirection {
    In,
    Out,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveNode {
    pub name: String,
    pub kind: PrimitiveNodeKind,
    pub nfanin: usize,
    pub nfanout: usize,
    pub isomorphic_sons: bool,
    pub latch_output: bool,
    pub binding: Option<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveEdge {
    pub this_node: PrimitiveNodeId,
    pub connected_node: Option<PrimitiveNodeId>,
    pub direction: PrimitiveEdgeDirection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveNetwork {
    nodes: Vec<PrimitiveNode>,
    inputs: Vec<PrimitiveNodeId>,
    outputs: Vec<PrimitiveNodeId>,
    edges: Vec<PrimitiveEdge>,
    matched_gates: Vec<String>,
}

impl PrimitiveNetwork {
    pub fn from_virtual_network(network: &VirtualMappedNetwork) -> Result<Self, PrimitiveError> {
        if network.outputs().is_empty() {
            return Err(PrimitiveError::NoOutputs);
        }
        if network.outputs().len() != 1 {
            return Err(PrimitiveError::NotSingleOutput {
                outputs: network.outputs().len(),
            });
        }

        let fanouts = virtual_fanouts(network)?;
        let output_drivers = output_drivers(network)?;
        let isomorphic_sons = find_isomorphic_sons(network);
        let mut primitive = Self {
            nodes: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            edges: Vec::new(),
            matched_gates: Vec::new(),
        };
        let mut mapping = vec![None; network.nodes().len()];

        for (index, node) in network.nodes().iter().enumerate() {
            if node.kind == NodeKind::PrimaryOutput || !is_active_primitive_node(node) {
                continue;
            }

            let mut kind = match node.kind {
                NodeKind::PrimaryInput => PrimitiveNodeKind::PrimaryInput,
                NodeKind::PrimaryOutput => unreachable!(),
                NodeKind::Internal => PrimitiveNodeKind::Internal,
            };
            let mut name = node.name.clone();
            if let Some(output_name) = output_drivers.get(&index) {
                kind = PrimitiveNodeKind::PrimaryOutput;
                name = output_name.clone();
            }

            let primitive_id = PrimitiveNodeId(primitive.nodes.len());
            primitive.nodes.push(PrimitiveNode {
                name,
                kind,
                nfanin: primitive_fanin_count(node),
                nfanout: fanouts.get(&index).map_or(0, Vec::len),
                isomorphic_sons: isomorphic_sons.contains(&index),
                latch_output: false,
                binding: None,
            });
            mapping[index] = Some(primitive_id);
        }

        for input in network.inputs() {
            let Some(primitive_input) = mapping.get(input.index()).and_then(|id| *id) else {
                return Err(PrimitiveError::MissingPrimitiveNode {
                    node: input.index(),
                });
            };
            primitive.inputs.push(primitive_input);
        }

        let output = network.outputs()[0];
        let output_node = network
            .node(output)
            .ok_or(PrimitiveError::MissingPrimitiveNode {
                node: output.index(),
            })?;
        let SourceRef::Node(output_driver) = only_fanin(output.index(), output_node)? else {
            return Err(PrimitiveError::ConstantOutput);
        };
        let Some(primitive_output) = mapping.get(output_driver.index()).and_then(|id| *id) else {
            return Err(PrimitiveError::MissingPrimitiveNode {
                node: output_driver.index(),
            });
        };
        primitive.outputs.push(primitive_output);

        let mut visited = BTreeSet::new();
        let mut visited_arcs = BTreeSet::new();
        reorder_virtual(
            network,
            output.index(),
            None,
            PrimitiveEdgeDirection::In,
            &mapping,
            &mut visited,
            &mut visited_arcs,
            &mut primitive.edges,
        )?;

        for (index, node) in network.nodes().iter().enumerate() {
            if is_reachable_required_node(node) && !visited.contains(&index) {
                return Err(PrimitiveError::DisconnectedNetwork);
            }
        }

        Ok(primitive)
    }

    pub fn nodes(&self) -> &[PrimitiveNode] {
        &self.nodes
    }

    pub fn inputs(&self) -> &[PrimitiveNodeId] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[PrimitiveNodeId] {
        &self.outputs
    }

    pub fn edges(&self) -> &[PrimitiveEdge] {
        &self.edges
    }

    pub fn matched_gates(&self) -> &[String] {
        &self.matched_gates
    }

    pub fn add_matched_gate(&mut self, gate: impl Into<String>) {
        self.matched_gates.push(gate.into());
    }

    pub fn root(&self) -> Option<PrimitiveNodeId> {
        self.edges.first().map(|edge| edge.this_node)
    }

    pub fn clear_bindings(&mut self) {
        for node in &mut self.nodes {
            node.binding = None;
        }
    }

    pub fn format_dump(&self, detail: bool) -> String {
        let mut output = String::new();
        output.push_str("matches:");
        for gate in &self.matched_gates {
            output.push(' ');
            output.push_str(gate);
        }
        output.push_str(" [type=COMBINATIONAL]\n");

        if detail {
            output.push_str(&format!("ninputs={}\n", self.inputs.len()));
            for input in &self.inputs {
                self.write_dump_node(&mut output, *input);
            }
            output.push_str(&format!("noutputs={}\n", self.outputs.len()));
            for output_node in &self.outputs {
                self.write_dump_node(&mut output, *output_node);
            }
            output.push_str("nodes ...\n");
            for index in 0..self.nodes.len() {
                self.write_dump_node(&mut output, PrimitiveNodeId(index));
            }
            output.push_str("edges ...\n");
            for edge in &self.edges {
                output.push_str(&format!(
                    "    this_node={} connected={} dir={}\n",
                    edge.this_node.index(),
                    edge.connected_node
                        .map(|node| node.index().to_string())
                        .unwrap_or_else(|| "none".to_string()),
                    match edge.direction {
                        PrimitiveEdgeDirection::In => "IN",
                        PrimitiveEdgeDirection::Out => "OUT",
                    }
                ));
            }
        }

        output
    }

    fn write_dump_node(&self, output: &mut String, node: PrimitiveNodeId) {
        let item = &self.nodes[node.index()];
        output.push_str(&format!(
            "\tname={:<10} iso={} nfanin={:2} nfanout={:2} type={}\n\tlatch_output={}\n",
            item.name,
            usize::from(item.isomorphic_sons),
            item.nfanin,
            item.nfanout,
            match item.kind {
                PrimitiveNodeKind::PrimaryInput => "PI",
                PrimitiveNodeKind::PrimaryOutput => "PO",
                PrimitiveNodeKind::Internal => "INT",
            },
            usize::from(item.latch_output)
        ));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PrimitiveKind {
    Inverter,
    Nand,
    Nor,
    Xor,
    Xnor,
    Mux,
    And,
    Or,
    One,
    Zero,
    Wire,
}

impl PrimitiveKind {
    pub fn gate_kind(self) -> GateKind {
        match self {
            Self::Inverter => GateKind::Inverter,
            Self::Nand => GateKind::Nand,
            Self::Nor => GateKind::Nor,
            Self::Xor => GateKind::Xor,
            Self::Xnor => GateKind::Xnor,
            Self::Mux => GateKind::Mux,
            Self::And => GateKind::And,
            Self::Or => GateKind::Or,
            Self::One => GateKind::One,
            Self::Zero => GateKind::Zero,
            Self::Wire => GateKind::Wire,
        }
    }

    pub fn arity_rule(self) -> PrimitiveArity {
        match self {
            Self::One | Self::Zero => PrimitiveArity::Exact(0),
            Self::Inverter | Self::Wire => PrimitiveArity::Exact(1),
            Self::Mux => PrimitiveArity::Exact(3),
            Self::Nand | Self::Nor | Self::Xor | Self::Xnor | Self::And | Self::Or => {
                PrimitiveArity::AtLeast(2)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveArity {
    Exact(usize),
    AtLeast(usize),
}

impl PrimitiveArity {
    pub fn accepts(self, arity: usize) -> bool {
        match self {
            Self::Exact(expected) => arity == expected,
            Self::AtLeast(minimum) => arity >= minimum,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveGate {
    pub name: String,
    pub kind: PrimitiveKind,
    pub input_names: Vec<String>,
    pub area: Option<f64>,
}

impl PrimitiveGate {
    pub fn new(
        name: impl Into<String>,
        kind: PrimitiveKind,
        input_names: Vec<String>,
    ) -> Result<Self, PrimitiveError> {
        Self::with_area(name, kind, input_names, None)
    }

    pub fn with_area(
        name: impl Into<String>,
        kind: PrimitiveKind,
        input_names: Vec<String>,
        area: Option<f64>,
    ) -> Result<Self, PrimitiveError> {
        let gate = Self {
            name: name.into(),
            kind,
            input_names,
            area,
        };
        gate.validate()?;
        Ok(gate)
    }

    pub fn from_genlib(gate: &GenlibGate) -> Result<Self, PrimitiveError> {
        let pin_names = gate
            .pins
            .iter()
            .map(|pin| pin.declared_name.clone())
            .collect::<Vec<_>>();
        let kind = infer_genlib_kind(gate)?;
        Self::with_area(gate.name.clone(), kind, pin_names, Some(gate.area))
    }

    pub fn validate(&self) -> Result<(), PrimitiveError> {
        if self.name.trim().is_empty() {
            return Err(PrimitiveError::EmptyName);
        }
        if let Some(area) = self.area {
            if !area.is_finite() || area < 0.0 {
                return Err(PrimitiveError::InvalidArea { area });
            }
        }
        if !self.kind.arity_rule().accepts(self.input_names.len()) {
            return Err(PrimitiveError::InvalidArity {
                kind: self.kind,
                arity: self.input_names.len(),
            });
        }
        if self.input_names.iter().any(|name| name.trim().is_empty()) {
            return Err(PrimitiveError::EmptyInputName);
        }

        Ok(())
    }

    pub fn add_to_virtual_network(
        &self,
        network: &mut VirtualMappedNetwork,
        output_name: impl Into<String>,
        fanins: Vec<SourceRef>,
    ) -> Result<super::virtual_net::NodeId, PrimitiveError> {
        if !self.kind.arity_rule().accepts(fanins.len()) {
            return Err(PrimitiveError::InvalidArity {
                kind: self.kind,
                arity: fanins.len(),
            });
        }

        Ok(network.add_gate(output_name, self.kind.gate_kind(), fanins))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveError {
    EmptyName,
    EmptyInputName,
    NoOutputs,
    NotSingleOutput { outputs: usize },
    ConstantOutput,
    DisconnectedNetwork,
    MissingPrimitiveNode { node: usize },
    InvalidPrimaryOutputFanin { node: usize, fanins: usize },
    InvalidArea { area: f64 },
    InvalidArity { kind: PrimitiveKind, arity: usize },
    ParseExpression { expression: String, message: String },
    UnsupportedFunction { gate: String, expression: String },
}

impl fmt::Display for PrimitiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "primitive gate name must not be empty"),
            Self::EmptyInputName => write!(f, "primitive gate input name must not be empty"),
            Self::NoOutputs => write!(f, "primitive network must have at least one output"),
            Self::NotSingleOutput { outputs } => {
                write!(f, "primitive network supports one output, got {outputs}")
            }
            Self::ConstantOutput => {
                write!(f, "primitive network output must be driven by a node")
            }
            Self::DisconnectedNetwork => write!(f, "primitive network is not connected"),
            Self::MissingPrimitiveNode { node } => {
                write!(f, "missing primitive node for virtual node {node}")
            }
            Self::InvalidPrimaryOutputFanin { node, fanins } => {
                write!(f, "primary output {node} must have one fanin, got {fanins}")
            }
            Self::InvalidArea { area } => write!(f, "primitive gate area {area} is invalid"),
            Self::InvalidArity { kind, arity } => {
                write!(f, "primitive gate {kind:?} does not accept {arity} inputs")
            }
            Self::ParseExpression {
                expression,
                message,
            } => {
                write!(
                    f,
                    "could not parse genlib expression '{expression}': {message}"
                )
            }
            Self::UnsupportedFunction { gate, expression } => {
                write!(
                    f,
                    "gate '{gate}' has unsupported primitive function '{expression}'"
                )
            }
        }
    }
}

impl Error for PrimitiveError {}

pub fn classify_genlib_gate(gate: &GenlibGate) -> Result<PrimitiveKind, PrimitiveError> {
    infer_genlib_kind(gate)
}

pub fn classify_expression(
    expression: &str,
    input_names: &[String],
) -> Result<PrimitiveKind, PrimitiveError> {
    let parsed =
        Parser::new(expression)
            .parse()
            .map_err(|message| PrimitiveError::ParseExpression {
                expression: expression.to_string(),
                message,
            })?;
    infer_expression_kind(&parsed, input_names).ok_or_else(|| PrimitiveError::UnsupportedFunction {
        gate: "<expression>".to_string(),
        expression: expression.to_string(),
    })
}

fn infer_genlib_kind(gate: &GenlibGate) -> Result<PrimitiveKind, PrimitiveError> {
    let input_names = gate
        .pins
        .iter()
        .map(|pin| pin.declared_name.clone())
        .collect::<Vec<_>>();

    classify_expression(&gate.output.expression, &input_names).map_err(|error| match error {
        PrimitiveError::UnsupportedFunction { .. } => PrimitiveError::UnsupportedFunction {
            gate: gate.name.clone(),
            expression: gate.output.expression.clone(),
        },
        other => other,
    })
}

fn infer_expression_kind(expression: &Expression, input_names: &[String]) -> Option<PrimitiveKind> {
    match expression {
        Expression::Const(false) => Some(PrimitiveKind::Zero),
        Expression::Const(true) => Some(PrimitiveKind::One),
        Expression::Variable(name) if input_names.len() == 1 && input_names[0] == *name => {
            Some(PrimitiveKind::Wire)
        }
        Expression::Not(inner) => match inner.as_ref() {
            Expression::Variable(name) if input_names.len() == 1 && input_names[0] == *name => {
                Some(PrimitiveKind::Inverter)
            }
            Expression::And(terms) if terms_cover_inputs(terms, input_names) => {
                Some(PrimitiveKind::Nand)
            }
            Expression::Or(terms) if terms_cover_inputs(terms, input_names) => {
                Some(PrimitiveKind::Nor)
            }
            Expression::Xor(terms) if terms_cover_inputs(terms, input_names) => {
                Some(PrimitiveKind::Xnor)
            }
            _ => None,
        },
        Expression::And(terms) if terms_cover_inputs(terms, input_names) => {
            Some(PrimitiveKind::And)
        }
        Expression::Or(terms) if terms_cover_inputs(terms, input_names) => Some(PrimitiveKind::Or),
        Expression::Xor(terms) if terms_cover_inputs(terms, input_names) => {
            Some(PrimitiveKind::Xor)
        }
        Expression::Or(terms) if is_mux(terms, input_names) => Some(PrimitiveKind::Mux),
        _ => None,
    }
}

fn virtual_fanouts(
    network: &VirtualMappedNetwork,
) -> Result<BTreeMap<usize, Vec<usize>>, PrimitiveError> {
    let mut fanouts = BTreeMap::<usize, Vec<usize>>::new();
    for (index, node) in network.nodes().iter().enumerate() {
        for source in primitive_fanins(index, node)? {
            if let SourceRef::Node(source) = source {
                fanouts.entry(source.index()).or_default().push(index);
            }
        }
    }

    Ok(fanouts)
}

fn output_drivers(
    network: &VirtualMappedNetwork,
) -> Result<BTreeMap<usize, String>, PrimitiveError> {
    let mut drivers = BTreeMap::new();
    for output in network.outputs() {
        let node = network
            .node(*output)
            .ok_or(PrimitiveError::MissingPrimitiveNode {
                node: output.index(),
            })?;
        let SourceRef::Node(driver) = only_fanin(output.index(), node)? else {
            continue;
        };
        drivers.insert(driver.index(), node.name.clone());
    }

    Ok(drivers)
}

fn reorder_virtual(
    network: &VirtualMappedNetwork,
    node: usize,
    previous: Option<usize>,
    direction: PrimitiveEdgeDirection,
    mapping: &[Option<PrimitiveNodeId>],
    visited: &mut BTreeSet<usize>,
    visited_arcs: &mut BTreeSet<(usize, usize)>,
    edges: &mut Vec<PrimitiveEdge>,
) -> Result<(), PrimitiveError> {
    let virtual_node = network
        .nodes()
        .get(node)
        .ok_or(PrimitiveError::DisconnectedNetwork)?;

    if virtual_node.kind != NodeKind::PrimaryOutput {
        let this_node = mapping
            .get(node)
            .and_then(|primitive_node| *primitive_node)
            .ok_or(PrimitiveError::DisconnectedNetwork)?;
        let connected_node = previous.and_then(|previous| {
            mapping
                .get(previous)
                .and_then(|primitive_node| *primitive_node)
        });
        edges.push(PrimitiveEdge {
            this_node,
            connected_node,
            direction,
        });
    }

    if !visited.insert(node) {
        return Ok(());
    }

    for source in primitive_fanins(node, virtual_node)? {
        if let SourceRef::Node(fanin) = source {
            if visited_arcs.insert((fanin.index(), node)) {
                reorder_virtual(
                    network,
                    fanin.index(),
                    Some(node),
                    PrimitiveEdgeDirection::In,
                    mapping,
                    visited,
                    visited_arcs,
                    edges,
                )?;
            }
        }
    }

    for fanout in virtual_fanout_nodes(network, node)? {
        if visited_arcs.insert((node, fanout)) {
            reorder_virtual(
                network,
                fanout,
                Some(node),
                PrimitiveEdgeDirection::Out,
                mapping,
                visited,
                visited_arcs,
                edges,
            )?;
        }
    }

    Ok(())
}

fn virtual_fanout_nodes(
    network: &VirtualMappedNetwork,
    source: usize,
) -> Result<Vec<usize>, PrimitiveError> {
    let mut fanouts = Vec::new();
    for (index, node) in network.nodes().iter().enumerate() {
        if primitive_fanins(index, node)?
            .iter()
            .any(|fanin| matches!(fanin, SourceRef::Node(fanin) if fanin.index() == source))
        {
            fanouts.push(index);
        }
    }

    Ok(fanouts)
}

fn primitive_fanins(
    node_id: usize,
    node: &super::virtual_net::VirtualMappedNode,
) -> Result<&[SourceRef], PrimitiveError> {
    match node.kind {
        NodeKind::PrimaryInput => Ok(&[]),
        NodeKind::PrimaryOutput => {
            if node.save_binding.len() != 1 {
                return Err(PrimitiveError::InvalidPrimaryOutputFanin {
                    node: node_id,
                    fanins: node.save_binding.len(),
                });
            }
            Ok(&node.save_binding)
        }
        NodeKind::Internal => Ok(&node.save_binding),
    }
}

fn only_fanin(
    node_id: usize,
    node: &super::virtual_net::VirtualMappedNode,
) -> Result<SourceRef, PrimitiveError> {
    if node.save_binding.len() != 1 {
        return Err(PrimitiveError::InvalidPrimaryOutputFanin {
            node: node_id,
            fanins: node.save_binding.len(),
        });
    }

    Ok(node.save_binding[0])
}

fn primitive_fanin_count(node: &super::virtual_net::VirtualMappedNode) -> usize {
    match node.kind {
        NodeKind::PrimaryInput => 0,
        NodeKind::PrimaryOutput | NodeKind::Internal => node.save_binding.len(),
    }
}

fn is_active_primitive_node(node: &super::virtual_net::VirtualMappedNode) -> bool {
    node.kind != NodeKind::Internal || node.gate.is_some()
}

fn is_reachable_required_node(node: &super::virtual_net::VirtualMappedNode) -> bool {
    node.kind != NodeKind::Internal || node.gate.is_some()
}

fn find_isomorphic_sons(network: &VirtualMappedNetwork) -> BTreeSet<usize> {
    let mut result = BTreeSet::new();
    for (index, node) in network.nodes().iter().enumerate() {
        let mut signatures = BTreeSet::new();
        let mut has_duplicate = false;
        for fanin in &node.save_binding {
            let SourceRef::Node(fanin) = fanin else {
                continue;
            };
            let signature = structural_signature(network, *fanin, &mut BTreeMap::new());
            if !signatures.insert(signature) {
                has_duplicate = true;
                break;
            }
        }
        if has_duplicate {
            result.insert(index);
        }
    }

    result
}

fn structural_signature(
    network: &VirtualMappedNetwork,
    node: NodeId,
    cache: &mut BTreeMap<NodeId, String>,
) -> String {
    if let Some(signature) = cache.get(&node) {
        return signature.clone();
    }

    let signature = match network.node(node) {
        Some(item) if item.kind == NodeKind::PrimaryInput => format!("pi:{}", item.name),
        Some(item) => {
            let gate = item
                .gate
                .as_ref()
                .map_or_else(|| "none".to_string(), |gate| format!("{gate:?}"));
            let fanins = item
                .save_binding
                .iter()
                .map(|source| match source {
                    SourceRef::ConstantZero => "0".to_string(),
                    SourceRef::ConstantOne => "1".to_string(),
                    SourceRef::Node(source) => structural_signature(network, *source, cache),
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{gate}({fanins})")
        }
        None => "missing".to_string(),
    };
    cache.insert(node, signature.clone());
    signature
}

fn terms_cover_inputs(terms: &[Expression], input_names: &[String]) -> bool {
    if terms.len() != input_names.len() || terms.len() < 2 {
        return false;
    }

    input_names.iter().all(|input| {
        terms
            .iter()
            .any(|term| matches!(term, Expression::Variable(name) if name == input))
    })
}

fn is_mux(terms: &[Expression], input_names: &[String]) -> bool {
    if input_names.len() != 3 || terms.len() != 2 {
        return false;
    }

    mux_product_candidates(&terms[0])
        .iter()
        .any(|(select, left_data, left_inverted)| {
            mux_product_candidates(&terms[1]).iter().any(
                |(right_select, right_data, right_inverted)| {
                    if select != right_select || left_inverted == right_inverted {
                        return false;
                    }

                    let select_present = input_names.iter().any(|name| name == select);
                    let left_present = input_names.iter().any(|name| name == left_data);
                    let right_present = input_names.iter().any(|name| name == right_data);

                    select_present && left_present && right_present && left_data != right_data
                },
            )
        })
}

fn mux_product_candidates(expression: &Expression) -> Vec<(&str, &str, bool)> {
    let Expression::And(terms) = expression else {
        return Vec::new();
    };
    if terms.len() != 2 {
        return Vec::new();
    }

    match (&terms[0], &terms[1]) {
        (Expression::Variable(data), Expression::Not(select))
        | (Expression::Not(select), Expression::Variable(data)) => match select.as_ref() {
            Expression::Variable(select) => vec![(select.as_str(), data.as_str(), true)],
            _ => Vec::new(),
        },
        (Expression::Variable(left), Expression::Variable(right)) => vec![
            (left.as_str(), right.as_str(), false),
            (right.as_str(), left.as_str(), false),
        ],
        _ => Vec::new(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Expression {
    Const(bool),
    Variable(String),
    Not(Box<Expression>),
    And(Vec<Expression>),
    Or(Vec<Expression>),
    Xor(Vec<Expression>),
}

struct Parser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn parse(mut self) -> Result<Expression, String> {
        let expression = self.parse_or()?;
        self.skip_whitespace();
        if self.peek().is_some() {
            return Err(format!("unexpected token at byte {}", self.position));
        }

        Ok(expression)
    }

    fn parse_or(&mut self) -> Result<Expression, String> {
        let mut terms = vec![self.parse_xor()?];
        while self.consume('+') || self.consume('|') {
            terms.push(self.parse_xor()?);
        }

        Ok(flatten_or(terms))
    }

    fn parse_xor(&mut self) -> Result<Expression, String> {
        let mut terms = vec![self.parse_and()?];
        while self.consume('^') {
            terms.push(self.parse_and()?);
        }

        Ok(flatten_xor(terms))
    }

    fn parse_and(&mut self) -> Result<Expression, String> {
        let mut terms = vec![self.parse_not()?];
        while self.consume('*') || self.consume('&') {
            terms.push(self.parse_not()?);
        }

        Ok(flatten_and(terms))
    }

    fn parse_not(&mut self) -> Result<Expression, String> {
        if self.consume('!') || self.consume('~') {
            return Ok(Expression::Not(Box::new(self.parse_not()?)));
        }

        let mut expression = self.parse_primary()?;
        while self.consume('\'') {
            expression = Expression::Not(Box::new(expression));
        }

        Ok(expression)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();
        if self.consume('(') {
            let expression = self.parse_or()?;
            if !self.consume(')') {
                return Err(format!("expected ')' at byte {}", self.position));
            }
            return Ok(expression);
        }

        let token = self.parse_token()?;
        match token.as_str() {
            "0" | "CONST0" | "const0" => Ok(Expression::Const(false)),
            "1" | "CONST1" | "const1" => Ok(Expression::Const(true)),
            _ => Ok(Expression::Variable(token)),
        }
    }

    fn parse_token(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        let start = self.position;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '$' {
                self.position += ch.len_utf8();
            } else {
                break;
            }
        }

        if self.position == start {
            Err(format!("expected variable or constant at byte {start}"))
        } else {
            Ok(self.input[start..self.position].to_string())
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        self.skip_whitespace();
        if self.peek() == Some(expected) {
            self.position += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.position += 1;
        }
    }
}

fn flatten_and(terms: Vec<Expression>) -> Expression {
    flatten(terms, Expression::And)
}

fn flatten_or(terms: Vec<Expression>) -> Expression {
    flatten(terms, Expression::Or)
}

fn flatten_xor(terms: Vec<Expression>) -> Expression {
    flatten(terms, Expression::Xor)
}

fn flatten(
    terms: Vec<Expression>,
    constructor: impl FnOnce(Vec<Expression>) -> Expression,
) -> Expression {
    if terms.len() == 1 {
        terms.into_iter().next().unwrap()
    } else {
        constructor(terms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::library::{GenlibOutput, GenlibPin, GenlibPinName, PinPhase};
    use crate::ports::map::virtual_net::SourceRef;

    fn pins(names: &[&str]) -> Vec<GenlibPin> {
        names
            .iter()
            .map(|name| GenlibPin {
                name: GenlibPinName::Declared((*name).to_string()),
                declared_name: (*name).to_string(),
                phase: PinPhase::Unknown,
                input_load: 1.0,
                max_load: 999.0,
                rise_block_delay: 1.0,
                rise_fanout_delay: 0.2,
                fall_block_delay: 1.0,
                fall_fanout_delay: 0.2,
            })
            .collect()
    }

    fn genlib_gate(name: &str, expression: &str, inputs: &[&str]) -> GenlibGate {
        GenlibGate::new(name, 1.0, GenlibOutput::new("O", expression), pins(inputs)).unwrap()
    }

    #[test]
    fn classifies_common_genlib_primitives() {
        let cases = [
            ("inv", "!a", &["a"][..], PrimitiveKind::Inverter),
            ("nand", "!(a*b)", &["a", "b"][..], PrimitiveKind::Nand),
            ("nor", "!(a+b)", &["a", "b"][..], PrimitiveKind::Nor),
            ("xor", "a^b", &["a", "b"][..], PrimitiveKind::Xor),
            ("xnor", "!(a^b)", &["a", "b"][..], PrimitiveKind::Xnor),
            ("and", "a*b", &["a", "b"][..], PrimitiveKind::And),
            ("or", "a+b", &["a", "b"][..], PrimitiveKind::Or),
            ("one", "CONST1", &[][..], PrimitiveKind::One),
            ("zero", "0", &[][..], PrimitiveKind::Zero),
            ("wire", "a", &["a"][..], PrimitiveKind::Wire),
        ];

        for (name, expression, inputs, expected) in cases {
            let gate = genlib_gate(name, expression, inputs);

            assert_eq!(classify_genlib_gate(&gate).unwrap(), expected);
            assert_eq!(PrimitiveGate::from_genlib(&gate).unwrap().kind, expected);
        }
    }

    #[test]
    fn classifies_two_product_mux() {
        let gate = genlib_gate("mux2", "a*!s+b*s", &["s", "a", "b"]);

        assert_eq!(classify_genlib_gate(&gate).unwrap(), PrimitiveKind::Mux);
    }

    #[test]
    fn rejects_unsupported_compound_function() {
        let gate = genlib_gate("complex", "a*b+c", &["a", "b", "c"]);

        assert!(matches!(
            classify_genlib_gate(&gate),
            Err(PrimitiveError::UnsupportedFunction { .. })
        ));
    }

    #[test]
    fn validates_primitive_arity() {
        assert!(PrimitiveGate::new("bad_inv", PrimitiveKind::Inverter, vec![]).is_err());
        assert!(
            PrimitiveGate::new(
                "and2",
                PrimitiveKind::And,
                vec!["a".to_string(), "b".to_string()]
            )
            .is_ok()
        );
    }

    #[test]
    fn constructs_virtual_network_gate() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let gate =
            PrimitiveGate::new("inv", PrimitiveKind::Inverter, vec!["a".to_string()]).unwrap();

        let node = gate
            .add_to_virtual_network(&mut network, "not_a", vec![SourceRef::Node(a)])
            .unwrap();

        assert_eq!(network.node(node).unwrap().gate, Some(GateKind::Inverter));
    }

    #[test]
    fn builds_primitive_network_from_virtual_network() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let gate = network.add_gate(
            "n1",
            GateKind::Nand,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        network
            .add_primary_output("f", SourceRef::Node(gate))
            .unwrap();

        let primitive = PrimitiveNetwork::from_virtual_network(&network).unwrap();

        assert_eq!(primitive.inputs().len(), 2);
        assert_eq!(primitive.outputs().len(), 1);
        assert_eq!(primitive.nodes().len(), 3);
        assert_eq!(primitive.edges().len(), 3);
        assert_eq!(
            primitive.nodes()[primitive.root().unwrap().index()].kind,
            PrimitiveNodeKind::PrimaryOutput
        );
        assert_eq!(
            primitive.nodes()[primitive.outputs()[0].index()].name,
            "f".to_string()
        );
    }

    #[test]
    fn rejects_disconnected_primitive_network() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        network.add_primary_input("unused");
        let gate = network.add_gate("n1", GateKind::Inverter, vec![SourceRef::Node(a)]);
        network
            .add_primary_output("f", SourceRef::Node(gate))
            .unwrap();

        assert!(matches!(
            PrimitiveNetwork::from_virtual_network(&network),
            Err(PrimitiveError::DisconnectedNetwork)
        ));
    }

    #[test]
    fn marks_nodes_with_isomorphic_sons() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let left = network.add_gate("left", GateKind::Inverter, vec![SourceRef::Node(a)]);
        let right = network.add_gate("right", GateKind::Inverter, vec![SourceRef::Node(a)]);
        let root = network.add_gate(
            "root",
            GateKind::And,
            vec![SourceRef::Node(left), SourceRef::Node(right)],
        );
        network
            .add_primary_output("f", SourceRef::Node(root))
            .unwrap();

        let primitive = PrimitiveNetwork::from_virtual_network(&network).unwrap();

        assert!(
            primitive
                .nodes()
                .iter()
                .any(|node| node.name == "f" && node.isomorphic_sons)
        );
    }

    #[test]
    fn rejects_multi_output_primitive_network() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let gate = network.add_gate("n1", GateKind::Inverter, vec![SourceRef::Node(a)]);
        network
            .add_primary_output("f", SourceRef::Node(gate))
            .unwrap();
        network
            .add_primary_output("g", SourceRef::Node(gate))
            .unwrap();

        assert!(matches!(
            PrimitiveNetwork::from_virtual_network(&network),
            Err(PrimitiveError::NotSingleOutput { outputs: 2 })
        ));
    }
}
