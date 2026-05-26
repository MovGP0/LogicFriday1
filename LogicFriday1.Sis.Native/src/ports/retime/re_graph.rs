//! Native Rust model for `LogicSynthesis/sis/retime/re_graph.c`.
//!
//! The C unit is the graph-level retiming driver: it legalizes negative edge
//! weights, checks whether the current cycle time already satisfies the user
//! constraint, binary-searches for a feasible retiming, and finally either
//! delegates initial-state computation or moves latches with unknown initial
//! values. This port keeps that control flow over owned Rust data. Solver and
//! SIS-network integration are explicit Rust dependencies, not legacy C ABI
//! exports.

use std::error::Error;
use std::fmt;

pub const EPS: f64 = 1.0e-9;
pub const FUDGE: f64 = 1.0;
pub const RETIME_TEST_NOT_SET: f64 = -50_000.0;
pub const UNKNOWN_INITIAL_VALUE: i32 = 3;

pub fn sis_retime_graph_blocked() -> Result<(), RetimeGraphError> {
    Err(RetimeGraphError::MissingNativeDependencies {
        operation: "retime_graph over SIS network_t/re_graph",
    })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EdgeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeType {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNode {
    pub id: NodeId,
    pub name: String,
    pub node_type: RetimeNodeType,
    pub fanins: Vec<EdgeId>,
    pub fanouts: Vec<EdgeId>,
    pub final_area: f64,
    pub final_delay: f64,
    pub user_time: f64,
}

impl RetimeNode {
    pub fn new(name: impl Into<String>, node_type: RetimeNodeType) -> Self {
        Self {
            id: NodeId(usize::MAX),
            name: name.into(),
            node_type,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            final_area: 0.0,
            final_delay: 0.0,
            user_time: RETIME_TEST_NOT_SET - 1.0,
        }
    }

    pub fn with_delay(mut self, delay: f64) -> Self {
        self.final_delay = delay;
        self
    }

    pub fn with_area(mut self, area: f64) -> Self {
        self.final_area = area;
        self
    }

    pub fn with_user_time(mut self, user_time: f64) -> Self {
        self.user_time = user_time;
        self
    }

    fn is_internal(&self) -> bool {
        self.node_type == RetimeNodeType::Internal
    }

    fn is_ignored(&self) -> bool {
        self.node_type == RetimeNodeType::Ignore
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeEdge {
    pub id: EdgeId,
    pub source: NodeId,
    pub sink: NodeId,
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
    pub primary_inputs: Vec<NodeId>,
    pub primary_outputs: Vec<NodeId>,
}

impl RetimeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, mut node: RetimeNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.id = id;
        match node.node_type {
            RetimeNodeType::PrimaryInput => self.primary_inputs.push(id),
            RetimeNodeType::PrimaryOutput => self.primary_outputs.push(id),
            RetimeNodeType::Internal | RetimeNodeType::Ignore => {}
        }
        self.nodes.push(node);
        id
    }

    pub fn add_edge(
        &mut self,
        source: NodeId,
        sink: NodeId,
        sink_fanin_id: usize,
        weight: i32,
        breadth: f64,
    ) -> Result<EdgeId, RetimeGraphError> {
        self.require_node(source)?;
        self.require_node(sink)?;

        let id = EdgeId(self.edges.len());
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
        self.nodes[source.0].fanouts.push(id);
        self.nodes[sink.0].fanins.push(id);
        Ok(id)
    }

    pub fn cycle_delay(&self, latch_delay: f64) -> Result<f64, RetimeGraphError> {
        let mut valid = vec![false; self.nodes.len()];
        let mut delay_table = vec![0.0; self.nodes.len()];

        for node in &self.nodes {
            if node.fanins.is_empty() {
                valid[node.id.0] = true;
                if node.user_time > RETIME_TEST_NOT_SET {
                    delay_table[node.id.0] = node.user_time;
                }
            }
        }

        for index in (0..self.nodes.len()).rev() {
            if !valid[index] {
                self.evaluate_delay(NodeId(index), &mut valid, &mut delay_table, &mut Vec::new())?;
            }
        }

        let mut critical = 0.0;
        for node in &self.nodes {
            let offset = if node.node_type == RetimeNodeType::PrimaryOutput
                && node.user_time > RETIME_TEST_NOT_SET
            {
                -node.user_time
            } else if self.max_fanout_weight(node.id)? > 0 {
                latch_delay
            } else {
                0.0
            };
            critical = f64::max(critical, delay_table[node.id.0] + offset);
        }

        Ok(critical)
    }

    pub fn sum_node_area(&self) -> f64 {
        self.nodes
            .iter()
            .filter(|node| node.node_type == RetimeNodeType::Internal)
            .map(|node| node.final_area)
            .sum()
    }

    pub fn check_graph(&self) -> Result<(), RetimeGraphError> {
        for node in &self.nodes {
            match node.node_type {
                RetimeNodeType::PrimaryInput if !node.fanins.is_empty() => {
                    return Err(RetimeGraphError::PrimaryInputHasFanins { node: node.id });
                }
                RetimeNodeType::PrimaryOutput => {
                    for edge in &node.fanouts {
                        if !self.ignore_edge(*edge)? {
                            return Err(RetimeGraphError::PrimaryOutputHasFanouts {
                                node: node.id,
                            });
                        }
                    }
                }
                RetimeNodeType::PrimaryInput
                | RetimeNodeType::Internal
                | RetimeNodeType::Ignore => {}
            }
        }

        for edge in &self.edges {
            if !self.ignore_edge(edge.id)? && edge.weight < 0 {
                return Err(RetimeGraphError::NegativeEdgeWeight {
                    edge: edge.id,
                    weight: edge.weight,
                });
            }
        }
        Ok(())
    }

    pub fn retime_single_node(&mut self, node: NodeId, lag: i32) -> Result<(), RetimeGraphError> {
        self.require_node(node)?;
        if lag == 0 {
            return Ok(());
        }

        let fanouts = self.nodes[node.0].fanouts.clone();
        for edge in fanouts {
            if !self.ignore_edge(edge)? {
                self.edges[edge.0].weight -= lag;
                self.edges[edge.0].latches_present = false;
                self.edges[edge.0].initial_values.clear();
            }
        }

        let fanins = self.nodes[node.0].fanins.clone();
        for edge in fanins {
            if !self.ignore_edge(edge)? {
                self.edges[edge.0].weight += lag;
                self.edges[edge.0].latches_present = false;
                self.edges[edge.0].initial_values.clear();
            }
        }

        Ok(())
    }

    pub fn set_unknown_initial_values(&mut self) -> Result<(), RetimeGraphError> {
        for edge_id in 0..self.edges.len() {
            let edge = EdgeId(edge_id);
            if self.ignore_edge(edge)? {
                continue;
            }
            if self.edges[edge_id].weight > 0 {
                let len = usize::try_from(self.edges[edge_id].weight).map_err(|_| {
                    RetimeGraphError::NegativeEdgeWeight {
                        edge,
                        weight: self.edges[edge_id].weight,
                    }
                })?;
                self.edges[edge_id].initial_values = vec![UNKNOWN_INITIAL_VALUE; len];
                self.edges[edge_id].latches_present = false;
            }
        }
        Ok(())
    }

    pub fn apply_retiming_with_unknown_initials(
        &mut self,
        retiming: &[i32],
    ) -> Result<bool, RetimeGraphError> {
        if retiming.len() != self.nodes.len() {
            return Err(RetimeGraphError::RetimingLengthMismatch {
                expected: self.nodes.len(),
                actual: retiming.len(),
            });
        }

        let mut changed = false;
        for index in (0..self.nodes.len()).rev() {
            let lag = retiming[index];
            if lag == 0 {
                continue;
            }

            changed = true;
            let node = &self.nodes[index];
            if !node.is_internal() {
                return Err(RetimeGraphError::HostVertexRetimed {
                    node: node.id,
                    value: lag,
                });
            }
            self.retime_single_node(NodeId(index), lag)?;
        }

        self.set_unknown_initial_values()?;
        Ok(changed)
    }

    fn evaluate_delay(
        &self,
        node: NodeId,
        valid: &mut [bool],
        delay_table: &mut [f64],
        active: &mut Vec<NodeId>,
    ) -> Result<(), RetimeGraphError> {
        self.require_node(node)?;
        if valid[node.0] {
            return Ok(());
        }
        if active.contains(&node) {
            return Err(RetimeGraphError::ZeroWeightDelayCycle { node });
        }

        active.push(node);
        for edge_id in &self.nodes[node.0].fanins {
            if self.ignore_edge(*edge_id)? {
                continue;
            }
            let edge = &self.edges[edge_id.0];
            if edge.weight == 0 && !valid[edge.source.0] {
                self.evaluate_delay(edge.source, valid, delay_table, active)?;
            }
        }

        let mut max_fanin_delay = 0.0;
        for edge_id in &self.nodes[node.0].fanins {
            if self.ignore_edge(*edge_id)? {
                continue;
            }
            let edge = &self.edges[edge_id.0];
            if edge.weight == 0 {
                max_fanin_delay = f64::max(max_fanin_delay, delay_table[edge.source.0]);
            }
        }

        active.pop();
        valid[node.0] = true;
        delay_table[node.0] = max_fanin_delay + self.nodes[node.0].final_delay;
        Ok(())
    }

    fn max_fanout_weight(&self, node: NodeId) -> Result<i32, RetimeGraphError> {
        self.require_node(node)?;
        let mut max_weight = 0;
        for edge_id in &self.nodes[node.0].fanouts {
            if !self.ignore_edge(*edge_id)? {
                max_weight = max_weight.max(self.edges[edge_id.0].weight);
            }
        }
        Ok(max_weight)
    }

    fn has_negative_edge_weight(&self) -> Result<bool, RetimeGraphError> {
        for edge in &self.edges {
            if !self.ignore_edge(edge.id)? && edge.weight < 0 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn ignore_edge(&self, edge: EdgeId) -> Result<bool, RetimeGraphError> {
        self.require_edge(edge)?;
        let edge = &self.edges[edge.0];
        Ok(self.nodes[edge.source.0].is_ignored() || self.nodes[edge.sink.0].is_ignored())
    }

    fn require_node(&self, node: NodeId) -> Result<(), RetimeGraphError> {
        match self.nodes.get(node.0) {
            Some(existing) if existing.id == node => Ok(()),
            _ => Err(RetimeGraphError::MissingNode(node)),
        }
    }

    fn require_edge(&self, edge: EdgeId) -> Result<(), RetimeGraphError> {
        match self.edges.get(edge.0) {
            Some(existing) if existing.id == edge => Ok(()),
            _ => Err(RetimeGraphError::MissingEdge(edge)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeAlgorithm {
    Lieserson,
    Nanni,
    MinRegister,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitialStateMode {
    Suppressed,
    WhenSmall,
    Force,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetimeGraphOptions {
    pub area_r: f64,
    pub delay_r: f64,
    pub retime_tol: f64,
    pub target_cycle_time: f64,
    pub algorithm: RetimeAlgorithm,
    pub initial_state_mode: InitialStateMode,
}

impl RetimeGraphOptions {
    pub fn new(target_cycle_time: f64) -> Self {
        Self {
            area_r: 0.0,
            delay_r: 0.0,
            retime_tol: 0.0,
            target_cycle_time,
            algorithm: RetimeAlgorithm::Lieserson,
            initial_state_mode: InitialStateMode::Suppressed,
        }
    }

    pub fn minimize_registers(mut self) -> Self {
        self.algorithm = RetimeAlgorithm::MinRegister;
        self
    }

    pub fn use_nanni(mut self) -> Self {
        self.algorithm = RetimeAlgorithm::Nanni;
        self
    }

    fn validate(&self) -> Result<(), RetimeGraphError> {
        for (name, value) in [
            ("area_r", self.area_r),
            ("delay_r", self.delay_r),
            ("retime_tol", self.retime_tol),
            ("target_cycle_time", self.target_cycle_time),
        ] {
            if !value.is_finite() {
                return Err(RetimeGraphError::NonFiniteOption { name, value });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SolverStatus {
    Feasible { retiming: Vec<i32> },
    Infeasible,
}

pub trait RetimeSolver {
    fn solve(
        &mut self,
        graph: &mut RetimeGraph,
        cycle_time_without_latch_delay: f64,
        algorithm: RetimeAlgorithm,
    ) -> Result<SolverStatus, RetimeGraphError>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetimeGraphStatus {
    AlreadyMeetsConstraint,
    Changed,
    Unchanged,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeGraphReport {
    pub status: RetimeGraphStatus,
    pub new_cycle_time: f64,
    pub can_initialize: bool,
    pub initial_cycle_time: f64,
    pub initial_register_count: usize,
    pub initial_logic_cost: f64,
    pub initial_register_cost: f64,
    pub attempts: Vec<RetimeAttempt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeAttempt {
    pub attempted_cycle_time: f64,
    pub solver_cycle_time: f64,
    pub feasible: bool,
    pub resulting_cycle_time: Option<f64>,
}

pub fn retime_graph_interface<S: RetimeSolver>(
    graph: &mut RetimeGraph,
    area_r: f64,
    delay_r: f64,
    retime_tol: f64,
    target_cycle_time: f64,
    solver: &mut S,
) -> Result<RetimeGraphReport, RetimeGraphError> {
    let mut options = RetimeGraphOptions::new(target_cycle_time);
    options.area_r = area_r;
    options.delay_r = delay_r;
    options.retime_tol = retime_tol;
    retime_graph(graph, options, 0, solver)
}

pub fn retime_graph<S: RetimeSolver>(
    graph: &mut RetimeGraph,
    options: RetimeGraphOptions,
    initial_register_count: usize,
    solver: &mut S,
) -> Result<RetimeGraphReport, RetimeGraphError> {
    options.validate()?;

    let mut best_cycle = graph.cycle_delay(options.delay_r)?;
    let mut made_legal = false;
    if graph.has_negative_edge_weight()? {
        made_legal = true;
        let status = solver.solve(graph, best_cycle + FUDGE, RetimeAlgorithm::Lieserson)?;
        if matches!(status, SolverStatus::Infeasible) {
            return Err(RetimeGraphError::CannotLegalizeNegativeLatches);
        }
        if graph.has_negative_edge_weight()? {
            return Err(RetimeGraphError::CannotLegalizeNegativeLatches);
        }
    }

    best_cycle = graph.cycle_delay(options.delay_r)?;
    let initial_cycle = best_cycle;
    if options.algorithm != RetimeAlgorithm::MinRegister && best_cycle <= options.target_cycle_time
    {
        return Ok(RetimeGraphReport {
            status: RetimeGraphStatus::AlreadyMeetsConstraint,
            new_cycle_time: best_cycle,
            can_initialize: true,
            initial_cycle_time: initial_cycle,
            initial_register_count,
            initial_logic_cost: graph.sum_node_area(),
            initial_register_cost: initial_register_count as f64 * options.area_r,
            attempts: Vec::new(),
        });
    }

    let mut attempts = Vec::new();
    let mut max_fail_cycle = options.target_cycle_time;
    let mut attempting_cycle = options.target_cycle_time;
    let mut best_retiming = None;

    loop {
        let mut candidate = graph.clone();
        let solver_cycle = attempting_cycle - options.delay_r;
        let status = solver.solve(&mut candidate, solver_cycle, options.algorithm)?;

        match status {
            SolverStatus::Infeasible => {
                attempts.push(RetimeAttempt {
                    attempted_cycle_time: attempting_cycle,
                    solver_cycle_time: solver_cycle,
                    feasible: false,
                    resulting_cycle_time: None,
                });
                if best_cycle - attempting_cycle > options.retime_tol {
                    max_fail_cycle = attempting_cycle;
                    attempting_cycle = 0.5 * (best_cycle + attempting_cycle);
                    continue;
                }
                break;
            }
            SolverStatus::Feasible { retiming } => {
                candidate.check_graph()?;
                let current_cycle = candidate.cycle_delay(options.delay_r)?;
                attempts.push(RetimeAttempt {
                    attempted_cycle_time: attempting_cycle,
                    solver_cycle_time: solver_cycle,
                    feasible: true,
                    resulting_cycle_time: Some(current_cycle),
                });

                if current_cycle > attempting_cycle + EPS {
                    return Err(RetimeGraphError::RetimedCycleExceedsAttempt {
                        current_cycle,
                        attempted_cycle_time: attempting_cycle,
                    });
                }

                best_cycle = current_cycle;
                best_retiming = Some(retiming);

                if current_cycle > options.target_cycle_time + options.retime_tol
                    && current_cycle > max_fail_cycle + options.retime_tol
                {
                    attempting_cycle = 0.5 * (max_fail_cycle + current_cycle);
                    continue;
                }
                break;
            }
        }
    }

    let mut changed = false;
    if let Some(retiming) = best_retiming {
        if options.initial_state_mode != InitialStateMode::Suppressed {
            return Err(RetimeGraphError::MissingNativeDependencies {
                operation: "retime_update_init_states",
            });
        }

        changed = graph.apply_retiming_with_unknown_initials(&retiming)?;
    }

    let status = if changed || made_legal {
        RetimeGraphStatus::Changed
    } else {
        RetimeGraphStatus::Unchanged
    };

    Ok(RetimeGraphReport {
        status,
        new_cycle_time: best_cycle,
        can_initialize: true,
        initial_cycle_time: initial_cycle,
        initial_register_count,
        initial_logic_cost: graph.sum_node_area(),
        initial_register_cost: initial_register_count as f64 * options.area_r,
        attempts,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetimeGraphError {
    MissingNativeDependencies {
        operation: &'static str,
    },
    MissingNode(NodeId),
    MissingEdge(EdgeId),
    PrimaryInputHasFanins {
        node: NodeId,
    },
    PrimaryOutputHasFanouts {
        node: NodeId,
    },
    NegativeEdgeWeight {
        edge: EdgeId,
        weight: i32,
    },
    ZeroWeightDelayCycle {
        node: NodeId,
    },
    RetimingLengthMismatch {
        expected: usize,
        actual: usize,
    },
    HostVertexRetimed {
        node: NodeId,
        value: i32,
    },
    CannotLegalizeNegativeLatches,
    RetimedCycleExceedsAttempt {
        current_cycle: f64,
        attempted_cycle_time: f64,
    },
    NonFiniteOption {
        name: &'static str,
        value: f64,
    },
}

impl fmt::Display for RetimeGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativeDependencies { operation } => {
                write!(f, "{operation} requires native prerequisite ports")
            }
            Self::MissingNode(node) => write!(f, "retime graph references missing node {}", node.0),
            Self::MissingEdge(edge) => write!(f, "retime graph references missing edge {}", edge.0),
            Self::PrimaryInputHasFanins { node } => {
                write!(f, "primary input node {} has fanins", node.0)
            }
            Self::PrimaryOutputHasFanouts { node } => {
                write!(f, "primary output node {} has fanouts", node.0)
            }
            Self::NegativeEdgeWeight { edge, weight } => {
                write!(f, "retime edge {} has negative weight {weight}", edge.0)
            }
            Self::ZeroWeightDelayCycle { node } => {
                write!(f, "zero-weight delay cycle reaches node {}", node.0)
            }
            Self::RetimingLengthMismatch { expected, actual } => write!(
                f,
                "retiming vector length mismatch: expected {expected}, got {actual}"
            ),
            Self::HostVertexRetimed { node, value } => {
                write!(f, "host vertex {} has non-zero retiming {value}", node.0)
            }
            Self::CannotLegalizeNegativeLatches => {
                write!(f, "cannot get rid of negative retime edge weights")
            }
            Self::RetimedCycleExceedsAttempt {
                current_cycle,
                attempted_cycle_time,
            } => write!(
                f,
                "retimed cycle {current_cycle:.3} exceeds attempted cycle {attempted_cycle_time:.3}"
            ),
            Self::NonFiniteOption { name, value } => {
                write!(f, "retime option {name} must be finite, got {value}")
            }
        }
    }
}

impl Error for RetimeGraphError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct ScriptedSolver {
        calls: Vec<(RetimeAlgorithm, f64)>,
        responses: Vec<SolverStatus>,
    }

    impl ScriptedSolver {
        fn with_responses(responses: Vec<SolverStatus>) -> Self {
            Self {
                calls: Vec::new(),
                responses,
            }
        }
    }

    impl RetimeSolver for ScriptedSolver {
        fn solve(
            &mut self,
            graph: &mut RetimeGraph,
            cycle_time_without_latch_delay: f64,
            algorithm: RetimeAlgorithm,
        ) -> Result<SolverStatus, RetimeGraphError> {
            self.calls.push((algorithm, cycle_time_without_latch_delay));
            let response = self.responses.remove(0);
            if let SolverStatus::Feasible { retiming } = &response {
                for (index, lag) in retiming.iter().copied().enumerate() {
                    if graph.nodes[index].node_type == RetimeNodeType::Internal {
                        graph.retime_single_node(NodeId(index), lag)?;
                    }
                }
            }
            Ok(response)
        }
    }

    fn chain_graph() -> RetimeGraph {
        let mut graph = RetimeGraph::new();
        let pi = graph.add_node(RetimeNode::new("a", RetimeNodeType::PrimaryInput));
        let n1 = graph.add_node(
            RetimeNode::new("n1", RetimeNodeType::Internal)
                .with_delay(2.0)
                .with_area(4.0),
        );
        let n2 = graph.add_node(
            RetimeNode::new("n2", RetimeNodeType::Internal)
                .with_delay(3.0)
                .with_area(5.0),
        );
        let po = graph.add_node(RetimeNode::new("z", RetimeNodeType::PrimaryOutput));
        graph.add_edge(pi, n1, 0, 0, 1.0).unwrap();
        graph.add_edge(n1, n2, 0, 0, 1.0).unwrap();
        graph.add_edge(n2, po, 0, 0, 1.0).unwrap();
        graph
    }

    #[test]
    fn cycle_delay_matches_c_zero_weight_delay_and_latch_offset_rules() {
        let mut graph = chain_graph();
        assert_eq!(graph.cycle_delay(0.5).unwrap(), 5.0);

        graph.edges[1].weight = 1;
        assert_eq!(graph.cycle_delay(0.5).unwrap(), 3.0);

        graph.nodes[3].user_time = -1.0;
        assert_eq!(graph.cycle_delay(0.5).unwrap(), 4.0);
    }

    #[test]
    fn current_graph_returns_without_solver_when_clock_already_meets_constraint() {
        let mut graph = chain_graph();
        let mut solver = ScriptedSolver::default();
        let mut options = RetimeGraphOptions::new(6.0);
        options.area_r = 2.0;

        let report = retime_graph(&mut graph, options, 3, &mut solver).unwrap();

        assert_eq!(report.status, RetimeGraphStatus::AlreadyMeetsConstraint);
        assert_eq!(report.new_cycle_time, 5.0);
        assert_eq!(report.initial_logic_cost, 9.0);
        assert_eq!(report.initial_register_cost, 6.0);
        assert!(solver.calls.is_empty());
    }

    #[test]
    fn negative_edges_are_legalized_with_lieserson_before_binary_search() {
        let mut graph = chain_graph();
        graph.edges[0].weight = 1;
        graph.edges[1].weight = -1;
        let mut solver = ScriptedSolver::with_responses(vec![
            SolverStatus::Feasible {
                retiming: vec![0, -1, 0, 0],
            },
            SolverStatus::Infeasible,
        ]);
        let mut options = RetimeGraphOptions::new(4.0);
        options.retime_tol = 2.0;

        let report = retime_graph(&mut graph, options, 0, &mut solver).unwrap();

        assert_eq!(solver.calls[0], (RetimeAlgorithm::Lieserson, 4.0));
        assert_eq!(report.status, RetimeGraphStatus::Changed);
        assert_eq!(graph.edges[1].weight, 0);
    }

    #[test]
    fn feasible_solution_applies_best_retiming_and_unknown_initial_values() {
        let mut graph = chain_graph();
        graph.edges[0].weight = 1;
        let mut solver = ScriptedSolver::with_responses(vec![SolverStatus::Feasible {
            retiming: vec![0, -1, 0, 0],
        }]);
        let options = RetimeGraphOptions::new(4.0);

        let report = retime_graph(&mut graph, options, 2, &mut solver).unwrap();

        assert_eq!(report.status, RetimeGraphStatus::Changed);
        assert_eq!(report.new_cycle_time, 3.0);
        assert_eq!(graph.edges[0].weight, 0);
        assert_eq!(graph.edges[1].weight, 1);
        assert_eq!(graph.edges[1].initial_values, vec![UNKNOWN_INITIAL_VALUE]);
    }

    #[test]
    fn infeasible_attempt_binary_searches_until_tolerance_window() {
        let mut graph = chain_graph();
        let mut solver = ScriptedSolver::with_responses(vec![
            SolverStatus::Infeasible,
            SolverStatus::Infeasible,
        ]);
        let mut options = RetimeGraphOptions::new(3.0);
        options.retime_tol = 1.0;

        let report = retime_graph(&mut graph, options, 0, &mut solver).unwrap();

        assert_eq!(report.status, RetimeGraphStatus::Unchanged);
        assert_eq!(solver.calls.len(), 2);
        assert_eq!(solver.calls[0], (RetimeAlgorithm::Lieserson, 3.0));
        assert_eq!(solver.calls[1], (RetimeAlgorithm::Lieserson, 4.0));
    }

    #[test]
    fn nanni_and_min_register_options_select_expected_solver() {
        let mut graph = chain_graph();
        let mut solver = ScriptedSolver::with_responses(vec![SolverStatus::Infeasible]);
        let mut options = RetimeGraphOptions::new(4.0).use_nanni();
        options.retime_tol = 2.0;

        retime_graph(&mut graph, options, 0, &mut solver).unwrap();
        assert_eq!(solver.calls[0].0, RetimeAlgorithm::Nanni);

        let mut graph = chain_graph();
        let mut solver = ScriptedSolver::with_responses(vec![SolverStatus::Infeasible]);
        let options = RetimeGraphOptions::new(6.0).minimize_registers();
        retime_graph(&mut graph, options, 0, &mut solver).unwrap();
        assert_eq!(solver.calls[0].0, RetimeAlgorithm::MinRegister);
    }
    #[test]
    fn host_vertex_retiming_is_rejected_when_applying_final_unknowns() {
        let mut graph = chain_graph();
        let error = graph
            .apply_retiming_with_unknown_initials(&[1, 0, 0, 0])
            .unwrap_err();

        assert_eq!(
            error,
            RetimeGraphError::HostVertexRetimed {
                node: NodeId(0),
                value: 1
            }
        );
    }
}
