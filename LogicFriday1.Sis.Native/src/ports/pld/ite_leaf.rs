//! Native Rust model for the unate ITE leaf construction.
//!
//! The legacy routine receives a sparse SIS cover matrix for a unate leaf,
//! handles universal and one-cube fast paths, solves a small column-cover
//! problem, recursively builds sub-leaves, and joins them as an OR of
//! literal-guarded sub-ITEs. This module keeps that behavior on owned data.
//! Direct SIS `node_t`, sparse-matrix allocation, network fanin lookup, and
//! global terminal-table integration are represented by explicit dependency
//! errors instead of per-file C ABI symbols.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Negative,
    Positive,
    DontCare,
}

impl LiteralPhase {
    pub fn from_sis_literal(value: i32) -> IteLeafResult<Self> {
        match value {
            0 => Ok(Self::Negative),
            1 => Ok(Self::Positive),
            2 => Ok(Self::DontCare),
            _ => Err(IteLeafError::InvalidLiteralValue(value)),
        }
    }

    pub fn matches(self, value: bool) -> bool {
        match self {
            Self::Negative => !value,
            Self::Positive => value,
            Self::DontCare => true,
        }
    }

    pub fn is_present(self) -> bool {
        self != Self::DontCare
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverMatrix {
    rows: Vec<Vec<LiteralPhase>>,
    column_names: Vec<String>,
}

impl CoverMatrix {
    pub fn new(column_names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            rows: Vec::new(),
            column_names: column_names.into_iter().map(Into::into).collect(),
        }
    }

    pub fn from_rows(
        column_names: impl IntoIterator<Item = impl Into<String>>,
        rows: Vec<Vec<LiteralPhase>>,
    ) -> IteLeafResult<Self> {
        let mut matrix = Self::new(column_names);
        for row in rows {
            matrix.push_row(row)?;
        }
        Ok(matrix)
    }

    pub fn push_row(&mut self, row: Vec<LiteralPhase>) -> IteLeafResult<()> {
        if row.len() != self.column_names.len() {
            return Err(IteLeafError::RowWidthMismatch {
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

    pub fn evaluate(&self, inputs: &[bool]) -> IteLeafResult<bool> {
        if inputs.len() != self.column_count() {
            return Err(IteLeafError::InputWidthMismatch {
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
pub enum Ite {
    Terminal(bool),
    Literal {
        column: usize,
        name: String,
        phase: LiteralPhase,
    },
    Shannon {
        condition: Box<Ite>,
        then_branch: Box<Ite>,
        else_branch: Box<Ite>,
    },
}

impl Ite {
    pub fn evaluate(&self, inputs: &[bool]) -> IteLeafResult<bool> {
        match self {
            Self::Terminal(value) => Ok(*value),
            Self::Literal { column, phase, .. } => {
                let value = inputs
                    .get(*column)
                    .copied()
                    .ok_or(IteLeafError::UnknownColumn(*column))?;
                Ok(phase.matches(value))
            }
            Self::Shannon {
                condition,
                then_branch,
                else_branch,
            } => {
                if condition.evaluate(inputs)? {
                    then_branch.evaluate(inputs)
                } else {
                    else_branch.evaluate(inputs)
                }
            }
        }
    }

    pub fn index_size(&self) -> usize {
        match self {
            Self::Terminal(_) => 0,
            Self::Literal { .. } => 1,
            Self::Shannon {
                condition,
                then_branch,
                else_branch,
            } => condition.index_size() + then_branch.index_size() + else_branch.index_size(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoverChoice {
    pub original_column: usize,
    pub phase: LiteralPhase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteLeafError {
    RowWidthMismatch { expected: usize, actual: usize },
    InputWidthMismatch { expected: usize, actual: usize },
    UnknownColumn(usize),
    EmptyColumnName { column: usize },
    InvalidLiteralValue(i32),
    NonUnateColumn { column: usize },
    UncoverableRows,
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for IteLeafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::NonUnateColumn { column } => {
                write!(f, "cover column {column} contains both literal phases")
            }
            Self::UncoverableRows => write!(f, "cover rows cannot be covered by literal columns"),
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS native dependencies"
            ),
        }
    }
}

impl Error for IteLeafError {}

pub type IteLeafResult<T> = Result<T, IteLeafError>;

pub fn unate_ite(matrix: &CoverMatrix) -> IteLeafResult<Ite> {
    if has_universal_cube(matrix) {
        return Ok(Ite::Terminal(true));
    }
    if matrix.row_count() == 0 {
        return Ok(Ite::Terminal(false));
    }
    if matrix.row_count() == 1 {
        return ite_for_cube(matrix);
    }
    if let Some(ite) = ite_check_for_single_literal_cubes(matrix)? {
        return Ok(ite);
    }

    let choices = minimum_unate_cover(matrix)?;
    let sub_matrices = unate_split_f(matrix, &choices)?;
    let mut terms = Vec::with_capacity(choices.len());

    for (choice, sub_matrix) in choices.iter().zip(&sub_matrices) {
        let sub_ite = unate_ite(sub_matrix)?;
        terms.push(ite_and_literal(choice, sub_ite, matrix)?);
    }

    if terms.len() != choices.len() {
        return Err(IteLeafError::UncoverableRows);
    }

    terms.sort_by_key(Ite::index_size);
    Ok(or_ite_vec(terms).unwrap_or(Ite::Terminal(false)))
}

pub fn unate_ite_blocked<Matrix, Node>(_matrix: &Matrix, _node: &Node) -> IteLeafResult<Ite> {
    Err(missing_native_ports(
        "unate_ite SIS sparse-matrix/node integration",
    ))
}

pub fn has_universal_cube(matrix: &CoverMatrix) -> bool {
    matrix
        .rows()
        .iter()
        .any(|row| row.iter().all(|phase| *phase == LiteralPhase::DontCare))
}

pub fn minimum_unate_cover(matrix: &CoverMatrix) -> IteLeafResult<Vec<CoverChoice>> {
    let column_data = active_column_data(matrix)?;
    if column_data.is_empty() {
        return if matrix.row_count() == 0 || has_universal_cube(matrix) {
            Ok(Vec::new())
        } else {
            Err(IteLeafError::UncoverableRows)
        };
    }

    let has_negative = column_data
        .iter()
        .any(|column| column.phase == LiteralPhase::Negative);
    let positive_weight = column_data.len() + 1;
    let columns = column_data
        .iter()
        .map(|column| CoverColumn {
            choice: CoverChoice {
                original_column: column.original_column,
                phase: column.phase,
            },
            rows: rows_covered_by_column(matrix, column.original_column),
            weight: if has_negative && column.phase == LiteralPhase::Negative {
                column_data.len()
            } else if has_negative {
                positive_weight
            } else {
                1
            },
        })
        .collect::<Vec<_>>();

    let all_rows = if matrix.row_count() >= usize::BITS as usize {
        return minimum_unate_cover_greedy(&columns, matrix.row_count());
    } else {
        (1usize << matrix.row_count()) - 1
    };

    let mut best = None;
    search_cover(&columns, all_rows, 0, Vec::new(), &mut best);
    best.map(|solution| {
        solution
            .indices
            .into_iter()
            .map(|index| columns[index].choice)
            .collect()
    })
    .ok_or(IteLeafError::UncoverableRows)
}

pub fn unate_split_f(
    matrix: &CoverMatrix,
    choices: &[CoverChoice],
) -> IteLeafResult<Vec<CoverMatrix>> {
    for choice in choices {
        ensure_column(matrix, choice.original_column)?;
    }

    let mut assigned = vec![false; matrix.row_count()];
    let mut matrices = Vec::with_capacity(choices.len());
    for choice in choices {
        let mut sub_matrix = CoverMatrix::new(matrix.column_names().iter().cloned());
        for (row_index, row) in matrix.rows().iter().enumerate() {
            if row[choice.original_column].is_present() && !assigned[row_index] {
                let mut copied = row.clone();
                copied[choice.original_column] = LiteralPhase::DontCare;
                sub_matrix.push_row(copied)?;
                assigned[row_index] = true;
            }
        }
        matrices.push(sub_matrix);
    }
    Ok(matrices)
}

fn ite_for_cube(matrix: &CoverMatrix) -> IteLeafResult<Ite> {
    let row = matrix.rows().first().ok_or(IteLeafError::UncoverableRows)?;
    let mut terms = Vec::new();
    for (column, phase) in row.iter().copied().enumerate() {
        if phase.is_present() {
            terms.push(literal_ite(matrix, column, phase)?);
        }
    }
    Ok(and_ite_vec(terms).unwrap_or(Ite::Terminal(true)))
}

fn ite_check_for_single_literal_cubes(matrix: &CoverMatrix) -> IteLeafResult<Option<Ite>> {
    let mut seen_columns = vec![false; matrix.column_count()];
    let mut terms = Vec::new();

    for row in matrix.rows() {
        let mut present = row
            .iter()
            .enumerate()
            .filter(|(_, phase)| phase.is_present());
        let Some((column, phase)) = present.next() else {
            continue;
        };
        if present.next().is_some() || seen_columns[column] {
            return Ok(None);
        }
        seen_columns[column] = true;
        terms.push(literal_ite(matrix, column, *phase)?);
    }

    Ok(Some(or_ite_vec(terms).unwrap_or(Ite::Terminal(false))))
}

fn active_column_data(matrix: &CoverMatrix) -> IteLeafResult<Vec<ActiveColumn>> {
    let mut columns = Vec::new();
    for column in 0..matrix.column_count() {
        let mut phase = None;
        for row in matrix.rows() {
            match row[column] {
                LiteralPhase::DontCare => {}
                current => {
                    if phase.is_some_and(|existing| existing != current) {
                        return Err(IteLeafError::NonUnateColumn { column });
                    }
                    phase = Some(current);
                }
            }
        }
        if let Some(phase) = phase {
            columns.push(ActiveColumn {
                original_column: column,
                phase,
            });
        }
    }
    Ok(columns)
}

fn rows_covered_by_column(matrix: &CoverMatrix, column: usize) -> Vec<usize> {
    matrix
        .rows()
        .iter()
        .enumerate()
        .filter(|(_, row)| row[column].is_present())
        .map(|(row, _)| row)
        .collect()
}

fn search_cover(
    columns: &[CoverColumn],
    remaining_rows: usize,
    start_index: usize,
    selected: Vec<usize>,
    best: &mut Option<CoverSolution>,
) {
    if remaining_rows == 0 {
        let candidate = CoverSolution::new(selected);
        if best
            .as_ref()
            .is_none_or(|current| candidate.is_better_than(current, columns))
        {
            *best = Some(candidate);
        }
        return;
    }
    if start_index >= columns.len() {
        return;
    }
    if best
        .as_ref()
        .is_some_and(|current| selected.len() >= current.indices.len())
    {
        return;
    }

    let row = remaining_rows.trailing_zeros() as usize;
    let mut candidate_indices = (start_index..columns.len())
        .filter(|index| columns[*index].covers_row(row))
        .collect::<Vec<_>>();
    candidate_indices.sort_by_key(|index| (columns[*index].weight, *index));

    for index in candidate_indices {
        let mut next_selected = selected.clone();
        next_selected.push(index);
        search_cover(
            columns,
            remaining_rows & !columns[index].row_mask(),
            index + 1,
            next_selected,
            best,
        );
    }
}

fn minimum_unate_cover_greedy(
    columns: &[CoverColumn],
    row_count: usize,
) -> IteLeafResult<Vec<CoverChoice>> {
    let mut remaining = vec![true; row_count];
    let mut choices = Vec::new();
    while remaining.iter().any(|row| *row) {
        let best = columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| {
                let covered = (0..row_count)
                    .filter(|row| remaining[*row] && column.covers_row(*row))
                    .count();
                (covered > 0).then_some((index, covered, column.weight))
            })
            .max_by_key(|(_, covered, weight)| (*covered, std::cmp::Reverse(*weight)))
            .map(|(index, _, _)| index)
            .ok_or(IteLeafError::UncoverableRows)?;

        for row in 0..row_count {
            if columns[best].covers_row(row) {
                remaining[row] = false;
            }
        }
        choices.push(columns[best].choice);
    }
    Ok(choices)
}

fn ite_and_literal(choice: &CoverChoice, sub_ite: Ite, matrix: &CoverMatrix) -> IteLeafResult<Ite> {
    let literal = literal_ite(matrix, choice.original_column, choice.phase)?;
    Ok(and_ite(literal, sub_ite))
}

fn literal_ite(matrix: &CoverMatrix, column: usize, phase: LiteralPhase) -> IteLeafResult<Ite> {
    let name = matrix
        .column_names()
        .get(column)
        .map(String::as_str)
        .filter(|name| !name.is_empty())
        .ok_or(IteLeafError::EmptyColumnName { column })?;
    Ok(Ite::Literal {
        column,
        name: name.to_owned(),
        phase,
    })
}

fn and_ite_vec(terms: Vec<Ite>) -> Option<Ite> {
    terms.into_iter().reduce(and_ite)
}

fn and_ite(left: Ite, right: Ite) -> Ite {
    my_shannon_ite(left, right, Ite::Terminal(false))
}

fn or_ite_vec(terms: Vec<Ite>) -> Option<Ite> {
    terms.into_iter().reduce(or_ite)
}

fn or_ite(left: Ite, right: Ite) -> Ite {
    my_shannon_ite(left, Ite::Terminal(true), right)
}

fn my_shannon_ite(condition: Ite, then_branch: Ite, else_branch: Ite) -> Ite {
    Ite::Shannon {
        condition: Box::new(condition),
        then_branch: Box::new(then_branch),
        else_branch: Box::new(else_branch),
    }
}

fn ensure_column(matrix: &CoverMatrix, column: usize) -> IteLeafResult<()> {
    if column >= matrix.column_count() {
        Err(IteLeafError::UnknownColumn(column))
    } else {
        Ok(())
    }
}

fn missing_native_ports(operation: &'static str) -> IteLeafError {
    IteLeafError::MissingNativePorts { operation }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActiveColumn {
    original_column: usize,
    phase: LiteralPhase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CoverColumn {
    choice: CoverChoice,
    rows: Vec<usize>,
    weight: usize,
}

impl CoverColumn {
    fn covers_row(&self, row: usize) -> bool {
        self.rows.contains(&row)
    }

    fn row_mask(&self) -> usize {
        self.rows
            .iter()
            .fold(0usize, |mask, row| mask | (1usize << row))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CoverSolution {
    indices: Vec<usize>,
}

impl CoverSolution {
    fn new(indices: Vec<usize>) -> Self {
        Self { indices }
    }

    fn weight(&self, columns: &[CoverColumn]) -> usize {
        self.indices
            .iter()
            .map(|index| columns[*index].weight)
            .sum()
    }

    fn is_better_than(&self, other: &Self, columns: &[CoverColumn]) -> bool {
        self.indices.len() < other.indices.len()
            || (self.indices.len() == other.indices.len()
                && self.weight(columns) < other.weight(columns))
            || (self.indices.len() == other.indices.len()
                && self.weight(columns) == other.weight(columns)
                && self.indices < other.indices)
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

    fn assert_equivalent_to_cover(cover: &CoverMatrix, ite: &Ite) {
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let inputs = [a, b, c];
                    assert_eq!(
                        ite.evaluate(&inputs).unwrap(),
                        cover.evaluate(&inputs).unwrap(),
                        "inputs: {inputs:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn universal_cube_returns_one_terminal() {
        assert_eq!(
            unate_ite(&matrix(&[&[1, 2, 2], &[2, 2, 2]])).unwrap(),
            Ite::Terminal(true)
        );
    }

    #[test]
    fn single_cube_constructs_literal_and_chain() {
        let cover = matrix(&[&[1, 0, 2]]);
        let ite = unate_ite(&cover).unwrap();

        assert_eq!(ite.evaluate(&[true, false, false]).unwrap(), true);
        assert_eq!(ite.evaluate(&[true, true, false]).unwrap(), false);
        assert_eq!(ite.evaluate(&[false, false, false]).unwrap(), false);
        assert_equivalent_to_cover(&cover, &ite);
    }

    #[test]
    fn single_literal_cubes_construct_or_chain() {
        let cover = matrix(&[&[1, 2, 2], &[2, 0, 2], &[2, 2, 1]]);
        let ite = unate_ite(&cover).unwrap();

        assert_eq!(ite.evaluate(&[true, true, false]).unwrap(), true);
        assert_eq!(ite.evaluate(&[false, false, false]).unwrap(), true);
        assert_eq!(ite.evaluate(&[false, true, true]).unwrap(), true);
        assert_eq!(ite.evaluate(&[false, true, false]).unwrap(), false);
        assert_equivalent_to_cover(&cover, &ite);
    }

    #[test]
    fn minimum_cover_omits_all_dont_care_columns_and_prefers_negative_tie() {
        let cover = matrix(&[&[0, 1, 2], &[0, 2, 2], &[2, 1, 2]]);
        let choices = minimum_unate_cover(&cover).unwrap();

        assert_eq!(
            choices,
            vec![
                CoverChoice {
                    original_column: 0,
                    phase: LiteralPhase::Negative
                },
                CoverChoice {
                    original_column: 1,
                    phase: LiteralPhase::Positive
                }
            ]
        );
    }

    #[test]
    fn split_assigns_each_row_to_first_covering_choice_and_clears_column() {
        let cover = matrix(&[&[0, 1, 2], &[0, 2, 2], &[2, 1, 2]]);
        let choices = vec![
            CoverChoice {
                original_column: 0,
                phase: LiteralPhase::Negative,
            },
            CoverChoice {
                original_column: 1,
                phase: LiteralPhase::Positive,
            },
        ];
        let split = unate_split_f(&cover, &choices).unwrap();

        assert_eq!(split[0].rows(), &[phases(&[2, 1, 2]), phases(&[2, 2, 2])]);
        assert_eq!(split[1].rows(), &[phases(&[2, 2, 2])]);
    }

    #[test]
    fn recursive_unate_ite_matches_cover_function() {
        let cover = matrix(&[&[0, 1, 2], &[0, 2, 1], &[2, 1, 1]]);
        let ite = unate_ite(&cover).unwrap();

        assert_equivalent_to_cover(&cover, &ite);
    }

    #[test]
    fn binate_input_is_rejected_for_unate_leaf_covering() {
        let cover = matrix(&[&[0, 2, 2], &[1, 2, 2]]);

        assert!(matches!(
            minimum_unate_cover(&cover),
            Err(IteLeafError::NonUnateColumn { column: 0 })
        ));
    }

    #[test]
    fn blocked_sis_integration_returns_runtime_diagnostic() {
        let err = unate_ite_blocked(&(), &()).unwrap_err();

        assert!(matches!(err, IteLeafError::MissingNativePorts { .. }));
        assert!(
            err.to_string()
                .contains("blocked by unported SIS native dependencies")
        );
    }
}
