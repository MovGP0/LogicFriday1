//! Native Rust BDD quantification routines corresponding to the legacy CMU
//! `bddqnt.c` unit.
//!
//! The C code quantifies the variables in the current variable association by
//! recursively combining cofactors with OR for existential quantification. This
//! module keeps that behavior in an owned, reduced BDD manager without exposing
//! C ABI entry points.

use std::collections::{HashMap, HashSet};
use std::fmt;

const OP_QUANTIFY: i64 = 10_000;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddQuantificationError
{
    InvalidNode(BddId),
    InvalidVariable
    {
        variable: usize,
        variable_count: usize,
    },
}

impl fmt::Display for BddQuantificationError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::InvalidNode(node) => write!(formatter, "invalid BDD node {}", node.index()),
            Self::InvalidVariable {
                variable,
                variable_count,
            } => write!(
                formatter,
                "variable {variable} is outside the manager variable count {variable_count}"
            ),
        }
    }
}

impl std::error::Error for BddQuantificationError {}

pub type BddQuantificationResult<T> = Result<T, BddQuantificationError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableAssociation
{
    variables: HashSet<usize>,
    last: Option<usize>,
    id: Option<i64>,
}

impl VariableAssociation
{
    pub fn new(variables: impl IntoIterator<Item = usize>) -> Self
    {
        Self::with_id(variables, None)
    }

    pub fn with_id(variables: impl IntoIterator<Item = usize>, id: Option<i64>) -> Self
    {
        let variables: HashSet<_> = variables.into_iter().collect();
        let last = variables.iter().copied().max();

        Self {
            variables,
            last,
            id,
        }
    }

    pub fn contains(&self, variable: usize) -> bool
    {
        self.variables.contains(&variable)
    }

    pub const fn last(&self) -> Option<usize>
    {
        self.last
    }

    pub const fn id(&self) -> Option<i64>
    {
        self.id
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BinaryOperation
{
    And,
    Or,
}

#[derive(Clone, Debug)]
pub struct BddManager
{
    variable_count: usize,
    nodes: Vec<Option<BddNode>>,
    unique_table: HashMap<(usize, BddId, BddId), BddId>,
    unary_cache: HashMap<(i64, BddId), BddId>,
    binary_cache: HashMap<(BinaryOperation, BddId, BddId), BddId>,
    not_cache: HashMap<BddId, BddId>,
    current_association: VariableAssociation,
    next_temporary_operation: i64,
}

impl BddManager
{
    pub fn new(variable_count: usize) -> Self
    {
        Self {
            variable_count,
            nodes: vec![None, None],
            unique_table: HashMap::new(),
            unary_cache: HashMap::new(),
            binary_cache: HashMap::new(),
            not_cache: HashMap::new(),
            current_association: VariableAssociation::new([]),
            next_temporary_operation: -1,
        }
    }

    pub const fn variable_count(&self) -> usize
    {
        self.variable_count
    }

    pub const fn zero(&self) -> BddId
    {
        BddId::ZERO
    }

    pub const fn one(&self) -> BddId
    {
        BddId::ONE
    }

    pub fn set_current_association(&mut self, association: VariableAssociation)
    {
        self.current_association = association;
    }

    pub fn variable(&mut self, variable: usize) -> BddQuantificationResult<BddId>
    {
        self.find(variable, BddId::ONE, BddId::ZERO)
    }

    pub fn node(&self, id: BddId) -> BddQuantificationResult<Option<BddNode>>
    {
        if id == BddId::ZERO || id == BddId::ONE {
            return Ok(None);
        }

        self.nodes
            .get(id.0)
            .copied()
            .flatten()
            .map(Some)
            .ok_or(BddQuantificationError::InvalidNode(id))
    }

    pub fn find(
        &mut self,
        variable: usize,
        then_branch: BddId,
        else_branch: BddId,
    ) -> BddQuantificationResult<BddId>
    {
        if variable >= self.variable_count {
            return Err(BddQuantificationError::InvalidVariable {
                variable,
                variable_count: self.variable_count,
            });
        }

        self.require_node(then_branch)?;
        self.require_node(else_branch)?;

        if then_branch == else_branch {
            return Ok(then_branch);
        }

        let key = (variable, then_branch, else_branch);
        if let Some(id) = self.unique_table.get(&key).copied() {
            return Ok(id);
        }

        let id = BddId(self.nodes.len());
        self.nodes
            .push(Some(BddNode::new(variable, then_branch, else_branch)));
        self.unique_table.insert(key, id);
        Ok(id)
    }

    pub fn not(&mut self, root: BddId) -> BddQuantificationResult<BddId>
    {
        self.require_node(root)?;

        if root == BddId::ZERO {
            return Ok(BddId::ONE);
        }

        if root == BddId::ONE {
            return Ok(BddId::ZERO);
        }

        if let Some(result) = self.not_cache.get(&root).copied() {
            return Ok(result);
        }

        let node = self
            .node(root)?
            .expect("non-constant BDD node has a branch record");
        let then_branch = self.not(node.then_branch)?;
        let else_branch = self.not(node.else_branch)?;
        let result = self.find(node.variable, then_branch, else_branch)?;
        self.not_cache.insert(root, result);
        Ok(result)
    }

    pub fn and(&mut self, left: BddId, right: BddId) -> BddQuantificationResult<BddId>
    {
        self.apply(BinaryOperation::And, left, right)
    }

    pub fn or(&mut self, left: BddId, right: BddId) -> BddQuantificationResult<BddId>
    {
        self.apply(BinaryOperation::Or, left, right)
    }

    pub fn exists(&mut self, root: BddId) -> BddQuantificationResult<BddId>
    {
        self.require_node(root)?;

        let association = self.current_association.clone();
        let operation = association
            .id()
            .map_or_else(|| self.take_temporary_operation(), |id| OP_QUANTIFY + id);

        self.exists_step(root, operation, &association)
    }

    pub fn exists_temp(&mut self, root: BddId, operation: i64) -> BddQuantificationResult<BddId>
    {
        self.require_node(root)?;

        let association = self.current_association.clone();
        let operation = association.id().map_or(operation, |id| OP_QUANTIFY + id);

        self.exists_step(root, operation, &association)
    }

    pub fn forall(&mut self, root: BddId) -> BddQuantificationResult<BddId>
    {
        let complement = self.not(root)?;
        let existential = self.exists(complement)?;

        self.not(existential)
    }

    pub fn evaluate(
        &self,
        root: BddId,
        assignment: &[bool],
    ) -> BddQuantificationResult<bool>
    {
        self.require_node(root)?;

        match root {
            BddId::ZERO => Ok(false),
            BddId::ONE => Ok(true),
            _ => {
                let node = self
                    .node(root)?
                    .expect("non-constant BDD node has a branch record");
                let Some(value) = assignment.get(node.variable).copied() else {
                    return Err(BddQuantificationError::InvalidVariable {
                        variable: node.variable,
                        variable_count: assignment.len(),
                    });
                };

                if value {
                    self.evaluate(node.then_branch, assignment)
                } else {
                    self.evaluate(node.else_branch, assignment)
                }
            }
        }
    }

    fn exists_step(
        &mut self,
        root: BddId,
        operation: i64,
        association: &VariableAssociation,
    ) -> BddQuantificationResult<BddId>
    {
        let Some(node) = self.node(root)? else {
            return Ok(root);
        };

        if association.last().is_none_or(|last| node.variable > last) {
            return Ok(root);
        }

        if let Some(result) = self.unary_cache.get(&(operation, root)).copied() {
            return Ok(result);
        }

        let quantifying = association.contains(node.variable);
        let then_result = self.exists_step(node.then_branch, operation, association)?;
        let result = if quantifying && then_result == BddId::ONE {
            then_result
        } else {
            let else_result = self.exists_step(node.else_branch, operation, association)?;

            if quantifying {
                self.or(then_result, else_result)?
            } else {
                self.find(node.variable, then_result, else_result)?
            }
        };

        self.unary_cache.insert((operation, root), result);
        Ok(result)
    }

    fn apply(
        &mut self,
        operation: BinaryOperation,
        left: BddId,
        right: BddId,
    ) -> BddQuantificationResult<BddId>
    {
        self.require_node(left)?;
        self.require_node(right)?;

        let key = if left <= right {
            (operation, left, right)
        } else {
            (operation, right, left)
        };

        if let Some(result) = self.binary_cache.get(&key).copied() {
            return Ok(result);
        }

        let result = match (operation, left, right) {
            (BinaryOperation::And, BddId::ZERO, _) | (BinaryOperation::And, _, BddId::ZERO) => {
                BddId::ZERO
            }
            (BinaryOperation::And, BddId::ONE, other) | (BinaryOperation::And, other, BddId::ONE) => {
                other
            }
            (BinaryOperation::Or, BddId::ONE, _) | (BinaryOperation::Or, _, BddId::ONE) => {
                BddId::ONE
            }
            (BinaryOperation::Or, BddId::ZERO, other) | (BinaryOperation::Or, other, BddId::ZERO) => {
                other
            }
            _ => {
                let top_variable = self.top_variable(left, right)?;
                let (left_then, left_else) = self.cofactors(left, top_variable)?;
                let (right_then, right_else) = self.cofactors(right, top_variable)?;
                let then_result = self.apply(operation, left_then, right_then)?;
                let else_result = self.apply(operation, left_else, right_else)?;

                self.find(top_variable, then_result, else_result)?
            }
        };

        self.binary_cache.insert(key, result);
        Ok(result)
    }

    fn top_variable(&self, left: BddId, right: BddId) -> BddQuantificationResult<usize>
    {
        let left_variable = self.node(left)?.map(|node| node.variable);
        let right_variable = self.node(right)?.map(|node| node.variable);

        match (left_variable, right_variable) {
            (Some(left_variable), Some(right_variable)) => Ok(left_variable.min(right_variable)),
            (Some(variable), None) | (None, Some(variable)) => Ok(variable),
            (None, None) => unreachable!("constant pairs are handled before top_variable"),
        }
    }

    fn cofactors(
        &self,
        root: BddId,
        variable: usize,
    ) -> BddQuantificationResult<(BddId, BddId)>
    {
        let Some(node) = self.node(root)? else {
            return Ok((root, root));
        };

        if node.variable == variable {
            Ok((node.then_branch, node.else_branch))
        } else {
            Ok((root, root))
        }
    }

    fn require_node(&self, id: BddId) -> BddQuantificationResult<()>
    {
        if id == BddId::ZERO || id == BddId::ONE {
            return Ok(());
        }

        match self.nodes.get(id.0) {
            Some(Some(_)) => Ok(()),
            _ => Err(BddQuantificationError::InvalidNode(id)),
        }
    }

    fn take_temporary_operation(&mut self) -> i64
    {
        let operation = self.next_temporary_operation;
        self.next_temporary_operation -= 1;
        operation
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn assignment_without(variable: usize, assignment: &[bool]) -> Vec<Vec<bool>>
    {
        let mut false_assignment = assignment.to_vec();
        false_assignment[variable] = false;

        let mut true_assignment = assignment.to_vec();
        true_assignment[variable] = true;

        vec![false_assignment, true_assignment]
    }

    #[test]
    fn existential_quantification_ors_then_and_else_cofactors()
    {
        let mut manager = BddManager::new(2);
        let x0 = manager.variable(0).unwrap();
        let x1 = manager.variable(1).unwrap();
        let function = manager.and(x0, x1).unwrap();

        manager.set_current_association(VariableAssociation::new([0]));

        let result = manager.exists(function).unwrap();

        assert_eq!(result, x1);
    }

    #[test]
    fn existential_quantification_short_circuits_when_then_branch_is_one()
    {
        let mut manager = BddManager::new(2);
        let x1 = manager.variable(1).unwrap();
        let function = manager.find(0, manager.one(), x1).unwrap();

        manager.set_current_association(VariableAssociation::new([0]));

        assert_eq!(manager.exists(function).unwrap(), manager.one());
    }

    #[test]
    fn unquantified_variables_are_rebuilt_with_quantified_children()
    {
        let mut manager = BddManager::new(3);
        let x1 = manager.variable(1).unwrap();
        let x2 = manager.variable(2).unwrap();
        let quantified_child = manager.and(x1, x2).unwrap();
        let function = manager.find(0, quantified_child, manager.zero()).unwrap();

        manager.set_current_association(VariableAssociation::new([1]));

        let result = manager.exists(function).unwrap();
        let expected = manager.find(0, x2, manager.zero()).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn variables_after_association_last_are_left_unchanged()
    {
        let mut manager = BddManager::new(3);
        let x1 = manager.variable(1).unwrap();
        let x2 = manager.variable(2).unwrap();
        let function = manager.and(x1, x2).unwrap();

        manager.set_current_association(VariableAssociation::new([0]));

        assert_eq!(manager.exists(function).unwrap(), function);
    }

    #[test]
    fn universal_quantification_is_dual_of_existential_quantification()
    {
        let mut manager = BddManager::new(2);
        let x0 = manager.variable(0).unwrap();
        let x1 = manager.variable(1).unwrap();
        let function = manager.or(x0, x1).unwrap();

        manager.set_current_association(VariableAssociation::new([0]));

        assert_eq!(manager.forall(function).unwrap(), x1);
    }

    #[test]
    fn quantified_result_matches_truth_table_projection()
    {
        let mut manager = BddManager::new(3);
        let x0 = manager.variable(0).unwrap();
        let x1 = manager.variable(1).unwrap();
        let x2 = manager.variable(2).unwrap();
        let x0_and_x1 = manager.and(x0, x1).unwrap();
        let function = manager.or(x0_and_x1, x2).unwrap();

        manager.set_current_association(VariableAssociation::new([0]));

        let result = manager.exists(function).unwrap();

        for assignment in [
            [false, false, false],
            [false, true, false],
            [false, false, true],
            [false, true, true],
        ] {
            let expected = assignment_without(0, &assignment)
                .iter()
                .any(|candidate| manager.evaluate(function, candidate).unwrap());

            assert_eq!(manager.evaluate(result, &assignment).unwrap(), expected);
        }
    }

    #[test]
    fn invalid_node_is_rejected()
    {
        let mut manager = BddManager::new(1);

        assert_eq!(
            manager.exists(BddId(42)),
            Err(BddQuantificationError::InvalidNode(BddId(42)))
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present()
    {
        let text = include_str!("bddqnt.rs");

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
