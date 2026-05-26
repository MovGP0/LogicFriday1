//! Native Rust model for `LogicSynthesis/sis/seqbdd/prl_seqinfo.c`.
//!
//! The C file owns the lifecycle of `seq_info_t`: attach a don't-care network,
//! let method-specific ordering fill BDD variables and output order, build BDDs
//! for outputs and don't-cares, compute the initial-state cube, free resources,
//! and run the reachability fixed point. This Rust port keeps those deterministic
//! pieces in owned data. Calls that still need SIS network pointers, ntbdd, or a
//! real BDD manager return generic missing-port diagnostics.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchInitialValue {
    Zero,
    One,
    DontCare,
    Unspecified,
}

impl LatchInitialValue {
    pub fn from_sis_value(value: i32) -> Result<Self, PrlSeqInfoError> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::DontCare),
            3 => Ok(Self::Unspecified),
            other => Err(PrlSeqInfoError::InvalidLatchInitialValue(other)),
        }
    }

    fn literal_value(self) -> Option<bool> {
        match self {
            Self::Zero | Self::Unspecified => Some(false),
            Self::One => Some(true),
            Self::DontCare => None,
        }
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
pub struct SeqInfoSeed {
    pub output_nodes: Vec<NodeId>,
    pub real_outputs: BTreeSet<NodeId>,
    pub node_bdds: HashMap<NodeId, BddId>,
    pub dc_map: HashMap<NodeId, NodeId>,
    pub dc_primary_inputs: Vec<NodeId>,
    pub leaves: HashMap<NodeId, usize>,
    pub present_state_vars: Vec<BddId>,
    pub ext_input_vars: Vec<BddId>,
    pub input_vars: Vec<BddId>,
    pub input_nodes: Vec<NodeId>,
    pub latches: Vec<SeqLatch>,
}

impl SeqInfoSeed {
    pub fn new(output_nodes: impl Into<Vec<NodeId>>) -> Self {
        Self {
            output_nodes: output_nodes.into(),
            real_outputs: BTreeSet::new(),
            node_bdds: HashMap::new(),
            dc_map: HashMap::new(),
            dc_primary_inputs: Vec::new(),
            leaves: HashMap::new(),
            present_state_vars: Vec::new(),
            ext_input_vars: Vec::new(),
            input_vars: Vec::new(),
            input_nodes: Vec::new(),
            latches: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialStateCube {
    literals: BTreeMap<usize, bool>,
    warnings: Vec<InitStateWarning>,
}

impl InitialStateCube {
    pub fn literals(&self) -> &BTreeMap<usize, bool> {
        &self.literals
    }

    pub fn warnings(&self) -> &[InitStateWarning] {
        &self.warnings
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitStateWarning {
    pub latch_input: NodeId,
    pub message: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqInfo {
    pub output_nodes: Vec<NodeId>,
    pub ext_output_fns: Vec<BddId>,
    pub next_state_fns: Vec<BddId>,
    pub ext_output_dc: Vec<BddId>,
    pub next_state_dc: Vec<BddId>,
    pub present_state_vars: Vec<BddId>,
    pub ext_input_vars: Vec<BddId>,
    pub input_vars: Vec<BddId>,
    pub input_nodes: Vec<NodeId>,
    pub leaves: HashMap<NodeId, usize>,
    pub dc_map: HashMap<NodeId, NodeId>,
    pub init_state_fn: InitialStateCube,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputBddPartition {
    pub ext_output_fns: Vec<BddId>,
    pub next_state_fns: Vec<BddId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DcBddPartition {
    pub ext_output_dc: Vec<BddId>,
    pub next_state_dc: Vec<BddId>,
    pub leaves: HashMap<NodeId, usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReachabilityTrace {
    pub steps: Vec<ReachabilityStep>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReachabilityStep {
    pub total_onset: usize,
    pub new_onset: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrlSeqInfoError {
    MissingNativePorts {
        operation: &'static str,
    },
    MissingNodeBdd {
        node: NodeId,
    },
    MissingDcInputMapping {
        dc_input: NodeId,
    },
    MissingLeafVariable {
        node: NodeId,
    },
    InvalidLatchInitialValue(i32),
    InconsistentInitLiteral {
        variable: usize,
        previous: bool,
        new_value: bool,
    },
    ConsistencyViolation {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for PrlSeqInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by missing native SIS ports")
            }
            Self::MissingNodeBdd { node } => {
                write!(f, "missing native BDD for seqbdd node {:?}", node)
            }
            Self::MissingDcInputMapping { dc_input } => write!(
                f,
                "don't-care primary input {:?} is absent from the dc_map",
                dc_input
            ),
            Self::MissingLeafVariable { node } => {
                write!(f, "seqbdd leaf variable is missing for node {:?}", node)
            }
            Self::InvalidLatchInitialValue(value) => {
                write!(f, "invalid SIS latch initial value {value}")
            }
            Self::InconsistentInitLiteral {
                variable,
                previous,
                new_value,
            } => write!(
                f,
                "initial-state cube assigns variable {variable} to both {previous} and {new_value}"
            ),
            Self::ConsistencyViolation {
                field,
                expected,
                actual,
            } => write!(
                f,
                "seq_info consistency check failed for {field}: expected {expected}, got {actual}"
            ),
        }
    }
}

impl Error for PrlSeqInfoError {}

pub fn init_seq_info_from_seed(seed: SeqInfoSeed) -> Result<SeqInfo, PrlSeqInfoError> {
    let output_partition =
        build_output_bdds(&seed.output_nodes, &seed.real_outputs, &seed.node_bdds)?;
    let dc_partition = build_dc_bdds(
        &seed.output_nodes,
        &seed.real_outputs,
        &seed.node_bdds,
        &seed.dc_map,
        &seed.dc_primary_inputs,
        &seed.leaves,
    )?;
    let init_state_fn = compute_initial_state(&seed.latches, &seed.leaves)?;

    let seq_info = SeqInfo {
        output_nodes: seed.output_nodes,
        ext_output_fns: output_partition.ext_output_fns,
        next_state_fns: output_partition.next_state_fns,
        ext_output_dc: dc_partition.ext_output_dc,
        next_state_dc: dc_partition.next_state_dc,
        present_state_vars: seed.present_state_vars,
        ext_input_vars: seed.ext_input_vars,
        input_vars: seed.input_vars,
        input_nodes: seed.input_nodes,
        leaves: dc_partition.leaves,
        dc_map: seed.dc_map,
        init_state_fn,
    };

    validate_seq_info(&seq_info, seed.latches.len())?;
    Ok(seq_info)
}

pub fn build_output_bdds(
    output_nodes: &[NodeId],
    real_outputs: &BTreeSet<NodeId>,
    node_bdds: &HashMap<NodeId, BddId>,
) -> Result<OutputBddPartition, PrlSeqInfoError> {
    let mut external_output_fns = Vec::new();
    let mut next_state_fns = Vec::new();

    for output in output_nodes {
        let function = *node_bdds
            .get(output)
            .ok_or(PrlSeqInfoError::MissingNodeBdd { node: *output })?;
        if real_outputs.contains(output) {
            external_output_fns.push(function);
        } else {
            next_state_fns.push(function);
        }
    }

    Ok(OutputBddPartition {
        ext_output_fns: external_output_fns,
        next_state_fns,
    })
}

pub fn build_dc_bdds(
    output_nodes: &[NodeId],
    real_outputs: &BTreeSet<NodeId>,
    node_bdds: &HashMap<NodeId, BddId>,
    dc_map: &HashMap<NodeId, NodeId>,
    dc_primary_inputs: &[NodeId],
    leaves: &HashMap<NodeId, usize>,
) -> Result<DcBddPartition, PrlSeqInfoError> {
    let mut leaves = register_dc_pis_as_bdd_inputs(dc_primary_inputs, dc_map, leaves)?;
    let mut external_output_dc = Vec::new();
    let mut next_state_dc = Vec::new();

    for output in output_nodes {
        let dc_function = match dc_map.get(output) {
            Some(dc_output) => *node_bdds
                .get(dc_output)
                .ok_or(PrlSeqInfoError::MissingNodeBdd { node: *dc_output })?,
            None => BddId(0),
        };
        if real_outputs.contains(output) {
            external_output_dc.push(dc_function);
        } else {
            next_state_dc.push(dc_function);
        }
    }

    Ok(DcBddPartition {
        ext_output_dc: external_output_dc,
        next_state_dc,
        leaves: std::mem::take(&mut leaves),
    })
}

pub fn register_dc_pis_as_bdd_inputs(
    dc_primary_inputs: &[NodeId],
    dc_map: &HashMap<NodeId, NodeId>,
    leaves: &HashMap<NodeId, usize>,
) -> Result<HashMap<NodeId, usize>, PrlSeqInfoError> {
    let mut result = leaves.clone();
    for dc_input in dc_primary_inputs {
        let input = *dc_map
            .get(dc_input)
            .ok_or(PrlSeqInfoError::MissingDcInputMapping {
                dc_input: *dc_input,
            })?;
        let var_index = *leaves
            .get(&input)
            .ok_or(PrlSeqInfoError::MissingLeafVariable { node: input })?;
        result.insert(*dc_input, var_index);
    }
    Ok(result)
}

pub fn compute_initial_state(
    latches: &[SeqLatch],
    leaves: &HashMap<NodeId, usize>,
) -> Result<InitialStateCube, PrlSeqInfoError> {
    let mut literals = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut warning_done = false;

    for latch in latches {
        let Some(value) = latch.initial_value.literal_value() else {
            continue;
        };
        if latch.initial_value == LatchInitialValue::Unspecified && !warning_done {
            warning_done = true;
            warnings.push(InitStateWarning {
                latch_input: latch.input,
                message: "unspecified init value set to 0",
            });
        }

        let variable = *leaves
            .get(&latch.output)
            .ok_or(PrlSeqInfoError::MissingLeafVariable { node: latch.output })?;
        if let Some(previous) = literals.insert(variable, value) {
            if previous != value {
                return Err(PrlSeqInfoError::InconsistentInitLiteral {
                    variable,
                    previous,
                    new_value: value,
                });
            }
        }
    }

    Ok(InitialStateCube { literals, warnings })
}

pub fn extract_reachable_states<State, ComputeNext>(
    init_state: BTreeSet<State>,
    mut compute_next_states: ComputeNext,
    verbose: bool,
) -> (BTreeSet<State>, ReachabilityTrace)
where
    State: Clone + Ord,
    ComputeNext: FnMut(&BTreeSet<State>) -> BTreeSet<State>,
{
    let mut current_set = init_state.clone();
    let mut total_set = init_state;
    let mut trace = ReachabilityTrace { steps: Vec::new() };

    loop {
        let new_current_set = compute_next_states(&current_set);
        if verbose {
            trace.steps.push(ReachabilityStep {
                total_onset: total_set.len(),
                new_onset: new_current_set.difference(&total_set).count(),
            });
        }
        if new_current_set.is_subset(&total_set) {
            break;
        }

        current_set = new_current_set.difference(&total_set).cloned().collect();
        total_set.extend(new_current_set);
    }

    (total_set, trace)
}

pub fn init_seq_info_from_sis_network<Network>(
    _network: &Network,
) -> Result<SeqInfo, PrlSeqInfoError> {
    Err(PrlSeqInfoError::MissingNativePorts {
        operation: "init_seq_info_from_sis_network",
    })
}

pub fn free_sis_seq_info<Network>(_seq_info: &mut Network) -> Result<(), PrlSeqInfoError> {
    Err(PrlSeqInfoError::MissingNativePorts {
        operation: "free_sis_seq_info",
    })
}

pub fn extract_reachable_states_from_sis_seq_info<SeqInfoRef>(
    _seq_info: &SeqInfoRef,
) -> Result<(), PrlSeqInfoError> {
    Err(PrlSeqInfoError::MissingNativePorts {
        operation: "extract_reachable_states_from_sis_seq_info",
    })
}

fn validate_seq_info(seq_info: &SeqInfo, latch_count: usize) -> Result<(), PrlSeqInfoError> {
    expect_len("next_state_fns", latch_count, seq_info.next_state_fns.len())?;
    expect_len(
        "present_state_vars",
        seq_info.next_state_fns.len(),
        seq_info.present_state_vars.len(),
    )?;
    expect_len(
        "output_nodes",
        seq_info.next_state_fns.len() + seq_info.ext_output_fns.len(),
        seq_info.output_nodes.len(),
    )?;
    expect_len(
        "input_vars",
        seq_info.input_nodes.len(),
        seq_info.input_vars.len(),
    )?;
    Ok(())
}

fn expect_len(field: &'static str, expected: usize, actual: usize) -> Result<(), PrlSeqInfoError> {
    if expected == actual {
        Ok(())
    } else {
        Err(PrlSeqInfoError::ConsistencyViolation {
            field,
            expected,
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize) -> NodeId {
        NodeId(id)
    }

    fn bdd(id: usize) -> BddId {
        BddId(id)
    }

    #[test]
    fn output_bdds_are_split_in_network_output_order() {
        let partition = build_output_bdds(
            &[node(10), node(11), node(12)],
            &BTreeSet::from([node(10), node(12)]),
            &HashMap::from([(node(10), bdd(1)), (node(11), bdd(2)), (node(12), bdd(3))]),
        )
        .unwrap();

        assert_eq!(partition.ext_output_fns, vec![bdd(1), bdd(3)]);
        assert_eq!(partition.next_state_fns, vec![bdd(2)]);
    }

    #[test]
    fn dc_bdds_register_dc_inputs_and_use_zero_when_no_dc_output_exists() {
        let partition = build_dc_bdds(
            &[node(10), node(11), node(12)],
            &BTreeSet::from([node(10), node(12)]),
            &HashMap::from([(node(100), bdd(7)), (node(102), bdd(9))]),
            &HashMap::from([
                (node(10), node(100)),
                (node(12), node(102)),
                (node(200), node(20)),
            ]),
            &[node(200)],
            &HashMap::from([(node(20), 5)]),
        )
        .unwrap();

        assert_eq!(partition.ext_output_dc, vec![bdd(7), bdd(9)]);
        assert_eq!(partition.next_state_dc, vec![bdd(0)]);
        assert_eq!(partition.leaves[&node(200)], 5);
    }

    #[test]
    fn initial_state_cube_skips_dc_and_warns_once_for_unspecified_values() {
        let cube = compute_initial_state(
            &[
                SeqLatch::new(node(30), node(40), LatchInitialValue::One),
                SeqLatch::new(node(31), node(41), LatchInitialValue::DontCare),
                SeqLatch::new(node(32), node(42), LatchInitialValue::Unspecified),
                SeqLatch::new(node(33), node(43), LatchInitialValue::Unspecified),
            ],
            &HashMap::from([(node(40), 0), (node(41), 1), (node(42), 2), (node(43), 3)]),
        )
        .unwrap();

        assert_eq!(
            cube.literals(),
            &BTreeMap::from([(0, true), (2, false), (3, false)])
        );
        assert_eq!(
            cube.warnings(),
            &[InitStateWarning {
                latch_input: node(32),
                message: "unspecified init value set to 0",
            }]
        );
    }

    #[test]
    fn init_seq_info_validates_c_consistency_invariants() {
        let mut seed = SeqInfoSeed::new([node(10), node(11)]);
        seed.real_outputs.insert(node(10));
        seed.node_bdds = HashMap::from([(node(10), bdd(1)), (node(11), bdd(2))]);
        seed.present_state_vars = vec![bdd(3)];
        seed.input_vars = vec![bdd(4), bdd(5)];
        seed.input_nodes = vec![node(20), node(21)];
        seed.latches = vec![SeqLatch::new(node(30), node(40), LatchInitialValue::Zero)];
        seed.leaves = HashMap::from([(node(40), 0)]);

        let seq_info = init_seq_info_from_seed(seed).unwrap();

        assert_eq!(seq_info.ext_output_fns, vec![bdd(1)]);
        assert_eq!(seq_info.next_state_fns, vec![bdd(2)]);
        assert_eq!(
            seq_info.init_state_fn.literals(),
            &BTreeMap::from([(0, false)])
        );
    }

    #[test]
    fn reachability_matches_c_fixed_point_update() {
        let transitions = HashMap::from([
            (0, BTreeSet::from([1])),
            (1, BTreeSet::from([2])),
            (2, BTreeSet::from([1, 3])),
            (3, BTreeSet::from([3])),
        ]);

        let (reachable, trace) = extract_reachable_states(
            BTreeSet::from([0]),
            |current| {
                current
                    .iter()
                    .flat_map(|state| transitions.get(state).into_iter().flatten().copied())
                    .collect()
            },
            true,
        );

        assert_eq!(reachable, BTreeSet::from([0, 1, 2, 3]));
        assert_eq!(
            trace.steps,
            vec![
                ReachabilityStep {
                    total_onset: 1,
                    new_onset: 1,
                },
                ReachabilityStep {
                    total_onset: 2,
                    new_onset: 1,
                },
                ReachabilityStep {
                    total_onset: 3,
                    new_onset: 1,
                },
                ReachabilityStep {
                    total_onset: 4,
                    new_onset: 0,
                },
            ]
        );
    }
    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("prl_seqinfo.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
