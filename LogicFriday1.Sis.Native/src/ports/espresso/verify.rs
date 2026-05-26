//! Native Rust verification utilities for Espresso-style covers.
//!
//! The legacy routines verify that a minimized cover preserves the original
//! care-space behavior, compare PLAs after name-based column permutation, and
//! check that ON, DC, and OFF covers partition the Boolean space. This port uses
//! owned Rust data and returns structured diagnostics instead of printing from
//! the algorithm.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    fn matches(self, value: bool) -> bool {
        match self {
            Self::Zero => !value,
            Self::One => value,
            Self::DontCare => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }

    pub fn minterm(width: usize, assignment: usize) -> Self {
        let literals = (0..width)
            .map(|index| {
                if ((assignment >> index) & 1) == 0 {
                    Literal::Zero
                } else {
                    Literal::One
                }
            })
            .collect();

        Self { literals }
    }

    pub fn width(&self) -> usize {
        self.literals.len()
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    pub fn contains_assignment(&self, assignment: usize) -> bool {
        self.literals.iter().enumerate().all(|(index, literal)| {
            let bit = ((assignment >> index) & 1) != 0;
            literal.matches(bit)
        })
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| {
                *left == Literal::DontCare || *right == Literal::DontCare || left == right
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    width: usize,
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(width: usize, cubes: Vec<Cube>) -> VerifyResult<Self> {
        for cube in &cubes {
            if cube.width() != width {
                return Err(VerifyError::CubeWidth {
                    expected: width,
                    actual: cube.width(),
                });
            }
        }

        Ok(Self { width, cubes })
    }

    pub fn empty(width: usize) -> Self {
        Self {
            width,
            cubes: Vec::new(),
        }
    }

    pub fn universe(width: usize) -> Self {
        Self {
            width,
            cubes: vec![Cube::new(vec![Literal::DontCare; width])],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn cube_count(&self) -> usize {
        self.cubes.len()
    }

    pub fn evaluates(&self, assignment: usize) -> bool {
        self.cubes
            .iter()
            .any(|cube| cube.contains_assignment(assignment))
    }

    pub fn permute(&self, permutation: &[usize]) -> VerifyResult<Self> {
        let mut cubes = Vec::with_capacity(self.cubes.len());
        for cube in &self.cubes {
            let mut literals = Vec::with_capacity(permutation.len());
            for &source in permutation {
                let literal =
                    cube.literals
                        .get(source)
                        .copied()
                        .ok_or(VerifyError::PermutationColumn {
                            column: source,
                            width: self.width,
                        })?;
                literals.push(literal);
            }
            cubes.push(Cube::new(literals));
        }

        Self::new(permutation.len(), cubes)
    }

    fn intersection(&self, other: &Self) -> VerifyResult<Self> {
        ensure_same_width(self, other)?;

        let mut cubes = Vec::new();
        for left in &self.cubes {
            for right in &other.cubes {
                if let Some(cube) = intersect_cubes(left, right) {
                    cubes.push(cube);
                }
            }
        }

        Self::new(self.width, cubes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pla {
    on_set: Cover,
    off_set: Cover,
    dont_care_set: Cover,
    labels: Option<Vec<String>>,
}

impl Pla {
    pub fn new(
        on_set: Cover,
        off_set: Cover,
        dont_care_set: Cover,
        labels: Option<Vec<String>>,
    ) -> VerifyResult<Self> {
        ensure_same_width(&on_set, &off_set)?;
        ensure_same_width(&on_set, &dont_care_set)?;

        if let Some(labels) = &labels {
            if labels.len() != on_set.width() {
                return Err(VerifyError::LabelCount {
                    labels: labels.len(),
                    width: on_set.width(),
                });
            }
        }

        Ok(Self {
            on_set,
            off_set,
            dont_care_set,
            labels,
        })
    }

    pub fn on_set(&self) -> &Cover {
        &self.on_set
    }

    pub fn off_set(&self) -> &Cover {
        &self.off_set
    }

    pub fn dont_care_set(&self) -> &Cover {
        &self.dont_care_set
    }

    pub fn labels(&self) -> Option<&[String]> {
        self.labels.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    diagnostics: Vec<VerificationDiagnostic>,
}

impl VerificationReport {
    pub fn new(diagnostics: Vec<VerificationDiagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn is_error(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    pub fn diagnostics(&self) -> &[VerificationDiagnostic] {
        &self.diagnostics
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationDiagnostic {
    CurrentHasUncoveredMinterm { assignment: usize },
    OriginalHasUncoveredMinterm { assignment: usize },
    MissingColumnNames,
    PlaSizeMismatch { left: usize, right: usize },
    OnSetIntersectsDontCare { assignments: Vec<usize> },
    OnSetIntersectsOffSet { assignments: Vec<usize> },
    DontCareIntersectsOffSet { assignments: Vec<usize> },
    UnspecifiedMinterms { assignments: Vec<usize> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifyError {
    CubeWidth { expected: usize, actual: usize },
    WidthMismatch { left: usize, right: usize },
    LabelCount { labels: usize, width: usize },
    MissingLabel { label: String },
    PermutationColumn { column: usize, width: usize },
    TooManyInputs { width: usize, max_supported: usize },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CubeWidth { expected, actual } => write!(
                f,
                "cube width {actual} does not match cover width {expected}"
            ),
            Self::WidthMismatch { left, right } => {
                write!(f, "cover width mismatch: {left} != {right}")
            }
            Self::LabelCount { labels, width } => {
                write!(f, "{labels} labels supplied for PLA width {width}")
            }
            Self::MissingLabel { label } => write!(f, "source PLA does not contain label {label}"),
            Self::PermutationColumn { column, width } => write!(
                f,
                "permutation column {column} is outside cover width {width}"
            ),
            Self::TooManyInputs {
                width,
                max_supported,
            } => write!(
                f,
                "verification supports at most {max_supported} inputs, got {width}"
            ),
        }
    }
}

impl Error for VerifyError {}

pub type VerifyResult<T> = Result<T, VerifyError>;

pub fn verify(
    current: &Cover,
    original: &Cover,
    dont_care: &Cover,
) -> VerifyResult<VerificationReport> {
    ensure_same_width(current, original)?;
    ensure_same_width(current, dont_care)?;
    validate_width(current.width())?;

    let mut diagnostics = Vec::new();
    if let Some(assignment) = first_uncovered_minterm(current, original, dont_care)? {
        diagnostics.push(VerificationDiagnostic::CurrentHasUncoveredMinterm { assignment });
    }

    if let Some(assignment) = first_uncovered_minterm(original, current, dont_care)? {
        diagnostics.push(VerificationDiagnostic::OriginalHasUncoveredMinterm { assignment });
    }

    Ok(VerificationReport::new(diagnostics))
}

pub fn pla_verify(left: &Pla, right: &Pla) -> VerifyResult<VerificationReport> {
    let (Some(left_labels), Some(right_labels)) = (left.labels(), right.labels()) else {
        return Ok(VerificationReport::new(vec![
            VerificationDiagnostic::MissingColumnNames,
        ]));
    };

    if left_labels.is_empty()
        || right_labels.is_empty()
        || left_labels[0].is_empty()
        || right_labels[0].is_empty()
    {
        return Ok(VerificationReport::new(vec![
            VerificationDiagnostic::MissingColumnNames,
        ]));
    }

    let permuted_left = pla_permute(left, right_labels)?;
    if permuted_left.on_set().width() != right.on_set().width() {
        return Ok(VerificationReport::new(vec![
            VerificationDiagnostic::PlaSizeMismatch {
                left: permuted_left.on_set().width(),
                right: right.on_set().width(),
            },
        ]));
    }

    verify(
        right.on_set(),
        permuted_left.on_set(),
        permuted_left.dont_care_set(),
    )
}

pub fn pla_permute(source: &Pla, target_labels: &[String]) -> VerifyResult<Pla> {
    let Some(source_labels) = source.labels() else {
        return Err(VerifyError::LabelCount {
            labels: 0,
            width: source.on_set().width(),
        });
    };

    let mut permutation = Vec::with_capacity(target_labels.len());
    for label in target_labels {
        let Some(index) = source_labels
            .iter()
            .position(|candidate| candidate == label)
        else {
            return Err(VerifyError::MissingLabel {
                label: label.clone(),
            });
        };
        permutation.push(index);
    }

    Pla::new(
        source.on_set().permute(&permutation)?,
        source.off_set().permute(&permutation)?,
        source.dont_care_set().permute(&permutation)?,
        Some(target_labels.to_vec()),
    )
}

pub fn check_consistency(pla: &Pla) -> VerifyResult<VerificationReport> {
    validate_width(pla.on_set().width())?;

    let mut diagnostics = Vec::new();
    push_overlap(
        &mut diagnostics,
        VerificationDiagnostic::OnSetIntersectsDontCare {
            assignments: intersection_assignments(pla.on_set(), pla.dont_care_set())?,
        },
    );
    push_overlap(
        &mut diagnostics,
        VerificationDiagnostic::OnSetIntersectsOffSet {
            assignments: intersection_assignments(pla.on_set(), pla.off_set())?,
        },
    );
    push_overlap(
        &mut diagnostics,
        VerificationDiagnostic::DontCareIntersectsOffSet {
            assignments: intersection_assignments(pla.dont_care_set(), pla.off_set())?,
        },
    );

    let unspecified = assignments(pla.on_set().width())?
        .filter(|assignment| {
            !pla.on_set().evaluates(*assignment)
                && !pla.dont_care_set().evaluates(*assignment)
                && !pla.off_set().evaluates(*assignment)
        })
        .collect::<Vec<_>>();

    if !unspecified.is_empty() {
        diagnostics.push(VerificationDiagnostic::UnspecifiedMinterms {
            assignments: unspecified,
        });
    }

    Ok(VerificationReport::new(diagnostics))
}

fn first_uncovered_minterm(
    source: &Cover,
    allowed: &Cover,
    dont_care: &Cover,
) -> VerifyResult<Option<usize>> {
    for assignment in assignments(source.width())? {
        if source.evaluates(assignment)
            && !allowed.evaluates(assignment)
            && !dont_care.evaluates(assignment)
        {
            return Ok(Some(assignment));
        }
    }

    Ok(None)
}

fn intersection_assignments(left: &Cover, right: &Cover) -> VerifyResult<Vec<usize>> {
    ensure_same_width(left, right)?;

    if left.intersection(right)?.cube_count() == 0 {
        return Ok(Vec::new());
    }

    Ok(assignments(left.width())?
        .filter(|assignment| left.evaluates(*assignment) && right.evaluates(*assignment))
        .collect())
}

fn intersect_cubes(left: &Cube, right: &Cube) -> Option<Cube> {
    if left.width() != right.width() || !left.intersects(right) {
        return None;
    }

    let literals = left
        .literals
        .iter()
        .zip(&right.literals)
        .map(|(left, right)| match (left, right) {
            (Literal::DontCare, value) => *value,
            (value, Literal::DontCare) => *value,
            (value, _) => *value,
        })
        .collect();

    Some(Cube::new(literals))
}

fn push_overlap(diagnostics: &mut Vec<VerificationDiagnostic>, diagnostic: VerificationDiagnostic) {
    let is_empty = match &diagnostic {
        VerificationDiagnostic::OnSetIntersectsDontCare { assignments }
        | VerificationDiagnostic::OnSetIntersectsOffSet { assignments }
        | VerificationDiagnostic::DontCareIntersectsOffSet { assignments } => {
            assignments.is_empty()
        }
        _ => false,
    };

    if !is_empty {
        diagnostics.push(diagnostic);
    }
}

fn ensure_same_width(left: &Cover, right: &Cover) -> VerifyResult<()> {
    if left.width() != right.width() {
        return Err(VerifyError::WidthMismatch {
            left: left.width(),
            right: right.width(),
        });
    }

    Ok(())
}

fn validate_width(width: usize) -> VerifyResult<()> {
    let max_supported = usize::BITS as usize - 1;
    if width > max_supported {
        return Err(VerifyError::TooManyInputs {
            width,
            max_supported,
        });
    }

    Ok(())
}

fn assignments(width: usize) -> VerifyResult<std::ops::Range<usize>> {
    validate_width(width)?;
    Ok(0..(1usize << width))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(pattern: &str) -> Cube {
        Cube::new(
            pattern
                .chars()
                .map(|ch| match ch {
                    '0' => Literal::Zero,
                    '1' => Literal::One,
                    '-' => Literal::DontCare,
                    _ => panic!("invalid test literal"),
                })
                .collect(),
        )
    }

    fn cover(width: usize, patterns: &[&str]) -> Cover {
        Cover::new(
            width,
            patterns.iter().map(|pattern| cube(pattern)).collect(),
        )
        .unwrap()
    }

    fn pla(on_set: Cover, off_set: Cover, dont_care_set: Cover, labels: &[&str]) -> Pla {
        Pla::new(
            on_set,
            off_set,
            dont_care_set,
            Some(labels.iter().map(|label| label.to_string()).collect()),
        )
        .unwrap()
    }

    #[test]
    fn verify_accepts_equivalent_covers_modulo_dont_care() {
        let current = cover(2, &["1-"]);
        let original = cover(2, &["10"]);
        let dont_care = cover(2, &["11"]);

        let report = verify(&current, &original, &dont_care).unwrap();

        assert!(!report.is_error());
    }

    #[test]
    fn verify_reports_current_growth_outside_original_and_dont_care() {
        let current = cover(2, &["1-"]);
        let original = cover(2, &["10"]);
        let dont_care = Cover::empty(2);

        let report = verify(&current, &original, &dont_care).unwrap();

        assert_eq!(
            report.diagnostics(),
            &[VerificationDiagnostic::CurrentHasUncoveredMinterm { assignment: 3 }]
        );
    }

    #[test]
    fn verify_reports_original_minterm_lost_by_current() {
        let current = cover(2, &["10"]);
        let original = cover(2, &["1-"]);
        let dont_care = Cover::empty(2);

        let report = verify(&current, &original, &dont_care).unwrap();

        assert_eq!(
            report.diagnostics(),
            &[VerificationDiagnostic::OriginalHasUncoveredMinterm { assignment: 3 }]
        );
    }

    #[test]
    fn pla_verify_permutes_left_pla_to_right_label_order() {
        let left = pla(
            cover(2, &["10"]),
            cover(2, &["01"]),
            Cover::empty(2),
            &["a", "b"],
        );
        let right = pla(
            cover(2, &["01"]),
            cover(2, &["10"]),
            Cover::empty(2),
            &["b", "a"],
        );

        let report = pla_verify(&left, &right).unwrap();

        assert!(!report.is_error());
    }

    #[test]
    fn pla_verify_reports_missing_column_names_as_legacy_error_condition() {
        let left = Pla::new(cover(1, &["1"]), cover(1, &["0"]), Cover::empty(1), None).unwrap();
        let right = pla(cover(1, &["1"]), cover(1, &["0"]), Cover::empty(1), &["a"]);

        let report = pla_verify(&left, &right).unwrap();

        assert_eq!(
            report.diagnostics(),
            &[VerificationDiagnostic::MissingColumnNames]
        );
    }

    #[test]
    fn pla_permute_discards_columns_not_requested_by_target_order() {
        let source = pla(
            cover(3, &["101"]),
            cover(3, &["010"]),
            Cover::empty(3),
            &["a", "b", "c"],
        );

        let target_labels = vec!["c".to_string(), "a".to_string()];
        let result = pla_permute(&source, &target_labels).unwrap();

        assert_eq!(result.on_set(), &cover(2, &["11"]));
        assert_eq!(result.off_set(), &cover(2, &["00"]));
        assert_eq!(result.labels(), Some(target_labels.as_slice()));
    }

    #[test]
    fn check_consistency_accepts_partition_of_boolean_space() {
        let pla = pla(
            cover(2, &["10"]),
            cover(2, &["00", "01"]),
            cover(2, &["11"]),
            &["a", "b"],
        );

        let report = check_consistency(&pla).unwrap();

        assert!(!report.is_error());
    }

    #[test]
    fn check_consistency_reports_overlaps_and_unspecified_minterms() {
        let pla = pla(
            cover(2, &["1-"]),
            cover(2, &["00"]),
            cover(2, &["11"]),
            &["a", "b"],
        );

        let report = check_consistency(&pla).unwrap();

        assert_eq!(
            report.diagnostics(),
            &[
                VerificationDiagnostic::OnSetIntersectsDontCare {
                    assignments: vec![3]
                },
                VerificationDiagnostic::UnspecifiedMinterms {
                    assignments: vec![2]
                }
            ]
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("verify.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }
}
