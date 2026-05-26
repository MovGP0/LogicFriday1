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
    pub unchanged: usize,
    pub restricted: usize,
    pub composed: usize,
    pub substituted: usize,
    pub cached: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComposeStats {
    pub compose_calls: usize,
    pub restrict_calls: usize,
    pub substitute_calls: usize,
    pub returns: ComposeReturnStats,
    pub cache_inserts: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum CacheKey {
    Compose {
        function: BddEdge,
        variable: BddVariableId,
        replacement: BddEdge,
    },
    Restrict {
        function: BddEdge,
        variable: BddVariableId,
        value: bool,
    },
    Substitute {
        function: BddEdge,
        association: usize,
    },
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
    cache: HashMap<CacheKey, BddEdge>,
    current_association: HashMap<BddVariableId, BddEdge>,
    current_association_id: usize,
    stats: ComposeStats,
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            unique_table: HashMap::new(),
            cache: HashMap::new(),
            current_association: HashMap::new(),
            current_association_id: 0,
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
        self.cache.len()
    }

    pub fn node(&self, edge: BddEdge) -> Result<&BddNode, BddCompError> {
        self.nodes
            .get(edge.node)
            .ok_or(BddCompError::MissingNode(edge.node))
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
    ) -> Result<BddEdge, BddCompError> {
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
    ) -> Result<BddEdge, BddCompError> {
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.ite_inner(condition, then_edge, else_edge)
    }

    pub fn and(&mut self, left: BddEdge, right: BddEdge) -> Result<BddEdge, BddCompError> {
        self.ite(left, right, self.zero())
    }

    pub fn or(&mut self, left: BddEdge, right: BddEdge) -> Result<BddEdge, BddCompError> {
        self.ite(left, self.one(), right)
    }

    pub fn compose(
        &mut self,
        function: BddEdge,
        variable: BddEdge,
        replacement: BddEdge,
    ) -> Result<BddEdge, BddCompError> {
        self.validate_edge(function)?;
        self.validate_edge(variable)?;
        self.validate_edge(replacement)?;

        let variable_id = self.variable_id_from_function(variable)?;
        self.compose_temp(function, variable_id, replacement)
    }

    pub fn compose_temp(
        &mut self,
        function: BddEdge,
        variable: BddVariableId,
        replacement: BddEdge,
    ) -> Result<BddEdge, BddCompError> {
        self.validate_edge(function)?;
        self.validate_edge(replacement)?;
        self.compose_step(function, variable, replacement)
    }

    pub fn set_current_association<I>(&mut self, replacements: I) -> Result<(), BddCompError>
    where
        I: IntoIterator<Item = (BddVariableId, BddEdge)>,
    {
        let mut association = HashMap::new();
        for (variable, replacement) in replacements {
            self.validate_edge(replacement)?;
            association.insert(variable, replacement);
        }

        self.current_association = association;
        self.current_association_id = self.current_association_id.wrapping_add(1);
        Ok(())
    }

    pub fn substitute(&mut self, function: BddEdge) -> Result<BddEdge, BddCompError> {
        self.validate_edge(function)?;

        if self.current_association.is_empty() {
            self.stats.returns.unchanged += 1;
            return Ok(function);
        }

        self.substitute_step(function)
    }

    pub fn eval(
        &self,
        root: BddEdge,
        assignment: &HashMap<BddVariableId, bool>,
    ) -> Result<bool, BddCompError> {
        self.validate_edge(root)?;

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

    fn compose_step(
        &mut self,
        function: BddEdge,
        variable: BddVariableId,
        replacement: BddEdge,
    ) -> Result<BddEdge, BddCompError> {
        self.stats.compose_calls += 1;

        if let Some(value) = self.constant_value(replacement)? {
            return self.restrict_step(function, variable, value);
        }

        let function_variable = self.sort_variable(function)?;
        if function_variable > variable {
            self.stats.returns.unchanged += 1;
            return Ok(function);
        }

        let cache_key = CacheKey::Compose {
            function,
            variable,
            replacement,
        };
        if let Some(cached) = self.cache.get(&cache_key).copied() {
            self.stats.returns.cached += 1;
            return Ok(cached);
        }

        let result = if function_variable == variable {
            let (then_edge, else_edge) = self.branches(function)?;
            self.ite_inner(replacement, then_edge, else_edge)?
        } else {
            let top_variable = function_variable.min(self.sort_variable(replacement)?);
            let (function_then, function_else) = self.cofactor(function, top_variable)?;
            let (replacement_then, replacement_else) = self.cofactor(replacement, top_variable)?;
            let composed_then = self.compose_step(function_then, variable, replacement_then)?;
            let composed_else = self.compose_step(function_else, variable, replacement_else)?;

            self.find_or_add_unchecked(top_variable, composed_then, composed_else)
        };

        self.cache.insert(cache_key, result);
        self.stats.cache_inserts += 1;
        self.stats.returns.composed += 1;
        Ok(result)
    }

    fn restrict_step(
        &mut self,
        function: BddEdge,
        variable: BddVariableId,
        value: bool,
    ) -> Result<BddEdge, BddCompError> {
        self.stats.restrict_calls += 1;

        let function_variable = self.sort_variable(function)?;
        if function_variable > variable {
            self.stats.returns.unchanged += 1;
            return Ok(function);
        }

        if function_variable == variable {
            let (then_edge, else_edge) = self.branches(function)?;
            self.stats.returns.restricted += 1;
            return Ok(if value { then_edge } else { else_edge });
        }

        let cache_key = CacheKey::Restrict {
            function,
            variable,
            value,
        };
        if let Some(cached) = self.cache.get(&cache_key).copied() {
            self.stats.returns.cached += 1;
            return Ok(cached);
        }

        let (then_edge, else_edge) = self.branches(function)?;
        let restricted_then = self.restrict_step(then_edge, variable, value)?;
        let restricted_else = self.restrict_step(else_edge, variable, value)?;
        let result =
            self.find_or_add_unchecked(function_variable, restricted_then, restricted_else);

        self.cache.insert(cache_key, result);
        self.stats.cache_inserts += 1;
        self.stats.returns.restricted += 1;
        Ok(result)
    }

    fn substitute_step(&mut self, function: BddEdge) -> Result<BddEdge, BddCompError> {
        self.stats.substitute_calls += 1;

        let Some(last_variable) = self.current_association.keys().copied().max() else {
            self.stats.returns.unchanged += 1;
            return Ok(function);
        };

        let function_variable = self.sort_variable(function)?;
        if function_variable > last_variable {
            self.stats.returns.unchanged += 1;
            return Ok(function);
        }

        let cache_key = CacheKey::Substitute {
            function,
            association: self.current_association_id,
        };
        if let Some(cached) = self.cache.get(&cache_key).copied() {
            self.stats.returns.cached += 1;
            return Ok(cached);
        }

        let (then_edge, else_edge) = self.branches(function)?;
        let substituted_then = self.substitute_step(then_edge)?;
        let substituted_else = self.substitute_step(else_edge)?;
        let result = match self.current_association.get(&function_variable).copied() {
            Some(replacement) if replacement == self.one() => substituted_then,
            Some(replacement) if replacement == self.zero() => substituted_else,
            Some(replacement) => {
                if self.can_find_directly(replacement, substituted_then, substituted_else)? {
                    self.find_or_add_unchecked(
                        function_variable_of(replacement, &self.nodes)?,
                        substituted_then,
                        substituted_else,
                    )
                } else {
                    self.ite_inner(replacement, substituted_then, substituted_else)?
                }
            }
            None => {
                self.find_or_add_unchecked(function_variable, substituted_then, substituted_else)
            }
        };

        self.cache.insert(cache_key, result);
        self.stats.cache_inserts += 1;
        self.stats.returns.substituted += 1;
        Ok(result)
    }

    fn ite_inner(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, BddCompError> {
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

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), BddCompError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Err(BddCompError::ExpectedBranch(edge.node)),
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

    fn cofactor(
        &self,
        edge: BddEdge,
        variable: BddVariableId,
    ) -> Result<(BddEdge, BddEdge), BddCompError> {
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

    fn validate_edge(&self, edge: BddEdge) -> Result<(), BddCompError> {
        self.nodes
            .get(edge.node)
            .map(|_| ())
            .ok_or(BddCompError::MissingNode(edge.node))
    }

    fn validate_order(&self, parent: BddVariableId, child: BddEdge) -> Result<(), BddCompError> {
        let child_variable = self.sort_variable(child)?;
        if child_variable == BddVariableId::MAX || parent < child_variable {
            Ok(())
        } else {
            Err(BddCompError::VariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn variable_id_from_function(&self, edge: BddEdge) -> Result<BddVariableId, BddCompError> {
        if edge.is_complemented() {
            return Err(BddCompError::ExpectedPositiveVariable(edge));
        }

        match self.node(edge)? {
            BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            } if *then_edge == self.one() && *else_edge == self.zero() => Ok(*variable),
            BddNode::Branch { .. } => Err(BddCompError::ExpectedPositiveVariable(edge)),
            BddNode::Constant(_) => Err(BddCompError::ExpectedPositiveVariable(edge)),
        }
    }

    fn sort_variable(&self, edge: BddEdge) -> Result<BddVariableId, BddCompError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Ok(BddVariableId::MAX),
            BddNode::Branch { variable, .. } => Ok(*variable),
        }
    }

    fn constant_value(&self, edge: BddEdge) -> Result<Option<bool>, BddCompError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(value) => Ok(Some(*value ^ edge.is_complemented())),
            BddNode::Branch { .. } => Ok(None),
        }
    }

    fn can_find_directly(
        &self,
        replacement: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<bool, BddCompError> {
        if replacement.is_complemented() {
            return Ok(false);
        }

        let replacement_variable = self.sort_variable(replacement)?;
        Ok(replacement_variable < self.sort_variable(then_edge)?
            && replacement_variable < self.sort_variable(else_edge)?)
    }
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddCompError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    ExpectedPositiveVariable(BddEdge),
    VariableOrder {
        parent: BddVariableId,
        child: BddVariableId,
    },
}

impl fmt::Display for BddCompError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch node"),
            Self::ExpectedPositiveVariable(edge) => write!(
                formatter,
                "bdd composition expects a positive variable argument, got {edge:?}"
            ),
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
        }
    }
}

impl Error for BddCompError {}

fn function_variable_of(edge: BddEdge, nodes: &[BddNode]) -> Result<BddVariableId, BddCompError> {
    match nodes.get(edge.node) {
        Some(BddNode::Branch { variable, .. }) => Ok(*variable),
        Some(BddNode::Constant(_)) => Err(BddCompError::ExpectedBranch(edge.node)),
        None => Err(BddCompError::MissingNode(edge.node)),
    }
}

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
    fn compose_substitutes_replacement_for_variable() {
        let (mut manager, x, y, z) = sample_manager();
        let x_and_y = manager.and(x, y).unwrap();

        let result = manager.compose(x_and_y, x, z).unwrap();

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
    fn compose_uses_restriction_for_constant_replacement() {
        let (mut manager, x, y, _) = sample_manager();
        let expression = manager.or(x, y).unwrap();

        let result = manager.compose(expression, x, manager.zero()).unwrap();

        assert_eq!(result, y);
        assert!(manager.stats().restrict_calls > 0);
        assert!(manager.stats().returns.restricted > 0);
    }

    #[test]
    fn compose_allows_replacement_to_depend_on_target_variable() {
        let (mut manager, x, y, z) = sample_manager();
        let x_and_y = manager.and(x, y).unwrap();
        let replacement = manager.or(x, z).unwrap();

        let result = manager.compose(x_and_y, x, replacement).unwrap();

        for x_value in [false, true] {
            for y_value in [false, true] {
                for z_value in [false, true] {
                    let assignment = values(&[(1, x_value), (2, y_value), (3, z_value)]);
                    assert_eq!(
                        manager.eval(result, &assignment).unwrap(),
                        (x_value || z_value) && y_value
                    );
                }
            }
        }
    }

    #[test]
    fn substitute_uses_current_association() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(x, y, z).unwrap();
        manager
            .set_current_association([(1, z), (2, manager.zero())])
            .unwrap();

        let result = manager.substitute(expression).unwrap();

        for z_value in [false, true] {
            let assignment = values(&[(1, false), (2, true), (3, z_value)]);
            assert!(!manager.eval(result, &assignment).unwrap());
        }
    }

    #[test]
    fn substitute_keeps_unassociated_variables() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(x, y, z).unwrap();
        manager
            .set_current_association([(2, manager.one())])
            .unwrap();

        let result = manager.substitute(expression).unwrap();

        for x_value in [false, true] {
            for z_value in [false, true] {
                let assignment = values(&[(1, x_value), (2, false), (3, z_value)]);
                assert_eq!(
                    manager.eval(result, &assignment).unwrap(),
                    if x_value { true } else { z_value }
                );
            }
        }
    }

    #[test]
    fn repeated_substitute_uses_cache_for_shared_graph() {
        let (mut manager, x, y, z) = sample_manager();
        let shared = manager.and(x, y).unwrap();
        manager.set_current_association([(1, z)]).unwrap();

        let first = manager.substitute(shared).unwrap();
        let second = manager.substitute(shared).unwrap();

        assert_eq!(second, first);
        assert!(manager.stats().returns.cached > 0);
        assert!(manager.cache_len() > 0);
    }

    #[test]
    fn rejects_non_variable_compose_argument() {
        let (mut manager, x, y, z) = sample_manager();
        let non_variable = manager.and(x, y).unwrap();

        let error = manager.compose(z, non_variable, y).unwrap_err();

        assert_eq!(error, BddCompError::ExpectedPositiveVariable(non_variable));
    }

    #[test]
    fn reports_invalid_references() {
        let (mut manager, x, _, z) = sample_manager();

        let error = manager.compose(BddEdge::regular(99), x, z).unwrap_err();

        assert_eq!(error, BddCompError::MissingNode(99));
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bddcomp.rs");
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
