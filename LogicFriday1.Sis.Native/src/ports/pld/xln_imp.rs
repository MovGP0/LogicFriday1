//! Native Rust model for `LogicSynthesis/sis/pld/xln_imp.c`.
//!
//! The C file is a PLD improvement orchestrator. It decides when to run
//! good/disjoint/technology/cofactor decomposition, AO mapping, trivial
//! collapse, exact cover, and recursive node replacement. The actual SIS
//! rewrites still depend on `network_t`, `node_t`, `array_t`, decomposition,
//! simplify, partition, and delay ports. This module keeps the deterministic
//! decision logic native on owned summaries and reports blocked SIS-bound entry
//! points with explicit missing-port diagnostics.

use std::error::Error;
use std::fmt;

pub const AREA: f64 = 0.0;
pub const DELAY: f64 = 1.0;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoodOrFast {
    Good,
    Fast,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CofactorMode {
    Area,
    Delay,
}

impl CofactorMode {
    pub fn from_c_mode(mode: f64) -> Self {
        if (mode - AREA).abs() < f64::EPSILON {
            Self::Area
        } else {
            Self::Delay
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveFaninsOptions {
    pub move_fanins: bool,
    pub max_fanins: usize,
}

impl Default for MoveFaninsOptions {
    fn default() -> Self {
        Self {
            move_fanins: false,
            max_fanins: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XlnInitParam {
    pub support: usize,
    pub cover_node_limit: usize,
    pub lit_bound: usize,
    pub flag_decomp_good: u8,
    pub good_or_fast: GoodOrFast,
    pub absorb: bool,
    pub move_fanins: MoveFaninsOptions,
}

impl XlnInitParam {
    pub const fn new(support: usize) -> Self {
        Self {
            support,
            cover_node_limit: 0,
            lit_bound: 0,
            flag_decomp_good: 0,
            good_or_fast: GoodOrFast::Good,
            absorb: false,
            move_fanins: MoveFaninsOptions {
                move_fanins: false,
                max_fanins: 0,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImpNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub sop_literals: usize,
    pub factored_literals: usize,
    pub cubes: Vec<Vec<LiteralPhase>>,
    pub slack_rise: f64,
}

impl ImpNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            sop_literals: 0,
            factored_literals: 0,
            cubes: Vec::new(),
            slack_rise: 0.0,
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }

    pub fn with_literals(mut self, sop_literals: usize, factored_literals: usize) -> Self {
        self.sop_literals = sop_literals;
        self.factored_literals = factored_literals;
        self
    }

    pub fn with_cubes(mut self, cubes: Vec<Vec<LiteralPhase>>) -> Self {
        self.cubes = cubes;
        self
    }

    pub fn with_slack_rise(mut self, slack_rise: f64) -> Self {
        self.slack_rise = slack_rise;
        self
    }

    pub fn fanin_count(&self) -> usize {
        self.fanins.len()
    }

    pub fn cube_count(&self) -> usize {
        self.cubes.len()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImpNetwork {
    nodes: Vec<ImpNode>,
}

impl ImpNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: ImpNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&ImpNode, XlnImpError> {
        self.nodes.get(id.0).ok_or(XlnImpError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[ImpNode] {
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

    pub fn total_sop_literals(&self) -> usize {
        self.nodes.iter().map(|node| node.sop_literals).sum()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Absent,
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImpOperation {
    SelectiveGoodDecomp,
    ImproveNetwork,
    ImproveNode,
    BestScript,
    CoverOrPartition,
    NetworkPrint,
    TryOtherMappingOptions,
    CofactorDecomp,
    CofactorDecompNode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingStep {
    CreateFromNode,
    SimplifyNode,
    DecompGoodNetwork,
    DecompGoodNode,
    DecompDisjNetwork,
    DecompTechNetwork,
    AoMap,
    MoveFaninsAbsorb,
    TrivialCollapse,
    PartitionNetworkExact,
    SplitNetwork,
    CofactorDecomp,
    ReplaceNodeByNetwork,
    NetworkSweep,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkCandidate {
    pub label: CandidateLabel,
    pub internal_count: usize,
    pub steps: Vec<MappingStep>,
}

impl NetworkCandidate {
    pub fn new(label: CandidateLabel, internal_count: usize) -> Self {
        Self {
            label,
            internal_count,
            steps: Vec::new(),
        }
    }

    fn push_step(&mut self, step: MappingStep) {
        self.steps.push(step);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabel {
    FromNode,
    GoodDecomp,
    SplitCover,
    Disjoint,
    Technology,
    Cofactor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptProfile {
    pub created_internal: usize,
    pub after_ao_map: usize,
    pub after_cover_or_partition: usize,
    pub good_created_internal: usize,
    pub good_after_ao_map: usize,
    pub good_after_cover_or_partition: usize,
    pub split_after_good_internal: usize,
    pub split_after_cover_or_partition: usize,
    pub disjoint_initial_internal: usize,
    pub disjoint_after_cover_or_partition: usize,
    pub technology_after_cover_or_partition: usize,
    pub cofactor_internal: usize,
}

impl Default for ScriptProfile {
    fn default() -> Self {
        Self {
            created_internal: 1,
            after_ao_map: 1,
            after_cover_or_partition: 1,
            good_created_internal: 1,
            good_after_ao_map: 1,
            good_after_cover_or_partition: 1,
            split_after_good_internal: 1,
            split_after_cover_or_partition: 1,
            disjoint_initial_internal: 1,
            disjoint_after_cover_or_partition: 1,
            technology_after_cover_or_partition: 1,
            cofactor_internal: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverPartitionPlan {
    pub steps: Vec<MappingStep>,
    pub uses_exact_cover: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImproveNodePlan {
    pub node: NodeId,
    pub changed: bool,
    pub replacement: Option<NetworkCandidate>,
    pub steps: Vec<MappingStep>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImproveNetworkPlan {
    pub node_plans: Vec<ImproveNodePlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnImpError {
    UnknownNode(NodeId),
    InvalidSupport { support: usize },
    CubeArityMismatch { expected: usize, actual: usize },
    NoFanins,
    MissingNativePorts { operation: ImpOperation },
}

impl fmt::Display for XlnImpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown xln_imp node {:?}", node),
            Self::InvalidSupport { support } => {
                write!(f, "PLD support must be positive, got {support}")
            }
            Self::CubeArityMismatch { expected, actual } => {
                write!(f, "cube has {actual} literals, expected {expected}")
            }
            Self::NoFanins => write!(f, "cofactor fanin selection requires at least one fanin"),
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation:?} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for XlnImpError {}

pub fn selective_good_decomp_blocked<Network>(
    _network: &mut Network,
    _lit_limit: usize,
    _alpha: f64,
) -> Result<(), XlnImpError> {
    missing_native_ports(ImpOperation::SelectiveGoodDecomp)
}

pub fn improve_network_blocked<Network>(
    _network: &mut Network,
    _init_param: XlnInitParam,
) -> Result<(), XlnImpError> {
    missing_native_ports(ImpOperation::ImproveNetwork)
}

pub fn improve_node_blocked<Node>(
    _node: &mut Node,
    _init_param: XlnInitParam,
) -> Result<(), XlnImpError> {
    missing_native_ports(ImpOperation::ImproveNode)
}

pub fn best_script_blocked<Node>(
    _node: &Node,
    _init_param: XlnInitParam,
) -> Result<(), XlnImpError> {
    missing_native_ports(ImpOperation::BestScript)
}

pub fn cofactor_decomp_blocked<Node>(
    _node: &Node,
    _support: usize,
    _mode: CofactorMode,
) -> Result<(), XlnImpError> {
    missing_native_ports(ImpOperation::CofactorDecomp)
}

fn missing_native_ports<T>(operation: ImpOperation) -> Result<T, XlnImpError> {
    Err(XlnImpError::MissingNativePorts { operation })
}

pub fn selective_good_decomp_nodes(
    network: &ImpNetwork,
    lit_limit: usize,
    alpha: f64,
) -> Vec<NodeId> {
    if network.total_sop_literals() < lit_limit {
        return Vec::new();
    }

    network
        .dfs_node_ids()
        .into_iter()
        .filter(|id| {
            let node = &network.nodes[id.0];
            (node.factored_literals as f64) < node.sop_literals as f64 * alpha
        })
        .collect()
}

pub fn cover_or_partition_plan(
    internal_count: usize,
    init_param: &XlnInitParam,
) -> CoverPartitionPlan {
    let mut steps = vec![MappingStep::TrivialCollapse];
    let uses_exact_cover = internal_count <= init_param.cover_node_limit;
    if uses_exact_cover {
        steps.push(MappingStep::PartitionNetworkExact);
    }
    CoverPartitionPlan {
        steps,
        uses_exact_cover,
    }
}

pub fn best_script_for_node(
    node: &ImpNode,
    init_param: &XlnInitParam,
    profile: &ScriptProfile,
) -> Result<NetworkCandidate, XlnImpError> {
    if init_param.support == 0 {
        return Err(XlnImpError::InvalidSupport {
            support: init_param.support,
        });
    }

    let num_fanin = node.fanin_count();
    if num_fanin <= init_param.support {
        let mut candidate =
            NetworkCandidate::new(CandidateLabel::FromNode, profile.created_internal);
        candidate.push_step(MappingStep::CreateFromNode);
        return Ok(candidate);
    }

    let mut network1 = NetworkCandidate::new(CandidateLabel::FromNode, profile.created_internal);
    network1.push_step(MappingStep::CreateFromNode);

    if node.sop_literals == num_fanin {
        network1.internal_count = profile.after_ao_map;
        network1.push_step(MappingStep::AoMap);
        return Ok(network1);
    }

    if init_param.flag_decomp_good == 1 {
        network1.push_step(MappingStep::DecompGoodNetwork);
        if init_param.absorb {
            network1.push_step(MappingStep::MoveFaninsAbsorb);
        }
    }

    network1.internal_count = profile.after_ao_map;
    network1.push_step(MappingStep::AoMap);
    if network1.internal_count == 2 {
        return Ok(network1);
    }

    let cover_plan = cover_or_partition_plan(network1.internal_count, init_param);
    network1.steps.extend(cover_plan.steps);
    network1.internal_count = profile.after_cover_or_partition;

    if init_param.flag_decomp_good == 2 && profile.good_created_internal != 1 {
        let mut network2 =
            NetworkCandidate::new(CandidateLabel::GoodDecomp, profile.good_created_internal);
        network2.push_step(MappingStep::CreateFromNode);
        network2.push_step(MappingStep::DecompGoodNetwork);
        if init_param.absorb {
            network2.push_step(MappingStep::MoveFaninsAbsorb);
        }
        network2.internal_count = profile.good_after_ao_map;
        network2.push_step(MappingStep::AoMap);
        let cover_plan = cover_or_partition_plan(network2.internal_count, init_param);
        network2.steps.extend(cover_plan.steps);
        network2.internal_count = profile.good_after_cover_or_partition;
        if network2.internal_count < network1.internal_count {
            network1 = network2;
        }
    }

    if should_use_cofactor(num_fanin, init_param.support, network1.internal_count) {
        let mut cofactor =
            NetworkCandidate::new(CandidateLabel::Cofactor, profile.cofactor_internal);
        cofactor.push_step(MappingStep::CofactorDecomp);
        network1 = cofactor;
    }

    if init_param.good_or_fast == GoodOrFast::Fast {
        return Ok(network1);
    }

    Ok(try_other_mapping_options_for_node(
        node, network1, init_param, profile,
    ))
}

pub fn try_other_mapping_options_for_node(
    node: &ImpNode,
    current: NetworkCandidate,
    init_param: &XlnInitParam,
    profile: &ScriptProfile,
) -> NetworkCandidate {
    let num_lit = node.sop_literals;
    let mut network1 = current;
    let mut num1 = network1.internal_count;

    if num1 <= 2 || node.fanin_count() == num_lit {
        return network1;
    }

    let mut flag = false;
    let mut network4 = NetworkCandidate::new(CandidateLabel::SplitCover, profile.created_internal);
    network4.push_step(MappingStep::CreateFromNode);
    if num_lit > init_param.lit_bound || init_param.flag_decomp_good != 0 {
        network4.push_step(MappingStep::DecompGoodNetwork);
    }
    if num_lit > init_param.lit_bound {
        network4.internal_count = profile.split_after_good_internal;
        if network4.internal_count > 1 {
            flag = true;
        }
    }
    network4.push_step(MappingStep::SplitNetwork);
    let cover_plan = cover_or_partition_plan(profile.split_after_cover_or_partition, init_param);
    network4.steps.extend(cover_plan.steps);
    network4.internal_count = profile.split_after_cover_or_partition;

    if network4.internal_count == 2 {
        return network4;
    }
    if network4.internal_count < num1 {
        num1 = network4.internal_count;
        network1 = network4;
    }

    let mut network3 =
        NetworkCandidate::new(CandidateLabel::Disjoint, profile.disjoint_initial_internal);
    network3.push_step(MappingStep::CreateFromNode);
    if init_param.flag_decomp_good != 0 {
        network3.push_step(MappingStep::DecompGoodNetwork);
    }
    network3.push_step(MappingStep::DecompDisjNetwork);
    if flag || network3.internal_count != 1 {
        network3.push_step(MappingStep::SplitNetwork);
        let cover_plan =
            cover_or_partition_plan(profile.disjoint_after_cover_or_partition, init_param);
        network3.steps.extend(cover_plan.steps);
        network3.internal_count = profile.disjoint_after_cover_or_partition;
        if num1 >= network3.internal_count {
            num1 = network3.internal_count;
            network1 = network3;
        }
    }

    if num1 == 2 {
        return network1;
    }

    let mut network2 = NetworkCandidate::new(
        CandidateLabel::Technology,
        profile.technology_after_cover_or_partition,
    );
    network2.push_step(MappingStep::CreateFromNode);
    if init_param.flag_decomp_good != 0 {
        network2.push_step(MappingStep::DecompGoodNetwork);
    }
    network2.push_step(MappingStep::DecompTechNetwork);
    let cover_plan = cover_or_partition_plan(network2.internal_count, init_param);
    network2.steps.extend(cover_plan.steps);
    if num1 >= network2.internal_count {
        network1 = network2;
    }

    network1
}

pub fn should_use_cofactor(num_fanin: usize, support: usize, current_internal: usize) -> bool {
    if !(support > 2 && support <= 5) {
        return false;
    }
    if num_fanin < support {
        return false;
    }
    let diff = num_fanin - support + 1;
    let upper_bound = if diff <= 31 {
        ((1_u64 << diff) - 1) as f64
    } else {
        2_f64.powi(diff as i32) - 1.0
    };
    current_internal as f64 > upper_bound
}

pub fn improve_node_plan(
    network: &ImpNetwork,
    node: NodeId,
    init_param: &XlnInitParam,
    profile: &ScriptProfile,
) -> Result<ImproveNodePlan, XlnImpError> {
    let node_ref = network.node(node)?;
    if node_ref.kind != NodeKind::Internal || node_ref.fanin_count() <= init_param.support {
        return Ok(ImproveNodePlan {
            node,
            changed: false,
            replacement: None,
            steps: Vec::new(),
        });
    }

    let replacement = best_script_for_node(node_ref, init_param, profile)?;
    let mut steps = vec![MappingStep::SimplifyNode];
    steps.extend(replacement.steps.iter().copied());
    steps.push(MappingStep::ReplaceNodeByNetwork);

    Ok(ImproveNodePlan {
        node,
        changed: true,
        replacement: Some(replacement),
        steps,
    })
}

pub fn improve_network_plan(
    network: &ImpNetwork,
    init_param: &XlnInitParam,
    profile: &ScriptProfile,
) -> Result<ImproveNetworkPlan, XlnImpError> {
    let mut node_plans = Vec::new();
    for node in network.dfs_node_ids() {
        node_plans.push(improve_node_plan(network, node, init_param, profile)?);
    }
    Ok(ImproveNetworkPlan { node_plans })
}

pub fn select_fanin_for_cofactor_area(node: &ImpNode) -> Result<NodeId, XlnImpError> {
    if node.fanins.is_empty() {
        return Err(XlnImpError::NoFanins);
    }
    for cube in &node.cubes {
        if cube.len() != node.fanins.len() {
            return Err(XlnImpError::CubeArityMismatch {
                expected: node.fanins.len(),
                actual: cube.len(),
            });
        }
    }

    for (index, fanin) in node.fanins.iter().copied().enumerate() {
        let all_positive = node
            .cubes
            .iter()
            .all(|cube| cube[index] == LiteralPhase::Positive);
        let all_negative = node
            .cubes
            .iter()
            .all(|cube| cube[index] == LiteralPhase::Negative);
        if all_positive || all_negative {
            return Ok(fanin);
        }
    }

    node.fanins.last().copied().ok_or(XlnImpError::NoFanins)
}

pub fn select_fanin_for_cofactor_delay(
    network: &ImpNetwork,
    node: &ImpNode,
) -> Result<NodeId, XlnImpError> {
    let mut best: Option<(NodeId, f64)> = None;
    for fanin in &node.fanins {
        let slack = network.node(*fanin)?.slack_rise;
        if best.is_none_or(|(_, best_slack)| slack < best_slack) {
            best = Some((*fanin, slack));
        }
    }
    best.map(|(id, _)| id).ok_or(XlnImpError::NoFanins)
}

pub fn select_fanin_for_cofactor(
    network: &ImpNetwork,
    node: &ImpNode,
    mode: CofactorMode,
) -> Result<NodeId, XlnImpError> {
    match mode {
        CofactorMode::Area => select_fanin_for_cofactor_area(node),
        CofactorMode::Delay => select_fanin_for_cofactor_delay(network, node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params() -> XlnInitParam {
        XlnInitParam {
            support: 5,
            cover_node_limit: 4,
            lit_bound: 10,
            flag_decomp_good: 0,
            good_or_fast: GoodOrFast::Good,
            absorb: false,
            move_fanins: MoveFaninsOptions::default(),
        }
    }

    #[test]
    fn selective_good_decomp_is_skipped_for_small_networks() {
        let mut network = ImpNetwork::new();
        network.add_node(
            ImpNode::new("a", NodeKind::Internal)
                .with_fanins(vec![NodeId(10), NodeId(11)])
                .with_literals(7, 1),
        );

        assert_eq!(selective_good_decomp_nodes(&network, 8, 0.8), Vec::new());
    }

    #[test]
    fn selective_good_decomp_selects_nodes_below_alpha_ratio() {
        let mut network = ImpNetwork::new();
        let a = network.add_node(
            ImpNode::new("a", NodeKind::Internal)
                .with_fanins(vec![NodeId(10), NodeId(11)])
                .with_literals(10, 4),
        );
        network.add_node(
            ImpNode::new("b", NodeKind::Internal)
                .with_fanins(vec![NodeId(10), NodeId(12)])
                .with_literals(10, 9),
        );

        assert_eq!(selective_good_decomp_nodes(&network, 1, 0.5), vec![a]);
    }

    #[test]
    fn best_script_returns_created_network_for_feasible_node() {
        let node = ImpNode::new("n", NodeKind::Internal)
            .with_fanins(vec![NodeId(0), NodeId(1)])
            .with_literals(5, 4);
        let profile = ScriptProfile {
            created_internal: 1,
            ..ScriptProfile::default()
        };

        let candidate = best_script_for_node(&node, &default_params(), &profile).unwrap();

        assert_eq!(candidate.label, CandidateLabel::FromNode);
        assert_eq!(candidate.steps, vec![MappingStep::CreateFromNode]);
    }

    #[test]
    fn best_script_ao_maps_literal_only_nodes_and_stops() {
        let node = ImpNode::new("n", NodeKind::Internal)
            .with_fanins(vec![
                NodeId(0),
                NodeId(1),
                NodeId(2),
                NodeId(3),
                NodeId(4),
                NodeId(5),
            ])
            .with_literals(6, 6);
        let profile = ScriptProfile {
            after_ao_map: 3,
            ..ScriptProfile::default()
        };

        let candidate = best_script_for_node(&node, &default_params(), &profile).unwrap();

        assert_eq!(candidate.internal_count, 3);
        assert_eq!(
            candidate.steps,
            vec![MappingStep::CreateFromNode, MappingStep::AoMap]
        );
    }

    #[test]
    fn cover_or_partition_adds_exact_cover_only_under_limit() {
        let mut params = default_params();
        params.cover_node_limit = 3;

        assert_eq!(
            cover_or_partition_plan(3, &params).steps,
            vec![
                MappingStep::TrivialCollapse,
                MappingStep::PartitionNetworkExact
            ]
        );
        assert_eq!(
            cover_or_partition_plan(4, &params).steps,
            vec![MappingStep::TrivialCollapse]
        );
    }

    #[test]
    fn flag_two_good_decomp_candidate_replaces_base_only_when_smaller() {
        let node = ImpNode::new("n", NodeKind::Internal)
            .with_fanins((0..7).map(NodeId).collect())
            .with_literals(20, 11);
        let mut params = default_params();
        params.flag_decomp_good = 2;
        params.good_or_fast = GoodOrFast::Fast;
        let profile = ScriptProfile {
            after_ao_map: 8,
            after_cover_or_partition: 6,
            good_created_internal: 3,
            good_after_ao_map: 4,
            good_after_cover_or_partition: 5,
            ..ScriptProfile::default()
        };

        let candidate = best_script_for_node(&node, &params, &profile).unwrap();

        assert_eq!(candidate.label, CandidateLabel::GoodDecomp);
        assert_eq!(candidate.internal_count, 5);
    }

    #[test]
    fn cofactor_replaces_candidate_when_internal_count_exceeds_bound() {
        let node = ImpNode::new("n", NodeKind::Internal)
            .with_fanins((0..7).map(NodeId).collect())
            .with_literals(20, 10);
        let mut params = default_params();
        params.support = 5;
        params.good_or_fast = GoodOrFast::Fast;
        let profile = ScriptProfile {
            after_ao_map: 40,
            after_cover_or_partition: 9,
            cofactor_internal: 4,
            ..ScriptProfile::default()
        };

        let candidate = best_script_for_node(&node, &params, &profile).unwrap();

        assert_eq!(candidate.label, CandidateLabel::Cofactor);
        assert_eq!(candidate.internal_count, 4);
        assert_eq!(candidate.steps, vec![MappingStep::CofactorDecomp]);
    }

    #[test]
    fn try_other_options_immediately_accepts_two_node_split_cover() {
        let node = ImpNode::new("n", NodeKind::Internal)
            .with_fanins((0..7).map(NodeId).collect())
            .with_literals(20, 10);
        let profile = ScriptProfile {
            split_after_good_internal: 3,
            split_after_cover_or_partition: 2,
            ..ScriptProfile::default()
        };

        let candidate = try_other_mapping_options_for_node(
            &node,
            NetworkCandidate::new(CandidateLabel::FromNode, 8),
            &default_params(),
            &profile,
        );

        assert_eq!(candidate.label, CandidateLabel::SplitCover);
        assert_eq!(candidate.internal_count, 2);
    }

    #[test]
    fn try_other_options_accepts_equal_disjoint_and_technology_like_c() {
        let node = ImpNode::new("n", NodeKind::Internal)
            .with_fanins((0..8).map(NodeId).collect())
            .with_literals(20, 10);
        let profile = ScriptProfile {
            split_after_good_internal: 1,
            split_after_cover_or_partition: 7,
            disjoint_initial_internal: 3,
            disjoint_after_cover_or_partition: 5,
            technology_after_cover_or_partition: 5,
            ..ScriptProfile::default()
        };

        let candidate = try_other_mapping_options_for_node(
            &node,
            NetworkCandidate::new(CandidateLabel::FromNode, 5),
            &default_params(),
            &profile,
        );

        assert_eq!(candidate.label, CandidateLabel::Technology);
        assert_eq!(candidate.internal_count, 5);
    }

    #[test]
    fn improve_node_plan_skips_non_internal_and_feasible_nodes() {
        let mut network = ImpNetwork::new();
        let pi = network.add_node(ImpNode::new("pi", NodeKind::PrimaryInput));
        let internal = network.add_node(
            ImpNode::new("n", NodeKind::Internal)
                .with_fanins(vec![pi])
                .with_literals(1, 1),
        );

        assert!(
            !improve_node_plan(&network, pi, &default_params(), &ScriptProfile::default())
                .unwrap()
                .changed
        );
        assert!(
            !improve_node_plan(
                &network,
                internal,
                &default_params(),
                &ScriptProfile::default()
            )
            .unwrap()
            .changed
        );
    }

    #[test]
    fn improve_node_plan_simplifies_maps_and_replaces_infeasible_internal_node() {
        let mut network = ImpNetwork::new();
        let n = network.add_node(
            ImpNode::new("n", NodeKind::Internal)
                .with_fanins((0..6).map(NodeId).collect())
                .with_literals(6, 6),
        );

        let plan =
            improve_node_plan(&network, n, &default_params(), &ScriptProfile::default()).unwrap();

        assert!(plan.changed);
        assert_eq!(plan.steps.first(), Some(&MappingStep::SimplifyNode));
        assert_eq!(plan.steps.last(), Some(&MappingStep::ReplaceNodeByNetwork));
    }

    #[test]
    fn area_cofactor_prefers_fanin_with_same_phase_in_all_cubes() {
        let node = ImpNode::new("n", NodeKind::Internal)
            .with_fanins(vec![NodeId(0), NodeId(1), NodeId(2)])
            .with_cubes(vec![
                vec![
                    LiteralPhase::Absent,
                    LiteralPhase::Positive,
                    LiteralPhase::Negative,
                ],
                vec![
                    LiteralPhase::Negative,
                    LiteralPhase::Positive,
                    LiteralPhase::Absent,
                ],
            ]);

        assert_eq!(select_fanin_for_cofactor_area(&node).unwrap(), NodeId(1));
    }

    #[test]
    fn area_cofactor_falls_back_to_last_fanin() {
        let node = ImpNode::new("n", NodeKind::Internal)
            .with_fanins(vec![NodeId(0), NodeId(1)])
            .with_cubes(vec![
                vec![LiteralPhase::Positive, LiteralPhase::Negative],
                vec![LiteralPhase::Negative, LiteralPhase::Positive],
            ]);

        assert_eq!(select_fanin_for_cofactor_area(&node).unwrap(), NodeId(1));
    }

    #[test]
    fn delay_cofactor_picks_lowest_rise_slack_fanin() {
        let mut network = ImpNetwork::new();
        let a = network.add_node(ImpNode::new("a", NodeKind::Internal).with_slack_rise(3.0));
        let b = network.add_node(ImpNode::new("b", NodeKind::Internal).with_slack_rise(-1.0));
        let node = ImpNode::new("n", NodeKind::Internal).with_fanins(vec![a, b]);

        assert_eq!(select_fanin_for_cofactor_delay(&network, &node).unwrap(), b);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_imp.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
