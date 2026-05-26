use std::fmt;
use std::io;
use std::io::Write;

pub const WARNING_PREFIX: &str = "BDD library: warning:";
pub const ERROR_PREFIX: &str = "BDD library: error:";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddNodeRef
{
    reference_count: u8,
}

impl BddNodeRef
{
    pub const fn new(reference_count: u8) -> Self
    {
        Self { reference_count }
    }
}

pub trait BddReference
{
    fn reference_count(&self) -> u8;
}

impl BddReference for BddNodeRef
{
    fn reference_count(&self) -> u8
    {
        self.reference_count
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BddArgumentError
{
    ZeroReferences
    {
        index: usize
    },
}

impl fmt::Display for BddArgumentError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::ZeroReferences { index } => write!(
                formatter,
                "bdd_check_arguments: argument {index} has zero references"
            ),
        }
    }
}

impl std::error::Error for BddArgumentError {}

pub fn warning_message(message: &str) -> String
{
    format!("{WARNING_PREFIX} {message}")
}

pub fn error_message(message: &str) -> String
{
    format!("{ERROR_PREFIX} {message}")
}

pub fn write_warning(mut writer: impl Write, message: &str) -> io::Result<()>
{
    writeln!(writer, "{}", warning_message(message))
}

pub fn write_error(mut writer: impl Write, message: &str) -> io::Result<()>
{
    writeln!(writer, "{}", error_message(message))
}

pub fn bdd_warning(message: &str)
{
    let _ = write_warning(io::stderr().lock(), message);
}

pub fn bdd_fatal(message: &str) -> !
{
    let _ = write_error(io::stderr().lock(), message);
    std::process::exit(1);
}

pub fn check_arguments<'a, T, I>(arguments: I) -> Result<bool, BddArgumentError>
where
    T: BddReference + 'a,
    I: IntoIterator<Item = Option<&'a T>>,
{
    let mut all_valid = true;

    for (index, argument) in arguments.into_iter().enumerate() {
        let Some(argument) = argument else {
            all_valid = false;
            continue;
        };

        if argument.reference_count() == 0 {
            return Err(BddArgumentError::ZeroReferences { index });
        }
    }

    Ok(all_valid)
}

pub fn check_array<'a, T, I>(arguments: I) -> Result<(), BddArgumentError>
where
    T: BddReference + 'a,
    I: IntoIterator<Item = Option<&'a T>>,
{
    for (index, argument) in arguments.into_iter().enumerate() {
        let Some(argument) = argument else {
            break;
        };

        if argument.reference_count() == 0 {
            return Err(BddArgumentError::ZeroReferences { index });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn warning_message_matches_legacy_prefix()
    {
        assert_eq!(
            warning_message("check_assoc: first element in pair is not a positive variable"),
            "BDD library: warning: check_assoc: first element in pair is not a positive variable"
        );
    }

    #[test]
    fn error_message_matches_legacy_prefix()
    {
        assert_eq!(
            error_message("bdd_check_arguments: argument has zero references"),
            "BDD library: error: bdd_check_arguments: argument has zero references"
        );
    }

    #[test]
    fn writers_append_newline_like_fprintf()
    {
        let mut output = Vec::new();

        write_warning(&mut output, "translated to no-op").unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "BDD library: warning: translated to no-op\n"
        );
    }

    #[test]
    fn check_arguments_accepts_all_referenced_nodes()
    {
        let first = BddNodeRef::new(1);
        let second = BddNodeRef::new(255);

        assert_eq!(
            check_arguments([Some(&first), Some(&second)]).unwrap(),
            true
        );
    }

    #[test]
    fn check_arguments_reports_null_arguments_as_invalid()
    {
        let node = BddNodeRef::new(1);

        assert_eq!(check_arguments([Some(&node), None]).unwrap(), false);
    }

    #[test]
    fn check_arguments_rejects_zero_references()
    {
        let valid = BddNodeRef::new(1);
        let invalid = BddNodeRef::new(0);

        assert_eq!(
            check_arguments([Some(&valid), Some(&invalid)]).unwrap_err(),
            BddArgumentError::ZeroReferences { index: 1 }
        );
    }

    #[test]
    fn check_array_stops_at_first_null_sentinel()
    {
        let valid = BddNodeRef::new(1);
        let invalid_after_sentinel = BddNodeRef::new(0);

        assert_eq!(
            check_array([Some(&valid), None, Some(&invalid_after_sentinel)]),
            Ok(())
        );
    }

    #[test]
    fn check_array_rejects_zero_references_before_sentinel()
    {
        let invalid = BddNodeRef::new(0);

        assert_eq!(
            check_array([Some(&invalid)]).unwrap_err(),
            BddArgumentError::ZeroReferences { index: 0 }
        );
    }
}
