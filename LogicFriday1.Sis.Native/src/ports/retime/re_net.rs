//! Native Rust model for `LogicSynthesis/sis/retime/re_net.c`.
//!
//! The C file converts between SIS `network_t` instances and retiming graphs,
//! walking latch chains as weighted retime edges and later rebuilding latch
//! chains from those edge weights. This module ports that behavior to owned
//! Rust data structures. Direct mutation of SIS `network_t`, `node_t`,
//! `latch_t`, mapped-gate, delay, and mapper data remains an explicit
//! dependency error with bead IDs and source files; no legacy C ABI symbols are
//! exposed here.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const LATCH_DONT_CARE: i32 = 2;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LatchId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EdgeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Deleted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Ignore,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LatchSynchType {
    RisingEdge,
    FallingEdge,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_SIS_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.230",
        source_file: "LogicSynthesis/sis/latch/latch.c",
        reason: "latch identity, input/output endpoint lookup, type, control, gate, and initial/current values",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.257",
        source_file: "LogicSynthesis/sis/map/library.c",
        reason: "mapped D-latch selection and lib_gate latch-pin/type/name queries",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.258",
        source_file: "LogicSynthesis/sis/map/libutil.c",
        reason: "lib_set_gate and mapped pin formal/actual binding used by retime_lib_set_gate",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.262",
        source_file: "LogicSynthesis/sis/map/maputil.c",
        reason: "map_network and mapped-network decomposition for complex latch remnants",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "network allocation, node insertion/deletion, latch iteration, PO ordering, and control-node classification",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "fanin/fanout traversal, fanout-pin IDs, and fanin patching",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node allocation, duplication, names, literals, types, copies, and replacement",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.415",
        source_file: "LogicSynthesis/sis/retime/re_graph.c",
        reason: "legacy re_graph storage and edge/node accessors consumed by re_net.c",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.422",
        source_file: "LogicSynthesis/sis/retime/re_util.c",
        reason: "retime graph allocation, edge creation, ignore-edge tests, and graph traversal helpers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.423",
        source_file: "LogicSynthesis/sis/retime/retime_util.c",
        reason: "retimability checks and temporary-node cleanup expected around network-to-graph conversion",
    },
];

pub fn required_sis_dependencies() -> &'static [PortDependency] {
    REQUIRED_SIS_DEPENDENCIES
}

pub fn sis_network_to_graph_blocked() -> Result<RetimeGraph, RetimeNetError> {
    Err(RetimeNetError::MissingSisPorts {
        operation: "retime_network_to_graph over SIS network_t",
        dependencies: REQUIRED_SIS_DEPENDENCIES,
    })
}

pub fn sis_graph_to_network_blocked() -> Result<NetworkSketch, RetimeNetError> {
    Err(RetimeNetError::MissingSisPorts {
        operation: "retime_graph_to_network over SIS network_t",
        dependencies: REQUIRED_SIS_DEPENDENCIES,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetimeNetError {
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    MissingMappedDLatch,
    MissingNode(NodeId),
    MissingEdge(EdgeId),
    MissingLatch(LatchId),
    MissingLatchOutput(NodeId),
    MissingFanin {
        node: NodeId,
        index: usize,
    },
    MissingGraphNodeForNetworkNode(NodeId),
    NegativeEdgeWeight {
        edge: EdgeId,
        weight: i32,
    },
    UntraversedLatchCycle {
        latches: Vec<LatchId>,
    },
}

impl fmt::Display for RetimeNetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} native SIS prerequisite ports",
                dependencies.len()
            ),
            Self::MissingMappedDLatch => {
                write!(f, "mapped retiming requires a native D-latch library gate")
            }
            Self::MissingNode(node) => write!(f, "missing network node {}", node.0),
            Self::MissingEdge(edge) => write!(f, "missing retime edge {}", edge.0),
            Self::MissingLatch(latch) => write!(f, "missing latch {}", latch.0),
            Self::MissingLatchOutput(node) => {
                write!(f, "node {} is not the output side of a latch", node.0)
            }
            Self::MissingFanin { node, index } => {
                write!(f, "node {} has no fanin at index {index}", node.0)
            }
            Self::MissingGraphNodeForNetworkNode(node) => {
                write!(
                    f,
                    "network node {} was not materialized in the retime graph",
                    node.0
                )
            }
            Self::NegativeEdgeWeight { edge, weight } => {
                write!(f, "retime edge {} has negative weight {weight}", edge.0)
            }
            Self::UntraversedLatchCycle { latches } => write!(
                f,
                "retime graph construction left {} latch-only cycle entries untraversed",
                latches.len()
            ),
        }
    }
}

impl Error for RetimeNetError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutRef {
    pub node: NodeId,
    pub pin: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub fanouts: Vec<FanoutRef>,
    pub latch_end: Option<NodeId>,
    pub is_control: bool,
    pub is_mapped_d_latch: bool,
    pub is_temporary: bool,
}

impl NetworkNode {
    fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            latch_end: None,
            is_control: false,
            is_mapped_d_latch: false,
            is_temporary: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatchSketch {
    pub id: LatchId,
    pub input: NodeId,
    pub output: NodeId,
    pub synch_type: LatchSynchType,
    pub control: Option<NodeId>,
    pub initial_value: i32,
    pub current_value: i32,
    pub mapped_gate_present: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NetworkSketch {
    pub nodes: Vec<NetworkNode>,
    pub latches: Vec<LatchSketch>,
    pub mapped_d_latch_available: bool,
}

impl NetworkSketch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, name: impl Into<String>, kind: NodeKind) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(NetworkNode::new(id, name, kind));
        id
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> NodeId {
        self.add_node(name, NodeKind::PrimaryInput)
    }

    pub fn add_primary_output(&mut self, name: impl Into<String>, fanin: NodeId) -> NodeId {
        let po = self.add_node(name, NodeKind::PrimaryOutput);
        self.connect(fanin, po);
        po
    }

    pub fn add_internal(&mut self, name: impl Into<String>, fanins: &[NodeId]) -> NodeId {
        let node = self.add_node(name, NodeKind::Internal);
        for fanin in fanins {
            self.connect(*fanin, node);
        }
        node
    }

    pub fn connect(&mut self, from: NodeId, to: NodeId) {
        let pin = self.nodes[to.0].fanins.len();
        self.nodes[to.0].fanins.push(from);
        self.nodes[from.0].fanouts.push(FanoutRef { node: to, pin });
    }

    pub fn replace_fanin(
        &mut self,
        node: NodeId,
        index: usize,
        replacement: NodeId,
    ) -> Result<NodeId, RetimeNetError> {
        self.require_node(node)?;
        self.require_node(replacement)?;
        let old = *self.nodes[node.0]
            .fanins
            .get(index)
            .ok_or(RetimeNetError::MissingFanin { node, index })?;

        self.nodes[old.0]
            .fanouts
            .retain(|fanout| !(fanout.node == node && fanout.pin == index));
        self.nodes[node.0].fanins[index] = replacement;
        self.nodes[replacement.0]
            .fanouts
            .push(FanoutRef { node, pin: index });
        Ok(old)
    }

    pub fn create_latch(
        &mut self,
        input: NodeId,
        output: NodeId,
        initial_value: i32,
        synch_type: LatchSynchType,
        control: Option<NodeId>,
        mapped_gate_present: bool,
    ) -> Result<LatchId, RetimeNetError> {
        self.require_node(input)?;
        self.require_node(output)?;
        let id = LatchId(self.latches.len());
        self.nodes[input.0].latch_end = Some(output);
        self.latches.push(LatchSketch {
            id,
            input,
            output,
            synch_type,
            control,
            initial_value,
            current_value: initial_value,
            mapped_gate_present,
        });
        Ok(id)
    }

    pub fn latch_from_output(&self, output: NodeId) -> Option<&LatchSketch> {
        self.latches.iter().find(|latch| latch.output == output)
    }

    pub fn latch_from_input(&self, input: NodeId) -> Option<&LatchSketch> {
        self.latches.iter().find(|latch| latch.input == input)
    }

    pub fn is_latch_endpoint(&self, node: NodeId) -> bool {
        self.latch_from_input(node).is_some() || self.latch_from_output(node).is_some()
    }

    pub fn retime_network_latch_end(
        &self,
        node: NodeId,
        use_mapped: bool,
    ) -> Result<Option<NodeId>, RetimeNetError> {
        let network_node = self.node(node)?;
        if let Some(latch) = self.latch_from_input(node) {
            return Ok(Some(latch.output));
        }

        if use_mapped && network_node.kind == NodeKind::Internal && network_node.is_mapped_d_latch {
            let po = network_node
                .fanouts
                .first()
                .ok_or(RetimeNetError::MissingLatchOutput(node))?
                .node;
            let end = self
                .node(po)?
                .latch_end
                .ok_or(RetimeNetError::MissingLatchOutput(po))?;
            return Ok(Some(end));
        }

        Ok(None)
    }

    pub fn delete_temp_nodes(&mut self) -> Result<usize, RetimeNetError> {
        let temp_nodes: Vec<NodeId> = self
            .nodes
            .iter()
            .filter(|node| {
                node.kind == NodeKind::Internal
                    && (node.is_temporary || node.name.contains(' '))
                    && !node.fanins.is_empty()
            })
            .map(|node| node.id)
            .collect();

        for temp in &temp_nodes {
            let source = self.nodes[temp.0].fanins[0];
            let fanouts = self.nodes[temp.0].fanouts.clone();
            for fanout in fanouts {
                self.replace_fanin(fanout.node, fanout.pin, source)?;
            }
            self.nodes[temp.0].kind = NodeKind::Deleted;
            self.nodes[temp.0].fanouts.clear();
        }

        Ok(temp_nodes.len())
    }

    fn add_temporary_buffer_after(
        &mut self,
        latch_output: NodeId,
    ) -> Result<NodeId, RetimeNetError> {
        self.require_node(latch_output)?;
        let name = format!("{} temp", self.nodes[latch_output.0].name);
        let temp = self.add_internal(name, &[latch_output]);
        self.nodes[temp.0].is_temporary = true;

        let old_fanouts: Vec<FanoutRef> = self.nodes[latch_output.0]
            .fanouts
            .iter()
            .filter(|fanout| fanout.node != temp)
            .cloned()
            .collect();
        for fanout in old_fanouts {
            self.replace_fanin(fanout.node, fanout.pin, temp)?;
        }

        Ok(temp)
    }

    fn node(&self, id: NodeId) -> Result<&NetworkNode, RetimeNetError> {
        self.nodes.get(id.0).ok_or(RetimeNetError::MissingNode(id))
    }

    fn require_node(&self, id: NodeId) -> Result<(), RetimeNetError> {
        self.node(id).map(|_| ())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetimeFanout {
    pub fanout: NodeId,
    pub fanin_id: usize,
    pub weight: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeNode {
    pub id: NodeId,
    pub network_node: NodeId,
    pub name: String,
    pub kind: RetimeNodeKind,
    pub fanins: Vec<EdgeId>,
    pub fanouts: Vec<EdgeId>,
    pub final_delay: f64,
    pub final_area: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeEdge {
    pub id: EdgeId,
    pub source: NodeId,
    pub sink: NodeId,
    pub sink_fanin_id: usize,
    pub weight: i32,
    pub breadth: f64,
    pub latches: Vec<LatchId>,
    pub initial_values: Vec<i32>,
}

impl RetimeEdge {
    pub fn assure_initial_values(&mut self, latch_values: &[i32]) -> Result<(), RetimeNetError> {
        if self.weight <= 0 || !self.initial_values.is_empty() {
            return Ok(());
        }

        let weight =
            usize::try_from(self.weight).map_err(|_| RetimeNetError::NegativeEdgeWeight {
                edge: self.id,
                weight: self.weight,
            })?;

        if latch_values.is_empty() {
            self.initial_values = vec![LATCH_DONT_CARE; weight];
        } else {
            self.initial_values = latch_values.iter().take(weight).copied().collect();
            self.initial_values.resize(weight, LATCH_DONT_CARE);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RetimeGraph {
    pub nodes: Vec<RetimeNode>,
    pub edges: Vec<RetimeEdge>,
    pub primary_inputs: Vec<NodeId>,
    pub primary_outputs: Vec<NodeId>,
    pub s_type: LatchSynchType,
    pub control_name: Option<String>,
}

impl RetimeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node_from_network(
        &mut self,
        network: &NetworkSketch,
        network_node: NodeId,
    ) -> Result<NodeId, RetimeNetError> {
        let source = network.node(network_node)?;
        let id = NodeId(self.nodes.len());
        let kind = match source.kind {
            NodeKind::PrimaryInput => RetimeNodeKind::PrimaryInput,
            NodeKind::PrimaryOutput => RetimeNodeKind::PrimaryOutput,
            NodeKind::Internal => RetimeNodeKind::Internal,
            NodeKind::Deleted => RetimeNodeKind::Ignore,
        };

        if kind == RetimeNodeKind::PrimaryInput {
            self.primary_inputs.push(id);
        } else if kind == RetimeNodeKind::PrimaryOutput {
            self.primary_outputs.push(id);
        }

        self.nodes.push(RetimeNode {
            id,
            network_node,
            name: source.name.clone(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            final_delay: 0.0,
            final_area: 0.0,
        });
        Ok(id)
    }

    pub fn add_edge(
        &mut self,
        source: NodeId,
        sink: NodeId,
        weight: usize,
        breadth: f64,
        sink_fanin_id: usize,
    ) -> Result<EdgeId, RetimeNetError> {
        self.require_node(source)?;
        self.require_node(sink)?;

        if let Some(existing) = self.nodes[source.0]
            .fanouts
            .iter()
            .copied()
            .find(|edge_id| {
                let edge = &self.edges[edge_id.0];
                edge.sink == sink
                    && edge.weight == weight as i32
                    && edge.sink_fanin_id == sink_fanin_id
            })
        {
            return Ok(existing);
        }

        let id = EdgeId(self.edges.len());
        self.edges.push(RetimeEdge {
            id,
            source,
            sink,
            sink_fanin_id,
            weight: weight as i32,
            breadth,
            latches: Vec::new(),
            initial_values: Vec::new(),
        });
        self.nodes[source.0].fanouts.push(id);
        self.nodes[sink.0].fanins.push(id);
        Ok(id)
    }

    pub fn require_node(&self, id: NodeId) -> Result<(), RetimeNetError> {
        match self.nodes.get(id.0) {
            Some(node) if node.id == id => Ok(()),
            _ => Err(RetimeNetError::MissingNode(id)),
        }
    }

    pub fn edge(&self, id: EdgeId) -> Result<&RetimeEdge, RetimeNetError> {
        self.edges.get(id.0).ok_or(RetimeNetError::MissingEdge(id))
    }

    pub fn edge_mut(&mut self, id: EdgeId) -> Result<&mut RetimeEdge, RetimeNetError> {
        self.edges
            .get_mut(id.0)
            .ok_or(RetimeNetError::MissingEdge(id))
    }
}

pub fn retime_gen_weights(
    network: &NetworkSketch,
    node: NodeId,
    use_mapped: bool,
) -> Result<Vec<RetimeFanout>, RetimeNetError> {
    let start = network.node(node)?;
    if matches!(start.kind, NodeKind::PrimaryInput | NodeKind::PrimaryOutput)
        && network.is_latch_endpoint(node)
    {
        return Ok(Vec::new());
    }

    let mut fanouts = Vec::new();
    let mut depth_by_node = HashMap::from([(node, 0_usize)]);
    let mut queue = VecDeque::from([node]);

    while let Some(current) = queue.pop_front() {
        let depth = depth_by_node[&current];
        let current_node = network.node(current)?;

        if current_node.kind == NodeKind::PrimaryOutput {
            if let Some(latch) = network.latch_from_input(current) {
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    depth_by_node.entry(latch.output)
                {
                    entry.insert(depth + 1);
                    queue.push_back(latch.output);
                }
            }
            continue;
        }

        for fanout in &current_node.fanouts {
            let end = network.retime_network_latch_end(fanout.node, use_mapped)?;
            if end.is_none() {
                if !network.node(fanout.node)?.is_control {
                    fanouts.push(RetimeFanout {
                        fanout: fanout.node,
                        fanin_id: fanout.pin,
                        weight: depth,
                    });
                }
            } else if let std::collections::hash_map::Entry::Vacant(entry) =
                depth_by_node.entry(fanout.node)
            {
                entry.insert(depth);
                queue.push_back(fanout.node);
            }
        }
    }

    Ok(fanouts)
}

pub fn latch_outputs_to_bfs_nodes(network: &NetworkSketch, latches: &[LatchId]) -> Vec<NodeId> {
    let mut visited = HashSet::new();
    let mut bfs = Vec::new();

    for latch_id in latches {
        let Some(latch) = network.latches.get(latch_id.0) else {
            continue;
        };
        let Some(output) = network.nodes.get(latch.output.0) else {
            continue;
        };
        for fanout in &output.fanouts {
            if network
                .nodes
                .get(fanout.node.0)
                .is_some_and(|node| node.kind == NodeKind::Internal)
                && visited.insert(fanout.node)
            {
                bfs.push(fanout.node);
            }
        }
    }

    bfs
}

pub fn retime_network_to_graph(
    network: &NetworkSketch,
    use_mapped: bool,
) -> Result<RetimeGraph, RetimeNetError> {
    if use_mapped && !network.mapped_d_latch_available {
        return Err(RetimeNetError::MissingMappedDLatch);
    }

    let mut working = network.clone();
    let mut graph = RetimeGraph::new();
    let mut node_to_graph = HashMap::new();

    for latch in &working.latches {
        if latch.synch_type != LatchSynchType::Unknown {
            graph.s_type = latch.synch_type;
        }
        if graph.control_name.is_none() {
            graph.control_name = latch
                .control
                .and_then(|control| working.nodes.get(control.0))
                .and_then(|control| control.fanins.first().copied())
                .and_then(|fanin| working.nodes.get(fanin.0))
                .map(|fanin| fanin.name.clone());
        }
    }

    for node in &working.nodes {
        if node.kind == NodeKind::Deleted
            || working.is_latch_endpoint(node.id)
            || node.is_control
            || (use_mapped && node.kind == NodeKind::Internal && node.is_mapped_d_latch)
        {
            continue;
        }
        let graph_id = graph.add_node_from_network(&working, node.id)?;
        node_to_graph.insert(node.id, graph_id);
    }

    let mut seeds: Vec<NodeId> = working
        .nodes
        .iter()
        .filter(|node| !working.is_latch_endpoint(node.id) && node.fanins.is_empty())
        .map(|node| node.id)
        .collect();
    let mut latch_table: HashSet<LatchId> = working.latches.iter().map(|latch| latch.id).collect();

    add_bfs_nodes(
        &working,
        &mut graph,
        &mut seeds,
        &mut latch_table,
        &node_to_graph,
        use_mapped,
    )?;

    if !latch_table.is_empty() {
        let remaining: Vec<LatchId> = latch_table.iter().copied().collect();
        let mut latch_end_bfs = latch_outputs_to_bfs_nodes(&working, &remaining);
        add_bfs_nodes(
            &working,
            &mut graph,
            &mut latch_end_bfs,
            &mut latch_table,
            &node_to_graph,
            use_mapped,
        )?;
    }

    if !latch_table.is_empty() {
        let remaining: Vec<LatchId> = latch_table.iter().copied().collect();
        let mut temp_nodes = Vec::new();
        for latch_id in remaining {
            let latch = working
                .latches
                .get(latch_id.0)
                .ok_or(RetimeNetError::MissingLatch(latch_id))?;
            if working.node(latch.output)?.fanouts.len() > 1 {
                let temp = working.add_temporary_buffer_after(latch.output)?;
                let graph_id = graph.add_node_from_network(&working, temp)?;
                graph.nodes[graph_id.0].final_delay = 0.0;
                graph.nodes[graph_id.0].final_area = 0.0;
                node_to_graph.insert(temp, graph_id);
                temp_nodes.push(temp);
            }
        }

        add_bfs_nodes(
            &working,
            &mut graph,
            &mut temp_nodes,
            &mut latch_table,
            &node_to_graph,
            use_mapped,
        )?;
    }

    if !latch_table.is_empty() {
        let mut latches: Vec<LatchId> = latch_table.into_iter().collect();
        latches.sort_unstable();
        return Err(RetimeNetError::UntraversedLatchCycle { latches });
    }

    Ok(graph)
}

fn add_bfs_nodes(
    network: &NetworkSketch,
    graph: &mut RetimeGraph,
    nodevec: &mut Vec<NodeId>,
    latch_table: &mut HashSet<LatchId>,
    node_to_graph: &HashMap<NodeId, NodeId>,
    use_mapped: bool,
) -> Result<(), RetimeNetError> {
    let mut visited = HashSet::new();
    let mut first = 0;

    while first < nodevec.len() {
        let last = nodevec.len();
        for i in first..last {
            let node = nodevec[i];
            if visited.contains(&node) {
                continue;
            }

            let fanouts = retime_gen_weights(network, node, use_mapped)?;
            for fanout in fanouts {
                if !visited.contains(&fanout.fanout) && !nodevec.contains(&fanout.fanout) {
                    nodevec.push(fanout.fanout);
                }

                let source = *node_to_graph
                    .get(&node)
                    .ok_or(RetimeNetError::MissingGraphNodeForNetworkNode(node))?;
                let sink = *node_to_graph.get(&fanout.fanout).ok_or(
                    RetimeNetError::MissingGraphNodeForNetworkNode(fanout.fanout),
                )?;
                let edge = graph.add_edge(source, sink, fanout.weight, 1.0, fanout.fanin_id)?;
                let latches = gen_latches(
                    network,
                    fanout.fanout,
                    fanout.fanin_id,
                    fanout.weight,
                    use_mapped,
                )?;
                for latch in &latches {
                    latch_table.remove(latch);
                }
                let initial_values = latches
                    .iter()
                    .map(|latch| {
                        network
                            .latches
                            .get(latch.0)
                            .map(|latch| latch.initial_value)
                            .ok_or(RetimeNetError::MissingLatch(*latch))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let edge = graph.edge_mut(edge)?;
                edge.latches = latches;
                edge.initial_values = initial_values;
            }

            visited.insert(node);
        }
        first = last;
    }

    Ok(())
}

pub fn gen_latches(
    network: &NetworkSketch,
    to: NodeId,
    index: usize,
    num: usize,
    use_mapped: bool,
) -> Result<Vec<LatchId>, RetimeNetError> {
    if num == 0 {
        return Ok(Vec::new());
    }

    let actual_to = *network
        .node(to)?
        .fanins
        .get(index)
        .ok_or(RetimeNetError::MissingFanin { node: to, index })?;
    let mut pi = actual_to;
    let mut latches = vec![LatchId(usize::MAX); num];

    for i in (0..num).rev() {
        let latch = network
            .latch_from_output(pi)
            .ok_or(RetimeNetError::MissingLatchOutput(pi))?;
        latches[i] = latch.id;

        let po = latch.input;
        pi = if use_mapped {
            let buffer = *network
                .node(po)?
                .fanins
                .first()
                .ok_or(RetimeNetError::MissingFanin { node: po, index: 0 })?;
            *network
                .node(buffer)?
                .fanins
                .first()
                .ok_or(RetimeNetError::MissingFanin {
                    node: buffer,
                    index: 0,
                })?
        } else {
            *network
                .node(po)?
                .fanins
                .first()
                .ok_or(RetimeNetError::MissingFanin { node: po, index: 0 })?
        };
    }

    Ok(latches)
}

pub fn retime_graph_to_network(graph: &RetimeGraph) -> Result<NetworkSketch, RetimeNetError> {
    let mut network = NetworkSketch::new();
    let mut copied = HashMap::new();

    for node in &graph.nodes {
        if matches!(
            node.kind,
            RetimeNodeKind::PrimaryOutput | RetimeNodeKind::Ignore
        ) {
            continue;
        }
        let kind = match node.kind {
            RetimeNodeKind::PrimaryInput => NodeKind::PrimaryInput,
            RetimeNodeKind::Internal => NodeKind::Internal,
            RetimeNodeKind::PrimaryOutput | RetimeNodeKind::Ignore => unreachable!(),
        };
        let copy = network.add_node(node.name.clone(), kind);
        copied.insert(node.id, copy);
    }

    for node in &graph.nodes {
        if node.kind == RetimeNodeKind::Internal {
            let to = copied[&node.id];
            for edge_id in &node.fanins {
                let edge = graph.edge(*edge_id)?;
                if edge.weight == 0 {
                    let from = copied[&edge.source];
                    network.replace_or_connect_fanin(to, edge.sink_fanin_id, from)?;
                } else {
                    add_latches_to_network(&mut network, &copied, edge, graph.s_type)?;
                }
            }
        }
    }

    for edge in &graph.edges {
        let sink = &graph.nodes[edge.sink.0];
        if sink.kind != RetimeNodeKind::PrimaryOutput {
            continue;
        }
        let from = copied[&edge.source];
        let po = network.add_primary_output(sink.name.clone(), from);
        if edge.weight > 0 {
            network.replace_fanin(po, 0, from)?;
            add_latch_chain(&mut network, from, po, 0, edge, graph.s_type)?;
        }
    }

    Ok(network)
}

trait NetworkSketchExt {
    fn replace_or_connect_fanin(
        &mut self,
        node: NodeId,
        index: usize,
        fanin: NodeId,
    ) -> Result<(), RetimeNetError>;
}

impl NetworkSketchExt for NetworkSketch {
    fn replace_or_connect_fanin(
        &mut self,
        node: NodeId,
        index: usize,
        fanin: NodeId,
    ) -> Result<(), RetimeNetError> {
        self.require_node(node)?;
        self.require_node(fanin)?;
        if index < self.nodes[node.0].fanins.len() {
            self.replace_fanin(node, index, fanin)?;
        } else {
            while self.nodes[node.0].fanins.len() < index {
                self.connect(fanin, node);
            }
            self.connect(fanin, node);
        }
        Ok(())
    }
}

fn add_latches_to_network(
    network: &mut NetworkSketch,
    copied: &HashMap<NodeId, NodeId>,
    edge: &RetimeEdge,
    synch_type: LatchSynchType,
) -> Result<(), RetimeNetError> {
    let from = copied[&edge.source];
    let to = copied[&edge.sink];
    add_latch_chain(network, from, to, edge.sink_fanin_id, edge, synch_type)
}

fn add_latch_chain(
    network: &mut NetworkSketch,
    from: NodeId,
    to: NodeId,
    index: usize,
    edge: &RetimeEdge,
    synch_type: LatchSynchType,
) -> Result<(), RetimeNetError> {
    let mut edge = edge.clone();
    let latch_values = edge.initial_values.clone();
    edge.assure_initial_values(&latch_values)?;
    let weight = usize::try_from(edge.weight).map_err(|_| RetimeNetError::NegativeEdgeWeight {
        edge: edge.id,
        weight: edge.weight,
    })?;

    let (mut start, deficit) = share_latches(network, from, weight, &edge.initial_values);
    if deficit == 0 {
        network.replace_or_connect_fanin(to, index, start)?;
        return Ok(());
    }

    let first_new = weight - deficit;
    for offset in 0..deficit {
        let latch_input =
            network.add_primary_output(format!("{}_latch_in_{offset}", edge.id.0), start);
        let latch_output = network.add_primary_input(format!("{}_latch_out_{offset}", edge.id.0));
        network.create_latch(
            latch_input,
            latch_output,
            edge.initial_values[first_new + offset],
            synch_type,
            None,
            false,
        )?;
        start = latch_output;
    }

    network.replace_or_connect_fanin(to, index, start)?;
    Ok(())
}

pub fn share_latches(
    network: &NetworkSketch,
    node: NodeId,
    required: usize,
    initial_values: &[i32],
) -> (NodeId, usize) {
    let mut current = node;
    let mut deficit = required;

    for value in initial_values.iter().take(required) {
        let Some(next_latch_input) = get_latch_input(network, current, *value) else {
            break;
        };
        let Some(latch_end) = network
            .latch_from_input(next_latch_input)
            .map(|latch| latch.output)
        else {
            break;
        };
        current = latch_end;
        deficit -= 1;
        if deficit == 0 {
            break;
        }
    }

    (current, deficit)
}

pub fn get_latch_input(network: &NetworkSketch, node: NodeId, value: i32) -> Option<NodeId> {
    let fanouts = &network.nodes.get(node.0)?.fanouts;
    for fanout in fanouts {
        let fanout_node = network.nodes.get(fanout.node.0)?;
        if fanout_node.is_mapped_d_latch {
            return fanout_node.fanouts.first().map(|next| next.node);
        }

        if let Some(latch_end) = network
            .latch_from_input(fanout.node)
            .map(|latch| latch.output)
        {
            let latch = network.latch_from_output(latch_end)?;
            if latch.initial_value == value {
                return Some(fanout.node);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn latch_chain_network() -> NetworkSketch {
        let mut network = NetworkSketch::new();
        let a = network.add_primary_input("a");
        let l0_in = network.add_primary_output("l0_in", a);
        let l0_out = network.add_primary_input("l0_out");
        network
            .create_latch(l0_in, l0_out, 0, LatchSynchType::RisingEdge, None, false)
            .unwrap();

        let l1_in = network.add_primary_output("l1_in", l0_out);
        let l1_out = network.add_primary_input("l1_out");
        network
            .create_latch(l1_in, l1_out, 1, LatchSynchType::RisingEdge, None, false)
            .unwrap();

        network.add_internal("n", &[l1_out]);
        network
    }

    #[test]
    fn gen_weights_collapses_latch_chain_into_weighted_fanout() {
        let network = latch_chain_network();

        let fanouts = retime_gen_weights(&network, NodeId(0), false).unwrap();

        assert_eq!(
            fanouts,
            vec![RetimeFanout {
                fanout: NodeId(5),
                fanin_id: 0,
                weight: 2
            }]
        );
    }

    #[test]
    fn network_to_graph_preserves_latch_weight_and_initial_values() {
        let network = latch_chain_network();

        let graph = retime_network_to_graph(&network, false).unwrap();

        assert_eq!(graph.s_type, LatchSynchType::RisingEdge);
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        let edge = &graph.edges[0];
        assert_eq!(edge.weight, 2);
        assert_eq!(edge.initial_values, vec![0, 1]);
        assert_eq!(edge.latches, vec![LatchId(0), LatchId(1)]);
    }

    #[test]
    fn latch_outputs_seed_bfs_for_components_without_primary_inputs() {
        let mut network = NetworkSketch::new();
        let latch_out = network.add_primary_input("state");
        let internal = network.add_internal("logic", &[latch_out]);
        let latch_in = network.add_primary_output("next", internal);
        network
            .create_latch(
                latch_in,
                latch_out,
                1,
                LatchSynchType::FallingEdge,
                None,
                false,
            )
            .unwrap();

        let bfs = latch_outputs_to_bfs_nodes(&network, &[LatchId(0)]);

        assert_eq!(bfs, vec![internal]);
    }

    #[test]
    fn assure_initial_values_uses_latches_then_dont_care_padding() {
        let mut edge = RetimeEdge {
            id: EdgeId(3),
            source: NodeId(0),
            sink: NodeId(1),
            sink_fanin_id: 0,
            weight: 3,
            breadth: 1.0,
            latches: Vec::new(),
            initial_values: Vec::new(),
        };

        edge.assure_initial_values(&[0, 1]).unwrap();

        assert_eq!(edge.initial_values, vec![0, 1, LATCH_DONT_CARE]);
    }

    #[test]
    fn graph_to_network_rebuilds_latch_chain_from_positive_edge() {
        let mut graph = RetimeGraph::new();
        graph.s_type = LatchSynchType::RisingEdge;
        let source = graph.nodes.len();
        let pi = graph.add_node_from_network(
            &{
                let mut n = NetworkSketch::new();
                n.add_primary_input("a");
                n
            },
            NodeId(0),
        );
        assert_eq!(pi.unwrap().0, source);

        let mut seed = NetworkSketch::new();
        seed.add_primary_input("a");
        seed.add_internal("n", &[]);
        let n = graph.add_node_from_network(&seed, NodeId(1)).unwrap();
        let edge = graph.add_edge(NodeId(0), n, 2, 1.0, 0).unwrap();
        graph.edges[edge.0].initial_values = vec![1, 0];

        let network = retime_graph_to_network(&graph).unwrap();

        assert_eq!(network.latches.len(), 2);
        assert_eq!(
            network
                .latches
                .iter()
                .map(|latch| latch.initial_value)
                .collect::<Vec<_>>(),
            vec![1, 0]
        );
        assert_eq!(network.nodes[1].fanins, vec![network.latches[1].output]);
    }

    #[test]
    fn delete_temp_nodes_patches_fanouts_back_to_original_source() {
        let mut network = NetworkSketch::new();
        let source = network.add_primary_input("a");
        let temp = network.add_internal("a temp", &[source]);
        let sink = network.add_internal("sink", &[temp]);

        let deleted = network.delete_temp_nodes().unwrap();

        assert_eq!(deleted, 1);
        assert_eq!(network.nodes[temp.0].kind, NodeKind::Deleted);
        assert_eq!(network.nodes[sink.0].fanins, vec![source]);
    }

    #[test]
    fn dependency_scaffold_reports_beads_and_source_files() {
        assert!(required_sis_dependencies().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.305"
                && dependency.source_file == "LogicSynthesis/sis/network/network_util.c"
        }));
        assert!(required_sis_dependencies().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.422"
                && dependency.source_file == "LogicSynthesis/sis/retime/re_util.c"
        }));

        assert_eq!(
            sis_network_to_graph_blocked(),
            Err(RetimeNetError::MissingSisPorts {
                operation: "retime_network_to_graph over SIS network_t",
                dependencies: REQUIRED_SIS_DEPENDENCIES,
            })
        );
    }
}
