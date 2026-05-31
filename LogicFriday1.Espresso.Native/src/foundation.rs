//! Low-level native Espresso foundations.
//!
//! This module ports the data structures and primitive behavior from Espresso's
//! `set.c`, `setc.c`, `cubestr.c`, and the block partition helper from
//! `part.c` into safe Rust data types.  The original C implementation uses
//! packed `unsigned int` arrays with metadata embedded in word zero; this port
//! keeps the same 32-bit packing and bit numbering while storing metadata in
//! normal Rust fields.

use std::cmp::Ordering;
use std::collections::{BTreeSet, VecDeque};

pub use crate::globals::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitSet {
    size: usize,
    words: Vec<u32>,
    flags: u32,
    stored_size: usize,
}

impl BitSet {
    pub fn new(size: usize) -> Self {
        Self::empty(size)
    }

    pub fn empty(size: usize) -> Self {
        Self {
            size,
            words: vec![0; word_count(size)],
            flags: 0,
            stored_size: 0,
        }
    }

    pub fn full(size: usize) -> Self {
        let mut set = Self::empty(size);
        set.fill();
        set
    }

    pub fn from_indices(size: usize, indices: impl IntoIterator<Item = usize>) -> Self {
        let mut set = Self::empty(size);
        for index in indices {
            set.insert(index);
        }
        set
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn loop_index(&self) -> usize {
        loopinit(self.size)
    }

    pub fn nelem(&self) -> usize {
        BPI * self.loop_index()
    }

    pub fn words(&self) -> &[u32] {
        &self.words
    }

    pub fn word(&self, word: usize) -> u32 {
        self.words[word - 1]
    }

    pub fn set_word(&mut self, word: usize, value: u32) {
        self.words[word - 1] = value;
        self.mask_unused_bits();
    }

    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    pub fn fill(&mut self) {
        self.words.fill(u32::MAX);
        self.mask_unused_bits();
    }

    pub fn insert(&mut self, element: usize) {
        assert!(
            element < self.size,
            "set element {element} out of range {}",
            self.size
        );
        self.words[element / BPI] |= 1u32 << (element % BPI);
    }

    pub fn remove(&mut self, element: usize) {
        assert!(
            element < self.size,
            "set element {element} out of range {}",
            self.size
        );
        self.words[element / BPI] &= !(1u32 << (element % BPI));
    }

    pub fn contains(&self, element: usize) -> bool {
        element < self.size && (self.words[element / BPI] & (1u32 << (element % BPI))) != 0
    }

    pub fn set_flag(&mut self, flag: u32) {
        self.flags |= flag;
    }

    pub fn reset_flag(&mut self, flag: u32) {
        self.flags &= !flag;
    }

    pub fn test_flag(&self, flag: u32) -> bool {
        (self.flags & flag) != 0
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn put_size(&mut self, size: usize) {
        self.stored_size = size;
    }

    pub fn stored_size(&self) -> usize {
        self.stored_size
    }

    pub fn ord(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    pub fn distance(&self, other: &Self) -> usize {
        self.assert_same_size(other);
        self.words
            .iter()
            .zip(&other.words)
            .map(|(a, b)| (a & b).count_ones() as usize)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|word| *word == 0)
    }

    pub fn is_full(&self) -> bool {
        self == &Self::full(self.size)
    }

    pub fn implies(&self, other: &Self) -> bool {
        self.assert_same_size(other);
        self.words
            .iter()
            .zip(&other.words)
            .all(|(a, b)| (a & !b) == 0)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.assert_same_size(other);
        self.words
            .iter()
            .zip(&other.words)
            .all(|(a, b)| (a & b) == 0)
    }

    pub fn and(&self, other: &Self) -> Self {
        self.binary_op(other, |a, b| a & b)
    }

    pub fn or(&self, other: &Self) -> Self {
        self.binary_op(other, |a, b| a | b)
    }

    pub fn diff(&self, other: &Self) -> Self {
        self.binary_op(other, |a, b| a & !b)
    }

    pub fn xor(&self, other: &Self) -> Self {
        self.binary_op(other, |a, b| a ^ b)
    }

    pub fn xnor(&self, other: &Self, fullset: &Self) -> Self {
        self.assert_same_size(other);
        self.assert_same_size(fullset);
        let mut result = Self::empty(self.size);
        for ((dst, (a, b)), full) in result
            .words
            .iter_mut()
            .zip(self.words.iter().zip(&other.words))
            .zip(&fullset.words)
        {
            *dst = full & !(a ^ b);
        }
        result
    }

    pub fn ndiff(&self, other: &Self, fullset: &Self) -> Self {
        self.assert_same_size(other);
        self.assert_same_size(fullset);
        let mut result = Self::empty(self.size);
        for ((dst, (a, b)), full) in result
            .words
            .iter_mut()
            .zip(self.words.iter().zip(&other.words))
            .zip(&fullset.words)
        {
            *dst = full & (a | !b);
        }
        result
    }

    pub fn merge(&self, other: &Self, mask: &Self) -> Self {
        self.assert_same_size(other);
        self.assert_same_size(mask);
        let mut result = Self::empty(self.size);
        for ((dst, (a, b)), mask_word) in result
            .words
            .iter_mut()
            .zip(self.words.iter().zip(&other.words))
            .zip(&mask.words)
        {
            *dst = (a & mask_word) | (b & !mask_word);
        }
        result.mask_unused_bits();
        result
    }

    pub fn elements(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.size).filter(|&element| self.contains(element))
    }

    pub fn to_element_string(&self) -> String {
        let values = self
            .elements()
            .map(|element| element.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("[{values}]")
    }

    pub fn to_bit_string(&self) -> String {
        (0..self.size)
            .map(|element| if self.contains(element) { '1' } else { '0' })
            .collect()
    }

    fn binary_op(&self, other: &Self, op: impl Fn(u32, u32) -> u32) -> Self {
        self.assert_same_size(other);
        let mut result = Self::empty(self.size);
        for (dst, (a, b)) in result
            .words
            .iter_mut()
            .zip(self.words.iter().zip(&other.words))
        {
            *dst = op(*a, *b);
        }
        result.mask_unused_bits();
        result
    }

    fn mask_unused_bits(&mut self) {
        let used = self.size % BPI;
        if used != 0 {
            if let Some(last) = self.words.last_mut() {
                *last &= (1u32 << used) - 1;
            }
        }
    }

    fn assert_same_size(&self, other: &Self) {
        assert_eq!(self.size, other.size, "set sizes differ");
    }
}

pub fn bit_index(value: u32) -> Option<usize> {
    if value == 0 {
        None
    } else {
        Some(value.trailing_zeros() as usize)
    }
}

pub fn which_word(element: usize) -> usize {
    (element >> LOGBPI) + 1
}

pub fn which_bit(element: usize) -> usize {
    element & (BPI - 1)
}

pub fn set_size(size: usize) -> usize {
    if size <= BPI {
        2
    } else {
        which_word(size - 1) + 1
    }
}

pub fn loopinit(size: usize) -> usize {
    if size <= BPI { 1 } else { which_word(size - 1) }
}

fn word_count(size: usize) -> usize {
    loopinit(size)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetFamily {
    set_size: usize,
    capacity: usize,
    sets: Vec<BitSet>,
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

    pub fn from_sets(set_size: usize, sets: impl IntoIterator<Item = BitSet>) -> Self {
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
        self.capacity.max(self.sets.capacity())
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

    pub fn sets(&self) -> &[BitSet] {
        &self.sets
    }

    pub fn sets_mut(&mut self) -> &mut [BitSet] {
        &mut self.sets
    }

    pub fn get(&self, index: usize) -> &BitSet {
        &self.sets[index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut BitSet {
        &mut self.sets[index]
    }

    pub fn add_set(&mut self, set: BitSet) {
        assert_eq!(set.size(), self.set_size, "set size does not match family");
        if self.sets.len() >= self.capacity {
            self.capacity = self.capacity + self.capacity / 2 + 1;
        }
        if set.test_flag(ACTIVE) {
            self.active_count += 1;
        }
        self.sets.push(set);
    }

    pub fn delete_set(&mut self, index: usize) -> BitSet {
        let removed = self.sets.swap_remove(index);
        if removed.test_flag(ACTIVE) {
            self.active_count -= 1;
        }
        removed
    }

    pub fn copy(&self) -> Self {
        self.clone()
    }

    pub fn append(&mut self, mut other: Self) {
        for set in other.sets.drain(..) {
            self.add_set(set);
        }
    }

    pub fn join(mut self, other: Self) -> Self {
        self.append(other);
        self
    }

    pub fn active(&self) -> Self {
        Self::from_sets(
            self.set_size,
            self.sets
                .iter()
                .filter(|set| set.test_flag(ACTIVE))
                .cloned(),
        )
    }

    pub fn inactive(&self) -> Self {
        Self::from_sets(
            self.set_size,
            self.sets
                .iter()
                .filter(|set| !set.test_flag(ACTIVE))
                .cloned(),
        )
    }

    pub fn union_all(&self) -> BitSet {
        self.sets
            .iter()
            .fold(BitSet::empty(self.set_size), |acc, set| acc.or(set))
    }

    pub fn intersect_all(&self) -> BitSet {
        let mut iter = self.sets.iter();
        let Some(first) = iter.next() else {
            return BitSet::empty(self.set_size);
        };
        iter.fold(first.clone(), |acc, set| acc.and(set))
    }

    pub fn count_columns(&self) -> Vec<usize> {
        let mut counts = vec![0; self.set_size];
        for set in &self.sets {
            for element in set.elements() {
                counts[element] += 1;
            }
        }
        counts
    }

    pub fn count_columns_restricted(&self, restriction: &BitSet) -> Vec<usize> {
        assert_eq!(
            restriction.size(),
            self.set_size,
            "restriction size differs"
        );
        let mut counts = vec![0; self.set_size];
        for set in &self.sets {
            let order = set.ord();
            if order <= 1 {
                continue;
            }
            let weight = 1024 / (order - 1);
            for element in set.and(restriction).elements() {
                counts[element] += weight;
            }
        }
        counts
    }

    pub fn add_columns(self, first_col: usize, count: usize) -> Self {
        if first_col == self.set_size && self.set_size + count <= BPI * loopinit(self.set_size) {
            return Self {
                set_size: self.set_size + count,
                capacity: self.capacity,
                sets: self
                    .sets
                    .into_iter()
                    .map(|set| resize_set(set, self.set_size + count))
                    .collect(),
                active_count: self.active_count,
            };
        }
        self.delete_columns(first_col, -(count as isize))
    }

    pub fn delete_columns(self, first_col: usize, count: isize) -> Self {
        let new_size = (self.set_size as isize - count) as usize;
        let mut result = Self::new(self.len(), new_size);
        for set in self.sets {
            let mut dst = BitSet::empty(new_size);
            for i in 0..first_col {
                if set.contains(i) {
                    dst.insert(i);
                }
            }
            let start = if count > 0 {
                first_col + count as usize
            } else {
                first_col
            };
            for i in start..set.size() {
                if set.contains(i) {
                    dst.insert((i as isize - count) as usize);
                }
            }
            if set.test_flag(ACTIVE) {
                dst.set_flag(ACTIVE);
            }
            result.add_set(dst);
        }
        result
    }

    pub fn compress(self, columns: &BitSet) -> Self {
        let mut result = Self::new(self.len(), columns.ord());
        let retained = columns.elements().collect::<Vec<_>>();
        for set in self.sets {
            let mut dst = BitSet::empty(retained.len());
            for (dst_col, src_col) in retained.iter().enumerate() {
                if set.contains(*src_col) {
                    dst.insert(dst_col);
                }
            }
            result.add_set(dst);
        }
        result
    }

    pub fn transpose(self) -> Self {
        let mut result = Self::new(self.set_size, self.len());
        for _ in 0..self.set_size {
            result.add_set(BitSet::empty(self.len()));
        }
        for (row, set) in self.sets.iter().enumerate() {
            for col in set.elements() {
                result.get_mut(col).insert(row);
            }
        }
        result
    }

    pub fn permute(self, columns: &[usize]) -> Self {
        let mut result = Self::new(self.len(), columns.len());
        for set in self.sets {
            let mut dst = BitSet::empty(columns.len());
            for (dst_col, src_col) in columns.iter().enumerate() {
                if set.contains(*src_col) {
                    dst.insert(dst_col);
                }
            }
            result.add_set(dst);
        }
        result
    }
}

fn resize_set(set: BitSet, new_size: usize) -> BitSet {
    let mut dst = BitSet::empty(new_size);
    for element in set.elements().filter(|element| *element < new_size) {
        dst.insert(element);
    }
    dst.flags = set.flags;
    dst.stored_size = set.stored_size;
    dst
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubeConfig {
    pub num_vars: usize,
    pub num_binary_vars: usize,
    pub part_size: Vec<isize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubeContext {
    pub size: usize,
    pub num_vars: usize,
    pub num_binary_vars: usize,
    pub first_part: Vec<usize>,
    pub last_part: Vec<usize>,
    pub part_size: Vec<usize>,
    pub first_word: Vec<usize>,
    pub last_word: Vec<usize>,
    pub binary_mask: BitSet,
    pub mv_mask: BitSet,
    pub var_mask: Vec<BitSet>,
    pub temp: Vec<BitSet>,
    pub fullset: BitSet,
    pub emptyset: BitSet,
    pub inmask: u32,
    pub inword: Option<usize>,
    pub sparse: Vec<bool>,
    pub num_mv_vars: usize,
    pub output: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CData {
    pub part_zeros: Vec<usize>,
    pub var_zeros: Vec<usize>,
    pub parts_active: Vec<usize>,
    pub is_unate: Vec<bool>,
    pub vars_active: usize,
    pub vars_unate: usize,
    pub best: Option<usize>,
}

impl CubeContext {
    pub fn setup(config: CubeConfig) -> Result<(Self, CData), String> {
        if config.num_vars < config.num_binary_vars {
            return Err("cube size is silly, error in .i/.o or .mv".to_string());
        }

        let num_mv_vars = config.num_vars - config.num_binary_vars;
        let output = (num_mv_vars > 0).then_some(config.num_vars - 1);
        let mut part_size = config.part_size;
        if part_size.len() < config.num_vars {
            part_size.resize(config.num_vars, 0);
        }

        let mut size = 0;
        let mut first_part = vec![0; config.num_vars];
        let mut last_part = vec![0; config.num_vars];
        let mut first_word = vec![0; config.num_vars];
        let mut last_word = vec![0; config.num_vars];
        let mut normalized_part_size = vec![0; config.num_vars];

        for var in 0..config.num_vars {
            if var < config.num_binary_vars {
                part_size[var] = 2;
            }
            let parts = part_size[var].unsigned_abs();
            if parts == 0 {
                return Err(format!("variable {var} has no parts"));
            }
            first_part[var] = size;
            first_word[var] = which_word(size);
            size += parts;
            last_part[var] = size - 1;
            last_word[var] = which_word(size - 1);
            normalized_part_size[var] = parts;
        }

        let mut binary_mask = BitSet::empty(size);
        let mut mv_mask = BitSet::empty(size);
        let mut var_mask = Vec::with_capacity(config.num_vars);
        let mut sparse = Vec::with_capacity(config.num_vars);

        for var in 0..config.num_vars {
            let mut mask = BitSet::empty(size);
            for part in first_part[var]..=last_part[var] {
                mask.insert(part);
            }
            if var < config.num_binary_vars {
                binary_mask = binary_mask.or(&mask);
                sparse.push(false);
            } else {
                mv_mask = mv_mask.or(&mask);
                sparse.push(true);
            }
            var_mask.push(mask);
        }

        let inword = (config.num_binary_vars > 0).then_some(last_word[config.num_binary_vars - 1]);
        let inmask = inword.map_or(0, |word| binary_mask.word(word) & DISJOINT);
        let fullset = BitSet::full(size);
        let emptyset = BitSet::empty(size);
        let temp = (0..CUBE_TEMP).map(|_| BitSet::empty(size)).collect();
        let cdata = CData {
            part_zeros: vec![0; size],
            var_zeros: vec![0; config.num_vars],
            parts_active: vec![0; config.num_vars],
            is_unate: vec![false; config.num_vars],
            vars_active: 0,
            vars_unate: 0,
            best: None,
        };

        Ok((
            Self {
                size,
                num_vars: config.num_vars,
                num_binary_vars: config.num_binary_vars,
                first_part,
                last_part,
                part_size: normalized_part_size,
                first_word,
                last_word,
                binary_mask,
                mv_mask,
                var_mask,
                temp,
                fullset,
                emptyset,
                inmask,
                inword,
                sparse,
                num_mv_vars,
                output,
            },
            cdata,
        ))
    }

    pub fn new_cube(&self) -> BitSet {
        BitSet::empty(self.size)
    }

    pub fn new_full_cube(&self) -> BitSet {
        BitSet::full(self.size)
    }

    pub fn get_input(cube: &BitSet, pos: usize) -> u8 {
        let bit = 2 * pos;
        ((cube.word(which_word(bit)) >> which_bit(bit)) & 3) as u8
    }

    pub fn put_input(cube: &mut BitSet, pos: usize, value: u8) {
        assert!(value <= 3, "binary cube input value out of range");
        let bit = 2 * pos;
        let word_index = which_word(bit);
        let shift = which_bit(bit);
        let word = (cube.word(word_index) & !(3u32 << shift)) | ((value as u32) << shift);
        cube.set_word(word_index, word);
    }

    pub fn get_output(&self, cube: &BitSet, pos: usize) -> bool {
        let output = self.output.expect("cube has no output variable");
        cube.contains(self.first_part[output] + pos)
    }

    pub fn put_output(&self, cube: &mut BitSet, pos: usize, value: bool) {
        let output = self.output.expect("cube has no output variable");
        let element = self.first_part[output] + pos;
        if value {
            cube.insert(element);
        } else {
            cube.remove(element);
        }
    }

    pub fn full_row(&self, p: &BitSet, cof: &BitSet) -> bool {
        p.or(cof) == self.fullset
    }

    pub fn cdist0(&self, a: &BitSet, b: &BitSet) -> bool {
        self.cdist(a, b) == 0
    }

    pub fn cdist01(&self, a: &BitSet, b: &BitSet) -> usize {
        self.cdist_limited(a, b, Some(1))
    }

    pub fn cdist(&self, a: &BitSet, b: &BitSet) -> usize {
        self.cdist_limited(a, b, None)
    }

    pub fn force_lower(&self, xlower: &mut BitSet, a: &BitSet, b: &BitSet) {
        if let Some(last) = self.inword {
            let x = disjoint_binary_word(a.word(last) & b.word(last), self.inmask);
            if x != 0 {
                let value = xlower.word(last) | ((x | (x << 1)) & a.word(last));
                xlower.set_word(last, value);
            }
            for word in 1..last {
                let x = disjoint_binary_word(a.word(word) & b.word(word), DISJOINT);
                if x != 0 {
                    let value = xlower.word(word) | ((x | (x << 1)) & a.word(word));
                    xlower.set_word(word, value);
                }
            }
        }

        for var in self.num_binary_vars..self.num_vars {
            if self.variable_intersects(a, b, var) {
                continue;
            }
            for part in self.first_part[var]..=self.last_part[var] {
                if a.contains(part) {
                    xlower.insert(part);
                }
            }
        }
    }

    pub fn consensus(&self, a: &BitSet, b: &BitSet) -> BitSet {
        let mut result = BitSet::empty(self.size);
        if let Some(last) = self.inword {
            let intersection = a.word(last) & b.word(last);
            let x = disjoint_binary_word(intersection, self.inmask);
            result.set_word(
                last,
                intersection | ((x | (x << 1)) & (a.word(last) | b.word(last))),
            );
            for word in 1..last {
                let intersection = a.word(word) & b.word(word);
                let x = disjoint_binary_word(intersection, DISJOINT);
                result.set_word(
                    word,
                    intersection | ((x | (x << 1)) & (a.word(word) | b.word(word))),
                );
            }
        }

        for var in self.num_binary_vars..self.num_vars {
            let mut empty = true;
            for part in self.first_part[var]..=self.last_part[var] {
                if a.contains(part) && b.contains(part) {
                    empty = false;
                    result.insert(part);
                }
            }
            if empty {
                for part in self.first_part[var]..=self.last_part[var] {
                    if a.contains(part) || b.contains(part) {
                        result.insert(part);
                    }
                }
            }
        }
        result
    }

    pub fn cactive(&self, a: &BitSet) -> Option<usize> {
        let mut active = None;
        let mut dist = 0usize;

        if let Some(last) = self.inword {
            let x = inactive_binary_word(a.word(last), self.inmask);
            if x != 0 {
                dist += x.count_ones() as usize;
                if dist > 1 {
                    return None;
                }
                active = Some((last - 1) * (BPI / 2) + bit_index(x).unwrap() / 2);
            }
            for word in 1..last {
                let x = inactive_binary_word(a.word(word), DISJOINT);
                if x != 0 {
                    dist += x.count_ones() as usize;
                    if dist > 1 {
                        return None;
                    }
                    active = Some((word - 1) * (BPI / 2) + bit_index(x).unwrap() / 2);
                }
            }
        }

        for var in self.num_binary_vars..self.num_vars {
            if (self.first_part[var]..=self.last_part[var]).any(|part| !a.contains(part)) {
                dist += 1;
                if dist > 1 {
                    return None;
                }
                active = Some(var);
            }
        }
        active
    }

    pub fn ccommon(&self, a: &BitSet, b: &BitSet, cof: &BitSet) -> bool {
        if let Some(last) = self.inword {
            let x = a.word(last) | cof.word(last);
            let y = b.word(last) | cof.word(last);
            if inactive_binary_word(x, self.inmask) & inactive_binary_word(y, self.inmask) != 0 {
                return true;
            }
            for word in 1..last {
                let x = a.word(word) | cof.word(word);
                let y = b.word(word) | cof.word(word);
                if inactive_binary_word(x, DISJOINT) & inactive_binary_word(y, DISJOINT) != 0 {
                    return true;
                }
            }
        }

        for var in self.num_binary_vars..self.num_vars {
            let missing_a = (self.first_part[var]..=self.last_part[var])
                .any(|part| !a.contains(part) && !cof.contains(part));
            let missing_b = (self.first_part[var]..=self.last_part[var])
                .any(|part| !b.contains(part) && !cof.contains(part));
            if missing_a && missing_b {
                return true;
            }
        }
        false
    }

    pub fn cvolume(&self, a: &BitSet) -> usize {
        (0..self.num_vars)
            .map(|var| {
                (self.first_part[var]..=self.last_part[var])
                    .filter(|part| a.contains(*part))
                    .count()
            })
            .product()
    }

    fn cdist_limited(&self, a: &BitSet, b: &BitSet, limit: Option<usize>) -> usize {
        let mut dist = 0usize;
        if let Some(last) = self.inword {
            let x = disjoint_binary_word(a.word(last) & b.word(last), self.inmask);
            dist += x.count_ones() as usize;
            if limit.is_some_and(|limit| dist > limit) {
                return limit.unwrap() + 1;
            }
            for word in 1..last {
                let x = disjoint_binary_word(a.word(word) & b.word(word), DISJOINT);
                dist += x.count_ones() as usize;
                if limit.is_some_and(|limit| dist > limit) {
                    return limit.unwrap() + 1;
                }
            }
        }

        for var in self.num_binary_vars..self.num_vars {
            if !self.variable_intersects(a, b, var) {
                dist += 1;
                if limit.is_some_and(|limit| dist > limit) {
                    return limit.unwrap() + 1;
                }
            }
        }
        dist
    }

    fn variable_intersects(&self, a: &BitSet, b: &BitSet, var: usize) -> bool {
        (self.first_part[var]..=self.last_part[var])
            .any(|part| a.contains(part) && b.contains(part))
    }
}

pub fn descend(a: &BitSet, b: &BitSet) -> Ordering {
    b.stored_size()
        .cmp(&a.stored_size())
        .then_with(|| lex_compare_desc(a, b))
}

pub fn ascend(a: &BitSet, b: &BitSet) -> Ordering {
    a.stored_size()
        .cmp(&b.stored_size())
        .then_with(|| lex_compare_asc(a, b))
}

pub fn lex_order(a: &BitSet, b: &BitSet) -> Ordering {
    lex_compare_desc(a, b)
}

pub fn d1_order(a: &BitSet, b: &BitSet, mask: &BitSet) -> Ordering {
    for word in (1..=a.loop_index()).rev() {
        let x1 = a.word(word) | mask.word(word);
        let x2 = b.word(word) | mask.word(word);
        match x2.cmp(&x1) {
            Ordering::Equal => {}
            order => return order,
        }
    }
    Ordering::Equal
}

pub fn desc1(a: Option<&BitSet>, b: Option<&BitSet>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => descend(a, b),
    }
}

fn lex_compare_desc(a: &BitSet, b: &BitSet) -> Ordering {
    for word in (1..=a.loop_index()).rev() {
        match b.word(word).cmp(&a.word(word)) {
            Ordering::Equal => {}
            order => return order,
        }
    }
    Ordering::Equal
}

fn lex_compare_asc(a: &BitSet, b: &BitSet) -> Ordering {
    for word in (1..=a.loop_index()).rev() {
        match a.word(word).cmp(&b.word(word)) {
            Ordering::Equal => {}
            order => return order,
        }
    }
    Ordering::Equal
}

fn disjoint_binary_word(intersection: u32, mask: u32) -> u32 {
    !(intersection | (intersection >> 1)) & mask
}

fn inactive_binary_word(word: u32, mask: u32) -> u32 {
    !(word & (word >> 1)) & mask
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SparseMatrix {
    rows: Vec<BTreeSet<usize>>,
}

impl SparseMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, row: usize, col: usize) {
        if row >= self.rows.len() {
            self.rows.resize_with(row + 1, BTreeSet::new);
        }
        self.rows[row].insert(col);
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.rows
            .iter()
            .flat_map(|row| row.iter())
            .copied()
            .max()
            .map_or(0, |col| col + 1)
    }

    pub fn rows(&self) -> &[BTreeSet<usize>] {
        &self.rows
    }

    pub fn block_partition(&self) -> Option<(Self, Self)> {
        if self.rows.is_empty() {
            return None;
        }

        let mut col_to_rows: Vec<Vec<usize>> = vec![Vec::new(); self.column_count()];
        for (row_index, row) in self.rows.iter().enumerate() {
            for &col in row {
                col_to_rows[col].push(row_index);
            }
        }

        let mut visited_rows = vec![false; self.rows.len()];
        let mut visited_cols = vec![false; col_to_rows.len()];
        let mut queue = VecDeque::from([0usize]);
        visited_rows[0] = true;

        while let Some(row) = queue.pop_front() {
            for &col in &self.rows[row] {
                if !visited_cols[col] {
                    visited_cols[col] = true;
                    for &next_row in &col_to_rows[col] {
                        if !visited_rows[next_row] {
                            visited_rows[next_row] = true;
                            queue.push_back(next_row);
                        }
                    }
                }
            }
        }

        if visited_rows.iter().all(|visited| *visited) {
            return None;
        }

        let mut left = Self::new();
        let mut right = Self::new();
        for (row_index, row) in self.rows.iter().enumerate() {
            let target = if visited_rows[row_index] {
                &mut left
            } else {
                &mut right
            };
            for &col in row {
                target.insert(row_index, col);
            }
            if row.is_empty() {
                target.rows.resize_with(row_index + 1, BTreeSet::new);
            }
        }
        Some((left, right))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_set_uses_espresso_word_layout() {
        assert_eq!(which_word(0), 1);
        assert_eq!(which_word(31), 1);
        assert_eq!(which_word(32), 2);
        assert_eq!(set_size(32), 2);
        assert_eq!(set_size(33), 3);

        let mut set = BitSet::empty(35);
        set.insert(0);
        set.insert(32);
        set.insert(34);

        assert_eq!(set.word(1), 1);
        assert_eq!(set.word(2), 0b101);
        assert_eq!(set.ord(), 3);
        assert_eq!(set.to_element_string(), "[0,32,34]");
        assert_eq!(bit_index(0b1000), Some(3));
        assert_eq!(bit_index(0), None);
    }

    #[test]
    fn set_operations_match_packed_bit_semantics() {
        let a = BitSet::from_indices(8, [0, 2, 4, 6]);
        let b = BitSet::from_indices(8, [1, 2, 5, 6]);

        assert_eq!(a.and(&b).elements().collect::<Vec<_>>(), vec![2, 6]);
        assert_eq!(a.or(&b).to_bit_string(), "11101110");
        assert_eq!(a.diff(&b).elements().collect::<Vec<_>>(), vec![0, 4]);
        assert_eq!(a.xor(&b).elements().collect::<Vec<_>>(), vec![0, 1, 4, 5]);
        assert_eq!(a.distance(&b), 2);
        assert!(BitSet::from_indices(8, [2]).implies(&a));
        assert!(BitSet::from_indices(8, [7]).is_disjoint(&a));
    }

    #[test]
    fn set_family_counts_and_column_transforms() {
        let family = SetFamily::from_sets(
            5,
            [
                BitSet::from_indices(5, [0, 2, 4]),
                BitSet::from_indices(5, [1, 2]),
                BitSet::from_indices(5, [2, 3]),
            ],
        );

        assert_eq!(family.count_columns(), vec![1, 1, 3, 1, 1]);
        let compressed = family.clone().compress(&BitSet::from_indices(5, [2, 4]));
        assert_eq!(compressed.set_size(), 2);
        assert_eq!(compressed.get(0).to_bit_string(), "11");
        assert_eq!(compressed.get(1).to_bit_string(), "10");

        let transposed = family.transpose();
        assert_eq!(transposed.len(), 5);
        assert_eq!(
            transposed.get(2).elements().collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn cube_setup_builds_masks_and_metadata() {
        let (cube, cdata) = CubeContext::setup(CubeConfig {
            num_vars: 3,
            num_binary_vars: 2,
            part_size: vec![0, 0, 3],
        })
        .unwrap();

        assert_eq!(cube.size, 7);
        assert_eq!(cube.first_part, vec![0, 2, 4]);
        assert_eq!(cube.last_part, vec![1, 3, 6]);
        assert_eq!(cube.output, Some(2));
        assert_eq!(cube.inword, Some(1));
        assert_eq!(cube.inmask, 0b0101);
        assert_eq!(cube.binary_mask.to_bit_string(), "1111000");
        assert_eq!(cube.mv_mask.to_bit_string(), "0000111");
        assert_eq!(cdata.part_zeros.len(), 7);
    }

    #[test]
    fn cube_distance_consensus_and_activity_match_espresso_rules() {
        let (context, _) = CubeContext::setup(CubeConfig {
            num_vars: 3,
            num_binary_vars: 2,
            part_size: vec![0, 0, 3],
        })
        .unwrap();

        let mut a = context.new_cube();
        CubeContext::put_input(&mut a, 0, ONE);
        CubeContext::put_input(&mut a, 1, DASH);
        a.insert(4);
        a.insert(5);

        let mut b = context.new_cube();
        CubeContext::put_input(&mut b, 0, ZERO);
        CubeContext::put_input(&mut b, 1, DASH);
        b.insert(5);
        b.insert(6);

        assert_eq!(context.cdist(&a, &b), 1);
        assert_eq!(context.cdist01(&a, &b), 1);
        assert!(!context.cdist0(&a, &b));

        let consensus = context.consensus(&a, &b);
        assert_eq!(CubeContext::get_input(&consensus, 0), DASH);
        assert_eq!(CubeContext::get_input(&consensus, 1), DASH);
        assert!(consensus.contains(5));
        assert!(!consensus.contains(4));
        assert!(!consensus.contains(6));

        let mut xlower = context.new_cube();
        context.force_lower(&mut xlower, &a, &b);
        assert_eq!(CubeContext::get_input(&xlower, 0), ONE);

        let mut single_active = context.new_full_cube();
        CubeContext::put_input(&mut single_active, 0, ONE);
        assert_eq!(context.cactive(&single_active), Some(0));
    }

    #[test]
    fn ccommon_and_full_row_cover_binary_and_mv_variables() {
        let (context, _) = CubeContext::setup(CubeConfig {
            num_vars: 2,
            num_binary_vars: 1,
            part_size: vec![0, 3],
        })
        .unwrap();

        let mut a = context.new_full_cube();
        a.remove(3);
        let mut b = context.new_full_cube();
        b.remove(4);
        let cof = context.new_cube();

        assert!(context.ccommon(&a, &b, &cof));
        assert!(context.full_row(&a, &BitSet::from_indices(context.size, [3])));
    }

    #[test]
    fn block_partition_splits_disconnected_sparse_matrix() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 0);
        matrix.insert(1, 0);
        matrix.insert(2, 3);

        let (left, right) = matrix.block_partition().unwrap();
        assert_eq!(left.rows()[0], BTreeSet::from([0]));
        assert_eq!(left.rows()[1], BTreeSet::from([0]));
        assert_eq!(right.rows()[2], BTreeSet::from([3]));

        let mut connected = SparseMatrix::new();
        connected.insert(0, 0);
        connected.insert(1, 0);
        connected.insert(1, 1);
        connected.insert(2, 1);
        assert!(connected.block_partition().is_none());
    }
}
