//! Native Rust port of `sis/extract/cubeindex.c`.
//!
//! SIS used this helper as a bidirectional table between sparse-matrix cube
//! rows and compact integer ids while building extraction matrices. The C
//! version duplicated each row before inserting it into an `st_table`; this port
//! keeps the same snapshot behavior with owned `SparseRow` values and stable
//! insertion-order indices.

use crate::ports::sparse::rows::SparseRow;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CubeIndex {
    cubes: Vec<SparseRow>,
}

impl CubeIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn get_index(&mut self, cube: &SparseRow) -> usize {
        if let Some(index) = self.cubes.iter().position(|stored| stored == cube) {
            return index;
        }

        let index = self.cubes.len();
        self.cubes.push(cube.clone());
        index
    }

    pub fn get_cube(&self, index: usize) -> Option<&SparseRow> {
        self.cubes.get(index)
    }

    pub fn cubes(&self) -> &[SparseRow] {
        &self.cubes
    }
}

impl FromIterator<SparseRow> for CubeIndex {
    fn from_iter<T: IntoIterator<Item = SparseRow>>(iter: T) -> Self {
        let mut index = Self::new();
        for cube in iter {
            index.get_index(&cube);
        }

        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        let index = CubeIndex::new();

        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
        assert_eq!(index.get_cube(0), None);
        assert_eq!(index.cubes(), &[]);
    }

    #[test]
    fn assigns_stable_indices_in_first_seen_order() {
        let mut index = CubeIndex::new();
        let first = row([3, 1]);
        let second = row([9]);

        assert_eq!(index.get_index(&first), 0);
        assert_eq!(index.get_index(&second), 1);
        assert_eq!(index.get_index(&first), 0);

        assert_eq!(index.len(), 2);
        assert_eq!(index.get_cube(0), Some(&first));
        assert_eq!(index.get_cube(1), Some(&second));
    }

    #[test]
    fn stores_a_snapshot_of_the_cube() {
        let mut index = CubeIndex::new();
        let mut cube = row([4, 8]);

        let cube_index = index.get_index(&cube);
        cube.insert(12);

        assert_eq!(cube_index, 0);
        assert_eq!(index.get_cube(0), Some(&row([4, 8])));
        assert_eq!(index.get_index(&cube), 1);
    }

    #[test]
    fn treats_duplicate_column_inputs_as_the_same_cube() {
        let mut index = CubeIndex::new();

        assert_eq!(index.get_index(&row([7, 2, 7])), 0);
        assert_eq!(index.get_index(&row([2, 7])), 0);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn from_iterator_deduplicates_like_repeated_get_index_calls() {
        let index = [row([3]), row([5]), row([3])]
            .into_iter()
            .collect::<CubeIndex>();

        assert_eq!(index.len(), 2);
        assert_eq!(index.get_cube(0), Some(&row([3])));
        assert_eq!(index.get_cube(1), Some(&row([5])));
        assert_eq!(index.get_cube(2), None);
    }

    fn row<const N: usize>(columns: [i32; N]) -> SparseRow {
        columns.into_iter().collect()
    }
}
