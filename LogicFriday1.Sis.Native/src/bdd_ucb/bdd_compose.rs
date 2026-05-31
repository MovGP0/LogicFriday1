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

    pub const fn not(self) -> Self {
        match (self.node, self.complemented) {
            (0, false) | (1, true) => Self::regular(1),
            (1, false) | (0, true) => Self::regular(0),
            _ => Self {
                node: self.node,
                complemented: !self.complemented,
            },
        }
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
pub struct ComposeReturnStats {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComposeStats {
    pub calls: usize,
    pub returns: ComposeReturnStats,
    pub cache_inserts: usize,
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
    compose_cache: HashMap<(BddEdge, BddVariableId, BddEdge), BddEdge>,
    stats: ComposeStats,
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            unique_table: HashMap::new(),
            compose_cache: HashMap::new(),
            stats: ComposeStats::default(),
        }
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::regular(0)
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::regular(1)
    }

    pub fn stats(&self) -> ComposeStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn cache_len(&self) -> usize {
        self.compose_cache.len()
    }

    pub fn node(&self, edge: BddEdge) -> Result<&BddNode, ComposeError> {
        self.nodes
            .get(edge.node)
            .ok_or(ComposeError::MissingNode(edge.node))
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
    ) -> Result<BddEdge, ComposeError> {
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
    ) -> Result<BddEdge, ComposeError> {
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.ite_inner(condition, then_edge, else_edge)
    }

    pub fn compose(
        &mut self,
        function: BddEdge,
        variable: BddEdge,
        replacement: BddEdge,
    ) -> Result<BddEdge, ComposeError> {
        self.validate_edge(function)?;
        self.validate_edge(variable)?;
        self.validate_edge(replacement)?;

        let variable_id = self.variable_id_from_function(variable)?;
        self.compose_cache.clear();
        self.compose_inner(function, variable_id, replacement)
    }

    pub fn eval(
        &self,
        root: BddEdge,
        assignment: &HashMap<BddVariableId, bool>,
    ) -> Result<bool, ComposeError> {
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

    fn compose_inner(
        &mut self,
        function: BddEdge,
        variable: BddVariableId,
        replacement: BddEdge,
    ) -> Result<BddEdge, ComposeError> {
        self.stats.calls += 1;

        let function_variable = self.sort_variable(function)?;
        if function_variable > variable {
            self.stats.returns.trivial += 1;
            return Ok(function);
        }

        let cache_key = (function, variable, replacement);
        if let Some(cached) = self.compose_cache.get(&cache_key).copied() {
            self.stats.returns.cached += 1;
            return Ok(cached);
        }

        let (then_edge, else_edge) = self.branches(function)?;
        let result = if function_variable == variable {
            self.ite(replacement, then_edge, else_edge)?
        } else {
            let composed_then = self.compose_inner(then_edge, variable, replacement)?;
            let composed_else = self.compose_inner(else_edge, variable, replacement)?;
            let condition = self.find_or_add_unchecked(function_variable, self.one(), self.zero());

            self.ite(condition, composed_then, composed_else)?
        };

        self.compose_cache.insert(cache_key, result);
        self.stats.cache_inserts += 1;
        self.stats.returns.full += 1;
        Ok(result)
    }

    fn ite_inner(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, ComposeError> {
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
            return Ok(condition.not());
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

        Ok(self.find_or_add_unchecked(variable, high, low))
    }

    fn cofactor(
        &self,
        edge: BddEdge,
        variable: BddVariableId,
    ) -> Result<(BddEdge, BddEdge), ComposeError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Branch {
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

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), ComposeError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Err(ComposeError::ExpectedBranch(edge.node)),
            BddNode::Branch {
                then_edge,
                else_edge,
                ..
            } => {
                if edge.is_complemented() {
                    Ok((then_edge.not(), else_edge.not()))
                } else {
                    Ok((*then_edge, *else_edge))
                }
            }
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

    fn validate_edge(&self, edge: BddEdge) -> Result<(), ComposeError> {
        self.nodes
            .get(edge.node)
            .map(|_| ())
            .ok_or(ComposeError::MissingNode(edge.node))
    }

    fn validate_order(&self, parent: BddVariableId, child: BddEdge) -> Result<(), ComposeError> {
        let child_variable = self.sort_variable(child)?;
        if child_variable == BddVariableId::MAX || parent < child_variable {
            Ok(())
        } else {
            Err(ComposeError::VariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn variable_id_from_function(&self, edge: BddEdge) -> Result<BddVariableId, ComposeError> {
        if edge.is_complemented() {
            return Err(ComposeError::ExpectedVariableFunction(edge));
        }

        match self.node(edge)? {
            BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            } if *then_edge == self.one() && *else_edge == self.zero() => Ok(*variable),
            BddNode::Branch { .. } => Err(ComposeError::ExpectedVariableFunction(edge)),
            BddNode::Constant(_) => Err(ComposeError::ExpectedVariableFunction(edge)),
        }
    }

    fn sort_variable(&self, edge: BddEdge) -> Result<BddVariableId, ComposeError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Ok(BddVariableId::MAX),
            BddNode::Branch { variable, .. } => Ok(*variable),
        }
    }
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComposeError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    ExpectedVariableFunction(BddEdge),
    VariableOrder {
        parent: BddVariableId,
        child: BddVariableId,
    },
}

impl fmt::Display for ComposeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch node"),
            Self::ExpectedVariableFunction(edge) => {
                write!(
                    formatter,
                    "bdd_compose: second argument {edge:?} is not a variable"
                )
            }
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
        }
    }
}

impl Error for ComposeError {}

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
    fn substitutes_replacement_for_target_variable() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(x, y, manager.zero()).unwrap();

        let result = manager.compose(expression, x, z).unwrap();

        for y_value in [false, true] {
            for z_value in [false, true] {
                let assignment = values(&[(1, false), (2, y_value), (3, z_value)]);
                assert_eq!(
                    manager.eval(result, &assignment).unwrap(),
                    z_value && y_value
                );
            }
        }
    }

    #[test]
    fn applies_ite_when_root_is_target_variable() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(x, y, z).unwrap();

        let result = manager.compose(expression, x, z).unwrap();

        for y_value in [false, true] {
            for z_value in [false, true] {
                let assignment = values(&[(1, true), (2, y_value), (3, z_value)]);
                assert_eq!(
                    manager.eval(result, &assignment).unwrap(),
                    if z_value { y_value } else { z_value }
                );
            }
        }
    }

    #[test]
    fn returns_original_when_target_variable_is_above_function() {
        let (mut manager, x, y, z) = sample_manager();

        let result = manager.compose(y, x, z).unwrap();

        assert_eq!(result, y);
        assert_eq!(manager.stats().returns.trivial, 1);
        assert_eq!(manager.cache_len(), 0);
    }

    #[test]
    fn preserves_higher_variables_around_composed_children() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(x, y, manager.one()).unwrap();

        let result = manager.compose(expression, y, z).unwrap();

        for x_value in [false, true] {
            for z_value in [false, true] {
                let assignment = values(&[(1, x_value), (2, false), (3, z_value)]);
                assert_eq!(
                    manager.eval(result, &assignment).unwrap(),
                    if x_value { z_value } else { true }
                );
            }
        }
    }

    #[test]
    fn uses_adhoc_cache_for_repeated_shared_subgraphs() {
        let (mut manager, x, y, z) = sample_manager();
        let shared = manager.ite(y, z, manager.zero()).unwrap();
        let result = manager.compose_inner(shared, 3, x).unwrap();
        let cached = manager.compose_inner(shared, 3, x).unwrap();

        assert_eq!(cached, result);
        assert!(manager.stats().returns.cached > 0);
        assert!(manager.cache_len() > 0);
    }

    #[test]
    fn accepts_complemented_function_and_replacement_edges() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(x, y, manager.zero()).unwrap().not();
        let replacement = z.not();

        let result = manager.compose(expression, x, replacement).unwrap();

        for y_value in [false, true] {
            for z_value in [false, true] {
                let assignment = values(&[(1, false), (2, y_value), (3, z_value)]);
                assert_eq!(
                    manager.eval(result, &assignment).unwrap(),
                    !(z_value ^ true && y_value)
                );
            }
        }
    }

    #[test]
    fn rejects_complemented_second_argument() {
        let (mut manager, x, y, z) = sample_manager();

        let error = manager.compose(y, x.not(), z).unwrap_err();

        assert_eq!(error, ComposeError::ExpectedVariableFunction(x.not()));
    }

    #[test]
    fn rejects_non_variable_second_argument() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(x, y, z).unwrap();

        let error = manager.compose(x, expression, z).unwrap_err();

        assert_eq!(error, ComposeError::ExpectedVariableFunction(expression));
    }

    #[test]
    fn rejects_invalid_references() {
        let (mut manager, x, _, z) = sample_manager();

        let error = manager.compose(BddEdge::regular(99), x, z).unwrap_err();

        assert_eq!(error, ComposeError::MissingNode(99));
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bdd_compose.rs");
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
