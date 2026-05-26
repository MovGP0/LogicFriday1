//! Native Rust port of the independent math in `sis/speed/weight.c`.
//!
//! The C file combines two separable concerns: numeric weighting of timing
//! samples and SIS graph traversal through `node_t`, `delay_*`, `lib_gate_t`,
//! `array_t`, and `st_table`. The numeric model is ported here as owned Rust
//! data. The graph-bound entry points intentionally return typed dependency
//! errors until the prerequisite native ports exist.

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
    InvalidDistance(i32),
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
            Self::InvalidDistance(distance) => {
                write!(f, "speed weight distance must be positive, got {distance}")
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

    let o_time = output_arrival.max_edge();
    let mut stats = WeightStats::default();

    for sample in samples {
        stats.push(weight_point(*sample, o_time));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "actual {actual} != expected {expected}"
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
