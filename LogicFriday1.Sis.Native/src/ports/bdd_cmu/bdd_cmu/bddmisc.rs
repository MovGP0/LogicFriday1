//! Miscellaneous helpers for the native CMU BDD port.
//!
//! The C implementation stores traversal marks directly in tagged BDD nodes and
//! exposes fallback names through static buffers. This module keeps the same
//! behavior with explicit handles, manager-owned marks, and owned strings.

use std::collections::BTreeMap;
use std::fmt;

pub type BddNodeId = usize;
pub type BddIndex = u32;
pub type TerminalValue = isize;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BddRef {
    Zero,
    One,
    Terminal { id: BddNodeId, complemented: bool },
    Node { id: BddNodeId, complemented: bool },
}

impl BddRef {
    pub const fn not(self) -> Self {
        match self {
            Self::Zero => Self::One,
            Self::One => Self::Zero,
            Self::Terminal { id, complemented } => Self::Terminal {
                id,
                complemented: !complemented,
            },
            Self::Node { id, complemented } => Self::Node {
                id,
                complemented: !complemented,
            },
        }
    }

    const fn outpos(self) -> Self {
        match self {
            Self::Terminal { id, .. } => Self::Terminal {
                id,
                complemented: false,
            },
            Self::Node { id, .. } => Self::Node {
                id,
                complemented: false,
            },
            constant => constant,
        }
    }

    const fn node_id(self) -> Option<BddNodeId> {
        match self {
            Self::Terminal { id, .. } | Self::Node { id, .. } => Some(id),
            Self::Zero | Self::One => None,
        }
    }

    const fn is_complemented(self) -> bool {
        match self {
            Self::Terminal { complemented, .. } | Self::Node { complemented, .. } => complemented,
            Self::Zero | Self::One => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddType {
    NonTerminal,
    Zero,
    One,
    PositiveVariable,
    NegativeVariable,
    Terminal,
}

#[derive(Clone, Debug)]
enum BddNode {
    Terminal {
        value1: TerminalValue,
        value2: TerminalValue,
    },
    Branch {
        index: BddIndex,
        then_edge: BddRef,
        else_edge: BddRef,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MiscError {
    MissingNode(BddNodeId),
    BranchExpected(BddRef),
    TerminalExpected(BddRef),
}

impl fmt::Display for MiscError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(id) => write!(formatter, "BDD node {id} is not present"),
            Self::BranchExpected(reference) => {
                write!(formatter, "expected a branch BDD node, got {reference:?}")
            }
            Self::TerminalExpected(reference) => {
                write!(formatter, "expected a terminal BDD node, got {reference:?}")
            }
        }
    }
}

impl std::error::Error for MiscError {}

#[derive(Clone, Debug, Default)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    marks: Vec<u8>,
}

impl BddManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn zero(&self) -> BddRef {
        BddRef::Zero
    }

    pub const fn one(&self) -> BddRef {
        BddRef::One
    }

    pub fn terminal(&mut self, value1: TerminalValue, value2: TerminalValue) -> BddRef {
        let id = self.insert_node(BddNode::Terminal { value1, value2 });

        BddRef::Terminal {
            id,
            complemented: false,
        }
    }

    pub fn variable(&mut self, index: BddIndex) -> BddRef {
        self.node(index, self.one(), self.zero())
    }

    pub fn node(&mut self, index: BddIndex, then_edge: BddRef, else_edge: BddRef) -> BddRef {
        let id = self.insert_node(BddNode::Branch {
            index,
            then_edge,
            else_edge,
        });

        BddRef::Node {
            id,
            complemented: false,
        }
    }

    pub fn bdd_type(&self, reference: BddRef) -> Result<BddType, MiscError> {
        match reference {
            BddRef::Zero => Ok(BddType::Zero),
            BddRef::One => Ok(BddType::One),
            BddRef::Terminal { .. } => Ok(BddType::Terminal),
            BddRef::Node { .. } => {
                let then_edge = self.then_branch(reference)?;
                let else_edge = self.else_branch(reference)?;

                if then_edge == self.one() && else_edge == self.zero() {
                    return Ok(BddType::PositiveVariable);
                }

                if then_edge == self.zero() && else_edge == self.one() {
                    return Ok(BddType::NegativeVariable);
                }

                Ok(BddType::NonTerminal)
            }
        }
    }

    pub fn mark_shared_nodes(&mut self, function: BddRef) -> Result<(), MiscError> {
        let function = function.outpos();

        if matches!(
            self.bdd_type(function)?,
            BddType::Zero | BddType::One | BddType::PositiveVariable
        ) {
            return Ok(());
        }

        let Some(id) = function.node_id() else {
            return Ok(());
        };

        match self.marks[id] {
            0 => {
                self.marks[id] = 1;
            }
            1 => {
                self.marks[id] = 2;
                return Ok(());
            }
            _ => return Ok(()),
        }

        let then_edge = self.then_branch(function)?;
        let else_edge = self.else_branch(function)?;
        self.mark_shared_nodes(then_edge)?;
        self.mark_shared_nodes(else_edge)
    }

    pub fn number_shared_nodes(
        &mut self,
        function: BddRef,
        shared_nodes: &mut BTreeMap<BddRef, usize>,
        next: &mut usize,
    ) -> Result<(), MiscError> {
        if matches!(
            self.bdd_type(function)?,
            BddType::Zero | BddType::One | BddType::PositiveVariable | BddType::NegativeVariable
        ) {
            return Ok(());
        }

        let Some(id) = function.node_id() else {
            return Ok(());
        };

        if self.marks[id] == 0 {
            return Ok(());
        }

        if self.marks[id] == 2 {
            shared_nodes.insert(function.outpos(), *next);
            *next += 1;
        }

        self.marks[id] = 0;
        let then_edge = self.then_branch(function)?;
        let else_edge = self.else_branch(function)?;
        self.number_shared_nodes(then_edge, shared_nodes, next)?;
        self.number_shared_nodes(else_edge, shared_nodes, next)
    }

    pub fn terminal_id<F>(
        &self,
        function: BddRef,
        terminal_id_fn: Option<F>,
    ) -> Result<String, MiscError>
    where
        F: FnOnce(TerminalValue, TerminalValue) -> Option<String>,
    {
        let (value1, value2) = self.mtbdd_terminal_value(function)?;

        if let Some(name) = terminal_id_fn.and_then(|naming| naming(value1, value2)) {
            return Ok(name);
        }

        match function {
            BddRef::One => Ok("1".to_owned()),
            BddRef::Zero => Ok("0".to_owned()),
            _ => Ok(format!("terminal {value1} {value2}")),
        }
    }

    pub fn var_name<F>(
        &self,
        variable: BddRef,
        var_naming_fn: Option<F>,
    ) -> Result<String, MiscError>
    where
        F: FnOnce(BddRef) -> Option<String>,
    {
        if let Some(name) = var_naming_fn.and_then(|naming| naming(variable)) {
            return Ok(name);
        }

        Ok(format!("var.{}", self.index(variable)?))
    }

    pub fn mtbdd_terminal_value(
        &self,
        function: BddRef,
    ) -> Result<(TerminalValue, TerminalValue), MiscError> {
        match function {
            BddRef::Zero => Ok((0, 0)),
            BddRef::One => Ok((1, 0)),
            BddRef::Terminal { id, .. } => {
                let BddNode::Terminal { value1, value2 } = self.node_entry(id)? else {
                    return Err(MiscError::TerminalExpected(function));
                };

                if function.is_complemented() {
                    Ok(transform_terminal_values(*value1, *value2))
                } else {
                    Ok((*value1, *value2))
                }
            }
            BddRef::Node { .. } => Err(MiscError::TerminalExpected(function)),
        }
    }

    pub fn mark(&self, reference: BddRef) -> Result<u8, MiscError> {
        let id = reference
            .node_id()
            .ok_or(MiscError::MissingNode(usize::MAX))?;

        self.marks
            .get(id)
            .copied()
            .ok_or(MiscError::MissingNode(id))
    }

    fn insert_node(&mut self, node: BddNode) -> BddNodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        self.marks.push(0);
        id
    }

    fn node_entry(&self, id: BddNodeId) -> Result<&BddNode, MiscError> {
        self.nodes.get(id).ok_or(MiscError::MissingNode(id))
    }

    fn branch(&self, reference: BddRef) -> Result<(BddIndex, BddRef, BddRef), MiscError> {
        let Some(id) = reference.node_id() else {
            return Err(MiscError::BranchExpected(reference));
        };

        let BddNode::Branch {
            index,
            then_edge,
            else_edge,
        } = self.node_entry(id)?
        else {
            return Err(MiscError::BranchExpected(reference));
        };

        Ok((*index, *then_edge, *else_edge))
    }

    fn then_branch(&self, reference: BddRef) -> Result<BddRef, MiscError> {
        let (_, then_edge, _) = self.branch(reference)?;

        if reference.is_complemented() {
            Ok(then_edge.not())
        } else {
            Ok(then_edge)
        }
    }

    fn else_branch(&self, reference: BddRef) -> Result<BddRef, MiscError> {
        let (_, _, else_edge) = self.branch(reference)?;

        if reference.is_complemented() {
            Ok(else_edge.not())
        } else {
            Ok(else_edge)
        }
    }

    fn index(&self, reference: BddRef) -> Result<BddIndex, MiscError> {
        let (index, _, _) = self.branch(reference)?;

        Ok(index)
    }
}

fn transform_terminal_values(
    value1: TerminalValue,
    value2: TerminalValue,
) -> (TerminalValue, TerminalValue) {
    (!value1, !value2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_only_nodes_reached_more_than_once_as_shared() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let shared = manager.node(2, y, manager.zero());
        let root = manager.node(0, shared, shared);

        manager.mark_shared_nodes(root).unwrap();

        assert_eq!(manager.mark(x), Ok(0));
        assert_eq!(manager.mark(y), Ok(0));
        assert_eq!(manager.mark(shared), Ok(2));
        assert_eq!(manager.mark(root), Ok(1));
    }

    #[test]
    fn numbers_shared_nodes_and_clears_visited_marks() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let shared = manager.node(1, x, manager.zero());
        let root = manager.node(0, shared, shared);
        let mut numbered = BTreeMap::new();
        let mut next = 7;

        manager.mark_shared_nodes(root).unwrap();
        manager
            .number_shared_nodes(root, &mut numbered, &mut next)
            .unwrap();

        assert_eq!(numbered.get(&shared), Some(&7));
        assert_eq!(next, 8);
        assert_eq!(manager.mark(root), Ok(0));
        assert_eq!(manager.mark(shared), Ok(0));
    }

    #[test]
    fn positive_and_negative_variables_are_not_numbered() {
        let mut manager = BddManager::new();
        let variable = manager.variable(3);
        let mut numbered = BTreeMap::new();
        let mut next = 0;

        manager.mark_shared_nodes(variable.not()).unwrap();
        manager
            .number_shared_nodes(variable.not(), &mut numbered, &mut next)
            .unwrap();

        assert!(numbered.is_empty());
        assert_eq!(next, 0);
    }

    #[test]
    fn terminal_id_uses_callback_then_legacy_fallbacks() {
        let mut manager = BddManager::new();
        let terminal = manager.terminal(42, -9);

        assert_eq!(
            manager
                .terminal_id(
                    terminal,
                    Some(|left, right| Some(format!("{left}:{right}")))
                )
                .unwrap(),
            "42:-9"
        );
        assert_eq!(
            manager
                .terminal_id(
                    terminal,
                    None::<fn(TerminalValue, TerminalValue) -> Option<String>>
                )
                .unwrap(),
            "terminal 42 -9"
        );
        assert_eq!(
            manager
                .terminal_id(
                    manager.one(),
                    None::<fn(TerminalValue, TerminalValue) -> Option<String>>
                )
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn var_name_uses_callback_or_default_index_name() {
        let mut manager = BddManager::new();
        let variable = manager.variable(11);

        assert_eq!(
            manager
                .var_name(variable, Some(|_| Some("input_a".to_owned())))
                .unwrap(),
            "input_a"
        );
        assert_eq!(
            manager
                .var_name(variable, None::<fn(BddRef) -> Option<String>>)
                .unwrap(),
            "var.11"
        );
    }

    #[test]
    fn complemented_terminal_values_are_transformed() {
        let mut manager = BddManager::new();
        let terminal = manager.terminal(0b1010, 0b0101);

        assert_eq!(manager.mtbdd_terminal_value(terminal), Ok((0b1010, 0b0101)));
        assert_eq!(
            manager.mtbdd_terminal_value(terminal.not()),
            Ok((!0b1010, !0b0101))
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bddmisc.rs");
        let legacy_export = concat!("no", "_", "mangle");
        let tracking_prefix = concat!("REQUIRED", "_");
        let dependency_type = concat!("Port", "Dependency");
        let bead_token = concat!("bead", "_", "id");
        let source_token = concat!("source", "_", "file");
        let c_abi = ["ex", "tern", " ", "\"C\""].concat();

        assert!(!source.contains(legacy_export));
        assert!(!source.contains(&c_abi));
        assert!(!source.contains(tracking_prefix));
        assert!(!source.contains(dependency_type));
        assert!(!source.contains(bead_token));
        assert!(!source.contains(source_token));
    }
}
