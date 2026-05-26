use std::fmt;
use std::time::Duration;

use super::sparse::{Cover, CubeStructure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoCoverCost {
    pub cubes: usize,
    pub input_literals: usize,
    pub output_literals: usize,
    pub mv_literals: usize,
    pub total_literals: usize,
    pub prime_cubes: usize,
}

impl EspressoCoverCost {
    pub const fn empty() -> Self {
        Self {
            cubes: 0,
            input_literals: 0,
            output_literals: 0,
            mv_literals: 0,
            total_literals: 0,
            prime_cubes: 0,
        }
    }

    pub fn nonprime_cubes(&self) -> usize {
        self.cubes.saturating_sub(self.prime_cubes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoverCostError {
    BinaryVariableCountExceedsVariableCount {
        binary_variable_count: usize,
        variable_count: usize,
    },
    PrimeFlagsLengthMismatch {
        cube_count: usize,
        prime_flag_count: usize,
    },
}

impl fmt::Display for CoverCostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BinaryVariableCountExceedsVariableCount {
                binary_variable_count,
                variable_count,
            } => write!(
                formatter,
                "binary variable count {binary_variable_count} exceeds variable count {variable_count}"
            ),
            Self::PrimeFlagsLengthMismatch {
                cube_count,
                prime_flag_count,
            } => write!(
                formatter,
                "cover has {cube_count} cubes but {prime_flag_count} prime flags were supplied"
            ),
        }
    }
}

impl std::error::Error for CoverCostError {}

pub fn cover_cost(
    cover: &Cover,
    structure: &CubeStructure,
    binary_variable_count: usize,
    prime_flags: &[bool],
) -> Result<EspressoCoverCost, CoverCostError> {
    let variable_count = structure.variable_count();
    if binary_variable_count > variable_count {
        return Err(CoverCostError::BinaryVariableCountExceedsVariableCount {
            binary_variable_count,
            variable_count,
        });
    }

    if prime_flags.len() != cover.len() {
        return Err(CoverCostError::PrimeFlagsLengthMismatch {
            cube_count: cover.len(),
            prime_flag_count: prime_flags.len(),
        });
    }

    let variable_zero_counts = variable_zero_counts(cover, structure);
    let mut cost = EspressoCoverCost {
        cubes: cover.len(),
        prime_cubes: prime_flags.iter().filter(|is_prime| **is_prime).count(),
        ..EspressoCoverCost::empty()
    };

    for zeros in variable_zero_counts.iter().take(binary_variable_count) {
        cost.input_literals += *zeros;
    }

    let output_variable_index = if binary_variable_count == variable_count {
        None
    } else {
        variable_count.checked_sub(1)
    };

    let mv_end = output_variable_index.unwrap_or(variable_count);
    for (variable_index, zeros) in variable_zero_counts
        .iter()
        .enumerate()
        .take(mv_end)
        .skip(binary_variable_count)
    {
        let variable = structure
            .variable(variable_index)
            .expect("valid variable index");
        let part_count = variable.part_count();
        if variable.is_sparse() {
            cost.mv_literals += cover.len() * part_count - zeros;
        } else {
            cost.mv_literals += zeros;
        }
    }

    if let Some(variable_index) = output_variable_index {
        let variable = structure
            .variable(variable_index)
            .expect("valid variable index");
        cost.output_literals =
            cover.len() * variable.part_count() - variable_zero_counts[variable_index];
    }

    cost.total_literals = cost.input_literals + cost.mv_literals + cost.output_literals;
    Ok(cost)
}

pub fn print_cost(
    cover: &Cover,
    structure: &CubeStructure,
    binary_variable_count: usize,
    prime_flags: &[bool],
) -> Result<String, CoverCostError> {
    cover_cost(cover, structure, binary_variable_count, prime_flags)
        .map(|cost| format_cost(&cost, structure.variable_count(), binary_variable_count))
}

pub fn format_cost(
    cost: &EspressoCoverCost,
    variable_count: usize,
    binary_variable_count: usize,
) -> String {
    if binary_variable_count + 1 == variable_count {
        format!(
            "c={}({}) in={} out={} tot={}",
            cost.cubes,
            cost.nonprime_cubes(),
            cost.input_literals,
            cost.output_literals,
            cost.total_literals
        )
    } else {
        format!(
            "c={}({}) in={} mv={} out={}",
            cost.cubes,
            cost.nonprime_cubes(),
            cost.input_literals,
            cost.mv_literals,
            cost.output_literals
        )
    }
}

pub fn copy_cost(source: &EspressoCoverCost) -> EspressoCoverCost {
    source.clone()
}

pub fn size_stamp(name: &str, cost_text: &str) -> String {
    format!("# {name}\tCost is {cost_text}")
}

pub fn print_trace(name: &str, elapsed: Duration, cost_text: &str) -> String {
    format!(
        "# {name}\tTime was {}, cost is {cost_text}",
        format_elapsed_time(elapsed)
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimingTotal {
    pub name: String,
    pub total_time: Duration,
    pub calls: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimingTotals {
    entries: Vec<TimingTotal>,
    trace: bool,
}

impl TimingTotals {
    pub fn new(names: impl IntoIterator<Item = impl Into<String>>, trace: bool) -> Self {
        Self {
            entries: names
                .into_iter()
                .map(|name| TimingTotal {
                    name: name.into(),
                    total_time: Duration::ZERO,
                    calls: 0,
                })
                .collect(),
            trace,
        }
    }

    pub fn entries(&self) -> &[TimingTotal] {
        &self.entries
    }

    pub fn record(
        &mut self,
        index: usize,
        elapsed: Duration,
        cost: &EspressoCoverCost,
        variable_count: usize,
        binary_variable_count: usize,
    ) -> Option<String> {
        let entry = self
            .entries
            .get_mut(index)
            .unwrap_or_else(|| panic!("timing total index {index} is out of range"));
        entry.total_time += elapsed;
        entry.calls += 1;

        if self.trace {
            let cost_text = format_cost(cost, variable_count, binary_variable_count);
            Some(print_trace(&entry.name, elapsed, &cost_text))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoFatalError {
    message: String,
}

impl EspressoFatalError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for EspressoFatalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "espresso: {}", self.message)
    }
}

impl std::error::Error for EspressoFatalError {}

pub fn fatal(message: impl Into<String>) -> EspressoFatalError {
    EspressoFatalError::new(message)
}

fn variable_zero_counts(cover: &Cover, structure: &CubeStructure) -> Vec<usize> {
    structure
        .variables()
        .iter()
        .copied()
        .map(|variable| {
            cover
                .cubes()
                .iter()
                .map(|cube| variable.part_count() - cube.part_count_for(variable))
                .sum()
        })
        .collect()
}

fn format_elapsed_time(elapsed: Duration) -> String {
    format!(
        "{}.{:02} sec",
        elapsed.as_secs(),
        elapsed.subsec_millis() / 10
    )
}

trait VariablePartCount {
    fn part_count(self) -> usize;
}

impl VariablePartCount for super::sparse::Variable {
    fn part_count(self) -> usize {
        self.last_part() - self.first_part() + 1
    }
}

#[cfg(test)]
mod tests {
    use super::super::sparse::{Cover, Cube, CubeStructure, Variable};
    use super::*;

    fn structure() -> CubeStructure {
        CubeStructure::new([
            Variable::new(0, 1, false),
            Variable::new(2, 3, false),
            Variable::new(4, 6, true),
            Variable::new(7, 8, false),
        ])
    }

    fn cube(parts: &[usize]) -> Cube {
        Cube::from_parts(parts.iter().copied())
    }

    fn cover(cubes: &[&[usize]]) -> Cover {
        Cover::new(cubes.iter().map(|parts| cube(parts)))
    }

    #[test]
    fn cover_cost_matches_binary_mv_and_output_rules() {
        let structure = structure();
        let cover = cover(&[&[0, 1, 2, 4, 6, 7], &[0, 3, 5, 8]]);
        let cost = cover_cost(&cover, &structure, 2, &[true, false]).unwrap();

        assert_eq!(
            cost,
            EspressoCoverCost {
                cubes: 2,
                input_literals: 3,
                mv_literals: 3,
                output_literals: 2,
                total_literals: 8,
                prime_cubes: 1,
            }
        );
    }

    #[test]
    fn dense_mv_variables_cost_missing_parts() {
        let structure = CubeStructure::new([
            Variable::new(0, 1, false),
            Variable::new(2, 4, false),
            Variable::new(5, 6, false),
        ]);
        let cover = cover(&[&[0, 2, 3, 5], &[1, 2, 6]]);
        let cost = cover_cost(&cover, &structure, 1, &[false, false]).unwrap();

        assert_eq!(cost.mv_literals, 3);
    }

    #[test]
    fn no_output_cover_charges_all_non_binary_variables_as_mv() {
        let structure = CubeStructure::new([Variable::new(0, 1, false), Variable::new(2, 4, true)]);
        let cover = cover(&[&[0, 2], &[1, 2, 3]]);
        let cost = cover_cost(&cover, &structure, 2, &[false, false]).unwrap();

        assert_eq!(cost.output_literals, 0);
        assert_eq!(cost.mv_literals, 0);
    }

    #[test]
    fn format_cost_uses_total_for_binary_input_output_cover() {
        let cost = EspressoCoverCost {
            cubes: 3,
            input_literals: 4,
            output_literals: 2,
            mv_literals: 0,
            total_literals: 6,
            prime_cubes: 1,
        };

        assert_eq!(format_cost(&cost, 3, 2), "c=3(2) in=4 out=2 tot=6");
    }

    #[test]
    fn format_cost_uses_mv_field_when_mv_variables_exist() {
        let cost = EspressoCoverCost {
            cubes: 3,
            input_literals: 4,
            output_literals: 2,
            mv_literals: 5,
            total_literals: 11,
            prime_cubes: 1,
        };

        assert_eq!(format_cost(&cost, 4, 2), "c=3(2) in=4 mv=5 out=2");
    }

    #[test]
    fn size_and_trace_lines_match_espresso_text_shape() {
        assert_eq!(
            size_stamp("EXPAND", "c=1(0) in=2 out=1 tot=3"),
            "# EXPAND\tCost is c=1(0) in=2 out=1 tot=3"
        );
        assert_eq!(
            print_trace(
                "REDUCE",
                Duration::from_millis(1_230),
                "c=1(0) in=2 out=1 tot=3"
            ),
            "# REDUCE\tTime was 1.23 sec, cost is c=1(0) in=2 out=1 tot=3"
        );
    }

    #[test]
    fn timing_totals_records_elapsed_time_calls_and_optional_trace() {
        let mut totals = TimingTotals::new(["EXPAND"], true);
        let cost = EspressoCoverCost {
            cubes: 1,
            input_literals: 2,
            output_literals: 1,
            mv_literals: 0,
            total_literals: 3,
            prime_cubes: 1,
        };

        let trace = totals.record(0, Duration::from_millis(250), &cost, 2, 1);

        assert_eq!(totals.entries()[0].total_time, Duration::from_millis(250));
        assert_eq!(totals.entries()[0].calls, 1);
        assert_eq!(
            trace,
            Some("# EXPAND\tTime was 0.25 sec, cost is c=1(0) in=2 out=1 tot=3".to_string())
        );
    }

    #[test]
    fn invalid_inputs_report_errors() {
        let structure = structure();
        let cover = cover(&[&[0, 1, 2, 4, 6, 7]]);

        assert_eq!(
            cover_cost(&cover, &structure, 5, &[false]).unwrap_err(),
            CoverCostError::BinaryVariableCountExceedsVariableCount {
                binary_variable_count: 5,
                variable_count: 4,
            }
        );
        assert_eq!(
            cover_cost(&cover, &structure, 2, &[]).unwrap_err(),
            CoverCostError::PrimeFlagsLengthMismatch {
                cube_count: 1,
                prime_flag_count: 0,
            }
        );
    }

    #[test]
    fn fatal_formats_without_exiting_process() {
        let error = fatal("bad cover");

        assert_eq!(error.message(), "bad cover");
        assert_eq!(error.to_string(), "espresso: bad cover");
    }
}
