//! Native Rust model for `LogicSynthesis/sis/extract/kernel.c`.
//!
//! This module ports the kernel/cube bookkeeping onto owned Rust data. The C
//! entry points that drive all-rectangle generation, subkernel selection, and
//! network mutation remain represented as explicit integration errors because
//! those operations are split across other extract modules.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Cube {
    literals: Vec<usize>,
}

impl Cube {
    pub fn new(literals: impl IntoIterator<Item = usize>) -> Self {
        let mut literals = literals.into_iter().collect::<Vec<_>>();
        literals.sort_unstable();
        literals.dedup();
        Self { literals }
    }

    pub fn len(&self) -> usize {
        self.literals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.literals.is_empty()
    }

    pub fn literals(&self) -> &[usize] {
        &self.literals
    }
}

impl FromIterator<usize> for Cube {
    fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
        Self::new(iter)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionCube {
    pub cube_number: usize,
    pub cube: Cube,
}

impl FunctionCube {
    pub fn new(cube_number: usize, literals: impl IntoIterator<Item = usize>) -> Self {
        Self {
            cube_number,
            cube: Cube::new(literals),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FunctionCover {
    cubes: BTreeMap<usize, Cube>,
}

impl FunctionCover {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_cubes(cubes: impl IntoIterator<Item = impl IntoIterator<Item = usize>>) -> Self {
        let mut cover = Self::new();
        for (cube_number, literals) in cubes.into_iter().enumerate() {
            cover.insert(cube_number, Cube::new(literals));
        }
        cover
    }

    pub fn insert(&mut self, cube_number: usize, cube: Cube) {
        self.cubes.insert(cube_number, cube);
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn cube(&self, cube_number: usize) -> Option<&Cube> {
        self.cubes.get(&cube_number)
    }

    pub fn cubes(&self) -> impl Iterator<Item = FunctionCube> + '_ {
        self.cubes.iter().map(|(cube_number, cube)| FunctionCube {
            cube_number: *cube_number,
            cube: cube.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelCandidate {
    pub kernel_cubes: Vec<FunctionCube>,
    pub co_kernel: Cube,
}

impl KernelCandidate {
    pub fn new(
        kernel_cubes: impl IntoIterator<Item = FunctionCube>,
        co_kernel: impl IntoIterator<Item = usize>,
    ) -> Self {
        Self {
            kernel_cubes: kernel_cubes.into_iter().collect(),
            co_kernel: Cube::new(co_kernel),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCell {
    pub value: usize,
    pub sis_index: usize,
    pub cube_number: usize,
    ref_count: usize,
}

impl ValueCell {
    fn new(sis_index: usize, cube_number: usize, value: usize) -> Self {
        Self {
            value,
            sis_index,
            cube_number,
            ref_count: 1,
        }
    }

    pub fn ref_count(&self) -> usize {
        self.ref_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelEntry {
    pub value_cell: usize,
    pub value: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KernelCubeMatrix {
    entries: BTreeMap<(usize, usize), KernelEntry>,
    rows: BTreeMap<usize, BTreeSet<usize>>,
    cols: BTreeMap<usize, BTreeSet<usize>>,
}

impl KernelCubeMatrix {
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.cols.len()
    }

    pub fn element_count(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.rows.keys().copied()
    }

    pub fn cols(&self) -> impl Iterator<Item = usize> + '_ {
        self.cols.keys().copied()
    }

    pub fn row_columns(&self, row: usize) -> Option<Vec<usize>> {
        self.rows
            .get(&row)
            .map(|cols| cols.iter().copied().collect())
    }

    pub fn col_rows(&self, col: usize) -> Option<Vec<usize>> {
        self.cols
            .get(&col)
            .map(|rows| rows.iter().copied().collect())
    }

    pub fn entry(&self, row: usize, col: usize) -> Option<&KernelEntry> {
        self.entries.get(&(row, col))
    }

    fn next_row(&self) -> usize {
        self.rows.keys().next_back().map(|row| row + 1).unwrap_or(0)
    }

    fn insert(&mut self, row: usize, col: usize, entry: KernelEntry) {
        if self.entries.insert((row, col), entry).is_none() {
            self.rows.entry(row).or_default().insert(col);
            self.cols.entry(col).or_default().insert(row);
        }
    }

    fn remove(&mut self, row: usize, col: usize) -> Option<KernelEntry> {
        let removed = self.entries.remove(&(row, col))?;
        if let Some(cols) = self.rows.get_mut(&row) {
            cols.remove(&col);
            if cols.is_empty() {
                self.rows.remove(&row);
            }
        }
        if let Some(rows) = self.cols.get_mut(&col) {
            rows.remove(&row);
            if rows.is_empty() {
                self.cols.remove(&col);
            }
        }
        Some(removed)
    }

    fn set_value(&mut self, row: usize, col: usize, value: usize) -> bool {
        let Some(entry) = self.entries.get_mut(&(row, col)) else {
            return false;
        };
        entry.value = value;
        true
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CubeIndex {
    cube_to_index: BTreeMap<Cube, usize>,
    index_to_cube: Vec<Cube>,
}

impl CubeIndex {
    pub fn len(&self) -> usize {
        self.index_to_cube.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index_to_cube.is_empty()
    }

    pub fn get_index(&mut self, cube: &Cube) -> usize {
        if let Some(index) = self.cube_to_index.get(cube) {
            return *index;
        }

        let index = self.index_to_cube.len();
        self.cube_to_index.insert(cube.clone(), index);
        self.index_to_cube.push(cube.clone());
        index
    }

    pub fn get_cube(&self, index: usize) -> Option<&Cube> {
        self.index_to_cube.get(index)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClearMode {
    NonOverlapping,
    Overlapping,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RectSelection {
    pub rows: BTreeSet<usize>,
    pub cols: BTreeSet<usize>,
    pub value: isize,
}

impl RectSelection {
    pub fn new(
        rows: impl IntoIterator<Item = usize>,
        cols: impl IntoIterator<Item = usize>,
        value: isize,
    ) -> Self {
        Self {
            rows: rows.into_iter().collect(),
            cols: cols.into_iter().collect(),
            value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KernelExtractError {
    MissingNativePorts { operation: &'static str },
    UnknownFunctionCube { cube_number: usize },
    UnknownKernelCube { column: usize },
    UnknownCoKernel { row: usize },
    MissingRectangleEntry { row: usize, col: usize },
}

impl fmt::Display for KernelExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(
                    f,
                    "{operation} requires native extract integration that is not available yet"
                )
            }
            Self::UnknownFunctionCube { cube_number } => {
                write!(f, "unknown function cube {cube_number}")
            }
            Self::UnknownKernelCube { column } => {
                write!(f, "unknown kernel-cube column {column}")
            }
            Self::UnknownCoKernel { row } => {
                write!(f, "unknown co-kernel row {row}")
            }
            Self::MissingRectangleEntry { row, col } => {
                write!(
                    f,
                    "rectangle entry ({row}, {col}) is not present in the kernel-cube matrix"
                )
            }
        }
    }
}

impl Error for KernelExtractError {}

pub type KernelExtractResult<T> = Result<T, KernelExtractError>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KernelExtractor {
    cube_table: CubeIndex,
    co_kernel_table: Vec<Cube>,
    kernel_cube_matrix: KernelCubeMatrix,
    value_cells: Vec<ValueCell>,
    value_cell_by_source: BTreeMap<(usize, usize), usize>,
    row_cost: Vec<Option<usize>>,
    col_cost: Vec<Option<usize>>,
}

impl KernelExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cube_table(&self) -> &CubeIndex {
        &self.cube_table
    }

    pub fn co_kernel_table(&self) -> &[Cube] {
        &self.co_kernel_table
    }

    pub fn kernel_cube_matrix(&self) -> &KernelCubeMatrix {
        &self.kernel_cube_matrix
    }

    pub fn value_cells(&self) -> &[ValueCell] {
        &self.value_cells
    }

    pub fn row_cost(&self) -> &[Option<usize>] {
        &self.row_cost
    }

    pub fn col_cost(&self) -> &[Option<usize>] {
        &self.col_cost
    }

    pub fn record_function_kernels(
        &mut self,
        function: &FunctionCover,
        sis_index: usize,
        use_all_kernels: bool,
        kernels: impl IntoIterator<Item = KernelCandidate>,
    ) -> KernelExtractResult<usize> {
        if function.is_empty() {
            return Ok(0);
        }

        let mut recorded = 0;
        for candidate in kernels {
            if self.record_kernel(function, sis_index, &candidate, use_all_kernels)? {
                recorded += 1;
            }
        }
        Ok(recorded)
    }

    pub fn finish_costs(&mut self) {
        self.row_cost.clear();
        self.col_cost.clear();

        for row in self.kernel_cube_matrix.rows() {
            if let Some(co_kernel) = self.co_kernel_table.get(row) {
                if self.row_cost.len() <= row {
                    self.row_cost.resize(row + 1, None);
                }
                self.row_cost[row] = Some(co_kernel.len() + 1);
            }
        }

        for col in self.kernel_cube_matrix.cols() {
            if let Some(kernel_cube) = self.cube_table.get_cube(col) {
                if self.col_cost.len() <= col {
                    self.col_cost.resize(col + 1, None);
                }
                self.col_cost[col] = Some(kernel_cube.len());
            }
        }
    }

    pub fn rect_to_kernel(&self, rect: &RectSelection) -> KernelExtractResult<FunctionCover> {
        let mut function = FunctionCover::new();
        for (row, col) in rect.cols.iter().enumerate() {
            let cube = self
                .cube_table
                .get_cube(*col)
                .ok_or(KernelExtractError::UnknownKernelCube { column: *col })?;
            function.insert(row, cube.clone());
        }
        Ok(function)
    }

    pub fn rect_to_cokernel(&self, rect: &RectSelection) -> KernelExtractResult<FunctionCover> {
        let mut function = FunctionCover::new();
        for (row_index, row) in rect.rows.iter().enumerate() {
            let cube = self
                .co_kernel_table
                .get(*row)
                .ok_or(KernelExtractError::UnknownCoKernel { row: *row })?;
            function.insert(row_index, cube.clone());
        }
        Ok(function)
    }

    pub fn clear_rect(&mut self, rect: &RectSelection, mode: ClearMode) -> KernelExtractResult<()> {
        match mode {
            ClearMode::NonOverlapping => self.clear_nonoverlapping_rect(rect),
            ClearMode::Overlapping => self.clear_overlapping_rect(rect),
        }
    }

    fn record_kernel(
        &mut self,
        function: &FunctionCover,
        sis_index: usize,
        candidate: &KernelCandidate,
        use_all_kernels: bool,
    ) -> KernelExtractResult<bool> {
        let co_rect = cover_from_candidate(candidate);
        if !use_all_kernels && !ex_is_level0_kernel(&co_rect) {
            return Ok(false);
        }

        let row = self.kernel_cube_matrix.next_row();
        for kernel_cube in &candidate.kernel_cubes {
            let source_cube = function.cube(kernel_cube.cube_number).ok_or(
                KernelExtractError::UnknownFunctionCube {
                    cube_number: kernel_cube.cube_number,
                },
            )?;
            let col = self.cube_table.get_index(&kernel_cube.cube);
            let value_cell =
                self.ensure_value_cell(sis_index, kernel_cube.cube_number, source_cube.len());
            self.kernel_cube_matrix.insert(
                row,
                col,
                KernelEntry {
                    value_cell,
                    value: self.value_cells[value_cell].value,
                },
            );
        }

        self.co_kernel_table.push(candidate.co_kernel.clone());
        Ok(true)
    }

    fn ensure_value_cell(&mut self, sis_index: usize, cube_number: usize, value: usize) -> usize {
        let key = (sis_index, cube_number);
        if let Some(index) = self.value_cell_by_source.get(&key) {
            let index = *index;
            self.value_cells[index].ref_count += 1;
            return index;
        }

        let index = self.value_cells.len();
        self.value_cells
            .push(ValueCell::new(sis_index, cube_number, value));
        self.value_cell_by_source.insert(key, index);
        index
    }

    fn release_value_cell(&mut self, index: usize) {
        if let Some(cell) = self.value_cells.get_mut(index) {
            cell.ref_count = cell.ref_count.saturating_sub(1);
        }
    }

    fn clear_nonoverlapping_rect(&mut self, rect: &RectSelection) -> KernelExtractResult<()> {
        let mut removed_value_cells = BTreeSet::new();
        for row in &rect.rows {
            for col in &rect.cols {
                let Some(entry) = self.kernel_cube_matrix.remove(*row, *col) else {
                    return Err(KernelExtractError::MissingRectangleEntry {
                        row: *row,
                        col: *col,
                    });
                };
                removed_value_cells.insert(entry.value_cell);
                self.release_value_cell(entry.value_cell);
            }
        }

        let entries_to_remove = self
            .kernel_cube_matrix
            .entries
            .iter()
            .filter_map(|((row, col), entry)| {
                removed_value_cells.contains(&entry.value_cell).then_some((
                    *row,
                    *col,
                    entry.value_cell,
                ))
            })
            .collect::<Vec<_>>();

        for (row, col, value_cell) in entries_to_remove {
            self.kernel_cube_matrix.remove(row, col);
            self.release_value_cell(value_cell);
        }

        Ok(())
    }

    fn clear_overlapping_rect(&mut self, rect: &RectSelection) -> KernelExtractResult<()> {
        for row in &rect.rows {
            for col in &rect.cols {
                if !self.kernel_cube_matrix.set_value(*row, *col, 0) {
                    return Err(KernelExtractError::MissingRectangleEntry {
                        row: *row,
                        col: *col,
                    });
                }
            }
        }
        Ok(())
    }
}

pub fn ex_is_level0_kernel(function: &FunctionCover) -> bool {
    if function.len() <= 1 {
        return false;
    }

    let mut literal_counts = BTreeMap::<usize, usize>::new();
    for cube in function.cubes() {
        for literal in cube.cube.literals() {
            let count = literal_counts.entry(*literal).or_default();
            *count += 1;
            if *count > 1 {
                return false;
            }
        }
    }

    true
}

pub fn sparse_kernel_extract_blocked() -> KernelExtractResult<usize> {
    Err(KernelExtractError::MissingNativePorts {
        operation: "sparse kernel extraction",
    })
}

pub fn overlapping_kernel_extract_blocked() -> KernelExtractResult<usize> {
    Err(KernelExtractError::MissingNativePorts {
        operation: "overlapping kernel extraction",
    })
}

pub fn kernel_extract_from_sis_blocked() -> KernelExtractResult<usize> {
    Err(KernelExtractError::MissingNativePorts {
        operation: "SIS kernel generation",
    })
}

fn cover_from_candidate(candidate: &KernelCandidate) -> FunctionCover {
    let mut cover = FunctionCover::new();
    for kernel_cube in &candidate.kernel_cubes {
        cover.insert(kernel_cube.cube_number, kernel_cube.cube.clone());
    }
    cover
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(cube_number: usize, literals: &[usize]) -> FunctionCube {
        FunctionCube::new(cube_number, literals.iter().copied())
    }

    fn cover(cubes: &[&[usize]]) -> FunctionCover {
        FunctionCover::from_cubes(cubes.iter().map(|cube| cube.iter().copied()))
    }

    #[test]
    fn cube_normalizes_literals_for_cubeindex_lookup() {
        let mut table = CubeIndex::default();
        let first = table.get_index(&Cube::new([3, 1, 3]));
        let second = table.get_index(&Cube::new([1, 3]));

        assert_eq!(first, second);
        assert_eq!(table.get_cube(first).unwrap().literals(), &[1, 3]);
    }

    #[test]
    fn level0_kernel_requires_multiple_rows_and_no_shared_column() {
        assert!(!ex_is_level0_kernel(&cover(&[&[1, 2]])));
        assert!(ex_is_level0_kernel(&cover(&[&[1, 2], &[3, 4]])));
        assert!(!ex_is_level0_kernel(&cover(&[&[1, 2], &[2, 4]])));
    }

    #[test]
    fn records_only_level0_kernel_when_all_kernels_are_disabled() {
        let function = cover(&[&[1, 2, 3], &[4, 5], &[7, 8]]);
        let kernels = [
            KernelCandidate::new([cube(0, &[1]), cube(1, &[2])], [10]),
            KernelCandidate::new([cube(0, &[1, 2]), cube(2, &[2, 9])], [11]),
        ];
        let mut extractor = KernelExtractor::new();

        let recorded = extractor
            .record_function_kernels(&function, 4, false, kernels)
            .unwrap();

        assert_eq!(recorded, 1);
        assert_eq!(extractor.kernel_cube_matrix().row_count(), 1);
        assert_eq!(extractor.kernel_cube_matrix().element_count(), 2);
        assert_eq!(extractor.co_kernel_table(), &[Cube::new([10])]);
        assert_eq!(extractor.value_cells()[0].value, 3);
        assert_eq!(extractor.value_cells()[1].value, 2);
    }

    #[test]
    fn finish_costs_match_c_row_and_column_cost_rules() {
        let function = cover(&[&[1, 2, 3], &[4, 5]]);
        let kernels = [KernelCandidate::new(
            [cube(0, &[1]), cube(1, &[2, 3])],
            [8, 9],
        )];
        let mut extractor = KernelExtractor::new();

        extractor
            .record_function_kernels(&function, 6, true, kernels)
            .unwrap();
        extractor.finish_costs();

        assert_eq!(extractor.row_cost(), &[Some(3)]);
        assert_eq!(extractor.col_cost(), &[Some(1), Some(2)]);
    }

    #[test]
    fn rect_conversion_uses_kernel_cube_columns_and_cokernel_rows() {
        let function = cover(&[&[1, 2, 3], &[4, 5]]);
        let kernels = [
            KernelCandidate::new([cube(0, &[1]), cube(1, &[2])], [8]),
            KernelCandidate::new([cube(0, &[1]), cube(1, &[3])], [9, 10]),
        ];
        let mut extractor = KernelExtractor::new();
        extractor
            .record_function_kernels(&function, 2, true, kernels)
            .unwrap();
        let rect = RectSelection::new([0, 1], [0, 2], 5);

        let kernel = extractor.rect_to_kernel(&rect).unwrap();
        let cokernel = extractor.rect_to_cokernel(&rect).unwrap();

        assert_eq!(kernel.cube(0).unwrap().literals(), &[1]);
        assert_eq!(kernel.cube(1).unwrap().literals(), &[3]);
        assert_eq!(cokernel.cube(0).unwrap().literals(), &[8]);
        assert_eq!(cokernel.cube(1).unwrap().literals(), &[9, 10]);
    }

    #[test]
    fn nonoverlapping_clear_removes_all_aliases_for_selected_value_cells() {
        let function = cover(&[&[1, 2, 3], &[4, 5]]);
        let kernels = [
            KernelCandidate::new([cube(0, &[1]), cube(1, &[2])], [8]),
            KernelCandidate::new([cube(0, &[3]), cube(1, &[4])], [9]),
        ];
        let mut extractor = KernelExtractor::new();
        extractor
            .record_function_kernels(&function, 2, true, kernels)
            .unwrap();

        extractor
            .clear_rect(&RectSelection::new([0], [0], 0), ClearMode::NonOverlapping)
            .unwrap();

        assert_eq!(extractor.kernel_cube_matrix().element_count(), 2);
        assert!(extractor.kernel_cube_matrix().entry(0, 0).is_none());
        assert!(extractor.kernel_cube_matrix().entry(1, 2).is_none());
        assert!(extractor.kernel_cube_matrix().entry(0, 1).is_some());
        assert!(extractor.kernel_cube_matrix().entry(1, 3).is_some());
    }

    #[test]
    fn overlapping_clear_zeroes_selected_entry_values_without_removal() {
        let function = cover(&[&[1, 2, 3], &[4, 5]]);
        let kernels = [KernelCandidate::new([cube(0, &[1]), cube(1, &[2])], [8])];
        let mut extractor = KernelExtractor::new();
        extractor
            .record_function_kernels(&function, 2, true, kernels)
            .unwrap();

        extractor
            .clear_rect(&RectSelection::new([0], [0], 0), ClearMode::Overlapping)
            .unwrap();

        assert_eq!(extractor.kernel_cube_matrix().element_count(), 2);
        assert_eq!(extractor.kernel_cube_matrix().entry(0, 0).unwrap().value, 0);
        assert_eq!(extractor.kernel_cube_matrix().entry(0, 1).unwrap().value, 2);
    }

    #[test]
    fn blocked_integration_entry_points_report_missing_ports() {
        assert!(matches!(
            sparse_kernel_extract_blocked(),
            Err(KernelExtractError::MissingNativePorts { .. })
        ));
        assert!(matches!(
            overlapping_kernel_extract_blocked(),
            Err(KernelExtractError::MissingNativePorts { .. })
        ));
        assert!(matches!(
            kernel_extract_from_sis_blocked(),
            Err(KernelExtractError::MissingNativePorts { .. })
        ));
    }
}
