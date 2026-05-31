//! Native algebraic GCD helpers for SIS-style sum-of-products covers.
//!
//! The legacy `gcd.c` works over `node_t` and the node package's algebraic
//! division routines. This port keeps the same algorithmic shape in owned Rust
//! data so callers can compute prime factors and multi-node GCDs without a C
//! ABI shim.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub type GcdResult<T> = Result<T, GcdError>;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    pub const fn is_care(self) -> bool {
        !matches!(self, Self::DontCare)
    }

    const fn phase(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            Self::DontCare => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }

    pub fn tautology(input_count: usize) -> Self {
        Self {
            literals: vec![Literal::DontCare; input_count],
        }
    }

    pub fn literal(input_count: usize, index: usize, phase: bool) -> GcdResult<Self> {
        if index >= input_count {
            return Err(GcdError::InputOutOfRange { input_count, index });
        }

        let mut literals = vec![Literal::DontCare; input_count];
        literals[index] = if phase { Literal::One } else { Literal::Zero };
        Ok(Self { literals })
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

    fn divides(&self, dividend: &Self) -> GcdResult<bool> {
        self.ensure_same_width(dividend)?;

        Ok(self
            .literals
            .iter()
            .zip(&dividend.literals)
            .all(|(divisor, dividend)| !divisor.is_care() || divisor == dividend))
    }

    fn quotient_after_dividing_by(&self, divisor: &Self) -> GcdResult<Self> {
        self.ensure_same_width(divisor)?;

        let literals = self
            .literals
            .iter()
            .zip(&divisor.literals)
            .map(|(dividend, divisor)| {
                if divisor.is_care() {
                    Literal::DontCare
                } else {
                    *dividend
                }
            })
            .collect();

        Ok(Self { literals })
    }

    fn and(&self, other: &Self) -> GcdResult<Option<Self>> {
        self.ensure_same_width(other)?;

        let mut literals = Vec::with_capacity(self.width());
        for (left, right) in self.literals.iter().zip(&other.literals) {
            match (left.phase(), right.phase()) {
                (Some(left), Some(right)) if left != right => return Ok(None),
                (Some(_), _) => literals.push(*left),
                (_, Some(_)) => literals.push(*right),
                (None, None) => literals.push(Literal::DontCare),
            }
        }

        Ok(Some(Self { literals }))
    }

    fn covers(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| !left.is_care() || left == right)
    }

    fn merge_distance_one(&self, other: &Self) -> Option<Self> {
        let mut difference = None;
        let mut literals = self.literals.clone();

        for (index, (left, right)) in self.literals.iter().zip(&other.literals).enumerate() {
            if left == right {
                continue;
            }

            match (left.phase(), right.phase()) {
                (Some(left), Some(right)) if left != right && difference.is_none() => {
                    difference = Some(index);
                    literals[index] = Literal::DontCare;
                }
                _ => return None,
            }
        }

        difference.map(|_| Self { literals })
    }

    fn ensure_same_width(&self, other: &Self) -> GcdResult<()> {
        if self.width() == other.width() {
            Ok(())
        } else {
            Err(GcdError::MismatchedInputCount {
                left: self.width(),
                right: other.width(),
            })
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sop {
    input_count: usize,
    cubes: Vec<Cube>,
    primary_input: Option<usize>,
}

impl Sop {
    pub fn new(input_count: usize, cubes: Vec<Cube>) -> GcdResult<Self> {
        let mut sop = Self {
            input_count,
            cubes,
            primary_input: None,
        };
        sop.validate()?;
        sop.normalize();
        Ok(sop)
    }

    pub fn zero(input_count: usize) -> Self {
        Self {
            input_count,
            cubes: Vec::new(),
            primary_input: None,
        }
    }

    pub fn one(input_count: usize) -> Self {
        Self {
            input_count,
            cubes: vec![Cube::tautology(input_count)],
            primary_input: None,
        }
    }

    pub fn literal(input_count: usize, index: usize, phase: bool) -> GcdResult<Self> {
        Ok(Self {
            input_count,
            cubes: vec![Cube::literal(input_count, index, phase)?],
            primary_input: Some(index),
        })
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn input_count(&self) -> usize {
        self.input_count
    }

    pub fn is_zero(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn is_one(&self) -> bool {
        self.cubes.len() == 1 && self.cubes[0].literal_count() == 0
    }

    pub fn is_primary_input(&self) -> bool {
        self.primary_input.is_some()
    }

    pub fn num_cubes(&self) -> usize {
        self.cubes.len()
    }

    pub fn largest_cube_divisor(&self) -> Self {
        if self.cubes.is_empty() {
            return Self::one(self.input_count);
        }

        let mut common = self.cubes[0].literals.clone();
        for cube in self.cubes.iter().skip(1) {
            for (literal, candidate) in common.iter_mut().zip(&cube.literals) {
                if literal != candidate {
                    *literal = Literal::DontCare;
                }
            }
        }

        Self {
            input_count: self.input_count,
            cubes: vec![Cube::new(common)],
            primary_input: None,
        }
    }

    pub fn literal_counts(&self) -> Vec<usize> {
        let mut counts = vec![0; self.input_count * 2];
        for cube in &self.cubes {
            for (index, literal) in cube.literals.iter().enumerate() {
                match literal {
                    Literal::Zero => counts[index * 2] += 1,
                    Literal::One => counts[index * 2 + 1] += 1,
                    Literal::DontCare => {}
                }
            }
        }
        counts
    }

    fn validate(&self) -> GcdResult<()> {
        if let Some(cube) = self
            .cubes
            .iter()
            .find(|cube| cube.width() != self.input_count)
        {
            return Err(GcdError::MismatchedInputCount {
                left: self.input_count,
                right: cube.width(),
            });
        }

        Ok(())
    }

    fn normalize(&mut self) {
        self.cubes.sort();
        self.cubes.dedup();

        loop {
            let mut merged = false;

            'outer: for left_index in 0..self.cubes.len() {
                for right_index in left_index + 1..self.cubes.len() {
                    if let Some(cube) =
                        self.cubes[left_index].merge_distance_one(&self.cubes[right_index])
                    {
                        self.cubes.remove(right_index);
                        self.cubes[left_index] = cube;
                        self.cubes.sort();
                        self.cubes.dedup();
                        merged = true;
                        break 'outer;
                    }
                }
            }

            if !merged {
                break;
            }
        }

        let mut keep = Vec::with_capacity(self.cubes.len());
        for (index, cube) in self.cubes.iter().enumerate() {
            let covered = self
                .cubes
                .iter()
                .enumerate()
                .any(|(other_index, other)| other_index != index && other.covers(cube));
            if !covered {
                keep.push(cube.clone());
            }
        }

        self.cubes = keep;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GcdError {
    EmptyInput,
    AllInputsAreZero,
    DivideByZero,
    NoSplitLiteral,
    InputOutOfRange { input_count: usize, index: usize },
    MismatchedInputCount { left: usize, right: usize },
}

impl fmt::Display for GcdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "gcd input vector is empty"),
            Self::AllInputsAreZero => write!(f, "gcd input vector contains only zero functions"),
            Self::DivideByZero => write!(f, "cannot divide by the zero function"),
            Self::NoSplitLiteral => write!(f, "no literal was available for prime-factor split"),
            Self::InputOutOfRange { input_count, index } => {
                write!(f, "input index {index} is outside {input_count} inputs")
            }
            Self::MismatchedInputCount { left, right } => {
                write!(f, "SOP input counts differ: {left} vs {right}")
            }
        }
    }
}

impl Error for GcdError {}

pub fn gcd_prime_factorize(function: &Sop) -> GcdResult<Vec<Sop>> {
    if function.is_primary_input() || function.num_cubes() <= 1 {
        return Ok(vec![function.clone()]);
    }

    let mut result = vec![function.largest_cube_divisor()];
    let mut remaining = node_div(function, &result[0])?.quotient;

    while !remaining.is_one() {
        let (factor, cofactor) = find_prime_factor(&remaining)?;
        result.push(factor);
        remaining = cofactor;
    }

    Ok(result)
}

pub fn gcd_nodevec(functions: &[Sop]) -> GcdResult<Sop> {
    if functions.is_empty() {
        return Err(GcdError::EmptyInput);
    }

    let mut nodes = functions
        .iter()
        .filter(|function| !function.is_zero())
        .cloned()
        .collect::<Vec<_>>();

    if nodes.is_empty() {
        return Err(GcdError::AllInputsAreZero);
    }

    ensure_same_input_count(&nodes)?;

    if nodes.len() == 1 {
        return Ok(nodes.remove(0));
    }

    let mut result = nodes[0].largest_cube_divisor();
    nodes[0] = node_div(&nodes[0], &result)?.quotient;
    let mut best_index = 0;
    let mut best_num = nodes[0].num_cubes();

    for index in 1..nodes.len() {
        if nodes[index].is_one() {
            return Ok(Sop::one(nodes[index].input_count()));
        }

        let next_cube = nodes[index].largest_cube_divisor();
        nodes[index] = node_div(&nodes[index], &next_cube)?.quotient;

        if !result.is_one() && !next_cube.is_one() {
            result = node_or(&result, &next_cube)?.largest_cube_divisor();
        }

        let new_num = nodes[index].num_cubes();
        if new_num < best_num {
            best_num = new_num;
            best_index = index;
        }
    }

    nodes.swap(0, best_index);
    let gcdvec = gcd_prime_factorize(&nodes[0])?;

    for divisor in gcdvec.iter().skip(1) {
        let mut divides_every_node = true;
        let mut divided_nodes = nodes.clone();

        for index in 1..nodes.len() {
            let division = node_div(&nodes[index], divisor)?;
            if division.remainder.is_zero() {
                divided_nodes[index] = division.quotient;
            } else {
                divides_every_node = false;
                break;
            }
        }

        if divides_every_node {
            result = node_and(&result, divisor)?;
            nodes = divided_nodes;

            if nodes.iter().skip(1).any(Sop::is_one) {
                break;
            }
        }
    }

    Ok(result)
}

pub fn internal_gcd(q: &Sop, r: &Sop) -> GcdResult<Sop> {
    q.ensure_compatible(r)?;

    if q.is_one() {
        return Ok(q.clone());
    }

    let (mut u, u_cube, mut v, v_cube) = if q.num_cubes() < r.num_cubes() {
        let (u, u_cube) = make_cube_free(q)?;
        let (v, v_cube) = make_cube_free(r)?;
        (u, u_cube, v, v_cube)
    } else {
        let (u, v_cube) = make_cube_free(r)?;
        let (v, u_cube) = make_cube_free(q)?;
        (u, u_cube, v, v_cube)
    };

    let mut result = node_or(&u_cube, &v_cube)?.largest_cube_divisor();

    while !u.is_one() {
        let (factor, next_u) = find_prime_factor(&u)?;
        let division = node_div(&v, &factor)?;

        if division.remainder.is_zero() {
            v = division.quotient;
            result = node_and(&result, &factor)?;
        }

        u = next_u;
    }

    Ok(result)
}

pub fn make_cube_free(function: &Sop) -> GcdResult<(Sop, Sop)> {
    let cube = function.largest_cube_divisor();
    let quotient = node_div(function, &cube)?.quotient;
    Ok((quotient, cube))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Division {
    pub quotient: Sop,
    pub remainder: Sop,
}

pub fn node_div(dividend: &Sop, divisor: &Sop) -> GcdResult<Division> {
    dividend.ensure_compatible(divisor)?;

    if divisor.is_zero() {
        return Err(GcdError::DivideByZero);
    }

    if dividend.is_zero() {
        return Ok(Division {
            quotient: Sop::zero(dividend.input_count),
            remainder: dividend.clone(),
        });
    }

    if divisor.is_one() {
        return Ok(Division {
            quotient: dividend.clone(),
            remainder: Sop::zero(dividend.input_count),
        });
    }

    let mut candidates = BTreeSet::new();
    for dividend_cube in &dividend.cubes {
        for divisor_cube in &divisor.cubes {
            if divisor_cube.divides(dividend_cube)? {
                candidates.insert(dividend_cube.quotient_after_dividing_by(divisor_cube)?);
            }
        }
    }

    let dividend_set = dividend.cubes.iter().cloned().collect::<BTreeSet<_>>();
    let mut quotient_cubes = Vec::new();
    let mut consumed = BTreeSet::new();

    for candidate in candidates {
        let mut products = Vec::new();
        let mut valid = true;

        for divisor_cube in &divisor.cubes {
            match candidate.and(divisor_cube)? {
                Some(product) if dividend_set.contains(&product) => products.push(product),
                _ => {
                    valid = false;
                    break;
                }
            }
        }

        if valid {
            quotient_cubes.push(candidate);
            consumed.extend(products);
        }
    }

    let quotient = Sop::new(dividend.input_count, quotient_cubes)?;
    let remainder = Sop::new(
        dividend.input_count,
        dividend
            .cubes
            .iter()
            .filter(|cube| !consumed.contains(*cube))
            .cloned()
            .collect(),
    )?;

    Ok(Division {
        quotient,
        remainder,
    })
}

pub fn node_and(left: &Sop, right: &Sop) -> GcdResult<Sop> {
    left.ensure_compatible(right)?;

    let mut cubes = Vec::new();
    for left_cube in &left.cubes {
        for right_cube in &right.cubes {
            if let Some(cube) = left_cube.and(right_cube)? {
                cubes.push(cube);
            }
        }
    }

    Sop::new(left.input_count, cubes)
}

pub fn node_or(left: &Sop, right: &Sop) -> GcdResult<Sop> {
    left.ensure_compatible(right)?;

    let mut cubes = left.cubes.clone();
    cubes.extend(right.cubes.iter().cloned());
    Sop::new(left.input_count, cubes)
}

fn find_prime_factor(function: &Sop) -> GcdResult<(Sop, Sop)> {
    let num_cubes = function.num_cubes();
    if num_cubes <= 1 {
        return Ok((function.clone(), Sop::one(function.input_count())));
    }

    let literal_counts = function.literal_counts();
    let mut best = num_cubes;
    let mut best_literal = None;

    for input in 0..function.input_count() {
        for phase in [true, false] {
            let count_index = input * 2 + usize::from(phase);
            let count = literal_counts[count_index];
            let literal_value = count.min(num_cubes - count);
            if literal_value < best && literal_value != 0 {
                best = literal_value;
                best_literal = Some((input, phase));
            }
        }
    }

    let (input, phase) = best_literal.ok_or(GcdError::NoSplitLiteral)?;
    let literal = Sop::literal(function.input_count(), input, phase)?;
    let split = node_div(function, &literal)?;
    let gcd = internal_gcd(&split.quotient, &split.remainder)?;
    let prime_factor = node_div(function, &gcd)?.quotient;

    Ok((prime_factor, gcd))
}

fn ensure_same_input_count(functions: &[Sop]) -> GcdResult<()> {
    if let Some((first, rest)) = functions.split_first() {
        for function in rest {
            first.ensure_compatible(function)?;
        }
    }

    Ok(())
}

impl Sop {
    fn ensure_compatible(&self, other: &Self) -> GcdResult<()> {
        if self.input_count == other.input_count {
            Ok(())
        } else {
            Err(GcdError::MismatchedInputCount {
                left: self.input_count,
                right: other.input_count,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(pattern: &str) -> Cube {
        Cube::new(
            pattern
                .chars()
                .map(|character| match character {
                    '0' => Literal::Zero,
                    '1' => Literal::One,
                    '-' => Literal::DontCare,
                    _ => panic!("unsupported cube character {character}"),
                })
                .collect(),
        )
    }

    fn sop(patterns: &[&str]) -> Sop {
        let input_count = patterns.first().map_or(0, |pattern| pattern.len());
        Sop::new(
            input_count,
            patterns.iter().map(|pattern| cube(pattern)).collect(),
        )
        .unwrap()
    }

    #[test]
    fn largest_cube_divisor_keeps_common_literals() {
        let function = sop(&["11-", "101"]);

        let divisor = function.largest_cube_divisor();

        assert_eq!(divisor, sop(&["1--"]));
    }

    #[test]
    fn node_div_returns_quotient_and_remainder() {
        let dividend = sop(&["110", "111", "001"]);
        let divisor = sop(&["1--"]);

        let division = node_div(&dividend, &divisor).unwrap();

        assert_eq!(division.quotient, sop(&["-10", "-11"]));
        assert_eq!(division.remainder, sop(&["001"]));
    }

    #[test]
    fn make_cube_free_splits_common_cube_from_function() {
        let function = sop(&["110", "111"]);

        let (cube_free, common_cube) = make_cube_free(&function).unwrap();

        assert_eq!(common_cube, sop(&["11-"]));
        assert_eq!(cube_free, Sop::one(3));
    }

    #[test]
    fn internal_gcd_combines_cube_and_prime_factors() {
        let left = sop(&["110", "111"]);
        let right = sop(&["100", "101"]);

        let gcd = internal_gcd(&left, &right).unwrap();

        assert_eq!(gcd, sop(&["1--"]));
    }

    #[test]
    fn prime_factorize_starts_with_largest_cube_divisor() {
        let function = sop(&["110", "101"]);

        let factors = gcd_prime_factorize(&function).unwrap();

        assert_eq!(factors.first(), Some(&sop(&["1--"])));
        assert_eq!(node_and(&factors[0], &factors[1]).unwrap(), function);
    }

    #[test]
    fn prime_factorize_leaves_primary_input_unchanged() {
        let input = Sop::literal(3, 1, true).unwrap();

        let factors = gcd_prime_factorize(&input).unwrap();

        assert_eq!(factors, vec![input]);
    }

    #[test]
    fn gcd_nodevec_ignores_zero_inputs() {
        let zero = Sop::zero(3);
        let first = sop(&["110", "111"]);
        let second = sop(&["100", "101"]);

        let gcd = gcd_nodevec(&[zero, first, second]).unwrap();

        assert_eq!(gcd, sop(&["1--"]));
    }

    #[test]
    fn gcd_nodevec_returns_one_when_any_cube_free_input_is_one() {
        let one = Sop::one(2);
        let function = sop(&["10", "11"]);

        let gcd = gcd_nodevec(&[function, one]).unwrap();

        assert_eq!(gcd, Sop::one(2));
    }

    #[test]
    fn gcd_nodevec_finds_non_cube_common_factor() {
        let first = sop(&["110", "101"]);
        let second = sop(&["110", "100"]);

        let gcd = gcd_nodevec(&[first, second]).unwrap();

        assert_eq!(gcd, sop(&["1--"]));
    }

    #[test]
    fn gcd_nodevec_rejects_all_zero_inputs() {
        let error = gcd_nodevec(&[Sop::zero(2), Sop::zero(2)]).unwrap_err();

        assert_eq!(error, GcdError::AllInputsAreZero);
    }

    #[test]
    fn incompatible_input_counts_are_rejected() {
        let error = gcd_nodevec(&[sop(&["10"]), sop(&["100"])]).unwrap_err();

        assert_eq!(error, GcdError::MismatchedInputCount { left: 2, right: 3 });
    }
}
