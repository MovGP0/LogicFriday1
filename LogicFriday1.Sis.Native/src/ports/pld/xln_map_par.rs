//! Native Rust model for `LogicSynthesis/sis/pld/xln_map_par.c`.
//!
//! The C file mixes three concerns: SIS network/maxflow construction,
//! binate-cover matrix selection, and partial-collapse rewriting. This port
//! keeps the deterministic cover heuristics and small partition-planning model
//! native. Direct mutation of SIS `network_t`, `node_t`, maxflow graphs, and
//! sparse-matrix backends is represented by explicit dependency errors.

use std::collections::{BTreeMap, BTreeSet};
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
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        reason: "xln_map_par.c stores sink, separating-set, and solution lists in array_t",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.278",
        source_file: "LogicSynthesis/sis/maxflow/cutset.c",
        reason: "generate_matching extracts cutsets from the maxflow graph",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.279",
        source_file: "LogicSynthesis/sis/maxflow/maxflow.c",
        reason: "generate_matching and squeeze_matching depend on maxflow computation",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.280",
        source_file: "LogicSynthesis/sis/maxflow/mf_input.c",
        reason: "construct_maxflow_network creates maxflow nodes and edges",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.302",
        source_file: "LogicSynthesis/sis/network/netchk.c",
        reason: "partition_network validates the rewritten SIS network",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "partition_network iterates, sweeps, and deletes SIS network nodes",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.309",
        source_file: "LogicSynthesis/sis/node/collapse.c",
        reason: "partial_collapse_node collapses fanins until the separating set is reached",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "fanin/fanout traversal drives maxflow graph and covering rows",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.317",
        source_file: "LogicSynthesis/sis/node/names.c",
        reason: "maxflow graph names are derived from node_long_name",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node type checks, duplication, replacement, and freeing are required",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.320",
        source_file: "LogicSynthesis/sis/node/nodeindex.c",
        reason: "separating-set and binate rows use nodeindex lookups",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.456",
        source_file: "LogicSynthesis/sis/sparse/cols.c",
        reason: "binate cover heuristics scan and delete sparse columns",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.457",
        source_file: "LogicSynthesis/sis/sparse/matrix.c",
        reason: "cover construction uses sm_matrix insert, duplicate, copy, and deletion",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.458",
        source_file: "LogicSynthesis/sis/sparse/rows.c",
        reason: "cover solutions are sparse rows",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "the nonbasic heuristic tracks rows covered by odd columns in st_table",
    },
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartitionNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
}

impl PartitionNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartitionNetwork {
    nodes: Vec<PartitionNode>,
}

impl PartitionNetwork {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: PartitionNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&PartitionNode, XlnMapParError> {
        self.nodes.get(id.0).ok_or(XlnMapParError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut PartitionNode, XlnMapParError> {
        self.nodes
            .get_mut(id.0)
            .ok_or(XlnMapParError::UnknownNode(id))
    }

    pub fn fanouts(&self, node: NodeId) -> Result<Vec<NodeId>, XlnMapParError> {
        self.node(node)?;
        Ok(self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                candidate.fanins.contains(&node).then_some(NodeId(index))
            })
            .collect())
    }

    pub fn node_names<'a>(&'a self, nodes: &[NodeId]) -> Result<Vec<&'a str>, XlnMapParError> {
        nodes
            .iter()
            .map(|node| self.node(*node).map(|node| node.name.as_str()))
            .collect()
    }
}

impl Default for PartitionNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Matching {
    pub sink: NodeId,
    pub separating_set: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinateCoverModel {
    pub matrix: SparseMatrix,
    pub weights: Vec<i32>,
    pub basic_num_rows: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseMatrix {
    rows: BTreeMap<usize, BTreeSet<usize>>,
}

impl SparseMatrix {
    pub fn new() -> Self {
        Self {
            rows: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, row: usize, col: usize) -> bool {
        self.rows.entry(row).or_default().insert(col)
    }

    pub fn contains(&self, row: usize, col: usize) -> bool {
        self.rows
            .get(&row)
            .is_some_and(|columns| columns.contains(&col))
    }

    pub fn row(&self, row: usize) -> Option<&BTreeSet<usize>> {
        self.rows.get(&row)
    }

    pub fn rows(&self) -> &BTreeMap<usize, BTreeSet<usize>> {
        &self.rows
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.column_numbers().len()
    }

    pub fn column_numbers(&self) -> BTreeSet<usize> {
        self.rows
            .values()
            .flat_map(|columns| columns.iter().copied())
            .collect()
    }

    pub fn column_length(&self, col: usize) -> usize {
        self.rows
            .values()
            .filter(|columns| columns.contains(&col))
            .count()
    }

    pub fn column_rows(&self, col: usize) -> BTreeSet<usize> {
        self.rows
            .iter()
            .filter_map(|(row, columns)| columns.contains(&col).then_some(*row))
            .collect()
    }

    pub fn delete_col(&mut self, col: usize) {
        let mut empty_rows = Vec::new();
        for (row, columns) in &mut self.rows {
            columns.remove(&col);
            if columns.is_empty() {
                empty_rows.push(*row);
            }
        }
        for row in empty_rows {
            self.rows.remove(&row);
        }
    }

    pub fn delete_row(&mut self, row: usize) {
        self.rows.remove(&row);
    }

    pub fn copy_rows_by<F>(&self, predicate: F) -> Self
    where
        F: Fn(usize) -> bool,
    {
        Self {
            rows: self
                .rows
                .iter()
                .filter_map(|(row, columns)| predicate(*row).then_some((*row, columns.clone())))
                .collect(),
        }
    }

    pub fn delete_odd_columns(&mut self) {
        let odd_columns: Vec<usize> = self
            .column_numbers()
            .into_iter()
            .filter(|col| col % 2 == 1)
            .collect();
        for col in odd_columns {
            self.delete_col(col);
        }
    }

    pub fn delete_cols_with_complements(&mut self, columns: &BTreeSet<usize>) {
        for col in columns {
            self.delete_col(*col);
            self.delete_col(*col + 1);
        }
    }

    pub fn delete_rows_intersecting(&mut self, columns: &BTreeSet<usize>) {
        let rows: Vec<usize> = self
            .rows
            .iter()
            .filter_map(|(row, row_columns)| {
                row_columns
                    .iter()
                    .any(|col| columns.contains(col))
                    .then_some(*row)
            })
            .collect();
        for row in rows {
            self.delete_row(row);
        }
    }
}

impl Default for SparseMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XlnMapParOperation {
    ConstructMaxflowNetwork,
    GenerateMatching,
    PartialCollapse,
    PartitionNetwork,
    ExactBinateCover,
    BasicUnateCover,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnMapParError {
    UnknownNode(NodeId),
    EmptyEvenColumns,
    DuplicateOddColumnRow {
        row: usize,
    },
    MissingOddColumnRow {
        row: usize,
        col: usize,
    },
    MissingNativePorts {
        operation: XlnMapParOperation,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for XlnMapParError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_map_par node {:?}", node),
            Self::EmptyEvenColumns => write!(f, "no valid even binate-cover columns remain"),
            Self::DuplicateOddColumnRow { row } => {
                write!(f, "row {row} is covered by more than one odd column")
            }
            Self::MissingOddColumnRow { row, col } => {
                write!(
                    f,
                    "odd column {col} expected row {row} in the uncovered-row table"
                )
            }
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => {
                write!(
                    f,
                    "{operation:?} requires native Rust ports for SIS dependencies: "
                )?;
                for (index, dependency) in dependencies.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} ({})", dependency.bead_id, dependency.source_file)?;
                }
                Ok(())
            }
        }
    }
}

impl Error for XlnMapParError {}

pub type XlnMapParResult<T> = Result<T, XlnMapParError>;

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn construct_maxflow_network_blocked() -> XlnMapParResult<()> {
    Err(missing(XlnMapParOperation::ConstructMaxflowNetwork))
}

pub fn generate_matching_blocked() -> XlnMapParResult<Vec<Matching>> {
    Err(missing(XlnMapParOperation::GenerateMatching))
}

pub fn partition_sis_network_blocked() -> XlnMapParResult<()> {
    Err(missing(XlnMapParOperation::PartitionNetwork))
}

pub fn partial_collapse_sis_network_blocked() -> XlnMapParResult<()> {
    Err(missing(XlnMapParOperation::PartialCollapse))
}

fn missing(operation: XlnMapParOperation) -> XlnMapParError {
    XlnMapParError::MissingNativePorts {
        operation,
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    }
}

pub fn dfs_covered_nodes(
    network: &PartitionNetwork,
    node: NodeId,
    separating_set: &[NodeId],
) -> XlnMapParResult<Vec<NodeId>> {
    let mut visited: BTreeSet<NodeId> = separating_set.iter().copied().collect();
    let mut covered = Vec::new();
    dfs_covered_nodes_inner(network, node, &mut visited, &mut covered)?;
    Ok(covered)
}

fn dfs_covered_nodes_inner(
    network: &PartitionNetwork,
    node: NodeId,
    visited: &mut BTreeSet<NodeId>,
    covered: &mut Vec<NodeId>,
) -> XlnMapParResult<()> {
    network.node(node)?;
    if !visited.insert(node) {
        return Ok(());
    }
    covered.push(node);
    for fanin in network.node(node)?.fanins.iter().copied() {
        dfs_covered_nodes_inner(network, fanin, visited, covered)?;
    }
    Ok(())
}

pub fn form_binate_matrix(
    network: &PartitionNetwork,
    matches: &[Matching],
) -> XlnMapParResult<BinateCoverModel> {
    let mut matrix = SparseMatrix::new();
    let mut row_index = network
        .nodes
        .iter()
        .filter(|node| node.kind != NodeKind::PrimaryOutput)
        .count()
        .saturating_sub(1);

    for po in network
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::PrimaryOutput)
    {
        let Some(&sink) = po.fanins.first() else {
            continue;
        };
        if network.node(sink)?.kind == NodeKind::Internal && network.node(sink)?.fanins.is_empty() {
            continue;
        }
        row_index += 1;
        for (match_index, matching) in matches.iter().enumerate() {
            if matching.sink == sink {
                matrix.insert(row_index, 2 * match_index);
            }
        }
    }

    let basic_num_rows = row_index + 1;
    let mut weights = vec![0; 2 * matches.len()];

    for (match_index, matching) in matches.iter().enumerate() {
        let covered = dfs_covered_nodes(network, matching.sink, &matching.separating_set)?;
        for node in covered {
            let node_ref = network.node(node)?;
            if node_ref.kind == NodeKind::Internal && node_ref.fanins.is_empty() {
                continue;
            }
            matrix.insert(node.0, 2 * match_index);
        }

        for input in &matching.separating_set {
            if network.node(*input)?.kind == NodeKind::PrimaryInput {
                continue;
            }
            row_index += 1;
            for (other_index, other) in matches.iter().enumerate() {
                if other.sink == *input {
                    matrix.insert(row_index, 2 * other_index);
                }
            }
            matrix.insert(row_index, 2 * match_index + 1);
        }

        weights[2 * match_index] = 1;
        weights[2 * match_index + 1] = 0;
    }

    Ok(BinateCoverModel {
        matrix,
        weights,
        basic_num_rows,
    })
}

pub fn xln_seq_check_row_elements_even(row: &BTreeSet<usize>) -> bool {
    row.iter().all(|col| col % 2 == 0)
}

pub fn get_column_with_max_elements(matrix: &SparseMatrix) -> XlnMapParResult<usize> {
    let mut best_col = None;
    let mut max_elements = 0;

    for col in matrix.column_numbers() {
        if col % 2 == 1 {
            continue;
        }
        let length = matrix.column_length(col);
        if length < max_elements {
            continue;
        }
        best_col = Some(col);
        max_elements = length;
    }

    best_col.ok_or(XlnMapParError::EmptyEvenColumns)
}

pub fn select_column_and_update_matrix(matrix: &mut SparseMatrix) -> XlnMapParResult<Vec<usize>> {
    let mut solution = Vec::new();

    while matrix.row_count() != 0 && matrix.col_count() != 0 {
        let best_col = get_column_with_max_elements(matrix)?;
        solution.push(best_col);

        let columns = matrix.column_numbers();
        let delete_complement = columns
            .range((best_col + 1)..)
            .next()
            .is_some_and(|next_col| *next_col == best_col + 1);
        let covered_rows: Vec<usize> = matrix.column_rows(best_col).into_iter().collect();

        matrix.delete_col(best_col);
        if delete_complement {
            matrix.delete_col(best_col + 1);
        }
        for row in covered_rows {
            matrix.delete_row(row);
        }
    }

    Ok(solution)
}

pub fn sm_generate_row(solution_array: &[usize]) -> BTreeSet<usize> {
    solution_array.iter().copied().collect()
}

pub fn sm_mat_bin_minimum_cover_greedy(
    matrix: &SparseMatrix,
    bincov_heuristics: i32,
) -> XlnMapParResult<BTreeSet<usize>> {
    if bincov_heuristics <= 1 || (bincov_heuristics == 3 && matrix.col_count() <= 40) {
        return Err(missing(XlnMapParOperation::ExactBinateCover));
    }

    let mut matrix_dup = matrix.clone();
    let solution_array = select_column_and_update_matrix(&mut matrix_dup)?;
    Ok(sm_generate_row(&solution_array))
}

pub fn rows_covered_by_odd_columns(matrix: &SparseMatrix) -> XlnMapParResult<BTreeSet<usize>> {
    let mut rows = BTreeSet::new();
    for col in matrix
        .column_numbers()
        .into_iter()
        .filter(|col| col % 2 == 1)
    {
        for row in matrix.column_rows(col) {
            if !rows.insert(row) {
                return Err(XlnMapParError::DuplicateOddColumnRow { row });
            }
        }
    }
    Ok(rows)
}

pub fn get_even_column_with_max_score(matrix: &SparseMatrix) -> Option<usize> {
    let columns = matrix.column_numbers();
    let mut best_col = None;
    let mut max_score = isize::MIN;

    for col in columns.iter().copied() {
        if col % 2 == 1 {
            continue;
        }
        let score = if columns.contains(&(col + 1)) {
            matrix.column_length(col) as isize - matrix.column_length(col + 1) as isize
        } else {
            matrix.column_length(col) as isize
        };
        if score < max_score {
            continue;
        }
        best_col = Some(col);
        max_score = score;
    }

    best_col
}

pub fn sm_mat_bin_minimum_cover_nonbasic(
    matrix: &mut SparseMatrix,
    row_solution: &mut BTreeSet<usize>,
    odd_covered_rows: &mut BTreeSet<usize>,
) -> XlnMapParResult<()> {
    if matrix.row_count() == odd_covered_rows.len() {
        for col in matrix
            .column_numbers()
            .into_iter()
            .filter(|col| col % 2 == 1)
        {
            row_solution.insert(col);
        }
        return Ok(());
    }

    let Some(best_col) = get_even_column_with_max_score(matrix) else {
        return Ok(());
    };
    row_solution.insert(best_col);

    if matrix.column_numbers().contains(&(best_col + 1)) {
        for row in matrix.column_rows(best_col + 1) {
            if !odd_covered_rows.remove(&row) {
                return Err(XlnMapParError::MissingOddColumnRow {
                    row,
                    col: best_col + 1,
                });
            }
        }
    }

    let rows_to_delete: Vec<usize> = matrix.column_rows(best_col).into_iter().collect();
    for row in rows_to_delete {
        matrix.delete_row(row);
    }
    matrix.delete_col(best_col);
    matrix.delete_col(best_col + 1);

    sm_mat_bin_minimum_cover_nonbasic(matrix, row_solution, odd_covered_rows)
}

pub fn sm_mat_bin_minimum_cover_my_with_basic_solution(
    matrix: &SparseMatrix,
    basic_num_rows: usize,
    row_solution_basic: BTreeSet<usize>,
) -> XlnMapParResult<BTreeSet<usize>> {
    let mut matrix_nonbasic = matrix.copy_rows_by(|row| row >= basic_num_rows);
    matrix_nonbasic.delete_rows_intersecting(&row_solution_basic);
    matrix_nonbasic.delete_cols_with_complements(&row_solution_basic);

    let mut table = rows_covered_by_odd_columns(&matrix_nonbasic)?;
    let mut row_solution_nonbasic = BTreeSet::new();
    sm_mat_bin_minimum_cover_nonbasic(
        &mut matrix_nonbasic,
        &mut row_solution_nonbasic,
        &mut table,
    )?;

    let mut combined = row_solution_basic;
    combined.extend(row_solution_nonbasic);
    Ok(combined)
}

pub fn sm_mat_bin_minimum_cover_my(
    matrix: &SparseMatrix,
    basic_num_rows: usize,
) -> XlnMapParResult<BTreeSet<usize>> {
    let mut matrix_basic = matrix.copy_rows_by(|row| row < basic_num_rows);
    matrix_basic.delete_odd_columns();
    if matrix_basic.row_count() != 0 {
        return Err(missing(XlnMapParOperation::BasicUnateCover));
    }
    sm_mat_bin_minimum_cover_my_with_basic_solution(matrix, basic_num_rows, BTreeSet::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[usize]) -> BTreeSet<usize> {
        values.iter().copied().collect()
    }

    #[test]
    fn dfs_covered_nodes_stops_at_separating_set() {
        let mut network = PartitionNetwork::new();
        let a = network.add_node(PartitionNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(PartitionNode::new("b", NodeKind::PrimaryInput));
        let x =
            network.add_node(PartitionNode::new("x", NodeKind::Internal).with_fanins(vec![a, b]));
        let y =
            network.add_node(PartitionNode::new("y", NodeKind::Internal).with_fanins(vec![x, b]));

        let covered = dfs_covered_nodes(&network, y, &[x]).unwrap();

        assert_eq!(network.node_names(&covered).unwrap(), vec!["y", "b"]);
    }

    #[test]
    fn form_binate_matrix_adds_po_cover_rows_covered_nodes_and_input_obligations() {
        let mut network = PartitionNetwork::new();
        let a = network.add_node(PartitionNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(PartitionNode::new("b", NodeKind::PrimaryInput));
        let x =
            network.add_node(PartitionNode::new("x", NodeKind::Internal).with_fanins(vec![a, b]));
        let y =
            network.add_node(PartitionNode::new("y", NodeKind::Internal).with_fanins(vec![x, b]));
        network.add_node(PartitionNode::new("out", NodeKind::PrimaryOutput).with_fanins(vec![y]));

        let model = form_binate_matrix(
            &network,
            &[
                Matching {
                    sink: x,
                    separating_set: vec![a, b],
                },
                Matching {
                    sink: y,
                    separating_set: vec![x, b],
                },
            ],
        )
        .unwrap();

        assert!(model.matrix.contains(y.0, 2));
        assert!(model.matrix.contains(4, 2));
        assert!(model.matrix.contains(5, 0));
        assert!(model.matrix.contains(5, 3));
        assert_eq!(model.weights, vec![1, 0, 1, 0]);
        assert_eq!(model.basic_num_rows, 5);
    }

    #[test]
    fn greedy_selects_largest_even_columns_and_deletes_complements() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 0);
        matrix.insert(1, 0);
        matrix.insert(2, 2);
        matrix.insert(3, 2);
        matrix.insert(4, 4);
        matrix.insert(4, 5);

        let solution = sm_mat_bin_minimum_cover_greedy(&matrix, 2).unwrap();

        assert_eq!(solution, set(&[0, 2, 4]));
    }

    #[test]
    fn exact_binate_cover_modes_report_dependency_beads_and_sources() {
        let matrix = SparseMatrix::new();
        let Err(XlnMapParError::MissingNativePorts {
            operation,
            dependencies,
        }) = sm_mat_bin_minimum_cover_greedy(&matrix, 1)
        else {
            panic!("expected exact-cover dependency error");
        };

        assert_eq!(operation, XlnMapParOperation::ExactBinateCover);
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.457"
                && dependency.source_file == "LogicSynthesis/sis/sparse/matrix.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.458"
                && dependency.source_file == "LogicSynthesis/sis/sparse/rows.c"
        }));
    }

    #[test]
    fn nonbasic_cover_inserts_all_odd_columns_when_they_cover_every_row() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(10, 1);
        matrix.insert(11, 3);
        let mut row_solution = BTreeSet::new();
        let mut table = rows_covered_by_odd_columns(&matrix).unwrap();

        sm_mat_bin_minimum_cover_nonbasic(&mut matrix, &mut row_solution, &mut table).unwrap();

        assert_eq!(row_solution, set(&[1, 3]));
    }

    #[test]
    fn nonbasic_cover_prefers_even_column_score_and_removes_odd_table_rows() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(10, 0);
        matrix.insert(11, 0);
        matrix.insert(11, 1);
        matrix.insert(12, 2);
        matrix.insert(13, 3);
        let mut row_solution = BTreeSet::new();
        let mut table = rows_covered_by_odd_columns(&matrix).unwrap();

        sm_mat_bin_minimum_cover_nonbasic(&mut matrix, &mut row_solution, &mut table).unwrap();

        assert_eq!(row_solution, set(&[0, 2]));
    }

    #[test]
    fn my_cover_combines_supplied_basic_solution_with_nonbasic_solution() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(0, 0);
        matrix.insert(1, 2);
        matrix.insert(3, 2);
        matrix.insert(4, 5);

        let solution =
            sm_mat_bin_minimum_cover_my_with_basic_solution(&matrix, 2, set(&[0])).unwrap();

        assert_eq!(solution, set(&[0, 2, 5]));
    }

    #[test]
    fn row_even_check_matches_c_helper() {
        assert!(xln_seq_check_row_elements_even(&set(&[0, 2, 8])));
        assert!(!xln_seq_check_row_elements_even(&set(&[0, 3, 8])));
    }

    #[test]
    fn sis_bound_entries_report_maxflow_network_node_and_sparse_blockers() {
        let error = partition_sis_network_blocked().unwrap_err();
        let XlnMapParError::MissingNativePorts {
            operation,
            dependencies,
        } = error
        else {
            panic!("expected dependency error");
        };

        assert_eq!(operation, XlnMapParOperation::PartitionNetwork);
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.279"
                && dependency.source_file == "LogicSynthesis/sis/maxflow/maxflow.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.320"
                && dependency.source_file == "LogicSynthesis/sis/node/nodeindex.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.302"
                && dependency.source_file == "LogicSynthesis/sis/network/netchk.c"
        }));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_map_par.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
