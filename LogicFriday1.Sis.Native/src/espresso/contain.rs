//! Native Rust containment operations for Espresso set families.
//!
//! The routines keep the original algorithms' observable set-family behavior:
//! sort by cube size, discard duplicates, and remove contained or containing
//! rows. They operate on owned Rust values and do not expose per-file ABI
//! entry points.

use std::cmp::Ordering;

use super::set::{Set, SetFamily};

pub fn sf_contain(family: SetFamily) -> SetFamily
{
    let set_size = family.set_size();
    let mut sets = sorted_sets(family, SortDirection::Descending);
    remove_equal(&mut sets);
    remove_contained_by_larger(&mut sets);
    unlist(set_size, sets)
}

pub fn sf_rev_contain(family: SetFamily) -> SetFamily
{
    let set_size = family.set_size();
    let mut sets = sorted_sets(family, SortDirection::Ascending);
    remove_equal(&mut sets);
    remove_containing_smaller(&mut sets);
    unlist(set_size, sets)
}

pub fn sf_ind_contain(family: SetFamily, row_indices: &[usize]) -> (SetFamily, Vec<usize>)
{
    assert_eq!(
        family.len(),
        row_indices.len(),
        "row index count must match set family row count"
    );

    let set_size = family.set_size();
    let mut indexed = family
        .sets()
        .iter()
        .cloned()
        .zip(row_indices.iter().copied())
        .collect::<Vec<_>>();

    indexed.sort_by(|left, right| compare_sets(&left.0, &right.0, SortDirection::Descending));
    remove_equal_indexed(&mut indexed);
    remove_contained_by_larger_indexed(&mut indexed);

    let retained_indices = indexed.iter().map(|(_, index)| *index).collect::<Vec<_>>();
    let retained_sets = indexed.into_iter().map(|(set, _)| set).collect::<Vec<_>>();

    (SetFamily::from_sets(set_size, retained_sets), retained_indices)
}

pub fn sf_dupl(family: SetFamily) -> SetFamily
{
    let set_size = family.set_size();
    let mut sets = sorted_sets(family, SortDirection::Descending);
    remove_equal(&mut sets);
    unlist(set_size, sets)
}

pub fn sf_union(left: SetFamily, right: SetFamily) -> SetFamily
{
    assert_eq!(
        left.set_size(),
        right.set_size(),
        "set family sizes must match"
    );

    let set_size = left.set_size();
    let mut sets = left
        .sets()
        .iter()
        .chain(right.sets())
        .cloned()
        .collect::<Vec<_>>();
    sets.sort_by(|left, right| compare_sets(left, right, SortDirection::Descending));
    remove_equal(&mut sets);
    remove_contained_by_larger(&mut sets);

    SetFamily::from_sets(set_size, sets)
}

pub fn dist_merge(family: SetFamily, mask: &Set) -> SetFamily
{
    assert_eq!(
        family.set_size(),
        mask.element_count(),
        "mask size must match set family size"
    );

    let set_size = family.set_size();
    let mut sets = family.sets().to_vec();
    sets.sort_by(|left, right| compare_masked_sets(left, right, mask));

    let mut merged = Vec::new();
    let mut current = None::<Set>;
    for set in sets
    {
        match current.take()
        {
            Some(accumulator)
                if compare_masked_sets(&accumulator, &set, mask) == Ordering::Equal =>
            {
                current = Some(accumulator.union(&set));
            }
            Some(accumulator) =>
            {
                merged.push(accumulator);
                current = Some(set);
            }
            None =>
            {
                current = Some(set);
            }
        }
    }

    if let Some(accumulator) = current
    {
        merged.push(accumulator);
    }

    SetFamily::from_sets(set_size, merged)
}

pub fn d1merge(family: SetFamily, variable_mask: &Set) -> SetFamily
{
    dist_merge(family, variable_mask)
}

fn sorted_sets(family: SetFamily, direction: SortDirection) -> Vec<Set>
{
    let mut sets = family.sets().to_vec();
    sets.sort_by(|left, right| compare_sets(left, right, direction));
    sets
}

fn unlist(set_size: usize, sets: Vec<Set>) -> SetFamily
{
    SetFamily::from_sets(set_size, sets)
}

fn remove_equal(sets: &mut Vec<Set>)
{
    sets.dedup_by(|left, right| same_elements(left, right));
}

fn remove_equal_indexed(sets: &mut Vec<(Set, usize)>)
{
    sets.dedup_by(|left, right| same_elements(&left.0, &right.0));
}

fn remove_contained_by_larger(sets: &mut Vec<Set>)
{
    let original = sets.clone();
    sets.retain(|set| {
        !original
            .iter()
            .any(|candidate| !same_elements(candidate, set) && set.implies(candidate))
    });
}

fn remove_contained_by_larger_indexed(sets: &mut Vec<(Set, usize)>)
{
    let original = sets.clone();
    sets.retain(|(set, _)| {
        !original.iter().any(|(candidate, _)| {
            !same_elements(candidate, set) && set.implies(candidate)
        })
    });
}

fn remove_containing_smaller(sets: &mut Vec<Set>)
{
    let original = sets.clone();
    sets.retain(|set| {
        !original
            .iter()
            .any(|candidate| !same_elements(candidate, set) && candidate.implies(set))
    });
}

fn compare_sets(left: &Set, right: &Set, direction: SortDirection) -> Ordering
{
    match left.cardinality().cmp(&right.cardinality())
    {
        Ordering::Greater => direction.large_first(),
        Ordering::Less => direction.small_first(),
        Ordering::Equal => compare_lexicographic(left, right, direction),
    }
}

fn compare_masked_sets(left: &Set, right: &Set, mask: &Set) -> Ordering
{
    let masked_left = left.union(mask);
    let masked_right = right.union(mask);
    compare_lexicographic(&masked_left, &masked_right, SortDirection::Descending)
}

fn compare_lexicographic(left: &Set, right: &Set, direction: SortDirection) -> Ordering
{
    assert_eq!(
        left.element_count(),
        right.element_count(),
        "set sizes must match"
    );

    for element in (0..left.element_count()).rev()
    {
        match (left.contains(element), right.contains(element))
        {
            (true, false) => return direction.large_first(),
            (false, true) => return direction.small_first(),
            _ => {}
        }
    }

    Ordering::Equal
}

fn same_elements(left: &Set, right: &Set) -> bool
{
    left.element_count() == right.element_count() && left.implies(right) && right.implies(left)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SortDirection
{
    Ascending,
    Descending,
}

impl SortDirection
{
    const fn large_first(self) -> Ordering
    {
        match self
        {
            Self::Ascending => Ordering::Greater,
            Self::Descending => Ordering::Less,
        }
    }

    const fn small_first(self) -> Ordering
    {
        match self
        {
            Self::Ascending => Ordering::Less,
            Self::Descending => Ordering::Greater,
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn set(size: usize, elements: &[usize]) -> Set
    {
        Set::from_elements(size, elements.iter().copied())
    }

    fn family(size: usize, rows: &[&[usize]]) -> SetFamily
    {
        SetFamily::from_sets(size, rows.iter().map(|row| set(size, row)))
    }

    fn rows(family: &SetFamily) -> Vec<Vec<usize>>
    {
        family
            .sets()
            .iter()
            .map(|set| set.elements().collect())
            .collect()
    }

    #[test]
    fn contain_removes_duplicates_and_subsets_in_decreasing_size_order()
    {
        let result = sf_contain(family(8, &[&[0, 1], &[0], &[2, 3, 4], &[0, 1], &[3]]));

        assert_eq!(rows(&result), vec![vec![2, 3, 4], vec![0, 1]]);
    }

    #[test]
    fn reverse_contain_keeps_minimal_sets_in_increasing_size_order()
    {
        let result = sf_rev_contain(family(8, &[&[0, 1], &[0], &[2, 3, 4], &[3], &[3]]));

        assert_eq!(rows(&result), vec![vec![0], vec![3]]);
    }

    #[test]
    fn indexed_contain_reports_retained_source_rows_after_sorting()
    {
        let (result, indices) = sf_ind_contain(
            family(8, &[&[0], &[2, 3, 4], &[0, 1], &[3]]),
            &[10, 11, 12, 13],
        );

        assert_eq!(rows(&result), vec![vec![2, 3, 4], vec![0, 1]]);
        assert_eq!(indices, vec![11, 12]);
    }

    #[test]
    fn duplicate_removal_keeps_one_copy_without_containment()
    {
        let result = sf_dupl(family(8, &[&[0], &[0, 1], &[0], &[2]]));

        assert_eq!(rows(&result), vec![vec![0, 1], vec![2], vec![0]]);
    }

    #[test]
    fn union_removes_cross_family_duplicates_and_contained_sets()
    {
        let left = family(8, &[&[0, 1], &[4]]);
        let right = family(8, &[&[0, 1], &[0], &[2, 3]]);

        let result = sf_union(left, right);

        assert_eq!(rows(&result), vec![vec![2, 3], vec![0, 1], vec![4]]);
    }

    #[test]
    fn distance_merge_unions_rows_that_are_equal_under_mask()
    {
        let result = dist_merge(
            family(6, &[&[0, 2], &[0, 3], &[1, 4]]),
            &set(6, &[2, 3]),
        );

        assert_eq!(rows(&result), vec![vec![1, 4], vec![0, 2, 3]]);
    }

    #[test]
    fn d1merge_uses_the_supplied_variable_mask()
    {
        let result = d1merge(
            family(6, &[&[0, 2], &[0, 3], &[0, 4]]),
            &set(6, &[2, 3]),
        );

        assert_eq!(rows(&result), vec![vec![0, 4], vec![0, 2, 3]]);
    }

    #[test]
    fn source_contains_no_dependency_metadata_or_c_abi_shims()
    {
        let source = include_str!("contain.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
