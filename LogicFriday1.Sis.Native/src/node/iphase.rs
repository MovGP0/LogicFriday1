//! Native cover input phase analysis.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Undefined,
    PrimaryInput,
    PrimaryOutput,
    ConstantZero,
    ConstantOne,
    Buffer,
    Inverter,
    And,
    Or,
    Complex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralValue {
    Negative,
    Positive,
    DontCare,
}

impl LiteralValue {
    pub fn from_sis_value(value: i32) -> Result<Self, InputPhaseError> {
        match value {
            0 => Ok(Self::Negative),
            1 => Ok(Self::Positive),
            2 => Ok(Self::DontCare),
            _ => Err(InputPhaseError::InvalidLiteralValue(value)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase {
    PositiveUnate,
    NegativeUnate,
    Binate,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeCover<Fanin> {
    pub function: NodeFunction,
    pub fanins: Vec<Fanin>,
    pub cubes: Vec<Vec<LiteralValue>>,
}

impl<Fanin> NodeCover<Fanin> {
    pub fn new(function: NodeFunction) -> Self {
        Self {
            function,
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = Fanin>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_cubes(mut self, cubes: Vec<Vec<LiteralValue>>) -> Self {
        self.cubes = cubes;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputPhaseError {
    PrimaryOutputHasNoCover,
    CubeWidthMismatch {
        row: usize,
        expected: usize,
        actual: usize,
    },
    ColumnOutOfRange {
        column: usize,
        width: usize,
    },
    InvalidLiteralValue(i32),
}

impl fmt::Display for InputPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrimaryOutputHasNoCover => {
                write!(f, "primary output nodes do not have input covers")
            }
            Self::CubeWidthMismatch {
                row,
                expected,
                actual,
            } => write!(f, "cover row {row} has width {actual}, expected {expected}"),
            Self::ColumnOutOfRange { column, width } => {
                write!(f, "cover column {column} is outside width {width}")
            }
            Self::InvalidLiteralValue(value) => {
                write!(f, "invalid cover literal value {value}")
            }
        }
    }
}

impl Error for InputPhaseError {}

pub type InputPhaseResult<T> = Result<T, InputPhaseError>;

pub fn node_input_phase<Fanin>(
    node: &NodeCover<Fanin>,
    fanin: &Fanin,
) -> InputPhaseResult<InputPhase>
where
    Fanin: Eq,
{
    if node.function == NodeFunction::PrimaryOutput {
        return Err(InputPhaseError::PrimaryOutputHasNoCover);
    }

    let Some(index) = node.fanins.iter().position(|candidate| candidate == fanin) else {
        return Ok(InputPhase::Unknown);
    };

    node_input_phase_at(node, index)
}

pub fn node_input_phase_at<Fanin>(
    node: &NodeCover<Fanin>,
    column: usize,
) -> InputPhaseResult<InputPhase> {
    if node.function == NodeFunction::PrimaryOutput {
        return Err(InputPhaseError::PrimaryOutputHasNoCover);
    }

    if column >= node.fanins.len() {
        return Err(InputPhaseError::ColumnOutOfRange {
            column,
            width: node.fanins.len(),
        });
    }

    let mut positive_used = false;
    let mut negative_used = false;

    for (row_index, row) in node.cubes.iter().enumerate() {
        if row.len() != node.fanins.len() {
            return Err(InputPhaseError::CubeWidthMismatch {
                row: row_index,
                expected: node.fanins.len(),
                actual: row.len(),
            });
        }

        match row[column] {
            LiteralValue::Positive => {
                positive_used = true;
            }
            LiteralValue::Negative => {
                negative_used = true;
            }
            LiteralValue::DontCare => {}
        }
    }

    Ok(match (positive_used, negative_used) {
        (true, true) => InputPhase::Binate,
        (true, false) => InputPhase::PositiveUnate,
        (false, true) => InputPhase::NegativeUnate,
        (false, false) => InputPhase::Unknown,
    })
}

pub fn node_input_phases<Fanin>(node: &NodeCover<Fanin>) -> InputPhaseResult<Vec<InputPhase>> {
    if node.function == NodeFunction::PrimaryOutput {
        return Err(InputPhaseError::PrimaryOutputHasNoCover);
    }

    (0..node.fanins.len())
        .map(|column| node_input_phase_at(node, column))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_unknown_when_fanin_is_not_present() {
        let node = sample_node();

        assert_eq!(
            node_input_phase(&node, &"missing").unwrap(),
            InputPhase::Unknown
        );
    }

    #[test]
    fn distinguishes_positive_negative_binate_and_unknown_columns() {
        let node = NodeCover::new(NodeFunction::Complex)
            .with_fanins(vec!["pos", "neg", "bin", "unused"])
            .with_cubes(vec![
                vec![
                    LiteralValue::Positive,
                    LiteralValue::Negative,
                    LiteralValue::Positive,
                    LiteralValue::DontCare,
                ],
                vec![
                    LiteralValue::Positive,
                    LiteralValue::DontCare,
                    LiteralValue::Negative,
                    LiteralValue::DontCare,
                ],
            ]);

        assert_eq!(
            node_input_phase(&node, &"pos").unwrap(),
            InputPhase::PositiveUnate
        );
        assert_eq!(
            node_input_phase(&node, &"neg").unwrap(),
            InputPhase::NegativeUnate
        );
        assert_eq!(node_input_phase(&node, &"bin").unwrap(), InputPhase::Binate);
        assert_eq!(
            node_input_phase(&node, &"unused").unwrap(),
            InputPhase::Unknown
        );
    }

    #[test]
    fn returns_all_input_phases_in_fanin_order() {
        let node = sample_node();

        assert_eq!(
            node_input_phases(&node).unwrap(),
            vec![
                InputPhase::PositiveUnate,
                InputPhase::NegativeUnate,
                InputPhase::Binate,
            ]
        );
    }

    #[test]
    fn rejects_primary_output_nodes() {
        let node = NodeCover::new(NodeFunction::PrimaryOutput)
            .with_fanins(vec!["a"])
            .with_cubes(vec![vec![LiteralValue::Positive]]);

        assert_eq!(
            node_input_phase(&node, &"a"),
            Err(InputPhaseError::PrimaryOutputHasNoCover)
        );
    }

    #[test]
    fn rejects_malformed_cube_width() {
        let node = NodeCover::new(NodeFunction::Complex)
            .with_fanins(vec!["a", "b"])
            .with_cubes(vec![vec![LiteralValue::Positive]]);

        assert_eq!(
            node_input_phase_at(&node, 0),
            Err(InputPhaseError::CubeWidthMismatch {
                row: 0,
                expected: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn converts_legacy_literal_values() {
        assert_eq!(
            LiteralValue::from_sis_value(0).unwrap(),
            LiteralValue::Negative
        );
        assert_eq!(
            LiteralValue::from_sis_value(1).unwrap(),
            LiteralValue::Positive
        );
        assert_eq!(
            LiteralValue::from_sis_value(2).unwrap(),
            LiteralValue::DontCare
        );
        assert_eq!(
            LiteralValue::from_sis_value(3),
            Err(InputPhaseError::InvalidLiteralValue(3))
        );
    }

    fn sample_node() -> NodeCover<&'static str> {
        NodeCover::new(NodeFunction::Complex)
            .with_fanins(vec!["a", "b", "c"])
            .with_cubes(vec![
                vec![
                    LiteralValue::Positive,
                    LiteralValue::Negative,
                    LiteralValue::Positive,
                ],
                vec![
                    LiteralValue::DontCare,
                    LiteralValue::DontCare,
                    LiteralValue::Negative,
                ],
            ])
    }
}
