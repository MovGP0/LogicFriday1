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

    if number_inv > 1 || negative_count == 0 {
        if number_inv > 1 {
            positive_count += number_inv;
        }
        inverter_node = None;
        negative_count = 0;
    }

    let mut targets = Vec::new();
    for (fanout_id, pin) in &root.fanouts {
        let fanout = all_nodes.iter().find(|node| node.id == *fanout_id);
        if number_inv <= 1 && fanout.is_some_and(|node| node.function == NodeFunction::Inverter) {
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
}

impl fmt::Display for SpBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDependency(message) => write!(f, "{message}"),
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
    fn max_load_and_area_recovery_comparison_match_c_rules() {
        assert!(max_load_violation(3.0, &[4.0, 2.5]));
        assert!(!max_load_violation(2.0, &[4.0, 2.5]));

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
    fn network_bound_entry_reports_missing_dependencies() {
        assert_eq!(
            speed_buffer_network_bound(),
            Err(SpBufferError::MissingDependency(
                "speed_buffer_network requires native buffer recursion, buffer delay/util, mapped library, node/network traversal, and delay trace ports",
            ))
        );
    }
}
