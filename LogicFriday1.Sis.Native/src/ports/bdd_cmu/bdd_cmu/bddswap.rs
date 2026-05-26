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
pub struct SwapReturnStats {
    pub unchanged: usize,
    pub cached: usize,
    pub rebuilt: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SwapStats {
    pub calls: usize,
    pub returns: SwapReturnStats,
    pub cache_inserts: usize,
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
    swap_cache: HashMap<(BddEdge, BddVariableId, BddVariableId), BddEdge>,
    recursion_limit: usize,
    stats: SwapStats,
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            unique_table: HashMap::new(),
            swap_cache: HashMap::new(),
            recursion_limit: 65_536,
            stats: SwapStats::default(),
        }
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::regular(0)
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::regular(1)
    }

    pub fn stats(&self) -> SwapStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn cache_len(&self) -> usize {
        self.swap_cache.len()
    }

    pub fn set_recursion_limit(&mut self, recursion_limit: usize) {
        self.recursion_limit = recursion_limit;
    }

    pub fn node(&self, edge: BddEdge) -> Result<&BddNode, SwapError> {
        self.nodes
            .get(edge.node)
            .ok_or(SwapError::MissingNode(edge.node))
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
    ) -> Result<BddEdge, SwapError> {
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
    ) -> Result<BddEdge, SwapError> {
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.ite_inner(condition, then_edge, else_edge, 0)
    }

    pub fn and(&mut self, left: BddEdge, right: BddEdge) -> Result<BddEdge, SwapError> {
        self.ite(left, right, self.zero())
    }

    pub fn or(&mut self, left: BddEdge, right: BddEdge) -> Result<BddEdge, SwapError> {
        self.ite(left, self.one(), right)
    }

    pub fn swap_vars(
        &mut self,
        function: BddEdge,
        first_variable: BddEdge,
        second_variable: BddEdge,
    ) -> Result<BddEdge, SwapError> {
        self.validate_edge(function)?;
        self.validate_edge(first_variable)?;
        self.validate_edge(second_variable)?;

        let first_variable_id = self.variable_id_from_function(first_variable)?;
        let second_variable_id = self.variable_id_from_function(second_variable)?;

        if first_variable_id == second_variable_id {
            self.stats.returns.unchanged += 1;
            return Ok(function);
        }

        self.swap_cache.clear();
        self.swap_inner(function, first_variable_id, second_variable_id, 0)
    }

    pub fn eval(
        &self,
        root: BddEdge,
        assignment: &HashMap<BddVariableId, bool>,
    ) -> Result<bool, SwapError> {
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

    fn swap_inner(
        &mut self,
        function: BddEdge,
        first_variable: BddVariableId,
        second_variable: BddVariableId,
        depth: usize,
    ) -> Result<BddEdge, SwapError> {
        if depth > self.recursion_limit {
            return Err(SwapError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        self.stats.calls += 1;

        if self.is_constant(function)? {
            self.stats.returns.unchanged += 1;
            return Ok(function);
        }

        let cache_key = (function, first_variable, second_variable);
        if let Some(cached) = self.swap_cache.get(&cache_key).copied() {
            self.stats.returns.cached += 1;
            return Ok(cached);
        }

        let function_variable = self.sort_variable(function)?;
        let (then_edge, else_edge) = self.branches(function)?;
        let swapped_then =
            self.swap_inner(then_edge, first_variable, second_variable, depth + 1)?;
        let swapped_else =
            self.swap_inner(else_edge, first_variable, second_variable, depth + 1)?;

        let result = if function_variable == first_variable {
            let replacement = self.variable_node(second_variable);
            self.ite_inner(replacement, swapped_then, swapped_else, depth + 1)?
        } else if function_variable == second_variable {
            let replacement = self.variable_node(first_variable);
            self.ite_inner(replacement, swapped_then, swapped_else, depth + 1)?
        } else {
            let condition = self.variable_node(function_variable);
            self.ite_inner(condition, swapped_then, swapped_else, depth + 1)?
        };

        self.swap_cache.insert(cache_key, result);
        self.stats.cache_inserts += 1;
        self.stats.returns.rebuilt += 1;
        Ok(result)
    }

    fn ite_inner(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
        depth: usize,
    ) -> Result<BddEdge, SwapError> {
        if depth > self.recursion_limit {
            return Err(SwapError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        if condition == self.one() {
            return Ok(then_edge);
        }

        if condition == self.zero() {
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

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), SwapError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Err(SwapError::ExpectedBranch(edge.node)),
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
    ) -> Result<(BddEdge, BddEdge), SwapError> {
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

    fn validate_edge(&self, edge: BddEdge) -> Result<(), SwapError> {
        self.nodes
            .get(edge.node)
            .map(|_| ())
            .ok_or(SwapError::MissingNode(edge.node))
    }

    fn validate_order(&self, parent: BddVariableId, child: BddEdge) -> Result<(), SwapError> {
        let child_variable = self.sort_variable(child)?;
        if child_variable == BddVariableId::MAX || parent < child_variable {
            Ok(())
        } else {
            Err(SwapError::VariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn variable_id_from_function(&self, edge: BddEdge) -> Result<BddVariableId, SwapError> {
        if edge.is_complemented() {
            return Err(SwapError::ExpectedPositiveVariable(edge));
        }

        match self.node(edge)? {
            BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            } if *then_edge == self.one() && *else_edge == self.zero() => Ok(*variable),
            BddNode::Branch { .. } => Err(SwapError::ExpectedPositiveVariable(edge)),
            BddNode::Constant(_) => Err(SwapError::ExpectedPositiveVariable(edge)),
        }
    }

    fn sort_variable(&self, edge: BddEdge) -> Result<BddVariableId, SwapError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Ok(BddVariableId::MAX),
            BddNode::Branch { variable, .. } => Ok(*variable),
        }
    }

    fn is_constant(&self, edge: BddEdge) -> Result<bool, SwapError> {
        Ok(matches!(
            self.node(BddEdge::regular(edge.node))?,
            BddNode::Constant(_)
        ))
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
pub enum SwapError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    ExpectedPositiveVariable(BddEdge),
    VariableOrder {
        parent: BddVariableId,
        child: BddVariableId,
    },
    RecursionLimitExceeded {
        limit: usize,
    },
}

impl fmt::Display for SwapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch node"),
            Self::ExpectedPositiveVariable(edge) => write!(
                formatter,
                "bdd_swap_vars: swap arguments must be positive variables, got {edge:?}"
            ),
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
            Self::RecursionLimitExceeded { limit } => {
                write!(formatter, "BDD swap recursion limit {limit} was exceeded")
            }
        }
    }
}

impl Error for SwapError {}

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
    fn swaps_two_variables_in_a_function() {
        let (mut manager, x, y, z) = sample_manager();
        let y_or_z = manager.or(y, z).unwrap();
        let function = manager.and(x, y_or_z).unwrap();

        let result = manager.swap_vars(function, x, z).unwrap();

        for x_value in [false, true] {
            for y_value in [false, true] {
                for z_value in [false, true] {
                    let assignment = values(&[(1, x_value), (2, y_value), (3, z_value)]);
                    assert_eq!(
                        manager.eval(result, &assignment).unwrap(),
                        z_value && (y_value || x_value)
                    );
                }
            }
        }
    }

    #[test]
    fn swap_is_symmetric_and_self_inverse() {
        let (mut manager, x, y, z) = sample_manager();
        let function = manager.ite(x, y, z).unwrap();

        let first = manager.swap_vars(function, x, z).unwrap();
        let second = manager.swap_vars(function, z, x).unwrap();
        let restored = manager.swap_vars(first, x, z).unwrap();

        assert_eq!(first, second);
        assert_eq!(restored, function);
    }

    #[test]
    fn keeps_function_when_variables_are_the_same() {
        let (mut manager, x, y, _) = sample_manager();
        let function = manager.and(x, y).unwrap();

        let result = manager.swap_vars(function, x, x).unwrap();

        assert_eq!(result, function);
        assert_eq!(manager.stats().returns.unchanged, 1);
        assert_eq!(manager.cache_len(), 0);
    }

    #[test]
    fn accepts_complemented_function_edges() {
        let (mut manager, x, y, z) = sample_manager();
        let function = manager.and(x, y).unwrap();
        let result = manager.swap_vars(manager.not(function), x, z).unwrap();

        for x_value in [false, true] {
            for y_value in [false, true] {
                for z_value in [false, true] {
                    let assignment = values(&[(1, x_value), (2, y_value), (3, z_value)]);
                    assert_eq!(
                        manager.eval(result, &assignment).unwrap(),
                        !(z_value && y_value)
                    );
                }
            }
        }
    }

    #[test]
    fn caches_repeated_shared_subgraphs() {
        let (mut manager, _, y, z) = sample_manager();
        let shared = manager.and(y, z).unwrap();

        let first = manager.swap_inner(shared, 2, 3, 0).unwrap();
        let second = manager.swap_inner(shared, 2, 3, 0).unwrap();

        assert_eq!(second, first);
        assert_eq!(
            manager.eval(first, &values(&[(1, true), (2, true), (3, false)])),
            Ok(false)
        );
        assert!(manager.stats().returns.cached > 0);
        assert!(manager.cache_len() > 0);
    }

    #[test]
    fn rejects_complemented_swap_variable() {
        let (mut manager, x, y, _) = sample_manager();

        let error = manager.swap_vars(y, manager.not(x), y).unwrap_err();

        assert_eq!(error, SwapError::ExpectedPositiveVariable(manager.not(x)));
    }

    #[test]
    fn rejects_non_variable_swap_argument() {
        let (mut manager, x, y, z) = sample_manager();
        let non_variable = manager.and(x, y).unwrap();

        let error = manager.swap_vars(z, non_variable, y).unwrap_err();

        assert_eq!(error, SwapError::ExpectedPositiveVariable(non_variable));
    }

    #[test]
    fn reports_invalid_references() {
        let (mut manager, x, y, _) = sample_manager();

        let error = manager.swap_vars(BddEdge::regular(999), x, y).unwrap_err();

        assert_eq!(error, SwapError::MissingNode(999));
    }

    #[test]
    fn recursion_limit_protects_swap_walks() {
        let (mut manager, x, y, z) = sample_manager();
        let function = manager.ite(x, y, z).unwrap();
        manager.set_recursion_limit(0);

        let error = manager.swap_vars(function, x, z).unwrap_err();

        assert_eq!(error, SwapError::RecursionLimitExceeded { limit: 0 });
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bddswap.rs");
        let legacy_export = concat!("no", "_", "mangle");
        let tracking_prefix = concat!("REQUIRED", "_");
        let dependency_type = concat!("Port", "Dependency");
        let bead_token = concat!("bead", "_id");
        let source_token = concat!("source", "_file");
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
