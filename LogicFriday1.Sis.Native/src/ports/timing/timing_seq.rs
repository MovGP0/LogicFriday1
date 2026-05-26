//! Native Rust port model for `LogicSynthesis/sis/timing/timing_seq.c`.
//!
//! The C file has two responsibilities: choose zero/unit/mapped delay for a
//! SIS `node_t`, and compute the max/min path delay through a mapped library
//! gate. The real SIS `node_t`, `lib_gate_t`, and shared `delay_map_simulate`
//! APIs are still represented by C-only ports, so this module exposes the
//! behavior as native Rust data structures and reports missing mapped-gate data
//! explicitly instead of adding legacy C ABI shims.

use std::error::Error;
use std::fmt;

use super::timing_graph::{DelayPin, DelayTime, PinPhase};

pub const INFINITY: f64 = 10_000.0;
pub const UNIT_FANOUT_SLOPE: f64 = 0.2;
pub const UNIT_BASE_DELAY: f64 = 1.0;
pub const MAPPED_GATE_LOAD: f64 = 0.0;

pub const REQUIRED_PORT_BEADS: &[&str] = &[
    "LogicFriday1-8j8.2.6.134", // delay/mapdelay.c: delay_map_simulate
    "LogicFriday1-8j8.2.6.257", // map/library.c: lib_gate_of, lib_gate_num_in
    "LogicFriday1-8j8.2.6.313", // node/fan.c: node_num_fanout
    "LogicFriday1-8j8.2.6.318", // node/node.c: node_function
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    Library,
    UnitFanout,
    Mapped,
    Unknown,
    Tdc,
}

impl DelayModel {
    pub fn from_c_name(name: &str) -> Option<Self> {
        match name {
            "DELAY_MODEL_UNIT" | "unit" => Some(Self::Unit),
            "DELAY_MODEL_LIBRARY" | "library" => Some(Self::Library),
            "DELAY_MODEL_UNIT_FANOUT" | "unit-fanout" => Some(Self::UnitFanout),
            "DELAY_MODEL_MAPPED" | "mapped" => Some(Self::Mapped),
            "DELAY_MODEL_UNKNOWN" | "unknown" => Some(Self::Unknown),
            "DELAY_MODEL_TDC" | "tdc" => Some(Self::Tdc),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimingNode {
    pub function: NodeFunction,
    pub fanout_count: usize,
    pub mapped_gate: Option<MappedGate>,
}

impl TimingNode {
    pub fn new(function: NodeFunction, fanout_count: usize) -> Self {
        Self {
            function,
            fanout_count,
            mapped_gate: None,
        }
    }

    pub fn with_mapped_gate(
        function: NodeFunction,
        fanout_count: usize,
        mapped_gate: MappedGate,
    ) -> Self {
        Self {
            function,
            fanout_count,
            mapped_gate: Some(mapped_gate),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappedGate {
    pub pin_delays: Vec<DelayPin>,
}

impl MappedGate {
    pub fn new(pin_delays: Vec<DelayPin>) -> Self {
        Self { pin_delays }
    }

    pub fn input_count(&self) -> usize {
        self.pin_delays.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimingSeqDependency {
    NodeFunction,
    NodeFanout,
    LibraryGate,
    DelayMapSimulation,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimingSeqError {
    MissingMappedGate,
    PinArrivalCountMismatch { arrivals: usize, pins: usize },
    MissingDependency(TimingSeqDependency),
}

impl fmt::Display for TimingSeqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMappedGate => write!(f, "SIS mapped gate data is not available"),
            Self::PinArrivalCountMismatch { arrivals, pins } => write!(
                f,
                "pin arrival count ({arrivals}) does not match pin delay count ({pins})"
            ),
            Self::MissingDependency(dependency) => match dependency {
                TimingSeqDependency::NodeFunction => {
                    write!(f, "SIS node_function is not ported to Rust yet")
                }
                TimingSeqDependency::NodeFanout => {
                    write!(f, "SIS node fanout APIs are not ported to Rust yet")
                }
                TimingSeqDependency::LibraryGate => {
                    write!(f, "SIS mapped library gate APIs are not ported to Rust yet")
                }
                TimingSeqDependency::DelayMapSimulation => {
                    write!(f, "shared SIS delay_map_simulate is not ported to Rust yet")
                }
            },
        }
    }
}

impl Error for TimingSeqError {}

pub fn required_port_beads() -> &'static [&'static str] {
    REQUIRED_PORT_BEADS
}

pub fn node_get_delay(node: &TimingNode, model: DelayModel) -> Result<DelayTime, TimingSeqError> {
    if matches!(
        node.function,
        NodeFunction::PrimaryInput | NodeFunction::PrimaryOutput
    ) {
        return Ok(DelayTime::new(0.0, 0.0));
    }

    if model == DelayModel::Unit {
        return Ok(unit_fanout_delay(node.fanout_count));
    }

    let gate = node
        .mapped_gate
        .as_ref()
        .ok_or(TimingSeqError::MissingMappedGate)?;
    map_get_delay(gate)
}

pub fn unit_fanout_delay(fanout_count: usize) -> DelayTime {
    let delay = UNIT_BASE_DELAY + UNIT_FANOUT_SLOPE * fanout_count as f64;
    DelayTime::new(delay, delay)
}

pub fn map_get_delay(gate: &MappedGate) -> Result<DelayTime, TimingSeqError> {
    let input_count = gate.input_count();
    let mut delay = DelayTime::new(-INFINITY, INFINITY);

    for input_index in 0..input_count {
        let mut arrivals = vec![DelayTime::new(-INFINITY, -INFINITY); input_count];
        arrivals[input_index] = DelayTime::new(0.0, 0.0);

        let time = delay_map_simulate(&arrivals, &gate.pin_delays, MAPPED_GATE_LOAD)?;
        let max_time = time.rise.max(time.fall);
        let min_time = time.rise.min(time.fall);

        delay.rise = delay.rise.max(max_time);
        delay.fall = delay.fall.min(min_time);
    }

    Ok(delay)
}

pub fn delay_map_simulate(
    pin_arrivals: &[DelayTime],
    pin_delays: &[DelayPin],
    load: f64,
) -> Result<DelayTime, TimingSeqError> {
    if pin_arrivals.len() != pin_delays.len() {
        return Err(TimingSeqError::PinArrivalCountMismatch {
            arrivals: pin_arrivals.len(),
            pins: pin_delays.len(),
        });
    }

    let mut arrival = DelayTime::new(-INFINITY, -INFINITY);

    for (pin_arrival, pin_delay) in pin_arrivals.iter().zip(pin_delays).rev() {
        let delay = DelayTime::new(
            pin_delay.block.rise + pin_delay.drive.rise * load,
            pin_delay.block.fall + pin_delay.drive.fall * load,
        );

        match pin_delay.phase {
            PinPhase::Inverting => {
                arrival.rise = arrival.rise.max(pin_arrival.fall + delay.rise);
                arrival.fall = arrival.fall.max(pin_arrival.rise + delay.fall);
            }
            PinPhase::NonInverting => {
                arrival.rise = arrival.rise.max(pin_arrival.rise + delay.rise);
                arrival.fall = arrival.fall.max(pin_arrival.fall + delay.fall);
            }
            PinPhase::Neither => {
                arrival.rise = arrival.rise.max(pin_arrival.fall + delay.rise);
                arrival.fall = arrival.fall.max(pin_arrival.rise + delay.fall);
                arrival.rise = arrival.rise.max(pin_arrival.rise + delay.rise);
                arrival.fall = arrival.fall.max(pin_arrival.fall + delay.fall);
            }
            PinPhase::NotGiven => {}
        }
    }

    Ok(arrival)
}

pub fn node_delay_from_unported_sis_node() -> Result<DelayTime, TimingSeqError> {
    Err(TimingSeqError::MissingDependency(
        TimingSeqDependency::NodeFunction,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(block: DelayTime, drive: DelayTime, phase: PinPhase) -> DelayPin {
        DelayPin::new(drive, block, phase)
    }

    #[test]
    fn constants_and_delay_models_match_c_headers() {
        assert_eq!(INFINITY, 10_000.0);
        assert_eq!(UNIT_BASE_DELAY, 1.0);
        assert_eq!(UNIT_FANOUT_SLOPE, 0.2);
        assert_eq!(MAPPED_GATE_LOAD, 0.0);

        assert_eq!(
            DelayModel::from_c_name("DELAY_MODEL_UNIT"),
            Some(DelayModel::Unit)
        );
        assert_eq!(
            DelayModel::from_c_name("DELAY_MODEL_MAPPED"),
            Some(DelayModel::Mapped)
        );
        assert_eq!(DelayModel::from_c_name("other"), None);
    }

    #[test]
    fn primary_io_nodes_have_zero_delay_for_all_models() {
        let pi = TimingNode::new(NodeFunction::PrimaryInput, 8);
        let po = TimingNode::new(NodeFunction::PrimaryOutput, 4);

        assert_eq!(
            node_get_delay(&pi, DelayModel::Unit).unwrap(),
            DelayTime::new(0.0, 0.0)
        );
        assert_eq!(
            node_get_delay(&po, DelayModel::Mapped).unwrap(),
            DelayTime::new(0.0, 0.0)
        );
    }

    #[test]
    fn unit_delay_uses_c_fanout_formula_for_internal_nodes() {
        let node = TimingNode::new(NodeFunction::Internal, 3);

        assert_eq!(
            node_get_delay(&node, DelayModel::Unit).unwrap(),
            DelayTime::new(1.6, 1.6)
        );
        assert_eq!(unit_fanout_delay(0), DelayTime::new(1.0, 1.0));
    }

    #[test]
    fn mapped_gate_delay_matches_timing_seq_extreme_scan() {
        let gate = MappedGate::new(vec![
            pin(
                DelayTime::new(2.0, 5.0),
                DelayTime::new(100.0, 100.0),
                PinPhase::NonInverting,
            ),
            pin(
                DelayTime::new(7.0, 3.0),
                DelayTime::new(100.0, 100.0),
                PinPhase::Inverting,
            ),
        ]);

        let node = TimingNode::with_mapped_gate(NodeFunction::Internal, 0, gate);

        assert_eq!(
            node_get_delay(&node, DelayModel::Mapped).unwrap(),
            DelayTime::new(7.0, 2.0)
        );
    }

    #[test]
    fn delay_map_simulate_applies_pin_phase_and_load() {
        let arrivals = [
            DelayTime::new(10.0, 20.0),
            DelayTime::new(1.0, 2.0),
            DelayTime::new(30.0, 40.0),
        ];
        let pins = [
            pin(
                DelayTime::new(1.0, 2.0),
                DelayTime::new(0.5, 0.25),
                PinPhase::Inverting,
            ),
            pin(
                DelayTime::new(3.0, 4.0),
                DelayTime::new(0.0, 1.0),
                PinPhase::NonInverting,
            ),
            pin(
                DelayTime::new(5.0, 6.0),
                DelayTime::new(1.0, 0.0),
                PinPhase::Neither,
            ),
        ];

        assert_eq!(
            delay_map_simulate(&arrivals, &pins, 2.0).unwrap(),
            DelayTime::new(47.0, 46.0)
        );
    }

    #[test]
    fn missing_dependencies_are_explicit() {
        let node = TimingNode::new(NodeFunction::Internal, 1);
        assert_eq!(
            node_get_delay(&node, DelayModel::Mapped),
            Err(TimingSeqError::MissingMappedGate)
        );
        assert_eq!(
            delay_map_simulate(
                &[],
                &[pin(
                    DelayTime::new(1.0, 1.0),
                    DelayTime::new(0.0, 0.0),
                    PinPhase::NotGiven,
                )],
                0.0
            ),
            Err(TimingSeqError::PinArrivalCountMismatch {
                arrivals: 0,
                pins: 1
            })
        );
        assert!(required_port_beads().contains(&"LogicFriday1-8j8.2.6.134"));
        assert_eq!(
            node_delay_from_unported_sis_node(),
            Err(TimingSeqError::MissingDependency(
                TimingSeqDependency::NodeFunction
            ))
        );
    }
}
