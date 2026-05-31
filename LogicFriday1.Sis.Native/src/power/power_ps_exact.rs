//! Native Rust model for `LogicSynthesis/sis/power/power_psExact.c`.
//!
//! The C file extracts an STG, builds the stationary-state linear system from
//! transition input probabilities, solves it exactly, and projects state
//! probabilities onto present-state latch-output lines. This module ports that
//! algorithm to owned Rust data structures. Direct SIS `network_t`, `stg`,
//! `st_table`, and `spMatrix` integration remains represented by explicit
//! dependency errors.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

const SINGULAR_TOLERANCE: f64 = 1.0e-12;
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct PrimaryInputPowerInfo {
    pub node: NodeId,
    pub probability_one: f64,
    pub is_real_primary_input: bool,
}

impl PrimaryInputPowerInfo {
    pub fn new(node: NodeId, probability_one: f64, is_real_primary_input: bool) -> Self {
        Self {
            node,
            probability_one,
            is_real_primary_input,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExactPowerNetwork {
    pub primary_inputs: Vec<PrimaryInputPowerInfo>,
    pub latch_outputs: Vec<NodeId>,
}

impl ExactPowerNetwork {
    pub fn new(primary_inputs: Vec<PrimaryInputPowerInfo>, latch_outputs: Vec<NodeId>) -> Self {
        Self {
            primary_inputs,
            latch_outputs,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub id: StateId,
    pub encoding: String,
}

impl State {
    pub fn new(id: StateId, encoding: impl Into<String>) -> Self {
        Self {
            id,
            encoding: encoding.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub from: StateId,
    pub to: StateId,
    pub input: String,
}

impl Transition {
    pub fn new(from: StateId, to: StateId, input: impl Into<String>) -> Self {
        Self {
            from,
            to,
            input: input.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraph {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
}

impl StateGraph {
    pub fn new(states: Vec<State>, transitions: Vec<Transition>) -> Self {
        Self {
            states,
            transitions,
        }
    }

    fn state_index(&self) -> Result<BTreeMap<StateId, usize>, PowerPsExactError> {
        let mut index = BTreeMap::new();
        for (position, state) in self.states.iter().enumerate() {
            if index.insert(state.id, position).is_some() {
                return Err(PowerPsExactError::DuplicateState(state.id));
            }
        }
        Ok(index)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExactStateProbabilityReport {
    pub state_probabilities: Vec<f64>,
    pub state_index: BTreeMap<String, usize>,
    pub state_line_index: BTreeMap<NodeId, usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresentStateLineReport {
    pub state_probabilities: Vec<f64>,
    pub state_index: BTreeMap<String, usize>,
    pub state_line_index: BTreeMap<NodeId, usize>,
    pub present_state_line_probabilities: Vec<f64>,
    pub updated_primary_inputs: BTreeMap<NodeId, f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PowerPsExactError {
    EmptyStateGraph,
    DuplicateState(StateId),
    UnknownState(StateId),
    DuplicateStateEncoding(String),
    InvalidProbability {
        node: NodeId,
        probability: f64,
    },
    TransitionInputTooLong {
        input: String,
        primary_inputs: usize,
    },
    InvalidStateEncoding {
        state: StateId,
        encoding: String,
        expected_bits: usize,
    },
    SingularStationarySystem,
    MissingPresentStateLine {
        node: NodeId,
    },
    MissingNativePorts {
        operation: &'static str,
    },
}

impl fmt::Display for PowerPsExactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStateGraph => {
                write!(f, "exact state probability requires at least one state")
            }
            Self::DuplicateState(state) => write!(f, "duplicate STG state id {:?}", state),
            Self::UnknownState(state) => {
                write!(f, "transition references unknown STG state {:?}", state)
            }
            Self::DuplicateStateEncoding(encoding) => {
                write!(f, "duplicate STG state encoding {encoding}")
            }
            Self::InvalidProbability { node, probability } => write!(
                f,
                "primary input {:?} has probability {probability}, expected 0.0..=1.0",
                node
            ),
            Self::TransitionInputTooLong {
                input,
                primary_inputs,
            } => write!(
                f,
                "transition input {input} has more bits than {primary_inputs} primary inputs"
            ),
            Self::InvalidStateEncoding {
                state,
                encoding,
                expected_bits,
            } => write!(
                f,
                "state {:?} has encoding {encoding}, expected {expected_bits} binary bits",
                state
            ),
            Self::SingularStationarySystem => {
                write!(f, "stationary-state probability system is singular")
            }
            Self::MissingPresentStateLine { node } => write!(
                f,
                "non-real primary input {:?} is not present in the latch-output index",
                node
            ),
            Self::MissingNativePorts { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
        }
    }
}

impl Error for PowerPsExactError {}

pub fn power_exact_state_prob(
    network: &ExactPowerNetwork,
    stg: &StateGraph,
) -> Result<ExactStateProbabilityReport, PowerPsExactError> {
    let state_probabilities = compute_state_probabilities(network, stg)?;
    Ok(ExactStateProbabilityReport {
        state_probabilities,
        state_index: generate_state_index(stg)?,
        state_line_index: generate_present_state_line_index(network),
    })
}

pub fn power_present_state_lines_from_state(
    network: &mut ExactPowerNetwork,
    stg: &StateGraph,
) -> Result<PresentStateLineReport, PowerPsExactError> {
    let state_probabilities = compute_state_probabilities(network, stg)?;
    let state_index = generate_state_index(stg)?;
    let state_line_index = generate_present_state_line_index(network);
    let present_state_line_probabilities = project_state_probabilities_to_present_state_lines(
        stg,
        &state_probabilities,
        network.latch_outputs.len(),
    )?;
    let updated_primary_inputs = update_present_state_line_probabilities(
        network,
        &present_state_line_probabilities,
        &state_line_index,
    )?;

    Ok(PresentStateLineReport {
        state_probabilities,
        state_index,
        state_line_index,
        present_state_line_probabilities,
        updated_primary_inputs,
    })
}

pub fn compute_state_probabilities(
    network: &ExactPowerNetwork,
    stg: &StateGraph,
) -> Result<Vec<f64>, PowerPsExactError> {
    if stg.states.is_empty() {
        return Err(PowerPsExactError::EmptyStateGraph);
    }

    let state_lookup = stg.state_index()?;
    let input_probabilities = primary_input_probabilities(network)?;
    let matrix_size = stg.states.len();
    let mut matrix = vec![vec![0.0; matrix_size]; matrix_size];

    for (row, state) in stg
        .states
        .iter()
        .enumerate()
        .take(matrix_size.saturating_sub(1))
    {
        for transition in stg
            .transitions
            .iter()
            .filter(|transition| transition.to == state.id)
        {
            let column = *state_lookup
                .get(&transition.from)
                .ok_or(PowerPsExactError::UnknownState(transition.from))?;
            matrix[row][column] += transition_probability(&transition.input, &input_probabilities)?;
        }
        matrix[row][row] -= 1.0;
    }

    for entry in &mut matrix[matrix_size - 1] {
        *entry = 1.0;
    }

    for transition in &stg.transitions {
        if !state_lookup.contains_key(&transition.to) {
            return Err(PowerPsExactError::UnknownState(transition.to));
        }
    }

    let mut rhs = vec![0.0; matrix_size];
    rhs[matrix_size - 1] = 1.0;
    solve_linear_system(matrix, rhs)
}

pub fn project_state_probabilities_to_present_state_lines(
    stg: &StateGraph,
    state_probabilities: &[f64],
    present_state_line_count: usize,
) -> Result<Vec<f64>, PowerPsExactError> {
    let mut probabilities = vec![0.0; present_state_line_count];

    for (state, probability) in stg.states.iter().zip(state_probabilities.iter()) {
        if state.encoding.len() != present_state_line_count
            || !state
                .encoding
                .bytes()
                .all(|value| matches!(value, b'0' | b'1'))
        {
            return Err(PowerPsExactError::InvalidStateEncoding {
                state: state.id,
                encoding: state.encoding.clone(),
                expected_bits: present_state_line_count,
            });
        }

        for (index, value) in state.encoding.bytes().enumerate() {
            if value == b'1' {
                probabilities[index] += *probability;
            }
        }
    }

    Ok(probabilities)
}

pub fn generate_present_state_line_index(network: &ExactPowerNetwork) -> BTreeMap<NodeId, usize> {
    network
        .latch_outputs
        .iter()
        .copied()
        .enumerate()
        .map(|(index, node)| (node, index))
        .collect()
}

pub fn update_present_state_line_probabilities(
    network: &mut ExactPowerNetwork,
    present_state_line_probabilities: &[f64],
    state_line_index: &BTreeMap<NodeId, usize>,
) -> Result<BTreeMap<NodeId, f64>, PowerPsExactError> {
    let mut updated = BTreeMap::new();

    for primary_input in &mut network.primary_inputs {
        if primary_input.is_real_primary_input {
            continue;
        }
        let index = state_line_index.get(&primary_input.node).copied().ok_or(
            PowerPsExactError::MissingPresentStateLine {
                node: primary_input.node,
            },
        )?;
        primary_input.probability_one = present_state_line_probabilities[index];
        updated.insert(primary_input.node, primary_input.probability_one);
    }

    Ok(updated)
}

pub fn power_present_state_lines_from_sis_network<Network, InfoTable>(
    _network: &mut Network,
    _info_table: &mut InfoTable,
) -> Result<PresentStateLineReport, PowerPsExactError> {
    Err(PowerPsExactError::MissingNativePorts {
        operation: "power_PS_lines_from_state",
    })
}

pub fn power_exact_state_prob_from_sis_network<Network, InfoTable>(
    _network: &Network,
    _info_table: &InfoTable,
) -> Result<ExactStateProbabilityReport, PowerPsExactError> {
    Err(PowerPsExactError::MissingNativePorts {
        operation: "power_exact_state_prob",
    })
}

fn primary_input_probabilities(network: &ExactPowerNetwork) -> Result<Vec<f64>, PowerPsExactError> {
    network
        .primary_inputs
        .iter()
        .map(|input| {
            if (0.0..=1.0).contains(&input.probability_one) {
                Ok(input.probability_one)
            } else {
                Err(PowerPsExactError::InvalidProbability {
                    node: input.node,
                    probability: input.probability_one,
                })
            }
        })
        .collect()
}

fn generate_state_index(stg: &StateGraph) -> Result<BTreeMap<String, usize>, PowerPsExactError> {
    let mut index = BTreeMap::new();
    for (position, state) in stg.states.iter().enumerate() {
        if index.insert(state.encoding.clone(), position).is_some() {
            return Err(PowerPsExactError::DuplicateStateEncoding(
                state.encoding.clone(),
            ));
        }
    }
    Ok(index)
}

fn transition_probability(
    transition_input: &str,
    input_probabilities: &[f64],
) -> Result<f64, PowerPsExactError> {
    if transition_input.len() > input_probabilities.len() {
        return Err(PowerPsExactError::TransitionInputTooLong {
            input: transition_input.to_owned(),
            primary_inputs: input_probabilities.len(),
        });
    }

    let mut probability = 1.0;
    for (index, value) in transition_input.bytes().enumerate() {
        match value {
            b'0' => probability *= 1.0 - input_probabilities[index],
            b'1' => probability *= input_probabilities[index],
            _ => {}
        }
    }
    Ok(probability)
}

fn solve_linear_system(
    mut matrix: Vec<Vec<f64>>,
    mut rhs: Vec<f64>,
) -> Result<Vec<f64>, PowerPsExactError> {
    let size = rhs.len();

    for pivot_column in 0..size {
        let mut pivot_row = pivot_column;
        let mut pivot_abs = matrix[pivot_row][pivot_column].abs();
        for (candidate, row) in matrix.iter().enumerate().skip(pivot_column + 1) {
            let candidate_abs = row[pivot_column].abs();
            if candidate_abs > pivot_abs {
                pivot_abs = candidate_abs;
                pivot_row = candidate;
            }
        }

        if pivot_abs <= SINGULAR_TOLERANCE {
            return Err(PowerPsExactError::SingularStationarySystem);
        }

        if pivot_row != pivot_column {
            matrix.swap(pivot_row, pivot_column);
            rhs.swap(pivot_row, pivot_column);
        }

        for row in (pivot_column + 1)..size {
            let factor = matrix[row][pivot_column] / matrix[pivot_column][pivot_column];
            if factor == 0.0 {
                continue;
            }
            matrix[row][pivot_column] = 0.0;
            for column in (pivot_column + 1)..size {
                matrix[row][column] -= factor * matrix[pivot_column][column];
            }
            rhs[row] -= factor * rhs[pivot_column];
        }
    }

    let mut solution = vec![0.0; size];
    for row in (0..size).rev() {
        let known: f64 = ((row + 1)..size)
            .map(|column| matrix[row][column] * solution[column])
            .sum();
        solution[row] = (rhs[row] - known) / matrix[row][row];
    }

    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pi(id: usize, probability_one: f64, is_real_primary_input: bool) -> PrimaryInputPowerInfo {
        PrimaryInputPowerInfo::new(NodeId(id), probability_one, is_real_primary_input)
    }

    fn state(id: usize, encoding: &str) -> State {
        State::new(StateId(id), encoding)
    }

    fn transition(from: usize, to: usize, input: &str) -> Transition {
        Transition::new(StateId(from), StateId(to), input)
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-10,
            "actual {actual}, expected {expected}"
        );
    }

    #[test]
    fn exact_state_probabilities_match_stationary_markov_solution() {
        let network = ExactPowerNetwork::new(vec![pi(0, 0.25, true)], vec![NodeId(10)]);
        let stg = StateGraph::new(
            vec![state(0, "0"), state(1, "1")],
            vec![
                transition(0, 0, "0"),
                transition(0, 1, "1"),
                transition(1, 0, "1"),
                transition(1, 1, "0"),
            ],
        );

        let report = power_exact_state_prob(&network, &stg).unwrap();

        assert_close(report.state_probabilities[0], 0.5);
        assert_close(report.state_probabilities[1], 0.5);
        assert_eq!(report.state_index["0"], 0);
        assert_eq!(report.state_index["1"], 1);
        assert_eq!(report.state_line_index[&NodeId(10)], 0);
    }

    #[test]
    fn transition_input_ignores_non_binary_positions_like_c_switch_default() {
        let network =
            ExactPowerNetwork::new(vec![pi(0, 0.25, true), pi(1, 0.9, true)], vec![NodeId(10)]);
        let stg = StateGraph::new(
            vec![state(0, "0"), state(1, "1")],
            vec![
                transition(0, 0, "0-"),
                transition(0, 1, "1-"),
                transition(1, 0, "1-"),
                transition(1, 1, "0-"),
            ],
        );

        let report = power_exact_state_prob(&network, &stg).unwrap();

        assert_close(report.state_probabilities[0], 0.5);
        assert_close(report.state_probabilities[1], 0.5);
    }

    #[test]
    fn present_state_lines_are_projected_and_written_to_non_real_inputs() {
        let mut network = ExactPowerNetwork::new(
            vec![pi(100, 0.1, true), pi(10, 0.0, false), pi(11, 0.0, false)],
            vec![NodeId(10), NodeId(11)],
        );
        let stg = StateGraph::new(
            vec![state(0, "00"), state(1, "01"), state(2, "10")],
            vec![
                transition(0, 1, ""),
                transition(1, 2, ""),
                transition(2, 0, ""),
            ],
        );

        let report = power_present_state_lines_from_state(&mut network, &stg).unwrap();

        for probability in &report.state_probabilities {
            assert_close(*probability, 1.0 / 3.0);
        }
        assert_close(report.present_state_line_probabilities[0], 1.0 / 3.0);
        assert_close(report.present_state_line_probabilities[1], 1.0 / 3.0);
        assert_close(report.updated_primary_inputs[&NodeId(10)], 1.0 / 3.0);
        assert_close(report.updated_primary_inputs[&NodeId(11)], 1.0 / 3.0);
        assert_close(network.primary_inputs[1].probability_one, 1.0 / 3.0);
        assert_close(network.primary_inputs[2].probability_one, 1.0 / 3.0);
        assert_close(network.primary_inputs[0].probability_one, 0.1);
    }

    #[test]
    fn rejects_invalid_probability_and_state_encoding() {
        let network = ExactPowerNetwork::new(vec![pi(0, 1.5, true)], vec![NodeId(10)]);
        let stg = StateGraph::new(vec![state(0, "0")], Vec::new());

        assert_eq!(
            compute_state_probabilities(&network, &stg),
            Err(PowerPsExactError::InvalidProbability {
                node: NodeId(0),
                probability: 1.5,
            })
        );

        let stg = StateGraph::new(vec![state(0, "2")], Vec::new());
        assert_eq!(
            project_state_probabilities_to_present_state_lines(&stg, &[1.0], 1),
            Err(PowerPsExactError::InvalidStateEncoding {
                state: StateId(0),
                encoding: "2".to_string(),
                expected_bits: 1,
            })
        );
    }

    #[test]
    fn missing_present_state_line_is_explicit() {
        let mut network = ExactPowerNetwork::new(vec![pi(99, 0.0, false)], vec![NodeId(10)]);
        let state_line_index = generate_present_state_line_index(&network);

        assert_eq!(
            update_present_state_line_probabilities(&mut network, &[0.5], &state_line_index),
            Err(PowerPsExactError::MissingPresentStateLine { node: NodeId(99) })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("power_ps_exact.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
