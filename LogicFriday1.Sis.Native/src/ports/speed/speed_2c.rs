//! Native Rust port scaffold for `sis/speed/speed_2c.c`.
//!
//! The C file searches two-cube kernels, evaluates each kernel/cokernel pair,
//! and recursively decomposes the chosen divisor. Kernel generation and SIS
//! node mutation are still C-only, so this module ports the threshold and
//! best-candidate selection behavior as native Rust while exposing the full
//! extraction entry point as an explicit blocked operation.

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
    pub kernel_is_zero: bool,
    pub cokernel_is_one: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BestKernel {
    pub id: String,
    pub side: CandidateSide,
    pub time_cost: f64,
    pub area_saving: f64,
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
        if candidate.kernel_is_zero
            || candidate.kernel_fanin_count == 1
            || candidate.cokernel_is_one
        {
            continue;
        }

        consider_side(
            &mut best,
            candidate,
            CandidateSide::Kernel,
            candidate.kernel_time_cost,
            candidate.kernel_area_saving,
        )?;

        if let (Some(time), Some(area)) =
            (candidate.cokernel_time_cost, candidate.cokernel_area_saving)
        {
            consider_side(&mut best, candidate, CandidateSide::Cokernel, time, area)?;
        }
    }

    Ok(best)
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
    FallbackAndOrDecomposition,
    DeleteSingleFaninWhenNoAddedInverters,
}

pub fn plan_two_cube_decomposition(
    best: Option<&BestKernel>,
    add_inv: bool,
) -> Vec<DecompositionPlan> {
    let mut plan = Vec::new();
    plan.push(DecompositionPlan::SearchTwoCubeKernels);
    if let Some(best) = best {
        plan.push(DecompositionPlan::UseBestKernel {
            use_cokernel: best.side == CandidateSide::Cokernel,
        });
    } else {
        plan.push(DecompositionPlan::FallbackAndOrDecomposition);
        if !add_inv {
            plan.push(DecompositionPlan::DeleteSingleFaninWhenNoAddedInverters);
        }
    }
    plan
}

pub fn speed_2c_decomp_network_bound() -> Result<(), Speed2cError> {
    Err(Speed2cError::MissingDependency(
        "speed_2c_decomp requires native two-cube kernel generation, node division/substitution, speed_net evaluation, speed_util critical-cube deletion, and network mutation ports",
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Speed2cError {
    NoFanins,
    NonFiniteArrival,
    NonFiniteCost,
    MissingDependency(&'static str),
}

impl fmt::Display for Speed2cError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFanins => write!(f, "cannot compute a two-cube threshold without fanins"),
            Self::NonFiniteArrival => write!(f, "fanin arrival time is not finite"),
            Self::NonFiniteCost => write!(f, "kernel cost is not finite"),
            Self::MissingDependency(message) => write!(f, "{message}"),
        }
    }
}

impl Error for Speed2cError {}

#[cfg(test)]
mod tests {
    use super::*;

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
        let candidates = [
            KernelCandidate {
                id: "a".to_string(),
                kernel_time_cost: 5.0,
                kernel_area_saving: 2.0,
                cokernel_time_cost: Some(5.0),
                cokernel_area_saving: Some(3.0),
                kernel_fanin_count: 2,
                kernel_is_zero: false,
                cokernel_is_one: false,
            },
            KernelCandidate {
                id: "b".to_string(),
                kernel_time_cost: 4.5,
                kernel_area_saving: 1.0,
                cokernel_time_cost: None,
                cokernel_area_saving: None,
                kernel_fanin_count: 2,
                kernel_is_zero: false,
                cokernel_is_one: false,
            },
        ];

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
    fn invalid_candidates_and_timeout_yield_no_best_kernel() {
        let candidates = [
            KernelCandidate {
                id: "zero".to_string(),
                kernel_time_cost: 1.0,
                kernel_area_saving: 1.0,
                cokernel_time_cost: None,
                cokernel_area_saving: None,
                kernel_fanin_count: 3,
                kernel_is_zero: true,
                cokernel_is_one: false,
            },
            KernelCandidate {
                id: "single".to_string(),
                kernel_time_cost: 1.0,
                kernel_area_saving: 1.0,
                cokernel_time_cost: None,
                cokernel_area_saving: None,
                kernel_fanin_count: 1,
                kernel_is_zero: false,
                cokernel_is_one: false,
            },
        ];

        assert_eq!(select_best_kernel(&candidates, false).unwrap(), None);
        assert_eq!(select_best_kernel(&candidates, true).unwrap(), None);
    }

    #[test]
    fn plan_records_fallback_delete_when_add_inv_is_disabled() {
        assert_eq!(
            plan_two_cube_decomposition(None, false),
            vec![
                DecompositionPlan::SearchTwoCubeKernels,
                DecompositionPlan::FallbackAndOrDecomposition,
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
    fn network_bound_entry_point_reports_missing_dependencies() {
        assert_eq!(
            speed_2c_decomp_network_bound(),
            Err(Speed2cError::MissingDependency(
                "speed_2c_decomp requires native two-cube kernel generation, node division/substitution, speed_net evaluation, speed_util critical-cube deletion, and network mutation ports",
            ))
        );
    }
}
