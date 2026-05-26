//! Native Rust profile-printing helpers for the CMU BDD package.
//!
//! The legacy C file formats node and function profile counts produced by the
//! BDD profile routines. This module keeps that responsibility separate from
//! graph traversal so callers can provide counts from any native BDD manager.

use std::fmt;
use std::io;
use std::io::Write;

pub const DEFAULT_MINIMUM_HISTOGRAM_WIDTH: usize = 20;
pub const DEFAULT_LEAF_LABEL: &str = "leaf";
pub const DEFAULT_OVERFLOW_MESSAGE: &str = "overflow\n";

pub trait BddProfileManager {
    type BddRef: Copy;
    type Error;

    fn variable_count(&self) -> usize;

    fn variable_at_level(&self, level: usize) -> Option<Self::BddRef>;

    fn variable_name(&self, variable: Self::BddRef) -> String;

    fn node_profile(&self, function: Self::BddRef, negout: bool)
        -> Result<Vec<usize>, Self::Error>;

    fn node_profile_multiple(
        &self,
        functions: &[Self::BddRef],
        negout: bool,
    ) -> Result<Vec<usize>, Self::Error>;

    fn function_profile(&self, function: Self::BddRef) -> Result<Vec<usize>, Self::Error>;

    fn function_profile_multiple(
        &self,
        functions: &[Self::BddRef],
    ) -> Result<Vec<usize>, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfilePrintError<E> {
    MissingLeafCount {
        expected_len: usize,
        actual_len: usize,
    },
    Provider(E),
    Write(String),
}

impl<E> fmt::Display for ProfilePrintError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLeafCount {
                expected_len,
                actual_len,
            } => write!(
                formatter,
                "profile count array must contain at least {expected_len} entries, got {actual_len}"
            ),
            Self::Provider(error) => write!(formatter, "{error}"),
            Self::Write(error) => write!(formatter, "{error}"),
        }
    }
}

impl<E> std::error::Error for ProfilePrintError<E> where E: fmt::Debug + fmt::Display {}

impl<E> From<io::Error> for ProfilePrintError<E> {
    fn from(error: io::Error) -> Self {
        Self::Write(error.to_string())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileLine {
    pub label: String,
    pub count: usize,
    pub histogram_marks: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileReport {
    lines: Vec<ProfileLine>,
    total: usize,
}

impl ProfileReport {
    pub fn lines(&self) -> &[ProfileLine] {
        &self.lines
    }

    pub const fn total(&self) -> usize {
        self.total
    }
}

pub fn print_profile_aux<I, S>(
    level_counts: &[usize],
    variable_names: I,
    line_length: usize,
    writer: impl Write,
) -> Result<ProfileReport, ProfilePrintError<io::Error>>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let names: Vec<String> = variable_names.into_iter().map(Into::into).collect();
    let report = build_profile_report(level_counts, names.iter().map(String::as_str), line_length)?;

    write_profile_report(&report, writer)?;

    Ok(report)
}

pub fn build_profile_report<'a, I>(
    level_counts: &[usize],
    variable_names: I,
    line_length: usize,
) -> Result<ProfileReport, ProfilePrintError<io::Error>>
where
    I: IntoIterator<Item = &'a str>,
{
    let names: Vec<&str> = variable_names.into_iter().collect();
    let expected_len = names.len() + 1;

    if level_counts.len() < expected_len {
        return Err(ProfilePrintError::MissingLeafCount {
            expected_len,
            actual_len: level_counts.len(),
        });
    }

    let leaf_count = level_counts[names.len()];
    let mut max_prefix_len = DEFAULT_LEAF_LABEL.len() + 1;
    let mut max_profile_width = leaf_count;
    let mut total = leaf_count;

    for (name, count) in names
        .iter()
        .zip(level_counts.iter())
        .filter(|(_, count)| **count != 0)
    {
        max_prefix_len = max_prefix_len.max(name.len() + count.to_string().len());
        max_profile_width = max_profile_width.max(*count);
        total += *count;
    }

    let histogram_column = max_prefix_len + 3;
    let histogram_width = line_length
        .saturating_sub(histogram_column)
        .saturating_sub(1)
        .max(DEFAULT_MINIMUM_HISTOGRAM_WIDTH);
    let profile_scale = if histogram_width >= max_profile_width {
        1
    } else {
        max_profile_width.div_ceil(histogram_width).max(1)
    };

    let mut lines = Vec::new();

    for (name, count) in names
        .iter()
        .zip(level_counts.iter())
        .filter(|(_, count)| **count != 0)
    {
        lines.push(ProfileLine {
            label: (*name).to_string(),
            count: *count,
            histogram_marks: *count / profile_scale,
        });
    }

    lines.push(ProfileLine {
        label: DEFAULT_LEAF_LABEL.to_string(),
        count: leaf_count,
        histogram_marks: leaf_count / profile_scale,
    });

    Ok(ProfileReport { lines, total })
}

pub fn write_profile_report(
    report: &ProfileReport,
    mut writer: impl Write,
) -> Result<(), ProfilePrintError<io::Error>> {
    let max_prefix_len = report
        .lines
        .iter()
        .map(|line| line.label.len() + line.count.to_string().len())
        .max()
        .unwrap_or(DEFAULT_LEAF_LABEL.len() + 1);

    for line in &report.lines {
        write!(writer, "{}:", line.label)?;
        let padding =
            max_prefix_len.saturating_sub(line.label.len() + line.count.to_string().len()) + 1;
        write!(writer, "{:padding$}{} ", "", line.count)?;
        write_repeated(&mut writer, b'#', line.histogram_marks)?;
        writeln!(writer)?;
    }

    writeln!(writer, "Total: {}", report.total)?;
    Ok(())
}

pub fn print_profile<M>(
    manager: &M,
    function: M::BddRef,
    line_length: usize,
    writer: impl Write,
) -> Result<ProfileReport, ProfilePrintError<M::Error>>
where
    M: BddProfileManager,
{
    let counts = manager
        .node_profile(function, true)
        .map_err(ProfilePrintError::Provider)?;

    print_counts_from_manager(manager, &counts, line_length, writer)
}

pub fn print_profile_multiple<M>(
    manager: &M,
    functions: &[M::BddRef],
    line_length: usize,
    writer: impl Write,
) -> Result<ProfileReport, ProfilePrintError<M::Error>>
where
    M: BddProfileManager,
{
    let counts = manager
        .node_profile_multiple(functions, true)
        .map_err(ProfilePrintError::Provider)?;

    print_counts_from_manager(manager, &counts, line_length, writer)
}

pub fn print_function_profile<M>(
    manager: &M,
    function: M::BddRef,
    line_length: usize,
    writer: impl Write,
) -> Result<ProfileReport, ProfilePrintError<M::Error>>
where
    M: BddProfileManager,
{
    let counts = manager
        .function_profile(function)
        .map_err(ProfilePrintError::Provider)?;

    print_counts_from_manager(manager, &counts, line_length, writer)
}

pub fn print_function_profile_multiple<M>(
    manager: &M,
    functions: &[M::BddRef],
    line_length: usize,
    writer: impl Write,
) -> Result<ProfileReport, ProfilePrintError<M::Error>>
where
    M: BddProfileManager,
{
    let counts = manager
        .function_profile_multiple(functions)
        .map_err(ProfilePrintError::Provider)?;

    print_counts_from_manager(manager, &counts, line_length, writer)
}

pub fn write_overflow(mut writer: impl Write) -> io::Result<()> {
    writer.write_all(DEFAULT_OVERFLOW_MESSAGE.as_bytes())
}

fn print_counts_from_manager<M>(
    manager: &M,
    counts: &[usize],
    line_length: usize,
    writer: impl Write,
) -> Result<ProfileReport, ProfilePrintError<M::Error>>
where
    M: BddProfileManager,
{
    let names: Vec<String> = (0..manager.variable_count())
        .map(|level| {
            manager
                .variable_at_level(level)
                .map(|variable| manager.variable_name(variable))
                .unwrap_or_else(|| level.to_string())
        })
        .collect();

    let report = build_profile_report(counts, names.iter().map(String::as_str), line_length)
        .map_err(convert_profile_error)?;

    write_profile_report(&report, writer).map_err(convert_profile_error)?;

    Ok(report)
}

fn write_repeated(mut writer: impl Write, byte: u8, count: usize) -> io::Result<()> {
    for _ in 0..count {
        writer.write_all(&[byte])?;
    }

    Ok(())
}

fn convert_profile_error<E>(error: ProfilePrintError<io::Error>) -> ProfilePrintError<E> {
    match error {
        ProfilePrintError::MissingLeafCount {
            expected_len,
            actual_len,
        } => ProfilePrintError::MissingLeafCount {
            expected_len,
            actual_len,
        },
        ProfilePrintError::Provider(error) => ProfilePrintError::Write(error.to_string()),
        ProfilePrintError::Write(error) => ProfilePrintError::Write(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct FakeManager {
        variable_names: Vec<&'static str>,
        node_profile: Vec<usize>,
        function_profile: Vec<usize>,
    }

    impl BddProfileManager for FakeManager {
        type BddRef = usize;
        type Error = &'static str;

        fn variable_count(&self) -> usize {
            self.variable_names.len()
        }

        fn variable_at_level(&self, level: usize) -> Option<Self::BddRef> {
            Some(level)
        }

        fn variable_name(&self, variable: Self::BddRef) -> String {
            self.variable_names[variable].to_string()
        }

        fn node_profile(
            &self,
            _function: Self::BddRef,
            negout: bool,
        ) -> Result<Vec<usize>, Self::Error> {
            assert!(negout);
            Ok(self.node_profile.clone())
        }

        fn node_profile_multiple(
            &self,
            functions: &[Self::BddRef],
            negout: bool,
        ) -> Result<Vec<usize>, Self::Error> {
            assert_eq!(functions, [1, 2]);
            assert!(negout);
            Ok(self.node_profile.clone())
        }

        fn function_profile(&self, _function: Self::BddRef) -> Result<Vec<usize>, Self::Error> {
            Ok(self.function_profile.clone())
        }

        fn function_profile_multiple(
            &self,
            functions: &[Self::BddRef],
        ) -> Result<Vec<usize>, Self::Error> {
            assert_eq!(functions, [3, 4]);
            Ok(self.function_profile.clone())
        }
    }

    #[test]
    fn print_profile_aux_matches_legacy_alignment_and_total() {
        let mut output = Vec::new();

        let report =
            print_profile_aux([2, 0, 5, 1].as_slice(), ["x", "y", "long"], 80, &mut output)
                .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "x:    2 ##\nlong: 5 #####\nleaf: 1 #\nTotal: 8\n"
        );
        assert_eq!(report.total(), 8);
        assert_eq!(
            report.lines(),
            [
                ProfileLine {
                    label: "x".to_string(),
                    count: 2,
                    histogram_marks: 2
                },
                ProfileLine {
                    label: "long".to_string(),
                    count: 5,
                    histogram_marks: 5
                },
                ProfileLine {
                    label: "leaf".to_string(),
                    count: 1,
                    histogram_marks: 1
                }
            ]
        );
    }

    #[test]
    fn histogram_is_scaled_to_line_length_with_legacy_minimum_width() {
        let report = build_profile_report(&[100, 40, 20], ["a", "b"], 10).unwrap();

        assert_eq!(report.lines()[0].histogram_marks, 20);
        assert_eq!(report.lines()[1].histogram_marks, 8);
        assert_eq!(report.lines()[2].histogram_marks, 4);
    }

    #[test]
    fn zero_variable_counts_are_omitted_but_leaf_count_is_printed() {
        let mut output = Vec::new();

        print_profile_aux(&[0, 0, 3], ["a", "b"], 80, &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "leaf: 3 ###\nTotal: 3\n"
        );
    }

    #[test]
    fn missing_leaf_count_is_rejected() {
        assert!(matches!(
            build_profile_report(&[1], ["a"], 80).unwrap_err(),
            ProfilePrintError::MissingLeafCount {
                expected_len: 2,
                actual_len: 1
            }
        ));
    }

    #[test]
    fn manager_wrappers_request_the_expected_profile_kind() {
        let manager = FakeManager {
            variable_names: vec!["a", "b"],
            node_profile: vec![1, 2, 3],
            function_profile: vec![3, 2, 1],
        };
        let mut output = Vec::new();

        let report = print_profile(&manager, 1, 80, &mut output).unwrap();

        assert_eq!(report.total(), 6);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "a:    1 #\nb:    2 ##\nleaf: 3 ###\nTotal: 6\n"
        );
    }

    #[test]
    fn multiple_and_function_wrappers_forward_to_provider() {
        let manager = FakeManager {
            variable_names: vec!["a", "b"],
            node_profile: vec![1, 0, 1],
            function_profile: vec![0, 5, 1],
        };
        let mut output = Vec::new();

        let node_report = print_profile_multiple(&manager, &[1, 2], 80, &mut output).unwrap();
        let function_report =
            print_function_profile_multiple(&manager, &[3, 4], 80, &mut output).unwrap();

        assert_eq!(node_report.total(), 2);
        assert_eq!(function_report.total(), 6);
    }

    #[test]
    fn write_overflow_matches_legacy_fallback() {
        let mut output = Vec::new();

        write_overflow(&mut output).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "overflow\n");
    }
}
