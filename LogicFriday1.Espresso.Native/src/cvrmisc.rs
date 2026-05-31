//! Source-aligned cover utility routines from Espresso `cvrmisc.c`.
//!
//! The original C module computes and formats cover costs, emits trace lines,
//! accumulates timing totals, and terminates on fatal input errors. This Rust
//! module keeps the same narrow API surface over the native [`Cover`] and
//! [`Pla`] representation used by `pla.rs`.

use std::fmt;
use std::time::Duration;

use crate::pla::{Cover, Pla};

/// Cost fields corresponding to Espresso's `cost_t`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct EspressoCost {
    pub cubes: usize,
    pub inputs: usize,
    pub outputs: usize,
    pub mv: usize,
    pub total: usize,
    pub primes: usize,
}

impl EspressoCost {
    pub fn nonprimes(self) -> usize {
        self.cubes.saturating_sub(self.primes)
    }
}

impl fmt::Display for EspressoCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_cost(self, false, f)
    }
}

/// Compute the cost of a cover, matching `cover_cost` for binary PLA covers.
///
/// The current native PLA representation is binary-valued at this layer, so
/// `mv` remains zero. Cubes do not carry Espresso's PRIME flag, therefore all
/// cubes are reported as prime for the `c=n(nonprime)` formatter field.
pub fn cover_cost(cover: &Cover, input_count: usize, output_count: usize) -> EspressoCost {
    let cost = cover.cost(input_count, output_count);
    EspressoCost {
        cubes: cost.cubes,
        inputs: cost.inputs,
        outputs: cost.outputs,
        mv: 0,
        total: cost.total,
        primes: cost.cubes,
    }
}

/// Compute the ON-set cost for a PLA, equivalent to `print_cost(PLA->F)`.
pub fn pla_onset_cost(pla: &Pla) -> EspressoCost {
    cover_cost(&pla.f, pla.input_count, pla.output_count)
}

/// Format a cost value using Espresso's `fmt_cost` spelling.
pub fn format_cost(cost: &EspressoCost) -> String {
    cost.to_string()
}

/// Copy a cost value, preserving the source-aligned `copy_cost` operation.
pub fn copy_cost(source: &EspressoCost) -> EspressoCost {
    *source
}

/// Return a `size_stamp` line instead of writing directly to stdout.
pub fn size_stamp(cover: &Cover, name: &str, input_count: usize, output_count: usize) -> String {
    format!(
        "# {name}\tCost is {}\n",
        cover_cost(cover, input_count, output_count)
    )
}

/// Return a `print_trace` line instead of writing directly to stdout.
pub fn print_trace(
    cover: &Cover,
    name: &str,
    elapsed: Duration,
    input_count: usize,
    output_count: usize,
) -> String {
    format!(
        "# {name}\tTime was {}, cost is {}\n",
        format_time(elapsed),
        cover_cost(cover, input_count, output_count)
    )
}

/// Accumulates the fields maintained by Espresso's `totals` routine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CostTotals {
    pub total_time: Duration,
    pub total_calls: usize,
    pub last_cost: EspressoCost,
}

impl CostTotals {
    pub fn add_call(
        &mut self,
        elapsed: Duration,
        cover: &Cover,
        input_count: usize,
        output_count: usize,
    ) -> EspressoCost {
        self.total_time += elapsed;
        self.total_calls += 1;
        self.last_cost = cover_cost(cover, input_count, output_count);
        self.last_cost
    }
}

/// Source-aligned fatal error type for callers that used C `fatal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FatalError {
    message: String,
}

impl FatalError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for FatalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "espresso: {}", self.message)
    }
}

impl std::error::Error for FatalError {}

fn fmt_cost(cost: &EspressoCost, has_mv: bool, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if has_mv {
        write!(
            f,
            "c={}({}) in={} mv={} out={}",
            cost.cubes,
            cost.nonprimes(),
            cost.inputs,
            cost.mv,
            cost.outputs
        )
    } else {
        write!(
            f,
            "c={}({}) in={} out={} tot={}",
            cost.cubes,
            cost.nonprimes(),
            cost.inputs,
            cost.outputs,
            cost.total
        )
    }
}

fn format_time(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1_000 {
        format!("{millis}ms")
    } else {
        format!("{:.3}s", duration.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::pla::Pla;

    #[test]
    fn cover_cost_counts_binary_input_and_output_literals() {
        let pla = Pla::parse(".i 2\n.o 2\n00 10\n1- 11\n.e\n").unwrap();

        let cost = cover_cost(&pla.f, pla.input_count, pla.output_count);

        assert_eq!(
            cost,
            EspressoCost {
                cubes: 2,
                inputs: 3,
                outputs: 3,
                mv: 0,
                total: 6,
                primes: 2,
            }
        );
        assert_eq!(format_cost(&cost), "c=2(0) in=3 out=3 tot=6");
    }

    #[test]
    fn trace_helpers_return_espresso_style_lines() {
        let pla = Pla::parse(".i 1\n.o 1\n0 1\n.e\n").unwrap();

        assert_eq!(
            size_stamp(&pla.f, "READ", pla.input_count, pla.output_count),
            "# READ\tCost is c=1(0) in=1 out=1 tot=2\n"
        );
        assert_eq!(
            print_trace(
                &pla.f,
                "EXPAND",
                Duration::from_millis(1250),
                pla.input_count,
                pla.output_count,
            ),
            "# EXPAND\tTime was 1.250s, cost is c=1(0) in=1 out=1 tot=2\n"
        );
    }
}
