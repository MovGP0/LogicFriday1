//! Native support for the ASTG backward state-coding pass.
//!
//! This module keeps the decision logic from the legacy pass independent from
//! SIS globals: select state signals that distinguish reduced STG partitions,
//! choose one class per state, and describe the state-variable transitions that
//! must be inserted between marking pairs.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub type StateCode = u64;
pub type StateId = usize;
pub type ClassId = usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind {
    Input,
    Output,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal {
    pub name: String,
    pub bit: StateCode,
    pub kind: SignalKind,
    pub cost: Option<usize>,
}

impl Signal {
    pub fn new(name: impl Into<String>, bit: StateCode, kind: SignalKind) -> Self {
        Self {
            name: name.into(),
            bit,
            kind,
            cost: None,
        }
    }

    pub fn with_cost(mut self, cost: usize) -> Self {
        self.cost = Some(cost);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub name: String,
    pub code: Option<String>,
}

impl State {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            code: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub from: StateId,
    pub to: StateId,
    pub newly_enabled: StateCode,
}

impl Transition {
    pub fn new(from: StateId, to: StateId, newly_enabled: StateCode) -> Self {
        Self {
            from,
            to,
            newly_enabled,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquivalenceClasses {
    class_count: usize,
    state_classes: Vec<BTreeSet<ClassId>>,
}

impl EquivalenceClasses {
    pub fn new(
        class_count: usize,
        state_classes: Vec<BTreeSet<ClassId>>,
    ) -> Result<Self, BwdCodeError> {
        if class_count == 0 && !state_classes.is_empty() {
            return Err(BwdCodeError::NoClasses);
        }

        for (state, classes) in state_classes.iter().enumerate() {
            if classes.is_empty() {
                return Err(BwdCodeError::StateHasNoClass { state });
            }

            for class in classes {
                if *class >= class_count {
                    return Err(BwdCodeError::ClassOutOfRange {
                        state,
                        class: *class,
                        class_count,
                    });
                }
            }
        }

        Ok(Self {
            class_count,
            state_classes,
        })
    }

    pub fn from_state_classes<const N: usize>(
        class_count: usize,
        state_classes: [[ClassId; N]; N],
    ) -> Result<Self, BwdCodeError> {
        Self::new(
            class_count,
            state_classes
                .into_iter()
                .map(|classes| classes.into_iter().collect())
                .collect(),
        )
    }

    pub fn class_count(&self) -> usize {
        self.class_count
    }

    pub fn state_count(&self) -> usize {
        self.state_classes.len()
    }

    pub fn classes_for_state(&self, state: StateId) -> Option<&BTreeSet<ClassId>> {
        self.state_classes.get(state)
    }

    pub fn same_class(&self, left: StateId, right: StateId) -> bool {
        match (self.state_classes.get(left), self.state_classes.get(right)) {
            (Some(left_classes), Some(right_classes)) => left_classes
                .iter()
                .any(|class| right_classes.contains(class)),
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionPhase {
    Positive,
    Negative,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSignalTransition {
    pub signal_name: String,
    pub phase: TransitionPhase,
    pub marking_pair_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkingPair {
    pub from_state: StateId,
    pub to_state: StateId,
}

impl MarkingPair {
    pub fn new(from_state: StateId, to_state: StateId) -> Self {
        Self {
            from_state,
            to_state,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateVariablePlan {
    pub state_signals: Vec<String>,
    pub transitions: Vec<StateSignalTransition>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartitionResult {
    pub selected_signal_bits: StateCode,
    pub selected_signal_names: Vec<String>,
    pub partitions: Vec<ClassId>,
    pub total_cost: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BwdCodeError {
    NoClasses,
    StateHasNoClass {
        state: StateId,
    },
    ClassOutOfRange {
        state: StateId,
        class: ClassId,
        class_count: usize,
    },
    StateOutOfRange {
        state: StateId,
        state_count: usize,
    },
    UnknownSignalBit {
        bit: StateCode,
    },
    EmptySignalBit {
        signal: String,
    },
    DuplicateSignalBit {
        bit: StateCode,
    },
    MissingStateCode {
        state: StateId,
    },
    StateCodeLengthMismatch {
        state: StateId,
        expected: usize,
        actual: usize,
    },
    InvalidStateCodeBit {
        state: StateId,
        index: usize,
        value: char,
    },
    NoPartitionAssignment,
}

impl fmt::Display for BwdCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoClasses => write!(formatter, "equivalence data has no classes"),
            Self::StateHasNoClass { state } => {
                write!(formatter, "state {state} belongs to no equivalence class")
            }
            Self::ClassOutOfRange {
                state,
                class,
                class_count,
            } => write!(
                formatter,
                "state {state} references class {class}, but only {class_count} classes exist"
            ),
            Self::StateOutOfRange { state, state_count } => write!(
                formatter,
                "state {state} is outside the {state_count}-state graph"
            ),
            Self::UnknownSignalBit { bit } => {
                write!(formatter, "transition references unknown signal bit {bit}")
            }
            Self::EmptySignalBit { signal } => {
                write!(formatter, "signal {signal} has no state bit")
            }
            Self::DuplicateSignalBit { bit } => {
                write!(formatter, "multiple signals use state bit {bit}")
            }
            Self::MissingStateCode { state } => {
                write!(formatter, "state {state} has no state encoding")
            }
            Self::StateCodeLengthMismatch {
                state,
                expected,
                actual,
            } => write!(
                formatter,
                "state {state} has {actual} encoding bits, expected {expected}"
            ),
            Self::InvalidStateCodeBit {
                state,
                index,
                value,
            } => write!(
                formatter,
                "state {state} has invalid encoding bit {value:?} at index {index}"
            ),
            Self::NoPartitionAssignment => write!(
                formatter,
                "no valid reduced-state partition assignment exists"
            ),
        }
    }
}

impl Error for BwdCodeError {}

pub fn signal_cost(signal: &Signal, signal_count: usize) -> usize {
    if let Some(cost) = signal.cost {
        return cost;
    }

    match signal.kind {
        SignalKind::Input => 100.max(2 * signal_count),
        SignalKind::Output | SignalKind::Internal => 1,
    }
}

pub fn subset_cost(signals: &[Signal], subset: StateCode) -> usize {
    signals
        .iter()
        .filter(|signal| (subset & signal.bit) != 0)
        .map(|signal| signal_cost(signal, signals.len()))
        .sum()
}

pub fn distinguishable(transition: &Transition, subset: StateCode) -> bool {
    transition.newly_enabled != 0 && (subset & transition.newly_enabled) == transition.newly_enabled
}

pub fn partition_states(
    signals: &[Signal],
    classes: &EquivalenceClasses,
    transitions: &[Transition],
) -> Result<PartitionResult, BwdCodeError> {
    validate_signals(signals)?;
    validate_transitions(classes.state_count(), signals, transitions)?;

    if classes.class_count() < 2 {
        return Ok(PartitionResult {
            selected_signal_bits: 0,
            selected_signal_names: Vec::new(),
            partitions: vec![0; classes.state_count()],
            total_cost: 0,
        });
    }

    let candidate_bits = candidate_signal_bits(signals);
    let mut best: Option<PartitionResult> = None;

    for subset in enumerate_subsets(&candidate_bits) {
        let Some(partitions) = choose_partitions(classes, transitions, subset)? else {
            continue;
        };

        let total_cost = subset_cost(signals, subset);
        let selected_signal_names = signals
            .iter()
            .filter(|signal| (subset & signal.bit) != 0)
            .map(|signal| signal.name.clone())
            .collect();
        let result = PartitionResult {
            selected_signal_bits: subset,
            selected_signal_names,
            partitions,
            total_cost,
        };

        if best
            .as_ref()
            .is_none_or(|best| result.total_cost < best.total_cost)
        {
            best = Some(result);
        }
    }

    best.ok_or(BwdCodeError::NoPartitionAssignment)
}

pub fn choose_partitions(
    classes: &EquivalenceClasses,
    transitions: &[Transition],
    selected_signal_bits: StateCode,
) -> Result<Option<Vec<ClassId>>, BwdCodeError> {
    validate_transitions(classes.state_count(), &[], transitions)?;

    let mut assignments = vec![None; classes.state_count()];
    let mut zero_enabled_components = ZeroEnabledComponents::new(classes.state_count());

    for transition in transitions {
        if transition.from == transition.to {
            continue;
        }

        if transition.newly_enabled == 0 {
            zero_enabled_components.union(transition.from, transition.to);
        }
    }

    let mut component_classes = BTreeMap::<StateId, BTreeSet<ClassId>>::new();
    for state in 0..classes.state_count() {
        let root = zero_enabled_components.find(state);
        let state_classes =
            classes
                .classes_for_state(state)
                .ok_or(BwdCodeError::StateOutOfRange {
                    state,
                    state_count: classes.state_count(),
                })?;

        component_classes
            .entry(root)
            .and_modify(|allowed| *allowed = allowed.intersection(state_classes).copied().collect())
            .or_insert_with(|| state_classes.clone());
    }

    if component_classes.values().any(BTreeSet::is_empty) {
        return Ok(None);
    }

    let mut components = component_classes
        .into_iter()
        .map(|(root, allowed)| ComponentChoice {
            root,
            states: (0..classes.state_count())
                .filter(|state| zero_enabled_components.find(*state) == root)
                .collect(),
            allowed: allowed.into_iter().collect(),
        })
        .collect::<Vec<_>>();

    components.sort_by_key(|component| component.allowed.len());

    if assign_component(
        0,
        &components,
        transitions,
        selected_signal_bits,
        &zero_enabled_components,
        &mut assignments,
    ) {
        Ok(Some(assignments.into_iter().map(Option::unwrap).collect()))
    } else {
        Ok(None)
    }
}

pub fn plan_state_variable_insertion(
    states: &[State],
    marking_pairs: &[MarkingPair],
    state_signal_prefix: &str,
) -> Result<StateVariablePlan, BwdCodeError> {
    let bit_count = state_code_bit_count(states)?;
    let state_signals = (0..bit_count)
        .map(|index| format!("{state_signal_prefix}{index}"))
        .collect::<Vec<_>>();
    let mut transitions = Vec::new();

    for (pair_index, pair) in marking_pairs.iter().enumerate() {
        let from_code = code_for_state(states, pair.from_state)?;
        let to_code = code_for_state(states, pair.to_state)?;

        for (bit_index, (from_bit, to_bit)) in from_code.chars().zip(to_code.chars()).enumerate() {
            if from_bit == to_bit {
                continue;
            }

            transitions.push(StateSignalTransition {
                signal_name: state_signals[bit_index].clone(),
                phase: transition_phase(from_bit, pair.from_state, bit_index)?,
                marking_pair_index: pair_index + 1,
            });
        }
    }

    Ok(StateVariablePlan {
        state_signals,
        transitions,
    })
}

fn validate_signals(signals: &[Signal]) -> Result<(), BwdCodeError> {
    let mut seen = BTreeSet::new();

    for signal in signals {
        if signal.bit == 0 {
            return Err(BwdCodeError::EmptySignalBit {
                signal: signal.name.clone(),
            });
        }

        if !seen.insert(signal.bit) {
            return Err(BwdCodeError::DuplicateSignalBit { bit: signal.bit });
        }
    }

    Ok(())
}

fn validate_transitions(
    state_count: usize,
    signals: &[Signal],
    transitions: &[Transition],
) -> Result<(), BwdCodeError> {
    let known_bits = signals
        .iter()
        .fold(0, |known_bits, signal| known_bits | signal.bit);

    for transition in transitions {
        if transition.from >= state_count {
            return Err(BwdCodeError::StateOutOfRange {
                state: transition.from,
                state_count,
            });
        }

        if transition.to >= state_count {
            return Err(BwdCodeError::StateOutOfRange {
                state: transition.to,
                state_count,
            });
        }

        if !signals.is_empty() && (transition.newly_enabled & !known_bits) != 0 {
            return Err(BwdCodeError::UnknownSignalBit {
                bit: transition.newly_enabled & !known_bits,
            });
        }
    }

    Ok(())
}

fn candidate_signal_bits(signals: &[Signal]) -> Vec<StateCode> {
    signals.iter().map(|signal| signal.bit).collect()
}

fn enumerate_subsets(bits: &[StateCode]) -> Vec<StateCode> {
    let mut subsets = Vec::new();
    let count = 1usize << bits.len();

    for mask in 0..count {
        let mut subset = 0;

        for (index, bit) in bits.iter().enumerate() {
            if (mask & (1usize << index)) != 0 {
                subset |= *bit;
            }
        }

        subsets.push(subset);
    }

    subsets
}

fn assign_component(
    index: usize,
    components: &[ComponentChoice],
    transitions: &[Transition],
    selected_signal_bits: StateCode,
    zero_enabled_components: &ZeroEnabledComponents,
    assignments: &mut [Option<ClassId>],
) -> bool {
    if index == components.len() {
        return true;
    }

    let component = &components[index];

    for class in &component.allowed {
        for state in &component.states {
            assignments[*state] = Some(*class);
        }

        if partial_assignment_is_valid(
            transitions,
            selected_signal_bits,
            zero_enabled_components,
            assignments,
        ) && assign_component(
            index + 1,
            components,
            transitions,
            selected_signal_bits,
            zero_enabled_components,
            assignments,
        ) {
            return true;
        }

        for state in &component.states {
            assignments[*state] = None;
        }
    }

    false
}

fn partial_assignment_is_valid(
    transitions: &[Transition],
    selected_signal_bits: StateCode,
    zero_enabled_components: &ZeroEnabledComponents,
    assignments: &[Option<ClassId>],
) -> bool {
    for transition in transitions {
        if transition.from == transition.to {
            continue;
        }

        let Some(from_class) = assignments[transition.from] else {
            continue;
        };
        let Some(to_class) = assignments[transition.to] else {
            continue;
        };

        if transition.newly_enabled == 0 {
            if zero_enabled_components.find(transition.from)
                == zero_enabled_components.find(transition.to)
            {
                continue;
            }

            if from_class != to_class {
                return false;
            }
        } else if from_class == to_class && !distinguishable(transition, selected_signal_bits) {
            return false;
        }
    }

    true
}

fn state_code_bit_count(states: &[State]) -> Result<usize, BwdCodeError> {
    let mut bit_count = None;

    for (state_index, state) in states.iter().enumerate() {
        let code = state
            .code
            .as_deref()
            .ok_or(BwdCodeError::MissingStateCode { state: state_index })?;

        for (bit_index, value) in code.chars().enumerate() {
            if value != '0' && value != '1' {
                return Err(BwdCodeError::InvalidStateCodeBit {
                    state: state_index,
                    index: bit_index,
                    value,
                });
            }
        }

        match bit_count {
            Some(expected) if expected != code.len() => {
                return Err(BwdCodeError::StateCodeLengthMismatch {
                    state: state_index,
                    expected,
                    actual: code.len(),
                })
            }
            Some(_) => {}
            None => bit_count = Some(code.len()),
        }
    }

    Ok(bit_count.unwrap_or(0))
}

fn code_for_state(states: &[State], state: StateId) -> Result<&str, BwdCodeError> {
    let state_count = states.len();
    let state_info = states
        .get(state)
        .ok_or(BwdCodeError::StateOutOfRange { state, state_count })?;

    state_info
        .code
        .as_deref()
        .ok_or(BwdCodeError::MissingStateCode { state })
}

fn transition_phase(
    from_bit: char,
    state: StateId,
    index: usize,
) -> Result<TransitionPhase, BwdCodeError> {
    match from_bit {
        '0' => Ok(TransitionPhase::Positive),
        '1' => Ok(TransitionPhase::Negative),
        value => Err(BwdCodeError::InvalidStateCodeBit {
            state,
            index,
            value,
        }),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ComponentChoice {
    root: StateId,
    states: Vec<StateId>,
    allowed: Vec<ClassId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ZeroEnabledComponents {
    parent: Vec<StateId>,
}

impl ZeroEnabledComponents {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
        }
    }

    fn find(&self, state: StateId) -> StateId {
        let mut current = state;

        while self.parent[current] != current {
            current = self.parent[current];
        }

        current
    }

    fn union(&mut self, left: StateId, right: StateId) {
        let left_root = self.find(left);
        let right_root = self.find(right);

        if left_root != right_root {
            self.parent[right_root] = left_root;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[ClassId]) -> BTreeSet<ClassId> {
        values.iter().copied().collect()
    }

    fn classes() -> EquivalenceClasses {
        EquivalenceClasses::new(2, vec![set(&[0]), set(&[0, 1]), set(&[1])]).unwrap()
    }

    #[test]
    fn signal_cost_matches_legacy_input_penalty() {
        let signals = vec![
            Signal::new("a", 0b001, SignalKind::Output),
            Signal::new("b", 0b010, SignalKind::Internal),
            Signal::new("c", 0b100, SignalKind::Input),
        ];

        assert_eq!(signal_cost(&signals[0], signals.len()), 1);
        assert_eq!(signal_cost(&signals[2], signals.len()), 100);
        assert_eq!(subset_cost(&signals, 0b101), 101);
    }

    #[test]
    fn distinguishable_requires_all_newly_enabled_signals() {
        let transition = Transition::new(0, 1, 0b011);

        assert!(!distinguishable(&transition, 0b001));
        assert!(distinguishable(&transition, 0b011));
        assert!(!distinguishable(&Transition::new(0, 1, 0), 0b111));
    }

    #[test]
    fn same_class_detects_intersecting_equivalence_sets() {
        let classes = classes();

        assert!(classes.same_class(0, 1));
        assert!(classes.same_class(1, 2));
        assert!(!classes.same_class(0, 2));
    }

    #[test]
    fn partition_states_chooses_lowest_cost_distinguishing_subset() {
        let signals = vec![
            Signal::new("a", 0b001, SignalKind::Output).with_cost(5),
            Signal::new("b", 0b010, SignalKind::Output).with_cost(1),
        ];
        let transitions = vec![Transition::new(0, 1, 0b010), Transition::new(1, 2, 0b010)];

        let result = partition_states(&signals, &classes(), &transitions).unwrap();

        assert_eq!(result.selected_signal_bits, 0b010);
        assert_eq!(result.selected_signal_names, vec!["b"]);
        assert_eq!(result.total_cost, 1);
        assert_eq!(result.partitions, vec![0, 0, 1]);
    }

    #[test]
    fn zero_enabled_edges_force_common_partition() {
        let classes = EquivalenceClasses::new(2, vec![set(&[0]), set(&[0, 1]), set(&[1])]).unwrap();
        let transitions = vec![Transition::new(0, 1, 0), Transition::new(1, 2, 0)];

        assert_eq!(choose_partitions(&classes, &transitions, 0).unwrap(), None);
    }

    #[test]
    fn state_variable_plan_emits_one_transition_per_changed_code_bit() {
        let states = vec![
            State::new("s0").with_code("00"),
            State::new("s1").with_code("10"),
            State::new("s2").with_code("11"),
        ];
        let marking_pairs = vec![MarkingPair::new(0, 1), MarkingPair::new(1, 2)];

        let plan = plan_state_variable_insertion(&states, &marking_pairs, "sv").unwrap();

        assert_eq!(plan.state_signals, vec!["sv0", "sv1"]);
        assert_eq!(
            plan.transitions,
            vec![
                StateSignalTransition {
                    signal_name: "sv0".to_string(),
                    phase: TransitionPhase::Positive,
                    marking_pair_index: 1,
                },
                StateSignalTransition {
                    signal_name: "sv1".to_string(),
                    phase: TransitionPhase::Positive,
                    marking_pair_index: 2,
                },
            ]
        );
    }

    #[test]
    fn state_variable_plan_validates_code_lengths() {
        let states = vec![
            State::new("s0").with_code("0"),
            State::new("s1").with_code("10"),
        ];

        assert_eq!(
            plan_state_variable_insertion(&states, &[MarkingPair::new(0, 1)], "sv").unwrap_err(),
            BwdCodeError::StateCodeLengthMismatch {
                state: 1,
                expected: 1,
                actual: 2,
            }
        );
    }

    #[test]
    fn no_legacy_exports_or_dependency_metadata_are_present() {
        let source = include_str!("bwd_code.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
