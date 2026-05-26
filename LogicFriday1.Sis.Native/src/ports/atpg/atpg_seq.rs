use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LogicValue
{
    Zero,
    One,
    DontCare,
}

impl LogicValue
{
    pub fn concretize(self) -> u8
    {
        match self
        {
            Self::Zero | Self::DontCare => 0,
            Self::One => 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtpgSeqError
{
    EmptyResetStates,
    InputWidthMismatch
    {
        expected: usize,
        actual: usize,
    },
    LatchOrderingMismatch
    {
        expected: usize,
        actual: usize,
    },
    MissingJustificationSequence,
    MissingPropagationSequence,
    MissingOutput,
    NoExcitationStateReachable,
    NoDistinguishingSequence,
}

impl fmt::Display for AtpgSeqError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyResetStates => formatter.write_str("sequential ATPG reset state set is empty"),
            Self::InputWidthMismatch
            {
                expected,
                actual,
            } => write!(
                formatter,
                "input vector width mismatch: expected {expected}, got {actual}"
            ),
            Self::LatchOrderingMismatch
            {
                expected,
                actual,
            } => write!(
                formatter,
                "latch ordering width mismatch: expected {expected}, got {actual}"
            ),
            Self::MissingJustificationSequence => {
                formatter.write_str("no cached justification sequence exists for the state")
            }
            Self::MissingPropagationSequence => {
                formatter.write_str("no cached propagation sequence exists for the key")
            }
            Self::MissingOutput => formatter.write_str("product machine has no checked outputs"),
            Self::NoExcitationStateReachable => {
                formatter.write_str("no excitation state is reachable from the reset states")
            }
            Self::NoDistinguishingSequence => {
                formatter.write_str("no product-machine distinguishing sequence was found")
            }
        }
    }
}

impl Error for AtpgSeqError {}

pub type InputVector = Vec<u8>;
pub type InputSequence = Vec<InputVector>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reachability<S>
{
    pub reached_sets: Vec<BTreeSet<S>>,
    pub total_set: BTreeSet<S>,
    pub converged: bool,
}

pub trait SequentialTransitionSystem
{
    type State: Clone + Eq + Hash + Ord;
    type Input: Clone + Eq + Hash + Ord;

    fn reset_states(&self) -> BTreeSet<Self::State>;

    fn successors(&self, state: &Self::State) -> Vec<(Self::Input, Self::State)>;

    fn predecessors(&self, state: &Self::State) -> Vec<(Self::Input, Self::State)>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplicitSequentialMachine<S, I>
{
    reset_states: BTreeSet<S>,
    transitions: BTreeMap<S, Vec<(I, S)>>,
    reverse_transitions: BTreeMap<S, Vec<(I, S)>>,
}

impl<S, I> ExplicitSequentialMachine<S, I>
where
    S: Clone + Eq + Hash + Ord,
    I: Clone + Eq + Hash + Ord,
{
    pub fn new(
        reset_states: impl IntoIterator<Item = S>,
        transitions: impl IntoIterator<Item = (S, I, S)>,
    ) -> Self
    {
        let mut forward = BTreeMap::<S, Vec<(I, S)>>::new();
        let mut reverse = BTreeMap::<S, Vec<(I, S)>>::new();

        for (from, input, to) in transitions
        {
            forward
                .entry(from.clone())
                .or_default()
                .push((input.clone(), to.clone()));
            reverse.entry(to).or_default().push((input, from));
        }

        Self
        {
            reset_states: reset_states.into_iter().collect(),
            transitions: forward,
            reverse_transitions: reverse,
        }
    }
}

impl<S, I> SequentialTransitionSystem for ExplicitSequentialMachine<S, I>
where
    S: Clone + Eq + Hash + Ord,
    I: Clone + Eq + Hash + Ord,
{
    type State = S;
    type Input = I;

    fn reset_states(&self) -> BTreeSet<Self::State>
    {
        self.reset_states.clone()
    }

    fn successors(&self, state: &Self::State) -> Vec<(Self::Input, Self::State)>
    {
        self.transitions.get(state).cloned().unwrap_or_default()
    }

    fn predecessors(&self, state: &Self::State) -> Vec<(Self::Input, Self::State)>
    {
        self.reverse_transitions
            .get(state)
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceCache<K>
where
    K: Ord,
{
    sequences: BTreeMap<K, Option<InputSequence>>,
}

impl<K> SequenceCache<K>
where
    K: Ord,
{
    pub fn new() -> Self
    {
        Self
        {
            sequences: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, sequence: Option<InputSequence>) -> Option<InputSequence>
    {
        self.sequences.insert(key, sequence).flatten()
    }

    pub fn get(&self, key: &K) -> Option<&Option<InputSequence>>
    {
        self.sequences.get(key)
    }

    pub fn contains_key(&self, key: &K) -> bool
    {
        self.sequences.contains_key(key)
    }
}

impl<K> Default for SequenceCache<K>
where
    K: Ord,
{
    fn default() -> Self
    {
        Self::new()
    }
}

pub fn calculate_reachable_states<T>(
    machine: &T,
    iteration_limit: usize,
) -> Result<Reachability<T::State>, AtpgSeqError>
where
    T: SequentialTransitionSystem,
{
    let reset_states = machine.reset_states();
    if reset_states.is_empty()
    {
        return Err(AtpgSeqError::EmptyResetStates);
    }

    let mut total_set = BTreeSet::new();
    let mut reached_sets = Vec::new();
    let mut frontier = reset_states;

    for _ in 0..iteration_limit
    {
        if frontier.is_subset(&total_set)
        {
            return Ok(Reachability
            {
                reached_sets,
                total_set,
                converged: true,
            });
        }

        total_set.extend(frontier.iter().cloned());
        reached_sets.push(frontier.clone());

        let mut next_frontier = BTreeSet::new();
        for state in &frontier
        {
            for (_, next_state) in machine.successors(state)
            {
                next_frontier.insert(next_state);
            }
        }
        frontier = next_frontier;
    }

    Ok(Reachability
    {
        reached_sets,
        total_set,
        converged: false,
    })
}

pub fn state_justify<T>(
    machine: &T,
    reached_sets: &[BTreeSet<T::State>],
    excite_states: &BTreeSet<T::State>,
) -> Result<(usize, Vec<T::Input>, T::State), AtpgSeqError>
where
    T: SequentialTransitionSystem,
{
    let mut depth = None;
    let mut target_state = None;

    for (index, reached_set) in reached_sets.iter().enumerate().skip(1)
    {
        if let Some(state) = reached_set.intersection(excite_states).next()
        {
            depth = Some(index);
            target_state = Some(state.clone());
            break;
        }
    }

    let depth = depth.ok_or(AtpgSeqError::NoExcitationStateReachable)?;
    let target_state = target_state.expect("target state is set with depth");
    let reset_states = machine.reset_states();
    let trace = reverse_generate_sequence(machine, reached_sets, depth, target_state.clone())?;

    debug_assert!(
        trace
            .last()
            .is_none_or(|(_, state)| reset_states.contains(state))
    );

    Ok((
        depth,
        trace
            .into_iter()
            .rev()
            .map(|(input, _)| input)
            .collect::<Vec<_>>(),
        target_state,
    ))
}

pub fn reuse_justification_sequence<K>(
    cache: &SequenceCache<K>,
    key: &K,
    destination: &mut InputSequence,
) -> Result<usize, AtpgSeqError>
where
    K: Ord,
{
    let sequence = cache
        .get(key)
        .and_then(|entry| entry.as_ref())
        .ok_or(AtpgSeqError::MissingJustificationSequence)?;

    copy_sequence(sequence, destination)?;
    Ok(sequence.len())
}

pub fn reuse_propagation_sequence<K>(
    cache: &SequenceCache<K>,
    key: &K,
    destination: &mut InputSequence,
) -> Result<usize, AtpgSeqError>
where
    K: Ord,
{
    let Some(entry) = cache.get(key) else
    {
        return Err(AtpgSeqError::MissingPropagationSequence);
    };

    let Some(sequence) = entry else
    {
        return Ok(0);
    };

    copy_sequence(sequence, destination)?;
    Ok(sequence.len())
}

pub fn convert_trace_to_sequence<K>(
    input_trace: &[Vec<LogicValue>],
    sequence: &mut InputSequence,
    cache: Option<&mut SequenceCache<K>>,
    key: K,
) -> Result<(), AtpgSeqError>
where
    K: Ord,
{
    let width = input_trace.first().map_or(0, Vec::len);
    for vector in input_trace
    {
        if vector.len() != width
        {
            return Err(AtpgSeqError::InputWidthMismatch
            {
                expected: width,
                actual: vector.len(),
            });
        }
    }

    ensure_sequence_shape(sequence, input_trace.len(), width);

    for (index, vector) in input_trace.iter().enumerate()
    {
        for (slot, value) in sequence[index].iter_mut().zip(vector)
        {
            *slot = value.concretize();
        }
    }

    if let Some(cache) = cache
    {
        if !cache.contains_key(&key)
        {
            cache.insert(key, Some(sequence[..input_trace.len()].to_vec()));
        }
    }

    Ok(())
}

pub fn construct_product_start_states(
    start_state_cubes: &[Vec<LogicValue>],
    latch_to_pi_ordering: &[usize],
) -> Result<BTreeSet<(Vec<u8>, Vec<u8>)>, AtpgSeqError>
{
    let mut product_start_states = BTreeSet::new();
    for cube in start_state_cubes
    {
        if latch_to_pi_ordering.len() > cube.len()
        {
            return Err(AtpgSeqError::LatchOrderingMismatch
            {
                expected: latch_to_pi_ordering.len(),
                actual: cube.len(),
            });
        }

        let mut state_cube = Vec::with_capacity(latch_to_pi_ordering.len());
        for position in latch_to_pi_ordering
        {
            let Some(value) = cube.get(*position) else
            {
                return Err(AtpgSeqError::LatchOrderingMismatch
                {
                    expected: *position + 1,
                    actual: cube.len(),
                });
            };
            state_cube.push(*value);
        }

        let state_width = state_cube.len();
        add_minterms_to_product_start_states(
            &mut product_start_states,
            &mut state_cube,
            state_width,
        );
    }

    Ok(product_start_states)
}

pub fn check_outputs(
    current_set: &BTreeSet<Vec<u8>>,
    external_outputs: &[BTreeSet<Vec<u8>>],
) -> Result<Option<usize>, AtpgSeqError>
{
    if external_outputs.is_empty()
    {
        return Err(AtpgSeqError::MissingOutput);
    }

    for (index, output) in external_outputs.iter().enumerate()
    {
        if !current_set.is_subset(output)
        {
            return Ok(Some(index));
        }
    }

    Ok(None)
}

pub fn traverse_product_machine<T, F>(
    machine: &T,
    init_states: BTreeSet<T::State>,
    output_is_correct: F,
) -> Result<Vec<T::Input>, AtpgSeqError>
where
    T: SequentialTransitionSystem,
    F: Fn(&T::State) -> bool,
{
    let mut queue = VecDeque::new();
    let mut predecessor = BTreeMap::<T::State, Option<(T::State, T::Input)>>::new();

    for state in init_states
    {
        queue.push_back(state.clone());
        predecessor.entry(state).or_insert(None);
    }

    while let Some(state) = queue.pop_front()
    {
        if !output_is_correct(&state)
        {
            return Ok(reconstruct_input_sequence(&predecessor, state));
        }

        for (input, next_state) in machine.successors(&state)
        {
            if predecessor.contains_key(&next_state)
            {
                continue;
            }
            predecessor.insert(next_state.clone(), Some((state.clone(), input)));
            queue.push_back(next_state);
        }
    }

    Err(AtpgSeqError::NoDistinguishingSequence)
}

fn reverse_generate_sequence<T>(
    machine: &T,
    reached_sets: &[BTreeSet<T::State>],
    depth: usize,
    mut state: T::State,
) -> Result<Vec<(T::Input, T::State)>, AtpgSeqError>
where
    T: SequentialTransitionSystem,
{
    let mut trace = Vec::with_capacity(depth);

    for index in (0..depth).rev()
    {
        let Some((input, previous_state)) = machine
            .predecessors(&state)
            .into_iter()
            .find(|(_, previous)| reached_sets[index].contains(previous))
        else
        {
            return Err(AtpgSeqError::NoExcitationStateReachable);
        };

        trace.push((input, previous_state.clone()));
        state = previous_state;
    }

    Ok(trace)
}

fn reconstruct_input_sequence<S, I>(
    predecessor: &BTreeMap<S, Option<(S, I)>>,
    mut state: S,
) -> Vec<I>
where
    S: Clone + Ord,
    I: Clone,
{
    let mut sequence = Vec::new();
    while let Some(Some((previous, input))) = predecessor.get(&state)
    {
        sequence.push(input.clone());
        state = previous.clone();
    }
    sequence.reverse();
    sequence
}

fn ensure_sequence_shape(sequence: &mut InputSequence, length: usize, width: usize)
{
    while sequence.len() < length
    {
        sequence.push(vec![0; width]);
    }

    for vector in sequence.iter_mut().take(length)
    {
        vector.resize(width, 0);
    }
}

fn copy_sequence(
    source: &InputSequence,
    destination: &mut InputSequence,
) -> Result<(), AtpgSeqError>
{
    let width = source.first().map_or(0, Vec::len);
    for vector in source
    {
        if vector.len() != width
        {
            return Err(AtpgSeqError::InputWidthMismatch
            {
                expected: width,
                actual: vector.len(),
            });
        }
    }

    ensure_sequence_shape(destination, source.len(), width);
    for (destination_vector, source_vector) in destination.iter_mut().zip(source)
    {
        destination_vector.copy_from_slice(source_vector);
    }

    Ok(())
}

fn add_minterms_to_product_start_states(
    product_start_states: &mut BTreeSet<(Vec<u8>, Vec<u8>)>,
    state_cube: &mut [LogicValue],
    n_to_check: usize,
)
{
    for index in (0..n_to_check).rev()
    {
        if state_cube[index] == LogicValue::DontCare
        {
            state_cube[index] = LogicValue::Zero;
            add_minterms_to_product_start_states(product_start_states, state_cube, index);
            state_cube[index] = LogicValue::One;
            add_minterms_to_product_start_states(product_start_states, state_cube, index);
            state_cube[index] = LogicValue::DontCare;
            return;
        }
    }

    let minterm = state_cube
        .iter()
        .map(|value| value.concretize())
        .collect::<Vec<_>>();
    product_start_states.insert((minterm.clone(), minterm));
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn machine() -> ExplicitSequentialMachine<&'static str, &'static str>
    {
        ExplicitSequentialMachine::new(
            ["reset"],
            [
                ("reset", "a", "s1"),
                ("s1", "b", "s2"),
                ("s2", "c", "s2"),
                ("s1", "x", "bad"),
            ],
        )
    }

    #[test]
    fn reachable_states_stop_when_frontier_is_contained_in_total_set()
    {
        let reachability = calculate_reachable_states(&machine(), 8).unwrap();

        assert!(reachability.converged);
        assert_eq!(
            reachability.reached_sets,
            vec![
                BTreeSet::from(["reset"]),
                BTreeSet::from(["s1"]),
                BTreeSet::from(["bad", "s2"]),
            ]
        );
        assert_eq!(reachability.total_set, BTreeSet::from(["bad", "reset", "s1", "s2"]));
    }

    #[test]
    fn state_justify_reconstructs_reverse_trace_to_reachable_excitation_state()
    {
        let machine = machine();
        let reachability = calculate_reachable_states(&machine, 8).unwrap();
        let (depth, sequence, target) =
            state_justify(&machine, &reachability.reached_sets, &BTreeSet::from(["s2"])).unwrap();

        assert_eq!(depth, 2);
        assert_eq!(target, "s2");
        assert_eq!(sequence, vec!["a", "b"]);
    }

    #[test]
    fn trace_conversion_concretizes_dont_care_to_zero_and_caches_once()
    {
        let mut sequence = vec![vec![9, 9, 9]];
        let mut cache = SequenceCache::new();

        convert_trace_to_sequence(
            &[
                vec![LogicValue::One, LogicValue::DontCare],
                vec![LogicValue::Zero, LogicValue::One],
            ],
            &mut sequence,
            Some(&mut cache),
            42,
        )
        .unwrap();

        assert_eq!(sequence, vec![vec![1, 0], vec![0, 1]]);
        assert_eq!(
            cache.get(&42),
            Some(&Some(vec![vec![1, 0], vec![0, 1]]))
        );

        sequence[0] = vec![7, 7];
        convert_trace_to_sequence(
            &[vec![LogicValue::Zero, LogicValue::Zero]],
            &mut sequence,
            Some(&mut cache),
            42,
        )
        .unwrap();

        assert_eq!(
            cache.get(&42),
            Some(&Some(vec![vec![1, 0], vec![0, 1]]))
        );
    }

    #[test]
    fn sequence_reuse_copies_cached_vectors_and_nil_propagation_is_zero_length()
    {
        let mut just_cache = SequenceCache::new();
        just_cache.insert(7, Some(vec![vec![1, 0], vec![0, 1]]));
        let mut destination = Vec::new();

        assert_eq!(
            reuse_justification_sequence(&just_cache, &7, &mut destination).unwrap(),
            2
        );
        assert_eq!(destination, vec![vec![1, 0], vec![0, 1]]);

        let mut prop_cache = SequenceCache::new();
        prop_cache.insert(9, None);
        assert_eq!(
            reuse_propagation_sequence(&prop_cache, &9, &mut destination).unwrap(),
            0
        );
    }

    #[test]
    fn product_start_states_expand_dont_care_latch_values_into_good_faulty_pairs()
    {
        let starts = construct_product_start_states(
            &[vec![LogicValue::One, LogicValue::DontCare, LogicValue::Zero]],
            &[1, 0],
        )
        .unwrap();

        assert_eq!(
            starts,
            BTreeSet::from([
                (vec![0, 1], vec![0, 1]),
                (vec![1, 1], vec![1, 1]),
            ])
        );
    }

    #[test]
    fn output_check_returns_first_output_not_covering_current_set()
    {
        let current_set = BTreeSet::from([vec![0], vec![1]]);
        let outputs = [
            BTreeSet::from([vec![0], vec![1]]),
            BTreeSet::from([vec![0]]),
        ];

        assert_eq!(check_outputs(&current_set, &outputs), Ok(Some(1)));
        assert_eq!(check_outputs(&current_set, &outputs[..1]), Ok(None));
    }

    #[test]
    fn product_traversal_returns_shortest_distinguishing_input_sequence()
    {
        let sequence = traverse_product_machine(
            &machine(),
            BTreeSet::from(["reset"]),
            |state| *state != "bad",
        )
        .unwrap();

        assert_eq!(sequence, vec!["a", "x"]);
    }
}
