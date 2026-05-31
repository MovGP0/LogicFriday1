//! Native Rust model for feasible behavior in `sis/speed/gbx.c`.
//!
//! Generalized bypass is tightly coupled to SIS `network_t`/`node_t` mutation:
//! Boolean-difference construction, KMS path duplication, fanin patching,
//! cutset selection, decomposition, and network sweeping all belong behind the
//! native graph backend. This module ports the timing record, bypass discovery,
//! cut weighting, and transform orchestration into an owned Rust graph model.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

const LARGE_SLACK: f64 = 1.0e29;
const GBX_MAXWEIGHT: i32 = 0x7fff_fffe;

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

impl InputPhase {
    pub fn compose_for_bypass(self, next: Self) -> Self {
        if self == next {
            Self::PositiveUnate
        } else {
            Self::NegativeUnate
        }
    }

    pub fn is_traceable(self) -> bool {
        matches!(self, Self::PositiveUnate | Self::NegativeUnate)
    }
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

    pub fn min_edge(self) -> f64 {
        self.rise.min(self.fall)
    }

    pub fn max_edge(self) -> f64 {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaninEdge {
    pub node: NodeId,
    pub phase: InputPhase,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GbxNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<FaninEdge>,
    pub fanouts: Vec<NodeId>,
    pub arrival: DelayTime,
    pub required: DelayTime,
    pub slack: DelayTime,
    pub pin_delays: Vec<DelayTime>,
}

impl GbxNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            arrival: DelayTime::zero(),
            required: DelayTime::zero(),
            slack: DelayTime::zero(),
            pin_delays: Vec::new(),
        }
    }

    pub fn with_timing(
        mut self,
        arrival: DelayTime,
        required: DelayTime,
        slack: DelayTime,
    ) -> Self {
        self.arrival = arrival;
        self.required = required;
        self.slack = slack;
        self
    }

    pub fn with_pin_delays(mut self, pin_delays: Vec<DelayTime>) -> Self {
        self.pin_delays = pin_delays;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GbxNetwork {
    nodes: Vec<GbxNode>,
}

impl GbxNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: GbxNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn add_fanin(
        &mut self,
        node: NodeId,
        fanin: NodeId,
        phase: InputPhase,
        pin_delay: DelayTime,
    ) -> Result<(), GbxError> {
        self.node(fanin)?;
        let node_data = self.node_mut(node)?;
        node_data.fanins.push(FaninEdge { node: fanin, phase });
        node_data.pin_delays.push(pin_delay);
        self.node_mut(fanin)?.fanouts.push(node);
        Ok(())
    }

    pub fn node(&self, id: NodeId) -> Result<&GbxNode, GbxError> {
        self.nodes.get(id.0).ok_or(GbxError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut GbxNode, GbxError> {
        self.nodes.get_mut(id.0).ok_or(GbxError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[GbxNode] {
        &self.nodes
    }

    pub fn fanin_index(&self, node: NodeId, fanin: NodeId) -> Result<usize, GbxError> {
        self.node(node)?
            .fanins
            .iter()
            .position(|edge| edge.node == fanin)
            .ok_or(GbxError::MissingFanin { node, fanin })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeBpRecord {
    pub pin_slacks: Vec<f64>,
    pub pin_weights: Vec<f64>,
    pub input_phases: Vec<InputPhase>,
    pub slack: f64,
    pub path_fanin: Option<NodeId>,
    pub path_slack: f64,
    pub mark: bool,
}

impl NodeBpRecord {
    pub fn primary_input(slack: f64) -> Self {
        Self {
            pin_slacks: Vec::new(),
            pin_weights: Vec::new(),
            input_phases: Vec::new(),
            slack,
            path_fanin: None,
            path_slack: LARGE_SLACK,
            mark: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bypass {
    pub first_node: NodeId,
    pub last_node: NodeId,
    pub gain: f64,
    pub slack: f64,
    pub side_delay: f64,
    pub control_delay: f64,
    pub weight: i32,
    pub dupe_at: Option<NodeId>,
    pub bypassed_nodes: Vec<NodeId>,
    pub phase: InputPhase,
    pub side_slack: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GbxTrace {
    NewTrace,
    NewerTrace,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GbxOptions {
    pub epsilon: f64,
    pub mux_delay: f64,
    pub delay_model: DelayModel,
    pub start_node_mode: bool,
    pub trace: GbxTrace,
}

impl GbxOptions {
    pub fn newer(epsilon: f64, mux_delay: f64, delay_model: DelayModel) -> Self {
        Self {
            epsilon,
            mux_delay,
            delay_model,
            start_node_mode: true,
            trace: GbxTrace::NewerTrace,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GbxAnalysis {
    pub records: HashMap<NodeId, NodeBpRecord>,
    pub bypasses: Vec<Bypass>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GbxTransformResult {
    NoBypassesFound,
    BypassesTaken,
    SomeBypassesNoCutset,
    NoCutset,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GbxError {
    UnknownNode(NodeId),
    MissingFanin { node: NodeId, fanin: NodeId },
    MissingPinDelay { node: NodeId, pin: usize },
    MissingRecord(NodeId),
}

impl fmt::Display for GbxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown GBX node {:?}", node),
            Self::MissingFanin { node, fanin } => {
                write!(f, "node {:?} does not have fanin {:?}", node, fanin)
            }
            Self::MissingPinDelay { node, pin, .. } => write!(
                f,
                "node {:?} has no native GBX pin delay for pin {pin}; SIS delay_node_pin is not ported",
                node
            ),
            Self::MissingRecord(node) => write!(f, "missing GBX timing record for {:?}", node),
        }
    }
}

impl Error for GbxError {}

pub fn build_node_table(network: &GbxNetwork) -> Result<HashMap<NodeId, NodeBpRecord>, GbxError> {
    let mut records = HashMap::with_capacity(network.nodes().len());
    for id in (0..network.nodes().len()).map(NodeId) {
        records.insert(id, new_node_bp_record(network, id)?);
    }
    Ok(records)
}

pub fn new_node_bp_record(network: &GbxNetwork, node: NodeId) -> Result<NodeBpRecord, GbxError> {
    let node_data = network.node(node)?;
    let slack = node_data.slack.min_edge();
    if node_data.kind == NodeKind::PrimaryInput {
        return Ok(NodeBpRecord::primary_input(slack));
    }

    let mut result = NodeBpRecord {
        pin_slacks: Vec::with_capacity(node_data.fanins.len()),
        pin_weights: Vec::with_capacity(node_data.fanins.len()),
        input_phases: Vec::with_capacity(node_data.fanins.len()),
        slack,
        path_fanin: None,
        path_slack: LARGE_SLACK,
        mark: false,
    };
    let mut min_slack = LARGE_SLACK;

    for (pin, edge) in node_data.fanins.iter().enumerate() {
        let delay = node_data
            .pin_delays
            .get(pin)
            .copied()
            .ok_or(GbxError::MissingPinDelay { node, pin })?;
        let fanin = network.node(edge.node)?;
        let mut required = node_data.required;
        let phase = if node_data.kind == NodeKind::PrimaryOutput {
            InputPhase::PositiveUnate
        } else {
            edge.phase
        };

        match phase {
            InputPhase::PositiveUnate => {
                required.rise -= delay.rise;
                required.fall -= delay.fall;
            }
            InputPhase::NegativeUnate => {
                required.rise -= delay.fall;
                required.fall -= delay.rise;
            }
            InputPhase::Binate | InputPhase::Unknown => {
                let required_min = required.rise.min(required.fall);
                let delay_max = delay.rise.max(delay.fall);
                required.rise = required_min - delay_max;
                required.fall = required.rise;
            }
        }

        let pin_slack =
            (required.rise - fanin.arrival.rise).min(required.fall - fanin.arrival.fall);
        result.pin_slacks.push(pin_slack);
        result.pin_weights.push(delay.max_edge());
        result.input_phases.push(phase);

        if pin_slack <= min_slack {
            result.path_slack = min_slack;
            min_slack = pin_slack;
            result.path_fanin = Some(edge.node);
        } else if pin_slack < result.path_slack {
            result.path_slack = pin_slack;
        }
    }

    result.path_slack -= min_slack;
    Ok(result)
}

pub fn retrieve_slack(
    records: &HashMap<NodeId, NodeBpRecord>,
    network: &GbxNetwork,
    fanout: NodeId,
    node: NodeId,
) -> Result<f64, GbxError> {
    let pin = network.fanin_index(fanout, node)?;
    records
        .get(&fanout)
        .ok_or(GbxError::MissingRecord(fanout))?
        .pin_slacks
        .get(pin)
        .copied()
        .ok_or(GbxError::MissingFanin {
            node: fanout,
            fanin: node,
        })
}

pub fn weight(
    records: &HashMap<NodeId, NodeBpRecord>,
    network: &GbxNetwork,
    fanout: NodeId,
    node: NodeId,
) -> Result<f64, GbxError> {
    let pin = network.fanin_index(fanout, node)?;
    records
        .get(&fanout)
        .ok_or(GbxError::MissingRecord(fanout))?
        .pin_weights
        .get(pin)
        .copied()
        .ok_or(GbxError::MissingFanin {
            node: fanout,
            fanin: node,
        })
}

pub fn retrieve_phase(
    records: &HashMap<NodeId, NodeBpRecord>,
    network: &GbxNetwork,
    fanout: NodeId,
    node: NodeId,
) -> Result<InputPhase, GbxError> {
    let pin = network.fanin_index(fanout, node)?;
    records
        .get(&fanout)
        .ok_or(GbxError::MissingRecord(fanout))?
        .input_phases
        .get(pin)
        .copied()
        .ok_or(GbxError::MissingFanin {
            node: fanout,
            fanin: node,
        })
}

pub fn new_bypass(
    network: &GbxNetwork,
    node: NodeId,
    fanout: NodeId,
    edge_weight: f64,
    slack: f64,
    phase: InputPhase,
) -> Result<Bypass, GbxError> {
    let control_delay = network.node(node)?.arrival.max_edge();
    let mut side_delay: f64 = 0.0;
    for edge in &network.node(fanout)?.fanins {
        if edge.node != node {
            side_delay = side_delay.max(network.node(edge.node)?.arrival.max_edge());
        }
    }

    Ok(Bypass {
        first_node: node,
        last_node: fanout,
        gain: edge_weight,
        slack,
        side_delay,
        control_delay,
        weight: 0,
        dupe_at: None,
        bypassed_nodes: vec![fanout],
        phase,
        side_slack: slack,
    })
}

pub fn bypass_is_extensible(
    bypass: &Bypass,
    network: &GbxNetwork,
    fanout: NodeId,
    node: NodeId,
    edge_weight: f64,
) -> Result<bool, GbxError> {
    if network.node(fanout)?.kind == NodeKind::PrimaryOutput {
        return Ok(false);
    }

    let mut control_delay = 0.0;
    let mut side_delay: f64 = 0.0;
    for edge in &network.node(fanout)?.fanins {
        if edge.node == node {
            control_delay = network.node(edge.node)?.arrival.max_edge();
        } else {
            side_delay = side_delay.max(network.node(edge.node)?.arrival.max_edge());
        }
    }
    let side_slack = bypass.side_slack.min(control_delay - side_delay);
    Ok(bypass.gain + edge_weight <= side_slack)
}

pub fn bypass_add_node(
    bypass: &mut Bypass,
    network: &GbxNetwork,
    records: &HashMap<NodeId, NodeBpRecord>,
    fanout: NodeId,
    node: NodeId,
    edge_weight: f64,
    phase: InputPhase,
) -> Result<(), GbxError> {
    bypass.last_node = fanout;
    bypass.bypassed_nodes.push(fanout);
    if network.node(node)?.fanouts.len() > 1 {
        bypass.dupe_at = Some(node);
    }
    bypass.gain += edge_weight;
    bypass.phase = bypass.phase.compose_for_bypass(phase);
    bypass.slack = bypass.slack.min(
        records
            .get(&fanout)
            .ok_or(GbxError::MissingRecord(fanout))?
            .path_slack,
    );
    Ok(())
}

pub fn bypass_new_add_node(
    bypass: &mut Bypass,
    network: &GbxNetwork,
    records: &HashMap<NodeId, NodeBpRecord>,
    fanout: NodeId,
    node: NodeId,
) -> Result<(), GbxError> {
    let mut control_delay = 0.0;
    let mut side_delay: f64 = 0.0;
    for edge in &network.node(fanout)?.fanins {
        if edge.node == node {
            control_delay = network.node(edge.node)?.arrival.max_edge();
        } else {
            side_delay = side_delay.max(network.node(edge.node)?.arrival.max_edge());
        }
    }
    bypass.side_slack = bypass.side_slack.min(control_delay - side_delay);
    let phase = retrieve_phase(records, network, fanout, node)?;
    let edge_weight = weight(records, network, fanout, node)?;
    bypass_add_node(bypass, network, records, fanout, node, edge_weight, phase)
}

pub fn path_fanouts(
    records: &HashMap<NodeId, NodeBpRecord>,
    network: &GbxNetwork,
    node: NodeId,
    epsilon: f64,
) -> Result<Vec<NodeId>, GbxError> {
    let mut result = Vec::new();
    for fanout in network.node(node)?.fanouts.iter().copied() {
        let record = records
            .get(&fanout)
            .ok_or(GbxError::MissingRecord(fanout))?;
        let pin = network.fanin_index(fanout, node)?;
        let phase = record.input_phases[pin];
        if record.pin_slacks[pin] <= epsilon
            && network.node(fanout)?.kind != NodeKind::PrimaryOutput
            && phase.is_traceable()
            && record.path_fanin == Some(node)
            && record.path_slack != 0.0
        {
            result.push(fanout);
        }
    }
    Ok(result)
}

pub fn path_fanout(
    records: &HashMap<NodeId, NodeBpRecord>,
    network: &GbxNetwork,
    node: NodeId,
    epsilon: f64,
) -> Result<Option<NodeId>, GbxError> {
    let mut found = None;
    for fanout in network.node(node)?.fanouts.iter().copied() {
        let record = records
            .get(&fanout)
            .ok_or(GbxError::MissingRecord(fanout))?;
        if record.path_fanin != Some(node) {
            return Ok(None);
        }
        if record.slack > epsilon {
            continue;
        }
        if network.node(fanout)?.kind == NodeKind::PrimaryOutput {
            return Ok(None);
        }
        if found.is_some() {
            return Ok(None);
        }
        let pin = network.fanin_index(fanout, node)?;
        if !record.input_phases[pin].is_traceable() {
            return Ok(None);
        }
        found = Some(fanout);
    }
    Ok(found)
}

pub fn bypass_extension_fanouts(
    bypass: &Bypass,
    records: &HashMap<NodeId, NodeBpRecord>,
    network: &GbxNetwork,
    node: NodeId,
) -> Result<Vec<NodeId>, GbxError> {
    let mut result = Vec::new();
    for fanout in network.node(node)?.fanouts.iter().copied() {
        let edge_weight = weight(records, network, fanout, node)?;
        if bypass_is_extensible(bypass, network, fanout, node, edge_weight)? {
            result.push(fanout);
        }
    }
    Ok(result)
}

pub fn find_bypasses(network: &GbxNetwork, options: &GbxOptions) -> Result<GbxAnalysis, GbxError> {
    let records = build_node_table(network)?;
    let bypasses = match options.trace {
        GbxTrace::NewTrace => new_find_bypass_nodes(network, &records, options)?,
        GbxTrace::NewerTrace => newer_find_bypass_nodes(network, &records, options)?,
    };
    Ok(GbxAnalysis { records, bypasses })
}

pub fn assign_bypass_weights(bypasses: &mut [Bypass]) {
    if bypasses.is_empty() {
        return;
    }
    let max_gain = bypasses
        .iter()
        .map(|bypass| bypass.gain)
        .fold(0.0, f64::max)
        + 1.0;
    for bypass in bypasses {
        bypass.weight = (0.5 + max_gain - bypass.gain) as i32;
    }
}

pub fn critical_node_cut_weights(
    network: &GbxNetwork,
    records: &HashMap<NodeId, NodeBpRecord>,
    bypasses: &[Bypass],
    epsilon: f64,
) -> HashMap<NodeId, i32> {
    let mut result = HashMap::new();
    let max_weight = bypasses
        .iter()
        .map(|bypass| bypass.weight)
        .max()
        .unwrap_or(0);
    for bypass in bypasses {
        result.insert(bypass.last_node, bypass.weight);
    }

    let noncrit_weight = max_weight
        .saturating_mul(bypasses.len() as i32)
        .saturating_add(1);
    let noncrit_count = network
        .nodes()
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            !matches!(node.kind, NodeKind::PrimaryInput | NodeKind::PrimaryOutput)
                && !result.contains_key(&NodeId(*index))
                && records
                    .get(&NodeId(*index))
                    .is_some_and(|record| record.slack > epsilon)
        })
        .count() as i32;
    let critical_weight = noncrit_weight.saturating_mul(noncrit_count.saturating_add(1));

    for (index, node) in network.nodes().iter().enumerate() {
        let id = NodeId(index);
        if result.contains_key(&id)
            || matches!(node.kind, NodeKind::PrimaryInput | NodeKind::PrimaryOutput)
        {
            continue;
        }
        if records
            .get(&id)
            .is_some_and(|record| record.slack <= epsilon)
        {
            result.insert(id, critical_weight.min(GBX_MAXWEIGHT));
        } else {
            result.insert(id, noncrit_weight);
        }
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GbxApplyOptions {
    pub mux_delay: f64,
    pub actual_mux_delay: f64,
    pub delay_model: DelayModel,
}

impl GbxApplyOptions {
    pub const fn new(mux_delay: f64, actual_mux_delay: f64, delay_model: DelayModel) -> Self {
        Self {
            mux_delay,
            actual_mux_delay,
            delay_model,
        }
    }
}

pub trait GbxTransformBackend {
    type Error: Error + Send + Sync + 'static;

    fn select_cutset(
        &mut self,
        cut_weights: &HashMap<NodeId, i32>,
    ) -> Result<Vec<NodeId>, Self::Error>;

    fn take_bypass(
        &mut self,
        bypass: &Bypass,
        options: GbxApplyOptions,
    ) -> Result<bool, Self::Error>;

    fn decompose_after_unit_delay(&mut self, options: GbxApplyOptions) -> Result<(), Self::Error>;

    fn sweep(&mut self) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum GbxTransformError<E> {
    Analysis(GbxError),
    Backend(E),
}

impl<E> fmt::Display for GbxTransformError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Analysis(error) => write!(f, "{error}"),
            Self::Backend(error) => write!(f, "{error}"),
        }
    }
}

impl<E> Error for GbxTransformError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Analysis(error) => Some(error),
            Self::Backend(error) => Some(error),
        }
    }
}

pub fn take_bypass_bound<B>(
    backend: &mut B,
    bypass: &Bypass,
    options: GbxApplyOptions,
) -> Result<bool, GbxTransformError<B::Error>>
where
    B: GbxTransformBackend,
{
    backend
        .take_bypass(bypass, options)
        .map_err(GbxTransformError::Backend)
}

pub fn do_gbx_transform_bound<B>(
    backend: &mut B,
    network: &GbxNetwork,
    options: &GbxOptions,
    actual_mux_delay: f64,
) -> Result<GbxTransformResult, GbxTransformError<B::Error>>
where
    B: GbxTransformBackend,
{
    let mut analysis = find_bypasses(network, options).map_err(GbxTransformError::Analysis)?;
    if analysis.bypasses.is_empty() {
        return Ok(GbxTransformResult::NoBypassesFound);
    }

    assign_bypass_weights(&mut analysis.bypasses);
    let by_last = analysis
        .bypasses
        .iter()
        .map(|bypass| (bypass.last_node, bypass))
        .collect::<HashMap<_, _>>();
    let cut_weights = critical_node_cut_weights(
        network,
        &analysis.records,
        &analysis.bypasses,
        options.epsilon,
    );
    let cutset = backend
        .select_cutset(&cut_weights)
        .map_err(GbxTransformError::Backend)?;
    let apply_options =
        GbxApplyOptions::new(options.mux_delay, actual_mux_delay, options.delay_model);
    let mut taken = false;
    let mut no_improvement = false;

    for node in cutset {
        if let Some(bypass) = by_last.get(&node) {
            if take_bypass_bound(backend, bypass, apply_options)? {
                taken = true;
            }
        } else if analysis
            .records
            .get(&node)
            .is_some_and(|record| record.slack == 0.0)
        {
            no_improvement = true;
        }
    }

    if matches!(
        options.delay_model,
        DelayModel::Unit | DelayModel::UnitFanout
    ) && actual_mux_delay > 1.0
    {
        backend
            .decompose_after_unit_delay(apply_options)
            .map_err(GbxTransformError::Backend)?;
    }
    backend.sweep().map_err(GbxTransformError::Backend)?;

    if !taken {
        Ok(GbxTransformResult::NoCutset)
    } else if no_improvement {
        Ok(GbxTransformResult::SomeBypassesNoCutset)
    } else {
        Ok(GbxTransformResult::BypassesTaken)
    }
}

pub fn do_bypass_transform_bound<B>(
    backend: &mut B,
    network: &GbxNetwork,
    options: &GbxOptions,
    actual_mux_delay: f64,
) -> Result<GbxTransformResult, GbxTransformError<B::Error>>
where
    B: GbxTransformBackend,
{
    let mut analysis = find_bypasses(network, options).map_err(GbxTransformError::Analysis)?;
    if analysis.bypasses.is_empty() {
        return Ok(GbxTransformResult::NoBypassesFound);
    }

    analysis
        .bypasses
        .sort_by(|left, right| right.gain.total_cmp(&left.gain));
    let apply_options =
        GbxApplyOptions::new(options.mux_delay, actual_mux_delay, options.delay_model);
    let mut taken = false;
    for bypass in &analysis.bypasses {
        if take_bypass_bound(backend, bypass, apply_options)? {
            taken = true;
        }
    }

    if matches!(
        options.delay_model,
        DelayModel::Unit | DelayModel::UnitFanout
    ) && actual_mux_delay > 1.0
    {
        backend
            .decompose_after_unit_delay(apply_options)
            .map_err(GbxTransformError::Backend)?;
    }
    backend.sweep().map_err(GbxTransformError::Backend)?;

    if taken {
        Ok(GbxTransformResult::BypassesTaken)
    } else {
        Ok(GbxTransformResult::NoCutset)
    }
}

fn new_find_bypass_nodes(
    network: &GbxNetwork,
    records: &HashMap<NodeId, NodeBpRecord>,
    options: &GbxOptions,
) -> Result<Vec<Bypass>, GbxError> {
    let mut registry = BypassRegistry::default();
    for node in (0..network.nodes().len()).map(NodeId) {
        let record = records.get(&node).ok_or(GbxError::MissingRecord(node))?;
        if record.slack > options.epsilon || network.node(node)?.kind == NodeKind::PrimaryOutput {
            continue;
        }

        for fanout in path_fanouts(records, network, node, options.epsilon)? {
            if let Some(bypass) = new_trace_bypass(network, records, options, fanout, node)? {
                registry.register_best(bypass);
            }
        }
    }
    Ok(registry.into_vec())
}

fn newer_find_bypass_nodes(
    network: &GbxNetwork,
    records: &HashMap<NodeId, NodeBpRecord>,
    options: &GbxOptions,
) -> Result<Vec<Bypass>, GbxError> {
    let mut registry = BypassRegistry::default();
    for node in (0..network.nodes().len()).map(NodeId) {
        let record = records.get(&node).ok_or(GbxError::MissingRecord(node))?;
        if record.slack > options.epsilon || network.node(node)?.kind == NodeKind::PrimaryOutput {
            continue;
        }

        for fanout in path_fanouts(records, network, node, options.epsilon)? {
            if let Some(bypass) = newer_trace_bypass(network, records, options, fanout, node)? {
                extend_bypass(
                    bypass,
                    fanout,
                    network,
                    records,
                    options.mux_delay,
                    &mut registry,
                    &mut HashSet::new(),
                )?;
            }
        }
    }
    Ok(registry.into_vec())
}

fn new_trace_bypass(
    network: &GbxNetwork,
    records: &HashMap<NodeId, NodeBpRecord>,
    options: &GbxOptions,
    fanout: NodeId,
    node: NodeId,
) -> Result<Option<Bypass>, GbxError> {
    let slack_offset = retrieve_slack(records, network, fanout, node)?;
    let mut edge_weight = weight(records, network, fanout, node)?;
    let mut slack = minimum_side_pin_slack(records, network, fanout, node)?;

    if is_start_node_relaxed(options, network, fanout)? {
        if slack - slack_offset > 0.0 {
            slack += 1.0;
        } else {
            return Ok(None);
        }
    } else if slack - slack_offset < edge_weight {
        return Ok(None);
    }

    let mut bypass = new_bypass(
        network,
        node,
        fanout,
        edge_weight,
        slack - slack_offset,
        retrieve_phase(records, network, fanout, node)?,
    )?;
    let mut node = fanout;
    let mut next = path_fanout(records, network, node, options.epsilon)?;
    while slack >= bypass.gain + slack_offset {
        let Some(fanout) = next else { break };
        slack = slack.min(minimum_side_pin_slack(records, network, fanout, node)?);
        edge_weight = weight(records, network, fanout, node)?;
        if slack >= bypass.gain + edge_weight + slack_offset {
            let phase = retrieve_phase(records, network, fanout, node)?;
            bypass_add_node(
                &mut bypass,
                network,
                records,
                fanout,
                node,
                edge_weight,
                phase,
            )?;
        } else {
            break;
        }
        node = fanout;
        next = path_fanout(records, network, node, options.epsilon)?;
    }
    bypass.gain -= options.mux_delay;
    Ok((bypass.gain > 0.0).then_some(bypass))
}

fn newer_trace_bypass(
    network: &GbxNetwork,
    records: &HashMap<NodeId, NodeBpRecord>,
    options: &GbxOptions,
    fanout: NodeId,
    node: NodeId,
) -> Result<Option<Bypass>, GbxError> {
    let slack_offset = retrieve_slack(records, network, fanout, node)?;
    let edge_weight = weight(records, network, fanout, node)?;
    let mut slack = minimum_side_pin_slack(records, network, fanout, node)?;

    if is_start_node_relaxed(options, network, fanout)? {
        if slack - slack_offset > 0.0 {
            slack += 1.0;
        } else {
            return Ok(None);
        }
    } else if slack - slack_offset < edge_weight {
        return Ok(None);
    }

    let bypass = new_bypass(
        network,
        node,
        fanout,
        edge_weight,
        slack - slack_offset,
        retrieve_phase(records, network, fanout, node)?,
    )?;
    Ok((bypass.gain <= bypass.side_slack).then_some(bypass))
}

fn extend_bypass(
    mut bypass: Bypass,
    mut node: NodeId,
    network: &GbxNetwork,
    records: &HashMap<NodeId, NodeBpRecord>,
    mux_delay: f64,
    registry: &mut BypassRegistry,
    active: &mut HashSet<NodeId>,
) -> Result<(), GbxError> {
    if !active.insert(node) {
        return Ok(());
    }

    let mut fanouts = bypass_extension_fanouts(&bypass, records, network, node)?;
    while bypass.gain <= bypass.side_slack && !fanouts.is_empty() {
        let last_index = fanouts.len() - 1;
        for fanout in fanouts.iter().take(last_index).copied() {
            let mut branch = bypass.clone();
            bypass_new_add_node(&mut branch, network, records, fanout, node)?;
            extend_bypass(
                branch, fanout, network, records, mux_delay, registry, active,
            )?;
        }

        let fanout = fanouts[last_index];
        bypass_new_add_node(&mut bypass, network, records, fanout, node)?;
        active.remove(&node);
        node = fanout;
        if !active.insert(node) {
            return Ok(());
        }
        fanouts = bypass_extension_fanouts(&bypass, records, network, node)?;
    }

    bypass.gain -= mux_delay;
    if bypass.gain > 0.0 {
        registry.register_best(bypass);
    }
    active.remove(&node);
    Ok(())
}

fn minimum_side_pin_slack(
    records: &HashMap<NodeId, NodeBpRecord>,
    network: &GbxNetwork,
    fanout: NodeId,
    path_node: NodeId,
) -> Result<f64, GbxError> {
    let record = records
        .get(&fanout)
        .ok_or(GbxError::MissingRecord(fanout))?;
    let mut slack = LARGE_SLACK;
    for edge in &network.node(fanout)?.fanins {
        if edge.node != path_node {
            slack = slack.min(retrieve_slack(records, network, fanout, edge.node)?);
        }
    }
    if network.node(fanout)?.fanins.len() == 1 {
        Ok(record.path_slack)
    } else {
        Ok(slack)
    }
}

fn is_start_node_relaxed(
    options: &GbxOptions,
    network: &GbxNetwork,
    fanout: NodeId,
) -> Result<bool, GbxError> {
    Ok(options.start_node_mode
        && matches!(
            options.delay_model,
            DelayModel::Unit | DelayModel::UnitFanout
        )
        && network.node(fanout)?.fanins.len() <= 2)
}

#[derive(Default)]
struct BypassRegistry {
    by_last: HashMap<NodeId, Bypass>,
}

impl BypassRegistry {
    fn register_best(&mut self, bypass: Bypass) {
        match self.by_last.get(&bypass.last_node) {
            Some(old) if old.gain >= bypass.gain => {}
            _ => {
                self.by_last.insert(bypass.last_node, bypass);
            }
        }
    }

    fn into_vec(self) -> Vec<Bypass> {
        let mut result: Vec<_> = self.by_last.into_values().collect();
        result.sort_by(|left, right| right.gain.total_cmp(&left.gain));
        result
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

    fn node(name: &str, slack: f64, arrival: f64, required: f64) -> GbxNode {
        GbxNode::new(name, NodeKind::Internal).with_timing(
            DelayTime::new(arrival, arrival),
            DelayTime::new(required, required),
            DelayTime::new(slack, slack),
        )
    }

    fn pi(name: &str, arrival: f64, slack: f64) -> GbxNode {
        GbxNode::new(name, NodeKind::PrimaryInput).with_timing(
            DelayTime::new(arrival, arrival),
            DelayTime::zero(),
            DelayTime::new(slack, slack),
        )
    }

    #[test]
    fn node_record_computes_pin_slacks_weights_and_path_side_slack() {
        let mut network = GbxNetwork::new();
        let a = network.add_node(pi("a", 3.0, 0.0));
        let b = network.add_node(pi("b", 5.0, 0.0));
        let n = network.add_node(node("n", 0.25, 0.0, 10.0));
        network
            .add_fanin(n, a, InputPhase::PositiveUnate, DelayTime::new(1.0, 1.0))
            .unwrap();
        network
            .add_fanin(n, b, InputPhase::NegativeUnate, DelayTime::new(2.0, 4.0))
            .unwrap();

        let record = new_node_bp_record(&network, n).unwrap();

        assert_eq!(record.path_fanin, Some(b));
        assert_eq!(record.pin_weights, vec![1.0, 4.0]);
        assert_eq!(
            record.input_phases,
            vec![InputPhase::PositiveUnate, InputPhase::NegativeUnate]
        );
        assert_close(record.pin_slacks[0], 6.0);
        assert_close(record.pin_slacks[1], 1.0);
        assert_close(record.path_slack, 5.0);
    }

    #[test]
    fn path_fanouts_filter_to_critical_unate_non_po_path_edges() {
        let mut network = GbxNetwork::new();
        let a = network.add_node(pi("a", 0.0, 0.0));
        let side = network.add_node(pi("side", 0.0, 0.0));
        let good = network.add_node(node("good", 0.0, 0.0, 1.0));
        let loose = network.add_node(node("loose", 3.0, 0.0, 2.0));
        let binate = network.add_node(node("binate", 0.0, 0.0, 1.0));
        network
            .add_fanin(good, a, InputPhase::PositiveUnate, DelayTime::new(1.0, 1.0))
            .unwrap();
        network
            .add_fanin(
                good,
                side,
                InputPhase::PositiveUnate,
                DelayTime::new(0.5, 0.5),
            )
            .unwrap();
        network
            .add_fanin(
                loose,
                a,
                InputPhase::PositiveUnate,
                DelayTime::new(1.0, 1.0),
            )
            .unwrap();
        network
            .add_fanin(binate, a, InputPhase::Binate, DelayTime::new(1.0, 1.0))
            .unwrap();

        let records = build_node_table(&network).unwrap();

        assert_eq!(
            path_fanouts(&records, &network, a, 0.0).unwrap(),
            vec![good]
        );
    }

    #[test]
    fn new_bypass_initializes_delays_and_bypassed_path() {
        let mut network = GbxNetwork::new();
        let a = network.add_node(pi("a", 8.0, 0.0));
        let side = network.add_node(pi("side", 3.0, 0.0));
        let fanout = network.add_node(node("fanout", 0.0, 10.0, 12.0));
        network
            .add_fanin(
                fanout,
                a,
                InputPhase::PositiveUnate,
                DelayTime::new(1.0, 1.0),
            )
            .unwrap();
        network
            .add_fanin(
                fanout,
                side,
                InputPhase::PositiveUnate,
                DelayTime::new(1.0, 1.0),
            )
            .unwrap();

        let bypass = new_bypass(&network, a, fanout, 2.0, 4.0, InputPhase::PositiveUnate).unwrap();

        assert_eq!(bypass.first_node, a);
        assert_eq!(bypass.last_node, fanout);
        assert_eq!(bypass.bypassed_nodes, vec![fanout]);
        assert_close(bypass.control_delay, 8.0);
        assert_close(bypass.side_delay, 3.0);
        assert_close(bypass.side_slack, 4.0);
    }

    #[test]
    fn bypass_new_add_node_updates_gain_phase_side_slack_and_duplicate_point() {
        let mut network = GbxNetwork::new();
        let a = network.add_node(pi("a", 10.0, 0.0));
        let side = network.add_node(pi("side", 7.0, 0.0));
        let first = network.add_node(node("first", 0.0, 11.0, 12.0));
        let second = network.add_node(node("second", 0.0, 12.0, 15.0));
        let extra = network.add_node(node("extra", 4.0, 0.0, 1.0));
        network
            .add_fanin(
                first,
                a,
                InputPhase::NegativeUnate,
                DelayTime::new(1.0, 1.0),
            )
            .unwrap();
        network
            .add_fanin(
                second,
                first,
                InputPhase::NegativeUnate,
                DelayTime::new(2.0, 2.0),
            )
            .unwrap();
        network
            .add_fanin(
                second,
                side,
                InputPhase::PositiveUnate,
                DelayTime::new(1.0, 1.0),
            )
            .unwrap();
        network
            .add_fanin(
                extra,
                first,
                InputPhase::PositiveUnate,
                DelayTime::new(1.0, 1.0),
            )
            .unwrap();
        let records = build_node_table(&network).unwrap();
        let mut bypass =
            new_bypass(&network, a, first, 1.0, 10.0, InputPhase::NegativeUnate).unwrap();

        bypass_new_add_node(&mut bypass, &network, &records, second, first).unwrap();

        assert_eq!(bypass.last_node, second);
        assert_eq!(bypass.bypassed_nodes, vec![first, second]);
        assert_eq!(bypass.dupe_at, Some(first));
        assert_eq!(bypass.phase, InputPhase::PositiveUnate);
        assert_close(bypass.gain, 3.0);
        assert_close(bypass.side_slack, 4.0);
    }

    #[test]
    fn newer_find_bypasses_extends_branching_paths_and_keeps_positive_gain() {
        let mut network = GbxNetwork::new();
        let a = network.add_node(pi("a", 10.0, 0.0));
        let side1 = network.add_node(pi("side1", 8.0, 0.0));
        let side2 = network.add_node(pi("side2", 7.0, 0.0));
        let n1 = network.add_node(node("n1", 0.0, 11.0, 11.0));
        let n2 = network.add_node(node("n2", 0.0, 12.0, 13.0));
        network
            .add_fanin(n1, a, InputPhase::PositiveUnate, DelayTime::new(1.0, 1.0))
            .unwrap();
        network
            .add_fanin(
                n1,
                side1,
                InputPhase::PositiveUnate,
                DelayTime::new(0.5, 0.5),
            )
            .unwrap();
        network
            .add_fanin(n2, n1, InputPhase::PositiveUnate, DelayTime::new(2.0, 2.0))
            .unwrap();
        network
            .add_fanin(
                n2,
                side2,
                InputPhase::PositiveUnate,
                DelayTime::new(0.5, 0.5),
            )
            .unwrap();

        let options = GbxOptions::newer(0.0, 1.0, DelayModel::Unit);
        let analysis = find_bypasses(&network, &options).unwrap();

        assert_eq!(analysis.bypasses.len(), 1);
        let bypass = &analysis.bypasses[0];
        assert_eq!(bypass.first_node, a);
        assert_eq!(bypass.last_node, n2);
        assert_eq!(bypass.bypassed_nodes, vec![n1, n2]);
        assert_close(bypass.gain, 2.0);
    }

    #[test]
    fn assign_bypass_weights_inverts_gain_ranking_like_c_cut_weights() {
        let mut bypasses = vec![
            Bypass {
                first_node: NodeId(0),
                last_node: NodeId(1),
                gain: 3.0,
                slack: 0.0,
                side_delay: 0.0,
                control_delay: 0.0,
                weight: 0,
                dupe_at: None,
                bypassed_nodes: vec![NodeId(1)],
                phase: InputPhase::PositiveUnate,
                side_slack: 0.0,
            },
            Bypass {
                first_node: NodeId(2),
                last_node: NodeId(3),
                gain: 1.5,
                slack: 0.0,
                side_delay: 0.0,
                control_delay: 0.0,
                weight: 0,
                dupe_at: None,
                bypassed_nodes: vec![NodeId(3)],
                phase: InputPhase::NegativeUnate,
                side_slack: 0.0,
            },
        ];

        assign_bypass_weights(&mut bypasses);

        assert_eq!(bypasses[0].weight, 1);
        assert_eq!(bypasses[1].weight, 3);
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestError;

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "failed")
        }
    }

    impl Error for TestError {}

    #[derive(Clone, Debug, PartialEq)]
    enum Event {
        SelectCutset(Vec<NodeId>),
        TakeBypass {
            first: NodeId,
            last: NodeId,
            mux_delay: f64,
            actual_mux_delay: f64,
        },
        DecomposeAfterUnitDelay,
        Sweep,
    }

    struct RecordingBackend {
        cutset: Vec<NodeId>,
        take_results: Vec<bool>,
        events: Vec<Event>,
    }

    impl RecordingBackend {
        fn new(cutset: Vec<NodeId>, take_results: Vec<bool>) -> Self {
            Self {
                cutset,
                take_results,
                events: Vec::new(),
            }
        }
    }

    impl GbxTransformBackend for RecordingBackend {
        type Error = TestError;

        fn select_cutset(
            &mut self,
            cut_weights: &HashMap<NodeId, i32>,
        ) -> Result<Vec<NodeId>, Self::Error> {
            let mut nodes = cut_weights.keys().copied().collect::<Vec<_>>();
            nodes.sort_by_key(|node| node.0);
            self.events.push(Event::SelectCutset(nodes));
            Ok(self.cutset.clone())
        }

        fn take_bypass(
            &mut self,
            bypass: &Bypass,
            options: GbxApplyOptions,
        ) -> Result<bool, Self::Error> {
            self.events.push(Event::TakeBypass {
                first: bypass.first_node,
                last: bypass.last_node,
                mux_delay: options.mux_delay,
                actual_mux_delay: options.actual_mux_delay,
            });
            Ok(self.take_results.pop().unwrap_or(true))
        }

        fn decompose_after_unit_delay(
            &mut self,
            _options: GbxApplyOptions,
        ) -> Result<(), Self::Error> {
            self.events.push(Event::DecomposeAfterUnitDelay);
            Ok(())
        }

        fn sweep(&mut self) -> Result<(), Self::Error> {
            self.events.push(Event::Sweep);
            Ok(())
        }
    }

    fn bypass_network() -> (GbxNetwork, NodeId, NodeId, NodeId) {
        let mut network = GbxNetwork::new();
        let a = network.add_node(pi("a", 10.0, 0.0));
        let side1 = network.add_node(pi("side1", 8.0, 0.0));
        let side2 = network.add_node(pi("side2", 7.0, 0.0));
        let n1 = network.add_node(node("n1", 0.0, 11.0, 11.0));
        let n2 = network.add_node(node("n2", 0.0, 12.0, 13.0));
        network
            .add_fanin(n1, a, InputPhase::PositiveUnate, DelayTime::new(1.0, 1.0))
            .unwrap();
        network
            .add_fanin(
                n1,
                side1,
                InputPhase::PositiveUnate,
                DelayTime::new(0.5, 0.5),
            )
            .unwrap();
        network
            .add_fanin(n2, n1, InputPhase::PositiveUnate, DelayTime::new(2.0, 2.0))
            .unwrap();
        network
            .add_fanin(
                n2,
                side2,
                InputPhase::PositiveUnate,
                DelayTime::new(0.5, 0.5),
            )
            .unwrap();

        (network, a, n1, n2)
    }

    #[test]
    fn gbx_cutset_transform_selects_weighted_cut_and_applies_matching_bypass() {
        let (network, a, _n1, n2) = bypass_network();
        let options = GbxOptions::newer(0.0, 1.0, DelayModel::Unit);
        let mut backend = RecordingBackend::new(vec![n2], vec![true]);

        assert_eq!(
            do_gbx_transform_bound(&mut backend, &network, &options, 2.0),
            Ok(GbxTransformResult::BypassesTaken)
        );

        assert_eq!(
            backend.events,
            vec![
                Event::SelectCutset(vec![NodeId(3), NodeId(4)]),
                Event::TakeBypass {
                    first: a,
                    last: n2,
                    mux_delay: 1.0,
                    actual_mux_delay: 2.0,
                },
                Event::DecomposeAfterUnitDelay,
                Event::Sweep,
            ]
        );
    }

    #[test]
    fn gbx_all_bypasses_transform_runs_bypasses_in_gain_order() {
        let (network, a, _n1, n2) = bypass_network();
        let options = GbxOptions::newer(0.0, 1.0, DelayModel::Unit);
        let mut backend = RecordingBackend::new(Vec::new(), vec![true]);

        assert_eq!(
            do_bypass_transform_bound(&mut backend, &network, &options, 1.0),
            Ok(GbxTransformResult::BypassesTaken)
        );

        assert_eq!(
            backend.events,
            vec![
                Event::TakeBypass {
                    first: a,
                    last: n2,
                    mux_delay: 1.0,
                    actual_mux_delay: 1.0,
                },
                Event::Sweep,
            ]
        );
    }
}
