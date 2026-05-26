//! Native Rust port scaffold for `sis/timing/timing_graph.c`.
//!
//! The C source builds a latch-to-latch timing graph by walking SIS
//! `network_t`, `node_t`, `latch_t`, `graph_t`, and delay metadata. Those
//! dependencies are still C-only in this repository, so the network traversal
//! entry points report that missing dependency instead of exposing a legacy C
//! ABI shim. The graph data model, path-delay edge merge behavior, edge `K`
//! update logic, latch graph formatting, and arrival-time arithmetic are ported
//! as native Rust APIs for use by later timing ports.

use std::error::Error;
use std::fmt;

pub const EPS: f64 = 1.0e-3;
pub const EPS1: f64 = 1.0e-5;
pub const EPS2: f64 = 1.0e-2;
pub const INFTY: f64 = 10_000.0;
pub const NOT_SET: i32 = -2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlgorithmType {
    OptimalClock,
    ClockVerify,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    Library,
    UnitFanout,
    Mapped,
    Unknown,
    Tdc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinPhase {
    NotGiven,
    Inverting,
    NonInverting,
    Neither,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexKind {
    NetworkIo,
    Latch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchType {
    Fff,
    Ffr,
    Lsh,
    Lsl,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClockId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VertexId(pub usize);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayPin {
    pub drive: DelayTime,
    pub block: DelayTime,
    pub phase: PinPhase,
}

impl DelayPin {
    pub fn new(drive: DelayTime, block: DelayTime, phase: PinPhase) -> Self {
        Self {
            drive,
            block,
            phase,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimingVertex {
    pub id: usize,
    pub kind: VertexKind,
    pub latch_type: Option<LatchType>,
    pub phase: Option<usize>,
    pub clock: Option<ClockId>,
    pub latch_input_name: Option<String>,
    pub latch_output_name: Option<String>,
}

impl TimingVertex {
    pub fn network_io() -> Self {
        Self {
            id: 0,
            kind: VertexKind::NetworkIo,
            latch_type: None,
            phase: None,
            clock: None,
            latch_input_name: None,
            latch_output_name: None,
        }
    }

    pub fn latch(
        id: usize,
        latch_type: LatchType,
        phase: usize,
        clock: ClockId,
        input_name: impl Into<String>,
        output_name: impl Into<String>,
    ) -> Self {
        Self {
            id,
            kind: VertexKind::Latch,
            latch_type: Some(latch_type),
            phase: Some(phase),
            clock: Some(clock),
            latch_input_name: Some(input_name.into()),
            latch_output_name: Some(output_name.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimingEdge {
    pub from: VertexId,
    pub to: VertexId,
    pub max_delay: f64,
    pub min_delay: f64,
    pub k: i32,
    pub weight: f64,
}

impl TimingEdge {
    pub fn new(from: VertexId, to: VertexId) -> Self {
        Self {
            from,
            to,
            max_delay: -INFTY,
            min_delay: INFTY,
            k: 0,
            weight: 0.0,
        }
    }

    pub fn merge_path_delays(
        &mut self,
        max_rise: f64,
        max_fall: f64,
        min_rise: f64,
        min_fall: f64,
    ) {
        self.max_delay = self.max_delay.max(max_rise).max(max_fall);
        self.min_delay = self.min_delay.min(min_rise).min(min_fall);
        assert!(
            self.max_delay >= self.min_delay,
            "merged timing edge has max delay smaller than min delay"
        );
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimingGraph {
    pub algorithm: AlgorithmType,
    pub clock_order: Vec<ClockId>,
    pub host: Option<VertexId>,
    vertices: Vec<TimingVertex>,
    edges: Vec<TimingEdge>,
}

impl TimingGraph {
    pub fn new(algorithm: AlgorithmType, clock_order: Vec<ClockId>) -> Self {
        Self {
            algorithm,
            clock_order,
            host: None,
            vertices: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn vertices(&self) -> &[TimingVertex] {
        &self.vertices
    }

    pub fn edges(&self) -> &[TimingEdge] {
        &self.edges
    }

    pub fn add_host(&mut self) -> VertexId {
        if let Some(host) = self.host {
            return host;
        }

        let vertex_id = self.push_vertex(TimingVertex::network_io());
        self.host = Some(vertex_id);
        vertex_id
    }

    pub fn add_latch(
        &mut self,
        latch_type: LatchType,
        phase: usize,
        clock: ClockId,
        input_name: impl Into<String>,
        output_name: impl Into<String>,
    ) -> VertexId {
        let vertex = TimingVertex::latch(
            self.next_latch_number(),
            latch_type,
            phase,
            clock,
            input_name,
            output_name,
        );
        self.push_vertex(vertex)
    }

    pub fn add_or_merge_edge(
        &mut self,
        from: VertexId,
        to: VertexId,
        max_rise: f64,
        max_fall: f64,
        min_rise: f64,
        min_fall: f64,
    ) -> Result<(), TimingGraphError> {
        self.validate_vertex(from)?;
        self.validate_vertex(to)?;

        let edge_index = self
            .edges
            .iter()
            .position(|edge| edge.from == from && edge.to == to);

        let edge = match edge_index {
            Some(index) => &mut self.edges[index],
            None => {
                self.edges.push(TimingEdge::new(from, to));
                self.edges.last_mut().expect("edge was just pushed")
            }
        };

        edge.merge_path_delays(max_rise, max_fall, min_rise, min_fall);
        Ok(())
    }

    pub fn update_k_edges(&mut self) -> Result<(), TimingGraphError> {
        let mut updates = Vec::with_capacity(self.edges.len());

        for edge in &self.edges {
            let from = self.vertex(edge.from)?;
            let to = self.vertex(edge.to)?;
            updates.push(compute_edge_k(from, to, &self.clock_order)?);
        }

        for (edge, k) in self.edges.iter_mut().zip(updates) {
            edge.k = k;
        }

        Ok(())
    }

    pub fn format_latch_graph(&self) -> String {
        let mut output = String::new();

        for (index, vertex) in self.vertices.iter().enumerate() {
            let vertex_id = VertexId(index);

            for edge in self.edges.iter().filter(|edge| edge.to == vertex_id) {
                let from = &self.vertices[edge.from.0];
                if from.kind == VertexKind::NetworkIo {
                    output.push_str(&format!(
                        "PI ->({}- {}) {}\n",
                        edge.min_delay, edge.max_delay, edge.k
                    ));
                } else {
                    output.push_str(&format!(
                        "{} ->({} - {}) {}\n",
                        from.id, edge.min_delay, edge.max_delay, edge.k
                    ));
                }
            }

            if vertex.kind == VertexKind::NetworkIo {
                output.push_str("HOST ->\n");
            } else {
                output.push_str(&format!(
                    " {}  (phase {})\n",
                    vertex.id,
                    vertex.phase.unwrap_or_default()
                ));
            }

            for edge in self.edges.iter().filter(|edge| edge.from == vertex_id) {
                let to = &self.vertices[edge.to.0];
                if to.kind == VertexKind::NetworkIo {
                    output.push_str(&format!(
                        "({} - {})->PO {}\n",
                        edge.min_delay, edge.max_delay, edge.k
                    ));
                } else {
                    output.push_str(&format!(
                        "({} - {})->{} {}\n",
                        edge.min_delay, edge.max_delay, to.id, edge.k
                    ));
                }
            }
        }

        output
    }

    fn push_vertex(&mut self, vertex: TimingVertex) -> VertexId {
        let id = VertexId(self.vertices.len());
        self.vertices.push(vertex);
        id
    }

    fn vertex(&self, id: VertexId) -> Result<&TimingVertex, TimingGraphError> {
        self.vertices
            .get(id.0)
            .ok_or(TimingGraphError::UnknownVertex(id))
    }

    fn validate_vertex(&self, id: VertexId) -> Result<(), TimingGraphError> {
        self.vertex(id).map(|_| ())
    }

    fn next_latch_number(&self) -> usize {
        self.vertices
            .iter()
            .filter(|vertex| vertex.kind == VertexKind::Latch)
            .map(|vertex| vertex.id)
            .max()
            .unwrap_or(0)
            + 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimingGraphError {
    MissingNetworkPort,
    MissingNetworkTraversalPort,
    MissingDelayPort,
    MissingClock(VertexId),
    MissingLatchType(VertexId),
    UnknownVertex(VertexId),
    ClockNotInOrder { from: ClockId, to: ClockId },
}

impl fmt::Display for TimingGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNetworkPort => {
                write!(
                    f,
                    "SIS network, latch, graph, and delay APIs are not ported to Rust yet"
                )
            }
            Self::MissingNetworkTraversalPort => {
                write!(
                    f,
                    "SIS transitive fanout traversal is not ported to Rust yet"
                )
            }
            Self::MissingDelayPort => write!(f, "SIS delay APIs are not ported to Rust yet"),
            Self::MissingClock(vertex) => write!(f, "vertex {:?} has no latch clock", vertex),
            Self::MissingLatchType(vertex) => write!(f, "vertex {:?} has no latch type", vertex),
            Self::UnknownVertex(vertex) => write!(f, "unknown timing graph vertex {:?}", vertex),
            Self::ClockNotInOrder { from, to } => {
                write!(f, "clock order does not contain {:?} before {:?}", from, to)
            }
        }
    }
}

impl Error for TimingGraphError {}

pub fn network_to_graph(
    _algorithm: AlgorithmType,
    _model: DelayModel,
) -> Result<TimingGraph, TimingGraphError> {
    Err(TimingGraphError::MissingNetworkPort)
}

pub fn build_graph() -> Result<(), TimingGraphError> {
    Err(TimingGraphError::MissingNetworkTraversalPort)
}

pub fn primary_input_arrival_time(
    pin_delay: DelayPin,
    delay_load: f64,
    arrival: Option<DelayTime>,
) -> DelayTime {
    let arrival = arrival.unwrap_or_default();
    DelayTime {
        rise: arrival.rise + pin_delay.drive.rise * delay_load,
        fall: arrival.fall + pin_delay.drive.fall * delay_load,
    }
}

pub fn latch_input_arrival_time(
    model: DelayModel,
    pin_delay: DelayPin,
    delay_load: f64,
    clock_delay: Option<DelayPin>,
) -> Result<DelayTime, TimingGraphError> {
    if model == DelayModel::Library {
        let clock_delay = clock_delay.ok_or(TimingGraphError::MissingDelayPort)?;
        Ok(DelayTime {
            rise: clock_delay.block.rise + clock_delay.drive.rise * delay_load,
            fall: clock_delay.block.fall + clock_delay.drive.fall * delay_load,
        })
    } else {
        Ok(DelayTime {
            rise: pin_delay.drive.rise * delay_load,
            fall: pin_delay.drive.fall * delay_load,
        })
    }
}

pub fn path_delay_from_fanin(
    fanin: DelayRange,
    pin_delay: DelayPin,
    delay_load: f64,
    node_is_primary_input: bool,
) -> DelayRange {
    let mut delay = DelayTime {
        rise: pin_delay.drive.rise * delay_load,
        fall: pin_delay.drive.fall * delay_load,
    };

    if !node_is_primary_input {
        delay.rise += pin_delay.block.rise;
        delay.fall += pin_delay.block.fall;
    }

    let mut result = DelayRange::empty();

    if matches!(pin_delay.phase, PinPhase::Inverting | PinPhase::Neither) {
        result.max.rise = result.max.rise.max(fanin.max.fall + delay.rise);
        result.max.fall = result.max.fall.max(fanin.max.rise + delay.fall);
        result.min.rise = result.min.rise.min(fanin.min.fall + delay.rise);
        result.min.fall = result.min.fall.min(fanin.min.rise + delay.fall);
    }

    if matches!(pin_delay.phase, PinPhase::NonInverting | PinPhase::Neither) {
        result.max.rise = result.max.rise.max(fanin.max.rise + delay.rise);
        result.max.fall = result.max.fall.max(fanin.max.fall + delay.fall);
        result.min.rise = result.min.rise.min(fanin.min.rise + delay.rise);
        result.min.fall = result.min.fall.min(fanin.min.fall + delay.fall);
    }

    result
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayRange {
    pub max: DelayTime,
    pub min: DelayTime,
}

impl DelayRange {
    pub fn empty() -> Self {
        Self {
            max: DelayTime::new(-INFTY, -INFTY),
            min: DelayTime::new(INFTY, INFTY),
        }
    }

    pub fn exact(rise: f64, fall: f64) -> Self {
        let delay = DelayTime::new(rise, fall);
        Self {
            max: delay,
            min: delay,
        }
    }
}

pub fn report_same_phase_latch_path(source: &TimingVertex, sink: &TimingVertex) -> String {
    format!(
        "Error, path between latches on same phase\n\t \t input \t output\nsource \t {} \t {} \nsink \t {} \t {} \n",
        source.latch_input_name.as_deref().unwrap_or("<unknown>"),
        source.latch_output_name.as_deref().unwrap_or("<unknown>"),
        sink.latch_input_name.as_deref().unwrap_or("<unknown>"),
        sink.latch_output_name.as_deref().unwrap_or("<unknown>")
    )
}

fn compute_edge_k(
    from: &TimingVertex,
    to: &TimingVertex,
    clock_order: &[ClockId],
) -> Result<i32, TimingGraphError> {
    if from.kind == VertexKind::NetworkIo || to.kind == VertexKind::NetworkIo {
        return Ok(0);
    }

    let clock1 = from
        .clock
        .ok_or(TimingGraphError::MissingClock(VertexId(from.id)))?;
    let clock2 = to
        .clock
        .ok_or(TimingGraphError::MissingClock(VertexId(to.id)))?;
    let from_latch_type = from
        .latch_type
        .ok_or(TimingGraphError::MissingLatchType(VertexId(from.id)))?;
    let to_latch_type = to
        .latch_type
        .ok_or(TimingGraphError::MissingLatchType(VertexId(to.id)))?;

    if clock1 == clock2 {
        if from_latch_type == to_latch_type
            || (from_latch_type == LatchType::Lsh && to_latch_type == LatchType::Lsl)
        {
            Ok(1)
        } else {
            Ok(0)
        }
    } else {
        let clock1_index = clock_order.iter().position(|clock| *clock == clock1);
        let clock2_index = clock_order.iter().position(|clock| *clock == clock2);

        match (clock1_index, clock2_index) {
            (Some(left), Some(right)) if left < right => Ok(0),
            (Some(left), Some(right)) if right < left => Ok(1),
            _ => Err(TimingGraphError::ClockNotInOrder {
                from: clock1,
                to: clock2,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_reports_unported_network_dependency() {
        assert_eq!(
            network_to_graph(AlgorithmType::OptimalClock, DelayModel::Library),
            Err(TimingGraphError::MissingNetworkPort)
        );
        assert_eq!(
            build_graph(),
            Err(TimingGraphError::MissingNetworkTraversalPort)
        );
    }

    #[test]
    fn add_or_merge_edge_keeps_extreme_path_delays() {
        let mut graph = TimingGraph::new(AlgorithmType::OptimalClock, vec![ClockId(1)]);
        let host = graph.add_host();
        let latch = graph.add_latch(LatchType::Fff, 1, ClockId(1), "i", "o");

        graph
            .add_or_merge_edge(host, latch, 3.0, 4.0, 1.5, 2.0)
            .unwrap();
        graph
            .add_or_merge_edge(host, latch, 5.0, 2.0, 2.5, 1.0)
            .unwrap();

        assert_eq!(graph.edges().len(), 1);
        assert_eq!(graph.edges()[0].max_delay, 5.0);
        assert_eq!(graph.edges()[0].min_delay, 1.0);
    }

    #[test]
    fn update_k_edges_matches_same_clock_latch_rules() {
        let mut graph = TimingGraph::new(AlgorithmType::OptimalClock, vec![ClockId(1)]);
        let fff = graph.add_latch(LatchType::Fff, 1, ClockId(1), "a", "b");
        let ffr = graph.add_latch(LatchType::Ffr, 1, ClockId(1), "c", "d");
        let lsh = graph.add_latch(LatchType::Lsh, 1, ClockId(1), "e", "f");
        let lsl = graph.add_latch(LatchType::Lsl, 1, ClockId(1), "g", "h");

        graph
            .add_or_merge_edge(fff, ffr, 1.0, 1.0, 1.0, 1.0)
            .unwrap();
        graph
            .add_or_merge_edge(lsh, lsl, 1.0, 1.0, 1.0, 1.0)
            .unwrap();
        graph
            .add_or_merge_edge(fff, fff, 1.0, 1.0, 1.0, 1.0)
            .unwrap();

        graph.update_k_edges().unwrap();

        assert_eq!(graph.edges()[0].k, 0);
        assert_eq!(graph.edges()[1].k, 1);
        assert_eq!(graph.edges()[2].k, 1);
    }

    #[test]
    fn update_k_edges_matches_clock_order_rules() {
        let mut graph =
            TimingGraph::new(AlgorithmType::OptimalClock, vec![ClockId(10), ClockId(20)]);
        let early = graph.add_latch(LatchType::Fff, 1, ClockId(10), "a", "b");
        let late = graph.add_latch(LatchType::Fff, 2, ClockId(20), "c", "d");

        graph
            .add_or_merge_edge(early, late, 1.0, 1.0, 1.0, 1.0)
            .unwrap();
        graph
            .add_or_merge_edge(late, early, 1.0, 1.0, 1.0, 1.0)
            .unwrap();

        graph.update_k_edges().unwrap();

        assert_eq!(graph.edges()[0].k, 0);
        assert_eq!(graph.edges()[1].k, 1);
    }

    #[test]
    fn arrival_time_helpers_match_c_arithmetic() {
        let pin = DelayPin::new(
            DelayTime::new(2.0, 3.0),
            DelayTime::new(5.0, 7.0),
            PinPhase::NonInverting,
        );

        assert_eq!(
            primary_input_arrival_time(pin, 4.0, Some(DelayTime::new(1.0, 2.0))),
            DelayTime::new(9.0, 14.0)
        );
        assert_eq!(
            latch_input_arrival_time(DelayModel::Mapped, pin, 4.0, None).unwrap(),
            DelayTime::new(8.0, 12.0)
        );
        assert_eq!(
            latch_input_arrival_time(DelayModel::Library, pin, 4.0, Some(pin)).unwrap(),
            DelayTime::new(13.0, 19.0)
        );
    }

    #[test]
    fn path_delay_from_fanin_applies_phase_polarity() {
        let fanin = DelayRange {
            max: DelayTime::new(10.0, 20.0),
            min: DelayTime::new(1.0, 2.0),
        };
        let pin = DelayPin::new(
            DelayTime::new(3.0, 4.0),
            DelayTime::new(5.0, 6.0),
            PinPhase::Inverting,
        );

        let result = path_delay_from_fanin(fanin, pin, 2.0, false);

        assert_eq!(result.max, DelayTime::new(31.0, 24.0));
        assert_eq!(result.min, DelayTime::new(13.0, 15.0));
    }

    #[test]
    fn format_latch_graph_uses_c_debug_shape() {
        let mut graph = TimingGraph::new(AlgorithmType::ClockVerify, vec![ClockId(1)]);
        let host = graph.add_host();
        let latch = graph.add_latch(LatchType::Fff, 1, ClockId(1), "in", "out");
        graph
            .add_or_merge_edge(host, latch, 4.0, 5.0, 1.0, 2.0)
            .unwrap();
        graph.update_k_edges().unwrap();

        let output = graph.format_latch_graph();

        assert!(output.contains("HOST ->"));
        assert!(output.contains("(1 - 5)->1 0"));
        assert!(output.contains("PI ->(1- 5) 0"));
        assert!(output.contains(" 1  (phase 1)"));
    }

    #[test]
    fn same_phase_latch_report_contains_latch_names() {
        let source = TimingVertex::latch(1, LatchType::Lsh, 1, ClockId(1), "si", "so");
        let sink = TimingVertex::latch(2, LatchType::Lsh, 1, ClockId(1), "ti", "to");

        let report = report_same_phase_latch_path(&source, &sink);

        assert!(report.contains("source \t si \t so"));
        assert!(report.contains("sink \t ti \t to"));
    }
}
