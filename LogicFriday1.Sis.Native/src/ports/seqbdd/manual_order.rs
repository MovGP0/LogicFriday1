//! Native Rust model for `LogicSynthesis/sis/seqbdd/manual_order.c`.
//!
//! The C file rewrites an existing node-order table by reading primary-input
//! names from a separate order network. This module ports that deterministic
//! name-normalization and ranking behavior onto owned Rust data. Direct SIS
//! `network_t`, `node_t`, `st_table`, and `verif_options_t` integration remains
//! blocked until those native ports are available.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub note: &'static str,
}

pub const REQUIRED_MANUAL_ORDER_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "order-network primary-input traversal and network ownership",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "node_t identity and node name storage used as order-table keys",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.426",
        source_file: "LogicSynthesis/sis/seqbdd/com_verify.c",
        note: "manual-order option parsing and order-network loading",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        note: "st_table name lookup and node-to-rank mutation semantics",
    },
];

pub fn required_manual_order_dependencies() -> &'static [PortDependency] {
    REQUIRED_MANUAL_ORDER_DEPENDENCIES
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualOrderNode {
    pub id: NodeId,
    pub name: String,
}

impl ManualOrderNode {
    pub fn new(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualOrderNetwork {
    primary_inputs: Vec<String>,
}

impl ManualOrderNetwork {
    pub fn from_primary_inputs(
        primary_inputs: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            primary_inputs: primary_inputs.into_iter().map(Into::into).collect(),
        }
    }

    pub fn primary_inputs(&self) -> &[String] {
        &self.primary_inputs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualOrderOptions {
    pub use_manual_order: bool,
    pub order_network: Option<ManualOrderNetwork>,
}

impl ManualOrderOptions {
    pub fn disabled() -> Self {
        Self {
            use_manual_order: false,
            order_network: None,
        }
    }

    pub fn enabled(order_network: ManualOrderNetwork) -> Self {
        Self {
            use_manual_order: true,
            order_network: Some(order_network),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingOrdering {
    pub node: NodeId,
    pub original_name: String,
    pub lookup_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualOrderReport {
    pub assigned_count: usize,
    pub missing_orderings: Vec<MissingOrdering>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManualOrderError {
    ManualOrderDisabled,
    MissingOrderNetwork,
    UnknownOrderNode(NodeId),
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for ManualOrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManualOrderDisabled => write!(f, "manual order was requested while disabled"),
            Self::MissingOrderNetwork => write!(f, "manual order requires an order network"),
            Self::UnknownOrderNode(node) => {
                write!(f, "manual order table references unknown node {:?}", node)
            }
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires native Rust ports for {} SIS dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for ManualOrderError {}

pub fn extract_order_info(network: &ManualOrderNetwork) -> HashMap<String, usize> {
    network
        .primary_inputs()
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, name)| (name, index))
        .collect()
}

pub fn get_manual_order(
    order: &mut BTreeMap<NodeId, usize>,
    nodes: &[ManualOrderNode],
    options: &ManualOrderOptions,
) -> Result<ManualOrderReport, ManualOrderError> {
    if !options.use_manual_order {
        return Err(ManualOrderError::ManualOrderDisabled);
    }
    let order_network = options
        .order_network
        .as_ref()
        .ok_or(ManualOrderError::MissingOrderNetwork)?;

    let name_table = extract_order_info(order_network);
    order_table(order, nodes, &name_table)
}

pub fn order_table(
    order: &mut BTreeMap<NodeId, usize>,
    nodes: &[ManualOrderNode],
    name_table: &HashMap<String, usize>,
) -> Result<ManualOrderReport, ManualOrderError> {
    let node_names = nodes
        .iter()
        .map(|node| (node.id, node.name.as_str()))
        .collect::<HashMap<_, _>>();
    let original_count = order.len();
    let ordered_nodes = order.keys().copied().collect::<Vec<_>>();
    let mut missing_orderings = Vec::new();

    for node in ordered_nodes {
        let name = node_names
            .get(&node)
            .copied()
            .ok_or(ManualOrderError::UnknownOrderNode(node))?;
        let lookup_name = normalize_order_name(name);
        let rank = match name_table.get(&lookup_name).copied() {
            Some(rank) => rank,
            None => {
                missing_orderings.push(MissingOrdering {
                    node,
                    original_name: name.to_owned(),
                    lookup_name,
                });
                0
            }
        };
        order.insert(node, rank);
    }

    debug_assert_eq!(original_count, order.len());
    Ok(ManualOrderReport {
        assigned_count: order.len(),
        missing_orderings,
    })
}

pub fn get_node_order(name: &str, name_table: &HashMap<String, usize>) -> Option<usize> {
    name_table.get(&normalize_order_name(name)).copied()
}

pub fn normalize_order_name(name: &str) -> String {
    replace_character(strip_trailing_y(name), ':', '_')
}

pub fn strip_trailing_y(name: &str) -> &str {
    let Some(index) = name.rfind(':') else {
        return name;
    };
    let suffix = &name[index + 1..];
    if suffix.starts_with('y') {
        &name[..index]
    } else {
        name
    }
}

pub fn replace_character(name: &str, old: char, new: char) -> String {
    name.chars()
        .map(|character| if character == old { new } else { character })
        .collect()
}

pub fn get_manual_order_from_sis() -> Result<(), ManualOrderError> {
    Err(ManualOrderError::MissingNativePorts {
        operation: "get_manual_order SIS network_t/st_table entry",
        dependencies: REQUIRED_MANUAL_ORDER_DEPENDENCIES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize, name: &str) -> ManualOrderNode {
        ManualOrderNode::new(NodeId(id), name)
    }

    #[test]
    fn extracts_primary_input_order_from_order_network() {
        let network = ManualOrderNetwork::from_primary_inputs(["a", "b", "c"]);

        assert_eq!(
            extract_order_info(&network),
            HashMap::from([
                ("a".to_owned(), 0),
                ("b".to_owned(), 1),
                ("c".to_owned(), 2),
            ])
        );
    }

    #[test]
    fn applies_manual_order_with_c_name_normalization() {
        let nodes = vec![node(0, "b:y0"), node(1, "state:bit:y17"), node(2, "a")];
        let mut order = BTreeMap::from([(NodeId(0), 99), (NodeId(1), 99), (NodeId(2), 99)]);
        let options = ManualOrderOptions::enabled(ManualOrderNetwork::from_primary_inputs([
            "a",
            "state_bit",
            "b",
        ]));

        let report = get_manual_order(&mut order, &nodes, &options).unwrap();

        assert_eq!(
            order,
            BTreeMap::from([(NodeId(0), 2), (NodeId(1), 1), (NodeId(2), 0)])
        );
        assert_eq!(
            report,
            ManualOrderReport {
                assigned_count: 3,
                missing_orderings: Vec::new(),
            }
        );
    }

    #[test]
    fn missing_lookup_preserves_c_fallback_rank_zero_and_reports_name() {
        let nodes = vec![node(0, "known"), node(1, "missing:y3")];
        let mut order = BTreeMap::from([(NodeId(0), 7), (NodeId(1), 7)]);
        let options =
            ManualOrderOptions::enabled(ManualOrderNetwork::from_primary_inputs(["known"]));

        let report = get_manual_order(&mut order, &nodes, &options).unwrap();

        assert_eq!(order, BTreeMap::from([(NodeId(0), 0), (NodeId(1), 0)]));
        assert_eq!(
            report.missing_orderings,
            vec![MissingOrdering {
                node: NodeId(1),
                original_name: "missing:y3".to_owned(),
                lookup_name: "missing".to_owned(),
            }]
        );
    }

    #[test]
    fn normalization_matches_extract_trailing_y_and_replace_character() {
        assert_eq!(strip_trailing_y("plain"), "plain");
        assert_eq!(strip_trailing_y("a:b:x0"), "a:b:x0");
        assert_eq!(strip_trailing_y("a:b:y"), "a:b");
        assert_eq!(strip_trailing_y("a:b:yabc"), "a:b");
        assert_eq!(normalize_order_name("a:b:x0"), "a_b_x0");
        assert_eq!(normalize_order_name("a:b:y12"), "a_b");
    }

    #[test]
    fn option_preconditions_replace_c_asserts_with_errors() {
        let nodes = vec![node(0, "a")];
        let mut order = BTreeMap::from([(NodeId(0), 0)]);

        assert_eq!(
            get_manual_order(&mut order, &nodes, &ManualOrderOptions::disabled()),
            Err(ManualOrderError::ManualOrderDisabled)
        );
        assert_eq!(
            get_manual_order(
                &mut order,
                &nodes,
                &ManualOrderOptions {
                    use_manual_order: true,
                    order_network: None,
                },
            ),
            Err(ManualOrderError::MissingOrderNetwork)
        );
    }

    #[test]
    fn order_table_rejects_unknown_node_key() {
        let nodes = vec![node(0, "a")];
        let mut order = BTreeMap::from([(NodeId(9), 0)]);
        let name_table = HashMap::from([("a".to_owned(), 0)]);

        assert_eq!(
            order_table(&mut order, &nodes, &name_table),
            Err(ManualOrderError::UnknownOrderNode(NodeId(9)))
        );
    }

    #[test]
    fn sis_entry_reports_dependency_beads_and_sources() {
        let error = get_manual_order_from_sis().unwrap_err();

        match error {
            ManualOrderError::MissingNativePorts {
                operation,
                dependencies,
            } => {
                assert_eq!(operation, "get_manual_order SIS network_t/st_table entry");
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.305"
                        && dependency.source_file == "LogicSynthesis/sis/network/network_util.c"
                }));
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.485"
                        && dependency.source_file == "LogicSynthesis/sis/st/st.c"
                }));
            }
            other => panic!("unexpected error: {other:?}"),
        }

        assert_eq!(
            required_manual_order_dependencies(),
            REQUIRED_MANUAL_ORDER_DEPENDENCIES
        );
    }
}
