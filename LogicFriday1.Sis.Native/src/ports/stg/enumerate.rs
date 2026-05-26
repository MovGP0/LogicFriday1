//! Native Rust port model for `sis/stg/enumerate.c`.
//!
//! The original file mixes two responsibilities: packed latch-state storage
//! and recursive sequential circuit enumeration over SIS `network_t`, `node_t`,
//! `graph_t`, and STG simulation globals. The packed-state logic is represented
//! as an owned Rust type. Enumeration is exposed as a native Rust API that
//! accepts a simulator callback and emits the Rust STG model directly.

use super::stg::{StateId, Stg, StgError};
use super::stg_sc_sim::LogicValue;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

pub const STG_CHUNK_SIZE: usize = 1000;
pub const MAX_ELENGTH: usize = 36;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackedStateTable {
    bits_per_word: usize,
    latch_count: usize,
    words_per_state: usize,
    states: HashSet<Vec<u32>>,
}

impl PackedStateTable {
    pub fn new(latch_count: usize, bits_per_word: usize) -> Result<Self, EnumerateError> {
        if bits_per_word == 0 || bits_per_word > u32::BITS as usize {
            return Err(EnumerateError::InvalidBitsPerWord(bits_per_word));
        }

        let words_per_state = if latch_count == 0 {
            0
        } else {
            latch_count.div_ceil(bits_per_word)
        };

        Ok(Self {
            bits_per_word,
            latch_count,
            words_per_state,
            states: HashSet::new(),
        })
    }

    pub fn bits_per_word(&self) -> usize {
        self.bits_per_word
    }

    pub fn latch_count(&self) -> usize {
        self.latch_count
    }

    pub fn words_per_state(&self) -> usize {
        self.words_per_state
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn hash_code(&mut self, estate: &[u8]) -> Result<Vec<u32>, EnumerateError> {
        let packed = self.pack_state(estate)?;
        self.states.insert(packed.clone());
        Ok(packed)
    }

    pub fn contains_packed(&self, packed: &[u32]) -> bool {
        self.states.contains(packed)
    }

    pub fn pack_state(&self, estate: &[u8]) -> Result<Vec<u32>, EnumerateError> {
        if estate.len() != self.latch_count {
            return Err(EnumerateError::StateLength {
                expected: self.latch_count,
                actual: estate.len(),
            });
        }
        if let Some((index, value)) = estate
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| *value > 1)
        {
            return Err(EnumerateError::InvalidStateBit { index, value });
        }

        let mut hashed = vec![0u32; self.words_per_state];
        let mut next_width = self.latch_count % self.bits_per_word;
        if next_width == 0 {
            next_width = self.bits_per_word;
        }

        let mut k = self.latch_count;
        for i in (0..self.words_per_state).rev() {
            let mut state = 0u32;
            for _ in 0..next_width {
                k -= 1;
                state = (state << 1) + estate[k] as u32;
            }
            hashed[i] = state;
            next_width = self.bits_per_word;
        }

        Ok(hashed)
    }

    pub fn translate_hashed_code(&self, h_state: &[u32]) -> Result<Vec<u8>, EnumerateError> {
        if h_state.len() != self.words_per_state {
            return Err(EnumerateError::PackedLength {
                expected: self.words_per_state,
                actual: h_state.len(),
            });
        }

        let mut stg_state = Vec::with_capacity(self.latch_count);
        for compact in h_state.iter().copied() {
            let mut compact_state = compact;
            for _ in 0..self.bits_per_word {
                if stg_state.len() == self.latch_count {
                    return Ok(stg_state);
                }
                stg_state.push((compact_state & 1) as u8);
                compact_state >>= 1;
            }
        }

        Ok(stg_state)
    }

    pub fn state_hash(&self, packed: &[u32], modulus: u32) -> Result<u32, EnumerateError> {
        if modulus == 0 {
            return Err(EnumerateError::ZeroModulus);
        }
        if packed.len() != self.words_per_state {
            return Err(EnumerateError::PackedLength {
                expected: self.words_per_state,
                actual: packed.len(),
            });
        }

        Ok(packed.first().copied().unwrap_or_default() % modulus)
    }

    pub fn compare_states(&self, left: &[u32], right: &[u32]) -> Result<i32, EnumerateError> {
        if left.len() != self.words_per_state {
            return Err(EnumerateError::PackedLength {
                expected: self.words_per_state,
                actual: left.len(),
            });
        }
        if right.len() != self.words_per_state {
            return Err(EnumerateError::PackedLength {
                expected: self.words_per_state,
                actual: right.len(),
            });
        }

        for i in (0..self.words_per_state).rev() {
            if left[i] > right[i] {
                return Ok(1);
            }
            if left[i] < right[i] {
                return Ok(-1);
            }
        }
        Ok(0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumerationRequest {
    pub latch_count: usize,
    pub primary_input_count: usize,
    pub primary_output_count: usize,
    pub max_depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitObservation {
    pub next_state: Vec<LogicValue>,
    pub outputs: Vec<LogicValue>,
}

impl CircuitObservation {
    pub fn new(next_state: Vec<LogicValue>, outputs: Vec<LogicValue>) -> Self {
        Self {
            next_state,
            outputs,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumerationResult {
    pub stg: Stg,
    pub unfinished_states: Vec<Vec<u8>>,
}

pub fn enumerate_sequential_circuit<F>(
    request: &EnumerationRequest,
    initial_state: &[u8],
    simulate: F,
) -> Result<EnumerationResult, EnumerateError>
where
    F: FnMut(&[u8], &[u8]) -> Result<Option<CircuitObservation>, EnumerateError>,
{
    enumerate_from_initial_state(request, initial_state, simulate)
}

pub fn enumerate_from_initial_state<F>(
    request: &EnumerationRequest,
    initial_state: &[u8],
    mut simulate: F,
) -> Result<EnumerationResult, EnumerateError>
where
    F: FnMut(&[u8], &[u8]) -> Result<Option<CircuitObservation>, EnumerateError>,
{
    validate_request(request)?;
    validate_binary_state(initial_state, request.latch_count)?;

    let bits_per_word = u32::BITS as usize;
    let mut packed_states = PackedStateTable::new(request.latch_count, bits_per_word)?;
    let mut stg = Stg::with_dimensions(request.primary_input_count, request.primary_output_count);
    let mut state_ids = HashMap::new();
    let mut unfinished_states = Vec::new();

    let first_state = initial_state.to_vec();
    packed_states.hash_code(&first_state)?;
    enumerate_reachable_state(
        request,
        &first_state,
        0,
        &mut simulate,
        &mut packed_states,
        &mut stg,
        &mut state_ids,
        &mut unfinished_states,
    )?;

    Ok(EnumerationResult {
        stg,
        unfinished_states,
    })
}

pub fn enumerate_complete_state_table<F>(
    request: &EnumerationRequest,
    mut simulate: F,
) -> Result<EnumerationResult, EnumerateError>
where
    F: FnMut(&[u8], &[u8]) -> Result<Option<CircuitObservation>, EnumerateError>,
{
    validate_request(request)?;
    if request.latch_count > 16 {
        return Err(EnumerateError::TooManyLatchesForCompleteTable {
            latches: request.latch_count,
            max: 16,
        });
    }

    let mut stg = Stg::with_dimensions(request.primary_input_count, request.primary_output_count);
    let mut state_ids = HashMap::new();
    let total_states = 1usize << request.latch_count;

    for state_number in 0..total_states {
        let present_state = state_from_number(state_number, request.latch_count);
        enumerate_state_edges(
            request,
            &present_state,
            &mut simulate,
            &mut stg,
            &mut state_ids,
        )?;
    }

    Ok(EnumerationResult {
        stg,
        unfinished_states: Vec::new(),
    })
}

fn enumerate_reachable_state<F>(
    request: &EnumerationRequest,
    present_state: &[u8],
    depth: usize,
    simulate: &mut F,
    packed_states: &mut PackedStateTable,
    stg: &mut Stg,
    state_ids: &mut HashMap<String, StateId>,
    unfinished_states: &mut Vec<Vec<u8>>,
) -> Result<(), EnumerateError>
where
    F: FnMut(&[u8], &[u8]) -> Result<Option<CircuitObservation>, EnumerateError>,
{
    for input_number in 0..input_assignment_count(request.primary_input_count)? {
        let inputs = state_from_number(input_number, request.primary_input_count);
        let Some(observation) = simulate(present_state, &inputs)? else {
            continue;
        };
        let next_state = observation_state(&observation, request.latch_count)?;

        save_edge(
            request,
            stg,
            state_ids,
            present_state,
            &next_state,
            &inputs,
            &observation.outputs,
        )?;

        let packed = packed_states.pack_state(&next_state)?;
        if packed_states.contains_packed(&packed) {
            continue;
        }

        packed_states.hash_code(&next_state)?;
        if depth + 1 == request.max_depth {
            unfinished_states.push(next_state);
        } else {
            enumerate_reachable_state(
                request,
                &next_state,
                depth + 1,
                simulate,
                packed_states,
                stg,
                state_ids,
                unfinished_states,
            )?;
        }
    }

    Ok(())
}

fn enumerate_state_edges<F>(
    request: &EnumerationRequest,
    present_state: &[u8],
    simulate: &mut F,
    stg: &mut Stg,
    state_ids: &mut HashMap<String, StateId>,
) -> Result<(), EnumerateError>
where
    F: FnMut(&[u8], &[u8]) -> Result<Option<CircuitObservation>, EnumerateError>,
{
    for input_number in 0..input_assignment_count(request.primary_input_count)? {
        let inputs = state_from_number(input_number, request.primary_input_count);
        let Some(observation) = simulate(present_state, &inputs)? else {
            continue;
        };
        let next_state = observation_state(&observation, request.latch_count)?;
        save_edge(
            request,
            stg,
            state_ids,
            present_state,
            &next_state,
            &inputs,
            &observation.outputs,
        )?;
    }

    Ok(())
}

fn save_edge(
    request: &EnumerationRequest,
    stg: &mut Stg,
    state_ids: &mut HashMap<String, StateId>,
    present_state: &[u8],
    next_state: &[u8],
    inputs: &[u8],
    outputs: &[LogicValue],
) -> Result<(), EnumerateError> {
    if inputs.len() != request.primary_input_count {
        return Err(EnumerateError::InputLength {
            expected: request.primary_input_count,
            actual: inputs.len(),
        });
    }
    if outputs.len() != request.primary_output_count {
        return Err(EnumerateError::OutputLength {
            expected: request.primary_output_count,
            actual: outputs.len(),
        });
    }

    let is_first_state = stg.num_states() == 0;
    let from_name = binary_state_string(present_state)?;
    let to_name = binary_state_string(next_state)?;
    let from = get_or_create_state(stg, state_ids, &from_name);
    let to = get_or_create_state(stg, state_ids, &to_name);

    if is_first_state {
        stg.set_start(from)?;
        stg.set_current(from)?;
    }

    stg.create_transition(
        from,
        to,
        binary_input_string(inputs)?,
        logic_values_string(outputs),
    )?;
    Ok(())
}

fn get_or_create_state(
    stg: &mut Stg,
    state_ids: &mut HashMap<String, StateId>,
    name: &str,
) -> StateId {
    if let Some(state) = state_ids.get(name) {
        return *state;
    }

    let state = stg.create_state(Some(name), Some(name));
    state_ids.insert(name.to_owned(), state);
    state
}

fn observation_state(
    observation: &CircuitObservation,
    latch_count: usize,
) -> Result<Vec<u8>, EnumerateError> {
    if observation.next_state.len() != latch_count {
        return Err(EnumerateError::StateLength {
            expected: latch_count,
            actual: observation.next_state.len(),
        });
    }

    observation
        .next_state
        .iter()
        .copied()
        .enumerate()
        .map(|(index, value)| match value {
            LogicValue::Zero => Ok(0),
            LogicValue::One => Ok(1),
            LogicValue::Unknown => Err(EnumerateError::UnresolvedNextStateBit { index }),
        })
        .collect()
}

fn validate_request(request: &EnumerationRequest) -> Result<(), EnumerateError> {
    if request.max_depth > MAX_ELENGTH {
        return Err(EnumerateError::DepthLimit {
            requested: request.max_depth,
            max: MAX_ELENGTH,
        });
    }
    if request.max_depth == 0 {
        return Err(EnumerateError::ZeroDepth);
    }
    input_assignment_count(request.primary_input_count)?;
    Ok(())
}

fn validate_binary_state(state: &[u8], latch_count: usize) -> Result<(), EnumerateError> {
    if state.len() != latch_count {
        return Err(EnumerateError::StateLength {
            expected: latch_count,
            actual: state.len(),
        });
    }
    if let Some((index, value)) = state
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| *value > 1)
    {
        return Err(EnumerateError::InvalidStateBit { index, value });
    }
    Ok(())
}

fn input_assignment_count(primary_input_count: usize) -> Result<usize, EnumerateError> {
    1usize
        .checked_shl(primary_input_count as u32)
        .ok_or(EnumerateError::TooManyPrimaryInputs {
            inputs: primary_input_count,
        })
}

fn state_from_number(mut number: usize, width: usize) -> Vec<u8> {
    let mut state = vec![0; width];
    for bit in &mut state {
        *bit = (number & 1) as u8;
        number >>= 1;
    }
    state
}

fn binary_state_string(state: &[u8]) -> Result<String, EnumerateError> {
    validate_binary_state(state, state.len())?;
    Ok(state
        .iter()
        .map(|value| if *value == 0 { '0' } else { '1' })
        .collect())
}

fn binary_input_string(inputs: &[u8]) -> Result<String, EnumerateError> {
    binary_state_string(inputs)
}

fn logic_values_string(values: &[LogicValue]) -> String {
    values.iter().map(|value| value.state_char()).collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnumerateError {
    InvalidBitsPerWord(usize),
    StateLength { expected: usize, actual: usize },
    PackedLength { expected: usize, actual: usize },
    InvalidStateBit { index: usize, value: u8 },
    ZeroModulus,
    DepthLimit { requested: usize, max: usize },
    ZeroDepth,
    InputLength { expected: usize, actual: usize },
    OutputLength { expected: usize, actual: usize },
    TooManyPrimaryInputs { inputs: usize },
    TooManyLatchesForCompleteTable { latches: usize, max: usize },
    UnresolvedNextStateBit { index: usize },
    Stg(StgError),
}

impl From<StgError> for EnumerateError {
    fn from(value: StgError) -> Self {
        Self::Stg(value)
    }
}

impl fmt::Display for EnumerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBitsPerWord(value) => {
                write!(f, "invalid bits-per-word value {value}; expected 1..=32")
            }
            Self::StateLength { expected, actual } => {
                write!(
                    f,
                    "state length {actual} does not match latch count {expected}"
                )
            }
            Self::PackedLength { expected, actual } => {
                write!(
                    f,
                    "packed state length {actual} does not match word count {expected}"
                )
            }
            Self::InvalidStateBit { index, value } => {
                write!(f, "state bit at index {index} has invalid value {value}")
            }
            Self::ZeroModulus => write!(f, "hash modulus must be non-zero"),
            Self::DepthLimit { requested, max } => {
                write!(
                    f,
                    "requested enumeration depth {requested} exceeds MAX_ELENGTH {max}"
                )
            }
            Self::ZeroDepth => write!(f, "enumeration depth must be greater than zero"),
            Self::InputLength { expected, actual } => {
                write!(
                    f,
                    "input assignment length {actual} does not match primary input count {expected}"
                )
            }
            Self::OutputLength { expected, actual } => {
                write!(
                    f,
                    "output vector length {actual} does not match primary output count {expected}"
                )
            }
            Self::TooManyPrimaryInputs { inputs } => {
                write!(
                    f,
                    "{inputs} primary inputs produce too many input assignments for this platform"
                )
            }
            Self::TooManyLatchesForCompleteTable { latches, max } => {
                write!(
                    f,
                    "complete state table enumeration supports at most {max} latches; got {latches}"
                )
            }
            Self::UnresolvedNextStateBit { index } => {
                write!(f, "next-state bit {index} remained unresolved")
            }
            Self::Stg(error) => write!(f, "{error}"),
        }
    }
}

impl Error for EnumerateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_state_matches_c_shashcode_word_order() {
        let table = PackedStateTable::new(5, 4).unwrap();

        assert_eq!(table.pack_state(&[1, 0, 1, 1, 0]).unwrap(), vec![0b1101, 0]);
        assert_eq!(
            table.translate_hashed_code(&[0b1101, 0]).unwrap(),
            vec![1, 0, 1, 1, 0]
        );
    }

    #[test]
    fn pack_state_handles_partial_most_significant_word_first() {
        let table = PackedStateTable::new(10, 4).unwrap();

        let packed = table.pack_state(&[1, 0, 1, 1, 0, 0, 1, 0, 1, 1]).unwrap();

        assert_eq!(packed, vec![0b1101, 0b0100, 0b11]);
        assert_eq!(
            table.translate_hashed_code(&packed).unwrap(),
            vec![1, 0, 1, 1, 0, 0, 1, 0, 1, 1]
        );
    }

    #[test]
    fn hash_code_interns_packed_states_like_st_storelist() {
        let mut table = PackedStateTable::new(4, 4).unwrap();

        let first = table.hash_code(&[1, 0, 0, 1]).unwrap();
        let second = table.hash_code(&[1, 0, 0, 1]).unwrap();
        let third = table.hash_code(&[0, 0, 0, 1]).unwrap();

        assert_eq!(first, second);
        assert_ne!(first, third);
        assert_eq!(table.len(), 2);
        assert!(table.contains_packed(&first));
    }

    #[test]
    fn compare_and_hash_match_c_helpers() {
        let table = PackedStateTable::new(8, 4).unwrap();

        assert_eq!(table.compare_states(&[3, 1], &[3, 2]).unwrap(), -1);
        assert_eq!(table.compare_states(&[9, 2], &[3, 2]).unwrap(), 1);
        assert_eq!(table.compare_states(&[3, 2], &[3, 2]).unwrap(), 0);
        assert_eq!(table.state_hash(&[17, 2], 5).unwrap(), 2);
    }

    #[test]
    fn sequential_circuit_entrypoint_uses_native_simulator_callback() {
        let request = EnumerationRequest {
            latch_count: 1,
            primary_input_count: 1,
            primary_output_count: 1,
            max_depth: MAX_ELENGTH,
        };

        let result = enumerate_sequential_circuit(&request, &[0], |_state, inputs| {
            Ok(Some(CircuitObservation::new(
                vec![LogicValue::from_bool(inputs[0] == 1)],
                vec![LogicValue::from_bool(inputs[0] == 0)],
            )))
        })
        .unwrap();

        assert_eq!(result.stg.num_states(), 2);
        assert_eq!(result.stg.num_products(), 4);
    }

    #[test]
    fn enumerate_from_initial_state_builds_reachable_stg() {
        let request = EnumerationRequest {
            latch_count: 1,
            primary_input_count: 1,
            primary_output_count: 1,
            max_depth: MAX_ELENGTH,
        };

        let result = enumerate_from_initial_state(&request, &[0], |state, inputs| {
            Ok(Some(CircuitObservation::new(
                vec![LogicValue::from_bool(inputs[0] == 1)],
                vec![LogicValue::from_bool(state[0] == 1)],
            )))
        })
        .unwrap();

        assert!(result.unfinished_states.is_empty());
        assert_eq!(result.stg.num_states(), 2);
        assert_eq!(result.stg.num_products(), 4);
        assert_eq!(
            result.stg.state_encoding(result.stg.start().unwrap()),
            Some("0")
        );
        assert!(
            result
                .stg
                .transitions()
                .iter()
                .any(|transition| transition.input == "1" && transition.output == "0")
        );
        assert!(
            result
                .stg
                .transitions()
                .iter()
                .any(|transition| transition.input == "0" && transition.output == "1")
        );
    }

    #[test]
    fn enumerate_from_initial_state_records_depth_limited_unfinished_states() {
        let request = EnumerationRequest {
            latch_count: 2,
            primary_input_count: 1,
            primary_output_count: 0,
            max_depth: 1,
        };

        let result = enumerate_from_initial_state(&request, &[0, 0], |_state, inputs| {
            Ok(Some(CircuitObservation::new(
                vec![LogicValue::Zero, LogicValue::from_bool(inputs[0] == 1)],
                vec![],
            )))
        })
        .unwrap();

        assert_eq!(result.unfinished_states, vec![vec![0, 1]]);
        assert_eq!(result.stg.num_states(), 2);
        assert_eq!(result.stg.num_products(), 2);
    }

    #[test]
    fn enumerate_complete_state_table_visits_every_present_state() {
        let request = EnumerationRequest {
            latch_count: 2,
            primary_input_count: 0,
            primary_output_count: 1,
            max_depth: MAX_ELENGTH,
        };

        let result = enumerate_complete_state_table(&request, |state, _inputs| {
            Ok(Some(CircuitObservation::new(
                vec![
                    LogicValue::from_bool(state[1] == 1),
                    LogicValue::from_bool(state[0] == 0),
                ],
                vec![LogicValue::from_bool(state == [1, 1])],
            )))
        })
        .unwrap();

        assert!(result.unfinished_states.is_empty());
        assert_eq!(result.stg.num_states(), 4);
        assert_eq!(result.stg.num_products(), 4);
        assert!(
            result
                .stg
                .states()
                .iter()
                .any(|state| state.encoding.as_deref() == Some("11"))
        );
    }

    #[test]
    fn enumeration_validates_callback_vectors() {
        let request = EnumerationRequest {
            latch_count: 1,
            primary_input_count: 0,
            primary_output_count: 1,
            max_depth: MAX_ELENGTH,
        };

        assert_eq!(
            enumerate_from_initial_state(&request, &[0], |_state, _inputs| {
                Ok(Some(CircuitObservation::new(
                    vec![LogicValue::Unknown],
                    vec![LogicValue::Zero],
                )))
            }),
            Err(EnumerateError::UnresolvedNextStateBit { index: 0 })
        );

        assert_eq!(
            enumerate_from_initial_state(&request, &[0], |_state, _inputs| {
                Ok(Some(CircuitObservation::new(
                    vec![LogicValue::Zero],
                    vec![LogicValue::Zero, LogicValue::One],
                )))
            }),
            Err(EnumerateError::OutputLength {
                expected: 1,
                actual: 2,
            })
        );
    }
}
