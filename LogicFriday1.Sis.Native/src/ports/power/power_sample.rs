//! Native Rust model for `LogicSynthesis/sis/power/power_sample.c`.
//!
//! The C file estimates power by generating packed random PI words, simulating
//! a symbolic network one machine word at a time, counting PO one bits, and
//! summing `switching_prob * cap_factor * CAPACITANCE * 250.0`. This module
//! ports that sampling core to owned Rust data structures. Direct binding to
//! legacy SIS `network_t`, delay tracing, and symbolic-network construction is
//! reported as an explicit missing native integration.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::hash::Hash;

pub const RAND_RANGE: u32 = 0x7fff_ffff;
pub const CAPACITANCE: f64 = 0.01;
pub const DEFAULT_PS_MAX_ALLOWED_ERROR: f64 = 0.01;
pub const CAP_IN_LATCH: f64 = 4.0;
pub const CAP_OUT_LATCH: f64 = 4.0;
pub const WORD_BITS: usize = usize::BITS as usize;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerCircuitType {
    Combinational,
    Sequential,
    Pipeline,
    Dynamic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelaySelection {
    Zero,
    Unit,
    Mapped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentStateProbability {
    Approximation,
    Exact,
    StateLine,
    Uniform,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerSampleDefaults {
    pub set_size: usize,
    pub delta: f64,
    pub verbose: bool,
    pub cap_in_latch: f64,
    pub cap_out_latch: f64,
}

impl Default for PowerSampleDefaults {
    fn default() -> Self {
        Self {
            set_size: 1,
            delta: DEFAULT_PS_MAX_ALLOWED_ERROR,
            verbose: false,
            cap_in_latch: CAP_IN_LATCH,
            cap_out_latch: CAP_OUT_LATCH,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerSimulationOptions {
    pub circuit_type: PowerCircuitType,
    pub delay: DelaySelection,
    pub ps_probability: PresentStateProbability,
    pub num_samples: usize,
    pub sample_gap: usize,
    pub defaults: PowerSampleDefaults,
}

impl PowerSimulationOptions {
    pub fn simple(
        circuit_type: PowerCircuitType,
        delay: DelaySelection,
        num_samples: usize,
    ) -> Self {
        Self {
            circuit_type,
            delay,
            ps_probability: PresentStateProbability::Approximation,
            num_samples,
            sample_gap: num_samples + 1,
            defaults: PowerSampleDefaults::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerSampleOperation {
    LegacySampleEstimate,
    LegacySimulationEstimate,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PowerSampleError {
    MissingSisDependencies { operation: PowerSampleOperation },
    MissingNode,
    MissingFanin,
    MissingOutputDriver,
    DuplicateNode,
    CubeArityMismatch { expected: usize, actual: usize },
    ProbabilityOutOfRange(f64),
    OutputOneCountMismatch { outputs: usize, counts: usize },
    EmptySampleCount,
}

impl fmt::Display for PowerSampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
            Self::MissingNode => write!(f, "sample network references a missing node"),
            Self::MissingFanin => write!(f, "sample network node references a missing fanin"),
            Self::MissingOutputDriver => write!(f, "sample network output has no driver"),
            Self::DuplicateNode => write!(f, "sample network contains a duplicate node id"),
            Self::CubeArityMismatch { expected, actual } => {
                write!(f, "cube has {actual} literals, expected {expected}")
            }
            Self::ProbabilityOutOfRange(probability) => {
                write!(f, "probability {probability} is outside 0.0..=1.0")
            }
            Self::OutputOneCountMismatch { outputs, counts } => {
                write!(f, "got {counts} output counts for {outputs} outputs")
            }
            Self::EmptySampleCount => write!(f, "power sampling requires at least one word sample"),
        }
    }
}

impl Error for PowerSampleError {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SampleNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    fn apply(self, value: usize) -> usize {
        match self {
            Self::Zero => !value,
            Self::One => value,
            Self::DontCare => usize::MAX,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SampleCube {
    pub literals: Vec<Literal>,
}

impl SampleCube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SampleNodeKind<N> {
    PrimaryInput {
        name: N,
        probability_one: f64,
    },
    Internal {
        fanins: Vec<SampleNodeId>,
        cubes: Vec<SampleCube>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SampleNode<N> {
    pub id: SampleNodeId,
    pub cap_factor: f64,
    pub kind: SampleNodeKind<N>,
}

impl<N> SampleNode<N> {
    pub fn primary_input(id: SampleNodeId, name: N, probability_one: f64) -> Self {
        Self {
            id,
            cap_factor: 0.0,
            kind: SampleNodeKind::PrimaryInput {
                name,
                probability_one,
            },
        }
    }

    pub fn internal(
        id: SampleNodeId,
        fanins: Vec<SampleNodeId>,
        cubes: Vec<SampleCube>,
        cap_factor: f64,
    ) -> Self {
        Self {
            id,
            cap_factor,
            kind: SampleNodeKind::Internal { fanins, cubes },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SampleOutput<N> {
    pub name: N,
    pub driver: SampleNodeId,
    pub cap_factor: f64,
    pub switching_prob: f64,
}

impl<N> SampleOutput<N> {
    pub fn new(name: N, driver: SampleNodeId, cap_factor: f64) -> Self {
        Self {
            name,
            driver,
            cap_factor,
            switching_prob: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerSampleNetwork<N> {
    pub nodes: Vec<SampleNode<N>>,
    pub primary_inputs: Vec<SampleNodeId>,
    pub outputs: Vec<SampleOutput<N>>,
}

impl<N> PowerSampleNetwork<N> {
    pub fn new(
        nodes: Vec<SampleNode<N>>,
        primary_inputs: Vec<SampleNodeId>,
        outputs: Vec<SampleOutput<N>>,
    ) -> Self {
        Self {
            nodes,
            primary_inputs,
            outputs,
        }
    }
}

impl<N> PowerSampleNetwork<N>
where
    N: Eq + Hash,
{
    pub fn primary_input_probabilities(&self) -> Result<Vec<f64>, PowerSampleError> {
        let by_id = self.node_index()?;
        let mut probabilities = Vec::with_capacity(self.primary_inputs.len());

        for input in &self.primary_inputs {
            let node = by_id.get(input).ok_or(PowerSampleError::MissingNode)?;
            let SampleNodeKind::PrimaryInput {
                probability_one, ..
            } = node.kind
            else {
                return Err(PowerSampleError::MissingNode);
            };
            validate_probability(probability_one)?;
            probabilities.push(probability_one);
        }

        Ok(probabilities)
    }

    pub fn simulate_word(&self, pi_values: &[usize]) -> Result<Vec<usize>, PowerSampleError> {
        let by_id = self.node_index()?;
        let mut values = HashMap::with_capacity(self.nodes.len());

        if pi_values.len() != self.primary_inputs.len() {
            return Err(PowerSampleError::MissingFanin);
        }

        for (node, value) in self.primary_inputs.iter().zip(pi_values.iter()) {
            values.insert(*node, *value);
        }

        let mut output_values = Vec::with_capacity(self.outputs.len());
        for output in &self.outputs {
            if !by_id.contains_key(&output.driver) {
                return Err(PowerSampleError::MissingOutputDriver);
            }
            output_values.push(self.simulate_word_rec(output.driver, &by_id, &mut values)?);
        }

        Ok(output_values)
    }

    pub fn calculate_power(
        &mut self,
        output_one_counts: &[usize],
        word_samples: usize,
    ) -> Result<f64, PowerSampleError> {
        if word_samples == 0 {
            return Err(PowerSampleError::EmptySampleCount);
        }
        if output_one_counts.len() != self.outputs.len() {
            return Err(PowerSampleError::OutputOneCountMismatch {
                outputs: self.outputs.len(),
                counts: output_one_counts.len(),
            });
        }

        let bit_samples = (word_samples * WORD_BITS) as f64;
        let mut power = 0.0;
        for (output, one_count) in self.outputs.iter_mut().zip(output_one_counts.iter()) {
            let switching_prob = *one_count as f64 / bit_samples;
            output.switching_prob = switching_prob;
            power += switching_prob * output.cap_factor;
        }

        Ok(power * CAPACITANCE * 250.0)
    }

    pub fn sample_estimate(
        &mut self,
        num_samples: usize,
        sample_gap: usize,
        seed: u32,
    ) -> Result<PowerSampleEstimate, PowerSampleError> {
        let probabilities = self.primary_input_probabilities()?;
        power_sample_do_estimate(self, num_samples, sample_gap, seed, &probabilities)
    }

    fn simulate_word_rec(
        &self,
        node_id: SampleNodeId,
        by_id: &HashMap<SampleNodeId, &SampleNode<N>>,
        values: &mut HashMap<SampleNodeId, usize>,
    ) -> Result<usize, PowerSampleError> {
        if let Some(value) = values.get(&node_id) {
            return Ok(*value);
        }

        let node = by_id.get(&node_id).ok_or(PowerSampleError::MissingNode)?;
        let SampleNodeKind::Internal { fanins, cubes } = &node.kind else {
            return Err(PowerSampleError::MissingNode);
        };

        let mut fanin_values = Vec::with_capacity(fanins.len());
        for fanin in fanins {
            if !by_id.contains_key(fanin) {
                return Err(PowerSampleError::MissingFanin);
            }
            fanin_values.push(self.simulate_word_rec(*fanin, by_id, values)?);
        }

        let result = simulate_node_word(cubes, &fanin_values)?;
        values.insert(node_id, result);
        Ok(result)
    }

    fn node_index(&self) -> Result<HashMap<SampleNodeId, &SampleNode<N>>, PowerSampleError> {
        let mut by_id = HashMap::with_capacity(self.nodes.len());
        for node in &self.nodes {
            if by_id.insert(node.id, node).is_some() {
                return Err(PowerSampleError::DuplicateNode);
            }
        }
        Ok(by_id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProgressPower {
    pub bit_iteration: usize,
    pub power: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerSampleEstimate {
    pub total_power: f64,
    pub output_one_counts: Vec<usize>,
    pub progress: Vec<ProgressPower>,
}

pub fn legacy_power_sample_estimate_blocked(
    _options: PowerSimulationOptions,
) -> Result<f64, PowerSampleError> {
    Err(PowerSampleError::MissingSisDependencies {
        operation: PowerSampleOperation::LegacySampleEstimate,
    })
}

pub fn legacy_power_simulation_estimate_blocked(
    _options: PowerSimulationOptions,
) -> Result<f64, PowerSampleError> {
    Err(PowerSampleError::MissingSisDependencies {
        operation: PowerSampleOperation::LegacySimulationEstimate,
    })
}

pub fn gen_random_pattern(
    probabilities: &[f64],
    seed: u32,
) -> Result<Vec<usize>, PowerSampleError> {
    let mut rng = SisRandom::new(seed);
    let mut pattern = vec![0; probabilities.len()];

    for (input, probability) in probabilities.iter().enumerate() {
        validate_probability(*probability)?;
        let threshold = (*probability * f64::from(RAND_RANGE)) as u32;
        for bit in 0..WORD_BITS {
            if rng.next() < threshold {
                pattern[input] |= 1usize << bit;
            }
        }
    }

    Ok(pattern)
}

pub fn simulate_node_word(
    cubes: &[SampleCube],
    fanin_values: &[usize],
) -> Result<usize, PowerSampleError> {
    let mut result = 0usize;

    for cube in cubes {
        if cube.literals.len() != fanin_values.len() {
            return Err(PowerSampleError::CubeArityMismatch {
                expected: fanin_values.len(),
                actual: cube.literals.len(),
            });
        }

        let mut and_result = usize::MAX;
        for (literal, value) in cube.literals.iter().zip(fanin_values.iter()) {
            and_result &= literal.apply(*value);
        }
        result |= and_result;
    }

    Ok(result)
}

fn power_sample_do_estimate<N>(
    network: &mut PowerSampleNetwork<N>,
    num_iter: usize,
    num_gap: usize,
    seed: u32,
    prob_pi: &[f64],
) -> Result<PowerSampleEstimate, PowerSampleError>
where
    N: Eq + Hash,
{
    if num_iter == 0 {
        return Err(PowerSampleError::EmptySampleCount);
    }

    let mut output_one_counts = vec![0usize; network.outputs.len()];
    let mut progress = Vec::new();
    let mut num_since_last = 0usize;

    for i in 0..num_iter {
        let pi_values = gen_random_pattern(prob_pi, seed.wrapping_add(i as u32))?;
        let po_values = network.simulate_word(&pi_values)?;

        for (count, value) in output_one_counts.iter_mut().zip(po_values.iter()) {
            *count += value.count_ones() as usize;
        }

        if num_since_last == num_gap {
            num_since_last = 0;
            progress.push(ProgressPower {
                bit_iteration: (i + 1) * WORD_BITS,
                power: network.calculate_power(&output_one_counts, i + 1)?,
            });
        } else {
            num_since_last += 1;
        }
    }

    let total_power = network.calculate_power(&output_one_counts, num_iter)?;
    Ok(PowerSampleEstimate {
        total_power,
        output_one_counts,
        progress,
    })
}

fn validate_probability(probability: f64) -> Result<(), PowerSampleError> {
    if (0.0..=1.0).contains(&probability) {
        Ok(())
    } else {
        Err(PowerSampleError::ProbabilityOutOfRange(probability))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SisRandom {
    state: u32,
}

impl SisRandom {
    fn new(seed: u32) -> Self {
        Self {
            state: seed & RAND_RANGE,
        }
    }

    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1_103_515_245).wrapping_add(12_345) & RAND_RANGE;
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn and_or_network() -> PowerSampleNetwork<&'static str> {
        let a = SampleNodeId(0);
        let b = SampleNodeId(1);
        let and = SampleNodeId(2);
        let or = SampleNodeId(3);

        PowerSampleNetwork::new(
            vec![
                SampleNode::primary_input(a, "a", 1.0),
                SampleNode::primary_input(b, "b", 0.0),
                SampleNode::internal(
                    and,
                    vec![a, b],
                    vec![SampleCube::new(vec![Literal::One, Literal::One])],
                    0.0,
                ),
                SampleNode::internal(
                    or,
                    vec![a, b],
                    vec![
                        SampleCube::new(vec![Literal::One, Literal::DontCare]),
                        SampleCube::new(vec![Literal::DontCare, Literal::One]),
                    ],
                    0.0,
                ),
            ],
            vec![a, b],
            vec![
                SampleOutput::new("and", and, 2.0),
                SampleOutput::new("or", or, 3.0),
            ],
        )
    }

    #[test]
    fn default_options_match_power_sample_estimate_setup() {
        assert_eq!(
            PowerSimulationOptions::simple(
                PowerCircuitType::Combinational,
                DelaySelection::Unit,
                9
            ),
            PowerSimulationOptions {
                circuit_type: PowerCircuitType::Combinational,
                delay: DelaySelection::Unit,
                ps_probability: PresentStateProbability::Approximation,
                num_samples: 9,
                sample_gap: 10,
                defaults: PowerSampleDefaults::default(),
            }
        );
    }

    #[test]
    fn random_pattern_honors_zero_and_one_probabilities() {
        let pattern = gen_random_pattern(&[0.0, 1.0], 123).unwrap();

        assert_eq!(pattern[0], 0);
        assert_eq!(pattern[1], usize::MAX);
    }

    #[test]
    fn node_word_simulation_matches_cube_sum_of_products() {
        let a = 0b1010usize;
        let b = 0b1100usize;

        assert_eq!(
            simulate_node_word(
                &[SampleCube::new(vec![Literal::One, Literal::Zero])],
                &[a, b],
            ),
            Ok(a & !b)
        );
        assert_eq!(
            simulate_node_word(
                &[
                    SampleCube::new(vec![Literal::One, Literal::DontCare]),
                    SampleCube::new(vec![Literal::DontCare, Literal::One]),
                ],
                &[a, b],
            ),
            Ok(a | b)
        );
    }

    #[test]
    fn simulate_network_recursively_reuses_primary_input_words() {
        let network = and_or_network();
        let values = network.simulate_word(&[usize::MAX, 0]).unwrap();

        assert_eq!(values, vec![0, usize::MAX]);
    }

    #[test]
    fn calculate_power_updates_output_switching_probabilities() {
        let mut network = and_or_network();
        let power = network
            .calculate_power(&[WORD_BITS / 2, WORD_BITS], 1)
            .unwrap();

        assert_eq!(network.outputs[0].switching_prob, 0.5);
        assert_eq!(network.outputs[1].switching_prob, 1.0);
        assert_eq!(power, (0.5 * 2.0 + 1.0 * 3.0) * CAPACITANCE * 250.0);
    }

    #[test]
    fn sample_estimate_counts_output_one_bits_and_reports_progress() {
        let mut network = and_or_network();
        let estimate = network.sample_estimate(2, 1, 7).unwrap();

        assert_eq!(estimate.output_one_counts, vec![0, WORD_BITS * 2]);
        assert_eq!(estimate.total_power, 3.0 * CAPACITANCE * 250.0);
        assert_eq!(estimate.progress.len(), 1);
        assert_eq!(estimate.progress[0].bit_iteration, WORD_BITS * 2);
    }

    #[test]
    fn rejects_invalid_probabilities_and_cube_shapes() {
        assert_eq!(
            gen_random_pattern(&[1.1], 0),
            Err(PowerSampleError::ProbabilityOutOfRange(1.1))
        );
        assert_eq!(
            simulate_node_word(&[SampleCube::new(vec![Literal::One])], &[1, 2]),
            Err(PowerSampleError::CubeArityMismatch {
                expected: 2,
                actual: 1,
            })
        );
    }
}
