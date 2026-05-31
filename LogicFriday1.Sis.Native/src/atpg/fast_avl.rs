//! Native Rust AVL tree corresponding to the SIS `atpg/fast_avl.c` utility.
//!
//! The original C implementation stores opaque key/value pointers, accepts a
//! comparison callback, permits duplicate keys, and returns whether an inserted
//! key already appeared on the search path. This port keeps those behaviors in
//! an owned, generic Rust API.

use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AvlDirection
{
    Forward,
    Backward,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AvlCheckError<K>
{
    BadHeight
    {
        key: K,
        computed: i32,
        stored: i32,
    },
    OutOfBalance
    {
        key: K,
        balance: i32,
    },
    BadLeftOrdering
    {
        parent: K,
        child: K,
    },
    BadRightOrdering
    {
        parent: K,
        child: K,
    },
}

impl<K> fmt::Display for AvlCheckError<K>
where
    K: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::BadHeight { key, computed, stored } => write!(
                f,
                "bad AVL height for {key:?}: computed {computed}, stored {stored}"
            ),
            Self::OutOfBalance { key, balance } => {
                write!(f, "AVL node {key:?} is out of balance by {balance}")
            }
            Self::BadLeftOrdering { parent, child } => {
                write!(f, "AVL left child {child:?} sorts after parent {parent:?}")
            }
            Self::BadRightOrdering { parent, child } => {
                write!(f, "AVL right child {child:?} sorts before parent {parent:?}")
            }
        }
    }
}

impl<K> std::error::Error for AvlCheckError<K> where K: fmt::Debug {}

struct Node<K, V>
{
    key: K,
    value: V,
    height: i32,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
}

impl<K, V> Node<K, V>
{
    fn new(key: K, value: V) -> Self
    {
        Self {
            key,
            value,
            height: 0,
            left: None,
            right: None,
        }
    }
}

pub struct FastAvlTree<K, V, C>
where
    C: Fn(&K, &K) -> Ordering,
{
    root: Option<Box<Node<K, V>>>,
    compare: C,
    len: usize,
    modified: bool,
}

impl<K, V, C> FastAvlTree<K, V, C>
where
    C: Fn(&K, &K) -> Ordering,
{
    pub fn new(compare: C) -> Self
    {
        Self {
            root: None,
            compare,
            len: 0,
            modified: false,
        }
    }

    pub fn len(&self) -> usize
    {
        self.len
    }

    pub fn is_empty(&self) -> bool
    {
        self.len == 0
    }

    pub fn is_modified(&self) -> bool
    {
        self.modified
    }

    pub fn clear_modified(&mut self)
    {
        self.modified = false;
    }

    pub fn get(&self, key: &K) -> Option<&V>
    {
        let mut node = self.root.as_deref();

        while let Some(current) = node
        {
            match (self.compare)(key, &current.key)
            {
                Ordering::Equal => return Some(&current.value),
                Ordering::Less => node = current.left.as_deref(),
                Ordering::Greater => node = current.right.as_deref(),
            }
        }

        None
    }

    pub fn contains_key(&self, key: &K) -> bool
    {
        self.get(key).is_some()
    }

    pub fn insert(&mut self, key: K, value: V) -> bool
    {
        let duplicate = insert_node(&mut self.root, key, value, &self.compare);
        self.len += 1;
        self.modified = true;
        duplicate
    }

    pub fn iter(&self) -> FastAvlIter<'_, K, V>
    {
        FastAvlIter::new(self.root.as_deref(), AvlDirection::Forward)
    }

    pub fn iter_rev(&self) -> FastAvlIter<'_, K, V>
    {
        FastAvlIter::new(self.root.as_deref(), AvlDirection::Backward)
    }

    pub fn iter_direction(&self, direction: AvlDirection) -> FastAvlIter<'_, K, V>
    {
        FastAvlIter::new(self.root.as_deref(), direction)
    }

    pub fn foreach(&self, direction: AvlDirection, mut visit: impl FnMut(&K, &V))
    {
        for (key, value) in self.iter_direction(direction)
        {
            visit(key, value);
        }
    }

    pub fn check_tree(&self) -> Result<(), Vec<AvlCheckError<K>>>
    where
        K: Clone,
    {
        let mut errors = Vec::new();
        check_node(self.root.as_deref(), &self.compare, &mut errors);

        if errors.is_empty()
        {
            Ok(())
        }
        else
        {
            Err(errors)
        }
    }
}

impl<K, V> FastAvlTree<K, V, fn(&K, &K) -> Ordering>
where
    K: Ord,
{
    pub fn ordered() -> Self
    {
        Self::new(K::cmp)
    }
}

impl<K, V, C> fmt::Debug for FastAvlTree<K, V, C>
where
    K: fmt::Debug,
    V: fmt::Debug,
    C: Fn(&K, &K) -> Ordering,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("FastAvlTree")
            .field("len", &self.len)
            .field("modified", &self.modified)
            .field("entries", &self.iter().collect::<Vec<_>>())
            .finish()
    }
}

pub struct FastAvlIter<'a, K, V>
{
    stack: Vec<&'a Node<K, V>>,
    direction: AvlDirection,
}

impl<'a, K, V> FastAvlIter<'a, K, V>
{
    fn new(root: Option<&'a Node<K, V>>, direction: AvlDirection) -> Self
    {
        let mut iter = Self {
            stack: Vec::new(),
            direction,
        };
        iter.push_path(root);
        iter
    }

    fn push_path(&mut self, mut node: Option<&'a Node<K, V>>)
    {
        while let Some(current) = node
        {
            self.stack.push(current);
            node = match self.direction
            {
                AvlDirection::Forward => current.left.as_deref(),
                AvlDirection::Backward => current.right.as_deref(),
            };
        }
    }
}

impl<'a, K, V> Iterator for FastAvlIter<'a, K, V>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item>
    {
        let node = self.stack.pop()?;
        let next_path = match self.direction
        {
            AvlDirection::Forward => node.right.as_deref(),
            AvlDirection::Backward => node.left.as_deref(),
        };
        self.push_path(next_path);
        Some((&node.key, &node.value))
    }
}

fn insert_node<K, V, C>(
    node: &mut Option<Box<Node<K, V>>>,
    key: K,
    value: V,
    compare: &C,
) -> bool
where
    C: Fn(&K, &K) -> Ordering,
{
    let Some(current) = node.as_mut() else
    {
        *node = Some(Box::new(Node::new(key, value)));
        return false;
    };

    let duplicate = match compare(&key, &current.key)
    {
        Ordering::Less => insert_node(&mut current.left, key, value, compare),
        Ordering::Equal => {
            insert_node(&mut current.right, key, value, compare);
            true
        }
        Ordering::Greater => insert_node(&mut current.right, key, value, compare),
    };

    rebalance(node);
    duplicate
}

fn check_node<K, V, C>(
    node: Option<&Node<K, V>>,
    compare: &C,
    errors: &mut Vec<AvlCheckError<K>>,
) -> i32
where
    K: Clone,
    C: Fn(&K, &K) -> Ordering,
{
    let Some(node) = node else
    {
        return -1;
    };

    let right_height = check_node(node.right.as_deref(), compare, errors);
    let left_height = check_node(node.left.as_deref(), compare, errors);
    let computed_height = left_height.max(right_height) + 1;
    let balance = right_height - left_height;

    if computed_height != node.height
    {
        errors.push(AvlCheckError::BadHeight {
            key: node.key.clone(),
            computed: computed_height,
            stored: node.height,
        });
    }

    if !(-1..=1).contains(&balance)
    {
        errors.push(AvlCheckError::OutOfBalance {
            key: node.key.clone(),
            balance,
        });
    }

    if let Some(left) = node.left.as_deref()
    {
        if compare(&left.key, &node.key).is_gt()
        {
            errors.push(AvlCheckError::BadLeftOrdering {
                parent: node.key.clone(),
                child: left.key.clone(),
            });
        }
    }

    if let Some(right) = node.right.as_deref()
    {
        if compare(&node.key, &right.key).is_gt()
        {
            errors.push(AvlCheckError::BadRightOrdering {
                parent: node.key.clone(),
                child: right.key.clone(),
            });
        }
    }

    computed_height
}

fn rebalance<K, V>(node: &mut Option<Box<Node<K, V>>>)
{
    let balance = balance_factor(node.as_deref());

    if balance < -1
    {
        let left_balance = node
            .as_deref()
            .and_then(|current| current.left.as_deref())
            .map(|left| balance_factor(Some(left)))
            .unwrap_or(0);

        if left_balance > 0
        {
            let current = node.as_mut().expect("left-heavy node exists");
            rotate_left(&mut current.left);
        }

        rotate_right(node);
    }
    else if balance > 1
    {
        let right_balance = node
            .as_deref()
            .and_then(|current| current.right.as_deref())
            .map(|right| balance_factor(Some(right)))
            .unwrap_or(0);

        if right_balance < 0
        {
            let current = node.as_mut().expect("right-heavy node exists");
            rotate_right(&mut current.right);
        }

        rotate_left(node);
    }
    else if let Some(current) = node.as_mut()
    {
        update_height(current);
    }
}

fn rotate_left<K, V>(node: &mut Option<Box<Node<K, V>>>)
{
    let Some(mut old_root) = node.take() else
    {
        return;
    };
    let Some(mut new_root) = old_root.right.take() else
    {
        *node = Some(old_root);
        return;
    };

    old_root.right = new_root.left.take();
    update_height(&mut old_root);
    new_root.left = Some(old_root);
    update_height(&mut new_root);
    *node = Some(new_root);
}

fn rotate_right<K, V>(node: &mut Option<Box<Node<K, V>>>)
{
    let Some(mut old_root) = node.take() else
    {
        return;
    };
    let Some(mut new_root) = old_root.left.take() else
    {
        *node = Some(old_root);
        return;
    };

    old_root.left = new_root.right.take();
    update_height(&mut old_root);
    new_root.right = Some(old_root);
    update_height(&mut new_root);
    *node = Some(new_root);
}

fn update_height<K, V>(node: &mut Node<K, V>)
{
    node.height = height(node.left.as_deref()).max(height(node.right.as_deref())) + 1;
}

fn balance_factor<K, V>(node: Option<&Node<K, V>>) -> i32
{
    node.map(|node| height(node.right.as_deref()) - height(node.left.as_deref()))
        .unwrap_or(0)
}

fn height<K, V>(node: Option<&Node<K, V>>) -> i32
{
    node.map(|node| node.height).unwrap_or(-1)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn insert_maintains_sorted_traversal_and_balance()
    {
        let mut tree = FastAvlTree::ordered();

        for key in [30, 10, 20, 40, 50, 25, 5]
        {
            assert!(!tree.insert(key, key * 10));
            tree.check_tree().unwrap();
        }

        let entries = tree.iter().map(|(key, value)| (*key, *value)).collect::<Vec<_>>();
        assert_eq!(
            entries,
            vec![(5, 50), (10, 100), (20, 200), (25, 250), (30, 300), (40, 400), (50, 500)]
        );
    }

    #[test]
    fn duplicate_keys_are_inserted_and_reported()
    {
        let mut tree = FastAvlTree::ordered();

        assert!(!tree.insert(7, "first"));
        assert!(tree.insert(7, "second"));
        assert!(tree.insert(7, "third"));

        assert_eq!(tree.len(), 3);
        assert!(matches!(tree.get(&7), Some(&"first" | &"second" | &"third")));
        assert_eq!(
            tree.iter().map(|(key, value)| (*key, *value)).collect::<Vec<_>>(),
            vec![(7, "first"), (7, "second"), (7, "third")]
        );
        tree.check_tree().unwrap();
    }

    #[test]
    fn backward_iteration_visits_largest_keys_first()
    {
        let mut tree = FastAvlTree::ordered();

        for key in [4, 2, 6, 1, 3, 5, 7]
        {
            tree.insert(key, key);
        }

        assert_eq!(
            tree.iter_rev().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec![7, 6, 5, 4, 3, 2, 1]
        );
        assert_eq!(
            tree.iter_direction(AvlDirection::Backward)
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
            vec![7, 6, 5, 4, 3, 2, 1]
        );
    }

    #[test]
    fn custom_comparator_controls_lookup_and_traversal()
    {
        let mut tree = FastAvlTree::new(|left: &&str, right: &&str| {
            left.len().cmp(&right.len()).then_with(|| left.cmp(right))
        });

        tree.insert("dddd", 4);
        tree.insert("a", 1);
        tree.insert("ccc", 3);
        tree.insert("bb", 2);

        assert_eq!(tree.get(&"ccc"), Some(&3));
        assert_eq!(
            tree.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec!["a", "bb", "ccc", "dddd"]
        );
        tree.check_tree().unwrap();
    }

    #[test]
    fn modified_flag_tracks_insertions()
    {
        let mut tree = FastAvlTree::ordered();

        assert!(!tree.is_modified());
        tree.insert(1, 10);
        assert!(tree.is_modified());
        tree.clear_modified();
        assert!(!tree.is_modified());
    }
}
