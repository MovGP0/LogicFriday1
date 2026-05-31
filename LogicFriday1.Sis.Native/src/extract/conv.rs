//! Native Rust conversion support for SIS extraction.
//!
//! The legacy module used package-wide globals to map network nodes to sparse
//! matrix columns and to insert newly extracted factors back into fanouts. This
//! port keeps that state in owned values so callers can convert covers to and
//! from extraction matrices without C ABI entry points or mutable global state.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SisIndex(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CubeNumber(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiteralValue {
    Zero,
    One,
    DontCare,
}

impl LiteralValue {
    fn matrix_column(self, fanin_index: SisIndex) -> Option<usize> {
        match self {
            Self::One => Some(fanin_index.0 * 2),
            Self::Zero => Some(fanin_index.0 * 2 + 1),
            Self::DontCare => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LogicCube {
    values: Vec<LiteralValue>,
}

impl LogicCube {
    pub fn new(values: impl Into<Vec<LiteralValue>>) -> Self {
        Self {
            values: values.into(),
        }
    }

    pub fn values(&self) -> &[LiteralValue] {
        &self.values
    }

    pub fn literal_count(&self) -> usize {
        self.values
            .iter()
            .filter(|value| **value != LiteralValue::DontCare)
            .count()
    }

    fn is_subcube_of(&self, other: &Self) -> bool {
        self.values.len() == other.values.len()
            && self
                .values
                .iter()
                .zip(other.values.iter())
                .all(|(left, right)| *left == LiteralValue::DontCare || left == right)
    }

    fn quotient_by(&self, divisor: &Self) -> Option<Self> {
        if !divisor.is_subcube_of(self) {
            return None;
        }

        let values = self
            .values
            .iter()
            .zip(divisor.values.iter())
            .map(|(left, right)| {
                if *right == LiteralValue::DontCare {
                    *left
                } else {
                    LiteralValue::DontCare
                }
            })
            .collect();

        Some(Self { values })
    }

    fn product(&self, other: &Self) -> Option<Self> {
        if self.values.len() != other.values.len() {
            return None;
        }

        let mut values = Vec::with_capacity(self.values.len());
        for (left, right) in self.values.iter().zip(other.values.iter()) {
            let value = match (*left, *right) {
                (LiteralValue::DontCare, value) | (value, LiteralValue::DontCare) => value,
                (left, right) if left == right => left,
                _ => return None,
            };
            values.push(value);
        }

        Some(Self { values })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicCover {
    cubes: Vec<LogicCube>,
}

impl LogicCover {
    pub fn new(cubes: impl Into<Vec<LogicCube>>) -> Self {
        let mut cubes = cubes.into();
        cubes.sort_unstable();
        cubes.dedup();

        Self { cubes }
    }

    pub fn empty() -> Self {
        Self { cubes: Vec::new() }
    }

    pub fn cubes(&self) -> &[LogicCube] {
        &self.cubes
    }

    pub fn add_cube(&mut self, cube: LogicCube) {
        match self.cubes.binary_search(&cube) {
            Ok(_) => {}
            Err(index) => {
                self.cubes.insert(index, cube);
            }
        }
    }

    fn contains(&self, cube: &LogicCube) -> bool {
        self.cubes.binary_search(cube).is_ok()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorSubstitution {
    factor: NodeId,
    complement: bool,
    quotient: Vec<LogicCube>,
}

impl FactorSubstitution {
    pub fn factor(&self) -> NodeId {
        self.factor
    }

    pub fn complement(&self) -> bool {
        self.complement
    }

    pub fn quotient(&self) -> &[LogicCube] {
        &self.quotient
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicNode {
    id: NodeId,
    name: String,
    kind: NodeKind,
    fanins: Vec<NodeId>,
    cover: LogicCover,
    substitutions: Vec<FactorSubstitution>,
}

impl LogicNode {
    pub fn new(
        id: NodeId,
        name: impl Into<String>,
        kind: NodeKind,
        fanins: impl Into<Vec<NodeId>>,
        cover: LogicCover,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            fanins: fanins.into(),
            cover,
            substitutions: Vec::new(),
        }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }

    pub fn cover(&self) -> &LogicCover {
        &self.cover
    }

    pub fn substitutions(&self) -> &[FactorSubstitution] {
        &self.substitutions
    }

    pub fn literal_count(&self) -> usize {
        self.cover.cubes.iter().map(LogicCube::literal_count).sum()
    }

    fn merge_original_cubes(&mut self, cubes: &[LogicCube]) {
        for cube in cubes {
            self.cover.add_cube(cube.clone());
        }
    }

    fn substitute_factor(&mut self, factor: &LogicNode, complement: bool) -> bool {
        let Some(divisor) = factor.cover_expanded_to(&self.fanins, self.fanins.len()) else {
            return false;
        };

        let Some(quotient) = find_complete_quotient(&self.cover, &divisor) else {
            return false;
        };

        let covered = factor_products(&divisor, &quotient);
        self.cover.cubes.retain(|cube| !covered.contains(cube));
        self.substitutions.push(FactorSubstitution {
            factor: factor.id,
            complement,
            quotient,
        });

        true
    }

    fn cover_expanded_to(&self, fanins: &[NodeId], width: usize) -> Option<LogicCover> {
        let positions = fanins
            .iter()
            .enumerate()
            .map(|(position, fanin)| (*fanin, position))
            .collect::<BTreeMap<_, _>>();
        let mut cubes = Vec::with_capacity(self.cover.cubes().len());

        for cube in self.cover.cubes() {
            if cube.values().len() != self.fanins.len() {
                return None;
            }

            let mut values = vec![LiteralValue::DontCare; width];
            for (source_position, fanin) in self.fanins.iter().enumerate() {
                let target_position = *positions.get(fanin)?;
                values[target_position] = cube.values()[source_position];
            }
            cubes.push(LogicCube::new(values));
        }

        Some(LogicCover::new(cubes))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LogicNetwork {
    nodes: BTreeMap<NodeId, LogicNode>,
    next_id: usize,
}

impl LogicNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_node(&mut self, node: LogicNode) -> Option<LogicNode> {
        self.next_id = self.next_id.max(node.id.0 + 1);
        self.nodes.insert(node.id, node)
    }

    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        kind: NodeKind,
        fanins: impl Into<Vec<NodeId>>,
        cover: LogicCover,
    ) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.nodes
            .insert(id, LogicNode::new(id, name, kind, fanins, cover));
        id
    }

    pub fn node(&self, id: NodeId) -> Option<&LogicNode> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut LogicNode> {
        self.nodes.get_mut(&id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &LogicNode> {
        self.nodes.values()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeIndex {
    nodes: Vec<NodeId>,
    positions: BTreeMap<NodeId, SisIndex>,
}

impl NodeIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, node: NodeId) -> SisIndex {
        if let Some(index) = self.positions.get(&node) {
            return *index;
        }

        let index = SisIndex(self.nodes.len());
        self.nodes.push(node);
        self.positions.insert(node, index);
        index
    }

    pub fn index_of(&self, node: NodeId) -> Option<SisIndex> {
        self.positions.get(&node).copied()
    }

    pub fn node_of(&self, index: SisIndex) -> Option<NodeId> {
        self.nodes.get(index.0).copied()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractionContext {
    node_index: NodeIndex,
    original_covers: BTreeMap<NodeId, LogicCover>,
}

impl ExtractionContext {
    pub fn for_network(network: &LogicNetwork, duplicate_functions: bool) -> Self {
        let mut node_index = NodeIndex::new();
        let mut original_covers = BTreeMap::new();

        for node in network.nodes() {
            if node.kind() != NodeKind::PrimaryOutput {
                node_index.insert(node.id());
                if duplicate_functions {
                    original_covers.insert(node.id(), node.cover().clone());
                }
            }
        }

        Self {
            node_index,
            original_covers,
        }
    }

    pub fn for_single_node(node: &LogicNode) -> Self {
        let mut node_index = NodeIndex::new();
        for fanin in node.fanins() {
            node_index.insert(*fanin);
        }
        node_index.insert(node.id());

        Self {
            node_index,
            original_covers: BTreeMap::new(),
        }
    }

    pub fn node_index(&self) -> &NodeIndex {
        &self.node_index
    }

    pub fn map_index_to_name<'a>(
        &self,
        network: &'a LogicNetwork,
        index: SisIndex,
    ) -> Result<&'a str, ConversionError> {
        let node_id = self
            .node_index
            .node_of(index)
            .ok_or(ConversionError::UnknownSisIndex(index))?;
        let node = network
            .node(node_id)
            .ok_or(ConversionError::MissingNode(node_id))?;

        Ok(node.name())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueCell {
    pub value: i32,
    pub sis_index: SisIndex,
    pub cube_number: CubeNumber,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExtractionMatrix {
    cells: BTreeMap<(usize, usize), ValueCell>,
    row_count: usize,
    col_count: usize,
}

impl ExtractionMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, row: usize, column: usize, cell: ValueCell) -> Option<ValueCell> {
        self.row_count = self.row_count.max(row + 1);
        self.col_count = self.col_count.max(column + 1);
        self.cells.insert((row, column), cell)
    }

    pub fn cell(&self, row: usize, column: usize) -> Option<&ValueCell> {
        self.cells.get(&(row, column))
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn col_count(&self) -> usize {
        self.col_count
    }

    pub fn element_count(&self) -> usize {
        self.cells.len()
    }

    pub fn rows(&self) -> Vec<usize> {
        self.cells
            .keys()
            .map(|(row, _)| *row)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn columns(&self) -> Vec<usize> {
        self.cells
            .keys()
            .map(|(_, column)| *column)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn row_columns(&self, row: usize) -> Vec<usize> {
        self.cells
            .keys()
            .filter_map(|(candidate_row, column)| (*candidate_row == row).then_some(*column))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DivideResult {
    pub inserted_index: Option<SisIndex>,
    pub changed_fanouts: usize,
    pub expected_fanouts: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConversionError {
    MissingNode(NodeId),
    MissingFanin { node: NodeId, fanin: NodeId },
    UnknownSisIndex(SisIndex),
    InconsistentCubeWidth { expected: usize, actual: usize },
    ConflictingMatrixColumn { row: usize, node: NodeId },
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "missing logic node {}", node.0),
            Self::MissingFanin { node, fanin } => {
                write!(f, "node {} references unindexed fanin {}", node.0, fanin.0)
            }
            Self::UnknownSisIndex(index) => write!(f, "unknown SIS node index {}", index.0),
            Self::InconsistentCubeWidth { expected, actual } => {
                write!(
                    f,
                    "cube width {actual} does not match fanin width {expected}"
                )
            }
            Self::ConflictingMatrixColumn { row, node } => {
                write!(
                    f,
                    "matrix row {row} contains both phases for node {}",
                    node.0
                )
            }
        }
    }
}

impl Error for ConversionError {}

pub fn node_to_extraction_matrix(
    node: &LogicNode,
    context: &ExtractionContext,
    matrix: &mut ExtractionMatrix,
) -> Result<(), ConversionError> {
    let node_index = context
        .node_index
        .index_of(node.id())
        .ok_or(ConversionError::UnknownSisIndex(SisIndex(node.id().0)))?;

    let fanin_indexes = node
        .fanins()
        .iter()
        .map(|fanin| {
            context
                .node_index
                .index_of(*fanin)
                .ok_or(ConversionError::MissingFanin {
                    node: node.id(),
                    fanin: *fanin,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    for cube in node.cover().cubes() {
        if cube.values().len() != fanin_indexes.len() {
            return Err(ConversionError::InconsistentCubeWidth {
                expected: fanin_indexes.len(),
                actual: cube.values().len(),
            });
        }

        let row = matrix.row_count();
        for (value, fanin_index) in cube.values().iter().zip(fanin_indexes.iter()) {
            if let Some(column) = value.matrix_column(*fanin_index) {
                matrix.insert(
                    row,
                    column,
                    ValueCell {
                        value: 1,
                        sis_index: node_index,
                        cube_number: CubeNumber(row),
                    },
                );
            }
        }

        matrix.row_count = matrix.row_count.max(row + 1);
    }

    Ok(())
}

pub fn network_to_extraction_matrix(
    network: &LogicNetwork,
    context: &ExtractionContext,
) -> Result<ExtractionMatrix, ConversionError> {
    let mut matrix = ExtractionMatrix::new();
    for node in network.nodes() {
        if node.kind() == NodeKind::Internal {
            node_to_extraction_matrix(node, context, &mut matrix)?;
        }
    }

    Ok(matrix)
}

pub fn extraction_matrix_to_node(
    matrix: &ExtractionMatrix,
    context: &ExtractionContext,
) -> Result<LogicNode, ConversionError> {
    let fanins = fanins_for_matrix_columns(matrix, context)?;
    let position_by_fanin = fanins
        .iter()
        .enumerate()
        .map(|(position, node)| (*node, position))
        .collect::<BTreeMap<_, _>>();
    let mut cubes = Vec::new();

    for row in matrix.rows() {
        let mut values = vec![LiteralValue::DontCare; fanins.len()];
        for column in matrix.row_columns(row) {
            let sis_index = SisIndex(column / 2);
            let fanin = context
                .node_index
                .node_of(sis_index)
                .ok_or(ConversionError::UnknownSisIndex(sis_index))?;
            let position = position_by_fanin[&fanin];
            let value = if column % 2 == 0 {
                LiteralValue::One
            } else {
                LiteralValue::Zero
            };

            if values[position] != LiteralValue::DontCare && values[position] != value {
                return Err(ConversionError::ConflictingMatrixColumn { row, node: fanin });
            }

            values[position] = value;
        }

        cubes.push(LogicCube::new(values));
    }

    Ok(LogicNode::new(
        NodeId(usize::MAX),
        "extracted",
        NodeKind::Internal,
        fanins,
        LogicCover::new(cubes),
    ))
}

pub fn divide_function_into_network(
    network: &mut LogicNetwork,
    context: &mut ExtractionContext,
    function: &ExtractionMatrix,
    fanouts: &[SisIndex],
    cubes: Option<&[Vec<CubeNumber>]>,
    complement: bool,
) -> Result<DivideResult, ConversionError> {
    let mut factor = extraction_matrix_to_node(function, context)?;
    let changed_fanouts =
        divide_each_fanout(network, context, fanouts, cubes, &factor, complement)?;

    let inserted_index = if changed_fanouts > 0 {
        let factor_id = network.add_node(
            "extracted",
            NodeKind::Internal,
            factor.fanins.clone(),
            factor.cover.clone(),
        );
        factor.id = factor_id;
        Some(context.node_index.insert(factor_id))
    } else {
        None
    };

    Ok(DivideResult {
        inserted_index,
        changed_fanouts,
        expected_fanouts: fanouts.len(),
    })
}

fn divide_each_fanout(
    network: &mut LogicNetwork,
    context: &ExtractionContext,
    fanouts: &[SisIndex],
    cubes: Option<&[Vec<CubeNumber>]>,
    factor: &LogicNode,
    complement: bool,
) -> Result<usize, ConversionError> {
    let mut changed = 0;

    for (position, fanout_index) in fanouts.iter().enumerate() {
        let fanout_id = context
            .node_index
            .node_of(*fanout_index)
            .ok_or(ConversionError::UnknownSisIndex(*fanout_index))?;

        if let Some(cube_groups) = cubes {
            if let Some(original_cover) = context.original_covers.get(&fanout_id) {
                let selected_cubes = cube_groups
                    .get(position)
                    .into_iter()
                    .flatten()
                    .filter_map(|cube| original_cover.cubes().get(cube.0))
                    .cloned()
                    .collect::<Vec<_>>();

                let node = network
                    .node_mut(fanout_id)
                    .ok_or(ConversionError::MissingNode(fanout_id))?;
                node.merge_original_cubes(&selected_cubes);
            }
        }

        let node = network
            .node_mut(fanout_id)
            .ok_or(ConversionError::MissingNode(fanout_id))?;
        if node.substitute_factor(factor, complement) {
            changed += 1;
        }
    }

    Ok(changed)
}

fn fanins_for_matrix_columns(
    matrix: &ExtractionMatrix,
    context: &ExtractionContext,
) -> Result<Vec<NodeId>, ConversionError> {
    let mut fanins = Vec::new();
    let mut last_fanin = None;

    for column in matrix.columns() {
        let sis_index = SisIndex(column / 2);
        let fanin = context
            .node_index
            .node_of(sis_index)
            .ok_or(ConversionError::UnknownSisIndex(sis_index))?;

        if column % 2 == 0 || last_fanin != Some(fanin) {
            fanins.push(fanin);
            last_fanin = Some(fanin);
        }
    }

    Ok(fanins)
}

fn find_complete_quotient(target: &LogicCover, divisor: &LogicCover) -> Option<Vec<LogicCube>> {
    let mut candidates = BTreeSet::new();

    for target_cube in target.cubes() {
        for divisor_cube in divisor.cubes() {
            if let Some(quotient) = target_cube.quotient_by(divisor_cube) {
                candidates.insert(quotient);
            }
        }
    }

    for quotient in candidates {
        let products = factor_products(divisor, std::slice::from_ref(&quotient));
        if products.iter().all(|cube| target.contains(cube)) {
            return Some(vec![quotient]);
        }
    }

    None
}

fn factor_products(divisor: &LogicCover, quotients: &[LogicCube]) -> BTreeSet<LogicCube> {
    let mut products = BTreeSet::new();
    for divisor_cube in divisor.cubes() {
        for quotient in quotients {
            if let Some(product) = divisor_cube.product(quotient) {
                products.insert(product);
            }
        }
    }

    products
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: NodeId = NodeId(0);
    const B: NodeId = NodeId(1);
    const F: NodeId = NodeId(2);
    const OUT: NodeId = NodeId(3);

    fn cube(values: &[LiteralValue]) -> LogicCube {
        LogicCube::new(values.to_vec())
    }

    fn cover(cubes: Vec<LogicCube>) -> LogicCover {
        LogicCover::new(cubes)
    }

    fn sample_network() -> LogicNetwork {
        let mut network = LogicNetwork::new();
        network.insert_node(LogicNode::new(
            A,
            "a",
            NodeKind::PrimaryInput,
            [],
            LogicCover::empty(),
        ));
        network.insert_node(LogicNode::new(
            B,
            "b",
            NodeKind::PrimaryInput,
            [],
            LogicCover::empty(),
        ));
        network.insert_node(LogicNode::new(
            F,
            "f",
            NodeKind::Internal,
            [A, B],
            cover(vec![
                cube(&[LiteralValue::One, LiteralValue::Zero]),
                cube(&[LiteralValue::Zero, LiteralValue::One]),
            ]),
        ));
        network.insert_node(LogicNode::new(
            OUT,
            "out",
            NodeKind::PrimaryOutput,
            [F],
            LogicCover::empty(),
        ));

        network
    }

    #[test]
    fn setup_context_indexes_non_outputs_and_maps_names() {
        let network = sample_network();
        let context = ExtractionContext::for_network(&network, true);

        assert_eq!(context.node_index().len(), 3);
        assert_eq!(context.map_index_to_name(&network, SisIndex(0)), Ok("a"));
        assert_eq!(context.map_index_to_name(&network, SisIndex(2)), Ok("f"));
        assert_eq!(context.node_index().index_of(OUT), None);
    }

    #[test]
    fn node_to_matrix_records_literal_columns_and_value_cell_origins() {
        let network = sample_network();
        let context = ExtractionContext::for_network(&network, false);
        let node = network.node(F).unwrap();
        let mut matrix = ExtractionMatrix::new();

        node_to_extraction_matrix(node, &context, &mut matrix).unwrap();

        assert_eq!(matrix.row_count(), 2);
        assert_eq!(matrix.element_count(), 4);
        assert_eq!(
            matrix.cell(0, 1),
            Some(&ValueCell {
                value: 1,
                sis_index: SisIndex(2),
                cube_number: CubeNumber(0),
            })
        );
        assert_eq!(
            matrix.cell(0, 2).map(|cell| cell.cube_number),
            Some(CubeNumber(0))
        );
        assert_eq!(
            matrix.cell(1, 0).map(|cell| cell.sis_index),
            Some(SisIndex(2))
        );
        assert_eq!(matrix.cell(1, 3).map(|cell| cell.value), Some(1));
    }

    #[test]
    fn network_to_matrix_skips_primary_outputs() {
        let network = sample_network();
        let context = ExtractionContext::for_network(&network, false);

        let matrix = network_to_extraction_matrix(&network, &context).unwrap();

        assert_eq!(matrix.row_count(), 2);
        assert_eq!(matrix.columns(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn matrix_to_node_reconstructs_fanins_and_cover() {
        let network = sample_network();
        let context = ExtractionContext::for_network(&network, false);
        let matrix = network_to_extraction_matrix(&network, &context).unwrap();

        let node = extraction_matrix_to_node(&matrix, &context).unwrap();

        assert_eq!(node.fanins(), &[A, B]);
        assert_eq!(
            node.cover(),
            &cover(vec![
                cube(&[LiteralValue::One, LiteralValue::Zero]),
                cube(&[LiteralValue::Zero, LiteralValue::One]),
            ])
        );
    }

    #[test]
    fn matrix_to_node_allows_column_with_only_negative_phase() {
        let mut network = LogicNetwork::new();
        network.insert_node(LogicNode::new(
            A,
            "a",
            NodeKind::PrimaryInput,
            [],
            LogicCover::empty(),
        ));
        let context = ExtractionContext::for_network(&network, false);
        let mut matrix = ExtractionMatrix::new();
        matrix.insert(
            0,
            1,
            ValueCell {
                value: 1,
                sis_index: SisIndex(0),
                cube_number: CubeNumber(0),
            },
        );

        let node = extraction_matrix_to_node(&matrix, &context).unwrap();

        assert_eq!(node.fanins(), &[A]);
        assert_eq!(node.cover(), &cover(vec![cube(&[LiteralValue::Zero])]));
    }

    #[test]
    fn divide_function_inserts_factor_when_it_divides_fanout() {
        let mut network = LogicNetwork::new();
        network.insert_node(LogicNode::new(
            A,
            "a",
            NodeKind::PrimaryInput,
            [],
            LogicCover::empty(),
        ));
        network.insert_node(LogicNode::new(
            B,
            "b",
            NodeKind::PrimaryInput,
            [],
            LogicCover::empty(),
        ));
        let target = network.add_node(
            "target",
            NodeKind::Internal,
            [A, B],
            cover(vec![
                cube(&[LiteralValue::One, LiteralValue::One]),
                cube(&[LiteralValue::Zero, LiteralValue::One]),
                cube(&[LiteralValue::One, LiteralValue::Zero]),
            ]),
        );
        let mut context = ExtractionContext::for_network(&network, false);
        let target_index = context.node_index().index_of(target).unwrap();
        let mut function = ExtractionMatrix::new();
        function.insert(
            0,
            0,
            ValueCell {
                value: 1,
                sis_index: target_index,
                cube_number: CubeNumber(0),
            },
        );
        function.insert(
            1,
            1,
            ValueCell {
                value: 1,
                sis_index: target_index,
                cube_number: CubeNumber(1),
            },
        );

        let result = divide_function_into_network(
            &mut network,
            &mut context,
            &function,
            &[target_index],
            None,
            false,
        )
        .unwrap();

        assert_eq!(result.changed_fanouts, 1);
        assert_eq!(result.inserted_index, Some(SisIndex(3)));
        let target_node = network.node(target).unwrap();
        assert_eq!(
            target_node.cover(),
            &cover(vec![cube(&[LiteralValue::One, LiteralValue::Zero])])
        );
        assert_eq!(target_node.substitutions().len(), 1);
        assert_eq!(
            target_node.substitutions()[0].quotient(),
            &[cube(&[LiteralValue::DontCare, LiteralValue::One])]
        );
    }

    #[test]
    fn divide_function_can_restore_selected_original_cubes_before_dividing() {
        let mut network = LogicNetwork::new();
        network.insert_node(LogicNode::new(
            A,
            "a",
            NodeKind::PrimaryInput,
            [],
            LogicCover::empty(),
        ));
        network.insert_node(LogicNode::new(
            B,
            "b",
            NodeKind::PrimaryInput,
            [],
            LogicCover::empty(),
        ));
        let target = network.add_node(
            "target",
            NodeKind::Internal,
            [A, B],
            cover(vec![cube(&[LiteralValue::One, LiteralValue::One])]),
        );
        let mut context = ExtractionContext::for_network(&network, true);
        let target_index = context.node_index().index_of(target).unwrap();
        network
            .node_mut(target)
            .unwrap()
            .cover
            .cubes
            .retain(|candidate| candidate != &cube(&[LiteralValue::Zero, LiteralValue::One]));

        context.original_covers.insert(
            target,
            cover(vec![
                cube(&[LiteralValue::One, LiteralValue::One]),
                cube(&[LiteralValue::Zero, LiteralValue::One]),
            ]),
        );

        let mut function = ExtractionMatrix::new();
        function.insert(
            0,
            0,
            ValueCell {
                value: 1,
                sis_index: target_index,
                cube_number: CubeNumber(0),
            },
        );
        function.insert(
            1,
            1,
            ValueCell {
                value: 1,
                sis_index: target_index,
                cube_number: CubeNumber(1),
            },
        );

        let result = divide_function_into_network(
            &mut network,
            &mut context,
            &function,
            &[target_index],
            Some(&[vec![CubeNumber(0)]]),
            true,
        )
        .unwrap();

        assert_eq!(result.changed_fanouts, 1);
        assert!(network.node(target).unwrap().cover().cubes().is_empty());
        assert!(network.node(target).unwrap().substitutions()[0].complement());
    }

    #[test]
    fn conflicting_matrix_columns_are_rejected() {
        let network = sample_network();
        let context = ExtractionContext::for_network(&network, false);
        let mut matrix = ExtractionMatrix::new();
        for column in [0, 1] {
            matrix.insert(
                0,
                column,
                ValueCell {
                    value: 1,
                    sis_index: SisIndex(2),
                    cube_number: CubeNumber(0),
                },
            );
        }

        assert_eq!(
            extraction_matrix_to_node(&matrix, &context),
            Err(ConversionError::ConflictingMatrixColumn { row: 0, node: A })
        );
    }

    #[test]
    fn no_legacy_c_abi_or_dependency_metadata_tokens_are_present() {
        let source = include_str!("conv.rs");

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
