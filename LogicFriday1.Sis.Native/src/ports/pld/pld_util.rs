//! Native Rust model for `LogicSynthesis/sis/pld/pld_util.c`.
//!
//! The C file is a shared PLD utility layer over SIS `node_t`, `network_t`,
//! `array_t`, `st_table`, and sparse-matrix APIs. This port models the
//! Boolean cube, fanin-set, replacement, simplification, and sparse deletion
//! behavior with owned Rust data.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::ports::node::node::{Cover, Cube, Node, NodeError, node_num_cube, node_num_literal};
use crate::ports::simplify::simp::{
    NativeSimplifyNode, NodeMetrics, SimAccept, SimDcType, SimFilter, SimMethod, SimpDcParameters,
    SimplifyError, SimplifyNodeOptions, simplify_node_native,
};

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

    pub fn replace_node_by_network(
        &mut self,
        node: NodeId,
        replacement_network: &PldNetwork,
    ) -> PldUtilResult<Vec<NodeId>> {
        self.node(node)?;

        let mut correspondence = remap_init_corr(self, replacement_network)?;
        let internal_nodes = replacement_network.internal_node_ids();
        let Some((&replacement_output, intermediate_nodes)) = internal_nodes.split_last() else {
            return Err(PldUtilError::NoReplacementOutput);
        };

        let mut added = Vec::new();
        for replacement_node in intermediate_nodes {
            let source = replacement_network.node(*replacement_node)?;
            let products = remap_get_node(source, &correspondence)?;
            let mapped = pld_node_from_products(source.name.clone(), source.kind, products)?;
            let mapped_id = self.add_node(mapped);
            correspondence.insert(*replacement_node, mapped_id);
            added.push(mapped_id);
        }

        let source = replacement_network.node(replacement_output)?;
        let products = remap_get_node(source, &correspondence)?;
        let mapped = pld_node_from_products(source.name.clone(), source.kind, products)?;
        let target = self.node_mut(node)?;
        target.fanins = mapped.fanins;
        target.cubes = mapped.cubes;
        Ok(added)
    }

    pub fn simplify_network_without_dc(&mut self) -> PldUtilResult<Vec<PldSimplifyReport>> {
        let node_ids = self.internal_node_ids();
        let mut reports = Vec::new();

        for node_id in node_ids {
            let source = self.node(node_id)?.clone();
            let native = native_node_from_pld_node(self, &source)?;
            let metrics = metrics_for_node(&native)?;
            let mut simplify_node = NativeSimplifyNode::new(native, metrics);
            let outcome = simplify_node_native(
                &mut simplify_node,
                SimplifyNodeOptions {
                    method: SimMethod::SNoComp,
                    dctype: SimDcType::None,
                    accept: SimAccept::SopLiterals,
                    filter: SimFilter::None,
                    parameters: SimpDcParameters {
                        fanin_level: 0,
                        fanin_fanout_level: 0,
                    },
                },
            )
            .map_err(|error| map_simplify_error("pld_simplify_network_without_dc", error))?;

            let simplified = pld_node_from_native_node(self, &source, &simplify_node.value)?;
            let target = self.node_mut(node_id)?;
            target.fanins = simplified.fanins;
            target.cubes = simplified.cubes;
            reports.push(PldSimplifyReport {
                node: node_id,
                replaced: outcome.replaced,
            });
        }

        Ok(reports)
    }

    fn internal_node_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (node.kind == NodeKind::Internal).then_some(NodeId(index)))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PldSimplifyReport {
    pub node: NodeId,
    pub replaced: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PldUtilError {
    UnknownNode(NodeId),
    MissingFaninMapping(NodeId),
    MissingPrimaryInput {
        name: String,
    },
    MissingSimplifiedFanin {
        name: String,
    },
    CubeArityMismatch {
        fanins: usize,
        literals: usize,
    },
    ConflictingProductLiteral {
        node: NodeId,
    },
    EmptyReplacementArray,
    NoReplacementOutput,
    MissingTableNode(NodeId),
    Node {
        operation: &'static str,
        reason: String,
    },
    Simplify {
        operation: &'static str,
        reason: String,
    },
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
            Self::MissingSimplifiedFanin { name } => {
                write!(f, "simplified fanin {name} is absent from PLD network")
            }
            Self::CubeArityMismatch { fanins, literals } => write!(
                f,
                "cube literal count {literals} does not match fanin count {fanins}"
            ),
            Self::ConflictingProductLiteral { node } => {
                write!(
                    f,
                    "product contains conflicting phases for fanin {:?}",
                    node
                )
            }
            Self::EmptyReplacementArray => write!(f, "node replacement array is empty"),
            Self::NoReplacementOutput => write!(f, "replacement network has no internal output"),
            Self::MissingTableNode(node) => write!(f, "node {:?} is absent from table", node),
            Self::Node { operation, reason } => {
                write!(f, "{operation} failed in native node port: {reason}")
            }
            Self::Simplify { operation, reason } => {
                write!(f, "{operation} failed in native simplify port: {reason}")
            }
        }
    }
}

impl Error for PldUtilError {}

pub type PldUtilResult<T> = Result<T, PldUtilError>;

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

pub fn simplify_network_without_dc(
    network: &mut PldNetwork,
) -> PldUtilResult<Vec<PldSimplifyReport>> {
    network.simplify_network_without_dc()
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

pub fn replace_node_by_network(
    network: &mut PldNetwork,
    node: NodeId,
    replacement_network: &PldNetwork,
) -> PldUtilResult<Vec<NodeId>> {
    network.replace_node_by_network(node, replacement_network)
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

fn pld_node_from_products(
    name: String,
    kind: NodeKind,
    products: Vec<ProductTerm>,
) -> PldUtilResult<PldNode> {
    let fanins = products
        .iter()
        .flat_map(|product| product.literals().iter().map(|literal| literal.node))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut cubes = Vec::new();

    for product in products {
        let mut literals = vec![CubeLiteral::DontCare; fanins.len()];
        for literal in product.literals() {
            let index = fanins
                .iter()
                .position(|fanin| *fanin == literal.node)
                .expect("product fanin should be present in collected fanins");
            let cube_literal = match literal.phase {
                LiteralPhase::Positive => CubeLiteral::One,
                LiteralPhase::Negative => CubeLiteral::Zero,
            };
            if literals[index] != CubeLiteral::DontCare && literals[index] != cube_literal {
                return Err(PldUtilError::ConflictingProductLiteral { node: literal.node });
            }
            literals[index] = cube_literal;
        }
        cubes.push(PldCube::new(literals));
    }

    Ok(PldNode::new(name, kind, fanins, cubes))
}

fn native_node_from_pld_node(network: &PldNetwork, node: &PldNode) -> PldUtilResult<Node> {
    let fanin_names = node
        .fanins()
        .iter()
        .map(|fanin| {
            network
                .node(*fanin)
                .map(|fanin_node| fanin_node.name.clone())
        })
        .collect::<PldUtilResult<Vec<_>>>()?;
    let cubes = node
        .cubes()
        .iter()
        .map(|cube| {
            Cube::new(
                cube.literals()
                    .iter()
                    .map(|literal| match literal {
                        CubeLiteral::Zero => Some(false),
                        CubeLiteral::One => Some(true),
                        CubeLiteral::DontCare => None,
                    })
                    .collect(),
            )
        })
        .collect();
    let cover = Cover::new(fanin_names.len(), cubes)
        .map_err(|error| map_node_error("PLD to native node conversion", error))?;
    let mut native = Node::new(cover, fanin_names);
    native.name = Some(node.name.clone());
    native.short_name = Some(node.name.clone());
    Ok(native)
}

fn pld_node_from_native_node(
    network: &PldNetwork,
    original: &PldNode,
    native: &Node,
) -> PldUtilResult<PldNode> {
    let fanins = native
        .fanins
        .iter()
        .map(|name| {
            network
                .find_node_by_name(name)
                .ok_or_else(|| PldUtilError::MissingSimplifiedFanin { name: name.clone() })
        })
        .collect::<PldUtilResult<Vec<_>>>()?;
    let Some(function) = native.function() else {
        return Ok(PldNode::new(
            original.name.clone(),
            original.kind,
            fanins,
            Vec::new(),
        ));
    };
    let cubes = function
        .cubes()
        .iter()
        .map(|cube| {
            PldCube::new(cube.inputs().iter().map(|input| match input {
                Some(false) => CubeLiteral::Zero,
                Some(true) => CubeLiteral::One,
                None => CubeLiteral::DontCare,
            }))
        })
        .collect();

    Ok(PldNode::new(
        original.name.clone(),
        original.kind,
        fanins,
        cubes,
    ))
}

fn metrics_for_node(node: &Node) -> PldUtilResult<NodeMetrics> {
    let sop_literals =
        node_num_literal(node).map_err(|error| map_node_error("node literal metrics", error))?;
    let cubes = node_num_cube(node).map_err(|error| map_node_error("node cube metrics", error))?;
    Ok(NodeMetrics::new(
        sop_literals,
        sop_literals,
        cubes,
        node.fanins.len(),
    ))
}

fn map_node_error(operation: &'static str, error: NodeError) -> PldUtilError {
    PldUtilError::Node {
        operation,
        reason: error.to_string(),
    }
}

fn map_simplify_error(operation: &'static str, error: SimplifyError) -> PldUtilError {
    PldUtilError::Simplify {
        operation,
        reason: error.to_string(),
    }
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
    fn replace_node_by_network_remaps_primary_inputs_and_intermediate_nodes() {
        let mut target = PldNetwork::new();
        let a = target.add_node(PldNode::primary_input("a"));
        let b = target.add_node(PldNode::primary_input("b"));
        let out = target.add_node(PldNode::constant_zero());

        let mut replacement = PldNetwork::new();
        let ra = replacement.add_node(PldNode::primary_input("a"));
        let rb = replacement.add_node(PldNode::primary_input("b"));
        let and = replacement.add_node(PldNode::internal(
            "and",
            vec![ra, rb],
            vec![PldCube::new([CubeLiteral::One, CubeLiteral::One])],
        ));
        replacement.add_node(PldNode::internal(
            "final",
            vec![and, rb],
            vec![PldCube::new([CubeLiteral::One, CubeLiteral::Zero])],
        ));

        let added = replace_node_by_network(&mut target, out, &replacement).unwrap();

        assert_eq!(added, vec![NodeId(3)]);
        assert_eq!(
            target.node(NodeId(3)).unwrap(),
            &PldNode::internal(
                "and",
                vec![a, b],
                vec![PldCube::new([CubeLiteral::One, CubeLiteral::One])]
            )
        );
        assert_eq!(
            target.node(out).unwrap(),
            &PldNode::internal(
                "$zero",
                vec![b, NodeId(3)],
                vec![PldCube::new([CubeLiteral::Zero, CubeLiteral::One])]
            )
        );
    }

    #[test]
    fn simplify_network_without_dc_runs_snocomp_on_internal_nodes() {
        let mut network = PldNetwork::new();
        let a = network.add_node(PldNode::primary_input("a"));
        let b = network.add_node(PldNode::primary_input("b"));
        let target = network.add_node(PldNode::internal(
            "f",
            vec![a, b],
            vec![
                PldCube::new([CubeLiteral::One, CubeLiteral::One]),
                PldCube::new([CubeLiteral::One, CubeLiteral::Zero]),
            ],
        ));

        let reports = simplify_network_without_dc(&mut network).unwrap();

        assert_eq!(
            reports,
            vec![PldSimplifyReport {
                node: target,
                replaced: true
            }]
        );
        assert_eq!(
            network.node(target).unwrap(),
            &PldNode::internal("f", vec![a], vec![PldCube::new([CubeLiteral::One])])
        );
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
