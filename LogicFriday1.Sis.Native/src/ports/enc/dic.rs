//! Native Rust dichotomy support for SIS encoding.

use crate::ports::espresso::set::Set;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dichotomy
{
    left: Set,
    right: Set,
}

impl Dichotomy
{
    pub fn new(element_count: usize) -> Self
    {
        Self
        {
            left: Set::empty(element_count),
            right: Set::empty(element_count),
        }
    }

    pub fn from_sets(left: Set, right: Set) -> Self
    {
        assert_eq!(
            left.element_count(),
            right.element_count(),
            "dichotomy sides must have the same element count"
        );

        Self
        {
            left,
            right,
        }
    }

    pub fn from_elements(
        element_count: usize,
        left: impl IntoIterator<Item = usize>,
        right: impl IntoIterator<Item = usize>,
    ) -> Self
    {
        Self::from_sets(
            Set::from_elements(element_count, left),
            Set::from_elements(element_count, right),
        )
    }

    pub fn element_count(&self) -> usize
    {
        self.left.element_count()
    }

    pub fn left(&self) -> &Set
    {
        &self.left
    }

    pub fn left_mut(&mut self) -> &mut Set
    {
        &mut self.left
    }

    pub fn right(&self) -> &Set
    {
        &self.right
    }

    pub fn right_mut(&mut self) -> &mut Set
    {
        &mut self.right
    }

    pub fn is_prime(&self) -> bool
    {
        self.left.union(&self.right).is_full()
    }

    pub fn implies(&self, other: &Self) -> bool
    {
        self.assert_same_size(other);

        self.left.implies(&other.left) && self.right.implies(&other.right)
    }

    pub fn covers(&self, other: &Self) -> bool
    {
        self.assert_same_size(other);

        self.implies(other)
            || (self.left.implies(&other.right) && self.right.implies(&other.left))
    }

    pub fn format_bit_vectors(&self) -> String
    {
        format!(
            "{};{}\n",
            self.left.to_bit_string(self.element_count()),
            self.right.to_bit_string(self.element_count())
        )
    }

    fn assert_same_size(&self, other: &Self)
    {
        assert_eq!(
            self.element_count(),
            other.element_count(),
            "dichotomy element counts must match"
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DichotomyFamily
{
    capacity: usize,
    element_count: usize,
    dichotomies: Vec<Dichotomy>,
}

impl DichotomyFamily
{
    pub fn new(capacity: usize, element_count: usize) -> Self
    {
        Self
        {
            capacity,
            element_count,
            dichotomies: Vec::with_capacity(capacity),
        }
    }

    pub fn from_dichotomies(
        element_count: usize,
        dichotomies: impl IntoIterator<Item = Dichotomy>,
    ) -> Self
    {
        let mut family = Self::new(0, element_count);
        for dichotomy in dichotomies
        {
            family.add(dichotomy);
        }

        family
    }

    pub fn capacity(&self) -> usize
    {
        self.capacity
    }

    pub fn element_count(&self) -> usize
    {
        self.element_count
    }

    pub fn len(&self) -> usize
    {
        self.dichotomies.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.dichotomies.is_empty()
    }

    pub fn dichotomies(&self) -> &[Dichotomy]
    {
        &self.dichotomies
    }

    pub fn add(&mut self, dichotomy: Dichotomy)
    {
        self.assert_dichotomy_size(&dichotomy);
        self.ensure_capacity_for_one_more();
        self.dichotomies.push(dichotomy);
    }

    pub fn add_if_absent(&mut self, dichotomy: Dichotomy) -> bool
    {
        self.assert_dichotomy_size(&dichotomy);

        if self.dichotomies.contains(&dichotomy)
        {
            return false;
        }

        self.add(dichotomy);
        true
    }

    pub fn add_irredundant(&mut self, dichotomy: Dichotomy) -> IrredundantAdd
    {
        self.assert_dichotomy_size(&dichotomy);

        for existing in &mut self.dichotomies
        {
            if dichotomy.implies(existing)
            {
                return IrredundantAdd::ImpliedByExisting;
            }

            if existing.implies(&dichotomy)
            {
                *existing = dichotomy;
                return IrredundantAdd::ReplacedExisting;
            }
        }

        self.add(dichotomy);
        IrredundantAdd::Added
    }

    pub fn format_bit_vectors(&self) -> String
    {
        self.dichotomies
            .iter()
            .map(Dichotomy::format_bit_vectors)
            .collect()
    }

    pub fn cross_product(&self, other: &Self) -> Self
    {
        assert_eq!(
            self.element_count,
            other.element_count,
            "dichotomy family element counts must match"
        );

        Self::new(0, self.element_count)
    }

    fn ensure_capacity_for_one_more(&mut self)
    {
        if self.dichotomies.len() >= self.capacity
        {
            self.capacity = self.capacity + self.capacity / 2 + 1;
        }
    }

    fn assert_dichotomy_size(&self, dichotomy: &Dichotomy)
    {
        assert_eq!(
            self.element_count,
            dichotomy.element_count(),
            "dichotomy size must match family element count"
        );
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrredundantAdd
{
    Added,
    ImpliedByExisting,
    ReplacedExisting,
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn dic(left: &[usize], right: &[usize]) -> Dichotomy
    {
        Dichotomy::from_elements(5, left.iter().copied(), right.iter().copied())
    }

    #[test]
    fn new_dichotomy_has_empty_sides_with_requested_size()
    {
        let dichotomy = Dichotomy::new(5);

        assert_eq!(dichotomy.element_count(), 5);
        assert!(dichotomy.left().is_empty());
        assert!(dichotomy.right().is_empty());
    }

    #[test]
    fn prime_checks_that_both_sides_cover_every_element()
    {
        assert!(dic(&[0, 2], &[1, 3, 4]).is_prime());
        assert!(dic(&[0, 1, 2, 3, 4], &[]).is_prime());
        assert!(!dic(&[0, 2], &[1, 4]).is_prime());
    }

    #[test]
    fn implication_equality_and_swapped_cover_match_dichotomy_semantics()
    {
        let small = dic(&[0], &[4]);
        let large = dic(&[0, 1], &[3, 4]);
        let swapped_large = dic(&[3, 4], &[0, 1]);

        assert!(small.implies(&large));
        assert!(!large.implies(&small));
        assert!(small.covers(&large));
        assert!(small.covers(&swapped_large));
        assert_eq!(small, dic(&[0], &[4]));
        assert_ne!(small, dic(&[4], &[0]));
    }

    #[test]
    fn add_grows_capacity_like_the_legacy_family_vector()
    {
        let mut family = DichotomyFamily::new(1, 5);

        family.add(dic(&[0], &[1]));
        family.add(dic(&[1], &[2]));

        assert_eq!(family.capacity(), 2);
        assert_eq!(family.len(), 2);
    }

    #[test]
    fn add_if_absent_keeps_exact_duplicates_out()
    {
        let mut family = DichotomyFamily::new(0, 5);

        assert!(family.add_if_absent(dic(&[0], &[1])));
        assert!(!family.add_if_absent(dic(&[0], &[1])));
        assert!(family.add_if_absent(dic(&[1], &[0])));

        assert_eq!(family.len(), 2);
    }

    #[test]
    fn irredundant_add_skips_existing_supersets_and_replaces_subsets()
    {
        let mut family = DichotomyFamily::new(0, 5);

        assert_eq!(
            family.add_irredundant(dic(&[0, 1], &[3, 4])),
            IrredundantAdd::Added
        );
        assert_eq!(
            family.add_irredundant(dic(&[0], &[4])),
            IrredundantAdd::ImpliedByExisting
        );
        assert_eq!(
            family.add_irredundant(dic(&[0, 1, 2], &[3, 4])),
            IrredundantAdd::ReplacedExisting
        );

        assert_eq!(family.dichotomies(), &[dic(&[0, 1, 2], &[3, 4])]);
    }

    #[test]
    fn formatting_matches_legacy_bit_vector_lines()
    {
        let mut family = DichotomyFamily::new(0, 5);
        family.add(dic(&[0, 3], &[1, 4]));
        family.add(dic(&[2], &[]));

        assert_eq!(dic(&[0, 3], &[1, 4]).format_bit_vectors(), "10010;01001\n");
        assert_eq!(family.format_bit_vectors(), "10010;01001\n00100;00000\n");
    }

    #[test]
    fn cross_product_preserves_the_legacy_commented_out_stub_behavior()
    {
        let left = DichotomyFamily::from_dichotomies(5, [dic(&[0], &[1])]);
        let right = DichotomyFamily::from_dichotomies(5, [dic(&[2], &[3])]);

        let result = left.cross_product(&right);

        assert_eq!(result.element_count(), 5);
        assert!(result.is_empty());
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("dic.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
