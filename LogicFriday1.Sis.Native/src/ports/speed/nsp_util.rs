//! Native Rust port scaffold for `sis/speed/nsp_util.c`.
//!
//! The C unit combines small timing/name/selection rules with large routines
//! that mutate SIS `network_t` and `node_t` graphs. This module ports the
//! rules that can be expressed over owned Rust data and reports explicit
//! graph API errors for behavior still blocked on native SIS graph ports.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

pub const LIMIT_NUM_CUBES: usize = 200;
pub const NSP_INPUT_SEPARATOR: char = '#';
pub const NSP_OUTPUT_SEPARATOR: char = '%';
pub const POS_LARGE: f64 = 10_000.0;
pub const NEG_LARGE: f64 = -10_000.0;
pub const V_SMALL: f64 = 1.0e-6;
pub const NSP_EPSILON: f64 = 1.0e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub const fn zero() -> Self {
        Self {
            rise: 0.0,
            fall: 0.0,
        }
    }

    pub const fn pos_large() -> Self {
        Self {
            rise: POS_LARGE,
            fall: POS_LARGE,
        }
    }

    pub const fn neg_large() -> Self {
        Self {
            rise: NEG_LARGE,
            fall: NEG_LARGE,
        }
    }

    pub fn min_edge(self) -> f64 {
        self.rise.min(self.fall)
    }

    pub fn max_edge(self) -> f64 {
        self.rise.max(self.fall)
    }

    pub fn edge_min(self, rhs: Self) -> Self {
        Self {
            rise: self.rise.min(rhs.rise),
            fall: self.fall.min(rhs.fall),
        }
    }

    pub fn edge_max(self, rhs: Self) -> Self {
        Self {
            rise: self.rise.max(rhs.rise),
            fall: self.fall.max(rhs.fall),
        }
    }

    pub fn drive_adjusted(self, drive: Self, load: f64) -> Self {
        Self {
            rise: self.rise - drive.rise * load,
            fall: self.fall - drive.fall * load,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    UnitFanout,
    Library,
    Mapped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputDelayData {
    pub arrival: DelayTime,
    pub required: DelayTime,
    pub slack: DelayTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayDataSummary {
    pub output_count: usize,
    pub total_gate_area: f64,
    pub maximum_arrival: DelayTime,
    pub maximum_output_slack: DelayTime,
    pub minimum_output_slack: DelayTime,
    pub total_negative_slack: DelayTime,
    pub failing_outputs: usize,
}

pub fn summarize_delay_data(outputs: &[OutputDelayData], total_gate_area: f64) -> DelayDataSummary {
    let mut summary = DelayDataSummary {
        output_count: outputs.len(),
        total_gate_area,
        maximum_arrival: DelayTime::neg_large(),
        maximum_output_slack: DelayTime::neg_large(),
        minimum_output_slack: DelayTime::pos_large(),
        total_negative_slack: DelayTime::zero(),
        failing_outputs: 0,
    };

    for output in outputs {
        if output.slack.min_edge() < 0.0 {
            summary.failing_outputs += 1;
            if output.slack.rise < 0.0 {
                summary.total_negative_slack.rise += output.slack.rise;
            }
            if output.slack.fall < 0.0 {
                summary.total_negative_slack.fall += output.slack.fall;
            }
        }

        summary.maximum_arrival = summary.maximum_arrival.edge_max(output.arrival);
        summary.maximum_output_slack = summary.maximum_output_slack.edge_max(output.slack);
        summary.minimum_output_slack = summary.minimum_output_slack.edge_min(output.slack);
    }

    summary
}

pub fn format_delay_data_summary(summary: &DelayDataSummary) -> String {
    format!(
        "# of outputs:          {}\n\
total gate area:       {:.2}\n\
maximum arrival time: ({:.2},{:.2})\n\
maximum po slack:     ({:.2},{:.2})\n\
minimum po slack:     ({:.2},{:.2})\n\
total neg slack:      ({:.2},{:.2})\n\
# of failing outputs:  {}\n",
        summary.output_count,
        summary.total_gate_area,
        summary.maximum_arrival.rise,
        summary.maximum_arrival.fall,
        summary.maximum_output_slack.rise,
        summary.maximum_output_slack.fall,
        summary.minimum_output_slack.rise,
        summary.minimum_output_slack.fall,
        summary.total_negative_slack.rise,
        summary.total_negative_slack.fall,
        summary.failing_outputs
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelaySavingInput {
    pub node_pin_delay: DelayTime,
    pub node_input_load: f64,
    pub fanin_drive: DelayTime,
    pub primary_output_load: f64,
}

pub fn compute_delay_saving(input: DelaySavingInput) -> DelayTime {
    let load_delta = input.primary_output_load - input.node_input_load;
    DelayTime {
        rise: input.node_pin_delay.rise - input.fanin_drive.rise * load_delta,
        fall: input.node_pin_delay.fall - input.fanin_drive.fall * load_delta,
    }
}

pub fn adjusted_po_arrival(
    arrival: DelayTime,
    model: DelayModel,
    can_adjust: bool,
    root_fanin_count: usize,
    saving: DelayTime,
) -> DelayTime {
    if model == DelayModel::Mapped || !can_adjust || root_fanin_count != 1 {
        return arrival;
    }

    DelayTime {
        rise: arrival.rise - saving.rise,
        fall: arrival.fall - saving.fall,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformType {
    Collapse,
    Fanout,
    Dual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimizeKind {
    NoAlgorithm,
    Repower,
    Fanout,
    Duplicate,
    AndOr,
    Divisor,
    TwoCubeKernel,
    ComplementDivisor,
    ComplementTwoCube,
    Cofactor,
    Bypass,
    Dualize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArrivalKind {
    Arrival,
    Required,
    Slack,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalTransform {
    pub name: &'static str,
    pub optimize_kind: OptimizeKind,
    pub arrival_kind: ArrivalKind,
    pub priority: i32,
    pub on: bool,
    pub transform_type: TransformType,
}

pub const DEFAULT_LOCAL_TRANSFORMS: &[LocalTransform] = &[
    LocalTransform {
        name: "noalg",
        optimize_kind: OptimizeKind::NoAlgorithm,
        arrival_kind: ArrivalKind::Arrival,
        priority: 0,
        on: false,
        transform_type: TransformType::Collapse,
    },
    LocalTransform {
        name: "repower",
        optimize_kind: OptimizeKind::Repower,
        arrival_kind: ArrivalKind::Required,
        priority: 0,
        on: false,
        transform_type: TransformType::Fanout,
    },
    LocalTransform {
        name: "fanout",
        optimize_kind: OptimizeKind::Fanout,
        arrival_kind: ArrivalKind::Required,
        priority: 1,
        on: false,
        transform_type: TransformType::Fanout,
    },
    LocalTransform {
        name: "duplicate",
        optimize_kind: OptimizeKind::Duplicate,
        arrival_kind: ArrivalKind::Required,
        priority: 1,
        on: false,
        transform_type: TransformType::Fanout,
    },
    LocalTransform {
        name: "and_or",
        optimize_kind: OptimizeKind::AndOr,
        arrival_kind: ArrivalKind::Arrival,
        priority: 2,
        on: false,
        transform_type: TransformType::Collapse,
    },
    LocalTransform {
        name: "divisor",
        optimize_kind: OptimizeKind::Divisor,
        arrival_kind: ArrivalKind::Arrival,
        priority: 2,
        on: true,
        transform_type: TransformType::Collapse,
    },
    LocalTransform {
        name: "2c_kernel",
        optimize_kind: OptimizeKind::TwoCubeKernel,
        arrival_kind: ArrivalKind::Arrival,
        priority: 2,
        on: false,
        transform_type: TransformType::Collapse,
    },
    LocalTransform {
        name: "comp_div",
        optimize_kind: OptimizeKind::ComplementDivisor,
        arrival_kind: ArrivalKind::Arrival,
        priority: 2,
        on: false,
        transform_type: TransformType::Collapse,
    },
    LocalTransform {
        name: "comp_2c",
        optimize_kind: OptimizeKind::ComplementTwoCube,
        arrival_kind: ArrivalKind::Arrival,
        priority: 2,
        on: false,
        transform_type: TransformType::Collapse,
    },
    LocalTransform {
        name: "cofactor",
        optimize_kind: OptimizeKind::Cofactor,
        arrival_kind: ArrivalKind::Arrival,
        priority: 2,
        on: false,
        transform_type: TransformType::Collapse,
    },
    LocalTransform {
        name: "bypass",
        optimize_kind: OptimizeKind::Bypass,
        arrival_kind: ArrivalKind::Arrival,
        priority: 2,
        on: false,
        transform_type: TransformType::Collapse,
    },
    LocalTransform {
        name: "dualize",
        optimize_kind: OptimizeKind::Dualize,
        arrival_kind: ArrivalKind::Slack,
        priority: 2,
        on: false,
        transform_type: TransformType::Dual,
    },
];

pub fn local_transforms(entries: &[&str]) -> Vec<LocalTransform> {
    let mut transforms = DEFAULT_LOCAL_TRANSFORMS.to_vec();
    if entries.is_empty() {
        return transforms;
    }

    let selected = entries.iter().copied().collect::<HashSet<_>>();
    for transform in &mut transforms {
        transform.on = selected.contains(transform.name);
    }
    transforms
}

pub fn active_local_transform_count(transforms: &[LocalTransform]) -> usize {
    transforms.iter().filter(|transform| transform.on).count()
}

pub fn active_local_transform_count_of_type(
    transforms: &[LocalTransform],
    transform_type: TransformType,
) -> usize {
    transforms
        .iter()
        .filter(|transform| transform.on && transform.transform_type == transform_type)
        .count()
}

pub fn local_transform_from_index(
    transforms: &[LocalTransform],
    index: usize,
) -> Option<&LocalTransform> {
    transforms.get(index)
}

pub fn active_transform_names(transforms: &[LocalTransform]) -> Vec<&'static str> {
    transforms
        .iter()
        .filter(|transform| transform.on)
        .map(|transform| transform.name)
        .collect()
}

pub fn inactive_transform_names(transforms: &[LocalTransform]) -> Vec<&'static str> {
    transforms
        .iter()
        .filter(|transform| !transform.on)
        .map(|transform| transform.name)
        .collect()
}

pub fn format_local_transforms(transforms: &[LocalTransform]) -> String {
    let in_use = active_transform_names(transforms)
        .into_iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(" ");
    let not_in_use = inactive_transform_names(transforms)
        .into_iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(" ");

    format!("Tech-Indep methods in use: {in_use}\nTech-Indep methods not in use: {not_in_use}\n")
}

pub fn is_fanout_primary_output(fanout_kinds: &[NodeKind]) -> bool {
    fanout_kinds
        .iter()
        .any(|kind| *kind == NodeKind::PrimaryOutput)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginalEdgeName<'a> {
    pub fanout_name: &'a str,
    pub fanin_index: i32,
}

pub fn parse_original_edge_name(name: &str) -> Result<OriginalEdgeName<'_>, NspUtilError> {
    let Some((fanout_name, index)) = name.rsplit_once(NSP_OUTPUT_SEPARATOR) else {
        return Err(NspUtilError::MissingOutputSeparator(name.to_string()));
    };
    let fanin_index = index
        .parse::<i32>()
        .map_err(|_| NspUtilError::InvalidEdgeIndex(index.to_string()))?;

    Ok(OriginalEdgeName {
        fanout_name,
        fanin_index,
    })
}

pub fn base_network_name(name: &str) -> &str {
    let input_pos = name.rfind(NSP_INPUT_SEPARATOR);
    let output_pos = name.rfind(NSP_OUTPUT_SEPARATOR);
    match (input_pos, output_pos) {
        (Some(left), Some(right)) => &name[..left.max(right)],
        (Some(pos), None) | (None, Some(pos)) => &name[..pos],
        (None, None) => name,
    }
}

pub fn network_find_name<'a, T>(
    nodes_by_name: &'a HashMap<String, T>,
    name: &str,
) -> Option<&'a T> {
    nodes_by_name
        .get(name)
        .or_else(|| nodes_by_name.get(base_network_name(name)))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutRef<N> {
    pub fanout: N,
    pub pin: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatchFanoutsPlan<N> {
    pub patches: Vec<(N, N)>,
}

pub fn patch_fanouts_plan<N: Clone>(fanouts: &[N], new_fanin: N) -> PatchFanoutsPlan<N> {
    PatchFanoutsPlan {
        patches: fanouts
            .iter()
            .rev()
            .cloned()
            .map(|fanout| (fanout, new_fanin.clone()))
            .collect(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutTiming<N> {
    pub fanout: N,
    pub pin: usize,
    pub load: f64,
    pub required: DelayTime,
}

pub fn fanout_compare<N>(left: &FanoutTiming<N>, right: &FanoutTiming<N>) -> Ordering {
    if req_equal(left.required, right.required) {
        Ordering::Equal
    } else {
        left.required
            .min_edge()
            .total_cmp(&right.required.min_edge())
    }
}

pub fn sorted_fanouts_by_criticality<N: Clone>(
    fanouts: &[FanoutTiming<N>],
) -> Vec<FanoutTiming<N>> {
    let mut sorted = fanouts.to_vec();
    sorted.sort_by(fanout_compare);
    sorted
}

pub fn req_equal(left: DelayTime, right: DelayTime) -> bool {
    (left.rise - right.rise).abs() < V_SMALL && (left.fall - right.fall).abs() < V_SMALL
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RequiredInputTiming {
    pub wire_required: DelayTime,
    pub drive: DelayTime,
    pub load: f64,
}

pub fn adjusted_required_input_time(input: RequiredInputTiming) -> DelayTime {
    input.wire_required.drive_adjusted(input.drive, input.load)
}

pub fn min_required_input_time(inputs: &[RequiredInputTiming]) -> DelayTime {
    inputs
        .iter()
        .map(|input| adjusted_required_input_time(*input))
        .fold(DelayTime::pos_large(), DelayTime::edge_min)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayValues {
    pub arrival: DelayTime,
    pub required: DelayTime,
    pub slack: DelayTime,
}

pub fn new_delay_arrival(values: DelayValues) -> DelayTime {
    values.arrival
}

pub fn new_delay_required(values: DelayValues) -> DelayTime {
    values.required
}

pub fn new_delay_slack(values: DelayValues) -> DelayTime {
    values.slack
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadLimitInput {
    pub current_pin_load: f64,
    pub fanin_fanout_count: usize,
    pub fanin_pin_drives: Vec<DelayTime>,
    pub fanin_arrival: DelayTime,
    pub original_wire_required: DelayTime,
    pub sibling_wire_required: Vec<DelayTime>,
}

pub fn load_limit(input: &LoadLimitInput) -> Result<f64, NspUtilError> {
    if input.fanin_fanout_count == 1 {
        return Ok(input.current_pin_load * POS_LARGE);
    }
    if input.fanin_pin_drives.is_empty() {
        return Err(NspUtilError::MissingFaninPinDrive);
    }

    let drive = input
        .fanin_pin_drives
        .iter()
        .copied()
        .fold(DelayTime::neg_large(), DelayTime::edge_max);
    let original_slack = DelayTime::new(
        input.original_wire_required.rise - input.fanin_arrival.rise,
        input.original_wire_required.fall - input.fanin_arrival.fall,
    );
    let minimum_fanout_slack = input
        .sibling_wire_required
        .iter()
        .map(|required| {
            DelayTime::new(
                required.rise - input.fanin_arrival.rise,
                required.fall - input.fanin_arrival.fall,
            )
        })
        .fold(DelayTime::pos_large(), DelayTime::edge_min);

    let diff = minimum_fanout_slack.min_edge() - original_slack.min_edge();
    let mut load = input.current_pin_load;
    if diff > 0.0 {
        load += diff / drive.max_edge();
    } else {
        load += NSP_EPSILON;
    }

    Ok(load)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SplitFanoutsAction {
    SkipUnitDelayModel,
    SkipInsufficientFanouts,
    EvaluateDuplication,
}

pub fn split_fanouts_action(model: DelayModel, root_fanout_count: usize) -> SplitFanoutsAction {
    if model == DelayModel::Unit {
        SplitFanoutsAction::SkipUnitDelayModel
    } else if root_fanout_count <= 2 {
        SplitFanoutsAction::SkipInsufficientFanouts
    } else {
        SplitFanoutsAction::EvaluateDuplication
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitFanoutsPlan {
    pub action: SplitFanoutsAction,
    pub best_index: Option<usize>,
    pub duplicate_root: bool,
    pub fanouts_moved_to_duplicate: Vec<usize>,
}

pub fn plan_split_fanouts(
    model: DelayModel,
    root_fanout_count: usize,
    initial_required_time: DelayTime,
    incremental_required_times: &[DelayTime],
) -> SplitFanoutsPlan {
    let action = split_fanouts_action(model, root_fanout_count);
    if action != SplitFanoutsAction::EvaluateDuplication || incremental_required_times.is_empty() {
        return SplitFanoutsPlan {
            action,
            best_index: None,
            duplicate_root: false,
            fanouts_moved_to_duplicate: Vec::new(),
        };
    }

    let mut best = initial_required_time;
    let mut best_index = None;
    for (index, required) in incremental_required_times.iter().copied().enumerate() {
        if req_improved(best, required) {
            best = required;
            best_index = Some(index);
        }
    }

    if best_index == Some(incremental_required_times.len() - 1) {
        best_index = None;
    }

    let fanouts_moved_to_duplicate = best_index
        .map(|index| (0..=index).collect())
        .unwrap_or_default();

    SplitFanoutsPlan {
        action,
        best_index,
        duplicate_root: best_index.is_some(),
        fanouts_moved_to_duplicate,
    }
}

pub fn req_improved(old: DelayTime, new: DelayTime) -> bool {
    new.rise - old.rise > V_SMALL && new.fall - old.fall > V_SMALL
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimizationKind {
    NoAlgorithm,
    Divisor,
    TwoCubeKernel,
    ComplementDivisor,
    ComplementTwoCube,
    AndOr,
    Fanout,
    Duplicate,
    Dual,
    Cofactor,
    Bypass,
}

pub fn cube_count_allows_decomposition(cube_count: usize) -> bool {
    cube_count < LIMIT_NUM_CUBES
}

pub fn optimization_plan(
    kind: OptimizationKind,
    input_exists: bool,
    mapped_network: bool,
    cube_count: Option<usize>,
) -> Result<OptimizationPlan, NspUtilError> {
    if !input_exists {
        return Ok(OptimizationPlan {
            duplicate_first: false,
            operation: PlannedOperation::NoNetwork,
            remap_if_required: false,
            trace_after: false,
        });
    }

    let remap_if_required = matches!(
        kind,
        OptimizationKind::NoAlgorithm
            | OptimizationKind::Divisor
            | OptimizationKind::TwoCubeKernel
            | OptimizationKind::ComplementDivisor
            | OptimizationKind::ComplementTwoCube
            | OptimizationKind::AndOr
            | OptimizationKind::Cofactor
            | OptimizationKind::Bypass
    );
    let duplicate_first = true;
    let trace_after = true;

    match kind {
        OptimizationKind::NoAlgorithm => Ok(OptimizationPlan {
            duplicate_first,
            operation: PlannedOperation::None,
            remap_if_required,
            trace_after,
        }),
        OptimizationKind::Fanout => Ok(OptimizationPlan {
            duplicate_first,
            operation: if mapped_network {
                PlannedOperation::BufferFanout { mode: 7 }
            } else {
                PlannedOperation::None
            },
            remap_if_required: false,
            trace_after,
        }),
        OptimizationKind::Duplicate => Ok(OptimizationPlan {
            duplicate_first,
            operation: PlannedOperation::SplitFanouts,
            remap_if_required: false,
            trace_after,
        }),
        OptimizationKind::Dual => Ok(OptimizationPlan {
            duplicate_first,
            operation: PlannedOperation::DualNotImplemented,
            remap_if_required: false,
            trace_after,
        }),
        other => {
            let cube_count = cube_count.ok_or(NspUtilError::MissingCubeCount)?;
            let operation = if cube_count_allows_decomposition(cube_count) {
                match other {
                    OptimizationKind::Divisor => PlannedOperation::TimingDivisor,
                    OptimizationKind::TwoCubeKernel => PlannedOperation::TwoCubeKernel,
                    OptimizationKind::ComplementDivisor => {
                        PlannedOperation::ComplementTimingDivisor
                    }
                    OptimizationKind::ComplementTwoCube => {
                        PlannedOperation::ComplementTwoCubeKernel
                    }
                    OptimizationKind::AndOr => PlannedOperation::AndOr,
                    OptimizationKind::Cofactor => PlannedOperation::Cofactor,
                    OptimizationKind::Bypass => PlannedOperation::Bypass,
                    _ => PlannedOperation::None,
                }
            } else {
                PlannedOperation::UseOriginalCopy
            };

            Ok(OptimizationPlan {
                duplicate_first,
                operation,
                remap_if_required,
                trace_after,
            })
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationPlan {
    pub duplicate_first: bool,
    pub operation: PlannedOperation,
    pub remap_if_required: bool,
    pub trace_after: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannedOperation {
    NoNetwork,
    None,
    TimingDivisor,
    TwoCubeKernel,
    ComplementTimingDivisor,
    ComplementTwoCubeKernel,
    AndOr,
    BufferFanout { mode: u8 },
    SplitFanouts,
    DualNotImplemented,
    Cofactor,
    Bypass,
    UseOriginalCopy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferParameters {
    pub limit: usize,
    pub trace: bool,
    pub threshold: f64,
    pub single_pass: bool,
    pub do_decomp: bool,
    pub use_mapped: bool,
}

pub fn init_buffer_parameters(model: DelayModel) -> BufferParameters {
    BufferParameters {
        limit: 2,
        trace: false,
        threshold: 0.5,
        single_pass: true,
        do_decomp: false,
        use_mapped: matches!(model, DelayModel::Mapped | DelayModel::Library),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredRequiredTimes<N> {
    pub generated_required_times: Vec<N>,
}

pub fn store_required_times<N: Clone>(
    outputs: &mut [(N, Option<DelayTime>, DelayTime)],
) -> StoredRequiredTimes<N> {
    let mut generated_required_times = Vec::new();
    for (node, user_required, computed_required) in outputs {
        if user_required.is_none() {
            *user_required = Some(*computed_required);
            generated_required_times.push(node.clone());
        }
    }

    StoredRequiredTimes {
        generated_required_times,
    }
}

pub fn restore_required_times<N>(outputs: &mut HashMap<N, Option<DelayTime>>, stored: &[N])
where
    N: Eq + Hash,
{
    for node in stored {
        if let Some(required) = outputs.get_mut(node) {
            *required = None;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NspUtilError {
    SisGraphDependency {
        operation: &'static str,
        source: &'static str,
    },
    MissingOutputSeparator(String),
    InvalidEdgeIndex(String),
    MissingCubeCount,
    MissingFaninPinDrive,
}

impl fmt::Display for NspUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SisGraphDependency { operation, source } => {
                write!(
                    f,
                    "{operation} requires SIS graph orchestration from {source}"
                )
            }
            Self::MissingOutputSeparator(name) => {
                write!(
                    f,
                    "edge name {name} does not contain '{NSP_OUTPUT_SEPARATOR}'"
                )
            }
            Self::InvalidEdgeIndex(index) => write!(f, "invalid edge fanin index {index}"),
            Self::MissingCubeCount => write!(f, "optimization plan requires collapsed cube count"),
            Self::MissingFaninPinDrive => write!(f, "load limit requires at least one fanin drive"),
        }
    }
}

impl Error for NspUtilError {}

pub fn create_collapse_record_from_sis_network() -> Result<(), NspUtilError> {
    Err(NspUtilError::SisGraphDependency {
        operation: "sp_create_collapse_record",
        source: "LogicSynthesis/sis/speed/nsp_util.c:228",
    })
}

pub fn delete_network_from_sis_network() -> Result<(), NspUtilError> {
    Err(NspUtilError::SisGraphDependency {
        operation: "sp_delete_network",
        source: "LogicSynthesis/sis/speed/nsp_util.c:576",
    })
}

pub fn append_network_to_sis_network() -> Result<(), NspUtilError> {
    Err(NspUtilError::SisGraphDependency {
        operation: "sp_append_network",
        source: "LogicSynthesis/sis/speed/nsp_util.c:798",
    })
}

pub fn optimize_sis_network() -> Result<(), NspUtilError> {
    Err(NspUtilError::SisGraphDependency {
        operation: "sp_*_opt",
        source: "LogicSynthesis/sis/speed/nsp_util.c:1395",
    })
}

pub fn duplicate_sis_network() -> Result<(), NspUtilError> {
    Err(NspUtilError::SisGraphDependency {
        operation: "speed_network_dup",
        source: "LogicSynthesis/sis/speed/nsp_util.c:1989",
    })
}

pub fn downsize_non_critical_sis_gates() -> Result<(), NspUtilError> {
    Err(NspUtilError::SisGraphDependency {
        operation: "nsp_downsize_non_crit_gates",
        source: "LogicSynthesis/sis/speed/nsp_util.c:1735",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "actual {} != expected {}",
            actual,
            expected
        );
    }

    #[test]
    fn delay_summary_matches_c_accumulators() {
        let outputs = [
            OutputDelayData {
                arrival: DelayTime::new(4.0, 3.0),
                required: DelayTime::new(5.0, 6.0),
                slack: DelayTime::new(1.0, -0.5),
            },
            OutputDelayData {
                arrival: DelayTime::new(2.0, 7.0),
                required: DelayTime::new(3.0, 4.0),
                slack: DelayTime::new(-1.5, -2.0),
            },
            OutputDelayData {
                arrival: DelayTime::new(5.0, 6.0),
                required: DelayTime::new(7.0, 8.0),
                slack: DelayTime::new(2.0, 3.0),
            },
        ];

        assert_eq!(
            summarize_delay_data(&outputs, 12.5),
            DelayDataSummary {
                output_count: 3,
                total_gate_area: 12.5,
                maximum_arrival: DelayTime::new(5.0, 7.0),
                maximum_output_slack: DelayTime::new(2.0, 3.0),
                minimum_output_slack: DelayTime::new(-1.5, -2.0),
                total_negative_slack: DelayTime::new(-1.5, -2.5),
                failing_outputs: 2,
            }
        );
        assert_eq!(
            format_delay_data_summary(&summarize_delay_data(&outputs, 12.5)),
            "# of outputs:          3\n\
total gate area:       12.50\n\
maximum arrival time: (5.00,7.00)\n\
maximum po slack:     (2.00,3.00)\n\
minimum po slack:     (-1.50,-2.00)\n\
total neg slack:      (-1.50,-2.50)\n\
# of failing outputs:  2\n"
        );
    }

    #[test]
    fn delay_saving_and_po_adjustment_follow_c_formula() {
        let saving = compute_delay_saving(DelaySavingInput {
            node_pin_delay: DelayTime::new(5.0, 7.0),
            node_input_load: 2.0,
            fanin_drive: DelayTime::new(0.5, 0.25),
            primary_output_load: 6.0,
        });

        assert_eq!(saving, DelayTime::new(3.0, 6.0));
        assert_eq!(
            adjusted_po_arrival(
                DelayTime::new(10.0, 12.0),
                DelayModel::UnitFanout,
                true,
                1,
                saving
            ),
            DelayTime::new(7.0, 6.0)
        );
        assert_eq!(
            adjusted_po_arrival(
                DelayTime::new(10.0, 12.0),
                DelayModel::Mapped,
                true,
                1,
                saving
            ),
            DelayTime::new(10.0, 12.0)
        );
    }

    #[test]
    fn local_transform_table_matches_new_speed_models_header() {
        let defaults = local_transforms(&[]);
        assert_eq!(defaults.len(), 12);
        assert_eq!(defaults[0].name, "noalg");
        assert_eq!(defaults[0].priority, 0);
        assert_eq!(defaults[1].name, "repower");
        assert_eq!(defaults[1].transform_type, TransformType::Fanout);
        assert_eq!(defaults[5].name, "divisor");
        assert!(defaults[5].on);
        assert_eq!(defaults[11].name, "dualize");
        assert_eq!(defaults[11].arrival_kind, ArrivalKind::Slack);

        let selected = local_transforms(&["fanout", "duplicate", "bypass"]);
        assert_eq!(active_local_transform_count(&selected), 3);
        assert_eq!(
            active_local_transform_count_of_type(&selected, TransformType::Fanout),
            2
        );
        assert_eq!(
            active_transform_names(&selected),
            vec!["fanout", "duplicate", "bypass"]
        );
        assert_eq!(local_transform_from_index(&selected, 20), None);
        assert_eq!(
            format_local_transforms(&selected),
            "Tech-Indep methods in use: \"fanout\" \"duplicate\" \"bypass\"\n\
Tech-Indep methods not in use: \"noalg\" \"repower\" \"and_or\" \"divisor\" \"2c_kernel\" \"comp_div\" \"comp_2c\" \"cofactor\" \"dualize\"\n"
        );
    }

    #[test]
    fn parses_made_up_node_names_without_mutating_input() {
        assert_eq!(
            parse_original_edge_name("fanout%12").unwrap(),
            OriginalEdgeName {
                fanout_name: "fanout",
                fanin_index: 12,
            }
        );
        assert_eq!(base_network_name("node#3"), "node");
        assert_eq!(base_network_name("node%4"), "node");
        assert_eq!(base_network_name("plain"), "plain");
        assert_eq!(
            parse_original_edge_name("bad").unwrap_err(),
            NspUtilError::MissingOutputSeparator("bad".to_string())
        );
        assert_eq!(
            parse_original_edge_name("fanout%bad").unwrap_err(),
            NspUtilError::InvalidEdgeIndex("bad".to_string())
        );
    }

    #[test]
    fn network_lookup_uses_base_name_for_special_inputs_and_outputs() {
        let nodes = HashMap::from([("a".to_string(), 10), ("z".to_string(), 20)]);

        assert_eq!(network_find_name(&nodes, "a"), Some(&10));
        assert_eq!(network_find_name(&nodes, "a#0"), Some(&10));
        assert_eq!(network_find_name(&nodes, "z%1"), Some(&20));
        assert_eq!(network_find_name(&nodes, "missing#1"), None);
    }

    #[test]
    fn fanout_compare_sorts_lowest_min_required_time_first() {
        let fanouts = [
            FanoutTiming {
                fanout: "late",
                pin: 0,
                load: 1.0,
                required: DelayTime::new(5.0, 6.0),
            },
            FanoutTiming {
                fanout: "critical",
                pin: 0,
                load: 1.0,
                required: DelayTime::new(2.0, 9.0),
            },
            FanoutTiming {
                fanout: "middle",
                pin: 0,
                load: 1.0,
                required: DelayTime::new(4.0, 4.5),
            },
        ];

        let names = sorted_fanouts_by_criticality(&fanouts)
            .into_iter()
            .map(|fanout| fanout.fanout)
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["critical", "middle", "late"]);
        assert!(req_equal(
            DelayTime::new(1.0, 2.0),
            DelayTime::new(1.0 + V_SMALL / 2.0, 2.0)
        ));
    }

    #[test]
    fn min_required_input_time_applies_drive_load_adjustment() {
        let best = min_required_input_time(&[
            RequiredInputTiming {
                wire_required: DelayTime::new(10.0, 8.0),
                drive: DelayTime::new(1.0, 2.0),
                load: 2.0,
            },
            RequiredInputTiming {
                wire_required: DelayTime::new(12.0, 9.0),
                drive: DelayTime::new(0.5, 1.0),
                load: 4.0,
            },
        ]);

        assert_eq!(best, DelayTime::new(8.0, 4.0));
    }

    #[test]
    fn delay_accessors_and_fanout_po_detection_are_direct_field_ports() {
        let values = DelayValues {
            arrival: DelayTime::new(1.0, 2.0),
            required: DelayTime::new(3.0, 4.0),
            slack: DelayTime::new(2.0, 2.0),
        };

        assert_eq!(new_delay_arrival(values), DelayTime::new(1.0, 2.0));
        assert_eq!(new_delay_required(values), DelayTime::new(3.0, 4.0));
        assert_eq!(new_delay_slack(values), DelayTime::new(2.0, 2.0));
        assert!(is_fanout_primary_output(&[
            NodeKind::Internal,
            NodeKind::PrimaryOutput
        ]));
        assert!(!is_fanout_primary_output(&[
            NodeKind::PrimaryInput,
            NodeKind::Internal
        ]));
    }

    #[test]
    fn load_limit_matches_c_slack_and_drive_formula() {
        assert_eq!(
            load_limit(&LoadLimitInput {
                current_pin_load: 2.5,
                fanin_fanout_count: 1,
                fanin_pin_drives: Vec::new(),
                fanin_arrival: DelayTime::new(0.0, 0.0),
                original_wire_required: DelayTime::new(0.0, 0.0),
                sibling_wire_required: Vec::new(),
            })
            .unwrap(),
            25_000.0
        );

        let limit = load_limit(&LoadLimitInput {
            current_pin_load: 3.0,
            fanin_fanout_count: 3,
            fanin_pin_drives: vec![DelayTime::new(0.5, 2.0), DelayTime::new(1.5, 1.0)],
            fanin_arrival: DelayTime::new(10.0, 20.0),
            original_wire_required: DelayTime::new(15.0, 24.0),
            sibling_wire_required: vec![DelayTime::new(20.0, 28.0), DelayTime::new(18.0, 27.0)],
        })
        .unwrap();
        assert_close(limit, 4.5);

        let limited = load_limit(&LoadLimitInput {
            current_pin_load: 3.0,
            fanin_fanout_count: 2,
            fanin_pin_drives: vec![DelayTime::new(1.0, 1.0)],
            fanin_arrival: DelayTime::new(10.0, 20.0),
            original_wire_required: DelayTime::new(15.0, 24.0),
            sibling_wire_required: vec![DelayTime::new(11.0, 21.0)],
        })
        .unwrap();
        assert_close(limited, 3.0 + NSP_EPSILON);

        assert_eq!(
            load_limit(&LoadLimitInput {
                current_pin_load: 3.0,
                fanin_fanout_count: 2,
                fanin_pin_drives: Vec::new(),
                fanin_arrival: DelayTime::new(10.0, 20.0),
                original_wire_required: DelayTime::new(15.0, 24.0),
                sibling_wire_required: Vec::new(),
            })
            .unwrap_err(),
            NspUtilError::MissingFaninPinDrive
        );
    }

    #[test]
    fn fanout_patch_plan_preserves_c_reverse_patch_order() {
        assert_eq!(
            patch_fanouts_plan(&["fo1", "fo2", "fo3"], "new"),
            PatchFanoutsPlan {
                patches: vec![("fo3", "new"), ("fo2", "new"), ("fo1", "new")],
            }
        );
    }

    #[test]
    fn split_fanouts_and_optimization_plans_capture_c_branches() {
        assert_eq!(
            split_fanouts_action(DelayModel::Unit, 10),
            SplitFanoutsAction::SkipUnitDelayModel
        );
        assert_eq!(
            split_fanouts_action(DelayModel::Mapped, 2),
            SplitFanoutsAction::SkipInsufficientFanouts
        );
        assert_eq!(
            split_fanouts_action(DelayModel::Mapped, 3),
            SplitFanoutsAction::EvaluateDuplication
        );
        assert!(req_improved(
            DelayTime::new(1.0, 1.0),
            DelayTime::new(1.0 + V_SMALL * 2.0, 1.0 + V_SMALL * 2.0)
        ));
        assert!(!req_improved(
            DelayTime::new(1.0, 1.0),
            DelayTime::new(1.0 + V_SMALL * 2.0, 1.0)
        ));
        assert_eq!(
            plan_split_fanouts(
                DelayModel::Mapped,
                4,
                DelayTime::new(5.0, 5.0),
                &[
                    DelayTime::new(6.0, 6.0),
                    DelayTime::new(6.5, 6.5),
                    DelayTime::new(6.2, 5.0),
                ]
            ),
            SplitFanoutsPlan {
                action: SplitFanoutsAction::EvaluateDuplication,
                best_index: Some(1),
                duplicate_root: true,
                fanouts_moved_to_duplicate: vec![0, 1],
            }
        );
        assert_eq!(
            plan_split_fanouts(
                DelayModel::Mapped,
                4,
                DelayTime::new(5.0, 5.0),
                &[DelayTime::new(5.0, 5.0), DelayTime::new(6.0, 6.0)]
            ),
            SplitFanoutsPlan {
                action: SplitFanoutsAction::EvaluateDuplication,
                best_index: None,
                duplicate_root: false,
                fanouts_moved_to_duplicate: Vec::new(),
            }
        );
        assert!(cube_count_allows_decomposition(199));
        assert!(!cube_count_allows_decomposition(200));

        assert_eq!(
            optimization_plan(OptimizationKind::Fanout, true, true, None).unwrap(),
            OptimizationPlan {
                duplicate_first: true,
                operation: PlannedOperation::BufferFanout { mode: 7 },
                remap_if_required: false,
                trace_after: true,
            }
        );
        assert_eq!(
            optimization_plan(OptimizationKind::Divisor, true, false, Some(250))
                .unwrap()
                .operation,
            PlannedOperation::UseOriginalCopy
        );
        assert_eq!(
            optimization_plan(OptimizationKind::Divisor, true, false, Some(20))
                .unwrap()
                .operation,
            PlannedOperation::TimingDivisor
        );
    }

    #[test]
    fn buffer_and_required_time_helpers_match_c_defaults() {
        assert_eq!(
            init_buffer_parameters(DelayModel::Mapped),
            BufferParameters {
                limit: 2,
                trace: false,
                threshold: 0.5,
                single_pass: true,
                do_decomp: false,
                use_mapped: true,
            }
        );
        assert!(!init_buffer_parameters(DelayModel::UnitFanout).use_mapped);

        let mut outputs = vec![
            ("po0", None, DelayTime::new(1.0, 2.0)),
            (
                "po1",
                Some(DelayTime::new(3.0, 4.0)),
                DelayTime::new(5.0, 6.0),
            ),
        ];
        let stored = store_required_times(&mut outputs);
        assert_eq!(stored.generated_required_times, vec!["po0"]);
        assert_eq!(outputs[0].1, Some(DelayTime::new(1.0, 2.0)));
        assert_eq!(outputs[1].1, Some(DelayTime::new(3.0, 4.0)));

        let mut required = HashMap::from([
            ("po0", Some(DelayTime::new(1.0, 2.0))),
            ("po1", Some(DelayTime::new(3.0, 4.0))),
        ]);
        restore_required_times(&mut required, &stored.generated_required_times);
        assert_eq!(required["po0"], None);
        assert_eq!(required["po1"], Some(DelayTime::new(3.0, 4.0)));
    }

    #[test]
    fn graph_bound_entry_points_report_missing_graph_apis() {
        assert_eq!(
            optimize_sis_network(),
            Err(NspUtilError::SisGraphDependency {
                operation: "sp_*_opt",
                source: "LogicSynthesis/sis/speed/nsp_util.c:1395",
            })
        );
        assert_eq!(
            duplicate_sis_network(),
            Err(NspUtilError::SisGraphDependency {
                operation: "speed_network_dup",
                source: "LogicSynthesis/sis/speed/nsp_util.c:1989",
            })
        );
    }

    #[test]
    fn close_helper_keeps_float_assertion_available() {
        assert_close(1.0 + 1.0e-10, 1.0);
    }
}
