//! Native Rust model for `LogicSynthesis/sis/seqbdd/prl_product.c`.
//!
//! The C file builds product/consistency variable order data, constructs XNOR
//! transition products, and computes next-state and reverse images with SIS BDDs.
//! This module ports the deterministic network/order bookkeeping onto owned Rust
//! records. SIS-bound BDD image operations remain explicit missing-dependency
//! errors until the native array, BDD, ntbdd, network, node, order, prioqueue,
//! and seqbdd helper ports are available.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fmt;

pub fn is_prl_product_sis_integration_blocked() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub is_real_pi: bool,
    pub is_real_po: bool,
}

impl ProductNode {
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            fanins: Vec::new(),
            is_real_pi: kind == NodeKind::PrimaryInput,
            is_real_po: kind == NodeKind::PrimaryOutput,
        }
    }

    pub fn with_fanins(mut self, fanins: impl Into<Vec<NodeId>>) -> Self {
        self.fanins = fanins.into();
        self
    }

    pub fn with_real_pi(mut self, is_real_pi: bool) -> Self {
        self.is_real_pi = is_real_pi;
        self
    }

    pub fn with_real_po(mut self, is_real_po: bool) -> Self {
        self.is_real_po = is_real_po;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductLatch {
    pub present_state_input: NodeId,
    pub next_state_output: NodeId,
}

impl ProductLatch {
    pub fn new(present_state_input: NodeId, next_state_output: NodeId) -> Self {
        Self {
            present_state_input,
            next_state_output,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductNetwork {
    nodes: Vec<ProductNode>,
    latches: Vec<ProductLatch>,
}

impl ProductNetwork {
    pub fn new(nodes: impl Into<Vec<ProductNode>>, latches: impl Into<Vec<ProductLatch>>) -> Self {
        Self {
            nodes: nodes.into(),
            latches: latches.into(),
        }
    }

    pub fn nodes(&self) -> &[ProductNode] {
        &self.nodes
    }

    pub fn latches(&self) -> &[ProductLatch] {
        &self.latches
    }

    pub fn node(&self, id: NodeId) -> Option<&ProductNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = &ProductNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryInput)
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = &ProductNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryOutput)
    }

    pub fn latch_end(&self, input: NodeId) -> Option<NodeId> {
        self.latches
            .iter()
            .find(|latch| latch.present_state_input == input)
            .map(|latch| latch.next_state_output)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportInfo {
    pub n_pi: usize,
    pub supports: Vec<BTreeSet<usize>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoOrdering {
    GreedySupport,
    Supplied(Vec<usize>),
}

impl Default for PoOrdering {
    fn default() -> Self {
        Self::GreedySupport
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductOptions {
    pub po_ordering: PoOrdering,
    pub verbose: usize,
    pub ordering_depth: usize,
}

impl Default for ProductOptions {
    fn default() -> Self {
        Self {
            po_ordering: PoOrdering::GreedySupport,
            verbose: 0,
            ordering_depth: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductOrderPlan {
    pub output_nodes: Vec<NodeId>,
    pub var_names: Vec<String>,
    pub input_nodes: Vec<NodeId>,
    pub input_vars: Vec<usize>,
    pub external_input_vars: Vec<usize>,
    pub present_state_vars: Vec<usize>,
    pub transition_vars: Vec<usize>,
    pub next_state_vars: Vec<usize>,
    pub leaf_order: BTreeMap<NodeId, usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionXnor {
    pub next_state_fn: usize,
    pub transition_var: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrlProductError {
    UnknownNode(NodeId),
    MissingPi(NodeId),
    MissingLatchEnd(NodeId),
    DuplicatePrimaryInput(NodeId),
    InvalidPoOrderingIndex { index: usize, output_count: usize },
    PoOrderingLengthMismatch { expected: usize, actual: usize },
    TransitionLengthMismatch { functions: usize, variables: usize },
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for PrlProductError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown prl_product node {:?}", node),
            Self::MissingPi(node) => write!(f, "node {:?} is not a primary input", node),
            Self::MissingLatchEnd(node) => {
                write!(f, "non-real PI {:?} has no modeled latch end", node)
            }
            Self::DuplicatePrimaryInput(node) => {
                write!(f, "duplicate primary input {:?} in product network", node)
            }
            Self::InvalidPoOrderingIndex {
                index,
                output_count,
            } => write!(
                f,
                "PO ordering index {index} is outside next-state output range 0..{output_count}"
            ),
            Self::PoOrderingLengthMismatch { expected, actual } => write!(
                f,
                "PO ordering length {actual} does not match next-state output count {expected}"
            ),
            Self::TransitionLengthMismatch {
                functions,
                variables,
            } => write!(
                f,
                "next-state function count {functions} does not match transition variable count {variables}"
            ),
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by missing native SIS ports")
            }
        }
    }
}

impl Error for PrlProductError {}

pub fn extract_support_info(
    network: &ProductNetwork,
    node_list: &[NodeId],
) -> Result<SupportInfo, PrlProductError> {
    let pi_order = primary_input_order(network)?;
    let mut supports = Vec::with_capacity(node_list.len());

    for node in node_list {
        let mut support = BTreeSet::new();
        let mut visited = HashSet::new();
        extract_support_info_rec(network, *node, &pi_order, &mut visited, &mut support)?;
        supports.push(support);
    }

    Ok(SupportInfo {
        n_pi: pi_order.len(),
        supports,
    })
}

pub fn product_bdd_order(
    network: &ProductNetwork,
    options: &ProductOptions,
) -> Result<ProductOrderPlan, PrlProductError> {
    let output_nodes = compute_po_ordering(network, &options.po_ordering)?;
    compute_pi_ordering(network, output_nodes)
}

pub fn product_init_transition_products(
    next_state_fns: &[usize],
    transition_vars: &[usize],
) -> Result<Vec<TransitionXnor>, PrlProductError> {
    if next_state_fns.len() != transition_vars.len() {
        return Err(PrlProductError::TransitionLengthMismatch {
            functions: next_state_fns.len(),
            variables: transition_vars.len(),
        });
    }

    Ok(next_state_fns
        .iter()
        .copied()
        .zip(transition_vars.iter().copied())
        .map(|(next_state_fn, transition_var)| TransitionXnor {
            next_state_fn,
            transition_var,
        })
        .collect())
}

pub fn product_extract_network_input_names(var_names: &[String]) -> Vec<Option<String>> {
    var_names
        .iter()
        .map(|name| {
            if name.starts_with("y:") {
                None
            } else {
                Some(name.clone())
            }
        })
        .collect()
}

pub fn product_compute_next_states_blocked() -> Result<(), PrlProductError> {
    missing_native_ports("Prl_ProductComputeNextStates")
}

pub fn product_reverse_image_blocked() -> Result<(), PrlProductError> {
    missing_native_ports("Prl_ProductReverseImage")
}

pub fn product_init_seq_info_bdd_blocked() -> Result<(), PrlProductError> {
    missing_native_ports("Prl_ProductInitSeqInfo BDD allocation")
}

pub fn product_free_seq_info_bdd_blocked() -> Result<(), PrlProductError> {
    missing_native_ports("Prl_ProductFreeSeqInfo BDD free")
}

fn missing_native_ports<T>(operation: &'static str) -> Result<T, PrlProductError> {
    Err(PrlProductError::MissingNativePorts { operation })
}

fn primary_input_order(
    network: &ProductNetwork,
) -> Result<HashMap<NodeId, usize>, PrlProductError> {
    let mut pi_order = HashMap::new();
    for (index, input) in network.primary_inputs().enumerate() {
        if pi_order.insert(input.id, index).is_some() {
            return Err(PrlProductError::DuplicatePrimaryInput(input.id));
        }
    }
    Ok(pi_order)
}

fn extract_support_info_rec(
    network: &ProductNetwork,
    node: NodeId,
    pi_order: &HashMap<NodeId, usize>,
    visited: &mut HashSet<NodeId>,
    support: &mut BTreeSet<usize>,
) -> Result<(), PrlProductError> {
    if !visited.insert(node) {
        return Ok(());
    }

    let node_ref = network
        .node(node)
        .ok_or(PrlProductError::UnknownNode(node))?;
    if node_ref.kind == NodeKind::PrimaryInput {
        let uid = pi_order
            .get(&node)
            .copied()
            .ok_or(PrlProductError::MissingPi(node))?;
        support.insert(uid);
    } else {
        for fanin in &node_ref.fanins {
            extract_support_info_rec(network, *fanin, pi_order, visited, support)?;
        }
    }
    Ok(())
}

fn compute_po_ordering(
    network: &ProductNetwork,
    po_ordering: &PoOrdering,
) -> Result<Vec<NodeId>, PrlProductError> {
    let next_state_po = network
        .primary_outputs()
        .filter(|output| !output.is_real_po)
        .map(|output| output.id)
        .collect::<Vec<_>>();
    let support_info = extract_support_info(network, &next_state_po)?;
    let indices = match po_ordering {
        PoOrdering::GreedySupport => greedy_set_order(&support_info.supports),
        PoOrdering::Supplied(indices) => indices.clone(),
    };

    if indices.len() != next_state_po.len() {
        return Err(PrlProductError::PoOrderingLengthMismatch {
            expected: next_state_po.len(),
            actual: indices.len(),
        });
    }

    let mut output_nodes = Vec::with_capacity(network.primary_outputs().count());
    for index in indices {
        let output =
            next_state_po
                .get(index)
                .copied()
                .ok_or(PrlProductError::InvalidPoOrderingIndex {
                    index,
                    output_count: next_state_po.len(),
                })?;
        output_nodes.push(output);
    }

    output_nodes.extend(
        network
            .primary_outputs()
            .filter(|output| output.is_real_po)
            .map(|output| output.id),
    );
    Ok(output_nodes)
}

pub fn greedy_set_order(supports: &[BTreeSet<usize>]) -> Vec<usize> {
    let mut remaining = (0..supports.len()).collect::<BTreeSet<_>>();
    let mut covered = BTreeSet::new();
    let mut result = Vec::with_capacity(supports.len());

    while let Some(best) = remaining.iter().copied().min_by_key(|index| {
        let new_support = supports[*index].difference(&covered).count();
        (new_support, supports[*index].len(), *index)
    }) {
        remaining.remove(&best);
        covered.extend(supports[best].iter().copied());
        result.push(best);
    }

    result
}

fn compute_pi_ordering(
    network: &ProductNetwork,
    output_nodes: Vec<NodeId>,
) -> Result<ProductOrderPlan, PrlProductError> {
    let mut leaves = network
        .primary_inputs()
        .map(|input| (input.id, None))
        .collect::<BTreeMap<NodeId, Option<usize>>>();
    let mut plan = ProductOrderPlan {
        output_nodes,
        var_names: Vec::new(),
        input_nodes: Vec::new(),
        input_vars: Vec::new(),
        external_input_vars: Vec::new(),
        present_state_vars: Vec::new(),
        transition_vars: Vec::new(),
        next_state_vars: Vec::new(),
        leaf_order: BTreeMap::new(),
    };

    let mut next_index = 0;
    let n_next_state_po = network.latches().len();
    for i in 0..n_next_state_po {
        let output = plan
            .output_nodes
            .get(i)
            .copied()
            .ok_or(PrlProductError::UnknownNode(NodeId(i)))?;
        let input_order = order_dfs_from_count(network, output, &mut leaves, &mut next_index)?;
        add_new_inputs(network, &mut plan, &leaves, &input_order)?;

        plan.var_names.push(format!("y:{i}"));
        plan.transition_vars.push(next_index);
        next_index += 1;
    }

    for i in n_next_state_po..plan.output_nodes.len() {
        let output = plan.output_nodes[i];
        let input_order = order_dfs_from_count(network, output, &mut leaves, &mut next_index)?;
        add_new_inputs(network, &mut plan, &leaves, &input_order)?;
    }

    let input_order = leaves
        .iter()
        .filter_map(|(input, index)| index.is_none().then_some(*input))
        .collect::<Vec<_>>();
    for (offset, input) in input_order.iter().copied().enumerate() {
        set_variable_order(&mut leaves, input, next_index + offset)?;
    }
    add_new_inputs(network, &mut plan, &leaves, &input_order)?;

    add_next_state_vars(network, &mut plan)?;
    plan.leaf_order = leaves
        .iter()
        .filter_map(|(node, index)| index.map(|index| (*node, index)))
        .collect();

    Ok(plan)
}

fn order_dfs_from_count(
    network: &ProductNetwork,
    root: NodeId,
    leaves: &mut BTreeMap<NodeId, Option<usize>>,
    order_count: &mut usize,
) -> Result<Vec<NodeId>, PrlProductError> {
    let mut local_leaves = leaves
        .keys()
        .copied()
        .map(|node| (node, None))
        .collect::<BTreeMap<_, _>>();
    let mut nodes = Vec::new();
    let mut visited = HashSet::new();
    dfs_order(network, root, &mut local_leaves, &mut visited, &mut nodes)?;

    let mut result = Vec::new();
    let mut previous_index = None;
    for node in nodes {
        let node_ref = network
            .node(node)
            .ok_or(PrlProductError::UnknownNode(node))?;
        if node_ref.kind != NodeKind::PrimaryInput {
            continue;
        }
        if leaves.get(&node).copied().flatten().is_some() {
            continue;
        }
        let Some(index) = local_leaves.get(&node).copied().flatten() else {
            continue;
        };
        debug_assert!(previous_index.is_none_or(|previous| previous < index));
        previous_index = Some(index);
        result.push(node);
    }

    for (offset, node) in result.iter().copied().enumerate() {
        set_variable_order(leaves, node, *order_count + offset)?;
    }
    *order_count += result.len();
    Ok(result)
}

fn dfs_order(
    network: &ProductNetwork,
    node: NodeId,
    leaves: &mut BTreeMap<NodeId, Option<usize>>,
    visited: &mut HashSet<NodeId>,
    ordered: &mut Vec<NodeId>,
) -> Result<(), PrlProductError> {
    if !visited.insert(node) {
        return Ok(());
    }

    if let Some(next_index) = leaves
        .len()
        .checked_sub(leaves.values().filter(|v| v.is_none()).count())
    {
        if let Some(index) = leaves.get_mut(&node) {
            *index = Some(next_index);
            ordered.push(node);
            return Ok(());
        }
    }

    let node_ref = network
        .node(node)
        .ok_or(PrlProductError::UnknownNode(node))?;
    for fanin in &node_ref.fanins {
        dfs_order(network, *fanin, leaves, visited, ordered)?;
    }
    ordered.push(node);
    Ok(())
}

fn set_variable_order(
    leaves: &mut BTreeMap<NodeId, Option<usize>>,
    node: NodeId,
    varid: usize,
) -> Result<(), PrlProductError> {
    let entry = leaves
        .get_mut(&node)
        .ok_or(PrlProductError::MissingPi(node))?;
    *entry = Some(varid);
    Ok(())
}

fn add_new_inputs(
    network: &ProductNetwork,
    plan: &mut ProductOrderPlan,
    leaves: &BTreeMap<NodeId, Option<usize>>,
    node_list: &[NodeId],
) -> Result<(), PrlProductError> {
    for input in node_list {
        let input_ref = network
            .node(*input)
            .ok_or(PrlProductError::UnknownNode(*input))?;
        if input_ref.kind != NodeKind::PrimaryInput {
            continue;
        }
        let index = leaves
            .get(input)
            .copied()
            .flatten()
            .ok_or(PrlProductError::MissingPi(*input))?;
        plan.var_names.push(input_ref.name.clone());
        plan.input_nodes.push(*input);
        plan.input_vars.push(index);
        if input_ref.is_real_pi {
            plan.external_input_vars.push(index);
        } else {
            plan.present_state_vars.push(index);
        }
    }
    Ok(())
}

fn add_next_state_vars(
    network: &ProductNetwork,
    plan: &mut ProductOrderPlan,
) -> Result<(), PrlProductError> {
    let next_state_table = plan
        .output_nodes
        .iter()
        .take(network.latches().len())
        .copied()
        .zip(plan.transition_vars.iter().copied())
        .collect::<HashMap<_, _>>();

    for input in &plan.input_nodes {
        let input_ref = network
            .node(*input)
            .ok_or(PrlProductError::UnknownNode(*input))?;
        if input_ref.is_real_pi {
            continue;
        }
        let output = network
            .latch_end(*input)
            .ok_or(PrlProductError::MissingLatchEnd(*input))?;
        let index = next_state_table
            .get(&output)
            .copied()
            .ok_or(PrlProductError::UnknownNode(output))?;
        plan.next_state_vars.push(index);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize, name: &str, kind: NodeKind) -> ProductNode {
        ProductNode::new(NodeId(id), name, kind)
    }

    fn sample_network() -> ProductNetwork {
        let a = node(0, "a", NodeKind::PrimaryInput);
        let b = node(1, "b", NodeKind::PrimaryInput);
        let ps = node(2, "ps", NodeKind::PrimaryInput).with_real_pi(false);
        let n0 = node(3, "n0", NodeKind::Internal).with_fanins([NodeId(0), NodeId(2)]);
        let n1 = node(4, "n1", NodeKind::Internal).with_fanins([NodeId(1), NodeId(3)]);
        let y0 = node(5, "y0", NodeKind::PrimaryOutput)
            .with_real_po(false)
            .with_fanins([NodeId(3)]);
        let z = node(6, "z", NodeKind::PrimaryOutput).with_fanins([NodeId(4)]);
        ProductNetwork::new(
            vec![a, b, ps, n0, n1, y0, z],
            [ProductLatch::new(NodeId(2), NodeId(5))],
        )
    }

    #[test]
    fn support_info_collects_primary_inputs_through_fanin_graph() {
        let network = sample_network();
        let support = extract_support_info(&network, &[NodeId(5), NodeId(6)])
            .expect("support extraction should succeed");

        assert_eq!(support.n_pi, 3);
        assert_eq!(support.supports[0], BTreeSet::from([0, 2]));
        assert_eq!(support.supports[1], BTreeSet::from([0, 1, 2]));
    }

    #[test]
    fn product_order_places_next_state_outputs_before_real_outputs() {
        let network = sample_network();
        let plan = product_bdd_order(&network, &ProductOptions::default())
            .expect("product order should be computed");

        assert_eq!(plan.output_nodes, vec![NodeId(5), NodeId(6)]);
        assert_eq!(plan.var_names, vec!["a", "ps", "y:0", "b"]);
        assert_eq!(plan.input_nodes, vec![NodeId(0), NodeId(2), NodeId(1)]);
        assert_eq!(plan.input_vars, vec![0, 1, 3]);
        assert_eq!(plan.external_input_vars, vec![0, 3]);
        assert_eq!(plan.present_state_vars, vec![1]);
        assert_eq!(plan.transition_vars, vec![2]);
        assert_eq!(plan.next_state_vars, vec![2]);
        assert_eq!(
            plan.leaf_order,
            BTreeMap::from([(NodeId(0), 0), (NodeId(1), 3), (NodeId(2), 1)])
        );
    }

    #[test]
    fn supplied_po_order_is_validated() {
        let network = sample_network();
        let options = ProductOptions {
            po_ordering: PoOrdering::Supplied(vec![1]),
            ..ProductOptions::default()
        };

        assert_eq!(
            product_bdd_order(&network, &options),
            Err(PrlProductError::InvalidPoOrderingIndex {
                index: 1,
                output_count: 1,
            })
        );
    }

    #[test]
    fn transition_product_pairs_next_state_functions_with_transition_vars() {
        assert_eq!(
            product_init_transition_products(&[10, 11], &[2, 5]).unwrap(),
            vec![
                TransitionXnor {
                    next_state_fn: 10,
                    transition_var: 2,
                },
                TransitionXnor {
                    next_state_fn: 11,
                    transition_var: 5,
                },
            ]
        );
        assert_eq!(
            product_init_transition_products(&[10], &[2, 5]),
            Err(PrlProductError::TransitionLengthMismatch {
                functions: 1,
                variables: 2,
            })
        );
    }

    #[test]
    fn network_input_names_hide_product_next_state_variables() {
        let names = ["a".to_owned(), "y:0".to_owned(), "ps".to_owned()];

        assert_eq!(
            product_extract_network_input_names(&names),
            vec![Some("a".to_owned()), None, Some("ps".to_owned())]
        );
    }
}
