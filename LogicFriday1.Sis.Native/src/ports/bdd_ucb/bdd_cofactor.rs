use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type BddVariableId = u32;
pub type BddNodeId = usize;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddEdge {
    node: BddNodeId,
    complemented: bool,
}

impl BddEdge {
    pub const fn regular(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: false,
        }
    }

    pub const fn complemented(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: true,
        }
    }

    pub const fn node(self) -> BddNodeId {
        self.node
    }

    pub const fn is_complemented(self) -> bool {
        self.complemented
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddNode {
    Constant(bool),
    Branch {
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CofactorReturnStats {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CofactorStats {
    pub calls: usize,
    pub returns: CofactorReturnStats,
    pub cache_inserts: usize,
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
    cofactor_cache: HashMap<(BddEdge, BddEdge), BddEdge>,
    recursion_limit: usize,
    stats: CofactorStats,
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            unique_table: HashMap::new(),
            cofactor_cache: HashMap::new(),
            recursion_limit: 65_536,
            stats: CofactorStats::default(),
        }
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::regular(0)
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::regular(1)
    }

    pub fn stats(&self) -> CofactorStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn cache_len(&self) -> usize {
        self.cofactor_cache.len()
    }

    pub fn set_recursion_limit(&mut self, recursion_limit: usize) {
        self.recursion_limit = recursion_limit;
    }

    pub fn node(&self, edge: BddEdge) -> Result<&BddNode, CofactorError> {
        self.nodes
            .get(edge.node)
            .ok_or(CofactorError::MissingNode(edge.node))
    }

    pub fn variable(&mut self, variable: BddVariableId) -> BddEdge {
        self.find_or_add(variable, self.one(), self.zero())
            .expect("variable nodes use ordered constant children")
    }

    pub fn find_or_add(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, CofactorError> {
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.validate_order(variable, then_edge)?;
        self.validate_order(variable, else_edge)?;
        Ok(self.find_or_add_unchecked(variable, then_edge, else_edge))
    }

    pub fn ite(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, CofactorError> {
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.ite_inner(condition, then_edge, else_edge, 0)
    }

    pub fn and(&mut self, left: BddEdge, right: BddEdge) -> Result<BddEdge, CofactorError> {
        self.ite(left, right, self.zero())
    }

    pub fn or(&mut self, left: BddEdge, right: BddEdge) -> Result<BddEdge, CofactorError> {
        self.ite(left, self.one(), right)
    }

    pub fn cofactor(
        &mut self,
        function: BddEdge,
        constraint: BddEdge,
    ) -> Result<BddEdge, CofactorError> {
        self.validate_edge(function)?;
        self.validate_edge(constraint)?;
        if self.is_zero(constraint)? {
            return Err(CofactorError::CofactorWithZero);
        }

        self.cofactor_cache.clear();
        self.cofactor_inner(function, constraint, 0)
    }

    pub fn eval(
        &self,
        root: BddEdge,
        assignment: &HashMap<BddVariableId, bool>,
    ) -> Result<bool, CofactorError> {
        let mut current = root;
        let mut complemented = false;

        loop {
            complemented ^= current.is_complemented();
            match self.node(BddEdge::regular(current.node))? {
                BddNode::Constant(value) => return Ok(*value ^ complemented),
                BddNode::Branch {
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

    fn cofactor_inner(
        &mut self,
        function: BddEdge,
        constraint: BddEdge,
        depth: usize,
    ) -> Result<BddEdge, CofactorError> {
        if depth > self.recursion_limit {
            return Err(CofactorError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        self.stats.calls += 1;

        if self.is_constant(function)? || constraint == self.one() {
            self.stats.returns.trivial += 1;
            return Ok(function);
        }

        let cache_key = (function, constraint);
        if let Some(cached) = self.cofactor_cache.get(&cache_key).copied() {
            self.stats.returns.cached += 1;
            return Ok(cached);
        }

        let function_id = self.sort_variable(function)?;
        let constraint_id = self.sort_variable(constraint)?;
        let (function_then, function_else) = self.branches(function)?;
        let (constraint_then, constraint_else) = self.branches(constraint)?;

        let result = if function_id > constraint_id {
            if self.is_zero(constraint_else)? {
                self.cofactor_inner(function, constraint_then, depth + 1)?
            } else if self.is_zero(constraint_then)? {
                self.cofactor_inner(function, constraint_else, depth + 1)?
            } else {
                let then_result = self.cofactor_inner(function, constraint_then, depth + 1)?;
                let else_result = self.cofactor_inner(function, constraint_else, depth + 1)?;
                let variable = self.variable_node(constraint_id);
                self.ite_inner(variable, then_result, else_result, depth + 1)?
            }
        } else if function_id == constraint_id {
            if self.is_zero(constraint_else)? {
                self.cofactor_inner(function_then, constraint_then, depth + 1)?
            } else if self.is_zero(constraint_then)? {
                self.cofactor_inner(function_else, constraint_else, depth + 1)?
            } else {
                let then_result = self.cofactor_inner(function_then, constraint_then, depth + 1)?;
                let else_result = self.cofactor_inner(function_else, constraint_else, depth + 1)?;
                let variable = self.variable_node(function_id);
                self.ite_inner(variable, then_result, else_result, depth + 1)?
            }
        } else {
            let then_result = self.cofactor_inner(function_then, constraint, depth + 1)?;
            let else_result = self.cofactor_inner(function_else, constraint, depth + 1)?;
            let variable = self.variable_node(function_id);
            self.ite_inner(variable, then_result, else_result, depth + 1)?
        };

        self.cofactor_cache.insert(cache_key, result);
        self.stats.cache_inserts += 1;
        self.stats.returns.full += 1;
        Ok(result)
    }

    fn ite_inner(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
        depth: usize,
    ) -> Result<BddEdge, CofactorError> {
        if depth > self.recursion_limit {
            return Err(CofactorError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        if self.is_one(condition)? {
            return Ok(then_edge);
        }

        if self.is_zero(condition)? {
            return Ok(else_edge);
        }

        if then_edge == else_edge {
            return Ok(then_edge);
        }

        if then_edge == self.one() && else_edge == self.zero() {
            return Ok(condition);
        }

        if then_edge == self.zero() && else_edge == self.one() {
            return Ok(self.not(condition));
        }

        let variable = self
            .sort_variable(condition)?
            .min(self.sort_variable(then_edge)?)
            .min(self.sort_variable(else_edge)?);
        let (condition_then, condition_else) = self.quick_cofactor(condition, variable)?;
        let (then_then, then_else) = self.quick_cofactor(then_edge, variable)?;
        let (else_then, else_else) = self.quick_cofactor(else_edge, variable)?;
        let high = self.ite_inner(condition_then, then_then, else_then, depth + 1)?;
        let low = self.ite_inner(condition_else, then_else, else_else, depth + 1)?;

        Ok(self.find_or_add_unchecked(variable, high, low))
    }

    fn variable_node(&mut self, variable: BddVariableId) -> BddEdge {
        self.find_or_add_unchecked(variable, self.one(), self.zero())
    }

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), CofactorError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Err(CofactorError::ExpectedBranch(edge.node)),
            BddNode::Branch {
                then_edge,
                else_edge,
                ..
            } => {
                if edge.is_complemented() {
                    Ok((self.not(*then_edge), self.not(*else_edge)))
                } else {
                    Ok((*then_edge, *else_edge))
                }
            }
        }
    }

    fn quick_cofactor(
        &self,
        edge: BddEdge,
        variable: BddVariableId,
    ) -> Result<(BddEdge, BddEdge), CofactorError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Branch {
                variable: node_variable,
                then_edge,
                else_edge,
            } if *node_variable == variable => {
                if edge.is_complemented() {
                    Ok((self.not(*then_edge), self.not(*else_edge)))
                } else {
                    Ok((*then_edge, *else_edge))
                }
            }
            _ => Ok((edge, edge)),
        }
    }

    fn find_or_add_unchecked(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> BddEdge {
        if then_edge == else_edge {
            return then_edge;
        }

        let key = (variable, then_edge, else_edge);
        if let Some(existing) = self.unique_table.get(&key).copied() {
            return existing;
        }

        let edge = BddEdge::regular(self.nodes.len());
        self.nodes.push(BddNode::Branch {
            variable,
            then_edge,
            else_edge,
        });
        self.unique_table.insert(key, edge);
        edge
    }

    fn validate_edge(&self, edge: BddEdge) -> Result<(), CofactorError> {
        self.nodes
            .get(edge.node)
            .map(|_| ())
            .ok_or(CofactorError::MissingNode(edge.node))
    }

    fn validate_order(&self, parent: BddVariableId, child: BddEdge) -> Result<(), CofactorError> {
        let child_variable = self.sort_variable(child)?;
        if child_variable == BddVariableId::MAX || parent < child_variable {
            Ok(())
        } else {
            Err(CofactorError::VariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn sort_variable(&self, edge: BddEdge) -> Result<BddVariableId, CofactorError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Ok(BddVariableId::MAX),
            BddNode::Branch { variable, .. } => Ok(*variable),
        }
    }

    fn is_constant(&self, edge: BddEdge) -> Result<bool, CofactorError> {
        Ok(matches!(
            self.node(BddEdge::regular(edge.node))?,
            BddNode::Constant(_)
        ))
    }

    fn is_zero(&self, edge: BddEdge) -> Result<bool, CofactorError> {
        self.constant_value(edge).map(|value| value == Some(false))
    }

    fn is_one(&self, edge: BddEdge) -> Result<bool, CofactorError> {
        self.constant_value(edge).map(|value| value == Some(true))
    }

    fn constant_value(&self, edge: BddEdge) -> Result<Option<bool>, CofactorError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(value) => Ok(Some(*value ^ edge.is_complemented())),
            BddNode::Branch { .. } => Ok(None),
        }
    }

    fn not(&self, edge: BddEdge) -> BddEdge {
        if edge == self.zero() {
            self.one()
        } else if edge == self.one() {
            self.zero()
        } else {
            BddEdge {
                node: edge.node,
                complemented: !edge.complemented,
            }
        }
    }
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CofactorError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    CofactorWithZero,
    VariableOrder {
        parent: BddVariableId,
        child: BddVariableId,
    },
    RecursionLimitExceeded {
        limit: usize,
    },
}

impl fmt::Display for CofactorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch node"),
            Self::CofactorWithZero => {
                write!(formatter, "bdd_cofactor: cofactor wrt zero not defined")
            }
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
            Self::RecursionLimitExceeded { limit } => write!(
                formatter,
                "BDD cofactor recursion limit {limit} was exceeded"
            ),
        }
    }
}

impl Error for CofactorError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(entries: &[(BddVariableId, bool)]) -> HashMap<BddVariableId, bool> {
        entries.iter().copied().collect()
    }

    fn sample_manager() -> (BddManager, BddEdge, BddEdge, BddEdge) {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);
        let z = manager.variable(3);

        (manager, x, y, z)
    }

    #[test]
    fn returns_constant_function_unchanged() {
        let (mut manager, _, y, _) = sample_manager();

        let result = manager.cofactor(manager.one(), y).unwrap();

        assert_eq!(result, manager.one());
        assert_eq!(manager.stats().returns.trivial, 1);
    }

    #[test]
    fn returns_function_when_constraint_is_one() {
        let (mut manager, x, y, _) = sample_manager();
        let function = manager.ite(x, y, manager.zero()).unwrap();

        let result = manager.cofactor(function, manager.one()).unwrap();

        assert_eq!(result, function);
    }

    #[test]
    fn rejects_zero_constraint() {
        let (mut manager, x, _, _) = sample_manager();

        let error = manager.cofactor(x, manager.zero()).unwrap_err();

        assert_eq!(error, CofactorError::CofactorWithZero);
    }

    #[test]
    fn cofactors_top_variable_with_positive_constraint() {
        let (mut manager, x, y, z) = sample_manager();
        let function = manager.ite(x, y, z).unwrap();

        let result = manager.cofactor(function, x).unwrap();

        for y_value in [false, true] {
            for z_value in [false, true] {
                let assignment = values(&[(1, true), (2, y_value), (3, z_value)]);
                assert_eq!(
                    manager.eval(result, &assignment).unwrap(),
                    manager.eval(y, &assignment).unwrap()
                );
            }
        }
    }

    #[test]
    fn cofactors_top_variable_with_negative_constraint() {
        let (mut manager, x, y, z) = sample_manager();
        let function = manager.ite(x, y, z).unwrap();
        let not_x = manager.not(x);

        let result = manager.cofactor(function, not_x).unwrap();

        for y_value in [false, true] {
            for z_value in [false, true] {
                let assignment = values(&[(1, false), (2, y_value), (3, z_value)]);
                assert_eq!(
                    manager.eval(result, &assignment).unwrap(),
                    manager.eval(z, &assignment).unwrap()
                );
            }
        }
    }

    #[test]
    fn preserves_constraint_variable_when_both_constraint_branches_are_possible() {
        let (mut manager, x, y, z) = sample_manager();
        let function = manager.ite(y, x, z).unwrap();
        let constraint = manager.ite(x, y, z).unwrap();

        let result = manager.cofactor(function, constraint).unwrap();

        for x_value in [false, true] {
            for y_value in [false, true] {
                for z_value in [false, true] {
                    let assignment = values(&[(1, x_value), (2, y_value), (3, z_value)]);
                    if manager.eval(constraint, &assignment).unwrap() {
                        assert_eq!(
                            manager.eval(result, &assignment).unwrap(),
                            manager.eval(function, &assignment).unwrap()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn decomposes_function_when_function_variable_precedes_constraint() {
        let (mut manager, x, y, z) = sample_manager();
        let function = manager.ite(x, y, z).unwrap();

        let result = manager.cofactor(function, y).unwrap();

        for x_value in [false, true] {
            for z_value in [false, true] {
                let assignment = values(&[(1, x_value), (2, true), (3, z_value)]);
                assert_eq!(
                    manager.eval(result, &assignment).unwrap(),
                    manager.eval(function, &assignment).unwrap()
                );
            }
        }
    }

    #[test]
    fn caches_repeated_recursive_results() {
        let (mut manager, x, y, z) = sample_manager();
        let y_or_z = manager.or(y, z).unwrap();
        let function = manager.ite(x, y_or_z, y_or_z).unwrap();
        let constraint = manager.ite(y, x, z).unwrap();

        let _ = manager.cofactor(function, constraint).unwrap();

        assert!(manager.stats().returns.cached > 0);
        assert!(manager.cache_len() > 0);
    }

    #[test]
    fn reports_invalid_edge() {
        let (mut manager, x, _, _) = sample_manager();

        let error = manager.cofactor(x, BddEdge::regular(999)).unwrap_err();

        assert_eq!(error, CofactorError::MissingNode(999));
    }
}
