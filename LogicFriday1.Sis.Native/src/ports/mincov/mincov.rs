//! Native Rust port of `LogicSynthesis/sis/mincov/mincov.c`.
//!
//! The SIS implementation solves sparse set cover by repeatedly applying
//! dominance and essential-column reductions, estimating a lower bound from a
//! maximal independent row set, and branching on the best remaining column.
//! This port keeps that behavior on top of the native sparse matrix model.

use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

use crate::ports::sparse::matrix::{SparseMatrix, SparseVector};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinCoverResult {
    pub cover: Vec<usize>,
    pub cost: i32,
    pub stats: MinCoverStats,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinCoverStats {
    pub max_depth: usize,
    pub nodes: usize,
    pub components: usize,
    pub gimpel_reductions: usize,
    pub lower_bound: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MinCoverError {
    MissingWeight { col: usize },
    NonPositiveWeight { col: usize, weight: i32 },
    CoverVerificationFailed,
}

impl fmt::Display for MinCoverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingWeight { col } => write!(f, "missing mincov weight for column {col}"),
            Self::NonPositiveWeight { col, weight } => {
                write!(f, "non-positive mincov weight {weight} for column {col}")
            }
            Self::CoverVerificationFailed => write!(f, "mincov cover verification failed"),
        }
    }
}

impl Error for MinCoverError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Solution {
    row: BTreeSet<usize>,
    cost: i32,
}

impl Solution {
    fn new() -> Self {
        Self {
            row: BTreeSet::new(),
            cost: 0,
        }
    }

    fn add(&mut self, weights: Option<&[i32]>, col: usize) -> Result<(), MinCoverError> {
        self.row.insert(col);
        self.cost += weight(weights, col)?;
        Ok(())
    }

    fn accept(
        &mut self,
        matrix: &mut SparseMatrix,
        weights: Option<&[i32]>,
        col: usize,
    ) -> Result<(), MinCoverError> {
        self.add(weights, col)?;

        let Some(column) = matrix.col(col) else {
            return Ok(());
        };
        for row in column.elements() {
            matrix.delete_row(*row);
        }

        Ok(())
    }

    fn reject(&mut self, matrix: &mut SparseMatrix, col: usize) {
        matrix.delete_col(col);
    }

    fn cover(&self) -> Vec<usize> {
        self.row.iter().copied().collect()
    }
}

#[derive(Clone, Debug)]
struct SearchStats {
    max_depth: usize,
    nodes: usize,
    component: usize,
    components: usize,
    gimpel_reductions: usize,
    gimpel: i32,
    no_branching: bool,
    lower_bound: i32,
}

impl SearchStats {
    fn new(heuristic: bool, debug_level: usize) -> Self {
        let _ = debug_level;
        Self {
            max_depth: 0,
            nodes: 0,
            component: 0,
            components: 0,
            gimpel_reductions: 0,
            gimpel: 0,
            no_branching: heuristic,
            lower_bound: -1,
        }
    }

    fn result_stats(&self) -> MinCoverStats {
        MinCoverStats {
            max_depth: self.max_depth,
            nodes: self.nodes,
            components: self.components,
            gimpel_reductions: self.gimpel_reductions,
            lower_bound: self.lower_bound,
        }
    }
}

pub fn sm_minimum_cover(
    matrix: &SparseMatrix,
    weights: Option<&[i32]>,
    heuristic: bool,
    debug_level: usize,
) -> Result<MinCoverResult, MinCoverError> {
    validate_weights(matrix, weights)?;
    if matrix.row_count() == 0 {
        return Ok(MinCoverResult {
            cover: Vec::new(),
            cost: 0,
            stats: SearchStats::new(heuristic, debug_level).result_stats(),
        });
    }

    let mut stats = SearchStats::new(heuristic, debug_level);
    let mut bound = 1;
    for col in matrix.cols() {
        bound += weight(weights, col.index())?;
    }

    let mut select = Solution::new();
    let mut working = matrix.clone();
    let best = sm_mincov(&mut working, &mut select, weights, 0, bound, 0, &mut stats)?
        .ok_or(MinCoverError::CoverVerificationFailed)?;

    if !verify_cover(matrix, &best.row) {
        return Err(MinCoverError::CoverVerificationFailed);
    }

    Ok(MinCoverResult {
        cover: best.cover(),
        cost: best.cost,
        stats: stats.result_stats(),
    })
}

pub fn minimum_cover(matrix: &SparseMatrix) -> Result<Vec<usize>, MinCoverError> {
    Ok(sm_minimum_cover(matrix, None, false, 0)?.cover)
}

fn sm_mincov(
    matrix: &mut SparseMatrix,
    select: &mut Solution,
    weights: Option<&[i32]>,
    lower_bound: i32,
    mut bound: i32,
    depth: usize,
    stats: &mut SearchStats,
) -> Result<Option<Solution>, MinCoverError> {
    stats.nodes += 1;
    stats.max_depth = stats.max_depth.max(depth);

    select_essential(matrix, select, weights, bound)?;
    if select.cost >= bound {
        return Ok(None);
    }

    if weights.is_none() {
        if let Some(best) = gimpel_reduce(matrix, select, lower_bound, bound, depth, stats)? {
            return Ok(best);
        }
    }

    let indep = maximal_independent_set(matrix, weights)?;
    let new_lower_bound = (select.cost + indep.cost).max(lower_bound);
    let pick = select_column(matrix, weights, Some(&indep));

    if depth == 0 {
        stats.lower_bound = new_lower_bound + stats.gimpel;
    }

    if new_lower_bound >= bound {
        return Ok(None);
    }

    if matrix.row_count() == 0 {
        return Ok(Some(select.clone()));
    }

    if let Some((mut left, mut right)) = block_partition(matrix) {
        if left.col_count() > right.col_count() {
            std::mem::swap(&mut left, &mut right);
        }

        stats.components += 1;
        let mut select1 = Solution::new();
        stats.component += 1;
        let best1 = sm_mincov(
            &mut left,
            &mut select1,
            weights,
            0,
            bound - select.cost,
            depth + 1,
            stats,
        )?;
        stats.component -= 1;

        return match best1 {
            Some(best1) => {
                for col in best1.row {
                    select.add(weights, col)?;
                }
                sm_mincov(
                    &mut right,
                    select,
                    weights,
                    new_lower_bound,
                    bound,
                    depth + 1,
                    stats,
                )
            }
            None => Ok(None),
        };
    }

    let Some(pick) = pick else {
        return Ok(None);
    };

    let mut accept_matrix = matrix.clone();
    let mut accept_select = select.clone();
    accept_select.accept(&mut accept_matrix, weights, pick)?;
    let best1 = sm_mincov(
        &mut accept_matrix,
        &mut accept_select,
        weights,
        new_lower_bound,
        bound,
        depth + 1,
        stats,
    )?;

    if let Some(best1) = &best1 {
        if bound > best1.cost {
            bound = best1.cost;
        }
    }

    if stats.no_branching {
        return Ok(best1);
    }

    if best1
        .as_ref()
        .is_some_and(|solution| solution.cost == new_lower_bound)
    {
        return Ok(best1);
    }

    let mut reject_matrix = matrix.clone();
    let mut reject_select = select.clone();
    reject_select.reject(&mut reject_matrix, pick);
    let best2 = sm_mincov(
        &mut reject_matrix,
        &mut reject_select,
        weights,
        new_lower_bound,
        bound,
        depth + 1,
        stats,
    )?;

    Ok(choose_best(best1, best2))
}

fn select_column(
    matrix: &SparseMatrix,
    weights: Option<&[i32]>,
    indep: Option<&Solution>,
) -> Option<usize> {
    let mut candidate_cols = BTreeSet::new();
    if let Some(indep) = indep {
        for row in &indep.row {
            if let Some(row_vector) = matrix.row(*row) {
                candidate_cols.extend(row_vector.elements().iter().copied());
            }
        }
    } else {
        candidate_cols.extend(matrix.cols().map(|col| col.index()));
    }

    let mut best_col = None;
    let mut best_score = -1.0;
    for col in candidate_cols {
        let Some(column) = matrix.col(col) else {
            continue;
        };

        let mut score = 0.0;
        for row in column.elements() {
            let Some(row_vector) = matrix.row(*row) else {
                continue;
            };
            if row_vector.len() > 1 {
                score += 1.0 / ((row_vector.len() as f64) - 1.0);
            }
        }

        let column_weight = weight(weights, col).unwrap_or(1) as f64;
        score /= column_weight;
        if score > best_score {
            best_score = score;
            best_col = Some(col);
        }
    }

    best_col
}

fn select_essential(
    matrix: &mut SparseMatrix,
    select: &mut Solution,
    weights: Option<&[i32]>,
    bound: i32,
) -> Result<(), MinCoverError> {
    loop {
        let deleted_cols = col_dominance(matrix, weights);

        let essentials = matrix
            .rows()
            .filter(|row| row.len() == 1)
            .flat_map(|row| row.elements().to_vec())
            .collect::<BTreeSet<_>>();
        let essential_count = essentials.len();

        for col in essentials {
            select.accept(matrix, weights, col)?;
            if select.cost >= bound {
                return Ok(());
            }
        }

        let deleted_rows = row_dominance(matrix);
        if deleted_cols == 0 && deleted_rows == 0 && essential_count == 0 {
            return Ok(());
        }
    }
}

fn row_dominance(matrix: &mut SparseMatrix) -> usize {
    let row_count = matrix.row_count();
    let rows = matrix.rows().collect::<Vec<_>>();

    for row in rows {
        if matrix.row(row.index()).is_none() || row.is_empty() {
            continue;
        }

        let Some(least_col) = row
            .elements()
            .iter()
            .filter_map(|col| matrix.col(*col))
            .min_by_key(SparseVector::len)
        else {
            continue;
        };

        for other_row_id in least_col.elements().to_vec() {
            let Some(other_row) = matrix.row(other_row_id) else {
                continue;
            };
            if (other_row.len() > row.len()
                || (other_row.len() == row.len() && other_row.index() > row.index()))
                && row_contains(&other_row, &row)
            {
                matrix.delete_row(other_row.index());
            }
        }
    }

    row_count - matrix.row_count()
}

fn col_dominance(matrix: &mut SparseMatrix, weights: Option<&[i32]>) -> usize {
    let col_count = matrix.col_count();
    let cols = matrix.cols().collect::<Vec<_>>();

    for col in cols {
        if matrix.col(col.index()).is_none() || col.is_empty() {
            continue;
        }

        let Some(least_row) = col
            .elements()
            .iter()
            .filter_map(|row| matrix.row(*row))
            .min_by_key(SparseVector::len)
        else {
            continue;
        };

        for other_col_id in least_row.elements().to_vec() {
            let Some(other_col) = matrix.col(other_col_id) else {
                continue;
            };
            if weights.is_some_and(|w| w[other_col.index()] > w[col.index()]) {
                continue;
            }
            if (other_col.len() > col.len()
                || (other_col.len() == col.len() && other_col.index() > col.index()))
                && col_contains(&other_col, &col)
            {
                matrix.delete_col(col.index());
                break;
            }
        }
    }

    col_count - matrix.col_count()
}

fn maximal_independent_set(
    matrix: &SparseMatrix,
    weights: Option<&[i32]>,
) -> Result<Solution, MinCoverError> {
    let mut indep = Solution::new();
    let mut intersection = build_intersection_matrix(matrix);

    while intersection.row_count() > 0 {
        let Some(best_row) = intersection.rows().min_by_key(SparseVector::len) else {
            break;
        };
        let row_num = best_row.index();

        let least_weight = if let Some(weights) = weights {
            matrix
                .row(row_num)
                .and_then(|row| {
                    row.elements()
                        .iter()
                        .map(|col| weights.get(*col).copied())
                        .collect::<Option<Vec<_>>>()
                })
                .and_then(|weights| weights.into_iter().min())
                .unwrap_or(1)
        } else {
            1
        };

        indep.cost += least_weight;
        indep.row.insert(row_num);

        let intersecting_rows = best_row.elements().to_vec();
        for row in intersecting_rows {
            intersection.delete_row(row);
            intersection.delete_col(row);
        }
    }

    Ok(indep)
}

fn build_intersection_matrix(matrix: &SparseMatrix) -> SparseMatrix {
    let mut intersection = SparseMatrix::new();
    for row in matrix.rows() {
        let mut reachable = BTreeSet::new();
        for col in row.elements() {
            if let Some(column) = matrix.col(*col) {
                reachable.extend(column.elements().iter().copied());
            }
        }
        for reachable_row in reachable {
            intersection.insert(row.index(), reachable_row);
        }
    }

    intersection
}

fn block_partition(matrix: &SparseMatrix) -> Option<(SparseMatrix, SparseMatrix)> {
    let first_row = matrix.rows().next()?.index();
    let mut visited_rows = BTreeSet::new();
    let mut visited_cols = BTreeSet::new();
    let mut queue = VecDeque::from([PartitionItem::Row(first_row)]);

    while let Some(item) = queue.pop_front() {
        match item {
            PartitionItem::Row(row) if visited_rows.insert(row) => {
                if let Some(row_vector) = matrix.row(row) {
                    for col in row_vector.elements() {
                        if !visited_cols.contains(col) {
                            queue.push_back(PartitionItem::Col(*col));
                        }
                    }
                }
            }
            PartitionItem::Col(col) if visited_cols.insert(col) => {
                if let Some(col_vector) = matrix.col(col) {
                    for row in col_vector.elements() {
                        if !visited_rows.contains(row) {
                            queue.push_back(PartitionItem::Row(*row));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if visited_rows.len() == matrix.row_count() {
        return None;
    }

    let mut left = SparseMatrix::new();
    let mut right = SparseMatrix::new();
    for row in matrix.rows() {
        let target = if visited_rows.contains(&row.index()) {
            &mut left
        } else {
            &mut right
        };
        for col in row.elements() {
            target.insert(row.index(), *col);
        }
    }

    Some((left, right))
}

fn gimpel_reduce(
    matrix: &mut SparseMatrix,
    select: &mut Solution,
    lower_bound: i32,
    bound: i32,
    depth: usize,
    stats: &mut SearchStats,
) -> Result<Option<Option<Solution>>, MinCoverError> {
    let mut reduction = None;
    for row in matrix.rows() {
        if row.len() != 2 {
            continue;
        }

        let c1 = row.elements()[0];
        let c2 = row.elements()[1];
        let c1_len = matrix.col(c1).map_or(0, |col| col.len());
        let c2_len = matrix.col(c2).map_or(0, |col| col.len());

        if c1_len == 2 {
            reduction = Some((row.index(), c1, c2));
            break;
        }
        if c2_len == 2 {
            reduction = Some((row.index(), c2, c1));
            break;
        }
    }

    let Some((primary_row, c1, c2)) = reduction else {
        return Ok(None);
    };
    let Some(c1_vector) = matrix.col(c1) else {
        return Ok(None);
    };
    let Some(secondary_row) = c1_vector
        .elements()
        .iter()
        .copied()
        .find(|row| *row != primary_row)
    else {
        return Ok(None);
    };
    let mut save_secondary = matrix
        .row(secondary_row)
        .map(|row| row.elements().iter().copied().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    save_secondary.remove(&c1);

    let c2_rows = matrix
        .col(c2)
        .map(|col| col.elements().to_vec())
        .unwrap_or_default();
    for row in c2_rows {
        if row != primary_row {
            for col in &save_secondary {
                matrix.insert(row, *col);
            }
        }
    }

    matrix.delete_col(c1);
    matrix.delete_col(c2);
    matrix.delete_row(primary_row);
    matrix.delete_row(secondary_row);

    stats.gimpel_reductions += 1;
    stats.gimpel += 1;
    let mut best = sm_mincov(
        matrix,
        select,
        None,
        lower_bound - 1,
        bound - 1,
        depth,
        stats,
    )?;
    stats.gimpel -= 1;

    if let Some(solution) = &mut best {
        if sets_intersect(&save_secondary, &solution.row) {
            solution.add(None, c2)?;
        } else {
            solution.add(None, c1)?;
        }
    }

    Ok(Some(best))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PartitionItem {
    Row(usize),
    Col(usize),
}

fn choose_best(best1: Option<Solution>, best2: Option<Solution>) -> Option<Solution> {
    match (best1, best2) {
        (Some(left), Some(right)) if left.cost <= right.cost => Some(left),
        (Some(_), Some(right)) => Some(right),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn verify_cover(matrix: &SparseMatrix, cover: &BTreeSet<usize>) -> bool {
    matrix
        .rows()
        .all(|row| row.elements().iter().any(|col| cover.contains(col)))
}

fn validate_weights(matrix: &SparseMatrix, weights: Option<&[i32]>) -> Result<(), MinCoverError> {
    let Some(weights) = weights else {
        return Ok(());
    };

    for col in matrix.cols() {
        let Some(&weight) = weights.get(col.index()) else {
            return Err(MinCoverError::MissingWeight { col: col.index() });
        };
        if weight <= 0 {
            return Err(MinCoverError::NonPositiveWeight {
                col: col.index(),
                weight,
            });
        }
    }

    Ok(())
}

fn weight(weights: Option<&[i32]>, col: usize) -> Result<i32, MinCoverError> {
    match weights {
        Some(weights) => weights
            .get(col)
            .copied()
            .ok_or(MinCoverError::MissingWeight { col }),
        None => Ok(1),
    }
}

fn row_contains(container: &SparseVector, contained: &SparseVector) -> bool {
    contained
        .elements()
        .iter()
        .all(|col| container.contains(*col))
}

fn col_contains(container: &SparseVector, contained: &SparseVector) -> bool {
    contained
        .elements()
        .iter()
        .all(|row| container.contains(*row))
}

fn sets_intersect(left: &BTreeSet<usize>, right: &BTreeSet<usize>) -> bool {
    left.iter().any(|entry| right.contains(entry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_matrix_has_empty_cover() {
        let matrix = SparseMatrix::new();

        let result = sm_minimum_cover(&matrix, None, false, 0).unwrap();

        assert_eq!(result.cover, Vec::<usize>::new());
        assert_eq!(result.cost, 0);
    }

    #[test]
    fn selects_essential_column_and_removes_dominated_work() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 1);
        matrix.insert(1, 1);
        matrix.insert(1, 2);

        let result = sm_minimum_cover(&matrix, None, false, 0).unwrap();

        assert_eq!(result.cover, vec![1]);
        assert_eq!(result.cost, 1);
        assert!(verify_cover(&matrix, &result.cover.into_iter().collect()));
    }

    #[test]
    fn chooses_minimum_weight_cover() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 0);
        matrix.insert(0, 1);
        matrix.insert(1, 0);
        matrix.insert(1, 2);
        matrix.insert(2, 1);
        matrix.insert(2, 2);
        let weights = [5, 1, 1];

        let result = sm_minimum_cover(&matrix, Some(&weights), false, 0).unwrap();

        assert_eq!(result.cover, vec![1, 2]);
        assert_eq!(result.cost, 2);
    }

    #[test]
    fn partitions_independent_blocks_and_combines_cover() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 0);
        matrix.insert(0, 1);
        matrix.insert(1, 1);
        matrix.insert(2, 4);
        matrix.insert(3, 4);
        matrix.insert(3, 5);

        let (left, right) = block_partition(&matrix).expect("matrix has two disjoint blocks");
        assert_eq!(left.row_count(), 2);
        assert_eq!(right.row_count(), 2);

        let result = sm_minimum_cover(&matrix, None, false, 0).unwrap();

        assert_eq!(result.cover, vec![1, 4]);
        assert_eq!(result.cost, 2);
    }

    #[test]
    fn heuristic_returns_a_valid_cover_without_second_branch() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 0);
        matrix.insert(0, 1);
        matrix.insert(1, 0);
        matrix.insert(1, 2);
        matrix.insert(2, 1);
        matrix.insert(2, 2);

        let result = sm_minimum_cover(&matrix, None, true, 0).unwrap();

        assert!(verify_cover(&matrix, &result.cover.into_iter().collect()));
    }

    #[test]
    fn rejects_invalid_weight_tables() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 3);

        assert_eq!(
            sm_minimum_cover(&matrix, Some(&[1, 1]), false, 0).unwrap_err(),
            MinCoverError::MissingWeight { col: 3 }
        );
        assert_eq!(
            sm_minimum_cover(&matrix, Some(&[1, 1, 1, 0]), false, 0).unwrap_err(),
            MinCoverError::NonPositiveWeight { col: 3, weight: 0 }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("mincov.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
