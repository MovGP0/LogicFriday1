//! Native Rust port scaffold for `LogicSynthesis/sis/speed/speedup.c`.
//!
//! The C file is the old SIS speed-up driver. It computes criticality, orders
//! cutset nodes, decides whether a node should be decomposed, replaces a node
//! with a decomposition result, and performs algebraic resubstitution cleanup.
//! The actual SIS graph mutation still depends on native ports for `network_t`,
//! `node_t`, delay tracing, cutset/weight computation, and Boolean
//! simplification. This module ports the deterministic decision logic over
//! owned Rust records and reports network-bound entry points as explicit
//! missing-dependency errors.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

pub const NSP_EPSILON: f64 = 1.0e-6;
pub const CLP: i32 = 0;
pub const FAN: i32 = 1;
pub const DUAL: i32 = 2;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    UnitFanout,
    Library,
    Mapped,
    Tdc,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedParameters {
    pub crit_slack: f64,
    pub coeff: f64,
    pub model: DelayModel,
    pub new_mode: bool,
    pub interactive: bool,
    pub add_inv: bool,
    pub debug: bool,
    pub del_crit_cubes: bool,
    pub area_reclaim: bool,
    pub num_tries: usize,
}

impl Default for SpeedParameters {
    fn default() -> Self {
        Self {
            crit_slack: 0.0,
            coeff: 0.0,
            model: DelayModel::Unit,
            new_mode: false,
            interactive: false,
            add_inv: false,
            debug: false,
            del_crit_cubes: true,
            area_reclaim: false,
            num_tries: 1,
        }
    }
}

pub fn speed_critical(slack: DelayTime, params: &SpeedParameters) -> bool {
    let threshold = params.crit_slack - NSP_EPSILON;
    slack.rise < threshold || slack.fall < threshold
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
    Buffer,
    Inverter,
    Other,
}

pub fn speed_is_fanout_po(fanout_functions: &[NodeFunction]) -> bool {
    fanout_functions
        .iter()
        .any(|function| *function == NodeFunction::PrimaryOutput)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedUpNodeAction {
    SingleLevelUpdate,
    DecomposeAndReplace { delay_flag: bool },
}

pub fn speed_up_node_action(
    kind: NodeKind,
    literal_count: usize,
    fanin_count: usize,
    cube_count: usize,
    delay_flag: bool,
) -> SpeedUpNodeAction {
    if kind != NodeKind::Internal || literal_count == 0 {
        return SpeedUpNodeAction::SingleLevelUpdate;
    }

    if fanin_count <= 2 && cube_count <= 1 {
        return SpeedUpNodeAction::SingleLevelUpdate;
    }

    SpeedUpNodeAction::DecomposeAndReplace { delay_flag }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InitialDecompAction<N> {
    BypassMappedNetwork,
    AddInvertersToNetwork,
    NetworkCsweep,
    DelayTrace,
    SimplifyNode(N),
    SpeedUpNode { node: N, delay_flag: bool },
    AlgebraicResubstitution,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InitialDecompPlan<N> {
    pub actions: Vec<InitialDecompAction<N>>,
    pub temporary_parameters: SpeedParameters,
    pub restored_parameters: SpeedParameters,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecompCandidate<N> {
    pub id: N,
    pub kind: NodeKind,
    pub fanin_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplaceAction<N> {
    FreeOriginalDecompNode(N),
    ComputeRootArrival(N),
    AddNode(N),
    FreePrimaryInputStub(N),
    ReplaceOriginalWithRoot { original: N, root: N },
    SetOriginalArrival(N),
    TryAlgebraicResubstitute { node: N, excluded_nodes: Vec<N> },
    DeleteResubstitutedNode(N),
    DeleteSingleFaninNode(N),
    DeleteOriginalIfSingleFaninNonPo(N),
    SimplifyOriginal(N),
    UpdateOriginalArrival(N),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrimaryOutputCleanupAction<N> {
    PatchPrimaryOutputFanin {
        primary_output: N,
        removed_buffer: N,
        replacement: N,
    },
    CollapseSingleFaninNodeIntoFanout {
        node: N,
        fanout: N,
    },
    DeleteIfFanoutless(N),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CutsetTransformType {
    Clp,
    Fan,
    Dual,
    Unknown(i32),
}

impl CutsetTransformType {
    pub fn from_c_type(value: i32) -> Self {
        match value {
            CLP => Self::Clp,
            FAN => Self::Fan,
            DUAL => Self::Dual,
            other => Self::Unknown(other),
        }
    }

    pub fn fanin_based(self) -> bool {
        self == Self::Clp
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CutsetWeight {
    pub best_technique: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CutsetNode<N> {
    pub id: N,
    pub fanins: Vec<N>,
}

#[derive(Clone, Debug)]
pub struct CutsetOrderContext<N> {
    pub nodes: HashMap<N, CutsetNode<N>>,
    pub weights: Option<HashMap<N, CutsetWeight>>,
    pub transforms: Vec<CutsetTransformType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedUpError {
    UnknownNode(String),
    MissingWeight(String),
    MissingTransform(usize),
    MissingSisPorts { operation: &'static str },
}

impl fmt::Display for SpeedUpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown speedup node {node}"),
            Self::MissingWeight(node) => write!(f, "missing cutset weight for node {node}"),
            Self::MissingTransform(index) => write!(f, "missing local transform index {index}"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} is blocked by unported SIS dependencies")
            }
        }
    }
}

impl Error for SpeedUpError {}

pub trait SpeedUpBackend {
    type NodeId: Clone + Eq + Hash + Ord + ToString;

    fn set_speed_thresh(&mut self, params: &SpeedParameters) -> Result<(), SpeedUpError>;
    fn speed_compute_weight(
        &mut self,
        params: &SpeedParameters,
    ) -> Result<HashMap<Self::NodeId, CutsetWeight>, SpeedUpError>;
    fn cutset(
        &mut self,
        weights: &HashMap<Self::NodeId, CutsetWeight>,
    ) -> Result<Vec<Self::NodeId>, SpeedUpError>;
    fn cutset_order_context(
        &self,
        weights: Option<HashMap<Self::NodeId, CutsetWeight>>,
        params: &SpeedParameters,
    ) -> Result<CutsetOrderContext<Self::NodeId>, SpeedUpError>;
    fn speed_absorb(
        &mut self,
        node: &Self::NodeId,
        params: &SpeedParameters,
    ) -> Result<(), SpeedUpError>;
    fn primary_outputs(&self) -> Result<Vec<Self::NodeId>, SpeedUpError>;
    fn network_nodes(&self) -> Result<Vec<Self::NodeId>, SpeedUpError>;
    fn network_dfs(&self) -> Result<Vec<Self::NodeId>, SpeedUpError>;
    fn node_kind(&self, node: &Self::NodeId) -> Result<NodeKind, SpeedUpError>;
    fn node_function(&self, node: &Self::NodeId) -> Result<NodeFunction, SpeedUpError>;
    fn node_fanin_count(&self, node: &Self::NodeId) -> Result<usize, SpeedUpError>;
    fn node_cube_count(&self, node: &Self::NodeId) -> Result<usize, SpeedUpError>;
    fn node_literal_count(&self, node: &Self::NodeId) -> Result<usize, SpeedUpError>;
    fn node_fanin_at(
        &self,
        node: &Self::NodeId,
        index: usize,
    ) -> Result<Self::NodeId, SpeedUpError>;
    fn node_fanins(&self, node: &Self::NodeId) -> Result<Vec<Self::NodeId>, SpeedUpError>;
    fn node_fanouts(&self, node: &Self::NodeId) -> Result<Vec<Self::NodeId>, SpeedUpError>;
    fn node_fanout_count(&self, node: &Self::NodeId) -> Result<usize, SpeedUpError>;
    fn patch_fanin(
        &mut self,
        node: &Self::NodeId,
        old_fanin: &Self::NodeId,
        new_fanin: &Self::NodeId,
    ) -> Result<(), SpeedUpError>;
    fn node_collapse(
        &mut self,
        fanout: &Self::NodeId,
        source: &Self::NodeId,
    ) -> Result<(), SpeedUpError>;
    fn delete_node(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError>;
    fn delete_single_fanin_node(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError>;
    fn speed_single_level_update(
        &mut self,
        node: &Self::NodeId,
        params: &SpeedParameters,
    ) -> Result<(), SpeedUpError>;
    fn speed_decomp(
        &mut self,
        node: &Self::NodeId,
        params: &SpeedParameters,
        delay_flag: bool,
    ) -> Result<Vec<DecompCandidate<Self::NodeId>>, SpeedUpError>;
    fn free_decomp_node(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError>;
    fn compute_root_arrival(
        &mut self,
        root: &Self::NodeId,
        params: &SpeedParameters,
    ) -> Result<(), SpeedUpError>;
    fn network_add_node(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError>;
    fn free_primary_input_stub(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError>;
    fn replace_node_with_root(
        &mut self,
        original: &Self::NodeId,
        root: &Self::NodeId,
    ) -> Result<(), SpeedUpError>;
    fn set_original_arrival(&mut self, original: &Self::NodeId) -> Result<(), SpeedUpError>;
    fn simplify_replace(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError>;
    fn update_arrival_time(
        &mut self,
        node: &Self::NodeId,
        params: &SpeedParameters,
    ) -> Result<(), SpeedUpError>;
    fn try_algebraic_resubstitute(
        &mut self,
        node: &Self::NodeId,
        excluded_nodes: &[Self::NodeId],
    ) -> Result<bool, SpeedUpError>;
    fn node_substitute(
        &mut self,
        source: &Self::NodeId,
        target: &Self::NodeId,
        complement: bool,
    ) -> Result<bool, SpeedUpError>;
    fn add_inv_network(&mut self) -> Result<(), SpeedUpError>;
    fn network_csweep(&mut self) -> Result<(), SpeedUpError>;
    fn speed_delay_trace(&mut self, params: &SpeedParameters) -> Result<(), SpeedUpError>;
    fn speed_set_delay_data(
        &mut self,
        params: &SpeedParameters,
        library_acceleration: bool,
    ) -> Result<(), SpeedUpError>;

    fn fanout_functions(&self, node: &Self::NodeId) -> Result<Vec<NodeFunction>, SpeedUpError> {
        self.node_fanouts(node)?
            .iter()
            .map(|fanout| self.node_function(fanout))
            .collect()
    }
}

pub fn plan_initial_decomp<N: Clone>(
    internal_dfs_nodes: &[N],
    params: &SpeedParameters,
) -> InitialDecompPlan<N> {
    let mut temporary_parameters = params.clone();
    let restored_parameters = params.clone();

    if params.new_mode && params.model == DelayModel::Mapped {
        return InitialDecompPlan {
            actions: vec![InitialDecompAction::BypassMappedNetwork],
            temporary_parameters,
            restored_parameters,
        };
    }

    temporary_parameters.num_tries = 1;
    temporary_parameters.debug = false;
    temporary_parameters.del_crit_cubes = true;

    let mut actions = Vec::new();
    if params.add_inv {
        actions.push(InitialDecompAction::AddInvertersToNetwork);
    } else {
        actions.push(InitialDecompAction::NetworkCsweep);
    }
    actions.push(InitialDecompAction::DelayTrace);

    for node in internal_dfs_nodes {
        actions.push(InitialDecompAction::SimplifyNode(node.clone()));
        actions.push(InitialDecompAction::SpeedUpNode {
            node: node.clone(),
            delay_flag: true,
        });
    }

    if params.area_reclaim {
        actions.push(InitialDecompAction::AlgebraicResubstitution);
    }

    if params.add_inv {
        actions.push(InitialDecompAction::AddInvertersToNetwork);
    } else {
        actions.push(InitialDecompAction::NetworkCsweep);
    }

    InitialDecompPlan {
        actions,
        temporary_parameters,
        restored_parameters,
    }
}

pub fn plan_speed_replace<N>(
    original: N,
    decomposed_nodes: &[DecompCandidate<N>],
    params: &SpeedParameters,
    original_fanin_count_after_replace: usize,
    original_has_po_fanout: bool,
) -> Vec<ReplaceAction<N>>
where
    N: Clone + Eq,
{
    if decomposed_nodes.len() <= 3 {
        return vec![
            ReplaceAction::SimplifyOriginal(original.clone()),
            ReplaceAction::UpdateOriginalArrival(original),
        ];
    }

    let mut actions = Vec::new();
    actions.push(ReplaceAction::FreeOriginalDecompNode(
        decomposed_nodes[0].id.clone(),
    ));

    let root = decomposed_nodes[1].id.clone();
    actions.push(ReplaceAction::ComputeRootArrival(root.clone()));

    let mut remaining: Vec<N> = decomposed_nodes[2..]
        .iter()
        .map(|node| node.id.clone())
        .collect();
    let mut single_fanin_nodes = Vec::new();
    let mut area_reclaim_nodes = Vec::new();

    for node in decomposed_nodes[2..].iter().rev() {
        if node.kind != NodeKind::PrimaryInput {
            remaining.retain(|candidate| candidate != &node.id);
            actions.push(ReplaceAction::AddNode(node.id.clone()));
            if node_is_single_fanin_like(node) {
                single_fanin_nodes.push(node.id.clone());
            } else if params.area_reclaim {
                area_reclaim_nodes.push((node.id.clone(), remaining.clone()));
            }
        } else {
            actions.push(ReplaceAction::FreePrimaryInputStub(node.id.clone()));
        }
    }

    actions.push(ReplaceAction::ReplaceOriginalWithRoot {
        original: original.clone(),
        root,
    });
    actions.push(ReplaceAction::SetOriginalArrival(original.clone()));

    for (node, excluded_nodes) in area_reclaim_nodes {
        actions.push(ReplaceAction::TryAlgebraicResubstitute {
            node: node.clone(),
            excluded_nodes,
        });
        actions.push(ReplaceAction::DeleteResubstitutedNode(node));
    }

    if !params.add_inv {
        for node in single_fanin_nodes {
            actions.push(ReplaceAction::DeleteSingleFaninNode(node));
        }
    }

    if original_fanin_count_after_replace == 1 && !original_has_po_fanout {
        actions.push(ReplaceAction::DeleteOriginalIfSingleFaninNonPo(original));
    }

    actions
}

fn node_is_single_fanin_like<N>(node: &DecompCandidate<N>) -> bool {
    node.fanin_count < 2
}

pub fn plan_primary_output_cleanup<N: Clone>(
    primary_output: N,
    fanin: N,
    fanin_function: NodeFunction,
    fanin_input: Option<N>,
    fanin_fanouts: &[N],
) -> Vec<PrimaryOutputCleanupAction<N>> {
    let mut actions = Vec::new();

    if fanin_function == NodeFunction::Buffer {
        if let Some(replacement) = fanin_input {
            actions.push(PrimaryOutputCleanupAction::PatchPrimaryOutputFanin {
                primary_output,
                removed_buffer: fanin.clone(),
                replacement,
            });
        }
    } else if fanin_function != NodeFunction::Inverter {
        return actions;
    }

    for fanout in fanin_fanouts {
        actions.push(
            PrimaryOutputCleanupAction::CollapseSingleFaninNodeIntoFanout {
                node: fanin.clone(),
                fanout: fanout.clone(),
            },
        );
    }
    actions.push(PrimaryOutputCleanupAction::DeleteIfFanoutless(fanin));
    actions
}

pub fn generate_revised_order<N>(
    cutset: &[N],
    context: &CutsetOrderContext<N>,
) -> Result<Vec<N>, SpeedUpError>
where
    N: Clone + Eq + Hash + ToString,
{
    let mut ordered = cutset.to_vec();
    for node in &ordered {
        if !context.nodes.contains_key(node) {
            return Err(SpeedUpError::UnknownNode(node.to_string()));
        }
    }

    let mut comparison_error = None;
    ordered.sort_by(
        |left, right| match compare_cutset_nodes(left, right, context) {
            Ok(ordering) => ordering,
            Err(error) => {
                comparison_error = Some(error);
                Ordering::Equal
            }
        },
    );

    if let Some(error) = comparison_error {
        Err(error)
    } else {
        Ok(ordered)
    }
}

pub fn compare_cutset_nodes<N>(
    node1: &N,
    node2: &N,
    context: &CutsetOrderContext<N>,
) -> Result<Ordering, SpeedUpError>
where
    N: Clone + Eq + Hash + ToString,
{
    let technique = cutset_technique_comparison(node1, node2, context)?;
    if let CutsetTechniqueComparison::Direct(ordering) = technique {
        return Ok(ordering);
    }
    let fanin_based_technique = technique == CutsetTechniqueComparison::FaninBased;

    if reaches_fanin(node1, node2, &context.nodes)? {
        return Ok(if fanin_based_technique {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    if reaches_fanin(node2, node1, &context.nodes)? {
        return Ok(if fanin_based_technique {
            Ordering::Greater
        } else {
            Ordering::Less
        });
    }

    Ok(Ordering::Equal)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CutsetTechniqueComparison {
    FaninBased,
    FanoutBased,
    Direct(Ordering),
}

fn cutset_technique_comparison<N>(
    node1: &N,
    node2: &N,
    context: &CutsetOrderContext<N>,
) -> Result<CutsetTechniqueComparison, SpeedUpError>
where
    N: Eq + Hash + ToString,
{
    let Some(weights) = &context.weights else {
        return Ok(CutsetTechniqueComparison::FaninBased);
    };

    let w1 = weights
        .get(node1)
        .ok_or_else(|| SpeedUpError::MissingWeight(node1.to_string()))?;
    let w2 = weights
        .get(node2)
        .ok_or_else(|| SpeedUpError::MissingWeight(node2.to_string()))?;
    let t1 = *context
        .transforms
        .get(w1.best_technique)
        .ok_or(SpeedUpError::MissingTransform(w1.best_technique))?;
    let t2 = *context
        .transforms
        .get(w2.best_technique)
        .ok_or(SpeedUpError::MissingTransform(w2.best_technique))?;

    if !t1.fanin_based() && !t2.fanin_based() {
        Ok(CutsetTechniqueComparison::FanoutBased)
    } else if t1.fanin_based() && t2.fanin_based() {
        Ok(CutsetTechniqueComparison::FaninBased)
    } else if !t1.fanin_based() {
        Ok(CutsetTechniqueComparison::Direct(Ordering::Less))
    } else {
        Ok(CutsetTechniqueComparison::Direct(Ordering::Greater))
    }
}

fn reaches_fanin<N>(
    source: &N,
    target: &N,
    nodes: &HashMap<N, CutsetNode<N>>,
) -> Result<bool, SpeedUpError>
where
    N: Clone + Eq + Hash + ToString,
{
    let mut stack = nodes
        .get(source)
        .ok_or_else(|| SpeedUpError::UnknownNode(source.to_string()))?
        .fanins
        .clone();
    let mut seen = HashSet::new();

    while let Some(node) = stack.pop() {
        if &node == target {
            return Ok(true);
        }
        if seen.insert(node.clone()) {
            let fanin_node = nodes
                .get(&node)
                .ok_or_else(|| SpeedUpError::UnknownNode(node.to_string()))?;
            stack.extend(fanin_node.fanins.iter().cloned());
        }
    }

    Ok(false)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResubNode<N> {
    pub id: N,
    pub kind: NodeKind,
    pub fanins: Vec<N>,
    pub fanouts: Vec<N>,
    pub literal_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResubstitutionAction<N> {
    TrySubstitute { source: N, target: N },
    CollapseSourceIntoFanout { source: N, fanout: N },
    DeleteSourceIfFanoutless(N),
}

pub fn algebraic_resubstitution_targets<N>(
    source: &N,
    nodes: &HashMap<N, ResubNode<N>>,
    excluded: Option<&HashSet<N>>,
) -> Result<Vec<N>, SpeedUpError>
where
    N: Clone + Eq + Hash + Ord + ToString,
{
    let source_node = nodes
        .get(source)
        .ok_or_else(|| SpeedUpError::UnknownNode(source.to_string()))?;
    if source_node.kind != NodeKind::Internal
        || source_node.literal_count < 1
        || source_node.fanouts.len() > 1
    {
        return Ok(Vec::new());
    }

    let mut targets = HashSet::new();
    for fanin in &source_node.fanins {
        let fanin_node = nodes
            .get(fanin)
            .ok_or_else(|| SpeedUpError::UnknownNode(fanin.to_string()))?;
        for fanout in &fanin_node.fanouts {
            let fanout_node = nodes
                .get(fanout)
                .ok_or_else(|| SpeedUpError::UnknownNode(fanout.to_string()))?;
            if fanout_node.fanins.len() <= 2
                && excluded.is_none_or(|excluded| !excluded.contains(fanout))
            {
                targets.insert(fanout.clone());
            }
        }
    }

    let mut targets = targets.into_iter().collect::<Vec<_>>();
    targets.sort();
    Ok(targets)
}

pub fn plan_resubstitution_attempts<N>(
    source: N,
    nodes: &HashMap<N, ResubNode<N>>,
    excluded: Option<&HashSet<N>>,
) -> Result<Vec<ResubstitutionAction<N>>, SpeedUpError>
where
    N: Clone + Eq + Hash + Ord + ToString,
{
    let source_node = nodes
        .get(&source)
        .ok_or_else(|| SpeedUpError::UnknownNode(source.to_string()))?;
    let mut actions = Vec::new();

    for target in algebraic_resubstitution_targets(&source, nodes, excluded)? {
        let target_node = nodes
            .get(&target)
            .ok_or_else(|| SpeedUpError::UnknownNode(target.to_string()))?;
        if target_node.fanouts.len() < 2 {
            actions.push(ResubstitutionAction::TrySubstitute {
                source: source.clone(),
                target,
            });
        }
    }

    if source_node.fanins.len() <= 1 {
        for fanout in &source_node.fanouts {
            actions.push(ResubstitutionAction::CollapseSourceIntoFanout {
                source: source.clone(),
                fanout: fanout.clone(),
            });
        }
        actions.push(ResubstitutionAction::DeleteSourceIfFanoutless(source));
    }

    Ok(actions)
}

pub fn speed_up_network_bound<Network>(
    network: &mut Network,
    params: &SpeedParameters,
) -> Result<(), SpeedUpError>
where
    Network: SpeedUpBackend,
{
    network.set_speed_thresh(params)?;
    let weights = network.speed_compute_weight(params)?;
    let mincut = network.cutset(&weights)?;
    let context = network.cutset_order_context(Some(weights), params)?;
    let mincut = generate_revised_order(&mincut, &context)?;

    for node in &mincut {
        network.speed_absorb(node, params)?;
    }
    for node in &mincut {
        if network.node_kind(node)? == NodeKind::Internal {
            speed_up_node_native(network, node.clone(), params, false)?;
        }
    }

    cleanup_primary_outputs(network)
}

pub fn speed_node_interface_bound<Network>(
    network: &mut Network,
    node: Network::NodeId,
    coeff: f64,
    model: DelayModel,
) -> Result<(), SpeedUpError>
where
    Network: SpeedUpBackend,
{
    let params = SpeedParameters {
        coeff,
        model,
        ..SpeedParameters::default()
    };
    network.speed_set_delay_data(&params, false)?;
    speed_up_node_native(network, node, &params, false)
}

pub fn speed_init_decomp_bound<Network>(
    network: &mut Network,
    params: &SpeedParameters,
) -> Result<(), SpeedUpError>
where
    Network: SpeedUpBackend,
{
    if params.new_mode && params.model == DelayModel::Mapped {
        return Ok(());
    }

    if params.add_inv {
        network.add_inv_network()?;
    } else {
        network.network_csweep()?;
    }

    network.speed_delay_trace(params)?;
    let mut temporary_params = params.clone();
    temporary_params.num_tries = 1;
    temporary_params.debug = false;
    temporary_params.del_crit_cubes = true;

    for node in network.network_dfs()? {
        if network.node_kind(&node)? == NodeKind::Internal {
            network.simplify_replace(&node)?;
            speed_up_node_native(network, node, &temporary_params, true)?;
        }
    }

    if params.area_reclaim {
        speed_resub_alge_network_bound(network)?;
    }

    if params.add_inv {
        network.add_inv_network()
    } else {
        network.network_csweep()
    }
}

pub fn speed_resub_alge_network_bound<Network>(network: &mut Network) -> Result<(), SpeedUpError>
where
    Network: SpeedUpBackend,
{
    let mut not_done = true;
    while not_done {
        not_done = false;
        for node in network.network_nodes()? {
            if speed_resub_alge_node_native(network, &node, None)? {
                network.delete_node(&node)?;
                not_done = true;
                break;
            }
        }
    }

    Ok(())
}

fn cleanup_primary_outputs<Network>(network: &mut Network) -> Result<(), SpeedUpError>
where
    Network: SpeedUpBackend,
{
    for primary_output in network.primary_outputs()? {
        let fanin = network.node_fanin_at(&primary_output, 0)?;
        match network.node_function(&fanin)? {
            NodeFunction::Buffer => {
                let fanin_input = network.node_fanin_at(&fanin, 0)?;
                network.patch_fanin(&primary_output, &fanin, &fanin_input)?;
            }
            NodeFunction::Inverter => {}
            _ => continue,
        }

        for fanout in network.node_fanouts(&fanin)? {
            network.node_collapse(&fanout, &fanin)?;
        }
        if network.node_fanout_count(&fanin)? == 0 {
            network.delete_node(&fanin)?;
        }
    }

    Ok(())
}

fn speed_up_node_native<Network>(
    network: &mut Network,
    node: Network::NodeId,
    params: &SpeedParameters,
    delay_flag: bool,
) -> Result<(), SpeedUpError>
where
    Network: SpeedUpBackend,
{
    match speed_up_node_action(
        network.node_kind(&node)?,
        network.node_literal_count(&node)?,
        network.node_fanin_count(&node)?,
        network.node_cube_count(&node)?,
        delay_flag,
    ) {
        SpeedUpNodeAction::SingleLevelUpdate => network.speed_single_level_update(&node, params),
        SpeedUpNodeAction::DecomposeAndReplace { delay_flag } => {
            let nodes = network.speed_decomp(&node, params, delay_flag)?;
            speed_replace_native(network, node, &nodes, params)
        }
    }
}

fn speed_replace_native<Network>(
    network: &mut Network,
    original: Network::NodeId,
    decomposed_nodes: &[DecompCandidate<Network::NodeId>],
    params: &SpeedParameters,
) -> Result<(), SpeedUpError>
where
    Network: SpeedUpBackend,
{
    if decomposed_nodes.len() <= 3 {
        network.simplify_replace(&original)?;
        return network.update_arrival_time(&original, params);
    }

    let original_fanin_count_after_replace = network.node_fanin_count(&original)?;
    let original_has_po_fanout = speed_is_fanout_po(&network.fanout_functions(&original)?);
    let actions = plan_speed_replace(
        original,
        decomposed_nodes,
        params,
        original_fanin_count_after_replace,
        original_has_po_fanout,
    );
    let mut resubstituted = HashSet::new();

    for action in actions {
        match action {
            ReplaceAction::FreeOriginalDecompNode(node) => network.free_decomp_node(&node)?,
            ReplaceAction::ComputeRootArrival(node) => {
                network.compute_root_arrival(&node, params)?
            }
            ReplaceAction::AddNode(node) => network.network_add_node(&node)?,
            ReplaceAction::FreePrimaryInputStub(node) => network.free_primary_input_stub(&node)?,
            ReplaceAction::ReplaceOriginalWithRoot { original, root } => {
                network.replace_node_with_root(&original, &root)?;
            }
            ReplaceAction::SetOriginalArrival(node) => network.set_original_arrival(&node)?,
            ReplaceAction::TryAlgebraicResubstitute {
                node,
                excluded_nodes,
            } => {
                if network.try_algebraic_resubstitute(&node, &excluded_nodes)? {
                    resubstituted.insert(node);
                }
            }
            ReplaceAction::DeleteResubstitutedNode(node) => {
                if resubstituted.remove(&node) {
                    network.delete_node(&node)?;
                }
            }
            ReplaceAction::DeleteSingleFaninNode(node)
            | ReplaceAction::DeleteOriginalIfSingleFaninNonPo(node) => {
                network.delete_single_fanin_node(&node)?;
            }
            ReplaceAction::SimplifyOriginal(node) => network.simplify_replace(&node)?,
            ReplaceAction::UpdateOriginalArrival(node) => {
                network.update_arrival_time(&node, params)?;
            }
        }
    }

    Ok(())
}

fn speed_resub_alge_node_native<Network>(
    network: &mut Network,
    source: &Network::NodeId,
    excluded: Option<&HashSet<Network::NodeId>>,
) -> Result<bool, SpeedUpError>
where
    Network: SpeedUpBackend,
{
    if matches!(
        network.node_kind(source)?,
        NodeKind::PrimaryInput | NodeKind::PrimaryOutput
    ) || network.node_literal_count(source)? < 1
        || network.node_fanout_count(source)? > 1
    {
        return Ok(false);
    }

    let mut targets = HashSet::new();
    for fanin in network.node_fanins(source)? {
        for fanout in network.node_fanouts(&fanin)? {
            if network.node_fanin_count(&fanout)? <= 2
                && excluded.is_none_or(|excluded| !excluded.contains(&fanout))
            {
                targets.insert(fanout);
            }
        }
    }

    let mut targets = targets.into_iter().collect::<Vec<_>>();
    targets.sort();

    for target in targets {
        if network.node_fanout_count(&target)? < 2
            && network.node_substitute(source, &target, false)?
            && network.node_fanin_count(source)? <= 1
        {
            for fanout in network.node_fanouts(source)? {
                network.node_collapse(&fanout, source)?;
            }
            if network.node_fanout_count(source)? == 0 {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cut_node(id: &'static str, fanins: &[&'static str]) -> CutsetNode<&'static str> {
        CutsetNode {
            id,
            fanins: fanins.to_vec(),
        }
    }

    fn resub(
        id: &'static str,
        kind: NodeKind,
        fanins: &[&'static str],
        fanouts: &[&'static str],
        literal_count: usize,
    ) -> ResubNode<&'static str> {
        ResubNode {
            id,
            kind,
            fanins: fanins.to_vec(),
            fanouts: fanouts.to_vec(),
            literal_count,
        }
    }

    #[test]
    fn criticality_uses_strict_slack_threshold_with_epsilon() {
        let params = SpeedParameters {
            crit_slack: 1.0,
            ..SpeedParameters::default()
        };

        assert!(speed_critical(DelayTime::new(0.9, 2.0), &params));
        assert!(!speed_critical(
            DelayTime::new(1.0 - NSP_EPSILON / 2.0, 2.0),
            &params
        ));
        assert!(speed_critical(
            DelayTime::new(2.0, 1.0 - NSP_EPSILON * 2.0),
            &params
        ));
    }

    #[test]
    fn node_decomposition_gate_matches_c_fast_paths() {
        assert_eq!(
            speed_up_node_action(NodeKind::PrimaryInput, 5, 4, 4, false),
            SpeedUpNodeAction::SingleLevelUpdate
        );
        assert_eq!(
            speed_up_node_action(NodeKind::Internal, 0, 4, 4, false),
            SpeedUpNodeAction::SingleLevelUpdate
        );
        assert_eq!(
            speed_up_node_action(NodeKind::Internal, 1, 2, 1, false),
            SpeedUpNodeAction::SingleLevelUpdate
        );
        assert_eq!(
            speed_up_node_action(NodeKind::Internal, 3, 3, 2, true),
            SpeedUpNodeAction::DecomposeAndReplace { delay_flag: true }
        );
    }

    #[test]
    fn initial_decomp_plan_bypasses_mapped_new_mode_or_sets_temporary_flags() {
        let mapped = SpeedParameters {
            new_mode: true,
            model: DelayModel::Mapped,
            interactive: true,
            ..SpeedParameters::default()
        };
        let bypass = plan_initial_decomp(&["n"], &mapped);
        assert_eq!(
            bypass.actions,
            vec![InitialDecompAction::BypassMappedNetwork]
        );

        let params = SpeedParameters {
            add_inv: false,
            debug: true,
            del_crit_cubes: false,
            area_reclaim: true,
            num_tries: 7,
            ..SpeedParameters::default()
        };
        let plan = plan_initial_decomp(&["n1", "n2"], &params);
        assert_eq!(plan.temporary_parameters.num_tries, 1);
        assert!(!plan.temporary_parameters.debug);
        assert!(plan.temporary_parameters.del_crit_cubes);
        assert_eq!(plan.restored_parameters, params);
        assert_eq!(
            plan.actions,
            vec![
                InitialDecompAction::NetworkCsweep,
                InitialDecompAction::DelayTrace,
                InitialDecompAction::SimplifyNode("n1"),
                InitialDecompAction::SpeedUpNode {
                    node: "n1",
                    delay_flag: true,
                },
                InitialDecompAction::SimplifyNode("n2"),
                InitialDecompAction::SpeedUpNode {
                    node: "n2",
                    delay_flag: true,
                },
                InitialDecompAction::AlgebraicResubstitution,
                InitialDecompAction::NetworkCsweep,
            ]
        );
    }

    #[test]
    fn replace_plan_models_reverse_add_resub_and_single_fanin_cleanup() {
        let params = SpeedParameters {
            area_reclaim: true,
            add_inv: false,
            ..SpeedParameters::default()
        };
        let nodes = [
            DecompCandidate {
                id: "old-copy",
                kind: NodeKind::Internal,
                fanin_count: 3,
            },
            DecompCandidate {
                id: "root",
                kind: NodeKind::Internal,
                fanin_count: 2,
            },
            DecompCandidate {
                id: "pi-stub",
                kind: NodeKind::PrimaryInput,
                fanin_count: 0,
            },
            DecompCandidate {
                id: "gate-a",
                kind: NodeKind::Internal,
                fanin_count: 2,
            },
            DecompCandidate {
                id: "gate-b",
                kind: NodeKind::Internal,
                fanin_count: 1,
            },
        ];

        assert_eq!(
            plan_speed_replace("orig", &nodes, &params, 1, false),
            vec![
                ReplaceAction::FreeOriginalDecompNode("old-copy"),
                ReplaceAction::ComputeRootArrival("root"),
                ReplaceAction::AddNode("gate-b"),
                ReplaceAction::AddNode("gate-a"),
                ReplaceAction::FreePrimaryInputStub("pi-stub"),
                ReplaceAction::ReplaceOriginalWithRoot {
                    original: "orig",
                    root: "root",
                },
                ReplaceAction::SetOriginalArrival("orig"),
                ReplaceAction::TryAlgebraicResubstitute {
                    node: "gate-a",
                    excluded_nodes: vec!["pi-stub"],
                },
                ReplaceAction::DeleteResubstitutedNode("gate-a"),
                ReplaceAction::DeleteSingleFaninNode("gate-b"),
                ReplaceAction::DeleteOriginalIfSingleFaninNonPo("orig"),
            ]
        );
    }

    #[test]
    fn trivial_replacement_simplifies_original_and_updates_arrival() {
        let params = SpeedParameters::default();
        let nodes = [
            DecompCandidate {
                id: "old",
                kind: NodeKind::Internal,
                fanin_count: 1,
            },
            DecompCandidate {
                id: "root",
                kind: NodeKind::Internal,
                fanin_count: 1,
            },
        ];

        assert_eq!(
            plan_speed_replace("orig", &nodes, &params, 2, false),
            vec![
                ReplaceAction::SimplifyOriginal("orig"),
                ReplaceAction::UpdateOriginalArrival("orig"),
            ]
        );
    }

    #[test]
    fn primary_output_cleanup_patches_buffers_and_collapses_inverters() {
        assert_eq!(
            plan_primary_output_cleanup(
                "po",
                "buf",
                NodeFunction::Buffer,
                Some("fanin"),
                &["po", "other"],
            ),
            vec![
                PrimaryOutputCleanupAction::PatchPrimaryOutputFanin {
                    primary_output: "po",
                    removed_buffer: "buf",
                    replacement: "fanin",
                },
                PrimaryOutputCleanupAction::CollapseSingleFaninNodeIntoFanout {
                    node: "buf",
                    fanout: "po",
                },
                PrimaryOutputCleanupAction::CollapseSingleFaninNodeIntoFanout {
                    node: "buf",
                    fanout: "other",
                },
                PrimaryOutputCleanupAction::DeleteIfFanoutless("buf"),
            ]
        );

        assert_eq!(
            plan_primary_output_cleanup("po", "inv", NodeFunction::Inverter, None, &["po"]),
            vec![
                PrimaryOutputCleanupAction::CollapseSingleFaninNodeIntoFanout {
                    node: "inv",
                    fanout: "po",
                },
                PrimaryOutputCleanupAction::DeleteIfFanoutless("inv"),
            ]
        );
    }

    #[test]
    fn revised_order_places_outputs_before_fanins_for_fanin_based_methods() {
        let context = CutsetOrderContext {
            nodes: HashMap::from([
                ("a", cut_node("a", &[])),
                ("b", cut_node("b", &["a"])),
                ("c", cut_node("c", &["b"])),
            ]),
            weights: None,
            transforms: Vec::new(),
        };

        assert_eq!(
            generate_revised_order(&["a", "b", "c"], &context).unwrap(),
            vec!["c", "b", "a"]
        );
    }

    #[test]
    fn revised_order_reverses_tfi_precedence_for_fanout_based_methods() {
        let context = CutsetOrderContext {
            nodes: HashMap::from([
                ("a", cut_node("a", &[])),
                ("b", cut_node("b", &["a"])),
                ("c", cut_node("c", &["b"])),
            ]),
            weights: Some(HashMap::from([
                ("a", CutsetWeight { best_technique: 0 }),
                ("b", CutsetWeight { best_technique: 0 }),
                ("c", CutsetWeight { best_technique: 0 }),
            ])),
            transforms: vec![CutsetTransformType::Fan],
        };

        assert_eq!(
            generate_revised_order(&["c", "b", "a"], &context).unwrap(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn mixed_cutset_techniques_place_fanout_optimizations_first() {
        let context = CutsetOrderContext {
            nodes: HashMap::from([
                ("fanin", cut_node("fanin", &[])),
                ("fanout", cut_node("fanout", &[])),
            ]),
            weights: Some(HashMap::from([
                ("fanin", CutsetWeight { best_technique: 0 }),
                ("fanout", CutsetWeight { best_technique: 1 }),
            ])),
            transforms: vec![CutsetTransformType::Clp, CutsetTransformType::Fan],
        };

        assert_eq!(
            generate_revised_order(&["fanin", "fanout"], &context).unwrap(),
            vec!["fanout", "fanin"]
        );
    }

    #[test]
    fn algebraic_resubstitution_targets_are_unique_sorted_and_filtered() {
        let nodes = HashMap::from([
            (
                "f",
                resub("f", NodeKind::Internal, &["a", "b"], &["only"], 2),
            ),
            ("a", resub("a", NodeKind::Internal, &[], &["x", "y"], 1)),
            ("b", resub("b", NodeKind::Internal, &[], &["y", "z"], 1)),
            ("x", resub("x", NodeKind::Internal, &["a"], &[], 1)),
            ("y", resub("y", NodeKind::Internal, &["a", "b"], &[], 1)),
            (
                "z",
                resub("z", NodeKind::Internal, &["b", "c", "d"], &[], 1),
            ),
            ("only", resub("only", NodeKind::Internal, &[], &[], 1)),
        ]);
        let excluded = HashSet::from(["x"]);

        assert_eq!(
            algebraic_resubstitution_targets(&"f", &nodes, Some(&excluded)).unwrap(),
            vec!["y"]
        );
    }

    #[test]
    fn resubstitution_plan_preserves_c_candidate_and_cleanup_rules() {
        let nodes = HashMap::from([
            ("f", resub("f", NodeKind::Internal, &["a"], &["fo"], 1)),
            ("a", resub("a", NodeKind::Internal, &[], &["target"], 1)),
            (
                "target",
                resub("target", NodeKind::Internal, &["a"], &["last"], 1),
            ),
            ("fo", resub("fo", NodeKind::Internal, &["f"], &[], 1)),
        ]);

        assert_eq!(
            plan_resubstitution_attempts("f", &nodes, None).unwrap(),
            vec![
                ResubstitutionAction::TrySubstitute {
                    source: "f",
                    target: "target",
                },
                ResubstitutionAction::CollapseSourceIntoFanout {
                    source: "f",
                    fanout: "fo",
                },
                ResubstitutionAction::DeleteSourceIfFanoutless("f"),
            ]
        );
    }

    #[derive(Default)]
    struct RecordingBackend {
        log: Vec<String>,
        po_patched: bool,
        f_collapsed: bool,
        f_deleted: bool,
    }

    impl SpeedUpBackend for RecordingBackend {
        type NodeId = &'static str;

        fn set_speed_thresh(&mut self, _params: &SpeedParameters) -> Result<(), SpeedUpError> {
            self.log.push("set_thresh".to_string());
            Ok(())
        }

        fn speed_compute_weight(
            &mut self,
            _params: &SpeedParameters,
        ) -> Result<HashMap<Self::NodeId, CutsetWeight>, SpeedUpError> {
            self.log.push("compute_weight".to_string());
            Ok(HashMap::from([
                ("n1", CutsetWeight { best_technique: 0 }),
                ("n2", CutsetWeight { best_technique: 0 }),
            ]))
        }

        fn cutset(
            &mut self,
            _weights: &HashMap<Self::NodeId, CutsetWeight>,
        ) -> Result<Vec<Self::NodeId>, SpeedUpError> {
            self.log.push("cutset".to_string());
            Ok(vec!["n1", "n2"])
        }

        fn cutset_order_context(
            &self,
            weights: Option<HashMap<Self::NodeId, CutsetWeight>>,
            _params: &SpeedParameters,
        ) -> Result<CutsetOrderContext<Self::NodeId>, SpeedUpError> {
            Ok(CutsetOrderContext {
                nodes: HashMap::from([
                    ("a", cut_node("a", &[])),
                    ("n1", cut_node("n1", &["a"])),
                    ("n2", cut_node("n2", &["n1"])),
                    ("po", cut_node("po", &["n2"])),
                ]),
                weights,
                transforms: vec![CutsetTransformType::Clp],
            })
        }

        fn speed_absorb(
            &mut self,
            node: &Self::NodeId,
            _params: &SpeedParameters,
        ) -> Result<(), SpeedUpError> {
            self.log.push(format!("absorb:{node}"));
            Ok(())
        }

        fn primary_outputs(&self) -> Result<Vec<Self::NodeId>, SpeedUpError> {
            Ok(vec!["po"])
        }

        fn network_nodes(&self) -> Result<Vec<Self::NodeId>, SpeedUpError> {
            Ok(if self.f_deleted {
                vec!["a", "fo", "target"]
            } else {
                vec!["a", "f", "fo", "target"]
            })
        }

        fn network_dfs(&self) -> Result<Vec<Self::NodeId>, SpeedUpError> {
            Ok(vec!["n"])
        }

        fn node_kind(&self, node: &Self::NodeId) -> Result<NodeKind, SpeedUpError> {
            Ok(match *node {
                "a" => NodeKind::PrimaryInput,
                "po" => NodeKind::PrimaryOutput,
                _ => NodeKind::Internal,
            })
        }

        fn node_function(&self, node: &Self::NodeId) -> Result<NodeFunction, SpeedUpError> {
            Ok(match *node {
                "po" => NodeFunction::PrimaryOutput,
                "n2" => NodeFunction::Buffer,
                _ => NodeFunction::Other,
            })
        }

        fn node_fanin_count(&self, node: &Self::NodeId) -> Result<usize, SpeedUpError> {
            Ok(self.node_fanins(node)?.len())
        }

        fn node_cube_count(&self, _node: &Self::NodeId) -> Result<usize, SpeedUpError> {
            Ok(0)
        }

        fn node_literal_count(&self, node: &Self::NodeId) -> Result<usize, SpeedUpError> {
            Ok(usize::from(*node == "f"))
        }

        fn node_fanin_at(
            &self,
            node: &Self::NodeId,
            index: usize,
        ) -> Result<Self::NodeId, SpeedUpError> {
            self.node_fanins(node)?
                .get(index)
                .copied()
                .ok_or_else(|| SpeedUpError::UnknownNode(format!("{node}[{index}]")))
        }

        fn node_fanins(&self, node: &Self::NodeId) -> Result<Vec<Self::NodeId>, SpeedUpError> {
            Ok(match *node {
                "po" if self.po_patched => vec!["n1"],
                "po" => vec!["n2"],
                "n2" => vec!["n1"],
                "n1" | "f" | "target" => vec!["a"],
                "fo" => vec!["f"],
                _ => Vec::new(),
            })
        }

        fn node_fanouts(&self, node: &Self::NodeId) -> Result<Vec<Self::NodeId>, SpeedUpError> {
            Ok(match *node {
                "a" => vec!["n1", "target"],
                "n1" => vec!["n2"],
                "n2" if self.po_patched => Vec::new(),
                "n2" => vec!["po"],
                "f" if self.f_collapsed => Vec::new(),
                "f" => vec!["fo"],
                _ => Vec::new(),
            })
        }

        fn node_fanout_count(&self, node: &Self::NodeId) -> Result<usize, SpeedUpError> {
            Ok(self.node_fanouts(node)?.len())
        }

        fn patch_fanin(
            &mut self,
            node: &Self::NodeId,
            old_fanin: &Self::NodeId,
            new_fanin: &Self::NodeId,
        ) -> Result<(), SpeedUpError> {
            self.log
                .push(format!("patch:{node}:{old_fanin}:{new_fanin}"));
            self.po_patched = true;
            Ok(())
        }

        fn node_collapse(
            &mut self,
            fanout: &Self::NodeId,
            source: &Self::NodeId,
        ) -> Result<(), SpeedUpError> {
            self.log.push(format!("collapse:{fanout}:{source}"));
            if *source == "f" {
                self.f_collapsed = true;
            }
            Ok(())
        }

        fn delete_node(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError> {
            self.log.push(format!("delete:{node}"));
            if *node == "f" {
                self.f_deleted = true;
            }
            Ok(())
        }

        fn delete_single_fanin_node(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError> {
            self.log.push(format!("delete_single:{node}"));
            Ok(())
        }

        fn speed_single_level_update(
            &mut self,
            node: &Self::NodeId,
            _params: &SpeedParameters,
        ) -> Result<(), SpeedUpError> {
            self.log.push(format!("single:{node}"));
            Ok(())
        }

        fn speed_decomp(
            &mut self,
            node: &Self::NodeId,
            _params: &SpeedParameters,
            delay_flag: bool,
        ) -> Result<Vec<DecompCandidate<Self::NodeId>>, SpeedUpError> {
            self.log.push(format!("decomp:{node}:{delay_flag}"));
            Ok(Vec::new())
        }

        fn free_decomp_node(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError> {
            self.log.push(format!("free:{node}"));
            Ok(())
        }

        fn compute_root_arrival(
            &mut self,
            root: &Self::NodeId,
            _params: &SpeedParameters,
        ) -> Result<(), SpeedUpError> {
            self.log.push(format!("root_arrival:{root}"));
            Ok(())
        }

        fn network_add_node(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError> {
            self.log.push(format!("add:{node}"));
            Ok(())
        }

        fn free_primary_input_stub(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError> {
            self.log.push(format!("free_pi:{node}"));
            Ok(())
        }

        fn replace_node_with_root(
            &mut self,
            original: &Self::NodeId,
            root: &Self::NodeId,
        ) -> Result<(), SpeedUpError> {
            self.log.push(format!("replace:{original}:{root}"));
            Ok(())
        }

        fn set_original_arrival(&mut self, original: &Self::NodeId) -> Result<(), SpeedUpError> {
            self.log.push(format!("set_arrival:{original}"));
            Ok(())
        }

        fn simplify_replace(&mut self, node: &Self::NodeId) -> Result<(), SpeedUpError> {
            self.log.push(format!("simplify:{node}"));
            Ok(())
        }

        fn update_arrival_time(
            &mut self,
            node: &Self::NodeId,
            _params: &SpeedParameters,
        ) -> Result<(), SpeedUpError> {
            self.log.push(format!("update_arrival:{node}"));
            Ok(())
        }

        fn try_algebraic_resubstitute(
            &mut self,
            node: &Self::NodeId,
            excluded_nodes: &[Self::NodeId],
        ) -> Result<bool, SpeedUpError> {
            self.log
                .push(format!("try_resub:{node}:{}", excluded_nodes.len()));
            Ok(false)
        }

        fn node_substitute(
            &mut self,
            source: &Self::NodeId,
            target: &Self::NodeId,
            complement: bool,
        ) -> Result<bool, SpeedUpError> {
            self.log
                .push(format!("substitute:{source}:{target}:{complement}"));
            Ok(*source == "f" && *target == "target")
        }

        fn add_inv_network(&mut self) -> Result<(), SpeedUpError> {
            self.log.push("add_inv".to_string());
            Ok(())
        }

        fn network_csweep(&mut self) -> Result<(), SpeedUpError> {
            self.log.push("csweep".to_string());
            Ok(())
        }

        fn speed_delay_trace(&mut self, _params: &SpeedParameters) -> Result<(), SpeedUpError> {
            self.log.push("delay_trace".to_string());
            Ok(())
        }

        fn speed_set_delay_data(
            &mut self,
            _params: &SpeedParameters,
            library_acceleration: bool,
        ) -> Result<(), SpeedUpError> {
            self.log
                .push(format!("set_delay_data:{library_acceleration}"));
            Ok(())
        }
    }

    #[test]
    fn speed_up_network_applies_weight_cutset_absorb_decompose_and_po_cleanup() {
        let mut network = RecordingBackend::default();

        speed_up_network_bound(&mut network, &SpeedParameters::default()).unwrap();

        assert_eq!(
            network.log,
            vec![
                "set_thresh",
                "compute_weight",
                "cutset",
                "absorb:n2",
                "absorb:n1",
                "single:n2",
                "single:n1",
                "patch:po:n2:n1",
                "delete:n2",
            ]
        );
    }

    #[test]
    fn node_interface_sets_delay_data_and_speeds_requested_node() {
        let mut network = RecordingBackend::default();

        speed_node_interface_bound(&mut network, "n", 0.5, DelayModel::Unit).unwrap();

        assert_eq!(network.log, vec!["set_delay_data:false", "single:n"]);
    }

    #[test]
    fn initial_decomp_runs_native_sweep_trace_simplify_and_cleanup_flow() {
        let mut network = RecordingBackend::default();

        speed_init_decomp_bound(&mut network, &SpeedParameters::default()).unwrap();

        assert_eq!(
            network.log,
            vec!["csweep", "delay_trace", "simplify:n", "single:n", "csweep"]
        );
    }

    #[test]
    fn algebraic_resub_network_repeats_until_substituted_source_is_deleted() {
        let mut network = RecordingBackend::default();

        speed_resub_alge_network_bound(&mut network).unwrap();

        assert_eq!(
            network.log,
            vec![
                "substitute:f:n1:false",
                "substitute:f:target:false",
                "collapse:fo:f",
                "delete:f",
            ]
        );
    }
}
