//! Native Rust model for `LogicSynthesis/sis/seqbdd/consistency.c`.
//!
//! The C implementation stores a BDD-backed transition relation, derives the
//! present-state/next-state variable pairing from latches, and computes forward
//! and reverse images by smoothing and substitution. This module ports that
//! behavior onto owned Rust data structures so the semantics are testable while
//! direct SIS network, st_table, array_t, ntbdd, and BDD-manager integration
//! remains represented by explicit dependency errors.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub note: &'static str,
}

pub const REQUIRED_CONSISTENCY_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        note: "array_t allocation, fetch, insertion, and release used for ordered node and BDD arrays",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.71",
        source_file: "LogicSynthesis/sis/bdd_cmu/bdd_port/bddport.c",
        note: "BDD manager bridge, bdd_leq, bdd_size, variable extraction, and BDD lifetime calls",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.75",
        source_file: "LogicSynthesis/sis/bdd_ucb/and_smooth.c",
        note: "bdd_and_smooth used by forward and reverse image computation",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.89",
        source_file: "LogicSynthesis/sis/bdd_ucb/bdd_substit.c",
        note: "bdd_substitute used to rename present-state and next-state variables",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.230",
        source_file: "LogicSynthesis/sis/latch/latch.c",
        note: "latch input/output traversal for state variable matching",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "network latch iteration and node name lookup",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "node identity, type, names, and BDD attachment lifetime",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.326",
        source_file: "LogicSynthesis/sis/ntbdd/bdd_at_node.c",
        note: "ntbdd_node_to_bdd and ntbdd_free_at_node for init, output, and consistency nodes",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.329",
        source_file: "LogicSynthesis/sis/ntbdd/manager.c",
        note: "ntbdd_start_manager and ntbdd_end_manager",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.442",
        source_file: "LogicSynthesis/sis/seqbdd/verif_util.c",
        note: "output_info construction, bdd_extract_var_array, and report_inconsistency",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        note: "st_table name and pointer maps used while pairing state variables",
    },
];

pub fn required_consistency_dependencies() -> &'static [PortDependency] {
    REQUIRED_CONSISTENCY_DEPENDENCIES
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VarId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub bdd_var: Option<VarId>,
}

impl SeqNode {
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            bdd_var: None,
        }
    }

    pub fn with_bdd_var(mut self, var: VarId) -> Self {
        self.bdd_var = Some(var);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqLatch {
    pub input: NodeId,
    pub output: NodeId,
}

impl SeqLatch {
    pub fn new(input: NodeId, output: NodeId) -> Self {
        Self { input, output }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqNetwork {
    nodes: BTreeMap<NodeId, SeqNode>,
    latches: Vec<SeqLatch>,
}

impl SeqNetwork {
    pub fn new(
        nodes: impl IntoIterator<Item = SeqNode>,
        latches: impl Into<Vec<SeqLatch>>,
    ) -> Self {
        Self {
            nodes: nodes.into_iter().map(|node| (node.id, node)).collect(),
            latches: latches.into(),
        }
    }

    pub fn node(&self, id: NodeId) -> Option<&SeqNode> {
        self.nodes.get(&id)
    }

    pub fn node_name(&self, id: NodeId) -> Option<&str> {
        self.node(id).map(|node| node.name.as_str())
    }

    pub fn latches(&self) -> &[SeqLatch] {
        &self.latches
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConsistencyError {
    MissingSisDependencies {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    UnknownNode(NodeId),
    DuplicatePiOrdering(NodeId),
    MissingPiOrdering(NodeId),
    MissingBddVariable(NodeId),
    LatchInputNotInPoOrdering {
        latch_input: NodeId,
        name: String,
    },
    LatchOutputIsNotPrimaryInput {
        latch_output: NodeId,
        kind: NodeKind,
    },
    NewPiLengthMismatch {
        po_ordering_len: usize,
        new_pi_len: usize,
    },
    StateVectorWidthMismatch {
        expected: usize,
        actual: usize,
    },
    TransitionWidthMismatch {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for ConsistencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} unported SIS/BDD dependencies",
                dependencies.len()
            ),
            Self::UnknownNode(node) => write!(f, "unknown seqbdd consistency node {:?}", node),
            Self::DuplicatePiOrdering(node) => {
                write!(f, "duplicate PI ordering entry for {:?}", node)
            }
            Self::MissingPiOrdering(node) => write!(f, "missing PI ordering for {:?}", node),
            Self::MissingBddVariable(node) => write!(f, "missing BDD variable for {:?}", node),
            Self::LatchInputNotInPoOrdering { latch_input, name } => write!(
                f,
                "latch input {:?} ({name}) is not present in the PO ordering",
                latch_input
            ),
            Self::LatchOutputIsNotPrimaryInput { latch_output, kind } => write!(
                f,
                "latch output {:?} has kind {:?}; expected primary input",
                latch_output, kind
            ),
            Self::NewPiLengthMismatch {
                po_ordering_len,
                new_pi_len,
            } => write!(
                f,
                "PO ordering length {po_ordering_len} does not match new PI length {new_pi_len}"
            ),
            Self::StateVectorWidthMismatch { expected, actual } => {
                write!(f, "state vector has width {actual}; expected {expected}")
            }
            Self::TransitionWidthMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "transition {field} vector has width {actual}; expected {expected}"
            ),
        }
    }
}

impl Error for ConsistencyError {}

pub type InputToOutputTable = BTreeMap<NodeId, NodeId>;

pub fn extract_input_to_output_table(
    network: &SeqNetwork,
    org_pi: &[NodeId],
    new_pi: &[NodeId],
    po_ordering: &[NodeId],
) -> Result<InputToOutputTable, ConsistencyError> {
    if po_ordering.len() != new_pi.len() {
        return Err(ConsistencyError::NewPiLengthMismatch {
            po_ordering_len: po_ordering.len(),
            new_pi_len: new_pi.len(),
        });
    }

    let mut po_index_by_name = HashMap::new();
    for (index, output) in po_ordering.iter().copied().enumerate() {
        let name = network
            .node_name(output)
            .ok_or(ConsistencyError::UnknownNode(output))?;
        po_index_by_name.insert(name.to_owned(), index);
    }

    let mut latch_output_name_to_po_index = HashMap::new();
    for latch in network.latches() {
        let output_node = network
            .node(latch.output)
            .ok_or(ConsistencyError::UnknownNode(latch.output))?;
        if output_node.kind != NodeKind::PrimaryInput {
            return Err(ConsistencyError::LatchOutputIsNotPrimaryInput {
                latch_output: latch.output,
                kind: output_node.kind,
            });
        }

        let input_name = network
            .node_name(latch.input)
            .ok_or(ConsistencyError::UnknownNode(latch.input))?;
        let index = po_index_by_name.get(input_name).copied().ok_or_else(|| {
            ConsistencyError::LatchInputNotInPoOrdering {
                latch_input: latch.input,
                name: input_name.to_owned(),
            }
        })?;
        latch_output_name_to_po_index.insert(output_node.name.clone(), index);
    }

    let mut result = BTreeMap::new();
    for ps_node in org_pi {
        let name = network
            .node_name(*ps_node)
            .ok_or(ConsistencyError::UnknownNode(*ps_node))?;
        if let Some(index) = latch_output_name_to_po_index.get(name).copied() {
            result.insert(*ps_node, new_pi[index]);
        }
    }

    Ok(result)
}

pub fn pi_ordering_from_slice(
    nodes: &[NodeId],
) -> Result<BTreeMap<NodeId, usize>, ConsistencyError> {
    let mut ordering = BTreeMap::new();
    for (index, node) in nodes.iter().copied().enumerate() {
        if ordering.insert(node, index).is_some() {
            return Err(ConsistencyError::DuplicatePiOrdering(node));
        }
    }
    Ok(ordering)
}

pub fn extract_state_input_vars(
    network: &SeqNetwork,
    pi_ordering: &BTreeMap<NodeId, usize>,
    input_to_output: &InputToOutputTable,
) -> Result<Vec<VarId>, ConsistencyError> {
    extract_state_vars(network, pi_ordering, input_to_output, StateVarSide::Input)
}

pub fn extract_state_output_vars(
    network: &SeqNetwork,
    pi_ordering: &BTreeMap<NodeId, usize>,
    input_to_output: &InputToOutputTable,
) -> Result<Vec<VarId>, ConsistencyError> {
    extract_state_vars(network, pi_ordering, input_to_output, StateVarSide::Output)
}

#[derive(Clone, Copy)]
enum StateVarSide {
    Input,
    Output,
}

fn extract_state_vars(
    network: &SeqNetwork,
    pi_ordering: &BTreeMap<NodeId, usize>,
    input_to_output: &InputToOutputTable,
    side: StateVarSide,
) -> Result<Vec<VarId>, ConsistencyError> {
    let mut ordered = vec![None; pi_ordering.len()];
    for (ps_node, ns_node) in input_to_output {
        let index = *pi_ordering
            .get(ps_node)
            .ok_or(ConsistencyError::MissingPiOrdering(*ps_node))?;
        let selected = match side {
            StateVarSide::Input => *ps_node,
            StateVarSide::Output => *ns_node,
        };
        let node = network
            .node(selected)
            .ok_or(ConsistencyError::UnknownNode(selected))?;
        ordered[index] = Some(
            node.bdd_var
                .ok_or(ConsistencyError::MissingBddVariable(selected))?,
        );
    }

    Ok(ordered.into_iter().flatten().collect())
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateVector(Vec<bool>);

impl StateVector {
    pub fn new(bits: impl Into<Vec<bool>>) -> Self {
        Self(bits.into())
    }

    pub fn bits(&self) -> &[bool] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSet {
    width: usize,
    states: BTreeSet<StateVector>,
}

impl StateSet {
    pub fn empty(width: usize) -> Self {
        Self {
            width,
            states: BTreeSet::new(),
        }
    }

    pub fn from_states(
        width: usize,
        states: impl IntoIterator<Item = StateVector>,
    ) -> Result<Self, ConsistencyError> {
        let mut result = BTreeSet::new();
        for state in states {
            if state.bits().len() != width {
                return Err(ConsistencyError::StateVectorWidthMismatch {
                    expected: width,
                    actual: state.bits().len(),
                });
            }
            result.insert(state);
        }
        Ok(Self {
            width,
            states: result,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn contains(&self, state: &StateVector) -> bool {
        self.states.contains(state)
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        self.width == other.width && self.states.is_subset(&other.states)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub current_state: StateVector,
    pub inputs: StateVector,
    pub next_state: StateVector,
}

impl Transition {
    pub fn new(current_state: StateVector, inputs: StateVector, next_state: StateVector) -> Self {
        Self {
            current_state,
            inputs,
            next_state,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsistencyRangeData {
    state_width: usize,
    external_input_width: usize,
    transitions: Vec<Transition>,
    output_allowed: Option<StateSet>,
}

impl ConsistencyRangeData {
    pub fn new(
        state_width: usize,
        external_input_width: usize,
        transitions: impl Into<Vec<Transition>>,
        output_allowed: Option<StateSet>,
    ) -> Result<Self, ConsistencyError> {
        let transitions = transitions.into();
        for transition in &transitions {
            validate_transition_widths(state_width, external_input_width, transition)?;
        }
        if let Some(output_allowed) = &output_allowed {
            if output_allowed.width() != state_width {
                return Err(ConsistencyError::StateVectorWidthMismatch {
                    expected: state_width,
                    actual: output_allowed.width(),
                });
            }
        }
        Ok(Self {
            state_width,
            external_input_width,
            transitions,
            output_allowed,
        })
    }

    pub fn state_width(&self) -> usize {
        self.state_width
    }

    pub fn external_input_width(&self) -> usize {
        self.external_input_width
    }

    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }
}

fn validate_transition_widths(
    state_width: usize,
    external_input_width: usize,
    transition: &Transition,
) -> Result<(), ConsistencyError> {
    validate_width(
        "current_state",
        state_width,
        transition.current_state.bits().len(),
    )?;
    validate_width(
        "external_inputs",
        external_input_width,
        transition.inputs.bits().len(),
    )?;
    validate_width(
        "next_state",
        state_width,
        transition.next_state.bits().len(),
    )
}

fn validate_width(
    field: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), ConsistencyError> {
    if expected == actual {
        Ok(())
    } else {
        Err(ConsistencyError::TransitionWidthMismatch {
            field,
            expected,
            actual,
        })
    }
}

pub fn consistency_compute_next_states(
    current_set: &StateSet,
    data: &ConsistencyRangeData,
) -> Result<StateSet, ConsistencyError> {
    if current_set.width() != data.state_width {
        return Err(ConsistencyError::StateVectorWidthMismatch {
            expected: data.state_width,
            actual: current_set.width(),
        });
    }

    let next_states = data
        .transitions
        .iter()
        .filter(|transition| current_set.contains(&transition.current_state))
        .map(|transition| transition.next_state.clone());
    StateSet::from_states(data.state_width, next_states)
}

pub fn consistency_compute_reverse_image(
    next_set: &StateSet,
    data: &ConsistencyRangeData,
) -> Result<StateSet, ConsistencyError> {
    if next_set.width() != data.state_width {
        return Err(ConsistencyError::StateVectorWidthMismatch {
            expected: data.state_width,
            actual: next_set.width(),
        });
    }

    let current_states = data
        .transitions
        .iter()
        .filter(|transition| next_set.contains(&transition.next_state))
        .map(|transition| transition.current_state.clone());
    StateSet::from_states(data.state_width, current_states)
}

pub fn consistency_check_output(
    current_set: &StateSet,
    data: &ConsistencyRangeData,
) -> Result<Option<StateSet>, ConsistencyError> {
    if current_set.width() != data.state_width {
        return Err(ConsistencyError::StateVectorWidthMismatch {
            expected: data.state_width,
            actual: current_set.width(),
        });
    }

    let Some(output_allowed) = &data.output_allowed else {
        return Ok(None);
    };

    if current_set.is_subset(output_allowed) {
        Ok(None)
    } else {
        let failing = current_set
            .states
            .difference(&output_allowed.states)
            .cloned()
            .collect::<Vec<_>>();
        Ok(Some(StateSet::from_states(data.state_width, failing)?))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsistencyBddSizes {
    pub consistency_fn_size: usize,
    pub output_fn_size: usize,
}

pub fn consistency_bdd_sizes(data: &ConsistencyRangeData) -> ConsistencyBddSizes {
    ConsistencyBddSizes {
        consistency_fn_size: data.transitions.len(),
        output_fn_size: data.output_allowed.as_ref().map_or(0, StateSet::len),
    }
}

pub fn consistency_alloc_range_data() -> Result<ConsistencyRangeData, ConsistencyError> {
    missing_sis_dependencies("consistency_alloc_range_data")
}

pub fn consistency_free_range_data() -> Result<(), ConsistencyError> {
    missing_sis_dependencies("consistency_free_range_data")
}

pub fn consistency_sis_compute_next_states() -> Result<(), ConsistencyError> {
    missing_sis_dependencies("consistency_compute_next_states")
}

pub fn consistency_sis_compute_reverse_image() -> Result<(), ConsistencyError> {
    missing_sis_dependencies("consistency_compute_reverse_image")
}

pub fn consistency_sis_check_output() -> Result<(), ConsistencyError> {
    missing_sis_dependencies("consistency_check_output")
}

fn missing_sis_dependencies<T>(operation: &'static str) -> Result<T, ConsistencyError> {
    Err(ConsistencyError::MissingSisDependencies {
        operation,
        dependencies: REQUIRED_CONSISTENCY_DEPENDENCIES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(id: usize, name: &str, kind: NodeKind, var: usize) -> SeqNode {
        SeqNode::new(NodeId(id), name, kind).with_bdd_var(VarId(var))
    }

    fn sample_network() -> SeqNetwork {
        SeqNetwork::new(
            [
                n(0, "s0", NodeKind::PrimaryInput, 0),
                n(1, "s1", NodeKind::PrimaryInput, 1),
                n(2, "ns0_po", NodeKind::PrimaryOutput, 2),
                n(3, "ns1_po", NodeKind::PrimaryOutput, 3),
                n(4, "ns0_pi", NodeKind::PrimaryInput, 4),
                n(5, "ns1_pi", NodeKind::PrimaryInput, 5),
            ],
            [
                SeqLatch::new(NodeId(2), NodeId(0)),
                SeqLatch::new(NodeId(3), NodeId(1)),
            ],
        )
    }

    fn state(bits: &[bool]) -> StateVector {
        StateVector::new(bits.to_vec())
    }

    fn relation() -> ConsistencyRangeData {
        ConsistencyRangeData::new(
            2,
            1,
            [
                Transition::new(
                    state(&[false, false]),
                    state(&[false]),
                    state(&[true, false]),
                ),
                Transition::new(
                    state(&[false, false]),
                    state(&[true]),
                    state(&[false, true]),
                ),
                Transition::new(state(&[true, false]), state(&[false]), state(&[true, true])),
                Transition::new(state(&[false, true]), state(&[true]), state(&[true, true])),
            ],
            Some(
                StateSet::from_states(2, [state(&[false, false]), state(&[true, false])]).unwrap(),
            ),
        )
        .unwrap()
    }

    #[test]
    fn input_to_output_table_matches_latches_by_name_and_po_order() {
        let network = sample_network();
        let table = extract_input_to_output_table(
            &network,
            &[NodeId(0), NodeId(1)],
            &[NodeId(4), NodeId(5)],
            &[NodeId(2), NodeId(3)],
        )
        .unwrap();

        assert_eq!(
            table,
            BTreeMap::from([(NodeId(0), NodeId(4)), (NodeId(1), NodeId(5))])
        );
    }

    #[test]
    fn state_var_extraction_follows_pi_ordering_and_skips_non_state_inputs() {
        let network = sample_network();
        let table = BTreeMap::from([(NodeId(1), NodeId(5))]);
        let ordering = pi_ordering_from_slice(&[NodeId(0), NodeId(1), NodeId(4)]).unwrap();

        assert_eq!(
            extract_state_input_vars(&network, &ordering, &table).unwrap(),
            vec![VarId(1)]
        );
        assert_eq!(
            extract_state_output_vars(&network, &ordering, &table).unwrap(),
            vec![VarId(5)]
        );
    }

    #[test]
    fn forward_image_smooths_external_inputs_and_renames_next_state_to_present_state() {
        let data = relation();
        let current = StateSet::from_states(2, [state(&[false, false])]).unwrap();

        let next = consistency_compute_next_states(&current, &data).unwrap();

        assert_eq!(
            next,
            StateSet::from_states(2, [state(&[true, false]), state(&[false, true])]).unwrap()
        );
    }

    #[test]
    fn reverse_image_finds_all_predecessors_for_next_set() {
        let data = relation();
        let next = StateSet::from_states(2, [state(&[true, true])]).unwrap();

        let current = consistency_compute_reverse_image(&next, &data).unwrap();

        assert_eq!(
            current,
            StateSet::from_states(2, [state(&[true, false]), state(&[false, true])]).unwrap()
        );
    }

    #[test]
    fn output_check_returns_failing_current_states() {
        let data = relation();
        let current =
            StateSet::from_states(2, [state(&[false, false]), state(&[false, true])]).unwrap();

        assert_eq!(
            consistency_check_output(&current, &data).unwrap(),
            Some(StateSet::from_states(2, [state(&[false, true])]).unwrap())
        );

        let valid = StateSet::from_states(2, [state(&[false, false])]).unwrap();
        assert_eq!(consistency_check_output(&valid, &data).unwrap(), None);
    }

    #[test]
    fn bdd_sizes_report_transition_and_output_set_sizes() {
        assert_eq!(
            consistency_bdd_sizes(&relation()),
            ConsistencyBddSizes {
                consistency_fn_size: 4,
                output_fn_size: 2,
            }
        );

        let no_output = ConsistencyRangeData::new(1, 0, [], None).unwrap();
        assert_eq!(
            consistency_bdd_sizes(&no_output),
            ConsistencyBddSizes {
                consistency_fn_size: 0,
                output_fn_size: 0,
            }
        );
    }

    #[test]
    fn width_errors_are_explicit() {
        let error = StateSet::from_states(2, [state(&[true])]).unwrap_err();
        assert_eq!(
            error,
            ConsistencyError::StateVectorWidthMismatch {
                expected: 2,
                actual: 1,
            }
        );

        let error = ConsistencyRangeData::new(
            2,
            1,
            [Transition::new(
                state(&[false]),
                state(&[true]),
                state(&[false, true]),
            )],
            None,
        )
        .unwrap_err();
        assert_eq!(
            error,
            ConsistencyError::TransitionWidthMismatch {
                field: "current_state",
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn sis_bound_entry_points_report_dependency_beads_and_sources() {
        let error = consistency_alloc_range_data().unwrap_err();
        let ConsistencyError::MissingSisDependencies {
            operation,
            dependencies,
        } = error
        else {
            panic!("unexpected error kind");
        };

        assert_eq!(operation, "consistency_alloc_range_data");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.75"
                && dependency.source_file == "LogicSynthesis/sis/bdd_ucb/and_smooth.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.442"
                && dependency.source_file == "LogicSynthesis/sis/seqbdd/verif_util.c"
        }));
        assert_eq!(
            required_consistency_dependencies(),
            REQUIRED_CONSISTENCY_DEPENDENCIES
        );
    }
}
