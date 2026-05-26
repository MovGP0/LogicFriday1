//! Native node-name management for the SIS node package.
//!
//! The legacy implementation kept process-global name mode and generated-name
//! counters. This port keeps the same name generation and display-name rules,
//! but owns the mutable state in `NodeNames`.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub type NameResult<T> = Result<T, NameError>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameMode {
    Long,
    Short,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamedNodeKind {
    Unassigned,
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedNode {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub kind: NamedNodeKind,
    pub fanins: Vec<NodeId>,
    pub fanouts: BTreeSet<NodeId>,
    pub real_primary_output: bool,
}

impl NamedNode {
    pub fn new(kind: NamedNodeKind) -> Self {
        Self {
            name: None,
            short_name: None,
            kind,
            fanins: Vec::new(),
            fanouts: BTreeSet::new(),
            real_primary_output: true,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.name = Some(name.clone());
        self.short_name = Some(name);
        self
    }

    pub fn with_short_name(mut self, short_name: impl Into<String>) -> Self {
        self.short_name = Some(short_name.into());
        self
    }

    pub fn with_fanins(mut self, fanins: impl Into<Vec<NodeId>>) -> Self {
        self.fanins = fanins.into();
        self
    }

    pub fn unreal_primary_output(mut self) -> Self {
        self.real_primary_output = false;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NameError {
    UnknownNode(NodeId),
    MissingLongName(NodeId),
    MissingShortName(NodeId),
    DuplicateLongName(String),
    DuplicateShortName(String),
    InvalidPrimaryOutput(NodeId),
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown named node {:?}", node),
            Self::MissingLongName(node) => write!(f, "missing long node name for {:?}", node),
            Self::MissingShortName(node) => write!(f, "missing short node name for {:?}", node),
            Self::DuplicateLongName(name) => write!(f, "duplicate long node name {name}"),
            Self::DuplicateShortName(name) => write!(f, "duplicate short node name {name}"),
            Self::InvalidPrimaryOutput(node) => {
                write!(f, "primary output {:?} must have exactly one fanin", node)
            }
        }
    }
}

impl Error for NameError {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeNames {
    nodes: Vec<NamedNode>,
    long_name_table: BTreeMap<String, NodeId>,
    short_name_table: BTreeMap<String, NodeId>,
    long_name_index: usize,
    short_name_index: usize,
}

impl NodeNames {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, mut node: NamedNode) -> NameResult<NodeId> {
        for fanin in &node.fanins {
            self.node(*fanin)?;
        }

        if node.kind == NamedNodeKind::PrimaryOutput && node.fanins.len() != 1 {
            return Err(NameError::InvalidPrimaryOutput(NodeId(self.nodes.len())));
        }

        if node.name.is_none() {
            node.name = Some(self.next_long_name());
        }

        if node.short_name.is_none() {
            node.short_name = Some(self.next_short_name());
        }

        if let Some(name) = &node.name {
            if self.long_name_table.contains_key(name) {
                return Err(NameError::DuplicateLongName(name.clone()));
            }
        }

        if let Some(name) = &node.short_name {
            if self.short_name_table.contains_key(name) {
                return Err(NameError::DuplicateShortName(name.clone()));
            }
        }

        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        for fanin in self.nodes[id.0].fanins.clone() {
            self.node_mut(fanin)?.fanouts.insert(id);
        }
        self.rehash_names(true, true)?;
        Ok(id)
    }

    pub fn node(&self, node: NodeId) -> NameResult<&NamedNode> {
        self.nodes.get(node.0).ok_or(NameError::UnknownNode(node))
    }

    pub fn node_mut(&mut self, node: NodeId) -> NameResult<&mut NamedNode> {
        self.nodes
            .get_mut(node.0)
            .ok_or(NameError::UnknownNode(node))
    }

    pub fn nodes(&self) -> &[NamedNode] {
        &self.nodes
    }

    pub fn find_long_name(&self, name: &str) -> Option<NodeId> {
        self.long_name_table.get(name).copied()
    }

    pub fn find_short_name(&self, name: &str) -> Option<NodeId> {
        self.short_name_table.get(name).copied()
    }

    pub fn node_name(&mut self, node: NodeId, mode: NameMode) -> NameResult<String> {
        self.ensure_name(node, mode)?;
        self.decorated_name(node, mode)
    }

    pub fn node_long_name(&self, node: NodeId) -> NameResult<Option<&str>> {
        Ok(self.node(node)?.name.as_deref())
    }

    pub fn assign_long_name(&mut self, node: NodeId) -> NameResult<String> {
        let name = self.next_long_name();
        self.node_mut(node)?.name = Some(name.clone());
        self.rehash_names(true, false)?;
        Ok(name)
    }

    pub fn assign_short_name(&mut self, node: NodeId) -> NameResult<String> {
        let name = self.next_short_name();
        self.node_mut(node)?.short_name = Some(name.clone());
        self.rehash_names(false, true)?;
        Ok(name)
    }

    pub fn reset_long_names(&mut self) -> NameResult<()> {
        self.long_name_index = 0;
        for index in 0..self.nodes.len() {
            let should_reset = self.nodes[index]
                .name
                .as_deref()
                .and_then(madeup_name_value)
                .is_some();

            if should_reset {
                let name = self.next_long_name();
                self.nodes[index].name = Some(name);
            }
        }

        self.rehash_names(true, false)
    }

    pub fn reset_short_names(&mut self) -> NameResult<()> {
        self.short_name_index = 0;
        let order = self
            .ordered_nodes_for_short_name_reset()
            .into_iter()
            .collect::<Vec<_>>();

        for node in order {
            let name = self.next_short_name();
            self.node_mut(node)?.short_name = Some(name);
        }

        self.rehash_names(false, true)
    }

    pub fn rehash_names(&mut self, long_name: bool, short_name: bool) -> NameResult<()> {
        if long_name {
            let mut table = BTreeMap::new();
            for (index, node) in self.nodes.iter().enumerate() {
                if let Some(name) = &node.name {
                    if table.insert(name.clone(), NodeId(index)).is_some() {
                        return Err(NameError::DuplicateLongName(name.clone()));
                    }
                }
            }
            self.long_name_table = table;
        }

        if short_name {
            let mut table = BTreeMap::new();
            for (index, node) in self.nodes.iter().enumerate() {
                if let Some(name) = &node.short_name {
                    if table.insert(name.clone(), NodeId(index)).is_some() {
                        return Err(NameError::DuplicateShortName(name.clone()));
                    }
                }
            }
            self.short_name_table = table;
        }

        Ok(())
    }

    fn ensure_name(&mut self, node: NodeId, mode: NameMode) -> NameResult<()> {
        match mode {
            NameMode::Long if self.node(node)?.name.is_none() => {
                self.assign_long_name(node)?;
            }
            NameMode::Short if self.node(node)?.short_name.is_none() => {
                self.assign_short_name(node)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn decorated_name(&self, node: NodeId, mode: NameMode) -> NameResult<String> {
        let named_node = self.node(node)?;
        let name = self.raw_name(node, mode)?.to_owned();

        match named_node.kind {
            NamedNodeKind::Unassigned | NamedNodeKind::PrimaryInput => Ok(name),
            NamedNodeKind::PrimaryOutput => {
                let mut display_name = name;
                if !named_node.real_primary_output {
                    let fanin = named_node.fanins[0];
                    if self.primary_output_fanout_count(fanin)? == 1 {
                        display_name = self.raw_name(fanin, mode)?.to_owned();
                    }
                }

                Ok(format!("{{{display_name}}}"))
            }
            NamedNodeKind::Internal => {
                let outputs = named_node
                    .fanouts
                    .iter()
                    .copied()
                    .filter(|fanout| {
                        self.node(*fanout)
                            .map(|node| {
                                node.kind == NamedNodeKind::PrimaryOutput
                                    && node.real_primary_output
                            })
                            .unwrap_or(false)
                    })
                    .map(|fanout| self.raw_name(fanout, mode).map(ToOwned::to_owned))
                    .collect::<NameResult<Vec<_>>>()?;

                if outputs.is_empty() {
                    Ok(name)
                } else {
                    Ok(format!("{{{}}}", outputs.join(",")))
                }
            }
        }
    }

    fn raw_name(&self, node_id: NodeId, mode: NameMode) -> NameResult<&str> {
        let node = self.node(node_id)?;
        match mode {
            NameMode::Long => node
                .name
                .as_deref()
                .ok_or(NameError::MissingLongName(node_id)),
            NameMode::Short => node
                .short_name
                .as_deref()
                .ok_or(NameError::MissingShortName(node_id)),
        }
    }

    fn primary_output_fanout_count(&self, node: NodeId) -> NameResult<usize> {
        Ok(self
            .node(node)?
            .fanouts
            .iter()
            .filter(|fanout| {
                self.node(**fanout)
                    .map(|node| node.kind == NamedNodeKind::PrimaryOutput)
                    .unwrap_or(false)
            })
            .count())
    }

    fn ordered_nodes_for_short_name_reset(&self) -> Vec<NodeId> {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for kind in [NamedNodeKind::PrimaryInput, NamedNodeKind::PrimaryOutput] {
            nodes.extend(
                self.nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, node)| node.kind == kind)
                    .map(|(index, _)| NodeId(index)),
            );
        }

        nodes.extend(
            self.nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| {
                    node.kind != NamedNodeKind::PrimaryInput
                        && node.kind != NamedNodeKind::PrimaryOutput
                })
                .map(|(index, _)| NodeId(index)),
        );

        nodes
    }

    fn next_long_name(&mut self) -> String {
        let name = format!("[{}]", self.long_name_index);
        self.long_name_index += 1;
        name
    }

    fn next_short_name(&mut self) -> String {
        let character = (b'a' + (self.short_name_index % 26) as u8) as char;
        let suffix = self.short_name_index / 26;
        self.short_name_index += 1;

        if suffix == 0 {
            character.to_string()
        } else {
            format!("{}{}", character, suffix - 1)
        }
    }
}

pub fn madeup_name_value(name: &str) -> Option<i32> {
    if !name.starts_with('[') || !name.ends_with(']') {
        return None;
    }

    name[1..name.len() - 1].parse().ok()
}

pub fn is_madeup_name(name: &str) -> bool {
    madeup_name_value(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_names() -> (NodeNames, NodeId, NodeId, NodeId, NodeId) {
        let mut names = NodeNames::new();
        let a = names
            .add_node(NamedNode::new(NamedNodeKind::PrimaryInput).with_name("a_in"))
            .unwrap();
        let b = names
            .add_node(NamedNode::new(NamedNodeKind::PrimaryInput).with_name("b_in"))
            .unwrap();
        let n = names
            .add_node(
                NamedNode::new(NamedNodeKind::Internal)
                    .with_name("n1")
                    .with_short_name("n")
                    .with_fanins(vec![a, b]),
            )
            .unwrap();
        let y = names
            .add_node(
                NamedNode::new(NamedNodeKind::PrimaryOutput)
                    .with_name("y")
                    .with_fanins(vec![n]),
            )
            .unwrap();

        (names, a, b, n, y)
    }

    #[test]
    fn generated_long_names_match_legacy_sequence() {
        let mut names = NodeNames::new();
        let first = names
            .add_node(NamedNode::new(NamedNodeKind::Internal))
            .unwrap();
        let second = names
            .add_node(NamedNode::new(NamedNodeKind::Internal))
            .unwrap();

        assert_eq!(names.node(first).unwrap().name.as_deref(), Some("[0]"));
        assert_eq!(names.node(second).unwrap().name.as_deref(), Some("[1]"));
    }

    #[test]
    fn generated_short_names_match_legacy_base_twenty_six_sequence() {
        let mut names = NodeNames::new();
        let nodes = (0..28)
            .map(|_| {
                names
                    .add_node(NamedNode::new(NamedNodeKind::Internal))
                    .unwrap()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            names.node(nodes[0]).unwrap().short_name.as_deref(),
            Some("a")
        );
        assert_eq!(
            names.node(nodes[25]).unwrap().short_name.as_deref(),
            Some("z")
        );
        assert_eq!(
            names.node(nodes[26]).unwrap().short_name.as_deref(),
            Some("a0")
        );
        assert_eq!(
            names.node(nodes[27]).unwrap().short_name.as_deref(),
            Some("b0")
        );
    }

    #[test]
    fn node_name_wraps_primary_outputs_and_reports_real_output_fanouts() {
        let (mut names, _, _, n, y) = sample_names();

        assert_eq!(names.node_name(y, NameMode::Long).unwrap(), "{y}");
        assert_eq!(names.node_name(n, NameMode::Long).unwrap(), "{y}");
        assert_eq!(names.node_name(n, NameMode::Short).unwrap(), "{y}");
    }

    #[test]
    fn unreal_primary_output_uses_single_fanin_name_when_it_is_the_only_output() {
        let mut names = NodeNames::new();
        let a = names
            .add_node(NamedNode::new(NamedNodeKind::PrimaryInput).with_name("a"))
            .unwrap();
        let fake = names
            .add_node(
                NamedNode::new(NamedNodeKind::PrimaryOutput)
                    .with_name("fake")
                    .with_fanins(vec![a])
                    .unreal_primary_output(),
            )
            .unwrap();

        assert_eq!(names.node_name(fake, NameMode::Long).unwrap(), "{a}");
    }

    #[test]
    fn reset_long_names_only_replaces_madeup_names() {
        let (mut names, a, _, n, _) = sample_names();
        names.node_mut(a).unwrap().name = Some("[42]".to_string());
        names.node_mut(n).unwrap().name = Some("logic".to_string());

        names.reset_long_names().unwrap();

        assert_eq!(names.node(a).unwrap().name.as_deref(), Some("[0]"));
        assert_eq!(names.node(n).unwrap().name.as_deref(), Some("logic"));
        assert_eq!(names.find_long_name("[0]"), Some(a));
        assert_eq!(names.find_long_name("logic"), Some(n));
    }

    #[test]
    fn reset_short_names_uses_primary_inputs_outputs_then_internal_nodes() {
        let (mut names, a, b, n, y) = sample_names();

        names.reset_short_names().unwrap();

        assert_eq!(names.node(a).unwrap().short_name.as_deref(), Some("a"));
        assert_eq!(names.node(b).unwrap().short_name.as_deref(), Some("b"));
        assert_eq!(names.node(y).unwrap().short_name.as_deref(), Some("c"));
        assert_eq!(names.node(n).unwrap().short_name.as_deref(), Some("d"));
        assert_eq!(names.find_short_name("d"), Some(n));
    }

    #[test]
    fn duplicate_names_are_reported_during_rehash() {
        let (mut names, a, b, _, _) = sample_names();
        names.node_mut(b).unwrap().name = names.node(a).unwrap().name.clone();

        assert_eq!(
            names.rehash_names(true, false),
            Err(NameError::DuplicateLongName("a_in".to_string()))
        );
    }

    #[test]
    fn madeup_name_parser_requires_outer_brackets_and_integer_body() {
        assert_eq!(madeup_name_value("[17]"), Some(17));
        assert_eq!(madeup_name_value("[-3]"), Some(-3));
        assert_eq!(madeup_name_value("17"), None);
        assert_eq!(madeup_name_value("[x]"), None);
        assert_eq!(madeup_name_value("[17]x"), None);
        assert!(is_madeup_name("[0]"));
        assert!(!is_madeup_name("node"));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("names.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
