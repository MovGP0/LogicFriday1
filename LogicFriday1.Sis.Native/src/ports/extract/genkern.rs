//! Native Rust kernel generation for algebraic extraction.
//!
//! The legacy implementation in `sis/extract/genkern.c` wrapped rectangle
//! enumeration with conversions between Boolean nodes and sparse matrix
//! kernels. This port keeps the same behavior in owned Rust data and avoids
//! legacy C ABI entry points.

use crate::ports::extract::rect::{Rectangle, generate_all_rectangles_with};
use crate::ports::node::node::{
    Cover, Cube, Node, NodeError, NodeFunction, node_constant, node_function,
};
use crate::ports::sparse::matrix::SparseMatrix;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub type KernelGenerationResult<T> = Result<T, KernelGenerationError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedKernel {
    pub kernel: Node,
    pub cokernel: Node,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedKernelCubeTable {
    fanins: Vec<String>,
    matrix: SparseMatrix,
    kernel_cubes: BTreeMap<usize, Vec<usize>>,
    co_kernels: BTreeMap<usize, Vec<usize>>,
}

impl ExtractedKernelCubeTable {
    pub fn new(fanins: impl Into<Vec<String>>) -> Self {
        Self {
            fanins: fanins.into(),
            matrix: SparseMatrix::new(),
            kernel_cubes: BTreeMap::new(),
            co_kernels: BTreeMap::new(),
        }
    }

    pub fn fanins(&self) -> &[String] {
        &self.fanins
    }

    pub fn matrix(&self) -> &SparseMatrix {
        &self.matrix
    }

    pub fn insert_entry(&mut self, row: usize, col: usize) {
        self.matrix.insert(row, col);
    }

    pub fn insert_kernel_cube(&mut self, col: usize, literals: impl IntoIterator<Item = usize>) {
        self.kernel_cubes
            .insert(col, sorted_unique_literals(literals));
    }

    pub fn insert_co_kernel(&mut self, row: usize, literals: impl IntoIterator<Item = usize>) {
        self.co_kernels
            .insert(row, sorted_unique_literals(literals));
    }
}

pub trait SubkernelExtractor {
    fn extract(
        &mut self,
        node: &Node,
        level: i32,
    ) -> KernelGenerationResult<ExtractedKernelCubeTable>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KernelGenerationError {
    Node(NodeError),
    UnsupportedNodeFunction(NodeFunction),
    MissingFunction,
    ConflictingLiteral { cube: usize, variable: usize },
    LiteralIndexOutOfRange { literal: usize, fanin_count: usize },
    MissingKernelCube { col: usize },
    MissingCoKernel { row: usize },
}

impl fmt::Display for KernelGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node(error) => write!(f, "{error}"),
            Self::UnsupportedNodeFunction(function) => {
                write!(f, "kernel generation does not support {function:?} nodes")
            }
            Self::MissingFunction => write!(f, "kernel generation requires a node function"),
            Self::ConflictingLiteral { cube, variable } => {
                write!(f, "cube {cube} contains both phases of fanin {variable}")
            }
            Self::LiteralIndexOutOfRange {
                literal,
                fanin_count,
            } => write!(
                f,
                "literal index {literal} is outside the {} available fanins",
                fanin_count
            ),
            Self::MissingKernelCube { col } => {
                write!(f, "subkernel table is missing kernel cube column {col}")
            }
            Self::MissingCoKernel { row } => {
                write!(f, "subkernel table is missing co-kernel row {row}")
            }
        }
    }
}

impl Error for KernelGenerationError {}

impl From<NodeError> for KernelGenerationError {
    fn from(value: NodeError) -> Self {
        Self::Node(value)
    }
}

pub fn generate_kernels(
    node: &Node,
    mut visit: impl FnMut(GeneratedKernel) -> KernelGenerationResult<bool>,
) -> KernelGenerationResult<()> {
    let node_matrix = node_to_sparse_matrix(node)?;
    let fanins = node.fanins.clone();
    let mut result = Ok(());

    generate_all_rectangles_with(
        &node_matrix,
        |co_rect, rectangle| match kernel_from_rectangle(co_rect, rectangle, &fanins)
            .and_then(|kernel| visit(kernel))
        {
            Ok(expand) => expand,
            Err(error) => {
                result = Err(error);
                false
            }
        },
    );

    result
}

pub fn collect_kernels(node: &Node) -> KernelGenerationResult<Vec<GeneratedKernel>> {
    let mut kernels = Vec::new();
    generate_kernels(node, |kernel| {
        kernels.push(kernel);
        Ok(true)
    })?;

    Ok(kernels)
}

pub fn generate_subkernels(
    node: &Node,
    level: i32,
    extractor: &mut impl SubkernelExtractor,
    visit: impl FnMut(GeneratedKernel) -> KernelGenerationResult<bool>,
) -> KernelGenerationResult<()> {
    let table = extractor.extract(node, level)?;

    generate_subkernels_from_table(&table, visit)
}

pub fn generate_subkernels_from_table(
    table: &ExtractedKernelCubeTable,
    mut visit: impl FnMut(GeneratedKernel) -> KernelGenerationResult<bool>,
) -> KernelGenerationResult<()> {
    let mut result = Ok(());

    generate_all_rectangles_with(table.matrix(), |_, rectangle| {
        if rectangle.cols().is_empty() || rectangle.rows().is_empty() {
            return true;
        }

        match subkernel_from_rectangle(table, rectangle).and_then(|kernel| visit(kernel)) {
            Ok(expand) => expand,
            Err(error) => {
                result = Err(error);
                false
            }
        }
    });

    result
}

pub fn collect_subkernels_from_table(
    table: &ExtractedKernelCubeTable,
) -> KernelGenerationResult<Vec<GeneratedKernel>> {
    let mut kernels = Vec::new();
    generate_subkernels_from_table(table, |kernel| {
        kernels.push(kernel);
        Ok(true)
    })?;

    Ok(kernels)
}

pub fn node_to_sparse_matrix(node: &Node) -> KernelGenerationResult<SparseMatrix> {
    match node_function(node)? {
        NodeFunction::Zero
        | NodeFunction::One
        | NodeFunction::Buffer
        | NodeFunction::Inverter
        | NodeFunction::And
        | NodeFunction::Or
        | NodeFunction::Complex => {}
        function => return Err(KernelGenerationError::UnsupportedNodeFunction(function)),
    }

    let Some(function) = node.function() else {
        return Err(KernelGenerationError::MissingFunction);
    };

    let mut matrix = SparseMatrix::new();
    for (row, cube) in function.cubes().iter().enumerate() {
        for (input, phase) in cube.inputs().iter().enumerate() {
            let Some(phase) = phase else {
                continue;
            };

            let literal = input * 2 + usize::from(!*phase);
            matrix.insert(row, literal);
        }
    }

    Ok(matrix)
}

pub fn sparse_matrix_to_node(
    matrix: &SparseMatrix,
    fanins: &[String],
) -> KernelGenerationResult<Node> {
    if matrix.is_empty() {
        return Ok(node_constant(0)?);
    }

    node_from_cube_rows(
        fanins,
        matrix.rows().map(|row| row.elements().to_vec()).collect(),
    )
}

fn kernel_from_rectangle(
    co_rect: &SparseMatrix,
    rectangle: &Rectangle,
    fanins: &[String],
) -> KernelGenerationResult<GeneratedKernel> {
    let kernel = sparse_matrix_to_node(co_rect, fanins)?;
    let cokernel = if rectangle.cols().is_empty() {
        node_constant(1)?
    } else {
        node_from_cube_rows(fanins, vec![rectangle.cols().iter().copied().collect()])?
    };

    Ok(GeneratedKernel { kernel, cokernel })
}

fn subkernel_from_rectangle(
    table: &ExtractedKernelCubeTable,
    rectangle: &Rectangle,
) -> KernelGenerationResult<GeneratedKernel> {
    let mut kernel_rows = Vec::new();
    for col in rectangle.cols() {
        let Some(cube) = table.kernel_cubes.get(col) else {
            return Err(KernelGenerationError::MissingKernelCube { col: *col });
        };

        kernel_rows.push(cube.clone());
    }

    let mut cokernel_rows = Vec::new();
    for row in rectangle.rows() {
        let Some(cube) = table.co_kernels.get(row) else {
            return Err(KernelGenerationError::MissingCoKernel { row: *row });
        };

        cokernel_rows.push(cube.clone());
    }

    Ok(GeneratedKernel {
        kernel: node_from_cube_rows(table.fanins(), kernel_rows)?,
        cokernel: node_from_cube_rows(table.fanins(), cokernel_rows)?,
    })
}

fn node_from_cube_rows(fanins: &[String], rows: Vec<Vec<usize>>) -> KernelGenerationResult<Node> {
    if rows.is_empty() {
        return Ok(node_constant(0)?);
    }

    if rows.len() == 1 && rows[0].is_empty() {
        return Ok(node_constant(1)?);
    }

    let mut cubes = Vec::new();
    for (cube_index, row) in rows.iter().enumerate() {
        let mut inputs = vec![None; fanins.len()];
        for literal in row {
            let variable = literal / 2;
            if variable >= fanins.len() {
                return Err(KernelGenerationError::LiteralIndexOutOfRange {
                    literal: *literal,
                    fanin_count: fanins.len(),
                });
            }

            let phase = literal % 2 == 0;
            if inputs[variable].is_some_and(|existing| existing != phase) {
                return Err(KernelGenerationError::ConflictingLiteral {
                    cube: cube_index,
                    variable,
                });
            }

            inputs[variable] = Some(phase);
        }

        cubes.push(Cube::new(inputs));
    }

    Ok(Node::new(Cover::new(fanins.len(), cubes)?, fanins.to_vec()))
}

fn sorted_unique_literals(literals: impl IntoIterator<Item = usize>) -> Vec<usize> {
    literals
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::node::node::{node_and, node_contains, node_equal, node_literal, node_or};

    fn lit(name: &str, phase: i32) -> Node {
        node_literal(name, phase).unwrap()
    }

    fn assert_equal(left: &Node, right: &Node) {
        assert!(node_equal(left, right).unwrap());
    }

    #[test]
    fn converts_node_cover_to_sparse_literal_matrix() {
        let ab = node_and(&lit("a", 1), &lit("b", 1)).unwrap();
        let ac = node_and(&lit("a", 1), &lit("c", 0)).unwrap();
        let node = node_or(&ab, &ac).unwrap();

        let matrix = node_to_sparse_matrix(&node).unwrap();

        assert_eq!(
            matrix
                .elements()
                .map(|element| (element.row, element.col))
                .collect::<Vec<_>>(),
            vec![(0, 0), (0, 2), (1, 0), (1, 5)]
        );
    }

    #[test]
    fn converts_sparse_literal_matrix_back_to_node() {
        let fanins = vec!["a".to_owned(), "b".to_owned()];
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 0);
        matrix.insert(0, 2);
        matrix.insert(1, 1);
        matrix.insert(1, 2);

        let node = sparse_matrix_to_node(&matrix, &fanins).unwrap();
        let ab = node_and(&lit("a", 1), &lit("b", 1)).unwrap();
        let anb = node_and(&lit("a", 0), &lit("b", 1)).unwrap();
        let expected = node_or(&ab, &anb).unwrap();

        assert_equal(&node, &expected);
    }

    #[test]
    fn generates_kernel_and_cokernel_pairs_from_rectangles() {
        let ab = node_and(&lit("a", 1), &lit("b", 1)).unwrap();
        let ac = node_and(&lit("a", 1), &lit("c", 1)).unwrap();
        let node = node_or(&ab, &ac).unwrap();

        let generated = collect_kernels(&node).unwrap();

        assert_eq!(generated.len(), 1);
        assert_equal(
            &generated[0].kernel,
            &node_or(&lit("b", 1), &lit("c", 1)).unwrap(),
        );
        assert_equal(&generated[0].cokernel, &lit("a", 1));
    }

    #[test]
    fn uses_constant_one_for_empty_cokernel_rectangle() {
        let ab = node_and(&lit("a", 1), &lit("b", 1)).unwrap();
        let cd = node_and(&lit("c", 1), &lit("d", 1)).unwrap();
        let node = node_or(&ab, &cd).unwrap();

        let generated = collect_kernels(&node).unwrap();

        assert!(generated.iter().any(|kernel| {
            node_equal(&kernel.kernel, &node).unwrap()
                && node_equal(&kernel.cokernel, &node_constant(1).unwrap()).unwrap()
        }));
    }

    #[test]
    fn single_literal_has_no_generated_kernel() {
        let generated = collect_kernels(&lit("a", 1)).unwrap();

        assert!(generated.is_empty());
    }

    #[test]
    fn callback_result_controls_recursive_kernel_expansion() {
        let ab = node_and(&lit("a", 1), &lit("b", 1)).unwrap();
        let ac = node_and(&lit("a", 1), &lit("c", 1)).unwrap();
        let ad = node_and(&lit("a", 1), &lit("d", 1)).unwrap();
        let node = node_or(&node_or(&ab, &ac).unwrap(), &ad).unwrap();
        let mut visited = 0;

        generate_kernels(&node, |_| {
            visited += 1;
            Ok(false)
        })
        .unwrap();

        assert_eq!(visited, 1);
    }

    #[test]
    fn generates_subkernels_from_extracted_kernel_cube_table() {
        let mut table = ExtractedKernelCubeTable::new(vec!["a".to_owned(), "b".to_owned()]);
        table.insert_co_kernel(0, [0]);
        table.insert_co_kernel(1, [2]);
        table.insert_kernel_cube(0, [0, 2]);
        table.insert_kernel_cube(1, [0]);
        table.insert_entry(0, 0);
        table.insert_entry(0, 1);
        table.insert_entry(1, 0);
        table.insert_entry(1, 1);

        let generated = collect_subkernels_from_table(&table).unwrap();

        assert_eq!(generated.len(), 1);
        assert_equal(
            &generated[0].kernel,
            &node_or(&node_and(&lit("a", 1), &lit("b", 1)).unwrap(), &lit("a", 1)).unwrap(),
        );
        assert_equal(
            &generated[0].cokernel,
            &node_or(&lit("a", 1), &lit("b", 1)).unwrap(),
        );
    }

    #[test]
    fn skips_empty_subkernel_rectangles() {
        let mut table = ExtractedKernelCubeTable::new(vec!["a".to_owned(), "b".to_owned()]);
        table.insert_entry(0, 0);

        let generated = collect_subkernels_from_table(&table).unwrap();

        assert!(generated.is_empty());
    }

    #[test]
    fn reports_missing_subkernel_cube_metadata() {
        let mut table = ExtractedKernelCubeTable::new(vec!["a".to_owned(), "b".to_owned()]);
        table.insert_co_kernel(0, [0]);
        table.insert_co_kernel(1, [2]);
        table.insert_entry(0, 0);
        table.insert_entry(1, 0);

        let error = collect_subkernels_from_table(&table).unwrap_err();

        assert_eq!(error, KernelGenerationError::MissingKernelCube { col: 0 });
    }

    #[test]
    fn rejects_conflicting_literals_when_building_nodes() {
        let error = node_from_cube_rows(&["a".to_owned()], vec![vec![0, 1]]).unwrap_err();

        assert_eq!(
            error,
            KernelGenerationError::ConflictingLiteral {
                cube: 0,
                variable: 0
            }
        );
    }

    #[test]
    fn generated_kernel_preserves_original_cover_meaning_when_multiplied_by_cokernel() {
        let ab = node_and(&lit("a", 1), &lit("b", 1)).unwrap();
        let ac = node_and(&lit("a", 1), &lit("c", 1)).unwrap();
        let node = node_or(&ab, &ac).unwrap();
        let generated = collect_kernels(&node).unwrap();
        let product = node_and(&generated[0].kernel, &generated[0].cokernel).unwrap();

        assert!(node_contains(&node, &product).unwrap());
        assert!(node_contains(&product, &node).unwrap());
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let source = include_str!("genkern.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }
}
