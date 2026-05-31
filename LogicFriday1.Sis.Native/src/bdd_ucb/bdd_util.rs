//! Native Rust utilities for the SIS UCB BDD package.
//!
//! The legacy `bdd_util.c` file is mostly public utility surface around BDD
//! variables, external pointers, complemented branch access, size traversal,
//! memory accounting, and memory-limit checks. This module models those
//! behaviors with owned Rust handles and typed errors instead of raw pointers.

use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::mem;

pub const BDD_ONE_ID: BddVariableId = BddVariableId(1 << 30);
pub const BDD_ONE_MEGABYTE: usize = 1_048_576;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddVariableId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddPointer {
    node: BddNodeId,
    complemented: bool,
}

impl BddPointer {
    pub const fn regular(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: false,
        }
    }

    pub const fn complemented(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: true,
        }
    }

    pub const fn node(self) -> BddNodeId {
        self.node
    }

    pub const fn is_complemented(self) -> bool {
        self.complemented
    }

    pub const fn without_complement(self) -> Self {
        Self::regular(self.node)
    }

    pub const fn not(self) -> Self {
        Self {
            node: self.node,
            complemented: !self.complemented,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddHandle(usize);

impl BddHandle {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BddNodeKey {
    variable_id: BddVariableId,
    then_child: BddPointer,
    else_child: BddPointer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNode {
    variable_id: BddVariableId,
    then_child: BddPointer,
    else_child: BddPointer,
}

impl BddNode {
    pub const fn variable_id(&self) -> BddVariableId {
        self.variable_id
    }

    pub const fn then_child(&self) -> BddPointer {
        self.then_child
    }

    pub const fn else_child(&self) -> BddPointer {
        self.else_child
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalBdd {
    node: BddPointer,
    freed: bool,
    origin: &'static str,
}

impl ExternalBdd {
    pub const fn node(&self) -> BddPointer {
        self.node
    }

    pub const fn is_freed(&self) -> bool {
        self.freed
    }

    pub const fn origin(&self) -> &'static str {
        self.origin
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExternalPointerStats {
    pub used: usize,
    pub freed: usize,
    pub total: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddBlockStats {
    pub total: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddCacheStats {
    pub buckets: usize,
    pub entries: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddAdhocCacheStats {
    pub bins: usize,
    pub entries: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddMemoryStats {
    pub last_sbrk: usize,
    pub manager: usize,
    pub nodes: usize,
    pub hashtable: usize,
    pub ext_ptrs: usize,
    pub ite_cache: usize,
    pub ite_const_cache: usize,
    pub adhoc_cache: usize,
    pub total: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddStats {
    pub blocks: BddBlockStats,
    pub extptrs: ExternalPointerStats,
    pub memory: BddMemoryStats,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddExternalHooks {
    pub network_data: Option<String>,
    pub application_data: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddUtilError {
    InvalidBdd,
    FreedBdd(BddHandle),
    UnknownHandle(BddHandle),
    ConstantBdd,
    VariableOutOfRange {
        requested: BddVariableId,
        variables: usize,
    },
    UnknownNode(BddNodeId),
    MissingMemoryDaemon,
}

impl fmt::Display for BddUtilError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBdd => formatter.write_str("invalid BDD"),
            Self::FreedBdd(handle) => {
                write!(formatter, "BDD handle {} has already been freed", handle.0)
            }
            Self::UnknownHandle(handle) => {
                write!(formatter, "unknown BDD handle {}", handle.0)
            }
            Self::ConstantBdd => formatter.write_str("constant BDD"),
            Self::VariableOutOfRange {
                requested,
                variables,
            } => {
                write!(
                    formatter,
                    "BDD variable {} is outside the known range 0..{}",
                    requested.0, variables
                )
            }
            Self::UnknownNode(node) => {
                write!(formatter, "unknown BDD node {}", node.0)
            }
            Self::MissingMemoryDaemon => {
                formatter.write_str("memory limit set, but no daemon registered")
            }
        }
    }
}

impl Error for BddUtilError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryLimitOutcome {
    WithinLimit,
    WouldExceed,
    DaemonCalled,
}

#[derive(Clone, Debug)]
pub struct BddManager {
    variables: usize,
    nodes: Vec<BddNode>,
    unique: HashMap<BddNodeKey, BddNodeId>,
    external: Vec<ExternalBdd>,
    hooks: BddExternalHooks,
    stats: BddStats,
    unique_buckets: usize,
    ite_cache: BddCacheStats,
    const_cache: BddCacheStats,
    adhoc_cache: Option<BddAdhocCacheStats>,
    memory_limit_mb: Option<usize>,
    memory_daemon_registered: bool,
    memory_daemon_calls: usize,
    safeframes_active: bool,
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new(0)
    }
}

impl BddManager {
    pub fn new(variables: usize) -> Self {
        Self {
            variables,
            nodes: vec![BddNode {
                variable_id: BDD_ONE_ID,
                then_child: BddPointer::regular(BddNodeId(0)),
                else_child: BddPointer::complemented(BddNodeId(0)),
            }],
            unique: HashMap::new(),
            external: Vec::new(),
            hooks: BddExternalHooks::default(),
            stats: BddStats::default(),
            unique_buckets: 1,
            ite_cache: BddCacheStats::default(),
            const_cache: BddCacheStats::default(),
            adhoc_cache: None,
            memory_limit_mb: None,
            memory_daemon_registered: false,
            memory_daemon_calls: 0,
            safeframes_active: false,
        }
    }

    pub fn num_vars(&self) -> usize {
        self.variables
    }

    pub fn one(&self) -> BddPointer {
        BddPointer::regular(BddNodeId(0))
    }

    pub fn zero(&self) -> BddPointer {
        self.one().not()
    }

    pub fn is_constant(&self, pointer: BddPointer) -> bool {
        pointer.node == BddNodeId(0)
    }

    pub fn node(&self, node: BddNodeId) -> Option<&BddNode> {
        self.nodes.get(node.0)
    }

    pub fn hooks(&self) -> &BddExternalHooks {
        &self.hooks
    }

    pub fn hooks_mut(&mut self) -> &mut BddExternalHooks {
        &mut self.hooks
    }

    pub fn stats(&self) -> BddStats {
        self.stats
    }

    pub fn external_pointer_stats(&self) -> ExternalPointerStats {
        self.stats.extptrs
    }

    pub fn memory_daemon_calls(&self) -> usize {
        self.memory_daemon_calls
    }

    pub fn safeframes_active(&self) -> bool {
        self.safeframes_active
    }

    pub fn set_memory_limit_mb(&mut self, limit: Option<usize>) {
        self.memory_limit_mb = limit;
    }

    pub fn set_memory_daemon_registered(&mut self, registered: bool) {
        self.memory_daemon_registered = registered;
    }

    pub fn set_safeframes_active(&mut self, active: bool) {
        self.safeframes_active = active;
    }

    pub fn set_memory_shape(
        &mut self,
        unique_buckets: usize,
        ite_cache: BddCacheStats,
        const_cache: BddCacheStats,
        adhoc_cache: Option<BddAdhocCacheStats>,
    ) {
        self.unique_buckets = unique_buckets;
        self.ite_cache = ite_cache;
        self.const_cache = const_cache;
        self.adhoc_cache = adhoc_cache;
    }

    pub fn create_variable(&mut self) -> BddHandle {
        let variable = BddVariableId(self.variables);
        self.variables += 1;

        self.get_variable(variable)
            .expect("newly-created variable is in range")
    }

    pub fn get_variable(&mut self, variable: BddVariableId) -> Result<BddHandle, BddUtilError> {
        if variable.0 >= self.variables {
            return Err(BddUtilError::VariableOutOfRange {
                requested: variable,
                variables: self.variables,
            });
        }

        let node = self.find_or_add(variable, self.one(), self.zero());

        Ok(self.make_external_pointer(node, "bdd_get_variable"))
    }

    pub fn dup(&mut self, handle: BddHandle) -> Result<BddHandle, BddUtilError> {
        let node = self.external_bdd(handle)?.node;

        Ok(self.make_external_pointer(node, "bdd_dup"))
    }

    pub fn free(&mut self, handle: BddHandle) -> Result<(), BddUtilError> {
        let bdd = self
            .external
            .get_mut(handle.0)
            .ok_or(BddUtilError::UnknownHandle(handle))?;

        if bdd.freed {
            return Err(BddUtilError::FreedBdd(handle));
        }

        bdd.freed = true;
        self.stats.extptrs.used -= 1;
        self.stats.extptrs.freed += 1;

        Ok(())
    }

    pub fn external_bdd(&self, handle: BddHandle) -> Result<&ExternalBdd, BddUtilError> {
        let bdd = self
            .external
            .get(handle.0)
            .ok_or(BddUtilError::UnknownHandle(handle))?;

        if bdd.freed {
            return Err(BddUtilError::FreedBdd(handle));
        }

        Ok(bdd)
    }

    pub fn make_external_pointer(&mut self, node: BddPointer, origin: &'static str) -> BddHandle {
        let handle = BddHandle(self.external.len());
        self.external.push(ExternalBdd {
            node,
            freed: false,
            origin,
        });
        self.stats.extptrs.used += 1;
        self.stats.extptrs.total += 1;

        handle
    }

    pub fn then_branch(&mut self, handle: BddHandle) -> Result<BddHandle, BddUtilError> {
        let branch = self.branch_pointer(handle, BranchSide::Then)?;

        Ok(self.make_external_pointer(branch, "bdd_then"))
    }

    pub fn else_branch(&mut self, handle: BddHandle) -> Result<BddHandle, BddUtilError> {
        let branch = self.branch_pointer(handle, BranchSide::Else)?;

        Ok(self.make_external_pointer(branch, "bdd_else"))
    }

    pub fn get_branches(
        &self,
        pointer: BddPointer,
    ) -> Result<(Option<BddPointer>, Option<BddPointer>), BddUtilError> {
        if self.is_constant(pointer) {
            return Ok((None, None));
        }

        let node = self
            .node(pointer.node)
            .ok_or(BddUtilError::UnknownNode(pointer.node))?;

        if pointer.complemented {
            Ok((Some(node.then_child.not()), Some(node.else_child.not())))
        } else {
            Ok((Some(node.then_child), Some(node.else_child)))
        }
    }

    pub fn top_var(&mut self, handle: BddHandle) -> Result<BddHandle, BddUtilError> {
        let pointer = self.external_bdd(handle)?.node;

        if pointer == self.one() {
            return Ok(self.make_external_pointer(self.one(), "bdd_top_var"));
        }

        if pointer == self.zero() {
            return Ok(self.make_external_pointer(self.zero(), "bdd_top_var"));
        }

        let variable = self.top_var_id(handle)?;
        let node = self.find_or_add(variable, self.one(), self.zero());

        Ok(self.make_external_pointer(node, "bdd_top_var"))
    }

    pub fn top_var_id(&self, handle: BddHandle) -> Result<BddVariableId, BddUtilError> {
        let pointer = self.external_bdd(handle)?.node;
        let node = self
            .node(pointer.node)
            .ok_or(BddUtilError::UnknownNode(pointer.node))?;

        Ok(node.variable_id)
    }

    pub fn get_node(&self, handle: BddHandle) -> Result<(BddNodeId, bool), BddUtilError> {
        let pointer = self.external_bdd(handle)?.node;

        Ok((pointer.node, pointer.complemented))
    }

    pub fn size(&self, handle: BddHandle) -> Result<usize, BddUtilError> {
        let pointer = self.external_bdd(handle)?.node;
        let mut visited = HashSet::new();

        self.size_from_pointer(pointer, &mut visited)
    }

    pub fn get_stats(&mut self) -> BddStats {
        self.update_memory_stats();

        self.stats
    }

    pub fn will_exceed_mem_limit(
        &mut self,
        allocation_bytes: usize,
        call_daemon: bool,
    ) -> Result<MemoryLimitOutcome, BddUtilError> {
        let Some(limit_mb) = self.memory_limit_mb else {
            return Ok(MemoryLimitOutcome::WithinLimit);
        };

        self.update_memory_stats();

        let limit_bytes = limit_mb.saturating_mul(BDD_ONE_MEGABYTE);
        if self.stats.memory.total.saturating_add(allocation_bytes) < limit_bytes {
            return Ok(MemoryLimitOutcome::WithinLimit);
        }

        if !call_daemon {
            return Ok(MemoryLimitOutcome::WouldExceed);
        }

        self.safeframes_active = false;
        self.adhoc_cache = None;

        if !self.memory_daemon_registered {
            return Err(BddUtilError::MissingMemoryDaemon);
        }

        self.memory_daemon_calls += 1;

        Ok(MemoryLimitOutcome::DaemonCalled)
    }

    pub fn dynamic_reordering_warning(algorithm_type: &str) -> String {
        format!(
            "WARNING: Dynamic variable reordering not implemented in the Berkeley BDD package. Requested algorithm: {algorithm_type}"
        )
    }

    fn branch_pointer(
        &self,
        handle: BddHandle,
        side: BranchSide,
    ) -> Result<BddPointer, BddUtilError> {
        let pointer = self.external_bdd(handle)?.node;

        if self.is_constant(pointer) {
            return Err(BddUtilError::ConstantBdd);
        }

        let node = self
            .node(pointer.node)
            .ok_or(BddUtilError::UnknownNode(pointer.node))?;
        let branch = match side {
            BranchSide::Then => node.then_child,
            BranchSide::Else => node.else_child,
        };

        if pointer.complemented {
            Ok(branch.not())
        } else {
            Ok(branch)
        }
    }

    fn find_or_add(
        &mut self,
        variable_id: BddVariableId,
        then_child: BddPointer,
        else_child: BddPointer,
    ) -> BddPointer {
        if then_child == else_child {
            return then_child;
        }

        let key = BddNodeKey {
            variable_id,
            then_child,
            else_child,
        };

        if let Some(node) = self.unique.get(&key) {
            return BddPointer::regular(*node);
        }

        let node = BddNodeId(self.nodes.len());
        self.nodes.push(BddNode {
            variable_id,
            then_child,
            else_child,
        });
        self.unique.insert(key, node);
        self.stats.blocks.total = self.nodes.len();

        BddPointer::regular(node)
    }

    fn size_from_pointer(
        &self,
        pointer: BddPointer,
        visited: &mut HashSet<BddNodeId>,
    ) -> Result<usize, BddUtilError> {
        if self.is_constant(pointer) {
            return Ok(0);
        }

        if !visited.insert(pointer.node) {
            return Ok(0);
        }

        let node = self
            .node(pointer.node)
            .ok_or(BddUtilError::UnknownNode(pointer.node))?;

        Ok(1 + self.size_from_pointer(node.then_child, visited)?
            + self.size_from_pointer(node.else_child, visited)?)
    }

    fn update_memory_stats(&mut self) {
        let manager = mem::size_of::<BddManager>();
        let nodes = self.nodes.capacity().max(self.nodes.len()) * mem::size_of::<BddNode>() * 2;
        let hashtable = self.unique_buckets * mem::size_of::<Option<BddNodeId>>();
        let ext_ptrs =
            self.external.capacity().max(self.external.len()) * mem::size_of::<ExternalBdd>();
        let ite_cache = self.ite_cache.buckets * mem::size_of::<Option<usize>>()
            + self.ite_cache.entries * mem::size_of::<usize>() * 4;
        let ite_const_cache = self.const_cache.buckets * mem::size_of::<Option<usize>>()
            + self.const_cache.entries * mem::size_of::<usize>() * 4;
        let adhoc_cache = self.adhoc_cache.map_or(0, |cache| {
            mem::size_of::<HashMap<usize, usize>>()
                + cache.bins * mem::size_of::<Option<usize>>()
                + cache.entries * (mem::size_of::<usize>() * 3)
        });
        let total = manager
            .saturating_add(nodes)
            .saturating_add(hashtable)
            .saturating_add(ext_ptrs)
            .saturating_add(ite_cache)
            .saturating_add(ite_const_cache)
            .saturating_add(adhoc_cache);

        self.stats.memory = BddMemoryStats {
            last_sbrk: total,
            manager,
            nodes,
            hashtable,
            ext_ptrs,
            ite_cache,
            ite_const_cache,
            adhoc_cache,
            total,
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BranchSide {
    Then,
    Else,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_variable_extends_manager_and_returns_variable_bdd() {
        let mut manager = BddManager::default();

        let handle = manager.create_variable();
        let bdd = manager.external_bdd(handle).unwrap();
        let node = manager.node(bdd.node().node()).unwrap();

        assert_eq!(manager.num_vars(), 1);
        assert_eq!(node.variable_id(), BddVariableId(0));
        assert_eq!(node.then_child(), manager.one());
        assert_eq!(node.else_child(), manager.zero());
    }

    #[test]
    fn get_variable_reuses_unique_node_but_allocates_new_external_pointer() {
        let mut manager = BddManager::new(1);

        let first = manager.get_variable(BddVariableId(0)).unwrap();
        let second = manager.get_variable(BddVariableId(0)).unwrap();

        assert_ne!(first, second);
        assert_eq!(
            manager.external_bdd(first).unwrap().node(),
            manager.external_bdd(second).unwrap().node()
        );
        assert_eq!(manager.external_pointer_stats().used, 2);
    }

    #[test]
    fn get_variable_rejects_unknown_variable() {
        let mut manager = BddManager::new(1);

        let error = manager.get_variable(BddVariableId(1)).unwrap_err();

        assert_eq!(
            error,
            BddUtilError::VariableOutOfRange {
                requested: BddVariableId(1),
                variables: 1,
            }
        );
    }

    #[test]
    fn dup_and_free_follow_external_pointer_lifecycle() {
        let mut manager = BddManager::new(1);
        let original = manager.get_variable(BddVariableId(0)).unwrap();
        let duplicate = manager.dup(original).unwrap();

        assert_ne!(original, duplicate);
        assert_eq!(manager.external_pointer_stats().used, 2);

        manager.free(original).unwrap();

        assert_eq!(
            manager.external_bdd(original).unwrap_err(),
            BddUtilError::FreedBdd(original)
        );
        assert_eq!(manager.external_pointer_stats().used, 1);
        assert_eq!(manager.external_pointer_stats().freed, 1);
        assert_eq!(manager.external_bdd(duplicate).unwrap().origin(), "bdd_dup");
    }

    #[test]
    fn branch_access_complements_children_for_complemented_parent() {
        let mut manager = BddManager::new(1);
        let variable = manager.get_variable(BddVariableId(0)).unwrap();
        let variable_pointer = manager.external_bdd(variable).unwrap().node();
        let complemented = manager.make_external_pointer(variable_pointer.not(), "test");

        let then_branch = manager.then_branch(complemented).unwrap();
        let else_branch = manager.else_branch(complemented).unwrap();

        assert_eq!(
            manager.external_bdd(then_branch).unwrap().node(),
            manager.zero()
        );
        assert_eq!(
            manager.external_bdd(else_branch).unwrap().node(),
            manager.one()
        );
    }

    #[test]
    fn branches_of_constant_are_absent() {
        let manager = BddManager::new(0);

        assert_eq!(manager.get_branches(manager.zero()).unwrap(), (None, None));
    }

    #[test]
    fn top_var_returns_constant_for_constant_inputs() {
        let mut manager = BddManager::new(0);
        let zero = manager.make_external_pointer(manager.zero(), "zero");
        let top = manager.top_var(zero).unwrap();

        assert_eq!(manager.external_bdd(top).unwrap().node(), manager.zero());
    }

    #[test]
    fn top_var_id_ignores_pointer_complement() {
        let mut manager = BddManager::new(2);
        let variable = manager.get_variable(BddVariableId(1)).unwrap();
        let complemented = manager
            .make_external_pointer(manager.external_bdd(variable).unwrap().node().not(), "test");

        assert_eq!(manager.top_var_id(complemented).unwrap(), BddVariableId(1));
    }

    #[test]
    fn size_counts_reachable_regular_nodes_once() {
        let mut manager = BddManager::new(2);
        let x = manager.get_variable(BddVariableId(0)).unwrap();
        let y = manager.get_variable(BddVariableId(1)).unwrap();
        let x_node = manager.external_bdd(x).unwrap().node();
        let y_node = manager.external_bdd(y).unwrap().node();
        let root = manager.find_or_add(BddVariableId(0), x_node, y_node);
        let root_handle = manager.make_external_pointer(root, "root");

        assert_eq!(manager.size(root_handle).unwrap(), 3);
    }

    #[test]
    fn get_node_reports_regular_node_and_complement_bit() {
        let mut manager = BddManager::new(1);
        let variable = manager.get_variable(BddVariableId(0)).unwrap();
        let pointer = manager.external_bdd(variable).unwrap().node().not();
        let complemented = manager.make_external_pointer(pointer, "test");

        assert_eq!(
            manager.get_node(complemented).unwrap(),
            (pointer.node(), true)
        );
    }

    #[test]
    fn memory_stats_include_configured_cache_shapes() {
        let mut manager = BddManager::new(1);
        manager.get_variable(BddVariableId(0)).unwrap();
        manager.set_memory_shape(
            17,
            BddCacheStats {
                buckets: 3,
                entries: 5,
            },
            BddCacheStats {
                buckets: 7,
                entries: 11,
            },
            Some(BddAdhocCacheStats {
                bins: 13,
                entries: 19,
            }),
        );

        let stats = manager.get_stats();

        assert!(stats.memory.manager > 0);
        assert!(stats.memory.nodes > 0);
        assert!(stats.memory.hashtable > 0);
        assert!(stats.memory.ite_cache > 0);
        assert!(stats.memory.ite_const_cache > 0);
        assert!(stats.memory.adhoc_cache > 0);
        assert_eq!(
            stats.memory.total,
            stats.memory.manager
                + stats.memory.nodes
                + stats.memory.hashtable
                + stats.memory.ext_ptrs
                + stats.memory.ite_cache
                + stats.memory.ite_const_cache
                + stats.memory.adhoc_cache
        );
    }

    #[test]
    fn memory_limit_without_daemon_only_reports_exceeded_limit() {
        let mut manager = BddManager::new(0);
        manager.set_memory_limit_mb(Some(0));

        let outcome = manager.will_exceed_mem_limit(1, false).unwrap();

        assert_eq!(outcome, MemoryLimitOutcome::WouldExceed);
        assert_eq!(manager.memory_daemon_calls(), 0);
    }

    #[test]
    fn memory_limit_daemon_path_clears_transient_state() {
        let mut manager = BddManager::new(0);
        manager.set_memory_limit_mb(Some(0));
        manager.set_memory_daemon_registered(true);
        manager.set_safeframes_active(true);
        manager.set_memory_shape(
            1,
            BddCacheStats::default(),
            BddCacheStats::default(),
            Some(BddAdhocCacheStats {
                bins: 1,
                entries: 1,
            }),
        );

        let outcome = manager.will_exceed_mem_limit(1, true).unwrap();
        let stats = manager.get_stats();

        assert_eq!(outcome, MemoryLimitOutcome::DaemonCalled);
        assert!(!manager.safeframes_active());
        assert_eq!(manager.memory_daemon_calls(), 1);
        assert_eq!(stats.memory.adhoc_cache, 0);
    }

    #[test]
    fn memory_limit_daemon_path_requires_registered_daemon() {
        let mut manager = BddManager::new(0);
        manager.set_memory_limit_mb(Some(0));
        manager.set_safeframes_active(true);

        let error = manager.will_exceed_mem_limit(1, true).unwrap_err();

        assert_eq!(error, BddUtilError::MissingMemoryDaemon);
        assert!(!manager.safeframes_active());
    }

    #[test]
    fn dynamic_reordering_reports_legacy_warning() {
        let warning = BddManager::dynamic_reordering_warning("sift");

        assert!(warning.contains("Dynamic variable reordering not implemented"));
        assert!(warning.contains("sift"));
    }

    #[test]
    fn source_does_not_contain_legacy_exports_or_tracking_metadata() {
        let source = include_str!("bdd_util.rs");

        for forbidden in [
            concat!("no", "_", "mangle"),
            concat!("extern", " \"", "C", "\""),
            concat!("REQUIRED", "_"),
            concat!("Port", "Dependency"),
            concat!("bead", "_", "id"),
            concat!("source", "_", "file"),
        ] {
            assert!(!source.contains(forbidden), "{forbidden}");
        }
    }
}
