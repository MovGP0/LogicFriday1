//! Fast extraction over owned sparse cube rows.
//!
//! This module ports the extract loop and divisor accounting to native Rust
//! data structures. The legacy implementation also mutates SIS network nodes
//! while rewriting the sparse matrix; that node/network side effect is exposed
//! here as an explicit operation error until native extraction orchestration can
//! provide a Rust network backend.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DivisorKind {
    D112,
    D222,
    D223,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeightStatus {
    New,
    Changed,
    Old,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeRow {
    node_index: i32,
    columns: BTreeSet<usize>,
}

impl CubeRow {
    pub fn new(node_index: i32, columns: impl IntoIterator<Item = usize>) -> Self {
        Self {
            node_index,
            columns: columns.into_iter().collect(),
        }
    }

    pub fn node_index(&self) -> i32 {
        self.node_index
    }

    pub fn columns(&self) -> &BTreeSet<usize> {
        &self.columns
    }

    pub fn contains(&self, column: usize) -> bool {
        self.columns.contains(&column)
    }

    fn remove_columns(&mut self, columns: &BTreeSet<usize>) {
        for column in columns {
            self.columns.remove(column);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractionMatrix {
    rows: Vec<CubeRow>,
    next_node_index: i32,
}

impl ExtractionMatrix {
    pub fn new(rows: impl Into<Vec<CubeRow>>) -> Self {
        let rows = rows.into();
        let next_node_index = rows
            .iter()
            .map(CubeRow::node_index)
            .max()
            .map_or(0, |index| index + 1);

        Self {
            rows,
            next_node_index,
        }
    }

    pub fn rows(&self) -> &[CubeRow] {
        &self.rows
    }

    pub fn push_row(&mut self, row: CubeRow) {
        self.next_node_index = self.next_node_index.max(row.node_index() + 1);
        self.rows.push(row);
    }

    pub fn allocate_node_index(&mut self) -> i32 {
        let index = self.next_node_index;
        self.next_node_index += 1;
        index
    }

    pub fn rows_with_columns(&self, columns: &BTreeSet<usize>) -> BTreeSet<usize> {
        self.rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| columns.is_subset(row.columns()).then_some(index))
            .collect()
    }

    pub fn rows_with_column_pair(&self, col1: usize, col2: usize) -> BTreeSet<usize> {
        self.rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (row.contains(col1) && row.contains(col2)).then_some(index))
            .collect()
    }

    fn row_mut(&mut self, index: usize) -> Option<&mut CubeRow> {
        self.rows.get_mut(index)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DivisorOccurrence {
    pub cube1_row: usize,
    pub cube2_row: usize,
    pub target_node_index: i32,
    pub phase: usize,
    pub base_length: usize,
}

impl DivisorOccurrence {
    pub fn new(cube1_row: usize, cube2_row: usize, target_node_index: i32) -> Self {
        Self {
            cube1_row,
            cube2_row,
            target_node_index,
            phase: 0,
            base_length: 0,
        }
    }

    pub fn with_phase(mut self, phase: usize) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_base_length(mut self, base_length: usize) -> Self {
        self.base_length = base_length;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoubleCubeDivisor {
    cube1: BTreeSet<usize>,
    cube2: BTreeSet<usize>,
    occurrences: Vec<DivisorOccurrence>,
    kind: DivisorKind,
    weight_status: WeightStatus,
    cached_weight: i32,
}

impl DoubleCubeDivisor {
    pub fn new(
        cube1: impl IntoIterator<Item = usize>,
        cube2: impl IntoIterator<Item = usize>,
        occurrences: impl Into<Vec<DivisorOccurrence>>,
        kind: DivisorKind,
    ) -> Self {
        let mut cube1 = cube1.into_iter().collect::<BTreeSet<_>>();
        let mut cube2 = cube2.into_iter().collect::<BTreeSet<_>>();
        if cube1.len() > cube2.len() {
            std::mem::swap(&mut cube1, &mut cube2);
        }

        Self {
            cube1,
            cube2,
            occurrences: occurrences.into(),
            kind,
            weight_status: WeightStatus::New,
            cached_weight: 0,
        }
    }

    pub fn cube1(&self) -> &BTreeSet<usize> {
        &self.cube1
    }

    pub fn cube2(&self) -> &BTreeSet<usize> {
        &self.cube2
    }

    pub fn occurrences(&self) -> &[DivisorOccurrence] {
        &self.occurrences
    }

    pub fn kind(&self) -> DivisorKind {
        self.kind
    }

    pub fn cached_weight(&self) -> i32 {
        self.cached_weight
    }

    pub fn compute_weight(&mut self, matrix: &ExtractionMatrix) -> i32 {
        let mut weight = if self.kind == DivisorKind::D112 {
            calc_d112_complement_weight(matrix, self)
        } else {
            0
        };

        if self.weight_status != WeightStatus::Old {
            let literal_width = self.cube1.len() + self.cube2.len();
            let shared = self
                .occurrences
                .iter()
                .map(|occurrence| occurrence.base_length)
                .sum::<usize>();
            let occurrence_count = self.occurrences.len();

            self.cached_weight = ((occurrence_count.saturating_sub(1) * literal_width) + shared)
                as i32
                - occurrence_count as i32;
            self.weight_status = WeightStatus::Old;
        }

        weight += self.cached_weight;
        weight
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SingleCubeDivisor {
    pub col1: usize,
    pub col2: usize,
    pub rows: BTreeSet<usize>,
}

impl SingleCubeDivisor {
    pub fn coin(&self) -> i32 {
        self.rows.len() as i32
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtractionStep {
    DoubleCube {
        new_node_index: i32,
        new_column: usize,
        changed_rows: Vec<usize>,
    },
    SingleCube {
        new_node_index: i32,
        new_column: usize,
        changed_rows: Vec<usize>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractionReport {
    pub steps: Vec<ExtractionStep>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FastExtractError {
    MissingNetworkBackend { operation: &'static str },
    InvalidRow { row: usize },
    InvalidPhase { phase: usize },
}

impl fmt::Display for FastExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNetworkBackend { operation } => {
                write!(
                    f,
                    "{operation} requires a native extraction network backend"
                )
            }
            Self::InvalidRow { row } => write!(f, "invalid extraction matrix row {row}"),
            Self::InvalidPhase { phase } => write!(f, "invalid divisor phase {phase}"),
        }
    }
}

impl Error for FastExtractError {}

pub type FastExtractResult<T> = Result<T, FastExtractError>;

pub fn fast_extract(
    matrix: &mut ExtractionMatrix,
    divisors: &mut Vec<DoubleCubeDivisor>,
) -> FastExtractResult<ExtractionReport> {
    let mut steps = Vec::new();

    loop {
        let best_double = choose_best_double_cube_divisor(matrix, divisors);
        let best_single =
            extract_single_cube_divisor(matrix, best_double.map_or(-1, |(_, weight)| weight));

        match (best_double, best_single) {
            (None, None) => break,
            (Some((index, weight)), None) if weight >= 0 => {
                let step = apply_double_cube_divisor(matrix, &divisors[index])?;
                steps.push(step);
                divisors.remove(index);
            }
            (_, Some(single)) if single.coin() >= 2 => {
                steps.push(apply_single_cube_divisor(matrix, &single)?);
            }
            _ => break,
        }
    }

    Ok(ExtractionReport { steps })
}

pub fn fast_extract_sis_network_blocked() -> FastExtractResult<()> {
    Err(FastExtractError::MissingNetworkBackend {
        operation: "fast extraction over SIS network nodes",
    })
}

pub fn choose_best_double_cube_divisor(
    matrix: &ExtractionMatrix,
    divisors: &mut Vec<DoubleCubeDivisor>,
) -> Option<(usize, i32)> {
    let mut best = None;
    let mut index = 0;

    while index < divisors.len() {
        if divisors[index].occurrences().is_empty() {
            divisors.remove(index);
            continue;
        }

        let weight = divisors[index].compute_weight(matrix);
        if weight < 0 {
            divisors.remove(index);
            continue;
        }

        if best.is_none_or(|(_, best_weight)| best_weight < weight) {
            best = Some((index, weight));
        }

        index += 1;
    }

    best
}

pub fn extract_single_cube_divisor(
    matrix: &ExtractionMatrix,
    double_cube_weight: i32,
) -> Option<SingleCubeDivisor> {
    let mut columns_to_rows = BTreeMap::<usize, BTreeSet<usize>>::new();
    for (row_index, row) in matrix.rows().iter().enumerate() {
        for column in row.columns() {
            columns_to_rows
                .entry(*column)
                .or_default()
                .insert(row_index);
        }
    }

    let columns = columns_to_rows.keys().copied().collect::<Vec<_>>();
    let mut best = None;
    for (left_index, col1) in columns.iter().enumerate() {
        for col2 in columns.iter().skip(left_index + 1) {
            let rows = columns_to_rows[col1]
                .intersection(&columns_to_rows[col2])
                .copied()
                .collect::<BTreeSet<_>>();
            let coin = rows.len() as i32;

            if coin < 2 || coin - 2 < double_cube_weight {
                continue;
            }

            if best
                .as_ref()
                .is_none_or(|candidate: &SingleCubeDivisor| candidate.coin() < coin)
            {
                best = Some(SingleCubeDivisor {
                    col1: *col1,
                    col2: *col2,
                    rows,
                });
            }
        }
    }

    best
}

pub fn compute_divisor_weight(matrix: &ExtractionMatrix, divisor: &mut DoubleCubeDivisor) -> i32 {
    divisor.compute_weight(matrix)
}

fn apply_double_cube_divisor(
    matrix: &mut ExtractionMatrix,
    divisor: &DoubleCubeDivisor,
) -> FastExtractResult<ExtractionStep> {
    let new_node_index = matrix.allocate_node_index();
    let new_column = (new_node_index as usize) * 2;
    let mut changed_rows = Vec::new();

    matrix.push_row(CubeRow::new(
        new_node_index,
        divisor.cube1().iter().copied(),
    ));
    matrix.push_row(CubeRow::new(
        new_node_index,
        divisor.cube2().iter().copied(),
    ));

    for occurrence in divisor.occurrences() {
        if occurrence.phase > 1 {
            return Err(FastExtractError::InvalidPhase {
                phase: occurrence.phase,
            });
        }

        let cube1 = matrix
            .rows()
            .get(occurrence.cube1_row)
            .ok_or(FastExtractError::InvalidRow {
                row: occurrence.cube1_row,
            })?
            .columns()
            .clone();
        let cube2 = matrix
            .rows()
            .get(occurrence.cube2_row)
            .ok_or(FastExtractError::InvalidRow {
                row: occurrence.cube2_row,
            })?
            .columns()
            .clone();
        let base = cube1.intersection(&cube2).copied().collect::<BTreeSet<_>>();
        let target_row = matrix.rows.len();
        let mut row = CubeRow::new(occurrence.target_node_index, base);
        row.columns.insert(new_column + occurrence.phase);
        matrix.push_row(row);
        changed_rows.push(target_row);
    }

    Ok(ExtractionStep::DoubleCube {
        new_node_index,
        new_column,
        changed_rows,
    })
}

fn apply_single_cube_divisor(
    matrix: &mut ExtractionMatrix,
    divisor: &SingleCubeDivisor,
) -> FastExtractResult<ExtractionStep> {
    let new_node_index = matrix.allocate_node_index();
    let new_column = (new_node_index as usize) * 2;
    let extracted_columns = BTreeSet::from([divisor.col1, divisor.col2]);
    let mut changed_rows = Vec::new();

    matrix.push_row(CubeRow::new(
        new_node_index,
        extracted_columns.iter().copied(),
    ));
    for row_index in &divisor.rows {
        let row = matrix
            .row_mut(*row_index)
            .ok_or(FastExtractError::InvalidRow { row: *row_index })?;
        row.remove_columns(&extracted_columns);
        row.columns.insert(new_column);
        changed_rows.push(*row_index);
    }

    Ok(ExtractionStep::SingleCube {
        new_node_index,
        new_column,
        changed_rows,
    })
}

fn calc_d112_complement_weight(matrix: &ExtractionMatrix, divisor: &DoubleCubeDivisor) -> i32 {
    let Some(col1) = divisor.cube1().iter().next().copied() else {
        return 0;
    };
    let Some(col2) = divisor.cube2().iter().next().copied() else {
        return 0;
    };

    matrix
        .rows_with_column_pair(complement_column(col1), complement_column(col2))
        .len() as i32
}

fn complement_column(column: usize) -> usize {
    column ^ 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divisor_weight_matches_legacy_formula_for_non_d112() {
        let matrix = sample_matrix();
        let mut divisor = DoubleCubeDivisor::new(
            [0, 2],
            [4],
            vec![
                DivisorOccurrence::new(0, 1, 10).with_base_length(1),
                DivisorOccurrence::new(2, 3, 11).with_base_length(2),
            ],
            DivisorKind::Other,
        );

        assert_eq!(compute_divisor_weight(&matrix, &mut divisor), 4);
        assert_eq!(divisor.cached_weight(), 4);
    }

    #[test]
    fn d112_weight_includes_complement_rectangle_rows() {
        let matrix = sample_matrix();
        let mut divisor = DoubleCubeDivisor::new(
            [0],
            [2],
            vec![DivisorOccurrence::new(0, 1, 10).with_base_length(0)],
            DivisorKind::D112,
        );

        assert_eq!(compute_divisor_weight(&matrix, &mut divisor), 1);
    }

    #[test]
    fn choosing_best_divisor_discards_empty_and_negative_candidates() {
        let matrix = sample_matrix();
        let mut divisors = vec![
            DoubleCubeDivisor::new([0], [2], Vec::new(), DivisorKind::Other),
            DoubleCubeDivisor::new(
                [0],
                [2],
                vec![DivisorOccurrence::new(0, 1, 10)],
                DivisorKind::Other,
            ),
            DoubleCubeDivisor::new(
                [0, 2],
                [4],
                vec![
                    DivisorOccurrence::new(0, 1, 10).with_base_length(1),
                    DivisorOccurrence::new(2, 3, 11).with_base_length(2),
                ],
                DivisorKind::Other,
            ),
        ];

        assert_eq!(
            choose_best_double_cube_divisor(&matrix, &mut divisors),
            Some((0, 4))
        );
        assert_eq!(divisors.len(), 1);
    }

    #[test]
    fn single_cube_selection_requires_two_rows_and_beats_double_weight() {
        let matrix = sample_matrix();

        assert_eq!(
            extract_single_cube_divisor(&matrix, -1),
            Some(SingleCubeDivisor {
                col1: 0,
                col2: 2,
                rows: BTreeSet::from([2, 3]),
            })
        );
        assert_eq!(extract_single_cube_divisor(&matrix, 2), None);
    }

    #[test]
    fn applying_single_cube_rewrites_common_rows_and_adds_factor_row() {
        let mut matrix = sample_matrix();
        let single = SingleCubeDivisor {
            col1: 1,
            col2: 3,
            rows: BTreeSet::from([0, 1]),
        };

        let step = apply_single_cube_divisor(&mut matrix, &single).unwrap();

        assert_eq!(
            step,
            ExtractionStep::SingleCube {
                new_node_index: 4,
                new_column: 8,
                changed_rows: vec![0, 1],
            }
        );
        assert_eq!(matrix.rows()[0].columns(), &BTreeSet::from([8]));
        assert_eq!(matrix.rows()[1].columns(), &BTreeSet::from([5, 8]));
        assert_eq!(matrix.rows()[4], CubeRow::new(4, [1, 3]));
    }

    #[test]
    fn applying_double_cube_adds_factor_rows_and_base_rows() {
        let mut matrix = sample_matrix();
        let divisor = DoubleCubeDivisor::new(
            [1, 3],
            [3, 5],
            vec![DivisorOccurrence::new(0, 1, 1).with_phase(1)],
            DivisorKind::D222,
        );

        let step = apply_double_cube_divisor(&mut matrix, &divisor).unwrap();

        assert_eq!(
            step,
            ExtractionStep::DoubleCube {
                new_node_index: 4,
                new_column: 8,
                changed_rows: vec![6],
            }
        );
        assert_eq!(matrix.rows()[4], CubeRow::new(4, [1, 3]));
        assert_eq!(matrix.rows()[5], CubeRow::new(4, [3, 5]));
        assert_eq!(matrix.rows()[6], CubeRow::new(1, [1, 3, 9]));
    }

    #[test]
    fn fast_extract_prefers_available_single_cube_when_it_matches_best_weight() {
        let mut matrix = sample_matrix();
        let mut divisors = vec![DoubleCubeDivisor::new(
            [0],
            [2],
            vec![DivisorOccurrence::new(0, 1, 1)],
            DivisorKind::Other,
        )];

        let report = fast_extract(&mut matrix, &mut divisors).unwrap();

        assert_eq!(report.steps.len(), 2);
        assert!(matches!(report.steps[0], ExtractionStep::SingleCube { .. }));
    }

    #[test]
    fn sis_network_entry_point_reports_missing_native_backend_without_c_abi() {
        assert_eq!(
            fast_extract_sis_network_blocked(),
            Err(FastExtractError::MissingNetworkBackend {
                operation: "fast extraction over SIS network nodes",
            })
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let source = include_str!("fast_extract.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }

    fn sample_matrix() -> ExtractionMatrix {
        ExtractionMatrix::new(vec![
            CubeRow::new(0, [1, 3]),
            CubeRow::new(1, [1, 3, 5]),
            CubeRow::new(2, [0, 2, 4]),
            CubeRow::new(3, [0, 2, 6]),
        ])
    }
}
