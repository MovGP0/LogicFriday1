//! Native balanced two-level buffering planner.
//!
//! The balanced transform divides positive and negative fanouts into nearly
//! equal groups, evaluates inverter choices for each level, and returns an
//! owned plan describing the selected topology.

use super::buf_recur::{BufferCell, Fanout, FanoutPhase, GateVersion, Phase};
use super::sp_buffer::{DelayTime, NEG_LARGE, POS_LARGE, V_SMALL};
use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Trans2Input {
    pub fanouts: Vec<Fanout>,
    pub positive_count: usize,
    pub negative_count: usize,
    pub max_input_load: f64,
    pub auto_route: f64,
    pub original_required: DelayTime,
    pub previous_drive: DelayTime,
    pub original_pin_load: f64,
    pub req_diff: DelayTime,
    pub root_gates: Vec<GateVersion>,
    pub inverters: Vec<BufferCell>,
}

impl Trans2Input {
    pub fn sorted_fanouts(&self) -> Vec<Fanout> {
        sort_fanouts_like_trans2(&self.fanouts)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trans2Plan {
    pub selection: Trans2Selection,
    pub topology: Trans2TopologyPlan,
    pub recursive_branch: Option<Trans2RecursiveBranch>,
    pub target: DelayTime,
    pub original_required_after_previous_drive: DelayTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trans2Selection {
    pub root_gate_index: usize,
    pub negative_partitions: usize,
    pub positive_partitions: usize,
    pub positive_inverter_index: usize,
    pub negative_inverter_index: usize,
    pub middle_inverter_index: usize,
    pub required_at_root_input: DelayTime,
    pub required_at_gate_output: DelayTime,
    pub gate_output_load: f64,
    pub required_positive_groups: DelayTime,
    pub required_middle: DelayTime,
    pub required_negative_groups: DelayTime,
    pub area: f64,
    pub met_target: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Trans2NodeKind {
    RootGate,
    PositiveGroupInverter,
    PositiveMiddleInverter,
    NegativeGroupInverter,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trans2NodePlan {
    pub kind: Trans2NodeKind,
    pub implementation_index: usize,
    pub fanout_ids: Vec<usize>,
    pub required: DelayTime,
    pub driven_load: f64,
    pub input_load: f64,
    pub area: f64,
    pub max_load: f64,
    pub violates_load_limit: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trans2TopologyPlan {
    pub root: Trans2NodePlan,
    pub positive_middle: Option<Trans2NodePlan>,
    pub positive_groups: Vec<Trans2NodePlan>,
    pub negative_groups: Vec<Trans2NodePlan>,
    pub load_violation_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trans2RecursiveBranch {
    pub fanouts: Vec<Fanout>,
    pub positive_count: usize,
    pub negative_count: usize,
    pub req_diff: DelayTime,
    pub max_input_load: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BalancedFanoutGroupKind {
    Positive,
    Negative,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BalancedFanoutGroup {
    pub kind: BalancedFanoutGroupKind,
    pub range: std::ops::Range<usize>,
    pub fanout_ids: Vec<usize>,
    pub required_before_buffer: DelayTime,
    pub required_after_buffer: DelayTime,
    pub driven_load: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupedRequired {
    pub groups: Vec<BalancedFanoutGroup>,
    pub required_at_driver: DelayTime,
    pub driver_load: f64,
    pub area: f64,
}

pub fn plan_balanced_trans2(input: &Trans2Input) -> Result<Option<Trans2Plan>, BufTrans2Error> {
    validate_input(input)?;
    let sorted = input.sorted_fanouts();
    validate_phase_partition(&sorted, input.positive_count, input.negative_count)?;

    let adjusted_original = drive_adjustment(
        input.previous_drive,
        input.original_pin_load,
        input.original_required,
    );
    let target = DelayTime::new(
        adjusted_original.rise + input.req_diff.rise,
        adjusted_original.fall + input.req_diff.fall,
    );
    let Some(selection) = select_balanced_trans2(input, &sorted, adjusted_original, target)? else {
        return Ok(None);
    };

    if !req_improved(selection.required_at_root_input, adjusted_original) {
        return Ok(None);
    }

    let topology = trans2_topology_for_selection(input, &sorted, &selection)?;
    let recursive_branch = (!selection.met_target)
        .then(|| recursive_branch_for_selection(input, &topology, &selection, adjusted_original));

    Ok(Some(Trans2Plan {
        selection,
        topology,
        recursive_branch,
        target,
        original_required_after_previous_drive: adjusted_original,
    }))
}

pub fn select_balanced_trans2(
    input: &Trans2Input,
    sorted_fanouts: &[Fanout],
    adjusted_original: DelayTime,
    target: DelayTime,
) -> Result<Option<Trans2Selection>, BufTrans2Error> {
    validate_input(input)?;
    validate_phase_partition(sorted_fanouts, input.positive_count, input.negative_count)?;

    if input.positive_count + input.negative_count == 0 {
        return Ok(None);
    }
    if partition_counts(input.positive_count).is_empty()
        && partition_counts(input.negative_count).is_empty()
    {
        return Ok(None);
    }

    let positive = &sorted_fanouts[..input.positive_count];
    let negative = &sorted_fanouts[input.positive_count..];
    let mut met_target = false;
    let mut min_area = POS_LARGE;
    let mut best_required = DelayTime::new(NEG_LARGE, NEG_LARGE);
    let mut best_selection = None;

    for (root_gate_index, root_gate) in input.root_gates.iter().enumerate() {
        if root_gate.input_load > input.max_input_load {
            continue;
        }

        for (negative_inverter_index, negative_inverter) in input.inverters.iter().enumerate() {
            for negative_partitions in partition_counts_or_zero(input.negative_count) {
                let negative_result = grouped_required(
                    negative,
                    negative_partitions,
                    negative_inverter,
                    input.auto_route,
                    BalancedFanoutGroupKind::Negative,
                )?;

                for (positive_inverter_index, positive_inverter) in
                    input.inverters.iter().enumerate()
                {
                    for positive_partitions in partition_counts_or_zero(input.positive_count) {
                        let positive_result = grouped_required(
                            positive,
                            positive_partitions,
                            positive_inverter,
                            input.auto_route,
                            BalancedFanoutGroupKind::Positive,
                        )?;

                        for (middle_inverter_index, middle_inverter) in
                            input.inverters.iter().enumerate()
                        {
                            let (required_middle, middle_load, middle_area) =
                                if input.positive_count > 0 {
                                    (
                                        subtract_delay(
                                            middle_inverter.phase,
                                            middle_inverter.block,
                                            middle_inverter.drive,
                                            positive_result.driver_load,
                                            positive_result.required_at_driver,
                                        ),
                                        middle_inverter.input_load + input.auto_route,
                                        middle_inverter.area,
                                    )
                                } else {
                                    (positive_result.required_at_driver, 0.0, 0.0)
                                };

                            let required_at_gate_output =
                                min_delay(required_middle, negative_result.required_at_driver);
                            let gate_output_load = middle_load + negative_result.driver_load;
                            let required_at_root_input = drive_adjustment(
                                input.previous_drive,
                                root_gate.input_load,
                                subtract_delay(
                                    root_gate.phase,
                                    root_gate.block,
                                    root_gate.drive,
                                    gate_output_load,
                                    required_at_gate_output,
                                ),
                            );
                            let area = root_gate.area
                                + positive_result.area
                                + middle_area
                                + negative_result.area;
                            let selection = Trans2Selection {
                                root_gate_index,
                                negative_partitions,
                                positive_partitions,
                                positive_inverter_index,
                                negative_inverter_index,
                                middle_inverter_index,
                                required_at_root_input,
                                required_at_gate_output,
                                gate_output_load,
                                required_positive_groups: positive_result.required_at_driver,
                                required_middle,
                                required_negative_groups: negative_result.required_at_driver,
                                area,
                                met_target: req_improved(required_at_root_input, target),
                            };

                            if selection.met_target && area < min_area {
                                met_target = true;
                                min_area = area;
                                best_selection = Some(selection);
                            } else if !met_target
                                && req_improved(required_at_root_input, best_required)
                            {
                                best_required = required_at_root_input;
                                best_selection = Some(selection);
                            }
                        }
                    }
                }
            }

            if input.negative_count == 0 {
                break;
            }
        }
    }

    Ok(best_selection.filter(|selection| {
        selection.met_target || req_improved(selection.required_at_root_input, adjusted_original)
    }))
}

pub fn trans2_topology_for_selection(
    input: &Trans2Input,
    sorted_fanouts: &[Fanout],
    selection: &Trans2Selection,
) -> Result<Trans2TopologyPlan, BufTrans2Error> {
    validate_input(input)?;
    validate_phase_partition(sorted_fanouts, input.positive_count, input.negative_count)?;

    let positive = &sorted_fanouts[..input.positive_count];
    let negative = &sorted_fanouts[input.positive_count..];
    let positive_inverter = &input.inverters[selection.positive_inverter_index];
    let negative_inverter = &input.inverters[selection.negative_inverter_index];
    let middle_inverter = &input.inverters[selection.middle_inverter_index];
    let root_gate = &input.root_gates[selection.root_gate_index];
    let positive_groups = grouped_required(
        positive,
        selection.positive_partitions,
        positive_inverter,
        input.auto_route,
        BalancedFanoutGroupKind::Positive,
    )?;
    let negative_groups = grouped_required(
        negative,
        selection.negative_partitions,
        negative_inverter,
        input.auto_route,
        BalancedFanoutGroupKind::Negative,
    )?;
    let mut load_violation_count = 0usize;

    let positive_group_nodes = positive_groups
        .groups
        .iter()
        .map(|group| {
            let violates = group.driven_load >= positive_inverter.max_load;
            load_violation_count += usize::from(violates);
            Trans2NodePlan {
                kind: Trans2NodeKind::PositiveGroupInverter,
                implementation_index: selection.positive_inverter_index,
                fanout_ids: group.fanout_ids.clone(),
                required: group.required_after_buffer,
                driven_load: group.driven_load,
                input_load: positive_inverter.input_load,
                area: positive_inverter.area,
                max_load: positive_inverter.max_load,
                violates_load_limit: violates,
            }
        })
        .collect::<Vec<_>>();

    let negative_group_nodes = negative_groups
        .groups
        .iter()
        .map(|group| {
            let violates = group.driven_load >= negative_inverter.max_load;
            load_violation_count += usize::from(violates);
            Trans2NodePlan {
                kind: Trans2NodeKind::NegativeGroupInverter,
                implementation_index: selection.negative_inverter_index,
                fanout_ids: group.fanout_ids.clone(),
                required: group.required_after_buffer,
                driven_load: group.driven_load,
                input_load: negative_inverter.input_load,
                area: negative_inverter.area,
                max_load: negative_inverter.max_load,
                violates_load_limit: violates,
            }
        })
        .collect::<Vec<_>>();

    let positive_middle = if input.positive_count > 0 {
        let violates = positive_groups.driver_load >= middle_inverter.max_load;
        load_violation_count += usize::from(violates);
        Some(Trans2NodePlan {
            kind: Trans2NodeKind::PositiveMiddleInverter,
            implementation_index: selection.middle_inverter_index,
            fanout_ids: positive_group_nodes
                .iter()
                .flat_map(|node| node.fanout_ids.iter().copied())
                .collect(),
            required: selection.required_middle,
            driven_load: positive_groups.driver_load,
            input_load: middle_inverter.input_load,
            area: middle_inverter.area,
            max_load: middle_inverter.max_load,
            violates_load_limit: violates,
        })
    } else {
        None
    };

    let root_violates = selection.gate_output_load >= root_gate.output_load_limit;
    load_violation_count += usize::from(root_violates);
    let root = Trans2NodePlan {
        kind: Trans2NodeKind::RootGate,
        implementation_index: selection.root_gate_index,
        fanout_ids: sorted_fanouts.iter().map(|fanout| fanout.id).collect(),
        required: selection.required_at_gate_output,
        driven_load: selection.gate_output_load,
        input_load: root_gate.input_load,
        area: root_gate.area,
        max_load: root_gate.output_load_limit,
        violates_load_limit: root_violates,
    };

    Ok(Trans2TopologyPlan {
        root,
        positive_middle,
        positive_groups: positive_group_nodes,
        negative_groups: negative_group_nodes,
        load_violation_count,
    })
}

pub fn grouped_required(
    fanouts: &[Fanout],
    partitions: usize,
    buffer: &BufferCell,
    auto_route: f64,
    kind: BalancedFanoutGroupKind,
) -> Result<GroupedRequired, BufTrans2Error> {
    if fanouts.is_empty() {
        if partitions == 0 {
            return Ok(GroupedRequired {
                groups: Vec::new(),
                required_at_driver: large_required_time(),
                driver_load: 0.0,
                area: 0.0,
            });
        }

        return Err(BufTrans2Error::InvalidPartitionCount {
            fanouts: 0,
            partitions,
        });
    }

    if partitions == 0 || partitions > fanouts.len() {
        return Err(BufTrans2Error::InvalidPartitionCount {
            fanouts: fanouts.len(),
            partitions,
        });
    }

    let mut required_at_driver = large_required_time();
    let mut groups = Vec::with_capacity(partitions);
    for range in partition_ranges(fanouts.len(), partitions)? {
        let partition = &fanouts[range.clone()];
        let required_before_buffer = min_required(partition);
        let driven_load = partition.iter().map(|fanout| fanout.load).sum();
        let required_after_buffer = subtract_delay(
            buffer.phase,
            buffer.block,
            buffer.drive,
            driven_load,
            required_before_buffer,
        );
        required_at_driver = min_delay(required_at_driver, required_after_buffer);
        groups.push(BalancedFanoutGroup {
            kind,
            range,
            fanout_ids: partition.iter().map(|fanout| fanout.id).collect(),
            required_before_buffer,
            required_after_buffer,
            driven_load,
        });
    }

    Ok(GroupedRequired {
        groups,
        required_at_driver,
        driver_load: partitions as f64 * (auto_route + buffer.input_load),
        area: partitions as f64 * buffer.area,
    })
}

pub fn partition_counts(fanout_count: usize) -> Vec<usize> {
    match fanout_count {
        0 => Vec::new(),
        1 => vec![1],
        _ => (2..=2usize.max(fanout_count / 2)).collect(),
    }
}

pub fn partition_counts_or_zero(fanout_count: usize) -> Vec<usize> {
    if fanout_count == 0 {
        vec![0]
    } else {
        partition_counts(fanout_count)
    }
}

pub fn partition_ranges(
    count: usize,
    partitions: usize,
) -> Result<Vec<std::ops::Range<usize>>, BufTrans2Error> {
    if partitions == 0 || partitions > count {
        return Err(BufTrans2Error::InvalidPartitionCount {
            fanouts: count,
            partitions,
        });
    }

    let mut ranges = Vec::with_capacity(partitions);
    let mut start = 0usize;
    for remaining in (1..=partitions).rev() {
        let end = start + (count - start) / remaining;
        ranges.push(start..end);
        start = end;
    }
    Ok(ranges)
}

pub fn subtract_delay(
    phase: Phase,
    block: DelayTime,
    drive: DelayTime,
    load: f64,
    required: DelayTime,
) -> DelayTime {
    let delay = DelayTime::new(
        block.rise + drive.rise * load,
        block.fall + drive.fall * load,
    );
    compute_required_time(phase, required, delay)
}

pub fn compute_required_time(phase: Phase, required: DelayTime, delay: DelayTime) -> DelayTime {
    let mut rise = f64::INFINITY;
    let mut fall = f64::INFINITY;

    if matches!(phase, Phase::Inverting | Phase::Neither) {
        rise = rise.min(required.fall - delay.fall);
        fall = fall.min(required.rise - delay.rise);
    }
    if matches!(phase, Phase::NonInverting | Phase::Neither) {
        rise = rise.min(required.rise - delay.rise);
        fall = fall.min(required.fall - delay.fall);
    }

    DelayTime::new(rise, fall)
}

pub fn drive_adjustment(drive: DelayTime, load: f64, required: DelayTime) -> DelayTime {
    DelayTime::new(
        required.rise - drive.rise * load,
        required.fall - drive.fall * load,
    )
}

pub fn sort_fanouts_like_trans2(fanouts: &[Fanout]) -> Vec<Fanout> {
    let mut sorted = fanouts.to_vec();
    sorted.sort_by(compare_fanout);
    sorted
}

pub fn compare_fanout(left: &Fanout, right: &Fanout) -> Ordering {
    fanout_phase_rank(left.phase)
        .cmp(&fanout_phase_rank(right.phase))
        .then_with(|| {
            left.required
                .min_edge()
                .partial_cmp(&right.required.min_edge())
                .unwrap_or(Ordering::Equal)
        })
}

fn recursive_branch_for_selection(
    input: &Trans2Input,
    topology: &Trans2TopologyPlan,
    selection: &Trans2Selection,
    adjusted_original: DelayTime,
) -> Trans2RecursiveBranch {
    let mut fanouts =
        Vec::with_capacity(topology.negative_groups.len() + topology.positive_groups.len());

    for (index, node) in topology.negative_groups.iter().enumerate() {
        fanouts.push(Fanout {
            id: index,
            pin: 0,
            phase: FanoutPhase::Positive,
            required: node.required,
            load: input.auto_route + node.input_load,
        });
    }
    for (index, node) in topology.positive_groups.iter().enumerate() {
        fanouts.push(Fanout {
            id: topology.negative_groups.len() + index,
            pin: 0,
            phase: FanoutPhase::Negative,
            required: node.required,
            load: input.auto_route + node.input_load,
        });
    }

    Trans2RecursiveBranch {
        fanouts,
        positive_count: selection.negative_partitions,
        negative_count: selection.positive_partitions,
        req_diff: DelayTime::new(
            input.req_diff.rise - selection.required_at_root_input.rise + adjusted_original.rise,
            input.req_diff.fall - selection.required_at_root_input.fall + adjusted_original.fall,
        ),
        max_input_load: input.max_input_load,
    }
}

fn validate_input(input: &Trans2Input) -> Result<(), BufTrans2Error> {
    if input.positive_count + input.negative_count != input.fanouts.len() {
        return Err(BufTrans2Error::InvalidFanoutCounts {
            positive: input.positive_count,
            negative: input.negative_count,
            fanouts: input.fanouts.len(),
        });
    }
    if input.root_gates.is_empty() {
        return Err(BufTrans2Error::NoRootGates);
    }
    if input.inverters.is_empty() {
        return Err(BufTrans2Error::NoInverters);
    }
    if input
        .inverters
        .iter()
        .any(|buffer| buffer.phase != Phase::Inverting)
    {
        return Err(BufTrans2Error::NonInvertingBuffer);
    }
    if !input.max_input_load.is_finite()
        || !input.auto_route.is_finite()
        || !input.original_pin_load.is_finite()
        || !delay_time_is_finite(input.original_required)
        || !delay_time_is_finite(input.previous_drive)
        || !delay_time_is_finite(input.req_diff)
        || input
            .fanouts
            .iter()
            .any(|fanout| !fanout.load.is_finite() || !delay_time_is_finite(fanout.required))
        || input.root_gates.iter().any(|gate| {
            !gate.area.is_finite()
                || !gate.input_load.is_finite()
                || !gate.output_load_limit.is_finite()
                || !delay_time_is_finite(gate.block)
                || !delay_time_is_finite(gate.drive)
        })
        || input.inverters.iter().any(|buffer| {
            !buffer.area.is_finite()
                || !buffer.input_load.is_finite()
                || !buffer.max_load.is_finite()
                || !delay_time_is_finite(buffer.block)
                || !delay_time_is_finite(buffer.drive)
        })
    {
        return Err(BufTrans2Error::NonFiniteDelayData);
    }

    Ok(())
}

fn validate_phase_partition(
    fanouts: &[Fanout],
    positive_count: usize,
    negative_count: usize,
) -> Result<(), BufTrans2Error> {
    if positive_count + negative_count != fanouts.len() {
        return Err(BufTrans2Error::InvalidFanoutCounts {
            positive: positive_count,
            negative: negative_count,
            fanouts: fanouts.len(),
        });
    }
    if fanouts[..positive_count]
        .iter()
        .any(|fanout| fanout.phase != FanoutPhase::Positive)
        || fanouts[positive_count..]
            .iter()
            .any(|fanout| fanout.phase != FanoutPhase::Negative)
    {
        return Err(BufTrans2Error::FanoutsNotPhasePartitioned);
    }
    Ok(())
}

fn fanout_phase_rank(phase: FanoutPhase) -> u8 {
    match phase {
        FanoutPhase::Positive => 0,
        FanoutPhase::Negative => 1,
    }
}

fn delay_time_is_finite(time: DelayTime) -> bool {
    time.rise.is_finite() && time.fall.is_finite()
}

fn large_required_time() -> DelayTime {
    DelayTime::new(POS_LARGE, POS_LARGE)
}

fn min_required(fanouts: &[Fanout]) -> DelayTime {
    fanouts.iter().fold(large_required_time(), |best, fanout| {
        min_delay(best, fanout.required)
    })
}

fn min_delay(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise.min(right.rise), left.fall.min(right.fall))
}

fn req_improved(left: DelayTime, right: DelayTime) -> bool {
    (left.rise - right.rise) > V_SMALL && (left.fall - right.fall) > V_SMALL
}

#[derive(Clone, Debug, PartialEq)]
pub enum BufTrans2Error {
    InvalidFanoutCounts {
        positive: usize,
        negative: usize,
        fanouts: usize,
    },
    FanoutsNotPhasePartitioned,
    InvalidPartitionCount {
        fanouts: usize,
        partitions: usize,
    },
    NoRootGates,
    NoInverters,
    NonInvertingBuffer,
    NonFiniteDelayData,
}

impl fmt::Display for BufTrans2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFanoutCounts {
                positive,
                negative,
                fanouts,
            } => write!(
                f,
                "invalid balanced-transform fanout counts: positive {positive} + negative {negative} != fanouts {fanouts}",
            ),
            Self::FanoutsNotPhasePartitioned => write!(
                f,
                "fanouts must be sorted with positive phase entries before negative phase entries",
            ),
            Self::InvalidPartitionCount {
                fanouts,
                partitions,
            } => write!(
                f,
                "cannot split {fanouts} balanced-transform fanouts into {partitions} partitions",
            ),
            Self::NoRootGates => write!(
                f,
                "balanced transform requires at least one root gate version",
            ),
            Self::NoInverters => write!(
                f,
                "balanced transform requires at least one inverter implementation",
            ),
            Self::NonInvertingBuffer => write!(
                f,
                "balanced transform buffer choices must be inverting implementations",
            ),
            Self::NonFiniteDelayData => write!(
                f,
                "balanced transform delay, load, and area data must be finite",
            ),
        }
    }
}

impl Error for BufTrans2Error {}

#[cfg(test)]
mod tests {
    use super::*;

    fn inv(name: &str, area: f64, input_load: f64, block: f64, drive: f64) -> BufferCell {
        BufferCell {
            name: name.to_owned(),
            depth: 1,
            area,
            input_load,
            max_load: 10.0,
            phase: Phase::Inverting,
            block: DelayTime::new(block, block),
            drive: DelayTime::new(drive, drive),
        }
    }

    fn gate(name: &str, area: f64, input_load: f64, block: f64, drive: f64) -> GateVersion {
        GateVersion {
            name: name.to_owned(),
            area,
            input_load,
            output_load_limit: 10.0,
            phase: Phase::NonInverting,
            block: DelayTime::new(block, block),
            drive: DelayTime::new(drive, drive),
        }
    }

    fn fanout(id: usize, phase: FanoutPhase, required: f64, load: f64) -> Fanout {
        Fanout {
            id,
            pin: 0,
            phase,
            required: DelayTime::new(required, required),
            load,
        }
    }

    fn base_input() -> Trans2Input {
        Trans2Input {
            fanouts: vec![
                fanout(1, FanoutPhase::Positive, 12.0, 1.0),
                fanout(2, FanoutPhase::Positive, 11.0, 2.0),
                fanout(3, FanoutPhase::Negative, 10.0, 1.0),
                fanout(4, FanoutPhase::Negative, 9.0, 2.0),
            ],
            positive_count: 2,
            negative_count: 2,
            max_input_load: 5.0,
            auto_route: 0.25,
            original_required: DelayTime::new(2.0, 2.0),
            previous_drive: DelayTime::new(0.0, 0.0),
            original_pin_load: 0.0,
            req_diff: DelayTime::new(2.0, 2.0),
            root_gates: vec![gate("g0", 3.0, 1.0, 0.1, 0.1)],
            inverters: vec![
                inv("expensive", 4.0, 1.0, 0.3, 0.1),
                inv("cheap", 1.0, 1.0, 0.3, 0.1),
            ],
        }
    }

    #[test]
    fn partition_counts_match_balanced_do_while_bounds() {
        assert_eq!(partition_counts(0), Vec::<usize>::new());
        assert_eq!(partition_counts(1), vec![1]);
        assert_eq!(partition_counts(2), vec![2]);
        assert_eq!(partition_counts(3), vec![2]);
        assert_eq!(partition_counts(6), vec![2, 3]);
        assert_eq!(partition_counts_or_zero(0), vec![0]);
    }

    #[test]
    fn partition_ranges_match_c_balanced_loop() {
        assert_eq!(partition_ranges(5, 2).unwrap(), vec![0..2, 2..5]);
        assert_eq!(partition_ranges(6, 3).unwrap(), vec![0..2, 2..4, 4..6]);
    }

    #[test]
    fn required_time_subtraction_applies_pin_phase() {
        assert_eq!(
            subtract_delay(
                Phase::NonInverting,
                DelayTime::new(1.0, 2.0),
                DelayTime::new(0.5, 0.25),
                4.0,
                DelayTime::new(20.0, 30.0),
            ),
            DelayTime::new(17.0, 27.0)
        );
        assert_eq!(
            subtract_delay(
                Phase::Inverting,
                DelayTime::new(1.0, 2.0),
                DelayTime::new(0.5, 0.25),
                4.0,
                DelayTime::new(20.0, 30.0),
            ),
            DelayTime::new(27.0, 17.0)
        );
    }

    #[test]
    fn grouped_required_builds_balanced_group_data() {
        let result = grouped_required(
            &[
                fanout(10, FanoutPhase::Positive, 10.0, 2.0),
                fanout(11, FanoutPhase::Positive, 8.0, 3.0),
            ],
            2,
            &inv("i0", 3.0, 1.0, 1.0, 0.5),
            0.25,
            BalancedFanoutGroupKind::Positive,
        )
        .unwrap();

        assert_eq!(result.driver_load, 2.5);
        assert_eq!(result.area, 6.0);
        assert_eq!(result.groups.len(), 2);
        assert_eq!(result.groups[0].fanout_ids, vec![10]);
        assert_eq!(
            result.groups[1].required_after_buffer,
            DelayTime::new(5.5, 5.5)
        );
        assert_eq!(result.required_at_driver, DelayTime::new(5.5, 5.5));
    }

    #[test]
    fn selection_prefers_lowest_area_candidate_after_target_is_met() {
        let input = base_input();
        let sorted = input.sorted_fanouts();
        let selection = select_balanced_trans2(
            &input,
            &sorted,
            input.original_required,
            DelayTime::new(4.0, 4.0),
        )
        .unwrap()
        .unwrap();

        assert!(selection.met_target);
        assert_eq!(selection.positive_inverter_index, 1);
        assert_eq!(selection.negative_inverter_index, 1);
        assert_eq!(selection.middle_inverter_index, 1);
        assert_eq!(selection.positive_partitions, 2);
        assert_eq!(selection.negative_partitions, 2);
    }

    #[test]
    fn planner_returns_topology_and_recursive_branch_when_short_of_target() {
        let mut input = base_input();
        input.req_diff = DelayTime::new(100.0, 100.0);

        let plan = plan_balanced_trans2(&input)
            .unwrap()
            .expect("expected balanced plan");

        assert!(!plan.selection.met_target);
        assert_eq!(plan.topology.positive_groups.len(), 2);
        assert_eq!(plan.topology.negative_groups.len(), 2);
        assert_eq!(
            plan.topology.positive_middle.as_ref().map(|node| node.kind),
            Some(Trans2NodeKind::PositiveMiddleInverter)
        );

        let branch = plan.recursive_branch.unwrap();
        assert_eq!(branch.positive_count, 2);
        assert_eq!(branch.negative_count, 2);
        assert!(
            branch
                .fanouts
                .iter()
                .take(2)
                .all(|fanout| fanout.phase == FanoutPhase::Positive)
        );
        assert!(
            branch
                .fanouts
                .iter()
                .skip(2)
                .all(|fanout| fanout.phase == FanoutPhase::Negative)
        );
    }

    #[test]
    fn no_fanouts_means_no_plan() {
        let mut input = base_input();
        input.fanouts.clear();
        input.positive_count = 0;
        input.negative_count = 0;

        assert_eq!(plan_balanced_trans2(&input).unwrap(), None);
    }

    #[test]
    fn validation_rejects_unsorted_or_noninverting_inputs() {
        let mut input = base_input();
        input.fanouts.swap(0, 2);
        assert!(matches!(
            select_balanced_trans2(
                &input,
                &input.fanouts,
                input.original_required,
                DelayTime::new(4.0, 4.0),
            ),
            Err(BufTrans2Error::FanoutsNotPhasePartitioned)
        ));

        let mut input = base_input();
        input.inverters[0].phase = Phase::NonInverting;
        assert_eq!(
            plan_balanced_trans2(&input),
            Err(BufTrans2Error::NonInvertingBuffer)
        );
    }
}
