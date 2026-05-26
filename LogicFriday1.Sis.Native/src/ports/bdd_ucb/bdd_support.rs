//! Native Rust support-set and onset-count helpers for the UCB BDD package.
//!
//! The legacy C routines are read-only walkers over a BDD: one gathers the
//! variables appearing in the graph, the other counts onset minterms relative
//! to a caller-provided variable set.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddVariableId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddNodeId(pub usize);

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

    pub const fn regularized(self) -> Self {
        Self::regular(self.node)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddBranch {
    pub variable: BddVariableId,
    pub then_edge: BddEdge,
    pub else_edge: BddEdge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddNode {
    Constant(bool),
    Branch(BddBranch),
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    variable_count: usize,
}

impl BddManager {
    pub fn new(variable_count: usize) -> Self {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            variable_count,
        }
    }

    pub fn variable_count(&self) -> usize {
        self.variable_count
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::regular(BddNodeId(0))
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::regular(BddNodeId(1))
    }

    pub fn add_branch(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, BddSupportError> {
        if variable.0 >= self.variable_count {
            return Err(BddSupportError::VariableOutOfRange {
                variable,
                variable_count: self.variable_count,
            });
        }

        self.node(then_edge.regularized())?;
        self.node(else_edge.regularized())?;

        let edge = BddEdge::regular(BddNodeId(self.nodes.len()));
        self.nodes.push(BddNode::Branch(BddBranch {
            variable,
            then_edge,
            else_edge,
        }));

        Ok(edge)
    }

    pub fn node(&self, edge: BddEdge) -> Result<BddNode, BddSupportError> {
        self.nodes
            .get(edge.node.0)
            .copied()
            .ok_or(BddSupportError::MissingNode(edge.node))
    }

    fn constant_value(&self, edge: BddEdge) -> Result<Option<bool>, BddSupportError> {
        match self.node(edge.regularized())? {
            BddNode::Constant(value) => Ok(Some(value ^ edge.is_complemented())),
            BddNode::Branch(_) => Ok(None),
        }
    }

    fn branch(&self, edge: BddEdge) -> Result<BddBranch, BddSupportError> {
        match self.node(edge.regularized())? {
            BddNode::Constant(_) => Err(BddSupportError::ExpectedBranch(edge.node)),
            BddNode::Branch(branch) if edge.is_complemented() => Ok(BddBranch {
                variable: branch.variable,
                then_edge: branch.then_edge.not(),
                else_edge: branch.else_edge.not(),
            }),
            BddNode::Branch(branch) => Ok(branch),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VarSet {
    elements: Vec<bool>,
}

impl VarSet {
    pub fn new(variable_count: usize) -> Self {
        Self {
            elements: vec![false; variable_count],
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn contains(&self, variable: BddVariableId) -> bool {
        self.elements.get(variable.0).copied().unwrap_or(false)
    }

    pub fn variables(&self) -> Vec<BddVariableId> {
        self.elements
            .iter()
            .enumerate()
            .filter_map(|(index, present)| present.then_some(BddVariableId(index)))
            .collect()
    }

    fn set(&mut self, variable: BddVariableId) -> Result<(), BddSupportError> {
        let Some(slot) = self.elements.get_mut(variable.0) else {
            return Err(BddSupportError::VariableOutOfRange {
                variable,
                variable_count: self.elements.len(),
            });
        };

        *slot = true;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BddSupportError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    VariableOutOfRange {
        variable: BddVariableId,
        variable_count: usize,
    },
    FunctionVariableMissing {
        variable: BddVariableId,
    },
}

impl fmt::Display for BddSupportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {:?} was not found", node),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {:?} is not a branch", node),
            Self::VariableOutOfRange {
                variable,
                variable_count,
            } => write!(
                formatter,
                "BDD variable {:?} is outside the manager range 0..{variable_count}",
                variable
            ),
            Self::FunctionVariableMissing { variable } => write!(
                formatter,
                "BDD variable {:?} appears in the function but not in the counting variable set",
                variable
            ),
        }
    }
}

impl Error for BddSupportError {}

pub fn get_support(manager: &BddManager, root: BddEdge) -> Result<VarSet, BddSupportError> {
    let mut result = VarSet::new(manager.variable_count());
    let mut visited = HashSet::new();

    extract_support(manager, root, &mut result, &mut visited)?;

    Ok(result)
}

pub fn count_onset(
    manager: &BddManager,
    root: BddEdge,
    variables: &[BddVariableId],
) -> Result<f64, BddSupportError> {
    let mut variables = variables.to_vec();
    variables.sort_unstable();

    if variables.is_empty() {
        return Ok(0.0);
    }

    let mut visited = HashMap::new();

    count_onset_inner(manager, root, 0, &variables, &mut visited)
}

fn extract_support(
    manager: &BddManager,
    root: BddEdge,
    result: &mut VarSet,
    visited: &mut HashSet<BddEdge>,
) -> Result<(), BddSupportError> {
    if manager.constant_value(root)?.is_some() {
        return Ok(());
    }

    if !visited.insert(root) {
        return Ok(());
    }

    let branch = manager.branch(root)?;

    result.set(branch.variable)?;
    extract_support(manager, branch.then_edge, result, visited)?;
    extract_support(manager, branch.else_edge, result, visited)?;

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CountCache {
    count: f64,
    index: usize,
}

fn count_onset_inner(
    manager: &BddManager,
    root: BddEdge,
    top_index: usize,
    variables: &[BddVariableId],
    visited: &mut HashMap<BddEdge, CountCache>,
) -> Result<f64, BddSupportError> {
    if let Some(value) = visited.get(&root) {
        debug_assert!(top_index <= value.index);
        return Ok(value.count * two_to(value.index - top_index));
    }

    if let Some(value) = manager.constant_value(root)? {
        return if value {
            Ok(two_to(variables.len().saturating_sub(top_index)))
        } else {
            Ok(0.0)
        };
    }

    let branch = manager.branch(root)?;
    let Some(top_variable) = variables.get(top_index).copied() else {
        return Err(BddSupportError::FunctionVariableMissing {
            variable: branch.variable,
        });
    };

    if top_variable > branch.variable {
        return Err(BddSupportError::FunctionVariableMissing {
            variable: branch.variable,
        });
    }

    if top_variable == branch.variable {
        let count =
            count_onset_inner(manager, branch.then_edge, top_index + 1, variables, visited)?
                + count_onset_inner(manager, branch.else_edge, top_index + 1, variables, visited)?;

        visited.insert(
            root,
            CountCache {
                count,
                index: top_index,
            },
        );

        Ok(count)
    } else {
        Ok(2.0 * count_onset_inner(manager, root, top_index + 1, variables, visited)?)
    }
}

fn two_to(power: usize) -> f64 {
    2_f64.powi(power as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager_for_xy() -> (BddManager, BddEdge, BddEdge) {
        let mut manager = BddManager::new(4);
        let y = manager
            .add_branch(BddVariableId(1), manager.one(), manager.zero())
            .unwrap();
        let x = manager
            .add_branch(BddVariableId(0), y, manager.zero())
            .unwrap();

        (manager, x, y)
    }

    #[test]
    fn support_contains_variables_reachable_from_both_branches() {
        let (manager, root, _) = manager_for_xy();

        let support = get_support(&manager, root).unwrap();

        assert_eq!(support.len(), 4);
        assert_eq!(
            support.variables(),
            vec![BddVariableId(0), BddVariableId(1)]
        );
        assert!(support.contains(BddVariableId(0)));
        assert!(support.contains(BddVariableId(1)));
        assert!(!support.contains(BddVariableId(2)));
    }

    #[test]
    fn support_ignores_constants_and_visits_shared_nodes_once() {
        let mut manager = BddManager::new(3);
        let y = manager
            .add_branch(BddVariableId(1), manager.one(), manager.zero())
            .unwrap();
        let root = manager.add_branch(BddVariableId(0), y, y).unwrap();

        let support = get_support(&manager, root).unwrap();

        assert_eq!(
            support.variables(),
            vec![BddVariableId(0), BddVariableId(1)]
        );
        assert!(
            get_support(&manager, manager.one())
                .unwrap()
                .variables()
                .is_empty()
        );
    }

    #[test]
    fn support_handles_complemented_edges_without_changing_variables() {
        let (manager, root, _) = manager_for_xy();

        let regular = get_support(&manager, root).unwrap();
        let complemented = get_support(&manager, root.not()).unwrap();

        assert_eq!(regular, complemented);
    }

    #[test]
    fn count_onset_returns_zero_when_variable_set_is_empty() {
        let (manager, root, _) = manager_for_xy();

        assert_eq!(count_onset(&manager, root, &[]).unwrap(), 0.0);
    }

    #[test]
    fn count_onset_counts_minterms_over_sorted_variables() {
        let (manager, root, _) = manager_for_xy();

        let count = count_onset(
            &manager,
            root,
            &[BddVariableId(1), BddVariableId(0), BddVariableId(2)],
        )
        .unwrap();

        assert_eq!(count, 2.0);
    }

    #[test]
    fn count_onset_accounts_for_variables_absent_from_function() {
        let (manager, _, y) = manager_for_xy();

        let count = count_onset(
            &manager,
            y,
            &[BddVariableId(0), BddVariableId(1), BddVariableId(2)],
        )
        .unwrap();

        assert_eq!(count, 4.0);
    }

    #[test]
    fn count_onset_handles_complemented_roots() {
        let (manager, root, _) = manager_for_xy();

        let variables = [BddVariableId(0), BddVariableId(1)];

        assert_eq!(count_onset(&manager, root, &variables).unwrap(), 1.0);
        assert_eq!(count_onset(&manager, root.not(), &variables).unwrap(), 3.0);
    }

    #[test]
    fn count_onset_rejects_function_variable_missing_from_counting_set() {
        let (manager, root, _) = manager_for_xy();

        let error = count_onset(&manager, root, &[BddVariableId(1)]).unwrap_err();

        assert_eq!(
            error,
            BddSupportError::FunctionVariableMissing {
                variable: BddVariableId(0),
            }
        );
    }

    #[test]
    fn add_branch_validates_variables_and_children() {
        let mut manager = BddManager::new(1);

        assert_eq!(
            manager.add_branch(BddVariableId(2), manager.one(), manager.zero()),
            Err(BddSupportError::VariableOutOfRange {
                variable: BddVariableId(2),
                variable_count: 1,
            })
        );
        assert_eq!(
            manager.add_branch(
                BddVariableId(0),
                BddEdge::regular(BddNodeId(99)),
                manager.zero(),
            ),
            Err(BddSupportError::MissingNode(BddNodeId(99)))
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bdd_support.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
