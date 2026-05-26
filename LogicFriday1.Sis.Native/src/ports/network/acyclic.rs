//! Native acyclic network checks for the SIS network layer.
//!
//! The legacy SIS routine starts from every primary output driver and then
//! checks unreached nodes so disconnected components are covered. This port
//! keeps that behavior and returns a structured cycle report instead of
//! appending process-global error text.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use super::network_util::{Network, NetworkUtilError, NodeId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkCycle {
    nodes: Vec<CycleNode>,
}

impl NetworkCycle {
    pub fn nodes(&self) -> &[CycleNode] {
        &self.nodes
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.nodes.iter().map(|node| node.name.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CycleNode {
    pub id: NodeId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AcyclicError {
    Network(NetworkUtilError),
    InvalidPrimaryOutput(NodeId),
    CycleDetected(NetworkCycle),
}

impl fmt::Display for AcyclicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(formatter, "{error}"),
            Self::InvalidPrimaryOutput(node) => {
                write!(
                    formatter,
                    "primary output {} must have exactly one fanin",
                    node.index()
                )
            }
            Self::CycleDetected(cycle) => {
                write!(formatter, "network contains a cycle")?;
                for node in cycle.nodes() {
                    write!(formatter, "; node '{}' is on the cycle", node.name)?;
                }
                Ok(())
            }
        }
    }
}

impl Error for AcyclicError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Network(error) => Some(error),
            Self::InvalidPrimaryOutput(_) | Self::CycleDetected(_) => None,
        }
    }
}

impl From<NetworkUtilError> for AcyclicError {
    fn from(value: NetworkUtilError) -> Self {
        Self::Network(value)
    }
}

pub type AcyclicResult<T> = Result<T, AcyclicError>;

pub fn network_is_acyclic(network: &Network) -> AcyclicResult<()> {
    match network_cycle(network)? {
        Some(cycle) => Err(AcyclicError::CycleDetected(cycle)),
        None => Ok(()),
    }
}

pub fn network_cycle(network: &Network) -> AcyclicResult<Option<NetworkCycle>> {
    let mut visit_states = BTreeMap::new();
    let mut active_path = Vec::new();

    for output in network.primary_outputs() {
        let output_node = network.node(*output)?;
        let [driver] = output_node.fanins.as_slice() else {
            return Err(AcyclicError::InvalidPrimaryOutput(*output));
        };

        if let Some(cycle) = check_acyclic(network, *driver, &mut visit_states, &mut active_path)? {
            return Ok(Some(cycle));
        }
    }

    for (node, _) in network.nodes() {
        if visit_states.contains_key(&node) {
            continue;
        }

        if let Some(cycle) = check_acyclic(network, node, &mut visit_states, &mut active_path)? {
            return Ok(Some(cycle));
        }
    }

    Ok(None)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Active,
    Done,
}

fn check_acyclic(
    network: &Network,
    node: NodeId,
    visit_states: &mut BTreeMap<NodeId, VisitState>,
    active_path: &mut Vec<NodeId>,
) -> AcyclicResult<Option<NetworkCycle>> {
    match visit_states.get(&node).copied() {
        Some(VisitState::Active) => {
            return Ok(Some(extract_cycle(network, active_path, node)?));
        }
        Some(VisitState::Done) => {
            return Ok(None);
        }
        None => {}
    }

    network.node(node)?;
    visit_states.insert(node, VisitState::Active);
    active_path.push(node);

    let fanins = network.node(node)?.fanins.clone();
    for fanin in fanins {
        if let Some(cycle) = check_acyclic(network, fanin, visit_states, active_path)? {
            return Ok(Some(cycle));
        }
    }

    active_path.pop();
    visit_states.insert(node, VisitState::Done);
    Ok(None)
}

fn extract_cycle(
    network: &Network,
    active_path: &[NodeId],
    repeated_node: NodeId,
) -> AcyclicResult<NetworkCycle> {
    let start = active_path
        .iter()
        .position(|node| *node == repeated_node)
        .unwrap_or(0);
    let mut seen = BTreeSet::new();
    let mut nodes = Vec::new();

    for node in &active_path[start..] {
        if !seen.insert(*node) {
            continue;
        }

        nodes.push(CycleNode {
            id: *node,
            name: network.node(*node)?.name.clone(),
        });
    }

    Ok(NetworkCycle { nodes })
}

#[cfg(test)]
mod tests {
    use super::super::network_util::{NetworkNode, NodeKind};
    use super::*;

    fn primary_input(name: &str) -> NetworkNode {
        NetworkNode::new(name, NodeKind::PrimaryInput)
    }

    fn internal(name: &str, fanins: impl Into<Vec<NodeId>>) -> NetworkNode {
        let mut node = NetworkNode::new(name, NodeKind::Internal);
        node.fanins = fanins.into();
        node
    }

    #[test]
    fn accepts_acyclic_network_reachable_from_output() {
        let mut network = Network::new();
        let a = network.add_primary_input(primary_input("a")).unwrap();
        let b = network.add_primary_input(primary_input("b")).unwrap();
        let n = network.add_node(internal("n", vec![a, b])).unwrap();
        network.add_primary_output(n).unwrap();

        assert_eq!(network_cycle(&network).unwrap(), None);
        assert_eq!(network_is_acyclic(&network), Ok(()));
    }

    #[test]
    fn reports_cycle_reachable_from_primary_output_driver() {
        let mut network = Network::new();
        let a = network.add_primary_input(primary_input("a")).unwrap();
        let n0 = network.add_node(internal("n0", vec![a])).unwrap();
        let n1 = network.add_node(internal("n1", vec![n0])).unwrap();
        network.node_mut(n0).unwrap().fanins.push(n1);
        network.add_primary_output(n1).unwrap();
        network.change_node_name(n1, "n1_driver").unwrap();

        let error = network_is_acyclic(&network).unwrap_err();

        let AcyclicError::CycleDetected(cycle) = error else {
            panic!("expected cycle");
        };
        assert_eq!(cycle.names().collect::<Vec<_>>(), vec!["n1_driver", "n0"]);
    }

    #[test]
    fn checks_components_not_reached_from_outputs() {
        let mut network = Network::new();
        let a = network.add_primary_input(primary_input("a")).unwrap();
        network.add_primary_output(a).unwrap();
        let n0 = network.add_node(internal("floating0", Vec::new())).unwrap();
        let n1 = network.add_node(internal("floating1", vec![n0])).unwrap();
        network.node_mut(n0).unwrap().fanins.push(n1);

        let cycle = network_cycle(&network).unwrap().unwrap();

        assert_eq!(
            cycle.names().collect::<Vec<_>>(),
            vec!["floating0", "floating1"]
        );
    }

    #[test]
    fn reports_malformed_primary_output() {
        let mut network = Network::new();
        let a = network.add_primary_input(primary_input("a")).unwrap();
        let output = network.add_primary_output(a).unwrap();
        network.node_mut(output).unwrap().fanins.clear();

        assert!(matches!(
            network_is_acyclic(&network),
            Err(AcyclicError::InvalidPrimaryOutput(node)) if node == output
        ));
    }
}
