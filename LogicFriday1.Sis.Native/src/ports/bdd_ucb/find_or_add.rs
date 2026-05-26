//! Native Rust model of the SIS UCB BDD unique-table find-or-add routine.
//!
//! The legacy implementation looks up a node by `(variable, then, else)`,
//! records a hashtable hit when it exists, otherwise grows the bucket table if
//! needed, allocates a node, links it at the bucket head, and records a miss.

use std::error::Error;
use std::fmt;
use std::mem;

pub const BDD_ONE_ID: BddVariableId = BddVariableId(1 << 30);
pub const BDD_BROKEN_HEART_ID: BddVariableId = BddVariableId((1 << 30) - 1);
pub const HASHTABLE_INITIAL_SIZE: usize = 113;
pub const HASHTABLE_MAX_CHAIN_LEN: usize = 4;
pub const POINTER_BYTES: usize = mem::size_of::<usize>();

const HASH_PRIMES: [usize; 28] = [
    3, 11, 23, 59, 113, 251, 503, 1019, 2039, 4091, 8179, 16369, 32749, 65521, 131063, 262139,
    524269, 1048571, 2097143, 4194287, 8388593, 16777199, 33554393, 67108859, 134217689, 268435399,
    536870879, 1073741789,
];

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
    pub const fn positive(node: BddNodeId) -> Self {
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

    pub const fn regular(self) -> Self {
        Self::positive(self.node)
    }

    fn hash_value(self) -> usize {
        (self.node.0 << 1) | usize::from(self.complemented)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddNodeKey {
    pub variable_id: BddVariableId,
    pub then_child: Option<BddPointer>,
    pub else_child: Option<BddPointer>,
}

impl BddNodeKey {
    pub const fn new(
        variable_id: BddVariableId,
        then_child: Option<BddPointer>,
        else_child: Option<BddPointer>,
    ) -> Self {
        Self {
            variable_id,
            then_child,
            else_child,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNode {
    key: BddNodeKey,
    next: Option<BddNodeId>,
}

impl BddNode {
    pub const fn new(key: BddNodeKey) -> Self {
        Self { key, next: None }
    }

    pub const fn key(&self) -> BddNodeKey {
        self.key
    }

    pub const fn next(&self) -> Option<BddNodeId> {
        self.next
    }

    pub const fn is_broken_heart(&self) -> bool {
        self.key.variable_id.0 == BDD_BROKEN_HEART_ID.0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HashtableStats {
    pub hits: usize,
    pub misses: usize,
}

impl HashtableStats {
    pub const fn lookups(self) -> usize {
        self.hits + self.misses
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddMemoryLimit {
    pub used_bytes: usize,
    pub limit_bytes: Option<usize>,
}

impl BddMemoryLimit {
    pub const fn unlimited(used_bytes: usize) -> Self {
        Self {
            used_bytes,
            limit_bytes: None,
        }
    }

    pub const fn with_limit(used_bytes: usize, limit_bytes: usize) -> Self {
        Self {
            used_bytes,
            limit_bytes: Some(limit_bytes),
        }
    }

    pub fn will_exceed(self, allocation_bytes: usize) -> bool {
        self.limit_bytes
            .is_some_and(|limit| self.used_bytes.saturating_add(allocation_bytes) >= limit)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddManager {
    buckets: Vec<Option<BddNodeId>>,
    nodes: Vec<BddNode>,
    rehash_at_nkeys: usize,
    stats: HashtableStats,
    memory: BddMemoryLimit,
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new(HASHTABLE_INITIAL_SIZE, BddMemoryLimit::unlimited(0))
    }
}

impl BddManager {
    pub fn new(nbuckets: usize, memory: BddMemoryLimit) -> Self {
        assert!(nbuckets > 0, "hash table must contain at least one bucket");

        Self {
            buckets: vec![None; nbuckets],
            nodes: Vec::new(),
            rehash_at_nkeys: nbuckets * HASHTABLE_MAX_CHAIN_LEN,
            stats: HashtableStats::default(),
            memory,
        }
    }

    pub fn nbuckets(&self) -> usize {
        self.buckets.len()
    }

    pub fn nkeys(&self) -> usize {
        self.nodes.len()
    }

    pub fn rehash_at_nkeys(&self) -> usize {
        self.rehash_at_nkeys
    }

    pub fn stats(&self) -> HashtableStats {
        self.stats
    }

    pub fn memory(&self) -> BddMemoryLimit {
        self.memory
    }

    pub fn node(&self, node: BddNodeId) -> Option<&BddNode> {
        self.nodes.get(node.0)
    }

    pub fn bucket_chain(&self, bucket: usize) -> Vec<BddNodeId> {
        let mut chain = Vec::new();
        let mut cursor = self.buckets[bucket];

        while let Some(node) = cursor {
            chain.push(node);
            cursor = self.nodes[node.0].next;
        }

        chain
    }

    pub fn mark_broken_heart(&mut self, node: BddNodeId) -> Result<(), FindOrAddError> {
        let entry = self
            .nodes
            .get_mut(node.0)
            .ok_or(FindOrAddError::UnknownChild(node))?;

        entry.key.variable_id = BDD_BROKEN_HEART_ID;
        Ok(())
    }

    pub fn find_or_add(
        &mut self,
        variable_id: BddVariableId,
        then_child: Option<BddPointer>,
        else_child: Option<BddPointer>,
    ) -> Result<FindOrAddOutcome, FindOrAddError> {
        let key = BddNodeKey::new(variable_id, then_child, else_child);
        self.validate_key(key)?;

        if let Some(node) = self.find_node(key) {
            self.stats.hits += 1;
            return Ok(FindOrAddOutcome {
                node,
                created: false,
                resize: ResizeOutcome::NotNeeded,
            });
        }

        let resize = if self.nkeys() + 1 >= self.rehash_at_nkeys {
            self.resize_hashtable()?
        } else {
            ResizeOutcome::NotNeeded
        };

        let node = self.insert_node(key);
        self.stats.misses += 1;

        Ok(FindOrAddOutcome {
            node,
            created: true,
            resize,
        })
    }

    fn validate_key(&self, key: BddNodeKey) -> Result<(), FindOrAddError> {
        if key.variable_id != BDD_ONE_ID {
            let then_child = key.then_child.ok_or(FindOrAddError::MissingThenChild)?;
            let else_child = key.else_child.ok_or(FindOrAddError::MissingElseChild)?;

            if then_child.is_complemented() {
                return Err(FindOrAddError::ComplementedThenChild(then_child));
            }

            self.validate_child(then_child)?;
            self.validate_child(else_child)?;
        } else {
            if let Some(then_child) = key.then_child {
                if then_child.is_complemented() {
                    return Err(FindOrAddError::ComplementedThenChild(then_child));
                }

                self.validate_child(then_child)?;
            }

            if let Some(else_child) = key.else_child {
                self.validate_child(else_child)?;
            }
        }

        Ok(())
    }

    fn validate_child(&self, pointer: BddPointer) -> Result<(), FindOrAddError> {
        let node = self
            .nodes
            .get(pointer.node().0)
            .ok_or(FindOrAddError::UnknownChild(pointer.node()))?;

        if node.is_broken_heart() {
            return Err(FindOrAddError::BrokenHeartChild(pointer.node()));
        }

        Ok(())
    }

    fn find_node(&self, key: BddNodeKey) -> Option<BddNodeId> {
        let bucket = bdd_raw_node_hash(key, self.nbuckets());
        let mut cursor = self.buckets[bucket];

        while let Some(node) = cursor {
            let entry = &self.nodes[node.0];
            if entry.key == key {
                return Some(node);
            }

            cursor = entry.next;
        }

        None
    }

    fn insert_node(&mut self, key: BddNodeKey) -> BddNodeId {
        let node = BddNodeId(self.nodes.len());
        let bucket = bdd_raw_node_hash(key, self.nbuckets());
        let mut entry = BddNode::new(key);

        entry.next = self.buckets[bucket];
        self.nodes.push(entry);
        self.buckets[bucket] = Some(node);

        node
    }

    fn resize_hashtable(&mut self) -> Result<ResizeOutcome, FindOrAddError> {
        let next_size = next_hash_prime(self.nbuckets())?;
        let allocation_bytes = next_size
            .checked_mul(POINTER_BYTES)
            .ok_or(FindOrAddError::AllocationSizeOverflow)?;

        if self.memory.will_exceed(allocation_bytes) {
            return Ok(ResizeOutcome::SkippedMemoryLimit {
                requested_buckets: next_size,
                allocation_bytes,
            });
        }

        let old_size = self.nbuckets();
        let old_buckets = mem::replace(&mut self.buckets, vec![None; next_size]);
        self.rehash_at_nkeys = next_size * HASHTABLE_MAX_CHAIN_LEN;

        for bucket in old_buckets {
            let mut cursor = bucket;
            while let Some(node) = cursor {
                let next = self.nodes[node.0].next;
                let position = bdd_raw_node_hash(self.nodes[node.0].key, next_size);

                self.nodes[node.0].next = self.buckets[position];
                self.buckets[position] = Some(node);
                cursor = next;
            }
        }

        self.memory.used_bytes = self
            .memory
            .used_bytes
            .saturating_add(allocation_bytes)
            .saturating_sub(old_size * POINTER_BYTES);

        Ok(ResizeOutcome::Resized {
            buckets: next_size,
            rehash_at_nkeys: self.rehash_at_nkeys,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FindOrAddOutcome {
    pub node: BddNodeId,
    pub created: bool,
    pub resize: ResizeOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResizeOutcome {
    NotNeeded,
    Resized {
        buckets: usize,
        rehash_at_nkeys: usize,
    },
    SkippedMemoryLimit {
        requested_buckets: usize,
        allocation_bytes: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindOrAddError {
    MissingThenChild,
    MissingElseChild,
    ComplementedThenChild(BddPointer),
    UnknownChild(BddNodeId),
    BrokenHeartChild(BddNodeId),
    UnknownHashPrime(usize),
    LargestHashPrime(usize),
    AllocationSizeOverflow,
}

impl fmt::Display for FindOrAddError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingThenChild => write!(f, "non-constant BDD nodes require a then child"),
            Self::MissingElseChild => write!(f, "non-constant BDD nodes require an else child"),
            Self::ComplementedThenChild(pointer) => {
                write!(f, "then child {:?} must be a regular BDD pointer", pointer)
            }
            Self::UnknownChild(node) => write!(f, "BDD child node {:?} was not found", node),
            Self::BrokenHeartChild(node) => {
                write!(
                    f,
                    "BDD child node {:?} is a garbage-collection forwarding node",
                    node
                )
            }
            Self::UnknownHashPrime(size) => {
                write!(f, "hash table size {size} is not a legacy BDD prime")
            }
            Self::LargestHashPrime(size) => {
                write!(
                    f,
                    "hash table size {size} is already the largest legacy BDD prime"
                )
            }
            Self::AllocationSizeOverflow => write!(f, "hash table allocation size overflowed"),
        }
    }
}

impl Error for FindOrAddError {}

pub fn bdd_raw_node_hash(key: BddNodeKey, nbuckets: usize) -> usize {
    assert!(nbuckets > 0, "hash table must contain at least one bucket");

    let then_child = key.then_child.map_or(0usize, BddPointer::hash_value);
    let else_child = key.else_child.map_or(0usize, BddPointer::hash_value);

    bdd_generic_hash(key.variable_id.0, then_child, else_child, nbuckets)
}

pub fn bdd_generic_hash(a: usize, b: usize, c: usize, nbuckets: usize) -> usize {
    assert!(nbuckets > 0, "hash table must contain at least one bucket");

    a.wrapping_shl(5)
        .wrapping_add(b.wrapping_shl(7))
        .wrapping_add(c.wrapping_shl(11))
        % nbuckets
}

pub fn next_hash_prime(current_size: usize) -> Result<usize, FindOrAddError> {
    for pair in HASH_PRIMES.windows(2) {
        if current_size == pair[0] {
            return Ok(pair[1]);
        }
    }

    if current_size == HASH_PRIMES[HASH_PRIMES.len() - 1] {
        Err(FindOrAddError::LargestHashPrime(current_size))
    } else {
        Err(FindOrAddError::UnknownHashPrime(current_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positive(node: usize) -> BddPointer {
        BddPointer::positive(BddNodeId(node))
    }

    fn negative(node: usize) -> BddPointer {
        BddPointer::complemented(BddNodeId(node))
    }

    fn key(variable_id: usize, then_child: usize, else_child: BddPointer) -> BddNodeKey {
        BddNodeKey::new(
            BddVariableId(variable_id),
            Some(positive(then_child)),
            Some(else_child),
        )
    }

    fn manager_with_constants() -> BddManager {
        let mut manager = BddManager::new(113, BddMemoryLimit::unlimited(113 * POINTER_BYTES));

        manager.find_or_add(BDD_ONE_ID, None, None).unwrap();
        manager
    }

    #[test]
    fn generic_hash_matches_legacy_shift_formula() {
        assert_eq!(
            bdd_generic_hash(3, 5, 7, 113),
            ((3usize << 5) + (5usize << 7) + (7usize << 11)) % 113
        );
    }

    #[test]
    fn constant_node_allows_nil_children() {
        let mut manager = BddManager::default();

        let outcome = manager.find_or_add(BDD_ONE_ID, None, None).unwrap();

        assert_eq!(outcome.node, BddNodeId(0));
        assert!(outcome.created);
        assert_eq!(manager.nkeys(), 1);
        assert_eq!(
            manager.node(outcome.node).unwrap().key(),
            BddNodeKey::new(BDD_ONE_ID, None, None)
        );
    }

    #[test]
    fn existing_node_is_returned_and_counts_as_hit() {
        let mut manager = manager_with_constants();
        let one = positive(0);
        let zero = negative(0);

        let first = manager
            .find_or_add(BddVariableId(2), Some(one), Some(zero))
            .unwrap();
        let second = manager
            .find_or_add(BddVariableId(2), Some(one), Some(zero))
            .unwrap();

        assert_eq!(second.node, first.node);
        assert!(!second.created);
        assert_eq!(manager.nkeys(), 2);
        assert_eq!(manager.stats(), HashtableStats { hits: 1, misses: 2 });
    }

    #[test]
    fn different_else_phase_creates_distinct_unique_table_entry() {
        let mut manager = manager_with_constants();
        let one = positive(0);
        let zero = negative(0);

        let positive_else = manager
            .find_or_add(BddVariableId(3), Some(one), Some(one))
            .unwrap();
        let negative_else = manager
            .find_or_add(BddVariableId(3), Some(one), Some(zero))
            .unwrap();

        assert_ne!(positive_else.node, negative_else.node);
        assert_eq!(manager.nkeys(), 3);
    }

    #[test]
    fn new_node_is_linked_at_bucket_head() {
        let mut manager = manager_with_constants();
        let first_key = key(1, 0, negative(0));
        let second_key = key(1 + 113, 0, negative(0));
        let bucket = bdd_raw_node_hash(first_key, manager.nbuckets());

        let first = manager
            .find_or_add(
                first_key.variable_id,
                first_key.then_child,
                first_key.else_child,
            )
            .unwrap();
        let second = manager
            .find_or_add(
                second_key.variable_id,
                second_key.then_child,
                second_key.else_child,
            )
            .unwrap();

        assert_eq!(bdd_raw_node_hash(second_key, manager.nbuckets()), bucket);
        assert_eq!(manager.bucket_chain(bucket)[0], second.node);
        assert!(manager.bucket_chain(bucket).contains(&first.node));
    }

    #[test]
    fn resize_happens_before_insert_when_threshold_is_reached() {
        let mut manager = BddManager::new(3, BddMemoryLimit::unlimited(3 * POINTER_BYTES));
        manager.find_or_add(BDD_ONE_ID, None, None).unwrap();
        let one = positive(0);
        let zero = negative(0);

        for variable_id in 1..11 {
            manager
                .find_or_add(BddVariableId(variable_id), Some(one), Some(zero))
                .unwrap();
        }

        let outcome = manager
            .find_or_add(BddVariableId(11), Some(one), Some(zero))
            .unwrap();

        assert_eq!(
            outcome.resize,
            ResizeOutcome::Resized {
                buckets: 11,
                rehash_at_nkeys: 44,
            }
        );
        assert_eq!(manager.nbuckets(), 11);
        assert_eq!(manager.nkeys(), 12);
        for node in 0..manager.nkeys() {
            let node_id = BddNodeId(node);
            let node_key = manager.node(node_id).unwrap().key();
            let bucket = bdd_raw_node_hash(node_key, manager.nbuckets());
            assert!(manager.bucket_chain(bucket).contains(&node_id));
        }
    }

    #[test]
    fn memory_limit_skip_keeps_existing_table_size_and_still_inserts() {
        let requested_bytes = 11 * POINTER_BYTES;
        let mut manager = BddManager::new(
            3,
            BddMemoryLimit::with_limit(3 * POINTER_BYTES, 3 * POINTER_BYTES + requested_bytes),
        );
        manager.find_or_add(BDD_ONE_ID, None, None).unwrap();
        let one = positive(0);
        let zero = negative(0);

        for variable_id in 1..11 {
            manager
                .find_or_add(BddVariableId(variable_id), Some(one), Some(zero))
                .unwrap();
        }

        let outcome = manager
            .find_or_add(BddVariableId(11), Some(one), Some(zero))
            .unwrap();

        assert_eq!(
            outcome.resize,
            ResizeOutcome::SkippedMemoryLimit {
                requested_buckets: 11,
                allocation_bytes: requested_bytes,
            }
        );
        assert_eq!(manager.nbuckets(), 3);
        assert_eq!(manager.nkeys(), 12);
    }

    #[test]
    fn non_constant_nodes_require_regular_then_and_existing_children() {
        let mut manager = manager_with_constants();

        assert_eq!(
            manager.find_or_add(BddVariableId(1), None, Some(negative(0))),
            Err(FindOrAddError::MissingThenChild)
        );
        assert_eq!(
            manager.find_or_add(BddVariableId(1), Some(positive(0)), None),
            Err(FindOrAddError::MissingElseChild)
        );
        assert_eq!(
            manager.find_or_add(BddVariableId(1), Some(negative(0)), Some(positive(0))),
            Err(FindOrAddError::ComplementedThenChild(negative(0)))
        );
        assert_eq!(
            manager.find_or_add(BddVariableId(1), Some(positive(99)), Some(positive(0))),
            Err(FindOrAddError::UnknownChild(BddNodeId(99)))
        );
    }

    #[test]
    fn broken_heart_children_are_rejected() {
        let mut manager = manager_with_constants();
        manager.mark_broken_heart(BddNodeId(0)).unwrap();

        assert_eq!(
            manager.find_or_add(BddVariableId(1), Some(positive(0)), Some(negative(0))),
            Err(FindOrAddError::BrokenHeartChild(BddNodeId(0)))
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present() {
        let source = include_str!("find_or_add.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
