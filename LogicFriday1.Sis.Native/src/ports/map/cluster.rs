//! Native Rust clustering model for `sis/map/cluster.c`.
//!
//! The original SIS implementation labels a DFS-ordered `network_t` with
//! Lawler's clustering algorithm, forms one-output clusters, optionally
//! relabels nodes to reduce duplication, and then collapses every cluster into
//! its root node. This port keeps the deterministic graph algorithm over owned
//! Rust data. Full SIS `network_t` mutation and Boolean node collapsing remain
//! an integration boundary and are reported as diagnostics instead of hidden
//! C ABI calls.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClusterConstraint {
    Depth,
    Size,
    SizeAsDepth,
    Statistics,
    BestRatio,
    Fanin,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClusterOptions {
    pub constraint: ClusterConstraint,
    pub cluster_size: usize,
    pub depth: usize,
    pub relabel: bool,
    pub verbose: usize,
    pub duplication_ratio_limit: f64,
}

impl Default for ClusterOptions {
    fn default() -> Self {
        Self {
            constraint: ClusterConstraint::Size,
            cluster_size: 8,
            depth: 1,
            relabel: false,
            verbose: 0,
            duplication_ratio_limit: 2.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ClusterNodeId(usize);

impl ClusterNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClusterNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClusterNode {
    pub name: String,
    pub kind: ClusterNodeKind,
    pub fanins: Vec<ClusterNodeId>,
}

impl ClusterNode {
    fn new(name: impl Into<String>, kind: ClusterNodeKind, fanins: Vec<ClusterNodeId>) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClusterNetwork {
    nodes: Vec<ClusterNode>,
}

impl ClusterNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> ClusterNodeId {
        self.add_node(name, ClusterNodeKind::PrimaryInput, Vec::new())
    }

    pub fn add_internal(
        &mut self,
        name: impl Into<String>,
        fanins: Vec<ClusterNodeId>,
    ) -> ClusterNodeId {
        self.add_node(name, ClusterNodeKind::Internal, fanins)
    }

    pub fn add_primary_output(
        &mut self,
        name: impl Into<String>,
        fanin: ClusterNodeId,
    ) -> ClusterNodeId {
        self.add_node(name, ClusterNodeKind::PrimaryOutput, vec![fanin])
    }

    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        kind: ClusterNodeKind,
        fanins: Vec<ClusterNodeId>,
    ) -> ClusterNodeId {
        let id = ClusterNodeId(self.nodes.len());
        self.nodes.push(ClusterNode::new(name, kind, fanins));
        id
    }

    pub fn node(&self, id: ClusterNodeId) -> Option<&ClusterNode> {
        self.nodes.get(id.index())
    }

    pub fn nodes(&self) -> &[ClusterNode] {
        &self.nodes
    }

    pub fn internal_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.kind == ClusterNodeKind::Internal)
            .count()
    }

    pub fn validate(&self) -> Result<(), ClusterError> {
        for (index, node) in self.nodes.iter().enumerate() {
            if node.kind == ClusterNodeKind::PrimaryOutput && node.fanins.len() != 1 {
                return Err(ClusterError::InvalidPrimaryOutput {
                    node: ClusterNodeId(index),
                    fanin_count: node.fanins.len(),
                });
            }

            for fanin in &node.fanins {
                if fanin.index() >= self.nodes.len() {
                    return Err(ClusterError::MissingNode(*fanin));
                }
            }
        }

        Ok(())
    }

    fn fanouts(&self) -> Vec<Vec<ClusterNodeId>> {
        let mut fanouts = vec![Vec::new(); self.nodes.len()];
        for (index, node) in self.nodes.iter().enumerate() {
            for fanin in &node.fanins {
                if let Some(list) = fanouts.get_mut(fanin.index()) {
                    list.push(ClusterNodeId(index));
                }
            }
        }
        fanouts
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterAnalysis {
    pub selected_cluster_size: usize,
    pub max_label: Option<usize>,
    pub level_count: usize,
    pub clusters: Vec<Cluster>,
    pub duplication_ratio: Option<f64>,
    pub statistics: Vec<ClusterStatisticsEntry>,
    pub diagnostics: Vec<ClusterDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cluster {
    pub root: ClusterNodeId,
    pub label: usize,
    pub nodes: BTreeSet<ClusterNodeId>,
}

impl Cluster {
    pub fn size(&self) -> usize {
        self.nodes.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterStatisticsEntry {
    pub cluster_size: usize,
    pub level_count: usize,
    pub duplication_ratio: f64,
    pub relabelled_duplication_ratio: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClusterDiagnostic {
    FullSisNetworkMutationUnavailable { operation: &'static str },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClusterError {
    MissingNode(ClusterNodeId),
    InvalidPrimaryOutput {
        node: ClusterNodeId,
        fanin_count: usize,
    },
    InvalidClusterSize(usize),
    InvalidDepth(usize),
    EmptySearchRange,
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for ClusterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "missing cluster node {}", node.index()),
            Self::InvalidPrimaryOutput { node, fanin_count } => {
                write!(
                    f,
                    "primary output {} must have one fanin, got {fanin_count}",
                    node.index()
                )
            }
            Self::InvalidClusterSize(size) => {
                write!(f, "cluster size must be positive, got {size}")
            }
            Self::InvalidDepth(depth) => write!(f, "depth must be positive, got {depth}"),
            Self::EmptySearchRange => write!(f, "cluster size search range is empty"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for ClusterError {}

pub fn sis_network_cluster_collapse_unavailable() -> Result<ClusterAnalysis, ClusterError> {
    Err(ClusterError::MissingSisPorts {
        operation: "cluster collapse over SIS network_t",
    })
}

pub fn cluster_under_constraint(
    network: &ClusterNetwork,
    options: ClusterOptions,
) -> Result<ClusterAnalysis, ClusterError> {
    network.validate()?;

    match options.constraint {
        ClusterConstraint::Size | ClusterConstraint::Fanin => {
            analyze_size_constraint(network, options, options.cluster_size)
        }
        ClusterConstraint::Depth => analyze_depth_constraint(network, options),
        ClusterConstraint::SizeAsDepth => analyze_size_as_depth_constraint(network, options),
        ClusterConstraint::Statistics => gather_cluster_statistics(network, options),
        ClusterConstraint::BestRatio => analyze_best_ratio_constraint(network, options),
    }
}

pub fn label_network(
    network: &ClusterNetwork,
    options: ClusterOptions,
) -> Result<BTreeMap<ClusterNodeId, usize>, ClusterError> {
    network.validate()?;
    validate_cluster_size(options.cluster_size)?;

    let infos = label_nodes(network, options.cluster_size, options.constraint)?;
    Ok(labels_from_infos(&infos))
}

pub fn form_network_clusters(
    network: &ClusterNetwork,
    options: ClusterOptions,
) -> Result<Vec<Cluster>, ClusterError> {
    network.validate()?;
    validate_cluster_size(options.cluster_size)?;

    let mut infos = label_nodes(network, options.cluster_size, options.constraint)?;
    let max_label = max_label(&infos);
    if options.relabel {
        relabel_nodes(network, &mut infos, max_label);
    }
    form_clusters(network, &mut infos)
}

fn analyze_size_constraint(
    network: &ClusterNetwork,
    mut options: ClusterOptions,
    cluster_size: usize,
) -> Result<ClusterAnalysis, ClusterError> {
    validate_cluster_size(cluster_size)?;
    options.cluster_size = cluster_size;

    let mut infos = label_nodes(network, cluster_size, options.constraint)?;
    let max_label = max_label(&infos);
    if options.relabel {
        relabel_nodes(network, &mut infos, max_label);
    }
    let clusters = form_clusters(network, &mut infos)?;
    let duplication_ratio = duplication_ratio(network, &clusters);

    Ok(ClusterAnalysis {
        selected_cluster_size: cluster_size,
        max_label,
        level_count: max_label.map_or(0, |label| label + 1),
        clusters,
        duplication_ratio,
        statistics: Vec::new(),
        diagnostics: vec![ClusterDiagnostic::FullSisNetworkMutationUnavailable {
            operation: "cluster collapse into SIS node roots",
        }],
    })
}

fn analyze_depth_constraint(
    network: &ClusterNetwork,
    options: ClusterOptions,
) -> Result<ClusterAnalysis, ClusterError> {
    validate_depth(options.depth)?;
    let internal_count = network.internal_count();
    if internal_count == 0 {
        return analyze_size_constraint(network, options, 1);
    }

    let target = options.depth.saturating_sub(1);
    let size = lazy_binary_search(1, internal_count, target, |cluster_size| {
        best_labelling(network, cluster_size, options.constraint)
    })?;

    analyze_size_constraint(network, options, size)
}

fn analyze_size_as_depth_constraint(
    network: &ClusterNetwork,
    options: ClusterOptions,
) -> Result<ClusterAnalysis, ClusterError> {
    validate_cluster_size(options.cluster_size)?;
    let max_label = best_labelling(network, options.cluster_size, options.constraint)?;
    let size = lazy_binary_search(1, options.cluster_size, max_label, |cluster_size| {
        best_labelling(network, cluster_size, options.constraint)
    })?;

    analyze_size_constraint(network, options, size)
}

fn gather_cluster_statistics(
    network: &ClusterNetwork,
    options: ClusterOptions,
) -> Result<ClusterAnalysis, ClusterError> {
    let mut statistics = Vec::new();
    let max_size = network.internal_count();
    for cluster_size in 1..=max_size {
        let infos = label_nodes(network, cluster_size, ClusterConstraint::Size)?;
        let max_label = max_label(&infos);
        let mut plain_infos = infos.clone();
        let plain_clusters = form_clusters(network, &mut plain_infos)?;
        let plain_duplication = duplication_ratio(network, &plain_clusters).unwrap_or(0.0);

        let mut relabelled_infos = infos;
        relabel_nodes(network, &mut relabelled_infos, max_label);
        let relabelled_clusters = form_clusters(network, &mut relabelled_infos)?;
        let relabelled_duplication =
            duplication_ratio(network, &relabelled_clusters).unwrap_or(0.0);

        statistics.push(ClusterStatisticsEntry {
            cluster_size,
            level_count: max_label.map_or(0, |label| label + 1),
            duplication_ratio: plain_duplication,
            relabelled_duplication_ratio: relabelled_duplication,
        });

        if max_label == Some(0) {
            break;
        }
    }

    Ok(ClusterAnalysis {
        selected_cluster_size: options.cluster_size,
        max_label: None,
        level_count: 0,
        clusters: Vec::new(),
        duplication_ratio: None,
        statistics,
        diagnostics: Vec::new(),
    })
}

fn analyze_best_ratio_constraint(
    network: &ClusterNetwork,
    options: ClusterOptions,
) -> Result<ClusterAnalysis, ClusterError> {
    let max_size = network.internal_count();
    if max_size < 2 {
        return analyze_size_constraint(network, options, 1);
    }

    let mut best_cluster_size = None;
    let mut best_level = usize::MAX;
    let mut best_duplication_ratio = f64::INFINITY;

    for cluster_size in 2..=max_size {
        let mut infos = label_nodes(network, cluster_size, ClusterConstraint::Size)?;
        let max_label = max_label(&infos);
        relabel_nodes(network, &mut infos, max_label);
        let clusters = form_clusters(network, &mut infos)?;
        let ratio = duplication_ratio(network, &clusters).unwrap_or(0.0);

        if ratio >= options.duplication_ratio_limit || max_label == Some(0) {
            break;
        }

        let level = max_label.unwrap_or(0);
        if level < best_level || (level == best_level && ratio < best_duplication_ratio) {
            best_level = level;
            best_duplication_ratio = ratio;
            best_cluster_size = Some(cluster_size);
        }
    }

    analyze_size_constraint(network, options, best_cluster_size.unwrap_or(2))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClusterNodeType {
    Normal,
    Root,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ClusterNodeInfo {
    label: usize,
    max_label: usize,
    weight: usize,
    node_type: ClusterNodeType,
    clusters: Vec<usize>,
}

impl ClusterNodeInfo {
    fn new() -> Self {
        Self {
            label: 0,
            max_label: 0,
            weight: 1,
            node_type: ClusterNodeType::Normal,
            clusters: Vec::new(),
        }
    }
}

fn label_nodes(
    network: &ClusterNetwork,
    cluster_size: usize,
    constraint: ClusterConstraint,
) -> Result<Vec<Option<ClusterNodeInfo>>, ClusterError> {
    validate_cluster_size(cluster_size)?;

    let mut infos = vec![None; network.nodes.len()];
    for (index, node) in network.nodes.iter().enumerate() {
        if node.kind != ClusterNodeKind::Internal {
            continue;
        }

        let id = ClusterNodeId(index);
        let mut info = ClusterNodeInfo::new();
        info.label = decide_label_node(network, &infos, id, cluster_size, constraint)?;
        infos[index] = Some(info);
    }

    Ok(infos)
}

fn decide_label_node(
    network: &ClusterNetwork,
    infos: &[Option<ClusterNodeInfo>],
    node: ClusterNodeId,
    cluster_size: usize,
    constraint: ClusterConstraint,
) -> Result<usize, ClusterError> {
    let max_fanin_label = max_fanin_label(network, infos, node)?;
    let Some(max_fanin_label) = max_fanin_label else {
        return Ok(0);
    };

    let mut scratch = infos.to_vec();
    if let Some(info) = scratch.get_mut(node.index()).and_then(Option::as_mut) {
        info.label = max_fanin_label;
    } else {
        scratch[node.index()] = Some(ClusterNodeInfo {
            label: max_fanin_label,
            ..ClusterNodeInfo::new()
        });
    }

    let total_weight =
        compute_tfi_weight_for_given_label(network, &scratch, node, max_fanin_label, constraint)?;
    if total_weight > cluster_size {
        Ok(max_fanin_label + 1)
    } else {
        Ok(max_fanin_label)
    }
}

fn max_fanin_label(
    network: &ClusterNetwork,
    infos: &[Option<ClusterNodeInfo>],
    node: ClusterNodeId,
) -> Result<Option<usize>, ClusterError> {
    let node = network.node(node).ok_or(ClusterError::MissingNode(node))?;
    let mut result: Option<usize> = None;

    for fanin in &node.fanins {
        let fanin_node = network
            .node(*fanin)
            .ok_or(ClusterError::MissingNode(*fanin))?;
        if fanin_node.kind == ClusterNodeKind::PrimaryInput {
            continue;
        }

        let label = infos
            .get(fanin.index())
            .and_then(Option::as_ref)
            .ok_or(ClusterError::MissingNode(*fanin))?
            .label;
        result = Some(result.map_or(label, |value| value.max(label)));
    }

    Ok(result)
}

fn compute_tfi_weight_for_given_label(
    network: &ClusterNetwork,
    infos: &[Option<ClusterNodeInfo>],
    node: ClusterNodeId,
    label: usize,
    constraint: ClusterConstraint,
) -> Result<usize, ClusterError> {
    let mut visited = BTreeSet::new();
    let mut visited_fanins = BTreeSet::new();
    compute_tfi_for_given_label_rec(
        network,
        infos,
        node,
        label,
        &mut visited,
        &mut visited_fanins,
    )?;

    if constraint == ClusterConstraint::Fanin {
        Ok(visited_fanins.len())
    } else {
        visited.into_iter().try_fold(0usize, |sum, id| {
            let weight = infos
                .get(id.index())
                .and_then(Option::as_ref)
                .ok_or(ClusterError::MissingNode(id))?
                .weight;
            Ok(sum + weight)
        })
    }
}

fn compute_tfi_for_given_label_rec(
    network: &ClusterNetwork,
    infos: &[Option<ClusterNodeInfo>],
    node: ClusterNodeId,
    label: usize,
    visited: &mut BTreeSet<ClusterNodeId>,
    visited_fanins: &mut BTreeSet<ClusterNodeId>,
) -> Result<(), ClusterError> {
    if visited.contains(&node) {
        return Ok(());
    }

    let item = network.node(node).ok_or(ClusterError::MissingNode(node))?;
    if item.kind == ClusterNodeKind::PrimaryInput {
        visited_fanins.insert(node);
        return Ok(());
    }

    let info = infos
        .get(node.index())
        .and_then(Option::as_ref)
        .ok_or(ClusterError::MissingNode(node))?;
    if info.label < label {
        visited_fanins.insert(node);
        return Ok(());
    }

    visited.insert(node);
    for fanin in &item.fanins {
        compute_tfi_for_given_label_rec(network, infos, *fanin, label, visited, visited_fanins)?;
    }

    Ok(())
}

fn relabel_nodes(
    network: &ClusterNetwork,
    infos: &mut [Option<ClusterNodeInfo>],
    max_label: Option<usize>,
) {
    let max_label = max_label.unwrap_or(0);

    for info in infos.iter_mut().filter_map(Option::as_mut) {
        info.max_label = max_label;
    }

    for (index, node) in network.nodes.iter().enumerate().rev() {
        if node.kind != ClusterNodeKind::Internal {
            continue;
        }

        let node_label = infos[index].as_ref().map_or(0, |info| info.label);
        let node_max_label = infos[index]
            .as_ref()
            .map_or(max_label, |info| info.max_label);
        for fanin in &node.fanins {
            if network
                .node(*fanin)
                .is_none_or(|node| node.kind != ClusterNodeKind::Internal)
            {
                continue;
            }

            if let Some(fanin_info) = infos.get_mut(fanin.index()).and_then(Option::as_mut) {
                let limit = if fanin_info.label == node_label {
                    node_max_label
                } else {
                    node_max_label.saturating_sub(1)
                };
                fanin_info.max_label = fanin_info.max_label.min(limit);
            }
        }

        if let Some(info) = infos[index].as_mut() {
            info.label = info.max_label;
        }
    }
}

fn form_clusters(
    network: &ClusterNetwork,
    infos: &mut [Option<ClusterNodeInfo>],
) -> Result<Vec<Cluster>, ClusterError> {
    let fanouts = network.fanouts();
    let mut clusters = Vec::new();

    for (index, node) in network.nodes.iter().enumerate().rev() {
        if node.kind != ClusterNodeKind::Internal {
            continue;
        }

        let id = ClusterNodeId(index);
        let label = infos[index]
            .as_ref()
            .ok_or(ClusterError::MissingNode(id))?
            .label;
        let mut has_larger_fanout_label = false;

        for fanout in &fanouts[index] {
            let fanout_node = network
                .node(*fanout)
                .ok_or(ClusterError::MissingNode(*fanout))?;
            if fanout_node.kind == ClusterNodeKind::PrimaryOutput {
                has_larger_fanout_label = true;
                continue;
            }

            if fanout_node.kind == ClusterNodeKind::Internal {
                let fanout_label = infos
                    .get(fanout.index())
                    .and_then(Option::as_ref)
                    .ok_or(ClusterError::MissingNode(*fanout))?
                    .label;
                has_larger_fanout_label |= fanout_label > label;
            }
        }

        if has_larger_fanout_label {
            let cluster_index = clusters.len();
            if let Some(info) = infos[index].as_mut() {
                info.node_type = ClusterNodeType::Root;
            }

            let mut cluster = Cluster {
                root: id,
                label,
                nodes: BTreeSet::new(),
            };
            put_tfi_in_cluster_rec(network, infos, &mut cluster, cluster_index, id, label)?;
            clusters.push(cluster);
        }
    }

    Ok(clusters)
}

fn put_tfi_in_cluster_rec(
    network: &ClusterNetwork,
    infos: &mut [Option<ClusterNodeInfo>],
    cluster: &mut Cluster,
    cluster_index: usize,
    node: ClusterNodeId,
    label: usize,
) -> Result<(), ClusterError> {
    let item = network.node(node).ok_or(ClusterError::MissingNode(node))?;
    if item.kind == ClusterNodeKind::PrimaryInput || cluster.nodes.contains(&node) {
        return Ok(());
    }

    let node_label = infos
        .get(node.index())
        .and_then(Option::as_ref)
        .ok_or(ClusterError::MissingNode(node))?
        .label;
    if node_label < label {
        return Ok(());
    }

    cluster.nodes.insert(node);
    if let Some(info) = infos[node.index()].as_mut() {
        info.clusters.push(cluster_index);
    }

    let fanins = item.fanins.clone();
    for fanin in fanins {
        put_tfi_in_cluster_rec(network, infos, cluster, cluster_index, fanin, label)?;
    }

    Ok(())
}

fn duplication_ratio(network: &ClusterNetwork, clusters: &[Cluster]) -> Option<f64> {
    let before = network.internal_count();
    if before == 0 {
        return None;
    }

    let after = clusters.iter().map(Cluster::size).sum::<usize>();
    Some(after as f64 / before as f64)
}

fn best_labelling(
    network: &ClusterNetwork,
    cluster_size: usize,
    constraint: ClusterConstraint,
) -> Result<usize, ClusterError> {
    Ok(max_label(&label_nodes(network, cluster_size, constraint)?).unwrap_or(0))
}

fn lazy_binary_search(
    mut min: usize,
    mut max: usize,
    value: usize,
    mut f: impl FnMut(usize) -> Result<usize, ClusterError>,
) -> Result<usize, ClusterError> {
    if min > max {
        return Err(ClusterError::EmptySearchRange);
    }

    let mut min_value = f(min)?;
    if min_value <= value {
        return Ok(min);
    }

    let max_value = f(max)?;
    if max_value > value {
        return Ok(max);
    }

    loop {
        let middle = min + (max - min) / 2;
        if middle == min {
            return Ok(if min_value <= value { min } else { max });
        }

        let middle_value = f(middle)?;
        if middle_value > value {
            min = middle;
            min_value = middle_value;
        } else {
            max = middle;
            let _ = middle_value;
        }
    }
}

fn labels_from_infos(infos: &[Option<ClusterNodeInfo>]) -> BTreeMap<ClusterNodeId, usize> {
    infos
        .iter()
        .enumerate()
        .filter_map(|(index, info)| info.as_ref().map(|info| (ClusterNodeId(index), info.label)))
        .collect()
}

fn max_label(infos: &[Option<ClusterNodeInfo>]) -> Option<usize> {
    infos
        .iter()
        .filter_map(|info| info.as_ref().map(|info| info.label))
        .max()
}

fn validate_cluster_size(cluster_size: usize) -> Result<(), ClusterError> {
    if cluster_size == 0 {
        Err(ClusterError::InvalidClusterSize(cluster_size))
    } else {
        Ok(())
    }
}

fn validate_depth(depth: usize) -> Result<(), ClusterError> {
    if depth == 0 {
        Err(ClusterError::InvalidDepth(depth))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain_network() -> (ClusterNetwork, ClusterNodeId, ClusterNodeId) {
        let mut network = ClusterNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let n1 = network.add_internal("n1", vec![a, b]);
        let n2 = network.add_internal("n2", vec![n1]);
        network.add_primary_output("y", n2);
        (network, n1, n2)
    }

    #[test]
    fn labels_nodes_with_lawler_cluster_size_limit() {
        let (network, n1, n2) = chain_network();

        let labels = label_network(
            &network,
            ClusterOptions {
                cluster_size: 1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(labels[&n1], 0);
        assert_eq!(labels[&n2], 1);
    }

    #[test]
    fn forms_roots_at_primary_outputs_and_label_boundaries() {
        let (network, n1, n2) = chain_network();

        let clusters = form_network_clusters(
            &network,
            ClusterOptions {
                cluster_size: 1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].root, n2);
        assert_eq!(clusters[0].nodes, BTreeSet::from([n2]));
        assert_eq!(clusters[1].root, n1);
        assert_eq!(clusters[1].nodes, BTreeSet::from([n1]));
    }

    #[test]
    fn size_constraint_reports_duplication_and_mutation_diagnostic() {
        let (network, _, _) = chain_network();

        let analysis = cluster_under_constraint(
            &network,
            ClusterOptions {
                cluster_size: 2,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(analysis.max_label, Some(0));
        assert_eq!(analysis.level_count, 1);
        assert_eq!(analysis.duplication_ratio, Some(1.0));
        assert_eq!(
            analysis.diagnostics,
            vec![ClusterDiagnostic::FullSisNetworkMutationUnavailable {
                operation: "cluster collapse into SIS node roots",
            }]
        );
    }

    #[test]
    fn depth_constraint_selects_minimum_cluster_size() {
        let (network, _, _) = chain_network();

        let analysis = cluster_under_constraint(
            &network,
            ClusterOptions {
                constraint: ClusterConstraint::Depth,
                depth: 1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(analysis.selected_cluster_size, 2);
        assert_eq!(analysis.level_count, 1);
    }

    #[test]
    fn statistics_reports_plain_and_relabelled_ratios() {
        let (network, _, _) = chain_network();

        let analysis = cluster_under_constraint(
            &network,
            ClusterOptions {
                constraint: ClusterConstraint::Statistics,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(analysis.statistics.len(), 2);
        assert_eq!(analysis.statistics[0].cluster_size, 1);
        assert_eq!(analysis.statistics[0].level_count, 2);
        assert_eq!(analysis.statistics[1].cluster_size, 2);
        assert_eq!(analysis.statistics[1].level_count, 1);
    }

    #[test]
    fn fanin_constraint_counts_distinct_cluster_inputs() {
        let mut network = ClusterNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let c = network.add_primary_input("c");
        let n1 = network.add_internal("n1", vec![a, b]);
        let n2 = network.add_internal("n2", vec![n1, c]);
        network.add_primary_output("y", n2);

        let labels = label_network(
            &network,
            ClusterOptions {
                constraint: ClusterConstraint::Fanin,
                cluster_size: 2,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(labels[&n1], 0);
        assert_eq!(labels[&n2], 1);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("cluster.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
