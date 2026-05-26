//! Native Rust packed-set and set-family support for Espresso.

use std::fmt;

const BITS_PER_WORD: usize = u32::BITS as usize;
const LOOP_MASK: u32 = 0x03ff;
const SIZE_SHIFT: u32 = 16;

pub const PRIME: u32 = 0x8000;
pub const NONESSEN: u32 = 0x4000;
pub const ACTIVE: u32 = 0x2000;
pub const REDUND: u32 = 0x1000;
pub const COVERED: u32 = 0x0800;
pub const RELESSEN: u32 = 0x0400;

#[derive(Clone)]
pub struct Set {
    words: Vec<u32>,
    element_count: usize,
}

impl Set {
    pub fn empty(element_count: usize) -> Self {
        let mut set = Self::with_capacity(element_count);
        set.clear();
        set
    }

    pub fn full(element_count: usize) -> Self {
        let mut set = Self::with_capacity(element_count);
        set.fill();
        set
    }

    pub fn from_elements(element_count: usize, elements: impl IntoIterator<Item = usize>) -> Self {
        let mut set = Self::empty(element_count);
        for element in elements {
            set.insert(element);
        }
        set
    }

    pub fn element_count(&self) -> usize {
        self.element_count
    }

    pub fn word_count(&self) -> usize {
        loop_index(self.element_count)
    }

    pub fn words(&self) -> &[u32] {
        &self.words
    }

    pub fn clear(&mut self) {
        self.words.fill(0);
        self.put_loop(self.word_count());
    }

    pub fn fill(&mut self) {
        let word_count = self.word_count();
        self.words.fill(0);
        self.put_loop(word_count);

        if word_count > 0 {
            for word in &mut self.words[1..=word_count] {
                *word = u32::MAX;
            }
        }

        if word_count > 0 {
            self.words[word_count] &= last_word_mask(self.element_count);
        }
    }

    pub fn contains(&self, element: usize) -> bool {
        assert!(
            element < self.element_count,
            "set element index {element} outside size {}",
            self.element_count
        );

        (self.words[word_index(element)] & bit_mask(element)) != 0
    }

    pub fn insert(&mut self, element: usize) -> bool {
        assert!(
            element < self.element_count,
            "set element index {element} outside size {}",
            self.element_count
        );

        let word = word_index(element);
        let mask = bit_mask(element);
        let was_present = (self.words[word] & mask) != 0;
        self.words[word] |= mask;
        !was_present
    }

    pub fn remove(&mut self, element: usize) -> bool {
        assert!(
            element < self.element_count,
            "set element index {element} outside size {}",
            self.element_count
        );

        let word = word_index(element);
        let mask = bit_mask(element);
        let was_present = (self.words[word] & mask) != 0;
        self.words[word] &= !mask;
        was_present
    }

    pub fn set_flag(&mut self, flag: u32) {
        self.words[0] |= flag;
    }

    pub fn reset_flag(&mut self, flag: u32) {
        self.words[0] &= !flag;
    }

    pub fn has_flag(&self, flag: u32) -> bool {
        (self.words[0] & flag) != 0
    }

    pub fn size_field(&self) -> u32 {
        self.words[0] >> SIZE_SHIFT
    }

    pub fn put_size_field(&mut self, size: u32) {
        self.words[0] = (self.words[0] & 0xffff) | (size << SIZE_SHIFT);
    }

    pub fn cardinality(&self) -> usize {
        self.data_words()
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    pub fn distance(&self, other: &Self) -> usize {
        self.assert_same_size(other);
        self.data_words()
            .iter()
            .zip(other.data_words())
            .map(|(left, right)| (left & right).count_ones() as usize)
            .sum()
    }

    pub fn intersection(&self, other: &Self) -> Self {
        self.binary_op(other, |left, right| left & right)
    }

    pub fn union(&self, other: &Self) -> Self {
        self.binary_op(other, |left, right| left | right)
    }

    pub fn difference(&self, other: &Self) -> Self {
        self.binary_op(other, |left, right| left & !right)
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self {
        self.binary_op(other, |left, right| left ^ right)
    }

    pub fn merge(&self, other: &Self, mask: &Self) -> Self {
        self.assert_same_size(other);
        self.assert_same_size(mask);

        let mut result = Self::empty(self.element_count);
        for index in 1..=self.word_count() {
            result.words[index] =
                (self.words[index] & mask.words[index]) | (other.words[index] & !mask.words[index]);
        }
        result.mask_unused_bits();
        result
    }

    pub fn intersection_nonempty(&self, other: &Self) -> (Self, bool) {
        let result = self.intersection(other);
        let nonempty = !result.is_empty();
        (result, nonempty)
    }

    pub fn union_nonempty(&self, other: &Self) -> (Self, bool) {
        let result = self.union(other);
        let nonempty = !result.is_empty();
        (result, nonempty)
    }

    pub fn is_empty(&self) -> bool {
        self.data_words().iter().all(|word| *word == 0)
    }

    pub fn is_full(&self) -> bool {
        let word_count = self.word_count();
        if word_count == 0 {
            return true;
        }

        if self.words[word_count] != last_word_mask(self.element_count) {
            return false;
        }

        self.words[1..word_count]
            .iter()
            .all(|word| *word == u32::MAX)
    }

    pub fn is_disjoint_from(&self, other: &Self) -> bool {
        self.assert_same_size(other);
        self.data_words()
            .iter()
            .zip(other.data_words())
            .all(|(left, right)| (left & right) == 0)
    }

    pub fn implies(&self, other: &Self) -> bool {
        self.assert_same_size(other);
        self.data_words()
            .iter()
            .zip(other.data_words())
            .all(|(left, right)| (left & !right) == 0)
    }

    pub fn elements(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.element_count).filter(|element| self.contains(*element))
    }

    pub fn adjust_counts(&self, counts: &mut [i32], weight: i32) {
        assert!(
            counts.len() >= self.element_count,
            "count table has {} entries for {} set elements",
            counts.len(),
            self.element_count
        );

        for element in self.elements() {
            counts[element] += weight;
        }
    }

    pub fn to_element_list_string(&self) -> String {
        let mut text = String::from("[");
        for (index, element) in self.elements().enumerate() {
            if index > 0 {
                text.push(',');
            }
            text.push_str(&element.to_string());
        }
        text.push(']');
        text
    }

    pub fn to_bit_string(&self, count: usize) -> String {
        assert!(
            count <= self.element_count,
            "bit string length {count} exceeds set size {}",
            self.element_count
        );

        (0..count)
            .map(|element| if self.contains(element) { '1' } else { '0' })
            .collect()
    }

    pub fn write_words(&self) -> String {
        self.words
            .iter()
            .map(|word| format!("{word:x}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn with_capacity(element_count: usize) -> Self {
        let word_count = loop_index(element_count);
        Self {
            words: vec![0; word_count + 1],
            element_count,
        }
    }

    fn data_words(&self) -> &[u32] {
        let word_count = self.word_count();
        if word_count == 0 {
            return &[];
        }

        &self.words[1..=word_count]
    }

    fn put_loop(&mut self, loop_value: usize) {
        assert!(loop_value <= LOOP_MASK as usize, "set is too large");
        self.words[0] = (self.words[0] & !LOOP_MASK) | loop_value as u32;
    }

    fn binary_op(&self, other: &Self, mut op: impl FnMut(u32, u32) -> u32) -> Self {
        self.assert_same_size(other);
        let mut result = Self::empty(self.element_count);
        for index in 1..=self.word_count() {
            result.words[index] = op(self.words[index], other.words[index]);
        }
        result.mask_unused_bits();
        result
    }

    fn mask_unused_bits(&mut self) {
        let word_count = self.word_count();
        if word_count > 0 {
            self.words[word_count] &= last_word_mask(self.element_count);
        }
    }

    fn assert_same_size(&self, other: &Self) {
        assert_eq!(
            self.element_count, other.element_count,
            "set sizes must match"
        );
    }
}

impl fmt::Debug for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Set")
            .field("element_count", &self.element_count)
            .field("elements", &self.elements().collect::<Vec<_>>())
            .field("flags", &(self.words[0] & !LOOP_MASK))
            .finish()
    }
}

impl PartialEq for Set {
    fn eq(&self, other: &Self) -> bool {
        self.element_count == other.element_count && self.data_words() == other.data_words()
    }
}

impl Eq for Set {}

#[derive(Clone, Debug)]
pub struct SetFamily {
    set_size: usize,
    capacity: usize,
    sets: Vec<Set>,
    active_count: usize,
}

impl SetFamily {
    pub fn new(capacity: usize, set_size: usize) -> Self {
        Self {
            set_size,
            capacity,
            sets: Vec::with_capacity(capacity),
            active_count: 0,
        }
    }

    pub fn from_sets(set_size: usize, sets: impl IntoIterator<Item = Set>) -> Self {
        let mut family = Self::new(0, set_size);
        for set in sets {
            family.add_set(set);
        }
        family
    }

    pub fn set_size(&self) -> usize {
        self.set_size
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    pub fn active_count(&self) -> usize {
        self.active_count
    }

    pub fn sets(&self) -> &[Set] {
        &self.sets
    }

    pub fn sets_mut(&mut self) -> &mut [Set] {
        &mut self.sets
    }

    pub fn active(&mut self) {
        for set in &mut self.sets {
            set.set_flag(ACTIVE);
        }
        self.active_count = self.sets.len();
    }

    pub fn remove_inactive(&mut self) {
        self.sets.retain(|set| set.has_flag(ACTIVE));
        self.active_count = self.sets.len();
    }

    pub fn add_set(&mut self, set: Set) {
        self.assert_set_size(&set);
        if self.sets.len() >= self.capacity {
            self.capacity = self.capacity + self.capacity / 2 + 1;
        }
        if set.has_flag(ACTIVE) {
            self.active_count += 1;
        }
        self.sets.push(set);
    }

    pub fn delete_set(&mut self, index: usize) -> Set {
        let removed = self.sets.swap_remove(index);
        if removed.has_flag(ACTIVE) {
            self.active_count -= 1;
        }
        removed
    }

    pub fn join(left: &Self, right: &Self) -> Self {
        left.assert_same_size(right);
        let mut result = Self::new(left.len() + right.len(), left.set_size);
        result.sets.extend(left.sets.iter().cloned());
        result.sets.extend(right.sets.iter().cloned());
        result.active_count = result
            .sets
            .iter()
            .filter(|set| set.has_flag(ACTIVE))
            .count();
        result
    }

    pub fn append(&mut self, mut other: Self) {
        self.assert_same_size(&other);
        self.capacity = self.sets.len() + other.sets.len();
        self.active_count += other.active_count;
        self.sets.append(&mut other.sets);
    }

    pub fn union_all(&self) -> Set {
        let mut result = Set::empty(self.set_size);
        for set in &self.sets {
            result = result.union(set);
        }
        result
    }

    pub fn intersection_all(&self) -> Set {
        let mut result = Set::full(self.set_size);
        for set in &self.sets {
            result = result.intersection(set);
        }
        result
    }

    pub fn count_columns(&self) -> Vec<i32> {
        let mut counts = vec![0; self.set_size];
        for set in &self.sets {
            set.adjust_counts(&mut counts, 1);
        }
        counts
    }

    pub fn count_columns_restricted(&self, restriction: &Set) -> Vec<i32> {
        self.assert_set_size(restriction);
        let mut counts = vec![0; self.set_size];
        for set in &self.sets {
            let denominator = set.cardinality().saturating_sub(1);
            if denominator == 0 {
                continue;
            }

            let weight = 1024 / denominator as i32;
            for element in set.intersection(restriction).elements() {
                counts[element] += weight;
            }
        }
        counts
    }

    pub fn delete_columns(self, first_column: usize, count: usize) -> Self {
        self.edit_columns(first_column, count as isize)
    }

    pub fn add_columns(self, first_column: usize, count: usize) -> Self {
        self.edit_columns(first_column, -(count as isize))
    }

    pub fn compress(self, columns: &Set) -> Self {
        self.assert_set_size(columns);
        let kept_columns = columns.elements().collect::<Vec<_>>();
        self.permute(&kept_columns)
    }

    pub fn transpose(self) -> Self {
        let mut result = Self::new(self.set_size, self.len());
        for column in 0..self.set_size {
            let mut transposed = Set::empty(self.len());
            for (row, set) in self.sets.iter().enumerate() {
                if set.contains(column) {
                    transposed.insert(row);
                }
            }
            result.add_set(transposed);
        }
        result
    }

    pub fn permute(self, permutation: &[usize]) -> Self {
        let mut result = Self::new(self.len(), permutation.len());
        for source in self.sets {
            let mut destination = Set::empty(permutation.len());
            for (destination_column, source_column) in permutation.iter().copied().enumerate() {
                if source.contains(source_column) {
                    destination.insert(destination_column);
                }
            }
            result.add_set(destination);
        }
        result
    }

    pub fn copy_column_from(
        &mut self,
        destination_column: usize,
        source: &Self,
        source_column: usize,
    ) {
        assert_eq!(
            self.len(),
            source.len(),
            "source and destination families must have the same row count"
        );
        assert!(destination_column < self.set_size);
        assert!(source_column < source.set_size);

        for (destination, source) in self.sets.iter_mut().zip(&source.sets) {
            if source.contains(source_column) {
                destination.insert(destination_column);
            }
        }
    }

    pub fn write_words(&self) -> String {
        let mut lines = vec![format!("{} {}", self.len(), self.set_size)];
        lines.extend(self.sets.iter().map(Set::write_words));
        lines.join("\n")
    }

    pub fn bit_matrix_string(&self) -> String {
        self.sets
            .iter()
            .enumerate()
            .map(|(index, set)| format!("[{index:4}] {}", set.to_bit_string(self.set_size)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn edit_columns(self, first_column: usize, delete_count: isize) -> Self {
        let destination_size = if delete_count >= 0 {
            self.set_size - delete_count as usize
        } else {
            self.set_size + (-delete_count) as usize
        };
        let mut result = Self::new(self.len(), destination_size);

        for source in self.sets {
            let mut destination = Set::empty(destination_size);
            for column in 0..first_column {
                if source.contains(column) {
                    destination.insert(column);
                }
            }

            let source_start = if delete_count > 0 {
                first_column + delete_count as usize
            } else {
                first_column
            };
            for source_column in source_start..self.set_size {
                if source.contains(source_column) {
                    let destination_column = (source_column as isize - delete_count) as usize;
                    destination.insert(destination_column);
                }
            }

            result.add_set(destination);
        }

        result
    }

    fn assert_set_size(&self, set: &Set) {
        assert_eq!(
            self.set_size,
            set.element_count(),
            "set size must match family set size"
        );
    }

    fn assert_same_size(&self, other: &Self) {
        assert_eq!(self.set_size, other.set_size, "set family sizes must match");
    }
}

impl PartialEq for SetFamily {
    fn eq(&self, other: &Self) -> bool {
        self.set_size == other.set_size && self.sets == other.sets
    }
}

impl Eq for SetFamily {}

pub fn bit_index(word: u32) -> Option<usize> {
    if word == 0 {
        None
    } else {
        Some(word.trailing_zeros() as usize)
    }
}

fn loop_index(element_count: usize) -> usize {
    if element_count == 0 {
        0
    } else {
        ((element_count - 1) / BITS_PER_WORD) + 1
    }
}

fn word_index(element: usize) -> usize {
    (element / BITS_PER_WORD) + 1
}

fn bit_mask(element: usize) -> u32 {
    1_u32 << (element % BITS_PER_WORD)
}

fn last_word_mask(element_count: usize) -> u32 {
    let used_bits = element_count % BITS_PER_WORD;
    if used_bits == 0 {
        u32::MAX
    } else {
        (1_u32 << used_bits) - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(size: usize, elements: &[usize]) -> Set {
        Set::from_elements(size, elements.iter().copied())
    }

    fn family(size: usize, rows: &[&[usize]]) -> SetFamily {
        SetFamily::from_sets(size, rows.iter().map(|row| set(size, row)))
    }

    #[test]
    fn bit_index_reports_first_low_order_bit() {
        assert_eq!(bit_index(0), None);
        assert_eq!(bit_index(0b0001), Some(0));
        assert_eq!(bit_index(0b1000), Some(3));
    }

    #[test]
    fn set_fill_masks_unused_bits_in_last_word() {
        let set = Set::full(35);

        assert_eq!(set.cardinality(), 35);
        assert!(set.is_full());
        assert_eq!(set.words()[2], 0b111);
    }

    #[test]
    fn basic_set_operations_match_packed_set_semantics() {
        let left = set(40, &[0, 2, 33]);
        let right = set(40, &[2, 3, 33, 34]);
        let mask = set(40, &[0, 2, 3]);

        assert_eq!(left.cardinality(), 3);
        assert_eq!(left.distance(&right), 2);
        assert_eq!(left.intersection(&right), set(40, &[2, 33]));
        assert_eq!(left.union(&right), set(40, &[0, 2, 3, 33, 34]));
        assert_eq!(left.difference(&right), set(40, &[0]));
        assert_eq!(left.symmetric_difference(&right), set(40, &[0, 3, 34]));
        assert_eq!(left.merge(&right, &mask), set(40, &[0, 2, 33, 34]));
    }

    #[test]
    fn predicates_cover_empty_full_equal_disjoint_and_implies() {
        let empty = Set::empty(8);
        let full = Set::full(8);
        let small = set(8, &[1, 3]);
        let superset = set(8, &[1, 2, 3]);
        let disjoint = set(8, &[0, 4]);

        assert!(empty.is_empty());
        assert!(full.is_full());
        assert!(small.implies(&superset));
        assert!(!superset.implies(&small));
        assert!(small.is_disjoint_from(&disjoint));
        assert_eq!(small, set(8, &[1, 3]));
    }

    #[test]
    fn active_filter_removes_inactive_sets_and_tracks_count() {
        let mut family = family(8, &[&[1], &[2], &[3]]);
        family.active();
        family.sets_mut()[1].reset_flag(ACTIVE);
        family.remove_inactive();

        assert_eq!(family.active_count(), 2);
        assert_eq!(family.sets(), &[set(8, &[1]), set(8, &[3])]);
    }

    #[test]
    fn set_family_join_append_delete_and_aggregate_operations_work() {
        let mut left = family(8, &[&[0, 1], &[2]]);
        let right = family(8, &[&[1, 3]]);

        assert_eq!(SetFamily::join(&left, &right).len(), 3);
        left.append(right);
        assert_eq!(left.union_all(), set(8, &[0, 1, 2, 3]));
        assert_eq!(left.intersection_all(), Set::empty(8));

        let removed = left.delete_set(1);
        assert_eq!(removed, set(8, &[2]));
        assert_eq!(left.len(), 2);
    }

    #[test]
    fn column_counting_supports_plain_and_restricted_weights() {
        let family = family(6, &[&[0, 2, 3], &[2, 4], &[2, 3, 5]]);
        let restriction = set(6, &[2, 3]);

        assert_eq!(family.count_columns(), vec![1, 0, 3, 2, 1, 1]);
        assert_eq!(
            family.count_columns_restricted(&restriction),
            vec![0, 0, 2048, 1024, 0, 0]
        );
    }

    #[test]
    fn columns_can_be_deleted_inserted_compressed_transposed_and_permuted() {
        let original = family(5, &[&[0, 2, 4], &[1, 3]]);

        assert_eq!(
            original.clone().delete_columns(1, 2),
            family(3, &[&[0, 2], &[1]])
        );
        assert_eq!(
            original.clone().add_columns(2, 2),
            family(7, &[&[0, 4, 6], &[1, 5]])
        );
        assert_eq!(
            original.clone().compress(&set(5, &[0, 2, 4])),
            family(3, &[&[0, 1, 2], &[]])
        );
        assert_eq!(
            original.clone().transpose(),
            family(2, &[&[0], &[1], &[0], &[1], &[0]])
        );
        assert_eq!(original.permute(&[4, 0]), family(2, &[&[0, 1], &[]]));
    }

    #[test]
    fn copy_column_sets_destination_bits_without_clearing_existing_bits() {
        let source = family(4, &[&[0], &[1], &[0, 1]]);
        let mut destination = family(4, &[&[3], &[], &[]]);

        destination.copy_column_from(2, &source, 0);

        assert_eq!(destination, family(4, &[&[2, 3], &[], &[2]]));
    }

    #[test]
    fn printable_forms_match_set_and_bit_matrix_expectations() {
        let set = set(6, &[0, 3, 5]);
        let family = family(6, &[&[0, 3, 5]]);

        assert_eq!(set.to_element_list_string(), "[0,3,5]");
        assert_eq!(set.to_bit_string(6), "100101");
        assert_eq!(family.bit_matrix_string(), "[   0] 100101");
        assert_eq!(family.write_words(), "1 6\n1 29");
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("set.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
