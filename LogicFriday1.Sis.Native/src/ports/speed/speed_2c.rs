//! Native Rust behavior for `sis/speed/speed_2c.c`.
//!
//! The C file searches two-cube kernels, evaluates each kernel/cokernel pair,
//! and recursively decomposes the chosen divisor. Kernel generation and SIS
//! node mutation are represented here as explicit planning decisions over owned
//! Rust data.

use std::error::Error;
use std::fmt;

pub const SCALE_2: i32 = 100_000;
pub const SCALE: i32 = 100;
pub const CRITICAL_FRACTION: f64 = 0.05;
pub const FUDGE: f64 = 0.0001;
pub const POS_LARGE: f64 = 10_000.0;
pub const NEG_LARGE: f64 = -10_000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn min_edge(self) -> f64 {
        self.rise.min(self.fall)
    }

    pub fn max_edge(self) -> f64 {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KernelSearch {
    pub threshold: f64,
    pub timeout_occurred: bool,
    pub delete_critical_cubes: bool,
}

pub fn threshold_for_attempt(
    fanin_arrivals: &[DelayTime],
    attempt_no: usize,
) -> Result<f64, Speed2cError> {
    if fanin_arrivals.is_empty() {
        return Err(Speed2cError::NoFanins);
    }

    let mut min_t = POS_LARGE;
    let mut max_t = NEG_LARGE;
    for arrival in fanin_arrivals {
        if !arrival.rise.is_finite() || !arrival.fall.is_finite() {
            return Err(Speed2cError::NonFiniteArrival);
        }
        min_t = min_t.min(arrival.min_edge());
        max_t = max_t.max(arrival.max_edge());
    }

    if (max_t - min_t) < 1.0e-3 {
        Ok(POS_LARGE)
    } else {
        Ok(max_t - (attempt_no as f64 * CRITICAL_FRACTION + FUDGE) * (max_t - min_t))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateSide {
    Kernel,
    Cokernel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KernelCandidate {
    pub id: String,
    pub kernel_time_cost: f64,
    pub kernel_area_saving: f64,
    pub cokernel_time_cost: Option<f64>,
    pub cokernel_area_saving: Option<f64>,
    pub kernel_fanin_count: usize,
    pub cokernel_fanin_count: usize,
    pub kernel_is_zero: bool,
    pub cokernel_is_zero: bool,
    pub cokernel_is_one: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BestKernel {
    pub id: String,
    pub side: CandidateSide,
    pub time_cost: f64,
    pub area_saving: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KernelVisit {
    StopGeneration,
    Continue { selected: Option<BestKernel> },
}

pub fn select_best_kernel(
    candidates: &[KernelCandidate],
    timeout_occurred: bool,
) -> Result<Option<BestKernel>, Speed2cError> {
    if timeout_occurred {
        return Ok(None);
    }

    let mut best: Option<BestKernel> = None;
    for candidate in candidates {
        match evaluate_kernel_candidate(candidate, best.as_ref(), false)? {
            KernelVisit::StopGeneration => break,
            KernelVisit::Continue { selected } => best = selected,
        }
    }

    Ok(best)
}

pub fn evaluate_kernel_candidate(
    candidate: &KernelCandidate,
    current_best: Option<&BestKernel>,
    timeout_occurred: bool,
) -> Result<KernelVisit, Speed2cError> {
    if timeout_occurred {
        return Ok(KernelVisit::StopGeneration);
    }

    if candidate.kernel_is_zero || candidate.kernel_fanin_count == 1 || candidate.cokernel_is_one {
        return Ok(KernelVisit::Continue {
            selected: current_best.cloned(),
        });
    }

    let mut selected = current_best.cloned();
    consider_side(
        &mut selected,
        candidate,
        CandidateSide::Kernel,
        candidate.kernel_time_cost,
        candidate.kernel_area_saving,
    )?;

    if !candidate.cokernel_is_zero && candidate.cokernel_fanin_count > 1 {
        let (Some(time), Some(area)) =
            (candidate.cokernel_time_cost, candidate.cokernel_area_saving)
        else {
            return Err(Speed2cError::MissingCokernelCost);
        };
        consider_side(
            &mut selected,
            candidate,
            CandidateSide::Cokernel,
            time,
            area,
        )?;
    }

    Ok(KernelVisit::Continue { selected })
}

fn consider_side(
    best: &mut Option<BestKernel>,
    candidate: &KernelCandidate,
    side: CandidateSide,
    time_cost: f64,
    area_saving: f64,
) -> Result<(), Speed2cError> {
    if !time_cost.is_finite() || !area_saving.is_finite() {
        return Err(Speed2cError::NonFiniteCost);
    }

    let improves = best.as_ref().is_none_or(|current| {
        time_cost < current.time_cost
            || (time_cost == current.time_cost && area_saving > current.area_saving)
    });
    if improves {
        *best = Some(BestKernel {
            id: candidate.id.clone(),
            side,
            time_cost,
            area_saving,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecompositionPlan {
    SearchTwoCubeKernels,
    UseBestKernel { use_cokernel: bool },
    FallbackAndOrDecomposition { reason: AndOrReason },
    DeleteSingleFaninWhenNoAddedInverters,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndOrReason {
    KernelTimeout,
    NoQuickDivisor,
    NoAppropriateKernel,
}

pub fn plan_two_cube_decomposition(
    best: Option<&BestKernel>,
    add_inv: bool,
) -> Vec<DecompositionPlan> {
    plan_two_cube_decomposition_with_reason(best, add_inv, AndOrReason::NoAppropriateKernel)
}

pub fn plan_two_cube_decomposition_with_reason(
    best: Option<&BestKernel>,
    add_inv: bool,
    fallback_reason: AndOrReason,
) -> Vec<DecompositionPlan> {
    let mut plan = Vec::new();
    plan.push(DecompositionPlan::SearchTwoCubeKernels);
    if let Some(best) = best {
        plan.push(DecompositionPlan::UseBestKernel {
            use_cokernel: best.side == CandidateSide::Cokernel,
        });
    } else {
        plan.push(DecompositionPlan::FallbackAndOrDecomposition {
            reason: fallback_reason,
        });
        if !add_inv {
            plan.push(DecompositionPlan::DeleteSingleFaninWhenNoAddedInverters);
        }
    }
    plan
}

#[derive(Clone, Debug, PartialEq)]
pub struct Speed2cInput {
    pub fanin_arrivals: Vec<DelayTime>,
    pub attempt_no: usize,
    pub timeout_occurred: bool,
    pub quick_divisor_exists: bool,
    pub add_inv: bool,
    pub candidates: Vec<KernelCandidate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Speed2cDecision {
    pub threshold: Option<f64>,
    pub best_kernel: Option<BestKernel>,
    pub plan: Vec<DecompositionPlan>,
}

pub fn plan_speed_2c_decomposition(input: Speed2cInput) -> Result<Speed2cDecision, Speed2cError> {
    if input.timeout_occurred {
        return Ok(Speed2cDecision {
            threshold: None,
            best_kernel: None,
            plan: plan_two_cube_decomposition_with_reason(
                None,
                input.add_inv,
                AndOrReason::KernelTimeout,
            ),
        });
    }

    if !input.quick_divisor_exists {
        return Ok(Speed2cDecision {
            threshold: None,
            best_kernel: None,
            plan: plan_two_cube_decomposition_with_reason(
                None,
                input.add_inv,
                AndOrReason::NoQuickDivisor,
            ),
        });
    }

    let threshold = threshold_for_attempt(&input.fanin_arrivals, input.attempt_no)?;
    let best_kernel = select_best_kernel(&input.candidates, false)?;
    let plan = plan_two_cube_decomposition(best_kernel.as_ref(), input.add_inv);

    Ok(Speed2cDecision {
        threshold: Some(threshold),
        best_kernel,
        plan,
    })
}

pub fn speed_2c_decomp_network_bound<Network, Node>(
    _network: &mut Network,
    _node: &mut Node,
    _attempt_no: usize,
) -> Result<(), Speed2cError> {
    Err(Speed2cError::MissingNativePorts {
        operation: "speed_2c_decomp",
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum Speed2cError {
    NoFanins,
    NonFiniteArrival,
    NonFiniteCost,
    MissingCokernelCost,
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for Speed2cError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFanins => write!(f, "cannot compute a two-cube threshold without fanins"),
            Self::NonFiniteArrival => write!(f, "fanin arrival time is not finite"),
            Self::NonFiniteCost => write!(f, "kernel cost is not finite"),
            Self::MissingCokernelCost => write!(f, "useful cokernel candidate has no cost"),
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS network integration")
            }
        }
    }
}

impl Error for Speed2cError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str) -> KernelCandidate {
        KernelCandidate {
            id: id.to_string(),
            kernel_time_cost: 5.0,
            kernel_area_saving: 2.0,
            cokernel_time_cost: None,
            cokernel_area_saving: None,
            kernel_fanin_count: 2,
            cokernel_fanin_count: 0,
            kernel_is_zero: false,
            cokernel_is_zero: true,
            cokernel_is_one: false,
        }
    }

    #[test]
    fn constants_match_speed_2c_c() {
        assert_eq!(SCALE_2, 100_000);
        assert_eq!(SCALE, 100);
        assert_eq!(CRITICAL_FRACTION, 0.05);
        assert_eq!(FUDGE, 0.0001);
    }

    #[test]
    fn threshold_matches_attempt_formula_and_flat_arrival_case() {
        let arrivals = [
            DelayTime {
                rise: 1.0,
                fall: 2.0,
            },
            DelayTime {
                rise: 6.0,
                fall: 4.0,
            },
        ];

        assert!((threshold_for_attempt(&arrivals, 2).unwrap() - 5.4995).abs() < 1.0e-9);
        assert_eq!(
            threshold_for_attempt(
                &[DelayTime {
                    rise: 3.0,
                    fall: 3.0005,
                }],
                3,
            )
            .unwrap(),
            POS_LARGE
        );
    }

    #[test]
    fn best_kernel_uses_lowest_time_then_greater_area_saving() {
        let mut first = candidate("a");
        first.cokernel_time_cost = Some(5.0);
        first.cokernel_area_saving = Some(3.0);
        first.cokernel_fanin_count = 2;
        first.cokernel_is_zero = false;

        let mut second = candidate("b");
        second.kernel_time_cost = 4.5;
        second.kernel_area_saving = 1.0;

        let candidates = [first.clone(), second];

        assert_eq!(
            select_best_kernel(&candidates, false).unwrap(),
            Some(BestKernel {
                id: "b".to_string(),
                side: CandidateSide::Kernel,
                time_cost: 4.5,
                area_saving: 1.0,
            })
        );

        assert_eq!(
            select_best_kernel(&candidates[..1], false).unwrap(),
            Some(BestKernel {
                id: "a".to_string(),
                side: CandidateSide::Cokernel,
                time_cost: 5.0,
                area_saving: 3.0,
            })
        );
    }

    #[test]
    fn evaluator_stops_on_timeout_and_keeps_current_best_for_rejected_pairs() {
        let current = BestKernel {
            id: "old".to_string(),
            side: CandidateSide::Kernel,
            time_cost: 3.0,
            area_saving: 1.0,
        };

        assert_eq!(
            evaluate_kernel_candidate(&candidate("new"), Some(&current), true).unwrap(),
            KernelVisit::StopGeneration
        );

        let mut rejected = candidate("rejected");
        rejected.cokernel_is_one = true;
        assert_eq!(
            evaluate_kernel_candidate(&rejected, Some(&current), false).unwrap(),
            KernelVisit::Continue {
                selected: Some(current)
            }
        );
    }

    #[test]
    fn useful_cokernel_requires_costs() {
        let mut candidate = candidate("missing");
        candidate.cokernel_is_zero = false;
        candidate.cokernel_fanin_count = 2;

        assert_eq!(
            evaluate_kernel_candidate(&candidate, None, false),
            Err(Speed2cError::MissingCokernelCost)
        );
    }

    #[test]
    fn invalid_candidates_and_timeout_yield_no_best_kernel() {
        let mut zero = candidate("zero");
        zero.kernel_is_zero = true;

        let mut single = candidate("single");
        single.kernel_fanin_count = 1;

        let candidates = [zero, single];

        assert_eq!(select_best_kernel(&candidates, false).unwrap(), None);
        assert_eq!(select_best_kernel(&candidates, true).unwrap(), None);
    }

    #[test]
    fn plan_records_fallback_delete_when_add_inv_is_disabled() {
        assert_eq!(
            plan_two_cube_decomposition(None, false),
            vec![
                DecompositionPlan::SearchTwoCubeKernels,
                DecompositionPlan::FallbackAndOrDecomposition {
                    reason: AndOrReason::NoAppropriateKernel,
                },
                DecompositionPlan::DeleteSingleFaninWhenNoAddedInverters,
            ]
        );

        let best = BestKernel {
            id: "k".to_string(),
            side: CandidateSide::Cokernel,
            time_cost: 2.0,
            area_saving: 1.0,
        };
        assert_eq!(
            plan_two_cube_decomposition(Some(&best), false),
            vec![
                DecompositionPlan::SearchTwoCubeKernels,
                DecompositionPlan::UseBestKernel { use_cokernel: true },
            ]
        );
    }

    #[test]
    fn top_level_decision_matches_quick_divisor_timeout_and_extraction_branches() {
        assert_eq!(
            plan_speed_2c_decomposition(Speed2cInput {
                fanin_arrivals: Vec::new(),
                attempt_no: 0,
                timeout_occurred: false,
                quick_divisor_exists: false,
                add_inv: false,
                candidates: Vec::new(),
            })
            .unwrap()
            .plan,
            vec![
                DecompositionPlan::SearchTwoCubeKernels,
                DecompositionPlan::FallbackAndOrDecomposition {
                    reason: AndOrReason::NoQuickDivisor,
                },
                DecompositionPlan::DeleteSingleFaninWhenNoAddedInverters,
            ]
        );

        let mut selected = candidate("k");
        selected.kernel_time_cost = 2.0;
        let decision = plan_speed_2c_decomposition(Speed2cInput {
            fanin_arrivals: vec![
                DelayTime {
                    rise: 1.0,
                    fall: 2.0,
                },
                DelayTime {
                    rise: 5.0,
                    fall: 4.0,
                },
            ],
            attempt_no: 0,
            timeout_occurred: false,
            quick_divisor_exists: true,
            add_inv: true,
            candidates: vec![selected],
        })
        .unwrap();

        assert_eq!(decision.threshold, Some(4.9996));
        assert_eq!(
            decision.best_kernel,
            Some(BestKernel {
                id: "k".to_string(),
                side: CandidateSide::Kernel,
                time_cost: 2.0,
                area_saving: 2.0,
            })
        );
        assert_eq!(
            decision.plan,
            vec![
                DecompositionPlan::SearchTwoCubeKernels,
                DecompositionPlan::UseBestKernel {
                    use_cokernel: false,
                },
            ]
        );
    }

    #[test]
    fn network_bound_entry_point_reports_missing_native_ports() {
        let mut network = ();
        let mut node = ();
        assert_eq!(
            speed_2c_decomp_network_bound(&mut network, &mut node, 0),
            Err(Speed2cError::MissingNativePorts {
                operation: "speed_2c_decomp",
            })
        );
    }
}
