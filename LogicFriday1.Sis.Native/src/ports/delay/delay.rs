//! Native Rust delay-trace model for `sis/delay/delay.c`.
//!
//! This file ports the reusable timing behavior from the SIS delay package into
//! an owned graph model: pin delay arithmetic, wire-load tables, fanout load
//! calculation, arrival and required time propagation, slacks, and terminal
//! timing constraints. SIS-bound library mapping and timing-driven cofactoring
//! integration are represented as typed errors until the surrounding native
//! graph and mapper ports provide those inputs directly.

use std::error::Error;
use std::fmt;

pub const DELAY_NOT_SET: f64 = -1_234_567.0;
pub const DELAY_VALUE_NOT_GIVEN: f64 = DELAY_NOT_SET;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime
{
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime
{
    pub const fn new(rise: f64, fall: f64) -> Self
    {
        Self { rise, fall }
    }

    pub const fn zero() -> Self
    {
        Self
        {
            rise: 0.0,
            fall: 0.0,
        }
    }

    pub const fn not_given() -> Self
    {
        Self
        {
            rise: DELAY_VALUE_NOT_GIVEN,
            fall: DELAY_VALUE_NOT_GIVEN,
        }
    }

    pub fn both_given(self) -> bool
    {
        self.rise != DELAY_VALUE_NOT_GIVEN && self.fall != DELAY_VALUE_NOT_GIVEN
    }

}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinPhase
{
    NotGiven,
    Inverting,
    NonInverting,
    Neither,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase
{
    PositiveUnate,
    NegativeUnate,
    Binate,
}

impl From<InputPhase> for PinPhase
{
    fn from(value: InputPhase) -> Self
    {
        match value
        {
            InputPhase::PositiveUnate => Self::NonInverting,
            InputPhase::NegativeUnate => Self::Inverting,
            InputPhase::Binate => Self::Neither,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel
{
    Unit,
    Library,
    UnitFanout,
    Mapped,
    Unknown,
    Tdc,
}

impl DelayModel
{
    pub fn from_name(name: &str) -> Self
    {
        match name
        {
            "unit" => Self::Unit,
            "unit-fanout" => Self::UnitFanout,
            "library" => Self::Library,
            "mapped" => Self::Mapped,
            "tdc" => Self::Tdc,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayPin
{
    pub block: DelayTime,
    pub drive: DelayTime,
    pub phase: PinPhase,
    pub load: f64,
    pub max_load: f64,
    pub user_time: DelayTime,
}

impl DelayPin
{
    pub const fn new(
        block: DelayTime,
        drive: DelayTime,
        phase: PinPhase,
        load: f64,
        max_load: f64,
        user_time: DelayTime,
    ) -> Self
    {
        Self
        {
            block,
            drive,
            phase,
            load,
            max_load,
            user_time,
        }
    }

    pub const fn unit() -> Self
    {
        Self::new(
            DelayTime::new(1.0, 1.0),
            DelayTime::zero(),
            PinPhase::NonInverting,
            1.0,
            f64::INFINITY,
            DelayTime::not_given(),
        )
    }

    pub const fn unit_fanout() -> Self
    {
        Self::new(
            DelayTime::new(1.0, 1.0),
            DelayTime::new(0.2, 0.2),
            PinPhase::NonInverting,
            1.0,
            f64::INFINITY,
            DelayTime::not_given(),
        )
    }

    pub const fn unspecified_pipo() -> Self
    {
        Self::new(
            DelayTime::not_given(),
            DelayTime::not_given(),
            PinPhase::NonInverting,
            DELAY_VALUE_NOT_GIVEN,
            DELAY_VALUE_NOT_GIVEN,
            DelayTime::not_given(),
        )
    }

    pub const fn backup_pipo() -> Self
    {
        Self::new(
            DelayTime::zero(),
            DelayTime::zero(),
            PinPhase::NonInverting,
            0.0,
            f64::INFINITY,
            DelayTime::zero(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind
{
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fanin
{
    pub node: NodeId,
    pub phase: InputPhase,
    pub pin_delay: Option<DelayPin>,
}

impl Fanin
{
    pub const fn new(node: NodeId, phase: InputPhase, pin_delay: Option<DelayPin>) -> Self
    {
        Self
        {
            node,
            phase,
            pin_delay,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayNode
{
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<Fanin>,
    pub fanouts: Vec<(NodeId, usize)>,
    pub terminal_pin: Option<DelayPin>,
    pub arrival: DelayTime,
    pub required: DelayTime,
    pub slack: DelayTime,
    pub load: f64,
}

impl DelayNode
{
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self
    {
        Self
        {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            terminal_pin: None,
            arrival: DelayTime::zero(),
            required: DelayTime::zero(),
            slack: DelayTime::zero(),
            load: 0.0,
        }
    }

    pub fn with_terminal_pin(mut self, pin: DelayPin) -> Self
    {
        self.terminal_pin = Some(pin);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WireLoadTable
{
    pub slope: f64,
    pub pins: Vec<f64>,
}

impl Default for WireLoadTable
{
    fn default() -> Self
    {
        Self
        {
            slope: DELAY_VALUE_NOT_GIVEN,
            pins: Vec::new(),
        }
    }
}

impl WireLoadTable
{
    pub fn compute(&self, pins: usize) -> f64
    {
        if pins == 0
        {
            return 0.0;
        }

        let n = self.pins.len();
        if pins <= n
        {
            return self.pins[pins - 1];
        }
        if self.slope > 0.0
        {
            return self.slope * pins as f64;
        }
        if n == 0
        {
            return 0.0;
        }
        if n == 1
        {
            return self.pins[0];
        }

        let extra = (pins - n) as f64;
        let last = self.pins[n - 1];
        let previous = self.pins[n - 2];
        last * (1.0 + extra) - extra * previous
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayParameter
{
    BlockRise,
    BlockFall,
    DriveRise,
    DriveFall,
    Phase,
    OutputLoad,
    InputLoad,
    MaxInputLoad,
    ArrivalRise,
    ArrivalFall,
    RequiredRise,
    RequiredFall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayDefaultParameter
{
    AddWireLoad,
    WireLoadSlope,
    DefaultDriveRise,
    DefaultDriveFall,
    DefaultOutputLoad,
    DefaultArrivalRise,
    DefaultArrivalFall,
    DefaultRequiredRise,
    DefaultRequiredFall,
    DefaultMaxInputLoad,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayNetwork
{
    nodes: Vec<DelayNode>,
    primary_inputs: Vec<NodeId>,
    primary_outputs: Vec<NodeId>,
    pub default_arrival: DelayTime,
    pub default_required: DelayTime,
    pub wire_load_table: WireLoadTable,
    pub pipo_model: DelayPin,
}

impl Default for DelayNetwork
{
    fn default() -> Self
    {
        Self
        {
            nodes: Vec::new(),
            primary_inputs: Vec::new(),
            primary_outputs: Vec::new(),
            default_arrival: DelayTime::not_given(),
            default_required: DelayTime::not_given(),
            wire_load_table: WireLoadTable::default(),
            pipo_model: DelayPin::unspecified_pipo(),
        }
    }
}

impl DelayNetwork
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn add_node(&mut self, node: DelayNode) -> NodeId
    {
        let id = NodeId(self.nodes.len());
        match node.kind
        {
            NodeKind::PrimaryInput => self.primary_inputs.push(id),
            NodeKind::PrimaryOutput => self.primary_outputs.push(id),
            NodeKind::Internal => {}
        }
        self.nodes.push(node);
        id
    }

    pub fn add_edge(
        &mut self,
        source: NodeId,
        sink: NodeId,
        phase: InputPhase,
        pin_delay: Option<DelayPin>,
    ) -> Result<(), DelayError>
    {
        self.require_node(source)?;
        self.require_node(sink)?;

        let pin = self.nodes[sink.0].fanins.len();
        self.nodes[sink.0]
            .fanins
            .push(Fanin::new(source, phase, pin_delay));
        self.nodes[source.0].fanouts.push((sink, pin));
        Ok(())
    }

    pub fn node(&self, node: NodeId) -> Result<&DelayNode, DelayError>
    {
        self.nodes.get(node.0).ok_or(DelayError::MissingNode(node))
    }

    pub fn node_mut(&mut self, node: NodeId) -> Result<&mut DelayNode, DelayError>
    {
        self.nodes.get_mut(node.0).ok_or(DelayError::MissingNode(node))
    }

    pub fn nodes(&self) -> &[DelayNode]
    {
        &self.nodes
    }

    pub fn delay_trace(&mut self, model: DelayModel) -> Result<(), DelayError>
    {
        let order = self.topological_order()?;

        for node in &order
        {
            let load = if model == DelayModel::Tdc
            {
                0.0
            }
            else
            {
                self.compute_fanout_load(*node, model)?
            };
            self.nodes[node.0].load = load;
        }

        for node in &order
        {
            self.set_arrival_time(*node, model)?;
        }

        let (_, latest) = self.latest_output()?;
        for node in order.iter().rev()
        {
            self.set_required_time(*node, latest, model)?;
        }

        for node in &mut self.nodes
        {
            node.slack = DelayTime::new(
                node.required.rise - node.arrival.rise,
                node.required.fall - node.arrival.fall,
            );
        }

        Ok(())
    }

    pub fn compute_arrival_time(
        &mut self,
        node: NodeId,
        model: DelayModel,
        load: f64,
    ) -> Result<DelayTime, DelayError>
    {
        self.require_node(node)?;
        self.nodes[node.0].load = load;
        self.set_arrival_time(node, model)?;
        Ok(self.nodes[node.0].arrival)
    }

    pub fn compute_fanout_load(
        &self,
        node: NodeId,
        model: DelayModel,
    ) -> Result<f64, DelayError>
    {
        self.require_node(node)?;
        if model == DelayModel::Tdc
        {
            return Ok(0.0);
        }

        let mut load = self.wire_load_table.compute(self.nodes[node.0].fanouts.len());
        for (fanout, pin) in &self.nodes[node.0].fanouts
        {
            let fanout_node = self.node(*fanout)?;
            if fanout_node.kind == NodeKind::PrimaryOutput
            {
                load += self.primary_output_load(*fanout);
            }
            else
            {
                load += self.get_pin_delay(*fanout, *pin, model)?.load;
            }
        }

        Ok(load)
    }

    pub fn node_pin_delay(
        &mut self,
        node: NodeId,
        pin: usize,
        model: DelayModel,
    ) -> Result<DelayTime, DelayError>
    {
        self.require_node(node)?;
        if self.nodes[node.0].kind == NodeKind::PrimaryOutput
        {
            return Ok(DelayTime::zero());
        }

        let load = self.compute_fanout_load(node, model)?;
        self.nodes[node.0].load = load;
        let pin_delay = self.get_pin_delay(node, pin, model)?;
        self.compute_delay(node, pin_delay, model)
    }

    pub fn wire_required_time(
        &self,
        node: NodeId,
        fanin_index: usize,
        model: DelayModel,
    ) -> Result<DelayTime, DelayError>
    {
        let node_data = self.node(node)?;
        if node_data.kind == NodeKind::PrimaryOutput
        {
            return Ok(node_data.required);
        }
        if node_data.kind != NodeKind::PrimaryInput && fanin_index >= node_data.fanins.len()
        {
            return Err(DelayError::MissingFanin
            {
                node,
                pin: fanin_index,
            });
        }

        let pin_delay = self.get_pin_delay(node, fanin_index, model)?;
        let delay = self.compute_delay(node, pin_delay, model)?;
        let phase = if node_data.kind == NodeKind::PrimaryInput
        {
            PinPhase::NonInverting
        }
        else
        {
            pin_delay.phase
        };
        required_at_input(node_data.required, delay, phase)
    }

    pub fn wire_slack_time(
        &self,
        node: NodeId,
        fanin_index: usize,
        model: DelayModel,
    ) -> Result<DelayTime, DelayError>
    {
        let node_data = self.node(node)?;
        let arrival = if node_data.kind == NodeKind::PrimaryInput
        {
            node_data.arrival
        }
        else
        {
            let fanin = node_data
                .fanins
                .get(fanin_index)
                .ok_or(DelayError::MissingFanin
                {
                    node,
                    pin: fanin_index,
                })?
                .node;
            self.node(fanin)?.arrival
        };
        let required = self.wire_required_time(node, fanin_index, model)?;

        Ok(DelayTime::new(
            required.rise - arrival.rise,
            required.fall - arrival.fall,
        ))
    }

    pub fn latest_output(&self) -> Result<(Option<NodeId>, f64), DelayError>
    {
        let mut latest = f64::NEG_INFINITY;
        let mut last_output = None;

        for output in &self.primary_outputs
        {
            let arrival = self.node(*output)?.arrival;
            if arrival.rise > latest
            {
                latest = arrival.rise;
                last_output = Some(*output);
            }
            if arrival.fall > latest
            {
                latest = arrival.fall;
                last_output = Some(*output);
            }
        }

        Ok((last_output, latest))
    }

    pub fn get_pin_delay(
        &self,
        node: NodeId,
        pin: usize,
        model: DelayModel,
    ) -> Result<DelayPin, DelayError>
    {
        let node_data = self.node(node)?;
        if matches!(node_data.kind, NodeKind::PrimaryInput | NodeKind::PrimaryOutput)
        {
            return self.io_delay(node, model);
        }

        match model
        {
            DelayModel::Unit =>
            {
                let fanin = node_data.fanins.get(pin).ok_or(DelayError::MissingFanin
                {
                    node,
                    pin,
                })?;
                let mut delay = DelayPin::unit();
                delay.phase = fanin.phase.into();
                Ok(delay)
            }
            DelayModel::UnitFanout =>
            {
                let fanin = node_data.fanins.get(pin).ok_or(DelayError::MissingFanin
                {
                    node,
                    pin,
                })?;
                let mut delay = DelayPin::unit_fanout();
                delay.phase = fanin.phase.into();
                Ok(delay)
            }
            DelayModel::Library | DelayModel::Mapped | DelayModel::Tdc =>
            {
                node_data
                    .fanins
                    .get(pin)
                    .ok_or(DelayError::MissingFanin
                    {
                        node,
                        pin,
                    })?
                    .pin_delay
                    .ok_or(DelayError::MissingNativePinDelay
                    {
                        node,
                        pin,
                        model,
                    })
            }
            DelayModel::Unknown => Err(DelayError::BadModel(model)),
        }
    }

    pub fn get_parameter(
        &self,
        node: NodeId,
        parameter: DelayParameter,
    ) -> Result<f64, DelayError>
    {
        let pin = self
            .node(node)?
            .terminal_pin
            .unwrap_or_else(DelayPin::unspecified_pipo);

        Ok(match parameter
        {
            DelayParameter::BlockRise => pin.block.rise,
            DelayParameter::BlockFall => pin.block.fall,
            DelayParameter::DriveRise => pin.drive.rise,
            DelayParameter::DriveFall => pin.drive.fall,
            DelayParameter::OutputLoad | DelayParameter::InputLoad => pin.load,
            DelayParameter::MaxInputLoad => pin.max_load,
            DelayParameter::Phase => match pin.phase
            {
                PinPhase::Inverting => 0.0,
                PinPhase::NonInverting => 1.0,
                PinPhase::Neither => 2.0,
                PinPhase::NotGiven => return Err(DelayError::BadPinPhase),
            },
            DelayParameter::ArrivalRise | DelayParameter::RequiredRise => pin.user_time.rise,
            DelayParameter::ArrivalFall | DelayParameter::RequiredFall => pin.user_time.fall,
        })
    }

    pub fn set_parameter(
        &mut self,
        node: NodeId,
        parameter: DelayParameter,
        value: f64,
    ) -> Result<(), DelayError>
    {
        self.require_node(node)?;
        let mut pin = self.nodes[node.0]
            .terminal_pin
            .unwrap_or_else(DelayPin::unspecified_pipo);

        match parameter
        {
            DelayParameter::BlockRise => pin.block.rise = value,
            DelayParameter::BlockFall => pin.block.fall = value,
            DelayParameter::DriveRise => pin.drive.rise = value,
            DelayParameter::DriveFall => pin.drive.fall = value,
            DelayParameter::OutputLoad | DelayParameter::InputLoad => pin.load = value,
            DelayParameter::MaxInputLoad => pin.max_load = value,
            DelayParameter::Phase =>
            {
                pin.phase = match value
                {
                    0.0 => PinPhase::Inverting,
                    1.0 => PinPhase::NonInverting,
                    2.0 => PinPhase::Neither,
                    _ => return Err(DelayError::BadPinPhase),
                };
            }
            DelayParameter::ArrivalRise | DelayParameter::RequiredRise =>
            {
                pin.user_time.rise = value;
            }
            DelayParameter::ArrivalFall | DelayParameter::RequiredFall =>
            {
                pin.user_time.fall = value;
            }
        }

        self.nodes[node.0].terminal_pin = Some(pin);
        Ok(())
    }

    pub fn set_default_parameter(&mut self, parameter: DelayDefaultParameter, value: f64)
    {
        match parameter
        {
            DelayDefaultParameter::AddWireLoad =>
            {
                if value < 0.0
                {
                    self.wire_load_table.pins.clear();
                }
                else
                {
                    self.wire_load_table.pins.push(value);
                }
            }
            DelayDefaultParameter::WireLoadSlope => self.wire_load_table.slope = value,
            DelayDefaultParameter::DefaultDriveRise => self.pipo_model.drive.rise = value,
            DelayDefaultParameter::DefaultDriveFall => self.pipo_model.drive.fall = value,
            DelayDefaultParameter::DefaultOutputLoad => self.pipo_model.load = value,
            DelayDefaultParameter::DefaultArrivalRise => self.default_arrival.rise = value,
            DelayDefaultParameter::DefaultArrivalFall => self.default_arrival.fall = value,
            DelayDefaultParameter::DefaultRequiredRise => self.default_required.rise = value,
            DelayDefaultParameter::DefaultRequiredFall => self.default_required.fall = value,
            DelayDefaultParameter::DefaultMaxInputLoad => self.pipo_model.max_load = value,
        }
    }

    pub fn get_default_parameter(&self, parameter: DelayDefaultParameter) -> Option<f64>
    {
        match parameter
        {
            DelayDefaultParameter::AddWireLoad => None,
            DelayDefaultParameter::WireLoadSlope => Some(self.wire_load_table.slope),
            DelayDefaultParameter::DefaultDriveRise => Some(self.pipo_model.drive.rise),
            DelayDefaultParameter::DefaultDriveFall => Some(self.pipo_model.drive.fall),
            DelayDefaultParameter::DefaultOutputLoad => Some(self.pipo_model.load),
            DelayDefaultParameter::DefaultArrivalRise => Some(self.default_arrival.rise),
            DelayDefaultParameter::DefaultArrivalFall => Some(self.default_arrival.fall),
            DelayDefaultParameter::DefaultRequiredRise => Some(self.default_required.rise),
            DelayDefaultParameter::DefaultRequiredFall => Some(self.default_required.fall),
            DelayDefaultParameter::DefaultMaxInputLoad => Some(self.pipo_model.max_load),
        }
    }

    pub fn get_po_load(&self, node: NodeId) -> Result<Option<f64>, DelayError>
    {
        if self.node(node)?.kind != NodeKind::PrimaryOutput
        {
            return Ok(None);
        }

        let load = self.get_parameter(node, DelayParameter::OutputLoad)?;
        Ok((load != DELAY_VALUE_NOT_GIVEN).then_some(load))
    }

    pub fn get_pi_drive(&self, node: NodeId) -> Result<Option<DelayTime>, DelayError>
    {
        if self.node(node)?.kind != NodeKind::PrimaryInput
        {
            return Ok(None);
        }

        let drive = DelayTime::new(
            self.get_parameter(node, DelayParameter::DriveRise)?,
            self.get_parameter(node, DelayParameter::DriveFall)?,
        );
        Ok(drive.both_given().then_some(drive))
    }

    pub fn get_pi_load_limit(&self, node: NodeId) -> Result<Option<f64>, DelayError>
    {
        if self.node(node)?.kind != NodeKind::PrimaryInput
        {
            return Ok(None);
        }

        let load_limit = self.get_parameter(node, DelayParameter::MaxInputLoad)?;
        Ok((load_limit != DELAY_VALUE_NOT_GIVEN).then_some(load_limit))
    }

    pub fn get_pi_arrival_time(&self, node: NodeId) -> Result<Option<DelayTime>, DelayError>
    {
        if self.node(node)?.kind != NodeKind::PrimaryInput
        {
            return Ok(None);
        }

        let arrival = DelayTime::new(
            self.get_parameter(node, DelayParameter::ArrivalRise)?,
            self.get_parameter(node, DelayParameter::ArrivalFall)?,
        );
        Ok(arrival.both_given().then_some(arrival))
    }

    pub fn get_po_required_time(&self, node: NodeId) -> Result<Option<DelayTime>, DelayError>
    {
        if self.node(node)?.kind != NodeKind::PrimaryOutput
        {
            return Ok(None);
        }

        let required = DelayTime::new(
            self.get_parameter(node, DelayParameter::RequiredRise)?,
            self.get_parameter(node, DelayParameter::RequiredFall)?,
        );
        Ok(required.both_given().then_some(required))
    }

    fn set_arrival_time(&mut self, node: NodeId, model: DelayModel) -> Result<(), DelayError>
    {
        let node_data = self.node(node)?.clone();
        match node_data.kind
        {
            NodeKind::PrimaryInput =>
            {
                let mut arrival = self.primary_input_arrival_time(node);
                let pin_delay = self.get_pin_delay(node, 0, model)?;
                let delay = self.compute_delay(node, pin_delay, model)?;
                arrival.rise += delay.rise;
                arrival.fall += delay.fall;
                self.nodes[node.0].arrival = arrival;
            }
            NodeKind::PrimaryOutput =>
            {
                let fanin = node_data.fanins.first().ok_or(DelayError::MissingFanin
                {
                    node,
                    pin: 0,
                })?;
                self.nodes[node.0].arrival = self.node(fanin.node)?.arrival;
            }
            NodeKind::Internal =>
            {
                let mut arrival = DelayTime::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
                for (pin, fanin) in node_data.fanins.iter().enumerate()
                {
                    let pin_delay = self.get_pin_delay(node, pin, model)?;
                    let delay = self.compute_delay(node, pin_delay, model)?;
                    let fanin_arrival = self.node(fanin.node)?.arrival;
                    merge_arrival(&mut arrival, fanin_arrival, delay, pin_delay.phase)?;
                }
                self.nodes[node.0].arrival = arrival;
            }
        }

        Ok(())
    }

    fn set_required_time(
        &mut self,
        node: NodeId,
        latest: f64,
        model: DelayModel,
    ) -> Result<(), DelayError>
    {
        let node_data = self.node(node)?.clone();
        if node_data.kind == NodeKind::PrimaryOutput
        {
            self.nodes[node.0].required = self.primary_output_required_time(node, latest);
            return Ok(());
        }

        let mut required = DelayTime::new(f64::INFINITY, f64::INFINITY);
        for (fanout, pin) in node_data.fanouts
        {
            let fanout_data = self.node(fanout)?;
            let fanout_required = fanout_data.required;
            if fanout_data.kind == NodeKind::PrimaryOutput
            {
                required.rise = required.rise.min(fanout_required.rise);
                required.fall = required.fall.min(fanout_required.fall);
                continue;
            }

            let pin_delay = self.get_pin_delay(fanout, pin, model)?;
            let delay = self.compute_delay(fanout, pin_delay, model)?;
            let input_required = required_at_input(fanout_required, delay, pin_delay.phase)?;
            required.rise = required.rise.min(input_required.rise);
            required.fall = required.fall.min(input_required.fall);
        }

        self.nodes[node.0].required = required;
        Ok(())
    }

    fn compute_delay(
        &self,
        node: NodeId,
        pin_delay: DelayPin,
        model: DelayModel,
    ) -> Result<DelayTime, DelayError>
    {
        if model == DelayModel::Unknown
        {
            return Err(DelayError::BadModel(model));
        }

        let node_data = self.node(node)?;
        let mut delay = DelayTime::new(
            pin_delay.drive.rise * node_data.load,
            pin_delay.drive.fall * node_data.load,
        );
        if node_data.kind != NodeKind::PrimaryInput
        {
            delay.rise += pin_delay.block.rise;
            delay.fall += pin_delay.block.fall;
        }

        Ok(delay)
    }

    fn io_delay(&self, node: NodeId, model: DelayModel) -> Result<DelayPin, DelayError>
    {
        match model
        {
            DelayModel::Unit => Ok(DelayPin::unit()),
            DelayModel::UnitFanout =>
            {
                let mut delay = DelayPin::unit_fanout();
                if let Some(drive) = self.get_pi_drive(node)?
                {
                    delay.drive = drive;
                }
                else
                {
                    if self.pipo_model.drive.rise != DELAY_VALUE_NOT_GIVEN
                    {
                        delay.drive.rise = self.pipo_model.drive.rise;
                    }
                    if self.pipo_model.drive.fall != DELAY_VALUE_NOT_GIVEN
                    {
                        delay.drive.fall = self.pipo_model.drive.fall;
                    }
                }
                Ok(delay)
            }
            DelayModel::Library | DelayModel::Mapped | DelayModel::Tdc =>
            {
                let pin = self
                    .node(node)?
                    .terminal_pin
                    .unwrap_or_else(DelayPin::unspecified_pipo);
                Ok(self.apply_pipo_defaults(pin))
            }
            DelayModel::Unknown => Err(DelayError::BadModel(model)),
        }
    }

    fn apply_pipo_defaults(&self, mut pin: DelayPin) -> DelayPin
    {
        let defaults = self.pipo_model;
        let backup = DelayPin::backup_pipo();

        if pin.block.rise == DELAY_VALUE_NOT_GIVEN
        {
            pin.block.rise = default_or_backup(defaults.block.rise, backup.block.rise);
        }
        if pin.block.fall == DELAY_VALUE_NOT_GIVEN
        {
            pin.block.fall = default_or_backup(defaults.block.fall, backup.block.fall);
        }
        if pin.drive.rise == DELAY_VALUE_NOT_GIVEN
        {
            pin.drive.rise = default_or_backup(defaults.drive.rise, backup.drive.rise);
        }
        if pin.drive.fall == DELAY_VALUE_NOT_GIVEN
        {
            pin.drive.fall = default_or_backup(defaults.drive.fall, backup.drive.fall);
        }
        if pin.load == DELAY_VALUE_NOT_GIVEN
        {
            pin.load = default_or_backup(defaults.load, backup.load);
        }
        if pin.max_load == DELAY_VALUE_NOT_GIVEN
        {
            pin.max_load = default_or_backup(defaults.max_load, backup.max_load);
        }
        if pin.phase == PinPhase::NotGiven
        {
            pin.phase = if defaults.phase == PinPhase::NotGiven
            {
                backup.phase
            }
            else
            {
                defaults.phase
            };
        }

        pin
    }

    fn primary_input_arrival_time(&self, node: NodeId) -> DelayTime
    {
        if let Some(pin_delay) = self.nodes[node.0].terminal_pin
        {
            if pin_delay.user_time.both_given()
            {
                return pin_delay.user_time;
            }
        }
        if self.default_arrival.both_given()
        {
            return self.default_arrival;
        }

        DelayTime::zero()
    }

    fn primary_output_required_time(&self, node: NodeId, latest: f64) -> DelayTime
    {
        if let Some(pin_delay) = self.nodes[node.0].terminal_pin
        {
            if pin_delay.user_time.both_given()
            {
                return pin_delay.user_time;
            }
        }
        if self.default_required.both_given()
        {
            return self.default_required;
        }

        DelayTime::new(latest, latest)
    }

    fn primary_output_load(&self, node: NodeId) -> f64
    {
        let mut load = if self.pipo_model.load == DELAY_NOT_SET
        {
            0.0
        }
        else
        {
            self.pipo_model.load
        };
        if let Some(pin) = self.nodes[node.0].terminal_pin
        {
            if pin.load != DELAY_VALUE_NOT_GIVEN
            {
                load = pin.load;
            }
        }

        load
    }

    fn topological_order(&self) -> Result<Vec<NodeId>, DelayError>
    {
        let mut indegrees = self.nodes.iter().map(|node| node.fanins.len()).collect::<Vec<_>>();
        let mut stack = indegrees
            .iter()
            .enumerate()
            .filter_map(|(index, indegree)| (*indegree == 0).then_some(NodeId(index)))
            .collect::<Vec<_>>();
        let mut order = Vec::with_capacity(self.nodes.len());

        while let Some(node) = stack.pop()
        {
            order.push(node);
            for (fanout, _) in &self.nodes[node.0].fanouts
            {
                indegrees[fanout.0] -= 1;
                if indegrees[fanout.0] == 0
                {
                    stack.push(*fanout);
                }
            }
        }

        if order.len() != self.nodes.len()
        {
            return Err(DelayError::CycleDetected);
        }

        Ok(order)
    }

    fn require_node(&self, node: NodeId) -> Result<(), DelayError>
    {
        self.nodes
            .get(node.0)
            .map(|_| ())
            .ok_or(DelayError::MissingNode(node))
    }
}

pub fn delay_trace(network: &mut DelayNetwork, model: DelayModel) -> Result<(), DelayError>
{
    network.delay_trace(model)
}

pub fn delay_get_model_from_name(name: &str) -> DelayModel
{
    DelayModel::from_name(name)
}

pub fn compute_wire_load(table: &WireLoadTable, pins: usize) -> f64
{
    table.compute(pins)
}

pub fn delay_arrival_time(network: &DelayNetwork, node: NodeId) -> Result<DelayTime, DelayError>
{
    Ok(network.node(node)?.arrival)
}

pub fn delay_required_time(network: &DelayNetwork, node: NodeId) -> Result<DelayTime, DelayError>
{
    Ok(network.node(node)?.required)
}

pub fn delay_slack_time(network: &DelayNetwork, node: NodeId) -> Result<DelayTime, DelayError>
{
    Ok(network.node(node)?.slack)
}

pub fn delay_load(network: &DelayNetwork, node: NodeId) -> Result<f64, DelayError>
{
    Ok(network.node(node)?.load)
}

pub fn delay_latest_output(network: &DelayNetwork) -> Result<(Option<NodeId>, f64), DelayError>
{
    network.latest_output()
}

pub fn delay_mapped_decomposition_unavailable() -> Result<(), DelayError>
{
    Err(DelayError::MissingSisIntegration
    {
        operation: "mapped-node decomposition",
    })
}

pub fn delay_tdc_parameters_unavailable() -> Result<(), DelayError>
{
    Err(DelayError::MissingSisIntegration
    {
        operation: "timing-driven cofactoring delay parameters",
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum DelayError
{
    MissingNode(NodeId),
    MissingFanin
    {
        node: NodeId,
        pin: usize,
    },
    MissingNativePinDelay
    {
        node: NodeId,
        pin: usize,
        model: DelayModel,
    },
    BadModel(DelayModel),
    BadPinPhase,
    CycleDetected,
    MissingSisIntegration
    {
        operation: &'static str,
    },
}

impl fmt::Display for DelayError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingNode(node) => write!(f, "missing delay node {}", node.0),
            Self::MissingFanin { node, pin } =>
            {
                write!(f, "delay node {} has no fanin at pin {pin}", node.0)
            }
            Self::MissingNativePinDelay { node, pin, model } =>
            {
                write!(
                    f,
                    "delay node {} pin {pin} has no native {:?} pin delay",
                    node.0, model
                )
            }
            Self::BadModel(model) => write!(f, "unsupported delay model {:?}", model),
            Self::BadPinPhase => write!(f, "pin delay phase is not usable for timing"),
            Self::CycleDetected => write!(f, "delay network contains a cycle"),
            Self::MissingSisIntegration { operation } =>
            {
                write!(f, "{operation} requires native SIS graph or mapper integration")
            }
        }
    }
}

impl Error for DelayError {}

fn default_or_backup(value: f64, backup: f64) -> f64
{
    if value == DELAY_VALUE_NOT_GIVEN
    {
        backup
    }
    else
    {
        value
    }
}

fn merge_arrival(
    arrival: &mut DelayTime,
    fanin_arrival: DelayTime,
    delay: DelayTime,
    phase: PinPhase,
) -> Result<(), DelayError>
{
    match phase
    {
        PinPhase::Inverting =>
        {
            arrival.rise = arrival.rise.max(fanin_arrival.fall + delay.rise);
            arrival.fall = arrival.fall.max(fanin_arrival.rise + delay.fall);
        }
        PinPhase::NonInverting =>
        {
            arrival.rise = arrival.rise.max(fanin_arrival.rise + delay.rise);
            arrival.fall = arrival.fall.max(fanin_arrival.fall + delay.fall);
        }
        PinPhase::Neither =>
        {
            arrival.rise = arrival.rise.max(fanin_arrival.fall + delay.rise);
            arrival.fall = arrival.fall.max(fanin_arrival.rise + delay.fall);
            arrival.rise = arrival.rise.max(fanin_arrival.rise + delay.rise);
            arrival.fall = arrival.fall.max(fanin_arrival.fall + delay.fall);
        }
        PinPhase::NotGiven => return Err(DelayError::BadPinPhase),
    }

    Ok(())
}

fn required_at_input(
    required: DelayTime,
    delay: DelayTime,
    phase: PinPhase,
) -> Result<DelayTime, DelayError>
{
    let mut input_required = DelayTime::new(f64::INFINITY, f64::INFINITY);

    match phase
    {
        PinPhase::Inverting =>
        {
            input_required.rise = input_required.rise.min(required.fall - delay.fall);
            input_required.fall = input_required.fall.min(required.rise - delay.rise);
        }
        PinPhase::NonInverting =>
        {
            input_required.rise = input_required.rise.min(required.rise - delay.rise);
            input_required.fall = input_required.fall.min(required.fall - delay.fall);
        }
        PinPhase::Neither =>
        {
            input_required.rise = input_required.rise.min(required.fall - delay.fall);
            input_required.fall = input_required.fall.min(required.rise - delay.rise);
            input_required.rise = input_required.rise.min(required.rise - delay.rise);
            input_required.fall = input_required.fall.min(required.fall - delay.fall);
        }
        PinPhase::NotGiven => return Err(DelayError::BadPinPhase),
    }

    Ok(input_required)
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn assert_close(actual: f64, expected: f64)
    {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "actual {actual} != expected {expected}"
        );
    }

    fn library_pin(block: DelayTime, drive: DelayTime, phase: PinPhase, load: f64) -> DelayPin
    {
        DelayPin::new(block, drive, phase, load, f64::INFINITY, DelayTime::not_given())
    }

    fn sample_network() -> DelayNetwork
    {
        let mut network = DelayNetwork::new();
        network.default_arrival = DelayTime::new(0.0, 0.0);
        network.default_required = DelayTime::not_given();
        network.set_default_parameter(DelayDefaultParameter::WireLoadSlope, 0.1);
        network.set_default_parameter(DelayDefaultParameter::DefaultOutputLoad, 0.5);

        let a = network.add_node(DelayNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(DelayNode::new("b", NodeKind::PrimaryInput));
        let n = network.add_node(DelayNode::new("n", NodeKind::Internal));
        let y = network.add_node(DelayNode::new("y", NodeKind::PrimaryOutput));

        network
            .add_edge(
                a,
                n,
                InputPhase::PositiveUnate,
                Some(library_pin(
                    DelayTime::new(1.0, 2.0),
                    DelayTime::new(0.5, 0.25),
                    PinPhase::NonInverting,
                    0.3,
                )),
            )
            .unwrap();
        network
            .add_edge(
                b,
                n,
                InputPhase::NegativeUnate,
                Some(library_pin(
                    DelayTime::new(3.0, 4.0),
                    DelayTime::new(0.25, 0.5),
                    PinPhase::Inverting,
                    0.4,
                )),
            )
            .unwrap();
        network
            .add_edge(n, y, InputPhase::PositiveUnate, None)
            .unwrap();

        network
    }

    #[test]
    fn parses_delay_model_names()
    {
        assert_eq!(delay_get_model_from_name("unit"), DelayModel::Unit);
        assert_eq!(
            delay_get_model_from_name("unit-fanout"),
            DelayModel::UnitFanout
        );
        assert_eq!(delay_get_model_from_name("library"), DelayModel::Library);
        assert_eq!(delay_get_model_from_name("mapped"), DelayModel::Mapped);
        assert_eq!(delay_get_model_from_name("tdc"), DelayModel::Tdc);
        assert_eq!(delay_get_model_from_name("other"), DelayModel::Unknown);
    }

    #[test]
    fn wire_load_table_matches_c_lookup_slope_and_extrapolation_rules()
    {
        let mut table = WireLoadTable::default();
        assert_eq!(compute_wire_load(&table, 0), 0.0);
        assert_eq!(compute_wire_load(&table, 3), 0.0);

        table.pins = vec![1.0, 1.7];
        assert_eq!(compute_wire_load(&table, 1), 1.0);
        assert_eq!(compute_wire_load(&table, 2), 1.7);
        assert_close(compute_wire_load(&table, 4), 3.1);

        table.slope = 0.25;
        assert_eq!(compute_wire_load(&table, 4), 1.0);
    }

    #[test]
    fn trace_computes_loads_arrivals_required_times_and_slacks()
    {
        let mut network = sample_network();

        delay_trace(&mut network, DelayModel::Library).unwrap();

        let n = NodeId(2);
        let y = NodeId(3);
        assert_close(delay_load(&network, n).unwrap(), 0.6);
        assert_eq!(
            delay_arrival_time(&network, n).unwrap(),
            DelayTime::new(3.15, 4.3)
        );
        assert_eq!(
            delay_arrival_time(&network, y).unwrap(),
            DelayTime::new(3.15, 4.3)
        );
        assert_eq!(
            delay_required_time(&network, y).unwrap(),
            DelayTime::new(4.3, 4.3)
        );
        assert_eq!(
            delay_slack_time(&network, n).unwrap(),
            DelayTime::new(1.15, 0.0)
        );
        assert_eq!(delay_latest_output(&network).unwrap(), (Some(y), 4.3));
    }

    #[test]
    fn wire_required_and_slack_use_pin_phase_rules()
    {
        let mut network = sample_network();
        delay_trace(&mut network, DelayModel::Library).unwrap();

        let req_a = network
            .wire_required_time(NodeId(2), 0, DelayModel::Library)
            .unwrap();
        let req_b = network
            .wire_required_time(NodeId(2), 1, DelayModel::Library)
            .unwrap();

        assert_close(req_a.rise, 3.0);
        assert_close(req_a.fall, 2.15);
        assert_eq!(req_b, DelayTime::new(0.0, 1.15));
        assert_eq!(
            network
                .wire_slack_time(NodeId(2), 1, DelayModel::Library)
                .unwrap(),
            DelayTime::new(0.0, 1.15)
        );
    }

    #[test]
    fn unit_model_uses_input_phase_and_one_unit_block_delay()
    {
        let mut network = DelayNetwork::new();
        let a = network.add_node(DelayNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(DelayNode::new("b", NodeKind::PrimaryInput));
        let n = network.add_node(DelayNode::new("n", NodeKind::Internal));
        network
            .add_edge(a, n, InputPhase::PositiveUnate, None)
            .unwrap();
        network
            .add_edge(b, n, InputPhase::NegativeUnate, None)
            .unwrap();
        network.node_mut(a).unwrap().arrival = DelayTime::new(1.0, 5.0);
        network.node_mut(b).unwrap().arrival = DelayTime::new(2.0, 7.0);
        network.compute_arrival_time(n, DelayModel::Unit, 0.0).unwrap();

        assert_eq!(
            delay_arrival_time(&network, n).unwrap(),
            DelayTime::new(8.0, 6.0)
        );
    }

    #[test]
    fn pipo_defaults_fill_unspecified_terminal_pin_values()
    {
        let mut network = DelayNetwork::new();
        network.pipo_model.drive = DelayTime::new(0.4, 0.6);
        network.pipo_model.load = 2.0;
        let pi = network.add_node(DelayNode::new("a", NodeKind::PrimaryInput));

        let pin = network.get_pin_delay(pi, 0, DelayModel::Library).unwrap();

        assert_eq!(pin.drive, DelayTime::new(0.4, 0.6));
        assert_eq!(pin.block, DelayTime::zero());
        assert_eq!(pin.load, 2.0);
        assert_eq!(pin.max_load, f64::INFINITY);
    }

    #[test]
    fn explicit_terminal_user_times_override_network_defaults()
    {
        let mut network = DelayNetwork::new();
        network.default_arrival = DelayTime::new(9.0, 9.0);
        let pin = DelayPin::new(
            DelayTime::not_given(),
            DelayTime::zero(),
            PinPhase::NonInverting,
            DELAY_VALUE_NOT_GIVEN,
            DELAY_VALUE_NOT_GIVEN,
            DelayTime::new(1.0, 2.0),
        );
        let pi = network.add_node(DelayNode::new("a", NodeKind::PrimaryInput).with_terminal_pin(pin));

        network
            .compute_arrival_time(pi, DelayModel::Library, 0.0)
            .unwrap();

        assert_eq!(delay_arrival_time(&network, pi).unwrap(), DelayTime::new(1.0, 2.0));
    }

    #[test]
    fn parameters_round_trip_through_terminal_pin()
    {
        let mut network = DelayNetwork::new();
        let output = network.add_node(DelayNode::new("y", NodeKind::PrimaryOutput));

        network
            .set_parameter(output, DelayParameter::OutputLoad, 3.5)
            .unwrap();
        network
            .set_parameter(output, DelayParameter::RequiredRise, 6.0)
            .unwrap();
        network
            .set_parameter(output, DelayParameter::RequiredFall, 7.0)
            .unwrap();

        assert_eq!(
            network.get_parameter(output, DelayParameter::OutputLoad).unwrap(),
            3.5
        );
        assert_eq!(
            network.get_po_required_time(output).unwrap(),
            Some(DelayTime::new(6.0, 7.0))
        );
        assert_eq!(network.get_po_load(output).unwrap(), Some(3.5));
    }

    #[test]
    fn sis_bound_operations_report_typed_integration_errors()
    {
        assert_eq!(
            delay_mapped_decomposition_unavailable(),
            Err(DelayError::MissingSisIntegration
            {
                operation: "mapped-node decomposition",
            })
        );
        assert_eq!(
            delay_tdc_parameters_unavailable(),
            Err(DelayError::MissingSisIntegration
            {
                operation: "timing-driven cofactoring delay parameters",
            })
        );
    }
}
