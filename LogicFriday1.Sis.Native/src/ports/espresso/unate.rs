use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct UnateSet {
    columns: BTreeSet<usize>,
}

impl UnateSet {
    pub fn empty() -> Self {
        Self {
            columns: BTreeSet::new(),
        }
    }

    pub fn full(column_count: usize) -> Self {
        Self {
            columns: (0..column_count).collect(),
        }
    }

    pub fn from_columns(columns: impl IntoIterator<Item = usize>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn columns(&self) -> &BTreeSet<usize> {
        &self.columns
    }

    pub fn into_columns(self) -> BTreeSet<usize> {
        self.columns
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    pub fn contains(&self, column: usize) -> bool {
        self.columns.contains(&column)
    }

    pub fn insert(&mut self, column: usize) {
        self.columns.insert(column);
    }

    pub fn remove(&mut self, column: usize) {
        self.columns.remove(&column);
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.columns.is_disjoint(&other.columns)
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        self.columns.is_subset(&other.columns)
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            columns: self.columns.union(&other.columns).copied().collect(),
        }
    }

    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            columns: self.columns.intersection(&other.columns).copied().collect(),
        }
    }

    pub fn difference(&self, other: &Self) -> Self {
        Self {
            columns: self.columns.difference(&other.columns).copied().collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnateCover {
    column_count: usize,
    rows: Vec<UnateSet>,
}

impl UnateCover {
    pub fn new(column_count: usize, rows: Vec<UnateSet>) -> Result<Self, UnateError> {
        for row in &rows {
            if let Some(column) = row.columns().iter().find(|column| **column >= column_count) {
                return Err(UnateError::ColumnOutOfRange {
                    column: *column,
                    column_count,
                });
            }
        }

        Ok(Self { column_count, rows })
    }

    pub fn from_rows<I, R>(column_count: usize, rows: I) -> Result<Self, UnateError>
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = usize>,
    {
        Self::new(
            column_count,
            rows.into_iter().map(UnateSet::from_columns).collect(),
        )
    }

    pub fn column_count(&self) -> usize {
        self.column_count
    }

    pub fn rows(&self) -> &[UnateSet] {
        &self.rows
    }

    pub fn into_rows(self) -> Vec<UnateSet> {
        self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnateColumnMapping {
    source_column: usize,
    remove_when_selected: UnateSet,
}

impl UnateColumnMapping {
    pub fn new(
        source_column: usize,
        remove_when_selected: impl IntoIterator<Item = usize>,
    ) -> Self {
        Self {
            source_column,
            remove_when_selected: UnateSet::from_columns(remove_when_selected),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnateError {
    ColumnOutOfRange { column: usize, column_count: usize },
    MappingColumnOutOfRange { column: usize, column_count: usize },
    CoverColumnCountMismatch { left: usize, right: usize },
}

impl fmt::Display for UnateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColumnOutOfRange {
                column,
                column_count,
            } => write!(formatter, "column {column} is outside 0..{column_count}"),
            Self::MappingColumnOutOfRange {
                column,
                column_count,
            } => write!(
                formatter,
                "mapping column {column} is outside source cover range 0..{column_count}"
            ),
            Self::CoverColumnCountMismatch { left, right } => {
                write!(formatter, "cover column counts differ: {left} != {right}")
            }
        }
    }
}

impl std::error::Error for UnateError {}

pub fn map_cover_to_unate(
    cover: &UnateCover,
    mappings: &[UnateColumnMapping],
) -> Result<UnateCover, UnateError> {
    for mapping in mappings {
        if mapping.source_column >= cover.column_count() {
            return Err(UnateError::MappingColumnOutOfRange {
                column: mapping.source_column,
                column_count: cover.column_count(),
            });
        }
    }

    let mut rows = Vec::with_capacity(cover.len());
    for source_row in cover.rows() {
        let mut mapped_row = UnateSet::empty();
        for (index, mapping) in mappings.iter().enumerate() {
            if !source_row.contains(mapping.source_column) {
                mapped_row.insert(index);
            }
        }

        rows.push(mapped_row);
    }

    UnateCover::new(mappings.len(), rows)
}

pub fn map_unate_to_cover(
    unate: &UnateCover,
    mappings: &[UnateColumnMapping],
    cover_column_count: usize,
) -> Result<UnateCover, UnateError> {
    if unate.column_count() != mappings.len() {
        return Err(UnateError::CoverColumnCountMismatch {
            left: unate.column_count(),
            right: mappings.len(),
        });
    }

    for mapping in mappings {
        for column in mapping.remove_when_selected.columns() {
            if *column >= cover_column_count {
                return Err(UnateError::MappingColumnOutOfRange {
                    column: *column,
                    column_count: cover_column_count,
                });
            }
        }
    }

    let mut rows = Vec::with_capacity(unate.len());
    for unate_row in unate.rows() {
        let mut mapped_row = UnateSet::full(cover_column_count);
        for (index, mapping) in mappings.iter().enumerate() {
            if unate_row.contains(index) {
                mapped_row = mapped_row.difference(&mapping.remove_when_selected);
            }
        }

        rows.push(mapped_row);
    }

    UnateCover::new(cover_column_count, rows)
}

pub fn unate_compl(cover: &UnateCover) -> UnateCover {
    let mut rows = unate_complement_rows(cover.rows(), cover.column_count());
    retain_minimal(&mut rows);
    UnateCover {
        column_count: cover.column_count(),
        rows,
    }
}

pub fn unate_intersect(
    left: &UnateCover,
    right: &UnateCover,
    largest_only: bool,
) -> Result<UnateCover, UnateError> {
    if left.column_count() != right.column_count() {
        return Err(UnateError::CoverColumnCountMismatch {
            left: left.column_count(),
            right: right.column_count(),
        });
    }

    let mut rows = Vec::new();
    let mut largest = 0;
    for left_row in left.rows() {
        for right_row in right.rows() {
            let row = left_row.intersection(right_row);
            if row.is_empty() {
                continue;
            }

            if largest_only {
                let row_size = row.len();
                if row_size > largest {
                    rows.clear();
                    largest = row_size;
                }

                if row_size < largest {
                    continue;
                }
            }

            rows.push(row);
        }
    }

    retain_maximal(&mut rows);
    UnateCover::new(left.column_count(), rows)
}

pub fn exact_minimum_cover(cover: &UnateCover) -> UnateCover {
    let mut rows = unate_compl(cover).into_rows();
    if let Some(minimum) = rows.iter().map(UnateSet::len).min() {
        rows.retain(|row| row.len() == minimum);
    }

    UnateCover {
        column_count: cover.column_count(),
        rows,
    }
}

fn unate_complement_rows(rows: &[UnateSet], column_count: usize) -> Vec<UnateSet> {
    if rows.is_empty() {
        return vec![UnateSet::empty()];
    }

    if rows.len() == 1 {
        return rows[0]
            .columns()
            .iter()
            .copied()
            .map(|column| UnateSet::from_columns([column]))
            .collect();
    }

    let min_size = rows.iter().map(UnateSet::len).min().unwrap_or(0);
    if min_size == 0 {
        return Vec::new();
    }

    let restricted = restricted_minimum_rows(rows, min_size);
    if min_size == 1 {
        let uncovered = rows_disjoint_from(rows, &restricted);
        let mut result = unate_complement_rows(&uncovered, column_count)
            .into_iter()
            .map(|row| row.union(&restricted))
            .collect::<Vec<_>>();
        retain_minimal(&mut result);
        return result;
    }

    let pick = select_restricted_column(rows, &restricted).unwrap_or(0);

    let covered = rows_without_column(rows, pick);
    let mut with_pick = unate_complement_rows(&covered, column_count)
        .into_iter()
        .map(|mut row| {
            row.insert(pick);
            row
        })
        .collect::<Vec<_>>();

    let without_pick_rows = rows_with_column_removed(rows, pick);
    let mut without_pick = unate_complement_rows(&without_pick_rows, column_count);
    with_pick.append(&mut without_pick);
    retain_minimal(&mut with_pick);
    with_pick
}

fn restricted_minimum_rows(rows: &[UnateSet], min_size: usize) -> UnateSet {
    let mut restricted = UnateSet::empty();
    for row in rows.iter().filter(|row| row.len() == min_size) {
        restricted = restricted.union(row);
    }

    restricted
}

fn rows_disjoint_from(rows: &[UnateSet], pick_set: &UnateSet) -> Vec<UnateSet> {
    rows.iter()
        .filter(|row| row.is_disjoint(pick_set))
        .cloned()
        .collect()
}

fn rows_without_column(rows: &[UnateSet], pick: usize) -> Vec<UnateSet> {
    rows.iter()
        .filter(|row| !row.contains(pick))
        .cloned()
        .collect()
}

fn rows_with_column_removed(rows: &[UnateSet], pick: usize) -> Vec<UnateSet> {
    rows.iter()
        .cloned()
        .map(|mut row| {
            row.remove(pick);
            row
        })
        .collect()
}

fn select_restricted_column(rows: &[UnateSet], restricted: &UnateSet) -> Option<usize> {
    let mut counts = vec![
        0usize;
        restricted
            .columns()
            .iter()
            .next_back()
            .copied()
            .unwrap_or(0)
            + 1
    ];
    for row in rows {
        if row.len() <= 1 {
            continue;
        }

        let weight = 1024 / (row.len() - 1);
        for column in row.columns().intersection(restricted.columns()) {
            if *column >= counts.len() {
                counts.resize(*column + 1, 0);
            }

            counts[*column] += weight;
        }
    }

    restricted.columns().iter().copied().max_by_key(|column| {
        (
            counts.get(*column).copied().unwrap_or(0),
            usize::MAX - *column,
        )
    })
}

fn retain_minimal(rows: &mut Vec<UnateSet>) {
    rows.sort();
    rows.dedup();
    let original = rows.clone();
    rows.retain(|row| {
        !original
            .iter()
            .any(|candidate| candidate != row && candidate.is_subset(row))
    });
    rows.sort_by_key(|row| (row.len(), row.columns().clone()));
}

fn retain_maximal(rows: &mut Vec<UnateSet>) {
    rows.sort();
    rows.dedup();
    let original = rows.clone();
    rows.retain(|row| {
        !original
            .iter()
            .any(|candidate| candidate != row && row.is_subset(candidate))
    });
    rows.sort_by_key(|row| (usize::MAX - row.len(), row.columns().clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complement_of_empty_family_is_empty_set() {
        let cover = UnateCover::from_rows(3, Vec::<Vec<usize>>::new()).unwrap();

        assert_eq!(rows(unate_compl(&cover)), vec![set(&[])]);
    }

    #[test]
    fn complement_of_single_row_is_singletons() {
        let cover = UnateCover::from_rows(4, vec![vec![1, 3]]).unwrap();

        assert_eq!(rows(unate_compl(&cover)), vec![set(&[1]), set(&[3])]);
    }

    #[test]
    fn complement_returns_minimal_hitting_sets() {
        let cover = UnateCover::from_rows(3, vec![vec![0, 1], vec![1, 2]]).unwrap();

        assert_eq!(rows(unate_compl(&cover)), vec![set(&[1]), set(&[0, 2])]);
    }

    #[test]
    fn complement_handles_essential_columns() {
        let cover = UnateCover::from_rows(3, vec![vec![0], vec![0, 1], vec![0, 2]]).unwrap();

        assert_eq!(rows(unate_compl(&cover)), vec![set(&[0])]);
    }

    #[test]
    fn exact_minimum_cover_filters_larger_minimal_covers() {
        let cover = UnateCover::from_rows(3, vec![vec![0, 1], vec![1, 2]]).unwrap();

        assert_eq!(rows(exact_minimum_cover(&cover)), vec![set(&[1])]);
    }

    #[test]
    fn intersect_keeps_maximal_intersections() {
        let left = UnateCover::from_rows(4, vec![vec![0, 1, 2], vec![2, 3]]).unwrap();
        let right = UnateCover::from_rows(4, vec![vec![1, 2], vec![2, 3]]).unwrap();

        assert_eq!(
            rows(unate_intersect(&left, &right, false).unwrap()),
            vec![set(&[1, 2]), set(&[2, 3])]
        );
    }

    #[test]
    fn intersect_largest_only_discards_smaller_intersections() {
        let left = UnateCover::from_rows(4, vec![vec![0, 1, 2], vec![2, 3]]).unwrap();
        let right = UnateCover::from_rows(4, vec![vec![1, 2], vec![2]]).unwrap();

        assert_eq!(
            rows(unate_intersect(&left, &right, true).unwrap()),
            vec![set(&[1, 2])]
        );
    }

    #[test]
    fn map_cover_to_unate_marks_missing_source_columns() {
        let cover = UnateCover::from_rows(5, vec![vec![0, 2], vec![1, 3]]).unwrap();
        let mappings = vec![
            UnateColumnMapping::new(0, [1]),
            UnateColumnMapping::new(3, [2, 4]),
        ];

        assert_eq!(
            rows(map_cover_to_unate(&cover, &mappings).unwrap()),
            vec![set(&[1]), set(&[0])]
        );
    }

    #[test]
    fn map_unate_to_cover_removes_selected_parts() {
        let unate = UnateCover::from_rows(2, vec![vec![0], vec![1], vec![0, 1]]).unwrap();
        let mappings = vec![
            UnateColumnMapping::new(0, [1]),
            UnateColumnMapping::new(3, [2, 4]),
        ];

        assert_eq!(
            rows(map_unate_to_cover(&unate, &mappings, 5).unwrap()),
            vec![set(&[0, 2, 3, 4]), set(&[0, 1, 3]), set(&[0, 3])]
        );
    }

    fn rows(cover: UnateCover) -> Vec<BTreeSet<usize>> {
        cover
            .into_rows()
            .into_iter()
            .map(UnateSet::into_columns)
            .collect()
    }

    fn set(columns: &[usize]) -> BTreeSet<usize> {
        columns.iter().copied().collect()
    }
}
