use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrueNodeNameMode {
    Long,
    Short,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrueNodeKind {
    Internal,
    PrimaryInput,
    PrimaryOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrueNodeDiagnostic {
    pub message: String,
}

impl TrueNodeDiagnostic {
    pub fn not_found(name: impl Into<String>) -> Self {
        Self {
            message: format!("'{}' not found", name.into()),
        }
    }
}

impl fmt::Display for TrueNodeDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrueNodeSelection<NodeId> {
    pub nodes: Vec<NodeId>,
    pub diagnostics: Vec<TrueNodeDiagnostic>,
}

impl<NodeId> Default for TrueNodeSelection<NodeId> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl<NodeId> TrueNodeSelection<NodeId> {
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrueSelectableNode<NodeId> {
    pub id: NodeId,
    pub long_name: String,
    pub short_name: String,
    pub kind: TrueNodeKind,
    pub fanins: Vec<NodeId>,
    pub fanouts: Vec<NodeId>,
    pub is_real_primary_input: bool,
    pub is_real_primary_output: bool,
}

impl<NodeId> TrueSelectableNode<NodeId> {
    pub fn new(id: NodeId, long_name: impl Into<String>, kind: TrueNodeKind) -> Self
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
            is_real_primary_input: kind == TrueNodeKind::PrimaryInput,
            is_real_primary_output: kind == TrueNodeKind::PrimaryOutput,
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

    pub fn with_real_primary_input(mut self, is_real: bool) -> Self {
        self.is_real_primary_input = is_real;
        self
    }

    pub fn with_real_primary_output(mut self, is_real: bool) -> Self {
        self.is_real_primary_output = is_real;
        self
    }
}

#[derive(Clone, Debug)]
pub struct TrueNodeNetwork<NodeId> {
    nodes: Vec<TrueSelectableNode<NodeId>>,
    by_long_name: HashMap<String, usize>,
    by_short_name: HashMap<String, usize>,
}

impl<NodeId> Default for TrueNodeNetwork<NodeId> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            by_long_name: HashMap::new(),
            by_short_name: HashMap::new(),
        }
    }
}

impl<NodeId> TrueNodeNetwork<NodeId>
where
    NodeId: Clone + Eq + Hash,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_nodes<I>(nodes: I) -> Self
    where
        I: IntoIterator<Item = TrueSelectableNode<NodeId>>,
    {
        let mut network = Self::new();
        for node in nodes {
            network.add_node(node);
        }

        network
    }

    pub fn add_node(&mut self, node: TrueSelectableNode<NodeId>) {
        let index = self.nodes.len();
        self.by_long_name.insert(node.long_name.clone(), index);
        self.by_short_name.insert(node.short_name.clone(), index);
        self.nodes.push(node);
    }

    pub fn select_from_command_argv<I, S>(
        &self,
        argv: I,
        name_mode: TrueNodeNameMode,
    ) -> TrueNodeSelection<NodeId>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let selectors = argv
            .into_iter()
            .skip(1)
            .map(|arg| arg.as_ref().to_owned())
            .collect::<Vec<_>>();

        self.select_true_nodes(selectors, name_mode)
    }

    pub fn select_true_nodes<I, S>(
        &self,
        selectors: I,
        name_mode: TrueNodeNameMode,
    ) -> TrueNodeSelection<NodeId>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.select_nodes(selectors, name_mode, false)
    }

    pub fn select_true_io_nodes<I, S>(
        &self,
        selectors: I,
        name_mode: TrueNodeNameMode,
    ) -> TrueNodeSelection<NodeId>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.select_nodes(selectors, name_mode, true)
    }

    fn select_nodes<I, S>(
        &self,
        selectors: I,
        name_mode: TrueNodeNameMode,
        real_io_only: bool,
    ) -> TrueNodeSelection<NodeId>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let selectors = selectors
            .into_iter()
            .map(|selector| selector.as_ref().to_owned())
            .collect::<Vec<_>>();

        let mut selection = TrueNodeSelection::default();
        if selectors.is_empty() {
            self.append_all_nodes(&mut selection.nodes);
            return selection;
        }

        for selector in selectors {
            if selector == "*" {
                self.append_all_nodes(&mut selection.nodes);
            } else if selector.starts_with("i(") {
                self.select_fanin_expression(&selector, name_mode, real_io_only, &mut selection);
            } else if selector.starts_with("o(") {
                self.select_fanout_expression(&selector, name_mode, real_io_only, &mut selection);
            } else {
                self.select_named_node(&selector, name_mode, &mut selection);
            }
        }

        selection
    }

    fn append_all_nodes(&self, nodes: &mut Vec<NodeId>) {
        nodes.extend(self.nodes.iter().map(|node| node.id.clone()));
    }

    fn select_fanin_expression(
        &self,
        selector: &str,
        name_mode: TrueNodeNameMode,
        real_io_only: bool,
        selection: &mut TrueNodeSelection<NodeId>,
    ) {
        let name = selector_argument(selector);
        if name.is_empty() {
            let inputs = self.nodes.iter().filter(|node| {
                node.kind == TrueNodeKind::PrimaryInput
                    && (!real_io_only || node.is_real_primary_input)
            });
            selection.nodes.extend(inputs.map(|node| node.id.clone()));
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
        name_mode: TrueNodeNameMode,
        real_io_only: bool,
        selection: &mut TrueNodeSelection<NodeId>,
    ) {
        let name = selector_argument(selector);
        if name.is_empty() {
            let outputs = self.nodes.iter().filter(|node| {
                node.kind == TrueNodeKind::PrimaryOutput
                    && (!real_io_only || node.is_real_primary_output)
            });
            selection.nodes.extend(outputs.map(|node| node.id.clone()));
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
        name_mode: TrueNodeNameMode,
        selection: &mut TrueNodeSelection<NodeId>,
    ) {
        if let Some(node) = self.node_by_name(name, name_mode, selection) {
            selection.nodes.push(node.id.clone());
        }
    }

    fn node_by_name(
        &self,
        name: &str,
        name_mode: TrueNodeNameMode,
        selection: &mut TrueNodeSelection<NodeId>,
    ) -> Option<&TrueSelectableNode<NodeId>> {
        let table = match name_mode {
            TrueNodeNameMode::Long => &self.by_long_name,
            TrueNodeNameMode::Short => &self.by_short_name,
        };

        let Some(index) = table.get(name).copied() else {
            selection
                .diagnostics
                .push(TrueNodeDiagnostic::not_found(name));
            return None;
        };

        Some(&self.nodes[index])
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

    fn sample_network() -> TrueNodeNetwork<usize> {
        TrueNodeNetwork::from_nodes([
            TrueSelectableNode::new(0, "input_a", TrueNodeKind::PrimaryInput).with_short_name("a"),
            TrueSelectableNode::new(1, "latch_input", TrueNodeKind::PrimaryInput)
                .with_short_name("li")
                .with_real_primary_input(false),
            TrueSelectableNode::new(2, "and_node", TrueNodeKind::Internal)
                .with_short_name("n")
                .with_fanins([0, 1])
                .with_fanouts([3, 4]),
            TrueSelectableNode::new(3, "output_y", TrueNodeKind::PrimaryOutput)
                .with_short_name("y")
                .with_fanins([2]),
            TrueSelectableNode::new(4, "latch_output", TrueNodeKind::PrimaryOutput)
                .with_short_name("lo")
                .with_fanins([2])
                .with_real_primary_output(false),
        ])
    }

    #[test]
    fn empty_selector_list_returns_all_nodes() {
        let selection =
            sample_network().select_true_nodes(std::iter::empty::<&str>(), TrueNodeNameMode::Long);

        assert_eq!(selection.nodes, vec![0, 1, 2, 3, 4]);
        assert!(selection.is_clean());
    }

    #[test]
    fn star_selector_preserves_duplicate_all_node_expansion() {
        let selection = sample_network().select_true_nodes(["*", "*"], TrueNodeNameMode::Long);

        assert_eq!(selection.nodes, vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]);
    }

    #[test]
    fn empty_fanin_and_fanout_expressions_select_primary_io_nodes() {
        let network = sample_network();

        assert_eq!(
            network
                .select_true_nodes(["i()"], TrueNodeNameMode::Long)
                .nodes,
            vec![0, 1]
        );
        assert_eq!(
            network
                .select_true_nodes(["o()"], TrueNodeNameMode::Long)
                .nodes,
            vec![3, 4]
        );
    }

    #[test]
    fn true_io_variant_filters_latch_boundary_nodes_only_for_empty_io_selectors() {
        let network = sample_network();

        assert_eq!(
            network
                .select_true_io_nodes(["i()", "o()"], TrueNodeNameMode::Long)
                .nodes,
            vec![0, 3]
        );
        assert_eq!(
            network
                .select_true_io_nodes(["i(and_node)", "o(and_node)"], TrueNodeNameMode::Long)
                .nodes,
            vec![0, 1, 3, 4]
        );
    }

    #[test]
    fn named_fanin_and_fanout_expressions_resolve_by_configured_name_mode() {
        let network = sample_network();

        assert_eq!(
            network
                .select_true_nodes(["i(and_node)"], TrueNodeNameMode::Long)
                .nodes,
            vec![0, 1]
        );
        assert_eq!(
            network
                .select_true_nodes(["o(n)"], TrueNodeNameMode::Short)
                .nodes,
            vec![3, 4]
        );
    }

    #[test]
    fn direct_primary_output_name_returns_the_primary_output_node() {
        let network = sample_network();

        assert_eq!(
            network
                .select_true_nodes(["output_y"], TrueNodeNameMode::Long)
                .nodes,
            vec![3]
        );
        assert_eq!(
            network
                .select_true_nodes(["y"], TrueNodeNameMode::Short)
                .nodes,
            vec![3]
        );
    }

    #[test]
    fn command_argv_skips_command_name() {
        let selection = sample_network()
            .select_from_command_argv(["command", "input_a"], TrueNodeNameMode::Long);

        assert_eq!(selection.nodes, vec![0]);
    }

    #[test]
    fn missing_nodes_are_reported_as_legacy_diagnostics_without_stopping_selection() {
        let selection =
            sample_network().select_true_nodes(["missing", "input_a"], TrueNodeNameMode::Long);

        assert_eq!(selection.nodes, vec![0]);
        assert_eq!(
            selection.diagnostics,
            vec![TrueNodeDiagnostic::not_found("missing")]
        );
    }

    #[test]
    fn selector_argument_matches_parenthesized_selector_forms() {
        assert_eq!(selector_argument("i()"), "");
        assert_eq!(selector_argument("o(and_node)"), "and_node");
        assert_eq!(selector_argument("i(and_node"), "and_node");
        assert_eq!(selector_argument("plain"), "");
    }
}
