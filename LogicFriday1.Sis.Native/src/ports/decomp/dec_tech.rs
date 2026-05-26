use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TechNodeKind
{
    Internal,
    Input,
    Output,
    Constant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TechNodeFunction
{
    And,
    Or,
    Complex,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase
{
    Positive,
    Negative,
    Binate,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase
{
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateKind
{
    And,
    Or,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TechDecompositionSummary
{
    pub visited_internal_nodes: usize,
    pub constant_or_replacements: usize,
    pub complex_replacements: usize,
    pub inverted_nodes: usize,
    pub decomposed_nodes: usize,
    pub added_nodes: usize,
    pub swept: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TechDecompositionError
{
    InvalidFaninLimit
    {
        gate: GateKind,
    },
    UnexpectedFunction
    {
        expected: TechNodeFunction,
        actual: TechNodeFunction,
    },
    InvalidInputPhase
    {
        phase: InputPhase,
    },
    MissingRoot,
    Backend
    {
        message: String,
    },
}

impl TechDecompositionError
{
    pub fn backend(message: impl Into<String>) -> Self
    {
        Self::Backend {
            message: message.into(),
        }
    }
}

impl fmt::Display for TechDecompositionError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::InvalidFaninLimit { gate } => {
                write!(f, "wrong fanin limit for {gate:?} gate")
            }
            Self::UnexpectedFunction { expected, actual } => {
                write!(f, "expected {expected:?} function, found {actual:?}")
            }
            Self::InvalidInputPhase { phase } => {
                write!(f, "unsupported input phase {phase:?}")
            }
            Self::MissingRoot => f.write_str("balanced tree has no root"),
            Self::Backend { message } => f.write_str(message),
        }
    }
}

impl Error for TechDecompositionError {}

pub type TechDecompositionResult<T> = Result<T, TechDecompositionError>;

pub trait TechDecompositionNetwork
{
    type Node: Clone;
    type NodeId: Clone;

    fn dfs_snapshot(&self) -> Vec<Self::NodeId>;

    fn node_kind(&self, node: &Self::NodeId) -> TechDecompositionResult<TechNodeKind>;

    fn node_function(&self, node: &Self::NodeId) -> TechDecompositionResult<TechNodeFunction>;

    fn fanin_count(&self, node: &Self::NodeId) -> TechDecompositionResult<usize>;

    fn fanin(&self, node: &Self::NodeId, index: usize) -> TechDecompositionResult<Self::NodeId>;

    fn input_phase(
        &self,
        node: &Self::NodeId,
        fanin: &Self::NodeId,
    ) -> TechDecompositionResult<InputPhase>;

    fn cube_count(&self, node: &Self::NodeId) -> TechDecompositionResult<usize>;

    fn cube_as_node(&self, node: &Self::NodeId, cube: usize) -> TechDecompositionResult<Self::Node>;

    fn constant_node(&self, value: bool) -> TechDecompositionResult<Self::Node>;

    fn literal_node(
        &self,
        fanin: &Self::NodeId,
        phase: LiteralPhase,
    ) -> TechDecompositionResult<Self::Node>;

    fn literal_of_node(
        &self,
        node: Self::Node,
        phase: LiteralPhase,
    ) -> TechDecompositionResult<Self::Node>;

    fn gate_node(
        &self,
        gate: GateKind,
        fanins: Vec<Self::Node>,
    ) -> TechDecompositionResult<Self::Node>;

    fn replace_node(
        &mut self,
        node: &Self::NodeId,
        replacement: Self::Node,
    ) -> TechDecompositionResult<()>;

    fn add_node(&mut self, node: Self::Node) -> TechDecompositionResult<Self::NodeId>;

    fn invert_node(&mut self, node: &Self::NodeId) -> TechDecompositionResult<()>;

    fn sweep(&mut self) -> TechDecompositionResult<()>;
}

pub fn decompose_technology_network<N>(
    network: &mut N,
    and_limit: usize,
    or_limit: usize,
) -> TechDecompositionResult<TechDecompositionSummary>
where
    N: TechDecompositionNetwork,
{
    let mut summary = TechDecompositionSummary::default();

    replace_binate_or_nodes_with_one(network, &mut summary)?;
    decompose_complex_nodes(network, &mut summary)?;
    invert_unsupported_single_gate_nodes(network, and_limit, or_limit, &mut summary)?;
    decompose_single_gate_nodes(network, and_limit, or_limit, &mut summary)?;
    network.sweep()?;
    summary.swept = true;

    Ok(summary)
}

fn replace_binate_or_nodes_with_one<N>(
    network: &mut N,
    summary: &mut TechDecompositionSummary,
) -> TechDecompositionResult<()>
where
    N: TechDecompositionNetwork,
{
    for node in internal_nodes_in_forward_order(network)?
    {
        summary.visited_internal_nodes += 1;

        if network.node_function(&node)? != TechNodeFunction::Or
        {
            continue;
        }

        let fanin_count = network.fanin_count(&node)?;
        let mut has_binate_input = false;

        for index in 0..fanin_count
        {
            let fanin = network.fanin(&node, index)?;

            if network.input_phase(&node, &fanin)? == InputPhase::Binate
            {
                has_binate_input = true;
                break;
            }
        }

        if has_binate_input
        {
            let one = network.constant_node(true)?;
            network.replace_node(&node, one)?;
            summary.constant_or_replacements += 1;
        }
    }

    Ok(())
}

fn decompose_complex_nodes<N>(
    network: &mut N,
    summary: &mut TechDecompositionSummary,
) -> TechDecompositionResult<()>
where
    N: TechDecompositionNetwork,
{
    for node in internal_nodes_in_reverse_dfs_order(network)?
    {
        if network.node_function(&node)? == TechNodeFunction::Complex
        {
            decompose_and_or(network, &node, summary)?;
        }
    }

    Ok(())
}

fn invert_unsupported_single_gate_nodes<N>(
    network: &mut N,
    and_limit: usize,
    or_limit: usize,
    summary: &mut TechDecompositionSummary,
) -> TechDecompositionResult<()>
where
    N: TechDecompositionNetwork,
{
    for node in internal_nodes_in_reverse_dfs_order(network)?
    {
        match network.node_function(&node)?
        {
            TechNodeFunction::Or if or_limit == 0 => {
                network.invert_node(&node)?;
                summary.inverted_nodes += 1;
            }
            TechNodeFunction::And if and_limit == 0 => {
                network.invert_node(&node)?;
                summary.inverted_nodes += 1;
            }
            _ => {}
        }
    }

    Ok(())
}

fn decompose_single_gate_nodes<N>(
    network: &mut N,
    and_limit: usize,
    or_limit: usize,
    summary: &mut TechDecompositionSummary,
) -> TechDecompositionResult<()>
where
    N: TechDecompositionNetwork,
{
    for node in internal_nodes_in_reverse_dfs_order(network)?
    {
        match network.node_function(&node)?
        {
            TechNodeFunction::And => decompose_gate(network, &node, GateKind::And, and_limit, summary)?,
            TechNodeFunction::Or => decompose_gate(network, &node, GateKind::Or, or_limit, summary)?,
            _ => {}
        }
    }

    Ok(())
}

fn decompose_and_or<N>(
    network: &mut N,
    node: &N::NodeId,
    summary: &mut TechDecompositionSummary,
) -> TechDecompositionResult<()>
where
    N: TechDecompositionNetwork,
{
    let mut root = network.constant_node(false)?;

    for cube in (0..network.cube_count(node)?).rev()
    {
        let cube_node = network.cube_as_node(node, cube)?;
        let cube_node_id = network.add_node(cube_node.clone())?;
        let cube_literal = network.literal_node(&cube_node_id, LiteralPhase::Positive)?;
        root = network.gate_node(GateKind::Or, vec![root, cube_literal])?;
        summary.added_nodes += 1;
    }

    network.replace_node(node, root)?;
    summary.complex_replacements += 1;

    Ok(())
}

fn decompose_gate<N>(
    network: &mut N,
    node: &N::NodeId,
    gate: GateKind,
    limit: usize,
    summary: &mut TechDecompositionSummary,
) -> TechDecompositionResult<()>
where
    N: TechDecompositionNetwork,
{
    if limit == 0
    {
        return Err(TechDecompositionError::InvalidFaninLimit { gate });
    }

    let expected = match gate
    {
        GateKind::And => TechNodeFunction::And,
        GateKind::Or => TechNodeFunction::Or,
    };
    let actual = network.node_function(node)?;

    if actual != expected
    {
        return Err(TechDecompositionError::UnexpectedFunction { expected, actual });
    }

    let leaves = gate_literals(network, node)?;
    let mut added_nodes = Vec::new();
    let root = balanced_tree(network, gate, limit, leaves, &mut added_nodes)?;
    network.replace_node(node, root)?;

    for added_node in added_nodes
    {
        network.add_node(added_node)?;
        summary.added_nodes += 1;
    }

    summary.decomposed_nodes += 1;

    Ok(())
}

fn gate_literals<N>(network: &N, node: &N::NodeId) -> TechDecompositionResult<Vec<N::Node>>
where
    N: TechDecompositionNetwork,
{
    let fanin_count = network.fanin_count(node)?;
    let mut literals = Vec::with_capacity(fanin_count);

    for index in 0..fanin_count
    {
        let fanin = network.fanin(node, index)?;
        let phase = match network.input_phase(node, &fanin)?
        {
            InputPhase::Positive => LiteralPhase::Positive,
            InputPhase::Negative => LiteralPhase::Negative,
            phase => return Err(TechDecompositionError::InvalidInputPhase { phase }),
        };

        literals.push(network.literal_node(&fanin, phase)?);
    }

    Ok(literals)
}

fn balanced_tree<N>(
    network: &N,
    gate: GateKind,
    limit: usize,
    leaves: Vec<N::Node>,
    added_nodes: &mut Vec<N::Node>,
) -> TechDecompositionResult<N::Node>
where
    N: TechDecompositionNetwork,
{
    if leaves.is_empty()
    {
        return Err(TechDecompositionError::MissingRoot);
    }

    if leaves.len() == 1
    {
        return Ok(leaves.into_iter().next().ok_or(TechDecompositionError::MissingRoot)?);
    }

    if leaves.len() <= limit
    {
        return network.gate_node(gate, leaves);
    }

    let branch_count = leaves.len().div_ceil(limit);
    let mut branches = Vec::with_capacity(branch_count);
    let mut remaining = leaves;

    while !remaining.is_empty()
    {
        let take = remaining.len().min(branch_count);
        let tail = remaining.split_off(take);
        let branch = balanced_tree(network, gate, limit, remaining, added_nodes)?;
        added_nodes.push(branch.clone());
        branches.push(network.literal_of_node(branch, LiteralPhase::Positive)?);
        remaining = tail;
    }

    balanced_tree(network, gate, limit, branches, added_nodes)
}

fn internal_nodes_in_forward_order<N>(network: &N) -> TechDecompositionResult<Vec<N::NodeId>>
where
    N: TechDecompositionNetwork,
{
    network
        .dfs_snapshot()
        .into_iter()
        .filter_map(|node| match network.node_kind(&node)
        {
            Ok(TechNodeKind::Internal) => Some(Ok(node)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn internal_nodes_in_reverse_dfs_order<N>(network: &N) -> TechDecompositionResult<Vec<N::NodeId>>
where
    N: TechDecompositionNetwork,
{
    let mut nodes = internal_nodes_in_forward_order(network)?;
    nodes.reverse();
    Ok(nodes)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum TestExpression
    {
        Constant(bool),
        Literal(String, LiteralPhase),
        Gate(GateKind, Vec<TestExpression>),
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNode
    {
        name: String,
        kind: TechNodeKind,
        function: TechNodeFunction,
        fanins: Vec<String>,
        phases: Vec<InputPhase>,
        cubes: Vec<Vec<(String, LiteralPhase)>>,
        expression: Option<TestExpression>,
    }

    impl TestNode
    {
        fn input(name: &str) -> Self
        {
            Self {
                name: name.to_owned(),
                kind: TechNodeKind::Input,
                function: TechNodeFunction::Other,
                fanins: Vec::new(),
                phases: Vec::new(),
                cubes: Vec::new(),
                expression: None,
            }
        }

        fn gate(
            name: &str,
            function: TechNodeFunction,
            fanins: &[&str],
            phases: &[InputPhase],
        ) -> Self
        {
            Self {
                name: name.to_owned(),
                kind: TechNodeKind::Internal,
                function,
                fanins: fanins.iter().map(|fanin| (*fanin).to_owned()).collect(),
                phases: phases.to_vec(),
                cubes: Vec::new(),
                expression: None,
            }
        }

        fn complex(name: &str, cubes: Vec<Vec<(&str, LiteralPhase)>>) -> Self
        {
            Self {
                name: name.to_owned(),
                kind: TechNodeKind::Internal,
                function: TechNodeFunction::Complex,
                fanins: Vec::new(),
                phases: Vec::new(),
                cubes: cubes
                    .into_iter()
                    .map(|cube| {
                        cube.into_iter()
                            .map(|(name, phase)| (name.to_owned(), phase))
                            .collect()
                    })
                    .collect(),
                expression: None,
            }
        }
    }

    #[derive(Default)]
    struct TestNetwork
    {
        nodes: Vec<TestNode>,
        swept: bool,
    }

    impl TestNetwork
    {
        fn new(nodes: Vec<TestNode>) -> Self
        {
            Self {
                nodes,
                swept: false,
            }
        }

        fn node(&self, name: &str) -> &TestNode
        {
            self.nodes.iter().find(|node| node.name == name).unwrap()
        }
    }

    impl TechDecompositionNetwork for TestNetwork
    {
        type Node = TestNode;
        type NodeId = String;

        fn dfs_snapshot(&self) -> Vec<Self::NodeId>
        {
            self.nodes.iter().map(|node| node.name.clone()).collect()
        }

        fn node_kind(&self, node: &Self::NodeId) -> TechDecompositionResult<TechNodeKind>
        {
            Ok(self.node(node).kind)
        }

        fn node_function(&self, node: &Self::NodeId) -> TechDecompositionResult<TechNodeFunction>
        {
            Ok(self.node(node).function)
        }

        fn fanin_count(&self, node: &Self::NodeId) -> TechDecompositionResult<usize>
        {
            Ok(self.node(node).fanins.len())
        }

        fn fanin(&self, node: &Self::NodeId, index: usize) -> TechDecompositionResult<Self::NodeId>
        {
            self.node(node)
                .fanins
                .get(index)
                .cloned()
                .ok_or_else(|| TechDecompositionError::backend("fanin index is outside the node"))
        }

        fn input_phase(
            &self,
            node: &Self::NodeId,
            fanin: &Self::NodeId,
        ) -> TechDecompositionResult<InputPhase>
        {
            let node = self.node(node);
            let index = node
                .fanins
                .iter()
                .position(|candidate| candidate == fanin)
                .ok_or_else(|| TechDecompositionError::backend("fanin is not connected to node"))?;

            Ok(node.phases[index])
        }

        fn cube_count(&self, node: &Self::NodeId) -> TechDecompositionResult<usize>
        {
            Ok(self.node(node).cubes.len())
        }

        fn cube_as_node(
            &self,
            node: &Self::NodeId,
            cube: usize,
        ) -> TechDecompositionResult<Self::Node>
        {
            let source = self.node(node);
            let cube_data = source
                .cubes
                .get(cube)
                .ok_or_else(|| TechDecompositionError::backend("cube index is outside the node"))?;

            Ok(TestNode {
                name: format!("{}_cube_{cube}", source.name),
                kind: TechNodeKind::Internal,
                function: TechNodeFunction::And,
                fanins: cube_data.iter().map(|(fanin, _)| fanin.clone()).collect(),
                phases: cube_data
                    .iter()
                    .map(|(_, phase)| match phase
                    {
                        LiteralPhase::Positive => InputPhase::Positive,
                        LiteralPhase::Negative => InputPhase::Negative,
                    })
                    .collect(),
                cubes: Vec::new(),
                expression: Some(TestExpression::Gate(
                    GateKind::And,
                    cube_data.iter()
                        .map(|(fanin, phase)| TestExpression::Literal(fanin.clone(), *phase))
                        .collect(),
                )),
            })
        }

        fn constant_node(&self, value: bool) -> TechDecompositionResult<Self::Node>
        {
            Ok(TestNode {
                name: if value { "one" } else { "zero" }.to_owned(),
                kind: TechNodeKind::Constant,
                function: TechNodeFunction::Other,
                fanins: Vec::new(),
                phases: Vec::new(),
                cubes: Vec::new(),
                expression: Some(TestExpression::Constant(value)),
            })
        }

        fn literal_node(
            &self,
            fanin: &Self::NodeId,
            phase: LiteralPhase,
        ) -> TechDecompositionResult<Self::Node>
        {
            Ok(TestNode {
                name: format!("{fanin}_literal"),
                kind: TechNodeKind::Internal,
                function: TechNodeFunction::Other,
                fanins: vec![fanin.clone()],
                phases: vec![match phase
                {
                    LiteralPhase::Positive => InputPhase::Positive,
                    LiteralPhase::Negative => InputPhase::Negative,
                }],
                cubes: Vec::new(),
                expression: Some(TestExpression::Literal(fanin.clone(), phase)),
            })
        }

        fn literal_of_node(
            &self,
            node: Self::Node,
            phase: LiteralPhase,
        ) -> TechDecompositionResult<Self::Node>
        {
            Ok(TestNode {
                name: format!("{}_literal", node.name),
                kind: TechNodeKind::Internal,
                function: TechNodeFunction::Other,
                fanins: vec![node.name],
                phases: vec![match phase
                {
                    LiteralPhase::Positive => InputPhase::Positive,
                    LiteralPhase::Negative => InputPhase::Negative,
                }],
                cubes: Vec::new(),
                expression: Some(TestExpression::Literal(
                    format!("{:?}", node.expression),
                    phase,
                )),
            })
        }

        fn gate_node(
            &self,
            gate: GateKind,
            fanins: Vec<Self::Node>,
        ) -> TechDecompositionResult<Self::Node>
        {
            Ok(TestNode {
                name: format!("{gate:?}_{}", fanins.len()),
                kind: TechNodeKind::Internal,
                function: match gate
                {
                    GateKind::And => TechNodeFunction::And,
                    GateKind::Or => TechNodeFunction::Or,
                },
                fanins: fanins.iter().map(|fanin| fanin.name.clone()).collect(),
                phases: vec![InputPhase::Positive; fanins.len()],
                cubes: Vec::new(),
                expression: Some(TestExpression::Gate(
                    gate,
                    fanins
                        .into_iter()
                        .filter_map(|fanin| fanin.expression)
                        .collect(),
                )),
            })
        }

        fn replace_node(
            &mut self,
            node: &Self::NodeId,
            replacement: Self::Node,
        ) -> TechDecompositionResult<()>
        {
            let index = self
                .nodes
                .iter()
                .position(|candidate| &candidate.name == node)
                .ok_or_else(|| TechDecompositionError::backend("node is outside the network"))?;

            self.nodes[index] = TestNode {
                name: node.clone(),
                ..replacement
            };

            Ok(())
        }

        fn add_node(&mut self, mut node: Self::Node) -> TechDecompositionResult<Self::NodeId>
        {
            let id = format!("{}_{}", node.name, self.nodes.len());
            node.name = id.clone();
            self.nodes.push(node);
            Ok(id)
        }

        fn invert_node(&mut self, node: &Self::NodeId) -> TechDecompositionResult<()>
        {
            let target = self
                .nodes
                .iter_mut()
                .find(|candidate| &candidate.name == node)
                .ok_or_else(|| TechDecompositionError::backend("node is outside the network"))?;

            target.function = match target.function
            {
                TechNodeFunction::And => TechNodeFunction::Or,
                TechNodeFunction::Or => TechNodeFunction::And,
                other => other,
            };

            Ok(())
        }

        fn sweep(&mut self) -> TechDecompositionResult<()>
        {
            self.swept = true;
            Ok(())
        }
    }

    #[test]
    fn binate_or_is_replaced_with_constant_one_before_gate_decomposition()
    {
        let mut network = TestNetwork::new(vec![
            TestNode::input("a"),
            TestNode::gate(
                "f",
                TechNodeFunction::Or,
                &["a"],
                &[InputPhase::Binate],
            ),
        ]);

        let summary = decompose_technology_network(&mut network, 2, 2).unwrap();

        assert_eq!(summary.constant_or_replacements, 1);
        assert_eq!(network.node("f").expression, Some(TestExpression::Constant(true)));
        assert!(summary.swept);
        assert!(network.swept);
    }

    #[test]
    fn complex_node_becomes_or_of_product_cubes()
    {
        let mut network = TestNetwork::new(vec![TestNode::complex(
            "f",
            vec![
                vec![("a", LiteralPhase::Positive), ("b", LiteralPhase::Negative)],
                vec![("c", LiteralPhase::Positive)],
            ],
        )]);

        let summary = decompose_technology_network(&mut network, 4, 4).unwrap();

        assert_eq!(summary.complex_replacements, 1);
        assert_eq!(summary.added_nodes, 2);
        assert_eq!(network.nodes.len(), 3);
        assert!(matches!(
            network.node("f").expression,
            Some(TestExpression::Gate(GateKind::Or, _))
        ));
    }

    #[test]
    fn and_gate_is_rebuilt_with_bounded_fanin_tree()
    {
        let mut network = TestNetwork::new(vec![TestNode::gate(
            "f",
            TechNodeFunction::And,
            &["a", "b", "c", "d", "e"],
            &[
                InputPhase::Positive,
                InputPhase::Negative,
                InputPhase::Positive,
                InputPhase::Positive,
                InputPhase::Negative,
            ],
        )]);

        let summary = decompose_technology_network(&mut network, 2, 4).unwrap();

        assert_eq!(summary.decomposed_nodes, 1);
        assert!(summary.added_nodes >= 2);
        assert!(matches!(
            network.node("f").expression,
            Some(TestExpression::Gate(GateKind::And, _))
        ));
    }

    #[test]
    fn zero_limit_inverts_matching_gate_kind()
    {
        let mut network = TestNetwork::new(vec![TestNode::gate(
            "f",
            TechNodeFunction::Or,
            &["a", "b"],
            &[InputPhase::Positive, InputPhase::Positive],
        )]);

        let summary = decompose_technology_network(&mut network, 2, 0).unwrap();

        assert_eq!(summary.inverted_nodes, 1);
        assert_eq!(network.node("f").function, TechNodeFunction::And);
    }

    #[test]
    fn binate_phase_is_rejected_for_plain_gate_decomposition()
    {
        let mut network = TestNetwork::new(vec![TestNode::gate(
            "f",
            TechNodeFunction::And,
            &["a"],
            &[InputPhase::Binate],
        )]);

        let error = decompose_technology_network(&mut network, 2, 2).unwrap_err();

        assert_eq!(
            error,
            TechDecompositionError::InvalidInputPhase {
                phase: InputPhase::Binate
            }
        );
    }
}
