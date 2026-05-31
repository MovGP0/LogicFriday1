//! Sparse matrix and minimum-cover support ported from Espresso.
//!
//! The original C implementation stores every non-zero as an element linked in
//! both row and column lists. This Rust port keeps the same sorted set
//! semantics with `BTreeMap`/`BTreeSet`, which preserves deterministic traversal
//! order while avoiding pointer aliasing.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SparseRow {
    pub row_num: usize,
    pub flag: bool,
    cols: BTreeSet<usize>,
}

impl SparseRow {
    pub fn new(row_num: usize) -> Self {
        Self {
            row_num,
            flag: false,
            cols: BTreeSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.cols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cols.is_empty()
    }

    pub fn cols(&self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.cols.iter().copied()
    }

    pub fn first_col(&self) -> Option<usize> {
        self.cols.iter().next().copied()
    }

    pub fn last_col(&self) -> Option<usize> {
        self.cols.iter().next_back().copied()
    }

    pub fn insert(&mut self, col: usize) -> bool {
        self.cols.insert(col)
    }

    pub fn remove(&mut self, col: usize) -> bool {
        self.cols.remove(&col)
    }

    pub fn contains_col(&self, col: usize) -> bool {
        self.cols.contains(&col)
    }

    /// Returns true when `other` contains every column in `self`.
    pub fn is_contained_by(&self, other: &SparseRow) -> bool {
        self.cols.is_subset(&other.cols)
    }

    pub fn intersects(&self, other: &SparseRow) -> bool {
        self.cols.iter().any(|col| other.cols.contains(col))
    }

    pub fn intersection(&self, other: &SparseRow) -> SparseRow {
        let mut result = SparseRow::new(0);
        result.cols = self.cols.intersection(&other.cols).copied().collect();
        result
    }

    pub fn compare_lexicographic(&self, other: &SparseRow) -> i32 {
        compare_sorted(self.cols(), other.cols())
    }

    pub fn hash_mod(&self, modulus: usize) -> usize {
        if modulus == 0 {
            return 0;
        }
        self.cols
            .iter()
            .fold(0usize, |sum, col| (sum * 17 + col) % modulus)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SparseCol {
    pub col_num: usize,
    pub flag: bool,
    rows: BTreeSet<usize>,
}

impl SparseCol {
    pub fn new(col_num: usize) -> Self {
        Self {
            col_num,
            flag: false,
            rows: BTreeSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn rows(&self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.rows.iter().copied()
    }

    pub fn first_row(&self) -> Option<usize> {
        self.rows.iter().next().copied()
    }

    pub fn last_row(&self) -> Option<usize> {
        self.rows.iter().next_back().copied()
    }

    pub fn contains_row(&self, row: usize) -> bool {
        self.rows.contains(&row)
    }

    /// Returns true when `other` contains every row in `self`.
    pub fn is_contained_by(&self, other: &SparseCol) -> bool {
        self.rows.is_subset(&other.rows)
    }

    pub fn intersects(&self, other: &SparseCol) -> bool {
        self.rows.iter().any(|row| other.rows.contains(row))
    }

    pub fn intersection(&self, other: &SparseCol) -> SparseCol {
        let mut result = SparseCol::new(0);
        result.rows = self.rows.intersection(&other.rows).copied().collect();
        result
    }

    pub fn compare_lexicographic(&self, other: &SparseCol) -> i32 {
        compare_sorted(self.rows(), other.rows())
    }

    pub fn hash_mod(&self, modulus: usize) -> usize {
        if modulus == 0 {
            return 0;
        }
        self.rows
            .iter()
            .fold(0usize, |sum, row| (sum * 17 + row) % modulus)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SparseMatrix {
    rows: BTreeMap<usize, SparseRow>,
    cols: BTreeMap<usize, SparseCol>,
}

impl SparseMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_size(_row: usize, _col: usize) -> Self {
        Self::new()
    }

    pub fn nrows(&self) -> usize {
        self.rows.len()
    }

    pub fn ncols(&self) -> usize {
        self.cols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn num_elements(&self) -> usize {
        self.rows.values().map(SparseRow::len).sum()
    }

    pub fn row(&self, row: usize) -> Option<&SparseRow> {
        self.rows.get(&row)
    }

    pub fn col(&self, col: usize) -> Option<&SparseCol> {
        self.cols.get(&col)
    }

    pub fn row_numbers(&self) -> impl Iterator<Item = usize> + '_ {
        self.rows.keys().copied()
    }

    pub fn col_numbers(&self) -> impl Iterator<Item = usize> + '_ {
        self.cols.keys().copied()
    }

    pub fn rows(&self) -> impl Iterator<Item = &SparseRow> {
        self.rows.values()
    }

    pub fn cols(&self) -> impl Iterator<Item = &SparseCol> {
        self.cols.values()
    }

    pub fn contains(&self, row: usize, col: usize) -> bool {
        self.rows
            .get(&row)
            .is_some_and(|sparse_row| sparse_row.contains_col(col))
    }

    pub fn insert(&mut self, row: usize, col: usize) -> bool {
        let inserted = self
            .rows
            .entry(row)
            .or_insert_with(|| SparseRow::new(row))
            .cols
            .insert(col);

        self.cols
            .entry(col)
            .or_insert_with(|| SparseCol::new(col))
            .rows
            .insert(row);

        inserted
    }

    pub fn remove(&mut self, row: usize, col: usize) -> bool {
        if !self.contains(row, col) {
            return false;
        }

        let remove_row = {
            let sparse_row = self.rows.get_mut(&row).expect("row exists");
            sparse_row.cols.remove(&col);
            sparse_row.is_empty()
        };
        if remove_row {
            self.rows.remove(&row);
        }

        let remove_col = {
            let sparse_col = self.cols.get_mut(&col).expect("column exists");
            sparse_col.rows.remove(&row);
            sparse_col.is_empty()
        };
        if remove_col {
            self.cols.remove(&col);
        }

        true
    }

    pub fn delete_row(&mut self, row: usize) -> bool {
        let Some(sparse_row) = self.rows.remove(&row) else {
            return false;
        };

        for col in sparse_row.cols {
            let remove_col = {
                let sparse_col = self.cols.get_mut(&col).expect("column exists");
                sparse_col.rows.remove(&row);
                sparse_col.is_empty()
            };
            if remove_col {
                self.cols.remove(&col);
            }
        }
        true
    }

    pub fn delete_col(&mut self, col: usize) -> bool {
        let Some(sparse_col) = self.cols.remove(&col) else {
            return false;
        };

        for row in sparse_col.rows {
            let remove_row = {
                let sparse_row = self.rows.get_mut(&row).expect("row exists");
                sparse_row.cols.remove(&col);
                sparse_row.is_empty()
            };
            if remove_row {
                self.rows.remove(&row);
            }
        }
        true
    }

    pub fn copy_row_from(&mut self, dest_row: usize, row: &SparseRow) {
        for col in row.cols() {
            self.insert(dest_row, col);
        }
    }

    pub fn copy_col_from(&mut self, dest_col: usize, col: &SparseCol) {
        for row in col.rows() {
            self.insert(row, dest_col);
        }
    }

    pub fn longest_row(&self) -> Option<&SparseRow> {
        self.rows.values().max_by_key(|row| row.len())
    }

    pub fn longest_col(&self) -> Option<&SparseCol> {
        self.cols.values().max_by_key(|col| col.len())
    }

    pub fn to_pairs(&self) -> Vec<(usize, usize)> {
        self.rows()
            .flat_map(|row| row.cols().map(move |col| (row.row_num, col)))
            .collect()
    }

    pub fn print_matrix(&self) -> String {
        let mut out = String::new();
        let cols: Vec<_> = self.col_numbers().collect();
        if cols.iter().any(|col| *col >= 100) {
            out.push_str("    ");
            for col in &cols {
                out.push(char::from(b'0' + ((*col / 100) % 10) as u8));
            }
            out.push('\n');
        }
        if cols.iter().any(|col| *col >= 10) {
            out.push_str("    ");
            for col in &cols {
                out.push(char::from(b'0' + ((*col / 10) % 10) as u8));
            }
            out.push('\n');
        }

        out.push_str("    ");
        for col in &cols {
            out.push(char::from(b'0' + (*col % 10) as u8));
        }
        out.push('\n');

        out.push_str("    ");
        out.extend(std::iter::repeat_n('-', cols.len()));
        out.push('\n');

        for row in self.rows() {
            out.push_str(&format!("{:3}:", row.row_num));
            for col in &cols {
                out.push(if row.contains_col(*col) { '1' } else { '.' });
            }
            out.push('\n');
        }
        out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverSolution {
    pub row: SparseRow,
    pub cost: i32,
}

impl Default for CoverSolution {
    fn default() -> Self {
        Self {
            row: SparseRow::new(0),
            cost: 0,
        }
    }
}

impl CoverSolution {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, weight: Option<&[i32]>, col: usize) {
        self.row.insert(col);
        self.cost += column_weight(weight, col);
    }

    pub fn accept(&mut self, matrix: &mut SparseMatrix, weight: Option<&[i32]>, col: usize) {
        self.add(weight, col);
        let rows: Vec<_> = matrix
            .col(col)
            .map(|sparse_col| sparse_col.rows().collect())
            .unwrap_or_default();
        for row in rows {
            matrix.delete_row(row);
        }
    }

    pub fn reject(&mut self, matrix: &mut SparseMatrix, col: usize) {
        matrix.delete_col(col);
    }

    pub fn choose_best(best1: Option<Self>, best2: Option<Self>) -> Option<Self> {
        match (best1, best2) {
            (Some(first), Some(second)) if first.cost <= second.cost => Some(first),
            (Some(_), Some(second)) => Some(second),
            (Some(first), None) => Some(first),
            (None, Some(second)) => Some(second),
            (None, None) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinCoverStats {
    pub max_depth: usize,
    pub nodes: usize,
    pub component: usize,
    pub comp_count: usize,
    pub gimpel_count: usize,
    pub gimpel: usize,
    pub no_branching: bool,
    pub lower_bound: i32,
}

impl MinCoverStats {
    pub fn new(heuristic: bool) -> Self {
        Self {
            max_depth: 0,
            nodes: 0,
            component: 0,
            comp_count: 0,
            gimpel_count: 0,
            gimpel: 0,
            no_branching: heuristic,
            lower_bound: -1,
        }
    }
}

pub fn minimum_cover(matrix: &SparseMatrix, weight: Option<&[i32]>, heuristic: bool) -> SparseRow {
    if matrix.nrows() == 0 {
        return SparseRow::new(0);
    }

    let mut bound = 1;
    for col in matrix.cols() {
        bound += column_weight(weight, col.col_num);
    }

    let mut select = CoverSolution::new();
    let mut working = matrix.clone();
    let mut stats = MinCoverStats::new(heuristic);
    let best = mincov(&mut working, &mut select, weight, 0, bound, 0, &mut stats)
        .expect("minimum cover should produce a solution");

    if !verify_cover(matrix, &best.row) {
        panic!("mincov: internal error -- cover verification failed");
    }
    best.row
}

/// Builds a sparse matrix from a family of column sets and returns a heuristic
/// minimum cover, matching Espresso's `do_sm_minimum_cover` interface.
pub fn do_sm_minimum_cover<'a, I>(sets: I) -> SparseRow
where
    I: IntoIterator<Item = &'a SparseRow>,
{
    let mut matrix = SparseMatrix::new();

    for (row_num, set) in sets.into_iter().enumerate() {
        for col in set.cols() {
            matrix.insert(row_num, col);
        }
    }

    minimum_cover(&matrix, None, true)
}

fn mincov(
    matrix: &mut SparseMatrix,
    select: &mut CoverSolution,
    weight: Option<&[i32]>,
    lb: i32,
    mut bound: i32,
    depth: usize,
    stats: &mut MinCoverStats,
) -> Option<CoverSolution> {
    stats.nodes += 1;
    stats.max_depth = stats.max_depth.max(depth);

    select_essential(matrix, select, weight, bound);
    if select.cost >= bound {
        return None;
    }

    if weight.is_none() {
        if let Some(best) = gimpel_reduce(matrix, select, weight, lb, bound, depth, stats) {
            return best;
        }
    }

    let indep = maximal_independent_set(matrix, weight);
    let lb_new = (select.cost + indep.cost).max(lb);
    let pick = select_column(matrix, weight, Some(&indep));

    if depth == 0 {
        stats.lower_bound = lb_new + stats.gimpel as i32;
    }

    if lb_new >= bound {
        None
    } else if matrix.nrows() == 0 {
        Some(select.clone())
    } else if let Some((mut left, mut right)) = block_partition(matrix) {
        if left.ncols() > right.ncols() {
            std::mem::swap(&mut left, &mut right);
        }

        stats.comp_count += 1;
        let mut select1 = CoverSolution::new();
        stats.component += 1;
        let best1 = mincov(
            &mut left,
            &mut select1,
            weight,
            0,
            bound - select.cost,
            depth + 1,
            stats,
        );
        stats.component -= 1;

        if let Some(best1) = best1 {
            for col in best1.row.cols() {
                select.add(weight, col);
            }
            mincov(&mut right, select, weight, lb_new, bound, depth + 1, stats)
        } else {
            None
        }
    } else {
        let mut matrix1 = matrix.clone();
        let mut select1 = select.clone();
        select1.accept(&mut matrix1, weight, pick);
        let best1 = mincov(
            &mut matrix1,
            &mut select1,
            weight,
            lb_new,
            bound,
            depth + 1,
            stats,
        );

        if let Some(best1_ref) = &best1 {
            bound = bound.min(best1_ref.cost);
        }

        if stats.no_branching {
            return best1;
        }

        if best1.as_ref().is_some_and(|best| best.cost == lb_new) {
            return best1;
        }

        let mut matrix2 = matrix.clone();
        let mut select2 = select.clone();
        select2.reject(&mut matrix2, pick);
        let best2 = mincov(
            &mut matrix2,
            &mut select2,
            weight,
            lb_new,
            bound,
            depth + 1,
            stats,
        );

        CoverSolution::choose_best(best1, best2)
    }
}

fn select_column(
    matrix: &SparseMatrix,
    weight: Option<&[i32]>,
    indep: Option<&CoverSolution>,
) -> usize {
    let mut indep_cols = SparseRow::new(0);

    if let Some(indep) = indep {
        for row_num in indep.row.cols() {
            if let Some(row) = matrix.row(row_num) {
                for col in row.cols() {
                    indep_cols.insert(col);
                }
            }
        }
    } else {
        for col in matrix.col_numbers() {
            indep_cols.insert(col);
        }
    }

    let mut best_col = usize::MAX;
    let mut best = -1.0f64;

    for candidate_col in indep_cols.cols() {
        let Some(col) = matrix.col(candidate_col) else {
            continue;
        };

        let mut value = 0.0;
        for row_num in col.rows() {
            let row_len = matrix.row(row_num).map_or(0, SparseRow::len);
            value += 1.0 / (row_len as f64 - 1.0);
        }
        value /= column_weight(weight, col.col_num) as f64;

        if value > best {
            best_col = col.col_num;
            best = value;
        }
    }

    best_col
}

fn select_essential(
    matrix: &mut SparseMatrix,
    select: &mut CoverSolution,
    weight: Option<&[i32]>,
    bound: i32,
) {
    loop {
        let delcols = col_dominance(matrix, weight);

        let essentials: Vec<_> = matrix
            .rows()
            .filter(|row| row.len() == 1)
            .filter_map(SparseRow::first_col)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        for col in &essentials {
            select.accept(matrix, weight, *col);
            if select.cost >= bound {
                return;
            }
        }

        let delrows = row_dominance(matrix);
        if delcols == 0 && delrows == 0 && essentials.is_empty() {
            break;
        }
    }
}

pub fn row_dominance(matrix: &mut SparseMatrix) -> usize {
    let row_count = matrix.nrows();
    let row_nums: Vec<_> = matrix.row_numbers().collect();

    for row_num in row_nums {
        let Some(row) = matrix.row(row_num).cloned() else {
            continue;
        };
        let Some(least_col_num) = row
            .cols()
            .filter_map(|col| matrix.col(col).map(|sparse_col| (col, sparse_col.len())))
            .min_by_key(|(_, len)| *len)
            .map(|(col, _)| col)
        else {
            continue;
        };

        let candidates: Vec<_> = matrix
            .col(least_col_num)
            .map(|col| col.rows().collect())
            .unwrap_or_default();

        for candidate_num in candidates {
            let Some(candidate) = matrix.row(candidate_num).cloned() else {
                continue;
            };
            if (candidate.len() > row.len()
                || (candidate.len() == row.len() && candidate.row_num > row.row_num))
                && row.is_contained_by(&candidate)
            {
                matrix.delete_row(candidate.row_num);
            }
        }
    }

    row_count - matrix.nrows()
}

pub fn col_dominance(matrix: &mut SparseMatrix, weight: Option<&[i32]>) -> usize {
    let col_count = matrix.ncols();
    let col_nums: Vec<_> = matrix.col_numbers().collect();

    for col_num in col_nums {
        let Some(col) = matrix.col(col_num).cloned() else {
            continue;
        };
        let Some(least_row_num) = col
            .rows()
            .filter_map(|row| matrix.row(row).map(|sparse_row| (row, sparse_row.len())))
            .min_by_key(|(_, len)| *len)
            .map(|(row, _)| row)
        else {
            continue;
        };

        let candidates: Vec<_> = matrix
            .row(least_row_num)
            .map(|row| row.cols().collect())
            .unwrap_or_default();

        for candidate_num in candidates {
            let Some(candidate) = matrix.col(candidate_num).cloned() else {
                continue;
            };
            if weight.is_some_and(|weights| weights[candidate.col_num] > weights[col.col_num]) {
                continue;
            }
            if (candidate.len() > col.len()
                || (candidate.len() == col.len() && candidate.col_num > col.col_num))
                && col.is_contained_by(&candidate)
            {
                matrix.delete_col(col.col_num);
                break;
            }
        }
    }

    col_count - matrix.ncols()
}

pub fn block_partition(matrix: &SparseMatrix) -> Option<(SparseMatrix, SparseMatrix)> {
    if matrix.nrows() == 0 {
        return None;
    }

    let mut visited_rows = BTreeSet::new();
    let mut visited_cols = BTreeSet::new();
    let first_row = matrix.row_numbers().next()?;
    visit_row(matrix, first_row, &mut visited_rows, &mut visited_cols);

    if visited_rows.len() == matrix.nrows() {
        return None;
    }

    let mut left = SparseMatrix::new();
    let mut right = SparseMatrix::new();
    for row in matrix.rows() {
        if visited_rows.contains(&row.row_num) {
            copy_row_same_number(&mut left, row);
        } else {
            copy_row_same_number(&mut right, row);
        }
    }

    Some((left, right))
}

pub fn maximal_independent_set(matrix: &SparseMatrix, weight: Option<&[i32]>) -> CoverSolution {
    let mut indep = CoverSolution::new();
    let mut intersection = build_intersection_matrix(matrix);

    while intersection.nrows() > 0 {
        let best_row = intersection
            .rows()
            .min_by_key(|row| row.len())
            .expect("non-empty intersection matrix")
            .row_num;

        let least_weight = if let Some(weights) = weight {
            matrix
                .row(best_row)
                .and_then(|row| row.cols().map(|col| weights[col]).min())
                .unwrap_or(1)
        } else {
            1
        };

        indep.cost += least_weight;
        indep.row.insert(best_row);

        let save: Vec<_> = intersection
            .row(best_row)
            .map(|row| row.cols().collect())
            .unwrap_or_default();
        for row_or_col in save {
            intersection.delete_row(row_or_col);
            intersection.delete_col(row_or_col);
        }
    }

    indep
}

fn build_intersection_matrix(matrix: &SparseMatrix) -> SparseMatrix {
    let mut result = SparseMatrix::new();

    for row in matrix.rows() {
        let mut reachable = BTreeSet::new();
        for col_num in row.cols() {
            if let Some(col) = matrix.col(col_num) {
                reachable.extend(col.rows());
            }
        }
        for reachable_row in reachable {
            result.insert(row.row_num, reachable_row);
        }
    }

    result
}

fn gimpel_reduce(
    matrix: &mut SparseMatrix,
    select: &mut CoverSolution,
    weight: Option<&[i32]>,
    lb: i32,
    bound: i32,
    depth: usize,
    stats: &mut MinCoverStats,
) -> Option<Option<CoverSolution>> {
    let mut reduction = None;

    for row in matrix.rows() {
        if row.len() != 2 {
            continue;
        }

        let first_col = row.first_col().expect("two-element row has first column");
        let last_col = row.last_col().expect("two-element row has last column");
        let first = matrix.col(first_col).expect("column exists");
        let last = matrix.col(last_col).expect("column exists");

        if first.len() == 2 {
            let secondary_row = secondary_row(first, row.row_num);
            reduction = Some((first_col, last_col, row.row_num, secondary_row));
            break;
        }

        if last.len() == 2 {
            let secondary_row = secondary_row(last, row.row_num);
            reduction = Some((last_col, first_col, row.row_num, secondary_row));
            break;
        }
    }

    let Some((c1_col_num, c2_col_num, primary_row_num, secondary_row_num)) = reduction else {
        return None;
    };

    let mut save_sec = matrix
        .row(secondary_row_num)
        .expect("secondary row exists")
        .clone();
    save_sec.remove(c1_col_num);

    let c2_rows: Vec<_> = matrix
        .col(c2_col_num)
        .map(|col| col.rows().collect())
        .unwrap_or_default();
    let save_cols: Vec<_> = save_sec.cols().collect();
    for row_num in c2_rows {
        if row_num != primary_row_num {
            for col_num in &save_cols {
                matrix.insert(row_num, *col_num);
            }
        }
    }

    matrix.delete_col(c1_col_num);
    matrix.delete_col(c2_col_num);
    matrix.delete_row(primary_row_num);
    matrix.delete_row(secondary_row_num);

    stats.gimpel_count += 1;
    stats.gimpel += 1;
    let mut best = mincov(matrix, select, weight, lb - 1, bound - 1, depth, stats);
    stats.gimpel -= 1;

    if let Some(best_solution) = &mut best {
        if save_sec.intersects(&best_solution.row) {
            best_solution.add(weight, c2_col_num);
        } else {
            best_solution.add(weight, c1_col_num);
        }
    }

    Some(best)
}

fn secondary_row(col: &SparseCol, primary_row: usize) -> usize {
    let first = col.first_row().expect("Gimpel column has first row");
    if first == primary_row {
        col.last_row().expect("Gimpel column has second row")
    } else {
        first
    }
}

fn verify_cover(matrix: &SparseMatrix, cover: &SparseRow) -> bool {
    matrix.rows().all(|row| row.intersects(cover))
}

fn visit_row(
    matrix: &SparseMatrix,
    row_num: usize,
    visited_rows: &mut BTreeSet<usize>,
    visited_cols: &mut BTreeSet<usize>,
) {
    if !visited_rows.insert(row_num) {
        return;
    }
    if let Some(row) = matrix.row(row_num) {
        for col_num in row.cols() {
            visit_col(matrix, col_num, visited_rows, visited_cols);
        }
    }
}

fn visit_col(
    matrix: &SparseMatrix,
    col_num: usize,
    visited_rows: &mut BTreeSet<usize>,
    visited_cols: &mut BTreeSet<usize>,
) {
    if !visited_cols.insert(col_num) {
        return;
    }
    if let Some(col) = matrix.col(col_num) {
        for row_num in col.rows() {
            visit_row(matrix, row_num, visited_rows, visited_cols);
        }
    }
}

fn copy_row_same_number(matrix: &mut SparseMatrix, row: &SparseRow) {
    for col in row.cols() {
        matrix.insert(row.row_num, col);
    }
}

fn compare_sorted(
    mut first: impl Iterator<Item = usize>,
    mut second: impl Iterator<Item = usize>,
) -> i32 {
    loop {
        match (first.next(), second.next()) {
            (Some(left), Some(right)) if left != right => return left as i32 - right as i32,
            (Some(_), Some(_)) => {}
            (Some(_), None) => return 1,
            (None, Some(_)) => return -1,
            (None, None) => return 0,
        }
    }
}

fn column_weight(weight: Option<&[i32]>, col: usize) -> i32 {
    weight.map_or(1, |weights| weights[col])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix_from_pairs(pairs: &[(usize, usize)]) -> SparseMatrix {
        let mut matrix = SparseMatrix::new();
        for (row, col) in pairs {
            matrix.insert(*row, *col);
        }
        matrix
    }

    #[test]
    fn matrix_insert_remove_keeps_rows_and_columns_consistent() {
        let mut matrix = matrix_from_pairs(&[(2, 4), (0, 3), (2, 1), (0, 3)]);

        assert_eq!(matrix.nrows(), 2);
        assert_eq!(matrix.ncols(), 3);
        assert_eq!(matrix.num_elements(), 3);
        assert_eq!(
            matrix.row(2).unwrap().cols().collect::<Vec<_>>(),
            vec![1, 4]
        );
        assert_eq!(matrix.col(3).unwrap().rows().collect::<Vec<_>>(), vec![0]);

        assert!(matrix.remove(2, 1));
        assert!(!matrix.contains(2, 1));
        assert!(matrix.col(1).is_none());
        assert_eq!(matrix.row(2).unwrap().cols().collect::<Vec<_>>(), vec![4]);

        assert!(matrix.delete_col(4));
        assert!(matrix.row(2).is_none());
    }

    #[test]
    fn row_and_column_set_operations_match_sparse_vectors() {
        let row_a = matrix_from_pairs(&[(0, 1), (0, 3), (0, 5)])
            .row(0)
            .unwrap()
            .clone();
        let row_b = matrix_from_pairs(&[(0, 1), (0, 2), (0, 3), (0, 5)])
            .row(0)
            .unwrap()
            .clone();

        assert!(row_a.is_contained_by(&row_b));
        assert!(row_a.intersects(&row_b));
        assert_eq!(
            row_a.intersection(&row_b).cols().collect::<Vec<_>>(),
            vec![1, 3, 5]
        );
        assert!(row_a.compare_lexicographic(&row_b) > 0);

        let matrix = matrix_from_pairs(&[(1, 7), (3, 7), (1, 9)]);
        let col_7 = matrix.col(7).unwrap();
        let col_9 = matrix.col(9).unwrap();
        assert!(col_9.is_contained_by(col_7));
        assert!(col_7.compare_lexicographic(col_9) > 0);
    }

    #[test]
    fn dominance_removes_superset_rows_and_more_expensive_columns() {
        let mut rows = matrix_from_pairs(&[(0, 1), (1, 1), (1, 2), (2, 3)]);
        assert_eq!(row_dominance(&mut rows), 1);
        assert!(rows.row(1).is_none());

        let mut cols = matrix_from_pairs(&[(0, 1), (1, 1), (0, 2)]);
        let weights = vec![0, 1, 2];
        assert_eq!(col_dominance(&mut cols, Some(&weights)), 1);
        assert!(cols.col(2).is_none());
        assert!(cols.col(1).is_some());
    }

    #[test]
    fn block_partition_splits_disconnected_components() {
        let matrix = matrix_from_pairs(&[(0, 0), (1, 0), (3, 4), (4, 5)]);
        let (left, right) = block_partition(&matrix).expect("partition exists");

        assert_eq!(left.to_pairs(), vec![(0, 0), (1, 0)]);
        assert_eq!(right.to_pairs(), vec![(3, 4), (4, 5)]);
    }

    #[test]
    fn maximum_independent_set_returns_disjoint_row_lower_bound() {
        let matrix = matrix_from_pairs(&[(0, 0), (1, 0), (1, 1), (2, 2)]);
        let indep = maximal_independent_set(&matrix, None);

        assert_eq!(indep.cost, 2);
        assert_eq!(indep.row.cols().collect::<Vec<_>>(), vec![0, 2]);
    }

    #[test]
    fn minimum_cover_finds_exact_unweighted_solution() {
        let matrix = matrix_from_pairs(&[(0, 0), (0, 1), (1, 1), (2, 0), (2, 2)]);
        let cover = minimum_cover(&matrix, None, false);

        assert_eq!(cover.cols().collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn minimum_cover_respects_weights() {
        let matrix = matrix_from_pairs(&[(0, 0), (0, 1), (1, 1), (2, 0), (2, 2)]);
        let weights = vec![10, 1, 1];
        let cover = minimum_cover(&matrix, Some(&weights), false);

        assert_eq!(cover.cols().collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn do_sm_minimum_cover_maps_set_family_rows_to_sparse_matrix_rows() {
        let sets = [
            matrix_from_pairs(&[(0, 0), (0, 1)]).row(0).unwrap().clone(),
            matrix_from_pairs(&[(0, 1), (0, 2)]).row(0).unwrap().clone(),
            matrix_from_pairs(&[(0, 2), (0, 3)]).row(0).unwrap().clone(),
        ];

        let cover = do_sm_minimum_cover(&sets);

        assert_eq!(cover.cols().collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn do_sm_minimum_cover_returns_empty_cover_for_empty_family() {
        let sets: [SparseRow; 0] = [];

        let cover = do_sm_minimum_cover(&sets);

        assert!(cover.is_empty());
    }

    #[test]
    fn gimpel_reduction_case_matches_cover_expectation() {
        let matrix = matrix_from_pairs(&[(0, 1), (0, 2), (1, 1), (1, 3), (2, 2), (2, 4)]);
        let cover = minimum_cover(&matrix, None, false);

        assert_eq!(cover.len(), 2);
        assert!(verify_cover(&matrix, &cover));
    }
}
