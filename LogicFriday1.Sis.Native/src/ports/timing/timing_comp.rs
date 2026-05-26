//! Scaffold for `LogicSynthesis/sis/timing/timing_comp.c`.
//!
//! The C unit computes optimal clocking constraints over SIS `graph_t`
//! latch/constraint graphs and writes the solved clock positions back through
//! `network_t` clock APIs. Those graph, latch, clock, and timing utility
//! dependencies are still C-only in this repository, so this file records the
//! native Rust API shape without adding per-file legacy C ABI exports.

use std::error::Error;
use std::fmt;

pub const EPS: f64 = 1.0e-3;
pub const EPS1: f64 = 1.0e-5;
pub const EPS2: f64 = 1.0e-2;
pub const INFINITY: f64 = 10_000.0;
pub const NOT_SET: i32 = -2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockConstraintMode {
    Feasible,
    Solve,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExteriorPathStatus {
    NegativeCycle,
    Infeasible,
    AllPositiveCycles,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingCompDependency {
    TimingGraph,
    TimingUtilities,
    NetworkClockModel,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TimingCompError {
    dependency: TimingCompDependency,
}

impl TimingCompError {
    pub const fn missing(dependency: TimingCompDependency) -> Self {
        Self { dependency }
    }

    pub const fn dependency(&self) -> TimingCompDependency {
        self.dependency
    }
}

impl fmt::Display for TimingCompError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.dependency {
            TimingCompDependency::TimingGraph => {
                write!(f, "SIS timing graph types are not ported to Rust yet")
            }
            TimingCompDependency::TimingUtilities => {
                write!(f, "SIS timing utility routines are not ported to Rust yet")
            }
            TimingCompDependency::NetworkClockModel => {
                write!(
                    f,
                    "SIS network clock update APIs are not ported to Rust yet"
                )
            }
        }
    }
}

impl Error for TimingCompError {}

#[derive(Clone, Debug, PartialEq)]
pub struct OptimalClockResult {
    pub cycle_time: f64,
}

/// Opaque placeholder for the latch graph currently represented by SIS
/// `graph_t` vertices and `l_node_t`/`l_edge_t` user data.
#[derive(Debug)]
pub struct LatchGraph {
    _private: (),
}

/// Opaque placeholder for the generated clock constraint graph currently
/// represented by SIS `graph_t`, `c_graph_t`, `c_node_t`, and `c_edge_t`.
#[derive(Debug)]
pub struct ConstraintGraph {
    _private: (),
}

/// Opaque placeholder for the SIS network clock model updated by
/// `clock_set_cycletime` and `clock_set_parameter`.
#[derive(Debug)]
pub struct ClockNetwork {
    _private: (),
}

/// Compute optimal clocking parameters for a SIS latch graph and clock network.
///
/// The C implementation orchestrates lower-bound search, constraint graph
/// construction, short-path/model/phase-separation constraints, feasibility or
/// general min/max solving, and network clock updates. Native Rust cannot
/// implement that behavior until the direct `timing_int.h` graph and utility
/// dependencies have Rust equivalents.
pub fn compute_optimal_clock(
    _latch_graph: &LatchGraph,
    _network: &mut ClockNetwork,
) -> Result<OptimalClockResult, TimingCompError> {
    Err(TimingCompError::missing(TimingCompDependency::TimingGraph))
}

/// Placeholder for C `tmg_clock_lower_bound`.
pub fn clock_lower_bound(_graph: &LatchGraph) -> Result<f64, TimingCompError> {
    Err(TimingCompError::missing(TimingCompDependency::TimingGraph))
}

/// Placeholder for C `tmg_guess_clock_bound`.
pub fn guess_clock_bound(_graph: &LatchGraph) -> Result<f64, TimingCompError> {
    Err(TimingCompError::missing(TimingCompDependency::TimingGraph))
}

/// Placeholder for C `tmg_all_negative_cycles`.
pub fn all_negative_cycles(
    _graph: &mut LatchGraph,
    _cycle_time: f64,
) -> Result<bool, TimingCompError> {
    Err(TimingCompError::missing(TimingCompDependency::TimingGraph))
}

/// Placeholder for C `tmg_construct_clock_graph`.
pub fn construct_clock_graph(
    _latch_graph: &LatchGraph,
    _clock_lower_bound: f64,
) -> Result<ConstraintGraph, TimingCompError> {
    Err(TimingCompError::missing(
        TimingCompDependency::TimingUtilities,
    ))
}

/// Placeholder for C `tmg_add_short_path_constraints`.
pub fn add_short_path_constraints(
    _clock_graph: &mut ConstraintGraph,
    _latch_graph: &LatchGraph,
) -> Result<(), TimingCompError> {
    Err(TimingCompError::missing(
        TimingCompDependency::TimingUtilities,
    ))
}

/// Placeholder for C `tmg_solve_constraints`.
pub fn solve_constraints(
    _clock_graph: &mut ConstraintGraph,
    _clock_lower_bound: f64,
) -> Result<f64, TimingCompError> {
    Err(TimingCompError::missing(
        TimingCompDependency::TimingUtilities,
    ))
}

/// Placeholder for C `tmg_is_feasible`.
pub fn is_feasible(
    _clock_graph: &mut ConstraintGraph,
    _cycle_time: f64,
) -> Result<bool, TimingCompError> {
    Err(TimingCompError::missing(
        TimingCompDependency::TimingUtilities,
    ))
}

/// Placeholder for C `tmg_solve_gen_constraints`.
pub fn solve_general_constraints(
    _clock_graph: &mut ConstraintGraph,
    _clock_lower_bound: f64,
) -> Result<f64, TimingCompError> {
    Err(TimingCompError::missing(
        TimingCompDependency::TimingUtilities,
    ))
}

/// Placeholder for C `tmg_exterior_path_search`.
pub fn exterior_path_search(
    _clock_graph: &mut ConstraintGraph,
    _left: f64,
    _right: f64,
) -> Result<f64, TimingCompError> {
    Err(TimingCompError::missing(
        TimingCompDependency::TimingUtilities,
    ))
}

/// Placeholder for C `tmg_circuit_clock_update`.
pub fn update_circuit_clock(
    _latch_graph: &LatchGraph,
    _clock_graph: &ConstraintGraph,
    _network: &mut ClockNetwork,
    _cycle_time: f64,
) -> Result<(), TimingCompError> {
    Err(TimingCompError::missing(
        TimingCompDependency::NetworkClockModel,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_timing_int_h() {
        assert_eq!(EPS, 1.0e-3);
        assert_eq!(EPS1, 1.0e-5);
        assert_eq!(EPS2, 1.0e-2);
        assert_eq!(INFINITY, 10_000.0);
        assert_eq!(NOT_SET, -2);
    }

    #[test]
    fn errors_report_the_missing_dependency() {
        let error = TimingCompError::missing(TimingCompDependency::TimingGraph);

        assert_eq!(error.dependency(), TimingCompDependency::TimingGraph);
        assert_eq!(
            error.to_string(),
            "SIS timing graph types are not ported to Rust yet"
        );
    }
}
