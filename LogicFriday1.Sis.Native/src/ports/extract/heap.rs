//! Native max-heap support for extraction routines.
//!
//! The heap stores owned entries keyed by signed integer priority. Equal keys
//! keep the same comparison behavior as the SIS extraction heap: insertion only
//! bubbles through strictly smaller parents, and deletion prefers the left child
//! when children have equal keys.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeapEntry<T> {
    key: i32,
    item: T,
}

impl<T> HeapEntry<T> {
    pub fn new(key: i32, item: T) -> Self {
        Self { key, item }
    }

    pub fn key(&self) -> i32 {
        self.key
    }

    pub fn item(&self) -> &T {
        &self.item
    }

    pub fn into_item(self) -> T {
        self.item
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExtractHeap<T> {
    entries: Vec<HeapEntry<T>>,
}

impl<T> ExtractHeap<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn find_max(&self) -> Option<&HeapEntry<T>> {
        self.entries.first()
    }

    pub fn insert(&mut self, entry: HeapEntry<T>) {
        self.entries.push(entry);
        self.sift_up(self.entries.len() - 1);
    }

    pub fn insert_item(&mut self, key: i32, item: T) {
        self.insert(HeapEntry::new(key, item));
    }

    pub fn delete_max(&mut self) -> Option<HeapEntry<T>> {
        match self.entries.len() {
            0 => None,
            1 => self.entries.pop(),
            _ => {
                let result = self.entries.swap_remove(0);
                self.sift_down(0);
                Some(result)
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn iter_heap_order(&self) -> impl Iterator<Item = &HeapEntry<T>> {
        self.entries.iter()
    }

    fn sift_up(&mut self, mut current: usize) {
        while current > 0 {
            let parent = (current - 1) / 2;
            if self.entries[parent].key >= self.entries[current].key {
                break;
            }

            self.entries.swap(parent, current);
            current = parent;
        }
    }

    fn sift_down(&mut self, mut current: usize) {
        loop {
            let left = current * 2 + 1;
            if left >= self.entries.len() {
                break;
            }

            let right = left + 1;
            let child =
                if right < self.entries.len() && self.entries[right].key > self.entries[left].key {
                    right
                } else {
                    left
                };

            if self.entries[current].key >= self.entries[child].key {
                break;
            }

            self.entries.swap(current, child);
            current = child;
        }
    }
}

impl<T> Extend<HeapEntry<T>> for ExtractHeap<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = HeapEntry<T>>,
    {
        for entry in iter {
            self.insert(entry);
        }
    }
}

impl<T> FromIterator<HeapEntry<T>> for ExtractHeap<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = HeapEntry<T>>,
    {
        let mut heap = Self::new();
        heap.extend(iter);
        heap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: i32, item: &'static str) -> HeapEntry<&'static str> {
        HeapEntry::new(key, item)
    }

    #[test]
    fn new_heap_is_empty() {
        let heap = ExtractHeap::<&str>::new();

        assert_eq!(heap.len(), 0);
        assert!(heap.is_empty());
        assert_eq!(heap.find_max(), None);
        assert_eq!(heap.iter_heap_order().count(), 0);
    }

    #[test]
    fn find_max_returns_largest_key_without_removing_it() {
        let mut heap = ExtractHeap::new();
        heap.insert(entry(3, "three"));
        heap.insert(entry(7, "seven"));
        heap.insert(entry(5, "five"));

        assert_eq!(heap.find_max(), Some(&entry(7, "seven")));
        assert_eq!(heap.len(), 3);
    }

    #[test]
    fn delete_max_returns_entries_by_descending_key() {
        let mut heap = ExtractHeap::new();
        for (key, item) in [(4, "four"), (1, "one"), (9, "nine"), (6, "six")] {
            heap.insert_item(key, item);
        }

        assert_eq!(heap.delete_max(), Some(entry(9, "nine")));
        assert_eq!(heap.delete_max(), Some(entry(6, "six")));
        assert_eq!(heap.delete_max(), Some(entry(4, "four")));
        assert_eq!(heap.delete_max(), Some(entry(1, "one")));
        assert_eq!(heap.delete_max(), None);
    }

    #[test]
    fn equal_keys_do_not_bubble_past_existing_parents() {
        let mut heap = ExtractHeap::new();
        heap.insert(entry(10, "first"));
        heap.insert(entry(10, "second"));

        assert_eq!(heap.find_max().map(HeapEntry::item), Some(&"first"));
    }

    #[test]
    fn delete_max_prefers_left_child_when_child_keys_are_equal() {
        let mut heap = ExtractHeap::new();
        heap.insert(entry(100, "root"));
        heap.insert(entry(50, "left"));
        heap.insert(entry(50, "right"));
        heap.insert(entry(1, "tail"));

        assert_eq!(heap.delete_max(), Some(entry(100, "root")));
        assert_eq!(heap.find_max().map(HeapEntry::item), Some(&"left"));
    }

    #[test]
    fn heap_grows_beyond_legacy_initial_size() {
        let mut heap = ExtractHeap::with_capacity(10);
        for key in 0..25 {
            heap.insert_item(key, key);
        }

        assert!(heap.capacity() >= 25);
        assert_eq!(heap.len(), 25);
        assert_eq!(heap.delete_max().map(HeapEntry::into_item), Some(24));
    }

    #[test]
    fn from_iterator_builds_valid_heap() {
        let mut heap = [entry(2, "two"), entry(8, "eight"), entry(5, "five")]
            .into_iter()
            .collect::<ExtractHeap<_>>();

        assert_eq!(heap.delete_max(), Some(entry(8, "eight")));
        assert_eq!(heap.delete_max(), Some(entry(5, "five")));
        assert_eq!(heap.delete_max(), Some(entry(2, "two")));
    }

    #[test]
    fn clear_drops_entries_and_allows_reuse() {
        let mut heap = ExtractHeap::new();
        heap.insert(entry(1, "old"));
        heap.clear();
        heap.insert(entry(2, "new"));

        assert_eq!(heap.len(), 1);
        assert_eq!(heap.delete_max(), Some(entry(2, "new")));
        assert_eq!(heap.delete_max(), None);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present() {
        let text = include_str!("heap.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
