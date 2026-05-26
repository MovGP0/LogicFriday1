//! Shared value-cell support for extraction sparse-matrix payloads.
//!
//! Value cells record the literal cost and origin of sparse-matrix entries used
//! by extraction. The original implementation stored the cells behind raw
//! pointers with a manual reference count. This port uses shared Rust ownership
//! for the same lifetime behavior: cloning a handle shares one cell, and
//! clearing the matrix payload slots releases the corresponding references.

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCell {
    value: i32,
    sis_index: i32,
    cube_number: i32,
}

impl ValueCell {
    pub fn new() -> Self {
        Self {
            value: 1,
            sis_index: -1,
            cube_number: 0,
        }
    }

    pub fn with_origin(value: i32, sis_index: i32, cube_number: i32) -> Self {
        Self {
            value,
            sis_index,
            cube_number,
        }
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn set_value(&mut self, value: i32) {
        self.value = value;
    }

    pub fn sis_index(&self) -> i32 {
        self.sis_index
    }

    pub fn set_sis_index(&mut self, sis_index: i32) {
        self.sis_index = sis_index;
    }

    pub fn cube_number(&self) -> i32 {
        self.cube_number
    }

    pub fn set_cube_number(&mut self, cube_number: i32) {
        self.cube_number = cube_number;
    }
}

impl Default for ValueCell {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct ValueCellHandle {
    cell: Rc<RefCell<ValueCell>>,
}

impl ValueCellHandle {
    pub fn new() -> Self {
        Self::from_cell(ValueCell::new())
    }

    pub fn from_cell(cell: ValueCell) -> Self {
        Self {
            cell: Rc::new(RefCell::new(cell)),
        }
    }

    pub fn snapshot(&self) -> ValueCell {
        self.cell.borrow().clone()
    }

    pub fn value(&self) -> i32 {
        self.cell.borrow().value()
    }

    pub fn set_value(&self, value: i32) {
        self.cell.borrow_mut().set_value(value);
    }

    pub fn sis_index(&self) -> i32 {
        self.cell.borrow().sis_index()
    }

    pub fn set_sis_index(&self, sis_index: i32) {
        self.cell.borrow_mut().set_sis_index(sis_index);
    }

    pub fn cube_number(&self) -> i32 {
        self.cell.borrow().cube_number()
    }

    pub fn set_cube_number(&self, cube_number: i32) {
        self.cell.borrow_mut().set_cube_number(cube_number);
    }

    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.cell)
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cell, &other.cell)
    }
}

impl Default for ValueCellHandle {
    fn default() -> Self {
        Self::new()
    }
}

pub fn release_value_cell(cell: ValueCellHandle) {
    drop(cell);
}

pub fn release_value_cells<'a, I>(cells: I) -> usize
where
    I: IntoIterator<Item = &'a mut Option<ValueCellHandle>>,
{
    let mut released = 0;
    for cell in cells {
        if cell.take().is_some() {
            released += 1;
        }
    }

    released
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cell_uses_sis_defaults() {
        let cell = ValueCell::new();

        assert_eq!(cell.value(), 1);
        assert_eq!(cell.sis_index(), -1);
        assert_eq!(cell.cube_number(), 0);
    }

    #[test]
    fn handle_clones_share_one_mutable_cell() {
        let cell = ValueCellHandle::new();
        let clone = cell.clone();

        assert!(cell.ptr_eq(&clone));
        assert_eq!(cell.strong_count(), 2);

        clone.set_value(7);
        clone.set_sis_index(3);
        clone.set_cube_number(11);

        assert_eq!(cell.snapshot(), ValueCell::with_origin(7, 3, 11));
    }

    #[test]
    fn releasing_one_handle_keeps_shared_cell_alive() {
        let cell = ValueCellHandle::new();
        let clone = cell.clone();

        release_value_cell(clone);

        assert_eq!(cell.strong_count(), 1);
        assert_eq!(cell.value(), 1);
    }

    #[test]
    fn releasing_slots_drops_each_present_payload_reference() {
        let shared = ValueCellHandle::new();
        let mut slots = vec![Some(shared.clone()), None, Some(shared.clone())];

        assert_eq!(shared.strong_count(), 3);
        assert_eq!(release_value_cells(slots.iter_mut()), 2);

        assert!(slots.iter().all(Option::is_none));
        assert_eq!(shared.strong_count(), 1);
    }
}
