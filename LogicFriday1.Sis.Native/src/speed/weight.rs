//! Native Rust port of the independent math in `sis/speed/weight.c`.
//!
//! The C file combines two separable concerns: numeric weighting of timing
//! samples and SIS graph traversal through `node_t`, `delay_*`, `lib_gate_t`,
//! `array_t`, and `st_table`. The numeric model is ported here as owned Rust
//! data. The graph-bound entry points intentionally return typed dependency
//! errors until the prerequisite native ports exist.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;
pub const NEG_LARGE: f64 = -10_000.0;
pub const MAXWEIGHT: f64 = 1_000.0;

const SP_PI: f64 = std::f64::consts::PI;
const SP_PI_2: f64 = std::f64::consts::FRAC_PI_2;
const SP_PI_4: f64 = std::f64::consts::FRAC_PI_4;
const SP_1_PI: f64 = std::f64::consts::FRAC_1_PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub fn min_edge(self) -> f64 {
        self.rise.min(self.fall)
    }

    pub fn max_edge(self) -> f64 {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unknown,
    Unit,
    Mapped,
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
pub struct TimingSample {
    pub arrival: DelayTime,
    pub slack: DelayTime,
    pub critical: bool,
}

impl TimingSample {
    pub const fn new(arrival: DelayTime, slack: DelayTime, critical: bool) -> Self {
        Self {
            arrival,
            slack,
            critical,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeightPoint {
    pub arrival_time: f64,
    pub delay_time: f64,
    pub critical: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WeightNodeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutPin {
    pub node: WeightNodeId,
    pub pin: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinTiming {
    pub phase: InputPhase,
    pub delay: DelayTime,
}

impl PinTiming {
    pub const fn new(phase: InputPhase, delay: DelayTime) -> Self {
        Self { phase, delay }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedWeightNode {
    pub kind: NodeKind,
    pub fanins: Vec<WeightNodeId>,
    pub fanouts: Vec<FanoutPin>,
    pub pins: Vec<PinTiming>,
    pub arrival: DelayTime,
    pub slack: DelayTime,
    pub required: DelayTime,
    pub critical: bool,
    pub gate_area: Option<f64>,
    pub literal_count: usize,
}

impl SpeedWeightNode {
    pub fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            pins: Vec::new(),
            arrival: DelayTime::new(0.0, 0.0),
            slack: DelayTime::new(0.0, 0.0),
            required: DelayTime::new(POS_LARGE, POS_LARGE),
            critical: false,
            gate_area: None,
            literal_count: 0,
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<WeightNodeId>, pins: Vec<PinTiming>) -> Self {
        self.fanins = fanins;
        self.pins = pins;
        self
    }

    pub fn with_timing(
        mut self,
        arrival: DelayTime,
        slack: DelayTime,
        required: DelayTime,
        critical: bool,
    ) -> Self {
        self.arrival = arrival;
        self.slack = slack;
        self.required = required;
        self.critical = critical;
        self
    }

    pub fn with_area(mut self, gate_area: Option<f64>, literal_count: usize) -> Self {
        self.gate_area = gate_area;
        self.literal_count = literal_count;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpeedWeightGraph {
    nodes: Vec<SpeedWeightNode>,
}

impl SpeedWeightGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: SpeedWeightNode) -> Result<WeightNodeId, SpeedWeightError> {
        if node.fanins.len() != node.pins.len() {
            return Err(SpeedWeightError::MismatchedPinCount {
                fanins: node.fanins.len(),
                pins: node.pins.len(),
            });
        }
        for fanin in &node.fanins {
            self.node(*fanin)?;
        }
        let id = WeightNodeId(self.nodes.len());
        self.nodes.push(node);
        for pin in 0..self.nodes[id.0].fanins.len() {
            let fanin = self.nodes[id.0].fanins[pin];
            self.nodes[fanin.0]
                .fanouts
                .push(FanoutPin { node: id, pin });
        }
        Ok(id)
    }

    pub fn node(&self, id: WeightNodeId) -> Result<&SpeedWeightNode, SpeedWeightError> {
        self.nodes
            .get(id.0)
            .ok_or(SpeedWeightError::UnknownNode(id))
    }
    pub fn nodes(&self) -> &[SpeedWeightNode] {
        &self.nodes
    }
    pub fn speed_weight(
        &self,
        node: WeightNodeId,
        distance: i32,
    ) -> Result<WeightComputation, SpeedWeightError> {
        self.speed_weight_with_model(node, distance, DelayModel::Unknown)
    }

    pub fn speed_weight_with_model(
        &self,
        node: WeightNodeId,
        distance: i32,
        duplicated_area_model: DelayModel,
    ) -> Result<WeightComputation, SpeedWeightError> {
        if distance <= 0 {
            return Err(SpeedWeightError::InvalidDistance(distance));
        }
        let output_arrival_time = self.node(node)?.arrival.max_edge();
        let traversal = self.weight_bfs(node, distance, output_arrival_time)?;
        let duplicated_area = self.compute_duplicated_area(
            &traversal.collapse_nodes,
            &traversal.table,
            duplicated_area_model,
        )?;
        let mut stats = WeightStats::default();
        for point in traversal.points {
            stats.push(point);
        }
        finish_weight_computation(stats, distance, duplicated_area)
    }

    pub fn compute_duplicated_area(
        &self,
        collapse_nodes: &[WeightNodeId],
        collapse_table: &HashSet<WeightNodeId>,
        model: DelayModel,
    ) -> Result<f64, SpeedWeightError> {
        let mut area_table = HashMap::new();
        for node in collapse_nodes.iter().skip(1).copied() {
            if area_table.contains_key(&node) || self.node(node)?.fanouts.len() <= 1 {
                continue;
            }
            if self
                .node(node)?
                .fanouts
                .iter()
                .any(|fanout| !collapse_table.contains(&fanout.node))
            {
                self.mark_input_cone(node, &mut area_table, collapse_table, model)?;
            }
        }
        area_table
            .keys()
            .copied()
            .map(|node| self.area_for_node(node))
            .sum()
    }

    pub fn compute_side_required_time(
        &self,
        node: WeightNodeId,
        collapse_table: &HashSet<WeightNodeId>,
        duplicated_required: &HashMap<WeightNodeId, Option<DelayTime>>,
    ) -> Result<DelayTime, SpeedWeightError> {
        let mut required = DelayTime::new(POS_LARGE, POS_LARGE);
        for fanout in &self.node(node)?.fanouts {
            let fanout_node = self.node(fanout.node)?;
            let current = if let Some(side_required) = duplicated_required.get(&fanout.node) {
                self.compute_input_required_time(
                    fanout.node,
                    fanout.pin,
                    side_required.unwrap_or(fanout_node.required),
                )?
            } else if !collapse_table.contains(&fanout.node) {
                self.compute_input_required_time(fanout.node, fanout.pin, fanout_node.required)?
            } else {
                continue;
            };
            required.rise = required.rise.min(current.rise);
            required.fall = required.fall.min(current.fall);
        }
        Ok(required)
    }

    pub fn compute_input_required_time(
        &self,
        fanout: WeightNodeId,
        pin: usize,
        fanout_required_time: DelayTime,
    ) -> Result<DelayTime, SpeedWeightError> {
        let node = self.node(fanout)?;
        if node.kind == NodeKind::PrimaryOutput {
            return Ok(fanout_required_time);
        }
        let pin_timing = node
            .pins
            .get(pin)
            .ok_or(SpeedWeightError::BadPin { node: fanout, pin })?;
        Ok(compute_input_required_time(
            node.kind,
            pin_timing.phase,
            pin_timing.delay,
            fanout_required_time,
        ))
    }

    fn weight_bfs(
        &self,
        node: WeightNodeId,
        distance: i32,
        output_arrival_time: f64,
    ) -> Result<WeightTraversal, SpeedWeightError> {
        let mut table = HashSet::new();
        let mut collapse_nodes = vec![node];
        let mut first = 0;
        let mut more_to_come = true;
        let mut stats = WeightStats::default();
        table.insert(node);
        for layer in (1..=distance).rev() {
            if !more_to_come {
                break;
            }
            more_to_come = false;
            let last = collapse_nodes.len();
            for index in first..last {
                for fanin in self.node(collapse_nodes[index])?.fanins.iter().copied() {
                    let fanin_node = self.node(fanin)?;
                    if fanin_node.kind == NodeKind::PrimaryInput {
                        if table.insert(fanin) {
                            stats.push(self.weight_point_for_node(fanin, output_arrival_time)?);
                        }
                    } else if !table.contains(&fanin) {
                        if fanin_node.critical {
                            table.insert(fanin);
                            collapse_nodes.push(fanin);
                            more_to_come = true;
                        } else {
                            table.insert(fanin);
                            stats.push(self.weight_point_for_node(fanin, output_arrival_time)?);
                        }
                    }
                }
            }
            first = last;
            if layer == 1 {
                break;
            }
        }
        for current in collapse_nodes.iter().skip(first).copied() {
            for fanin in self.node(current)?.fanins.iter().copied() {
                if table.insert(fanin) {
                    stats.push(self.weight_point_for_node(fanin, output_arrival_time)?);
                }
            }
        }
        Ok(WeightTraversal {
            collapse_nodes,
            table,
            points: stats.points,
        })
    }

    fn mark_input_cone(
        &self,
        node: WeightNodeId,
        area_table: &mut HashMap<WeightNodeId, Option<DelayTime>>,
        collapse_table: &HashSet<WeightNodeId>,
        model: DelayModel,
    ) -> Result<(), SpeedWeightError> {
        let mut queue = VecDeque::new();
        queue.push_back(node);
        while let Some(current) = queue.pop_front() {
            for fanin in self.node(current)?.fanins.iter().copied() {
                if collapse_table.contains(&fanin) && !area_table.contains_key(&fanin) {
                    area_table.insert(fanin, None);
                    queue.push_back(fanin);
                }
            }
        }
        let required = if model == DelayModel::Unknown {
            None
        } else {
            Some(self.compute_side_required_time(node, collapse_table, area_table)?)
        };
        area_table.insert(node, required);
        Ok(())
    }

    fn weight_point_for_node(
        &self,
        node: WeightNodeId,
        output_arrival_time: f64,
    ) -> Result<WeightPoint, SpeedWeightError> {
        let node = self.node(node)?;
        Ok(weight_point(
            TimingSample::new(node.arrival, node.slack, node.critical),
            output_arrival_time,
        ))
    }

    fn area_for_node(&self, node: WeightNodeId) -> Result<f64, SpeedWeightError> {
        let node = self.node(node)?;
        if node.kind == NodeKind::Internal {
            Ok(node.gate_area.unwrap_or(node.literal_count as f64))
        } else {
            Ok(0.0)
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct WeightComputation {
    pub accepted: bool,
    pub timing_cost: f64,
    pub area_cost: i32,
    pub transfer_factor: f64,
    pub standard_deviation: f64,
    pub angle_radians: f64,
    pub critical_fraction: f64,
    pub points: Vec<WeightPoint>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AreaNode {
    pub kind: NodeKind,
    pub gate_area: Option<f64>,
    pub literal_count: usize,
}

impl AreaNode {
    pub const fn internal(gate_area: Option<f64>, literal_count: usize) -> Self {
        Self {
            kind: NodeKind::Internal,
            gate_area,
            literal_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedWeightError {
    MissingSisGraphPorts {},
    MismatchedPointCount { arrivals: usize, delays: usize },
    MismatchedPinCount { fanins: usize, pins: usize },
    InvalidDistance(i32),
    UnknownNode(WeightNodeId),
    BadPin { node: WeightNodeId, pin: usize },
}

impl fmt::Display for SpeedWeightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisGraphPorts {} => write!(
                f,
                "SIS speed weight graph traversal is blocked by unported SIS dependencies"
            ),
            Self::MismatchedPointCount { arrivals, delays } => write!(
                f,
                "arrival and delay point arrays differ in length: {arrivals} != {delays}"
            ),
            Self::MismatchedPinCount { fanins, pins } => {
                write!(
                    f,
                    "fanin and pin timing arrays differ in length: {fanins} != {pins}"
                )
            }
            Self::InvalidDistance(distance) => {
                write!(f, "speed weight distance must be positive, got {distance}")
            }
            Self::UnknownNode(node) => write!(f, "unknown speed weight node {:?}", node),
            Self::BadPin { node, pin } => {
                write!(f, "bad speed weight pin {pin} for node {:?}", node)
            }
        }
    }
}

impl Error for SpeedWeightError {}

pub fn speed_weight_for_sis_node() -> Result<WeightComputation, SpeedWeightError> {
    Err(SpeedWeightError::MissingSisGraphPorts {})
}

pub fn compute_duplicated_area_from_sis_network() -> Result<f64, SpeedWeightError> {
    Err(SpeedWeightError::MissingSisGraphPorts {})
}

pub fn compute_side_required_time_from_sis_network() -> Result<DelayTime, SpeedWeightError> {
    Err(SpeedWeightError::MissingSisGraphPorts {})
}

pub fn compute_weight_from_samples(
    samples: &[TimingSample],
    output_arrival: DelayTime,
    distance: i32,
    duplicated_area: f64,
) -> Result<WeightComputation, SpeedWeightError> {
    if distance <= 0 {
        return Err(SpeedWeightError::InvalidDistance(distance));
    }

    let mut stats = WeightStats::default();

    for sample in samples {
        stats.push(weight_point(*sample, output_arrival.max_edge()));
    }

    finish_weight_computation(stats, distance, duplicated_area)
}

fn finish_weight_computation(
    stats: WeightStats,
    distance: i32,
    duplicated_area: f64,
) -> Result<WeightComputation, SpeedWeightError> {
    let (transfer_factor, standard_deviation, angle_radians) =
        timing_transfer_factor(&stats.points, distance)?;
    let critical_fraction = if stats.points.is_empty() {
        1.0
    } else {
        stats.critical_count as f64 / stats.points.len() as f64
    };

    let (accepted, timing_cost) = if transfer_factor <= 0.0 {
        (false, POS_LARGE)
    } else {
        (true, (MAXWEIGHT * critical_fraction) / transfer_factor)
    };

    Ok(WeightComputation {
        accepted,
        timing_cost,
        area_cost: duplicated_area.ceil() as i32,
        transfer_factor,
        standard_deviation,
        angle_radians,
        critical_fraction,
        points: stats.points,
    })
}

pub fn weight_point(sample: TimingSample, output_arrival_time: f64) -> WeightPoint {
    let arrival_time = sample.arrival.min_edge();
    let slack_time = sample.slack.min_edge();
    let delay_time = output_arrival_time - slack_time - arrival_time;

    WeightPoint {
        arrival_time,
        delay_time,
        critical: sample.critical,
    }
}

pub fn timing_transfer_factor(
    points: &[WeightPoint],
    distance: i32,
) -> Result<(f64, f64, f64), SpeedWeightError> {
    if distance <= 0 {
        return Err(SpeedWeightError::InvalidDistance(distance));
    }

    let n = points.len();
    if n <= 2 {
        return Ok((-1.0, 0.0, -SP_PI));
    }

    let mean_arrival = points.iter().map(|point| point.arrival_time).sum::<f64>() / n as f64;
    let mean_delay = points.iter().map(|point| point.delay_time).sum::<f64>() / n as f64;
    let radius = points
        .iter()
        .map(|point| {
            let arrival_delta = point.arrival_time - mean_arrival;
            let delay_delta = point.delay_time - mean_delay;
            delay_delta.powi(2) + arrival_delta.powi(2)
        })
        .sum::<f64>();
    let standard_deviation = radius.sqrt() / n as f64;

    let min_arrival = points
        .iter()
        .map(|point| point.arrival_time)
        .fold(POS_LARGE, f64::min);
    let max_arrival = points
        .iter()
        .map(|point| point.arrival_time)
        .fold(NEG_LARGE, f64::max);
    let min_delay = points
        .iter()
        .map(|point| point.delay_time)
        .fold(POS_LARGE, f64::min);

    let angle = if min_arrival == max_arrival {
        SP_PI_2
    } else {
        let arrivals = points
            .iter()
            .map(|point| point.arrival_time)
            .collect::<Vec<_>>();
        let delays = points
            .iter()
            .map(|point| point.delay_time)
            .collect::<Vec<_>>();
        linfit(&arrivals, &delays, min_arrival, min_delay)?.atan()
    };

    let mut coefficient = if angle > SP_PI_4 {
        4.0 - (4.0 * angle * SP_1_PI)
    } else {
        2.0 + (4.0 * angle * SP_1_PI)
    };
    coefficient = coefficient.max(0.3);

    let transfer_factor = coefficient * (standard_deviation / distance as f64).powi(2);
    Ok((transfer_factor, standard_deviation, angle))
}

pub fn linfit(
    arrivals: &[f64],
    delays: &[f64],
    min_arrival: f64,
    min_delay: f64,
) -> Result<f64, SpeedWeightError> {
    if arrivals.len() != delays.len() {
        return Err(SpeedWeightError::MismatchedPointCount {
            arrivals: arrivals.len(),
            delays: delays.len(),
        });
    }

    let mut x = arrivals
        .iter()
        .map(|value| *value - min_arrival)
        .collect::<Vec<_>>();
    let mut y = delays
        .iter()
        .map(|value| *value - min_delay)
        .collect::<Vec<_>>();

    sort2(&mut y, &mut x)?;
    rank(&mut y);
    sort2(&mut x, &mut y)?;
    rank(&mut x);

    let n = x.len() as f64;
    let sum_x = x.iter().sum::<f64>();
    let sum_y = y.iter().sum::<f64>();
    let sum_xx = x.iter().map(|value| value * value).sum::<f64>();
    let sum_xy = x
        .iter()
        .zip(&y)
        .map(|(left, right)| left * right)
        .sum::<f64>();
    let x_dev = (sum_xx / n) - ((sum_x * sum_x) / (n * n));

    Ok(((sum_xy / n) - ((sum_x * sum_y) / (n * n))) / x_dev.sqrt())
}

pub fn sort2(primary: &mut [f64], secondary: &mut [f64]) -> Result<(), SpeedWeightError> {
    if primary.len() != secondary.len() {
        return Err(SpeedWeightError::MismatchedPointCount {
            arrivals: primary.len(),
            delays: secondary.len(),
        });
    }

    let mut paired = primary
        .iter()
        .copied()
        .zip(secondary.iter().copied())
        .collect::<Vec<_>>();
    paired.sort_by(|left, right| left.0.total_cmp(&right.0));

    for (index, (first, second)) in paired.into_iter().enumerate() {
        primary[index] = first;
        secondary[index] = second;
    }

    Ok(())
}

pub fn rank(values: &mut [f64]) {
    if values.is_empty() {
        return;
    }

    let mut index = 0;
    while index < values.len() {
        let mut next = index + 1;
        while next < values.len() && values[next] == values[index] {
            next += 1;
        }

        if next == index + 1 {
            values[index] = (index + 1) as f64;
        } else {
            let rank = 0.5 * ((index + 1) as f64 + next as f64);
            for value in &mut values[index..next] {
                *value = rank;
            }
        }

        index = next;
    }
}

pub fn duplicated_area(nodes: &[AreaNode]) -> f64 {
    nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Internal)
        .map(|node| node.gate_area.unwrap_or(node.literal_count as f64))
        .sum()
}

pub fn compute_input_required_time(
    fanout_kind: NodeKind,
    phase: InputPhase,
    pin_delay: DelayTime,
    fanout_required_time: DelayTime,
) -> DelayTime {
    if fanout_kind == NodeKind::PrimaryOutput {
        return fanout_required_time;
    }

    match phase {
        InputPhase::PositiveUnate => DelayTime {
            rise: fanout_required_time.rise - pin_delay.rise,
            fall: fanout_required_time.fall - pin_delay.fall,
        },
        InputPhase::NegativeUnate => DelayTime {
            rise: fanout_required_time.fall - pin_delay.fall,
            fall: fanout_required_time.rise - pin_delay.rise,
        },
        InputPhase::Binate => {
            let delay = pin_delay.rise.max(pin_delay.fall);
            DelayTime {
                rise: fanout_required_time.rise - delay,
                fall: fanout_required_time.fall - delay,
            }
        }
        InputPhase::Unknown => fanout_required_time,
    }
}

#[derive(Default)]
struct WeightStats {
    points: Vec<WeightPoint>,
    critical_count: usize,
}

impl WeightStats {
    fn push(&mut self, point: WeightPoint) {
        if point.critical {
            self.critical_count += 1;
        }
        self.points.push(point);
    }
}

struct WeightTraversal {
    collapse_nodes: Vec<WeightNodeId>,
    table: HashSet<WeightNodeId>,
    points: Vec<WeightPoint>,
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

    fn graph_node(kind: NodeKind, arrival: f64, slack: f64, critical: bool) -> SpeedWeightNode {
        SpeedWeightNode::new(kind).with_timing(
            DelayTime::new(arrival, arrival),
            DelayTime::new(slack, slack),
            DelayTime::new(20.0, 20.0),
            critical,
        )
    }

    fn pin() -> PinTiming {
        PinTiming::new(InputPhase::PositiveUnate, DelayTime::new(0.0, 0.0))
    }

    #[test]
    fn graph_speed_weight_traverses_critical_fanin_cone_and_side_area() {
        let mut graph = SpeedWeightGraph::new();
        let a = graph
            .add_node(graph_node(NodeKind::PrimaryInput, 1.0, 1.0, true))
            .unwrap();
        let b = graph
            .add_node(graph_node(NodeKind::PrimaryInput, 2.0, 1.0, false))
            .unwrap();
        let c = graph
            .add_node(graph_node(NodeKind::PrimaryInput, 3.0, 2.0, true))
            .unwrap();
        let n1 = graph
            .add_node(
                graph_node(NodeKind::Internal, 4.0, 1.0, true)
                    .with_fanins(vec![a, b], vec![pin(), pin()])
                    .with_area(Some(2.5), 9),
            )
            .unwrap();
        let _side = graph
            .add_node(graph_node(NodeKind::Internal, 6.0, 1.0, false).with_fanins(
                vec![n1],
                vec![PinTiming::new(
                    InputPhase::PositiveUnate,
                    DelayTime::new(1.0, 1.0),
                )],
            ))
            .unwrap();
        let root = graph
            .add_node(
                graph_node(NodeKind::Internal, 10.0, 0.0, true)
                    .with_fanins(vec![n1, c], vec![pin(), pin()])
                    .with_area(None, 5),
            )
            .unwrap();

        let result = graph.speed_weight(root, 2).unwrap();

        assert!(result.accepted);
        assert_eq!(result.area_cost, 3);
        assert_eq!(result.points.len(), 3);
        assert_close(result.critical_fraction, 2.0 / 3.0);
        assert!(result.points.contains(&WeightPoint {
            arrival_time: 1.0,
            delay_time: 8.0,
            critical: true,
        }));
    }

    #[test]
    fn graph_side_required_time_uses_fanout_pin_phase() {
        let mut graph = SpeedWeightGraph::new();
        let input = graph
            .add_node(graph_node(NodeKind::PrimaryInput, 0.0, 0.0, false))
            .unwrap();
        let fanout = graph
            .add_node(
                graph_node(NodeKind::Internal, 0.0, 0.0, false)
                    .with_fanins(
                        vec![input],
                        vec![PinTiming::new(
                            InputPhase::NegativeUnate,
                            DelayTime::new(1.0, 2.0),
                        )],
                    )
                    .with_timing(
                        DelayTime::new(0.0, 0.0),
                        DelayTime::new(0.0, 0.0),
                        DelayTime::new(10.0, 12.0),
                        false,
                    ),
            )
            .unwrap();
        let collapse = HashSet::new();
        let duplicated = HashMap::new();

        assert_eq!(
            graph
                .compute_side_required_time(input, &collapse, &duplicated)
                .unwrap(),
            DelayTime::new(10.0, 9.0)
        );
        assert_eq!(
            graph.compute_input_required_time(fanout, 9, DelayTime::new(1.0, 1.0)),
            Err(SpeedWeightError::BadPin {
                node: fanout,
                pin: 9
            })
        );
    }
    #[test]
    fn weight_point_uses_min_arrival_and_slack_edges() {
        let point = weight_point(
            TimingSample::new(DelayTime::new(3.0, 5.0), DelayTime::new(1.5, 2.0), true),
            12.0,
        );

        assert_eq!(
            point,
            WeightPoint {
                arrival_time: 3.0,
                delay_time: 7.5,
                critical: true,
            }
        );
    }

    #[test]
    fn rank_matches_c_tie_average_behavior() {
        let mut values = vec![1.0, 2.0, 2.0, 4.0, 4.0, 4.0];
        rank(&mut values);

        assert_eq!(values, vec![1.0, 2.5, 2.5, 5.0, 5.0, 5.0]);
    }

    #[test]
    fn sort2_keeps_secondary_values_with_primary_order() {
        let mut primary = vec![3.0, 1.0, 2.0];
        let mut secondary = vec![30.0, 10.0, 20.0];
        sort2(&mut primary, &mut secondary).unwrap();

        assert_eq!(primary, vec![1.0, 2.0, 3.0]);
        assert_eq!(secondary, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn linfit_uses_ranked_least_squares_like_weight_c() {
        let slope = linfit(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0], 1.0, 2.0).unwrap();

        assert_close(slope, (2.0_f64 / 3.0).sqrt());
    }

    #[test]
    fn transfer_factor_rejects_two_or_fewer_points() {
        let points = [
            WeightPoint {
                arrival_time: 1.0,
                delay_time: 2.0,
                critical: false,
            },
            WeightPoint {
                arrival_time: 2.0,
                delay_time: 3.0,
                critical: true,
            },
        ];

        let (transfer_factor, standard_deviation, angle) =
            timing_transfer_factor(&points, 3).unwrap();

        assert_eq!(transfer_factor, -1.0);
        assert_eq!(standard_deviation, 0.0);
        assert_close(angle, -SP_PI);
    }

    #[test]
    fn transfer_factor_uses_vertical_angle_when_arrivals_are_equal() {
        let points = [
            WeightPoint {
                arrival_time: 2.0,
                delay_time: 1.0,
                critical: false,
            },
            WeightPoint {
                arrival_time: 2.0,
                delay_time: 3.0,
                critical: false,
            },
            WeightPoint {
                arrival_time: 2.0,
                delay_time: 5.0,
                critical: true,
            },
        ];

        let (transfer_factor, standard_deviation, angle) =
            timing_transfer_factor(&points, 2).unwrap();

        assert_close(angle, SP_PI_2);
        assert_close(standard_deviation, (8.0_f64).sqrt() / 3.0);
        assert_close(transfer_factor, 2.0 * (standard_deviation / 2.0).powi(2));
    }

    #[test]
    fn compute_weight_applies_critical_fraction_area_and_threshold() {
        let samples = [
            TimingSample::new(DelayTime::new(1.0, 1.5), DelayTime::new(1.0, 2.0), true),
            TimingSample::new(DelayTime::new(2.0, 2.5), DelayTime::new(1.5, 2.0), false),
            TimingSample::new(DelayTime::new(3.0, 3.5), DelayTime::new(2.0, 3.0), true),
        ];

        let result = compute_weight_from_samples(&samples, DelayTime::new(8.0, 10.0), 2, 3.1)
            .expect("sample-only weight should compute");

        assert!(result.accepted);
        assert_eq!(result.area_cost, 4);
        assert_close(result.critical_fraction, 2.0 / 3.0);
        assert_close(
            result.timing_cost,
            MAXWEIGHT * result.critical_fraction / result.transfer_factor,
        );
    }

    #[test]
    fn compute_weight_rejects_non_positive_transfer_factor() {
        let samples = [
            TimingSample::new(DelayTime::new(1.0, 1.0), DelayTime::new(1.0, 1.0), false),
            TimingSample::new(DelayTime::new(2.0, 2.0), DelayTime::new(1.0, 1.0), false),
        ];

        let result = compute_weight_from_samples(&samples, DelayTime::new(5.0, 5.0), 3, 0.0)
            .expect("two samples should produce the C rejection path");

        assert!(!result.accepted);
        assert_eq!(result.timing_cost, POS_LARGE);
        assert_eq!(result.transfer_factor, -1.0);
    }

    #[test]
    fn duplicated_area_prefers_gate_area_and_ignores_non_internal_nodes() {
        let area = duplicated_area(&[
            AreaNode {
                kind: NodeKind::PrimaryInput,
                gate_area: Some(50.0),
                literal_count: 50,
            },
            AreaNode::internal(Some(2.5), 9),
            AreaNode::internal(None, 4),
        ]);

        assert_eq!(area, 6.5);
    }

    #[test]
    fn input_required_time_matches_phase_specific_c_cases() {
        let required = DelayTime::new(10.0, 12.0);
        let delay = DelayTime::new(1.5, 2.5);

        assert_eq!(
            compute_input_required_time(
                NodeKind::Internal,
                InputPhase::PositiveUnate,
                delay,
                required
            ),
            DelayTime::new(8.5, 9.5)
        );
        assert_eq!(
            compute_input_required_time(
                NodeKind::Internal,
                InputPhase::NegativeUnate,
                delay,
                required
            ),
            DelayTime::new(9.5, 8.5)
        );
        assert_eq!(
            compute_input_required_time(NodeKind::Internal, InputPhase::Binate, delay, required),
            DelayTime::new(7.5, 9.5)
        );
        assert_eq!(
            compute_input_required_time(NodeKind::Internal, InputPhase::Unknown, delay, required),
            required
        );
        assert_eq!(
            compute_input_required_time(
                NodeKind::PrimaryOutput,
                InputPhase::PositiveUnate,
                delay,
                required,
            ),
            required
        );
    }

    #[test]
    fn graph_bound_entry_points_report_missing_ports() {
        assert_eq!(
            speed_weight_for_sis_node(),
            Err(SpeedWeightError::MissingSisGraphPorts {})
        );
        assert_eq!(
            compute_duplicated_area_from_sis_network(),
            Err(SpeedWeightError::MissingSisGraphPorts {})
        );
        assert_eq!(
            compute_side_required_time_from_sis_network(),
            Err(SpeedWeightError::MissingSisGraphPorts {})
        );
    }
}
