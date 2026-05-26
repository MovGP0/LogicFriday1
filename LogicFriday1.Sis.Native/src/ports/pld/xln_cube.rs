//! Native Rust model for `LogicSynthesis/sis/pld/xln_cube.c`.
//!
//! The C file decomposes over-wide AND/OR logic for Xilinx lookup tables:
//! large product cubes are factored into size-limited AND nodes, cube nodes are
//! greedily packed into table lookup nodes by best-fit composite fanin, and the
//! original node is replaced by the packed OR. This port models that behavior
//! with owned sum-of-products data. Direct mutation of SIS `network_t`,
//! `node_t`, and `array_t` remains blocked behind explicit dependency errors.

use std::collections::BTreeSet;
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
        reason: "xln_cube.c stores DFS nodes, cube nodes, AND nodes, and TLU nodes in array_t",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.297",
        source_file: "LogicSynthesis/sis/network/dfs.c",
        reason: "xln_network_ao_map visits nodes in network_dfs order",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "network_add_node inserts extracted AND nodes and full TLU nodes",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "fanin traversal and node_num_fanin drive cube extraction and TLU packing",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node constructors and Boolean AND/OR/literal operations build extracted logic",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.321",
        source_file: "LogicSynthesis/sis/node/nodemisc.c",
        reason: "node_replace installs packed TLU functions in the original node",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.325",
        source_file: "LogicSynthesis/sis/node/substitute.c",
        reason: "node_substitute replaces extracted product terms with the generated AND node",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.376",
        source_file: "LogicSynthesis/sis/pld/pld_util.c",
        reason: "pld_num_fanin_cube and pld_make_node_from_cube convert SIS cubes to node functions",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.390",
        source_file: "LogicSynthesis/sis/pld/xln_level.c",
        reason: "xln_is_cube_absorbed uses xln_num_composite_fanin to test TLU capacity",
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiteralPhase {
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CubeLiteral {
    pub node: NodeId,
    pub phase: LiteralPhase,
}

impl CubeLiteral {
    pub fn positive(node: NodeId) -> Self {
        Self {
            node,
            phase: LiteralPhase::Positive,
        }
    }

    pub fn negative(node: NodeId) -> Self {
        Self {
            node,
            phase: LiteralPhase::Negative,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProductCube {
    literals: Vec<CubeLiteral>,
}

impl ProductCube {
    pub fn new(literals: impl IntoIterator<Item = CubeLiteral>) -> Self {
        let mut cube = Self {
            literals: literals.into_iter().collect(),
        };
        cube.normalize();
        cube
    }

    pub fn literals(&self) -> &[CubeLiteral] {
        &self.literals
    }

    pub fn literal_count(&self) -> usize {
        self.literals.len()
    }

    fn contains_all(&self, extracted: &[CubeLiteral]) -> bool {
        extracted
            .iter()
            .all(|literal| self.literals.contains(literal))
    }

    fn replace_literals_with_node(&mut self, extracted: &[CubeLiteral], replacement: NodeId) {
        self.literals.retain(|literal| !extracted.contains(literal));
        self.literals.push(CubeLiteral::positive(replacement));
        self.normalize();
    }

    fn normalize(&mut self) {
        self.literals.sort_unstable();
        self.literals.dedup();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BooleanNode {
    pub name: String,
    pub kind: NodeKind,
    cubes: Vec<ProductCube>,
}

impl BooleanNode {
    pub fn new(name: impl Into<String>, kind: NodeKind, cubes: Vec<ProductCube>) -> Self {
        Self {
            name: name.into(),
            kind,
            cubes,
        }
    }

    pub fn primary_input(name: impl Into<String>) -> Self {
        Self::new(name, NodeKind::PrimaryInput, Vec::new())
    }

    pub fn internal(name: impl Into<String>, cubes: Vec<ProductCube>) -> Self {
        Self::new(name, NodeKind::Internal, cubes)
    }

    pub fn cubes(&self) -> &[ProductCube] {
        &self.cubes
    }

    pub fn fanins(&self) -> BTreeSet<NodeId> {
        let mut fanins = BTreeSet::new();
        for cube in &self.cubes {
            for literal in cube.literals() {
                fanins.insert(literal.node);
            }
        }
        fanins
    }

    pub fn fanin_count(&self) -> usize {
        self.fanins().len()
    }

    pub fn cube_count(&self) -> usize {
        self.cubes.len()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AoNetwork {
    nodes: Vec<BooleanNode>,
}

impl AoNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: BooleanNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> XlnCubeResult<&BooleanNode> {
        self.nodes.get(id.0).ok_or(XlnCubeError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> XlnCubeResult<&mut BooleanNode> {
        self.nodes
            .get_mut(id.0)
            .ok_or(XlnCubeError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[BooleanNode] {
        &self.nodes
    }

    pub fn dfs_order(&self) -> Vec<NodeId> {
        (0..self.nodes.len()).map(NodeId).collect()
    }

    fn replace_node(&mut self, id: NodeId, replacement: BooleanNode) -> XlnCubeResult<()> {
        let existing = self.node_mut(id)?;
        existing.cubes = replacement.cubes;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AoMapReport {
    pub extracted_and_nodes: Vec<NodeId>,
    pub materialized_tlus: Vec<NodeId>,
    pub replaced_nodes: Vec<NodeId>,
}

impl AoMapReport {
    fn append(&mut self, other: AoMapReport) {
        self.extracted_and_nodes.extend(other.extracted_and_nodes);
        self.materialized_tlus.extend(other.materialized_tlus);
        self.replaced_nodes.extend(other.replaced_nodes);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnCubeError {
    UnknownNode(NodeId),
    UnknownCube {
        node: NodeId,
        cube: usize,
    },
    InvalidLookupSize(usize),
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for XlnCubeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_cube node {:?}", node),
            Self::UnknownCube { node, cube } => {
                write!(f, "unknown cube {cube} for xln_cube node {:?}", node)
            }
            Self::InvalidLookupSize(size) => {
                write!(f, "lookup-table size must be positive, got {size}")
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

impl Error for XlnCubeError {}

pub type XlnCubeResult<T> = Result<T, XlnCubeError>;

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn network_ao_map_blocked<Network>(
    _network: &mut Network,
    _size: usize,
) -> XlnCubeResult<AoMapReport> {
    Err(XlnCubeError::MissingNativePorts {
        operation: "xln_network_ao_map SIS integration",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn node_ao_map_blocked<Node>(_node: &mut Node, _size: usize) -> XlnCubeResult<AoMapReport> {
    Err(XlnCubeError::MissingNativePorts {
        operation: "xln_node_ao_map SIS integration",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn network_ao_map(network: &mut AoNetwork, size: usize) -> XlnCubeResult<AoMapReport> {
    validate_size(size)?;
    let mut report = AoMapReport::default();
    for node in network.dfs_order() {
        report.append(node_ao_map(network, node, size)?);
    }
    Ok(report)
}

pub fn node_ao_map(
    network: &mut AoNetwork,
    node: NodeId,
    size: usize,
) -> XlnCubeResult<AoMapReport> {
    validate_size(size)?;
    let node_ref = network.node(node)?;
    if node_ref.kind != NodeKind::Internal || node_ref.fanin_count() <= size {
        return Ok(AoMapReport::default());
    }

    let mut report = AoMapReport::default();
    loop {
        let mut node_changed = false;
        let cube_count = network.node(node)?.cube_count();
        for cube in 0..cube_count {
            let extracted = extract_big_ands_from_cube(network, node, cube, size)?;
            if !extracted.is_empty() {
                node_changed = true;
                report.extracted_and_nodes.extend(extracted);
            }
        }
        if !node_changed {
            break;
        }
    }

    report.append(ao_replace_node_with_tlus(network, node, size)?);
    Ok(report)
}

pub fn extract_big_ands_from_cube(
    network: &mut AoNetwork,
    node: NodeId,
    cube_index: usize,
    size: usize,
) -> XlnCubeResult<Vec<NodeId>> {
    validate_size(size)?;
    let cube =
        network
            .node(node)?
            .cubes()
            .get(cube_index)
            .cloned()
            .ok_or(XlnCubeError::UnknownCube {
                node,
                cube: cube_index,
            })?;
    if cube.literal_count() <= size {
        return Ok(Vec::new());
    }

    let mut extracted_nodes = Vec::new();
    let mut one_literals = Vec::new();
    let mut zero_literals = Vec::new();
    let mut count = 0;

    for literal in cube.literals() {
        match literal.phase {
            LiteralPhase::Positive => one_literals.push(literal.node),
            LiteralPhase::Negative => zero_literals.push(literal.node),
        }
        count += 1;

        if count == size {
            let product = make_big_and(&one_literals, &zero_literals);
            let product_literals = product.cubes()[0].literals().to_vec();
            let and_id = network.add_node(product);
            substitute_product_node(network, node, &product_literals, and_id)?;
            extracted_nodes.push(and_id);

            one_literals.clear();
            zero_literals.clear();
            count = 0;
        }
    }

    Ok(extracted_nodes)
}

pub fn make_big_and(one_nodes: &[NodeId], zero_nodes: &[NodeId]) -> BooleanNode {
    let literals = one_nodes
        .iter()
        .copied()
        .map(CubeLiteral::positive)
        .chain(zero_nodes.iter().copied().map(CubeLiteral::negative));
    BooleanNode::internal("$and", vec![ProductCube::new(literals)])
}

pub fn ao_replace_node_with_tlus(
    network: &mut AoNetwork,
    node: NodeId,
    size: usize,
) -> XlnCubeResult<AoMapReport> {
    validate_size(size)?;
    if network.node(node)?.fanin_count() <= size {
        return Ok(AoMapReport::default());
    }

    let mut report = AoMapReport::default();
    let mut cubevec: Vec<BooleanNode> = network
        .node(node)?
        .cubes()
        .iter()
        .cloned()
        .map(|cube| BooleanNode::internal("$cube", vec![cube]))
        .collect();
    let mut tluvec = Vec::new();

    while !cubevec.is_empty() {
        cubevec.sort_by(ao_compare);
        let (next_cubevec, next_tluvec, materialized) =
            make_tlus(network, node, cubevec, tluvec, size)?;
        cubevec = next_cubevec;
        tluvec = next_tluvec;
        report.materialized_tlus.extend(materialized);
    }

    if !tluvec.is_empty() {
        let replacement = or_nodes(tluvec.clone());
        network.replace_node(node, replacement)?;
        report.replaced_nodes.push(node);
    }

    Ok(report)
}

pub fn make_tlus(
    network: &mut AoNetwork,
    node: NodeId,
    cubevec: Vec<BooleanNode>,
    mut tluvec: Vec<BooleanNode>,
    size: usize,
) -> XlnCubeResult<(Vec<BooleanNode>, Vec<BooleanNode>, Vec<NodeId>)> {
    validate_size(size)?;
    for cube_node in cubevec {
        if is_cube_absorbed(&cube_node, &mut tluvec, size).is_none() {
            tluvec.push(cube_node);
        }
    }

    tluvec.sort_by(ao_compare);
    if tluvec.len() == 1 {
        network.replace_node(node, tluvec.remove(0))?;
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    }

    let mut cubevec = Vec::new();
    let mut materialized = Vec::new();
    let split = tluvec
        .iter()
        .position(|tlu| tlu.fanin_count() < size)
        .unwrap_or(tluvec.len());
    let remaining_tlus = tluvec.split_off(split);
    for tlu in tluvec {
        let tlu_id = network.add_node(tlu);
        cubevec.push(BooleanNode::internal(
            "$tlu_literal",
            vec![ProductCube::new([CubeLiteral::positive(tlu_id)])],
        ));
        materialized.push(tlu_id);
    }

    Ok((cubevec, remaining_tlus, materialized))
}

pub fn ao_compare(left: &BooleanNode, right: &BooleanNode) -> std::cmp::Ordering {
    right.fanin_count().cmp(&left.fanin_count())
}

pub fn is_cube_absorbed(
    cube_node: &BooleanNode,
    tluvec: &mut [BooleanNode],
    size: usize,
) -> Option<usize> {
    let mut best = None;
    let mut best_composite_fanin = usize::MAX;

    for (index, tlu) in tluvec.iter().enumerate() {
        let composite = num_composite_fanin(cube_node, tlu);
        if composite <= size && composite < best_composite_fanin {
            best = Some(index);
            best_composite_fanin = composite;
        }
    }

    let index = best?;
    tluvec[index] = or_nodes([cube_node.clone(), tluvec[index].clone()]);
    Some(index)
}

pub fn num_composite_fanin(left: &BooleanNode, right: &BooleanNode) -> usize {
    left.fanins().union(&right.fanins()).count()
}

fn validate_size(size: usize) -> XlnCubeResult<()> {
    if size == 0 {
        Err(XlnCubeError::InvalidLookupSize(size))
    } else {
        Ok(())
    }
}

fn substitute_product_node(
    network: &mut AoNetwork,
    node: NodeId,
    product_literals: &[CubeLiteral],
    replacement: NodeId,
) -> XlnCubeResult<bool> {
    let mut changed = false;
    for cube in &mut network.node_mut(node)?.cubes {
        if cube.contains_all(product_literals) {
            cube.replace_literals_with_node(product_literals, replacement);
            changed = true;
        }
    }
    Ok(changed)
}

fn or_nodes(nodes: impl IntoIterator<Item = BooleanNode>) -> BooleanNode {
    let mut cubes = Vec::new();
    for node in nodes {
        cubes.extend(node.cubes);
    }
    BooleanNode::internal("$or", cubes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(network: &mut AoNetwork, names: &[&str]) -> Vec<NodeId> {
        names
            .iter()
            .map(|name| network.add_node(BooleanNode::primary_input(*name)))
            .collect()
    }

    #[test]
    fn make_big_and_preserves_positive_then_negative_literal_phases() {
        let node = make_big_and(&[NodeId(1), NodeId(2)], &[NodeId(3)]);

        assert_eq!(
            node.cubes()[0].literals(),
            &[
                CubeLiteral::positive(NodeId(1)),
                CubeLiteral::positive(NodeId(2)),
                CubeLiteral::negative(NodeId(3)),
            ]
        );
    }

    #[test]
    fn extract_big_ands_splits_full_size_groups_and_substitutes_them() {
        let mut network = AoNetwork::new();
        let ids = inputs(&mut network, &["a", "b", "c", "d", "e"]);
        let root = network.add_node(BooleanNode::internal(
            "root",
            vec![ProductCube::new(
                ids.iter().copied().map(CubeLiteral::positive),
            )],
        ));

        let extracted = extract_big_ands_from_cube(&mut network, root, 0, 2).unwrap();

        assert_eq!(extracted, vec![NodeId(6), NodeId(7)]);
        assert_eq!(network.node(NodeId(6)).unwrap().fanin_count(), 2);
        assert_eq!(network.node(NodeId(7)).unwrap().fanin_count(), 2);
        assert_eq!(
            network.node(root).unwrap().cubes()[0].literals(),
            &[
                CubeLiteral::positive(ids[4]),
                CubeLiteral::positive(NodeId(6)),
                CubeLiteral::positive(NodeId(7)),
            ]
        );
    }

    #[test]
    fn node_ao_map_repeats_extraction_until_cube_fits() {
        let mut network = AoNetwork::new();
        let ids = inputs(&mut network, &["a", "b", "c", "d", "e"]);
        let root = network.add_node(BooleanNode::internal(
            "root",
            vec![ProductCube::new(
                ids.iter().copied().map(CubeLiteral::positive),
            )],
        ));

        let report = node_ao_map(&mut network, root, 2).unwrap();

        assert_eq!(
            report.extracted_and_nodes,
            vec![NodeId(6), NodeId(7), NodeId(8)]
        );
        assert_eq!(network.node(root).unwrap().fanin_count(), 2);
    }

    #[test]
    fn cube_absorption_uses_best_fit_composite_fanin() {
        let cube = BooleanNode::internal(
            "cube",
            vec![ProductCube::new([
                CubeLiteral::positive(NodeId(1)),
                CubeLiteral::positive(NodeId(2)),
            ])],
        );
        let loose = BooleanNode::internal(
            "loose",
            vec![ProductCube::new([
                CubeLiteral::positive(NodeId(1)),
                CubeLiteral::positive(NodeId(3)),
                CubeLiteral::positive(NodeId(4)),
            ])],
        );
        let tight = BooleanNode::internal(
            "tight",
            vec![ProductCube::new([
                CubeLiteral::positive(NodeId(1)),
                CubeLiteral::positive(NodeId(2)),
                CubeLiteral::positive(NodeId(3)),
            ])],
        );
        let mut tlus = vec![loose, tight];

        assert_eq!(is_cube_absorbed(&cube, &mut tlus, 4), Some(1));
        assert_eq!(tlus[1].cube_count(), 2);
    }

    #[test]
    fn make_tlus_materializes_full_lookup_tables_as_single_literal_cubes() {
        let mut network = AoNetwork::new();
        let ids = inputs(&mut network, &["a", "b", "c", "d"]);
        let root = network.add_node(BooleanNode::internal("root", Vec::new()));
        let cubevec = vec![
            BooleanNode::internal(
                "abc",
                vec![ProductCube::new(
                    ids[0..3].iter().copied().map(CubeLiteral::positive),
                )],
            ),
            BooleanNode::internal("d", vec![ProductCube::new([CubeLiteral::positive(ids[3])])]),
        ];

        let (cubevec, tluvec, materialized) =
            make_tlus(&mut network, root, cubevec, Vec::new(), 3).unwrap();

        assert_eq!(materialized, vec![NodeId(5)]);
        assert_eq!(cubevec.len(), 1);
        assert_eq!(
            cubevec[0].cubes()[0].literals(),
            &[CubeLiteral::positive(NodeId(5))]
        );
        assert_eq!(tluvec.len(), 1);
    }

    #[test]
    fn blocked_entries_report_dependency_beads_and_sources() {
        let mut network = ();
        let error = network_ao_map_blocked(&mut network, 5).unwrap_err();

        let XlnCubeError::MissingNativePorts {
            operation,
            dependencies,
        } = error
        else {
            panic!("expected missing dependency error");
        };

        assert_eq!(operation, "xln_network_ao_map SIS integration");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.376"
                && dependency.source_file == "LogicSynthesis/sis/pld/pld_util.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.390"
                && dependency.source_file == "LogicSynthesis/sis/pld/xln_level.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.325"
                && dependency.source_file == "LogicSynthesis/sis/node/substitute.c"
        }));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_cube.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
