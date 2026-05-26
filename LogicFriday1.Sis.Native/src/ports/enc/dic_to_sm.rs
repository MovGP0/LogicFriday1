//! Native Rust port of `LogicSynthesis/sis/enc/dic_to_sm.c`.
//!
//! The C routine builds a sparse covering matrix with seed dichotomies as rows
//! and prime dichotomies as columns. This port keeps the same coordinate
//! behavior on owned Rust values and stores the prime attached to each inserted
//! column so selected cover columns can be formatted without raw pointer
//! payloads.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::ports::sparse::matrix::SparseMatrix;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dichotomy
{
    element_count: usize,
    lhs: BTreeSet<usize>,
    rhs: BTreeSet<usize>,
}

impl Dichotomy
{
    pub fn new(element_count: usize) -> Self
    {
        Self {
            element_count,
            lhs: BTreeSet::new(),
            rhs: BTreeSet::new(),
        }
    }

    pub fn from_sides<L, R>(element_count: usize, lhs: L, rhs: R) -> DicToSmResult<Self>
    where
        L: IntoIterator<Item = usize>,
        R: IntoIterator<Item = usize>,
    {
        let mut dichotomy = Self::new(element_count);
        for element in lhs
        {
            dichotomy.insert_lhs(element)?;
        }

        for element in rhs
        {
            dichotomy.insert_rhs(element)?;
        }

        Ok(dichotomy)
    }

    pub fn element_count(&self) -> usize
    {
        self.element_count
    }

    pub fn lhs(&self) -> &BTreeSet<usize>
    {
        &self.lhs
    }

    pub fn rhs(&self) -> &BTreeSet<usize>
    {
        &self.rhs
    }

    pub fn insert_lhs(&mut self, element: usize) -> DicToSmResult<bool>
    {
        self.validate_element(element)?;
        Ok(self.lhs.insert(element))
    }

    pub fn insert_rhs(&mut self, element: usize) -> DicToSmResult<bool>
    {
        self.validate_element(element)?;
        Ok(self.rhs.insert(element))
    }

    pub fn covers(&self, prime: &Dichotomy) -> bool
    {
        self.same_size_as(prime)
            && ((self.lhs.is_subset(&prime.lhs) && self.rhs.is_subset(&prime.rhs))
                || (self.lhs.is_subset(&prime.rhs) && self.rhs.is_subset(&prime.lhs)))
    }

    pub fn format_sides(&self) -> String
    {
        format!(
            "{} ; {}",
            format_bit_vector(&self.lhs, self.element_count),
            format_bit_vector(&self.rhs, self.element_count)
        )
    }

    fn same_size_as(&self, other: &Dichotomy) -> bool
    {
        self.element_count == other.element_count
    }

    fn validate_element(&self, element: usize) -> DicToSmResult<()>
    {
        if element >= self.element_count
        {
            return Err(DicToSmError::ElementOutOfRange {
                element,
                element_count: self.element_count,
            });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DichotomyFamily
{
    element_count: usize,
    dichotomies: Vec<Dichotomy>,
}

impl DichotomyFamily
{
    pub fn new(element_count: usize) -> Self
    {
        Self {
            element_count,
            dichotomies: Vec::new(),
        }
    }

    pub fn from_dichotomies<I>(element_count: usize, dichotomies: I) -> DicToSmResult<Self>
    where
        I: IntoIterator<Item = Dichotomy>,
    {
        let mut family = Self::new(element_count);
        for dichotomy in dichotomies
        {
            family.push(dichotomy)?;
        }

        Ok(family)
    }

    pub fn element_count(&self) -> usize
    {
        self.element_count
    }

    pub fn dichotomies(&self) -> &[Dichotomy]
    {
        &self.dichotomies
    }

    pub fn push(&mut self, dichotomy: Dichotomy) -> DicToSmResult<()>
    {
        if dichotomy.element_count() != self.element_count
        {
            return Err(DicToSmError::DichotomySizeMismatch {
                expected: self.element_count,
                actual: dichotomy.element_count(),
            });
        }

        self.dichotomies.push(dichotomy);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DichotomyCoverMatrix
{
    matrix: SparseMatrix,
    column_primes: BTreeMap<usize, Dichotomy>,
}

impl DichotomyCoverMatrix
{
    pub fn matrix(&self) -> &SparseMatrix
    {
        &self.matrix
    }

    pub fn column_prime(&self, column: usize) -> Option<&Dichotomy>
    {
        self.column_primes.get(&column)
    }

    pub fn column_primes(&self) -> &BTreeMap<usize, Dichotomy>
    {
        &self.column_primes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DicToSmError
{
    ElementOutOfRange { element: usize, element_count: usize },
    DichotomySizeMismatch { expected: usize, actual: usize },
    FamilySizeMismatch { primes: usize, seeds: usize },
    MissingColumnPrime { column: usize },
}

impl fmt::Display for DicToSmError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::ElementOutOfRange {
                element,
                element_count,
            } => write!(
                f,
                "dichotomy element {element} is outside element count {element_count}"
            ),
            Self::DichotomySizeMismatch { expected, actual } => write!(
                f,
                "dichotomy has element count {actual}; expected {expected}"
            ),
            Self::FamilySizeMismatch { primes, seeds } => write!(
                f,
                "prime family element count {primes} differs from seed family element count {seeds}"
            ),
            Self::MissingColumnPrime { column } => {
                write!(f, "cover references sparse column {column} without a stored prime")
            }
        }
    }
}

impl Error for DicToSmError {}

pub type DicToSmResult<T> = Result<T, DicToSmError>;

pub fn dic_to_sm(
    prime_list: &DichotomyFamily,
    seed_list: &DichotomyFamily,
) -> DicToSmResult<DichotomyCoverMatrix>
{
    if prime_list.element_count() != seed_list.element_count()
    {
        return Err(DicToSmError::FamilySizeMismatch {
            primes: prime_list.element_count(),
            seeds: seed_list.element_count(),
        });
    }

    let mut matrix = SparseMatrix::new();
    let mut column_primes = BTreeMap::new();

    for (column, prime) in prime_list.dichotomies().iter().enumerate()
    {
        let mut inserted = false;
        for (row, seed) in seed_list.dichotomies().iter().enumerate()
        {
            if seed.covers(prime)
            {
                matrix.insert(row, column);
                inserted = true;
            }
        }

        if inserted
        {
            column_primes.insert(column, prime.clone());
        }
    }

    Ok(DichotomyCoverMatrix {
        matrix,
        column_primes,
    })
}

pub fn print_min_cover<I>(matrix: &DichotomyCoverMatrix, cover: I) -> DicToSmResult<String>
where
    I: IntoIterator<Item = usize>,
{
    let mut output = String::from("\nMinimum cover is \n");
    for column in cover
    {
        let prime = matrix
            .column_prime(column)
            .ok_or(DicToSmError::MissingColumnPrime { column })?;
        output.push_str(&prime.format_sides());
        output.push('\n');
    }

    Ok(output)
}

fn format_bit_vector(set: &BTreeSet<usize>, element_count: usize) -> String
{
    (0..element_count)
        .map(|element| if set.contains(&element) { '1' } else { '0' })
        .collect()
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn dic(lhs: &[usize], rhs: &[usize]) -> Dichotomy
    {
        Dichotomy::from_sides(4, lhs.iter().copied(), rhs.iter().copied()).unwrap()
    }

    fn family(rows: &[Dichotomy]) -> DichotomyFamily
    {
        DichotomyFamily::from_dichotomies(4, rows.iter().cloned()).unwrap()
    }

    #[test]
    fn builds_sparse_matrix_with_seed_rows_and_prime_columns()
    {
        let primes = family(&[
            dic(&[0, 1], &[2]),
            dic(&[0], &[3]),
            dic(&[1], &[3]),
        ]);
        let seeds = family(&[
            dic(&[0], &[2]),
            dic(&[3], &[0]),
            dic(&[2], &[0]),
        ]);

        let result = dic_to_sm(&primes, &seeds).unwrap();
        let matrix = result.matrix();

        assert!(matrix.contains(0, 0));
        assert!(matrix.contains(1, 1));
        assert!(matrix.contains(2, 0));
        assert!(!matrix.contains(0, 1));
        assert!(!matrix.contains(1, 2));
        assert_eq!(matrix.element_count(), 3);
        assert_eq!(result.column_primes().keys().copied().collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn covers_allows_reversed_dichotomy_orientation()
    {
        let seed = dic(&[2], &[0]);
        let prime = dic(&[0, 1], &[2]);

        assert!(seed.covers(&prime));
    }

    #[test]
    fn uncovered_prime_does_not_get_column_payload()
    {
        let primes = family(&[dic(&[0], &[1]), dic(&[2], &[3])]);
        let seeds = family(&[dic(&[0], &[1])]);

        let result = dic_to_sm(&primes, &seeds).unwrap();

        assert!(result.matrix().contains(0, 0));
        assert_eq!(result.column_prime(0), Some(&dic(&[0], &[1])));
        assert_eq!(result.column_prime(1), None);
    }

    #[test]
    fn prints_selected_cover_columns_like_c_min_cover_output()
    {
        let primes = family(&[dic(&[0], &[1]), dic(&[2], &[3])]);
        let seeds = family(&[dic(&[0], &[1]), dic(&[2], &[3])]);
        let matrix = dic_to_sm(&primes, &seeds).unwrap();

        let output = print_min_cover(&matrix, [1, 0]).unwrap();

        assert_eq!(output, "\nMinimum cover is \n0010 ; 0001\n1000 ; 0100\n");
    }

    #[test]
    fn reports_size_mismatch_between_prime_and_seed_families()
    {
        let primes = DichotomyFamily::from_dichotomies(
            3,
            [Dichotomy::from_sides(3, [0], [1]).unwrap()],
        )
        .unwrap();
        let seeds = DichotomyFamily::from_dichotomies(
            4,
            [Dichotomy::from_sides(4, [0], [1]).unwrap()],
        )
        .unwrap();

        assert_eq!(
            dic_to_sm(&primes, &seeds).unwrap_err(),
            DicToSmError::FamilySizeMismatch {
                primes: 3,
                seeds: 4
            }
        );
    }

    #[test]
    fn rejects_out_of_range_elements()
    {
        assert_eq!(
            Dichotomy::from_sides(2, [0, 2], [1]).unwrap_err(),
            DicToSmError::ElementOutOfRange {
                element: 2,
                element_count: 2
            }
        );
    }

    #[test]
    fn rejects_cover_columns_without_prime_payload()
    {
        let matrix = DichotomyCoverMatrix {
            matrix: SparseMatrix::new(),
            column_primes: BTreeMap::new(),
        };

        assert_eq!(
            print_min_cover(&matrix, [7]).unwrap_err(),
            DicToSmError::MissingColumnPrime { column: 7 }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("dic_to_sm.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
