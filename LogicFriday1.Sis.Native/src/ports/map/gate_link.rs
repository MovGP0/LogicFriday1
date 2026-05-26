//! Native Rust gate-link helpers for `sis/map/gate_link.c`.
//!
//! SIS stores gate links as reverse pointers from a source node to every gate
//! pin or primary output driven by that source. `VirtualMappedNetwork` owns the
//! integrated reverse links; this module provides the standalone collection and
//! typed helper surface that mirrors the original `gate_link_*` operations
//! without adding legacy per-file C ABI exports.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use super::virtual_net::{
    DelayTime, GateLink, GateLinkKey, NodeId, SourceRef, VirtualMappedNetwork, VirtualNetworkError,
};

pub const PLUS_INFINITY: DelayTime = DelayTime {
    rise: f64::INFINITY,
    fall: f64::INFINITY,
};

#[derive(Clone, Debug, PartialEq)]
pub enum GateLinkError {
    InvalidPin {
        node: NodeId,
        pin: isize,
    },
    InvalidLoad {
        node: NodeId,
        pin: isize,
        load: f64,
    },
    InvalidSlack {
        node: NodeId,
        pin: isize,
        slack: f64,
    },
    InvalidRequired {
        node: NodeId,
        pin: isize,
    },
    VirtualNetwork(VirtualNetworkError),
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for GateLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPin { node, pin } => {
                write!(
                    f,
                    "gate link to node {} has invalid pin {pin}",
                    node.index()
                )
            }
            Self::InvalidLoad { node, pin, load } => write!(
                f,
                "gate link to node {} pin {pin} has invalid load {load}",
                node.index()
            ),
            Self::InvalidSlack { node, pin, slack } => write!(
                f,
                "gate link to node {} pin {pin} has invalid slack {slack}",
                node.index()
            ),
            Self::InvalidRequired { node, pin } => write!(
                f,
                "gate link to node {} pin {pin} has invalid required time",
                node.index()
            ),
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for GateLinkError {}

impl From<VirtualNetworkError> for GateLinkError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GateLinkCollection {
    links: BTreeMap<GateLinkKey, GateLink>,
}

impl GateLinkCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.links.len()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &GateLink> + ExactSizeIterator {
        self.links.values()
    }

    pub fn put(&mut self, link: GateLink) -> Result<Option<GateLink>, GateLinkError> {
        validate_link(link)?;
        Ok(self.links.insert(gate_link_key(link), link))
    }

    pub fn get(&self, node: NodeId, pin: isize) -> Option<&GateLink> {
        self.links.get(&GateLinkKey { node, pin })
    }

    pub fn remove(&mut self, node: NodeId, pin: isize) -> Option<GateLink> {
        self.links.remove(&GateLinkKey { node, pin })
    }

    pub fn clear(&mut self) {
        self.links.clear();
    }

    pub fn compute_load(&self, wire_load: impl Fn(usize) -> f64) -> f64 {
        self.links.values().map(|link| link.load).sum::<f64>() + wire_load(self.links.len())
    }

    pub fn compute_min_required(&self) -> DelayTime {
        self.links
            .values()
            .map(|link| link.required)
            .reduce(DelayTime::min)
            .unwrap_or(PLUS_INFINITY)
    }
}

pub fn full_sis_gate_link_unavailable() -> Result<GateLinkCollection, GateLinkError> {
    Err(GateLinkError::MissingSisPorts {
        operation: "gate_link full SIS graph integration",
    })
}

pub fn rebuild_network_gate_links(network: &mut VirtualMappedNetwork) -> Result<(), GateLinkError> {
    network.setup_gate_links().map_err(GateLinkError::from)
}

pub fn put_network_gate_link(
    network: &mut VirtualMappedNetwork,
    source: SourceRef,
    link: GateLink,
) -> Result<(), GateLinkError> {
    validate_link(link)?;
    network
        .add_to_gate_link(source, link)
        .map_err(GateLinkError::from)
}

pub fn get_network_gate_link(
    network: &VirtualMappedNetwork,
    source: NodeId,
    node: NodeId,
    pin: isize,
) -> Result<Option<GateLink>, GateLinkError> {
    ensure_node(network, source)?;
    ensure_node(network, node)?;
    Ok(network.gate_link(source, node, pin).copied())
}

pub fn remove_network_gate_link(
    network: &mut VirtualMappedNetwork,
    source: NodeId,
    node: NodeId,
    pin: isize,
) -> Result<Option<GateLink>, GateLinkError> {
    ensure_node(network, source)?;
    ensure_node(network, node)?;
    Ok(network.remove_gate_link(source, node, pin))
}

pub fn network_gate_link_count(
    network: &VirtualMappedNetwork,
    source: NodeId,
) -> Result<usize, GateLinkError> {
    Ok(ensure_node(network, source)?.gate_links().count())
}

pub fn network_gate_link_is_empty(
    network: &VirtualMappedNetwork,
    source: NodeId,
) -> Result<bool, GateLinkError> {
    Ok(network_gate_link_count(network, source)? == 0)
}

pub fn compute_network_gate_link_load(
    network: &VirtualMappedNetwork,
    source: NodeId,
    wire_load: impl Fn(usize) -> f64,
) -> Result<f64, GateLinkError> {
    ensure_node(network, source)?;
    Ok(network.compute_load(source, wire_load))
}

pub fn compute_network_min_required(
    network: &VirtualMappedNetwork,
    source: NodeId,
) -> Result<DelayTime, GateLinkError> {
    ensure_node(network, source)?;
    Ok(network
        .compute_min_required(source)
        .unwrap_or(PLUS_INFINITY))
}

pub fn snapshot_network_gate_links(
    network: &VirtualMappedNetwork,
    source: NodeId,
) -> Result<GateLinkCollection, GateLinkError> {
    let node = ensure_node(network, source)?;
    let mut collection = GateLinkCollection::new();
    for link in node.gate_links().copied() {
        collection.put(link)?;
    }

    Ok(collection)
}

fn validate_link(link: GateLink) -> Result<(), GateLinkError> {
    if link.pin < -1 {
        return Err(GateLinkError::InvalidPin {
            node: link.node,
            pin: link.pin,
        });
    }

    if !link.load.is_finite() || link.load < 0.0 {
        return Err(GateLinkError::InvalidLoad {
            node: link.node,
            pin: link.pin,
            load: link.load,
        });
    }

    if !link.slack.is_finite() {
        return Err(GateLinkError::InvalidSlack {
            node: link.node,
            pin: link.pin,
            slack: link.slack,
        });
    }

    if link.required.rise.is_nan() || link.required.fall.is_nan() {
        return Err(GateLinkError::InvalidRequired {
            node: link.node,
            pin: link.pin,
        });
    }

    Ok(())
}

fn ensure_node(
    network: &VirtualMappedNetwork,
    node: NodeId,
) -> Result<&super::virtual_net::VirtualMappedNode, GateLinkError> {
    network.node(node).ok_or(GateLinkError::VirtualNetwork(
        VirtualNetworkError::MissingNode(node),
    ))
}

fn gate_link_key(link: GateLink) -> GateLinkKey {
    GateLinkKey {
        node: link.node,
        pin: link.pin,
    }
}

#[cfg(test)]
mod tests {
    use super::super::virtual_net::{GateKind, MINUS_INFINITY, SourceRef};
    use super::*;

    fn node_ids(count: usize) -> Vec<NodeId> {
        let mut network = VirtualMappedNetwork::new();
        (0..count)
            .map(|index| network.add_primary_input(format!("n{index}")))
            .collect()
    }

    #[test]
    fn collection_put_replaces_by_node_and_pin() {
        let mut collection = GateLinkCollection::new();
        let node = node_ids(1)[0];

        collection
            .put(GateLink {
                node,
                pin: 1,
                load: 2.0,
                slack: 4.0,
                required: DelayTime::new(9.0, 10.0),
            })
            .unwrap();
        let old = collection
            .put(GateLink {
                node,
                pin: 1,
                load: 5.0,
                slack: 6.0,
                required: DelayTime::new(7.0, 8.0),
            })
            .unwrap()
            .unwrap();

        assert_eq!(old.load, 2.0);
        assert_eq!(collection.len(), 1);
        assert_eq!(collection.get(node, 1).unwrap().load, 5.0);
    }

    #[test]
    fn collection_load_adds_wire_load_by_fanout_count() {
        let mut collection = GateLinkCollection::new();
        let nodes = node_ids(2);
        collection
            .put(GateLink {
                node: nodes[0],
                pin: 0,
                load: 1.5,
                slack: 0.0,
                required: MINUS_INFINITY,
            })
            .unwrap();
        collection
            .put(GateLink {
                node: nodes[1],
                pin: -1,
                load: 2.5,
                slack: 0.0,
                required: MINUS_INFINITY,
            })
            .unwrap();

        assert_eq!(
            collection.compute_load(|fanouts| fanouts as f64 * 0.25),
            4.5
        );
    }

    #[test]
    fn collection_min_required_matches_c_plus_infinity_empty_semantics() {
        let mut collection = GateLinkCollection::new();
        let nodes = node_ids(2);
        assert_eq!(collection.compute_min_required(), PLUS_INFINITY);

        collection
            .put(GateLink {
                node: nodes[0],
                pin: 0,
                load: 0.0,
                slack: 0.0,
                required: DelayTime::new(3.0, 8.0),
            })
            .unwrap();
        collection
            .put(GateLink {
                node: nodes[1],
                pin: 1,
                load: 0.0,
                slack: 0.0,
                required: DelayTime::new(5.0, 4.0),
            })
            .unwrap();

        assert_eq!(collection.compute_min_required(), DelayTime::new(3.0, 4.0));
    }

    #[test]
    fn network_helpers_follow_virtual_mapped_network_links() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let gate = network.add_gate("n1", GateKind::Inverter, vec![SourceRef::Node(a)]);
        let output = network
            .add_primary_output("y", SourceRef::Node(gate))
            .unwrap();

        rebuild_network_gate_links(&mut network).unwrap();
        put_network_gate_link(
            &mut network,
            SourceRef::Node(a),
            GateLink {
                node: gate,
                pin: 0,
                load: 3.0,
                slack: 1.0,
                required: DelayTime::new(11.0, 12.0),
            },
        )
        .unwrap();

        assert_eq!(network_gate_link_count(&network, a).unwrap(), 1);
        assert_eq!(
            get_network_gate_link(&network, a, gate, 0)
                .unwrap()
                .unwrap()
                .required,
            DelayTime::new(11.0, 12.0)
        );
        assert_eq!(
            get_network_gate_link(&network, gate, output, -1)
                .unwrap()
                .unwrap()
                .node,
            output
        );
        assert_eq!(
            compute_network_gate_link_load(&network, a, |fanouts| fanouts as f64).unwrap(),
            4.0
        );
    }

    #[test]
    fn network_min_required_returns_plus_infinity_for_empty_link_table() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");

        rebuild_network_gate_links(&mut network).unwrap();

        assert_eq!(
            compute_network_min_required(&network, a).unwrap(),
            PLUS_INFINITY
        );
        assert!(network_gate_link_is_empty(&network, a).unwrap());
    }

    #[test]
    fn rejects_invalid_links_before_mutating_collection_or_network() {
        let mut collection = GateLinkCollection::new();
        let node = node_ids(1)[0];
        let error = collection
            .put(GateLink {
                node,
                pin: -2,
                load: 1.0,
                slack: 0.0,
                required: MINUS_INFINITY,
            })
            .unwrap_err();

        assert_eq!(error, GateLinkError::InvalidPin { node, pin: -2 });
        assert!(collection.is_empty());
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("gate_link.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
