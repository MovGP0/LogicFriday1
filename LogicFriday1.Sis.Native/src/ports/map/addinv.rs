//! Native inverter-normalization support for `sis/map/addinv.c`.
//!
//! The legacy SIS implementation mutates a full `network_t` before and after
//! mapper replacement. This port keeps the same graph rewrite semantics in a
//! small owned Rust model: add shared inverter phases where mapping expects
//! them, remove redundant serial inverters, and merge parallel inverters.
//! Integration with the complete native SIS network mutation layer is still a
//! caller responsibility.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AddInvNodeId(pub usize);

impl AddInvNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddInvNodeKind {
    PrimaryInput,
    PrimaryOutput { latch: bool },
    Inverter,
    And,
    Or,
    Zero,
    One,
}

impl AddInvNodeKind {
    pub fn is_inverter(self) -> bool {
        matches!(self, Self::Inverter)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddInvNode {
    pub name: String,
    pub kind: AddInvNodeKind,
    pub fanins: Vec<AddInvNodeId>,
    deleted: bool,
}

impl AddInvNode {
    fn new(name: impl Into<String>, kind: AddInvNodeKind, fanins: Vec<AddInvNodeId>) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins,
            deleted: false,
        }
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AddInvNetwork {
    nodes: Vec<AddInvNode>,
}

impl AddInvNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        kind: AddInvNodeKind,
        fanins: Vec<AddInvNodeId>,
    ) -> AddInvNodeId {
        let id = AddInvNodeId(self.nodes.len());
        self.nodes.push(AddInvNode::new(name, kind, fanins));
        id
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> AddInvNodeId {
        self.add_node(name, AddInvNodeKind::PrimaryInput, Vec::new())
    }

    pub fn add_primary_output(
        &mut self,
        name: impl Into<String>,
        fanin: AddInvNodeId,
        latch: bool,
    ) -> AddInvNodeId {
        self.add_node(name, AddInvNodeKind::PrimaryOutput { latch }, vec![fanin])
    }

    pub fn add_inverter(&mut self, fanin: AddInvNodeId) -> AddInvNodeId {
        let name = format!("{}_inv{}", self.node_name(fanin), self.nodes.len());
        self.add_node(name, AddInvNodeKind::Inverter, vec![fanin])
    }

    pub fn node(&self, id: AddInvNodeId) -> Option<&AddInvNode> {
        self.nodes.get(id.index()).filter(|node| !node.deleted)
    }

    pub fn raw_node(&self, id: AddInvNodeId) -> Option<&AddInvNode> {
        self.nodes.get(id.index())
    }

    pub fn nodes(&self) -> &[AddInvNode] {
        &self.nodes
    }

    pub fn fanouts(&self, node: AddInvNodeId) -> Vec<AddInvNodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, candidate)| !candidate.deleted)
            .filter(|(_, candidate)| candidate.fanins.iter().any(|fanin| *fanin == node))
            .map(|(index, _)| AddInvNodeId(index))
            .collect()
    }

    pub fn fanout_count(&self, node: AddInvNodeId) -> usize {
        self.fanouts(node).len()
    }

    pub fn patch_fanin(
        &mut self,
        user: AddInvNodeId,
        old_fanin: AddInvNodeId,
        new_fanin: AddInvNodeId,
    ) -> Result<(), AddInvError> {
        self.ensure_node(old_fanin)?;
        self.ensure_node(new_fanin)?;

        let user_node = self
            .nodes
            .get_mut(user.index())
            .filter(|node| !node.deleted)
            .ok_or(AddInvError::MissingNode(user))?;
        let mut patched = false;

        for fanin in &mut user_node.fanins {
            if *fanin == old_fanin {
                *fanin = new_fanin;
                patched = true;
            }
        }

        patched.then_some(()).ok_or(AddInvError::MissingFanin {
            user,
            fanin: old_fanin,
        })
    }

    pub fn delete_node(&mut self, node: AddInvNodeId) -> Result<(), AddInvError> {
        if !self.fanouts(node).is_empty() {
            return Err(AddInvError::NodeStillHasFanout(node));
        }

        let item = self
            .nodes
            .get_mut(node.index())
            .filter(|node| !node.deleted)
            .ok_or(AddInvError::MissingNode(node))?;
        item.deleted = true;
        item.fanins.clear();
        Ok(())
    }

    pub fn cleanup_dangling_nodes(&mut self) {
        loop {
            let dangling = self.nodes.iter().enumerate().find_map(|(index, node)| {
                let id = AddInvNodeId(index);
                (!node.deleted
                    && matches!(
                        node.kind,
                        AddInvNodeKind::Inverter | AddInvNodeKind::And | AddInvNodeKind::Or
                    )
                    && self.fanouts(id).is_empty())
                .then_some(id)
            });

            let Some(node) = dangling else {
                break;
            };

            if let Some(item) = self.nodes.get_mut(node.index()) {
                item.deleted = true;
                item.fanins.clear();
            }
        }
    }

    fn ensure_node(&self, node: AddInvNodeId) -> Result<(), AddInvError> {
        self.node(node)
            .map(|_| ())
            .ok_or(AddInvError::MissingNode(node))
    }

    fn node_name(&self, node: AddInvNodeId) -> &str {
        self.nodes
            .get(node.index())
            .map(|node| node.name.as_str())
            .unwrap_or("node")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddInverterOptions {
    pub add_at_pipo: bool,
}

impl Default for AddInverterOptions {
    fn default() -> Self {
        Self { add_at_pipo: false }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AddInvReport {
    pub created_inverters: Vec<AddInvNodeId>,
    pub deleted_inverters: Vec<AddInvNodeId>,
    pub diagnostics: Vec<AddInvDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AddInvDiagnostic {
    FullSisNetworkMutationUnavailable { operation: &'static str },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AddInvError {
    MissingNode(AddInvNodeId),
    MissingFanin {
        user: AddInvNodeId,
        fanin: AddInvNodeId,
    },
    NodeStillHasFanout(AddInvNodeId),
    UnsupportedNodeKind {
        node: AddInvNodeId,
        kind: AddInvNodeKind,
    },
}

impl fmt::Display for AddInvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(f, "missing add-inverter node {}", node.index()),
            Self::MissingFanin { user, fanin } => {
                write!(
                    f,
                    "node {} does not use node {} as a fanin",
                    user.index(),
                    fanin.index()
                )
            }
            Self::NodeStillHasFanout(node) => {
                write!(
                    f,
                    "cannot delete node {} while it still has fanout",
                    node.index()
                )
            }
            Self::UnsupportedNodeKind { node, kind } => {
                write!(
                    f,
                    "add-inverter normalization cannot process node {} with kind {:?}",
                    node.index(),
                    kind
                )
            }
        }
    }
}

impl Error for AddInvError {}

pub fn full_sis_network_mutation_unavailable() -> AddInvReport {
    AddInvReport {
        diagnostics: vec![AddInvDiagnostic::FullSisNetworkMutationUnavailable {
            operation: "add/remove inverters on a complete SIS network",
        }],
        ..AddInvReport::default()
    }
}

pub fn map_add_inverter(
    network: &mut AddInvNetwork,
    options: AddInverterOptions,
) -> Result<AddInvReport, AddInvError> {
    let mut report = AddInvReport::default();
    let original_nodes = network
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| (!node.deleted).then_some(AddInvNodeId(index)))
        .collect::<Vec<_>>();
    let mut deleted = BTreeSet::new();

    for node in original_nodes {
        if deleted.contains(&node) || network.node(node).is_none() {
            continue;
        }

        if needs_inverter(network, node, options)? {
            add_inverters(network, node, &mut deleted, &mut report)?;
        }
    }

    for node in deleted {
        network.delete_node(node)?;
        report.deleted_inverters.push(node);
    }

    Ok(report)
}

pub fn map_remove_inverter(network: &mut AddInvNetwork) -> Result<AddInvReport, AddInvError> {
    let mut report = AddInvReport::default();
    remove_serial_inverters(network, &mut report)?;
    remove_parallel_inverters(network, &mut report)?;
    network.cleanup_dangling_nodes();
    Ok(report)
}

fn needs_inverter(
    network: &AddInvNetwork,
    node: AddInvNodeId,
    options: AddInverterOptions,
) -> Result<bool, AddInvError> {
    let kind = network
        .node(node)
        .ok_or(AddInvError::MissingNode(node))?
        .kind;

    match kind {
        AddInvNodeKind::PrimaryInput => Ok(network.fanout_count(node) > 1 || options.add_at_pipo),
        AddInvNodeKind::PrimaryOutput { .. }
        | AddInvNodeKind::Inverter
        | AddInvNodeKind::Zero
        | AddInvNodeKind::One => Ok(false),
        AddInvNodeKind::And | AddInvNodeKind::Or => {
            if network.fanout_count(node) == 1 && feeds_primary_output(network, node) {
                Ok(!feeds_latch(network, node) && options.add_at_pipo)
            } else {
                Ok(true)
            }
        }
    }
}

fn feeds_primary_output(network: &AddInvNetwork, node: AddInvNodeId) -> bool {
    network.fanouts(node).into_iter().any(|fanout| {
        network
            .node(fanout)
            .is_some_and(|node| matches!(node.kind, AddInvNodeKind::PrimaryOutput { .. }))
    })
}

fn feeds_latch(network: &AddInvNetwork, node: AddInvNodeId) -> bool {
    network.fanouts(node).into_iter().any(|fanout| {
        network
            .node(fanout)
            .is_some_and(|node| matches!(node.kind, AddInvNodeKind::PrimaryOutput { latch: true }))
    })
}

fn add_inverters(
    network: &mut AddInvNetwork,
    node: AddInvNodeId,
    deleted: &mut BTreeSet<AddInvNodeId>,
    report: &mut AddInvReport,
) -> Result<(), AddInvError> {
    let output_inv = network
        .fanouts(node)
        .into_iter()
        .filter(|fanout| {
            network
                .node(*fanout)
                .is_some_and(|node| node.kind == AddInvNodeKind::Inverter)
        })
        .collect::<Vec<_>>();

    let inv = if let Some(inv) = output_inv.first().copied() {
        inv
    } else {
        create_inverter(network, node, report)
    };

    for fanout in network.fanouts(node) {
        let is_inverter = network
            .node(fanout)
            .is_some_and(|node| node.kind == AddInvNodeKind::Inverter);

        if !is_inverter {
            let inv1 = create_inverter(network, inv, report);
            network.patch_fanin(fanout, node, inv1)?;
        }
    }

    for other_inv in output_inv {
        let fanouts = network.fanouts(other_inv);
        for fanout in fanouts {
            let fanout_kind = network
                .node(fanout)
                .ok_or(AddInvError::MissingNode(fanout))?
                .kind;
            if fanout_kind != AddInvNodeKind::Inverter
                && !matches!(fanout_kind, AddInvNodeKind::PrimaryOutput { .. })
            {
                let inv1 = create_inverter(network, inv, report);
                let inv2 = create_inverter(network, inv1, report);
                network.patch_fanin(fanout, other_inv, inv2)?;
            } else if other_inv != inv {
                network.patch_fanin(fanout, other_inv, inv)?;
            }
        }

        if other_inv != inv {
            deleted.insert(other_inv);
        }
    }

    Ok(())
}

fn create_inverter(
    network: &mut AddInvNetwork,
    fanin: AddInvNodeId,
    report: &mut AddInvReport,
) -> AddInvNodeId {
    let inv = network.add_inverter(fanin);
    report.created_inverters.push(inv);
    inv
}

fn remove_serial_inverters(
    network: &mut AddInvNetwork,
    report: &mut AddInvReport,
) -> Result<(), AddInvError> {
    let nodes = live_node_ids(network);

    for inv2 in nodes {
        if !network
            .node(inv2)
            .is_some_and(|node| node.kind == AddInvNodeKind::Inverter)
        {
            continue;
        }

        let Some(inv1) = network
            .node(inv2)
            .and_then(|node| node.fanins.first())
            .copied()
        else {
            continue;
        };

        if !network
            .node(inv1)
            .is_some_and(|node| node.kind == AddInvNodeKind::Inverter)
        {
            continue;
        }

        let Some(node) = network
            .node(inv1)
            .and_then(|node| node.fanins.first())
            .copied()
        else {
            continue;
        };

        for fanout in network.fanouts(inv2) {
            network.patch_fanin(fanout, inv2, node)?;
        }

        network.delete_node(inv2)?;
        report.deleted_inverters.push(inv2);
    }

    Ok(())
}

fn remove_parallel_inverters(
    network: &mut AddInvNetwork,
    report: &mut AddInvReport,
) -> Result<(), AddInvError> {
    let nodes = live_node_ids(network);

    for node in nodes {
        if network
            .node(node)
            .is_some_and(|node| node.kind == AddInvNodeKind::Inverter)
        {
            continue;
        }

        let mut first_inv = None;
        let fanouts = network.fanouts(node);

        for fanout in fanouts {
            if !network
                .node(fanout)
                .is_some_and(|node| node.kind == AddInvNodeKind::Inverter)
            {
                continue;
            }

            if let Some(inv1) = first_inv {
                for fanout1 in network.fanouts(fanout) {
                    network.patch_fanin(fanout1, fanout, inv1)?;
                }

                network.delete_node(fanout)?;
                report.deleted_inverters.push(fanout);
            } else {
                first_inv = Some(fanout);
            }
        }
    }

    Ok(())
}

fn live_node_ids(network: &AddInvNetwork) -> Vec<AddInvNodeId> {
    network
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| (!node.deleted).then_some(AddInvNodeId(index)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_shared_inverter_and_double_inverters_for_positive_fanouts() {
        let mut network = AddInvNetwork::new();
        let a = network.add_primary_input("a");
        let y1 = network.add_node("y1", AddInvNodeKind::And, vec![a]);
        let y2 = network.add_node("y2", AddInvNodeKind::Or, vec![a]);
        network.add_primary_output("f1", y1, false);
        network.add_primary_output("f2", y2, false);

        let report =
            map_add_inverter(&mut network, AddInverterOptions { add_at_pipo: false }).unwrap();

        assert_eq!(report.created_inverters.len(), 3);
        assert!(
            network
                .node(report.created_inverters[0])
                .unwrap()
                .fanins
                .contains(&a)
        );
        assert_ne!(network.node(y1).unwrap().fanins[0], a);
        assert_ne!(network.node(y2).unwrap().fanins[0], a);
    }

    #[test]
    fn merges_existing_output_inverters() {
        let mut network = AddInvNetwork::new();
        let a = network.add_primary_input("a");
        let inv1 = network.add_inverter(a);
        let inv2 = network.add_inverter(a);
        let y1 = network.add_node("y1", AddInvNodeKind::And, vec![inv1]);
        let y2 = network.add_node("y2", AddInvNodeKind::Or, vec![inv2]);
        network.add_primary_output("f1", y1, false);
        network.add_primary_output("f2", y2, false);

        let report =
            map_add_inverter(&mut network, AddInverterOptions { add_at_pipo: false }).unwrap();

        assert!(report.deleted_inverters.contains(&inv2));
        assert!(network.raw_node(inv2).unwrap().is_deleted());
        assert_ne!(network.node(y1).unwrap().fanins[0], inv1);
        assert_ne!(network.node(y2).unwrap().fanins[0], inv2);
    }

    #[test]
    fn skips_single_fanout_pipo_without_add_at_pipo() {
        let mut network = AddInvNetwork::new();
        let a = network.add_primary_input("a");
        let y = network.add_node("y", AddInvNodeKind::And, vec![a]);
        network.add_primary_output("f", y, false);

        let report =
            map_add_inverter(&mut network, AddInverterOptions { add_at_pipo: false }).unwrap();

        assert!(report.created_inverters.is_empty());
    }

    #[test]
    fn keeps_latch_output_single_fanout_without_added_inverter() {
        let mut network = AddInvNetwork::new();
        let y = network.add_node("y", AddInvNodeKind::And, Vec::new());
        network.add_primary_output("latched", y, true);

        let report =
            map_add_inverter(&mut network, AddInverterOptions { add_at_pipo: true }).unwrap();

        assert!(report.created_inverters.is_empty());
    }

    #[test]
    fn removes_serial_inverters() {
        let mut network = AddInvNetwork::new();
        let a = network.add_primary_input("a");
        let inv1 = network.add_inverter(a);
        let inv2 = network.add_inverter(inv1);
        let y = network.add_node("y", AddInvNodeKind::And, vec![inv2]);
        network.add_primary_output("f", y, false);

        let report = map_remove_inverter(&mut network).unwrap();

        assert!(report.deleted_inverters.contains(&inv2));
        assert_eq!(network.node(y).unwrap().fanins, vec![a]);
    }

    #[test]
    fn removes_parallel_inverters() {
        let mut network = AddInvNetwork::new();
        let a = network.add_primary_input("a");
        let inv1 = network.add_inverter(a);
        let inv2 = network.add_inverter(a);
        let y1 = network.add_node("y1", AddInvNodeKind::And, vec![inv1]);
        let y2 = network.add_node("y2", AddInvNodeKind::Or, vec![inv2]);
        network.add_primary_output("f1", y1, false);
        network.add_primary_output("f2", y2, false);

        let report = map_remove_inverter(&mut network).unwrap();

        assert!(report.deleted_inverters.contains(&inv2));
        assert_eq!(network.node(y1).unwrap().fanins, vec![inv1]);
        assert_eq!(network.node(y2).unwrap().fanins, vec![inv1]);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("addinv.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
