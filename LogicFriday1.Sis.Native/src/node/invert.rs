//! Native inversion operations for SIS-style Boolean nodes.

use super::node::{Cover, Cube, Node, NodeError, NodeResult, NodeType, node_literal, node_not};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub fn node_invert(node: &mut Node) -> NodeResult<bool> {
    if matches!(
        node.node_type,
        NodeType::PrimaryInput | NodeType::PrimaryOutput
    ) {
        return Ok(false);
    }

    let name = node.name.clone();
    let short_name = node.short_name.clone();
    let mut inverted = node_not(node)?;
    inverted.name = name;
    inverted.short_name = short_name;
    inverted.node_type = node.node_type;
    node.replace_with(inverted);
    Ok(true)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InvertNetwork {
    nodes: BTreeMap<String, Node>,
}

impl InvertNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, name: impl Into<String>, mut node: Node) -> InvertResult<()> {
        let name = name.into();
        if self.nodes.contains_key(&name) {
            return Err(InvertError::DuplicateNode { node: name });
        }

        node.name = Some(name.clone());
        self.nodes.insert(name, node);
        Ok(())
    }

    pub fn node(&self, name: &str) -> InvertResult<&Node> {
        self.nodes
            .get(name)
            .ok_or_else(|| InvertError::UnknownNode {
                node: name.to_owned(),
            })
    }

    pub fn node_mut(&mut self, name: &str) -> InvertResult<&mut Node> {
        self.nodes
            .get_mut(name)
            .ok_or_else(|| InvertError::UnknownNode {
                node: name.to_owned(),
            })
    }

    pub fn node_names(&self) -> impl Iterator<Item = &str> {
        self.nodes.keys().map(String::as_str)
    }

    pub fn invert_node(&mut self, name: &str) -> InvertResult<bool> {
        let changed = {
            let node = self.node_mut(name)?;
            node_invert(node)?
        };

        if !changed {
            return Ok(false);
        }

        let fanouts = self.fanouts(name);
        let mut primary_outputs = Vec::new();

        for fanout_name in fanouts {
            let fanout = self.node_mut(&fanout_name)?;
            match fanout.node_type {
                NodeType::Internal => {
                    flip_fanin_phase(fanout, name)?;
                    fanout.is_dup_free = false;
                    fanout.is_scc_minimal = false;
                }
                NodeType::PrimaryOutput => {
                    primary_outputs.push(fanout_name);
                }
                NodeType::PrimaryInput => {
                    return Err(InvertError::InvalidFanoutType {
                        node: fanout_name,
                        node_type: NodeType::PrimaryInput,
                    });
                }
            }
        }

        if !primary_outputs.is_empty() {
            let inverter_name = self.unique_inverter_name(name);
            let mut inverter = node_literal(name.to_owned(), 0)?;
            inverter.name = Some(inverter_name.clone());
            self.add_node(inverter_name.clone(), inverter)?;

            for output_name in primary_outputs {
                patch_fanin(self.node_mut(&output_name)?, name, &inverter_name)?;
            }
        }

        Ok(true)
    }

    fn fanouts(&self, name: &str) -> Vec<String> {
        self.nodes
            .iter()
            .filter_map(|(candidate_name, node)| {
                node.fanins
                    .iter()
                    .any(|fanin| fanin == name)
                    .then_some(candidate_name.clone())
            })
            .collect()
    }

    fn unique_inverter_name(&self, name: &str) -> String {
        let base = format!("{name}_inv");
        if !self.nodes.contains_key(&base) {
            return base;
        }

        for index in 1.. {
            let candidate = format!("{base}_{index}");
            if !self.nodes.contains_key(&candidate) {
                return candidate;
            }
        }

        unreachable!("unbounded inverter name search must find an unused name")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvertError {
    Node(NodeError),
    UnknownNode { node: String },
    DuplicateNode { node: String },
    MissingFanin { node: String, fanin: String },
    InvalidFanoutType { node: String, node_type: NodeType },
}

impl fmt::Display for InvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node(error) => write!(f, "{error}"),
            Self::UnknownNode { node } => write!(f, "unknown node '{node}'"),
            Self::DuplicateNode { node } => write!(f, "duplicate node '{node}'"),
            Self::MissingFanin { node, fanin } => {
                write!(f, "node '{node}' does not use fanin '{fanin}'")
            }
            Self::InvalidFanoutType { node, node_type } => {
                write!(f, "node '{node}' has invalid fanout type {node_type:?}")
            }
        }
    }
}

impl Error for InvertError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Node(error) => Some(error),
            _ => None,
        }
    }
}

impl From<NodeError> for InvertError {
    fn from(value: NodeError) -> Self {
        Self::Node(value)
    }
}

pub type InvertResult<T> = Result<T, InvertError>;

fn flip_fanin_phase(node: &mut Node, fanin: &str) -> InvertResult<()> {
    let index = node
        .fanins
        .iter()
        .position(|candidate| candidate == fanin)
        .ok_or_else(|| InvertError::MissingFanin {
            node: display_name(node),
            fanin: fanin.to_owned(),
        })?;

    let function = node.function().ok_or(NodeError::MissingFunction {
        operation: "node_invert",
    })?;
    let cubes = function
        .cubes()
        .iter()
        .map(|cube| {
            let mut inputs = cube.inputs().to_vec();
            if let Some(value) = inputs[index] {
                inputs[index] = Some(!value);
            }

            Cube::new(inputs)
        })
        .collect();
    let cover = Cover::new(function.input_count(), cubes)?;
    let mut replacement = Node::new(cover, node.fanins.clone());
    replacement.name = node.name.clone();
    replacement.short_name = node.short_name.clone();
    replacement.node_type = node.node_type;
    replacement.is_dup_free = false;
    replacement.is_scc_minimal = false;
    node.replace_with(replacement);
    Ok(())
}

fn patch_fanin(node: &mut Node, old_fanin: &str, new_fanin: &str) -> InvertResult<()> {
    let Some(index) = node.fanins.iter().position(|fanin| fanin == old_fanin) else {
        return Err(InvertError::MissingFanin {
            node: display_name(node),
            fanin: old_fanin.to_owned(),
        });
    };

    node.fanins[index] = new_fanin.to_owned();
    Ok(())
}

fn display_name(node: &Node) -> String {
    node.name
        .clone()
        .or_else(|| node.short_name.clone())
        .unwrap_or_else(|| "<unnamed>".to_owned())
}

#[cfg(test)]
mod tests {
    use super::super::node::{
        NodeFunction, node_and, node_contains, node_function, node_or, node_xor,
    };
    use super::*;

    fn lit(name: &str, phase: i32) -> Node {
        node_literal(name, phase).unwrap()
    }

    #[test]
    fn inverts_internal_node_in_place_and_preserves_names() {
        let a = lit("a", 1);
        let b = lit("b", 1);
        let mut node = node_and(&a, &b).unwrap();
        node.name = Some("n".to_owned());
        node.short_name = Some("short".to_owned());

        assert!(node_invert(&mut node).unwrap());

        assert_eq!(node.name.as_deref(), Some("n"));
        assert_eq!(node.short_name.as_deref(), Some("short"));
        assert!(node_contains(&node, &node_xor(&a, &b).unwrap()).unwrap());
        assert_eq!(node.fanins, vec!["a", "b"]);
        assert!(node.is_scc_minimal);
    }

    #[test]
    fn primary_io_nodes_are_not_inverted() {
        let mut input = Node::primary_input("a");
        let mut output = Node::primary_output("y", "a");

        assert!(!node_invert(&mut input).unwrap());
        assert!(!node_invert(&mut output).unwrap());
        assert_eq!(input.node_type, NodeType::PrimaryInput);
        assert_eq!(output.node_type, NodeType::PrimaryOutput);
    }

    #[test]
    fn network_inversion_flips_internal_fanout_literal() {
        let a = lit("a", 1);
        let b = lit("b", 1);
        let n = node_and(&a, &b).unwrap();
        let f = node_or(&lit("n", 1), &lit("a", 1)).unwrap();
        let mut network = InvertNetwork::new();
        network.add_node("n", n).unwrap();
        network.add_node("f", f).unwrap();

        assert!(network.invert_node("n").unwrap());

        let f = network.node("f").unwrap();
        assert_eq!(f.fanins, vec!["n", "a"]);
        assert_eq!(node_function(f).unwrap(), NodeFunction::Or);
        assert_eq!(f.function().unwrap().cubes()[0].inputs()[0], Some(false));
        assert!(!f.is_dup_free);
        assert!(!f.is_scc_minimal);
    }

    #[test]
    fn network_inversion_inserts_one_inverter_for_primary_outputs() {
        let mut network = InvertNetwork::new();
        network.add_node("n", lit("a", 1)).unwrap();
        network
            .add_node("out0", Node::primary_output("out0", "n"))
            .unwrap();
        network
            .add_node("out1", Node::primary_output("out1", "n"))
            .unwrap();

        assert!(network.invert_node("n").unwrap());

        let inverter = network.node("n_inv").unwrap();
        assert_eq!(node_function(inverter).unwrap(), NodeFunction::Inverter);
        assert_eq!(inverter.fanins, vec!["n"]);
        assert_eq!(network.node("out0").unwrap().fanins, vec!["n_inv"]);
        assert_eq!(network.node("out1").unwrap().fanins, vec!["n_inv"]);
    }

    #[test]
    fn no_legacy_c_abi_or_beads_metadata_tokens_are_present() {
        let source = include_str!("invert.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-")));
    }
}
