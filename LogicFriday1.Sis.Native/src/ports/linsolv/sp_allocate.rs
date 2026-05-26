#![allow(dead_code)]

const DEFAULT_THRESHOLD: f64 = 1.0e-3;
const SPACE_FOR_ELEMENTS: usize = 6;
const SPACE_FOR_FILL_INS: usize = 4;
const ELEMENTS_PER_ALLOCATION: usize = 31;
const MINIMUM_ALLOCATED_SIZE: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparseError {
    Okay,
    SmallPivot,
    ZeroDiagonal,
    Singular,
    NoMemory,
    Panic,
}

impl SparseError {
    pub fn is_singular_location_error(self) -> bool {
        matches!(self, Self::Singular | Self::ZeroDiagonal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatrixElement {
    pub real: f64,
    pub row: usize,
    pub col: usize,
    pub next_in_row: Option<ElementHandle>,
    pub next_in_col: Option<ElementHandle>,
}

impl Default for MatrixElement {
    fn default() -> Self {
        Self {
            real: 0.0,
            row: 0,
            col: 0,
            next_in_row: None,
            next_in_col: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementHandle(usize);

impl ElementHandle {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementOrigin {
    Element,
    Fillin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocatedElement {
    pub handle: ElementHandle,
    pub origin: ElementOrigin,
}

#[derive(Debug, Clone)]
pub struct SparseMatrix {
    complex: bool,
    previous_matrix_was_complex: bool,
    factored: bool,
    elements: usize,
    error: SparseError,
    fillins: usize,
    reordered: bool,
    needs_ordering: bool,
    number_of_interchanges_is_odd: bool,
    partitioned: bool,
    rows_linked: bool,
    internal_vectors_allocated: bool,
    singular_col: usize,
    singular_row: usize,
    size: usize,
    allocated_size: usize,
    ext_size: usize,
    allocated_ext_size: usize,
    current_size: usize,
    ext_to_int_col_map: Option<Vec<isize>>,
    ext_to_int_row_map: Option<Vec<isize>>,
    int_to_ext_col_map: Vec<usize>,
    int_to_ext_row_map: Vec<usize>,
    diag: Vec<Option<ElementHandle>>,
    first_in_col: Vec<Option<ElementHandle>>,
    first_in_row: Vec<Option<ElementHandle>>,
    markowitz_row: Option<Vec<usize>>,
    markowitz_col: Option<Vec<usize>>,
    markowitz_prod: Option<Vec<usize>>,
    do_complex_direct: Option<Vec<bool>>,
    do_real_direct: Option<Vec<bool>>,
    intermediate: Option<Vec<f64>>,
    rel_threshold: f64,
    abs_threshold: f64,
    trash_can: MatrixElement,
    element_pool: Vec<MatrixElement>,
    fillin_pool: Vec<MatrixElement>,
    elements_remaining: usize,
    fillins_remaining: usize,
    next_avail_element: usize,
    next_avail_fillin: usize,
    element_blocks: Vec<(usize, usize)>,
    fillin_blocks: Vec<(usize, usize)>,
    allocation_records: usize,
    records_remaining: usize,
}

impl SparseMatrix {
    pub fn create(size: usize, complex: bool) -> Result<Self, SparseError> {
        if size == 0 {
            return Err(SparseError::Panic);
        }

        if complex {
            return Err(SparseError::Panic);
        }

        let allocated_size = size.max(MINIMUM_ALLOCATED_SIZE);
        let vector_len = allocated_size + 1;

        let mut matrix = Self {
            complex,
            previous_matrix_was_complex: complex,
            factored: false,
            elements: 0,
            error: SparseError::Okay,
            fillins: 0,
            reordered: false,
            needs_ordering: true,
            number_of_interchanges_is_odd: false,
            partitioned: false,
            rows_linked: false,
            internal_vectors_allocated: false,
            singular_col: 0,
            singular_row: 0,
            size,
            allocated_size,
            ext_size: size,
            allocated_ext_size: allocated_size,
            current_size: 0,
            ext_to_int_col_map: None,
            ext_to_int_row_map: None,
            int_to_ext_col_map: vec![0; vector_len],
            int_to_ext_row_map: vec![0; vector_len],
            diag: vec![None; vector_len],
            first_in_col: vec![None; vector_len],
            first_in_row: vec![None; vector_len],
            markowitz_row: None,
            markowitz_col: None,
            markowitz_prod: None,
            do_complex_direct: None,
            do_real_direct: None,
            intermediate: None,
            rel_threshold: DEFAULT_THRESHOLD,
            abs_threshold: 0.0,
            trash_can: MatrixElement::default(),
            element_pool: Vec::new(),
            fillin_pool: Vec::new(),
            elements_remaining: 0,
            fillins_remaining: 0,
            next_avail_element: 0,
            next_avail_fillin: 0,
            element_blocks: Vec::new(),
            fillin_blocks: Vec::new(),
            allocation_records: 0,
            records_remaining: 0,
        };

        for index in 1..=allocated_size {
            matrix.int_to_ext_row_map[index] = index;
            matrix.int_to_ext_col_map[index] = index;
        }

        matrix.record_allocation();
        matrix.record_allocation();
        matrix.record_allocation();
        matrix.record_allocation();
        matrix.record_allocation();

        matrix.initialize_element_blocks(
            SPACE_FOR_ELEMENTS * allocated_size,
            SPACE_FOR_FILL_INS * allocated_size,
        );

        Ok(matrix)
    }

    pub fn destroy(self) {
        drop(self);
    }

    pub fn get_element(&mut self) -> Result<AllocatedElement, SparseError> {
        if self.elements_remaining == 0 {
            self.allocate_element_block(ELEMENTS_PER_ALLOCATION);
        }

        let handle = ElementHandle(self.next_avail_element);
        self.next_avail_element += 1;
        self.elements_remaining -= 1;

        Ok(AllocatedElement {
            handle,
            origin: ElementOrigin::Element,
        })
    }

    pub fn get_fillin(&mut self) -> Result<AllocatedElement, SparseError> {
        if self.fillins_remaining == 0 {
            return self.get_element();
        }

        let handle = ElementHandle(self.next_avail_fillin);
        self.next_avail_fillin += 1;
        self.fillins_remaining -= 1;

        Ok(AllocatedElement {
            handle,
            origin: ElementOrigin::Fillin,
        })
    }

    pub fn error(&self) -> SparseError {
        self.error
    }

    pub fn null_matrix_error() -> SparseError {
        SparseError::NoMemory
    }

    pub fn where_singular(&self) -> (usize, usize) {
        if self.error.is_singular_location_error() {
            (self.singular_row, self.singular_col)
        } else {
            (0, 0)
        }
    }

    pub fn get_size(&self, external: bool) -> usize {
        if external { self.ext_size } else { self.size }
    }

    pub fn set_real(&mut self) {
        self.complex = false;
    }

    pub fn set_complex(&mut self) -> Result<(), SparseError> {
        self.error = SparseError::Panic;
        Err(SparseError::Panic)
    }

    pub fn fillin_count(&self) -> usize {
        self.fillins
    }

    pub fn element_count(&self) -> usize {
        self.elements
    }

    pub fn is_complex(&self) -> bool {
        self.complex
    }

    pub fn is_factored(&self) -> bool {
        self.factored
    }

    pub fn needs_ordering(&self) -> bool {
        self.needs_ordering
    }

    pub fn allocated_size(&self) -> usize {
        self.allocated_size
    }

    pub fn allocated_ext_size(&self) -> usize {
        self.allocated_ext_size
    }

    pub fn current_size(&self) -> usize {
        self.current_size
    }

    pub fn trash_can(&self) -> MatrixElement {
        self.trash_can
    }

    pub fn int_to_ext_col_map(&self) -> &[usize] {
        &self.int_to_ext_col_map
    }

    pub fn int_to_ext_row_map(&self) -> &[usize] {
        &self.int_to_ext_row_map
    }

    pub fn ext_to_int_col_map(&self) -> Option<&[isize]> {
        self.ext_to_int_col_map.as_deref()
    }

    pub fn ext_to_int_row_map(&self) -> Option<&[isize]> {
        self.ext_to_int_row_map.as_deref()
    }

    pub fn diag(&self) -> &[Option<ElementHandle>] {
        &self.diag
    }

    pub fn first_in_col(&self) -> &[Option<ElementHandle>] {
        &self.first_in_col
    }

    pub fn first_in_row(&self) -> &[Option<ElementHandle>] {
        &self.first_in_row
    }

    pub fn rel_threshold(&self) -> f64 {
        self.rel_threshold
    }

    pub fn abs_threshold(&self) -> f64 {
        self.abs_threshold
    }

    pub fn elements_remaining(&self) -> usize {
        self.elements_remaining
    }

    pub fn fillins_remaining(&self) -> usize {
        self.fillins_remaining
    }

    pub fn element_blocks(&self) -> &[(usize, usize)] {
        &self.element_blocks
    }

    pub fn fillin_blocks(&self) -> &[(usize, usize)] {
        &self.fillin_blocks
    }

    pub fn allocation_record_count(&self) -> usize {
        self.allocation_records
    }

    pub fn allocation_records_remaining(&self) -> usize {
        self.records_remaining
    }

    pub fn set_singular_location_for_test(&mut self, error: SparseError, row: usize, col: usize) {
        self.error = error;
        self.singular_row = row;
        self.singular_col = col;
    }

    fn initialize_element_blocks(
        &mut self,
        initial_number_of_elements: usize,
        number_of_fillins_expected: usize,
    ) {
        self.allocate_element_block(initial_number_of_elements);
        self.allocate_fillin_block(number_of_fillins_expected);
    }

    fn allocate_element_block(&mut self, count: usize) {
        let start = self.element_pool.len();
        self.element_pool
            .resize(start + count, MatrixElement::default());
        self.elements_remaining = count;
        self.next_avail_element = start;
        self.element_blocks.push((start, count));
        self.record_allocation();
    }

    fn allocate_fillin_block(&mut self, count: usize) {
        let start = self.fillin_pool.len();
        self.fillin_pool
            .resize(start + count, MatrixElement::default());
        self.fillins_remaining = count;
        self.next_avail_fillin = start;
        self.fillin_blocks.push((start, count));
        self.record_allocation();
    }

    fn record_allocation(&mut self) {
        if self.records_remaining == 0 {
            self.allocate_allocation_record_block();
        }

        self.allocation_records += 1;
        self.records_remaining -= 1;
    }

    fn allocate_allocation_record_block(&mut self) {
        self.records_remaining = ELEMENTS_PER_ALLOCATION;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_rejects_zero_size_when_not_expandable() {
        assert!(matches!(
            SparseMatrix::create(0, false),
            Err(SparseError::Panic)
        ));
    }

    #[test]
    fn create_rejects_complex_matrix_for_real_only_configuration() {
        assert!(matches!(
            SparseMatrix::create(3, true),
            Err(SparseError::Panic)
        ));
    }

    #[test]
    fn create_initializes_frame_defaults_and_minimum_allocation() {
        let matrix = SparseMatrix::create(3, false).unwrap();

        assert!(!matrix.is_complex());
        assert!(!matrix.is_factored());
        assert!(matrix.needs_ordering());
        assert_eq!(matrix.error(), SparseError::Okay);
        assert_eq!(matrix.get_size(false), 3);
        assert_eq!(matrix.get_size(true), 3);
        assert_eq!(matrix.allocated_size(), MINIMUM_ALLOCATED_SIZE);
        assert_eq!(matrix.allocated_ext_size(), MINIMUM_ALLOCATED_SIZE);
        assert_eq!(matrix.current_size(), 0);
        assert_eq!(matrix.rel_threshold(), DEFAULT_THRESHOLD);
        assert_eq!(matrix.abs_threshold(), 0.0);
        assert_eq!(matrix.trash_can(), MatrixElement::default());
    }

    #[test]
    fn create_initializes_one_based_internal_to_external_maps() {
        let matrix = SparseMatrix::create(8, false).unwrap();

        assert_eq!(matrix.int_to_ext_row_map()[0], 0);
        assert_eq!(matrix.int_to_ext_col_map()[0], 0);

        for index in 1..=8 {
            assert_eq!(matrix.int_to_ext_row_map()[index], index);
            assert_eq!(matrix.int_to_ext_col_map()[index], index);
        }
    }

    #[test]
    fn create_leaves_translation_maps_unallocated() {
        let matrix = SparseMatrix::create(2, false).unwrap();

        assert!(matrix.ext_to_int_col_map().is_none());
        assert!(matrix.ext_to_int_row_map().is_none());
    }

    #[test]
    fn create_allocates_pointer_vectors_with_one_extra_slot() {
        let matrix = SparseMatrix::create(9, false).unwrap();

        assert_eq!(matrix.diag().len(), 10);
        assert_eq!(matrix.first_in_col().len(), 10);
        assert_eq!(matrix.first_in_row().len(), 10);
        assert!(matrix.diag().iter().all(Option::is_none));
        assert!(matrix.first_in_col().iter().all(Option::is_none));
        assert!(matrix.first_in_row().iter().all(Option::is_none));
    }

    #[test]
    fn create_reserves_initial_element_and_fillin_blocks() {
        let matrix = SparseMatrix::create(3, false).unwrap();

        assert_eq!(
            matrix.element_blocks(),
            &[(0, SPACE_FOR_ELEMENTS * MINIMUM_ALLOCATED_SIZE)]
        );
        assert_eq!(
            matrix.fillin_blocks(),
            &[(0, SPACE_FOR_FILL_INS * MINIMUM_ALLOCATED_SIZE)]
        );
        assert_eq!(
            matrix.elements_remaining(),
            SPACE_FOR_ELEMENTS * MINIMUM_ALLOCATED_SIZE
        );
        assert_eq!(
            matrix.fillins_remaining(),
            SPACE_FOR_FILL_INS * MINIMUM_ALLOCATED_SIZE
        );
    }

    #[test]
    fn get_element_doles_out_from_initial_block() {
        let mut matrix = SparseMatrix::create(1, false).unwrap();

        let first = matrix.get_element().unwrap();
        let second = matrix.get_element().unwrap();

        assert_eq!(first.origin, ElementOrigin::Element);
        assert_eq!(second.origin, ElementOrigin::Element);
        assert_eq!(first.handle.index(), 0);
        assert_eq!(second.handle.index(), 1);
        assert_eq!(
            matrix.elements_remaining(),
            SPACE_FOR_ELEMENTS * MINIMUM_ALLOCATED_SIZE - 2
        );
    }

    #[test]
    fn get_element_allocates_another_block_after_initial_block_is_exhausted() {
        let mut matrix = SparseMatrix::create(1, false).unwrap();
        let initial_count = matrix.elements_remaining();

        for _ in 0..initial_count {
            matrix.get_element().unwrap();
        }

        let next = matrix.get_element().unwrap();

        assert_eq!(next.handle.index(), initial_count);
        assert_eq!(
            matrix.element_blocks(),
            &[
                (0, SPACE_FOR_ELEMENTS * MINIMUM_ALLOCATED_SIZE),
                (initial_count, ELEMENTS_PER_ALLOCATION),
            ]
        );
        assert_eq!(matrix.elements_remaining(), ELEMENTS_PER_ALLOCATION - 1);
    }

    #[test]
    fn get_fillin_uses_reserved_fillin_block_first() {
        let mut matrix = SparseMatrix::create(1, false).unwrap();

        let first = matrix.get_fillin().unwrap();

        assert_eq!(first.origin, ElementOrigin::Fillin);
        assert_eq!(first.handle.index(), 0);
        assert_eq!(
            matrix.fillins_remaining(),
            SPACE_FOR_FILL_INS * MINIMUM_ALLOCATED_SIZE - 1
        );
        assert_eq!(
            matrix.elements_remaining(),
            SPACE_FOR_ELEMENTS * MINIMUM_ALLOCATED_SIZE
        );
    }

    #[test]
    fn get_fillin_falls_back_to_element_block_when_reserved_fillins_are_exhausted() {
        let mut matrix = SparseMatrix::create(1, false).unwrap();
        let fillin_count = matrix.fillins_remaining();

        for _ in 0..fillin_count {
            matrix.get_fillin().unwrap();
        }

        let fallback = matrix.get_fillin().unwrap();

        assert_eq!(fallback.origin, ElementOrigin::Element);
        assert_eq!(fallback.handle.index(), 0);
        assert_eq!(matrix.fillins_remaining(), 0);
        assert_eq!(
            matrix.elements_remaining(),
            SPACE_FOR_ELEMENTS * MINIMUM_ALLOCATED_SIZE - 1
        );
    }

    #[test]
    fn where_singular_reports_location_only_for_singular_errors() {
        let mut matrix = SparseMatrix::create(2, false).unwrap();

        matrix.set_singular_location_for_test(SparseError::SmallPivot, 3, 4);
        assert_eq!(matrix.where_singular(), (0, 0));

        matrix.set_singular_location_for_test(SparseError::Singular, 3, 4);
        assert_eq!(matrix.where_singular(), (3, 4));

        matrix.set_singular_location_for_test(SparseError::ZeroDiagonal, 5, 6);
        assert_eq!(matrix.where_singular(), (5, 6));
    }

    #[test]
    fn set_real_and_set_complex_match_real_only_configuration() {
        let mut matrix = SparseMatrix::create(2, false).unwrap();

        matrix.set_real();
        assert!(!matrix.is_complex());
        assert_eq!(matrix.set_complex(), Err(SparseError::Panic));
        assert_eq!(matrix.error(), SparseError::Panic);
    }

    #[test]
    fn count_accessors_return_frame_counts() {
        let matrix = SparseMatrix::create(2, false).unwrap();

        assert_eq!(matrix.element_count(), 0);
        assert_eq!(matrix.fillin_count(), 0);
    }

    #[test]
    fn allocation_records_are_tracked_in_blocks() {
        let matrix = SparseMatrix::create(2, false).unwrap();

        assert_eq!(matrix.allocation_record_count(), 7);
        assert_eq!(
            matrix.allocation_records_remaining(),
            ELEMENTS_PER_ALLOCATION - 7
        );
    }

    #[test]
    fn null_matrix_error_matches_c_fallback() {
        assert_eq!(SparseMatrix::null_matrix_error(), SparseError::NoMemory);
    }
}
