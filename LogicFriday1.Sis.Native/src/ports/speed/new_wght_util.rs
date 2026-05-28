//! Native Rust model for feasible behavior in `sis/speed/new_wght_util.c`.
//!
//! The C module is the coordination point for new-speed local transforms. Its
//! full entry points depend on SIS `network_t`/`node_t`, delay tracing, local
//! optimization routines, BDD managers, and network replacement helpers. This
//! file ports the deterministic data-model and algorithmic parts over owned
//! Rust values, and reports the SIS-backed path as an explicit dependency error.

use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

pub const NSP_BDD_LIMIT: usize = 100_000;
pub const ACCEPTABLE_FRACTION: f64 = 0.2;
pub const MIN_ACCEPTABLE_SAVING: f64 = 0.01;
pub const NSP_EPSILON: f64 = 1.0e-6;
pub const POS_LARGE: f64 = 10_000.0;
pub const NEG_LARGE: f64 = -10_000.0;

pub const CLP: i32 = 0;
pub const FAN: i32 = 1;
pub const DUAL: i32 = 2;
pub const TRANSFORM_BASED: i32 = 1;
pub const BEST_BENEFIT: i32 = 0;
pub const BEST_BANG_FOR_BUCK: i32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub enum NewWeightError {
    SisGraphDependency {
        operation: &'static str,
        source: &'static str,
    },
    MissingNode(String),
    InvalidTransformIndex(usize),
    NonFiniteCost {
        transform_index: usize,
        value: f64,
    },
    SelectionVariableOutOfRange {
        variable: usize,
        variables: usize,
    },
}

impl fmt::Display for NewWeightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SisGraphDependency { operation, source } => {
                write!(f, "{operation} requires SIS graph weighting from {source}")
            }
            Self::MissingNode(node) => write!(f, "new weight graph references missing node {node}"),
            Self::InvalidTransformIndex(index) => {
                write!(f, "selected transform index {index} has no candidate")
            }
            Self::NonFiniteCost {
                transform_index,
                value,
            } => write!(
                f,
                "transform {transform_index} produced a non-finite cost or improvement: {value}"
            ),
            Self::SelectionVariableOutOfRange {
                variable,
                variables,
            } => write!(
                f,
                "selection expression references variable {variable}, but only {variables} variables exist"
            ),
        }
    }
}

impl Error for NewWeightError {}

pub fn compute_weight_from_sis_network() -> Result<(), NewWeightError> {
    Err(NewWeightError::SisGraphDependency {
        operation: "new_speed_compute_weight",
        source: "LogicSynthesis/sis/speed/new_wght_util.c:38",
    })
}

pub fn select_xform_from_sis_network() -> Result<(), NewWeightError> {
    Err(NewWeightError::SisGraphDependency {
        operation: "new_speed_select_xform",
        source: "LogicSynthesis/sis/speed/new_wght_util.c:1038",
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub fn min_edge(self) -> f64 {
        self.rise.min(self.fall)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    Mapped,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformType {
    Collapse,
    Fanout,
    Dual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformSelectionMode {
    BestBenefit,
    BestBangForBuck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionObjective {
    AreaBased,
    TransformBased,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransformCandidate {
    pub index: usize,
    pub transform_type: TransformType,
    pub on: bool,
    pub available: bool,
    pub improvement: f64,
    pub area_cost: f64,
}

impl TransformCandidate {
    pub const fn new(
        index: usize,
        transform_type: TransformType,
        on: bool,
        available: bool,
        improvement: f64,
        area_cost: f64,
    ) -> Self {
        Self {
            index,
            transform_type,
            on,
            available,
            improvement,
            area_cost,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BestTransform {
    pub index: Option<usize>,
    pub improvement: f64,
    pub area_cost: f64,
    pub retained_network: Option<TransformType>,
}

pub fn sp_improvement(best: Option<usize>, improvements: &[f64]) -> f64 {
    best.and_then(|index| improvements.get(index).copied())
        .unwrap_or(NEG_LARGE)
}

pub fn sp_cost(best: Option<usize>, costs: &[f64]) -> f64 {
    best.and_then(|index| costs.get(index).copied())
        .unwrap_or(POS_LARGE)
}

pub fn is_zero(value: f64) -> bool {
    value.abs() < NSP_EPSILON
}

pub fn select_best_transform(
    candidates: &[TransformCandidate],
    mode: TransformSelectionMode,
) -> Result<BestTransform, NewWeightError> {
    let mut best_index = None;
    let mut best_imp = NEG_LARGE;
    let mut best_area = POS_LARGE;

    for candidate in candidates.iter().filter(|candidate| candidate.on) {
        let (cur_impr, cur_area) = if candidate.available {
            validate_candidate(candidate)?;
            (candidate.improvement, candidate.area_cost)
        } else {
            (NEG_LARGE, POS_LARGE)
        };

        if best_imp == NEG_LARGE {
            best_imp = cur_impr;
            best_area = cur_area;
            best_index = Some(candidate.index);
        } else if is_zero(cur_impr - best_imp) {
            if cur_area < best_area {
                best_area = cur_area;
                best_index = Some(candidate.index);
            }
        } else if cur_impr > NSP_EPSILON {
            match mode {
                TransformSelectionMode::BestBenefit => {
                    if cur_impr > best_imp {
                        best_imp = cur_impr;
                        best_area = cur_area;
                        best_index = Some(candidate.index);
                    }
                }
                TransformSelectionMode::BestBangForBuck => {
                    if should_replace_by_bang_for_buck(cur_impr, cur_area, best_imp, best_area) {
                        best_imp = cur_impr;
                        best_area = cur_area;
                        best_index = Some(candidate.index);
                    }
                }
            }
        }
    }

    if best_imp < NSP_EPSILON {
        return Ok(BestTransform {
            index: None,
            improvement: NEG_LARGE,
            area_cost: POS_LARGE,
            retained_network: None,
        });
    }

    let index = best_index.expect("positive best improvement requires selected index");
    let retained_network = candidates
        .iter()
        .find(|candidate| candidate.index == index)
        .map(|candidate| candidate.transform_type)
        .ok_or(NewWeightError::InvalidTransformIndex(index))?;

    Ok(BestTransform {
        index: Some(index),
        improvement: best_imp,
        area_cost: best_area,
        retained_network: Some(retained_network),
    })
}

fn validate_candidate(candidate: &TransformCandidate) -> Result<(), NewWeightError> {
    if !candidate.improvement.is_finite() {
        return Err(NewWeightError::NonFiniteCost {
            transform_index: candidate.index,
            value: candidate.improvement,
        });
    }
    if !candidate.area_cost.is_finite() {
        return Err(NewWeightError::NonFiniteCost {
            transform_index: candidate.index,
            value: candidate.area_cost,
        });
    }
    Ok(())
}

fn should_replace_by_bang_for_buck(
    cur_impr: f64,
    cur_area: f64,
    best_imp: f64,
    best_area: f64,
) -> bool {
    if best_imp < NSP_EPSILON {
        return true;
    }
    if !is_zero(cur_area) && !is_zero(best_area) {
        return (cur_area < 0.0 && best_area > 0.0)
            || (cur_area > 0.0
                && best_area > 0.0
                && (cur_impr / cur_area) > (best_imp / best_area))
            || (cur_area < 0.0
                && best_area < 0.0
                && (cur_impr / cur_area) < (best_imp / best_area));
    }

    (is_zero(cur_area) && best_area > 0.0) || (is_zero(best_area) && cur_area < 0.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutImprovementInput {
    pub new_required: DelayTime,
    pub old_required: DelayTime,
    pub drive: DelayTime,
    pub load_diff: f64,
}

pub fn fanout_improvement(input: FanoutImprovementInput) -> f64 {
    let adjusted = DelayTime {
        rise: input.new_required.rise - input.drive.rise * input.load_diff,
        fall: input.new_required.fall - input.drive.fall * input.load_diff,
    };
    (adjusted.rise - input.old_required.rise).min(adjusted.fall - input.old_required.fall)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollapseImprovementInput {
    pub original_arrival: DelayTime,
    pub optimized_output_arrival: DelayTime,
    pub inversion_delay_saving: Option<DelayTime>,
    pub trivial_mapped_network: bool,
}

pub fn collapse_improvement(input: CollapseImprovementInput) -> f64 {
    if input.trivial_mapped_network {
        return -1.0;
    }

    let mut optimized = input.optimized_output_arrival;
    if let Some(saving) = input.inversion_delay_saving {
        optimized.rise -= saving.rise;
        optimized.fall -= saving.fall;
    }

    (input.original_arrival.rise - optimized.rise).min(input.original_arrival.fall - optimized.fall)
}

pub fn transform_area_cost(
    transform_type: TransformType,
    optimized_area: f64,
    duplicated_area: f64,
    original_area: f64,
) -> f64 {
    match transform_type {
        TransformType::Fanout => optimized_area,
        TransformType::Collapse | TransformType::Dual => {
            optimized_area + duplicated_area - original_area
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeightedNode<N> {
    pub id: N,
    pub fanins: Vec<CriticalFanin<N>>,
    pub local_improvement: f64,
    pub epsilon: f64,
    pub select_flag: bool,
}

impl<N> WeightedNode<N> {
    pub fn new(id: N, fanins: Vec<CriticalFanin<N>>, local_improvement: f64) -> Self {
        Self {
            id,
            fanins,
            local_improvement,
            epsilon: 0.0,
            select_flag: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CriticalFanin<N> {
    pub id: N,
    pub required: DelayTime,
    pub arrival: DelayTime,
}

impl<N> CriticalFanin<N> {
    pub const fn new(id: N, required: DelayTime, arrival: DelayTime) -> Self {
        Self {
            id,
            required,
            arrival,
        }
    }

    pub fn slack(&self) -> DelayTime {
        DelayTime {
            rise: self.required.rise - self.arrival.rise,
            fall: self.required.fall - self.arrival.fall,
        }
    }
}

pub fn compute_achievable_savings<N>(
    nodes_in_dfs_order: &mut [WeightedNode<N>],
    critical_slack: f64,
) -> Result<(), NewWeightError>
where
    N: Clone + Eq + Hash + ToString,
{
    let by_id: HashMap<N, usize> = nodes_in_dfs_order
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();

    for index in 0..nodes_in_dfs_order.len() {
        let mut critical_inputs = Vec::new();
        let mut min_input_slack = POS_LARGE;

        for fanin in &nodes_in_dfs_order[index].fanins {
            if !by_id.contains_key(&fanin.id) {
                continue;
            }

            let slack = fanin.slack().min_edge();
            if slack < critical_slack - NSP_EPSILON {
                min_input_slack = min_input_slack.min(slack);
                critical_inputs.push((fanin.id.clone(), slack));
            }
        }

        let input_epsilon = if critical_inputs.is_empty() {
            0.0
        } else {
            let mut input_epsilon = POS_LARGE;
            for (fanin_id, slack) in critical_inputs {
                let fanin_index = by_id
                    .get(&fanin_id)
                    .copied()
                    .ok_or_else(|| NewWeightError::MissingNode(fanin_id.to_string()))?;
                let fanin_epsilon = nodes_in_dfs_order[fanin_index].epsilon;
                input_epsilon = input_epsilon.min(fanin_epsilon + slack - min_input_slack);
            }
            input_epsilon
        };

        if nodes_in_dfs_order[index].local_improvement >= input_epsilon {
            nodes_in_dfs_order[index].epsilon = nodes_in_dfs_order[index].local_improvement;
            nodes_in_dfs_order[index].select_flag = true;
        } else {
            nodes_in_dfs_order[index].epsilon = input_epsilon;
            nodes_in_dfs_order[index].select_flag = false;
        }
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputSlack<N> {
    pub id: N,
    pub slack: DelayTime,
    pub critical: bool,
    pub fanin: Option<N>,
}

impl<N> OutputSlack<N> {
    pub const fn new(id: N, slack: DelayTime, critical: bool, fanin: Option<N>) -> Self {
        Self {
            id,
            slack,
            critical,
            fanin,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuaranteedSaving<N> {
    pub epsilon: f64,
    pub new_critical_slack: f64,
    pub new_min_slack: DelayTime,
    pub critical_output_count: usize,
    pub newly_critical_outputs: Vec<N>,
}

pub fn compute_guaranteed_saving<N>(
    outputs: &[OutputSlack<N>],
    node_epsilon: &HashMap<N, f64>,
) -> GuaranteedSaving<N>
where
    N: Clone + Eq + Hash,
{
    let min_po_slack = outputs
        .iter()
        .map(|output| output.slack.min_edge())
        .fold(POS_LARGE, f64::min);

    let mut new_min_slack = DelayTime::new(POS_LARGE, POS_LARGE);
    for output in outputs.iter().filter(|output| output.critical) {
        let improvement = output
            .fanin
            .as_ref()
            .and_then(|fanin| node_epsilon.get(fanin).copied())
            .unwrap_or(0.0);
        new_min_slack.rise = new_min_slack.rise.min(output.slack.rise + improvement);
        new_min_slack.fall = new_min_slack.fall.min(output.slack.fall + improvement);
    }

    let new_critical_slack = new_min_slack.min_edge();
    let epsilon = (new_critical_slack - min_po_slack).max(0.0);
    let adjusted_new_min = DelayTime {
        rise: new_min_slack.rise - NSP_EPSILON,
        fall: new_min_slack.fall - NSP_EPSILON,
    };

    let mut critical_output_count = 0;
    let mut newly_critical_outputs = Vec::new();
    for output in outputs {
        if output.critical {
            critical_output_count += 1;
        } else if output.slack.rise < adjusted_new_min.rise
            || output.slack.fall < adjusted_new_min.fall
        {
            critical_output_count += 1;
            newly_critical_outputs.push(output.id.clone());
        }
    }

    GuaranteedSaving {
        epsilon,
        new_critical_slack,
        new_min_slack: adjusted_new_min,
        critical_output_count,
        newly_critical_outputs,
    }
}

pub fn first_slack_diff(output_slacks: &[DelayTime]) -> Option<f64> {
    if output_slacks.len() == 1 {
        return None;
    }

    let min_slack =
        output_slacks
            .iter()
            .fold(DelayTime::new(POS_LARGE, POS_LARGE), |min_slack, slack| {
                DelayTime {
                    rise: min_slack.rise.min(slack.rise),
                    fall: min_slack.fall.min(slack.fall),
                }
            });

    let mut diff = DelayTime::new(POS_LARGE, POS_LARGE);
    for slack in output_slacks {
        if slack.rise > min_slack.rise + NSP_EPSILON {
            diff.rise = diff.rise.min(slack.rise - min_slack.rise);
        }
        if slack.fall > min_slack.fall + NSP_EPSILON {
            diff.fall = diff.fall.min(slack.fall - min_slack.fall);
        }
    }

    if diff.rise == POS_LARGE || diff.fall == POS_LARGE {
        None
    } else {
        Some(diff.min_edge())
    }
}

pub fn critical_edge(required: Option<DelayTime>, arrival: DelayTime, critical_slack: f64) -> bool {
    let Some(required) = required else {
        return false;
    };

    required.rise - arrival.rise < critical_slack - NSP_EPSILON
        || required.fall - arrival.fall < critical_slack - NSP_EPSILON
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetworkNodeShape {
    pub kind: NodeKind,
    pub fanin_count: usize,
}

pub fn trivial_mapped_network(nodes: &[NetworkNodeShape]) -> bool {
    let mut internal_count = 0;
    for node in nodes {
        if node.kind == NodeKind::Internal {
            internal_count += 1;
            if node.fanin_count > 1 {
                return false;
            }
        }
    }
    internal_count == 0
        || nodes
            .iter()
            .all(|node| node.kind != NodeKind::Internal || node.fanin_count <= 1)
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionCost<N> {
    pub node: N,
    pub variable: usize,
    pub area_cost: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CostArray {
    pub then_costs: Vec<i32>,
    pub max_area_cost: i32,
}

pub fn selection_costs<N>(
    num_variables: usize,
    weighted_nodes: &[SelectionCost<N>],
    objective: SelectionObjective,
) -> Result<CostArray, NewWeightError> {
    let mut then_costs = vec![POS_LARGE as i32; num_variables];
    let mut max_area_cost = NEG_LARGE as i32;
    let mut rounded = Vec::with_capacity(weighted_nodes.len());

    for node in weighted_nodes {
        if node.variable >= num_variables {
            return Err(NewWeightError::SelectionVariableOutOfRange {
                variable: node.variable,
                variables: num_variables,
            });
        }
        let area_cost = node.area_cost.ceil();
        if !area_cost.is_finite() || area_cost < i32::MIN as f64 || area_cost > i32::MAX as f64 {
            return Err(NewWeightError::NonFiniteCost {
                transform_index: node.variable,
                value: area_cost,
            });
        }
        let area_cost = area_cost as i32;
        max_area_cost = max_area_cost.max(area_cost);
        rounded.push((node.variable, area_cost));
    }

    for (variable, area_cost) in rounded {
        then_costs[variable] = match objective {
            SelectionObjective::TransformBased => area_cost + max_area_cost * num_variables as i32,
            SelectionObjective::AreaBased => 1 + num_variables as i32 * area_cost,
        };
    }

    Ok(CostArray {
        then_costs,
        max_area_cost,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionExpr {
    Zero,
    One,
    Var(usize),
    Not(Box<SelectionExpr>),
    And(Box<SelectionExpr>, Box<SelectionExpr>),
    Or(Box<SelectionExpr>, Box<SelectionExpr>),
}

impl SelectionExpr {
    pub fn var(index: usize) -> Self {
        Self::Var(index)
    }

    pub fn and(left: Self, right: Self) -> Self {
        Self::And(Box::new(left), Box::new(right))
    }

    pub fn or(left: Self, right: Self) -> Self {
        Self::Or(Box::new(left), Box::new(right))
    }

    pub fn evaluate(&self, assignment: &[bool]) -> Result<bool, NewWeightError> {
        match self {
            Self::Zero => Ok(false),
            Self::One => Ok(true),
            Self::Var(index) => {
                assignment
                    .get(*index)
                    .copied()
                    .ok_or(NewWeightError::SelectionVariableOutOfRange {
                        variable: *index,
                        variables: assignment.len(),
                    })
            }
            Self::Not(expr) => Ok(!expr.evaluate(assignment)?),
            Self::And(left, right) => Ok(left.evaluate(assignment)? && right.evaluate(assignment)?),
            Self::Or(left, right) => Ok(left.evaluate(assignment)? || right.evaluate(assignment)?),
        }
    }

    pub fn variables(&self) -> BTreeSet<usize> {
        let mut variables = BTreeSet::new();
        self.collect_variables(&mut variables);
        variables
    }

    fn collect_variables(&self, variables: &mut BTreeSet<usize>) {
        match self {
            Self::Var(index) => {
                variables.insert(*index);
            }
            Self::Not(expr) => expr.collect_variables(variables),
            Self::And(left, right) | Self::Or(left, right) => {
                left.collect_variables(variables);
                right.collect_variables(variables);
            }
            Self::Zero | Self::One => {}
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selection<N> {
    pub nodes: Vec<N>,
    pub cost: i32,
}

pub fn best_selection<N: Clone>(
    expr: &SelectionExpr,
    variables: &[N],
    then_costs: &[i32],
    else_cost: i32,
) -> Result<Option<Selection<N>>, NewWeightError> {
    if variables.len() != then_costs.len() {
        return Err(NewWeightError::SelectionVariableOutOfRange {
            variable: then_costs.len(),
            variables: variables.len(),
        });
    }

    for variable in expr.variables() {
        if variable >= variables.len() {
            return Err(NewWeightError::SelectionVariableOutOfRange {
                variable,
                variables: variables.len(),
            });
        }
    }

    let mut assignment = vec![false; variables.len()];
    let mut best: Option<Selection<N>> = None;
    search_selection(
        0,
        expr,
        variables,
        then_costs,
        else_cost,
        &mut assignment,
        &mut best,
    )?;

    Ok(best)
}

fn search_selection<N: Clone>(
    index: usize,
    expr: &SelectionExpr,
    variables: &[N],
    then_costs: &[i32],
    else_cost: i32,
    assignment: &mut [bool],
    best: &mut Option<Selection<N>>,
) -> Result<(), NewWeightError> {
    if index == assignment.len() {
        if !expr.evaluate(assignment)? {
            return Ok(());
        }

        let mut nodes = Vec::new();
        let mut cost = 0;
        for (variable, selected) in assignment.iter().copied().enumerate() {
            if selected {
                nodes.push(variables[variable].clone());
                cost += then_costs[variable];
            } else {
                cost += else_cost;
            }
        }

        if best.as_ref().is_none_or(|current| cost < current.cost) {
            *best = Some(Selection { nodes, cost });
        }
        return Ok(());
    }

    assignment[index] = false;
    search_selection(
        index + 1,
        expr,
        variables,
        then_costs,
        else_cost,
        assignment,
        best,
    )?;

    assignment[index] = true;
    search_selection(
        index + 1,
        expr,
        variables,
        then_costs,
        else_cost,
        assignment,
        best,
    )?;
    assignment[index] = false;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionEvaluation {
    pub found_good_set: bool,
    pub negative_improvement: bool,
    pub check_failed: bool,
    pub improvement: f64,
    pub desired: f64,
}

pub fn evaluate_selection_effect(
    model: DelayModel,
    req_times_set: bool,
    threshold: f64,
    old_value: f64,
    new_value: f64,
) -> SelectionEvaluation {
    if model == DelayModel::Unit {
        return SelectionEvaluation {
            found_good_set: true,
            negative_improvement: false,
            check_failed: false,
            improvement: threshold,
            desired: old_value,
        };
    }

    if req_times_set {
        let desired = old_value + threshold * ACCEPTABLE_FRACTION;
        SelectionEvaluation {
            found_good_set: new_value >= desired,
            negative_improvement: new_value < old_value,
            check_failed: new_value < desired,
            improvement: new_value - old_value,
            desired,
        }
    } else {
        let desired = old_value - threshold * ACCEPTABLE_FRACTION;
        SelectionEvaluation {
            found_good_set: new_value <= desired,
            negative_improvement: new_value > old_value,
            check_failed: new_value > desired,
            improvement: old_value - new_value,
            desired,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "actual {actual} != expected {expected}"
        );
    }

    #[test]
    fn constants_match_c_file_and_speed_header() {
        assert_eq!(NSP_BDD_LIMIT, 100_000);
        assert_eq!(ACCEPTABLE_FRACTION, 0.2);
        assert_eq!(MIN_ACCEPTABLE_SAVING, 0.01);
        assert_eq!(NSP_EPSILON, 1.0e-6);
        assert_eq!(CLP, 0);
        assert_eq!(FAN, 1);
        assert_eq!(DUAL, 2);
        assert_eq!(TRANSFORM_BASED, 1);
        assert_eq!(BEST_BENEFIT, 0);
        assert_eq!(BEST_BANG_FOR_BUCK, 1);
    }

    #[test]
    fn best_transform_prefers_improvement_or_bang_for_buck_like_c() {
        let candidates = [
            TransformCandidate::new(0, TransformType::Collapse, true, true, 2.0, 10.0),
            TransformCandidate::new(1, TransformType::Fanout, true, true, 3.0, 30.0),
            TransformCandidate::new(2, TransformType::Dual, false, true, 100.0, 1.0),
        ];

        assert_eq!(
            select_best_transform(&candidates, TransformSelectionMode::BestBenefit).unwrap(),
            BestTransform {
                index: Some(1),
                improvement: 3.0,
                area_cost: 30.0,
                retained_network: Some(TransformType::Fanout),
            }
        );
        assert_eq!(
            select_best_transform(&candidates, TransformSelectionMode::BestBangForBuck).unwrap(),
            BestTransform {
                index: Some(0),
                improvement: 2.0,
                area_cost: 10.0,
                retained_network: Some(TransformType::Collapse),
            }
        );
    }

    #[test]
    fn best_transform_uses_area_tie_break_and_rejects_no_improvement() {
        let tie = [
            TransformCandidate::new(0, TransformType::Collapse, true, true, 2.0, 5.0),
            TransformCandidate::new(
                1,
                TransformType::Fanout,
                true,
                true,
                2.0 + NSP_EPSILON / 2.0,
                2.0,
            ),
        ];
        assert_eq!(
            select_best_transform(&tie, TransformSelectionMode::BestBenefit)
                .unwrap()
                .index,
            Some(1)
        );

        let bad = [TransformCandidate::new(
            0,
            TransformType::Collapse,
            true,
            true,
            0.0,
            1.0,
        )];
        assert_eq!(
            select_best_transform(&bad, TransformSelectionMode::BestBenefit).unwrap(),
            BestTransform {
                index: None,
                improvement: NEG_LARGE,
                area_cost: POS_LARGE,
                retained_network: None,
            }
        );
    }

    #[test]
    fn improvement_and_area_formulas_match_c_branches() {
        let fan = fanout_improvement(FanoutImprovementInput {
            new_required: DelayTime::new(10.0, 12.0),
            old_required: DelayTime::new(7.0, 8.0),
            drive: DelayTime::new(0.5, 1.0),
            load_diff: 2.0,
        });
        assert_eq!(fan, 2.0);

        let collapsed = collapse_improvement(CollapseImprovementInput {
            original_arrival: DelayTime::new(20.0, 30.0),
            optimized_output_arrival: DelayTime::new(17.0, 25.0),
            inversion_delay_saving: Some(DelayTime::new(1.0, 2.0)),
            trivial_mapped_network: false,
        });
        assert_eq!(collapsed, 4.0);
        assert_eq!(
            collapse_improvement(CollapseImprovementInput {
                original_arrival: DelayTime::new(20.0, 30.0),
                optimized_output_arrival: DelayTime::new(17.0, 25.0),
                inversion_delay_saving: None,
                trivial_mapped_network: true,
            }),
            -1.0
        );
        assert_eq!(
            transform_area_cost(TransformType::Fanout, 4.0, 9.0, 2.0),
            4.0
        );
        assert_eq!(
            transform_area_cost(TransformType::Collapse, 4.0, 9.0, 2.0),
            11.0
        );
    }

    #[test]
    fn achievable_saving_propagates_input_epsilon_offsets() {
        let mut nodes = vec![
            WeightedNode::new("a", vec![], 1.5),
            WeightedNode::new("b", vec![], 2.0),
            WeightedNode::new(
                "c",
                vec![
                    CriticalFanin::new("a", DelayTime::new(4.0, 4.0), DelayTime::new(3.0, 3.0)),
                    CriticalFanin::new("b", DelayTime::new(4.5, 4.5), DelayTime::new(3.0, 3.0)),
                ],
                0.7,
            ),
        ];

        compute_achievable_savings(&mut nodes, 2.0).unwrap();

        assert_eq!(nodes[0].epsilon, 1.5);
        assert!(nodes[0].select_flag);
        assert_eq!(nodes[1].epsilon, 2.0);
        assert!(nodes[1].select_flag);
        assert_eq!(nodes[2].epsilon, 1.5);
        assert!(!nodes[2].select_flag);
    }

    #[test]
    fn guaranteed_saving_counts_new_critical_outputs() {
        let outputs = [
            OutputSlack::new("po1", DelayTime::new(-2.0, -1.0), true, Some("n1")),
            OutputSlack::new("po2", DelayTime::new(-1.4, 0.0), false, Some("n2")),
            OutputSlack::new("po3", DelayTime::new(5.0, 5.0), false, None),
        ];
        let eps = HashMap::from([("n1", 1.0)]);

        let saving = compute_guaranteed_saving(&outputs, &eps);

        assert_eq!(saving.epsilon, 1.0);
        assert_eq!(saving.new_critical_slack, -1.0);
        assert_eq!(saving.critical_output_count, 2);
        assert_eq!(saving.newly_critical_outputs, vec!["po2"]);
    }

    #[test]
    fn slack_diff_and_critical_edge_match_c_thresholds() {
        assert_eq!(first_slack_diff(&[DelayTime::new(1.0, 2.0)]), None);
        assert_eq!(
            first_slack_diff(&[
                DelayTime::new(0.0, 0.0),
                DelayTime::new(2.0, 3.0),
                DelayTime::new(5.0, 1.0),
            ]),
            Some(1.0)
        );
        assert!(critical_edge(
            Some(DelayTime::new(5.0, 6.0)),
            DelayTime::new(4.5, 7.0),
            0.0,
        ));
        assert!(!critical_edge(None, DelayTime::new(0.0, 0.0), 0.0));
    }

    #[test]
    fn trivial_mapped_network_accepts_only_empty_or_single_fanin_internals() {
        assert!(trivial_mapped_network(&[]));
        assert!(trivial_mapped_network(&[
            NetworkNodeShape {
                kind: NodeKind::Internal,
                fanin_count: 1,
            },
            NetworkNodeShape {
                kind: NodeKind::PrimaryOutput,
                fanin_count: 1,
            },
        ]));
        assert!(!trivial_mapped_network(&[NetworkNodeShape {
            kind: NodeKind::Internal,
            fanin_count: 2,
        }]));
    }

    #[test]
    fn selection_costs_match_c_objective_formulas() {
        let weighted = [
            SelectionCost {
                node: "a",
                variable: 0,
                area_cost: 1.2,
            },
            SelectionCost {
                node: "b",
                variable: 2,
                area_cost: 4.0,
            },
        ];

        assert_eq!(
            selection_costs(3, &weighted, SelectionObjective::TransformBased).unwrap(),
            CostArray {
                then_costs: vec![14, POS_LARGE as i32, 16],
                max_area_cost: 4,
            }
        );
        assert_eq!(
            selection_costs(3, &weighted, SelectionObjective::AreaBased).unwrap(),
            CostArray {
                then_costs: vec![7, POS_LARGE as i32, 13],
                max_area_cost: 4,
            }
        );
    }

    #[test]
    fn best_selection_finds_min_cost_satisfying_assignment() {
        let expr = SelectionExpr::and(
            SelectionExpr::or(SelectionExpr::var(0), SelectionExpr::var(1)),
            SelectionExpr::or(SelectionExpr::var(1), SelectionExpr::var(2)),
        );
        let selection = best_selection(&expr, &["a", "b", "c"], &[5, 2, 3], 0)
            .unwrap()
            .unwrap();

        assert_eq!(
            selection,
            Selection {
                nodes: vec!["b"],
                cost: 2,
            }
        );
        assert_eq!(
            best_selection::<&str>(&SelectionExpr::Zero, &[], &[], 0).unwrap(),
            None
        );
        assert_eq!(
            best_selection(&SelectionExpr::One, &["a"], &[3], 0).unwrap(),
            Some(Selection {
                nodes: vec![],
                cost: 0,
            })
        );
    }

    #[test]
    fn selection_effect_uses_slack_or_delay_direction() {
        let slack = evaluate_selection_effect(DelayModel::Mapped, true, 10.0, 1.0, 4.0);
        assert_eq!(
            slack,
            SelectionEvaluation {
                found_good_set: true,
                negative_improvement: false,
                check_failed: false,
                improvement: 3.0,
                desired: 3.0,
            }
        );

        let delay = evaluate_selection_effect(DelayModel::Mapped, false, 10.0, 20.0, 19.0);
        assert_eq!(
            delay,
            SelectionEvaluation {
                found_good_set: false,
                negative_improvement: false,
                check_failed: true,
                improvement: 1.0,
                desired: 18.0,
            }
        );

        let unit = evaluate_selection_effect(DelayModel::Unit, false, 5.0, 20.0, 99.0);
        assert!(unit.found_good_set);
        assert!(!unit.negative_improvement);
    }

    #[test]
    fn sis_backed_entry_points_report_explicit_dependencies() {
        assert_eq!(
            compute_weight_from_sis_network(),
            Err(NewWeightError::SisGraphDependency {
                operation: "new_speed_compute_weight",
                source: "LogicSynthesis/sis/speed/new_wght_util.c:38",
            })
        );
        assert_eq!(
            select_xform_from_sis_network(),
            Err(NewWeightError::SisGraphDependency {
                operation: "new_speed_select_xform",
                source: "LogicSynthesis/sis/speed/new_wght_util.c:1038",
            })
        );
    }

    #[test]
    fn sp_macros_and_zero_predicate_match_c() {
        assert_eq!(sp_improvement(None, &[1.0]), NEG_LARGE);
        assert_eq!(sp_cost(None, &[1.0]), POS_LARGE);
        assert_eq!(sp_improvement(Some(1), &[1.0, 2.0]), 2.0);
        assert_eq!(sp_cost(Some(0), &[3.0]), 3.0);
        assert!(is_zero(NSP_EPSILON / 2.0));
        assert!(!is_zero(NSP_EPSILON * 2.0));
        assert_close(ACCEPTABLE_FRACTION, 0.2);
    }
}
