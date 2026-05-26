//! Native Rust port scaffold for `sis/speed/speed_and.c`.
//!
//! The C routine repeatedly chooses the two earliest literals of a single-cube
//! node, creates an AND node for them, substitutes it back into the original
//! node, and recurses until at most two literals remain. The actual SIS node
//! creation/substitution is blocked by unported node/network APIs, but the
//! literal selection and decomposition plan are represented here as native Rust.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;

pub const REQUIRED_PORT_BEADS: &[&str] = &[
    "LogicFriday1-8j8.2.6.318", // node/node.c: node literals, replace, dup, not, and
    "LogicFriday1-8j8.2.6.313", // node/fan.c: fanin traversal
    "LogicFriday1-8j8.2.6.474", // speed/speed_delay.c: arrival-time lookup and updates
    "LogicFriday1-8j8.2.6.476", // speed/speed_net.c: network mutation helpers
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralValue {
    Zero,
    One,
    DontCare,
}

impl LiteralValue {
    pub fn phase(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            Self::DontCare => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn arrival(self) -> f64 {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndLiteral {
    pub fanin_index: usize,
    pub literal: LiteralValue,
    pub arrival: DelayTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedLiteral {
    pub fanin_index: usize,
    pub phase: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AndDecompStep {
    SingleLevelUpdate,
    InvertConstantOrSingleLiteral,
    CreateAndNode {
        first: SelectedLiteral,
        second: SelectedLiteral,
    },
    SubstituteAndRecurse,
    AddExplicitInverterForMultiLiteralNode,
}

pub fn select_two_earliest_literals(literals: &[AndLiteral]) -> Vec<SelectedLiteral> {
    let mut selected: Vec<(f64, SelectedLiteral)> = literals
        .iter()
        .filter_map(|literal| {
            literal.literal.phase().map(|phase| {
                (
                    literal.arrival.arrival(),
                    SelectedLiteral {
                        fanin_index: literal.fanin_index,
                        phase,
                    },
                )
            })
        })
        .collect();

    selected.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then(left.1.fanin_index.cmp(&right.1.fanin_index))
    });
    selected
        .into_iter()
        .take(2)
        .map(|(_, literal)| literal)
        .collect()
}

pub fn plan_and_decomposition(
    literals: &[AndLiteral],
    invert_result: bool,
) -> Result<Vec<AndDecompStep>, SpeedAndError> {
    let literal_count = literals
        .iter()
        .filter(|literal| literal.literal.phase().is_some())
        .count();

    if literal_count == 0 {
        return Ok(if invert_result {
            vec![
                AndDecompStep::InvertConstantOrSingleLiteral,
                AndDecompStep::SingleLevelUpdate,
            ]
        } else {
            vec![AndDecompStep::SingleLevelUpdate]
        });
    }

    if literal_count > 2 {
        let selected = select_two_earliest_literals(literals);
        if selected.len() != 2 {
            return Err(SpeedAndError::NoSubstitutablePair);
        }
        return Ok(vec![
            AndDecompStep::CreateAndNode {
                first: selected[0].clone(),
                second: selected[1].clone(),
            },
            AndDecompStep::SingleLevelUpdate,
            AndDecompStep::SubstituteAndRecurse,
        ]);
    }

    let mut steps = Vec::new();
    if invert_result {
        if literal_count > 1 {
            steps.push(AndDecompStep::AddExplicitInverterForMultiLiteralNode);
        } else {
            steps.push(AndDecompStep::InvertConstantOrSingleLiteral);
        }
    }
    steps.push(AndDecompStep::SingleLevelUpdate);
    Ok(steps)
}

pub fn speed_and_decomp_network_bound() -> Result<(), SpeedAndError> {
    Err(SpeedAndError::MissingDependency(
        "speed_and_decomp requires native node fanin/literal/substitution APIs, speed delay updates, and network mutation ports",
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedAndError {
    NoSubstitutablePair,
    MissingDependency(&'static str),
}

impl fmt::Display for SpeedAndError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSubstitutablePair => write!(f, "no two substitutable literals were found"),
            Self::MissingDependency(message) => write!(f, "{message}"),
        }
    }
}

impl Error for SpeedAndError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(fanin_index: usize, literal: LiteralValue, rise: f64, fall: f64) -> AndLiteral {
        AndLiteral {
            fanin_index,
            literal,
            arrival: DelayTime { rise, fall },
        }
    }

    #[test]
    fn selects_two_earliest_care_literals_by_max_arrival() {
        let selected = select_two_earliest_literals(&[
            lit(0, LiteralValue::One, 5.0, 2.0),
            lit(1, LiteralValue::DontCare, 1.0, 1.0),
            lit(2, LiteralValue::Zero, 3.0, 4.0),
            lit(3, LiteralValue::One, 1.0, 2.0),
        ]);

        assert_eq!(
            selected,
            vec![
                SelectedLiteral {
                    fanin_index: 3,
                    phase: true,
                },
                SelectedLiteral {
                    fanin_index: 2,
                    phase: false,
                },
            ]
        );
    }

    #[test]
    fn plans_recursive_substitution_for_more_than_two_literals() {
        let plan = plan_and_decomposition(
            &[
                lit(0, LiteralValue::One, 5.0, 2.0),
                lit(1, LiteralValue::Zero, 1.0, 1.0),
                lit(2, LiteralValue::One, 3.0, 3.0),
            ],
            false,
        )
        .unwrap();

        assert_eq!(
            plan,
            vec![
                AndDecompStep::CreateAndNode {
                    first: SelectedLiteral {
                        fanin_index: 1,
                        phase: false,
                    },
                    second: SelectedLiteral {
                        fanin_index: 2,
                        phase: true,
                    },
                },
                AndDecompStep::SingleLevelUpdate,
                AndDecompStep::SubstituteAndRecurse,
            ]
        );
    }

    #[test]
    fn plans_explicit_inverter_only_for_inverted_two_literal_terminal_case() {
        let plan = plan_and_decomposition(
            &[
                lit(0, LiteralValue::One, 1.0, 1.0),
                lit(1, LiteralValue::Zero, 2.0, 2.0),
            ],
            true,
        )
        .unwrap();

        assert_eq!(
            plan,
            vec![
                AndDecompStep::AddExplicitInverterForMultiLiteralNode,
                AndDecompStep::SingleLevelUpdate,
            ]
        );
    }

    #[test]
    fn network_bound_entry_point_reports_missing_dependencies() {
        assert_eq!(
            speed_and_decomp_network_bound(),
            Err(SpeedAndError::MissingDependency(
                "speed_and_decomp requires native node fanin/literal/substitution APIs, speed delay updates, and network mutation ports",
            ))
        );
    }
}
