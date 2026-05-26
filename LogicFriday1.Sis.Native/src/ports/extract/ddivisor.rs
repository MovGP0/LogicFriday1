//! Native Rust port of `sis/extract/ddivisor.c`.
//!
//! The legacy implementation builds double-cube divisors from pairs of sparse
//! matrix rows, classifies the divisor shape, and coalesces equivalent divisors
//! while recording every cube pair that produced them. This port keeps that
//! behavior in owned Rust data structures and deliberately avoids legacy C ABI
//! entry points.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoubleCubeDivisorType {
    D112,
    D222,
    D223,
    D224,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeightStatus {
    Old,
    New,
    Changed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DivisorStatus {
    NonCheck,
    InBase,
    InBetween,
    InDivisor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoubleCubeDivisor {
    cube1: Vec<usize>,
    cube2: Vec<usize>,
    divisor_type: DoubleCubeDivisorType,
    weight_status: WeightStatus,
    weight: i32,
    level: i32,
    status: DivisorStatus,
    occurrence_indices: Vec<usize>,
}

impl DoubleCubeDivisor {
    pub fn cube1(&self) -> &[usize] {
        &self.cube1
    }

    pub fn cube2(&self) -> &[usize] {
        &self.cube2
    }

    pub fn divisor_type(&self) -> DoubleCubeDivisorType {
        self.divisor_type
    }

    pub fn weight_status(&self) -> WeightStatus {
        self.weight_status
    }

    pub fn weight(&self) -> i32 {
        self.weight
    }

    pub fn level(&self) -> i32 {
        self.level
    }

    pub fn status(&self) -> DivisorStatus {
        self.status
    }

    pub fn occurrence_indices(&self) -> &[usize] {
        &self.occurrence_indices
    }
}

impl Default for DoubleCubeDivisor {
    fn default() -> Self {
        Self {
            cube1: Vec::new(),
            cube2: Vec::new(),
            divisor_type: DoubleCubeDivisorType::Other,
            weight_status: WeightStatus::New,
            weight: 0,
            level: 0,
            status: DivisorStatus::NonCheck,
            occurrence_indices: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoubleCubeDivisorCell {
    cube1_row: usize,
    cube2_row: usize,
    sis_id: i32,
    phase: usize,
    base_length: usize,
    divisor_index: usize,
}

impl DoubleCubeDivisorCell {
    pub fn cube1_row(&self) -> usize {
        self.cube1_row
    }

    pub fn cube2_row(&self) -> usize {
        self.cube2_row
    }

    pub fn sis_id(&self) -> i32 {
        self.sis_id
    }

    pub fn phase(&self) -> usize {
        self.phase
    }

    pub fn base_length(&self) -> usize {
        self.base_length
    }

    pub fn divisor_index(&self) -> usize {
        self.divisor_index
    }
}

impl Default for DoubleCubeDivisorCell {
    fn default() -> Self {
        Self {
            cube1_row: 0,
            cube2_row: 0,
            sis_id: -1,
            phase: 0,
            base_length: 0,
            divisor_index: usize::MAX,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseCube {
    sis_id: i32,
    divisor_cell_indices: Vec<usize>,
}

impl SparseCube {
    pub fn sis_id(&self) -> i32 {
        self.sis_id
    }

    pub fn divisor_cell_indices(&self) -> &[usize] {
        &self.divisor_cell_indices
    }
}

impl Default for SparseCube {
    fn default() -> Self {
        Self {
            sis_id: -1,
            divisor_cell_indices: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DoubleCubeExtractOptions {
    pub max_cube1_length: Option<usize>,
    pub max_cube2_length: Option<usize>,
}

impl DoubleCubeExtractOptions {
    pub fn unrestricted() -> Self {
        Self {
            max_cube1_length: None,
            max_cube2_length: None,
        }
    }

    pub fn with_length_filter(max_cube1_length: usize, max_cube2_length: usize) -> Self {
        Self {
            max_cube1_length: Some(max_cube1_length),
            max_cube2_length: Some(max_cube2_length),
        }
    }
}

impl Default for DoubleCubeExtractOptions {
    fn default() -> Self {
        Self::unrestricted()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DoubleCubeDivisorSet {
    divisors: Vec<DoubleCubeDivisor>,
    cells: Vec<DoubleCubeDivisorCell>,
    node_cubes: BTreeMap<i32, Vec<usize>>,
    cube_metadata: BTreeMap<usize, SparseCube>,
    d112_set: BTreeMap<(usize, usize), Vec<usize>>,
    d222_set: BTreeMap<(usize, usize), Vec<usize>>,
    d223_set: BTreeMap<(usize, usize), Vec<usize>>,
    d224_set: BTreeMap<(usize, usize), Vec<usize>>,
    other_set: BTreeMap<(usize, usize, usize, usize), Vec<usize>>,
}

impl DoubleCubeDivisorSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn divisors(&self) -> &[DoubleCubeDivisor] {
        &self.divisors
    }

    pub fn cells(&self) -> &[DoubleCubeDivisorCell] {
        &self.cells
    }

    pub fn node_cubes(&self) -> &BTreeMap<i32, Vec<usize>> {
        &self.node_cubes
    }

    pub fn cube_metadata(&self) -> &BTreeMap<usize, SparseCube> {
        &self.cube_metadata
    }

    pub fn append_divisor(
        &mut self,
        mut divisor: DoubleCubeDivisor,
        mut cell: DoubleCubeDivisorCell,
    ) -> usize {
        divisor.divisor_type = decide_divisor_type(&mut divisor);
        let candidates = self.check_set(&divisor).cloned().unwrap_or_default();

        for candidate_index in candidates {
            let mut phase = 0;
            if check_exists(&divisor, &self.divisors[candidate_index], &mut phase) {
                cell.phase = phase;
                cell.divisor_index = candidate_index;
                let cell_index = self.cells.len();
                self.cells.push(cell);
                self.divisors[candidate_index]
                    .occurrence_indices
                    .push(cell_index);
                return candidate_index;
            }
        }

        let divisor_index = self.divisors.len();
        cell.divisor_index = divisor_index;
        let cell_index = self.cells.len();
        divisor.occurrence_indices.push(cell_index);
        self.cells.push(cell);
        self.hash_divisor(divisor_index, &divisor);
        self.divisors.push(divisor);

        divisor_index
    }

    fn record_node_cubes(&mut self, sis_id: i32, row_range: impl Iterator<Item = usize>) {
        let node_rows = self.node_cubes.entry(sis_id).or_default();
        for row in row_range {
            node_rows.push(row);
            self.cube_metadata.insert(
                row,
                SparseCube {
                    sis_id,
                    divisor_cell_indices: Vec::new(),
                },
            );
        }
    }

    fn check_set(&self, divisor: &DoubleCubeDivisor) -> Option<&Vec<usize>> {
        match divisor.divisor_type {
            DoubleCubeDivisorType::D112 => self
                .d112_set
                .get(&(first_col(&divisor.cube1)?, last_col(&divisor.cube2)?)),
            DoubleCubeDivisorType::D222 => self.d222_set.get(&(
                first_col(&divisor.cube1)? / 2,
                last_col(&divisor.cube1)? / 2,
            )),
            DoubleCubeDivisorType::D223 => self.d223_set.get(&(
                first_col(&divisor.cube1)? / 2,
                last_col(&divisor.cube1)? / 2,
            )),
            DoubleCubeDivisorType::D224 => self
                .d224_set
                .get(&(last_col(&divisor.cube1)?, last_col(&divisor.cube2)?)),
            DoubleCubeDivisorType::Other => self.other_set.get(&(
                divisor.cube1.len(),
                divisor.cube2.len(),
                last_col(&divisor.cube1)?,
                last_col(&divisor.cube2)?,
            )),
        }
    }

    fn hash_divisor(&mut self, divisor_index: usize, divisor: &DoubleCubeDivisor) {
        match divisor.divisor_type {
            DoubleCubeDivisorType::D112 => push_if_keyed(
                &mut self.d112_set,
                first_col(&divisor.cube1).zip(last_col(&divisor.cube2)),
                divisor_index,
            ),
            DoubleCubeDivisorType::D222 => push_if_keyed(
                &mut self.d222_set,
                first_col(&divisor.cube1)
                    .zip(last_col(&divisor.cube1))
                    .map(|(key1, key2)| (key1 / 2, key2 / 2)),
                divisor_index,
            ),
            DoubleCubeDivisorType::D223 => push_if_keyed(
                &mut self.d223_set,
                first_col(&divisor.cube1)
                    .zip(last_col(&divisor.cube1))
                    .map(|(key1, key2)| (key1 / 2, key2 / 2)),
                divisor_index,
            ),
            DoubleCubeDivisorType::D224 => push_if_keyed(
                &mut self.d224_set,
                last_col(&divisor.cube1).zip(last_col(&divisor.cube2)),
                divisor_index,
            ),
            DoubleCubeDivisorType::Other => push_if_keyed(
                &mut self.other_set,
                last_col(&divisor.cube1)
                    .zip(last_col(&divisor.cube2))
                    .map(|(key1, key2)| (divisor.cube1.len(), divisor.cube2.len(), key1, key2)),
                divisor_index,
            ),
        }
    }
}

pub fn clear_row_elements(cube: &mut Vec<usize>, to_remove: &[usize]) {
    let remove_set = to_remove.iter().copied().collect::<BTreeSet<_>>();
    cube.retain(|col| !remove_set.contains(col));
}

pub fn generate_double_cube_divisor(
    cube1: &[usize],
    cube2: &[usize],
    cube1_row: usize,
    cube2_row: usize,
    sis_id: i32,
) -> (DoubleCubeDivisor, DoubleCubeDivisorCell) {
    let normalized_cube1 = normalized_row(cube1);
    let normalized_cube2 = normalized_row(cube2);
    let base = row_intersection(&normalized_cube1, &normalized_cube2);

    let mut divisor = DoubleCubeDivisor {
        cube1: normalized_cube1,
        cube2: normalized_cube2,
        ..DoubleCubeDivisor::default()
    };

    if !base.is_empty() {
        clear_row_elements(&mut divisor.cube1, &base);
        clear_row_elements(&mut divisor.cube2, &base);
    }

    let cell = DoubleCubeDivisorCell {
        cube1_row,
        cube2_row,
        sis_id,
        base_length: base.len(),
        ..DoubleCubeDivisorCell::default()
    };

    (divisor, cell)
}

pub fn decide_divisor_type(divisor: &mut DoubleCubeDivisor) -> DoubleCubeDivisorType {
    if divisor.cube1.len() == 1 && divisor.cube2.len() == 1 {
        order_by_first_column(divisor);
        return DoubleCubeDivisorType::D112;
    }

    if divisor.cube1.len() == 2 && divisor.cube2.len() == 2 {
        order_by_first_column(divisor);
        return match variable_count(divisor) {
            2 => DoubleCubeDivisorType::D222,
            3 => DoubleCubeDivisorType::D223,
            4 => DoubleCubeDivisorType::D224,
            _ => DoubleCubeDivisorType::Other,
        };
    }

    if divisor.cube1.len() == divisor.cube2.len() {
        order_by_first_column(divisor);
    }

    DoubleCubeDivisorType::Other
}

pub fn check_exists(
    divisor: &DoubleCubeDivisor,
    existing: &DoubleCubeDivisor,
    phase: &mut usize,
) -> bool {
    *phase = 0;
    match divisor.divisor_type {
        DoubleCubeDivisorType::D112 => true,
        DoubleCubeDivisorType::D222 => check_d222(divisor, existing, phase),
        DoubleCubeDivisorType::D223 => check_d223(divisor, existing, phase),
        _ => check_other(divisor, existing),
    }
}

pub fn extract_double_cube_divisors(
    rows: &[Vec<usize>],
    sis_id: i32,
    start_row: usize,
    divisor_set: &mut DoubleCubeDivisorSet,
    options: DoubleCubeExtractOptions,
    mut should_stop: impl FnMut() -> bool,
) -> usize {
    if start_row >= rows.len() {
        return 0;
    }

    if rows.len() - start_row == 1 {
        divisor_set.cube_metadata.insert(
            start_row,
            SparseCube {
                sis_id,
                divisor_cell_indices: Vec::new(),
            },
        );
        return 0;
    }

    divisor_set.record_node_cubes(sis_id, start_row..rows.len());
    let initial_cell_count = divisor_set.cells.len();

    for i in start_row..rows.len() - 1 {
        for j in i + 1..rows.len() {
            let (first_row, second_row) = if rows[i].len() < rows[j].len() {
                (i, j)
            } else {
                (j, i)
            };

            let (divisor, cell) = generate_double_cube_divisor(
                &rows[first_row],
                &rows[second_row],
                first_row,
                second_row,
                sis_id,
            );

            if is_filtered_by_length(&divisor, options) {
                continue;
            }

            let divisor_index = divisor_set.append_divisor(divisor, cell);
            let cell_index = divisor_set.divisors[divisor_index]
                .occurrence_indices
                .last()
                .copied()
                .expect("append_divisor records an occurrence");

            divisor_set
                .cube_metadata
                .entry(i)
                .or_default()
                .divisor_cell_indices
                .push(cell_index);
            divisor_set
                .cube_metadata
                .entry(j)
                .or_default()
                .divisor_cell_indices
                .push(cell_index);

            if should_stop() {
                return divisor_set.cells.len() - initial_cell_count;
            }
        }
    }

    divisor_set.cells.len() - initial_cell_count
}

fn is_filtered_by_length(divisor: &DoubleCubeDivisor, options: DoubleCubeExtractOptions) -> bool {
    options
        .max_cube1_length
        .is_some_and(|length| divisor.cube1.len() > length)
        || options
            .max_cube2_length
            .is_some_and(|length| divisor.cube2.len() > length)
}

fn check_other(divisor: &DoubleCubeDivisor, existing: &DoubleCubeDivisor) -> bool {
    first_col(&divisor.cube2) == first_col(&existing.cube2)
        && divisor.cube1 == existing.cube1
        && divisor.cube2 == existing.cube2
}

fn check_d222(
    divisor: &DoubleCubeDivisor,
    existing: &DoubleCubeDivisor,
    phase: &mut usize,
) -> bool {
    *phase = usize::from(
        first_col(&divisor.cube1) != first_col(&existing.cube1)
            || last_col(&divisor.cube1) != last_col(&existing.cube1),
    );
    true
}

fn check_d223(
    divisor: &DoubleCubeDivisor,
    existing: &DoubleCubeDivisor,
    phase: &mut usize,
) -> bool {
    let cube1 = &divisor.cube1;
    let cube2 = &divisor.cube2;
    let cube3 = &existing.cube1;
    let cube4 = &existing.cube2;

    let con1 = first_col(cube1) == first_col(cube3);
    let con2 = last_col(cube1) == last_col(cube3);

    *phase = 0;
    match usize::from(con1) + usize::from(con2) {
        2 => cube2 == cube4,
        1 => {
            let Some(cube2_first) = first_col(cube2) else {
                return false;
            };
            let Some(cube2_last) = last_col(cube2) else {
                return false;
            };
            let Some(cube4_first) = first_col(cube4) else {
                return false;
            };
            let Some(cube4_last) = last_col(cube4) else {
                return false;
            };

            if cube2_first / 2 != cube4_first / 2 || cube2_last / 2 != cube4_last / 2 {
                return false;
            }

            let mut complement_col = if con1 {
                first_col(cube1).expect("con1 requires a first column")
            } else {
                last_col(cube1).expect("con2 requires a last column")
            };

            complement_col = if complement_col % 2 == 0 {
                complement_col + 1
            } else {
                complement_col - 1
            };

            if !cube4.binary_search(&complement_col).is_ok() {
                return false;
            }

            let con3 = cube2_first.abs_diff(cube4_first);
            let con4 = cube2_last.abs_diff(cube4_last);
            if con3 + con4 == 1 {
                *phase = 1;
                return true;
            }

            false
        }
        _ => false,
    }
}

fn variable_count(divisor: &DoubleCubeDivisor) -> usize {
    let mut count = 4;
    let terminals1 = [first_col(&divisor.cube1), last_col(&divisor.cube1)];
    let terminals2 = [first_col(&divisor.cube2), last_col(&divisor.cube2)];

    for col1 in terminals1.into_iter().flatten() {
        for col2 in terminals2.into_iter().flatten() {
            if col1 / 2 == col2 / 2 {
                count -= 1;
                break;
            }
        }
    }

    count
}

fn order_by_first_column(divisor: &mut DoubleCubeDivisor) {
    if first_col(&divisor.cube1) > first_col(&divisor.cube2) {
        std::mem::swap(&mut divisor.cube1, &mut divisor.cube2);
    }
}

fn normalized_row(row: &[usize]) -> Vec<usize> {
    row.iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn row_intersection(row1: &[usize], row2: &[usize]) -> Vec<usize> {
    row1.iter()
        .copied()
        .filter(|col| row2.binary_search(col).is_ok())
        .collect()
}

fn first_col(row: &[usize]) -> Option<usize> {
    row.first().copied()
}

fn last_col(row: &[usize]) -> Option<usize> {
    row.last().copied()
}

fn push_if_keyed<K>(table: &mut BTreeMap<K, Vec<usize>>, key: Option<K>, divisor_index: usize)
where
    K: Ord,
{
    if let Some(key) = key {
        table.entry(key).or_default().push(divisor_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_divisor_removes_common_base_literals() {
        let (divisor, cell) = generate_double_cube_divisor(&[2, 4, 6], &[4, 6, 8], 0, 1, 11);

        assert_eq!(divisor.cube1(), &[2]);
        assert_eq!(divisor.cube2(), &[8]);
        assert_eq!(cell.base_length(), 2);
        assert_eq!(cell.sis_id(), 11);
    }

    #[test]
    fn decide_type_sorts_single_literal_cubes_as_d112() {
        let mut divisor = DoubleCubeDivisor {
            cube1: vec![9],
            cube2: vec![2],
            ..DoubleCubeDivisor::default()
        };

        assert_eq!(
            decide_divisor_type(&mut divisor),
            DoubleCubeDivisorType::D112
        );
        assert_eq!(divisor.cube1(), &[2]);
        assert_eq!(divisor.cube2(), &[9]);
    }

    #[test]
    fn decide_type_classifies_two_literal_variable_counts() {
        let mut d222 = divisor_from_cubes([0, 2], [1, 3]);
        let mut d223 = divisor_from_cubes([0, 2], [1, 5]);
        let mut d224 = divisor_from_cubes([0, 2], [5, 7]);

        assert_eq!(decide_divisor_type(&mut d222), DoubleCubeDivisorType::D222);
        assert_eq!(decide_divisor_type(&mut d223), DoubleCubeDivisorType::D223);
        assert_eq!(decide_divisor_type(&mut d224), DoubleCubeDivisorType::D224);
    }

    #[test]
    fn d222_equivalence_tracks_complement_phase() {
        let mut set = DoubleCubeDivisorSet::new();
        let divisor1 = classified_divisor([0, 2], [1, 3]);
        let divisor2 = classified_divisor([1, 2], [0, 3]);

        set.append_divisor(divisor1, cell_from_rows(0, 1));
        let index = set.append_divisor(divisor2, cell_from_rows(2, 3));

        assert_eq!(index, 0);
        assert_eq!(set.divisors().len(), 1);
        assert_eq!(set.cells()[1].phase(), 1);
    }

    #[test]
    fn d223_equivalence_accepts_legacy_phase_complement_case() {
        let mut set = DoubleCubeDivisorSet::new();
        let divisor1 = classified_divisor([0, 2], [1, 5]);
        let divisor2 = classified_divisor([0, 3], [1, 4]);

        set.append_divisor(divisor1, cell_from_rows(0, 1));
        let index = set.append_divisor(divisor2, cell_from_rows(2, 3));

        assert_eq!(index, 0);
        assert_eq!(set.divisors().len(), 1);
        assert_eq!(set.cells()[1].phase(), 1);
    }

    #[test]
    fn other_divisors_coalesce_only_when_both_residual_cubes_match() {
        let mut set = DoubleCubeDivisorSet::new();
        let first = classified_divisor([1, 3, 5], [2, 4]);
        let same = classified_divisor([1, 3, 5], [2, 4]);
        let different = classified_divisor([1, 3, 5], [2, 6]);

        set.append_divisor(first, cell_from_rows(0, 1));
        set.append_divisor(same, cell_from_rows(2, 3));
        set.append_divisor(different, cell_from_rows(4, 5));

        assert_eq!(set.divisors().len(), 2);
        assert_eq!(set.divisors()[0].occurrence_indices(), &[0, 1]);
        assert_eq!(set.divisors()[1].occurrence_indices(), &[2]);
    }

    #[test]
    fn extract_records_cells_on_both_source_cubes() {
        let rows = vec![vec![0, 2, 4], vec![0, 2, 6], vec![0, 2, 8]];
        let mut set = DoubleCubeDivisorSet::new();

        let count = extract_double_cube_divisors(
            &rows,
            7,
            0,
            &mut set,
            DoubleCubeExtractOptions::unrestricted(),
            || false,
        );

        assert_eq!(count, 3);
        assert_eq!(set.divisors().len(), 3);
        assert_eq!(set.node_cubes().get(&7), Some(&vec![0, 1, 2]));
        assert_eq!(set.cube_metadata()[&0].divisor_cell_indices(), &[0, 1]);
        assert_eq!(set.cube_metadata()[&1].divisor_cell_indices(), &[0, 2]);
        assert_eq!(set.cube_metadata()[&2].divisor_cell_indices(), &[1, 2]);
    }

    #[test]
    fn extract_applies_legacy_delete_length_filter() {
        let rows = vec![vec![0, 2, 4], vec![6, 8, 10], vec![0, 2, 12]];
        let mut set = DoubleCubeDivisorSet::new();

        let count = extract_double_cube_divisors(
            &rows,
            7,
            0,
            &mut set,
            DoubleCubeExtractOptions::with_length_filter(1, 1),
            || false,
        );

        assert_eq!(count, 1);
        assert_eq!(set.cells()[0].cube1_row(), 2);
        assert_eq!(set.cells()[0].cube2_row(), 0);
        assert_eq!(set.cells()[0].base_length(), 2);
    }

    #[test]
    fn extract_stops_after_current_pair_when_timeout_closure_triggers() {
        let rows = vec![vec![0], vec![2], vec![4]];
        let mut set = DoubleCubeDivisorSet::new();

        let count = extract_double_cube_divisors(
            &rows,
            7,
            0,
            &mut set,
            DoubleCubeExtractOptions::unrestricted(),
            || true,
        );

        assert_eq!(count, 1);
        assert_eq!(set.cells().len(), 1);
    }

    fn divisor_from_cubes(
        cube1: impl IntoIterator<Item = usize>,
        cube2: impl IntoIterator<Item = usize>,
    ) -> DoubleCubeDivisor {
        DoubleCubeDivisor {
            cube1: cube1.into_iter().collect(),
            cube2: cube2.into_iter().collect(),
            ..DoubleCubeDivisor::default()
        }
    }

    fn classified_divisor(
        cube1: impl IntoIterator<Item = usize>,
        cube2: impl IntoIterator<Item = usize>,
    ) -> DoubleCubeDivisor {
        let mut divisor = divisor_from_cubes(cube1, cube2);
        divisor.divisor_type = decide_divisor_type(&mut divisor);
        divisor
    }

    fn cell_from_rows(cube1_row: usize, cube2_row: usize) -> DoubleCubeDivisorCell {
        DoubleCubeDivisorCell {
            cube1_row,
            cube2_row,
            ..DoubleCubeDivisorCell::default()
        }
    }
}
