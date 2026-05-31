//! Native Rust BDD iterators corresponding to the legacy CMU `bdditer.c` unit.
//!
//! The C implementation exposes stateful generators for two traversals:
//! onset cube enumeration and unique BDD node enumeration. This module keeps
//! those behaviors with Rust-owned handles and `Iterator` implementations.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddNodeId(usize);

impl BddNodeId {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);

    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddLiteral {
    Negative,
    Positive,
    DontCare,
}

impl BddLiteral {
    pub const fn legacy_value(self) -> u8 {
        match self {
            Self::Negative => 0,
            Self::Positive => 1,
            Self::DontCare => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddNode {
    variable: usize,
    else_branch: BddNodeId,
    then_branch: BddNodeId,
}

impl BddNode {
    pub const fn new(variable: usize, else_branch: BddNodeId, then_branch: BddNodeId) -> Self {
        Self {
            variable,
            else_branch,
            then_branch,
        }
    }

    pub const fn variable(self) -> usize {
        self.variable
    }

    pub const fn else_branch(self) -> BddNodeId {
        self.else_branch
    }

    pub const fn then_branch(self) -> BddNodeId {
        self.then_branch
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddIteratorError {
    InvalidNode(BddNodeId),
    VariableOutOfRange {
        variable: usize,
        variable_count: usize,
    },
}

#[derive(Clone, Debug)]
pub struct BddManager {
    variable_count: usize,
    nodes: Vec<Option<BddNode>>,
}

impl BddManager {
    pub fn new(variable_count: usize) -> Self {
        Self {
            variable_count,
            nodes: vec![None, None],
        }
    }

    pub fn variable_count(&self) -> usize {
        self.variable_count
    }

    pub fn zero(&self) -> BddNodeId {
        BddNodeId::ZERO
    }

    pub fn one(&self) -> BddNodeId {
        BddNodeId::ONE
    }

    pub fn add_node(
        &mut self,
        variable: usize,
        else_branch: BddNodeId,
        then_branch: BddNodeId,
    ) -> Result<BddNodeId, BddIteratorError> {
        if variable >= self.variable_count {
            return Err(BddIteratorError::VariableOutOfRange {
                variable,
                variable_count: self.variable_count,
            });
        }

        self.require_node(else_branch)?;
        self.require_node(then_branch)?;

        if else_branch == then_branch {
            return Ok(else_branch);
        }

        let id = BddNodeId(self.nodes.len());
        self.nodes
            .push(Some(BddNode::new(variable, else_branch, then_branch)));
        Ok(id)
    }

    pub fn node(&self, id: BddNodeId) -> Result<Option<BddNode>, BddIteratorError> {
        if id == BddNodeId::ZERO || id == BddNodeId::ONE {
            return Ok(None);
        }

        self.nodes
            .get(id.0)
            .copied()
            .flatten()
            .map(Some)
            .ok_or(BddIteratorError::InvalidNode(id))
    }

    pub fn cubes(&self, root: BddNodeId) -> Result<CubeIter<'_>, BddIteratorError> {
        self.require_node(root)?;
        Ok(CubeIter::new(self, root))
    }

    pub fn nodes_postorder(&self, root: BddNodeId) -> Result<NodeIter, BddIteratorError> {
        self.require_node(root)?;

        let mut visited = HashSet::new();
        let mut order = Vec::new();
        self.collect_postorder(root, &mut visited, &mut order)?;

        Ok(NodeIter { order, index: 0 })
    }

    fn require_node(&self, id: BddNodeId) -> Result<(), BddIteratorError> {
        if id == BddNodeId::ZERO || id == BddNodeId::ONE {
            return Ok(());
        }

        match self.nodes.get(id.0) {
            Some(Some(_)) => Ok(()),
            _ => Err(BddIteratorError::InvalidNode(id)),
        }
    }

    fn collect_postorder(
        &self,
        id: BddNodeId,
        visited: &mut HashSet<BddNodeId>,
        order: &mut Vec<BddNodeId>,
    ) -> Result<(), BddIteratorError> {
        if !visited.insert(id) {
            return Ok(());
        }

        if let Some(node) = self.node(id)? {
            self.collect_postorder(node.else_branch, visited, order)?;
            self.collect_postorder(node.then_branch, visited, order)?;
        }

        order.push(id);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CubeIter<'a> {
    manager: &'a BddManager,
    root: BddNodeId,
    cube: Vec<BddLiteral>,
    stack: Vec<BddNodeId>,
    started: bool,
    finished: bool,
}

impl<'a> CubeIter<'a> {
    fn new(manager: &'a BddManager, root: BddNodeId) -> Self {
        Self {
            manager,
            root,
            cube: vec![BddLiteral::DontCare; manager.variable_count()],
            stack: Vec::new(),
            started: false,
            finished: false,
        }
    }

    pub fn current_cube(&self) -> &[BddLiteral] {
        &self.cube
    }

    fn descend_to_cube(&mut self, id: BddNodeId) -> Result<bool, BddIteratorError> {
        if id == BddNodeId::ZERO {
            return Ok(false);
        }

        if id == BddNodeId::ONE {
            return Ok(true);
        }

        let node = self
            .manager
            .node(id)?
            .expect("non-constant BDD node has a branch record");

        if node.then_branch == BddNodeId::ZERO {
            self.cube[node.variable] = BddLiteral::Negative;
            return self.descend_to_cube(node.else_branch);
        }

        if node.else_branch == BddNodeId::ZERO {
            self.cube[node.variable] = BddLiteral::Positive;
            return self.descend_to_cube(node.then_branch);
        }

        self.stack.push(id);
        self.cube[node.variable] = BddLiteral::Negative;
        self.descend_to_cube(node.else_branch)
    }

    fn next_from_stack(&mut self) -> Result<bool, BddIteratorError> {
        let Some(id) = self.stack.pop() else {
            self.finished = true;
            return Ok(false);
        };

        let node = self
            .manager
            .node(id)?
            .expect("stack only contains non-constant BDD nodes");

        self.cube[node.variable] = BddLiteral::Positive;

        for literal in self.cube.iter_mut().skip(node.variable + 1) {
            *literal = BddLiteral::DontCare;
        }

        self.descend_to_cube(node.then_branch)
    }
}

impl Iterator for CubeIter<'_> {
    type Item = Result<Vec<BddLiteral>, BddIteratorError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let found = if self.started {
            self.next_from_stack()
        } else {
            self.started = true;
            self.descend_to_cube(self.root)
        };

        match found {
            Ok(true) => Some(Ok(self.cube.clone())),
            Ok(false) => {
                self.finished = true;
                None
            }
            Err(error) => {
                self.finished = true;
                Some(Err(error))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeIter {
    order: Vec<BddNodeId>,
    index: usize,
}

impl Iterator for NodeIter {
    type Item = BddNodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.order.get(self.index).copied()?;
        self.index += 1;
        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal_values(cube: &[BddLiteral]) -> Vec<u8> {
        cube.iter().map(|literal| literal.legacy_value()).collect()
    }

    #[test]
    fn zero_has_empty_onset() {
        let manager = BddManager::new(3);

        let cubes: Vec<_> = manager.cubes(manager.zero()).unwrap().collect();

        assert!(cubes.is_empty());
    }

    #[test]
    fn one_has_single_all_dont_care_cube() {
        let manager = BddManager::new(3);
        let cubes: Vec<_> = manager
            .cubes(manager.one())
            .unwrap()
            .map(|cube| literal_values(&cube.unwrap()))
            .collect();

        assert_eq!(cubes, vec![vec![2, 2, 2]]);
    }

    #[test]
    fn cube_iterator_returns_disjoint_cover_else_branch_first() {
        let mut manager = BddManager::new(3);
        let x2 = manager.add_node(2, manager.zero(), manager.one()).unwrap();
        let x1 = manager.add_node(1, manager.zero(), manager.one()).unwrap();
        let root = manager.add_node(0, x1, x2).unwrap();

        let cubes: Vec<_> = manager
            .cubes(root)
            .unwrap()
            .map(|cube| literal_values(&cube.unwrap()))
            .collect();

        assert_eq!(cubes, vec![vec![0, 1, 2], vec![1, 2, 1]]);
    }

    #[test]
    fn forced_branches_do_not_push_extra_cubes() {
        let mut manager = BddManager::new(2);
        let x1 = manager.add_node(1, manager.zero(), manager.one()).unwrap();
        let root = manager.add_node(0, x1, manager.zero()).unwrap();

        let cubes: Vec<_> = manager
            .cubes(root)
            .unwrap()
            .map(|cube| literal_values(&cube.unwrap()))
            .collect();

        assert_eq!(cubes, vec![vec![0, 1]]);
    }

    #[test]
    fn node_iterator_returns_unique_postorder_nodes() {
        let mut manager = BddManager::new(3);
        let shared = manager.add_node(2, manager.zero(), manager.one()).unwrap();
        let left = manager.add_node(1, manager.zero(), shared).unwrap();
        let root = manager.add_node(0, left, shared).unwrap();

        let nodes: Vec<_> = manager.nodes_postorder(root).unwrap().collect();

        assert_eq!(
            nodes,
            vec![manager.zero(), manager.one(), shared, left, root]
        );
    }

    #[test]
    fn invalid_branch_is_rejected_when_building_nodes() {
        let mut manager = BddManager::new(1);

        assert_eq!(
            manager.add_node(0, BddNodeId(42), manager.one()),
            Err(BddIteratorError::InvalidNode(BddNodeId(42)))
        );
    }

    #[test]
    fn invalid_variable_is_rejected_when_building_nodes() {
        let mut manager = BddManager::new(1);

        assert_eq!(
            manager.add_node(1, manager.zero(), manager.one()),
            Err(BddIteratorError::VariableOutOfRange {
                variable: 1,
                variable_count: 1
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present() {
        let text = include_str!("bdditer.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
