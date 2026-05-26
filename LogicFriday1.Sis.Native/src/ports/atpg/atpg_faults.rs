//! Native stuck-at fault generation for SIS ATPG.
//!
//! The legacy ATPG implementation stores faults as linked-list entries over
//! `node_t` pointers. This Rust port keeps the same collapse rules on an owned
//! graph model and exposes Rust data structures only.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

pub type AtpgFaultResult<T> = Result<T, AtpgFaultError>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AtpgNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtpgNodeFunction
{
    PrimaryInput,
    PrimaryOutput,
    Zero,
    One,
    And,
    Or,
    Complex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase
{
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StuckValue
{
    StuckAt0,
    StuckAt1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultStatus
{
    Untested,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgFanin
{
    pub node_id: AtpgNodeId,
    pub phase: InputPhase,
}

impl AtpgFanin
{
    pub fn new(node_id: AtpgNodeId, phase: InputPhase) -> Self
    {
        Self { node_id, phase }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgNode
{
    pub id: AtpgNodeId,
    pub function: AtpgNodeFunction,
    pub fanins: Vec<AtpgFanin>,
}

impl AtpgNode
{
    pub fn new(id: usize, function: AtpgNodeFunction) -> Self
    {
        Self
        {
            id: AtpgNodeId(id),
            function,
            fanins: Vec::new(),
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = AtpgFanin>) -> Self
    {
        self.fanins = fanins.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgNetwork
{
    nodes: Vec<AtpgNode>,
    positions: HashMap<AtpgNodeId, usize>,
    fanouts: HashMap<AtpgNodeId, Vec<AtpgNodeId>>,
}

impl AtpgNetwork
{
    pub fn new(nodes: Vec<AtpgNode>) -> AtpgFaultResult<Self>
    {
        let mut positions = HashMap::with_capacity(nodes.len());
        for (position, node) in nodes.iter().enumerate()
        {
            if positions.insert(node.id, position).is_some()
            {
                return Err(AtpgFaultError::DuplicateNode { node_id: node.id });
            }
        }

        let mut fanouts: HashMap<AtpgNodeId, Vec<AtpgNodeId>> =
            nodes.iter().map(|node| (node.id, Vec::new())).collect();

        for node in &nodes
        {
            for fanin in &node.fanins
            {
                if !positions.contains_key(&fanin.node_id)
                {
                    return Err(AtpgFaultError::MissingFanin
                    {
                        node_id: node.id,
                        fanin_id: fanin.node_id,
                    });
                }

                fanouts.entry(fanin.node_id).or_default().push(node.id);
            }
        }

        Ok(Self
        {
            nodes,
            positions,
            fanouts,
        })
    }

    pub fn nodes(&self) -> &[AtpgNode]
    {
        &self.nodes
    }

    pub fn node(&self, node_id: AtpgNodeId) -> AtpgFaultResult<&AtpgNode>
    {
        self.positions
            .get(&node_id)
            .map(|position| &self.nodes[*position])
            .ok_or(AtpgFaultError::MissingNode { node_id })
    }

    pub fn fanouts(&self, node_id: AtpgNodeId) -> AtpgFaultResult<&[AtpgNodeId]>
    {
        self.node(node_id)?;
        Ok(self
            .fanouts
            .get(&node_id)
            .map(Vec::as_slice)
            .unwrap_or_default())
    }

    fn fanout_count(&self, node_id: AtpgNodeId) -> AtpgFaultResult<usize>
    {
        Ok(self.fanouts(node_id)?.len())
    }

    fn fanout(&self, node_id: AtpgNodeId, index: usize) -> AtpgFaultResult<&AtpgNode>
    {
        let fanout_id = *self
            .fanouts(node_id)?
            .get(index)
            .ok_or(AtpgFaultError::MissingFanout { node_id, index })?;

        self.node(fanout_id)
    }

    fn fanin_index(&self, node: &AtpgNode, fanin_id: AtpgNodeId) -> AtpgFaultResult<usize>
    {
        node.fanins
            .iter()
            .position(|fanin| fanin.node_id == fanin_id)
            .ok_or(AtpgFaultError::FaninNotFound
            {
                node_id: node.id,
                fanin_id,
            })
    }

    fn input_phase(&self, node: &AtpgNode, fanin_id: AtpgNodeId) -> AtpgFaultResult<InputPhase>
    {
        node.fanins
            .iter()
            .find(|fanin| fanin.node_id == fanin_id)
            .map(|fanin| fanin.phase)
            .ok_or(AtpgFaultError::FaninNotFound
            {
                node_id: node.id,
                fanin_id,
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fault
{
    pub node_id: AtpgNodeId,
    pub fanin_id: Option<AtpgNodeId>,
    pub index: isize,
    pub value: StuckValue,
    pub status: FaultStatus,
    pub is_covered: bool,
    pub current_state: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgFaultOptions
{
    pub latch_count: usize,
    pub control_nodes: HashSet<AtpgNodeId>,
}

impl AtpgFaultOptions
{
    pub fn new(latch_count: usize) -> Self
    {
        Self
        {
            latch_count,
            control_nodes: HashSet::new(),
        }
    }

    pub fn with_control_nodes(
        mut self,
        control_nodes: impl IntoIterator<Item = AtpgNodeId>,
    ) -> Self
    {
        self.control_nodes = control_nodes.into_iter().collect();
        self
    }
}

pub fn generate_faults(
    network: &AtpgNetwork,
    options: &AtpgFaultOptions,
) -> AtpgFaultResult<Vec<Fault>>
{
    let mut faults = Vec::new();
    let node_order = network_dfs(network)?;

    for node_id in node_order.into_iter().rev()
    {
        if options.control_nodes.contains(&node_id)
        {
            continue;
        }

        let node = network.node(node_id)?;
        collapse_node_faults(&mut faults, network, node, options.latch_count)?;
    }

    Ok(faults)
}

pub fn generate_node_faults(
    network: &AtpgNetwork,
    node_ids: &[AtpgNodeId],
    latch_count: usize,
) -> AtpgFaultResult<Vec<Fault>>
{
    let mut faults = Vec::new();

    for node_id in node_ids.iter().rev()
    {
        let node = network.node(*node_id)?;
        append_node_faults(&mut faults, network, node, latch_count)?;
    }

    Ok(faults)
}

pub fn new_fault(
    network: &AtpgNetwork,
    node_id: AtpgNodeId,
    fanin_id: Option<AtpgNodeId>,
    value: StuckValue,
    latch_count: usize,
) -> AtpgFaultResult<Fault>
{
    let node = network.node(node_id)?;
    let index = match fanin_id
    {
        Some(fanin_id) => network.fanin_index(node, fanin_id)? as isize,
        None => -1,
    };

    Ok(Fault
    {
        node_id,
        fanin_id,
        index,
        value,
        status: FaultStatus::Untested,
        is_covered: false,
        current_state: vec![0; latch_count],
    })
}

fn collapse_node_faults(
    faults: &mut Vec<Fault>,
    network: &AtpgNetwork,
    node: &AtpgNode,
    latch_count: usize,
) -> AtpgFaultResult<()>
{
    match node.function
    {
        AtpgNodeFunction::Complex =>
        {
            append_node_faults(faults, network, node, latch_count)?;
        }
        AtpgNodeFunction::PrimaryOutput | AtpgNodeFunction::Zero | AtpgNodeFunction::One => {}
        AtpgNodeFunction::PrimaryInput => match network.fanout_count(node.id)?
        {
            0 => {}
            1 => pi_fault_collapse(faults, network, node, latch_count)?,
            _ =>
            {
                faults.push(new_fault(
                    network,
                    node.id,
                    None,
                    StuckValue::StuckAt0,
                    latch_count,
                )?);
                faults.push(new_fault(
                    network,
                    node.id,
                    None,
                    StuckValue::StuckAt1,
                    latch_count,
                )?);
            }
        },
        _ =>
        {
            for fanin in &node.fanins
            {
                if network.fanout_count(fanin.node_id)? > 1
                {
                    collapse_fanin_fault(faults, network, node, fanin.node_id, latch_count)?;
                }
            }
        }
    }

    Ok(())
}

fn pi_fault_collapse(
    faults: &mut Vec<Fault>,
    network: &AtpgNetwork,
    node: &AtpgNode,
    latch_count: usize,
) -> AtpgFaultResult<()>
{
    debug_assert_eq!(network.fanout_count(node.id).ok(), Some(1));

    let fanout = network.fanout(node.id, 0)?;
    append_collapsed_faults_for_gate(faults, network, fanout, node.id, None, latch_count)
}

fn collapse_fanin_fault(
    faults: &mut Vec<Fault>,
    network: &AtpgNetwork,
    node: &AtpgNode,
    fanin_id: AtpgNodeId,
    latch_count: usize,
) -> AtpgFaultResult<()>
{
    debug_assert!(network.fanout_count(fanin_id).is_ok_and(|count| count > 1));
    append_collapsed_faults_for_gate(faults, network, node, fanin_id, Some(fanin_id), latch_count)
}

fn append_collapsed_faults_for_gate(
    faults: &mut Vec<Fault>,
    network: &AtpgNetwork,
    gate: &AtpgNode,
    driving_fanin_id: AtpgNodeId,
    fault_fanin_id: Option<AtpgNodeId>,
    latch_count: usize,
) -> AtpgFaultResult<()>
{
    if matches!(gate.function, AtpgNodeFunction::And | AtpgNodeFunction::Or)
    {
        let phase = network.input_phase(gate, driving_fanin_id)?;

        if network.fanin_index(gate, driving_fanin_id)? == 0
        {
            faults.push(new_fault(
                network,
                fault_node_id(gate, driving_fanin_id, fault_fanin_id),
                fault_fanin_id,
                equivalent_value(gate.function, phase),
                latch_count,
            )?);
        }

        faults.push(new_fault(
            network,
            fault_node_id(gate, driving_fanin_id, fault_fanin_id),
            fault_fanin_id,
            opposite_value(gate.function, phase),
            latch_count,
        )?);
    } else
    {
        let node_id = fault_node_id(gate, driving_fanin_id, fault_fanin_id);
        faults.push(new_fault(
            network,
            node_id,
            fault_fanin_id,
            StuckValue::StuckAt0,
            latch_count,
        )?);
        faults.push(new_fault(
            network,
            node_id,
            fault_fanin_id,
            StuckValue::StuckAt1,
            latch_count,
        )?);
    }

    Ok(())
}

fn fault_node_id(
    gate: &AtpgNode,
    driving_fanin_id: AtpgNodeId,
    fault_fanin_id: Option<AtpgNodeId>,
) -> AtpgNodeId
{
    if fault_fanin_id.is_some()
    {
        gate.id
    } else
    {
        driving_fanin_id
    }
}

fn equivalent_value(function: AtpgNodeFunction, phase: InputPhase) -> StuckValue
{
    match (function, phase)
    {
        (AtpgNodeFunction::And, InputPhase::Positive) => StuckValue::StuckAt0,
        (AtpgNodeFunction::And, InputPhase::Negative) => StuckValue::StuckAt1,
        (AtpgNodeFunction::Or, InputPhase::Positive) => StuckValue::StuckAt1,
        (AtpgNodeFunction::Or, InputPhase::Negative) => StuckValue::StuckAt0,
        _ => unreachable!("equivalent values are defined only for AND/OR gates"),
    }
}

fn opposite_value(function: AtpgNodeFunction, phase: InputPhase) -> StuckValue
{
    match (function, phase)
    {
        (AtpgNodeFunction::And, InputPhase::Positive) => StuckValue::StuckAt1,
        (AtpgNodeFunction::And, InputPhase::Negative) => StuckValue::StuckAt0,
        (AtpgNodeFunction::Or, InputPhase::Positive) => StuckValue::StuckAt0,
        (AtpgNodeFunction::Or, InputPhase::Negative) => StuckValue::StuckAt1,
        _ => unreachable!("opposite values are defined only for AND/OR gates"),
    }
}

fn append_node_faults(
    faults: &mut Vec<Fault>,
    network: &AtpgNetwork,
    node: &AtpgNode,
    latch_count: usize,
) -> AtpgFaultResult<()>
{
    for fanin in &node.fanins
    {
        faults.push(new_fault(
            network,
            node.id,
            Some(fanin.node_id),
            StuckValue::StuckAt0,
            latch_count,
        )?);
        faults.push(new_fault(
            network,
            node.id,
            Some(fanin.node_id),
            StuckValue::StuckAt1,
            latch_count,
        )?);
    }

    faults.push(new_fault(
        network,
        node.id,
        None,
        StuckValue::StuckAt0,
        latch_count,
    )?);
    faults.push(new_fault(
        network,
        node.id,
        None,
        StuckValue::StuckAt1,
        latch_count,
    )?);

    Ok(())
}

fn network_dfs(network: &AtpgNetwork) -> AtpgFaultResult<Vec<AtpgNodeId>>
{
    let mut roots = network
        .nodes()
        .iter()
        .filter(|node| node.function == AtpgNodeFunction::PrimaryOutput)
        .map(|node| node.id)
        .collect::<Vec<_>>();

    roots.extend(
        network
            .nodes()
            .iter()
            .filter(|node|
            {
                network.fanouts.get(&node.id).is_none_or(Vec::is_empty)
                    && node.function != AtpgNodeFunction::PrimaryOutput
            })
            .map(|node| node.id),
    );

    let mut visited = HashMap::new();
    let mut order = Vec::new();

    for root in roots
    {
        dfs_recur(network, root, &mut visited, &mut order)?;
    }

    Ok(order)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState
{
    Active,
    Done,
}

fn dfs_recur(
    network: &AtpgNetwork,
    node_id: AtpgNodeId,
    visited: &mut HashMap<AtpgNodeId, VisitState>,
    order: &mut Vec<AtpgNodeId>,
) -> AtpgFaultResult<()>
{
    if let Some(state) = visited.get(&node_id)
    {
        return match state
        {
            VisitState::Active => Err(AtpgFaultError::CycleDetected { node_id }),
            VisitState::Done => Ok(()),
        };
    }

    visited.insert(node_id, VisitState::Active);

    let node = network.node(node_id)?;
    for fanin in &node.fanins
    {
        dfs_recur(network, fanin.node_id, visited, order)?;
    }

    visited.insert(node_id, VisitState::Done);
    order.push(node_id);
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtpgFaultError
{
    DuplicateNode
    {
        node_id: AtpgNodeId,
    },
    MissingNode
    {
        node_id: AtpgNodeId,
    },
    MissingFanin
    {
        node_id: AtpgNodeId,
        fanin_id: AtpgNodeId,
    },
    MissingFanout
    {
        node_id: AtpgNodeId,
        index: usize,
    },
    FaninNotFound
    {
        node_id: AtpgNodeId,
        fanin_id: AtpgNodeId,
    },
    CycleDetected
    {
        node_id: AtpgNodeId,
    },
}

impl fmt::Display for AtpgFaultError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::DuplicateNode { node_id } =>
            {
                write!(formatter, "duplicate ATPG node {}", node_id.0)
            }
            Self::MissingNode { node_id } =>
            {
                write!(formatter, "missing ATPG node {}", node_id.0)
            }
            Self::MissingFanin { node_id, fanin_id } => write!(
                formatter,
                "ATPG node {} references missing fanin {}",
                node_id.0, fanin_id.0
            ),
            Self::MissingFanout { node_id, index } => write!(
                formatter,
                "ATPG node {} has no fanout at index {index}",
                node_id.0
            ),
            Self::FaninNotFound { node_id, fanin_id } => write!(
                formatter,
                "ATPG node {} does not have fanin {}",
                node_id.0, fanin_id.0
            ),
            Self::CycleDetected { node_id } =>
            {
                write!(
                    formatter,
                    "ATPG network contains a cycle through node {}",
                    node_id.0
                )
            }
        }
    }
}

impl Error for AtpgFaultError {}

#[cfg(test)]
mod tests
{
    use super::*;

    fn fanin(id: usize) -> AtpgFanin
    {
        AtpgFanin::new(AtpgNodeId(id), InputPhase::Positive)
    }

    fn inverted_fanin(id: usize) -> AtpgFanin
    {
        AtpgFanin::new(AtpgNodeId(id), InputPhase::Negative)
    }

    fn fault_key(fault: &Fault) -> (usize, Option<usize>, isize, StuckValue)
    {
        (
            fault.node_id.0,
            fault.fanin_id.map(|node_id| node_id.0),
            fault.index,
            fault.value,
        )
    }

    #[test]
    fn node_fault_generation_adds_input_and_output_faults()
    {
        let network = AtpgNetwork::new(vec![
            AtpgNode::new(1, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(2, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(3, AtpgNodeFunction::And).with_fanins([fanin(1), fanin(2)]),
        ])
        .unwrap();

        let faults = generate_node_faults(&network, &[AtpgNodeId(3)], 2).unwrap();

        assert_eq!(
            faults.iter().map(fault_key).collect::<Vec<_>>(),
            vec![
                (3, Some(1), 0, StuckValue::StuckAt0),
                (3, Some(1), 0, StuckValue::StuckAt1),
                (3, Some(2), 1, StuckValue::StuckAt0),
                (3, Some(2), 1, StuckValue::StuckAt1),
                (3, None, -1, StuckValue::StuckAt0),
                (3, None, -1, StuckValue::StuckAt1),
            ]
        );
        assert!(faults
            .iter()
            .all(|fault| fault.status == FaultStatus::Untested));
        assert!(faults.iter().all(|fault| fault.current_state == vec![0, 0]));
    }

    #[test]
    fn collapsed_faults_visit_nodes_closer_to_outputs_first()
    {
        let network = AtpgNetwork::new(vec![
            AtpgNode::new(1, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(2, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(3, AtpgNodeFunction::Complex).with_fanins([fanin(1), fanin(2)]),
            AtpgNode::new(4, AtpgNodeFunction::PrimaryOutput).with_fanins([fanin(3)]),
        ])
        .unwrap();

        let faults = generate_faults(&network, &AtpgFaultOptions::new(0)).unwrap();

        assert_eq!(faults.first().unwrap().node_id, AtpgNodeId(3));
        assert_eq!(faults.len(), 10);
    }

    #[test]
    fn primary_output_constants_and_control_nodes_are_skipped_when_collapsing()
    {
        let network = AtpgNetwork::new(vec![
            AtpgNode::new(1, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(2, AtpgNodeFunction::Zero),
            AtpgNode::new(3, AtpgNodeFunction::Complex).with_fanins([fanin(1)]),
            AtpgNode::new(4, AtpgNodeFunction::PrimaryOutput).with_fanins([fanin(3)]),
        ])
        .unwrap();
        let options = AtpgFaultOptions::new(0).with_control_nodes([AtpgNodeId(3)]);

        let faults = generate_faults(&network, &options).unwrap();

        assert_eq!(
            faults.iter().map(fault_key).collect::<Vec<_>>(),
            vec![
                (1, None, -1, StuckValue::StuckAt0),
                (1, None, -1, StuckValue::StuckAt1),
            ]
        );
    }

    #[test]
    fn pi_with_no_fanout_contributes_no_collapsed_faults()
    {
        let network =
            AtpgNetwork::new(vec![AtpgNode::new(1, AtpgNodeFunction::PrimaryInput)]).unwrap();

        let faults = generate_faults(&network, &AtpgFaultOptions::new(0)).unwrap();

        assert!(faults.is_empty());
    }

    #[test]
    fn pi_with_multiple_fanouts_keeps_both_output_faults()
    {
        let network = AtpgNetwork::new(vec![
            AtpgNode::new(1, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(2, AtpgNodeFunction::Complex).with_fanins([fanin(1)]),
            AtpgNode::new(3, AtpgNodeFunction::Complex).with_fanins([fanin(1)]),
        ])
        .unwrap();

        let faults = generate_faults(&network, &AtpgFaultOptions::new(0)).unwrap();

        assert!(faults
            .iter()
            .any(|fault| fault_key(fault) == (1, None, -1, StuckValue::StuckAt0)));
        assert!(faults
            .iter()
            .any(|fault| fault_key(fault) == (1, None, -1, StuckValue::StuckAt1)));
    }

    #[test]
    fn pi_single_fanout_uses_and_or_equivalence_rules()
    {
        let network = AtpgNetwork::new(vec![
            AtpgNode::new(1, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(2, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(3, AtpgNodeFunction::And).with_fanins([inverted_fanin(1), fanin(2)]),
            AtpgNode::new(4, AtpgNodeFunction::PrimaryOutput).with_fanins([fanin(3)]),
        ])
        .unwrap();

        let faults = generate_faults(&network, &AtpgFaultOptions::new(0)).unwrap();
        let first_pi_faults = faults
            .iter()
            .filter(|fault| fault.node_id == AtpgNodeId(1))
            .map(fault_key)
            .collect::<Vec<_>>();

        assert_eq!(
            first_pi_faults,
            vec![
                (1, None, -1, StuckValue::StuckAt1),
                (1, None, -1, StuckValue::StuckAt0),
            ]
        );
    }

    #[test]
    fn fanout_branch_collapse_uses_branch_index_and_phase()
    {
        let network = AtpgNetwork::new(vec![
            AtpgNode::new(1, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(2, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(3, AtpgNodeFunction::Or).with_fanins([fanin(1), fanin(2)]),
            AtpgNode::new(4, AtpgNodeFunction::And).with_fanins([inverted_fanin(1), fanin(2)]),
            AtpgNode::new(5, AtpgNodeFunction::PrimaryOutput).with_fanins([fanin(3)]),
            AtpgNode::new(6, AtpgNodeFunction::PrimaryOutput).with_fanins([fanin(4)]),
        ])
        .unwrap();

        let faults = generate_faults(&network, &AtpgFaultOptions::new(0)).unwrap();

        assert!(faults
            .iter()
            .any(|fault| fault_key(fault) == (3, Some(1), 0, StuckValue::StuckAt1)));
        assert!(faults
            .iter()
            .any(|fault| fault_key(fault) == (3, Some(1), 0, StuckValue::StuckAt0)));
        assert!(faults
            .iter()
            .any(|fault| fault_key(fault) == (4, Some(1), 0, StuckValue::StuckAt1)));
        assert!(faults
            .iter()
            .any(|fault| fault_key(fault) == (4, Some(1), 0, StuckValue::StuckAt0)));
    }

    #[test]
    fn non_and_or_collapse_keeps_both_fault_values()
    {
        let network = AtpgNetwork::new(vec![
            AtpgNode::new(1, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(2, AtpgNodeFunction::PrimaryInput),
            AtpgNode::new(3, AtpgNodeFunction::Complex).with_fanins([fanin(1)]),
            AtpgNode::new(4, AtpgNodeFunction::Complex).with_fanins([fanin(1), fanin(2)]),
        ])
        .unwrap();

        let faults = generate_faults(&network, &AtpgFaultOptions::new(0)).unwrap();

        assert!(faults
            .iter()
            .any(|fault| fault_key(fault) == (4, Some(1), 0, StuckValue::StuckAt0)));
        assert!(faults
            .iter()
            .any(|fault| fault_key(fault) == (4, Some(1), 0, StuckValue::StuckAt1)));
    }

    #[test]
    fn invalid_networks_report_precise_errors()
    {
        assert!(matches!(
            AtpgNetwork::new(vec![
                AtpgNode::new(1, AtpgNodeFunction::PrimaryInput),
                AtpgNode::new(1, AtpgNodeFunction::PrimaryInput),
            ]),
            Err(AtpgFaultError::DuplicateNode { .. })
        ));

        assert!(matches!(
            AtpgNetwork::new(vec![
                AtpgNode::new(1, AtpgNodeFunction::And).with_fanins([fanin(2)]),
            ]),
            Err(AtpgFaultError::MissingFanin { .. })
        ));

        let cyclic = AtpgNetwork::new(vec![
            AtpgNode::new(1, AtpgNodeFunction::And).with_fanins([fanin(2)]),
            AtpgNode::new(2, AtpgNodeFunction::And).with_fanins([fanin(1)]),
            AtpgNode::new(3, AtpgNodeFunction::PrimaryOutput).with_fanins([fanin(1)]),
        ])
        .unwrap();

        assert!(matches!(
            generate_faults(&cyclic, &AtpgFaultOptions::new(0)),
            Err(AtpgFaultError::CycleDetected { .. })
        ));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port()
    {
        let source = include_str!("atpg_faults.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}


