//! Native Rust model of the CMU BDD reduce and generalized cofactor routines.
//!
//! The legacy C implementation works over the CMU package's internal node and
//! cache tables. This port keeps the same recursive behavior over owned Rust
//! nodes: `reduce` agrees with a function on a care set and may shrink the
//! result, while `cofactor` computes the generalized cofactor under a non-empty
//! care set.

use std::collections::HashMap;
use std::fmt;

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

    pub const fn is_constant(self) -> bool
    {
        self.0 <= 1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddNode
{
    variable: usize,
    then_branch: BddId,
    else_branch: BddId,
}

impl BddNode
{
    pub const fn new(variable: usize, then_branch: BddId, else_branch: BddId) -> Self
    {
        Self {
            variable,
            then_branch,
            else_branch,
        }
    }

    pub const fn variable(self) -> usize
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum CachedOperation
{
    Reduce,
    Cofactor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddReduceError
{
    InvalidNode(BddId),
    InvalidVariable
    {
        variable: usize,
        variable_count: usize,
    },
    VariableOrder
    {
        parent: usize,
        child: usize,
    },
    EmptyCareSet,
}

impl fmt::Display for BddReduceError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::InvalidNode(node) => {
                write!(formatter, "BDD node {} is not present", node.index())
            }
            Self::InvalidVariable {
                variable,
                variable_count,
            } => write!(
                formatter,
                "BDD variable {variable} is outside the manager range 0..{variable_count}"
            ),
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} precedes child variable {child}"
            ),
            Self::EmptyCareSet => formatter.write_str("BDD cofactor care set must not be false"),
        }
    }
}

impl std::error::Error for BddReduceError {}

#[derive(Clone, Debug)]
pub struct BddManager
{
    variable_count: usize,
    nodes: Vec<Option<BddNode>>,
    unique: HashMap<(usize, BddId, BddId), BddId>,
    cache: HashMap<(CachedOperation, BddId, BddId), Option<BddId>>,
}

impl BddManager
{
    pub fn new(variable_count: usize) -> Self
    {
        Self {
            variable_count,
            nodes: vec![None, None],
            unique: HashMap::new(),
            cache: HashMap::new(),
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

    pub fn variable_count(&self) -> usize
    {
        self.variable_count
    }

    pub fn cache_len(&self) -> usize
    {
        self.cache.len()
    }

    pub fn variable(&mut self, variable: usize) -> Result<BddId, BddReduceError>
    {
        self.find_or_add(variable, self.one(), self.zero())
    }

    pub fn find_or_add(
        &mut self,
        variable: usize,
        then_branch: BddId,
        else_branch: BddId,
    ) -> Result<BddId, BddReduceError>
    {
        self.require_valid_branch(variable, then_branch)?;
        self.require_valid_branch(variable, else_branch)?;

        Ok(self.find_or_add_unchecked(variable, then_branch, else_branch))
    }

    pub fn node(&self, id: BddId) -> Result<Option<BddNode>, BddReduceError>
    {
        if id.is_constant() {
            return Ok(None);
        }

        self.nodes
            .get(id.0)
            .copied()
            .flatten()
            .map(Some)
            .ok_or(BddReduceError::InvalidNode(id))
    }

    pub fn reduce(&mut self, function: BddId, care: BddId) -> Result<BddId, BddReduceError>
    {
        self.require_node(function)?;
        self.require_node(care)?;
        self.cache.clear();

        Ok(self
            .reduce_step(function, care)?
            .unwrap_or_else(|| self.zero()))
    }

    pub fn cofactor(&mut self, function: BddId, care: BddId) -> Result<BddId, BddReduceError>
    {
        self.require_node(function)?;
        self.require_node(care)?;

        if care == self.zero() {
            return Err(BddReduceError::EmptyCareSet);
        }

        self.cache.clear();

        self.cofactor_step(function, care)?
            .ok_or(BddReduceError::EmptyCareSet)
    }

    pub fn evaluate(&self, root: BddId, assignment: &[bool]) -> Result<bool, BddReduceError>
    {
        match root {
            BddId::ZERO => Ok(false),
            BddId::ONE => Ok(true),
            _ => {
                let node = self
                    .node(root)?
                    .expect("non-constant node has an entry after validation");
                let value = assignment.get(node.variable).copied().ok_or(
                    BddReduceError::InvalidVariable {
                        variable: node.variable,
                        variable_count: assignment.len(),
                    },
                )?;

                self.evaluate(
                    if value {
                        node.then_branch
                    } else {
                        node.else_branch
                    },
                    assignment,
                )
            }
        }
    }

    fn reduce_step(&mut self, function: BddId, care: BddId)
    -> Result<Option<BddId>, BddReduceError>
    {
        if care == self.zero() {
            return Ok(None);
        }

        if care == self.one() || function.is_constant() {
            return Ok(Some(function));
        }

        let cache_key = (CachedOperation::Reduce, function, care);
        if let Some(cached) = self.cache.get(&cache_key).copied() {
            return Ok(cached);
        }

        let top_variable = self.top_variable(function, care)?;
        let (function_then, function_else) = self.cofactors(function, top_variable)?;
        let (care_then, care_else) = self.cofactors(care, top_variable)?;

        let result = if function == function_then {
            let reduced_care = self.ite(care_then, self.one(), care_else)?;
            self.reduce_step(function, reduced_care)?
        } else {
            let then_result = self.reduce_step(function_then, care_then)?;
            let else_result = self.reduce_step(function_else, care_else)?;

            match (then_result, else_result) {
                (None, None) => None,
                (Some(then_result), None) => Some(then_result),
                (None, Some(else_result)) => Some(else_result),
                (Some(then_result), Some(else_result)) => {
                    Some(self.find_or_add_unchecked(top_variable, then_result, else_result))
                }
            }
        };

        self.cache.insert(cache_key, result);
        Ok(result)
    }

    fn cofactor_step(
        &mut self,
        function: BddId,
        care: BddId,
    ) -> Result<Option<BddId>, BddReduceError>
    {
        if care == self.zero() {
            return Ok(None);
        }

        if care == self.one() || function.is_constant() {
            return Ok(Some(function));
        }

        let cache_key = (CachedOperation::Cofactor, function, care);
        if let Some(cached) = self.cache.get(&cache_key).copied() {
            return Ok(cached);
        }

        let top_variable = self.top_variable(function, care)?;
        let (function_then, function_else) = self.cofactors(function, top_variable)?;
        let (care_then, care_else) = self.cofactors(care, top_variable)?;
        let then_result = self.cofactor_step(function_then, care_then)?;
        let else_result = self.cofactor_step(function_else, care_else)?;

        let result = match (then_result, else_result) {
            (None, None) => None,
            (Some(then_result), None) => Some(then_result),
            (None, Some(else_result)) => Some(else_result),
            (Some(then_result), Some(else_result)) => {
                Some(self.find_or_add_unchecked(top_variable, then_result, else_result))
            }
        };

        self.cache.insert(cache_key, result);
        Ok(result)
    }

    fn ite(
        &mut self,
        condition: BddId,
        then_branch: BddId,
        else_branch: BddId,
    ) -> Result<BddId, BddReduceError>
    {
        if condition == self.one() {
            return Ok(then_branch);
        }

        if condition == self.zero() {
            return Ok(else_branch);
        }

        if then_branch == else_branch {
            return Ok(then_branch);
        }

        let top_variable = self.top_variable3(condition, then_branch, else_branch)?;
        let (condition_then, condition_else) = self.cofactors(condition, top_variable)?;
        let (then_then, then_else) = self.cofactors(then_branch, top_variable)?;
        let (else_then, else_else) = self.cofactors(else_branch, top_variable)?;
        let high = self.ite(condition_then, then_then, else_then)?;
        let low = self.ite(condition_else, then_else, else_else)?;

        Ok(self.find_or_add_unchecked(top_variable, high, low))
    }

    fn top_variable(&self, first: BddId, second: BddId) -> Result<usize, BddReduceError>
    {
        let first_variable = self.node(first)?.map(|node| node.variable);
        let second_variable = self.node(second)?.map(|node| node.variable);

        match (first_variable, second_variable) {
            (Some(first_variable), Some(second_variable)) => {
                Ok(first_variable.min(second_variable))
            }
            (Some(variable), None) | (None, Some(variable)) => Ok(variable),
            (None, None) => unreachable!("top_variable requires at least one non-constant input"),
        }
    }

    fn top_variable3(
        &self,
        first: BddId,
        second: BddId,
        third: BddId,
    ) -> Result<usize, BddReduceError>
    {
        [first, second, third]
            .into_iter()
            .filter_map(|id| self.node(id).transpose())
            .map(|node| node.map(|node| node.variable))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min()
            .ok_or(BddReduceError::InvalidNode(first))
    }

    fn cofactors(&self, id: BddId, variable: usize) -> Result<(BddId, BddId), BddReduceError>
    {
        let Some(node) = self.node(id)? else {
            return Ok((id, id));
        };

        if node.variable == variable {
            Ok((node.then_branch, node.else_branch))
        } else {
            Ok((id, id))
        }
    }

    fn find_or_add_unchecked(
        &mut self,
        variable: usize,
        then_branch: BddId,
        else_branch: BddId,
    ) -> BddId
    {
        if then_branch == else_branch {
            return then_branch;
        }

        let key = (variable, then_branch, else_branch);
        if let Some(id) = self.unique.get(&key).copied() {
            return id;
        }

        let id = BddId(self.nodes.len());
        self.nodes
            .push(Some(BddNode::new(variable, then_branch, else_branch)));
        self.unique.insert(key, id);
        id
    }

    fn require_valid_branch(&self, variable: usize, branch: BddId) -> Result<(), BddReduceError>
    {
        if variable >= self.variable_count {
            return Err(BddReduceError::InvalidVariable {
                variable,
                variable_count: self.variable_count,
            });
        }

        self.require_node(branch)?;

        if let Some(node) = self.node(branch)? {
            if node.variable <= variable {
                return Err(BddReduceError::VariableOrder {
                    parent: variable,
                    child: node.variable,
                });
            }
        }

        Ok(())
    }

    fn require_node(&self, id: BddId) -> Result<(), BddReduceError>
    {
        if id.is_constant() {
            return Ok(());
        }

        match self.nodes.get(id.0) {
            Some(Some(_)) => Ok(()),
            _ => Err(BddReduceError::InvalidNode(id)),
        }
    }
}

impl Default for BddManager
{
    fn default() -> Self
    {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn assignments(variable_count: usize) -> Vec<Vec<bool>>
    {
        (0..(1_usize << variable_count))
            .map(|bits| {
                (0..variable_count)
                    .map(|variable| ((bits >> variable) & 1) == 1)
                    .collect()
            })
            .collect()
    }

    #[test]
    fn reduce_with_true_care_returns_original_function()
    {
        let mut manager = BddManager::new(2);
        let x = manager.variable(0).unwrap();

        assert_eq!(manager.reduce(x, manager.one()).unwrap(), x);
    }

    #[test]
    fn reduce_with_false_care_returns_zero()
    {
        let mut manager = BddManager::new(2);
        let x = manager.variable(0).unwrap();

        assert_eq!(manager.reduce(x, manager.zero()).unwrap(), manager.zero());
    }

    #[test]
    fn reduce_preserves_function_on_care_set()
    {
        let mut manager = BddManager::new(3);
        let x = manager.variable(0).unwrap();
        let y = manager.variable(1).unwrap();
        let z = manager.variable(2).unwrap();
        let function = manager.ite(x, y, z).unwrap();
        let care = manager.ite(x, manager.one(), z).unwrap();
        let reduced = manager.reduce(function, care).unwrap();

        for assignment in assignments(manager.variable_count()) {
            if manager.evaluate(care, &assignment).unwrap() {
                assert_eq!(
                    manager.evaluate(reduced, &assignment).unwrap(),
                    manager.evaluate(function, &assignment).unwrap()
                );
            }
        }
    }

    #[test]
    fn reduce_can_eliminate_variables_outside_the_care_set()
    {
        let mut manager = BddManager::new(2);
        let x = manager.variable(0).unwrap();
        let y = manager.variable(1).unwrap();
        let function = manager.ite(x, manager.one(), y).unwrap();

        assert_eq!(manager.reduce(function, x).unwrap(), manager.one());
    }

    #[test]
    fn cofactor_with_true_care_returns_original_function()
    {
        let mut manager = BddManager::new(2);
        let x = manager.variable(0).unwrap();

        assert_eq!(manager.cofactor(x, manager.one()).unwrap(), x);
    }

    #[test]
    fn cofactor_restricts_positive_literal_care()
    {
        let mut manager = BddManager::new(2);
        let x = manager.variable(0).unwrap();
        let y = manager.variable(1).unwrap();
        let function = manager.ite(x, y, manager.zero()).unwrap();

        assert_eq!(manager.cofactor(function, x).unwrap(), y);
    }

    #[test]
    fn cofactor_rejects_empty_care_set()
    {
        let mut manager = BddManager::new(1);
        let x = manager.variable(0).unwrap();

        assert_eq!(
            manager.cofactor(x, manager.zero()).unwrap_err(),
            BddReduceError::EmptyCareSet
        );
    }

    #[test]
    fn unique_table_reuses_nodes_and_skips_equal_children()
    {
        let mut manager = BddManager::new(2);
        let y = manager.variable(1).unwrap();

        assert_eq!(manager.find_or_add(0, y, y).unwrap(), y);
        assert_eq!(
            manager.find_or_add(0, y, manager.zero()).unwrap(),
            manager.find_or_add(0, y, manager.zero()).unwrap()
        );
    }

    #[test]
    fn invalid_variable_order_is_rejected()
    {
        let mut manager = BddManager::new(2);
        let x = manager.variable(0).unwrap();

        assert_eq!(
            manager.find_or_add(1, x, manager.zero()).unwrap_err(),
            BddReduceError::VariableOrder {
                parent: 1,
                child: 0
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present()
    {
        let text = include_str!("bddreduce.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
