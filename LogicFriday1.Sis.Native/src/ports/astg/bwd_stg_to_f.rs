use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub type StateCode = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind {
    Input,
    Output,
    Internal,
    Dummy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal {
    pub name: String,
    pub state_bit: StateCode,
    pub kind: SignalKind,
}

impl Signal {
    pub fn new(name: impl Into<String>, state_bit: StateCode, kind: SignalKind) -> Self {
        Self {
            name: name.into(),
            state_bit,
            kind,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Place {
    pub name: String,
    pub outputs: Vec<usize>,
}

impl Place {
    pub fn new(name: impl Into<String>, outputs: Vec<usize>) -> Self {
        Self {
            name: name.into(),
            outputs,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub signal: usize,
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
}

impl Transition {
    pub fn new(signal: usize, inputs: Vec<usize>, outputs: Vec<usize>) -> Self {
        Self {
            signal,
            inputs,
            outputs,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Marking {
    marked_places: BTreeSet<usize>,
}

impl Marking {
    pub fn new(marked_places: impl IntoIterator<Item = usize>) -> Self {
        Self {
            marked_places: marked_places.into_iter().collect(),
        }
    }

    pub fn is_marked(&self, place: usize) -> bool {
        self.marked_places.contains(&place)
    }

    pub fn marked_places(&self) -> impl Iterator<Item = usize> + '_ {
        self.marked_places.iter().copied()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateMarkings {
    pub state: StateCode,
    pub markings: Vec<Marking>,
}

impl StateMarkings {
    pub fn new(state: StateCode, markings: Vec<Marking>) -> Self {
        Self { state, markings }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgGraph {
    pub signals: Vec<Signal>,
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub states: Vec<StateMarkings>,
}

impl AstgGraph {
    pub fn new(
        signals: Vec<Signal>,
        places: Vec<Place>,
        transitions: Vec<Transition>,
        states: Vec<StateMarkings>,
    ) -> Self {
        Self {
            signals,
            places,
            transitions,
            states,
        }
    }

    fn non_dummy_signals(&self) -> impl Iterator<Item = &Signal> {
        self.signals
            .iter()
            .filter(|signal| signal.kind != SignalKind::Dummy)
    }

    fn markings_for_state(&self, state: StateCode) -> Option<&[Marking]> {
        self.states
            .iter()
            .find(|entry| entry.state == state)
            .map(|entry| entry.markings.as_slice())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    pub inputs: Vec<Literal>,
}

impl Cube {
    pub fn new(inputs: Vec<Literal>) -> Self {
        Self { inputs }
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.inputs
            .iter()
            .zip(other.inputs.iter())
            .all(|(left, right)| {
                !matches!(
                    (left, right),
                    (Literal::Zero, Literal::One) | (Literal::One, Literal::Zero)
                )
            })
    }

    pub fn implies(&self, other: &Self) -> bool {
        self.inputs
            .iter()
            .zip(other.inputs.iter())
            .all(|(left, right)| *right == Literal::DontCare || left == right)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Cover {
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    fn add_cube(&mut self, cube: Cube) {
        if !self.cubes.contains(&cube) {
            self.cubes.push(cube);
        }
    }

    fn contain(&mut self) {
        let mut contained = vec![false; self.cubes.len()];

        for left_index in 0..self.cubes.len() {
            for right_index in 0..self.cubes.len() {
                if left_index != right_index
                    && self.cubes[left_index].implies(&self.cubes[right_index])
                {
                    contained[left_index] = true;
                    break;
                }
            }
        }

        self.cubes = self
            .cubes
            .iter()
            .enumerate()
            .filter_map(|(index, cube)| (!contained[index]).then_some(cube.clone()))
            .collect();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionCovers {
    pub signal_name: String,
    pub on_set: Cover,
    pub off_set: Cover,
    pub conflicts: Vec<Conflict>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conflict {
    pub cube: Cube,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SynthesisOptions {
    pub force_disjoint: bool,
}

impl Default for SynthesisOptions {
    fn default() -> Self {
        Self {
            force_disjoint: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BwdStgToFError {
    MissingSignal {
        signal_index: usize,
    },
    MissingState {
        state: StateCode,
    },
    MissingTransition {
        transition_index: usize,
    },
    MissingPlace {
        place_index: usize,
    },
    DeadMarking {
        state: StateCode,
        marking: Marking,
    },
    ConflictingCovers {
        signal_name: String,
        conflicts: Vec<Conflict>,
    },
}

impl fmt::Display for BwdStgToFError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSignal { signal_index } => {
                write!(formatter, "missing ASTG signal at index {signal_index}")
            }
            Self::MissingState { state } => write!(formatter, "missing ASTG state {state:#x}"),
            Self::MissingTransition { transition_index } => write!(
                formatter,
                "missing ASTG transition at index {transition_index}"
            ),
            Self::MissingPlace { place_index } => {
                write!(formatter, "missing ASTG place at index {place_index}")
            }
            Self::DeadMarking { state, .. } => write!(
                formatter,
                "the STG is not live after reaching state {state:#x}"
            ),
            Self::ConflictingCovers {
                signal_name,
                conflicts,
            } => write!(
                formatter,
                "state assignment problem for {signal_name}: {} conflicting cube(s)",
                conflicts.len()
            ),
        }
    }
}

impl Error for BwdStgToFError {}

pub fn synthesize_signal_function(
    astg: &AstgGraph,
    signal_index: usize,
    initial_state: StateCode,
    options: SynthesisOptions,
) -> Result<FunctionCovers, BwdStgToFError> {
    let signal = astg
        .signals
        .get(signal_index)
        .ok_or(BwdStgToFError::MissingSignal { signal_index })?;
    let mut context = SearchContext::new(astg, signal.state_bit);

    context.reached.insert(initial_state);
    astg_to_f_recur(&mut context, initial_state)?;
    context.on_set.contain();
    context.off_set.contain();

    let conflicts = cover_intersection(&context.on_set, &context.off_set);
    if !conflicts.is_empty() {
        if !options.force_disjoint {
            return Err(BwdStgToFError::ConflictingCovers {
                signal_name: signal.name.clone(),
                conflicts,
            });
        }

        let on_set = context.on_set.cubes.clone();

        context
            .off_set
            .cubes
            .retain(|cube| !on_set.iter().any(|on_cube| cube.intersects(on_cube)));
    }

    Ok(FunctionCovers {
        signal_name: signal.name.clone(),
        on_set: context.on_set,
        off_set: context.off_set,
        conflicts: Vec::new(),
    })
}

pub fn marking_enabled(astg: &AstgGraph, marking: &Marking) -> Result<StateCode, BwdStgToFError> {
    let mut enabled = 0;

    for transition in &astg.transitions {
        if transition
            .inputs
            .iter()
            .all(|place| marking.is_marked(*place))
        {
            let signal =
                astg.signals
                    .get(transition.signal)
                    .ok_or(BwdStgToFError::MissingSignal {
                        signal_index: transition.signal,
                    })?;
            enabled |= signal.state_bit;
        }
    }

    Ok(enabled)
}

pub fn marking_enabled_with_dummy_closure(
    astg: &AstgGraph,
    marking: &Marking,
) -> Result<StateCode, BwdStgToFError> {
    let mut active_dummy_transitions = BTreeSet::new();
    marking_enabled_dummy_recur(astg, marking, &mut active_dummy_transitions)
}

fn astg_to_f_recur(
    context: &mut SearchContext<'_>,
    state: StateCode,
) -> Result<(), BwdStgToFError> {
    let markings = context
        .astg
        .markings_for_state(state)
        .ok_or(BwdStgToFError::MissingState { state })?
        .to_vec();

    for marking in markings {
        let enabled = marking_enabled(context.astg, &marking)?;
        if enabled == 0 {
            continue;
        }

        let value = if (enabled & context.signal_bit) != 0 {
            state ^ context.signal_bit
        } else {
            state
        };
        let Some(first_signal) = first_enabled_signal_at_or_after(context.astg, enabled, 0) else {
            continue;
        };

        find_maximal_subset(context, 0, first_signal, state, value, enabled)?;

        for signal in &context.astg.signals {
            let to_fire = signal.state_bit;
            if (to_fire & enabled) == 0 {
                continue;
            }

            let new_state = state ^ to_fire;
            if context.reached.insert(new_state) {
                astg_to_f_recur(context, new_state)?;
            }
        }
    }

    Ok(())
}

fn find_maximal_subset(
    context: &mut SearchContext<'_>,
    current_free_choice: usize,
    current_signal: usize,
    state: StateCode,
    value: StateCode,
    enabled: StateCode,
) -> Result<(), BwdStgToFError> {
    let new_state = state ^ enabled;

    if let Some(place_index) =
        next_marked_free_choice_place(context.astg, state, current_free_choice)?
    {
        let place = context
            .astg
            .places
            .get(place_index)
            .ok_or(BwdStgToFError::MissingPlace { place_index })?;
        let mut free_choice_bits = 0;
        let mut free_choice_mask = !0;

        for transition_index in &place.outputs {
            let transition = context.astg.transitions.get(*transition_index).ok_or(
                BwdStgToFError::MissingTransition {
                    transition_index: *transition_index,
                },
            )?;
            let signal = context.astg.signals.get(transition.signal).ok_or(
                BwdStgToFError::MissingSignal {
                    signal_index: transition.signal,
                },
            )?;
            free_choice_bits |= signal.state_bit;
            free_choice_mask &= !signal.state_bit;
        }

        let enabled_free_choice_bits = free_choice_bits & enabled;
        let enabled_without_choice = enabled & free_choice_mask;

        if enabled_free_choice_bits == 0 {
            find_maximal_subset(
                context,
                current_free_choice + 1,
                current_signal,
                state,
                value,
                enabled,
            )?;
        } else {
            for transition_index in &place.outputs {
                let transition = &context.astg.transitions[*transition_index];
                let bit = context.astg.signals[transition.signal].state_bit;
                if (enabled_free_choice_bits & bit) != 0 {
                    find_maximal_subset(
                        context,
                        current_free_choice + 1,
                        current_signal,
                        state,
                        value,
                        enabled_without_choice | bit,
                    )?;
                }
            }
        }

        return Ok(());
    }

    let markings = context
        .astg
        .markings_for_state(new_state)
        .ok_or(BwdStgToFError::MissingState { state: new_state })?
        .to_vec();

    for marking in markings {
        let new_enabled = marking_enabled_with_dummy_closure(context.astg, &marking)?;
        if new_enabled == 0 {
            return Err(BwdStgToFError::DeadMarking {
                state: new_state,
                marking,
            });
        }

        if (context.signal_bit & new_enabled) != 0 {
            let Some(split_signal) =
                first_enabled_signal_at_or_after(context.astg, enabled, current_signal)
            else {
                continue;
            };
            let bit = context.astg.signals[split_signal].state_bit;

            find_maximal_subset(
                context,
                current_free_choice,
                split_signal + 1,
                state,
                value,
                enabled,
            )?;

            let reduced_enabled = enabled & !bit;
            if reduced_enabled != 0 {
                find_maximal_subset(
                    context,
                    current_free_choice,
                    split_signal + 1,
                    state,
                    value,
                    reduced_enabled,
                )?;
            }
        } else {
            let cube = cube_for_state_value(context.astg, value, enabled);
            if (context.signal_bit & value) != 0 {
                context.on_set.add_cube(cube);
            } else {
                context.off_set.add_cube(cube);
            }
        }
    }

    Ok(())
}

fn next_marked_free_choice_place(
    astg: &AstgGraph,
    state: StateCode,
    current_free_choice: usize,
) -> Result<Option<usize>, BwdStgToFError> {
    let markings = astg
        .markings_for_state(state)
        .ok_or(BwdStgToFError::MissingState { state })?;
    let mut free_choice_count = 0;

    for marking in markings {
        for (place_index, place) in astg.places.iter().enumerate() {
            if !marking.is_marked(place_index) {
                continue;
            }

            if place.outputs.len() > 1 {
                free_choice_count += 1;
            }

            if free_choice_count > current_free_choice {
                return Ok(Some(place_index));
            }
        }
    }

    Ok(None)
}

fn marking_enabled_dummy_recur(
    astg: &AstgGraph,
    marking: &Marking,
    active_dummy_transitions: &mut BTreeSet<usize>,
) -> Result<StateCode, BwdStgToFError> {
    let mut result = 0;

    for (transition_index, transition) in astg.transitions.iter().enumerate() {
        if !transition
            .inputs
            .iter()
            .all(|place| marking.is_marked(*place))
        {
            continue;
        }

        let signal = astg
            .signals
            .get(transition.signal)
            .ok_or(BwdStgToFError::MissingSignal {
                signal_index: transition.signal,
            })?;

        result |= signal.state_bit;

        if signal.kind == SignalKind::Dummy && active_dummy_transitions.insert(transition_index) {
            let fired_marking = fire_transition(marking, transition);
            result |= marking_enabled_dummy_recur(astg, &fired_marking, active_dummy_transitions)?;
            active_dummy_transitions.remove(&transition_index);
        }
    }

    Ok(result)
}

fn fire_transition(marking: &Marking, transition: &Transition) -> Marking {
    let mut marked = marking.marked_places.clone();

    for input in &transition.inputs {
        marked.remove(input);
    }

    for output in &transition.outputs {
        marked.insert(*output);
    }

    Marking {
        marked_places: marked,
    }
}

fn first_enabled_signal_at_or_after(
    astg: &AstgGraph,
    enabled: StateCode,
    first_index: usize,
) -> Option<usize> {
    astg.signals
        .iter()
        .enumerate()
        .skip(first_index)
        .find_map(|(index, signal)| ((signal.state_bit & enabled) != 0).then_some(index))
}

fn cube_for_state_value(astg: &AstgGraph, value: StateCode, enabled: StateCode) -> Cube {
    Cube::new(
        astg.non_dummy_signals()
            .map(|signal| {
                if (signal.state_bit & enabled) != 0 {
                    Literal::DontCare
                } else if (signal.state_bit & value) != 0 {
                    Literal::One
                } else {
                    Literal::Zero
                }
            })
            .collect(),
    )
}

fn cover_intersection(on_set: &Cover, off_set: &Cover) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    for on_cube in &on_set.cubes {
        for off_cube in &off_set.cubes {
            if on_cube.intersects(off_cube) {
                conflicts.push(Conflict {
                    cube: on_cube.clone(),
                });
                break;
            }
        }
    }

    conflicts
}

struct SearchContext<'a> {
    astg: &'a AstgGraph,
    signal_bit: StateCode,
    reached: BTreeSet<StateCode>,
    on_set: Cover,
    off_set: Cover,
}

impl<'a> SearchContext<'a> {
    fn new(astg: &'a AstgGraph, signal_bit: StateCode) -> Self {
        Self {
            astg,
            signal_bit,
            reached: BTreeSet::new(),
            on_set: Cover::new(),
            off_set: Cover::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_signal_graph() -> AstgGraph {
        AstgGraph::new(
            vec![
                Signal::new("a", 0b01, SignalKind::Output),
                Signal::new("b", 0b10, SignalKind::Output),
            ],
            vec![
                Place::new("p0", vec![0]),
                Place::new("p1", vec![1]),
                Place::new("p2", vec![2]),
                Place::new("p3", vec![3]),
            ],
            vec![
                Transition::new(0, vec![0], vec![1]),
                Transition::new(1, vec![1], vec![2]),
                Transition::new(0, vec![2], vec![3]),
                Transition::new(1, vec![3], vec![0]),
            ],
            vec![
                StateMarkings::new(0b00, vec![Marking::new([0])]),
                StateMarkings::new(0b01, vec![Marking::new([1])]),
                StateMarkings::new(0b11, vec![Marking::new([2])]),
                StateMarkings::new(0b10, vec![Marking::new([3])]),
            ],
        )
    }

    #[test]
    fn dummy_closure_includes_transitions_after_dummy_fire() {
        let astg = AstgGraph::new(
            vec![
                Signal::new("d", 0b01, SignalKind::Dummy),
                Signal::new("x", 0b10, SignalKind::Output),
            ],
            vec![Place::new("p0", vec![0]), Place::new("p1", vec![1])],
            vec![
                Transition::new(0, vec![0], vec![1]),
                Transition::new(1, vec![1], vec![]),
            ],
            vec![StateMarkings::new(0, vec![Marking::new([0])])],
        );

        let enabled = marking_enabled_with_dummy_closure(&astg, &Marking::new([0])).unwrap();

        assert_eq!(enabled, 0b11);
    }

    #[test]
    fn synthesize_signal_function_generates_on_and_off_covers() {
        let astg = two_signal_graph();

        let result = synthesize_signal_function(&astg, 0, 0, SynthesisOptions::default()).unwrap();

        assert_eq!(result.signal_name, "a");
        assert_eq!(
            result.on_set.cubes(),
            &[Cube::new(vec![Literal::DontCare, Literal::Zero])]
        );
        assert_eq!(
            result.off_set.cubes(),
            &[Cube::new(vec![Literal::DontCare, Literal::One])]
        );
    }

    #[test]
    fn free_choice_place_splits_enabled_fanout_transitions() {
        let astg = AstgGraph::new(
            vec![
                Signal::new("a", 0b01, SignalKind::Output),
                Signal::new("b", 0b10, SignalKind::Output),
            ],
            vec![
                Place::new("p0", vec![0, 1]),
                Place::new("p1", vec![2]),
                Place::new("p2", vec![3]),
                Place::new("p3", vec![4]),
            ],
            vec![
                Transition::new(0, vec![0], vec![1]),
                Transition::new(1, vec![0], vec![2]),
                Transition::new(1, vec![1], vec![3]),
                Transition::new(0, vec![2], vec![3]),
                Transition::new(0, vec![3], vec![0]),
            ],
            vec![
                StateMarkings::new(0b00, vec![Marking::new([0])]),
                StateMarkings::new(0b01, vec![Marking::new([1])]),
                StateMarkings::new(0b10, vec![Marking::new([2])]),
                StateMarkings::new(0b11, vec![Marking::new([3])]),
            ],
        );

        let result = synthesize_signal_function(&astg, 0, 0, SynthesisOptions::default()).unwrap();

        assert!(result
            .on_set
            .cubes()
            .contains(&Cube::new(vec![Literal::DontCare, Literal::Zero])));
        assert_eq!(
            result.on_set.cubes(),
            &[Cube::new(vec![Literal::DontCare, Literal::Zero])]
        );
    }

    #[test]
    fn cover_intersection_reports_conflicting_cubes() {
        let mut on_set = Cover::new();
        let mut off_set = Cover::new();
        on_set.add_cube(Cube::new(vec![Literal::One, Literal::DontCare]));
        off_set.add_cube(Cube::new(vec![Literal::DontCare, Literal::Zero]));

        let conflicts = cover_intersection(&on_set, &off_set);

        assert_eq!(
            conflicts,
            vec![Conflict {
                cube: Cube::new(vec![Literal::One, Literal::DontCare]),
            }]
        );
    }

    #[test]
    fn forced_conflicts_remove_intersecting_off_set_cubes() {
        let astg = AstgGraph::new(
            vec![
                Signal::new("a", 0b01, SignalKind::Output),
                Signal::new("b", 0b10, SignalKind::Output),
            ],
            vec![Place::new("p0", vec![0]), Place::new("p1", vec![1])],
            vec![
                Transition::new(0, vec![0], vec![0]),
                Transition::new(0, vec![1], vec![1]),
            ],
            vec![
                StateMarkings::new(0b00, vec![Marking::new([0])]),
                StateMarkings::new(0b01, vec![Marking::new([1])]),
            ],
        );

        let result = synthesize_signal_function(
            &astg,
            0,
            0,
            SynthesisOptions {
                force_disjoint: true,
            },
        )
        .unwrap();

        assert!(result.off_set.cubes().is_empty());
    }

    #[test]
    fn no_legacy_exports_or_dependency_metadata_are_present() {
        let source = include_str!("bwd_stg_to_f.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
