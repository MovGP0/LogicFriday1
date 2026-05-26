use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetmakeNodeKind {
    Internal,
    PrimaryInput,
    PrimaryOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetmakeNode<F> {
    pub name: String,
    pub kind: NetmakeNodeKind,
    pub fanins: Vec<NodeId>,
    pub function: Option<F>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetmakeNetwork<F> {
    nodes: Vec<NetmakeNode<F>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetmakeError {
    MissingNode { node: NodeId },
    MissingFanin { node: NodeId, fanin: NodeId },
    DuplicateRequestedNode { node: NodeId },
}

pub fn network_create_from_node<F>(
    source: &NetmakeNetwork<F>,
    node: NodeId,
) -> Result<NetmakeNetwork<F>, NetmakeError>
where
    F: Clone,
{
    network_from_nodevec(source, [node])
}

pub fn network_from_nodevec<F, I>(
    source: &NetmakeNetwork<F>,
    nodes: I,
) -> Result<NetmakeNetwork<F>, NetmakeError>
where
    F: Clone,
    I: IntoIterator<Item = NodeId>,
{
    let requested = nodes.into_iter().collect::<Vec<_>>();
    let mut result = NetmakeNetwork::new();
    let mut copies = HashMap::with_capacity(requested.len());

    for node in &requested {
        let source_node = source
            .node(*node)
            .ok_or(NetmakeError::MissingNode { node: *node })?;

        if copies.contains_key(node) {
            return Err(NetmakeError::DuplicateRequestedNode { node: *node });
        }

        let copy = result.add_node(NetmakeNode {
            name: source_node.name.clone(),
            kind: source_node.kind,
            fanins: Vec::new(),
            function: source_node.function.clone(),
        });
        copies.insert(*node, copy);
    }

    for node in &requested {
        let source_node = source
            .node(*node)
            .ok_or(NetmakeError::MissingNode { node: *node })?;
        let mut replacement_fanins = Vec::with_capacity(source_node.fanins.len());

        for fanin in &source_node.fanins {
            if let Some(copy) = copies.get(fanin) {
                replacement_fanins.push(*copy);
                continue;
            }

            let source_fanin = source.node(*fanin).ok_or(NetmakeError::MissingFanin {
                node: *node,
                fanin: *fanin,
            })?;
            let copy = result.add_primary_input(source_fanin.name.clone());
            copies.insert(*fanin, copy);
            replacement_fanins.push(copy);
        }

        let copy = copies[node];
        let target = result
            .node_mut(copy)
            .ok_or(NetmakeError::MissingNode { node: copy })?;
        target.fanins = replacement_fanins;
        target.function = source_node.function.clone();
    }

    result.promote_unobserved_nodes_to_outputs();

    Ok(result)
}

impl<F> NetmakeNetwork<F> {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: NetmakeNode<F>) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> NodeId {
        self.add_node(NetmakeNode {
            name: name.into(),
            kind: NetmakeNodeKind::PrimaryInput,
            fanins: Vec::new(),
            function: None,
        })
    }

    pub fn add_internal(
        &mut self,
        name: impl Into<String>,
        fanins: impl IntoIterator<Item = NodeId>,
        function: F,
    ) -> NodeId {
        self.add_node(NetmakeNode {
            name: name.into(),
            kind: NetmakeNodeKind::Internal,
            fanins: fanins.into_iter().collect(),
            function: Some(function),
        })
    }

    pub fn add_primary_output(
        &mut self,
        name: impl Into<String>,
        fanins: impl IntoIterator<Item = NodeId>,
        function: F,
    ) -> NodeId {
        self.add_node(NetmakeNode {
            name: name.into(),
            kind: NetmakeNodeKind::PrimaryOutput,
            fanins: fanins.into_iter().collect(),
            function: Some(function),
        })
    }

    pub fn node(&self, id: NodeId) -> Option<&NetmakeNode<F>> {
        self.nodes.get(id.0)
    }

    pub fn nodes(&self) -> &[NetmakeNode<F>] {
        &self.nodes
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = (NodeId, &NetmakeNode<F>)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.kind == NetmakeNodeKind::PrimaryInput)
            .map(|(index, node)| (NodeId(index), node))
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = (NodeId, &NetmakeNode<F>)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.kind == NetmakeNodeKind::PrimaryOutput)
            .map(|(index, node)| (NodeId(index), node))
    }

    pub fn fanout_count(&self, id: NodeId) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.fanins.contains(&id))
            .count()
    }

    fn node_mut(&mut self, id: NodeId) -> Option<&mut NetmakeNode<F>> {
        self.nodes.get_mut(id.0)
    }

    fn promote_unobserved_nodes_to_outputs(&mut self) {
        let mut fanout_counts = vec![0_usize; self.nodes.len()];
        for node in &self.nodes {
            for fanin in &node.fanins {
                if let Some(count) = fanout_counts.get_mut(fanin.0) {
                    *count += 1;
                }
            }
        }

        for (index, node) in self.nodes.iter_mut().enumerate() {
            if node.kind != NetmakeNodeKind::PrimaryOutput && fanout_counts[index] == 0 {
                node.kind = NetmakeNodeKind::PrimaryOutput;
            }
        }
    }
}

impl<F> Default for NetmakeNetwork<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NetmakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode { node } => write!(formatter, "node {} is not present", node.0),
            Self::MissingFanin { node, fanin } => write!(
                formatter,
                "node {} references missing fanin {}",
                node.0, fanin.0
            ),
            Self::DuplicateRequestedNode { node } => {
                write!(formatter, "node {} was requested more than once", node.0)
            }
        }
    }
}

impl Error for NetmakeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_network_from_single_node_and_promotes_it_to_output() {
        let mut source = NetmakeNetwork::new();
        let a = source.add_primary_input("a");
        let b = source.add_primary_input("b");
        let y = source.add_internal("y", [a, b], "and");

        let result = network_create_from_node(&source, y).unwrap();

        assert_eq!(result.nodes().len(), 3);
        assert_eq!(result.primary_inputs().count(), 2);
        assert_eq!(result.primary_outputs().count(), 1);

        let output = result.primary_outputs().next().unwrap().1;
        assert_eq!(output.name, "y");
        assert_eq!(output.fanins, vec![NodeId(1), NodeId(2)]);
        assert_eq!(output.function, Some("and"));
    }

    #[test]
    fn reuses_copied_nodes_for_fanins_inside_requested_vector() {
        let mut source = NetmakeNetwork::new();
        let a = source.add_primary_input("a");
        let b = source.add_primary_input("b");
        let x = source.add_internal("x", [a, b], "and");
        let y = source.add_internal("y", [x], "buf");

        let result = network_from_nodevec(&source, [x, y]).unwrap();

        assert_eq!(result.nodes().len(), 4);
        assert_eq!(result.node(NodeId(0)).unwrap().name, "x");
        assert_eq!(result.node(NodeId(1)).unwrap().name, "y");
        assert_eq!(result.node(NodeId(1)).unwrap().fanins, vec![NodeId(0)]);
        assert_eq!(result.fanout_count(NodeId(0)), 1);
        assert_eq!(
            result.node(NodeId(0)).unwrap().kind,
            NetmakeNodeKind::Internal
        );
        assert_eq!(
            result.node(NodeId(1)).unwrap().kind,
            NetmakeNodeKind::PrimaryOutput
        );
    }

    #[test]
    fn creates_one_boundary_input_for_shared_external_fanin() {
        let mut source = NetmakeNetwork::new();
        let a = source.add_primary_input("a");
        let x = source.add_internal("x", [a], "buf");
        let y = source.add_internal("y", [a], "not");

        let result = network_from_nodevec(&source, [x, y]).unwrap();

        assert_eq!(result.nodes().len(), 3);
        assert_eq!(result.primary_inputs().count(), 1);
        assert_eq!(result.node(NodeId(0)).unwrap().fanins, vec![NodeId(2)]);
        assert_eq!(result.node(NodeId(1)).unwrap().fanins, vec![NodeId(2)]);
        assert_eq!(result.node(NodeId(2)).unwrap().name, "a");
    }

    #[test]
    fn preserves_existing_primary_output_kind() {
        let mut source = NetmakeNetwork::new();
        let a = source.add_primary_input("a");
        let y = source.add_primary_output("y", [a], "buf");

        let result = network_create_from_node(&source, y).unwrap();

        assert_eq!(
            result.node(NodeId(0)).unwrap().kind,
            NetmakeNodeKind::PrimaryOutput
        );
        assert_eq!(result.primary_outputs().count(), 1);
    }

    #[test]
    fn reports_missing_requested_node() {
        let source = NetmakeNetwork::<&str>::new();

        let error = network_create_from_node(&source, NodeId(42)).unwrap_err();

        assert_eq!(error, NetmakeError::MissingNode { node: NodeId(42) });
        assert!(error.to_string().contains("not present"));
    }

    #[test]
    fn reports_missing_fanin() {
        let mut source = NetmakeNetwork::new();
        let y = source.add_internal("y", [NodeId(9)], "buf");

        let error = network_create_from_node(&source, y).unwrap_err();

        assert_eq!(
            error,
            NetmakeError::MissingFanin {
                node: y,
                fanin: NodeId(9)
            }
        );
    }
}
