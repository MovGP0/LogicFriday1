//! Native Rust unique-table support for the CMU BDD package.
//!
//! The original `bddunique.c` owns node interning, complemented-edge
//! canonicalization, reference cleanup, and unique-table garbage collection.
//! This port keeps those responsibilities behind Rust-owned handles instead
//! of preserving the legacy tagged pointer ABI.

use std::fmt;

const MIN_GC_LIMIT: usize = 10_000;
const CONST_INDEX_INDEX: usize = 0;
const GC_MARK: u8 = 0x80;
const MAX_REFS: u8 = u8::MAX;
const MAX_TEMP_REFS: u8 = GC_MARK - 1;
const TABLE_SIZES: [usize; 49] = [
    1, 2, 3, 7, 13, 23, 59, 113, 241, 503, 1019, 2039, 4091, 8179, 11587, 16369, 23143, 32749,
    46349, 65521, 92683, 131063, 185363, 262139, 330287, 416147, 524269, 660557, 832253,
    1048571, 1321109, 1664501, 2097143, 2642201, 3328979, 4194287, 5284393, 6657919, 8388593,
    10568797, 13315831, 16777199, 33554393, 67108859, 134217689, 268435399, 536870879,
    1073741789, 2147483629,
];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddHandle
{
    node: usize,
    complemented: bool,
}

impl BddHandle
{
    pub const fn new(node: usize) -> Self
    {
        Self
        {
            node,
            complemented: false,
        }
    }

    pub const fn node(self) -> usize
    {
        self.node
    }

    pub const fn is_complemented(self) -> bool
    {
        self.complemented
    }

    pub const fn is_outpos(self) -> bool
    {
        !self.complemented
    }

    pub const fn not(self) -> Self
    {
        Self
        {
            node: self.node,
            complemented: !self.complemented,
        }
    }

    fn raw(self) -> isize
    {
        ((self.node as isize) << 1) | isize::from(self.complemented)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BddNodeData
{
    Branch
    {
        then_branch: BddHandle,
        else_branch: BddHandle,
    },
    Terminal
    {
        value1: isize,
        value2: isize,
    },
}

impl BddNodeData
{
    fn key_parts(self) -> (isize, isize)
    {
        match self
        {
            Self::Branch
            {
                then_branch,
                else_branch,
            } => (then_branch.raw(), else_branch.raw()),
            Self::Terminal
            {
                value1,
                value2,
            } => (value1, value2),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNode
{
    index_index: usize,
    refs: u8,
    mark: u8,
    data: BddNodeData,
}

impl BddNode
{
    pub fn index_index(&self) -> usize
    {
        self.index_index
    }

    pub fn refs(&self) -> u8
    {
        self.refs
    }

    pub fn temp_refs(&self) -> u8
    {
        self.mark & !GC_MARK
    }

    pub fn is_marked_for_gc(&self) -> bool
    {
        self.mark & GC_MARK != 0
    }

    pub fn data(&self) -> BddNodeData
    {
        self.data
    }

    fn is_used(&self) -> bool
    {
        self.is_marked_for_gc()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UniqueStats
{
    pub entries: usize,
    pub gc_limit: usize,
    pub node_limit: usize,
    pub gcs: usize,
    pub freed: usize,
    pub finds: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VarTableStats
{
    pub size_index: usize,
    pub size: usize,
    pub entries: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BddUniqueError
{
    InvalidIndexIndex(usize),
    InvalidHandle(BddHandle),
    InvalidNode(usize),
    AbortRequested,
    Overflow,
    Reordered,
}

impl fmt::Display for BddUniqueError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::InvalidIndexIndex(index_index) => {
                write!(formatter, "invalid BDD index index {index_index}")
            }
            Self::InvalidHandle(handle) => {
                write!(formatter, "invalid BDD handle {:?}", handle)
            }
            Self::InvalidNode(node) => {
                write!(formatter, "invalid BDD node {node}")
            }
            Self::AbortRequested => formatter.write_str("BDD operation aborted"),
            Self::Overflow => formatter.write_str("BDD unique table node limit exceeded"),
            Self::Reordered => formatter.write_str("BDD variables reordered"),
        }
    }
}

impl std::error::Error for BddUniqueError
{
}

#[derive(Clone, Debug)]
struct VarTable
{
    size_index: usize,
    entries: usize,
    buckets: Vec<Vec<usize>>,
}

impl VarTable
{
    fn new() -> Self
    {
        let size_index = 3;
        Self
        {
            size_index,
            entries: 0,
            buckets: vec![Vec::new(); table_size(size_index)],
        }
    }

    fn size(&self) -> usize
    {
        self.buckets.len()
    }

    fn stats(&self) -> VarTableStats
    {
        VarTableStats
        {
            size_index: self.size_index,
            size: self.size(),
            entries: self.entries,
        }
    }
}

struct UniqueTable
{
    tables: Vec<VarTable>,
    entries: usize,
    gc_limit: usize,
    node_limit: usize,
    gcs: usize,
    freed: usize,
    finds: usize,
    free_terminal: Option<Box<dyn FnMut(isize, isize)>>,
}

impl UniqueTable
{
    fn new(maxvars: usize) -> Self
    {
        let mut tables = Vec::with_capacity(maxvars + 1);
        tables.resize_with(maxvars + 1, VarTable::new);

        Self
        {
            tables,
            entries: 0,
            gc_limit: MIN_GC_LIMIT,
            node_limit: 0,
            gcs: 0,
            freed: 0,
            finds: 0,
            free_terminal: None,
        }
    }

    fn stats(&self) -> UniqueStats
    {
        UniqueStats
        {
            entries: self.entries,
            gc_limit: self.gc_limit,
            node_limit: self.node_limit,
            gcs: self.gcs,
            freed: self.freed,
            finds: self.finds,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CacheStats
{
    pub size: usize,
    pub entries: usize,
    pub cache_ratio: usize,
    pub purges: usize,
    pub rehashes: usize,
}

pub struct BddManager
{
    unique_table: UniqueTable,
    nodes: Vec<Option<BddNode>>,
    check: usize,
    one: BddHandle,
    zero: BddHandle,
    variables: Vec<Option<BddHandle>>,
    vars: usize,
    maxvars: usize,
    index_indexes: Vec<usize>,
    associations: Vec<Vec<Option<BddHandle>>>,
    op_cache: CacheStats,
    allow_reordering: bool,
    nodes_at_start: usize,
    overflow: bool,
    bag_it: Option<Box<dyn FnMut()>>,
    overflow_fn: Option<Box<dyn FnMut()>>,
    reorder_fn: Option<Box<dyn FnMut()>>,
    canonical_terminal: Box<dyn Fn(isize, isize) -> bool>,
    transform_terminal: Box<dyn Fn(isize, isize) -> (isize, isize)>,
}

impl BddManager
{
    pub fn new(maxvars: usize) -> Self
    {
        let mut manager = Self
        {
            unique_table: UniqueTable::new(maxvars),
            nodes: Vec::new(),
            check: 100,
            one: BddHandle::new(0),
            zero: BddHandle::new(0).not(),
            variables: vec![None; maxvars + 1],
            vars: 0,
            maxvars,
            index_indexes: (0..=maxvars).collect(),
            associations: Vec::new(),
            op_cache: CacheStats
            {
                cache_ratio: 4,
                ..CacheStats::default()
            },
            allow_reordering: false,
            nodes_at_start: 0,
            overflow: false,
            bag_it: None,
            overflow_fn: None,
            reorder_fn: None,
            canonical_terminal: Box::new(|_, _| false),
            transform_terminal: Box::new(|value1, value2| (value1, value2)),
        };

        let one = manager.find_aux(CONST_INDEX_INDEX, BddNodeData::Terminal {
            value1: 1,
            value2: 0,
        }).expect("constant table is initialized");
        manager.node_mut(one).refs = MAX_REFS;
        manager.one = one;
        manager.zero = one.not();

        manager
    }

    pub fn one(&self) -> BddHandle
    {
        self.one
    }

    pub fn zero(&self) -> BddHandle
    {
        self.zero
    }

    pub fn vars(&self) -> usize
    {
        self.vars
    }

    pub fn maxvars(&self) -> usize
    {
        self.maxvars
    }

    pub fn unique_stats(&self) -> UniqueStats
    {
        self.unique_table.stats()
    }

    pub fn cache_stats(&self) -> &CacheStats
    {
        &self.op_cache
    }

    pub fn var_table_stats(&self, index_index: usize) -> Result<VarTableStats, BddUniqueError>
    {
        Ok(self.var_table(index_index)?.stats())
    }

    pub fn node(&self, handle: BddHandle) -> Result<&BddNode, BddUniqueError>
    {
        self.nodes
            .get(handle.node)
            .and_then(Option::as_ref)
            .ok_or(BddUniqueError::InvalidHandle(handle))
    }

    pub fn set_node_limit(&mut self, node_limit: usize)
    {
        self.unique_table.node_limit = node_limit;
        self.set_gc_limit();
    }

    pub fn set_free_terminal<F>(&mut self, free_terminal: F)
    where
        F: FnMut(isize, isize) + 'static,
    {
        self.unique_table.free_terminal = Some(Box::new(free_terminal));
    }

    pub fn clear_free_terminal(&mut self)
    {
        self.unique_table.free_terminal = None;
    }

    pub fn set_overflow_handler<F>(&mut self, overflow_fn: F)
    where
        F: FnMut() + 'static,
    {
        self.overflow_fn = Some(Box::new(overflow_fn));
    }

    pub fn set_abort_handler<F>(&mut self, bag_it: F)
    where
        F: FnMut() + 'static,
    {
        self.bag_it = Some(Box::new(bag_it));
    }

    pub fn set_reorder_handler<F>(&mut self, reorder_fn: F)
    where
        F: FnMut() + 'static,
    {
        self.reorder_fn = Some(Box::new(reorder_fn));
    }

    pub fn set_terminal_transform<C, T>(&mut self, canonical: C, transform: T)
    where
        C: Fn(isize, isize) -> bool + 'static,
        T: Fn(isize, isize) -> (isize, isize) + 'static,
    {
        self.canonical_terminal = Box::new(canonical);
        self.transform_terminal = Box::new(transform);
    }

    pub fn create_variable(&mut self, index_index: usize) -> Result<BddHandle, BddUniqueError>
    {
        self.ensure_index_index(index_index)?;
        self.vars = self.vars.max(index_index);
        self.index_indexes[index_index] = index_index;

        let variable = self.find(index_index, self.one, self.zero)?;
        self.node_mut(variable).refs = MAX_REFS;
        self.variables[index_index] = Some(variable);

        Ok(variable)
    }

    pub fn find(
        &mut self,
        index_index: usize,
        then_branch: BddHandle,
        else_branch: BddHandle,
    ) -> Result<BddHandle, BddUniqueError>
    {
        self.ensure_handle(then_branch)?;
        self.ensure_handle(else_branch)?;

        let result = if then_branch == else_branch
        {
            self.temp_decrefs(then_branch)?;
            then_branch
        }
        else if then_branch.is_outpos()
        {
            let result = self.find_aux(index_index, BddNodeData::Branch {
                then_branch,
                else_branch,
            })?;
            self.temp_increfs(result)?;
            self.temp_decrefs(then_branch)?;
            self.temp_decrefs(else_branch)?;
            result
        }
        else
        {
            let result = self.find_aux(index_index, BddNodeData::Branch {
                then_branch: then_branch.not(),
                else_branch: else_branch.not(),
            })?.not();
            self.temp_increfs(result)?;
            self.temp_decrefs(then_branch)?;
            self.temp_decrefs(else_branch)?;
            result
        };

        self.after_find()?;
        Ok(result)
    }

    pub fn find_terminal(
        &mut self,
        mut value1: isize,
        mut value2: isize,
    ) -> Result<BddHandle, BddUniqueError>
    {
        let complemented = (self.canonical_terminal)(value1, value2);

        if complemented
        {
            (value1, value2) = (self.transform_terminal)(value1, value2);
        }

        let mut result = self.find_aux(CONST_INDEX_INDEX, BddNodeData::Terminal {
            value1,
            value2,
        })?;

        if complemented
        {
            result = result.not();
        }

        self.temp_increfs(result)?;
        self.after_find()?;
        Ok(result)
    }

    pub fn increfs(&mut self, handle: BddHandle) -> Result<(), BddUniqueError>
    {
        let node = self.node_mut(handle);

        if node.refs >= MAX_REFS - 1
        {
            node.refs = MAX_REFS;
            node.mark &= GC_MARK;
        }
        else
        {
            node.refs += 1;
        }

        Ok(())
    }

    pub fn decrefs(&mut self, handle: BddHandle) -> Result<(), BddUniqueError>
    {
        let node = self.node_mut(handle);

        if node.refs < MAX_REFS
        {
            node.refs = node.refs.saturating_sub(1);
        }

        Ok(())
    }

    pub fn temp_increfs(&mut self, handle: BddHandle) -> Result<(), BddUniqueError>
    {
        let node = self.node_mut(handle);

        if node.refs < MAX_REFS
        {
            let temp_refs = node.temp_refs().saturating_add(1);

            if temp_refs == MAX_TEMP_REFS
            {
                node.refs = MAX_REFS;
                node.mark &= GC_MARK;
            }
            else
            {
                node.mark = (node.mark & GC_MARK) | temp_refs;
            }
        }

        Ok(())
    }

    pub fn temp_decrefs(&mut self, handle: BddHandle) -> Result<(), BddUniqueError>
    {
        let node = self.node_mut(handle);

        if node.refs < MAX_REFS
        {
            let temp_refs = node.temp_refs().saturating_sub(1);
            node.mark = (node.mark & GC_MARK) | temp_refs;
        }

        Ok(())
    }

    pub fn clear_temps(&mut self) -> Result<(), BddUniqueError>
    {
        for index_index in 0..=self.vars
        {
            let table_nodes = self.table_nodes(index_index)?;

            for node_id in table_nodes
            {
                self.node_mut_by_id(node_id)?.mark &= GC_MARK;
            }
        }

        self.gc()?;
        self.set_gc_limit();
        Ok(())
    }

    pub fn cleanup(&mut self, code: CleanupCode) -> Result<(), BddUniqueError>
    {
        self.clear_temps()?;

        match code
        {
            CleanupCode::Aborted => {
                if let Some(bag_it) = &mut self.bag_it
                {
                    bag_it();
                }
                Err(BddUniqueError::AbortRequested)
            }
            CleanupCode::Overflowed => {
                if let Some(overflow_fn) = &mut self.overflow_fn
                {
                    overflow_fn();
                }
                Err(BddUniqueError::Overflow)
            }
            CleanupCode::Reordered => Err(BddUniqueError::Reordered),
        }
    }

    pub fn gc(&mut self) -> Result<(), BddUniqueError>
    {
        self.mark_live_nodes()?;
        self.purge_cache();
        self.sweep()?;
        self.unique_table.gcs += 1;
        Ok(())
    }

    pub fn sweep(&mut self) -> Result<(), BddUniqueError>
    {
        for index_index in 0..=self.vars
        {
            self.sweep_var_table(index_index, true)?;
        }

        Ok(())
    }

    pub fn sweep_var_table(
        &mut self,
        index_index: usize,
        maybe_rehash: bool,
    ) -> Result<(), BddUniqueError>
    {
        self.ensure_index_index(index_index)?;
        let mut removed = Vec::new();
        let bucket_count = self.var_table(index_index)?.size();

        for bucket_index in 0..bucket_count
        {
            let old_bucket = std::mem::take(&mut self.unique_table.tables[index_index].buckets[bucket_index]);
            let mut retained = Vec::with_capacity(old_bucket.len());

            for node_id in old_bucket
            {
                let used = self.node_by_id(node_id)?.is_used();

                if used
                {
                    self.node_mut_by_id(node_id)?.mark &= !GC_MARK;
                    retained.push(node_id);
                }
                else
                {
                    removed.push(node_id);
                }
            }

            self.unique_table.tables[index_index].buckets[bucket_index] = retained;
        }

        for node_id in removed
        {
            let node = self
                .nodes
                .get_mut(node_id)
                .and_then(Option::take)
                .ok_or(BddUniqueError::InvalidNode(node_id))?;

            if index_index == CONST_INDEX_INDEX
            {
                if let Some(free_terminal) = &mut self.unique_table.free_terminal
                {
                    if let BddNodeData::Terminal
                    {
                        value1,
                        value2,
                    } = node.data
                    {
                        free_terminal(value1, value2);
                    }
                }
            }

            self.unique_table.tables[index_index].entries -= 1;
            self.unique_table.entries -= 1;
            self.unique_table.freed += 1;
        }

        let table = self.var_table(index_index)?;

        if maybe_rehash && table.size() > table.entries && table.size_index > 3
        {
            self.rehash_var_table(index_index, false)?;
        }

        Ok(())
    }

    pub fn clear_refs(&mut self) -> Result<(), BddUniqueError>
    {
        for index_index in 0..=self.vars
        {
            for node_id in self.table_nodes(index_index)?
            {
                self.node_mut_by_id(node_id)?.refs = 0;
            }
        }

        for index_index in 1..=self.vars
        {
            if let Some(variable) = self.variables[index_index]
            {
                self.node_mut(variable).refs = MAX_REFS;
            }
        }

        self.node_mut(self.one).refs = MAX_REFS;

        let association_handles: Vec<_> = self
            .associations
            .iter()
            .flat_map(|association| association.iter().copied().flatten())
            .collect();

        for handle in association_handles
        {
            self.increfs(handle)?;
        }

        Ok(())
    }

    pub fn rehash_var_table(
        &mut self,
        index_index: usize,
        grow: bool,
    ) -> Result<(), BddUniqueError>
    {
        self.ensure_index_index(index_index)?;
        let table = &mut self.unique_table.tables[index_index];

        if grow
        {
            table.size_index = (table.size_index + 1).min(TABLE_SIZES.len() - 1);
        }
        else
        {
            table.size_index = table.size_index.saturating_sub(1);
        }

        let mut new_buckets = vec![Vec::new(); table_size(table.size_index)];

        for node_id in table.buckets.iter().flatten().copied()
        {
            let node = self
                .nodes
                .get(node_id)
                .and_then(Option::as_ref)
                .ok_or(BddUniqueError::InvalidNode(node_id))?;
            let hash = reduce_hash(hash_node(node.data), new_buckets.len());
            new_buckets[hash].push(node_id);
        }

        table.buckets = new_buckets;
        Ok(())
    }

    pub fn add_association(&mut self, association: Vec<Option<BddHandle>>)
    {
        self.associations.push(association);
    }

    fn find_aux(
        &mut self,
        index_index: usize,
        data: BddNodeData,
    ) -> Result<BddHandle, BddUniqueError>
    {
        self.ensure_index_index(index_index)?;
        let hash = reduce_hash(hash_node(data), self.unique_table.tables[index_index].size());

        for node_id in self.unique_table.tables[index_index].buckets[hash].iter().copied()
        {
            let node = self.node_by_id(node_id)?;

            if node.data == data
            {
                self.unique_table.finds += 1;
                return Ok(BddHandle::new(node_id));
            }
        }

        let node_id = self.nodes.len();
        self.nodes.push(Some(BddNode
        {
            index_index,
            refs: 0,
            mark: 0,
            data,
        }));
        self.unique_table.tables[index_index].buckets[hash].push(node_id);
        self.unique_table.tables[index_index].entries += 1;
        self.unique_table.entries += 1;

        if 4 * self.unique_table.tables[index_index].size() < self.unique_table.tables[index_index].entries
        {
            self.rehash_var_table(index_index, true)?;
        }

        self.unique_table.finds += 1;
        Ok(BddHandle::new(node_id))
    }

    fn after_find(&mut self) -> Result<(), BddUniqueError>
    {
        self.check = self.check.saturating_sub(1);

        if self.check == 0
        {
            self.check_unique_table()?;
        }

        Ok(())
    }

    fn check_unique_table(&mut self) -> Result<(), BddUniqueError>
    {
        self.check = 100;

        if self.bag_it.is_some()
        {
            return self.cleanup(CleanupCode::Aborted);
        }

        if self.unique_table.entries > self.unique_table.gc_limit
        {
            self.gc()?;
            let nodes = self.unique_table.entries;

            if 3 * nodes > 2 * self.unique_table.gc_limit
                && self.allow_reordering
                && self.reorder_fn.is_some()
            {
                if let Some(reorder_fn) = &mut self.reorder_fn
                {
                    reorder_fn();
                }

                if 4 * self.unique_table.entries > 3 * nodes && 3 * nodes > 4 * self.nodes_at_start
                {
                    self.allow_reordering = false;
                }

                self.set_gc_limit();
                return Err(BddUniqueError::Reordered);
            }

            self.set_gc_limit();

            if self.unique_table.node_limit != 0
                && self.unique_table.entries >= self.unique_table.node_limit.saturating_sub(1000)
            {
                self.overflow = true;
                return self.cleanup(CleanupCode::Overflowed);
            }
        }

        if 3 * self.op_cache.size < 2 * self.op_cache.entries
            && 32 * self.op_cache.size < self.op_cache.cache_ratio * self.unique_table.entries
        {
            self.rehash_cache();
        }

        Ok(())
    }

    fn set_gc_limit(&mut self)
    {
        self.unique_table.gc_limit = (2 * self.unique_table.entries).max(MIN_GC_LIMIT);

        if self.unique_table.node_limit != 0 && self.unique_table.gc_limit > self.unique_table.node_limit
        {
            self.unique_table.gc_limit = self.unique_table.node_limit;
        }
    }

    fn mark_live_nodes(&mut self) -> Result<(), BddUniqueError>
    {
        let mut stack = Vec::new();

        for index_index in 0..=self.vars
        {
            for node_id in self.table_nodes(index_index)?
            {
                let node = self.node_by_id(node_id)?;

                if node.refs != 0 || node.temp_refs() != 0
                {
                    stack.push(node_id);
                }
            }
        }

        while let Some(node_id) = stack.pop()
        {
            let node = self.node_mut_by_id(node_id)?;

            if node.is_used()
            {
                continue;
            }

            node.mark |= GC_MARK;

            if let BddNodeData::Branch
            {
                then_branch,
                else_branch,
            } = node.data
            {
                stack.push(then_branch.node);
                stack.push(else_branch.node);
            }
        }

        Ok(())
    }

    fn purge_cache(&mut self)
    {
        self.op_cache.entries = 0;
        self.op_cache.purges += 1;
    }

    fn rehash_cache(&mut self)
    {
        self.op_cache.size = if self.op_cache.size == 0
        {
            1
        }
        else
        {
            self.op_cache.size * 2
        };
        self.op_cache.rehashes += 1;
    }

    fn table_nodes(&self, index_index: usize) -> Result<Vec<usize>, BddUniqueError>
    {
        Ok(self
            .var_table(index_index)?
            .buckets
            .iter()
            .flatten()
            .copied()
            .collect())
    }

    fn ensure_index_index(&self, index_index: usize) -> Result<(), BddUniqueError>
    {
        if index_index >= self.unique_table.tables.len()
        {
            Err(BddUniqueError::InvalidIndexIndex(index_index))
        }
        else
        {
            Ok(())
        }
    }

    fn ensure_handle(&self, handle: BddHandle) -> Result<(), BddUniqueError>
    {
        self.node(handle).map(|_| ())
    }

    fn var_table(&self, index_index: usize) -> Result<&VarTable, BddUniqueError>
    {
        self.unique_table
            .tables
            .get(index_index)
            .ok_or(BddUniqueError::InvalidIndexIndex(index_index))
    }

    fn node_by_id(&self, node_id: usize) -> Result<&BddNode, BddUniqueError>
    {
        self.nodes
            .get(node_id)
            .and_then(Option::as_ref)
            .ok_or(BddUniqueError::InvalidNode(node_id))
    }

    fn node_mut(&mut self, handle: BddHandle) -> &mut BddNode
    {
        self.nodes[handle.node]
            .as_mut()
            .expect("checked handles reference live nodes")
    }

    fn node_mut_by_id(&mut self, node_id: usize) -> Result<&mut BddNode, BddUniqueError>
    {
        self.nodes
            .get_mut(node_id)
            .and_then(Option::as_mut)
            .ok_or(BddUniqueError::InvalidNode(node_id))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanupCode
{
    Aborted,
    Overflowed,
    Reordered,
}

fn table_size(size_index: usize) -> usize
{
    TABLE_SIZES[size_index]
}

fn hash_node(data: BddNodeData) -> isize
{
    let (first, second) = data.key_parts();
    first.wrapping_shl(1).wrapping_add(second)
}

fn reduce_hash(hash: isize, size: usize) -> usize
{
    hash.rem_euclid(size as isize) as usize
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn initializes_constant_table_with_one_and_complemented_zero()
    {
        let manager = BddManager::new(4);

        assert_eq!(manager.one(), BddHandle::new(0));
        assert_eq!(manager.zero(), BddHandle::new(0).not());
        assert_eq!(manager.node(manager.one()).unwrap().refs(), MAX_REFS);
        assert_eq!(manager.var_table_stats(CONST_INDEX_INDEX).unwrap().entries, 1);
    }

    #[test]
    fn find_reuses_existing_branch_nodes()
    {
        let mut manager = BddManager::new(4);

        let first = manager.find(1, manager.one(), manager.zero()).unwrap();
        let second = manager.find(1, manager.one(), manager.zero()).unwrap();

        assert_eq!(first, second);
        assert_eq!(manager.var_table_stats(1).unwrap().entries, 1);
        assert_eq!(manager.unique_stats().finds, 3);
    }

    #[test]
    fn find_canonicalizes_complemented_then_branch()
    {
        let mut manager = BddManager::new(4);

        let normal = manager.find(1, manager.one(), manager.zero()).unwrap();
        let complemented = manager.find(1, manager.zero(), manager.one()).unwrap();

        assert_eq!(complemented, normal.not());
        assert_eq!(manager.var_table_stats(1).unwrap().entries, 1);
    }

    #[test]
    fn find_returns_then_branch_when_children_are_identical()
    {
        let mut manager = BddManager::new(4);

        let result = manager.find(1, manager.one(), manager.one()).unwrap();

        assert_eq!(result, manager.one());
        assert_eq!(manager.var_table_stats(1).unwrap().entries, 0);
    }

    #[test]
    fn terminal_transform_stores_canonical_value_and_returns_complement()
    {
        let mut manager = BddManager::new(2);
        manager.set_terminal_transform(|value1, _| value1 < 0, |value1, value2| (-value1, value2));

        let negative = manager.find_terminal(-7, 3).unwrap();
        let positive = manager.find_terminal(7, 3).unwrap();

        assert_eq!(negative, positive.not());
        assert_eq!(manager.var_table_stats(CONST_INDEX_INDEX).unwrap().entries, 2);
    }

    #[test]
    fn variable_table_rehashes_when_density_exceeds_legacy_threshold()
    {
        let mut manager = BddManager::new(16);

        for value in 1..=29
        {
            let terminal = manager.find_terminal(value, 0).unwrap();
            manager.find(1, terminal, manager.zero()).unwrap();
        }

        assert!(manager.var_table_stats(1).unwrap().size_index > 3);
    }

    #[test]
    fn garbage_collection_keeps_referenced_roots_and_their_children()
    {
        let mut manager = BddManager::new(4);
        let variable = manager.find(1, manager.one(), manager.zero()).unwrap();
        let parent = manager.find(2, variable, manager.zero()).unwrap();
        manager.increfs(parent).unwrap();
        manager.clear_temps().unwrap();

        assert!(manager.node(parent).is_ok());
        assert!(manager.node(variable).is_ok());
    }

    #[test]
    fn sweep_removes_unreferenced_nodes_and_invokes_terminal_cleanup()
    {
        let freed = Rc::new(RefCell::new(Vec::new()));
        let freed_for_callback = Rc::clone(&freed);
        let mut manager = BddManager::new(4);
        manager.set_free_terminal(move |value1, value2| {
            freed_for_callback.borrow_mut().push((value1, value2));
        });
        let terminal = manager.find_terminal(42, 0).unwrap();
        manager.temp_decrefs(terminal).unwrap();

        manager.gc().unwrap();

        assert!(manager.node(terminal).is_err());
        assert_eq!(freed.borrow().as_slice(), &[(42, 0)]);
        assert_eq!(manager.unique_stats().freed, 1);
    }

    #[test]
    fn clear_refs_restores_permanent_variables_and_one()
    {
        let mut manager = BddManager::new(4);
        let variable = manager.create_variable(1).unwrap();
        let terminal = manager.find_terminal(7, 0).unwrap();
        let temporary = manager.find(1, manager.one(), terminal).unwrap();
        manager.increfs(temporary).unwrap();

        manager.clear_refs().unwrap();

        assert_eq!(manager.node(variable).unwrap().refs(), MAX_REFS);
        assert_eq!(manager.node(manager.one()).unwrap().refs(), MAX_REFS);
        assert_eq!(manager.node(temporary).unwrap().refs(), 0);
    }

    #[test]
    fn check_reports_requested_abort_and_runs_cleanup_handler()
    {
        let aborted = Rc::new(RefCell::new(false));
        let aborted_for_callback = Rc::clone(&aborted);
        let mut manager = BddManager::new(2);
        manager.set_abort_handler(move || {
            *aborted_for_callback.borrow_mut() = true;
        });

        for _ in 0..99
        {
            manager.find_terminal(1, 0).unwrap();
        }

        let error = manager.find_terminal(2, 0).unwrap_err();

        assert_eq!(error, BddUniqueError::AbortRequested);
        assert!(*aborted.borrow());
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present()
    {
        let source = include_str!("bddunique.rs");

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
