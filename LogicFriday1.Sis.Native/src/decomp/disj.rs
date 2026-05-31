use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisjointDecompositionError<E>
{
    Backend(E),
    EmptyMatrixNode,
}

impl<E> fmt::Display for DisjointDecompositionError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::Backend(error) => error.fmt(f),
            Self::EmptyMatrixNode => {
                write!(f, "disjoint decomposition produced an empty matrix node")
            }
        }
    }
}

impl<E> Error for DisjointDecompositionError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match self
        {
            Self::Backend(error) => Some(error),
            Self::EmptyMatrixNode => None,
        }
    }
}

pub type DisjointDecompositionResult<T, E> = Result<T, DisjointDecompositionError<E>>;

pub trait DisjointDecompositionBackend
{
    type Node: Clone;
    type Matrix;
    type Error;

    fn node_to_matrix(&mut self, node: &Self::Node) -> Result<Self::Matrix, Self::Error>;

    fn block_partition(
        &mut self,
        matrix: Self::Matrix,
    ) -> Result<BlockPartition<Self::Matrix>, Self::Error>;

    fn matrix_to_node(&mut self, matrix: Self::Matrix) -> Result<Self::Node, Self::Error>;

    fn positive_literal(&mut self, node: &Self::Node) -> Result<Self::Node, Self::Error>;

    fn or(&mut self, left: &Self::Node, right: &Self::Node) -> Result<Self::Node, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlockPartition<M>
{
    Connected(M),
    Split { left: M, right: M },
}

pub fn decompose_disjoint<B>(
    backend: &mut B,
    node: &B::Node,
) -> DisjointDecompositionResult<Vec<B::Node>, B::Error>
where
    B: DisjointDecompositionBackend,
{
    let matrix = backend
        .node_to_matrix(node)
        .map_err(DisjointDecompositionError::Backend)?;

    let BlockPartition::Split { left, right } = backend
        .block_partition(matrix)
        .map_err(DisjointDecompositionError::Backend)?
    else
    {
        return Ok(vec![node.clone()]);
    };

    let mut nodes = Vec::new();
    let first_part = backend
        .matrix_to_node(left)
        .map_err(DisjointDecompositionError::Backend)?;
    let mut root = backend
        .positive_literal(&first_part)
        .map_err(DisjointDecompositionError::Backend)?;
    nodes.push(root.clone());
    nodes.push(first_part);

    let mut rest = right;
    loop
    {
        match backend
            .block_partition(rest)
            .map_err(DisjointDecompositionError::Backend)?
        {
            BlockPartition::Split { left, right } =>
            {
                let part = backend
                    .matrix_to_node(left)
                    .map_err(DisjointDecompositionError::Backend)?;
                let literal = backend
                    .positive_literal(&part)
                    .map_err(DisjointDecompositionError::Backend)?;
                root = backend
                    .or(&root, &literal)
                    .map_err(DisjointDecompositionError::Backend)?;
                nodes.push(part);
                rest = right;
            }
            BlockPartition::Connected(matrix) =>
            {
                let part = backend
                    .matrix_to_node(matrix)
                    .map_err(DisjointDecompositionError::Backend)?;
                let literal = backend
                    .positive_literal(&part)
                    .map_err(DisjointDecompositionError::Backend)?;
                root = backend
                    .or(&root, &literal)
                    .map_err(DisjointDecompositionError::Backend)?;
                nodes[0] = root;
                nodes.push(part);
                return Ok(nodes);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SopMatrix
{
    rows: Vec<Vec<Option<bool>>>,
}

impl SopMatrix
{
    pub fn new(rows: Vec<Vec<Option<bool>>>) -> Self
    {
        Self { rows }
    }

    pub fn rows(&self) -> &[Vec<Option<bool>>]
    {
        &self.rows
    }

    pub fn block_partition(self) -> BlockPartition<Self>
    {
        if self.rows.is_empty()
        {
            return BlockPartition::Connected(self);
        }

        let column_count = self.column_count();
        let mut row_visited = vec![false; self.rows.len()];
        let mut column_visited = vec![false; column_count];
        visit_row(&self.rows, 0, &mut row_visited, &mut column_visited);

        if row_visited.iter().all(|visited| *visited)
        {
            return BlockPartition::Connected(self);
        }

        let mut left = Vec::new();
        let mut right = Vec::new();
        for (index, row) in self.rows.into_iter().enumerate()
        {
            if row_visited[index]
            {
                left.push(row);
            }
            else
            {
                right.push(row);
            }
        }

        BlockPartition::Split {
            left: Self::new(left),
            right: Self::new(right),
        }
    }

    fn column_count(&self) -> usize
    {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }
}

fn visit_row(
    rows: &[Vec<Option<bool>>],
    row_index: usize,
    row_visited: &mut [bool],
    column_visited: &mut [bool],
)
{
    if row_visited[row_index]
    {
        return;
    }

    row_visited[row_index] = true;
    for (column_index, value) in rows[row_index].iter().enumerate()
    {
        if value.is_some()
        {
            visit_column(rows, column_index, row_visited, column_visited);
        }
    }
}

fn visit_column(
    rows: &[Vec<Option<bool>>],
    column_index: usize,
    row_visited: &mut [bool],
    column_visited: &mut [bool],
)
{
    if column_visited[column_index]
    {
        return;
    }

    column_visited[column_index] = true;
    for (row_index, row) in rows.iter().enumerate()
    {
        if row.get(column_index).is_some_and(Option::is_some)
        {
            visit_row(rows, row_index, row_visited, column_visited);
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum TestNode
    {
        Function(SopMatrix),
        Literal(String),
        Or(Box<TestNode>, Box<TestNode>),
    }

    #[derive(Debug, Eq, PartialEq)]
    struct TestError;

    impl fmt::Display for TestError
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
        {
            write!(f, "test backend error")
        }
    }

    impl Error for TestError {}

    #[derive(Default)]
    struct TestBackend
    {
        next_name: usize,
    }

    impl DisjointDecompositionBackend for TestBackend
    {
        type Node = TestNode;
        type Matrix = SopMatrix;
        type Error = TestError;

        fn node_to_matrix(&mut self, node: &Self::Node) -> Result<Self::Matrix, Self::Error>
        {
            match node
            {
                TestNode::Function(matrix) => Ok(matrix.clone()),
                _ => Err(TestError),
            }
        }

        fn block_partition(
            &mut self,
            matrix: Self::Matrix,
        ) -> Result<BlockPartition<Self::Matrix>, Self::Error>
        {
            Ok(matrix.block_partition())
        }

        fn matrix_to_node(&mut self, matrix: Self::Matrix) -> Result<Self::Node, Self::Error>
        {
            let name = format!("p{}", self.next_name);
            self.next_name += 1;
            Ok(TestNode::Literal(format!("{name}:{:?}", matrix.rows())))
        }

        fn positive_literal(&mut self, node: &Self::Node) -> Result<Self::Node, Self::Error>
        {
            Ok(node.clone())
        }

        fn or(&mut self, left: &Self::Node, right: &Self::Node) -> Result<Self::Node, Self::Error>
        {
            Ok(TestNode::Or(Box::new(left.clone()), Box::new(right.clone())))
        }
    }

    fn row(values: &[Option<bool>]) -> Vec<Option<bool>>
    {
        values.to_vec()
    }

    fn labels(nodes: &[TestNode]) -> Vec<String>
    {
        nodes.iter().map(label).collect()
    }

    fn label(node: &TestNode) -> String
    {
        match node
        {
            TestNode::Function(_) => "function".to_owned(),
            TestNode::Literal(label) => label.clone(),
            TestNode::Or(left, right) => format!("({} | {})", label(left), label(right)),
        }
    }

    #[test]
    fn connected_function_returns_original_node()
    {
        let node = TestNode::Function(SopMatrix::new(vec![
            row(&[Some(true), None]),
            row(&[Some(false), Some(true)]),
        ]));
        let mut backend = TestBackend::default();

        let result = decompose_disjoint(&mut backend, &node).unwrap();

        assert_eq!(result, vec![node]);
    }

    #[test]
    fn two_part_partition_replaces_first_slot_with_or_root()
    {
        let node = TestNode::Function(SopMatrix::new(vec![
            row(&[Some(true), None, None]),
            row(&[None, Some(false), None]),
        ]));
        let mut backend = TestBackend::default();

        let result = decompose_disjoint(&mut backend, &node).unwrap();

        assert_eq!(
            labels(&result),
            vec![
                "(p0:[[Some(true), None, None]] | p1:[[None, Some(false), None]])",
                "p0:[[Some(true), None, None]]",
                "p1:[[None, Some(false), None]]",
            ]
        );
    }

    #[test]
    fn repeated_partitions_keep_c_source_order()
    {
        let node = TestNode::Function(SopMatrix::new(vec![
            row(&[Some(true), None, None]),
            row(&[None, Some(false), None]),
            row(&[None, None, Some(true)]),
        ]));
        let mut backend = TestBackend::default();

        let result = decompose_disjoint(&mut backend, &node).unwrap();

        assert_eq!(
            labels(&result),
            vec![
                "((p0:[[Some(true), None, None]] | p1:[[None, Some(false), None]]) | p2:[[None, None, Some(true)]])",
                "p0:[[Some(true), None, None]]",
                "p1:[[None, Some(false), None]]",
                "p2:[[None, None, Some(true)]]",
            ]
        );
    }
}
