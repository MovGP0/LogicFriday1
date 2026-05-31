//! Native Rust behavior for `sis/speed/sp_network.c`.
//!
//! The original C file builds small temporary networks from a node, duplicates
//! delay parameters onto the temporary boundary nodes, and converts networks to
//! arrays of duplicated nodes with fanin pointers rewritten. This module keeps
//! those operations as owned Rust data transformations.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DelayParameter {
    BlockRise,
    DriveRise,
    BlockFall,
    DriveFall,
    MaxInputLoad,
    ArrivalRise,
    ArrivalFall,
    OutputLoad,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DelaySnapshot {
    pub params: HashMap<DelayParameter, f64>,
    pub arrival: Option<DelayTime>,
    pub speed_arrival: Option<DelayTime>,
    pub load: Option<f64>,
}

impl DelaySnapshot {
    pub fn get_parameter(&self, parameter: DelayParameter) -> Result<f64, SpNetworkError> {
        self.params
            .get(&parameter)
            .copied()
            .ok_or(SpNetworkError::MissingDelayParameter(parameter))
    }

    pub fn set_parameter(&mut self, parameter: DelayParameter, value: f64) {
        self.params.insert(parameter, value);

        match parameter {
            DelayParameter::ArrivalRise => {
                let fall = self.arrival.map_or(0.0, |time| time.fall);
                self.arrival = Some(DelayTime::new(value, fall));
            }
            DelayParameter::ArrivalFall => {
                let rise = self.arrival.map_or(0.0, |time| time.rise);
                self.arrival = Some(DelayTime::new(rise, value));
            }
            DelayParameter::OutputLoad => {
                self.load = Some(value);
            }
            DelayParameter::BlockRise
            | DelayParameter::DriveRise
            | DelayParameter::BlockFall
            | DelayParameter::DriveFall
            | DelayParameter::MaxInputLoad => {}
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkNode {
    pub id: usize,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<usize>,
    pub fanouts: Vec<usize>,
    pub library_gate: Option<String>,
    pub delay: DelaySnapshot,
}

impl NetworkNode {
    pub fn new(id: usize, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            library_gate: None,
            delay: DelaySnapshot::default(),
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = usize>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_delay(mut self, delay: DelaySnapshot) -> Self {
        self.delay = delay;
        self
    }

    pub fn with_library_gate(mut self, library_gate: impl Into<String>) -> Self {
        self.library_gate = Some(library_gate.into());
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpeedNetwork {
    nodes: Vec<NetworkNode>,
}

impl SpeedNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_nodes(nodes: impl Into<Vec<NetworkNode>>) -> Result<Self, SpNetworkError> {
        let mut network = Self {
            nodes: nodes.into(),
        };
        network.rebuild_fanouts()?;
        Ok(network)
    }

    pub fn nodes(&self) -> &[NetworkNode] {
        &self.nodes
    }

    pub fn node(&self, id: usize) -> Result<&NetworkNode, SpNetworkError> {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .ok_or(SpNetworkError::MissingNode(id))
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = &NetworkNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryInput)
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = &NetworkNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryOutput)
    }

    fn node_mut(&mut self, id: usize) -> Result<&mut NetworkNode, SpNetworkError> {
        self.nodes
            .iter_mut()
            .find(|node| node.id == id)
            .ok_or(SpNetworkError::MissingNode(id))
    }

    fn name_to_node(&self, name: &str) -> Option<&NetworkNode> {
        self.nodes.iter().find(|node| node.name == name)
    }

    fn rebuild_fanouts(&mut self) -> Result<(), SpNetworkError> {
        let ids = self.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        for node in &mut self.nodes {
            node.fanouts.clear();
        }

        for node_id in ids {
            let fanins = self.node(node_id)?.fanins.clone();
            for fanin in fanins {
                self.node_mut(fanin)?.fanouts.push(node_id);
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayCopyPlan {
    pub copied_parameters: Vec<(DelayParameter, f64)>,
}

pub fn plan_delay_parameter_duplication(
    from: &DelaySnapshot,
    to_kind: NodeKind,
    delay_flag: bool,
) -> Result<DelayCopyPlan, SpNetworkError> {
    let mut copied_parameters = Vec::new();
    append_delay_parameter_duplication(from, to_kind, delay_flag, &mut copied_parameters)?;
    Ok(DelayCopyPlan { copied_parameters })
}

pub fn speed_delay_parameters_dup(
    from: &NetworkNode,
    to: &mut NetworkNode,
    delay_flag: bool,
) -> Result<(), SpNetworkError> {
    let plan = plan_delay_parameter_duplication(&from.delay, to.kind, delay_flag)?;

    for (parameter, value) in plan.copied_parameters {
        to.delay.set_parameter(parameter, value);
    }

    Ok(())
}

pub fn speed_network_create_from_node(
    source: &SpeedNetwork,
    node_id: usize,
    delay_flag: bool,
) -> Result<SpeedNetwork, SpNetworkError> {
    let source_node = source.node(node_id)?;
    let mut result_nodes = Vec::new();
    let mut copied_by_source = HashMap::new();

    let mut root = source_node.clone();
    root.id = 0;
    root.kind = NodeKind::PrimaryOutput;
    root.fanins.clear();
    root.fanouts.clear();
    speed_delay_parameters_dup(source_node, &mut root, delay_flag)?;
    copied_by_source.insert(source_node.id, root.id);
    result_nodes.push(root);

    for fanin_id in &source_node.fanins {
        let source_fanin = source.node(*fanin_id)?;
        let copy_id = result_nodes.len();
        let mut pi = NetworkNode::new(copy_id, source_fanin.name.clone(), NodeKind::PrimaryInput);
        speed_delay_parameters_dup(source_fanin, &mut pi, delay_flag)?;
        copied_by_source.insert(source_fanin.id, copy_id);
        result_nodes.push(pi);
    }

    let root_fanins = source_node
        .fanins
        .iter()
        .map(|fanin| copied_by_source[fanin])
        .collect::<Vec<_>>();
    result_nodes[0].fanins = root_fanins;

    SpeedNetwork::from_nodes(result_nodes)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayNodeCopy {
    pub original_id: usize,
    pub copied_id: usize,
    pub copied_fanins: Vec<usize>,
    pub library_gate: Option<String>,
}

pub fn network_to_array(network: &SpeedNetwork) -> Result<Vec<NetworkNode>, SpNetworkError> {
    let ordered = dfs_from_input(network)?;
    copy_ordered_nodes(network, &ordered, false, None).map(|result| result.nodes)
}

pub fn network_and_node_to_array(
    network: &SpeedNetwork,
    innode: &SpeedNetwork,
) -> Result<(Vec<NetworkNode>, HashMap<usize, Option<String>>), SpNetworkError> {
    let ordered = dfs_from_input(network)?;
    let copied = copy_ordered_nodes(network, &ordered, true, Some(innode))?;
    Ok((copied.nodes, copied.library_gates))
}

pub fn plan_network_to_array(
    nodes_in_dfs_order: &[NetworkNode],
) -> Result<Vec<ArrayNodeCopy>, SpNetworkError> {
    let network = SpeedNetwork::from_nodes(nodes_in_dfs_order.to_vec())?;
    let nodes = copy_ordered_nodes(
        &network,
        &nodes_in_dfs_order
            .iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        false,
        None,
    )?
    .nodes;

    Ok(nodes
        .into_iter()
        .zip(nodes_in_dfs_order.iter())
        .map(|(node, original)| ArrayNodeCopy {
            original_id: original.id,
            copied_id: node.id,
            copied_fanins: node.fanins,
            library_gate: node.library_gate,
        })
        .collect())
}

pub fn plan_network_and_node_to_array(
    nodes_in_dfs_order: &[NetworkNode],
    original_inputs_by_name: &HashMap<String, usize>,
) -> Result<Vec<ArrayNodeCopy>, SpNetworkError> {
    let mut originals = SpeedNetwork::new();
    originals.nodes = original_inputs_by_name
        .iter()
        .map(|(name, id)| NetworkNode::new(*id, name.clone(), NodeKind::PrimaryInput))
        .collect();

    let network = SpeedNetwork::from_nodes(nodes_in_dfs_order.to_vec())?;
    let nodes = copy_ordered_nodes(
        &network,
        &nodes_in_dfs_order
            .iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        true,
        Some(&originals),
    )?
    .nodes;

    Ok(nodes
        .into_iter()
        .zip(nodes_in_dfs_order.iter())
        .map(|(node, original)| ArrayNodeCopy {
            original_id: original.id,
            copied_id: node.id,
            copied_fanins: node.fanins,
            library_gate: node.library_gate,
        })
        .collect())
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpNetworkError {
    MissingDelayParameter(DelayParameter),
    MissingArrival,
    MissingSpeedArrival,
    MissingLoad,
    MissingNode(usize),
    MissingOriginalInput(String),
    CycleDetected(usize),
}

impl fmt::Display for SpNetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDelayParameter(parameter) => {
                write!(f, "missing delay parameter {parameter:?}")
            }
            Self::MissingArrival => write!(f, "missing ordinary arrival time"),
            Self::MissingSpeedArrival => write!(f, "missing speed arrival time"),
            Self::MissingLoad => write!(f, "missing output load"),
            Self::MissingNode(id) => write!(f, "missing network node {id}"),
            Self::MissingOriginalInput(name) => {
                write!(f, "failed to retrieve the original node named {name}")
            }
            Self::CycleDetected(id) => write!(f, "network contains a cycle through node {id}"),
        }
    }
}

impl Error for SpNetworkError {}

#[derive(Clone, Debug, PartialEq)]
struct CopiedNodes {
    nodes: Vec<NetworkNode>,
    library_gates: HashMap<usize, Option<String>>,
}

fn append_delay_parameter_duplication(
    from: &DelaySnapshot,
    to_kind: NodeKind,
    delay_flag: bool,
    copied_parameters: &mut Vec<(DelayParameter, f64)>,
) -> Result<(), SpNetworkError> {
    for parameter in [
        DelayParameter::BlockRise,
        DelayParameter::DriveRise,
        DelayParameter::BlockFall,
        DelayParameter::DriveFall,
        DelayParameter::MaxInputLoad,
    ] {
        copied_parameters.push((parameter, from.get_parameter(parameter)?));
    }

    match to_kind {
        NodeKind::PrimaryInput => {
            let time = if delay_flag {
                from.speed_arrival
                    .ok_or(SpNetworkError::MissingSpeedArrival)?
            } else {
                from.arrival.ok_or(SpNetworkError::MissingArrival)?
            };
            copied_parameters.push((DelayParameter::ArrivalRise, time.rise));
            copied_parameters.push((DelayParameter::ArrivalFall, time.fall));
        }
        NodeKind::PrimaryOutput => {
            copied_parameters.push((
                DelayParameter::OutputLoad,
                from.load.ok_or(SpNetworkError::MissingLoad)?,
            ));
        }
        NodeKind::Internal => {}
    }

    Ok(())
}

fn copy_ordered_nodes(
    network: &SpeedNetwork,
    ordered: &[usize],
    patch_primary_inputs_to_originals: bool,
    originals: Option<&SpeedNetwork>,
) -> Result<CopiedNodes, SpNetworkError> {
    let id_to_copy = ordered
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect::<HashMap<_, _>>();
    let mut nodes = Vec::with_capacity(ordered.len());
    let mut library_gates = HashMap::new();

    for (copy_id, original_id) in ordered.iter().enumerate() {
        let original = network.node(*original_id)?;
        let mut copy = original.clone();
        copy.id = copy_id;
        copy.fanins.clear();
        copy.fanouts.clear();

        if patch_primary_inputs_to_originals {
            library_gates.insert(copy_id, original.library_gate.clone());
        } else {
            copy.library_gate = None;
        }

        nodes.push(copy);
    }

    for (copy_id, original_id) in ordered.iter().enumerate() {
        let original = network.node(*original_id)?;
        let mut fanins = Vec::with_capacity(original.fanins.len());

        for fanin_id in &original.fanins {
            let fanin = network.node(*fanin_id)?;
            if patch_primary_inputs_to_originals && fanin.kind == NodeKind::PrimaryInput {
                let original_node = originals
                    .and_then(|items| items.name_to_node(&fanin.name))
                    .ok_or_else(|| SpNetworkError::MissingOriginalInput(fanin.name.clone()))?;
                fanins.push(original_node.id);
            } else {
                fanins.push(
                    *id_to_copy
                        .get(fanin_id)
                        .ok_or(SpNetworkError::MissingNode(*fanin_id))?,
                );
            }
        }

        nodes[copy_id].fanins = fanins;
    }

    rebuild_fanouts(&mut nodes);

    Ok(CopiedNodes {
        nodes,
        library_gates,
    })
}

fn dfs_from_input(network: &SpeedNetwork) -> Result<Vec<usize>, SpNetworkError> {
    let mut order = Vec::new();
    let mut state = HashMap::new();

    for input in network.primary_inputs() {
        dfs_fanout_recur(network, input.id, &mut order, &mut state)?;
    }

    for node in network
        .nodes()
        .iter()
        .filter(|node| node.fanins.is_empty() && node.kind != NodeKind::PrimaryInput)
    {
        dfs_fanout_recur(network, node.id, &mut order, &mut state)?;
    }

    Ok(order)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Active,
    Done,
}

fn dfs_fanout_recur(
    network: &SpeedNetwork,
    node_id: usize,
    order: &mut Vec<usize>,
    state: &mut HashMap<usize, VisitState>,
) -> Result<(), SpNetworkError> {
    if let Some(current) = state.get(&node_id) {
        return match current {
            VisitState::Active => Err(SpNetworkError::CycleDetected(node_id)),
            VisitState::Done => Ok(()),
        };
    }

    state.insert(node_id, VisitState::Active);
    for fanout in network.node(node_id)?.fanouts.clone() {
        dfs_fanout_recur(network, fanout, order, state)?;
    }
    state.insert(node_id, VisitState::Done);
    order.push(node_id);
    Ok(())
}

fn rebuild_fanouts(nodes: &mut [NetworkNode]) {
    for node in nodes.iter_mut() {
        node.fanouts.clear();
    }

    let fanins_by_node = nodes
        .iter()
        .map(|node| (node.id, node.fanins.clone()))
        .collect::<Vec<_>>();
    for (node_id, fanins) in fanins_by_node {
        for fanin in fanins {
            if let Some(fanin_node) = nodes.iter_mut().find(|node| node.id == fanin) {
                fanin_node.fanouts.push(node_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> DelaySnapshot {
        let mut params = HashMap::new();
        params.insert(DelayParameter::BlockRise, 1.0);
        params.insert(DelayParameter::DriveRise, 2.0);
        params.insert(DelayParameter::BlockFall, 3.0);
        params.insert(DelayParameter::DriveFall, 4.0);
        params.insert(DelayParameter::MaxInputLoad, 5.0);
        DelaySnapshot {
            params,
            arrival: Some(DelayTime::new(6.0, 7.0)),
            speed_arrival: Some(DelayTime::new(8.0, 9.0)),
            load: Some(10.0),
        }
    }

    #[test]
    fn duplicates_base_delay_parameters_and_pi_arrival_from_selected_source() {
        let source = NetworkNode::new(0, "a", NodeKind::PrimaryInput).with_delay(snapshot());
        let mut normal = NetworkNode::new(1, "a", NodeKind::PrimaryInput);
        speed_delay_parameters_dup(&source, &mut normal, false).unwrap();

        assert_eq!(
            normal
                .delay
                .get_parameter(DelayParameter::ArrivalRise)
                .unwrap(),
            6.0
        );
        assert_eq!(
            normal
                .delay
                .get_parameter(DelayParameter::ArrivalFall)
                .unwrap(),
            7.0
        );

        let mut speed = NetworkNode::new(2, "a", NodeKind::PrimaryInput);
        speed_delay_parameters_dup(&source, &mut speed, true).unwrap();

        assert_eq!(
            speed
                .delay
                .get_parameter(DelayParameter::ArrivalRise)
                .unwrap(),
            8.0
        );
        assert_eq!(
            speed
                .delay
                .get_parameter(DelayParameter::ArrivalFall)
                .unwrap(),
            9.0
        );
        assert_eq!(
            speed
                .delay
                .get_parameter(DelayParameter::BlockRise)
                .unwrap(),
            1.0
        );
    }

    #[test]
    fn duplicates_output_load_for_primary_outputs() {
        let source = NetworkNode::new(0, "n", NodeKind::Internal).with_delay(snapshot());
        let mut output = NetworkNode::new(1, "n", NodeKind::PrimaryOutput);

        speed_delay_parameters_dup(&source, &mut output, false).unwrap();

        assert_eq!(
            output
                .delay
                .get_parameter(DelayParameter::OutputLoad)
                .unwrap(),
            10.0
        );
    }

    #[test]
    fn creates_single_node_network_with_boundary_input_delays() {
        let a = NetworkNode::new(10, "a", NodeKind::PrimaryInput).with_delay(snapshot());
        let b = NetworkNode::new(20, "b", NodeKind::PrimaryInput).with_delay(snapshot());
        let y = NetworkNode::new(30, "y", NodeKind::Internal)
            .with_fanins([10, 20])
            .with_delay(snapshot());
        let source = SpeedNetwork::from_nodes([a, b, y]).unwrap();

        let result = speed_network_create_from_node(&source, 30, true).unwrap();

        assert_eq!(result.nodes().len(), 3);
        assert_eq!(result.primary_inputs().count(), 2);
        assert_eq!(result.primary_outputs().count(), 1);
        assert_eq!(result.node(0).unwrap().kind, NodeKind::PrimaryOutput);
        assert_eq!(result.node(0).unwrap().fanins, vec![1, 2]);
        assert_eq!(result.node(1).unwrap().name, "a");
        assert_eq!(
            result
                .node(1)
                .unwrap()
                .delay
                .get_parameter(DelayParameter::ArrivalRise)
                .unwrap(),
            8.0
        );
        assert_eq!(
            result
                .node(0)
                .unwrap()
                .delay
                .get_parameter(DelayParameter::OutputLoad)
                .unwrap(),
            10.0
        );
    }

    #[test]
    fn network_to_array_rewrites_fanins_to_copied_nodes_and_rebuilds_fanouts() {
        let nodes = [
            NetworkNode::new(10, "a", NodeKind::PrimaryInput),
            NetworkNode::new(20, "n", NodeKind::Internal)
                .with_fanins([10])
                .with_library_gate("g1"),
            NetworkNode::new(30, "y", NodeKind::PrimaryOutput).with_fanins([20]),
        ];
        let network = SpeedNetwork::from_nodes(nodes).unwrap();

        let array = network_to_array(&network).unwrap();

        assert_eq!(
            array.iter().map(|node| node.id).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(array[0].name, "y");
        assert_eq!(array[0].fanins, vec![1]);
        assert_eq!(array[1].name, "n");
        assert_eq!(array[1].fanins, vec![2]);
        assert_eq!(array[1].fanouts, vec![0]);
        assert_eq!(array[1].library_gate, None);
        assert_eq!(array[2].fanouts, vec![1]);
    }

    #[test]
    fn network_and_node_to_array_patches_primary_inputs_to_original_nodes() {
        let network = SpeedNetwork::from_nodes([
            NetworkNode::new(10, "a", NodeKind::PrimaryInput),
            NetworkNode::new(20, "n", NodeKind::Internal)
                .with_fanins([10])
                .with_library_gate("g1"),
            NetworkNode::new(30, "y", NodeKind::PrimaryOutput).with_fanins([20]),
        ])
        .unwrap();
        let originals =
            SpeedNetwork::from_nodes([NetworkNode::new(99, "a", NodeKind::PrimaryInput)]).unwrap();

        let (array, table) = network_and_node_to_array(&network, &originals).unwrap();

        assert_eq!(array[1].name, "n");
        assert_eq!(array[1].fanins, vec![99]);
        assert_eq!(table.get(&1), Some(&Some("g1".to_string())));
    }

    #[test]
    fn reports_missing_original_input_when_patching_array() {
        let network = SpeedNetwork::from_nodes([
            NetworkNode::new(10, "a", NodeKind::PrimaryInput),
            NetworkNode::new(20, "n", NodeKind::Internal).with_fanins([10]),
        ])
        .unwrap();
        let originals = SpeedNetwork::new();

        assert_eq!(
            network_and_node_to_array(&network, &originals),
            Err(SpNetworkError::MissingOriginalInput("a".to_string()))
        );
    }
}
