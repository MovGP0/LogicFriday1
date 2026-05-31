//! Source-aligned Rust surface for Espresso `setc.c`.
//!
//! `setc.c` is the cube-specialized layer on top of `set.c`.  This module keeps
//! that API traceable while using the shared native [`CubeContext`] and [`BitSet`]
//! representation from `foundation.rs`.

use std::cmp::Ordering;

use crate::foundation::{
    BitSet, CubeContext, ascend as foundation_ascend, d1_order as foundation_d1_order,
    desc1 as foundation_desc1, descend as foundation_descend, lex_order as foundation_lex_order,
};

#[allow(unused_imports)]
pub use crate::foundation::{CData, CubeConfig, DASH, DISJOINT, ONE, TWO, ZERO};

/// C `full_row`: true when `p | cof` is the cube fullset.
pub fn full_row(context: &CubeContext, p: &BitSet, cof: &BitSet) -> bool {
    context.full_row(p, cof)
}

/// C `cdist0`: true when two cubes intersect in every variable.
pub fn cdist0(context: &CubeContext, a: &BitSet, b: &BitSet) -> bool {
    context.cdist0(a, b)
}

/// C `cdist01`: cube distance, capped at 2 once it exceeds 1.
pub fn cdist01(context: &CubeContext, a: &BitSet, b: &BitSet) -> usize {
    context.cdist01(a, b)
}

/// C `cdist`: number of variables whose intersection is null.
pub fn cdist(context: &CubeContext, a: &BitSet, b: &BitSet) -> usize {
    context.cdist(a, b)
}

/// C `force_lower`: add variables from `a` that do not intersect `b`.
pub fn force_lower(context: &CubeContext, xlower: &mut BitSet, a: &BitSet, b: &BitSet) {
    context.force_lower(xlower, a, b);
}

/// C `consensus`: per-variable intersection, or union for null intersections.
pub fn consensus(context: &CubeContext, a: &BitSet, b: &BitSet) -> BitSet {
    context.consensus(a, b)
}

/// C `cactive`: index of the single active variable, or `None`.
pub fn cactive(context: &CubeContext, a: &BitSet) -> Option<usize> {
    context.cactive(a)
}

/// C `ccommon`: true when `a` and `b` share an active variable under `cof`.
pub fn ccommon(context: &CubeContext, a: &BitSet, b: &BitSet, cof: &BitSet) -> bool {
    context.ccommon(a, b, cof)
}

/// C `descend`: descending size, then descending lexical word order.
pub fn descend(a: &BitSet, b: &BitSet) -> Ordering {
    foundation_descend(a, b)
}

/// C `ascend`: ascending size, then ascending lexical word order.
pub fn ascend(a: &BitSet, b: &BitSet) -> Ordering {
    foundation_ascend(a, b)
}

/// C `lex_order`: descending lexical word order.
pub fn lex_order(a: &BitSet, b: &BitSet) -> Ordering {
    foundation_lex_order(a, b)
}

/// C `d1_order`: lexical word order after OR-ing each cube with the merge mask.
pub fn d1_order(a: &BitSet, b: &BitSet, mask: &BitSet) -> Ordering {
    foundation_d1_order(a, b, mask)
}

/// C `desc1`: descending comparison with null pointers sorted last.
pub fn desc1(a: Option<&BitSet>, b: Option<&BitSet>) -> Ordering {
    foundation_desc1(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mixed_context() -> CubeContext {
        CubeContext::setup(CubeConfig {
            num_vars: 3,
            num_binary_vars: 2,
            part_size: vec![0, 0, 3],
        })
        .unwrap()
        .0
    }

    #[test]
    fn distance_variants_match_setc_null_intersection_rules() {
        let context = mixed_context();
        let mut a = context.new_cube();
        CubeContext::put_input(&mut a, 0, ONE);
        CubeContext::put_input(&mut a, 1, DASH);
        a.insert(4);
        a.insert(5);

        let mut b = context.new_cube();
        CubeContext::put_input(&mut b, 0, ZERO);
        CubeContext::put_input(&mut b, 1, DASH);
        b.insert(5);
        b.insert(6);

        assert_eq!(cdist(&context, &a, &b), 1);
        assert_eq!(cdist01(&context, &a, &b), 1);
        assert!(!cdist0(&context, &a, &b));

        let mut c = context.new_cube();
        CubeContext::put_input(&mut c, 0, ZERO);
        CubeContext::put_input(&mut c, 1, ZERO);
        c.insert(6);
        assert_eq!(cdist(&context, &a, &c), 2);
        assert_eq!(cdist01(&context, &a, &c), 2);
    }

    #[test]
    fn consensus_and_force_lower_follow_per_variable_setc_behavior() {
        let context = mixed_context();
        let mut a = context.new_cube();
        CubeContext::put_input(&mut a, 0, ONE);
        CubeContext::put_input(&mut a, 1, DASH);
        a.insert(4);
        a.insert(5);

        let mut b = context.new_cube();
        CubeContext::put_input(&mut b, 0, ZERO);
        CubeContext::put_input(&mut b, 1, DASH);
        b.insert(5);
        b.insert(6);

        let result = consensus(&context, &a, &b);
        assert_eq!(CubeContext::get_input(&result, 0), DASH);
        assert_eq!(CubeContext::get_input(&result, 1), DASH);
        assert_eq!(result.elements().collect::<Vec<_>>(), vec![0, 1, 2, 3, 5]);

        let mut lower = context.new_cube();
        force_lower(&context, &mut lower, &a, &b);
        assert_eq!(CubeContext::get_input(&lower, 0), ONE);
        assert!(!lower.contains(4));
        assert!(!lower.contains(5));
    }

    #[test]
    fn activity_and_common_active_variable_match_setc_behavior() {
        let context = CubeContext::setup(CubeConfig {
            num_vars: 2,
            num_binary_vars: 1,
            part_size: vec![0, 3],
        })
        .unwrap()
        .0;

        let mut single_active = context.new_full_cube();
        CubeContext::put_input(&mut single_active, 0, ONE);
        assert_eq!(cactive(&context, &single_active), Some(0));

        let mut two_active = single_active.clone();
        two_active.remove(3);
        assert_eq!(cactive(&context, &two_active), None);

        let mut a = context.new_full_cube();
        a.remove(3);
        let mut b = context.new_full_cube();
        b.remove(4);
        let cof = context.new_cube();
        assert!(ccommon(&context, &a, &b, &cof));
        assert!(full_row(
            &context,
            &a,
            &BitSet::from_indices(context.size, [3])
        ));
    }

    #[test]
    fn qsort_comparators_match_setc_ordering_contracts() {
        let mut large = BitSet::from_indices(8, [0, 2, 4]);
        large.put_size(3);
        let mut small = BitSet::from_indices(8, [7]);
        small.put_size(1);
        let mut peer_high = BitSet::from_indices(8, [6]);
        peer_high.put_size(1);

        assert_eq!(descend(&large, &small), Ordering::Less);
        assert_eq!(ascend(&large, &small), Ordering::Greater);
        assert_eq!(lex_order(&peer_high, &small), Ordering::Greater);
        assert_eq!(desc1(None, Some(&large)), Ordering::Greater);

        let mask = BitSet::from_indices(8, [7]);
        assert_eq!(d1_order(&small, &peer_high, &mask), Ordering::Greater);
    }
}
