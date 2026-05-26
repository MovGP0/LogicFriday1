//! Native Rust network utilities for the SIS network layer.
//!
//! The original utility file owns the basic network lifecycle, node membership,
//! PI/PO lists, name tables, duplication, and external don't-care attachment.
//! This port keeps those behaviors as safe owned Rust APIs. Full SIS runtime
//! integrations such as delay, mapper, latch, STG, ASTG, and BDD-manager object
//! lifetimes are represented by ordinary owned data hooks rather than legacy C
//! entry points.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NodeId(usize);

impl NodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    Unassigned,
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverValue {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    values: Vec<CoverValue>,
}

impl Cube {
    pub fn new(values: impl Into<Vec<CoverValue>>) -> Self {
        Self {
            values: values.into(),
        }
    }

    pub fn values(&self) -> &[CoverValue] {
        &self.values
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct SopCover {
    cubes: Vec<Cube>,
}

impl SopCover {
    pub fn new(cubes: impl Into<Vec<Cube>>) -> Self {
        Self {
            cubes: cubes.into(),
        }
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoolExpr {
    Constant(bool),
    Literal { node: NodeId, phase: bool },
    Not(Box<BoolExpr>),
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
}

impl BoolExpr {
    pub fn constant(value: bool) -> Self {
        Self::Constant(value)
    }

    pub fn literal(node: NodeId, phase: bool) -> Self {
        Self::Literal { node, phase }
    }

    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Constant(false), _) | (_, Self::Constant(false)) => Self::Constant(false),
            (Self::Constant(true), value) | (value, Self::Constant(true)) => value,
            (Self::And(mut left), Self::And(right)) => {
                left.extend(right);
                Self::And(left)
            }
            (Self::And(mut left), right) => {
                left.push(right);
                Self::And(left)
            }
            (left, Self::And(mut right)) => {
                let mut values = vec![left];
                values.append(&mut right);
                Self::And(values)
            }
            (left, right) => Self::And(vec![left, right]),
        }
    }

    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Constant(true), _) | (_, Self::Constant(true)) => Self::Constant(true),
            (Self::Constant(false), value) | (value, Self::Constant(false)) => value,
            (Self::Or(mut left), Self::Or(right)) => {
                left.extend(right);
                Self::Or(left)
            }
            (Self::Or(mut left), right) => {
                left.push(right);
                Self::Or(left)
            }
            (left, Self::Or(mut right)) => {
                let mut values = vec![left];
                values.append(&mut right);
                Self::Or(values)
            }
            (left, right) => Self::Or(vec![left, right]),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub name: String,
    pub short_name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub fanouts: BTreeSet<NodeId>,
    pub cover: Option<SopCover>,
    pub expression: Option<BoolExpr>,
}

impl NetworkNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        let name = name.into();
        Self {
            short_name: name.clone(),
            name,
            kind,
            fanins: Vec::new(),
            fanouts: BTreeSet::new(),
            cover: None,
            expression: None,
        }
    }

    pub fn with_cover(mut self, fanins: impl Into<Vec<NodeId>>, cover: SopCover) -> Self {
        self.fanins = fanins.into();
        self.cover = Some(cover);
        self
    }

    pub fn with_expression(mut self, expression: BoolExpr) -> Self {
        self.expression = Some(expression);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkUtilError {
    MissingNode(NodeId),
    MissingName(String),
    DuplicateName(String),
    DuplicateShortName(String),
    NodeBelongsToDifferentNetwork(NodeId),
    InvalidPrimaryOutput(NodeId),
    InvalidCover { node: NodeId, cube: usize },
    MissingDcNetwork,
    MissingDcAttachment(NodeId),
    CycleDetected,
}

impl fmt::Display for NetworkUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "missing network node {}", node.index()),
            Self::MissingName(name) => write!(f, "missing network node named {name}"),
            Self::DuplicateName(name) => write!(f, "duplicate network node name {name}"),
            Self::DuplicateShortName(name) => write!(f, "duplicate network node short name {name}"),
            Self::NodeBelongsToDifferentNetwork(node) => {
                write!(f, "node {} is not a member of this network", node.index())
            }
            Self::InvalidPrimaryOutput(node) => {
                write!(
                    f,
                    "primary output {} must have exactly one fanin",
                    node.index()
                )
            }
            Self::InvalidCover { node, cube } => {
                write!(f, "node {} has an invalid cover cube {cube}", node.index())
            }
            Self::MissingDcNetwork => write!(f, "network has no external don't-care network"),
            Self::MissingDcAttachment(node) => {
                write!(
                    f,
                    "missing external don't-care attachment for node {}",
                    node.index()
                )
            }
            Self::CycleDetected => write!(f, "network contains a cycle"),
        }
    }
}

impl Error for NetworkUtilError {}

pub type NetworkUtilResult<T> = Result<T, NetworkUtilError>;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ExternalDcAttachment {
    care_po_to_dc_po: BTreeMap<NodeId, NodeId>,
    dc_pi_to_care_pi: BTreeMap<NodeId, NodeId>,
}

impl ExternalDcAttachment {
    pub fn care_output(&self, care_output: NodeId) -> Option<NodeId> {
        self.care_po_to_dc_po.get(&care_output).copied()
    }

    pub fn dc_input(&self, dc_input: NodeId) -> Option<NodeId> {
        self.dc_pi_to_care_pi.get(&dc_input).copied()
    }

    pub fn care_outputs(&self) -> &BTreeMap<NodeId, NodeId> {
        &self.care_po_to_dc_po
    }

    pub fn dc_inputs(&self) -> &BTreeMap<NodeId, NodeId> {
        &self.dc_pi_to_care_pi
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Network {
    name: Option<String>,
    nodes: Vec<Option<NetworkNode>>,
    order: Vec<NodeId>,
    inputs: Vec<NodeId>,
    outputs: Vec<NodeId>,
    name_table: BTreeMap<String, NodeId>,
    short_name_table: BTreeMap<String, NodeId>,
    original: Option<Box<Network>>,
    dc_network: Option<Box<Network>>,
    area_given: bool,
    area: f64,
    bdd_list: Vec<String>,
    default_delay: Option<String>,
    next_generated_name: usize,
    next_generated_short_name: usize,
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

impl Network {
    pub fn new() -> Self {
        Self {
            name: None,
            nodes: Vec::new(),
            order: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            name_table: BTreeMap::new(),
            short_name_table: BTreeMap::new(),
            original: None,
            dc_network: None,
            area_given: false,
            area: 0.0,
            bdd_list: Vec::new(),
            default_delay: None,
            next_generated_name: 0,
            next_generated_short_name: 0,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("unknown")
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }

    pub fn original(&self) -> Option<&Network> {
        self.original.as_deref()
    }

    pub fn set_original(&mut self, original: Option<Network>) {
        self.original = original.map(Box::new);
    }

    pub fn dc_network(&self) -> Option<&Network> {
        self.dc_network.as_deref()
    }

    pub fn set_dc_network(&mut self, dc_network: Option<Network>) {
        self.dc_network = dc_network.map(Box::new);
    }

    pub fn area_given(&self) -> bool {
        self.area_given
    }

    pub fn area(&self) -> f64 {
        self.area
    }

    pub fn set_area(&mut self, area: f64, area_given: bool) {
        self.area = area;
        self.area_given = area_given;
    }

    pub fn default_delay(&self) -> Option<&str> {
        self.default_delay.as_deref()
    }

    pub fn set_default_delay(&mut self, default_delay: Option<String>) {
        self.default_delay = default_delay;
    }

    pub fn bdd_list(&self) -> &[String] {
        &self.bdd_list
    }

    pub fn add_bdd(&mut self, bdd: impl Into<String>) -> usize {
        self.bdd_list.insert(0, bdd.into());
        0
    }

    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &NetworkNode)> {
        self.order.iter().filter_map(|id| {
            self.nodes
                .get(id.index())
                .and_then(Option::as_ref)
                .map(|node| (*id, node))
        })
    }

    pub fn primary_inputs(&self) -> &[NodeId] {
        &self.inputs
    }

    pub fn primary_outputs(&self) -> &[NodeId] {
        &self.outputs
    }

    pub fn num_pi(&self) -> usize {
        self.inputs.len()
    }

    pub fn num_po(&self) -> usize {
        self.outputs.len()
    }

    pub fn num_internal(&self) -> usize {
        self.order
            .len()
            .saturating_sub(self.inputs.len() + self.outputs.len())
    }

    pub fn get_pi(&self, index: usize) -> Option<NodeId> {
        self.inputs.get(index).copied()
    }

    pub fn get_po(&self, index: usize) -> Option<NodeId> {
        self.outputs.get(index).copied()
    }

    pub fn node(&self, node: NodeId) -> NetworkUtilResult<&NetworkNode> {
        self.nodes
            .get(node.index())
            .and_then(Option::as_ref)
            .ok_or(NetworkUtilError::MissingNode(node))
    }

    pub fn node_mut(&mut self, node: NodeId) -> NetworkUtilResult<&mut NetworkNode> {
        self.nodes
            .get_mut(node.index())
            .and_then(Option::as_mut)
            .ok_or(NetworkUtilError::MissingNode(node))
    }

    pub fn find_node(&self, name: &str) -> Option<NodeId> {
        self.name_table.get(name).copied()
    }

    pub fn add_primary_input(&mut self, mut node: NetworkNode) -> NetworkUtilResult<NodeId> {
        node.kind = NodeKind::PrimaryInput;
        node.fanins.clear();
        self.add_node(node)
    }

    pub fn add_primary_output(&mut self, fanin: NodeId) -> NetworkUtilResult<NodeId> {
        self.node(fanin)?;
        let output_name = self.next_name();
        let short_name = self.next_short_name();
        let mut output = NetworkNode::new(output_name, NodeKind::PrimaryOutput);
        output.short_name = short_name;
        output.fanins.push(fanin);
        let output = self.add_node(output)?;
        self.swap_names(fanin, output)?;
        Ok(output)
    }

    pub fn add_internal(
        &mut self,
        name: impl Into<String>,
        fanins: impl Into<Vec<NodeId>>,
        cover: SopCover,
    ) -> NetworkUtilResult<NodeId> {
        let node = NetworkNode::new(name, NodeKind::Internal).with_cover(fanins, cover);
        self.add_node(node)
    }

    pub fn add_expression_node(
        &mut self,
        name: impl Into<String>,
        expression: BoolExpr,
    ) -> NetworkUtilResult<NodeId> {
        let node = NetworkNode::new(name, NodeKind::Internal).with_expression(expression);
        self.add_node(node)
    }

    pub fn add_node(&mut self, mut node: NetworkNode) -> NetworkUtilResult<NodeId> {
        if node.kind == NodeKind::Unassigned {
            node.kind = NodeKind::Internal;
        }

        if node.name.is_empty() {
            node.name = self.next_name();
        }

        if node.short_name.is_empty() {
            node.short_name = self.next_short_name();
        }

        if is_madeup_name(&node.name) {
            while self.name_exists_in_self_or_dc(&node.name) {
                node.name = self.next_name();
            }
        } else if self.name_table.contains_key(&node.name) {
            return Err(NetworkUtilError::DuplicateName(node.name));
        }

        while self.short_name_table.contains_key(&node.short_name) {
            node.short_name = self.next_short_name();
        }

        for fanin in &node.fanins {
            self.node(*fanin)?;
        }

        if node.kind == NodeKind::PrimaryOutput && node.fanins.len() != 1 {
            return Err(NetworkUtilError::InvalidPrimaryOutput(NodeId(
                self.nodes.len(),
            )));
        }

        let id = NodeId(self.nodes.len());
        self.name_table.insert(node.name.clone(), id);
        self.short_name_table.insert(node.short_name.clone(), id);

        if node.kind == NodeKind::PrimaryInput {
            self.inputs.push(id);
        } else if node.kind == NodeKind::PrimaryOutput {
            self.outputs.push(id);
        }

        let fanins = node.fanins.clone();
        self.nodes.push(Some(node));
        self.order.push(id);

        for fanin in fanins {
            self.node_mut(fanin)?.fanouts.insert(id);
        }

        Ok(id)
    }

    pub fn delete_node(&mut self, node: NodeId) -> NetworkUtilResult<NetworkNode> {
        self.node(node)?;
        self.change_node_type(node, NodeKind::Internal)?;

        let removed = self.nodes[node.index()]
            .take()
            .ok_or(NetworkUtilError::MissingNode(node))?;
        self.order.retain(|id| *id != node);
        self.name_table.remove(&removed.name);
        self.short_name_table.remove(&removed.short_name);

        for fanin in &removed.fanins {
            if let Some(fanin_node) = self.nodes.get_mut(fanin.index()).and_then(Option::as_mut) {
                fanin_node.fanouts.remove(&node);
            }
        }

        for fanout in removed.fanouts.iter().copied().collect::<Vec<_>>() {
            if let Some(fanout_node) = self.nodes.get_mut(fanout.index()).and_then(Option::as_mut) {
                fanout_node.fanins.retain(|fanin| *fanin != node);
            }
        }

        Ok(removed)
    }

    pub fn change_node_type(&mut self, node: NodeId, new_kind: NodeKind) -> NetworkUtilResult<()> {
        let old_kind = self.node(node)?.kind;

        if old_kind == NodeKind::PrimaryInput {
            remove_from_list(&mut self.inputs, node)?;
        } else if old_kind == NodeKind::PrimaryOutput {
            remove_from_list(&mut self.outputs, node)?;
        }

        if new_kind == NodeKind::PrimaryInput {
            self.inputs.push(node);
            self.node_mut(node)?.fanins.clear();
        } else if new_kind == NodeKind::PrimaryOutput {
            if self.node(node)?.fanins.len() != 1 {
                return Err(NetworkUtilError::InvalidPrimaryOutput(node));
            }
            self.outputs.push(node);
        }

        self.node_mut(node)?.kind = new_kind;
        Ok(())
    }

    pub fn change_node_name(
        &mut self,
        node: NodeId,
        new_name: impl Into<String>,
    ) -> NetworkUtilResult<()> {
        self.node(node)?;
        let new_name = new_name.into();
        if let Some(existing) = self.name_table.get(&new_name) {
            if *existing != node {
                return Err(NetworkUtilError::DuplicateName(new_name));
            }
        }

        let old_name = self.node(node)?.name.clone();
        self.name_table.remove(&old_name);
        self.node_mut(node)?.name = new_name.clone();
        self.name_table.insert(new_name, node);
        Ok(())
    }

    pub fn change_node_short_name(
        &mut self,
        node: NodeId,
        new_name: impl Into<String>,
    ) -> NetworkUtilResult<()> {
        self.node(node)?;
        let new_name = new_name.into();
        if let Some(existing) = self.short_name_table.get(&new_name) {
            if *existing != node {
                return Err(NetworkUtilError::DuplicateShortName(new_name));
            }
        }

        let old_name = self.node(node)?.short_name.clone();
        self.short_name_table.remove(&old_name);
        self.node_mut(node)?.short_name = new_name.clone();
        self.short_name_table.insert(new_name, node);
        Ok(())
    }

    pub fn swap_names(&mut self, first: NodeId, second: NodeId) -> NetworkUtilResult<()> {
        self.node(first)?;
        self.node(second)?;

        let first_name = self.node(first)?.name.clone();
        let first_short = self.node(first)?.short_name.clone();
        let second_name = self.node(second)?.name.clone();
        let second_short = self.node(second)?.short_name.clone();

        self.name_table.remove(&first_name);
        self.name_table.remove(&second_name);
        self.short_name_table.remove(&first_short);
        self.short_name_table.remove(&second_short);

        self.node_mut(first)?.name = second_name.clone();
        self.node_mut(second)?.name = first_name.clone();
        self.node_mut(first)?.short_name = second_short.clone();
        self.node_mut(second)?.short_name = first_short.clone();

        self.name_table.insert(first_name, second);
        self.name_table.insert(second_name, first);
        self.short_name_table.insert(first_short, second);
        self.short_name_table.insert(second_short, first);
        Ok(())
    }

    pub fn duplicate(&self) -> NetworkUtilResult<Self> {
        let mut new = Self::new();
        new.name = self.name.clone();
        new.area_given = self.area_given;
        new.area = self.area;
        new.bdd_list = self.bdd_list.clone();
        new.default_delay = self.default_delay.clone();
        new.next_generated_name = self.next_generated_name;
        new.next_generated_short_name = self.next_generated_short_name;

        let mut copies = BTreeMap::new();
        for (old_id, node) in self.nodes() {
            let mut new_node = node.clone();
            new_node.fanins.clear();
            new_node.fanouts.clear();
            let new_id = NodeId(new.nodes.len());
            new.name_table.insert(new_node.name.clone(), new_id);
            new.short_name_table
                .insert(new_node.short_name.clone(), new_id);
            if new_node.kind == NodeKind::PrimaryInput {
                new.inputs.push(new_id);
            } else if new_node.kind == NodeKind::PrimaryOutput {
                new.outputs.push(new_id);
            }
            new.nodes.push(Some(new_node));
            new.order.push(new_id);
            copies.insert(old_id, new_id);
        }

        for (old_id, node) in self.nodes() {
            let new_id = copies[&old_id];
            let fanins = node
                .fanins
                .iter()
                .map(|fanin| copies[fanin])
                .collect::<Vec<_>>();
            new.node_mut(new_id)?.fanins = fanins.clone();
            for fanin in fanins {
                new.node_mut(fanin)?.fanouts.insert(new_id);
            }
        }

        new.original = match &self.original {
            Some(original) => Some(Box::new(original.duplicate()?)),
            None => None,
        };
        new.dc_network = match &self.dc_network {
            Some(dc_network) => Some(Box::new(dc_network.duplicate()?)),
            None => None,
        };

        Ok(new)
    }

    pub fn attach_dc_network(&self) -> NetworkUtilResult<ExternalDcAttachment> {
        let dc_network = self
            .dc_network
            .as_deref()
            .ok_or(NetworkUtilError::MissingDcNetwork)?;
        let mut attachment = ExternalDcAttachment::default();

        for care_output in &self.outputs {
            let care_name = &self.node(*care_output)?.name;
            for dc_output in &dc_network.outputs {
                if dc_network.node(*dc_output)?.name == *care_name {
                    attachment.care_po_to_dc_po.insert(*care_output, *dc_output);
                }
            }
        }

        for care_input in &self.inputs {
            let care_name = &self.node(*care_input)?.name;
            for dc_input in &dc_network.inputs {
                if dc_network.node(*dc_input)?.name == *care_name {
                    attachment.dc_pi_to_care_pi.insert(*dc_input, *care_input);
                }
            }
        }

        Ok(attachment)
    }

    pub fn find_external_dc(
        &self,
        primary_output: NodeId,
        attachment: &ExternalDcAttachment,
    ) -> NetworkUtilResult<BoolExpr> {
        let dc_network = self
            .dc_network
            .as_deref()
            .ok_or(NetworkUtilError::MissingDcNetwork)?;
        let Some(dc_output) = attachment.care_output(primary_output) else {
            return Ok(BoolExpr::constant(false));
        };
        let dc_output_node = dc_network.node(dc_output)?;
        if dc_output_node.fanins.len() != 1 {
            return Err(NetworkUtilError::InvalidPrimaryOutput(dc_output));
        }

        let driver = dc_output_node.fanins[0];
        dc_network.expression_for_dc_node(driver, attachment)
    }

    pub fn or_with_dc_network(&self) -> NetworkUtilResult<Self> {
        let mut net = self.duplicate()?;
        let attachment = net.attach_dc_network()?;
        let outputs = net.outputs.clone();

        for output in outputs {
            let dc_expr = net.find_external_dc(output, &attachment)?;
            if dc_expr == BoolExpr::Constant(false) {
                continue;
            }

            let original_driver = {
                let output_node = net.node(output)?;
                if output_node.fanins.len() != 1 {
                    return Err(NetworkUtilError::InvalidPrimaryOutput(output));
                }
                output_node.fanins[0]
            };
            let original_expr = BoolExpr::literal(original_driver, true);
            let merged_expr = dc_expr.or(original_expr);
            let merged_name = net.next_name();
            let merged = net.add_expression_node(merged_name, merged_expr)?;
            net.patch_fanin(output, original_driver, merged)?;
        }

        Ok(net)
    }

    pub fn patch_fanin(
        &mut self,
        node: NodeId,
        old_fanin: NodeId,
        new_fanin: NodeId,
    ) -> NetworkUtilResult<()> {
        self.node(old_fanin)?;
        self.node(new_fanin)?;
        let fanin_replaced = {
            let target = self.node_mut(node)?;
            let mut replaced = false;
            for fanin in &mut target.fanins {
                if *fanin == old_fanin {
                    *fanin = new_fanin;
                    replaced = true;
                }
            }
            replaced
        };

        if fanin_replaced {
            self.node_mut(old_fanin)?.fanouts.remove(&node);
            self.node_mut(new_fanin)?.fanouts.insert(node);
        }

        Ok(())
    }

    fn expression_for_dc_node(
        &self,
        node: NodeId,
        attachment: &ExternalDcAttachment,
    ) -> NetworkUtilResult<BoolExpr> {
        let mut memo = BTreeMap::new();
        self.expression_for_dc_node_inner(node, attachment, &mut memo, &mut BTreeSet::new())
    }

    fn expression_for_dc_node_inner(
        &self,
        node: NodeId,
        attachment: &ExternalDcAttachment,
        memo: &mut BTreeMap<NodeId, BoolExpr>,
        active: &mut BTreeSet<NodeId>,
    ) -> NetworkUtilResult<BoolExpr> {
        if let Some(expression) = memo.get(&node) {
            return Ok(expression.clone());
        }

        if !active.insert(node) {
            return Err(NetworkUtilError::CycleDetected);
        }

        let network_node = self.node(node)?;
        let expression = match network_node.kind {
            NodeKind::PrimaryInput => {
                let care_input = attachment
                    .dc_input(node)
                    .ok_or(NetworkUtilError::MissingDcAttachment(node))?;
                BoolExpr::literal(care_input, true)
            }
            NodeKind::PrimaryOutput => {
                if network_node.fanins.len() != 1 {
                    return Err(NetworkUtilError::InvalidPrimaryOutput(node));
                }
                self.expression_for_dc_node_inner(network_node.fanins[0], attachment, memo, active)?
            }
            NodeKind::Internal | NodeKind::Unassigned => {
                self.expression_from_cover(node, attachment, memo, active)?
            }
        };

        active.remove(&node);
        memo.insert(node, expression.clone());
        Ok(expression)
    }

    fn expression_from_cover(
        &self,
        node: NodeId,
        attachment: &ExternalDcAttachment,
        memo: &mut BTreeMap<NodeId, BoolExpr>,
        active: &mut BTreeSet<NodeId>,
    ) -> NetworkUtilResult<BoolExpr> {
        let network_node = self.node(node)?;
        let Some(cover) = &network_node.cover else {
            return Ok(network_node
                .expression
                .clone()
                .unwrap_or(BoolExpr::constant(false)));
        };

        let mut sum = BoolExpr::constant(false);
        for (cube_index, cube) in cover.cubes().iter().enumerate() {
            if cube.values().len() != network_node.fanins.len() {
                return Err(NetworkUtilError::InvalidCover {
                    node,
                    cube: cube_index,
                });
            }

            let mut product = BoolExpr::constant(true);
            for (pin, value) in cube.values().iter().enumerate() {
                if *value == CoverValue::DontCare {
                    continue;
                }

                let fanin = network_node.fanins[pin];
                let fanin_expr =
                    self.expression_for_dc_node_inner(fanin, attachment, memo, active)?;
                let literal = match value {
                    CoverValue::One => fanin_expr,
                    CoverValue::Zero => negate_expression(fanin_expr),
                    CoverValue::DontCare => unreachable!(),
                };
                product = product.and(literal);
            }
            sum = sum.or(product);
        }

        Ok(sum)
    }

    fn name_exists_in_self_or_dc(&self, name: &str) -> bool {
        if self.name_table.contains_key(name) {
            return true;
        }

        if let Some(dc_network) = &self.dc_network {
            return dc_network.name_table.contains_key(name);
        }

        false
    }

    fn next_name(&mut self) -> String {
        let value = format!("[{}]", self.next_generated_name);
        self.next_generated_name += 1;
        value
    }

    fn next_short_name(&mut self) -> String {
        let index = self.next_generated_short_name;
        let letter = (b'a' + (index % 26) as u8) as char;
        let suffix = index / 26;
        self.next_generated_short_name += 1;
        if suffix == 0 {
            letter.to_string()
        } else {
            format!("{letter}{}", suffix - 1)
        }
    }
}

fn is_madeup_name(name: &str) -> bool {
    name.strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .is_some_and(|value| !value.is_empty() && value.chars().all(|item| item.is_ascii_digit()))
}

fn negate_expression(expression: BoolExpr) -> BoolExpr {
    match expression {
        BoolExpr::Constant(value) => BoolExpr::Constant(!value),
        BoolExpr::Literal { node, phase } => BoolExpr::Literal {
            node,
            phase: !phase,
        },
        BoolExpr::Not(inner) => *inner,
        value => BoolExpr::Not(Box::new(value)),
    }
}

fn remove_from_list(list: &mut Vec<NodeId>, node: NodeId) -> NetworkUtilResult<()> {
    let before = list.len();
    list.retain(|item| *item != node);
    if list.len() == before {
        return Err(NetworkUtilError::NodeBelongsToDifferentNetwork(node));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(values: &[CoverValue]) -> Cube {
        Cube::new(values.to_vec())
    }

    #[test]
    fn network_tracks_primary_io_and_internal_counts() {
        let mut network = Network::new();
        network.set_name("demo");
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::Unassigned))
            .unwrap();
        let b = network
            .add_primary_input(NetworkNode::new("b", NodeKind::Unassigned))
            .unwrap();
        let n = network
            .add_internal(
                "n",
                vec![a, b],
                SopCover::new([cube(&[CoverValue::One, CoverValue::One])]),
            )
            .unwrap();
        let y = network.add_primary_output(n).unwrap();

        assert_eq!(network.name(), "demo");
        assert_eq!(network.num_pi(), 2);
        assert_eq!(network.num_po(), 1);
        assert_eq!(network.num_internal(), 1);
        assert_eq!(network.get_pi(0), Some(a));
        assert_eq!(network.get_po(0), Some(y));
        assert_eq!(network.node(n).unwrap().fanouts, BTreeSet::from([y]));
    }

    #[test]
    fn duplicate_preserves_order_and_rebuilds_fanouts() {
        let mut network = Network::new();
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let n = network
            .add_internal("n", vec![a], SopCover::new([cube(&[CoverValue::Zero])]))
            .unwrap();
        network.add_primary_output(n).unwrap();

        let copy = network.duplicate().unwrap();
        let copied_a = copy.find_node("a").unwrap();
        let copied_output = copy.get_po(0).unwrap();
        let copied_n = copy.node(copied_output).unwrap().fanins[0];

        assert_eq!(copy.node(copied_n).unwrap().fanins, vec![copied_a]);
        assert!(copy.node(copied_a).unwrap().fanouts.contains(&copied_n));
    }

    #[test]
    fn renaming_and_swapping_update_name_tables() {
        let mut network = Network::new();
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();

        network.change_node_name(a, "aa").unwrap();
        assert_eq!(network.find_node("aa"), Some(a));
        assert_eq!(network.find_node("a"), None);

        network.swap_names(a, b).unwrap();
        assert_eq!(network.find_node("b"), Some(a));
        assert_eq!(network.find_node("aa"), Some(b));
    }

    #[test]
    fn attach_dc_network_matches_primary_io_by_name() {
        let mut care = Network::new();
        let a = care
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let y_driver = care
            .add_internal("n", vec![a], SopCover::new([cube(&[CoverValue::One])]))
            .unwrap();
        let y = care.add_primary_output(y_driver).unwrap();

        let mut dc = Network::new();
        let dc_a = dc
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let dc_driver = dc
            .add_internal(
                "dc_n",
                vec![dc_a],
                SopCover::new([cube(&[CoverValue::Zero])]),
            )
            .unwrap();
        let dc_y = dc.add_primary_output(dc_driver).unwrap();
        dc.change_node_name(dc_y, care.node(y).unwrap().name.clone())
            .unwrap();

        care.set_dc_network(Some(dc));
        let attachment = care.attach_dc_network().unwrap();

        assert_eq!(attachment.care_output(y), Some(dc_y));
        assert_eq!(attachment.dc_input(dc_a), Some(a));
    }

    #[test]
    fn find_external_dc_translates_dc_cover_to_care_inputs() {
        let mut care = Network::new();
        let a = care
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = care
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let y_driver = care
            .add_internal(
                "n",
                vec![a, b],
                SopCover::new([cube(&[CoverValue::One, CoverValue::One])]),
            )
            .unwrap();
        let y = care.add_primary_output(y_driver).unwrap();

        let mut dc = Network::new();
        let dc_a = dc
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let dc_b = dc
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let dc_driver = dc
            .add_internal(
                "dc_n",
                vec![dc_a, dc_b],
                SopCover::new([cube(&[CoverValue::One, CoverValue::Zero])]),
            )
            .unwrap();
        let dc_y = dc.add_primary_output(dc_driver).unwrap();
        dc.change_node_name(dc_y, care.node(y).unwrap().name.clone())
            .unwrap();

        care.set_dc_network(Some(dc));
        let attachment = care.attach_dc_network().unwrap();
        let expression = care.find_external_dc(y, &attachment).unwrap();

        assert_eq!(
            expression,
            BoolExpr::And(vec![
                BoolExpr::Literal {
                    node: a,
                    phase: true,
                },
                BoolExpr::Literal {
                    node: b,
                    phase: false,
                },
            ])
        );
    }

    #[test]
    fn or_with_dc_network_patches_primary_output_driver() {
        let mut care = Network::new();
        let a = care
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let y_driver = care
            .add_internal("n", vec![a], SopCover::new([cube(&[CoverValue::One])]))
            .unwrap();
        let y = care.add_primary_output(y_driver).unwrap();

        let mut dc = Network::new();
        let dc_a = dc
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let dc_driver = dc
            .add_internal(
                "dc_n",
                vec![dc_a],
                SopCover::new([cube(&[CoverValue::Zero])]),
            )
            .unwrap();
        let dc_y = dc.add_primary_output(dc_driver).unwrap();
        dc.change_node_name(dc_y, care.node(y).unwrap().name.clone())
            .unwrap();
        care.set_dc_network(Some(dc));

        let merged = care.or_with_dc_network().unwrap();
        let output = merged.get_po(0).unwrap();
        let output_driver = merged.node(output).unwrap().fanins[0];

        assert_ne!(output_driver, y_driver);
        assert!(merged.node(output_driver).unwrap().expression.is_some());
    }
}
