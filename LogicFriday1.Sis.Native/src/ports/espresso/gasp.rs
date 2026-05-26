//! Native Rust implementation of the Espresso GASP heuristics.
//!
//! The routines in this module operate on owned Boolean cubes. They keep the
//! legacy algorithm shape: reduce each ON-set cube without replacement, expand
//! useful reduced cubes into new primes, and finish by extracting an
//! irredundant cover. The implementation uses exact assignment checks for the
//! local feasibility decisions so it can stand alone while adjacent Espresso
//! ports are still being filled in.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Literal
{
    Zero,
    One,
    DontCare,
}

impl Literal
{
    fn from_bit(bit: bool) -> Self
    {
        if bit
        {
            Self::One
        }
        else
        {
            Self::Zero
        }
    }

    fn matches(self, bit: bool) -> bool
    {
        match self
        {
            Self::Zero => !bit,
            Self::One => bit,
            Self::DontCare => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Cube
{
    literals: Vec<Literal>,
}

impl Cube
{
    pub fn new(literals: Vec<Literal>) -> Self
    {
        Self { literals }
    }

    pub fn minterm(width: usize, assignment: usize) -> Self
    {
        Self {
            literals: (0..width)
                .map(|index| Literal::from_bit(((assignment >> index) & 1) != 0))
                .collect(),
        }
    }

    pub fn width(&self) -> usize
    {
        self.literals.len()
    }

    pub fn literals(&self) -> &[Literal]
    {
        &self.literals
    }

    pub fn literal(&self, index: usize) -> Option<Literal>
    {
        self.literals.get(index).copied()
    }

    pub fn contains_assignment(&self, assignment: usize) -> bool
    {
        self.literals.iter().enumerate().all(|(index, literal)| {
            literal.matches(((assignment >> index) & 1) != 0)
        })
    }

    pub fn contains_cube(&self, other: &Self) -> bool
    {
        self.width() == other.width()
            && self.literals.iter().zip(&other.literals).all(|(left, right)| {
                *left == Literal::DontCare || left == right
            })
    }

    pub fn intersects(&self, other: &Self) -> bool
    {
        self.width() == other.width()
            && self.literals.iter().zip(&other.literals).all(|(left, right)| {
                *left == Literal::DontCare || *right == Literal::DontCare || left == right
            })
    }

    pub fn joined_with(&self, other: &Self) -> GaspResult<Self>
    {
        ensure_same_cube_width(self, other)?;

        Ok(Self {
            literals: self
                .literals
                .iter()
                .zip(&other.literals)
                .map(|(left, right)| {
                    if left == right
                    {
                        *left
                    }
                    else
                    {
                        Literal::DontCare
                    }
                })
                .collect(),
        })
    }

    fn with_literal(&self, index: usize, literal: Literal) -> Self
    {
        let mut result = self.clone();
        result.literals[index] = literal;
        result
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover
{
    width: usize,
    cubes: Vec<Cube>,
}

impl Cover
{
    pub fn new(width: usize, cubes: Vec<Cube>) -> GaspResult<Self>
    {
        for cube in &cubes
        {
            if cube.width() != width
            {
                return Err(GaspError::CubeWidth {
                    expected: width,
                    actual: cube.width(),
                });
            }
        }

        Ok(Self { width, cubes })
    }

    pub fn empty(width: usize) -> Self
    {
        Self {
            width,
            cubes: Vec::new(),
        }
    }

    pub fn width(&self) -> usize
    {
        self.width
    }

    pub fn cubes(&self) -> &[Cube]
    {
        &self.cubes
    }

    pub fn into_cubes(self) -> Vec<Cube>
    {
        self.cubes
    }

    pub fn len(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }

    pub fn push(&mut self, cube: Cube) -> GaspResult<()>
    {
        if cube.width() != self.width
        {
            return Err(GaspError::CubeWidth {
                expected: self.width,
                actual: cube.width(),
            });
        }

        self.cubes.push(cube);
        Ok(())
    }

    pub fn evaluates(&self, assignment: usize) -> bool
    {
        self.cubes
            .iter()
            .any(|cube| cube.contains_assignment(assignment))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GaspResultCover
{
    cover: Cover,
    cost: GaspCost,
}

impl GaspResultCover
{
    pub fn new(cover: Cover, cost: GaspCost) -> Self
    {
        Self { cover, cost }
    }

    pub fn cover(&self) -> &Cover
    {
        &self.cover
    }

    pub fn into_cover(self) -> Cover
    {
        self.cover
    }

    pub fn cost(&self) -> GaspCost
    {
        self.cost
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GaspCost
{
    pub reduced_cubes: usize,
    pub generated_primes: usize,
    pub final_cubes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GaspError
{
    CubeWidth { expected: usize, actual: usize },
    WidthMismatch { left: usize, right: usize },
    EmptyReduction { cube_index: usize },
    TooManyInputs { width: usize, max_supported: usize },
}

impl fmt::Display for GaspError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::CubeWidth { expected, actual } => {
                write!(formatter, "cube width {actual} does not match cover width {expected}")
            }
            Self::WidthMismatch { left, right } => {
                write!(formatter, "cover widths differ: {left} != {right}")
            }
            Self::EmptyReduction { cube_index } => {
                write!(formatter, "cube {cube_index} has no private care minterms")
            }
            Self::TooManyInputs {
                width,
                max_supported,
            } => write!(
                formatter,
                "GASP assignment enumeration supports at most {max_supported} inputs, got {width}"
            ),
        }
    }
}

impl Error for GaspError {}

pub type GaspResult<T> = Result<T, GaspError>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReducedCube
{
    cube: Cube,
    prime: bool,
}

pub fn reduce_gasp(on_set: &Cover, dont_care: &Cover) -> GaspResult<Cover>
{
    let reduced = reduce_gasp_rows(on_set, dont_care)?;
    Cover::new(
        on_set.width(),
        reduced.into_iter().map(|row| row.cube).collect(),
    )
}

pub fn expand_gasp(
    reduced_on_set: &Cover,
    dont_care: &Cover,
    off_set: &Cover,
    original_on_set: &Cover,
) -> GaspResult<Cover>
{
    let mut rows = Vec::with_capacity(reduced_on_set.len());
    for (index, cube) in reduced_on_set.cubes().iter().enumerate()
    {
        let prime = original_on_set.cubes().get(index) == Some(cube);
        rows.push(ReducedCube {
            cube: cube.clone(),
            prime,
        });
    }

    expand_gasp_rows(&rows, dont_care, off_set, original_on_set)
}

pub fn irred_gasp(on_set: Cover, dont_care: &Cover, generated: Cover) -> GaspResult<Cover>
{
    let mut cubes = on_set.into_cubes();
    cubes.extend(generated.into_cubes());
    irredundant(Cover::new(dont_care.width(), cubes)?, dont_care)
}

pub fn last_gasp(on_set: Cover, dont_care: &Cover, off_set: &Cover) -> GaspResult<GaspResultCover>
{
    ensure_same_cover_width(&on_set, dont_care)?;
    ensure_same_cover_width(&on_set, off_set)?;

    let reduced = reduce_gasp_rows(&on_set, dont_care)?;
    let reduced_cover = Cover::new(
        on_set.width(),
        reduced.iter().map(|row| row.cube.clone()).collect(),
    )?;
    let generated = expand_gasp_rows(&reduced, dont_care, off_set, &on_set)?;
    let generated_primes = generated.len();
    let final_cover = irred_gasp(on_set, dont_care, generated)?;

    let cost = GaspCost {
        reduced_cubes: reduced_cover.len(),
        generated_primes,
        final_cubes: final_cover.len(),
    };

    Ok(GaspResultCover::new(final_cover, cost))
}

pub fn super_gasp(on_set: Cover, dont_care: &Cover, off_set: &Cover) -> GaspResult<GaspResultCover>
{
    ensure_same_cover_width(&on_set, dont_care)?;
    ensure_same_cover_width(&on_set, off_set)?;

    let reduced = reduce_gasp_rows(&on_set, dont_care)?;
    let reduced_cover = Cover::new(
        on_set.width(),
        reduced.iter().map(|row| row.cube.clone()).collect(),
    )?;
    let generated = all_primes_covering(&reduced_cover, off_set)?;
    let generated_primes = generated.len();
    let final_cover = irred_gasp(on_set, dont_care, generated)?;

    let cost = GaspCost {
        reduced_cubes: reduced_cover.len(),
        generated_primes,
        final_cubes: final_cover.len(),
    };

    Ok(GaspResultCover::new(final_cover, cost))
}

fn reduce_gasp_rows(on_set: &Cover, dont_care: &Cover) -> GaspResult<Vec<ReducedCube>>
{
    ensure_same_cover_width(on_set, dont_care)?;
    validate_width(on_set.width())?;

    let mut result = Vec::with_capacity(on_set.len());
    for index in 0..on_set.len()
    {
        let reduced = reduce_cube_without_replacement(on_set, dont_care, index)?;
        if reduced.is_none()
        {
            return Err(GaspError::EmptyReduction { cube_index: index });
        }

        let cube = reduced.expect("checked above");
        result.push(ReducedCube {
            prime: &cube == cube_at(on_set, index),
            cube,
        });
    }

    Ok(result)
}

fn expand_gasp_rows(
    reduced_on_set: &[ReducedCube],
    dont_care: &Cover,
    off_set: &Cover,
    original_on_set: &Cover,
) -> GaspResult<Cover>
{
    ensure_same_cover_width(dont_care, off_set)?;
    ensure_same_cover_width(dont_care, original_on_set)?;
    validate_width(original_on_set.width())?;

    let mut candidates = Vec::new();
    for c1_index in 0..reduced_on_set.len()
    {
        expand_one_gasp(
            reduced_on_set,
            dont_care,
            off_set,
            original_on_set,
            c1_index,
            &mut candidates,
        )?;
    }

    let unique = unique_cubes(candidates);
    let mut primes = Vec::with_capacity(unique.len());
    for cube in unique
    {
        primes.push(expand_to_prime(&cube, off_set)?);
    }

    Cover::new(original_on_set.width(), unique_cubes(primes))
}

fn expand_one_gasp(
    reduced_on_set: &[ReducedCube],
    dont_care: &Cover,
    off_set: &Cover,
    original_on_set: &Cover,
    c1_index: usize,
    candidates: &mut Vec<Cube>,
) -> GaspResult<()>
{
    let c1_under = &reduced_on_set[c1_index].cube;
    if reduced_on_set[c1_index].prime
    {
        return Ok(());
    }

    for (c2_index, c2_under) in reduced_on_set.iter().enumerate()
    {
        if c1_index == c2_index || c2_under.prime
        {
            continue;
        }

        let raise = c1_under.joined_with(&c2_under.cube)?;
        if !is_feasible(&raise, off_set)
        {
            continue;
        }

        let mut replacement = original_on_set.clone();
        replacement.cubes[c1_index] = c1_under.clone();

        let Some(c2_essential) =
            reduce_cube_without_replacement(&replacement, dont_care, c2_index)?
        else
        {
            continue;
        };

        let candidate = c1_under.joined_with(&c2_essential)?;
        if is_feasible(&candidate, off_set)
        {
            candidates.push(candidate);
        }
    }

    Ok(())
}

fn reduce_cube_without_replacement(
    on_set: &Cover,
    dont_care: &Cover,
    cube_index: usize,
) -> GaspResult<Option<Cube>>
{
    ensure_same_cover_width(on_set, dont_care)?;

    let cube = cube_at(on_set, cube_index);
    let private_assignments = assignments(on_set.width())?
        .filter(|assignment| cube.contains_assignment(*assignment))
        .filter(|assignment| !dont_care.evaluates(*assignment))
        .filter(|assignment| {
            !on_set.cubes().iter().enumerate().any(|(index, other)| {
                index != cube_index && other.contains_assignment(*assignment)
            })
        })
        .collect::<Vec<_>>();

    if private_assignments.is_empty()
    {
        return Ok(None);
    }

    Ok(Some(cube_from_assignments(
        on_set.width(),
        &private_assignments,
    )))
}

fn expand_to_prime(cube: &Cube, off_set: &Cover) -> GaspResult<Cube>
{
    validate_width(cube.width())?;

    let mut result = cube.clone();
    for index in 0..result.width()
    {
        if result.literal(index) == Some(Literal::DontCare)
        {
            continue;
        }

        let raised = result.with_literal(index, Literal::DontCare);
        if is_feasible(&raised, off_set)
        {
            result = raised;
        }
    }

    Ok(result)
}

fn all_primes_covering(reduced_on_set: &Cover, off_set: &Cover) -> GaspResult<Cover>
{
    ensure_same_cover_width(reduced_on_set, off_set)?;
    validate_width(reduced_on_set.width())?;
    validate_prime_enumeration_width(reduced_on_set.width())?;

    let mut primes = Vec::new();
    for cube in all_cubes(reduced_on_set.width())
    {
        if !is_feasible(&cube, off_set)
        {
            continue;
        }

        if !reduced_on_set
            .cubes()
            .iter()
            .any(|reduced| cube.contains_cube(reduced))
        {
            continue;
        }

        if is_prime_implicant(&cube, off_set)
        {
            primes.push(cube);
        }
    }

    Cover::new(reduced_on_set.width(), unique_cubes(primes))
}

fn irredundant(mut cover: Cover, dont_care: &Cover) -> GaspResult<Cover>
{
    ensure_same_cover_width(&cover, dont_care)?;
    validate_width(cover.width())?;

    cover.cubes = unique_cubes(cover.cubes);
    let mut index = 0;
    while index < cover.cubes.len()
    {
        if cube_is_redundant(&cover, dont_care, index)?
        {
            cover.cubes.remove(index);
        }
        else
        {
            index += 1;
        }
    }

    Ok(cover)
}

fn cube_is_redundant(cover: &Cover, dont_care: &Cover, cube_index: usize) -> GaspResult<bool>
{
    let cube = cube_at(cover, cube_index);

    Ok(assignments(cover.width())?
        .filter(|assignment| cube.contains_assignment(*assignment))
        .all(|assignment| {
            dont_care.evaluates(assignment)
                || cover.cubes().iter().enumerate().any(|(index, other)| {
                    index != cube_index && other.contains_assignment(assignment)
                })
        }))
}

fn is_prime_implicant(cube: &Cube, off_set: &Cover) -> bool
{
    (0..cube.width()).all(|index| {
        cube.literal(index) == Some(Literal::DontCare)
            || !is_feasible(&cube.with_literal(index, Literal::DontCare), off_set)
    })
}

fn is_feasible(cube: &Cube, off_set: &Cover) -> bool
{
    !off_set
        .cubes()
        .iter()
        .any(|off_cube| cube.intersects(off_cube))
}

fn cube_from_assignments(width: usize, assignments: &[usize]) -> Cube
{
    let literals = (0..width)
        .map(|index| {
            let first = ((assignments[0] >> index) & 1) != 0;
            if assignments
                .iter()
                .all(|assignment| (((assignment >> index) & 1) != 0) == first)
            {
                Literal::from_bit(first)
            }
            else
            {
                Literal::DontCare
            }
        })
        .collect();

    Cube::new(literals)
}

fn all_cubes(width: usize) -> Vec<Cube>
{
    let mut cubes = Vec::new();
    let mut literals = Vec::with_capacity(width);
    build_cubes(width, &mut literals, &mut cubes);
    cubes
}

fn build_cubes(width: usize, literals: &mut Vec<Literal>, cubes: &mut Vec<Cube>)
{
    if literals.len() == width
    {
        cubes.push(Cube::new(literals.clone()));
        return;
    }

    for literal in [Literal::Zero, Literal::One, Literal::DontCare]
    {
        literals.push(literal);
        build_cubes(width, literals, cubes);
        literals.pop();
    }
}

fn unique_cubes(cubes: Vec<Cube>) -> Vec<Cube>
{
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for cube in cubes
    {
        if seen.insert(cube.clone())
        {
            result.push(cube);
        }
    }

    result
}

fn cube_at(cover: &Cover, index: usize) -> &Cube
{
    &cover.cubes()[index]
}

fn ensure_same_cover_width(left: &Cover, right: &Cover) -> GaspResult<()>
{
    if left.width() != right.width()
    {
        return Err(GaspError::WidthMismatch {
            left: left.width(),
            right: right.width(),
        });
    }

    Ok(())
}

fn ensure_same_cube_width(left: &Cube, right: &Cube) -> GaspResult<()>
{
    if left.width() != right.width()
    {
        return Err(GaspError::WidthMismatch {
            left: left.width(),
            right: right.width(),
        });
    }

    Ok(())
}

fn validate_width(width: usize) -> GaspResult<()>
{
    let max_supported = usize::BITS as usize - 1;
    if width > max_supported
    {
        return Err(GaspError::TooManyInputs {
            width,
            max_supported,
        });
    }

    Ok(())
}

fn validate_prime_enumeration_width(width: usize) -> GaspResult<()>
{
    let max_supported = 12;
    if width > max_supported
    {
        return Err(GaspError::TooManyInputs {
            width,
            max_supported,
        });
    }

    Ok(())
}

fn assignments(width: usize) -> GaspResult<std::ops::Range<usize>>
{
    validate_width(width)?;
    Ok(0..(1usize << width))
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn cube(pattern: &str) -> Cube
    {
        Cube::new(
            pattern
                .chars()
                .map(|ch| match ch
                {
                    '0' => Literal::Zero,
                    '1' => Literal::One,
                    '-' => Literal::DontCare,
                    _ => panic!("invalid test literal"),
                })
                .collect(),
        )
    }

    fn cover(width: usize, patterns: &[&str]) -> Cover
    {
        Cover::new(
            width,
            patterns.iter().map(|pattern| cube(pattern)).collect(),
        )
        .unwrap()
    }

    #[test]
    fn reduce_gasp_keeps_prime_cube_when_no_literal_can_be_lowered()
    {
        let on_set = cover(2, &["10", "11"]);
        let dont_care = Cover::empty(2);

        let reduced = reduce_gasp(&on_set, &dont_care).unwrap();

        assert_eq!(reduced, on_set);
    }

    #[test]
    fn reduce_gasp_reduces_cube_to_private_care_region()
    {
        let on_set = cover(2, &["1-", "0-"]);
        let dont_care = cover(2, &["11"]);

        let reduced = reduce_gasp(&on_set, &dont_care).unwrap();

        assert_eq!(reduced, cover(2, &["10", "0-"]));
    }

    #[test]
    fn reduce_gasp_reports_redundant_cube_as_empty_reduction()
    {
        let on_set = cover(2, &["1-", "11", "10"]);
        let dont_care = Cover::empty(2);

        assert_eq!(
            reduce_gasp(&on_set, &dont_care).unwrap_err(),
            GaspError::EmptyReduction { cube_index: 0 }
        );
    }

    #[test]
    fn expand_gasp_generates_prime_covering_another_reduced_cube()
    {
        let original = cover(2, &["1-", "-1"]);
        let reduced = cover(2, &["10", "01"]);
        let dont_care = Cover::empty(2);
        let off_set = Cover::empty(2);

        let generated = expand_gasp(&reduced, &dont_care, &off_set, &original).unwrap();

        assert_eq!(generated, cover(2, &["--"]));
    }

    #[test]
    fn expand_gasp_skips_candidates_that_intersect_the_off_set()
    {
        let original = cover(2, &["1-", "-1"]);
        let reduced = cover(2, &["10", "01"]);
        let dont_care = Cover::empty(2);
        let off_set = cover(2, &["00", "11"]);

        let generated = expand_gasp(&reduced, &dont_care, &off_set, &original).unwrap();

        assert!(generated.is_empty());
    }

    #[test]
    fn irred_gasp_removes_cubes_covered_by_new_prime()
    {
        let on_set = cover(2, &["10", "11"]);
        let dont_care = Cover::empty(2);
        let generated = cover(2, &["1-"]);

        let result = irred_gasp(on_set, &dont_care, generated).unwrap();

        assert_eq!(result, cover(2, &["1-"]));
    }

    #[test]
    fn last_gasp_adds_generated_prime_and_returns_costs()
    {
        let on_set = cover(2, &["1-", "-1"]);
        let dont_care = Cover::empty(2);
        let off_set = Cover::empty(2);

        let result = last_gasp(on_set, &dont_care, &off_set).unwrap();

        assert_eq!(result.cover(), &cover(2, &["--"]));
        assert_eq!(
            result.cost(),
            GaspCost {
                reduced_cubes: 2,
                generated_primes: 1,
                final_cubes: 1,
            }
        );
    }

    #[test]
    fn super_gasp_uses_all_feasible_primes_covering_reduced_cubes()
    {
        let on_set = cover(2, &["10", "11"]);
        let dont_care = Cover::empty(2);
        let off_set = cover(2, &["00", "01"]);

        let result = super_gasp(on_set, &dont_care, &off_set).unwrap();

        assert_eq!(result.cover(), &cover(2, &["1-"]));
        assert_eq!(result.cost().generated_primes, 1);
    }

    #[test]
    fn rejects_width_mismatch()
    {
        let on_set = cover(2, &["10"]);
        let dont_care = Cover::empty(3);
        let off_set = Cover::empty(2);

        assert_eq!(
            last_gasp(on_set, &dont_care, &off_set).unwrap_err(),
            GaspError::WidthMismatch { left: 2, right: 3 }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("gasp.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
