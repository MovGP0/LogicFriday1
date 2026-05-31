//! Native Rust basic BDD routines for the CMU BDD package.
//!
//! This module ports the public behavior from the original basic BDD unit to
//! an owned Rust manager. Handles keep the legacy complemented-edge semantics,
//! while nodes, variables, reference counts, and operation caches are ordinary
//! Rust data structures.

use std::collections::HashMap;
use std::fmt;

const MAX_REFS: u8 = u8::MAX;
const DEFAULT_MAX_VARIABLES: usize = 30;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Bdd {
    Constant(bool),
    Node { id: usize, complemented: bool },
}

impl Bdd {
    pub const fn not(self) -> Self {
        match self {
            Self::Constant(value) => Self::Constant(!value),
            Self::Node { id, complemented } => Self::Node {
                id,
                complemented: !complemented,
            },
        }
    }

    pub const fn is_constant(self) -> bool {
        matches!(self, Self::Constant(_))
    }

    fn node_id(self) -> Option<usize> {
        match self {
            Self::Constant(_) => None,
            Self::Node { id, .. } => Some(id),
        }
    }

    fn is_outpos(self) -> bool {
        !matches!(
            self,
            Self::Node {
                complemented: true,
                ..
            }
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddType {
    NonTerminal,
    Zero,
    One,
    Constant,
    PositiveVariable,
    NegativeVariable,
    Overflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddError {
    InvalidHandle(Bdd),
    IndexOutOfRange { index: usize, variables: usize },
    NoMoreIndexes,
    NotPositiveVariable(Bdd),
    ConstantArgument(Bdd),
    ReferenceUnderflow(Bdd),
}

impl fmt::Display for BddError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle(reference) => write!(formatter, "invalid BDD handle {reference:?}"),
            Self::IndexOutOfRange { index, variables } => write!(
                formatter,
                "variable index {index} is outside the current order with {variables} variables"
            ),
            Self::NoMoreIndexes => {
                formatter.write_str("no more BDD variable indexes are available")
            }
            Self::NotPositiveVariable(reference) => {
                write!(formatter, "expected a positive variable, got {reference:?}")
            }
            Self::ConstantArgument(reference) => {
                write!(formatter, "expected a nonconstant BDD, got {reference:?}")
            }
            Self::ReferenceUnderflow(reference) => {
                write!(
                    formatter,
                    "attempt to free BDD node with zero references: {reference:?}"
                )
            }
        }
    }
}

impl std::error::Error for BddError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddStats {
    pub approximate_bytes_used: usize,
    pub node_count: usize,
    pub node_limit: usize,
    pub overflow: bool,
    pub cache_entries: usize,
    pub cache_lookups: usize,
    pub cache_hits: usize,
    pub cache_insertions: usize,
    pub variable_count: usize,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct NodeKey {
    variable_id: usize,
    high: Bdd,
    low: Bdd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BddNode {
    variable_id: usize,
    high: Bdd,
    low: Bdd,
    refs: u8,
    temp_refs: u8,
}

#[derive(Clone, Debug)]
pub struct VariableBlock {
    reorderable: bool,
    first_index: isize,
    last_index: isize,
    children: Vec<VariableBlock>,
}

impl VariableBlock {
    pub fn new(reorderable: bool, first_index: isize, last_index: isize) -> Self {
        Self {
            reorderable,
            first_index,
            last_index,
            children: Vec::new(),
        }
    }

    pub fn reorderable(&self) -> bool {
        self.reorderable
    }

    pub fn first_index(&self) -> isize {
        self.first_index
    }

    pub fn last_index(&self) -> isize {
        self.last_index
    }

    pub fn children(&self) -> &[VariableBlock] {
        &self.children
    }

    pub fn find_block(&self, index: isize) -> usize {
        let mut first = 0;
        let mut last = self.children.len();

        while first < last {
            let middle = first + (last - first) / 2;
            let child = &self.children[middle];

            if child.first_index <= index && child.last_index >= index {
                return middle;
            }

            if child.first_index > index {
                last = middle;
            } else {
                first = middle + 1;
            }
        }

        first
    }

    pub fn shift_indexes(&mut self, delta: isize) {
        self.first_index += delta;
        self.last_index += delta;

        for child in &mut self.children {
            child.shift_indexes(delta);
        }
    }

    fn shift_for_insert(&mut self, index: isize) {
        if self.first_index >= index {
            self.shift_indexes(1);
            return;
        }

        if self.last_index < index {
            return;
        }

        self.last_index += 1;
        let child_index = self.find_block(index);

        if child_index == self.children.len() || self.children[child_index].first_index == index {
            for child in &mut self.children[child_index..] {
                child.shift_for_insert(index);
            }

            self.children
                .insert(child_index, Self::new(false, index, index));
            return;
        }

        for child in &mut self.children[child_index..] {
            child.shift_for_insert(index);
        }
    }
}

#[derive(Default)]
struct OperationCache {
    ite: HashMap<(Bdd, Bdd, Bdd), Bdd>,
    lookups: usize,
    hits: usize,
    insertions: usize,
    cache_ratio: usize,
}

impl OperationCache {
    fn new() -> Self {
        Self {
            cache_ratio: 4,
            ..Self::default()
        }
    }

    fn lookup_ite(&mut self, f: Bdd, g: Bdd, h: Bdd) -> Option<Bdd> {
        self.lookups += 1;
        let result = self.ite.get(&(f, g, h)).copied();

        if result.is_some() {
            self.hits += 1;
        }

        result
    }

    fn insert_ite(&mut self, f: Bdd, g: Bdd, h: Bdd, result: Bdd) {
        self.ite.insert((f, g, h), result);
        self.insertions += 1;
    }
}

pub struct BddManager {
    nodes: Vec<Option<BddNode>>,
    unique_table: HashMap<NodeKey, usize>,
    variables_by_id: Vec<Option<Bdd>>,
    variable_order: Vec<usize>,
    next_variable_id: usize,
    max_variables: usize,
    op_cache: OperationCache,
    super_block: VariableBlock,
    node_limit: usize,
    overflow: bool,
    overflow_fn: Option<Box<dyn FnMut()>>,
    abort_fn: Option<Box<dyn FnMut()>>,
}

impl BddManager {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_VARIABLES)
    }

    pub fn with_capacity(max_variables: usize) -> Self {
        Self {
            nodes: Vec::new(),
            unique_table: HashMap::new(),
            variables_by_id: Vec::new(),
            variable_order: Vec::new(),
            next_variable_id: 0,
            max_variables,
            op_cache: OperationCache::new(),
            super_block: VariableBlock::new(true, -1, 0),
            node_limit: 0,
            overflow: false,
            overflow_fn: None,
            abort_fn: None,
        }
    }

    pub const fn one(&self) -> Bdd {
        Bdd::Constant(true)
    }

    pub const fn zero(&self) -> Bdd {
        Bdd::Constant(false)
    }

    pub fn make_external(&mut self, function: Bdd) -> Result<Bdd, BddError> {
        self.ensure_handle(function)?;
        self.increfs(function)?;
        self.temp_decrefs_if_present(function);
        Ok(function)
    }

    pub fn new_var_first(&mut self) -> Result<Bdd, BddError> {
        self.new_var(0)
    }

    pub fn new_var_last(&mut self) -> Result<Bdd, BddError> {
        self.new_var(self.variable_order.len())
    }

    pub fn new_var_before(&mut self, variable: Bdd) -> Result<Bdd, BddError> {
        if self.bdd_type(variable)? != BddType::PositiveVariable {
            if variable.is_constant() {
                return self.new_var_last();
            }

            return Err(BddError::NotPositiveVariable(variable));
        }

        self.new_var(self.if_index(variable)? as usize)
    }

    pub fn new_var_after(&mut self, variable: Bdd) -> Result<Bdd, BddError> {
        if self.bdd_type(variable)? != BddType::PositiveVariable {
            if variable.is_constant() {
                return self.new_var_last();
            }

            return Err(BddError::NotPositiveVariable(variable));
        }

        self.new_var(self.if_index(variable)? as usize + 1)
    }

    pub fn var_with_index(&self, index: usize) -> Option<Bdd> {
        let variable_id = *self.variable_order.get(index)?;
        self.variables_by_id
            .get(variable_id)
            .and_then(|variable| *variable)
    }

    pub fn var_with_id(&self, variable_id: usize) -> Option<Bdd> {
        self.variables_by_id
            .get(variable_id)
            .and_then(|variable| *variable)
    }

    pub fn ite(&mut self, f: Bdd, g: Bdd, h: Bdd) -> Result<Bdd, BddError> {
        self.ensure_handle(f)?;
        self.ensure_handle(g)?;
        self.ensure_handle(h)?;

        let result = self.ite_step(f, g, h)?;
        self.make_external(result)
    }

    pub fn and(&mut self, f: Bdd, g: Bdd) -> Result<Bdd, BddError> {
        self.ite(f, g, self.zero())
    }

    pub fn nand(&mut self, f: Bdd, g: Bdd) -> Result<Bdd, BddError> {
        Ok(self.and(f, g)?.not())
    }

    pub fn or(&mut self, f: Bdd, g: Bdd) -> Result<Bdd, BddError> {
        self.ite(f, self.one(), g)
    }

    pub fn nor(&mut self, f: Bdd, g: Bdd) -> Result<Bdd, BddError> {
        Ok(self.or(f, g)?.not())
    }

    pub fn xor(&mut self, f: Bdd, g: Bdd) -> Result<Bdd, BddError> {
        self.ite(f, g.not(), g)
    }

    pub fn xnor(&mut self, f: Bdd, g: Bdd) -> Result<Bdd, BddError> {
        Ok(self.xor(f, g)?.not())
    }

    pub fn identity(&mut self, function: Bdd) -> Result<Bdd, BddError> {
        self.ensure_handle(function)?;
        self.increfs(function)?;
        Ok(function)
    }

    pub fn not(&mut self, function: Bdd) -> Result<Bdd, BddError> {
        self.identity(function)?;
        Ok(function.not())
    }

    pub fn if_variable(&mut self, function: Bdd) -> Result<Bdd, BddError> {
        self.ensure_handle(function)?;

        if function.is_constant() {
            return Ok(function);
        }

        let variable_id = self.node(function)?.variable_id;
        self.variables_by_id
            .get(variable_id)
            .and_then(|variable| *variable)
            .ok_or(BddError::InvalidHandle(function))
    }

    pub fn if_index(&self, function: Bdd) -> Result<isize, BddError> {
        self.ensure_handle(function)?;

        let Some(variable_id) = function
            .node_id()
            .map(|_| self.node(function).map(|node| node.variable_id))
            .transpose()?
        else {
            return Ok(-1);
        };

        Ok(self.variable_index(variable_id).unwrap_or(usize::MAX) as isize)
    }

    pub fn if_id(&self, function: Bdd) -> Result<isize, BddError> {
        self.ensure_handle(function)?;

        match function.node_id() {
            Some(_) => Ok(self.node(function)?.variable_id as isize),
            None => Ok(-1),
        }
    }

    pub fn then_branch(&mut self, function: Bdd) -> Result<Bdd, BddError> {
        self.ensure_handle(function)?;

        if function.is_constant() {
            return Ok(function);
        }

        let branch = self.then_branch_raw(function)?;
        self.increfs(branch)?;
        Ok(branch)
    }

    pub fn else_branch(&mut self, function: Bdd) -> Result<Bdd, BddError> {
        self.ensure_handle(function)?;

        if function.is_constant() {
            return Ok(function);
        }

        let branch = self.else_branch_raw(function)?;
        self.increfs(branch)?;
        Ok(branch)
    }

    pub fn intersects(&mut self, f: Bdd, g: Bdd) -> Result<Bdd, BddError> {
        self.ensure_handle(f)?;
        self.ensure_handle(g)?;

        let result = self.intersects_step(f, g)?;
        self.make_external(result)
    }

    pub fn implies_counterexample(&mut self, f: Bdd, g: Bdd) -> Result<Bdd, BddError> {
        self.intersects(f, g.not())
    }

    pub fn implies(&mut self, f: Bdd, g: Bdd) -> Result<bool, BddError> {
        Ok(self.implies_counterexample(f, g)? == self.zero())
    }

    pub fn bdd_type(&self, function: Bdd) -> Result<BddType, BddError> {
        self.ensure_handle(function)?;

        match function {
            Bdd::Constant(false) => Ok(BddType::Zero),
            Bdd::Constant(true) => Ok(BddType::One),
            Bdd::Node { .. } => {
                let high = self.then_branch_raw(function)?;
                let low = self.else_branch_raw(function)?;

                if high == self.one() && low == self.zero() {
                    return Ok(BddType::PositiveVariable);
                }

                if high == self.zero() && low == self.one() {
                    return Ok(BddType::NegativeVariable);
                }

                Ok(BddType::NonTerminal)
            }
        }
    }

    pub fn unfree(&mut self, function: Bdd) -> Result<(), BddError> {
        self.increfs(function)
    }

    pub fn free(&mut self, function: Bdd) -> Result<(), BddError> {
        self.ensure_handle(function)?;

        let Some(node_id) = function.node_id() else {
            return Ok(());
        };

        let node = self
            .nodes
            .get_mut(node_id)
            .and_then(Option::as_mut)
            .ok_or(BddError::InvalidHandle(function))?;

        if node.refs == 0 {
            return Err(BddError::ReferenceUnderflow(function));
        }

        if node.refs < MAX_REFS {
            node.refs -= 1;
        }

        Ok(())
    }

    pub fn vars(&self) -> usize {
        self.variable_order.len()
    }

    pub fn total_size(&self) -> usize {
        self.nodes.iter().filter(|node| node.is_some()).count()
    }

    pub fn cache_ratio(&mut self, new_ratio: usize) -> usize {
        let old_ratio = self.op_cache.cache_ratio;
        self.op_cache.cache_ratio = new_ratio.clamp(1, 32);
        old_ratio
    }

    pub fn node_limit(&mut self, new_limit: usize) -> usize {
        let old_limit = self.node_limit;
        self.node_limit = new_limit;
        old_limit
    }

    pub fn overflow(&mut self) -> bool {
        let result = self.overflow;
        self.overflow = false;
        result
    }

    pub fn set_overflow_handler<F>(&mut self, overflow_fn: Option<F>)
    where
        F: FnMut() + 'static,
    {
        self.overflow_fn = overflow_fn.map(|callback| Box::new(callback) as Box<dyn FnMut()>);
    }

    pub fn set_abort_handler<F>(&mut self, abort_fn: Option<F>)
    where
        F: FnMut() + 'static,
    {
        self.abort_fn = abort_fn.map(|callback| Box::new(callback) as Box<dyn FnMut()>);
    }

    pub fn stats(&self) -> BddStats {
        BddStats {
            approximate_bytes_used: self.total_size() * std::mem::size_of::<BddNode>()
                + self.op_cache.ite.len() * std::mem::size_of::<((Bdd, Bdd, Bdd), Bdd)>(),
            node_count: self.total_size(),
            node_limit: self.node_limit,
            overflow: self.overflow,
            cache_entries: self.op_cache.ite.len(),
            cache_lookups: self.op_cache.lookups,
            cache_hits: self.op_cache.hits,
            cache_insertions: self.op_cache.insertions,
            variable_count: self.vars(),
        }
    }

    pub fn variable_block(&self) -> &VariableBlock {
        &self.super_block
    }

    fn new_var(&mut self, index: usize) -> Result<Bdd, BddError> {
        if self.variable_order.len() == usize::MAX {
            return Err(BddError::NoMoreIndexes);
        }

        if index > self.variable_order.len() {
            return Err(BddError::IndexOutOfRange {
                index,
                variables: self.variable_order.len(),
            });
        }

        if self.variable_order.len() == self.max_variables {
            self.max_variables = self.max_variables.saturating_mul(2).max(1);
        }

        let variable_id = self.next_variable_id;
        self.next_variable_id += 1;
        self.variable_order.insert(index, variable_id);

        if self.variables_by_id.len() <= variable_id {
            self.variables_by_id.resize(variable_id + 1, None);
        }

        let variable = self.find(variable_id, self.one(), self.zero())?;
        self.node_mut(variable)?.refs = MAX_REFS;
        self.variables_by_id[variable_id] = Some(variable);
        self.super_block.shift_for_insert(index as isize);

        Ok(variable)
    }

    fn ite_step(&mut self, mut f: Bdd, mut g: Bdd, mut h: Bdd) -> Result<Bdd, BddError> {
        if let Bdd::Constant(value) = f {
            return Ok(if value { g } else { h });
        }

        if g == h {
            return Ok(g);
        }

        if same_or_negations(f, g) {
            g = if f == g { self.one() } else { self.zero() };
        }

        if same_or_negations(f, h) {
            h = if f == h { self.zero() } else { self.one() };
        }

        match (g, h) {
            (Bdd::Constant(g_value), Bdd::Constant(h_value)) if g_value == h_value => Ok(g),
            (Bdd::Constant(true), Bdd::Constant(false)) => Ok(f),
            (Bdd::Constant(false), Bdd::Constant(true)) => Ok(f.not()),
            (Bdd::Constant(false), _) => self.and_step(f.not(), h),
            (Bdd::Constant(true), _) => Ok(self.and_step(f.not(), h.not())?.not()),
            (_, Bdd::Constant(false)) => self.and_step(f, g),
            (_, Bdd::Constant(true)) => Ok(self.and_step(f, g.not())?.not()),
            _ if same_or_negations(g, h) => {
                if g == h {
                    Ok(g)
                } else {
                    self.xnor_step(f, g)
                }
            }
            _ => {
                if !f.is_outpos() {
                    f = f.not();
                    std::mem::swap(&mut g, &mut h);
                }

                let outneg = !g.is_outpos();

                if outneg {
                    g = g.not();
                    h = h.not();
                }

                if let Some(result) = self.op_cache.lookup_ite(f, g, h) {
                    return Ok(if outneg { result.not() } else { result });
                }

                let top_variable = self.top_variable3(f, g, h)?;
                let (f_high, f_low) = self.cofactor(top_variable, f)?;
                let (g_high, g_low) = self.cofactor(top_variable, g)?;
                let (h_high, h_low) = self.cofactor(top_variable, h)?;
                let high = self.ite_step(f_high, g_high, h_high)?;
                let low = self.ite_step(f_low, g_low, h_low)?;
                let result = self.find(top_variable, high, low)?;
                self.op_cache.insert_ite(f, g, h, result);

                Ok(if outneg { result.not() } else { result })
            }
        }
    }

    fn and_step(&mut self, mut f: Bdd, mut g: Bdd) -> Result<Bdd, BddError> {
        if f == self.zero() || g == self.zero() {
            return Ok(self.zero());
        }

        if f == self.one() {
            return Ok(g);
        }

        if g == self.one() {
            return Ok(f);
        }

        if same_or_negations(f, g) {
            return Ok(if f == g { f } else { self.zero() });
        }

        if f > g {
            std::mem::swap(&mut f, &mut g);
        }

        if let Some(result) = self.op_cache.lookup_ite(f, g, self.zero()) {
            return Ok(result);
        }

        let top_variable = self.top_variable2(f, g)?;
        let (f_high, f_low) = self.cofactor(top_variable, f)?;
        let (g_high, g_low) = self.cofactor(top_variable, g)?;
        let high = self.and_step(f_high, g_high)?;
        let low = self.and_step(f_low, g_low)?;
        let result = self.find(top_variable, high, low)?;
        self.op_cache.insert_ite(f, g, self.zero(), result);
        Ok(result)
    }

    fn xnor_step(&mut self, mut f: Bdd, mut g: Bdd) -> Result<Bdd, BddError> {
        if f == self.one() {
            return Ok(g);
        }

        if f == self.zero() {
            return Ok(g.not());
        }

        if g == self.one() {
            return Ok(f);
        }

        if g == self.zero() {
            return Ok(f.not());
        }

        if same_or_negations(f, g) {
            return Ok(if f == g { self.one() } else { self.zero() });
        }

        if f > g {
            std::mem::swap(&mut f, &mut g);
        }

        let outneg = !g.is_outpos();

        if outneg {
            g = g.not();
        }

        if let Some(result) = self.op_cache.lookup_ite(f, g, g.not()) {
            return Ok(if outneg { result.not() } else { result });
        }

        let top_variable = self.top_variable2(f, g)?;
        let (f_high, f_low) = self.cofactor(top_variable, f)?;
        let (g_high, g_low) = self.cofactor(top_variable, g)?;
        let high = self.xnor_step(f_high, g_high)?;
        let low = self.xnor_step(f_low, g_low)?;
        let result = self.find(top_variable, high, low)?;
        self.op_cache.insert_ite(f, g, g.not(), result);

        Ok(if outneg { result.not() } else { result })
    }

    fn intersects_step(&mut self, mut f: Bdd, mut g: Bdd) -> Result<Bdd, BddError> {
        if f == self.zero() || g == self.zero() {
            return Ok(self.zero());
        }

        if f == self.one() {
            return Ok(g);
        }

        if g == self.one() {
            return Ok(f);
        }

        if same_or_negations(f, g) {
            return Ok(if f == g { f } else { self.zero() });
        }

        if f > g {
            std::mem::swap(&mut f, &mut g);
        }

        if let Some(result) = self.op_cache.lookup_ite(f, g, self.zero()) {
            return Ok(result);
        }

        let top_variable = self.top_variable2(f, g)?;
        let (f_high, f_low) = self.cofactor(top_variable, f)?;
        let (g_high, g_low) = self.cofactor(top_variable, g)?;
        let high = self.intersects_step(f_high, g_high)?;

        if high != self.zero() {
            return self.find(top_variable, high, self.zero());
        }

        let low = self.intersects_step(f_low, g_low)?;
        let result = self.find(top_variable, self.zero(), low)?;

        if result == self.zero() {
            self.op_cache.insert_ite(f, g, self.zero(), result);
        }

        Ok(result)
    }

    fn find(&mut self, variable_id: usize, high: Bdd, low: Bdd) -> Result<Bdd, BddError> {
        self.ensure_handle(high)?;
        self.ensure_handle(low)?;

        if high == low {
            return Ok(high);
        }

        let (stored_high, stored_low, complemented) = if high.is_outpos() {
            (high, low, false)
        } else {
            (high.not(), low.not(), true)
        };

        let key = NodeKey {
            variable_id,
            high: stored_high,
            low: stored_low,
        };

        let node_id = if let Some(node_id) = self.unique_table.get(&key) {
            *node_id
        } else {
            if self.node_limit != 0 && self.total_size() >= self.node_limit {
                self.overflow = true;

                if let Some(overflow_fn) = &mut self.overflow_fn {
                    overflow_fn();
                }
            }

            let node_id = self.nodes.len();
            self.nodes.push(Some(BddNode {
                variable_id,
                high: stored_high,
                low: stored_low,
                refs: 0,
                temp_refs: 0,
            }));
            self.unique_table.insert(key, node_id);
            node_id
        };

        let result = Bdd::Node {
            id: node_id,
            complemented,
        };
        self.temp_increfs(result)?;
        self.temp_decrefs_if_present(high);
        self.temp_decrefs_if_present(low);
        Ok(result)
    }

    fn top_variable2(&self, f: Bdd, g: Bdd) -> Result<usize, BddError> {
        let f_variable = self.node(f)?.variable_id;
        let g_variable = self.node(g)?.variable_id;

        Ok(
            if self.variable_index(f_variable) <= self.variable_index(g_variable) {
                f_variable
            } else {
                g_variable
            },
        )
    }

    fn top_variable3(&self, f: Bdd, g: Bdd, h: Bdd) -> Result<usize, BddError> {
        let mut variables = Vec::new();

        for reference in [f, g, h] {
            if !reference.is_constant() {
                variables.push(self.node(reference)?.variable_id);
            }
        }

        variables
            .into_iter()
            .min_by_key(|variable_id| self.variable_index(*variable_id).unwrap_or(usize::MAX))
            .ok_or(BddError::ConstantArgument(f))
    }

    fn cofactor(&self, variable_id: usize, function: Bdd) -> Result<(Bdd, Bdd), BddError> {
        if function.is_constant() {
            return Ok((function, function));
        }

        if self.node(function)?.variable_id == variable_id {
            Ok((
                self.then_branch_raw(function)?,
                self.else_branch_raw(function)?,
            ))
        } else {
            Ok((function, function))
        }
    }

    fn then_branch_raw(&self, function: Bdd) -> Result<Bdd, BddError> {
        let node = self.node(function)?;

        Ok(if function.is_outpos() {
            node.high
        } else {
            node.high.not()
        })
    }

    fn else_branch_raw(&self, function: Bdd) -> Result<Bdd, BddError> {
        let node = self.node(function)?;

        Ok(if function.is_outpos() {
            node.low
        } else {
            node.low.not()
        })
    }

    fn variable_index(&self, variable_id: usize) -> Option<usize> {
        self.variable_order
            .iter()
            .position(|candidate| *candidate == variable_id)
    }

    fn ensure_handle(&self, function: Bdd) -> Result<(), BddError> {
        match function {
            Bdd::Constant(_) => Ok(()),
            Bdd::Node { id, .. } => self
                .nodes
                .get(id)
                .and_then(Option::as_ref)
                .map(|_| ())
                .ok_or(BddError::InvalidHandle(function)),
        }
    }

    fn node(&self, function: Bdd) -> Result<&BddNode, BddError> {
        match function {
            Bdd::Constant(_) => Err(BddError::ConstantArgument(function)),
            Bdd::Node { id, .. } => self
                .nodes
                .get(id)
                .and_then(Option::as_ref)
                .ok_or(BddError::InvalidHandle(function)),
        }
    }

    fn node_mut(&mut self, function: Bdd) -> Result<&mut BddNode, BddError> {
        match function {
            Bdd::Constant(_) => Err(BddError::ConstantArgument(function)),
            Bdd::Node { id, .. } => self
                .nodes
                .get_mut(id)
                .and_then(Option::as_mut)
                .ok_or(BddError::InvalidHandle(function)),
        }
    }

    fn increfs(&mut self, function: Bdd) -> Result<(), BddError> {
        self.ensure_handle(function)?;

        if function.is_constant() {
            return Ok(());
        }

        let node = self.node_mut(function)?;

        if node.refs >= MAX_REFS - 1 {
            node.refs = MAX_REFS;
            node.temp_refs = 0;
        } else {
            node.refs += 1;
        }

        Ok(())
    }

    fn temp_increfs(&mut self, function: Bdd) -> Result<(), BddError> {
        if function.is_constant() {
            return Ok(());
        }

        let node = self.node_mut(function)?;

        if node.refs < MAX_REFS {
            node.temp_refs = node.temp_refs.saturating_add(1);
        }

        Ok(())
    }

    fn temp_decrefs_if_present(&mut self, function: Bdd) {
        if let Some(node_id) = function.node_id() {
            if let Some(Some(node)) = self.nodes.get_mut(node_id) {
                if node.refs < MAX_REFS {
                    node.temp_refs = node.temp_refs.saturating_sub(1);
                }
            }
        }
    }
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

fn same_or_negations(first: Bdd, second: Bdd) -> bool {
    match (first, second) {
        (Bdd::Constant(_), Bdd::Constant(_)) => true,
        (Bdd::Node { id: first_id, .. }, Bdd::Node { id: second_id, .. }) => first_id == second_id,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn constants_and_new_variables_match_basic_types() {
        let mut manager = BddManager::new();
        let x = manager.new_var_last().unwrap();

        assert_eq!(manager.one(), Bdd::Constant(true));
        assert_eq!(manager.zero(), Bdd::Constant(false));
        assert_eq!(manager.bdd_type(manager.one()), Ok(BddType::One));
        assert_eq!(manager.bdd_type(manager.zero()), Ok(BddType::Zero));
        assert_eq!(manager.bdd_type(x), Ok(BddType::PositiveVariable));
        assert_eq!(manager.bdd_type(x.not()), Ok(BddType::NegativeVariable));
    }

    #[test]
    fn variable_insertion_shifts_order_without_changing_ids() {
        let mut manager = BddManager::new();
        let x = manager.new_var_last().unwrap();
        let y = manager.new_var_last().unwrap();
        let z = manager.new_var_before(y).unwrap();

        assert_eq!(manager.vars(), 3);
        assert_eq!(manager.var_with_index(0), Some(x));
        assert_eq!(manager.var_with_index(1), Some(z));
        assert_eq!(manager.var_with_index(2), Some(y));
        assert_eq!(manager.var_with_id(0), Some(x));
        assert_eq!(manager.var_with_id(1), Some(y));
        assert_eq!(manager.var_with_id(2), Some(z));
    }

    #[test]
    fn boolean_operations_reduce_to_canonical_nodes() {
        let mut manager = BddManager::new();
        let x = manager.new_var_last().unwrap();
        let y = manager.new_var_last().unwrap();

        assert_eq!(manager.and(x, manager.one()).unwrap(), x);
        assert_eq!(manager.and(x, x.not()).unwrap(), manager.zero());
        assert_eq!(manager.or(x, manager.zero()).unwrap(), x);
        assert_eq!(manager.xor(x, x).unwrap(), manager.zero());
        assert_eq!(manager.xnor(x, x).unwrap(), manager.one());

        let xy = manager.and(x, y).unwrap();
        assert_eq!(manager.then_branch(xy).unwrap(), y);
        assert_eq!(manager.else_branch(xy).unwrap(), manager.zero());
        assert_eq!(manager.if_variable(xy).unwrap(), x);
    }

    #[test]
    fn ite_obeys_complemented_edge_semantics() {
        let mut manager = BddManager::new();
        let x = manager.new_var_last().unwrap();
        let y = manager.new_var_last().unwrap();

        let if_x_then_not_y_else_y = manager.ite(x, y.not(), y).unwrap();

        assert_eq!(
            manager.then_branch(if_x_then_not_y_else_y).unwrap(),
            y.not()
        );
        assert_eq!(manager.else_branch(if_x_then_not_y_else_y).unwrap(), y);
        assert_eq!(manager.xor(x, y).unwrap(), if_x_then_not_y_else_y);
    }

    #[test]
    fn intersects_returns_zero_or_a_subset_of_the_conjunction() {
        let mut manager = BddManager::new();
        let x = manager.new_var_last().unwrap();
        let y = manager.new_var_last().unwrap();
        let xy = manager.and(x, y).unwrap();

        assert_eq!(manager.intersects(x, x.not()).unwrap(), manager.zero());
        assert_ne!(manager.intersects(xy, y).unwrap(), manager.zero());
        assert_eq!(manager.implies(xy, x), Ok(true));
        assert_eq!(manager.implies(x, xy), Ok(false));
    }

    #[test]
    fn reference_management_reports_underflow_for_unowned_nodes() {
        let mut manager = BddManager::new();
        let x = manager.new_var_last().unwrap();
        let y = manager.new_var_last().unwrap();
        let xy = manager.and(x, y).unwrap();
        let xx = manager.identity(xy).unwrap();

        assert_eq!(xx, xy);
        assert!(manager.free(xy).is_ok());
        assert!(manager.free(xy).is_ok());
        assert_eq!(manager.free(xy), Err(BddError::ReferenceUnderflow(xy)));
    }

    #[test]
    fn cache_ratio_node_limit_and_overflow_follow_legacy_bounds() {
        let overflowed = Rc::new(RefCell::new(false));
        let overflowed_for_callback = Rc::clone(&overflowed);
        let mut manager = BddManager::with_capacity(2);
        manager.set_overflow_handler(Some(move || {
            *overflowed_for_callback.borrow_mut() = true;
        }));

        assert_eq!(manager.cache_ratio(0), 4);
        assert_eq!(manager.stats().cache_entries, 0);
        assert_eq!(manager.cache_ratio(40), 1);
        assert_eq!(manager.node_limit(1), 0);

        let x = manager.new_var_last().unwrap();
        let y = manager.new_var_last().unwrap();
        let _ = manager.and(x, y).unwrap();

        assert!(manager.overflow());
        assert!(*overflowed.borrow());
        assert!(!manager.overflow());
    }

    #[test]
    fn variable_blocks_binary_search_and_shift_like_the_c_source() {
        let mut block = VariableBlock::new(true, 0, 5);
        block.children.push(VariableBlock::new(false, 0, 1));
        block.children.push(VariableBlock::new(false, 3, 4));

        assert_eq!(block.find_block(0), 0);
        assert_eq!(block.find_block(2), 1);
        assert_eq!(block.find_block(5), 2);

        block.shift_indexes(2);

        assert_eq!(block.first_index(), 2);
        assert_eq!(block.children()[0].first_index(), 2);
        assert_eq!(block.children()[1].first_index(), 5);
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present() {
        let source = include_str!("bdd.rs");

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
