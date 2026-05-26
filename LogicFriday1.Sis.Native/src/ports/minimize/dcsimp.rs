//! Native Rust port of `LogicSynthesis/sis/minimize/dcsimp.c`.
//!
//! The original routine recursively reduces an on-set against a don't-care set,
//! greedily combines compatible reduced cubes, then expands the result back to
//! large primes inside the on-set plus don't-care region. This module keeps that
//! behavior over owned Rust cover data and leaves SIS object wiring to callers.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    fn matches(self, value: bool) -> bool {
        match self {
            Self::Zero => !value,
            Self::One => value,
            Self::DontCare => true,
        }
    }

    fn is_care(self) -> bool {
        self != Self::DontCare
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }

    pub fn tautology(width: usize) -> Self {
        Self {
            literals: vec![Literal::DontCare; width],
        }
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    pub fn width(&self) -> usize {
        self.literals.len()
    }

    pub fn literal_count(&self) -> usize {
        self.literals
            .iter()
            .filter(|literal| literal.is_care())
            .count()
    }

    pub fn dont_care_count(&self) -> usize {
        self.width() - self.literal_count()
    }

    pub fn contains_assignment(&self, assignment: usize) -> bool {
        self.literals.iter().enumerate().all(|(index, literal)| {
            let bit = ((assignment >> index) & 1) != 0;
            literal.matches(bit)
        })
    }

    fn covers(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| *left == Literal::DontCare || left == right)
    }

    fn intersects(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| {
                *left == Literal::DontCare || *right == Literal::DontCare || left == right
            })
    }

    fn supercube(&self, other: &Self) -> Self {
        let literals = self
            .literals
            .iter()
            .zip(&other.literals)
            .map(|(left, right)| {
                if left == right {
                    *left
                } else {
                    Literal::DontCare
                }
            })
            .collect();
        Self { literals }
    }

    fn literals_to_raise(&self, other: &Self) -> usize {
        self.literals
            .iter()
            .zip(&other.literals)
            .filter(|(left, right)| **left != Literal::DontCare && left != right)
            .count()
    }

    fn cofactor(&self, variable: usize, value: bool) -> Option<Self> {
        let mut literals = self.literals.clone();
        match literals[variable] {
            Literal::Zero if value => return None,
            Literal::One if !value => return None,
            Literal::Zero | Literal::One => literals[variable] = Literal::DontCare,
            Literal::DontCare => {}
        }
        Some(Self { literals })
    }

    fn constrain(&mut self, variable: usize, value: bool) {
        self.literals[variable] = if value { Literal::One } else { Literal::Zero };
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    width: usize,
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(width: usize, cubes: Vec<Cube>) -> Result<Self, DcsimpError> {
        for cube in &cubes {
            if cube.width() != width {
                return Err(DcsimpError::CubeWidth {
                    expected: width,
                    actual: cube.width(),
                });
            }
        }
        Ok(Self {
            width,
            cubes: contain(cubes),
        })
    }

    pub fn empty(width: usize) -> Self {
        Self {
            width,
            cubes: Vec::new(),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn cube_count(&self) -> usize {
        self.cubes.len()
    }

    pub fn literal_count(&self) -> usize {
        self.cubes.iter().map(Cube::literal_count).sum()
    }

    pub fn evaluates(&self, assignment: usize) -> bool {
        self.cubes
            .iter()
            .any(|cube| cube.contains_assignment(assignment))
    }

    fn push_contained(&mut self, cube: Cube) {
        if self.cubes.iter().any(|existing| existing.covers(&cube)) {
            return;
        }
        self.cubes.retain(|existing| !cube.covers(existing));
        self.cubes.push(cube);
    }

    fn union(&self, other: &Self) -> Self {
        let mut output = Self::empty(self.width);
        for cube in self.cubes.iter().chain(&other.cubes) {
            output.push_contained(cube.clone());
        }
        output
    }

    fn cofactor(&self, variable: usize, value: bool) -> Self {
        let cubes = self
            .cubes
            .iter()
            .filter_map(|cube| cube.cofactor(variable, value))
            .collect();
        Self {
            width: self.width,
            cubes: contain(cubes),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DcsimpOptions {
    pub verify: bool,
    pub large_cover_threshold: usize,
}

impl Default for DcsimpOptions {
    fn default() -> Self {
        Self {
            verify: true,
            large_cover_threshold: 20,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DcsimpError {
    CubeWidth { expected: usize, actual: usize },
    WidthMismatch { on_set: usize, dont_care: usize },
    TooManyInputs { width: usize, max_supported: usize },
    OnSetIntersectsDontCare { assignment: usize },
    VerificationFailed,
}

impl fmt::Display for DcsimpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CubeWidth { expected, actual } => {
                write!(
                    f,
                    "cube width {actual} does not match cover width {expected}"
                )
            }
            Self::WidthMismatch { on_set, dont_care } => {
                write!(
                    f,
                    "on-set width {on_set} does not match don't-care width {dont_care}"
                )
            }
            Self::TooManyInputs {
                width,
                max_supported,
            } => write!(
                f,
                "dcsimp exhaustive verification supports at most {max_supported} inputs, got {width}"
            ),
            Self::OnSetIntersectsDontCare { assignment } => write!(
                f,
                "on-set intersects don't-care set at assignment {assignment}"
            ),
            Self::VerificationFailed => write!(f, "dcsimp result does not preserve care behavior"),
        }
    }
}

impl Error for DcsimpError {}

pub fn dcsimp(on_set: &Cover, dont_care: &Cover) -> Result<Cover, DcsimpError> {
    dcsimp_with_options(on_set, dont_care, &DcsimpOptions::default())
}

pub fn dcsimp_with_options(
    on_set: &Cover,
    dont_care: &Cover,
    options: &DcsimpOptions,
) -> Result<Cover, DcsimpError> {
    validate_problem(on_set, dont_care)?;

    let essential = essential_cubes(on_set, dont_care)?;
    let mut reduced_source = Cover::empty(on_set.width);
    for cube in &on_set.cubes {
        if !essential.cubes.iter().any(|essential| essential == cube) {
            reduced_source.push_contained(cube.clone());
        }
    }

    let mut reduced = dc_simplify(&reduced_source, dont_care)?;
    if reduced.cube_count() > 0 {
        let ros = reduced_offset(&reduced, on_set, dont_care)?;
        if reduced.cube_count() > options.large_cover_threshold {
            let literal_distance = (on_set.width + 1) / 3;
            reduced = greedy_expand(reduced, &ros, literal_distance);
            reduced = reduce_against_dc(&reduced, dont_care)?;
        }

        if reduced.cube_count() > 1 {
            reduced = greedy_expand(reduced, &ros, on_set.width);
        }
        reduced = expand_to_largest(reduced, &ros);
    }

    let result = reduced.union(&essential);
    if options.verify && !verify_cover(&result, on_set, dont_care)? {
        return Err(DcsimpError::VerificationFailed);
    }

    Ok(result)
}

fn validate_problem(on_set: &Cover, dont_care: &Cover) -> Result<(), DcsimpError> {
    if on_set.width != dont_care.width {
        return Err(DcsimpError::WidthMismatch {
            on_set: on_set.width,
            dont_care: dont_care.width,
        });
    }
    validate_width(on_set.width)?;
    for assignment in assignments(on_set.width) {
        if on_set.evaluates(assignment) && dont_care.evaluates(assignment) {
            return Err(DcsimpError::OnSetIntersectsDontCare { assignment });
        }
    }
    Ok(())
}

fn validate_width(width: usize) -> Result<(), DcsimpError> {
    let max_supported = usize::BITS as usize - 1;
    if width > max_supported {
        return Err(DcsimpError::TooManyInputs {
            width,
            max_supported,
        });
    }
    Ok(())
}

fn assignments(width: usize) -> std::ops::Range<usize> {
    0..(1usize << width)
}

fn essential_cubes(on_set: &Cover, dont_care: &Cover) -> Result<Cover, DcsimpError> {
    let mut output = Cover::empty(on_set.width);
    for cube in &on_set.cubes {
        let has_private_minterm = assignments(on_set.width).any(|assignment| {
            cube.contains_assignment(assignment)
                && !dont_care.evaluates(assignment)
                && !on_set
                    .cubes
                    .iter()
                    .any(|other| other != cube && other.contains_assignment(assignment))
        });
        if has_private_minterm {
            output.push_contained(cube.clone());
        }
    }
    Ok(output)
}

fn dc_simplify(on_set: &Cover, dont_care: &Cover) -> Result<Cover, DcsimpError> {
    if on_set.cubes.is_empty() {
        return Ok(Cover::empty(on_set.width));
    }

    if is_unate(on_set) {
        return reduce_against_dc(on_set, dont_care);
    }

    let Some(variable) = most_binate_variable(on_set) else {
        return reduce_against_dc(on_set, dont_care);
    };

    let mut left = dc_simplify(
        &on_set.cofactor(variable, false),
        &dont_care.cofactor(variable, false),
    )?;
    for cube in &mut left.cubes {
        cube.constrain(variable, false);
    }

    let mut right = dc_simplify(
        &on_set.cofactor(variable, true),
        &dont_care.cofactor(variable, true),
    )?;
    for cube in &mut right.cubes {
        cube.constrain(variable, true);
    }

    Ok(left.union(&right))
}

fn is_unate(cover: &Cover) -> bool {
    (0..cover.width).all(|variable| {
        let has_zero = cover
            .cubes
            .iter()
            .any(|cube| cube.literals[variable] == Literal::Zero);
        let has_one = cover
            .cubes
            .iter()
            .any(|cube| cube.literals[variable] == Literal::One);
        !(has_zero && has_one)
    })
}

fn most_binate_variable(cover: &Cover) -> Option<usize> {
    let mut best = None;
    for variable in 0..cover.width {
        let zeros = cover
            .cubes
            .iter()
            .filter(|cube| cube.literals[variable] == Literal::Zero)
            .count();
        let ones = cover
            .cubes
            .iter()
            .filter(|cube| cube.literals[variable] == Literal::One)
            .count();
        if zeros == 0 || ones == 0 {
            continue;
        }

        let balance = zeros.min(ones);
        let activity = zeros + ones;
        if best
            .map(|(_, best_balance, best_activity)| {
                balance > best_balance || (balance == best_balance && activity > best_activity)
            })
            .unwrap_or(true)
        {
            best = Some((variable, balance, activity));
        }
    }
    best.map(|(variable, _, _)| variable)
}

fn reduce_against_dc(on_set: &Cover, dont_care: &Cover) -> Result<Cover, DcsimpError> {
    let allowed = on_set.union(dont_care);
    let mut output = Cover::empty(on_set.width);
    for cube in &on_set.cubes {
        let mut reduced = cube.clone();
        for variable in 0..on_set.width {
            if reduced.literals[variable] == Literal::DontCare {
                continue;
            }
            let previous = reduced.literals[variable];
            reduced.literals[variable] = Literal::DontCare;
            if !cube_care_subset_of_cover(&reduced, &allowed) {
                reduced.literals[variable] = previous;
            }
        }
        output.push_contained(reduced);
    }
    Ok(output)
}

fn cube_care_subset_of_cover(cube: &Cube, cover: &Cover) -> bool {
    assignments(cover.width)
        .all(|assignment| !cube.contains_assignment(assignment) || cover.evaluates(assignment))
}

fn reduced_offset(
    reduced: &Cover,
    on_set: &Cover,
    dont_care: &Cover,
) -> Result<Cover, DcsimpError> {
    let allowed = on_set.union(dont_care);
    let mut cubes = Vec::new();
    for assignment in assignments(reduced.width) {
        if !allowed.evaluates(assignment) {
            cubes.push(minterm_cube(reduced.width, assignment));
        }
    }
    Cover::new(reduced.width, cubes)
}

fn minterm_cube(width: usize, assignment: usize) -> Cube {
    let literals = (0..width)
        .map(|index| {
            if ((assignment >> index) & 1) == 0 {
                Literal::Zero
            } else {
                Literal::One
            }
        })
        .collect();
    Cube { literals }
}

fn greedy_expand(mut cover: Cover, reduced_offset: &Cover, literal_distance: usize) -> Cover {
    cover
        .cubes
        .sort_by_key(|cube| std::cmp::Reverse(cube.dont_care_count()));

    let mut active = vec![true; cover.cubes.len()];
    let mut output = Cover::empty(cover.width);
    for index in 0..cover.cubes.len() {
        if !active[index] {
            continue;
        }

        let mut candidate = cover.cubes[index].clone();
        active[index] = false;

        for other_index in (index + 1)..cover.cubes.len() {
            if !active[other_index]
                || candidate.literals_to_raise(&cover.cubes[other_index]) >= literal_distance
            {
                continue;
            }

            let supercube = candidate.supercube(&cover.cubes[other_index]);
            if is_orthogonal_to_cover(&supercube, reduced_offset) {
                candidate = supercube;
                active[other_index] = false;
            }
        }

        output.push_contained(candidate);
    }
    output
}

fn is_orthogonal_to_cover(cube: &Cube, cover: &Cover) -> bool {
    cover.cubes.iter().all(|other| !cube.intersects(other))
}

fn expand_to_largest(mut cover: Cover, reduced_offset: &Cover) -> Cover {
    for cube in &mut cover.cubes {
        loop {
            let mut changed = false;
            for variable in 0..cover.width {
                if cube.literals[variable] == Literal::DontCare {
                    continue;
                }
                let previous = cube.literals[variable];
                cube.literals[variable] = Literal::DontCare;
                if is_orthogonal_to_cover(cube, reduced_offset) {
                    changed = true;
                } else {
                    cube.literals[variable] = previous;
                }
            }
            if !changed {
                break;
            }
        }
    }
    cover.cubes = contain(cover.cubes);
    cover
}

fn verify_cover(result: &Cover, on_set: &Cover, dont_care: &Cover) -> Result<bool, DcsimpError> {
    validate_width(result.width)?;
    Ok(assignments(result.width).all(|assignment| {
        dont_care.evaluates(assignment)
            || result.evaluates(assignment) == on_set.evaluates(assignment)
    }))
}

fn contain(mut cubes: Vec<Cube>) -> Vec<Cube> {
    let mut keep = BTreeSet::new();
    cubes.retain(|cube| {
        let key = cube
            .literals
            .iter()
            .map(|literal| match literal {
                Literal::Zero => 0_u8,
                Literal::One => 1,
                Literal::DontCare => 2,
            })
            .collect::<Vec<_>>();
        keep.insert(key)
    });

    let mut output = Vec::new();
    for (index, cube) in cubes.iter().enumerate() {
        let covered = cubes
            .iter()
            .enumerate()
            .any(|(other_index, other)| other_index != index && other.covers(cube));
        if !covered {
            output.push(cube.clone());
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(values: &[Literal]) -> Cube {
        Cube::new(values.to_vec())
    }

    fn cover(width: usize, cubes: Vec<Cube>) -> Cover {
        Cover::new(width, cubes).unwrap()
    }

    fn assert_preserves_care(result: &Cover, on_set: &Cover, dont_care: &Cover) {
        assert!(verify_cover(result, on_set, dont_care).unwrap());
    }

    #[test]
    fn empty_on_set_stays_empty() {
        let on_set = Cover::empty(2);
        let dont_care = Cover::empty(2);

        let result = dcsimp(&on_set, &dont_care).unwrap();

        assert_eq!(result, Cover::empty(2));
    }

    #[test]
    fn public_dcsimp_preserves_essential_cube_shape() {
        let on_set = cover(2, vec![cube(&[Literal::One, Literal::Zero])]);
        let dont_care = cover(2, vec![cube(&[Literal::One, Literal::One])]);

        let result = dcsimp(&on_set, &dont_care).unwrap();

        assert_eq!(result.cubes(), &[cube(&[Literal::One, Literal::Zero])]);
        assert_preserves_care(&result, &on_set, &dont_care);
    }

    #[test]
    fn greedy_expand_combines_adjacent_cubes_when_supercube_avoids_offset() {
        let reduced = cover(
            3,
            vec![
                cube(&[Literal::One, Literal::Zero, Literal::Zero]),
                cube(&[Literal::One, Literal::One, Literal::Zero]),
            ],
        );
        let reduced_offset = cover(
            3,
            vec![
                cube(&[Literal::Zero, Literal::Zero, Literal::Zero]),
                cube(&[Literal::Zero, Literal::One, Literal::Zero]),
                cube(&[Literal::Zero, Literal::Zero, Literal::One]),
                cube(&[Literal::Zero, Literal::One, Literal::One]),
                cube(&[Literal::One, Literal::Zero, Literal::One]),
                cube(&[Literal::One, Literal::One, Literal::One]),
            ],
        );

        let result = greedy_expand(reduced, &reduced_offset, 3);

        assert_eq!(
            result.cubes(),
            &[cube(&[Literal::One, Literal::DontCare, Literal::Zero])]
        );
    }

    #[test]
    fn expands_reduced_cube_to_largest_prime_against_offset() {
        let on_set = cover(3, vec![cube(&[Literal::One, Literal::Zero, Literal::Zero])]);
        let reduced_offset = cover(
            3,
            vec![
                cube(&[Literal::Zero, Literal::Zero, Literal::Zero]),
                cube(&[Literal::Zero, Literal::One, Literal::Zero]),
                cube(&[Literal::Zero, Literal::Zero, Literal::One]),
                cube(&[Literal::Zero, Literal::One, Literal::One]),
                cube(&[Literal::One, Literal::Zero, Literal::One]),
                cube(&[Literal::One, Literal::One, Literal::One]),
            ],
        );

        let result = expand_to_largest(on_set, &reduced_offset);

        assert_eq!(
            result.cubes(),
            &[cube(&[Literal::One, Literal::DontCare, Literal::Zero])]
        );
    }

    #[test]
    fn preserves_xor_care_function_when_no_dc_expansion_is_possible() {
        let on_set = cover(
            2,
            vec![
                cube(&[Literal::One, Literal::Zero]),
                cube(&[Literal::Zero, Literal::One]),
            ],
        );
        let dont_care = Cover::empty(2);

        let result = dcsimp(&on_set, &dont_care).unwrap();

        assert_eq!(result.cube_count(), 2);
        assert_preserves_care(&result, &on_set, &dont_care);
    }

    #[test]
    fn rejects_overlapping_on_set_and_dont_care() {
        let on_set = cover(1, vec![cube(&[Literal::One])]);
        let dont_care = cover(1, vec![cube(&[Literal::DontCare])]);

        let result = dcsimp(&on_set, &dont_care);

        assert_eq!(
            result,
            Err(DcsimpError::OnSetIntersectsDontCare { assignment: 1 })
        );
    }

    #[test]
    fn no_legacy_c_abi_or_source_dependency_metadata_is_present() {
        let source = include_str!("dcsimp.rs");

        let forbidden = [
            concat!("extern ", "\"C\""),
            concat!("no", "_mangle"),
            concat!("REQUIRED", "_"),
            concat!("Port", "Dependency"),
            concat!("bead", "_id"),
            concat!("source", "_file"),
            concat!("Logic", "Friday1", "-8j8"),
        ];

        for token in forbidden {
            assert!(!source.contains(token), "{token}");
        }
    }
}
