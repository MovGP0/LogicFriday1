//! Native Rust model for `LogicSynthesis/sis/simplify/dc_filter.c`.
//!
//! The C file filters a don't-care cover by converting the on-set and
//! don't-care set into a sparse matrix, deleting selected don't-care rows, and
//! converting the matrix back into a SIS `node_t`. This module ports the
//! deterministic row-filtering behavior onto an owned Rust matrix. Direct SIS
//! `node_t`, `sm_matrix`, Espresso set-family, and ST-table integration remains
//! an explicit missing-dependency error until those native ports are available.

use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const SIZE_BOUND: usize = 3;
pub const DIST_BOUND: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead: &'static str,
    pub c_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.2",
        c_file: "LogicSynthesis/sis/array/array.c",
        reason: "sm_col_count_init uses array allocation, fetch, insert, and free",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.165",
        c_file: "LogicSynthesis/sis/espresso/set.c",
        reason: "node_d1merge/node_d2merge depend on Espresso cover merge semantics",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.318",
        c_file: "LogicSynthesis/sis/node/node.c",
        reason: "node_t constants, literals, AND construction, metrics, and node_free",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.321",
        c_file: "LogicSynthesis/sis/node/nodemisc.c",
        reason: "node_scc and node_minimum_base after repeated distance-one merge",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.453",
        c_file: "LogicSynthesis/sis/simplify/simp_sm.c",
        reason: "simp_node_to_sm and simp_sm_to_node conversion boundary",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.454",
        c_file: "LogicSynthesis/sis/simplify/filter_util.c",
        reason: "canonical sparse-matrix filter helper ownership",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.456",
        c_file: "LogicSynthesis/sis/sparse/cols.c",
        reason: "native sm_col traversal, flags, and column lengths",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.457",
        c_file: "LogicSynthesis/sis/sparse/matrix.c",
        reason: "native sm_matrix row/column deletion and lookup semantics",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.458",
        c_file: "LogicSynthesis/sis/sparse/rows.c",
        reason: "native sm_row metadata, flags, lengths, and traversal order",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.485",
        c_file: "LogicSynthesis/sis/st/st.c",
        reason: "simp_obsdc_filter consumes an ST table of allowed variables",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DcFilter {
    None,
    Exact,
    DisjointSupport,
    Size,
    FirstDistance,
    SecondDistance,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowKind {
    OnSet,
    DontCare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralValue {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterMatrix {
    rows: BTreeMap<usize, RowKind>,
    elements: BTreeMap<(usize, usize), LiteralValue>,
    rows_size: usize,
    cols_size: usize,
}

impl Default for FilterMatrix {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterMatrix {
    pub fn new() -> Self {
        Self {
            rows: BTreeMap::new(),
            elements: BTreeMap::new(),
            rows_size: 0,
            cols_size: 0,
        }
    }

    pub fn insert_row(&mut self, row: usize, kind: RowKind) {
        self.rows.insert(row, kind);
        self.rows_size = self.rows_size.max(row + 1);
    }

    pub fn insert_literal(&mut self, row: usize, col: usize, value: LiteralValue, kind: RowKind) {
        self.insert_row(row, kind);
        self.elements.insert((row, col), value);
        self.cols_size = self.cols_size.max(col + 1);
    }

    pub fn row_kind(&self, row: usize) -> Option<RowKind> {
        self.rows.get(&row).copied()
    }

    pub fn literal(&self, row: usize, col: usize) -> Option<LiteralValue> {
        self.elements.get(&(row, col)).copied()
    }

    pub fn rows_size(&self) -> usize {
        self.rows_size
    }

    pub fn cols_size(&self) -> usize {
        self.cols_size
    }

    pub fn nrows(&self) -> usize {
        self.rows.len()
    }

    pub fn row_length(&self, row: usize) -> usize {
        self.elements.keys().filter(|(r, _)| *r == row).count()
    }

    pub fn active_rows(&self) -> Vec<usize> {
        self.rows.keys().copied().collect()
    }

    pub fn active_columns(&self) -> Vec<usize> {
        self.elements
            .keys()
            .map(|(_, col)| *col)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn columns_for_row(&self, row: usize) -> Vec<usize> {
        self.elements
            .keys()
            .filter_map(|(r, col)| (*r == row).then_some(*col))
            .collect()
    }

    pub fn rows_for_column(&self, col: usize) -> Vec<usize> {
        self.elements
            .keys()
            .filter_map(|(row, c)| (*c == col).then_some(*row))
            .collect()
    }

    pub fn delete_row(&mut self, row: usize) -> bool {
        let removed = self.rows.remove(&row).is_some();
        if removed {
            self.elements.retain(|(r, _), _| *r != row);
        }
        removed
    }

    pub fn dont_care_rows(&self) -> Vec<usize> {
        self.rows
            .iter()
            .filter_map(|(row, kind)| (*kind == RowKind::DontCare).then_some(*row))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DcFilterError {
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for DcFilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS C-file dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for DcFilterError {}

pub fn required_port_beads() -> &'static [PortDependency] {
    REQUIRED_PORT_BEADS
}

pub fn simp_dc_filter_in_sis_network() -> Result<(), DcFilterError> {
    Err(DcFilterError::MissingSisPorts {
        operation: "simp_dc_filter",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn simp_obsdc_filter_in_sis_network() -> Result<(), DcFilterError> {
    Err(DcFilterError::MissingSisPorts {
        operation: "simp_obsdc_filter",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn simp_obssatdc_filter_in_sis_network() -> Result<(), DcFilterError> {
    Err(DcFilterError::MissingSisPorts {
        operation: "simp_obssatdc_filter",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn node_d2merge_in_sis_network() -> Result<(), DcFilterError> {
    Err(DcFilterError::MissingSisPorts {
        operation: "node_d2merge",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn simp_dc_filter_matrix(mut matrix: FilterMatrix, filter: DcFilter) -> FilterMatrix {
    match filter {
        DcFilter::Exact => filter_exact(&mut matrix),
        DcFilter::DisjointSupport => {
            filter_exact(&mut matrix);
            filter_disjoint_support(&mut matrix);
        }
        DcFilter::Size => {
            filter_exact(&mut matrix);
            filter_size(&mut matrix);
        }
        DcFilter::FirstDistance => {
            filter_exact(&mut matrix);
            filter_first_distance(&mut matrix, DIST_BOUND);
        }
        DcFilter::SecondDistance => {
            filter_exact(&mut matrix);
            filter_second_distance(&mut matrix, DIST_BOUND);
        }
        DcFilter::All => {
            filter_exact(&mut matrix);
            filter_disjoint_support(&mut matrix);
            filter_size(&mut matrix);
        }
        DcFilter::None => {}
    }
    matrix
}

pub fn filter_exact(matrix: &mut FilterMatrix) {
    let mut chosen = HashSet::new();
    while let Some(col) = longest_unchosen_column(matrix, &chosen) {
        chosen.insert(col);
        filter_exact_excluding_column(matrix, col);
    }
}

pub fn filter_disjoint_support(matrix: &mut FilterMatrix) {
    if matrix.nrows() == 0 {
        return;
    }

    let on_set_support = on_set_columns(matrix);
    let rows_to_delete = matrix
        .active_rows()
        .into_iter()
        .filter(|row| {
            matrix.row_kind(*row) == Some(RowKind::DontCare)
                && matrix
                    .columns_for_row(*row)
                    .into_iter()
                    .all(|col| !on_set_support.contains(&col))
        })
        .collect::<Vec<_>>();

    for row in rows_to_delete {
        matrix.delete_row(row);
    }
}

pub fn filter_size(matrix: &mut FilterMatrix) {
    if matrix.nrows() == 0 {
        return;
    }

    let on_set_support = on_set_columns(matrix);
    let rows_to_delete = matrix
        .active_rows()
        .into_iter()
        .filter(|row| {
            matrix.row_kind(*row) == Some(RowKind::DontCare)
                && matrix.row_length(*row) > SIZE_BOUND
                && matrix
                    .columns_for_row(*row)
                    .into_iter()
                    .any(|col| !on_set_support.contains(&col))
        })
        .collect::<Vec<_>>();

    for row in rows_to_delete {
        matrix.delete_row(row);
    }
}

pub fn filter_first_distance(matrix: &mut FilterMatrix, distance: usize) {
    if matrix.nrows() == 0 {
        return;
    }

    let on_set_support = on_set_columns(matrix);
    let rows_to_delete = matrix
        .active_rows()
        .into_iter()
        .filter(|row| {
            matrix.row_kind(*row) == Some(RowKind::DontCare)
                && outside_support_distance(matrix, *row, &on_set_support) > distance
        })
        .collect::<Vec<_>>();

    for row in rows_to_delete {
        matrix.delete_row(row);
    }
}

pub fn filter_second_distance(matrix: &mut FilterMatrix, distance: usize) {
    if matrix.nrows() == 0 {
        return;
    }

    let on_set_support = on_set_columns(matrix);
    let on_rows = matrix
        .active_rows()
        .into_iter()
        .filter(|row| matrix.row_kind(*row) == Some(RowKind::OnSet))
        .collect::<Vec<_>>();
    let rows_to_delete = matrix
        .active_rows()
        .into_iter()
        .filter(|row| {
            matrix.row_kind(*row) == Some(RowKind::DontCare)
                && !on_rows
                    .iter()
                    .any(|on_row| dist_check(matrix, *on_row, *row, distance, &on_set_support))
        })
        .collect::<Vec<_>>();

    for row in rows_to_delete {
        matrix.delete_row(row);
    }
}

pub fn filter_observability_support(matrix: &mut FilterMatrix, variable_allowance: usize) {
    if matrix.nrows() == 0 {
        return;
    }

    let on_set_support = on_set_columns(matrix);
    let rows_to_delete = matrix
        .active_rows()
        .into_iter()
        .filter(|row| {
            matrix.row_kind(*row) == Some(RowKind::DontCare)
                && matrix
                    .columns_for_row(*row)
                    .into_iter()
                    .filter(|col| !on_set_support.contains(col))
                    .count()
                    > variable_allowance
        })
        .collect::<Vec<_>>();

    for row in rows_to_delete {
        matrix.delete_row(row);
    }
}

fn filter_exact_excluding_column(matrix: &mut FilterMatrix, excluded_col: usize) {
    if matrix.nrows() == 0 {
        return;
    }

    let mut marked_rows = matrix
        .active_rows()
        .into_iter()
        .filter(|row| matrix.row_kind(*row) == Some(RowKind::OnSet))
        .collect::<HashSet<_>>();
    let mut marked_cols = HashSet::new();
    let mut queue = marked_rows.iter().copied().collect::<VecDeque<_>>();

    while let Some(row) = queue.pop_front() {
        for col in matrix.columns_for_row(row) {
            if col == excluded_col || !marked_cols.insert(col) {
                continue;
            }
            for next_row in matrix.rows_for_column(col) {
                if marked_rows.insert(next_row) {
                    queue.push_back(next_row);
                }
            }
        }
    }

    let rows_to_delete = matrix
        .active_rows()
        .into_iter()
        .filter(|row| !marked_rows.contains(row))
        .collect::<Vec<_>>();
    for row in rows_to_delete {
        matrix.delete_row(row);
    }
}

fn longest_unchosen_column(matrix: &FilterMatrix, chosen: &HashSet<usize>) -> Option<usize> {
    matrix
        .active_columns()
        .into_iter()
        .filter(|col| !chosen.contains(col))
        .max_by_key(|col| (matrix.rows_for_column(*col).len(), std::cmp::Reverse(*col)))
}

fn on_set_columns(matrix: &FilterMatrix) -> HashSet<usize> {
    matrix
        .active_rows()
        .into_iter()
        .filter(|row| matrix.row_kind(*row) == Some(RowKind::OnSet))
        .flat_map(|row| matrix.columns_for_row(row))
        .collect()
}

fn outside_support_distance(
    matrix: &FilterMatrix,
    row: usize,
    on_set_support: &HashSet<usize>,
) -> usize {
    matrix
        .columns_for_row(row)
        .into_iter()
        .filter(|col| !on_set_support.contains(col))
        .filter(|col| matrix.literal(row, *col) != Some(LiteralValue::DontCare))
        .count()
}

fn dist_check(
    matrix: &FilterMatrix,
    on_row: usize,
    dc_row: usize,
    distance: usize,
    on_set_support: &HashSet<usize>,
) -> bool {
    let cols = matrix
        .columns_for_row(on_row)
        .into_iter()
        .chain(matrix.columns_for_row(dc_row))
        .collect::<BTreeSet<_>>();
    let mut cdist = 0usize;

    for col in cols {
        let on_value = matrix.literal(on_row, col);
        let dc_value = matrix.literal(dc_row, col);
        let contributes = match (on_value, dc_value) {
            (_, None) => false,
            (None, Some(LiteralValue::DontCare)) => false,
            (None, Some(_)) => !on_set_support.contains(&col),
            (Some(LiteralValue::DontCare), Some(LiteralValue::DontCare)) => false,
            (Some(LiteralValue::DontCare), Some(_)) => !on_set_support.contains(&col),
            (Some(_), Some(LiteralValue::DontCare)) => false,
            (Some(left), Some(right)) if left == right => false,
            (Some(_), Some(_)) => true,
        };
        if contributes {
            cdist += 1;
            if cdist > distance {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_matrix() -> FilterMatrix {
        let mut matrix = FilterMatrix::new();
        matrix.insert_literal(0, 0, LiteralValue::One, RowKind::OnSet);
        matrix.insert_literal(0, 1, LiteralValue::Zero, RowKind::OnSet);
        matrix.insert_literal(0, 4, LiteralValue::One, RowKind::OnSet);
        matrix.insert_literal(1, 0, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 2, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 4, LiteralValue::Zero, RowKind::DontCare);
        matrix.insert_literal(2, 3, LiteralValue::One, RowKind::DontCare);
        matrix
    }

    #[test]
    fn exact_filter_keeps_rows_connected_to_on_set_without_excluded_columns() {
        let mut matrix = FilterMatrix::new();
        matrix.insert_literal(0, 0, LiteralValue::One, RowKind::OnSet);
        matrix.insert_literal(0, 2, LiteralValue::One, RowKind::OnSet);
        matrix.insert_literal(1, 0, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 1, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 2, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 1, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 2, LiteralValue::Zero, RowKind::DontCare);

        let filtered = simp_dc_filter_matrix(matrix, DcFilter::Exact);

        assert_eq!(filtered.row_kind(0), Some(RowKind::OnSet));
        assert_eq!(filtered.row_kind(1), Some(RowKind::DontCare));
        assert_eq!(filtered.row_kind(2), Some(RowKind::DontCare));
    }

    #[test]
    fn disjoint_support_filter_deletes_dc_rows_sharing_no_on_set_columns() {
        let mut filtered = sample_matrix();
        filter_disjoint_support(&mut filtered);

        assert_eq!(filtered.row_kind(1), Some(RowKind::DontCare));
        assert_eq!(filtered.row_kind(2), None);
    }

    #[test]
    fn size_filter_deletes_large_dc_rows_with_support_outside_on_set() {
        let mut matrix = FilterMatrix::new();
        matrix.insert_literal(0, 0, LiteralValue::One, RowKind::OnSet);
        matrix.insert_literal(1, 0, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 1, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 2, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 3, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 1, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 2, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 3, LiteralValue::One, RowKind::DontCare);

        filter_size(&mut matrix);

        assert_eq!(matrix.row_kind(1), None);
        assert_eq!(matrix.row_kind(2), Some(RowKind::DontCare));
    }

    #[test]
    fn first_distance_filter_counts_literals_outside_on_set_support() {
        let mut matrix = FilterMatrix::new();
        matrix.insert_literal(0, 0, LiteralValue::One, RowKind::OnSet);
        matrix.insert_literal(1, 1, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 2, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 1, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 2, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 3, LiteralValue::One, RowKind::DontCare);

        filter_first_distance(&mut matrix, 2);

        assert_eq!(matrix.row_kind(1), Some(RowKind::DontCare));
        assert_eq!(matrix.row_kind(2), None);
    }

    #[test]
    fn second_distance_filter_uses_distance_from_each_on_set_cube() {
        let mut matrix = FilterMatrix::new();
        matrix.insert_literal(0, 0, LiteralValue::One, RowKind::OnSet);
        matrix.insert_literal(0, 1, LiteralValue::Zero, RowKind::OnSet);
        matrix.insert_literal(1, 0, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 1, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 0, LiteralValue::Zero, RowKind::DontCare);
        matrix.insert_literal(2, 1, LiteralValue::One, RowKind::DontCare);

        filter_second_distance(&mut matrix, 1);

        assert_eq!(matrix.row_kind(1), Some(RowKind::DontCare));
        assert_eq!(matrix.row_kind(2), None);
    }

    #[test]
    fn all_filter_runs_exact_disjoint_support_and_size_in_order() {
        let filtered = simp_dc_filter_matrix(sample_matrix(), DcFilter::All);

        assert_eq!(filtered.row_kind(1), Some(RowKind::DontCare));
        assert_eq!(filtered.row_kind(2), None);
    }

    #[test]
    fn observability_filter_deletes_rows_over_variable_allowance() {
        let mut matrix = FilterMatrix::new();
        matrix.insert_literal(0, 0, LiteralValue::One, RowKind::OnSet);
        matrix.insert_literal(1, 0, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(1, 1, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 1, LiteralValue::One, RowKind::DontCare);
        matrix.insert_literal(2, 2, LiteralValue::Zero, RowKind::DontCare);

        filter_observability_support(&mut matrix, 1);

        assert_eq!(matrix.row_kind(1), Some(RowKind::DontCare));
        assert_eq!(matrix.row_kind(2), None);
    }

    #[test]
    fn sis_bound_entries_report_dependency_beads_and_sources() {
        assert!(required_port_beads().iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.453"
                && dependency.c_file == "LogicSynthesis/sis/simplify/simp_sm.c"
        }));
        assert!(required_port_beads().iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.454"
                && dependency.c_file == "LogicSynthesis/sis/simplify/filter_util.c"
        }));
        assert!(required_port_beads().iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.318"
                && dependency.c_file == "LogicSynthesis/sis/node/node.c"
        }));

        assert_eq!(
            simp_dc_filter_in_sis_network(),
            Err(DcFilterError::MissingSisPorts {
                operation: "simp_dc_filter",
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
        assert_eq!(
            node_d2merge_in_sis_network(),
            Err(DcFilterError::MissingSisPorts {
                operation: "node_d2merge",
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
    }
}
