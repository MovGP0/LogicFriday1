//! Scaffold for `LogicSynthesis/sis/timing/timing_fast_comp.c`.
//!
//! The original unit implements SIS optimal clock computation over the SIS
//! latch graph and network clock model. Those graph, latch, and clock types are
//! still C-only in this port, so this file exposes the intended Rust API shape
//! without adding legacy per-file C ABI exports.

use std::error::Error;
use std::fmt;

pub const EPS: f64 = 1.0e-3;
pub const EPS1: f64 = 1.0e-5;
pub const EPS2: f64 = 1.0e-2;
pub const INFINITY: f64 = 10_000.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingFastCompStatus {
    Feasible,
    Solve,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExteriorPathStatus {
    NegativeCycle,
    Infeasible,
    AllPositiveCycles,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TimingFastCompError {
    MissingTimingGraphPort,
    MissingTimingUtilityPort,
    MissingNetworkClockPort,
}

impl fmt::Display for TimingFastCompError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTimingGraphPort => {
                write!(f, "SIS timing graph types are not ported to Rust yet")
            }
            Self::MissingTimingUtilityPort => {
                write!(f, "SIS timing utility routines are not ported to Rust yet")
            }
            Self::MissingNetworkClockPort => {
                write!(
                    f,
                    "SIS network clock update APIs are not ported to Rust yet"
                )
            }
        }
    }
}

impl Error for TimingFastCompError {}

#[derive(Clone, Debug, PartialEq)]
pub struct OptimalClockResult {
    pub cycle_time: f64,
}

/// Compute optimal clocking parameters for a SIS latch graph and network.
///
/// The C implementation depends on `graph_t`, `network_t`, `vertex_t`,
/// `edge_t`, `sis_clock_t`, and timing utility routines declared by
/// `timing_int.h`. Those dependencies do not have native Rust equivalents in
/// `LogicFriday1.Sis.Native` yet, so this API is intentionally a scaffold.
pub fn compute_optimal_clock() -> Result<OptimalClockResult, TimingFastCompError> {
    Err(TimingFastCompError::MissingTimingGraphPort)
}

/// Placeholder for the C `clock_lower_bound` routine.
pub fn clock_lower_bound() -> Result<f64, TimingFastCompError> {
    Err(TimingFastCompError::MissingTimingGraphPort)
}

/// Placeholder for the C `solve_constraints` routine.
pub fn solve_constraints() -> Result<f64, TimingFastCompError> {
    Err(TimingFastCompError::MissingTimingUtilityPort)
}

/// Placeholder for the C `solve_gen_constraints` routine.
pub fn solve_general_constraints() -> Result<f64, TimingFastCompError> {
    Err(TimingFastCompError::MissingTimingUtilityPort)
}

/// Placeholder for writing solved clock positions back into a SIS network.
pub fn update_circuit_clock() -> Result<(), TimingFastCompError> {
    Err(TimingFastCompError::MissingNetworkClockPort)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_scaffold_reports_missing_graph_dependency() {
        assert_eq!(
            compute_optimal_clock(),
            Err(TimingFastCompError::MissingTimingGraphPort)
        );
        assert_eq!(
            clock_lower_bound(),
            Err(TimingFastCompError::MissingTimingGraphPort)
        );
    }

    #[test]
    fn constants_match_timing_int_h() {
        assert_eq!(EPS, 1.0e-3);
        assert_eq!(EPS1, 1.0e-5);
        assert_eq!(EPS2, 1.0e-2);
        assert_eq!(INFINITY, 10_000.0);
    }
}
