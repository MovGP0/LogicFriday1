//! Native Rust model for the ITE improvement pass.
//!
//! The original SIS unit coordinates iterative decomposition, partial
//! collapse, last-gasp remapping, alternate ITE/BDD representations, and node
//! replacement. The SIS-specific decomposition and mapping operations are
//! represented by a backend trait so the owned-data control flow can be tested
//! independently from the still-separate graph ports.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecompMethod {
    GoodDecomp,
    Factor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapMethod {
    Old,
    New,
    WithIter,
    WithJustDecomp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollapseUpdate {
    Inexpensive,
    Full,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActInitParams {
    pub num_iter: usize,
    pub fanin_collapse: usize,
    pub decomp_fanin: usize,
    pub quick_phase: bool,
    pub heuristic_num: i32,
    pub disjoint_decomp: bool,
    pub gain_factor: f64,
    pub map_alg: i32,
    pub mode: f64,
    pub collapse_fanins_of_fanout: usize,
    pub ite_fanin_limit_for_bdd: usize,
    pub cost_limit: i32,
    pub last_gasp: bool,
    pub break_network: bool,
    pub alternate_rep: bool,
    pub collapse_update: CollapseUpdate,
    pub map_method: MapMethod,
    pub var_selection_lit: i32,
    pub collapse_method: i32,
    pub decomp_method: DecompMethod,
}

impl ActInitParams {
    pub fn remap_params(&self, decomp_context: bool) -> Self {
        let mut params = Self {
            num_iter: 1,
            fanin_collapse: 8,
            decomp_fanin: 4,
            quick_phase: !decomp_context,
            heuristic_num: if decomp_context {
                self.heuristic_num
            } else {
                3
            },
            disjoint_decomp: false,
            gain_factor: 0.001,
            map_alg: self.map_alg,
            mode: 0.0,
            collapse_fanins_of_fanout: 15,
            ite_fanin_limit_for_bdd: 40,
            cost_limit: 3,
            last_gasp: false,
            break_network: true,
            alternate_rep: false,
            collapse_update: CollapseUpdate::Inexpensive,
            map_method: self.map_method,
            var_selection_lit: self.var_selection_lit,
            collapse_method: self.collapse_method,
            decomp_method: self.decomp_method,
        };

        if !decomp_context {
            params.decomp_fanin = 4;
        }

        params
    }
}

impl Default for ActInitParams {
    fn default() -> Self {
        Self {
            num_iter: 0,
            fanin_collapse: 0,
            decomp_fanin: 4,
            quick_phase: false,
            heuristic_num: 0,
            disjoint_decomp: false,
            gain_factor: 0.001,
            map_alg: 0,
            mode: 0.0,
            collapse_fanins_of_fanout: 15,
            ite_fanin_limit_for_bdd: 40,
            cost_limit: 3,
            last_gasp: false,
            break_network: false,
            alternate_rep: false,
            collapse_update: CollapseUpdate::Inexpensive,
            map_method: MapMethod::New,
            var_selection_lit: 0,
            collapse_method: 0,
            decomp_method: DecompMethod::GoodDecomp,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteImpNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub cube_count: usize,
    pub literal_count: usize,
    pub cost: i32,
    pub arrival_time: i32,
    pub has_ite: bool,
    pub has_act: bool,
    pub implementation_inputs: Vec<String>,
}

impl IteImpNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            cube_count: 0,
            literal_count: 0,
            cost: 0,
            arrival_time: 0,
            has_ite: false,
            has_act: false,
            implementation_inputs: Vec::new(),
        }
    }

    pub fn primary_input(name: impl Into<String>) -> Self {
        Self::new(name, NodeKind::PrimaryInput)
    }

    pub fn primary_output(name: impl Into<String>, fanins: Vec<NodeId>) -> Self {
        let mut node = Self::new(name, NodeKind::PrimaryOutput);
        node.fanins = fanins;
        node
    }

    pub fn internal(name: impl Into<String>, fanins: Vec<NodeId>, cost: i32) -> Self {
        let mut node = Self::new(name, NodeKind::Internal);
        node.fanins = fanins;
        node.cost = cost;
        node
    }

    pub fn with_cover(mut self, cube_count: usize, literal_count: usize) -> Self {
        self.cube_count = cube_count;
        self.literal_count = literal_count;
        self
    }

    fn is_primary(&self) -> bool {
        matches!(self.kind, NodeKind::PrimaryInput | NodeKind::PrimaryOutput)
    }

    fn is_single_mux_optimal_shape(&self) -> bool {
        self.cube_count == 1 || self.cube_count == self.literal_count
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IteImpNetwork {
    nodes: Vec<IteImpNode>,
}

impl IteImpNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: IteImpNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> IteImpResult<&IteImpNode> {
        self.nodes.get(id.0).ok_or(IteImpError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> IteImpResult<&mut IteImpNode> {
        self.nodes.get_mut(id.0).ok_or(IteImpError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[IteImpNode] {
        &self.nodes
    }

    pub fn dfs_node_ids(&self) -> Vec<NodeId> {
        (0..self.nodes.len()).map(NodeId).collect()
    }

    pub fn internal_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Internal)
            .count()
    }

    pub fn total_cost(&self) -> i32 {
        self.nodes
            .iter()
            .filter(|node| !node.is_primary())
            .map(|node| node.cost)
            .sum()
    }

    fn add_remapped_internal(&mut self, mut node: IteImpNode) -> NodeId {
        node.kind = NodeKind::Internal;
        self.add_node(node)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemapReason {
    Decomposition,
    LastGasp,
    NetworkRemap,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RemapContext {
    pub reason: RemapReason,
    pub params: ActInitParams,
}

impl RemapContext {
    pub fn is_decomposition(&self) -> bool {
        self.reason == RemapReason::Decomposition
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRemap {
    pub mapped_cost: i32,
    pub added_internal_nodes: Vec<IteImpNode>,
    pub replacement: IteImpNode,
}

impl NodeRemap {
    pub fn new(mapped_cost: i32, replacement: IteImpNode) -> Self {
        Self {
            mapped_cost,
            added_internal_nodes: Vec::new(),
            replacement,
        }
    }

    pub fn with_added_nodes(mut self, nodes: Vec<IteImpNode>) -> Self {
        self.added_internal_nodes = nodes;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImprovementReport {
    pub iterations: usize,
    pub decomp_gain: i32,
    pub collapse_gain: i32,
    pub last_gasp_gain: i32,
    pub total_cost: i32,
}

pub trait IteImpBackend {
    fn partial_collapse(
        &mut self,
        network: &mut IteImpNetwork,
        params: &ActInitParams,
    ) -> IteImpResult<i32>;

    fn map_factored_form(
        &mut self,
        node: &mut IteImpNode,
        params: &ActInitParams,
    ) -> IteImpResult<i32>;

    fn use_alternate_rep(
        &mut self,
        node: &mut IteImpNode,
        params: &ActInitParams,
    ) -> IteImpResult<bool>;

    fn remap_node(
        &mut self,
        network: &IteImpNetwork,
        node: NodeId,
        context: &RemapContext,
    ) -> IteImpResult<Option<NodeRemap>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteImpError {
    UnknownNode(NodeId),
    InvalidRemap { node: NodeId, message: String },
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for IteImpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown ITE improvement node {}", node.0),
            Self::InvalidRemap { node, message } => {
                write!(f, "invalid ITE remap for node {}: {message}", node.0)
            }
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} requires native Rust ports for SIS graph dependencies"
            ),
        }
    }
}

impl Error for IteImpError {}

pub type IteImpResult<T> = Result<T, IteImpError>;

pub struct MissingIteImpBackend;

impl IteImpBackend for MissingIteImpBackend {
    fn partial_collapse(
        &mut self,
        _network: &mut IteImpNetwork,
        _params: &ActInitParams,
    ) -> IteImpResult<i32> {
        Err(missing_native_ports("act_ite_partial_collapse"))
    }

    fn map_factored_form(
        &mut self,
        _node: &mut IteImpNode,
        _params: &ActInitParams,
    ) -> IteImpResult<i32> {
        Err(missing_native_ports("act_ite_map_factored_form"))
    }

    fn use_alternate_rep(
        &mut self,
        _node: &mut IteImpNode,
        _params: &ActInitParams,
    ) -> IteImpResult<bool> {
        Err(missing_native_ports("act_ite_use_alternate_rep"))
    }

    fn remap_node(
        &mut self,
        _network: &IteImpNetwork,
        _node: NodeId,
        context: &RemapContext,
    ) -> IteImpResult<Option<NodeRemap>> {
        let operation = match context.reason {
            RemapReason::Decomposition => "act_ite_node_remap decomposition",
            RemapReason::LastGasp => "act_ite_node_remap last-gasp",
            RemapReason::NetworkRemap => "act_ite_network_remap",
        };

        Err(missing_native_ports(operation))
    }
}

pub fn act_ite_iterative_improvement<B: IteImpBackend>(
    network: &mut IteImpNetwork,
    init_params: &ActInitParams,
    backend: &mut B,
) -> IteImpResult<ImprovementReport> {
    let mut report = if init_params.num_iter > 0 {
        ite_improve_network(network, init_params, backend)?
    } else {
        ImprovementReport {
            iterations: 0,
            decomp_gain: 0,
            collapse_gain: 0,
            last_gasp_gain: 0,
            total_cost: network.total_cost(),
        }
    };

    if init_params.last_gasp {
        report.last_gasp_gain = act_last_gasp(network, init_params, backend)?;
        report.total_cost = network.total_cost();
    }

    Ok(report)
}

pub fn act_ite_iterative_improvement_blocked(
    network: &mut IteImpNetwork,
    init_params: &ActInitParams,
) -> IteImpResult<ImprovementReport> {
    let mut backend = MissingIteImpBackend;
    act_ite_iterative_improvement(network, init_params, &mut backend)
}

pub fn ite_improve_network<B: IteImpBackend>(
    network: &mut IteImpNetwork,
    init_params: &ActInitParams,
    backend: &mut B,
) -> IteImpResult<ImprovementReport> {
    let mut report = ImprovementReport {
        iterations: 0,
        decomp_gain: 0,
        collapse_gain: 0,
        last_gasp_gain: 0,
        total_cost: network.total_cost(),
    };

    while report.iterations < init_params.num_iter {
        let decomp_gain = ite_decomp_big_nodes(network, init_params, backend)?;
        let collapse_gain = backend.partial_collapse(network, init_params)?;
        let iteration_gain = decomp_gain + collapse_gain;

        report.iterations += 1;
        report.decomp_gain += decomp_gain;
        report.collapse_gain += collapse_gain;
        report.total_cost = network.total_cost();

        if iteration_gain <= 0 {
            break;
        }
    }

    Ok(report)
}

pub fn ite_decomp_big_nodes<B: IteImpBackend>(
    network: &mut IteImpNetwork,
    init_params: &ActInitParams,
    backend: &mut B,
) -> IteImpResult<i32> {
    let mut gain = 0;
    let node_ids = network.dfs_node_ids();

    for node_id in node_ids {
        if !should_decompose_node(network.node(node_id)?, init_params) {
            continue;
        }

        gain += match init_params.decomp_method {
            DecompMethod::GoodDecomp => {
                act_ite_node_remap(network, node_id, true, init_params, backend)?
            }
            DecompMethod::Factor => {
                let node = network.node_mut(node_id)?;
                backend.map_factored_form(node, init_params)?
            }
        };
    }

    Ok(gain)
}

pub fn act_ite_network_remap<B: IteImpBackend>(
    network: &mut IteImpNetwork,
    init_params: &ActInitParams,
    backend: &mut B,
) -> IteImpResult<i32> {
    let mut gain = 0;

    for node_id in network.dfs_node_ids() {
        gain += act_ite_node_remap_with_reason(
            network,
            node_id,
            RemapReason::NetworkRemap,
            init_params,
            backend,
        )?;
    }

    Ok(gain)
}

pub fn act_last_gasp<B: IteImpBackend>(
    network: &mut IteImpNetwork,
    init_params: &ActInitParams,
    backend: &mut B,
) -> IteImpResult<i32> {
    let mut gain = 0;

    for node_id in network.dfs_node_ids() {
        gain += act_ite_node_remap_with_reason(
            network,
            node_id,
            RemapReason::LastGasp,
            init_params,
            backend,
        )?;
    }

    Ok(gain)
}

pub fn act_ite_node_remap<B: IteImpBackend>(
    network: &mut IteImpNetwork,
    node_id: NodeId,
    decomp_context: bool,
    init_params: &ActInitParams,
    backend: &mut B,
) -> IteImpResult<i32> {
    let reason = if decomp_context {
        RemapReason::Decomposition
    } else {
        RemapReason::LastGasp
    };

    act_ite_node_remap_with_reason(network, node_id, reason, init_params, backend)
}

pub fn act_ite_node_remap_with_reason<B: IteImpBackend>(
    network: &mut IteImpNetwork,
    node_id: NodeId,
    reason: RemapReason,
    init_params: &ActInitParams,
    backend: &mut B,
) -> IteImpResult<i32> {
    let original = network.node(node_id)?.clone();

    if should_skip_node_remap(&original) {
        return Ok(0);
    }

    let mut alternate_gain = 0;
    let mut cost_node_original = original.cost;

    if reason != RemapReason::Decomposition && init_params.alternate_rep {
        let node = network.node_mut(node_id)?;

        if backend.use_alternate_rep(node, init_params)? {
            alternate_gain = cost_node_original - node.cost;
            cost_node_original = node.cost;

            if cost_node_original <= 2 {
                return Ok(alternate_gain);
            }
        }
    }

    let context = RemapContext {
        reason,
        params: init_params.remap_params(reason == RemapReason::Decomposition),
    };
    let Some(remap) = backend.remap_node(network, node_id, &context)? else {
        return Ok(alternate_gain);
    };

    let gain = cost_node_original - remap.mapped_cost;

    if gain <= 0 {
        return Ok(alternate_gain);
    }

    apply_node_remap(network, node_id, remap)?;
    Ok(gain + alternate_gain)
}

pub fn act_ite_network_update_pi(
    node: &mut IteImpNode,
    replacement_inputs: impl IntoIterator<Item = impl Into<String>>,
) {
    node.implementation_inputs = replacement_inputs
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
}

pub fn ite_print_network(network: &IteImpNetwork) -> i32 {
    network.total_cost()
}

pub fn act_ite_use_alternate_rep<B: IteImpBackend>(
    node: &mut IteImpNode,
    init_params: &ActInitParams,
    backend: &mut B,
) -> IteImpResult<bool> {
    if node.is_primary() || init_params.heuristic_num == 3 || node.cost <= 2 {
        return Ok(false);
    }

    backend.use_alternate_rep(node, init_params)
}

pub fn act_ite_use_alternate_rep_blocked(
    node: &mut IteImpNode,
    init_params: &ActInitParams,
) -> IteImpResult<bool> {
    let mut backend = MissingIteImpBackend;
    act_ite_use_alternate_rep(node, init_params, &mut backend)
}

fn should_decompose_node(node: &IteImpNode, init_params: &ActInitParams) -> bool {
    !node.is_primary()
        && node.fanins.len() >= init_params.decomp_fanin
        && node.cost > 2
        && !node.is_single_mux_optimal_shape()
}

fn should_skip_node_remap(node: &IteImpNode) -> bool {
    node.is_primary() || node.cost <= 2 || node.is_single_mux_optimal_shape()
}

fn apply_node_remap(
    network: &mut IteImpNetwork,
    node_id: NodeId,
    remap: NodeRemap,
) -> IteImpResult<()> {
    if remap.mapped_cost < 0 {
        return Err(IteImpError::InvalidRemap {
            node: node_id,
            message: "mapped cost must be non-negative".to_owned(),
        });
    }

    let added_ids = remap
        .added_internal_nodes
        .into_iter()
        .map(|node| network.add_remapped_internal(node))
        .collect::<Vec<_>>();

    let original_name = network.node(node_id)?.name.clone();
    let mut replacement = remap.replacement;
    replacement.kind = NodeKind::Internal;
    replacement.cost = remap.mapped_cost;

    if replacement.name.is_empty() {
        replacement.name = original_name;
    }

    if replacement.fanins.is_empty() && !added_ids.is_empty() {
        replacement.fanins = added_ids;
    }

    *network.node_mut(node_id)? = replacement;
    Ok(())
}

fn missing_native_ports(operation: &'static str) -> IteImpError {
    IteImpError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct FakeBackend {
        collapse_gains: VecDeque<i32>,
        remaps: VecDeque<Option<NodeRemap>>,
        alternate_cost: Option<i32>,
        factored_gain: i32,
        remap_reasons: Vec<RemapReason>,
        remap_params: Vec<ActInitParams>,
    }

    impl IteImpBackend for FakeBackend {
        fn partial_collapse(
            &mut self,
            _network: &mut IteImpNetwork,
            _params: &ActInitParams,
        ) -> IteImpResult<i32> {
            Ok(self.collapse_gains.pop_front().unwrap_or(0))
        }

        fn map_factored_form(
            &mut self,
            node: &mut IteImpNode,
            _params: &ActInitParams,
        ) -> IteImpResult<i32> {
            node.cost -= self.factored_gain;
            Ok(self.factored_gain)
        }

        fn use_alternate_rep(
            &mut self,
            node: &mut IteImpNode,
            _params: &ActInitParams,
        ) -> IteImpResult<bool> {
            if let Some(cost) = self.alternate_cost {
                node.cost = cost;
                Ok(true)
            } else {
                Ok(false)
            }
        }

        fn remap_node(
            &mut self,
            _network: &IteImpNetwork,
            _node: NodeId,
            context: &RemapContext,
        ) -> IteImpResult<Option<NodeRemap>> {
            self.remap_reasons.push(context.reason);
            self.remap_params.push(context.params.clone());
            Ok(self.remaps.pop_front().unwrap_or(None))
        }
    }

    fn expensive_internal(name: &str, fanin_count: usize, cost: i32) -> IteImpNode {
        let fanins = (0..fanin_count).map(NodeId).collect::<Vec<_>>();
        IteImpNode::internal(name, fanins, cost).with_cover(2, fanin_count + 1)
    }

    #[test]
    fn iterative_improvement_stops_after_no_gain() {
        let mut network = IteImpNetwork::new();
        network.add_node(expensive_internal("n1", 4, 8));

        let mut backend = FakeBackend {
            collapse_gains: VecDeque::from([1, -1]),
            remaps: VecDeque::from([
                Some(NodeRemap::new(
                    5,
                    IteImpNode::internal("n1_remap", Vec::new(), 0),
                )),
                Some(NodeRemap::new(
                    4,
                    IteImpNode::internal("n1_remap2", Vec::new(), 0),
                )),
            ]),
            ..FakeBackend::default()
        };
        let params = ActInitParams {
            num_iter: 5,
            decomp_fanin: 4,
            ..ActInitParams::default()
        };

        let report = act_ite_iterative_improvement(&mut network, &params, &mut backend).unwrap();

        assert_eq!(report.iterations, 2);
        assert_eq!(report.decomp_gain, 3);
        assert_eq!(report.collapse_gain, 0);
        assert_eq!(network.node(NodeId(0)).unwrap().cost, 5);
    }

    #[test]
    fn decomp_big_nodes_skips_primary_cheap_small_and_single_cube_nodes() {
        let mut network = IteImpNetwork::new();
        network.add_node(IteImpNode::primary_input("a"));
        network.add_node(expensive_internal("small", 3, 8));
        network.add_node(expensive_internal("cheap", 4, 2));
        network.add_node(expensive_internal("single", 4, 8).with_cover(1, 6));
        network.add_node(expensive_internal("candidate", 4, 9));

        let mut backend = FakeBackend {
            remaps: VecDeque::from([Some(NodeRemap::new(
                6,
                IteImpNode::internal("candidate_new", Vec::new(), 0),
            ))]),
            ..FakeBackend::default()
        };
        let params = ActInitParams {
            decomp_fanin: 4,
            ..ActInitParams::default()
        };

        let gain = ite_decomp_big_nodes(&mut network, &params, &mut backend).unwrap();

        assert_eq!(gain, 3);
        assert_eq!(backend.remap_reasons, vec![RemapReason::Decomposition]);
        assert_eq!(network.node(NodeId(4)).unwrap().name, "candidate_new");
    }

    #[test]
    fn alternate_rep_gain_can_finish_remap_early() {
        let mut network = IteImpNetwork::new();
        network.add_node(expensive_internal("n1", 4, 5));

        let mut backend = FakeBackend {
            alternate_cost: Some(2),
            ..FakeBackend::default()
        };
        let params = ActInitParams {
            alternate_rep: true,
            heuristic_num: 1,
            ..ActInitParams::default()
        };

        let gain =
            act_ite_node_remap(&mut network, NodeId(0), false, &params, &mut backend).unwrap();

        assert_eq!(gain, 3);
        assert!(backend.remap_reasons.is_empty());
        assert_eq!(network.node(NodeId(0)).unwrap().cost, 2);
    }

    #[test]
    fn positive_remap_adds_internal_nodes_and_replaces_node() {
        let mut network = IteImpNetwork::new();
        network.add_node(expensive_internal("n1", 4, 10));

        let added = IteImpNode::internal("helper", Vec::new(), 1);
        let remap = NodeRemap::new(6, IteImpNode::internal("", Vec::new(), 0))
            .with_added_nodes(vec![added]);
        let mut backend = FakeBackend {
            remaps: VecDeque::from([Some(remap)]),
            ..FakeBackend::default()
        };

        let gain = act_ite_node_remap(
            &mut network,
            NodeId(0),
            true,
            &ActInitParams::default(),
            &mut backend,
        )
        .unwrap();

        assert_eq!(gain, 4);
        assert_eq!(network.nodes().len(), 2);
        assert_eq!(network.node(NodeId(0)).unwrap().name, "n1");
        assert_eq!(network.node(NodeId(0)).unwrap().cost, 6);
        assert_eq!(network.node(NodeId(0)).unwrap().fanins, vec![NodeId(1)]);
    }

    #[test]
    fn factored_decomp_uses_factored_backend_for_candidates() {
        let mut network = IteImpNetwork::new();
        network.add_node(expensive_internal("n1", 4, 9));

        let mut backend = FakeBackend {
            factored_gain: 2,
            ..FakeBackend::default()
        };
        let params = ActInitParams {
            decomp_method: DecompMethod::Factor,
            decomp_fanin: 4,
            ..ActInitParams::default()
        };

        let gain = ite_decomp_big_nodes(&mut network, &params, &mut backend).unwrap();

        assert_eq!(gain, 2);
        assert_eq!(network.node(NodeId(0)).unwrap().cost, 7);
        assert!(backend.remap_reasons.is_empty());
    }

    #[test]
    fn missing_backend_returns_generic_runtime_diagnostic() {
        let mut network = IteImpNetwork::new();
        network.add_node(expensive_internal("n1", 4, 9));

        let error = act_ite_iterative_improvement_blocked(
            &mut network,
            &ActInitParams {
                num_iter: 1,
                ..ActInitParams::default()
            },
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "act_ite_node_remap decomposition requires native Rust ports for SIS graph dependencies"
        );
    }

    #[test]
    fn blocked_path_keeps_runtime_diagnostics_generic() {
        let error = missing_native_ports("act_ite_node_remap");

        assert!(error.to_string().contains("native Rust ports"));
    }
}
