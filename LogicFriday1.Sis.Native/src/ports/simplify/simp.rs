//! Native Rust port scaffold for `LogicSynthesis/sis/simplify/simp.c`.
//!
//! The C file is the single-node simplify driver: it selects a don't-care
//! source, filters it, maps the requested SIS simplify method to a node
//! minimizer mode, asks `node_simplify` for a candidate, and accepts the
//! candidate only when the selected cost metric improves. The actual SIS
//! `node_t`, BDD, sparse-matrix, and network mutation APIs are still separate
//! porting units, so this module ports the deterministic policy logic onto
//! native Rust records and reports full node-bound execution as blocked.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimMethod {
    SimpComp,
    Espresso,
    Exact,
    ExactLits,
    DcSimp,
    NoComp,
    SNoComp,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeSimType {
    SimpComp,
    Espresso,
    Exact,
    ExactLits,
    DcSimp,
    NoComp,
    SNoComp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimAccept {
    FactoredLiterals,
    Cubes,
    SopLiterals,
    Always,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimDcType {
    None,
    Fanin,
    Fanout,
    Inout,
    All,
    SubFanin,
    Level,
    X,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimFilter {
    None,
    Exact,
    DisjointSupport,
    Size,
    FirstDistance,
    SecondDistance,
    Level,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimFlag {
    pub method: SimMethod,
    pub accept: SimAccept,
    pub dctype: SimDcType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeMetrics {
    pub factor_literals: usize,
    pub sop_literals: usize,
    pub cubes: usize,
    pub fanins: usize,
}

impl NodeMetrics {
    pub const fn new(
        factor_literals: usize,
        sop_literals: usize,
        cubes: usize,
        fanins: usize,
    ) -> Self {
        Self {
            factor_literals,
            sop_literals,
            cubes,
            fanins,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceptMetric {
    FactoredLiterals,
    SopLiterals,
    Cubes,
    Forced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceptDecision {
    Replace {
        metric: AcceptMetric,
        old_cost: usize,
        new_cost: usize,
    },
    Discard {
        metric: AcceptMetric,
        old_cost: usize,
        new_cost: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DcSource {
    ConstantZero,
    TransitiveFanin {
        fanin_level: i32,
        fanin_fanout_level: i32,
    },
    Fanout,
    Inout {
        fanin_level: i32,
        fanin_fanout_level: i32,
    },
    SubFanin,
    All,
    Level,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimpDcParameters {
    pub fanin_level: i32,
    pub fanin_fanout_level: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimplifyAction {
    TraceOriginalMetrics(NodeMetrics),
    GenerateDontCare(DcSource),
    FilterDontCare(SimFilter),
    SimplifyWith(NodeSimType),
    AcceptCandidate(SimAccept),
    StoreSimFlag(SimFlag),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplifyNodePlan {
    pub actions: Vec<SimplifyAction>,
    pub node_sim_type: NodeSimType,
    pub sim_flag: SimFlag,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeSimplifyNode<N> {
    pub value: N,
    pub metrics: NodeMetrics,
    pub sim_flag: Option<SimFlag>,
}

impl<N> NativeSimplifyNode<N> {
    pub const fn new(value: N, metrics: NodeMetrics) -> Self {
        Self {
            value,
            metrics,
            sim_flag: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeDontCare<D> {
    pub value: D,
    pub metrics: Option<NodeMetrics>,
}

impl<D> NativeDontCare<D> {
    pub const fn new(value: D, metrics: Option<NodeMetrics>) -> Self {
        Self { value, metrics }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeSimplifyCandidate<N> {
    pub value: N,
    pub metrics: NodeMetrics,
}

impl<N> NativeSimplifyCandidate<N> {
    pub const fn new(value: N, metrics: NodeMetrics) -> Self {
        Self { value, metrics }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimplifyNodeOptions {
    pub method: SimMethod,
    pub dctype: SimDcType,
    pub accept: SimAccept,
    pub filter: SimFilter,
    pub parameters: SimpDcParameters,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplifyNodeOutcome<D> {
    pub dc_source: DcSource,
    pub before_filter_metrics: Option<NodeMetrics>,
    pub after_filter_metrics: Option<NodeMetrics>,
    pub node_sim_type: NodeSimType,
    pub decision: AcceptDecision,
    pub sim_flag: SimFlag,
    pub replaced: bool,
    pub filtered_dc: D,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CspfLocalDcBase {
    Level,
    SubFanin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DcStats {
    pub fanins: usize,
    pub cubes: usize,
    pub literals: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CspfAction {
    GenerateBaseDontCare(CspfLocalDcBase),
    BuildCareSetFromTransitiveFanoutPos,
    OrWithComplementedCareSet,
    ObservabilitySatFilter { variable_allowance: usize },
    SimplifyWith(NodeSimType),
    AcceptCandidate(SimAccept),
    StoreSimFlag(SimFlag),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CspfSimplifyPlan {
    pub actions: Vec<CspfAction>,
    pub node_sim_type: NodeSimType,
    pub sim_flag: SimFlag,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CspfNodeOptions {
    pub method: SimMethod,
    pub dctype: SimDcType,
    pub accept: SimAccept,
    pub filter: SimFilter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CspfLocalDc<D> {
    pub value: D,
    pub metrics: DcStats,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CspfSimplifyOutcome<D> {
    pub base_dc: CspfLocalDcBase,
    pub node_sim_type: NodeSimType,
    pub filter_allowances: Vec<usize>,
    pub decision: AcceptDecision,
    pub sim_flag: SimFlag,
    pub replaced: bool,
    pub filtered_dc: D,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimplifyError {
    UnknownMethod,
    UnknownAcceptCriteria,
    UnknownDontCareType,
    MissingSisPorts { operation: &'static str },
}

impl fmt::Display for SimplifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMethod => write!(f, "unknown SIS simplification method"),
            Self::UnknownAcceptCriteria => {
                write!(f, "unknown SIS simplification acceptance criteria")
            }
            Self::UnknownDontCareType => write!(f, "unknown SIS simplify don't-care type"),
            Self::MissingSisPorts { operation } => write!(
                f,
                "{operation} requires native Rust SIS ports that are not available yet"
            ),
        }
    }
}

impl Error for SimplifyError {}

pub fn map_method(method: SimMethod) -> Result<NodeSimType, SimplifyError> {
    match method {
        SimMethod::SimpComp => Ok(NodeSimType::SimpComp),
        SimMethod::Espresso => Ok(NodeSimType::Espresso),
        SimMethod::Exact => Ok(NodeSimType::Exact),
        SimMethod::ExactLits => Ok(NodeSimType::ExactLits),
        SimMethod::DcSimp => Ok(NodeSimType::DcSimp),
        SimMethod::NoComp => Ok(NodeSimType::NoComp),
        SimMethod::SNoComp => Ok(NodeSimType::SNoComp),
        SimMethod::Unknown => Err(SimplifyError::UnknownMethod),
    }
}

pub fn dc_source(
    dctype: SimDcType,
    parameters: SimpDcParameters,
) -> Result<DcSource, SimplifyError> {
    match dctype {
        SimDcType::None => Ok(DcSource::ConstantZero),
        SimDcType::Fanin => Ok(DcSource::TransitiveFanin {
            fanin_level: parameters.fanin_level,
            fanin_fanout_level: parameters.fanin_fanout_level,
        }),
        SimDcType::Fanout => Ok(DcSource::Fanout),
        SimDcType::Inout => Ok(DcSource::Inout {
            fanin_level: parameters.fanin_level,
            fanin_fanout_level: parameters.fanin_fanout_level,
        }),
        SimDcType::All => Ok(DcSource::All),
        SimDcType::SubFanin => Ok(DcSource::SubFanin),
        SimDcType::Level => Ok(DcSource::Level),
        SimDcType::X | SimDcType::Unknown => Err(SimplifyError::UnknownDontCareType),
    }
}

pub fn accept_candidate(
    accept: SimAccept,
    old: NodeMetrics,
    new: NodeMetrics,
) -> Result<AcceptDecision, SimplifyError> {
    let (metric, old_cost, new_cost, force_accept) = match accept {
        SimAccept::FactoredLiterals => (
            AcceptMetric::FactoredLiterals,
            old.factor_literals,
            new.factor_literals,
            false,
        ),
        SimAccept::SopLiterals => (
            AcceptMetric::SopLiterals,
            old.sop_literals,
            new.sop_literals,
            false,
        ),
        SimAccept::Cubes => (AcceptMetric::Cubes, old.cubes, new.cubes, false),
        SimAccept::Always => (
            AcceptMetric::Forced,
            old.factor_literals,
            new.factor_literals,
            true,
        ),
        SimAccept::Unknown => return Err(SimplifyError::UnknownAcceptCriteria),
    };

    let (metric, old_cost, new_cost) =
        if accept == SimAccept::FactoredLiterals && old_cost == new_cost {
            (
                AcceptMetric::SopLiterals,
                old.sop_literals,
                new.sop_literals,
            )
        } else {
            (metric, old_cost, new_cost)
        };

    if force_accept || new_cost < old_cost {
        Ok(AcceptDecision::Replace {
            metric,
            old_cost,
            new_cost,
        })
    } else {
        Ok(AcceptDecision::Discard {
            metric,
            old_cost,
            new_cost,
        })
    }
}

pub fn plan_simplify_node(
    method: SimMethod,
    dctype: SimDcType,
    accept: SimAccept,
    filter: SimFilter,
    parameters: SimpDcParameters,
    original_metrics: NodeMetrics,
) -> Result<SimplifyNodePlan, SimplifyError> {
    let node_sim_type = map_method(method)?;
    let dc_source = dc_source(dctype, parameters)?;
    if accept == SimAccept::Unknown {
        return Err(SimplifyError::UnknownAcceptCriteria);
    }

    let sim_flag = SimFlag {
        method,
        accept,
        dctype,
    };

    Ok(SimplifyNodePlan {
        actions: vec![
            SimplifyAction::TraceOriginalMetrics(original_metrics),
            SimplifyAction::GenerateDontCare(dc_source),
            SimplifyAction::FilterDontCare(filter),
            SimplifyAction::SimplifyWith(node_sim_type),
            SimplifyAction::AcceptCandidate(accept),
            SimplifyAction::StoreSimFlag(sim_flag),
        ],
        node_sim_type,
        sim_flag,
    })
}

pub fn execute_simplify_node<N, D>(
    node: &mut NativeSimplifyNode<N>,
    options: SimplifyNodeOptions,
    mut generate_dc: impl FnMut(DcSource) -> Result<NativeDontCare<D>, SimplifyError>,
    mut filter_dc: impl FnMut(
        &N,
        NativeDontCare<D>,
        SimFilter,
    ) -> Result<NativeDontCare<D>, SimplifyError>,
    mut simplify: impl FnMut(&N, &D, NodeSimType) -> Result<NativeSimplifyCandidate<N>, SimplifyError>,
) -> Result<SimplifyNodeOutcome<D>, SimplifyError> {
    let node_sim_type = map_method(options.method)?;
    let dc_source = dc_source(options.dctype, options.parameters)?;
    if options.accept == SimAccept::Unknown {
        return Err(SimplifyError::UnknownAcceptCriteria);
    }

    let sim_flag = SimFlag {
        method: options.method,
        accept: options.accept,
        dctype: options.dctype,
    };

    let original_metrics = node.metrics;
    let dc_before_filter = generate_dc(dc_source)?;
    let before_filter_metrics = dc_before_filter.metrics;
    let dc_after_filter = filter_dc(&node.value, dc_before_filter, options.filter)?;
    let after_filter_metrics = dc_after_filter.metrics;
    let candidate = simplify(&node.value, &dc_after_filter.value, node_sim_type)?;
    let decision = accept_candidate(options.accept, original_metrics, candidate.metrics)?;
    let replaced = matches!(decision, AcceptDecision::Replace { .. });
    if replaced {
        node.value = candidate.value;
        node.metrics = candidate.metrics;
    }
    node.sim_flag = Some(sim_flag);

    Ok(SimplifyNodeOutcome {
        dc_source,
        before_filter_metrics,
        after_filter_metrics,
        node_sim_type,
        decision,
        sim_flag,
        replaced,
        filtered_dc: dc_after_filter.value,
    })
}

pub fn cspf_base_dc(filter: SimFilter) -> CspfLocalDcBase {
    if filter == SimFilter::Level {
        CspfLocalDcBase::Level
    } else {
        CspfLocalDcBase::SubFanin
    }
}

pub fn cspf_obssat_filter_allowances(
    filter: SimFilter,
    stats_after_passes: &[DcStats],
) -> Vec<usize> {
    if filter == SimFilter::None {
        return Vec::new();
    }

    let mut allowances = vec![2];
    for (index, stats) in stats_after_passes.iter().take(2).enumerate() {
        if stats.cubes > 100 && stats.fanins > 20 {
            allowances.push(1 - index);
        } else {
            break;
        }
    }
    allowances
}

pub fn plan_simplify_cspf_node(
    method: SimMethod,
    dctype: SimDcType,
    accept: SimAccept,
    filter: SimFilter,
    stats_after_filter_passes: &[DcStats],
) -> Result<CspfSimplifyPlan, SimplifyError> {
    if matches!(dctype, SimDcType::Unknown | SimDcType::X) {
        return Err(SimplifyError::UnknownDontCareType);
    }
    if accept == SimAccept::Unknown {
        return Err(SimplifyError::UnknownAcceptCriteria);
    }

    let node_sim_type = map_method(method)?;
    let sim_flag = SimFlag {
        method,
        accept,
        dctype,
    };

    let mut actions = vec![
        CspfAction::GenerateBaseDontCare(cspf_base_dc(filter)),
        CspfAction::BuildCareSetFromTransitiveFanoutPos,
        CspfAction::OrWithComplementedCareSet,
    ];
    actions.extend(
        cspf_obssat_filter_allowances(filter, stats_after_filter_passes)
            .into_iter()
            .map(|variable_allowance| CspfAction::ObservabilitySatFilter { variable_allowance }),
    );
    actions.extend([
        CspfAction::SimplifyWith(node_sim_type),
        CspfAction::AcceptCandidate(accept),
        CspfAction::StoreSimFlag(sim_flag),
    ]);

    Ok(CspfSimplifyPlan {
        actions,
        node_sim_type,
        sim_flag,
    })
}

pub fn execute_simplify_cspf_node<N, D>(
    node: &mut NativeSimplifyNode<N>,
    options: CspfNodeOptions,
    mut generate_local_dc: impl FnMut(CspfLocalDcBase) -> Result<CspfLocalDc<D>, SimplifyError>,
    mut merge_care_set: impl FnMut(&N, D) -> Result<CspfLocalDc<D>, SimplifyError>,
    mut observability_sat_filter: impl FnMut(D, &N, usize) -> Result<CspfLocalDc<D>, SimplifyError>,
    mut simplify: impl FnMut(&N, &D, NodeSimType) -> Result<NativeSimplifyCandidate<N>, SimplifyError>,
) -> Result<CspfSimplifyOutcome<D>, SimplifyError> {
    if matches!(options.dctype, SimDcType::Unknown | SimDcType::X) {
        return Err(SimplifyError::UnknownDontCareType);
    }
    if options.accept == SimAccept::Unknown {
        return Err(SimplifyError::UnknownAcceptCriteria);
    }

    let node_sim_type = map_method(options.method)?;
    let sim_flag = SimFlag {
        method: options.method,
        accept: options.accept,
        dctype: options.dctype,
    };
    let base_dc = cspf_base_dc(options.filter);
    let original_metrics = node.metrics;

    let local_dc = generate_local_dc(base_dc)?;
    let mut dc = merge_care_set(&node.value, local_dc.value)?;
    let mut filter_allowances = Vec::new();
    if options.filter != SimFilter::None {
        for variable_allowance in [2, 1, 0] {
            dc = observability_sat_filter(dc.value, &node.value, variable_allowance)?;
            filter_allowances.push(variable_allowance);
            if dc.metrics.cubes <= 100 || dc.metrics.fanins <= 20 {
                break;
            }
        }
    }

    let candidate = simplify(&node.value, &dc.value, node_sim_type)?;
    let decision = accept_candidate(options.accept, original_metrics, candidate.metrics)?;
    let replaced = matches!(decision, AcceptDecision::Replace { .. });
    if replaced {
        node.value = candidate.value;
        node.metrics = candidate.metrics;
    }
    node.sim_flag = Some(sim_flag);

    Ok(CspfSimplifyOutcome {
        base_dc,
        node_sim_type,
        filter_allowances,
        decision,
        sim_flag,
        replaced,
        filtered_dc: dc.value,
    })
}

pub fn simplify_node_native() -> Result<(), SimplifyError> {
    Err(SimplifyError::MissingSisPorts {
        operation: "simplify_node",
    })
}

pub fn simplify_cspf_node_native() -> Result<(), SimplifyError> {
    Err(SimplifyError::MissingSisPorts {
        operation: "simplify_cspf_node",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn metrics(factor_literals: usize, sop_literals: usize, cubes: usize) -> NodeMetrics {
        NodeMetrics::new(factor_literals, sop_literals, cubes, 0)
    }

    #[test]
    fn maps_simplify_methods_to_node_modes() {
        assert_eq!(map_method(SimMethod::SimpComp), Ok(NodeSimType::SimpComp));
        assert_eq!(map_method(SimMethod::Espresso), Ok(NodeSimType::Espresso));
        assert_eq!(map_method(SimMethod::Exact), Ok(NodeSimType::Exact));
        assert_eq!(map_method(SimMethod::ExactLits), Ok(NodeSimType::ExactLits));
        assert_eq!(map_method(SimMethod::DcSimp), Ok(NodeSimType::DcSimp));
        assert_eq!(map_method(SimMethod::NoComp), Ok(NodeSimType::NoComp));
        assert_eq!(map_method(SimMethod::SNoComp), Ok(NodeSimType::SNoComp));
        assert_eq!(
            map_method(SimMethod::Unknown),
            Err(SimplifyError::UnknownMethod)
        );
    }

    #[test]
    fn maps_dont_care_types_to_generation_sources() {
        let parameters = SimpDcParameters {
            fanin_level: 2,
            fanin_fanout_level: 3,
        };

        assert_eq!(
            dc_source(SimDcType::None, parameters),
            Ok(DcSource::ConstantZero)
        );
        assert_eq!(
            dc_source(SimDcType::Fanin, parameters),
            Ok(DcSource::TransitiveFanin {
                fanin_level: 2,
                fanin_fanout_level: 3,
            })
        );
        assert_eq!(
            dc_source(SimDcType::Fanout, parameters),
            Ok(DcSource::Fanout)
        );
        assert_eq!(
            dc_source(SimDcType::Inout, parameters),
            Ok(DcSource::Inout {
                fanin_level: 2,
                fanin_fanout_level: 3,
            })
        );
        assert_eq!(
            dc_source(SimDcType::SubFanin, parameters),
            Ok(DcSource::SubFanin)
        );
        assert_eq!(dc_source(SimDcType::All, parameters), Ok(DcSource::All));
        assert_eq!(dc_source(SimDcType::Level, parameters), Ok(DcSource::Level));
        assert_eq!(
            dc_source(SimDcType::Unknown, parameters),
            Err(SimplifyError::UnknownDontCareType)
        );
    }

    #[test]
    fn factored_literal_acceptance_falls_back_to_sop_literals_on_tie() {
        assert_eq!(
            accept_candidate(
                SimAccept::FactoredLiterals,
                metrics(4, 8, 3),
                metrics(4, 7, 5)
            ),
            Ok(AcceptDecision::Replace {
                metric: AcceptMetric::SopLiterals,
                old_cost: 8,
                new_cost: 7,
            })
        );
    }

    #[test]
    fn acceptance_discards_non_improving_candidates() {
        assert_eq!(
            accept_candidate(SimAccept::Cubes, metrics(4, 8, 3), metrics(2, 2, 3)),
            Ok(AcceptDecision::Discard {
                metric: AcceptMetric::Cubes,
                old_cost: 3,
                new_cost: 3,
            })
        );
    }

    #[test]
    fn always_accepts_even_when_candidate_is_more_expensive() {
        assert_eq!(
            accept_candidate(SimAccept::Always, metrics(4, 8, 3), metrics(7, 9, 5)),
            Ok(AcceptDecision::Replace {
                metric: AcceptMetric::Forced,
                old_cost: 4,
                new_cost: 7,
            })
        );
    }

    #[test]
    fn simplify_plan_matches_c_driver_order() {
        let plan = plan_simplify_node(
            SimMethod::ExactLits,
            SimDcType::Fanin,
            SimAccept::SopLiterals,
            SimFilter::Size,
            SimpDcParameters {
                fanin_level: 1,
                fanin_fanout_level: 2,
            },
            NodeMetrics::new(5, 6, 2, 3),
        )
        .unwrap();

        assert_eq!(plan.node_sim_type, NodeSimType::ExactLits);
        assert_eq!(
            plan.actions,
            vec![
                SimplifyAction::TraceOriginalMetrics(NodeMetrics::new(5, 6, 2, 3)),
                SimplifyAction::GenerateDontCare(DcSource::TransitiveFanin {
                    fanin_level: 1,
                    fanin_fanout_level: 2,
                }),
                SimplifyAction::FilterDontCare(SimFilter::Size),
                SimplifyAction::SimplifyWith(NodeSimType::ExactLits),
                SimplifyAction::AcceptCandidate(SimAccept::SopLiterals),
                SimplifyAction::StoreSimFlag(SimFlag {
                    method: SimMethod::ExactLits,
                    accept: SimAccept::SopLiterals,
                    dctype: SimDcType::Fanin,
                }),
            ]
        );
    }

    #[test]
    fn execute_simplify_node_replaces_improving_candidate_and_stores_flag() {
        let mut node = NativeSimplifyNode::new("old", metrics(8, 10, 4));
        let calls = RefCell::new(Vec::new());

        let outcome = execute_simplify_node(
            &mut node,
            SimplifyNodeOptions {
                method: SimMethod::Exact,
                dctype: SimDcType::Fanin,
                accept: SimAccept::FactoredLiterals,
                filter: SimFilter::Exact,
                parameters: SimpDcParameters {
                    fanin_level: 2,
                    fanin_fanout_level: 3,
                },
            },
            |source| {
                calls.borrow_mut().push(format!("dc:{source:?}"));
                Ok(NativeDontCare::new("raw-dc", Some(metrics(0, 12, 5))))
            },
            |node_value, dc, filter| {
                calls
                    .borrow_mut()
                    .push(format!("filter:{node_value}:{filter:?}:{}", dc.value));
                Ok(NativeDontCare::new("filtered-dc", Some(metrics(0, 4, 2))))
            },
            |node_value, dc, sim_type| {
                calls
                    .borrow_mut()
                    .push(format!("simplify:{node_value}:{dc}:{sim_type:?}"));
                Ok(NativeSimplifyCandidate::new("new", metrics(6, 9, 3)))
            },
        )
        .unwrap();

        assert_eq!(node.value, "new");
        assert_eq!(node.metrics, metrics(6, 9, 3));
        assert_eq!(
            node.sim_flag,
            Some(SimFlag {
                method: SimMethod::Exact,
                accept: SimAccept::FactoredLiterals,
                dctype: SimDcType::Fanin,
            })
        );
        assert_eq!(outcome.filtered_dc, "filtered-dc");
        assert_eq!(outcome.before_filter_metrics, Some(metrics(0, 12, 5)));
        assert_eq!(outcome.after_filter_metrics, Some(metrics(0, 4, 2)));
        assert_eq!(
            outcome.decision,
            AcceptDecision::Replace {
                metric: AcceptMetric::FactoredLiterals,
                old_cost: 8,
                new_cost: 6,
            }
        );
        assert_eq!(
            calls.into_inner(),
            vec![
                "dc:TransitiveFanin { fanin_level: 2, fanin_fanout_level: 3 }",
                "filter:old:Exact:raw-dc",
                "simplify:old:filtered-dc:Exact",
            ]
        );
    }

    #[test]
    fn execute_simplify_node_discards_non_improving_candidate() {
        let mut node = NativeSimplifyNode::new("old", metrics(8, 10, 4));

        let outcome = execute_simplify_node(
            &mut node,
            SimplifyNodeOptions {
                method: SimMethod::Espresso,
                dctype: SimDcType::None,
                accept: SimAccept::Cubes,
                filter: SimFilter::None,
                parameters: SimpDcParameters {
                    fanin_level: 0,
                    fanin_fanout_level: 0,
                },
            },
            |source| {
                assert_eq!(source, DcSource::ConstantZero);
                Ok(NativeDontCare::new("dc", None))
            },
            |_node_value, dc, filter| {
                assert_eq!(filter, SimFilter::None);
                Ok(dc)
            },
            |_node_value, dc, sim_type| {
                assert_eq!(*dc, "dc");
                assert_eq!(sim_type, NodeSimType::Espresso);
                Ok(NativeSimplifyCandidate::new("worse", metrics(3, 6, 4)))
            },
        )
        .unwrap();

        assert_eq!(node.value, "old");
        assert_eq!(node.metrics, metrics(8, 10, 4));
        assert!(!outcome.replaced);
        assert_eq!(
            outcome.decision,
            AcceptDecision::Discard {
                metric: AcceptMetric::Cubes,
                old_cost: 4,
                new_cost: 4,
            }
        );
    }

    #[test]
    fn cspf_uses_level_base_only_for_level_filter() {
        assert_eq!(cspf_base_dc(SimFilter::Level), CspfLocalDcBase::Level);
        assert_eq!(cspf_base_dc(SimFilter::Exact), CspfLocalDcBase::SubFanin);
        assert_eq!(cspf_base_dc(SimFilter::None), CspfLocalDcBase::SubFanin);
    }

    #[test]
    fn cspf_obssat_filter_repeats_while_dc_stays_large() {
        let large = DcStats {
            fanins: 21,
            cubes: 101,
            literals: 500,
        };
        let small = DcStats {
            fanins: 21,
            cubes: 100,
            literals: 400,
        };

        assert_eq!(
            cspf_obssat_filter_allowances(SimFilter::All, &[large, large]),
            vec![2, 1, 0]
        );
        assert_eq!(
            cspf_obssat_filter_allowances(SimFilter::All, &[small, large]),
            vec![2]
        );
        assert_eq!(
            cspf_obssat_filter_allowances(SimFilter::None, &[large, large]),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn cspf_plan_includes_local_dc_and_filter_schedule() {
        let plan = plan_simplify_cspf_node(
            SimMethod::NoComp,
            SimDcType::SubFanin,
            SimAccept::Always,
            SimFilter::Level,
            &[DcStats {
                fanins: 30,
                cubes: 150,
                literals: 700,
            }],
        )
        .unwrap();

        assert_eq!(plan.node_sim_type, NodeSimType::NoComp);
        assert_eq!(
            plan.actions,
            vec![
                CspfAction::GenerateBaseDontCare(CspfLocalDcBase::Level),
                CspfAction::BuildCareSetFromTransitiveFanoutPos,
                CspfAction::OrWithComplementedCareSet,
                CspfAction::ObservabilitySatFilter {
                    variable_allowance: 2,
                },
                CspfAction::ObservabilitySatFilter {
                    variable_allowance: 1,
                },
                CspfAction::SimplifyWith(NodeSimType::NoComp),
                CspfAction::AcceptCandidate(SimAccept::Always),
                CspfAction::StoreSimFlag(SimFlag {
                    method: SimMethod::NoComp,
                    accept: SimAccept::Always,
                    dctype: SimDcType::SubFanin,
                }),
            ]
        );
    }

    #[test]
    fn execute_simplify_cspf_node_runs_obssat_schedule_until_dc_is_small() {
        let mut node = NativeSimplifyNode::new("old", metrics(12, 18, 6));
        let mut allowances = Vec::new();

        let outcome = execute_simplify_cspf_node(
            &mut node,
            CspfNodeOptions {
                method: SimMethod::NoComp,
                dctype: SimDcType::SubFanin,
                accept: SimAccept::Always,
                filter: SimFilter::Level,
            },
            |base| {
                assert_eq!(base, CspfLocalDcBase::Level);
                Ok(CspfLocalDc {
                    value: "local",
                    metrics: DcStats {
                        fanins: 40,
                        cubes: 200,
                        literals: 900,
                    },
                })
            },
            |node_value, dc| {
                assert_eq!(node_value, &"old");
                assert_eq!(dc, "local");
                Ok(CspfLocalDc {
                    value: "merged",
                    metrics: DcStats {
                        fanins: 40,
                        cubes: 200,
                        literals: 900,
                    },
                })
            },
            |dc, node_value, allowance| {
                assert_eq!(node_value, &"old");
                allowances.push(allowance);
                let cubes = if allowance == 1 { 80 } else { 150 };
                Ok(CspfLocalDc {
                    value: dc,
                    metrics: DcStats {
                        fanins: 25,
                        cubes,
                        literals: 600,
                    },
                })
            },
            |node_value, dc, sim_type| {
                assert_eq!(node_value, &"old");
                assert_eq!(*dc, "merged");
                assert_eq!(sim_type, NodeSimType::NoComp);
                Ok(NativeSimplifyCandidate::new("new", metrics(20, 30, 10)))
            },
        )
        .unwrap();

        assert_eq!(node.value, "new");
        assert_eq!(outcome.base_dc, CspfLocalDcBase::Level);
        assert_eq!(outcome.filter_allowances, vec![2, 1]);
        assert_eq!(allowances, vec![2, 1]);
        assert!(outcome.replaced);
        assert_eq!(
            node.sim_flag,
            Some(SimFlag {
                method: SimMethod::NoComp,
                accept: SimAccept::Always,
                dctype: SimDcType::SubFanin,
            })
        );
    }

    #[test]
    fn execute_simplify_cspf_node_skips_filter_when_disabled() {
        let mut node = NativeSimplifyNode::new("old", metrics(4, 6, 2));
        let mut filter_calls = 0;

        let outcome = execute_simplify_cspf_node(
            &mut node,
            CspfNodeOptions {
                method: SimMethod::SNoComp,
                dctype: SimDcType::SubFanin,
                accept: SimAccept::SopLiterals,
                filter: SimFilter::None,
            },
            |base| {
                assert_eq!(base, CspfLocalDcBase::SubFanin);
                Ok(CspfLocalDc {
                    value: "local",
                    metrics: DcStats {
                        fanins: 99,
                        cubes: 999,
                        literals: 999,
                    },
                })
            },
            |_node_value, dc| {
                Ok(CspfLocalDc {
                    value: dc,
                    metrics: DcStats {
                        fanins: 99,
                        cubes: 999,
                        literals: 999,
                    },
                })
            },
            |dc, _node_value, _allowance| {
                filter_calls += 1;
                Ok(CspfLocalDc {
                    value: dc,
                    metrics: DcStats {
                        fanins: 0,
                        cubes: 0,
                        literals: 0,
                    },
                })
            },
            |_node_value, _dc, _sim_type| Ok(NativeSimplifyCandidate::new("new", metrics(3, 5, 2))),
        )
        .unwrap();

        assert_eq!(filter_calls, 0);
        assert_eq!(outcome.filter_allowances, Vec::<usize>::new());
        assert_eq!(node.value, "new");
    }

    #[test]
    fn node_bound_entries_report_missing_sis_ports() {
        assert_eq!(
            simplify_node_native(),
            Err(SimplifyError::MissingSisPorts {
                operation: "simplify_node",
            })
        );
        assert_eq!(
            simplify_cspf_node_native(),
            Err(SimplifyError::MissingSisPorts {
                operation: "simplify_cspf_node",
            })
        );
    }
}
