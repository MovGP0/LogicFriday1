//! Native Rust port for `sis/stg/stg_sc_sim.c`.
//!
//! The C file has two layers: a small single-cube AND evaluator and a
//! scheduler that propagates changed primary-input values through the varying
//! non-primary-output nodes.

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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StgNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StgNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StgSimNode {
    pub kind: StgNodeKind,
    pub data: StgNodeData,
    pub fanins: Vec<StgNodeId>,
    pub fanouts: Vec<StgNodeId>,
}

impl StgSimNode {
    pub fn primary_input() -> Self {
        Self {
            kind: StgNodeKind::PrimaryInput,
            data: StgNodeData::with_cube(0),
            fanins: Vec::new(),
            fanouts: Vec::new(),
        }
    }

    pub fn primary_output(fanin: StgNodeId) -> Self {
        Self {
            kind: StgNodeKind::PrimaryOutput,
            data: StgNodeData::with_cube(0),
            fanins: vec![fanin],
            fanouts: Vec::new(),
        }
    }

    pub fn internal(cube: u64, fanins: Vec<StgNodeId>) -> Self {
        Self {
            kind: StgNodeKind::Internal,
            data: StgNodeData::with_cube(cube),
            fanins,
            fanouts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StgScSimNetwork {
    nodes: Vec<StgSimNode>,
    primary_inputs: Vec<StgNodeId>,
    varying_nodes: Vec<StgNodeId>,
}

impl StgScSimNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_primary_input(&mut self) -> StgNodeId {
        let node_id = self.insert_node(StgSimNode::primary_input());
        self.primary_inputs.push(node_id);
        node_id
    }

    pub fn add_primary_output(&mut self, fanin: StgNodeId) -> Result<StgNodeId, StgScSimError> {
        self.add_node(StgSimNode::primary_output(fanin))
    }

    pub fn add_internal(
        &mut self,
        cube: u64,
        fanins: Vec<StgNodeId>,
    ) -> Result<StgNodeId, StgScSimError> {
        self.add_node(StgSimNode::internal(cube, fanins))
    }

    pub fn node(&self, node: StgNodeId) -> Result<&StgSimNode, StgScSimError> {
        self.nodes
            .get(node.0)
            .ok_or(StgScSimError::MissingNode(node))
    }

    pub fn node_mut(&mut self, node: StgNodeId) -> Result<&mut StgSimNode, StgScSimError> {
        self.nodes
            .get_mut(node.0)
            .ok_or(StgScSimError::MissingNode(node))
    }

    pub fn set_primary_input_value(
        &mut self,
        node: StgNodeId,
        cid: usize,
        value: LogicValue,
    ) -> Result<(), StgScSimError> {
        check_cid(cid)?;

        let node_data = self.node_mut(node)?;
        if node_data.kind != StgNodeKind::PrimaryInput {
            return Err(StgScSimError::WrongNodeKind {
                node,
                expected: StgNodeKind::PrimaryInput,
                actual: node_data.kind,
            });
        }

        if node_data.data.value[cid] != value {
            node_data.data.value[cid] = value;
            node_data.data.jflag[cid] |= CHANGED;
        }

        Ok(())
    }

    fn add_node(&mut self, node: StgSimNode) -> Result<StgNodeId, StgScSimError> {
        for fanin in node.fanins.iter().copied() {
            self.node(fanin)?;
        }

        let fanins = node.fanins.clone();
        let kind = node.kind;
        let node_id = self.insert_node(node);

        for fanin in fanins {
            self.node_mut(fanin)?.fanouts.push(node_id);
        }

        if kind != StgNodeKind::PrimaryInput {
            self.varying_nodes.push(node_id);
        }

        Ok(node_id)
    }

    fn insert_node(&mut self, node: StgSimNode) -> StgNodeId {
        let node_id = StgNodeId(self.nodes.len());
        self.nodes.push(node);
        node_id
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum StgScSimError {
    InvalidCycleId {
        cid: usize,
        max_exclusive: usize,
    },
    InvalidLogicValue(u8),
    MissingNode(StgNodeId),
    WrongNodeKind {
        node: StgNodeId,
        expected: StgNodeKind,
        actual: StgNodeKind,
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
            Self::MissingNode(node) => {
                write!(f, "missing STG simulation node {:?}", node)
            }
            Self::WrongNodeKind {
                node,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "STG simulation node {:?} has kind {:?}; expected {:?}",
                    node, actual, expected
                )
            }
        }
    }
}

impl Error for StgScSimError {}

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

pub fn stg_sc_sim(network: &mut StgScSimNetwork, cid: usize) -> Result<(), StgScSimError> {
    check_cid(cid)?;

    let primary_inputs = network.primary_inputs.clone();
    for node in primary_inputs {
        if network.node(node)?.data.jflag[cid] & CHANGED != 0 {
            network.node_mut(node)?.data.jflag[cid] &= !CHANGED;
            schedule_fanouts(network, node, cid)?;
        }
    }

    let varying_nodes = network.varying_nodes.clone();
    for node in varying_nodes {
        if network.node(node)?.data.jflag[cid] & SCHEDULED == 0 {
            continue;
        }

        network.node_mut(node)?.data.jflag[cid] &= !SCHEDULED;

        if network.node(node)?.kind == StgNodeKind::PrimaryOutput {
            continue;
        }

        evaluate_network_node(network, node, cid)?;

        if network.node(node)?.data.jflag[cid] & CHANGED != 0 {
            network.node_mut(node)?.data.jflag[cid] &= !CHANGED;
            schedule_fanouts(network, node, cid)?;
        }
    }

    Ok(())
}

fn evaluate_network_node(
    network: &mut StgScSimNetwork,
    node: StgNodeId,
    cid: usize,
) -> Result<LogicValue, StgScSimError> {
    let fanins = network.node(node)?.fanins.clone();
    let fanin_values = fanins
        .iter()
        .map(|fanin| network.node(*fanin).map(|fanin| fanin.data.value[cid]))
        .collect::<Result<Vec<_>, _>>()?;

    evaluate_node(&mut network.node_mut(node)?.data, cid, &fanin_values)
}

fn schedule_fanouts(
    network: &mut StgScSimNetwork,
    node: StgNodeId,
    cid: usize,
) -> Result<(), StgScSimError> {
    let fanouts = network.node(node)?.fanouts.clone();
    for fanout in fanouts {
        let fanout_node = network.node_mut(fanout)?;
        if fanout_node.kind != StgNodeKind::PrimaryOutput {
            fanout_node.data.jflag[cid] |= SCHEDULED;
        }
    }

    Ok(())
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
    fn scheduler_propagates_changed_primary_inputs_through_internal_nodes() {
        let mut network = StgScSimNetwork::new();
        let a = network.add_primary_input();
        let b = network.add_primary_input();
        let and = network.add_internal(0b11, vec![a, b]).unwrap();
        let inverted = network.add_internal(0b0, vec![and]).unwrap();
        let _output = network.add_primary_output(inverted).unwrap();

        network
            .set_primary_input_value(a, 0, LogicValue::One)
            .unwrap();
        network
            .set_primary_input_value(b, 0, LogicValue::One)
            .unwrap();
        stg_sc_sim(&mut network, 0).unwrap();

        assert_eq!(
            network.node(and).unwrap().data.value_at(0),
            Ok(LogicValue::One)
        );
        assert_eq!(
            network.node(inverted).unwrap().data.value_at(0),
            Ok(LogicValue::Zero)
        );
        assert_eq!(network.node(and).unwrap().data.flags_at(0), Ok(0));
        assert_eq!(network.node(inverted).unwrap().data.flags_at(0), Ok(0));

        network
            .set_primary_input_value(a, 0, LogicValue::Zero)
            .unwrap();
        stg_sc_sim(&mut network, 0).unwrap();

        assert_eq!(
            network.node(and).unwrap().data.value_at(0),
            Ok(LogicValue::Zero)
        );
        assert_eq!(
            network.node(inverted).unwrap().data.value_at(0),
            Ok(LogicValue::One)
        );
    }

    #[test]
    fn scheduler_ignores_primary_outputs_and_reports_invalid_inputs() {
        let mut network = StgScSimNetwork::new();
        let a = network.add_primary_input();
        let output = network.add_primary_output(a).unwrap();

        assert_eq!(
            network.set_primary_input_value(output, 0, LogicValue::One),
            Err(StgScSimError::WrongNodeKind {
                node: output,
                expected: StgNodeKind::PrimaryInput,
                actual: StgNodeKind::PrimaryOutput,
            })
        );
        assert_eq!(
            network.add_internal(0, vec![StgNodeId(99)]),
            Err(StgScSimError::MissingNode(StgNodeId(99)))
        );
        assert_eq!(
            stg_sc_sim(&mut network, MAX_ELENGTH),
            Err(StgScSimError::InvalidCycleId {
                cid: MAX_ELENGTH,
                max_exclusive: MAX_ELENGTH,
            })
        );
    }
}
