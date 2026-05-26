//! Native Rust planning port for `sis/speed/buf_recur.c`.
//!
//! The C module combines two concerns: selecting a recursive buffering
//! transformation and mutating SIS nodes/networks to apply it.  The native
//! Rust port keeps the feasible, testable part as a pure planner: fanout
//! ordering, cumulative capacitance tables, required-time propagation, the
//! unbalanced `trans3` cost search, recursive branch planning, and load-limit
//! checks.  Direct SIS graph mutation is represented by explicit dependency
//! errors until the node, network, delay, and library layers are native Rust.

use super::sp_buffer::{DelayTime, NEG_LARGE, POS_LARGE, V_SMALL};
use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

pub const EPS: f64 = 1.0e-6;
pub const REPOWER_MASK: u8 = 1 << 0;
pub const UNBALANCED_MASK: u8 = 1 << 1;
pub const BALANCED_MASK: u8 = 1 << 2;
pub const DEFAULT_BUFFER_MODE: u8 = REPOWER_MASK | UNBALANCED_MASK | BALANCED_MASK;

pub const REQUIRED_PORT_BEADS: &[&str] = &[
    "LogicFriday1-8j8.2.6.460", // speed/buf_delay.c: delay and required-time helpers
    "LogicFriday1-8j8.2.6.462", // speed/buf_replace.c: cell-strength replacement
    "LogicFriday1-8j8.2.6.463", // speed/buf_trans2.c: balanced decomposition
    "LogicFriday1-8j8.2.6.464", // speed/buf_util.c: buffer library and annotation helpers
    "LogicFriday1-8j8.2.6.258", // map/libutil.c: mapped gate library traversal
    "LogicFriday1-8j8.2.6.313", // node/fan.c: fanin/fanout rewiring
    "LogicFriday1-8j8.2.6.318", // node/node.c: node creation and literals
    "LogicFriday1-8j8.2.6.297", // network/dfs.c and network mutation helpers
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    NotGiven,
    Inverting,
    NonInverting,
    Neither,
}

impl Phase {
    pub fn is_inverting_buffer(self) -> bool {
        self == Self::Inverting
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferCell {
    pub name: String,
    pub depth: usize,
    pub area: f64,
    pub input_load: f64,
    pub max_load: f64,
    pub phase: Phase,
    pub block: DelayTime,
    pub drive: DelayTime,
}

impl BufferCell {
    pub fn output_required_time(&self, required: DelayTime, load: f64) -> DelayTime {
        subtract_delay(self.phase, self.block, self.drive, load, required)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GateVersion {
    pub name: String,
    pub area: f64,
    pub input_load: f64,
    pub output_load_limit: f64,
    pub phase: Phase,
    pub block: DelayTime,
    pub drive: DelayTime,
}

impl GateVersion {
    pub fn output_required_time(&self, required: DelayTime, load: f64) -> DelayTime {
        subtract_delay(self.phase, self.block, self.drive, load, required)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutPhase {
    Positive,
    Negative,
}

impl FanoutPhase {
    fn sort_rank(self) -> u8 {
        match self {
            Self::Positive => 0,
            Self::Negative => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fanout {
    pub id: usize,
    pub pin: usize,
    pub phase: FanoutPhase,
    pub required: DelayTime,
    pub load: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CumulativeCapacitance {
    pub cap_k_pos: Vec<f64>,
    pub cap_k_neg: Vec<f64>,
    pub cap_l_pos: Vec<f64>,
    pub cap_l_neg: Vec<f64>,
    pub do_unbalanced: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferRecurInput {
    pub fanouts: Vec<Fanout>,
    pub positive_count: usize,
    pub negative_count: usize,
    pub buffers: Vec<BufferCell>,
    pub inverting_buffer_count: usize,
    pub gate_versions: Vec<GateVersion>,
    pub req_diff: DelayTime,
    pub original_required: DelayTime,
    pub original_load: f64,
    pub previous_drive: DelayTime,
    pub max_input_load: f64,
    pub auto_route: f64,
    pub min_req_diff: f64,
    pub mode: u8,
    pub current_level: usize,
    pub root_is_original_node: bool,
    pub do_decomp: bool,
    pub interactive: bool,
    pub alternate_input_slack_would_fail: bool,
}

impl BufferRecurInput {
    pub fn sorted_fanouts(&self) -> Vec<Fanout> {
        sort_fanouts_like_sis(&self.fanouts)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trans3Selection {
    pub b_index: usize,
    pub big_b_index: usize,
    pub inverter_index: usize,
    pub gate_index: usize,
    pub positive_split: usize,
    pub negative_split: usize,
    pub use_noninverting_buffers: bool,
    pub met_target: bool,
    pub area: f64,
    pub required_at_root_input: DelayTime,
    pub required_at_gate_output: DelayTime,
    pub gate_output_load: f64,
    pub required_b: DelayTime,
    pub required_big_b: DelayTime,
    pub required_inverter: DelayTime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecursiveBranchKind {
    NewPositiveBuffer,
    NewNegativeBuffer,
    NewMixedInvertingBuffer,
    OriginalRoot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecursiveBranchPlan {
    pub kind: RecursiveBranchKind,
    pub positive_count: usize,
    pub negative_count: usize,
    pub req_diff: DelayTime,
    pub max_input_load: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BufferRecurAction {
    NoChangeRequired,
    ReplaceSingleFanoutCell,
    UseUnbalancedTrans3 {
        selection: Trans3Selection,
        recursive_branches: Vec<RecursiveBranchPlan>,
    },
    TryBalancedDecomposition {
        reason: BalancedFallbackReason,
    },
    TryRootDuplication {
        reason: RootDuplicationReason,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BalancedFallbackReason {
    UnbalancedNotAllowed,
    UnbalancedNotWarranted,
    UnbalancedDidNotImprove,
    BetterForAllNegativeFanouts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootDuplicationReason {
    RedistributionDidNotMeetTarget,
    BalancedDisabled,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferRecurPlan {
    pub action: BufferRecurAction,
    pub target: DelayTime,
    pub original_required_after_previous_drive: DelayTime,
    pub cumulative_capacitance: Option<CumulativeCapacitance>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoadImplementation {
    None,
    Buffer { max_load: f64 },
    Gate { max_load: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnnotatedLoad {
    pub implementation: LoadImplementation,
    pub load: f64,
}

pub fn plan_buffer_recursion(input: &BufferRecurInput) -> Result<BufferRecurPlan, BufRecurError> {
    validate_input(input)?;

    let adjusted_original = drive_adjustment(
        input.previous_drive,
        input.original_load,
        input.original_required,
    );
    let target = DelayTime::new(
        adjusted_original.rise + input.req_diff.rise,
        adjusted_original.fall + input.req_diff.fall,
    );

    if input.req_diff.max_edge() < EPS {
        return Ok(BufferRecurPlan {
            action: BufferRecurAction::NoChangeRequired,
            target,
            original_required_after_previous_drive: adjusted_original,
            cumulative_capacitance: None,
        });
    }

    if input.fanouts.len() == 1 && input.current_level == 1 {
        return Ok(BufferRecurPlan {
            action: BufferRecurAction::ReplaceSingleFanoutCell,
            target,
            original_required_after_previous_drive: adjusted_original,
            cumulative_capacitance: None,
        });
    }

    let sorted = input.sorted_fanouts();
    let cumulative = cumulative_capacitance(
        &sorted,
        input.positive_count,
        input.negative_count,
        input.min_req_diff,
    )?;

    let unbalanced_allowed = (input.mode & UNBALANCED_MASK) != 0;
    if unbalanced_allowed && cumulative.do_unbalanced {
        if let Some(selection) =
            select_unbalanced_trans3(input, &sorted, &cumulative, adjusted_original, target)?
        {
            if selection.met_target
                || req_improved(selection.required_at_root_input, adjusted_original)
            {
                if input.interactive
                    && !selection.met_target
                    && input.gate_versions.len() > selection.gate_index
                    && input.alternate_input_slack_would_fail
                {
                    return balanced_or_duplication(
                        input,
                        target,
                        adjusted_original,
                        Some(cumulative),
                    );
                }

                let branches = recursive_branches_for_selection(
                    input,
                    &sorted,
                    &selection,
                    target,
                    adjusted_original,
                );
                return Ok(BufferRecurPlan {
                    action: BufferRecurAction::UseUnbalancedTrans3 {
                        selection,
                        recursive_branches: branches,
                    },
                    target,
                    original_required_after_previous_drive: adjusted_original,
                    cumulative_capacitance: Some(cumulative),
                });
            }
        }
    }

    balanced_or_duplication(input, target, adjusted_original, Some(cumulative))
}

fn balanced_or_duplication(
    input: &BufferRecurInput,
    target: DelayTime,
    adjusted_original: DelayTime,
    cumulative: Option<CumulativeCapacitance>,
) -> Result<BufferRecurPlan, BufRecurError> {
    if (input.mode & BALANCED_MASK) != 0 {
        let reason = if (input.mode & UNBALANCED_MASK) == 0 {
            BalancedFallbackReason::UnbalancedNotAllowed
        } else if cumulative.as_ref().is_some_and(|cap| !cap.do_unbalanced) {
            BalancedFallbackReason::UnbalancedNotWarranted
        } else {
            BalancedFallbackReason::UnbalancedDidNotImprove
        };
        Ok(BufferRecurPlan {
            action: BufferRecurAction::TryBalancedDecomposition { reason },
            target,
            original_required_after_previous_drive: adjusted_original,
            cumulative_capacitance: cumulative,
        })
    } else if input.current_level == 1 && input.do_decomp {
        Ok(BufferRecurPlan {
            action: BufferRecurAction::TryRootDuplication {
                reason: RootDuplicationReason::RedistributionDidNotMeetTarget,
            },
            target,
            original_required_after_previous_drive: adjusted_original,
            cumulative_capacitance: cumulative,
        })
    } else {
        Ok(BufferRecurPlan {
            action: BufferRecurAction::NoChangeRequired,
            target,
            original_required_after_previous_drive: adjusted_original,
            cumulative_capacitance: cumulative,
        })
    }
}

pub fn select_unbalanced_trans3(
    input: &BufferRecurInput,
    sorted_fanouts: &[Fanout],
    caps: &CumulativeCapacitance,
    _original_required: DelayTime,
    target: DelayTime,
) -> Result<Option<Trans3Selection>, BufRecurError> {
    validate_input(input)?;
    validate_sorted_counts(sorted_fanouts, input.positive_count, input.negative_count)?;

    let num_pos = input.positive_count;
    let num_neg = input.negative_count;
    let num_buf = input.buffers.len();
    let num_inv = input.inverting_buffer_count;
    let mut met_target = false;
    let mut min_area = POS_LARGE;
    let mut min_area_not_met = POS_LARGE;
    let mut best_required = DelayTime::new(NEG_LARGE, NEG_LARGE);
    let mut best_selection = None;

    for inverter_index in 0..num_inv {
        let inverter = &input.buffers[inverter_index];
        for positive_split in 0..=num_pos {
            let early_pos = min_required(&sorted_fanouts[..positive_split]);
            let late_pos = min_required(&sorted_fanouts[positive_split..num_pos]);

            for b_index in 0..num_buf {
                let b = &input.buffers[b_index];
                let (required_b, load_b, area_b) = if positive_split == num_pos {
                    (large_required_time(), 0.0, 0.0)
                } else {
                    let load_op_b = caps.cap_l_pos[positive_split];
                    (
                        b.output_required_time(late_pos, load_op_b),
                        b.input_load + input.auto_route,
                        b.area,
                    )
                };

                for negative_split in 0..=num_neg {
                    let early_neg =
                        min_required(&sorted_fanouts[num_pos..num_pos + negative_split]);
                    let late_neg =
                        min_required(&sorted_fanouts[num_pos + negative_split..num_pos + num_neg]);

                    for big_b_index in 0..num_buf {
                        if big_b_index >= num_inv && b_index < num_inv {
                            break;
                        }

                        let big_b = &input.buffers[big_b_index];
                        let (required_big_b, load_big_b, area_big_b) =
                            if (negative_split == num_neg && big_b_index >= num_inv)
                                || (negative_split == num_neg && positive_split == num_pos)
                            {
                                (large_required_time(), 0.0, 0.0)
                            } else {
                                let mut load_op_big_b = 0.0;
                                let mut req = if big_b_index >= num_inv {
                                    late_neg
                                } else {
                                    load_op_big_b = load_b;
                                    min_delay(required_b, late_neg)
                                };
                                load_op_big_b += caps.cap_l_neg[negative_split];
                                req = big_b.output_required_time(req, load_op_big_b);
                                (req, big_b.input_load + input.auto_route, big_b.area)
                            };

                        let (required_inverter, load_inverter, area_inverter) =
                            if negative_split == 0 && big_b_index < num_inv {
                                (large_required_time(), 0.0, 0.0)
                            } else {
                                let mut load_op_inverter = 0.0;
                                let mut req = if big_b_index >= num_inv {
                                    load_op_inverter += load_big_b;
                                    min_delay(required_big_b, early_neg)
                                } else {
                                    early_neg
                                };
                                load_op_inverter += caps.cap_k_neg[negative_split];
                                req = inverter.output_required_time(req, load_op_inverter);
                                (req, inverter.input_load + input.auto_route, inverter.area)
                            };

                        for (gate_index, gate) in input.gate_versions.iter().enumerate() {
                            if gate.input_load > input.max_input_load {
                                continue;
                            }

                            let mut load_op_gate = load_inverter;
                            let mut required_gate = if b_index >= num_inv {
                                load_op_gate += load_b;
                                min_delay(required_b, required_inverter)
                            } else {
                                load_op_gate += load_big_b;
                                min_delay(required_big_b, required_inverter)
                            };
                            if positive_split != 0 {
                                required_gate = min_delay(required_gate, early_pos);
                            }
                            load_op_gate += caps.cap_k_pos[positive_split];
                            let required_at_gate_output = required_gate;

                            required_gate = gate.output_required_time(required_gate, load_op_gate);
                            required_gate = drive_adjustment(
                                input.previous_drive,
                                gate.input_load,
                                required_gate,
                            );

                            let valid_config = (big_b_index < num_inv) == (b_index < num_inv);
                            if !valid_config {
                                continue;
                            }

                            let area = gate.area + area_b + area_big_b + area_inverter;
                            let selection = Trans3Selection {
                                b_index,
                                big_b_index,
                                inverter_index,
                                gate_index,
                                positive_split,
                                negative_split,
                                use_noninverting_buffers: b_index >= num_inv
                                    && big_b_index >= num_inv,
                                met_target: req_improved(required_gate, target),
                                area,
                                required_at_root_input: required_gate,
                                required_at_gate_output,
                                gate_output_load: load_op_gate,
                                required_b,
                                required_big_b,
                                required_inverter,
                            };

                            if selection.met_target && area < min_area {
                                met_target = true;
                                min_area = area;
                                best_selection = Some(selection);
                            } else if !met_target
                                && (req_improved(required_gate, best_required)
                                    || (req_equal(required_gate, best_required)
                                        && area < min_area_not_met))
                            {
                                best_required = required_gate;
                                min_area_not_met = area;
                                best_selection = Some(selection);
                            }
                        }
                    }
                }

                if num_pos == 0 {
                    break;
                }
            }
        }

        if num_neg == 0 {
            break;
        }
    }

    Ok(best_selection.map(|mut selection| {
        selection.met_target = req_improved(selection.required_at_root_input, target);
        selection
    }))
}

pub fn recursive_branches_for_selection(
    input: &BufferRecurInput,
    sorted_fanouts: &[Fanout],
    selection: &Trans3Selection,
    target: DelayTime,
    adjusted_original: DelayTime,
) -> Vec<RecursiveBranchPlan> {
    let num_pos = input.positive_count;
    let num_neg = input.negative_count;
    let old_pos = selection.positive_split;
    let old_neg = selection.negative_split;
    let root_drive = input.gate_versions[selection.gate_index].drive.max_edge();
    let root_drive = if root_drive.abs() < V_SMALL {
        V_SMALL
    } else {
        root_drive
    };
    let mut branches = Vec::new();

    if selection.use_noninverting_buffers {
        let new_pos = num_pos - old_pos;
        let new_neg = num_neg - old_neg;
        let pos_branch_req = min_required(&sorted_fanouts[..old_pos]);
        let neg_branch_req = min_required(&sorted_fanouts[num_pos..num_pos + old_neg]);
        let margin_pos = delay_difference(pos_branch_req, selection.required_b);
        let margin_neg = delay_difference(neg_branch_req, selection.required_big_b);

        if new_pos > 0 && positive_margin(margin_pos) {
            let buffer = &input.buffers[selection.big_b_index];
            branches.push(RecursiveBranchPlan {
                kind: RecursiveBranchKind::NewPositiveBuffer,
                positive_count: new_pos,
                negative_count: 0,
                req_diff: margin_pos,
                max_input_load: buffer.input_load + margin_pos.min_edge() / root_drive,
            });
        }

        if new_neg > 0 && positive_margin(margin_neg) {
            let buffer = &input.buffers[selection.big_b_index];
            branches.push(RecursiveBranchPlan {
                kind: RecursiveBranchKind::NewNegativeBuffer,
                positive_count: new_neg,
                negative_count: 0,
                req_diff: margin_neg,
                max_input_load: buffer.input_load + margin_neg.min_edge() / root_drive,
            });
        }
    } else {
        let new_pos = num_neg - old_neg;
        let new_neg = num_pos - old_pos;
        let pos_branch_req = min_delay(
            selection.required_inverter,
            min_required(&sorted_fanouts[..old_pos]),
        );
        let margin = delay_difference(pos_branch_req, selection.required_big_b);

        if new_pos + new_neg > 0 && positive_margin(margin) {
            let buffer = &input.buffers[selection.big_b_index];
            branches.push(RecursiveBranchPlan {
                kind: RecursiveBranchKind::NewMixedInvertingBuffer,
                positive_count: new_pos,
                negative_count: new_neg,
                req_diff: margin,
                max_input_load: buffer.input_load + margin.min_edge() / root_drive,
            });
        }
    }

    let achieved = selection.required_at_root_input;
    let remaining = delay_difference(target, achieved);
    if positive_margin(remaining) {
        branches.push(RecursiveBranchPlan {
            kind: RecursiveBranchKind::OriginalRoot,
            positive_count: old_pos + usize::from(selection.use_noninverting_buffers),
            negative_count: old_neg,
            req_diff: remaining,
            max_input_load: input.max_input_load,
        });
    } else if req_improved(achieved, adjusted_original) && !selection.met_target {
        branches.push(RecursiveBranchPlan {
            kind: RecursiveBranchKind::OriginalRoot,
            positive_count: old_pos,
            negative_count: old_neg,
            req_diff: DelayTime::new(0.0, 0.0),
            max_input_load: input.max_input_load,
        });
    }

    branches
}

pub fn sort_fanouts_like_sis(fanouts: &[Fanout]) -> Vec<Fanout> {
    let mut sorted = fanouts.to_vec();
    sorted.sort_by(compare_fanout);
    sorted
}

pub fn compare_fanout(left: &Fanout, right: &Fanout) -> Ordering {
    left.phase
        .sort_rank()
        .cmp(&right.phase.sort_rank())
        .then_with(|| {
            left.required
                .min_edge()
                .partial_cmp(&right.required.min_edge())
                .unwrap_or(Ordering::Equal)
        })
}

pub fn cumulative_capacitance(
    sorted_fanouts: &[Fanout],
    num_pos: usize,
    num_neg: usize,
    min_req_diff: f64,
) -> Result<CumulativeCapacitance, BufRecurError> {
    validate_sorted_counts(sorted_fanouts, num_pos, num_neg)?;

    let mut cap_k_pos = vec![0.0; num_pos + 1];
    let mut cap_k_neg = vec![0.0; num_neg + 1];
    let mut pos_spread = RequiredSpread::default();
    let mut neg_spread = RequiredSpread::default();

    for (index, fanout) in sorted_fanouts[..num_pos].iter().enumerate() {
        cap_k_pos[index + 1] = cap_k_pos[index] + fanout.load;
        pos_spread.include(fanout.required);
    }
    for (index, fanout) in sorted_fanouts[num_pos..num_pos + num_neg]
        .iter()
        .enumerate()
    {
        cap_k_neg[index + 1] = cap_k_neg[index] + fanout.load;
        neg_spread.include(fanout.required);
    }

    let total_cap_pos = cap_k_pos[num_pos];
    let total_cap_neg = cap_k_neg[num_neg];
    let cap_l_pos = cap_k_pos.iter().map(|cap| total_cap_pos - cap).collect();
    let cap_l_neg = cap_k_neg.iter().map(|cap| total_cap_neg - cap).collect();
    let do_unbalanced = pos_spread.exceeds(min_req_diff) || neg_spread.exceeds(min_req_diff);

    Ok(CumulativeCapacitance {
        cap_k_pos,
        cap_k_neg,
        cap_l_pos,
        cap_l_neg,
        do_unbalanced,
    })
}

#[derive(Clone, Copy, Debug)]
struct RequiredSpread {
    min_rise: f64,
    max_rise: f64,
    min_fall: f64,
    max_fall: f64,
    seen: bool,
}

impl Default for RequiredSpread {
    fn default() -> Self {
        Self {
            min_rise: POS_LARGE,
            max_rise: NEG_LARGE,
            min_fall: POS_LARGE,
            max_fall: NEG_LARGE,
            seen: false,
        }
    }
}

impl RequiredSpread {
    fn include(&mut self, required: DelayTime) {
        self.seen = true;
        self.min_rise = self.min_rise.min(required.rise);
        self.max_rise = self.max_rise.max(required.rise);
        self.min_fall = self.min_fall.min(required.fall);
        self.max_fall = self.max_fall.max(required.fall);
    }

    fn exceeds(self, min_req_diff: f64) -> bool {
        self.seen
            && (self.max_rise - self.min_rise) > min_req_diff
            && (self.max_fall - self.min_fall) > min_req_diff
    }
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

pub fn load_constraint_met(node: Option<&AnnotatedLoad>) -> bool {
    let Some(node) = node else {
        return true;
    };

    match node.implementation {
        LoadImplementation::None => true,
        LoadImplementation::Buffer { max_load } | LoadImplementation::Gate { max_load } => {
            max_load > node.load
        }
    }
}

pub fn sp_buffer_recur_network_bound<Network>(
    _network: &mut Network,
) -> Result<BufferRecurPlan, BufRecurError> {
    Err(BufRecurError::MissingSisDependency {
        operation: "sp_buffer_recur",
        missing: REQUIRED_PORT_BEADS,
    })
}

fn validate_input(input: &BufferRecurInput) -> Result<(), BufRecurError> {
    if input.positive_count + input.negative_count != input.fanouts.len() {
        return Err(BufRecurError::InvalidFanoutCounts {
            positive: input.positive_count,
            negative: input.negative_count,
            fanouts: input.fanouts.len(),
        });
    }
    if input.inverting_buffer_count > input.buffers.len() {
        return Err(BufRecurError::InvalidBufferCounts {
            inverting: input.inverting_buffer_count,
            total: input.buffers.len(),
        });
    }
    if input.buffers.is_empty() {
        return Err(BufRecurError::NoBuffers);
    }
    if input.gate_versions.is_empty() {
        return Err(BufRecurError::NoGateVersions);
    }
    if input.buffers.iter().any(|buffer| {
        !buffer.area.is_finite()
            || !buffer.input_load.is_finite()
            || !buffer.max_load.is_finite()
            || !buffer.block.rise.is_finite()
            || !buffer.block.fall.is_finite()
            || !buffer.drive.rise.is_finite()
            || !buffer.drive.fall.is_finite()
    }) || input.gate_versions.iter().any(|gate| {
        !gate.area.is_finite()
            || !gate.input_load.is_finite()
            || !gate.output_load_limit.is_finite()
            || !gate.block.rise.is_finite()
            || !gate.block.fall.is_finite()
            || !gate.drive.rise.is_finite()
            || !gate.drive.fall.is_finite()
    }) || input.fanouts.iter().any(|fanout| {
        !fanout.required.rise.is_finite()
            || !fanout.required.fall.is_finite()
            || !fanout.load.is_finite()
    }) {
        return Err(BufRecurError::NonFiniteDelayData);
    }
    Ok(())
}

fn validate_sorted_counts(
    sorted_fanouts: &[Fanout],
    num_pos: usize,
    num_neg: usize,
) -> Result<(), BufRecurError> {
    if num_pos + num_neg != sorted_fanouts.len() {
        return Err(BufRecurError::InvalidFanoutCounts {
            positive: num_pos,
            negative: num_neg,
            fanouts: sorted_fanouts.len(),
        });
    }
    if sorted_fanouts[..num_pos]
        .iter()
        .any(|fanout| fanout.phase != FanoutPhase::Positive)
        || sorted_fanouts[num_pos..]
            .iter()
            .any(|fanout| fanout.phase != FanoutPhase::Negative)
    {
        return Err(BufRecurError::FanoutsNotPhasePartitioned);
    }
    Ok(())
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

fn delay_difference(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise - right.rise, left.fall - right.fall)
}

fn positive_margin(margin: DelayTime) -> bool {
    margin.rise > EPS && margin.fall > EPS
}

fn req_equal(left: DelayTime, right: DelayTime) -> bool {
    (left.rise - right.rise).abs() < V_SMALL && (left.fall - right.fall).abs() < V_SMALL
}

fn req_improved(left: DelayTime, right: DelayTime) -> bool {
    (left.rise - right.rise) > V_SMALL && (left.fall - right.fall) > V_SMALL
}

#[derive(Clone, Debug, PartialEq)]
pub enum BufRecurError {
    InvalidFanoutCounts {
        positive: usize,
        negative: usize,
        fanouts: usize,
    },
    InvalidBufferCounts {
        inverting: usize,
        total: usize,
    },
    FanoutsNotPhasePartitioned,
    NoBuffers,
    NoGateVersions,
    NonFiniteDelayData,
    MissingSisDependency {
        operation: &'static str,
        missing: &'static [&'static str],
    },
}

impl fmt::Display for BufRecurError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFanoutCounts {
                positive,
                negative,
                fanouts,
            } => write!(
                f,
                "invalid buffer-recursion fanout counts: positive {positive} + negative {negative} != fanouts {fanouts}",
            ),
            Self::InvalidBufferCounts { inverting, total } => write!(
                f,
                "invalid buffer library counts: inverting buffer count {inverting} exceeds total buffers {total}",
            ),
            Self::FanoutsNotPhasePartitioned => {
                write!(
                    f,
                    "fanouts must be sorted with positive phase entries before negative phase entries"
                )
            }
            Self::NoBuffers => write!(
                f,
                "buffer recursion requires at least one buffer implementation"
            ),
            Self::NoGateVersions => write!(
                f,
                "buffer recursion requires at least one root gate version"
            ),
            Self::NonFiniteDelayData => write!(
                f,
                "buffer recursion delay, load, and area data must be finite"
            ),
            Self::MissingSisDependency { operation, missing } => write!(
                f,
                "{operation} requires native Rust SIS node/network/delay/library ports: {}",
                missing.join(", ")
            ),
        }
    }
}

impl Error for BufRecurError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn inv(name: &str, area: f64, input_load: f64, drive: f64) -> BufferCell {
        BufferCell {
            name: name.to_owned(),
            depth: 1,
            area,
            input_load,
            max_load: 9.0,
            phase: Phase::Inverting,
            block: DelayTime::new(0.2, 0.2),
            drive: DelayTime::new(drive, drive),
        }
    }

    fn buf(name: &str, area: f64, input_load: f64, drive: f64) -> BufferCell {
        BufferCell {
            name: name.to_owned(),
            depth: 1,
            area,
            input_load,
            max_load: 9.0,
            phase: Phase::NonInverting,
            block: DelayTime::new(0.2, 0.2),
            drive: DelayTime::new(drive, drive),
        }
    }

    fn gate(name: &str, area: f64, input_load: f64, drive: f64) -> GateVersion {
        GateVersion {
            name: name.to_owned(),
            area,
            input_load,
            output_load_limit: 10.0,
            phase: Phase::NonInverting,
            block: DelayTime::new(0.1, 0.1),
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

    fn base_input() -> BufferRecurInput {
        BufferRecurInput {
            fanouts: vec![
                fanout(1, FanoutPhase::Positive, 8.0, 1.0),
                fanout(2, FanoutPhase::Positive, 20.0, 1.0),
                fanout(3, FanoutPhase::Negative, 7.5, 1.0),
                fanout(4, FanoutPhase::Negative, 18.0, 1.0),
            ],
            positive_count: 2,
            negative_count: 2,
            buffers: vec![inv("inv", 1.0, 0.5, 0.05), buf("buf", 1.2, 0.6, 0.05)],
            inverting_buffer_count: 1,
            gate_versions: vec![gate("g0", 5.0, 0.8, 0.08), gate("g1", 7.0, 0.7, 0.04)],
            req_diff: DelayTime::new(1.0, 1.0),
            original_required: DelayTime::new(6.0, 6.0),
            original_load: 2.0,
            previous_drive: DelayTime::new(0.1, 0.1),
            max_input_load: 1.0,
            auto_route: 0.1,
            min_req_diff: 2.0,
            mode: DEFAULT_BUFFER_MODE,
            current_level: 1,
            root_is_original_node: true,
            do_decomp: true,
            interactive: false,
            alternate_input_slack_would_fail: false,
        }
    }

    #[test]
    fn delay_math_matches_buf_delay_c_phase_rules() {
        assert_eq!(
            subtract_delay(
                Phase::NonInverting,
                DelayTime::new(1.0, 2.0),
                DelayTime::new(0.5, 0.25),
                4.0,
                DelayTime::new(10.0, 12.0),
            ),
            DelayTime::new(7.0, 9.0)
        );
        assert_eq!(
            subtract_delay(
                Phase::Inverting,
                DelayTime::new(1.0, 2.0),
                DelayTime::new(0.5, 0.25),
                4.0,
                DelayTime::new(10.0, 12.0),
            ),
            DelayTime::new(9.0, 7.0)
        );
        assert_eq!(
            compute_required_time(
                Phase::Neither,
                DelayTime::new(10.0, 12.0),
                DelayTime::new(3.0, 3.0),
            ),
            DelayTime::new(7.0, 7.0)
        );
    }

    #[test]
    fn fanout_sort_and_cumulative_capacitance_match_c_tables() {
        let fanouts = vec![
            fanout(3, FanoutPhase::Negative, 9.0, 3.0),
            fanout(1, FanoutPhase::Positive, 8.0, 1.0),
            fanout(2, FanoutPhase::Positive, 12.0, 2.0),
            fanout(4, FanoutPhase::Negative, 15.0, 4.0),
        ];

        let sorted = sort_fanouts_like_sis(&fanouts);
        assert_eq!(
            sorted.iter().map(|fanout| fanout.id).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );

        let caps = cumulative_capacitance(&sorted, 2, 2, 2.0).unwrap();
        assert_eq!(caps.cap_k_pos, vec![0.0, 1.0, 3.0]);
        assert_eq!(caps.cap_l_pos, vec![3.0, 2.0, 0.0]);
        assert_eq!(caps.cap_k_neg, vec![0.0, 3.0, 7.0]);
        assert_eq!(caps.cap_l_neg, vec![7.0, 4.0, 0.0]);
        assert!(caps.do_unbalanced);
    }

    #[test]
    fn unbalanced_selection_prefers_smallest_area_that_meets_target() {
        let input = base_input();
        let sorted = input.sorted_fanouts();
        let caps = cumulative_capacitance(
            &sorted,
            input.positive_count,
            input.negative_count,
            input.min_req_diff,
        )
        .unwrap();
        let adjusted_original = drive_adjustment(
            input.previous_drive,
            input.original_load,
            input.original_required,
        );
        let target = DelayTime::new(adjusted_original.rise + 1.0, adjusted_original.fall + 1.0);

        let selection = select_unbalanced_trans3(&input, &sorted, &caps, adjusted_original, target)
            .unwrap()
            .expect("expected a feasible unbalanced transform");

        assert!(selection.met_target);
        assert_eq!(selection.gate_index, 0);
        assert_eq!(selection.b_index, 0);
        assert_eq!(selection.big_b_index, 0);
        assert!(!selection.use_noninverting_buffers);
        assert!(selection.area < 8.0);
    }

    #[test]
    fn planner_returns_recursive_unbalanced_action() {
        let plan = plan_buffer_recursion(&base_input()).unwrap();

        let BufferRecurAction::UseUnbalancedTrans3 {
            selection,
            recursive_branches,
        } = plan.action
        else {
            panic!("expected unbalanced trans3 plan");
        };

        assert!(selection.met_target);
        assert!(
            recursive_branches
                .iter()
                .all(|branch| branch.req_diff.rise >= 0.0 && branch.req_diff.fall >= 0.0)
        );
    }

    #[test]
    fn planner_handles_terminal_and_fallback_cases() {
        let mut no_work = base_input();
        no_work.req_diff = DelayTime::new(0.0, 0.0);
        assert_eq!(
            plan_buffer_recursion(&no_work).unwrap().action,
            BufferRecurAction::NoChangeRequired
        );

        let mut single = base_input();
        single.fanouts = vec![fanout(1, FanoutPhase::Positive, 8.0, 1.0)];
        single.positive_count = 1;
        single.negative_count = 0;
        assert_eq!(
            plan_buffer_recursion(&single).unwrap().action,
            BufferRecurAction::ReplaceSingleFanoutCell
        );

        let mut balanced = base_input();
        balanced.mode = BALANCED_MASK;
        assert_eq!(
            plan_buffer_recursion(&balanced).unwrap().action,
            BufferRecurAction::TryBalancedDecomposition {
                reason: BalancedFallbackReason::UnbalancedNotAllowed
            }
        );
    }

    #[test]
    fn load_constraint_and_dependency_errors_are_explicit() {
        assert!(load_constraint_met(None));
        assert!(load_constraint_met(Some(&AnnotatedLoad {
            implementation: LoadImplementation::None,
            load: 100.0,
        })));
        assert!(!load_constraint_met(Some(&AnnotatedLoad {
            implementation: LoadImplementation::Buffer { max_load: 3.0 },
            load: 3.0,
        })));
        assert!(load_constraint_met(Some(&AnnotatedLoad {
            implementation: LoadImplementation::Gate { max_load: 3.1 },
            load: 3.0,
        })));

        let mut network = ();
        assert_eq!(
            sp_buffer_recur_network_bound(&mut network),
            Err(BufRecurError::MissingSisDependency {
                operation: "sp_buffer_recur",
                missing: REQUIRED_PORT_BEADS,
            })
        );
    }
}
