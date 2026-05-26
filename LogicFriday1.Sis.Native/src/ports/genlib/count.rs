//! Native Rust port of SIS `genlib/count.c`.
//!
//! The C implementation memoizes `gl_number_of_gates(n, s, p)` in a fixed
//! `f[40][20][20]` table and recursively counts generated gate forms over the
//! partition stream from `genlib/comb.c`.  This module keeps the same counting
//! recurrence behind an owned counter object instead of using global state.

use std::error::Error;
use std::fmt;

pub const MAX_LEVELS: usize = 40;
pub const MAX_SERIES: usize = 20;
pub const MAX_PARALLEL: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateCountError {
    LevelOutOfRange { level: usize, max: usize },
    SeriesOutOfRange { series: usize, max: usize },
    ParallelOutOfRange { parallel: usize, max: usize },
    UnsupportedSeries { series: usize },
    CountOverflow,
}

impl fmt::Display for GateCountError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LevelOutOfRange { level, max } => {
                write!(formatter, "gate count level {level} is outside 0..{max}")
            }
            Self::SeriesOutOfRange { series, max } => {
                write!(formatter, "series count {series} is outside 0..{max}")
            }
            Self::ParallelOutOfRange { parallel, max } => {
                write!(formatter, "parallel count {parallel} is outside 0..{max}")
            }
            Self::UnsupportedSeries { series } => {
                write!(formatter, "series counts >= 7 are not supported: {series}")
            }
            Self::CountOverflow => write!(formatter, "gate count overflowed u64"),
        }
    }
}

impl Error for GateCountError {}

pub type GateCountResult<T> = Result<T, GateCountError>;

#[derive(Debug, Clone)]
pub struct GateCounter {
    memo: Vec<Option<u64>>,
}

impl Default for GateCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl GateCounter {
    pub fn new() -> Self {
        Self {
            memo: vec![None; MAX_LEVELS * MAX_SERIES * MAX_PARALLEL],
        }
    }

    pub fn clear(&mut self) {
        self.memo.fill(None);
    }

    pub fn number_of_gates(
        &mut self,
        level: usize,
        series: usize,
        parallel: usize,
    ) -> GateCountResult<u64> {
        validate_index(level, series, parallel)?;

        let index = memo_index(level, series, parallel);
        if let Some(count) = self.memo[index] {
            return Ok(count);
        }

        let count = if level == 0 {
            1
        } else {
            let mut sum = 1_u64;
            for partition in Partitions::new(series) {
                let mut child_counts = Vec::with_capacity(partition.len());
                for value in partition {
                    child_counts.push(self.number_of_gates(level - 1, parallel, value)?);
                }
                sum = sum
                    .checked_add(count_it(&child_counts)?)
                    .ok_or(GateCountError::CountOverflow)?;
            }

            let adjustment = self.hack_adjust(level, series, parallel)?;
            sum.checked_sub(adjustment)
                .ok_or(GateCountError::CountOverflow)?
        };

        self.memo[index] = Some(count);
        Ok(count)
    }

    pub fn table_by_level(
        &mut self,
        max_series: usize,
        max_parallel: usize,
        max_level: usize,
    ) -> GateCountResult<Vec<LevelTableRow>> {
        let mut rows = Vec::new();
        for series in 1..=max_series {
            for parallel in 1..=max_parallel {
                let total = self.number_of_gates(series + parallel, series, parallel)?;
                let mut by_level = Vec::with_capacity(max_level + 1);
                for level in 0..=max_level {
                    let count = self.number_of_gates(level, series, parallel)?;
                    let previous = if level == 0 {
                        0
                    } else {
                        self.number_of_gates(level - 1, series, parallel)?
                    };
                    by_level.push(count.saturating_sub(previous));
                }
                rows.push(LevelTableRow {
                    series,
                    parallel,
                    total,
                    by_level,
                });
            }
        }
        Ok(rows)
    }

    pub fn table_of_gate_count(
        &mut self,
        max_series: usize,
        max_parallel: usize,
    ) -> GateCountResult<Vec<Vec<u64>>> {
        let mut rows = Vec::new();
        for series in 1..=max_series {
            let mut row = Vec::new();
            for parallel in 1..=max_parallel {
                let level = series + parallel - 2;
                let left = self.number_of_gates(level, series, parallel)?;
                let right = self.number_of_gates(level, parallel, series)?;
                row.push(
                    left.checked_add(right)
                        .and_then(|value| value.checked_sub(1))
                        .ok_or(GateCountError::CountOverflow)?,
                );
            }
            rows.push(row);
        }
        Ok(rows)
    }

    fn hack_adjust(
        &mut self,
        level: usize,
        series: usize,
        parallel: usize,
    ) -> GateCountResult<u64> {
        match series {
            0..=3 => Ok(0),
            4 => {
                let values = [
                    self.number_of_gates(level - 1, parallel, 2)?,
                    self.number_of_gates(level - 1, parallel, 1)?,
                ];
                count_it(&values)
            }
            5 => {
                let first = [
                    self.number_of_gates(level - 1, parallel, 3)?,
                    self.number_of_gates(level - 1, parallel, 1)?,
                ];
                let second = [
                    self.number_of_gates(level - 1, parallel, 2)?,
                    self.number_of_gates(level - 1, parallel, 1)?,
                    self.number_of_gates(level - 1, parallel, 1)?,
                ];
                count_it(&first)?
                    .checked_add(count_it(&second)?)
                    .ok_or(GateCountError::CountOverflow)
            }
            6 => {
                let adjustments = [
                    vec![
                        self.number_of_gates(level - 1, parallel, 4)?,
                        self.number_of_gates(level - 1, parallel, 1)?,
                    ],
                    vec![
                        self.number_of_gates(level - 1, parallel, 3)?,
                        self.number_of_gates(level - 1, parallel, 2)?,
                    ],
                    vec![
                        self.number_of_gates(level - 1, parallel, 2)?,
                        self.number_of_gates(level - 1, parallel, 2)?,
                        self.number_of_gates(level - 1, parallel, 1)?,
                    ],
                    vec![
                        self.number_of_gates(level - 1, parallel, 3)?,
                        self.number_of_gates(level - 1, parallel, 1)?,
                        self.number_of_gates(level - 1, parallel, 1)?,
                    ],
                    vec![
                        self.number_of_gates(level - 1, parallel, 2)?,
                        self.number_of_gates(level - 1, parallel, 1)?,
                        self.number_of_gates(level - 1, parallel, 1)?,
                        self.number_of_gates(level - 1, parallel, 1)?,
                    ],
                ];

                let mut sum = 0_u64;
                for values in adjustments {
                    sum = sum
                        .checked_add(count_it(&values)?)
                        .ok_or(GateCountError::CountOverflow)?;
                }
                Ok(sum)
            }
            _ => Err(GateCountError::UnsupportedSeries { series }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelTableRow {
    pub series: usize,
    pub parallel: usize,
    pub total: u64,
    pub by_level: Vec<u64>,
}

pub fn number_of_gates(level: usize, series: usize, parallel: usize) -> GateCountResult<u64> {
    GateCounter::new().number_of_gates(level, series, parallel)
}

fn validate_index(level: usize, series: usize, parallel: usize) -> GateCountResult<()> {
    if level >= MAX_LEVELS {
        return Err(GateCountError::LevelOutOfRange {
            level,
            max: MAX_LEVELS,
        });
    }
    if series >= MAX_SERIES {
        return Err(GateCountError::SeriesOutOfRange {
            series,
            max: MAX_SERIES,
        });
    }
    if parallel >= MAX_PARALLEL {
        return Err(GateCountError::ParallelOutOfRange {
            parallel,
            max: MAX_PARALLEL,
        });
    }
    Ok(())
}

fn memo_index(level: usize, series: usize, parallel: usize) -> usize {
    (level * MAX_SERIES * MAX_PARALLEL) + (series * MAX_PARALLEL) + parallel
}

fn count_it(values: &[u64]) -> GateCountResult<u64> {
    if values.is_empty() {
        return Ok(0);
    }
    if values.len() == 1 {
        return Ok(values[0]);
    }

    let smallest = *values.last().expect("values is not empty");
    let mut sum = 0_u64;
    for offset in 0..smallest {
        let mut next = Vec::with_capacity(values.len() - 1);
        for value in &values[..values.len() - 1] {
            next.push(
                value
                    .checked_sub(offset)
                    .ok_or(GateCountError::CountOverflow)?,
            );
        }
        sum = sum
            .checked_add(count_it(&next)?)
            .ok_or(GateCountError::CountOverflow)?;
    }
    Ok(sum)
}

#[derive(Debug, Clone)]
struct Partitions {
    value: Vec<usize>,
    non_zero: usize,
    sum: usize,
    max_sum: usize,
    finished: bool,
}

impl Partitions {
    fn new(sum: usize) -> Self {
        let mut value = vec![0; sum];
        if sum > 1 {
            value[1] = 1;
        }
        Self {
            value,
            non_zero: usize::from(sum > 0),
            sum: usize::from(sum > 0),
            max_sum: sum,
            finished: false,
        }
    }

    fn next_less_than(&mut self) -> Option<Vec<usize>> {
        if self.finished || self.max_sum == 0 {
            return None;
        }

        for k in 0..self.value.len() {
            if self.value[k] == 0 {
                self.non_zero += 1;
            }
            self.value[k] += 1;
            self.sum += 1;

            if self.sum <= self.max_sum {
                return Some(self.value[..self.non_zero].to_vec());
            }

            if k == self.value.len() - 1 {
                self.finished = true;
                return None;
            }

            for l in 0..=k {
                let new_value = self.value[k + 1] + 1;
                self.sum = self.sum - self.value[l] + new_value;
                self.value[l] = new_value;
            }
        }

        self.finished = true;
        None
    }
}

impl Iterator for Partitions {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(partition) = self.next_less_than() {
            if self.sum == self.max_sum {
                return Some(partition);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_it_matches_simple_nested_c_counts() {
        assert_eq!(count_it(&[4]).unwrap(), 4);
        assert_eq!(count_it(&[4, 2]).unwrap(), 7);
        assert_eq!(count_it(&[5, 3, 2]).unwrap(), 19);
    }

    #[test]
    fn partition_iterator_matches_comb_c_order_for_small_sums() {
        assert_eq!(
            Partitions::new(1).collect::<Vec<_>>(),
            Vec::<Vec<usize>>::new()
        );
        assert_eq!(Partitions::new(2).collect::<Vec<_>>(), vec![vec![1, 1]]);
        assert_eq!(
            Partitions::new(4).collect::<Vec<_>>(),
            vec![vec![3, 1], vec![2, 2], vec![2, 1, 1], vec![1, 1, 1, 1]]
        );
    }

    #[test]
    fn base_level_has_one_form_for_valid_indices() {
        assert_eq!(number_of_gates(0, 0, 0).unwrap(), 1);
        assert_eq!(number_of_gates(0, 6, 6).unwrap(), 1);
    }

    #[test]
    fn small_gate_counts_match_the_c_recurrence() {
        let mut counter = GateCounter::new();

        assert_eq!(counter.number_of_gates(1, 1, 1).unwrap(), 1);
        assert_eq!(counter.number_of_gates(1, 2, 1).unwrap(), 2);
        assert_eq!(counter.number_of_gates(2, 2, 1).unwrap(), 2);
        assert_eq!(counter.number_of_gates(2, 2, 2).unwrap(), 4);
        assert_eq!(counter.number_of_gates(3, 3, 2).unwrap(), 12);
        assert_eq!(counter.number_of_gates(4, 4, 2).unwrap(), 32);
        assert_eq!(counter.number_of_gates(5, 5, 2).unwrap(), 75);
        assert_eq!(counter.number_of_gates(6, 6, 2).unwrap(), 165);
    }

    #[test]
    fn unsupported_hack_adjust_is_returned_as_error() {
        assert_eq!(
            number_of_gates(1, 7, 1).unwrap_err(),
            GateCountError::UnsupportedSeries { series: 7 }
        );
    }

    #[test]
    fn validates_fixed_c_table_bounds() {
        assert_eq!(
            number_of_gates(MAX_LEVELS, 1, 1).unwrap_err(),
            GateCountError::LevelOutOfRange {
                level: MAX_LEVELS,
                max: MAX_LEVELS
            }
        );
        assert_eq!(
            number_of_gates(1, MAX_SERIES, 1).unwrap_err(),
            GateCountError::SeriesOutOfRange {
                series: MAX_SERIES,
                max: MAX_SERIES
            }
        );
        assert_eq!(
            number_of_gates(1, 1, MAX_PARALLEL).unwrap_err(),
            GateCountError::ParallelOutOfRange {
                parallel: MAX_PARALLEL,
                max: MAX_PARALLEL
            }
        );
    }

    #[test]
    fn table_of_gate_count_matches_io_c_formula() {
        let mut counter = GateCounter::new();
        let table = counter.table_of_gate_count(3, 3).unwrap();

        assert_eq!(table[0], vec![1, 2, 3]);
        assert_eq!(table[1], vec![2, 7, 18]);
        assert_eq!(table[2], vec![3, 18, 87]);
    }

    #[test]
    fn level_table_reports_incremental_counts() {
        let mut counter = GateCounter::new();
        let rows = counter.table_by_level(1, 2, 3).unwrap();

        assert_eq!(
            rows,
            vec![
                LevelTableRow {
                    series: 1,
                    parallel: 1,
                    total: 1,
                    by_level: vec![1, 0, 0, 0],
                },
                LevelTableRow {
                    series: 1,
                    parallel: 2,
                    total: 1,
                    by_level: vec![1, 0, 0, 0],
                },
            ]
        );
    }
}
