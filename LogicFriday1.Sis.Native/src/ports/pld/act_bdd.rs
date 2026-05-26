//! Native ACT BDD construction over owned Rust cover data.
//!
//! The original implementation creates ACT decision vertices from SIS sparse
//! matrix rows and a global terminal table. This port keeps the construction
//! behavior explicit: cube conjunctions, binate joins, ordered OR chaining, and
//! trace output are represented with owned Rust values. Direct SIS sparse-matrix
//! integration is left to higher-level native ports.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Negative,
    Positive,
    DontCare,
}

impl LiteralPhase {
    pub fn from_sis_literal(value: i32) -> ActBddResult<Self> {
        match value {
            0 => Ok(Self::Negative),
            1 => Ok(Self::Positive),
            2 => Ok(Self::DontCare),
            _ => Err(ActBddError::InvalidLiteralValue(value)),
        }
    }

    pub fn is_present(self) -> bool {
        self != Self::DontCare
    }

    pub fn matches(self, value: bool) -> bool {
        match self {
            Self::Negative => !value,
            Self::Positive => value,
            Self::DontCare => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoverLiteral {
    pub column: usize,
    pub phase: LiteralPhase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverMatrix {
    column_names: Vec<String>,
    rows: Vec<Vec<LiteralPhase>>,
}

impl CoverMatrix {
    pub fn new(column_names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            column_names: column_names.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
        }
    }

    pub fn from_rows(
        column_names: impl IntoIterator<Item = impl Into<String>>,
        rows: Vec<Vec<LiteralPhase>>,
    ) -> ActBddResult<Self> {
        let mut matrix = Self::new(column_names);
        for row in rows {
            matrix.push_row(row)?;
        }
        Ok(matrix)
    }

    pub fn push_row(&mut self, row: Vec<LiteralPhase>) -> ActBddResult<()> {
        if row.len() != self.column_names.len() {
            return Err(ActBddError::RowWidthMismatch {
                expected: self.column_names.len(),
                actual: row.len(),
            });
        }

        self.rows.push(row);
        Ok(())
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.column_names.len()
    }

    pub fn rows(&self) -> &[Vec<LiteralPhase>] {
        &self.rows
    }

    pub fn column_names(&self) -> &[String] {
        &self.column_names
    }

    pub fn evaluate(&self, inputs: &[bool]) -> ActBddResult<bool> {
        if inputs.len() != self.column_count() {
            return Err(ActBddError::InputWidthMismatch {
                expected: self.column_count(),
                actual: inputs.len(),
            });
        }

        Ok(self.rows.iter().any(|row| {
            row.iter()
                .zip(inputs)
                .all(|(phase, value)| phase.matches(*value))
        }))
    }

    fn column_name(&self, column: usize) -> ActBddResult<String> {
        let name = self
            .column_names
            .get(column)
            .ok_or(ActBddError::UnknownColumn(column))?;
        if name.is_empty() {
            return Err(ActBddError::EmptyColumnName { column });
        }

        Ok(name.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Act {
    Terminal(bool),
    Decision {
        column: usize,
        name: String,
        low: Box<Act>,
        high: Box<Act>,
        index_size: usize,
    },
}

impl Act {
    pub fn init() -> Self {
        Self::Terminal(false)
    }

    pub fn one() -> Self {
        Self::Terminal(true)
    }

    pub fn zero() -> Self {
        Self::Terminal(false)
    }

    pub fn evaluate(&self, inputs: &[bool]) -> ActBddResult<bool> {
        match self {
            Self::Terminal(value) => Ok(*value),
            Self::Decision {
                column, low, high, ..
            } => {
                let value = inputs
                    .get(*column)
                    .copied()
                    .ok_or(ActBddError::UnknownColumn(*column))?;
                if value {
                    high.evaluate(inputs)
                } else {
                    low.evaluate(inputs)
                }
            }
        }
    }

    pub fn index_size(&self) -> usize {
        match self {
            Self::Terminal(_) => 0,
            Self::Decision { index_size, .. } => *index_size,
        }
    }

    pub fn trace(&self) -> Vec<String> {
        let mut lines = Vec::new();
        self.trace_into(&mut lines);
        lines
    }

    fn replace_false_terminals(&mut self, replacement: &Act) {
        match self {
            Self::Terminal(false) => {
                *self = replacement.clone();
            }
            Self::Terminal(true) => {}
            Self::Decision { low, high, .. } => {
                low.replace_false_terminals(replacement);
                high.replace_false_terminals(replacement);
            }
        }
    }

    fn trace_into(&self, lines: &mut Vec<String>) {
        match self {
            Self::Terminal(value) => {
                lines.push(format!("END- {}", usize::from(*value)));
            }
            Self::Decision {
                name, low, high, ..
            } => {
                lines.push(format!("current variable - {name}"));
                low.trace_into(lines);
                high.trace_into(lines);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActBddError {
    MissingNativePorts { operation: &'static str },
    RowWidthMismatch { expected: usize, actual: usize },
    InputWidthMismatch { expected: usize, actual: usize },
    UnknownColumn(usize),
    EmptyColumnName { column: usize },
    InvalidLiteralValue(i32),
    ExpectedSingleCube { actual: usize },
    EmptyCube,
    InvalidLiteralPhase { phase: LiteralPhase },
    MismatchedOrInputs { acts: usize, cover_literals: usize },
}

impl fmt::Display for ActBddError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
            Self::RowWidthMismatch { expected, actual } => {
                write!(f, "cover row has width {actual}, expected {expected}")
            }
            Self::InputWidthMismatch { expected, actual } => {
                write!(f, "input vector has width {actual}, expected {expected}")
            }
            Self::UnknownColumn(column) => write!(f, "unknown cover column {column}"),
            Self::EmptyColumnName { column } => {
                write!(f, "cover column {column} has no fanin name")
            }
            Self::InvalidLiteralValue(value) => {
                write!(f, "invalid SIS literal value {value}; expected 0, 1, or 2")
            }
            Self::ExpectedSingleCube { actual } => {
                write!(f, "cube ACT construction expected one row, got {actual}")
            }
            Self::EmptyCube => write!(f, "cannot build a cube ACT from an empty cube"),
            Self::InvalidLiteralPhase { phase } => {
                write!(f, "literal phase {phase:?} cannot create an ACT decision")
            }
            Self::MismatchedOrInputs {
                acts,
                cover_literals,
            } => {
                write!(
                    f,
                    "OR ACT construction got {acts} branches and {cover_literals} cover literals"
                )
            }
        }
    }
}

impl Error for ActBddError {}

pub type ActBddResult<T> = Result<T, ActBddError>;

pub fn act_f(matrix: &CoverMatrix) -> ActBddResult<Act> {
    if matrix.row_count() != 1 {
        return Err(ActBddError::ExpectedSingleCube {
            actual: matrix.row_count(),
        });
    }

    let row = &matrix.rows()[0];
    let present = row
        .iter()
        .enumerate()
        .filter_map(|(column, phase)| phase.is_present().then_some((column, *phase)))
        .collect::<Vec<_>>();

    if present.is_empty() {
        return Err(ActBddError::EmptyCube);
    }

    let mut act = Act::one();
    let skip_position = present
        .iter()
        .position(|(_, phase)| *phase == LiteralPhase::Positive);

    if let Some(position) = skip_position {
        let (column, phase) = present[position];
        act = act_and_literal(
            CoverLiteral { column, phase },
            matrix.column_name(column)?,
            act,
        )?;
    }

    for (index, (column, phase)) in present.into_iter().enumerate() {
        if Some(index) == skip_position {
            continue;
        }

        act = act_and_literal(
            CoverLiteral { column, phase },
            matrix.column_name(column)?,
            act,
        )?;
    }

    Ok(set_index_size(
        act,
        row.iter().filter(|phase| phase.is_present()).count(),
    ))
}

pub fn act_and_literal(
    literal: CoverLiteral,
    name: String,
    continuation: Act,
) -> ActBddResult<Act> {
    let index_size = continuation.index_size() + 1;
    match literal.phase {
        LiteralPhase::Negative => Ok(Act::Decision {
            column: literal.column,
            name,
            low: Box::new(continuation),
            high: Box::new(Act::zero()),
            index_size,
        }),
        LiteralPhase::Positive => Ok(Act::Decision {
            column: literal.column,
            name,
            low: Box::new(Act::zero()),
            high: Box::new(continuation),
            index_size,
        }),
        LiteralPhase::DontCare => Err(ActBddError::InvalidLiteralPhase {
            phase: literal.phase,
        }),
    }
}

pub fn or_act_f(
    acts: impl IntoIterator<Item = Act>,
    cover_literals: impl IntoIterator<Item = CoverLiteral>,
    matrices: &[CoverMatrix],
) -> ActBddResult<Act> {
    let acts = acts.into_iter().collect::<Vec<_>>();
    let cover_literals = cover_literals.into_iter().collect::<Vec<_>>();
    if acts.len() != cover_literals.len() {
        return Err(ActBddError::MismatchedOrInputs {
            acts: acts.len(),
            cover_literals: cover_literals.len(),
        });
    }
    if matrices.len() < cover_literals.len() {
        return Err(ActBddError::MismatchedOrInputs {
            acts: matrices.len(),
            cover_literals: cover_literals.len(),
        });
    }

    let mut branches = acts
        .into_iter()
        .zip(cover_literals)
        .enumerate()
        .map(|(index, (act, literal))| {
            let name = matrices[index].column_name(literal.column)?;
            act_and_literal(literal, name, act)
        })
        .collect::<ActBddResult<Vec<_>>>()?;

    branches.sort_by_key(Act::index_size);
    join_ordered_acts(branches)
}

pub fn and_act(low: Act, high: Act, column: usize, name: impl Into<String>) -> Act {
    let index_size = low.index_size() + high.index_size() + 1;
    Act::Decision {
        column,
        name: name.into(),
        low: Box::new(low),
        high: Box::new(high),
        index_size,
    }
}

pub fn act_bdd_sis_integration_blocked<Matrix>(_matrix: &Matrix) -> ActBddResult<Act> {
    Err(ActBddError::MissingNativePorts {
        operation: "ACT BDD SIS sparse-matrix integration",
    })
}

pub fn trace_act(act: &Act) -> Vec<String> {
    act.trace()
}

fn join_ordered_acts(mut branches: Vec<Act>) -> ActBddResult<Act> {
    if branches.is_empty() {
        return Ok(Act::zero());
    }

    for index in (1..branches.len()).rev() {
        let replacement = branches[index].clone();
        branches[index - 1].replace_false_terminals(&replacement);
    }

    Ok(branches.remove(0))
}

fn set_index_size(act: Act, size: usize) -> Act {
    match act {
        Act::Terminal(value) => Act::Terminal(value),
        Act::Decision {
            column,
            name,
            low,
            high,
            ..
        } => Act::Decision {
            column,
            name,
            low,
            high,
            index_size: size,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phases(values: &[i32]) -> Vec<LiteralPhase> {
        values
            .iter()
            .map(|value| LiteralPhase::from_sis_literal(*value).unwrap())
            .collect()
    }

    fn matrix(rows: &[&[i32]]) -> CoverMatrix {
        CoverMatrix::from_rows(
            ["a", "b", "c"],
            rows.iter().map(|row| phases(row)).collect(),
        )
        .unwrap()
    }

    fn assert_equivalent_to_cover(act: &Act, cover: &CoverMatrix) {
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let inputs = [a, b, c];
                    assert_eq!(
                        act.evaluate(&inputs).unwrap(),
                        cover.evaluate(&inputs).unwrap(),
                        "inputs: {inputs:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn act_f_builds_single_cube_conjunction() {
        let cover = matrix(&[&[1, 0, 2]]);
        let act = act_f(&cover).unwrap();

        assert_equivalent_to_cover(&act, &cover);
        assert_eq!(act.index_size(), 2);
        assert_eq!(act.evaluate(&[true, false, false]).unwrap(), true);
        assert_eq!(act.evaluate(&[true, true, false]).unwrap(), false);
    }

    #[test]
    fn act_f_places_first_positive_literal_at_deepest_decision() {
        let act = act_f(&matrix(&[&[1, 0, 1]])).unwrap();

        match act {
            Act::Decision { high, .. } => {
                let Act::Decision { low, .. } = *high else {
                    panic!("expected wrapped negative decision");
                };
                let Act::Decision { column, .. } = *low else {
                    panic!("expected deepest positive decision");
                };
                assert_eq!(column, 0);
            }
            _ => panic!("expected decision ACT"),
        }
    }

    #[test]
    fn act_f_rejects_non_single_cube_input() {
        assert!(matches!(
            act_f(&matrix(&[&[1, 2, 2], &[2, 1, 2]])),
            Err(ActBddError::ExpectedSingleCube { actual: 2 })
        ));
    }

    #[test]
    fn and_act_builds_binate_decision_with_summed_index_size() {
        let low = act_f(&matrix(&[&[0, 2, 2]])).unwrap();
        let high = act_f(&matrix(&[&[2, 1, 2]])).unwrap();
        let act = and_act(low, high, 2, "c");

        assert_eq!(act.index_size(), 3);
        assert_eq!(act.evaluate(&[false, false, false]).unwrap(), true);
        assert_eq!(act.evaluate(&[false, true, true]).unwrap(), true);
        assert_eq!(act.evaluate(&[true, false, false]).unwrap(), false);
    }

    #[test]
    fn or_act_f_prefixes_branches_sorts_by_size_and_matches_union() {
        let short_cover = matrix(&[&[1, 2, 2]]);
        let long_cover = matrix(&[&[2, 1, 0]]);
        let short = act_f(&short_cover).unwrap();
        let long = act_f(&long_cover).unwrap();
        let act = or_act_f(
            [long, short],
            [
                CoverLiteral {
                    column: 2,
                    phase: LiteralPhase::Positive,
                },
                CoverLiteral {
                    column: 1,
                    phase: LiteralPhase::Negative,
                },
            ],
            &[long_cover, short_cover],
        )
        .unwrap();

        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let inputs = [a, b, c];
                    let expected = (c && b && !c) || (!b && a);
                    assert_eq!(act.evaluate(&inputs).unwrap(), expected);
                }
            }
        }

        assert!(matches!(act, Act::Decision { column: 1, .. }));
    }

    #[test]
    fn trace_act_reports_decisions_and_terminals() {
        let act = act_f(&matrix(&[&[1, 2, 2]])).unwrap();

        assert_eq!(
            trace_act(&act),
            vec![
                "current variable - a".to_owned(),
                "END- 0".to_owned(),
                "END- 1".to_owned(),
            ]
        );
    }

    #[test]
    fn blocked_sis_integration_returns_generic_diagnostic() {
        let error = act_bdd_sis_integration_blocked(&()).unwrap_err();

        assert!(matches!(error, ActBddError::MissingNativePorts { .. }));
        assert!(
            error
                .to_string()
                .contains("requires native SIS prerequisite ports")
        );
    }

    #[test]
    fn no_legacy_c_abi_or_task_metadata_tokens_are_present() {
        let text = include_str!("act_bdd.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("be", "ad", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday1", "-")));
    }
}
