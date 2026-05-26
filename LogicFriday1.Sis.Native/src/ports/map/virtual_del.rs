//! Native Rust virtual-delay support for `sis/map/virtual_del.c`.
//!
//! The original SIS code updates delay fields stored in `MAP(node)` while
//! walking a virtual mapped network. This port keeps the same owned-data
//! behavior without adding legacy C ABI exports: link loads and required times
//! are written back to `VirtualMappedNetwork`, while arrival and per-input
//! arrival snapshots are returned in a separate timing state.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use super::library::{GenlibGate, PinPhase};
use super::virtual_net::{
    DelayTime, GateKind, GateLink, MINUS_INFINITY, NodeId, NodeKind, SourceRef,
    VirtualMappedNetwork, VirtualNetworkError,
};

pub const ZERO_DELAY: DelayTime = DelayTime {
    rise: 0.0,
    fall: 0.0,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimaryInputTiming {
    pub arrival: DelayTime,
    pub drive: DelayTime,
}

impl Default for PrimaryInputTiming {
    fn default() -> Self {
        Self {
            arrival: ZERO_DELAY,
            drive: ZERO_DELAY,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimaryOutputTiming {
    pub load: f64,
    pub required: DelayTime,
}

impl Default for PrimaryOutputTiming {
    fn default() -> Self {
        Self {
            load: 0.0,
            required: MINUS_INFINITY,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WireLoadModel {
    pub per_fanout: f64,
}

impl Default for WireLoadModel {
    fn default() -> Self {
        Self { per_fanout: 0.0 }
    }
}

impl WireLoadModel {
    pub fn load_for_fanout_count(self, fanout_count: usize) -> f64 {
        self.per_fanout * fanout_count as f64
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualDelayOptions {
    pub wire_load: WireLoadModel,
    pub warn_on_constants: bool,
}

impl Default for VirtualDelayOptions {
    fn default() -> Self {
        Self {
            wire_load: WireLoadModel::default(),
            warn_on_constants: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingPhase {
    Inverting,
    NonInverting,
    Unknown,
}

impl From<PinPhase> for TimingPhase {
    fn from(value: PinPhase) -> Self {
        match value {
            PinPhase::Inv => Self::Inverting,
            PinPhase::NonInv => Self::NonInverting,
            PinPhase::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualDelayPinTiming {
    pub phase: TimingPhase,
    pub input_load: f64,
    pub max_load: f64,
    pub rise_block_delay: f64,
    pub rise_fanout_delay: f64,
    pub fall_block_delay: f64,
    pub fall_fanout_delay: f64,
}

impl VirtualDelayPinTiming {
    pub fn new(
        phase: TimingPhase,
        input_load: f64,
        max_load: f64,
        rise_block_delay: f64,
        rise_fanout_delay: f64,
        fall_block_delay: f64,
        fall_fanout_delay: f64,
    ) -> Self {
        Self {
            phase,
            input_load,
            max_load,
            rise_block_delay,
            rise_fanout_delay,
            fall_block_delay,
            fall_fanout_delay,
        }
    }

    fn validate(self, gate: &str, pin: usize) -> Result<(), VirtualDelayError> {
        let values = [
            self.input_load,
            self.max_load,
            self.rise_block_delay,
            self.rise_fanout_delay,
            self.fall_block_delay,
            self.fall_fanout_delay,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(VirtualDelayError::InvalidTiming {
                gate: gate.to_string(),
                pin,
            });
        }

        Ok(())
    }

    fn rise_delay(self, load: f64) -> f64 {
        self.rise_block_delay + self.rise_fanout_delay * load
    }

    fn fall_delay(self, load: f64) -> f64 {
        self.fall_block_delay + self.fall_fanout_delay * load
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualDelayGateTiming {
    pub gate_name: String,
    pub pins: Vec<VirtualDelayPinTiming>,
    pub is_wire: bool,
}

impl VirtualDelayGateTiming {
    pub fn new(
        gate_name: impl Into<String>,
        pins: Vec<VirtualDelayPinTiming>,
    ) -> Result<Self, VirtualDelayError> {
        let timing = Self {
            gate_name: gate_name.into(),
            pins,
            is_wire: false,
        };
        timing.validate()?;
        Ok(timing)
    }

    pub fn wire() -> Self {
        Self {
            gate_name: "**wire**".to_string(),
            pins: vec![VirtualDelayPinTiming::new(
                TimingPhase::NonInverting,
                0.0,
                f64::INFINITY,
                0.0,
                0.0,
                0.0,
                0.0,
            )],
            is_wire: true,
        }
    }

    pub fn from_genlib_gate(gate: &GenlibGate) -> Result<Self, VirtualDelayError> {
        let pins = gate
            .pins
            .iter()
            .map(|pin| {
                VirtualDelayPinTiming::new(
                    pin.phase.into(),
                    pin.input_load,
                    pin.max_load,
                    pin.rise_block_delay,
                    pin.rise_fanout_delay,
                    pin.fall_block_delay,
                    pin.fall_fanout_delay,
                )
            })
            .collect::<Vec<_>>();

        Self::new(gate.name.clone(), pins)
    }

    fn validate(&self) -> Result<(), VirtualDelayError> {
        if self.gate_name.is_empty() {
            return Err(VirtualDelayError::EmptyGateName);
        }
        for (pin, timing) in self.pins.iter().copied().enumerate() {
            timing.validate(&self.gate_name, pin)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualDelayLibrary {
    default_pin_timing: VirtualDelayPinTiming,
    gate_timings: Vec<VirtualDelayGateTiming>,
}

impl VirtualDelayLibrary {
    pub fn new(
        default_pin_timing: VirtualDelayPinTiming,
        gate_timings: Vec<VirtualDelayGateTiming>,
    ) -> Result<Self, VirtualDelayError> {
        default_pin_timing.validate("<default>", 0)?;
        for timing in &gate_timings {
            timing.validate()?;
        }

        Ok(Self {
            default_pin_timing,
            gate_timings,
        })
    }

    pub fn unit_delay() -> Self {
        Self {
            default_pin_timing: VirtualDelayPinTiming::new(
                TimingPhase::Unknown,
                1.0,
                f64::INFINITY,
                1.0,
                0.0,
                1.0,
                0.0,
            ),
            gate_timings: Vec::new(),
        }
    }

    pub fn with_genlib_gates(gates: &[GenlibGate]) -> Result<Self, VirtualDelayError> {
        Self::new(
            Self::unit_delay().default_pin_timing,
            gates
                .iter()
                .map(VirtualDelayGateTiming::from_genlib_gate)
                .collect::<Result<Vec<_>, _>>()?,
        )
    }

    pub fn push_gate_timing(
        &mut self,
        timing: VirtualDelayGateTiming,
    ) -> Result<(), VirtualDelayError> {
        timing.validate()?;
        self.gate_timings.push(timing);
        Ok(())
    }

    pub fn gate_timing(
        &self,
        gate: &GateKind,
        fanin_count: usize,
    ) -> Result<VirtualDelayGateTiming, VirtualDelayError> {
        if gate.is_wire() {
            return Ok(VirtualDelayGateTiming::wire());
        }

        if let Some(timing) = self
            .gate_timings
            .iter()
            .find(|timing| timing.gate_name == gate.mnemonic())
        {
            return repeated_or_exact_timing(timing, fanin_count);
        }

        repeated_default_timing(gate.mnemonic(), self.default_pin_timing, fanin_count)
    }
}

impl Default for VirtualDelayLibrary {
    fn default() -> Self {
        Self::unit_delay()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualDelayInput {
    pub node: NodeId,
    pub timing: PrimaryInputTiming,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualDelayOutput {
    pub node: NodeId,
    pub timing: PrimaryOutputTiming,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualDelayConstraints {
    pub inputs: Vec<VirtualDelayInput>,
    pub outputs: Vec<VirtualDelayOutput>,
}

impl VirtualDelayConstraints {
    pub fn default_for_network(network: &VirtualMappedNetwork) -> Self {
        Self {
            inputs: network
                .inputs()
                .iter()
                .copied()
                .map(|node| VirtualDelayInput {
                    node,
                    timing: PrimaryInputTiming::default(),
                })
                .collect(),
            outputs: network
                .outputs()
                .iter()
                .copied()
                .map(|node| VirtualDelayOutput {
                    node,
                    timing: PrimaryOutputTiming::default(),
                })
                .collect(),
        }
    }

    pub fn input_timing(&self, node: NodeId) -> PrimaryInputTiming {
        self.inputs
            .iter()
            .find(|input| input.node == node)
            .map(|input| input.timing)
            .unwrap_or_default()
    }

    pub fn output_timing(&self, node: NodeId) -> PrimaryOutputTiming {
        self.outputs
            .iter()
            .find(|output| output.node == node)
            .map(|output| output.timing)
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualDelayState {
    pub node_arrivals: Vec<Option<DelayTime>>,
    pub input_arrivals: Vec<Vec<DelayTime>>,
    pub constant_warning_count: usize,
}

impl VirtualDelayState {
    pub fn arrival(&self, node: NodeId) -> Option<DelayTime> {
        self.node_arrivals.get(node.index()).copied().flatten()
    }

    pub fn arrival_inputs(&self, node: NodeId) -> Option<&[DelayTime]> {
        self.input_arrivals
            .get(node.index())
            .map(Vec::as_slice)
            .filter(|values| !values.is_empty())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VirtualDelayError {
    EmptyGateName,
    InvalidTiming {
        gate: String,
        pin: usize,
    },
    InvalidLoad {
        node: NodeId,
        load: f64,
    },
    MissingGate {
        node: NodeId,
    },
    MissingFanin {
        node: NodeId,
        pin: usize,
    },
    InvalidPrimaryInput {
        node: NodeId,
    },
    InvalidPrimaryOutput {
        node: NodeId,
    },
    PinTimingMismatch {
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
    CycleDetected {
        node: NodeId,
    },
    VirtualNetwork(VirtualNetworkError),
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for VirtualDelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGateName => write!(f, "virtual-delay gate name cannot be empty"),
            Self::InvalidTiming { gate, pin } => {
                write!(f, "gate '{gate}' has invalid timing on pin {pin}")
            }
            Self::InvalidLoad { node, load } => {
                write!(f, "node {} has invalid load {load}", node.index())
            }
            Self::MissingGate { node } => {
                write!(f, "node {} has no mapped gate timing", node.index())
            }
            Self::MissingFanin { node, pin } => {
                write!(f, "node {} is missing fanin pin {pin}", node.index())
            }
            Self::InvalidPrimaryInput { node } => {
                write!(f, "node {} is not a primary input", node.index())
            }
            Self::InvalidPrimaryOutput { node } => {
                write!(f, "node {} is not a primary output", node.index())
            }
            Self::PinTimingMismatch {
                gate,
                expected,
                actual,
            } => write!(
                f,
                "gate '{gate}' expected {expected} pin timing entries but has {actual}"
            ),
            Self::LoadExceedsPinLimit {
                gate,
                pin,
                load,
                max_load,
            } => write!(
                f,
                "gate '{gate}' pin {pin} load {load} exceeds max load {max_load}"
            ),
            Self::CycleDetected { node } => {
                write!(
                    f,
                    "virtual-delay traversal found a cycle at node {}",
                    node.index()
                )
            }
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => write!(f, "{operation} requires unavailable native SIS integration"),
        }
    }
}

impl Error for VirtualDelayError {}

impl From<VirtualNetworkError> for VirtualDelayError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

pub fn full_sis_virtual_delay_unavailable() -> Result<VirtualDelayState, VirtualDelayError> {
    Err(VirtualDelayError::MissingSisPorts {
        operation: "virtual_delay full SIS graph timing",
    })
}

pub fn compute_arrival_times(
    network: &mut VirtualMappedNetwork,
    constraints: &VirtualDelayConstraints,
    library: &VirtualDelayLibrary,
    options: VirtualDelayOptions,
) -> Result<VirtualDelayState, VirtualDelayError> {
    network.setup_gate_links()?;
    update_gate_link_loads(network, constraints, library, options)?;

    let mut state = VirtualDelayState {
        node_arrivals: vec![None; network.nodes().len()],
        input_arrivals: vec![Vec::new(); network.nodes().len()],
        constant_warning_count: 0,
    };
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();

    for output in network.outputs().to_vec() {
        compute_arrival_rec(
            network,
            output,
            constraints,
            library,
            options,
            &mut state,
            &mut visiting,
            &mut visited,
        )?;
    }

    Ok(state)
}

pub fn set_po_required_times(
    network: &mut VirtualMappedNetwork,
    constraints: &VirtualDelayConstraints,
    state: &VirtualDelayState,
) -> Result<DelayTime, VirtualDelayError> {
    let max_po_arrival = network
        .outputs()
        .iter()
        .copied()
        .filter_map(|output| state.arrival(output))
        .fold(MINUS_INFINITY, max_delay_time);

    for output in network.outputs().to_vec() {
        ensure_primary_output(network, output)?;
        let configured = constraints.output_timing(output).required;
        let required = if is_minus_infinity(configured) {
            max_po_arrival
        } else {
            configured
        };
        network
            .node_mut(output)
            .ok_or(VirtualNetworkError::MissingNode(output))?
            .required = required;
        network.update_link_required_times(output, &[])?;
    }

    Ok(max_po_arrival)
}

pub fn set_po_negative_required_times(
    network: &mut VirtualMappedNetwork,
    constraints: &VirtualDelayConstraints,
) -> Result<f64, VirtualDelayError> {
    let max_po_required = network
        .outputs()
        .iter()
        .copied()
        .map(|output| constraints.output_timing(output).required)
        .fold(MINUS_INFINITY, max_delay_time);
    let max_value = max_po_required.rise.max(max_po_required.fall);
    let mut shift = 1.0;
    while max_value.is_finite() && shift < max_value {
        shift *= 10.0;
    }

    for output in network.outputs().to_vec() {
        ensure_primary_output(network, output)?;
        let configured = constraints.output_timing(output).required;
        let required = DelayTime::new(configured.rise - shift, configured.fall - shift);
        network
            .node_mut(output)
            .ok_or(VirtualNetworkError::MissingNode(output))?
            .required = required;
        network.update_link_required_times(output, &[])?;
    }

    Ok(shift)
}

pub fn compute_node_required_time(
    network: &mut VirtualMappedNetwork,
    node: NodeId,
    library: &VirtualDelayLibrary,
    options: VirtualDelayOptions,
) -> Result<DelayTime, VirtualDelayError> {
    let kind = network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?
        .kind;

    if kind != NodeKind::PrimaryInput
        && network
            .node(node)
            .and_then(|item| item.gate.as_ref())
            .is_none()
    {
        return Err(VirtualDelayError::MissingGate { node });
    }

    let required = network.compute_min_required(node).unwrap_or(MINUS_INFINITY);
    network
        .node_mut(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?
        .required = required;

    if kind == NodeKind::PrimaryInput {
        return Ok(required);
    }

    let load = compute_node_load(network, node, options)?;
    let timing = timing_for_node(network, node, library)?;
    if timing.is_wire {
        network.update_link_required_times(node, &[required])?;
        return Ok(required);
    }

    let pin_required = timing
        .pins
        .iter()
        .copied()
        .enumerate()
        .map(|(pin, pin_timing)| {
            required_at_pin(&timing.gate_name, pin, pin_timing, required, load)
        })
        .collect::<Result<Vec<_>, _>>()?;
    network.update_link_required_times(node, &pin_required)?;

    Ok(required)
}

fn update_gate_link_loads(
    network: &mut VirtualMappedNetwork,
    constraints: &VirtualDelayConstraints,
    library: &VirtualDelayLibrary,
    options: VirtualDelayOptions,
) -> Result<(), VirtualDelayError> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();

    for output in network.outputs().to_vec() {
        update_gate_link_loads_rec(
            network,
            output,
            constraints,
            library,
            options,
            &mut visiting,
            &mut visited,
        )?;
    }

    for input in network.inputs().to_vec() {
        let load = compute_node_load(network, input, options)?;
        network
            .node_mut(input)
            .ok_or(VirtualNetworkError::MissingNode(input))?
            .load = load;
    }

    Ok(())
}

fn update_gate_link_loads_rec(
    network: &mut VirtualMappedNetwork,
    node: NodeId,
    constraints: &VirtualDelayConstraints,
    library: &VirtualDelayLibrary,
    options: VirtualDelayOptions,
    visiting: &mut BTreeSet<NodeId>,
    visited: &mut BTreeSet<NodeId>,
) -> Result<(), VirtualDelayError> {
    if visited.contains(&node) {
        return Ok(());
    }
    if !visiting.insert(node) {
        return Err(VirtualDelayError::CycleDetected { node });
    }

    let kind = network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?
        .kind;

    if kind == NodeKind::PrimaryOutput {
        let load = constraints.output_timing(node).load;
        validate_load(node, load)?;
        network
            .node_mut(node)
            .ok_or(VirtualNetworkError::MissingNode(node))?
            .load = load;
        let source = primary_output_fanin(network, node)?;
        if let SourceRef::Node(source) = source {
            network.add_to_gate_link(
                SourceRef::Node(source),
                GateLink {
                    node,
                    pin: -1,
                    load,
                    slack: 0.0,
                    required: MINUS_INFINITY,
                },
            )?;
            visiting.remove(&node);
            visited.insert(node);
            return update_gate_link_loads_rec(
                network,
                source,
                constraints,
                library,
                options,
                visiting,
                visited,
            );
        }
        visiting.remove(&node);
        visited.insert(node);
        return Ok(());
    }

    let links = network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?
        .gate_links()
        .copied()
        .collect::<Vec<_>>();
    for link in links {
        update_gate_link_loads_rec(
            network,
            link.node,
            constraints,
            library,
            options,
            visiting,
            visited,
        )?;
    }

    let load = compute_node_load(network, node, options)?;
    network
        .node_mut(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?
        .load = load;

    if kind == NodeKind::PrimaryInput {
        visiting.remove(&node);
        visited.insert(node);
        return Ok(());
    }

    let timing = timing_for_node(network, node, library)?;
    for (pin, pin_timing) in timing.pins.iter().copied().enumerate() {
        validate_pin_load(
            &timing.gate_name,
            pin,
            pin_timing.input_load,
            pin_timing.max_load,
        )?;
        let source = fanin_at(network, node, pin)?;
        if let SourceRef::Node(source) = source {
            network.add_to_gate_link(
                SourceRef::Node(source),
                GateLink {
                    node,
                    pin: pin as isize,
                    load: pin_timing.input_load,
                    slack: 0.0,
                    required: MINUS_INFINITY,
                },
            )?;
        }
    }

    visiting.remove(&node);
    visited.insert(node);
    Ok(())
}

fn compute_arrival_rec(
    network: &VirtualMappedNetwork,
    node: NodeId,
    constraints: &VirtualDelayConstraints,
    library: &VirtualDelayLibrary,
    options: VirtualDelayOptions,
    state: &mut VirtualDelayState,
    visiting: &mut BTreeSet<NodeId>,
    visited: &mut BTreeSet<NodeId>,
) -> Result<DelayTime, VirtualDelayError> {
    if visited.contains(&node) {
        return state
            .arrival(node)
            .ok_or(VirtualNetworkError::MissingNode(node).into());
    }
    if !visiting.insert(node) {
        return Err(VirtualDelayError::CycleDetected { node });
    }

    let item = network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?;
    let arrival = match item.kind {
        NodeKind::PrimaryInput => {
            let timing = constraints.input_timing(node);
            DelayTime::new(
                timing.arrival.rise + timing.drive.rise * item.load,
                timing.arrival.fall + timing.drive.fall * item.load,
            )
        }
        NodeKind::PrimaryOutput => {
            let source = primary_output_fanin(network, node)?;
            match source {
                SourceRef::Node(source) => compute_arrival_rec(
                    network,
                    source,
                    constraints,
                    library,
                    options,
                    state,
                    visiting,
                    visited,
                )?,
                SourceRef::ConstantZero | SourceRef::ConstantOne => {
                    if options.warn_on_constants {
                        state.constant_warning_count += 1;
                    }
                    ZERO_DELAY
                }
            }
        }
        NodeKind::Internal => compute_internal_arrival(
            network,
            node,
            constraints,
            library,
            options,
            state,
            visiting,
            visited,
        )?,
    };

    state.node_arrivals[node.index()] = Some(arrival);
    visiting.remove(&node);
    visited.insert(node);
    Ok(arrival)
}

fn compute_internal_arrival(
    network: &VirtualMappedNetwork,
    node: NodeId,
    constraints: &VirtualDelayConstraints,
    library: &VirtualDelayLibrary,
    options: VirtualDelayOptions,
    state: &mut VirtualDelayState,
    visiting: &mut BTreeSet<NodeId>,
    visited: &mut BTreeSet<NodeId>,
) -> Result<DelayTime, VirtualDelayError> {
    let gate = network
        .node(node)
        .and_then(|item| item.gate.as_ref())
        .ok_or(VirtualDelayError::MissingGate { node })?;

    match gate {
        GateKind::Zero | GateKind::One => {
            if options.warn_on_constants {
                state.constant_warning_count += 1;
            }
            return Ok(ZERO_DELAY);
        }
        GateKind::Wire => {
            let source = fanin_at(network, node, 0)?;
            let arrival = source_arrival(
                network,
                source,
                constraints,
                library,
                options,
                state,
                visiting,
                visited,
            )?;
            state.input_arrivals[node.index()] = vec![arrival];
            return Ok(arrival);
        }
        _ => {}
    }

    let timing = timing_for_node(network, node, library)?;
    let load = network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?
        .load;
    validate_load(node, load)?;

    let mut input_arrivals = Vec::with_capacity(timing.pins.len());
    for pin in 0..timing.pins.len() {
        let source = fanin_at(network, node, pin)?;
        input_arrivals.push(source_arrival(
            network,
            source,
            constraints,
            library,
            options,
            state,
            visiting,
            visited,
        )?);
    }

    let arrival = simulate_gate_arrival(&timing, &input_arrivals, load)?;
    state.input_arrivals[node.index()] = input_arrivals;
    Ok(arrival)
}

fn source_arrival(
    network: &VirtualMappedNetwork,
    source: SourceRef,
    constraints: &VirtualDelayConstraints,
    library: &VirtualDelayLibrary,
    options: VirtualDelayOptions,
    state: &mut VirtualDelayState,
    visiting: &mut BTreeSet<NodeId>,
    visited: &mut BTreeSet<NodeId>,
) -> Result<DelayTime, VirtualDelayError> {
    match source {
        SourceRef::Node(node) => compute_arrival_rec(
            network,
            node,
            constraints,
            library,
            options,
            state,
            visiting,
            visited,
        ),
        SourceRef::ConstantZero | SourceRef::ConstantOne => {
            if options.warn_on_constants {
                state.constant_warning_count += 1;
            }
            Ok(ZERO_DELAY)
        }
    }
}

fn simulate_gate_arrival(
    timing: &VirtualDelayGateTiming,
    input_arrivals: &[DelayTime],
    load: f64,
) -> Result<DelayTime, VirtualDelayError> {
    if timing.pins.len() != input_arrivals.len() {
        return Err(VirtualDelayError::PinTimingMismatch {
            gate: timing.gate_name.clone(),
            expected: timing.pins.len(),
            actual: input_arrivals.len(),
        });
    }

    let mut arrival = MINUS_INFINITY;
    for (pin, (pin_timing, input)) in timing
        .pins
        .iter()
        .copied()
        .zip(input_arrivals.iter().copied())
        .enumerate()
    {
        validate_pin_load(&timing.gate_name, pin, load, pin_timing.max_load)?;
        let rise_delay = pin_timing.rise_delay(load);
        let fall_delay = pin_timing.fall_delay(load);
        let pin_arrival = match pin_timing.phase {
            TimingPhase::NonInverting => {
                DelayTime::new(input.rise + rise_delay, input.fall + fall_delay)
            }
            TimingPhase::Inverting => {
                DelayTime::new(input.fall + rise_delay, input.rise + fall_delay)
            }
            TimingPhase::Unknown => DelayTime::new(
                input.rise.max(input.fall) + rise_delay,
                input.rise.max(input.fall) + fall_delay,
            ),
        };
        arrival = max_delay_time(arrival, pin_arrival);
    }

    Ok(arrival)
}

fn required_at_pin(
    gate: &str,
    pin: usize,
    timing: VirtualDelayPinTiming,
    required: DelayTime,
    load: f64,
) -> Result<DelayTime, VirtualDelayError> {
    validate_pin_load(gate, pin, load, timing.max_load)?;
    let rise_limit = required.rise - timing.rise_delay(load);
    let fall_limit = required.fall - timing.fall_delay(load);
    let pin_required = match timing.phase {
        TimingPhase::NonInverting => DelayTime::new(rise_limit, fall_limit),
        TimingPhase::Inverting => DelayTime::new(fall_limit, rise_limit),
        TimingPhase::Unknown => {
            let limit = rise_limit.min(fall_limit);
            DelayTime::new(limit, limit)
        }
    };

    Ok(pin_required)
}

fn timing_for_node(
    network: &VirtualMappedNetwork,
    node: NodeId,
    library: &VirtualDelayLibrary,
) -> Result<VirtualDelayGateTiming, VirtualDelayError> {
    let item = network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?;
    let gate = item
        .gate
        .as_ref()
        .ok_or(VirtualDelayError::MissingGate { node })?;
    library.gate_timing(gate, item.save_binding.len())
}

fn repeated_default_timing(
    gate_name: &str,
    pin_timing: VirtualDelayPinTiming,
    fanin_count: usize,
) -> Result<VirtualDelayGateTiming, VirtualDelayError> {
    VirtualDelayGateTiming::new(gate_name.to_string(), vec![pin_timing; fanin_count])
}

fn repeated_or_exact_timing(
    timing: &VirtualDelayGateTiming,
    fanin_count: usize,
) -> Result<VirtualDelayGateTiming, VirtualDelayError> {
    if timing.pins.len() == fanin_count {
        return Ok(timing.clone());
    }
    if timing.pins.len() == 1 && fanin_count > 1 {
        return VirtualDelayGateTiming::new(
            timing.gate_name.clone(),
            vec![timing.pins[0]; fanin_count],
        );
    }

    Err(VirtualDelayError::PinTimingMismatch {
        gate: timing.gate_name.clone(),
        expected: fanin_count,
        actual: timing.pins.len(),
    })
}

fn compute_node_load(
    network: &VirtualMappedNetwork,
    node: NodeId,
    options: VirtualDelayOptions,
) -> Result<f64, VirtualDelayError> {
    let load = network.compute_load(node, |fanout_count| {
        options.wire_load.load_for_fanout_count(fanout_count)
    });
    validate_load(node, load)?;
    Ok(load)
}

fn fanin_at(
    network: &VirtualMappedNetwork,
    node: NodeId,
    pin: usize,
) -> Result<SourceRef, VirtualDelayError> {
    network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?
        .save_binding
        .get(pin)
        .copied()
        .ok_or(VirtualDelayError::MissingFanin { node, pin })
}

fn primary_output_fanin(
    network: &VirtualMappedNetwork,
    node: NodeId,
) -> Result<SourceRef, VirtualDelayError> {
    let item = network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?;
    if item.kind != NodeKind::PrimaryOutput || item.save_binding.len() != 1 {
        return Err(VirtualDelayError::InvalidPrimaryOutput { node });
    }

    Ok(item.save_binding[0])
}

fn ensure_primary_output(
    network: &VirtualMappedNetwork,
    node: NodeId,
) -> Result<(), VirtualDelayError> {
    if network
        .node(node)
        .ok_or(VirtualNetworkError::MissingNode(node))?
        .kind
        != NodeKind::PrimaryOutput
    {
        return Err(VirtualDelayError::InvalidPrimaryOutput { node });
    }

    Ok(())
}

fn validate_load(node: NodeId, load: f64) -> Result<(), VirtualDelayError> {
    if !load.is_finite() || load < 0.0 {
        return Err(VirtualDelayError::InvalidLoad { node, load });
    }

    Ok(())
}

fn validate_pin_load(
    gate: &str,
    pin: usize,
    load: f64,
    max_load: f64,
) -> Result<(), VirtualDelayError> {
    if max_load.is_finite() && load > max_load {
        return Err(VirtualDelayError::LoadExceedsPinLimit {
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

fn is_minus_infinity(value: DelayTime) -> bool {
    value.rise == f64::NEG_INFINITY && value.fall == f64::NEG_INFINITY
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> (VirtualMappedNetwork, NodeId, NodeId, NodeId, NodeId) {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let n1 = network.add_gate(
            "n1",
            GateKind::And,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        let y = network
            .add_primary_output("y", SourceRef::Node(n1))
            .unwrap();
        (network, a, b, n1, y)
    }

    #[test]
    fn computes_link_loads_and_gate_arrival_times() {
        let (mut network, a, b, n1, y) = sample_network();
        let constraints = VirtualDelayConstraints {
            inputs: vec![
                VirtualDelayInput {
                    node: a,
                    timing: PrimaryInputTiming {
                        arrival: DelayTime::new(1.0, 2.0),
                        drive: DelayTime::new(0.5, 0.25),
                    },
                },
                VirtualDelayInput {
                    node: b,
                    timing: PrimaryInputTiming {
                        arrival: DelayTime::new(0.0, 1.0),
                        drive: ZERO_DELAY,
                    },
                },
            ],
            outputs: vec![VirtualDelayOutput {
                node: y,
                timing: PrimaryOutputTiming {
                    load: 3.0,
                    required: DelayTime::new(10.0, 10.0),
                },
            }],
        };
        let library = VirtualDelayLibrary::new(
            VirtualDelayPinTiming::new(TimingPhase::NonInverting, 2.0, 10.0, 1.0, 0.5, 2.0, 0.25),
            Vec::new(),
        )
        .unwrap();

        let state = compute_arrival_times(
            &mut network,
            &constraints,
            &library,
            VirtualDelayOptions::default(),
        )
        .unwrap();

        assert_eq!(network.node(y).unwrap().load, 3.0);
        assert_eq!(network.node(n1).unwrap().load, 3.0);
        assert_eq!(network.gate_link(a, n1, 0).unwrap().load, 2.0);
        assert_eq!(network.gate_link(b, n1, 1).unwrap().load, 2.0);
        assert_eq!(state.arrival(a).unwrap(), DelayTime::new(2.0, 2.5));
        assert_eq!(state.arrival(n1).unwrap(), DelayTime::new(4.5, 5.25));
        assert_eq!(state.arrival(y).unwrap(), DelayTime::new(4.5, 5.25));
        assert_eq!(
            state.arrival_inputs(n1).unwrap(),
            &[DelayTime::new(2.0, 2.5), DelayTime::new(0.0, 1.0)]
        );
    }

    #[test]
    fn computes_required_times_for_po_and_gate_inputs() {
        let (mut network, a, b, n1, y) = sample_network();
        let constraints = VirtualDelayConstraints {
            inputs: Vec::new(),
            outputs: vec![VirtualDelayOutput {
                node: y,
                timing: PrimaryOutputTiming {
                    load: 2.0,
                    required: DelayTime::new(9.0, 11.0),
                },
            }],
        };
        let library = VirtualDelayLibrary::new(
            VirtualDelayPinTiming::new(TimingPhase::NonInverting, 1.5, 10.0, 1.0, 0.5, 2.0, 0.25),
            Vec::new(),
        )
        .unwrap();
        let state = compute_arrival_times(
            &mut network,
            &constraints,
            &library,
            VirtualDelayOptions::default(),
        )
        .unwrap();

        set_po_required_times(&mut network, &constraints, &state).unwrap();
        let required =
            compute_node_required_time(&mut network, n1, &library, VirtualDelayOptions::default())
                .unwrap();

        assert_eq!(required, DelayTime::new(9.0, 11.0));
        assert_eq!(
            network.gate_link(a, n1, 0).unwrap().required,
            DelayTime::new(7.0, 8.5)
        );
        assert_eq!(
            network.gate_link(b, n1, 1).unwrap().required,
            DelayTime::new(7.0, 8.5)
        );
    }

    #[test]
    fn negative_po_required_times_preserve_relative_shift() {
        let (mut network, _a, _b, _n1, y) = sample_network();
        network.setup_gate_links().unwrap();
        let constraints = VirtualDelayConstraints {
            inputs: Vec::new(),
            outputs: vec![VirtualDelayOutput {
                node: y,
                timing: PrimaryOutputTiming {
                    load: 0.0,
                    required: DelayTime::new(12.0, 4.0),
                },
            }],
        };

        let shift = set_po_negative_required_times(&mut network, &constraints).unwrap();

        assert_eq!(shift, 100.0);
        assert_eq!(
            network.node(y).unwrap().required,
            DelayTime::new(-88.0, -96.0)
        );

        let source = include_str!("virtual_del.rs");
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
