//! Native Rust model for `LogicSynthesis/sis/pld/pld_util.c`.
//!
//! The C file is a shared PLD utility layer over SIS `node_t`, `network_t`,
//! `array_t`, `st_table`, and sparse-matrix APIs. This port models the
//! independent Boolean cube, fanin-set, replacement, and sparse deletion
//! behavior with owned Rust data. Entry points that still require direct SIS
//! graph mutation report explicit missing dependency errors with bead IDs and
//! source files.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CubeLiteral {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiteralPhase {
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProductLiteral {
    pub node: NodeId,
    pub phase: LiteralPhase,
}

impl ProductLiteral {
    pub const fn positive(node: NodeId) -> Self {
        Self {
            node,
            phase: LiteralPhase::Positive,
        }
    }

    pub const fn negative(node: NodeId) -> Self {
        Self {
            node,
            phase: LiteralPhase::Negative,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductTerm {
    literals: Vec<ProductLiteral>,
}

impl ProductTerm {
    pub fn new(literals: impl IntoIterator<Item = ProductLiteral>) -> Self {
        let mut literals: Vec<_> = literals.into_iter().collect();
        literals.sort_unstable();
        literals.dedup();
        Self { literals }
    }

    pub fn constant_one() -> Self {
        Self {
            literals: Vec::new(),
        }
    }

    pub fn literals(&self) -> &[ProductLiteral] {
        &self.literals
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PldCube {
    literals: Vec<CubeLiteral>,
}

impl PldCube {
    pub fn new(literals: impl IntoIterator<Item = CubeLiteral>) -> Self {
        Self {
            literals: literals.into_iter().collect(),
        }
    }

    pub fn literals(&self) -> &[CubeLiteral] {
        &self.literals
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PldNode {
    pub name: String,
    pub kind: NodeKind,
    fanins: Vec<NodeId>,
    cubes: Vec<PldCube>,
}

impl PldNode {
    pub fn new(
        name: impl Into<String>,
        kind: NodeKind,
        fanins: Vec<NodeId>,
        cubes: Vec<PldCube>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins,
            cubes,
        }
    }

    pub fn primary_input(name: impl Into<String>) -> Self {
        Self::new(name, NodeKind::PrimaryInput, Vec::new(), Vec::new())
    }

    pub fn internal(name: impl Into<String>, fanins: Vec<NodeId>, cubes: Vec<PldCube>) -> Self {
        Self::new(name, NodeKind::Internal, fanins, cubes)
    }

    pub fn constant_zero() -> Self {
        Self::internal("$zero", Vec::new(), Vec::new())
    }

    pub fn constant_one() -> Self {
        Self::internal("$one", Vec::new(), vec![PldCube::new([])])
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }

    pub fn cubes(&self) -> &[PldCube] {
        &self.cubes
    }

    pub fn fanin_count(&self) -> usize {
        self.fanins.len()
    }

    pub fn function_kind(&self) -> NodeFunction {
        if self.cubes.is_empty() {
            NodeFunction::Zero
        } else if self.fanins.is_empty()
            && self.cubes.len() == 1
            && self.cubes[0].literals.is_empty()
        {
            NodeFunction::One
        } else {
            NodeFunction::Other
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Other,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PldNetwork {
    nodes: Vec<PldNode>,
}

impl PldNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: PldNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> PldUtilResult<&PldNode> {
        self.nodes.get(id.0).ok_or(PldUtilError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> PldUtilResult<&mut PldNode> {
        self.nodes
            .get_mut(id.0)
            .ok_or(PldUtilError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[PldNode] {
        &self.nodes
    }

    pub fn find_node_by_name(&self, name: &str) -> Option<NodeId> {
        self.nodes
            .iter()
            .position(|node| node.name == name)
            .map(NodeId)
    }

    pub fn replace_node_with_array(
        &mut self,
        node: NodeId,
        mut replacements: Vec<PldNode>,
    ) -> PldUtilResult<Vec<NodeId>> {
        if replacements.is_empty() {
            return Err(PldUtilError::EmptyReplacementArray);
        }

        let replacement = replacements.remove(0);
        let mut added = Vec::new();
        for extra in replacements {
            added.push(self.add_node(extra));
        }

        let target = self.node_mut(node)?;
        target.fanins = replacement.fanins;
        target.cubes = replacement.cubes;
        Ok(added)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PldUtilError {
    UnknownNode(NodeId),
    MissingFaninMapping(NodeId),
    MissingPrimaryInput { name: String },
    CubeArityMismatch { fanins: usize, literals: usize },
    EmptyReplacementArray,
    MissingTableNode(NodeId),
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for PldUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown pld_util node {:?}", node),
            Self::MissingFaninMapping(node) => {
                write!(f, "missing remap correspondence for fanin {:?}", node)
            }
            Self::MissingPrimaryInput { name } => {
                write!(
                    f,
                    "replacement primary input {name} is absent from target network"
                )
            }
            Self::CubeArityMismatch { fanins, literals } => write!(
                f,
                "cube literal count {literals} does not match fanin count {fanins}"
            ),
            Self::EmptyReplacementArray => write!(f, "node replacement array is empty"),
            Self::MissingTableNode(node) => write!(f, "node {:?} is absent from table", node),
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} requires native Rust ports for SIS dependencies"
            ),
        }
    }
}

impl Error for PldUtilError {}

pub type PldUtilResult<T> = Result<T, PldUtilError>;

pub fn sis_bound_operation_unavailable(operation: &'static str) -> PldUtilResult<()> {
    Err(missing_native_ports(operation))
}

pub fn remap_init_corr(
    target_network: &PldNetwork,
    replacement_network: &PldNetwork,
) -> PldUtilResult<BTreeMap<NodeId, NodeId>> {
    let mut correspondence = BTreeMap::new();
    for (index, node) in replacement_network.nodes().iter().enumerate() {
        if node.kind != NodeKind::PrimaryInput {
            continue;
        }
        let target = target_network
            .find_node_by_name(&node.name)
            .ok_or_else(|| PldUtilError::MissingPrimaryInput {
                name: node.name.clone(),
            })?;
        correspondence.insert(NodeId(index), target);
    }
    Ok(correspondence)
}

pub fn remap_get_node(
    node: &PldNode,
    correspondence: &BTreeMap<NodeId, NodeId>,
) -> PldUtilResult<Vec<ProductTerm>> {
    let mut products = Vec::new();
    for cube in node.cubes().iter().rev() {
        products.push(make_product_from_cube(node.fanins(), cube, correspondence)?);
    }
    Ok(products)
}

pub fn nodes_from_cubes(node: &PldNode) -> PldUtilResult<Vec<Vec<ProductTerm>>> {
    let identity = identity_mapping(node.fanins());
    node.cubes()
        .iter()
        .rev()
        .map(|cube| make_node_from_cube(node, cube, &identity).map(|term| vec![term]))
        .collect()
}

pub fn make_node_from_cube(
    node: &PldNode,
    cube: &PldCube,
    correspondence: &BTreeMap<NodeId, NodeId>,
) -> PldUtilResult<ProductTerm> {
    make_product_from_cube(node.fanins(), cube, correspondence)
}

pub fn cubes_of_node(node: &PldNode) -> Vec<PldCube> {
    node.cubes().iter().rev().cloned().collect()
}

pub fn get_non_common_fanins(node: &PldNode, common: &[NodeId]) -> Vec<NodeId> {
    node.fanins()
        .iter()
        .copied()
        .filter(|fanin| !is_node_in_array(*fanin, common))
        .collect()
}

pub fn is_node_in_array(node: NodeId, nodes: &[NodeId]) -> bool {
    nodes.contains(&node)
}

pub fn num_fanin_cube(cube: &PldCube, node: &PldNode) -> PldUtilResult<usize> {
    if cube.literals().len() != node.fanin_count() {
        return Err(PldUtilError::CubeArityMismatch {
            fanins: node.fanin_count(),
            literals: cube.literals().len(),
        });
    }

    Ok(cube
        .literals()
        .iter()
        .filter(|literal| matches!(literal, CubeLiteral::One | CubeLiteral::Zero))
        .count())
}

pub fn is_fanin_subset(n1: &PldNode, n2: &PldNode) -> bool {
    n1.fanins().iter().all(|fanin| n2.fanins().contains(fanin))
}

pub fn get_array_of_fanins(node: &PldNode) -> Option<Vec<NodeId>> {
    (node.kind != NodeKind::PrimaryInput).then(|| node.fanins().to_vec())
}

pub fn simplify_network_without_dc_blocked<Network>(_network: &mut Network) -> PldUtilResult<()> {
    Err(missing_native_ports(
        "pld_simplify_network_without_dc SIS integration",
    ))
}

pub fn is_network_feasible(network: &PldNetwork, support: usize) -> bool {
    network
        .nodes()
        .iter()
        .filter(|node| node.kind == NodeKind::Internal)
        .all(|node| node.fanin_count() <= support)
}

pub fn my_node_equal(node1: &PldNode, node2: &PldNode) -> bool {
    let fn1 = node1.function_kind();
    let fn2 = node2.function_kind();
    if (fn1 == NodeFunction::Zero && fn2 == NodeFunction::Zero)
        || (fn1 == NodeFunction::One && fn2 == NodeFunction::One)
    {
        return true;
    }
    node1.fanins == node2.fanins && node1.cubes == node2.cubes
}

pub fn insert_intermediate_nodes_in_table(network: &PldNetwork) -> BTreeSet<NodeId> {
    network
        .nodes()
        .iter()
        .enumerate()
        .filter_map(|(index, node)| (node.kind == NodeKind::Internal).then_some(NodeId(index)))
        .collect()
}

pub fn delete_array_nodes_from_table(
    table: &mut BTreeSet<NodeId>,
    nodes: &[NodeId],
) -> PldUtilResult<()> {
    for node in nodes {
        if !table.remove(node) {
            return Err(PldUtilError::MissingTableNode(*node));
        }
    }
    Ok(())
}

pub fn replace_node_by_network_blocked<Node, Network>(
    _node: &mut Node,
    _network: &Network,
) -> PldUtilResult<()> {
    Err(missing_native_ports(
        "pld_replace_node_by_network SIS integration",
    ))
}

fn make_product_from_cube(
    fanins: &[NodeId],
    cube: &PldCube,
    correspondence: &BTreeMap<NodeId, NodeId>,
) -> PldUtilResult<ProductTerm> {
    if cube.literals().len() != fanins.len() {
        return Err(PldUtilError::CubeArityMismatch {
            fanins: fanins.len(),
            literals: cube.literals().len(),
        });
    }

    let mut product = Vec::new();
    for (fanin, literal) in fanins.iter().copied().zip(cube.literals()) {
        let mapped = *correspondence
            .get(&fanin)
            .ok_or(PldUtilError::MissingFaninMapping(fanin))?;
        match literal {
            CubeLiteral::One => product.push(ProductLiteral::positive(mapped)),
            CubeLiteral::Zero => product.push(ProductLiteral::negative(mapped)),
            CubeLiteral::DontCare => {}
        }
    }
    Ok(ProductTerm::new(product))
}

fn identity_mapping(fanins: &[NodeId]) -> BTreeMap<NodeId, NodeId> {
    fanins.iter().copied().map(|fanin| (fanin, fanin)).collect()
}

fn missing_native_ports(operation: &'static str) -> PldUtilError {
    PldUtilError::MissingNativePorts { operation }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RowId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ColId(pub usize);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SparseMatrix {
    rows: BTreeMap<RowId, BTreeSet<ColId>>,
    cols: BTreeMap<ColId, BTreeSet<RowId>>,
}

impl SparseMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, row: RowId, col: ColId) {
        self.rows.entry(row).or_default().insert(col);
        self.cols.entry(col).or_default().insert(row);
    }

    pub fn rows(&self) -> &BTreeMap<RowId, BTreeSet<ColId>> {
        &self.rows
    }

    pub fn cols(&self) -> &BTreeMap<ColId, BTreeSet<RowId>> {
        &self.cols
    }

    pub fn contains(&self, row: RowId, col: ColId) -> bool {
        self.rows.get(&row).is_some_and(|cols| cols.contains(&col))
    }

    pub fn delrow(&mut self, row: RowId) {
        if let Some(cols) = self.rows.remove(&row) {
            for col in cols {
                if let Some(rows) = self.cols.get_mut(&col) {
                    rows.remove(&row);
                    if rows.is_empty() {
                        self.cols.remove(&col);
                    }
                }
            }
        }
    }

    pub fn delcol(&mut self, col: ColId) {
        if let Some(rows) = self.cols.remove(&col) {
            for row in rows {
                if let Some(cols) = self.rows.get_mut(&row) {
                    cols.remove(&col);
                    if cols.is_empty() {
                        self.rows.remove(&row);
                    }
                }
            }
        }
    }
}

pub fn sm_get_rows_covered_by_col(matrix: &SparseMatrix, col: ColId) -> Vec<RowId> {
    matrix
        .cols()
        .get(&col)
        .map(|rows| rows.iter().copied().collect())
        .unwrap_or_default()
}

pub fn sm_get_cols_covered_by_row(matrix: &SparseMatrix, row: RowId) -> Vec<ColId> {
    matrix
        .rows()
        .get(&row)
        .map(|cols| cols.iter().copied().collect())
        .unwrap_or_default()
}

pub fn sm_delete_rows_in_array(matrix: &mut SparseMatrix, rows: &[RowId]) {
    for row in rows {
        matrix.delrow(*row);
    }
}

pub fn sm_delete_cols_in_array(matrix: &mut SparseMatrix, cols: &[ColId]) {
    for col in cols {
        matrix.delcol(*col);
    }
}

pub fn sm_delete_rows_covered_by_col(matrix: &mut SparseMatrix, col: ColId) {
    let rows = sm_get_rows_covered_by_col(matrix, col);
    sm_delete_rows_in_array(matrix, &rows);
}

pub fn sm_delete_cols_covered_by_row(matrix: &mut SparseMatrix, row: RowId) {
    let cols = sm_get_cols_covered_by_row(matrix, row);
    sm_delete_cols_in_array(matrix, &cols);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_node() -> PldNode {
        PldNode::internal(
            "f",
            vec![NodeId(1), NodeId(2), NodeId(3)],
            vec![
                PldCube::new([CubeLiteral::One, CubeLiteral::DontCare, CubeLiteral::Zero]),
                PldCube::new([CubeLiteral::Zero, CubeLiteral::One, CubeLiteral::DontCare]),
            ],
        )
    }

    #[test]
    fn remap_get_node_walks_cubes_in_reverse_c_order_and_maps_phases() {
        let node = sample_node();
        let correspondence = BTreeMap::from([
            (NodeId(1), NodeId(10)),
            (NodeId(2), NodeId(20)),
            (NodeId(3), NodeId(30)),
        ]);

        let products = remap_get_node(&node, &correspondence).unwrap();

        assert_eq!(
            products,
            vec![
                ProductTerm::new([
                    ProductLiteral::negative(NodeId(10)),
                    ProductLiteral::positive(NodeId(20)),
                ]),
                ProductTerm::new([
                    ProductLiteral::positive(NodeId(10)),
                    ProductLiteral::negative(NodeId(30)),
                ]),
            ]
        );
    }

    #[test]
    fn nodes_and_cubes_from_cubes_preserve_reverse_cube_order() {
        let node = sample_node();

        assert_eq!(
            nodes_from_cubes(&node).unwrap(),
            vec![
                vec![ProductTerm::new([
                    ProductLiteral::negative(NodeId(1)),
                    ProductLiteral::positive(NodeId(2)),
                ])],
                vec![ProductTerm::new([
                    ProductLiteral::positive(NodeId(1)),
                    ProductLiteral::negative(NodeId(3)),
                ])],
            ]
        );
        assert_eq!(
            cubes_of_node(&node),
            vec![node.cubes()[1].clone(), node.cubes()[0].clone()]
        );
    }

    #[test]
    fn fanin_utilities_match_pointer_array_membership_semantics() {
        let node = sample_node();
        let subset = PldNode::internal("g", vec![NodeId(1), NodeId(3)], Vec::new());

        assert_eq!(
            get_non_common_fanins(&node, &[NodeId(2)]),
            vec![NodeId(1), NodeId(3)]
        );
        assert!(is_node_in_array(NodeId(2), node.fanins()));
        assert_eq!(num_fanin_cube(&node.cubes()[0], &node), Ok(2));
        assert!(is_fanin_subset(&subset, &node));
        assert_eq!(get_array_of_fanins(&node), Some(node.fanins().to_vec()));
        assert_eq!(get_array_of_fanins(&PldNode::primary_input("a")), None);
    }

    #[test]
    fn network_feasibility_and_node_equality_follow_c_special_cases() {
        let mut network = PldNetwork::new();
        network.add_node(PldNode::primary_input("a"));
        network.add_node(PldNode::constant_one());
        network.add_node(sample_node());

        assert!(is_network_feasible(&network, 3));
        assert!(!is_network_feasible(&network, 2));
        assert!(my_node_equal(
            &PldNode::constant_zero(),
            &PldNode::internal("different-zero-name", Vec::new(), Vec::new())
        ));
        assert!(my_node_equal(
            &PldNode::constant_one(),
            &PldNode::constant_one()
        ));
        assert!(!my_node_equal(&sample_node(), &PldNode::constant_one()));
    }

    #[test]
    fn remap_init_corr_matches_primary_inputs_by_name() {
        let mut target = PldNetwork::new();
        let a = target.add_node(PldNode::primary_input("a"));
        let b = target.add_node(PldNode::primary_input("b"));
        let mut replacement = PldNetwork::new();
        let rb = replacement.add_node(PldNode::primary_input("b"));
        let ra = replacement.add_node(PldNode::primary_input("a"));
        replacement.add_node(sample_node());

        assert_eq!(
            remap_init_corr(&target, &replacement).unwrap(),
            BTreeMap::from([(rb, b), (ra, a)])
        );
    }

    #[test]
    fn replacement_and_intermediate_table_helpers_are_native() {
        let mut network = PldNetwork::new();
        network.add_node(PldNode::primary_input("a"));
        let target = network.add_node(PldNode::constant_zero());
        let added = network
            .replace_node_with_array(
                target,
                vec![
                    sample_node(),
                    PldNode::internal("extra", vec![NodeId(0)], Vec::new()),
                ],
            )
            .unwrap();

        assert_eq!(added, vec![NodeId(2)]);
        assert_eq!(network.node(target).unwrap().fanin_count(), 3);

        let mut table = insert_intermediate_nodes_in_table(&network);
        assert!(table.contains(&target));
        assert!(table.contains(&NodeId(2)));
        delete_array_nodes_from_table(&mut table, &[target, NodeId(2)]).unwrap();
        assert_eq!(table, BTreeSet::new());
    }

    #[test]
    fn sparse_matrix_helpers_snapshot_then_delete_covered_rows_and_columns() {
        let mut matrix = SparseMatrix::new();
        matrix.insert(RowId(1), ColId(10));
        matrix.insert(RowId(2), ColId(10));
        matrix.insert(RowId(2), ColId(20));
        matrix.insert(RowId(3), ColId(30));

        assert_eq!(
            sm_get_rows_covered_by_col(&matrix, ColId(10)),
            vec![RowId(1), RowId(2)]
        );
        sm_delete_rows_covered_by_col(&mut matrix, ColId(10));
        assert!(!matrix.contains(RowId(1), ColId(10)));
        assert!(!matrix.contains(RowId(2), ColId(20)));
        assert!(matrix.contains(RowId(3), ColId(30)));

        matrix.insert(RowId(4), ColId(40));
        matrix.insert(RowId(4), ColId(50));
        matrix.insert(RowId(5), ColId(50));
        assert_eq!(
            sm_get_cols_covered_by_row(&matrix, RowId(4)),
            vec![ColId(40), ColId(50)]
        );
        sm_delete_cols_covered_by_row(&mut matrix, RowId(4));
        assert!(!matrix.contains(RowId(4), ColId(40)));
        assert!(!matrix.contains(RowId(5), ColId(50)));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("pld_util.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
