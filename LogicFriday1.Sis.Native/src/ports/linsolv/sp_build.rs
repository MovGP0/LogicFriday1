use std::cmp;
use std::collections::BTreeMap;

pub type RealNumber = f64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SparseBuildError {
    Panic,
    OutOfBounds,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SparseStatus {
    Okay,
    SmallPivot,
    ZeroDiagonal,
    Singular,
    NoMemory,
    Panic,
    Fatal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementHandle {
    Element(usize),
    Trash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElementTemplate {
    pub element1: ElementHandle,
    pub element2: ElementHandle,
    pub element3_negated: ElementHandle,
    pub element4_negated: ElementHandle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixElement {
    pub row: usize,
    pub col: usize,
    pub real: RealNumber,
    pub fillin: bool,
    next_in_row: Option<usize>,
    next_in_col: Option<usize>,
}

impl MatrixElement {
    fn new(row: usize, col: usize, fillin: bool) -> Self {
        Self {
            row,
            col,
            real: 0.0,
            fillin,
            next_in_row: None,
            next_in_col: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SparseBuildMatrix {
    size: usize,
    allocated_size: usize,
    expandable: bool,
    elements: Vec<MatrixElement>,
    first_in_col: Vec<Option<usize>>,
    first_in_row: Vec<Option<usize>>,
    diag: Vec<Option<usize>>,
    positions: BTreeMap<(usize, usize), usize>,
    trash_can: MatrixElement,
    rows_linked: bool,
    needs_ordering: bool,
    factored: bool,
    complex: bool,
    previous_matrix_was_complex: bool,
    fillins: usize,
    error: SparseStatus,
    singular_row: usize,
    singular_col: usize,
}

impl SparseBuildMatrix {
    pub fn new(size: usize) -> Result<Self, SparseBuildError> {
        Self::with_expandable(size, false)
    }

    pub fn with_expandable(size: usize, expandable: bool) -> Result<Self, SparseBuildError> {
        if size == 0 && !expandable {
            return Err(SparseBuildError::Panic);
        }

        let allocated_size = cmp::max(size, 6);
        let vector_size = allocated_size + 1;

        Ok(Self {
            size,
            allocated_size,
            expandable,
            elements: Vec::new(),
            first_in_col: vec![None; vector_size],
            first_in_row: vec![None; vector_size],
            diag: vec![None; vector_size],
            positions: BTreeMap::new(),
            trash_can: MatrixElement::new(0, 0, false),
            rows_linked: false,
            needs_ordering: true,
            factored: false,
            complex: false,
            previous_matrix_was_complex: false,
            fillins: 0,
            error: SparseStatus::Okay,
            singular_row: 0,
            singular_col: 0,
        })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    pub fn fillin_count(&self) -> usize {
        self.fillins
    }

    pub fn error(&self) -> SparseStatus {
        self.error
    }

    pub fn needs_ordering(&self) -> bool {
        self.needs_ordering
    }

    pub fn rows_linked(&self) -> bool {
        self.rows_linked
    }

    pub fn trash_value(&self) -> RealNumber {
        self.trash_can.real
    }

    pub fn singular_location(&self) -> Option<(usize, usize)> {
        match self.error {
            SparseStatus::Singular | SparseStatus::ZeroDiagonal => {
                Some((self.singular_row, self.singular_col))
            }
            _ => None,
        }
    }

    pub fn set_singular_location(&mut self, row: usize, col: usize, error: SparseStatus) {
        self.singular_row = row;
        self.singular_col = col;
        self.error = error;
    }

    pub fn set_complex(&mut self, complex: bool) {
        self.complex = complex;
    }

    pub fn clear(&mut self) {
        for element in &mut self.elements {
            element.real = 0.0;
        }

        self.trash_can.real = 0.0;
        self.error = SparseStatus::Okay;
        self.factored = false;
        self.singular_row = 0;
        self.singular_col = 0;
        self.previous_matrix_was_complex = self.complex;
    }

    pub fn get_element(
        &mut self,
        row: usize,
        col: usize,
    ) -> Result<ElementHandle, SparseBuildError> {
        if row == 0 || col == 0 {
            return Ok(ElementHandle::Trash);
        }

        self.ensure_index(row, col)?;

        if row == col {
            if let Some(index) = self.diag[row] {
                return Ok(ElementHandle::Element(index));
            }
        }

        self.find_element_in_col(row, col, true)
    }

    pub fn find_element_in_col(
        &mut self,
        row: usize,
        col: usize,
        create_if_missing: bool,
    ) -> Result<ElementHandle, SparseBuildError> {
        if row == 0 || col == 0 {
            return Ok(ElementHandle::Trash);
        }

        self.ensure_index(row, col)?;

        if let Some(index) = self.positions.get(&(row, col)) {
            return Ok(ElementHandle::Element(*index));
        }

        if create_if_missing {
            Ok(ElementHandle::Element(self.create_element(row, col, false)))
        } else {
            Err(SparseBuildError::OutOfBounds)
        }
    }

    pub fn create_fillin(
        &mut self,
        row: usize,
        col: usize,
    ) -> Result<ElementHandle, SparseBuildError> {
        if row == 0 || col == 0 {
            return Ok(ElementHandle::Trash);
        }

        self.ensure_index(row, col)?;

        if let Some(index) = self.positions.get(&(row, col)) {
            return Ok(ElementHandle::Element(*index));
        }

        Ok(ElementHandle::Element(self.create_element(row, col, true)))
    }

    pub fn link_rows(&mut self) {
        for entry in &mut self.first_in_row {
            *entry = None;
        }

        for element in &mut self.elements {
            element.next_in_row = None;
        }

        for row in (1..=self.size).rev() {
            let mut row_entries = self.row_indices(row);
            row_entries.sort_by_key(|index| self.elements[*index].col);
            for index in row_entries.into_iter().rev() {
                self.elements[index].next_in_row = self.first_in_row[row];
                self.first_in_row[row] = Some(index);
            }
        }

        self.rows_linked = true;
    }

    pub fn get_admittance(
        &mut self,
        node1: usize,
        node2: usize,
    ) -> Result<ElementTemplate, SparseBuildError> {
        let mut element1 = self.get_element(node1, node1)?;
        let mut element2 = self.get_element(node2, node2)?;
        let element3_negated = self.get_element(node2, node1)?;
        let element4_negated = self.get_element(node1, node2)?;

        if node1 == 0 {
            std::mem::swap(&mut element1, &mut element2);
        }

        Ok(ElementTemplate {
            element1,
            element2,
            element3_negated,
            element4_negated,
        })
    }

    pub fn get_quad(
        &mut self,
        row1: usize,
        row2: usize,
        col1: usize,
        col2: usize,
    ) -> Result<ElementTemplate, SparseBuildError> {
        let mut element1 = self.get_element(row1, col1)?;
        let mut element2 = self.get_element(row2, col2)?;
        let element3_negated = self.get_element(row2, col1)?;
        let element4_negated = self.get_element(row1, col2)?;

        if element1 == ElementHandle::Trash {
            std::mem::swap(&mut element1, &mut element2);
        }

        Ok(ElementTemplate {
            element1,
            element2,
            element3_negated,
            element4_negated,
        })
    }

    pub fn get_ones(
        &mut self,
        pos: usize,
        neg: usize,
        eqn: usize,
    ) -> Result<ElementTemplate, SparseBuildError> {
        let template = ElementTemplate {
            element4_negated: self.get_element(neg, eqn)?,
            element3_negated: self.get_element(eqn, neg)?,
            element2: self.get_element(pos, eqn)?,
            element1: self.get_element(eqn, pos)?,
        };

        self.add_real_quad(template, 1.0);

        Ok(template)
    }

    pub fn add_real_element(&mut self, handle: ElementHandle, value: RealNumber) {
        match handle {
            ElementHandle::Element(index) => {
                self.elements[index].real += value;
            }
            ElementHandle::Trash => {
                self.trash_can.real += value;
            }
        }
    }

    pub fn add_real_quad(&mut self, template: ElementTemplate, value: RealNumber) {
        self.add_real_element(template.element1, value);
        self.add_real_element(template.element2, value);
        self.add_real_element(template.element3_negated, -value);
        self.add_real_element(template.element4_negated, -value);
    }

    pub fn value(&self, handle: ElementHandle) -> RealNumber {
        match handle {
            ElementHandle::Element(index) => self.elements[index].real,
            ElementHandle::Trash => self.trash_can.real,
        }
    }

    pub fn value_at(&self, row: usize, col: usize) -> Option<RealNumber> {
        self.positions
            .get(&(row, col))
            .map(|index| self.elements[*index].real)
    }

    pub fn column_entries(&self, col: usize) -> Vec<(usize, usize, RealNumber)> {
        let mut result = Vec::new();
        let mut current = self.first_in_col.get(col).copied().flatten();

        while let Some(index) = current {
            let element = &self.elements[index];
            result.push((element.row, element.col, element.real));
            current = element.next_in_col;
        }

        result
    }

    pub fn row_entries(&self, row: usize) -> Vec<(usize, usize, RealNumber)> {
        let mut result = Vec::new();
        let mut current = self.first_in_row.get(row).copied().flatten();

        while let Some(index) = current {
            let element = &self.elements[index];
            result.push((element.row, element.col, element.real));
            current = element.next_in_row;
        }

        result
    }

    fn ensure_index(&mut self, row: usize, col: usize) -> Result<(), SparseBuildError> {
        let required_size = cmp::max(row, col);
        if required_size <= self.size {
            return Ok(());
        }

        if !self.expandable {
            return Err(SparseBuildError::OutOfBounds);
        }

        self.enlarge_matrix(required_size);
        Ok(())
    }

    fn create_element(&mut self, row: usize, col: usize, fillin: bool) -> usize {
        let index = self.elements.len();
        let mut element = MatrixElement::new(row, col, fillin);

        element.next_in_col = self.first_in_col[col];
        self.first_in_col[col] = Some(index);

        self.elements.push(element);
        self.positions.insert((row, col), index);
        self.relink_column(col);

        if self.rows_linked {
            self.relink_row(row);
        }

        if row == col {
            self.diag[row] = Some(index);
        }

        if fillin {
            self.fillins += 1;
        } else {
            self.needs_ordering = true;
        }

        index
    }

    fn enlarge_matrix(&mut self, new_size: usize) {
        self.size = new_size;

        if new_size <= self.allocated_size {
            return;
        }

        let grown_size = cmp::max(
            new_size,
            self.allocated_size + (self.allocated_size + 1) / 2,
        );
        self.allocated_size = grown_size;
        self.first_in_col.resize(grown_size + 1, None);
        self.first_in_row.resize(grown_size + 1, None);
        self.diag.resize(grown_size + 1, None);
    }

    fn relink_column(&mut self, col: usize) {
        let mut column_entries = self.column_indices(col);
        column_entries.sort_by_key(|index| self.elements[*index].row);
        self.first_in_col[col] = None;

        for index in column_entries.into_iter().rev() {
            self.elements[index].next_in_col = self.first_in_col[col];
            self.first_in_col[col] = Some(index);
        }
    }

    fn relink_row(&mut self, row: usize) {
        let mut row_entries = self.row_indices(row);
        row_entries.sort_by_key(|index| self.elements[*index].col);
        self.first_in_row[row] = None;

        for index in row_entries.into_iter().rev() {
            self.elements[index].next_in_row = self.first_in_row[row];
            self.first_in_row[row] = Some(index);
        }
    }

    fn column_indices(&self, col: usize) -> Vec<usize> {
        self.positions
            .iter()
            .filter_map(|(&(row, entry_col), &index)| {
                (entry_col == col && row > 0).then_some(index)
            })
            .collect()
    }

    fn row_indices(&self, row: usize) -> Vec<usize> {
        self.positions
            .iter()
            .filter_map(|(&(entry_row, col), &index)| {
                (entry_row == row && col > 0).then_some(index)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creating_zero_sized_nonexpandable_matrix_is_panic() {
        assert_eq!(
            SparseBuildMatrix::new(0).unwrap_err(),
            SparseBuildError::Panic
        );
    }

    #[test]
    fn get_element_creates_and_reuses_diagonal_element() {
        let mut matrix = SparseBuildMatrix::new(3).unwrap();

        let first = matrix.get_element(2, 2).unwrap();
        matrix.add_real_element(first, 4.0);
        let second = matrix.get_element(2, 2).unwrap();

        assert_eq!(first, second);
        assert_eq!(matrix.value(second), 4.0);
        assert_eq!(matrix.element_count(), 1);
        assert!(matrix.needs_ordering());
    }

    #[test]
    fn column_links_are_kept_in_row_order() {
        let mut matrix = SparseBuildMatrix::new(4).unwrap();

        matrix.get_element(3, 2).unwrap();
        matrix.get_element(1, 2).unwrap();
        matrix.get_element(4, 2).unwrap();

        let rows: Vec<usize> = matrix
            .column_entries(2)
            .into_iter()
            .map(|(row, _, _)| row)
            .collect();

        assert_eq!(rows, vec![1, 3, 4]);
    }

    #[test]
    fn row_links_are_created_on_demand() {
        let mut matrix = SparseBuildMatrix::new(4).unwrap();

        matrix.get_element(2, 4).unwrap();
        matrix.get_element(2, 1).unwrap();

        assert!(matrix.row_entries(2).is_empty());

        matrix.link_rows();

        let cols: Vec<usize> = matrix
            .row_entries(2)
            .into_iter()
            .map(|(_, col, _)| col)
            .collect();

        assert_eq!(cols, vec![1, 4]);
        assert!(matrix.rows_linked());
    }

    #[test]
    fn new_elements_after_row_linking_are_spliced_into_rows() {
        let mut matrix = SparseBuildMatrix::new(4).unwrap();

        matrix.get_element(2, 4).unwrap();
        matrix.link_rows();
        matrix.get_element(2, 1).unwrap();

        let cols: Vec<usize> = matrix
            .row_entries(2)
            .into_iter()
            .map(|(_, col, _)| col)
            .collect();

        assert_eq!(cols, vec![1, 4]);
    }

    #[test]
    fn zero_row_or_column_uses_trash_can() {
        let mut matrix = SparseBuildMatrix::new(3).unwrap();

        let handle = matrix.get_element(0, 2).unwrap();
        matrix.add_real_element(handle, 3.5);

        assert_eq!(handle, ElementHandle::Trash);
        assert_eq!(matrix.trash_value(), 3.5);
        assert_eq!(matrix.element_count(), 0);
    }

    #[test]
    fn clear_resets_values_error_and_singularity() {
        let mut matrix = SparseBuildMatrix::new(2).unwrap();
        let handle = matrix.get_element(1, 1).unwrap();
        matrix.add_real_element(handle, 9.0);
        matrix.add_real_element(ElementHandle::Trash, 2.0);
        matrix.set_singular_location(1, 1, SparseStatus::Singular);

        matrix.clear();

        assert_eq!(matrix.value(handle), 0.0);
        assert_eq!(matrix.trash_value(), 0.0);
        assert_eq!(matrix.error(), SparseStatus::Okay);
        assert_eq!(matrix.singular_location(), None);
    }

    #[test]
    fn get_admittance_builds_four_terminal_stamp_handles() {
        let mut matrix = SparseBuildMatrix::new(3).unwrap();

        let template = matrix.get_admittance(1, 2).unwrap();
        matrix.add_real_quad(template, 2.0);

        assert_eq!(matrix.value_at(1, 1), Some(2.0));
        assert_eq!(matrix.value_at(2, 2), Some(2.0));
        assert_eq!(matrix.value_at(2, 1), Some(-2.0));
        assert_eq!(matrix.value_at(1, 2), Some(-2.0));
    }

    #[test]
    fn grounded_admittance_swaps_positive_template_elements() {
        let mut matrix = SparseBuildMatrix::new(3).unwrap();

        let template = matrix.get_admittance(0, 2).unwrap();
        matrix.add_real_quad(template, 1.0);

        assert_eq!(matrix.value_at(2, 2), Some(1.0));
        assert_eq!(matrix.trash_value(), -1.0);
    }

    #[test]
    fn get_quad_swaps_when_first_element_is_grounded() {
        let mut matrix = SparseBuildMatrix::new(3).unwrap();

        let template = matrix.get_quad(0, 2, 1, 3).unwrap();
        matrix.add_real_quad(template, 5.0);

        assert_eq!(matrix.value_at(2, 3), Some(5.0));
        assert_eq!(matrix.value_at(2, 1), Some(-5.0));
        assert_eq!(matrix.trash_value(), 0.0);
    }

    #[test]
    fn get_ones_adds_structural_unit_stamp() {
        let mut matrix = SparseBuildMatrix::new(4).unwrap();

        matrix.get_ones(1, 2, 3).unwrap();

        assert_eq!(matrix.value_at(3, 1), Some(1.0));
        assert_eq!(matrix.value_at(1, 3), Some(1.0));
        assert_eq!(matrix.value_at(3, 2), Some(-1.0));
        assert_eq!(matrix.value_at(2, 3), Some(-1.0));
    }

    #[test]
    fn expandable_matrix_grows_to_requested_element() {
        let mut matrix = SparseBuildMatrix::with_expandable(1, true).unwrap();

        let handle = matrix.get_element(4, 2).unwrap();
        matrix.add_real_element(handle, 7.0);

        assert_eq!(matrix.size(), 4);
        assert_eq!(matrix.value_at(4, 2), Some(7.0));
    }

    #[test]
    fn nonexpandable_matrix_rejects_out_of_range_element() {
        let mut matrix = SparseBuildMatrix::new(2).unwrap();

        assert_eq!(
            matrix.get_element(3, 1).unwrap_err(),
            SparseBuildError::OutOfBounds
        );
    }

    #[test]
    fn fillins_are_counted_and_do_not_force_reordering() {
        let mut matrix = SparseBuildMatrix::new(3).unwrap();
        matrix.link_rows();

        matrix.create_fillin(2, 3).unwrap();

        assert_eq!(matrix.fillin_count(), 1);
        assert_eq!(matrix.element_count(), 1);
        assert!(matrix.needs_ordering());
        assert_eq!(matrix.row_entries(2), vec![(2, 3, 0.0)]);
    }
}
