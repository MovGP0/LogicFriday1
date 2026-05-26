//! Native Rust comparison helpers for CMU BDDs.
//!
//! The legacy routine compares two functions with respect to a pivot variable.
//! It walks cofactors in variable order through that pivot, compares the false
//! branch first, and falls back to satisfying-fraction comparison below the
//! pivot.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type BddVariableId = u32;
pub type BddNodeId = usize;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddEdge
{
    node: BddNodeId,
    complemented: bool,
}

impl BddEdge
{
    pub const fn regular(node: BddNodeId) -> Self
    {
        Self {
            node,
            complemented: false,
        }
    }

    pub const fn complemented(node: BddNodeId) -> Self
    {
        Self {
            node,
            complemented: true,
        }
    }

    pub const fn node(self) -> BddNodeId
    {
        self.node
    }

    pub const fn is_complemented(self) -> bool
    {
        self.complemented
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddNode
{
    Constant(bool),
    Branch
    {
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompareStats
{
    pub calls: usize,
    pub cache_hits: usize,
    pub cache_inserts: usize,
    pub fraction_comparisons: usize,
}

#[derive(Clone, Debug)]
pub struct BddManager
{
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
    compare_cache: HashMap<(BddEdge, BddEdge, BddVariableId), Ordering>,
    fraction_cache: HashMap<BddEdge, f64>,
    recursion_limit: usize,
    stats: CompareStats,
}

impl BddManager
{
    pub fn new() -> Self
    {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            unique_table: HashMap::new(),
            compare_cache: HashMap::new(),
            fraction_cache: HashMap::new(),
            recursion_limit: 65_536,
            stats: CompareStats::default(),
        }
    }

    pub fn zero(&self) -> BddEdge
    {
        BddEdge::regular(0)
    }

    pub fn one(&self) -> BddEdge
    {
        BddEdge::regular(1)
    }

    pub fn stats(&self) -> CompareStats
    {
        self.stats
    }

    pub fn cache_len(&self) -> usize
    {
        self.compare_cache.len()
    }

    pub fn set_recursion_limit(&mut self, recursion_limit: usize)
    {
        self.recursion_limit = recursion_limit;
    }

    pub fn node(&self, edge: BddEdge) -> Result<&BddNode, CompareError>
    {
        self.nodes
            .get(edge.node)
            .ok_or(CompareError::MissingNode(edge.node))
    }

    pub fn variable(&mut self, variable: BddVariableId) -> BddEdge
    {
        self.find_or_add(variable, self.one(), self.zero())
            .expect("variable nodes use ordered constant children")
    }

    pub fn find_or_add(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, CompareError>
    {
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.validate_order(variable, then_edge)?;
        self.validate_order(variable, else_edge)?;

        Ok(self.find_or_add_unchecked(variable, then_edge, else_edge))
    }

    pub fn compare(
        &mut self,
        first: BddEdge,
        second: BddEdge,
        pivot_variable: BddEdge,
    ) -> Result<Ordering, CompareError>
    {
        self.validate_edge(first)?;
        self.validate_edge(second)?;
        self.validate_edge(pivot_variable)?;

        let pivot_variable = self.positive_variable_id(pivot_variable)?;
        self.compare_cache.clear();
        self.fraction_cache.clear();
        self.compare_step(first, second, pivot_variable, 0)
    }

    pub fn compare_temp(
        &mut self,
        first: BddEdge,
        second: BddEdge,
        pivot_variable: BddEdge,
    ) -> Result<Ordering, CompareError>
    {
        self.validate_edge(first)?;
        self.validate_edge(second)?;
        self.validate_edge(pivot_variable)?;

        let pivot_variable = self.sort_variable(pivot_variable)?;
        self.compare_step(first, second, pivot_variable, 0)
    }

    pub fn satisfying_fraction(&mut self, edge: BddEdge) -> Result<f64, CompareError>
    {
        self.validate_edge(edge)?;
        self.satisfying_fraction_step(edge)
    }

    pub fn eval(
        &self,
        root: BddEdge,
        assignment: &HashMap<BddVariableId, bool>,
    ) -> Result<bool, CompareError>
    {
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

    pub fn not(&self, edge: BddEdge) -> BddEdge
    {
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

    fn compare_step(
        &mut self,
        first: BddEdge,
        second: BddEdge,
        pivot_variable: BddVariableId,
        depth: usize,
    ) -> Result<Ordering, CompareError>
    {
        if depth > self.recursion_limit {
            return Err(CompareError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        self.stats.calls += 1;

        if first == second {
            return Ok(Ordering::Equal);
        }

        if self.is_constant(first)? || self.is_constant(second)? {
            return self.compare_constants(first, second);
        }

        let cache_key = (first, second, pivot_variable);
        if let Some(result) = self.compare_cache.get(&cache_key).copied() {
            self.stats.cache_hits += 1;
            return Ok(result);
        }

        let top_variable = self
            .sort_variable(first)?
            .min(self.sort_variable(second)?);
        let result = if top_variable > pivot_variable {
            self.stats.fraction_comparisons += 1;
            self.compare_fraction(first, second)?
        } else {
            let (first_then, first_else) = self.cofactor(first, top_variable)?;
            let (second_then, second_else) = self.cofactor(second, top_variable)?;
            let else_result =
                self.compare_step(first_else, second_else, pivot_variable, depth + 1)?;

            if else_result == Ordering::Equal {
                self.compare_step(first_then, second_then, pivot_variable, depth + 1)?
            } else {
                else_result
            }
        };

        self.compare_cache.insert(cache_key, result);
        self.stats.cache_inserts += 1;
        Ok(result)
    }

    fn compare_constants(
        &self,
        first: BddEdge,
        second: BddEdge,
    ) -> Result<Ordering, CompareError>
    {
        if first == self.zero() || second == self.one() {
            Ok(Ordering::Less)
        } else {
            Ok(Ordering::Greater)
        }
    }

    fn compare_fraction(
        &mut self,
        first: BddEdge,
        second: BddEdge,
    ) -> Result<Ordering, CompareError>
    {
        let first_fraction = self.satisfying_fraction_step(first)?;
        let second_fraction = self.satisfying_fraction_step(second)?;

        first_fraction
            .partial_cmp(&second_fraction)
            .ok_or(CompareError::FractionNotComparable)
    }

    fn satisfying_fraction_step(&mut self, edge: BddEdge) -> Result<f64, CompareError>
    {
        if edge == self.zero() {
            return Ok(0.0);
        }

        if edge == self.one() {
            return Ok(1.0);
        }

        if let Some(result) = self.fraction_cache.get(&edge).copied() {
            return Ok(result);
        }

        let (then_edge, else_edge) = self.branches(edge)?;
        let result = 0.5 * self.satisfying_fraction_step(then_edge)?
            + 0.5 * self.satisfying_fraction_step(else_edge)?;
        self.fraction_cache.insert(edge, result);

        Ok(result)
    }

    fn cofactor(
        &self,
        edge: BddEdge,
        variable: BddVariableId,
    ) -> Result<(BddEdge, BddEdge), CompareError>
    {
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

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), CompareError>
    {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Err(CompareError::ExpectedBranch(edge.node)),
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

    fn find_or_add_unchecked(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> BddEdge
    {
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
        self.fraction_cache.clear();

        edge
    }

    fn positive_variable_id(&self, edge: BddEdge) -> Result<BddVariableId, CompareError>
    {
        if edge.is_complemented() {
            return Err(CompareError::ExpectedPositiveVariable(edge));
        }

        match self.node(edge)? {
            BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            } if *then_edge == self.one() && *else_edge == self.zero() => Ok(*variable),
            BddNode::Branch { .. } => Err(CompareError::ExpectedPositiveVariable(edge)),
            BddNode::Constant(_) => Err(CompareError::ExpectedPositiveVariable(edge)),
        }
    }

    fn validate_edge(&self, edge: BddEdge) -> Result<(), CompareError>
    {
        self.nodes
            .get(edge.node)
            .map(|_| ())
            .ok_or(CompareError::MissingNode(edge.node))
    }

    fn validate_order(
        &self,
        parent: BddVariableId,
        child: BddEdge,
    ) -> Result<(), CompareError>
    {
        let child_variable = self.sort_variable(child)?;
        if child_variable == BddVariableId::MAX || parent < child_variable {
            Ok(())
        } else {
            Err(CompareError::VariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn sort_variable(&self, edge: BddEdge) -> Result<BddVariableId, CompareError>
    {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Constant(_) => Ok(BddVariableId::MAX),
            BddNode::Branch { variable, .. } => Ok(*variable),
        }
    }

    fn is_constant(&self, edge: BddEdge) -> Result<bool, CompareError>
    {
        Ok(matches!(
            self.node(BddEdge::regular(edge.node))?,
            BddNode::Constant(_)
        ))
    }
}

impl Default for BddManager
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompareError
{
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    ExpectedPositiveVariable(BddEdge),
    VariableOrder
    {
        parent: BddVariableId,
        child: BddVariableId,
    },
    FractionNotComparable,
    RecursionLimitExceeded
    {
        limit: usize,
    },
}

impl fmt::Display for CompareError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch node"),
            Self::ExpectedPositiveVariable(edge) => write!(
                formatter,
                "cmu_bdd_compare: third argument is not a positive variable, got {edge:?}"
            ),
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
            Self::FractionNotComparable => formatter.write_str("BDD fraction was not comparable"),
            Self::RecursionLimitExceeded { limit } => {
                write!(formatter, "BDD compare recursion limit {limit} was exceeded")
            }
        }
    }
}

impl Error for CompareError {}

#[cfg(test)]
mod tests
{
    use super::*;

    fn values(entries: &[(BddVariableId, bool)]) -> HashMap<BddVariableId, bool>
    {
        entries.iter().copied().collect()
    }

    fn sample_manager() -> (BddManager, BddEdge, BddEdge, BddEdge)
    {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);
        let z = manager.variable(3);

        (manager, x, y, z)
    }

    #[test]
    fn compares_constants_like_legacy_routine()
    {
        let (mut manager, x, _, _) = sample_manager();

        assert_eq!(manager.compare(manager.zero(), manager.one(), x), Ok(Ordering::Less));
        assert_eq!(manager.compare(manager.one(), manager.zero(), x), Ok(Ordering::Greater));
        assert_eq!(manager.compare(manager.zero(), manager.zero(), x), Ok(Ordering::Equal));
    }

    #[test]
    fn compares_false_cofactor_before_true_cofactor()
    {
        let (mut manager, x, y, _) = sample_manager();
        let first = manager.find_or_add(1, manager.zero(), y).unwrap();
        let second = manager.find_or_add(1, y, manager.zero()).unwrap();

        assert_eq!(manager.compare(first, second, x), Ok(Ordering::Greater));
        assert_eq!(manager.compare(second, first, x), Ok(Ordering::Less));
    }

    #[test]
    fn falls_back_to_satisfying_fraction_below_pivot()
    {
        let (mut manager, x, y, z) = sample_manager();
        let y_or_z = manager.find_or_add(2, manager.one(), z).unwrap();

        assert_eq!(manager.satisfying_fraction(y).unwrap(), 0.5);
        assert_eq!(manager.satisfying_fraction(y_or_z).unwrap(), 0.75);
        assert_eq!(manager.compare(y, y_or_z, x), Ok(Ordering::Less));
        assert_eq!(manager.stats().fraction_comparisons, 1);
    }

    #[test]
    fn accepts_complemented_function_edges()
    {
        let (mut manager, x, y, _) = sample_manager();
        let not_y = manager.not(y);

        assert_eq!(manager.compare(not_y, y, x), Ok(Ordering::Equal));
        assert_eq!(manager.compare(not_y, y, y), Ok(Ordering::Greater));
        assert_eq!(manager.eval(not_y, &values(&[(2, false)])), Ok(true));
    }

    #[test]
    fn caches_shared_comparisons()
    {
        let (mut manager, x, y, z) = sample_manager();
        let first = BddEdge::regular(manager.nodes.len());
        manager.nodes.push(BddNode::Branch {
            variable: 1,
            then_edge: y,
            else_edge: y,
        });
        let second = BddEdge::regular(manager.nodes.len());
        manager.nodes.push(BddNode::Branch {
            variable: 1,
            then_edge: z,
            else_edge: z,
        });

        assert_eq!(manager.compare(first, second, x), Ok(Ordering::Equal));
        assert!(manager.stats().cache_hits > 0);
        assert!(manager.cache_len() > 0);
    }

    #[test]
    fn rejects_non_positive_pivot_variable()
    {
        let (mut manager, x, y, _) = sample_manager();
        let non_variable = manager.find_or_add(1, y, manager.zero()).unwrap();

        assert_eq!(
            manager.compare(y, x, non_variable),
            Err(CompareError::ExpectedPositiveVariable(non_variable))
        );
        assert_eq!(
            manager.compare(y, x, manager.not(x)),
            Err(CompareError::ExpectedPositiveVariable(manager.not(x)))
        );
    }

    #[test]
    fn reports_invalid_references()
    {
        let (mut manager, x, _, _) = sample_manager();

        assert_eq!(
            manager.compare(BddEdge::regular(999), manager.one(), x),
            Err(CompareError::MissingNode(999))
        );
    }

    #[test]
    fn recursion_limit_protects_compare_walks()
    {
        let (mut manager, x, y, _z) = sample_manager();
        let first = manager.find_or_add(1, y, manager.zero()).unwrap();
        let second = manager.find_or_add(1, manager.one(), manager.zero()).unwrap();
        manager.set_recursion_limit(0);

        assert_eq!(
            manager.compare(first, second, x),
            Err(CompareError::RecursionLimitExceeded { limit: 0 })
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens()
    {
        let source = include_str!("bddcmp.rs");
        let legacy_export = concat!("no", "_", "mangle");
        let tracking_prefix = concat!("REQUIRED", "_");
        let dependency_type = concat!("Port", "Dependency");
        let bead_token = concat!("bead", "_", "id");
        let source_token = concat!("source", "_", "file");
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
