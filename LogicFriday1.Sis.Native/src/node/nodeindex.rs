//! Native node-to-index table for SIS node algorithms.
//!
//! The legacy implementation stored `node_t *` keys in an `st_table` and kept
//! the reverse lookup in an array. This Rust version keeps the same stable
//! insertion-order numbering while exposing an idiomatic generic handle table.

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIndex<N>
where
    N: Copy + Eq + Hash,
{
    node_to_index: HashMap<N, usize>,
    index_to_node: Vec<N>,
}

impl<N> NodeIndex<N>
where
    N: Copy + Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            node_to_index: HashMap::new(),
            index_to_node: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            node_to_index: HashMap::with_capacity(capacity),
            index_to_node: Vec::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, node: N) -> usize {
        if let Some(index) = self.node_to_index.get(&node).copied() {
            return index;
        }

        let index = self.index_to_node.len();
        self.node_to_index.insert(node, index);
        self.index_to_node.push(node);
        index
    }

    pub fn index_of(&self, node: N) -> Option<usize> {
        self.node_to_index.get(&node).copied()
    }

    pub fn node_of(&self, index: usize) -> Option<N> {
        self.index_to_node.get(index).copied()
    }

    pub fn len(&self) -> usize {
        self.index_to_node.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index_to_node.is_empty()
    }

    pub fn clear(&mut self) {
        self.node_to_index.clear();
        self.index_to_node.clear();
    }

    pub fn nodes(&self) -> &[N] {
        &self.index_to_node
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, N)> + '_ {
        self.index_to_node.iter().copied().enumerate()
    }
}

impl<N> Default for NodeIndex<N>
where
    N: Copy + Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::NodeIndex;

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    struct TestNode(usize);

    #[test]
    fn insert_assigns_stable_sequential_indices() {
        let mut table = NodeIndex::new();

        assert_eq!(table.insert(TestNode(10)), 0);
        assert_eq!(table.insert(TestNode(20)), 1);
        assert_eq!(table.insert(TestNode(30)), 2);

        assert_eq!(table.len(), 3);
        assert_eq!(table.nodes(), &[TestNode(10), TestNode(20), TestNode(30)]);
    }

    #[test]
    fn duplicate_insert_returns_existing_index() {
        let mut table = NodeIndex::new();

        assert_eq!(table.insert(TestNode(10)), 0);
        assert_eq!(table.insert(TestNode(20)), 1);
        assert_eq!(table.insert(TestNode(10)), 0);

        assert_eq!(table.len(), 2);
        assert_eq!(table.nodes(), &[TestNode(10), TestNode(20)]);
    }

    #[test]
    fn lookups_report_missing_nodes_and_indices() {
        let mut table = NodeIndex::new();
        table.insert(TestNode(10));

        assert_eq!(table.index_of(TestNode(10)), Some(0));
        assert_eq!(table.index_of(TestNode(99)), None);
        assert_eq!(table.node_of(0), Some(TestNode(10)));
        assert_eq!(table.node_of(1), None);
    }

    #[test]
    fn clear_resets_both_lookup_directions() {
        let mut table = NodeIndex::with_capacity(4);
        table.insert(TestNode(10));
        table.insert(TestNode(20));

        table.clear();

        assert!(table.is_empty());
        assert_eq!(table.index_of(TestNode(10)), None);
        assert_eq!(table.node_of(0), None);
        assert_eq!(table.insert(TestNode(30)), 0);
    }

    #[test]
    fn iter_returns_indices_with_nodes_in_insertion_order() {
        let mut table = NodeIndex::new();
        table.insert(TestNode(10));
        table.insert(TestNode(20));

        let entries: Vec<_> = table.iter().collect();

        assert_eq!(entries, vec![(0, TestNode(10)), (1, TestNode(20))]);
    }
}
