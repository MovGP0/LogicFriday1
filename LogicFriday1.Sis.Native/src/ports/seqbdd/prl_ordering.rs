//! Native Rust model for `LogicSynthesis/sis/seqbdd/prl_ordering.c`.
//!
//! The C file implements `Prl_OrderSetHeuristic`, a branch-and-bound ordering
//! heuristic for sets used by parallel range computation. This module ports the
//! owned set-ordering behavior without preserving the legacy C ABI. Direct SIS
//! `array_t`, `st_table`, and `var_set_t` interop remains blocked until the
//! corresponding native ports are available.

use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

const LARGE_NUMBER: i32 = 0x1fff_ffff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub note: &'static str,
}

pub const REQUIRED_PRL_ORDERING_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        note: "legacy array_t allocation, insertion, fetch, sorting, and ownership",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        note: "legacy st_table cache keyed by var_set_t values",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.518",
        source_file: "LogicSynthesis/sis/var_set/var_set.c",
        note: "legacy var_set_t bitsets, hashing, comparison, and set operations",
    },
];

pub fn required_prl_ordering_dependencies() -> &'static [PortDependency] {
    REQUIRED_PRL_ORDERING_DEPENDENCIES
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrlOrderingError {
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    EmptySetCollection,
    SetVariableOutOfRange {
        set_index: usize,
        variable: usize,
        n_vars: usize,
    },
    InvalidOrdering {
        index: usize,
        n_sets: usize,
    },
    DuplicateOrderingIndex(usize),
}

impl fmt::Display for PrlOrderingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires native Rust ports for {} SIS dependencies",
                dependencies.len()
            ),
            Self::EmptySetCollection => write!(f, "set ordering requires at least one set"),
            Self::SetVariableOutOfRange {
                set_index,
                variable,
                n_vars,
            } => write!(
                f,
                "set {set_index} contains variable {variable} outside range 0..{n_vars}"
            ),
            Self::InvalidOrdering { index, n_sets } => {
                write!(f, "ordering index {index} is outside range 0..{n_sets}")
            }
            Self::DuplicateOrderingIndex(index) => {
                write!(f, "ordering contains duplicate set index {index}")
            }
        }
    }
}

impl Error for PrlOrderingError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetInfo {
    n_vars: usize,
    sets: Vec<BTreeSet<usize>>,
}

impl SetInfo {
    pub fn new(
        n_vars: usize,
        sets: impl IntoIterator<Item = impl IntoIterator<Item = usize>>,
    ) -> Result<Self, PrlOrderingError> {
        let sets = sets
            .into_iter()
            .enumerate()
            .map(|(set_index, set)| {
                let set = set.into_iter().collect::<BTreeSet<_>>();
                for variable in &set {
                    if *variable >= n_vars {
                        return Err(PrlOrderingError::SetVariableOutOfRange {
                            set_index,
                            variable: *variable,
                            n_vars,
                        });
                    }
                }
                Ok(set)
            })
            .collect::<Result<Vec<_>, _>>()?;

        if sets.is_empty() {
            return Err(PrlOrderingError::EmptySetCollection);
        }

        Ok(Self { n_vars, sets })
    }

    pub fn n_vars(&self) -> usize {
        self.n_vars
    }

    pub fn n_sets(&self) -> usize {
        self.sets.len()
    }

    pub fn sets(&self) -> &[BTreeSet<usize>] {
        &self.sets
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderingResult {
    pub order: Vec<usize>,
    pub cost: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SetValue {
    index: usize,
    cost: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Dominator {
    index: usize,
    bound: i32,
    cost: i32,
    size: usize,
}

pub fn prl_order_set_heuristic(
    info: &SetInfo,
    level: i32,
) -> Result<OrderingResult, PrlOrderingError> {
    let placed_so_far = BTreeSet::new();
    let mut cache = HashMap::new();
    let cost = do_find_best_set_order(info, placed_so_far.clone(), &mut cache, level, LARGE_NUMBER);
    let order = extract_best_order_from_cache(info, &cache, &placed_so_far);
    let recomputed_cost = compute_order_cost(info, &order)?;
    debug_assert_eq!(cost, recomputed_cost);

    Ok(OrderingResult {
        order,
        cost: recomputed_cost,
    })
}

pub fn prl_order_set_heuristic_from_sis() -> Result<OrderingResult, PrlOrderingError> {
    Err(PrlOrderingError::MissingNativePorts {
        operation: "Prl_OrderSetHeuristic SIS array_t/var_set_t entry",
        dependencies: REQUIRED_PRL_ORDERING_DEPENDENCIES,
    })
}

pub fn compute_order_cost(info: &SetInfo, order: &[usize]) -> Result<i32, PrlOrderingError> {
    let mut seen = BTreeSet::new();
    let mut union_so_far = BTreeSet::new();
    let mut result = 0;

    for index in order {
        if *index >= info.n_sets() {
            return Err(PrlOrderingError::InvalidOrdering {
                index: *index,
                n_sets: info.n_sets(),
            });
        }
        if !seen.insert(*index) {
            return Err(PrlOrderingError::DuplicateOrderingIndex(*index));
        }
        union_so_far.extend(info.sets[*index].iter().copied());
        result += union_so_far.len() as i32;
    }

    Ok(result)
}

fn do_find_best_set_order(
    info: &SetInfo,
    placed_so_far: BTreeSet<usize>,
    cache: &mut HashMap<BTreeSet<usize>, SetValue>,
    depth: i32,
    allocated: i32,
) -> i32 {
    let n_remaining_sets = info.n_sets() - placed_so_far.len();
    if n_remaining_sets == 0 {
        return 0;
    }
    if allocated < 0 {
        return LARGE_NUMBER;
    }
    if let Some(value) = cache.get(&placed_so_far) {
        return value.cost;
    }

    let remaining_vars = extract_uncovered_variables(info, &placed_so_far);
    let dead_vars = complement(info.n_vars(), &remaining_vars);
    let mut dominators = if depth <= 0 {
        let (index, size) = arg_min_remaining_size(info, &placed_so_far, &dead_vars);
        vec![Dominator {
            index,
            bound: 0,
            cost: LARGE_NUMBER,
            size,
        }]
    } else {
        let mut dominators = extract_dominators(info, &placed_so_far, &remaining_vars);
        if dominators.len() > 1 {
            dominators.sort_by_key(|dominator| dominator.size);
            compute_bounds(info, &placed_so_far, &remaining_vars, &mut dominators);
        }
        dominators
    };

    debug_assert!(!dominators.is_empty());

    let mut value = SetValue {
        index: usize::MAX,
        cost: LARGE_NUMBER,
    };

    for dominator in &mut dominators {
        if dominator.bound >= value.cost {
            continue;
        }

        let local_cost = intersection_len(&info.sets[dominator.index], &remaining_vars) as i32
            * n_remaining_sets as i32;
        let allocated = value.cost - local_cost;
        let mut new_key = placed_so_far.clone();
        new_key.insert(dominator.index);
        dominator.cost =
            local_cost + do_find_best_set_order(info, new_key, cache, depth - 1, allocated);

        if value.cost > dominator.cost {
            value.cost = dominator.cost;
            value.index = dominator.index;
        }
    }

    debug_assert_ne!(value.index, usize::MAX);
    debug_assert!(!placed_so_far.contains(&value.index));
    let cost = value.cost;
    cache.insert(placed_so_far, value);
    cost
}

fn extract_best_order_from_cache(
    info: &SetInfo,
    cache: &HashMap<BTreeSet<usize>, SetValue>,
    first_key: &BTreeSet<usize>,
) -> Vec<usize> {
    let mut placed_so_far = first_key.clone();
    let mut result = Vec::with_capacity(info.n_sets());

    for _ in 0..info.n_sets() {
        let Some(value) = cache.get(&placed_so_far) else {
            break;
        };
        debug_assert!(value.index < info.n_sets());
        debug_assert!(!placed_so_far.contains(&value.index));
        result.push(value.index);
        placed_so_far.insert(value.index);
    }

    if result.len() < info.n_sets() {
        for index in 0..info.n_sets() {
            if !placed_so_far.contains(&index) {
                result.push(index);
                placed_so_far.insert(index);
            }
        }
    }

    result
}

fn compute_bounds(
    info: &SetInfo,
    placed_so_far: &BTreeSet<usize>,
    mask: &BTreeSet<usize>,
    dominators: &mut [Dominator],
) {
    let n_remaining = info.n_sets() - placed_so_far.len();

    for dominator in dominators {
        dominator.bound = (dominator.size * n_remaining) as i32;
        let mut other_remaining_union = BTreeSet::new();
        for (index, set) in info.sets.iter().enumerate() {
            if index != dominator.index && !placed_so_far.contains(&index) {
                other_remaining_union.extend(set.iter().copied());
            }
        }
        let masked_other_without_dominator = other_remaining_union
            .intersection(mask)
            .filter(|variable| !info.sets[dominator.index].contains(variable))
            .count();
        dominator.bound += masked_other_without_dominator as i32;
    }
}

fn extract_dominators(
    info: &SetInfo,
    placed_so_far: &BTreeSet<usize>,
    mask: &BTreeSet<usize>,
) -> Vec<Dominator> {
    let mut dominator_indices = BTreeSet::new();

    for i in 0..info.n_sets() {
        if placed_so_far.contains(&i) {
            continue;
        }
        let mut is_dominator = true;
        for j in 0..info.n_sets() {
            if j == i || placed_so_far.contains(&j) {
                continue;
            }
            if set_is_less_than(&info.sets[j], &info.sets[i], mask, j as isize - i as isize) {
                is_dominator = false;
                break;
            }
        }
        if is_dominator {
            dominator_indices.insert(i);
        }
    }

    dominator_indices
        .into_iter()
        .map(|index| Dominator {
            index,
            size: intersection_len(mask, &info.sets[index]),
            cost: LARGE_NUMBER,
            bound: 0,
        })
        .collect()
}

fn set_is_less_than(
    set1: &BTreeSet<usize>,
    set2: &BTreeSet<usize>,
    mask: &BTreeSet<usize>,
    diff: isize,
) -> bool {
    let masked1 = set1.intersection(mask).copied().collect::<BTreeSet<_>>();
    let masked2 = set2.intersection(mask).copied().collect::<BTreeSet<_>>();

    if masked1 == masked2 {
        diff < 0
    } else {
        masked1.is_subset(&masked2)
    }
}

fn extract_uncovered_variables(info: &SetInfo, placed_so_far: &BTreeSet<usize>) -> BTreeSet<usize> {
    let covered = placed_so_far
        .iter()
        .flat_map(|index| info.sets[*index].iter().copied())
        .collect::<BTreeSet<_>>();

    complement(info.n_vars(), &covered)
}

fn arg_min_remaining_size(
    info: &SetInfo,
    placed_so_far: &BTreeSet<usize>,
    mask: &BTreeSet<usize>,
) -> (usize, usize) {
    let mut best_index = usize::MAX;
    let mut best_size = usize::MAX;

    for (index, set) in info.sets.iter().enumerate() {
        if placed_so_far.contains(&index) {
            continue;
        }
        let size = mask.union(set).count();
        if size < best_size {
            best_size = size;
            best_index = index;
        }
    }

    (best_index, best_size - mask.len())
}

fn complement(n_vars: usize, set: &BTreeSet<usize>) -> BTreeSet<usize> {
    (0..n_vars)
        .filter(|variable| !set.contains(variable))
        .collect()
}

fn intersection_len(left: &BTreeSet<usize>, right: &BTreeSet<usize>) -> usize {
    left.intersection(right).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(n_vars: usize, sets: &[&[usize]]) -> SetInfo {
        SetInfo::new(n_vars, sets.iter().map(|set| set.iter().copied())).unwrap()
    }

    fn brute_force_best_order(info: &SetInfo) -> OrderingResult {
        let mut remaining = (0..info.n_sets()).collect::<Vec<_>>();
        let mut best = OrderingResult {
            order: Vec::new(),
            cost: LARGE_NUMBER,
        };
        permute(info, &mut Vec::new(), &mut remaining, &mut best);
        best
    }

    fn permute(
        info: &SetInfo,
        prefix: &mut Vec<usize>,
        remaining: &mut Vec<usize>,
        best: &mut OrderingResult,
    ) {
        if remaining.is_empty() {
            let cost = compute_order_cost(info, prefix).unwrap();
            if cost < best.cost {
                best.cost = cost;
                best.order = prefix.clone();
            }
            return;
        }

        for index in 0..remaining.len() {
            let next = remaining.remove(index);
            prefix.push(next);
            permute(info, prefix, remaining, best);
            prefix.pop();
            remaining.insert(index, next);
        }
    }

    #[test]
    fn branch_and_bound_matches_bruteforce_for_overlapping_sets() {
        let info = info(5, &[&[0, 1, 4], &[1, 2], &[2, 3], &[0, 3]]);
        let result = prl_order_set_heuristic(&info, 4).unwrap();
        let brute_force = brute_force_best_order(&info);

        assert_eq!(result.cost, brute_force.cost);
        assert_eq!(compute_order_cost(&info, &result.order), Ok(result.cost));
        assert_eq!(result.order.len(), info.n_sets());
    }

    #[test]
    fn depth_zero_uses_minimum_incremental_size_choice() {
        let info = info(5, &[&[0, 1, 2], &[0], &[3, 4]]);
        let result = prl_order_set_heuristic(&info, 0).unwrap();

        assert_eq!(result.order, vec![1, 0, 2]);
        assert_eq!(result.cost, 9);
    }

    #[test]
    fn equivalent_sets_break_ties_by_lowest_index() {
        let info = info(3, &[&[0, 1], &[0, 1], &[0, 1]]);
        let result = prl_order_set_heuristic(&info, 3).unwrap();

        assert_eq!(result.order[0], 0);
        assert_eq!(result.cost, brute_force_best_order(&info).cost);
    }

    #[test]
    fn subset_dominator_rule_places_smaller_set_first() {
        let info = info(4, &[&[0], &[0, 1, 2], &[3]]);
        let dominators =
            extract_dominators(&info, &BTreeSet::new(), &complement(4, &BTreeSet::new()));

        assert_eq!(
            dominators
                .iter()
                .map(|dominator| dominator.index)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
    }

    #[test]
    fn compute_order_cost_rejects_invalid_permutations() {
        let info = info(2, &[&[0], &[1]]);

        assert_eq!(
            compute_order_cost(&info, &[0, 2]),
            Err(PrlOrderingError::InvalidOrdering {
                index: 2,
                n_sets: 2,
            })
        );
        assert_eq!(
            compute_order_cost(&info, &[1, 1]),
            Err(PrlOrderingError::DuplicateOrderingIndex(1))
        );
    }

    #[test]
    fn validates_set_dimensions() {
        assert_eq!(
            SetInfo::new(2, [vec![0, 2]]),
            Err(PrlOrderingError::SetVariableOutOfRange {
                set_index: 0,
                variable: 2,
                n_vars: 2,
            })
        );
        assert_eq!(
            SetInfo::new(2, Vec::<Vec<usize>>::new()),
            Err(PrlOrderingError::EmptySetCollection)
        );
    }

    #[test]
    fn sis_entry_reports_dependency_beads_and_sources() {
        let error = prl_order_set_heuristic_from_sis().unwrap_err();

        match error {
            PrlOrderingError::MissingNativePorts {
                operation,
                dependencies,
            } => {
                assert_eq!(
                    operation,
                    "Prl_OrderSetHeuristic SIS array_t/var_set_t entry"
                );
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.2"
                        && dependency.source_file == "LogicSynthesis/sis/array/array.c"
                }));
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.518"
                        && dependency.source_file == "LogicSynthesis/sis/var_set/var_set.c"
                }));
            }
            other => panic!("unexpected error: {other:?}"),
        }

        assert_eq!(
            required_prl_ordering_dependencies(),
            REQUIRED_PRL_ORDERING_DEPENDENCIES
        );
    }
}
