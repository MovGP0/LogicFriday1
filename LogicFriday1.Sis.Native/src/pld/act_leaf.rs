//! Owned Rust model for unate ACT leaf construction.
//!
//! The original C routine receives an SIS sparse matrix with literal phases in
//! `user_word`, solves a column cover, recursively splits the matrix, and joins
//! the resulting ACTs. This port keeps that behavior on owned Rust data and
//! leaves direct SIS sparse-matrix/global-terminal integration to higher-level
//! native ports.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Negative,
    Positive,
    DontCare,
}

impl LiteralPhase {
    pub fn from_sis_literal(value: i32) -> ActLeafResult<Self> {
        match value {
            0 => Ok(Self::Negative),
            1 => Ok(Self::Positive),
            2 => Ok(Self::DontCare),
            _ => Err(ActLeafError::InvalidLiteralValue(value)),
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnateCover {
    column_names: Vec<String>,
    rows: Vec<Vec<LiteralPhase>>,
}

impl UnateCover {
    pub fn new(column_names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            column_names: column_names.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
        }
    }

    pub fn from_rows(
        column_names: impl IntoIterator<Item = impl Into<String>>,
        rows: Vec<Vec<LiteralPhase>>,
    ) -> ActLeafResult<Self> {
        let mut cover = Self::new(column_names);
        for row in rows {
            cover.push_row(row)?;
        }
        Ok(cover)
    }

    pub fn push_row(&mut self, row: Vec<LiteralPhase>) -> ActLeafResult<()> {
        if row.len() != self.column_names.len() {
            return Err(ActLeafError::RowWidthMismatch {
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

    pub fn literal_count(&self) -> usize {
        self.rows
            .iter()
            .flatten()
            .filter(|phase| phase.is_present())
            .count()
    }

    pub fn evaluate(&self, inputs: &[bool]) -> ActLeafResult<bool> {
        if inputs.len() != self.column_count() {
            return Err(ActLeafError::InputWidthMismatch {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Act {
    Terminal(bool),
    Decision {
        column: usize,
        name: String,
        high: Box<Act>,
        low: Box<Act>,
    },
}

impl Act {
    pub fn evaluate(&self, inputs: &[bool]) -> ActLeafResult<bool> {
        match self {
            Self::Terminal(value) => Ok(*value),
            Self::Decision {
                column, high, low, ..
            } => {
                let value = inputs
                    .get(*column)
                    .copied()
                    .ok_or(ActLeafError::UnknownColumn(*column))?;
                if value {
                    high.evaluate(inputs)
                } else {
                    low.evaluate(inputs)
                }
            }
        }
    }

    pub fn node_count(&self) -> usize {
        match self {
            Self::Terminal(_) => 1,
            Self::Decision { high, low, .. } => 1 + high.node_count() + low.node_count(),
        }
    }

    pub fn index_size(&self) -> usize {
        match self {
            Self::Terminal(_) => 0,
            Self::Decision { high, low, .. } => 1 + high.index_size().max(low.index_size()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoverLiteral {
    pub column: usize,
    pub phase: LiteralPhase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActLeafError {
    MissingNativePorts { operation: &'static str },
    RowWidthMismatch { expected: usize, actual: usize },
    InputWidthMismatch { expected: usize, actual: usize },
    UnknownColumn(usize),
    InvalidLiteralValue(i32),
    BinateColumn { column: usize },
    EmptyCoverSelection,
}

impl fmt::Display for ActLeafError {
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
            Self::InvalidLiteralValue(value) => {
                write!(f, "invalid SIS literal value {value}; expected 0, 1, or 2")
            }
            Self::BinateColumn { column } => {
                write!(
                    f,
                    "unate ACT leaf construction requires a unate cover; column {column} is binate"
                )
            }
            Self::EmptyCoverSelection => write!(f, "minimum cover produced no selected columns"),
        }
    }
}

impl Error for ActLeafError {}

pub type ActLeafResult<T> = Result<T, ActLeafError>;

pub fn unate_act_sis_integration_blocked<Node>(_node: &Node) -> ActLeafResult<Act> {
    Err(ActLeafError::MissingNativePorts {
        operation: "unate_act SIS sparse-matrix and terminal-table integration",
    })
}

pub fn unate_act(cover: &UnateCover) -> ActLeafResult<Act> {
    validate_unate(cover)?;

    if has_universal_cube(cover) {
        return Ok(Act::Terminal(true));
    }
    if cover.row_count() == 0 {
        return Ok(Act::Terminal(false));
    }
    if cover.row_count() == 1 {
        return act_for_cube(cover);
    }

    let selected = minimum_column_cover(cover)?;
    if selected.is_empty() {
        return Err(ActLeafError::EmptyCoverSelection);
    }

    let subcovers = unate_split_cover(cover, &selected)?;
    let mut terms = selected
        .iter()
        .zip(subcovers.iter())
        .map(|(literal, subcover)| {
            let subact = unate_act(subcover)?;
            Ok(act_and_literal(
                *literal,
                cover.column_names[literal.column].clone(),
                subact,
            ))
        })
        .collect::<ActLeafResult<Vec<_>>>()?;

    terms.sort_by_key(Act::index_size);
    Ok(or_acts(terms))
}

pub fn has_universal_cube(cover: &UnateCover) -> bool {
    cover
        .rows()
        .iter()
        .any(|row| row.iter().all(|phase| *phase == LiteralPhase::DontCare))
}

pub fn minimum_column_cover(cover: &UnateCover) -> ActLeafResult<Vec<CoverLiteral>> {
    validate_unate(cover)?;

    let columns = cover_columns(cover);
    let required_rows = required_row_mask(cover);
    if required_rows == 0 {
        return Ok(Vec::new());
    }

    let selected_columns = if columns.len() <= 24 && cover.row_count() <= 128 {
        exact_minimum_cover(&columns, required_rows)
            .unwrap_or_else(|| greedy_cover(&columns, required_rows))
    } else {
        greedy_cover(&columns, required_rows)
    };

    Ok(selected_columns
        .into_iter()
        .map(|column| CoverLiteral {
            column: columns[column].column,
            phase: columns[column].phase,
        })
        .collect())
}

pub fn unate_split_cover(
    cover: &UnateCover,
    selected: &[CoverLiteral],
) -> ActLeafResult<Vec<UnateCover>> {
    for literal in selected {
        ensure_column(cover, literal.column)?;
    }

    let mut claimed_rows = vec![false; cover.row_count()];
    let mut subcovers = Vec::with_capacity(selected.len());
    for literal in selected {
        let mut subcover = UnateCover::new(cover.column_names.iter().cloned());
        for (row_index, row) in cover.rows.iter().enumerate() {
            if claimed_rows[row_index] || row[literal.column] == LiteralPhase::DontCare {
                continue;
            }

            let mut copied = row.clone();
            copied[literal.column] = LiteralPhase::DontCare;
            subcover.push_row(copied)?;
            claimed_rows[row_index] = true;
        }
        subcovers.push(subcover);
    }

    Ok(subcovers)
}

fn validate_unate(cover: &UnateCover) -> ActLeafResult<()> {
    for row in cover.rows() {
        if row.len() != cover.column_count() {
            return Err(ActLeafError::RowWidthMismatch {
                expected: cover.column_count(),
                actual: row.len(),
            });
        }
    }

    for column in 0..cover.column_count() {
        let mut positive = false;
        let mut negative = false;
        for row in cover.rows() {
            match row[column] {
                LiteralPhase::Positive => positive = true,
                LiteralPhase::Negative => negative = true,
                LiteralPhase::DontCare => {}
            }
        }
        if positive && negative {
            return Err(ActLeafError::BinateColumn { column });
        }
    }

    Ok(())
}

fn act_for_cube(cover: &UnateCover) -> ActLeafResult<Act> {
    let row = cover
        .rows
        .first()
        .ok_or(ActLeafError::EmptyCoverSelection)?;
    let mut literals = row
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, phase)| phase.is_present())
        .collect::<Vec<_>>();

    if let Some(index) = literals
        .iter()
        .position(|(_, phase)| *phase == LiteralPhase::Positive)
    {
        let first = literals.remove(index);
        literals.insert(0, first);
    }

    let mut act = Act::Terminal(true);
    for (column, phase) in literals.into_iter().rev() {
        act = act_and_literal(
            CoverLiteral { column, phase },
            cover.column_names[column].clone(),
            act,
        );
    }
    Ok(act)
}

fn act_and_literal(literal: CoverLiteral, name: String, rest: Act) -> Act {
    match literal.phase {
        LiteralPhase::Positive => Act::Decision {
            column: literal.column,
            name,
            high: Box::new(rest),
            low: Box::new(Act::Terminal(false)),
        },
        LiteralPhase::Negative => Act::Decision {
            column: literal.column,
            name,
            high: Box::new(Act::Terminal(false)),
            low: Box::new(rest),
        },
        LiteralPhase::DontCare => rest,
    }
}

fn or_acts(mut acts: Vec<Act>) -> Act {
    if acts.is_empty() {
        return Act::Terminal(false);
    }

    let mut act = acts.pop().expect("checked non-empty");
    while let Some(next) = acts.pop() {
        act = or_two(next, act);
    }
    act
}

fn or_two(left: Act, right: Act) -> Act {
    match left {
        Act::Terminal(false) => right,
        Act::Terminal(true) => Act::Terminal(true),
        Act::Decision {
            column,
            name,
            high,
            low,
        } => Act::Decision {
            column,
            name,
            high: Box::new(or_two(*high, right.clone())),
            low: Box::new(or_two(*low, right)),
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CoverColumn {
    column: usize,
    phase: LiteralPhase,
    rows: u128,
}

fn cover_columns(cover: &UnateCover) -> Vec<CoverColumn> {
    let mut columns = Vec::new();
    for column in 0..cover.column_count() {
        let mut rows = 0u128;
        let mut phase = LiteralPhase::DontCare;
        for (row_index, row) in cover.rows().iter().enumerate() {
            if row[column].is_present() {
                rows |= if row_index < 128 {
                    1u128 << row_index
                } else {
                    u128::MAX
                };
                phase = row[column];
            }
        }
        if rows != 0 {
            columns.push(CoverColumn {
                column,
                phase,
                rows,
            });
        }
    }
    columns
}

fn required_row_mask(cover: &UnateCover) -> u128 {
    if cover.row_count() >= 128 {
        return u128::MAX;
    }

    (0..cover.row_count()).fold(0u128, |mask, row| mask | (1u128 << row))
}

fn exact_minimum_cover(columns: &[CoverColumn], required_rows: u128) -> Option<Vec<usize>> {
    for target_size in 1..=columns.len() {
        let mut selected = Vec::with_capacity(target_size);
        if search_cover(columns, required_rows, target_size, 0, 0, &mut selected) {
            return Some(selected);
        }
    }
    None
}

fn search_cover(
    columns: &[CoverColumn],
    required_rows: u128,
    target_size: usize,
    start: usize,
    covered: u128,
    selected: &mut Vec<usize>,
) -> bool {
    if covered & required_rows == required_rows {
        return true;
    }
    if selected.len() == target_size {
        return false;
    }

    let remaining_slots = target_size - selected.len();
    if columns.len().saturating_sub(start) < remaining_slots {
        return false;
    }

    for index in start..columns.len() {
        selected.push(index);
        if search_cover(
            columns,
            required_rows,
            target_size,
            index + 1,
            covered | columns[index].rows,
            selected,
        ) {
            return true;
        }
        selected.pop();
    }

    false
}

fn greedy_cover(columns: &[CoverColumn], required_rows: u128) -> Vec<usize> {
    let mut selected = Vec::new();
    let mut covered = 0u128;
    while covered & required_rows != required_rows {
        let best = columns
            .iter()
            .enumerate()
            .filter(|(index, _)| !selected.contains(index))
            .max_by_key(|(_, column)| ((column.rows & required_rows) & !covered).count_ones());

        match best {
            Some((index, column)) if ((column.rows & required_rows) & !covered) != 0 => {
                selected.push(index);
                covered |= column.rows;
            }
            _ => break,
        }
    }
    selected.sort_unstable();
    selected
}

fn ensure_column(cover: &UnateCover, column: usize) -> ActLeafResult<()> {
    if column >= cover.column_count() {
        Err(ActLeafError::UnknownColumn(column))
    } else {
        Ok(())
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

    fn cover(rows: &[&[i32]]) -> UnateCover {
        UnateCover::from_rows(
            ["a", "b", "c", "d"],
            rows.iter().map(|row| phases(row)).collect(),
        )
        .unwrap()
    }

    fn assert_equivalent_to_cover(act: &Act, cover: &UnateCover) {
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    for d in [false, true] {
                        let inputs = [a, b, c, d];
                        assert_eq!(
                            act.evaluate(&inputs).unwrap(),
                            cover.evaluate(&inputs).unwrap(),
                            "inputs: {inputs:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn universal_cube_returns_one_terminal() {
        let matrix = cover(&[&[2, 2, 2, 2], &[1, 2, 2, 2]]);

        assert_eq!(unate_act(&matrix).unwrap(), Act::Terminal(true));
    }

    #[test]
    fn empty_cover_returns_zero_terminal() {
        let matrix = UnateCover::new(["a", "b"]);

        assert_eq!(unate_act(&matrix).unwrap(), Act::Terminal(false));
    }

    #[test]
    fn single_cube_builds_literal_conjunction() {
        let matrix = cover(&[&[1, 0, 2, 1]]);
        let act = unate_act(&matrix).unwrap();

        assert_equivalent_to_cover(&act, &matrix);
        assert_eq!(act.evaluate(&[true, false, false, true]).unwrap(), true);
        assert_eq!(act.evaluate(&[true, true, false, true]).unwrap(), false);
    }

    #[test]
    fn minimum_cover_selects_fewest_columns_that_cover_all_rows() {
        let matrix = cover(&[&[1, 2, 2, 2], &[2, 1, 2, 2], &[1, 2, 1, 2]]);

        assert_eq!(
            minimum_column_cover(&matrix).unwrap(),
            vec![
                CoverLiteral {
                    column: 0,
                    phase: LiteralPhase::Positive
                },
                CoverLiteral {
                    column: 1,
                    phase: LiteralPhase::Positive
                }
            ]
        );
    }

    #[test]
    fn split_cover_claims_each_original_row_once_and_removes_split_literal() {
        let matrix = cover(&[&[1, 1, 2, 2], &[1, 2, 1, 2], &[2, 1, 2, 1]]);
        let selected = vec![
            CoverLiteral {
                column: 0,
                phase: LiteralPhase::Positive,
            },
            CoverLiteral {
                column: 1,
                phase: LiteralPhase::Positive,
            },
        ];

        let split = unate_split_cover(&matrix, &selected).unwrap();

        assert_eq!(split.len(), 2);
        assert_eq!(
            split[0].rows(),
            &[phases(&[2, 1, 2, 2]), phases(&[2, 2, 1, 2])]
        );
        assert_eq!(split[1].rows(), &[phases(&[2, 2, 2, 1])]);
    }

    #[test]
    fn recursive_unate_act_evaluates_like_original_cover() {
        let matrix = cover(&[&[1, 1, 2, 2], &[1, 2, 1, 2], &[2, 1, 2, 1]]);
        let act = unate_act(&matrix).unwrap();

        assert_equivalent_to_cover(&act, &matrix);
    }

    #[test]
    fn negative_unate_columns_are_preserved_in_cover_literals() {
        let matrix = cover(&[&[0, 2, 2, 2], &[0, 1, 2, 2], &[2, 1, 2, 2]]);
        let selected = minimum_column_cover(&matrix).unwrap();

        assert_eq!(
            selected,
            vec![
                CoverLiteral {
                    column: 0,
                    phase: LiteralPhase::Negative
                },
                CoverLiteral {
                    column: 1,
                    phase: LiteralPhase::Positive
                }
            ]
        );

        assert_equivalent_to_cover(&unate_act(&matrix).unwrap(), &matrix);
    }

    #[test]
    fn binate_input_is_reported_as_runtime_diagnostic() {
        let matrix = cover(&[&[1, 2, 2, 2], &[0, 1, 2, 2]]);

        assert!(matches!(
            unate_act(&matrix),
            Err(ActLeafError::BinateColumn { column: 0 })
        ));
    }

    #[test]
    fn sis_integration_placeholder_returns_generic_missing_port_error() {
        assert!(matches!(
            unate_act_sis_integration_blocked(&()),
            Err(ActLeafError::MissingNativePorts { .. })
        ));
    }
}
