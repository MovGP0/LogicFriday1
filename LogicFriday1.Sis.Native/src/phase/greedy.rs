//! Native greedy phase optimization.
//!
//! The algorithms in this module operate on any phase state that can report its
//! current cost, choose the best unmarked inversion candidate, invert a
//! candidate, and manage temporary marks used by the Kernighan-Lin style pass.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

pub trait GreedyPhaseState: Clone {
    type Node: Copy + Eq;

    fn cost(&self) -> f64;

    fn best_node(&self) -> Option<Self::Node>;

    fn value(&self, node: Self::Node) -> f64;

    fn invert(&mut self, node: Self::Node);

    fn mark(&mut self, node: Self::Node);

    fn unmark_all(&mut self);
}

#[derive(Clone, Debug, PartialEq)]
pub struct GreedyDownReport {
    pub inversions: usize,
    pub initial_cost: f64,
    pub final_cost: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GoodPhaseReport {
    pub passes: usize,
    pub inversions: usize,
    pub initial_cost: f64,
    pub final_cost: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RandomAssignmentReport {
    pub attempt: usize,
    pub initial_cost: f64,
    pub final_cost: f64,
    pub inversions: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RandomGreedyReport<State> {
    pub best_state: State,
    pub assignments: Vec<RandomAssignmentReport>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GreedyPhaseError {
    InvalidAttemptCount,
    NonFiniteCost,
    NonFiniteValue,
    AssignmentFailed { attempt: usize, message: String },
}

impl fmt::Display for GreedyPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAttemptCount => write!(f, "attempt count must be greater than zero"),
            Self::NonFiniteCost => write!(f, "phase cost must be finite"),
            Self::NonFiniteValue => write!(f, "phase value must be finite"),
            Self::AssignmentFailed { attempt, message } => {
                write!(f, "phase assignment {attempt} failed: {message}")
            }
        }
    }
}

impl Error for GreedyPhaseError {}

pub type GreedyPhaseResult<T> = Result<T, GreedyPhaseError>;

pub fn phase_quick<State>(state: &mut State) -> GreedyPhaseResult<GreedyDownReport>
where
    State: GreedyPhaseState,
{
    greedy_down(state)
}

pub fn phase_good<State>(state: &mut State) -> GreedyPhaseResult<GoodPhaseReport>
where
    State: GreedyPhaseState,
{
    let initial_cost = finite_cost(state.cost())?;
    let mut passes = 0usize;
    let mut inversions = 0usize;

    loop {
        let report = greedy_down(state)?;
        inversions += report.inversions;
        let up_report = kl_up(state)?;
        passes += 1;
        inversions += up_report.inversions;

        if !up_report.improved {
            break;
        }
    }

    Ok(GoodPhaseReport {
        passes,
        inversions,
        initial_cost,
        final_cost: finite_cost(state.cost())?,
    })
}

pub fn phase_random_greedy<State, Assign, AssignError>(
    state: &mut State,
    attempts: usize,
    mut assign: Assign,
) -> GreedyPhaseResult<RandomGreedyReport<State>>
where
    State: GreedyPhaseState,
    Assign: FnMut(usize, &mut State) -> Result<(), AssignError>,
    AssignError: fmt::Display,
{
    if attempts == 0 {
        return Err(GreedyPhaseError::InvalidAttemptCount);
    }

    let mut best_state = state.clone();
    let mut assignments = Vec::with_capacity(attempts);

    for attempt in 0..attempts {
        assign(attempt, state).map_err(|error| GreedyPhaseError::AssignmentFailed {
            attempt: attempt + 1,
            message: error.to_string(),
        })?;

        let initial_cost = finite_cost(state.cost())?;
        let down_report = greedy_down(state)?;
        let final_cost = finite_cost(state.cost())?;

        assignments.push(RandomAssignmentReport {
            attempt: attempt + 1,
            initial_cost,
            final_cost,
            inversions: down_report.inversions,
        });

        if final_cost < finite_cost(best_state.cost())? {
            best_state = state.clone();
        }
    }

    *state = best_state.clone();

    Ok(RandomGreedyReport {
        best_state,
        assignments,
    })
}

pub fn greedy_down<State>(state: &mut State) -> GreedyPhaseResult<GreedyDownReport>
where
    State: GreedyPhaseState,
{
    let initial_cost = finite_cost(state.cost())?;
    let mut inversions = 0usize;

    loop {
        let Some(node) = state.best_node() else {
            break;
        };
        let value = finite_value(state.value(node))?;

        if value <= 0.0 {
            break;
        }

        state.invert(node);
        inversions += 1;
        finite_cost(state.cost())?;
    }

    Ok(GreedyDownReport {
        inversions,
        initial_cost,
        final_cost: finite_cost(state.cost())?,
    })
}

pub fn kl_up<State>(state: &mut State) -> GreedyPhaseResult<KlUpReport>
where
    State: GreedyPhaseState,
{
    let best_state = state.clone();
    let best_cost = finite_cost(best_state.cost())?;
    let mut inversions = 0usize;

    loop {
        let Some(node) = state.best_node() else {
            *state = best_state;
            state.unmark_all();
            return Ok(KlUpReport {
                improved: false,
                inversions,
            });
        };

        finite_value(state.value(node))?;
        state.invert(node);
        state.mark(node);
        inversions += 1;

        if finite_cost(state.cost())? < best_cost {
            state.unmark_all();
            return Ok(KlUpReport {
                improved: true,
                inversions,
            });
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KlUpReport {
    pub improved: bool,
    pub inversions: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PhaseNodeId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseCandidate {
    normal_value: f64,
    inverted_value: f64,
    invertible: bool,
    marked: bool,
    inverted: bool,
}

impl PhaseCandidate {
    pub fn new(normal_value: f64, inverted_value: f64) -> Self {
        Self {
            normal_value,
            inverted_value,
            invertible: true,
            marked: false,
            inverted: false,
        }
    }

    pub fn non_invertible() -> Self {
        Self {
            normal_value: 0.0,
            inverted_value: 0.0,
            invertible: false,
            marked: false,
            inverted: false,
        }
    }

    pub fn value(&self) -> f64 {
        if self.inverted {
            self.inverted_value
        } else {
            self.normal_value
        }
    }

    pub fn is_inverted(&self) -> bool {
        self.inverted
    }

    pub fn is_marked(&self) -> bool {
        self.marked
    }

    pub fn is_invertible(&self) -> bool {
        self.invertible
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseState {
    cost: f64,
    candidates: Vec<PhaseCandidate>,
}

impl PhaseState {
    pub fn new(cost: f64, candidates: impl Into<Vec<PhaseCandidate>>) -> Self {
        Self {
            cost,
            candidates: candidates.into(),
        }
    }

    pub fn candidates(&self) -> &[PhaseCandidate] {
        &self.candidates
    }

    pub fn candidate(&self, node: PhaseNodeId) -> Option<&PhaseCandidate> {
        self.candidates.get(node.0)
    }

    pub fn toggle(&mut self, node: PhaseNodeId) {
        self.invert(node);
    }
}

impl GreedyPhaseState for PhaseState {
    type Node = PhaseNodeId;

    fn cost(&self) -> f64 {
        self.cost
    }

    fn best_node(&self) -> Option<Self::Node> {
        self.candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate.invertible && !candidate.marked)
            .max_by(|left, right| compare_values(left.1.value(), right.1.value()))
            .map(|(index, _)| PhaseNodeId(index))
    }

    fn value(&self, node: Self::Node) -> f64 {
        self.candidates
            .get(node.0)
            .map(PhaseCandidate::value)
            .unwrap_or(f64::NAN)
    }

    fn invert(&mut self, node: Self::Node) {
        if let Some(candidate) = self.candidates.get_mut(node.0) {
            if candidate.invertible {
                let value = candidate.value();
                self.cost -= value;
                candidate.inverted = !candidate.inverted;
            }
        }
    }

    fn mark(&mut self, node: Self::Node) {
        if let Some(candidate) = self.candidates.get_mut(node.0) {
            candidate.marked = true;
        }
    }

    fn unmark_all(&mut self) {
        for candidate in &mut self.candidates {
            candidate.marked = false;
        }
    }
}

fn compare_values(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Less)
}

fn finite_cost(cost: f64) -> GreedyPhaseResult<f64> {
    if cost.is_finite() {
        Ok(cost)
    } else {
        Err(GreedyPhaseError::NonFiniteCost)
    }
}

fn finite_value(value: f64) -> GreedyPhaseResult<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(GreedyPhaseError::NonFiniteValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_down_inverts_positive_best_candidates_until_no_gain_remains() {
        let mut state = PhaseState::new(
            10.0,
            vec![
                PhaseCandidate::new(2.0, -2.0),
                PhaseCandidate::new(4.0, -4.0),
                PhaseCandidate::new(0.0, 0.0),
            ],
        );

        let report = greedy_down(&mut state).unwrap();

        assert_eq!(report.inversions, 2);
        assert_eq!(state.cost(), 4.0);
        assert!(state.candidate(PhaseNodeId(0)).unwrap().is_inverted());
        assert!(state.candidate(PhaseNodeId(1)).unwrap().is_inverted());
        assert!(!state.candidate(PhaseNodeId(2)).unwrap().is_inverted());
    }

    #[test]
    fn kl_up_rolls_back_when_all_one_time_flips_fail_to_improve_cost() {
        let mut state = PhaseState::new(
            5.0,
            vec![
                PhaseCandidate::new(-1.0, 1.0),
                PhaseCandidate::new(-2.0, 2.0),
            ],
        );

        let report = kl_up(&mut state).unwrap();

        assert_eq!(
            report,
            KlUpReport {
                improved: false,
                inversions: 2
            }
        );
        assert_eq!(state.cost(), 5.0);
        assert!(!state.candidate(PhaseNodeId(0)).unwrap().is_inverted());
        assert!(!state.candidate(PhaseNodeId(1)).unwrap().is_inverted());
        assert!(!state.candidate(PhaseNodeId(0)).unwrap().is_marked());
        assert!(!state.candidate(PhaseNodeId(1)).unwrap().is_marked());
    }

    #[test]
    fn kl_up_keeps_first_prefix_that_beats_original_cost() {
        let mut state = PhaseState::new(
            5.0,
            vec![
                PhaseCandidate::new(3.0, -3.0),
                PhaseCandidate::new(-2.0, 6.0),
            ],
        );

        let report = kl_up(&mut state).unwrap();

        assert_eq!(
            report,
            KlUpReport {
                improved: true,
                inversions: 1
            }
        );
        assert_eq!(state.cost(), 2.0);
        assert!(state.candidate(PhaseNodeId(0)).unwrap().is_inverted());
        assert!(!state.candidate(PhaseNodeId(1)).unwrap().is_inverted());
        assert!(!state.candidate(PhaseNodeId(0)).unwrap().is_marked());
        assert!(!state.candidate(PhaseNodeId(1)).unwrap().is_marked());
    }

    #[test]
    fn phase_good_repeats_greedy_down_and_kl_up_until_no_improvement() {
        let mut state = PhaseState::new(
            12.0,
            vec![
                PhaseCandidate::new(3.0, -3.0),
                PhaseCandidate::new(-2.0, 4.0),
                PhaseCandidate::new(1.0, -1.0),
            ],
        );

        let report = phase_good(&mut state).unwrap();

        assert_eq!(report.passes, 1);
        assert_eq!(report.initial_cost, 12.0);
        assert_eq!(report.final_cost, 8.0);
        assert!(state.candidate(PhaseNodeId(0)).unwrap().is_inverted());
        assert!(!state.candidate(PhaseNodeId(1)).unwrap().is_inverted());
        assert!(state.candidate(PhaseNodeId(2)).unwrap().is_inverted());
    }

    #[test]
    fn random_greedy_retains_best_assignment() {
        let mut state = PhaseState::new(
            10.0,
            vec![
                PhaseCandidate::new(-1.0, 4.0),
                PhaseCandidate::new(-3.0, 8.0),
            ],
        );

        let report = phase_random_greedy(&mut state, 2, |attempt, state| {
            if attempt == 0 {
                state.toggle(PhaseNodeId(0));
            } else {
                state.toggle(PhaseNodeId(1));
            }

            Ok::<_, std::convert::Infallible>(())
        })
        .unwrap();

        assert_eq!(report.assignments.len(), 2);
        assert_eq!(state.cost(), 2.0);
        assert!(!state.candidate(PhaseNodeId(0)).unwrap().is_inverted());
        assert!(!state.candidate(PhaseNodeId(1)).unwrap().is_inverted());
    }

    #[test]
    fn non_finite_diagnostics_are_generic() {
        let mut state = PhaseState::new(f64::NAN, vec![PhaseCandidate::new(1.0, -1.0)]);

        assert_eq!(
            greedy_down(&mut state),
            Err(GreedyPhaseError::NonFiniteCost)
        );

        let mut state = PhaseState::new(1.0, vec![PhaseCandidate::new(f64::NAN, 0.0)]);

        assert_eq!(
            greedy_down(&mut state),
            Err(GreedyPhaseError::NonFiniteValue)
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present_in_this_port() {
        let source = include_str!("greedy.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday", "1-")));
    }
}
