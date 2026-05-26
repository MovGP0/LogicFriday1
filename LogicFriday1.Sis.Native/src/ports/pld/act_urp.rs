//! Owned-data Rust port of the ACT unate-recursive construction.
//!
//! The original implementation builds an ACT from a SIS node by translating the
//! node cover into an `sm_matrix`, recursively selecting binate or common
//! variables, splitting cofactors, and delegating unate leaves to ACT leaf
//! construction. This module keeps that behavior on explicit Rust cover data.
//! Direct SIS `node_t`, `sm_matrix`, `st_table`, and ACT slot mutation is
//! represented by blocked helpers until those integration layers have native
//! Rust ports.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Cover,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Zero,
    One,
    DontCare,
}

impl LiteralPhase {
    pub fn from_sis_literal(value: i32) -> ActUrpResult<Self> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::DontCare),
            _ => Err(ActUrpError::InvalidLiteralValue(value)),
        }
    }

    pub fn matches(self, input: bool) -> bool {
        match self {
            Self::Zero => !input,
            Self::One => input,
            Self::DontCare => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverNode {
    pub name: String,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub fanins: Vec<String>,
    pub cubes: Vec<Vec<LiteralPhase>>,
}

impl CoverNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            function: NodeFunction::Cover,
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }

    pub fn constant_zero(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Internal,
            function: NodeFunction::Zero,
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }

    pub fn constant_one(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Internal,
            function: NodeFunction::One,
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.fanins = fanins.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_cubes(mut self, cubes: Vec<Vec<LiteralPhase>>) -> Self {
        self.cubes = cubes;
        self
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
    ) -> ActUrpResult<Self> {
        let mut matrix = Self::new(column_names);
        for row in rows {
            matrix.push_row(row)?;
        }
        Ok(matrix)
    }

    pub fn push_row(&mut self, row: Vec<LiteralPhase>) -> ActUrpResult<()> {
        if row.len() != self.column_names.len() {
            return Err(ActUrpError::RowWidthMismatch {
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

    pub fn evaluate(&self, inputs: &[bool]) -> ActUrpResult<bool> {
        if inputs.len() != self.column_count() {
            return Err(ActUrpError::InputWidthMismatch {
                expected: self.column_count(),
                actual: inputs.len(),
            });
        }

        Ok(self.rows.iter().any(|row| {
            row.iter()
                .zip(inputs)
                .all(|(phase, input)| phase.matches(*input))
        }))
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
    Or(Vec<Act>),
}

impl Act {
    pub fn evaluate(&self, inputs: &[bool]) -> ActUrpResult<bool> {
        match self {
            Self::Terminal(value) => Ok(*value),
            Self::Decision {
                column, low, high, ..
            } => {
                let branch = if inputs
                    .get(*column)
                    .copied()
                    .ok_or(ActUrpError::UnknownColumn(*column))?
                {
                    high
                } else {
                    low
                };
                branch.evaluate(inputs)
            }
            Self::Or(acts) => {
                for act in acts {
                    if act.evaluate(inputs)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    pub fn index_size(&self) -> usize {
        match self {
            Self::Terminal(_) => 0,
            Self::Decision { index_size, .. } => *index_size,
            Self::Or(acts) => acts.iter().map(Self::index_size).sum(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SplitVariable {
    pub column: usize,
    pub common_to_all_rows: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActWithName {
    pub root: Act,
    pub node_name: String,
    pub order_style: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActUrpError {
    PrimaryNode { kind: NodeKind },
    RowWidthMismatch { expected: usize, actual: usize },
    InputWidthMismatch { expected: usize, actual: usize },
    UnknownColumn(usize),
    EmptyColumnName { column: usize },
    InvalidLiteralValue(i32),
    InvalidCommonVariable { column: usize, phase: LiteralPhase },
    EmptyCubeForAct,
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for ActUrpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrimaryNode { kind } => {
                write!(f, "make_act requires an internal node, got {kind:?}")
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
            Self::InvalidCommonVariable { column, phase } => {
                write!(
                    f,
                    "common variable column {column} has invalid phase {phase:?}"
                )
            }
            Self::EmptyCubeForAct => write!(f, "cannot build a cube ACT from an empty cube"),
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
        }
    }
}

impl Error for ActUrpError {}

pub type ActUrpResult<T> = Result<T, ActUrpError>;

pub fn make_act(node: &CoverNode) -> ActUrpResult<ActWithName> {
    match node.kind {
        NodeKind::Internal => {}
        kind => return Err(ActUrpError::PrimaryNode { kind }),
    }

    let root = match node.function {
        NodeFunction::Zero => Act::Terminal(false),
        NodeFunction::One => Act::Terminal(true),
        NodeFunction::Cover => urp_f(&build_f(node)?)?,
    };

    Ok(ActWithName {
        root,
        node_name: node.name.clone(),
        order_style: 0,
    })
}

pub fn build_f(node: &CoverNode) -> ActUrpResult<CoverMatrix> {
    CoverMatrix::from_rows(node.fanins.iter().cloned(), node.cubes.clone())
}

pub fn urp_f(matrix: &CoverMatrix) -> ActUrpResult<Act> {
    if matrix.row_count() == 0 {
        return Ok(Act::Terminal(false));
    }

    match good_bin_var(matrix) {
        Some(split) if split.common_to_all_rows => urp_common_variable(matrix, split.column),
        Some(split) => urp_binate_variable(matrix, split.column),
        None => unate_act(matrix),
    }
}

pub fn good_bin_var(matrix: &CoverMatrix) -> Option<SplitVariable> {
    let mut ones = vec![0usize; matrix.column_count()];
    let mut zeros = vec![0usize; matrix.column_count()];

    for row in matrix.rows() {
        for (column, phase) in row.iter().enumerate() {
            match phase {
                LiteralPhase::One => ones[column] += 1,
                LiteralPhase::Zero => zeros[column] += 1,
                LiteralPhase::DontCare => {}
            }
        }
    }

    for column in 0..matrix.column_count() {
        if ones[column] == matrix.row_count() || zeros[column] == matrix.row_count() {
            return Some(SplitVariable {
                column,
                common_to_all_rows: true,
            });
        }
    }

    find_var(&ones, &zeros).map(|column| SplitVariable {
        column,
        common_to_all_rows: false,
    })
}

pub fn find_var(ones: &[usize], zeros: &[usize]) -> Option<usize> {
    let mut best_total = 0usize;
    let mut best_difference = usize::MAX;
    let mut best_column = None;

    for (column, (one_count, zero_count)) in ones.iter().zip(zeros).enumerate() {
        if *one_count == 0 || *zero_count == 0 {
            continue;
        }

        let total = one_count + zero_count;
        let difference = one_count.abs_diff(*zero_count);
        if total > best_total || (total == best_total && difference < best_difference) {
            best_total = total;
            best_difference = difference;
            best_column = Some(column);
        }
    }

    best_column
}

pub fn split_f(matrix: &CoverMatrix, column: usize) -> ActUrpResult<(CoverMatrix, CoverMatrix)> {
    ensure_column(matrix, column)?;

    let mut low = CoverMatrix::new(matrix.column_names().iter().cloned());
    let mut high = CoverMatrix::new(matrix.column_names().iter().cloned());

    for row in matrix.rows() {
        let copied = copy_row_with_column_as_dont_care(row, column);
        match row[column] {
            LiteralPhase::Zero => low.push_row(copied)?,
            LiteralPhase::One => high.push_row(copied)?,
            LiteralPhase::DontCare => {
                low.push_row(copied.clone())?;
                high.push_row(copied)?;
            }
        }
    }

    Ok((low, high))
}

pub fn update_f(matrix: &mut CoverMatrix, column: usize) -> ActUrpResult<()> {
    ensure_column(matrix, column)?;
    for row in &mut matrix.rows {
        row[column] = LiteralPhase::DontCare;
    }
    Ok(())
}

pub fn unate_act(matrix: &CoverMatrix) -> ActUrpResult<Act> {
    if has_universal_cube(matrix) {
        return Ok(Act::Terminal(true));
    }
    if matrix.row_count() == 0 {
        return Ok(Act::Terminal(false));
    }

    let mut cube_acts = Vec::with_capacity(matrix.row_count());
    for row in matrix.rows() {
        cube_acts.push(act_f(matrix, row)?);
    }

    cube_acts.sort_by_key(Act::index_size);
    if cube_acts.len() == 1 {
        Ok(cube_acts.remove(0))
    } else {
        Ok(Act::Or(cube_acts))
    }
}

pub fn make_act_blocked<Node>(_node: &mut Node) -> ActUrpResult<ActWithName> {
    Err(missing_native_ports(
        "make_act SIS node/ACT slot integration",
    ))
}

pub fn build_f_blocked<Node>(_node: &Node) -> ActUrpResult<CoverMatrix> {
    Err(missing_native_ports(
        "build_F SIS node/sparse-matrix integration",
    ))
}

pub fn urp_f_blocked<Matrix>(_matrix: &Matrix) -> ActUrpResult<Act> {
    Err(missing_native_ports(
        "urp_F SIS sparse-matrix/terminal-table integration",
    ))
}

fn urp_binate_variable(matrix: &CoverMatrix, column: usize) -> ActUrpResult<Act> {
    let name = column_name(matrix, column)?.to_owned();
    let (low_matrix, high_matrix) = split_f(matrix, column)?;
    let low = urp_f(&low_matrix)?;
    let high = urp_f(&high_matrix)?;
    Ok(my_and_act(low, high, column, name))
}

fn urp_common_variable(matrix: &CoverMatrix, column: usize) -> ActUrpResult<Act> {
    let mut reduced = matrix.clone();
    let name = column_name(&reduced, column)?.to_owned();
    let phase = reduced
        .rows()
        .first()
        .and_then(|row| row.get(column))
        .copied()
        .ok_or(ActUrpError::UnknownColumn(column))?;
    update_f(&mut reduced, column)?;

    match phase {
        LiteralPhase::One => Ok(my_and_act(
            Act::Terminal(false),
            urp_f(&reduced)?,
            column,
            name,
        )),
        LiteralPhase::Zero => Ok(my_and_act(
            urp_f(&reduced)?,
            Act::Terminal(false),
            column,
            name,
        )),
        LiteralPhase::DontCare => Err(ActUrpError::InvalidCommonVariable { column, phase }),
    }
}

fn act_f(matrix: &CoverMatrix, row: &[LiteralPhase]) -> ActUrpResult<Act> {
    let mut literal_columns = row
        .iter()
        .enumerate()
        .filter_map(|(column, phase)| (*phase != LiteralPhase::DontCare).then_some(column))
        .collect::<Vec<_>>();

    if literal_columns.is_empty() {
        return Ok(Act::Terminal(true));
    }

    if let Some(first_positive) = literal_columns
        .iter()
        .position(|column| row[*column] == LiteralPhase::One)
    {
        literal_columns.swap(0, first_positive);
    }

    let first_column = literal_columns[0];
    let mut act = cube_literal_act(matrix, row[first_column], first_column, Act::Terminal(true))?;
    for column in literal_columns.into_iter().skip(1) {
        act = cube_literal_act(matrix, row[column], column, act)?;
    }

    Ok(act)
}

fn cube_literal_act(
    matrix: &CoverMatrix,
    phase: LiteralPhase,
    column: usize,
    continuation: Act,
) -> ActUrpResult<Act> {
    let name = column_name(matrix, column)?.to_owned();
    let index_size = continuation.index_size() + 1;
    match phase {
        LiteralPhase::Zero => Ok(Act::Decision {
            column,
            name,
            low: Box::new(continuation),
            high: Box::new(Act::Terminal(false)),
            index_size,
        }),
        LiteralPhase::One => Ok(Act::Decision {
            column,
            name,
            low: Box::new(Act::Terminal(false)),
            high: Box::new(continuation),
            index_size,
        }),
        LiteralPhase::DontCare => Err(ActUrpError::EmptyCubeForAct),
    }
}

fn my_and_act(low: Act, high: Act, column: usize, name: String) -> Act {
    let index_size = low.index_size() + high.index_size() + 1;
    Act::Decision {
        column,
        name,
        low: Box::new(low),
        high: Box::new(high),
        index_size,
    }
}

fn has_universal_cube(matrix: &CoverMatrix) -> bool {
    matrix
        .rows()
        .iter()
        .any(|row| row.iter().all(|phase| *phase == LiteralPhase::DontCare))
}

fn copy_row_with_column_as_dont_care(row: &[LiteralPhase], column: usize) -> Vec<LiteralPhase> {
    let mut copied = row.to_vec();
    copied[column] = LiteralPhase::DontCare;
    copied
}

fn column_name(matrix: &CoverMatrix, column: usize) -> ActUrpResult<&str> {
    let name = matrix
        .column_names()
        .get(column)
        .ok_or(ActUrpError::UnknownColumn(column))?;
    if name.is_empty() {
        return Err(ActUrpError::EmptyColumnName { column });
    }
    Ok(name)
}

fn ensure_column(matrix: &CoverMatrix, column: usize) -> ActUrpResult<()> {
    if column >= matrix.column_count() {
        Err(ActUrpError::UnknownColumn(column))
    } else {
        Ok(())
    }
}

fn missing_native_ports(operation: &'static str) -> ActUrpError {
    ActUrpError::MissingNativePorts { operation }
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

    fn assert_matches_cover(cover: &CoverMatrix, act: &Act) {
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let inputs = [a, b, c];
                    assert_eq!(
                        act.evaluate(&inputs).unwrap(),
                        cover.evaluate(&inputs).unwrap(),
                        "inputs = {inputs:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn make_act_handles_constant_nodes() {
        assert_eq!(
            make_act(&CoverNode::constant_zero("zero")).unwrap().root,
            Act::Terminal(false)
        );
        assert_eq!(
            make_act(&CoverNode::constant_one("one")).unwrap().root,
            Act::Terminal(true)
        );
    }

    #[test]
    fn build_f_preserves_literal_phases_and_fanin_names() {
        let node = CoverNode::new("n", NodeKind::Internal)
            .with_fanins(["a", "b", "c"])
            .with_cubes(vec![phases(&[1, 0, 2]), phases(&[2, 1, 0])]);
        let cover = build_f(&node).unwrap();

        assert_eq!(cover.column_names(), ["a", "b", "c"]);
        assert_eq!(cover.rows(), &[phases(&[1, 0, 2]), phases(&[2, 1, 0])]);
    }

    #[test]
    fn good_bin_var_prefers_common_then_most_binate_balanced_column() {
        assert_eq!(
            good_bin_var(&matrix(&[&[1, 0, 2], &[1, 1, 2]])),
            Some(SplitVariable {
                column: 0,
                common_to_all_rows: true
            })
        );
        assert_eq!(
            good_bin_var(&matrix(&[&[1, 0, 2], &[0, 1, 2], &[1, 1, 0]])),
            Some(SplitVariable {
                column: 0,
                common_to_all_rows: false
            })
        );
        assert_eq!(good_bin_var(&matrix(&[&[1, 2, 2], &[2, 2, 0]])), None);
    }

    #[test]
    fn split_f_routes_negative_positive_and_dont_care_rows() {
        let (low, high) = split_f(&matrix(&[&[2, 1, 0], &[1, 0, 2], &[0, 2, 1]]), 0).unwrap();

        assert_eq!(low.rows(), &[phases(&[2, 1, 0]), phases(&[2, 2, 1])]);
        assert_eq!(high.rows(), &[phases(&[2, 1, 0]), phases(&[2, 0, 2])]);
    }

    #[test]
    fn update_f_sets_common_column_to_dont_care() {
        let mut cover = matrix(&[&[1, 0, 2], &[0, 1, 2]]);

        update_f(&mut cover, 1).unwrap();

        assert_eq!(cover.rows(), &[phases(&[1, 2, 2]), phases(&[0, 2, 2])]);
    }

    #[test]
    fn unate_act_builds_single_cube_act_with_matching_function() {
        let cover = matrix(&[&[1, 0, 2]]);
        let act = unate_act(&cover).unwrap();

        assert_matches_cover(&cover, &act);
        assert_eq!(act.index_size(), 2);
    }

    #[test]
    fn universal_cube_prunes_to_one_terminal() {
        assert_eq!(
            unate_act(&matrix(&[&[2, 2, 2]])).unwrap(),
            Act::Terminal(true)
        );
    }

    #[test]
    fn recursive_urp_matches_original_cover_function() {
        let cover = matrix(&[&[1, 0, 2], &[0, 1, 2], &[1, 1, 0]]);
        let act = urp_f(&cover).unwrap();

        assert_matches_cover(&cover, &act);
    }

    #[test]
    fn common_variable_path_matches_cover_function() {
        let cover = matrix(&[&[1, 0, 2], &[1, 1, 0]]);
        let act = urp_f(&cover).unwrap();

        assert_matches_cover(&cover, &act);
        assert_eq!(act.evaluate(&[false, false, false]).unwrap(), false);
        assert_eq!(act.evaluate(&[true, false, true]).unwrap(), true);
        assert_eq!(act.evaluate(&[true, true, false]).unwrap(), true);
        assert_eq!(act.evaluate(&[true, true, true]).unwrap(), false);
    }

    #[test]
    fn sis_bound_helpers_report_generic_dependency_error() {
        let error = make_act_blocked(&mut ()).unwrap_err();

        assert!(matches!(error, ActUrpError::MissingNativePorts { .. }));
        assert!(
            error
                .to_string()
                .contains("requires native SIS prerequisite ports")
        );
    }
}
