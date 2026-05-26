//! Native Rust diagnostics for the SIS encoding command support.
//!
//! The C unit only owns the encoding debug flag and a helper that reports how
//! many dichotomies exceed a per-side element bound. This port keeps that
//! behavior as ordinary Rust data and writers instead of global state or command
//! callbacks.

use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::io::Write;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EncodingDiagnostics
{
    debug_enabled: bool,
}

impl EncodingDiagnostics
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn with_debug_enabled(debug_enabled: bool) -> Self
    {
        Self
        {
            debug_enabled,
        }
    }

    pub fn debug_enabled(&self) -> bool
    {
        self.debug_enabled
    }

    pub fn set_debug_enabled(&mut self, debug_enabled: bool)
    {
        self.debug_enabled = debug_enabled;
    }

    pub fn infeasible_count(&self, family: &DicFamily, bound: usize) -> usize
    {
        infeasible_count(family, bound)
    }

    pub fn write_infeasible_count<W>(
        &self,
        writer: &mut W,
        family: &DicFamily,
        bound: usize,
    ) -> io::Result<usize>
    where
        W: Write,
    {
        write_infeasible_count(writer, family, bound)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DicFamily
{
    dichotomies: Vec<Dichotomy>,
}

impl DicFamily
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn from_dichotomies(dichotomies: impl IntoIterator<Item = Dichotomy>) -> Self
    {
        Self
        {
            dichotomies: dichotomies.into_iter().collect(),
        }
    }

    pub fn push(&mut self, dichotomy: Dichotomy)
    {
        self.dichotomies.push(dichotomy);
    }

    pub fn len(&self) -> usize
    {
        self.dichotomies.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.dichotomies.is_empty()
    }

    pub fn dichotomies(&self) -> &[Dichotomy]
    {
        &self.dichotomies
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Dichotomy
{
    lhs: BTreeSet<usize>,
    rhs: BTreeSet<usize>,
}

impl Dichotomy
{
    pub fn new(
        lhs: impl IntoIterator<Item = usize>,
        rhs: impl IntoIterator<Item = usize>,
    ) -> Self
    {
        Self
        {
            lhs: lhs.into_iter().collect(),
            rhs: rhs.into_iter().collect(),
        }
    }

    pub fn empty() -> Self
    {
        Self::default()
    }

    pub fn lhs(&self) -> &BTreeSet<usize>
    {
        &self.lhs
    }

    pub fn rhs(&self) -> &BTreeSet<usize>
    {
        &self.rhs
    }

    pub fn lhs_len(&self) -> usize
    {
        self.lhs.len()
    }

    pub fn rhs_len(&self) -> usize
    {
        self.rhs.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InfeasibleCountReport
{
    pub reduced_by: usize,
}

impl InfeasibleCountReport
{
    pub fn new(reduced_by: usize) -> Self
    {
        Self
        {
            reduced_by,
        }
    }
}

impl fmt::Display for InfeasibleCountReport
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        writeln!(f, "Reduce list by {}", self.reduced_by)
    }
}

pub fn infeasible_count(family: &DicFamily, bound: usize) -> usize
{
    family
        .dichotomies()
        .iter()
        .filter(|dichotomy| dichotomy.lhs_len() > bound || dichotomy.rhs_len() > bound)
        .count()
}

pub fn infeasible_count_report(family: &DicFamily, bound: usize) -> InfeasibleCountReport
{
    InfeasibleCountReport::new(infeasible_count(family, bound))
}

pub fn write_infeasible_count<W>(
    writer: &mut W,
    family: &DicFamily,
    bound: usize,
) -> io::Result<usize>
where
    W: Write,
{
    let report = infeasible_count_report(family, bound);
    write!(writer, "{report}")?;
    Ok(report.reduced_by)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn diagnostics_debug_flag_defaults_to_disabled()
    {
        let mut diagnostics = EncodingDiagnostics::new();

        assert!(!diagnostics.debug_enabled());

        diagnostics.set_debug_enabled(true);

        assert!(diagnostics.debug_enabled());
        assert_eq!(
            EncodingDiagnostics::with_debug_enabled(false),
            EncodingDiagnostics::new()
        );
    }

    #[test]
    fn counts_dichotomies_with_either_side_above_bound()
    {
        let family = DicFamily::from_dichotomies(
            [
                Dichotomy::new([0, 1], [2]),
                Dichotomy::new([0], [1, 2, 3]),
                Dichotomy::new([1, 2], [3, 4]),
                Dichotomy::new([0, 1, 2], [3]),
            ],
        );

        assert_eq!(infeasible_count(&family, 2), 2);
        assert_eq!(infeasible_count(&family, 3), 0);
    }

    #[test]
    fn equal_to_bound_is_not_infeasible()
    {
        let family = DicFamily::from_dichotomies(
            [
                Dichotomy::new([0, 1], [2, 3]),
                Dichotomy::new([], [4]),
            ],
        );

        assert_eq!(infeasible_count(&family, 2), 0);
    }

    #[test]
    fn duplicate_set_members_are_counted_once()
    {
        let dichotomy = Dichotomy::new([0, 0, 1], [2, 2, 3]);
        let family = DicFamily::from_dichotomies([dichotomy]);

        assert_eq!(family.dichotomies()[0].lhs_len(), 2);
        assert_eq!(family.dichotomies()[0].rhs_len(), 2);
        assert_eq!(infeasible_count(&family, 2), 0);
    }

    #[test]
    fn writes_legacy_report_text()
    {
        let family = DicFamily::from_dichotomies(
            [
                Dichotomy::new([0, 1, 2], [3]),
                Dichotomy::new([0], [1]),
            ],
        );
        let mut output = Vec::new();

        let reduced_by = write_infeasible_count(&mut output, &family, 2).unwrap();

        assert_eq!(reduced_by, 1);
        assert_eq!(String::from_utf8(output).unwrap(), "Reduce list by 1\n");
    }

    #[test]
    fn diagnostics_delegate_to_report_helpers()
    {
        let diagnostics = EncodingDiagnostics::new();
        let family = DicFamily::from_dichotomies([Dichotomy::new([0, 1, 2], [3])]);
        let mut output = Vec::new();

        assert_eq!(diagnostics.infeasible_count(&family, 2), 1);
        assert_eq!(
            diagnostics
                .write_infeasible_count(&mut output, &family, 2)
                .unwrap(),
            1
        );
        assert_eq!(String::from_utf8(output).unwrap(), "Reduce list by 1\n");
    }
}
