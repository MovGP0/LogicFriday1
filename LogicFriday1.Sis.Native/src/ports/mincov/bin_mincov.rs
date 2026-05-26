//! Native Rust port of `sis/mincov/bin_mincov.c`.
//!
//! The original routine solves unate and binate sparse covering matrices with a
//! recursive branch-and-bound search. This port keeps that behavior in safe,
//! owned Rust data structures: rows are sums of column literals, selected
//! columns form the product term, and binate mode rejects complementary
//! literal pairs.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MincovMatrix
{
    rows: BTreeMap<usize, BTreeSet<usize>>,
}

impl MincovMatrix
{
    pub fn new() -> Self
    {
        Self {
            rows: BTreeMap::new(),
        }
    }

    pub fn from_rows(rows: impl IntoIterator<Item = impl IntoIterator<Item = usize>>) -> Self
    {
        let mut matrix = Self::new();
        for (row, columns) in rows.into_iter().enumerate()
        {
            for col in columns
            {
                matrix.insert(row, col);
            }
        }
        matrix
    }

    pub fn insert(&mut self, row: usize, col: usize) -> bool
    {
        self.rows.entry(row).or_default().insert(col)
    }

    pub fn row(&self, row: usize) -> Option<&BTreeSet<usize>>
    {
        self.rows.get(&row)
    }

    pub fn rows(&self) -> &BTreeMap<usize, BTreeSet<usize>>
    {
        &self.rows
    }

    pub fn row_count(&self) -> usize
    {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize
    {
        self.columns().len()
    }

    pub fn columns(&self) -> BTreeSet<usize>
    {
        self.rows
            .values()
            .flat_map(|columns| columns.iter().copied())
            .collect()
    }

    pub fn max_col(&self) -> Option<usize>
    {
        self.rows
            .values()
            .filter_map(|columns| columns.iter().next_back().copied())
            .max()
    }

    pub fn covered_by(&self, cover: &BTreeSet<usize>) -> bool
    {
        self.rows
            .values()
            .all(|row| row.iter().any(|column| cover.contains(column)))
    }
}

impl Default for MincovMatrix
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MincovResult
{
    pub cover: BTreeSet<usize>,
    pub cost: i32,
    pub calls: usize,
    pub leaves: usize,
    pub rule1_prunes: usize,
    pub rule3_skips: usize,
    pub rule5_prunes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MincovError
{
    EmptyRow { row: usize },
    NoCoverWithinBound,
    CoverVerificationFailed,
}

impl fmt::Display for MincovError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyRow { row } => write!(f, "mincov row {row} has no covering columns"),
            Self::NoCoverWithinBound => write!(f, "no mincov cover was found within the bound"),
            Self::CoverVerificationFailed => write!(f, "mincov cover verification failed"),
        }
    }
}

impl Error for MincovError {}

#[derive(Clone, Debug)]
struct SearchState
{
    is_unate: bool,
    heuristic: bool,
    weights: Vec<i32>,
    rows: Vec<Vec<usize>>,
    order: Vec<Vec<usize>>,
    solution: BTreeSet<usize>,
    solution_cost: i32,
    best_cover: BTreeSet<usize>,
    best_cost: i32,
    ended: bool,
    calls: usize,
    leaves: usize,
    rule1_prunes: usize,
    rule3_skips: usize,
    rule5_prunes: usize,
}

pub fn sm_mat_bin_minimum_cover(
    matrix: &MincovMatrix,
    weights: Option<&[i32]>,
    heuristic: bool,
    _debug: i32,
    upper_bound: Option<i32>,
    option: i32,
) -> Result<MincovResult, MincovError>
{
    minimum_cover(matrix, weights, heuristic, upper_bound, option, false)
}

pub fn sm_mat_minimum_cover(
    matrix: &MincovMatrix,
    weights: Option<&[i32]>,
    heuristic: bool,
    _debug: i32,
    upper_bound: Option<i32>,
    option: i32,
) -> Result<MincovResult, MincovError>
{
    minimum_cover(matrix, weights, heuristic, upper_bound, option, true)
}

fn minimum_cover(
    matrix: &MincovMatrix,
    weights: Option<&[i32]>,
    heuristic: bool,
    upper_bound: Option<i32>,
    option: i32,
    is_unate: bool,
) -> Result<MincovResult, MincovError>
{
    if matrix.row_count() == 0
    {
        return Ok(MincovResult {
            cover: BTreeSet::new(),
            cost: 0,
            calls: 0,
            leaves: 0,
            rule1_prunes: 0,
            rule3_skips: 0,
            rule5_prunes: 0,
        });
    }

    for (row, columns) in matrix.rows()
    {
        if columns.is_empty()
        {
            return Err(MincovError::EmptyRow { row: *row });
        }
    }

    let max_col = matrix.max_col().unwrap_or(0);
    let expanded_weights = expand_weights(weights, max_col);
    let rows = reorder_rows(matrix, &expanded_weights, option);
    let order = order_columns(&rows, &expanded_weights, option, is_unate);
    let initial_bound = upper_bound
        .filter(|bound| *bound > 0)
        .unwrap_or_else(|| matrix.columns().iter().map(|col| weight(&expanded_weights, *col)).sum());

    let mut state = SearchState {
        is_unate,
        heuristic,
        weights: expanded_weights,
        rows,
        order,
        solution: BTreeSet::new(),
        solution_cost: 0,
        best_cover: BTreeSet::new(),
        best_cost: initial_bound.saturating_add(1),
        ended: false,
        calls: 0,
        leaves: 0,
        rule1_prunes: 0,
        rule3_skips: 0,
        rule5_prunes: 0,
    };

    search(&mut state, 0);

    if state.best_cover.is_empty() && state.best_cost != 0
    {
        return Err(MincovError::NoCoverWithinBound);
    }

    if !matrix.covered_by(&state.best_cover)
    {
        return Err(MincovError::CoverVerificationFailed);
    }

    Ok(MincovResult {
        cover: state.best_cover,
        cost: state.best_cost,
        calls: state.calls,
        leaves: state.leaves,
        rule1_prunes: state.rule1_prunes,
        rule3_skips: state.rule3_skips,
        rule5_prunes: state.rule5_prunes,
    })
}

fn search(state: &mut SearchState, row_num: usize)
{
    state.calls += 1;

    if row_num == state.rows.len()
    {
        state.leaves += 1;
        if state.solution_cost < state.best_cost
        {
            state.best_cost = state.solution_cost;
            state.best_cover = state.solution.clone();
        }
        if state.heuristic
        {
            state.ended = true;
        }
        return;
    }

    if state.rows[row_num]
        .iter()
        .any(|literal| state.solution.contains(literal))
    {
        state.rule3_skips += 1;
        search(state, row_num + 1);
        return;
    }

    for literal in state.order[row_num].clone()
    {
        if state.ended
        {
            break;
        }

        if !state.is_unate && state.solution.contains(&complement(literal))
        {
            state.rule1_prunes += 1;
            continue;
        }

        let literal_cost = weight(&state.weights, literal);
        if state.solution_cost + literal_cost >= state.best_cost
        {
            state.rule5_prunes += 1;
            continue;
        }

        state.solution.insert(literal);
        state.solution_cost += literal_cost;
        search(state, row_num + 1);
        state.solution_cost -= literal_cost;
        state.solution.remove(&literal);
    }
}

fn reorder_rows(matrix: &MincovMatrix, weights: &[i32], option: i32) -> Vec<Vec<usize>>
{
    let mut row_values: Vec<(usize, f64)> = matrix
        .rows()
        .iter()
        .map(|(row, columns)|
        {
            let mut value = if option & 1 != 0
            {
                10_000.0 * columns.len() as f64 + *row as f64
            }
            else if option & 256 != 0
            {
                columns
                    .iter()
                    .map(|column| weight(weights, *column) as f64)
                    .sum()
            }
            else
            {
                columns.len() as f64
            };

            if option & (512 | 1024) != 0 && row_is_unate(columns)
            {
                if option & 512 != 0
                {
                    value += 10_000.0;
                }
                else
                {
                    value -= 10_000.0;
                }
            }

            (*row, value)
        })
        .collect();

    if option & 2 == 0
    {
        if option & 4096 != 0
        {
            row_values.sort_by(|left, right| compare_f64(left.1, right.1));
        }
        else
        {
            row_values.sort_by(|left, right| compare_f64(right.1, left.1));
        }
    }

    row_values
        .into_iter()
        .map(|(row, _)| matrix.row(row).unwrap().iter().copied().collect())
        .collect()
}

fn order_columns(rows: &[Vec<usize>], weights: &[i32], option: i32, is_unate: bool) -> Vec<Vec<usize>>
{
    let mut saved_values = HashMap::<usize, f64>::new();
    let mut ordered = vec![Vec::new(); rows.len()];

    for row in (0..rows.len()).rev()
    {
        let mut values = Vec::new();
        for literal in &rows[row]
        {
            let key = if is_unate
            {
                *literal
            }
            else
            {
                literal / 2
            };

            if is_unate || option & 4 == 0 || literal % 2 == 0
            {
                *saved_values.entry(key).or_default() += 1.0;
            }

            let mut value = *saved_values.get(&key).unwrap_or(&0.0);
            if !is_unate
            {
                if option & 64 != 0
                {
                    let literal_weight = weight(weights, *literal);
                    if literal_weight > 0
                    {
                        value /= literal_weight as f64;
                    }
                    else
                    {
                        value *= 2.0;
                    }
                }

                if option & 32 != 0 && literal % 2 == 1
                {
                    value = 1_000_000.0;
                }
                else if option & 128 != 0 && literal % 2 == 1
                {
                    value = 0.0;
                }
            }

            values.push((*literal, value));
        }

        if option & 8 == 0
        {
            if option & 16 != 0
            {
                values.sort_by(|left, right| compare_f64(left.1, right.1));
            }
            else
            {
                values.sort_by(|left, right| compare_f64(right.1, left.1));
            }
        }

        ordered[row] = values.into_iter().map(|(literal, _)| literal).collect();
    }

    ordered
}

fn expand_weights(weights: Option<&[i32]>, max_col: usize) -> Vec<i32>
{
    let mut expanded = vec![1; max_col + 1];
    if let Some(weights) = weights
    {
        for (index, value) in weights.iter().enumerate().take(expanded.len())
        {
            expanded[index] = *value;
        }
    }
    expanded
}

fn weight(weights: &[i32], col: usize) -> i32
{
    weights.get(col).copied().unwrap_or(1)
}

fn complement(literal: usize) -> usize
{
    if literal & 1 == 1
    {
        literal - 1
    }
    else
    {
        literal + 1
    }
}

fn row_is_unate(columns: &BTreeSet<usize>) -> bool
{
    let has_odd = columns.iter().any(|column| column % 2 == 1);
    let has_even = columns.iter().any(|column| column % 2 == 0);
    has_odd != has_even
}

fn compare_f64(left: f64, right: f64) -> std::cmp::Ordering
{
    left.partial_cmp(&right).unwrap_or(std::cmp::Ordering::Equal)
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn set(values: &[usize]) -> BTreeSet<usize>
    {
        values.iter().copied().collect()
    }

    #[test]
    fn empty_matrix_has_empty_zero_cost_cover()
    {
        let matrix = MincovMatrix::new();

        let result = sm_mat_minimum_cover(&matrix, None, false, 0, None, 0).unwrap();

        assert_eq!(result.cover, BTreeSet::new());
        assert_eq!(result.cost, 0);
    }

    #[test]
    fn unate_cover_selects_lowest_weight_column_covering_all_rows()
    {
        let matrix = MincovMatrix::from_rows([vec![0, 1], vec![1, 2]]);

        let result = sm_mat_minimum_cover(&matrix, Some(&[3, 1, 2]), false, 0, None, 0).unwrap();

        assert_eq!(result.cover, set(&[1]));
        assert_eq!(result.cost, 1);
        assert!(matrix.covered_by(&result.cover));
    }

    #[test]
    fn binate_cover_rejects_complementary_literals()
    {
        let matrix = MincovMatrix::from_rows([vec![0], vec![1]]);

        let result = sm_mat_bin_minimum_cover(&matrix, None, false, 0, None, 0);

        assert_eq!(result, Err(MincovError::NoCoverWithinBound));
    }

    #[test]
    fn binate_cover_finds_nonconflicting_solution()
    {
        let matrix = MincovMatrix::from_rows([vec![0, 1], vec![0, 2], vec![1, 3]]);

        let result = sm_mat_bin_minimum_cover(&matrix, Some(&[1, 1, 4, 1]), false, 0, None, 0)
            .unwrap();

        assert_eq!(result.cover, set(&[0, 3]));
        assert_eq!(result.cost, 2);
    }

    #[test]
    fn strict_upper_bound_reports_no_cover()
    {
        let matrix = MincovMatrix::from_rows([vec![0], vec![2]]);

        let result = sm_mat_minimum_cover(&matrix, None, false, 0, Some(1), 0);

        assert_eq!(result, Err(MincovError::NoCoverWithinBound));
    }

    #[test]
    fn heuristic_stops_at_first_leaf()
    {
        let matrix = MincovMatrix::from_rows([vec![0, 1], vec![0, 2]]);

        let result = sm_mat_minimum_cover(&matrix, Some(&[5, 1, 1]), true, 0, None, 8).unwrap();

        assert_eq!(result.cover, set(&[0]));
        assert_eq!(result.leaves, 1);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port()
    {
        let source = include_str!("bin_mincov.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
