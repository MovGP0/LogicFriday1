//! Native Rust model for `LogicSynthesis/sis/speed/weight_util.c`.
//!
//! The original routine builds a SIS pointer table by traversing a network,
//! asking other speed-package routines for criticality, critical levels, and
//! raw delay/area weight costs. Those SIS graph and delay services are still
//! separate porting beads, so this module exposes the feasible aggregation
//! logic over native Rust records and reports the missing SIS-backed path as an
//! explicit error.

use std::error::Error;
use std::fmt;

pub const MAXWEIGHT: i32 = 1_000;
pub const AVOID_WEIGHT: i32 = MAXWEIGHT * 100;
pub const DEFAULT_SPEED_COEFF: f64 = 0.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeedParameters {
    pub coeff: f64,
}

impl Default for SpeedParameters {
    fn default() -> Self {
        Self {
            coeff: DEFAULT_SPEED_COEFF,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawWeightCost {
    pub time: f64,
    pub area: i32,
}

impl RawWeightCost {
    pub fn new(time: f64, area: i32) -> Self {
        Self { time, area }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeightNode<N> {
    pub id: N,
    pub kind: SpeedNodeKind,
    pub critical: bool,
    pub critical_level: Option<usize>,
    pub raw_cost: Option<RawWeightCost>,
}

impl<N> WeightNode<N> {
    pub fn new(
        id: N,
        kind: SpeedNodeKind,
        critical: bool,
        critical_level: Option<usize>,
        raw_cost: Option<RawWeightCost>,
    ) -> Self {
        Self {
            id,
            kind,
            critical,
            critical_level,
            raw_cost,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComputedWeight<N> {
    pub id: N,
    pub weight: i32,
    pub detail: WeightDetail,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WeightDetail {
    AvoidedCriticalNode,
    Aggregated {
        level: usize,
        level_count: usize,
        normalized_time: f64,
        normalized_area: f64,
        combined_weight: f64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedWeightDependency {
    NetworkTraversal,
    NodeData,
    DelayData,
    Criticality,
    CriticalLevelization,
    RawSpeedWeight,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpeedWeightError {
    NonFiniteCoefficient(f64),
    NonFiniteRawTime { node_index: usize, time: f64 },
    MissingCriticalLevel { node_index: usize },
    ZeroCriticalSignalsAtLevel { level: usize },
    ComputedWeightOutOfRange { node_index: usize, value: f64 },
    MissingDependency(SpeedWeightDependency),
}

impl fmt::Display for SpeedWeightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCoefficient(coeff) => {
                write!(f, "speed weight coefficient is not finite: {coeff}")
            }
            Self::NonFiniteRawTime { node_index, time } => {
                write!(
                    f,
                    "raw time weight for node index {node_index} is not finite: {time}"
                )
            }
            Self::MissingCriticalLevel { node_index } => write!(
                f,
                "critical internal node at index {node_index} has no critical level"
            ),
            Self::ZeroCriticalSignalsAtLevel { level } => {
                write!(f, "critical level {level} has no critical signal count")
            }
            Self::ComputedWeightOutOfRange { node_index, value } => write!(
                f,
                "computed weight for node index {node_index} is outside i32 range: {value}"
            ),
            Self::MissingDependency(dependency) => match dependency {
                SpeedWeightDependency::NetworkTraversal => {
                    write!(f, "SIS network traversal APIs are not ported to Rust yet")
                }
                SpeedWeightDependency::NodeData => {
                    write!(f, "SIS node type/function APIs are not ported to Rust yet")
                }
                SpeedWeightDependency::DelayData => {
                    write!(
                        f,
                        "SIS delay arrival/slack/required APIs are not ported to Rust yet"
                    )
                }
                SpeedWeightDependency::Criticality => {
                    write!(f, "SIS speed_critical is not ported to Rust yet")
                }
                SpeedWeightDependency::CriticalLevelization => {
                    write!(f, "SIS speed_levelize_crit is not ported to Rust yet")
                }
                SpeedWeightDependency::RawSpeedWeight => {
                    write!(f, "SIS speed_weight is not ported to Rust yet")
                }
            },
        }
    }
}

impl Error for SpeedWeightError {}

pub fn critical_level_counts<N>(nodes: &[WeightNode<N>]) -> Vec<usize> {
    let max_level = nodes.iter().filter_map(|node| node.critical_level).max();
    let Some(max_level) = max_level else {
        return Vec::new();
    };

    let mut counts = vec![0; max_level + 1];
    for node in nodes {
        if node.kind == SpeedNodeKind::Internal {
            if let Some(level) = node.critical_level {
                counts[level] += 1;
            }
        }
    }
    counts
}

pub fn compute_weight_table<N: Clone>(
    nodes: &[WeightNode<N>],
    speed_param: SpeedParameters,
) -> Result<Vec<ComputedWeight<N>>, SpeedWeightError> {
    if !speed_param.coeff.is_finite() {
        return Err(SpeedWeightError::NonFiniteCoefficient(speed_param.coeff));
    }

    let level_counts = critical_level_counts(nodes);
    let mut weighted = Vec::new();
    let mut min_time = f64::INFINITY;
    let mut max_time = f64::NEG_INFINITY;
    let mut min_area = i32::MAX;
    let mut max_area = i32::MIN;

    for (index, node) in nodes.iter().enumerate() {
        if !is_weighted_node(node) {
            continue;
        }

        if node.critical_level.is_none() {
            return Err(SpeedWeightError::MissingCriticalLevel { node_index: index });
        }

        if let Some(raw_cost) = node.raw_cost {
            if !raw_cost.time.is_finite() {
                return Err(SpeedWeightError::NonFiniteRawTime {
                    node_index: index,
                    time: raw_cost.time,
                });
            }

            min_time = min_time.min(raw_cost.time);
            max_time = max_time.max(raw_cost.time);
            min_area = min_area.min(raw_cost.area);
            max_area = max_area.max(raw_cost.area);
            weighted.push((index, raw_cost));
        }
    }

    let time_range = c_range(max_time - min_time);
    let area_range = c_range((max_area as f64) - (min_area as f64));
    let mut result = Vec::new();

    for (index, node) in nodes.iter().enumerate() {
        if !is_weighted_node(node) {
            continue;
        }

        let Some(raw_cost) = node.raw_cost else {
            result.push(ComputedWeight {
                id: node.id.clone(),
                weight: AVOID_WEIGHT,
                detail: WeightDetail::AvoidedCriticalNode,
            });
            continue;
        };

        let level = node
            .critical_level
            .ok_or(SpeedWeightError::MissingCriticalLevel { node_index: index })?;
        let level_count = level_counts
            .get(level)
            .copied()
            .filter(|count| *count > 0)
            .ok_or(SpeedWeightError::ZeroCriticalSignalsAtLevel { level })?;

        let normalized_time = (MAXWEIGHT as f64 * (raw_cost.time - min_time) / time_range) + 1.0;
        let normalized_area = MAXWEIGHT as f64 * ((raw_cost.area - min_area) as f64) / area_range;
        let combined_weight = normalized_time + speed_param.coeff * normalized_area;
        let scaled_weight = (combined_weight / level_count as f64).ceil();
        let weight = checked_i32_weight(index, scaled_weight)?;

        result.push(ComputedWeight {
            id: node.id.clone(),
            weight,
            detail: WeightDetail::Aggregated {
                level,
                level_count,
                normalized_time,
                normalized_area,
                combined_weight,
            },
        });
    }

    debug_assert_eq!(weighted.len(), count_weighted_costs(nodes));
    Ok(result)
}

pub fn compute_weight_from_unported_sis_network<N>()
-> Result<Vec<ComputedWeight<N>>, SpeedWeightError> {
    Err(SpeedWeightError::MissingDependency(
        SpeedWeightDependency::NetworkTraversal,
    ))
}

fn is_weighted_node<N>(node: &WeightNode<N>) -> bool {
    node.kind == SpeedNodeKind::Internal && node.critical
}

fn c_range(range: f64) -> f64 {
    if range == 0.0 { 1.0 } else { range }
}

fn checked_i32_weight(node_index: usize, value: f64) -> Result<i32, SpeedWeightError> {
    if value < i32::MIN as f64 || value > i32::MAX as f64 || !value.is_finite() {
        return Err(SpeedWeightError::ComputedWeightOutOfRange { node_index, value });
    }
    Ok(value as i32)
}

fn count_weighted_costs<N>(nodes: &[WeightNode<N>]) -> usize {
    nodes
        .iter()
        .filter(|node| is_weighted_node(node) && node.raw_cost.is_some())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn internal(
        id: &'static str,
        critical: bool,
        level: Option<usize>,
        raw_cost: Option<RawWeightCost>,
    ) -> WeightNode<&'static str> {
        WeightNode::new(id, SpeedNodeKind::Internal, critical, level, raw_cost)
    }

    #[test]
    fn constants_match_speed_int_header() {
        assert_eq!(MAXWEIGHT, 1_000);
        assert_eq!(AVOID_WEIGHT, 100_000);
        assert_eq!(DEFAULT_SPEED_COEFF, 0.0);
    }

    #[test]
    fn counts_critical_internal_nodes_by_level() {
        let nodes = [
            internal("a", true, Some(0), Some(RawWeightCost::new(1.0, 1))),
            internal("b", true, Some(2), Some(RawWeightCost::new(1.0, 1))),
            WeightNode::new("po", SpeedNodeKind::PrimaryOutput, true, Some(2), None),
            internal("c", false, None, None),
        ];

        assert_eq!(critical_level_counts(&nodes), vec![1, 0, 1]);
    }

    #[test]
    fn aggregates_time_and_area_weights_using_c_formula() {
        let nodes = [
            internal("a", true, Some(0), Some(RawWeightCost::new(10.0, 1))),
            internal("b", true, Some(0), Some(RawWeightCost::new(20.0, 3))),
            internal("c", true, Some(1), Some(RawWeightCost::new(15.0, 2))),
        ];

        let weights = compute_weight_table(&nodes, SpeedParameters { coeff: 0.5 }).unwrap();

        assert_eq!(
            weights,
            vec![
                ComputedWeight {
                    id: "a",
                    weight: 1,
                    detail: WeightDetail::Aggregated {
                        level: 0,
                        level_count: 2,
                        normalized_time: 1.0,
                        normalized_area: 0.0,
                        combined_weight: 1.0,
                    },
                },
                ComputedWeight {
                    id: "b",
                    weight: 751,
                    detail: WeightDetail::Aggregated {
                        level: 0,
                        level_count: 2,
                        normalized_time: 1001.0,
                        normalized_area: 1000.0,
                        combined_weight: 1501.0,
                    },
                },
                ComputedWeight {
                    id: "c",
                    weight: 751,
                    detail: WeightDetail::Aggregated {
                        level: 1,
                        level_count: 1,
                        normalized_time: 501.0,
                        normalized_area: 500.0,
                        combined_weight: 751.0,
                    },
                },
            ]
        );
    }

    #[test]
    fn zero_time_and_area_ranges_match_c_fallback_range() {
        let nodes = [
            internal("a", true, Some(0), Some(RawWeightCost::new(7.0, 4))),
            internal("b", true, Some(0), Some(RawWeightCost::new(7.0, 4))),
        ];

        let weights = compute_weight_table(&nodes, SpeedParameters { coeff: 100.0 }).unwrap();

        assert_eq!(weights[0].weight, 1);
        assert_eq!(weights[1].weight, 1);
        assert_eq!(
            weights[0].detail,
            WeightDetail::Aggregated {
                level: 0,
                level_count: 2,
                normalized_time: 1.0,
                normalized_area: 0.0,
                combined_weight: 1.0,
            }
        );
    }

    #[test]
    fn failed_raw_weight_uses_avoidance_sentinel() {
        let nodes = [
            internal("avoid", true, Some(0), None),
            internal("ok", true, Some(0), Some(RawWeightCost::new(3.0, 1))),
            internal("noncrit", false, None, Some(RawWeightCost::new(9.0, 9))),
        ];

        let weights = compute_weight_table(&nodes, SpeedParameters::default()).unwrap();

        assert_eq!(
            weights,
            vec![
                ComputedWeight {
                    id: "avoid",
                    weight: AVOID_WEIGHT,
                    detail: WeightDetail::AvoidedCriticalNode,
                },
                ComputedWeight {
                    id: "ok",
                    weight: 1,
                    detail: WeightDetail::Aggregated {
                        level: 0,
                        level_count: 2,
                        normalized_time: 1.0,
                        normalized_area: 0.0,
                        combined_weight: 1.0,
                    },
                },
            ]
        );
    }

    #[test]
    fn reports_missing_levels_and_non_finite_inputs() {
        let missing_level = [internal("a", true, None, Some(RawWeightCost::new(1.0, 1)))];
        assert_eq!(
            compute_weight_table(&missing_level, SpeedParameters::default()),
            Err(SpeedWeightError::MissingCriticalLevel { node_index: 0 })
        );

        let non_finite = [internal(
            "a",
            true,
            Some(0),
            Some(RawWeightCost::new(f64::INFINITY, 1)),
        )];
        assert_eq!(
            compute_weight_table(&non_finite, SpeedParameters::default()),
            Err(SpeedWeightError::NonFiniteRawTime {
                node_index: 0,
                time: f64::INFINITY,
            })
        );

        assert_eq!(
            compute_weight_table::<&str>(
                &[],
                SpeedParameters {
                    coeff: f64::INFINITY
                }
            ),
            Err(SpeedWeightError::NonFiniteCoefficient(f64::INFINITY))
        );
    }

    #[test]
    fn sis_backed_path_is_blocked_on_explicit_dependencies() {
        assert_eq!(
            compute_weight_from_unported_sis_network::<&str>(),
            Err(SpeedWeightError::MissingDependency(
                SpeedWeightDependency::NetworkTraversal
            ))
        );
    }
}
