//! Native Rust port of `LogicSynthesis/sis/st/st_bench1.c`.
//!
//! The C source is a standalone benchmark program for the SIS `st_table`
//! string-key table: read words from standard input, scramble their order, time
//! insertion into a table, and print a one-line elapsed-time report. This port
//! exposes that behavior as ordinary Rust functions instead of recreating a
//! process entry point or legacy C ABI export.

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StBench1Report {
    object_count: usize,
    distinct_count: usize,
    elapsed: Duration,
}

impl StBench1Report {
    pub fn object_count(&self) -> usize {
        self.object_count
    }

    pub fn distinct_count(&self) -> usize {
        self.distinct_count
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn elapsed_milliseconds(&self) -> u128 {
        self.elapsed.as_millis()
    }

    pub fn elapsed_time_text(&self) -> String {
        print_time(self.elapsed_milliseconds())
    }

    pub fn summary_line(&self) -> String {
        format!(
            "Elapsed time for insert of {} objects was {}",
            self.object_count,
            self.elapsed_time_text()
        )
    }
}

pub trait RandomIndexSource {
    fn next_index(&mut self, upper_exclusive: usize) -> usize;
}

#[derive(Clone, Debug)]
pub struct SisRandom {
    state: u64,
}

impl SisRandom {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
}

impl Default for SisRandom {
    fn default() -> Self {
        Self::new(1)
    }
}

impl RandomIndexSource for SisRandom {
    fn next_index(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive == 0 {
            return 0;
        }

        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((self.state >> 1) as usize) % upper_exclusive
    }
}

pub fn read_words(input: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    input
        .into_iter()
        .map(|line| line.as_ref().to_owned())
        .collect()
}

pub fn scramble_words(words: &mut [String], random: &mut impl RandomIndexSource) {
    for i in (1..words.len()).rev() {
        let j = random.next_index(i);
        words.swap(i, j);
    }
}

pub fn benchmark_words(
    words: impl IntoIterator<Item = impl AsRef<str>>,
    random: &mut impl RandomIndexSource,
) -> StBench1Report {
    benchmark_words_with_clock(words, random, Instant::now)
}

pub fn benchmark_words_with_clock(
    words: impl IntoIterator<Item = impl AsRef<str>>,
    random: &mut impl RandomIndexSource,
    clock: impl FnOnce() -> Instant,
) -> StBench1Report {
    let mut words = read_words(words);
    scramble_words(&mut words, random);

    let start = clock();
    let mut table = HashMap::with_capacity(words.len());

    for word in words.iter().rev() {
        table.insert(word.clone(), ());
    }

    StBench1Report {
        object_count: words.len(),
        distinct_count: table.len(),
        elapsed: start.elapsed(),
    }
}

pub fn benchmark_summary(
    input: impl IntoIterator<Item = impl AsRef<str>>,
    random: &mut impl RandomIndexSource,
) -> String {
    benchmark_words(input, random).summary_line()
}

fn print_time(milliseconds: u128) -> String {
    format!(
        "{}.{:02} sec",
        milliseconds / 1_000,
        (milliseconds % 1_000) / 10
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FixedRandom {
        values: Vec<usize>,
    }

    impl FixedRandom {
        fn new(values: &[usize]) -> Self {
            Self {
                values: values.iter().rev().copied().collect(),
            }
        }
    }

    impl RandomIndexSource for FixedRandom {
        fn next_index(&mut self, upper_exclusive: usize) -> usize {
            self.values.pop().unwrap_or(0) % upper_exclusive
        }
    }

    #[test]
    fn read_words_preserves_lines() {
        let words = read_words(["alpha", "", "beta"]);

        assert_eq!(words, ["alpha", "", "beta"]);
    }

    #[test]
    fn scramble_matches_c_loop_bounds() {
        let mut words = read_words(["a", "b", "c", "d"]);
        let mut random = FixedRandom::new(&[1, 0, 0]);

        scramble_words(&mut words, &mut random);

        assert_eq!(words, ["d", "c", "a", "b"]);
    }

    #[test]
    fn report_formats_elapsed_time_like_util_print_time() {
        let report = StBench1Report {
            object_count: 12,
            distinct_count: 10,
            elapsed: Duration::from_millis(1_234),
        };

        assert_eq!(report.elapsed_time_text(), "1.23 sec");
        assert_eq!(
            report.summary_line(),
            "Elapsed time for insert of 12 objects was 1.23 sec"
        );
        assert_eq!(report.distinct_count(), 10);
    }

    #[test]
    fn benchmark_counts_input_objects_and_distinct_table_keys() {
        let mut random = FixedRandom::default();
        let report = benchmark_words(["x", "y", "x"], &mut random);

        assert_eq!(report.object_count(), 3);
        assert_eq!(report.distinct_count(), 2);
    }
}
