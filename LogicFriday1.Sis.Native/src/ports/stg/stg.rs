//! Native Rust port of `sis/stg/stg.c`.
//!
//! The C implementation stores STG data in generic SIS `graph_t` slots and uses
//! `array_t` plus `lsList` iteration for names and graph walks. This native port
//! keeps the STG behavior in explicit Rust data structures. Import/export of the
//! legacy SIS graph representation remains blocked until the graph/list/array
//! ports exist.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StateId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TransitionId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct StgClock {
    pub name: String,
    pub cycle_time: f64,
    pub nominal_rise: f64,
    pub nominal_fall: f64,
    pub min_rise: f64,
    pub min_fall: f64,
    pub max_rise: f64,
    pub max_fall: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub name: Option<String>,
    pub encoding: Option<String>,
}

impl State {
    pub fn new(name: Option<impl Into<String>>, encoding: Option<impl Into<String>>) -> Self {
        Self {
            name: name.map(Into::into),
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stg {
    states: Vec<State>,
    transitions: Vec<Transition>,
    start: Option<StateId>,
    current: Option<StateId>,
    input_names: Option<Vec<String>>,
    output_names: Option<Vec<String>>,
    clock_data: Option<StgClock>,
    edge_index: usize,
    num_inputs: usize,
    num_outputs: usize,
}

impl Stg {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_dimensions(num_inputs: usize, num_outputs: usize) -> Self {
        Self {
            num_inputs,
            num_outputs,
            ..Self::default()
        }
    }

    pub fn states(&self) -> &[State] {
        &self.states
    }

    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    pub fn start(&self) -> Option<StateId> {
        self.start
    }

    pub fn current(&self) -> Option<StateId> {
        self.current
    }

    pub fn num_inputs(&self) -> usize {
        self.num_inputs
    }

    pub fn set_num_inputs(&mut self, num_inputs: usize) {
        self.num_inputs = num_inputs;
    }

    pub fn num_outputs(&self) -> usize {
        self.num_outputs
    }

    pub fn set_num_outputs(&mut self, num_outputs: usize) {
        self.num_outputs = num_outputs;
    }

    pub fn num_products(&self) -> usize {
        self.transitions.len()
    }

    pub fn num_states(&self) -> usize {
        self.states.len()
    }

    pub fn edge_index(&self) -> usize {
        self.edge_index
    }

    pub fn set_edge_index(&mut self, edge_index: usize) {
        self.edge_index = edge_index;
    }

    pub fn input_names(&self) -> Option<&[String]> {
        self.input_names.as_deref()
    }

    pub fn output_names(&self) -> Option<&[String]> {
        self.output_names.as_deref()
    }

    pub fn set_input_names(&mut self, names: Option<Vec<String>>) {
        self.input_names = names;
    }

    pub fn set_output_names(&mut self, names: Option<Vec<String>>) {
        self.output_names = names;
    }

    pub fn clock_data(&self) -> Option<&StgClock> {
        self.clock_data.as_ref()
    }

    pub fn set_clock_data(&mut self, clock_data: Option<StgClock>) {
        self.clock_data = clock_data;
    }

    pub fn create_state(
        &mut self,
        name: Option<impl Into<String>>,
        encoding: Option<impl Into<String>>,
    ) -> StateId {
        let id = StateId(self.states.len());
        self.states.push(State::new(name, encoding));
        id
    }

    pub fn set_start(&mut self, state: StateId) -> Result<(), StgError> {
        self.require_state(state)?;
        self.start = Some(state);
        Ok(())
    }

    pub fn set_current(&mut self, state: StateId) -> Result<(), StgError> {
        self.require_state(state)?;
        self.current = Some(state);
        Ok(())
    }

    pub fn reset(&mut self) {
        self.current = self.start;
    }

    pub fn state_name(&self, state: StateId) -> Option<&str> {
        self.states
            .get(state.0)
            .and_then(|state| state.name.as_deref())
    }

    pub fn state_encoding(&self, state: StateId) -> Option<&str> {
        self.states
            .get(state.0)
            .and_then(|state| state.encoding.as_deref())
    }

    pub fn set_state_name(
        &mut self,
        state: StateId,
        name: impl Into<String>,
    ) -> Result<(), StgError> {
        self.require_state(state)?;
        self.states[state.0].name = Some(name.into());
        Ok(())
    }

    pub fn set_state_encoding(
        &mut self,
        state: StateId,
        encoding: impl Into<String>,
    ) -> Result<(), StgError> {
        self.require_state(state)?;
        self.states[state.0].encoding = Some(encoding.into());
        Ok(())
    }

    pub fn state_by_name(&self, name: &str) -> Option<StateId> {
        self.states
            .iter()
            .position(|state| state.name.as_deref() == Some(name))
            .map(StateId)
    }

    pub fn state_by_encoding(&self, encoding: &str) -> Option<StateId> {
        self.states
            .iter()
            .position(|state| state.encoding.as_deref() == Some(encoding))
            .map(StateId)
    }

    pub fn create_transition(
        &mut self,
        from: StateId,
        to: StateId,
        input: impl Into<String>,
        output: impl Into<String>,
    ) -> Result<TransitionId, StgError> {
        self.require_state(from)?;
        self.require_state(to)?;

        let input = input.into();
        let output = output.into();
        if input.len() != self.num_inputs {
            return Err(StgError::InvalidInputLength {
                expected: self.num_inputs,
                actual: input.len(),
            });
        }
        if output.len() != self.num_outputs {
            return Err(StgError::InvalidOutputLength {
                expected: self.num_outputs,
                actual: output.len(),
            });
        }

        if let Some(existing) = self.transitions.iter().find(|transition| {
            transition.from == from
                && transition.to != to
                && equiv_transition(&transition.input, &input)
        }) {
            return Err(StgError::TransitionToDifferentState {
                from,
                existing_to: existing.to,
                new_to: to,
                input,
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

    pub fn check(&mut self) -> StgCheckReport {
        let mut report = StgCheckReport::default();
        let Some(start) = self.start else {
            report
                .fatal_errors
                .push(StgCheckIssue::NoStartStateSpecified);
            return report;
        };

        let code_length = self.state_encoding(start).map(str::len).unwrap_or(0);

        if self.current.is_none() {
            report.warnings.push(StgCheckIssue::NoCurrentStateSpecified);
            self.current = Some(start);
        }

        if let Some(names) = &self.input_names {
            if names.len() != self.num_inputs {
                report
                    .warnings
                    .push(StgCheckIssue::IncorrectInputNameCount {
                        expected: self.num_inputs,
                        actual: names.len(),
                    });
            }
        }

        if let Some(names) = &self.output_names {
            if names.len() != self.num_outputs {
                report
                    .warnings
                    .push(StgCheckIssue::IncorrectOutputNameCount {
                        expected: self.num_outputs,
                        actual: names.len(),
                    });
            }
        }

        let any_state = self
            .state_by_name("ANY")
            .or_else(|| self.state_by_name("*"));
        let any_transitions = any_state
            .map(|state| self.outgoing_transition_ids(state))
            .unwrap_or_default();

        for state_id in (0..self.states.len()).map(StateId) {
            let state_len = self.state_encoding(state_id).map(str::len).unwrap_or(0);
            if state_len != code_length {
                report
                    .fatal_errors
                    .push(StgCheckIssue::WrongEncodingLength {
                        state: state_id,
                        expected: code_length,
                        actual: state_len,
                    });
                return report;
            }

            if state_id != start && self.incoming_transition_ids(state_id).is_empty() {
                if Some(state_id) != any_state {
                    report
                        .warnings
                        .push(StgCheckIssue::UnreachableState { state: state_id });
                }
            }

            let mut effective_transitions = self.outgoing_transition_ids(state_id);
            effective_transitions.extend(any_transitions.iter().copied());

            match effective_transitions.len() {
                0 => report
                    .warnings
                    .push(StgCheckIssue::StateDoesNotFanout { state: state_id }),
                1 => {}
                _ => {
                    if let Some(issue) =
                        self.first_nondeterministic_pair(state_id, &effective_transitions)
                    {
                        report.fatal_errors.push(issue);
                        return report;
                    }
                }
            }
        }

        report
    }

    pub fn simulate(&mut self, vector: &str) -> Result<SimulationStep, StgError> {
        let current = self.current.ok_or(StgError::MissingCurrentState)?;
        self.require_state(current)?;

        let transition_id = self
            .outgoing_transition_ids(current)
            .into_iter()
            .find(|id| equiv_transition(&self.transitions[id.0].input, vector))
            .ok_or_else(|| StgError::UndeterminableNextState {
                current,
                vector: vector.to_owned(),
            })?;

        let transition = self.transitions[transition_id.0].clone();
        self.current = Some(transition.to);

        Ok(SimulationStep {
            transition: transition_id,
            input: transition.input,
            from: current,
            from_name: self.state_name(current).map(str::to_owned),
            to: transition.to,
            to_name: self.state_name(transition.to).map(str::to_owned),
            output: transition.output,
        })
    }

    pub fn import_from_sis_graph() -> Result<Self, StgError> {
        Err(StgError::MissingSisGraphPort)
    }

    pub fn export_to_sis_graph(&self) -> Result<(), StgError> {
        Err(StgError::MissingSisGraphPort)
    }

    fn require_state(&self, state: StateId) -> Result<(), StgError> {
        if state.0 < self.states.len() {
            Ok(())
        } else {
            Err(StgError::UnknownState(state))
        }
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

    fn outgoing_transition_ids(&self, state: StateId) -> Vec<TransitionId> {
        self.transitions
            .iter()
            .enumerate()
            .filter_map(|(index, transition)| {
                (transition.from == state).then_some(TransitionId(index))
            })
            .collect()
    }

    fn first_nondeterministic_pair(
        &self,
        state: StateId,
        transition_ids: &[TransitionId],
    ) -> Option<StgCheckIssue> {
        for (right_index, right_id) in transition_ids.iter().enumerate() {
            let right = &self.transitions[right_id.0];
            for left_id in &transition_ids[..right_index] {
                let left = &self.transitions[left_id.0];
                if equiv_transition(&right.input, &left.input)
                    && (right.output != left.output || right.to != left.to)
                {
                    return Some(StgCheckIssue::MachineIsNotDeterministic {
                        state,
                        first: *left_id,
                        second: *right_id,
                    });
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StgCheckReport {
    pub warnings: Vec<StgCheckIssue>,
    pub fatal_errors: Vec<StgCheckIssue>,
}

impl StgCheckReport {
    pub fn is_ok(&self) -> bool {
        self.fatal_errors.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StgCheckIssue {
    NoStartStateSpecified,
    NoCurrentStateSpecified,
    IncorrectInputNameCount {
        expected: usize,
        actual: usize,
    },
    IncorrectOutputNameCount {
        expected: usize,
        actual: usize,
    },
    WrongEncodingLength {
        state: StateId,
        expected: usize,
        actual: usize,
    },
    UnreachableState {
        state: StateId,
    },
    StateDoesNotFanout {
        state: StateId,
    },
    MachineIsNotDeterministic {
        state: StateId,
        first: TransitionId,
        second: TransitionId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationStep {
    pub transition: TransitionId,
    pub input: String,
    pub from: StateId,
    pub from_name: Option<String>,
    pub to: StateId,
    pub to_name: Option<String>,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StgError {
    MissingSisGraphPort,
    UnknownState(StateId),
    MissingCurrentState,
    InvalidInputLength {
        expected: usize,
        actual: usize,
    },
    InvalidOutputLength {
        expected: usize,
        actual: usize,
    },
    TransitionToDifferentState {
        from: StateId,
        existing_to: StateId,
        new_to: StateId,
        input: String,
    },
    UndeterminableNextState {
        current: StateId,
        vector: String,
    },
}

impl fmt::Display for StgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisGraphPort => write!(
                f,
                "legacy SIS graph/list/array-backed STG representation is not ported to Rust yet"
            ),
            Self::UnknownState(state) => write!(f, "unknown STG state {:?}", state),
            Self::MissingCurrentState => write!(f, "STG has no current state"),
            Self::InvalidInputLength { expected, actual } => {
                write!(
                    f,
                    "invalid transition input length: expected {expected}, got {actual}"
                )
            }
            Self::InvalidOutputLength { expected, actual } => {
                write!(
                    f,
                    "invalid transition output length: expected {expected}, got {actual}"
                )
            }
            Self::TransitionToDifferentState {
                from,
                existing_to,
                new_to,
                input,
            } => write!(
                f,
                "transition from {:?} on input {input} conflicts: existing destination {:?}, new destination {:?}",
                from, existing_to, new_to
            ),
            Self::UndeterminableNextState { current, vector } => {
                write!(
                    f,
                    "next state is undeterminable from {:?} for input {vector}",
                    current
                )
            }
        }
    }
}

impl Error for StgError {}

pub fn equiv_transition(left: &str, right: &str) -> bool {
    let mut right_chars = right.chars();
    for left in left.chars() {
        let right = right_chars.next().unwrap_or('\0');
        if left != '-' && right != '-' && left != right {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_transition_equivalence_matches_stg_c() {
        assert!(equiv_transition("1-0", "110"));
        assert!(equiv_transition("---", "010"));
        assert!(equiv_transition("010", "0-0"));
        assert!(!equiv_transition("010", "011"));
        assert!(equiv_transition("01", "010"));
        assert!(!equiv_transition("010", "01"));
    }

    #[test]
    fn create_transition_rejects_equivalent_input_to_different_state() {
        let mut stg = Stg::with_dimensions(2, 1);
        let s0 = stg.create_state(Some("s0"), Some("0"));
        let s1 = stg.create_state(Some("s1"), Some("1"));
        let s2 = stg.create_state(Some("s2"), Some("1"));

        stg.create_transition(s0, s1, "1-", "0").unwrap();

        assert_eq!(
            stg.create_transition(s0, s2, "10", "1"),
            Err(StgError::TransitionToDifferentState {
                from: s0,
                existing_to: s1,
                new_to: s2,
                input: "10".to_owned()
            })
        );
    }

    #[test]
    fn duplicate_equivalent_transition_to_same_state_is_left_for_check() {
        let mut stg = Stg::with_dimensions(2, 1);
        let s0 = stg.create_state(Some("s0"), Some("0"));
        let s1 = stg.create_state(Some("s1"), Some("1"));
        stg.set_start(s0).unwrap();
        stg.set_current(s0).unwrap();

        let first = stg.create_transition(s0, s1, "1-", "0").unwrap();
        let second = stg.create_transition(s0, s1, "10", "1").unwrap();

        let report = stg.check();
        assert_eq!(
            report.fatal_errors,
            vec![StgCheckIssue::MachineIsNotDeterministic {
                state: s0,
                first,
                second
            }]
        );
    }

    #[test]
    fn check_sets_missing_current_to_start_and_reports_name_count_warnings() {
        let mut stg = Stg::with_dimensions(2, 1);
        let s0 = stg.create_state(Some("s0"), Some("0"));
        stg.set_start(s0).unwrap();
        stg.set_input_names(Some(vec!["a".to_owned()]));
        stg.set_output_names(Some(vec!["z".to_owned(), "extra".to_owned()]));

        let report = stg.check();

        assert!(report.is_ok());
        assert_eq!(stg.current(), Some(s0));
        assert_eq!(
            report.warnings,
            vec![
                StgCheckIssue::NoCurrentStateSpecified,
                StgCheckIssue::IncorrectInputNameCount {
                    expected: 2,
                    actual: 1
                },
                StgCheckIssue::IncorrectOutputNameCount {
                    expected: 1,
                    actual: 2
                },
                StgCheckIssue::StateDoesNotFanout { state: s0 }
            ]
        );
    }

    #[test]
    fn any_state_edges_participate_in_determinism_checks() {
        let mut stg = Stg::with_dimensions(1, 1);
        let s0 = stg.create_state(Some("s0"), Some("0"));
        let s1 = stg.create_state(Some("s1"), Some("1"));
        let any = stg.create_state(Some("ANY"), Some("1"));
        stg.set_start(s0).unwrap();
        stg.set_current(s0).unwrap();

        let first = stg.create_transition(s0, s0, "1", "0").unwrap();
        let any_edge = stg.create_transition(any, s1, "-", "0").unwrap();

        let report = stg.check();

        assert_eq!(
            report.fatal_errors,
            vec![StgCheckIssue::MachineIsNotDeterministic {
                state: s0,
                first,
                second: any_edge
            }]
        );
    }

    #[test]
    fn reset_and_simulate_update_current_state() {
        let mut stg = Stg::with_dimensions(2, 1);
        let s0 = stg.create_state(Some("s0"), Some("0"));
        let s1 = stg.create_state(Some("s1"), Some("1"));
        stg.set_start(s0).unwrap();
        stg.set_current(s1).unwrap();
        stg.create_transition(s0, s1, "0-", "1").unwrap();

        stg.reset();
        let step = stg.simulate("01").unwrap();

        assert_eq!(step.from, s0);
        assert_eq!(step.from_name.as_deref(), Some("s0"));
        assert_eq!(step.to, s1);
        assert_eq!(step.to_name.as_deref(), Some("s1"));
        assert_eq!(step.output, "1");
        assert_eq!(stg.current(), Some(s1));
    }

    #[test]
    fn wrong_encoding_length_is_fatal() {
        let mut stg = Stg::with_dimensions(1, 1);
        let s0 = stg.create_state(Some("s0"), Some("00"));
        let s1 = stg.create_state(Some("s1"), Some("1"));
        stg.set_start(s0).unwrap();

        let report = stg.check();

        assert_eq!(
            report.fatal_errors,
            vec![StgCheckIssue::WrongEncodingLength {
                state: s1,
                expected: 2,
                actual: 1
            }]
        );
    }

    #[test]
    fn sis_graph_import_export_report_blocked_dependency() {
        assert_eq!(
            Stg::import_from_sis_graph(),
            Err(StgError::MissingSisGraphPort)
        );
        assert_eq!(
            Stg::new().export_to_sis_graph(),
            Err(StgError::MissingSisGraphPort)
        );
    }
}
