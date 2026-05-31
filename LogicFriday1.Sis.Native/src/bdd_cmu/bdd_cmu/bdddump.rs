use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;
use std::io::{self, Read, Write};

pub type BddNodeId = usize;
pub type BddValue = i32;
pub type BddVariableId = u16;

const MAGIC_COOKIE: u32 = 0x5e02_f795;
const LEGACY_LONG_BYTES: usize = 4;
const INDEX_BYTES: usize = 2;
const TRUE_ENCODING: u32 = 0xffff_ff00;
const FALSE_ENCODING: u32 = 0xffff_ff01;
const POSVAR_ENCODING: u32 = 0xffff_ff02;
const NEGVAR_ENCODING: u32 = 0xffff_ff03;
const POSNODE_ENCODING: u32 = 0xffff_ff04;
const NEGNODE_ENCODING: u32 = 0xffff_ff05;
const NODELABEL_ENCODING: u32 = 0xffff_ff06;
const CONSTANT_ENCODING: u32 = 0xffff_ff07;

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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddNode {
    Terminal(BddValue, BddValue),
    Branch {
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    },
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    terminals: HashMap<(BddValue, BddValue), BddEdge>,
    branches: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
}

impl BddManager {
    pub fn new() -> Self {
        let mut manager = Self {
            nodes: Vec::new(),
            terminals: HashMap::new(),
            branches: HashMap::new(),
        };

        let zero = manager.insert_terminal_unchecked(0, 0);
        let one = manager.insert_terminal_unchecked(1, 0);
        debug_assert_eq!(zero, manager.zero());
        debug_assert_eq!(one, manager.one());

        manager
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::regular(0)
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::regular(1)
    }

    pub fn not(&self, edge: BddEdge) -> BddEdge {
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

    pub fn node(&self, edge: BddEdge) -> Result<&BddNode, BddDumpError> {
        self.nodes
            .get(edge.node)
            .ok_or(BddDumpError::MissingNode(edge.node))
    }

    pub fn terminal(&mut self, first: BddValue, second: BddValue) -> BddEdge {
        if let Some(edge) = self.terminals.get(&(first, second)).copied() {
            return edge;
        }

        self.insert_terminal_unchecked(first, second)
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
    ) -> Result<BddEdge, BddDumpError> {
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
    ) -> Result<BddEdge, BddDumpError> {
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.ite_inner(condition, then_edge, else_edge)
    }

    pub fn dump_bdd(
        &self,
        root: BddEdge,
        support: &[BddEdge],
        mut writer: impl Write,
    ) -> Result<(), BddDumpError> {
        self.validate_edge(root)?;
        let normalized_indexes = self.normalized_indexes(root, support)?;
        let number_vars = support.len();
        let shared_numbers = self.shared_numbers(root)?;
        let index_size = bytes_needed(number_vars + 1);
        let node_number_size = bytes_needed(shared_numbers.len());
        let mut labelled = BTreeSet::new();

        write_unsigned(&mut writer, MAGIC_COOKIE as u64, LEGACY_LONG_BYTES)?;
        write_unsigned(&mut writer, number_vars as u64, INDEX_BYTES)?;
        write_unsigned(&mut writer, shared_numbers.len() as u64, LEGACY_LONG_BYTES)?;
        self.dump_step(
            root,
            &mut writer,
            &normalized_indexes,
            &shared_numbers,
            &mut labelled,
            index_size,
            node_number_size,
        )
    }

    pub fn undump_bdd(
        &mut self,
        support: &[BddEdge],
        mut reader: impl Read,
    ) -> Result<BddEdge, BddUndumpError> {
        for edge in support {
            if !self
                .is_positive_variable(*edge)
                .map_err(BddUndumpError::Dump)?
            {
                return Err(BddUndumpError::SupportNotPositiveVariable(*edge));
            }
        }

        if read_unsigned(&mut reader, LEGACY_LONG_BYTES)? != MAGIC_COOKIE as u64 {
            return Err(BddUndumpError::Format);
        }

        let number_vars = read_unsigned(&mut reader, INDEX_BYTES)? as usize;
        if number_vars != support.len() {
            return Err(BddUndumpError::Format);
        }

        let number_shared = read_unsigned(&mut reader, LEGACY_LONG_BYTES)? as usize;
        let index_size = bytes_needed(number_vars + 1);
        let node_number_size = bytes_needed(number_shared);
        let mut shared = vec![None; number_shared];
        let mut shared_so_far = 0;
        let result = self.undump_step(
            support,
            &mut reader,
            &mut shared,
            &mut shared_so_far,
            index_size,
            node_number_size,
        )?;

        if shared_so_far != number_shared {
            return Err(BddUndumpError::Format);
        }

        Ok(result)
    }

    pub fn eval(
        &self,
        root: BddEdge,
        assignment: &HashMap<BddVariableId, bool>,
    ) -> Result<(BddValue, BddValue), BddDumpError> {
        let mut current = root;
        let mut complemented = false;

        loop {
            complemented ^= current.is_complemented();
            match self.node(BddEdge::regular(current.node))? {
                BddNode::Terminal(first, second) => {
                    if complemented {
                        return Ok(complement_terminal(*first, *second));
                    }

                    return Ok((*first, *second));
                }
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

    fn dump_step(
        &self,
        edge: BddEdge,
        writer: &mut impl Write,
        normalized_indexes: &BTreeMap<BddVariableId, usize>,
        shared_numbers: &BTreeMap<BddNodeId, usize>,
        labelled: &mut BTreeSet<BddNodeId>,
        index_size: usize,
        node_number_size: usize,
    ) -> Result<(), BddDumpError> {
        if edge == self.zero() {
            return write_encoding(writer, FALSE_ENCODING, index_size);
        }

        if edge == self.one() {
            return write_encoding(writer, TRUE_ENCODING, index_size);
        }

        if self.is_positive_variable(edge)? {
            write_encoding(writer, POSVAR_ENCODING, index_size)?;
            let variable = self.branch_variable(edge)?;
            return write_normalized_index(writer, normalized_indexes, variable, index_size);
        }

        if self.is_negative_variable(edge)? {
            write_encoding(writer, NEGVAR_ENCODING, index_size)?;
            let variable = self.branch_variable(edge)?;
            return write_normalized_index(writer, normalized_indexes, variable, index_size);
        }

        if let BddNode::Terminal(first, second) = self.node(BddEdge::regular(edge.node))? {
            write_encoding(writer, CONSTANT_ENCODING, index_size)?;
            let (first, second) = if edge.is_complemented() {
                (-*first, -*second)
            } else {
                (*first, *second)
            };
            write_signed(writer, first, LEGACY_LONG_BYTES)?;
            return write_signed(writer, second, LEGACY_LONG_BYTES);
        }

        if let Some(number) = shared_numbers.get(&edge.node).copied() {
            if !labelled.insert(edge.node) {
                let encoding = if edge.is_complemented() {
                    NEGNODE_ENCODING
                } else {
                    POSNODE_ENCODING
                };

                write_encoding(writer, encoding, index_size)?;
                return write_unsigned(writer, number as u64, node_number_size);
            }

            write_encoding(writer, NODELABEL_ENCODING, index_size)?;
        }

        let variable = self.branch_variable(edge)?;
        write_normalized_index(writer, normalized_indexes, variable, index_size)?;
        let (then_edge, else_edge) = self.branches(edge)?;
        self.dump_step(
            then_edge,
            writer,
            normalized_indexes,
            shared_numbers,
            labelled,
            index_size,
            node_number_size,
        )?;
        self.dump_step(
            else_edge,
            writer,
            normalized_indexes,
            shared_numbers,
            labelled,
            index_size,
            node_number_size,
        )
    }

    fn undump_step(
        &mut self,
        support: &[BddEdge],
        reader: &mut impl Read,
        shared: &mut [Option<BddEdge>],
        shared_so_far: &mut usize,
        index_size: usize,
        node_number_size: usize,
    ) -> Result<BddEdge, BddUndumpError> {
        let index = read_unsigned(reader, index_size)? as usize;
        if index == index_mask(index_size) {
            let encoding = 0xffff_ff00 | read_unsigned(reader, 1)? as u32;
            return self.undump_encoded(
                encoding,
                support,
                reader,
                shared,
                shared_so_far,
                index_size,
                node_number_size,
            );
        }

        if index >= support.len() {
            return Err(BddUndumpError::Format);
        }

        let then_edge = self.undump_step(
            support,
            reader,
            shared,
            shared_so_far,
            index_size,
            node_number_size,
        )?;
        let else_edge = self.undump_step(
            support,
            reader,
            shared,
            shared_so_far,
            index_size,
            node_number_size,
        )?;

        self.ite(support[index], then_edge, else_edge)
            .map_err(BddUndumpError::Dump)
    }

    fn undump_encoded(
        &mut self,
        encoding: u32,
        support: &[BddEdge],
        reader: &mut impl Read,
        shared: &mut [Option<BddEdge>],
        shared_so_far: &mut usize,
        index_size: usize,
        node_number_size: usize,
    ) -> Result<BddEdge, BddUndumpError> {
        match encoding {
            TRUE_ENCODING => Ok(self.one()),
            FALSE_ENCODING => Ok(self.zero()),
            CONSTANT_ENCODING => {
                let first = read_signed(reader, LEGACY_LONG_BYTES)?;
                let second = read_signed(reader, LEGACY_LONG_BYTES)?;
                Ok(self.terminal(first, second))
            }
            POSVAR_ENCODING | NEGVAR_ENCODING => {
                let index = read_unsigned(reader, index_size)? as usize;
                if index >= support.len() {
                    return Err(BddUndumpError::Format);
                }

                if encoding == POSVAR_ENCODING {
                    Ok(support[index])
                } else {
                    Ok(self.not(support[index]))
                }
            }
            POSNODE_ENCODING | NEGNODE_ENCODING => {
                let node_number = read_unsigned(reader, node_number_size)? as usize;
                let Some(edge) = shared.get(node_number).and_then(|edge| *edge) else {
                    return Err(BddUndumpError::Format);
                };

                if encoding == POSNODE_ENCODING {
                    Ok(edge)
                } else {
                    Ok(self.not(edge))
                }
            }
            NODELABEL_ENCODING => {
                let node_number = *shared_so_far;
                *shared_so_far += 1;
                if node_number >= shared.len() {
                    return Err(BddUndumpError::Format);
                }

                let edge = self.undump_step(
                    support,
                    reader,
                    shared,
                    shared_so_far,
                    index_size,
                    node_number_size,
                )?;
                shared[node_number] = Some(edge);
                Ok(edge)
            }
            _ => Err(BddUndumpError::Format),
        }
    }

    fn normalized_indexes(
        &self,
        root: BddEdge,
        support: &[BddEdge],
    ) -> Result<BTreeMap<BddVariableId, usize>, BddDumpError> {
        let mut normalized_indexes = BTreeMap::new();

        for (position, edge) in support.iter().copied().enumerate() {
            if !self.is_positive_variable(edge)? {
                return Err(BddDumpError::SupportNotPositiveVariable(edge));
            }

            let variable = self.branch_variable(edge)?;
            if normalized_indexes.insert(variable, position).is_some() {
                return Err(BddDumpError::DuplicateSupportVariable(variable));
            }
        }

        for variable in self.support_variables(root)? {
            if !normalized_indexes.contains_key(&variable) {
                return Err(BddDumpError::IncompleteSupport(variable));
            }
        }

        Ok(normalized_indexes)
    }

    fn support_variables(&self, root: BddEdge) -> Result<BTreeSet<BddVariableId>, BddDumpError> {
        let mut support = BTreeSet::new();
        let mut visited = BTreeSet::new();
        self.support_step(root, &mut support, &mut visited)?;
        Ok(support)
    }

    fn support_step(
        &self,
        edge: BddEdge,
        support: &mut BTreeSet<BddVariableId>,
        visited: &mut BTreeSet<BddNodeId>,
    ) -> Result<(), BddDumpError> {
        if edge == self.zero() || edge == self.one() {
            return Ok(());
        }

        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Terminal(_, _) => Ok(()),
            BddNode::Branch { variable, .. } => {
                if !visited.insert(edge.node) {
                    return Ok(());
                }

                support.insert(*variable);
                let (then_edge, else_edge) = self.branches(edge)?;
                self.support_step(then_edge, support, visited)?;
                self.support_step(else_edge, support, visited)
            }
        }
    }

    fn shared_numbers(&self, root: BddEdge) -> Result<BTreeMap<BddNodeId, usize>, BddDumpError> {
        let mut counts = BTreeMap::new();
        self.count_nodes(root, &mut counts)?;

        let mut numbers = BTreeMap::new();
        for (node, count) in counts {
            if count > 1 {
                numbers.insert(node, numbers.len());
            }
        }

        Ok(numbers)
    }

    fn count_nodes(
        &self,
        edge: BddEdge,
        counts: &mut BTreeMap<BddNodeId, usize>,
    ) -> Result<(), BddDumpError> {
        if edge == self.zero() || edge == self.one() {
            return Ok(());
        }

        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Terminal(_, _) => Ok(()),
            BddNode::Branch { .. } => {
                *counts.entry(edge.node).or_default() += 1;
                let (then_edge, else_edge) = self.branches(edge)?;
                self.count_nodes(then_edge, counts)?;
                self.count_nodes(else_edge, counts)
            }
        }
    }

    fn ite_inner(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, BddDumpError> {
        if condition == self.one() {
            return Ok(then_edge);
        }

        if condition == self.zero() {
            return Ok(else_edge);
        }

        if then_edge == else_edge {
            return Ok(then_edge);
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
    ) -> Result<(BddEdge, BddEdge), BddDumpError> {
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

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), BddDumpError> {
        match self.node(BddEdge::regular(edge.node))? {
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
            BddNode::Terminal(_, _) => Err(BddDumpError::ExpectedBranch(edge.node)),
        }
    }

    fn branch_variable(&self, edge: BddEdge) -> Result<BddVariableId, BddDumpError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Branch { variable, .. } => Ok(*variable),
            BddNode::Terminal(_, _) => Err(BddDumpError::ExpectedBranch(edge.node)),
        }
    }

    fn sort_variable(&self, edge: BddEdge) -> Result<BddVariableId, BddDumpError> {
        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Branch { variable, .. } => Ok(*variable),
            BddNode::Terminal(_, _) => Ok(BddVariableId::MAX),
        }
    }

    fn is_positive_variable(&self, edge: BddEdge) -> Result<bool, BddDumpError> {
        if edge.is_complemented() {
            return Ok(false);
        }

        match self.node(edge)? {
            BddNode::Branch {
                then_edge,
                else_edge,
                ..
            } => Ok(*then_edge == self.one() && *else_edge == self.zero()),
            BddNode::Terminal(_, _) => Ok(false),
        }
    }

    fn is_negative_variable(&self, edge: BddEdge) -> Result<bool, BddDumpError> {
        if !edge.is_complemented() {
            return Ok(false);
        }

        match self.node(BddEdge::regular(edge.node))? {
            BddNode::Branch {
                then_edge,
                else_edge,
                ..
            } => Ok(*then_edge == self.one() && *else_edge == self.zero()),
            BddNode::Terminal(_, _) => Ok(false),
        }
    }

    fn validate_edge(&self, edge: BddEdge) -> Result<(), BddDumpError> {
        self.nodes
            .get(edge.node)
            .map(|_| ())
            .ok_or(BddDumpError::MissingNode(edge.node))
    }

    fn validate_order(&self, parent: BddVariableId, child: BddEdge) -> Result<(), BddDumpError> {
        let child_variable = self.sort_variable(child)?;
        if child_variable == BddVariableId::MAX || parent < child_variable {
            Ok(())
        } else {
            Err(BddDumpError::VariableOrder {
                parent,
                child: child_variable,
            })
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
        if let Some(edge) = self.branches.get(&key).copied() {
            return edge;
        }

        let edge = BddEdge::regular(self.nodes.len());
        self.nodes.push(BddNode::Branch {
            variable,
            then_edge,
            else_edge,
        });
        self.branches.insert(key, edge);
        edge
    }

    fn insert_terminal_unchecked(&mut self, first: BddValue, second: BddValue) -> BddEdge {
        let edge = BddEdge::regular(self.nodes.len());
        self.nodes.push(BddNode::Terminal(first, second));
        self.terminals.insert((first, second), edge);
        edge
    }
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddDumpError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
    SupportNotPositiveVariable(BddEdge),
    DuplicateSupportVariable(BddVariableId),
    IncompleteSupport(BddVariableId),
    VariableOrder {
        parent: BddVariableId,
        child: BddVariableId,
    },
    IntegerOverflow {
        value: u64,
        bytes: usize,
    },
    Io(String),
}

impl fmt::Display for BddDumpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch node"),
            Self::SupportNotPositiveVariable(edge) => {
                write!(
                    formatter,
                    "support edge {edge:?} is not a positive variable"
                )
            }
            Self::DuplicateSupportVariable(variable) => {
                write!(formatter, "support variable {variable} is duplicated")
            }
            Self::IncompleteSupport(variable) => {
                write!(formatter, "support is missing variable {variable}")
            }
            Self::VariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
            Self::IntegerOverflow { value, bytes } => {
                write!(formatter, "integer {value} does not fit in {bytes} byte(s)")
            }
            Self::Io(message) => formatter.write_str(message),
        }
    }
}

impl Error for BddDumpError {}

impl From<io::Error> for BddDumpError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddUndumpError {
    Dump(BddDumpError),
    SupportNotPositiveVariable(BddEdge),
    Eof,
    Format,
    Io(String),
}

impl fmt::Display for BddUndumpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dump(error) => error.fmt(formatter),
            Self::SupportNotPositiveVariable(edge) => {
                write!(
                    formatter,
                    "support edge {edge:?} is not a positive variable"
                )
            }
            Self::Eof => formatter.write_str("unexpected end of BDD dump"),
            Self::Format => formatter.write_str("invalid BDD dump format"),
            Self::Io(message) => formatter.write_str(message),
        }
    }
}

impl Error for BddUndumpError {}

impl From<io::Error> for BddUndumpError {
    fn from(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            Self::Eof
        } else {
            Self::Io(error.to_string())
        }
    }
}

fn bytes_needed(number: usize) -> usize {
    if number <= 0x100 {
        1
    } else if number <= 0x1_0000 {
        2
    } else if number <= 0x100_0000 {
        3
    } else {
        4
    }
}

fn index_mask(bytes: usize) -> usize {
    match bytes {
        1 => 0xff,
        2 => 0xffff,
        3 => 0xff_ffff,
        _ => 0xffff_ffff,
    }
}

fn complement_terminal(first: BddValue, second: BddValue) -> (BddValue, BddValue) {
    match (first, second) {
        (0, 0) => (1, 0),
        (1, 0) => (0, 0),
        _ => (-first, -second),
    }
}

fn write_encoding(
    writer: &mut impl Write,
    encoding: u32,
    index_size: usize,
) -> Result<(), BddDumpError> {
    write_truncated_unsigned(writer, encoding as u64, index_size + 1)
}

fn write_normalized_index(
    writer: &mut impl Write,
    normalized_indexes: &BTreeMap<BddVariableId, usize>,
    variable: BddVariableId,
    index_size: usize,
) -> Result<(), BddDumpError> {
    let index = normalized_indexes
        .get(&variable)
        .copied()
        .ok_or(BddDumpError::IncompleteSupport(variable))?;
    write_unsigned(writer, index as u64, index_size)
}

fn write_signed(
    writer: &mut impl Write,
    value: BddValue,
    bytes: usize,
) -> Result<(), BddDumpError> {
    write_unsigned(writer, value as u32 as u64, bytes)
}

fn write_unsigned(writer: &mut impl Write, value: u64, bytes: usize) -> Result<(), BddDumpError> {
    if bytes < 8 && value > ((1_u64 << (bytes * 8)) - 1) {
        return Err(BddDumpError::IntegerOverflow { value, bytes });
    }

    for shift in (0..bytes).rev().map(|byte| byte * 8) {
        writer.write_all(&[((value >> shift) & 0xff) as u8])?;
    }

    Ok(())
}

fn write_truncated_unsigned(
    writer: &mut impl Write,
    value: u64,
    bytes: usize,
) -> Result<(), BddDumpError> {
    for shift in (0..bytes).rev().map(|byte| byte * 8) {
        writer.write_all(&[((value >> shift) & 0xff) as u8])?;
    }

    Ok(())
}

fn read_signed(reader: &mut impl Read, bytes: usize) -> Result<BddValue, BddUndumpError> {
    Ok(read_unsigned(reader, bytes)? as u32 as i32)
}

fn read_unsigned(reader: &mut impl Read, bytes: usize) -> Result<u64, BddUndumpError> {
    let mut result = 0_u64;
    let mut byte = [0_u8; 1];

    for _ in 0..bytes {
        reader.read_exact(&mut byte)?;
        result = (result << 8) | u64::from(byte[0]);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(entries: &[(BddVariableId, bool)]) -> HashMap<BddVariableId, bool> {
        entries.iter().copied().collect()
    }

    fn sample_function() -> (BddManager, BddEdge, BddEdge, BddEdge, BddEdge) {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);
        let terminal = manager.terminal(7, -3);
        let shared = manager.find_or_add(2, terminal, manager.zero()).unwrap();
        let root = manager.find_or_add(1, shared, shared).unwrap();

        (manager, x, y, terminal, root)
    }

    #[test]
    fn dumps_and_undumps_bdd_with_shared_nodes() {
        let (manager, x, y, _, root) = sample_function();
        let mut bytes = Vec::new();

        manager.dump_bdd(root, &[x, y], &mut bytes).unwrap();

        let mut restored = BddManager::new();
        let restored_x = restored.variable(1);
        let restored_y = restored.variable(2);
        let restored_root = restored
            .undump_bdd(&[restored_x, restored_y], bytes.as_slice())
            .unwrap();

        for x_value in [false, true] {
            for y_value in [false, true] {
                let assignment = values(&[(1, x_value), (2, y_value)]);
                assert_eq!(
                    restored.eval(restored_root, &assignment).unwrap(),
                    manager.eval(root, &assignment).unwrap()
                );
            }
        }
    }

    #[test]
    fn preserves_positive_and_negative_variable_encodings() {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let root = manager.not(x);
        let mut bytes = Vec::new();

        manager.dump_bdd(root, &[x], &mut bytes).unwrap();

        let mut restored = BddManager::new();
        let restored_x = restored.variable(1);
        let restored_root = restored
            .undump_bdd(&[restored_x], bytes.as_slice())
            .unwrap();

        assert_eq!(
            restored
                .eval(restored_root, &values(&[(1, false)]))
                .unwrap(),
            (1, 0)
        );
        assert_eq!(
            restored.eval(restored_root, &values(&[(1, true)])).unwrap(),
            (0, 0)
        );
    }

    #[test]
    fn rejects_incomplete_or_duplicate_support() {
        let (manager, x, _, _, root) = sample_function();

        assert_eq!(
            manager.dump_bdd(root, &[x], Vec::new()).unwrap_err(),
            BddDumpError::IncompleteSupport(2)
        );
        assert_eq!(
            manager.dump_bdd(root, &[x, x], Vec::new()).unwrap_err(),
            BddDumpError::DuplicateSupportVariable(1)
        );
    }

    #[test]
    fn rejects_bad_magic_and_wrong_support_count() {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let mut bytes = Vec::new();
        manager.dump_bdd(x, &[x], &mut bytes).unwrap();
        bytes[0] = 0;

        assert_eq!(
            manager.undump_bdd(&[x], bytes.as_slice()).unwrap_err(),
            BddUndumpError::Format
        );

        let mut bytes = Vec::new();
        manager.dump_bdd(x, &[x], &mut bytes).unwrap();
        assert_eq!(
            manager.undump_bdd(&[], bytes.as_slice()).unwrap_err(),
            BddUndumpError::Format
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bdddump.rs");
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
