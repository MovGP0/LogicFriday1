//! Native state-encoding interface for dichotomy constraints.
//!
//! The original routine builds seed dichotomies from symbolic constraints,
//! expands them into valid prime dichotomies, solves a set-cover instance, and
//! derives one binary code bit per selected prime. This module keeps that flow
//! on owned Rust values and leaves external interop to higher-level facade
//! boundaries.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

const PRIME_LIMIT: usize = 5_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodingOptions
{
    pub dominance_edges: Vec<(usize, usize)>,
    pub prime_limit: usize,
}

impl Default for EncodingOptions
{
    fn default() -> Self
    {
        Self
        {
            dominance_edges: Vec::new(),
            prime_limit: PRIME_LIMIT,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodingResult
{
    pub code_length: usize,
    pub codes: Vec<String>,
    pub selected_primes: Vec<Dichotomy>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncodingError
{
    EmptyConstraintSet,
    EmptySymbolSet,
    InconsistentSymbolCount
    {
        expected: usize,
        actual: usize,
    },
    InvalidConstraintSymbol
    {
        symbol: char,
        index: usize,
    },
    DominanceIndexOutOfRange
    {
        index: usize,
        symbol_count: usize,
    },
    CyclicDominance
    {
        index: usize,
    },
    PrimeLimitExceeded
    {
        limit: usize,
    },
    UnsatisfiableSeeds
    {
        uncovered: Vec<Dichotomy>,
    },
}

impl fmt::Display for EncodingError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyConstraintSet => write!(f, "state encoding requires at least one constraint"),
            Self::EmptySymbolSet => write!(f, "state encoding requires at least one symbol"),
            Self::InconsistentSymbolCount { expected, actual } => write!(
                f,
                "state encoding constraint has {actual} symbols; expected {expected}"
            ),
            Self::InvalidConstraintSymbol { symbol, index } => write!(
                f,
                "invalid state encoding constraint symbol '{symbol}' at index {index}"
            ),
            Self::DominanceIndexOutOfRange {
                index,
                symbol_count,
            } => write!(
                f,
                "dominance constraint index {index} is outside symbol count {symbol_count}"
            ),
            Self::CyclicDominance { index } => {
                write!(f, "dominance constraints contain a cycle at symbol {index}")
            }
            Self::PrimeLimitExceeded { limit } => {
                write!(f, "state encoding prime cover exceeded limit {limit}")
            }
            Self::UnsatisfiableSeeds { uncovered } => write!(
                f,
                "state encoding leaves {} seed dichotomies uncovered",
                uncovered.len()
            ),
        }
    }
}

impl Error for EncodingError {}

pub type EncodingResultValue<T> = Result<T, EncodingError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicConstraint
{
    values: Vec<ConstraintValue>,
}

impl SymbolicConstraint
{
    pub fn parse(input: &str) -> EncodingResultValue<Self>
    {
        let values = input
            .chars()
            .enumerate()
            .map(|(index, symbol)| match symbol
            {
                '0' => Ok(ConstraintValue::Zero),
                '1' => Ok(ConstraintValue::One),
                '-' => Ok(ConstraintValue::DontCare),
                _ => Err(EncodingError::InvalidConstraintSymbol { symbol, index }),
            })
            .collect::<EncodingResultValue<Vec<_>>>()?;

        if values.is_empty()
        {
            return Err(EncodingError::EmptySymbolSet);
        }

        Ok(Self { values })
    }

    pub fn symbol_count(&self) -> usize
    {
        self.values.len()
    }

    pub fn values(&self) -> &[ConstraintValue]
    {
        &self.values
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintValue
{
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Dichotomy
{
    lhs: BTreeSet<usize>,
    rhs: BTreeSet<usize>,
}

impl Dichotomy
{
    pub fn new<L, R>(lhs: L, rhs: R) -> Self
    where
        L: IntoIterator<Item = usize>,
        R: IntoIterator<Item = usize>,
    {
        Self
        {
            lhs: lhs.into_iter().collect(),
            rhs: rhs.into_iter().collect(),
        }
    }

    pub fn lhs(&self) -> &BTreeSet<usize>
    {
        &self.lhs
    }

    pub fn rhs(&self) -> &BTreeSet<usize>
    {
        &self.rhs
    }

    fn implies(&self, other: &Self) -> bool
    {
        self.lhs.is_subset(&other.lhs) && self.rhs.is_subset(&other.rhs)
    }

    fn covers(&self, other: &Self) -> bool
    {
        (self.lhs.is_subset(&other.lhs) && self.rhs.is_subset(&other.rhs))
            || (self.lhs.is_subset(&other.rhs) && self.rhs.is_subset(&other.lhs))
    }

    fn has_symbol(&self, symbol: usize) -> bool
    {
        self.lhs.contains(&symbol) || self.rhs.contains(&symbol)
    }

    fn is_compatible_with(&self, other: &Self) -> bool
    {
        self.lhs.is_disjoint(&other.rhs) && self.rhs.is_disjoint(&other.lhs)
    }

    fn union_from<'a>(items: impl IntoIterator<Item = &'a Dichotomy>) -> Self
    {
        let mut lhs = BTreeSet::new();
        let mut rhs = BTreeSet::new();

        for item in items
        {
            lhs.extend(item.lhs.iter().copied());
            rhs.extend(item.rhs.iter().copied());
        }

        Self { lhs, rhs }
    }
}

pub fn encode_constraint_patterns<I, S>(patterns: I) -> EncodingResultValue<EncodingResult>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    encode_constraint_patterns_with_options(patterns, &EncodingOptions::default())
}

pub fn encode_constraint_patterns_with_options<I, S>(
    patterns: I,
    options: &EncodingOptions,
) -> EncodingResultValue<EncodingResult>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let constraints = patterns
        .into_iter()
        .map(|pattern| SymbolicConstraint::parse(pattern.as_ref()))
        .collect::<EncodingResultValue<Vec<_>>>()?;

    encode_constraints_with_options(&constraints, options)
}

pub fn encode_constraints(constraints: &[SymbolicConstraint]) -> EncodingResultValue<EncodingResult>
{
    encode_constraints_with_options(constraints, &EncodingOptions::default())
}

pub fn encode_constraints_with_options(
    constraints: &[SymbolicConstraint],
    options: &EncodingOptions,
) -> EncodingResultValue<EncodingResult>
{
    let symbol_count = validate_constraints(constraints)?;
    let closure = transitive_closure(symbol_count, &options.dominance_edges)?;

    let seed_list = build_seed_dichotomies(constraints);
    if seed_list.is_empty()
    {
        return Ok(EncodingResult
        {
            code_length: 0,
            codes: vec![String::new(); symbol_count],
            selected_primes: Vec::new(),
        });
    }

    let unique_seeds = add_missing_pair_seeds(&seed_list, symbol_count);
    let reduced_seeds = reduce_seeds(&unique_seeds, symbol_count);
    let raised_seeds = filter_by_dominance(&reduced_seeds, &closure);
    let filter_list = reduce_seeds(&raised_seeds, symbol_count);
    let prime_list = generate_prime_dichotomies(&filter_list, symbol_count, options.prime_limit)?;
    let mut satisfy_list = filter_by_dominance(&prime_list, &closure);
    for seed in &reduced_seeds
    {
        if !satisfy_list.iter().any(|prime| seed.covers(prime))
        {
            satisfy_list.push(seed.clone());
        }
    }

    satisfy_list.sort();
    satisfy_list.dedup();

    let (matrix, uncovered) = build_cover_matrix(&satisfy_list, &reduced_seeds);

    if !uncovered.is_empty()
    {
        return Err(EncodingError::UnsatisfiableSeeds { uncovered });
    }

    let cover = minimum_cover(&matrix, satisfy_list.len());
    let selected_primes = cover
        .iter()
        .map(|column| satisfy_list[*column].clone())
        .collect::<Vec<_>>();
    let codes = derive_codes(&selected_primes, symbol_count);

    Ok(EncodingResult
    {
        code_length: selected_primes.len(),
        codes,
        selected_primes,
    })
}

fn validate_constraints(constraints: &[SymbolicConstraint]) -> EncodingResultValue<usize>
{
    let Some(first) = constraints.first() else
    {
        return Err(EncodingError::EmptyConstraintSet);
    };

    let symbol_count = first.symbol_count();
    for constraint in constraints.iter().skip(1)
    {
        if constraint.symbol_count() != symbol_count
        {
            return Err(EncodingError::InconsistentSymbolCount
            {
                expected: symbol_count,
                actual: constraint.symbol_count(),
            });
        }
    }

    Ok(symbol_count)
}

fn transitive_closure(
    symbol_count: usize,
    edges: &[(usize, usize)],
) -> EncodingResultValue<Vec<Vec<bool>>>
{
    let mut closure = vec![vec![false; symbol_count]; symbol_count];

    for &(from, to) in edges
    {
        if from >= symbol_count
        {
            return Err(EncodingError::DominanceIndexOutOfRange
            {
                index: from,
                symbol_count,
            });
        }

        if to >= symbol_count
        {
            return Err(EncodingError::DominanceIndexOutOfRange
            {
                index: to,
                symbol_count,
            });
        }

        closure[from][to] = true;
    }

    for pivot in 0..symbol_count
    {
        for from in 0..symbol_count
        {
            if !closure[from][pivot]
            {
                continue;
            }

            for to in 0..symbol_count
            {
                closure[from][to] |= closure[pivot][to];
            }
        }
    }

    for (index, row) in closure.iter().enumerate()
    {
        if row[index]
        {
            return Err(EncodingError::CyclicDominance { index });
        }
    }

    Ok(closure)
}

fn build_seed_dichotomies(constraints: &[SymbolicConstraint]) -> Vec<Dichotomy>
{
    let mut result = Vec::new();

    for constraint in constraints
    {
        let lhs = constraint
            .values()
            .iter()
            .enumerate()
            .filter_map(|(index, value)| (*value == ConstraintValue::One).then_some(index))
            .collect::<BTreeSet<_>>();

        for (index, value) in constraint.values().iter().enumerate()
        {
            if *value != ConstraintValue::Zero
            {
                continue;
            }

            add_irredundant(&mut result, Dichotomy::new(lhs.iter().copied(), [index]));
            add_irredundant(&mut result, Dichotomy::new([index], lhs.iter().copied()));
        }
    }

    result
}

fn add_missing_pair_seeds(seeds: &[Dichotomy], symbol_count: usize) -> Vec<Dichotomy>
{
    let mut result = seeds.to_vec();
    let mut implied = vec![vec![false; symbol_count]; symbol_count];

    for seed in seeds
    {
        for lhs in seed.lhs()
        {
            for rhs in seed.rhs()
            {
                let low = (*lhs).min(*rhs);
                let high = (*lhs).max(*rhs);
                implied[low][high] = true;
            }
        }
    }

    for lhs in 0..symbol_count
    {
        for rhs in (lhs + 1)..symbol_count
        {
            if !implied[lhs][rhs]
            {
                result.push(Dichotomy::new([lhs], [rhs]));
                result.push(Dichotomy::new([rhs], [lhs]));
            }
        }
    }

    result
}

fn reduce_seeds(seeds: &[Dichotomy], symbol_count: usize) -> Vec<Dichotomy>
{
    let mut active = vec![true; seeds.len()];
    for index in 0..seeds.len()
    {
        if !active[index]
        {
            continue;
        }

        for next in (index + 1)..seeds.len()
        {
            if seeds[next].implies(&seeds[index])
            {
                active[next] = false;
            }
        }
    }

    let reduced = seeds
        .iter()
        .zip(active)
        .filter_map(|(seed, active)| active.then_some(seed.clone()))
        .collect::<Vec<_>>();

    let Some(anchor) = most_frequent_symbol(&reduced, symbol_count) else
    {
        return reduced;
    };

    reduced
        .into_iter()
        .filter(|seed| !seed.lhs().contains(&anchor))
        .collect()
}

fn most_frequent_symbol(seeds: &[Dichotomy], symbol_count: usize) -> Option<usize>
{
    let mut counts = vec![0usize; symbol_count];
    for seed in seeds
    {
        for symbol in 0..symbol_count
        {
            if seed.has_symbol(symbol)
            {
                counts[symbol] += 1;
            }
        }
    }

    counts
        .into_iter()
        .enumerate()
        .max_by_key(|(_, count)| *count)
        .and_then(|(index, count)| (count > 0).then_some(index))
}

fn filter_by_dominance(seeds: &[Dichotomy], closure: &[Vec<bool>]) -> Vec<Dichotomy>
{
    seeds
        .iter()
        .filter(|seed| is_dominance_consistent(seed, closure))
        .cloned()
        .collect()
}

fn is_dominance_consistent(seed: &Dichotomy, closure: &[Vec<bool>]) -> bool
{
    for lhs in seed.lhs()
    {
        for rhs in seed.rhs()
        {
            if closure[*lhs][*rhs] || closure[*rhs][*lhs]
            {
                return false;
            }
        }
    }

    true
}

fn generate_prime_dichotomies(
    seeds: &[Dichotomy],
    symbol_count: usize,
    limit: usize,
) -> EncodingResultValue<Vec<Dichotomy>>
{
    let mut clauses = Vec::new();
    for left in 0..seeds.len()
    {
        for right in (left + 1)..seeds.len()
        {
            if !seeds[left].is_compatible_with(&seeds[right])
            {
                clauses.push((left, right));
            }
        }
    }

    let covers = expand_cnf(&clauses, seeds.len(), limit)?;
    let mut primes = covers
        .iter()
        .map(|cover| {
            Dichotomy::union_from(
                (0..seeds.len())
                    .filter(|index| !cover.contains(index))
                    .map(|index| &seeds[index]),
            )
        })
        .filter(|prime| !prime.lhs().is_empty() || !prime.rhs().is_empty())
        .filter(|prime| prime.lhs().iter().all(|symbol| *symbol < symbol_count))
        .filter(|prime| prime.rhs().iter().all(|symbol| *symbol < symbol_count))
        .collect::<Vec<_>>();

    for seed in seeds
    {
        if !primes.iter().any(|prime| seed.covers(prime))
        {
            primes.push(seed.clone());
        }
    }

    primes.sort();
    primes.dedup();
    Ok(primes)
}

fn expand_cnf(
    clauses: &[(usize, usize)],
    variable_count: usize,
    limit: usize,
) -> EncodingResultValue<Vec<BTreeSet<usize>>>
{
    if clauses.is_empty()
    {
        return Ok(vec![BTreeSet::new()]);
    }

    let mut counts = vec![0usize; variable_count];
    for &(left, right) in clauses
    {
        counts[left] += 1;
        counts[right] += 1;
    }

    let mut result = expand_cnf_with_counts(clauses.to_vec(), counts, variable_count, limit)?;
    result.sort();
    result.dedup();
    Ok(remove_contained_sets(result))
}

fn expand_cnf_with_counts(
    clauses: Vec<(usize, usize)>,
    counts: Vec<usize>,
    variable_count: usize,
    limit: usize,
) -> EncodingResultValue<Vec<BTreeSet<usize>>>
{
    if clauses.is_empty()
    {
        return Ok(vec![BTreeSet::new()]);
    }

    if let Some(special) = expand_special_case(clauses.len(), &counts, variable_count)
    {
        return limited(special, limit);
    }

    let split = counts
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .map(|(index, _)| index)
        .unwrap_or(0);

    let mut left_clauses = Vec::new();
    let mut next_counts = counts;
    let mut rhs_product = BTreeSet::new();

    for (left, right) in clauses
    {
        if left == split
        {
            next_counts[split] -= 1;
            next_counts[right] -= 1;
            rhs_product.insert(right);
        }
        else if right == split
        {
            next_counts[split] -= 1;
            next_counts[left] -= 1;
            rhs_product.insert(left);
        }
        else
        {
            left_clauses.push((left, right));
        }
    }

    let left_cover = expand_cnf_with_counts(left_clauses, next_counts, variable_count, limit)?;
    limited(merge_covers(variable_count, split, &rhs_product, &left_cover), limit)
}

fn expand_special_case(
    term_count: usize,
    counts: &[usize],
    variable_count: usize,
) -> Option<Vec<BTreeSet<usize>>>
{
    for variable in 0..variable_count
    {
        if counts[variable] == term_count
        {
            let mut rest = BTreeSet::new();
            for (index, count) in counts.iter().enumerate()
            {
                if *count != 0 && index != variable
                {
                    rest.insert(index);
                }
            }

            return Some(vec![BTreeSet::from([variable]), rest]);
        }
    }

    None
}

fn merge_covers(
    _variable_count: usize,
    split: usize,
    rhs_product: &BTreeSet<usize>,
    left_cover: &[BTreeSet<usize>],
) -> Vec<BTreeSet<usize>>
{
    let mut sop = left_cover
        .iter()
        .map(|cover| {
            let mut merged = cover.clone();
            merged.extend(rhs_product.iter().copied());
            merged
        })
        .collect::<Vec<_>>();

    if sop.iter().any(|cover| cover.is_subset(rhs_product))
    {
        sop = vec![rhs_product.clone()];
    }
    else
    {
        sop = remove_contained_sets(sop);
    }

    for cover in left_cover
    {
        if rhs_product.is_subset(cover)
        {
            continue;
        }

        let mut with_split = cover.clone();
        with_split.insert(split);
        sop.push(with_split);
    }

    remove_contained_sets(sop)
}

fn remove_contained_sets(mut sets: Vec<BTreeSet<usize>>) -> Vec<BTreeSet<usize>>
{
    sets.sort();
    sets.dedup();

    let mut keep = vec![true; sets.len()];
    for left in 0..sets.len()
    {
        for right in 0..sets.len()
        {
            if left != right && sets[left].is_superset(&sets[right])
            {
                keep[left] = false;
                break;
            }
        }
    }

    sets.into_iter()
        .zip(keep)
        .filter_map(|(set, keep)| keep.then_some(set))
        .collect()
}

fn limited<T>(items: Vec<T>, limit: usize) -> EncodingResultValue<Vec<T>>
{
    if items.len() >= limit
    {
        return Err(EncodingError::PrimeLimitExceeded { limit });
    }

    Ok(items)
}

fn build_cover_matrix(
    prime_list: &[Dichotomy],
    seed_list: &[Dichotomy],
) -> (Vec<BTreeSet<usize>>, Vec<Dichotomy>)
{
    let mut matrix = vec![BTreeSet::new(); seed_list.len()];
    let mut covered = vec![false; seed_list.len()];

    for (column, prime) in prime_list.iter().enumerate()
    {
        for (row, seed) in seed_list.iter().enumerate()
        {
            if seed.covers(prime)
            {
                matrix[row].insert(column);
                covered[row] = true;
            }
        }
    }

    let uncovered = seed_list
        .iter()
        .zip(covered)
        .filter_map(|(seed, covered)| (!covered).then_some(seed.clone()))
        .collect();

    (matrix, uncovered)
}

fn minimum_cover(matrix: &[BTreeSet<usize>], column_count: usize) -> Vec<usize>
{
    if matrix.is_empty()
    {
        return Vec::new();
    }

    let mut best = greedy_cover(matrix, column_count);
    search_cover(matrix, &mut BTreeSet::new(), &mut best);
    best
}

fn greedy_cover(matrix: &[BTreeSet<usize>], column_count: usize) -> Vec<usize>
{
    let mut uncovered = (0..matrix.len()).collect::<BTreeSet<_>>();
    let mut cover = Vec::new();

    while !uncovered.is_empty()
    {
        let Some(best_column) = (0..column_count).max_by_key(|column| {
            uncovered
                .iter()
                .filter(|row| matrix[**row].contains(column))
                .count()
        }) else
        {
            break;
        };

        cover.push(best_column);
        uncovered.retain(|row| !matrix[*row].contains(&best_column));
    }

    cover.sort_unstable();
    cover.dedup();
    cover
}

fn search_cover(matrix: &[BTreeSet<usize>], partial: &mut BTreeSet<usize>, best: &mut Vec<usize>)
{
    if partial.len() >= best.len()
    {
        return;
    }

    let Some(row) = first_uncovered_row(matrix, partial) else
    {
        *best = partial.iter().copied().collect();
        return;
    };

    for column in &matrix[row]
    {
        partial.insert(*column);
        search_cover(matrix, partial, best);
        partial.remove(column);
    }
}

fn first_uncovered_row(matrix: &[BTreeSet<usize>], partial: &BTreeSet<usize>) -> Option<usize>
{
    matrix
        .iter()
        .enumerate()
        .find(|(_, columns)| columns.is_disjoint(partial))
        .map(|(row, _)| row)
}

fn derive_codes(primes: &[Dichotomy], symbol_count: usize) -> Vec<String>
{
    let mut codes = vec![String::with_capacity(primes.len()); symbol_count];

    for prime in primes
    {
        for (symbol, code) in codes.iter_mut().enumerate()
        {
            if prime.rhs().contains(&symbol)
            {
                code.push('1');
            }
            else
            {
                code.push('0');
            }
        }
    }

    codes
}

fn add_irredundant(items: &mut Vec<Dichotomy>, item: Dichotomy)
{
    for existing in items.iter_mut()
    {
        if item.implies(existing)
        {
            return;
        }

        if existing.implies(&item)
        {
            *existing = item;
            return;
        }
    }

    items.push(item);
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn parses_symbolic_constraints()
    {
        let constraint = SymbolicConstraint::parse("10-").unwrap();

        assert_eq!(
            constraint.values(),
            &[
                ConstraintValue::One,
                ConstraintValue::Zero,
                ConstraintValue::DontCare
            ]
        );
    }

    #[test]
    fn rejects_inconsistent_constraint_widths()
    {
        let error = encode_constraint_patterns(["10", "100"]).unwrap_err();

        assert_eq!(
            error,
            EncodingError::InconsistentSymbolCount
            {
                expected: 2,
                actual: 3,
            }
        );
    }

    #[test]
    fn returns_empty_encoding_when_constraints_create_no_seeds()
    {
        let result = encode_constraint_patterns(["11-", "-1-"]).unwrap();

        assert_eq!(result.code_length, 0);
        assert_eq!(result.codes, vec!["", "", ""]);
        assert!(result.selected_primes.is_empty());
    }

    #[test]
    fn encodes_simple_partition_constraint()
    {
        let result = encode_constraint_patterns(["100"]).unwrap();

        assert_eq!(result.code_length, 2);
        assert_ne!(result.codes[0], result.codes[1]);
        assert_ne!(result.codes[0], result.codes[2]);
        assert_ne!(result.codes[1], result.codes[2]);
    }

    #[test]
    fn encodes_pairwise_distinct_constraints()
    {
        let result = encode_constraint_patterns(["100", "010", "001"]).unwrap();

        assert_eq!(result.code_length, 2);
        assert_ne!(result.codes[0], result.codes[1]);
        assert_ne!(result.codes[0], result.codes[2]);
        assert_ne!(result.codes[1], result.codes[2]);
    }

    #[test]
    fn reports_dominance_cycles_before_encoding()
    {
        let options = EncodingOptions
        {
            dominance_edges: vec![(0, 1), (1, 0)],
            prime_limit: PRIME_LIMIT,
        };

        let error = encode_constraint_patterns_with_options(["10"], &options).unwrap_err();

        assert_eq!(error, EncodingError::CyclicDominance { index: 0 });
    }

    #[test]
    fn cover_matrix_reports_uncovered_seed()
    {
        let seed = Dichotomy::new([0], [1]);
        let prime = Dichotomy::new([2], [3]);

        let (_matrix, uncovered) = build_cover_matrix(&[prime], &[seed.clone()]);

        assert_eq!(uncovered, vec![seed]);
    }

    #[test]
    fn minimum_cover_selects_shared_prime()
    {
        let matrix = vec![
            BTreeSet::from([0, 2]),
            BTreeSet::from([1, 2]),
            BTreeSet::from([2, 3]),
        ];

        assert_eq!(minimum_cover(&matrix, 4), vec![2]);
    }
}
