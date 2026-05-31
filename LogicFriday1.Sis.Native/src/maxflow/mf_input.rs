//! Native Rust graph input model for `LogicSynthesis/sis/maxflow/mf_input.c`.
//!
//! The legacy C module owns allocation and mutation of the maxflow graph:
//! nodes are read by name, directed capacity edges are attached to fanout and
//! fanin lists, existing edges can be reread with a new capacity, and node
//! removal drops all incident edges. This Rust port keeps those semantics as a
//! safe owned data model.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MfNodeKind
{
    Internal,
    Source,
    Sink,
    Fictitious,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MfNode
{
    pub name: String,
    pub kind: MfNodeKind,
    pub fanin_edges: Vec<usize>,
    pub fanout_edges: Vec<usize>,
    pub increment_flow: i32,
    pub direction: i16,
    pub labelled: bool,
    pub marked: bool,
    pub current_trace: bool,
}

impl MfNode
{
    fn new(name: String, kind: MfNodeKind) -> Self
    {
        Self
        {
            name,
            kind,
            fanin_edges: Vec::new(),
            fanout_edges: Vec::new(),
            increment_flow: 0,
            direction: 0,
            labelled: false,
            marked: false,
            current_trace: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MfEdge
{
    pub tail: usize,
    pub head: usize,
    pub capacity: i32,
    pub flow: i32,
    pub on_min_cut: bool,
}

impl MfEdge
{
    fn new(tail: usize, head: usize, capacity: i32) -> Self
    {
        Self
        {
            tail,
            head,
            capacity,
            flow: 0,
            on_min_cut: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MfCutset
{
    pub from_node: Vec<String>,
    pub to_node: Vec<String>,
    pub capacity: Vec<i32>,
}

impl MfCutset
{
    pub fn new(from_node: Vec<String>, to_node: Vec<String>, capacity: Vec<i32>) -> Self
    {
        Self
        {
            from_node,
            to_node,
            capacity,
        }
    }

    pub fn len(&self) -> usize
    {
        self.capacity.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.capacity.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MfInputError
{
    DuplicateNode(String),
    MultipleSource,
    MultipleSink,
    UnknownNode(String),
    NegativeCapacity(i32),
    SelfLoop(String),
}

impl fmt::Display for MfInputError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::DuplicateNode(name) => write!(f, "node {name} defined twice"),
            Self::MultipleSource => write!(f, "multiple declaration of source node"),
            Self::MultipleSink => write!(f, "multiple declaration of sink node"),
            Self::UnknownNode(name) => write!(f, "node {name} is undefined"),
            Self::NegativeCapacity(capacity) => {
                write!(f, "negative capacity assigned: {capacity}")
            }
            Self::SelfLoop(name) => write!(f, "self-loop is not allowed for node {name}"),
        }
    }
}

impl Error for MfInputError {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MfGraph
{
    nodes: Vec<MfNode>,
    node_by_name: HashMap<String, usize>,
    edges: Vec<Option<MfEdge>>,
    source_node: Option<usize>,
    sink_node: Option<usize>,
}

impl MfGraph
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn read_node(
        &mut self,
        name: impl Into<String>,
        kind: MfNodeKind,
    ) -> Result<usize, MfInputError>
    {
        let name = name.into();
        if self.node_by_name.contains_key(&name)
        {
            return Err(MfInputError::DuplicateNode(name));
        }

        match kind
        {
            MfNodeKind::Source if self.source_node.is_some() => {
                return Err(MfInputError::MultipleSource);
            }
            MfNodeKind::Sink if self.sink_node.is_some() => {
                return Err(MfInputError::MultipleSink);
            }
            _ => {}
        }

        let index = self.nodes.len();
        self.nodes.push(MfNode::new(name.clone(), kind));
        self.node_by_name.insert(name, index);
        match kind
        {
            MfNodeKind::Source => self.source_node = Some(index),
            MfNodeKind::Sink => self.sink_node = Some(index),
            MfNodeKind::Internal | MfNodeKind::Fictitious => {}
        }

        Ok(index)
    }

    pub fn read_edge(
        &mut self,
        tail: impl AsRef<str>,
        head: impl AsRef<str>,
        capacity: i32,
    ) -> Result<usize, MfInputError>
    {
        let (tail, head) = self.validate_edge(tail.as_ref(), head.as_ref(), capacity)?;
        Ok(self.push_edge(tail, head, capacity))
    }

    pub fn reread_edge(
        &mut self,
        tail: impl AsRef<str>,
        head: impl AsRef<str>,
        capacity: i32,
    ) -> Result<bool, MfInputError>
    {
        let (tail, head) = self.validate_edge(tail.as_ref(), head.as_ref(), capacity)?;
        if let Some(edge_index) = self.find_edge(tail, head)
        {
            let edge = self.edges[edge_index]
                .as_mut()
                .expect("active edge index points at an active edge");
            edge.capacity = capacity;
            edge.flow = edge.flow.clamp(0, capacity);
            return Ok(false);
        }

        self.push_edge(tail, head, capacity);
        Ok(true)
    }

    pub fn remove_node(&mut self, name: &str) -> bool
    {
        let Some(index) = self.node_by_name.remove(name) else
        {
            return false;
        };

        let incident_edges: Vec<usize> = self.nodes[index]
            .fanin_edges
            .iter()
            .chain(self.nodes[index].fanout_edges.iter())
            .copied()
            .collect();
        for edge_index in incident_edges
        {
            self.remove_edge(edge_index);
        }

        if self.source_node == Some(index)
        {
            self.source_node = None;
        }
        if self.sink_node == Some(index)
        {
            self.sink_node = None;
        }

        self.nodes.remove(index);
        self.reindex_after_removed_node(index);
        true
    }

    pub fn change_node_type(
        &mut self,
        name: &str,
        kind: MfNodeKind,
    ) -> Result<(), MfInputError>
    {
        let index = self
            .node_index(name)
            .ok_or_else(|| MfInputError::UnknownNode(name.to_owned()))?;

        if self.source_node == Some(index) && kind != MfNodeKind::Source
        {
            self.source_node = None;
        }
        if self.sink_node == Some(index) && kind != MfNodeKind::Sink
        {
            self.sink_node = None;
        }
        match kind
        {
            MfNodeKind::Source => self.source_node = Some(index),
            MfNodeKind::Sink => self.sink_node = Some(index),
            MfNodeKind::Internal | MfNodeKind::Fictitious => {}
        }
        self.nodes[index].kind = kind;
        Ok(())
    }

    pub fn node_index(&self, name: &str) -> Option<usize>
    {
        self.node_by_name.get(name).copied()
    }

    pub fn get_node(&self, name: &str) -> Option<&MfNode>
    {
        self.node_index(name).and_then(|index| self.nodes.get(index))
    }

    pub fn get_node_mut(&mut self, name: &str) -> Option<&mut MfNode>
    {
        self.node_index(name)
            .and_then(|index| self.nodes.get_mut(index))
    }

    pub fn source_node(&self) -> Option<&MfNode>
    {
        self.source_node.and_then(|index| self.nodes.get(index))
    }

    pub fn sink_node(&self) -> Option<&MfNode>
    {
        self.sink_node.and_then(|index| self.nodes.get(index))
    }

    pub fn nodes(&self) -> &[MfNode]
    {
        &self.nodes
    }

    pub fn edges(&self) -> impl Iterator<Item = &MfEdge>
    {
        self.edges.iter().filter_map(Option::as_ref)
    }

    pub fn edge(&self, index: usize) -> Option<&MfEdge>
    {
        self.edges.get(index).and_then(Option::as_ref)
    }

    pub fn edge_mut(&mut self, index: usize) -> Option<&mut MfEdge>
    {
        self.edges.get_mut(index).and_then(Option::as_mut)
    }

    pub fn num_nodes(&self) -> usize
    {
        self.nodes.len()
    }

    pub fn num_edges(&self) -> usize
    {
        self.edges.iter().filter(|edge| edge.is_some()).count()
    }

    pub fn num_fanin(&self, name: &str) -> Option<usize>
    {
        self.get_node(name).map(|node| node.fanin_edges.len())
    }

    pub fn num_fanout(&self, name: &str) -> Option<usize>
    {
        self.get_node(name).map(|node| node.fanout_edges.len())
    }

    pub fn tail_of_edge(&self, edge: &MfEdge) -> Option<&MfNode>
    {
        self.nodes.get(edge.tail)
    }

    pub fn head_of_edge(&self, edge: &MfEdge) -> Option<&MfNode>
    {
        self.nodes.get(edge.head)
    }

    fn validate_edge(
        &self,
        tail: &str,
        head: &str,
        capacity: i32,
    ) -> Result<(usize, usize), MfInputError>
    {
        if capacity < 0
        {
            return Err(MfInputError::NegativeCapacity(capacity));
        }
        if tail == head
        {
            return Err(MfInputError::SelfLoop(tail.to_owned()));
        }

        let tail = self
            .node_index(tail)
            .ok_or_else(|| MfInputError::UnknownNode(tail.to_owned()))?;
        let head = self
            .node_index(head)
            .ok_or_else(|| MfInputError::UnknownNode(head.to_owned()))?;
        Ok((tail, head))
    }

    fn push_edge(&mut self, tail: usize, head: usize, capacity: i32) -> usize
    {
        let edge_index = self.edges.len();
        self.edges.push(Some(MfEdge::new(tail, head, capacity)));
        self.nodes[tail].fanout_edges.push(edge_index);
        self.nodes[head].fanin_edges.push(edge_index);
        edge_index
    }

    fn find_edge(&self, tail: usize, head: usize) -> Option<usize>
    {
        self.nodes[tail].fanout_edges.iter().copied().find(|edge_index| {
            self.edges
                .get(*edge_index)
                .and_then(Option::as_ref)
                .is_some_and(|edge| edge.head == head)
        })
    }

    fn remove_edge(&mut self, edge_index: usize)
    {
        let Some(edge) = self.edges.get_mut(edge_index).and_then(Option::take) else
        {
            return;
        };

        self.nodes[edge.tail]
            .fanout_edges
            .retain(|existing| *existing != edge_index);
        self.nodes[edge.head]
            .fanin_edges
            .retain(|existing| *existing != edge_index);
    }

    fn reindex_after_removed_node(&mut self, removed_index: usize)
    {
        self.node_by_name.clear();
        for (index, node) in self.nodes.iter().enumerate()
        {
            self.node_by_name.insert(node.name.clone(), index);
        }

        for edge in self.edges.iter_mut().filter_map(Option::as_mut)
        {
            if edge.tail > removed_index
            {
                edge.tail -= 1;
            }
            if edge.head > removed_index
            {
                edge.head -= 1;
            }
        }

        self.source_node = self.source_node.map(|index| {
            if index > removed_index
            {
                index - 1
            }
            else
            {
                index
            }
        });
        self.sink_node = self.sink_node.map(|index| {
            if index > removed_index
            {
                index - 1
            }
            else
            {
                index
            }
        });
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn reads_nodes_and_edges_by_name()
    {
        let mut graph = MfGraph::new();
        graph.read_node("source", MfNodeKind::Source).unwrap();
        graph.read_node("sink", MfNodeKind::Sink).unwrap();
        graph.read_node("middle", MfNodeKind::Internal).unwrap();

        let first = graph.read_edge("source", "middle", 4).unwrap();
        let second = graph.read_edge("middle", "sink", 3).unwrap();

        assert_eq!(graph.num_nodes(), 3);
        assert_eq!(graph.num_edges(), 2);
        assert_eq!(graph.source_node().map(|node| node.name.as_str()), Some("source"));
        assert_eq!(graph.sink_node().map(|node| node.name.as_str()), Some("sink"));
        assert_eq!(graph.num_fanout("source"), Some(1));
        assert_eq!(graph.num_fanin("sink"), Some(1));
        assert_eq!(graph.edge(first).map(|edge| edge.capacity), Some(4));
        assert_eq!(graph.edge(second).map(|edge| edge.capacity), Some(3));
    }

    #[test]
    fn rejects_invalid_node_and_edge_input()
    {
        let mut graph = MfGraph::new();
        graph.read_node("source", MfNodeKind::Source).unwrap();
        graph.read_node("sink", MfNodeKind::Sink).unwrap();

        assert_eq!(
            graph.read_node("source", MfNodeKind::Internal),
            Err(MfInputError::DuplicateNode("source".to_owned()))
        );
        assert_eq!(
            graph.read_node("again", MfNodeKind::Source),
            Err(MfInputError::MultipleSource)
        );
        assert_eq!(
            graph.read_edge("source", "sink", -1),
            Err(MfInputError::NegativeCapacity(-1))
        );
        assert_eq!(
            graph.read_edge("source", "source", 1),
            Err(MfInputError::SelfLoop("source".to_owned()))
        );
        assert_eq!(
            graph.read_edge("source", "missing", 1),
            Err(MfInputError::UnknownNode("missing".to_owned()))
        );
    }

    #[test]
    fn reread_edge_updates_or_creates_edge()
    {
        let mut graph = sample_graph();
        assert_eq!(graph.reread_edge("a", "b", 5), Ok(false));
        assert_eq!(graph.num_edges(), 2);
        assert_eq!(
            graph
                .edges()
                .find(|edge| edge.tail == graph.node_index("a").unwrap())
                .map(|edge| edge.capacity),
            Some(5)
        );

        assert_eq!(graph.reread_edge("b", "source", 2), Ok(true));

        assert_eq!(graph.num_edges(), 3);
        assert_eq!(graph.num_fanout("b"), Some(2));
        assert_eq!(graph.num_fanin("source"), Some(1));
    }

    #[test]
    fn remove_node_drops_incident_edges_and_reindexes()
    {
        let mut graph = sample_graph();
        graph.read_node("c", MfNodeKind::Internal).unwrap();
        graph.read_edge("source", "c", 8).unwrap();
        graph.read_edge("c", "sink", 9).unwrap();

        assert!(graph.remove_node("a"));

        assert_eq!(graph.num_nodes(), 4);
        assert_eq!(graph.num_edges(), 3);
        assert_eq!(graph.get_node("a"), None);
        assert_eq!(graph.num_fanout("source"), Some(1));
        assert_eq!(graph.num_fanin("b"), Some(0));
        assert_eq!(graph.node_index("sink"), Some(1));
        assert_eq!(graph.sink_node().map(|node| node.name.as_str()), Some("sink"));
        assert!(!graph.remove_node("a"));
    }

    #[test]
    fn change_node_type_tracks_current_source_and_sink()
    {
        let mut graph = sample_graph();

        graph.change_node_type("a", MfNodeKind::Source).unwrap();
        graph.change_node_type("sink", MfNodeKind::Internal).unwrap();
        graph.change_node_type("b", MfNodeKind::Sink).unwrap();

        assert_eq!(graph.source_node().map(|node| node.name.as_str()), Some("a"));
        assert_eq!(graph.sink_node().map(|node| node.name.as_str()), Some("b"));
        assert_eq!(
            graph.change_node_type("missing", MfNodeKind::Sink),
            Err(MfInputError::UnknownNode("missing".to_owned()))
        );
    }

    #[test]
    fn cutset_container_preserves_arc_arrays()
    {
        let cutset = MfCutset::new(
            vec!["source".to_owned()],
            vec!["sink".to_owned()],
            vec![7],
        );

        assert_eq!(cutset.len(), 1);
        assert!(!cutset.is_empty());
        assert_eq!(cutset.from_node[0], "source");
        assert_eq!(cutset.to_node[0], "sink");
        assert_eq!(cutset.capacity[0], 7);
    }

    fn sample_graph() -> MfGraph
    {
        let mut graph = MfGraph::new();
        graph.read_node("source", MfNodeKind::Source).unwrap();
        graph.read_node("sink", MfNodeKind::Sink).unwrap();
        graph.read_node("a", MfNodeKind::Internal).unwrap();
        graph.read_node("b", MfNodeKind::Internal).unwrap();
        graph.read_edge("a", "b", 1).unwrap();
        graph.read_edge("b", "sink", 1).unwrap();
        graph
    }
}
