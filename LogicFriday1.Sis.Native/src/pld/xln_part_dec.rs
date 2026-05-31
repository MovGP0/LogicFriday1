//! Native Rust model for `LogicSynthesis/sis/pld/xln_part_dec.c`.
//!
//! The original SIS unit recursively splits oversized Xilinx PLD nodes. Most
//! graph mutation is delegated to SIS `network_t`, `node_t`, and extract APIs,
//! so this file ports the deterministic local behavior as owned Rust data:
//! fanin infeasibility scoring, kernel/cokernel/remainder cost calculation,
//! stable best-divisor selection, and split planning. Direct SIS network
//! integration remains gated by explicit dependency errors.

use std::error::Error;
use std::fmt;

pub const HICOST: i32 = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DivisorKind {
    Cokernel,
    Kernel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PldNodeSummary {
    pub name: Option<String>,
    pub fanin_count: usize,
    pub literal_count: usize,
}

impl PldNodeSummary {
    pub fn new(fanin_count: usize, literal_count: usize) -> Self {
        Self {
            name: None,
            fanin_count,
            literal_count,
        }
    }

    pub fn named(name: impl Into<String>, fanin_count: usize, literal_count: usize) -> Self {
        Self {
            name: Some(name.into()),
            fanin_count,
            literal_count,
        }
    }

    pub fn infeasibility_measure(&self, size: usize) -> i32 {
        xln_infeasibility_measure(self.fanin_count, self.literal_count, size)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelEvaluation {
    pub kernel: PldNodeSummary,
    pub new_cokernel: PldNodeSummary,
    pub remainder: PldNodeSummary,
}

impl KernelEvaluation {
    pub fn new(
        kernel: PldNodeSummary,
        new_cokernel: PldNodeSummary,
        remainder: PldNodeSummary,
    ) -> Self {
        Self {
            kernel,
            new_cokernel,
            remainder,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DivisorCandidate {
    pub kind: DivisorKind,
    pub divisor: PldNodeSummary,
    pub kernel_size: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SplitPlan {
    AlreadyFeasible,
    UseKernelDivisor {
        divisor: DivisorCandidate,
        recursive_targets: Vec<PldNodeSummary>,
    },
    UseAndOrDecomposition,
    NoAndOrDecomposition,
}

pub fn split_network_blocked() -> Result<(), XlnPartDecError> {
    Err(XlnPartDecError::MissingIntegrationDependencies {
        operation: "SIS split_network network traversal and mutation",
    })
}

pub fn split_node_blocked() -> Result<(), XlnPartDecError> {
    Err(XlnPartDecError::MissingIntegrationDependencies {
        operation: "SIS split_node node rewriting",
    })
}

pub fn xln_infeasibility_measure(num_fanin: usize, num_literals: usize, size: usize) -> i32 {
    if num_fanin <= size {
        return 0;
    }

    let diff = num_fanin - size;
    if diff == 1 {
        return 1;
    }

    if diff <= 4 {
        if num_literals <= 25 {
            return 2;
        }
        if num_literals <= 50 {
            return 3;
        }
        if num_literals <= 100 {
            return 5;
        }
    }

    6
}

pub fn kernel_cost(evaluation: &KernelEvaluation, size: usize) -> i32 {
    let num_nc = evaluation.new_cokernel.fanin_count;
    let num_k = evaluation.kernel.fanin_count;
    let num_rem = evaluation.remainder.fanin_count;

    if num_nc == size && num_k == size && num_rem == size {
        -HICOST
    } else {
        (num_nc + num_k + num_rem) as i32
    }
}

pub fn kernel_value_for_summary(
    parent: &PldNodeSummary,
    evaluation: &KernelEvaluation,
    size: usize,
) -> Vec<DivisorCandidate> {
    let mut candidates = Vec::with_capacity(2);
    let cost = kernel_cost(evaluation, size);
    let num_node = parent.fanin_count;
    let num_nc = evaluation.new_cokernel.fanin_count;
    let num_k = evaluation.kernel.fanin_count;

    if num_nc > 1 && num_nc < num_node {
        candidates.push(DivisorCandidate {
            kind: DivisorKind::Cokernel,
            divisor: evaluation.new_cokernel.clone(),
            kernel_size: cost,
        });
    }

    if num_k > 1 && num_k < num_node {
        candidates.push(DivisorCandidate {
            kind: DivisorKind::Kernel,
            divisor: evaluation.kernel.clone(),
            kernel_size: cost,
        });
    }

    candidates
}

pub fn collect_divisor_candidates(
    parent: &PldNodeSummary,
    evaluations: &[KernelEvaluation],
    size: usize,
) -> Vec<DivisorCandidate> {
    evaluations
        .iter()
        .flat_map(|evaluation| kernel_value_for_summary(parent, evaluation, size))
        .collect()
}

pub fn find_best_divisor(candidates: &[DivisorCandidate]) -> Option<DivisorCandidate> {
    candidates
        .iter()
        .min_by_key(|candidate| candidate.kernel_size)
        .cloned()
}

pub fn plan_split_node(
    node: &PldNodeSummary,
    size: usize,
    quick_divisor_exists: bool,
    evaluations: &[KernelEvaluation],
    and_or_decomposition_possible: bool,
) -> Result<SplitPlan, XlnPartDecError> {
    if size == 0 {
        return Err(XlnPartDecError::InvalidSize { size });
    }

    if node.fanin_count <= size {
        return Ok(SplitPlan::AlreadyFeasible);
    }

    if quick_divisor_exists {
        let candidates = collect_divisor_candidates(node, evaluations, size);
        if let Some(divisor) = find_best_divisor(&candidates) {
            return Ok(SplitPlan::UseKernelDivisor {
                divisor,
                recursive_targets: recursive_split_targets(evaluations),
            });
        }
    }

    if and_or_decomposition_possible {
        Ok(SplitPlan::UseAndOrDecomposition)
    } else {
        Ok(SplitPlan::NoAndOrDecomposition)
    }
}

fn recursive_split_targets(evaluations: &[KernelEvaluation]) -> Vec<PldNodeSummary> {
    match evaluations.first() {
        Some(evaluation) => vec![
            evaluation.remainder.clone(),
            evaluation.new_cokernel.clone(),
            evaluation.kernel.clone(),
        ],
        None => Vec::new(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnPartDecError {
    InvalidSize { size: usize },
    MissingIntegrationDependencies { operation: &'static str },
}

impl fmt::Display for XlnPartDecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize { size } => write!(f, "split size must be positive, got {size}"),
            Self::MissingIntegrationDependencies { operation } => {
                write!(f, "{operation} is blocked by unported SIS dependencies")
            }
        }
    }
}

impl Error for XlnPartDecError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent() -> PldNodeSummary {
        PldNodeSummary::named("f", 7, 32)
    }

    fn evaluation(
        kernel_fanins: usize,
        cokernel_fanins: usize,
        remainder_fanins: usize,
    ) -> KernelEvaluation {
        KernelEvaluation::new(
            PldNodeSummary::named("k", kernel_fanins, 10),
            PldNodeSummary::named("c", cokernel_fanins, 10),
            PldNodeSummary::named("r", remainder_fanins, 10),
        )
    }

    #[test]
    fn infeasibility_measure_matches_c_thresholds() {
        assert_eq!(xln_infeasibility_measure(5, 999, 5), 0);
        assert_eq!(xln_infeasibility_measure(6, 999, 5), 1);
        assert_eq!(xln_infeasibility_measure(8, 25, 5), 2);
        assert_eq!(xln_infeasibility_measure(8, 50, 5), 3);
        assert_eq!(xln_infeasibility_measure(8, 100, 5), 5);
        assert_eq!(xln_infeasibility_measure(8, 101, 5), 6);
        assert_eq!(xln_infeasibility_measure(10, 1, 5), 6);
    }

    #[test]
    fn kernel_value_keeps_only_nontrivial_smaller_divisors() {
        let candidates = kernel_value_for_summary(&parent(), &evaluation(2, 7, 4), 5);

        assert_eq!(
            candidates,
            vec![DivisorCandidate {
                kind: DivisorKind::Kernel,
                divisor: PldNodeSummary::named("k", 2, 10),
                kernel_size: 13,
            }]
        );
    }

    #[test]
    fn exact_size_kernel_cost_gets_c_high_priority_sentinel() {
        let eval = evaluation(5, 5, 5);
        let candidates = kernel_value_for_summary(&parent(), &eval, 5);

        assert_eq!(kernel_cost(&eval, 5), -HICOST);
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.kernel_size == -HICOST)
        );
    }

    #[test]
    fn best_divisor_selection_is_stable_for_equal_costs() {
        let candidates = vec![
            DivisorCandidate {
                kind: DivisorKind::Cokernel,
                divisor: PldNodeSummary::named("first", 3, 4),
                kernel_size: 9,
            },
            DivisorCandidate {
                kind: DivisorKind::Kernel,
                divisor: PldNodeSummary::named("second", 4, 5),
                kernel_size: 9,
            },
        ];

        assert_eq!(find_best_divisor(&candidates).unwrap(), candidates[0]);
    }

    #[test]
    fn split_plan_prefers_kernel_divisor_then_and_or_fallback() {
        let evals = vec![evaluation(4, 3, 2)];
        let plan = plan_split_node(&parent(), 5, true, &evals, true).unwrap();

        match plan {
            SplitPlan::UseKernelDivisor {
                divisor,
                recursive_targets,
            } => {
                assert_eq!(divisor.kind, DivisorKind::Cokernel);
                assert_eq!(recursive_targets.len(), 3);
                assert_eq!(recursive_targets[0].name.as_deref(), Some("r"));
            }
            other => panic!("unexpected plan: {other:?}"),
        }

        assert_eq!(
            plan_split_node(&parent(), 5, false, &evals, true),
            Ok(SplitPlan::UseAndOrDecomposition)
        );
        assert_eq!(
            plan_split_node(&parent(), 5, false, &evals, false),
            Ok(SplitPlan::NoAndOrDecomposition)
        );
    }

    #[test]
    fn feasible_nodes_are_not_split_and_zero_size_is_rejected() {
        assert_eq!(
            plan_split_node(&PldNodeSummary::new(4, 9), 5, true, &[], true),
            Ok(SplitPlan::AlreadyFeasible)
        );
        assert_eq!(
            plan_split_node(&parent(), 0, true, &[], true),
            Err(XlnPartDecError::InvalidSize { size: 0 })
        );
    }
}
