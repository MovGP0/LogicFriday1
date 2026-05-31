//! Native Rust iterators for the UCB BDD cube and node generators.
//!
//! The legacy SIS implementation exposes stateful generators for onset cubes
//! and BDD nodes. This module keeps the same traversal semantics in Rust:
//! cubes form a disjoint onset cover with else branches explored first, and
//! nodes are yielded in postorder with complemented edges regularized.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

pub type BddVariableId = usize;
pub type BddNodeId = usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddLiteral {
    Negative,
    Positive,
    DontCare,
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    variables: usize,
}

impl BddManager {
    pub fn new(variables: usize) -> Self {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            variables,
        }
    }

    pub const fn variables(&self) -> usize {
        self.variables
    }

    pub const fn zero(&self) -> BddEdge {
        BddEdge::regular(0)
    }

    pub const fn one(&self) -> BddEdge {
        BddEdge::regular(1)
    }

    pub fn add_branch(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, BddIterError> {
        if variable >= self.variables {
            return Err(BddIterError::VariableOutOfRange {
                variable,
                variables: self.variables,
            });
        }

        self.node(then_edge)?;
        self.node(else_edge)?;

        let edge = BddEdge::regular(self.nodes.len());
        self.nodes.push(BddNode::Branch {
            variable,
            then_edge,
            else_edge,
        });

        Ok(edge)
    }

    pub fn node(&self, edge: BddEdge) -> Result<&BddNode, BddIterError> {
        self.nodes
            .get(edge.node)
            .ok_or(BddIterError::MissingNode(edge.node))
    }

    pub fn regular_node(&self, node: BddNodeId) -> Result<&BddNode, BddIterError> {
        self.nodes.get(node).ok_or(BddIterError::MissingNode(node))
    }

    fn constant_value(&self, edge: BddEdge) -> Result<Option<bool>, BddIterError> {
        match self.node(edge)? {
            BddNode::Constant(value) => Ok(Some(*value ^ edge.is_complemented())),
            BddNode::Branch { .. } => Ok(None),
        }
    }

    fn branches(&self, edge: BddEdge) -> Result<(BddVariableId, BddEdge, BddEdge), BddIterError> {
        match self.node(BddEdge::regular(edge.node()))? {
            BddNode::Constant(_) => Err(BddIterError::ExpectedBranch(edge.node())),
            BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            } => {
                if edge.is_complemented() {
                    Ok((*variable, then_edge.not(), else_edge.not()))
                } else {
                    Ok((*variable, *then_edge, *else_edge))
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddIterError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    VariableOutOfRange {
        variable: BddVariableId,
        variables: usize,
    },
}

impl fmt::Display for BddIterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch"),
            Self::VariableOutOfRange {
                variable,
                variables,
            } => write!(
                formatter,
                "BDD variable {variable} is outside the manager range 0..{variables}"
            ),
        }
    }
}

impl Error for BddIterError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeIterator {
    cubes: Vec<Vec<BddLiteral>>,
    position: usize,
}

impl CubeIterator {
    pub fn new(manager: &BddManager, root: BddEdge) -> Result<Self, BddIterError> {
        manager.node(root)?;

        let mut cubes = Vec::new();
        let cube = vec![BddLiteral::DontCare; manager.variables()];
        collect_cubes(manager, root, cube, &mut cubes)?;

        Ok(Self { cubes, position: 0 })
    }
}

impl Iterator for CubeIterator {
    type Item = Vec<BddLiteral>;

    fn next(&mut self) -> Option<Self::Item> {
        let cube = self.cubes.get(self.position)?.clone();
        self.position += 1;
        Some(cube)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIterator {
    nodes: Vec<BddNodeId>,
    position: usize,
}

impl NodeIterator {
    pub fn new(manager: &BddManager, root: BddEdge) -> Result<Self, BddIterError> {
        manager.node(root)?;

        let mut visited = HashSet::new();
        let mut nodes = Vec::new();
        collect_nodes(manager, root, &mut visited, &mut nodes)?;

        Ok(Self { nodes, position: 0 })
    }
}

impl Iterator for NodeIterator {
    type Item = BddNodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.nodes.get(self.position).copied()?;
        self.position += 1;
        Some(node)
    }
}

pub fn cubes(manager: &BddManager, root: BddEdge) -> Result<CubeIterator, BddIterError> {
    CubeIterator::new(manager, root)
}

pub fn nodes(manager: &BddManager, root: BddEdge) -> Result<NodeIterator, BddIterError> {
    NodeIterator::new(manager, root)
}

fn collect_cubes(
    manager: &BddManager,
    edge: BddEdge,
    cube: Vec<BddLiteral>,
    cubes: &mut Vec<Vec<BddLiteral>>,
) -> Result<(), BddIterError> {
    if let Some(value) = manager.constant_value(edge)? {
        if value {
            cubes.push(cube);
        }

        return Ok(());
    }

    let (variable, then_edge, else_edge) = manager.branches(edge)?;
    let then_is_zero = manager
        .constant_value(then_edge)?
        .is_some_and(|value| !value);
    let else_is_zero = manager
        .constant_value(else_edge)?
        .is_some_and(|value| !value);

    if then_is_zero {
        let mut else_cube = cube;
        else_cube[variable] = BddLiteral::Negative;
        collect_cubes(manager, else_edge, else_cube, cubes)?;
    } else if else_is_zero {
        let mut then_cube = cube;
        then_cube[variable] = BddLiteral::Positive;
        collect_cubes(manager, then_edge, then_cube, cubes)?;
    } else {
        let mut else_cube = cube.clone();
        else_cube[variable] = BddLiteral::Negative;
        collect_cubes(manager, else_edge, else_cube, cubes)?;

        let mut then_cube = cube;
        then_cube[variable] = BddLiteral::Positive;
        collect_cubes(manager, then_edge, then_cube, cubes)?;
    }

    Ok(())
}

fn collect_nodes(
    manager: &BddManager,
    edge: BddEdge,
    visited: &mut HashSet<BddNodeId>,
    nodes: &mut Vec<BddNodeId>,
) -> Result<(), BddIterError> {
    let node = edge.node();
    if visited.contains(&node) {
        return Ok(());
    }

    match manager.regular_node(node)? {
        BddNode::Constant(_) => {}
        BddNode::Branch {
            then_edge,
            else_edge,
            ..
        } => {
            let then_edge = if edge.is_complemented() {
                then_edge.not()
            } else {
                *then_edge
            };
            let else_edge = if edge.is_complemented() {
                else_edge.not()
            } else {
                *else_edge
            };

            collect_nodes(manager, else_edge, visited, nodes)?;
            collect_nodes(manager, then_edge, visited, nodes)?;
        }
    }

    visited.insert(node);
    nodes.push(node);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manager() -> (BddManager, BddEdge, BddEdge, BddEdge) {
        let mut manager = BddManager::new(3);
        let x = manager
            .add_branch(0, manager.one(), manager.zero())
            .unwrap();
        let y = manager
            .add_branch(1, manager.one(), manager.zero())
            .unwrap();
        let root = manager.add_branch(2, y, x).unwrap();

        (manager, x, y, root)
    }

    #[test]
    fn zero_has_no_onset_cubes() {
        let manager = BddManager::new(2);

        let cubes: Vec<_> = cubes(&manager, manager.zero()).unwrap().collect();

        assert!(cubes.is_empty());
    }

    #[test]
    fn one_has_single_all_dont_care_cube() {
        let manager = BddManager::new(3);

        let cubes: Vec<_> = cubes(&manager, manager.one()).unwrap().collect();

        assert_eq!(
            cubes,
            vec![vec![
                BddLiteral::DontCare,
                BddLiteral::DontCare,
                BddLiteral::DontCare,
            ]]
        );
    }

    #[test]
    fn cube_iterator_explores_else_branch_before_then_branch() {
        let (manager, _, _, root) = sample_manager();

        let cubes: Vec<_> = cubes(&manager, root).unwrap().collect();

        assert_eq!(
            cubes,
            vec![
                vec![
                    BddLiteral::Positive,
                    BddLiteral::DontCare,
                    BddLiteral::Negative,
                ],
                vec![
                    BddLiteral::DontCare,
                    BddLiteral::Positive,
                    BddLiteral::Positive,
                ],
            ]
        );
    }

    #[test]
    fn cube_iterator_handles_complemented_edges() {
        let (manager, _, _, root) = sample_manager();

        let cubes: Vec<_> = cubes(&manager, root.not()).unwrap().collect();

        assert_eq!(
            cubes,
            vec![
                vec![
                    BddLiteral::Negative,
                    BddLiteral::DontCare,
                    BddLiteral::Negative,
                ],
                vec![
                    BddLiteral::DontCare,
                    BddLiteral::Negative,
                    BddLiteral::Positive,
                ],
            ]
        );
    }

    #[test]
    fn variable_without_branching_choice_is_kept_in_cube() {
        let mut manager = BddManager::new(2);
        let y = manager
            .add_branch(1, manager.one(), manager.zero())
            .unwrap();
        let root = manager.add_branch(0, y, manager.zero()).unwrap();

        let cubes: Vec<_> = cubes(&manager, root).unwrap().collect();

        assert_eq!(
            cubes,
            vec![vec![BddLiteral::Positive, BddLiteral::Positive]]
        );
    }

    #[test]
    fn node_iterator_returns_regularized_postorder_nodes() {
        let (manager, x, y, root) = sample_manager();

        let nodes: Vec<_> = nodes(&manager, root.not()).unwrap().collect();

        assert_eq!(
            nodes,
            vec![
                manager.zero().node(),
                manager.one().node(),
                x.node(),
                y.node(),
                root.node()
            ]
        );
    }

    #[test]
    fn node_iterator_visits_shared_nodes_once() {
        let mut manager = BddManager::new(2);
        let x = manager
            .add_branch(0, manager.one(), manager.zero())
            .unwrap();
        let root = manager.add_branch(1, x, x.not()).unwrap();

        let nodes: Vec<_> = nodes(&manager, root).unwrap().collect();

        assert_eq!(
            nodes,
            vec![
                manager.zero().node(),
                manager.one().node(),
                x.node(),
                root.node()
            ]
        );
    }

    #[test]
    fn missing_root_is_reported() {
        let manager = BddManager::new(1);

        assert_eq!(
            cubes(&manager, BddEdge::regular(99)),
            Err(BddIterError::MissingNode(99))
        );
        assert_eq!(
            nodes(&manager, BddEdge::regular(99)),
            Err(BddIterError::MissingNode(99))
        );
    }

    #[test]
    fn branch_variables_are_validated() {
        let mut manager = BddManager::new(1);

        assert_eq!(
            manager.add_branch(1, manager.one(), manager.zero()),
            Err(BddIterError::VariableOutOfRange {
                variable: 1,
                variables: 1,
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present() {
        let source = include_str!("bdd_iter.rs");

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
