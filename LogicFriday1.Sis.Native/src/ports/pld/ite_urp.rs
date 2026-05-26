//! Native Rust model for `LogicSynthesis/sis/pld/ite_urp.c`.
//!
//! The C file builds an ITE for a SIS node by translating its cover into an
//! `sm_matrix`, recursively choosing binate or common variables, splitting the
//! cover into cofactors, and assembling Shannon ITE vertices. This module keeps
//! that deterministic cover/ITE behavior on owned Rust data. Direct mutation of
//! SIS `node_t`, `network_t`, `sm_matrix`, and global terminal tables remains
//! represented by explicit dependency errors.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.457",
        source_file: "LogicSynthesis/sis/sparse/matrix.c",
        reason: "ite_urp.c stores and splits node covers with sm_matrix rows, columns, and elements",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.456",
        source_file: "LogicSynthesis/sis/sparse/cols.c",
        reason: "good_bin_var, update_F, and ite_split_F iterate sparse matrix columns",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.458",
        source_file: "LogicSynthesis/sis/sparse/rows.c",
        reason: "build_F and act_mux_inputs_special_case inspect sparse matrix rows",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "make_ite uses ite_end_table to share terminal ITE vertices",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "build_F enumerates fanins and urp_F resolves selected columns back to fanin nodes",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.317",
        source_file: "LogicSynthesis/sis/node/names.c",
        reason: "build_F stores node_long_name values in cover columns",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "make_ite depends on node_function, node cubes, node literals, and node type checks",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.366",
        source_file: "LogicSynthesis/sis/pld/ite_imp.c",
        reason: "legacy SIS integration stores the resulting ITE in ACT_ITE_ite(node)",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.367",
        source_file: "LogicSynthesis/sis/pld/ite_leaf.c",
        reason: "urp_F delegates unate covers to unate_ite",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.360",
        source_file: "LogicSynthesis/sis/pld/act_util.c",
        reason: "ite_split_F calls my_sm_copy_row in the SIS sparse-matrix implementation",
    },
];

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
    pub fn from_sis_literal(value: i32) -> IteUrpResult<Self> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::DontCare),
            _ => Err(IteUrpError::InvalidLiteralValue(value)),
        }
    }

    pub fn matches(self, value: bool) -> bool {
        match self {
            Self::Zero => !value,
            Self::One => value,
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
    ) -> IteUrpResult<Self> {
        let mut matrix = Self::new(column_names);
        for row in rows {
            matrix.push_row(row)?;
        }
        Ok(matrix)
    }

    pub fn push_row(&mut self, row: Vec<LiteralPhase>) -> IteUrpResult<()> {
        if row.len() != self.column_names.len() {
            return Err(IteUrpError::RowWidthMismatch {
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

    pub fn evaluate(&self, inputs: &[bool]) -> IteUrpResult<bool> {
        if inputs.len() != self.column_count() {
            return Err(IteUrpError::InputWidthMismatch {
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
    },
    Shannon {
        condition: Box<Ite>,
        then_branch: Box<Ite>,
        else_branch: Box<Ite>,
    },
    UnateCover(CoverMatrix),
}

impl Ite {
    pub fn evaluate(&self, inputs: &[bool]) -> IteUrpResult<bool> {
        match self {
            Self::Terminal(value) => Ok(*value),
            Self::Literal { column, .. } => inputs
                .get(*column)
                .copied()
                .ok_or(IteUrpError::UnknownColumn(*column)),
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
            Self::UnateCover(matrix) => matrix.evaluate(inputs),
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
            Self::UnateCover(matrix) => matrix
                .rows()
                .iter()
                .map(|row| {
                    row.iter()
                        .filter(|phase| **phase != LiteralPhase::DontCare)
                        .count()
                })
                .sum(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SplitVariable {
    pub column: usize,
    pub common_to_all_rows: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MuxSpecialCase {
    NegativeLiteral { a_col: usize },
    NegativeProduct { a_col: usize, b_col: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteUrpError {
    PrimaryNode {
        kind: NodeKind,
    },
    RowWidthMismatch {
        expected: usize,
        actual: usize,
    },
    InputWidthMismatch {
        expected: usize,
        actual: usize,
    },
    UnknownColumn(usize),
    EmptyColumnName {
        column: usize,
    },
    InvalidLiteralValue(i32),
    InvalidCommonVariable {
        column: usize,
        phase: LiteralPhase,
    },
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for IteUrpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrimaryNode { kind } => {
                write!(f, "make_ite requires an internal node, got {kind:?}")
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
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS C-file dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for IteUrpError {}

pub type IteUrpResult<T> = Result<T, IteUrpError>;

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn make_ite(node: &CoverNode) -> IteUrpResult<Ite> {
    match node.kind {
        NodeKind::Internal => {}
        kind => return Err(IteUrpError::PrimaryNode { kind }),
    }

    match node.function {
        NodeFunction::Zero => return Ok(Ite::Terminal(false)),
        NodeFunction::One => return Ok(Ite::Terminal(true)),
        NodeFunction::Cover => {}
    }

    let matrix = build_f(node)?;
    urp_f(&matrix)
}

pub fn build_f(node: &CoverNode) -> IteUrpResult<CoverMatrix> {
    CoverMatrix::from_rows(node.fanins.iter().cloned(), node.cubes.clone())
}

pub fn urp_f(matrix: &CoverMatrix) -> IteUrpResult<Ite> {
    if matrix.row_count() == 0 {
        return Ok(Ite::Terminal(false));
    }

    match good_bin_var(matrix) {
        Some(split) if split.common_to_all_rows => urp_common_variable(matrix, split.column),
        Some(split) => urp_binate_variable(matrix, split.column),
        None => Ok(Ite::UnateCover(matrix.clone())),
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

    if (0..matrix.column_count()).all(|column| ones[column] == 0 || zeros[column] == 0) {
        return None;
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
    let mut best_balance_total = 0usize;
    let mut best_difference = usize::MAX;
    let mut best_column = None;

    for (column, (one_count, zero_count)) in ones.iter().zip(zeros).enumerate() {
        if *one_count == 0 || *zero_count == 0 {
            continue;
        }

        let total = one_count + zero_count;
        let difference = one_count.abs_diff(*zero_count);
        if total > best_balance_total
            || (total == best_balance_total && difference < best_difference)
        {
            best_balance_total = total;
            best_difference = difference;
            best_column = Some(column);
        }
    }

    best_column
}

pub fn ite_split_f(
    matrix: &CoverMatrix,
    column: usize,
) -> IteUrpResult<(CoverMatrix, CoverMatrix, CoverMatrix)> {
    ensure_column(matrix, column)?;
    let mut then_else = CoverMatrix::new(matrix.column_names().iter().cloned());
    let mut then_matrix = CoverMatrix::new(matrix.column_names().iter().cloned());
    let mut else_matrix = CoverMatrix::new(matrix.column_names().iter().cloned());

    for row in matrix.rows() {
        let copied = copy_row_with_column_as_dont_care(row, column);
        match row[column] {
            LiteralPhase::Zero => else_matrix.push_row(copied)?,
            LiteralPhase::One => then_matrix.push_row(copied)?,
            LiteralPhase::DontCare => then_else.push_row(copied)?,
        }
    }

    Ok((then_else, then_matrix, else_matrix))
}

pub fn update_f(matrix: &mut CoverMatrix, column: usize) -> IteUrpResult<()> {
    ensure_column(matrix, column)?;
    for row in &mut matrix.rows {
        row[column] = LiteralPhase::DontCare;
    }
    Ok(())
}

pub fn act_mux_inputs_special_case(matrix: &CoverMatrix) -> Option<MuxSpecialCase> {
    if matrix.row_count() != 1 {
        return None;
    }

    let mut negative_columns = matrix.rows()[0]
        .iter()
        .enumerate()
        .filter_map(|(column, phase)| match phase {
            LiteralPhase::Zero => Some(column),
            LiteralPhase::One => None,
            LiteralPhase::DontCare => None,
        });

    if matrix.rows()[0].contains(&LiteralPhase::One) {
        return None;
    }

    let a_col = negative_columns.next()?;
    match (negative_columns.next(), negative_columns.next()) {
        (None, _) => Some(MuxSpecialCase::NegativeLiteral { a_col }),
        (Some(b_col), None) => Some(MuxSpecialCase::NegativeProduct { a_col, b_col }),
        (Some(_), Some(_)) => None,
    }
}

pub fn act_sm_complement_special_case(
    matrix: &CoverMatrix,
    special_case: MuxSpecialCase,
) -> IteUrpResult<CoverMatrix> {
    let mut not_f = CoverMatrix::new(matrix.column_names().iter().cloned());
    match special_case {
        MuxSpecialCase::NegativeLiteral { a_col } => {
            not_f.push_row(single_positive_row(matrix.column_count(), a_col)?)?;
        }
        MuxSpecialCase::NegativeProduct { a_col, b_col } => {
            not_f.push_row(single_positive_row(matrix.column_count(), a_col)?)?;
            not_f.push_row(single_positive_row(matrix.column_count(), b_col)?)?;
        }
    }
    Ok(not_f)
}

pub fn make_ite_blocked<Node>(_node: &mut Node) -> IteUrpResult<Ite> {
    Err(missing_native_ports("make_ite SIS node integration"))
}

pub fn build_f_blocked<Node>(_node: &Node) -> IteUrpResult<CoverMatrix> {
    Err(missing_native_ports(
        "build_F SIS node/sm_matrix integration",
    ))
}

pub fn urp_f_blocked<Matrix, Node>(_matrix: &Matrix, _node: &Node) -> IteUrpResult<Ite> {
    Err(missing_native_ports("urp_F SIS sm_matrix/node integration"))
}

fn urp_binate_variable(matrix: &CoverMatrix, column: usize) -> IteUrpResult<Ite> {
    let name = column_name(matrix, column)?.to_owned();
    let (mut if_matrix, then_matrix, else_matrix) = ite_split_f(matrix, column)?;
    let special_case = act_mux_inputs_special_case(&if_matrix);

    if let Some(special_case) = special_case {
        if_matrix = act_sm_complement_special_case(&if_matrix, special_case)?;
    }

    let if_ite = urp_f(&if_matrix)?;
    let else_then = urp_f(&then_matrix)?;
    let else_else = urp_f(&else_matrix)?;
    let else_if = ite_literal(column, name);
    let else_ite = my_shannon_ite(else_if, else_then, else_else);

    if if_matrix.row_count() == 0 {
        Ok(else_ite)
    } else if special_case.is_some() {
        Ok(my_shannon_ite(if_ite, else_ite, Ite::Terminal(true)))
    } else {
        Ok(my_shannon_ite(if_ite, Ite::Terminal(true), else_ite))
    }
}

fn urp_common_variable(matrix: &CoverMatrix, column: usize) -> IteUrpResult<Ite> {
    let mut reduced = matrix.clone();
    let name = column_name(&reduced, column)?.to_owned();
    let phase = reduced
        .rows()
        .first()
        .and_then(|row| row.get(column))
        .copied()
        .ok_or(IteUrpError::UnknownColumn(column))?;
    update_f(&mut reduced, column)?;

    let if_ite = ite_literal(column, name);
    match phase {
        LiteralPhase::One => Ok(my_shannon_ite(
            if_ite,
            urp_f(&reduced)?,
            Ite::Terminal(false),
        )),
        LiteralPhase::Zero => Ok(my_shannon_ite(
            if_ite,
            Ite::Terminal(false),
            urp_f(&reduced)?,
        )),
        LiteralPhase::DontCare => Err(IteUrpError::InvalidCommonVariable { column, phase }),
    }
}

fn ite_literal(column: usize, name: String) -> Ite {
    Ite::Literal { column, name }
}

fn my_shannon_ite(condition: Ite, then_branch: Ite, else_branch: Ite) -> Ite {
    Ite::Shannon {
        condition: Box::new(condition),
        then_branch: Box::new(then_branch),
        else_branch: Box::new(else_branch),
    }
}

fn copy_row_with_column_as_dont_care(row: &[LiteralPhase], column: usize) -> Vec<LiteralPhase> {
    let mut copied = row.to_vec();
    copied[column] = LiteralPhase::DontCare;
    copied
}

fn single_positive_row(width: usize, column: usize) -> IteUrpResult<Vec<LiteralPhase>> {
    if column >= width {
        return Err(IteUrpError::UnknownColumn(column));
    }

    let mut row = vec![LiteralPhase::DontCare; width];
    row[column] = LiteralPhase::One;
    Ok(row)
}

fn column_name(matrix: &CoverMatrix, column: usize) -> IteUrpResult<&str> {
    let name = matrix
        .column_names()
        .get(column)
        .ok_or(IteUrpError::UnknownColumn(column))?;
    if name.is_empty() {
        return Err(IteUrpError::EmptyColumnName { column });
    }
    Ok(name)
}

fn ensure_column(matrix: &CoverMatrix, column: usize) -> IteUrpResult<()> {
    if column >= matrix.column_count() {
        Err(IteUrpError::UnknownColumn(column))
    } else {
        Ok(())
    }
}

fn missing_native_ports(operation: &'static str) -> IteUrpError {
    IteUrpError::MissingNativePorts {
        operation,
        dependencies: REQUIRED_PORT_DEPENDENCIES,
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

    #[test]
    fn constants_match_make_ite_trivial_cases() {
        assert_eq!(
            make_ite(&CoverNode::constant_zero("zero")).unwrap(),
            Ite::Terminal(false)
        );
        assert_eq!(
            make_ite(&CoverNode::constant_one("one")).unwrap(),
            Ite::Terminal(true)
        );
    }

    #[test]
    fn build_f_preserves_cube_literal_phases_and_fanin_names() {
        let node = CoverNode::new("n", NodeKind::Internal)
            .with_fanins(["a", "b", "c"])
            .with_cubes(vec![phases(&[1, 0, 2]), phases(&[2, 1, 0])]);
        let cover = build_f(&node).unwrap();

        assert_eq!(cover.column_names(), ["a", "b", "c"]);
        assert_eq!(cover.rows(), &[phases(&[1, 0, 2]), phases(&[2, 1, 0])]);
    }

    #[test]
    fn good_bin_var_detects_unate_common_and_most_binate_cases() {
        assert_eq!(good_bin_var(&matrix(&[&[1, 2, 0], &[1, 2, 2]])), None);
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
    }

    #[test]
    fn split_sets_factored_column_to_dont_care() {
        let (then_else, then_matrix, else_matrix) =
            ite_split_f(&matrix(&[&[2, 1, 0], &[1, 0, 2], &[0, 2, 1]]), 0).unwrap();

        assert_eq!(then_else.rows(), &[phases(&[2, 1, 0])]);
        assert_eq!(then_matrix.rows(), &[phases(&[2, 0, 2])]);
        assert_eq!(else_matrix.rows(), &[phases(&[2, 2, 1])]);
    }

    #[test]
    fn mux_special_case_detects_negative_literal_and_product() {
        let negative_literal = matrix(&[&[0, 2, 2]]);
        let negative_product = matrix(&[&[0, 2, 0]]);

        assert_eq!(
            act_mux_inputs_special_case(&negative_literal),
            Some(MuxSpecialCase::NegativeLiteral { a_col: 0 })
        );
        assert_eq!(
            act_mux_inputs_special_case(&negative_product),
            Some(MuxSpecialCase::NegativeProduct { a_col: 0, b_col: 2 })
        );
        assert_eq!(act_mux_inputs_special_case(&matrix(&[&[1, 0, 2]])), None);
    }

    #[test]
    fn complement_special_case_turns_negative_product_into_or_rows() {
        let cover = matrix(&[&[0, 2, 0]]);
        let complemented = act_sm_complement_special_case(
            &cover,
            MuxSpecialCase::NegativeProduct { a_col: 0, b_col: 2 },
        )
        .unwrap();

        assert_eq!(
            complemented.rows(),
            &[phases(&[1, 2, 2]), phases(&[2, 2, 1])]
        );
    }

    #[test]
    fn recursive_ite_evaluates_same_function_as_cover() {
        let cover = matrix(&[&[1, 0, 2], &[0, 1, 2], &[1, 1, 0]]);
        let ite = urp_f(&cover).unwrap();

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
    fn common_variable_path_matches_cover_function() {
        let cover = matrix(&[&[1, 0, 2], &[1, 1, 0]]);
        let ite = urp_f(&cover).unwrap();

        assert_eq!(ite.evaluate(&[false, false, false]).unwrap(), false);
        assert_eq!(ite.evaluate(&[true, false, true]).unwrap(), true);
        assert_eq!(ite.evaluate(&[true, true, false]).unwrap(), true);
        assert_eq!(ite.evaluate(&[true, true, true]).unwrap(), false);
    }

    #[test]
    fn blocked_sis_entries_report_dependency_beads_and_sources() {
        let mut sis_node = ();
        let Err(IteUrpError::MissingNativePorts { dependencies, .. }) =
            make_ite_blocked(&mut sis_node)
        else {
            panic!("expected missing dependency error");
        };

        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.367"
                && dependency.source_file == "LogicSynthesis/sis/pld/ite_leaf.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.457"
                && dependency.source_file == "LogicSynthesis/sis/sparse/matrix.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.318"
                && dependency.source_file == "LogicSynthesis/sis/node/node.c"
        }));
    }
}
