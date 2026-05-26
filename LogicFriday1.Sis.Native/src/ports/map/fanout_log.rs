//! Native Rust fanout-optimization log for `sis/map/fanout_log.c`.
//!
//! The C implementation uses process-global state to remember fanout buffers
//! created during iterative fanout optimization. Cleanup visits those created
//! nodes in reverse order, removes their mapping annotations, deletes them from
//! the SIS network, and then clears MAP annotations from the rest of the
//! network. This Rust port keeps the deterministic owned-data behavior for
//! `VirtualMappedNetwork` and exposes explicit dependency errors for the
//! remaining full SIS network deletion path. It intentionally exposes no legacy
//! C ABI entry points.

use std::error::Error;
use std::fmt;

use super::virtual_net::{NodeId, NodeKind, VirtualMappedNetwork, VirtualNetworkError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FanoutLogOptions {
    pub enabled: bool,
}

impl FanoutLogOptions {
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoggedFanoutNode {
    pub node: NodeId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutCleanupReport {
    pub registered_count: usize,
    pub retired_logged_nodes: Vec<NodeId>,
    pub unmapped_network_nodes: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FanoutLogEvent {
    Initialized {
        enabled: bool,
    },
    NodeRegistered {
        node: NodeId,
        name: String,
    },
    NodeRegistrationIgnored {
        node: NodeId,
        name: String,
    },
    CleanupStarted {
        registered_count: usize,
        network_node_count: usize,
    },
    NodeUnmapped {
        node: NodeId,
        name: String,
        reason: FanoutUnmapReason,
    },
    LoggedNodeRetired {
        node: NodeId,
        name: String,
    },
    CleanupFinished {
        retired_count: usize,
        unmapped_count: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutUnmapReason {
    LoggedTemporaryNode,
    NetworkAnnotation,
}

impl FanoutUnmapReason {
    fn trace_name(self) -> &'static str {
        match self {
            Self::LoggedTemporaryNode => "logged-temporary-node",
            Self::NetworkAnnotation => "network-annotation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FanoutLogError {
    LogDisabled,
    MissingNode(NodeId),
    LoggedNodeHasFanout {
        node: NodeId,
        fanout_count: usize,
    },
    LoggedNodeNotInternal(NodeId),
    VirtualNetwork(VirtualNetworkError),
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for FanoutLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LogDisabled => write!(f, "fanout optimization log is disabled"),
            Self::MissingNode(node) => {
                write!(f, "fanout log references missing node {}", node.index())
            }
            Self::LoggedNodeHasFanout { node, fanout_count } => write!(
                f,
                "logged fanout node {} still has {fanout_count} virtual fanouts",
                node.index()
            ),
            Self::LoggedNodeNotInternal(node) => {
                write!(
                    f,
                    "logged fanout node {} is not an internal node",
                    node.index()
                )
            }
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => write!(f, "{operation} requires unavailable native SIS integration"),
        }
    }
}

impl Error for FanoutLogError {}

impl From<VirtualNetworkError> for FanoutLogError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutOptimizationLog {
    enabled: bool,
    nodes: Vec<LoggedFanoutNode>,
    events: Vec<FanoutLogEvent>,
}

impl FanoutOptimizationLog {
    pub fn new(options: FanoutLogOptions) -> Self {
        Self {
            enabled: options.enabled,
            nodes: Vec::new(),
            events: vec![FanoutLogEvent::Initialized {
                enabled: options.enabled,
            }],
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn registered_nodes(&self) -> &[LoggedFanoutNode] {
        &self.nodes
    }

    pub fn events(&self) -> &[FanoutLogEvent] {
        &self.events
    }

    pub fn register_node(
        &mut self,
        network: &VirtualMappedNetwork,
        node: NodeId,
    ) -> Result<(), FanoutLogError> {
        let name = node_name(network, node)?;
        if !self.enabled {
            self.events
                .push(FanoutLogEvent::NodeRegistrationIgnored { node, name });
            return Ok(());
        }

        self.nodes.push(LoggedFanoutNode {
            node,
            name: name.clone(),
        });
        self.events
            .push(FanoutLogEvent::NodeRegistered { node, name });
        Ok(())
    }

    pub fn cleanup_virtual_network(
        &mut self,
        network: &mut VirtualMappedNetwork,
    ) -> Result<FanoutCleanupReport, FanoutLogError> {
        if !self.enabled {
            return Err(FanoutLogError::LogDisabled);
        }

        self.events.push(FanoutLogEvent::CleanupStarted {
            registered_count: self.nodes.len(),
            network_node_count: network.nodes().len(),
        });

        let mut report = FanoutCleanupReport {
            registered_count: self.nodes.len(),
            retired_logged_nodes: Vec::new(),
            unmapped_network_nodes: Vec::new(),
        };

        for logged in self.nodes.iter().rev() {
            validate_logged_node_can_retire(network, logged.node)?;
            unmap_internal_node(network, logged.node)?;
            report.retired_logged_nodes.push(logged.node);
            self.events.push(FanoutLogEvent::NodeUnmapped {
                node: logged.node,
                name: logged.name.clone(),
                reason: FanoutUnmapReason::LoggedTemporaryNode,
            });
            self.events.push(FanoutLogEvent::LoggedNodeRetired {
                node: logged.node,
                name: logged.name.clone(),
            });
        }

        for node in node_ids_for_len(network.nodes().len()) {
            if should_unmap_network_annotation(network, node)? {
                let name = node_name(network, node)?;
                unmap_internal_node(network, node)?;
                report.unmapped_network_nodes.push(node);
                self.events.push(FanoutLogEvent::NodeUnmapped {
                    node,
                    name,
                    reason: FanoutUnmapReason::NetworkAnnotation,
                });
            }
        }

        self.events.push(FanoutLogEvent::CleanupFinished {
            retired_count: report.retired_logged_nodes.len(),
            unmapped_count: report.unmapped_network_nodes.len(),
        });
        self.nodes.clear();

        Ok(report)
    }

    pub fn format_trace(&self) -> String {
        format_fanout_log_trace(&self.events)
    }
}

pub fn full_sis_fanout_cleanup_unavailable() -> Result<(), FanoutLogError> {
    Err(FanoutLogError::MissingSisPorts {
        operation: "fanout_log full SIS network cleanup",
    })
}

pub fn format_fanout_log_trace(events: &[FanoutLogEvent]) -> String {
    let mut trace = String::new();
    for event in events {
        match event {
            FanoutLogEvent::Initialized { enabled } => {
                trace.push_str("fanout-log:init enabled=");
                trace.push_str(bool_trace(*enabled));
            }
            FanoutLogEvent::NodeRegistered { node, name } => {
                push_node_event(&mut trace, "fanout-log:register", *node, name);
            }
            FanoutLogEvent::NodeRegistrationIgnored { node, name } => {
                push_node_event(&mut trace, "fanout-log:register-ignored", *node, name);
            }
            FanoutLogEvent::CleanupStarted {
                registered_count,
                network_node_count,
            } => {
                trace.push_str("fanout-log:cleanup-start registered=");
                trace.push_str(&registered_count.to_string());
                trace.push_str(" network-nodes=");
                trace.push_str(&network_node_count.to_string());
            }
            FanoutLogEvent::NodeUnmapped { node, name, reason } => {
                push_node_event(&mut trace, "fanout-log:unmap", *node, name);
                trace.push_str(" reason=");
                trace.push_str(reason.trace_name());
            }
            FanoutLogEvent::LoggedNodeRetired { node, name } => {
                push_node_event(&mut trace, "fanout-log:retire", *node, name);
            }
            FanoutLogEvent::CleanupFinished {
                retired_count,
                unmapped_count,
            } => {
                trace.push_str("fanout-log:cleanup-finish retired=");
                trace.push_str(&retired_count.to_string());
                trace.push_str(" unmapped=");
                trace.push_str(&unmapped_count.to_string());
            }
        }
        trace.push('\n');
    }
    trace
}

fn validate_logged_node_can_retire(
    network: &VirtualMappedNetwork,
    node: NodeId,
) -> Result<(), FanoutLogError> {
    let item = network
        .node(node)
        .ok_or(FanoutLogError::MissingNode(node))?;
    if item.kind != NodeKind::Internal {
        return Err(FanoutLogError::LoggedNodeNotInternal(node));
    }

    let fanout_count = item.gate_links().count();
    if fanout_count != 0 {
        return Err(FanoutLogError::LoggedNodeHasFanout { node, fanout_count });
    }

    Ok(())
}

fn should_unmap_network_annotation(
    network: &VirtualMappedNetwork,
    node: NodeId,
) -> Result<bool, FanoutLogError> {
    let item = network
        .node(node)
        .ok_or(FanoutLogError::MissingNode(node))?;
    Ok(item.kind == NodeKind::Internal && item.gate.is_some())
}

fn unmap_internal_node(
    network: &mut VirtualMappedNetwork,
    node: NodeId,
) -> Result<(), FanoutLogError> {
    let kind = network
        .node(node)
        .ok_or(FanoutLogError::MissingNode(node))?
        .kind;
    if kind != NodeKind::Internal {
        return Err(FanoutLogError::LoggedNodeNotInternal(node));
    }

    network.remove_node(node, false)?;
    Ok(())
}

fn node_name(network: &VirtualMappedNetwork, node: NodeId) -> Result<String, FanoutLogError> {
    network
        .node(node)
        .map(|item| item.name.clone())
        .ok_or(FanoutLogError::MissingNode(node))
}

fn push_node_event(trace: &mut String, label: &str, node: NodeId, name: &str) {
    trace.push_str(label);
    trace.push_str(" node=");
    trace.push_str(&node.index().to_string());
    trace.push_str(" name=\"");
    push_escaped_name(trace, name);
    trace.push('"');
}

fn push_escaped_name(trace: &mut String, name: &str) {
    for character in name.chars() {
        match character {
            '\\' => trace.push_str("\\\\"),
            '"' => trace.push_str("\\\""),
            '\n' => trace.push_str("\\n"),
            '\r' => trace.push_str("\\r"),
            '\t' => trace.push_str("\\t"),
            character => trace.push(character),
        }
    }
}

fn bool_trace(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn node_ids_for_len(len: usize) -> Vec<NodeId> {
    let mut probe = VirtualMappedNetwork::new();
    let mut ids = Vec::with_capacity(len);
    for index in 0..len {
        ids.push(probe.add_primary_input(index.to_string()));
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::virtual_net::{GateKind, SourceRef};

    fn sample_network() -> (VirtualMappedNetwork, NodeId, NodeId, NodeId) {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let detached = network.add_gate("tmp-buffer", GateKind::Inverter, vec![SourceRef::Node(a)]);
        let b = network.add_primary_input("b");
        let annotated = network.add_gate(
            "mapped",
            GateKind::And,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        network
            .add_primary_output("f", SourceRef::Node(annotated))
            .unwrap();
        network.setup_gate_links().unwrap();
        network.remove_gate_link(a, detached, 0);
        (network, a, detached, annotated)
    }

    #[test]
    fn cleanup_unmaps_logged_nodes_in_reverse_order_and_clears_annotations() {
        let (mut network, _, detached, annotated) = sample_network();
        let mut log = FanoutOptimizationLog::new(FanoutLogOptions::enabled());

        log.register_node(&network, detached).unwrap();
        let report = log.cleanup_virtual_network(&mut network).unwrap();

        assert_eq!(report.registered_count, 1);
        assert_eq!(report.retired_logged_nodes, vec![detached]);
        assert_eq!(report.unmapped_network_nodes, vec![annotated]);
        assert!(network.node(detached).unwrap().gate.is_none());
        assert!(network.node(annotated).unwrap().gate.is_none());
        assert!(log.registered_nodes().is_empty());
        assert_eq!(
            log.format_trace(),
            concat!(
                "fanout-log:init enabled=true\n",
                "fanout-log:register node=1 name=\"tmp-buffer\"\n",
                "fanout-log:cleanup-start registered=1 network-nodes=5\n",
                "fanout-log:unmap node=1 name=\"tmp-buffer\" reason=logged-temporary-node\n",
                "fanout-log:retire node=1 name=\"tmp-buffer\"\n",
                "fanout-log:unmap node=3 name=\"mapped\" reason=network-annotation\n",
                "fanout-log:cleanup-finish retired=1 unmapped=1\n"
            )
        );
    }

    #[test]
    fn disabled_log_ignores_registration_and_rejects_cleanup() {
        let (mut network, _, detached, _) = sample_network();
        let mut log = FanoutOptimizationLog::new(FanoutLogOptions::disabled());

        log.register_node(&network, detached).unwrap();

        assert!(log.registered_nodes().is_empty());
        assert_eq!(
            log.cleanup_virtual_network(&mut network),
            Err(FanoutLogError::LogDisabled)
        );
        assert_eq!(
            log.format_trace(),
            concat!(
                "fanout-log:init enabled=false\n",
                "fanout-log:register-ignored node=1 name=\"tmp-buffer\"\n"
            )
        );
    }

    #[test]
    fn trace_format_escapes_names_deterministically() {
        let events = vec![
            FanoutLogEvent::NodeRegistered {
                node: node_ids_for_len(1)[0],
                name: "a \"quoted\"\nnode".to_string(),
            },
            FanoutLogEvent::CleanupFinished {
                retired_count: 2,
                unmapped_count: 3,
            },
        ];

        assert_eq!(
            format_fanout_log_trace(&events),
            "fanout-log:register node=0 name=\"a \\\"quoted\\\"\\nnode\"\nfanout-log:cleanup-finish retired=2 unmapped=3\n"
        );
    }
}
