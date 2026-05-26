//! Native Rust structural checks for SIS networks.
//!
//! The legacy `netchk.c` routine validates the internal consistency of a SIS
//! network after editing passes. This port keeps that behavior as a Rust-native
//! checker over an owned graph model: primary input/output lists, fanin and
//! fanout pins, node membership, and name tables must all agree.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NetchkNodeId(pub usize);

impl NetchkNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetchkNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetchkNodeStatus {
    Valid,
    Warning(String),
    Invalid(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetchkFanoutPin {
    pub node: NetchkNodeId,
    pub pin: usize,
}

impl NetchkFanoutPin {
    pub fn new(node: NetchkNodeId, pin: usize) -> Self {
        Self { node, pin }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetchkNode {
    pub name: String,
    pub short_name: String,
    pub kind: NetchkNodeKind,
    pub fanins: Vec<NetchkNodeId>,
    pub fanouts: Vec<NetchkFanoutPin>,
    pub belongs_to_network: bool,
    pub status: NetchkNodeStatus,
}

impl NetchkNode {
    pub fn new(name: impl Into<String>, kind: NetchkNodeKind) -> Self {
        let name = name.into();
        Self {
            short_name: name.clone(),
            name,
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            belongs_to_network: true,
            status: NetchkNodeStatus::Valid,
        }
    }

    pub fn with_short_name(mut self, short_name: impl Into<String>) -> Self {
        self.short_name = short_name.into();
        self
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NetchkNodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_status(mut self, status: NetchkNodeStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_membership(mut self, belongs_to_network: bool) -> Self {
        self.belongs_to_network = belongs_to_network;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetchkNetwork {
    nodes: Vec<NetchkNode>,
    primary_inputs: Vec<NetchkNodeId>,
    primary_outputs: Vec<NetchkNodeId>,
    name_table: BTreeMap<String, NetchkNodeId>,
    short_name_table: BTreeMap<String, NetchkNodeId>,
}

impl Default for NetchkNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl NetchkNetwork {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            primary_inputs: Vec::new(),
            primary_outputs: Vec::new(),
            name_table: BTreeMap::new(),
            short_name_table: BTreeMap::new(),
        }
    }

    pub fn from_raw_parts(
        nodes: Vec<NetchkNode>,
        primary_inputs: Vec<NetchkNodeId>,
        primary_outputs: Vec<NetchkNodeId>,
        name_table: BTreeMap<String, NetchkNodeId>,
        short_name_table: BTreeMap<String, NetchkNodeId>,
    ) -> Self {
        Self {
            nodes,
            primary_inputs,
            primary_outputs,
            name_table,
            short_name_table,
        }
    }

    pub fn add_node(&mut self, mut node: NetchkNode) -> Result<NetchkNodeId, NetchkCheckError> {
        let id = NetchkNodeId(self.nodes.len());
        if self.name_table.contains_key(&node.name) {
            return Err(NetchkCheckError::DuplicateName { name: node.name });
        }

        if self.short_name_table.contains_key(&node.short_name) {
            return Err(NetchkCheckError::DuplicateShortName {
                name: node.short_name,
            });
        }

        for fanin in &node.fanins {
            self.node(*fanin)?;
        }

        node.fanouts.clear();
        let fanins = node.fanins.clone();
        if node.kind == NetchkNodeKind::PrimaryInput {
            self.primary_inputs.push(id);
        } else if node.kind == NetchkNodeKind::PrimaryOutput {
            self.primary_outputs.push(id);
        }

        self.name_table.insert(node.name.clone(), id);
        self.short_name_table.insert(node.short_name.clone(), id);
        self.nodes.push(node);

        for (pin, fanin) in fanins.into_iter().enumerate() {
            self.nodes[fanin.index()]
                .fanouts
                .push(NetchkFanoutPin::new(id, pin));
        }

        Ok(id)
    }

    pub fn node(&self, node: NetchkNodeId) -> Result<&NetchkNode, NetchkCheckError> {
        self.nodes
            .get(node.index())
            .ok_or(NetchkCheckError::MissingNode { node })
    }

    pub fn node_mut(&mut self, node: NetchkNodeId) -> Result<&mut NetchkNode, NetchkCheckError> {
        self.nodes
            .get_mut(node.index())
            .ok_or(NetchkCheckError::MissingNode { node })
    }

    pub fn nodes(&self) -> impl Iterator<Item = (NetchkNodeId, &NetchkNode)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (NetchkNodeId(index), node))
    }

    pub fn primary_inputs(&self) -> &[NetchkNodeId] {
        &self.primary_inputs
    }

    pub fn primary_outputs(&self) -> &[NetchkNodeId] {
        &self.primary_outputs
    }

    pub fn name_table(&self) -> &BTreeMap<String, NetchkNodeId> {
        &self.name_table
    }

    pub fn short_name_table(&self) -> &BTreeMap<String, NetchkNodeId> {
        &self.short_name_table
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetchkReport {
    pub warnings: Vec<NetchkWarning>,
}

impl NetchkReport {
    pub fn is_clean(&self) -> bool {
        self.warnings.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetchkWarning {
    pub node: NetchkNodeId,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetchkCheckError {
    MissingNode {
        node: NetchkNodeId,
    },
    DuplicateName {
        name: String,
    },
    DuplicateShortName {
        name: String,
    },
    NodeCheckFailed {
        node: NetchkNodeId,
        message: String,
    },
    NodeNotInNetwork {
        node: NetchkNodeId,
    },
    FaninNotInNetwork {
        node: NetchkNodeId,
        fanin: NetchkNodeId,
    },
    FanoutNotInNetwork {
        node: NetchkNodeId,
        fanout: NetchkNodeId,
    },
    PrimaryOutputListContainsNonOutput {
        node: NetchkNodeId,
    },
    PrimaryOutputFaninCount {
        node: NetchkNodeId,
        count: usize,
    },
    PrimaryOutputHasFanout {
        node: NetchkNodeId,
    },
    PrimaryInputListContainsNonInput {
        node: NetchkNodeId,
    },
    PrimaryInputHasFanin {
        node: NetchkNodeId,
    },
    PrimaryOutputMissingFromList {
        node: NetchkNodeId,
    },
    PrimaryInputMissingFromList {
        node: NetchkNodeId,
    },
    MissingFanoutPin {
        node: NetchkNodeId,
        fanin: NetchkNodeId,
        pin: usize,
    },
    DuplicateFanoutPin {
        node: NetchkNodeId,
        fanin: NetchkNodeId,
        pin: usize,
    },
    MissingFaninPin {
        node: NetchkNodeId,
        fanout: NetchkNodeId,
        pin: usize,
    },
    DuplicateFaninPin {
        node: NetchkNodeId,
        fanout: NetchkNodeId,
        pin: usize,
    },
    NameTableMissing {
        node: NetchkNodeId,
        name: String,
    },
    NameTableMismatch {
        node: NetchkNodeId,
        name: String,
        mapped: NetchkNodeId,
    },
    NameTableSuperfluous {
        name: String,
    },
    ShortNameTableMissing {
        node: NetchkNodeId,
        name: String,
    },
    ShortNameTableMismatch {
        node: NetchkNodeId,
        name: String,
        mapped: NetchkNodeId,
    },
    ShortNameTableSuperfluous {
        name: String,
    },
}

impl fmt::Display for NetchkCheckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode {
                node,
            } => write!(formatter, "missing network node {}", node.index()),
            Self::DuplicateName {
                name,
            } => write!(formatter, "duplicate network node name {name}"),
            Self::DuplicateShortName {
                name,
            } => write!(formatter, "duplicate network node short name {name}"),
            Self::NodeCheckFailed {
                node,
                message,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- {message}",
                node.index()
            ),
            Self::NodeNotInNetwork {
                node,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node on network list not in network",
                node.index()
            ),
            Self::FaninNotInNetwork {
                node,
                fanin,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- fanin {} is not in network",
                node.index(),
                fanin.index()
            ),
            Self::FanoutNotInNetwork {
                node,
                fanout,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- fanout {} is not in network",
                node.index(),
                fanout.index()
            ),
            Self::PrimaryOutputListContainsNonOutput {
                node,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node on PO list is not type PO",
                node.index()
            ),
            Self::PrimaryOutputFaninCount {
                node,
                count,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node on PO list has {count} fanins",
                node.index()
            ),
            Self::PrimaryOutputHasFanout {
                node,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node on PO list has fanout not 0",
                node.index()
            ),
            Self::PrimaryInputListContainsNonInput {
                node,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node on PI list is not type PI",
                node.index()
            ),
            Self::PrimaryInputHasFanin {
                node,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node on PI list has fanin not 0",
                node.index()
            ),
            Self::PrimaryOutputMissingFromList {
                node,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node has type PO, but is not on PO list",
                node.index()
            ),
            Self::PrimaryInputMissingFromList {
                node,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node has type PI, but is not on PI list",
                node.index()
            ),
            Self::MissingFanoutPin {
                node,
                fanin,
                pin,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- fanin {} has no fanout pin {pin}",
                node.index(),
                fanin.index()
            ),
            Self::DuplicateFanoutPin {
                node,
                fanin,
                pin,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- fanout duplicated on fanin {} pin {pin}",
                node.index(),
                fanin.index()
            ),
            Self::MissingFaninPin {
                node,
                fanout,
                pin,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- fanout {} has no fanin pin {pin}",
                node.index(),
                fanout.index()
            ),
            Self::DuplicateFaninPin {
                node,
                fanout,
                pin,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- fanin duplicated on fanout {} pin {pin}",
                node.index(),
                fanout.index()
            ),
            Self::NameTableMissing {
                node,
                name,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node name {name} is not in name_table",
                node.index()
            ),
            Self::NameTableMismatch {
                node,
                name,
                mapped,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- name_table maps {name} to node {}",
                node.index(),
                mapped.index()
            ),
            Self::NameTableSuperfluous {
                name,
            } => write!(
                formatter,
                "network_check: inconsistency detected -- name_table contains superfluous entry {name}"
            ),
            Self::ShortNameTableMissing {
                node,
                name,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- node short_name {name} is not in short_name_table",
                node.index()
            ),
            Self::ShortNameTableMismatch {
                node,
                name,
                mapped,
            } => write!(
                formatter,
                "network_check: inconsistency detected at node {} -- short_name_table maps {name} to node {}",
                node.index(),
                mapped.index()
            ),
            Self::ShortNameTableSuperfluous {
                name,
            } => write!(
                formatter,
                "network_check: inconsistency detected -- short_name_table contains superfluous entry {name}"
            ),
        }
    }
}

impl Error for NetchkCheckError {}

pub fn network_check(network: &NetchkNetwork) -> Result<NetchkReport, NetchkCheckError> {
    let mut warnings = Vec::new();

    for (node, node_data) in network.nodes() {
        match &node_data.status {
            NetchkNodeStatus::Valid => {}
            NetchkNodeStatus::Warning(message) => warnings.push(NetchkWarning {
                node,
                message: message.clone(),
            }),
            NetchkNodeStatus::Invalid(message) => {
                return Err(NetchkCheckError::NodeCheckFailed {
                    node,
                    message: message.clone(),
                });
            }
        }
    }

    check_membership(network)?;
    check_primary_output_list(network)?;
    check_primary_input_list(network)?;
    check_node_type_lists(network)?;
    check_bidirectional_pins(network)?;
    check_name_table(network)?;
    check_short_name_table(network)?;

    Ok(NetchkReport { warnings })
}

pub fn network_is_clean(network: &NetchkNetwork) -> Result<bool, NetchkCheckError> {
    network_check(network).map(|report| report.is_clean())
}

fn check_membership(network: &NetchkNetwork) -> Result<(), NetchkCheckError> {
    for (node, node_data) in network.nodes() {
        if !node_data.belongs_to_network {
            return Err(NetchkCheckError::NodeNotInNetwork { node });
        }

        for fanin in &node_data.fanins {
            let fanin_node = network.node(*fanin)?;
            if !fanin_node.belongs_to_network {
                return Err(NetchkCheckError::FaninNotInNetwork {
                    node,
                    fanin: *fanin,
                });
            }
        }

        for fanout in &node_data.fanouts {
            let fanout_node = network.node(fanout.node)?;
            if !fanout_node.belongs_to_network {
                return Err(NetchkCheckError::FanoutNotInNetwork {
                    node,
                    fanout: fanout.node,
                });
            }
        }
    }

    Ok(())
}

fn check_primary_output_list(network: &NetchkNetwork) -> Result<(), NetchkCheckError> {
    for output in &network.primary_outputs {
        let node = network.node(*output)?;
        if node.kind != NetchkNodeKind::PrimaryOutput {
            return Err(NetchkCheckError::PrimaryOutputListContainsNonOutput { node: *output });
        }

        if node.fanins.len() != 1 {
            return Err(NetchkCheckError::PrimaryOutputFaninCount {
                node: *output,
                count: node.fanins.len(),
            });
        }

        if !node.fanouts.is_empty() {
            return Err(NetchkCheckError::PrimaryOutputHasFanout { node: *output });
        }
    }

    Ok(())
}

fn check_primary_input_list(network: &NetchkNetwork) -> Result<(), NetchkCheckError> {
    for input in &network.primary_inputs {
        let node = network.node(*input)?;
        if node.kind != NetchkNodeKind::PrimaryInput {
            return Err(NetchkCheckError::PrimaryInputListContainsNonInput { node: *input });
        }

        if !node.fanins.is_empty() {
            return Err(NetchkCheckError::PrimaryInputHasFanin { node: *input });
        }
    }

    Ok(())
}

fn check_node_type_lists(network: &NetchkNetwork) -> Result<(), NetchkCheckError> {
    let primary_inputs = network
        .primary_inputs
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let primary_outputs = network
        .primary_outputs
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    for (node, node_data) in network.nodes() {
        match node_data.kind {
            NetchkNodeKind::PrimaryInput if !primary_inputs.contains(&node) => {
                return Err(NetchkCheckError::PrimaryInputMissingFromList { node });
            }
            NetchkNodeKind::PrimaryOutput if !primary_outputs.contains(&node) => {
                return Err(NetchkCheckError::PrimaryOutputMissingFromList { node });
            }
            NetchkNodeKind::PrimaryInput
            | NetchkNodeKind::PrimaryOutput
            | NetchkNodeKind::Internal => {}
        }
    }

    Ok(())
}

fn check_bidirectional_pins(network: &NetchkNetwork) -> Result<(), NetchkCheckError> {
    for (node, node_data) in network.nodes() {
        if node_data.kind == NetchkNodeKind::Internal {
            for (pin, fanin) in node_data.fanins.iter().copied().enumerate() {
                let matches = network
                    .node(fanin)?
                    .fanouts
                    .iter()
                    .filter(|fanout| fanout.node == node && fanout.pin == pin)
                    .count();

                if matches == 0 {
                    return Err(NetchkCheckError::MissingFanoutPin { node, fanin, pin });
                }

                if matches > 1 {
                    return Err(NetchkCheckError::DuplicateFanoutPin { node, fanin, pin });
                }
            }
        }

        for fanout_pin in &node_data.fanouts {
            let fanout = network.node(fanout_pin.node)?;
            let matches = fanout
                .fanins
                .iter()
                .enumerate()
                .filter(|(pin, fanin)| **fanin == node && *pin == fanout_pin.pin)
                .count();

            if matches == 0 {
                return Err(NetchkCheckError::MissingFaninPin {
                    node,
                    fanout: fanout_pin.node,
                    pin: fanout_pin.pin,
                });
            }

            if matches > 1 {
                return Err(NetchkCheckError::DuplicateFaninPin {
                    node,
                    fanout: fanout_pin.node,
                    pin: fanout_pin.pin,
                });
            }
        }
    }

    Ok(())
}

fn check_name_table(network: &NetchkNetwork) -> Result<(), NetchkCheckError> {
    let mut remaining = network.name_table.clone();

    for (node, node_data) in network.nodes() {
        let Some(mapped) = remaining.remove(&node_data.name) else {
            return Err(NetchkCheckError::NameTableMissing {
                node,
                name: node_data.name.clone(),
            });
        };

        if mapped != node {
            return Err(NetchkCheckError::NameTableMismatch {
                node,
                name: node_data.name.clone(),
                mapped,
            });
        }
    }

    if let Some(name) = remaining.keys().next() {
        return Err(NetchkCheckError::NameTableSuperfluous { name: name.clone() });
    }

    Ok(())
}

fn check_short_name_table(network: &NetchkNetwork) -> Result<(), NetchkCheckError> {
    let mut remaining = network.short_name_table.clone();

    for (node, node_data) in network.nodes() {
        let Some(mapped) = remaining.remove(&node_data.short_name) else {
            return Err(NetchkCheckError::ShortNameTableMissing {
                node,
                name: node_data.short_name.clone(),
            });
        };

        if mapped != node {
            return Err(NetchkCheckError::ShortNameTableMismatch {
                node,
                name: node_data.short_name.clone(),
                mapped,
            });
        }
    }

    if let Some(name) = remaining.keys().next() {
        return Err(NetchkCheckError::ShortNameTableSuperfluous { name: name.clone() });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> NetchkNetwork {
        let mut network = NetchkNetwork::new();
        let a = network
            .add_node(NetchkNode::new("a", NetchkNodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_node(NetchkNode::new("b", NetchkNodeKind::PrimaryInput))
            .unwrap();
        let n = network
            .add_node(NetchkNode::new("n", NetchkNodeKind::Internal).with_fanins([a, b]))
            .unwrap();
        network
            .add_node(NetchkNode::new("y", NetchkNodeKind::PrimaryOutput).with_fanins([n]))
            .unwrap();
        network
    }

    #[test]
    fn accepts_consistent_network() {
        let network = sample_network();

        let report = network_check(&network).unwrap();

        assert!(report.is_clean());
        assert_eq!(network_is_clean(&network), Ok(true));
    }

    #[test]
    fn preserves_node_check_warnings_as_unclean_report() {
        let mut network = sample_network();
        network.node_mut(NetchkNodeId(2)).unwrap().status =
            NetchkNodeStatus::Warning("cover was minimized with warnings".to_string());

        let report = network_check(&network).unwrap();

        assert!(!report.is_clean());
        assert_eq!(
            report.warnings,
            vec![NetchkWarning {
                node: NetchkNodeId(2),
                message: "cover was minimized with warnings".to_string(),
            }]
        );
        assert_eq!(network_is_clean(&network), Ok(false));
    }

    #[test]
    fn rejects_primary_output_with_wrong_shape() {
        let mut network = sample_network();
        network.node_mut(NetchkNodeId(3)).unwrap().fanins.clear();

        let error = network_check(&network).unwrap_err();

        assert_eq!(
            error,
            NetchkCheckError::PrimaryOutputFaninCount {
                node: NetchkNodeId(3),
                count: 0,
            }
        );
    }

    #[test]
    fn rejects_missing_fanout_pin_for_internal_fanin() {
        let mut network = sample_network();
        network.node_mut(NetchkNodeId(0)).unwrap().fanouts.clear();

        let error = network_check(&network).unwrap_err();

        assert_eq!(
            error,
            NetchkCheckError::MissingFanoutPin {
                node: NetchkNodeId(2),
                fanin: NetchkNodeId(0),
                pin: 0,
            }
        );
    }

    #[test]
    fn rejects_corrupt_fanout_back_reference() {
        let mut network = sample_network();
        network
            .node_mut(NetchkNodeId(0))
            .unwrap()
            .fanouts
            .push(NetchkFanoutPin::new(NetchkNodeId(3), 0));

        let error = network_check(&network).unwrap_err();

        assert_eq!(
            error,
            NetchkCheckError::MissingFaninPin {
                node: NetchkNodeId(0),
                fanout: NetchkNodeId(3),
                pin: 0,
            }
        );
    }

    #[test]
    fn rejects_node_missing_from_type_list() {
        let mut network = sample_network();
        network.primary_inputs.clear();

        let error = network_check(&network).unwrap_err();

        assert_eq!(
            error,
            NetchkCheckError::PrimaryInputMissingFromList {
                node: NetchkNodeId(0),
            }
        );
    }

    #[test]
    fn rejects_name_table_mismatch_and_superfluous_entries() {
        let mut network = sample_network();
        network.name_table.insert("a".to_string(), NetchkNodeId(1));

        let error = network_check(&network).unwrap_err();

        assert_eq!(
            error,
            NetchkCheckError::NameTableMismatch {
                node: NetchkNodeId(0),
                name: "a".to_string(),
                mapped: NetchkNodeId(1),
            }
        );

        let mut network = sample_network();
        network
            .name_table
            .insert("extra".to_string(), NetchkNodeId(0));

        let error = network_check(&network).unwrap_err();

        assert_eq!(
            error,
            NetchkCheckError::NameTableSuperfluous {
                name: "extra".to_string(),
            }
        );
    }

    #[test]
    fn rejects_short_name_table_mismatch() {
        let mut network = sample_network();
        network
            .short_name_table
            .insert("a".to_string(), NetchkNodeId(1));

        let error = network_check(&network).unwrap_err();

        assert_eq!(
            error,
            NetchkCheckError::ShortNameTableMismatch {
                node: NetchkNodeId(0),
                name: "a".to_string(),
                mapped: NetchkNodeId(1),
            }
        );
    }
}
