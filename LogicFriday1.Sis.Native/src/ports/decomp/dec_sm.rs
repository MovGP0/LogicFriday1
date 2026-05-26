//! Native Rust model for `LogicSynthesis/sis/decomp/dec_sm.c`.
//!
//! The original unit converts an SIS node cover to an `sm_matrix`, stores each
//! matrix element as a literal phase, stores each column as its fanin node, and
//! rebuilds the cover by OR-ing row products. This port keeps that behavior in
//! owned Rust data without legacy C ABI entry points.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub type DecompositionSmResult<T> = Result<T, DecompositionSmError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal
{
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicNode<N>
{
    pub fanins: Vec<N>,
    pub cubes: Vec<Vec<Literal>>,
}

impl<N> LogicNode<N>
{
    pub fn new(fanins: Vec<N>, cubes: Vec<Vec<Literal>>) -> DecompositionSmResult<Self>
    {
        validate_cube_widths(fanins.len(), &cubes)?;

        Ok(Self { fanins, cubes })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecompositionMatrix<N>
{
    rows: BTreeSet<usize>,
    cols: BTreeMap<usize, N>,
    elements: BTreeMap<(usize, usize), bool>,
    rows_size: usize,
    cols_size: usize,
}

impl<N> Default for DecompositionMatrix<N>
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl<N> DecompositionMatrix<N>
{
    pub fn new() -> Self
    {
        Self {
            rows: BTreeSet::new(),
            cols: BTreeMap::new(),
            elements: BTreeMap::new(),
            rows_size: 0,
            cols_size: 0,
        }
    }

    pub fn rows_size(&self) -> usize
    {
        self.rows_size
    }

    pub fn cols_size(&self) -> usize
    {
        self.cols_size
    }

    pub fn row_count(&self) -> usize
    {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize
    {
        self.cols.len()
    }

    pub fn row_exists(&self, row: usize) -> bool
    {
        self.rows.contains(&row)
    }

    pub fn column_fanin(&self, col: usize) -> Option<&N>
    {
        self.cols.get(&col)
    }

    pub fn element(&self, row: usize, col: usize) -> Option<bool>
    {
        self.elements.get(&(row, col)).copied()
    }

    pub fn rows(&self) -> impl Iterator<Item = usize> + '_
    {
        self.rows.iter().copied()
    }

    pub fn columns(&self) -> impl Iterator<Item = usize> + '_
    {
        self.cols.keys().copied()
    }

    pub fn insert(&mut self, row: usize, col: usize, value: bool, fanin: N)
    {
        self.ensure_row(row);
        self.ensure_col(col, fanin);
        self.elements.insert((row, col), value);
    }

    pub fn ensure_row(&mut self, row: usize)
    {
        self.rows.insert(row);
        self.rows_size = self.rows_size.max(row + 1);
    }

    fn ensure_col(&mut self, col: usize, fanin: N)
    {
        self.cols.entry(col).or_insert(fanin);
        self.cols_size = self.cols_size.max(col + 1);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecompositionSmError
{
    CubeWidthMismatch {
        cube_index: usize,
        expected: usize,
        actual: usize,
    },
    MissingColumnFanin {
        col: usize,
    },
}

impl fmt::Display for DecompositionSmError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::CubeWidthMismatch {
                cube_index,
                expected,
                actual,
            } => write!(
                f,
                "cube {cube_index} has {actual} literals, expected {expected}"
            ),
            Self::MissingColumnFanin { col } => {
                write!(f, "matrix column {col} has elements but no fanin metadata")
            }
        }
    }
}

impl Error for DecompositionSmError {}

pub fn dec_node_to_sm<N>(node: &LogicNode<N>) -> DecompositionSmResult<DecompositionMatrix<N>>
where
    N: Clone,
{
    validate_cube_widths(node.fanins.len(), &node.cubes)?;

    let mut matrix = DecompositionMatrix::new();

    for (row, cube) in node.cubes.iter().enumerate()
    {
        matrix.ensure_row(row);

        for (col, literal) in cube.iter().copied().enumerate()
        {
            let value = match literal
            {
                Literal::Zero => false,
                Literal::One => true,
                Literal::DontCare => continue,
            };

            matrix.insert(row, col, value, node.fanins[col].clone());
        }
    }

    Ok(matrix)
}

pub fn dec_sm_to_node<N>(matrix: &DecompositionMatrix<N>) -> DecompositionSmResult<LogicNode<N>>
where
    N: Clone,
{
    let columns = matrix.columns().collect::<Vec<_>>();
    let mut fanins = Vec::with_capacity(columns.len());

    for col in &columns
    {
        let Some(fanin) = matrix.column_fanin(*col) else
        {
            return Err(DecompositionSmError::MissingColumnFanin { col: *col });
        };
        fanins.push(fanin.clone());
    }

    let mut cubes = Vec::new();
    for row in matrix.rows()
    {
        let mut cube = vec![Literal::DontCare; columns.len()];
        for (fanin_index, col) in columns.iter().enumerate()
        {
            let Some(value) = matrix.element(row, *col) else
            {
                continue;
            };
            cube[fanin_index] = if value
            {
                Literal::One
            }
            else
            {
                Literal::Zero
            };
        }
        cubes.push(cube);
    }

    Ok(LogicNode { fanins, cubes })
}

pub fn dec_sm_print<N>(matrix: &DecompositionMatrix<N>) -> String
where
    N: fmt::Display,
{
    let mut output = String::new();

    for row in 0..matrix.rows_size()
    {
        for col in 0..matrix.cols_size()
        {
            match matrix.element(row, col)
            {
                Some(false) => output.push_str("  0"),
                Some(true) => output.push_str("  1"),
                None => output.push_str("  ."),
            }
        }
        output.push('\n');
    }

    for col in 0..matrix.cols_size()
    {
        if let Some(fanin) = matrix.column_fanin(col)
        {
            output.push_str(&format!("{fanin:>3}"));
        }
    }
    output.push('\n');

    output
}

fn validate_cube_widths(
    fanin_count: usize,
    cubes: &[Vec<Literal>],
) -> DecompositionSmResult<()>
{
    if let Some((cube_index, actual)) = cubes
        .iter()
        .enumerate()
        .find_map(|(index, cube)| (cube.len() != fanin_count).then_some((index, cube.len())))
    {
        return Err(DecompositionSmError::CubeWidthMismatch {
            cube_index,
            expected: fanin_count,
            actual,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn node(fanins: &[&str], cubes: Vec<Vec<Literal>>) -> LogicNode<String>
    {
        LogicNode::new(
            fanins.iter().map(|fanin| (*fanin).to_string()).collect(),
            cubes,
        )
        .unwrap()
    }

    #[test]
    fn node_to_sm_stores_only_care_literals_and_column_fanins()
    {
        let source = node(
            &["a", "b", "c"],
            vec![
                vec![Literal::One, Literal::DontCare, Literal::Zero],
                vec![Literal::DontCare, Literal::Zero, Literal::One],
            ],
        );

        let matrix = dec_node_to_sm(&source).unwrap();

        assert_eq!(matrix.rows_size(), 2);
        assert_eq!(matrix.cols_size(), 3);
        assert_eq!(matrix.row_count(), 2);
        assert_eq!(matrix.col_count(), 3);
        assert_eq!(matrix.column_fanin(0), Some(&"a".to_string()));
        assert_eq!(matrix.column_fanin(1), Some(&"b".to_string()));
        assert_eq!(matrix.column_fanin(2), Some(&"c".to_string()));
        assert_eq!(matrix.element(0, 0), Some(true));
        assert_eq!(matrix.element(0, 1), None);
        assert_eq!(matrix.element(0, 2), Some(false));
        assert_eq!(matrix.element(1, 1), Some(false));
        assert_eq!(matrix.element(1, 2), Some(true));
    }

    #[test]
    fn sm_to_node_rebuilds_sop_rows_in_column_order()
    {
        let source = node(
            &["a", "b", "c"],
            vec![
                vec![Literal::One, Literal::DontCare, Literal::Zero],
                vec![Literal::DontCare, Literal::Zero, Literal::One],
            ],
        );
        let matrix = dec_node_to_sm(&source).unwrap();

        let rebuilt = dec_sm_to_node(&matrix).unwrap();

        assert_eq!(rebuilt, source);
    }

    #[test]
    fn empty_cubes_round_trip_as_tautology_rows()
    {
        let source = node(&["a"], vec![vec![Literal::DontCare]]);
        let matrix = dec_node_to_sm(&source).unwrap();

        let rebuilt = dec_sm_to_node(&matrix).unwrap();

        assert_eq!(matrix.rows_size(), 1);
        assert_eq!(matrix.cols_size(), 0);
        assert!(matrix.row_exists(0));
        assert_eq!(
            rebuilt,
            LogicNode {
                fanins: Vec::<String>::new(),
                cubes: vec![Vec::new()],
            }
        );
    }

    #[test]
    fn print_matches_legacy_matrix_shape()
    {
        let source = node(
            &["a", "b"],
            vec![
                vec![Literal::Zero, Literal::One],
                vec![Literal::DontCare, Literal::Zero],
            ],
        );
        let matrix = dec_node_to_sm(&source).unwrap();

        assert_eq!(dec_sm_print(&matrix), "  0  1\n  .  0\n  a  b\n");
    }

    #[test]
    fn malformed_cube_width_reports_an_error()
    {
        let err = LogicNode::new(vec!["a"], vec![vec![Literal::One, Literal::Zero]])
            .expect_err("cube width must be validated");

        assert_eq!(
            err,
            DecompositionSmError::CubeWidthMismatch {
                cube_index: 0,
                expected: 1,
                actual: 2,
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port()
    {
        let source = include_str!("dec_sm.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
