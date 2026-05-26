//! Native Rust harness for the legacy `sis/array/arr_main.c` exerciser.
//!
//! The C source was a standalone command-line smoke test for the SIS `array_t`
//! package. This port keeps the observable data-generation, append, sort, and
//! uniq behavior on owned Rust vectors. Timing and process-exit handling are
//! intentionally left to callers.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitialData {
    InOrder,
    ReverseOrder,
    Random,
    RandomRange { range: i32 },
}

impl Default for InitialData {
    fn default() -> Self
    {
        Self::RandomRange { range: 10 }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayExerciseOptions {
    pub data: InitialData,
    pub count: usize,
}

impl Default for ArrayExerciseOptions {
    fn default() -> Self
    {
        Self {
            data: InitialData::default(),
            count: 15,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayExerciseReport {
    pub initial_values: Vec<i32>,
    pub joined_values: Vec<i32>,
    pub sorted_values: Vec<i32>,
    pub unique_values: Vec<i32>,
    pub sort_comparisons: usize,
    pub unique_comparisons: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArrayExerciseError {
    MissingArgument { option: String },
    UnknownOption { option: String },
    UnexpectedArgument { argument: String },
    InvalidCount { value: String },
    InvalidRange { value: String },
    EmptyRandomRange,
    CountTooLarge { count: usize },
}

impl fmt::Display for ArrayExerciseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::MissingArgument { option } => {
                write!(f, "array exerciser error: option {option} requires an argument")
            }
            Self::UnknownOption { option } => {
                write!(f, "array exerciser error: unknown option {option}")
            }
            Self::UnexpectedArgument { argument } => {
                write!(f, "array exerciser error: unexpected argument {argument}")
            }
            Self::InvalidCount { value } => {
                write!(f, "array exerciser error: invalid item count {value}")
            }
            Self::InvalidRange { value } => {
                write!(f, "array exerciser error: invalid random range {value}")
            }
            Self::EmptyRandomRange => {
                write!(f, "array exerciser error: random range must be greater than zero")
            }
            Self::CountTooLarge { count } => {
                write!(f, "array exerciser error: item count {count} exceeds i32 range")
            }
        }
    }
}

impl Error for ArrayExerciseError {}

pub fn parse_options<I, S>(arguments: I) -> Result<ArrayExerciseOptions, ArrayExerciseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = ArrayExerciseOptions::default();
    let mut iterator = arguments.into_iter();

    while let Some(argument) = iterator.next()
    {
        match argument.as_ref()
        {
            "-o" => {
                options.data = InitialData::InOrder;
            }
            "-r" => {
                options.data = InitialData::ReverseOrder;
            }
            "-n" => {
                let value = next_option_value(&mut iterator, "-n")?;
                options.count = value
                    .as_ref()
                    .parse()
                    .map_err(|_| ArrayExerciseError::InvalidCount {
                        value: value.as_ref().to_string(),
                    })?;
            }
            "-b" => {
                let value = next_option_value(&mut iterator, "-b")?;
                let range = value
                    .as_ref()
                    .parse()
                    .map_err(|_| ArrayExerciseError::InvalidRange {
                        value: value.as_ref().to_string(),
                    })?;
                if range <= 0
                {
                    return Err(ArrayExerciseError::EmptyRandomRange);
                }

                options.data = InitialData::RandomRange { range };
            }
            option if option.starts_with('-') => {
                return Err(ArrayExerciseError::UnknownOption {
                    option: option.to_string(),
                });
            }
            argument => {
                return Err(ArrayExerciseError::UnexpectedArgument {
                    argument: argument.to_string(),
                });
            }
        }
    }

    Ok(options)
}

pub fn exercise_array(options: &ArrayExerciseOptions) -> Result<ArrayExerciseReport, ArrayExerciseError>
{
    let initial_values = generate_values(options)?;
    let mut joined_values = initial_values.clone();
    joined_values.extend([2, 1, 0]);

    let (sorted_values, sort_comparisons) = sort_with_comparison_count(joined_values.clone());
    let (unique_values, unique_comparisons) = unique_sorted_with_comparison_count(&sorted_values);

    Ok(ArrayExerciseReport {
        initial_values,
        joined_values,
        sorted_values,
        unique_values,
        sort_comparisons,
        unique_comparisons,
    })
}

pub fn generate_values(options: &ArrayExerciseOptions) -> Result<Vec<i32>, ArrayExerciseError>
{
    if options.count > i32::MAX as usize
    {
        return Err(ArrayExerciseError::CountTooLarge {
            count: options.count,
        });
    }

    let mut random = SisRandom::new(1);
    let mut values = Vec::with_capacity(options.count);
    for index in 0..options.count
    {
        let value = match options.data
        {
            InitialData::InOrder => index as i32,
            InitialData::ReverseOrder => options.count as i32 - index as i32,
            InitialData::Random => random.next(),
            InitialData::RandomRange { range } => {
                if range <= 0
                {
                    return Err(ArrayExerciseError::EmptyRandomRange);
                }

                random.next() % range
            }
        };
        values.push(value);
    }

    Ok(values)
}

pub fn sort_with_comparison_count(mut values: Vec<i32>) -> (Vec<i32>, usize)
{
    let mut comparisons = 0usize;
    values.sort_by(|left, right| {
        comparisons += 1;
        compare_ints(*left, *right)
    });

    (values, comparisons)
}

pub fn unique_sorted_with_comparison_count(values: &[i32]) -> (Vec<i32>, usize)
{
    let mut comparisons = 0usize;
    let mut unique = Vec::with_capacity(values.len());

    for &value in values
    {
        let is_duplicate = unique
            .last()
            .map(|&last| {
                comparisons += 1;
                compare_ints(last, value) == Ordering::Equal
            })
            .unwrap_or(false);

        if !is_duplicate
        {
            unique.push(value);
        }
    }

    (unique, comparisons)
}

pub fn usage(program: &str) -> String
{
    format!(
        "{program}: check out the array package\n\t-o\tinitial data in order\n\t-r\tinitial data in reverse order\n\t-n #\tnumber of values to sort\n\t-b #\tmaximum value for the random values\n"
    )
}

fn compare_ints(left: i32, right: i32) -> Ordering
{
    left.cmp(&right)
}

fn next_option_value<I, S>(iterator: &mut I, option: &str) -> Result<S, ArrayExerciseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    iterator
        .next()
        .ok_or_else(|| ArrayExerciseError::MissingArgument {
            option: option.to_string(),
        })
}

struct SisRandom {
    state: u64,
}

impl SisRandom {
    fn new(seed: u32) -> Self
    {
        Self {
            state: ((seed as u64) << 16) | 0x330e,
        }
    }

    fn next(&mut self) -> i32
    {
        self.state = self
            .state
            .wrapping_mul(0x5deece66d)
            .wrapping_add(0xb)
            & ((1u64 << 48) - 1);

        (self.state >> 17) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_legacy_command()
    {
        let report = exercise_array(&ArrayExerciseOptions::default()).unwrap();

        assert_eq!(
            report.initial_values,
            vec![4, 3, 5, 5, 7, 5, 0, 1, 1, 8, 4, 6, 2, 6, 4]
        );
        assert_eq!(
            report.joined_values,
            vec![4, 3, 5, 5, 7, 5, 0, 1, 1, 8, 4, 6, 2, 6, 4, 2, 1, 0]
        );
        assert_eq!(
            report.unique_values,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(report.unique_comparisons, report.sorted_values.len() - 1);
    }

    #[test]
    fn ordered_and_reverse_generation_follow_c_branches()
    {
        let in_order = ArrayExerciseOptions {
            data: InitialData::InOrder,
            count: 5,
        };
        let reverse = ArrayExerciseOptions {
            data: InitialData::ReverseOrder,
            count: 5,
        };

        assert_eq!(generate_values(&in_order).unwrap(), vec![0, 1, 2, 3, 4]);
        assert_eq!(generate_values(&reverse).unwrap(), vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn sort_and_uniq_count_comparisons()
    {
        let (sorted, sort_comparisons) = sort_with_comparison_count(vec![3, 1, 2, 1, 3]);
        let (unique, unique_comparisons) = unique_sorted_with_comparison_count(&sorted);

        assert_eq!(sorted, vec![1, 1, 2, 3, 3]);
        assert!(sort_comparisons > 0);
        assert_eq!(unique, vec![1, 2, 3]);
        assert_eq!(unique_comparisons, 4);
    }

    #[test]
    fn parses_legacy_options()
    {
        assert_eq!(
            parse_options(["-o", "-n", "3"]).unwrap(),
            ArrayExerciseOptions {
                data: InitialData::InOrder,
                count: 3,
            }
        );
        assert_eq!(
            parse_options(["-b", "4"]).unwrap(),
            ArrayExerciseOptions {
                data: InitialData::RandomRange { range: 4 },
                count: 15,
            }
        );
    }

    #[test]
    fn rejects_bad_options()
    {
        assert_eq!(
            parse_options(["-n"]).unwrap_err(),
            ArrayExerciseError::MissingArgument {
                option: "-n".to_string(),
            }
        );
        assert_eq!(
            parse_options(["-b", "0"]).unwrap_err(),
            ArrayExerciseError::EmptyRandomRange
        );
        assert_eq!(
            parse_options(["input"]).unwrap_err(),
            ArrayExerciseError::UnexpectedArgument {
                argument: "input".to_string(),
            }
        );
    }

    #[test]
    fn usage_preserves_legacy_text()
    {
        assert_eq!(
            usage("arr_main"),
            "arr_main: check out the array package\n\t-o\tinitial data in order\n\t-r\tinitial data in reverse order\n\t-n #\tnumber of values to sort\n\t-b #\tmaximum value for the random values\n"
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port()
    {
        let source = include_str!("arr_main.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
