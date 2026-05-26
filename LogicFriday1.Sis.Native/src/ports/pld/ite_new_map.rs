//! Native Rust model for `LogicSynthesis/sis/pld/ite_new_map.c`.
//!
//! The C unit maps one SIS node by making a temporary network from that node,
//! forcing a one-iteration `NEW` mapping pass, optionally repeating the pass
//! for expensive non-AND/OR nodes under `MAP_WITH_ITER`, and storing the
//! temporary network plus its non-zero mapped cost in the node's ACT/ITE slot.
//! This file ports that control flow over owned Rust data. Direct SIS-backed
//! integration points are represented by explicit dependency errors until the
//! prerequisite C files have native Rust ports.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapMethod {
    Old,
    New,
    WithIter,
    WithJustDecomp,
    Other(i32),
}

impl MapMethod {
    pub fn from_c_value(value: i32) -> Self {
        match value {
            0 => Self::Old,
            1 => Self::New,
            2 => Self::WithIter,
            3 => Self::WithJustDecomp,
            other => Self::Other(other),
        }
    }

    pub fn c_value(self) -> i32 {
        match self {
            Self::Old => 0,
            Self::New => 1,
            Self::WithIter => 2,
            Self::WithJustDecomp => 3,
            Self::Other(value) => value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActInitParams {
    pub num_iter: i32,
    pub break_after_map: i32,
    pub last_gasp: i32,
    pub map_method: MapMethod,
    pub fanin_collapse: i32,
}

impl ActInitParams {
    pub fn new(map_method: MapMethod) -> Self {
        Self {
            num_iter: 0,
            break_after_map: 0,
            last_gasp: 0,
            map_method,
            fanin_collapse: 1,
        }
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
    PrimaryInput,
    PrimaryOutput,
    Zero,
    One,
    Buffer,
    And,
    Or,
    Other,
}

impl NodeFunction {
    fn is_trivial_cost(self) -> bool {
        matches!(self, Self::Zero | Self::One | Self::Buffer)
    }

    fn suppress_second_iter_pass(self) -> bool {
        matches!(self, Self::And | Self::Or)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActIteSlot {
    pub cost: i32,
    pub network: Option<MappedNetwork>,
}

impl ActIteSlot {
    pub fn new(cost: i32) -> Self {
        Self {
            cost,
            network: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappedNode {
    pub name: String,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub slot: ActIteSlot,
}

impl MappedNode {
    pub fn new(name: impl Into<String>, kind: NodeKind, function: NodeFunction) -> Self {
        Self {
            name: name.into(),
            kind,
            function,
            slot: ActIteSlot::new(0),
        }
    }

    pub fn internal(name: impl Into<String>, function: NodeFunction) -> Self {
        Self::new(name, NodeKind::Internal, function)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MappedNetwork {
    pub nodes: Vec<MappedNode>,
    pub free_count: usize,
}

impl MappedNetwork {
    pub fn new(nodes: Vec<MappedNode>) -> Self {
        Self {
            nodes,
            free_count: 0,
        }
    }
}

pub trait IteNewMapHooks {
    fn create_network_from_node(&mut self, node: &MappedNode) -> IteNewMapResult<MappedNetwork>;
    fn map_network_with_iter(
        &mut self,
        network: &mut MappedNetwork,
        init_params: &ActInitParams,
    ) -> IteNewMapResult<()>;
    fn free_ite_network(&mut self, network: &mut MappedNetwork) -> IteNewMapResult<()>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteNewMapError {
    MissingNativePorts { operation: &'static str },
    MappingFailed(String),
}

impl fmt::Display for IteNewMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by unported SIS dependencies")
            }
            Self::MappingFailed(message) => write!(f, "ITE new-map operation failed: {message}"),
        }
    }
}

impl Error for IteNewMapError {}

pub type IteNewMapResult<T> = Result<T, IteNewMapError>;

pub fn act_ite_map_node_with_iter_imp_blocked(
    _node: &mut MappedNode,
    _init_params: &mut ActInitParams,
) -> IteNewMapResult<i32> {
    Err(missing_native_ports(
        "act_ite_map_node_with_iter_imp SIS node/network integration",
    ))
}

pub fn network_num_nonzero_cost_nodes_blocked<Network>(_network: &Network) -> IteNewMapResult<i32> {
    Err(missing_native_ports(
        "network_num_nonzero_cost_nodes SIS network traversal",
    ))
}

pub fn act_ite_map_node_with_iter_imp<H: IteNewMapHooks>(
    node: &mut MappedNode,
    init_params: &mut ActInitParams,
    hooks: &mut H,
) -> IteNewMapResult<i32> {
    if matches!(
        node.function,
        NodeFunction::PrimaryInput | NodeFunction::PrimaryOutput
    ) {
        return Ok(0);
    }

    if node.function.is_trivial_cost() {
        node.slot.network = None;
        node.slot.cost = 0;
        return Ok(0);
    }

    let save_params = init_params.clone();
    let mut network = hooks.create_network_from_node(node)?;

    if init_params.map_method == MapMethod::WithJustDecomp {
        init_params.fanin_collapse = 0;
    }
    init_params.num_iter = 1;
    init_params.map_method = MapMethod::New;
    init_params.break_after_map = 1;
    init_params.last_gasp = 0;

    hooks.map_network_with_iter(&mut network, init_params)?;
    hooks.free_ite_network(&mut network)?;
    node.slot.cost = network_num_nonzero_cost_nodes(&network);

    if node.slot.cost > 2
        && !node.function.suppress_second_iter_pass()
        && save_params.map_method == MapMethod::WithIter
    {
        hooks.map_network_with_iter(&mut network, init_params)?;
        hooks.free_ite_network(&mut network)?;
        node.slot.cost = network_num_nonzero_cost_nodes(&network);
    }

    node.slot.network = Some(network);
    *init_params = save_params;
    Ok(node.slot.cost)
}

pub fn network_num_nonzero_cost_nodes(network: &MappedNetwork) -> i32 {
    network
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Internal && !node.function.is_trivial_cost())
        .count() as i32
}

fn missing_native_ports(operation: &'static str) -> IteNewMapError {
    IteNewMapError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingHooks {
        created_network: MappedNetwork,
        map_snapshots: Vec<ActInitParams>,
        costs_after_each_pass: Vec<Vec<NodeFunction>>,
    }

    impl RecordingHooks {
        fn with_passes(passes: Vec<Vec<NodeFunction>>) -> Self {
            Self {
                created_network: MappedNetwork::new(Vec::new()),
                map_snapshots: Vec::new(),
                costs_after_each_pass: passes,
            }
        }
    }

    impl IteNewMapHooks for RecordingHooks {
        fn create_network_from_node(
            &mut self,
            _node: &MappedNode,
        ) -> IteNewMapResult<MappedNetwork> {
            Ok(self.created_network.clone())
        }

        fn map_network_with_iter(
            &mut self,
            network: &mut MappedNetwork,
            init_params: &ActInitParams,
        ) -> IteNewMapResult<()> {
            self.map_snapshots.push(init_params.clone());
            let pass = self.map_snapshots.len() - 1;
            let functions = self
                .costs_after_each_pass
                .get(pass)
                .ok_or_else(|| IteNewMapError::MappingFailed(format!("missing pass {pass}")))?;
            network.nodes = functions
                .iter()
                .enumerate()
                .map(|(index, function)| MappedNode::internal(format!("n{index}"), *function))
                .collect();
            Ok(())
        }

        fn free_ite_network(&mut self, network: &mut MappedNetwork) -> IteNewMapResult<()> {
            network.free_count += 1;
            Ok(())
        }
    }

    #[test]
    fn count_ignores_primary_and_trivial_internal_nodes() {
        let network = MappedNetwork::new(vec![
            MappedNode::new("pi", NodeKind::PrimaryInput, NodeFunction::PrimaryInput),
            MappedNode::internal("zero", NodeFunction::Zero),
            MappedNode::internal("buf", NodeFunction::Buffer),
            MappedNode::internal("and", NodeFunction::And),
            MappedNode::internal("other", NodeFunction::Other),
            MappedNode::new("po", NodeKind::PrimaryOutput, NodeFunction::PrimaryOutput),
        ]);

        assert_eq!(network_num_nonzero_cost_nodes(&network), 2);
    }

    #[test]
    fn primary_nodes_return_zero_without_touching_existing_slot() {
        let mut node = MappedNode::new("pi", NodeKind::PrimaryInput, NodeFunction::PrimaryInput);
        node.slot.cost = 7;
        node.slot.network = Some(MappedNetwork::default());
        let mut params = ActInitParams::new(MapMethod::WithIter);
        let mut hooks = RecordingHooks::default();

        assert_eq!(
            act_ite_map_node_with_iter_imp(&mut node, &mut params, &mut hooks).unwrap(),
            0
        );
        assert_eq!(node.slot.cost, 7);
        assert!(node.slot.network.is_some());
        assert!(hooks.map_snapshots.is_empty());
    }

    #[test]
    fn constants_and_buffers_clear_network_and_cost() {
        let mut node = MappedNode::internal("buf", NodeFunction::Buffer);
        node.slot.cost = 4;
        node.slot.network = Some(MappedNetwork::default());
        let mut params = ActInitParams::new(MapMethod::New);
        let mut hooks = RecordingHooks::default();

        assert_eq!(
            act_ite_map_node_with_iter_imp(&mut node, &mut params, &mut hooks).unwrap(),
            0
        );
        assert_eq!(node.slot, ActIteSlot::new(0));
    }

    #[test]
    fn mapping_forces_one_new_pass_and_restores_parameters() {
        let original = ActInitParams {
            num_iter: 9,
            break_after_map: 0,
            last_gasp: 1,
            map_method: MapMethod::WithJustDecomp,
            fanin_collapse: 1,
        };
        let mut params = original.clone();
        let mut node = MappedNode::internal("f", NodeFunction::Other);
        let mut hooks = RecordingHooks::with_passes(vec![vec![
            NodeFunction::Other,
            NodeFunction::Zero,
            NodeFunction::Buffer,
            NodeFunction::And,
        ]]);

        assert_eq!(
            act_ite_map_node_with_iter_imp(&mut node, &mut params, &mut hooks).unwrap(),
            2
        );

        assert_eq!(params, original);
        assert_eq!(hooks.map_snapshots.len(), 1);
        assert_eq!(
            hooks.map_snapshots[0],
            ActInitParams {
                num_iter: 1,
                break_after_map: 1,
                last_gasp: 0,
                map_method: MapMethod::New,
                fanin_collapse: 0,
            }
        );
        assert_eq!(node.slot.cost, 2);
        assert_eq!(node.slot.network.as_ref().unwrap().free_count, 1);
    }

    #[test]
    fn expensive_iterative_non_gate_nodes_get_second_pass() {
        let mut params = ActInitParams {
            num_iter: 3,
            break_after_map: 0,
            last_gasp: 1,
            map_method: MapMethod::WithIter,
            fanin_collapse: 1,
        };
        let mut node = MappedNode::internal("f", NodeFunction::Other);
        let mut hooks = RecordingHooks::with_passes(vec![
            vec![
                NodeFunction::Other,
                NodeFunction::And,
                NodeFunction::Or,
                NodeFunction::Other,
            ],
            vec![NodeFunction::Other, NodeFunction::Buffer],
        ]);

        assert_eq!(
            act_ite_map_node_with_iter_imp(&mut node, &mut params, &mut hooks).unwrap(),
            1
        );

        assert_eq!(hooks.map_snapshots.len(), 2);
        assert_eq!(node.slot.cost, 1);
        assert_eq!(node.slot.network.as_ref().unwrap().free_count, 2);
    }

    #[test]
    fn and_or_nodes_do_not_get_second_iterative_pass() {
        let mut params = ActInitParams::new(MapMethod::WithIter);
        let mut node = MappedNode::internal("and", NodeFunction::And);
        let mut hooks = RecordingHooks::with_passes(vec![vec![
            NodeFunction::Other,
            NodeFunction::And,
            NodeFunction::Or,
        ]]);

        assert_eq!(
            act_ite_map_node_with_iter_imp(&mut node, &mut params, &mut hooks).unwrap(),
            3
        );

        assert_eq!(hooks.map_snapshots.len(), 1);
    }
}
