//! Native Rust orchestration for `sis/speed/speed_net.c`.
//!
//! The original routine enumerates SIS kernels, scores each kernel/cokernel
//! divisor, mutates a `network_t`, and recursively decomposes the affected
//! nodes. This module keeps the mutation behavior behind a native Rust backend
//! contract and ports the deterministic scoring and branch selection directly.

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

pub trait SpeedNetBackend {
    type Node: Clone + Eq;
    type Error: Error + Send + Sync + 'static;

    fn kernel_timeout_occurred(&self) -> bool;

    fn quick_divisor_exists(&mut self, node: &Self::Node) -> Result<bool, Self::Error>;

    fn fanin_arrivals(&self, node: &Self::Node) -> Result<Vec<DelayTime>, Self::Error>;

    fn divisor_candidates(
        &mut self,
        node: &Self::Node,
        options: SpeedNetOptions,
        threshold: f64,
    ) -> Result<Vec<DivisorCandidate<Self::Node>>, Self::Error>;

    fn add_node(&mut self, node: Self::Node) -> Result<(), Self::Error>;

    fn substitute_node(
        &mut self,
        node: &Self::Node,
        divisor: &Self::Node,
        complement: bool,
    ) -> Result<bool, Self::Error>;

    fn speed_and_or_decompose(
        &mut self,
        node: &Self::Node,
        options: SpeedNetOptions,
    ) -> Result<bool, Self::Error>;

    fn delete_single_fanin_node(&mut self, node: &Self::Node) -> Result<(), Self::Error>;
}

pub fn speed_decomp_network_bound<B>(
    backend: &mut B,
    node: B::Node,
    options: SpeedNetOptions,
    attempt_no: usize,
) -> Result<(), SpeedNetError<B::Error>>
where
    B: SpeedNetBackend,
{
    let input = if backend.kernel_timeout_occurred() {
        SpeedNetworkInput {
            fanin_arrivals: Vec::new(),
            attempt_no,
            kernel_timeout_occurred: true,
            quick_divisor_exists: false,
            divisor_candidates: Vec::new(),
            options,
        }
    } else if backend
        .quick_divisor_exists(&node)
        .map_err(SpeedNetError::Backend)?
    {
        let fanin_arrivals = backend
            .fanin_arrivals(&node)
            .map_err(SpeedNetError::Backend)?;
        let threshold = critical_threshold(&fanin_arrivals, attempt_no);
        let divisor_candidates = backend
            .divisor_candidates(&node, options, threshold)
            .map_err(SpeedNetError::Backend)?;

        SpeedNetworkInput {
            fanin_arrivals,
            attempt_no,
            kernel_timeout_occurred: false,
            quick_divisor_exists: true,
            divisor_candidates,
            options,
        }
    } else {
        SpeedNetworkInput {
            fanin_arrivals: Vec::new(),
            attempt_no,
            kernel_timeout_occurred: false,
            quick_divisor_exists: false,
            divisor_candidates: Vec::new(),
            options,
        }
    };

    match plan_speed_decomp_network(input).branch {
        SpeedNetworkBranch::ExtractDivisor { divisor, .. } => {
            let divisor_node = divisor.node;
            backend
                .add_node(divisor_node.clone())
                .map_err(SpeedNetError::Backend)?;

            if !backend
                .substitute_node(&node, &divisor_node, false)
                .map_err(SpeedNetError::Backend)?
            {
                return Err(SpeedNetError::SubstituteFailed);
            }

            speed_decomp_network_bound(backend, divisor_node, options, attempt_no)?;
            speed_decomp_network_bound(backend, node, options, attempt_no)
        }
        SpeedNetworkBranch::AndOrDecompose {
            delete_single_fanin_node,
            ..
        } => {
            if !backend
                .speed_and_or_decompose(&node, options)
                .map_err(SpeedNetError::Backend)?
            {
                return Err(SpeedNetError::AndOrDecomposeFailed);
            }

            if delete_single_fanin_node {
                backend
                    .delete_single_fanin_node(&node)
                    .map_err(SpeedNetError::Backend)?;
            }

            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedNetError<E> {
    Backend(E),
    SubstituteFailed,
    AndOrDecomposeFailed,
}

impl<E> fmt::Display for SpeedNetError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(f, "{error}"),
            Self::SubstituteFailed => write!(f, "failed to substitute selected speed divisor"),
            Self::AndOrDecomposeFailed => write!(f, "failed to run AND/OR speed decomposition"),
        }
    }
}

impl<E> Error for SpeedNetError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::SubstituteFailed | Self::AndOrDecomposeFailed => None,
        }
    }
}

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
    use std::collections::{BTreeMap, VecDeque};

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
    fn network_bound_extracts_substitutes_and_recursively_decomposes_divisor_and_remainder() {
        let options = SpeedNetOptions {
            coeff: 0.0,
            add_inv: true,
            del_crit_cubes: false,
        };
        let mut backend = RecordingBackend::new(BTreeMap::from([
            (
                "root",
                NodeScript {
                    quick_divisor_exists: true,
                    fanin_arrivals: vec![arrival(1.0, 2.0), arrival(4.0, 5.0)],
                    divisor_candidates: vec![DivisorCandidate {
                        node: "div",
                        cost: NodeCost {
                            delay: 4.0,
                            area: 3.0,
                        },
                    }],
                },
            ),
            (
                "div",
                NodeScript {
                    quick_divisor_exists: false,
                    fanin_arrivals: Vec::new(),
                    divisor_candidates: Vec::new(),
                },
            ),
        ]));
        backend.root_quick_results = VecDeque::from([true, false]);

        assert_eq!(
            speed_decomp_network_bound(&mut backend, "root", options, 0),
            Ok(())
        );

        assert_eq!(
            backend.events,
            vec![
                Event::QuickDivisor("root"),
                Event::FaninArrivals("root"),
                Event::DivisorCandidates {
                    node: "root",
                    threshold: 4.9996,
                    del_crit_cubes: false,
                },
                Event::AddNode("div"),
                Event::Substitute {
                    node: "root",
                    divisor: "div",
                    complement: false,
                },
                Event::QuickDivisor("div"),
                Event::AndOr {
                    node: "div",
                    add_inv: true,
                },
                Event::QuickDivisor("root"),
                Event::AndOr {
                    node: "root",
                    add_inv: true,
                },
            ]
        );
    }

    #[test]
    fn network_bound_falls_back_to_and_or_and_deletes_single_fanin_when_enabled() {
        let options = SpeedNetOptions {
            coeff: 0.0,
            add_inv: false,
            del_crit_cubes: false,
        };
        let mut backend = RecordingBackend::new(BTreeMap::from([(
            "root",
            NodeScript {
                quick_divisor_exists: false,
                fanin_arrivals: Vec::new(),
                divisor_candidates: Vec::new(),
            },
        )]));

        assert_eq!(
            speed_decomp_network_bound(&mut backend, "root", options, 1),
            Ok(())
        );

        assert_eq!(
            backend.events,
            vec![
                Event::QuickDivisor("root"),
                Event::AndOr {
                    node: "root",
                    add_inv: false,
                },
                Event::DeleteSingleFanin("root"),
            ]
        );
    }

    #[test]
    fn network_bound_reports_substitution_failure_without_recursing() {
        let options = SpeedNetOptions {
            coeff: 0.0,
            add_inv: true,
            del_crit_cubes: false,
        };
        let mut backend = RecordingBackend::new(BTreeMap::from([(
            "root",
            NodeScript {
                quick_divisor_exists: true,
                fanin_arrivals: vec![arrival(0.0, 2.0), arrival(3.0, 5.0)],
                divisor_candidates: vec![DivisorCandidate {
                    node: "div",
                    cost: NodeCost {
                        delay: 1.0,
                        area: 1.0,
                    },
                }],
            },
        )]));
        backend.substitute_result = false;

        assert_eq!(
            speed_decomp_network_bound(&mut backend, "root", options, 0),
            Err(SpeedNetError::SubstituteFailed)
        );
        assert!(
            !backend
                .events
                .iter()
                .any(|event| matches!(event, Event::AndOr { .. }))
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let text = include_str!("speed_net.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
    }

    #[derive(Clone, Debug, PartialEq)]
    struct NodeScript {
        quick_divisor_exists: bool,
        fanin_arrivals: Vec<DelayTime>,
        divisor_candidates: Vec<DivisorCandidate<&'static str>>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum TestError {
        MissingNode(&'static str),
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::MissingNode(node) => write!(f, "missing node {node}"),
            }
        }
    }

    impl Error for TestError {}

    #[derive(Clone, Debug, PartialEq)]
    enum Event {
        QuickDivisor(&'static str),
        FaninArrivals(&'static str),
        DivisorCandidates {
            node: &'static str,
            threshold: f64,
            del_crit_cubes: bool,
        },
        AddNode(&'static str),
        Substitute {
            node: &'static str,
            divisor: &'static str,
            complement: bool,
        },
        AndOr {
            node: &'static str,
            add_inv: bool,
        },
        DeleteSingleFanin(&'static str),
    }

    struct RecordingBackend {
        scripts: BTreeMap<&'static str, NodeScript>,
        root_quick_results: VecDeque<bool>,
        substitute_result: bool,
        events: Vec<Event>,
    }

    impl RecordingBackend {
        fn new(scripts: BTreeMap<&'static str, NodeScript>) -> Self {
            Self {
                scripts,
                root_quick_results: VecDeque::new(),
                substitute_result: true,
                events: Vec::new(),
            }
        }

        fn script(&self, node: &'static str) -> Result<&NodeScript, TestError> {
            self.scripts.get(node).ok_or(TestError::MissingNode(node))
        }
    }

    impl SpeedNetBackend for RecordingBackend {
        type Node = &'static str;
        type Error = TestError;

        fn kernel_timeout_occurred(&self) -> bool {
            false
        }

        fn quick_divisor_exists(&mut self, node: &Self::Node) -> Result<bool, Self::Error> {
            self.events.push(Event::QuickDivisor(*node));
            if *node == "root" {
                if let Some(result) = self.root_quick_results.pop_front() {
                    return Ok(result);
                }
            }

            Ok(self.script(*node)?.quick_divisor_exists)
        }

        fn fanin_arrivals(&self, node: &Self::Node) -> Result<Vec<DelayTime>, Self::Error> {
            Ok(self.script(*node)?.fanin_arrivals.clone())
        }

        fn divisor_candidates(
            &mut self,
            node: &Self::Node,
            options: SpeedNetOptions,
            threshold: f64,
        ) -> Result<Vec<DivisorCandidate<Self::Node>>, Self::Error> {
            self.events.push(Event::FaninArrivals(*node));
            self.events.push(Event::DivisorCandidates {
                node: *node,
                threshold,
                del_crit_cubes: options.del_crit_cubes,
            });
            Ok(self.script(*node)?.divisor_candidates.clone())
        }

        fn add_node(&mut self, node: Self::Node) -> Result<(), Self::Error> {
            self.events.push(Event::AddNode(node));
            Ok(())
        }

        fn substitute_node(
            &mut self,
            node: &Self::Node,
            divisor: &Self::Node,
            complement: bool,
        ) -> Result<bool, Self::Error> {
            self.events.push(Event::Substitute {
                node: *node,
                divisor: *divisor,
                complement,
            });
            Ok(self.substitute_result)
        }

        fn speed_and_or_decompose(
            &mut self,
            node: &Self::Node,
            options: SpeedNetOptions,
        ) -> Result<bool, Self::Error> {
            self.events.push(Event::AndOr {
                node: *node,
                add_inv: options.add_inv,
            });
            Ok(true)
        }

        fn delete_single_fanin_node(&mut self, node: &Self::Node) -> Result<(), Self::Error> {
            self.events.push(Event::DeleteSingleFanin(*node));
            Ok(())
        }
    }
}
