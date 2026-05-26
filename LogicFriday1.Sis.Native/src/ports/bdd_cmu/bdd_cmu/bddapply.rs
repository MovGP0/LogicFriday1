//! Native Rust generic BDD apply routines.
//!
//! The legacy unit provides unary and binary recursive apply walkers. Each
//! walker lets a caller-supplied terminal callback simplify or finish the
//! current subproblem before the operation cache is consulted. This port keeps
//! that callback-first ordering and uses Rust-owned BDD handles.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type BddVariableId = u32;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddId(usize);

impl BddId
{
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);

    pub const fn index(self) -> usize
    {
        self.0
    }

    pub const fn is_zero(self) -> bool
    {
        self.0 == Self::ZERO.0
    }

    pub const fn is_one(self) -> bool
    {
        self.0 == Self::ONE.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddNode
{
    variable: BddVariableId,
    then_branch: BddId,
    else_branch: BddId,
}

impl BddNode
{
    pub const fn variable(self) -> BddVariableId
    {
        self.variable
    }

    pub const fn then_branch(self) -> BddId
    {
        self.then_branch
    }

    pub const fn else_branch(self) -> BddId
    {
        self.else_branch
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ApplyStats
{
    pub unary_calls: usize,
    pub binary_calls: usize,
    pub unary_cache_hits: usize,
    pub binary_cache_hits: usize,
    pub unary_cache_inserts: usize,
    pub binary_cache_inserts: usize,
    pub terminal_returns: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddApplyError
{
    MissingNode(BddId),
    VariableOrder
    {
        parent: BddVariableId,
        child: BddVariableId,
    },
}

impl fmt::Display for BddApplyError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingNode(node) => write!(formatter, "BDD node {} is not present", node.index()),
            Self::VariableOrder
            {
                parent,
                child,
            } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
        }
    }
}

impl Error for BddApplyError
{
}

pub type ApplyResult<T> = Result<T, BddApplyError>;

#[derive(Clone, Debug)]
pub struct BddManager
{
    nodes: Vec<BddNode>,
    unique_table: HashMap<BddNode, BddId>,
    unary_cache: HashMap<(i64, BddId), BddId>,
    binary_cache: HashMap<(i64, BddId, BddId), BddId>,
    next_temporary_operation: i64,
    stats: ApplyStats,
}

impl BddManager
{
    pub fn new() -> Self
    {
        Self
        {
            nodes: Vec::new(),
            unique_table: HashMap::new(),
            unary_cache: HashMap::new(),
            binary_cache: HashMap::new(),
            next_temporary_operation: -1,
            stats: ApplyStats::default(),
        }
    }

    pub const fn zero(&self) -> BddId
    {
        BddId::ZERO
    }

    pub const fn one(&self) -> BddId
    {
        BddId::ONE
    }

    pub fn stats(&self) -> ApplyStats
    {
        self.stats
    }

    pub fn node_count(&self) -> usize
    {
        self.nodes.len() + 2
    }

    pub fn variable(&mut self, variable: BddVariableId) -> BddId
    {
        self.find_unchecked(variable, BddId::ONE, BddId::ZERO)
    }

    pub fn node(&self, id: BddId) -> ApplyResult<Option<BddNode>>
    {
        if id.is_zero() || id.is_one()
        {
            return Ok(None);
        }

        self.nodes
            .get(id.0 - 2)
            .copied()
            .map(Some)
            .ok_or(BddApplyError::MissingNode(id))
    }

    pub fn find(
        &mut self,
        variable: BddVariableId,
        then_branch: BddId,
        else_branch: BddId,
    ) -> ApplyResult<BddId>
    {
        self.validate_handle(then_branch)?;
        self.validate_handle(else_branch)?;
        self.validate_order(variable, then_branch)?;
        self.validate_order(variable, else_branch)?;

        Ok(self.find_unchecked(variable, then_branch, else_branch))
    }

    pub fn apply_unary<E, F>(
        &mut self,
        root: BddId,
        env: &mut E,
        mut terminal: F,
    ) -> ApplyResult<BddId>
    where
        F: FnMut(&mut BddManager, &mut BddId, &mut E) -> ApplyResult<Option<BddId>>,
    {
        self.validate_handle(root)?;
        let operation = self.take_temporary_operation();

        self.apply_unary_step(operation, root, env, &mut terminal)
    }

    pub fn apply_binary<E, F>(
        &mut self,
        left: BddId,
        right: BddId,
        env: &mut E,
        mut terminal: F,
    ) -> ApplyResult<BddId>
    where
        F: FnMut(&mut BddManager, &mut BddId, &mut BddId, &mut E) -> ApplyResult<Option<BddId>>,
    {
        self.validate_handle(left)?;
        self.validate_handle(right)?;
        let operation = self.take_temporary_operation();

        self.apply_binary_step(operation, left, right, env, &mut terminal)
    }

    pub fn apply_unary_temp<E, F>(
        &mut self,
        operation: i64,
        root: BddId,
        env: &mut E,
        mut terminal: F,
    ) -> ApplyResult<BddId>
    where
        F: FnMut(&mut BddManager, &mut BddId, &mut E) -> ApplyResult<Option<BddId>>,
    {
        self.validate_handle(root)?;

        self.apply_unary_step(operation, root, env, &mut terminal)
    }

    pub fn apply_binary_temp<E, F>(
        &mut self,
        operation: i64,
        left: BddId,
        right: BddId,
        env: &mut E,
        mut terminal: F,
    ) -> ApplyResult<BddId>
    where
        F: FnMut(&mut BddManager, &mut BddId, &mut BddId, &mut E) -> ApplyResult<Option<BddId>>,
    {
        self.validate_handle(left)?;
        self.validate_handle(right)?;

        self.apply_binary_step(operation, left, right, env, &mut terminal)
    }

    pub fn evaluate(
        &self,
        root: BddId,
        assignment: &HashMap<BddVariableId, bool>,
    ) -> ApplyResult<bool>
    {
        self.validate_handle(root)?;

        let mut cursor = root;
        loop
        {
            if cursor.is_zero()
            {
                return Ok(false);
            }

            if cursor.is_one()
            {
                return Ok(true);
            }

            let node = self.node_unchecked(cursor);
            cursor = if assignment.get(&node.variable).copied().unwrap_or(false)
            {
                node.then_branch
            }
            else
            {
                node.else_branch
            };
        }
    }

    fn apply_unary_step<E, F>(
        &mut self,
        operation: i64,
        mut root: BddId,
        env: &mut E,
        terminal: &mut F,
    ) -> ApplyResult<BddId>
    where
        F: FnMut(&mut BddManager, &mut BddId, &mut E) -> ApplyResult<Option<BddId>>,
    {
        self.stats.unary_calls += 1;
        if let Some(result) = terminal(self, &mut root, env)?
        {
            self.validate_handle(result)?;
            self.stats.terminal_returns += 1;
            return Ok(result);
        }

        if let Some(result) = self.unary_cache.get(&(operation, root)).copied()
        {
            self.stats.unary_cache_hits += 1;
            return Ok(result);
        }

        let Some(node) = self.node(root)? else
        {
            return Err(BddApplyError::MissingNode(root));
        };
        let then_result = self.apply_unary_step(operation, node.then_branch, env, terminal)?;
        let else_result = self.apply_unary_step(operation, node.else_branch, env, terminal)?;
        let result = self.find_unchecked(node.variable, then_result, else_result);

        self.unary_cache.insert((operation, root), result);
        self.stats.unary_cache_inserts += 1;
        Ok(result)
    }

    fn apply_binary_step<E, F>(
        &mut self,
        operation: i64,
        mut left: BddId,
        mut right: BddId,
        env: &mut E,
        terminal: &mut F,
    ) -> ApplyResult<BddId>
    where
        F: FnMut(&mut BddManager, &mut BddId, &mut BddId, &mut E) -> ApplyResult<Option<BddId>>,
    {
        self.stats.binary_calls += 1;
        if let Some(result) = terminal(self, &mut left, &mut right, env)?
        {
            self.validate_handle(result)?;
            self.stats.terminal_returns += 1;
            return Ok(result);
        }

        if let Some(result) = self.binary_cache.get(&(operation, left, right)).copied()
        {
            self.stats.binary_cache_hits += 1;
            return Ok(result);
        }

        let variable = self.top_variable(left, right)?;
        let (left_then, left_else) = self.cofactors(left, variable)?;
        let (right_then, right_else) = self.cofactors(right, variable)?;
        let then_result =
            self.apply_binary_step(operation, left_then, right_then, env, terminal)?;
        let else_result =
            self.apply_binary_step(operation, left_else, right_else, env, terminal)?;
        let result = self.find_unchecked(variable, then_result, else_result);

        self.binary_cache.insert((operation, left, right), result);
        self.stats.binary_cache_inserts += 1;
        Ok(result)
    }

    fn top_variable(&self, left: BddId, right: BddId) -> ApplyResult<BddVariableId>
    {
        let left_variable = self.node(left)?.map(|node| node.variable);
        let right_variable = self.node(right)?.map(|node| node.variable);

        match (left_variable, right_variable)
        {
            (Some(left_variable), Some(right_variable)) => Ok(left_variable.min(right_variable)),
            (Some(variable), None) | (None, Some(variable)) => Ok(variable),
            (None, None) => Err(BddApplyError::MissingNode(left)),
        }
    }

    fn cofactors(&self, root: BddId, variable: BddVariableId) -> ApplyResult<(BddId, BddId)>
    {
        let Some(node) = self.node(root)? else
        {
            return Ok((root, root));
        };

        if node.variable == variable
        {
            Ok((node.then_branch, node.else_branch))
        }
        else
        {
            Ok((root, root))
        }
    }

    fn find_unchecked(
        &mut self,
        variable: BddVariableId,
        then_branch: BddId,
        else_branch: BddId,
    ) -> BddId
    {
        if then_branch == else_branch
        {
            return then_branch;
        }

        let node = BddNode
        {
            variable,
            then_branch,
            else_branch,
        };

        if let Some(existing) = self.unique_table.get(&node).copied()
        {
            return existing;
        }

        let id = BddId(self.nodes.len() + 2);
        self.nodes.push(node);
        self.unique_table.insert(node, id);
        id
    }

    fn validate_handle(&self, id: BddId) -> ApplyResult<()>
    {
        if id.is_zero() || id.is_one() || id.0 >= 2 && id.0 - 2 < self.nodes.len()
        {
            Ok(())
        }
        else
        {
            Err(BddApplyError::MissingNode(id))
        }
    }

    fn validate_order(&self, parent: BddVariableId, child: BddId) -> ApplyResult<()>
    {
        let Some(node) = self.node(child)? else
        {
            return Ok(());
        };

        if parent < node.variable
        {
            Ok(())
        }
        else
        {
            Err(BddApplyError::VariableOrder
            {
                parent,
                child: node.variable,
            })
        }
    }

    fn node_unchecked(&self, id: BddId) -> BddNode
    {
        self.nodes[id.0 - 2]
    }

    fn take_temporary_operation(&mut self) -> i64
    {
        let operation = self.next_temporary_operation;
        self.next_temporary_operation -= 1;
        operation
    }
}

impl Default for BddManager
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn values(entries: &[(BddVariableId, bool)]) -> HashMap<BddVariableId, bool>
    {
        entries.iter().copied().collect()
    }

    fn and_terminal(
        manager: &mut BddManager,
        left: &mut BddId,
        right: &mut BddId,
        calls: &mut usize,
    ) -> ApplyResult<Option<BddId>>
    {
        *calls += 1;

        if *left > *right
        {
            std::mem::swap(left, right);
        }

        if left.is_zero() || right.is_zero()
        {
            return Ok(Some(manager.zero()));
        }

        if left.is_one()
        {
            return Ok(Some(*right));
        }

        if right.is_one() || left == right
        {
            return Ok(Some(*left));
        }

        Ok(None)
    }

    fn not_terminal(
        manager: &mut BddManager,
        root: &mut BddId,
        calls: &mut usize,
    ) -> ApplyResult<Option<BddId>>
    {
        *calls += 1;

        if root.is_zero()
        {
            return Ok(Some(manager.one()));
        }

        if root.is_one()
        {
            return Ok(Some(manager.zero()));
        }

        Ok(None)
    }

    #[test]
    fn binary_apply_conjoins_by_recursing_over_top_variable()
    {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);
        let mut terminal_calls = 0;

        let result = manager
            .apply_binary(x, y, &mut terminal_calls, and_terminal)
            .unwrap();

        for x_value in [false, true]
        {
            for y_value in [false, true]
            {
                let assignment = values(&[(1, x_value), (2, y_value)]);
                assert_eq!(
                    manager.evaluate(result, &assignment).unwrap(),
                    x_value && y_value
                );
            }
        }
        assert!(terminal_calls > 0);
        assert!(manager.stats().binary_cache_inserts > 0);
    }

    #[test]
    fn binary_apply_terminal_callback_can_canonicalize_arguments_before_cache_lookup()
    {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);
        let x_and_y = manager
            .apply_binary_temp(42, x, y, &mut 0, and_terminal)
            .unwrap();

        let reversed = manager
            .apply_binary_temp(42, y, x, &mut 0, and_terminal)
            .unwrap();

        assert_eq!(reversed, x_and_y);
        assert!(manager.stats().binary_cache_hits > 0);
    }

    #[test]
    fn unary_apply_maps_terminal_results_and_rebuilds_reduced_nodes()
    {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);
        let x_and_y = manager.apply_binary(x, y, &mut 0, and_terminal).unwrap();

        let result = manager.apply_unary(x_and_y, &mut 0, not_terminal).unwrap();

        for x_value in [false, true]
        {
            for y_value in [false, true]
            {
                let assignment = values(&[(1, x_value), (2, y_value)]);
                assert_eq!(
                    manager.evaluate(result, &assignment).unwrap(),
                    !(x_value && y_value)
                );
            }
        }
        assert!(manager.stats().unary_cache_inserts > 0);
    }

    #[test]
    fn temporary_operation_ids_keep_independent_apply_calls_separate()
    {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);

        let first = manager.apply_binary(x, y, &mut 0, and_terminal).unwrap();
        let second = manager.apply_binary(x, y, &mut 0, and_terminal).unwrap();

        assert_eq!(first, second);
        assert_eq!(manager.stats().binary_cache_hits, 0);
    }

    #[test]
    fn invalid_handles_are_rejected()
    {
        let mut manager = BddManager::new();

        assert_eq!(
            manager.apply_unary(BddId(99), &mut 0, not_terminal),
            Err(BddApplyError::MissingNode(BddId(99)))
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens()
    {
        let source = include_str!("bddapply.rs");

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
