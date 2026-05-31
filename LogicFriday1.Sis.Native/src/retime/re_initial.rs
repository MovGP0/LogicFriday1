//! Native Rust model for `LogicSynthesis/sis/retime/re_initial.c`.
//!
//! The original C file updates latch initial values after retiming. The part
//! that operates on an already-built retime graph is modeled here with owned
//! Rust vectors and indices. The top-level SIS workflow is still blocked on
//! native network, latch, node, and sequential-BDD ports; those dependencies are
//! reported explicitly instead of exposing legacy C ABI shims.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub fn retime_update_init_states_from_sis_blocked() -> Result<(), ReInitialError> {
    Err(ReInitialError::MissingSisDependencies {
        operation: "retime_update_init_states",
    })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EdgeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LatchId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeType {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CubeLiteral {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Const0,
    Const1,
    SumOfProducts(Vec<Vec<CubeLiteral>>),
}

impl NodeFunction {
    pub fn evaluate(&self, inputs: &[i32]) -> Result<i32, ReInitialError> {
        match self {
            Self::Const0 => Ok(0),
            Self::Const1 => Ok(1),
            Self::SumOfProducts(cubes) => {
                let mut output = TernaryBit::Unknown;
                for cube in cubes {
                    if cube.len() != inputs.len() {
                        return Err(ReInitialError::FunctionArityMismatch {
                            expected: cube.len(),
                            actual: inputs.len(),
                        });
                    }

                    let mut local = TernaryBit::Unknown;
                    for (literal, input) in cube.iter().zip(inputs) {
                        let input = BinaryValue::try_from_i32(*input)?;
                        local = match literal {
                            CubeLiteral::Zero => local.and(input.invert().into()),
                            CubeLiteral::One => local.and(input.into()),
                            CubeLiteral::DontCare => local,
                        };
                        if local == TernaryBit::Zero {
                            break;
                        }
                    }

                    if local == TernaryBit::Unknown {
                        return Err(ReInitialError::EmptyProductCube);
                    }

                    output = output.or(local);
                    if output == TernaryBit::One {
                        break;
                    }
                }
                output.to_binary()
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BinaryValue {
    Zero,
    One,
}

impl BinaryValue {
    fn try_from_i32(value: i32) -> Result<Self, ReInitialError> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            _ => Err(ReInitialError::InvalidBinaryValue(value)),
        }
    }

    fn invert(self) -> Self {
        match self {
            Self::Zero => Self::One,
            Self::One => Self::Zero,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TernaryBit {
    Zero,
    One,
    Unknown,
}

impl TernaryBit {
    fn and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::Zero, _) | (_, Self::Zero) => Self::Zero,
            (Self::One, Self::One) => Self::One,
            (Self::Unknown, Self::One) | (Self::One, Self::Unknown) => Self::One,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        }
    }

    fn or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::One, _) | (_, Self::One) => Self::One,
            (Self::Zero, Self::Zero) => Self::Zero,
            (Self::Unknown, Self::Zero) | (Self::Zero, Self::Unknown) => Self::Zero,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        }
    }

    fn to_binary(self) -> Result<i32, ReInitialError> {
        match self {
            Self::Zero => Ok(0),
            Self::One => Ok(1),
            Self::Unknown => Err(ReInitialError::EmptyCover),
        }
    }
}

impl From<BinaryValue> for TernaryBit {
    fn from(value: BinaryValue) -> Self {
        match value {
            BinaryValue::Zero => Self::Zero,
            BinaryValue::One => Self::One,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetimeNode {
    pub id: NodeId,
    pub kind: RetimeNodeType,
    pub name: String,
    pub function: NodeFunction,
    pub fanins: Vec<EdgeId>,
    pub fanouts: Vec<EdgeId>,
}

impl RetimeNode {
    pub fn new(id: NodeId, kind: RetimeNodeType, name: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            name: name.into(),
            function: NodeFunction::Const0,
            fanins: Vec::new(),
            fanouts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetimeEdge {
    pub id: EdgeId,
    pub source: NodeId,
    pub sink: NodeId,
    pub sink_fanin_id: usize,
    pub weight: usize,
    pub latches: Vec<LatchId>,
    pub initial_values: Vec<i32>,
}

impl RetimeEdge {
    pub fn new(
        id: EdgeId,
        source: NodeId,
        sink: NodeId,
        sink_fanin_id: usize,
        weight: usize,
    ) -> Self {
        Self {
            id,
            source,
            sink,
            sink_fanin_id,
            weight,
            latches: Vec::new(),
            initial_values: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RetimeGraph {
    pub nodes: Vec<RetimeNode>,
    pub edges: Vec<RetimeEdge>,
    pub primary_inputs: Vec<NodeId>,
    pub primary_outputs: Vec<NodeId>,
}

impl RetimeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, kind: RetimeNodeType, name: impl Into<String>) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(RetimeNode::new(id, kind, name));
        match kind {
            RetimeNodeType::PrimaryInput => self.primary_inputs.push(id),
            RetimeNodeType::PrimaryOutput => self.primary_outputs.push(id),
            RetimeNodeType::Internal | RetimeNodeType::Ignore => {}
        }
        id
    }

    pub fn set_function(
        &mut self,
        node: NodeId,
        function: NodeFunction,
    ) -> Result<(), ReInitialError> {
        self.require_node(node)?;
        self.nodes[node.0].function = function;
        Ok(())
    }

    pub fn add_edge(
        &mut self,
        source: NodeId,
        sink: NodeId,
        sink_fanin_id: usize,
        weight: usize,
    ) -> Result<EdgeId, ReInitialError> {
        self.require_node(source)?;
        self.require_node(sink)?;

        let id = EdgeId(self.edges.len());
        self.edges
            .push(RetimeEdge::new(id, source, sink, sink_fanin_id, weight));
        self.nodes[source.0].fanouts.push(id);
        self.nodes[sink.0].fanins.push(id);
        Ok(id)
    }

    pub fn make_retiming_negative(&self, retiming: &mut [i32]) -> Result<usize, ReInitialError> {
        if retiming.len() != self.nodes.len() {
            return Err(ReInitialError::RetimingLengthMismatch {
                expected: self.nodes.len(),
                actual: retiming.len(),
            });
        }

        for node in self
            .primary_inputs
            .iter()
            .chain(self.primary_outputs.iter())
            .copied()
        {
            if retiming[node.0] != 0 {
                return Err(ReInitialError::HostVertexRetimed {
                    node,
                    value: retiming[node.0],
                });
            }
        }

        let shift = retiming.iter().copied().max().unwrap_or(0).max(0);
        for value in retiming {
            *value -= shift;
        }
        usize::try_from(shift).map_err(|_| ReInitialError::NegativeShift(shift))
    }

    pub fn set_initial_state(
        &mut self,
        latch_values: &HashMap<LatchId, i32>,
    ) -> Result<(), ReInitialError> {
        for edge_id in 0..self.edges.len() {
            if self.ignore_edge(EdgeId(edge_id))? || self.edges[edge_id].weight == 0 {
                continue;
            }

            if self.edges[edge_id].latches.len() < self.edges[edge_id].weight {
                return Err(ReInitialError::LatchCountMismatch {
                    edge: EdgeId(edge_id),
                    weight: self.edges[edge_id].weight,
                    latches: self.edges[edge_id].latches.len(),
                });
            }

            let values = self.edges[edge_id]
                .latches
                .iter()
                .take(self.edges[edge_id].weight)
                .map(|latch| {
                    latch_values
                        .get(latch)
                        .copied()
                        .ok_or(ReInitialError::MissingLatchInitialValue(*latch))
                })
                .collect::<Result<Vec<_>, _>>()?;

            self.edges[edge_id].initial_values = values;
            self.edges[edge_id].latches.clear();
        }
        Ok(())
    }

    pub fn simulate_forward(
        &mut self,
        retiming: &mut [i32],
        input_sequence: &[Vec<i32>],
    ) -> Result<(), ReInitialError> {
        if retiming.len() != self.nodes.len() {
            return Err(ReInitialError::RetimingLengthMismatch {
                expected: self.nodes.len(),
                actual: retiming.len(),
            });
        }

        let mut stack = RetimeStack::new(self.nodes.len());
        for (index, value) in retiming.iter().enumerate() {
            if *value < 0 {
                self.push_if_ready(&mut stack, NodeId(index))?;
            }
        }

        while let Some(node) = stack.pop() {
            let needed = usize::try_from(-retiming[node.0])
                .map_err(|_| ReInitialError::RetimingOverflow(retiming[node.0]))?;
            let local_shift = self.min_n_input_latch(node)?.min(needed);
            if local_shift == 0 {
                return Err(ReInitialError::NodeNotReady(node));
            }

            match self.nodes[node.0].kind {
                RetimeNodeType::PrimaryInput => {
                    let input_index = self
                        .primary_inputs
                        .iter()
                        .position(|candidate| *candidate == node)
                        .ok_or(ReInitialError::MissingPrimaryInputOrder(node))?;
                    if input_sequence.len() != local_shift {
                        return Err(ReInitialError::PrimaryInputSequenceLengthMismatch {
                            expected: local_shift,
                            actual: input_sequence.len(),
                        });
                    }
                    self.retime_primary_input_forward(node, input_index, input_sequence)?;
                }
                RetimeNodeType::PrimaryOutput => {
                    self.retime_primary_output_forward(node, local_shift)?;
                }
                RetimeNodeType::Internal => {
                    self.retime_node_forward(node, local_shift)?;
                }
                RetimeNodeType::Ignore => return Err(ReInitialError::IgnoredNodeRetimed(node)),
            }

            retiming[node.0] += i32::try_from(local_shift)
                .map_err(|_| ReInitialError::ShiftTooLarge(local_shift))?;

            let fanouts = self.nodes[node.0].fanouts.clone();
            for edge in fanouts {
                if self.ignore_edge(edge)? {
                    continue;
                }
                let output = self.edges[edge.0].sink;
                if retiming[output.0] < 0 {
                    self.push_if_ready(&mut stack, output)?;
                }
            }
        }

        if let Some((index, value)) = retiming
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| *value != 0)
        {
            return Err(ReInitialError::IncompleteRetiming {
                node: NodeId(index),
                remaining: value,
            });
        }

        Ok(())
    }

    fn retime_primary_input_forward(
        &mut self,
        node: NodeId,
        input_index: usize,
        input_sequence: &[Vec<i32>],
    ) -> Result<(), ReInitialError> {
        let n_shift = input_sequence.len();
        if n_shift == 0 {
            return Ok(());
        }

        let fanouts = self.nodes[node.0].fanouts.clone();
        for edge in &fanouts {
            if !self.ignore_edge(*edge)? {
                self.resize_initial_values(*edge, isize::try_from(n_shift).unwrap())?;
            }
        }

        for edge in fanouts {
            if self.ignore_edge(edge)? {
                continue;
            }
            for (index, input) in input_sequence.iter().enumerate() {
                let value = *input
                    .get(input_index)
                    .ok_or(ReInitialError::InputVectorTooShort {
                        vector: index,
                        expected_index: input_index,
                        actual_len: input.len(),
                    })?;
                BinaryValue::try_from_i32(value)?;
                self.edges[edge.0].initial_values[index] = value;
            }
        }

        Ok(())
    }

    fn retime_primary_output_forward(
        &mut self,
        node: NodeId,
        n_shift: usize,
    ) -> Result<(), ReInitialError> {
        let fanins = self.nodes[node.0].fanins.clone();
        for edge in fanins {
            if !self.ignore_edge(edge)? {
                self.resize_initial_values(edge, -isize::try_from(n_shift).unwrap())?;
            }
        }
        Ok(())
    }

    fn retime_node_forward(&mut self, node: NodeId, n_shift: usize) -> Result<(), ReInitialError> {
        let fanins = self.nodes[node.0].fanins.clone();
        let fanouts = self.nodes[node.0].fanouts.clone();
        let n_inputs = fanins
            .iter()
            .filter(|edge| !self.ignore_edge(**edge).unwrap_or(true))
            .count();
        let mut input_values = vec![vec![0; n_inputs]; n_shift];

        for i in 0..n_shift {
            for edge in &fanins {
                if self.ignore_edge(*edge)? {
                    continue;
                }
                let edge_ref = &self.edges[edge.0];
                let source_index = edge_ref.weight - n_shift + i;
                let target_index = edge_ref.sink_fanin_id;
                let input_row =
                    input_values
                        .get_mut(i)
                        .ok_or(ReInitialError::InputMatrixOutOfRange {
                            row: i,
                            column: target_index,
                        })?;
                let target = input_row.get_mut(target_index).ok_or(
                    ReInitialError::InputMatrixOutOfRange {
                        row: i,
                        column: target_index,
                    },
                )?;
                *target = *edge_ref.initial_values.get(source_index).ok_or(
                    ReInitialError::InitialValuesLengthMismatch {
                        edge: *edge,
                        weight: edge_ref.weight,
                        values: edge_ref.initial_values.len(),
                    },
                )?;
            }
        }

        for edge in &fanins {
            if !self.ignore_edge(*edge)? {
                self.resize_initial_values(*edge, -isize::try_from(n_shift).unwrap())?;
            }
        }
        for edge in &fanouts {
            if !self.ignore_edge(*edge)? {
                self.resize_initial_values(*edge, isize::try_from(n_shift).unwrap())?;
            }
        }

        let function = self.nodes[node.0].function.clone();
        let output_values = input_values
            .iter()
            .map(|inputs| function.evaluate(inputs))
            .collect::<Result<Vec<_>, _>>()?;

        for edge in fanouts {
            if self.ignore_edge(edge)? {
                continue;
            }
            for (index, value) in output_values.iter().copied().enumerate() {
                self.edges[edge.0].initial_values[index] = value;
            }
        }

        Ok(())
    }

    fn resize_initial_values(
        &mut self,
        edge: EdgeId,
        size_incr: isize,
    ) -> Result<(), ReInitialError> {
        self.require_edge(edge)?;
        if size_incr == 0 {
            return Ok(());
        }

        let old_weight = self.edges[edge.0].weight;
        if self.edges[edge.0].initial_values.len() != old_weight {
            return Err(ReInitialError::InitialValuesLengthMismatch {
                edge,
                weight: old_weight,
                values: self.edges[edge.0].initial_values.len(),
            });
        }

        let new_weight =
            old_weight
                .checked_add_signed(size_incr)
                .ok_or(ReInitialError::NegativeEdgeWeight {
                    edge,
                    weight: size_incr,
                })?;
        let mut new_values = vec![0; new_weight];
        if size_incr > 0 {
            let shift = usize::try_from(size_incr).unwrap();
            for (index, value) in self.edges[edge.0]
                .initial_values
                .iter()
                .copied()
                .enumerate()
            {
                new_values[index + shift] = value;
            }
        } else {
            new_values.copy_from_slice(&self.edges[edge.0].initial_values[..new_weight]);
        }

        self.edges[edge.0].initial_values = new_values;
        self.edges[edge.0].weight = new_weight;
        Ok(())
    }

    fn push_if_ready(&self, stack: &mut RetimeStack, node: NodeId) -> Result<(), ReInitialError> {
        if self.min_n_input_latch(node)? > 0 {
            stack.push(node)?;
        }
        Ok(())
    }

    fn min_n_input_latch(&self, node: NodeId) -> Result<usize, ReInitialError> {
        self.require_node(node)?;
        let mut min_weight = usize::MAX;
        for edge in &self.nodes[node.0].fanins {
            if self.ignore_edge(*edge)? {
                continue;
            }
            min_weight = min_weight.min(self.edges[edge.0].weight);
        }
        Ok(min_weight)
    }

    fn ignore_edge(&self, edge: EdgeId) -> Result<bool, ReInitialError> {
        self.require_edge(edge)?;
        Ok(
            self.nodes[self.edges[edge.0].source.0].kind == RetimeNodeType::Ignore
                || self.nodes[self.edges[edge.0].sink.0].kind == RetimeNodeType::Ignore,
        )
    }

    fn require_node(&self, node: NodeId) -> Result<(), ReInitialError> {
        match self.nodes.get(node.0) {
            Some(candidate) if candidate.id == node => Ok(()),
            _ => Err(ReInitialError::MissingNode(node)),
        }
    }

    fn require_edge(&self, edge: EdgeId) -> Result<(), ReInitialError> {
        match self.edges.get(edge.0) {
            Some(candidate) if candidate.id == edge => Ok(()),
            _ => Err(ReInitialError::MissingEdge(edge)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RetimeStack {
    nodes: Vec<NodeId>,
    in_stack: Vec<bool>,
}

impl RetimeStack {
    fn new(node_count: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(node_count),
            in_stack: vec![false; node_count],
        }
    }

    fn push(&mut self, node: NodeId) -> Result<(), ReInitialError> {
        let Some(in_stack) = self.in_stack.get_mut(node.0) else {
            return Err(ReInitialError::MissingNode(node));
        };
        if *in_stack {
            return Ok(());
        }
        self.nodes.push(node);
        *in_stack = true;
        Ok(())
    }

    fn pop(&mut self) -> Option<NodeId> {
        let node = self.nodes.pop()?;
        self.in_stack[node.0] = false;
        Some(node)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReInitialError {
    MissingSisDependencies {
        operation: &'static str,
    },
    MissingNode(NodeId),
    MissingEdge(EdgeId),
    MissingLatchInitialValue(LatchId),
    RetimingLengthMismatch {
        expected: usize,
        actual: usize,
    },
    HostVertexRetimed {
        node: NodeId,
        value: i32,
    },
    NegativeShift(i32),
    RetimingOverflow(i32),
    ShiftTooLarge(usize),
    NodeNotReady(NodeId),
    IgnoredNodeRetimed(NodeId),
    IncompleteRetiming {
        node: NodeId,
        remaining: i32,
    },
    MissingPrimaryInputOrder(NodeId),
    PrimaryInputSequenceLengthMismatch {
        expected: usize,
        actual: usize,
    },
    InputVectorTooShort {
        vector: usize,
        expected_index: usize,
        actual_len: usize,
    },
    InputMatrixOutOfRange {
        row: usize,
        column: usize,
    },
    LatchCountMismatch {
        edge: EdgeId,
        weight: usize,
        latches: usize,
    },
    InitialValuesLengthMismatch {
        edge: EdgeId,
        weight: usize,
        values: usize,
    },
    NegativeEdgeWeight {
        edge: EdgeId,
        weight: isize,
    },
    InvalidBinaryValue(i32),
    FunctionArityMismatch {
        expected: usize,
        actual: usize,
    },
    EmptyProductCube,
    EmptyCover,
}

impl fmt::Display for ReInitialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => {
                write!(f, "{operation} requires native prerequisite ports")
            }
            Self::MissingNode(node) => write!(f, "missing retime node {}", node.0),
            Self::MissingEdge(edge) => write!(f, "missing retime edge {}", edge.0),
            Self::MissingLatchInitialValue(latch) => {
                write!(f, "missing initial value for latch {}", latch.0)
            }
            Self::RetimingLengthMismatch { expected, actual } => write!(
                f,
                "retiming vector length mismatch: expected {expected}, got {actual}"
            ),
            Self::HostVertexRetimed { node, value } => {
                write!(f, "host vertex {} has non-zero retiming {value}", node.0)
            }
            Self::NegativeShift(shift) => write!(f, "negative retiming shift {shift}"),
            Self::RetimingOverflow(value) => write!(f, "retiming value {value} overflows"),
            Self::ShiftTooLarge(shift) => write!(f, "retiming shift {shift} is too large"),
            Self::NodeNotReady(node) => write!(f, "retime node {} is not ready", node.0),
            Self::IgnoredNodeRetimed(node) => {
                write!(f, "ignored retime node {} cannot be retimed", node.0)
            }
            Self::IncompleteRetiming { node, remaining } => {
                write!(f, "node {} still has retiming {remaining}", node.0)
            }
            Self::MissingPrimaryInputOrder(node) => {
                write!(
                    f,
                    "primary input node {} is missing from input order",
                    node.0
                )
            }
            Self::PrimaryInputSequenceLengthMismatch { expected, actual } => write!(
                f,
                "primary input sequence length mismatch: expected {expected}, got {actual}"
            ),
            Self::InputVectorTooShort {
                vector,
                expected_index,
                actual_len,
            } => write!(
                f,
                "input vector {vector} has length {actual_len}, missing index {expected_index}"
            ),
            Self::InputMatrixOutOfRange { row, column } => {
                write!(f, "input matrix position ({row}, {column}) is out of range")
            }
            Self::LatchCountMismatch {
                edge,
                weight,
                latches,
            } => write!(
                f,
                "edge {} has weight {weight} but only {latches} latch IDs",
                edge.0
            ),
            Self::InitialValuesLengthMismatch {
                edge,
                weight,
                values,
            } => write!(
                f,
                "edge {} has weight {weight} but {values} initial values",
                edge.0
            ),
            Self::NegativeEdgeWeight { edge, weight } => {
                write!(
                    f,
                    "edge {} would receive negative weight delta {weight}",
                    edge.0
                )
            }
            Self::InvalidBinaryValue(value) => {
                write!(f, "expected binary value 0 or 1, got {value}")
            }
            Self::FunctionArityMismatch { expected, actual } => write!(
                f,
                "node function arity mismatch: expected {expected}, got {actual}"
            ),
            Self::EmptyProductCube => write!(f, "sum-of-products cube has no asserted literal"),
            Self::EmptyCover => write!(f, "sum-of-products cover did not determine an output"),
        }
    }
}

impl Error for ReInitialError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> RetimeGraph {
        let mut graph = RetimeGraph::new();
        let a = graph.add_node(RetimeNodeType::PrimaryInput, "a");
        let b = graph.add_node(RetimeNodeType::PrimaryInput, "b");
        let n = graph.add_node(RetimeNodeType::Internal, "n");
        let z = graph.add_node(RetimeNodeType::PrimaryOutput, "z");
        graph
            .set_function(
                n,
                NodeFunction::SumOfProducts(vec![vec![CubeLiteral::One, CubeLiteral::Zero]]),
            )
            .unwrap();

        let an = graph.add_edge(a, n, 0, 1).unwrap();
        let bn = graph.add_edge(b, n, 1, 1).unwrap();
        let nz = graph.add_edge(n, z, 0, 1).unwrap();

        graph.edges[an.0].latches = vec![LatchId(10)];
        graph.edges[bn.0].latches = vec![LatchId(11)];
        graph.edges[nz.0].latches = vec![LatchId(12)];
        graph
    }

    #[test]
    fn make_retiming_negative_checks_host_vertices_and_shifts_by_max() {
        let graph = sample_graph();
        let mut retiming = vec![0, 0, 3, 0];

        assert_eq!(graph.make_retiming_negative(&mut retiming), Ok(3));
        assert_eq!(retiming, vec![-3, -3, 0, -3]);

        let mut bad = vec![1, 0, 3, 0];
        assert_eq!(
            graph.make_retiming_negative(&mut bad),
            Err(ReInitialError::HostVertexRetimed {
                node: NodeId(0),
                value: 1
            })
        );
    }

    #[test]
    fn set_initial_state_copies_latch_values_and_clears_latch_identity() {
        let mut graph = sample_graph();
        let latch_values = HashMap::from([(LatchId(10), 1), (LatchId(11), 0), (LatchId(12), 1)]);

        graph.set_initial_state(&latch_values).unwrap();

        assert_eq!(graph.edges[0].initial_values, vec![1]);
        assert_eq!(graph.edges[1].initial_values, vec![0]);
        assert_eq!(graph.edges[2].initial_values, vec![1]);
        assert!(graph.edges.iter().all(|edge| edge.latches.is_empty()));
    }

    #[test]
    fn node_function_matches_c_ternary_and_or_tables() {
        let and_not = NodeFunction::SumOfProducts(vec![vec![
            CubeLiteral::One,
            CubeLiteral::Zero,
            CubeLiteral::DontCare,
        ]]);
        assert_eq!(and_not.evaluate(&[1, 0, 1]), Ok(1));
        assert_eq!(and_not.evaluate(&[1, 1, 1]), Ok(0));

        let or_cover = NodeFunction::SumOfProducts(vec![
            vec![CubeLiteral::One, CubeLiteral::DontCare],
            vec![CubeLiteral::DontCare, CubeLiteral::One],
        ]);
        assert_eq!(or_cover.evaluate(&[0, 0]), Ok(0));
        assert_eq!(or_cover.evaluate(&[0, 1]), Ok(1));
    }

    #[test]
    fn simulate_forward_moves_pi_values_through_internal_node_to_po() {
        let mut graph = sample_graph();
        let latch_values = HashMap::from([(LatchId(10), 1), (LatchId(11), 0), (LatchId(12), 0)]);
        graph.set_initial_state(&latch_values).unwrap();
        let mut retiming = vec![-1, -1, -1, -1];
        let input_sequence = vec![vec![1, 0]];

        graph
            .simulate_forward(&mut retiming, &input_sequence)
            .unwrap();

        assert_eq!(retiming, vec![0, 0, 0, 0]);
        assert_eq!(graph.edges[0].weight, 1);
        assert_eq!(graph.edges[0].initial_values, vec![1]);
        assert_eq!(graph.edges[1].weight, 1);
        assert_eq!(graph.edges[1].initial_values, vec![0]);
        assert_eq!(graph.edges[2].weight, 1);
        assert_eq!(graph.edges[2].initial_values, vec![1]);
    }

    #[test]
    fn simulate_forward_leaves_incomplete_retiming_when_no_input_latches_exist() {
        let mut graph = RetimeGraph::new();
        let a = graph.add_node(RetimeNodeType::PrimaryInput, "a");
        let n = graph.add_node(RetimeNodeType::Internal, "n");
        graph.add_edge(a, n, 0, 0).unwrap();
        graph.set_function(n, NodeFunction::Const1).unwrap();
        let mut retiming = vec![0, -1];

        assert_eq!(
            graph.simulate_forward(&mut retiming, &[]),
            Err(ReInitialError::IncompleteRetiming {
                node: n,
                remaining: -1
            })
        );
    }
}
