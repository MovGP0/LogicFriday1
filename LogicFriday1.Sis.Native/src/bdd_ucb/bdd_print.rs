//! Native Rust printer for UCB BDD graphs.
//!
//! The legacy SIS routine prints variable index names, then walks a BDD from
//! its root while suppressing repeated branch nodes by their regular identity.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::io;
use std::io::Write;

pub type BddNodeId = usize;
pub type BddVariableId = usize;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddPrintNode {
    Constant(bool),
    Branch {
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddPrintManager {
    nodes: Vec<BddPrintNode>,
    variable_count: usize,
}

impl BddPrintManager {
    pub fn new(variable_count: usize) -> Self {
        Self {
            nodes: vec![BddPrintNode::Constant(false), BddPrintNode::Constant(true)],
            variable_count,
        }
    }

    pub fn variable_count(&self) -> usize {
        self.variable_count
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::regular(0)
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::regular(1)
    }

    pub fn add_branch(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> BddEdge {
        let edge = BddEdge::regular(self.nodes.len());
        self.nodes.push(BddPrintNode::Branch {
            variable,
            then_edge,
            else_edge,
        });
        edge
    }

    pub fn node(&self, edge: BddEdge) -> Result<&BddPrintNode, BddPrintError> {
        self.nodes
            .get(edge.node)
            .ok_or(BddPrintError::MissingNode(edge.node))
    }

    fn branch(&self, edge: BddEdge) -> Result<(BddVariableId, BddEdge, BddEdge), BddPrintError> {
        match self.node(edge)? {
            BddPrintNode::Constant(_) => Err(BddPrintError::ExpectedBranch(edge.node)),
            BddPrintNode::Branch {
                variable,
                then_edge,
                else_edge,
            } => Ok((*variable, *then_edge, *else_edge)),
        }
    }
}

impl Default for BddPrintManager {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddPrintError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
}

impl fmt::Display for BddPrintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch node"),
        }
    }
}

impl Error for BddPrintError {}

pub fn format_bdd(manager: &BddPrintManager, root: BddEdge) -> Result<String, BddPrintError> {
    let mut output = Vec::new();
    match write_bdd(manager, root, &mut output) {
        Ok(()) => {}
        Err(BddPrintIoError::Print(error)) => return Err(error),
        Err(BddPrintIoError::Io(error)) => {
            panic!("writing to String buffer cannot fail: {error}");
        }
    }

    Ok(String::from_utf8(output).expect("BDD print output is ASCII"))
}

pub fn write_bdd<W>(
    manager: &BddPrintManager,
    root: BddEdge,
    writer: &mut W,
) -> Result<(), BddPrintIoError>
where
    W: Write,
{
    manager.node(root).map_err(BddPrintIoError::Print)?;

    for index in 0..manager.variable_count() {
        writeln!(writer, "\tindex {index} is v#{index}").map_err(BddPrintIoError::Io)?;
    }

    let mut printed = HashSet::new();
    write_node(manager, root, &mut printed, writer)?;
    writeln!(writer).map_err(BddPrintIoError::Io)
}

#[derive(Debug)]
pub enum BddPrintIoError {
    Print(BddPrintError),
    Io(io::Error),
}

impl fmt::Display for BddPrintIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Print(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl Error for BddPrintIoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Print(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

fn write_node<W>(
    manager: &BddPrintManager,
    edge: BddEdge,
    printed: &mut HashSet<BddNodeId>,
    writer: &mut W,
) -> Result<(), BddPrintIoError>
where
    W: Write,
{
    match manager.node(edge).map_err(BddPrintIoError::Print)? {
        BddPrintNode::Constant(value) => {
            let value = if *value { 1 } else { 0 };
            writeln!(writer, "ID =  {value}").map_err(BddPrintIoError::Io)?;
            Ok(())
        }
        BddPrintNode::Branch { .. } => {
            if !printed.insert(edge.node()) {
                return Ok(());
            }

            let (variable, then_edge, else_edge) =
                manager.branch(edge).map_err(BddPrintIoError::Print)?;

            write!(writer, "ID = {}\tindex = {variable}\t", id_name(edge))
                .map_err(BddPrintIoError::Io)?;

            let then_is_constant = write_successor("T", manager, then_edge, writer)?;
            let else_is_constant = write_successor("E", manager, else_edge, writer)?;
            writeln!(writer).map_err(BddPrintIoError::Io)?;

            if !else_is_constant {
                write_node(manager, else_edge, printed, writer)?;
            }

            if !then_is_constant {
                write_node(manager, then_edge, printed, writer)?;
            }

            Ok(())
        }
    }
}

fn write_successor<W>(
    label: &str,
    manager: &BddPrintManager,
    edge: BddEdge,
    writer: &mut W,
) -> Result<bool, BddPrintIoError>
where
    W: Write,
{
    match manager.node(edge).map_err(BddPrintIoError::Print)? {
        BddPrintNode::Constant(value) => {
            let value = if *value { 1 } else { 0 };
            if label == "T" {
                write!(writer, "{label} =  {value}\t\t").map_err(BddPrintIoError::Io)?;
            } else {
                write!(writer, "{label} =  {value}").map_err(BddPrintIoError::Io)?;
            }

            Ok(true)
        }
        BddPrintNode::Branch { .. } => {
            if label == "T" {
                write!(writer, "{label} =  {}\t", id_name(edge)).map_err(BddPrintIoError::Io)?;
            } else {
                write!(writer, "{label} = {}", id_name(edge)).map_err(BddPrintIoError::Io)?;
            }

            Ok(false)
        }
    }
}

pub fn id_name(edge: BddEdge) -> String {
    let prefix = if edge.is_complemented() { '!' } else { ' ' };
    format!("{prefix}{:#x}", edge.node())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_name_preserves_complement_marker_and_hex_identity() {
        assert_eq!(id_name(BddEdge::regular(26)), " 0x1a");
        assert_eq!(id_name(BddEdge::complemented(26)), "!0x1a");
    }

    #[test]
    fn formatter_prints_variable_preamble_and_constant_root() {
        let manager = BddPrintManager::new(2);
        let report = format_bdd(&manager, manager.one()).unwrap();

        assert_eq!(report, "\tindex 0 is v#0\n\tindex 1 is v#1\nID =  1\n\n");
    }

    #[test]
    fn formatter_prints_branch_successors_and_else_before_then_recursion() {
        let mut manager = BddPrintManager::new(3);
        let x = manager.add_branch(0, manager.one(), manager.zero());
        let y = manager.add_branch(1, x, manager.one());
        let z = manager.add_branch(2, y, x.not());

        let report = format_bdd(&manager, z).unwrap();

        assert!(report.contains("ID =  0x4\tindex = 2\tT =   0x3\tE = !0x2\n"));
        assert!(report.contains("ID = !0x2\tindex = 0\tT =  1\t\tE =  0\n"));
        assert!(report.contains("ID =  0x3\tindex = 1\tT =   0x2\tE =  1\n"));
        assert!(report.find("ID = !0x2").unwrap() < report.find("ID =  0x3").unwrap());
    }

    #[test]
    fn formatter_suppresses_repeated_regular_branch_nodes() {
        let mut manager = BddPrintManager::new(2);
        let x = manager.add_branch(0, manager.one(), manager.zero());
        let root = manager.add_branch(1, x, x.not());

        let report = format_bdd(&manager, root).unwrap();

        assert_eq!(report.matches("index = 0").count(), 1);
    }

    #[test]
    fn missing_root_is_reported_before_any_output() {
        let manager = BddPrintManager::new(1);

        let error = format_bdd(&manager, BddEdge::regular(99)).unwrap_err();

        assert_eq!(error, BddPrintError::MissingNode(99));
    }

    #[test]
    fn writer_api_propagates_output_errors() {
        struct FailingWriter;

        impl Write for FailingWriter {
            fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("sink failed"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let manager = BddPrintManager::new(1);
        let error = write_bdd(&manager, manager.one(), &mut FailingWriter).unwrap_err();

        assert!(matches!(error, BddPrintIoError::Io(_)));
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present() {
        let source = include_str!("bdd_print.rs");

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
