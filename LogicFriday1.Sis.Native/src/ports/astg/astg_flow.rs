use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub type StateCode = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgFlowStatus {
    Ok,
    NotSafe,
    NotLive,
    NotUsc,
    NotCsa,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind {
    Input,
    Output,
    Internal,
    Dummy,
}

impl SignalKind {
    fn is_input(self) -> bool {
        matches!(self, Self::Input)
    }

    fn is_noninput(self) -> bool {
        matches!(self, Self::Output | Self::Internal)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionKind {
    Positive,
    Negative,
    Toggle,
    Dummy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SignalId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PlaceId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TransitionId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal {
    pub name: String,
    pub kind: SignalKind,
    pub state_bit: StateCode,
}

impl Signal {
    pub fn new(name: impl Into<String>, kind: SignalKind) -> Self {
        Self {
            name: name.into(),
            kind,
            state_bit: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Place {
    pub name: String,
    pub initial_token: bool,
    pub useful: bool,
}

impl Place {
    pub fn new(name: impl Into<String>, initial_token: bool) -> Self {
        Self {
            name: name.into(),
            initial_token,
            useful: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub name: String,
    pub signal: SignalId,
    pub kind: TransitionKind,
    pub input_places: Vec<PlaceId>,
    pub output_places: Vec<PlaceId>,
    pub useful: bool,
}

impl Transition {
    pub fn new(name: impl Into<String>, signal: SignalId, kind: TransitionKind) -> Self {
        Self {
            name: name.into(),
            signal,
            kind,
            input_places: Vec::new(),
            output_places: Vec::new(),
            useful: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Marking {
    pub state_code: StateCode,
    pub enabled: StateCode,
    pub marked_places: Vec<bool>,
    pub is_dummy: bool,
}

impl Marking {
    pub fn is_marked(&self, place: PlaceId) -> bool {
        self.marked_places.get(place.0).copied().unwrap_or(true)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub code: StateCode,
    pub markings: Vec<Marking>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    pub name: String,
    pub transitions: Vec<TransitionId>,
    pub places: Vec<PlaceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenFlowReport {
    pub status: AstgFlowStatus,
    pub redo_flow: bool,
    pub has_unique_state_coding: bool,
    pub selections: Vec<Selection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgFlowError {
    UnknownSignal(SignalId),
    UnknownPlace(PlaceId),
    UnknownTransition(TransitionId),
    TooManyStateSignals,
}

impl fmt::Display for AstgFlowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSignal(signal) => write!(formatter, "unknown ASTG signal {:?}", signal),
            Self::UnknownPlace(place) => write!(formatter, "unknown ASTG place {:?}", place),
            Self::UnknownTransition(transition) => {
                write!(formatter, "unknown ASTG transition {:?}", transition)
            }
            Self::TooManyStateSignals => formatter.write_str("too many ASTG state signals"),
        }
    }
}

impl Error for AstgFlowError {}

#[derive(Clone, Debug)]
pub struct AstgFlowGraph {
    pub signals: Vec<Signal>,
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub states: BTreeMap<StateCode, State>,
    pub flow_status: AstgFlowStatus,
    pub phase_adj: StateCode,
    change_count: u64,
    flow_change_count: Option<u64>,
    selections: Vec<Selection>,
}

impl AstgFlowGraph {
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
            places: Vec::new(),
            transitions: Vec::new(),
            states: BTreeMap::new(),
            flow_status: AstgFlowStatus::Ok,
            phase_adj: 0,
            change_count: 0,
            flow_change_count: None,
            selections: Vec::new(),
        }
    }

    pub fn add_signal(&mut self, name: impl Into<String>, kind: SignalKind) -> SignalId {
        let id = SignalId(self.signals.len());
        self.signals.push(Signal::new(name, kind));
        self.note_changed();
        id
    }

    pub fn add_place(&mut self, name: impl Into<String>, initial_token: bool) -> PlaceId {
        let id = PlaceId(self.places.len());
        self.places.push(Place::new(name, initial_token));
        self.note_changed();
        id
    }

    pub fn add_transition(
        &mut self,
        name: impl Into<String>,
        signal: SignalId,
        kind: TransitionKind,
    ) -> Result<TransitionId, AstgFlowError> {
        self.require_signal(signal)?;

        let id = TransitionId(self.transitions.len());
        self.transitions.push(Transition::new(name, signal, kind));
        self.note_changed();
        Ok(id)
    }

    pub fn add_input_place(
        &mut self,
        transition: TransitionId,
        place: PlaceId,
    ) -> Result<(), AstgFlowError> {
        self.require_place(place)?;
        let transition = self.transition_mut(transition)?;
        transition.input_places.push(place);
        self.note_changed();
        Ok(())
    }

    pub fn add_output_place(
        &mut self,
        transition: TransitionId,
        place: PlaceId,
    ) -> Result<(), AstgFlowError> {
        self.require_place(place)?;
        let transition = self.transition_mut(transition)?;
        transition.output_places.push(place);
        self.note_changed();
        Ok(())
    }

    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    pub fn token_flow(&mut self, force_safe: bool) -> Result<TokenFlowReport, AstgFlowError> {
        let redo_flow = self.reset_state_graph()?;
        self.selections.clear();

        if redo_flow {
            let initial = self.initial_marking();
            self.flow_state(initial, 0, force_safe);
        }

        let has_unique_state_coding = self.check_csc();
        self.check_redundant();

        Ok(TokenFlowReport {
            status: self.flow_status,
            redo_flow,
            has_unique_state_coding,
            selections: self.selections.clone(),
        })
    }

    fn note_changed(&mut self) {
        self.change_count = self.change_count.saturating_add(1);
    }

    fn reset_state_graph(&mut self) -> Result<bool, AstgFlowError> {
        if self.flow_change_count == Some(self.change_count) {
            return Ok(false);
        }

        self.flow_status = AstgFlowStatus::Ok;
        self.phase_adj = 0;
        self.states.clear();
        self.flow_change_count = Some(self.change_count);

        let mut state_bit = 1;
        for signal in &mut self.signals {
            if signal.kind.is_noninput() {
                signal.state_bit = state_bit;
                state_bit = next_state_bit(state_bit)?;
            } else {
                signal.state_bit = 0;
            }
        }

        for signal in &mut self.signals {
            if signal.kind.is_input() {
                signal.state_bit = state_bit;
                state_bit = next_state_bit(state_bit)?;
            }
        }

        for transition in &mut self.transitions {
            transition.useful = false;
        }

        for place in &mut self.places {
            place.useful = false;
        }

        Ok(true)
    }

    fn initial_marking(&self) -> Marking {
        Marking {
            state_code: 0,
            enabled: 0,
            marked_places: self
                .places
                .iter()
                .map(|place| place.initial_token)
                .collect(),
            is_dummy: false,
        }
    }

    fn flow_state(&mut self, marking: Marking, new_state: StateCode, force_safe: bool) {
        let enabled = self.check_new_state(marking.clone(), new_state, force_safe);
        for transition in enabled.into_iter().rev() {
            if self.flow_status != AstgFlowStatus::Ok {
                return;
            }

            let Some((next_marking, next_state)) = self.fire_marking(&marking, transition) else {
                continue;
            };

            self.flow_state(next_marking, next_state, force_safe);
        }
    }

    fn check_new_state(
        &mut self,
        marking: Marking,
        new_state: StateCode,
        force_safe: bool,
    ) -> Vec<TransitionId> {
        if self.flow_status != AstgFlowStatus::Ok {
            return Vec::new();
        }

        let adjusted_code = new_state ^ self.phase_adj;
        if self.states.get(&adjusted_code).is_some_and(|state| {
            state
                .markings
                .iter()
                .any(|seen| same_marking(seen, &marking))
        }) {
            return Vec::new();
        }

        let mut enabled = Vec::new();
        for transition_id in (0..self.transitions.len()).map(TransitionId) {
            let n_disabled = self.disabled_count(transition_id, &marking);
            if n_disabled == 0 {
                if force_safe && self.output_places_marked(transition_id, &marking) {
                    continue;
                }

                enabled.push(transition_id);
                self.transitions[transition_id.0].useful = true;
            } else if n_disabled == 1 {
                for place in self.transitions[transition_id.0].input_places.clone() {
                    if !marking.is_marked(place) {
                        self.places[place.0].useful = true;
                    }
                }
            }
        }

        let stored_marking = self.marking_with_enabled(marking);
        self.states
            .entry(adjusted_code)
            .or_insert_with(|| State {
                code: adjusted_code,
                markings: Vec::new(),
            })
            .markings
            .push(stored_marking);

        enabled
    }

    fn disabled_count(&self, transition: TransitionId, marking: &Marking) -> usize {
        self.transitions[transition.0]
            .input_places
            .iter()
            .filter(|place| !marking.is_marked(**place))
            .count()
    }

    fn output_places_marked(&self, transition: TransitionId, marking: &Marking) -> bool {
        self.transitions[transition.0]
            .output_places
            .iter()
            .any(|place| marking.is_marked(*place))
    }

    fn marking_with_enabled(&self, mut marking: Marking) -> Marking {
        let mut enabled = 0;
        let mut is_dummy = false;

        for transition in &self.transitions {
            if transition
                .input_places
                .iter()
                .all(|place| marking.is_marked(*place))
            {
                let signal = &self.signals[transition.signal.0];
                enabled |= signal.state_bit;
                if signal.kind == SignalKind::Dummy || transition.kind == TransitionKind::Dummy {
                    is_dummy = true;
                }
            }
        }

        marking.enabled = enabled;
        marking.is_dummy = is_dummy;
        marking
    }

    fn fire_marking(
        &mut self,
        marking: &Marking,
        transition_id: TransitionId,
    ) -> Option<(Marking, StateCode)> {
        let transition = &self.transitions[transition_id.0];
        let signal = &self.signals[transition.signal.0];
        let tmask = signal.state_bit;
        let new_state = marking.state_code ^ tmask;

        if matches!(
            transition.kind,
            TransitionKind::Positive | TransitionKind::Negative
        ) {
            let real_state = new_state ^ self.phase_adj;
            let should_be = transition.kind == TransitionKind::Positive;
            let is = real_state & tmask != 0;
            if should_be != is {
                if self.phase_adj & tmask != 0 {
                    self.flow_status = AstgFlowStatus::NotCsa;
                    self.selections.push(Selection {
                        name: "Inconsistent state assignment".to_owned(),
                        transitions: vec![transition_id],
                        places: Vec::new(),
                    });
                }

                self.phase_adj |= tmask;
            }
        }

        let mut next = marking.clone();
        for place in &transition.input_places {
            next.marked_places[place.0] = false;
        }

        next.state_code ^= tmask;
        for place in &transition.output_places {
            if next.marked_places[place.0] {
                self.flow_status = AstgFlowStatus::NotSafe;
                self.selections.push(Selection {
                    name: "unsafe place".to_owned(),
                    transitions: Vec::new(),
                    places: vec![*place],
                });
                return None;
            }

            next.marked_places[place.0] = true;
        }

        Some((next, new_state))
    }

    fn output_mask(&self) -> StateCode {
        self.transitions
            .iter()
            .map(|transition| &self.signals[transition.signal.0])
            .filter(|signal| signal.kind.is_noninput())
            .fold(0, |mask, signal| mask | signal.state_bit)
    }

    fn check_csc(&mut self) -> bool {
        if self.flow_status != AstgFlowStatus::Ok {
            return false;
        }

        let out_mask = self.output_mask();
        let mut has_unique_state_coding = true;

        for state in self.states.values() {
            let mut min_enabled = out_mask;
            let mut max_enabled = 0;

            for marking in &state.markings {
                if marking.is_dummy {
                    continue;
                }

                min_enabled &= marking.enabled;
                max_enabled |= marking.enabled;
            }

            max_enabled &= out_mask;
            if min_enabled != max_enabled {
                has_unique_state_coding = false;
                self.flow_status = AstgFlowStatus::NotUsc;
            }
        }

        has_unique_state_coding
    }

    fn check_redundant(&mut self) {
        if self.flow_status != AstgFlowStatus::Ok {
            return;
        }

        let transitions = self
            .transitions
            .iter()
            .enumerate()
            .filter_map(|(index, transition)| (!transition.useful).then_some(TransitionId(index)))
            .collect::<Vec<_>>();
        if !transitions.is_empty() {
            self.selections.push(Selection {
                name: "unfired transitions".to_owned(),
                transitions,
                places: Vec::new(),
            });
            self.flow_status = AstgFlowStatus::NotLive;
            return;
        }

        let places = self
            .places
            .iter()
            .enumerate()
            .filter_map(|(index, place)| (!place.useful).then_some(PlaceId(index)))
            .collect::<Vec<_>>();
        if !places.is_empty() {
            self.selections.push(Selection {
                name: "redundant places".to_owned(),
                transitions: Vec::new(),
                places,
            });
        }
    }

    fn require_signal(&self, signal: SignalId) -> Result<(), AstgFlowError> {
        if self.signals.get(signal.0).is_some() {
            Ok(())
        } else {
            Err(AstgFlowError::UnknownSignal(signal))
        }
    }

    fn require_place(&self, place: PlaceId) -> Result<(), AstgFlowError> {
        if self.places.get(place.0).is_some() {
            Ok(())
        } else {
            Err(AstgFlowError::UnknownPlace(place))
        }
    }

    fn transition_mut(
        &mut self,
        transition: TransitionId,
    ) -> Result<&mut Transition, AstgFlowError> {
        self.transitions
            .get_mut(transition.0)
            .ok_or(AstgFlowError::UnknownTransition(transition))
    }
}

impl Default for AstgFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

fn next_state_bit(state_bit: StateCode) -> Result<StateCode, AstgFlowError> {
    state_bit
        .checked_shl(1)
        .filter(|next| *next != 0)
        .ok_or(AstgFlowError::TooManyStateSignals)
}

fn same_marking(left: &Marking, right: &Marking) -> bool {
    left.state_code == right.state_code && left.marked_places == right.marked_places
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_transition_cycle() -> AstgFlowGraph {
        let mut graph = AstgFlowGraph::new();
        let signal = graph.add_signal("x", SignalKind::Output);
        let p0 = graph.add_place("p0", true);
        let p1 = graph.add_place("p1", false);
        let rise = graph
            .add_transition("x+", signal, TransitionKind::Positive)
            .unwrap();
        let fall = graph
            .add_transition("x-", signal, TransitionKind::Negative)
            .unwrap();

        graph.add_input_place(rise, p0).unwrap();
        graph.add_output_place(rise, p1).unwrap();
        graph.add_input_place(fall, p1).unwrap();
        graph.add_output_place(fall, p0).unwrap();

        graph
    }

    #[test]
    fn token_flow_reaches_the_cycle_markings() {
        let mut graph = two_transition_cycle();

        let report = graph.token_flow(false).unwrap();

        assert_eq!(report.status, AstgFlowStatus::Ok);
        assert!(report.redo_flow);
        assert_eq!(graph.state_count(), 2);
        assert!(graph.transitions.iter().all(|transition| transition.useful));
        assert!(graph.places.iter().all(|place| place.useful));
        assert_eq!(graph.states[&0].markings[0].enabled, 1);
        assert_eq!(graph.states[&1].markings[0].enabled, 1);
    }

    #[test]
    fn repeated_flow_reuses_current_state_graph() {
        let mut graph = two_transition_cycle();

        assert!(graph.token_flow(false).unwrap().redo_flow);
        assert!(!graph.token_flow(false).unwrap().redo_flow);
    }

    #[test]
    fn force_safe_suppresses_transitions_that_would_double_mark_outputs() {
        let mut graph = AstgFlowGraph::new();
        let signal = graph.add_signal("x", SignalKind::Output);
        let p0 = graph.add_place("p0", true);
        let p1 = graph.add_place("p1", true);
        let transition = graph
            .add_transition("x+", signal, TransitionKind::Positive)
            .unwrap();
        graph.add_input_place(transition, p0).unwrap();
        graph.add_output_place(transition, p1).unwrap();

        let report = graph.token_flow(true).unwrap();

        assert_eq!(report.status, AstgFlowStatus::NotLive);
        assert_eq!(report.selections[0].name, "unfired transitions");
        assert!(!graph.transitions[transition.0].useful);
    }

    #[test]
    fn unsafe_fire_reports_the_output_place() {
        let mut graph = AstgFlowGraph::new();
        let signal = graph.add_signal("x", SignalKind::Output);
        let p0 = graph.add_place("p0", true);
        let p1 = graph.add_place("p1", true);
        let transition = graph
            .add_transition("x+", signal, TransitionKind::Positive)
            .unwrap();
        graph.add_input_place(transition, p0).unwrap();
        graph.add_output_place(transition, p1).unwrap();

        let report = graph.token_flow(false).unwrap();

        assert_eq!(report.status, AstgFlowStatus::NotSafe);
        assert_eq!(report.selections[0].name, "unsafe place");
        assert_eq!(report.selections[0].places, vec![p1]);
    }

    #[test]
    fn csc_violation_is_detected_when_same_state_has_different_output_enables() {
        let mut graph = AstgFlowGraph::new();
        let output = graph.add_signal("x", SignalKind::Output);
        let input = graph.add_signal("a", SignalKind::Input);
        let p0 = graph.add_place("p0", true);
        let p1 = graph.add_place("p1", false);
        let p2 = graph.add_place("p2", false);
        let _p3 = graph.add_place("p3", false);
        let p4 = graph.add_place("p4", false);
        let a_rise = graph
            .add_transition("a+", input, TransitionKind::Positive)
            .unwrap();
        let a_fall = graph
            .add_transition("a-", input, TransitionKind::Negative)
            .unwrap();
        let x_rise = graph
            .add_transition("x+", output, TransitionKind::Positive)
            .unwrap();

        graph.add_input_place(a_rise, p0).unwrap();
        graph.add_output_place(a_rise, p2).unwrap();
        graph.add_input_place(a_fall, p2).unwrap();
        graph.add_output_place(a_fall, p1).unwrap();
        graph.add_input_place(x_rise, p1).unwrap();
        graph.add_output_place(x_rise, p4).unwrap();

        let report = graph.token_flow(false).unwrap();

        assert_eq!(report.status, AstgFlowStatus::NotUsc);
        assert!(!report.has_unique_state_coding);
        assert_eq!(graph.states[&0].markings.len(), 2);
    }

    #[test]
    fn unfired_transition_marks_graph_not_live() {
        let mut graph = two_transition_cycle();
        let signal = graph.add_signal("y", SignalKind::Output);
        let place = graph.add_place("never", false);
        let transition = graph
            .add_transition("y+", signal, TransitionKind::Positive)
            .unwrap();
        graph.add_input_place(transition, place).unwrap();

        let report = graph.token_flow(false).unwrap();

        assert_eq!(report.status, AstgFlowStatus::NotLive);
        assert_eq!(report.selections[0].name, "unfired transitions");
        assert!(report.selections[0].transitions.contains(&transition));
    }

    #[test]
    fn source_contains_no_legacy_c_exports() {
        let source = include_str!("astg_flow.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
