//! Native Rust port scaffold for `sis/timing/timing_verify.c`.
//!
//! The original C file verifies a clocking scheme over SIS `graph_t` and
//! `network_t` values and reads clock positions through the SIS clock package.
//! Those integration types are still C-only, so this module ports the
//! independent verification algorithm onto a small Rust graph model and reports
//! explicit dependency errors for the blocked SIS-network entry point.

use std::error::Error;
use std::fmt;

pub const EPS: f64 = 1.0e-3;
pub const EPS1: f64 = 1.0e-5;
pub const EPS2: f64 = 1.0e-2;
pub const INFINITY: f64 = 10_000.0;
pub const NOT_SET: i32 = -2;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VertexId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LatchType {
    FallingEdgeFlipFlop,
    RisingEdgeFlipFlop,
    ActiveHighLevelSensitive,
    ActiveLowLevelSensitive,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VertexKind {
    NetworkIo,
    Latch {
        latch_type: LatchType,
        phase: usize,
        setup: f64,
        hold: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifyVertex {
    pub id: usize,
    pub kind: VertexKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifyEdge {
    pub from: VertexId,
    pub to: VertexId,
    pub max_delay: f64,
    pub min_delay: f64,
    pub k: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClockVerificationGraph {
    vertices: Vec<VerifyVertex>,
    edges: Vec<VerifyEdge>,
}

impl ClockVerificationGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vertices(&self) -> &[VerifyVertex] {
        &self.vertices
    }

    pub fn edges(&self) -> &[VerifyEdge] {
        &self.edges
    }

    pub fn add_network_io(&mut self) -> VertexId {
        self.push_vertex(VertexKind::NetworkIo)
    }

    pub fn add_latch(
        &mut self,
        latch_type: LatchType,
        phase: usize,
        setup: f64,
        hold: f64,
    ) -> VertexId {
        self.push_vertex(VertexKind::Latch {
            latch_type,
            phase,
            setup,
            hold,
        })
    }

    pub fn add_edge(
        &mut self,
        from: VertexId,
        to: VertexId,
        max_delay: f64,
        min_delay: f64,
        k: i32,
    ) -> Result<(), ClockVerifyError> {
        self.vertex(from)?;
        self.vertex(to)?;
        self.edges.push(VerifyEdge {
            from,
            to,
            max_delay,
            min_delay,
            k,
        });
        Ok(())
    }

    fn push_vertex(&mut self, kind: VertexKind) -> VertexId {
        let id = VertexId(self.vertices.len());
        self.vertices.push(VerifyVertex {
            id: self.vertices.len(),
            kind,
        });
        id
    }

    fn vertex(&self, id: VertexId) -> Result<&VerifyVertex, ClockVerifyError> {
        self.vertices
            .get(id.0)
            .ok_or(ClockVerifyError::UnknownVertex(id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockPhaseSpec {
    pub rise: Option<f64>,
    pub fall: Option<f64>,
}

impl ClockPhaseSpec {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self {
            rise: Some(rise),
            fall: Some(fall),
        }
    }

    pub fn parse_debug_line(line: &str) -> Result<(usize, Self), ClockVerifyError> {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 6 || fields[0] != "Phase" || fields[2] != "^" || fields[4] != "v" {
            return Err(ClockVerifyError::ParseClockEventLine(line.to_owned()));
        }

        let phase = fields[1]
            .parse()
            .map_err(|_| ClockVerifyError::ParseClockEventLine(line.to_owned()))?;
        let rise = fields[3]
            .parse()
            .map_err(|_| ClockVerifyError::ParseClockEventLine(line.to_owned()))?;
        let fall = fields[5]
            .parse()
            .map_err(|_| ClockVerifyError::ParseClockEventLine(line.to_owned()))?;

        Ok((phase, Self::new(rise, fall)))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockEvents {
    pub rise: Vec<f64>,
    pub fall: Vec<f64>,
    pub shift: Vec<f64>,
}

impl ClockEvents {
    pub fn from_phase_specs(
        cycle_time: f64,
        phases: &[ClockPhaseSpec],
    ) -> Result<Self, ClockVerifyError> {
        let mut rise = Vec::with_capacity(phases.len());
        let mut fall = Vec::with_capacity(phases.len());

        for (index, phase) in phases.iter().enumerate() {
            let fall_event = phase
                .fall
                .ok_or(ClockVerifyError::MissingClockEvent { phase: index })?;
            let rise_event = phase
                .rise
                .ok_or(ClockVerifyError::MissingClockEvent { phase: index })?;
            fall.push(fall_event);
            rise.push(rise_event);
        }

        Self::new(cycle_time, rise, fall)
    }

    pub fn new(cycle_time: f64, rise: Vec<f64>, fall: Vec<f64>) -> Result<Self, ClockVerifyError> {
        if rise.len() != fall.len() {
            return Err(ClockVerifyError::MismatchedClockEventCount {
                rise: rise.len(),
                fall: fall.len(),
            });
        }

        let shift = fall
            .iter()
            .enumerate()
            .map(|(phase, fall_event)| {
                let shift = cycle_time - fall_event;
                if shift < 0.0 {
                    Err(ClockVerifyError::ClockEventOutsideCycle { phase })
                } else {
                    Ok(shift)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { rise, fall, shift })
    }

    pub fn from_debug_lines<'a>(
        cycle_time: f64,
        lines: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, ClockVerifyError> {
        let mut specs = Vec::<Option<ClockPhaseSpec>>::new();

        for line in lines {
            let (phase, spec) = ClockPhaseSpec::parse_debug_line(line)?;
            if specs.len() <= phase {
                specs.resize(phase + 1, None);
            }
            if specs[phase].replace(spec).is_some() {
                return Err(ClockVerifyError::DuplicateClockPhase { phase });
            }
        }

        let phases = specs
            .into_iter()
            .enumerate()
            .map(|(phase, spec)| spec.ok_or(ClockVerifyError::MissingClockPhase { phase }))
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_phase_specs(cycle_time, &phases)
    }

    pub fn len(&self) -> usize {
        self.rise.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rise.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockVerifyState {
    pub latest_arrival: f64,
    pub earliest_arrival: f64,
    pub latest_departure: f64,
    pub earliest_departure: f64,
}

impl ClockVerifyState {
    fn network_io() -> Self {
        Self {
            latest_arrival: 0.0,
            earliest_arrival: 0.0,
            latest_departure: 0.0,
            earliest_departure: 0.0,
        }
    }

    fn latch_initial() -> Self {
        Self {
            latest_arrival: -INFINITY,
            earliest_arrival: -INFINITY,
            latest_departure: -INFINITY,
            earliest_departure: -INFINITY,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockVerificationResult {
    pub states: Vec<ClockVerifyState>,
    pub iterations: usize,
    pub converged: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimingTrace {
    pub start: TraceStart,
    pub steps: Vec<TraceStep>,
}

impl TimingTrace {
    pub fn format_c_style(&self) -> String {
        let mut output = String::new();
        match self.start {
            TraceStart::NetworkIo => {
                output.push_str("HOST->\n");
                output.push_str("Start 0.00\n");
            }
            TraceStart::Latch { vertex, time } => {
                output.push_str(&format!("{} ->\n", vertex.0));
                output.push_str(&format!("Start {time:.2}\n"));
            }
        }

        for step in &self.steps {
            output.push_str(&format!(
                " {:.2} \t{} -> {} \t {:.2}\n",
                step.max_delay, step.k, step.to.0, step.cumulative_time
            ));
        }
        output
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TraceStart {
    NetworkIo,
    Latch { vertex: VertexId, time: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceStep {
    pub from: VertexId,
    pub to: VertexId,
    pub max_delay: f64,
    pub k: i32,
    pub cumulative_time: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClockVerifyError {
    MissingNetworkPort,
    CycleTimeNotSet,
    MissingClockEvent {
        phase: usize,
    },
    ClockEventOutsideCycle {
        phase: usize,
    },
    MismatchedClockEventCount {
        rise: usize,
        fall: usize,
    },
    ParseClockEventLine(String),
    DuplicateClockPhase {
        phase: usize,
    },
    MissingClockPhase {
        phase: usize,
    },
    UnknownVertex(VertexId),
    InvalidLatchPhase {
        vertex: VertexId,
        phase: usize,
        clock_count: usize,
    },
    NonMonotonicArrival {
        vertex: VertexId,
    },
    SetupViolation {
        vertex: VertexId,
        arrival: f64,
        limit: f64,
        trace: Option<TimingTrace>,
    },
    HoldViolation {
        vertex: VertexId,
        arrival: f64,
        limit: f64,
    },
    ConvergenceFailed,
}

impl fmt::Display for ClockVerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNetworkPort => {
                write!(
                    f,
                    "SIS graph, network, and clock APIs are not ported to Rust yet"
                )
            }
            Self::CycleTimeNotSet => write!(f, "clock cycle not set"),
            Self::MissingClockEvent { phase } => {
                write!(f, "clock event for phase {phase} is not set")
            }
            Self::ClockEventOutsideCycle { phase } => {
                write!(f, "clock event for phase {phase} is outside the cycle time")
            }
            Self::MismatchedClockEventCount { rise, fall } => {
                write!(
                    f,
                    "rise/fall clock event counts differ: {rise} rise, {fall} fall"
                )
            }
            Self::ParseClockEventLine(line) => {
                write!(f, "could not parse clock event line {line:?}")
            }
            Self::DuplicateClockPhase { phase } => {
                write!(f, "duplicate clock event line for phase {phase}")
            }
            Self::MissingClockPhase { phase } => {
                write!(f, "missing clock event line for phase {phase}")
            }
            Self::UnknownVertex(vertex) => write!(f, "unknown verification vertex {:?}", vertex),
            Self::InvalidLatchPhase {
                vertex,
                phase,
                clock_count,
            } => write!(
                f,
                "vertex {:?} has invalid one-based phase {phase} for {clock_count} clocks",
                vertex
            ),
            Self::NonMonotonicArrival { vertex } => {
                write!(
                    f,
                    "arrival update for vertex {:?} was not monotonic",
                    vertex
                )
            }
            Self::SetupViolation {
                vertex,
                arrival,
                limit,
                ..
            } => write!(
                f,
                "set-up violation at {:?}: arrival {arrival:.3} exceeds {limit:.3}",
                vertex
            ),
            Self::HoldViolation {
                vertex,
                arrival,
                limit,
            } => write!(
                f,
                "hold violation at {:?}: arrival {arrival:.3} is below {limit:.3}",
                vertex
            ),
            Self::ConvergenceFailed => write!(f, "clock verification failed to converge"),
        }
    }
}

impl Error for ClockVerifyError {}

/// Placeholder for the C entry point that accepts SIS `graph_t`/`network_t`.
pub fn check_sis_clocking_scheme() -> Result<ClockVerificationResult, ClockVerifyError> {
    Err(ClockVerifyError::MissingNetworkPort)
}

pub fn check_clocking_scheme(
    graph: &ClockVerificationGraph,
    cycle_time: f64,
    clock_events: &ClockEvents,
) -> Result<ClockVerificationResult, ClockVerifyError> {
    if cycle_time < 0.0 {
        return Err(ClockVerifyError::CycleTimeNotSet);
    }

    validate_graph_phases(graph, clock_events)?;

    let incoming = incoming_edges(graph);
    let mut states = graph
        .vertices
        .iter()
        .map(|vertex| match vertex.kind {
            VertexKind::NetworkIo => ClockVerifyState::network_io(),
            VertexKind::Latch { .. } => ClockVerifyState::latch_initial(),
        })
        .collect::<Vec<_>>();

    if graph.edges.is_empty() {
        return Ok(ClockVerificationResult {
            states,
            iterations: 0,
            converged: true,
        });
    }

    let mut converged = false;
    let mut iterations = 0;

    for iteration in 0..=graph.edges.len() {
        iterations = iteration + 1;
        converged = true;

        for vertex_id in 0..graph.vertices.len() {
            set_departure_times(
                &graph.vertices[vertex_id],
                VertexId(vertex_id),
                &mut states[vertex_id],
                cycle_time,
                clock_events,
            )?;
        }

        for vertex_id in 0..graph.vertices.len() {
            let vertex = &graph.vertices[vertex_id];
            let VertexKind::Latch {
                latch_type,
                phase,
                setup,
                hold,
            } = vertex.kind
            else {
                continue;
            };

            let phase_index = phase - 1;
            let (max_arrival, min_arrival) = arrival_bounds(
                graph,
                &incoming[vertex_id],
                &states,
                phase_index,
                clock_events,
                cycle_time,
            );

            if states[vertex_id].latest_arrival > max_arrival + EPS {
                return Err(ClockVerifyError::NonMonotonicArrival {
                    vertex: VertexId(vertex_id),
                });
            }
            if states[vertex_id].earliest_arrival > min_arrival + EPS {
                return Err(ClockVerifyError::NonMonotonicArrival {
                    vertex: VertexId(vertex_id),
                });
            }

            if states[vertex_id].latest_arrival + EPS < max_arrival {
                states[vertex_id].latest_arrival = max_arrival;
                converged = false;

                let setup_limit =
                    setup_limit(latch_type, phase_index, setup, cycle_time, clock_events);
                if states[vertex_id].latest_arrival > setup_limit + EPS {
                    let trace = trace_erroneous_path(
                        graph,
                        clock_events,
                        cycle_time,
                        &states,
                        VertexId(vertex_id),
                        states[vertex_id].latest_arrival,
                    );
                    return Err(ClockVerifyError::SetupViolation {
                        vertex: VertexId(vertex_id),
                        arrival: states[vertex_id].latest_arrival,
                        limit: setup_limit,
                        trace,
                    });
                }
            }

            if states[vertex_id].earliest_arrival + EPS < min_arrival {
                states[vertex_id].earliest_arrival = min_arrival;
                converged = false;
            }

            if iteration == 0 {
                let hold_limit = hold_limit(latch_type, phase_index, hold, clock_events);
                if states[vertex_id].earliest_arrival + EPS < hold_limit {
                    return Err(ClockVerifyError::HoldViolation {
                        vertex: VertexId(vertex_id),
                        arrival: states[vertex_id].earliest_arrival,
                        limit: hold_limit,
                    });
                }
            }
        }

        if converged {
            break;
        }
    }

    if !converged {
        return Err(ClockVerifyError::ConvergenceFailed);
    }

    Ok(ClockVerificationResult {
        states,
        iterations,
        converged,
    })
}

fn validate_graph_phases(
    graph: &ClockVerificationGraph,
    clock_events: &ClockEvents,
) -> Result<(), ClockVerifyError> {
    for (index, vertex) in graph.vertices.iter().enumerate() {
        if let VertexKind::Latch { phase, .. } = vertex.kind {
            if phase == 0 || phase > clock_events.len() {
                return Err(ClockVerifyError::InvalidLatchPhase {
                    vertex: VertexId(index),
                    phase,
                    clock_count: clock_events.len(),
                });
            }
        }
    }
    Ok(())
}

fn incoming_edges(graph: &ClockVerificationGraph) -> Vec<Vec<usize>> {
    let mut incoming = vec![Vec::new(); graph.vertices.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        incoming[edge.to.0].push(index);
    }
    incoming
}

fn set_departure_times(
    vertex: &VerifyVertex,
    vertex_id: VertexId,
    state: &mut ClockVerifyState,
    cycle_time: f64,
    clock_events: &ClockEvents,
) -> Result<(), ClockVerifyError> {
    match vertex.kind {
        VertexKind::NetworkIo => {
            state.latest_departure = 0.0;
            state.earliest_departure = 0.0;
        }
        VertexKind::Latch {
            latch_type, phase, ..
        } => {
            let phase_index = checked_phase_index(vertex_id, phase, clock_events)?;
            let base = departure_base(latch_type, phase_index, cycle_time, clock_events);
            match latch_type {
                LatchType::FallingEdgeFlipFlop | LatchType::RisingEdgeFlipFlop => {
                    state.latest_departure = base;
                    state.earliest_departure = base;
                }
                LatchType::ActiveHighLevelSensitive | LatchType::ActiveLowLevelSensitive => {
                    state.latest_departure = state.latest_arrival.max(base);
                    state.earliest_departure = state.earliest_arrival.max(base);
                }
            }
        }
    }
    Ok(())
}

fn checked_phase_index(
    vertex: VertexId,
    phase: usize,
    clock_events: &ClockEvents,
) -> Result<usize, ClockVerifyError> {
    if phase == 0 || phase > clock_events.len() {
        Err(ClockVerifyError::InvalidLatchPhase {
            vertex,
            phase,
            clock_count: clock_events.len(),
        })
    } else {
        Ok(phase - 1)
    }
}

fn departure_base(
    latch_type: LatchType,
    phase_index: usize,
    cycle_time: f64,
    clock_events: &ClockEvents,
) -> f64 {
    match latch_type {
        LatchType::FallingEdgeFlipFlop => cycle_time,
        LatchType::RisingEdgeFlipFlop | LatchType::ActiveHighLevelSensitive => {
            clock_events.rise[phase_index] + clock_events.shift[phase_index]
        }
        LatchType::ActiveLowLevelSensitive => 0.0,
    }
}

fn arrival_bounds(
    graph: &ClockVerificationGraph,
    incoming_edges: &[usize],
    states: &[ClockVerifyState],
    sink_phase: usize,
    clock_events: &ClockEvents,
    cycle_time: f64,
) -> (f64, f64) {
    let mut max_arrival = -INFINITY;
    let mut min_arrival = INFINITY;

    for edge_index in incoming_edges {
        let edge = &graph.edges[*edge_index];
        let source = &graph.vertices[edge.from.0];
        let shift = match source.kind {
            VertexKind::NetworkIo => 0.0,
            VertexKind::Latch { phase, .. } => {
                let source_phase = phase - 1;
                clock_events.fall[sink_phase] - clock_events.fall[source_phase]
                    + f64::from(edge.k) * cycle_time
            }
        };

        max_arrival =
            max_arrival.max(edge.max_delay + states[edge.from.0].latest_departure - shift);
        min_arrival =
            min_arrival.min(edge.min_delay + states[edge.from.0].earliest_departure - shift);
    }

    (max_arrival, min_arrival)
}

fn setup_limit(
    latch_type: LatchType,
    phase_index: usize,
    setup: f64,
    cycle_time: f64,
    clock_events: &ClockEvents,
) -> f64 {
    match latch_type {
        LatchType::FallingEdgeFlipFlop | LatchType::ActiveHighLevelSensitive => cycle_time - setup,
        LatchType::RisingEdgeFlipFlop | LatchType::ActiveLowLevelSensitive => {
            clock_events.rise[phase_index] + clock_events.shift[phase_index] - setup
        }
    }
}

fn hold_limit(
    latch_type: LatchType,
    phase_index: usize,
    hold: f64,
    clock_events: &ClockEvents,
) -> f64 {
    match latch_type {
        LatchType::FallingEdgeFlipFlop | LatchType::ActiveHighLevelSensitive => hold,
        LatchType::RisingEdgeFlipFlop | LatchType::ActiveLowLevelSensitive => {
            clock_events.rise[phase_index] + clock_events.shift[phase_index] + hold
        }
    }
}

fn trace_erroneous_path(
    graph: &ClockVerificationGraph,
    clock_events: &ClockEvents,
    cycle_time: f64,
    states: &[ClockVerifyState],
    start: VertexId,
    arrival: f64,
) -> Option<TimingTrace> {
    let incoming = incoming_edges(graph);
    let mut dirty = vec![false; graph.vertices.len()];
    let mut trace = trace_recursive_path(
        graph,
        clock_events,
        cycle_time,
        states,
        &incoming,
        &mut dirty,
        start,
        arrival,
    )?;
    trace.steps.reverse();
    Some(trace)
}

#[allow(clippy::too_many_arguments)]
fn trace_recursive_path(
    graph: &ClockVerificationGraph,
    clock_events: &ClockEvents,
    cycle_time: f64,
    states: &[ClockVerifyState],
    incoming: &[Vec<usize>],
    dirty: &mut [bool],
    vertex_id: VertexId,
    arrival: f64,
) -> Option<TimingTrace> {
    let vertex = &graph.vertices[vertex_id.0];
    let phase_index = match vertex.kind {
        VertexKind::NetworkIo => {
            if arrival.abs() < EPS {
                return Some(TimingTrace {
                    start: TraceStart::NetworkIo,
                    steps: Vec::new(),
                });
            }
            return None;
        }
        VertexKind::Latch { phase, .. } => phase - 1,
    };

    if let VertexKind::Latch { latch_type, .. } = vertex.kind {
        let base = departure_base(latch_type, phase_index, cycle_time, clock_events);
        if (arrival - base).abs() < EPS {
            return Some(TimingTrace {
                start: TraceStart::Latch {
                    vertex: vertex_id,
                    time: base,
                },
                steps: Vec::new(),
            });
        }
    }

    if dirty[vertex_id.0] {
        return None;
    }
    dirty[vertex_id.0] = true;

    for edge_index in &incoming[vertex_id.0] {
        let edge = &graph.edges[*edge_index];
        let source = &graph.vertices[edge.from.0];
        let shift = match source.kind {
            VertexKind::NetworkIo => 0.0,
            VertexKind::Latch { phase, .. } => {
                let source_phase = phase - 1;
                clock_events.fall[source_phase]
                    - clock_events.fall[phase_index]
                    - f64::from(edge.k) * cycle_time
            }
        };
        let new_arrival = arrival - edge.max_delay - shift;

        if (new_arrival - states[edge.from.0].latest_departure).abs() > EPS
            && !matches!(source.kind, VertexKind::NetworkIo)
        {
            continue;
        }

        if let Some(mut trace) = trace_recursive_path(
            graph,
            clock_events,
            cycle_time,
            states,
            incoming,
            dirty,
            edge.from,
            new_arrival,
        ) {
            let previous = trace
                .steps
                .last()
                .map(|step| step.cumulative_time)
                .unwrap_or(match trace.start {
                    TraceStart::NetworkIo => 0.0,
                    TraceStart::Latch { time, .. } => time,
                });
            trace.steps.push(TraceStep {
                from: edge.from,
                to: edge.to,
                max_delay: edge.max_delay,
                k: edge.k,
                cumulative_time: previous + edge.max_delay + shift,
            });
            dirty[vertex_id.0] = false;
            return Some(trace);
        }
    }

    dirty[vertex_id.0] = false;
    None
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
    fn clock_events_match_c_shift_calculation_and_errors() {
        let events = ClockEvents::from_phase_specs(
            10.0,
            &[ClockPhaseSpec::new(1.0, 4.0), ClockPhaseSpec::new(2.0, 9.0)],
        )
        .unwrap();

        assert_eq!(events.rise, vec![1.0, 2.0]);
        assert_eq!(events.fall, vec![4.0, 9.0]);
        assert_eq!(events.shift, vec![6.0, 1.0]);

        assert_eq!(
            ClockEvents::from_phase_specs(
                10.0,
                &[ClockPhaseSpec {
                    rise: Some(1.0),
                    fall: None
                }]
            ),
            Err(ClockVerifyError::MissingClockEvent { phase: 0 })
        );
        assert_eq!(
            ClockEvents::from_phase_specs(10.0, &[ClockPhaseSpec::new(1.0, 11.0)]),
            Err(ClockVerifyError::ClockEventOutsideCycle { phase: 0 })
        );
    }

    #[test]
    fn parses_c_debug_clock_event_lines() {
        let events = ClockEvents::from_debug_lines(
            10.0,
            ["Phase 0  ^ 1.25  v 4.50", "Phase 1  ^ 2.00  v 8.00"],
        )
        .unwrap();

        assert_eq!(events.rise, vec![1.25, 2.0]);
        assert_eq!(events.fall, vec![4.5, 8.0]);
        assert_eq!(events.shift, vec![5.5, 2.0]);

        assert_eq!(
            ClockPhaseSpec::parse_debug_line("Clock 0 rise 1 fall 2"),
            Err(ClockVerifyError::ParseClockEventLine(
                "Clock 0 rise 1 fall 2".to_owned()
            ))
        );
    }

    #[test]
    fn verifies_simple_host_to_falling_edge_latch_path() {
        let mut graph = ClockVerificationGraph::new();
        let host = graph.add_network_io();
        let latch = graph.add_latch(LatchType::FallingEdgeFlipFlop, 1, 1.0, 0.5);
        graph.add_edge(host, latch, 8.0, 1.0, 0).unwrap();
        let events = ClockEvents::from_phase_specs(10.0, &[ClockPhaseSpec::new(1.0, 5.0)]).unwrap();

        let result = check_clocking_scheme(&graph, 10.0, &events).unwrap();

        assert!(result.converged);
        assert_eq!(result.iterations, 2);
        assert_eq!(result.states[latch.0].latest_arrival, 8.0);
        assert_eq!(result.states[latch.0].earliest_arrival, 1.0);
    }

    #[test]
    fn reports_setup_violation_with_c_style_trace() {
        let mut graph = ClockVerificationGraph::new();
        let host = graph.add_network_io();
        let latch = graph.add_latch(LatchType::FallingEdgeFlipFlop, 1, 1.0, 0.0);
        graph.add_edge(host, latch, 9.5, 1.0, 0).unwrap();
        let events = ClockEvents::from_phase_specs(10.0, &[ClockPhaseSpec::new(1.0, 5.0)]).unwrap();

        let error = check_clocking_scheme(&graph, 10.0, &events).unwrap_err();

        let ClockVerifyError::SetupViolation {
            vertex,
            arrival,
            limit,
            trace,
        } = error
        else {
            panic!("expected setup violation");
        };
        assert_eq!(vertex, latch);
        assert_eq!(arrival, 9.5);
        assert_eq!(limit, 9.0);
        let trace = trace.unwrap().format_c_style();
        assert!(trace.contains("HOST->"));
        assert!(trace.contains(" 9.50 \t0 -> 1 \t 9.50"));
    }

    #[test]
    fn reports_hold_violation_on_first_iteration() {
        let mut graph = ClockVerificationGraph::new();
        let host = graph.add_network_io();
        let latch = graph.add_latch(LatchType::FallingEdgeFlipFlop, 1, 0.0, 0.5);
        graph.add_edge(host, latch, 2.0, 0.1, 0).unwrap();
        let events = ClockEvents::from_phase_specs(10.0, &[ClockPhaseSpec::new(1.0, 5.0)]).unwrap();

        assert_eq!(
            check_clocking_scheme(&graph, 10.0, &events),
            Err(ClockVerifyError::HoldViolation {
                vertex: latch,
                arrival: 0.1,
                limit: 0.5
            })
        );
    }

    #[test]
    fn level_sensitive_latch_uses_arrival_as_departure_after_first_pass() {
        let mut graph = ClockVerificationGraph::new();
        let host = graph.add_network_io();
        let latch = graph.add_latch(LatchType::ActiveHighLevelSensitive, 1, 0.0, 0.0);
        graph.add_edge(host, latch, 8.0, 8.0, 0).unwrap();
        let events = ClockEvents::from_phase_specs(10.0, &[ClockPhaseSpec::new(1.0, 4.0)]).unwrap();

        let result = check_clocking_scheme(&graph, 10.0, &events).unwrap();

        assert_eq!(result.states[latch.0].latest_arrival, 8.0);
        assert_eq!(result.states[latch.0].latest_departure, 8.0);
    }

    #[test]
    fn blocked_sis_entry_point_reports_unported_dependency() {
        assert_eq!(
            check_sis_clocking_scheme(),
            Err(ClockVerifyError::MissingNetworkPort)
        );
    }
}
