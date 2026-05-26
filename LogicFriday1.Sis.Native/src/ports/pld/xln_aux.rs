//! Native Rust model for `LogicSynthesis/sis/pld/xln_aux.c`.
//!
//! The C file provides three PLD helpers: ceiling integer log2, fixed-width
//! binary string formatting, and conversion of a node subset to fanin indices.
//! The first two helpers are pure and are ported directly. The fanin-index
//! helper is exposed over native slices; integration with SIS `array_t` and
//! `node_t` remains an explicit missing-dependency error.

use std::error::Error;
use std::fmt;

pub const SIS_BUFSIZE: usize = 500;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        reason: "xln_array_to_indices receives its Y subset as array_t and returns ALLOC-managed storage",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "node_get_fanin_index defines pointer-identity fanin lookup and the -1 missing-fanin result",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "native SIS integration needs node_t fanin storage and node ownership semantics",
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnAuxError {
    NonPositiveLogInput(i32),
    BinaryLengthExceedsBuffer {
        length: usize,
        buffer: usize,
    },
    BinaryValueDoesNotFit {
        value: u64,
        length: usize,
    },
    MissingFanin {
        subset_index: usize,
    },
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for XlnAuxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveLogInput(value) => {
                write!(f, "cannot take ceil(log2) of non-positive value {value}")
            }
            Self::BinaryLengthExceedsBuffer { length, buffer } => write!(
                f,
                "binary output length {length} exceeds SIS BUFSIZE limit {buffer}"
            ),
            Self::BinaryValueDoesNotFit { value, length } => {
                write!(
                    f,
                    "value {value} cannot be represented in {length} binary digits"
                )
            }
            Self::MissingFanin { subset_index } => write!(
                f,
                "subset entry #{subset_index} is not a fanin of the supplied node"
            ),
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS C-file dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for XlnAuxError {}

pub type XlnAuxResult<T> = Result<T, XlnAuxError>;

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn intlog2(value: i32) -> XlnAuxResult<usize> {
    if value <= 0 {
        return Err(XlnAuxError::NonPositiveLogInput(value));
    }

    let value = value as u32;
    if value <= 1 {
        return Ok(0);
    }

    Ok((u32::BITS - (value - 1).leading_zeros()) as usize)
}

pub fn xl_binary1(value: u64, length: usize) -> XlnAuxResult<String> {
    if length >= SIS_BUFSIZE {
        return Err(XlnAuxError::BinaryLengthExceedsBuffer {
            length,
            buffer: SIS_BUFSIZE,
        });
    }

    if length < u64::BITS as usize && value >= (1_u64 << length) {
        return Err(XlnAuxError::BinaryValueDoesNotFit { value, length });
    }

    let mut result = String::with_capacity(length);
    for bit in (0..length).rev() {
        let is_set = bit < u64::BITS as usize && (value & (1_u64 << bit)) != 0;
        result.push(if is_set { '1' } else { '0' });
    }

    Ok(result)
}

pub fn fanin_index<N: Eq>(node_fanins: &[N], fanin: &N) -> Option<usize> {
    node_fanins.iter().position(|candidate| candidate == fanin)
}

pub fn xln_array_to_indices<N: Eq>(subset: &[N], node_fanins: &[N]) -> XlnAuxResult<Vec<usize>> {
    subset
        .iter()
        .enumerate()
        .map(|(subset_index, fanin)| {
            fanin_index(node_fanins, fanin).ok_or(XlnAuxError::MissingFanin { subset_index })
        })
        .collect()
}

pub fn xln_array_to_indices_c_semantics<N: Eq>(subset: &[N], node_fanins: &[N]) -> Vec<isize> {
    subset
        .iter()
        .map(|fanin| {
            fanin_index(node_fanins, fanin)
                .map(|index| index as isize)
                .unwrap_or(-1)
        })
        .collect()
}

pub fn xln_array_to_indices_blocked<Array, Node>(
    _subset: &Array,
    _node: &Node,
) -> XlnAuxResult<Vec<usize>> {
    Err(XlnAuxError::MissingNativePorts {
        operation: "xln_array_to_indices",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intlog2_matches_c_ceiling_for_powers_and_non_powers() {
        let cases = [
            (1, 0),
            (2, 1),
            (3, 2),
            (4, 2),
            (5, 3),
            (7, 3),
            (8, 3),
            (9, 4),
            (1024, 10),
            (1025, 11),
        ];

        for (input, expected) in cases {
            assert_eq!(intlog2(input), Ok(expected));
        }
    }

    #[test]
    fn intlog2_rejects_non_positive_inputs() {
        assert_eq!(intlog2(0), Err(XlnAuxError::NonPositiveLogInput(0)));
        assert_eq!(intlog2(-7), Err(XlnAuxError::NonPositiveLogInput(-7)));
    }

    #[test]
    fn xl_binary1_formats_fixed_width_binary_strings() {
        assert_eq!(xl_binary1(0, 4), Ok("0000".to_owned()));
        assert_eq!(xl_binary1(1, 4), Ok("0001".to_owned()));
        assert_eq!(xl_binary1(5, 4), Ok("0101".to_owned()));
        assert_eq!(xl_binary1(15, 4), Ok("1111".to_owned()));
        assert_eq!(xl_binary1(5, 8), Ok("00000101".to_owned()));
    }

    #[test]
    fn xl_binary1_reports_c_buffer_and_width_errors() {
        assert_eq!(
            xl_binary1(16, 4),
            Err(XlnAuxError::BinaryValueDoesNotFit {
                value: 16,
                length: 4
            })
        );
        assert_eq!(
            xl_binary1(0, SIS_BUFSIZE),
            Err(XlnAuxError::BinaryLengthExceedsBuffer {
                length: SIS_BUFSIZE,
                buffer: SIS_BUFSIZE
            })
        );
    }

    #[test]
    fn array_to_indices_maps_subset_entries_to_node_fanin_positions() {
        let a = "a";
        let b = "b";
        let c = "c";
        let d = "d";
        let node_fanins = [a, b, c, d];
        let subset = [c, a, d];

        assert_eq!(
            xln_array_to_indices(&subset, &node_fanins),
            Ok(vec![2, 0, 3])
        );
    }

    #[test]
    fn array_to_indices_reports_missing_fanins_or_c_minus_one() {
        let node_fanins = ["a", "b"];
        let subset = ["b", "missing", "a"];

        assert_eq!(
            xln_array_to_indices(&subset, &node_fanins),
            Err(XlnAuxError::MissingFanin { subset_index: 1 })
        );
        assert_eq!(
            xln_array_to_indices_c_semantics(&subset, &node_fanins),
            vec![1, -1, 0]
        );
    }

    #[test]
    fn sis_backed_entry_reports_dependency_beads_and_sources() {
        let Err(XlnAuxError::MissingNativePorts {
            operation,
            dependencies,
        }) = xln_array_to_indices_blocked(&(), &())
        else {
            panic!("expected missing native ports");
        };

        assert_eq!(operation, "xln_array_to_indices");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.2"
                && dependency.source_file == "LogicSynthesis/sis/array/array.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.313"
                && dependency.source_file == "LogicSynthesis/sis/node/fan.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.318"
                && dependency.source_file == "LogicSynthesis/sis/node/node.c"
        }));
    }
}
