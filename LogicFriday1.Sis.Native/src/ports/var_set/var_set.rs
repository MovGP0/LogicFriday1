//! Port of `sis/var_set/var_set.c`.
//!
//! The C implementation stores a fixed-size set of boolean variables in
//! `unsigned int` words. This Rust port keeps the compact word representation
//! but exposes it through an owned `VarSet` value instead of raw allocation and
//! pointer-oriented result parameters.

use std::fmt::{self, Write};

const WORD_BITS: usize = u32::BITS as usize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VarSet {
    len: usize,
    data: Vec<u32>,
}

impl VarSet {
    pub fn new(len: usize) -> Self {
        Self {
            len,
            data: vec![0; len.div_ceil(WORD_BITS)],
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty_len(&self) -> bool {
        self.len == 0
    }

    pub fn count(&self) -> usize {
        self.data
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    pub fn assign_from(&mut self, source: &Self) {
        self.assert_same_len(source);
        self.data.copy_from_slice(&source.data);
    }

    pub fn union(&self, other: &Self) -> Self {
        self.assert_same_len(other);
        let data = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a | b)
            .collect();

        Self {
            len: self.len,
            data,
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        self.assert_same_len(other);
        let data = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| a & b)
            .collect();

        Self {
            len: self.len,
            data,
        }
    }

    pub fn complement(&self) -> Self {
        let mut data: Vec<u32> = self.data.iter().map(|word| !word).collect();
        Self::mask_unused_tail_bits(self.len, &mut data);

        Self {
            len: self.len,
            data,
        }
    }

    pub fn contains(&self, index: usize) -> bool {
        self.assert_index(index);
        let (word, bit) = Self::word_bit(index);
        self.data[word] & (1u32 << bit) != 0
    }

    pub fn insert(&mut self, index: usize) {
        self.assert_index(index);
        let (word, bit) = Self::word_bit(index);
        self.data[word] |= 1u32 << bit;
    }

    pub fn remove(&mut self, index: usize) {
        self.assert_index(index);
        let (word, bit) = Self::word_bit(index);
        self.data[word] &= !(1u32 << bit);
    }

    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.assert_same_len(other);
        self.data
            .iter()
            .zip(other.data.iter())
            .any(|(a, b)| a & b != 0)
    }

    pub fn has_no_members(&self) -> bool {
        self.data.iter().all(|word| *word == 0)
    }

    pub fn is_full(&self) -> bool {
        self.count() == self.len
    }

    pub fn bit_string(&self) -> String {
        let mut output = String::new();
        self.write_bits(&mut output)
            .expect("writing to a String should not fail");
        output
    }

    pub fn write_bits(&self, writer: &mut impl Write) -> fmt::Result {
        for index in 0..self.len {
            writer.write_char(if self.contains(index) { '1' } else { '0' })?;
            writer.write_char(' ')?;
        }
        writer.write_char('\n')
    }

    pub fn compare_words(&self, other: &Self) -> std::cmp::Ordering {
        self.assert_same_len(other);
        self.data.cmp(&other.data)
    }

    pub fn sis_cmp(&self, other: &Self) -> i32 {
        i32::from(self != other)
    }

    pub fn sis_hash(&self) -> u32 {
        self.data
            .iter()
            .fold(0u32, |result, word| result.wrapping_add(*word))
    }

    pub fn words(&self) -> &[u32] {
        &self.data
    }

    fn assert_same_len(&self, other: &Self) {
        assert_eq!(self.len, other.len);
    }

    fn assert_index(&self, index: usize) {
        assert!(
            index < self.len,
            "var-set index {index} out of bounds for length {}",
            self.len
        );
    }

    fn word_bit(index: usize) -> (usize, usize) {
        (index / WORD_BITS, index % WORD_BITS)
    }

    fn mask_unused_tail_bits(len: usize, data: &mut [u32]) {
        let used_tail_bits = len % WORD_BITS;
        if used_tail_bits == 0 {
            return;
        }

        if let Some(last) = data.last_mut() {
            *last &= (1u32 << used_tail_bits) - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty_and_tracks_inserted_members() {
        let mut set = VarSet::new(65);

        assert_eq!(set.len(), 65);
        assert_eq!(set.words().len(), 3);
        assert!(set.has_no_members());

        set.insert(0);
        set.insert(32);
        set.insert(64);

        assert!(set.contains(0));
        assert!(set.contains(32));
        assert!(set.contains(64));
        assert_eq!(set.count(), 3);
        assert_eq!(set.sis_hash(), 3);

        set.remove(32);

        assert!(!set.contains(32));
        assert_eq!(set.count(), 2);
    }

    #[test]
    fn combines_sets_with_same_length() {
        let mut a = VarSet::new(6);
        let mut b = VarSet::new(6);
        a.insert(1);
        a.insert(4);
        b.insert(4);
        b.insert(5);

        let union = a.union(&b);
        let intersection = a.intersect(&b);

        assert_eq!(union.bit_string(), "0 1 0 0 1 1 \n");
        assert_eq!(intersection.bit_string(), "0 0 0 0 1 0 \n");
        assert!(a.overlaps(&b));
    }

    #[test]
    fn complement_masks_unused_tail_bits() {
        let mut set = VarSet::new(33);
        set.insert(32);

        let complement = set.complement();

        assert_eq!(complement.count(), 32);
        assert!(complement.contains(0));
        assert!(!complement.contains(32));
        assert_eq!(complement.words()[1], 0);
    }

    #[test]
    fn detects_full_and_clear_states() {
        let mut set = VarSet::new(3);

        assert!(!set.is_full());

        set.insert(0);
        set.insert(1);
        set.insert(2);

        assert!(set.is_full());

        set.clear();

        assert!(set.has_no_members());
        assert!(!set.is_full());
    }

    #[test]
    fn assignment_and_sis_comparison_match_c_semantics() {
        let mut source = VarSet::new(4);
        let mut target = VarSet::new(4);
        source.insert(2);

        target.assign_from(&source);

        assert_eq!(target, source);
        assert_eq!(target.sis_cmp(&source), 0);

        target.insert(3);

        assert_ne!(target, source);
        assert_eq!(target.sis_cmp(&source), 1);
    }
}
