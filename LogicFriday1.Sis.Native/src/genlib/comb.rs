//! Native Rust generators for `sis/genlib/comb.c`.
//!
//! The original SIS code exposes mutable C generators for mixed-radix
//! combinations and integer partitions. This module keeps the same iteration
//! order with owned Rust state and slice-based accessors.

use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombinationError {
    EmptyRadices,
    ZeroRadix { index: usize },
}

impl fmt::Display for CombinationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRadices => write!(f, "combination generator requires at least one radix"),
            Self::ZeroRadix { index } => {
                write!(f, "combination radix at index {index} must be positive")
            }
        }
    }
}

impl Error for CombinationError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombinationGenerator {
    radices: Vec<usize>,
    state: Vec<usize>,
    started: bool,
}

impl CombinationGenerator {
    pub fn new(radices: impl Into<Vec<usize>>) -> Result<Self, CombinationError> {
        let radices = radices.into();
        if radices.is_empty() {
            return Err(CombinationError::EmptyRadices);
        }

        if let Some((index, _)) = radices.iter().enumerate().find(|(_, radix)| **radix == 0) {
            return Err(CombinationError::ZeroRadix { index });
        }

        Ok(Self {
            state: vec![0; radices.len()],
            radices,
            started: false,
        })
    }

    pub fn radices(&self) -> &[usize] {
        &self.radices
    }

    pub fn state(&self) -> &[usize] {
        &self.state
    }

    pub fn next_combination(&mut self) -> Option<&[usize]> {
        self.advance_combination()?;
        Some(&self.state)
    }

    pub fn next_nondecreasing_combination(&mut self) -> Option<&[usize]> {
        loop {
            self.advance_combination()?;
            if self.state.windows(2).all(|window| window[0] <= window[1]) {
                return Some(&self.state);
            }
        }
    }

    pub fn next_nonincreasing_combination(&mut self) -> Option<&[usize]> {
        loop {
            self.advance_combination()?;
            if self.state.windows(2).all(|window| window[0] >= window[1]) {
                return Some(&self.state);
            }
        }
    }

    fn advance_combination(&mut self) -> Option<()> {
        for index in 0..self.state.len() {
            if !self.started {
                self.started = true;
                return Some(());
            }

            self.state[index] += 1;
            if self.state[index] >= self.radices[index] {
                self.state[index] = 0;
            } else {
                return Some(());
            }
        }

        None
    }
}

impl Iterator for CombinationGenerator {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_combination().map(<[usize]>::to_vec)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PartitionError {
    ZeroMaximum,
}

impl fmt::Display for PartitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroMaximum => write!(f, "partition generator maximum sum must be positive"),
        }
    }
}

impl Error for PartitionError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Partition<'a> {
    pub values: &'a [usize],
    pub non_zero: usize,
    pub sum: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedPartition {
    pub values: Vec<usize>,
    pub non_zero: usize,
    pub sum: usize,
}

impl<'a> From<Partition<'a>> for OwnedPartition {
    fn from(value: Partition<'a>) -> Self {
        Self {
            values: value.values.to_vec(),
            non_zero: value.non_zero,
            sum: value.sum,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartitionGenerator {
    values: Vec<usize>,
    non_zero: usize,
    sum: usize,
    max_sum: usize,
}

impl PartitionGenerator {
    pub fn new(max_sum: usize) -> Result<Self, PartitionError> {
        if max_sum == 0 {
            return Err(PartitionError::ZeroMaximum);
        }

        let mut values = vec![0; max_sum];
        if max_sum > 1 {
            values[1] = 1;
        }

        Ok(Self {
            values,
            non_zero: 1,
            sum: 1,
            max_sum,
        })
    }

    pub fn values(&self) -> &[usize] {
        &self.values
    }

    pub fn non_zero(&self) -> usize {
        self.non_zero
    }

    pub fn sum(&self) -> usize {
        self.sum
    }

    pub fn max_sum(&self) -> usize {
        self.max_sum
    }

    pub fn next_partition_less_than(&mut self) -> Option<Partition<'_>> {
        for index in 0..self.values.len() {
            if self.values[index] == 0 {
                self.non_zero += 1;
            }
            self.values[index] += 1;
            self.sum += 1;

            if self.sum <= self.max_sum {
                return Some(self.current_partition());
            }

            if index == self.values.len() - 1 {
                return None;
            }

            for reset_index in 0..=index {
                let new_value = self.values[index + 1] + 1;
                self.sum = self.sum - self.values[reset_index] + new_value;
                self.values[reset_index] = new_value;
            }
        }

        None
    }

    pub fn next_partition(&mut self) -> Option<Partition<'_>> {
        loop {
            self.next_partition_less_than()?;
            if self.sum == self.max_sum {
                return Some(self.current_partition());
            }
        }
    }

    fn current_partition(&self) -> Partition<'_> {
        Partition {
            values: &self.values,
            non_zero: self.non_zero,
            sum: self.sum,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_combinations(mut generator: CombinationGenerator) -> Vec<Vec<usize>> {
        let mut combinations = Vec::new();
        while let Some(combination) = generator.next_combination() {
            combinations.push(combination.to_vec());
        }
        combinations
    }

    fn collect_nondecreasing(mut generator: CombinationGenerator) -> Vec<Vec<usize>> {
        let mut combinations = Vec::new();
        while let Some(combination) = generator.next_nondecreasing_combination() {
            combinations.push(combination.to_vec());
        }
        combinations
    }

    fn collect_nonincreasing(mut generator: CombinationGenerator) -> Vec<Vec<usize>> {
        let mut combinations = Vec::new();
        while let Some(combination) = generator.next_nonincreasing_combination() {
            combinations.push(combination.to_vec());
        }
        combinations
    }

    fn collect_partitions_less_than(max_sum: usize) -> Vec<OwnedPartition> {
        let mut generator = PartitionGenerator::new(max_sum).unwrap();
        let mut partitions = Vec::new();
        while let Some(partition) = generator.next_partition_less_than() {
            partitions.push(partition.into());
        }
        partitions
    }

    fn collect_partitions(max_sum: usize) -> Vec<OwnedPartition> {
        let mut generator = PartitionGenerator::new(max_sum).unwrap();
        let mut partitions = Vec::new();
        while let Some(partition) = generator.next_partition() {
            partitions.push(partition.into());
        }
        partitions
    }

    #[test]
    fn combinations_follow_c_mixed_radix_order() {
        let generator = CombinationGenerator::new(vec![2, 3]).unwrap();

        assert_eq!(
            collect_combinations(generator),
            vec![
                vec![0, 0],
                vec![1, 0],
                vec![0, 1],
                vec![1, 1],
                vec![0, 2],
                vec![1, 2],
            ]
        );
    }

    #[test]
    fn iterator_yields_owned_combinations() {
        let generator = CombinationGenerator::new(vec![2, 2]).unwrap();

        assert_eq!(
            generator.collect::<Vec<_>>(),
            vec![vec![0, 0], vec![1, 0], vec![0, 1], vec![1, 1]]
        );
    }

    #[test]
    fn nondecreasing_combinations_are_filtered_in_original_order() {
        let generator = CombinationGenerator::new(vec![3, 3, 3]).unwrap();

        assert_eq!(
            collect_nondecreasing(generator),
            vec![
                vec![0, 0, 0],
                vec![0, 0, 1],
                vec![0, 1, 1],
                vec![1, 1, 1],
                vec![0, 0, 2],
                vec![0, 1, 2],
                vec![1, 1, 2],
                vec![0, 2, 2],
                vec![1, 2, 2],
                vec![2, 2, 2],
            ]
        );
    }

    #[test]
    fn nonincreasing_combinations_are_filtered_in_original_order() {
        let generator = CombinationGenerator::new(vec![3, 3, 3]).unwrap();

        assert_eq!(
            collect_nonincreasing(generator),
            vec![
                vec![0, 0, 0],
                vec![1, 0, 0],
                vec![2, 0, 0],
                vec![1, 1, 0],
                vec![2, 1, 0],
                vec![2, 2, 0],
                vec![1, 1, 1],
                vec![2, 1, 1],
                vec![2, 2, 1],
                vec![2, 2, 2],
            ]
        );
    }

    #[test]
    fn combination_constructor_rejects_invalid_inputs() {
        assert_eq!(
            CombinationGenerator::new(Vec::<usize>::new()).unwrap_err(),
            CombinationError::EmptyRadices
        );
        assert_eq!(
            CombinationGenerator::new(vec![2, 0, 3]).unwrap_err(),
            CombinationError::ZeroRadix { index: 1 }
        );
    }

    #[test]
    fn partition_less_than_preserves_intermediate_and_exact_sums() {
        let partitions = collect_partitions_less_than(3);

        assert_eq!(
            partitions,
            vec![
                OwnedPartition {
                    values: vec![1, 1, 0],
                    non_zero: 2,
                    sum: 2,
                },
                OwnedPartition {
                    values: vec![2, 1, 0],
                    non_zero: 2,
                    sum: 3,
                },
                OwnedPartition {
                    values: vec![1, 1, 1],
                    non_zero: 3,
                    sum: 3,
                },
            ]
        );
    }

    #[test]
    fn exact_partitions_return_only_max_sum_entries() {
        let partitions = collect_partitions(5);

        assert_eq!(
            partitions,
            vec![
                OwnedPartition {
                    values: vec![4, 1, 0, 0, 0],
                    non_zero: 2,
                    sum: 5,
                },
                OwnedPartition {
                    values: vec![3, 2, 0, 0, 0],
                    non_zero: 2,
                    sum: 5,
                },
                OwnedPartition {
                    values: vec![3, 1, 1, 0, 0],
                    non_zero: 3,
                    sum: 5,
                },
                OwnedPartition {
                    values: vec![2, 2, 1, 0, 0],
                    non_zero: 3,
                    sum: 5,
                },
                OwnedPartition {
                    values: vec![2, 1, 1, 1, 0],
                    non_zero: 4,
                    sum: 5,
                },
                OwnedPartition {
                    values: vec![1, 1, 1, 1, 1],
                    non_zero: 5,
                    sum: 5,
                },
            ]
        );
    }

    #[test]
    fn partition_of_one_matches_original_exhaustion_behavior() {
        assert!(collect_partitions_less_than(1).is_empty());
        assert!(collect_partitions(1).is_empty());
    }

    #[test]
    fn partition_constructor_rejects_zero_maximum() {
        assert_eq!(
            PartitionGenerator::new(0).unwrap_err(),
            PartitionError::ZeroMaximum
        );
    }
}
