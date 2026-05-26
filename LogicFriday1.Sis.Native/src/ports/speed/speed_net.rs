//! Native Rust decision model for `sis/speed/speed_net.c`.
//!
//! The original routine enumerates SIS kernels, scores each kernel/cokernel
//! divisor, mutates a `network_t`, and recursively decomposes the affected
//! nodes. The native SIS network and node APIs are not available yet, so this
//! module ports the deterministic scoring and decomposition planning behavior
//! into Rust data models and leaves the mutation-bound entry point as an
//! explicit blocked operation.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;
pub const SCALE_2: f64 = 100_000.0;
pub const SCALE: f64 = 100.0;
pub const CRITICAL_FRACTION: f64 = 0.05;
pub const FUDGE: f64 = 0.0001;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn latest(self) -> f64 {
        self.rise.max(self.fall)
    }

    pub fn earliest(self) -> f64 {
        self.rise.min(self.fall)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeCostInput {
    pub function: NodeFunction,
    pub cube_count: usize,
    pub literal_count: usize,
    pub fanin_arrivals: Vec<DelayTime>,
}

impl NodeCostInput {
    pub fn input_count(&self) -> usize {
        self.fanin_arrivals.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeCost {
    pub delay: f64,
    pub area: f64,
}

pub fn critical_threshold(fanin_arrivals: &[DelayTime], attempt_no: usize) -> f64 {
    let (min_time, max_time) = min_max_arrival(fanin_arrivals);
    if max_time - min_time < 1.0e-3 {
        POS_LARGE
    } else {
        max_time - (attempt_no as f64 * CRITICAL_FRACTION + FUDGE) * (max_time - min_time)
    }
}

pub fn evaluate_node_cost(node: &NodeCostInput, co_node: &NodeCostInput) -> NodeCost {
    if node.input_count() <= 1 {
        return NodeCost {
            delay: POS_LARGE,
            area: POS_LARGE,
        };
    }

    let lit_saving = if co_node.function == NodeFunction::Zero {
        0
    } else {
        let node_cubes = node.cube_count as i64;
        let node_literals = node.literal_count as i64;
        let co_cubes = co_node.cube_count as i64;
        let co_literals = co_node.literal_count as i64;
        node_cubes * co_literals + co_cubes * node_literals
            - (node_literals + co_literals + co_cubes)
    };
    let (min_time, max_time) = min_max_arrival(&node.fanin_arrivals);

    NodeCost {
        delay: 0.9 * max_time + 0.1 * min_time,
        area: lit_saving as f64,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DivisorCandidate<N> {
    pub node: N,
    pub cost: NodeCost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DivisorScale {
    pub min_delay: f64,
    pub max_delay: f64,
    pub delay_scale: f64,
    pub min_area: f64,
    pub max_area: f64,
    pub area_scale: f64,
}

impl DivisorScale {
    pub fn from_candidates<N>(candidates: &[DivisorCandidate<N>]) -> Option<Self> {
        let mut min_delay = f64::INFINITY;
        let mut max_delay = f64::NEG_INFINITY;
        let mut min_area = f64::INFINITY;
        let mut max_area = f64::NEG_INFINITY;
        let mut saw_candidate = false;

        for candidate in candidates {
            if candidate.cost.delay < POS_LARGE || candidate.cost.area < POS_LARGE {
                saw_candidate = true;
                min_delay = min_delay.min(candidate.cost.delay);
                max_delay = max_delay.max(candidate.cost.delay);
                min_area = min_area.min(candidate.cost.area);
                max_area = max_area.max(candidate.cost.area);
            }
        }

        if !saw_candidate {
            return None;
        }

        let delay_range = max_delay - min_delay;
        let area_range = max_area - min_area;
        Some(Self {
            min_delay,
            max_delay,
            delay_scale: SCALE_2 / if delay_range > 0.0 { delay_range } else { 1.0 },
            min_area,
            max_area,
            area_scale: SCALE / if area_range > 0.0 { area_range } else { 1.0 },
        })
    }

    pub fn scaled_cost(self, cost: NodeCost, alpha: f64) -> Option<f64> {
        if cost.delay >= POS_LARGE || cost.area >= POS_LARGE {
            return None;
        }

        Some(if self.min_delay == self.max_delay {
            -cost.area
        } else {
            (cost.delay - self.min_delay) * self.delay_scale
                - alpha * (cost.area - self.min_area) * self.area_scale
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedDivisor<N> {
    pub node: N,
    pub scaled_cost: f64,
    pub raw_cost: NodeCost,
}

pub fn select_best_divisor<N>(
    candidates: impl IntoIterator<Item = DivisorCandidate<N>>,
    alpha: f64,
) -> Option<SelectedDivisor<N>> {
    let candidates: Vec<_> = candidates.into_iter().collect();
    let scale = DivisorScale::from_candidates(&candidates)?;
    let mut best: Option<SelectedDivisor<N>> = None;

    for candidate in candidates {
        let Some(scaled_cost) = scale.scaled_cost(candidate.cost, alpha) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|current| scaled_cost < current.scaled_cost)
        {
            best = Some(SelectedDivisor {
                node: candidate.node,
                scaled_cost,
                raw_cost: candidate.cost,
            });
        }
    }

    best
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeedNetOptions {
    pub coeff: f64,
    pub add_inv: bool,
    pub del_crit_cubes: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedNetworkInput<N> {
    pub fanin_arrivals: Vec<DelayTime>,
    pub attempt_no: usize,
    pub kernel_timeout_occurred: bool,
    pub quick_divisor_exists: bool,
    pub divisor_candidates: Vec<DivisorCandidate<N>>,
    pub options: SpeedNetOptions,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedNetworkPlan<N> {
    pub threshold: Option<f64>,
    pub branch: SpeedNetworkBranch<N>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpeedNetworkBranch<N> {
    ExtractDivisor {
        divisor: SelectedDivisor<N>,
        recursively_decompose_divisor: bool,
        recursively_decompose_remainder: bool,
    },
    AndOrDecompose {
        reason: AndOrReason,
        delete_single_fanin_node: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndOrReason {
    KernelTimeout,
    NoQuickDivisor,
    NoAppropriateDivisor,
}

pub fn plan_speed_decomp_network<N>(input: SpeedNetworkInput<N>) -> SpeedNetworkPlan<N> {
    if input.kernel_timeout_occurred {
        return SpeedNetworkPlan {
            threshold: None,
            branch: SpeedNetworkBranch::AndOrDecompose {
                reason: AndOrReason::KernelTimeout,
                delete_single_fanin_node: !input.options.add_inv,
            },
        };
    }

    if !input.quick_divisor_exists {
        return SpeedNetworkPlan {
            threshold: None,
            branch: SpeedNetworkBranch::AndOrDecompose {
                reason: AndOrReason::NoQuickDivisor,
                delete_single_fanin_node: !input.options.add_inv,
            },
        };
    }

    let threshold = critical_threshold(&input.fanin_arrivals, input.attempt_no);
    let divisor = select_best_divisor(input.divisor_candidates, input.options.coeff);
    let branch = match divisor {
        Some(divisor) => SpeedNetworkBranch::ExtractDivisor {
            divisor,
            recursively_decompose_divisor: true,
            recursively_decompose_remainder: true,
        },
        None => SpeedNetworkBranch::AndOrDecompose {
            reason: AndOrReason::NoAppropriateDivisor,
            delete_single_fanin_node: !input.options.add_inv,
        },
    };

    SpeedNetworkPlan {
        threshold: Some(threshold),
        branch,
    }
}

pub fn speed_decomp_network_bound<Network, Node>(
    _network: &mut Network,
    _node: &mut Node,
    _options: SpeedNetOptions,
    _attempt_no: usize,
) -> Result<(), SpeedNetError> {
    Err(SpeedNetError::MissingNativePorts {
        operation: "speed_decomp_network",
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedNetError {
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for SpeedNetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by unported SIS dependencies")
            }
        }
    }
}

impl Error for SpeedNetError {}

fn min_max_arrival(fanin_arrivals: &[DelayTime]) -> (f64, f64) {
    let mut min_time = f64::INFINITY;
    let mut max_time = f64::NEG_INFINITY;
    for arrival in fanin_arrivals {
        min_time = min_time.min(arrival.earliest());
        max_time = max_time.max(arrival.latest());
    }

    if fanin_arrivals.is_empty() {
        (0.0, 0.0)
    } else {
        (min_time, max_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arrival(rise: f64, fall: f64) -> DelayTime {
        DelayTime { rise, fall }
    }

    fn node(
        function: NodeFunction,
        cube_count: usize,
        literal_count: usize,
        fanin_arrivals: &[DelayTime],
    ) -> NodeCostInput {
        NodeCostInput {
            function,
            cube_count,
            literal_count,
            fanin_arrivals: fanin_arrivals.to_vec(),
        }
    }

    #[test]
    fn threshold_uses_latest_fanin_window_and_attempt_fraction() {
        let threshold = critical_threshold(
            &[arrival(1.0, 2.0), arrival(5.0, 4.0), arrival(3.0, 1.5)],
            2,
        );

        assert!((threshold - 4.5996).abs() < 1.0e-9);
    }

    #[test]
    fn threshold_is_unbounded_when_arrivals_are_effectively_equal() {
        let threshold = critical_threshold(&[arrival(3.0, 3.0004), arrival(3.0002, 3.0)], 1);

        assert_eq!(threshold, POS_LARGE);
    }

    #[test]
    fn node_evaluate_ports_delay_weight_and_literal_saving_formula() {
        let divisor = node(
            NodeFunction::Other,
            3,
            7,
            &[arrival(1.0, 2.0), arrival(4.0, 3.0), arrival(2.0, 1.0)],
        );
        let cokernel = node(NodeFunction::Other, 2, 5, &[arrival(0.0, 0.0)]);

        assert_eq!(
            evaluate_node_cost(&divisor, &cokernel),
            NodeCost {
                delay: 3.7,
                area: 15.0,
            }
        );
    }

    #[test]
    fn node_evaluate_rejects_single_input_divisors_with_large_cost() {
        let divisor = node(NodeFunction::Other, 1, 1, &[arrival(1.0, 2.0)]);
        let cokernel = node(NodeFunction::Other, 2, 3, &[arrival(0.0, 0.0)]);

        assert_eq!(
            evaluate_node_cost(&divisor, &cokernel),
            NodeCost {
                delay: POS_LARGE,
                area: POS_LARGE,
            }
        );
    }

    #[test]
    fn select_best_divisor_keeps_first_minimum_scaled_cost_like_c_loop() {
        let best = select_best_divisor(
            vec![
                DivisorCandidate {
                    node: "slow_area_saver",
                    cost: NodeCost {
                        delay: 7.0,
                        area: 30.0,
                    },
                },
                DivisorCandidate {
                    node: "fast_enough",
                    cost: NodeCost {
                        delay: 5.0,
                        area: 5.0,
                    },
                },
                DivisorCandidate {
                    node: "invalid",
                    cost: NodeCost {
                        delay: POS_LARGE,
                        area: POS_LARGE,
                    },
                },
            ],
            1.0,
        )
        .unwrap();

        assert_eq!(best.node, "fast_enough");
        assert_eq!(
            best.raw_cost,
            NodeCost {
                delay: 5.0,
                area: 5.0,
            }
        );
    }

    #[test]
    fn equal_delay_range_selects_largest_area_saving() {
        let best = select_best_divisor(
            vec![
                DivisorCandidate {
                    node: "small_saving",
                    cost: NodeCost {
                        delay: 5.0,
                        area: 2.0,
                    },
                },
                DivisorCandidate {
                    node: "large_saving",
                    cost: NodeCost {
                        delay: 5.0,
                        area: 6.0,
                    },
                },
            ],
            0.0,
        )
        .unwrap();

        assert_eq!(best.node, "large_saving");
        assert_eq!(best.scaled_cost, -6.0);
    }

    #[test]
    fn plan_extracts_selected_divisor_and_recurses_when_kernel_candidate_is_useful() {
        let plan = plan_speed_decomp_network(SpeedNetworkInput {
            fanin_arrivals: vec![arrival(1.0, 2.0), arrival(4.0, 5.0)],
            attempt_no: 0,
            kernel_timeout_occurred: false,
            quick_divisor_exists: true,
            divisor_candidates: vec![DivisorCandidate {
                node: "k1",
                cost: NodeCost {
                    delay: 4.0,
                    area: 3.0,
                },
            }],
            options: SpeedNetOptions {
                coeff: 0.0,
                add_inv: false,
                del_crit_cubes: true,
            },
        });

        assert_eq!(
            plan.branch,
            SpeedNetworkBranch::ExtractDivisor {
                divisor: SelectedDivisor {
                    node: "k1",
                    scaled_cost: -3.0,
                    raw_cost: NodeCost {
                        delay: 4.0,
                        area: 3.0,
                    },
                },
                recursively_decompose_divisor: true,
                recursively_decompose_remainder: true,
            }
        );
        assert_eq!(plan.threshold, Some(4.9996));
    }

    #[test]
    fn plan_falls_back_to_and_or_and_marks_single_fanin_cleanup_when_no_kernel_exists() {
        let plan = plan_speed_decomp_network::<&'static str>(SpeedNetworkInput {
            fanin_arrivals: vec![],
            attempt_no: 0,
            kernel_timeout_occurred: false,
            quick_divisor_exists: false,
            divisor_candidates: vec![],
            options: SpeedNetOptions {
                coeff: 0.0,
                add_inv: false,
                del_crit_cubes: false,
            },
        });

        assert_eq!(
            plan,
            SpeedNetworkPlan {
                threshold: None,
                branch: SpeedNetworkBranch::AndOrDecompose {
                    reason: AndOrReason::NoQuickDivisor,
                    delete_single_fanin_node: true,
                },
            }
        );
    }

    #[test]
    fn network_bound_entry_point_reports_missing_dependencies() {
        let mut network = ();
        let mut node = ();
        let result = speed_decomp_network_bound(
            &mut network,
            &mut node,
            SpeedNetOptions {
                coeff: 0.0,
                add_inv: true,
                del_crit_cubes: false,
            },
            0,
        );

        assert_eq!(
            result,
            Err(SpeedNetError::MissingNativePorts {
                operation: "speed_decomp_network",
            })
        );
    }
}
