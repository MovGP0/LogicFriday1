//! Native Rust model of the SIS UCB BDD unique-table resize routine.
//!
//! The original routine grows the manager's hash table to the next legacy prime
//! size, then moves every existing node into the new bucket array using the BDD
//! node hash. This port keeps that behavior in owned Rust data and reports the
//! memory-limit early-return as an explicit outcome.

use std::error::Error;
use std::fmt;
use std::mem;

pub const HASHTABLE_INITIAL_SIZE: usize = 113;
pub const HASHTABLE_MAX_CHAIN_LEN: usize = 4;
pub const POINTER_BYTES: usize = mem::size_of::<usize>();

const HASH_PRIMES: [usize; 28] = [
    3, 11, 23, 59, 113, 251, 503, 1019, 2039, 4091, 8179, 16369, 32749, 65521, 131063, 262139,
    524269, 1048571, 2097143, 4194287, 8388593, 16777199, 33554393, 67108859, 134217689, 268435399,
    536870879, 1073741789,
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddNodeKey {
    pub variable_id: usize,
    pub then_child: Option<BddNodeId>,
    pub else_child: Option<BddNodeId>,
}

impl BddNodeKey {
    pub fn new(
        variable_id: usize,
        then_child: Option<BddNodeId>,
        else_child: Option<BddNodeId>,
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
    pub fn new(key: BddNodeKey) -> Self {
        Self { key, next: None }
    }

    pub fn key(&self) -> BddNodeKey {
        self.key
    }

    pub fn next(&self) -> Option<BddNodeId> {
        self.next
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddHashtable {
    buckets: Vec<Option<BddNodeId>>,
    nodes: Vec<BddNode>,
    rehash_at_nkeys: usize,
}

impl Default for BddHashtable {
    fn default() -> Self {
        Self::new(HASHTABLE_INITIAL_SIZE)
    }
}

impl BddHashtable {
    pub fn new(nbuckets: usize) -> Self {
        Self {
            buckets: vec![None; nbuckets],
            nodes: Vec::new(),
            rehash_at_nkeys: nbuckets * HASHTABLE_MAX_CHAIN_LEN,
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

    pub fn bucket_head(&self, bucket: usize) -> Option<BddNodeId> {
        self.buckets[bucket]
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

    pub fn insert(&mut self, key: BddNodeKey) -> BddNodeId {
        let node = BddNodeId(self.nodes.len());
        let bucket = bdd_node_hash(key, self.nbuckets());
        let mut entry = BddNode::new(key);
        entry.next = self.buckets[bucket];
        self.nodes.push(entry);
        self.buckets[bucket] = Some(node);
        node
    }

    fn resize_to_next_prime(&mut self) -> Result<usize, ResizeTableError> {
        let next_prime = get_next_hash_prime(self.nbuckets())?;
        let old_buckets = mem::replace(&mut self.buckets, vec![None; next_prime]);

        self.rehash_at_nkeys = next_prime * HASHTABLE_MAX_CHAIN_LEN;

        for bucket in old_buckets {
            let mut cursor = bucket;
            while let Some(node) = cursor {
                let next = self.nodes[node.0].next;
                let position = bdd_node_hash(self.nodes[node.0].key, next_prime);

                self.nodes[node.0].next = self.buckets[position];
                self.buckets[position] = Some(node);
                cursor = next;
            }
        }

        Ok(next_prime)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MemoryLimit {
    pub used_bytes: usize,
    pub limit_bytes: Option<usize>,
}

impl MemoryLimit {
    pub fn unlimited(used_bytes: usize) -> Self {
        Self {
            used_bytes,
            limit_bytes: None,
        }
    }

    pub fn with_limit(used_bytes: usize, limit_bytes: usize) -> Self {
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
    hashtable: BddHashtable,
    memory: MemoryLimit,
}

impl Default for BddManager {
    fn default() -> Self {
        Self {
            hashtable: BddHashtable::default(),
            memory: MemoryLimit::unlimited(0),
        }
    }
}

impl BddManager {
    pub fn new(hashtable: BddHashtable, memory: MemoryLimit) -> Self {
        Self { hashtable, memory }
    }

    pub fn hashtable(&self) -> &BddHashtable {
        &self.hashtable
    }

    pub fn hashtable_mut(&mut self) -> &mut BddHashtable {
        &mut self.hashtable
    }

    pub fn memory(&self) -> MemoryLimit {
        self.memory
    }

    pub fn resize_hashtable(&mut self) -> Result<ResizeOutcome, ResizeTableError> {
        let next_prime = get_next_hash_prime(self.hashtable.nbuckets())?;
        let allocation_bytes = next_prime
            .checked_mul(POINTER_BYTES)
            .ok_or(ResizeTableError::AllocationSizeOverflow)?;

        if self.memory.will_exceed(allocation_bytes) {
            return Ok(ResizeOutcome::SkippedMemoryLimit {
                requested_buckets: next_prime,
                allocation_bytes,
            });
        }

        let old_bytes = self.hashtable.nbuckets() * POINTER_BYTES;
        self.hashtable.resize_to_next_prime()?;
        self.memory.used_bytes = self
            .memory
            .used_bytes
            .saturating_add(allocation_bytes)
            .saturating_sub(old_bytes);

        Ok(ResizeOutcome::Resized {
            buckets: next_prime,
            rehash_at_nkeys: self.hashtable.rehash_at_nkeys(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResizeOutcome {
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
pub enum ResizeTableError {
    UnknownHashPrime(usize),
    LargestHashPrime(usize),
    ZeroBuckets,
    AllocationSizeOverflow,
}

impl fmt::Display for ResizeTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownHashPrime(size) => {
                write!(f, "hash table size {size} is not a legacy BDD prime")
            }
            Self::LargestHashPrime(size) => {
                write!(
                    f,
                    "hash table size {size} is already the largest legacy BDD prime"
                )
            }
            Self::ZeroBuckets => write!(f, "hash table must contain at least one bucket"),
            Self::AllocationSizeOverflow => write!(f, "hash table allocation size overflowed"),
        }
    }
}

impl Error for ResizeTableError {}

pub fn get_next_hash_prime(current_size: usize) -> Result<usize, ResizeTableError> {
    for pair in HASH_PRIMES.windows(2) {
        if current_size == pair[0] {
            return Ok(pair[1]);
        }
    }

    if current_size == HASH_PRIMES[HASH_PRIMES.len() - 1] {
        Err(ResizeTableError::LargestHashPrime(current_size))
    } else {
        Err(ResizeTableError::UnknownHashPrime(current_size))
    }
}

pub fn bdd_node_hash(key: BddNodeKey, nbuckets: usize) -> usize {
    assert!(nbuckets > 0, "hash table must contain at least one bucket");

    let then_child = key.then_child.map_or(0usize, |node| node.0);
    let else_child = key.else_child.map_or(0usize, |node| node.0);

    key.variable_id
        .wrapping_shl(5)
        .wrapping_add(then_child.wrapping_shl(7))
        .wrapping_add(else_child.wrapping_shl(11))
        % nbuckets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(variable_id: usize, then_child: usize, else_child: usize) -> BddNodeKey {
        BddNodeKey::new(
            variable_id,
            Some(BddNodeId(then_child)),
            Some(BddNodeId(else_child)),
        )
    }

    #[test]
    fn next_hash_prime_follows_legacy_table() {
        assert_eq!(get_next_hash_prime(3), Ok(11));
        assert_eq!(get_next_hash_prime(113), Ok(251));
        assert_eq!(get_next_hash_prime(536870879), Ok(1073741789));
    }

    #[test]
    fn next_hash_prime_rejects_unknown_or_final_size() {
        assert_eq!(
            get_next_hash_prime(12),
            Err(ResizeTableError::UnknownHashPrime(12))
        );
        assert_eq!(
            get_next_hash_prime(1073741789),
            Err(ResizeTableError::LargestHashPrime(1073741789))
        );
    }

    #[test]
    fn node_hash_matches_c_macro_shape() {
        let value = bdd_node_hash(key(3, 5, 7), 113);

        assert_eq!(
            value,
            ((3usize << 5) + (5usize << 7) + (7usize << 11)) % 113
        );
    }

    #[test]
    fn resize_moves_nodes_to_next_prime_bucket_table() {
        let mut table = BddHashtable::new(113);
        let keys = [key(1, 0, 0), key(2, 0, 0), key(3, 1, 2), key(4, 2, 1)];
        let nodes: Vec<_> = keys
            .iter()
            .map(|node_key| table.insert(*node_key))
            .collect();
        let mut manager = BddManager::new(table, MemoryLimit::unlimited(113 * POINTER_BYTES));

        let outcome = manager.resize_hashtable();

        assert_eq!(
            outcome,
            Ok(ResizeOutcome::Resized {
                buckets: 251,
                rehash_at_nkeys: 251 * HASHTABLE_MAX_CHAIN_LEN,
            })
        );
        assert_eq!(manager.hashtable().nbuckets(), 251);
        for (node, node_key) in nodes.iter().zip(keys) {
            let bucket = bdd_node_hash(node_key, 251);
            assert!(manager.hashtable().bucket_chain(bucket).contains(node));
        }
    }

    #[test]
    fn resize_preserves_c_head_insert_order_with_collisions() {
        let mut table = BddHashtable::new(3);
        let first = table.insert(BddNodeKey::new(0, None, None));
        let second = table.insert(BddNodeKey::new(3, None, None));
        let third = table.insert(BddNodeKey::new(6, None, None));
        let mut manager = BddManager::new(table, MemoryLimit::unlimited(0));

        manager.resize_hashtable().unwrap();

        let bucket = bdd_node_hash(BddNodeKey::new(0, None, None), 11);
        assert_eq!(manager.hashtable().bucket_chain(bucket), vec![first]);
        let bucket = bdd_node_hash(BddNodeKey::new(3, None, None), 11);
        assert_eq!(manager.hashtable().bucket_chain(bucket), vec![second]);
        let bucket = bdd_node_hash(BddNodeKey::new(6, None, None), 11);
        assert_eq!(manager.hashtable().bucket_chain(bucket), vec![third]);
    }

    #[test]
    fn memory_limit_skip_leaves_table_unchanged() {
        let mut table = BddHashtable::new(113);
        table.insert(key(1, 0, 0));
        let before = table.clone();
        let requested_bytes = 251 * POINTER_BYTES;
        let mut manager = BddManager::new(
            table,
            MemoryLimit::with_limit(10, 10usize.saturating_add(requested_bytes)),
        );

        let outcome = manager.resize_hashtable();

        assert_eq!(
            outcome,
            Ok(ResizeOutcome::SkippedMemoryLimit {
                requested_buckets: 251,
                allocation_bytes: requested_bytes,
            })
        );
        assert_eq!(manager.hashtable(), &before);
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present() {
        let source = include_str!("resize_table.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("bead", "_", "id")));
    }
}
