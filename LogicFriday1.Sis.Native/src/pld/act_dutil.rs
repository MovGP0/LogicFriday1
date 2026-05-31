//! Native Rust delay and collapse utilities for ACT-style PLD mapping.
//!
//! This module models the behavior with owned Rust graph data. It does not
//! expose legacy per-file C ABI entry points.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;

pub const DELAY_NOT_SET: f64 = f64::NEG_INFINITY;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayNode {
    pub name: String,
    pub kind: NodeKind,
    fanins: Vec<NodeId>,
    arrival_rise: Option<f64>,
    required_rise: Option<f64>,
}

impl DelayNode {
    pub fn new(name: impl Into<String>, kind: NodeKind, fanins: Vec<NodeId>) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins,
            arrival_rise: None,
            required_rise: None,
        }
    }

    pub fn primary_input(name: impl Into<String>) -> Self {
        Self::new(name, NodeKind::PrimaryInput, Vec::new())
    }

    pub fn primary_output(name: impl Into<String>, fanin: NodeId) -> Self {
        Self::new(name, NodeKind::PrimaryOutput, vec![fanin])
    }

    pub fn internal(name: impl Into<String>, fanins: Vec<NodeId>) -> Self {
        Self::new(name, NodeKind::Internal, fanins)
    }

    pub fn with_arrival_rise(mut self, arrival: f64) -> Self {
        self.arrival_rise = Some(arrival);
        self
    }

    pub fn with_required_rise(mut self, required: f64) -> Self {
        self.required_rise = Some(required);
        self
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }

    pub fn arrival_rise(&self) -> Option<f64> {
        self.arrival_rise
    }

    pub fn required_rise(&self) -> Option<f64> {
        self.required_rise
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DelayNetwork {
    nodes: Vec<DelayNode>,
}

impl DelayNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: DelayNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> ActDutilResult<&DelayNode> {
        self.nodes.get(id.0).ok_or(ActDutilError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[DelayNode] {
        &self.nodes
    }

    pub fn primary_outputs(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                (node.kind == NodeKind::PrimaryOutput).then_some(NodeId(index))
            })
            .collect()
    }

    pub fn fanouts(&self, node: NodeId) -> ActDutilResult<Vec<NodeId>> {
        self.node(node)?;
        Ok(self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                candidate.fanins.contains(&node).then_some(NodeId(index))
            })
            .collect())
    }

    pub fn fanout_count(&self, node: NodeId) -> ActDutilResult<usize> {
        Ok(self.fanouts(node)?.len())
    }

    pub fn find_node_by_name(&self, name: &str) -> Option<NodeId> {
        self.nodes
            .iter()
            .position(|node| node.name == name)
            .map(NodeId)
    }

    pub fn topological_order(&self) -> ActDutilResult<Vec<NodeId>> {
        for (index, node) in self.nodes.iter().enumerate() {
            let current = NodeId(index);
            for fanin in &node.fanins {
                self.node(*fanin)?;
                if fanin.0 >= current.0 {
                    return Err(ActDutilError::NotTopological {
                        node: current,
                        fanin: *fanin,
                    });
                }
            }
        }

        Ok((0..self.nodes.len()).map(NodeId).collect())
    }

    pub fn reverse_topological_order(&self) -> ActDutilResult<Vec<NodeId>> {
        let mut order = self.topological_order()?;
        order.reverse();
        Ok(order)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActVertexFanout {
    pub multiple_fanouts: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CostEntry {
    pub node: NodeId,
    pub arrival_time: f64,
    pub required_time: f64,
    pub cost_and_arrival_time: f64,
    pub cost: i32,
    pub slack: f64,
    pub is_critical: bool,
    pub area_weight: f64,
}

impl CostEntry {
    pub fn new(node: NodeId) -> Self {
        Self {
            node,
            arrival_time: -1.0,
            required_time: -1.0,
            cost_and_arrival_time: -1.0,
            cost: 0,
            slack: 0.0,
            is_critical: false,
            area_weight: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CostTable {
    entries: HashMap<NodeId, CostEntry>,
}

impl CostTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, node: NodeId) -> ActDutilResult<&CostEntry> {
        self.entries
            .get(&node)
            .ok_or(ActDutilError::MissingCostEntry(node))
    }

    pub fn get_mut(&mut self, node: NodeId) -> ActDutilResult<&mut CostEntry> {
        self.entries
            .get_mut(&node)
            .ok_or(ActDutilError::MissingCostEntry(node))
    }

    pub fn entry_or_insert(&mut self, node: NodeId) -> &mut CostEntry {
        self.entries
            .entry(node)
            .or_insert_with(|| CostEntry::new(node))
    }

    pub fn set_cost(&mut self, node: NodeId, cost: i32) {
        self.entry_or_insert(node).cost = cost;
    }

    pub fn entries(&self) -> &HashMap<NodeId, CostEntry> {
        &self.entries
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForwardDelayReport {
    pub slowest_output: Option<NodeId>,
    pub max_output_arrival_time: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollapsiblePair {
    pub nodename: String,
    pub fanoutname: String,
    pub weight: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopologicalEntry {
    pub nodename: String,
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActDutilError {
    UnknownNode(NodeId),
    MissingCostEntry(NodeId),
    MissingPrimaryOutputFanin(NodeId),
    ZeroFanout(NodeId),
    InvalidDelayTable { len: usize },
    NotTopological { node: NodeId, fanin: NodeId },
    MissingTopologicalEntry(NodeId),
    MissingNodeForTopologicalName(String),
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for ActDutilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown ACT delay node {:?}", node),
            Self::MissingCostEntry(node) => {
                write!(f, "missing ACT delay cost entry for {:?}", node)
            }
            Self::MissingPrimaryOutputFanin(node) => {
                write!(f, "primary output {:?} has no fanin", node)
            }
            Self::ZeroFanout(node) => write!(f, "ACT delay node {:?} has zero fanout", node),
            Self::InvalidDelayTable { len } => {
                write!(
                    f,
                    "ACT delay table must contain at least two values, got {len}"
                )
            }
            Self::NotTopological { node, fanin } => {
                write!(f, "node {:?} appears before its fanin {:?}", node, fanin)
            }
            Self::MissingTopologicalEntry(node) => {
                write!(f, "missing topological entry for fanin {:?}", node)
            }
            Self::MissingNodeForTopologicalName(name) => {
                write!(f, "missing node named {name} while sorting fanins")
            }
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
        }
    }
}

impl Error for ActDutilError {}

pub type ActDutilResult<T> = Result<T, ActDutilError>;

pub fn sis_bound_operation_unavailable(operation: &'static str) -> ActDutilResult<()> {
    Err(ActDutilError::MissingNativePorts { operation })
}

pub fn delay_trace_forward(
    network: &DelayNetwork,
    cost_table: &mut CostTable,
    delay_values: &[f64],
) -> ActDutilResult<ForwardDelayReport> {
    validate_delay_table(delay_values)?;
    let mut max_output_arrival_time = -1.0;
    let mut slowest_output = None;

    for node_id in network.topological_order()? {
        let node = network.node(node_id)?;
        match node.kind {
            NodeKind::PrimaryInput => {
                set_pi_arrival_time_node(network, node_id, cost_table)?;
            }
            NodeKind::PrimaryOutput => {
                let fanin = node
                    .fanins()
                    .first()
                    .copied()
                    .ok_or(ActDutilError::MissingPrimaryOutputFanin(node_id))?;
                let fanin_delay = get_arrival_time(fanin, cost_table)?;
                let cost_node = set_arrival_time(node_id, cost_table, fanin_delay);
                if cost_node.arrival_time > max_output_arrival_time {
                    max_output_arrival_time = cost_node.arrival_time;
                    slowest_output = Some(node_id);
                }
            }
            NodeKind::Internal => {
                let mut max_fanin_delay = 0.0;
                for fanin in node.fanins() {
                    let fanin_delay = get_arrival_time(*fanin, cost_table)?;
                    if fanin_delay >= max_fanin_delay {
                        max_fanin_delay = fanin_delay;
                    }
                }

                let fanout_count = network.fanout_count(node_id)?;
                if fanout_count == 0 {
                    return Err(ActDutilError::ZeroFanout(node_id));
                }

                let total_delay = delay_for_fanout(delay_values, fanout_count)? + max_fanin_delay;
                set_arrival_time(node_id, cost_table, total_delay);
            }
        }
    }

    Ok(ForwardDelayReport {
        slowest_output,
        max_output_arrival_time,
    })
}

pub fn delay_trace_backward(
    network: &DelayNetwork,
    cost_table: &mut CostTable,
    delay_values: &[f64],
) -> ActDutilResult<()> {
    validate_delay_table(delay_values)?;
    let primary_outputs = network.primary_outputs();
    let mut max_output_required_time: f64 = -1.0;
    let mut any_required_set = false;
    let mut some_required_unset = false;

    for po in &primary_outputs {
        if let Some(required) = network.node(*po)?.required_rise() {
            any_required_set = true;
            max_output_required_time = max_output_required_time.max(required);
            set_required_time(*po, cost_table, required);
        } else {
            some_required_unset = true;
        }
    }

    if !any_required_set {
        let max_arrival_time = get_max_arrival_time(network, cost_table)?;
        for po in &primary_outputs {
            set_required_time(*po, cost_table, max_arrival_time);
        }
    } else if some_required_unset {
        for po in &primary_outputs {
            if network.node(*po)?.required_rise().is_none() {
                set_required_time(*po, cost_table, max_output_required_time);
            }
        }
    }

    for node_id in network.reverse_topological_order()? {
        let node = network.node(node_id)?;
        if node.kind == NodeKind::PrimaryOutput {
            continue;
        }

        let mut min_node_required_time = f64::from(i32::MAX);
        for fanout in network.fanouts(node_id)? {
            let fanout_required_time = get_required_time(fanout, cost_table)?;
            let fanout_node = network.node(fanout)?;
            let node_required_time = if fanout_node.kind == NodeKind::PrimaryOutput {
                fanout_required_time
            } else {
                let fanout_count = network.fanout_count(fanout)?;
                if fanout_count == 0 {
                    return Err(ActDutilError::ZeroFanout(fanout));
                }
                fanout_required_time - delay_for_fanout(delay_values, fanout_count)?
            };

            if node_required_time < min_node_required_time {
                min_node_required_time = node_required_time;
            }
        }

        set_required_time(node_id, cost_table, min_node_required_time);
    }

    Ok(())
}

pub fn get_arrival_time(node: NodeId, cost_table: &CostTable) -> ActDutilResult<f64> {
    Ok(cost_table.get(node)?.arrival_time)
}

pub fn get_required_time(node: NodeId, cost_table: &CostTable) -> ActDutilResult<f64> {
    Ok(cost_table.get(node)?.required_time)
}

pub fn set_arrival_time(node: NodeId, cost_table: &mut CostTable, arrival: f64) -> &mut CostEntry {
    let cost_node = cost_table.entry_or_insert(node);
    cost_node.arrival_time = arrival;
    cost_node
}

pub fn set_required_time(
    node: NodeId,
    cost_table: &mut CostTable,
    required: f64,
) -> &mut CostEntry {
    let cost_node = cost_table.entry_or_insert(node);
    cost_node.required_time = required;
    cost_node
}

pub fn get_bddfanout_delay(vertex: ActVertexFanout, delay_values: &[f64]) -> ActDutilResult<f64> {
    delay_for_fanout(delay_values, vertex.multiple_fanouts + 1)
}

pub fn get_node_delay_correction(
    delay_values: &[f64],
    assumed_numfo: usize,
    actual_numfo: usize,
) -> ActDutilResult<f64> {
    if actual_numfo == 0 || assumed_numfo == 0 || actual_numfo == assumed_numfo {
        return Ok(0.0);
    }

    Ok(delay_for_fanout(delay_values, actual_numfo)?
        - delay_for_fanout(delay_values, assumed_numfo)?)
}

pub fn set_pi_arrival_time_network(
    network: &DelayNetwork,
    cost_table: &mut CostTable,
) -> ActDutilResult<()> {
    for (index, node) in network.nodes().iter().enumerate() {
        if node.kind == NodeKind::PrimaryInput {
            set_pi_arrival_time_node(network, NodeId(index), cost_table)?;
        }
    }
    Ok(())
}

pub fn set_pi_arrival_time_node<'a>(
    network: &DelayNetwork,
    pi: NodeId,
    cost_table: &'a mut CostTable,
) -> ActDutilResult<&'a mut CostEntry> {
    let arrival_time = network.node(pi)?.arrival_rise().unwrap_or(0.0);
    Ok(set_arrival_time(pi, cost_table, arrival_time))
}

pub fn invalidate_cost_and_arrival_time(cost_node: &mut CostEntry) {
    cost_node.cost_and_arrival_time = -1.0;
}

pub fn cost_delay(cost_node: &CostEntry, mode: f32) -> f64 {
    if cost_node.cost_and_arrival_time < 0.0 {
        ((1.0 - f64::from(mode)) * f64::from(cost_node.cost))
            + (f64::from(mode) * cost_node.arrival_time)
    } else {
        cost_node.cost_and_arrival_time
    }
}

pub fn delay_for_fanout(delay_values: &[f64], num_fanout: usize) -> ActDutilResult<f64> {
    validate_delay_table(delay_values)?;
    let last_index = delay_values.len() - 1;

    if num_fanout > last_index {
        let delaynum1 = delay_values[last_index];
        let delaynum2 = delay_values[last_index - 1];
        Ok(delaynum1 + (delaynum1 - delaynum2) * (num_fanout - last_index) as f64)
    } else {
        Ok(delay_values[num_fanout])
    }
}

pub fn set_slack_network(network: &DelayNetwork, cost_table: &mut CostTable) -> ActDutilResult<()> {
    for node_id in network.topological_order()? {
        set_slack_node(node_id, cost_table)?;
    }
    Ok(())
}

pub fn set_slack_node(node: NodeId, cost_table: &mut CostTable) -> ActDutilResult<&mut CostEntry> {
    let cost_node = cost_table.get_mut(node)?;
    cost_node.slack = cost_node.required_time - cost_node.arrival_time;
    Ok(cost_node)
}

pub fn get_slack_node(node: NodeId, cost_table: &CostTable) -> ActDutilResult<f64> {
    Ok(cost_table.get(node)?.slack)
}

pub fn find_critical_nodes(
    network: &DelayNetwork,
    cost_table: &mut CostTable,
    threshold_slack: f64,
) -> ActDutilResult<usize> {
    let mut critical_count = 0;
    for node_id in network.topological_order()? {
        let cost_node = cost_table.get_mut(node_id)?;
        cost_node.is_critical = cost_node.slack <= threshold_slack;
        critical_count += usize::from(cost_node.is_critical);
    }
    Ok(critical_count)
}

pub fn compute_area_delay_weight_network_for_collapse(
    network: &DelayNetwork,
    cost_table: &mut CostTable,
    mode: f32,
) -> ActDutilResult<Vec<CollapsiblePair>> {
    let mut pairs = Vec::new();
    for node_id in network.topological_order()? {
        if network.node(node_id)?.kind != NodeKind::Internal {
            continue;
        }

        let area_weight = {
            let cost_node = cost_table.get(node_id)?;
            compute_area_weight_node_for_collapse(network, node_id, cost_node)?
        };
        cost_table.get_mut(node_id)?.area_weight = area_weight;

        pairs.extend(compute_area_delay_weight_node_for_collapse(
            network, node_id, cost_table, mode,
        )?);
    }

    Ok(pairs)
}

pub fn compute_area_delay_weight_node_for_collapse(
    network: &DelayNetwork,
    node: NodeId,
    cost_table: &CostTable,
    mode: f32,
) -> ActDutilResult<Vec<CollapsiblePair>> {
    let cost_node = cost_table.get(node)?;
    if !cost_node.is_critical || network.node(node)?.kind != NodeKind::Internal {
        return Ok(Vec::new());
    }

    let mut pairs = Vec::new();
    for fanout in network.fanouts(node)? {
        if network.node(fanout)?.kind == NodeKind::PrimaryOutput {
            continue;
        }

        let cost_fanout = cost_table.get(fanout)?;
        if !cost_fanout.is_critical {
            continue;
        }

        let small_arrival_at_input_of_fanout =
            smallest_fanin_arrival_time_at_fanout_except_node(network, fanout, node, cost_table)?;
        if small_arrival_at_input_of_fanout >= f64::from(i32::MAX) {
            continue;
        }

        let largest_arrival_at_input_of_node =
            largest_fanin_arrival_time_at_node(network, node, cost_table)?;
        let diff = largest_arrival_at_input_of_node - small_arrival_at_input_of_fanout;
        if diff <= 0.0 {
            continue;
        }

        let node_name = network.node(node)?.name.clone();
        let fanout_name = network.node(fanout)?.name.clone();
        pairs.push(CollapsiblePair {
            nodename: node_name,
            fanoutname: fanout_name,
            weight: (cost_node.area_weight * (1.0 - f64::from(mode))) + (diff * f64::from(mode)),
        });
    }

    Ok(pairs)
}

pub fn compute_area_weight_node_for_collapse(
    network: &DelayNetwork,
    node: NodeId,
    cost_node: &CostEntry,
) -> ActDutilResult<f64> {
    if network.node(node)?.kind != NodeKind::Internal {
        return Ok(-1.0);
    }

    let fanout_count = network.fanout_count(node)?;
    if fanout_count == 0 {
        return Err(ActDutilError::ZeroFanout(node));
    }
    if fanout_count == 1 {
        return Ok(0.0);
    }

    Ok(f64::from(cost_node.cost))
}

pub fn allocate_collapsible_pair() -> CollapsiblePair {
    CollapsiblePair {
        nodename: String::new(),
        fanoutname: String::new(),
        weight: 0.0,
    }
}

pub fn smallest_fanin_arrival_time_at_fanout_except_node(
    network: &DelayNetwork,
    fanout: NodeId,
    node: NodeId,
    cost_table: &CostTable,
) -> ActDutilResult<f64> {
    let mut minimum = f64::from(i32::MAX);
    for fanin in network.node(fanout)?.fanins() {
        if *fanin == node {
            continue;
        }

        minimum = minimum.min(cost_table.get(*fanin)?.arrival_time);
    }
    Ok(minimum)
}

pub fn largest_fanin_arrival_time_at_node(
    network: &DelayNetwork,
    node: NodeId,
    cost_table: &CostTable,
) -> ActDutilResult<f64> {
    let mut maximum: f64 = -1.0;
    for fanin in network.node(node)?.fanins() {
        maximum = maximum.max(cost_table.get(*fanin)?.arrival_time);
    }
    Ok(maximum)
}

pub fn compare_collapsible_pairs(left: &CollapsiblePair, right: &CollapsiblePair) -> Ordering {
    left.weight
        .partial_cmp(&right.weight)
        .unwrap_or(Ordering::Equal)
}

pub fn get_max_arrival_time(network: &DelayNetwork, cost_table: &CostTable) -> ActDutilResult<f64> {
    let mut max_arrival_time: f64 = -1.0;
    for po in network.primary_outputs() {
        max_arrival_time = max_arrival_time.max(get_arrival_time(po, cost_table)?);
    }
    Ok(max_arrival_time)
}

pub fn assign_topol_indices_network(
    network: &DelayNetwork,
) -> ActDutilResult<BTreeMap<String, TopologicalEntry>> {
    let mut topol_table = BTreeMap::new();
    for (index, node_id) in network.topological_order()?.into_iter().enumerate() {
        let nodename = network.node(node_id)?.name.clone();
        topol_table.insert(nodename.clone(), TopologicalEntry { nodename, index });
    }
    Ok(topol_table)
}

pub fn topol_sort_fanins(
    network: &DelayNetwork,
    node: NodeId,
    topol_table: &BTreeMap<String, TopologicalEntry>,
) -> ActDutilResult<Vec<NodeId>> {
    let mut fanins = Vec::new();
    for fanin in network.node(node)?.fanins() {
        let name = &network.node(*fanin)?.name;
        let topol_fanin = topol_table
            .get(name)
            .ok_or(ActDutilError::MissingTopologicalEntry(*fanin))?;
        fanins.push((topol_fanin.index, topol_fanin.nodename.clone()));
    }

    fanins.sort_by_key(|(index, _)| *index);
    fanins
        .into_iter()
        .map(|(_, name)| {
            network
                .find_node_by_name(&name)
                .ok_or(ActDutilError::MissingNodeForTopologicalName(name))
        })
        .collect()
}

pub fn compare_topological_entries(left: &TopologicalEntry, right: &TopologicalEntry) -> Ordering {
    left.index.cmp(&right.index)
}

fn validate_delay_table(delay_values: &[f64]) -> ActDutilResult<()> {
    if delay_values.len() < 2 {
        return Err(ActDutilError::InvalidDelayTable {
            len: delay_values.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> (DelayNetwork, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let mut network = DelayNetwork::new();
        let a = network.add_node(DelayNode::primary_input("a").with_arrival_rise(1.5));
        let b = network.add_node(DelayNode::primary_input("b"));
        let x = network.add_node(DelayNode::internal("x", vec![a, b]));
        let y = network.add_node(DelayNode::internal("y", vec![x, b]));
        let out = network.add_node(DelayNode::primary_output("out", y).with_required_rise(9.0));
        (network, a, b, x, y, out)
    }

    #[test]
    fn delay_for_fanout_uses_table_or_linear_extrapolation() {
        let delays = [0.0, 2.0, 3.5, 5.0];

        assert_eq!(delay_for_fanout(&delays, 2), Ok(3.5));
        assert_eq!(delay_for_fanout(&delays, 5), Ok(8.0));
        assert_eq!(
            delay_for_fanout(&[1.0], 1),
            Err(ActDutilError::InvalidDelayTable { len: 1 })
        );
    }

    #[test]
    fn forward_trace_sets_pi_internal_and_output_arrivals() {
        let (network, a, b, x, y, out) = sample_network();
        let mut table = CostTable::new();

        let report = delay_trace_forward(&network, &mut table, &[0.0, 2.0, 3.0]).unwrap();

        assert_eq!(get_arrival_time(a, &table), Ok(1.5));
        assert_eq!(get_arrival_time(b, &table), Ok(0.0));
        assert_eq!(get_arrival_time(x, &table), Ok(3.5));
        assert_eq!(get_arrival_time(y, &table), Ok(5.5));
        assert_eq!(get_arrival_time(out, &table), Ok(5.5));
        assert_eq!(
            report,
            ForwardDelayReport {
                slowest_output: Some(out),
                max_output_arrival_time: 5.5,
            }
        );
    }

    #[test]
    fn backward_trace_uses_output_requirements_and_fanout_delays() {
        let (network, a, b, x, y, out) = sample_network();
        let mut table = CostTable::new();
        delay_trace_forward(&network, &mut table, &[0.0, 2.0, 3.0]).unwrap();

        delay_trace_backward(&network, &mut table, &[0.0, 2.0, 3.0]).unwrap();

        assert_eq!(get_required_time(out, &table), Ok(9.0));
        assert_eq!(get_required_time(y, &table), Ok(9.0));
        assert_eq!(get_required_time(x, &table), Ok(7.0));
        assert_eq!(get_required_time(a, &table), Ok(5.0));
        assert_eq!(get_required_time(b, &table), Ok(5.0));
    }

    #[test]
    fn backward_trace_defaults_unset_outputs_to_max_arrival() {
        let mut network = DelayNetwork::new();
        let a = network.add_node(DelayNode::primary_input("a"));
        let x = network.add_node(DelayNode::internal("x", vec![a]));
        let out = network.add_node(DelayNode::primary_output("out", x));
        let mut table = CostTable::new();
        delay_trace_forward(&network, &mut table, &[0.0, 4.0]).unwrap();

        delay_trace_backward(&network, &mut table, &[0.0, 4.0]).unwrap();

        assert_eq!(get_required_time(out, &table), Ok(4.0));
        assert_eq!(get_required_time(x, &table), Ok(4.0));
        assert_eq!(get_required_time(a, &table), Ok(0.0));
    }

    #[test]
    fn cost_table_helpers_match_c_defaults_and_cost_delay_formula() {
        let mut table = CostTable::new();
        let entry = set_arrival_time(NodeId(1), &mut table, 7.0);
        assert_eq!(entry.required_time, -1.0);
        entry.cost = 11;

        assert_eq!(cost_delay(entry, 0.25), 10.0);
        entry.cost_and_arrival_time = 3.0;
        assert_eq!(cost_delay(entry, 0.25), 3.0);
        invalidate_cost_and_arrival_time(entry);
        assert_eq!(entry.cost_and_arrival_time, -1.0);
    }

    #[test]
    fn slack_and_critical_flags_are_computed_from_arrival_and_required() {
        let (network, a, _, x, y, out) = sample_network();
        let mut table = CostTable::new();
        delay_trace_forward(&network, &mut table, &[0.0, 2.0, 3.0]).unwrap();
        delay_trace_backward(&network, &mut table, &[0.0, 2.0, 3.0]).unwrap();

        set_slack_network(&network, &mut table).unwrap();
        assert_eq!(get_slack_node(out, &table), Ok(3.5));
        assert_eq!(get_slack_node(y, &table), Ok(3.5));
        assert_eq!(get_slack_node(x, &table), Ok(3.5));
        assert_eq!(get_slack_node(a, &table), Ok(3.5));

        assert_eq!(find_critical_nodes(&network, &mut table, 3.5).unwrap(), 4);
        assert!(table.get(x).unwrap().is_critical);
    }

    #[test]
    fn area_delay_weights_consider_only_critical_internal_fanouts() {
        let (network, a, b, x, y, _) = sample_network();
        let mut table = CostTable::new();
        set_arrival_time(a, &mut table, 5.0);
        set_arrival_time(b, &mut table, 1.0);
        set_arrival_time(x, &mut table, 8.0);
        set_arrival_time(y, &mut table, 9.0);
        set_arrival_time(NodeId(4), &mut table, 9.0);
        table.set_cost(x, 6);
        table.get_mut(x).unwrap().slack = 0.0;
        table.get_mut(y).unwrap().slack = 0.0;
        find_critical_nodes(&network, &mut table, 0.0).unwrap();

        let pairs =
            compute_area_delay_weight_network_for_collapse(&network, &mut table, 0.5).unwrap();

        assert_eq!(
            pairs,
            vec![CollapsiblePair {
                nodename: "x".to_owned(),
                fanoutname: "y".to_owned(),
                weight: 2.0,
            }]
        );
    }

    #[test]
    fn topological_helpers_store_names_and_sort_current_fanins() {
        let (network, a, b, x, y, _) = sample_network();
        let table = assign_topol_indices_network(&network).unwrap();

        assert_eq!(table["x"].index, x.0);
        assert_eq!(topol_sort_fanins(&network, y, &table).unwrap(), vec![b, x]);
        assert_eq!(topol_sort_fanins(&network, x, &table).unwrap(), vec![a, b]);
    }

    #[test]
    fn fanout_delay_and_delay_correction_follow_legacy_counting() {
        let delays = [0.0, 2.0, 3.0];

        assert_eq!(
            get_bddfanout_delay(
                ActVertexFanout {
                    multiple_fanouts: 2,
                },
                &delays,
            ),
            Ok(4.0)
        );
        assert_eq!(get_node_delay_correction(&delays, 1, 3), Ok(2.0));
        assert_eq!(get_node_delay_correction(&delays, 0, 3), Ok(0.0));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("act_dutil.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
