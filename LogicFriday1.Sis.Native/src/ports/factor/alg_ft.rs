//! Native algebraic factoring for SIS sum-of-products covers.
//!
//! The original `alg_ft.c` rewrites a temporary node tree by repeatedly
//! extracting generated kernels, choosing the special or general factoring
//! case from the co-kernel shape, substituting the extracted factors, and then
//! recursing into the extracted subfunctions. This port keeps that control
//! flow in owned Rust data and exposes each intermediate cover for higher-level
//! integration.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VariableId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FactorId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiteralPhase {
    Negative,
    Positive,
}

impl LiteralPhase {
    pub const fn complement(self) -> Self {
        match self {
            Self::Negative => Self::Positive,
            Self::Positive => Self::Negative,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Atom {
    Variable(VariableId),
    Factor(FactorId),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Literal {
    pub atom: Atom,
    pub phase: LiteralPhase,
}

impl Literal {
    pub const fn variable(variable: VariableId, phase: LiteralPhase) -> Self {
        Self {
            atom: Atom::Variable(variable),
            phase,
        }
    }

    pub const fn positive_variable(variable: VariableId) -> Self {
        Self::variable(variable, LiteralPhase::Positive)
    }

    pub const fn negative_variable(variable: VariableId) -> Self {
        Self::variable(variable, LiteralPhase::Negative)
    }

    pub const fn factor(factor: FactorId, phase: LiteralPhase) -> Self {
        Self {
            atom: Atom::Factor(factor),
            phase,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: impl Into<Vec<Literal>>) -> FactorResult<Self> {
        let mut literals = literals.into();
        literals.sort_unstable();
        literals.dedup();
        validate_literals(&literals)?;
        Ok(Self { literals })
    }

    pub fn one() -> Self {
        Self {
            literals: Vec::new(),
        }
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    pub fn literal_count(&self) -> usize {
        self.literals.len()
    }

    pub fn contains(&self, literal: Literal) -> bool {
        self.literals.binary_search(&literal).is_ok()
    }

    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.literals.iter().all(|literal| other.contains(*literal))
    }

    pub fn quotient_by(&self, divisor: &Self) -> Option<Self> {
        divisor.is_subset_of(self).then(|| Self {
            literals: self
                .literals
                .iter()
                .copied()
                .filter(|literal| !divisor.contains(*literal))
                .collect(),
        })
    }

    pub fn multiply(&self, other: &Self) -> FactorResult<Option<Self>> {
        let mut literals = Vec::with_capacity(self.literals.len() + other.literals.len());
        literals.extend_from_slice(&self.literals);
        literals.extend_from_slice(&other.literals);
        literals.sort_unstable();
        literals.dedup();

        if has_conflict(&literals) {
            Ok(None)
        } else {
            Ok(Some(Self { literals }))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(cubes: impl Into<Vec<Cube>>) -> Self {
        Self {
            cubes: minimized_cubes(cubes.into()),
        }
    }

    pub fn zero() -> Self {
        Self { cubes: Vec::new() }
    }

    pub fn one() -> Self {
        Self {
            cubes: vec![Cube::one()],
        }
    }

    pub fn literal(literal: Literal) -> Self {
        Self {
            cubes: vec![Cube {
                literals: vec![literal],
            }],
        }
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn cube_count(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_zero(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn is_one(&self) -> bool {
        self.cubes.len() == 1 && self.cubes[0].literals.is_empty()
    }

    pub fn atoms(&self) -> BTreeSet<Atom> {
        self.cubes
            .iter()
            .flat_map(|cube| cube.literals.iter().map(|literal| literal.atom))
            .collect()
    }

    pub fn and(&self, other: &Self) -> FactorResult<Self> {
        let mut cubes = Vec::new();
        for left in &self.cubes {
            for right in &other.cubes {
                if let Some(product) = left.multiply(right)? {
                    cubes.push(product);
                }
            }
        }

        Ok(Self::new(cubes))
    }

    pub fn or(&self, other: &Self) -> Self {
        let mut cubes = self.cubes.clone();
        cubes.extend_from_slice(&other.cubes);
        Self::new(cubes)
    }

    pub fn quotient(&self, divisor: &Self) -> Self {
        if divisor.is_zero() || self.cubes.len() < divisor.cubes.len() {
            return Self::zero();
        }

        if divisor.cubes.len() == 1 {
            return Self::new(
                self.cubes
                    .iter()
                    .filter_map(|cube| cube.quotient_by(&divisor.cubes[0]))
                    .collect::<Vec<_>>(),
            );
        }

        if !self.atoms().is_superset(&divisor.atoms()) {
            return Self::zero();
        }

        let mut candidates: BTreeMap<Cube, BTreeSet<usize>> = BTreeMap::new();
        for dividend_cube in &self.cubes {
            for (divisor_index, divisor_cube) in divisor.cubes.iter().enumerate() {
                if let Some(quotient_cube) = dividend_cube.quotient_by(divisor_cube) {
                    candidates
                        .entry(quotient_cube)
                        .or_default()
                        .insert(divisor_index);
                }
            }
        }

        Self::new(
            candidates
                .into_iter()
                .filter_map(|(cube, matches)| {
                    (matches.len() == divisor.cubes.len()).then_some(cube)
                })
                .collect::<Vec<_>>(),
        )
    }

    pub fn remainder_after_dividing_by(&self, divisor: &Self) -> FactorResult<Self> {
        let quotient = self.quotient(divisor);
        if quotient.is_zero() {
            return Ok(self.clone());
        }

        let product = quotient.and(divisor)?;
        Ok(Self::new(
            self.cubes
                .iter()
                .filter(|cube| !product.cubes.contains(cube))
                .cloned()
                .collect::<Vec<_>>(),
        ))
    }

    pub fn largest_cube_divisor(&self) -> Self {
        let Some(first) = self.cubes.first() else {
            return Self::one();
        };

        let common = first
            .literals
            .iter()
            .copied()
            .filter(|literal| {
                self.cubes
                    .iter()
                    .skip(1)
                    .all(|cube| cube.contains(*literal))
            })
            .collect::<Vec<_>>();

        Self::new(vec![Cube { literals: common }])
    }

    pub fn best_literal_with(&self, co_kernel: &Self) -> Option<Literal> {
        let co_kernel_atoms = co_kernel.atoms();
        let mut counts: BTreeMap<Literal, usize> = BTreeMap::new();
        for cube in &self.cubes {
            for literal in &cube.literals {
                if co_kernel_atoms.contains(&literal.atom) {
                    *counts.entry(*literal).or_default() += 1;
                }
            }
        }

        counts
            .into_iter()
            .max_by_key(|(literal, count)| (*count, *literal))
            .map(|(literal, _)| literal)
    }

    pub fn substitute_factor(
        &self,
        divisor: &Self,
        factor: FactorId,
    ) -> FactorResult<(bool, Self)> {
        let quotient = self.quotient(divisor);
        if quotient.is_zero() {
            return Ok((false, self.clone()));
        }

        let remainder = self.remainder_after_dividing_by(divisor)?;
        let factor_cover = Cover::literal(Literal::factor(factor, LiteralPhase::Positive));
        Ok((true, quotient.and(&factor_cover)?.or(&remainder)))
    }

    pub fn evaluates(&self, assignment: &BTreeMap<Atom, bool>) -> FactorResult<bool> {
        for cube in &self.cubes {
            let mut matches = true;
            for literal in &cube.literals {
                let value = assignment
                    .get(&literal.atom)
                    .copied()
                    .ok_or(FactorError::MissingAssignment(literal.atom))?;
                matches &= match literal.phase {
                    LiteralPhase::Negative => !value,
                    LiteralPhase::Positive => value,
                };
            }

            if matches {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlgebraicFactorNode {
    pub id: FactorId,
    pub cover: Cover,
    pub children: Vec<AlgebraicFactorNode>,
}

impl AlgebraicFactorNode {
    pub fn new(id: FactorId, cover: Cover) -> Self {
        Self {
            id,
            cover,
            children: Vec::new(),
        }
    }

    pub fn child(&self, id: FactorId) -> Option<&Self> {
        self.children.iter().find(|child| child.id == id)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FactorStats {
    pub generated_kernels: usize,
    pub special_cases: usize,
    pub general_cases: usize,
    pub substitutions: usize,
}

impl FactorStats {
    fn merge(&mut self, other: Self) {
        self.generated_kernels += other.generated_kernels;
        self.special_cases += other.special_cases;
        self.general_cases += other.general_cases;
        self.substitutions += other.substitutions;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorError {
    ConflictingLiteral { atom: Atom },
    EmptySpecialCoKernel,
    MissingBestLiteral,
    MissingAssignment(Atom),
    RecursionLimitExceeded { limit: usize },
}

impl fmt::Display for FactorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingLiteral { atom } => {
                write!(f, "cube contains both phases for atom {:?}", atom)
            }
            Self::EmptySpecialCoKernel => {
                write!(f, "special factoring received an empty co-kernel")
            }
            Self::MissingBestLiteral => write!(f, "special factoring could not choose a literal"),
            Self::MissingAssignment(atom) => write!(f, "missing assignment for {:?}", atom),
            Self::RecursionLimitExceeded { limit } => {
                write!(f, "algebraic factoring exceeded recursion limit {limit}")
            }
        }
    }
}

impl Error for FactorError {}

pub type FactorResult<T> = Result<T, FactorError>;

pub fn factor_recur<G>(
    node: &mut AlgebraicFactorNode,
    mut gen_factor: G,
) -> FactorResult<FactorStats>
where
    G: FnMut(&Cover) -> Option<Cover>,
{
    let mut allocator = FactorAllocator::new(next_factor_id(node));
    factor_recur_with_limit(node, &mut gen_factor, &mut allocator, 128)
}

pub fn factor_recur_with_limit<G>(
    node: &mut AlgebraicFactorNode,
    gen_factor: &mut G,
    allocator: &mut FactorAllocator,
    recursion_limit: usize,
) -> FactorResult<FactorStats>
where
    G: FnMut(&Cover) -> Option<Cover>,
{
    if recursion_limit == 0 {
        return Err(FactorError::RecursionLimitExceeded { limit: 0 });
    }

    let Some(kernel) = gen_factor(&node.cover) else {
        return Ok(FactorStats::default());
    };

    if kernel.is_zero() {
        return Ok(FactorStats::default());
    }

    let co_kernel = node.cover.quotient(&kernel);
    if co_kernel.is_zero() {
        return Ok(FactorStats::default());
    }

    let mut stats = FactorStats {
        generated_kernels: 1,
        ..FactorStats::default()
    };

    if co_kernel.cube_count() == 1 {
        stats.merge(factor_special(
            node,
            &co_kernel,
            gen_factor,
            allocator,
            recursion_limit - 1,
        )?);
    } else {
        stats.merge(factor_general(
            node,
            &co_kernel,
            gen_factor,
            allocator,
            recursion_limit - 1,
        )?);
    }

    Ok(stats)
}

pub fn factor_special<G>(
    node: &mut AlgebraicFactorNode,
    co_kernel: &Cover,
    gen_factor: &mut G,
    allocator: &mut FactorAllocator,
    recursion_limit: usize,
) -> FactorResult<FactorStats>
where
    G: FnMut(&Cover) -> Option<Cover>,
{
    let best_literal = node
        .cover
        .best_literal_with(co_kernel)
        .ok_or(FactorError::MissingBestLiteral)?;
    let best_literal_cover = Cover::literal(best_literal);
    let p1 = node.cover.quotient(&best_literal_cover);
    let c1 = p1.largest_cube_divisor();
    let c2 = best_literal_cover.and(&c1)?;
    let p = node.cover.quotient(&c2);
    if p.is_zero() {
        return Err(FactorError::EmptySpecialCoKernel);
    }

    let child_id = allocator.next();
    let (changed, rewritten) = node.cover.substitute_factor(&p, child_id)?;
    let mut child = AlgebraicFactorNode::new(child_id, p);
    node.cover = rewritten;
    node.children.push(child.clone());

    let mut stats = FactorStats {
        special_cases: 1,
        substitutions: usize::from(changed),
        ..FactorStats::default()
    };
    stats.merge(factor_recur_with_limit(
        node,
        gen_factor,
        allocator,
        recursion_limit,
    )?);
    stats.merge(factor_recur_with_limit(
        &mut child,
        gen_factor,
        allocator,
        recursion_limit,
    )?);
    replace_child(node, child);
    Ok(stats)
}

pub fn factor_general<G>(
    node: &mut AlgebraicFactorNode,
    co_kernel: &Cover,
    gen_factor: &mut G,
    allocator: &mut FactorAllocator,
    recursion_limit: usize,
) -> FactorResult<FactorStats>
where
    G: FnMut(&Cover) -> Option<Cover>,
{
    let c1 = co_kernel.largest_cube_divisor();
    let p = co_kernel.quotient(&c1);
    let q1 = node.cover.quotient(&p);
    let c2 = q1.largest_cube_divisor();
    let q = q1.quotient(&c2);

    let p_id = allocator.next();
    let q_id = allocator.next();
    let (p_changed, after_p) = node.cover.substitute_factor(&p, p_id)?;
    let (q_changed, after_q) = after_p.substitute_factor(&q, q_id)?;

    let mut p_child = AlgebraicFactorNode::new(p_id, p);
    let mut q_child = AlgebraicFactorNode::new(q_id, q);
    node.cover = after_q;
    node.children.push(p_child.clone());
    node.children.push(q_child.clone());

    let mut stats = FactorStats {
        general_cases: 1,
        substitutions: usize::from(p_changed) + usize::from(q_changed),
        ..FactorStats::default()
    };
    stats.merge(factor_recur_with_limit(
        node,
        gen_factor,
        allocator,
        recursion_limit,
    )?);
    stats.merge(factor_recur_with_limit(
        &mut p_child,
        gen_factor,
        allocator,
        recursion_limit,
    )?);
    stats.merge(factor_recur_with_limit(
        &mut q_child,
        gen_factor,
        allocator,
        recursion_limit,
    )?);
    replace_child(node, p_child);
    replace_child(node, q_child);
    Ok(stats)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorAllocator {
    next: usize,
}

impl FactorAllocator {
    pub const fn new(next: usize) -> Self {
        Self { next }
    }

    pub fn next(&mut self) -> FactorId {
        let id = FactorId(self.next);
        self.next += 1;
        id
    }
}

fn replace_child(node: &mut AlgebraicFactorNode, child: AlgebraicFactorNode) {
    if let Some(slot) = node
        .children
        .iter_mut()
        .find(|candidate| candidate.id == child.id)
    {
        *slot = child;
    }
}

fn next_factor_id(node: &AlgebraicFactorNode) -> usize {
    let mut next = node.id.0 + 1;
    for child in &node.children {
        next = next.max(next_factor_id(child));
    }

    next
}

fn validate_literals(literals: &[Literal]) -> FactorResult<()> {
    if let Some(atom) = first_conflicting_atom(literals) {
        Err(FactorError::ConflictingLiteral { atom })
    } else {
        Ok(())
    }
}

fn has_conflict(literals: &[Literal]) -> bool {
    first_conflicting_atom(literals).is_some()
}

fn first_conflicting_atom(literals: &[Literal]) -> Option<Atom> {
    for window in literals.windows(2) {
        if window[0].atom == window[1].atom && window[0].phase != window[1].phase {
            return Some(window[0].atom);
        }
    }

    None
}

fn minimized_cubes(mut cubes: Vec<Cube>) -> Vec<Cube> {
    cubes.sort_unstable();
    cubes.dedup();

    let mut reduced = Vec::new();
    'candidate: for (index, cube) in cubes.iter().enumerate() {
        for (other_index, other) in cubes.iter().enumerate() {
            if index != other_index && other.is_subset_of(cube) {
                continue 'candidate;
            }
        }

        reduced.push(cube.clone());
    }

    reduced
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: FactorId = FactorId(10);
    const A: VariableId = VariableId(0);
    const B: VariableId = VariableId(1);
    const C: VariableId = VariableId(2);
    const D: VariableId = VariableId(3);
    const E: VariableId = VariableId(4);

    fn pos(variable: VariableId) -> Literal {
        Literal::positive_variable(variable)
    }

    fn neg(variable: VariableId) -> Literal {
        Literal::negative_variable(variable)
    }

    fn cube(literals: &[Literal]) -> Cube {
        Cube::new(literals.to_vec()).unwrap()
    }

    fn cover(cubes: Vec<Cube>) -> Cover {
        Cover::new(cubes)
    }

    #[test]
    fn quotient_extracts_common_algebraic_divisor() {
        let f = cover(vec![
            cube(&[pos(A), pos(C)]),
            cube(&[pos(B), pos(C)]),
            cube(&[pos(A), pos(D)]),
            cube(&[pos(B), pos(D)]),
            cube(&[neg(A), pos(D)]),
        ]);
        let divisor = cover(vec![cube(&[pos(A)]), cube(&[pos(B)])]);

        let quotient = f.quotient(&divisor);

        assert_eq!(quotient, cover(vec![cube(&[pos(C)]), cube(&[pos(D)])]));
    }

    #[test]
    fn largest_cube_divisor_keeps_only_literals_shared_by_every_cube() {
        let f = cover(vec![
            cube(&[pos(A), pos(B), pos(C)]),
            cube(&[pos(A), neg(B), pos(C)]),
            cube(&[pos(A), pos(C), pos(D)]),
        ]);

        assert_eq!(
            f.largest_cube_divisor(),
            cover(vec![cube(&[pos(A), pos(C)])])
        );
    }

    #[test]
    fn substitution_replaces_divisor_with_new_factor_literal() {
        let f = cover(vec![
            cube(&[pos(A), pos(C)]),
            cube(&[pos(B), pos(C)]),
            cube(&[pos(D)]),
        ]);
        let divisor = cover(vec![cube(&[pos(A)]), cube(&[pos(B)])]);

        let (changed, rewritten) = f.substitute_factor(&divisor, FactorId(99)).unwrap();

        assert!(changed);
        assert_eq!(
            rewritten,
            cover(vec![
                cube(&[
                    Literal::factor(FactorId(99), LiteralPhase::Positive),
                    pos(C)
                ]),
                cube(&[pos(D)])
            ])
        );
    }

    #[test]
    fn special_case_extracts_single_cokernel_shape() {
        let mut node = AlgebraicFactorNode::new(
            ROOT,
            cover(vec![
                cube(&[pos(A), pos(B)]),
                cube(&[pos(A), pos(C)]),
                cube(&[pos(D)]),
            ]),
        );
        let mut kernel_generator = |candidate: &Cover| {
            if candidate.atoms().contains(&Atom::Variable(A)) {
                Some(cover(vec![cube(&[pos(B)]), cube(&[pos(C)])]))
            } else {
                None
            }
        };
        let mut allocator = FactorAllocator::new(20);

        let stats =
            factor_recur_with_limit(&mut node, &mut kernel_generator, &mut allocator, 8).unwrap();

        assert_eq!(stats.special_cases, 1);
        assert_eq!(stats.general_cases, 0);
        assert_eq!(stats.substitutions, 1);
        assert_eq!(node.children.len(), 1);
        assert_eq!(
            node.cover,
            cover(vec![
                cube(&[
                    Literal::factor(FactorId(20), LiteralPhase::Positive),
                    pos(A)
                ]),
                cube(&[pos(D)])
            ])
        );
        assert_eq!(
            node.child(FactorId(20)).unwrap().cover,
            cover(vec![cube(&[pos(B)]), cube(&[pos(C)])])
        );
    }

    #[test]
    fn general_case_extracts_p_and_q_from_multi_cube_cokernel() {
        let original = cover(vec![
            cube(&[pos(A), pos(C)]),
            cube(&[pos(A), pos(D)]),
            cube(&[pos(B), pos(C)]),
            cube(&[pos(B), pos(D)]),
        ]);
        let mut node = AlgebraicFactorNode::new(ROOT, original.clone());
        let mut kernel_generator = |candidate: &Cover| {
            if candidate == &original {
                Some(cover(vec![cube(&[pos(C)]), cube(&[pos(D)])]))
            } else {
                None
            }
        };
        let mut allocator = FactorAllocator::new(30);

        let stats =
            factor_recur_with_limit(&mut node, &mut kernel_generator, &mut allocator, 8).unwrap();

        assert_eq!(stats.general_cases, 1);
        assert_eq!(stats.special_cases, 0);
        assert_eq!(stats.substitutions, 2);
        assert_eq!(
            node.cover,
            cover(vec![cube(&[
                Literal::factor(FactorId(30), LiteralPhase::Positive),
                Literal::factor(FactorId(31), LiteralPhase::Positive)
            ])])
        );
        assert_eq!(
            node.child(FactorId(30)).unwrap().cover,
            cover(vec![cube(&[pos(A)]), cube(&[pos(B)])])
        );
        assert_eq!(
            node.child(FactorId(31)).unwrap().cover,
            cover(vec![cube(&[pos(C)]), cube(&[pos(D)])])
        );
    }

    #[test]
    fn recursion_descends_into_extracted_children() {
        let nested = cover(vec![
            cube(&[pos(A), pos(C)]),
            cube(&[pos(A), pos(D)]),
            cube(&[pos(B), pos(C)]),
            cube(&[pos(B), pos(D)]),
        ]);
        let original = cover(vec![
            cube(&[pos(E), pos(A), pos(C)]),
            cube(&[pos(E), pos(A), pos(D)]),
            cube(&[pos(E), pos(B), pos(C)]),
            cube(&[pos(E), pos(B), pos(D)]),
        ]);
        let mut node = AlgebraicFactorNode::new(ROOT, original.clone());
        let mut kernel_generator = |candidate: &Cover| {
            if candidate == &original {
                Some(nested.clone())
            } else if candidate == &nested {
                Some(cover(vec![cube(&[pos(C)]), cube(&[pos(D)])]))
            } else {
                None
            }
        };

        let stats = factor_recur(&mut node, &mut kernel_generator).unwrap();

        assert_eq!(stats.generated_kernels, 2);
        assert_eq!(stats.special_cases, 1);
        assert_eq!(stats.general_cases, 1);
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].children.len(), 2);
    }

    #[test]
    fn conflicting_cube_literals_are_rejected() {
        assert_eq!(
            Cube::new(vec![pos(A), neg(A)]),
            Err(FactorError::ConflictingLiteral {
                atom: Atom::Variable(A)
            })
        );
    }

    #[test]
    fn evaluation_uses_factor_literals_as_external_atoms() {
        let cover = cover(vec![
            cube(&[
                Literal::factor(FactorId(11), LiteralPhase::Positive),
                pos(A),
            ]),
            cube(&[neg(B)]),
        ]);
        let assignment = BTreeMap::from([
            (Atom::Factor(FactorId(11)), true),
            (Atom::Variable(A), true),
            (Atom::Variable(B), true),
        ]);

        assert!(cover.evaluates(&assignment).unwrap());
    }

    #[test]
    fn recursion_limit_prevents_unbounded_generators() {
        let mut node = AlgebraicFactorNode::new(
            ROOT,
            cover(vec![cube(&[pos(A), pos(B)]), cube(&[pos(A), pos(C)])]),
        );
        let mut kernel_generator = |_cover: &Cover| Some(Cover::literal(pos(A)));
        let mut allocator = FactorAllocator::new(40);

        assert_eq!(
            factor_recur_with_limit(&mut node, &mut kernel_generator, &mut allocator, 0),
            Err(FactorError::RecursionLimitExceeded { limit: 0 })
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let source = include_str!("alg_ft.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }
}
