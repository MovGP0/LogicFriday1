//! Native Rust model for `LogicSynthesis/sis/seqbdd/prioqueue.c`.
//!
//! The C file implements a mutable binary min-heap of pointer identities. A
//! side `st_table` maps each pointer to its heap entry so callers can adjust an
//! item after changing data that the comparison callback observes. This module
//! ports that behavior to owned Rust keys: callers keep the mutable priority
//! data outside the queue and use stable, hashable keys in the heap.
//!
//! No legacy C ABI is exposed here. Higher-level SIS integrations that need to
//! bind this queue to original `fn_info_t` pointers remain explicit
//! missing-dependency errors until their native Rust ports are available.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_INTEGRATION_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.436",
        source_file: "LogicSynthesis/sis/seqbdd/prl_product.c",
        reason: "parallel product integration owns fn_info_t priority data observed by the queue comparator",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.440",
        source_file: "LogicSynthesis/sis/seqbdd/product.c",
        reason: "sequential product integration owns fn_info_t priority data observed by the queue comparator",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "legacy C queue entry points used st_table pointer identity maps; native Rust uses HashMap keys instead",
    },
];

pub fn required_integration_dependencies() -> &'static [PortDependency] {
    REQUIRED_INTEGRATION_DEPENDENCIES
}

pub fn is_legacy_sis_pointer_queue_blocked() -> bool {
    true
}

pub fn legacy_sis_pointer_queue_blocked() -> Result<(), PriorityQueueError<()>> {
    missing_integration_dependencies("legacy SIS pointer priority queue facade")
}

fn missing_integration_dependencies<T>(
    operation: &'static str,
) -> Result<T, PriorityQueueError<()>> {
    Err(PriorityQueueError::MissingIntegrationDependencies {
        operation,
        dependencies: REQUIRED_INTEGRATION_DEPENDENCIES,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PriorityQueueError<T> {
    Full {
        capacity: usize,
    },
    DuplicateItem {
        item: T,
    },
    MissingItem {
        item: T,
    },
    PositionMismatch {
        item: T,
        heap_index: usize,
        table_index: Option<usize>,
    },
    OrderingMismatch {
        parent: T,
        child: T,
        parent_index: usize,
        child_index: usize,
    },
    AccountingMismatch {
        heap_len: usize,
        table_len: usize,
    },
    MissingIntegrationDependencies {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl<T: fmt::Debug> fmt::Display for PriorityQueueError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full { capacity } => write!(f, "priority queue is full at capacity {capacity}"),
            Self::DuplicateItem { item } => {
                write!(f, "priority queue already contains item {item:?}")
            }
            Self::MissingItem { item } => {
                write!(f, "priority queue does not contain item {item:?}")
            }
            Self::PositionMismatch {
                item,
                heap_index,
                table_index,
            } => write!(
                f,
                "priority queue position mismatch for {item:?}: heap index {heap_index}, table index {table_index:?}"
            ),
            Self::OrderingMismatch {
                parent,
                child,
                parent_index,
                child_index,
            } => write!(
                f,
                "priority queue ordering mismatch: parent {parent:?} at {parent_index} sorts after child {child:?} at {child_index}"
            ),
            Self::AccountingMismatch {
                heap_len,
                table_len,
            } => write!(
                f,
                "priority queue accounting mismatch: heap has {heap_len} items, table has {table_len}"
            ),
            Self::MissingIntegrationDependencies {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS integration dependencies",
                dependencies.len()
            ),
        }
    }
}

impl<T: fmt::Debug> Error for PriorityQueueError<T> {}

pub struct PriorityQueue<T, C>
where
    T: Clone + Eq + Hash,
    C: Fn(&T, &T) -> Ordering,
{
    heap: Vec<T>,
    positions: HashMap<T, usize>,
    capacity: usize,
    compare: C,
}

impl<T, C> PriorityQueue<T, C>
where
    T: Clone + Eq + Hash,
    C: Fn(&T, &T) -> Ordering,
{
    pub fn new(capacity: usize, compare: C) -> Self {
        Self {
            heap: Vec::with_capacity(capacity),
            positions: HashMap::with_capacity(capacity),
            capacity,
            compare,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn contains(&self, item: &T) -> bool {
        self.positions.contains_key(item)
    }

    pub fn put(&mut self, item: T) -> Result<(), PriorityQueueError<T>> {
        if self.heap.len() == self.capacity {
            return Err(PriorityQueueError::Full {
                capacity: self.capacity,
            });
        }
        if self.positions.contains_key(&item) {
            return Err(PriorityQueueError::DuplicateItem { item });
        }

        let index = self.heap.len();
        self.heap.push(item.clone());
        self.positions.insert(item, index);
        self.sift_up(index);
        Ok(())
    }

    pub fn top(&self) -> Option<&T> {
        self.heap.first()
    }

    pub fn get(&mut self) -> Option<T> {
        if self.heap.is_empty() {
            return None;
        }

        let result = self.heap.swap_remove(0);
        self.positions.remove(&result);
        if let Some(root) = self.heap.first().cloned() {
            self.positions.insert(root, 0);
            self.sift_down(0);
        }
        Some(result)
    }

    pub fn adjust(&mut self, item: &T) -> Result<(), PriorityQueueError<T>> {
        let index = self
            .positions
            .get(item)
            .copied()
            .ok_or_else(|| PriorityQueueError::MissingItem { item: item.clone() })?;
        let index = self.sift_up(index);
        self.sift_down(index);
        Ok(())
    }

    pub fn adjust_up(&mut self, item: &T) -> Result<(), PriorityQueueError<T>> {
        let index = self
            .positions
            .get(item)
            .copied()
            .ok_or_else(|| PriorityQueueError::MissingItem { item: item.clone() })?;
        self.sift_up(index);
        Ok(())
    }

    pub fn adjust_down(&mut self, item: &T) -> Result<(), PriorityQueueError<T>> {
        let index = self
            .positions
            .get(item)
            .copied()
            .ok_or_else(|| PriorityQueueError::MissingItem { item: item.clone() })?;
        self.sift_down(index);
        Ok(())
    }

    pub fn iter_heap_order(&self) -> impl Iterator<Item = &T> {
        self.heap.iter()
    }

    pub fn render_levels<F>(&self, mut print_entry: F) -> String
    where
        F: FnMut(&T) -> String,
    {
        let mut output = String::from("priority queue\n\t");
        let mut stop = 2usize;
        for (index, item) in self.heap.iter().enumerate() {
            let c_index = index + 1;
            if c_index == stop {
                output.push_str("\n\t");
                stop *= 2;
            }
            output.push_str(&print_entry(item));
            output.push(' ');
        }
        output.push('\n');
        output
    }

    pub fn check_queue(&self) -> Result<(), PriorityQueueError<T>> {
        if self.heap.len() != self.positions.len() {
            return Err(PriorityQueueError::AccountingMismatch {
                heap_len: self.heap.len(),
                table_len: self.positions.len(),
            });
        }

        for (index, item) in self.heap.iter().enumerate() {
            let table_index = self.positions.get(item).copied();
            if table_index != Some(index) {
                return Err(PriorityQueueError::PositionMismatch {
                    item: item.clone(),
                    heap_index: index,
                    table_index,
                });
            }
        }

        for child_index in 1..self.heap.len() {
            let parent_index = (child_index - 1) / 2;
            let parent = &self.heap[parent_index];
            let child = &self.heap[child_index];
            if (self.compare)(parent, child).is_gt() {
                return Err(PriorityQueueError::OrderingMismatch {
                    parent: parent.clone(),
                    child: child.clone(),
                    parent_index,
                    child_index,
                });
            }
        }

        Ok(())
    }

    fn sift_up(&mut self, mut index: usize) -> usize {
        while index > 0 {
            let parent = (index - 1) / 2;
            if (self.compare)(&self.heap[index], &self.heap[parent]).is_ge() {
                break;
            }
            self.swap_entries(index, parent);
            index = parent;
        }
        index
    }

    fn sift_down(&mut self, mut index: usize) -> usize {
        loop {
            let left = (index * 2) + 1;
            if left >= self.heap.len() {
                break;
            }

            let right = left + 1;
            let mut next = left;
            if right < self.heap.len()
                && (self.compare)(&self.heap[right], &self.heap[left]).is_lt()
            {
                next = right;
            }

            if (self.compare)(&self.heap[index], &self.heap[next]).is_le() {
                break;
            }
            self.swap_entries(index, next);
            index = next;
        }
        index
    }

    fn swap_entries(&mut self, a: usize, b: usize) {
        self.heap.swap(a, b);
        let a_item = self.heap[a].clone();
        let b_item = self.heap[b].clone();
        self.positions.insert(a_item, a);
        self.positions.insert(b_item, b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    fn by_value(a: &i32, b: &i32) -> Ordering {
        a.cmp(b)
    }

    #[test]
    fn get_pops_items_in_priority_order() {
        let mut queue = PriorityQueue::new(5, by_value);

        queue.put(3).unwrap();
        queue.put(1).unwrap();
        queue.put(4).unwrap();
        queue.put(2).unwrap();

        assert_eq!(queue.top(), Some(&1));
        assert_eq!(queue.len(), 4);
        assert_eq!(queue.get(), Some(1));
        assert_eq!(queue.get(), Some(2));
        assert_eq!(queue.get(), Some(3));
        assert_eq!(queue.get(), Some(4));
        assert_eq!(queue.get(), None);
        assert!(queue.is_empty());
    }

    #[test]
    fn put_rejects_duplicates_and_capacity_overflow() {
        let mut queue = PriorityQueue::new(2, by_value);

        queue.put(2).unwrap();
        assert_eq!(
            queue.put(2),
            Err(PriorityQueueError::DuplicateItem { item: 2 })
        );
        queue.put(1).unwrap();
        assert_eq!(queue.put(0), Err(PriorityQueueError::Full { capacity: 2 }));
    }

    #[test]
    fn adjust_up_reorders_existing_item_after_priority_improves() {
        let priorities = RefCell::new(HashMap::from([("a", 10), ("b", 20), ("c", 30)]));
        let mut queue = PriorityQueue::new(3, |a: &&str, b: &&str| {
            priorities.borrow()[a].cmp(&priorities.borrow()[b])
        });

        queue.put("a").unwrap();
        queue.put("b").unwrap();
        queue.put("c").unwrap();
        priorities.borrow_mut().insert("c", 5);
        queue.adjust_up(&"c").unwrap();

        assert_eq!(queue.top(), Some(&"c"));
        assert_eq!(queue.check_queue(), Ok(()));
    }

    #[test]
    fn adjust_down_reorders_existing_item_after_priority_worsens() {
        let priorities = RefCell::new(HashMap::from([("a", 10), ("b", 20), ("c", 30)]));
        let mut queue = PriorityQueue::new(3, |a: &&str, b: &&str| {
            priorities.borrow()[a].cmp(&priorities.borrow()[b])
        });

        queue.put("a").unwrap();
        queue.put("b").unwrap();
        queue.put("c").unwrap();
        priorities.borrow_mut().insert("a", 40);
        queue.adjust_down(&"a").unwrap();

        assert_eq!(queue.top(), Some(&"b"));
        assert_eq!(queue.get(), Some("b"));
        assert_eq!(queue.get(), Some("c"));
        assert_eq!(queue.get(), Some("a"));
    }

    #[test]
    fn adjust_handles_unknown_direction_priority_change() {
        let priorities = RefCell::new(HashMap::from([(0, 30), (1, 20), (2, 10)]));
        let mut queue = PriorityQueue::new(3, |a: &i32, b: &i32| {
            priorities.borrow()[a].cmp(&priorities.borrow()[b])
        });

        queue.put(0).unwrap();
        queue.put(1).unwrap();
        queue.put(2).unwrap();
        priorities.borrow_mut().insert(0, 5);
        queue.adjust(&0).unwrap();

        assert_eq!(queue.get(), Some(0));
        priorities.borrow_mut().insert(2, 50);
        queue.adjust(&2).unwrap();
        assert_eq!(queue.get(), Some(1));
        assert_eq!(queue.get(), Some(2));
    }

    #[test]
    fn adjust_reports_missing_item() {
        let mut queue = PriorityQueue::new(1, by_value);

        assert_eq!(
            queue.adjust(&4),
            Err(PriorityQueueError::MissingItem { item: 4 })
        );
    }

    #[test]
    fn render_levels_matches_c_shape() {
        let mut queue = PriorityQueue::new(4, by_value);
        queue.put(1).unwrap();
        queue.put(2).unwrap();
        queue.put(3).unwrap();

        assert_eq!(
            queue.render_levels(|item| item.to_string()),
            "priority queue\n\t1 \n\t2 3 \n"
        );
    }

    #[test]
    fn blocked_legacy_facade_reports_dependency_beads_and_sources() {
        let error = legacy_sis_pointer_queue_blocked()
            .expect_err("legacy pointer facade should remain blocked");
        let PriorityQueueError::MissingIntegrationDependencies {
            operation,
            dependencies,
        } = error
        else {
            panic!("unexpected error kind");
        };

        assert_eq!(operation, "legacy SIS pointer priority queue facade");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.436"
                && dependency.source_file == "LogicSynthesis/sis/seqbdd/prl_product.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.440"
                && dependency.source_file == "LogicSynthesis/sis/seqbdd/product.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.485"
                && dependency.source_file == "LogicSynthesis/sis/st/st.c"
        }));
        assert!(is_legacy_sis_pointer_queue_blocked());
        assert_eq!(
            required_integration_dependencies(),
            REQUIRED_INTEGRATION_DEPENDENCIES
        );
    }
}
