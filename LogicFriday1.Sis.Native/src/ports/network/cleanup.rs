//! Native Rust cleanup pass for SIS-style networks.
//!
//! Cleanup removes internal nodes that no longer drive anything. The sequential
//! variant also removes latches whose output is unobserved, mirrors the original
//! DC-network adjustments for those latch endpoints, and then discards the
//! external DC network when a latch was removed.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CleanupNodeId(usize);

impl CleanupNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CleanupLatchId(usize);

impl CleanupLatchId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanupNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupFunction {
    Unknown,
    Constant(bool),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupNode {
    pub name: String,
    pub kind: CleanupNodeKind,
    pub fanins: Vec<CleanupNodeId>,
    pub fanouts: BTreeSet<CleanupNodeId>,
    pub function: CleanupFunction,
}

impl CleanupNode {
    pub fn new(name: impl Into<String>, kind: CleanupNodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: BTreeSet::new(),
            function: CleanupFunction::Unknown,
        }
    }

    pub fn constant(name: impl Into<String>, value: bool) -> Self {
        Self {
            name: name.into(),
            kind: CleanupNodeKind::Internal,
            fanins: Vec::new(),
            fanouts: BTreeSet::new(),
            function: CleanupFunction::Constant(value),
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = CleanupNodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupLatch {
    pub input: CleanupNodeId,
    pub output: CleanupNodeId,
}

impl CleanupLatch {
    pub fn new(input: CleanupNodeId, output: CleanupNodeId) -> Self {
        Self { input, output }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupReport {
    pub changed: bool,
    pub latch_removed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupError {
    MissingNode(CleanupNodeId),
    MissingLatch(CleanupLatchId),
    DuplicateFanin {
        node: CleanupNodeId,
        fanin: CleanupNodeId,
    },
    InvalidPrimaryOutput(CleanupNodeId),
}

impl fmt::Display for CleanupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => {
                write!(formatter, "missing cleanup node {}", node.index())
            }
            Self::MissingLatch(latch) => {
                write!(formatter, "missing cleanup latch {}", latch.index())
            }
            Self::DuplicateFanin { node, fanin } => {
                write!(
                    formatter,
                    "cleanup node {} references fanin {} more than once",
                    node.index(),
                    fanin.index()
                )
            }
            Self::InvalidPrimaryOutput(node) => {
                write!(
                    formatter,
                    "primary output {} must have exactly one fanin",
                    node.index()
                )
            }
        }
    }
}

impl Error for CleanupError {}

pub type CleanupResult<T> = Result<T, CleanupError>;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct CleanupNetwork {
    nodes: Vec<Option<CleanupNode>>,
    order: Vec<CleanupNodeId>,
    latches: Vec<Option<CleanupLatch>>,
    dc_network: Option<Box<CleanupNetwork>>,
    next_constant_name: usize,
}

impl CleanupNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node(&self, node: CleanupNodeId) -> CleanupResult<&CleanupNode> {
        self.nodes
            .get(node.index())
            .and_then(Option::as_ref)
            .ok_or(CleanupError::MissingNode(node))
    }

    pub fn node_mut(&mut self, node: CleanupNodeId) -> CleanupResult<&mut CleanupNode> {
        self.nodes
            .get_mut(node.index())
            .and_then(Option::as_mut)
            .ok_or(CleanupError::MissingNode(node))
    }

    pub fn nodes(&self) -> impl Iterator<Item = (CleanupNodeId, &CleanupNode)> {
        self.order.iter().filter_map(|id| {
            self.nodes
                .get(id.index())
                .and_then(Option::as_ref)
                .map(|node| (*id, node))
        })
    }

    pub fn latch(&self, latch: CleanupLatchId) -> CleanupResult<&CleanupLatch> {
        self.latches
            .get(latch.index())
            .and_then(Option::as_ref)
            .ok_or(CleanupError::MissingLatch(latch))
    }

    pub fn latches(&self) -> impl Iterator<Item = (CleanupLatchId, &CleanupLatch)> {
        self.latches
            .iter()
            .enumerate()
            .filter_map(|(index, latch)| latch.as_ref().map(|latch| (CleanupLatchId(index), latch)))
    }

    pub fn dc_network(&self) -> Option<&CleanupNetwork> {
        self.dc_network.as_deref()
    }

    pub fn set_dc_network(&mut self, dc_network: Option<CleanupNetwork>) {
        self.dc_network = dc_network.map(Box::new);
    }

    pub fn find_node(&self, name: &str) -> Option<CleanupNodeId> {
        self.nodes()
            .find_map(|(id, node)| (node.name == name).then_some(id))
    }

    pub fn add_node(&mut self, mut node: CleanupNode) -> CleanupResult<CleanupNodeId> {
        let id = CleanupNodeId(self.nodes.len());
        let mut seen = BTreeSet::new();
        for fanin in &node.fanins {
            self.node(*fanin)?;
            if !seen.insert(*fanin) {
                return Err(CleanupError::DuplicateFanin {
                    node: id,
                    fanin: *fanin,
                });
            }
        }

        if node.kind == CleanupNodeKind::PrimaryOutput && node.fanins.len() != 1 {
            return Err(CleanupError::InvalidPrimaryOutput(id));
        }

        let fanins = node.fanins.clone();
        node.fanouts.clear();
        self.nodes.push(Some(node));
        self.order.push(id);

        for fanin in fanins {
            self.node_mut(fanin)?.fanouts.insert(id);
        }

        Ok(id)
    }

    pub fn add_latch(&mut self, latch: CleanupLatch) -> CleanupResult<CleanupLatchId> {
        self.node(latch.input)?;
        self.node(latch.output)?;
        let id = CleanupLatchId(self.latches.len());
        self.latches.push(Some(latch));
        Ok(id)
    }

    pub fn cleanup(&mut self) -> CleanupResult<CleanupReport> {
        let report = self.cleanup_util(true)?;
        if report.latch_removed {
            self.dc_network = None;
        }

        Ok(report)
    }

    pub fn combinational_cleanup(&mut self) -> CleanupResult<bool> {
        self.cleanup_util(false).map(|report| report.changed)
    }

    pub fn cleanup_util(&mut self, sweep_latches: bool) -> CleanupResult<CleanupReport> {
        let mut changed_any = false;
        let mut latch_removed = false;

        loop {
            let mut changed = false;
            while let Some(node) = self.next_unobserved_internal() {
                self.delete_node(node)?;
                changed = true;
                changed_any = true;
            }

            if sweep_latches {
                while let Some(latch) = self.next_unobserved_latch()? {
                    let latch_data = self.latch(latch)?.clone();
                    self.update_dc_for_removed_latch(latch_data.input, latch_data.output)?;
                    self.delete_latch_with_nodes(latch, latch_data.input, latch_data.output)?;
                    changed = true;
                    changed_any = true;
                    latch_removed = true;
                }
            }

            if !changed {
                break;
            }
        }

        if let Some(dc_network) = &mut self.dc_network {
            dc_network.combinational_cleanup()?;
        }

        Ok(CleanupReport {
            changed: changed_any,
            latch_removed,
        })
    }

    fn next_unobserved_internal(&self) -> Option<CleanupNodeId> {
        self.nodes().find_map(|(id, node)| {
            (node.kind == CleanupNodeKind::Internal && node.fanouts.is_empty()).then_some(id)
        })
    }

    fn next_unobserved_latch(&self) -> CleanupResult<Option<CleanupLatchId>> {
        for (id, latch) in self.latches() {
            if self.node(latch.output)?.fanouts.is_empty() {
                return Ok(Some(id));
            }
        }

        Ok(None)
    }

    fn update_dc_for_removed_latch(
        &mut self,
        input: CleanupNodeId,
        output: CleanupNodeId,
    ) -> CleanupResult<()> {
        let output_name = self.node(output)?.name.clone();
        let input_name = self.node(input)?.name.clone();
        let Some(dc_network) = &mut self.dc_network else {
            return Ok(());
        };

        if let Some(dc_input) = dc_network.find_node(&output_name) {
            let constant = dc_network.add_constant(false)?;
            dc_network.patch_all_fanouts(dc_input, constant)?;
            dc_network.delete_node(dc_input)?;
        }

        if let Some(dc_output) = dc_network.find_node(&input_name) {
            dc_network.delete_node(dc_output)?;
        }

        Ok(())
    }

    fn patch_all_fanouts(
        &mut self,
        old_fanin: CleanupNodeId,
        new_fanin: CleanupNodeId,
    ) -> CleanupResult<()> {
        let fanouts = self
            .node(old_fanin)?
            .fanouts
            .iter()
            .copied()
            .collect::<Vec<_>>();

        for fanout in fanouts {
            self.patch_fanin(fanout, old_fanin, new_fanin)?;
        }

        Ok(())
    }

    fn patch_fanin(
        &mut self,
        node: CleanupNodeId,
        old_fanin: CleanupNodeId,
        new_fanin: CleanupNodeId,
    ) -> CleanupResult<()> {
        self.node(old_fanin)?;
        self.node(new_fanin)?;

        let replaced = {
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

        if replaced {
            self.node_mut(old_fanin)?.fanouts.remove(&node);
            self.node_mut(new_fanin)?.fanouts.insert(node);
        }

        Ok(())
    }

    fn add_constant(&mut self, value: bool) -> CleanupResult<CleanupNodeId> {
        let name = format!("const_{}", self.next_constant_name);
        self.next_constant_name += 1;
        self.add_node(CleanupNode::constant(name, value))
    }

    fn delete_latch_with_nodes(
        &mut self,
        latch: CleanupLatchId,
        input: CleanupNodeId,
        output: CleanupNodeId,
    ) -> CleanupResult<()> {
        if self
            .latches
            .get(latch.index())
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(CleanupError::MissingLatch(latch));
        }

        self.latches[latch.index()] = None;
        self.delete_node(input)?;
        self.delete_node(output)?;
        Ok(())
    }

    fn delete_node(&mut self, node: CleanupNodeId) -> CleanupResult<CleanupNode> {
        self.node(node)?;
        let removed = self.nodes[node.index()]
            .take()
            .ok_or(CleanupError::MissingNode(node))?;
        self.order.retain(|candidate| *candidate != node);

        for fanin in &removed.fanins {
            if let Some(fanin_node) = self.nodes.get_mut(fanin.index()).and_then(Option::as_mut) {
                fanin_node.fanouts.remove(&node);
            }
        }

        for fanout in &removed.fanouts {
            if let Some(fanout_node) = self.nodes.get_mut(fanout.index()).and_then(Option::as_mut) {
                fanout_node.fanins.retain(|fanin| *fanin != node);
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> CleanupNode {
        CleanupNode::new(name, CleanupNodeKind::PrimaryInput)
    }

    fn internal(name: &str, fanins: &[CleanupNodeId]) -> CleanupNode {
        CleanupNode::new(name, CleanupNodeKind::Internal).with_fanins(fanins.iter().copied())
    }

    fn output(name: &str, fanin: CleanupNodeId) -> CleanupNode {
        CleanupNode::new(name, CleanupNodeKind::PrimaryOutput).with_fanins([fanin])
    }

    #[test]
    fn cleanup_iteratively_removes_unobserved_internal_cone() {
        let mut network = CleanupNetwork::new();
        let primary = network.add_node(input("a")).unwrap();
        let first = network.add_node(internal("n1", &[primary])).unwrap();
        let second = network.add_node(internal("n2", &[first])).unwrap();

        let report = network.cleanup().unwrap();

        assert_eq!(
            report,
            CleanupReport {
                changed: true,
                latch_removed: false,
            }
        );
        assert!(network.node(second).is_err());
        assert!(network.node(first).is_err());
        assert!(network.node(primary).is_ok());
    }

    #[test]
    fn observed_internal_node_is_kept() {
        let mut network = CleanupNetwork::new();
        let primary = network.add_node(input("a")).unwrap();
        let driver = network.add_node(internal("n", &[primary])).unwrap();
        let output = network.add_node(output("y", driver)).unwrap();

        let report = network.cleanup().unwrap();

        assert_eq!(
            report,
            CleanupReport {
                changed: false,
                latch_removed: false,
            }
        );
        assert_eq!(network.node(output).unwrap().fanins, vec![driver]);
        assert!(network.node(driver).unwrap().fanouts.contains(&output));
    }

    #[test]
    fn combinational_cleanup_does_not_remove_unobserved_latch() {
        let mut network = CleanupNetwork::new();
        let primary = network.add_node(input("a")).unwrap();
        let latch_input = network.add_node(output("li", primary)).unwrap();
        let latch_output = network.add_node(input("lo")).unwrap();
        network
            .add_latch(CleanupLatch::new(latch_input, latch_output))
            .unwrap();

        let changed = network.combinational_cleanup().unwrap();

        assert!(!changed);
        assert_eq!(network.latches().count(), 1);
        assert!(network.node(latch_input).is_ok());
        assert!(network.node(latch_output).is_ok());
    }

    #[test]
    fn cleanup_removes_unobserved_latch_and_discards_dc_network() {
        let mut network = CleanupNetwork::new();
        let primary = network.add_node(input("a")).unwrap();
        let latch_input = network.add_node(output("li", primary)).unwrap();
        let latch_output = network.add_node(input("lo")).unwrap();
        network
            .add_latch(CleanupLatch::new(latch_input, latch_output))
            .unwrap();
        network.set_dc_network(Some(CleanupNetwork::new()));

        let report = network.cleanup().unwrap();

        assert_eq!(
            report,
            CleanupReport {
                changed: true,
                latch_removed: true,
            }
        );
        assert!(network.dc_network().is_none());
        assert_eq!(network.latches().count(), 0);
        assert!(network.node(latch_input).is_err());
        assert!(network.node(latch_output).is_err());
    }

    #[test]
    fn cleanup_util_patches_dc_network_for_removed_latch() {
        let mut network = CleanupNetwork::new();
        let primary = network.add_node(input("a")).unwrap();
        let latch_input = network.add_node(output("li", primary)).unwrap();
        let latch_output = network.add_node(input("lo")).unwrap();
        network
            .add_latch(CleanupLatch::new(latch_input, latch_output))
            .unwrap();

        let mut dc_network = CleanupNetwork::new();
        let dc_latch_output = dc_network.add_node(input("lo")).unwrap();
        let dc_consumer = dc_network
            .add_node(internal("dc_consumer", &[dc_latch_output]))
            .unwrap();
        let dc_latch_input = dc_network.add_node(output("li", dc_consumer)).unwrap();
        dc_network.add_node(output("visible", dc_consumer)).unwrap();
        network.set_dc_network(Some(dc_network));

        let report = network.cleanup_util(true).unwrap();
        let dc_network = network.dc_network().unwrap();
        let dc_consumer = dc_network.find_node("dc_consumer").unwrap();
        let replacement = dc_network.node(dc_consumer).unwrap().fanins[0];

        assert!(report.latch_removed);
        assert!(dc_network.find_node("lo").is_none());
        assert!(dc_network.find_node("li").is_none());
        assert!(dc_network.node(dc_latch_input).is_err());
        assert_eq!(
            dc_network.node(replacement).unwrap().function,
            CleanupFunction::Constant(false)
        );
    }

    #[test]
    fn dc_network_is_swept_after_cleanup() {
        let mut network = CleanupNetwork::new();
        let primary = network.add_node(input("a")).unwrap();
        network.add_node(output("y", primary)).unwrap();

        let mut dc_network = CleanupNetwork::new();
        let dead = dc_network.add_node(internal("dead", &[])).unwrap();
        network.set_dc_network(Some(dc_network));

        let report = network.cleanup().unwrap();

        assert!(!report.changed);
        assert!(network.dc_network().unwrap().node(dead).is_err());
    }
}
