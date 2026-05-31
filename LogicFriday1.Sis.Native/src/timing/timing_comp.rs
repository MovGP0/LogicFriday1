//! Native optimal clock constraint solver.
//!
//! This module ports the clock-bound search, constraint graph arithmetic,
//! Bellman-Ford feasibility test, general exterior-path search, model
//! constraints, phase-separation constraints, and solved clock update logic to
//! safe Rust data structures.

use std::error::Error;
use std::fmt;

pub const EPS: f64 = 1.0e-3;
pub const EPS1: f64 = 1.0e-5;
pub const EPS2: f64 = 1.0e-2;
pub const INFTY: f64 = 10_000.0;
pub const INFINITY: f64 = INFTY;
pub const NOT_SET: i32 = -2;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LatchNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ConstraintNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ConstraintEdgeId(pub usize);

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
pub enum LatchNodeKind {
    NetworkIo,
    NetworkNode,
    Latch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchType {
    FallingEdgeFlipFlop,
    RisingEdgeFlipFlop,
    ActiveHighLevelSensitive,
    ActiveLowLevelSensitive,
}

impl LatchType {
    fn from_event(self, phase: usize, phase_count: usize) -> usize {
        match self {
            Self::RisingEdgeFlipFlop | Self::ActiveHighLevelSensitive => phase,
            Self::FallingEdgeFlipFlop | Self::ActiveLowLevelSensitive => phase + phase_count,
        }
    }

    fn to_event(self, phase: usize, phase_count: usize) -> usize {
        match self {
            Self::RisingEdgeFlipFlop | Self::ActiveLowLevelSensitive => phase,
            Self::FallingEdgeFlipFlop | Self::ActiveHighLevelSensitive => phase + phase_count,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatchNode {
    pub kind: LatchNodeKind,
    pub latch_type: LatchType,
    pub phase: usize,
    pub setup: f64,
    pub hold: f64,
    pub min_clock_skew: f64,
    pub max_clock_skew: f64,
    weight: f64,
    previous_weight: f64,
    reachable: i32,
    dirty: bool,
    parent: Option<LatchNodeId>,
}

impl LatchNode {
    pub fn network_io() -> Self {
        Self {
            kind: LatchNodeKind::NetworkIo,
            latch_type: LatchType::FallingEdgeFlipFlop,
            phase: 0,
            setup: 0.0,
            hold: 0.0,
            min_clock_skew: 0.0,
            max_clock_skew: 0.0,
            weight: 0.0,
            previous_weight: 0.0,
            reachable: -1,
            dirty: false,
            parent: None,
        }
    }

    pub fn latch(latch_type: LatchType, phase: usize) -> Self {
        Self {
            kind: LatchNodeKind::Latch,
            latch_type,
            phase,
            setup: 0.0,
            hold: 0.0,
            min_clock_skew: 0.0,
            max_clock_skew: 0.0,
            weight: 0.0,
            previous_weight: 0.0,
            reachable: -1,
            dirty: false,
            parent: None,
        }
    }

    pub fn with_timing(mut self, setup: f64, hold: f64) -> Self {
        self.setup = setup;
        self.hold = hold;
        self
    }

    pub fn with_clock_skew(mut self, min_clock_skew: f64, max_clock_skew: f64) -> Self {
        self.min_clock_skew = min_clock_skew;
        self.max_clock_skew = max_clock_skew;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatchEdge {
    pub source: LatchNodeId,
    pub destination: LatchNodeId,
    pub max_delay: f64,
    pub min_delay: f64,
    pub memory_elements: i32,
    pub weight: f64,
}

impl LatchEdge {
    pub fn new(
        source: LatchNodeId,
        destination: LatchNodeId,
        max_delay: f64,
        min_delay: f64,
        memory_elements: i32,
    ) -> Self {
        Self {
            source,
            destination,
            max_delay,
            min_delay,
            memory_elements,
            weight: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatchGraph {
    pub nodes: Vec<LatchNode>,
    pub edges: Vec<LatchEdge>,
    pub host: Option<LatchNodeId>,
    pub phase_count: usize,
    pub min_phase_separation: f64,
    pub max_phase_separation: f64,
    pub use_general_algorithm: bool,
    pub phase_inversion: bool,
}

impl LatchGraph {
    pub fn new(phase_count: usize) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            host: None,
            phase_count,
            min_phase_separation: 0.0,
            max_phase_separation: 1.0,
            use_general_algorithm: false,
            phase_inversion: false,
        }
    }

    pub fn add_node(&mut self, node: LatchNode) -> LatchNodeId {
        let id = LatchNodeId(self.nodes.len());
        if node.kind == LatchNodeKind::NetworkIo && self.host.is_none() {
            self.host = Some(id);
        }
        self.nodes.push(node);
        id
    }

    pub fn add_edge(
        &mut self,
        source: LatchNodeId,
        destination: LatchNodeId,
        max_delay: f64,
        min_delay: f64,
        memory_elements: i32,
    ) -> Result<(), TimingCompError> {
        self.node(source)?;
        self.node(destination)?;
        self.edges.push(LatchEdge::new(
            source,
            destination,
            max_delay,
            min_delay,
            memory_elements,
        ));
        Ok(())
    }

    pub fn node(&self, id: LatchNodeId) -> Result<&LatchNode, TimingCompError> {
        self.nodes
            .get(id.0)
            .ok_or(TimingCompError::UnknownLatchNode(id))
    }

    fn node_mut(&mut self, id: LatchNodeId) -> Result<&mut LatchNode, TimingCompError> {
        self.nodes
            .get_mut(id.0)
            .ok_or(TimingCompError::UnknownLatchNode(id))
    }

    fn event_index(
        &self,
        id: LatchNodeId,
        endpoint: ConstraintEndpoint,
    ) -> Result<usize, TimingCompError> {
        let node = self.node(id)?;
        if node.kind != LatchNodeKind::Latch {
            return Err(TimingCompError::NonLatchConstraintNode(id));
        }
        if node.phase >= self.phase_count {
            return Err(TimingCompError::InvalidPhase {
                node: id,
                phase: node.phase,
                phase_count: self.phase_count,
            });
        }
        Ok(match endpoint {
            ConstraintEndpoint::From => node.latch_type.from_event(node.phase, self.phase_count),
            ConstraintEndpoint::To => node.latch_type.to_event(node.phase, self.phase_count),
        })
    }

    fn host_or_first(&self) -> Result<LatchNodeId, TimingCompError> {
        self.host
            .or_else(|| (!self.nodes.is_empty()).then_some(LatchNodeId(0)))
            .ok_or(TimingCompError::EmptyGraph)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConstraintEndpoint {
    From,
    To,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintNode {
    pub id: i32,
    pub position: f64,
    matrix_id: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintEdge {
    pub source: ConstraintNodeId,
    pub destination: ConstraintNodeId,
    pub ignore: bool,
    pub linear_constraints: Vec<(i32, f64)>,
    pub short_path_constraints: [Option<f64>; 2],
    pub weight: f64,
    pub fixed_weight: Option<f64>,
    pub duty: Option<f64>,
}

impl ConstraintEdge {
    fn new(source: ConstraintNodeId, destination: ConstraintNodeId) -> Self {
        Self {
            source,
            destination,
            ignore: false,
            linear_constraints: Vec::new(),
            short_path_constraints: [None, None],
            weight: INFTY,
            fixed_weight: None,
            duty: None,
        }
    }

    pub fn set_fixed_max(&mut self, value: f64) {
        self.fixed_weight = Some(
            self.fixed_weight
                .map_or(value, |current| current.min(value)),
        );
    }

    pub fn set_duty_max(&mut self, value: f64) {
        self.duty = Some(self.duty.map_or(value, |current| current.min(value)));
    }

    pub fn add_linear_constraint(&mut self, cycles: i32, value: f64) {
        if let Some((_, existing)) = self
            .linear_constraints
            .iter_mut()
            .find(|(existing_cycles, _)| *existing_cycles == cycles)
        {
            *existing = existing.max(value);
        } else {
            self.linear_constraints.push((cycles, value));
        }
    }

    pub fn add_short_path_constraint(&mut self, cycles: usize, value: f64) {
        let slot = &mut self.short_path_constraints[cycles];
        *slot = Some(slot.map_or(value, |current| current.min(value)));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Phase {
    pub rise: ConstraintNodeId,
    pub fall: ConstraintNodeId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintGraph {
    pub phase_count: usize,
    pub phases: Vec<Phase>,
    pub zero: ConstraintNodeId,
    pub final_event: ConstraintNodeId,
    pub nodes: Vec<ConstraintNode>,
    pub edges: Vec<ConstraintEdge>,
}

impl ConstraintGraph {
    pub fn new(phase_count: usize) -> Self {
        let mut nodes = Vec::with_capacity(2 * phase_count + 1);
        let mut phases = Vec::with_capacity(phase_count);

        for index in 0..phase_count {
            let rise = ConstraintNodeId(nodes.len());
            nodes.push(ConstraintNode {
                id: (index + 1) as i32,
                position: INFTY,
                matrix_id: 0,
            });
            let fall = ConstraintNodeId(nodes.len());
            nodes.push(ConstraintNode {
                id: -((index + 1) as i32),
                position: INFTY,
                matrix_id: 0,
            });
            phases.push(Phase { rise, fall });
        }

        let zero = ConstraintNodeId(nodes.len());
        nodes.push(ConstraintNode {
            id: 0,
            position: 0.0,
            matrix_id: 0,
        });

        let final_event = phases.last().map(|phase| phase.fall).unwrap_or(zero);

        Self {
            phase_count,
            phases,
            zero,
            final_event,
            nodes,
            edges: Vec::new(),
        }
    }

    pub fn node(&self, id: ConstraintNodeId) -> Result<&ConstraintNode, TimingCompError> {
        self.nodes
            .get(id.0)
            .ok_or(TimingCompError::UnknownConstraintNode(id))
    }

    pub fn node_mut(
        &mut self,
        id: ConstraintNodeId,
    ) -> Result<&mut ConstraintNode, TimingCompError> {
        self.nodes
            .get_mut(id.0)
            .ok_or(TimingCompError::UnknownConstraintNode(id))
    }

    pub fn event_from_index(&self, index: usize) -> Result<ConstraintNodeId, TimingCompError> {
        if index < self.phase_count {
            Ok(self.phases[index].rise)
        } else {
            self.phases
                .get(index - self.phase_count)
                .map(|phase| phase.fall)
                .ok_or(TimingCompError::UnknownEventIndex(index))
        }
    }

    pub fn get_or_add_edge(
        &mut self,
        source: ConstraintNodeId,
        destination: ConstraintNodeId,
    ) -> Result<ConstraintEdgeId, TimingCompError> {
        self.node(source)?;
        self.node(destination)?;

        if let Some(index) = self
            .edges
            .iter()
            .position(|edge| edge.source == source && edge.destination == destination)
        {
            return Ok(ConstraintEdgeId(index));
        }

        let id = ConstraintEdgeId(self.edges.len());
        self.edges.push(ConstraintEdge::new(source, destination));
        Ok(id)
    }

    pub fn edge_mut(
        &mut self,
        id: ConstraintEdgeId,
    ) -> Result<&mut ConstraintEdge, TimingCompError> {
        self.edges
            .get_mut(id.0)
            .ok_or(TimingCompError::UnknownConstraintEdge(id))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockNetwork {
    pub cycle_time: f64,
    pub clocks: Vec<ClockPhasePosition>,
}

impl ClockNetwork {
    pub fn new(phase_count: usize) -> Self {
        Self {
            cycle_time: 0.0,
            clocks: vec![ClockPhasePosition::default(); phase_count],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClockPhasePosition {
    pub rise: f64,
    pub fall: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptimalClockResult {
    pub cycle_time: f64,
    pub clock_graph: ConstraintGraph,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimingCompError {
    EmptyGraph,
    UnknownLatchNode(LatchNodeId),
    UnknownConstraintNode(ConstraintNodeId),
    UnknownConstraintEdge(ConstraintEdgeId),
    UnknownEventIndex(usize),
    InvalidPhase {
        node: LatchNodeId,
        phase: usize,
        phase_count: usize,
    },
    NonLatchConstraintNode(LatchNodeId),
    ClockNetworkPhaseMismatch {
        expected: usize,
        actual: usize,
    },
    Infeasible,
}

impl fmt::Display for TimingCompError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph => write!(f, "timing graph is empty"),
            Self::UnknownLatchNode(id) => write!(f, "unknown latch graph node {:?}", id),
            Self::UnknownConstraintNode(id) => write!(f, "unknown constraint graph node {:?}", id),
            Self::UnknownConstraintEdge(id) => write!(f, "unknown constraint graph edge {:?}", id),
            Self::UnknownEventIndex(index) => write!(f, "unknown constraint event index {index}"),
            Self::InvalidPhase {
                node,
                phase,
                phase_count,
            } => write!(
                f,
                "latch node {:?} has phase {phase}, but graph has {phase_count} phases",
                node
            ),
            Self::NonLatchConstraintNode(id) => {
                write!(f, "latch node {:?} does not map to a clock event", id)
            }
            Self::ClockNetworkPhaseMismatch { expected, actual } => write!(
                f,
                "clock network has {actual} phases, but constraint graph has {expected}"
            ),
            Self::Infeasible => write!(f, "clock constraints are infeasible"),
        }
    }
}

impl Error for TimingCompError {}

pub fn compute_optimal_clock(
    latch_graph: &mut LatchGraph,
    network: &mut ClockNetwork,
) -> Result<OptimalClockResult, TimingCompError> {
    let clock_lower_bound = clock_lower_bound(latch_graph)?;
    let mut clock_graph = construct_clock_graph(latch_graph, clock_lower_bound)?;
    add_short_path_constraints(&mut clock_graph, latch_graph)?;
    add_model_constraints(&mut clock_graph);
    add_phase_separation_constraints(&mut clock_graph, latch_graph);

    let cycle_time = if latch_graph.use_general_algorithm {
        solve_general_constraints(&mut clock_graph, clock_lower_bound)?
    } else {
        solve_constraints(&mut clock_graph, clock_lower_bound)?
    };

    update_circuit_clock(&clock_graph, network, cycle_time)?;

    Ok(OptimalClockResult {
        cycle_time,
        clock_graph,
    })
}

pub fn clock_lower_bound(graph: &mut LatchGraph) -> Result<f64, TimingCompError> {
    let mut clock_upper_bound = 0.0;
    let mut new_clock = guess_clock_bound(graph)?;
    let mut clock_lower_bound = new_clock;

    loop {
        if all_negative_cycles(graph, new_clock)? {
            clock_upper_bound = new_clock;
        } else {
            clock_lower_bound = new_clock;
        }

        if (clock_upper_bound - clock_lower_bound).abs() < EPS2 {
            break;
        }

        if clock_upper_bound > clock_lower_bound {
            new_clock = (clock_upper_bound + clock_lower_bound) / 2.0;
        } else {
            new_clock *= 2.0;
        }
    }

    Ok(clock_upper_bound)
}

pub fn guess_clock_bound(graph: &mut LatchGraph) -> Result<f64, TimingCompError> {
    for node in &mut graph.nodes {
        node.reachable = -1;
        node.weight = 0.0;
        node.dirty = false;
    }

    let host = graph.host_or_first()?;
    graph.node_mut(host)?.reachable = 0;

    let mut cycle_bound: f64 = 0.0;
    let mut path_bound: f64 = 0.0;
    guess_clock_bound_recursive(graph, host, &mut cycle_bound, &mut path_bound)?;

    Ok(cycle_bound.max(path_bound).max(EPS))
}

fn guess_clock_bound_recursive(
    graph: &mut LatchGraph,
    source: LatchNodeId,
    cycle_bound: &mut f64,
    path_bound: &mut f64,
) -> Result<(), TimingCompError> {
    let source_reachable = graph.node(source)?.reachable;
    let source_weight = graph.node(source)?.weight;
    *path_bound = path_bound.max(source_weight / f64::from(source_reachable + 1));
    graph.node_mut(source)?.dirty = true;

    let outgoing = graph
        .edges
        .iter()
        .filter(|edge| edge.source == source)
        .cloned()
        .collect::<Vec<_>>();

    for edge in outgoing {
        let destination = edge.destination;
        if graph.node(destination)?.dirty {
            let destination_reachable = graph.node(destination)?.reachable;
            if destination_reachable >= 0 {
                let denominator = source_reachable + edge.memory_elements - destination_reachable;
                if denominator > 0 {
                    let destination_weight = graph.node(destination)?.weight;
                    *cycle_bound = cycle_bound.max(
                        (source_weight + edge.max_delay - destination_weight)
                            / f64::from(denominator),
                    );
                }
            }
        } else {
            let node = graph.node_mut(destination)?;
            node.reachable = source_reachable + edge.memory_elements;
            node.weight = source_weight + edge.max_delay;
            guess_clock_bound_recursive(graph, destination, cycle_bound, path_bound)?;
        }
    }

    graph.node_mut(source)?.reachable = -1;
    Ok(())
}

pub fn all_negative_cycles(
    graph: &mut LatchGraph,
    cycle_time: f64,
) -> Result<bool, TimingCompError> {
    graph.host_or_first()?;

    for edge in &mut graph.edges {
        edge.weight = edge.max_delay
            - if edge.memory_elements == 0 {
                0.0
            } else {
                cycle_time
            };
    }

    for node in &mut graph.nodes {
        node.weight = -INFTY;
    }

    if let Some(host) = graph.host {
        graph.node_mut(host)?.weight = 0.0;
    }

    for node in &mut graph.nodes {
        if node.kind == LatchNodeKind::Latch && node.phase == 0 {
            node.weight = 0.0;
        }
    }

    for _ in 0..graph.nodes.len() {
        let mut converged = true;

        for edge in graph.edges.clone() {
            let source_weight = graph.node(edge.source)?.weight;
            if source_weight <= -INFTY {
                continue;
            }

            let value = source_weight + edge.weight;
            if value > graph.node(edge.destination)?.weight {
                converged = false;
                if value > cycle_time + EPS {
                    return Ok(false);
                }
                graph.node_mut(edge.destination)?.weight = value;
            }
        }

        if converged {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn construct_clock_graph(
    latch_graph: &mut LatchGraph,
    clock_lower_bound: f64,
) -> Result<ConstraintGraph, TimingCompError> {
    let mut clock_graph = ConstraintGraph::new(latch_graph.phase_count);
    let mut starts_by_event = vec![Vec::new(); 2 * latch_graph.phase_count];

    for (index, node) in latch_graph.nodes.iter().enumerate() {
        if node.kind == LatchNodeKind::Latch {
            let latch_id = LatchNodeId(index);
            let event_index = latch_graph.event_index(latch_id, ConstraintEndpoint::From)?;
            starts_by_event[event_index].push(latch_id);
        }
    }

    let red_edges = latch_graph
        .edges
        .iter()
        .filter(|edge| edge.memory_elements != 0)
        .cloned()
        .collect::<Vec<_>>();
    let black_edges = latch_graph
        .edges
        .iter()
        .filter(|edge| edge.memory_elements == 0)
        .cloned()
        .collect::<Vec<_>>();

    for (event_index, starts) in starts_by_event.into_iter().enumerate() {
        if starts.is_empty() {
            continue;
        }

        reset_constraint_search(latch_graph);
        let mut event = clock_graph.event_from_index(event_index)?;
        if latch_graph.phase_inversion && event_index == 1 {
            event = clock_graph.zero;
        }
        gen_constraints_from(
            latch_graph,
            &mut clock_graph,
            &starts,
            event,
            clock_lower_bound,
            &red_edges,
            &black_edges,
        )?;
    }

    if let Some(host) = latch_graph.host {
        reset_constraint_search(latch_graph);
        let zero = clock_graph.zero;
        gen_constraints_from(
            latch_graph,
            &mut clock_graph,
            &[host],
            zero,
            clock_lower_bound,
            &red_edges,
            &black_edges,
        )?;
    }

    Ok(clock_graph)
}

fn gen_constraints_from(
    latch_graph: &mut LatchGraph,
    clock_graph: &mut ConstraintGraph,
    starts: &[LatchNodeId],
    source_event: ConstraintNodeId,
    clock_lower_bound: f64,
    red_edges: &[LatchEdge],
    black_edges: &[LatchEdge],
) -> Result<(), TimingCompError> {
    let mut cached_edges = vec![None; 2 * latch_graph.phase_count];

    for start in starts {
        let node = latch_graph.node_mut(*start)?;
        node.weight = if node.kind == LatchNodeKind::Latch {
            node.max_clock_skew
        } else {
            0.0
        };
        node.reachable = 0;
        node.dirty = true;
    }

    let mut current_key = 0;
    loop {
        let mut more_to_come = false;

        for node in &mut latch_graph.nodes {
            if node.reachable == current_key {
                node.previous_weight = node.weight;
            }
        }

        for _ in 0..latch_graph.nodes.len() {
            let mut local_converged = true;

            for edge in black_edges {
                if latch_graph.node(edge.destination)?.kind == LatchNodeKind::NetworkNode {
                    continue;
                }
                let source = latch_graph.node(edge.source)?;
                let source_kind = source.kind;
                let source_weight = source.weight;
                let source_reachable = source.reachable;
                if source_weight <= -INFTY {
                    continue;
                }
                if source_kind == LatchNodeKind::NetworkNode && source_weight < 0.0 {
                    continue;
                }

                let destination_adjusted = latch_graph.node(edge.destination)?.weight
                    - f64::from(latch_graph.node(edge.destination)?.reachable) * clock_lower_bound;
                let candidate =
                    source_weight + edge.max_delay - f64::from(current_key) * clock_lower_bound;

                if candidate > destination_adjusted {
                    let node = latch_graph.node_mut(edge.destination)?;
                    node.weight = source_weight + edge.max_delay;
                    node.reachable = source_reachable;
                    node.dirty = true;
                    local_converged = false;
                }
            }

            if local_converged {
                break;
            }
        }

        for index in 0..latch_graph.nodes.len() {
            let latch_id = LatchNodeId(index);
            let node = latch_graph.node(latch_id)?;
            if node.kind == LatchNodeKind::NetworkNode
                || !node.dirty
                || node.reachable != current_key
            {
                continue;
            }

            if node.kind == LatchNodeKind::Latch {
                let to_index = latch_graph.event_index(latch_id, ConstraintEndpoint::To)?;
                let edge_id = match cached_edges[to_index] {
                    Some(edge_id) => edge_id,
                    None => {
                        let destination_event = clock_graph.event_from_index(to_index)?;
                        let edge_id =
                            clock_graph.get_or_add_edge(destination_event, source_event)?;
                        cached_edges[to_index] = Some(edge_id);
                        edge_id
                    }
                };

                let value = node.weight + node.setup - node.min_clock_skew;
                clock_graph
                    .edge_mut(edge_id)?
                    .add_linear_constraint(current_key, value);
            }

            latch_graph.node_mut(latch_id)?.previous_weight = node.weight;
        }

        for edge in red_edges {
            let source = latch_graph.node(edge.source)?;
            let source_kind = source.kind;
            let source_latch_type = source.latch_type;
            let source_dirty = source.dirty;
            let source_weight = source.weight;
            let source_previous_weight = source.previous_weight;
            if matches!(
                source_latch_type,
                LatchType::FallingEdgeFlipFlop | LatchType::RisingEdgeFlipFlop
            ) && source_dirty
            {
                continue;
            }
            if latch_graph.node(edge.destination)?.kind == LatchNodeKind::NetworkNode {
                continue;
            }
            if source_kind == LatchNodeKind::NetworkNode && source_weight < 0.0 {
                continue;
            }

            let destination = latch_graph.node(edge.destination)?;
            let candidate = source_previous_weight + edge.max_delay
                - f64::from(current_key + 1) * clock_lower_bound;
            let current = destination.weight - f64::from(destination.reachable) * clock_lower_bound;

            if candidate > current + EPS {
                let node = latch_graph.node_mut(edge.destination)?;
                node.weight = source_previous_weight + edge.max_delay;
                node.reachable = current_key + 1;
                node.dirty = true;
                more_to_come = true;
            }
        }

        if !more_to_come {
            break;
        }

        current_key += 1;
    }

    Ok(())
}

fn reset_constraint_search(graph: &mut LatchGraph) {
    for node in &mut graph.nodes {
        node.weight = -INFTY;
        node.previous_weight = -INFTY;
        node.reachable = 0;
        node.dirty = false;
    }
}

pub fn add_short_path_constraints(
    clock_graph: &mut ConstraintGraph,
    latch_graph: &LatchGraph,
) -> Result<(), TimingCompError> {
    for edge in &latch_graph.edges {
        let from_index = latch_graph.event_index(edge.source, ConstraintEndpoint::From)?;
        let to_index = latch_graph.event_index(edge.destination, ConstraintEndpoint::To)?;
        let from_event = clock_graph.event_from_index(from_index)?;
        let to_event = clock_graph.event_from_index(to_index)?;
        let graph_edge = clock_graph.get_or_add_edge(from_event, to_event)?;
        let destination = latch_graph.node(edge.destination)?;
        let value = -(edge.min_delay - destination.hold + destination.max_clock_skew);
        let cycles = edge.memory_elements.clamp(0, 1) as usize;
        clock_graph
            .edge_mut(graph_edge)?
            .add_short_path_constraint(cycles, value);
    }

    Ok(())
}

pub fn add_model_constraints(clock_graph: &mut ConstraintGraph) {
    let final_event = clock_graph.final_event;
    let zero = clock_graph.zero;

    for index in 0..clock_graph.nodes.len() {
        let node = ConstraintNodeId(index);
        if node == final_event || node == zero {
            continue;
        }

        let lower_edge = clock_graph.get_or_add_edge(node, zero).expect("valid node");
        clock_graph
            .edge_mut(lower_edge)
            .expect("valid edge")
            .set_fixed_max(0.0);

        let upper_edge = clock_graph
            .get_or_add_edge(final_event, node)
            .expect("valid node");
        clock_graph
            .edge_mut(upper_edge)
            .expect("valid edge")
            .set_fixed_max(0.0);
    }

    let final_to_zero = clock_graph
        .get_or_add_edge(zero, final_event)
        .expect("valid final edge");
    clock_graph
        .edge_mut(final_to_zero)
        .expect("valid edge")
        .set_duty_max(1.0);

    let zero_to_final = clock_graph
        .get_or_add_edge(final_event, zero)
        .expect("valid zero edge");
    clock_graph
        .edge_mut(zero_to_final)
        .expect("valid edge")
        .set_duty_max(-1.0);
}

pub fn add_phase_separation_constraints(
    clock_graph: &mut ConstraintGraph,
    latch_graph: &LatchGraph,
) {
    let mut previous_fall = None;

    for phase in clock_graph.phases.clone() {
        let max_edge = clock_graph
            .get_or_add_edge(phase.rise, phase.fall)
            .expect("valid phase edge");
        clock_graph
            .edge_mut(max_edge)
            .expect("valid edge")
            .set_duty_max(latch_graph.max_phase_separation);

        let min_edge = clock_graph
            .get_or_add_edge(phase.fall, phase.rise)
            .expect("valid phase edge");
        clock_graph
            .edge_mut(min_edge)
            .expect("valid edge")
            .set_duty_max(-latch_graph.min_phase_separation);

        if let Some(previous_fall) = previous_fall {
            let order_edge = clock_graph
                .get_or_add_edge(phase.fall, previous_fall)
                .expect("valid order edge");
            clock_graph
                .edge_mut(order_edge)
                .expect("valid edge")
                .set_fixed_max(0.0);
        }

        previous_fall = Some(phase.fall);
    }
}

pub fn solve_constraints(
    clock_graph: &mut ConstraintGraph,
    clock_lower_bound: f64,
) -> Result<f64, TimingCompError> {
    let mut lower = clock_lower_bound;
    let mut upper = INFTY;
    let mut current = lower;

    loop {
        if !is_feasible(clock_graph, current)? {
            lower = current;
            current = if upper == INFTY {
                (2.0 * current).max(EPS)
            } else {
                (lower + upper) / 2.0
            };
        } else {
            if upper - lower < EPS {
                break;
            }
            upper = current;
            current = (upper + lower) / 2.0;
        }
    }

    Ok(current)
}

pub fn is_feasible(
    clock_graph: &mut ConstraintGraph,
    cycle_time: f64,
) -> Result<bool, TimingCompError> {
    if cycle_time == 0.0 {
        return Ok(false);
    }

    evaluate_right_hand_side(clock_graph, cycle_time);

    for node in &mut clock_graph.nodes {
        node.position = INFTY;
    }
    clock_graph.node_mut(clock_graph.zero)?.position = 0.0;

    let mut converged = true;
    for _ in 0..clock_graph.nodes.len() {
        converged = true;

        for edge in clock_graph.edges.clone() {
            if edge.ignore {
                continue;
            }
            let source = clock_graph.node(edge.source)?.position;
            let destination = clock_graph.node(edge.destination)?.position;

            if destination > source + edge.weight {
                clock_graph.node_mut(edge.destination)?.position = source + edge.weight;
                converged = false;
            }
        }

        if converged {
            break;
        }
    }

    Ok(converged)
}

pub fn solve_general_constraints(
    clock_graph: &mut ConstraintGraph,
    clock_lower_bound: f64,
) -> Result<f64, TimingCompError> {
    let clock = exterior_path_search(clock_graph, clock_lower_bound, INFTY)?;
    if !is_feasible(clock_graph, clock)? {
        return Err(TimingCompError::Infeasible);
    }
    Ok(clock)
}

pub fn exterior_path_search(
    clock_graph: &mut ConstraintGraph,
    lower_bound: f64,
    _upper_bound: f64,
) -> Result<f64, TimingCompError> {
    let mut matrix = SearchMatrix::new(clock_graph, lower_bound);

    loop {
        let cycle_time = matrix.current_clock;
        evaluate_matrix(clock_graph, &mut matrix, cycle_time);

        let mut status = ExteriorPathStatus::AllPositiveCycles;
        let num_nodes = matrix.node_count;

        for _ in 0..num_nodes {
            for i in 0..num_nodes {
                for j in 0..num_nodes {
                    matrix.weight_new[i][j] = matrix.weight_old[i][j];
                    matrix.beta_new[i][j] = matrix.beta_old[i][j];

                    for k in 0..num_nodes {
                        if k == i || k == j {
                            continue;
                        }
                        if matrix.weight_old[i][k] == INFTY || matrix.weight_old[k][j] == INFTY {
                            continue;
                        }

                        let value = matrix.weight_old[i][k] + matrix.weight_old[k][j];
                        if value < matrix.weight_new[i][j] {
                            matrix.weight_new[i][j] = value;
                            matrix.beta_new[i][j] = matrix.beta_old[i][k] + matrix.beta_old[k][j];
                        }
                    }
                }
            }

            status = update_matrix(&mut matrix);
            if status != ExteriorPathStatus::AllPositiveCycles {
                break;
            }
        }

        match status {
            ExteriorPathStatus::NegativeCycle => continue,
            ExteriorPathStatus::Infeasible => return Err(TimingCompError::Infeasible),
            ExteriorPathStatus::AllPositiveCycles => return Ok(cycle_time),
        }
    }
}

pub fn update_circuit_clock(
    clock_graph: &ConstraintGraph,
    network: &mut ClockNetwork,
    cycle_time: f64,
) -> Result<(), TimingCompError> {
    if network.clocks.len() != clock_graph.phase_count {
        return Err(TimingCompError::ClockNetworkPhaseMismatch {
            expected: clock_graph.phase_count,
            actual: network.clocks.len(),
        });
    }

    network.cycle_time = cycle_time;

    for node in &clock_graph.nodes {
        if node.id > 0 {
            network.clocks[(node.id - 1) as usize].rise = node.position;
        } else if node.id < 0 {
            network.clocks[(-node.id - 1) as usize].fall = node.position;
        }
    }

    Ok(())
}

pub fn evaluate_right_hand_side(clock_graph: &mut ConstraintGraph, cycle_time: f64) {
    for edge in &mut clock_graph.edges {
        edge.weight = edge.fixed_weight.unwrap_or(INFTY);

        if let Some(duty) = edge.duty {
            edge.weight = edge.weight.min(duty * cycle_time);
        }

        for (cycles, value) in &edge.linear_constraints {
            edge.weight = edge.weight.min(-*value + f64::from(*cycles) * cycle_time);
        }

        for (cycles, value) in edge.short_path_constraints.iter().enumerate() {
            if let Some(value) = value {
                edge.weight = edge.weight.min(*value + cycles as f64 * cycle_time);
            }
        }

        edge.ignore = edge.weight == INFTY;
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SearchMatrix {
    weight_old: Vec<Vec<f64>>,
    weight_new: Vec<Vec<f64>>,
    beta_old: Vec<Vec<f64>>,
    beta_new: Vec<Vec<f64>>,
    current_clock: f64,
    lower_bound: f64,
    upper_bound: f64,
    node_count: usize,
}

impl SearchMatrix {
    fn new(clock_graph: &mut ConstraintGraph, cycle_time: f64) -> Self {
        for (index, node) in clock_graph.nodes.iter_mut().enumerate() {
            node.matrix_id = index;
        }

        let node_count = clock_graph.nodes.len();
        Self {
            weight_old: vec![vec![INFTY; node_count]; node_count],
            weight_new: vec![vec![INFTY; node_count]; node_count],
            beta_old: vec![vec![INFTY; node_count]; node_count],
            beta_new: vec![vec![INFTY; node_count]; node_count],
            current_clock: cycle_time,
            lower_bound: -INFTY,
            upper_bound: INFTY,
            node_count,
        }
    }

    fn clear_paths(&mut self) {
        for i in 0..self.node_count {
            for j in 0..self.node_count {
                self.weight_old[i][j] = INFTY;
                self.weight_new[i][j] = INFTY;
                self.beta_old[i][j] = INFTY;
                self.beta_new[i][j] = INFTY;
            }
        }
    }
}

fn evaluate_matrix(clock_graph: &ConstraintGraph, matrix: &mut SearchMatrix, cycle_time: f64) {
    matrix.clear_paths();

    for edge in &clock_graph.edges {
        let i = clock_graph.nodes[edge.source.0].matrix_id;
        let j = clock_graph.nodes[edge.destination.0].matrix_id;

        if let Some(fixed) = edge.fixed_weight {
            matrix.weight_old[i][j] = fixed;
            matrix.beta_old[i][j] = 0.0;
        }

        if let Some(duty) = edge.duty {
            let value = duty * cycle_time;
            if value < matrix.weight_old[i][j] {
                matrix.weight_old[i][j] = value;
                matrix.beta_old[i][j] = duty;
            }
        }

        for (cycles, value) in &edge.linear_constraints {
            let candidate = -*value + f64::from(*cycles) * cycle_time;
            if candidate < matrix.weight_old[i][j] {
                matrix.weight_old[i][j] = candidate;
                matrix.beta_old[i][j] = f64::from(*cycles);
            }
        }

        for (cycles, value) in edge.short_path_constraints.iter().enumerate() {
            if let Some(value) = value {
                let candidate = *value + cycles as f64 * cycle_time;
                if candidate < matrix.weight_old[i][j] {
                    matrix.weight_old[i][j] = candidate;
                    matrix.beta_old[i][j] = cycles as f64;
                }
            }
        }
    }
}

fn update_matrix(matrix: &mut SearchMatrix) -> ExteriorPathStatus {
    let mut saw_negative_cycle = false;
    let cycle_time = matrix.current_clock;

    for i in 0..matrix.node_count {
        for j in 0..matrix.node_count {
            if i == j {
                if matrix.weight_new[i][j] < -EPS1 {
                    if matrix.beta_new[i][i] <= 0.0 {
                        return ExteriorPathStatus::Infeasible;
                    }

                    matrix.lower_bound = matrix
                        .lower_bound
                        .max((-matrix.weight_new[i][i] / matrix.beta_new[i][i]) + cycle_time);
                    saw_negative_cycle = true;
                } else if matrix.weight_new[i][i] >= 0.0
                    && matrix.weight_new[i][i] < INFTY
                    && matrix.beta_new[i][i] < 0.0
                {
                    matrix.upper_bound = matrix
                        .upper_bound
                        .min((matrix.weight_new[i][i] / -matrix.beta_new[i][i]) + cycle_time);
                }
                matrix.weight_old[i][j] = matrix.weight_new[i][j];
                matrix.beta_old[i][j] = matrix.beta_new[i][j];
            } else if matrix.weight_new[i][j] < INFTY {
                matrix.weight_old[i][j] = matrix.weight_new[i][j];
                matrix.beta_old[i][j] = matrix.beta_new[i][j];
            }
        }
    }

    if saw_negative_cycle {
        if matrix.lower_bound > matrix.upper_bound {
            return ExteriorPathStatus::Infeasible;
        }

        matrix.current_clock = matrix.lower_bound;
        matrix.clear_paths();
        return ExteriorPathStatus::NegativeCycle;
    }

    ExteriorPathStatus::AllPositiveCycles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_timing_int_h() {
        assert_eq!(EPS, 1.0e-3);
        assert_eq!(EPS1, 1.0e-5);
        assert_eq!(EPS2, 1.0e-2);
        assert_eq!(INFTY, 10_000.0);
        assert_eq!(INFINITY, INFTY);
        assert_eq!(NOT_SET, -2);
    }

    #[test]
    fn latch_type_event_mapping_matches_c_macros() {
        assert_eq!(LatchType::RisingEdgeFlipFlop.from_event(0, 3), 0);
        assert_eq!(LatchType::ActiveHighLevelSensitive.from_event(1, 3), 1);
        assert_eq!(LatchType::FallingEdgeFlipFlop.from_event(0, 3), 3);
        assert_eq!(LatchType::ActiveLowLevelSensitive.from_event(2, 3), 5);
        assert_eq!(LatchType::RisingEdgeFlipFlop.to_event(0, 3), 0);
        assert_eq!(LatchType::ActiveLowLevelSensitive.to_event(1, 3), 1);
        assert_eq!(LatchType::FallingEdgeFlipFlop.to_event(0, 3), 3);
        assert_eq!(LatchType::ActiveHighLevelSensitive.to_event(2, 3), 5);
    }

    #[test]
    fn all_negative_cycles_detects_clock_that_is_too_small() {
        let mut graph = LatchGraph::new(1);
        let latch = graph.add_node(LatchNode::latch(LatchType::FallingEdgeFlipFlop, 0));
        graph.add_edge(latch, latch, 6.0, 1.0, 1).unwrap();

        assert!(!all_negative_cycles(&mut graph, 5.0).unwrap());
        assert!(all_negative_cycles(&mut graph, 6.1).unwrap());
    }

    #[test]
    fn clock_lower_bound_uses_lawler_binary_search() {
        let mut graph = LatchGraph::new(1);
        let latch = graph.add_node(LatchNode::latch(LatchType::FallingEdgeFlipFlop, 0));
        graph.add_edge(latch, latch, 6.0, 1.0, 1).unwrap();

        let bound = clock_lower_bound(&mut graph).unwrap();

        assert!((bound - 6.0).abs() < EPS2);
    }

    #[test]
    fn right_hand_side_uses_minimum_active_constraint() {
        let mut graph = ConstraintGraph::new(1);
        let edge = graph
            .get_or_add_edge(graph.phases[0].rise, graph.zero)
            .unwrap();
        let edge = graph.edge_mut(edge).unwrap();
        edge.set_fixed_max(8.0);
        edge.set_duty_max(0.5);
        edge.add_linear_constraint(1, 3.0);
        edge.add_short_path_constraint(0, 2.0);

        evaluate_right_hand_side(&mut graph, 10.0);

        assert_eq!(graph.edges[0].weight, 2.0);
        assert!(!graph.edges[0].ignore);
    }

    #[test]
    fn feasibility_detects_negative_cycle() {
        let mut graph = ConstraintGraph::new(1);
        let a = graph.phases[0].rise;
        let b = graph.phases[0].fall;
        let ab = graph.get_or_add_edge(a, b).unwrap();
        graph.edge_mut(ab).unwrap().set_fixed_max(-2.0);
        let ba = graph.get_or_add_edge(b, a).unwrap();
        graph.edge_mut(ba).unwrap().set_fixed_max(1.0);

        assert!(!is_feasible(&mut graph, 1.0).unwrap());
    }

    #[test]
    fn solve_constraints_finds_minimum_feasible_cycle() {
        let mut graph = ConstraintGraph::new(1);
        let edge = graph.get_or_add_edge(graph.zero, graph.zero).unwrap();
        graph.edge_mut(edge).unwrap().add_linear_constraint(1, 5.0);

        let cycle = solve_constraints(&mut graph, 1.0).unwrap();

        assert!((cycle - 5.0).abs() < EPS);
    }

    #[test]
    fn model_and_phase_constraints_are_added() {
        let mut latch_graph = LatchGraph::new(2);
        latch_graph.min_phase_separation = 0.25;
        latch_graph.max_phase_separation = 0.75;
        let mut graph = ConstraintGraph::new(2);

        add_model_constraints(&mut graph);
        add_phase_separation_constraints(&mut graph, &latch_graph);

        assert!(graph.edges.iter().any(|edge| edge.duty == Some(1.0)));
        assert!(graph.edges.iter().any(|edge| edge.duty == Some(-1.0)));
        assert!(graph.edges.iter().any(|edge| edge.duty == Some(0.75)));
        assert!(graph.edges.iter().any(|edge| edge.duty == Some(-0.25)));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.fixed_weight == Some(0.0)));
    }

    #[test]
    fn compute_optimal_clock_updates_network_positions() {
        let mut graph = LatchGraph::new(1);
        graph.min_phase_separation = 0.0;
        graph.max_phase_separation = 1.0;
        let latch = graph.add_node(
            LatchNode::latch(LatchType::FallingEdgeFlipFlop, 0)
                .with_timing(0.0, 0.0)
                .with_clock_skew(0.0, 0.0),
        );
        graph.add_edge(latch, latch, 4.0, 1.0, 1).unwrap();

        let mut network = ClockNetwork::new(1);
        let result = compute_optimal_clock(&mut graph, &mut network).unwrap();

        assert!(result.cycle_time >= 4.0 - EPS2);
        assert_eq!(network.cycle_time, result.cycle_time);
        assert!(network.clocks[0].fall.is_finite());
    }

    #[test]
    fn general_exterior_path_search_raises_lower_bound_for_negative_cycle() {
        let mut graph = ConstraintGraph::new(1);
        let edge = graph.get_or_add_edge(graph.zero, graph.zero).unwrap();
        graph.edge_mut(edge).unwrap().add_linear_constraint(1, 5.0);

        let cycle = exterior_path_search(&mut graph, 1.0, INFTY).unwrap();

        assert!((cycle - 5.0).abs() < EPS);
    }
}
