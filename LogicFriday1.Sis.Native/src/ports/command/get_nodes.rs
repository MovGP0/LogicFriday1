use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeNameMode {
    Long,
    Short,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    Internal,
    PrimaryInput,
    PrimaryOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionDiagnostic {
    pub message: String,
}

impl SelectionDiagnostic {
    pub fn node_not_found(name: impl Into<String>) -> Self {
        Self {
            message: format!("node '{}' was not found", name.into()),
        }
    }

    pub fn missing_primary_output_fanin(name: impl Into<String>) -> Self {
        Self {
            message: format!("primary output '{}' has no fanin", name.into()),
        }
    }
}

impl fmt::Display for SelectionDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeSelection<NodeId> {
    pub nodes: Vec<NodeId>,
    pub diagnostics: Vec<SelectionDiagnostic>,
}

impl<NodeId> Default for NodeSelection<NodeId> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl<NodeId> NodeSelection<NodeId> {
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectableNode<NodeId> {
    pub id: NodeId,
    pub long_name: String,
    pub short_name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub fanouts: Vec<NodeId>,
}

impl<NodeId> SelectableNode<NodeId> {
    pub fn new(id: NodeId, long_name: impl Into<String>, kind: NodeKind) -> Self
    where
        NodeId: Clone,
    {
        let long_name = long_name.into();
        Self {
            id,
            short_name: long_name.clone(),
            long_name,
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
        }
    }

    pub fn with_short_name(mut self, short_name: impl Into<String>) -> Self {
        self.short_name = short_name.into();
        self
    }

    pub fn with_fanins<I>(mut self, fanins: I) -> Self
    where
        I: IntoIterator<Item = NodeId>,
    {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_fanouts<I>(mut self, fanouts: I) -> Self
    where
        I: IntoIterator<Item = NodeId>,
    {
        self.fanouts = fanouts.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug)]
pub struct GetNodesNetwork<NodeId> {
    nodes: Vec<SelectableNode<NodeId>>,
    by_long_name: HashMap<String, usize>,
    by_short_name: HashMap<String, usize>,
    by_id: HashMap<NodeId, usize>,
}

impl<NodeId> Default for GetNodesNetwork<NodeId>
where
    NodeId: Clone + Eq + Hash,
{
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            by_long_name: HashMap::new(),
            by_short_name: HashMap::new(),
            by_id: HashMap::new(),
        }
    }
}

impl<NodeId> GetNodesNetwork<NodeId>
where
    NodeId: Clone + Eq + Hash,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_nodes<I>(nodes: I) -> Self
    where
        I: IntoIterator<Item = SelectableNode<NodeId>>,
    {
        let mut network = Self::new();
        for node in nodes {
            network.add_node(node);
        }
        network
    }

    pub fn add_node(&mut self, node: SelectableNode<NodeId>) {
        let index = self.nodes.len();
        self.by_long_name.insert(node.long_name.clone(), index);
        self.by_short_name.insert(node.short_name.clone(), index);
        self.by_id.insert(node.id.clone(), index);
        self.nodes.push(node);
    }

    pub fn node(&self, id: &NodeId) -> Option<&SelectableNode<NodeId>> {
        self.by_id.get(id).map(|index| &self.nodes[*index])
    }

    pub fn select_from_command_argv<I, S>(
        &self,
        argv: I,
        name_mode: NodeNameMode,
    ) -> NodeSelection<NodeId>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let selectors: Vec<String> = argv
            .into_iter()
            .skip(1)
            .map(|arg| arg.as_ref().to_owned())
            .collect();

        self.select_nodes(selectors, name_mode)
    }

    pub fn select_nodes<I, S>(&self, selectors: I, name_mode: NodeNameMode) -> NodeSelection<NodeId>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let selectors: Vec<String> = selectors
            .into_iter()
            .map(|selector| selector.as_ref().to_owned())
            .collect();

        let mut selection = NodeSelection::default();
        if selectors.is_empty() {
            self.append_all_nodes(&mut selection.nodes);
            return selection;
        }

        for selector in selectors {
            if selector == "*" {
                self.append_all_nodes(&mut selection.nodes);
            } else if selector.starts_with("i(") {
                self.select_fanin_expression(&selector, name_mode, &mut selection);
            } else if selector.starts_with("o(") {
                self.select_fanout_expression(&selector, name_mode, &mut selection);
            } else {
                self.select_named_node(&selector, name_mode, &mut selection);
            }
        }

        selection
    }

    fn append_all_nodes(&self, output: &mut Vec<NodeId>) {
        output.extend(self.nodes.iter().map(|node| node.id.clone()));
    }

    fn select_fanin_expression(
        &self,
        selector: &str,
        name_mode: NodeNameMode,
        selection: &mut NodeSelection<NodeId>,
    ) {
        let name = selector_argument(selector);
        if name.is_empty() {
            selection.nodes.extend(
                self.nodes
                    .iter()
                    .filter(|node| node.kind == NodeKind::PrimaryInput)
                    .map(|node| node.id.clone()),
            );
            return;
        }

        let Some(node) = self.node_by_name(name, name_mode, selection) else {
            return;
        };

        selection.nodes.extend(node.fanins.iter().cloned());
    }

    fn select_fanout_expression(
        &self,
        selector: &str,
        name_mode: NodeNameMode,
        selection: &mut NodeSelection<NodeId>,
    ) {
        let name = selector_argument(selector);
        if name.is_empty() {
            for node in self
                .nodes
                .iter()
                .filter(|node| node.kind == NodeKind::PrimaryOutput)
            {
                match node.fanins.first() {
                    Some(fanin) => selection.nodes.push(fanin.clone()),
                    None => selection.diagnostics.push(
                        SelectionDiagnostic::missing_primary_output_fanin(node.long_name.clone()),
                    ),
                }
            }
            return;
        }

        let Some(node) = self.node_by_name(name, name_mode, selection) else {
            return;
        };

        selection.nodes.extend(node.fanouts.iter().cloned());
    }

    fn select_named_node(
        &self,
        name: &str,
        name_mode: NodeNameMode,
        selection: &mut NodeSelection<NodeId>,
    ) {
        if let Some(node) = self.node_by_name(name, name_mode, selection) {
            selection.nodes.push(node.id.clone());
        }
    }

    fn node_by_name(
        &self,
        name: &str,
        name_mode: NodeNameMode,
        selection: &mut NodeSelection<NodeId>,
    ) -> Option<&SelectableNode<NodeId>> {
        let table = match name_mode {
            NodeNameMode::Long => &self.by_long_name,
            NodeNameMode::Short => &self.by_short_name,
        };

        let Some(index) = table.get(name).copied() else {
            selection
                .diagnostics
                .push(SelectionDiagnostic::node_not_found(name));
            return None;
        };

        let node = &self.nodes[index];
        if node.kind != NodeKind::PrimaryOutput {
            return Some(node);
        }

        let Some(fanin) = node.fanins.first() else {
            selection
                .diagnostics
                .push(SelectionDiagnostic::missing_primary_output_fanin(
                    node.long_name.clone(),
                ));
            return None;
        };

        self.node(fanin)
    }
}

pub fn selector_argument(selector: &str) -> &str {
    let Some(open_index) = selector.find('(') else {
        return "";
    };
    let argument = &selector[open_index + 1..];
    argument.strip_suffix(')').unwrap_or(argument)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> GetNodesNetwork<usize> {
        GetNodesNetwork::from_nodes([
            SelectableNode::new(0, "input_a", NodeKind::PrimaryInput).with_short_name("a"),
            SelectableNode::new(1, "input_b", NodeKind::PrimaryInput).with_short_name("b"),
            SelectableNode::new(2, "and_node", NodeKind::Internal)
                .with_short_name("n")
                .with_fanins([0, 1])
                .with_fanouts([3, 4]),
            SelectableNode::new(3, "output_y", NodeKind::PrimaryOutput)
                .with_short_name("y")
                .with_fanins([2]),
            SelectableNode::new(4, "output_z", NodeKind::PrimaryOutput)
                .with_short_name("z")
                .with_fanins([2]),
        ])
    }

    #[test]
    fn empty_selector_list_returns_all_nodes() {
        let selection =
            sample_network().select_nodes(std::iter::empty::<&str>(), NodeNameMode::Long);

        assert_eq!(selection.nodes, vec![0, 1, 2, 3, 4]);
        assert!(selection.is_clean());
    }

    #[test]
    fn star_selector_preserves_duplicate_all_node_expansion() {
        let selection = sample_network().select_nodes(["*", "*"], NodeNameMode::Long);

        assert_eq!(selection.nodes, vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]);
    }

    #[test]
    fn empty_fanin_and_fanout_expressions_select_primary_io_boundaries() {
        let network = sample_network();

        assert_eq!(
            network.select_nodes(["i()"], NodeNameMode::Long).nodes,
            vec![0, 1]
        );
        assert_eq!(
            network.select_nodes(["o()"], NodeNameMode::Long).nodes,
            vec![2, 2]
        );
    }

    #[test]
    fn named_fanin_and_fanout_expressions_resolve_by_configured_name_mode() {
        let network = sample_network();

        assert_eq!(
            network
                .select_nodes(["i(and_node)"], NodeNameMode::Long)
                .nodes,
            vec![0, 1]
        );
        assert_eq!(
            network.select_nodes(["o(n)"], NodeNameMode::Short).nodes,
            vec![3, 4]
        );
    }

    #[test]
    fn direct_primary_output_name_resolves_to_its_fanin() {
        let network = sample_network();

        assert_eq!(
            network.select_nodes(["output_y"], NodeNameMode::Long).nodes,
            vec![2]
        );
        assert_eq!(
            network.select_nodes(["y"], NodeNameMode::Short).nodes,
            vec![2]
        );
    }

    #[test]
    fn command_argv_skips_command_name() {
        let selection =
            sample_network().select_from_command_argv(["command", "input_a"], NodeNameMode::Long);

        assert_eq!(selection.nodes, vec![0]);
    }

    #[test]
    fn missing_nodes_are_reported_as_diagnostics_without_stopping_selection() {
        let selection = sample_network().select_nodes(["missing", "input_b"], NodeNameMode::Long);

        assert_eq!(selection.nodes, vec![1]);
        assert_eq!(
            selection.diagnostics,
            vec![SelectionDiagnostic::node_not_found("missing")]
        );
    }

    #[test]
    fn selector_argument_matches_parenthesized_selector_forms() {
        assert_eq!(selector_argument("i()"), "");
        assert_eq!(selector_argument("o(and_node)"), "and_node");
        assert_eq!(selector_argument("plain"), "");
    }
}
