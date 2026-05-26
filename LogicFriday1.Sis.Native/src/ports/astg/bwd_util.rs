//! Native utilities for bounded-wire-delay ASTG/STG conversion support.
//!
//! The legacy unit mixes small string/cube/STG helpers with routines that
//! require the SIS graph, ASTG marking, and network layers. This module ports
//! the helpers that can run on native Rust data structures and keeps the
//! graph/network entry points explicit as blocked operations until those layers
//! are available to this ASTG port.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub type AstgScode = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind {
    Input,
    Output,
    Internal,
    Dummy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgSignal {
    pub name: String,
    pub kind: SignalKind,
    pub state_bit: AstgScode,
}

impl AstgSignal {
    pub fn new(name: impl Into<String>, kind: SignalKind, state_bit: AstgScode) -> Self {
        Self {
            name: name.into(),
            kind,
            state_bit,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkedPlaces {
    pub n_elem: usize,
    pub bit_array: Vec<u32>,
}

impl MarkedPlaces {
    pub fn new(n_elem: usize, bit_array: impl Into<Vec<u32>>) -> Self {
        Self {
            n_elem,
            bit_array: bit_array.into(),
        }
    }

    pub fn n_word(&self) -> usize {
        self.bit_array.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgMarking {
    pub marked_places: MarkedPlaces,
    pub enabled: AstgScode,
}

impl AstgMarking {
    pub fn new(marked_places: MarkedPlaces, enabled: AstgScode) -> Self {
        Self {
            marked_places,
            enabled,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TransitionId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub name: String,
    pub encoding: Option<String>,
}

impl State {
    pub fn new(name: impl Into<String>, encoding: Option<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            encoding: encoding.map(Into::into),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub from: StateId,
    pub to: StateId,
    pub input: String,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StgGraph {
    states: Vec<State>,
    transitions: Vec<Transition>,
    start: Option<StateId>,
    current: Option<StateId>,
    num_inputs: usize,
    num_outputs: usize,
}

impl StgGraph {
    pub fn new(num_inputs: usize, num_outputs: usize) -> Self {
        Self {
            states: Vec::new(),
            transitions: Vec::new(),
            start: None,
            current: None,
            num_inputs,
            num_outputs,
        }
    }

    pub fn states(&self) -> &[State] {
        &self.states
    }

    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    pub fn num_inputs(&self) -> usize {
        self.num_inputs
    }

    pub fn num_outputs(&self) -> usize {
        self.num_outputs
    }

    pub fn start(&self) -> Option<StateId> {
        self.start
    }

    pub fn current(&self) -> Option<StateId> {
        self.current
    }

    pub fn create_state(
        &mut self,
        name: impl Into<String>,
        encoding: Option<impl Into<String>>,
    ) -> StateId {
        let id = StateId(self.states.len());
        self.states.push(State::new(name, encoding));
        id
    }

    pub fn set_start(&mut self, state: StateId) -> Result<(), BwdUtilError> {
        self.require_state(state)?;
        self.start = Some(state);
        Ok(())
    }

    pub fn set_current(&mut self, state: StateId) -> Result<(), BwdUtilError> {
        self.require_state(state)?;
        self.current = Some(state);
        Ok(())
    }

    pub fn state_name(&self, state: StateId) -> Option<&str> {
        self.states.get(state.0).map(|state| state.name.as_str())
    }

    pub fn state_encoding(&self, state: StateId) -> Option<&str> {
        self.states
            .get(state.0)
            .and_then(|state| state.encoding.as_deref())
    }

    pub fn state_by_name(&self, name: &str) -> Option<StateId> {
        self.states
            .iter()
            .position(|state| state.name == name)
            .map(StateId)
    }

    pub fn create_transition(
        &mut self,
        from: StateId,
        to: StateId,
        input: impl Into<String>,
        output: impl Into<String>,
    ) -> Result<TransitionId, BwdUtilError> {
        let input = input.into();
        let output = output.into();
        if duplicate(self, from, to, input.clone(), output.clone())? {
            return Err(BwdUtilError::DuplicateTransition);
        }

        self.create_nfa_transition(from, to, input, output)
    }

    fn create_nfa_transition(
        &mut self,
        from: StateId,
        to: StateId,
        input: impl Into<String>,
        output: impl Into<String>,
    ) -> Result<TransitionId, BwdUtilError> {
        self.require_state(from)?;
        self.require_state(to)?;

        let input = input.into();
        let output = output.into();
        if input.len() != self.num_inputs {
            return Err(BwdUtilError::InvalidInputLength {
                expected: self.num_inputs,
                actual: input.len(),
            });
        }
        if output.len() != self.num_outputs {
            return Err(BwdUtilError::InvalidOutputLength {
                expected: self.num_outputs,
                actual: output.len(),
            });
        }

        let id = TransitionId(self.transitions.len());
        self.transitions.push(Transition {
            from,
            to,
            input,
            output,
        });
        Ok(id)
    }

    fn require_state(&self, state: StateId) -> Result<(), BwdUtilError> {
        if state.0 < self.states.len() {
            Ok(())
        } else {
            Err(BwdUtilError::UnknownState(state))
        }
    }

    fn outgoing_transition_ids(&self, state: StateId) -> Vec<TransitionId> {
        self.transitions
            .iter()
            .enumerate()
            .filter_map(|(index, transition)| {
                (transition.from == state).then_some(TransitionId(index))
            })
            .collect()
    }

    fn incoming_transition_ids(&self, state: StateId) -> Vec<TransitionId> {
        self.transitions
            .iter()
            .enumerate()
            .filter_map(|(index, transition)| {
                (transition.to == state).then_some(TransitionId(index))
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BwdUtilError {
    UnknownState(StateId),
    InvalidInputLength { expected: usize, actual: usize },
    InvalidOutputLength { expected: usize, actual: usize },
    DuplicateTransition,
    MissingStartState,
    MissingIncomingOutput { state: StateId },
    InvalidOutputDontCare { transition: TransitionId },
    MissingAstgGraphLayer,
    MissingNetworkLayer,
}

impl fmt::Display for BwdUtilError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownState(state) => write!(formatter, "unknown STG state {:?}", state),
            Self::InvalidInputLength { expected, actual } => {
                write!(
                    formatter,
                    "invalid input length: expected {expected}, got {actual}"
                )
            }
            Self::InvalidOutputLength { expected, actual } => {
                write!(
                    formatter,
                    "invalid output length: expected {expected}, got {actual}"
                )
            }
            Self::DuplicateTransition => write!(formatter, "duplicate STG transition"),
            Self::MissingStartState => write!(formatter, "STG has no start state"),
            Self::MissingIncomingOutput { state } => {
                write!(
                    formatter,
                    "state {:?} has no non-self incoming output label",
                    state
                )
            }
            Self::InvalidOutputDontCare { transition } => {
                write!(
                    formatter,
                    "transition {:?} has an output don't-care",
                    transition
                )
            }
            Self::MissingAstgGraphLayer => {
                write!(
                    formatter,
                    "ASTG graph and marking traversal is not available yet"
                )
            }
            Self::MissingNetworkLayer => {
                write!(formatter, "network/latch manipulation is not available yet")
            }
        }
    }
}

impl Error for BwdUtilError {}

pub fn bwd_astg_state_bit(signal: &AstgSignal) -> AstgScode {
    signal.state_bit
}

pub fn stg_create_nfa_transition(
    stg: &mut StgGraph,
    from: StateId,
    to: StateId,
    input: impl Into<String>,
    output: impl Into<String>,
) -> Result<TransitionId, BwdUtilError> {
    stg.create_nfa_transition(from, to, input, output)
}

pub fn bwd_log2(arg: AstgScode) -> usize {
    let mut i = 1;
    let mut value = 0;
    while i < arg {
        i <<= 1;
        value += 1;
    }

    value
}

pub fn bwd_get_state_by_name(stg: &StgGraph, name: &str) -> Option<StateId> {
    stg.state_by_name(name)
}

pub fn bwd_new_enabled_signals(
    from_markings: &[AstgMarking],
    to_markings: &[AstgMarking],
    firing_signals: &[AstgSignal],
) -> AstgScode {
    let mut from_enabled = from_markings
        .iter()
        .fold(0, |acc, marking| acc | marking.enabled);
    let mut new_enabled = to_markings
        .iter()
        .fold(0, |acc, marking| acc | marking.enabled);
    let firing = firing_signals
        .iter()
        .fold(0, |acc, signal| acc | bwd_astg_state_bit(signal));

    if firing & new_enabled != 0 {
        from_enabled &= !firing;
    }
    new_enabled &= !from_enabled;

    new_enabled
}

pub fn bwd_marking_string(marked_places: &MarkedPlaces) -> String {
    assert!(marked_places.n_word() > 0);
    marked_places
        .bit_array
        .iter()
        .take(64)
        .map(|word| format!("_{word:x}"))
        .collect::<String>()
}

pub fn duplicate(
    stg: &mut StgGraph,
    from: StateId,
    to: StateId,
    input: impl Into<String>,
    output: impl Into<String>,
) -> Result<bool, BwdUtilError> {
    stg.require_state(from)?;
    stg.require_state(to)?;

    let input = input.into();
    let output = output.into();
    let mut duplicate_id = None;
    let mut result = false;

    for (index, edge) in stg.transitions.iter().enumerate() {
        if edge.from == from && edge.input == input {
            result = true;
            if edge.to != to || edge.output != output {
                duplicate_id = Some(index);
            }
        }
    }

    let Some(index) = duplicate_id else {
        return Ok(result);
    };

    let duplicate_edge = stg.transitions[index].clone();
    if to == from && duplicate_edge.to == duplicate_edge.from {
        let base = input.len().saturating_sub(output.len());
        let mut old_out = duplicate_edge.output.into_bytes();
        for (i, new_value) in output.bytes().enumerate() {
            if old_out[i] != new_value && input.as_bytes()[base + i] != new_value {
                old_out[i] = new_value;
            }
        }
        stg.transitions[index].output = String::from_utf8(old_out).expect("STG labels are ASCII");
    }

    Ok(true)
}

pub fn bwd_marking_hash(marking: &AstgMarking, modulus: usize) -> usize {
    if modulus == 0 {
        return 0;
    }

    marking
        .marked_places
        .bit_array
        .iter()
        .fold(0_u32, |result, word| result ^ word) as usize
        % modulus
}

pub fn bwd_marking_cmp(left: &AstgMarking, right: &AstgMarking) -> i32 {
    if left.marked_places.n_elem != right.marked_places.n_elem {
        return left.marked_places.n_elem as i32 - right.marked_places.n_elem as i32;
    }

    for (left_word, right_word) in left
        .marked_places
        .bit_array
        .iter()
        .zip(right.marked_places.bit_array.iter())
    {
        if left_word != right_word {
            return if left_word < right_word { -1 } else { 1 };
        }
    }

    left.marked_places.n_word() as i32 - right.marked_places.n_word() as i32
}

pub fn bwd_marking_ptr_cmp(left: &&AstgMarking, right: &&AstgMarking) -> i32 {
    bwd_marking_cmp(left, right)
}

pub fn bwd_po_name(name: &str) -> String {
    if let Some(stripped) = name.strip_suffix("_next") {
        stripped.to_owned()
    } else if let Some(stripped) = name.strip_suffix('_') {
        stripped.to_owned()
    } else {
        name.to_owned()
    }
}

pub fn bwd_fake_po_name(name: &str) -> String {
    format!("{}_next", bwd_po_name(name))
}

pub fn bwd_fake_pi_name(name: &str) -> String {
    format!("{}_", bwd_po_name(name))
}

pub fn graph_scc(stg: &StgGraph) -> Vec<Vec<StateId>> {
    let mut search = SccSearch::new(stg);
    for index in 0..stg.states.len() {
        let state = StateId(index);
        if !search.depth_first.contains_key(&state) {
            search.search(state);
        }
    }

    search.components
}

pub fn covers(in1: &str, in2: &str) -> bool {
    in1.chars()
        .zip(in2.chars())
        .all(|(left, right)| left == '-' || left == right)
}

pub fn compatible(in1: &str, in2: &str) -> bool {
    in1.chars()
        .zip(in2.chars())
        .all(|(left, right)| left == '-' || right == '-' || left == right)
}

pub fn is_burst_subset(loop_input: &str, in1: &str, in2: &str) -> bool {
    loop_input
        .chars()
        .zip(in1.chars())
        .zip(in2.chars())
        .all(|((loop_value, left), right)| {
            loop_value == '-'
                || left == '-'
                || right == '-'
                || loop_value == left
                || loop_value != right
        })
}

pub fn string_to_set(value: &str) -> Vec<Option<bool>> {
    value
        .chars()
        .map(|ch| match ch {
            '0' => Some(false),
            '1' => Some(true),
            _ => None,
        })
        .collect()
}

pub fn set_to_string(set: &[Option<bool>]) -> String {
    set.iter()
        .map(|value| match value {
            Some(false) => '0',
            Some(true) => '1',
            None => '-',
        })
        .collect()
}

pub fn bwd_dc_to_self_loop(stg: &mut StgGraph) -> Result<usize, BwdUtilError> {
    let original_state_count = stg.states.len();
    let mut added = 0;

    for state_index in 0..original_state_count {
        let state = StateId(state_index);
        let covered = stg
            .outgoing_transition_ids(state)
            .into_iter()
            .map(|id| stg.transitions[id.0].input.clone())
            .collect::<Vec<_>>();
        let output = incoming_non_self_output(stg, state)?.to_owned();
        let uncovered = uncovered_binary_inputs(stg.num_inputs, &covered);

        for input in uncovered {
            stg.create_nfa_transition(state, state, input, output.clone())?;
            added += 1;
        }
    }

    Ok(added)
}

pub fn bwd_stg_scr(old_stg: &StgGraph) -> Result<StgGraph, BwdUtilError> {
    for (index, transition) in old_stg.transitions.iter().enumerate() {
        if transition.output.contains('-') {
            return Err(BwdUtilError::InvalidOutputDontCare {
                transition: TransitionId(index),
            });
        }
    }

    let old_start = old_stg.start.ok_or(BwdUtilError::MissingStartState)?;
    let mut new_stg = StgGraph::new(old_stg.num_inputs, old_stg.num_outputs);
    let mut states = Vec::<ScrState>::new();
    stg_scr_recur(&mut states, old_start, old_stg, &mut new_stg)?;

    let old_start_name = old_stg.state_name(old_start).unwrap_or_default();
    if let Some(start) = new_stg
        .states
        .iter()
        .position(|state| state.name.starts_with(old_start_name))
        .map(StateId)
    {
        new_stg.set_start(start)?;
        new_stg.set_current(start)?;
    }

    Ok(new_stg)
}

pub fn bwd_write_sg(stg: &StgGraph, input_names: Option<&[String]>) -> String {
    let mut output = String::from("SG:\nSTATEVECTOR:");
    if let Some(names) = input_names {
        for name in names.iter().take(stg.num_inputs) {
            let po_name = bwd_po_name(name);
            if name.ends_with('_') {
                output.push_str(&format!("{po_name} "));
            } else {
                output.push_str(&format!("INP {po_name} "));
            }
        }
    } else {
        output.push_str("input names cannot be found.\n");
    }
    output.push_str("\nSTATES:\n");

    for state_index in 0..stg.states.len() {
        let state = StateId(state_index);
        let mut state_encoding = String::new();
        let mut first = true;

        for edge_id in stg.outgoing_transition_ids(state) {
            let edge = &stg.transitions[edge_id.0];
            if first {
                state_encoding = edge.input.clone();
                first = false;
            } else {
                for (index, next_value) in edge.input.chars().enumerate().take(stg.num_inputs) {
                    let current = state_encoding.as_bytes()[index] as char;
                    if next_value != current {
                        state_encoding.replace_range(
                            index..index + 1,
                            if current == '1' { "F" } else { "R" },
                        );
                    }
                }
            }
        }

        output.push_str(&state_encoding);
        output.push('\n');
    }

    output
}

pub fn bwd_astg_to_stg() -> Result<StgGraph, BwdUtilError> {
    Err(BwdUtilError::MissingAstgGraphLayer)
}

pub fn bwd_stg_to_astg() -> Result<(), BwdUtilError> {
    Err(BwdUtilError::MissingAstgGraphLayer)
}

pub fn bwd_astg_latch() -> Result<(), BwdUtilError> {
    Err(BwdUtilError::MissingNetworkLayer)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScrState {
    name: String,
    input: String,
    output: String,
    state: StateId,
}

struct SccSearch<'a> {
    stg: &'a StgGraph,
    depth_first: BTreeMap<StateId, usize>,
    stack_set: BTreeSet<StateId>,
    stack: Vec<StateId>,
    components: Vec<Vec<StateId>>,
    count: usize,
}

impl<'a> SccSearch<'a> {
    fn new(stg: &'a StgGraph) -> Self {
        Self {
            stg,
            depth_first: BTreeMap::new(),
            stack_set: BTreeSet::new(),
            stack: Vec::new(),
            components: Vec::new(),
            count: 0,
        }
    }

    fn search(&mut self, state: StateId) -> usize {
        let df_v = self.count;
        self.count += 1;
        self.depth_first.insert(state, df_v);
        self.stack_set.insert(state);
        self.stack.push(state);
        let mut low_v = df_v;

        for edge_id in self.stg.outgoing_transition_ids(state) {
            let next = self.stg.transitions[edge_id.0].to;
            if let Some(df_w) = self.depth_first.get(&next).copied() {
                if df_w < df_v && self.stack_set.contains(&next) {
                    low_v = low_v.min(df_w);
                }
            } else {
                let low_w = self.search(next);
                low_v = low_v.min(low_w);
            }
        }

        if low_v == df_v {
            let mut component = Vec::new();
            loop {
                let next = self.stack.pop().expect("SCC stack underflow");
                self.stack_set.remove(&next);
                component.push(next);
                if next == state {
                    break;
                }
            }
            self.components.push(component);
        }

        low_v
    }
}

fn incoming_non_self_output(stg: &StgGraph, state: StateId) -> Result<&str, BwdUtilError> {
    stg.incoming_transition_ids(state)
        .into_iter()
        .find_map(|edge_id| {
            let edge = &stg.transitions[edge_id.0];
            (edge.from != state).then_some(edge.output.as_str())
        })
        .ok_or(BwdUtilError::MissingIncomingOutput { state })
}

fn uncovered_binary_inputs(width: usize, covered: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for value in 0..(1_usize << width) {
        let input = (0..width)
            .rev()
            .map(|bit| if value & (1 << bit) == 0 { '0' } else { '1' })
            .collect::<String>();
        if !covered.iter().any(|cube| covers(cube, &input)) {
            result.push(input);
        }
    }

    result
}

fn search_scr_state(
    new_stg: &mut StgGraph,
    states: &mut Vec<ScrState>,
    name: &str,
    input: &str,
    output: &str,
    code: Option<&str>,
) -> Result<(StateId, bool), BwdUtilError> {
    for state in states.iter() {
        if state.name == name && compatible(&state.input, input) && state.output == output {
            return Ok((state.state, false));
        }
    }

    let new_name = format!("{name}_{input}");
    let state = new_stg.create_state(new_name, code.map(str::to_owned));
    states.push(ScrState {
        name: name.to_owned(),
        input: input.to_owned(),
        output: output.to_owned(),
        state,
    });

    Ok((state, true))
}

fn stg_scr_recur(
    states: &mut Vec<ScrState>,
    old_from: StateId,
    old_stg: &StgGraph,
    new_stg: &mut StgGraph,
) -> Result<(), BwdUtilError> {
    let incoming = old_stg
        .incoming_transition_ids(old_from)
        .into_iter()
        .filter(|edge_id| old_stg.transitions[edge_id.0].from != old_from)
        .collect::<Vec<_>>();

    for inedge_id in incoming {
        let inedge = &old_stg.transitions[inedge_id.0];
        let name = old_stg.state_name(old_from).unwrap_or_default();
        let code = old_stg.state_encoding(old_from);
        let (new_from, _) =
            search_scr_state(new_stg, states, name, &inedge.input, &inedge.output, code)?;

        for outedge_id in old_stg.outgoing_transition_ids(old_from) {
            let outedge = &old_stg.transitions[outedge_id.0];
            let old_to = outedge.to;

            if old_to == old_from {
                stg_create_nfa_transition(
                    new_stg,
                    new_from,
                    new_from,
                    outedge.input.clone(),
                    outedge.output.clone(),
                )?;
            } else {
                let to_name = old_stg.state_name(old_to).unwrap_or_default();
                let to_code = old_stg.state_encoding(old_to);
                let (new_to, is_new) = search_scr_state(
                    new_stg,
                    states,
                    to_name,
                    &outedge.input,
                    &outedge.output,
                    to_code,
                )?;
                if is_new {
                    stg_scr_recur(states, old_to, old_stg, new_stg)?;
                }

                if !duplicate(
                    new_stg,
                    new_from,
                    new_to,
                    outedge.input.clone(),
                    outedge.output.clone(),
                )? {
                    stg_create_nfa_transition(
                        new_stg,
                        new_from,
                        new_to,
                        outedge.input.clone(),
                        outedge.output.clone(),
                    )?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_state_stg() -> StgGraph {
        let mut stg = StgGraph::new(2, 1);
        let s0 = stg.create_state("s0", Some("0"));
        let s1 = stg.create_state("s1", Some("1"));
        stg.set_start(s0).unwrap();
        stg.set_current(s0).unwrap();
        stg_create_nfa_transition(&mut stg, s0, s1, "00", "0").unwrap();
        stg_create_nfa_transition(&mut stg, s1, s0, "11", "1").unwrap();
        stg
    }

    #[test]
    fn log2_matches_legacy_ceiling_loop() {
        assert_eq!(bwd_log2(0), 0);
        assert_eq!(bwd_log2(1), 0);
        assert_eq!(bwd_log2(2), 1);
        assert_eq!(bwd_log2(3), 2);
        assert_eq!(bwd_log2(4), 2);
        assert_eq!(bwd_log2(5), 3);
    }

    #[test]
    fn marking_string_hash_and_compare_match_c_shape() {
        let left = AstgMarking::new(MarkedPlaces::new(64, [0x12_u32, 0xab_u32]), 0);
        let right = AstgMarking::new(MarkedPlaces::new(64, [0x12_u32, 0xac_u32]), 0);

        assert_eq!(bwd_marking_string(&left.marked_places), "_12_ab");
        assert_eq!(
            bwd_marking_hash(&left, 17),
            ((0x12_u32 ^ 0xab_u32) as usize) % 17
        );
        assert!(bwd_marking_cmp(&left, &right) < 0);
    }

    #[test]
    fn new_enabled_signals_keeps_refired_signal_enabled() {
        let a = AstgSignal::new("a", SignalKind::Input, 0b0001);
        let b = AstgSignal::new("b", SignalKind::Output, 0b0010);
        let from = [AstgMarking::new(MarkedPlaces::new(1, [1]), 0b0011)];
        let to = [AstgMarking::new(MarkedPlaces::new(1, [2]), 0b0111)];

        assert_eq!(bwd_new_enabled_signals(&from, &to, &[a, b]), 0b0111);
    }

    #[test]
    fn duplicate_updates_self_loop_output_maximally() {
        let mut stg = StgGraph::new(3, 1);
        let s0 = stg.create_state("s0", None::<String>);
        stg_create_nfa_transition(&mut stg, s0, s0, "100", "0").unwrap();

        assert!(duplicate(&mut stg, s0, s0, "100", "1").unwrap());
        assert_eq!(stg.transitions()[0].output, "1");
    }

    #[test]
    fn names_match_po_fake_po_and_fake_pi_rules() {
        assert_eq!(bwd_po_name("ready_"), "ready");
        assert_eq!(bwd_po_name("ready_next"), "ready");
        assert_eq!(bwd_po_name("ready"), "ready");
        assert_eq!(bwd_fake_po_name("ready_"), "ready_next");
        assert_eq!(bwd_fake_pi_name("ready_next"), "ready_");
    }

    #[test]
    fn cube_helpers_match_legacy_semantics() {
        assert!(covers("1-", "10"));
        assert!(!covers("10", "1-"));
        assert!(compatible("10-", "1-1"));
        assert!(!compatible("100", "1-1"));
        assert!(is_burst_subset("000", "110", "111"));
        assert!(!is_burst_subset("000", "110", "100"));
        assert_eq!(set_to_string(&string_to_set("10-")), "10-");
    }

    #[test]
    fn graph_scc_finds_cycles_and_singletons() {
        let mut stg = two_state_stg();
        let s2 = stg.create_state("s2", Some("0"));
        stg_create_nfa_transition(&mut stg, s2, s2, "01", "0").unwrap();

        let normalized = graph_scc(&stg)
            .into_iter()
            .map(|mut component| {
                component.sort();
                component
                    .into_iter()
                    .map(|state| state.0)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        assert!(normalized.contains(&vec![0, 1]));
        assert!(normalized.contains(&vec![2]));
    }

    #[test]
    fn dc_to_self_loop_adds_uncovered_binary_inputs() {
        let mut stg = two_state_stg();

        let added = bwd_dc_to_self_loop(&mut stg).unwrap();

        assert_eq!(added, 6);
        assert!(stg
            .transitions()
            .iter()
            .any(|transition| transition.from == StateId(0)
                && transition.to == StateId(0)
                && transition.input == "01"
                && transition.output == "1"));
    }

    #[test]
    fn stg_scr_splits_states_by_incoming_cube() {
        let mut stg = StgGraph::new(2, 1);
        let s0 = stg.create_state("s0", Some("0"));
        let s1 = stg.create_state("s1", Some("1"));
        stg.set_start(s0).unwrap();
        stg_create_nfa_transition(&mut stg, s0, s1, "00", "0").unwrap();
        stg_create_nfa_transition(&mut stg, s1, s0, "01", "1").unwrap();
        stg_create_nfa_transition(&mut stg, s0, s0, "10", "0").unwrap();

        let split = bwd_stg_scr(&stg).unwrap();
        let names = split
            .states()
            .iter()
            .map(|state| state.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"s0_01"));
        assert!(names.contains(&"s1_00"));
        assert!(split
            .transitions()
            .iter()
            .any(|transition| transition.from == transition.to));
    }

    #[test]
    fn write_sg_marks_rising_and_falling_inputs() {
        let stg = two_state_stg();
        let names = vec!["a".to_owned(), "b_".to_owned()];

        let output = bwd_write_sg(&stg, Some(&names));

        assert!(output.starts_with("SG:\nSTATEVECTOR:INP a b "));
        assert!(output.contains("STATES:\n00\n11\n"));
    }

    #[test]
    fn graph_and_network_boundaries_are_explicit() {
        assert_eq!(bwd_astg_to_stg(), Err(BwdUtilError::MissingAstgGraphLayer));
        assert_eq!(bwd_stg_to_astg(), Err(BwdUtilError::MissingAstgGraphLayer));
        assert_eq!(bwd_astg_latch(), Err(BwdUtilError::MissingNetworkLayer));
    }

    #[test]
    fn source_contains_no_legacy_c_abi_or_dependency_metadata() {
        let source = include_str!("bwd_util.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
