//! Native Rust model for `LogicSynthesis/sis/retime/re_util.c`.
//!
//! The C file is a collection of retiming graph utilities plus thin calls into
//! SIS `network_t`, `node_t`, latch, delay, and mapped-library APIs. This module
//! ports the graph behavior to owned Rust data structures. Direct SIS
//! integration remains an explicit dependency error
//! files; no legacy C ABI entry points are exposed here.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const RETIME_NOT_SET: i32 = -1;
pub const RETIME_USER_NOT_SET: f64 = -100_000.0;
pub const NEG_LARGE: f64 = -10_000.0;
pub const INFINITY_LEVEL: usize = usize::MAX;
pub const UNIT_FANOUT_BASE_DELAY: f64 = 1.0;
pub const UNIT_FANOUT_SLOPE: f64 = 0.2;

pub fn sis_network_graph_build_blocked() -> Result<(), RetimeUtilError> {
    missing_sis_dependencies("retime SIS network graph build")
}

pub fn sis_clock_data_blocked() -> Result<(), RetimeUtilError> {
    missing_sis_dependencies("retime SIS clock-data extraction")
}

fn missing_sis_dependencies(operation: &'static str) -> Result<(), RetimeUtilError> {
    Err(RetimeUtilError::MissingSisDependencies { operation })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeType {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SynchronizationType {
    RisingEdge,
    FallingEdge,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockEdgeFlag {
    BeforeClockEdge,
    AfterClockEdge,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GatePinDelay {
    pub block: DelayTime,
    pub load_slope: f64,
}

impl GatePinDelay {
    pub fn new(rise_block: f64, fall_block: f64, load_slope: f64) -> Self {
        Self {
            block: DelayTime::new(rise_block, fall_block),
            load_slope,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappedGate {
    pub area: f64,
    pub pins: Vec<GatePinDelay>,
}

impl MappedGate {
    pub fn new(area: f64, pins: Vec<GatePinDelay>) -> Self {
        Self { area, pins }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNodeSpec {
    pub name: String,
    pub node_type: RetimeNodeType,
    pub literal_count: usize,
    pub effective_fanout_count: usize,
    pub user_constraint: Option<UserTimingConstraint>,
    pub mapped_gate: Option<MappedGate>,
    pub mapped_load: f64,
}

impl RetimeNodeSpec {
    pub fn new(name: impl Into<String>, node_type: RetimeNodeType) -> Self {
        Self {
            name: name.into(),
            node_type,
            literal_count: 0,
            effective_fanout_count: 0,
            user_constraint: None,
            mapped_gate: None,
            mapped_load: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UserTimingConstraint {
    pub edge_flag: ClockEdgeFlag,
    pub arrival: Option<DelayTime>,
    pub required: Option<DelayTime>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNode {
    pub id: usize,
    pub name: String,
    pub node_type: RetimeNodeType,
    pub lp_index: Option<usize>,
    pub fanins: Vec<usize>,
    pub fanouts: Vec<usize>,
    pub scaled_delay: i32,
    pub final_area: f64,
    pub final_delay: f64,
    pub user_time: f64,
    pub scaled_user_time: i32,
}

impl RetimeNode {
    pub fn allocated(id: usize, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            node_type: RetimeNodeType::Internal,
            lp_index: None,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            scaled_delay: RETIME_NOT_SET,
            final_area: 0.0,
            final_delay: 0.0,
            user_time: RETIME_USER_NOT_SET,
            scaled_user_time: 0,
        }
    }

    pub fn is_host_vertex(&self) -> bool {
        matches!(
            self.node_type,
            RetimeNodeType::PrimaryInput | RetimeNodeType::PrimaryOutput
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeEdge {
    pub id: usize,
    pub source: usize,
    pub sink: usize,
    pub sink_fanin_id: usize,
    pub weight: i32,
    pub breadth: f64,
    pub temp_breadth: f64,
    pub latches_present: bool,
    pub initial_values: Vec<i32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RetimeGraph {
    pub nodes: Vec<RetimeNode>,
    pub edges: Vec<RetimeEdge>,
    pub primary_inputs: Vec<usize>,
    pub primary_outputs: Vec<usize>,
    pub synchronization_type: Option<SynchronizationType>,
    pub control_name: Option<String>,
}

impl RetimeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node_from_spec(
        &mut self,
        spec: RetimeNodeSpec,
        use_mapped: bool,
    ) -> Result<usize, RetimeUtilError> {
        let id = self.nodes.len();
        let mut node = RetimeNode::allocated(id, spec.name);
        node.node_type = spec.node_type;

        match spec.node_type {
            RetimeNodeType::PrimaryInput => {
                node.user_time = retime_get_user_constraint(
                    RetimeNodeType::PrimaryInput,
                    spec.user_constraint.as_ref(),
                );
                self.primary_inputs.push(id);
            }
            RetimeNodeType::PrimaryOutput => {
                node.user_time = retime_get_user_constraint(
                    RetimeNodeType::PrimaryOutput,
                    spec.user_constraint.as_ref(),
                );
                self.primary_outputs.push(id);
            }
            RetimeNodeType::Internal | RetimeNodeType::Ignore => {
                node.user_time = RETIME_USER_NOT_SET;
                if spec.literal_count == 0 {
                    node.final_area = 0.0;
                    node.final_delay = 0.0;
                } else if use_mapped {
                    let gate = spec
                        .mapped_gate
                        .ok_or(RetimeUtilError::MissingMappedGate { node_id: id })?;
                    node.final_area = gate.area;
                    node.final_delay = retime_simulate_gate(&gate, spec.mapped_load);
                } else {
                    node.final_area = spec.literal_count as f64;
                    node.final_delay = UNIT_FANOUT_BASE_DELAY
                        + UNIT_FANOUT_SLOPE * spec.effective_fanout_count as f64;
                }
            }
        }

        self.nodes.push(node);
        Ok(id)
    }

    pub fn add_edge(
        &mut self,
        source: usize,
        sink: usize,
        weight: i32,
        breadth: f64,
        sink_fanin_id: usize,
    ) -> Result<usize, RetimeUtilError> {
        self.require_node(source)?;
        self.require_node(sink)?;

        if let Some(existing) = self.nodes[source].fanouts.iter().copied().find(|edge_id| {
            let edge = &self.edges[*edge_id];
            edge.sink == sink && edge.weight == weight && edge.sink_fanin_id == sink_fanin_id
        }) {
            return Ok(existing);
        }

        let id = self.edges.len();
        self.edges.push(RetimeEdge {
            id,
            source,
            sink,
            sink_fanin_id,
            weight,
            breadth,
            temp_breadth: breadth,
            latches_present: false,
            initial_values: Vec::new(),
        });
        self.nodes[source].fanouts.push(id);
        self.nodes[sink].fanins.push(id);
        Ok(id)
    }

    pub fn dfs_from_outputs(&self) -> Result<Vec<usize>, RetimeUtilError> {
        self.dfs_from_roots(&self.primary_outputs, true)
    }

    pub fn dfs_from_inputs(&self) -> Result<Vec<usize>, RetimeUtilError> {
        self.dfs_from_roots(&self.primary_inputs, false)
    }

    pub fn retime_node(
        &mut self,
        node_id: usize,
        retime_amount: i32,
    ) -> Result<(), RetimeUtilError> {
        self.require_internal_node(node_id)?;
        let fanins = self.nodes[node_id].fanins.clone();
        let fanouts = self.nodes[node_id].fanouts.clone();
        for edge_id in fanins {
            if !self.ignore_edge(edge_id)? {
                self.edges[edge_id].weight += retime_amount;
            }
        }
        for edge_id in fanouts {
            if !self.ignore_edge(edge_id)? {
                self.edges[edge_id].weight -= retime_amount;
            }
        }
        Ok(())
    }

    pub fn retime_single_node(&mut self, node_id: usize, lag: i32) -> Result<(), RetimeUtilError> {
        self.require_node(node_id)?;
        if lag == 0 {
            return Ok(());
        }

        let fanouts = self.nodes[node_id].fanouts.clone();
        for edge_id in fanouts {
            if !self.ignore_edge(edge_id)? {
                self.edges[edge_id].weight -= lag;
                self.edges[edge_id].latches_present = false;
                self.edges[edge_id].initial_values.clear();
            }
        }

        let fanins = self.nodes[node_id].fanins.clone();
        for edge_id in fanins {
            if !self.ignore_edge(edge_id)? {
                self.edges[edge_id].weight += lag;
                self.edges[edge_id].latches_present = false;
                self.edges[edge_id].initial_values.clear();
            }
        }

        Ok(())
    }

    pub fn check_graph(&self) -> Result<(), RetimeUtilError> {
        for node in &self.nodes {
            if node.node_type == RetimeNodeType::PrimaryInput && !node.fanins.is_empty() {
                return Err(RetimeUtilError::PrimaryInputHasFanins { node_id: node.id });
            }
            if node.node_type == RetimeNodeType::PrimaryOutput {
                for edge_id in &node.fanouts {
                    if !self.ignore_edge(*edge_id)? {
                        return Err(RetimeUtilError::PrimaryOutputHasFanouts { node_id: node.id });
                    }
                }
            }
        }

        for edge in &self.edges {
            if !self.ignore_edge(edge.id)? && edge.weight < 0 {
                return Err(RetimeUtilError::NegativeEdgeWeight {
                    edge_id: edge.id,
                    weight: edge.weight,
                });
            }
        }

        Ok(())
    }

    pub fn dump_graph(&self) -> String {
        let mut output = String::new();
        output.push_str("Inputs --\n\t");
        for node_id in &self.primary_inputs {
            output.push_str(&self.nodes[*node_id].name);
            output.push_str("  ");
        }
        output.push_str("\nOutputs --\n\t");
        for node_id in &self.primary_outputs {
            output.push_str(&self.nodes[*node_id].name);
            output.push_str("  ");
        }
        output.push_str("\n\nFanin(weight) --> Node -->Fanouts(weight)\n\n");

        for node in &self.nodes {
            if node.node_type == RetimeNodeType::PrimaryInput {
                continue;
            }
            for edge_id in &node.fanins {
                let edge = &self.edges[*edge_id];
                output.push_str(&format!(
                    "{}({})  ",
                    self.nodes[edge.source].name, edge.weight
                ));
            }
            output.push_str(&format!(" --> {} -->  ", node.name));
            for edge_id in &node.fanouts {
                let edge = &self.edges[*edge_id];
                output.push_str(&format!(
                    "{}({})  ",
                    self.nodes[edge.sink].name, edge.weight
                ));
            }
            output.push_str(&format!(
                "\n\t Delay: {:.2} , Area: {:.0}\n",
                node.final_delay, node.final_area
            ));
        }

        output
    }

    fn dfs_from_roots(
        &self,
        roots: &[usize],
        visit_inputs: bool,
    ) -> Result<Vec<usize>, RetimeUtilError> {
        let mut visited = HashMap::new();
        let mut node_vec = Vec::new();
        for root in roots {
            self.dfs_recur(
                *root,
                &mut node_vec,
                &mut visited,
                visit_inputs,
                INFINITY_LEVEL,
                0,
            )?;
        }
        Ok(node_vec)
    }

    fn dfs_recur(
        &self,
        node_id: usize,
        node_vec: &mut Vec<usize>,
        visited: &mut HashMap<usize, Option<i32>>,
        visit_inputs: bool,
        level: usize,
        weight: i32,
    ) -> Result<(), RetimeUtilError> {
        self.require_node(node_id)?;
        if level == 0 {
            return Ok(());
        }

        if let Some(active_weight) = visited.get(&node_id) {
            if *active_weight == Some(weight) {
                return Err(RetimeUtilError::ZeroWeightCycle { node_id, weight });
            }
            return Ok(());
        }

        visited.insert(node_id, Some(weight));
        if level > 1 {
            let edge_ids = if visit_inputs {
                self.nodes[node_id].fanins.clone()
            } else {
                self.nodes[node_id].fanouts.clone()
            };

            for edge_id in edge_ids {
                if self.ignore_edge(edge_id)? {
                    continue;
                }
                let edge = &self.edges[edge_id];
                let next = if visit_inputs { edge.source } else { edge.sink };
                self.dfs_recur(
                    next,
                    node_vec,
                    visited,
                    visit_inputs,
                    level.saturating_sub(1),
                    weight + edge.weight,
                )?;
            }
        }

        visited.remove(&node_id);
        node_vec.push(node_id);
        Ok(())
    }

    fn ignore_edge(&self, edge_id: usize) -> Result<bool, RetimeUtilError> {
        let edge = self
            .edges
            .get(edge_id)
            .ok_or(RetimeUtilError::MissingEdge { edge_id })?;
        Ok(self.nodes[edge.source].node_type == RetimeNodeType::Ignore
            || self.nodes[edge.sink].node_type == RetimeNodeType::Ignore)
    }

    fn require_node(&self, node_id: usize) -> Result<(), RetimeUtilError> {
        if node_id < self.nodes.len() {
            Ok(())
        } else {
            Err(RetimeUtilError::MissingNode { node_id })
        }
    }

    fn require_internal_node(&self, node_id: usize) -> Result<(), RetimeUtilError> {
        self.require_node(node_id)?;
        if self.nodes[node_id].node_type == RetimeNodeType::Internal {
            Ok(())
        } else {
            Err(RetimeUtilError::CannotRetimeNonInternal {
                node_id,
                node_type: self.nodes[node_id].node_type,
            })
        }
    }
}

pub fn retime_simulate_gate(gate: &MappedGate, load: f64) -> f64 {
    gate.pins.iter().fold(NEG_LARGE, |delay, pin| {
        let rise = pin.block.rise + pin.load_slope * load;
        let fall = pin.block.fall + pin.load_slope * load;
        delay.max(rise.max(fall))
    })
}

pub fn retime_get_user_constraint(
    node_type: RetimeNodeType,
    constraint: Option<&UserTimingConstraint>,
) -> f64 {
    let Some(constraint) = constraint else {
        return RETIME_USER_NOT_SET;
    };

    match node_type {
        RetimeNodeType::PrimaryInput => constraint.arrival.map_or(RETIME_USER_NOT_SET, |time| {
            if constraint.edge_flag == ClockEdgeFlag::BeforeClockEdge {
                -time.rise.min(time.fall)
            } else {
                time.rise.max(time.fall)
            }
        }),
        RetimeNodeType::PrimaryOutput => constraint.required.map_or(RETIME_USER_NOT_SET, |time| {
            if constraint.edge_flag == ClockEdgeFlag::BeforeClockEdge {
                -time.rise.max(time.fall)
            } else {
                time.rise.min(time.fall)
            }
        }),
        RetimeNodeType::Internal | RetimeNodeType::Ignore => RETIME_USER_NOT_SET,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeLatchInfo {
    pub latch_type: SynchronizationType,
    pub control_name: Option<String>,
    pub initial_value: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockData {
    pub found_unknown_type: bool,
    pub should_init: bool,
    pub old_type: SynchronizationType,
    pub latch_delay: f64,
    pub latch_area: f64,
}

pub fn retime_get_clock_data_from_latches(
    latches: &[RetimeLatchInfo],
    use_mapped: bool,
    mapped_dff: Option<&MappedGate>,
) -> Result<ClockData, RetimeUtilError> {
    let mut found_unknown_type = false;
    let mut old_type = SynchronizationType::Unknown;
    let mut first_known = true;
    let mut prev_control: Option<&str> = None;
    let mut all_initial_unknown = true;

    for latch in latches {
        match latch.latch_type {
            SynchronizationType::RisingEdge | SynchronizationType::FallingEdge => {
                if first_known {
                    old_type = latch.latch_type;
                    first_known = false;
                } else if latch.latch_type != old_type {
                    return Err(RetimeUtilError::MixedSynchronizationTypes);
                }

                let control = latch.control_name.as_deref();
                if let Some(previous) = prev_control {
                    if Some(previous) != control {
                        return Err(RetimeUtilError::DifferentLatchControls);
                    }
                } else {
                    prev_control = control;
                }
            }
            SynchronizationType::Unknown => {
                found_unknown_type = true;
            }
        }
        all_initial_unknown = all_initial_unknown && latch.initial_value >= 2;
    }

    let (latch_area, latch_delay) = if use_mapped {
        let gate = mapped_dff.ok_or(RetimeUtilError::MissingMappedDff)?;
        let load = gate.pins.first().map_or(0.0, |pin| pin.load_slope);
        (gate.area, retime_simulate_gate(gate, load))
    } else {
        (0.0, 0.0)
    };

    Ok(ClockData {
        found_unknown_type,
        should_init: !all_initial_unknown,
        old_type,
        latch_delay,
        latch_area,
    })
}

pub fn retime_is_modeled_network_retimable(latches: &[RetimeLatchInfo]) -> bool {
    retime_get_clock_data_from_latches(latches, false, None).is_ok()
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetimeUtilError {
    MissingSisDependencies {
        operation: &'static str,
    },
    MissingNode {
        node_id: usize,
    },
    MissingEdge {
        edge_id: usize,
    },
    MissingMappedGate {
        node_id: usize,
    },
    MissingMappedDff,
    CannotRetimeNonInternal {
        node_id: usize,
        node_type: RetimeNodeType,
    },
    PrimaryInputHasFanins {
        node_id: usize,
    },
    PrimaryOutputHasFanouts {
        node_id: usize,
    },
    NegativeEdgeWeight {
        edge_id: usize,
        weight: i32,
    },
    ZeroWeightCycle {
        node_id: usize,
        weight: i32,
    },
    MixedSynchronizationTypes,
    DifferentLatchControls,
}

impl fmt::Display for RetimeUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => {
                write!(f, "{operation} requires native prerequisite ports")
            }
            Self::MissingNode { node_id } => write!(f, "missing retime node {node_id}"),
            Self::MissingEdge { edge_id } => write!(f, "missing retime edge {edge_id}"),
            Self::MissingMappedGate { node_id } => {
                write!(f, "missing mapped gate data for retime node {node_id}")
            }
            Self::MissingMappedDff => write!(f, "missing mapped D-FF gate data"),
            Self::CannotRetimeNonInternal { node_id, node_type } => {
                write!(
                    f,
                    "cannot retime non-internal node {node_id} ({node_type:?})"
                )
            }
            Self::PrimaryInputHasFanins { node_id } => {
                write!(f, "primary input node {node_id} has non-zero fanins")
            }
            Self::PrimaryOutputHasFanouts { node_id } => {
                write!(f, "primary output node {node_id} has non-zero fanouts")
            }
            Self::NegativeEdgeWeight { edge_id, weight } => {
                write!(f, "retime edge {edge_id} has negative weight {weight}")
            }
            Self::ZeroWeightCycle { node_id, weight } => {
                write!(
                    f,
                    "retime graph contains a zero-weight cycle through node {node_id} at path weight {weight}"
                )
            }
            Self::MixedSynchronizationTypes => {
                write!(f, "synchronization types of clocked latches differ")
            }
            Self::DifferentLatchControls => write!(f, "latches are not clocked by the same signal"),
        }
    }
}

impl Error for RetimeUtilError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate() -> MappedGate {
        MappedGate::new(
            3.0,
            vec![
                GatePinDelay::new(1.0, 1.5, 0.2),
                GatePinDelay::new(0.5, 2.0, 0.1),
            ],
        )
    }

    fn small_graph() -> RetimeGraph {
        let mut graph = RetimeGraph::new();
        let a = graph
            .add_node_from_spec(
                RetimeNodeSpec::new("a", RetimeNodeType::PrimaryInput),
                false,
            )
            .unwrap();
        let mut n_spec = RetimeNodeSpec::new("n", RetimeNodeType::Internal);
        n_spec.literal_count = 4;
        n_spec.effective_fanout_count = 3;
        let n = graph.add_node_from_spec(n_spec, false).unwrap();
        let z = graph
            .add_node_from_spec(
                RetimeNodeSpec::new("z", RetimeNodeType::PrimaryOutput),
                false,
            )
            .unwrap();
        graph.add_edge(a, n, 1, 2.0, 0).unwrap();
        graph.add_edge(n, z, 0, 2.0, 0).unwrap();
        graph
    }

    #[test]
    fn add_node_ports_pi_po_and_unit_fanout_delay() {
        let graph = small_graph();
        assert_eq!(graph.primary_inputs, vec![0]);
        assert_eq!(graph.primary_outputs, vec![2]);
        assert_eq!(graph.nodes[1].final_area, 4.0);
        assert_eq!(graph.nodes[1].final_delay, 1.6);
        assert_eq!(graph.nodes[0].final_delay, 0.0);
        assert_eq!(graph.nodes[2].final_area, 0.0);
    }

    #[test]
    fn mapped_gate_node_uses_gate_area_and_max_pin_delay() {
        let mut graph = RetimeGraph::new();
        let mut spec = RetimeNodeSpec::new("mapped", RetimeNodeType::Internal);
        spec.literal_count = 1;
        spec.mapped_gate = Some(gate());
        spec.mapped_load = 5.0;
        let id = graph.add_node_from_spec(spec, true).unwrap();
        assert_eq!(graph.nodes[id].final_area, 3.0);
        assert_eq!(graph.nodes[id].final_delay, 2.5);
    }

    #[test]
    fn add_edge_deduplicates_by_sink_weight_and_fanin_index() {
        let mut graph = small_graph();
        let first = graph.add_edge(0, 1, 1, 8.0, 0).unwrap();
        let second = graph.add_edge(0, 1, 1, 9.0, 0).unwrap();
        let third = graph.add_edge(0, 1, 2, 9.0, 0).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, third);
    }

    #[test]
    fn dfs_from_outputs_returns_postorder_and_detects_zero_weight_cycle() {
        let graph = small_graph();
        assert_eq!(graph.dfs_from_outputs().unwrap(), vec![0, 1, 2]);

        let mut cyclic = RetimeGraph::new();
        let mut spec = RetimeNodeSpec::new("x", RetimeNodeType::Internal);
        spec.literal_count = 1;
        let x = cyclic.add_node_from_spec(spec.clone(), false).unwrap();
        spec.name = "y".to_owned();
        let y = cyclic.add_node_from_spec(spec, false).unwrap();
        cyclic.primary_outputs.push(y);
        cyclic.add_edge(x, y, 0, 1.0, 0).unwrap();
        cyclic.add_edge(y, x, 0, 1.0, 0).unwrap();
        assert!(matches!(
            cyclic.dfs_from_outputs(),
            Err(RetimeUtilError::ZeroWeightCycle { .. })
        ));
    }

    #[test]
    fn retime_node_adjusts_incident_weights_only_for_internal_nodes() {
        let mut graph = small_graph();
        graph.retime_node(1, 2).unwrap();
        assert_eq!(graph.edges[0].weight, 3);
        assert_eq!(graph.edges[1].weight, -2);
        assert!(matches!(
            graph.retime_node(0, 1),
            Err(RetimeUtilError::CannotRetimeNonInternal { .. })
        ));
    }

    #[test]
    fn single_node_retime_clears_latch_correspondence() {
        let mut graph = small_graph();
        graph.edges[0].latches_present = true;
        graph.edges[0].initial_values = vec![1, 0];
        graph.retime_single_node(1, 1).unwrap();
        assert_eq!(graph.edges[0].weight, 2);
        assert!(!graph.edges[0].latches_present);
        assert!(graph.edges[0].initial_values.is_empty());
    }

    #[test]
    fn check_graph_reports_illegal_io_and_negative_weights() {
        let mut graph = small_graph();
        graph.edges[1].weight = -1;
        assert!(matches!(
            graph.check_graph(),
            Err(RetimeUtilError::NegativeEdgeWeight { .. })
        ));
        graph.edges[1].weight = 0;
        graph.add_edge(2, 1, 0, 1.0, 1).unwrap();
        assert!(matches!(
            graph.check_graph(),
            Err(RetimeUtilError::PrimaryOutputHasFanouts { .. })
        ));
    }

    #[test]
    fn user_constraints_match_c_pi_po_sign_rules() {
        let before = UserTimingConstraint {
            edge_flag: ClockEdgeFlag::BeforeClockEdge,
            arrival: Some(DelayTime::new(3.0, 5.0)),
            required: Some(DelayTime::new(7.0, 11.0)),
        };
        let after = UserTimingConstraint {
            edge_flag: ClockEdgeFlag::AfterClockEdge,
            arrival: Some(DelayTime::new(3.0, 5.0)),
            required: Some(DelayTime::new(7.0, 11.0)),
        };
        assert_eq!(
            retime_get_user_constraint(RetimeNodeType::PrimaryInput, Some(&before)),
            -3.0
        );
        assert_eq!(
            retime_get_user_constraint(RetimeNodeType::PrimaryOutput, Some(&before)),
            -11.0
        );
        assert_eq!(
            retime_get_user_constraint(RetimeNodeType::PrimaryInput, Some(&after)),
            5.0
        );
        assert_eq!(
            retime_get_user_constraint(RetimeNodeType::PrimaryOutput, Some(&after)),
            7.0
        );
    }

    #[test]
    fn clock_data_validates_latch_type_control_and_initialization() {
        let latches = vec![
            RetimeLatchInfo {
                latch_type: SynchronizationType::RisingEdge,
                control_name: Some("clk".to_owned()),
                initial_value: 2,
            },
            RetimeLatchInfo {
                latch_type: SynchronizationType::Unknown,
                control_name: None,
                initial_value: 3,
            },
        ];
        let data = retime_get_clock_data_from_latches(&latches, false, None).unwrap();
        assert!(data.found_unknown_type);
        assert!(!data.should_init);
        assert_eq!(data.old_type, SynchronizationType::RisingEdge);

        let mixed = vec![
            RetimeLatchInfo {
                latch_type: SynchronizationType::RisingEdge,
                control_name: Some("clk".to_owned()),
                initial_value: 0,
            },
            RetimeLatchInfo {
                latch_type: SynchronizationType::FallingEdge,
                control_name: Some("clk".to_owned()),
                initial_value: 0,
            },
        ];
        assert_eq!(
            retime_get_clock_data_from_latches(&mixed, false, None),
            Err(RetimeUtilError::MixedSynchronizationTypes)
        );
    }
    #[test]
    fn dump_graph_uses_c_style_sections() {
        let graph = small_graph();
        let dump = graph.dump_graph();
        assert!(dump.contains("Inputs --"));
        assert!(dump.contains("Outputs --"));
        assert!(dump.contains("a(1)"));
        assert!(dump.contains("Delay: 1.60 , Area: 4"));
    }
}
