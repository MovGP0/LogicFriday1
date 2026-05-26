//! Native Rust model for `LogicSynthesis/sis/seqbdd/prl_remlatch.c`.
//!
//! The C implementation removes redundant sequential state bits, tries to
//! remove boot latches, and locally retimes latches forward across logic. The
//! SIS implementation mutates `network_t` objects and relies on BDD, ntbdd,
//! latch, node, array, and st_table APIs that are not all native Rust ports yet.
//! This module ports the deterministic decision logic onto owned Rust data
//! structures and makes SIS-bound entry points fail with explicit dependency
//! bead/source information.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fmt;

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
        note: "array_t storage for BDD variable, node, and latch collections",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.71",
        source_file: "LogicSynthesis/sis/bdd_cmu/bdd_port/bddport.c",
        note: "BDD boolean operations, smoothing, consensus, cofactor, support, and variable creation",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.230",
        source_file: "LogicSynthesis/sis/latch/latch.c",
        note: "latch lookup, deletion, creation, and initial/current value mutation",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.299",
        source_file: "LogicSynthesis/sis/network/net_seq.c",
        note: "real PI/PO classification, latch counting, and sequential network traversal",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "network duplication, sweeping, node lookup, latch rewiring, and DC-network removal",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        note: "fanin/fanout traversal and fanin patching",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "node constants, literals, simulation values, and node type/function checks",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.326",
        source_file: "LogicSynthesis/sis/ntbdd/bdd_at_node.c",
        note: "BDD-to-network extraction used to synthesize latch recoding logic",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.329",
        source_file: "LogicSynthesis/sis/ntbdd/manager.c",
        note: "native BDD manager ownership for seqbdd state variables",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.429",
        source_file: "LogicSynthesis/sis/seqbdd/network_info.c",
        note: "Prl_SeqInitNetwork, seq_info_t layout, and network input-name extraction",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.442",
        source_file: "LogicSynthesis/sis/seqbdd/verif_util.c",
        note: "Prl_GetSimpleDc, Prl_GetOneEdge, PI-to-var maps, and copy helpers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        note: "pointer-keyed node, latch, and BDD variable maps",
    },
];

pub fn required_port_beads() -> &'static [PortDependency] {
    REQUIRED_PORT_BEADS
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LatchId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateVar(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstantValue {
    Zero,
    One,
}

impl ConstantValue {
    pub fn bit(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatchNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub is_real_pi: bool,
    pub constant: Option<ConstantValue>,
}

impl LatchNode {
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            fanins: Vec::new(),
            is_real_pi: kind == NodeKind::PrimaryInput,
            constant: None,
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

    pub fn with_constant(mut self, constant: ConstantValue) -> Self {
        self.constant = Some(constant);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Latch {
    pub id: LatchId,
    pub input: NodeId,
    pub output: NodeId,
    pub initial_value: u8,
}

impl Latch {
    pub fn new(
        id: LatchId,
        input: NodeId,
        output: NodeId,
        initial_value: u8,
    ) -> Result<Self, PrlRemLatchError> {
        validate_bit(initial_value)?;
        Ok(Self {
            id,
            input,
            output,
            initial_value,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatchNetwork {
    nodes: BTreeMap<NodeId, LatchNode>,
    latches: BTreeMap<LatchId, Latch>,
}

impl LatchNetwork {
    pub fn new(
        nodes: impl IntoIterator<Item = LatchNode>,
        latches: impl IntoIterator<Item = Latch>,
    ) -> Self {
        Self {
            nodes: nodes.into_iter().map(|node| (node.id, node)).collect(),
            latches: latches.into_iter().map(|latch| (latch.id, latch)).collect(),
        }
    }

    pub fn node(&self, id: NodeId) -> Option<&LatchNode> {
        self.nodes.get(&id)
    }

    pub fn latch(&self, id: LatchId) -> Option<&Latch> {
        self.latches.get(&id)
    }

    pub fn latches(&self) -> impl Iterator<Item = &Latch> {
        self.latches.values()
    }

    pub fn latch_from_output(&self, output: NodeId) -> Option<&Latch> {
        self.latches.values().find(|latch| latch.output == output)
    }

    pub fn fanouts(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes
            .values()
            .filter(|node| node.fanins.contains(&id))
            .map(|node| node.id)
            .collect()
    }

    pub fn fanout_count(&self, id: NodeId) -> usize {
        self.fanouts(id).len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrlRemLatchError {
    InvalidBinaryValue(u8),
    UnknownNode(NodeId),
    UnknownLatch(LatchId),
    EmptyStateSpace,
    StateOutsideUniverse {
        state: u64,
        state_bits: usize,
    },
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for PrlRemLatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBinaryValue(value) => {
                write!(f, "seqbdd latch values must be 0 or 1, got {value}")
            }
            Self::UnknownNode(node) => write!(f, "unknown latch-removal node {:?}", node),
            Self::UnknownLatch(latch) => write!(f, "unknown latch {:?}", latch),
            Self::EmptyStateSpace => write!(f, "state-space width must be nonzero"),
            Self::StateOutsideUniverse { state, state_bits } => write!(
                f,
                "state {state} cannot be represented with {state_bits} state bits"
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

impl Error for PrlRemLatchError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteStateSet {
    state_bits: usize,
    states: BTreeSet<u64>,
}

impl FiniteStateSet {
    pub fn new(
        state_bits: usize,
        states: impl IntoIterator<Item = u64>,
    ) -> Result<Self, PrlRemLatchError> {
        if state_bits == 0 {
            return Err(PrlRemLatchError::EmptyStateSpace);
        }
        let limit = state_limit(state_bits);
        let states = states.into_iter().collect::<BTreeSet<_>>();
        if let Some(state) = states.iter().copied().find(|state| *state >= limit) {
            return Err(PrlRemLatchError::StateOutsideUniverse { state, state_bits });
        }
        Ok(Self { state_bits, states })
    }

    pub fn state_bits(&self) -> usize {
        self.state_bits
    }

    pub fn states(&self) -> &BTreeSet<u64> {
        &self.states
    }

    pub fn contains(&self, state: u64) -> bool {
        self.states.contains(&state)
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedundantLatchInfo {
    pub variable: StateVar,
    pub new_reachable_states: FiniteStateSet,
    pub recoding_function: BTreeMap<u64, u8>,
    pub recoding_dont_care_states: BTreeSet<u64>,
    pub support_variables: BTreeSet<StateVar>,
    pub support_size: usize,
}

pub fn extract_redundant_latch_info(
    reachable_states: &FiniteStateSet,
    variable: StateVar,
) -> Option<RedundantLatchInfo> {
    if variable.0 >= reachable_states.state_bits {
        return None;
    }
    if !is_variable_redundant(reachable_states, variable) {
        return None;
    }

    let new_reachable_states = smooth_variable(reachable_states, variable);
    let recoding_function = recoding_function(reachable_states, variable);
    let support_variables = support_variables(
        reachable_states.state_bits,
        variable,
        &recoding_function,
        &new_reachable_states.states,
    );
    let universe = 0..state_limit(reachable_states.state_bits);
    let recoding_dont_care_states = universe
        .filter(|state| !new_reachable_states.states.contains(state))
        .collect();

    Some(RedundantLatchInfo {
        variable,
        new_reachable_states,
        recoding_function,
        recoding_dont_care_states,
        support_size: support_variables.len(),
        support_variables,
    })
}

pub fn first_fit_latch_removal_plan(
    reachable_states: &FiniteStateSet,
    variables: &[StateVar],
    max_cost: usize,
) -> Vec<RedundantLatchInfo> {
    let mut set = reachable_states.clone();
    let mut result = Vec::new();
    for variable in variables {
        let Some(info) = extract_redundant_latch_info(&set, *variable) else {
            continue;
        };
        if info.support_size > max_cost {
            continue;
        }
        set = info.new_reachable_states.clone();
        result.push(info);
    }
    result
}

fn is_variable_redundant(reachable_states: &FiniteStateSet, variable: StateVar) -> bool {
    let mask = 1_u64 << variable.0;
    reachable_states
        .states
        .iter()
        .all(|state| !reachable_states.states.contains(&(state ^ mask)))
}

fn smooth_variable(reachable_states: &FiniteStateSet, variable: StateVar) -> FiniteStateSet {
    let mask = 1_u64 << variable.0;
    let mut states = BTreeSet::new();
    for state in &reachable_states.states {
        states.insert(*state & !mask);
        states.insert(*state | mask);
    }
    FiniteStateSet {
        state_bits: reachable_states.state_bits,
        states,
    }
}

fn recoding_function(reachable_states: &FiniteStateSet, variable: StateVar) -> BTreeMap<u64, u8> {
    let mask = 1_u64 << variable.0;
    let mut function = BTreeMap::new();
    for state in &reachable_states.states {
        function.insert(*state & !mask, ((*state & mask) != 0) as u8);
    }
    function
}

fn support_variables(
    state_bits: usize,
    removed_variable: StateVar,
    function: &BTreeMap<u64, u8>,
    domain: &BTreeSet<u64>,
) -> BTreeSet<StateVar> {
    let mut support = BTreeSet::new();
    for variable in 0..state_bits {
        if variable == removed_variable.0 {
            continue;
        }
        let mask = 1_u64 << variable;
        if domain.iter().any(|state| {
            let left = *state & !(1_u64 << removed_variable.0);
            let right = (state ^ mask) & !(1_u64 << removed_variable.0);
            domain.contains(&(state ^ mask)) && function.get(&left) != function.get(&right)
        }) {
            support.insert(StateVar(variable));
        }
    }
    support
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetimingOpportunity {
    pub node: NodeId,
    pub latches_saved: usize,
}

pub fn save_enough_latches(
    network: &LatchNetwork,
    node: NodeId,
) -> Result<Option<RetimingOpportunity>, PrlRemLatchError> {
    let node_ref = network
        .node(node)
        .ok_or(PrlRemLatchError::UnknownNode(node))?;
    if node_ref.kind != NodeKind::Internal {
        return Ok(None);
    }

    let mut saved: isize = -1;
    for fanin in &node_ref.fanins {
        let fanin_ref = network
            .node(*fanin)
            .ok_or(PrlRemLatchError::UnknownNode(*fanin))?;
        if fanin_ref.kind != NodeKind::PrimaryInput || fanin_ref.is_real_pi {
            return Ok(None);
        }
        if network.fanout_count(*fanin) == 1 {
            saved += 1;
        }
    }

    if saved < 0 {
        Ok(None)
    } else {
        Ok(Some(RetimingOpportunity {
            node,
            latches_saved: saved as usize,
        }))
    }
}

pub fn node_only_fed_by_latches(
    network: &LatchNetwork,
    node: NodeId,
) -> Result<bool, PrlRemLatchError> {
    let node_ref = network
        .node(node)
        .ok_or(PrlRemLatchError::UnknownNode(node))?;
    if node_ref.kind != NodeKind::Internal {
        return Ok(false);
    }

    for fanin in &node_ref.fanins {
        let fanin_ref = network
            .node(*fanin)
            .ok_or(PrlRemLatchError::UnknownNode(*fanin))?;
        if fanin_ref.kind != NodeKind::PrimaryInput || network.latch_from_output(*fanin).is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn get_candidate_node(
    network: &LatchNetwork,
    node: NodeId,
) -> Result<Option<NodeId>, PrlRemLatchError> {
    let mut visited = HashSet::new();
    get_candidate_node_rec(network, node, &mut visited)
}

fn get_candidate_node_rec(
    network: &LatchNetwork,
    node: NodeId,
    visited: &mut HashSet<NodeId>,
) -> Result<Option<NodeId>, PrlRemLatchError> {
    if !visited.insert(node) {
        return Ok(None);
    }
    if node_only_fed_by_latches(network, node)? {
        return Ok(Some(node));
    }
    let node_ref = network
        .node(node)
        .ok_or(PrlRemLatchError::UnknownNode(node))?;
    for fanin in &node_ref.fanins {
        if let Some(candidate) = get_candidate_node_rec(network, *fanin, visited)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchedOutputStatus {
    AlreadyLatched,
    ConstantFanin,
    CandidateFound(NodeId),
    CombinationalDependency,
}

pub fn classify_latched_output_step(
    network: &LatchNetwork,
    primary_output: NodeId,
) -> Result<LatchedOutputStatus, PrlRemLatchError> {
    let po = network
        .node(primary_output)
        .ok_or(PrlRemLatchError::UnknownNode(primary_output))?;
    let Some(fanin) = po.fanins.first().copied() else {
        return Ok(LatchedOutputStatus::ConstantFanin);
    };
    let fanin_ref = network
        .node(fanin)
        .ok_or(PrlRemLatchError::UnknownNode(fanin))?;

    if fanin_ref.kind == NodeKind::PrimaryInput && network.latch_from_output(fanin).is_some() {
        return Ok(LatchedOutputStatus::AlreadyLatched);
    }
    if fanin_ref.fanins.is_empty() {
        return Ok(LatchedOutputStatus::ConstantFanin);
    }
    match get_candidate_node(network, fanin)? {
        Some(candidate) => Ok(LatchedOutputStatus::CandidateFound(candidate)),
        None => Ok(LatchedOutputStatus::CombinationalDependency),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootLatchRemoval {
    pub latch: LatchId,
    pub replacement_constant: u8,
    pub new_initial_state: u64,
    pub new_latch_initial_values: BTreeMap<LatchId, u8>,
}

pub fn find_boot_latch_removal(
    network: &LatchNetwork,
    initial_state: u64,
    state_bits: usize,
    state_signatures: &HashMap<u64, Vec<u8>>,
) -> Result<Option<BootLatchRemoval>, PrlRemLatchError> {
    validate_state(initial_state, state_bits)?;
    let Some(initial_signature) = state_signatures.get(&initial_state) else {
        return Ok(None);
    };

    for latch in network.latches() {
        validate_bit(latch.initial_value)?;
        let input = network
            .node(latch.input)
            .ok_or(PrlRemLatchError::UnknownNode(latch.input))?;
        let Some(input_value) = input.constant.map(ConstantValue::bit) else {
            continue;
        };
        if input_value == latch.initial_value {
            continue;
        }

        let variable = StateVar(latch.id.0);
        if variable.0 >= state_bits {
            continue;
        }
        if let Some(new_initial_state) =
            find_alternate_initial_state(initial_signature, variable, input_value, state_signatures)
        {
            let mut new_values = BTreeMap::new();
            for other in network.latches() {
                if other.id.0 < state_bits {
                    new_values.insert(other.id, bit_at(new_initial_state, StateVar(other.id.0)));
                }
            }
            return Ok(Some(BootLatchRemoval {
                latch: latch.id,
                replacement_constant: latch.initial_value,
                new_initial_state,
                new_latch_initial_values: new_values,
            }));
        }
    }

    Ok(None)
}

fn find_alternate_initial_state(
    initial_signature: &[u8],
    variable: StateVar,
    required_value: u8,
    state_signatures: &HashMap<u64, Vec<u8>>,
) -> Option<u64> {
    state_signatures
        .iter()
        .filter(|(_, signature)| signature.as_slice() == initial_signature)
        .map(|(state, _)| *state)
        .filter(|state| bit_at(*state, variable) == required_value)
        .min()
}

pub fn binary_merge<T, F>(mut values: Vec<T>, neutral: T, mut merge: F) -> T
where
    F: FnMut(T, T) -> T,
{
    if values.is_empty() {
        return neutral;
    }

    loop {
        if values.len() == 1 {
            return values.remove(0);
        }

        let mut next = Vec::with_capacity(values.len().div_ceil(2));
        let mut iter = values.into_iter();
        while let Some(left) = iter.next() {
            if let Some(right) = iter.next() {
                next.push(merge(left, right));
            } else {
                next.push(left);
            }
        }
        values = next;
    }
}

pub fn remove_latches_from_sis_network<Network>(
    _network: &mut Network,
) -> Result<(), PrlRemLatchError> {
    Err(PrlRemLatchError::MissingNativePorts {
        operation: "Prl_RemoveLatches",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn latch_output_in_sis_network<Network, Node>(
    _network: &mut Network,
    _nodes: &[Node],
) -> Result<(), PrlRemLatchError> {
    Err(PrlRemLatchError::MissingNativePorts {
        operation: "Prl_LatchOutput",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

fn validate_bit(value: u8) -> Result<(), PrlRemLatchError> {
    if value > 1 {
        Err(PrlRemLatchError::InvalidBinaryValue(value))
    } else {
        Ok(())
    }
}

fn validate_state(state: u64, state_bits: usize) -> Result<(), PrlRemLatchError> {
    if state_bits == 0 {
        return Err(PrlRemLatchError::EmptyStateSpace);
    }
    if state >= state_limit(state_bits) {
        Err(PrlRemLatchError::StateOutsideUniverse { state, state_bits })
    } else {
        Ok(())
    }
}

fn state_limit(state_bits: usize) -> u64 {
    1_u64
        .checked_shl(state_bits as u32)
        .expect("native latch-removal model supports at most 63 state bits")
}

fn bit_at(state: u64, variable: StateVar) -> u8 {
    ((state >> variable.0) & 1) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize, name: &str, kind: NodeKind, fanins: &[usize]) -> LatchNode {
        LatchNode::new(NodeId(id), name, kind)
            .with_fanins(fanins.iter().copied().map(NodeId).collect::<Vec<_>>())
    }

    fn latch(id: usize, input: usize, output: usize, initial: u8) -> Latch {
        Latch::new(LatchId(id), NodeId(input), NodeId(output), initial).unwrap()
    }

    #[test]
    fn redundant_latch_detection_matches_consensus_empty_case() {
        let reachable = FiniteStateSet::new(3, [0b000, 0b010, 0b101, 0b111]).unwrap();
        let info = extract_redundant_latch_info(&reachable, StateVar(0)).unwrap();

        assert_eq!(
            info.new_reachable_states.states(),
            &BTreeSet::from([0, 1, 2, 3, 4, 5, 6, 7])
        );
        assert_eq!(
            info.recoding_function,
            BTreeMap::from([(0b000, 0), (0b010, 0), (0b100, 1), (0b110, 1)])
        );
        assert_eq!(info.support_variables, BTreeSet::from([StateVar(2)]));
        assert_eq!(info.support_size, 1);

        let not_redundant = FiniteStateSet::new(2, [0b00, 0b01]).unwrap();
        assert_eq!(
            extract_redundant_latch_info(&not_redundant, StateVar(0)),
            None
        );
    }

    #[test]
    fn first_fit_updates_reachable_set_and_respects_support_cost() {
        let reachable = FiniteStateSet::new(3, [0b000, 0b010, 0b101, 0b111]).unwrap();

        assert_eq!(
            first_fit_latch_removal_plan(&reachable, &[StateVar(0), StateVar(1)], 0),
            Vec::<RedundantLatchInfo>::new()
        );

        let plan = first_fit_latch_removal_plan(&reachable, &[StateVar(0), StateVar(1)], 2);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].variable, StateVar(0));
    }

    #[test]
    fn save_enough_latches_requires_internal_node_fed_only_by_non_real_latch_pis() {
        let network = LatchNetwork::new(
            [
                node(0, "lat0", NodeKind::PrimaryInput, &[]).with_real_pi(false),
                node(1, "lat1", NodeKind::PrimaryInput, &[]).with_real_pi(false),
                node(2, "n", NodeKind::Internal, &[0, 1]),
                node(3, "po", NodeKind::PrimaryOutput, &[2]),
                node(4, "d0", NodeKind::Internal, &[]),
                node(5, "d1", NodeKind::Internal, &[]),
            ],
            [latch(0, 4, 0, 0), latch(1, 5, 1, 1)],
        );

        assert_eq!(
            save_enough_latches(&network, NodeId(2)).unwrap(),
            Some(RetimingOpportunity {
                node: NodeId(2),
                latches_saved: 1,
            })
        );
        assert_eq!(save_enough_latches(&network, NodeId(3)).unwrap(), None);
    }

    #[test]
    fn candidate_search_finds_first_transitive_node_only_fed_by_latches() {
        let network = LatchNetwork::new(
            [
                node(0, "lat0", NodeKind::PrimaryInput, &[]).with_real_pi(false),
                node(1, "lat1", NodeKind::PrimaryInput, &[]).with_real_pi(false),
                node(2, "candidate", NodeKind::Internal, &[0, 1]),
                node(3, "top", NodeKind::Internal, &[2]),
                node(4, "po", NodeKind::PrimaryOutput, &[3]),
                node(5, "d0", NodeKind::Internal, &[]),
                node(6, "d1", NodeKind::Internal, &[]),
            ],
            [latch(0, 5, 0, 0), latch(1, 6, 1, 1)],
        );

        assert_eq!(node_only_fed_by_latches(&network, NodeId(2)), Ok(true));
        assert_eq!(get_candidate_node(&network, NodeId(3)), Ok(Some(NodeId(2))));
        assert_eq!(
            classify_latched_output_step(&network, NodeId(4)),
            Ok(LatchedOutputStatus::CandidateFound(NodeId(2)))
        );
    }

    #[test]
    fn latched_output_status_reports_already_latched_constant_and_dependency() {
        let latched = LatchNetwork::new(
            [
                node(0, "lat", NodeKind::PrimaryInput, &[]).with_real_pi(false),
                node(1, "po", NodeKind::PrimaryOutput, &[0]),
                node(2, "d", NodeKind::Internal, &[]),
            ],
            [latch(0, 2, 0, 1)],
        );
        assert_eq!(
            classify_latched_output_step(&latched, NodeId(1)),
            Ok(LatchedOutputStatus::AlreadyLatched)
        );

        let constant = LatchNetwork::new(
            [
                node(0, "one", NodeKind::Internal, &[]).with_constant(ConstantValue::One),
                node(1, "po", NodeKind::PrimaryOutput, &[0]),
            ],
            [],
        );
        assert_eq!(
            classify_latched_output_step(&constant, NodeId(1)),
            Ok(LatchedOutputStatus::ConstantFanin)
        );

        let dependency = LatchNetwork::new(
            [
                node(0, "in", NodeKind::PrimaryInput, &[]),
                node(1, "n", NodeKind::Internal, &[0]),
                node(2, "po", NodeKind::PrimaryOutput, &[1]),
            ],
            [],
        );
        assert_eq!(
            classify_latched_output_step(&dependency, NodeId(2)),
            Ok(LatchedOutputStatus::CombinationalDependency)
        );
    }

    #[test]
    fn boot_latch_removal_chooses_equivalent_state_with_flipped_bit() {
        let network = LatchNetwork::new(
            [
                node(0, "c1", NodeKind::Internal, &[]).with_constant(ConstantValue::One),
                node(1, "c0", NodeKind::Internal, &[]).with_constant(ConstantValue::Zero),
                node(2, "lat0", NodeKind::PrimaryInput, &[]).with_real_pi(false),
                node(3, "lat1", NodeKind::PrimaryInput, &[]).with_real_pi(false),
            ],
            [latch(0, 0, 2, 0), latch(1, 1, 3, 1)],
        );
        let signatures = HashMap::from([
            (0b00, vec![1, 0, 1]),
            (0b01, vec![1, 0, 1]),
            (0b10, vec![0, 0, 1]),
        ]);

        let removal = find_boot_latch_removal(&network, 0b00, 2, &signatures)
            .unwrap()
            .unwrap();

        assert_eq!(removal.latch, LatchId(0));
        assert_eq!(removal.replacement_constant, 0);
        assert_eq!(removal.new_initial_state, 0b01);
        assert_eq!(
            removal.new_latch_initial_values,
            BTreeMap::from([(LatchId(0), 1), (LatchId(1), 0)])
        );
    }

    #[test]
    fn binary_merge_uses_neutral_only_for_empty_input_and_pairs_like_c() {
        assert_eq!(binary_merge(Vec::<i32>::new(), 7, |a, b| a + b), 7);
        assert_eq!(binary_merge(vec![1, 2, 3, 4, 5], 0, |a, b| a + b), 15);

        let merged = binary_merge(
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
            String::new(),
            |a, b| format!("({a}{b})"),
        );
        assert_eq!(merged, "((ab)c)");
    }

    #[test]
    fn blocked_sis_entries_report_dependency_beads_and_source_files() {
        let error = remove_latches_from_sis_network(&mut ()).unwrap_err();

        match error {
            PrlRemLatchError::MissingNativePorts {
                operation,
                dependencies,
            } => {
                assert_eq!(operation, "Prl_RemoveLatches");
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.429"
                        && dependency.source_file == "LogicSynthesis/sis/seqbdd/network_info.c"
                }));
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.442"
                        && dependency.source_file == "LogicSynthesis/sis/seqbdd/verif_util.c"
                }));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("prl_remlatch.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
