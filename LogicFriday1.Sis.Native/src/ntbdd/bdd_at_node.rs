//! Native Rust model for `LogicSynthesis/sis/ntbdd/bdd_at_node.c`.
//!
//! The original unit stores one optional `bdd_t *` on each SIS node. Replacing
//! that value releases the previous BDD, deleting a node releases the stored
//! BDD, and assigning a BDD to a node in a network records that network in the
//! owning manager's network table. This port keeps the same ownership behavior
//! with typed handles and explicit return values for released BDDs.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddManagerId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddFormulaId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NetworkId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddHandle {
    id: BddFormulaId,
    manager: BddManagerId,
}

impl BddHandle {
    pub fn new(id: usize, manager: BddManagerId) -> Self {
        Self {
            id: BddFormulaId(id),
            manager,
        }
    }

    pub fn id(&self) -> BddFormulaId {
        self.id
    }

    pub fn manager(&self) -> BddManagerId {
        self.manager
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NtBddNode {
    network: Option<NetworkId>,
    bdd: Option<BddHandle>,
}

impl NtBddNode {
    pub fn new(network: Option<NetworkId>) -> Self {
        Self { network, bdd: None }
    }

    pub fn network(&self) -> Option<NetworkId> {
        self.network
    }

    pub fn set_network(&mut self, network: Option<NetworkId>) {
        self.network = network;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NtBddManager {
    id: BddManagerId,
    last_network: Option<NetworkId>,
    network_table: BTreeSet<NetworkId>,
}

impl NtBddManager {
    pub fn new(id: usize) -> Self {
        Self {
            id: BddManagerId(id),
            last_network: None,
            network_table: BTreeSet::new(),
        }
    }

    pub fn id(&self) -> BddManagerId {
        self.id
    }

    pub fn last_network(&self) -> Option<NetworkId> {
        self.last_network
    }

    pub fn tracked_networks(&self) -> &BTreeSet<NetworkId> {
        &self.network_table
    }

    fn record_network(&mut self, bdd: &BddHandle, network: NetworkId) -> Result<(), NtBddError> {
        if bdd.manager != self.id {
            return Err(NtBddError::ManagerMismatch {
                bdd: bdd.id,
                bdd_manager: bdd.manager,
                supplied_manager: self.id,
            });
        }

        if self.last_network == Some(network) {
            return Ok(());
        }

        self.last_network = Some(network);
        self.network_table.insert(network);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NtBddError {
    MissingNode,
    MissingManager {
        bdd: BddFormulaId,
        network: NetworkId,
    },
    ManagerMismatch {
        bdd: BddFormulaId,
        bdd_manager: BddManagerId,
        supplied_manager: BddManagerId,
    },
}

impl fmt::Display for NtBddError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode => write!(f, "ntbdd node access requires a node"),
            Self::MissingManager { bdd, network } => write!(
                f,
                "setting BDD {:?} on network {:?} requires manager tracking state",
                bdd, network
            ),
            Self::ManagerMismatch {
                bdd,
                bdd_manager,
                supplied_manager,
            } => write!(
                f,
                "BDD {:?} belongs to manager {:?}, but manager {:?} was supplied",
                bdd, bdd_manager, supplied_manager
            ),
        }
    }
}

impl Error for NtBddError {}

pub fn bdd_alloc_demon(node: &mut NtBddNode) {
    node.bdd = None;
}

pub fn bdd_free_demon(node: &mut NtBddNode) -> Option<BddHandle> {
    ntbdd_free_at_node(node)
}

pub fn ntbdd_at_node(node: Option<&NtBddNode>) -> Result<Option<&BddHandle>, NtBddError> {
    node.map(|node| node.bdd.as_ref())
        .ok_or(NtBddError::MissingNode)
}

pub fn ntbdd_set_at_node(
    node: &mut NtBddNode,
    new_bdd: Option<BddHandle>,
    mut manager: Option<&mut NtBddManager>,
) -> Result<Option<BddHandle>, NtBddError> {
    if node.bdd == new_bdd {
        return Ok(None);
    }

    if let (Some(network), Some(bdd)) = (node.network, new_bdd.as_ref()) {
        let manager = manager.as_deref_mut().ok_or(NtBddError::MissingManager {
            bdd: bdd.id,
            network,
        })?;
        manager.record_network(bdd, network)?;
    }

    let released = node.bdd.replace_with(new_bdd);
    Ok(released)
}

pub fn ntbdd_free_at_node(node: &mut NtBddNode) -> Option<BddHandle> {
    node.bdd.take()
}

trait ReplaceWith<T> {
    fn replace_with(&mut self, value: T) -> T;
}

impl<T> ReplaceWith<T> for T {
    fn replace_with(&mut self, value: T) -> T {
        std::mem::replace(self, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bdd(id: usize) -> BddHandle {
        BddHandle::new(id, BddManagerId(7))
    }

    #[test]
    fn alloc_demon_clears_the_node_bdd_field() {
        let mut node = NtBddNode {
            network: None,
            bdd: Some(bdd(1)),
        };

        bdd_alloc_demon(&mut node);

        assert_eq!(ntbdd_at_node(Some(&node)).unwrap(), None);
    }

    #[test]
    fn at_node_reports_missing_node_like_the_c_nil_check() {
        assert_eq!(ntbdd_at_node(None), Err(NtBddError::MissingNode));
    }

    #[test]
    fn set_at_node_releases_old_bdd_and_stores_new_bdd() {
        let mut manager = NtBddManager::new(7);
        let mut node = NtBddNode::new(Some(NetworkId(3)));
        let old = bdd(1);
        let new = bdd(2);

        assert_eq!(
            ntbdd_set_at_node(&mut node, Some(old.clone()), Some(&mut manager)).unwrap(),
            None
        );
        assert_eq!(
            ntbdd_set_at_node(&mut node, Some(new.clone()), Some(&mut manager)).unwrap(),
            Some(old)
        );

        assert_eq!(ntbdd_at_node(Some(&node)).unwrap(), Some(&new));
        assert_eq!(manager.tracked_networks(), &BTreeSet::from([NetworkId(3)]));
        assert_eq!(manager.last_network(), Some(NetworkId(3)));
    }

    #[test]
    fn setting_the_same_bdd_is_a_no_op() {
        let mut manager = NtBddManager::new(7);
        let mut node = NtBddNode::new(Some(NetworkId(3)));
        let existing = bdd(1);

        ntbdd_set_at_node(&mut node, Some(existing.clone()), Some(&mut manager)).unwrap();
        let before = manager.clone();

        assert_eq!(
            ntbdd_set_at_node(&mut node, Some(existing.clone()), Some(&mut manager)).unwrap(),
            None
        );

        assert_eq!(ntbdd_at_node(Some(&node)).unwrap(), Some(&existing));
        assert_eq!(manager, before);
    }

    #[test]
    fn setting_none_releases_the_old_bdd_without_tracking_a_network() {
        let mut manager = NtBddManager::new(7);
        let mut node = NtBddNode::new(Some(NetworkId(3)));
        let old = bdd(1);

        ntbdd_set_at_node(&mut node, Some(old.clone()), Some(&mut manager)).unwrap();
        let released = ntbdd_set_at_node(&mut node, None, Some(&mut manager)).unwrap();

        assert_eq!(released, Some(old));
        assert_eq!(ntbdd_at_node(Some(&node)).unwrap(), None);
        assert_eq!(manager.tracked_networks(), &BTreeSet::from([NetworkId(3)]));
    }

    #[test]
    fn nodes_without_networks_do_not_update_manager_tracking() {
        let mut manager = NtBddManager::new(7);
        let mut node = NtBddNode::new(None);
        let handle = bdd(1);

        ntbdd_set_at_node(&mut node, Some(handle), Some(&mut manager)).unwrap();

        assert!(manager.tracked_networks().is_empty());
        assert_eq!(manager.last_network(), None);
    }

    #[test]
    fn network_nodes_require_manager_tracking_when_setting_a_bdd() {
        let mut node = NtBddNode::new(Some(NetworkId(3)));

        let error = ntbdd_set_at_node(&mut node, Some(bdd(1)), None).unwrap_err();

        assert_eq!(
            error,
            NtBddError::MissingManager {
                bdd: BddFormulaId(1),
                network: NetworkId(3),
            }
        );
        assert_eq!(ntbdd_at_node(Some(&node)).unwrap(), None);
    }

    #[test]
    fn manager_tracking_uses_a_one_entry_last_network_cache() {
        let mut manager = NtBddManager::new(7);
        let mut first = NtBddNode::new(Some(NetworkId(3)));
        let mut second = NtBddNode::new(Some(NetworkId(3)));
        let mut third = NtBddNode::new(Some(NetworkId(4)));

        ntbdd_set_at_node(&mut first, Some(bdd(1)), Some(&mut manager)).unwrap();
        ntbdd_set_at_node(&mut second, Some(bdd(2)), Some(&mut manager)).unwrap();
        ntbdd_set_at_node(&mut third, Some(bdd(3)), Some(&mut manager)).unwrap();

        assert_eq!(
            manager.tracked_networks(),
            &BTreeSet::from([NetworkId(3), NetworkId(4)])
        );
        assert_eq!(manager.last_network(), Some(NetworkId(4)));
    }

    #[test]
    fn mismatched_manager_rejects_the_set_without_mutating_the_node() {
        let mut manager = NtBddManager::new(8);
        let mut node = NtBddNode::new(Some(NetworkId(3)));
        let handle = bdd(1);

        let error =
            ntbdd_set_at_node(&mut node, Some(handle.clone()), Some(&mut manager)).unwrap_err();

        assert_eq!(
            error,
            NtBddError::ManagerMismatch {
                bdd: BddFormulaId(1),
                bdd_manager: BddManagerId(7),
                supplied_manager: BddManagerId(8),
            }
        );
        assert_eq!(ntbdd_at_node(Some(&node)).unwrap(), None);
    }

    #[test]
    fn free_helpers_take_the_stored_bdd_once() {
        let mut node = NtBddNode {
            network: None,
            bdd: Some(bdd(1)),
        };

        assert_eq!(ntbdd_free_at_node(&mut node), Some(bdd(1)));
        assert_eq!(bdd_free_demon(&mut node), None);
        assert_eq!(ntbdd_at_node(Some(&node)).unwrap(), None);
    }

    #[test]
    fn no_legacy_c_abi_or_dependency_metadata_tokens_are_present_in_this_port() {
        let source = include_str!("bdd_at_node.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-8j8")));
    }
}
