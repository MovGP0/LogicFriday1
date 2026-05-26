//! Native Rust setup and validation model for `sis/stg/senum_main.c`.
//!
//! The C file is the coordinator for STG extraction. It validates the clocking
//! topology, removes clock/control logic, records latch and primary I/O counts,
//! initializes packed-state bookkeeping, then delegates to levelization,
//! simulation, enumeration, and legacy `graph_t` STG construction. This module
//! ports the independent setup/validation behavior to explicit Rust data
//! structures. The full network-to-STG extraction remains blocked until the
//! native SIS network, node, latch, clock, and graph ports are available.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead: &'static str,
    pub c_file: &'static str,
    pub reason: &'static str,
}

pub const BLOCKED_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.487",
        c_file: "LogicSynthesis/sis/stg/enumerate.c",
        reason: "performs recursive state enumeration and packed-state queueing",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.488",
        c_file: "LogicSynthesis/sis/stg/level_c.c",
        reason: "builds ndata records, latch endpoint arrays, levels, and varying-node order",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.490",
        c_file: "LogicSynthesis/sis/stg/stg_sc_sim.c",
        reason: "simulates one combinational step during enumeration",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.492",
        c_file: "LogicSynthesis/sis/stg/stg.c",
        reason: "allocates and mutates the target STG graph",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.110",
        c_file: "LogicSynthesis/sis/clock/clock.c",
        reason: "provides clock lookup, transitive clock relation, and edge timing data",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.230",
        c_file: "LogicSynthesis/sis/latch/latch.c",
        reason: "provides latch control, type, and initial-value APIs",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.305",
        c_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "provides network checks, duplication, primary I/O counts, and traversal",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.307",
        c_file: "LogicSynthesis/sis/network/sweep.c",
        reason: "removes dangling and parallel latch structure before extraction",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.313",
        c_file: "LogicSynthesis/sis/node/fan.c",
        reason: "provides transitive fanin/fanout traversal for clock validation and removal",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.318",
        c_file: "LogicSynthesis/sis/node/node.c",
        reason: "provides node kind/name storage and deletion behavior",
    },
];

pub fn blocked_dependencies() -> &'static [PortDependency] {
    BLOCKED_DEPENDENCIES
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtractionMode {
    InitialStateOnly,
    CompleteStateTable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub name: String,
    pub kind: NodeKind,
    pub is_control: bool,
    pub transitive_fanin: Vec<usize>,
    pub transitive_fanout: Vec<usize>,
    pub clock_relation: Option<ClockRelation>,
    pub deleted: bool,
}

impl NetworkNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            is_control: false,
            transitive_fanin: Vec::new(),
            transitive_fanout: Vec::new(),
            clock_relation: None,
            deleted: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchSynchType {
    Unknown,
    RisingEdge,
    FallingEdge,
    ActiveHigh,
    ActiveLow,
}

impl LatchSynchType {
    pub const fn is_edge_or_unknown(self) -> bool {
        matches!(self, Self::Unknown | Self::RisingEdge | Self::FallingEdge)
    }

    pub const fn edge_index(self) -> i32 {
        match self {
            Self::RisingEdge => 0,
            Self::FallingEdge => 1,
            _ => -1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SenumLatch {
    pub control: Option<usize>,
    pub synch_type: LatchSynchType,
    pub initial_value: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase {
    PositiveUnate,
    NegativeUnate,
    Binate,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockRelation {
    pub clock_name: String,
    pub phase: InputPhase,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockEdgeTiming {
    pub nominal: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SisClock {
    pub name: String,
    pub cycle_time: f64,
    pub rise: ClockEdgeTiming,
    pub fall: ClockEdgeTiming,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SenumClockData {
    pub name: String,
    pub cycle_time: f64,
    pub nominal_rise: f64,
    pub min_rise: f64,
    pub max_rise: f64,
    pub nominal_fall: f64,
    pub min_fall: f64,
    pub max_fall: f64,
}

impl From<&SisClock> for SenumClockData {
    fn from(clock: &SisClock) -> Self {
        Self {
            name: clock.name.clone(),
            cycle_time: clock.cycle_time,
            nominal_rise: clock.rise.nominal,
            min_rise: clock.rise.min,
            max_rise: clock.rise.max,
            nominal_fall: clock.fall.nominal,
            min_fall: clock.fall.min,
            max_fall: clock.fall.max,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SenumNetwork {
    pub nodes: Vec<NetworkNode>,
    pub latches: Vec<SenumLatch>,
    pub clocks: Vec<SisClock>,
    pub primary_input_count: usize,
    pub primary_output_count: usize,
    pub internal_count: usize,
}

impl SenumNetwork {
    pub fn new(
        nodes: Vec<NetworkNode>,
        latches: Vec<SenumLatch>,
        clocks: Vec<SisClock>,
        primary_input_count: usize,
        primary_output_count: usize,
        internal_count: usize,
    ) -> Self {
        Self {
            nodes,
            latches,
            clocks,
            primary_input_count,
            primary_output_count,
            internal_count,
        }
    }

    pub fn find_node_by_name(&self, name: &str) -> Option<usize> {
        self.nodes
            .iter()
            .position(|node| !node.deleted && node.name == name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidNetwork {
    pub control_node: Option<usize>,
    pub clock_name: Option<String>,
    pub edge_index: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SenumSetup {
    pub validation: ValidNetwork,
    pub latch_count: usize,
    pub primary_input_count: usize,
    pub primary_output_count: usize,
    pub bits_per_word: usize,
    pub words_per_state: usize,
    pub varying_node_capacity: usize,
    pub initial_state: Option<String>,
    pub total_no_of_states: usize,
    pub total_no_of_edges: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlRemoval {
    pub cleared_latches: usize,
    pub deleted_nodes: Vec<usize>,
}

pub fn validate_network(network: &SenumNetwork) -> Result<ValidNetwork, SenumError> {
    if network.clocks.len() > 1 {
        return Err(SenumError::MultipleClocks);
    }

    let clock_names: HashSet<&str> = network
        .clocks
        .iter()
        .map(|clock| clock.name.as_str())
        .collect();
    let mut control = None;
    let mut latch_type = LatchSynchType::Unknown;

    for (latch_index, latch) in network.latches.iter().enumerate() {
        if let Some(new_control) = latch.control {
            require_node(network, new_control)?;
            match control {
                None => control = Some(new_control),
                Some(existing) if existing != new_control => {
                    return Err(SenumError::DifferentLatchControls {
                        first: existing,
                        second: new_control,
                    });
                }
                Some(_) => {}
            }
        }

        let new_type = latch.synch_type;
        if !new_type.is_edge_or_unknown() {
            return Err(SenumError::NonEdgeTriggeredLatch {
                latch: latch_index,
                synch_type: new_type,
            });
        }
        if latch_type != LatchSynchType::Unknown && latch_type != new_type {
            return Err(SenumError::DifferentLatchTypes {
                first: latch_type,
                second: new_type,
            });
        }
        if matches!(
            new_type,
            LatchSynchType::RisingEdge | LatchSynchType::FallingEdge
        ) {
            latch_type = new_type;
        }
    }

    let mut clock_name = None;
    if let Some(control_node) = control {
        let control_data = require_node(network, control_node)?;
        for &fanin in &control_data.transitive_fanin {
            let fanin_data = require_node(network, fanin)?;
            if fanin_data.kind == NodeKind::PrimaryInput
                && !clock_names.contains(fanin_data.name.as_str())
            {
                return Err(SenumError::GatedClock { node: fanin });
            }
        }

        let relation =
            control_data
                .clock_relation
                .as_ref()
                .ok_or(SenumError::MissingTransitiveClock {
                    control: control_node,
                })?;
        clock_name = Some(relation.clock_name.clone());
        latch_type = apply_clock_phase(latch_type, relation.phase)?;
    } else if let Some(clock) = network.clocks.first() {
        let clock_node = network
            .find_node_by_name(&clock.name)
            .ok_or_else(|| SenumError::ClockNodeNotFound(clock.name.clone()))?;
        for &fanout in &network.nodes[clock_node].transitive_fanout {
            let fanout_data = require_node(network, fanout)?;
            if fanout_data.kind == NodeKind::PrimaryOutput && !fanout_data.is_control {
                return Err(SenumError::ClockPathToPoOrLatchInput { node: fanout });
            }
        }
    }

    Ok(ValidNetwork {
        control_node: control,
        clock_name,
        edge_index: if control.is_some() {
            latch_type.edge_index()
        } else {
            -1
        },
    })
}

pub fn prepare_extraction(
    network: &SenumNetwork,
    mode: ExtractionMode,
) -> Result<SenumSetup, SenumError> {
    let validation = validate_network(network)?;
    let latch_count = network.latches.len();
    if network.primary_input_count < latch_count || network.primary_output_count < latch_count {
        return Err(SenumError::InvalidNetworkCounts {
            primary_inputs: network.primary_input_count,
            primary_outputs: network.primary_output_count,
            latches: latch_count,
        });
    }

    let initial_state = match mode {
        ExtractionMode::InitialStateOnly => {
            validate_single_initial_state(&network.latches)?;
            None
        }
        ExtractionMode::CompleteStateTable => Some(initial_state_encoding(&network.latches)?),
    };
    let bits_per_word = usize::BITS as usize;
    let words_per_state = if latch_count == 0 {
        1
    } else {
        latch_count.div_ceil(bits_per_word)
    };
    let primary_input_count = network.primary_input_count - latch_count;
    let primary_output_count = network.primary_output_count - latch_count;

    Ok(SenumSetup {
        validation,
        latch_count,
        primary_input_count,
        primary_output_count,
        bits_per_word,
        words_per_state,
        varying_node_capacity: network.internal_count + primary_input_count,
        initial_state,
        total_no_of_states: 0,
        total_no_of_edges: 0,
    })
}

pub fn stg_extract(network: &SenumNetwork, mode: ExtractionMode) -> Result<SenumSetup, SenumError> {
    let _ = prepare_extraction(network, mode)?;
    Err(SenumError::MissingExtractionDependencies {
        dependencies: blocked_dependencies(),
    })
}

pub fn remove_control_logic(network: &mut SenumNetwork) -> ControlRemoval {
    let mut cleared_latches = 0;
    for latch in &mut network.latches {
        if latch.control.take().is_some() {
            cleared_latches += 1;
        }
    }

    let Some(clock) = network.clocks.first() else {
        return ControlRemoval {
            cleared_latches,
            deleted_nodes: Vec::new(),
        };
    };
    let Some(control_node) = network.find_node_by_name(&clock.name) else {
        return ControlRemoval {
            cleared_latches,
            deleted_nodes: Vec::new(),
        };
    };

    let mut deleted_nodes = network.nodes[control_node].transitive_fanout.clone();
    deleted_nodes.push(control_node);
    for &node in &deleted_nodes {
        if let Some(data) = network.nodes.get_mut(node) {
            data.deleted = true;
        }
    }

    ControlRemoval {
        cleared_latches,
        deleted_nodes,
    }
}

pub fn clock_info(network: &SenumNetwork) -> Option<SenumClockData> {
    (network.clocks.len() == 1).then(|| SenumClockData::from(&network.clocks[0]))
}

pub fn validate_single_initial_state(latches: &[SenumLatch]) -> Result<(), SenumError> {
    for (latch, data) in latches.iter().enumerate() {
        if data.initial_value != 0 && data.initial_value != 1 {
            return Err(SenumError::NonBinaryInitialState {
                latch,
                value: data.initial_value,
            });
        }
    }
    Ok(())
}

pub fn initial_state_encoding(latches: &[SenumLatch]) -> Result<String, SenumError> {
    let mut encoded = String::with_capacity(latches.len());
    for (latch, data) in latches.iter().enumerate() {
        let digit = match data.initial_value {
            0 => '0',
            1 => '1',
            2 => '2',
            3 => '3',
            value => {
                return Err(SenumError::UnknownLatchInitialValue { latch, value });
            }
        };
        encoded.push(digit);
    }
    Ok(encoded)
}

fn apply_clock_phase(
    mut latch_type: LatchSynchType,
    phase: InputPhase,
) -> Result<LatchSynchType, SenumError> {
    match phase {
        InputPhase::NegativeUnate => {
            latch_type = if latch_type == LatchSynchType::RisingEdge {
                LatchSynchType::FallingEdge
            } else {
                LatchSynchType::RisingEdge
            };
        }
        InputPhase::Binate => return Err(SenumError::BinateClockControlPhase),
        InputPhase::PositiveUnate | InputPhase::Unknown => {}
    }
    Ok(latch_type)
}

fn require_node(network: &SenumNetwork, node: usize) -> Result<&NetworkNode, SenumError> {
    network
        .nodes
        .get(node)
        .filter(|node| !node.deleted)
        .ok_or(SenumError::UnknownNode { node })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SenumError {
    MultipleClocks,
    UnknownNode {
        node: usize,
    },
    DifferentLatchControls {
        first: usize,
        second: usize,
    },
    NonEdgeTriggeredLatch {
        latch: usize,
        synch_type: LatchSynchType,
    },
    DifferentLatchTypes {
        first: LatchSynchType,
        second: LatchSynchType,
    },
    GatedClock {
        node: usize,
    },
    MissingTransitiveClock {
        control: usize,
    },
    BinateClockControlPhase,
    ClockNodeNotFound(String),
    ClockPathToPoOrLatchInput {
        node: usize,
    },
    NonBinaryInitialState {
        latch: usize,
        value: i32,
    },
    UnknownLatchInitialValue {
        latch: usize,
        value: i32,
    },
    InvalidNetworkCounts {
        primary_inputs: usize,
        primary_outputs: usize,
        latches: usize,
    },
    MissingExtractionDependencies {
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for SenumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MultipleClocks => write!(f, "multiple clocks found in the design"),
            Self::UnknownNode { node } => write!(f, "unknown network node {node}"),
            Self::DifferentLatchControls { first, second } => {
                write!(f, "different signals control latches: {first} and {second}")
            }
            Self::NonEdgeTriggeredLatch { latch, synch_type } => write!(
                f,
                "latch {latch} has unsupported non-edge synchronization type {synch_type:?}"
            ),
            Self::DifferentLatchTypes { first, second } => {
                write!(
                    f,
                    "latches of different types are present: {first:?} and {second:?}"
                )
            }
            Self::GatedClock { node } => write!(f, "gated clock input found at node {node}"),
            Self::MissingTransitiveClock { control } => write!(
                f,
                "control node {control} has no native transitive clock relation"
            ),
            Self::BinateClockControlPhase => {
                write!(
                    f,
                    "phase relationship between clock and control is not unique"
                )
            }
            Self::ClockNodeNotFound(name) => write!(f, "clock node {name} was not found"),
            Self::ClockPathToPoOrLatchInput { node } => {
                write!(
                    f,
                    "path from clock to latch input or primary output reaches node {node}"
                )
            }
            Self::NonBinaryInitialState { latch, value } => write!(
                f,
                "latch {latch} has initial value {value}; expected 0 or 1"
            ),
            Self::UnknownLatchInitialValue { latch, value } => {
                write!(f, "latch {latch} has unknown initial value {value}")
            }
            Self::InvalidNetworkCounts {
                primary_inputs,
                primary_outputs,
                latches,
            } => write!(
                f,
                "network counts are inconsistent: {primary_inputs} PIs, {primary_outputs} POs, {latches} latches"
            ),
            Self::MissingExtractionDependencies { dependencies } => write!(
                f,
                "STG extraction is blocked by {} unported SIS dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for SenumError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(name: &str) -> SisClock {
        SisClock {
            name: name.to_owned(),
            cycle_time: 10.0,
            rise: ClockEdgeTiming {
                nominal: 1.0,
                min: 0.5,
                max: 1.5,
            },
            fall: ClockEdgeTiming {
                nominal: 6.0,
                min: 5.5,
                max: 6.5,
            },
        }
    }

    fn latch(control: Option<usize>, synch_type: LatchSynchType, initial_value: i32) -> SenumLatch {
        SenumLatch {
            control,
            synch_type,
            initial_value,
        }
    }

    #[test]
    fn validation_rejects_multiple_clocks_and_non_edge_latches() {
        let network = SenumNetwork::new(
            vec![],
            vec![],
            vec![clock("clk_a"), clock("clk_b")],
            0,
            0,
            0,
        );
        assert_eq!(validate_network(&network), Err(SenumError::MultipleClocks));

        let network = SenumNetwork::new(
            vec![NetworkNode::new("ctl", NodeKind::PrimaryInput)],
            vec![latch(Some(0), LatchSynchType::ActiveHigh, 0)],
            vec![],
            1,
            1,
            0,
        );
        assert_eq!(
            validate_network(&network),
            Err(SenumError::NonEdgeTriggeredLatch {
                latch: 0,
                synch_type: LatchSynchType::ActiveHigh
            })
        );
    }

    #[test]
    fn validation_rejects_mixed_controls_and_latch_types() {
        let network = SenumNetwork::new(
            vec![
                NetworkNode::new("ctl_a", NodeKind::PrimaryInput),
                NetworkNode::new("ctl_b", NodeKind::PrimaryInput),
            ],
            vec![
                latch(Some(0), LatchSynchType::RisingEdge, 0),
                latch(Some(1), LatchSynchType::RisingEdge, 1),
            ],
            vec![],
            2,
            2,
            0,
        );
        assert_eq!(
            validate_network(&network),
            Err(SenumError::DifferentLatchControls {
                first: 0,
                second: 1
            })
        );

        let network = SenumNetwork::new(
            vec![],
            vec![
                latch(None, LatchSynchType::RisingEdge, 0),
                latch(None, LatchSynchType::FallingEdge, 1),
            ],
            vec![],
            2,
            2,
            0,
        );
        assert_eq!(
            validate_network(&network),
            Err(SenumError::DifferentLatchTypes {
                first: LatchSynchType::RisingEdge,
                second: LatchSynchType::FallingEdge
            })
        );
    }

    #[test]
    fn validation_reports_gated_clock_inputs() {
        let mut control = NetworkNode::new("ctl", NodeKind::Internal);
        control.transitive_fanin = vec![1];
        control.clock_relation = Some(ClockRelation {
            clock_name: "clk".to_owned(),
            phase: InputPhase::PositiveUnate,
        });
        let data_input = NetworkNode::new("data", NodeKind::PrimaryInput);
        let network = SenumNetwork::new(
            vec![control, data_input],
            vec![latch(Some(0), LatchSynchType::RisingEdge, 0)],
            vec![clock("clk")],
            2,
            1,
            1,
        );

        assert_eq!(
            validate_network(&network),
            Err(SenumError::GatedClock { node: 1 })
        );
    }

    #[test]
    fn validation_sets_clock_name_and_flips_negative_phase_edge() {
        let mut control = NetworkNode::new("ctl", NodeKind::Internal);
        control.clock_relation = Some(ClockRelation {
            clock_name: "clk".to_owned(),
            phase: InputPhase::NegativeUnate,
        });
        let network = SenumNetwork::new(
            vec![control],
            vec![latch(Some(0), LatchSynchType::RisingEdge, 0)],
            vec![clock("clk")],
            1,
            1,
            1,
        );

        assert_eq!(
            validate_network(&network).unwrap(),
            ValidNetwork {
                control_node: Some(0),
                clock_name: Some("clk".to_owned()),
                edge_index: 1,
            }
        );
    }

    #[test]
    fn validation_rejects_clock_path_to_primary_output_without_control_latch() {
        let mut clock_node = NetworkNode::new("clk", NodeKind::PrimaryInput);
        clock_node.transitive_fanout = vec![1];
        let po = NetworkNode::new("out", NodeKind::PrimaryOutput);
        let network = SenumNetwork::new(
            vec![clock_node, po],
            vec![latch(None, LatchSynchType::Unknown, 0)],
            vec![clock("clk")],
            2,
            1,
            0,
        );

        assert_eq!(
            validate_network(&network),
            Err(SenumError::ClockPathToPoOrLatchInput { node: 1 })
        );
    }

    #[test]
    fn clock_info_copies_single_clock_timing() {
        let network = SenumNetwork::new(vec![], vec![], vec![clock("clk")], 0, 0, 0);

        assert_eq!(
            clock_info(&network),
            Some(SenumClockData {
                name: "clk".to_owned(),
                cycle_time: 10.0,
                nominal_rise: 1.0,
                min_rise: 0.5,
                max_rise: 1.5,
                nominal_fall: 6.0,
                min_fall: 5.5,
                max_fall: 6.5,
            })
        );
    }

    #[test]
    fn setup_validates_initial_state_modes_and_counts() {
        let network = SenumNetwork::new(
            vec![],
            vec![
                latch(None, LatchSynchType::Unknown, 0),
                latch(None, LatchSynchType::Unknown, 2),
            ],
            vec![],
            5,
            4,
            7,
        );

        assert_eq!(
            prepare_extraction(&network, ExtractionMode::InitialStateOnly),
            Err(SenumError::NonBinaryInitialState { latch: 1, value: 2 })
        );

        let setup = prepare_extraction(&network, ExtractionMode::CompleteStateTable).unwrap();
        assert_eq!(setup.initial_state.as_deref(), Some("02"));
        assert_eq!(setup.latch_count, 2);
        assert_eq!(setup.primary_input_count, 3);
        assert_eq!(setup.primary_output_count, 2);
        assert_eq!(setup.words_per_state, 1);
        assert_eq!(setup.varying_node_capacity, 10);
        assert_eq!(setup.total_no_of_states, 0);
        assert_eq!(setup.total_no_of_edges, 0);
    }

    #[test]
    fn remove_control_logic_clears_latches_and_deletes_clock_fanout() {
        let mut clock_node = NetworkNode::new("clk", NodeKind::PrimaryInput);
        clock_node.transitive_fanout = vec![1, 2];
        let mut network = SenumNetwork::new(
            vec![
                clock_node,
                NetworkNode::new("ctl_buf", NodeKind::Internal),
                NetworkNode::new("ctl_po", NodeKind::PrimaryOutput),
            ],
            vec![
                latch(Some(1), LatchSynchType::RisingEdge, 0),
                latch(Some(1), LatchSynchType::RisingEdge, 1),
            ],
            vec![clock("clk")],
            3,
            2,
            1,
        );

        let removed = remove_control_logic(&mut network);

        assert_eq!(
            removed,
            ControlRemoval {
                cleared_latches: 2,
                deleted_nodes: vec![1, 2, 0],
            }
        );
        assert!(network.latches.iter().all(|latch| latch.control.is_none()));
        assert!(network.nodes[0].deleted);
        assert!(network.nodes[1].deleted);
        assert!(network.nodes[2].deleted);
    }

    #[test]
    fn top_level_extraction_reports_blocked_dependencies_after_setup() {
        let network = SenumNetwork::new(vec![], vec![], vec![], 0, 0, 0);
        let error = stg_extract(&network, ExtractionMode::InitialStateOnly)
            .expect_err("full STG extraction is intentionally blocked");

        let SenumError::MissingExtractionDependencies { dependencies } = error else {
            panic!("unexpected error kind");
        };
        assert!(
            dependencies
                .iter()
                .any(|dependency| dependency.bead == "LogicFriday1-8j8.2.6.487")
        );
        assert!(
            dependencies
                .iter()
                .any(|dependency| dependency.bead == "LogicFriday1-8j8.2.6.490")
        );
        assert!(
            dependencies
                .iter()
                .any(|dependency| dependency.bead == "LogicFriday1-8j8.2.6.230")
        );
    }
}
