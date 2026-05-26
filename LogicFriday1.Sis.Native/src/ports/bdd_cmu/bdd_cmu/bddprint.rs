use std::collections::HashMap;
use std::fmt;
use std::io;
use std::io::Write;

pub type BddVariableId = u32;
pub type TerminalWord = isize;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddEdge {
    node: usize,
    complemented: bool,
}

impl BddEdge {
    pub const fn regular(node: usize) -> Self {
        Self {
            node,
            complemented: false,
        }
    }

    pub const fn complemented(node: usize) -> Self {
        Self {
            node,
            complemented: true,
        }
    }

    pub const fn node(self) -> usize {
        self.node
    }

    pub const fn is_complemented(self) -> bool {
        self.complemented
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddNode {
    False,
    True,
    Terminal(TerminalWord, TerminalWord),
    Branch {
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrintError {
    Io(String),
    MissingNode(usize),
    VariableOrder {
        parent: BddVariableId,
        child: BddVariableId,
    },
}

impl fmt::Display for PrintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => formatter.write_str(message),
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
        }
    }
}

impl std::error::Error for PrintError {}

impl From<io::Error> for PrintError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            nodes: vec![BddNode::False, BddNode::True],
            unique_table: HashMap::new(),
        }
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::regular(0)
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::regular(1)
    }

    pub fn not(&self, edge: BddEdge) -> BddEdge {
        match self.effective_node(edge) {
            Ok(BddNode::False) => self.one(),
            Ok(BddNode::True) => self.zero(),
            _ => BddEdge {
                node: edge.node,
                complemented: !edge.complemented,
            },
        }
    }

    pub fn terminal(&mut self, high_word: TerminalWord, low_word: TerminalWord) -> BddEdge {
        let edge = BddEdge::regular(self.nodes.len());
        self.nodes.push(BddNode::Terminal(high_word, low_word));
        edge
    }

    pub fn variable(&mut self, variable: BddVariableId) -> BddEdge {
        self.find_or_add(variable, self.one(), self.zero())
            .expect("variable nodes have ordered constant children")
    }

    pub fn find_or_add(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, PrintError> {
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.validate_order(variable, then_edge)?;
        self.validate_order(variable, else_edge)?;

        if then_edge == else_edge {
            return Ok(then_edge);
        }

        let key = (variable, then_edge, else_edge);
        if let Some(edge) = self.unique_table.get(&key).copied() {
            return Ok(edge);
        }

        let edge = BddEdge::regular(self.nodes.len());
        self.nodes.push(BddNode::Branch {
            variable,
            then_edge,
            else_edge,
        });
        self.unique_table.insert(key, edge);
        Ok(edge)
    }

    pub fn print_bdd<W, V, T>(
        &self,
        root: BddEdge,
        writer: W,
        var_naming: V,
        terminal_id: T,
    ) -> Result<(), PrintError>
    where
        W: Write,
        V: Fn(BddVariableId) -> Option<String>,
        T: Fn(TerminalWord, TerminalWord) -> Option<String>,
    {
        self.validate_edge(root)?;

        let mut printer = BddPrinter {
            manager: self,
            writer,
            var_naming,
            terminal_id,
            shared_numbers: self.shared_numbers(root)?,
            printed: HashMap::new(),
        };

        printer.print_step(root, 0)
    }

    pub fn format_bdd<V, T>(
        &self,
        root: BddEdge,
        var_naming: V,
        terminal_id: T,
    ) -> Result<String, PrintError>
    where
        V: Fn(BddVariableId) -> Option<String>,
        T: Fn(TerminalWord, TerminalWord) -> Option<String>,
    {
        let mut output = Vec::new();
        self.print_bdd(root, &mut output, var_naming, terminal_id)?;
        String::from_utf8(output).map_err(|error| PrintError::Io(error.to_string()))
    }

    fn validate_edge(&self, edge: BddEdge) -> Result<(), PrintError> {
        self.nodes
            .get(edge.node)
            .map(|_| ())
            .ok_or(PrintError::MissingNode(edge.node))
    }

    fn validate_order(&self, parent: BddVariableId, child: BddEdge) -> Result<(), PrintError> {
        let child_variable = self.top_variable(child)?;
        if child_variable.map_or(true, |child| parent < child) {
            Ok(())
        } else {
            Err(PrintError::VariableOrder {
                parent,
                child: child_variable.expect("child variable is present"),
            })
        }
    }

    fn top_variable(&self, edge: BddEdge) -> Result<Option<BddVariableId>, PrintError> {
        match self.effective_node(edge)? {
            BddNode::Branch { variable, .. } => Ok(Some(variable)),
            _ => Ok(None),
        }
    }

    fn effective_node(&self, edge: BddEdge) -> Result<BddNode, PrintError> {
        let node = *self
            .nodes
            .get(edge.node)
            .ok_or(PrintError::MissingNode(edge.node))?;

        if !edge.complemented {
            return Ok(node);
        }

        Ok(match node {
            BddNode::False => BddNode::True,
            BddNode::True => BddNode::False,
            BddNode::Terminal(high_word, low_word) => BddNode::Terminal(!high_word, !low_word),
            BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            } => BddNode::Branch {
                variable,
                then_edge: self.not(then_edge),
                else_edge: self.not(else_edge),
            },
        })
    }

    fn shared_numbers(&self, root: BddEdge) -> Result<HashMap<usize, usize>, PrintError> {
        let mut references = HashMap::new();
        self.count_references(root, &mut references)?;

        let mut shared = references
            .into_iter()
            .filter_map(|(node, count)| (count > 1 && self.is_branch_node(node)).then_some(node))
            .collect::<Vec<_>>();
        shared.sort_unstable();

        Ok(shared
            .into_iter()
            .enumerate()
            .map(|(number, node)| (node, number))
            .collect())
    }

    fn count_references(
        &self,
        edge: BddEdge,
        references: &mut HashMap<usize, usize>,
    ) -> Result<(), PrintError> {
        self.validate_edge(edge)?;
        *references.entry(edge.node).or_insert(0) += 1;

        if let BddNode::Branch {
            then_edge,
            else_edge,
            ..
        } = self.effective_node(edge)?
        {
            self.count_references(then_edge, references)?;
            self.count_references(else_edge, references)?;
        }

        Ok(())
    }

    fn is_branch_node(&self, node: usize) -> bool {
        matches!(self.nodes.get(node), Some(BddNode::Branch { .. }))
    }
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

struct BddPrinter<'a, W, V, T>
where
    W: Write,
    V: Fn(BddVariableId) -> Option<String>,
    T: Fn(TerminalWord, TerminalWord) -> Option<String>,
{
    manager: &'a BddManager,
    writer: W,
    var_naming: V,
    terminal_id: T,
    shared_numbers: HashMap<usize, usize>,
    printed: HashMap<usize, bool>,
}

impl<W, V, T> BddPrinter<'_, W, V, T>
where
    W: Write,
    V: Fn(BddVariableId) -> Option<String>,
    T: Fn(TerminalWord, TerminalWord) -> Option<String>,
{
    fn print_step(&mut self, edge: BddEdge, indentation: usize) -> Result<(), PrintError> {
        self.write_indent(indentation)?;

        match self.manager.effective_node(edge)? {
            BddNode::False => writeln!(self.writer, "0").map_err(Into::into),
            BddNode::True => writeln!(self.writer, "1").map_err(Into::into),
            BddNode::Terminal(high_word, low_word) => {
                writeln!(self.writer, "{}", self.terminal_name(high_word, low_word))
                    .map_err(Into::into)
            }
            BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            } => {
                if self.print_subformula_reference(edge)? {
                    return Ok(());
                }

                self.write_shared_definition(edge)?;

                if self.is_negative_variable(then_edge, else_edge) {
                    write!(self.writer, "!")?;
                    self.print_top_variable(variable)
                } else if self.is_positive_variable(then_edge, else_edge) {
                    self.print_top_variable(variable)
                } else {
                    write!(self.writer, "if ")?;
                    self.print_top_variable(variable)?;
                    self.print_step(then_edge, indentation + 2)?;
                    self.write_indent(indentation)?;
                    write!(self.writer, "else if !")?;
                    self.print_top_variable(variable)?;
                    self.print_step(else_edge, indentation + 2)?;
                    self.write_indent(indentation)?;
                    write!(self.writer, "endif ")?;
                    self.print_top_variable(variable)
                }
            }
        }
    }

    fn print_subformula_reference(&mut self, edge: BddEdge) -> Result<bool, PrintError> {
        let Some(number) = self.shared_numbers.get(&edge.node).copied() else {
            return Ok(false);
        };

        if !self.printed.get(&edge.node).copied().unwrap_or(false) {
            return Ok(false);
        }

        if edge.complemented {
            write!(self.writer, "!")?;
        }

        writeln!(self.writer, "subformula {number}")?;
        Ok(true)
    }

    fn write_shared_definition(&mut self, edge: BddEdge) -> Result<(), PrintError> {
        let Some(number) = self.shared_numbers.get(&edge.node).copied() else {
            return Ok(());
        };

        if self.printed.insert(edge.node, true).unwrap_or(false) {
            return Ok(());
        }

        if edge.complemented {
            write!(self.writer, "!")?;
        }

        write!(self.writer, "{number}: ")?;
        Ok(())
    }

    fn print_top_variable(&mut self, variable: BddVariableId) -> Result<(), PrintError> {
        writeln!(self.writer, "{}", self.variable_name(variable))?;
        Ok(())
    }

    fn write_indent(&mut self, indentation: usize) -> Result<(), PrintError> {
        for _ in 0..indentation {
            write!(self.writer, " ")?;
        }

        Ok(())
    }

    fn variable_name(&self, variable: BddVariableId) -> String {
        (self.var_naming)(variable).unwrap_or_else(|| variable.to_string())
    }

    fn terminal_name(&self, high_word: TerminalWord, low_word: TerminalWord) -> String {
        (self.terminal_id)(high_word, low_word)
            .unwrap_or_else(|| format!("terminal({high_word}, {low_word})"))
    }

    fn is_positive_variable(&self, then_edge: BddEdge, else_edge: BddEdge) -> bool {
        matches!(
            (
                self.manager.effective_node(then_edge),
                self.manager.effective_node(else_edge),
            ),
            (Ok(BddNode::True), Ok(BddNode::False))
        )
    }

    fn is_negative_variable(&self, then_edge: BddEdge, else_edge: BddEdge) -> bool {
        matches!(
            (
                self.manager.effective_node(then_edge),
                self.manager.effective_node(else_edge),
            ),
            (Ok(BddNode::False), Ok(BddNode::True))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variable_name(variable: BddVariableId) -> Option<String> {
        Some(format!("x{variable}"))
    }

    fn terminal_id(high_word: TerminalWord, low_word: TerminalWord) -> Option<String> {
        Some(format!("T:{high_word}:{low_word}"))
    }

    #[test]
    fn prints_constants_like_legacy_terminal_ids() {
        let manager = BddManager::new();

        assert_eq!(
            manager
                .format_bdd(manager.zero(), variable_name, terminal_id)
                .unwrap(),
            "0\n"
        );
        assert_eq!(
            manager
                .format_bdd(manager.one(), variable_name, terminal_id)
                .unwrap(),
            "1\n"
        );
    }

    #[test]
    fn prints_positive_and_negative_variables() {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let not_x = manager.not(x);

        assert_eq!(
            manager.format_bdd(x, variable_name, terminal_id).unwrap(),
            "x1\n"
        );
        assert_eq!(
            manager
                .format_bdd(not_x, variable_name, terminal_id)
                .unwrap(),
            "!x1\n"
        );
    }

    #[test]
    fn prints_nested_if_else_tree_with_legacy_indentation() {
        let mut manager = BddManager::new();
        let y = manager.variable(2);
        let root = manager.find_or_add(1, manager.one(), y).unwrap();

        assert_eq!(
            manager
                .format_bdd(root, variable_name, terminal_id)
                .unwrap(),
            "if x1\n  1\nelse if !x1\n  x2\nendif x1\n"
        );
    }

    #[test]
    fn prints_custom_terminal_nodes() {
        let mut manager = BddManager::new();
        let terminal = manager.terminal(7, 11);

        assert_eq!(
            manager
                .format_bdd(terminal, variable_name, terminal_id)
                .unwrap(),
            "T:7:11\n"
        );
    }

    #[test]
    fn prints_shared_subformula_references() {
        let mut manager = BddManager::new();
        let shared = manager
            .find_or_add(2, manager.one(), manager.zero())
            .unwrap();
        let root = manager.find_or_add(1, shared, shared).unwrap();

        assert_eq!(root, shared);

        let left = manager.find_or_add(1, shared, manager.zero()).unwrap();
        let right = manager.find_or_add(1, shared, manager.one()).unwrap();
        let root = manager.find_or_add(0, left, right).unwrap();

        assert_eq!(
            manager.format_bdd(root, variable_name, terminal_id).unwrap(),
            "if x0\n  if x1\n    0: x2\n  else if !x1\n    0\n  endif x1\nelse if !x0\n  if x1\n    subformula 0\n  else if !x1\n    1\n  endif x1\nendif x0\n"
        );
    }

    #[test]
    fn prints_complemented_shared_references_with_prefix() {
        let mut manager = BddManager::new();
        let shared = manager
            .find_or_add(2, manager.one(), manager.zero())
            .unwrap();
        let left = manager.find_or_add(1, shared, manager.zero()).unwrap();
        let right = manager
            .find_or_add(1, manager.not(shared), manager.one())
            .unwrap();
        let root = manager.find_or_add(0, left, right).unwrap();

        assert_eq!(
            manager.format_bdd(root, variable_name, terminal_id).unwrap(),
            "if x0\n  if x1\n    0: x2\n  else if !x1\n    0\n  endif x1\nelse if !x0\n  if x1\n    !subformula 0\n  else if !x1\n    1\n  endif x1\nendif x0\n"
        );
    }

    #[test]
    fn rejects_missing_nodes() {
        let manager = BddManager::new();

        assert_eq!(
            manager
                .format_bdd(BddEdge::regular(99), variable_name, terminal_id)
                .unwrap_err(),
            PrintError::MissingNode(99)
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bddprint.rs");
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
