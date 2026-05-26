//! Native Rust port of `LogicSynthesis/sis/avl/avl.c`.
//!
//! The SIS AVL package stores caller-owned key/value pointers and exposes
//! result-parameter functions. This port keeps the tree behavior in owned Rust
//! form: caller supplied ordering, duplicate-aware insertion, find-or-add,
//! delete, first/last lookup, ordered traversal, and structural validation.

use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AvlDirection {
    Forward,
    Backward,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AvlEntry<K, V> {
    key: K,
    value: V,
}

impl<K, V> AvlEntry<K, V> {
    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn value(&self) -> &V {
        &self.value
    }

    pub fn into_pair(self) -> (K, V) {
        (self.key, self.value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AvlNode<K, V> {
    key: K,
    value: V,
    height: i32,
    left: Option<Box<AvlNode<K, V>>>,
    right: Option<Box<AvlNode<K, V>>>,
}

pub struct AvlTree<K, V, C> {
    root: Option<Box<AvlNode<K, V>>>,
    compare: C,
    len: usize,
    modified: bool,
}

impl<K, V, C> AvlTree<K, V, C>
where
    C: Fn(&K, &K) -> Ordering,
{
    pub fn new(compare: C) -> Self {
        Self {
            root: None,
            compare,
            len: 0,
            modified: false,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn was_modified(&self) -> bool {
        self.modified
    }

    pub fn clear_modified(&mut self) {
        self.modified = false;
    }

    pub fn lookup(&self, key: &K) -> Option<&V> {
        let mut node = self.root.as_deref();
        while let Some(current) = node {
            match (self.compare)(key, &current.key) {
                Ordering::Equal => {
                    return Some(&current.value);
                }
                Ordering::Less => {
                    node = current.left.as_deref();
                }
                Ordering::Greater => {
                    node = current.right.as_deref();
                }
            }
        }

        None
    }

    pub fn lookup_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut node = self.root.as_deref_mut();
        while let Some(current) = node {
            match (self.compare)(key, &current.key) {
                Ordering::Equal => {
                    return Some(&mut current.value);
                }
                Ordering::Less => {
                    node = current.left.as_deref_mut();
                }
                Ordering::Greater => {
                    node = current.right.as_deref_mut();
                }
            }
        }

        None
    }

    pub fn first(&self) -> Option<(&K, &V)> {
        let mut node = self.root.as_deref()?;
        while let Some(left) = node.left.as_deref() {
            node = left;
        }

        Some((&node.key, &node.value))
    }

    pub fn last(&self) -> Option<(&K, &V)> {
        let mut node = self.root.as_deref()?;
        while let Some(right) = node.right.as_deref() {
            node = right;
        }

        Some((&node.key, &node.value))
    }

    pub fn insert(&mut self, key: K, value: V) -> bool {
        let found_existing = Self::insert_node(&mut self.root, key, value, &self.compare);
        self.len += 1;
        self.modified = true;
        found_existing
    }

    pub fn find_or_insert_with(&mut self, key: K, value: impl FnOnce() -> V) -> (bool, &mut V)
    where
        K: Clone,
    {
        let lookup_key = key.clone();
        let mut value = Some(value);
        let found_existing =
            Self::find_or_insert_node(&mut self.root, key, &mut value, &self.compare);
        if !found_existing {
            self.len += 1;
            self.modified = true;
        }

        let value = self
            .lookup_mut(&lookup_key)
            .expect("inserted or matched AVL node must be reachable");

        (found_existing, value)
    }

    pub fn delete(&mut self, key: &K) -> Option<AvlEntry<K, V>> {
        let removed = Self::delete_node(&mut self.root, key, &self.compare)?;
        self.len -= 1;
        self.modified = true;
        Some(removed)
    }

    pub fn iter(&self) -> AvlIter<'_, K, V> {
        AvlIter::new(self.root.as_deref(), AvlDirection::Forward)
    }

    pub fn iter_backward(&self) -> AvlIter<'_, K, V> {
        AvlIter::new(self.root.as_deref(), AvlDirection::Backward)
    }

    pub fn iter_direction(&self, direction: AvlDirection) -> AvlIter<'_, K, V> {
        AvlIter::new(self.root.as_deref(), direction)
    }

    pub fn check_tree(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        Self::check_node(self.root.as_deref(), None, None, &self.compare, &mut errors);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn insert_node(node: &mut Option<Box<AvlNode<K, V>>>, key: K, value: V, compare: &C) -> bool {
        let Some(current) = node.as_mut() else {
            *node = Some(Box::new(AvlNode::new(key, value)));
            return false;
        };

        let found_existing = match compare(&key, &current.key) {
            Ordering::Less => Self::insert_node(&mut current.left, key, value, compare),
            Ordering::Equal => {
                Self::insert_node(&mut current.right, key, value, compare);
                true
            }
            Ordering::Greater => Self::insert_node(&mut current.right, key, value, compare),
        };

        Self::rebalance(node);
        found_existing
    }

    fn find_or_insert_node(
        node: &mut Option<Box<AvlNode<K, V>>>,
        key: K,
        value: &mut Option<impl FnOnce() -> V>,
        compare: &C,
    ) -> bool {
        let Some(current) = node.as_mut() else {
            let value = value
                .take()
                .expect("find-or-insert value factory must be consumed once")(
            );
            *node = Some(Box::new(AvlNode::new(key, value)));
            return false;
        };

        let found_existing = match compare(&key, &current.key) {
            Ordering::Less => Self::find_or_insert_node(&mut current.left, key, value, compare),
            Ordering::Equal => true,
            Ordering::Greater => Self::find_or_insert_node(&mut current.right, key, value, compare),
        };

        if !found_existing {
            Self::rebalance(node);
        }

        found_existing
    }

    fn delete_node(
        node: &mut Option<Box<AvlNode<K, V>>>,
        key: &K,
        compare: &C,
    ) -> Option<AvlEntry<K, V>> {
        let ordering = compare(key, &node.as_ref()?.key);
        let removed = match ordering {
            Ordering::Less => Self::delete_node(&mut node.as_mut()?.left, key, compare),
            Ordering::Greater => Self::delete_node(&mut node.as_mut()?.right, key, compare),
            Ordering::Equal => Some(Self::remove_current(node)),
        };

        if removed.is_some() && node.is_some() {
            Self::rebalance(node);
        }

        removed
    }

    fn remove_current(node: &mut Option<Box<AvlNode<K, V>>>) -> AvlEntry<K, V> {
        let mut old = node.take().expect("current AVL node must exist");
        let removed = AvlEntry {
            key: old.key,
            value: old.value,
        };

        match (old.left.take(), old.right.take()) {
            (None, right) => {
                *node = right;
            }
            (Some(left), None) => {
                *node = Some(left);
            }
            (Some(left), Some(right)) => {
                let mut left = Some(left);
                let mut replacement = Self::remove_rightmost(&mut left);
                replacement.left = left;
                replacement.right = Some(right);
                Self::refresh_height(&mut replacement);
                *node = Some(replacement);
                Self::rebalance(node);
            }
        }

        removed
    }

    fn remove_rightmost(node: &mut Option<Box<AvlNode<K, V>>>) -> Box<AvlNode<K, V>> {
        if node
            .as_ref()
            .expect("rightmost search needs a node")
            .right
            .is_none()
        {
            let mut rightmost = node.take().expect("rightmost node must exist");
            *node = rightmost.left.take();
            return rightmost;
        }

        let rightmost = Self::remove_rightmost(&mut node.as_mut().expect("node must exist").right);
        Self::rebalance(node);
        rightmost
    }

    fn rebalance(node: &mut Option<Box<AvlNode<K, V>>>) {
        let Some(current) = node.as_mut() else {
            return;
        };

        Self::refresh_height(current);
        let balance = Self::height(&current.right) - Self::height(&current.left);

        if balance < -1 {
            if Self::balance(&current.left) > 0 {
                Self::rotate_left(&mut current.left);
            }
            Self::rotate_right(node);
        } else if balance > 1 {
            if Self::balance(&current.right) < 0 {
                Self::rotate_right(&mut current.right);
            }
            Self::rotate_left(node);
        }
    }

    fn rotate_left(node: &mut Option<Box<AvlNode<K, V>>>) {
        let mut old_root = node.take().expect("left rotation needs a root");
        let mut new_root = old_root
            .right
            .take()
            .expect("left rotation needs a right child");
        old_root.right = new_root.left.take();
        Self::refresh_height(&mut old_root);
        new_root.left = Some(old_root);
        Self::refresh_height(&mut new_root);
        *node = Some(new_root);
    }

    fn rotate_right(node: &mut Option<Box<AvlNode<K, V>>>) {
        let mut old_root = node.take().expect("right rotation needs a root");
        let mut new_root = old_root
            .left
            .take()
            .expect("right rotation needs a left child");
        old_root.left = new_root.right.take();
        Self::refresh_height(&mut old_root);
        new_root.right = Some(old_root);
        Self::refresh_height(&mut new_root);
        *node = Some(new_root);
    }

    fn refresh_height(node: &mut AvlNode<K, V>) {
        node.height = Self::height(&node.left).max(Self::height(&node.right)) + 1;
    }

    fn height(node: &Option<Box<AvlNode<K, V>>>) -> i32 {
        node.as_ref().map_or(-1, |node| node.height)
    }

    fn balance(node: &Option<Box<AvlNode<K, V>>>) -> i32 {
        node.as_ref().map_or(0, |node| {
            Self::height(&node.right) - Self::height(&node.left)
        })
    }

    fn check_node(
        node: Option<&AvlNode<K, V>>,
        min: Option<&K>,
        max: Option<&K>,
        compare: &C,
        errors: &mut Vec<String>,
    ) -> i32 {
        let Some(node) = node else {
            return -1;
        };

        if min.is_some_and(|min| compare(&node.key, min) == Ordering::Less) {
            errors.push("node key is less than allowed lower bound".to_string());
        }

        if max.is_some_and(|max| compare(&node.key, max) == Ordering::Greater) {
            errors.push("node key is greater than allowed upper bound".to_string());
        }

        let left_height =
            Self::check_node(node.left.as_deref(), min, Some(&node.key), compare, errors);
        let right_height =
            Self::check_node(node.right.as_deref(), Some(&node.key), max, compare, errors);
        let computed_height = left_height.max(right_height) + 1;
        if computed_height != node.height {
            errors.push(format!(
                "bad height: computed {computed_height}, stored {}",
                node.height
            ));
        }

        let balance = right_height - left_height;
        if !(-1..=1).contains(&balance) {
            errors.push(format!("node out of balance: {balance}"));
        }

        computed_height
    }
}

impl<K, V> AvlNode<K, V> {
    fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            height: 0,
            left: None,
            right: None,
        }
    }
}

pub struct AvlIter<'a, K, V> {
    stack: Vec<&'a AvlNode<K, V>>,
    direction: AvlDirection,
}

impl<'a, K, V> AvlIter<'a, K, V> {
    fn new(root: Option<&'a AvlNode<K, V>>, direction: AvlDirection) -> Self {
        let mut iter = Self {
            stack: Vec::new(),
            direction,
        };
        iter.push_edge(root);
        iter
    }

    fn push_edge(&mut self, mut node: Option<&'a AvlNode<K, V>>) {
        while let Some(current) = node {
            self.stack.push(current);
            node = match self.direction {
                AvlDirection::Forward => current.left.as_deref(),
                AvlDirection::Backward => current.right.as_deref(),
            };
        }
    }
}

impl<'a, K, V> Iterator for AvlIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        match self.direction {
            AvlDirection::Forward => {
                self.push_edge(node.right.as_deref());
            }
            AvlDirection::Backward => {
                self.push_edge(node.left.as_deref());
            }
        }

        Some((&node.key, &node.value))
    }
}

pub fn numcmp(left: &isize, right: &isize) -> Ordering {
    left.cmp(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_lookup_and_boundaries_follow_order() {
        let mut tree = AvlTree::new(|left: &i32, right: &i32| left.cmp(right));

        assert!(!tree.insert(4, "four"));
        assert!(!tree.insert(2, "two"));
        assert!(!tree.insert(6, "six"));
        assert!(!tree.insert(1, "one"));
        assert!(!tree.insert(3, "three"));
        assert!(!tree.insert(5, "five"));
        assert!(!tree.insert(7, "seven"));

        assert_eq!(tree.len(), 7);
        assert_eq!(tree.lookup(&4), Some(&"four"));
        assert_eq!(tree.lookup(&8), None);
        assert_eq!(tree.first(), Some((&1, &"one")));
        assert_eq!(tree.last(), Some((&7, &"seven")));
        assert_eq!(
            tree.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6, 7]
        );
        assert_eq!(
            tree.iter_backward()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
            vec![7, 6, 5, 4, 3, 2, 1]
        );
        assert!(tree.check_tree().is_ok());
    }

    #[test]
    fn insert_reports_duplicates_but_preserves_both_entries() {
        let mut tree = AvlTree::new(|left: &i32, right: &i32| left.cmp(right));

        assert!(!tree.insert(2, "first"));
        assert!(tree.insert(2, "second"));

        assert_eq!(tree.len(), 2);
        assert!(tree.lookup(&2).is_some());
        assert_eq!(
            tree.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec![2, 2]
        );
        assert!(tree.check_tree().is_ok());
    }

    #[test]
    fn find_or_insert_returns_existing_slot_without_adding_duplicate() {
        let mut tree = AvlTree::new(|left: &i32, right: &i32| left.cmp(right));

        let (found, value) = tree.find_or_insert_with(10, || "ten".to_string());
        assert!(!found);
        value.push('!');

        let (found, value) = tree.find_or_insert_with(10, || "duplicate".to_string());
        assert!(found);
        assert_eq!(value, "ten!");
        value.push('?');

        assert_eq!(tree.len(), 1);
        assert_eq!(tree.lookup(&10).map(String::as_str), Some("ten!?"));
        assert!(tree.check_tree().is_ok());
    }

    #[test]
    fn delete_removes_one_entry_and_rebalances() {
        let mut tree = AvlTree::new(|left: &i32, right: &i32| left.cmp(right));
        for key in [8, 4, 12, 2, 6, 10, 14, 1, 3, 5, 7, 9, 11, 13, 15] {
            tree.insert(key, key * 10);
        }

        let removed = tree.delete(&8).map(AvlEntry::into_pair);

        assert_eq!(removed, Some((8, 80)));
        assert_eq!(tree.len(), 14);
        assert_eq!(
            tree.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15]
        );
        assert!(tree.check_tree().is_ok());
    }

    #[test]
    fn rotations_cover_left_right_and_right_left_insertions() {
        let mut left_right = AvlTree::new(|left: &i32, right: &i32| left.cmp(right));
        for key in [3, 1, 2] {
            left_right.insert(key, key);
        }

        let mut right_left = AvlTree::new(|left: &i32, right: &i32| left.cmp(right));
        for key in [1, 3, 2] {
            right_left.insert(key, key);
        }

        assert_eq!(
            left_right.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            right_left.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(left_right.check_tree().is_ok());
        assert!(right_left.check_tree().is_ok());
    }
}
