//! Native Rust model for `LogicSynthesis/sis/seqbdd/verif_util.c`.
//!
//! The C file mixes three concerns:
//! - top-level verification/range orchestration over SIS `network_t` and BDDs;
//! - deterministic support/order bookkeeping for PI and PO variables;
//! - diagnostic formatting and small lifetime helpers.
//!
//! This module ports the deterministic behavior onto owned Rust data
//! structures. Operations that still require SIS network, node, latch, array,
//! st_table, or BDD ports return explicit dependency errors with bead IDs and
//! source files.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub note: &'static str,
}

pub const REQUIRED_PORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        note: "array_t allocation, indexed insert/fetch, sorting, and ownership",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.71",
        source_file: "LogicSynthesis/sis/bdd_cmu/bdd_port/bddport.c",
        note: "BDD constants, boolean operations, cofactor, smoothing, cube iteration, and var IDs",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.230",
        source_file: "LogicSynthesis/sis/latch/latch.c",
        note: "latch lookup from PI nodes and latch initial values",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.297",
        source_file: "LogicSynthesis/sis/network/dfs.c",
        note: "network_dfs used while building unreached-state EXDC networks",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.299",
        source_file: "LogicSynthesis/sis/network/net_seq.c",
        note: "real PI/PO and latch classification",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "network lookup, node naming, PO/PI iteration, and mutation",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        note: "fanin traversal and node_get_fanin",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.317",
        source_file: "LogicSynthesis/sis/node/names.c",
        note: "node_long_name used in failing minterm reports",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "node types, duplication, literals, constants, AND/XNOR creation, and BDD slots",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.326",
        source_file: "LogicSynthesis/sis/ntbdd/bdd_at_node.c",
        note: "ntbdd_at_node, ntbdd_set_at_node, and ntbdd_node_to_bdd",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.335",
        source_file: "LogicSynthesis/sis/order/dfs_order.c",
        note: "order_dfs PI ordering heuristic",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.430",
        source_file: "LogicSynthesis/sis/seqbdd/ordering.c",
        note: "find_best_set_order PO support ordering heuristic",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.429",
        source_file: "LogicSynthesis/sis/seqbdd/network_info.c",
        note: "extract_network_info and product-network construction inputs",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.440",
        source_file: "LogicSynthesis/sis/seqbdd/product.c",
        note: "product-method range data, next-state image, reverse image, and output checks",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.425",
        source_file: "LogicSynthesis/sis/seqbdd/bull.c",
        note: "BULL-method range data, next-state image, reverse image, and output checks",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.427",
        source_file: "LogicSynthesis/sis/seqbdd/consistency.c",
        note: "consistency-method range data, next-state image, reverse image, and output checks",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        note: "st_table ordering maps and pointer/name tables",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.504",
        source_file: "LogicSynthesis/sis/util/cpu_time.c",
        note: "elapsed timing reports",
    },
];

pub fn required_port_beads() -> &'static [PortDependency] {
    REQUIRED_PORT_BEADS
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RangeMethod {
    Consistency,
    Bull,
    Product,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchInitialValue {
    Zero,
    One,
    DontCare,
    Invalid(i32),
}

impl LatchInitialValue {
    pub fn from_sis_value(value: i32) -> Self {
        match value {
            0 => Self::Zero,
            1 => Self::One,
            2 => Self::DontCare,
            other => Self::Invalid(other),
        }
    }

    fn is_valid_network1(self) -> bool {
        matches!(self, Self::Zero | Self::One | Self::DontCare)
    }

    fn is_invalid_network2(self) -> bool {
        matches!(self, Self::Invalid(3))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub is_real_pi: bool,
    pub is_real_po: bool,
    pub bdd_var_id: Option<usize>,
}

impl SeqNode {
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        let is_real_pi = kind == NodeKind::PrimaryInput;
        let is_real_po = kind == NodeKind::PrimaryOutput;
        Self {
            id,
            name: name.into(),
            kind,
            fanins: Vec::new(),
            is_real_pi,
            is_real_po,
            bdd_var_id: None,
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

    pub fn with_bdd_var_id(mut self, bdd_var_id: usize) -> Self {
        self.bdd_var_id = Some(bdd_var_id);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqLatch {
    pub input: NodeId,
    pub output: NodeId,
    pub initial_value: LatchInitialValue,
}

impl SeqLatch {
    pub fn new(input: NodeId, output: NodeId, initial_value: LatchInitialValue) -> Self {
        Self {
            input,
            output,
            initial_value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqNetwork {
    nodes: Vec<SeqNode>,
    latches: Vec<SeqLatch>,
}

impl SeqNetwork {
    pub fn new(nodes: impl Into<Vec<SeqNode>>, latches: impl Into<Vec<SeqLatch>>) -> Self {
        Self {
            nodes: nodes.into(),
            latches: latches.into(),
        }
    }

    pub fn nodes(&self) -> &[SeqNode] {
        &self.nodes
    }

    pub fn latches(&self) -> &[SeqLatch] {
        &self.latches
    }

    pub fn node(&self, id: NodeId) -> Option<&SeqNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn node_name(&self, id: NodeId) -> Option<&str> {
        self.node(id).map(|node| node.name.as_str())
    }

    pub fn find_node_by_name(&self, name: &str) -> Option<&SeqNode> {
        self.nodes.iter().find(|node| node.name == name)
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = &SeqNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryInput)
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = &SeqNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryOutput)
    }

    pub fn latch_from_node(&self, node: NodeId) -> Option<&SeqLatch> {
        self.latches.iter().find(|latch| latch.output == node)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationPrecheck {
    pub real_input_count: usize,
    pub real_output_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifUtilError {
    MissingInputInNetwork2 {
        name: String,
    },
    MissingOutputInNetwork2 {
        name: String,
    },
    InputCountMismatch {
        network1: usize,
        network2: usize,
    },
    OutputCountMismatch {
        network1: usize,
        network2: usize,
    },
    NoRealOutputs,
    InvalidLatchInitialValue {
        network_index: usize,
        input_name: String,
        output_name: String,
        value: LatchInitialValue,
    },
    UnknownNode(NodeId),
    DuplicatePiOrder(NodeId),
    NewPiLengthMismatch {
        po_count: usize,
        new_pi_count: usize,
    },
    MissingPiInNetwork(NodeId),
    MissingBddVariable {
        node: NodeId,
        pi_index: usize,
    },
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for VerifUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInputInNetwork2 { name } => write!(
                f,
                "input {name} appears in network1 but not network2; finite state machines are not equal"
            ),
            Self::MissingOutputInNetwork2 { name } => write!(
                f,
                "output {name} appears in network1 but not network2; finite state machines are not equal"
            ),
            Self::InputCountMismatch { network1, network2 } => write!(
                f,
                "the number of inputs in the networks are not equal ({network1} != {network2})"
            ),
            Self::OutputCountMismatch { network1, network2 } => write!(
                f,
                "the number of outputs in the networks are not equal ({network1} != {network2})"
            ),
            Self::NoRealOutputs => {
                write!(f, "verification not performed: no outputs in the network")
            }
            Self::InvalidLatchInitialValue {
                network_index,
                input_name,
                output_name,
                value,
            } => write!(
                f,
                "network{network_index} latch with input {input_name} and output {output_name} is not properly initialized: {value:?}"
            ),
            Self::UnknownNode(node) => write!(f, "unknown seqbdd node {:?}", node),
            Self::DuplicatePiOrder(node) => write!(f, "duplicate PI ordering entry for {:?}", node),
            Self::NewPiLengthMismatch {
                po_count,
                new_pi_count,
            } => write!(
                f,
                "PO ordering length {po_count} does not match new PI length {new_pi_count}"
            ),
            Self::MissingPiInNetwork(node) => {
                write!(f, "node {:?} is not a PI in the network", node)
            }
            Self::MissingBddVariable { node, pi_index } => write!(
                f,
                "node {:?} at PI ordering index {pi_index} has no native BDD variable",
                node
            ),
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires native Rust ports for {} SIS dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for VerifUtilError {}

pub fn precheck_sequential_verification(
    network1: &SeqNetwork,
    network2: &SeqNetwork,
) -> Result<VerificationPrecheck, VerifUtilError> {
    let count1 = check_network1_inputs(network1, network2)?;
    let count2 = check_network2_inputs(network2)?;

    if count1 != count2 {
        return Err(VerifUtilError::InputCountMismatch {
            network1: count1,
            network2: count2,
        });
    }

    let count1 = check_network1_outputs(network1, network2)?;
    let count2 = network2
        .primary_outputs()
        .filter(|output| output.is_real_po)
        .count();

    if count1 != count2 {
        return Err(VerifUtilError::OutputCountMismatch {
            network1: count1,
            network2: count2,
        });
    }
    if count1 == 0 {
        return Err(VerifUtilError::NoRealOutputs);
    }

    Ok(VerificationPrecheck {
        real_input_count: count2,
        real_output_count: count1,
    })
}

fn check_network1_inputs(
    network1: &SeqNetwork,
    network2: &SeqNetwork,
) -> Result<usize, VerifUtilError> {
    let mut count = 0;
    for input in network1.primary_inputs() {
        if let Some(latch) = network1.latch_from_node(input.id) {
            if !latch.initial_value.is_valid_network1() {
                return Err(invalid_latch_error(1, network1, latch));
            }
        } else {
            if network2.find_node_by_name(&input.name).is_none() {
                return Err(VerifUtilError::MissingInputInNetwork2 {
                    name: input.name.clone(),
                });
            }
            count += 1;
        }
    }
    Ok(count)
}

fn check_network2_inputs(network2: &SeqNetwork) -> Result<usize, VerifUtilError> {
    let mut count = 0;
    for input in network2.primary_inputs() {
        if let Some(latch) = network2.latch_from_node(input.id) {
            if latch.initial_value.is_invalid_network2() {
                return Err(invalid_latch_error(2, network2, latch));
            }
        } else {
            count += 1;
        }
    }
    Ok(count)
}

fn check_network1_outputs(
    network1: &SeqNetwork,
    network2: &SeqNetwork,
) -> Result<usize, VerifUtilError> {
    let mut count = 0;
    for output in network1
        .primary_outputs()
        .filter(|output| output.is_real_po)
    {
        if network2.find_node_by_name(&output.name).is_none() {
            return Err(VerifUtilError::MissingOutputInNetwork2 {
                name: output.name.clone(),
            });
        }
        count += 1;
    }
    Ok(count)
}

fn invalid_latch_error(
    network_index: usize,
    network: &SeqNetwork,
    latch: &SeqLatch,
) -> VerifUtilError {
    VerifUtilError::InvalidLatchInitialValue {
        network_index,
        input_name: network
            .node_name(latch.input)
            .unwrap_or("<unknown>")
            .to_owned(),
        output_name: network
            .node_name(latch.output)
            .unwrap_or("<unknown>")
            .to_owned(),
        value: latch.initial_value,
    }
}

pub fn get_remaining_po(network: &SeqNetwork, po_array: &[NodeId]) -> Vec<NodeId> {
    let selected: HashSet<_> = po_array.iter().copied().collect();
    network
        .primary_outputs()
        .filter(|po| !selected.contains(&po.id))
        .map(|po| po.id)
        .collect()
}

pub fn from_array_to_table(array: &[NodeId]) -> HashMap<NodeId, usize> {
    array
        .iter()
        .copied()
        .enumerate()
        .map(|(index, node)| (node, index))
        .collect()
}

pub fn print_node_array(network: &SeqNetwork, array: &[Option<NodeId>]) -> String {
    let mut text = String::new();
    for node in array {
        match node {
            Some(node) => text.push_str(network.node_name(*node).unwrap_or("<unknown>")),
            None => text.push_str("---"),
        }
        text.push(' ');
    }
    text.push('\n');
    text
}

pub fn print_node_table(network: &SeqNetwork, table: &HashMap<NodeId, usize>) -> String {
    let mut entries: Vec<_> = table.iter().collect();
    entries.sort_by(|(left_node, left_value), (right_node, right_value)| {
        left_value.cmp(right_value).then_with(|| {
            let left_name = network.node_name(**left_node).unwrap_or("");
            let right_name = network.node_name(**right_node).unwrap_or("");
            left_name.cmp(right_name)
        })
    });

    let mut text = String::new();
    for (node, value) in entries {
        text.push_str(network.node_name(*node).unwrap_or("<unknown>"));
        text.push_str("=>");
        text.push_str(&value.to_string());
        text.push(' ');
    }
    text.push('\n');
    text
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportInfo {
    pub n_pi: usize,
    pub supports: Vec<BTreeSet<usize>>,
}

pub fn extract_support_info(
    network: &SeqNetwork,
    node_list: &[NodeId],
) -> Result<SupportInfo, VerifUtilError> {
    let pi_order: HashMap<_, _> = network
        .primary_inputs()
        .enumerate()
        .map(|(index, input)| (input.id, index))
        .collect();

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

fn extract_support_info_rec(
    network: &SeqNetwork,
    node: NodeId,
    pi_order: &HashMap<NodeId, usize>,
    visited: &mut HashSet<NodeId>,
    support: &mut BTreeSet<usize>,
) -> Result<(), VerifUtilError> {
    if !visited.insert(node) {
        return Ok(());
    }

    let node_ref = network
        .node(node)
        .ok_or(VerifUtilError::UnknownNode(node))?;
    if node_ref.kind == NodeKind::PrimaryInput {
        let uid = pi_order
            .get(&node)
            .copied()
            .ok_or(VerifUtilError::MissingPiInNetwork(node))?;
        support.insert(uid);
    } else {
        for fanin in &node_ref.fanins {
            extract_support_info_rec(network, *fanin, pi_order, visited, support)?;
        }
    }
    Ok(())
}

pub fn get_po_ordering(
    network: &SeqNetwork,
    next_state_po: &[NodeId],
    supplied_index_order: Option<&[usize]>,
) -> Result<Vec<NodeId>, VerifUtilError> {
    let support_info = extract_support_info(network, next_state_po)?;
    let indices = match supplied_index_order {
        Some(ordering) => ordering.to_vec(),
        None => greedy_set_order(&support_info.supports),
    };

    indices
        .into_iter()
        .map(|index| {
            next_state_po
                .get(index)
                .copied()
                .ok_or(VerifUtilError::UnknownNode(NodeId(index)))
        })
        .collect()
}

pub fn greedy_set_order(supports: &[BTreeSet<usize>]) -> Vec<usize> {
    let mut remaining: BTreeSet<_> = (0..supports.len()).collect();
    let mut covered = BTreeSet::new();
    let mut result = Vec::with_capacity(supports.len());

    while !remaining.is_empty() {
        let best = remaining
            .iter()
            .copied()
            .min_by_key(|index| {
                let new_support = supports[*index].difference(&covered).count();
                (new_support, supports[*index].len(), *index)
            })
            .expect("remaining set is not empty");
        remaining.remove(&best);
        covered.extend(supports[best].iter().copied());
        result.push(best);
    }

    result
}

pub fn get_pi_ordering(
    network: &SeqNetwork,
    po_ordering: &[NodeId],
    new_pi: &[NodeId],
) -> Result<HashMap<NodeId, usize>, VerifUtilError> {
    if po_ordering.len() != new_pi.len() {
        return Err(VerifUtilError::NewPiLengthMismatch {
            po_count: po_ordering.len(),
            new_pi_count: new_pi.len(),
        });
    }

    let support_info = extract_support_info(network, po_ordering)?;
    let mut remaining_pi: BTreeSet<_> = network.primary_inputs().map(|pi| pi.id).collect();
    let old_pi = order_nodes(network, po_ordering, true)?;
    let remaining_po = get_remaining_po(network, po_ordering);
    let other_pi = order_nodes(network, &remaining_po, true)?;
    let mut so_far = BTreeSet::new();
    let mut pi_order = Vec::new();
    let mut old_index = 0;

    for (po_index, new_input) in new_pi.iter().copied().enumerate() {
        let support = &support_info.supports[po_index];
        loop {
            let Some(node) = old_pi.get(old_index).copied() else {
                break;
            };
            let pi_index = primary_input_index(network, node)?;
            if !support.contains(&pi_index) {
                break;
            }
            if so_far.insert(pi_index) {
                pi_order.push(node);
                remaining_pi.remove(&node);
            }
            old_index += 1;
        }
        pi_order.push(new_input);
        remaining_pi.remove(&new_input);
        so_far.extend(support.iter().copied());
    }

    for node in other_pi {
        if remaining_pi.remove(&node) {
            pi_order.push(node);
        }
    }

    pi_order.extend(remaining_pi);
    Ok(from_array_to_table(&pi_order))
}

fn primary_input_index(network: &SeqNetwork, node: NodeId) -> Result<usize, VerifUtilError> {
    network
        .primary_inputs()
        .position(|pi| pi.id == node)
        .ok_or(VerifUtilError::MissingPiInNetwork(node))
}

pub fn order_nodes(
    network: &SeqNetwork,
    node_vec: &[NodeId],
    pi_only: bool,
) -> Result<Vec<NodeId>, VerifUtilError> {
    let mut ordered = Vec::new();
    let mut visited = HashSet::new();
    for node in node_vec {
        dfs_order(network, *node, pi_only, &mut visited, &mut ordered)?;
    }
    Ok(ordered)
}

fn dfs_order(
    network: &SeqNetwork,
    node: NodeId,
    pi_only: bool,
    visited: &mut HashSet<NodeId>,
    ordered: &mut Vec<NodeId>,
) -> Result<(), VerifUtilError> {
    if !visited.insert(node) {
        return Ok(());
    }

    let node_ref = network
        .node(node)
        .ok_or(VerifUtilError::UnknownNode(node))?;
    for fanin in &node_ref.fanins {
        dfs_order(network, *fanin, pi_only, visited, ordered)?;
    }
    if !pi_only || node_ref.kind == NodeKind::PrimaryInput {
        ordered.push(node);
    }
    Ok(())
}

pub fn bdd_extract_var_array(
    network: &SeqNetwork,
    node_list: &[Option<NodeId>],
    pi_ordering: &HashMap<NodeId, usize>,
) -> Result<Vec<usize>, VerifUtilError> {
    let mut result = Vec::new();
    for node in node_list.iter().flatten() {
        let node_ref = network
            .node(*node)
            .ok_or(VerifUtilError::UnknownNode(*node))?;
        if let Some(var_id) = node_ref.bdd_var_id {
            result.push(var_id);
        } else {
            let index = *pi_ordering
                .get(node)
                .ok_or(VerifUtilError::MissingPiInNetwork(*node))?;
            result.push(index);
        }
    }
    Ok(result)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteStateSet {
    states: BTreeSet<u64>,
}

impl FiniteStateSet {
    pub fn empty() -> Self {
        Self {
            states: BTreeSet::new(),
        }
    }

    pub fn from_states(states: impl IntoIterator<Item = u64>) -> Self {
        Self {
            states: states.into_iter().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        self.states.is_subset(&other.states)
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            states: self.states.union(&other.states).copied().collect(),
        }
    }

    pub fn difference(&self, other: &Self) -> Self {
        Self {
            states: self.states.difference(&other.states).copied().collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraversalOutcome {
    pub verified: bool,
    pub depth: usize,
    pub total_set: FiniteStateSet,
    pub output_index: Option<usize>,
}

pub fn breadth_first_state_traversal<CheckOutput, ComputeNext>(
    init_state: FiniteStateSet,
    does_verification: bool,
    n_iter: usize,
    mut check_output: CheckOutput,
    mut compute_next_states: ComputeNext,
) -> TraversalOutcome
where
    CheckOutput: FnMut(&FiniteStateSet) -> Option<usize>,
    ComputeNext: FnMut(&FiniteStateSet) -> FiniteStateSet,
{
    let mut i = 0;
    let mut current_set = init_state;
    let mut total_set = FiniteStateSet::empty();

    loop {
        if does_verification {
            if let Some(output_index) = check_output(&current_set) {
                return TraversalOutcome {
                    verified: false,
                    depth: i,
                    total_set,
                    output_index: Some(output_index),
                };
            }
        }
        if current_set.is_subset(&total_set) {
            break;
        }
        if !does_verification && i >= n_iter {
            break;
        }

        let new_current_set = current_set.difference(&total_set);
        total_set = total_set.union(&current_set);
        current_set = compute_next_states(&new_current_set);
        i += 1;
    }

    TraversalOutcome {
        verified: true,
        depth: i,
        total_set,
        output_index: None,
    }
}

pub fn report_elapsed_time(
    last_time: &mut Duration,
    total_time: &mut Duration,
    new_time: Duration,
) -> String {
    let elapsed = new_time.saturating_sub(*last_time);
    *last_time = new_time;
    *total_time += elapsed;
    format!(
        "*** [elapsed({:.1}),total({:.1})] ***\n",
        elapsed.as_secs_f64(),
        total_time.as_secs_f64()
    )
}

pub fn seq_verify_interface<Network>(
    _network1: &mut Network,
    _network2: &mut Network,
    _method: RangeMethod,
) -> Result<(), VerifUtilError> {
    Err(VerifUtilError::MissingNativePorts {
        operation: "seq_verify_interface",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn range_computation_interface<Network>(
    _network: &mut Network,
    _method: RangeMethod,
) -> Result<(), VerifUtilError> {
    Err(VerifUtilError::MissingNativePorts {
        operation: "range_computation_interface",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn network_copy_subnetwork<Network, Node>(
    _network: &mut Network,
    _node: &Node,
) -> Result<(), VerifUtilError> {
    Err(VerifUtilError::MissingNativePorts {
        operation: "network_copy_subnetwork",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn report_inconsistency<Bdd>(
    _current_set: &Bdd,
    _output_fn: &Bdd,
) -> Result<(), VerifUtilError> {
    Err(VerifUtilError::MissingNativePorts {
        operation: "report_inconsistency",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize, name: &str, kind: NodeKind, fanins: &[usize]) -> SeqNode {
        SeqNode::new(NodeId(id), name, kind)
            .with_fanins(fanins.iter().copied().map(NodeId).collect::<Vec<_>>())
    }

    fn sample_network() -> SeqNetwork {
        SeqNetwork::new(
            vec![
                node(0, "a", NodeKind::PrimaryInput, &[]).with_bdd_var_id(10),
                node(1, "b", NodeKind::PrimaryInput, &[]),
                node(2, "s", NodeKind::PrimaryInput, &[]).with_real_pi(false),
                node(3, "n1", NodeKind::Internal, &[0, 1]),
                node(4, "n2", NodeKind::Internal, &[1, 2]),
                node(5, "po1", NodeKind::PrimaryOutput, &[3]),
                node(6, "po2", NodeKind::PrimaryOutput, &[4]),
                node(7, "ns", NodeKind::Internal, &[2]),
            ],
            vec![SeqLatch::new(
                NodeId(7),
                NodeId(2),
                LatchInitialValue::DontCare,
            )],
        )
    }

    #[test]
    fn precheck_accepts_matching_real_inputs_and_outputs() {
        let net1 = sample_network();
        let net2 = sample_network();

        assert_eq!(
            precheck_sequential_verification(&net1, &net2),
            Ok(VerificationPrecheck {
                real_input_count: 2,
                real_output_count: 2,
            })
        );
    }

    #[test]
    fn precheck_reports_missing_input_and_output_by_name() {
        let net1 = sample_network();
        let mut net2 = sample_network();
        net2.nodes.retain(|node| node.name != "a");

        assert_eq!(
            precheck_sequential_verification(&net1, &net2),
            Err(VerifUtilError::MissingInputInNetwork2 {
                name: "a".to_owned()
            })
        );

        let mut net2 = sample_network();
        net2.nodes.retain(|node| node.name != "po2");
        assert_eq!(
            precheck_sequential_verification(&net1, &net2),
            Err(VerifUtilError::MissingOutputInNetwork2 {
                name: "po2".to_owned()
            })
        );
    }

    #[test]
    fn precheck_preserves_c_network2_latch_value_three_rule() {
        let net1 = sample_network();
        let mut net2 = sample_network();
        net2.latches[0].initial_value = LatchInitialValue::Invalid(3);

        assert!(matches!(
            precheck_sequential_verification(&net1, &net2),
            Err(VerifUtilError::InvalidLatchInitialValue {
                network_index: 2,
                ..
            })
        ));
    }

    #[test]
    fn support_info_walks_transitive_fanins_and_skips_revisits() {
        let network = sample_network();
        let support = extract_support_info(&network, &[NodeId(5), NodeId(6)]).unwrap();

        assert_eq!(support.n_pi, 3);
        assert_eq!(
            support.supports,
            vec![BTreeSet::from([0, 1]), BTreeSet::from([1, 2]),]
        );
    }

    #[test]
    fn po_ordering_uses_supplied_index_order_or_greedy_supports() {
        let network = sample_network();

        assert_eq!(
            get_po_ordering(&network, &[NodeId(5), NodeId(6)], Some(&[1, 0])).unwrap(),
            vec![NodeId(6), NodeId(5)]
        );
        assert_eq!(
            get_po_ordering(&network, &[NodeId(5), NodeId(6)], None).unwrap(),
            vec![NodeId(5), NodeId(6)]
        );
    }

    #[test]
    fn pi_ordering_inserts_new_inputs_after_each_po_support_slice() {
        let network = sample_network();
        let ordering =
            get_pi_ordering(&network, &[NodeId(5), NodeId(6)], &[NodeId(8), NodeId(9)]).unwrap();

        assert_eq!(ordering[&NodeId(0)], 0);
        assert_eq!(ordering[&NodeId(1)], 1);
        assert_eq!(ordering[&NodeId(8)], 2);
        assert_eq!(ordering[&NodeId(2)], 3);
        assert_eq!(ordering[&NodeId(9)], 4);
    }

    #[test]
    fn bdd_extract_var_array_uses_existing_node_bdd_or_pi_index() {
        let network = sample_network();
        let pi_ordering = HashMap::from([(NodeId(0), 0), (NodeId(1), 1), (NodeId(2), 2)]);

        assert_eq!(
            bdd_extract_var_array(
                &network,
                &[Some(NodeId(0)), None, Some(NodeId(1))],
                &pi_ordering
            )
            .unwrap(),
            vec![10, 1]
        );
    }

    #[test]
    fn breadth_first_state_traversal_tracks_total_set_and_failure_output() {
        let outcome = breadth_first_state_traversal(
            FiniteStateSet::from_states([0]),
            true,
            10,
            |states| states.states.contains(&2).then_some(4),
            |states| {
                if states.states.contains(&0) {
                    FiniteStateSet::from_states([1])
                } else if states.states.contains(&1) {
                    FiniteStateSet::from_states([2])
                } else {
                    FiniteStateSet::empty()
                }
            },
        );

        assert_eq!(
            outcome,
            TraversalOutcome {
                verified: false,
                depth: 2,
                total_set: FiniteStateSet::from_states([0, 1]),
                output_index: Some(4),
            }
        );
    }

    #[test]
    fn print_helpers_match_c_spacing() {
        let network = sample_network();

        assert_eq!(
            print_node_array(&network, &[Some(NodeId(0)), None, Some(NodeId(6))]),
            "a --- po2 \n"
        );
        assert_eq!(
            print_node_table(&network, &HashMap::from([(NodeId(1), 2), (NodeId(0), 1)])),
            "a=>1 b=>2 \n"
        );
    }

    #[test]
    fn blocked_operations_report_dependency_beads_and_source_files() {
        let error = seq_verify_interface(&mut (), &mut (), RangeMethod::Product).unwrap_err();

        match error {
            VerifUtilError::MissingNativePorts {
                operation,
                dependencies,
            } => {
                assert_eq!(operation, "seq_verify_interface");
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.71"
                        && dependency.source_file == "LogicSynthesis/sis/bdd_cmu/bdd_port/bddport.c"
                }));
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.440"
                        && dependency.source_file == "LogicSynthesis/sis/seqbdd/product.c"
                }));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
