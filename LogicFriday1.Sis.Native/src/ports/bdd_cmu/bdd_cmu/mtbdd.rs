use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type MtbddValue = isize;
pub type MtbddVariableId = u32;
pub type MtbddNodeId = usize;

pub type CanonicalFn = Box<dyn Fn(MtbddValue, MtbddValue) -> bool>;
pub type TransformFn = Box<dyn Fn(MtbddValue, MtbddValue) -> (MtbddValue, MtbddValue)>;
pub type FreeTerminalFn = Box<dyn FnMut(MtbddValue, MtbddValue)>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MtbddEdge {
    node: MtbddNodeId,
    complemented: bool,
}

impl MtbddEdge {
    pub const fn regular(node: MtbddNodeId) -> Self {
        Self {
            node,
            complemented: false,
        }
    }

    pub const fn complemented(node: MtbddNodeId) -> Self {
        Self {
            node,
            complemented: true,
        }
    }

    pub const fn node(self) -> MtbddNodeId {
        self.node
    }

    pub const fn is_complemented(self) -> bool {
        self.complemented
    }

    pub const fn not(self) -> Self {
        Self {
            node: self.node,
            complemented: !self.complemented,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MtbddNode {
    Terminal(MtbddValue, MtbddValue),
    Branch {
        variable: MtbddVariableId,
        then_edge: MtbddEdge,
        else_edge: MtbddEdge,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MtbddStats {
    pub ite_calls: usize,
    pub ite_cache_hits: usize,
    pub ite_cache_inserts: usize,
    pub equal_calls: usize,
    pub equal_cache_hits: usize,
    pub equal_cache_inserts: usize,
    pub terminal_hits: usize,
    pub terminal_misses: usize,
    pub transformed_terminals: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct TerminalKey {
    first: MtbddValue,
    second: MtbddValue,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BranchKey {
    variable: MtbddVariableId,
    then_edge: MtbddEdge,
    else_edge: MtbddEdge,
}

pub struct MtbddManager {
    nodes: Vec<MtbddNode>,
    terminals: HashMap<TerminalKey, MtbddEdge>,
    branches: HashMap<BranchKey, MtbddEdge>,
    ite_cache: HashMap<(MtbddEdge, MtbddEdge, MtbddEdge), MtbddEdge>,
    equal_cache: HashMap<(MtbddEdge, MtbddEdge), MtbddEdge>,
    substitutions: HashMap<MtbddVariableId, MtbddVariableId>,
    canonical_fn: CanonicalFn,
    transform_fn: TransformFn,
    free_terminal_fn: Option<FreeTerminalFn>,
    stats: MtbddStats,
}

impl MtbddManager {
    pub fn new() -> Self {
        let mut manager = Self {
            nodes: Vec::new(),
            terminals: HashMap::new(),
            branches: HashMap::new(),
            ite_cache: HashMap::new(),
            equal_cache: HashMap::new(),
            substitutions: HashMap::new(),
            canonical_fn: Box::new(|_, _| false),
            transform_fn: Box::new(|first, second| (first, second)),
            free_terminal_fn: None,
            stats: MtbddStats::default(),
        };

        let one = manager.insert_terminal_unchecked(1, 0);
        debug_assert_eq!(one, manager.one());

        manager
    }

    pub fn one(&self) -> MtbddEdge {
        MtbddEdge::regular(0)
    }

    pub fn stats(&self) -> MtbddStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn node(&self, edge: MtbddEdge) -> Result<&MtbddNode, MtbddError> {
        self.nodes
            .get(edge.node)
            .ok_or(MtbddError::MissingNode(edge.node))
    }

    pub fn set_transform_closure<C, T>(&mut self, canonical_fn: C, transform_fn: T)
    where
        C: Fn(MtbddValue, MtbddValue) -> bool + 'static,
        T: Fn(MtbddValue, MtbddValue) -> (MtbddValue, MtbddValue) + 'static,
    {
        self.canonical_fn = Box::new(canonical_fn);
        self.transform_fn = Box::new(transform_fn);
    }

    pub fn set_one_data(
        &mut self,
        first: MtbddValue,
        second: MtbddValue,
    ) -> Result<(), MtbddError> {
        if self.terminal_count() != 1 || self.nodes.len() != 1 {
            return Err(MtbddError::TerminalsAlreadyExist);
        }

        self.terminals.clear();
        self.nodes[0] = MtbddNode::Terminal(first, second);
        self.terminals
            .insert(TerminalKey { first, second }, self.one());

        Ok(())
    }

    pub fn set_free_terminal_closure<F>(&mut self, free_terminal_fn: Option<F>)
    where
        F: FnMut(MtbddValue, MtbddValue) + 'static,
    {
        self.free_terminal_fn =
            free_terminal_fn.map(|function| Box::new(function) as FreeTerminalFn);
    }

    pub fn terminal(
        &mut self,
        first: MtbddValue,
        second: MtbddValue,
    ) -> Result<MtbddEdge, MtbddError> {
        let mut first = first;
        let mut second = second;
        let mut complemented = false;

        if (self.canonical_fn)(first, second) {
            let transformed = (self.transform_fn)(first, second);
            first = transformed.0;
            second = transformed.1;
            complemented = true;
            self.stats.transformed_terminals += 1;
        }

        let key = TerminalKey { first, second };
        if let Some(edge) = self.terminals.get(&key).copied() {
            self.stats.terminal_hits += 1;
            return Ok(if complemented { edge.not() } else { edge });
        }

        self.stats.terminal_misses += 1;
        let edge = self.insert_terminal_unchecked(first, second);

        Ok(if complemented { edge.not() } else { edge })
    }

    pub fn terminal_value(&self, edge: MtbddEdge) -> Result<(MtbddValue, MtbddValue), MtbddError> {
        match self.node(MtbddEdge::regular(edge.node))? {
            MtbddNode::Terminal(first, second) => {
                if edge.is_complemented() {
                    Ok((-*first, -*second))
                } else {
                    Ok((*first, *second))
                }
            }
            MtbddNode::Branch { .. } => Err(MtbddError::ExpectedTerminal(edge.node)),
        }
    }

    pub fn variable(&mut self, variable: MtbddVariableId) -> MtbddEdge {
        let one = self.one();
        let zero = self.terminal_zero();

        self.find_or_add_unchecked(variable, one, zero)
    }

    pub fn find_or_add(
        &mut self,
        variable: MtbddVariableId,
        then_edge: MtbddEdge,
        else_edge: MtbddEdge,
    ) -> Result<MtbddEdge, MtbddError> {
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.validate_order(variable, then_edge)?;
        self.validate_order(variable, else_edge)?;

        Ok(self.find_or_add_unchecked(variable, then_edge, else_edge))
    }

    pub fn ite(
        &mut self,
        condition: MtbddEdge,
        then_edge: MtbddEdge,
        else_edge: MtbddEdge,
    ) -> Result<MtbddEdge, MtbddError> {
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.ite_inner(condition, then_edge, else_edge)
    }

    pub fn substitute(
        &mut self,
        root: MtbddEdge,
        substitutions: &[(MtbddVariableId, MtbddVariableId)],
    ) -> Result<MtbddEdge, MtbddError> {
        self.validate_edge(root)?;
        self.substitutions.clear();
        self.substitutions.extend(substitutions.iter().copied());
        let result = self.substitute_inner(root);
        self.substitutions.clear();
        result
    }

    pub fn equal(&mut self, left: MtbddEdge, right: MtbddEdge) -> Result<MtbddEdge, MtbddError> {
        self.validate_edge(left)?;
        self.validate_edge(right)?;
        self.equal_inner(left, right)
    }

    pub fn eval(
        &self,
        root: MtbddEdge,
        assignment: &HashMap<MtbddVariableId, bool>,
    ) -> Result<(MtbddValue, MtbddValue), MtbddError> {
        let mut current = root;
        let mut complemented = false;

        loop {
            complemented ^= current.is_complemented();
            match self.node(MtbddEdge::regular(current.node))? {
                MtbddNode::Terminal(first, second) => {
                    if complemented {
                        return Ok((-*first, -*second));
                    }

                    return Ok((*first, *second));
                }
                MtbddNode::Branch {
                    variable,
                    then_edge,
                    else_edge,
                } => {
                    current = if assignment.get(variable).copied().unwrap_or(false) {
                        *then_edge
                    } else {
                        *else_edge
                    };
                }
            }
        }
    }

    fn ite_inner(
        &mut self,
        mut condition: MtbddEdge,
        mut then_edge: MtbddEdge,
        mut else_edge: MtbddEdge,
    ) -> Result<MtbddEdge, MtbddError> {
        self.stats.ite_calls += 1;

        if self.is_one(condition)? {
            return Ok(then_edge);
        }

        if self.is_zero(condition)? {
            return Ok(else_edge);
        }

        if then_edge == else_edge {
            return Ok(then_edge);
        }

        if condition.is_complemented() {
            condition = condition.not();
            std::mem::swap(&mut then_edge, &mut else_edge);
        }

        let cache_key = (condition, then_edge, else_edge);
        if let Some(cached) = self.ite_cache.get(&cache_key).copied() {
            self.stats.ite_cache_hits += 1;
            return Ok(cached);
        }

        let variable = self
            .sort_variable(condition)?
            .min(self.sort_variable(then_edge)?)
            .min(self.sort_variable(else_edge)?);
        let (condition_then, condition_else) = self.cofactor(condition, variable)?;
        let (then_then, then_else) = self.cofactor(then_edge, variable)?;
        let (else_then, else_else) = self.cofactor(else_edge, variable)?;
        let high = self.ite_inner(condition_then, then_then, else_then)?;
        let low = self.ite_inner(condition_else, then_else, else_else)?;
        let result = self.find_or_add_unchecked(variable, high, low);

        self.ite_cache.insert(cache_key, result);
        self.stats.ite_cache_inserts += 1;
        Ok(result)
    }

    fn substitute_inner(&mut self, edge: MtbddEdge) -> Result<MtbddEdge, MtbddError> {
        match self.node(MtbddEdge::regular(edge.node))?.clone() {
            MtbddNode::Terminal(_, _) => Ok(edge),
            MtbddNode::Branch {
                variable,
                then_edge,
                else_edge,
            } => {
                let high = self.substitute_inner(if edge.is_complemented() {
                    then_edge.not()
                } else {
                    then_edge
                })?;
                let low = self.substitute_inner(if edge.is_complemented() {
                    else_edge.not()
                } else {
                    else_edge
                })?;
                let replacement = self
                    .substitutions
                    .get(&variable)
                    .copied()
                    .unwrap_or(variable);
                let variable_edge = self.variable(replacement);

                self.ite_inner(variable_edge, high, low)
            }
        }
    }

    fn equal_inner(
        &mut self,
        mut left: MtbddEdge,
        mut right: MtbddEdge,
    ) -> Result<MtbddEdge, MtbddError> {
        self.stats.equal_calls += 1;

        if left == right {
            return Ok(self.one());
        }

        if self.is_terminal(left)? && self.is_terminal(right)? {
            return Ok(self.terminal_zero());
        }

        if edge_order_key(left) > edge_order_key(right) {
            std::mem::swap(&mut left, &mut right);
        }

        let cache_key = (left, right);
        if let Some(cached) = self.equal_cache.get(&cache_key).copied() {
            self.stats.equal_cache_hits += 1;
            return Ok(cached);
        }

        let variable = self.sort_variable(left)?.min(self.sort_variable(right)?);
        let (left_then, left_else) = self.cofactor(left, variable)?;
        let (right_then, right_else) = self.cofactor(right, variable)?;
        let high = self.equal_inner(left_then, right_then)?;
        let low = self.equal_inner(left_else, right_else)?;
        let result = self.find_or_add_unchecked(variable, high, low);

        self.equal_cache.insert(cache_key, result);
        self.stats.equal_cache_inserts += 1;
        Ok(result)
    }

    fn cofactor(
        &self,
        edge: MtbddEdge,
        variable: MtbddVariableId,
    ) -> Result<(MtbddEdge, MtbddEdge), MtbddError> {
        match self.node(MtbddEdge::regular(edge.node))? {
            MtbddNode::Branch {
                variable: node_variable,
                then_edge,
                else_edge,
            } if *node_variable == variable => {
                if edge.is_complemented() {
                    Ok((then_edge.not(), else_edge.not()))
                } else {
                    Ok((*then_edge, *else_edge))
                }
            }
            _ => Ok((edge, edge)),
        }
    }

    fn find_or_add_unchecked(
        &mut self,
        variable: MtbddVariableId,
        then_edge: MtbddEdge,
        else_edge: MtbddEdge,
    ) -> MtbddEdge {
        if then_edge == else_edge {
            return then_edge;
        }

        let key = BranchKey {
            variable,
            then_edge,
            else_edge,
        };
        if let Some(existing) = self.branches.get(&key).copied() {
            return existing;
        }

        let edge = MtbddEdge::regular(self.nodes.len());
        self.nodes.push(MtbddNode::Branch {
            variable,
            then_edge,
            else_edge,
        });
        self.branches.insert(key, edge);
        edge
    }

    fn insert_terminal_unchecked(&mut self, first: MtbddValue, second: MtbddValue) -> MtbddEdge {
        let edge = MtbddEdge::regular(self.nodes.len());
        self.nodes.push(MtbddNode::Terminal(first, second));
        self.terminals.insert(TerminalKey { first, second }, edge);
        edge
    }

    fn terminal_zero(&mut self) -> MtbddEdge {
        self.terminal(0, 0)
            .expect("zero terminal creation should not fail")
    }

    fn validate_edge(&self, edge: MtbddEdge) -> Result<(), MtbddError> {
        self.nodes
            .get(edge.node)
            .map(|_| ())
            .ok_or(MtbddError::MissingNode(edge.node))
    }

    fn validate_order(&self, parent: MtbddVariableId, child: MtbddEdge) -> Result<(), MtbddError> {
        let child_variable = self.sort_variable(child)?;
        if child_variable == MtbddVariableId::MAX || parent < child_variable {
            Ok(())
        } else {
            Err(MtbddError::VariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn sort_variable(&self, edge: MtbddEdge) -> Result<MtbddVariableId, MtbddError> {
        match self.node(MtbddEdge::regular(edge.node))? {
            MtbddNode::Terminal(_, _) => Ok(MtbddVariableId::MAX),
            MtbddNode::Branch { variable, .. } => Ok(*variable),
        }
    }

    fn is_terminal(&self, edge: MtbddEdge) -> Result<bool, MtbddError> {
        Ok(matches!(
            self.node(MtbddEdge::regular(edge.node))?,
            MtbddNode::Terminal(_, _)
        ))
    }

    fn is_one(&self, edge: MtbddEdge) -> Result<bool, MtbddError> {
        match self.node(MtbddEdge::regular(edge.node))? {
            MtbddNode::Terminal(_, _) => self.terminal_value(edge).map(|value| value == (1, 0)),
            MtbddNode::Branch { .. } => Ok(false),
        }
    }

    fn is_zero(&self, edge: MtbddEdge) -> Result<bool, MtbddError> {
        match self.node(MtbddEdge::regular(edge.node))? {
            MtbddNode::Terminal(_, _) => self.terminal_value(edge).map(|value| value == (0, 0)),
            MtbddNode::Branch { .. } => Ok(false),
        }
    }

    fn terminal_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| matches!(node, MtbddNode::Terminal(_, _)))
            .count()
    }
}

impl Default for MtbddManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MtbddManager {
    fn drop(&mut self) {
        if let Some(free_terminal_fn) = self.free_terminal_fn.as_mut() {
            for node in &self.nodes {
                if let MtbddNode::Terminal(first, second) = node {
                    free_terminal_fn(*first, *second);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MtbddError {
    MissingNode(MtbddNodeId),
    ExpectedTerminal(MtbddNodeId),
    TerminalsAlreadyExist,
    VariableOrder {
        parent: MtbddVariableId,
        child: MtbddVariableId,
    },
}

impl fmt::Display for MtbddError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "MTBDD node {node} is not present"),
            Self::ExpectedTerminal(node) => {
                write!(formatter, "MTBDD node {node} is not a terminal node")
            }
            Self::TerminalsAlreadyExist => {
                write!(formatter, "mtcmu_bdd_one_data: other terminal nodes already exist")
            }
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "MTBDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
        }
    }
}

impl Error for MtbddError {}

fn edge_order_key(edge: MtbddEdge) -> (MtbddNodeId, bool) {
    (edge.node, edge.complemented)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn values(entries: &[(MtbddVariableId, bool)]) -> HashMap<MtbddVariableId, bool> {
        entries.iter().copied().collect()
    }

    #[test]
    fn sets_one_data_before_other_terminals_are_created() {
        let mut manager = MtbddManager::new();

        manager.set_one_data(7, 9).unwrap();

        assert_eq!(manager.terminal_value(manager.one()).unwrap(), (7, 9));
        assert_eq!(manager.terminal(7, 9).unwrap(), manager.one());
    }

    #[test]
    fn rejects_one_data_after_terminal_creation() {
        let mut manager = MtbddManager::new();
        let _ = manager.terminal(2, 3).unwrap();

        let error = manager.set_one_data(7, 9).unwrap_err();

        assert_eq!(error, MtbddError::TerminalsAlreadyExist);
    }

    #[test]
    fn canonical_transform_returns_complemented_terminal() {
        let mut manager = MtbddManager::new();
        manager.set_transform_closure(|first, _| first < 0, |first, second| (-first, -second));

        let positive = manager.terminal(4, 5).unwrap();
        let negative = manager.terminal(-4, -5).unwrap();

        assert_eq!(negative, positive.not());
        assert_eq!(manager.terminal_value(negative).unwrap(), (-4, -5));
        assert_eq!(manager.stats().transformed_terminals, 1);
    }

    #[test]
    fn ite_selects_multi_terminal_branches() {
        let mut manager = MtbddManager::new();
        let condition = manager.variable(1);
        let first = manager.terminal(10, 20).unwrap();
        let second = manager.terminal(30, 40).unwrap();

        let result = manager.ite(condition, first, second).unwrap();

        assert_eq!(
            manager.eval(result, &values(&[(1, true)])).unwrap(),
            (10, 20)
        );
        assert_eq!(
            manager.eval(result, &values(&[(1, false)])).unwrap(),
            (30, 40)
        );
    }

    #[test]
    fn complemented_condition_swaps_ite_branches() {
        let mut manager = MtbddManager::new();
        let condition = manager.variable(1);
        let first = manager.terminal(10, 20).unwrap();
        let second = manager.terminal(30, 40).unwrap();

        let result = manager.ite(condition.not(), first, second).unwrap();

        assert_eq!(
            manager.eval(result, &values(&[(1, true)])).unwrap(),
            (30, 40)
        );
        assert_eq!(
            manager.eval(result, &values(&[(1, false)])).unwrap(),
            (10, 20)
        );
    }

    #[test]
    fn substitute_rebuilds_with_replacement_variables() {
        let mut manager = MtbddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);
        let first = manager.terminal(10, 20).unwrap();
        let second = manager.terminal(30, 40).unwrap();
        let expression = manager.ite(x, first, second).unwrap();

        let result = manager.substitute(expression, &[(1, 2)]).unwrap();

        assert_eq!(
            manager.eval(result, &values(&[(2, true)])).unwrap(),
            (10, 20)
        );
        assert_eq!(
            manager.eval(result, &values(&[(2, false)])).unwrap(),
            (30, 40)
        );
        assert_eq!(manager.eval(y, &values(&[(2, true)])).unwrap(), (1, 0));
    }

    #[test]
    fn equal_returns_boolean_bdd_for_matching_terminal_regions() {
        let mut manager = MtbddManager::new();
        let x = manager.variable(1);
        let a = manager.terminal(10, 20).unwrap();
        let b = manager.terminal(30, 40).unwrap();
        let left = manager.ite(x, a, b).unwrap();
        let right = manager.ite(x, a, a).unwrap();

        let equal = manager.equal(left, right).unwrap();

        assert_eq!(manager.eval(equal, &values(&[(1, true)])).unwrap(), (1, 0));
        assert_eq!(manager.eval(equal, &values(&[(1, false)])).unwrap(), (0, 0));
    }

    #[test]
    fn free_terminal_closure_runs_for_terminal_nodes() {
        let freed = Rc::new(RefCell::new(Vec::new()));
        {
            let mut manager = MtbddManager::new();
            let freed_clone = Rc::clone(&freed);
            manager.set_free_terminal_closure(Some(move |first, second| {
                freed_clone.borrow_mut().push((first, second));
            }));
            let _ = manager.terminal(2, 3).unwrap();
        }

        assert_eq!(&*freed.borrow(), &[(1, 0), (2, 3)]);
    }

    #[test]
    fn rejects_invalid_edges_and_ordering() {
        let mut manager = MtbddManager::new();
        let x = manager.variable(1);

        assert_eq!(
            manager.terminal_value(MtbddEdge::regular(999)),
            Err(MtbddError::MissingNode(999))
        );
        assert_eq!(
            manager.find_or_add(1, x, manager.one()),
            Err(MtbddError::VariableOrder {
                parent: 1,
                child: 1
            })
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("mtbdd.rs");
        let legacy_export = concat!("no", "_", "mangle");
        let tracking_prefix = concat!("REQUIRED", "_");
        let dependency_type = concat!("Port", "Dependency");
        let bead_token = concat!("bead", "_", "id");
        let source_token = concat!("source", "_", "file");
        let bead_prefix = concat!("Logic", "Friday1", "-", "8j8");

        assert!(!source.contains(legacy_export));
        assert!(!source.contains("extern \"C\""));
        assert!(!source.contains(tracking_prefix));
        assert!(!source.contains(dependency_type));
        assert!(!source.contains(bead_token));
        assert!(!source.contains(source_token));
        assert!(!source.contains(bead_prefix));
    }
}
