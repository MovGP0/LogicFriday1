//! Native Rust scaffold for `LogicSynthesis/sis/seqbdd/seqbdd_cycle.c`.
//!
//! The C routine computes a cycle through the encoded initial-state set, then
//! walks reverse images to pick one predecessor edge per BFS layer. This file
//! keeps that behavior in a native, testable transition-system model. Binding
//! it directly to SIS networks and BDDs is deliberately blocked until the
//! seqbdd, ntbdd, network, and BDD manager ports exist.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InputId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge<S, I> {
    pub from: S,
    pub input: I,
    pub to: S,
}

impl<S, I> Edge<S, I> {
    pub fn new(from: S, input: I, to: S) -> Self {
        Self { from, input, to }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputVector {
    values: Vec<u8>,
}

impl InputVector {
    pub fn new(values: impl Into<Vec<u8>>) -> Result<Self, SeqBddCycleError> {
        let values = values.into();
        if let Some(value) = values.iter().find(|value| **value > 1) {
            return Err(SeqBddCycleError::InvalidBinaryValue(*value));
        }
        Ok(Self { values })
    }

    pub fn values(&self) -> &[u8] {
        &self.values
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResetAncestor<S> {
    values: BTreeMap<S, u8>,
}

impl<S> Default for ResetAncestor<S> {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }
}

impl<S> ResetAncestor<S>
where
    S: Ord,
{
    pub fn insert(&mut self, latch_state: S, value: u8) -> Result<(), SeqBddCycleError> {
        if value > 1 {
            return Err(SeqBddCycleError::InvalidBinaryValue(value));
        }
        self.values.insert(latch_state, value);
        Ok(())
    }

    pub fn values(&self) -> &BTreeMap<S, u8> {
        &self.values
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedInputSequence<S, I> {
    pub ancestor_state: S,
    pub reset_ancestor: ResetAncestor<S>,
    pub input_sequence: Vec<I>,
    pub cycle_states: Vec<S>,
    pub cycle_inputs: Vec<I>,
}

pub trait SeqBddTransitionSystem {
    type State: Clone + Eq + Hash + Ord;
    type Input: Clone + Eq;

    fn initial_state(&self) -> Option<Self::State>;
    fn successors(&self, state: &Self::State) -> Vec<Edge<Self::State, Self::Input>>;
    fn latch_values(&self, state: &Self::State) -> Vec<(Self::State, u8)>;
}

#[derive(Clone, Debug)]
pub struct ExplicitTransitionSystem<S, I> {
    initial_state: Option<S>,
    edges: Vec<Edge<S, I>>,
    latch_values: HashMap<S, Vec<(S, u8)>>,
}

impl<S, I> ExplicitTransitionSystem<S, I>
where
    S: Clone + Eq + Hash,
{
    pub fn new(initial_state: Option<S>) -> Self {
        Self {
            initial_state,
            edges: Vec::new(),
            latch_values: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, from: S, input: I, to: S) {
        self.edges.push(Edge::new(from, input, to));
    }

    pub fn set_latch_values(
        &mut self,
        state: S,
        values: Vec<(S, u8)>,
    ) -> Result<(), SeqBddCycleError> {
        if let Some((_, value)) = values.iter().find(|(_, value)| *value > 1) {
            return Err(SeqBddCycleError::InvalidBinaryValue(*value));
        }
        self.latch_values.insert(state, values);
        Ok(())
    }
}

impl<S, I> SeqBddTransitionSystem for ExplicitTransitionSystem<S, I>
where
    S: Clone + Eq + Hash + Ord,
    I: Clone + Eq,
{
    type State = S;
    type Input = I;

    fn initial_state(&self) -> Option<Self::State> {
        self.initial_state.clone()
    }

    fn successors(&self, state: &Self::State) -> Vec<Edge<Self::State, Self::Input>> {
        self.edges
            .iter()
            .filter(|edge| &edge.from == state)
            .cloned()
            .collect()
    }

    fn latch_values(&self, state: &Self::State) -> Vec<(Self::State, u8)> {
        self.latch_values.get(state).cloned().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SeqBddCycleError {
    MissingInitialState,
    InitialStateNotReachableFromItself,
    NoReverseEdge { target_depth: usize },
    InvalidBinaryValue(u8),
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for SeqBddCycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInitialState => write!(f, "network has no seqbdd initial-state node"),
            Self::InitialStateNotReachableFromItself => {
                write!(f, "initial state is not reachable from itself")
            }
            Self::NoReverseEdge { target_depth } => write!(
                f,
                "could not extract a reverse edge into the state at BFS depth {target_depth}"
            ),
            Self::InvalidBinaryValue(value) => {
                write!(
                    f,
                    "seqbdd input and latch values must be 0 or 1, got {value}"
                )
            }
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by missing native SIS ports")
            }
        }
    }
}

impl Error for SeqBddCycleError {}

pub fn extract_input_sequence<T>(
    system: &T,
    n_shift: usize,
) -> Result<ExtractedInputSequence<T::State, T::Input>, SeqBddCycleError>
where
    T: SeqBddTransitionSystem,
{
    let initial_state = system
        .initial_state()
        .ok_or(SeqBddCycleError::MissingInitialState)?;

    let mut total_sets = Vec::new();
    let mut current_set = BTreeSet::from([initial_state.clone()]);
    let mut total_set = current_set.clone();

    loop {
        total_sets.push(total_set.clone());
        let new_current_set = compute_next_states(system, &current_set);
        if new_current_set.contains(&initial_state) {
            break;
        }
        if new_current_set.is_subset(&total_set) {
            return Err(SeqBddCycleError::InitialStateNotReachableFromItself);
        }

        current_set = new_current_set.difference(&total_set).cloned().collect();
        total_set.extend(new_current_set);
    }

    let mut current_state = initial_state.clone();
    let mut cycle_states = Vec::with_capacity(total_sets.len());
    let mut cycle_inputs = Vec::with_capacity(total_sets.len());

    for depth in (0..total_sets.len()).rev() {
        let edge = reverse_edge_into(system, &current_state, &total_sets[depth]).ok_or(
            SeqBddCycleError::NoReverseEdge {
                target_depth: depth,
            },
        )?;
        current_state = edge.from;
        cycle_states.push(current_state.clone());
        cycle_inputs.push(edge.input);
    }

    cycle_states.reverse();
    cycle_inputs.reverse();

    debug_assert!(cycle_states.first() == Some(&initial_state));

    let cycle_length = cycle_states.len();
    let ancestor_index = ancestor_index(n_shift, cycle_length);
    let ancestor_state = cycle_states[ancestor_index].clone();
    let mut reset_ancestor = ResetAncestor::default();
    for (latch_state, value) in system.latch_values(&ancestor_state) {
        reset_ancestor.insert(latch_state, value)?;
    }

    let mut input_sequence = Vec::with_capacity(n_shift);
    let mut edge_index = ancestor_index;
    for _ in 0..n_shift {
        input_sequence.push(cycle_inputs[edge_index % cycle_length].clone());
        edge_index += 1;
    }
    input_sequence.reverse();

    Ok(ExtractedInputSequence {
        ancestor_state,
        reset_ancestor,
        input_sequence,
        cycle_states,
        cycle_inputs,
    })
}

pub fn ancestor_index(n_shift: usize, cycle_length: usize) -> usize {
    if n_shift == 0 || cycle_length <= 1 {
        0
    } else {
        (cycle_length - (n_shift % cycle_length)) % cycle_length
    }
}

pub fn input_minterm_to_ordered_values(
    input_order: &[(InputId, usize)],
    minterm_values: &HashMap<InputId, u8>,
) -> Result<InputVector, SeqBddCycleError> {
    let mut values = vec![0; input_order.len()];
    for (input, index) in input_order {
        let value = minterm_values.get(input).copied().unwrap_or(0);
        if value > 1 {
            return Err(SeqBddCycleError::InvalidBinaryValue(value));
        }
        values[*index] = value;
    }
    InputVector::new(values)
}

pub fn one_minterm_from_cube(
    cube_literals: &[u8],
    selected_varids: &HashSet<usize>,
) -> Result<BTreeMap<usize, u8>, SeqBddCycleError> {
    let mut minterm = BTreeMap::new();
    for (varid, literal) in cube_literals.iter().copied().enumerate() {
        if !selected_varids.contains(&varid) {
            continue;
        }
        let value = match literal {
            0 | 2 => 0,
            1 => 1,
            value => return Err(SeqBddCycleError::InvalidBinaryValue(value)),
        };
        minterm.insert(varid, value);
    }
    Ok(minterm)
}

pub fn extract_input_sequence_from_sis_network<Network>(
    _network: &Network,
    _n_shift: usize,
) -> Result<(), SeqBddCycleError> {
    Err(SeqBddCycleError::MissingNativePorts {
        operation: "extract_input_sequence_from_sis_network",
    })
}

fn compute_next_states<T>(system: &T, current_set: &BTreeSet<T::State>) -> BTreeSet<T::State>
where
    T: SeqBddTransitionSystem,
{
    current_set
        .iter()
        .flat_map(|state| system.successors(state))
        .map(|edge| edge.to)
        .collect()
}

fn reverse_edge_into<T>(
    system: &T,
    target_state: &T::State,
    allowed_sources: &BTreeSet<T::State>,
) -> Option<Edge<T::State, T::Input>>
where
    T: SeqBddTransitionSystem,
{
    allowed_sources.iter().find_map(|source| {
        system
            .successors(source)
            .into_iter()
            .find(|edge| &edge.to == target_state)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(ids: &[usize]) -> Vec<StateId> {
        ids.iter().copied().map(StateId).collect()
    }

    #[test]
    fn ancestor_index_matches_c_negative_modulo_adjustment() {
        assert_eq!(ancestor_index(0, 3), 0);
        assert_eq!(ancestor_index(1, 3), 2);
        assert_eq!(ancestor_index(2, 3), 1);
        assert_eq!(ancestor_index(3, 3), 0);
        assert_eq!(ancestor_index(4, 3), 2);
        assert_eq!(ancestor_index(5, 1), 0);
    }

    #[test]
    fn extracts_cycle_inputs_in_simulation_time_order() {
        let mut system = ExplicitTransitionSystem::new(Some(StateId(0)));
        system.add_edge(StateId(0), InputId(10), StateId(1));
        system.add_edge(StateId(1), InputId(11), StateId(2));
        system.add_edge(StateId(2), InputId(12), StateId(0));
        system
            .set_latch_values(StateId(1), vec![(StateId(100), 1), (StateId(101), 0)])
            .unwrap();

        let extracted = extract_input_sequence(&system, 5).unwrap();

        assert_eq!(extracted.cycle_states, ids(&[0, 1, 2]));
        assert_eq!(
            extracted.cycle_inputs,
            vec![InputId(10), InputId(11), InputId(12)]
        );
        assert_eq!(extracted.ancestor_state, StateId(1));
        assert_eq!(
            extracted.reset_ancestor.values(),
            &BTreeMap::from([(StateId(100), 1), (StateId(101), 0)])
        );
        assert_eq!(
            extracted.input_sequence,
            vec![
                InputId(12),
                InputId(11),
                InputId(10),
                InputId(12),
                InputId(11)
            ]
        );
    }

    #[test]
    fn reports_c_return_one_case_as_error() {
        let mut system = ExplicitTransitionSystem::new(Some(StateId(0)));
        system.add_edge(StateId(0), InputId(1), StateId(1));
        system.add_edge(StateId(1), InputId(2), StateId(1));

        assert_eq!(
            extract_input_sequence(&system, 2),
            Err(SeqBddCycleError::InitialStateNotReachableFromItself)
        );
    }

    #[test]
    fn converts_input_minterm_by_explicit_order_and_defaults_missing_to_zero() {
        let values = input_minterm_to_ordered_values(
            &[(InputId(7), 1), (InputId(4), 0), (InputId(9), 2)],
            &HashMap::from([(InputId(7), 1), (InputId(4), 0)]),
        )
        .unwrap();

        assert_eq!(values.values(), &[0, 1, 0]);
    }

    #[test]
    fn one_minterm_treats_dc_as_negative_literal_like_c() {
        let selected = HashSet::from([0, 2, 3]);

        assert_eq!(
            one_minterm_from_cube(&[1, 0, 2, 0], &selected).unwrap(),
            BTreeMap::from([(0, 1), (2, 0), (3, 0)])
        );
    }
    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("seqbdd_cycle.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
