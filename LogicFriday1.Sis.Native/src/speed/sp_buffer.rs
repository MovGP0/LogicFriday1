//! Native Rust port scaffold for `sis/speed/sp_buffer.c`.
//!
//! The original module drives mapped buffering over SIS networks and library
//! gates. Full mutation depends on the unported buffer recursion, delay, node,
//! network, and library layers. This module ports the independent decision
//! rules used by that flow: performance comparison, fanout-set expansion,
//! target load/required-time math, max-load checks, and area-recovery ranking.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;
pub const NEG_LARGE: f64 = -10_000.0;
pub const V_SMALL: f64 = 0.000001;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub fn max_edge(self) -> f64 {
        self.rise.max(self.fall)
    }

    pub fn min_edge(self) -> f64 {
        self.rise.min(self.fall)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferOptions {
    pub limit: usize,
    pub trace: bool,
    pub threshold: f64,
    pub single_pass: bool,
    pub do_decomp: bool,
}

pub fn normalize_changed(changed: i32) -> i32 {
    if changed == 0 { 0 } else { 1 }
}

pub fn req_improved(newer: DelayTime, older: DelayTime) -> bool {
    (newer.rise - older.rise) > V_SMALL && (newer.fall - older.fall) > V_SMALL
}

pub fn performance_worsened(
    required_times_set: bool,
    previous: DelayTime,
    current: DelayTime,
) -> bool {
    if required_times_set {
        current.min_edge() < previous.min_edge()
    } else {
        current.max_edge() > previous.max_edge()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Inverter,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutNode {
    pub id: usize,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub fanouts: Vec<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutPhase {
    NonInverting,
    Inverting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutTarget {
    pub fanout: usize,
    pub pin: usize,
    pub phase: FanoutPhase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutSet {
    pub root: usize,
    pub inverter_node: Option<usize>,
    pub positive_count: usize,
    pub negative_count: usize,
    pub targets: Vec<FanoutTarget>,
}

pub fn fanout_set(root: &FanoutNode, all_nodes: &[FanoutNode]) -> FanoutSet {
    let mut number_inv = 0usize;
    let mut inverter_node = None;
    let mut positive_count = 0usize;
    let mut negative_count = 0usize;

    for (fanout_id, _) in &root.fanouts {
        if all_nodes
            .iter()
            .find(|node| node.id == *fanout_id)
            .is_some_and(|node| node.function == NodeFunction::Inverter)
        {
            number_inv += 1;
            inverter_node = Some(*fanout_id);
            negative_count += all_nodes
                .iter()
                .find(|node| node.id == *fanout_id)
                .map_or(0, |node| node.fanouts.len());
        } else {
            positive_count += 1;
        }
    }

    let expand_single_inverter = number_inv == 1 && negative_count > 0;
    if !expand_single_inverter {
        if number_inv > 0 {
            positive_count += number_inv;
        }
        inverter_node = None;
        negative_count = 0;
    }

    let mut targets = Vec::new();
    for (fanout_id, pin) in &root.fanouts {
        let fanout = all_nodes.iter().find(|node| node.id == *fanout_id);
        if expand_single_inverter
            && fanout.is_some_and(|node| node.function == NodeFunction::Inverter)
        {
            if let Some(inv) = fanout {
                for (inv_fanout, inv_pin) in &inv.fanouts {
                    targets.push(FanoutTarget {
                        fanout: *inv_fanout,
                        pin: *inv_pin,
                        phase: FanoutPhase::Inverting,
                    });
                }
            }
        } else {
            targets.push(FanoutTarget {
                fanout: *fanout_id,
                pin: *pin,
                phase: FanoutPhase::NonInverting,
            });
        }
    }

    FanoutSet {
        root: root.id,
        inverter_node,
        positive_count,
        negative_count,
        targets,
    }
}

pub fn target_for_primary_input(
    max_input_load: Option<f64>,
    constrained_slack: Option<DelayTime>,
) -> (DelayTime, f64) {
    let max_load = max_input_load.unwrap_or(POS_LARGE);
    let req_diff = constrained_slack
        .map(|slack| DelayTime::new(-slack.rise, -slack.fall))
        .unwrap_or_else(|| DelayTime::new(POS_LARGE, POS_LARGE));
    (req_diff, max_load)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaninTargetTiming {
    pub fanin: usize,
    pub critical: bool,
    pub wire_required: DelayTime,
    pub arrival: DelayTime,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiblingFanoutTiming {
    pub fanout: usize,
    pub pin: usize,
    pub wire_required: DelayTime,
    pub arrival: DelayTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferTargetInput {
    pub node: usize,
    pub node_kind: NodeKind,
    pub constrained: bool,
    pub primary_input_max_load: Option<f64>,
    pub primary_input_slack: Option<DelayTime>,
    pub fanins: Vec<FaninTargetTiming>,
    pub annotated_critical_fanin_index: Option<usize>,
    pub critical_fanin_sibling_fanouts: Vec<SiblingFanoutTiming>,
    pub pin_load: f64,
    pub previous_drive: DelayTime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetReason {
    PrimaryInput,
    NotCritical,
    CriticalFanin,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BufferTarget {
    pub req_diff: DelayTime,
    pub max_load: f64,
    pub critical_fanin_index: Option<usize>,
    pub reason: TargetReason,
}

pub fn buffer_target(input: &BufferTargetInput) -> Result<BufferTarget, SpBufferError> {
    if input.node_kind == NodeKind::PrimaryInput {
        let constrained_slack = if input.constrained {
            input.primary_input_slack
        } else {
            None
        };
        let (req_diff, max_load) =
            target_for_primary_input(input.primary_input_max_load, constrained_slack);
        return Ok(BufferTarget {
            req_diff,
            max_load,
            critical_fanin_index: None,
            reason: TargetReason::PrimaryInput,
        });
    }

    let mut min_slack = DelayTime::new(POS_LARGE, POS_LARGE);
    let mut critical_fanin_index = None;
    for (index, fanin) in input.fanins.iter().enumerate() {
        if fanin.critical {
            let slack = DelayTime::new(
                fanin.wire_required.rise - fanin.arrival.rise,
                fanin.wire_required.fall - fanin.arrival.fall,
            );
            if slack.min_edge() < min_slack.min_edge() {
                critical_fanin_index = Some(index);
            }
            min_slack = DelayTime::new(
                min_slack.rise.min(slack.rise),
                min_slack.fall.min(slack.fall),
            );
        }
    }

    let Some(index) = critical_fanin_index else {
        return Ok(BufferTarget {
            req_diff: DelayTime::new(0.0, 0.0),
            max_load: POS_LARGE,
            critical_fanin_index: None,
            reason: TargetReason::NotCritical,
        });
    };

    if let Some(annotated) = input.annotated_critical_fanin_index {
        if annotated != index {
            return Err(SpBufferError::CriticalFaninMismatch {
                annotated,
                computed: index,
            });
        }
    }

    let req_diff = if input.constrained {
        DelayTime::new(-min_slack.rise, -min_slack.fall)
    } else {
        DelayTime::new(POS_LARGE, POS_LARGE)
    };
    let min_fanout_slack = input
        .critical_fanin_sibling_fanouts
        .iter()
        .filter(|fanout| fanout.fanout != input.node)
        .fold(DelayTime::new(POS_LARGE, POS_LARGE), |best, fanout| {
            let slack = DelayTime::new(
                fanout.wire_required.rise - fanout.arrival.rise,
                fanout.wire_required.fall - fanout.arrival.fall,
            );
            DelayTime::new(best.rise.min(slack.rise), best.fall.min(slack.fall))
        });

    Ok(BufferTarget {
        req_diff,
        max_load: target_load_from_slack(
            input.pin_load,
            min_fanout_slack,
            min_slack,
            input.previous_drive,
        ),
        critical_fanin_index: Some(index),
        reason: TargetReason::CriticalFanin,
    })
}

pub fn target_load_from_slack(
    pin_load: f64,
    min_fanout_slack: DelayTime,
    min_slack: DelayTime,
    previous_drive: DelayTime,
) -> f64 {
    let diff = min_fanout_slack.min_edge() - min_slack.min_edge();
    let load = if diff > 0.0 {
        diff / previous_drive.max_edge()
    } else {
        V_SMALL
    };
    pin_load + load
}

pub fn max_load_violation(load: f64, input_limits: &[f64]) -> bool {
    input_limits.iter().any(|limit| load > *limit)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferMark {
    None,
    Buffer,
    Gate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BufferedGraphNode {
    pub id: usize,
    pub buffer_mark: BufferMark,
    pub fanouts: Vec<usize>,
}

pub fn mark_already_buffered(nodes: &mut [BufferedGraphNode], start: usize) -> Vec<usize> {
    let mut visited = Vec::new();
    mark_already_buffered_from(nodes, start, &mut visited);
    visited
}

fn mark_already_buffered_from(
    nodes: &mut [BufferedGraphNode],
    node_id: usize,
    visited: &mut Vec<usize>,
) {
    if visited.contains(&node_id) {
        return;
    }

    let Some(index) = nodes.iter().position(|node| node.id == node_id) else {
        return;
    };
    let fanouts = nodes[index].fanouts.clone();

    for fanout_id in fanouts {
        let Some(fanout_index) = nodes.iter().position(|node| node.id == fanout_id) else {
            continue;
        };
        if nodes[fanout_index].buffer_mark != BufferMark::None {
            nodes[fanout_index].buffer_mark = BufferMark::None;
            mark_already_buffered_from(nodes, fanout_id, visited);
        }
    }

    visited.push(node_id);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayBufferNode {
    pub id: usize,
    pub kind: NodeKind,
    pub fanouts: Vec<usize>,
    pub buffer_node_changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayBufferPlan {
    pub changed: bool,
    pub delay_trace_count: usize,
    pub buffered_nodes: Vec<usize>,
}

pub fn plan_array_buffering(nodes: &[ArrayBufferNode]) -> ArrayBufferPlan {
    let mut changed = false;
    let mut delay_trace_count = 0usize;
    let mut buffered_nodes = Vec::new();

    for node in nodes {
        if node.kind == NodeKind::PrimaryOutput {
            continue;
        }

        if node
            .fanouts
            .iter()
            .any(|fanout| buffered_nodes.contains(fanout))
        {
            delay_trace_count += 1;
            buffered_nodes.clear();
        }

        if node.buffer_node_changed {
            changed = true;
            buffered_nodes.push(node.id);
        }
    }

    ArrayBufferPlan {
        changed,
        delay_trace_count,
        buffered_nodes,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaxLoadNode<'a> {
    pub id: usize,
    pub kind: NodeKind,
    pub load: f64,
    pub input_limits: &'a [f64],
}

pub fn max_load_redo_nodes(nodes: &[MaxLoadNode<'_>]) -> Vec<usize> {
    nodes
        .iter()
        .filter(|node| {
            node.kind == NodeKind::Internal && max_load_violation(node.load, node.input_limits)
        })
        .map(|node| node.id)
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BufferCost {
    pub slack: DelayTime,
    pub area: f64,
}

pub fn current_version_is_better(
    candidate: BufferCost,
    current_best: BufferCost,
    min_slack: f64,
) -> bool {
    let mut slack1 = candidate.slack.min_edge();
    let mut slack2 = current_best.slack.min_edge();
    let mut diff = slack1 - slack2;

    if (slack1 - min_slack).abs() < V_SMALL {
        slack1 = min_slack;
    }
    if (slack2 - min_slack).abs() < V_SMALL {
        slack2 = min_slack;
    }
    if diff.abs() < V_SMALL {
        diff = 0.0;
    }

    if slack2 < min_slack {
        diff > 0.0
    } else if slack1 < min_slack {
        false
    } else {
        candidate.area < current_best.area
    }
}

pub fn speed_buffer_network_bound() -> Result<(), SpBufferError> {
    Err(SpBufferError::MissingDependency(
        "speed_buffer_network requires native buffer recursion, buffer delay/util, mapped library, node/network traversal, and delay trace ports",
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpBufferError {
    MissingDependency(&'static str),
    CriticalFaninMismatch { annotated: usize, computed: usize },
}

impl fmt::Display for SpBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDependency(message) => write!(f, "{message}"),
            Self::CriticalFaninMismatch {
                annotated,
                computed,
            } => write!(
                f,
                "annotated critical fanin index {annotated} does not match computed index {computed}",
            ),
        }
    }
}

impl Error for SpBufferError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn performance_helpers_match_buffer_macros() {
        assert_eq!(normalize_changed(0), 0);
        assert_eq!(normalize_changed(3), 1);
        assert!(req_improved(
            DelayTime::new(2.0, 3.0),
            DelayTime::new(1.0, 1.5)
        ));
        assert!(performance_worsened(
            true,
            DelayTime::new(1.0, 2.0),
            DelayTime::new(0.5, 4.0)
        ));
        assert!(performance_worsened(
            false,
            DelayTime::new(1.0, 2.0),
            DelayTime::new(1.0, 3.0)
        ));
    }

    #[test]
    fn fanout_set_expands_through_one_inverter_only() {
        let root = FanoutNode {
            id: 1,
            kind: NodeKind::Internal,
            function: NodeFunction::Other,
            fanouts: vec![(2, 0), (3, 1)],
        };
        let inv = FanoutNode {
            id: 2,
            kind: NodeKind::Internal,
            function: NodeFunction::Inverter,
            fanouts: vec![(4, 0), (5, 1)],
        };
        let other = FanoutNode {
            id: 3,
            kind: NodeKind::Internal,
            function: NodeFunction::Other,
            fanouts: vec![],
        };

        let set = fanout_set(&root, &[root.clone(), inv, other]);

        assert_eq!(set.inverter_node, Some(2));
        assert_eq!(set.positive_count, 1);
        assert_eq!(set.negative_count, 2);
        assert_eq!(
            set.targets,
            vec![
                FanoutTarget {
                    fanout: 4,
                    pin: 0,
                    phase: FanoutPhase::Inverting,
                },
                FanoutTarget {
                    fanout: 5,
                    pin: 1,
                    phase: FanoutPhase::Inverting,
                },
                FanoutTarget {
                    fanout: 3,
                    pin: 1,
                    phase: FanoutPhase::NonInverting,
                },
            ]
        );
    }

    #[test]
    fn fanout_set_keeps_dead_or_multiple_inverters_as_positive_fanouts() {
        let root = FanoutNode {
            id: 1,
            kind: NodeKind::Internal,
            function: NodeFunction::Other,
            fanouts: vec![(2, 0), (3, 1)],
        };
        let dead_inv = FanoutNode {
            id: 2,
            kind: NodeKind::Internal,
            function: NodeFunction::Inverter,
            fanouts: vec![],
        };
        let other = FanoutNode {
            id: 3,
            kind: NodeKind::Internal,
            function: NodeFunction::Other,
            fanouts: vec![],
        };

        let set = fanout_set(&root, &[root.clone(), dead_inv, other]);

        assert_eq!(set.inverter_node, None);
        assert_eq!(set.positive_count, 2);
        assert_eq!(set.negative_count, 0);
        assert_eq!(
            set.targets,
            vec![
                FanoutTarget {
                    fanout: 2,
                    pin: 0,
                    phase: FanoutPhase::NonInverting,
                },
                FanoutTarget {
                    fanout: 3,
                    pin: 1,
                    phase: FanoutPhase::NonInverting,
                },
            ]
        );

        let first_inv = FanoutNode {
            id: 2,
            kind: NodeKind::Internal,
            function: NodeFunction::Inverter,
            fanouts: vec![(6, 0)],
        };
        let second_inv = FanoutNode {
            id: 4,
            kind: NodeKind::Internal,
            function: NodeFunction::Inverter,
            fanouts: vec![(5, 0)],
        };
        let multi_root = FanoutNode {
            id: 1,
            kind: NodeKind::Internal,
            function: NodeFunction::Other,
            fanouts: vec![(2, 0), (4, 1)],
        };

        let multi = fanout_set(&multi_root, &[multi_root.clone(), first_inv, second_inv]);

        assert_eq!(multi.inverter_node, None);
        assert_eq!(multi.positive_count, 2);
        assert_eq!(multi.negative_count, 0);
        assert_eq!(multi.targets.len(), 2);
    }

    #[test]
    fn target_helpers_match_c_default_and_load_formula() {
        assert_eq!(
            target_for_primary_input(Some(4.0), Some(DelayTime::new(-1.0, 2.0))),
            (DelayTime::new(1.0, -2.0), 4.0)
        );
        assert_eq!(
            target_load_from_slack(
                2.0,
                DelayTime::new(5.0, 6.0),
                DelayTime::new(1.0, 2.0),
                DelayTime::new(2.0, 4.0)
            ),
            3.0
        );
    }

    #[test]
    fn buffer_target_matches_primary_input_and_internal_critical_path_rules() {
        let primary = buffer_target(&BufferTargetInput {
            node: 1,
            node_kind: NodeKind::PrimaryInput,
            constrained: true,
            primary_input_max_load: Some(3.5),
            primary_input_slack: Some(DelayTime::new(-2.0, 1.0)),
            fanins: vec![],
            annotated_critical_fanin_index: None,
            critical_fanin_sibling_fanouts: vec![],
            pin_load: 0.0,
            previous_drive: DelayTime::new(1.0, 1.0),
        })
        .unwrap();

        assert_eq!(
            primary,
            BufferTarget {
                req_diff: DelayTime::new(2.0, -1.0),
                max_load: 3.5,
                critical_fanin_index: None,
                reason: TargetReason::PrimaryInput,
            }
        );

        let internal = buffer_target(&BufferTargetInput {
            node: 10,
            node_kind: NodeKind::Internal,
            constrained: true,
            primary_input_max_load: None,
            primary_input_slack: None,
            fanins: vec![
                FaninTargetTiming {
                    fanin: 1,
                    critical: true,
                    wire_required: DelayTime::new(9.0, 8.0),
                    arrival: DelayTime::new(3.0, 3.0),
                },
                FaninTargetTiming {
                    fanin: 2,
                    critical: true,
                    wire_required: DelayTime::new(5.0, 6.0),
                    arrival: DelayTime::new(4.0, 2.0),
                },
            ],
            annotated_critical_fanin_index: Some(1),
            critical_fanin_sibling_fanouts: vec![
                SiblingFanoutTiming {
                    fanout: 10,
                    pin: 0,
                    wire_required: DelayTime::new(5.0, 6.0),
                    arrival: DelayTime::new(4.0, 2.0),
                },
                SiblingFanoutTiming {
                    fanout: 11,
                    pin: 0,
                    wire_required: DelayTime::new(8.0, 9.0),
                    arrival: DelayTime::new(4.0, 2.0),
                },
            ],
            pin_load: 1.25,
            previous_drive: DelayTime::new(1.0, 2.0),
        })
        .unwrap();

        assert_eq!(internal.req_diff, DelayTime::new(-1.0, -4.0));
        assert_eq!(internal.max_load, 2.75);
        assert_eq!(internal.critical_fanin_index, Some(1));
        assert_eq!(internal.reason, TargetReason::CriticalFanin);
    }

    #[test]
    fn buffer_target_detects_critical_fanin_annotation_mismatch() {
        let result = buffer_target(&BufferTargetInput {
            node: 10,
            node_kind: NodeKind::Internal,
            constrained: false,
            primary_input_max_load: None,
            primary_input_slack: None,
            fanins: vec![FaninTargetTiming {
                fanin: 1,
                critical: true,
                wire_required: DelayTime::new(4.0, 4.0),
                arrival: DelayTime::new(3.0, 3.0),
            }],
            annotated_critical_fanin_index: Some(1),
            critical_fanin_sibling_fanouts: vec![],
            pin_load: 0.0,
            previous_drive: DelayTime::new(1.0, 1.0),
        });

        assert_eq!(
            result,
            Err(SpBufferError::CriticalFaninMismatch {
                annotated: 1,
                computed: 0,
            })
        );
    }

    #[test]
    fn max_load_and_area_recovery_comparison_match_c_rules() {
        assert!(max_load_violation(3.0, &[4.0, 2.5]));
        assert!(!max_load_violation(2.0, &[4.0, 2.5]));
        assert_eq!(
            max_load_redo_nodes(&[
                MaxLoadNode {
                    id: 1,
                    kind: NodeKind::PrimaryInput,
                    load: 9.0,
                    input_limits: &[1.0],
                },
                MaxLoadNode {
                    id: 2,
                    kind: NodeKind::Internal,
                    load: 3.0,
                    input_limits: &[4.0, 2.5],
                },
                MaxLoadNode {
                    id: 3,
                    kind: NodeKind::Internal,
                    load: 2.0,
                    input_limits: &[4.0, 2.5],
                },
            ]),
            vec![2]
        );

        assert!(current_version_is_better(
            BufferCost {
                slack: DelayTime::new(2.0, 2.0),
                area: 3.0,
            },
            BufferCost {
                slack: DelayTime::new(2.0, 2.0),
                area: 4.0,
            },
            1.0,
        ));
        assert!(!current_version_is_better(
            BufferCost {
                slack: DelayTime::new(0.5, 2.0),
                area: 1.0,
            },
            BufferCost {
                slack: DelayTime::new(2.0, 2.0),
                area: 4.0,
            },
            1.0,
        ));
    }

    #[test]
    fn already_buffered_and_array_buffering_follow_c_control_flow() {
        let mut graph = vec![
            BufferedGraphNode {
                id: 1,
                buffer_mark: BufferMark::Gate,
                fanouts: vec![2, 3],
            },
            BufferedGraphNode {
                id: 2,
                buffer_mark: BufferMark::Buffer,
                fanouts: vec![4],
            },
            BufferedGraphNode {
                id: 3,
                buffer_mark: BufferMark::None,
                fanouts: vec![5],
            },
            BufferedGraphNode {
                id: 4,
                buffer_mark: BufferMark::Gate,
                fanouts: vec![],
            },
        ];

        let visited = mark_already_buffered(&mut graph, 1);

        assert_eq!(visited, vec![4, 2, 1]);
        assert_eq!(graph[0].buffer_mark, BufferMark::Gate);
        assert_eq!(graph[1].buffer_mark, BufferMark::None);
        assert_eq!(graph[2].buffer_mark, BufferMark::None);
        assert_eq!(graph[3].buffer_mark, BufferMark::None);

        let plan = plan_array_buffering(&[
            ArrayBufferNode {
                id: 1,
                kind: NodeKind::Internal,
                fanouts: vec![2],
                buffer_node_changed: true,
            },
            ArrayBufferNode {
                id: 2,
                kind: NodeKind::Internal,
                fanouts: vec![1],
                buffer_node_changed: true,
            },
            ArrayBufferNode {
                id: 3,
                kind: NodeKind::PrimaryOutput,
                fanouts: vec![2],
                buffer_node_changed: true,
            },
        ]);

        assert_eq!(
            plan,
            ArrayBufferPlan {
                changed: true,
                delay_trace_count: 1,
                buffered_nodes: vec![2],
            }
        );
    }

    #[test]
    fn network_bound_entry_reports_missing_dependencies() {
        assert_eq!(
            speed_buffer_network_bound(),
            Err(SpBufferError::MissingDependency(
                "speed_buffer_network requires native buffer recursion, buffer delay/util, mapped library, node/network traversal, and delay trace ports",
            ))
        );
    }
}
