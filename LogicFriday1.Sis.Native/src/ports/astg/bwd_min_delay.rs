use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinDelayTime
{
    pub rise: f64,
    pub fall: f64,
}

impl MinDelayTime
{
    pub const fn new(rise: f64, fall: f64) -> Self
    {
        Self { rise, fall }
    }

    pub const fn zero() -> Self
    {
        Self {
            rise: 0.0,
            fall: 0.0,
        }
    }

    pub const fn infinity() -> Self
    {
        Self {
            rise: f64::INFINITY,
            fall: f64::INFINITY,
        }
    }

    pub const fn negative_infinity() -> Self
    {
        Self {
            rise: f64::NEG_INFINITY,
            fall: f64::NEG_INFINITY,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinDelayPinPhase
{
    NotGiven,
    Inverting,
    NonInverting,
    Neither,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinDelayInputPhase
{
    PositiveUnate,
    NegativeUnate,
    Binate,
}

impl From<MinDelayInputPhase> for MinDelayPinPhase
{
    fn from(value: MinDelayInputPhase) -> Self
    {
        match value
        {
            MinDelayInputPhase::PositiveUnate => Self::NonInverting,
            MinDelayInputPhase::NegativeUnate => Self::Inverting,
            MinDelayInputPhase::Binate => Self::Neither,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinDelayModel
{
    Unit,
    UnitFanout,
    Library,
    Mapped,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinDelayPin
{
    pub block: MinDelayTime,
    pub drive: MinDelayTime,
    pub phase: MinDelayPinPhase,
    pub load: f64,
    pub max_load: f64,
    pub user_time: Option<MinDelayTime>,
}

impl MinDelayPin
{
    pub const fn new(
        block: MinDelayTime,
        drive: MinDelayTime,
        phase: MinDelayPinPhase,
        load: f64,
        max_load: f64,
        user_time: Option<MinDelayTime>,
    ) -> Self
    {
        Self {
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
            MinDelayTime::new(1.0, 1.0),
            MinDelayTime::zero(),
            MinDelayPinPhase::NonInverting,
            1.0,
            f64::INFINITY,
            None,
        )
    }

    pub const fn unit_fanout() -> Self
    {
        Self::new(
            MinDelayTime::new(1.0, 1.0),
            MinDelayTime::new(0.2, 0.2),
            MinDelayPinPhase::NonInverting,
            1.0,
            f64::INFINITY,
            None,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MinDelayNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinDelayNodeKind
{
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinDelayFanin
{
    pub node: MinDelayNodeId,
    pub phase: MinDelayInputPhase,
    pub pin_delay: Option<MinDelayPin>,
}

impl MinDelayFanin
{
    pub const fn new(
        node: MinDelayNodeId,
        phase: MinDelayInputPhase,
        pin_delay: Option<MinDelayPin>,
    ) -> Self
    {
        Self {
            node,
            phase,
            pin_delay,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinDelayNode
{
    pub name: String,
    pub kind: MinDelayNodeKind,
    pub fanins: Vec<MinDelayFanin>,
    pub fanouts: Vec<(MinDelayNodeId, usize)>,
    pub terminal_pin: Option<MinDelayPin>,
    pub arrival: MinDelayTime,
    pub required: MinDelayTime,
    pub slack: MinDelayTime,
    pub load: f64,
}

impl MinDelayNode
{
    pub fn new(name: impl Into<String>, kind: MinDelayNodeKind) -> Self
    {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            terminal_pin: None,
            arrival: MinDelayTime::zero(),
            required: MinDelayTime::zero(),
            slack: MinDelayTime::zero(),
            load: 0.0,
        }
    }

    pub fn with_terminal_pin(mut self, pin: MinDelayPin) -> Self
    {
        self.terminal_pin = Some(pin);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinDelayWireLoadTable
{
    pub slope: Option<f64>,
    pub pins: Vec<f64>,
}

impl Default for MinDelayWireLoadTable
{
    fn default() -> Self
    {
        Self {
            slope: None,
            pins: Vec::new(),
        }
    }
}

impl MinDelayWireLoadTable
{
    pub fn compute(&self, pins: usize) -> f64
    {
        if pins == 0
        {
            return 0.0;
        }

        let count = self.pins.len();
        if pins <= count
        {
            return self.pins[pins - 1];
        }
        if let Some(slope) = self.slope.filter(|value| *value > 0.0)
        {
            return slope * pins as f64;
        }
        if count == 0
        {
            return 0.0;
        }
        if count == 1
        {
            return self.pins[0];
        }

        let extra = (pins - count) as f64;
        let last = self.pins[count - 1];
        let previous = self.pins[count - 2];
        last * (1.0 + extra) - extra * previous
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinDelayNetwork
{
    nodes: Vec<MinDelayNode>,
    primary_inputs: Vec<MinDelayNodeId>,
    primary_outputs: Vec<MinDelayNodeId>,
    pub default_arrival: Option<MinDelayTime>,
    pub default_required: Option<MinDelayTime>,
    pub wire_load_table: MinDelayWireLoadTable,
    pub pipo_model: MinDelayPin,
}

impl Default for MinDelayNetwork
{
    fn default() -> Self
    {
        Self {
            nodes: Vec::new(),
            primary_inputs: Vec::new(),
            primary_outputs: Vec::new(),
            default_arrival: None,
            default_required: None,
            wire_load_table: MinDelayWireLoadTable::default(),
            pipo_model: MinDelayPin::new(
                MinDelayTime::zero(),
                MinDelayTime::zero(),
                MinDelayPinPhase::NonInverting,
                0.0,
                f64::INFINITY,
                None,
            ),
        }
    }
}

impl MinDelayNetwork
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn add_node(&mut self, node: MinDelayNode) -> MinDelayNodeId
    {
        let id = MinDelayNodeId(self.nodes.len());
        match node.kind
        {
            MinDelayNodeKind::PrimaryInput => self.primary_inputs.push(id),
            MinDelayNodeKind::PrimaryOutput => self.primary_outputs.push(id),
            MinDelayNodeKind::Internal =>
            {}
        }
        self.nodes.push(node);
        id
    }

    pub fn add_edge(
        &mut self,
        source: MinDelayNodeId,
        sink: MinDelayNodeId,
        phase: MinDelayInputPhase,
        pin_delay: Option<MinDelayPin>,
    ) -> Result<(), MinDelayError>
    {
        self.require_node(source)?;
        self.require_node(sink)?;

        let pin = self.nodes[sink.0].fanins.len();
        self.nodes[sink.0]
            .fanins
            .push(MinDelayFanin::new(source, phase, pin_delay));
        self.nodes[source.0].fanouts.push((sink, pin));
        Ok(())
    }

    pub fn node(&self, node: MinDelayNodeId) -> Result<&MinDelayNode, MinDelayError>
    {
        self.nodes
            .get(node.0)
            .ok_or(MinDelayError::MissingNode(node))
    }

    pub fn node_mut(&mut self, node: MinDelayNodeId) -> Result<&mut MinDelayNode, MinDelayError>
    {
        self.nodes
            .get_mut(node.0)
            .ok_or(MinDelayError::MissingNode(node))
    }

    pub fn nodes(&self) -> &[MinDelayNode]
    {
        &self.nodes
    }

    pub fn min_delay_trace(&mut self, model: MinDelayModel) -> Result<(), MinDelayError>
    {
        let order = self.topological_order()?;

        for node in &order
        {
            let load = self.compute_fanout_load(*node, model)?;
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
            node.slack = MinDelayTime::new(
                node.required.rise - node.arrival.rise,
                node.required.fall - node.arrival.fall,
            );
        }

        Ok(())
    }

    pub fn compute_fanout_load(
        &self,
        node: MinDelayNodeId,
        model: MinDelayModel,
    ) -> Result<f64, MinDelayError>
    {
        self.require_node(node)?;

        let mut load = self
            .wire_load_table
            .compute(self.nodes[node.0].fanouts.len());
        for (fanout, pin) in &self.nodes[node.0].fanouts
        {
            if self.nodes[fanout.0].kind == MinDelayNodeKind::PrimaryOutput
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
        node: MinDelayNodeId,
        pin: usize,
        model: MinDelayModel,
    ) -> Result<MinDelayTime, MinDelayError>
    {
        self.require_node(node)?;
        if self.nodes[node.0].kind == MinDelayNodeKind::PrimaryOutput
        {
            return Ok(MinDelayTime::zero());
        }

        let load = self.compute_fanout_load(node, model)?;
        self.nodes[node.0].load = load;
        let pin_delay = self.get_pin_delay(node, pin, model)?;
        self.compute_delay(node, pin_delay)
    }

    pub fn wire_required_time(
        &self,
        node: MinDelayNodeId,
        fanin_index: usize,
        model: MinDelayModel,
    ) -> Result<MinDelayTime, MinDelayError>
    {
        let node_data = self.node(node)?;
        if node_data.kind == MinDelayNodeKind::PrimaryOutput
        {
            return Ok(node_data.required);
        }
        if node_data.kind != MinDelayNodeKind::PrimaryInput && fanin_index >= node_data.fanins.len()
        {
            return Err(MinDelayError::MissingFanin {
                node,
                pin: fanin_index,
            });
        }

        let pin_delay = self.get_pin_delay(node, fanin_index, model)?;
        let delay = self.compute_delay(node, pin_delay)?;
        let phase = if node_data.kind == MinDelayNodeKind::PrimaryInput
        {
            MinDelayPinPhase::NonInverting
        }
        else
        {
            pin_delay.phase
        };

        required_at_input(node_data.required, delay, phase)
    }

    pub fn wire_slack_time(
        &self,
        node: MinDelayNodeId,
        fanin_index: usize,
        model: MinDelayModel,
    ) -> Result<MinDelayTime, MinDelayError>
    {
        let node_data = self.node(node)?;
        let arrival = if node_data.kind == MinDelayNodeKind::PrimaryInput
        {
            node_data.arrival
        }
        else
        {
            let fanin = node_data
                .fanins
                .get(fanin_index)
                .ok_or(MinDelayError::MissingFanin {
                    node,
                    pin: fanin_index,
                })?
                .node;
            self.node(fanin)?.arrival
        };
        let required = self.wire_required_time(node, fanin_index, model)?;

        Ok(MinDelayTime::new(
            required.rise - arrival.rise,
            required.fall - arrival.fall,
        ))
    }

    pub fn latest_output(&self) -> Result<(Option<MinDelayNodeId>, f64), MinDelayError>
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
        node: MinDelayNodeId,
        pin: usize,
        model: MinDelayModel,
    ) -> Result<MinDelayPin, MinDelayError>
    {
        let node_data = self.node(node)?;
        if matches!(
            node_data.kind,
            MinDelayNodeKind::PrimaryInput | MinDelayNodeKind::PrimaryOutput
        )
        {
            return Ok(self.io_delay(node, model));
        }

        match model
        {
            MinDelayModel::Unit =>
            {
                let fanin = node_data
                    .fanins
                    .get(pin)
                    .ok_or(MinDelayError::MissingFanin { node, pin })?;
                let mut delay = MinDelayPin::unit();
                delay.phase = fanin.phase.into();
                Ok(delay)
            }
            MinDelayModel::UnitFanout =>
            {
                let fanin = node_data
                    .fanins
                    .get(pin)
                    .ok_or(MinDelayError::MissingFanin { node, pin })?;
                let mut delay = MinDelayPin::unit_fanout();
                delay.phase = fanin.phase.into();
                Ok(delay)
            }
            MinDelayModel::Library | MinDelayModel::Mapped => node_data
                .fanins
                .get(pin)
                .ok_or(MinDelayError::MissingFanin { node, pin })?
                .pin_delay
                .ok_or(MinDelayError::MissingNativePinDelay { node, pin, model }),
        }
    }

    fn set_arrival_time(
        &mut self,
        node: MinDelayNodeId,
        model: MinDelayModel,
    ) -> Result<(), MinDelayError>
    {
        let kind = self.nodes[node.0].kind;
        if kind == MinDelayNodeKind::PrimaryInput
        {
            let pin_delay = self.get_pin_delay(node, 0, model)?;
            let delay = self.compute_delay(node, pin_delay)?;
            let mut arrival = self.primary_input_arrival_time(node);
            arrival.rise += delay.rise;
            arrival.fall += delay.fall;
            self.nodes[node.0].arrival = arrival;
            return Ok(());
        }
        if kind == MinDelayNodeKind::PrimaryOutput
        {
            let fanin = self
                .nodes
                .get(node.0)
                .and_then(|node_data| node_data.fanins.first())
                .ok_or(MinDelayError::MissingFanin { node, pin: 0 })?
                .node;
            self.nodes[node.0].arrival = self.nodes[fanin.0].arrival;
            return Ok(());
        }

        let fanins = self.nodes[node.0].fanins.clone();
        let mut arrival = MinDelayTime::infinity();
        for (pin, fanin) in fanins.iter().enumerate()
        {
            let pin_delay = self.get_pin_delay(node, pin, model)?;
            let delay = self.compute_delay(node, pin_delay)?;
            let fanin_arrival = self.nodes[fanin.node.0].arrival;
            merge_min_arrival(&mut arrival, fanin_arrival, delay, pin_delay.phase)?;
        }

        self.nodes[node.0].arrival = arrival;
        Ok(())
    }

    fn set_required_time(
        &mut self,
        node: MinDelayNodeId,
        latest: f64,
        model: MinDelayModel,
    ) -> Result<(), MinDelayError>
    {
        if self.nodes[node.0].kind == MinDelayNodeKind::PrimaryOutput
        {
            self.nodes[node.0].required = self.primary_output_required_time(node, latest);
            return Ok(());
        }

        let fanouts = self.nodes[node.0].fanouts.clone();
        let mut required = MinDelayTime::infinity();
        for (fanout, pin) in fanouts
        {
            let fanout_required = self.nodes[fanout.0].required;
            if self.nodes[fanout.0].kind == MinDelayNodeKind::PrimaryOutput
            {
                required.rise = required.rise.min(fanout_required.rise);
                required.fall = required.fall.min(fanout_required.fall);
                continue;
            }

            let pin_delay = self.get_pin_delay(fanout, pin, model)?;
            let delay = self.compute_delay(fanout, pin_delay)?;
            merge_min_required(&mut required, fanout_required, delay, pin_delay.phase)?;
        }

        self.nodes[node.0].required = required;
        Ok(())
    }

    fn compute_delay(
        &self,
        node: MinDelayNodeId,
        pin_delay: MinDelayPin,
    ) -> Result<MinDelayTime, MinDelayError>
    {
        let node_data = self.node(node)?;
        let mut delay = MinDelayTime::new(
            pin_delay.drive.rise * node_data.load,
            pin_delay.drive.fall * node_data.load,
        );
        if node_data.kind != MinDelayNodeKind::PrimaryInput
        {
            delay.rise += pin_delay.block.rise;
            delay.fall += pin_delay.block.fall;
        }

        Ok(delay)
    }

    fn io_delay(&self, node: MinDelayNodeId, model: MinDelayModel) -> MinDelayPin
    {
        match model
        {
            MinDelayModel::Unit => MinDelayPin::unit(),
            MinDelayModel::UnitFanout => MinDelayPin::unit_fanout(),
            MinDelayModel::Library | MinDelayModel::Mapped =>
            {
                self.nodes[node.0].terminal_pin.unwrap_or(self.pipo_model)
            }
        }
    }

    fn primary_input_arrival_time(&self, node: MinDelayNodeId) -> MinDelayTime
    {
        if let Some(time) = self.nodes[node.0]
            .terminal_pin
            .and_then(|pin| pin.user_time)
        {
            return time;
        }
        self.default_arrival.unwrap_or_else(MinDelayTime::zero)
    }

    fn primary_output_required_time(&self, node: MinDelayNodeId, latest: f64) -> MinDelayTime
    {
        if let Some(time) = self.nodes[node.0]
            .terminal_pin
            .and_then(|pin| pin.user_time)
        {
            return time;
        }
        self.default_required
            .unwrap_or_else(|| MinDelayTime::new(latest, latest))
    }

    fn primary_output_load(&self, node: MinDelayNodeId) -> f64
    {
        self.nodes[node.0]
            .terminal_pin
            .map(|pin| pin.load)
            .unwrap_or(self.pipo_model.load)
    }

    fn topological_order(&self) -> Result<Vec<MinDelayNodeId>, MinDelayError>
    {
        let mut indegrees = self
            .nodes
            .iter()
            .map(|node| node.fanins.len())
            .collect::<Vec<_>>();
        let mut stack = indegrees
            .iter()
            .enumerate()
            .filter_map(|(index, indegree)| (*indegree == 0).then_some(MinDelayNodeId(index)))
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
            return Err(MinDelayError::CycleDetected);
        }

        Ok(order)
    }

    fn require_node(&self, node: MinDelayNodeId) -> Result<(), MinDelayError>
    {
        self.nodes
            .get(node.0)
            .map(|_| ())
            .ok_or(MinDelayError::MissingNode(node))
    }
}

pub fn bwd_min_delay_trace(
    network: &mut MinDelayNetwork,
    model: MinDelayModel,
) -> Result<(), MinDelayError>
{
    network.min_delay_trace(model)
}

pub fn min_delay_latest_output(
    network: &MinDelayNetwork,
) -> Result<(Option<MinDelayNodeId>, f64), MinDelayError>
{
    network.latest_output()
}

pub fn min_delay_generate_decomposition_unavailable() -> Result<(), MinDelayError>
{
    Err(MinDelayError::MissingSisIntegration {
        operation: "mapped-node decomposition",
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum MinDelayError
{
    MissingNode(MinDelayNodeId),
    MissingFanin
    {
        node: MinDelayNodeId,
        pin: usize,
    },
    MissingNativePinDelay
    {
        node: MinDelayNodeId,
        pin: usize,
        model: MinDelayModel,
    },
    BadPinPhase,
    CycleDetected,
    MissingSisIntegration
    {
        operation: &'static str,
    },
}

impl fmt::Display for MinDelayError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingNode(node) => write!(f, "missing minimum-delay node {}", node.0),
            Self::MissingFanin { node, pin } =>
            {
                write!(f, "minimum-delay node {} has no fanin at pin {pin}", node.0)
            }
            Self::MissingNativePinDelay { node, pin, model } =>
            {
                write!(
                    f,
                    "minimum-delay node {} pin {pin} has no native {:?} pin delay",
                    node.0, model
                )
            }
            Self::BadPinPhase => write!(f, "pin delay phase is not usable for minimum timing"),
            Self::CycleDetected => write!(f, "minimum-delay network contains a cycle"),
            Self::MissingSisIntegration { operation } =>
            {
                write!(
                    f,
                    "{operation} requires native SIS graph or mapper integration"
                )
            }
        }
    }
}

impl Error for MinDelayError {}

fn merge_min_arrival(
    arrival: &mut MinDelayTime,
    fanin_arrival: MinDelayTime,
    delay: MinDelayTime,
    phase: MinDelayPinPhase,
) -> Result<(), MinDelayError>
{
    match phase
    {
        MinDelayPinPhase::Inverting =>
        {
            arrival.rise = arrival.rise.min(fanin_arrival.fall + delay.rise);
            arrival.fall = arrival.fall.min(fanin_arrival.rise + delay.fall);
        }
        MinDelayPinPhase::NonInverting =>
        {
            arrival.rise = arrival.rise.min(fanin_arrival.rise + delay.rise);
            arrival.fall = arrival.fall.min(fanin_arrival.fall + delay.fall);
        }
        MinDelayPinPhase::Neither =>
        {
            arrival.rise = arrival.rise.min(fanin_arrival.fall + delay.rise);
            arrival.fall = arrival.fall.min(fanin_arrival.rise + delay.fall);
            arrival.rise = arrival.rise.min(fanin_arrival.rise + delay.rise);
            arrival.fall = arrival.fall.min(fanin_arrival.fall + delay.fall);
        }
        MinDelayPinPhase::NotGiven => return Err(MinDelayError::BadPinPhase),
    }

    Ok(())
}

fn merge_min_required(
    required: &mut MinDelayTime,
    fanout_required: MinDelayTime,
    delay: MinDelayTime,
    phase: MinDelayPinPhase,
) -> Result<(), MinDelayError>
{
    match phase
    {
        MinDelayPinPhase::Inverting =>
        {
            required.rise = required.rise.min(fanout_required.fall - delay.fall);
            required.fall = required.fall.min(fanout_required.rise - delay.rise);
        }
        MinDelayPinPhase::NonInverting =>
        {
            required.rise = required.rise.min(fanout_required.rise - delay.rise);
            required.fall = required.fall.min(fanout_required.fall - delay.fall);
        }
        MinDelayPinPhase::Neither =>
        {
            required.rise = required.rise.min(fanout_required.fall - delay.fall);
            required.fall = required.fall.min(fanout_required.rise - delay.rise);
            required.rise = required.rise.min(fanout_required.rise - delay.rise);
            required.fall = required.fall.min(fanout_required.fall - delay.fall);
        }
        MinDelayPinPhase::NotGiven => return Err(MinDelayError::BadPinPhase),
    }

    Ok(())
}

fn required_at_input(
    required: MinDelayTime,
    delay: MinDelayTime,
    phase: MinDelayPinPhase,
) -> Result<MinDelayTime, MinDelayError>
{
    let mut input_required = MinDelayTime::infinity();
    merge_min_required(&mut input_required, required, delay, phase)?;
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

    fn pin(
        block: MinDelayTime,
        drive: MinDelayTime,
        phase: MinDelayPinPhase,
        load: f64,
    ) -> MinDelayPin
    {
        MinDelayPin::new(block, drive, phase, load, f64::INFINITY, None)
    }

    fn sample_network() -> MinDelayNetwork
    {
        let mut network = MinDelayNetwork::new();
        network.default_arrival = Some(MinDelayTime::zero());
        network.wire_load_table.slope = Some(0.1);
        network.pipo_model.load = 0.5;

        let a = network.add_node(MinDelayNode::new("a", MinDelayNodeKind::PrimaryInput));
        let b = network.add_node(MinDelayNode::new("b", MinDelayNodeKind::PrimaryInput));
        let n = network.add_node(MinDelayNode::new("n", MinDelayNodeKind::Internal));
        let y = network.add_node(MinDelayNode::new("y", MinDelayNodeKind::PrimaryOutput));

        network
            .add_edge(
                a,
                n,
                MinDelayInputPhase::PositiveUnate,
                Some(pin(
                    MinDelayTime::new(4.0, 5.0),
                    MinDelayTime::new(0.5, 0.25),
                    MinDelayPinPhase::NonInverting,
                    0.3,
                )),
            )
            .unwrap();
        network
            .add_edge(
                b,
                n,
                MinDelayInputPhase::NegativeUnate,
                Some(pin(
                    MinDelayTime::new(1.0, 2.0),
                    MinDelayTime::new(0.25, 0.5),
                    MinDelayPinPhase::Inverting,
                    0.4,
                )),
            )
            .unwrap();
        network
            .add_edge(n, y, MinDelayInputPhase::PositiveUnate, None)
            .unwrap();

        network
    }

    #[test]
    fn wire_load_table_matches_lookup_slope_and_extrapolation_rules()
    {
        let mut table = MinDelayWireLoadTable::default();
        assert_eq!(table.compute(0), 0.0);
        assert_eq!(table.compute(3), 0.0);

        table.pins = vec![1.0, 1.7];
        assert_eq!(table.compute(1), 1.0);
        assert_eq!(table.compute(2), 1.7);
        assert_close(table.compute(4), 3.1);

        table.slope = Some(0.25);
        assert_eq!(table.compute(4), 1.0);
    }

    #[test]
    fn trace_uses_minimum_arrival_path_and_computes_slack()
    {
        let mut network = sample_network();

        bwd_min_delay_trace(&mut network, MinDelayModel::Library).unwrap();

        let n = MinDelayNodeId(2);
        let y = MinDelayNodeId(3);
        assert_close(network.node(n).unwrap().load, 0.6);
        assert_eq!(
            network.node(n).unwrap().arrival,
            MinDelayTime::new(1.15, 2.3)
        );
        assert_eq!(
            network.node(y).unwrap().arrival,
            MinDelayTime::new(1.15, 2.3)
        );
        assert_eq!(
            network.node(y).unwrap().required,
            MinDelayTime::new(2.3, 2.3)
        );
        assert_eq!(network.node(n).unwrap().slack, MinDelayTime::new(1.15, 0.0));
        assert_eq!(min_delay_latest_output(&network).unwrap(), (Some(y), 2.3));
    }

    #[test]
    fn inverting_and_neither_phases_choose_earliest_compatible_edge()
    {
        let mut network = MinDelayNetwork::new();
        let a = network.add_node(MinDelayNode::new("a", MinDelayNodeKind::PrimaryInput));
        let b = network.add_node(MinDelayNode::new("b", MinDelayNodeKind::PrimaryInput));
        let n = network.add_node(MinDelayNode::new("n", MinDelayNodeKind::Internal));

        network
            .add_edge(a, n, MinDelayInputPhase::NegativeUnate, None)
            .unwrap();
        network
            .add_edge(b, n, MinDelayInputPhase::Binate, None)
            .unwrap();
        network.node_mut(a).unwrap().arrival = MinDelayTime::new(9.0, 2.0);
        network.node_mut(b).unwrap().arrival = MinDelayTime::new(7.0, 3.0);
        network.nodes[n.0].load = 0.0;
        network.set_arrival_time(n, MinDelayModel::Unit).unwrap();

        assert_eq!(
            network.node(n).unwrap().arrival,
            MinDelayTime::new(3.0, 4.0)
        );
    }

    #[test]
    fn wire_required_and_slack_use_pin_phase_rules()
    {
        let mut network = sample_network();
        bwd_min_delay_trace(&mut network, MinDelayModel::Library).unwrap();

        let req_a = network
            .wire_required_time(MinDelayNodeId(2), 0, MinDelayModel::Library)
            .unwrap();
        let req_b = network
            .wire_required_time(MinDelayNodeId(2), 1, MinDelayModel::Library)
            .unwrap();

        assert_close(req_a.rise, -2.0);
        assert_close(req_a.fall, -2.85);
        assert_eq!(req_b, MinDelayTime::new(0.0, 1.15));
        assert_eq!(
            network
                .wire_slack_time(MinDelayNodeId(2), 1, MinDelayModel::Library)
                .unwrap(),
            MinDelayTime::new(0.0, 1.15)
        );
    }

    #[test]
    fn explicit_terminal_user_times_override_network_defaults()
    {
        let mut network = MinDelayNetwork::new();
        network.default_arrival = Some(MinDelayTime::new(9.0, 9.0));
        let pin = MinDelayPin::new(
            MinDelayTime::zero(),
            MinDelayTime::zero(),
            MinDelayPinPhase::NonInverting,
            0.0,
            f64::INFINITY,
            Some(MinDelayTime::new(1.0, 2.0)),
        );
        let pi = network.add_node(
            MinDelayNode::new("a", MinDelayNodeKind::PrimaryInput).with_terminal_pin(pin),
        );

        network
            .set_arrival_time(pi, MinDelayModel::Library)
            .unwrap();

        assert_eq!(
            network.node(pi).unwrap().arrival,
            MinDelayTime::new(1.0, 2.0)
        );
    }

    #[test]
    fn sis_bound_mapped_decomposition_reports_typed_integration_error()
    {
        assert_eq!(
            min_delay_generate_decomposition_unavailable(),
            Err(MinDelayError::MissingSisIntegration {
                operation: "mapped-node decomposition",
            })
        );
    }
}
