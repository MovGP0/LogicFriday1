//! Native Rust latch primitives for the SIS latch layer.
//!
//! The original latch module is a small ownership and lookup layer around
//! `latch_t`: allocation defaults, field accessors, latch-table registration
//! for controls and endpoints, lookup by node, and equality over timing-related
//! latch attributes. This port keeps that behavior as owned Rust data with
//! explicit errors for invalid network membership.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LatchId(pub usize);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GateRef(String);

impl GateRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchSynchType {
    ActiveHigh,
    ActiveLow,
    RisingEdge,
    FallingEdge,
    Combinational,
    Asynchronous,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Latch {
    input: Option<NodeId>,
    output: Option<NodeId>,
    initial_value: i32,
    current_value: i32,
    synch_type: LatchSynchType,
    gate: Option<GateRef>,
    control: Option<NodeId>,
    user_data: Option<String>,
}

impl Default for Latch {
    fn default() -> Self {
        Self::new()
    }
}

impl Latch {
    pub fn new() -> Self {
        Self {
            input: None,
            output: None,
            initial_value: 3,
            current_value: 3,
            synch_type: LatchSynchType::Unknown,
            gate: None,
            control: None,
            user_data: None,
        }
    }

    pub fn input(&self) -> Option<NodeId> {
        self.input
    }

    pub fn set_input(&mut self, input: Option<NodeId>) {
        self.input = input;
    }

    pub fn output(&self) -> Option<NodeId> {
        self.output
    }

    pub fn set_output(&mut self, output: Option<NodeId>) {
        self.output = output;
    }

    pub fn initial_value(&self) -> i32 {
        self.initial_value
    }

    pub fn set_initial_value(&mut self, value: i32) {
        self.initial_value = value;
    }

    pub fn current_value(&self) -> i32 {
        self.current_value
    }

    pub fn set_current_value(&mut self, value: i32) {
        self.current_value = value;
    }

    pub fn synch_type(&self) -> LatchSynchType {
        self.synch_type
    }

    pub fn set_synch_type(&mut self, synch_type: LatchSynchType) {
        self.synch_type = synch_type;
    }

    pub fn gate(&self) -> Option<&GateRef> {
        self.gate.as_ref()
    }

    pub fn set_gate(&mut self, gate: Option<GateRef>) {
        self.gate = gate;
    }

    pub fn control(&self) -> Option<NodeId> {
        self.control
    }

    pub fn set_control_unchecked(&mut self, control: Option<NodeId>) {
        self.control = control;
    }

    pub fn user_data(&self) -> Option<&str> {
        self.user_data.as_deref()
    }

    pub fn set_user_data(&mut self, user_data: Option<String>) {
        self.user_data = user_data;
    }

    pub fn timing_equal(&self, other: &Self) -> bool {
        self.initial_value == other.initial_value
            && self.synch_type == other.synch_type
            && self.control == other.control
            && self.gate == other.gate
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum LatchTableEntry {
    Latch(LatchId),
    Control,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LatchError {
    MissingNode(NodeId),
    MissingLatch(LatchId),
    NodeNotInNetwork(NodeId),
    NodeAlreadyRegistered(NodeId),
}

impl fmt::Display for LatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "missing node {}", node.0),
            Self::MissingLatch(latch) => write!(formatter, "missing latch {}", latch.0),
            Self::NodeNotInNetwork(node) => {
                write!(formatter, "node {} is not part of a network", node.0)
            }
            Self::NodeAlreadyRegistered(node) => {
                write!(
                    formatter,
                    "node {} is already registered in the latch table",
                    node.0
                )
            }
        }
    }
}

impl Error for LatchError {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LatchNetwork {
    nodes: BTreeSet<NodeId>,
    latches: Vec<Option<Latch>>,
    latch_table: BTreeMap<NodeId, LatchTableEntry>,
}

impl LatchNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self) -> NodeId {
        let node = NodeId(self.nodes.len());
        self.nodes.insert(node);
        node
    }

    pub fn insert_node(&mut self, node: NodeId) -> bool {
        self.nodes.insert(node)
    }

    pub fn contains_node(&self, node: NodeId) -> bool {
        self.nodes.contains(&node)
    }

    pub fn allocate_latch(&mut self) -> LatchId {
        let latch = LatchId(self.latches.len());
        self.latches.push(Some(Latch::new()));
        latch
    }

    pub fn add_latch(
        &mut self,
        input: Option<NodeId>,
        output: Option<NodeId>,
    ) -> Result<LatchId, LatchError> {
        if let Some(input) = input {
            self.require_node(input)?;
        }
        if let Some(output) = output {
            self.require_node(output)?;
        }

        let latch = self.allocate_latch();
        {
            let stored = self.latch_mut(latch)?;
            stored.set_input(input);
            stored.set_output(output);
        }
        self.register_latch_endpoints(latch)?;
        Ok(latch)
    }

    pub fn latch(&self, latch: LatchId) -> Result<&Latch, LatchError> {
        self.latches
            .get(latch.0)
            .and_then(Option::as_ref)
            .ok_or(LatchError::MissingLatch(latch))
    }

    pub fn latch_mut(&mut self, latch: LatchId) -> Result<&mut Latch, LatchError> {
        self.latches
            .get_mut(latch.0)
            .and_then(Option::as_mut)
            .ok_or(LatchError::MissingLatch(latch))
    }

    pub fn remove_latch(&mut self, latch: LatchId) -> Result<Latch, LatchError> {
        let removed = self
            .latches
            .get_mut(latch.0)
            .and_then(Option::take)
            .ok_or(LatchError::MissingLatch(latch))?;
        self.latch_table
            .retain(|_, entry| !matches!(entry, LatchTableEntry::Latch(id) if *id == latch));
        Ok(removed)
    }

    pub fn set_latch_input(
        &mut self,
        latch: LatchId,
        input: Option<NodeId>,
    ) -> Result<(), LatchError> {
        if let Some(input) = input {
            self.require_node(input)?;
        }

        let old_input = self.latch(latch)?.input();
        if let Some(old_input) = old_input {
            self.remove_latch_endpoint_if_owned(old_input, latch);
        }
        self.latch_mut(latch)?.set_input(input);
        if let Some(input) = input {
            self.register_latch_endpoint(input, latch)?;
        }
        Ok(())
    }

    pub fn set_latch_output(
        &mut self,
        latch: LatchId,
        output: Option<NodeId>,
    ) -> Result<(), LatchError> {
        if let Some(output) = output {
            self.require_node(output)?;
        }

        let old_output = self.latch(latch)?.output();
        if let Some(old_output) = old_output {
            self.remove_latch_endpoint_if_owned(old_output, latch);
        }
        self.latch_mut(latch)?.set_output(output);
        if let Some(output) = output {
            self.register_latch_endpoint(output, latch)?;
        }
        Ok(())
    }

    pub fn set_latch_control(
        &mut self,
        latch: LatchId,
        control: Option<NodeId>,
    ) -> Result<(), LatchError> {
        if let Some(control) = control {
            self.require_node(control)?;
            self.latch_table.insert(control, LatchTableEntry::Control);
        }
        self.latch_mut(latch)?.set_control_unchecked(control);
        Ok(())
    }

    pub fn latch_from_node(&self, node: NodeId) -> Result<Option<LatchId>, LatchError> {
        self.require_node(node)?;
        match self.latch_table.get(&node) {
            Some(LatchTableEntry::Latch(latch)) => Ok(Some(*latch)),
            Some(LatchTableEntry::Control) | None => Ok(None),
        }
    }

    pub fn is_control_registered(&self, node: NodeId) -> Result<bool, LatchError> {
        self.require_node(node)?;
        Ok(matches!(
            self.latch_table.get(&node),
            Some(LatchTableEntry::Control)
        ))
    }

    fn register_latch_endpoints(&mut self, latch: LatchId) -> Result<(), LatchError> {
        let stored = self.latch(latch)?;
        let endpoints = [stored.input(), stored.output()];
        for endpoint in endpoints.into_iter().flatten() {
            self.register_latch_endpoint(endpoint, latch)?;
        }
        Ok(())
    }

    fn register_latch_endpoint(&mut self, node: NodeId, latch: LatchId) -> Result<(), LatchError> {
        self.require_node(node)?;
        if let Some(entry) = self.latch_table.get(&node) {
            if *entry != LatchTableEntry::Latch(latch) {
                return Err(LatchError::NodeAlreadyRegistered(node));
            }
        }
        self.latch_table.insert(node, LatchTableEntry::Latch(latch));
        Ok(())
    }

    fn remove_latch_endpoint_if_owned(&mut self, node: NodeId, latch: LatchId) {
        if self.latch_table.get(&node) == Some(&LatchTableEntry::Latch(latch)) {
            self.latch_table.remove(&node);
        }
    }

    fn require_node(&self, node: NodeId) -> Result<(), LatchError> {
        if self.nodes.contains(&node) {
            Ok(())
        } else {
            Err(LatchError::NodeNotInNetwork(node))
        }
    }
}

pub fn latch_alloc() -> Latch {
    Latch::new()
}

pub fn latch_free(latch: Option<Latch>) {
    drop(latch);
}

pub fn latch_equal(first: &Latch, second: &Latch) -> bool {
    first.timing_equal(second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_matches_sis_defaults() {
        let latch = latch_alloc();

        assert_eq!(latch.input(), None);
        assert_eq!(latch.output(), None);
        assert_eq!(latch.initial_value(), 3);
        assert_eq!(latch.current_value(), 3);
        assert_eq!(latch.synch_type(), LatchSynchType::Unknown);
        assert_eq!(latch.gate(), None);
        assert_eq!(latch.control(), None);
        assert_eq!(latch.user_data(), None);
    }

    #[test]
    fn setters_preserve_all_latch_fields() {
        let mut latch = Latch::new();
        let input = NodeId(1);
        let output = NodeId(2);
        let control = NodeId(3);
        let gate = GateRef::new("dff");

        latch.set_input(Some(input));
        latch.set_output(Some(output));
        latch.set_initial_value(1);
        latch.set_current_value(0);
        latch.set_synch_type(LatchSynchType::RisingEdge);
        latch.set_gate(Some(gate.clone()));
        latch.set_control_unchecked(Some(control));
        latch.set_user_data(Some("owner".to_owned()));

        assert_eq!(latch.input(), Some(input));
        assert_eq!(latch.output(), Some(output));
        assert_eq!(latch.initial_value(), 1);
        assert_eq!(latch.current_value(), 0);
        assert_eq!(latch.synch_type(), LatchSynchType::RisingEdge);
        assert_eq!(latch.gate(), Some(&gate));
        assert_eq!(latch.control(), Some(control));
        assert_eq!(latch.user_data(), Some("owner"));
    }

    #[test]
    fn latch_table_finds_latch_from_input_and_output_nodes() {
        let mut network = LatchNetwork::new();
        let input = network.add_node();
        let output = network.add_node();

        let latch = network.add_latch(Some(input), Some(output)).unwrap();

        assert_eq!(network.latch_from_node(input).unwrap(), Some(latch));
        assert_eq!(network.latch_from_node(output).unwrap(), Some(latch));
    }

    #[test]
    fn control_registration_requires_network_membership_and_does_not_return_latch() {
        let mut network = LatchNetwork::new();
        let input = network.add_node();
        let output = network.add_node();
        let control = network.add_node();
        let latch = network.add_latch(Some(input), Some(output)).unwrap();

        network.set_latch_control(latch, Some(control)).unwrap();

        assert_eq!(network.latch(latch).unwrap().control(), Some(control));
        assert!(network.is_control_registered(control).unwrap());
        assert_eq!(network.latch_from_node(control).unwrap(), None);
    }

    #[test]
    fn setting_control_rejects_node_outside_network() {
        let mut network = LatchNetwork::new();
        let input = network.add_node();
        let output = network.add_node();
        let latch = network.add_latch(Some(input), Some(output)).unwrap();

        let error = network
            .set_latch_control(latch, Some(NodeId(99)))
            .unwrap_err();

        assert_eq!(error, LatchError::NodeNotInNetwork(NodeId(99)));
    }

    #[test]
    fn endpoint_updates_maintain_latch_table() {
        let mut network = LatchNetwork::new();
        let old_input = network.add_node();
        let new_input = network.add_node();
        let output = network.add_node();
        let latch = network.add_latch(Some(old_input), Some(output)).unwrap();

        network.set_latch_input(latch, Some(new_input)).unwrap();

        assert_eq!(network.latch_from_node(old_input).unwrap(), None);
        assert_eq!(network.latch_from_node(new_input).unwrap(), Some(latch));
        assert_eq!(network.latch_from_node(output).unwrap(), Some(latch));
    }

    #[test]
    fn duplicate_endpoint_registration_is_rejected() {
        let mut network = LatchNetwork::new();
        let first_input = network.add_node();
        let first_output = network.add_node();
        let second_output = network.add_node();
        let first = network
            .add_latch(Some(first_input), Some(first_output))
            .unwrap();
        let second = network.add_latch(None, Some(second_output)).unwrap();

        let error = network
            .set_latch_input(second, Some(first_input))
            .unwrap_err();

        assert_eq!(error, LatchError::NodeAlreadyRegistered(first_input));
        assert_eq!(network.latch_from_node(first_input).unwrap(), Some(first));
    }

    #[test]
    fn removing_latch_clears_only_its_endpoint_entries() {
        let mut network = LatchNetwork::new();
        let input = network.add_node();
        let output = network.add_node();
        let control = network.add_node();
        let latch = network.add_latch(Some(input), Some(output)).unwrap();
        network.set_latch_control(latch, Some(control)).unwrap();

        let removed = network.remove_latch(latch).unwrap();

        assert_eq!(removed.input(), Some(input));
        assert_eq!(network.latch_from_node(input).unwrap(), None);
        assert_eq!(network.latch_from_node(output).unwrap(), None);
        assert!(network.is_control_registered(control).unwrap());
    }

    #[test]
    fn equality_ignores_input_output_current_value_and_user_data() {
        let mut first = Latch::new();
        let mut second = Latch::new();
        first.set_input(Some(NodeId(1)));
        first.set_output(Some(NodeId(2)));
        first.set_current_value(0);
        first.set_user_data(Some("first".to_owned()));
        second.set_input(Some(NodeId(3)));
        second.set_output(Some(NodeId(4)));
        second.set_current_value(1);
        second.set_user_data(Some("second".to_owned()));

        assert!(latch_equal(&first, &second));

        second.set_initial_value(1);
        assert!(!latch_equal(&first, &second));
    }

    #[test]
    fn equality_compares_type_control_and_gate() {
        let mut first = Latch::new();
        let mut second = Latch::new();
        first.set_initial_value(1);
        second.set_initial_value(1);
        first.set_synch_type(LatchSynchType::ActiveHigh);
        second.set_synch_type(LatchSynchType::ActiveHigh);
        first.set_gate(Some(GateRef::new("lat")));
        second.set_gate(Some(GateRef::new("lat")));
        first.set_control_unchecked(Some(NodeId(7)));
        second.set_control_unchecked(Some(NodeId(7)));

        assert!(latch_equal(&first, &second));

        second.set_control_unchecked(Some(NodeId(8)));
        assert!(!latch_equal(&first, &second));
    }
}
