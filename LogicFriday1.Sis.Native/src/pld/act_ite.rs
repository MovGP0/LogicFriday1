//! Owned-data model for the ACT ITE construction helpers.
//!
//! This module ports the cover-to-ITE behavior from the SIS PLD ACT path into
//! Rust data structures. Direct SIS `node_t`, `network_t`, sparse-matrix, and
//! global-terminal integration is represented by explicit runtime diagnostics
//! until those dependencies have native Rust integration points.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Negative,
    Positive,
    DontCare,
}

impl LiteralPhase {
    pub fn from_sis_value(value: i32) -> ActIteResult<Self> {
        match value {
            0 => Ok(Self::Negative),
            1 => Ok(Self::Positive),
            2 => Ok(Self::DontCare),
            _ => Err(ActIteError::InvalidLiteralPhase(value)),
        }
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
pub struct CoverElement {
    pub column: usize,
    pub phase: LiteralPhase,
}

impl CoverElement {
    pub fn new(column: usize, phase: LiteralPhase) -> Self {
        Self { column, phase }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CoverRow {
    elements: Vec<CoverElement>,
}

impl CoverRow {
    pub fn new(elements: impl IntoIterator<Item = CoverElement>) -> Self {
        Self {
            elements: elements.into_iter().collect(),
        }
    }

    pub fn elements(&self) -> &[CoverElement] {
        &self.elements
    }

    pub fn active_elements(&self) -> impl Iterator<Item = &CoverElement> {
        self.elements
            .iter()
            .filter(|element| element.phase != LiteralPhase::DontCare)
    }

    fn phase_at(&self, column: usize) -> LiteralPhase {
        self.elements
            .iter()
            .find(|element| element.column == column)
            .map(|element| element.phase)
            .unwrap_or(LiteralPhase::DontCare)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverMatrix {
    columns: Vec<String>,
    rows: Vec<CoverRow>,
}

impl CoverMatrix {
    pub fn new(columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            columns: columns.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
        }
    }

    pub fn from_rows(
        columns: impl IntoIterator<Item = impl Into<String>>,
        rows: impl IntoIterator<Item = CoverRow>,
    ) -> ActIteResult<Self> {
        let mut matrix = Self::new(columns);
        for row in rows {
            matrix.push_row(row)?;
        }
        Ok(matrix)
    }

    pub fn push_row(&mut self, row: CoverRow) -> ActIteResult<()> {
        let mut seen = HashSet::new();
        for element in row.elements() {
            if element.column >= self.columns.len() {
                return Err(ActIteError::UnknownColumn(element.column));
            }
            if !seen.insert(element.column) {
                return Err(ActIteError::DuplicateColumn(element.column));
            }
        }
        self.rows.push(row);
        Ok(())
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    pub fn rows(&self) -> &[CoverRow] {
        &self.rows
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_name(&self, column: usize) -> ActIteResult<&str> {
        self.columns
            .get(column)
            .map(String::as_str)
            .ok_or(ActIteError::UnknownColumn(column))
    }

    pub fn evaluate(&self, inputs: &[bool]) -> ActIteResult<bool> {
        if inputs.len() != self.columns.len() {
            return Err(ActIteError::InputWidthMismatch {
                expected: self.columns.len(),
                actual: inputs.len(),
            });
        }

        Ok(self.rows.iter().any(|row| {
            (0..self.columns.len()).all(|column| row.phase_at(column).matches(inputs[column]))
        }))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BooleanNode {
    Constant(bool),
    Literal { column: usize, phase: LiteralPhase },
    And(Vec<BooleanNode>),
    Or(Vec<BooleanNode>),
}

impl BooleanNode {
    pub fn evaluate(&self, inputs: &[bool]) -> ActIteResult<bool> {
        match self {
            Self::Constant(value) => Ok(*value),
            Self::Literal { column, phase } => inputs
                .get(*column)
                .map(|value| phase.matches(*value))
                .ok_or(ActIteError::UnknownColumn(*column)),
            Self::And(nodes) => {
                for node in nodes {
                    if !node.evaluate(inputs)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Self::Or(nodes) => {
                for node in nodes {
                    if node.evaluate(inputs)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    pub fn fanin_phases(&self) -> Vec<(usize, LiteralPhase)> {
        let mut phases = Vec::new();
        self.collect_fanin_phases(&mut phases);
        phases
    }

    fn collect_fanin_phases(&self, phases: &mut Vec<(usize, LiteralPhase)>) {
        match self {
            Self::Literal { column, phase } if *phase != LiteralPhase::DontCare => {
                phases.push((*column, *phase));
            }
            Self::And(nodes) | Self::Or(nodes) => {
                for node in nodes {
                    node.collect_fanin_phases(phases);
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ite {
    Terminal(bool),
    Literal {
        column: usize,
        name: String,
    },
    Shannon {
        condition: Box<Ite>,
        then_branch: Box<Ite>,
        else_branch: Box<Ite>,
    },
}

impl Ite {
    pub fn evaluate(&self, inputs: &[bool]) -> ActIteResult<bool> {
        match self {
            Self::Terminal(value) => Ok(*value),
            Self::Literal { column, .. } => inputs
                .get(*column)
                .copied()
                .ok_or(ActIteError::UnknownColumn(*column)),
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

    pub fn terminal_count(&self, value: bool) -> usize {
        match self {
            Self::Terminal(terminal_value) => usize::from(*terminal_value == value),
            Self::Literal { .. } => 0,
            Self::Shannon {
                condition,
                then_branch,
                else_branch,
            } => {
                condition.terminal_count(value)
                    + then_branch.terminal_count(value)
                    + else_branch.terminal_count(value)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActIteError {
    InvalidLiteralPhase(i32),
    InputWidthMismatch { expected: usize, actual: usize },
    UnknownColumn(usize),
    DuplicateColumn(usize),
    EmptyCubeCover { rows: usize },
    EmptyIteVector,
    BinateOrUnknownLiteral { column: usize },
    DuplicateSingleLiteral(String),
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for ActIteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLiteralPhase(value) => {
                write!(f, "invalid literal phase {value}; expected 0, 1, or 2")
            }
            Self::InputWidthMismatch { expected, actual } => {
                write!(f, "input vector has width {actual}, expected {expected}")
            }
            Self::UnknownColumn(column) => write!(f, "unknown cover column {column}"),
            Self::DuplicateColumn(column) => write!(f, "duplicate cover column {column}"),
            Self::EmptyCubeCover { rows } => {
                write!(f, "expected exactly one cube row, got {rows}")
            }
            Self::EmptyIteVector => write!(f, "expected at least one ITE"),
            Self::BinateOrUnknownLiteral { column } => {
                write!(f, "column {column} has a binate or unknown literal phase")
            }
            Self::DuplicateSingleLiteral(name) => {
                write!(f, "single-literal cover repeats variable {name}")
            }
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
        }
    }
}

impl Error for ActIteError {}

pub type ActIteResult<T> = Result<T, ActIteError>;

pub fn sis_bound_operation_unavailable(operation: &'static str) -> ActIteResult<()> {
    Err(ActIteError::MissingNativePorts { operation })
}

pub fn ite_for_cube_f(matrix: &CoverMatrix) -> ActIteResult<Ite> {
    let row = match matrix.rows() {
        [row] => row,
        rows => {
            return Err(ActIteError::EmptyCubeCover { rows: rows.len() });
        }
    };

    let cube_node = act_make_node_from_row(row, matrix)?;
    let mut vertex = None;
    for (column, phase) in single_cube_order(&cube_node).into_iter().rev() {
        let element = CoverElement::new(column, phase);
        vertex = Some(my_ite_and(
            &element,
            vertex,
            matrix.column_name(column)?.to_owned(),
        )?);
    }

    Ok(vertex.unwrap_or(Ite::Terminal(true)))
}

pub fn my_ite_and(
    element: &CoverElement,
    vertex: Option<Ite>,
    name: impl Into<String>,
) -> ActIteResult<Ite> {
    let condition = ite_literal(element.column, name);
    match (element.phase, vertex) {
        (LiteralPhase::Negative, None) => Ok(my_shannon_ite(
            condition,
            Ite::Terminal(false),
            Ite::Terminal(true),
        )),
        (LiteralPhase::Positive, None) => Ok(my_shannon_ite(
            condition,
            Ite::Terminal(true),
            Ite::Terminal(false),
        )),
        (LiteralPhase::Negative, Some(vertex)) => {
            Ok(my_shannon_ite(condition, Ite::Terminal(false), vertex))
        }
        (LiteralPhase::Positive, Some(vertex)) => {
            Ok(my_shannon_ite(condition, vertex, Ite::Terminal(false)))
        }
        (LiteralPhase::DontCare, _) => Err(ActIteError::BinateOrUnknownLiteral {
            column: element.column,
        }),
    }
}

pub fn my_or_ite_f(
    sub_ites: impl IntoIterator<Item = Ite>,
    cover: &CoverRow,
    matrices: &[CoverMatrix],
) -> ActIteResult<Ite> {
    let mut ites = sub_ites.into_iter().collect::<Vec<_>>();
    if cover.elements().len() != ites.len() || matrices.len() != ites.len() {
        return Err(ActIteError::InputWidthMismatch {
            expected: cover.elements().len(),
            actual: ites.len().max(matrices.len()),
        });
    }

    for (index, element) in cover.elements().iter().enumerate() {
        let name = matrices[index].column_name(element.column)?.to_owned();
        let previous = std::mem::replace(&mut ites[index], Ite::Terminal(false));
        ites[index] = my_ite_and(element, Some(previous), name)?;
    }

    ites.sort_by_key(Ite::index_size);
    ite_or_itevec(ites)
}

pub fn ite_or_itevec(ites: impl IntoIterator<Item = Ite>) -> ActIteResult<Ite> {
    let mut ites = ites.into_iter();
    let mut root = ites.next().ok_or(ActIteError::EmptyIteVector)?;
    for down in ites {
        my_ite_or(&mut root, &down);
    }
    Ok(root)
}

pub fn my_ite_or(up_vertex: &mut Ite, down_vertex: &Ite) {
    match up_vertex {
        Ite::Shannon {
            condition,
            then_branch,
            else_branch,
        } => {
            replace_zero_child_or_recurse(condition, down_vertex);
            replace_zero_child_or_recurse(then_branch, down_vertex);
            replace_zero_child_or_recurse(else_branch, down_vertex);
        }
        Ite::Terminal(_) | Ite::Literal { .. } => {}
    }
}

pub fn ite_literal(column: usize, name: impl Into<String>) -> Ite {
    Ite::Literal {
        column,
        name: name.into(),
    }
}

pub fn my_shannon_ite(condition: Ite, then_branch: Ite, else_branch: Ite) -> Ite {
    Ite::Shannon {
        condition: Box::new(condition),
        then_branch: Box::new(then_branch),
        else_branch: Box::new(else_branch),
    }
}

pub fn act_make_node_from_row(row: &CoverRow, matrix: &CoverMatrix) -> ActIteResult<BooleanNode> {
    let mut terms = Vec::new();
    for element in row.active_elements() {
        matrix.column_name(element.column)?;
        terms.push(BooleanNode::Literal {
            column: element.column,
            phase: element.phase,
        });
    }
    Ok(BooleanNode::And(terms))
}

pub fn act_make_name_to_element_table(
    row: &CoverRow,
    matrix: &CoverMatrix,
) -> ActIteResult<HashMap<String, CoverElement>> {
    let mut table = HashMap::new();
    for element in row.elements() {
        let name = matrix.column_name(element.column)?.to_owned();
        if table.insert(name.clone(), element.clone()).is_some() {
            return Err(ActIteError::DuplicateSingleLiteral(name));
        }
    }
    Ok(table)
}

pub fn ite_check_for_single_literal_cubes(matrix: &CoverMatrix) -> ActIteResult<Option<Ite>> {
    let mut literals = Vec::new();
    let mut seen_names = HashSet::new();

    for row in matrix.rows() {
        let active = row.active_elements().collect::<Vec<_>>();
        if active.len() > 1 {
            return Ok(None);
        }
        let Some(element) = active.first() else {
            continue;
        };
        let name = matrix.column_name(element.column)?.to_owned();
        if !seen_names.insert(name.clone()) {
            return Err(ActIteError::DuplicateSingleLiteral(name));
        }
        literals.push((element.column, element.phase));
    }

    if literals.is_empty() {
        return Ok(Some(Ite::Terminal(false)));
    }

    let node = BooleanNode::Or(
        literals
            .iter()
            .map(|(column, phase)| BooleanNode::Literal {
                column: *column,
                phase: *phase,
            })
            .collect(),
    );
    let mut ites = Vec::new();
    for (column, phase) in or_literal_order(&node) {
        let condition = ite_literal(column, matrix.column_name(column)?.to_owned());
        ites.push(match phase {
            LiteralPhase::Positive => {
                my_shannon_ite(condition, Ite::Terminal(true), Ite::Terminal(false))
            }
            LiteralPhase::Negative => {
                my_shannon_ite(condition, Ite::Terminal(false), Ite::Terminal(true))
            }
            LiteralPhase::DontCare => {
                return Err(ActIteError::BinateOrUnknownLiteral { column });
            }
        });
    }

    ite_or_itevec(ites).map(Some)
}

pub fn act_make_node_from_matrix(matrix: &CoverMatrix) -> ActIteResult<BooleanNode> {
    let mut terms = Vec::new();
    for row in matrix.rows() {
        terms.push(act_make_node_from_row(row, matrix)?);
    }
    Ok(BooleanNode::Or(terms))
}

pub fn single_cube_order(node: &BooleanNode) -> Vec<(usize, LiteralPhase)> {
    staged_unate_order(
        node.fanin_phases(),
        &[
            LiteralPhase::Positive,
            LiteralPhase::Positive,
            LiteralPhase::Negative,
            LiteralPhase::Negative,
        ],
    )
}

pub fn or_literal_order(node: &BooleanNode) -> Vec<(usize, LiteralPhase)> {
    staged_unate_order(
        node.fanin_phases(),
        &[
            LiteralPhase::Positive,
            LiteralPhase::Negative,
            LiteralPhase::Positive,
            LiteralPhase::Positive,
        ],
    )
}

pub fn ite_for_cube_f_blocked<Matrix, Node>(_matrix: &Matrix, _node: &Node) -> ActIteResult<Ite> {
    Err(ActIteError::MissingNativePorts {
        operation: "ite_for_cube_F SIS sparse-matrix/node integration",
    })
}

pub fn my_or_ite_f_blocked<Array, Cover, Matrix, Network>(
    _array_b: &Array,
    _cover: &Cover,
    _array: &Matrix,
    _network: &Network,
) -> ActIteResult<Ite> {
    Err(ActIteError::MissingNativePorts {
        operation: "my_or_ite_F SIS array/network integration",
    })
}

fn replace_zero_child_or_recurse(child: &mut Box<Ite>, down_vertex: &Ite) {
    if matches!(child.as_ref(), Ite::Terminal(false)) {
        *child = Box::new(down_vertex.clone());
    } else {
        my_ite_or(child, down_vertex);
    }
}

fn staged_unate_order(
    fanins: Vec<(usize, LiteralPhase)>,
    phase_preference: &[LiteralPhase; 4],
) -> Vec<(usize, LiteralPhase)> {
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for fanin in fanins.into_iter().rev() {
        match fanin.1 {
            LiteralPhase::Positive => positive.push(fanin),
            LiteralPhase::Negative => negative.push(fanin),
            LiteralPhase::DontCare => {}
        }
    }

    let mut order_rev = Vec::new();
    let mut positive_index = 0;
    let mut negative_index = 0;
    let mut stage = 0;
    let total = positive.len() + negative.len();

    for _ in 0..total {
        let preferred = phase_preference[stage];
        let next = match preferred {
            LiteralPhase::Positive => take_next(
                &positive,
                &mut positive_index,
                &negative,
                &mut negative_index,
            ),
            LiteralPhase::Negative => take_next(
                &negative,
                &mut negative_index,
                &positive,
                &mut positive_index,
            ),
            LiteralPhase::DontCare => None,
        };
        if let Some(fanin) = next {
            order_rev.push(fanin);
        }
        stage = if stage % 3 == 0 { 1 } else { stage + 1 };
    }

    order_rev.into_iter().rev().collect()
}

fn take_next(
    primary: &[(usize, LiteralPhase)],
    primary_index: &mut usize,
    fallback: &[(usize, LiteralPhase)],
    fallback_index: &mut usize,
) -> Option<(usize, LiteralPhase)> {
    if let Some(value) = primary.get(*primary_index).copied() {
        *primary_index += 1;
        Some(value)
    } else if let Some(value) = fallback.get(*fallback_index).copied() {
        *fallback_index += 1;
        Some(value)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(values: &[i32]) -> CoverRow {
        CoverRow::new(values.iter().enumerate().map(|(column, value)| {
            CoverElement::new(column, LiteralPhase::from_sis_value(*value).unwrap())
        }))
    }

    fn matrix(rows: &[&[i32]]) -> CoverMatrix {
        CoverMatrix::from_rows(["a", "b", "c"], rows.iter().map(|values| row(values))).unwrap()
    }

    fn assert_matches_matrix(ite: &Ite, cover: &CoverMatrix) {
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let inputs = [a, b, c];
                    assert_eq!(
                        ite.evaluate(&inputs).unwrap(),
                        cover.evaluate(&inputs).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn cube_ite_matches_single_cube_cover() {
        let cover = matrix(&[&[1, 0, 2]]);
        let ite = ite_for_cube_f(&cover).unwrap();

        assert_matches_matrix(&ite, &cover);
        assert_eq!(ite.index_size(), 2);
    }

    #[test]
    fn my_ite_and_builds_positive_and_negative_literals() {
        let pos = my_ite_and(&CoverElement::new(0, LiteralPhase::Positive), None, "a").unwrap();
        let neg = my_ite_and(&CoverElement::new(0, LiteralPhase::Negative), None, "a").unwrap();

        assert!(pos.evaluate(&[true]).unwrap());
        assert!(!pos.evaluate(&[false]).unwrap());
        assert!(!neg.evaluate(&[true]).unwrap());
        assert!(neg.evaluate(&[false]).unwrap());
    }

    #[test]
    fn ite_or_itevec_replaces_zero_terminals_to_form_or_chain() {
        let a = my_ite_and(&CoverElement::new(0, LiteralPhase::Positive), None, "a").unwrap();
        let b = my_ite_and(&CoverElement::new(1, LiteralPhase::Positive), None, "b").unwrap();

        let ite = ite_or_itevec([a, b]).unwrap();

        assert!(!ite.evaluate(&[false, false]).unwrap());
        assert!(ite.evaluate(&[true, false]).unwrap());
        assert!(ite.evaluate(&[false, true]).unwrap());
    }

    #[test]
    fn single_literal_cube_detection_returns_none_for_multi_literal_row() {
        assert_eq!(
            ite_check_for_single_literal_cubes(&matrix(&[&[1, 0, 2]])).unwrap(),
            None
        );
    }

    #[test]
    fn single_literal_cube_detection_builds_or_ite() {
        let cover = matrix(&[&[1, 2, 2], &[2, 0, 2], &[2, 2, 1]]);
        let ite = ite_check_for_single_literal_cubes(&cover).unwrap().unwrap();

        assert_matches_matrix(&ite, &cover);
    }

    #[test]
    fn act_make_node_from_matrix_matches_cover() {
        let cover = matrix(&[&[1, 0, 2], &[2, 2, 1]]);
        let node = act_make_node_from_matrix(&cover).unwrap();

        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let inputs = [a, b, c];
                    assert_eq!(
                        node.evaluate(&inputs).unwrap(),
                        cover.evaluate(&inputs).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn name_to_element_table_keeps_column_names() {
        let cover = matrix(&[&[1, 0, 2]]);
        let table = act_make_name_to_element_table(&cover.rows()[0], &cover).unwrap();

        assert_eq!(table["a"].phase, LiteralPhase::Positive);
        assert_eq!(table["b"].phase, LiteralPhase::Negative);
        assert_eq!(table["c"].phase, LiteralPhase::DontCare);
    }

    #[test]
    fn blocked_sis_integration_reports_generic_prerequisite_error() {
        assert!(matches!(
            ite_for_cube_f_blocked(&(), &()),
            Err(ActIteError::MissingNativePorts { .. })
        ));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("act_ite.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
