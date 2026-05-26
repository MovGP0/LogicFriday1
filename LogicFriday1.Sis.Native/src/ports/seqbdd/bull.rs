//! Native Rust model for `LogicSynthesis/sis/seqbdd/bull.c`.
//!
//! The C implementation builds BULL-method range data from SIS networks and
//! then computes next-state images with a recursive BDD cofactor algorithm. The
//! actual `network_t`, `node_t`, `st_table`, `array_t`, and `bdd_t` integration
//! remains blocked on other SIS ports, so those entry points return explicit
//! dependency errors. The cache-key and support-partition behavior is modeled
//! with owned Rust data and covered by tests.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub note: &'static str,
}

pub const REQUIRED_BULL_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        note: "array_t allocation, fetch, insert, and ownership used throughout bull.c",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.71",
        source_file: "LogicSynthesis/sis/bdd_cmu/bdd_port/bddport.c",
        note: "BDD manager, constants, variables, equality, size, leq, and top-var access",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.78",
        source_file: "LogicSynthesis/sis/bdd_ucb/bdd_cofactor.c",
        note: "bdd_cofactor used by bull_compute_next_states and bull_cofactor recursion",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.89",
        source_file: "LogicSynthesis/sis/bdd_ucb/bdd_substit.c",
        note: "bdd_substitute and bdd_compose used when reusing BULL cache entries",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.90",
        source_file: "LogicSynthesis/sis/bdd_ucb/bdd_support.c",
        note: "bdd_get_support used to partition recursive cofactor work",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.230",
        source_file: "LogicSynthesis/sis/latch/latch.c",
        note: "network_latch_end maps next-state outputs to present-state inputs",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "primary-input iteration and network/node lookup for range-data allocation",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "node_t identity and BDD attachment lifetime",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.326",
        source_file: "LogicSynthesis/sis/ntbdd/bdd_at_node.c",
        note: "ntbdd_at_node and ntbdd_free_at_node",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.329",
        source_file: "LogicSynthesis/sis/ntbdd/manager.c",
        note: "ntbdd_start_manager and ntbdd_end_manager",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.330",
        source_file: "LogicSynthesis/sis/ntbdd/node_to_bdd.c",
        note: "ntbdd_node_to_bdd for initial state, outputs, and next-state functions",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.424",
        source_file: "LogicSynthesis/sis/seqbdd/bull_util.c",
        note: "input_cofactor, disjoint_support_functions, and range_2_compute helpers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.442",
        source_file: "LogicSynthesis/sis/seqbdd/verif_util.c",
        note: "order_nodes, get_remaining_po, from_array_to_table, report_inconsistency",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        note: "st_table maps for PI ordering and the BULL cache",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.518",
        source_file: "LogicSynthesis/sis/var_set/var_set.c",
        note: "var_set support arithmetic used by bull_cofactor",
    },
];

pub fn required_bull_dependencies() -> &'static [PortDependency] {
    REQUIRED_BULL_DEPENDENCIES
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BullPortDisposition {
    CacheAndPlannerPortedSisBddIntegrationBlocked,
}

pub fn bull_port_disposition() -> BullPortDisposition {
    BullPortDisposition::CacheAndPlannerPortedSisBddIntegrationBlocked
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BullError {
    MissingSisDependencies {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    EmptyTransitionSet,
    VariableOutOfRange {
        variable: usize,
        n_vars: usize,
    },
    CacheHashModulusZero,
}

impl fmt::Display for BullError {
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
            Self::EmptyTransitionSet => write!(f, "BULL transition set is empty"),
            Self::VariableOutOfRange { variable, n_vars } => {
                write!(
                    f,
                    "BULL support variable {variable} is outside manager range 0..{n_vars}"
                )
            }
            Self::CacheHashModulusZero => write!(f, "BULL cache hash modulus must be nonzero"),
        }
    }
}

impl Error for BullError {}

pub fn bull_alloc_range_data_blocked() -> Result<(), BullError> {
    missing_dependencies("bull_alloc_range_data")
}

pub fn bull_compute_next_states_blocked() -> Result<(), BullError> {
    missing_dependencies("bull_compute_next_states")
}

pub fn bull_compute_reverse_image_blocked() -> Result<(), BullError> {
    missing_dependencies("bull_compute_reverse_image")
}

pub fn bull_check_output_blocked() -> Result<(), BullError> {
    missing_dependencies("bull_check_output")
}

pub fn bull_bdd_sizes_blocked() -> Result<(), BullError> {
    missing_dependencies("bull_bdd_sizes")
}

fn missing_dependencies(operation: &'static str) -> Result<(), BullError> {
    Err(BullError::MissingSisDependencies {
        operation,
        dependencies: REQUIRED_BULL_DEPENDENCIES,
    })
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddSignature {
    pub canonical_id: usize,
    pub top_var_id: usize,
    pub complemented: bool,
}

impl BddSignature {
    pub fn new(canonical_id: usize, top_var_id: usize, complemented: bool) -> Self {
        Self {
            canonical_id,
            top_var_id,
            complemented,
        }
    }

    pub fn complement(self) -> Self {
        Self {
            complemented: !self.complemented,
            ..self
        }
    }

    pub fn equal_or_complement(self, other: Self) -> bool {
        self.canonical_id == other.canonical_id
    }

    pub fn is_complement_of(self, other: Self) -> bool {
        self.canonical_id == other.canonical_id && self.complemented != other.complemented
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BullCacheKey {
    pub functions: Vec<Option<BddSignature>>,
}

impl BullCacheKey {
    pub fn new(functions: impl Into<Vec<Option<BddSignature>>>) -> Self {
        Self {
            functions: functions.into(),
        }
    }

    pub fn matches_c_key_cmp(&self, other: &Self) -> bool {
        self.functions.len() == other.functions.len()
            && self
                .functions
                .iter()
                .zip(&other.functions)
                .all(|(left, right)| match (*left, *right) {
                    (None, None) => true,
                    (Some(left), Some(right)) => left.equal_or_complement(right),
                    _ => false,
                })
    }

    pub fn hash_mod(&self, modulus: usize) -> Result<usize, BullError> {
        if modulus == 0 {
            return Err(BullError::CacheHashModulusZero);
        }

        let mut result = 0usize;
        for function in self.functions.iter().flatten() {
            result = (result << 1).wrapping_add(function.top_var_id);
        }
        Ok(result % modulus)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BullCacheEntry<T> {
    pub key: BullCacheKey,
    pub input_variables: Vec<usize>,
    pub range: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BullCacheLookupPlan<T> {
    pub entry_index: usize,
    pub substituted_range: T,
    pub substitutions: Vec<(usize, usize)>,
    pub complemented_inputs: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BullCache<T> {
    entries: Vec<BullCacheEntry<T>>,
}

impl<T> Default for BullCache<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T: Clone> BullCache<T> {
    pub fn insert(&mut self, key: BullCacheKey, input_variables: Vec<usize>, range: T) {
        self.entries.push(BullCacheEntry {
            key,
            input_variables,
            range,
        });
    }

    pub fn lookup_plan(
        &self,
        key: &BullCacheKey,
        input_variables: &[usize],
    ) -> Option<BullCacheLookupPlan<T>> {
        self.entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.key.matches_c_key_cmp(key))
            .map(|(entry_index, entry)| {
                let substitutions = entry
                    .input_variables
                    .iter()
                    .copied()
                    .zip(input_variables.iter().copied())
                    .collect();
                let complemented_inputs = entry
                    .key
                    .functions
                    .iter()
                    .zip(&key.functions)
                    .zip(input_variables.iter().copied())
                    .filter_map(|((cached, requested), input)| match (*cached, *requested) {
                        (Some(cached), Some(requested)) if cached.is_complement_of(requested) => {
                            Some(input)
                        }
                        _ => None,
                    })
                    .collect();

                BullCacheLookupPlan {
                    entry_index,
                    substituted_range: entry.range.clone(),
                    substitutions,
                    complemented_inputs,
                }
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FunctionClass {
    Nil,
    Zero,
    One,
    NonConstant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstantLiteral {
    pub input_variable: usize,
    pub positive: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BullCofactorTerminal {
    ConstantCube(Vec<ConstantLiteral>),
    NeedsRecursiveCofactor(BullRecursionChoice),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BullRecursionChoice {
    InputCofactorOnCommonSupport,
    DisjointSupportPartition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BullCofactorPlan {
    pub active_count: usize,
    pub constant_literals: Vec<ConstantLiteral>,
    pub common_support: BTreeSet<usize>,
    pub partitions: Vec<BTreeSet<usize>>,
    pub terminal: BullCofactorTerminal,
}

pub fn plan_bull_cofactor(
    functions: &[FunctionClass],
    input_variables: &[usize],
    supports: &[BTreeSet<usize>],
    n_vars: usize,
) -> Result<BullCofactorPlan, BullError> {
    if functions.is_empty() {
        return Err(BullError::EmptyTransitionSet);
    }
    assert_eq!(
        functions.len(),
        input_variables.len(),
        "one input variable is required for each BULL transition function"
    );

    let mut active_supports = Vec::new();
    let mut constant_literals = Vec::new();

    for (index, function) in functions.iter().copied().enumerate() {
        match function {
            FunctionClass::Nil => {}
            FunctionClass::One => constant_literals.push(ConstantLiteral {
                input_variable: input_variables[index],
                positive: true,
            }),
            FunctionClass::Zero => constant_literals.push(ConstantLiteral {
                input_variable: input_variables[index],
                positive: false,
            }),
            FunctionClass::NonConstant => {
                let support = supports.get(index).cloned().unwrap_or_default();
                validate_support(&support, n_vars)?;
                active_supports.push((index, support));
            }
        }
    }

    if active_supports.len() <= 1 {
        return Ok(BullCofactorPlan {
            active_count: active_supports.len(),
            constant_literals: constant_literals.clone(),
            common_support: BTreeSet::new(),
            partitions: Vec::new(),
            terminal: BullCofactorTerminal::ConstantCube(constant_literals),
        });
    }

    let common_support = intersect_supports(active_supports.iter().map(|(_, support)| support));
    let partitions = disjoint_support_function_partitions(
        active_supports
            .iter()
            .map(|(index, support)| (*index, support.clone())),
    );
    let choice = if !common_support.is_empty() && common_support.len() < active_supports.len() / 4 {
        BullRecursionChoice::InputCofactorOnCommonSupport
    } else {
        BullRecursionChoice::DisjointSupportPartition
    };

    Ok(BullCofactorPlan {
        active_count: active_supports.len(),
        constant_literals,
        common_support,
        partitions,
        terminal: BullCofactorTerminal::NeedsRecursiveCofactor(choice),
    })
}

pub fn disjoint_support_function_partitions(
    supports: impl IntoIterator<Item = (usize, BTreeSet<usize>)>,
) -> Vec<BTreeSet<usize>> {
    let mut components: Vec<(BTreeSet<usize>, BTreeSet<usize>)> = Vec::new();

    for (function_index, support) in supports {
        let mut function_indices = BTreeSet::from([function_index]);
        let mut variables = support;
        let mut cursor = 0;

        while cursor < components.len() {
            if variables.is_disjoint(&components[cursor].1) {
                cursor += 1;
            } else {
                let (other_functions, other_variables) = components.remove(cursor);
                function_indices.extend(other_functions);
                variables.extend(other_variables);
                cursor = 0;
            }
        }

        components.push((function_indices, variables));
    }

    let mut partitions: Vec<_> = components
        .into_iter()
        .map(|(function_indices, _)| function_indices)
        .collect();
    partitions.sort_by_key(|partition| partition.first().copied());
    partitions
}

fn validate_support(support: &BTreeSet<usize>, n_vars: usize) -> Result<(), BullError> {
    if let Some(variable) = support.iter().copied().find(|variable| *variable >= n_vars) {
        return Err(BullError::VariableOutOfRange { variable, n_vars });
    }
    Ok(())
}

fn intersect_supports<'a>(
    mut supports: impl Iterator<Item = &'a BTreeSet<usize>>,
) -> BTreeSet<usize> {
    let Some(first) = supports.next() else {
        return BTreeSet::new();
    };
    supports.fold(first.clone(), |intersection, support| {
        intersection.intersection(support).copied().collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[usize]) -> BTreeSet<usize> {
        values.iter().copied().collect()
    }

    #[test]
    fn dependency_errors_name_bull_operations_and_blockers() {
        let error = bull_compute_next_states_blocked().unwrap_err();
        match error {
            BullError::MissingSisDependencies {
                operation,
                dependencies,
            } => {
                assert_eq!(operation, "bull_compute_next_states");
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.424"
                        && dependency.source_file == "LogicSynthesis/sis/seqbdd/bull_util.c"
                }));
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.442"
                        && dependency.source_file == "LogicSynthesis/sis/seqbdd/verif_util.c"
                }));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn cache_key_matches_functions_equal_up_to_complement() {
        let f0 = BddSignature::new(10, 2, false);
        let f1 = BddSignature::new(11, 5, false);
        let key = BullCacheKey::new(vec![Some(f0), None, Some(f1)]);
        let complement_key = BullCacheKey::new(vec![Some(f0.complement()), None, Some(f1)]);
        let different_key =
            BullCacheKey::new(vec![Some(f0), None, Some(BddSignature::new(12, 5, false))]);

        assert!(key.matches_c_key_cmp(&complement_key));
        assert!(!key.matches_c_key_cmp(&different_key));
    }

    #[test]
    fn cache_hash_uses_top_variables_and_skips_nil_entries() {
        let key = BullCacheKey::new(vec![
            Some(BddSignature::new(10, 2, false)),
            None,
            Some(BddSignature::new(11, 5, true)),
        ]);

        assert_eq!(key.hash_mod(17).unwrap(), ((2usize << 1) + 5) % 17);
        assert_eq!(
            key.hash_mod(0).unwrap_err(),
            BullError::CacheHashModulusZero
        );
    }

    #[test]
    fn cache_lookup_records_substitutions_and_complemented_inputs() {
        let f0 = BddSignature::new(10, 2, false);
        let f1 = BddSignature::new(11, 5, false);
        let mut cache = BullCache::default();
        cache.insert(
            BullCacheKey::new(vec![Some(f0), Some(f1)]),
            vec![100, 101],
            "range",
        );

        let plan = cache
            .lookup_plan(
                &BullCacheKey::new(vec![Some(f0.complement()), Some(f1)]),
                &[200, 201],
            )
            .unwrap();

        assert_eq!(plan.entry_index, 0);
        assert_eq!(plan.substituted_range, "range");
        assert_eq!(plan.substitutions, vec![(100, 200), (101, 201)]);
        assert_eq!(plan.complemented_inputs, vec![200]);
    }

    #[test]
    fn support_partitions_merge_transitive_overlaps() {
        let partitions = disjoint_support_function_partitions([
            (0, set(&[1, 2])),
            (1, set(&[3])),
            (2, set(&[2, 4])),
            (3, set(&[5])),
            (4, set(&[4, 6])),
        ]);

        assert_eq!(partitions, vec![set(&[0, 2, 4]), set(&[1]), set(&[3])]);
    }

    #[test]
    fn cofactor_plan_extracts_constant_cube_when_at_most_one_function_remains() {
        let plan = plan_bull_cofactor(
            &[
                FunctionClass::One,
                FunctionClass::Nil,
                FunctionClass::Zero,
                FunctionClass::NonConstant,
            ],
            &[10, 11, 12, 13],
            &[set(&[]), set(&[]), set(&[]), set(&[2])],
            4,
        )
        .unwrap();

        assert_eq!(plan.active_count, 1);
        assert_eq!(
            plan.terminal,
            BullCofactorTerminal::ConstantCube(vec![
                ConstantLiteral {
                    input_variable: 10,
                    positive: true,
                },
                ConstantLiteral {
                    input_variable: 12,
                    positive: false,
                },
            ])
        );
    }

    #[test]
    fn cofactor_plan_prefers_input_cofactor_for_small_common_support() {
        let functions = vec![FunctionClass::NonConstant; 8];
        let inputs: Vec<_> = (0..8).collect();
        let supports = vec![
            set(&[0, 1]),
            set(&[0, 2]),
            set(&[0, 3]),
            set(&[0, 4]),
            set(&[0, 5]),
            set(&[0, 6]),
            set(&[0, 7]),
            set(&[0, 8]),
        ];

        let plan = plan_bull_cofactor(&functions, &inputs, &supports, 9).unwrap();

        assert_eq!(plan.common_support, set(&[0]));
        assert_eq!(
            plan.terminal,
            BullCofactorTerminal::NeedsRecursiveCofactor(
                BullRecursionChoice::InputCofactorOnCommonSupport
            )
        );
    }

    #[test]
    fn cofactor_plan_uses_disjoint_support_partition_when_common_support_is_not_small() {
        let plan = plan_bull_cofactor(
            &[
                FunctionClass::NonConstant,
                FunctionClass::NonConstant,
                FunctionClass::NonConstant,
            ],
            &[0, 1, 2],
            &[set(&[0, 1]), set(&[1, 2]), set(&[4])],
            5,
        )
        .unwrap();

        assert_eq!(plan.partitions, vec![set(&[0, 1]), set(&[2])]);
        assert_eq!(
            plan.terminal,
            BullCofactorTerminal::NeedsRecursiveCofactor(
                BullRecursionChoice::DisjointSupportPartition
            )
        );
    }

    #[test]
    fn cofactor_plan_rejects_support_outside_manager_range() {
        let error =
            plan_bull_cofactor(&[FunctionClass::NonConstant], &[0], &[set(&[6])], 6).unwrap_err();

        assert_eq!(
            error,
            BullError::VariableOutOfRange {
                variable: 6,
                n_vars: 6,
            }
        );
    }
}
