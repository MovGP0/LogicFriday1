//! Native Rust port scaffold for `sis/stg/stg_sc_sim.c`.
//!
//! The C file has two layers: a small single-cube AND evaluator and a
//! scheduler that walks SIS network fanin/fanout lists through `node_t`,
//! `network_t`, and `ndata` globals prepared by the STG enumeration setup.
//! This module ports the evaluator and the `ndata` value/flag behavior, while
//! reporting the still-blocked network/node scheduler explicitly.

use std::error::Error;
use std::fmt;

pub const MAX_ELENGTH: usize = 36;

pub const SCHEDULED: u8 = 1;
pub const ALL_ASSIGNED: u8 = 2;
pub const MARKED: u8 = 4;
pub const CHANGED: u8 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum LogicValue {
    Zero = 0,
    One = 1,
    Unknown = 2,
}

impl LogicValue {
    pub fn from_bool(value: bool) -> Self {
        if value { Self::One } else { Self::Zero }
    }

    pub fn as_c_value(self) -> u8 {
        self as u8
    }

    pub fn state_char(self) -> char {
        match self {
            Self::Zero => '0',
            Self::One => '1',
            Self::Unknown => '-',
        }
    }
}

impl TryFrom<u8> for LogicValue {
    type Error = StgScSimError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Unknown),
            _ => Err(StgScSimError::InvalidLogicValue(value)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StgNodeData {
    pub cube: u64,
    pub value: [LogicValue; MAX_ELENGTH],
    pub jflag: [u8; MAX_ELENGTH],
    pub level: usize,
}

impl StgNodeData {
    pub fn with_cube(cube: u64) -> Self {
        Self {
            cube,
            value: [LogicValue::Unknown; MAX_ELENGTH],
            jflag: [0; MAX_ELENGTH],
            level: 0,
        }
    }

    pub fn value_at(&self, cid: usize) -> Result<LogicValue, StgScSimError> {
        check_cid(cid)?;
        Ok(self.value[cid])
    }

    pub fn flags_at(&self, cid: usize) -> Result<u8, StgScSimError> {
        check_cid(cid)?;
        Ok(self.jflag[cid])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead: &'static str,
    pub c_file: &'static str,
    pub reason: &'static str,
}

pub const BLOCKED_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.488",
        c_file: "LogicSynthesis/sis/stg/level_c.c",
        reason: "creates ndata records, cube literals, levels, and varying_node order",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.489",
        c_file: "LogicSynthesis/sis/stg/senum_main.c",
        reason: "owns STG simulation globals such as copy, npi, n_varying_nodes, and varying_node",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.305",
        c_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "provides native network traversal for primary inputs and fanout scheduling",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.313",
        c_file: "LogicSynthesis/sis/node/fan.c",
        reason: "provides native node fanin/fanout relationships",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.318",
        c_file: "LogicSynthesis/sis/node/node.c",
        reason: "provides native node type and node storage behavior",
    },
];

#[derive(Debug, Eq, PartialEq)]
pub enum StgScSimError {
    InvalidCycleId {
        cid: usize,
        max_exclusive: usize,
    },
    InvalidLogicValue(u8),
    MissingNetworkNodePorts {
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for StgScSimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCycleId { cid, max_exclusive } => {
                write!(
                    f,
                    "STG simulation cycle id {cid} is outside 0..{max_exclusive}"
                )
            }
            Self::InvalidLogicValue(value) => {
                write!(f, "invalid STG simulation logic value {value}")
            }
            Self::MissingNetworkNodePorts { dependencies } => {
                write!(
                    f,
                    "SIS network/node scheduling for stg_sc_sim is blocked by {} unported dependencies",
                    dependencies.len()
                )
            }
        }
    }
}

impl Error for StgScSimError {}

pub fn blocked_dependencies() -> &'static [PortDependency] {
    BLOCKED_DEPENDENCIES
}

pub fn required_value_for_fanin(cube: u64, fanin_index: usize) -> LogicValue {
    LogicValue::from_bool(((cube >> fanin_index) & 1) != 0)
}

pub fn evaluate_single_cube_and<I>(cube: u64, fanin_values: I) -> LogicValue
where
    I: IntoIterator<Item = LogicValue>,
{
    let mut covered = LogicValue::One;

    for (fanin_index, value) in fanin_values.into_iter().enumerate() {
        let literal = required_value_for_fanin(cube, fanin_index);
        match value {
            LogicValue::Unknown => covered = LogicValue::Unknown,
            LogicValue::Zero | LogicValue::One if value != literal => return LogicValue::Zero,
            LogicValue::Zero | LogicValue::One => {}
        }
    }

    covered
}

pub fn evaluate_node(
    node: &mut StgNodeData,
    cid: usize,
    fanin_values: &[LogicValue],
) -> Result<LogicValue, StgScSimError> {
    check_cid(cid)?;

    let covered = evaluate_single_cube_and(node.cube, fanin_values.iter().copied());
    if node.value[cid] != covered {
        node.value[cid] = covered;
        node.jflag[cid] |= CHANGED;
    }

    Ok(covered)
}

pub fn stg_sc_sim(_cid: usize) -> Result<(), StgScSimError> {
    Err(StgScSimError::MissingNetworkNodePorts {
        dependencies: blocked_dependencies(),
    })
}

fn check_cid(cid: usize) -> Result<(), StgScSimError> {
    if cid < MAX_ELENGTH {
        Ok(())
    } else {
        Err(StgScSimError::InvalidCycleId {
            cid,
            max_exclusive: MAX_ELENGTH,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_literal_bits_map_to_required_fanin_values() {
        let cube = 0b101;

        assert_eq!(required_value_for_fanin(cube, 0), LogicValue::One);
        assert_eq!(required_value_for_fanin(cube, 1), LogicValue::Zero);
        assert_eq!(required_value_for_fanin(cube, 2), LogicValue::One);
        assert_eq!(LogicValue::One.as_c_value(), 1);
        assert_eq!(LogicValue::Unknown.state_char(), '-');
        assert_eq!(LogicValue::try_from(2), Ok(LogicValue::Unknown));
        assert_eq!(
            LogicValue::try_from(3),
            Err(StgScSimError::InvalidLogicValue(3))
        );
    }

    #[test]
    fn single_cube_and_matches_c_covered_value_behavior() {
        let cube = 0b101;

        assert_eq!(
            evaluate_single_cube_and(cube, [LogicValue::One, LogicValue::Zero, LogicValue::One],),
            LogicValue::One
        );
        assert_eq!(
            evaluate_single_cube_and(cube, [LogicValue::One, LogicValue::One, LogicValue::One],),
            LogicValue::Zero
        );
        assert_eq!(
            evaluate_single_cube_and(
                cube,
                [LogicValue::One, LogicValue::Unknown, LogicValue::One],
            ),
            LogicValue::Unknown
        );
        assert_eq!(
            evaluate_single_cube_and(
                cube,
                [LogicValue::Unknown, LogicValue::One, LogicValue::One],
            ),
            LogicValue::Zero
        );
    }

    #[test]
    fn node_evaluation_updates_value_and_changed_flag_only_on_change() {
        let mut node = StgNodeData::with_cube(0b01);

        assert_eq!(
            evaluate_node(&mut node, 0, &[LogicValue::One, LogicValue::Zero]),
            Ok(LogicValue::One)
        );
        assert_eq!(node.value_at(0), Ok(LogicValue::One));
        assert_eq!(node.flags_at(0), Ok(CHANGED));

        node.jflag[0] = 0;
        assert_eq!(
            evaluate_node(&mut node, 0, &[LogicValue::One, LogicValue::Zero]),
            Ok(LogicValue::One)
        );
        assert_eq!(node.flags_at(0), Ok(0));

        assert_eq!(
            evaluate_node(&mut node, MAX_ELENGTH, &[LogicValue::One]),
            Err(StgScSimError::InvalidCycleId {
                cid: MAX_ELENGTH,
                max_exclusive: MAX_ELENGTH,
            })
        );
    }

    #[test]
    fn top_level_scheduler_reports_blocked_native_dependencies() {
        let error = stg_sc_sim(0).expect_err("network scheduler should be blocked");
        let StgScSimError::MissingNetworkNodePorts { dependencies } = error else {
            panic!("unexpected error kind");
        };

        assert!(dependencies.iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.488"
                && dependency.c_file == "LogicSynthesis/sis/stg/level_c.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.489"
                && dependency.c_file == "LogicSynthesis/sis/stg/senum_main.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.313"
                && dependency.c_file == "LogicSynthesis/sis/node/fan.c"
        }));
        assert!(
            format!(
                "{}",
                StgScSimError::MissingNetworkNodePorts { dependencies }
            )
            .contains("SIS network/node scheduling")
        );
    }
}
