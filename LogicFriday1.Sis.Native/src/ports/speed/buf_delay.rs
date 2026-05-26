//! Native Rust port of feasible behavior in `sis/speed/buf_delay.c`.
//!
//! The C file is a small timing helper layer for the SIS buffering package. It
//! mostly performs edge-specific required-time arithmetic, but it also reaches
//! through `node_t`, `network_t`, delay pins, mapped library buffers, and
//! primary input/output delay parameters. This module keeps the arithmetic
//! native and explicit, and models the SIS-bound data as owned Rust structs.
//! Missing C-backed state is reported as typed errors instead of legacy C ABI
//! shims.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;
pub const INFINITY_TIME: f64 = f64::INFINITY;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead: &'static str,
    pub c_file: &'static str,
}

pub const REQUIRED_PORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.133",
        c_file: "LogicSynthesis/sis/delay/delay.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.257",
        c_file: "LogicSynthesis/sis/map/library.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.258",
        c_file: "LogicSynthesis/sis/map/libutil.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.305",
        c_file: "LogicSynthesis/sis/network/network_util.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.313",
        c_file: "LogicSynthesis/sis/node/fan.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.318",
        c_file: "LogicSynthesis/sis/node/node.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.464",
        c_file: "LogicSynthesis/sis/speed/buf_util.c",
    },
];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub const fn zero() -> Self {
        Self {
            rise: 0.0,
            fall: 0.0,
        }
    }

    pub const fn pos_large() -> Self {
        Self {
            rise: POS_LARGE,
            fall: POS_LARGE,
        }
    }

    pub const fn infinity() -> Self {
        Self {
            rise: INFINITY_TIME,
            fall: INFINITY_TIME,
        }
    }

    pub fn edge_min(self, other: Self) -> Self {
        Self {
            rise: self.rise.min(other.rise),
            fall: self.fall.min(other.fall),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayPin {
    pub block: DelayTime,
    pub drive: DelayTime,
    pub phase: PinPhase,
    pub load: f64,
    pub max_load: f64,
}

impl DelayPin {
    pub const fn new(
        block: DelayTime,
        drive: DelayTime,
        phase: PinPhase,
        load: f64,
        max_load: f64,
    ) -> Self {
        Self {
            block,
            drive,
            phase,
            load,
            max_load,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WireLoadModel {
    Zero,
    Constant(f64),
    Linear { base: f64, slope: f64 },
}

impl WireLoadModel {
    pub fn load(self, pins: usize) -> f64 {
        match self {
            Self::Zero => 0.0,
            Self::Constant(load) => load,
            Self::Linear { base, slope } => base + slope * pins as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferImplementationKind {
    None,
    Buffer,
    Gate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferSpec {
    pub name: String,
    pub area: f64,
    pub ip_load: f64,
    pub max_load: f64,
    pub phase: PinPhase,
    pub block: DelayTime,
    pub drive: DelayTime,
}

impl BufferSpec {
    pub fn new(
        name: impl Into<String>,
        phase: PinPhase,
        ip_load: f64,
        block: DelayTime,
        drive: DelayTime,
    ) -> Self {
        Self {
            name: name.into(),
            area: 0.0,
            ip_load,
            max_load: POS_LARGE,
            phase,
            block,
            drive,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferNodeTiming {
    pub implementation: BufferImplementationKind,
    pub buffer: Option<BufferSpec>,
    pub critical_fanin: Option<usize>,
    pub load: f64,
    pub req_time: DelayTime,
    pub prev_drive: DelayTime,
    pub prev_phase: PinPhase,
}

impl Default for BufferNodeTiming {
    fn default() -> Self {
        Self {
            implementation: BufferImplementationKind::None,
            buffer: None,
            critical_fanin: None,
            load: 0.0,
            req_time: DelayTime::pos_large(),
            prev_drive: DelayTime::zero(),
            prev_phase: PinPhase::NotGiven,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayEnvironment {
    pub input_drive: Option<DelayTime>,
    pub output_load: Option<f64>,
    pub wire_required_times: Vec<DelayTime>,
}

impl DelayEnvironment {
    pub fn empty() -> Self {
        Self {
            input_drive: None,
            output_load: None,
            wire_required_times: Vec::new(),
        }
    }
}

impl Default for DelayEnvironment {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferDelayNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub pin_delays: Vec<DelayPin>,
    pub buffer: BufferNodeTiming,
    pub delay: DelayEnvironment,
}

impl BufferDelayNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            pin_delays: Vec::new(),
            buffer: BufferNodeTiming::default(),
            delay: DelayEnvironment::default(),
        }
    }

    pub fn with_pin_delays(mut self, pin_delays: Vec<DelayPin>) -> Self {
        self.pin_delays = pin_delays;
        self
    }

    pub fn with_buffer(mut self, buffer: BufferSpec) -> Self {
        self.buffer.implementation = BufferImplementationKind::Buffer;
        self.buffer.buffer = Some(buffer);
        self
    }

    pub fn with_gate(mut self, critical_fanin: usize) -> Self {
        self.buffer.implementation = BufferImplementationKind::Gate;
        self.buffer.critical_fanin = Some(critical_fanin);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferDelayNetwork {
    nodes: Vec<BufferDelayNode>,
    primary_inputs: Vec<NodeId>,
    primary_outputs: Vec<NodeId>,
    pub wire_load: WireLoadModel,
}

impl BufferDelayNetwork {
    pub fn new(wire_load: WireLoadModel) -> Self {
        Self {
            nodes: Vec::new(),
            primary_inputs: Vec::new(),
            primary_outputs: Vec::new(),
            wire_load,
        }
    }

    pub fn add_node(&mut self, node: BufferDelayNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        match node.kind {
            NodeKind::PrimaryInput => self.primary_inputs.push(id),
            NodeKind::PrimaryOutput => self.primary_outputs.push(id),
            NodeKind::Internal => {}
        }
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&BufferDelayNode, BufDelayError> {
        self.nodes.get(id.0).ok_or(BufDelayError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut BufferDelayNode, BufDelayError> {
        self.nodes
            .get_mut(id.0)
            .ok_or(BufDelayError::UnknownNode(id))
    }

    pub fn primary_inputs(&self) -> &[NodeId] {
        &self.primary_inputs
    }

    pub fn primary_outputs(&self) -> &[NodeId] {
        &self.primary_outputs
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferDelayParams {
    pub model: DelayModel,
    pub auto_route: f64,
    pub interactive: bool,
    pub buffers: Vec<BufferSpec>,
    pub num_inv: usize,
}

impl BufferDelayParams {
    pub fn new(model: DelayModel) -> Self {
        Self {
            model,
            auto_route: 0.0,
            interactive: false,
            buffers: Vec::new(),
            num_inv: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferPinRequirement {
    pub required_time: DelayTime,
    pub load: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipoDefaultUpdate {
    pub output_load_nodes: Vec<NodeId>,
    pub input_drive_nodes: Vec<NodeId>,
}

impl PipoDefaultUpdate {
    pub fn any_changed(&self) -> bool {
        !self.output_load_nodes.is_empty() || !self.input_drive_nodes.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BufDelayError {
    UnknownNode(NodeId),
    MissingFanin {
        node: NodeId,
        pin: usize,
    },
    MissingCriticalFanin(NodeId),
    MissingBuffer(NodeId),
    MissingPinDelay {
        node: NodeId,
        pin: usize,
        dependencies: &'static [PortDependency],
    },
    MissingWireRequiredTime {
        node: NodeId,
        pin: usize,
        dependencies: &'static [PortDependency],
    },
    MissingDefaultInverter {
        num_inv: usize,
        buffer_count: usize,
    },
    PartitionOutOfRange {
        requested: usize,
        available: usize,
    },
}

impl fmt::Display for BufDelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown buffer-delay node {:?}", node),
            Self::MissingFanin { node, pin } => {
                write!(f, "node {:?} has no fanin at pin {pin}", node)
            }
            Self::MissingCriticalFanin(node) => {
                write!(f, "node {:?} has no selected critical fanin", node)
            }
            Self::MissingBuffer(node) => {
                write!(
                    f,
                    "node {:?} is marked as a buffer without buffer data",
                    node
                )
            }
            Self::MissingPinDelay { node, pin, .. } => write!(
                f,
                "node {:?} has no native delay pin for pin {pin}; SIS get_pin_delay is not ported",
                node
            ),
            Self::MissingWireRequiredTime { node, pin, .. } => write!(
                f,
                "node {:?} has no native wire required time for pin {pin}; SIS delay_wire_required_time is not ported",
                node
            ),
            Self::MissingDefaultInverter {
                num_inv,
                buffer_count,
            } => write!(
                f,
                "cannot choose C default inverter from {num_inv} inverters and {buffer_count} buffers",
            ),
            Self::PartitionOutOfRange {
                requested,
                available,
            } => write!(
                f,
                "required-time partition requested {requested} entries but only {available} are available",
            ),
        }
    }
}

impl Error for BufDelayError {}

pub fn buffer_pin_requirement(
    fanout: &BufferDelayNode,
    fanout_id: NodeId,
    params: &BufferDelayParams,
    pin: usize,
) -> Result<BufferPinRequirement, BufDelayError> {
    let required_time = fanout.delay.wire_required_times.get(pin).copied().ok_or(
        BufDelayError::MissingWireRequiredTime {
            node: fanout_id,
            pin,
            dependencies: REQUIRED_PORT_BEADS,
        },
    )?;
    let pin_delay = pin_delay(fanout, fanout_id, pin)?;

    Ok(BufferPinRequirement {
        required_time,
        load: pin_delay.load + params.auto_route,
    })
}

pub fn pin_load(node: &BufferDelayNode, node_id: NodeId) -> Result<f64, BufDelayError> {
    match node.kind {
        NodeKind::PrimaryInput => Ok(0.0),
        _ if node.buffer.implementation == BufferImplementationKind::Buffer => Ok(node
            .buffer
            .buffer
            .as_ref()
            .ok_or(BufDelayError::MissingBuffer(node_id))?
            .ip_load),
        _ => {
            let cfi = node
                .buffer
                .critical_fanin
                .ok_or(BufDelayError::MissingCriticalFanin(node_id))?;
            Ok(pin_delay(node, node_id, cfi)?.load)
        }
    }
}

pub fn input_drive(
    network: &BufferDelayNetwork,
    node_id: NodeId,
) -> Result<(DelayTime, PinPhase), BufDelayError> {
    let node = network.node(node_id)?;
    if node.kind == NodeKind::PrimaryInput {
        return Ok((DelayTime::zero(), PinPhase::NonInverting));
    }

    let cfi = node
        .buffer
        .critical_fanin
        .ok_or(BufDelayError::MissingCriticalFanin(node_id))?;
    let fanin_id = *node.fanins.get(cfi).ok_or(BufDelayError::MissingFanin {
        node: node_id,
        pin: cfi,
    })?;
    let fanin = network.node(fanin_id)?;

    if fanin.buffer.implementation == BufferImplementationKind::Buffer {
        let buffer = fanin
            .buffer
            .buffer
            .as_ref()
            .ok_or(BufDelayError::MissingBuffer(fanin_id))?;
        Ok((buffer.drive, buffer.phase))
    } else {
        let pin = pin_delay(fanin, fanin_id, 0)?;
        Ok((pin.drive, pin.phase))
    }
}

pub fn subtract_delay(
    phase: PinPhase,
    block: DelayTime,
    drive: DelayTime,
    load: f64,
    req: &mut DelayTime,
) {
    let delay = DelayTime {
        rise: block.rise + drive.rise * load,
        fall: block.fall + drive.fall * load,
    };
    compute_required_time(phase, req, delay);
}

pub fn compute_required_time(phase: PinPhase, req: &mut DelayTime, delay: DelayTime) {
    let mut input_req = DelayTime::infinity();

    if matches!(phase, PinPhase::Inverting | PinPhase::Neither) {
        input_req.rise = input_req.rise.min(req.fall - delay.fall);
        input_req.fall = input_req.fall.min(req.rise - delay.rise);
    }
    if matches!(phase, PinPhase::NonInverting | PinPhase::Neither) {
        input_req.rise = input_req.rise.min(req.rise - delay.rise);
        input_req.fall = input_req.fall.min(req.fall - delay.fall);
    }

    *req = input_req;
}

pub fn compute_buffer_required_time(
    network: &mut BufferDelayNetwork,
    node_id: NodeId,
    req_times: &[DelayTime],
    cap_k: f64,
    partition_index: usize,
    added_buffer_id: NodeId,
) -> Result<DelayTime, BufDelayError> {
    if partition_index > req_times.len() {
        return Err(BufDelayError::PartitionOutOfRange {
            requested: partition_index,
            available: req_times.len(),
        });
    }

    let (phase, drive, block) = driving_delay(network.node(node_id)?, node_id)?;
    let mut best = req_times
        .iter()
        .take(partition_index)
        .copied()
        .fold(DelayTime::pos_large(), DelayTime::edge_min);
    best = best.edge_min(network.node(added_buffer_id)?.buffer.req_time);

    let added_buffer_load = pin_load(network.node(added_buffer_id)?, added_buffer_id)?;
    let load = cap_k + added_buffer_load + network.wire_load.load(1);
    let delay = DelayTime {
        rise: block.rise + drive.rise * load,
        fall: block.fall + drive.fall * load,
    };
    compute_required_time(phase, &mut best, delay);

    network.node_mut(node_id)?.buffer.req_time = best;
    Ok(best)
}

pub fn set_pipo_defaults(
    network: &mut BufferDelayNetwork,
    params: &BufferDelayParams,
) -> Result<PipoDefaultUpdate, BufDelayError> {
    let default_index = default_inverter_index(params)?;
    let default_load = if params.model == DelayModel::Mapped {
        params.buffers[default_index].ip_load
    } else {
        1.0
    };
    let default_drive = if params.model == DelayModel::Mapped {
        params.buffers[default_index].drive
    } else {
        DelayTime::new(0.2, 0.2)
    };

    let mut update = PipoDefaultUpdate {
        output_load_nodes: Vec::new(),
        input_drive_nodes: Vec::new(),
    };

    let outputs = network.primary_outputs().to_vec();
    for output in outputs {
        let node = network.node_mut(output)?;
        if node.delay.output_load.is_none() {
            node.delay.output_load = Some(default_load);
            update.output_load_nodes.push(output);
        }
    }

    let inputs = network.primary_inputs().to_vec();
    for input in inputs {
        let node = network.node_mut(input)?;
        if node.delay.input_drive.is_none() {
            node.delay.input_drive = Some(default_drive);
            update.input_drive_nodes.push(input);
        }
    }

    Ok(update)
}

pub fn buffer_delay_from_sis_network() -> Result<(), BufDelayError> {
    Err(BufDelayError::MissingWireRequiredTime {
        node: NodeId(0),
        pin: 0,
        dependencies: REQUIRED_PORT_BEADS,
    })
}

fn default_inverter_index(params: &BufferDelayParams) -> Result<usize, BufDelayError> {
    if params.num_inv == 0 || params.buffers.len() < params.num_inv {
        return Err(BufDelayError::MissingDefaultInverter {
            num_inv: params.num_inv,
            buffer_count: params.buffers.len(),
        });
    }

    Ok(if params.num_inv > 1 {
        params.num_inv - 2
    } else {
        params.num_inv - 1
    })
}

fn driving_delay(
    node: &BufferDelayNode,
    node_id: NodeId,
) -> Result<(PinPhase, DelayTime, DelayTime), BufDelayError> {
    match node.kind {
        NodeKind::PrimaryInput => {
            let pin = pin_delay(node, node_id, 0)?;
            Ok((pin.phase, pin.drive, pin.block))
        }
        _ if node.buffer.implementation == BufferImplementationKind::Gate => {
            let cfi = node
                .buffer
                .critical_fanin
                .ok_or(BufDelayError::MissingCriticalFanin(node_id))?;
            let pin = pin_delay(node, node_id, cfi)?;
            Ok((pin.phase, pin.drive, pin.block))
        }
        _ => {
            let buffer = node
                .buffer
                .buffer
                .as_ref()
                .ok_or(BufDelayError::MissingBuffer(node_id))?;
            Ok((buffer.phase, buffer.drive, buffer.block))
        }
    }
}

fn pin_delay(
    node: &BufferDelayNode,
    node_id: NodeId,
    pin: usize,
) -> Result<DelayPin, BufDelayError> {
    node.pin_delays
        .get(pin)
        .copied()
        .ok_or(BufDelayError::MissingPinDelay {
            node: node_id,
            pin,
            dependencies: REQUIRED_PORT_BEADS,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "actual {actual} != expected {expected}"
        );
    }

    fn pin(block: (f64, f64), drive: (f64, f64), phase: PinPhase, load: f64) -> DelayPin {
        DelayPin::new(
            DelayTime::new(block.0, block.1),
            DelayTime::new(drive.0, drive.1),
            phase,
            load,
            POS_LARGE,
        )
    }

    fn inverter(name: &str, ip_load: f64, drive: DelayTime) -> BufferSpec {
        BufferSpec::new(
            name,
            PinPhase::Inverting,
            ip_load,
            DelayTime::new(1.0, 2.0),
            drive,
        )
    }

    #[test]
    fn compute_required_time_matches_phase_rules() {
        let mut req = DelayTime::new(10.0, 20.0);
        compute_required_time(PinPhase::NonInverting, &mut req, DelayTime::new(1.0, 3.0));
        assert_eq!(req, DelayTime::new(9.0, 17.0));

        let mut req = DelayTime::new(10.0, 20.0);
        compute_required_time(PinPhase::Inverting, &mut req, DelayTime::new(1.0, 3.0));
        assert_eq!(req, DelayTime::new(17.0, 9.0));

        let mut req = DelayTime::new(10.0, 20.0);
        compute_required_time(PinPhase::Neither, &mut req, DelayTime::new(1.0, 3.0));
        assert_eq!(req, DelayTime::new(9.0, 9.0));
    }

    #[test]
    fn subtract_delay_applies_block_and_drive_load_before_phase_math() {
        let mut req = DelayTime::new(30.0, 40.0);
        subtract_delay(
            PinPhase::NonInverting,
            DelayTime::new(1.0, 2.0),
            DelayTime::new(3.0, 4.0),
            5.0,
            &mut req,
        );

        assert_eq!(req, DelayTime::new(14.0, 18.0));
    }

    #[test]
    fn buffer_pin_requirement_uses_wire_required_time_and_auto_route_load() {
        let mut fanout = BufferDelayNode::new("fo", NodeKind::Internal)
            .with_pin_delays(vec![pin(
                (0.0, 0.0),
                (1.0, 1.0),
                PinPhase::NonInverting,
                2.5,
            )])
            .with_gate(0);
        fanout
            .delay
            .wire_required_times
            .push(DelayTime::new(7.0, 8.0));
        let params = BufferDelayParams {
            auto_route: 0.75,
            ..BufferDelayParams::new(DelayModel::Mapped)
        };

        assert_eq!(
            buffer_pin_requirement(&fanout, NodeId(3), &params, 0).unwrap(),
            BufferPinRequirement {
                required_time: DelayTime::new(7.0, 8.0),
                load: 3.25,
            }
        );
    }

    #[test]
    fn pin_load_follows_pi_buffer_and_gate_cases() {
        let pi = BufferDelayNode::new("pi", NodeKind::PrimaryInput);
        assert_eq!(pin_load(&pi, NodeId(0)).unwrap(), 0.0);

        let buffer = BufferDelayNode::new("b", NodeKind::Internal).with_buffer(BufferSpec::new(
            "buf",
            PinPhase::NonInverting,
            4.0,
            DelayTime::zero(),
            DelayTime::zero(),
        ));
        assert_eq!(pin_load(&buffer, NodeId(1)).unwrap(), 4.0);

        let gate = BufferDelayNode::new("g", NodeKind::Internal)
            .with_pin_delays(vec![pin(
                (0.0, 0.0),
                (0.0, 0.0),
                PinPhase::NonInverting,
                5.5,
            )])
            .with_gate(0);
        assert_eq!(pin_load(&gate, NodeId(2)).unwrap(), 5.5);
    }

    #[test]
    fn input_drive_uses_primary_input_zero_buffer_drive_or_fanin_pin_zero() {
        let mut network = BufferDelayNetwork::new(WireLoadModel::Zero);
        let pi = network.add_node(BufferDelayNode::new("pi", NodeKind::PrimaryInput));
        assert_eq!(
            input_drive(&network, pi).unwrap(),
            (DelayTime::zero(), PinPhase::NonInverting)
        );

        let b = network.add_node(BufferDelayNode::new("b", NodeKind::Internal).with_buffer(
            BufferSpec::new(
                "inv",
                PinPhase::Inverting,
                1.0,
                DelayTime::zero(),
                DelayTime::new(2.0, 3.0),
            ),
        ));
        let mut n = BufferDelayNode::new("n", NodeKind::Internal).with_gate(0);
        n.fanins.push(b);
        let n = network.add_node(n);
        assert_eq!(
            input_drive(&network, n).unwrap(),
            (DelayTime::new(2.0, 3.0), PinPhase::Inverting)
        );

        let g = network.add_node(
            BufferDelayNode::new("g", NodeKind::Internal)
                .with_pin_delays(vec![pin((0.0, 0.0), (4.0, 5.0), PinPhase::Neither, 1.0)])
                .with_gate(0),
        );
        let mut n2 = BufferDelayNode::new("n2", NodeKind::Internal).with_gate(0);
        n2.fanins.push(g);
        let n2 = network.add_node(n2);
        assert_eq!(
            input_drive(&network, n2).unwrap(),
            (DelayTime::new(4.0, 5.0), PinPhase::Neither)
        );
    }

    #[test]
    fn compute_buffer_required_time_selects_best_fanout_requirement_and_updates_node() {
        let mut network = BufferDelayNetwork::new(WireLoadModel::Constant(0.25));
        let node = network.add_node(
            BufferDelayNode::new("root", NodeKind::Internal)
                .with_pin_delays(vec![pin(
                    (1.0, 2.0),
                    (0.5, 0.25),
                    PinPhase::NonInverting,
                    1.0,
                )])
                .with_gate(0),
        );
        let mut added =
            BufferDelayNode::new("added", NodeKind::Internal).with_buffer(BufferSpec::new(
                "buf",
                PinPhase::NonInverting,
                3.0,
                DelayTime::zero(),
                DelayTime::zero(),
            ));
        added.buffer.req_time = DelayTime::new(8.0, 30.0);
        let added = network.add_node(added);

        let result = compute_buffer_required_time(
            &mut network,
            node,
            &[DelayTime::new(20.0, 6.0), DelayTime::new(9.0, 9.0)],
            1.75,
            2,
            added,
        )
        .unwrap();

        assert_close(result.rise, 4.5);
        assert_close(result.fall, 2.75);
        assert_eq!(network.node(node).unwrap().buffer.req_time, result);
    }

    #[test]
    fn set_pipo_defaults_uses_next_to_smallest_mapped_inverter_and_tracks_changes() {
        let mut network = BufferDelayNetwork::new(WireLoadModel::Zero);
        let pi = network.add_node(BufferDelayNode::new("a", NodeKind::PrimaryInput));
        let mut existing_pi = BufferDelayNode::new("b", NodeKind::PrimaryInput);
        existing_pi.delay.input_drive = Some(DelayTime::new(9.0, 9.0));
        network.add_node(existing_pi);
        let po = network.add_node(BufferDelayNode::new("z", NodeKind::PrimaryOutput));

        let params = BufferDelayParams {
            model: DelayModel::Mapped,
            auto_route: 0.0,
            interactive: true,
            buffers: vec![
                inverter("big", 4.0, DelayTime::new(8.0, 8.0)),
                inverter("default", 2.0, DelayTime::new(3.0, 4.0)),
                BufferSpec::new(
                    "noninv",
                    PinPhase::NonInverting,
                    5.0,
                    DelayTime::zero(),
                    DelayTime::zero(),
                ),
            ],
            num_inv: 2,
        };

        let update = set_pipo_defaults(&mut network, &params).unwrap();

        assert!(update.any_changed());
        assert_eq!(update.output_load_nodes, vec![po]);
        assert_eq!(update.input_drive_nodes, vec![pi]);
        assert_eq!(network.node(po).unwrap().delay.output_load, Some(4.0));
        assert_eq!(
            network.node(pi).unwrap().delay.input_drive,
            Some(DelayTime::new(8.0, 8.0))
        );
    }

    #[test]
    fn set_pipo_defaults_uses_unit_fanout_defaults_for_unmapped_model() {
        let mut network = BufferDelayNetwork::new(WireLoadModel::Zero);
        let pi = network.add_node(BufferDelayNode::new("a", NodeKind::PrimaryInput));
        let po = network.add_node(BufferDelayNode::new("z", NodeKind::PrimaryOutput));
        let params = BufferDelayParams {
            model: DelayModel::UnitFanout,
            buffers: vec![inverter("dummy", 9.0, DelayTime::new(9.0, 9.0))],
            num_inv: 1,
            ..BufferDelayParams::new(DelayModel::UnitFanout)
        };

        set_pipo_defaults(&mut network, &params).unwrap();

        assert_eq!(network.node(po).unwrap().delay.output_load, Some(1.0));
        assert_eq!(
            network.node(pi).unwrap().delay.input_drive,
            Some(DelayTime::new(0.2, 0.2))
        );
    }

    #[test]
    fn missing_native_delay_data_reports_explicit_dependencies() {
        let node = BufferDelayNode::new("n", NodeKind::Internal).with_gate(0);
        assert_eq!(
            pin_load(&node, NodeId(7)),
            Err(BufDelayError::MissingPinDelay {
                node: NodeId(7),
                pin: 0,
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
        assert_eq!(
            buffer_pin_requirement(
                &node,
                NodeId(7),
                &BufferDelayParams::new(DelayModel::Mapped),
                0
            ),
            Err(BufDelayError::MissingWireRequiredTime {
                node: NodeId(7),
                pin: 0,
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
        assert_eq!(
            buffer_delay_from_sis_network(),
            Err(BufDelayError::MissingWireRequiredTime {
                node: NodeId(0),
                pin: 0,
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
    }
}
