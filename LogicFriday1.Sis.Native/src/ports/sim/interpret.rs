//! Native Rust model for feasible behavior in `sis/sim/interpret.c`.
//!
//! The C source combines direct `node_t`/`network_t` mutation, STG graph
//! traversal, and a 32-bit packed random verifier. This module ports the
//! observable simulation rules into owned Rust data structures. Direct SIS
//! object integration is left as explicit dependency errors until those source
//! files have native Rust ports.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const ZERO: usize = 1;
pub const ONE: usize = 2;
pub const TWO: usize = 3;
pub const PACKED_PATTERN_BITS: usize = 32;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicValue {
    Zero,
    One,
}

impl LogicValue {
    pub fn bit(self) -> bool {
        self == Self::One
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
        }
    }

    pub fn packed(self) -> u32 {
        if self.bit() { u32::MAX } else { 0 }
    }
}

impl From<bool> for LogicValue {
    fn from(value: bool) -> Self {
        if value { Self::One } else { Self::Zero }
    }
}

impl TryFrom<i32> for LogicValue {
    type Error = InterpretError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            _ => Err(InterpretError::InvalidInputValue(value)),
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
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    pub fn from_c_value(value: usize) -> Result<Self, InterpretError> {
        match value {
            ZERO => Ok(Self::Zero),
            ONE => Ok(Self::One),
            TWO => Ok(Self::DontCare),
            _ => Err(InterpretError::InvalidLiteral(value)),
        }
    }

    pub fn c_value(self) -> usize {
        match self {
            Self::Zero => ZERO,
            Self::One => ONE,
            Self::DontCare => TWO,
        }
    }

    fn accepts(self, value: LogicValue) -> bool {
        match self {
            Self::Zero => value == LogicValue::Zero,
            Self::One => value == LogicValue::One,
            Self::DontCare => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }

    pub fn from_c_literals(
        literals: impl IntoIterator<Item = usize>,
    ) -> Result<Self, InterpretError> {
        literals
            .into_iter()
            .map(Literal::from_c_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Self::new)
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    fn matches_values(&self, values: &[LogicValue]) -> bool {
        self.literals
            .iter()
            .zip(values.iter())
            .all(|(literal, value)| literal.accepts(*value))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub cubes: Vec<Cube>,
    pub value: Option<LogicValue>,
}

impl SimNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            cubes: Vec::new(),
            value: None,
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }

    pub fn with_cubes(mut self, cubes: Vec<Cube>) -> Self {
        self.cubes = cubes;
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimNetwork {
    nodes: Vec<SimNode>,
    primary_inputs: Vec<NodeId>,
    primary_outputs: Vec<NodeId>,
}

impl SimNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: SimNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        match node.kind {
            NodeKind::PrimaryInput => self.primary_inputs.push(id),
            NodeKind::PrimaryOutput => self.primary_outputs.push(id),
            NodeKind::Internal => {}
        }
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&SimNode, InterpretError> {
        self.nodes.get(id.0).ok_or(InterpretError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut SimNode, InterpretError> {
        self.nodes
            .get_mut(id.0)
            .ok_or(InterpretError::UnknownNode(id))
    }

    pub fn primary_inputs(&self) -> &[NodeId] {
        &self.primary_inputs
    }

    pub fn primary_outputs(&self) -> &[NodeId] {
        &self.primary_outputs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StgEdge {
    pub from: String,
    pub to: String,
    pub input: String,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StgModel {
    current: String,
    output_count: usize,
    edges: Vec<StgEdge>,
}

impl StgModel {
    pub fn new(current: impl Into<String>, output_count: usize) -> Self {
        Self {
            current: current.into(),
            output_count,
            edges: Vec::new(),
        }
    }

    pub fn current_state(&self) -> &str {
        &self.current
    }

    pub fn add_edge(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        input: impl Into<String>,
        output: impl Into<String>,
    ) {
        self.edges.push(StgEdge {
            from: from.into(),
            to: to.into(),
            input: input.into(),
            output: output.into(),
        });
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StgStep {
    pub output: Vec<char>,
    pub next_state: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationFailure {
    pub input_pattern: Vec<LogicValue>,
    pub internal_outputs: Vec<LogicValue>,
    pub read_in_outputs: Vec<LogicValue>,
    pub faulty_output: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationResult {
    Passed { random_vectors: usize },
    Failed(VerificationFailure),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InterpretError {
    UnknownNode(NodeId),
    InvalidInputValue(i32),
    InvalidLiteral(usize),
    WrongInputCount {
        expected: usize,
        actual: usize,
    },
    MissingNodeValue(NodeId),
    MissingOutputDriver(NodeId),
    CubeArityMismatch {
        node: NodeId,
        cube_literals: usize,
        fanins: usize,
    },
    InvalidStgValue(LogicValue),
    InvalidStgOutputWidth {
        state: String,
        output: String,
        expected: usize,
    },
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for InterpretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown simulation node {:?}", node),
            Self::InvalidInputValue(value) => write!(f, "invalid binary simulation value {value}"),
            Self::InvalidLiteral(value) => write!(f, "invalid SIS cube literal value {value}"),
            Self::WrongInputCount { expected, actual } => {
                write!(f, "expected {expected} primary-input values, got {actual}")
            }
            Self::MissingNodeValue(node) => {
                write!(f, "simulation value for node {:?} is not set", node)
            }
            Self::MissingOutputDriver(node) => {
                write!(f, "primary output {:?} has no driving fanin", node)
            }
            Self::CubeArityMismatch {
                node,
                cube_literals,
                fanins,
            } => write!(
                f,
                "node {:?} cube has {cube_literals} literals but node has {fanins} fanins",
                node
            ),
            Self::InvalidStgValue(value) => {
                write!(f, "STG simulation accepts only 0/1 values, got {:?}", value)
            }
            Self::InvalidStgOutputWidth {
                state,
                output,
                expected,
            } => write!(
                f,
                "STG edge from {state} produced output {output:?} with width {}, expected {expected}",
                output.chars().count()
            ),
            Self::MissingSisPorts { operation } => write!(
                f,
                "{operation} is blocked by unported native SIS dependencies"
            ),
        }
    }
}

impl Error for InterpretError {}

pub fn simulate_node(network: &mut SimNetwork, node: NodeId) -> Result<LogicValue, InterpretError> {
    let node_data = network.node(node)?.clone();
    let fanin_values = node_data
        .fanins
        .iter()
        .map(|fanin| {
            network
                .node(*fanin)?
                .value
                .ok_or(InterpretError::MissingNodeValue(*fanin))
        })
        .collect::<Result<Vec<_>, _>>()?;

    for cube in &node_data.cubes {
        if cube.literals.len() != fanin_values.len() {
            return Err(InterpretError::CubeArityMismatch {
                node,
                cube_literals: cube.literals.len(),
                fanins: fanin_values.len(),
            });
        }

        if cube.matches_values(&fanin_values) {
            network.node_mut(node)?.value = Some(LogicValue::One);
            return Ok(LogicValue::One);
        }
    }

    network.node_mut(node)?.value = Some(LogicValue::Zero);
    Ok(LogicValue::Zero)
}

pub fn simulate_network(
    network: &mut SimNetwork,
    node_order: &[NodeId],
    input_values: &[LogicValue],
) -> Result<Vec<LogicValue>, InterpretError> {
    if input_values.len() != network.primary_inputs.len() {
        return Err(InterpretError::WrongInputCount {
            expected: network.primary_inputs.len(),
            actual: input_values.len(),
        });
    }

    let primary_inputs = network.primary_inputs.clone();
    for (input, value) in primary_inputs.into_iter().zip(input_values.iter().copied()) {
        network.node_mut(input)?.value = Some(value);
    }

    for node in node_order {
        if network.node(*node)?.kind == NodeKind::Internal {
            simulate_node(network, *node)?;
        }
    }

    network
        .primary_outputs
        .iter()
        .map(|po| {
            let driver = *network
                .node(*po)?
                .fanins
                .first()
                .ok_or(InterpretError::MissingOutputDriver(*po))?;
            network
                .node(driver)?
                .value
                .ok_or(InterpretError::MissingNodeValue(driver))
        })
        .collect()
}

pub fn equivtrans(pattern: &str, value: &str) -> bool {
    pattern
        .chars()
        .zip(value.chars())
        .all(|(left, right)| left == '-' || right == '-' || left == right)
        && pattern.chars().count() == value.chars().count()
}

pub fn simulate_stg(
    stg: &mut StgModel,
    input_values: &[LogicValue],
) -> Result<Option<StgStep>, InterpretError> {
    let input = input_values
        .iter()
        .map(|value| match value {
            LogicValue::Zero => Ok('0'),
            LogicValue::One => Ok('1'),
        })
        .collect::<Result<String, _>>()?;

    let current = stg.current.clone();
    let candidates = [&current[..], "ANY", "*"];
    for state in candidates {
        if let Some(edge) = stg
            .edges
            .iter()
            .find(|edge| edge.from == state && equivtrans(&edge.input, &input))
            .cloned()
        {
            let output_width = edge.output.chars().count();
            if output_width != stg.output_count {
                return Err(InterpretError::InvalidStgOutputWidth {
                    state: edge.from,
                    output: edge.output,
                    expected: stg.output_count,
                });
            }

            stg.current = edge.to.clone();
            return Ok(Some(StgStep {
                output: edge.output.chars().collect(),
                next_state: edge.to,
            }));
        }
    }

    Ok(None)
}

pub fn simulate_node_32(node: &SimNode, packed_fanins: &[u32]) -> Result<u32, InterpretError> {
    let mut result = 0_u32;

    for cube in &node.cubes {
        if cube.literals.len() != packed_fanins.len() {
            return Err(InterpretError::CubeArityMismatch {
                node: NodeId(usize::MAX),
                cube_literals: cube.literals.len(),
                fanins: packed_fanins.len(),
            });
        }

        let mut and_result = u32::MAX;
        for (literal, input) in cube.literals.iter().zip(packed_fanins.iter().copied()) {
            and_result &= match literal {
                Literal::Zero => !input,
                Literal::One => input,
                Literal::DontCare => u32::MAX,
            };
        }
        result |= and_result;
    }

    Ok(result)
}

pub fn simulate_network_32(
    network: &SimNetwork,
    input_values: &[u32],
) -> Result<Vec<u32>, InterpretError> {
    if input_values.len() != network.primary_inputs.len() {
        return Err(InterpretError::WrongInputCount {
            expected: network.primary_inputs.len(),
            actual: input_values.len(),
        });
    }

    let mut values = HashMap::new();
    for (input, value) in network
        .primary_inputs
        .iter()
        .copied()
        .zip(input_values.iter().copied())
    {
        values.insert(input, value);
    }

    network
        .primary_outputs
        .iter()
        .map(|output| {
            let driver = *network
                .node(*output)?
                .fanins
                .first()
                .ok_or(InterpretError::MissingOutputDriver(*output))?;
            simulate_network_32_rec(network, &mut values, driver)
        })
        .collect()
}

pub fn sim_verify(
    network0: &SimNetwork,
    network1: &SimNetwork,
    n_patterns: usize,
    seed: u32,
) -> Result<VerificationResult, InterpretError> {
    let n_batches = n_patterns >> 5;
    let n_pi = network0.primary_inputs.len();
    let n_po = network0.primary_outputs.len();

    if network1.primary_inputs.len() != n_pi {
        return Err(InterpretError::WrongInputCount {
            expected: n_pi,
            actual: network1.primary_inputs.len(),
        });
    }
    if network1.primary_outputs.len() != n_po {
        return Err(InterpretError::WrongInputCount {
            expected: n_po,
            actual: network1.primary_outputs.len(),
        });
    }

    let mut rng = SisRandom::new(seed);
    for _ in 0..n_batches {
        let pi_values = gen_random_pattern(n_pi, &mut rng);
        let po_values0 = simulate_network_32(network0, &pi_values)?;
        let po_values1 = simulate_network_32(network1, &pi_values)?;

        for output_index in 0..n_po {
            let diff = po_values0[output_index] ^ po_values1[output_index];
            if diff != 0 {
                let mask = diff & diff.wrapping_neg();
                return Ok(VerificationResult::Failed(VerificationFailure {
                    input_pattern: print_pattern(&pi_values, mask),
                    internal_outputs: print_pattern(&po_values0, mask),
                    read_in_outputs: print_pattern(&po_values1, mask),
                    faulty_output: output_index,
                }));
            }
        }
    }

    Ok(VerificationResult::Passed {
        random_vectors: n_batches << 5,
    })
}

pub fn gen_random_pattern(n: usize, rng: &mut SisRandom) -> Vec<u32> {
    (0..n)
        .map(|_| (rng.random31() << 16) | (rng.random31() & 0xffff))
        .collect()
}

pub fn print_pattern(pattern: &[u32], mask: u32) -> Vec<LogicValue> {
    pattern
        .iter()
        .map(|value| LogicValue::from((value & mask) != 0))
        .collect()
}

pub fn simulate_node_from_sis() -> Result<LogicValue, InterpretError> {
    Err(InterpretError::MissingSisPorts {
        operation: "simulate_node(node_t *)",
    })
}

pub fn simulate_network_from_sis() -> Result<Vec<LogicValue>, InterpretError> {
    Err(InterpretError::MissingSisPorts {
        operation: "simulate_network(network_t *, array_t *, array_t *)",
    })
}

pub fn simulate_stg_from_sis() -> Result<Option<StgStep>, InterpretError> {
    Err(InterpretError::MissingSisPorts {
        operation: "simulate_stg(graph_t *, array_t *, vertex_t **)",
    })
}

fn simulate_network_32_rec(
    network: &SimNetwork,
    values: &mut HashMap<NodeId, u32>,
    node: NodeId,
) -> Result<u32, InterpretError> {
    if let Some(value) = values.get(&node) {
        return Ok(*value);
    }

    let node_data = network.node(node)?;
    let mut inputs = Vec::with_capacity(node_data.fanins.len());
    for fanin in &node_data.fanins {
        inputs.push(simulate_network_32_rec(network, values, *fanin)?);
    }

    let result = simulate_node_32(node_data, &inputs)?;
    values.insert(node, result);
    Ok(result)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisRandom {
    state: u32,
}

impl SisRandom {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    pub fn random31(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.state >> 1) & 0x7fff_ffff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inv_network() -> SimNetwork {
        let mut network = SimNetwork::new();
        let a = network.add_node(SimNode::new("a", NodeKind::PrimaryInput));
        let n = network.add_node(
            SimNode::new("n", NodeKind::Internal)
                .with_fanins(vec![a])
                .with_cubes(vec![Cube::new(vec![Literal::Zero])]),
        );
        network.add_node(SimNode::new("out", NodeKind::PrimaryOutput).with_fanins(vec![n]));
        network
    }

    #[test]
    fn literal_c_values_match_sis_set_encoding() {
        assert_eq!(Literal::from_c_value(ZERO), Ok(Literal::Zero));
        assert_eq!(Literal::from_c_value(ONE), Ok(Literal::One));
        assert_eq!(Literal::from_c_value(TWO), Ok(Literal::DontCare));
        assert_eq!(Literal::DontCare.c_value(), TWO);
        assert_eq!(
            Cube::from_c_literals([ONE, ZERO, TWO]).unwrap().literals(),
            &[Literal::One, Literal::Zero, Literal::DontCare]
        );
    }

    #[test]
    fn simulate_node_sets_one_when_any_cube_matches() {
        let mut network = SimNetwork::new();
        let a = network.add_node(SimNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(SimNode::new("b", NodeKind::PrimaryInput));
        let node = network.add_node(
            SimNode::new("and", NodeKind::Internal)
                .with_fanins(vec![a, b])
                .with_cubes(vec![Cube::new(vec![Literal::One, Literal::One])]),
        );

        network.node_mut(a).unwrap().value = Some(LogicValue::One);
        network.node_mut(b).unwrap().value = Some(LogicValue::One);
        assert_eq!(simulate_node(&mut network, node), Ok(LogicValue::One));

        network.node_mut(b).unwrap().value = Some(LogicValue::Zero);
        assert_eq!(simulate_node(&mut network, node), Ok(LogicValue::Zero));
    }

    #[test]
    fn simulate_network_sets_inputs_runs_internal_order_and_reads_output_drivers() {
        let mut network = inv_network();
        assert_eq!(
            simulate_network(&mut network, &[NodeId(1)], &[LogicValue::Zero]),
            Ok(vec![LogicValue::One])
        );
        assert_eq!(
            simulate_network(&mut network, &[NodeId(1)], &[LogicValue::One]),
            Ok(vec![LogicValue::Zero])
        );
        assert_eq!(
            simulate_network(&mut network, &[NodeId(1)], &[]),
            Err(InterpretError::WrongInputCount {
                expected: 1,
                actual: 0,
            })
        );
    }

    #[test]
    fn equivtrans_treats_dash_as_dont_care_on_either_side() {
        assert!(equivtrans("1-0", "110"));
        assert!(equivtrans("100", "1-0"));
        assert!(!equivtrans("100", "110"));
        assert!(!equivtrans("10", "100"));
    }

    #[test]
    fn simulate_stg_uses_current_state_then_any_and_star_fallbacks() {
        let mut stg = StgModel::new("S0", 2);
        stg.add_edge("S0", "S1", "00", "10");
        stg.add_edge("ANY", "S2", "1-", "01");
        stg.add_edge("*", "S3", "01", "11");

        let step = simulate_stg(&mut stg, &[LogicValue::Zero, LogicValue::Zero])
            .unwrap()
            .unwrap();
        assert_eq!(step.output, vec!['1', '0']);
        assert_eq!(stg.current_state(), "S1");

        let step = simulate_stg(&mut stg, &[LogicValue::One, LogicValue::Zero])
            .unwrap()
            .unwrap();
        assert_eq!(step.next_state, "S2");
        assert_eq!(step.output, vec!['0', '1']);
    }

    #[test]
    fn simulate_node_32_matches_sop_packed_bit_behavior() {
        let node = SimNode::new("muxish", NodeKind::Internal)
            .with_fanins(vec![NodeId(0), NodeId(1)])
            .with_cubes(vec![
                Cube::new(vec![Literal::One, Literal::Zero]),
                Cube::new(vec![Literal::Zero, Literal::One]),
            ]);

        let result = simulate_node_32(&node, &[0b1010, 0b1100]).unwrap();
        assert_eq!(result & 0b1111, 0b0110);
    }

    #[test]
    fn simulate_network_32_recurses_from_primary_outputs() {
        let network = inv_network();
        let outputs = simulate_network_32(&network, &[0x0f0f_0f0f]).unwrap();
        assert_eq!(outputs, vec![!0x0f0f_0f0f]);
    }

    #[test]
    fn sim_verify_reports_first_mismatching_packed_bit() {
        let good = inv_network();
        let mut bad = SimNetwork::new();
        let a = bad.add_node(SimNode::new("a", NodeKind::PrimaryInput));
        let n = bad.add_node(
            SimNode::new("buf", NodeKind::Internal)
                .with_fanins(vec![a])
                .with_cubes(vec![Cube::new(vec![Literal::One])]),
        );
        bad.add_node(SimNode::new("out", NodeKind::PrimaryOutput).with_fanins(vec![n]));

        let result = sim_verify(&good, &bad, PACKED_PATTERN_BITS, 7).unwrap();
        let VerificationResult::Failed(failure) = result else {
            panic!("expected verification failure");
        };
        assert_eq!(failure.faulty_output, 0);
        assert_eq!(failure.internal_outputs.len(), 1);
        assert_eq!(failure.read_in_outputs.len(), 1);
    }

    #[test]
    fn sim_verify_reports_passed_vector_count_in_multiples_of_32() {
        let left = inv_network();
        let right = inv_network();

        assert_eq!(
            sim_verify(&left, &right, 65, 123),
            Ok(VerificationResult::Passed { random_vectors: 64 })
        );
    }

    #[test]
    fn sis_bound_entry_points_report_missing_native_prerequisites() {
        assert_eq!(
            simulate_node_from_sis(),
            Err(InterpretError::MissingSisPorts {
                operation: "simulate_node(node_t *)",
            })
        );

        assert!(format!("{}", simulate_network_from_sis().unwrap_err()).contains("blocked"));
    }
}
