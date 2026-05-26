//! Native Rust mapped-delay helpers for `sis/map/map_delay.c`.
//!
//! The original file combines mapped-gate timing math with legacy global SIS
//! PI/PO delay state and fanout-delay setup. This port keeps the useful
//! behavior as owned-data helpers over `GenlibLibrary` and
//! `VirtualMappedNetwork`: mapped gate arrival calculation, mapped gate
//! required-time back propagation, primary input/output default extraction,
//! and simple fanout load estimates. Operations that still require the legacy
//! SIS network or fanout-delay optimizer return typed dependency errors.

use std::error::Error;
use std::fmt;

use super::library::{GenlibGate, GenlibLibrary, PinPhase};
use super::libutil::{self, LibUtilError, PinDelay};
use super::virtual_del::{
    self, PrimaryInputTiming, PrimaryOutputTiming, TimingPhase, VirtualDelayConstraints,
    VirtualDelayGateTiming, VirtualDelayInput, VirtualDelayLibrary, VirtualDelayOptions,
    VirtualDelayOutput, VirtualDelayPinTiming, VirtualDelayState, ZERO_DELAY,
};
use super::virtual_net::{
    DelayTime, GateKind, MINUS_INFINITY, NodeId, NodeKind, VirtualMappedNetwork,
    VirtualNetworkError,
};

pub const MAX_PRECOMPUTED_FANOUT_LOADS: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimaryIoDefaults {
    pub arrival: DelayTime,
    pub drive: DelayTime,
    pub output_load: f64,
    pub input_load_limit: f64,
    pub required: DelayTime,
}

impl Default for PrimaryIoDefaults {
    fn default() -> Self {
        Self {
            arrival: ZERO_DELAY,
            drive: ZERO_DELAY,
            output_load: 0.0,
            input_load_limit: f64::INFINITY,
            required: MINUS_INFINITY,
        }
    }
}

impl PrimaryIoDefaults {
    pub fn from_default_inverter(default_inverter: &GenlibGate) -> Result<Self, MapDelayError> {
        let pin = libutil::pin_delay(default_inverter, 0)?;

        Ok(Self {
            arrival: ZERO_DELAY,
            drive: DelayTime::new(pin.rise_fanout_delay, pin.fall_fanout_delay),
            output_load: pin.input_load,
            input_load_limit: pin.max_load,
            required: MINUS_INFINITY,
        })
    }

    pub fn constraints_for_network(
        &self,
        network: &VirtualMappedNetwork,
    ) -> VirtualDelayConstraints {
        VirtualDelayConstraints {
            inputs: network
                .inputs()
                .iter()
                .copied()
                .map(|node| VirtualDelayInput {
                    node,
                    timing: PrimaryInputTiming {
                        arrival: self.arrival,
                        drive: self.drive,
                    },
                })
                .collect(),
            outputs: network
                .outputs()
                .iter()
                .copied()
                .map(|node| VirtualDelayOutput {
                    node,
                    timing: PrimaryOutputTiming {
                        load: self.output_load,
                        required: self.required,
                    },
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutLoadEstimation {
    None,
    DirectBufferLoad,
    BufferedTree,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutLoadModel {
    pub estimation: FanoutLoadEstimation,
    pub buffer_load: f64,
}

impl Default for FanoutLoadModel {
    fn default() -> Self {
        Self {
            estimation: FanoutLoadEstimation::None,
            buffer_load: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrecomputedFanoutLoads {
    loads: [f64; MAX_PRECOMPUTED_FANOUT_LOADS],
}

impl PrecomputedFanoutLoads {
    pub fn new(model: FanoutLoadModel) -> Result<Self, MapDelayError> {
        let mut loads = [0.0; MAX_PRECOMPUTED_FANOUT_LOADS];
        for (fanout_count, load) in loads.iter_mut().enumerate().skip(1) {
            *load = compute_fanout_load_correction(fanout_count, model)?;
        }

        Ok(Self { loads })
    }

    pub fn load_for_fanout_count(
        &self,
        fanout_count: usize,
        model: FanoutLoadModel,
    ) -> Result<f64, MapDelayError> {
        if fanout_count == 0 {
            return Err(MapDelayError::InvalidFanoutCount { fanout_count });
        }
        if fanout_count < MAX_PRECOMPUTED_FANOUT_LOADS {
            return Ok(self.loads[fanout_count]);
        }

        compute_fanout_load_correction(fanout_count, model)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapDelayState {
    pub timing: VirtualDelayState,
    pub max_output_arrival: DelayTime,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MapDelayError {
    InvalidLoad {
        load: f64,
    },
    InvalidFanoutCount {
        fanout_count: usize,
    },
    MissingGate {
        node: NodeId,
    },
    MissingLibraryGate {
        name: String,
    },
    PinCountMismatch {
        gate: String,
        expected: usize,
        actual: usize,
    },
    LoadExceedsPinLimit {
        gate: String,
        pin: usize,
        load: f64,
        max_load: f64,
    },
    InvalidPrimaryInput {
        node: NodeId,
    },
    InvalidPrimaryOutput {
        node: NodeId,
    },
    VirtualNetwork(VirtualNetworkError),
    VirtualDelay(virtual_del::VirtualDelayError),
    LibUtil(LibUtilError),
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for MapDelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLoad { load } => write!(f, "invalid mapped delay load {load}"),
            Self::InvalidFanoutCount { fanout_count } => {
                write!(
                    f,
                    "fanout load correction requires a positive fanout count, got {fanout_count}"
                )
            }
            Self::MissingGate { node } => {
                write!(
                    f,
                    "virtual network node {} has no mapped gate",
                    node.index()
                )
            }
            Self::MissingLibraryGate { name } => {
                write!(
                    f,
                    "library gate '{name}' was not found for mapped-delay calculation"
                )
            }
            Self::PinCountMismatch {
                gate,
                expected,
                actual,
            } => write!(f, "gate '{gate}' expected {expected} pins but got {actual}"),
            Self::LoadExceedsPinLimit {
                gate,
                pin,
                load,
                max_load,
            } => write!(
                f,
                "gate '{gate}' pin {pin} load {load} exceeds max load {max_load}"
            ),
            Self::InvalidPrimaryInput { node } => {
                write!(f, "node {} is not a primary input", node.index())
            }
            Self::InvalidPrimaryOutput { node } => {
                write!(f, "node {} is not a primary output", node.index())
            }
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::VirtualDelay(error) => write!(f, "{error}"),
            Self::LibUtil(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for MapDelayError {}

impl From<VirtualNetworkError> for MapDelayError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

impl From<virtual_del::VirtualDelayError> for MapDelayError {
    fn from(value: virtual_del::VirtualDelayError) -> Self {
        Self::VirtualDelay(value)
    }
}

impl From<LibUtilError> for MapDelayError {
    fn from(value: LibUtilError) -> Self {
        Self::LibUtil(value)
    }
}

pub fn map_alloc_delay_info_unavailable() -> Result<(), MapDelayError> {
    Err(MapDelayError::MissingSisPorts {
        operation: "map_alloc_delay_info legacy SIS network delay metadata mutation",
    })
}

pub fn map_free_delay_info_unavailable() -> Result<(), MapDelayError> {
    Err(MapDelayError::MissingSisPorts {
        operation: "map_free_delay_info legacy fanout-delay global cleanup",
    })
}

pub fn compute_fanout_load_correction(
    fanout_count: usize,
    model: FanoutLoadModel,
) -> Result<f64, MapDelayError> {
    if fanout_count == 0 {
        return Err(MapDelayError::InvalidFanoutCount { fanout_count });
    }
    if !model.buffer_load.is_finite() || model.buffer_load < 0.0 {
        return Err(MapDelayError::InvalidLoad {
            load: model.buffer_load,
        });
    }

    match model.estimation {
        FanoutLoadEstimation::None => Ok(0.0),
        FanoutLoadEstimation::DirectBufferLoad => Ok(fanout_count as f64 * model.buffer_load),
        FanoutLoadEstimation::BufferedTree => Err(MapDelayError::MissingSisPorts {
            operation: "map_compute_fanout_load_correction buffered-tree estimate",
        }),
    }
}

pub fn compute_wire_load(
    wire_load_per_fanout: f64,
    fanout_count: usize,
) -> Result<f64, MapDelayError> {
    if !wire_load_per_fanout.is_finite() || wire_load_per_fanout < 0.0 {
        return Err(MapDelayError::InvalidLoad {
            load: wire_load_per_fanout,
        });
    }

    Ok(wire_load_per_fanout * fanout_count as f64)
}

pub fn pin_polarity(pin_delay: PinDelay) -> PinPhase {
    pin_delay.phase
}

pub fn compute_gate_arrival_time(
    gate: &GenlibGate,
    input_arrivals: &[DelayTime],
    load: f64,
) -> Result<DelayTime, MapDelayError> {
    validate_load(load)?;
    if gate.pins.len() != input_arrivals.len() {
        return Err(MapDelayError::PinCountMismatch {
            gate: gate.name.clone(),
            expected: gate.pins.len(),
            actual: input_arrivals.len(),
        });
    }

    let mut arrival = MINUS_INFINITY;
    for (pin, (pin_data, input)) in gate.pins.iter().zip(input_arrivals.iter()).enumerate() {
        let pin_delay = PinDelay::from(pin_data);
        validate_pin_load(&gate.name, pin, load, pin_delay.max_load)?;
        let rise_delay = pin_delay.rise_block_delay + pin_delay.rise_fanout_delay * load;
        let fall_delay = pin_delay.fall_block_delay + pin_delay.fall_fanout_delay * load;
        let pin_arrival = match pin_delay.phase {
            PinPhase::NonInv => DelayTime::new(input.rise + rise_delay, input.fall + fall_delay),
            PinPhase::Inv => DelayTime::new(input.fall + rise_delay, input.rise + fall_delay),
            PinPhase::Unknown => {
                let input_arrival = input.rise.max(input.fall);
                DelayTime::new(input_arrival + rise_delay, input_arrival + fall_delay)
            }
        };
        arrival = max_delay_time(arrival, pin_arrival);
    }

    Ok(arrival)
}

pub fn compute_gate_input_required_times(
    gate: &GenlibGate,
    node_required: DelayTime,
    load: f64,
) -> Result<Vec<DelayTime>, MapDelayError> {
    validate_load(load)?;
    gate.pins
        .iter()
        .enumerate()
        .map(|(pin, pin_data)| {
            let pin_delay = PinDelay::from(pin_data);
            validate_pin_load(&gate.name, pin, load, pin_delay.max_load)?;
            Ok(compute_pin_required_time(pin_delay, node_required, load))
        })
        .collect()
}

pub fn compute_mapped_gate_arrival_time(
    library: &GenlibLibrary,
    network: &VirtualMappedNetwork,
    node: NodeId,
    input_arrivals: &[DelayTime],
) -> Result<DelayTime, MapDelayError> {
    let item = network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?;
    let Some(gate_kind) = &item.gate else {
        return Err(MapDelayError::MissingGate { node });
    };

    match gate_kind {
        GateKind::Wire => input_arrivals
            .first()
            .copied()
            .ok_or(MapDelayError::PinCountMismatch {
                gate: gate_kind.mnemonic().to_string(),
                expected: 1,
                actual: 0,
            }),
        GateKind::One | GateKind::Zero => Ok(ZERO_DELAY),
        _ => {
            let gate = library_gate_for_kind(library, gate_kind)?;
            compute_gate_arrival_time(gate, input_arrivals, item.load)
        }
    }
}

pub fn compute_mapped_gate_required_times(
    library: &GenlibLibrary,
    network: &VirtualMappedNetwork,
    node: NodeId,
    node_required: DelayTime,
) -> Result<Vec<DelayTime>, MapDelayError> {
    let item = network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?;
    let Some(gate_kind) = &item.gate else {
        return Err(MapDelayError::MissingGate { node });
    };

    match gate_kind {
        GateKind::Wire => Ok(vec![node_required]),
        GateKind::One | GateKind::Zero => Ok(Vec::new()),
        _ => {
            let gate = library_gate_for_kind(library, gate_kind)?;
            compute_gate_input_required_times(gate, node_required, item.load)
        }
    }
}

pub fn apply_mapped_gate_required_times(
    library: &GenlibLibrary,
    network: &mut VirtualMappedNetwork,
    node: NodeId,
    node_required: DelayTime,
) -> Result<Vec<DelayTime>, MapDelayError> {
    let required = compute_mapped_gate_required_times(library, network, node, node_required)?;
    network
        .node_mut(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?
        .required = node_required;
    network.update_link_required_times(node, &required)?;

    Ok(required)
}

pub fn compute_mapped_network_delay(
    library: &GenlibLibrary,
    network: &mut VirtualMappedNetwork,
    constraints: &VirtualDelayConstraints,
    options: VirtualDelayOptions,
) -> Result<MapDelayState, MapDelayError> {
    let virtual_library = virtual_delay_library_from_genlib(library)?;
    let timing =
        virtual_del::compute_arrival_times(network, constraints, &virtual_library, options)?;
    let max_output_arrival = virtual_del::set_po_required_times(network, constraints, &timing)?;

    for level in network.levels()?.into_iter().rev() {
        for node in level {
            if network
                .node(node)
                .ok_or(VirtualNetworkError::MissingNode(node))?
                .kind
                != NodeKind::PrimaryOutput
            {
                virtual_del::compute_node_required_time(network, node, &virtual_library, options)?;
            }
        }
    }
    for input in network.inputs().to_vec() {
        virtual_del::compute_node_required_time(network, input, &virtual_library, options)?;
    }

    Ok(MapDelayState {
        timing,
        max_output_arrival,
    })
}

fn virtual_delay_library_from_genlib(
    library: &GenlibLibrary,
) -> Result<VirtualDelayLibrary, MapDelayError> {
    let default_pin_timing =
        VirtualDelayPinTiming::new(TimingPhase::Unknown, 1.0, f64::MAX, 1.0, 0.0, 1.0, 0.0);
    let gate_timings = library
        .gates
        .iter()
        .map(VirtualDelayGateTiming::from_genlib_gate)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(VirtualDelayLibrary::new(default_pin_timing, gate_timings)?)
}

pub fn primary_input_arrival(
    constraints: &VirtualDelayConstraints,
    node: NodeId,
) -> PrimaryInputTiming {
    constraints.input_timing(node)
}

pub fn primary_output_required(
    constraints: &VirtualDelayConstraints,
    node: NodeId,
) -> PrimaryOutputTiming {
    constraints.output_timing(node)
}

pub fn set_default_po_required(
    defaults: &mut PrimaryIoDefaults,
    default_value: DelayTime,
    required_was_not_set: bool,
) -> bool {
    if required_was_not_set {
        defaults.required = default_value;
        return true;
    }

    false
}

fn library_gate_for_kind<'a>(
    library: &'a GenlibLibrary,
    gate_kind: &GateKind,
) -> Result<&'a GenlibGate, MapDelayError> {
    let name = match gate_kind {
        GateKind::Library(name) => name.as_str(),
        _ => gate_kind.mnemonic(),
    };

    library
        .gate(name)
        .ok_or_else(|| MapDelayError::MissingLibraryGate {
            name: name.to_string(),
        })
}

fn compute_pin_required_time(
    pin_delay: PinDelay,
    node_required: DelayTime,
    load: f64,
) -> DelayTime {
    let rise_delay = pin_delay.rise_block_delay + pin_delay.rise_fanout_delay * load;
    let fall_delay = pin_delay.fall_block_delay + pin_delay.fall_fanout_delay * load;
    let rise_limit = node_required.rise - rise_delay;
    let fall_limit = node_required.fall - fall_delay;

    match pin_delay.phase {
        PinPhase::Inv => DelayTime::new(fall_limit, rise_limit),
        PinPhase::NonInv => DelayTime::new(rise_limit, fall_limit),
        PinPhase::Unknown => {
            let limit = rise_limit.min(fall_limit);
            DelayTime::new(limit, limit)
        }
    }
}

fn validate_load(load: f64) -> Result<(), MapDelayError> {
    if !load.is_finite() || load < 0.0 {
        return Err(MapDelayError::InvalidLoad { load });
    }

    Ok(())
}

fn validate_pin_load(
    gate: &str,
    pin: usize,
    load: f64,
    max_load: f64,
) -> Result<(), MapDelayError> {
    if max_load.is_finite() && load > max_load {
        return Err(MapDelayError::LoadExceedsPinLimit {
            gate: gate.to_string(),
            pin,
            load,
            max_load,
        });
    }

    Ok(())
}

fn max_delay_time(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise.max(right.rise), left.fall.max(right.fall))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::library::parse_genlib;
    use crate::ports::map::virtual_net::{SourceRef, VirtualMappedNetwork};

    fn sample_library() -> GenlibLibrary {
        parse_genlib(concat!(
            "GATE and2 2 O=a*b;\n",
            "PIN a NONINV 1 20 1 .5 2 .25\n",
            "PIN b NONINV 1 20 3 .25 4 .5\n",
            "GATE inv 1 O=!a;\n",
            "PIN a INV 2 10 5 .5 7 .25\n",
            "GATE unk 1 O=a;\n",
            "PIN a UNKNOWN 1 20 2 1 3 2\n",
        ))
        .unwrap()
    }

    #[test]
    fn computes_required_times_for_pin_phases() {
        let library = sample_library();
        let and_gate = library.gate("and2").unwrap();
        let inv_gate = library.gate("inv").unwrap();
        let unk_gate = library.gate("unk").unwrap();

        assert_eq!(
            compute_gate_input_required_times(and_gate, DelayTime::new(20.0, 30.0), 4.0).unwrap(),
            vec![DelayTime::new(17.0, 27.0), DelayTime::new(16.0, 24.0)]
        );
        assert_eq!(
            compute_gate_input_required_times(inv_gate, DelayTime::new(20.0, 30.0), 4.0).unwrap(),
            vec![DelayTime::new(22.0, 13.0)]
        );
        assert_eq!(
            compute_gate_input_required_times(unk_gate, DelayTime::new(20.0, 30.0), 4.0).unwrap(),
            vec![DelayTime::new(14.0, 14.0)]
        );
    }

    #[test]
    fn computes_gate_arrival_time_from_genlib_delay() {
        let library = sample_library();
        let gate = library.gate("and2").unwrap();

        assert_eq!(
            compute_gate_arrival_time(
                gate,
                &[DelayTime::new(1.0, 2.0), DelayTime::new(4.0, 5.0)],
                4.0
            )
            .unwrap(),
            DelayTime::new(8.0, 11.0)
        );
    }

    #[test]
    fn applies_mapped_required_times_to_virtual_gate_links() {
        let library = sample_library();
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let gate = network.add_gate(
            "n1",
            GateKind::Library("and2".to_string()),
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        network
            .add_primary_output("y", SourceRef::Node(gate))
            .unwrap();
        network.setup_gate_links().unwrap();
        network.node_mut(gate).unwrap().load = 4.0;

        let required = apply_mapped_gate_required_times(
            &library,
            &mut network,
            gate,
            DelayTime::new(20.0, 30.0),
        )
        .unwrap();

        assert_eq!(
            required,
            vec![DelayTime::new(17.0, 27.0), DelayTime::new(16.0, 24.0)]
        );
        assert_eq!(
            network.gate_link(a, gate, 0).unwrap().required,
            DelayTime::new(17.0, 27.0)
        );
        assert_eq!(
            network.gate_link(b, gate, 1).unwrap().required,
            DelayTime::new(16.0, 24.0)
        );
    }

    #[test]
    fn computes_owned_network_delay_and_required_times() {
        let library = sample_library();
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let gate = network.add_gate(
            "n1",
            GateKind::Library("and2".to_string()),
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        let y = network
            .add_primary_output("y", SourceRef::Node(gate))
            .unwrap();
        let constraints = VirtualDelayConstraints {
            inputs: vec![
                VirtualDelayInput {
                    node: a,
                    timing: PrimaryInputTiming {
                        arrival: DelayTime::new(1.0, 2.0),
                        drive: ZERO_DELAY,
                    },
                },
                VirtualDelayInput {
                    node: b,
                    timing: PrimaryInputTiming {
                        arrival: DelayTime::new(4.0, 5.0),
                        drive: ZERO_DELAY,
                    },
                },
            ],
            outputs: vec![VirtualDelayOutput {
                node: y,
                timing: PrimaryOutputTiming {
                    load: 4.0,
                    required: DelayTime::new(20.0, 30.0),
                },
            }],
        };

        let state = compute_mapped_network_delay(
            &library,
            &mut network,
            &constraints,
            VirtualDelayOptions::default(),
        )
        .unwrap();

        assert_eq!(
            state.timing.arrival(gate).unwrap(),
            DelayTime::new(8.0, 11.0)
        );
        assert_eq!(
            network.node(y).unwrap().required,
            DelayTime::new(20.0, 30.0)
        );
        assert_eq!(
            network.node(gate).unwrap().required,
            DelayTime::new(20.0, 30.0)
        );
        assert_eq!(
            network.node(a).unwrap().required,
            DelayTime::new(17.0, 27.0)
        );
        assert_eq!(
            network.node(b).unwrap().required,
            DelayTime::new(16.0, 24.0)
        );
    }

    #[test]
    fn computes_primary_io_defaults_and_fanout_loads() {
        let library = sample_library();
        let defaults =
            PrimaryIoDefaults::from_default_inverter(library.gate("inv").unwrap()).unwrap();

        assert_eq!(defaults.drive, DelayTime::new(0.5, 0.25));
        assert_eq!(defaults.output_load, 2.0);
        assert_eq!(defaults.input_load_limit, 10.0);

        let model = FanoutLoadModel {
            estimation: FanoutLoadEstimation::DirectBufferLoad,
            buffer_load: defaults.output_load,
        };
        let precomputed = PrecomputedFanoutLoads::new(model).unwrap();
        assert_eq!(precomputed.load_for_fanout_count(3, model).unwrap(), 6.0);
    }
}
