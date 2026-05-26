//! Native Rust model for `LogicSynthesis/sis/pld/xln_filter.c`.
//!
//! The C file looks for SIS nodes that can be absorbed into their fanins by
//! trying every fanin partition and then delegating the accepted bound set to
//! Roth-Karp decomposition. This port keeps the deterministic filter behavior
//! native: node/fanin modelling, fanin-union counts, binary partition
//! generation, internal-node accounting, and alpha-bound calculation. The
//! SIS-mutating entry points remain explicit dependency errors until the node,
//! network, and decomposition ports can supply a real native integration.

use std::collections::HashSet;
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
}

impl FilterNode {
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FilterNetwork {
    nodes: Vec<FilterNode>,
}

impl FilterNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: FilterNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&FilterNode, XlnFilterError> {
        self.nodes.get(id.0).ok_or(XlnFilterError::UnknownNode(id))
    }

    pub fn fanouts(&self, id: NodeId) -> Result<Vec<NodeId>, XlnFilterError> {
        self.node(id)?;
        Ok(self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| node.fanins.contains(&id).then_some(NodeId(index)))
            .collect())
    }

    pub fn fanout_count(&self, id: NodeId) -> Result<usize, XlnFilterError> {
        Ok(self.fanouts(id)?.len())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaninPartition {
    pub y: Vec<NodeId>,
    pub z: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbsorptionCandidate {
    pub combination: usize,
    pub partition: FaninPartition,
    pub num_union_fanin_y: usize,
    pub num_union_fanin_z: usize,
    pub num_internal_y: usize,
    pub num_internal_z: usize,
    pub bound1: isize,
    pub bound2: isize,
    pub bound_alphas: isize,
    pub lambda_indices: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnFilterError {
    UnknownNode(NodeId),
    InvalidSupport {
        support: usize,
    },
    TooManyFanins {
        fanin_count: usize,
    },
    InvalidCombination {
        combination: usize,
        fanin_count: usize,
    },
    NodeMustBeInternal(NodeId),
    MissingNativePorts {
        operation: &'static str,
    },
}

impl fmt::Display for XlnFilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_filter node {:?}", node),
            Self::InvalidSupport { support } => {
                write!(f, "xln_filter support must be positive, got {support}")
            }
            Self::TooManyFanins { fanin_count } => write!(
                f,
                "xln_filter only enumerates up to 31 fanins, got {fanin_count}"
            ),
            Self::InvalidCombination {
                combination,
                fanin_count,
            } => write!(
                f,
                "combination {combination} cannot be encoded in {fanin_count} fanin bits"
            ),
            Self::NodeMustBeInternal(node) => {
                write!(f, "xln_filter node {:?} must be internal", node)
            }
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for XlnFilterError {}

pub fn xln_absorb_nodes_in_fanins_blocked<Network>(
    _network: &mut Network,
    _support: usize,
) -> Result<(), XlnFilterError> {
    missing_native_ports("xln_absorb_nodes_in_fanins")
}

pub fn xln_can_node_be_absorbed_in_fanins_blocked<Node>(
    _node: &Node,
    _support: usize,
) -> Result<(), XlnFilterError> {
    missing_native_ports("xln_can_node_be_absorbed_in_fanins")
}

fn missing_native_ports(operation: &'static str) -> Result<(), XlnFilterError> {
    Err(XlnFilterError::MissingNativePorts { operation })
}

pub fn can_node_be_absorbed_in_fanins_with<F>(
    network: &FilterNetwork,
    node: NodeId,
    support: usize,
    mut decompose: F,
) -> Result<Option<AbsorptionCandidate>, XlnFilterError>
where
    F: FnMut(&[usize], isize) -> bool,
{
    if support == 0 {
        return Err(XlnFilterError::InvalidSupport { support });
    }

    let node_ref = network.node(node)?;
    if node_ref.kind != NodeKind::Internal {
        return Err(XlnFilterError::NodeMustBeInternal(node));
    }

    let num_fanin = node_ref.fanins.len();
    let support_x_2 = 2 * support;
    if num_fanin >= support_x_2 {
        return Ok(None);
    }

    let mut has_internal_fanin = false;
    for fanin in node_ref.fanins.iter().copied() {
        if network.node(fanin)?.kind == NodeKind::Internal {
            if network.fanout_count(fanin)? > 1 {
                return Ok(None);
            }
            has_internal_fanin = true;
        }
    }
    if !has_internal_fanin {
        return Ok(None);
    }

    let faninvec = get_array_of_fanins(network, node)?;
    if num_union_of_fanins(network, &faninvec)? > support_x_2 {
        return Ok(None);
    }

    for combination in 0..num_combinations(num_fanin)? {
        let partition = generate_fanin_combination(network, node, combination)?;
        if partition.y.len() <= 1 || partition.z.is_empty() {
            continue;
        }

        let num_union_fanin_y = num_union_of_fanins(network, &partition.y)?;
        let num_union_fanin_z = num_union_of_fanins(network, &partition.z)?;
        if num_union_fanin_y > support || num_union_fanin_z > support {
            continue;
        }

        let num_internal_y = num_internal_nodes(network, &partition.y)?;
        let num_internal_z = num_internal_nodes(network, &partition.z)?;
        let bound1 = support as isize - num_union_fanin_z as isize;
        let bound2 = (num_internal_y + num_internal_z) as isize - 1;
        let bound_alphas = bound1.min(bound2);
        let lambda_indices = array_to_indices(network, node, &partition.y)?;

        if decompose(&lambda_indices, bound_alphas) {
            return Ok(Some(AbsorptionCandidate {
                combination,
                partition,
                num_union_fanin_y,
                num_union_fanin_z,
                num_internal_y,
                num_internal_z,
                bound1,
                bound2,
                bound_alphas,
                lambda_indices,
            }));
        }
    }

    Ok(None)
}

pub fn first_filter_passing_partition(
    network: &FilterNetwork,
    node: NodeId,
    support: usize,
) -> Result<Option<AbsorptionCandidate>, XlnFilterError> {
    can_node_be_absorbed_in_fanins_with(network, node, support, |_lambda_indices, _bound| true)
}

pub fn get_array_of_fanins(
    network: &FilterNetwork,
    node: NodeId,
) -> Result<Vec<NodeId>, XlnFilterError> {
    let mut fanins = Vec::new();
    for fanin in network.node(node)?.fanins.iter().copied() {
        if network.node(fanin)?.kind == NodeKind::PrimaryInput {
            push_unique(&mut fanins, fanin);
        } else {
            for second_order in network.node(fanin)?.fanins.iter().copied() {
                network.node(second_order)?;
                push_unique(&mut fanins, second_order);
            }
        }
    }
    Ok(fanins)
}

pub fn num_union_of_fanins(
    network: &FilterNetwork,
    nodevec: &[NodeId],
) -> Result<usize, XlnFilterError> {
    let mut union = HashSet::new();
    for node in nodevec.iter().copied() {
        let node_ref = network.node(node)?;
        if node_ref.kind == NodeKind::PrimaryInput {
            union.insert(node);
        } else {
            for fanin in node_ref.fanins.iter().copied() {
                network.node(fanin)?;
                union.insert(fanin);
            }
        }
    }
    Ok(union.len())
}

pub fn generate_all_fanin_combinations(
    network: &FilterNetwork,
    node: NodeId,
) -> Result<Vec<FaninPartition>, XlnFilterError> {
    let fanin_count = network.node(node)?.fanins.len();
    let mut partitions = Vec::with_capacity(num_combinations(fanin_count)?);
    for combination in 0..num_combinations(fanin_count)? {
        partitions.push(generate_fanin_combination(network, node, combination)?);
    }
    Ok(partitions)
}

pub fn generate_fanin_combination(
    network: &FilterNetwork,
    node: NodeId,
    combination: usize,
) -> Result<FaninPartition, XlnFilterError> {
    let fanins = &network.node(node)?.fanins;
    let encoded = xl_binary1(combination, fanins.len())?;
    let mut y = Vec::new();
    let mut z = Vec::new();

    for (fanin, bit) in fanins.iter().copied().zip(encoded) {
        if bit {
            z.push(fanin);
        } else {
            y.push(fanin);
        }
    }

    Ok(FaninPartition { y, z })
}

pub fn format_absorb_node(
    network: &FilterNetwork,
    node: NodeId,
    partition: &FaninPartition,
) -> Result<String, XlnFilterError> {
    let node_ref = network.node(node)?;
    let mut output = format!(
        "candidate node for absorption => {}\n---total number of fanins = {}\n",
        node_ref.name,
        node_ref.fanins.len()
    );
    output.push_str(&format!(
        "bound set: number of nodes = {}\n",
        partition.y.len()
    ));
    for fanin in &partition.y {
        output.push_str(&format!(" {}, ", network.node(*fanin)?.name));
    }
    output.push_str("\nfree set: number of nodes = ");
    output.push_str(&partition.z.len().to_string());
    output.push('\n');
    for fanin in &partition.z {
        output.push_str(&format!(" {}, ", network.node(*fanin)?.name));
    }
    output.push('\n');
    Ok(output)
}

pub fn num_internal_nodes(
    network: &FilterNetwork,
    nodevec: &[NodeId],
) -> Result<usize, XlnFilterError> {
    let mut sum = 0;
    for node in nodevec.iter().copied() {
        if network.node(node)?.kind == NodeKind::Internal {
            sum += 1;
        }
    }
    Ok(sum)
}

pub fn array_to_indices(
    network: &FilterNetwork,
    node: NodeId,
    selected: &[NodeId],
) -> Result<Vec<usize>, XlnFilterError> {
    let fanins = &network.node(node)?.fanins;
    let selected = selected.iter().copied().collect::<HashSet<_>>();
    Ok(fanins
        .iter()
        .enumerate()
        .filter_map(|(index, fanin)| selected.contains(fanin).then_some(index))
        .collect())
}

pub fn xl_binary1(value: usize, length: usize) -> Result<Vec<bool>, XlnFilterError> {
    if length > 31 {
        return Err(XlnFilterError::TooManyFanins {
            fanin_count: length,
        });
    }
    if length < usize::BITS as usize && value >= (1usize << length) {
        return Err(XlnFilterError::InvalidCombination {
            combination: value,
            fanin_count: length,
        });
    }

    Ok((0..length)
        .rev()
        .map(|bit| ((value >> bit) & 1) == 1)
        .collect())
}

fn num_combinations(fanin_count: usize) -> Result<usize, XlnFilterError> {
    if fanin_count > 31 {
        return Err(XlnFilterError::TooManyFanins { fanin_count });
    }
    Ok(1usize << fanin_count)
}

fn push_unique(values: &mut Vec<NodeId>, value: NodeId) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> (
        FilterNetwork,
        NodeId,
        NodeId,
        NodeId,
        NodeId,
        NodeId,
        NodeId,
    ) {
        let mut network = FilterNetwork::new();
        let a = network.add_node(FilterNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(FilterNode::new("b", NodeKind::PrimaryInput));
        let c = network.add_node(FilterNode::new("c", NodeKind::PrimaryInput));
        let x = network.add_node(FilterNode::new("x", NodeKind::Internal).with_fanins(vec![a, b]));
        let y = network.add_node(FilterNode::new("y", NodeKind::Internal).with_fanins(vec![b, c]));
        let f =
            network.add_node(FilterNode::new("f", NodeKind::Internal).with_fanins(vec![x, y, c]));
        (network, a, b, c, x, y, f)
    }

    fn names(network: &FilterNetwork, ids: &[NodeId]) -> Vec<String> {
        ids.iter()
            .map(|id| network.node(*id).unwrap().name.clone())
            .collect()
    }

    #[test]
    fn binary_encoding_and_partition_order_match_xl_binary1() {
        let (network, _a, _b, c, x, y, f) = sample_network();

        assert_eq!(xl_binary1(3, 4), Ok(vec![false, false, true, true]));
        assert_eq!(
            generate_fanin_combination(&network, f, 1).unwrap(),
            FaninPartition {
                y: vec![x, y],
                z: vec![c],
            }
        );
        assert_eq!(
            generate_fanin_combination(&network, f, 5).unwrap(),
            FaninPartition {
                y: vec![y],
                z: vec![x, c],
            }
        );
    }

    #[test]
    fn generates_all_fanin_partitions_in_c_enumeration_order() {
        let (network, _a, _b, _c, _x, _y, f) = sample_network();

        let partitions = generate_all_fanin_combinations(&network, f).unwrap();

        assert_eq!(partitions.len(), 8);
        assert_eq!(names(&network, &partitions[0].y), vec!["x", "y", "c"]);
        assert_eq!(partitions[0].z, Vec::<NodeId>::new());
        assert_eq!(partitions[7].y, Vec::<NodeId>::new());
        assert_eq!(names(&network, &partitions[7].z), vec!["x", "y", "c"]);
    }

    #[test]
    fn union_of_fanins_counts_primary_inputs_as_themselves() {
        let (network, a, b, c, x, y, _f) = sample_network();

        assert_eq!(num_union_of_fanins(&network, &[a, x]).unwrap(), 2);
        assert_eq!(num_union_of_fanins(&network, &[x, y]).unwrap(), 3);
        assert_eq!(num_union_of_fanins(&network, &[c]).unwrap(), 1);
        assert_eq!(get_array_of_fanins(&network, y).unwrap(), vec![b, c]);
    }

    #[test]
    fn internal_node_count_and_lambda_indices_follow_fanin_order() {
        let (network, _a, _b, c, x, y, f) = sample_network();

        assert_eq!(num_internal_nodes(&network, &[x, y, c]).unwrap(), 2);
        assert_eq!(array_to_indices(&network, f, &[y, x]).unwrap(), vec![0, 1]);
    }

    #[test]
    fn filter_rejects_high_support_all_pi_and_multi_fanout_internal_fanins() {
        let (network, _a, _b, _c, _x, _y, f) = sample_network();
        assert_eq!(
            can_node_be_absorbed_in_fanins_with(&network, f, 1, |_, _| true).unwrap(),
            None
        );

        let mut pi_only = FilterNetwork::new();
        let a = pi_only.add_node(FilterNode::new("a", NodeKind::PrimaryInput));
        let b = pi_only.add_node(FilterNode::new("b", NodeKind::PrimaryInput));
        let out =
            pi_only.add_node(FilterNode::new("out", NodeKind::Internal).with_fanins(vec![a, b]));
        assert_eq!(
            can_node_be_absorbed_in_fanins_with(&pi_only, out, 3, |_, _| true).unwrap(),
            None
        );

        let mut multi = network.clone();
        multi.add_node(FilterNode::new("g", NodeKind::Internal).with_fanins(vec![NodeId(3)]));
        assert_eq!(
            can_node_be_absorbed_in_fanins_with(&multi, f, 3, |_, _| true).unwrap(),
            None
        );
    }

    #[test]
    fn accepted_partition_reports_bounds_and_decomposition_inputs() {
        let (network, _a, _b, _c, _x, _y, f) = sample_network();
        let mut calls = Vec::new();

        let candidate = can_node_be_absorbed_in_fanins_with(&network, f, 3, |lambda, bound| {
            calls.push((lambda.to_vec(), bound));
            lambda == [0, 1] && bound == 1
        })
        .unwrap()
        .unwrap();

        assert_eq!(candidate.combination, 1);
        assert_eq!(names(&network, &candidate.partition.y), vec!["x", "y"]);
        assert_eq!(names(&network, &candidate.partition.z), vec!["c"]);
        assert_eq!(candidate.num_union_fanin_y, 3);
        assert_eq!(candidate.num_union_fanin_z, 1);
        assert_eq!(candidate.num_internal_y, 2);
        assert_eq!(candidate.num_internal_z, 0);
        assert_eq!(candidate.bound1, 2);
        assert_eq!(candidate.bound2, 1);
        assert_eq!(candidate.bound_alphas, 1);
        assert_eq!(candidate.lambda_indices, vec![0, 1]);
        assert_eq!(calls, vec![(vec![0, 1], 1)]);
    }

    #[test]
    fn decomposition_oracle_can_reject_filter_passing_partitions() {
        let (network, _a, _b, _c, _x, _y, f) = sample_network();

        assert_eq!(
            can_node_be_absorbed_in_fanins_with(&network, f, 3, |_lambda, _bound| false).unwrap(),
            None
        );
        assert!(
            first_filter_passing_partition(&network, f, 3)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn debug_format_matches_c_output_shape() {
        let (network, _a, _b, c, x, y, f) = sample_network();
        let partition = FaninPartition {
            y: vec![x, y],
            z: vec![c],
        };

        assert_eq!(
            format_absorb_node(&network, f, &partition).unwrap(),
            concat!(
                "candidate node for absorption => f\n",
                "---total number of fanins = 3\n",
                "bound set: number of nodes = 2\n",
                " x,  y, \n",
                "free set: number of nodes = 1\n",
                " c, \n"
            )
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_filter.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
