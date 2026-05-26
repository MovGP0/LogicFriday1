//! Native Rust model for `LogicSynthesis/sis/seqbdd/product.c`.
//!
//! The original C file combines two concerns:
//! - BDD/network integration for range-data allocation, image computation, and
//!   output checking.
//! - The incremental product planner that orders transition functions and
//!   smooths variables as soon as their last dependent functions have been
//!   merged.
//!
//! The planner is ported here with owned Rust data so it can be tested without
//! the legacy SIS C ABI. BDD-backed entry points report typed dependency errors
//! until the required native BDD, ntbdd, node, network, and seqbdd helper ports
//! are available.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead: &'static str,
    pub source_file: &'static str,
}

pub const REQUIRED_PRODUCT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.75",
        source_file: "LogicSynthesis/sis/bdd_ucb/and_smooth.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.78",
        source_file: "LogicSynthesis/sis/bdd_ucb/bdd_cofactor.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.87",
        source_file: "LogicSynthesis/sis/bdd_ucb/bdd_quantify.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.89",
        source_file: "LogicSynthesis/sis/bdd_ucb/bdd_substit.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.90",
        source_file: "LogicSynthesis/sis/bdd_ucb/bdd_support.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.326",
        source_file: "LogicSynthesis/sis/ntbdd/bdd_at_node.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.329",
        source_file: "LogicSynthesis/sis/ntbdd/manager.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.330",
        source_file: "LogicSynthesis/sis/ntbdd/node_to_bdd.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.431",
        source_file: "LogicSynthesis/sis/seqbdd/prioqueue.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.442",
        source_file: "LogicSynthesis/sis/seqbdd/verif_util.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.518",
        source_file: "LogicSynthesis/sis/var_set/var_set.c",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductPortDisposition {
    PlannerPortedSisBddIntegrationBlocked,
}

pub fn product_port_disposition() -> ProductPortDisposition {
    ProductPortDisposition::PlannerPortedSisBddIntegrationBlocked
}

pub fn is_product_sis_integration_blocked() -> bool {
    product_port_disposition() == ProductPortDisposition::PlannerPortedSisBddIntegrationBlocked
}

pub fn required_product_dependencies() -> &'static [PortDependency] {
    REQUIRED_PRODUCT_DEPENDENCIES
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductError {
    MissingSisDependencies {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    EmptyTransitionSet,
    VariableOutOfRange {
        variable: usize,
        n_vars: usize,
    },
}

impl fmt::Display for ProductError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS/BDD dependencies",
                dependencies.len()
            ),
            Self::EmptyTransitionSet => write!(f, "product transition set is empty"),
            Self::VariableOutOfRange { variable, n_vars } => write!(
                f,
                "product support variable {variable} is outside manager range 0..{n_vars}"
            ),
        }
    }
}

impl Error for ProductError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSupport {
    pub fnid: usize,
    pub variables: BTreeSet<usize>,
}

impl FunctionSupport {
    pub fn new(fnid: usize, variables: impl IntoIterator<Item = usize>) -> Self {
        Self {
            fnid,
            variables: variables.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LonelySmooth {
    pub fnid: usize,
    pub variable: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeStep {
    pub absorbed_fnid: usize,
    pub target_fnid: usize,
    pub smoothed_variables: Vec<usize>,
    pub moved_variables: Vec<usize>,
    pub target_partition_before: usize,
    pub absorbed_partition: usize,
    pub target_partition_after: usize,
    pub target_cost_after: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductMergePlan {
    pub initial_function_count: usize,
    pub lonely_smoothing: Vec<LonelySmooth>,
    pub merges: Vec<MergeStep>,
    pub final_fnid: usize,
    pub final_support: BTreeSet<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FnInfo {
    fnid: usize,
    support: BTreeSet<usize>,
    partition: usize,
    cost: usize,
}

pub fn product_alloc_range_data() -> Result<(), ProductError> {
    missing_sis_dependencies("product_alloc_range_data")
}

pub fn product_compute_next_states() -> Result<(), ProductError> {
    missing_sis_dependencies("product_compute_next_states")
}

pub fn product_compute_reverse_image() -> Result<(), ProductError> {
    missing_sis_dependencies("product_compute_reverse_image")
}

pub fn product_check_output() -> Result<(), ProductError> {
    missing_sis_dependencies("product_check_output")
}

fn missing_sis_dependencies<T>(operation: &'static str) -> Result<T, ProductError> {
    Err(ProductError::MissingSisDependencies {
        operation,
        dependencies: REQUIRED_PRODUCT_DEPENDENCIES,
    })
}

pub fn plan_incremental_and_smooth(
    supports: impl IntoIterator<Item = FunctionSupport>,
    keep_variables: impl IntoIterator<Item = usize>,
    n_vars: usize,
) -> Result<ProductMergePlan, ProductError> {
    let keep_variables = keep_variables.into_iter().collect::<BTreeSet<_>>();
    validate_variables(keep_variables.iter().copied(), n_vars)?;

    let mut fns = supports
        .into_iter()
        .enumerate()
        .map(|(cost, support)| {
            validate_variables(support.variables.iter().copied(), n_vars)?;
            Ok(FnInfo {
                fnid: support.fnid,
                support: support.variables,
                partition: cost,
                cost,
            })
        })
        .collect::<Result<Vec<_>, ProductError>>()?;

    if fns.is_empty() {
        return Err(ProductError::EmptyTransitionSet);
    }

    let initial_function_count = fns.len();
    let (next_partition, mut partition_count) = initialize_partition_info(&mut fns);
    let mut var_table = extract_var_table(&fns, &keep_variables, n_vars);
    let lonely_smoothing = smooth_lonely_variables(&mut fns, &mut var_table, n_vars);
    let mut queue = fns;
    let mut merges = Vec::new();

    loop {
        sort_queue(&mut queue);
        let mut fn0 = queue.remove(0);
        if queue.is_empty() {
            return Ok(ProductMergePlan {
                initial_function_count,
                lonely_smoothing,
                merges,
                final_fnid: fn0.fnid,
                final_support: fn0.support,
            });
        }
        let fn1 = queue.remove(0);

        let target_partition_before = fn0.partition;
        let (smoothed_variables, moved_variables) =
            smooth_vars_extract(&mut fn0, &fn1, &mut var_table, n_vars);

        let mut merged_support = fn0.support.clone();
        merged_support.extend(fn1.support.iter().copied());
        for variable in &smoothed_variables {
            merged_support.remove(variable);
        }
        fn0.support = merged_support;

        partition_count[fn1.partition] -= 1;
        if partition_count[fn0.partition] == 1 {
            partition_count[fn0.partition] -= 1;
            fn0.partition = next_partition[fn0.partition];
            partition_count[fn0.partition] += 1;
        }
        fn0.cost += initial_function_count;

        merges.push(MergeStep {
            absorbed_fnid: fn1.fnid,
            target_fnid: fn0.fnid,
            smoothed_variables,
            moved_variables,
            target_partition_before,
            absorbed_partition: fn1.partition,
            target_partition_after: fn0.partition,
            target_cost_after: fn0.cost,
        });

        queue.push(fn0);
    }
}

fn validate_variables(
    variables: impl IntoIterator<Item = usize>,
    n_vars: usize,
) -> Result<(), ProductError> {
    for variable in variables {
        if variable >= n_vars {
            return Err(ProductError::VariableOutOfRange { variable, n_vars });
        }
    }

    Ok(())
}

fn extract_var_table(
    fns: &[FnInfo],
    keep_variables: &BTreeSet<usize>,
    n_vars: usize,
) -> Vec<BTreeSet<usize>> {
    let mut table = vec![BTreeSet::new(); n_vars];
    for function in fns {
        for variable in &function.support {
            if !keep_variables.contains(variable) {
                table[*variable].insert(function.fnid);
            }
        }
    }

    table
}

fn smooth_lonely_variables(
    fns: &mut [FnInfo],
    var_table: &mut [BTreeSet<usize>],
    n_vars: usize,
) -> Vec<LonelySmooth> {
    let fn_index_by_id = fns
        .iter()
        .enumerate()
        .map(|(index, function)| (function.fnid, index))
        .collect::<BTreeMap<_, _>>();
    let mut result = Vec::new();

    for variable in 0..n_vars {
        if var_table[variable].len() == 1 {
            let fnid = *var_table[variable]
                .iter()
                .next()
                .expect("single dependent function exists");
            let fn_index = fn_index_by_id[&fnid];
            fns[fn_index].support.remove(&variable);
            result.push(LonelySmooth { fnid, variable });
            var_table[variable].clear();
        } else if var_table[variable].is_empty() {
            var_table[variable].clear();
        }
    }

    result
}

fn smooth_vars_extract(
    fn0: &mut FnInfo,
    fn1: &FnInfo,
    var_table: &mut [BTreeSet<usize>],
    n_vars: usize,
) -> (Vec<usize>, Vec<usize>) {
    let mut smoothed = Vec::new();
    let mut moved = Vec::new();

    for variable in 0..n_vars {
        if var_table[variable].is_empty() || !fn1.support.contains(&variable) {
            continue;
        }

        let remaining = var_table[variable].len();
        debug_assert!(remaining > 1);

        if !fn0.support.contains(&variable) {
            fn0.support.insert(variable);
            var_table[variable].remove(&fn1.fnid);
            var_table[variable].insert(fn0.fnid);
            moved.push(variable);
        } else if remaining == 2 {
            var_table[variable].clear();
            smoothed.push(variable);
        } else {
            var_table[variable].remove(&fn1.fnid);
        }
    }

    (smoothed, moved)
}

fn initialize_partition_info(fns: &mut [FnInfo]) -> (Vec<usize>, Vec<usize>) {
    let n_partitions = fns.len();
    let mut n = 1;
    while n <= n_partitions {
        n <<= 1;
    }
    if n > n_partitions {
        n >>= 1;
    }
    let x = n_partitions - n;

    let mut partition_map = vec![0; n_partitions];
    for (i, entry) in partition_map.iter_mut().enumerate().take(2 * x) {
        *entry = i;
    }
    for (i, entry) in partition_map.iter_mut().enumerate().skip(2 * x) {
        *entry = i + x;
    }

    let n_entries = (n_partitions * 2) - 1;
    let mut next_partition = vec![0; n_entries];
    for (i, entry) in next_partition.iter_mut().enumerate().take(2 * x) {
        *entry = (2 * x) + (i / 2);
    }

    let mut count = n_entries - 1;
    let mut i = n_entries.saturating_sub(3);
    while i >= 2 * x && i < n_entries {
        next_partition[i] = count;
        next_partition[i + 1] = count;
        if i < 2 {
            break;
        }
        i -= 2;
        count -= 1;
    }
    next_partition[n_entries - 1] = n_entries - 1;

    let mut partition_count = vec![0; n_entries];
    for function in fns {
        function.partition = partition_map[function.partition];
        partition_count[function.partition] += 1;
    }

    (next_partition, partition_count)
}

fn sort_queue(queue: &mut [FnInfo]) {
    queue.sort_by_key(|function| (function.partition, function.cost, function.fnid));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn support(fnid: usize, variables: &[usize]) -> FunctionSupport {
        FunctionSupport::new(fnid, variables.iter().copied())
    }

    #[test]
    fn planner_reports_empty_transition_set() {
        assert_eq!(
            plan_incremental_and_smooth([], [], 3),
            Err(ProductError::EmptyTransitionSet)
        );
    }

    #[test]
    fn planner_rejects_variables_outside_manager_range() {
        assert_eq!(
            plan_incremental_and_smooth([support(0, &[0, 3])], [], 3),
            Err(ProductError::VariableOutOfRange {
                variable: 3,
                n_vars: 3,
            })
        );
        assert_eq!(
            plan_incremental_and_smooth([support(0, &[0])], [4], 3),
            Err(ProductError::VariableOutOfRange {
                variable: 4,
                n_vars: 3,
            })
        );
    }

    #[test]
    fn lonely_variables_are_smoothed_before_merging() {
        let plan = plan_incremental_and_smooth([support(0, &[0, 1]), support(1, &[1, 2])], [1], 3)
            .expect("planner should succeed");

        assert_eq!(
            plan.lonely_smoothing,
            vec![
                LonelySmooth {
                    fnid: 0,
                    variable: 0,
                },
                LonelySmooth {
                    fnid: 1,
                    variable: 2,
                },
            ]
        );
        assert_eq!(plan.merges.len(), 1);
        assert_eq!(plan.merges[0].smoothed_variables, Vec::<usize>::new());
        assert_eq!(plan.final_support, BTreeSet::from([1]));
    }

    #[test]
    fn merge_smooths_variable_when_two_remaining_functions_share_it() {
        let plan = plan_incremental_and_smooth(
            [support(0, &[0, 1]), support(1, &[0, 2]), support(2, &[2])],
            [],
            3,
        )
        .expect("planner should succeed");

        assert_eq!(
            plan.lonely_smoothing,
            vec![LonelySmooth {
                fnid: 0,
                variable: 1,
            }]
        );
        assert_eq!(plan.merges[0].absorbed_fnid, 1);
        assert_eq!(plan.merges[0].target_fnid, 0);
        assert_eq!(plan.merges[0].smoothed_variables, vec![0]);
        assert_eq!(plan.merges[0].moved_variables, vec![2]);
        assert_eq!(plan.final_support, BTreeSet::new());
    }

    #[test]
    fn partition_promotion_matches_product_c_binary_tree_mapping() {
        let plan = plan_incremental_and_smooth(
            [
                support(0, &[0]),
                support(1, &[0]),
                support(2, &[1]),
                support(3, &[1]),
                support(4, &[2]),
            ],
            [],
            3,
        )
        .expect("planner should succeed");

        assert_eq!(
            plan.merges
                .iter()
                .map(|step| (step.target_partition_before, step.absorbed_partition))
                .collect::<Vec<_>>(),
            vec![(0, 1), (2, 3), (4, 5), (6, 7)]
        );
        assert_eq!(
            plan.merges
                .iter()
                .map(|step| step.target_partition_after)
                .collect::<Vec<_>>(),
            vec![2, 6, 7, 8]
        );
    }

    #[test]
    fn sis_bdd_entry_points_report_dependency_beads_and_sources() {
        let error = product_compute_next_states().expect_err("BDD integration should be blocked");
        let ProductError::MissingSisDependencies {
            operation,
            dependencies,
        } = error
        else {
            panic!("unexpected error kind");
        };

        assert_eq!(operation, "product_compute_next_states");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.75"
                && dependency.source_file == "LogicSynthesis/sis/bdd_ucb/and_smooth.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.431"
                && dependency.source_file == "LogicSynthesis/sis/seqbdd/prioqueue.c"
        }));
        assert!(is_product_sis_integration_blocked());
        assert_eq!(
            required_product_dependencies(),
            REQUIRED_PRODUCT_DEPENDENCIES
        );
    }
}
