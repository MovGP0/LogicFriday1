//! Native Rust port scaffold for `sis/speed/speed_no.c`.
//!
//! The original file drives speed decomposition by creating a temporary SIS
//! network from one node, repeatedly trying alternative decompositions, keeping
//! the network with the smallest output arrival time, optionally inserting
//! inverters, then converting the result back to an array of nodes. That
//! end-to-end flow depends on the unported SIS network, node, delay, and speed
//! decomposition layers. This module ports the attempt-selection and phase
//! adjustment decision rules while representing the network-bound entry points
//! as explicit blocked operations.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Buffer,
    Inverter,
    PrimaryOutput,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn worst(self) -> f64 {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedDecompOptions {
    pub coeff: f64,
    pub model: DelayModel,
    pub num_tries: usize,
    pub debug: bool,
    pub add_inv: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    UnitFanout,
    Mapped,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecompositionAttempt {
    pub attempt_index: usize,
    pub output_arrival: DelayTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BestAttempt {
    pub attempt_index: usize,
    pub delay: f64,
}

pub fn select_best_attempt(attempts: &[DecompositionAttempt]) -> Result<BestAttempt, SpeedNoError> {
    let mut best: Option<BestAttempt> = None;

    for attempt in attempts {
        let delay = attempt.output_arrival.worst();
        if best.as_ref().is_none_or(|current| delay < current.delay) {
            best = Some(BestAttempt {
                attempt_index: attempt.attempt_index,
                delay,
            });
        }
    }

    best.ok_or(SpeedNoError::NoAttempts)
}

pub fn format_attempt_trace(attempts: &[DecompositionAttempt]) -> String {
    if attempts.len() <= 1 {
        return String::new();
    }

    let best = select_best_attempt(attempts).ok();
    let mut trace = String::new();
    for attempt in attempts {
        trace.push_str(&format!(
            "{} => {:.2}\t",
            attempt.attempt_index,
            attempt.output_arrival.worst()
        ));
    }
    if let Some(best) = best {
        trace.push_str(&format!(" BEST is {}\n", best.attempt_index));
    }
    trace
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseNode {
    pub id: usize,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub fanouts: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhaseAction {
    CollapseIntoFanout { node: usize, fanout: usize },
    DeleteIfFanoutless { node: usize },
}

pub fn phase_adjustment_actions(nodes: &[PhaseNode]) -> Vec<PhaseAction> {
    let mut actions = Vec::new();

    for node in nodes {
        if node.kind != NodeKind::Internal {
            continue;
        }
        if !matches!(node.function, NodeFunction::Buffer | NodeFunction::Inverter) {
            continue;
        }

        let mut collapsible_fanouts = 0usize;
        for fanout in &node.fanouts {
            if nodes
                .iter()
                .find(|candidate| candidate.id == *fanout)
                .is_some_and(|candidate| candidate.function != NodeFunction::PrimaryOutput)
            {
                actions.push(PhaseAction::CollapseIntoFanout {
                    node: node.id,
                    fanout: *fanout,
                });
                collapsible_fanouts += 1;
            }
        }

        if node.fanouts.is_empty() || collapsible_fanouts == node.fanouts.len() {
            actions.push(PhaseAction::DeleteIfFanoutless { node: node.id });
        }
    }

    actions
}

pub fn speed_decomp_interface(
    _node_name: &str,
    _options: &SpeedDecompOptions,
) -> Result<(), SpeedNoError> {
    Err(SpeedNoError::MissingDependency(
        "speed_decomp_interface requires native speed_net, speed_delay, speedup, network, and node ports",
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpeedNoError {
    NoAttempts,
    MissingDependency(&'static str),
}

impl fmt::Display for SpeedNoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAttempts => write!(f, "speed decomposition had no attempts to compare"),
            Self::MissingDependency(message) => write!(f, "{message}"),
        }
    }
}

impl Error for SpeedNoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_lowest_max_rise_fall_delay_like_c_loop() {
        let attempts = vec![
            DecompositionAttempt {
                attempt_index: 0,
                output_arrival: DelayTime {
                    rise: 4.0,
                    fall: 7.0,
                },
            },
            DecompositionAttempt {
                attempt_index: 1,
                output_arrival: DelayTime {
                    rise: 5.0,
                    fall: 3.0,
                },
            },
            DecompositionAttempt {
                attempt_index: 2,
                output_arrival: DelayTime {
                    rise: 6.0,
                    fall: 6.5,
                },
            },
        ];

        assert_eq!(
            select_best_attempt(&attempts).unwrap(),
            BestAttempt {
                attempt_index: 1,
                delay: 5.0,
            }
        );
    }

    #[test]
    fn formats_debug_attempt_trace_with_best_index() {
        let attempts = vec![
            DecompositionAttempt {
                attempt_index: 0,
                output_arrival: DelayTime {
                    rise: 4.0,
                    fall: 7.0,
                },
            },
            DecompositionAttempt {
                attempt_index: 1,
                output_arrival: DelayTime {
                    rise: 5.0,
                    fall: 3.0,
                },
            },
        ];

        assert_eq!(
            format_attempt_trace(&attempts),
            "0 => 7.00\t1 => 5.00\t BEST is 1\n"
        );
    }

    #[test]
    fn phase_adjustment_collapses_buffers_and_inverters_except_into_pos() {
        let nodes = vec![
            PhaseNode {
                id: 1,
                kind: NodeKind::Internal,
                function: NodeFunction::Buffer,
                fanouts: vec![2, 3],
            },
            PhaseNode {
                id: 2,
                kind: NodeKind::Internal,
                function: NodeFunction::Other,
                fanouts: vec![],
            },
            PhaseNode {
                id: 3,
                kind: NodeKind::PrimaryOutput,
                function: NodeFunction::PrimaryOutput,
                fanouts: vec![],
            },
            PhaseNode {
                id: 4,
                kind: NodeKind::Internal,
                function: NodeFunction::Inverter,
                fanouts: vec![],
            },
        ];

        assert_eq!(
            phase_adjustment_actions(&nodes),
            vec![
                PhaseAction::CollapseIntoFanout { node: 1, fanout: 2 },
                PhaseAction::DeleteIfFanoutless { node: 4 },
            ]
        );
    }

    #[test]
    fn network_bound_entry_point_reports_missing_dependencies() {
        let options = SpeedDecompOptions {
            coeff: 0.0,
            model: DelayModel::UnitFanout,
            num_tries: 1,
            debug: false,
            add_inv: false,
        };

        assert_eq!(
            speed_decomp_interface("n1", &options),
            Err(SpeedNoError::MissingDependency(
                "speed_decomp_interface requires native speed_net, speed_delay, speedup, network, and node ports",
            ))
        );
    }
}
