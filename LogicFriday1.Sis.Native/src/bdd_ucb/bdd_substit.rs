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
        Self {
            node: self.node,
            complemented: !self.complemented,
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
pub struct SubstituteReturnStats {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SubstituteStats {
    pub calls: usize,
    pub returns: SubstituteReturnStats,
    pub cache_inserts: usize,
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
    substitution_cache: HashMap<(BddEdge, usize), BddEdge>,
    stats: SubstituteStats,
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            unique_table: HashMap::new(),
            substitution_cache: HashMap::new(),
            stats: SubstituteStats::default(),
        }
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::regular(0)
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::regular(1)
    }

    pub fn stats(&self) -> SubstituteStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn cache_len(&self) -> usize {
        self.substitution_cache.len()
    }

    pub fn node(&self, edge: BddEdge) -> Result<&BddNode, SubstituteError> {
        self.nodes
            .get(edge.node)
            .ok_or(SubstituteError::MissingNode(edge.node))
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
    ) -> Result<BddEdge, SubstituteError> {
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
    ) -> Result<BddEdge, SubstituteError> {
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.ite_inner(condition, then_edge, else_edge)
    }

    pub fn substitute(
        &mut self,
        root: BddEdge,
        old_variables: &[BddEdge],
        new_variables: &[BddEdge],
    ) -> Result<BddEdge, SubstituteError> {
        self.validate_edge(root)?;
        if old_variables.len() != new_variables.len() {
            return Err(SubstituteError::VariableCountMismatch {
                old: old_variables.len(),
                new: new_variables.len(),
            });
        }

        let mut pairs = old_variables
            .iter()
            .copied()
            .zip(new_variables.iter().copied())
            .map(|(old, new)| {
                Ok((
                    self.variable_id_from_function(old)?,
                    self.variable_id_from_function(new)?,
                ))
            })
            .collect::<Result<Vec<_>, SubstituteError>>()?;

        pairs.sort_by_key(|(old, _)| *old);

        let mut unique = Vec::with_capacity(pairs.len());
        for (old, new) in pairs {
            if let Some((_, existing_new)) =
                unique.last_mut().filter(|(seen_old, _)| *seen_old == old)
            {
                *existing_new = new;
            } else {
                unique.push((old, new));
            }
        }

        let old_ids = unique.iter().map(|(old, _)| *old).collect::<Vec<_>>();
        let new_ids = unique.iter().map(|(_, new)| *new).collect::<Vec<_>>();

        self.substitution_cache.clear();
        self.substitute_inner(root, 0, &old_ids, &new_ids)
    }

    pub fn eval(
        &self,
        root: BddEdge,
        assignment: &HashMap<BddVariableId, bool>,
    ) -> Result<bool, SubstituteError> {
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

    fn substitute_inner(
        &mut self,
        root: BddEdge,
        old_index: usize,
        old_variables: &[BddVariableId],
        new_variables: &[BddVariableId],
    ) -> Result<BddEdge, SubstituteError> {
        self.stats.calls += 1;

        if old_index >= old_variables.len() {
            self.stats.returns.trivial += 1;
            return Ok(root);
        }

        let root_variable = self.sort_variable(root)?;
        let last_variable = old_variables[old_variables.len() - 1];
        if root_variable > last_variable {
            self.stats.returns.trivial += 1;
            return Ok(root);
        }

        let cache_key = (root, old_index);
        if let Some(cached) = self.substitution_cache.get(&cache_key).copied() {
            self.stats.returns.cached += 1;
            return Ok(cached);
        }

        let top_variable = old_variables[old_index];
        let result = if root_variable > top_variable {
            self.substitute_inner(root, old_index + 1, old_variables, new_variables)?
        } else {
            let (next_index, new_variable) = if root_variable < top_variable {
                (old_index, root_variable)
            } else {
                (old_index + 1, new_variables[old_index])
            };

            let (then_edge, else_edge) = self.branches(root)?;
            let substituted_then =
                self.substitute_inner(then_edge, next_index, old_variables, new_variables)?;
            let substituted_else =
                self.substitute_inner(else_edge, next_index, old_variables, new_variables)?;
            let replacement_variable =
                self.find_or_add_unchecked(new_variable, self.one(), self.zero());

            self.ite(replacement_variable, substituted_then, substituted_else)?
        };

        self.substitution_cache.insert(cache_key, result);
        self.stats.cache_inserts += 1;
        self.stats.returns.full += 1;
        Ok(result)
    }

    fn ite_inner(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, SubstituteError> {
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
    ) -> Result<(BddEdge, BddEdge), SubstituteError> {
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

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), SubstituteError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Err(SubstituteError::ExpectedBranch(edge.node)),
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

    fn validate_edge(&self, edge: BddEdge) -> Result<(), SubstituteError> {
        self.nodes
            .get(edge.node)
            .map(|_| ())
            .ok_or(SubstituteError::MissingNode(edge.node))
    }

    fn validate_order(&self, parent: BddVariableId, child: BddEdge) -> Result<(), SubstituteError> {
        let child_variable = self.sort_variable(child)?;
        if child_variable == BddVariableId::MAX || parent < child_variable {
            Ok(())
        } else {
            Err(SubstituteError::VariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn variable_id_from_function(&self, edge: BddEdge) -> Result<BddVariableId, SubstituteError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            } if *then_edge == self.one() && *else_edge == self.zero() => Ok(*variable),
            BddNode::Branch { .. } => Err(SubstituteError::ExpectedVariableFunction(edge)),
            BddNode::Constant(_) => Err(SubstituteError::ExpectedVariableFunction(edge)),
        }
    }

    fn sort_variable(&self, edge: BddEdge) -> Result<BddVariableId, SubstituteError> {
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
pub enum SubstituteError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    ExpectedVariableFunction(BddEdge),
    VariableCountMismatch {
        old: usize,
        new: usize,
    },
    VariableOrder {
        parent: BddVariableId,
        child: BddVariableId,
    },
}

impl fmt::Display for SubstituteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch node"),
            Self::ExpectedVariableFunction(edge) => {
                write!(formatter, "BDD edge {edge:?} is not a variable function")
            }
            Self::VariableCountMismatch { old, new } => write!(
                formatter,
                "bdd_substitute: mismatch of number of new and old variables ({old} != {new})"
            ),
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
        }
    }
}

impl Error for SubstituteError {}

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
    fn substitutes_one_variable_in_single_pass() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(x, y, z).unwrap();

        let result = manager.substitute(expression, &[x], &[z]).unwrap();

        for x_value in [false, true] {
            for y_value in [false, true] {
                for z_value in [false, true] {
                    let assignment = values(&[(1, x_value), (2, y_value), (3, z_value)]);
                    assert_eq!(
                        manager.eval(result, &assignment).unwrap(),
                        if z_value { y_value } else { z_value }
                    );
                }
            }
        }
    }

    #[test]
    fn sorts_old_variables_while_preserving_new_variable_correspondence() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(x, y, manager.zero()).unwrap();

        let result = manager.substitute(expression, &[y, x], &[x, z]).unwrap();

        for x_value in [false, true] {
            for y_value in [false, true] {
                for z_value in [false, true] {
                    let assignment = values(&[(1, x_value), (2, y_value), (3, z_value)]);
                    assert_eq!(
                        manager.eval(result, &assignment).unwrap(),
                        z_value && x_value
                    );
                }
            }
        }
    }

    #[test]
    fn returns_original_when_all_remaining_substitutions_are_below_root() {
        let (mut manager, x, y, _) = sample_manager();

        let result = manager.substitute(y, &[x], &[y]).unwrap();

        assert_eq!(result, y);
        assert_eq!(manager.stats().returns.trivial, 1);
        assert_eq!(manager.cache_len(), 0);
    }

    #[test]
    fn keeps_unsubstituted_variables_and_recurses_past_lower_substitution_ids() {
        let (mut manager, x, y, z) = sample_manager();
        let expression = manager.ite(y, x, z).unwrap();

        let result = manager.substitute(expression, &[x, z], &[z, x]).unwrap();

        for y_value in [false, true] {
            let assignment = values(&[(1, false), (2, y_value), (3, true)]);
            assert_eq!(manager.eval(result, &assignment).unwrap(), y_value);
        }
    }

    #[test]
    fn uses_adhoc_style_cache_for_shared_subgraphs() {
        let (mut manager, x, y, z) = sample_manager();
        let shared = manager.ite(y, z, manager.zero()).unwrap();

        let result = manager
            .substitute_inner(shared, 0, &[3], &[1])
            .expect("first recursive substitution should succeed");
        let cached = manager
            .substitute_inner(shared, 0, &[3], &[1])
            .expect("second recursive substitution should reuse cache");

        assert_eq!(cached, result);
        assert_eq!(result, manager.ite(y, x, manager.zero()).unwrap());
        assert!(manager.stats().returns.cached > 0);
        assert!(manager.cache_len() > 0);
    }

    #[test]
    fn rejects_mismatched_variable_arrays() {
        let (mut manager, x, y, _) = sample_manager();

        let error = manager.substitute(x, &[x, y], &[y]).unwrap_err();

        assert_eq!(
            error,
            SubstituteError::VariableCountMismatch { old: 2, new: 1 }
        );
    }

    #[test]
    fn rejects_non_variable_substitution_entries() {
        let (mut manager, x, y, _) = sample_manager();
        let expression = manager.ite(x, y, manager.zero()).unwrap();

        let error = manager.substitute(x, &[expression], &[y]).unwrap_err();

        assert_eq!(error, SubstituteError::ExpectedVariableFunction(expression));
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bdd_substit.rs");
        let legacy_export = concat!("no", "_", "mangle");
        let tracking_prefix = concat!("REQUIRED", "_");
        let dependency_type = concat!("Port", "Dependency");
        let bead_token = concat!("bead", "_", "id");
        let source_token = concat!("source", "_", "file");
        let bead_prefix = concat!("LogicFriday", "1-", "8j8");

        assert!(!source.contains(legacy_export));
        assert!(!source.contains("extern \"C\""));
        assert!(!source.contains(tracking_prefix));
        assert!(!source.contains(dependency_type));
        assert!(!source.contains(bead_token));
        assert!(!source.contains(source_token));
        assert!(!source.contains(bead_prefix));
    }
}
