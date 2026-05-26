//! Native Rust node utility lifecycle support.
//!
//! The legacy utility code owned node allocation, duplication, invalidation,
//! daemon callback registration, and direct network lookup. This port keeps
//! those behaviors over owned Rust data and leaves C interop to higher level
//! facades.

use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_SIS_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NetworkId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UtilityNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Unassigned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutRecord {
    pub fanout: NodeId,
    pub pin: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    cubes: Vec<Vec<LiteralValue>>,
}

impl Cover {
    pub fn new(cubes: Vec<Vec<LiteralValue>>) -> Self {
        Self { cubes }
    }

    pub fn cubes(&self) -> &[Vec<LiteralValue>] {
        &self.cubes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralValue {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UtilityNodeSlots {
    pub simulation: Option<String>,
    pub factored: Option<String>,
    pub delay: Option<String>,
    pub map: Option<String>,
    pub simplify: Option<String>,
    pub bdd: Option<String>,
    pub pld: Option<String>,
    pub ite: Option<String>,
    pub buffer: Option<String>,
    pub cspf: Option<String>,
    pub bin: Option<String>,
    pub atpg: Option<String>,
    pub undefined: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtilityNode {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub kind: UtilityNodeKind,
    pub sis_id: usize,
    pub fanin_changed: bool,
    pub fanout_changed: bool,
    pub is_dup_free: bool,
    pub is_min_base: bool,
    pub is_scc_minimal: bool,
    pub fanins: Vec<NodeId>,
    pub fanouts: Vec<FanoutRecord>,
    pub fanin_fanout: Vec<usize>,
    pub on_set: Option<Cover>,
    pub dc_set: Option<Cover>,
    pub off_set: Option<Cover>,
    pub copy: Option<NodeId>,
    pub network: Option<NetworkId>,
    pub net_handle: Option<usize>,
    pub slots: UtilityNodeSlots,
}

impl UtilityNode {
    pub fn allocate(registry: &mut DaemonRegistry) -> Self {
        let mut node = Self::default_unregistered();
        registry.run_alloc(&mut node);
        node
    }

    pub fn duplicate(&self, registry: &mut DaemonRegistry) -> Self {
        let mut duplicate = Self::default_unregistered();

        duplicate.name = self.name.clone();
        duplicate.short_name = self.short_name.clone();
        duplicate.kind = self.kind;
        duplicate.fanin_changed = self.fanin_changed;
        duplicate.fanout_changed = self.fanout_changed;
        duplicate.is_dup_free = self.is_dup_free;
        duplicate.is_min_base = self.is_min_base;
        duplicate.is_scc_minimal = self.is_scc_minimal;
        duplicate.fanins = duplicate_fanins(&self.fanins);
        duplicate.on_set = self.on_set.clone();
        duplicate.dc_set = self.dc_set.clone();
        duplicate.off_set = self.off_set.clone();

        registry.run_dup(self, &mut duplicate);
        duplicate
    }

    pub fn invalidate(&mut self, registry: &mut DaemonRegistry) {
        self.dc_set = None;
        self.off_set = None;
        registry.run_invalid(self);
        self.is_dup_free = false;
        self.is_min_base = false;
        self.is_scc_minimal = false;
    }

    pub fn network(&self) -> Option<NetworkId> {
        self.network
    }

    fn default_unregistered() -> Self {
        Self {
            name: None,
            short_name: None,
            kind: UtilityNodeKind::Unassigned,
            sis_id: NEXT_SIS_ID.fetch_add(1, Ordering::Relaxed),
            fanin_changed: false,
            fanout_changed: false,
            is_dup_free: false,
            is_min_base: false,
            is_scc_minimal: false,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            fanin_fanout: Vec::new(),
            on_set: None,
            dc_set: None,
            off_set: None,
            copy: None,
            network: None,
            net_handle: None,
            slots: UtilityNodeSlots::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DaemonType {
    Alloc,
    Free,
    Invalid,
    Dup,
}

#[derive(Debug, Eq, PartialEq)]
pub enum NodeUtilError {
    InvalidDaemonType,
}

impl fmt::Display for NodeUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDaemonType => write!(f, "invalid node daemon type"),
        }
    }
}

impl Error for NodeUtilError {}

type AllocDaemon = Box<dyn FnMut(&mut UtilityNode)>;
type FreeDaemon = Box<dyn FnMut(&mut UtilityNode)>;
type InvalidDaemon = Box<dyn FnMut(&mut UtilityNode)>;
type DupDaemon = Box<dyn FnMut(&UtilityNode, &mut UtilityNode)>;

#[derive(Default)]
pub struct DaemonRegistry {
    alloc: Vec<AllocDaemon>,
    free: Vec<FreeDaemon>,
    invalid: Vec<InvalidDaemon>,
    dup: Vec<DupDaemon>,
}

impl DaemonRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_alloc<F>(&mut self, daemon: F)
    where
        F: FnMut(&mut UtilityNode) + 'static,
    {
        self.alloc.push(Box::new(daemon));
    }

    pub fn register_free<F>(&mut self, daemon: F)
    where
        F: FnMut(&mut UtilityNode) + 'static,
    {
        self.free.push(Box::new(daemon));
    }

    pub fn register_invalid<F>(&mut self, daemon: F)
    where
        F: FnMut(&mut UtilityNode) + 'static,
    {
        self.invalid.push(Box::new(daemon));
    }

    pub fn register_dup<F>(&mut self, daemon: F)
    where
        F: FnMut(&UtilityNode, &mut UtilityNode) + 'static,
    {
        self.dup.push(Box::new(daemon));
    }

    pub fn discard_all(&mut self) {
        self.alloc.clear();
        self.free.clear();
        self.invalid.clear();
        self.dup.clear();
    }

    pub fn daemon_count(&self, daemon_type: DaemonType) -> usize {
        match daemon_type {
            DaemonType::Alloc => self.alloc.len(),
            DaemonType::Free => self.free.len(),
            DaemonType::Invalid => self.invalid.len(),
            DaemonType::Dup => self.dup.len(),
        }
    }

    fn run_alloc(&mut self, node: &mut UtilityNode) {
        for daemon in self.alloc.iter_mut().rev() {
            daemon(node);
        }
    }

    fn run_free(&mut self, node: &mut UtilityNode) {
        for daemon in self.free.iter_mut().rev() {
            daemon(node);
        }
    }

    fn run_invalid(&mut self, node: &mut UtilityNode) {
        for daemon in self.invalid.iter_mut().rev() {
            daemon(node);
        }
    }

    fn run_dup(&mut self, old: &UtilityNode, new: &mut UtilityNode) {
        for daemon in self.dup.iter_mut().rev() {
            daemon(old, new);
        }
    }
}

pub fn allocate_node(registry: &mut DaemonRegistry) -> UtilityNode {
    UtilityNode::allocate(registry)
}

pub fn free_node(mut node: UtilityNode, registry: &mut DaemonRegistry) {
    registry.run_free(&mut node);
}

pub fn duplicate_node(
    node: Option<&UtilityNode>,
    registry: &mut DaemonRegistry,
) -> Option<UtilityNode> {
    node.map(|node| node.duplicate(registry))
}

pub fn duplicate_fanins(fanins: &[NodeId]) -> Vec<NodeId> {
    fanins.to_vec()
}

pub fn invalidate_node(node: &mut UtilityNode, registry: &mut DaemonRegistry) {
    node.invalidate(registry);
}

pub fn node_network(node: &UtilityNode) -> Option<NetworkId> {
    node.network()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn allocation_sets_legacy_defaults_and_runs_alloc_daemons_in_registration_stack_order() {
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut registry = DaemonRegistry::new();

        {
            let order = Rc::clone(&order);
            registry.register_alloc(move |node| {
                order.borrow_mut().push(1);
                node.name = Some("first".to_string());
            });
        }
        {
            let order = Rc::clone(&order);
            registry.register_alloc(move |node| {
                order.borrow_mut().push(2);
                node.short_name = node.name.clone();
            });
        }

        let node = allocate_node(&mut registry);

        assert_eq!(node.kind, UtilityNodeKind::Unassigned);
        assert_eq!(node.name, Some("first".to_string()));
        assert_eq!(node.short_name, None);
        assert_eq!(*order.borrow(), vec![2, 1]);
        assert!(node.fanins.is_empty());
        assert!(node.fanouts.is_empty());
        assert!(node.on_set.is_none());
        assert!(node.network.is_none());
    }

    #[test]
    fn duplicate_copies_owned_node_data_but_not_graph_membership() {
        let mut registry = DaemonRegistry::new();
        registry.register_dup(|old, new| {
            new.slots.factored = Some(format!("dup-{}", old.sis_id));
        });

        let mut node = allocate_node(&mut DaemonRegistry::new());
        node.name = Some("n".to_string());
        node.short_name = Some("s".to_string());
        node.kind = UtilityNodeKind::Internal;
        node.is_dup_free = true;
        node.is_min_base = true;
        node.is_scc_minimal = true;
        node.fanins = vec![NodeId(1), NodeId(2)];
        node.fanouts = vec![FanoutRecord {
            fanout: NodeId(9),
            pin: 0,
        }];
        node.fanin_fanout = vec![3, 4];
        node.on_set = Some(Cover::new(vec![vec![LiteralValue::One]]));
        node.dc_set = Some(Cover::new(vec![vec![LiteralValue::DontCare]]));
        node.off_set = Some(Cover::new(vec![vec![LiteralValue::Zero]]));
        node.copy = Some(NodeId(7));
        node.network = Some(NetworkId(5));
        node.net_handle = Some(11);

        let duplicate = duplicate_node(Some(&node), &mut registry).unwrap();

        assert_eq!(duplicate.name, node.name);
        assert_eq!(duplicate.short_name, node.short_name);
        assert_eq!(duplicate.kind, node.kind);
        assert_eq!(duplicate.fanins, node.fanins);
        assert_eq!(duplicate.on_set, node.on_set);
        assert_eq!(duplicate.dc_set, node.dc_set);
        assert_eq!(duplicate.off_set, node.off_set);
        assert_ne!(duplicate.sis_id, node.sis_id);
        assert!(duplicate.fanouts.is_empty());
        assert!(duplicate.fanin_fanout.is_empty());
        assert!(duplicate.copy.is_none());
        assert!(duplicate.network.is_none());
        assert!(duplicate.net_handle.is_none());
        assert_eq!(
            duplicate.slots.factored,
            Some(format!("dup-{}", node.sis_id))
        );
    }

    #[test]
    fn duplicate_none_matches_null_legacy_behavior() {
        let mut registry = DaemonRegistry::new();

        assert!(duplicate_node(None, &mut registry).is_none());
    }

    #[test]
    fn invalidation_drops_cached_sets_runs_daemons_and_clears_minimality_flags() {
        let calls = Rc::new(RefCell::new(0));
        let mut registry = DaemonRegistry::new();
        {
            let calls = Rc::clone(&calls);
            registry.register_invalid(move |node| {
                *calls.borrow_mut() += 1;
                node.slots.simplify = Some("invalidated".to_string());
            });
        }

        let mut node = allocate_node(&mut DaemonRegistry::new());
        node.dc_set = Some(Cover::new(vec![vec![LiteralValue::DontCare]]));
        node.off_set = Some(Cover::new(vec![vec![LiteralValue::Zero]]));
        node.is_dup_free = true;
        node.is_min_base = true;
        node.is_scc_minimal = true;

        invalidate_node(&mut node, &mut registry);

        assert!(node.dc_set.is_none());
        assert!(node.off_set.is_none());
        assert!(!node.is_dup_free);
        assert!(!node.is_min_base);
        assert!(!node.is_scc_minimal);
        assert_eq!(node.slots.simplify, Some("invalidated".to_string()));
        assert_eq!(*calls.borrow(), 1);
    }

    #[test]
    fn free_runs_registered_callbacks_before_node_is_dropped() {
        let freed = Rc::new(RefCell::new(Vec::new()));
        let mut registry = DaemonRegistry::new();
        {
            let freed = Rc::clone(&freed);
            registry.register_free(move |node| {
                freed.borrow_mut().push(node.name.clone());
            });
        }

        let mut node = allocate_node(&mut DaemonRegistry::new());
        node.name = Some("old".to_string());

        free_node(node, &mut registry);

        assert_eq!(*freed.borrow(), vec![Some("old".to_string())]);
    }

    #[test]
    fn discarding_daemons_removes_all_callback_lists() {
        let mut registry = DaemonRegistry::new();
        registry.register_alloc(|_| {});
        registry.register_free(|_| {});
        registry.register_invalid(|_| {});
        registry.register_dup(|_, _| {});

        registry.discard_all();

        assert_eq!(registry.daemon_count(DaemonType::Alloc), 0);
        assert_eq!(registry.daemon_count(DaemonType::Free), 0);
        assert_eq!(registry.daemon_count(DaemonType::Invalid), 0);
        assert_eq!(registry.daemon_count(DaemonType::Dup), 0);
    }

    #[test]
    fn node_network_returns_current_membership() {
        let mut node = allocate_node(&mut DaemonRegistry::new());

        assert_eq!(node_network(&node), None);

        node.network = Some(NetworkId(42));

        assert_eq!(node_network(&node), Some(NetworkId(42)));
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let text = include_str!("nodeutil.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday1", "-")));
    }
}
