//! Native Rust model for feasible behavior in `sis/speed/speed_delay.c`.
//!
//! The original C file mixes delay arithmetic with direct `network_t`,
//! `node_t`, delay-library, and mapped-library access. This module ports the
//! arrival/slack decision rules into an owned Rust graph model and leaves the
//! SIS-bound entry points as explicit dependency errors until those C files have
//! native ports.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;
pub const NEG_LARGE: f64 = -10_000.0;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

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

    pub const fn not_set() -> Self {
        Self {
            rise: NEG_LARGE,
            fall: NEG_LARGE,
        }
    }

    pub fn min_edge(self) -> f64 {
        self.rise.min(self.fall)
    }

    pub fn max_edge(self) -> f64 {
        self.rise.max(self.fall)
    }

    pub fn clamp_negative_to_zero(self) -> Self {
        Self {
            rise: self.rise.max(0.0),
            fall: self.fall.max(0.0),
        }
    }

    pub fn add_edges(self, rhs: DelayTime) -> DelayTime {
        DelayTime {
            rise: self.rise + rhs.rise,
            fall: self.fall + rhs.fall,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayPin {
    pub drive: DelayTime,
    pub block: DelayTime,
    pub load: f64,
}

impl DelayPin {
    pub const fn new(drive: DelayTime, block: DelayTime, load: f64) -> Self {
        Self { drive, block, load }
    }

    pub fn delay_for_load(self, load: f64) -> DelayTime {
        DelayTime {
            rise: self.drive.rise * load + self.block.rise,
            fall: self.drive.fall * load + self.block.fall,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    UnitFanout,
    Library,
    Mapped,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase {
    PositiveUnate,
    NegativeUnate,
    Binate,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WireLoadModel {
    Zero,
    Constant(f64),
    Linear { base: f64, slope: f64 },
}

impl WireLoadModel {
    pub fn load(self, fanouts: usize) -> f64 {
        match self {
            Self::Zero => 0.0,
            Self::Constant(load) => load,
            Self::Linear { base, slope } => base + slope * fanouts as f64,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedDelayParams {
    pub model: DelayModel,
    pub debug: bool,
    pub library_accl: bool,
    pub pin_cap: f64,
    pub nand_pin_delay: DelayPin,
    pub inv_pin_delay: DelayPin,
    pub wire_load: WireLoadModel,
}

impl SpeedDelayParams {
    pub fn new(model: DelayModel) -> Self {
        Self {
            model,
            debug: false,
            library_accl: false,
            pin_cap: 0.0,
            nand_pin_delay: DelayPin::new(DelayTime::zero(), DelayTime::zero(), 0.0),
            inv_pin_delay: DelayPin::new(DelayTime::zero(), DelayTime::zero(), 0.0),
            wire_load: WireLoadModel::Zero,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveFunction {
    Inverter,
    Nand2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimitiveGateDelay {
    pub function: PrimitiveFunction,
    pub area: f64,
    pub pin_delay: DelayPin,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpeedPrimitiveLibrary {
    gates: Vec<PrimitiveGateDelay>,
}

impl SpeedPrimitiveLibrary {
    pub fn new(gates: Vec<PrimitiveGateDelay>) -> Self {
        Self { gates }
    }

    pub fn smallest_pin_delay(&self, function: PrimitiveFunction) -> Option<DelayPin> {
        self.gates
            .iter()
            .filter(|gate| gate.function == function)
            .min_by(|left, right| left.area.total_cmp(&right.area))
            .map(|gate| gate.pin_delay)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaninEdge {
    pub node: NodeId,
    pub phase: InputPhase,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedDelayNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<FaninEdge>,
    pub fanout_count: usize,
    pub literal_count: usize,
    pub pin_delays: Vec<DelayTime>,
    pub arrival: Option<DelayTime>,
    pub slack: Option<DelayTime>,
    pub required_time: Option<DelayTime>,
}

impl SpeedDelayNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanout_count: 0,
            literal_count: 0,
            pin_delays: Vec::new(),
            arrival: None,
            slack: None,
            required_time: None,
        }
    }

    pub fn with_arrival(mut self, arrival: DelayTime) -> Self {
        self.arrival = Some(arrival);
        self
    }

    pub fn with_pin_delays(mut self, pin_delays: Vec<DelayTime>) -> Self {
        self.pin_delays = pin_delays;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpeedDelayNetwork {
    nodes: Vec<SpeedDelayNode>,
    primary_outputs: Vec<NodeId>,
    pub default_required_rise: Option<f64>,
    pub default_required_fall: Option<f64>,
}

impl SpeedDelayNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn nodes(&self) -> &[SpeedDelayNode] {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> Result<&SpeedDelayNode, SpeedDelayError> {
        self.nodes.get(id.0).ok_or(SpeedDelayError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut SpeedDelayNode, SpeedDelayError> {
        self.nodes
            .get_mut(id.0)
            .ok_or(SpeedDelayError::UnknownNode(id))
    }

    pub fn add_node(&mut self, node: SpeedDelayNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        if node.kind == NodeKind::PrimaryOutput {
            self.primary_outputs.push(id);
        }
        self.nodes.push(node);
        id
    }

    pub fn primary_outputs(&self) -> &[NodeId] {
        &self.primary_outputs
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpeedDelayError {
    UnknownNode(NodeId),
    MissingFanin { node: NodeId, pin: usize },
    MissingPinDelay { node: NodeId, pin: usize },
    MissingPrimitiveDelay(PrimitiveFunction),
    MissingSisPorts { operation: &'static str },
}

impl fmt::Display for SpeedDelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown speed-delay node {:?}", node),
            Self::MissingFanin { node, pin } => {
                write!(f, "node {:?} has no fanin at pin {pin}", node)
            }
            Self::MissingPinDelay { node, pin, .. } => write!(
                f,
                "node {:?} has no native pin delay for pin {pin}; SIS delay_node_pin is not ported",
                node
            ),
            Self::MissingPrimitiveDelay(function) => {
                write!(
                    f,
                    "mapped primitive delay for {:?} is not available",
                    function
                )
            }
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} is blocked by unported SIS dependencies")
            }
        }
    }
}

impl Error for SpeedDelayError {}

pub fn minimum_slack(network: &SpeedDelayNetwork) -> Option<(NodeId, f64)> {
    network
        .primary_outputs()
        .iter()
        .filter_map(|id| {
            network
                .node(*id)
                .ok()
                .and_then(|node| node.slack.map(|slack| (*id, slack.min_edge())))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
}

pub fn po_required_times_set(network: &SpeedDelayNetwork) -> bool {
    network.default_required_rise.is_some()
        || network.default_required_fall.is_some()
        || network.primary_outputs().iter().any(|id| {
            network
                .node(*id)
                .is_ok_and(|node| node.required_time.is_some())
        })
}

pub fn set_delay_data(
    params: &mut SpeedDelayParams,
    library: &SpeedPrimitiveLibrary,
    use_accl: bool,
) -> Result<(), SpeedDelayError> {
    params.library_accl = use_accl;
    if params.model != DelayModel::Mapped {
        return Ok(());
    }

    let inv = library
        .smallest_pin_delay(PrimitiveFunction::Inverter)
        .ok_or(SpeedDelayError::MissingPrimitiveDelay(
            PrimitiveFunction::Inverter,
        ))?;
    let nand = library.smallest_pin_delay(PrimitiveFunction::Nand2).ok_or(
        SpeedDelayError::MissingPrimitiveDelay(PrimitiveFunction::Nand2),
    )?;

    params.inv_pin_delay = inv;
    params.nand_pin_delay = nand;
    params.pin_cap = inv.load.max(nand.load);
    Ok(())
}

pub fn set_library_accl(params: &mut SpeedDelayParams, value: bool) {
    params.library_accl = value;
}

pub fn library_accl(params: &SpeedDelayParams) -> bool {
    params.library_accl
}

pub fn update_arrival_time(
    network: &mut SpeedDelayNetwork,
    node: NodeId,
    params: &SpeedDelayParams,
) -> Result<DelayTime, SpeedDelayError> {
    update_arrival_time_recur(network, node, params)
}

pub fn delay_trace(
    network: &mut SpeedDelayNetwork,
    params: &SpeedDelayParams,
) -> Result<(), SpeedDelayError> {
    for node in &mut network.nodes {
        if node.kind == NodeKind::Internal {
            reset_arrival_time(node);
        }
    }

    let outputs = network.primary_outputs.clone();
    for output in outputs {
        update_arrival_time(network, output, params)?;
    }
    Ok(())
}

pub fn delay_arrival_time(
    network: &SpeedDelayNetwork,
    node: NodeId,
    params: &SpeedDelayParams,
) -> Result<DelayTime, SpeedDelayError> {
    let node_data = network.node(node)?;
    match node_data.kind {
        NodeKind::PrimaryOutput => {
            let fanin = node_data
                .fanins
                .first()
                .ok_or(SpeedDelayError::MissingFanin { node, pin: 0 })?;
            delay_arrival_time(network, fanin.node, params)
        }
        NodeKind::PrimaryInput => {
            let pin_delay = delay_node_pin(network, node, 0, params)?;
            Ok(node_data
                .arrival
                .unwrap_or_else(DelayTime::zero)
                .clamp_negative_to_zero()
                .add_edges(pin_delay))
        }
        NodeKind::Internal => Ok(node_data
            .arrival
            .unwrap_or_else(DelayTime::zero)
            .clamp_negative_to_zero()),
    }
}

pub fn set_arrival_time(node: &mut SpeedDelayNode, time: DelayTime) {
    node.arrival = Some(time.clamp_negative_to_zero());
}

pub fn single_level_update(
    network: &mut SpeedDelayNetwork,
    node: NodeId,
    params: &SpeedDelayParams,
) -> Result<Option<DelayTime>, SpeedDelayError> {
    if network.node(node)?.kind != NodeKind::Internal {
        return Ok(None);
    }

    let node_data = network.node(node)?.clone();
    let mut delay = DelayTime::not_set();

    for (pin, fanin) in node_data.fanins.iter().enumerate() {
        let fanin_time = delay_arrival_time(network, fanin.node, params)?;
        let pin_delay = delay_node_pin(network, node, pin, params)?;
        merge_phase_arrival(&mut delay, fanin_time, pin_delay, fanin.phase);
    }

    if node_data.literal_count == 0 {
        delay = DelayTime::zero();
    }
    set_arrival_time(network.node_mut(node)?, delay);
    Ok(Some(delay))
}

pub fn reset_arrival_time(node: &mut SpeedDelayNode) {
    node.arrival = None;
}

pub fn update_fanout_from_sis_network() -> Result<(), SpeedDelayError> {
    Err(SpeedDelayError::MissingSisPorts {
        operation: "speed_update_fanout",
    })
}

pub fn delay_data_from_sis_library() -> Result<SpeedDelayParams, SpeedDelayError> {
    Err(SpeedDelayError::MissingSisPorts {
        operation: "speed_set_delay_data",
    })
}

pub fn delay_node_pin(
    network: &SpeedDelayNetwork,
    node: NodeId,
    pin: usize,
    params: &SpeedDelayParams,
) -> Result<DelayTime, SpeedDelayError> {
    let node_data = network.node(node)?;
    let fanin_count = node_data.fanins.len();

    if params.model == DelayModel::Mapped && params.library_accl && fanin_count < 3 {
        if fanin_count == 0 {
            return Ok(DelayTime::zero());
        }

        let pin_delay = if fanin_count == 2 {
            params.nand_pin_delay
        } else {
            params.inv_pin_delay
        };
        let load = params.wire_load.load(node_data.fanout_count)
            + node_data.fanout_count as f64 * params.pin_cap;
        return Ok(pin_delay.delay_for_load(load));
    }

    node_data
        .pin_delays
        .get(pin)
        .copied()
        .ok_or(SpeedDelayError::MissingPinDelay { node, pin })
}

fn update_arrival_time_recur(
    network: &mut SpeedDelayNetwork,
    node: NodeId,
    params: &SpeedDelayParams,
) -> Result<DelayTime, SpeedDelayError> {
    let node_data = network.node(node)?.clone();
    match node_data.kind {
        NodeKind::PrimaryInput => {
            let pin_delay = delay_node_pin(network, node, 0, params)?;
            Ok(node_data
                .arrival
                .unwrap_or_else(DelayTime::zero)
                .clamp_negative_to_zero()
                .add_edges(pin_delay))
        }
        NodeKind::PrimaryOutput => {
            let fanin = node_data
                .fanins
                .first()
                .ok_or(SpeedDelayError::MissingFanin { node, pin: 0 })?;
            update_arrival_time_recur(network, fanin.node, params)
        }
        NodeKind::Internal => {
            if let Some(arrival) = node_data.arrival {
                return Ok(arrival);
            }

            let mut delay = DelayTime::not_set();
            for (pin, fanin) in node_data.fanins.iter().enumerate() {
                let fanin_time = update_arrival_time_recur(network, fanin.node, params)?;
                let pin_delay = delay_node_pin(network, node, pin, params)?;
                merge_phase_arrival(&mut delay, fanin_time, pin_delay, fanin.phase);
            }

            if node_data.literal_count == 0 {
                delay = DelayTime::zero();
            }
            set_arrival_time(network.node_mut(node)?, delay);
            Ok(delay)
        }
    }
}

fn merge_phase_arrival(
    delay: &mut DelayTime,
    fanin_time: DelayTime,
    pin_delay: DelayTime,
    phase: InputPhase,
) {
    match phase {
        InputPhase::PositiveUnate => {
            delay.rise = delay.rise.max(fanin_time.rise + pin_delay.rise);
            delay.fall = delay.fall.max(fanin_time.fall + pin_delay.fall);
        }
        InputPhase::NegativeUnate => {
            delay.rise = delay.rise.max(fanin_time.fall + pin_delay.rise);
            delay.fall = delay.fall.max(fanin_time.rise + pin_delay.fall);
        }
        InputPhase::Binate => {
            delay.rise = delay.rise.max(fanin_time.rise + pin_delay.rise);
            delay.rise = delay.rise.max(fanin_time.fall + pin_delay.rise);
            delay.fall = delay.fall.max(fanin_time.rise + pin_delay.fall);
            delay.fall = delay.fall.max(fanin_time.fall + pin_delay.fall);
        }
        InputPhase::Unknown => {}
    }
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

    fn params() -> SpeedDelayParams {
        SpeedDelayParams::new(DelayModel::Unit)
    }

    fn pi(name: &str, rise: f64, fall: f64) -> SpeedDelayNode {
        SpeedDelayNode::new(name, NodeKind::PrimaryInput)
            .with_arrival(DelayTime::new(rise, fall))
            .with_pin_delays(vec![DelayTime::zero()])
    }

    fn internal(name: &str, fanins: Vec<FaninEdge>, pin_delays: Vec<DelayTime>) -> SpeedDelayNode {
        let mut node = SpeedDelayNode::new(name, NodeKind::Internal).with_pin_delays(pin_delays);
        node.fanins = fanins;
        node.literal_count = 1;
        node
    }

    #[test]
    fn minimum_slack_chooses_po_with_smallest_edge_slack() {
        let mut network = SpeedDelayNetwork::new();
        let mut slow = SpeedDelayNode::new("slow", NodeKind::PrimaryOutput);
        slow.slack = Some(DelayTime::new(2.0, 0.5));
        let mut fast = SpeedDelayNode::new("fast", NodeKind::PrimaryOutput);
        fast.slack = Some(DelayTime::new(-1.0, 3.0));
        let slow_id = network.add_node(slow);
        let fast_id = network.add_node(fast);

        assert_eq!(minimum_slack(&network), Some((fast_id, -1.0)));
        assert_ne!(minimum_slack(&network), Some((slow_id, 0.5)));
    }

    #[test]
    fn po_required_times_set_checks_defaults_and_primary_outputs() {
        let mut network = SpeedDelayNetwork::new();
        network.add_node(SpeedDelayNode::new("out", NodeKind::PrimaryOutput));
        assert!(!po_required_times_set(&network));

        network.default_required_fall = Some(12.0);
        assert!(po_required_times_set(&network));

        network.default_required_fall = None;
        network.node_mut(NodeId(0)).unwrap().required_time = Some(DelayTime::new(4.0, 5.0));
        assert!(po_required_times_set(&network));
    }

    #[test]
    fn pi_arrival_defaults_negative_edges_to_zero_then_adds_pin_delay() {
        let mut network = SpeedDelayNetwork::new();
        let input = SpeedDelayNode::new("a", NodeKind::PrimaryInput)
            .with_arrival(DelayTime::new(-2.0, 3.0))
            .with_pin_delays(vec![DelayTime::new(1.5, 2.5)]);
        let id = network.add_node(input);

        assert_eq!(
            delay_arrival_time(&network, id, &params()).unwrap(),
            DelayTime::new(1.5, 5.5)
        );
    }

    #[test]
    fn recursive_update_applies_positive_negative_and_binate_phase_rules() {
        let mut network = SpeedDelayNetwork::new();
        let a = network.add_node(pi("a", 10.0, 20.0));
        let b = network.add_node(pi("b", 7.0, 4.0));
        let c = network.add_node(pi("c", 1.0, 30.0));
        let n = network.add_node(internal(
            "n",
            vec![
                FaninEdge {
                    node: a,
                    phase: InputPhase::PositiveUnate,
                },
                FaninEdge {
                    node: b,
                    phase: InputPhase::NegativeUnate,
                },
                FaninEdge {
                    node: c,
                    phase: InputPhase::Binate,
                },
            ],
            vec![
                DelayTime::new(1.0, 2.0),
                DelayTime::new(3.0, 5.0),
                DelayTime::new(2.0, 4.0),
            ],
        ));

        assert_eq!(
            update_arrival_time(&mut network, n, &params()).unwrap(),
            DelayTime::new(32.0, 34.0)
        );
        assert_eq!(
            network.node(n).unwrap().arrival,
            Some(DelayTime::new(32.0, 34.0))
        );
    }

    #[test]
    fn constant_internal_node_arrival_is_zero() {
        let mut network = SpeedDelayNetwork::new();
        let node = network.add_node(SpeedDelayNode::new("const", NodeKind::Internal));

        assert_eq!(
            update_arrival_time(&mut network, node, &params()).unwrap(),
            DelayTime::zero()
        );
    }

    #[test]
    fn delay_trace_resets_stale_internal_arrival_and_updates_primary_outputs() {
        let mut network = SpeedDelayNetwork::new();
        let a = network.add_node(pi("a", 1.0, 2.0));
        let mut n = internal(
            "n",
            vec![FaninEdge {
                node: a,
                phase: InputPhase::PositiveUnate,
            }],
            vec![DelayTime::new(3.0, 4.0)],
        );
        n.arrival = Some(DelayTime::new(99.0, 99.0));
        let n = network.add_node(n);
        let mut out = SpeedDelayNode::new("out", NodeKind::PrimaryOutput);
        out.fanins.push(FaninEdge {
            node: n,
            phase: InputPhase::PositiveUnate,
        });
        network.add_node(out);

        delay_trace(&mut network, &params()).unwrap();

        assert_eq!(
            network.node(n).unwrap().arrival,
            Some(DelayTime::new(4.0, 6.0))
        );
    }

    #[test]
    fn single_level_update_uses_existing_fanin_arrivals_without_recursing() {
        let mut network = SpeedDelayNetwork::new();
        let a = network.add_node(pi("a", 5.0, 6.0));
        let n = network.add_node(internal(
            "n",
            vec![FaninEdge {
                node: a,
                phase: InputPhase::PositiveUnate,
            }],
            vec![DelayTime::new(1.0, 2.0)],
        ));

        assert_eq!(
            single_level_update(&mut network, n, &params()).unwrap(),
            Some(DelayTime::new(6.0, 8.0))
        );
    }

    #[test]
    fn set_delay_data_selects_lowest_area_primitives_and_pin_cap() {
        let mut params = SpeedDelayParams::new(DelayModel::Mapped);
        let library = SpeedPrimitiveLibrary::new(vec![
            PrimitiveGateDelay {
                function: PrimitiveFunction::Inverter,
                area: 3.0,
                pin_delay: DelayPin::new(DelayTime::new(9.0, 9.0), DelayTime::zero(), 0.9),
            },
            PrimitiveGateDelay {
                function: PrimitiveFunction::Inverter,
                area: 1.0,
                pin_delay: DelayPin::new(DelayTime::new(1.0, 2.0), DelayTime::zero(), 0.2),
            },
            PrimitiveGateDelay {
                function: PrimitiveFunction::Nand2,
                area: 2.0,
                pin_delay: DelayPin::new(DelayTime::new(3.0, 4.0), DelayTime::zero(), 0.5),
            },
        ]);

        set_delay_data(&mut params, &library, true).unwrap();

        assert!(library_accl(&params));
        assert_eq!(params.inv_pin_delay.drive, DelayTime::new(1.0, 2.0));
        assert_eq!(params.nand_pin_delay.drive, DelayTime::new(3.0, 4.0));
        assert_eq!(params.pin_cap, 0.5);
    }

    #[test]
    fn mapped_accelerator_uses_inv_or_nand_delay_with_wire_and_pin_load() {
        let mut params = SpeedDelayParams::new(DelayModel::Mapped);
        params.library_accl = true;
        params.pin_cap = 0.5;
        params.wire_load = WireLoadModel::Constant(0.4);
        params.inv_pin_delay =
            DelayPin::new(DelayTime::new(2.0, 3.0), DelayTime::new(1.0, 1.5), 0.5);

        let mut network = SpeedDelayNetwork::new();
        let a = network.add_node(pi("a", 0.0, 0.0));
        let mut inverter = internal(
            "inv",
            vec![FaninEdge {
                node: a,
                phase: InputPhase::NegativeUnate,
            }],
            Vec::new(),
        );
        inverter.fanout_count = 3;
        let inv = network.add_node(inverter);

        let delay = delay_node_pin(&network, inv, 0, &params).unwrap();
        assert_close(delay.rise, 4.8);
        assert_close(delay.fall, 7.2);
    }

    #[test]
    fn missing_native_pin_delay_reports_dependency_beads() {
        let mut network = SpeedDelayNetwork::new();
        let node = network.add_node(SpeedDelayNode::new("n", NodeKind::Internal));

        assert_eq!(
            delay_node_pin(&network, node, 0, &params()),
            Err(SpeedDelayError::MissingPinDelay { node, pin: 0 })
        );
    }

    #[test]
    fn sis_bound_entry_points_report_blocking_ports() {
        assert_eq!(
            update_fanout_from_sis_network(),
            Err(SpeedDelayError::MissingSisPorts {
                operation: "speed_update_fanout",
            })
        );
        assert_eq!(
            delay_data_from_sis_library(),
            Err(SpeedDelayError::MissingSisPorts {
                operation: "speed_set_delay_data",
            })
        );
    }
}
