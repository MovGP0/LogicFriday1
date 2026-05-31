//! Native Rust support-set helpers for the CMU BDD package.
//!
//! The original implementation used mutable node marks while walking a tagged
//! pointer graph. This port keeps the same traversal behavior with explicit
//! handles and per-call visited sets.

use std::collections::BTreeSet;
use std::fmt;

pub type BddIndex = u16;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BddRef {
    Constant(bool),
    Node { id: usize, complemented: bool },
}

impl BddRef {
    pub const fn is_constant(self) -> bool {
        matches!(self, Self::Constant(_))
    }

    pub const fn not(self) -> Self {
        match self {
            Self::Constant(value) => Self::Constant(!value),
            Self::Node { id, complemented } => Self::Node {
                id,
                complemented: !complemented,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddType {
    NonTerminal,
    Zero,
    One,
    PositiveVariable,
    NegativeVariable,
    Constant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupportError {
    UnknownNode(BddRef),
    VariableExpected(BddRef),
    VariableIndexNotFound(BddIndex),
}

impl fmt::Display for SupportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(reference) => write!(formatter, "unknown BDD node {reference:?}"),
            Self::VariableExpected(reference) => {
                write!(formatter, "expected a positive variable, got {reference:?}")
            }
            Self::VariableIndexNotFound(index) => {
                write!(formatter, "variable index {index} is not registered")
            }
        }
    }
}

impl std::error::Error for SupportError {}

#[derive(Clone, Debug)]
struct BddNode {
    index: BddIndex,
    high: BddRef,
    low: BddRef,
}

#[derive(Clone, Debug, Default)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    variables_by_index: Vec<Option<BddRef>>,
}

impl BddManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn zero(&self) -> BddRef {
        BddRef::Constant(false)
    }

    pub const fn one(&self) -> BddRef {
        BddRef::Constant(true)
    }

    pub fn new_variable(&mut self, index: BddIndex) -> BddRef {
        let variable = self.new_node(index, self.one(), self.zero());
        let slot = index as usize;

        if self.variables_by_index.len() <= slot {
            self.variables_by_index.resize(slot + 1, None);
        }

        self.variables_by_index[slot] = Some(variable);
        variable
    }

    pub fn new_node(&mut self, index: BddIndex, high: BddRef, low: BddRef) -> BddRef {
        let id = self.nodes.len();
        self.nodes.push(BddNode { index, high, low });

        BddRef::Node {
            id,
            complemented: false,
        }
    }

    pub fn variable_for_index(&self, index: BddIndex) -> Result<BddRef, SupportError> {
        self.variables_by_index
            .get(index as usize)
            .and_then(|variable| *variable)
            .ok_or(SupportError::VariableIndexNotFound(index))
    }

    pub fn bdd_type(&self, reference: BddRef) -> Result<BddType, SupportError> {
        match reference {
            BddRef::Constant(false) => Ok(BddType::Zero),
            BddRef::Constant(true) => Ok(BddType::One),
            BddRef::Node { .. } => {
                let high = self.then_branch(reference)?;
                let low = self.else_branch(reference)?;

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

    pub fn depends_on(&self, function: BddRef, variable: BddRef) -> Result<bool, SupportError> {
        let BddType::PositiveVariable = self.bdd_type(variable)? else {
            if variable.is_constant() {
                return Ok(true);
            }

            return Err(SupportError::VariableExpected(variable));
        };

        let variable_index = self.index(variable)?;
        let mut visited = BTreeSet::new();

        self.depends_on_step(function, variable_index, &mut visited)
    }

    pub fn support(&self, function: BddRef) -> Result<Vec<BddRef>, SupportError> {
        let mut support = Vec::new();
        let mut visited_nodes = BTreeSet::new();
        let mut visited_variables = BTreeSet::new();

        self.support_step(
            function,
            &mut support,
            &mut visited_nodes,
            &mut visited_variables,
        )?;

        Ok(support)
    }

    fn depends_on_step(
        &self,
        function: BddRef,
        variable_index: BddIndex,
        visited: &mut BTreeSet<usize>,
    ) -> Result<bool, SupportError> {
        if function.is_constant() {
            return Ok(false);
        }

        let function_index = self.index(function)?;

        if function_index > variable_index {
            return Ok(false);
        }

        if function_index == variable_index {
            return Ok(true);
        }

        let node_id = self.node_id(function)?;

        if !visited.insert(node_id) {
            return Ok(false);
        }

        if self.depends_on_step(self.then_branch(function)?, variable_index, visited)? {
            return Ok(true);
        }

        self.depends_on_step(self.else_branch(function)?, variable_index, visited)
    }

    fn support_step(
        &self,
        function: BddRef,
        support: &mut Vec<BddRef>,
        visited_nodes: &mut BTreeSet<usize>,
        visited_variables: &mut BTreeSet<BddIndex>,
    ) -> Result<(), SupportError> {
        if function.is_constant() {
            return Ok(());
        }

        let node_id = self.node_id(function)?;

        if !visited_nodes.insert(node_id) {
            return Ok(());
        }

        let index = self.index(function)?;

        if visited_variables.insert(index) {
            support.push(self.variable_for_index(index)?);
        }

        self.support_step(
            self.then_branch(function)?,
            support,
            visited_nodes,
            visited_variables,
        )?;
        self.support_step(
            self.else_branch(function)?,
            support,
            visited_nodes,
            visited_variables,
        )
    }

    fn then_branch(&self, reference: BddRef) -> Result<BddRef, SupportError> {
        let node = self.node(reference)?;
        let branch = node.high;

        if self.is_complemented(reference) {
            Ok(branch.not())
        } else {
            Ok(branch)
        }
    }

    fn else_branch(&self, reference: BddRef) -> Result<BddRef, SupportError> {
        let node = self.node(reference)?;
        let branch = node.low;

        if self.is_complemented(reference) {
            Ok(branch.not())
        } else {
            Ok(branch)
        }
    }

    fn index(&self, reference: BddRef) -> Result<BddIndex, SupportError> {
        Ok(self.node(reference)?.index)
    }

    fn node_id(&self, reference: BddRef) -> Result<usize, SupportError> {
        match reference {
            BddRef::Node { id, .. } => Ok(id),
            BddRef::Constant(_) => Err(SupportError::UnknownNode(reference)),
        }
    }

    fn node(&self, reference: BddRef) -> Result<&BddNode, SupportError> {
        match reference {
            BddRef::Node { id, .. } => self
                .nodes
                .get(id)
                .ok_or(SupportError::UnknownNode(reference)),
            BddRef::Constant(_) => Err(SupportError::UnknownNode(reference)),
        }
    }

    const fn is_complemented(&self, reference: BddRef) -> bool {
        match reference {
            BddRef::Node { complemented, .. } => complemented,
            BddRef::Constant(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_dependency_when_variable_is_reached() {
        let mut manager = BddManager::new();
        let x = manager.new_variable(0);
        let y = manager.new_variable(1);
        let function = manager.new_node(0, y, manager.zero());

        assert_eq!(manager.depends_on(function, x), Ok(true));
        assert_eq!(manager.depends_on(function, y), Ok(true));
    }

    #[test]
    fn prunes_branches_below_requested_variable_index() {
        let mut manager = BddManager::new();
        let x = manager.new_variable(0);
        let y = manager.new_variable(1);
        let z = manager.new_variable(2);
        let function = manager.new_node(0, y, manager.zero());

        assert_eq!(manager.depends_on(function, z), Ok(false));
        assert_eq!(manager.support(function), Ok(vec![x, y]));
    }

    #[test]
    fn shared_nodes_are_visited_once_when_collecting_support() {
        let mut manager = BddManager::new();
        let x = manager.new_variable(0);
        let y = manager.new_variable(1);
        let shared = manager.new_node(1, manager.one(), manager.zero());
        let function = manager.new_node(0, shared, shared.not());

        assert_eq!(manager.support(function), Ok(vec![x, y]));
    }

    #[test]
    fn complemented_references_flip_branches_like_legacy_tagged_edges() {
        let mut manager = BddManager::new();
        let x = manager.new_variable(0);

        assert_eq!(manager.bdd_type(x), Ok(BddType::PositiveVariable));
        assert_eq!(manager.bdd_type(x.not()), Ok(BddType::NegativeVariable));
        assert_eq!(manager.depends_on(x.not(), x), Ok(true));
    }

    #[test]
    fn constants_have_empty_support_and_match_legacy_depends_on_result() {
        let manager = BddManager::new();

        assert_eq!(manager.support(manager.one()), Ok(Vec::new()));
        assert_eq!(manager.depends_on(manager.zero(), manager.one()), Ok(true));
    }

    #[test]
    fn non_positive_variable_argument_is_rejected() {
        let mut manager = BddManager::new();
        let x = manager.new_variable(0);
        let y = manager.new_variable(1);
        let function = manager.new_node(0, y, manager.zero());

        assert_eq!(
            manager.depends_on(function, x.not()),
            Err(SupportError::VariableExpected(x.not()))
        );
    }

    #[test]
    fn support_reports_missing_variable_registration() {
        let mut manager = BddManager::new();
        let function = manager.new_node(3, manager.one(), manager.zero());

        assert_eq!(
            manager.support(function),
            Err(SupportError::VariableIndexNotFound(3))
        );
    }
}
