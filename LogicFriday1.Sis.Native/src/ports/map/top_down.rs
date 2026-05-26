//! Native top-down mapper selection helpers for `sis/map/top_down.c`.
//!
//! The original SIS implementation walks the mapped network from outputs toward
//! leaves and chooses load-sensitive tree matches. Full `network_t` traversal,
//! tree construction, matching, and replacement are not native in this crate
//! yet, so this module exposes the owned-data part of that behavior: deterministic
//! load-sensitive selection over already-produced tree candidates, plus a
//! virtual-network level planner for existing mapped virtual gates.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use super::two_level::PortDependency;
use super::virtual_net::{DelayTime, NodeId, NodeKind, VirtualMappedNetwork, VirtualNetworkError};

pub const REQUIRED_FULL_GRAPH_BEADS: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.271",
        source_file: "LogicSynthesis/sis/map/tree.c",
        note: "native mapper tree construction",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.272",
        source_file: "LogicSynthesis/sis/map/treemap.c",
        note: "native tree match enumeration",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.269",
        source_file: "LogicSynthesis/sis/map/replace.c",
        note: "native replacement of selected tree matches",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "native SIS graph fanin and fanout traversal",
    },
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TopDownOptions {
    pub load_weight: f64,
    pub delay_weight: f64,
    pub area_weight: f64,
    pub slack_weight: f64,
}

impl Default for TopDownOptions {
    fn default() -> Self {
        Self {
            load_weight: 1.0,
            delay_weight: 1.0,
            area_weight: 0.001,
            slack_weight: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappedTreeCandidate {
    pub root: String,
    pub candidate: String,
    pub level: usize,
    pub input_count: usize,
    pub area: f64,
    pub arrival: DelayTime,
    pub required: Option<DelayTime>,
    pub output_load: f64,
    pub inherited_load: f64,
}

impl MappedTreeCandidate {
    pub fn new(
        root: impl Into<String>,
        candidate: impl Into<String>,
        level: usize,
        area: f64,
        arrival: DelayTime,
    ) -> Self {
        Self {
            root: root.into(),
            candidate: candidate.into(),
            level,
            input_count: 0,
            area,
            arrival,
            required: None,
            output_load: 0.0,
            inherited_load: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopDownSelection {
    pub root: String,
    pub candidate: String,
    pub level: usize,
    pub input_count: usize,
    pub area: f64,
    pub arrival: DelayTime,
    pub required: Option<DelayTime>,
    pub output_load: f64,
    pub inherited_load: f64,
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopDownPlan {
    pub selections: Vec<TopDownSelection>,
    pub total_area: f64,
    pub maximum_arrival: DelayTime,
    pub total_output_load: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TopDownError {
    EmptyRoot {
        candidate: String,
    },
    EmptyCandidate {
        root: String,
    },
    InvalidMetric {
        root: String,
        candidate: String,
        metric: &'static str,
        value: f64,
    },
    MissingCandidates {
        root: String,
    },
    VirtualNetwork(VirtualNetworkError),
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for TopDownError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRoot { candidate } => {
                write!(f, "candidate '{candidate}' has an empty root")
            }
            Self::EmptyCandidate { root } => write!(f, "root '{root}' has an empty candidate id"),
            Self::InvalidMetric {
                root,
                candidate,
                metric,
                value,
            } => write!(
                f,
                "candidate '{candidate}' for root '{root}' has invalid {metric} {value}"
            ),
            Self::MissingCandidates { root } => {
                write!(f, "root '{root}' has no top-down mapping candidates")
            }
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::MissingSisPorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} native SIS prerequisite ports",
                dependencies.len()
            ),
        }
    }
}

impl Error for TopDownError {}

impl From<VirtualNetworkError> for TopDownError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

pub fn required_full_graph_beads() -> &'static [PortDependency] {
    REQUIRED_FULL_GRAPH_BEADS
}

pub fn full_sis_top_down_unavailable() -> Result<TopDownPlan, TopDownError> {
    Err(TopDownError::MissingSisPorts {
        operation: "top_down full SIS graph mapping",
        dependencies: REQUIRED_FULL_GRAPH_BEADS,
    })
}

pub fn plan_top_down_candidates(
    candidates: &[MappedTreeCandidate],
    options: TopDownOptions,
) -> Result<TopDownPlan, TopDownError> {
    validate_options(options)?;

    let mut by_root = BTreeMap::<String, Vec<&MappedTreeCandidate>>::new();
    for candidate in candidates {
        validate_candidate(candidate)?;
        by_root
            .entry(candidate.root.clone())
            .or_default()
            .push(candidate);
    }

    let mut selections = Vec::new();
    for (root, root_candidates) in by_root {
        let selected = root_candidates
            .iter()
            .min_by(|left, right| compare_candidates(left, right, options))
            .ok_or_else(|| TopDownError::MissingCandidates { root })?;
        selections.push(selection_from_candidate(selected, options));
    }
    selections.sort_by(|left, right| {
        right
            .level
            .cmp(&left.level)
            .then_with(|| left.root.cmp(&right.root))
            .then_with(|| left.candidate.cmp(&right.candidate))
    });

    Ok(plan_from_selections(selections))
}

pub fn plan_virtual_network_levels(
    network: &VirtualMappedNetwork,
    options: TopDownOptions,
) -> Result<TopDownPlan, TopDownError> {
    validate_options(options)?;

    let levels = network.levels()?;
    let mut candidates = Vec::new();

    for (level, nodes) in levels.iter().enumerate().rev() {
        for node in nodes {
            let mapped_node = network
                .node(*node)
                .ok_or(VirtualNetworkError::MissingNode(*node))?;
            if mapped_node.kind != NodeKind::Internal || mapped_node.gate.is_none() {
                continue;
            }

            let output_load = network.compute_load(*node, |_| 0.0);
            let required = network
                .compute_min_required(*node)
                .filter(|required| required.rise.is_finite() && required.fall.is_finite());
            let mut candidate = MappedTreeCandidate::new(
                mapped_node.name.clone(),
                virtual_candidate_id(*node, level),
                level,
                1.0,
                DelayTime::new(level as f64, level as f64),
            );
            candidate.input_count = mapped_node.save_binding.len();
            candidate.required = required;
            candidate.output_load = output_load;
            candidates.push(candidate);
        }
    }

    plan_top_down_candidates(&candidates, options)
}

fn validate_options(options: TopDownOptions) -> Result<(), TopDownError> {
    validate_option("load_weight", options.load_weight)?;
    validate_option("delay_weight", options.delay_weight)?;
    validate_option("area_weight", options.area_weight)?;
    validate_option("slack_weight", options.slack_weight)?;
    Ok(())
}

fn validate_option(metric: &'static str, value: f64) -> Result<(), TopDownError> {
    if !value.is_finite() || value < 0.0 {
        return Err(TopDownError::InvalidMetric {
            root: "<options>".to_string(),
            candidate: "<options>".to_string(),
            metric,
            value,
        });
    }

    Ok(())
}

fn validate_candidate(candidate: &MappedTreeCandidate) -> Result<(), TopDownError> {
    if candidate.root.is_empty() {
        return Err(TopDownError::EmptyRoot {
            candidate: candidate.candidate.clone(),
        });
    }
    if candidate.candidate.is_empty() {
        return Err(TopDownError::EmptyCandidate {
            root: candidate.root.clone(),
        });
    }

    validate_candidate_metric(candidate, "area", candidate.area)?;
    validate_candidate_metric(candidate, "arrival.rise", candidate.arrival.rise)?;
    validate_candidate_metric(candidate, "arrival.fall", candidate.arrival.fall)?;
    validate_candidate_metric(candidate, "output_load", candidate.output_load)?;
    validate_candidate_metric(candidate, "inherited_load", candidate.inherited_load)?;
    if let Some(required) = candidate.required {
        validate_candidate_metric(candidate, "required.rise", required.rise)?;
        validate_candidate_metric(candidate, "required.fall", required.fall)?;
    }

    Ok(())
}

fn validate_candidate_metric(
    candidate: &MappedTreeCandidate,
    metric: &'static str,
    value: f64,
) -> Result<(), TopDownError> {
    if !value.is_finite() {
        return Err(TopDownError::InvalidMetric {
            root: candidate.root.clone(),
            candidate: candidate.candidate.clone(),
            metric,
            value,
        });
    }
    if matches!(metric, "area" | "output_load" | "inherited_load") && value < 0.0 {
        return Err(TopDownError::InvalidMetric {
            root: candidate.root.clone(),
            candidate: candidate.candidate.clone(),
            metric,
            value,
        });
    }

    Ok(())
}

fn compare_candidates(
    left: &&MappedTreeCandidate,
    right: &&MappedTreeCandidate,
    options: TopDownOptions,
) -> std::cmp::Ordering {
    score(left, options)
        .total_cmp(&score(right, options))
        .then_with(|| left.level.cmp(&right.level))
        .then_with(|| left.input_count.cmp(&right.input_count))
        .then_with(|| left.area.total_cmp(&right.area))
        .then_with(|| left.candidate.cmp(&right.candidate))
}

fn selection_from_candidate(
    candidate: &MappedTreeCandidate,
    options: TopDownOptions,
) -> TopDownSelection {
    TopDownSelection {
        root: candidate.root.clone(),
        candidate: candidate.candidate.clone(),
        level: candidate.level,
        input_count: candidate.input_count,
        area: candidate.area,
        arrival: candidate.arrival,
        required: candidate.required,
        output_load: candidate.output_load,
        inherited_load: candidate.inherited_load,
        score: score(candidate, options),
    }
}

fn plan_from_selections(selections: Vec<TopDownSelection>) -> TopDownPlan {
    let total_area = selections.iter().map(|selection| selection.area).sum();
    let total_output_load = selections
        .iter()
        .map(|selection| selection.output_load)
        .sum();
    let maximum_arrival = selections.iter().fold(
        DelayTime::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        |acc, item| {
            DelayTime::new(
                acc.rise.max(item.arrival.rise),
                acc.fall.max(item.arrival.fall),
            )
        },
    );

    TopDownPlan {
        selections,
        total_area,
        maximum_arrival,
        total_output_load,
    }
}

fn score(candidate: &MappedTreeCandidate, options: TopDownOptions) -> f64 {
    let delay = candidate.arrival.rise.max(candidate.arrival.fall);
    let load = candidate.output_load + candidate.inherited_load;
    let slack_penalty = candidate.required.map_or(0.0, |required| {
        let rise_violation = (candidate.arrival.rise - required.rise).max(0.0);
        let fall_violation = (candidate.arrival.fall - required.fall).max(0.0);
        rise_violation.max(fall_violation)
    });

    (delay * options.delay_weight)
        + (load * options.load_weight)
        + (candidate.area * options.area_weight)
        + (slack_penalty * options.slack_weight)
}

fn virtual_candidate_id(node: NodeId, level: usize) -> String {
    format!("level:{level}:node:{}", node.index())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::virtual_net::{GateKind, SourceRef, VirtualMappedNetwork};

    #[test]
    fn selects_load_sensitive_candidate_per_root_in_deterministic_order() {
        let candidates = vec![
            MappedTreeCandidate {
                root: "z".to_string(),
                candidate: "fast-heavy".to_string(),
                level: 2,
                input_count: 3,
                area: 4.0,
                arrival: DelayTime::new(1.0, 1.0),
                required: Some(DelayTime::new(5.0, 5.0)),
                output_load: 10.0,
                inherited_load: 0.0,
            },
            MappedTreeCandidate {
                root: "z".to_string(),
                candidate: "slower-light".to_string(),
                level: 2,
                input_count: 2,
                area: 3.0,
                arrival: DelayTime::new(2.0, 2.0),
                required: Some(DelayTime::new(5.0, 5.0)),
                output_load: 1.0,
                inherited_load: 0.0,
            },
            MappedTreeCandidate {
                root: "a".to_string(),
                candidate: "only".to_string(),
                level: 0,
                input_count: 1,
                area: 1.0,
                arrival: DelayTime::new(0.5, 0.5),
                required: None,
                output_load: 0.0,
                inherited_load: 0.0,
            },
        ];

        let plan = plan_top_down_candidates(
            &candidates,
            TopDownOptions {
                area_weight: 0.0,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(plan.selections[0].root, "z");
        assert_eq!(plan.selections[0].candidate, "slower-light");
        assert_eq!(plan.selections[1].root, "a");
        assert_eq!(plan.total_area, 4.0);
        assert_eq!(plan.total_output_load, 1.0);
    }

    #[test]
    fn tie_breaks_by_level_input_count_area_and_candidate_id() {
        let candidates = vec![
            MappedTreeCandidate {
                root: "n".to_string(),
                candidate: "b".to_string(),
                level: 1,
                input_count: 2,
                area: 2.0,
                arrival: DelayTime::new(1.0, 1.0),
                required: None,
                output_load: 0.0,
                inherited_load: 0.0,
            },
            MappedTreeCandidate {
                root: "n".to_string(),
                candidate: "a".to_string(),
                level: 1,
                input_count: 2,
                area: 2.0,
                arrival: DelayTime::new(1.0, 1.0),
                required: None,
                output_load: 0.0,
                inherited_load: 0.0,
            },
        ];

        let plan = plan_top_down_candidates(&candidates, TopDownOptions::default()).unwrap();

        assert_eq!(plan.selections[0].candidate, "a");
    }

    #[test]
    fn applies_required_time_violation_to_score() {
        let candidates = vec![
            MappedTreeCandidate {
                root: "n".to_string(),
                candidate: "late".to_string(),
                level: 0,
                input_count: 1,
                area: 1.0,
                arrival: DelayTime::new(3.0, 3.0),
                required: Some(DelayTime::new(1.0, 1.0)),
                output_load: 0.0,
                inherited_load: 0.0,
            },
            MappedTreeCandidate {
                root: "n".to_string(),
                candidate: "on-time".to_string(),
                level: 0,
                input_count: 1,
                area: 100.0,
                arrival: DelayTime::new(2.0, 2.0),
                required: Some(DelayTime::new(2.0, 2.0)),
                output_load: 0.0,
                inherited_load: 0.0,
            },
        ];

        let plan = plan_top_down_candidates(
            &candidates,
            TopDownOptions {
                area_weight: 0.0,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(plan.selections[0].candidate, "on-time");
    }

    #[test]
    fn plans_virtual_network_in_top_down_level_order() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let n1 = network.add_gate(
            "n1",
            GateKind::And,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        let n2 = network.add_gate("n2", GateKind::Inverter, vec![SourceRef::Node(n1)]);
        network
            .add_primary_output("f", SourceRef::Node(n2))
            .unwrap();
        network.setup_gate_links().unwrap();

        let plan = plan_virtual_network_levels(&network, TopDownOptions::default()).unwrap();

        assert_eq!(
            plan.selections
                .iter()
                .map(|selection| selection.root.as_str())
                .collect::<Vec<_>>(),
            vec!["n2", "n1"]
        );
        assert!(
            plan.selections
                .iter()
                .any(|selection| selection.candidate == "level:2:node:3")
        );
    }

    #[test]
    fn reports_dependency_error_for_full_sis_graph_mapping() {
        assert!(required_full_graph_beads().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.272"
                && dependency.source_file == "LogicSynthesis/sis/map/treemap.c"
        }));

        assert_eq!(
            full_sis_top_down_unavailable(),
            Err(TopDownError::MissingSisPorts {
                operation: "top_down full SIS graph mapping",
                dependencies: REQUIRED_FULL_GRAPH_BEADS,
            })
        );
    }

    #[test]
    fn rejects_invalid_candidates_without_c_abi_exports() {
        let err = plan_top_down_candidates(
            &[MappedTreeCandidate::new(
                "",
                "candidate",
                0,
                1.0,
                DelayTime::new(0.0, 0.0),
            )],
            TopDownOptions::default(),
        )
        .expect_err("empty root should be rejected");

        assert_eq!(
            err,
            TopDownError::EmptyRoot {
                candidate: "candidate".to_string()
            }
        );

        let source = include_str!("top_down.rs");
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
