//! Native Rust port scaffold for `sis/timing/timing_util.c`.
//!
//! The original C file is mostly utility allocation and timing-default state
//! for the SIS latch and constraint graphs. The actual graph, network, clock,
//! latch, and library-gate types are still C-only in this repository, so this
//! module ports the independent data model and exposes explicit scaffold
//! errors for routines that cannot be implemented until those native types
//! exist.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::{Mutex, OnceLock};

pub const EPS: f64 = 1.0e-3;
pub const EPS1: f64 = 1.0e-5;
pub const EPS2: f64 = 1.0e-2;
pub const INFINITY: f64 = 10_000.0;
pub const NOT_SET: i32 = -2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebugType {
    LatchGraph,
    ConstraintGraph,
    BranchAndBound,
    General,
    None,
    Verify,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlgorithmType {
    OptimalClock,
    ClockVerify,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchType {
    FallingEdgeFlipFlop,
    RisingEdgeFlipFlop,
    ActiveHighLevelSensitive,
    ActiveLowLevelSensitive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SisLatchKind {
    FallingEdge,
    RisingEdge,
    ActiveHigh,
    ActiveLow,
    Unknown,
}

impl SisLatchKind {
    pub fn timing_type(self) -> LatchType {
        match self {
            Self::FallingEdge | Self::Unknown => LatchType::FallingEdgeFlipFlop,
            Self::RisingEdge => LatchType::RisingEdgeFlipFlop,
            Self::ActiveHigh => LatchType::ActiveHighLevelSensitive,
            Self::ActiveLow => LatchType::ActiveLowLevelSensitive,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatchGraph<Clock, Host = ()> {
    pub clock_order: Vec<Clock>,
    pub host: Option<Host>,
}

impl<Clock, Host> LatchGraph<Clock, Host> {
    pub fn new(clock_order: Vec<Clock>) -> Self {
        Self {
            clock_order,
            host: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatchGraphNode<Latch = ()> {
    pub pio: i32,
    pub num: i32,
    pub latch: Option<Latch>,
    pub latch_type: Option<LatchType>,
    pub phase: Option<usize>,
    pub optimal_clock: Option<OptimalClockNode>,
    pub clock_verify: Option<ClockVerifyNode>,
}

impl<Latch> LatchGraphNode<Latch> {
    pub fn new(algorithm: AlgorithmType) -> Self {
        let optimal_clock = (algorithm == AlgorithmType::OptimalClock).then(OptimalClockNode::new);
        let clock_verify = (algorithm == AlgorithmType::ClockVerify).then(ClockVerifyNode::new);

        Self {
            pio: -1,
            num: -1,
            latch: None,
            latch_type: None,
            phase: None,
            optimal_clock,
            clock_verify,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatchGraphEdge {
    pub dmax: f64,
    pub dmin: f64,
    pub k: i32,
    pub weight: f64,
}

impl Default for LatchGraphEdge {
    fn default() -> Self {
        Self {
            dmax: -INFINITY,
            dmin: INFINITY,
            k: -1,
            weight: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptimalClockNode {
    pub weight: f64,
    pub previous_weight: f64,
    pub reachable: bool,
    pub dirty: bool,
}

impl OptimalClockNode {
    pub fn new() -> Self {
        Self {
            weight: 0.0,
            previous_weight: 0.0,
            reachable: false,
            dirty: false,
        }
    }
}

impl Default for OptimalClockNode {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockVerifyNode {
    pub latest_arrival: f64,
    pub earliest_arrival: f64,
    pub latest_departure: f64,
    pub earliest_departure: f64,
    pub dirty: bool,
}

impl ClockVerifyNode {
    pub fn new() -> Self {
        Self {
            latest_arrival: 0.0,
            earliest_arrival: 0.0,
            latest_departure: 0.0,
            earliest_departure: 0.0,
            dirty: false,
        }
    }
}

impl Default for ClockVerifyNode {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintGraph {
    pub phases: Vec<Phase>,
    pub zero: Option<ConstraintNode>,
}

impl ConstraintGraph {
    pub fn new() -> Self {
        Self {
            phases: Vec::new(),
            zero: None,
        }
    }

    pub fn allocate_phase_vertices(&mut self, phase_count: usize) {
        self.phases = (0..phase_count)
            .map(|index| {
                let id = (index + 1) as i32;
                Phase {
                    rise: ConstraintNode {
                        p: 0.0,
                        id,
                        matrix_id: 0,
                    },
                    fall: ConstraintNode {
                        p: 0.0,
                        id: -id,
                        matrix_id: 0,
                    },
                }
            })
            .collect();
    }
}

impl Default for ConstraintGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintEdge {
    pub ignore: bool,
    pub evaluate_c: i32,
    pub c1: HashMap<isize, f64>,
    pub c2: Option<f64>,
    pub weight: f64,
    pub fixed_weight: f64,
    pub duty: f64,
}

impl ConstraintEdge {
    pub fn new() -> Self {
        Self {
            ignore: false,
            evaluate_c: NOT_SET,
            c1: HashMap::new(),
            c2: None,
            weight: INFINITY,
            fixed_weight: INFINITY,
            duty: 2.0,
        }
    }
}

impl Default for ConstraintEdge {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintNode {
    pub p: f64,
    pub id: i32,
    pub matrix_id: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Phase {
    pub rise: ConstraintNode,
    pub fall: ConstraintNode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatchTiming {
    pub setup: f64,
    pub hold: f64,
}

pub trait LatchTimingSource {
    fn latch_timing(&self) -> Option<LatchTiming>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockOrderStatus {
    SortedByFallingEdge,
    KeptInputOrderBecausePhaseMissing,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TimingUtilError {
    MissingTimingGraphPort,
    MissingNetworkPort,
    MissingLatchPort,
    MissingClockPort,
}

impl fmt::Display for TimingUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTimingGraphPort => {
                write!(f, "SIS timing graph types are not ported to Rust yet")
            }
            Self::MissingNetworkPort => write!(f, "SIS network APIs are not ported to Rust yet"),
            Self::MissingLatchPort => write!(f, "SIS latch APIs are not ported to Rust yet"),
            Self::MissingClockPort => write!(f, "SIS clock APIs are not ported to Rust yet"),
        }
    }
}

impl Error for TimingUtilError {}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TimingDefaults {
    setup: f64,
    hold: f64,
    min_sep: f64,
    max_sep: f64,
    algorithm_flag: i32,
    phase_inv: bool,
}

impl Default for TimingDefaults {
    fn default() -> Self {
        Self {
            setup: 0.0,
            hold: 0.0,
            min_sep: 0.0,
            max_sep: 0.0,
            algorithm_flag: 0,
            phase_inv: false,
        }
    }
}

fn defaults() -> &'static Mutex<TimingDefaults> {
    static DEFAULTS: OnceLock<Mutex<TimingDefaults>> = OnceLock::new();
    DEFAULTS.get_or_init(|| Mutex::new(TimingDefaults::default()))
}

pub fn alloc_graph<Clock>(clock_order: Vec<Clock>) -> LatchGraph<Clock> {
    LatchGraph::new(clock_order)
}

pub fn alloc_node<Latch>(algorithm: AlgorithmType) -> LatchGraphNode<Latch> {
    LatchGraphNode::new(algorithm)
}

pub fn alloc_edge() -> LatchGraphEdge {
    LatchGraphEdge::default()
}

pub fn get_latch_type(kind: SisLatchKind) -> LatchType {
    kind.timing_type()
}

pub fn determine_clock_order<Clock: Clone>(
    clocks: &[Clock],
    mut nominal_falling_edge: impl FnMut(&Clock) -> Option<f64>,
) -> (Vec<Clock>, ClockOrderStatus) {
    let mut indexed = Vec::with_capacity(clocks.len());

    for clock in clocks {
        let Some(edge) = nominal_falling_edge(clock) else {
            return (
                clocks.to_vec(),
                ClockOrderStatus::KeptInputOrderBecausePhaseMissing,
            );
        };
        indexed.push((clock.clone(), edge));
    }

    indexed.sort_by(|(_, left), (_, right)| left.partial_cmp(right).unwrap_or(Ordering::Equal));

    (
        indexed.into_iter().map(|(clock, _)| clock).collect(),
        ClockOrderStatus::SortedByFallingEdge,
    )
}

pub fn latch_get_clock() -> Result<(), TimingUtilError> {
    Err(TimingUtilError::MissingLatchPort)
}

pub fn latch_get_phase() -> Result<usize, TimingUtilError> {
    Err(TimingUtilError::MissingClockPort)
}

pub fn network_check() -> Result<(), TimingUtilError> {
    Err(TimingUtilError::MissingNetworkPort)
}

pub fn graph_get_edge() -> Result<(), TimingUtilError> {
    Err(TimingUtilError::MissingTimingGraphPort)
}

pub fn set_setup(value: f64) {
    defaults().lock().expect("timing defaults poisoned").setup = value;
}

pub fn set_hold(value: f64) {
    defaults().lock().expect("timing defaults poisoned").hold = value;
}

pub fn get_setup(source: Option<&impl LatchTimingSource>) -> f64 {
    source
        .and_then(LatchTimingSource::latch_timing)
        .map(|timing| timing.setup)
        .unwrap_or_else(|| defaults().lock().expect("timing defaults poisoned").setup)
}

pub fn get_hold(source: Option<&impl LatchTimingSource>) -> f64 {
    source
        .and_then(LatchTimingSource::latch_timing)
        .map(|timing| timing.hold)
        .unwrap_or_else(|| defaults().lock().expect("timing defaults poisoned").hold)
}

pub fn set_min_sep(value: f64) {
    defaults().lock().expect("timing defaults poisoned").min_sep = value;
}

pub fn set_max_sep(value: f64) {
    defaults().lock().expect("timing defaults poisoned").max_sep = value;
}

pub fn get_min_sep() -> f64 {
    defaults().lock().expect("timing defaults poisoned").min_sep
}

pub fn get_max_sep() -> f64 {
    defaults().lock().expect("timing defaults poisoned").max_sep
}

pub fn alloc_cedge() -> ConstraintEdge {
    ConstraintEdge::new()
}

pub fn alloc_cgraph() -> ConstraintGraph {
    ConstraintGraph::new()
}

pub fn set_general_algorithm_flag(value: i32) {
    defaults()
        .lock()
        .expect("timing defaults poisoned")
        .algorithm_flag = value;
}

pub fn get_general_algorithm_flag() -> i32 {
    defaults()
        .lock()
        .expect("timing defaults poisoned")
        .algorithm_flag
}

pub fn set_phase_inversion(flag: bool) {
    defaults()
        .lock()
        .expect("timing defaults poisoned")
        .phase_inv = flag;
}

pub fn get_phase_inversion() -> bool {
    defaults()
        .lock()
        .expect("timing defaults poisoned")
        .phase_inv
}

pub fn max_clock_skew() -> f64 {
    0.0
}

pub fn min_clock_skew() -> f64 {
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    struct GateTiming(Option<LatchTiming>);

    impl LatchTimingSource for GateTiming {
        fn latch_timing(&self) -> Option<LatchTiming> {
            self.0
        }
    }

    #[test]
    fn allocated_latch_graph_nodes_match_c_initializers() {
        let optimal = alloc_node::<()>(AlgorithmType::OptimalClock);
        assert_eq!(optimal.pio, -1);
        assert_eq!(optimal.num, -1);
        assert_eq!(optimal.phase, None);
        assert_eq!(optimal.optimal_clock, Some(OptimalClockNode::new()));
        assert_eq!(optimal.clock_verify, None);

        let verify = alloc_node::<()>(AlgorithmType::ClockVerify);
        assert_eq!(verify.optimal_clock, None);
        assert_eq!(verify.clock_verify, Some(ClockVerifyNode::new()));

        let edge = alloc_edge();
        assert_eq!(edge.dmax, -INFINITY);
        assert_eq!(edge.dmin, INFINITY);
        assert_eq!(edge.k, -1);
    }

    #[test]
    fn constraint_edge_and_phase_vertices_match_c_defaults() {
        let edge = alloc_cedge();
        assert!(!edge.ignore);
        assert_eq!(edge.evaluate_c, NOT_SET);
        assert_eq!(edge.weight, INFINITY);
        assert_eq!(edge.fixed_weight, INFINITY);
        assert_eq!(edge.duty, 2.0);
        assert!(edge.c1.is_empty());
        assert_eq!(edge.c2, None);

        let mut graph = alloc_cgraph();
        graph.allocate_phase_vertices(2);
        assert_eq!(graph.phases[0].rise.id, 1);
        assert_eq!(graph.phases[0].fall.id, -1);
        assert_eq!(graph.phases[1].rise.id, 2);
        assert_eq!(graph.phases[1].fall.id, -2);
    }

    #[test]
    fn clock_order_sorts_by_falling_edge_or_keeps_input_order() {
        let clocks = vec!["late", "early", "middle"];
        let (ordered, status) = determine_clock_order(&clocks, |clock| match *clock {
            "early" => Some(1.0),
            "middle" => Some(2.0),
            "late" => Some(3.0),
            _ => None,
        });
        assert_eq!(status, ClockOrderStatus::SortedByFallingEdge);
        assert_eq!(ordered, vec!["early", "middle", "late"]);

        let (ordered, status) = determine_clock_order(&clocks, |_| None);
        assert_eq!(status, ClockOrderStatus::KeptInputOrderBecausePhaseMissing);
        assert_eq!(ordered, clocks);
    }

    #[test]
    fn timing_defaults_fall_back_when_library_gate_timing_is_absent() {
        set_setup(1.25);
        set_hold(0.75);
        assert_eq!(get_setup(None::<&GateTiming>), 1.25);
        assert_eq!(get_hold(None::<&GateTiming>), 0.75);

        let gate = GateTiming(Some(LatchTiming {
            setup: 3.0,
            hold: 4.0,
        }));
        assert_eq!(get_setup(Some(&gate)), 3.0);
        assert_eq!(get_hold(Some(&gate)), 4.0);
    }

    #[test]
    fn global_flags_are_available_without_c_static_storage() {
        set_min_sep(0.5);
        set_max_sep(8.0);
        set_general_algorithm_flag(7);
        set_phase_inversion(true);

        assert_eq!(get_min_sep(), 0.5);
        assert_eq!(get_max_sep(), 8.0);
        assert_eq!(get_general_algorithm_flag(), 7);
        assert!(get_phase_inversion());
    }

    #[test]
    fn graph_bound_routines_report_unported_dependencies() {
        assert_eq!(network_check(), Err(TimingUtilError::MissingNetworkPort));
        assert_eq!(
            graph_get_edge(),
            Err(TimingUtilError::MissingTimingGraphPort)
        );
        assert_eq!(latch_get_clock(), Err(TimingUtilError::MissingLatchPort));
        assert_eq!(latch_get_phase(), Err(TimingUtilError::MissingClockPort));
        assert_eq!(max_clock_skew(), 0.0);
        assert_eq!(min_clock_skew(), 0.0);
    }
}
