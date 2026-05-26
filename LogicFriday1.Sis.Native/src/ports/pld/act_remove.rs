//! Owned-data port of SIS PLD ACT removal.
//!
//! The native API models the behavior of removing local or global ACT entries
//! from selected nodes. Destroying an entry consumes the owned value and returns
//! a report describing the root DAG that was reachable from the entry.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActLocality {
    Global,
    Local,
}

impl ActLocality {
    const fn slot_index(self) -> usize {
        match self {
            Self::Global => 0,
            Self::Local => 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActRemoveRequest {
    pub locality: ActLocality,
    pub node_names: Vec<String>,
}

impl ActRemoveRequest {
    pub fn global(node_names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            locality: ActLocality::Global,
            node_names: node_names.into_iter().map(Into::into).collect(),
        }
    }

    pub fn local(node_names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            locality: ActLocality::Local,
            node_names: node_names.into_iter().map(Into::into).collect(),
        }
    }

    pub fn parse_args(args: &[impl AsRef<str>]) -> ActRemoveResult<Self> {
        let mut locality = ActLocality::Global;
        let mut node_names = Vec::new();
        let mut parsing_options = true;

        for arg in args {
            let arg = arg.as_ref();
            if parsing_options && arg == "--" {
                parsing_options = false;
                continue;
            }

            if parsing_options && arg.starts_with('-') && arg.len() > 1 {
                for option in arg[1..].chars() {
                    match option {
                        'g' => locality = ActLocality::Global,
                        'l' => locality = ActLocality::Local,
                        _ => return Err(ActRemoveError::Usage),
                    }
                }
                continue;
            }

            node_names.push(arg.to_owned());
        }

        if node_names.is_empty() {
            return Err(ActRemoveError::Usage);
        }

        Ok(Self {
            locality,
            node_names,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActNetwork {
    nodes: Vec<ActNode>,
}

impl ActNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: ActNode) -> ActNodeId {
        let id = ActNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: ActNodeId) -> ActRemoveResult<&ActNode> {
        self.nodes
            .get(id.0)
            .ok_or(ActRemoveError::UnknownNodeId(id))
    }

    pub fn node_mut(&mut self, id: ActNodeId) -> ActRemoveResult<&mut ActNode> {
        self.nodes
            .get_mut(id.0)
            .ok_or(ActRemoveError::UnknownNodeId(id))
    }

    pub fn find_node(&self, name: &str) -> Option<ActNodeId> {
        self.nodes
            .iter()
            .position(|node| node.name == name)
            .map(ActNodeId)
    }

    pub fn nodes(&self) -> &[ActNode] {
        &self.nodes
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActNodeId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActNode {
    pub name: String,
    act_set: ActSet,
}

impl ActNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            act_set: ActSet::default(),
        }
    }

    pub fn act_set(&self) -> &ActSet {
        &self.act_set
    }

    pub fn act_set_mut(&mut self) -> &mut ActSet {
        &mut self.act_set
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActSet {
    entries: [Option<ActEntry>; 2],
}

impl ActSet {
    pub fn entry(&self, locality: ActLocality) -> Option<&ActEntry> {
        self.entries[locality.slot_index()].as_ref()
    }

    pub fn set_entry(&mut self, locality: ActLocality, entry: ActEntry) {
        self.entries[locality.slot_index()] = Some(entry);
    }

    pub fn take_entry(&mut self, locality: ActLocality) -> Option<ActEntry> {
        self.entries[locality.slot_index()].take()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActEntry {
    pub act: Act,
    pub order_style: i32,
}

impl ActEntry {
    pub fn new(act: Act, order_style: i32) -> Self {
        Self { act, order_style }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Act {
    pub root: Option<Rc<ActVertex>>,
    pub node_list: Vec<ActNodeId>,
    pub node_name: String,
}

impl Act {
    pub fn new(
        root: Option<Rc<ActVertex>>,
        node_list: impl IntoIterator<Item = ActNodeId>,
        node_name: impl Into<String>,
    ) -> Self {
        Self {
            root,
            node_list: node_list.into_iter().collect(),
            node_name: node_name.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActVertex {
    pub index: usize,
    pub value: i32,
    pub index_size: usize,
    pub low: Option<Rc<ActVertex>>,
    pub high: Option<Rc<ActVertex>>,
}

impl ActVertex {
    pub fn terminal(value: i32, index_size: usize) -> Rc<Self> {
        Rc::new(Self {
            index: index_size,
            value,
            index_size,
            low: None,
            high: None,
        })
    }

    pub fn branch(
        index: usize,
        index_size: usize,
        low: Rc<ActVertex>,
        high: Rc<ActVertex>,
    ) -> Rc<Self> {
        Rc::new(Self {
            index,
            value: 4,
            index_size,
            low: Some(low),
            high: Some(high),
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActRemoveReport {
    pub locality: Option<ActLocality>,
    pub removed_entries: usize,
    pub destroyed_dag_vertices: usize,
    pub released_local_node_lists: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActDagDestroyReport {
    pub destroyed_vertices: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActRemoveError {
    Usage,
    UnknownNode { name: String },
    UnknownNodeId(ActNodeId),
    InvalidDagChild { index: usize },
    InvalidDagIndex { index: usize, index_size: usize },
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for ActRemoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(
                f,
                "usage: act_remove [-l] [-g] nodelist\n      -l          local\n      -g          global"
            ),
            Self::UnknownNode { name } => write!(f, "unknown ACT node {name}"),
            Self::UnknownNodeId(node) => write!(f, "unknown ACT node id {:?}", node),
            Self::InvalidDagChild { index } => {
                write!(f, "ACT vertex at index {index} is missing a child")
            }
            Self::InvalidDagIndex { index, index_size } => write!(
                f,
                "ACT vertex index {index} is outside index list 0..={index_size}"
            ),
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for ActRemoveError {}

pub type ActRemoveResult<T> = Result<T, ActRemoveError>;

pub fn act_remove(
    network: &mut ActNetwork,
    request: &ActRemoveRequest,
) -> ActRemoveResult<ActRemoveReport> {
    let node_ids = request
        .node_names
        .iter()
        .map(|name| {
            network
                .find_node(name)
                .ok_or_else(|| ActRemoveError::UnknownNode { name: name.clone() })
        })
        .collect::<ActRemoveResult<Vec<_>>>()?;

    let mut report = ActRemoveReport {
        locality: Some(request.locality),
        ..ActRemoveReport::default()
    };

    for node_id in node_ids {
        let node = network.node_mut(node_id)?;
        let entry = node.act_set_mut().take_entry(request.locality);
        let entry_report = act_destroy(entry, request.locality)?;
        report.removed_entries += entry_report.removed_entries;
        report.destroyed_dag_vertices += entry_report.destroyed_dag_vertices;
        report.released_local_node_lists += entry_report.released_local_node_lists;
    }

    Ok(report)
}

pub fn act_remove_args(
    network: &mut ActNetwork,
    args: &[impl AsRef<str>],
) -> ActRemoveResult<ActRemoveReport> {
    let request = ActRemoveRequest::parse_args(args)?;
    act_remove(network, &request)
}

pub fn act_destroy(
    entry: Option<ActEntry>,
    locality: ActLocality,
) -> ActRemoveResult<ActRemoveReport> {
    let Some(entry) = entry else {
        return Ok(ActRemoveReport::default());
    };

    let destroyed_dag_vertices = match &entry.act.root {
        Some(root) => dag_destroy(root)?.destroyed_vertices,
        None => 0,
    };
    let released_local_node_lists = usize::from(locality == ActLocality::Local);

    Ok(ActRemoveReport {
        locality: Some(locality),
        removed_entries: 1,
        destroyed_dag_vertices,
        released_local_node_lists,
    })
}

pub fn dag_destroy(root: &Rc<ActVertex>) -> ActRemoveResult<ActDagDestroyReport> {
    let mut visited = HashSet::new();
    let mut counts_by_index = vec![0usize; root.index_size + 1];
    collect_dag_vertices(root, root.index_size, &mut visited, &mut counts_by_index)?;

    Ok(ActDagDestroyReport {
        destroyed_vertices: counts_by_index.into_iter().sum(),
    })
}

pub fn act_remove_sis_network_blocked<Network>(
    _network: &mut Network,
) -> ActRemoveResult<ActRemoveReport> {
    Err(missing_native_ports("act_remove SIS network integration"))
}

fn collect_dag_vertices(
    vertex: &Rc<ActVertex>,
    root_index_size: usize,
    visited: &mut HashSet<*const ActVertex>,
    counts_by_index: &mut [usize],
) -> ActRemoveResult<()> {
    if vertex.index > root_index_size {
        return Err(ActRemoveError::InvalidDagIndex {
            index: vertex.index,
            index_size: root_index_size,
        });
    }

    let pointer = Rc::as_ptr(vertex);
    if !visited.insert(pointer) {
        return Ok(());
    }

    counts_by_index[vertex.index] += 1;

    if vertex.index != vertex.index_size {
        let low = vertex.low.as_ref().ok_or(ActRemoveError::InvalidDagChild {
            index: vertex.index,
        })?;
        let high = vertex
            .high
            .as_ref()
            .ok_or(ActRemoveError::InvalidDagChild {
                index: vertex.index,
            })?;
        collect_dag_vertices(low, root_index_size, visited, counts_by_index)?;
        collect_dag_vertices(high, root_index_size, visited, counts_by_index)?;
    }

    Ok(())
}

fn missing_native_ports(operation: &'static str) -> ActRemoveError {
    ActRemoveError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dag() -> Rc<ActVertex> {
        let zero = ActVertex::terminal(0, 2);
        let one = ActVertex::terminal(1, 2);
        let shared = ActVertex::branch(1, 2, zero.clone(), one.clone());

        ActVertex::branch(0, 2, shared.clone(), shared)
    }

    fn entry(name: &str) -> ActEntry {
        ActEntry::new(
            Act::new(Some(sample_dag()), [ActNodeId(10), ActNodeId(11)], name),
            0,
        )
    }

    fn network_with_entries() -> ActNetwork {
        let mut network = ActNetwork::new();
        let mut node = ActNode::new("n1");
        node.act_set_mut()
            .set_entry(ActLocality::Global, entry("global"));
        node.act_set_mut()
            .set_entry(ActLocality::Local, entry("local"));
        network.add_node(node);
        network.add_node(ActNode::new("n2"));
        network
    }

    #[test]
    fn parse_args_defaults_to_global_and_last_locality_option_wins() {
        assert_eq!(
            ActRemoveRequest::parse_args(&["n1", "n2"]).unwrap(),
            ActRemoveRequest::global(["n1", "n2"])
        );
        assert_eq!(
            ActRemoveRequest::parse_args(&["-g", "-l", "n1"]).unwrap(),
            ActRemoveRequest::local(["n1"])
        );
        assert_eq!(
            ActRemoveRequest::parse_args(&["-lg", "n1"]).unwrap(),
            ActRemoveRequest::global(["n1"])
        );
    }

    #[test]
    fn parse_args_rejects_unknown_options_and_empty_node_list() {
        assert_eq!(
            ActRemoveRequest::parse_args(&["-x"]),
            Err(ActRemoveError::Usage)
        );
        assert_eq!(
            ActRemoveRequest::parse_args(&["-l"]),
            Err(ActRemoveError::Usage)
        );
    }

    #[test]
    fn act_remove_clears_only_requested_locality() {
        let mut network = network_with_entries();

        let report = act_remove(&mut network, &ActRemoveRequest::local(["n1"])).unwrap();

        assert_eq!(
            report,
            ActRemoveReport {
                locality: Some(ActLocality::Local),
                removed_entries: 1,
                destroyed_dag_vertices: 4,
                released_local_node_lists: 1,
            }
        );
        assert!(
            network.nodes()[0]
                .act_set()
                .entry(ActLocality::Local)
                .is_none()
        );
        assert!(
            network.nodes()[0]
                .act_set()
                .entry(ActLocality::Global)
                .is_some()
        );
    }

    #[test]
    fn global_act_remove_does_not_release_local_node_list() {
        let mut network = network_with_entries();

        let report = act_remove(&mut network, &ActRemoveRequest::global(["n1"])).unwrap();

        assert_eq!(report.removed_entries, 1);
        assert_eq!(report.destroyed_dag_vertices, 4);
        assert_eq!(report.released_local_node_lists, 0);
        assert!(
            network.nodes()[0]
                .act_set()
                .entry(ActLocality::Global)
                .is_none()
        );
        assert!(
            network.nodes()[0]
                .act_set()
                .entry(ActLocality::Local)
                .is_some()
        );
    }

    #[test]
    fn act_remove_is_noop_for_nodes_without_requested_entry() {
        let mut network = network_with_entries();

        let report = act_remove(&mut network, &ActRemoveRequest::global(["n2"])).unwrap();

        assert_eq!(
            report,
            ActRemoveReport {
                locality: Some(ActLocality::Global),
                removed_entries: 0,
                destroyed_dag_vertices: 0,
                released_local_node_lists: 0,
            }
        );
    }

    #[test]
    fn dag_destroy_counts_shared_vertices_once() {
        let report = dag_destroy(&sample_dag()).unwrap();

        assert_eq!(report.destroyed_vertices, 4);
    }

    #[test]
    fn dag_destroy_rejects_malformed_branch_without_children() {
        let malformed = Rc::new(ActVertex {
            index: 0,
            value: 4,
            index_size: 1,
            low: None,
            high: None,
        });

        assert_eq!(
            dag_destroy(&malformed),
            Err(ActRemoveError::InvalidDagChild { index: 0 })
        );
    }

    #[test]
    fn act_remove_reports_unknown_node_before_mutating_network() {
        let mut network = network_with_entries();

        let error = act_remove(&mut network, &ActRemoveRequest::global(["n1", "missing"]));

        assert_eq!(
            error,
            Err(ActRemoveError::UnknownNode {
                name: "missing".to_owned()
            })
        );
        assert!(
            network.nodes()[0]
                .act_set()
                .entry(ActLocality::Global)
                .is_some()
        );
    }

    #[test]
    fn no_legacy_c_abi_or_beads_metadata_tokens_are_present_in_this_port() {
        let source = include_str!("act_remove.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
