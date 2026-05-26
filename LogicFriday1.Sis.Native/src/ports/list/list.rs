//! Native Rust port of `LogicSynthesis/sis/list/list.c`.
//!
//! SIS exposes a small generic doubly linked list package. The C version uses
//! raw item handles and generator handles; this port keeps the same operations
//! while validating handles against an owning list and avoiding pointer casts.

use std::cmp::Ordering;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

static NEXT_LIST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListStatus {
    Ok,
    BadState,
    BadParam,
    NoMore,
    Stop,
    Delete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandlePosition {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ListHandle {
    list_id: u64,
    index: usize,
    generation: u64,
}

impl ListHandle {
    pub fn list_id(&self) -> u64 {
        self.list_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListGenerator {
    list_id: u64,
    before_spot: Option<ListHandle>,
    after_spot: Option<ListHandle>,
}

impl ListGenerator {
    pub fn list_id(&self) -> u64 {
        self.list_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ListElement<T> {
    prev: Option<usize>,
    next: Option<usize>,
    generation: u64,
    user_data: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkedList<T> {
    list_id: u64,
    top: Option<usize>,
    bottom: Option<usize>,
    length: usize,
    nodes: Vec<Option<ListElement<T>>>,
    free_nodes: Vec<usize>,
    next_generation: u64,
}

impl<T> Default for LinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self {
            list_id: NEXT_LIST_ID.fetch_add(1, AtomicOrdering::Relaxed),
            top: None,
            bottom: None,
            length: 0,
            nodes: Vec::new(),
            free_nodes: Vec::new(),
            next_generation: 1,
        }
    }

    pub fn list_id(&self) -> u64 {
        self.list_id
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn copy_with<U>(&self, mut copy_func: impl FnMut(&T) -> U) -> LinkedList<U> {
        let mut copy = LinkedList::new();
        let mut cursor = self.top;
        while let Some(index) = cursor {
            let element = self.node_at(index);
            copy.push_back(copy_func(&element.user_data));
            cursor = element.next;
        }

        copy
    }

    pub fn push_front(&mut self, user_data: T) -> ListHandle {
        let old_top = self.top;
        let handle = self.allocate_node(None, old_top, user_data);
        if let Some(old_top) = old_top {
            self.node_at_mut(old_top).prev = Some(handle.index);
        } else {
            self.bottom = Some(handle.index);
        }

        self.top = Some(handle.index);
        self.length += 1;
        handle
    }

    pub fn push_back(&mut self, user_data: T) -> ListHandle {
        let old_bottom = self.bottom;
        let handle = self.allocate_node(old_bottom, None, user_data);
        if let Some(old_bottom) = old_bottom {
            self.node_at_mut(old_bottom).next = Some(handle.index);
        } else {
            self.top = Some(handle.index);
        }

        self.bottom = Some(handle.index);
        self.length += 1;
        handle
    }

    pub fn first(&self) -> Result<(&T, ListHandle), ListStatus> {
        self.top
            .map(|index| {
                let element = self.node_at(index);
                (
                    &element.user_data,
                    self.handle_for(index, element.generation),
                )
            })
            .ok_or(ListStatus::NoMore)
    }

    pub fn last(&self) -> Result<(&T, ListHandle), ListStatus> {
        self.bottom
            .map(|index| {
                let element = self.node_at(index);
                (
                    &element.user_data,
                    self.handle_for(index, element.generation),
                )
            })
            .ok_or(ListStatus::NoMore)
    }

    pub fn pop_front(&mut self) -> Result<T, ListStatus> {
        let Some(index) = self.top else {
            return Err(ListStatus::NoMore);
        };

        Ok(self.unlink_index(index))
    }

    pub fn pop_back(&mut self) -> Result<T, ListStatus> {
        let Some(index) = self.bottom else {
            return Err(ListStatus::NoMore);
        };

        Ok(self.unlink_index(index))
    }

    pub fn start(&self) -> ListGenerator {
        ListGenerator {
            list_id: self.list_id,
            before_spot: None,
            after_spot: self.handle_for_index(self.top),
        }
    }

    pub fn end(&self) -> ListGenerator {
        ListGenerator {
            list_id: self.list_id,
            before_spot: self.handle_for_index(self.bottom),
            after_spot: None,
        }
    }

    pub fn generator_from_handle(
        &self,
        handle: ListHandle,
        position: HandlePosition,
    ) -> Result<(ListGenerator, &T), ListStatus> {
        let index = self.validate_handle(handle)?;
        let element = self.node_at(index);
        let generator = match position {
            HandlePosition::Before => ListGenerator {
                list_id: self.list_id,
                before_spot: self.handle_for_index(element.prev),
                after_spot: Some(handle),
            },
            HandlePosition::After => ListGenerator {
                list_id: self.list_id,
                before_spot: Some(handle),
                after_spot: self.handle_for_index(element.next),
            },
        };

        Ok((generator, &element.user_data))
    }

    pub fn next(&self, generator: &mut ListGenerator) -> Result<(&T, ListHandle), ListStatus> {
        self.validate_generator(generator)?;
        let Some(handle) = generator.after_spot else {
            return Err(ListStatus::NoMore);
        };

        let index = self.validate_handle(handle)?;
        let element = self.node_at(index);
        generator.before_spot = Some(handle);
        generator.after_spot = self.handle_for_index(element.next);
        Ok((&element.user_data, handle))
    }

    pub fn previous(&self, generator: &mut ListGenerator) -> Result<(&T, ListHandle), ListStatus> {
        self.validate_generator(generator)?;
        let Some(handle) = generator.before_spot else {
            return Err(ListStatus::NoMore);
        };

        let index = self.validate_handle(handle)?;
        let element = self.node_at(index);
        generator.after_spot = Some(handle);
        generator.before_spot = self.handle_for_index(element.prev);
        Ok((&element.user_data, handle))
    }

    pub fn insert_before(
        &mut self,
        generator: &mut ListGenerator,
        user_data: T,
    ) -> Result<ListHandle, ListStatus> {
        self.validate_generator(generator)?;
        if generator.before_spot.is_none() {
            let handle = self.push_front(user_data);
            generator.before_spot = Some(handle);
            return Ok(handle);
        }

        if generator.after_spot.is_none() {
            let handle = self.push_back(user_data);
            generator.after_spot = Some(handle);
            return Ok(handle);
        }

        let before = self.validate_handle(generator.before_spot.expect("checked above"))?;
        let after = self.validate_handle(generator.after_spot.expect("checked above"))?;
        let handle = self.insert_between(Some(before), Some(after), user_data);
        generator.before_spot = Some(handle);
        Ok(handle)
    }

    pub fn insert_after(
        &mut self,
        generator: &mut ListGenerator,
        user_data: T,
    ) -> Result<ListHandle, ListStatus> {
        self.validate_generator(generator)?;
        if generator.before_spot.is_none() {
            let handle = self.push_front(user_data);
            generator.before_spot = Some(handle);
            return Ok(handle);
        }

        if generator.after_spot.is_none() {
            let handle = self.push_back(user_data);
            generator.after_spot = Some(handle);
            return Ok(handle);
        }

        let before = self.validate_handle(generator.before_spot.expect("checked above"))?;
        let after = self.validate_handle(generator.after_spot.expect("checked above"))?;
        let handle = self.insert_between(Some(before), Some(after), user_data);
        generator.after_spot = Some(handle);
        Ok(handle)
    }

    pub fn delete_before(&mut self, generator: &mut ListGenerator) -> Result<T, ListStatus> {
        self.validate_generator(generator)?;
        let Some(handle) = generator.before_spot else {
            return Err(ListStatus::BadState);
        };

        let index = self.validate_handle(handle)?;
        let previous = self.node_at(index).prev;
        generator.before_spot = self.handle_for_index(previous);
        Ok(self.unlink_index(index))
    }

    pub fn delete_after(&mut self, generator: &mut ListGenerator) -> Result<T, ListStatus> {
        self.validate_generator(generator)?;
        let Some(handle) = generator.after_spot else {
            return Err(ListStatus::BadState);
        };

        let index = self.validate_handle(handle)?;
        let next = self.node_at(index).next;
        generator.after_spot = self.handle_for_index(next);
        Ok(self.unlink_index(index))
    }

    pub fn foreach(
        &mut self,
        mut user_func: impl FnMut(&T) -> ListStatus,
        mut delete_func: impl FnMut(T),
    ) -> ListStatus {
        let mut generator = self.start();
        while let Ok((_, handle)) = self.next(&mut generator) {
            let status = {
                let item = &self.node_at(handle.index).user_data;
                user_func(item)
            };

            match status {
                ListStatus::Ok => {}
                ListStatus::Stop => return ListStatus::Stop,
                ListStatus::Delete => match self.delete_before(&mut generator) {
                    Ok(item) => delete_func(item),
                    Err(status) => return status,
                },
                _ => return ListStatus::BadParam,
            }
        }

        ListStatus::Ok
    }

    pub fn backeach(
        &mut self,
        mut user_func: impl FnMut(&T) -> ListStatus,
        mut delete_func: impl FnMut(T),
    ) -> ListStatus {
        let mut generator = self.end();
        while let Ok((_, handle)) = self.previous(&mut generator) {
            let status = {
                let item = &self.node_at(handle.index).user_data;
                user_func(item)
            };

            match status {
                ListStatus::Ok => {}
                ListStatus::Stop => return ListStatus::Stop,
                ListStatus::Delete => match self.delete_after(&mut generator) {
                    Ok(item) => delete_func(item),
                    Err(status) => return status,
                },
                _ => return ListStatus::BadParam,
            }
        }

        ListStatus::Ok
    }

    pub fn query_handle(&self, handle: ListHandle) -> Option<u64> {
        self.validate_handle(handle).ok().map(|_| self.list_id)
    }

    pub fn fetch_handle(&self, handle: ListHandle) -> Option<&T> {
        let index = self.validate_handle(handle).ok()?;
        Some(&self.node_at(index).user_data)
    }

    pub fn remove_item(&mut self, handle: ListHandle) -> Result<T, ListStatus> {
        let index = self.validate_handle(handle)?;
        Ok(self.unlink_index(index))
    }

    pub fn sort_by(&mut self, mut compare: impl FnMut(&T, &T) -> Ordering) -> ListStatus {
        let mut indices = self.indices_in_order();
        indices.sort_by(|left, right| {
            let left_item = &self.node_at(*left).user_data;
            let right_item = &self.node_at(*right).user_data;
            compare(left_item, right_item)
        });
        self.relink_in_order(&indices);
        ListStatus::Ok
    }

    pub fn uniq_by(
        &mut self,
        mut compare: impl FnMut(&T, &T) -> Ordering,
        mut delete_func: impl FnMut(T),
    ) -> ListStatus {
        if self.length <= 1 {
            return ListStatus::Ok;
        }

        let mut current = self.top.and_then(|index| self.node_at(index).next);
        while let Some(current_index) = current {
            let previous_index = self
                .node_at(current_index)
                .prev
                .expect("non-first item has previous link");
            let is_duplicate = {
                let current_item = &self.node_at(current_index).user_data;
                let previous_item = &self.node_at(previous_index).user_data;
                compare(current_item, previous_item) == Ordering::Equal
            };

            if is_duplicate {
                current = self.node_at(current_index).next;
                let duplicate = self.unlink_index(current_index);
                delete_func(duplicate);
            } else {
                current = self.node_at(current_index).next;
            }
        }

        ListStatus::Ok
    }

    pub fn to_vec(&self) -> Vec<&T> {
        let mut items = Vec::with_capacity(self.length);
        let mut cursor = self.top;
        while let Some(index) = cursor {
            let element = self.node_at(index);
            items.push(&element.user_data);
            cursor = element.next;
        }

        items
    }

    fn allocate_node(
        &mut self,
        prev: Option<usize>,
        next: Option<usize>,
        user_data: T,
    ) -> ListHandle {
        let generation = self.next_generation;
        self.next_generation += 1;
        let element = ListElement {
            prev,
            next,
            generation,
            user_data,
        };

        let index = match self.free_nodes.pop() {
            Some(index) => {
                self.nodes[index] = Some(element);
                index
            }
            None => {
                self.nodes.push(Some(element));
                self.nodes.len() - 1
            }
        };

        self.handle_for(index, generation)
    }

    fn insert_between(
        &mut self,
        before: Option<usize>,
        after: Option<usize>,
        user_data: T,
    ) -> ListHandle {
        let handle = self.allocate_node(before, after, user_data);
        if let Some(before) = before {
            self.node_at_mut(before).next = Some(handle.index);
        } else {
            self.top = Some(handle.index);
        }

        if let Some(after) = after {
            self.node_at_mut(after).prev = Some(handle.index);
        } else {
            self.bottom = Some(handle.index);
        }

        self.length += 1;
        handle
    }

    fn unlink_index(&mut self, index: usize) -> T {
        let element = self.nodes[index]
            .take()
            .expect("validated element exists before unlink");
        if let Some(previous) = element.prev {
            self.node_at_mut(previous).next = element.next;
        } else {
            self.top = element.next;
        }

        if let Some(next) = element.next {
            self.node_at_mut(next).prev = element.prev;
        } else {
            self.bottom = element.prev;
        }

        self.length -= 1;
        self.free_nodes.push(index);
        element.user_data
    }

    fn validate_generator(&self, generator: &ListGenerator) -> Result<(), ListStatus> {
        if generator.list_id != self.list_id {
            return Err(ListStatus::BadState);
        }

        if let Some(before) = generator.before_spot {
            self.validate_handle(before)?;
        }

        if let Some(after) = generator.after_spot {
            self.validate_handle(after)?;
        }

        Ok(())
    }

    fn validate_handle(&self, handle: ListHandle) -> Result<usize, ListStatus> {
        if handle.list_id != self.list_id {
            return Err(ListStatus::BadState);
        }

        let Some(Some(element)) = self.nodes.get(handle.index) else {
            return Err(ListStatus::BadState);
        };

        if element.generation != handle.generation {
            return Err(ListStatus::BadState);
        }

        Ok(handle.index)
    }

    fn handle_for_index(&self, index: Option<usize>) -> Option<ListHandle> {
        index.map(|index| {
            let generation = self.node_at(index).generation;
            self.handle_for(index, generation)
        })
    }

    fn handle_for(&self, index: usize, generation: u64) -> ListHandle {
        ListHandle {
            list_id: self.list_id,
            index,
            generation,
        }
    }

    fn node_at(&self, index: usize) -> &ListElement<T> {
        self.nodes[index]
            .as_ref()
            .expect("active list link points at active element")
    }

    fn node_at_mut(&mut self, index: usize) -> &mut ListElement<T> {
        self.nodes[index]
            .as_mut()
            .expect("active list link points at active element")
    }

    fn indices_in_order(&self) -> Vec<usize> {
        let mut indices = Vec::with_capacity(self.length);
        let mut cursor = self.top;
        while let Some(index) = cursor {
            indices.push(index);
            cursor = self.node_at(index).next;
        }

        indices
    }

    fn relink_in_order(&mut self, indices: &[usize]) {
        self.top = indices.first().copied();
        self.bottom = indices.last().copied();

        for (position, index) in indices.iter().copied().enumerate() {
            let previous = position
                .checked_sub(1)
                .and_then(|p| indices.get(p))
                .copied();
            let next = indices.get(position + 1).copied();
            let element = self.node_at_mut(index);
            element.prev = previous;
            element.next = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values<T: Copy>(list: &LinkedList<T>) -> Vec<T> {
        list.to_vec().into_iter().copied().collect()
    }

    #[test]
    fn push_and_pop_preserve_endpoints() {
        let mut list = LinkedList::new();
        assert_eq!(list.first(), Err(ListStatus::NoMore));
        assert_eq!(list.last(), Err(ListStatus::NoMore));

        let first = list.push_front(2);
        let second = list.push_front(1);
        let third = list.push_back(3);

        assert_eq!(list.len(), 3);
        assert_eq!(*list.fetch_handle(first).unwrap(), 2);
        assert_eq!(*list.fetch_handle(second).unwrap(), 1);
        assert_eq!(*list.fetch_handle(third).unwrap(), 3);
        assert_eq!(*list.first().unwrap().0, 1);
        assert_eq!(*list.last().unwrap().0, 3);

        assert_eq!(list.pop_front(), Ok(1));
        assert_eq!(list.pop_back(), Ok(3));
        assert_eq!(list.pop_back(), Ok(2));
        assert_eq!(list.pop_back(), Err(ListStatus::NoMore));
        assert!(list.is_empty());
    }

    #[test]
    fn copy_uses_supplied_transform() {
        let mut list = LinkedList::new();
        list.push_back("a");
        list.push_back("bb");

        let copy = list.copy_with(|item| item.len());

        assert_eq!(values(&copy), vec![1, 2]);
    }

    #[test]
    fn generator_walks_forward_and_backward() {
        let mut list = LinkedList::new();
        list.push_back(1);
        list.push_back(2);

        let mut generator = list.start();
        assert_eq!(*list.next(&mut generator).unwrap().0, 1);
        assert_eq!(*list.previous(&mut generator).unwrap().0, 1);
        assert_eq!(list.previous(&mut generator), Err(ListStatus::NoMore));

        let mut generator = list.end();
        assert_eq!(*list.previous(&mut generator).unwrap().0, 2);
        assert_eq!(*list.next(&mut generator).unwrap().0, 2);
        assert_eq!(list.next(&mut generator), Err(ListStatus::NoMore));
    }

    #[test]
    fn generator_from_handle_positions_before_or_after_item() {
        let mut list = LinkedList::new();
        list.push_back(1);
        let middle = list.push_back(2);
        list.push_back(3);

        let (mut before, data) = list
            .generator_from_handle(middle, HandlePosition::Before)
            .unwrap();
        assert_eq!(*data, 2);
        assert_eq!(*list.next(&mut before).unwrap().0, 2);

        let (mut after, _) = list
            .generator_from_handle(middle, HandlePosition::After)
            .unwrap();
        assert_eq!(*list.previous(&mut after).unwrap().0, 2);
        assert_eq!(*list.next(&mut after).unwrap().0, 2);
    }

    #[test]
    fn insert_before_and_after_match_generator_spot_semantics() {
        let mut list = LinkedList::new();
        list.push_back(1);
        list.push_back(4);

        let mut generator = list.start();
        assert_eq!(*list.next(&mut generator).unwrap().0, 1);
        list.insert_after(&mut generator, 3).unwrap();
        assert_eq!(*list.next(&mut generator).unwrap().0, 3);
        list.insert_before(&mut generator, 2).unwrap();
        assert_eq!(*list.previous(&mut generator).unwrap().0, 2);

        assert_eq!(values(&list), vec![1, 3, 2, 4]);
    }

    #[test]
    fn delete_before_and_after_update_generator_spot() {
        let mut list = LinkedList::new();
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);
        list.push_back(4);

        let mut generator = list.start();
        assert_eq!(*list.next(&mut generator).unwrap().0, 1);
        assert_eq!(*list.next(&mut generator).unwrap().0, 2);
        assert_eq!(list.delete_before(&mut generator), Ok(2));
        assert_eq!(*list.next(&mut generator).unwrap().0, 3);
        assert_eq!(list.delete_after(&mut generator), Ok(4));
        assert_eq!(list.next(&mut generator), Err(ListStatus::NoMore));

        assert_eq!(values(&list), vec![1, 3]);
    }

    #[test]
    fn delete_on_empty_side_reports_bad_state() {
        let mut list = LinkedList::<i32>::new();
        let mut start = list.start();
        let mut end = list.end();

        assert_eq!(list.delete_before(&mut start), Err(ListStatus::BadState));
        assert_eq!(list.delete_after(&mut end), Err(ListStatus::BadState));
    }

    #[test]
    fn foreach_can_delete_or_stop() {
        let mut list = LinkedList::new();
        for item in 1..=5 {
            list.push_back(item);
        }

        let mut deleted = Vec::new();
        let status = list.foreach(
            |item| {
                if item % 2 == 0 {
                    ListStatus::Delete
                } else {
                    ListStatus::Ok
                }
            },
            |item| deleted.push(item),
        );

        assert_eq!(status, ListStatus::Ok);
        assert_eq!(deleted, vec![2, 4]);
        assert_eq!(values(&list), vec![1, 3, 5]);

        let status = list.foreach(
            |item| {
                if *item == 3 {
                    ListStatus::Stop
                } else {
                    ListStatus::Ok
                }
            },
            |_| {},
        );
        assert_eq!(status, ListStatus::Stop);
    }

    #[test]
    fn backeach_deletes_from_tail_to_head() {
        let mut list = LinkedList::new();
        for item in 1..=4 {
            list.push_back(item);
        }

        let mut deleted = Vec::new();
        let status = list.backeach(
            |item| {
                if *item > 2 {
                    ListStatus::Delete
                } else {
                    ListStatus::Ok
                }
            },
            |item| deleted.push(item),
        );

        assert_eq!(status, ListStatus::Ok);
        assert_eq!(deleted, vec![4, 3]);
        assert_eq!(values(&list), vec![1, 2]);
    }

    #[test]
    fn remove_item_invalidates_handle_without_breaking_reused_slots() {
        let mut list = LinkedList::new();
        let removed = list.push_back(1);
        assert_eq!(list.remove_item(removed), Ok(1));
        assert_eq!(list.fetch_handle(removed), None);

        let replacement = list.push_back(2);
        assert_ne!(removed, replacement);
        assert_eq!(*list.fetch_handle(replacement).unwrap(), 2);
    }

    #[test]
    fn sort_and_uniq_match_sorted_duplicate_removal() {
        let mut list = LinkedList::new();
        for item in [3, 1, 2, 2, 1, 3, 3] {
            list.push_back(item);
        }

        assert_eq!(list.sort_by(i32::cmp), ListStatus::Ok);
        assert_eq!(values(&list), vec![1, 1, 2, 2, 3, 3, 3]);

        let mut deleted = Vec::new();
        assert_eq!(
            list.uniq_by(i32::cmp, |item| deleted.push(item)),
            ListStatus::Ok
        );

        assert_eq!(values(&list), vec![1, 2, 3]);
        assert_eq!(deleted, vec![1, 2, 3, 3]);
    }

    #[test]
    fn foreign_handles_and_generators_are_rejected() {
        let mut left = LinkedList::new();
        let mut right = LinkedList::new();
        let foreign_handle = right.push_back(10);
        let mut foreign_generator = right.start();

        assert_eq!(left.remove_item(foreign_handle), Err(ListStatus::BadState));
        assert_eq!(
            left.insert_after(&mut foreign_generator, 20),
            Err(ListStatus::BadState)
        );
    }
}
