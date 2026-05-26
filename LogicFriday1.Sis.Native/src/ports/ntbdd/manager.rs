//! Native Rust manager lifecycle for the SIS ntbdd package.
//!
//! The C implementation attaches ntbdd bookkeeping to a BDD manager, remembers
//! networks that have node-level BDDs, and frees only the node BDDs owned by the
//! manager during shutdown. This port models that ownership explicitly with
//! safe identifiers and owned network data.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_MANAGER_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddManagerId(u64);

impl BddManagerId {
    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NetworkId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeBdd {
    id: BddId,
    manager: BddManagerId,
}

impl NodeBdd {
    pub fn new(id: BddId, manager: BddManagerId) -> Self {
        Self { id, manager }
    }

    pub fn id(self) -> BddId {
        self.id
    }

    pub fn manager(self) -> BddManagerId {
        self.manager
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    id: NodeId,
    bdd: Option<NodeBdd>,
}

impl NetworkNode {
    pub fn new(id: NodeId) -> Self {
        Self { id, bdd: None }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn bdd(&self) -> Option<NodeBdd> {
        self.bdd
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Network {
    id: NetworkId,
    nodes: BTreeMap<NodeId, NetworkNode>,
}

impl Network {
    pub fn new(id: NetworkId) -> Self {
        Self {
            id,
            nodes: BTreeMap::new(),
        }
    }

    pub fn id(&self) -> NetworkId {
        self.id
    }

    pub fn add_node(&mut self, node: NodeId) -> Result<(), NtbddManagerError> {
        if self.nodes.contains_key(&node) {
            return Err(NtbddManagerError::DuplicateNode {
                network: self.id,
                node,
            });
        }

        self.nodes.insert(node, NetworkNode::new(node));
        Ok(())
    }

    pub fn node(&self, node: NodeId) -> Result<&NetworkNode, NtbddManagerError> {
        self.nodes.get(&node).ok_or(NtbddManagerError::MissingNode {
            network: self.id,
            node,
        })
    }

    pub fn node_mut(&mut self, node: NodeId) -> Result<&mut NetworkNode, NtbddManagerError> {
        self.nodes
            .get_mut(&node)
            .ok_or(NtbddManagerError::MissingNode {
                network: self.id,
                node,
            })
    }

    pub fn nodes(&self) -> impl Iterator<Item = &NetworkNode> {
        self.nodes.values()
    }

    pub fn set_node_bdd(
        &mut self,
        node: NodeId,
        bdd: Option<NodeBdd>,
    ) -> Result<Option<NodeBdd>, NtbddManagerError> {
        let slot = &mut self.node_mut(node)?.bdd;
        Ok(std::mem::replace(slot, bdd))
    }

    fn free_bdds_owned_by(&mut self, manager: BddManagerId) -> NetworkFreeReport {
        let mut report = NetworkFreeReport::default();
        for node in self.nodes.values_mut() {
            match node.bdd {
                Some(bdd) if bdd.manager() == manager => {
                    node.bdd = None;
                    report.freed += 1;
                }
                Some(_) => {
                    report.retained_foreign += 1;
                }
                None => {
                    report.empty += 1;
                }
            }
        }

        report
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkBddManager {
    id: BddManagerId,
    variable_count: usize,
    last_network: Option<NetworkId>,
    network_table: BTreeSet<NetworkId>,
    ended: bool,
}

impl NetworkBddManager {
    pub fn start(variable_count: usize) -> Self {
        let id = BddManagerId(NEXT_MANAGER_ID.fetch_add(1, Ordering::Relaxed));
        Self {
            id,
            variable_count,
            last_network: None,
            network_table: BTreeSet::new(),
            ended: false,
        }
    }

    pub fn id(&self) -> BddManagerId {
        self.id
    }

    pub fn variable_count(&self) -> usize {
        self.variable_count
    }

    pub fn last_network(&self) -> Option<NetworkId> {
        self.last_network
    }

    pub fn registered_networks(&self) -> &BTreeSet<NetworkId> {
        &self.network_table
    }

    pub fn is_ended(&self) -> bool {
        self.ended
    }

    pub fn register_network(&mut self, network: NetworkId) -> Result<bool, NtbddManagerError> {
        self.ensure_active()?;
        self.last_network = Some(network);
        Ok(self.network_table.insert(network))
    }

    pub fn unregister_network(&mut self, network: NetworkId) -> Result<bool, NtbddManagerError> {
        self.ensure_active()?;
        if self.last_network == Some(network) {
            self.last_network = None;
        }

        Ok(self.network_table.remove(&network))
    }

    pub fn clear_last_network(&mut self) -> Result<(), NtbddManagerError> {
        self.ensure_active()?;
        self.last_network = None;
        Ok(())
    }

    pub fn attach_node_bdd(
        &mut self,
        networks: &mut BTreeMap<NetworkId, Network>,
        network: NetworkId,
        node: NodeId,
        bdd: BddId,
    ) -> Result<Option<NodeBdd>, NtbddManagerError> {
        self.ensure_active()?;
        self.register_network(network)?;
        let network_ref = networks
            .get_mut(&network)
            .ok_or(NtbddManagerError::MissingNetwork(network))?;
        network_ref.set_node_bdd(node, Some(NodeBdd::new(bdd, self.id)))
    }

    pub fn end(
        &mut self,
        networks: &mut BTreeMap<NetworkId, Network>,
    ) -> Result<EndManagerReport, NtbddManagerError> {
        self.ensure_active()?;

        let mut report = EndManagerReport {
            manager: self.id,
            registered_networks: self.network_table.len(),
            ..EndManagerReport::default()
        };

        for network_id in &self.network_table {
            let network = networks
                .get_mut(network_id)
                .ok_or(NtbddManagerError::MissingNetwork(*network_id))?;
            let network_report = network.free_bdds_owned_by(self.id);
            report.freed_bdds += network_report.freed;
            report.retained_foreign_bdds += network_report.retained_foreign;
            report.empty_nodes += network_report.empty;
        }

        self.network_table.clear();
        self.last_network = None;
        self.ended = true;
        Ok(report)
    }

    fn ensure_active(&self) -> Result<(), NtbddManagerError> {
        if self.ended {
            Err(NtbddManagerError::ManagerEnded(self.id))
        } else {
            Ok(())
        }
    }
}

pub fn ntbdd_start_manager(variable_count: usize) -> NetworkBddManager {
    NetworkBddManager::start(variable_count)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetworkFreeReport {
    pub freed: usize,
    pub retained_foreign: usize,
    pub empty: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EndManagerReport {
    pub manager: BddManagerId,
    pub registered_networks: usize,
    pub freed_bdds: usize,
    pub retained_foreign_bdds: usize,
    pub empty_nodes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NtbddManagerError {
    ManagerEnded(BddManagerId),
    MissingNetwork(NetworkId),
    MissingNode { network: NetworkId, node: NodeId },
    DuplicateNode { network: NetworkId, node: NodeId },
}

impl fmt::Display for NtbddManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManagerEnded(manager) => {
                write!(f, "ntbdd manager {} has already ended", manager.value())
            }
            Self::MissingNetwork(network) => {
                write!(f, "ntbdd manager references missing network {:?}", network)
            }
            Self::MissingNode { network, node } => {
                write!(f, "network {:?} has no node {:?}", network, node)
            }
            Self::DuplicateNode { network, node } => {
                write!(f, "network {:?} already has node {:?}", network, node)
            }
        }
    }
}

impl Error for NtbddManagerError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn network(id: usize, nodes: &[usize]) -> Network {
        let mut network = Network::new(NetworkId(id));
        for node in nodes {
            network.add_node(NodeId(*node)).unwrap();
        }

        network
    }

    #[test]
    fn start_manager_initializes_empty_hook_state() {
        let manager = ntbdd_start_manager(5);

        assert_eq!(manager.variable_count(), 5);
        assert_eq!(manager.last_network(), None);
        assert!(manager.registered_networks().is_empty());
        assert!(!manager.is_ended());
    }

    #[test]
    fn attach_node_bdd_registers_network_and_replaces_existing_bdd() {
        let mut manager = ntbdd_start_manager(2);
        let mut networks = BTreeMap::from([(NetworkId(10), network(10, &[1]))]);

        let previous = manager.attach_node_bdd(&mut networks, NetworkId(10), NodeId(1), BddId(20));
        assert_eq!(previous, Ok(None));
        assert_eq!(manager.last_network(), Some(NetworkId(10)));
        assert!(manager.registered_networks().contains(&NetworkId(10)));

        let previous = manager.attach_node_bdd(&mut networks, NetworkId(10), NodeId(1), BddId(21));
        assert_eq!(previous, Ok(Some(NodeBdd::new(BddId(20), manager.id()))));
        assert_eq!(
            networks[&NetworkId(10)].node(NodeId(1)).unwrap().bdd(),
            Some(NodeBdd::new(BddId(21), manager.id()))
        );
    }

    #[test]
    fn end_frees_bdds_owned_by_manager_across_registered_networks() {
        let mut manager = ntbdd_start_manager(3);
        let foreign = ntbdd_start_manager(3);
        let mut networks = BTreeMap::from([
            (NetworkId(1), network(1, &[1, 2, 3])),
            (NetworkId(2), network(2, &[1])),
        ]);

        manager
            .attach_node_bdd(&mut networks, NetworkId(1), NodeId(1), BddId(100))
            .unwrap();
        manager
            .attach_node_bdd(&mut networks, NetworkId(2), NodeId(1), BddId(101))
            .unwrap();
        networks
            .get_mut(&NetworkId(1))
            .unwrap()
            .set_node_bdd(NodeId(2), Some(NodeBdd::new(BddId(200), foreign.id())))
            .unwrap();

        let report = manager.end(&mut networks).unwrap();

        assert_eq!(report.registered_networks, 2);
        assert_eq!(report.freed_bdds, 2);
        assert_eq!(report.retained_foreign_bdds, 1);
        assert_eq!(report.empty_nodes, 1);
        assert_eq!(networks[&NetworkId(1)].node(NodeId(1)).unwrap().bdd(), None);
        assert_eq!(
            networks[&NetworkId(1)].node(NodeId(2)).unwrap().bdd(),
            Some(NodeBdd::new(BddId(200), foreign.id()))
        );
        assert_eq!(networks[&NetworkId(2)].node(NodeId(1)).unwrap().bdd(), None);
        assert!(manager.registered_networks().is_empty());
        assert_eq!(manager.last_network(), None);
        assert!(manager.is_ended());
    }

    #[test]
    fn end_only_visits_registered_networks() {
        let mut manager = ntbdd_start_manager(1);
        let mut networks = BTreeMap::from([
            (NetworkId(1), network(1, &[1])),
            (NetworkId(2), network(2, &[1])),
        ]);
        manager
            .attach_node_bdd(&mut networks, NetworkId(1), NodeId(1), BddId(10))
            .unwrap();
        networks
            .get_mut(&NetworkId(2))
            .unwrap()
            .set_node_bdd(NodeId(1), Some(NodeBdd::new(BddId(11), manager.id())))
            .unwrap();

        let report = manager.end(&mut networks).unwrap();

        assert_eq!(report.freed_bdds, 1);
        assert_eq!(
            networks[&NetworkId(2)].node(NodeId(1)).unwrap().bdd(),
            Some(NodeBdd::new(BddId(11), manager.id()))
        );
    }

    #[test]
    fn operations_after_end_are_rejected() {
        let mut manager = ntbdd_start_manager(1);
        let mut networks = BTreeMap::new();

        manager.end(&mut networks).unwrap();

        assert_eq!(
            manager.register_network(NetworkId(1)),
            Err(NtbddManagerError::ManagerEnded(manager.id()))
        );
        assert_eq!(
            manager.attach_node_bdd(&mut networks, NetworkId(1), NodeId(1), BddId(1)),
            Err(NtbddManagerError::ManagerEnded(manager.id()))
        );
    }

    #[test]
    fn missing_registered_network_is_reported() {
        let mut manager = ntbdd_start_manager(1);
        let mut networks = BTreeMap::new();

        manager.register_network(NetworkId(7)).unwrap();

        assert_eq!(
            manager.end(&mut networks),
            Err(NtbddManagerError::MissingNetwork(NetworkId(7)))
        );
        assert!(!manager.is_ended());
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("manager.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
