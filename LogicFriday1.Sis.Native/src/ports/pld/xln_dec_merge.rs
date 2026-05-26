//! Native Rust model for `LogicSynthesis/sis/pld/xln_dec_merge.c`.
//!
//! The C file chooses infeasible PLD nodes, ranks pairs by common fanins,
//! explodes covers into cube nodes, and plans extractable mergeable
//! subfunctions before mutating SIS `network_t`/`node_t` state. This port keeps
//! those deterministic choices in owned Rust data. Direct SIS graph mutation
//! remains gated by explicit dependency errors until the prerequisite ports are
//! available.

use std::collections::{HashMap, HashSet};
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiteralPhase {
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CubeLiteral {
    pub fanin: NodeId,
    pub phase: LiteralPhase,
}

impl CubeLiteral {
    pub const fn positive(fanin: NodeId) -> Self {
        Self {
            fanin,
            phase: LiteralPhase::Positive,
        }
    }

    pub const fn negative(fanin: NodeId) -> Self {
        Self {
            fanin,
            phase: LiteralPhase::Negative,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    literals: Vec<CubeLiteral>,
}

impl Cube {
    pub fn new(literals: Vec<CubeLiteral>) -> Result<Self, XlnDecMergeError> {
        let mut seen = HashSet::new();
        for literal in &literals {
            if !seen.insert(literal.fanin) {
                return Err(XlnDecMergeError::DuplicateCubeFanin {
                    fanin: literal.fanin,
                });
            }
        }
        Ok(Self { literals })
    }

    pub fn literals(&self) -> &[CubeLiteral] {
        &self.literals
    }

    pub fn fanins(&self) -> Vec<NodeId> {
        self.literals.iter().map(|literal| literal.fanin).collect()
    }

    pub fn fanin_count(&self) -> usize {
        self.literals.len()
    }

    fn literal_for(&self, fanin: NodeId) -> Option<CubeLiteral> {
        self.literals
            .iter()
            .copied()
            .find(|literal| literal.fanin == fanin)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecMergeNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub cubes: Vec<Cube>,
}

impl DecMergeNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }

    pub fn with_cubes(
        mut self,
        fanins: Vec<NodeId>,
        cubes: Vec<Cube>,
    ) -> Result<Self, XlnDecMergeError> {
        let fanin_set: HashSet<NodeId> = fanins.iter().copied().collect();
        for cube in &cubes {
            for literal in cube.literals() {
                if !fanin_set.contains(&literal.fanin) {
                    return Err(XlnDecMergeError::CubeFaninNotInNode {
                        fanin: literal.fanin,
                    });
                }
            }
        }
        self.fanins = fanins;
        self.cubes = cubes;
        Ok(self)
    }

    pub fn fanin_count(&self) -> usize {
        self.fanins.len()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DecMergeNetwork {
    nodes: Vec<DecMergeNode>,
}

impl DecMergeNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: DecMergeNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&DecMergeNode, XlnDecMergeError> {
        self.nodes
            .get(id.0)
            .ok_or(XlnDecMergeError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[DecMergeNode] {
        &self.nodes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MergeHeuristic {
    AllCubes,
    PairNodes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecMergeOptions {
    pub support: usize,
    pub heuristic: MergeHeuristic,
    pub common_lower_bound: usize,
    pub cube_support_lower_bound: usize,
    pub max_common_fanin: usize,
    pub max_fanin: usize,
    pub max_union_fanin: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CubeNodeId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Affinity {
    pub node1: NodeId,
    pub node2: NodeId,
    pub common: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeAffinity {
    pub cube_node1: CubeNodeId,
    pub cube_node2: CubeNodeId,
    pub common: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeTables {
    pub cube_nodes: Vec<DecMergeNode>,
    pub cube_to_node: Vec<NodeId>,
    pub node_to_cubes: HashMap<NodeId, Vec<CubeNodeId>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedMergePair {
    pub source1: NodeId,
    pub source2: NodeId,
    pub sub_node1: DecMergeNode,
    pub sub_node2: DecMergeNode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecMergePlan {
    pub infeasible_nodes: Vec<NodeId>,
    pub matched_node_pairs: Vec<Affinity>,
    pub cube_tables: CubeTables,
    pub matched_cube_pairs: Vec<CubeAffinity>,
    pub extracted_pairs: Vec<ExtractedMergePair>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnDecMergeError {
    UnknownNode(NodeId),
    UnknownCubeNode(CubeNodeId),
    DuplicateCubeFanin {
        fanin: NodeId,
    },
    CubeFaninNotInNode {
        fanin: NodeId,
    },
    MissingCubeOrigin(CubeNodeId),
    MissingNodeCubeVector(NodeId),
    AmbiguousInputPhase {
        node: String,
        fanin: NodeId,
    },
    MissingInputPhase {
        node: String,
        fanin: NodeId,
    },
    InvalidAddMoreCubeShape {
        sub_node_fanins: usize,
        cube_node_fanins: usize,
    },
    MissingNativePorts {
        operation: &'static str,
    },
}

impl fmt::Display for XlnDecMergeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_dec_merge node {:?}", node),
            Self::UnknownCubeNode(node) => write!(f, "unknown xln_dec_merge cube node {:?}", node),
            Self::DuplicateCubeFanin { fanin } => {
                write!(f, "cube contains duplicate fanin {:?}", fanin)
            }
            Self::CubeFaninNotInNode { fanin } => {
                write!(
                    f,
                    "cube fanin {:?} is not present in the owning node",
                    fanin
                )
            }
            Self::MissingCubeOrigin(cube_node) => {
                write!(f, "cube node {:?} has no source-node mapping", cube_node)
            }
            Self::MissingNodeCubeVector(node) => {
                write!(f, "node {:?} has no node-to-cube vector mapping", node)
            }
            Self::AmbiguousInputPhase { node, fanin } => {
                write!(f, "node {node} has both phases for fanin {:?}", fanin)
            }
            Self::MissingInputPhase { node, fanin } => {
                write!(f, "node {node} has no phase for fanin {:?}", fanin)
            }
            Self::InvalidAddMoreCubeShape {
                sub_node_fanins,
                cube_node_fanins,
            } => write!(
                f,
                "sub-node fanin count {sub_node_fanins} cannot consume cube-node fanin count {cube_node_fanins}"
            ),
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for XlnDecMergeError {}

pub fn xln_decomp_for_merging_network_blocked<Network>(
    _network: &mut Network,
    _options: DecMergeOptions,
) -> Result<DecMergePlan, XlnDecMergeError> {
    missing_native_ports("xln_decomp_for_merging_network against SIS network_t")
}

pub fn apply_merge_plan_to_sis_network_blocked<Network>(
    _network: &mut Network,
    _plan: &DecMergePlan,
) -> Result<(), XlnDecMergeError> {
    missing_native_ports("network_add_node and node_substitute for extracted mergeable functions")
}

fn missing_native_ports<T>(operation: &'static str) -> Result<T, XlnDecMergeError> {
    Err(XlnDecMergeError::MissingNativePorts { operation })
}

pub fn plan_decomp_for_merging_network(
    network: &DecMergeNetwork,
    options: DecMergeOptions,
) -> Result<DecMergePlan, XlnDecMergeError> {
    let infeasible_nodes = xln_infeasible_nodes(network, options.support);
    match options.heuristic {
        MergeHeuristic::AllCubes => plan_decomp_for_merging_all_cubes(
            network,
            infeasible_nodes.clone(),
            infeasible_nodes,
            options,
        ),
        MergeHeuristic::PairNodes => {
            let matched_node_pairs = match_pair_nodes(network, &infeasible_nodes)?;
            let mut combined = CubeTables {
                cube_nodes: Vec::new(),
                cube_to_node: Vec::new(),
                node_to_cubes: HashMap::new(),
            };
            let mut matched_cube_pairs = Vec::new();
            let mut extracted_pairs = Vec::new();

            for affinity in &matched_node_pairs {
                let nodevec = vec![affinity.node1, affinity.node2];
                let tables = xln_fill_tables(network, &nodevec, options)?;
                let mut cube_plan = xln_decomp_for_merge_cube_nodes(network, &tables, options)?;
                let offset = combined.cube_nodes.len();
                append_tables(&mut combined, tables);
                for cube_pair in &mut cube_plan.matched_cube_pairs {
                    cube_pair.cube_node1.0 += offset;
                    cube_pair.cube_node2.0 += offset;
                }
                matched_cube_pairs.extend(cube_plan.matched_cube_pairs);
                extracted_pairs.extend(cube_plan.extracted_pairs);
            }

            Ok(DecMergePlan {
                infeasible_nodes,
                matched_node_pairs,
                cube_tables: combined,
                matched_cube_pairs,
                extracted_pairs,
            })
        }
    }
}

fn plan_decomp_for_merging_all_cubes(
    network: &DecMergeNetwork,
    infeasible_nodes: Vec<NodeId>,
    nodevec: Vec<NodeId>,
    options: DecMergeOptions,
) -> Result<DecMergePlan, XlnDecMergeError> {
    let cube_tables = xln_fill_tables(network, &nodevec, options)?;
    let cube_plan = xln_decomp_for_merge_cube_nodes(network, &cube_tables, options)?;
    Ok(DecMergePlan {
        infeasible_nodes,
        matched_node_pairs: Vec::new(),
        cube_tables,
        matched_cube_pairs: cube_plan.matched_cube_pairs,
        extracted_pairs: cube_plan.extracted_pairs,
    })
}

fn append_tables(target: &mut CubeTables, source: CubeTables) {
    let offset = target.cube_nodes.len();
    target.cube_nodes.extend(source.cube_nodes);
    target.cube_to_node.extend(source.cube_to_node);
    for (node, cubes) in source.node_to_cubes {
        target.node_to_cubes.entry(node).or_default().extend(
            cubes
                .into_iter()
                .map(|cube_node| CubeNodeId(cube_node.0 + offset)),
        );
    }
}

pub fn xln_infeasible_nodes(network: &DecMergeNetwork, support: usize) -> Vec<NodeId> {
    network
        .nodes()
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            (node.kind == NodeKind::Internal && node.fanin_count() > support)
                .then_some(NodeId(index))
        })
        .collect()
}

pub fn match_pair_nodes(
    network: &DecMergeNetwork,
    nodevec: &[NodeId],
) -> Result<Vec<Affinity>, XlnDecMergeError> {
    let mut affinities = Vec::new();
    for (i, node1) in nodevec.iter().copied().enumerate() {
        for node2 in nodevec.iter().copied().skip(i + 1) {
            affinities.push(Affinity {
                node1,
                node2,
                common: xln_node_find_common_inputs(network.node(node1)?, network.node(node2)?),
            });
        }
    }
    sort_affinities(&mut affinities);

    let mut table1 = HashSet::new();
    let mut table2 = HashSet::new();
    let mut matched = Vec::new();
    for affinity in affinities {
        if table1.contains(&affinity.node1) || table2.contains(&affinity.node2) {
            continue;
        }
        table1.insert(affinity.node1);
        table2.insert(affinity.node2);
        matched.push(affinity);
    }
    Ok(matched)
}

pub fn xln_decomp_for_merge_pair_of_nodes(
    network: &DecMergeNetwork,
    n1: NodeId,
    n2: NodeId,
    options: DecMergeOptions,
) -> Result<DecMergePlan, XlnDecMergeError> {
    plan_decomp_for_merging_all_cubes(network, vec![n1, n2], vec![n1, n2], options)
}

pub fn xln_fill_tables(
    network: &DecMergeNetwork,
    nodevec: &[NodeId],
    options: DecMergeOptions,
) -> Result<CubeTables, XlnDecMergeError> {
    let mut cube_nodes = Vec::new();
    let mut cube_to_node = Vec::new();
    let mut node_to_cubes = HashMap::new();

    for node_id in nodevec {
        let node = network.node(*node_id)?;
        if node.kind != NodeKind::Internal {
            continue;
        }
        let mut cubevec = Vec::new();
        for cube in node.cubes.iter().rev() {
            let cube_node = pld_make_node_from_cube(node, cube)?;
            let cube_id = CubeNodeId(cube_nodes.len());
            cube_nodes.push(cube_node);
            cubevec.push(cube_id);
            if cube.fanin_count() >= options.cube_support_lower_bound {
                cube_to_node.push(*node_id);
            } else {
                cube_to_node.push(NodeId(usize::MAX));
            }
        }
        node_to_cubes.insert(*node_id, cubevec);
    }

    let retained = cube_to_node
        .iter()
        .enumerate()
        .filter_map(|(index, source)| (*source != NodeId(usize::MAX)).then_some(index))
        .collect::<HashSet<_>>();
    let mut remap = HashMap::new();
    let mut retained_cube_nodes = Vec::new();
    let mut retained_cube_to_node = Vec::new();
    for index in 0..cube_nodes.len() {
        if retained.contains(&index) {
            remap.insert(CubeNodeId(index), CubeNodeId(retained_cube_nodes.len()));
            retained_cube_nodes.push(cube_nodes[index].clone());
            retained_cube_to_node.push(cube_to_node[index]);
        }
    }
    for cubevec in node_to_cubes.values_mut() {
        *cubevec = cubevec
            .iter()
            .filter_map(|cube_node| remap.get(cube_node).copied())
            .collect();
    }

    Ok(CubeTables {
        cube_nodes: retained_cube_nodes,
        cube_to_node: retained_cube_to_node,
        node_to_cubes,
    })
}

pub fn xln_decomp_for_merge_cube_nodes(
    network: &DecMergeNetwork,
    tables: &CubeTables,
    options: DecMergeOptions,
) -> Result<DecMergePlan, XlnDecMergeError> {
    let mut affinities = Vec::new();
    for i in 0..tables.cube_nodes.len() {
        for j in (i + 1)..tables.cube_nodes.len() {
            let cube_node1 = CubeNodeId(i);
            let cube_node2 = CubeNodeId(j);
            let n1 = cube_origin(tables, cube_node1)?;
            let n2 = cube_origin(tables, cube_node2)?;
            if n1 == n2
                && xln_num_composite_fanin(
                    cube_node(tables, cube_node1)?,
                    cube_node(tables, cube_node2)?,
                ) <= options.support
            {
                continue;
            }

            let common = xln_node_find_common_inputs(
                cube_node(tables, cube_node1)?,
                cube_node(tables, cube_node2)?,
            );
            if common.len() < options.common_lower_bound {
                continue;
            }
            affinities.push(CubeAffinity {
                cube_node1,
                cube_node2,
                common,
            });
        }
    }
    sort_cube_affinities(&mut affinities);

    let mut matched_cubes = HashSet::new();
    let mut matched_cube_pairs = Vec::new();
    let mut extracted_pairs = Vec::new();
    for affinity in affinities {
        if matched_cubes.contains(&affinity.cube_node1)
            || matched_cubes.contains(&affinity.cube_node2)
        {
            continue;
        }
        matched_cubes.insert(affinity.cube_node1);
        matched_cubes.insert(affinity.cube_node2);

        let n1 = cube_origin(tables, affinity.cube_node1)?;
        let n2 = cube_origin(tables, affinity.cube_node2)?;
        let extracted = xln_extract_mergeable_fns_from_cubes(
            network,
            tables,
            &affinity,
            &mut matched_cubes,
            options,
        )?;
        matched_cube_pairs.push(affinity);
        extracted_pairs.extend(extracted.into_iter().map(|(sub_node1, sub_node2)| {
            ExtractedMergePair {
                source1: n1,
                source2: n2,
                sub_node1,
                sub_node2,
            }
        }));
    }

    Ok(DecMergePlan {
        infeasible_nodes: Vec::new(),
        matched_node_pairs: Vec::new(),
        cube_tables: tables.clone(),
        matched_cube_pairs,
        extracted_pairs,
    })
}

pub fn xln_node_find_common_inputs(node1: &DecMergeNode, node2: &DecMergeNode) -> Vec<NodeId> {
    node1
        .fanins
        .iter()
        .copied()
        .filter(|fanin| node2.fanins.contains(fanin))
        .collect()
}

pub fn pld_get_non_common_fanins(node: &DecMergeNode, common: &[NodeId]) -> Vec<NodeId> {
    node.fanins
        .iter()
        .copied()
        .filter(|fanin| !common.contains(fanin))
        .collect()
}

pub fn xln_extract_mergeable_fns_from_cubes(
    network: &DecMergeNetwork,
    tables: &CubeTables,
    affinity: &CubeAffinity,
    matched_cubes: &mut HashSet<CubeNodeId>,
    options: DecMergeOptions,
) -> Result<Vec<(DecMergeNode, DecMergeNode)>, XlnDecMergeError> {
    let node1 = cube_node(tables, affinity.cube_node1)?;
    let node2 = cube_node(tables, affinity.cube_node2)?;
    let n1 = cube_origin(tables, affinity.cube_node1)?;
    let n2 = cube_origin(tables, affinity.cube_node2)?;
    let mut nc1_ptr = 0;
    let mut nc2_ptr = 0;
    let nc1 = pld_get_non_common_fanins(node1, &affinity.common);
    let nc2 = pld_get_non_common_fanins(node2, &affinity.common);
    let mut common_ptr = 0;
    let mut extracted = Vec::new();

    while common_ptr < affinity.common.len() {
        let end = (common_ptr + options.max_common_fanin).min(affinity.common.len());
        let subset_common = affinity.common[common_ptr..end].to_vec();
        common_ptr = end;

        let mut sub_node1 = xln_extract_supercube_from_cube(node1, &subset_common)?;
        let mut sub_node2 = xln_extract_supercube_from_cube(node2, &subset_common)?;
        let num_subset_common = subset_common.len();

        let mut fanin_bound_more1 = options.max_fanin.saturating_sub(num_subset_common);
        let mut fanin_bound_more2 = fanin_bound_more1;
        fanin_bound_more1 = fanin_bound_more1.min(nc1.len() - nc1_ptr);
        fanin_bound_more2 = fanin_bound_more2.min(nc2.len() - nc2_ptr);

        let bound_union_more = options.max_union_fanin.saturating_sub(num_subset_common);
        if bound_union_more < fanin_bound_more1 + fanin_bound_more2 {
            if fanin_bound_more1.min(fanin_bound_more2) >= bound_union_more {
                fanin_bound_more1 = bound_union_more / 2;
                fanin_bound_more2 = bound_union_more - fanin_bound_more1;
            } else if fanin_bound_more1 < fanin_bound_more2 {
                fanin_bound_more2 = (bound_union_more - fanin_bound_more1).min(fanin_bound_more2);
            } else {
                fanin_bound_more1 = (bound_union_more - fanin_bound_more2).min(fanin_bound_more1);
            }
        }

        xln_add_more_literals(&mut sub_node1, node1, fanin_bound_more1, &nc1, nc1_ptr)?;
        xln_add_more_literals(&mut sub_node2, node2, fanin_bound_more2, &nc2, nc2_ptr)?;
        nc1_ptr += fanin_bound_more1;
        nc2_ptr += fanin_bound_more2;

        xln_add_more_cubes(&mut sub_node1, node1, n1, tables, matched_cubes)?;
        xln_add_more_cubes(&mut sub_node2, node2, n2, tables, matched_cubes)?;
        let left_name = format!("{}_merge_{}", network.node(n1)?.name, extracted.len());
        let right_name = format!("{}_merge_{}", network.node(n2)?.name, extracted.len());
        sub_node1.name = left_name;
        sub_node2.name = right_name;
        extracted.push((sub_node1, sub_node2));
    }

    Ok(extracted)
}

pub fn xln_extract_supercube_from_cube(
    node: &DecMergeNode,
    subset_common: &[NodeId],
) -> Result<DecMergeNode, XlnDecMergeError> {
    let mut literals = Vec::with_capacity(subset_common.len());
    for fanin in subset_common {
        literals.push(input_literal(node, *fanin)?);
    }
    let cube = Cube::new(literals)?;
    pld_make_node_from_cube(node, &cube)
}

pub fn xln_add_more_literals(
    sub_node: &mut DecMergeNode,
    node: &DecMergeNode,
    fanin_bound_more: usize,
    nc: &[NodeId],
    nc_ptr: usize,
) -> Result<(), XlnDecMergeError> {
    let final_index = nc_ptr + fanin_bound_more;
    if final_index > nc.len() {
        return Err(XlnDecMergeError::MissingInputPhase {
            node: node.name.clone(),
            fanin: NodeId(final_index),
        });
    }
    let mut cube = sub_node.cubes.first().cloned().unwrap_or(Cube {
        literals: Vec::new(),
    });
    for fanin in &nc[nc_ptr..final_index] {
        cube.literals.push(input_literal(node, *fanin)?);
        sub_node.fanins.push(*fanin);
    }
    sub_node.cubes = vec![Cube::new(cube.literals)?];
    Ok(())
}

pub fn xln_add_more_cubes(
    sub_node: &mut DecMergeNode,
    matched_cube_node: &DecMergeNode,
    source_node: NodeId,
    tables: &CubeTables,
    matched_cubes: &mut HashSet<CubeNodeId>,
) -> Result<(), XlnDecMergeError> {
    let nin_sub_node = sub_node.fanin_count();
    let nin_cube_node = matched_cube_node.fanin_count();
    if nin_sub_node < nin_cube_node {
        return Ok(());
    }
    if nin_sub_node != nin_cube_node {
        return Err(XlnDecMergeError::InvalidAddMoreCubeShape {
            sub_node_fanins: nin_sub_node,
            cube_node_fanins: nin_cube_node,
        });
    }

    let cubenode_vec = tables
        .node_to_cubes
        .get(&source_node)
        .ok_or(XlnDecMergeError::MissingNodeCubeVector(source_node))?;
    for cube_id in cubenode_vec {
        if matched_cubes.contains(cube_id) {
            continue;
        }
        let cube = cube_node(tables, *cube_id)?;
        if pld_is_fanin_subset(cube, sub_node) {
            for cube_cover in &cube.cubes {
                if !sub_node.cubes.contains(cube_cover) {
                    sub_node.cubes.push(cube_cover.clone());
                }
            }
            matched_cubes.insert(*cube_id);
        }
    }
    Ok(())
}

pub fn pld_make_node_from_cube(
    source: &DecMergeNode,
    cube: &Cube,
) -> Result<DecMergeNode, XlnDecMergeError> {
    Ok(DecMergeNode {
        name: format!("{}_cube", source.name),
        kind: NodeKind::Internal,
        fanins: cube.fanins(),
        cubes: vec![cube.clone()],
    })
}

pub fn pld_is_fanin_subset(cube: &DecMergeNode, sub_node: &DecMergeNode) -> bool {
    cube.fanins
        .iter()
        .all(|fanin| sub_node.fanins.contains(fanin))
}

pub fn xln_num_composite_fanin(node1: &DecMergeNode, node2: &DecMergeNode) -> usize {
    let mut fanins = node1.fanins.clone();
    for fanin in &node2.fanins {
        if !fanins.contains(fanin) {
            fanins.push(*fanin);
        }
    }
    fanins.len()
}

pub fn pld_affinity_compare_function(
    left_common: usize,
    right_common: usize,
) -> std::cmp::Ordering {
    right_common.cmp(&left_common)
}

fn sort_affinities(affinities: &mut [Affinity]) {
    affinities.sort_by(|left, right| {
        pld_affinity_compare_function(left.common.len(), right.common.len())
            .then_with(|| left.node1.cmp(&right.node1))
            .then_with(|| left.node2.cmp(&right.node2))
    });
}

fn sort_cube_affinities(affinities: &mut [CubeAffinity]) {
    affinities.sort_by(|left, right| {
        pld_affinity_compare_function(left.common.len(), right.common.len())
            .then_with(|| left.cube_node1.0.cmp(&right.cube_node1.0))
            .then_with(|| left.cube_node2.0.cmp(&right.cube_node2.0))
    });
}

fn input_literal(node: &DecMergeNode, fanin: NodeId) -> Result<CubeLiteral, XlnDecMergeError> {
    let mut phases = node
        .cubes
        .iter()
        .filter_map(|cube| cube.literal_for(fanin).map(|literal| literal.phase));
    let Some(first) = phases.next() else {
        return Err(XlnDecMergeError::MissingInputPhase {
            node: node.name.clone(),
            fanin,
        });
    };
    if phases.any(|phase| phase != first) {
        return Err(XlnDecMergeError::AmbiguousInputPhase {
            node: node.name.clone(),
            fanin,
        });
    }
    Ok(CubeLiteral {
        fanin,
        phase: first,
    })
}

fn cube_node(tables: &CubeTables, id: CubeNodeId) -> Result<&DecMergeNode, XlnDecMergeError> {
    tables
        .cube_nodes
        .get(id.0)
        .ok_or(XlnDecMergeError::UnknownCubeNode(id))
}

fn cube_origin(tables: &CubeTables, id: CubeNodeId) -> Result<NodeId, XlnDecMergeError> {
    tables
        .cube_to_node
        .get(id.0)
        .copied()
        .ok_or(XlnDecMergeError::MissingCubeOrigin(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(heuristic: MergeHeuristic) -> DecMergeOptions {
        DecMergeOptions {
            support: 2,
            heuristic,
            common_lower_bound: 1,
            cube_support_lower_bound: 1,
            max_common_fanin: 2,
            max_fanin: 3,
            max_union_fanin: 4,
        }
    }

    fn cube(literals: &[CubeLiteral]) -> Cube {
        Cube::new(literals.to_vec()).unwrap()
    }

    fn internal(name: &str, fanins: Vec<NodeId>, cubes: Vec<Cube>) -> DecMergeNode {
        DecMergeNode::new(name, NodeKind::Internal)
            .with_cubes(fanins, cubes)
            .unwrap()
    }

    fn sample_network() -> DecMergeNetwork {
        let mut network = DecMergeNetwork::new();
        let a = network.add_node(DecMergeNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(DecMergeNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(DecMergeNode::new("c", NodeKind::PrimaryInput));
        let d = network.add_node(DecMergeNode::new("d", NodeKind::PrimaryInput));
        network.add_node(internal(
            "n1",
            vec![a, b, c],
            vec![
                cube(&[
                    CubeLiteral::positive(a),
                    CubeLiteral::positive(b),
                    CubeLiteral::negative(c),
                ]),
                cube(&[CubeLiteral::positive(a), CubeLiteral::positive(b)]),
            ],
        ));
        network.add_node(internal(
            "n2",
            vec![a, b, d],
            vec![cube(&[
                CubeLiteral::positive(a),
                CubeLiteral::positive(b),
                CubeLiteral::positive(d),
            ])],
        ));
        network.add_node(internal(
            "small",
            vec![a],
            vec![cube(&[CubeLiteral::positive(a)])],
        ));
        network
    }

    #[test]
    fn infeasible_nodes_are_internal_nodes_above_support() {
        let network = sample_network();

        assert_eq!(
            xln_infeasible_nodes(&network, 2),
            vec![NodeId(4), NodeId(5)]
        );
    }

    #[test]
    fn common_inputs_preserve_left_node_fanin_order() {
        let network = sample_network();

        assert_eq!(
            xln_node_find_common_inputs(
                network.node(NodeId(4)).unwrap(),
                network.node(NodeId(5)).unwrap()
            ),
            vec![NodeId(0), NodeId(1)]
        );
    }

    #[test]
    fn pair_node_matching_sorts_by_decreasing_common_fanins() {
        let mut network = sample_network();
        let e = network.add_node(DecMergeNode::new("e", NodeKind::PrimaryInput));
        let n3 = network.add_node(internal(
            "n3",
            vec![NodeId(0), e, NodeId(3)],
            vec![cube(&[
                CubeLiteral::positive(NodeId(0)),
                CubeLiteral::positive(e),
                CubeLiteral::positive(NodeId(3)),
            ])],
        ));

        let matched = match_pair_nodes(&network, &[NodeId(4), NodeId(5), n3]).unwrap();

        assert_eq!(
            matched,
            vec![
                Affinity {
                    node1: NodeId(4),
                    node2: NodeId(5),
                    common: vec![NodeId(0), NodeId(1)],
                },
                Affinity {
                    node1: NodeId(5),
                    node2: n3,
                    common: vec![NodeId(0), NodeId(3)],
                },
            ]
        );
    }

    #[test]
    fn fill_tables_expands_cubes_in_reverse_c_order_and_filters_small_cubes() {
        let network = sample_network();
        let mut options = opts(MergeHeuristic::AllCubes);
        options.cube_support_lower_bound = 3;

        let tables = xln_fill_tables(&network, &[NodeId(4), NodeId(5)], options).unwrap();

        assert_eq!(tables.cube_nodes.len(), 2);
        assert_eq!(tables.cube_to_node, vec![NodeId(4), NodeId(5)]);
        assert_eq!(tables.cube_nodes[0].fanin_count(), 3);
        assert_eq!(
            tables.cube_nodes[0].cubes[0].literals()[2].phase,
            LiteralPhase::Negative
        );
    }

    #[test]
    fn extract_supercube_and_more_literals_preserve_input_phases() {
        let network = sample_network();
        let node = network.node(NodeId(4)).unwrap();
        let mut sub_node = xln_extract_supercube_from_cube(node, &[NodeId(0), NodeId(1)]).unwrap();

        xln_add_more_literals(&mut sub_node, node, 1, &[NodeId(2)], 0).unwrap();

        assert_eq!(
            sub_node.cubes[0].literals(),
            &[
                CubeLiteral::positive(NodeId(0)),
                CubeLiteral::positive(NodeId(1)),
                CubeLiteral::negative(NodeId(2)),
            ]
        );
    }

    #[test]
    fn all_cubes_plan_extracts_mergeable_pairs() {
        let network = sample_network();
        let plan =
            plan_decomp_for_merging_network(&network, opts(MergeHeuristic::AllCubes)).unwrap();

        assert_eq!(plan.infeasible_nodes, vec![NodeId(4), NodeId(5)]);
        assert_eq!(plan.matched_cube_pairs.len(), 1);
        assert_eq!(plan.extracted_pairs.len(), 1);
        assert_eq!(plan.extracted_pairs[0].source1, NodeId(4));
        assert_eq!(plan.extracted_pairs[0].source2, NodeId(4));
        assert_eq!(plan.extracted_pairs[0].sub_node1.fanin_count(), 2);
        assert_eq!(plan.extracted_pairs[0].sub_node2.fanin_count(), 3);
        assert_eq!(plan.extracted_pairs[0].sub_node1.cubes.len(), 1);
    }

    #[test]
    fn pair_nodes_plan_uses_matched_node_pairs_before_cube_extraction() {
        let network = sample_network();
        let plan =
            plan_decomp_for_merging_network(&network, opts(MergeHeuristic::PairNodes)).unwrap();

        assert_eq!(
            plan.matched_node_pairs,
            vec![Affinity {
                node1: NodeId(4),
                node2: NodeId(5),
                common: vec![NodeId(0), NodeId(1)],
            }]
        );
        assert_eq!(plan.extracted_pairs.len(), 1);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_dec_merge.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
