//! Native Rust model of the SIS UCB BDD compatible projection routine.
//!
//! Compatible projection keeps one satisfying assignment for each assignment
//! over the variables outside the projection set. The legacy routine uses the
//! all-ones reference vertex when it has a choice; this port preserves that
//! branch preference while exposing owned Rust data structures.

use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

pub type BddVariableId = u32;
pub type BddNodeId = usize;

pub const CPROJECT_CACHE_OFFSET: usize = 1_000_000;

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

    pub const fn regularized(self) -> Self {
        Self::regular(self.node)
    }

    pub const fn not(self) -> Self {
        Self {
            node: self.node,
            complemented: !self.complemented,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddNode {
    Constant(bool),
    Branch {
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProjectionReturnStats {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProjectionStats {
    pub calls: usize,
    pub returns: ProjectionReturnStats,
    pub cache_inserts: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct CacheKey {
    function: BddEdge,
    namespace: usize,
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
    projection_cache: HashMap<CacheKey, BddEdge>,
    stats: ProjectionStats,
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            unique_table: HashMap::new(),
            projection_cache: HashMap::new(),
            stats: ProjectionStats::default(),
        }
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::regular(0)
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::regular(1)
    }

    pub fn stats(&self) -> ProjectionStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn cache_len(&self) -> usize {
        self.projection_cache.len()
    }

    pub fn node(&self, edge: BddEdge) -> Result<BddNode, CProjectError> {
        self.nodes
            .get(edge.node)
            .copied()
            .ok_or(CProjectError::MissingNode(edge.node))
    }

    pub fn variable(&mut self, variable: BddVariableId) -> BddEdge {
        self.find_or_add_unchecked(variable, self.one(), self.zero())
    }

    pub fn find_or_add(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, CProjectError> {
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
    ) -> Result<BddEdge, CProjectError> {
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.ite_inner(condition, then_edge, else_edge)
    }

    pub fn compatible_project(
        &mut self,
        function: BddEdge,
        variables: &[BddEdge],
    ) -> Result<BddEdge, CProjectError> {
        self.validate_edge(function)?;

        let variables = self.sorted_variable_ids(variables)?;
        self.projection_cache.clear();
        self.cproject(function, 0, &variables)
    }

    pub fn exists(
        &mut self,
        function: BddEdge,
        variables: &[BddVariableId],
    ) -> Result<BddEdge, CProjectError> {
        self.validate_edge(function)?;
        let variables = sorted_unique_ids(variables);
        self.exists_inner(function, 0, &variables)
    }

    pub fn eval(
        &self,
        root: BddEdge,
        assignment: &HashMap<BddVariableId, bool>,
    ) -> Result<bool, CProjectError> {
        let mut edge = root;
        let mut complemented = false;

        loop {
            complemented ^= edge.is_complemented();
            match self.node(edge.regularized())? {
                BddNode::Constant(value) => return Ok(value ^ complemented),
                BddNode::Branch {
                    variable,
                    then_edge,
                    else_edge,
                } => {
                    edge = if assignment.get(&variable).copied().unwrap_or(false) {
                        then_edge
                    } else {
                        else_edge
                    };
                }
            }
        }
    }

    fn cproject(
        &mut self,
        function: BddEdge,
        index: usize,
        variables: &[BddVariableId],
    ) -> Result<BddEdge, CProjectError> {
        self.stats.calls += 1;

        if index >= variables.len() || function == self.zero() {
            self.stats.returns.trivial += 1;
            return Ok(function);
        }

        let cache_key = CacheKey {
            function,
            namespace: index + CPROJECT_CACHE_OFFSET,
        };

        if let Some(result) = self.projection_cache.get(&cache_key).copied() {
            self.stats.returns.cached += 1;
            return Ok(result);
        }

        let top_variable = variables[index];
        let top_variable_edge = self.variable(top_variable);
        let result = match self.variable_id(function)? {
            None => {
                let projection = self.cproject(function, index + 1, variables)?;
                self.ite(top_variable_edge, projection, self.zero())?
            }
            Some(function_variable) if function_variable > top_variable => {
                let projection = self.cproject(function, index + 1, variables)?;
                self.ite(top_variable_edge, projection, self.zero())?
            }
            Some(function_variable) if function_variable == top_variable => {
                let (then_edge, else_edge) = self.branches(function)?;
                let smoothed = self.exists_inner(then_edge, index + 1, variables)?;

                if smoothed == self.one() {
                    let projection = self.cproject(then_edge, index + 1, variables)?;
                    self.ite(top_variable_edge, projection, self.zero())?
                } else if smoothed == self.zero() {
                    let projection = self.cproject(else_edge, index + 1, variables)?;
                    self.ite(top_variable_edge, self.zero(), projection)?
                } else {
                    let then_projection = self.cproject(then_edge, index + 1, variables)?;
                    let else_projection = self.cproject(else_edge, index + 1, variables)?;
                    let guarded_else = self.ite(smoothed, self.zero(), else_projection)?;
                    self.ite(top_variable_edge, then_projection, guarded_else)?
                }
            }
            Some(function_variable) => {
                let (then_edge, else_edge) = self.branches(function)?;
                let then_projection = self.cproject(then_edge, index, variables)?;
                let else_projection = self.cproject(else_edge, index, variables)?;
                let local_variable = self.variable(function_variable);
                self.ite(local_variable, then_projection, else_projection)?
            }
        };

        self.projection_cache.insert(cache_key, result);
        self.stats.cache_inserts += 1;
        self.stats.returns.full += 1;
        Ok(result)
    }

    fn exists_inner(
        &mut self,
        function: BddEdge,
        index: usize,
        variables: &[BddVariableId],
    ) -> Result<BddEdge, CProjectError> {
        if index >= variables.len() || self.is_constant(function)? {
            return Ok(function);
        }

        let Some(function_variable) = self.variable_id(function)? else {
            return Ok(function);
        };

        let variable = variables[index];
        if function_variable > variable {
            return self.exists_inner(function, index + 1, variables);
        }

        let (then_edge, else_edge) = self.branches(function)?;
        if function_variable == variable {
            let then_result = self.exists_inner(then_edge, index + 1, variables)?;
            let else_result = self.exists_inner(else_edge, index + 1, variables)?;
            self.ite(then_result, self.one(), else_result)
        } else {
            let then_result = self.exists_inner(then_edge, index, variables)?;
            let else_result = self.exists_inner(else_edge, index, variables)?;
            Ok(self.find_or_add_unchecked(function_variable, then_result, else_result))
        }
    }

    fn ite_inner(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, CProjectError> {
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

        let variable = [
            self.variable_id(condition)?,
            self.variable_id(then_edge)?,
            self.variable_id(else_edge)?,
        ]
        .into_iter()
        .flatten()
        .min()
        .expect("non-trivial ITE always has at least one branch");

        let (condition_then, condition_else) = self.quick_cofactor(condition, variable)?;
        let (then_then, then_else) = self.quick_cofactor(then_edge, variable)?;
        let (else_then, else_else) = self.quick_cofactor(else_edge, variable)?;

        let high = self.ite_inner(condition_then, then_then, else_then)?;
        let low = self.ite_inner(condition_else, then_else, else_else)?;
        Ok(self.find_or_add_unchecked(variable, high, low))
    }

    fn quick_cofactor(
        &self,
        edge: BddEdge,
        variable: BddVariableId,
    ) -> Result<(BddEdge, BddEdge), CProjectError> {
        match self.node(edge.regularized())? {
            BddNode::Branch {
                variable: node_variable,
                then_edge,
                else_edge,
            } if node_variable == variable && edge.is_complemented() => {
                Ok((then_edge.not(), else_edge.not()))
            }
            BddNode::Branch {
                variable: node_variable,
                then_edge,
                else_edge,
            } if node_variable == variable => Ok((then_edge, else_edge)),
            _ => Ok((edge, edge)),
        }
    }

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), CProjectError> {
        match self.node(edge.regularized())? {
            BddNode::Constant(_) => Err(CProjectError::ExpectedBranch(edge.node)),
            BddNode::Branch {
                then_edge,
                else_edge,
                ..
            } if edge.is_complemented() => Ok((then_edge.not(), else_edge.not())),
            BddNode::Branch {
                then_edge,
                else_edge,
                ..
            } => Ok((then_edge, else_edge)),
        }
    }

    fn variable_id(&self, edge: BddEdge) -> Result<Option<BddVariableId>, CProjectError> {
        match self.node(edge.regularized())? {
            BddNode::Constant(_) => Ok(None),
            BddNode::Branch { variable, .. } => Ok(Some(variable)),
        }
    }

    fn is_constant(&self, edge: BddEdge) -> Result<bool, CProjectError> {
        Ok(matches!(
            self.node(edge.regularized())?,
            BddNode::Constant(_)
        ))
    }

    fn sorted_variable_ids(
        &self,
        variables: &[BddEdge],
    ) -> Result<Vec<BddVariableId>, CProjectError> {
        variables
            .iter()
            .copied()
            .map(|variable| {
                if variable.is_complemented() {
                    return Err(CProjectError::ComplementedVariable(variable));
                }

                match self.node(variable)? {
                    BddNode::Branch {
                        variable: variable_id,
                        then_edge,
                        else_edge,
                    } if then_edge == self.one() && else_edge == self.zero() => Ok(variable_id),
                    BddNode::Branch { .. } => {
                        Err(CProjectError::ExpectedPositiveVariable(variable))
                    }
                    BddNode::Constant(_) => Err(CProjectError::ExpectedPositiveVariable(variable)),
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|ids| sorted_unique_ids(&ids))
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
        if let Some(edge) = self.unique_table.get(&key).copied() {
            return edge;
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

    fn validate_edge(&self, edge: BddEdge) -> Result<(), CProjectError> {
        self.node(edge.regularized()).map(|_| ())
    }

    fn validate_order(&self, parent: BddVariableId, child: BddEdge) -> Result<(), CProjectError> {
        match self.variable_id(child)? {
            Some(child_variable) if parent >= child_variable => Err(CProjectError::VariableOrder {
                parent,
                child: child_variable,
            }),
            _ => Ok(()),
        }
    }
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CProjectError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    ExpectedPositiveVariable(BddEdge),
    ComplementedVariable(BddEdge),
    VariableOrder {
        parent: BddVariableId,
        child: BddVariableId,
    },
}

impl fmt::Display for CProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch"),
            Self::ExpectedPositiveVariable(edge) => {
                write!(formatter, "BDD edge {edge:?} is not a positive variable")
            }
            Self::ComplementedVariable(edge) => {
                write!(formatter, "BDD variable edge {edge:?} is complemented")
            }
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
        }
    }
}

impl Error for CProjectError {}

fn sorted_unique_ids(ids: &[BddVariableId]) -> Vec<BddVariableId> {
    ids.iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projecting_no_variables_returns_original_function() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);

        let result = manager.compatible_project(x, &[]).unwrap();

        assert_eq!(result, x);
        assert_eq!(manager.stats().returns.trivial, 1);
    }

    #[test]
    fn zero_is_preserved_for_any_projection_set() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);

        let result = manager.compatible_project(manager.zero(), &[x]).unwrap();

        assert_eq!(result, manager.zero());
    }

    #[test]
    fn independent_function_sets_projected_variable_to_one() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.compatible_project(y, &[x]).unwrap();

        assert_eq!(result, manager.find_or_add(0, y, manager.zero()).unwrap());
        assert!(
            manager
                .eval(result, &assignment(&[(0, true), (1, true)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result, &assignment(&[(0, false), (1, true)]))
                .unwrap()
        );
    }

    #[test]
    fn reference_vertex_prefers_all_ones_remaining_assignment_when_possible() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let function = manager.ite(x, manager.one(), y).unwrap();

        let result = manager.compatible_project(function, &[x, y]).unwrap();

        assert_eq!(result, manager.find_or_add(0, y, manager.zero()).unwrap());
        assert!(
            manager
                .eval(result, &assignment(&[(0, true), (1, true)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result, &assignment(&[(0, false), (1, true)]))
                .unwrap()
        );
    }

    #[test]
    fn else_branch_is_used_when_then_branch_cannot_satisfy_remaining_variables() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let function = manager.ite(x, manager.zero(), y).unwrap();

        let result = manager.compatible_project(function, &[x, y]).unwrap();

        assert_eq!(result, manager.find_or_add(0, manager.zero(), y).unwrap());
        assert!(
            manager
                .eval(result, &assignment(&[(0, false), (1, true)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result, &assignment(&[(0, true), (1, true)]))
                .unwrap()
        );
    }

    #[test]
    fn partial_projection_preserves_unprojected_decisions() {
        let mut manager = BddManager::new();
        let a = manager.variable(0);
        let b = manager.variable(1);
        let c = manager.variable(2);
        let branch = manager.ite(b, c, manager.zero()).unwrap();
        let function = manager.ite(a, branch, c).unwrap();

        let result = manager.compatible_project(function, &[b]).unwrap();

        assert!(
            manager
                .eval(result, &assignment(&[(0, true), (1, true), (2, true)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result, &assignment(&[(0, true), (1, false), (2, true)]))
                .unwrap()
        );
        assert!(
            manager
                .eval(result, &assignment(&[(0, false), (1, true), (2, true)]))
                .unwrap()
        );
    }

    #[test]
    fn variable_list_is_sorted_and_deduplicated() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let function = manager.ite(x, y, manager.zero()).unwrap();

        let sorted = manager.compatible_project(function, &[y, x, y]).unwrap();
        let already_sorted = manager.compatible_project(function, &[x, y]).unwrap();

        assert_eq!(sorted, already_sorted);
    }

    #[test]
    fn projection_cache_records_recursive_reuse() {
        let mut manager = BddManager::new();
        let a = manager.variable(0);
        let b = manager.variable(1);
        let c = manager.variable(2);
        let shared = manager.ite(c, manager.one(), manager.zero()).unwrap();
        let low = manager.ite(b, shared, manager.zero()).unwrap();
        let function = manager.ite(a, shared, low).unwrap();

        let _ = manager.compatible_project(function, &[c]).unwrap();

        assert!(manager.stats().cache_inserts > 0);
        assert!(manager.cache_len() > 0);
    }

    #[test]
    fn invalid_projection_variable_is_rejected() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let not_x = x.not();

        let error = manager.compatible_project(x, &[not_x]).unwrap_err();

        assert_eq!(error, CProjectError::ComplementedVariable(not_x));
    }

    #[test]
    fn source_contains_no_c_abi_or_dependency_metadata() {
        let source = include_str!("bdd_cproject.rs");

        for forbidden in [
            concat!("no", "_mangle"),
            concat!("extern ", "\"", "C", "\""),
            concat!("REQUIRED", "_"),
            concat!("Port", "Dependency"),
            concat!("bead", "_id"),
            concat!("source", "_file"),
            concat!("Logic", "Friday1", "-", "8j8"),
        ] {
            assert!(!source.contains(forbidden), "{forbidden}");
        }
    }

    fn assignment(values: &[(BddVariableId, bool)]) -> HashMap<BddVariableId, bool> {
        values.iter().copied().collect()
    }
}
