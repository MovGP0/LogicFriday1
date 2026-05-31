//! Native Rust port of `LogicSynthesis/sis/mincov/bin_sol.c`.
//!
//! The original C module owns a packed Espresso bitset and keeps the running
//! weighted cost for branch-and-bound binate covering. This port keeps that
//! behavior as a safe, bounded Rust value type. Repeated add/delete calls still
//! adjust the cost, matching the C `set_insert`/`set_remove` macros that do not
//! test whether the bit changed.

use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinSolution
{
    cost: i32,
    selected: Vec<bool>,
}

impl BinSolution
{
    pub fn new(size: usize) -> Self
    {
        Self {
            cost: 0,
            selected: vec![false; size],
        }
    }

    pub fn duplicate(&self, size: usize) -> Result<Self, BinSolutionError>
    {
        if size < self.selected.len()
        {
            return Err(BinSolutionError::DuplicateSizeTooSmall {
                size,
                required: self.selected.len(),
            });
        }

        let mut selected = vec![false; size];
        selected[..self.selected.len()].copy_from_slice(&self.selected);

        Ok(Self {
            cost: self.cost,
            selected,
        })
    }

    pub fn capacity(&self) -> usize
    {
        self.selected.len()
    }

    pub fn cost(&self) -> i32
    {
        self.cost
    }

    pub fn contains(&self, col: usize) -> bool
    {
        self.selected.get(col).copied().unwrap_or(false)
    }

    pub fn columns(&self) -> impl Iterator<Item = usize> + '_
    {
        self.selected
            .iter()
            .enumerate()
            .filter_map(|(column, selected)| selected.then_some(column))
    }

    pub fn add(&mut self, weights: Option<&[i32]>, col: usize) -> Result<(), BinSolutionError>
    {
        let weight = self.weight_for(weights, col)?;

        self.selected[col] = true;
        self.cost += weight;

        Ok(())
    }

    pub fn delete(&mut self, weights: Option<&[i32]>, col: usize) -> Result<(), BinSolutionError>
    {
        let weight = self.weight_for(weights, col)?;

        self.selected[col] = false;
        self.cost -= weight;

        Ok(())
    }

    fn weight_for(&self, weights: Option<&[i32]>, col: usize) -> Result<i32, BinSolutionError>
    {
        if col >= self.selected.len()
        {
            return Err(BinSolutionError::ColumnOutOfRange {
                col,
                size: self.selected.len(),
            });
        }

        match weights
        {
            Some(weights) => weights.get(col).copied().ok_or(BinSolutionError::WeightOutOfRange {
                col,
                len: weights.len(),
            }),
            None => Ok(1),
        }
    }
}

impl Default for BinSolution
{
    fn default() -> Self
    {
        Self::new(0)
    }
}

pub fn bin_solution_alloc(size: usize) -> BinSolution
{
    BinSolution::new(size)
}

pub fn bin_solution_dup(sol: &BinSolution, size: usize) -> Result<BinSolution, BinSolutionError>
{
    sol.duplicate(size)
}

pub fn bin_solution_add(
    sol: &mut BinSolution,
    weights: Option<&[i32]>,
    col: usize,
) -> Result<(), BinSolutionError>
{
    sol.add(weights, col)
}

pub fn bin_solution_del(
    sol: &mut BinSolution,
    weights: Option<&[i32]>,
    col: usize,
) -> Result<(), BinSolutionError>
{
    sol.delete(weights, col)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinSolutionError
{
    ColumnOutOfRange
    {
        col: usize,
        size: usize,
    },
    DuplicateSizeTooSmall
    {
        size: usize,
        required: usize,
    },
    WeightOutOfRange
    {
        col: usize,
        len: usize,
    },
}

impl fmt::Display for BinSolutionError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::ColumnOutOfRange { col, size } => {
                write!(f, "column {col} is outside bin-solution size {size}")
            }
            Self::DuplicateSizeTooSmall { size, required } => {
                write!(
                    f,
                    "duplicate size {size} is smaller than source bin-solution size {required}"
                )
            }
            Self::WeightOutOfRange { col, len } => {
                write!(f, "column {col} has no weight in weight array of length {len}")
            }
        }
    }
}

impl Error for BinSolutionError {}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn alloc_starts_with_zero_cost_and_empty_set()
    {
        let solution = bin_solution_alloc(4);

        assert_eq!(solution.cost(), 0);
        assert_eq!(solution.capacity(), 4);
        assert_eq!(solution.columns().collect::<Vec<_>>(), Vec::<usize>::new());
    }

    #[test]
    fn add_and_delete_use_unit_weight_when_weight_array_is_absent()
    {
        let mut solution = BinSolution::new(5);

        bin_solution_add(&mut solution, None, 2).unwrap();
        bin_solution_add(&mut solution, None, 4).unwrap();
        bin_solution_del(&mut solution, None, 2).unwrap();

        assert!(!solution.contains(2));
        assert!(solution.contains(4));
        assert_eq!(solution.cost(), 1);
        assert_eq!(solution.columns().collect::<Vec<_>>(), vec![4]);
    }

    #[test]
    fn add_and_delete_use_column_weights()
    {
        let weights = [4, 7, 11, 13];
        let mut solution = BinSolution::new(weights.len());

        solution.add(Some(&weights), 1).unwrap();
        solution.add(Some(&weights), 3).unwrap();
        solution.delete(Some(&weights), 1).unwrap();

        assert_eq!(solution.cost(), 13);
        assert_eq!(solution.columns().collect::<Vec<_>>(), vec![3]);
    }

    #[test]
    fn repeated_add_and_delete_keep_c_cost_semantics()
    {
        let weights = [2, 5, 9];
        let mut solution = BinSolution::new(weights.len());

        solution.add(Some(&weights), 1).unwrap();
        solution.add(Some(&weights), 1).unwrap();
        assert_eq!(solution.cost(), 10);
        assert_eq!(solution.columns().collect::<Vec<_>>(), vec![1]);

        solution.delete(Some(&weights), 1).unwrap();
        solution.delete(Some(&weights), 1).unwrap();
        assert_eq!(solution.cost(), 0);
        assert_eq!(solution.columns().collect::<Vec<_>>(), Vec::<usize>::new());
    }

    #[test]
    fn duplicate_copies_cost_and_set_into_requested_size()
    {
        let weights = [3, 5, 7];
        let mut solution = BinSolution::new(weights.len());
        solution.add(Some(&weights), 0).unwrap();
        solution.add(Some(&weights), 2).unwrap();

        let duplicate = bin_solution_dup(&solution, 5).unwrap();

        assert_eq!(duplicate.cost(), 10);
        assert_eq!(duplicate.capacity(), 5);
        assert_eq!(duplicate.columns().collect::<Vec<_>>(), vec![0, 2]);
    }

    #[test]
    fn duplicate_rejects_smaller_destination_size()
    {
        let solution = BinSolution::new(4);

        assert_eq!(
            solution.duplicate(3),
            Err(BinSolutionError::DuplicateSizeTooSmall {
                size: 3,
                required: 4,
            })
        );
    }

    #[test]
    fn invalid_columns_and_weights_are_reported_without_mutation()
    {
        let mut solution = BinSolution::new(3);

        assert_eq!(
            solution.add(None, 3),
            Err(BinSolutionError::ColumnOutOfRange { col: 3, size: 3 })
        );
        assert_eq!(
            solution.add(Some(&[10]), 2),
            Err(BinSolutionError::WeightOutOfRange { col: 2, len: 1 })
        );
        assert_eq!(solution.cost(), 0);
        assert_eq!(solution.columns().collect::<Vec<_>>(), Vec::<usize>::new());
    }
}
