//! Native Rust model for ITE partial-collapse decisions.
//!
//! This module keeps the collapse driver in owned Rust data structures. Direct
//! SIS-backed simplification, remapping, and ACT graph relabeling are exposed as
//! dependency errors until their native ports can be composed here.

use std::collections::{HashMap, HashSet};
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
pub enum CollapseMethod {
    Existing,
    New,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollapseUpdate {
    Cheap,
    Expensive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapMethod {
    Old,
    New,
    WithIter,
    WithJustDecomp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActIteInitParam {
    pub fanin_collapse: usize,
    pub collapse_fanins_of_fanout: usize,
    pub cost_limit: i32,
    pub collapse_method: CollapseMethod,
    pub collapse_update: CollapseUpdate,
    pub map_method: MapMethod,
}

impl Default for ActIteInitParam {
    fn default() -> Self {
        Self {
            fanin_collapse: usize::MAX,
            collapse_fanins_of_fanout: usize::MAX,
            cost_limit: i32::MAX,
            collapse_method: CollapseMethod::Existing,
            collapse_update: CollapseUpdate::Cheap,
            map_method: MapMethod::Old,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteCollapseNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub cost: ActIteCost,
    deleted: bool,
}

impl IteCollapseNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            cost: ActIteCost::default(),
            deleted: false,
        }
    }

    pub fn internal(name: impl Into<String>, fanins: Vec<NodeId>, cost: i32) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Internal,
            fanins,
            cost: ActIteCost::new(cost),
            deleted: false,
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self
    }

    pub fn with_cost(mut self, cost: ActIteCost) -> Self {
        self.cost = cost;
        self
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActIteCost {
    pub cost: i32,
    pub node: Option<NodeId>,
    pub match_present: bool,
    pub ite: Option<IteGraph>,
    pub act: Option<ActGraph>,
}

impl ActIteCost {
    pub fn new(cost: i32) -> Self {
        Self {
            cost,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IteVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IteValue {
    Zero,
    One,
    Literal,
    IfThenElse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteVertex {
    pub value: IteValue,
    pub fanin: Option<NodeId>,
    pub name: Option<String>,
    pub node: Option<NodeId>,
    pub children: Vec<IteVertexId>,
}

impl IteVertex {
    pub fn literal(fanin: NodeId) -> Self {
        Self {
            value: IteValue::Literal,
            fanin: Some(fanin),
            name: None,
            node: None,
            children: Vec::new(),
        }
    }

    pub fn ite(children: Vec<IteVertexId>) -> Self {
        Self {
            value: IteValue::IfThenElse,
            fanin: None,
            name: None,
            node: None,
            children,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IteGraph {
    vertices: Vec<IteVertex>,
    root: Option<IteVertexId>,
}

impl IteGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vertex(&mut self, vertex: IteVertex) -> IteVertexId {
        let id = IteVertexId(self.vertices.len());
        self.vertices.push(vertex);
        if self.root.is_none() {
            self.root = Some(id);
        }
        id
    }

    pub fn set_root(&mut self, root: IteVertexId) -> Result<(), IteCollapseError> {
        self.vertex(root)?;
        self.root = Some(root);
        Ok(())
    }

    pub fn vertex(&self, id: IteVertexId) -> Result<&IteVertex, IteCollapseError> {
        self.vertices
            .get(id.0)
            .ok_or(IteCollapseError::MissingIteVertex(id))
    }

    pub fn vertex_mut(&mut self, id: IteVertexId) -> Result<&mut IteVertex, IteCollapseError> {
        self.vertices
            .get_mut(id.0)
            .ok_or(IteCollapseError::MissingIteVertex(id))
    }

    pub fn put_node_names(&mut self, network: &IteCollapseNetwork) -> Result<(), IteCollapseError> {
        let Some(root) = self.root else {
            return Ok(());
        };
        let mut seen = HashSet::new();
        self.put_node_names_from(root, network, &mut seen)
    }

    fn put_node_names_from(
        &mut self,
        vertex: IteVertexId,
        network: &IteCollapseNetwork,
        seen: &mut HashSet<IteVertexId>,
    ) -> Result<(), IteCollapseError> {
        if !seen.insert(vertex) {
            return Ok(());
        }

        let children = {
            let vertex_ref = self.vertex_mut(vertex)?;
            if vertex_ref.value == IteValue::Literal {
                let fanin = vertex_ref
                    .fanin
                    .ok_or(IteCollapseError::MissingLiteralFanin(vertex))?;
                vertex_ref.name = Some(network.node(fanin)?.name.clone());
            }
            vertex_ref.children.clone()
        };

        for child in children {
            self.put_node_names_from(child, network, seen)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActGraph {
    pub root_node: Option<NodeId>,
    pub literal_nodes: Vec<NodeId>,
    pub literal_names: Vec<String>,
}

impl ActGraph {
    pub fn put_node_names(&mut self, network: &IteCollapseNetwork) -> Result<(), IteCollapseError> {
        self.literal_names.clear();
        for node in &self.literal_nodes {
            self.literal_names.push(network.node(*node)?.name.clone());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IteCollapseNetwork {
    nodes: Vec<IteCollapseNode>,
    names: HashMap<String, NodeId>,
}

impl IteCollapseNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, mut node: IteCollapseNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.cost.node = Some(id);
        self.names.insert(node.name.clone(), id);
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Result<&IteCollapseNode, IteCollapseError> {
        self.nodes
            .get(id.0)
            .ok_or(IteCollapseError::MissingNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut IteCollapseNode, IteCollapseError> {
        self.nodes
            .get_mut(id.0)
            .ok_or(IteCollapseError::MissingNode(id))
    }

    pub fn find_node(&self, name: &str) -> Option<NodeId> {
        self.names.get(name).copied()
    }

    pub fn active_node_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (!node.deleted).then_some(NodeId(index)))
            .collect()
    }

    pub fn internal_node_ids(&self) -> Vec<NodeId> {
        self.active_node_ids()
            .into_iter()
            .filter(|id| self.nodes[id.0].kind == NodeKind::Internal)
            .collect()
    }

    pub fn fanouts(&self, node: NodeId) -> Result<Vec<NodeId>, IteCollapseError> {
        self.node(node)?;
        Ok(self
            .active_node_ids()
            .into_iter()
            .filter(|candidate| self.nodes[candidate.0].fanins.contains(&node))
            .collect())
    }

    pub fn has_primary_output_fanout(&self, node: NodeId) -> Result<bool, IteCollapseError> {
        Ok(self
            .fanouts(node)?
            .into_iter()
            .any(|fanout| self.nodes[fanout.0].kind == NodeKind::PrimaryOutput))
    }

    pub fn num_composite_fanin(
        &self,
        collapsed: NodeId,
        fanout: NodeId,
    ) -> Result<usize, IteCollapseError> {
        let collapsed_node = self.node(collapsed)?;
        let fanout_node = self.node(fanout)?;
        if !fanout_node.fanins.contains(&collapsed) {
            return Err(IteCollapseError::NotAFanin {
                fanout,
                fanin: collapsed,
            });
        }

        let mut composite = 0;
        for fanin in &fanout_node.fanins {
            if *fanin == collapsed {
                composite += collapsed_node.fanins.len();
            } else {
                composite += 1;
            }
        }
        Ok(composite)
    }

    pub fn replace_fanin_with_fanins(
        &mut self,
        fanout: NodeId,
        collapsed: NodeId,
    ) -> Result<(), IteCollapseError> {
        let replacement_fanins = self.node(collapsed)?.fanins.clone();
        let fanout_node = self.node_mut(fanout)?;
        if fanout_node.deleted {
            return Err(IteCollapseError::DeletedNode(fanout));
        }
        if !fanout_node.fanins.contains(&collapsed) {
            return Err(IteCollapseError::NotAFanin {
                fanout,
                fanin: collapsed,
            });
        }

        let mut revised = Vec::new();
        for fanin in fanout_node.fanins.iter().copied() {
            if fanin == collapsed {
                for replacement in &replacement_fanins {
                    if *replacement != fanout && !revised.contains(replacement) {
                        revised.push(*replacement);
                    }
                }
            } else if !revised.contains(&fanin) {
                revised.push(fanin);
            }
        }
        fanout_node.fanins = revised;
        Ok(())
    }

    pub fn replace_node_cost(
        &mut self,
        node: NodeId,
        mut cost: ActIteCost,
    ) -> Result<(), IteCollapseError> {
        cost.node = Some(node);
        let node_ref = self.node_mut(node)?;
        node_ref.cost = cost;
        Ok(())
    }

    pub fn delete_node(&mut self, node: NodeId) -> Result<(), IteCollapseError> {
        let node_ref = self.node_mut(node)?;
        node_ref.deleted = true;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollapsedFanout {
    pub fanout: NodeId,
    pub cost: ActIteCost,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollapseAttempt {
    pub node: NodeId,
    pub fanouts: Vec<NodeId>,
    pub gain: i32,
    pub accepted: bool,
    pub deleted_node: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialCollapseReport {
    pub attempts: Vec<CollapseAttempt>,
    pub total_gain: i32,
    pub scores: HashMap<NodeId, i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteCollapseError {
    MissingNode(NodeId),
    DeletedNode(NodeId),
    MissingIteVertex(IteVertexId),
    MissingLiteralFanin(IteVertexId),
    NotAFanin { fanout: NodeId, fanin: NodeId },
    UnknownMappedFanout(NodeId),
    UnknownMapMethod(MapMethod),
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for IteCollapseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "missing ITE collapse node {}", node.0),
            Self::DeletedNode(node) => write!(f, "ITE collapse node {} was deleted", node.0),
            Self::MissingIteVertex(vertex) => write!(f, "missing ITE vertex {}", vertex.0),
            Self::MissingLiteralFanin(vertex) => {
                write!(f, "ITE literal vertex {} has no fanin", vertex.0)
            }
            Self::NotAFanin { fanout, fanin } => {
                write!(f, "node {} is not a fanin of {}", fanin.0, fanout.0)
            }
            Self::UnknownMappedFanout(fanout) => {
                write!(f, "collapse mapper did not return fanout {}", fanout.0)
            }
            Self::UnknownMapMethod(method) => write!(f, "mapping method {method:?} is not known"),
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
        }
    }
}

impl Error for IteCollapseError {}

pub fn act_ite_partial_collapse_blocked<Network>(
    _network: &mut Network,
    _init_param: &ActIteInitParam,
) -> Result<i32, IteCollapseError> {
    Err(missing_native_ports("act_ite_partial_collapse"))
}

pub fn act_ite_partial_collapse_node_blocked<Network, Node>(
    _network: &mut Network,
    _node: &Node,
    _init_param: &ActIteInitParam,
) -> Result<i32, IteCollapseError> {
    Err(missing_native_ports("act_ite_partial_collapse_node"))
}

pub fn assign_score_node(
    network: &IteCollapseNetwork,
    node: NodeId,
    scores: &mut HashMap<NodeId, i32>,
    init_param: &ActIteInitParam,
) -> Result<i32, IteCollapseError> {
    let node_ref = network.node(node)?;
    if node_ref.kind != NodeKind::Internal {
        return Ok(-1);
    }

    let score = if node_ref.fanins.len() > init_param.fanin_collapse
        || network.has_primary_output_fanout(node)?
        || node_ref.cost.cost > init_param.cost_limit
    {
        0
    } else {
        let cost = node_ref.cost.cost.max(1);
        let fanout_cost = network
            .fanouts(node)?
            .into_iter()
            .map(|fanout| network.node(fanout).map(|node| node.cost.cost))
            .try_fold(0_i32, |sum, cost| cost.map(|cost| sum.saturating_add(cost)))?;
        fanout_cost.div_euclid(cost)
    };

    scores.insert(node, score);
    Ok(score)
}

pub fn assign_score_network(
    network: &IteCollapseNetwork,
    scores: &mut HashMap<NodeId, i32>,
    init_param: &ActIteInitParam,
) -> Result<(), IteCollapseError> {
    for node in network.active_node_ids() {
        assign_score_node(network, node, scores, init_param)?;
    }
    Ok(())
}

pub fn find_max_score(
    network: &IteCollapseNetwork,
    scores: &HashMap<NodeId, i32>,
) -> Result<Option<(NodeId, i32)>, IteCollapseError> {
    let mut max = None;
    for node in network.internal_node_ids() {
        let score = *scores.get(&node).unwrap_or(&0);
        if max.is_none_or(|(_, max_score)| score > max_score) {
            max = Some((node, score));
        }
    }
    Ok(max)
}

pub fn partial_collapse_network<F>(
    network: &mut IteCollapseNetwork,
    init_param: &ActIteInitParam,
    mut map_collapsed_fanouts: F,
) -> Result<PartialCollapseReport, IteCollapseError>
where
    F: FnMut(
        &IteCollapseNetwork,
        NodeId,
        &[NodeId],
        &ActIteInitParam,
    ) -> Result<Vec<CollapsedFanout>, IteCollapseError>,
{
    if init_param.fanin_collapse == 0 {
        return Ok(PartialCollapseReport {
            attempts: Vec::new(),
            total_gain: 0,
            scores: HashMap::new(),
        });
    }

    let mut scores = HashMap::new();
    assign_score_network(network, &mut scores, init_param)?;
    let mut attempts = Vec::new();
    let mut total_gain = 0;

    loop {
        let Some((node, score)) = find_max_score(network, &scores)? else {
            break;
        };
        if score == 0 {
            break;
        }

        let attempt = partial_collapse_node(
            network,
            node,
            init_param,
            &mut scores,
            &mut map_collapsed_fanouts,
        )?;
        if attempt.accepted {
            total_gain += attempt.gain;
        }
        attempts.push(attempt);
    }

    Ok(PartialCollapseReport {
        attempts,
        total_gain,
        scores,
    })
}

pub fn partial_collapse_node<F>(
    network: &mut IteCollapseNetwork,
    node: NodeId,
    init_param: &ActIteInitParam,
    scores: &mut HashMap<NodeId, i32>,
    mut map_collapsed_fanouts: F,
) -> Result<CollapseAttempt, IteCollapseError>
where
    F: FnMut(
        &IteCollapseNetwork,
        NodeId,
        &[NodeId],
        &ActIteInitParam,
    ) -> Result<Vec<CollapsedFanout>, IteCollapseError>,
{
    match init_param.map_method {
        MapMethod::Old | MapMethod::New | MapMethod::WithIter | MapMethod::WithJustDecomp => {}
    }

    if !fanout_limit_allows_collapse(network, node, init_param)? {
        scores.insert(node, 0);
        return Ok(CollapseAttempt {
            node,
            fanouts: network.fanouts(node)?,
            gain: 0,
            accepted: false,
            deleted_node: false,
        });
    }

    let fanouts = network.fanouts(node)?;
    let mapped_fanouts = map_collapsed_fanouts(network, node, &fanouts, init_param)?;
    match init_param.collapse_method {
        CollapseMethod::Existing => {
            collapse_node_existing(network, node, init_param, scores, fanouts, mapped_fanouts)
        }
        CollapseMethod::New => {
            collapse_node_new(network, node, init_param, scores, fanouts, mapped_fanouts)
        }
    }
}

pub fn update_ite_fields(
    network: &IteCollapseNetwork,
    node: NodeId,
    cost: &mut ActIteCost,
) -> Result<(), IteCollapseError> {
    cost.node = Some(node);
    if cost.match_present {
        return Ok(());
    }

    if let Some(graph) = cost.ite.as_mut() {
        if let Some(root) = graph.root {
            graph.vertex_mut(root)?.node = Some(node);
        }
        graph.put_node_names(network)?;
        if cost.act.is_some() {
            return Err(missing_native_ports("ambiguous ITE/ACT cost slot"));
        }
        return Ok(());
    }

    if let Some(graph) = cost.act.as_mut() {
        graph.root_node = Some(node);
        graph.put_node_names(network)?;
        return Ok(());
    }

    Ok(())
}

pub fn put_node_names_in_ite(
    graph: &mut IteGraph,
    network: &IteCollapseNetwork,
) -> Result<(), IteCollapseError> {
    graph.put_node_names(network)
}

fn collapse_node_existing(
    network: &mut IteCollapseNetwork,
    node: NodeId,
    init_param: &ActIteInitParam,
    scores: &mut HashMap<NodeId, i32>,
    fanouts: Vec<NodeId>,
    mapped_fanouts: Vec<CollapsedFanout>,
) -> Result<CollapseAttempt, IteCollapseError> {
    let mut gain = network.node(node)?.cost.cost;
    for fanout in &fanouts {
        gain = gain.saturating_add(network.node(*fanout)?.cost.cost);
    }

    let mut mapped_by_fanout = mapped_fanouts
        .into_iter()
        .map(|entry| (entry.fanout, entry.cost))
        .collect::<HashMap<_, _>>();
    let mut replacements = Vec::with_capacity(fanouts.len());
    for fanout in &fanouts {
        let mut cost = mapped_by_fanout
            .remove(fanout)
            .ok_or(IteCollapseError::UnknownMappedFanout(*fanout))?;
        update_ite_fields(network, *fanout, &mut cost)?;
        gain = gain.saturating_sub(cost.cost);
        replacements.push((*fanout, cost));
        if gain <= 0 {
            scores.insert(node, 0);
            return Ok(CollapseAttempt {
                node,
                fanouts,
                gain,
                accepted: false,
                deleted_node: false,
            });
        }
    }

    apply_replacements(network, &replacements, node)?;
    rescore_after_collapse(network, node, &fanouts, init_param, scores, true)?;
    network.delete_node(node)?;
    Ok(CollapseAttempt {
        node,
        fanouts,
        gain,
        accepted: true,
        deleted_node: true,
    })
}

fn collapse_node_new(
    network: &mut IteCollapseNetwork,
    node: NodeId,
    init_param: &ActIteInitParam,
    scores: &mut HashMap<NodeId, i32>,
    fanouts: Vec<NodeId>,
    mapped_fanouts: Vec<CollapsedFanout>,
) -> Result<CollapseAttempt, IteCollapseError> {
    let mut gain_bad = network.node(node)?.cost.cost;
    let mut gain_good = 0;
    let mut good = Vec::new();
    let mut bad = Vec::new();
    let mut mapped_by_fanout = mapped_fanouts
        .into_iter()
        .map(|entry| (entry.fanout, entry.cost))
        .collect::<HashMap<_, _>>();

    for fanout in &fanouts {
        let mut new_cost = mapped_by_fanout
            .remove(fanout)
            .ok_or(IteCollapseError::UnknownMappedFanout(*fanout))?;
        update_ite_fields(network, *fanout, &mut new_cost)?;
        let gain_node = network.node(*fanout)?.cost.cost - new_cost.cost;
        if gain_node > 0 {
            gain_good += gain_node;
            good.push((*fanout, new_cost));
        } else {
            gain_bad += gain_node;
            bad.push((*fanout, new_cost));
        }
    }

    if gain_bad <= 0 {
        scores.insert(node, 0);
    } else {
        apply_replacements(network, &bad, node)?;
    }
    apply_replacements(network, &good, node)?;

    let deleted_node = gain_bad > 0;
    if deleted_node {
        network.delete_node(node)?;
    }

    let changed_fanouts = good
        .iter()
        .chain(if gain_bad > 0 { bad.iter() } else { [].iter() })
        .map(|(fanout, _)| *fanout)
        .collect::<Vec<_>>();
    rescore_after_collapse(
        network,
        node,
        &changed_fanouts,
        init_param,
        scores,
        deleted_node,
    )?;

    let gain = gain_good + if gain_bad > 0 { gain_bad } else { 0 };
    Ok(CollapseAttempt {
        node,
        fanouts: changed_fanouts,
        gain,
        accepted: gain > 0,
        deleted_node,
    })
}

fn apply_replacements(
    network: &mut IteCollapseNetwork,
    replacements: &[(NodeId, ActIteCost)],
    collapsed: NodeId,
) -> Result<(), IteCollapseError> {
    for (fanout, cost) in replacements {
        network.replace_fanin_with_fanins(*fanout, collapsed)?;
        network.replace_node_cost(*fanout, cost.clone())?;
    }
    Ok(())
}

fn fanout_limit_allows_collapse(
    network: &IteCollapseNetwork,
    node: NodeId,
    init_param: &ActIteInitParam,
) -> Result<bool, IteCollapseError> {
    for fanout in network.fanouts(node)? {
        if network.num_composite_fanin(node, fanout)? > init_param.collapse_fanins_of_fanout {
            return Ok(false);
        }
    }
    Ok(true)
}

fn rescore_after_collapse(
    network: &IteCollapseNetwork,
    collapsed: NodeId,
    changed_fanouts: &[NodeId],
    init_param: &ActIteInitParam,
    scores: &mut HashMap<NodeId, i32>,
    include_collapsed_fanins: bool,
) -> Result<(), IteCollapseError> {
    for fanout in changed_fanouts {
        if !network.node(*fanout)?.deleted {
            assign_score_node(network, *fanout, scores, init_param)?;
        }
    }

    if init_param.collapse_update != CollapseUpdate::Expensive {
        return Ok(());
    }

    let mut update_nodes = HashSet::new();
    if include_collapsed_fanins {
        for fanin in &network.node(collapsed)?.fanins {
            update_nodes.insert(*fanin);
        }
    }
    for fanout in changed_fanouts {
        for fanin in &network.node(*fanout)?.fanins {
            update_nodes.insert(*fanin);
        }
    }

    for candidate in update_nodes {
        if network
            .node(candidate)
            .is_ok_and(|node| node.kind == NodeKind::Internal && !node.deleted)
        {
            assign_score_node(network, candidate, scores, init_param)?;
        }
    }
    Ok(())
}

fn missing_native_ports(operation: &'static str) -> IteCollapseError {
    IteCollapseError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> (IteCollapseNetwork, NodeId, NodeId, NodeId, NodeId) {
        let mut network = IteCollapseNetwork::new();
        let a = network.add_node(IteCollapseNode::new("a", NodeKind::PrimaryInput));
        let b = network.add_node(IteCollapseNode::new("b", NodeKind::PrimaryInput));
        let x = network.add_node(IteCollapseNode::internal("x", vec![a, b], 2));
        let y = network.add_node(IteCollapseNode::internal("y", vec![x, b], 5));
        let z = network.add_node(IteCollapseNode::internal("z", vec![x, y], 4));
        (network, x, y, z, b)
    }

    fn params(method: CollapseMethod) -> ActIteInitParam {
        ActIteInitParam {
            fanin_collapse: 4,
            collapse_fanins_of_fanout: 8,
            cost_limit: 20,
            collapse_method: method,
            collapse_update: CollapseUpdate::Expensive,
            map_method: MapMethod::Old,
        }
    }

    #[test]
    fn assign_score_zeroes_primary_output_fanout_cost_limit_and_fanin_limit() {
        let mut network = IteCollapseNetwork::new();
        let a = network.add_node(IteCollapseNode::new("a", NodeKind::PrimaryInput));
        let x = network.add_node(IteCollapseNode::internal("x", vec![a], 3));
        network.add_node(IteCollapseNode::new("out", NodeKind::PrimaryOutput).with_fanins(vec![x]));
        let mut scores = HashMap::new();

        assert_eq!(
            assign_score_node(&network, x, &mut scores, &params(CollapseMethod::Existing)),
            Ok(0)
        );
        assert_eq!(scores[&x], 0);

        let mut limited = params(CollapseMethod::Existing);
        limited.fanin_collapse = 0;
        assert_eq!(assign_score_node(&network, x, &mut scores, &limited), Ok(0));

        let mut cost_limited = params(CollapseMethod::Existing);
        cost_limited.cost_limit = 2;
        assert_eq!(
            assign_score_node(&network, x, &mut scores, &cost_limited),
            Ok(0)
        );
    }

    #[test]
    fn find_max_score_returns_highest_internal_score() {
        let (network, x, y, z, _) = sample_network();
        let scores = HashMap::from([(x, 1), (y, 7), (z, 3)]);

        assert_eq!(find_max_score(&network, &scores).unwrap(), Some((y, 7)));
    }

    #[test]
    fn existing_collapse_rejects_when_running_gain_becomes_non_positive() {
        let (mut network, x, y, z, _) = sample_network();
        let mut scores = HashMap::new();

        let attempt = partial_collapse_node(
            &mut network,
            x,
            &params(CollapseMethod::Existing),
            &mut scores,
            |_, _, fanouts, _| {
                Ok(fanouts
                    .iter()
                    .map(|fanout| CollapsedFanout {
                        fanout: *fanout,
                        cost: ActIteCost::new(20),
                    })
                    .collect())
            },
        )
        .unwrap();

        assert!(!attempt.accepted);
        assert_eq!(scores[&x], 0);
        assert_eq!(network.node(y).unwrap().fanins, vec![x, NodeId(1)]);
        assert_eq!(network.node(z).unwrap().fanins, vec![x, y]);
        assert!(!network.node(x).unwrap().is_deleted());
    }

    #[test]
    fn existing_collapse_accepts_positive_gain_replaces_all_fanouts_and_deletes_node() {
        let (mut network, x, y, z, b) = sample_network();
        let mut scores = HashMap::new();

        let attempt = partial_collapse_node(
            &mut network,
            x,
            &params(CollapseMethod::Existing),
            &mut scores,
            |_, _, fanouts, _| {
                Ok(fanouts
                    .iter()
                    .map(|fanout| CollapsedFanout {
                        fanout: *fanout,
                        cost: ActIteCost::new(1),
                    })
                    .collect())
            },
        )
        .unwrap();

        assert!(attempt.accepted);
        assert!(attempt.deleted_node);
        assert_eq!(attempt.gain, 9);
        assert_eq!(network.node(y).unwrap().fanins, vec![NodeId(0), b]);
        assert_eq!(network.node(z).unwrap().fanins, vec![NodeId(0), b, y]);
        assert_eq!(network.node(y).unwrap().cost.cost, 1);
        assert!(network.node(x).unwrap().is_deleted());
    }

    #[test]
    fn new_collapse_keeps_node_when_only_good_fanouts_are_beneficial() {
        let (mut network, x, y, z, b) = sample_network();
        let mut scores = HashMap::new();

        let attempt = partial_collapse_node(
            &mut network,
            x,
            &params(CollapseMethod::New),
            &mut scores,
            |_, _, _, _| {
                Ok(vec![
                    CollapsedFanout {
                        fanout: y,
                        cost: ActIteCost::new(1),
                    },
                    CollapsedFanout {
                        fanout: z,
                        cost: ActIteCost::new(10),
                    },
                ])
            },
        )
        .unwrap();

        assert_eq!(attempt.gain, 4);
        assert!(attempt.accepted);
        assert!(!attempt.deleted_node);
        assert_eq!(attempt.fanouts, vec![y]);
        assert_eq!(network.node(y).unwrap().fanins, vec![NodeId(0), b]);
        assert_eq!(network.node(z).unwrap().fanins, vec![x, y]);
        assert!(!network.node(x).unwrap().is_deleted());
        assert_eq!(scores[&x], 0);
    }

    #[test]
    fn new_collapse_deletes_node_when_bad_bucket_still_has_positive_gain() {
        let (mut network, x, y, z, b) = sample_network();
        let mut scores = HashMap::new();

        let attempt = partial_collapse_node(
            &mut network,
            x,
            &params(CollapseMethod::New),
            &mut scores,
            |_, _, _, _| {
                Ok(vec![
                    CollapsedFanout {
                        fanout: y,
                        cost: ActIteCost::new(1),
                    },
                    CollapsedFanout {
                        fanout: z,
                        cost: ActIteCost::new(5),
                    },
                ])
            },
        )
        .unwrap();

        assert_eq!(attempt.gain, 5);
        assert!(attempt.deleted_node);
        assert_eq!(attempt.fanouts, vec![y, z]);
        assert_eq!(network.node(z).unwrap().fanins, vec![NodeId(0), b, y]);
        assert!(network.node(x).unwrap().is_deleted());
    }

    #[test]
    fn fanout_composite_limit_prevents_mapping_attempt() {
        let (mut network, x, _, _, _) = sample_network();
        let mut scores = HashMap::new();
        let mut limited = params(CollapseMethod::Existing);
        limited.collapse_fanins_of_fanout = 1;
        let mut called = false;

        let attempt =
            partial_collapse_node(&mut network, x, &limited, &mut scores, |_, _, _, _| {
                called = true;
                Ok(Vec::new())
            })
            .unwrap();

        assert!(!called);
        assert!(!attempt.accepted);
        assert_eq!(scores[&x], 0);
    }

    #[test]
    fn partial_collapse_network_repeats_until_max_score_is_zero() {
        let (mut network, x, y, z, _) = sample_network();

        let report = partial_collapse_network(
            &mut network,
            &params(CollapseMethod::Existing),
            |network, _, fanouts, _| {
                Ok(fanouts
                    .iter()
                    .map(|fanout| CollapsedFanout {
                        fanout: *fanout,
                        cost: ActIteCost::new(if network.node(*fanout).unwrap().name == "z" {
                            1
                        } else {
                            1
                        }),
                    })
                    .collect())
            },
        )
        .unwrap();

        assert_eq!(
            report
                .attempts
                .iter()
                .map(|attempt| attempt.node)
                .collect::<Vec<_>>(),
            vec![x, y]
        );
        assert_eq!(report.total_gain, 10);
        assert!(network.node(x).unwrap().is_deleted());
        assert!(network.node(y).unwrap().is_deleted());
        assert!(!network.node(z).unwrap().is_deleted());
    }

    #[test]
    fn update_ite_fields_relabels_literal_names_and_root_node() {
        let (network, x, y, _, _) = sample_network();
        let mut graph = IteGraph::new();
        let lit = graph.add_vertex(IteVertex::literal(x));
        let root = graph.add_vertex(IteVertex::ite(vec![lit]));
        graph.set_root(root).unwrap();
        let mut cost = ActIteCost {
            cost: 1,
            ite: Some(graph),
            ..ActIteCost::default()
        };

        update_ite_fields(&network, y, &mut cost).unwrap();

        let graph = cost.ite.as_ref().unwrap();
        assert_eq!(graph.vertex(root).unwrap().node, Some(y));
        assert_eq!(graph.vertex(lit).unwrap().name.as_deref(), Some("x"));
    }

    #[test]
    fn act_field_relabeling_records_root_and_literal_names() {
        let (network, x, y, _, _) = sample_network();
        let mut cost = ActIteCost {
            cost: 1,
            act: Some(ActGraph {
                root_node: None,
                literal_nodes: vec![x],
                literal_names: Vec::new(),
            }),
            ..ActIteCost::default()
        };

        update_ite_fields(&network, y, &mut cost).unwrap();

        let graph = cost.act.as_ref().unwrap();
        assert_eq!(graph.root_node, Some(y));
        assert_eq!(graph.literal_names, vec!["x"]);
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_tokens_are_present_in_this_port() {
        let source = include_str!("ite_collapse.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
