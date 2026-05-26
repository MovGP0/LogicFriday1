//! Native Rust port of `sis/mincov/gimpel.c`.
//!
//! Gimpel reduction recognizes a two-column primary row where one column has
//! exactly one other row. The reduction removes that small choice, solves the
//! reduced cover problem, then lifts the solution by selecting one of the two
//! removed columns.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CoverMatrix
{
    rows: BTreeMap<usize, BTreeSet<usize>>,
    cols: BTreeMap<usize, BTreeSet<usize>>,
}

impl CoverMatrix
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn from_rows(rows: impl IntoIterator<Item = (usize, impl IntoIterator<Item = usize>)>) -> Self
    {
        let mut matrix = Self::new();
        for (row, cols) in rows
        {
            for col in cols
            {
                matrix.insert(row, col);
            }
        }

        matrix
    }

    pub fn insert(&mut self, row: usize, col: usize) -> bool
    {
        let inserted = self.rows.entry(row).or_default().insert(col);
        if inserted
        {
            self.cols.entry(col).or_default().insert(row);
        }

        inserted
    }

    pub fn row(&self, row: usize) -> Option<&BTreeSet<usize>>
    {
        self.rows.get(&row)
    }

    pub fn col(&self, col: usize) -> Option<&BTreeSet<usize>>
    {
        self.cols.get(&col)
    }

    pub fn rows(&self) -> impl Iterator<Item = (usize, &BTreeSet<usize>)> + '_
    {
        self.rows.iter().map(|(row, cols)| (*row, cols))
    }

    pub fn contains(&self, row: usize, col: usize) -> bool
    {
        self.rows.get(&row).is_some_and(|cols| cols.contains(&col))
    }

    pub fn delete_row(&mut self, row: usize) -> bool
    {
        let Some(cols) = self.rows.remove(&row) else
        {
            return false;
        };

        for col in cols
        {
            self.remove_row_from_col(row, col);
        }

        true
    }

    pub fn delete_col(&mut self, col: usize) -> bool
    {
        let Some(rows) = self.cols.remove(&col) else
        {
            return false;
        };

        for row in rows
        {
            self.remove_col_from_row(row, col);
        }

        true
    }

    pub fn row_count(&self) -> usize
    {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize
    {
        self.cols.len()
    }

    fn remove_col_from_row(&mut self, row: usize, col: usize)
    {
        let should_remove = if let Some(cols) = self.rows.get_mut(&row)
        {
            cols.remove(&col);
            cols.is_empty()
        }
        else
        {
            false
        };

        if should_remove
        {
            self.rows.remove(&row);
        }
    }

    fn remove_row_from_col(&mut self, row: usize, col: usize)
    {
        let should_remove = if let Some(rows) = self.cols.get_mut(&col)
        {
            rows.remove(&row);
            rows.is_empty()
        }
        else
        {
            false
        };

        if should_remove
        {
            self.cols.remove(&col);
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CoverSolution
{
    columns: BTreeSet<usize>,
    pub cost: i32,
}

impl CoverSolution
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn from_columns(columns: impl IntoIterator<Item = usize>, weights: Option<&[i32]>) -> Self
    {
        let mut solution = Self::new();
        for col in columns
        {
            solution.add(weights, col);
        }

        solution
    }

    pub fn add(&mut self, weights: Option<&[i32]>, col: usize) -> bool
    {
        let inserted = self.columns.insert(col);
        if inserted
        {
            self.cost += weight(weights, col);
        }

        inserted
    }

    pub fn contains(&self, col: usize) -> bool
    {
        self.columns.contains(&col)
    }

    pub fn intersects(&self, row: &BTreeSet<usize>) -> bool
    {
        self.columns.iter().any(|col| row.contains(col))
    }

    pub fn columns(&self) -> &BTreeSet<usize>
    {
        &self.columns
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GimpelStats
{
    pub gimpel_count: usize,
    pub gimpel: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GimpelApplication
{
    pub primary_row: usize,
    pub secondary_row: usize,
    pub degree_two_col: usize,
    pub other_col: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GimpelReduction
{
    NotApplicable,
    Applied {
        application: GimpelApplication,
        best: Option<CoverSolution>,
    },
}

pub fn gimpel_reduce<F>(
    matrix: &mut CoverMatrix,
    select: &CoverSolution,
    weights: Option<&[i32]>,
    lb: i32,
    bound: i32,
    depth: usize,
    stats: &mut GimpelStats,
    solve_reduced: F,
) -> GimpelReduction
where
    F: FnOnce(
        &mut CoverMatrix,
        &CoverSolution,
        Option<&[i32]>,
        i32,
        i32,
        usize,
        &mut GimpelStats,
    ) -> Option<CoverSolution>,
{
    let Some(application) = find_gimpel_application(matrix) else
    {
        return GimpelReduction::NotApplicable;
    };

    let save_secondary = matrix
        .row(application.secondary_row)
        .map(|row| row.iter().copied().collect::<BTreeSet<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|col| *col != application.degree_two_col)
        .collect::<BTreeSet<_>>();

    let other_col_rows = matrix
        .col(application.other_col)
        .map(|rows| rows.iter().copied().collect::<Vec<_>>())
        .unwrap_or_default();

    for row in other_col_rows
    {
        if row != application.primary_row
        {
            for col in &save_secondary
            {
                matrix.insert(row, *col);
            }
        }
    }

    matrix.delete_col(application.degree_two_col);
    matrix.delete_col(application.other_col);
    matrix.delete_row(application.primary_row);
    matrix.delete_row(application.secondary_row);

    stats.gimpel_count += 1;
    stats.gimpel += 1;
    let mut best = solve_reduced(matrix, select, weights, lb - 1, bound - 1, depth, stats);
    stats.gimpel -= 1;

    if let Some(solution) = &mut best
    {
        if solution.intersects(&save_secondary)
        {
            solution.add(weights, application.other_col);
        }
        else
        {
            solution.add(weights, application.degree_two_col);
        }
    }

    GimpelReduction::Applied { application, best }
}

fn find_gimpel_application(matrix: &CoverMatrix) -> Option<GimpelApplication>
{
    for (primary_row, row_cols) in matrix.rows()
    {
        if row_cols.len() != 2
        {
            continue;
        }

        let mut cols = row_cols.iter().copied();
        let first_col = cols.next().expect("two-column row has a first column");
        let second_col = cols.next().expect("two-column row has a second column");

        if matrix.col(first_col).is_some_and(|col| col.len() == 2)
        {
            return build_application(matrix, primary_row, first_col, second_col);
        }

        if matrix.col(second_col).is_some_and(|col| col.len() == 2)
        {
            return build_application(matrix, primary_row, second_col, first_col);
        }
    }

    None
}

fn build_application(
    matrix: &CoverMatrix,
    primary_row: usize,
    degree_two_col: usize,
    other_col: usize,
) -> Option<GimpelApplication>
{
    let secondary_row = matrix
        .col(degree_two_col)?
        .iter()
        .copied()
        .find(|row| *row != primary_row)?;

    Some(GimpelApplication {
        primary_row,
        secondary_row,
        degree_two_col,
        other_col,
    })
}

fn weight(weights: Option<&[i32]>, col: usize) -> i32
{
    weights.and_then(|values| values.get(col)).copied().unwrap_or(1)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn returns_not_applicable_without_two_column_degree_two_pattern()
    {
        let mut matrix = CoverMatrix::from_rows([
            (0, vec![1, 2]),
            (1, vec![1, 3]),
            (2, vec![2, 4]),
            (3, vec![1, 5]),
            (4, vec![2, 6]),
        ]);
        let select = CoverSolution::new();
        let mut stats = GimpelStats::default();

        let reduction = gimpel_reduce(
            &mut matrix,
            &select,
            None,
            0,
            10,
            2,
            &mut stats,
            |_, _, _, _, _, _, _| panic!("reduced solver should not run"),
        );

        assert_eq!(reduction, GimpelReduction::NotApplicable);
        assert_eq!(stats, GimpelStats::default());
        assert_eq!(matrix.row_count(), 5);
    }

    #[test]
    fn merges_secondary_columns_deletes_pattern_and_lifts_other_column()
    {
        let mut matrix = CoverMatrix::from_rows([
            (0, vec![1, 2]),
            (1, vec![1, 3, 4]),
            (2, vec![2, 5]),
            (3, vec![2, 6]),
            (4, vec![7]),
        ]);
        let select = CoverSolution::new();
        let mut stats = GimpelStats::default();

        let reduction = gimpel_reduce(
            &mut matrix,
            &select,
            None,
            3,
            12,
            5,
            &mut stats,
            |reduced, _, _, lb, bound, depth, stats| {
                assert_eq!(lb, 2);
                assert_eq!(bound, 11);
                assert_eq!(depth, 5);
                assert_eq!(stats.gimpel, 1);
                assert_eq!(reduced.row(2).unwrap(), &set(&[3, 4, 5]));
                assert_eq!(reduced.row(3).unwrap(), &set(&[3, 4, 6]));
                assert_eq!(reduced.row(4).unwrap(), &set(&[7]));
                assert!(reduced.col(1).is_none());
                assert!(reduced.col(2).is_none());

                Some(CoverSolution::from_columns([3, 7], None))
            },
        );

        let GimpelReduction::Applied { application, best } = reduction else
        {
            panic!("expected Gimpel reduction to apply");
        };

        assert_eq!(
            application,
            GimpelApplication {
                primary_row: 0,
                secondary_row: 1,
                degree_two_col: 1,
                other_col: 2,
            }
        );
        let best = best.expect("reduced solver should produce a solution");
        assert!(best.contains(2));
        assert!(!best.contains(1));
        assert_eq!(best.cost, 3);
        assert_eq!(stats.gimpel_count, 1);
        assert_eq!(stats.gimpel, 0);
    }

    #[test]
    fn lifts_degree_two_column_when_secondary_row_is_uncovered()
    {
        let mut matrix = CoverMatrix::from_rows([(0, vec![1, 2]), (1, vec![1, 3]), (2, vec![2, 4])]);
        let select = CoverSolution::new();
        let weights = [1, 9, 7, 1, 1];
        let mut stats = GimpelStats::default();

        let reduction = gimpel_reduce(
            &mut matrix,
            &select,
            Some(&weights),
            0,
            20,
            0,
            &mut stats,
            |_, _, weights, _, _, _, _| Some(CoverSolution::from_columns([4], weights)),
        );

        let GimpelReduction::Applied { best, .. } = reduction else
        {
            panic!("expected Gimpel reduction to apply");
        };

        let best = best.expect("reduced solver should produce a solution");
        assert!(best.contains(1));
        assert!(!best.contains(2));
        assert_eq!(best.cost, 10);
    }

    #[test]
    fn flips_columns_when_second_primary_column_has_degree_two()
    {
        let mut matrix = CoverMatrix::from_rows([
            (0, vec![1, 2]),
            (1, vec![1, 3]),
            (2, vec![1, 4]),
            (3, vec![2, 5]),
        ]);
        let select = CoverSolution::new();
        let mut stats = GimpelStats::default();

        let reduction = gimpel_reduce(
            &mut matrix,
            &select,
            None,
            0,
            10,
            0,
            &mut stats,
            |_, _, _, _, _, _, _| None,
        );

        assert_eq!(
            reduction,
            GimpelReduction::Applied {
                application: GimpelApplication {
                    primary_row: 0,
                    secondary_row: 3,
                    degree_two_col: 2,
                    other_col: 1,
                },
                best: None,
            }
        );
    }

    #[test]
    fn leaves_lift_step_unapplied_when_reduced_solver_fails()
    {
        let mut matrix = CoverMatrix::from_rows([(0, vec![1, 2]), (1, vec![1, 3]), (2, vec![2, 4])]);
        let select = CoverSolution::new();
        let mut stats = GimpelStats::default();

        let reduction = gimpel_reduce(
            &mut matrix,
            &select,
            None,
            0,
            10,
            0,
            &mut stats,
            |_, _, _, _, _, _, _| None,
        );

        let GimpelReduction::Applied { best, .. } = reduction else
        {
            panic!("expected Gimpel reduction to apply");
        };

        assert!(best.is_none());
        assert_eq!(stats.gimpel_count, 1);
        assert_eq!(stats.gimpel, 0);
    }

    fn set(values: &[usize]) -> BTreeSet<usize>
    {
        values.iter().copied().collect()
    }
}
