//! Native Rust fanout optimization planning for `sis/map/fanout_opt.c`.
//!
//! The C file mutates a SIS `network_t`: it extracts gate links, tries enabled
//! fanout algorithms for one or both polarities, selects the best cost, builds
//! fanout trees, removes unused virtual sources, and optionally performs global
//! area recovery. Native SIS graph mutation and the individual fanout tree
//! algorithms are still separate porting units, so this module keeps the owned
//! data part explicit: bounded extraction, deterministic source-assignment
//! scoring, algorithm/property selection, and typed dependency errors for the
//! full mutating operations.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use super::virtual_net::{
    DelayTime, GateLink, NodeId, NodeKind, VirtualMappedNetwork, VirtualNetworkError,
};

pub const DEFAULT_LIMITS: FanoutPlanningLimits = FanoutPlanningLimits {
    max_algorithms: 256,
    max_properties_per_algorithm: 64,
    max_sinks_per_polarity: 65_536,
    max_candidate_trees_per_source: 4_096,
    max_total_load: 1.0e12,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Polarity {
    X,
    Y,
}

impl Polarity {
    pub fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
        }
    }

    pub fn inverse(self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::X,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutPlanningLimits {
    pub max_algorithms: usize,
    pub max_properties_per_algorithm: usize,
    pub max_sinks_per_polarity: usize,
    pub max_candidate_trees_per_source: usize,
    pub max_total_load: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutPlanningOptions {
    pub optimize_single_fanout: bool,
    pub iterate: bool,
    pub check_load_limit: bool,
    pub penalty_factor: f64,
    pub limits: FanoutPlanningLimits,
}

impl Default for FanoutPlanningOptions {
    fn default() -> Self {
        Self {
            optimize_single_fanout: false,
            iterate: false,
            check_load_limit: false,
            penalty_factor: 1.0,
            limits: DEFAULT_LIMITS,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutAlgorithm {
    pub name: String,
    pub enabled: bool,
    pub min_size: usize,
    pub peephole: bool,
    pub properties: Vec<FanoutAlgorithmProperty>,
}

impl FanoutAlgorithm {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: false,
            min_size: 0,
            peephole: false,
            properties: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutAlgorithmProperty {
    pub name: String,
    pub value: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutCost {
    pub slack: DelayTime,
    pub area: f64,
}

impl FanoutCost {
    pub fn new(slack: DelayTime, area: f64) -> Self {
        Self { slack, area }
    }

    pub fn impossible() -> Self {
        Self {
            slack: DelayTime::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            area: f64::INFINITY,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSink {
    pub node: NodeId,
    pub pin: isize,
    pub load: f64,
    pub required: DelayTime,
}

impl From<GateLink> for FanoutSink {
    fn from(value: GateLink) -> Self {
        Self {
            node: value.node,
            pin: value.pin,
            load: value.load,
            required: value.required,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutInfo {
    pub polarity: Polarity,
    pub sinks: Vec<FanoutSink>,
    pub cumulative_load: Vec<f64>,
    pub minimum_required: Vec<DelayTime>,
    pub total_load: f64,
}

impl FanoutInfo {
    pub fn empty(polarity: Polarity) -> Self {
        Self {
            polarity,
            sinks: Vec::new(),
            cumulative_load: vec![0.0],
            minimum_required: Vec::new(),
            total_load: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSource {
    pub node: NodeId,
    pub polarity: Polarity,
    pub is_external: bool,
    pub existing_gate_area: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutProblem {
    pub root: NodeId,
    pub sources: BTreeMap<Polarity, FanoutSource>,
    pub fanout_info: BTreeMap<Polarity, FanoutInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutTreeCandidate {
    pub algorithm: String,
    pub source: Polarity,
    pub sinks: Vec<Polarity>,
    pub cost: FanoutCost,
    pub buffer_count: usize,
    pub depth: usize,
    pub edge_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceAssignment {
    XAlone,
    YAlone,
    XOnX,
    XOnY,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSourcePlan {
    pub source: Polarity,
    pub tree: FanoutTreeCandidate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutOptimizationPlan {
    pub root: NodeId,
    pub assignment: SourceAssignment,
    pub source_plans: Vec<FanoutSourcePlan>,
    pub cost: FanoutCost,
    pub removed_sources: Vec<Polarity>,
    pub saved_iteration_source: Option<Polarity>,
    pub total_sinks: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FanoutOptError {
    EmptyName {
        kind: &'static str,
    },
    DuplicateAlgorithm {
        name: String,
    },
    UnknownAlgorithm {
        name: String,
    },
    UnknownProperty {
        algorithm: String,
        property: String,
    },
    InvalidMetric {
        context: String,
        metric: &'static str,
        value: f64,
    },
    LimitExceeded {
        limit: &'static str,
        max: usize,
    },
    MissingNode(NodeId),
    MissingExternalSource {
        polarity: Polarity,
    },
    NoFanoutProblem {
        root: NodeId,
    },
    NoEnabledAlgorithm,
    NoCandidate {
        source: Polarity,
    },
    VirtualNetwork(VirtualNetworkError),
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for FanoutOptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName { kind } => write!(f, "{kind} name cannot be empty"),
            Self::DuplicateAlgorithm { name } => write!(f, "duplicate fanout algorithm '{name}'"),
            Self::UnknownAlgorithm { name } => write!(f, "unknown fanout algorithm '{name}'"),
            Self::UnknownProperty {
                algorithm,
                property,
            } => write!(
                f,
                "unknown fanout property '{property}' for algorithm '{algorithm}'"
            ),
            Self::InvalidMetric {
                context,
                metric,
                value,
            } => write!(f, "{context} has invalid {metric} {value}"),
            Self::LimitExceeded { limit, max } => write!(f, "{limit} exceeds limit {max}"),
            Self::MissingNode(node) => write!(f, "missing fanout node {}", node.index()),
            Self::MissingExternalSource { polarity } => {
                write!(f, "fanout source {polarity:?} is not an external source")
            }
            Self::NoFanoutProblem { root } => {
                write!(
                    f,
                    "node {} does not have enough fanouts to optimize",
                    root.index()
                )
            }
            Self::NoEnabledAlgorithm => write!(f, "no fanout algorithms are enabled"),
            Self::NoCandidate { source } => {
                write!(f, "no fanout tree candidate was available for {source:?}")
            }
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for FanoutOptError {}

impl From<VirtualNetworkError> for FanoutOptError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

pub fn fanout_optimization_unavailable() -> Result<FanoutOptimizationPlan, FanoutOptError> {
    Err(FanoutOptError::MissingSisPorts {
        operation: "fanout_optimization full SIS network mutation",
    })
}

pub fn configure_algorithms(
    algorithms: &[FanoutAlgorithm],
    enabled_names: &[&str],
    limits: FanoutPlanningLimits,
) -> Result<Vec<FanoutAlgorithm>, FanoutOptError> {
    validate_algorithms(algorithms, limits)?;

    let mut result = algorithms.to_vec();
    if enabled_names.is_empty() {
        return Ok(result);
    }

    let known = result
        .iter()
        .map(|algorithm| algorithm.name.as_str())
        .collect::<BTreeSet<_>>();
    for name in enabled_names {
        if !known.contains(name) {
            return Err(FanoutOptError::UnknownAlgorithm {
                name: (*name).to_string(),
            });
        }
    }

    for algorithm in &mut result {
        algorithm.enabled = algorithm.name == "noalg";
    }
    for name in enabled_names {
        if let Some(algorithm) = result.iter_mut().find(|item| item.name == *name) {
            algorithm.enabled = true;
        }
    }

    Ok(result)
}

pub fn set_algorithm_property(
    algorithms: &mut [FanoutAlgorithm],
    algorithm_name: &str,
    property_name: &str,
    value: i32,
) -> Result<usize, FanoutOptError> {
    let Some(index) = algorithms
        .iter()
        .position(|algorithm| algorithm.name == algorithm_name)
    else {
        return Err(FanoutOptError::UnknownAlgorithm {
            name: algorithm_name.to_string(),
        });
    };

    let algorithm = &mut algorithms[index];
    let Some(property) = algorithm
        .properties
        .iter_mut()
        .find(|property| property.name == property_name)
    else {
        return Err(FanoutOptError::UnknownProperty {
            algorithm: algorithm_name.to_string(),
            property: property_name.to_string(),
        });
    };

    property.value = value;
    match property_name {
        "peephole" => algorithm.peephole = value != 0,
        "size" | "min_size" => algorithm.min_size = value.max(0) as usize,
        _ => {}
    }

    Ok(index)
}

pub fn preprocess_fanout_info(
    polarity: Polarity,
    sinks: impl IntoIterator<Item = FanoutSink>,
    limits: FanoutPlanningLimits,
) -> Result<FanoutInfo, FanoutOptError> {
    let mut sinks = sinks.into_iter().collect::<Vec<_>>();
    if sinks.len() > limits.max_sinks_per_polarity {
        return Err(FanoutOptError::LimitExceeded {
            limit: "fanout sinks per polarity",
            max: limits.max_sinks_per_polarity,
        });
    }

    for sink in &sinks {
        validate_non_negative_finite("fanout sink", "load", sink.load)?;
        validate_delay("fanout sink required", sink.required, true)?;
    }

    sinks.sort_by(compare_sinks);

    let mut cumulative_load = Vec::with_capacity(sinks.len() + 1);
    let mut total_load = 0.0;
    cumulative_load.push(total_load);
    for sink in &sinks {
        total_load += sink.load;
        if total_load > limits.max_total_load {
            return Err(FanoutOptError::InvalidMetric {
                context: "fanout info".to_string(),
                metric: "total_load",
                value: total_load,
            });
        }
        cumulative_load.push(total_load);
    }

    let mut minimum_required = vec![DelayTime::new(f64::INFINITY, f64::INFINITY); sinks.len()];
    let mut required = DelayTime::new(f64::INFINITY, f64::INFINITY);
    for index in (0..sinks.len()).rev() {
        required = min_delay(required, sinks[index].required);
        minimum_required[index] = required;
    }

    Ok(FanoutInfo {
        polarity,
        sinks,
        cumulative_load,
        minimum_required,
        total_load,
    })
}

pub fn extract_virtual_fanout_problem(
    network: &VirtualMappedNetwork,
    root: NodeId,
    inverted_root: Option<NodeId>,
    options: FanoutPlanningOptions,
) -> Result<FanoutProblem, FanoutOptError> {
    validate_options(options)?;
    let root_node = network
        .node(root)
        .ok_or(FanoutOptError::MissingNode(root))?;
    let inverted_root_node = inverted_root
        .map(|node| network.node(node).ok_or(FanoutOptError::MissingNode(node)))
        .transpose()?;

    let mut sources = BTreeMap::new();
    if is_external_source(root_node) {
        sources.insert(
            Polarity::X,
            FanoutSource {
                node: root,
                polarity: Polarity::X,
                is_external: true,
                existing_gate_area: mapped_gate_area(root_node),
            },
        );
    }
    if let Some(node) = inverted_root_node {
        if is_external_source(node) {
            sources.insert(
                Polarity::Y,
                FanoutSource {
                    node: inverted_root.expect("node was checked above"),
                    polarity: Polarity::Y,
                    is_external: true,
                    existing_gate_area: mapped_gate_area(node),
                },
            );
        }
    }

    let mut fanout_info = BTreeMap::new();
    let x_sinks = extract_links(network, root, inverted_root)?;
    fanout_info.insert(
        Polarity::X,
        preprocess_fanout_info(Polarity::X, x_sinks, options.limits)?,
    );
    let y_sinks = if let Some(inverted_root) = inverted_root {
        extract_links(network, inverted_root, Some(root))?
    } else {
        Vec::new()
    };
    fanout_info.insert(
        Polarity::Y,
        preprocess_fanout_info(Polarity::Y, y_sinks, options.limits)?,
    );

    let total_sinks = fanout_info
        .values()
        .map(|info| info.sinks.len())
        .sum::<usize>();
    if total_sinks > 1 || (total_sinks >= 1 && options.optimize_single_fanout) {
        Ok(FanoutProblem {
            root,
            sources,
            fanout_info,
        })
    } else {
        Err(FanoutOptError::NoFanoutProblem { root })
    }
}

pub fn plan_fanout_optimization(
    problem: &FanoutProblem,
    algorithms: &[FanoutAlgorithm],
    candidates: &[FanoutTreeCandidate],
    options: FanoutPlanningOptions,
) -> Result<FanoutOptimizationPlan, FanoutOptError> {
    validate_options(options)?;
    validate_algorithms(algorithms, options.limits)?;
    validate_candidates(candidates, options.limits)?;

    if !algorithms.iter().any(|algorithm| algorithm.enabled) {
        return Err(FanoutOptError::NoEnabledAlgorithm);
    }

    let total_sinks = problem
        .fanout_info
        .values()
        .map(|info| info.sinks.len())
        .sum::<usize>();
    if total_sinks <= 1 && !options.optimize_single_fanout {
        return Err(FanoutOptError::NoFanoutProblem { root: problem.root });
    }

    let eligible_algorithms = algorithms
        .iter()
        .filter(|algorithm| algorithm.enabled && algorithm.min_size <= total_sinks)
        .map(|algorithm| algorithm.name.as_str())
        .collect::<BTreeSet<_>>();
    if eligible_algorithms.is_empty() {
        return Err(FanoutOptError::NoEnabledAlgorithm);
    }

    let mut plans = Vec::new();
    if problem.sources.contains_key(&Polarity::X) {
        plans.push(plan_assignment(
            problem,
            SourceAssignment::XAlone,
            &[(Polarity::X, &[Polarity::X, Polarity::Y][..])],
            candidates,
            &eligible_algorithms,
        )?);
    }
    if problem.sources.contains_key(&Polarity::Y) {
        plans.push(plan_assignment(
            problem,
            SourceAssignment::YAlone,
            &[(Polarity::Y, &[Polarity::X, Polarity::Y][..])],
            candidates,
            &eligible_algorithms,
        )?);
    }

    let has_both_sources =
        problem.sources.contains_key(&Polarity::X) && problem.sources.contains_key(&Polarity::Y);
    let has_both_sink_polarities =
        fanout_count(problem, Polarity::X) > 0 && fanout_count(problem, Polarity::Y) > 0;
    if has_both_sources && has_both_sink_polarities {
        plans.push(plan_assignment(
            problem,
            SourceAssignment::XOnX,
            &[
                (Polarity::X, &[Polarity::X][..]),
                (Polarity::Y, &[Polarity::Y][..]),
            ],
            candidates,
            &eligible_algorithms,
        )?);
        plans.push(plan_assignment(
            problem,
            SourceAssignment::XOnY,
            &[
                (Polarity::X, &[Polarity::Y][..]),
                (Polarity::Y, &[Polarity::X][..]),
            ],
            candidates,
            &eligible_algorithms,
        )?);
    }

    let mut best = plans
        .into_iter()
        .reduce(|best, plan| {
            if is_better_cost(plan.cost, best.cost) {
                plan
            } else {
                best
            }
        })
        .ok_or(FanoutOptError::NoEnabledAlgorithm)?;

    best.saved_iteration_source = if options.iterate && best.total_sinks > 1 {
        best.source_plans
            .first()
            .map(|source_plan| source_plan.source)
    } else {
        None
    };

    Ok(best)
}

pub fn is_better_cost(candidate: FanoutCost, incumbent: FanoutCost) -> bool {
    let slack1 = normalized_min_slack(candidate.slack);
    let slack2 = normalized_min_slack(incumbent.slack);
    let diff = normalize_zero(slack1 - slack2);

    if slack2 < 0.0 {
        diff > 0.0
    } else if slack1 < 0.0 {
        false
    } else {
        candidate.area < incumbent.area
    }
}

fn plan_assignment(
    problem: &FanoutProblem,
    assignment: SourceAssignment,
    source_sinks: &[(Polarity, &[Polarity])],
    candidates: &[FanoutTreeCandidate],
    eligible_algorithms: &BTreeSet<&str>,
) -> Result<FanoutOptimizationPlan, FanoutOptError> {
    let mut source_plans = Vec::new();
    let mut cost = FanoutCost::new(DelayTime::new(f64::INFINITY, f64::INFINITY), 0.0);
    let mut assigned_sources = BTreeSet::new();

    for (source, sinks) in source_sinks {
        let source_info = problem
            .sources
            .get(source)
            .ok_or(FanoutOptError::MissingExternalSource { polarity: *source })?;
        let tree = select_tree_candidate(*source, sinks, candidates, eligible_algorithms)?;
        cost = add_cost(cost, tree.cost);
        assigned_sources.insert(*source);
        source_plans.push(FanoutSourcePlan {
            source: *source,
            tree,
        });

        if source_sinks.len() > 1 {
            cost.area += source_info.existing_gate_area;
        }
    }

    let removed_sources = [Polarity::X, Polarity::Y]
        .into_iter()
        .filter(|polarity| problem.sources.contains_key(polarity))
        .filter(|polarity| !assigned_sources.contains(polarity))
        .collect::<Vec<_>>();

    Ok(FanoutOptimizationPlan {
        root: problem.root,
        assignment,
        source_plans,
        cost,
        removed_sources,
        saved_iteration_source: None,
        total_sinks: problem
            .fanout_info
            .values()
            .map(|info| info.sinks.len())
            .sum(),
    })
}

fn select_tree_candidate(
    source: Polarity,
    sinks: &[Polarity],
    candidates: &[FanoutTreeCandidate],
    eligible_algorithms: &BTreeSet<&str>,
) -> Result<FanoutTreeCandidate, FanoutOptError> {
    candidates
        .iter()
        .filter(|candidate| eligible_algorithms.contains(candidate.algorithm.as_str()))
        .filter(|candidate| candidate.source == source)
        .filter(|candidate| same_sink_set(&candidate.sinks, sinks))
        .min_by(|left, right| compare_tree_candidates(left, right))
        .cloned()
        .ok_or(FanoutOptError::NoCandidate { source })
}

fn compare_tree_candidates(
    left: &&FanoutTreeCandidate,
    right: &&FanoutTreeCandidate,
) -> std::cmp::Ordering {
    if is_better_cost(left.cost, right.cost) {
        std::cmp::Ordering::Less
    } else if is_better_cost(right.cost, left.cost) {
        std::cmp::Ordering::Greater
    } else {
        left.buffer_count
            .cmp(&right.buffer_count)
            .then_with(|| left.depth.cmp(&right.depth))
            .then_with(|| left.edge_count.cmp(&right.edge_count))
            .then_with(|| left.algorithm.cmp(&right.algorithm))
    }
}

fn validate_algorithms(
    algorithms: &[FanoutAlgorithm],
    limits: FanoutPlanningLimits,
) -> Result<(), FanoutOptError> {
    if algorithms.len() > limits.max_algorithms {
        return Err(FanoutOptError::LimitExceeded {
            limit: "fanout algorithms",
            max: limits.max_algorithms,
        });
    }

    let mut seen = BTreeSet::new();
    for algorithm in algorithms {
        if algorithm.name.is_empty() {
            return Err(FanoutOptError::EmptyName { kind: "algorithm" });
        }
        if !seen.insert(algorithm.name.as_str()) {
            return Err(FanoutOptError::DuplicateAlgorithm {
                name: algorithm.name.clone(),
            });
        }
        if algorithm.properties.len() > limits.max_properties_per_algorithm {
            return Err(FanoutOptError::LimitExceeded {
                limit: "fanout algorithm properties",
                max: limits.max_properties_per_algorithm,
            });
        }
        for property in &algorithm.properties {
            if property.name.is_empty() {
                return Err(FanoutOptError::EmptyName { kind: "property" });
            }
        }
    }

    Ok(())
}

fn validate_candidates(
    candidates: &[FanoutTreeCandidate],
    limits: FanoutPlanningLimits,
) -> Result<(), FanoutOptError> {
    if candidates.len() > limits.max_candidate_trees_per_source {
        return Err(FanoutOptError::LimitExceeded {
            limit: "fanout tree candidates",
            max: limits.max_candidate_trees_per_source,
        });
    }
    for candidate in candidates {
        if candidate.algorithm.is_empty() {
            return Err(FanoutOptError::EmptyName {
                kind: "candidate algorithm",
            });
        }
        validate_cost(&candidate.algorithm, candidate.cost)?;
    }

    Ok(())
}

fn validate_options(options: FanoutPlanningOptions) -> Result<(), FanoutOptError> {
    validate_non_negative_finite("fanout options", "penalty_factor", options.penalty_factor)?;
    if options.penalty_factor == 0.0 {
        return Err(FanoutOptError::InvalidMetric {
            context: "fanout options".to_string(),
            metric: "penalty_factor",
            value: options.penalty_factor,
        });
    }
    Ok(())
}

fn validate_cost(context: &str, cost: FanoutCost) -> Result<(), FanoutOptError> {
    validate_delay(context, cost.slack, false)?;
    validate_non_negative_finite(context, "area", cost.area)
}

fn validate_delay(
    context: &str,
    value: DelayTime,
    allow_infinity: bool,
) -> Result<(), FanoutOptError> {
    let valid = if allow_infinity {
        !value.rise.is_nan() && !value.fall.is_nan()
    } else {
        value.rise.is_finite() && value.fall.is_finite()
    };
    if !valid {
        return Err(FanoutOptError::InvalidMetric {
            context: context.to_string(),
            metric: "delay",
            value: value.rise.max(value.fall),
        });
    }

    Ok(())
}

fn validate_non_negative_finite(
    context: &str,
    metric: &'static str,
    value: f64,
) -> Result<(), FanoutOptError> {
    if !value.is_finite() || value < 0.0 {
        return Err(FanoutOptError::InvalidMetric {
            context: context.to_string(),
            metric,
            value,
        });
    }

    Ok(())
}

fn extract_links(
    network: &VirtualMappedNetwork,
    source: NodeId,
    exception: Option<NodeId>,
) -> Result<Vec<FanoutSink>, FanoutOptError> {
    let node = network
        .node(source)
        .ok_or(FanoutOptError::MissingNode(source))?;

    Ok(node
        .gate_links()
        .filter(|link| Some(link.node) != exception)
        .copied()
        .map(FanoutSink::from)
        .collect())
}

fn is_external_source(node: &super::virtual_net::VirtualMappedNode) -> bool {
    node.kind == NodeKind::PrimaryInput
        || (node.gate.is_some() && node.save_binding.len() > 1)
        || matches!(node.gate, Some(super::virtual_net::GateKind::Library(_)))
}

fn mapped_gate_area(node: &super::virtual_net::VirtualMappedNode) -> f64 {
    if node.gate.is_some() { 1.0 } else { 0.0 }
}

fn fanout_count(problem: &FanoutProblem, polarity: Polarity) -> usize {
    problem
        .fanout_info
        .get(&polarity)
        .map_or(0, |info| info.sinks.len())
}

fn compare_sinks(left: &FanoutSink, right: &FanoutSink) -> std::cmp::Ordering {
    min_required(left.required)
        .total_cmp(&min_required(right.required))
        .then_with(|| right.load.total_cmp(&left.load))
        .then_with(|| left.pin.cmp(&right.pin))
        .then_with(|| left.node.cmp(&right.node))
}

fn same_sink_set(left: &[Polarity], right: &[Polarity]) -> bool {
    left.iter().copied().collect::<BTreeSet<_>>() == right.iter().copied().collect::<BTreeSet<_>>()
}

fn add_cost(left: FanoutCost, right: FanoutCost) -> FanoutCost {
    FanoutCost {
        slack: min_delay(left.slack, right.slack),
        area: left.area + right.area,
    }
}

fn min_delay(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise.min(right.rise), left.fall.min(right.fall))
}

fn min_required(required: DelayTime) -> f64 {
    required.rise.min(required.fall)
}

fn normalized_min_slack(slack: DelayTime) -> f64 {
    normalize_zero(min_required(slack))
}

fn normalize_zero(value: f64) -> f64 {
    if value.abs() <= 1.0e-9 { 0.0 } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::virtual_net::{GateKind, SourceRef};

    fn cost(slack: f64, area: f64) -> FanoutCost {
        FanoutCost::new(DelayTime::new(slack, slack), area)
    }

    fn candidate(
        algorithm: &str,
        source: Polarity,
        sinks: Vec<Polarity>,
        slack: f64,
        area: f64,
    ) -> FanoutTreeCandidate {
        FanoutTreeCandidate {
            algorithm: algorithm.to_string(),
            source,
            sinks,
            cost: cost(slack, area),
            buffer_count: 0,
            depth: 0,
            edge_count: 0,
        }
    }

    fn test_nodes(count: usize) -> Vec<NodeId> {
        let mut network = VirtualMappedNetwork::new();
        (0..count)
            .map(|index| network.add_primary_input(format!("n{index}")))
            .collect()
    }

    #[test]
    fn configures_algorithm_flags_like_command_line_entries() {
        let algorithms = vec![
            FanoutAlgorithm {
                name: "noalg".to_string(),
                enabled: true,
                min_size: 0,
                peephole: false,
                properties: vec![],
            },
            FanoutAlgorithm {
                name: "bottom_up".to_string(),
                enabled: false,
                min_size: 0,
                peephole: false,
                properties: vec![FanoutAlgorithmProperty {
                    name: "size".to_string(),
                    value: 0,
                }],
            },
        ];

        let mut configured =
            configure_algorithms(&algorithms, &["bottom_up"], DEFAULT_LIMITS).unwrap();
        let updated = set_algorithm_property(&mut configured, "bottom_up", "size", 3).unwrap();

        assert_eq!(updated, 1);
        assert!(configured[0].enabled);
        assert!(configured[1].enabled);
        assert_eq!(configured[1].min_size, 3);
    }

    #[test]
    fn preprocesses_links_by_required_time_load_and_pin() {
        let nodes = test_nodes(3);
        let a = nodes[1];
        let b = nodes[2];
        let info = preprocess_fanout_info(
            Polarity::X,
            [
                FanoutSink {
                    node: a,
                    pin: 1,
                    load: 1.0,
                    required: DelayTime::new(4.0, 4.0),
                },
                FanoutSink {
                    node: b,
                    pin: 0,
                    load: 3.0,
                    required: DelayTime::new(4.0, 4.0),
                },
                FanoutSink {
                    node: a,
                    pin: 0,
                    load: 2.0,
                    required: DelayTime::new(2.0, 2.0),
                },
            ],
            DEFAULT_LIMITS,
        )
        .unwrap();

        assert_eq!(
            info.sinks.iter().map(|sink| sink.pin).collect::<Vec<_>>(),
            vec![0, 0, 1]
        );
        assert_eq!(info.cumulative_load, vec![0.0, 2.0, 5.0, 6.0]);
        assert_eq!(
            info.minimum_required,
            vec![
                DelayTime::new(2.0, 2.0),
                DelayTime::new(4.0, 4.0),
                DelayTime::new(4.0, 4.0),
            ]
        );
    }

    #[test]
    fn compares_costs_with_negative_slack_before_area() {
        assert!(is_better_cost(cost(-1.0, 10.0), cost(-2.0, 1.0)));
        assert!(!is_better_cost(cost(-1.0, 1.0), cost(0.0, 100.0)));
        assert!(is_better_cost(cost(0.0, 1.0), cost(1.0, 2.0)));
    }

    #[test]
    fn chooses_best_single_source_plan_and_removed_source() {
        let nodes = test_nodes(4);
        let root = nodes[0];
        let mut sources = BTreeMap::new();
        sources.insert(
            Polarity::X,
            FanoutSource {
                node: root,
                polarity: Polarity::X,
                is_external: true,
                existing_gate_area: 1.0,
            },
        );
        sources.insert(
            Polarity::Y,
            FanoutSource {
                node: nodes[1],
                polarity: Polarity::Y,
                is_external: true,
                existing_gate_area: 1.0,
            },
        );
        let problem = FanoutProblem {
            root,
            sources,
            fanout_info: BTreeMap::from([
                (
                    Polarity::X,
                    FanoutInfo {
                        polarity: Polarity::X,
                        sinks: vec![FanoutSink {
                            node: nodes[2],
                            pin: 0,
                            load: 1.0,
                            required: DelayTime::new(3.0, 3.0),
                        }],
                        cumulative_load: vec![0.0, 1.0],
                        minimum_required: vec![DelayTime::new(3.0, 3.0)],
                        total_load: 1.0,
                    },
                ),
                (
                    Polarity::Y,
                    FanoutInfo {
                        polarity: Polarity::Y,
                        sinks: vec![FanoutSink {
                            node: nodes[3],
                            pin: 0,
                            load: 1.0,
                            required: DelayTime::new(3.0, 3.0),
                        }],
                        cumulative_load: vec![0.0, 1.0],
                        minimum_required: vec![DelayTime::new(3.0, 3.0)],
                        total_load: 1.0,
                    },
                ),
            ]),
        };
        let algorithms = vec![FanoutAlgorithm {
            name: "noalg".to_string(),
            enabled: true,
            min_size: 0,
            peephole: false,
            properties: vec![],
        }];
        let candidates = vec![
            candidate(
                "noalg",
                Polarity::X,
                vec![Polarity::X, Polarity::Y],
                1.0,
                5.0,
            ),
            candidate(
                "noalg",
                Polarity::Y,
                vec![Polarity::X, Polarity::Y],
                1.0,
                3.0,
            ),
            candidate("noalg", Polarity::X, vec![Polarity::X], 1.0, 1.0),
            candidate("noalg", Polarity::Y, vec![Polarity::Y], 1.0, 1.0),
            candidate("noalg", Polarity::X, vec![Polarity::Y], 1.0, 1.0),
            candidate("noalg", Polarity::Y, vec![Polarity::X], 1.0, 2.0),
        ];

        let plan = plan_fanout_optimization(
            &problem,
            &algorithms,
            &candidates,
            FanoutPlanningOptions::default(),
        )
        .unwrap();

        assert_eq!(plan.assignment, SourceAssignment::YAlone);
        assert_eq!(plan.removed_sources, vec![Polarity::X]);
        assert_eq!(plan.cost.area, 3.0);
    }

    #[test]
    fn extracts_virtual_problem_from_owned_gate_links() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let n1 = network.add_gate(
            "n1",
            GateKind::And,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        let n2 = network.add_gate("n2", GateKind::Inverter, vec![SourceRef::Node(n1)]);
        let n3 = network.add_gate("n3", GateKind::Inverter, vec![SourceRef::Node(n1)]);
        network
            .add_primary_output("f", SourceRef::Node(n2))
            .unwrap();
        network
            .add_primary_output("g", SourceRef::Node(n3))
            .unwrap();
        network.setup_gate_links().unwrap();

        let problem =
            extract_virtual_fanout_problem(&network, n1, None, FanoutPlanningOptions::default())
                .unwrap();

        assert_eq!(problem.root, n1);
        assert!(problem.sources.contains_key(&Polarity::X));
        assert_eq!(problem.fanout_info[&Polarity::X].sinks.len(), 2);
    }
}
