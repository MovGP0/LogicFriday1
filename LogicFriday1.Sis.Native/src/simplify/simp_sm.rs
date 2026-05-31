//! Native Rust model for `LogicSynthesis/sis/simplify/simp_sm.c`.
//!
//! The original C file converts SIS `node_t` covers into an `sm_matrix`, prints
//! that matrix, and converts the remaining don't-care rows back into a node. The
//! owned model here preserves those transformations without exposing legacy C
//! ABI entry points. Integration with native SIS `node_t`, sparse-matrix, and
//! Espresso set-family ports is represented by explicit unavailable-operation
//! errors until a higher-level native integration layer wires those models.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

pub const F_SET: RowKind = RowKind::OnSet;
pub const DC_SET: RowKind = RowKind::DontCare;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicNode<N> {
    pub fanins: Vec<N>,
    pub cubes: Vec<Vec<Literal>>,
}

impl<N> LogicNode<N> {
    pub fn new(fanins: Vec<N>, cubes: Vec<Vec<Literal>>) -> Result<Self, SimpSmError> {
        let fanin_count = fanins.len();
        if let Some((cube_index, literal_count)) = cubes
            .iter()
            .enumerate()
            .find_map(|(index, cube)| (cube.len() != fanin_count).then_some((index, cube.len())))
        {
            return Err(SimpSmError::CubeWidthMismatch {
                cube_index,
                expected: fanin_count,
                actual: literal_count,
            });
        }

        Ok(Self { fanins, cubes })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowKind {
    OnSet,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimpSmMatrix<N> {
    rows: BTreeMap<usize, RowKind>,
    cols: BTreeMap<usize, N>,
    elements: BTreeMap<(usize, usize), bool>,
    rows_size: usize,
    cols_size: usize,
}

impl<N> Default for SimpSmMatrix<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N> SimpSmMatrix<N> {
    pub fn new() -> Self {
        Self {
            rows: BTreeMap::new(),
            cols: BTreeMap::new(),
            elements: BTreeMap::new(),
            rows_size: 0,
            cols_size: 0,
        }
    }

    pub fn rows_size(&self) -> usize {
        self.rows_size
    }

    pub fn cols_size(&self) -> usize {
        self.cols_size
    }

    pub fn nrows(&self) -> usize {
        self.rows.len()
    }

    pub fn ncols(&self) -> usize {
        self.cols.len()
    }

    pub fn row_kind(&self, row: usize) -> Option<RowKind> {
        self.rows.get(&row).copied()
    }

    pub fn column_fanin(&self, col: usize) -> Option<&N> {
        self.cols.get(&col)
    }

    pub fn element(&self, row: usize, col: usize) -> Option<bool> {
        self.elements.get(&(row, col)).copied()
    }

    pub fn active_columns(&self) -> impl Iterator<Item = usize> + '_ {
        self.cols.keys().copied().filter(|col| {
            self.elements
                .keys()
                .any(|(_, element_col)| element_col == col)
        })
    }

    pub fn active_rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.rows.keys().copied().filter(|row| {
            self.elements
                .keys()
                .any(|(element_row, _)| element_row == row)
        })
    }

    fn insert(&mut self, row: usize, col: usize, value: bool, fanin: N, kind: RowKind) {
        self.rows.insert(row, kind);
        self.cols.insert(col, fanin);
        self.elements.insert((row, col), value);
        self.rows_size = self.rows_size.max(row + 1);
        self.cols_size = self.cols_size.max(col + 1);
    }

    fn ensure_row(&mut self, row: usize, kind: RowKind) {
        self.rows.insert(row, kind);
        self.rows_size = self.rows_size.max(row + 1);
    }

    fn delete_on_set_rows(&mut self) {
        let on_rows = self
            .rows
            .iter()
            .filter_map(|(row, kind)| (*kind == RowKind::OnSet).then_some(*row))
            .collect::<BTreeSet<_>>();
        self.rows.retain(|row, _| !on_rows.contains(row));
        self.elements.retain(|(row, _), _| !on_rows.contains(row));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimpSmError {
    CubeWidthMismatch {
        cube_index: usize,
        expected: usize,
        actual: usize,
    },
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for SimpSmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CubeWidthMismatch {
                cube_index,
                expected,
                actual,
            } => write!(
                f,
                "cube {cube_index} has {actual} literals, expected {expected}"
            ),
            Self::MissingSisPorts { operation } => write!(
                f,
                "{operation} requires a native Rust SIS integration layer"
            ),
        }
    }
}

impl Error for SimpSmError {}

pub fn sis_node_to_sm_unavailable<N>() -> Result<SimpSmMatrix<N>, SimpSmError> {
    Err(SimpSmError::MissingSisPorts {
        operation: "simp_node_to_sm native SIS integration",
    })
}

pub fn sis_sm_to_node_unavailable<N>() -> Result<LogicNode<N>, SimpSmError> {
    Err(SimpSmError::MissingSisPorts {
        operation: "simp_sm_to_node native SIS integration",
    })
}

pub fn simp_node_to_sm<N>(on_set: &LogicNode<N>, dc_set: &LogicNode<N>) -> SimpSmMatrix<N>
where
    N: Clone + Eq + Hash,
{
    let mut matrix = SimpSmMatrix::new();
    let mut fanin_to_col = HashMap::new();

    for (row_index, cube) in on_set.cubes.iter().enumerate() {
        for (fanin_index, literal) in cube.iter().copied().enumerate() {
            let value = match literal {
                Literal::Zero => false,
                Literal::One => true,
                Literal::DontCare => continue,
            };
            let fanin = on_set.fanins[fanin_index].clone();
            fanin_to_col.insert(fanin.clone(), fanin_index);
            matrix.insert(row_index, fanin_index, value, fanin, RowKind::OnSet);
        }
        matrix.ensure_row(row_index, RowKind::OnSet);
    }

    let f_numrows = on_set.cubes.len();
    for (dc_index, cube) in dc_set.cubes.iter().enumerate() {
        let row_index = dc_index + f_numrows;
        for (fanin_index, literal) in cube.iter().copied().enumerate() {
            let value = match literal {
                Literal::Zero => false,
                Literal::One => true,
                Literal::DontCare => continue,
            };
            let fanin = dc_set.fanins[fanin_index].clone();
            let next_col = matrix.cols_size();
            let col = *fanin_to_col.entry(fanin.clone()).or_insert(next_col);
            matrix.insert(row_index, col, value, fanin, RowKind::DontCare);
        }
        matrix.ensure_row(row_index, RowKind::DontCare);
    }

    matrix
}

pub fn simp_sm_to_node<N>(matrix: &SimpSmMatrix<N>) -> LogicNode<N>
where
    N: Clone,
{
    let mut dc_matrix = matrix.clone();
    dc_matrix.delete_on_set_rows();

    let active_cols = dc_matrix.active_columns().collect::<Vec<_>>();
    let col_to_fanin_index = active_cols
        .iter()
        .enumerate()
        .map(|(index, col)| (*col, index))
        .collect::<HashMap<_, _>>();
    let fanins = active_cols
        .iter()
        .filter_map(|col| dc_matrix.column_fanin(*col).cloned())
        .collect::<Vec<_>>();

    let mut cubes = Vec::new();
    for row in dc_matrix.rows.keys().copied() {
        if dc_matrix.row_kind(row) != Some(RowKind::DontCare) {
            continue;
        }

        let mut cube = vec![Literal::DontCare; fanins.len()];
        for col in &active_cols {
            let Some(value) = dc_matrix.element(row, *col) else {
                continue;
            };
            let fanin_index = col_to_fanin_index[col];
            cube[fanin_index] = if value { Literal::One } else { Literal::Zero };
        }
        cubes.push(cube);
    }

    LogicNode { fanins, cubes }
}

pub fn simp_sm_print<N>(matrix: &SimpSmMatrix<N>) -> String
where
    N: fmt::Display,
{
    let cols = matrix.cols.keys().copied().collect::<Vec<_>>();
    let mut output = String::from("\n   ");
    for col in &cols {
        let name = matrix
            .column_fanin(*col)
            .expect("column key came from the column map");
        output.push_str(&format!("{name:>3}"));
    }
    output.push('\n');

    for row in 0..matrix.rows_size() {
        let Some(kind) = matrix.row_kind(row) else {
            continue;
        };
        let row_label = match kind {
            RowKind::OnSet => 'f',
            RowKind::DontCare => 'd',
        };
        output.push_str(&format!(" {row_label} "));
        for col in &cols {
            match matrix.element(row, *col) {
                Some(false) => output.push_str("  0"),
                Some(true) => output.push_str("  1"),
                None => output.push_str("  ."),
            }
        }
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(fanins: &[&str], cubes: Vec<Vec<Literal>>) -> LogicNode<String> {
        LogicNode::new(
            fanins.iter().map(|name| (*name).to_string()).collect(),
            cubes,
        )
        .unwrap()
    }

    #[test]
    fn node_to_sm_preserves_on_set_columns_before_new_dc_columns() {
        let f = node(&["a", "b"], vec![vec![Literal::One, Literal::DontCare]]);
        let dc = node(
            &["b", "c", "a"],
            vec![vec![Literal::Zero, Literal::One, Literal::One]],
        );

        let matrix = simp_node_to_sm(&f, &dc);

        assert_eq!(matrix.row_kind(0), Some(F_SET));
        assert_eq!(matrix.row_kind(1), Some(DC_SET));
        assert_eq!(matrix.column_fanin(0), Some(&"a".to_string()));
        assert_eq!(matrix.column_fanin(1), Some(&"b".to_string()));
        assert_eq!(matrix.column_fanin(2), Some(&"c".to_string()));
        assert_eq!(matrix.element(0, 0), Some(true));
        assert_eq!(matrix.element(1, 1), Some(false));
        assert_eq!(matrix.element(1, 2), Some(true));
        assert_eq!(matrix.element(1, 0), Some(true));
    }

    #[test]
    fn sm_to_node_drops_on_set_rows_and_rebuilds_dc_cover() {
        let f = node(&["a", "b"], vec![vec![Literal::One, Literal::DontCare]]);
        let dc = node(
            &["b", "c", "a"],
            vec![
                vec![Literal::Zero, Literal::One, Literal::One],
                vec![Literal::DontCare, Literal::Zero, Literal::DontCare],
            ],
        );

        let matrix = simp_node_to_sm(&f, &dc);
        let rebuilt = simp_sm_to_node(&matrix);

        assert_eq!(
            rebuilt,
            LogicNode {
                fanins: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                cubes: vec![
                    vec![Literal::One, Literal::Zero, Literal::One],
                    vec![Literal::DontCare, Literal::DontCare, Literal::Zero],
                ],
            }
        );
    }

    #[test]
    fn sm_to_node_keeps_empty_dc_rows_as_empty_cubes() {
        let f = node(&[], Vec::new());
        let dc = node(&[], vec![Vec::new()]);

        let matrix = simp_node_to_sm(&f, &dc);
        let rebuilt = simp_sm_to_node(&matrix);

        assert_eq!(
            rebuilt,
            LogicNode {
                fanins: Vec::<String>::new(),
                cubes: vec![Vec::new()],
            }
        );
    }

    #[test]
    fn print_matches_c_layout_shape() {
        let f = node(&["a", "b"], vec![vec![Literal::Zero, Literal::One]]);
        let matrix = simp_node_to_sm(&f, &node(&[], Vec::new()));

        assert_eq!(simp_sm_print(&matrix), "\n     a  b\n f   0  1\n");
    }

    #[test]
    fn malformed_cube_width_reports_an_error() {
        let err = LogicNode::new(vec!["a"], vec![vec![Literal::One, Literal::Zero]])
            .expect_err("cube width must be validated");

        assert_eq!(
            err,
            SimpSmError::CubeWidthMismatch {
                cube_index: 0,
                expected: 1,
                actual: 2,
            }
        );
    }

    #[test]
    fn sis_integration_reports_missing_sis_ports() {
        let err = sis_node_to_sm_unavailable::<String>()
            .expect_err("SIS integration should report unavailable native support");

        match err {
            SimpSmError::MissingSisPorts { operation } => {
                assert_eq!(operation, "simp_node_to_sm native SIS integration");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
