//! Native Rust model for
//! `LogicSynthesis/sis/seqbdd/prl_extract.c`.
//!
//! The C file builds environment/FSM product networks, extracts sequential
//! don't-cares, verifies two FSMs under an environment, and prints shortest
//! counterexample traces. The real SIS implementation depends on unported
//! `network_t`, `node_t`, latch, BDD, ntbdd, array, and seqbdd option layers, so
//! SIS-bound entry points return generic missing-port diagnostics. The product
//! planning, output filtering, BDD-variable bookkeeping, breadth-first
//! traversal, and shortest-trace logic are represented with owned Rust data and
//! covered by tests.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrlExtractError {
    MissingNativePorts {
        operation: &'static str,
    },
    DuplicateProductOutputName {
        network_name: String,
        output_name: String,
    },
    MissingMatchingRealOutput {
        output_name: String,
    },
    EmptyInitialSet,
    VerificationFailed {
        output_index: usize,
        depth: usize,
    },
    NoCounterexampleTarget,
    NoReverseEdge {
        depth: usize,
    },
    InvalidBinaryValue(u8),
}

impl fmt::Display for PrlExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by missing native SIS ports")
            }
            Self::DuplicateProductOutputName {
                network_name,
                output_name,
            } => write!(
                f,
                "network \"{network_name}\" already has a PO named \"{output_name}\""
            ),
            Self::MissingMatchingRealOutput { output_name } => write!(
                f,
                "source FSM real PO \"{output_name}\" has no matching destination real PO"
            ),
            Self::EmptyInitialSet => write!(f, "environment/FSM traversal has no initial states"),
            Self::VerificationFailed {
                output_index,
                depth,
            } => write!(
                f,
                "environment/FSM verification failed at output {output_index} and BFS depth {depth}"
            ),
            Self::NoCounterexampleTarget => {
                write!(
                    f,
                    "could not find a failing reachable state for the selected output"
                )
            }
            Self::NoReverseEdge { depth } => {
                write!(
                    f,
                    "could not extract a predecessor edge at BFS depth {depth}"
                )
            }
            Self::InvalidBinaryValue(value) => {
                write!(f, "input and latch values must be 0 or 1, got {value}")
            }
        }
    }
}

impl Error for PrlExtractError {}

pub fn extract_env_dc_from_sis_networks<Network, Options>(
    _fsm_network: &mut Network,
    _env_network: &Network,
    _options: &Options,
) -> Result<(), PrlExtractError> {
    missing_native_ports("Prl_ExtractEnvDc")
}

pub fn verify_env_fsm_with_sis_networks<Network, Options>(
    _fsm_network: &Network,
    _check_network: &mut Network,
    _env_network: &Network,
    _options: &Options,
) -> Result<(), PrlExtractError> {
    missing_native_ports("Prl_VerifyEnvFsm")
}

fn missing_native_ports<T>(operation: &'static str) -> Result<T, PrlExtractError> {
    Err(PrlExtractError::MissingNativePorts { operation })
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
pub struct ExtractNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub is_real_pi: bool,
    pub is_real_po: bool,
}

impl ExtractNode {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractLatch {
    pub input: NodeId,
    pub output: NodeId,
    pub initial_value: u8,
}

impl ExtractLatch {
    pub fn new(input: NodeId, output: NodeId, initial_value: u8) -> Result<Self, PrlExtractError> {
        if initial_value > 1 {
            return Err(PrlExtractError::InvalidBinaryValue(initial_value));
        }

        Ok(Self {
            input,
            output,
            initial_value,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractNetwork {
    pub name: String,
    nodes: Vec<ExtractNode>,
    latches: Vec<ExtractLatch>,
}

impl ExtractNetwork {
    pub fn new(
        name: impl Into<String>,
        nodes: impl Into<Vec<ExtractNode>>,
        latches: impl Into<Vec<ExtractLatch>>,
    ) -> Self {
        Self {
            name: name.into(),
            nodes: nodes.into(),
            latches: latches.into(),
        }
    }

    pub fn nodes(&self) -> &[ExtractNode] {
        &self.nodes
    }

    pub fn latches(&self) -> &[ExtractLatch] {
        &self.latches
    }

    pub fn node(&self, id: NodeId) -> Option<&ExtractNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn find_node_by_name(&self, name: &str) -> Option<&ExtractNode> {
        self.nodes.iter().find(|node| node.name == name)
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = &ExtractNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryInput)
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = &ExtractNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryOutput)
    }

    pub fn is_latch_output(&self, node: NodeId) -> bool {
        self.latches.iter().any(|latch| latch.output == node)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductNodeRef {
    Existing(NodeId),
    CopiedEnvOutput(NodeId),
    CopiedFsmOutput(NodeId),
    XnorOutput { check_po: NodeId, copied_po: NodeId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Connection {
    pub input_name: String,
    pub copy_output: Option<ProductNodeRef>,
    pub pi: Option<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CopiedLatch {
    pub output_name: String,
    pub copied_output: ProductNodeRef,
    pub original_po: NodeId,
    pub initial_value: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvFsmProductPlan {
    pub connections: Vec<Connection>,
    pub copied_latches: Vec<CopiedLatch>,
    pub copied_real_outputs: Vec<(String, ProductNodeRef)>,
    pub deleted_real_inputs: Vec<NodeId>,
}

pub fn plan_add_env_fsm(
    to_network: &ExtractNetwork,
    env_network: &ExtractNetwork,
) -> Result<EnvFsmProductPlan, PrlExtractError> {
    let mut connections = to_network
        .primary_inputs()
        .filter(|input| input.is_real_pi)
        .map(|input| Connection {
            input_name: input.name.clone(),
            copy_output: None,
            pi: Some(input.id),
        })
        .collect::<Vec<_>>();
    let mut copied_latches = Vec::new();
    let mut copied_real_outputs = Vec::new();

    for env_output in env_network.primary_outputs() {
        let output_name = env_output.name.clone();
        let copied_output = ProductNodeRef::CopiedEnvOutput(env_output.id);
        if !env_output.is_real_po || env_network.is_latch_output(env_output.id) {
            let initial_value = env_network
                .latches()
                .iter()
                .find(|latch| latch.output == env_output.id)
                .map(|latch| latch.initial_value)
                .unwrap_or(0);
            copied_latches.push(CopiedLatch {
                output_name,
                copied_output,
                original_po: env_output.id,
                initial_value,
            });
            continue;
        }

        match to_network.find_node_by_name(&output_name) {
            Some(node) if node.kind == NodeKind::PrimaryInput && node.is_real_pi => {
                if let Some(connection) = connections
                    .iter_mut()
                    .find(|connection| connection.pi == Some(node.id))
                {
                    connection.copy_output = Some(copied_output);
                }
            }
            Some(_) => {
                return Err(PrlExtractError::DuplicateProductOutputName {
                    network_name: to_network.name.clone(),
                    output_name,
                });
            }
            None => copied_real_outputs.push((output_name, copied_output)),
        }
    }

    let deleted_real_inputs = connections
        .iter_mut()
        .filter_map(|connection| {
            connection.copy_output.as_ref()?;
            let deleted = connection.pi.take();
            deleted
        })
        .collect();

    Ok(EnvFsmProductPlan {
        connections,
        copied_latches,
        copied_real_outputs,
        deleted_real_inputs,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FsmProductPlan {
    pub xnor_outputs: Vec<ProductNodeRef>,
    pub other_outputs: Vec<NodeId>,
    pub copied_latches: Vec<CopiedLatch>,
}

pub fn plan_make_product_fsm(
    to_network: &ExtractNetwork,
    from_network: &ExtractNetwork,
) -> Result<FsmProductPlan, PrlExtractError> {
    let mut xnor_outputs = Vec::new();
    let mut other_outputs = Vec::new();
    let mut copied_latches = Vec::new();

    for from_output in from_network.primary_outputs() {
        let output_name = from_output.name.clone();
        let copied_output = ProductNodeRef::CopiedFsmOutput(from_output.id);
        if !from_output.is_real_po || from_network.is_latch_output(from_output.id) {
            let initial_value = from_network
                .latches()
                .iter()
                .find(|latch| latch.output == from_output.id)
                .map(|latch| latch.initial_value)
                .unwrap_or(0);
            copied_latches.push(CopiedLatch {
                output_name,
                copied_output,
                original_po: from_output.id,
                initial_value,
            });
            continue;
        }

        let Some(to_output) = to_network.find_node_by_name(&output_name) else {
            return Err(PrlExtractError::MissingMatchingRealOutput { output_name });
        };
        if to_output.kind != NodeKind::PrimaryOutput || !to_output.is_real_po {
            return Err(PrlExtractError::MissingMatchingRealOutput { output_name });
        }

        xnor_outputs.push(ProductNodeRef::XnorOutput {
            check_po: to_output.id,
            copied_po: from_output.id,
        });
        other_outputs.push(to_output.id);
    }

    Ok(FsmProductPlan {
        xnor_outputs,
        other_outputs,
        copied_latches,
    })
}

pub fn real_pos_removed_by_filter(
    network: &ExtractNetwork,
    used_outputs: &[NodeId],
) -> Vec<String> {
    let used = used_outputs.iter().copied().collect::<BTreeSet<_>>();
    network
        .primary_outputs()
        .filter(|output| output.is_real_po)
        .filter(|output| !used.contains(&output.id))
        .map(|output| output.name.clone())
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddVarId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeqInputOrigin {
    EnvNetwork,
    FsmState,
    FreeFsmInput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqInputVar {
    pub node: NodeId,
    pub var: BddVarId,
    pub origin: SeqInputOrigin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedBddVars {
    pub env_vars: Vec<BddVarId>,
    pub fsm_state_vars: Vec<BddVarId>,
}

pub fn extract_bdd_vars(inputs: &[SeqInputVar]) -> ExtractedBddVars {
    let mut env_vars = Vec::new();
    let mut fsm_state_vars = Vec::new();

    for input in inputs {
        match input.origin {
            SeqInputOrigin::EnvNetwork => env_vars.push(input.var),
            SeqInputOrigin::FsmState => fsm_state_vars.push(input.var),
            SeqInputOrigin::FreeFsmInput => {}
        }
    }

    ExtractedBddVars {
        env_vars,
        fsm_state_vars,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputRelationPlan {
    pub added_input_names: Vec<String>,
    pub added_vars: Vec<BddVarId>,
    pub xnor_terms: Vec<(BddVarId, ProductNodeRef)>,
}

pub fn plan_make_fsm_input_relation(
    connections: &[Connection],
    first_new_var: usize,
) -> InputRelationPlan {
    let mut next_var = first_new_var;
    let mut added_input_names = Vec::new();
    let mut added_vars = Vec::new();
    let mut xnor_terms = Vec::new();

    for connection in connections {
        if let Some(copy_output) = connection.copy_output.clone() {
            let var = BddVarId(next_var);
            next_var += 1;
            added_input_names.push(connection.input_name.clone());
            added_vars.push(var);
            xnor_terms.push((var, copy_output));
        }
    }

    InputRelationPlan {
        added_input_names,
        added_vars,
        xnor_terms,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteStateSet<S> {
    states: BTreeSet<S>,
}

impl<S> FiniteStateSet<S>
where
    S: Ord,
{
    pub fn empty() -> Self {
        Self {
            states: BTreeSet::new(),
        }
    }

    pub fn from_states(states: impl IntoIterator<Item = S>) -> Self {
        Self {
            states: states.into_iter().collect(),
        }
    }

    pub fn states(&self) -> &BTreeSet<S> {
        &self.states
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        self.states.is_subset(&other.states)
    }

    pub fn union(&self, other: &Self) -> Self
    where
        S: Clone,
    {
        Self::from_states(self.states.union(&other.states).cloned())
    }

    pub fn difference(&self, other: &Self) -> Self
    where
        S: Clone,
    {
        Self::from_states(self.states.difference(&other.states).cloned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraversalFailure {
    pub output_index: usize,
    pub depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraversalOutcome<S> {
    pub verified: bool,
    pub depth: usize,
    pub total_set: FiniteStateSet<S>,
    pub failure: Option<TraversalFailure>,
    pub total_sets: Vec<FiniteStateSet<S>>,
}

pub fn breadth_first_traversal<S, Check, Next>(
    initial_set: FiniteStateSet<S>,
    mut check: Check,
    mut compute_next: Next,
) -> Result<TraversalOutcome<S>, PrlExtractError>
where
    S: Clone + Ord,
    Check: FnMut(&FiniteStateSet<S>, &FiniteStateSet<S>) -> Option<usize>,
    Next: FnMut(&FiniteStateSet<S>) -> FiniteStateSet<S>,
{
    if initial_set.is_empty() {
        return Err(PrlExtractError::EmptyInitialSet);
    }

    let mut depth = 0;
    let mut current_set = initial_set;
    let mut total_set = current_set.clone();
    let mut total_sets = Vec::new();

    loop {
        total_sets.push(total_set.clone());
        if let Some(output_index) = check(&current_set, &total_set) {
            return Ok(TraversalOutcome {
                verified: false,
                depth,
                total_set,
                failure: Some(TraversalFailure {
                    output_index,
                    depth,
                }),
                total_sets,
            });
        }

        let new_current_set = compute_next(&current_set);
        if new_current_set.is_subset(&total_set) {
            return Ok(TraversalOutcome {
                verified: true,
                depth,
                total_set,
                failure: None,
                total_sets,
            });
        }

        current_set = new_current_set.difference(&total_set);
        total_set = total_set.union(&new_current_set);
        depth += 1;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge<S, I> {
    pub from: S,
    pub input: I,
    pub to: S,
}

impl<S, I> Edge<S, I> {
    pub fn new(from: S, input: I, to: S) -> Self {
        Self { from, input, to }
    }
}

#[derive(Clone, Debug)]
pub struct ExplicitEnvFsm<S, I> {
    initial_states: BTreeSet<S>,
    edges: Vec<Edge<S, I>>,
    failing_outputs: HashMap<S, BTreeSet<usize>>,
    fsm_input_values: HashMap<I, Vec<u8>>,
}

impl<S, I> ExplicitEnvFsm<S, I>
where
    S: Clone + Eq + Hash + Ord,
    I: Clone + Eq + Hash,
{
    pub fn new(initial_states: impl IntoIterator<Item = S>) -> Self {
        Self {
            initial_states: initial_states.into_iter().collect(),
            edges: Vec::new(),
            failing_outputs: HashMap::new(),
            fsm_input_values: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, from: S, input: I, to: S) {
        self.edges.push(Edge::new(from, input, to));
    }

    pub fn add_failing_output(&mut self, state: S, output_index: usize) {
        self.failing_outputs
            .entry(state)
            .or_default()
            .insert(output_index);
    }

    pub fn set_fsm_input_values(
        &mut self,
        input: I,
        values: impl Into<Vec<u8>>,
    ) -> Result<(), PrlExtractError> {
        let values = values.into();
        if let Some(value) = values.iter().find(|value| **value > 1) {
            return Err(PrlExtractError::InvalidBinaryValue(*value));
        }
        self.fsm_input_values.insert(input, values);
        Ok(())
    }

    pub fn initial_set(&self) -> FiniteStateSet<S> {
        FiniteStateSet::from_states(self.initial_states.iter().cloned())
    }

    pub fn successors(&self, current: &FiniteStateSet<S>) -> FiniteStateSet<S> {
        FiniteStateSet::from_states(
            self.edges
                .iter()
                .filter(|edge| current.states.contains(&edge.from))
                .map(|edge| edge.to.clone()),
        )
    }

    pub fn failing_output_in(&self, states: &FiniteStateSet<S>) -> Option<usize> {
        states.states.iter().find_map(|state| {
            self.failing_outputs
                .get(state)
                .and_then(|outputs| outputs.iter().next().copied())
        })
    }

    pub fn shortest_error_trace(&self) -> Result<ErrorTrace<S, I>, PrlExtractError> {
        let outcome = breadth_first_traversal(
            self.initial_set(),
            |current, _total| self.failing_output_in(current),
            |current| self.successors(current),
        )?;
        let Some(failure) = outcome.failure else {
            return Err(PrlExtractError::NoCounterexampleTarget);
        };
        let target = outcome.total_sets[failure.depth]
            .states
            .iter()
            .find(|state| {
                self.failing_outputs
                    .get(*state)
                    .is_some_and(|outputs| outputs.contains(&failure.output_index))
            })
            .cloned()
            .ok_or(PrlExtractError::NoCounterexampleTarget)?;

        let mut states = VecDeque::from([target.clone()]);
        let mut inputs = VecDeque::new();
        let mut current = target;

        for depth in (1..=failure.depth).rev() {
            let allowed_sources = &outcome.total_sets[depth - 1].states;
            let edge = self
                .edges
                .iter()
                .find(|edge| edge.to == current && allowed_sources.contains(&edge.from))
                .ok_or(PrlExtractError::NoReverseEdge { depth })?;
            current = edge.from.clone();
            states.push_front(current.clone());
            inputs.push_front(edge.input.clone());
        }

        let simulated_fsm_inputs = inputs
            .iter()
            .map(|input| {
                self.fsm_input_values
                    .get(input)
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();

        Ok(ErrorTrace {
            output_index: failure.output_index,
            states: states.into_iter().collect(),
            env_inputs: inputs.into_iter().collect(),
            simulated_fsm_inputs,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorTrace<S, I> {
    pub output_index: usize,
    pub states: Vec<S>,
    pub env_inputs: Vec<I>,
    pub simulated_fsm_inputs: Vec<Vec<u8>>,
}

pub fn connection_header(connections: &[Connection]) -> String {
    let mut text = connections
        .iter()
        .map(|connection| connection.input_name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    text.push('\n');
    text
}

pub fn format_simulation_trace(rows: &[Vec<u8>]) -> Result<String, PrlExtractError> {
    let mut text = String::new();
    for row in rows {
        text.push_str("simulate ");
        for value in row {
            if *value > 1 {
                return Err(PrlExtractError::InvalidBinaryValue(*value));
            }
            text.push_str(&value.to_string());
            text.push(' ');
        }
        text.push('\n');
    }
    Ok(text)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvDcDisposition {
    RemoveDcNetwork,
    StoreSingleOutputDcNetwork,
}

pub fn env_dc_disposition(is_seq_dc_zero: bool) -> EnvDcDisposition {
    if is_seq_dc_zero {
        EnvDcDisposition::RemoveDcNetwork
    } else {
        EnvDcDisposition::StoreSingleOutputDcNetwork
    }
}

pub fn reachable_transition_complement(
    all_assignments: impl IntoIterator<Item = u64>,
    reachable_transitions: impl IntoIterator<Item = u64>,
) -> BTreeSet<u64> {
    let all = all_assignments.into_iter().collect::<BTreeSet<_>>();
    let reachable = reachable_transitions.into_iter().collect::<BTreeSet<_>>();
    all.difference(&reachable).copied().collect()
}

pub fn state_input_patterns_after_env_smoothing(
    reachable_product_transitions: impl IntoIterator<Item = u64>,
    fsm_input_relation: impl IntoIterator<Item = u64>,
    env_mask: u64,
) -> BTreeSet<u64> {
    let relation = fsm_input_relation.into_iter().collect::<BTreeSet<_>>();
    reachable_product_transitions
        .into_iter()
        .filter(|pattern| relation.contains(pattern))
        .map(|pattern| pattern & !env_mask)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize, name: &str, kind: NodeKind) -> ExtractNode {
        ExtractNode::new(NodeId(id), name, kind)
    }

    fn fsm_network() -> ExtractNetwork {
        ExtractNetwork::new(
            "fsm",
            vec![
                node(0, "a", NodeKind::PrimaryInput),
                node(1, "b", NodeKind::PrimaryInput),
                node(2, "out", NodeKind::PrimaryOutput),
                node(3, "state", NodeKind::PrimaryInput).with_real_pi(false),
            ],
            Vec::new(),
        )
    }

    #[test]
    fn add_env_fsm_records_controlled_inputs_and_deletes_replaced_pis() {
        let fsm = fsm_network();
        let env = ExtractNetwork::new(
            "env",
            vec![
                node(10, "a", NodeKind::PrimaryOutput),
                node(11, "free_env_output", NodeKind::PrimaryOutput),
            ],
            Vec::new(),
        );

        let plan = plan_add_env_fsm(&fsm, &env).unwrap();

        assert_eq!(
            plan.connections,
            vec![
                Connection {
                    input_name: "a".to_owned(),
                    copy_output: Some(ProductNodeRef::CopiedEnvOutput(NodeId(10))),
                    pi: None,
                },
                Connection {
                    input_name: "b".to_owned(),
                    copy_output: None,
                    pi: Some(NodeId(1)),
                },
            ]
        );
        assert_eq!(plan.deleted_real_inputs, vec![NodeId(0)]);
        assert_eq!(
            plan.copied_real_outputs,
            vec![(
                "free_env_output".to_owned(),
                ProductNodeRef::CopiedEnvOutput(NodeId(11))
            )]
        );
    }

    #[test]
    fn add_env_fsm_rejects_real_env_output_name_conflict_that_is_not_real_pi() {
        let fsm = fsm_network();
        let env = ExtractNetwork::new(
            "env",
            vec![node(10, "out", NodeKind::PrimaryOutput)],
            Vec::new(),
        );

        assert_eq!(
            plan_add_env_fsm(&fsm, &env),
            Err(PrlExtractError::DuplicateProductOutputName {
                network_name: "fsm".to_owned(),
                output_name: "out".to_owned(),
            })
        );
    }

    #[test]
    fn add_env_fsm_copies_latch_outputs_without_creating_connections() {
        let fsm = fsm_network();
        let env = ExtractNetwork::new(
            "env",
            vec![
                node(20, "next_s", NodeKind::Internal),
                node(21, "s", NodeKind::PrimaryOutput).with_real_po(false),
            ],
            vec![ExtractLatch::new(NodeId(20), NodeId(21), 1).unwrap()],
        );

        let plan = plan_add_env_fsm(&fsm, &env).unwrap();

        assert_eq!(
            plan.copied_latches,
            vec![CopiedLatch {
                output_name: "s".to_owned(),
                copied_output: ProductNodeRef::CopiedEnvOutput(NodeId(21)),
                original_po: NodeId(21),
                initial_value: 1,
            }]
        );
        assert!(plan.deleted_real_inputs.is_empty());
    }

    #[test]
    fn product_fsm_pairs_matching_real_outputs_with_xnor_outputs() {
        let check = ExtractNetwork::new(
            "check",
            vec![node(0, "out", NodeKind::PrimaryOutput)],
            Vec::new(),
        );
        let fsm = ExtractNetwork::new(
            "fsm",
            vec![node(10, "out", NodeKind::PrimaryOutput)],
            Vec::new(),
        );

        let plan = plan_make_product_fsm(&check, &fsm).unwrap();

        assert_eq!(
            plan.xnor_outputs,
            vec![ProductNodeRef::XnorOutput {
                check_po: NodeId(0),
                copied_po: NodeId(10),
            }]
        );
        assert_eq!(plan.other_outputs, vec![NodeId(0)]);
    }

    #[test]
    fn product_fsm_reports_missing_matching_real_output() {
        let check = ExtractNetwork::new(
            "check",
            vec![node(0, "other", NodeKind::PrimaryOutput)],
            Vec::new(),
        );
        let fsm = ExtractNetwork::new(
            "fsm",
            vec![node(10, "out", NodeKind::PrimaryOutput)],
            Vec::new(),
        );

        assert_eq!(
            plan_make_product_fsm(&check, &fsm),
            Err(PrlExtractError::MissingMatchingRealOutput {
                output_name: "out".to_owned(),
            })
        );
    }

    #[test]
    fn remove_unused_pos_returns_real_outputs_not_marked_as_xnor_outputs() {
        let network = ExtractNetwork::new(
            "check",
            vec![
                node(0, "keep", NodeKind::PrimaryOutput),
                node(1, "drop", NodeKind::PrimaryOutput),
                node(2, "latch", NodeKind::PrimaryOutput).with_real_po(false),
            ],
            Vec::new(),
        );

        assert_eq!(
            real_pos_removed_by_filter(&network, &[NodeId(0)]),
            vec!["drop".to_owned()]
        );
    }

    #[test]
    fn extract_bdd_vars_splits_env_vars_from_fsm_state_vars() {
        assert_eq!(
            extract_bdd_vars(&[
                SeqInputVar {
                    node: NodeId(0),
                    var: BddVarId(0),
                    origin: SeqInputOrigin::EnvNetwork,
                },
                SeqInputVar {
                    node: NodeId(1),
                    var: BddVarId(1),
                    origin: SeqInputOrigin::FreeFsmInput,
                },
                SeqInputVar {
                    node: NodeId(2),
                    var: BddVarId(2),
                    origin: SeqInputOrigin::FsmState,
                },
            ]),
            ExtractedBddVars {
                env_vars: vec![BddVarId(0)],
                fsm_state_vars: vec![BddVarId(2)],
            }
        );
    }

    #[test]
    fn input_relation_adds_new_vars_only_for_controlled_fsm_inputs() {
        let connections = vec![
            Connection {
                input_name: "a".to_owned(),
                copy_output: Some(ProductNodeRef::CopiedEnvOutput(NodeId(10))),
                pi: None,
            },
            Connection {
                input_name: "b".to_owned(),
                copy_output: None,
                pi: Some(NodeId(1)),
            },
            Connection {
                input_name: "c".to_owned(),
                copy_output: Some(ProductNodeRef::CopiedEnvOutput(NodeId(12))),
                pi: None,
            },
        ];

        assert_eq!(
            plan_make_fsm_input_relation(&connections, 4),
            InputRelationPlan {
                added_input_names: vec!["a".to_owned(), "c".to_owned()],
                added_vars: vec![BddVarId(4), BddVarId(5)],
                xnor_terms: vec![
                    (BddVarId(4), ProductNodeRef::CopiedEnvOutput(NodeId(10))),
                    (BddVarId(5), ProductNodeRef::CopiedEnvOutput(NodeId(12))),
                ],
            }
        );
    }

    #[test]
    fn breadth_first_traversal_matches_c_stop_when_next_set_is_covered() {
        let outcome = breadth_first_traversal(
            FiniteStateSet::from_states([0_u8]),
            |_current, _total| None,
            |current| {
                if current.states().contains(&0) {
                    FiniteStateSet::from_states([1])
                } else if current.states().contains(&1) {
                    FiniteStateSet::from_states([1])
                } else {
                    FiniteStateSet::empty()
                }
            },
        )
        .unwrap();

        assert_eq!(outcome.verified, true);
        assert_eq!(outcome.depth, 1);
        assert_eq!(outcome.total_set, FiniteStateSet::from_states([0, 1]));
    }

    #[test]
    fn explicit_env_fsm_extracts_shortest_error_trace_and_simulated_fsm_inputs() {
        let mut machine = ExplicitEnvFsm::new([0_u8]);
        machine.add_edge(0, "i0", 1);
        machine.add_edge(1, "i1", 2);
        machine.add_edge(0, "slow", 3);
        machine.add_edge(3, "slow2", 4);
        machine.add_edge(4, "slow3", 2);
        machine.add_failing_output(2, 7);
        machine.set_fsm_input_values("i0", vec![1, 0]).unwrap();
        machine.set_fsm_input_values("i1", vec![0, 1]).unwrap();

        let trace = machine.shortest_error_trace().unwrap();

        assert_eq!(
            trace,
            ErrorTrace {
                output_index: 7,
                states: vec![0, 1, 2],
                env_inputs: vec!["i0", "i1"],
                simulated_fsm_inputs: vec![vec![1, 0], vec![0, 1]],
            }
        );
    }

    #[test]
    fn formatting_helpers_match_c_spacing() {
        let connections = vec![
            Connection {
                input_name: "a".to_owned(),
                copy_output: None,
                pi: Some(NodeId(0)),
            },
            Connection {
                input_name: "b".to_owned(),
                copy_output: Some(ProductNodeRef::CopiedEnvOutput(NodeId(9))),
                pi: None,
            },
        ];

        assert_eq!(connection_header(&connections), "a b\n");
        assert_eq!(
            format_simulation_trace(&[vec![1, 0], vec![0, 1]]).unwrap(),
            "simulate 1 0 \nsimulate 0 1 \n"
        );
    }

    #[test]
    fn env_dc_set_helpers_model_relation_and_complement_steps() {
        assert_eq!(
            state_input_patterns_after_env_smoothing(
                [0b0001, 0b0011, 0b0101],
                [0b0011, 0b0101],
                0b0100
            ),
            BTreeSet::from([0b0011, 0b0001])
        );
        assert_eq!(
            reachable_transition_complement([0, 1, 2, 3], [1, 3]),
            BTreeSet::from([0, 2])
        );
        assert_eq!(env_dc_disposition(true), EnvDcDisposition::RemoveDcNetwork);
        assert_eq!(
            env_dc_disposition(false),
            EnvDcDisposition::StoreSingleOutputDcNetwork
        );
    }
    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("prl_extract.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
