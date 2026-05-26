//! Native Rust model and dependency scaffold for
//! `LogicSynthesis/sis/seqbdd/prl_dep.c`.
//!
//! The C routine removes structural dependencies between one external PI and a
//! set of external POs by walking the PI transitive fanout backward. It
//! duplicates split internal nodes, redirects root-side fanouts through those
//! duplicates, and replaces root-side PI fanouts with an inserted constant.
//! Direct SIS `network_t`, `node_t`, `array_t`, and `st_table` integration is
//! still blocked by the ports listed in `required_port_dependencies`.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub note: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        note: "array_t node-vector allocation, fetch, insertion, and cleanup",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.297",
        source_file: "LogicSynthesis/sis/network/dfs.c",
        note: "network_dfs topological ordering used to extract the PI TFO",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.299",
        source_file: "LogicSynthesis/sis/network/net_seq.c",
        note: "network_is_real_pi and network_is_real_po validation",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "network_add_node and network_sweep mutation semantics",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        note: "fanin/fanout traversal and node_patch_fanin",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "node duplication, constants, names, fanout counts, and node_scc",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        note: "pointer-keyed root set used while walking dependencies",
    },
];

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstantValue {
    Zero,
    One,
}

impl ConstantValue {
    pub fn from_insert_a_one(insert_a_one: bool) -> Self {
        if insert_a_one { Self::One } else { Self::Zero }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepNode {
    pub id: NodeId,
    pub name: Option<String>,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub is_real_pi: bool,
    pub is_real_po: bool,
    pub constant: Option<ConstantValue>,
}

impl DepNode {
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            name: Some(name.into()),
            kind,
            fanins: Vec::new(),
            is_real_pi: kind == NodeKind::PrimaryInput,
            is_real_po: kind == NodeKind::PrimaryOutput,
            constant: None,
        }
    }

    pub fn with_fanins(mut self, fanins: impl Into<Vec<NodeId>>) -> Self {
        self.fanins = fanins.into();
        self
    }

    pub fn with_real_pi(mut self, is_real_pi: bool) -> Self {
        self.is_real_pi = is_real_pi;
        self
    }

    pub fn with_real_po(mut self, is_real_po: bool) -> Self {
        self.is_real_po = is_real_po;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyNetwork {
    nodes: Vec<Option<DepNode>>,
}

impl DependencyNetwork {
    pub fn new(nodes: impl IntoIterator<Item = DepNode>) -> Self {
        let mut network = Self { nodes: Vec::new() };
        for mut node in nodes {
            let id = NodeId(network.nodes.len());
            node.id = id;
            network.nodes.push(Some(node));
        }
        network
    }

    pub fn node(&self, id: NodeId) -> Option<&DepNode> {
        self.nodes.get(id.0).and_then(Option::as_ref)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut DepNode> {
        self.nodes.get_mut(id.0).and_then(Option::as_mut)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.iter().filter(|node| node.is_some()).count()
    }

    pub fn add_node(&mut self, mut node: DepNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.id = id;
        self.nodes.push(Some(node));
        id
    }

    pub fn add_internal(&mut self, name: impl Into<String>, fanins: Vec<NodeId>) -> NodeId {
        self.add_node(DepNode::new(NodeId(0), name, NodeKind::Internal).with_fanins(fanins))
    }

    pub fn add_primary_output(&mut self, name: impl Into<String>, fanin: NodeId) -> NodeId {
        self.add_node(
            DepNode::new(NodeId(0), name, NodeKind::PrimaryOutput).with_fanins(vec![fanin]),
        )
    }

    pub fn add_constant(&mut self, value: ConstantValue) -> NodeId {
        let mut node = DepNode::new(NodeId(0), "", NodeKind::Internal);
        node.name = None;
        node.constant = Some(value);
        self.add_node(node)
    }

    pub fn duplicate_internal_anonymous(&mut self, id: NodeId) -> Result<NodeId, PrlDepError> {
        let source = self.node(id).ok_or(PrlDepError::UnknownNode(id))?;
        if source.kind != NodeKind::Internal {
            return Err(PrlDepError::CannotDuplicateNonInternal(id));
        }
        let mut copy = source.clone();
        copy.name = None;
        Ok(self.add_node(copy))
    }

    pub fn fanouts(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter_map(Option::as_ref)
            .filter(|node| node.fanins.contains(&id))
            .map(|node| node.id)
            .collect()
    }

    pub fn topological_order(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter_map(Option::as_ref)
            .map(|node| node.id)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemoveDependencyOptions {
    pub verbosity: bool,
    pub insert_a_one: bool,
}

impl Default for RemoveDependencyOptions {
    fn default() -> Self {
        Self {
            verbosity: false,
            insert_a_one: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaninPatch {
    pub node: NodeId,
    pub old: NodeId,
    pub new: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveDependencyReport {
    pub pi_tfo: Vec<NodeId>,
    pub created_nodes: Vec<NodeId>,
    pub patches: Vec<FaninPatch>,
    pub new_roots: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrlDepError {
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    EmptyNodeVector,
    UnknownNode(NodeId),
    FirstNodeIsNotRealPi(NodeId),
    NodeIsNotRealPo(NodeId),
    CannotDuplicateNonInternal(NodeId),
    PatchMissingFanin {
        node: NodeId,
        old: NodeId,
        new: NodeId,
    },
}

impl fmt::Display for PrlDepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires native Rust ports for {} SIS dependencies",
                dependencies.len()
            ),
            Self::EmptyNodeVector => write!(f, "Prl_RemoveDependencies requires at least one node"),
            Self::UnknownNode(node) => write!(f, "unknown dependency node {:?}", node),
            Self::FirstNodeIsNotRealPi(node) => {
                write!(f, "first node {:?} is not an external PI", node)
            }
            Self::NodeIsNotRealPo(node) => write!(f, "node {:?} is not an external PO", node),
            Self::CannotDuplicateNonInternal(node) => {
                write!(f, "cannot duplicate non-internal node {:?}", node)
            }
            Self::PatchMissingFanin { node, old, new } => write!(
                f,
                "cannot patch node {:?}: fanin {:?} was not found for replacement by {:?}",
                node, old, new
            ),
        }
    }
}

impl Error for PrlDepError {}

pub fn remove_dependencies_from_sis_network<Network, Options>(
    _network: &mut Network,
    _nodevec: &[NodeId],
    _options: &Options,
) -> Result<RemoveDependencyReport, PrlDepError> {
    Err(PrlDepError::MissingNativePorts {
        operation: "Prl_RemoveDependencies SIS network_t entry",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn remove_dependencies(
    network: &mut DependencyNetwork,
    nodevec: &[NodeId],
    options: RemoveDependencyOptions,
) -> Result<RemoveDependencyReport, PrlDepError> {
    check_input(network, nodevec)?;
    if nodevec.len() == 1 {
        return Ok(RemoveDependencyReport {
            pi_tfo: Vec::new(),
            created_nodes: Vec::new(),
            patches: Vec::new(),
            new_roots: Vec::new(),
        });
    }

    let pi_tfo = extract_pi_tfo(network, nodevec[0])?;
    let mut roots = nodevec[1..].iter().copied().collect::<BTreeSet<_>>();
    let mut report = RemoveDependencyReport {
        pi_tfo: pi_tfo.clone(),
        created_nodes: Vec::new(),
        patches: Vec::new(),
        new_roots: Vec::new(),
    };

    blow_away_dependencies(network, &pi_tfo, &mut roots, options, &mut report)?;
    Ok(report)
}

pub fn check_input(network: &DependencyNetwork, nodevec: &[NodeId]) -> Result<(), PrlDepError> {
    let Some(pi) = nodevec.first().copied() else {
        return Err(PrlDepError::EmptyNodeVector);
    };
    let pi_node = network.node(pi).ok_or(PrlDepError::UnknownNode(pi))?;
    if pi_node.kind != NodeKind::PrimaryInput || !pi_node.is_real_pi {
        return Err(PrlDepError::FirstNodeIsNotRealPi(pi));
    }

    for po in &nodevec[1..] {
        let node = network.node(*po).ok_or(PrlDepError::UnknownNode(*po))?;
        if node.kind != NodeKind::PrimaryOutput || !node.is_real_po {
            return Err(PrlDepError::NodeIsNotRealPo(*po));
        }
    }

    Ok(())
}

pub fn extract_pi_tfo(network: &DependencyNetwork, pi: NodeId) -> Result<Vec<NodeId>, PrlDepError> {
    if network.node(pi).is_none() {
        return Err(PrlDepError::UnknownNode(pi));
    }

    let mut roots = BTreeSet::from([pi]);
    let mut pi_tfo = vec![pi];
    for node_id in network.topological_order() {
        if node_id == pi {
            continue;
        }
        let node = network
            .node(node_id)
            .ok_or(PrlDepError::UnknownNode(node_id))?;
        if node.fanins.iter().any(|fanin| roots.contains(fanin)) {
            roots.insert(node_id);
            pi_tfo.push(node_id);
        }
    }
    Ok(pi_tfo)
}

fn blow_away_dependencies(
    network: &mut DependencyNetwork,
    nodes: &[NodeId],
    roots: &mut BTreeSet<NodeId>,
    options: RemoveDependencyOptions,
    report: &mut RemoveDependencyReport,
) -> Result<(), PrlDepError> {
    for node_id in nodes.iter().rev().copied() {
        let kind = network
            .node(node_id)
            .ok_or(PrlDepError::UnknownNode(node_id))?
            .kind;
        if kind == NodeKind::PrimaryOutput {
            continue;
        }

        let root_fanouts = get_root_fanouts(network, node_id, roots);
        match kind {
            NodeKind::Internal => {
                if let Some(root) = process_internal_node(network, node_id, &root_fanouts, report)?
                {
                    roots.insert(root);
                    report.new_roots.push(root);
                }
            }
            NodeKind::PrimaryInput => {
                process_primary_input(network, node_id, &root_fanouts, options, report)?;
            }
            NodeKind::PrimaryOutput => unreachable!("primary outputs are skipped above"),
        }
    }
    Ok(())
}

pub fn get_root_fanouts(
    network: &DependencyNetwork,
    node: NodeId,
    roots: &BTreeSet<NodeId>,
) -> Vec<NodeId> {
    network
        .fanouts(node)
        .into_iter()
        .filter(|fanout| roots.contains(fanout))
        .collect()
}

fn process_internal_node(
    network: &mut DependencyNetwork,
    node: NodeId,
    root_fanouts: &[NodeId],
    report: &mut RemoveDependencyReport,
) -> Result<Option<NodeId>, PrlDepError> {
    if root_fanouts.is_empty() {
        return Ok(None);
    }
    if root_fanouts.len() == network.fanouts(node).len() {
        return Ok(Some(node));
    }

    let copy = network.duplicate_internal_anonymous(node)?;
    report.created_nodes.push(copy);
    patch_fanin(network, root_fanouts, node, copy, report)?;
    Ok(Some(copy))
}

fn process_primary_input(
    network: &mut DependencyNetwork,
    node: NodeId,
    root_fanouts: &[NodeId],
    options: RemoveDependencyOptions,
    report: &mut RemoveDependencyReport,
) -> Result<(), PrlDepError> {
    if root_fanouts.is_empty() {
        return Ok(());
    }

    let value = ConstantValue::from_insert_a_one(options.insert_a_one);
    let copy = network.add_constant(value);
    report.created_nodes.push(copy);
    patch_fanin(network, root_fanouts, node, copy, report)
}

fn patch_fanin(
    network: &mut DependencyNetwork,
    nodevec: &[NodeId],
    old: NodeId,
    new: NodeId,
    report: &mut RemoveDependencyReport,
) -> Result<(), PrlDepError> {
    for node_id in nodevec {
        let node = network
            .node_mut(*node_id)
            .ok_or(PrlDepError::UnknownNode(*node_id))?;
        let mut patched = false;
        for fanin in &mut node.fanins {
            if *fanin == old {
                *fanin = new;
                patched = true;
            }
        }
        if !patched {
            return Err(PrlDepError::PatchMissingFanin {
                node: *node_id,
                old,
                new,
            });
        }
        report.patches.push(FaninPatch {
            node: *node_id,
            old,
            new,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: usize, name: &str, kind: NodeKind) -> DepNode {
        DepNode::new(NodeId(id), name, kind)
    }

    fn fanins(network: &DependencyNetwork, node: NodeId) -> Vec<NodeId> {
        network.node(node).unwrap().fanins.clone()
    }

    #[test]
    fn validates_first_node_as_real_pi_and_rest_as_real_pos() {
        let network = DependencyNetwork::new([
            node(0, "pi", NodeKind::PrimaryInput).with_real_pi(false),
            node(1, "po", NodeKind::PrimaryOutput),
            node(2, "int", NodeKind::Internal),
        ]);

        assert_eq!(
            check_input(&network, &[NodeId(0), NodeId(1)]),
            Err(PrlDepError::FirstNodeIsNotRealPi(NodeId(0)))
        );
        assert_eq!(
            check_input(&network, &[NodeId(0)]),
            Err(PrlDepError::FirstNodeIsNotRealPi(NodeId(0)))
        );

        let network = DependencyNetwork::new([
            node(0, "pi", NodeKind::PrimaryInput),
            node(1, "po", NodeKind::PrimaryOutput).with_real_po(false),
            node(2, "int", NodeKind::Internal),
        ]);
        assert_eq!(
            check_input(&network, &[NodeId(0), NodeId(1)]),
            Err(PrlDepError::NodeIsNotRealPo(NodeId(1)))
        );
        assert_eq!(
            check_input(&network, &[]),
            Err(PrlDepError::EmptyNodeVector)
        );
    }

    #[test]
    fn extracts_pi_tfo_in_topological_order() {
        let network = DependencyNetwork::new([
            node(0, "pi", NodeKind::PrimaryInput),
            node(1, "unrelated", NodeKind::Internal),
            node(2, "a", NodeKind::Internal).with_fanins([NodeId(0)]),
            node(3, "b", NodeKind::Internal).with_fanins([NodeId(2)]),
            node(4, "po", NodeKind::PrimaryOutput).with_fanins([NodeId(3)]),
        ]);

        assert_eq!(
            extract_pi_tfo(&network, NodeId(0)).unwrap(),
            vec![NodeId(0), NodeId(2), NodeId(3), NodeId(4)]
        );
    }

    #[test]
    fn one_node_input_is_a_valid_noop() {
        let mut network = DependencyNetwork::new([node(0, "pi", NodeKind::PrimaryInput)]);

        let report = remove_dependencies(
            &mut network,
            &[NodeId(0)],
            RemoveDependencyOptions::default(),
        )
        .unwrap();

        assert_eq!(
            report,
            RemoveDependencyReport {
                pi_tfo: Vec::new(),
                created_nodes: Vec::new(),
                patches: Vec::new(),
                new_roots: Vec::new(),
            }
        );
    }

    #[test]
    fn splits_shared_internal_nodes_and_replaces_root_pi_fanout_with_constant() {
        let mut network = DependencyNetwork::new([
            node(0, "pi", NodeKind::PrimaryInput),
            node(1, "shared", NodeKind::Internal).with_fanins([NodeId(0)]),
            node(2, "root_po", NodeKind::PrimaryOutput).with_fanins([NodeId(1)]),
            node(3, "side", NodeKind::Internal).with_fanins([NodeId(1)]),
            node(4, "side_po", NodeKind::PrimaryOutput).with_fanins([NodeId(3)]),
        ]);

        let report = remove_dependencies(
            &mut network,
            &[NodeId(0), NodeId(2)],
            RemoveDependencyOptions {
                verbosity: false,
                insert_a_one: false,
            },
        )
        .unwrap();

        let duplicated_shared = report.created_nodes[0];
        let constant = report.created_nodes[1];
        assert_eq!(network.node(duplicated_shared).unwrap().name, None);
        assert_eq!(
            network.node(constant).unwrap().constant,
            Some(ConstantValue::Zero)
        );
        assert_eq!(fanins(&network, NodeId(2)), vec![duplicated_shared]);
        assert_eq!(fanins(&network, NodeId(3)), vec![NodeId(1)]);
        assert_eq!(fanins(&network, duplicated_shared), vec![constant]);
        assert_eq!(
            report.patches,
            vec![
                FaninPatch {
                    node: NodeId(2),
                    old: NodeId(1),
                    new: duplicated_shared,
                },
                FaninPatch {
                    node: duplicated_shared,
                    old: NodeId(0),
                    new: constant,
                },
            ]
        );
    }

    #[test]
    fn all_root_internal_fanouts_mark_original_node_as_new_root() {
        let mut network = DependencyNetwork::new([
            node(0, "pi", NodeKind::PrimaryInput),
            node(1, "mid", NodeKind::Internal).with_fanins([NodeId(0)]),
            node(2, "root_po", NodeKind::PrimaryOutput).with_fanins([NodeId(1)]),
        ]);

        let report = remove_dependencies(
            &mut network,
            &[NodeId(0), NodeId(2)],
            RemoveDependencyOptions::default(),
        )
        .unwrap();

        assert_eq!(report.new_roots, vec![NodeId(1)]);
        assert_eq!(report.created_nodes.len(), 1);
        let constant = report.created_nodes[0];
        assert_eq!(
            network.node(constant).unwrap().constant,
            Some(ConstantValue::One)
        );
        assert_eq!(fanins(&network, NodeId(1)), vec![constant]);
        assert_eq!(fanins(&network, NodeId(2)), vec![NodeId(1)]);
    }

    #[test]
    fn sis_entry_reports_dependency_beads_and_source_files() {
        let error = remove_dependencies_from_sis_network(&mut (), &[NodeId(0)], &()).unwrap_err();

        match error {
            PrlDepError::MissingNativePorts {
                operation,
                dependencies,
            } => {
                assert_eq!(operation, "Prl_RemoveDependencies SIS network_t entry");
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.297"
                        && dependency.source_file == "LogicSynthesis/sis/network/dfs.c"
                }));
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.313"
                        && dependency.source_file == "LogicSynthesis/sis/node/fan.c"
                }));
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.485"
                        && dependency.source_file == "LogicSynthesis/sis/st/st.c"
                }));
            }
            other => panic!("unexpected error: {other:?}"),
        }

        assert_eq!(required_port_dependencies(), REQUIRED_PORT_DEPENDENCIES);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("prl_dep.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
