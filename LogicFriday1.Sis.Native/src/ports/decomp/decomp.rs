use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    Internal,
    Input,
    Output,
    Constant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecompositionMethod {
    Quick,
    Good,
    Disjoint,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NetworkDecompositionSummary {
    pub visited_internal_nodes: usize,
    pub replaced_nodes: usize,
    pub added_nodes: usize,
    pub unchanged_internal_nodes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecompositionError {
    EmptyResult { method: DecompositionMethod },
    SubstitutionRejected,
    Backend { message: String },
}

impl DecompositionError {
    pub fn backend(message: impl Into<String>) -> Self {
        Self::Backend {
            message: message.into(),
        }
    }
}

impl fmt::Display for DecompositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyResult { method } => write!(f, "{method:?} decomposition produced no nodes"),
            Self::SubstitutionRejected => {
                write!(f, "decomposition divisor could not be substituted")
            }
            Self::Backend { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for DecompositionError {}

pub type DecompositionResult<T> = Result<T, DecompositionError>;

pub trait KernelDecomposition {
    type Node: Clone;

    fn normalize_support(&mut self, node: &mut Self::Node) -> DecompositionResult<()>;

    fn quick_divisor(&mut self, node: &Self::Node) -> DecompositionResult<Option<Self::Node>>;

    fn good_divisor(&mut self, node: &Self::Node) -> DecompositionResult<Option<Self::Node>>;

    fn substitute_divisor(
        &mut self,
        node: &mut Self::Node,
        divisor: &Self::Node,
    ) -> DecompositionResult<bool>;

    fn disjoint_decomposition(&mut self, node: &Self::Node)
    -> DecompositionResult<Vec<Self::Node>>;
}

pub trait NetworkDecomposition {
    type Node: Clone;
    type NodeId: Clone;

    fn node_ids_snapshot(&self) -> Vec<Self::NodeId>;

    fn node_kind(&self, node: &Self::NodeId) -> DecompositionResult<NodeKind>;

    fn clone_node(&self, node: &Self::NodeId) -> DecompositionResult<Self::Node>;

    fn replace_node(
        &mut self,
        node: &Self::NodeId,
        replacement: Self::Node,
    ) -> DecompositionResult<()>;

    fn add_node(&mut self, node: Self::Node) -> DecompositionResult<Self::NodeId>;
}

pub fn decompose_quick_node<A>(algebra: &mut A, node: &A::Node) -> DecompositionResult<Vec<A::Node>>
where
    A: KernelDecomposition,
{
    decompose_with_kernel(algebra, node, DecompositionMethod::Quick)
}

pub fn decompose_good_node<A>(algebra: &mut A, node: &A::Node) -> DecompositionResult<Vec<A::Node>>
where
    A: KernelDecomposition,
{
    decompose_with_kernel(algebra, node, DecompositionMethod::Good)
}

pub fn decompose_disjoint_node<A>(
    algebra: &mut A,
    node: &A::Node,
) -> DecompositionResult<Vec<A::Node>>
where
    A: KernelDecomposition,
{
    let result = algebra.disjoint_decomposition(node)?;

    if result.is_empty() {
        Err(DecompositionError::EmptyResult {
            method: DecompositionMethod::Disjoint,
        })
    } else {
        Ok(result)
    }
}

pub fn decompose_quick_network<N, A>(
    network: &mut N,
    algebra: &mut A,
) -> DecompositionResult<NetworkDecompositionSummary>
where
    N: NetworkDecomposition<Node = A::Node>,
    A: KernelDecomposition,
{
    decompose_network(network, algebra, DecompositionMethod::Quick)
}

pub fn decompose_good_network<N, A>(
    network: &mut N,
    algebra: &mut A,
) -> DecompositionResult<NetworkDecompositionSummary>
where
    N: NetworkDecomposition<Node = A::Node>,
    A: KernelDecomposition,
{
    decompose_network(network, algebra, DecompositionMethod::Good)
}

pub fn decompose_disjoint_network<N, A>(
    network: &mut N,
    algebra: &mut A,
) -> DecompositionResult<NetworkDecompositionSummary>
where
    N: NetworkDecomposition<Node = A::Node>,
    A: KernelDecomposition,
{
    decompose_network(network, algebra, DecompositionMethod::Disjoint)
}

pub fn decompose_network<N, A>(
    network: &mut N,
    algebra: &mut A,
    method: DecompositionMethod,
) -> DecompositionResult<NetworkDecompositionSummary>
where
    N: NetworkDecomposition<Node = A::Node>,
    A: KernelDecomposition,
{
    let mut summary = NetworkDecompositionSummary::default();
    let node_ids = network.node_ids_snapshot();

    for node_id in node_ids {
        if network.node_kind(&node_id)? != NodeKind::Internal {
            continue;
        }

        summary.visited_internal_nodes += 1;
        let node = network.clone_node(&node_id)?;
        let decomposition = decompose_node_by_method(algebra, &node, method)?;
        apply_node_decomposition(network, &node_id, decomposition, method, &mut summary)?;
    }

    Ok(summary)
}

pub fn decompose_single_network_node<N, A>(
    network: &mut N,
    algebra: &mut A,
    node: &N::NodeId,
    method: DecompositionMethod,
) -> DecompositionResult<NetworkDecompositionSummary>
where
    N: NetworkDecomposition<Node = A::Node>,
    A: KernelDecomposition,
{
    let mut summary = NetworkDecompositionSummary::default();

    if network.node_kind(node)? != NodeKind::Internal {
        return Ok(summary);
    }

    summary.visited_internal_nodes = 1;
    let original = network.clone_node(node)?;
    let decomposition = decompose_node_by_method(algebra, &original, method)?;
    apply_node_decomposition(network, node, decomposition, method, &mut summary)?;

    Ok(summary)
}

fn decompose_with_kernel<A>(
    algebra: &mut A,
    node: &A::Node,
    method: DecompositionMethod,
) -> DecompositionResult<Vec<A::Node>>
where
    A: KernelDecomposition,
{
    let mut root = node.clone();
    algebra.normalize_support(&mut root)?;
    decompose_recur(algebra, root, method)
}

fn decompose_recur<A>(
    algebra: &mut A,
    mut node: A::Node,
    method: DecompositionMethod,
) -> DecompositionResult<Vec<A::Node>>
where
    A: KernelDecomposition,
{
    let divisor = match method {
        DecompositionMethod::Quick => algebra.quick_divisor(&node)?,
        DecompositionMethod::Good => algebra.good_divisor(&node)?,
        DecompositionMethod::Disjoint => None,
    };

    let Some(divisor) = divisor else {
        return Ok(vec![node]);
    };

    if !algebra.substitute_divisor(&mut node, &divisor)? {
        return Err(DecompositionError::SubstitutionRejected);
    }

    let mut remainder_nodes = decompose_recur(algebra, node, method)?;
    let mut divisor_nodes = decompose_recur(algebra, divisor, method)?;
    remainder_nodes.append(&mut divisor_nodes);

    Ok(remainder_nodes)
}

fn decompose_node_by_method<A>(
    algebra: &mut A,
    node: &A::Node,
    method: DecompositionMethod,
) -> DecompositionResult<Vec<A::Node>>
where
    A: KernelDecomposition,
{
    match method {
        DecompositionMethod::Quick => decompose_quick_node(algebra, node),
        DecompositionMethod::Good => decompose_good_node(algebra, node),
        DecompositionMethod::Disjoint => decompose_disjoint_node(algebra, node),
    }
}

fn apply_node_decomposition<N>(
    network: &mut N,
    node_id: &N::NodeId,
    mut decomposition: Vec<N::Node>,
    method: DecompositionMethod,
    summary: &mut NetworkDecompositionSummary,
) -> DecompositionResult<()>
where
    N: NetworkDecomposition,
{
    if decomposition.is_empty() {
        return Err(DecompositionError::EmptyResult { method });
    }

    if decomposition.len() == 1 {
        summary.unchanged_internal_nodes += 1;
        return Ok(());
    }

    let root = decomposition.remove(0);
    network.replace_node(node_id, root)?;
    summary.replaced_nodes += 1;

    for node in decomposition {
        network.add_node(node)?;
        summary.added_nodes += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNode {
        name: String,
        kind: NodeKind,
        quick_divisors: Vec<String>,
        good_divisors: Vec<String>,
        disjoint_result: Option<Vec<TestNode>>,
        normalized: bool,
    }

    impl TestNode {
        fn internal(name: &str) -> Self {
            Self {
                name: name.to_owned(),
                kind: NodeKind::Internal,
                quick_divisors: Vec::new(),
                good_divisors: Vec::new(),
                disjoint_result: None,
                normalized: false,
            }
        }

        fn input(name: &str) -> Self {
            Self {
                name: name.to_owned(),
                kind: NodeKind::Input,
                quick_divisors: Vec::new(),
                good_divisors: Vec::new(),
                disjoint_result: None,
                normalized: false,
            }
        }

        fn with_quick_divisors(mut self, divisors: &[&str]) -> Self {
            self.quick_divisors = divisors.iter().map(|name| (*name).to_owned()).collect();
            self
        }

        fn with_good_divisors(mut self, divisors: &[&str]) -> Self {
            self.good_divisors = divisors.iter().map(|name| (*name).to_owned()).collect();
            self
        }

        fn with_disjoint_result(mut self, result: Vec<TestNode>) -> Self {
            self.disjoint_result = Some(result);
            self
        }
    }

    #[derive(Default)]
    struct TestAlgebra {
        reject_substitution: bool,
    }

    impl KernelDecomposition for TestAlgebra {
        type Node = TestNode;

        fn normalize_support(&mut self, node: &mut Self::Node) -> DecompositionResult<()> {
            node.normalized = true;
            Ok(())
        }

        fn quick_divisor(&mut self, node: &Self::Node) -> DecompositionResult<Option<Self::Node>> {
            Ok(node
                .quick_divisors
                .first()
                .map(|name| TestNode::internal(name)))
        }

        fn good_divisor(&mut self, node: &Self::Node) -> DecompositionResult<Option<Self::Node>> {
            Ok(node
                .good_divisors
                .first()
                .map(|name| TestNode::internal(name)))
        }

        fn substitute_divisor(
            &mut self,
            node: &mut Self::Node,
            divisor: &Self::Node,
        ) -> DecompositionResult<bool> {
            if self.reject_substitution {
                return Ok(false);
            }

            remove_divisor(&mut node.quick_divisors, &divisor.name);
            remove_divisor(&mut node.good_divisors, &divisor.name);
            node.name = format!("{}-without-{}", node.name, divisor.name);

            Ok(true)
        }

        fn disjoint_decomposition(
            &mut self,
            node: &Self::Node,
        ) -> DecompositionResult<Vec<Self::Node>> {
            Ok(node
                .disjoint_result
                .clone()
                .unwrap_or_else(|| vec![node.clone()]))
        }
    }

    fn remove_divisor(divisors: &mut Vec<String>, name: &str) {
        if let Some(index) = divisors.iter().position(|divisor| divisor == name) {
            divisors.remove(index);
        }
    }

    #[derive(Default)]
    struct TestNetwork {
        nodes: Vec<TestNode>,
    }

    impl TestNetwork {
        fn new(nodes: Vec<TestNode>) -> Self {
            Self { nodes }
        }

        fn names(&self) -> Vec<&str> {
            self.nodes.iter().map(|node| node.name.as_str()).collect()
        }
    }

    impl NetworkDecomposition for TestNetwork {
        type Node = TestNode;
        type NodeId = usize;

        fn node_ids_snapshot(&self) -> Vec<Self::NodeId> {
            (0..self.nodes.len()).collect()
        }

        fn node_kind(&self, node: &Self::NodeId) -> DecompositionResult<NodeKind> {
            self.nodes
                .get(*node)
                .map(|node| node.kind)
                .ok_or_else(|| DecompositionError::backend("node id is outside the network"))
        }

        fn clone_node(&self, node: &Self::NodeId) -> DecompositionResult<Self::Node> {
            self.nodes
                .get(*node)
                .cloned()
                .ok_or_else(|| DecompositionError::backend("node id is outside the network"))
        }

        fn replace_node(
            &mut self,
            node: &Self::NodeId,
            replacement: Self::Node,
        ) -> DecompositionResult<()> {
            let Some(slot) = self.nodes.get_mut(*node) else {
                return Err(DecompositionError::backend(
                    "node id is outside the network",
                ));
            };

            *slot = replacement;
            Ok(())
        }

        fn add_node(&mut self, node: Self::Node) -> DecompositionResult<Self::NodeId> {
            let id = self.nodes.len();
            self.nodes.push(node);
            Ok(id)
        }
    }

    #[test]
    fn quick_network_replaces_internal_root_and_adds_remaining_nodes() {
        let mut network = TestNetwork::new(vec![
            TestNode::input("a"),
            TestNode::internal("f").with_quick_divisors(&["g"]),
        ]);
        let mut algebra = TestAlgebra::default();

        let summary = decompose_quick_network(&mut network, &mut algebra).unwrap();

        assert_eq!(
            summary,
            NetworkDecompositionSummary {
                visited_internal_nodes: 1,
                replaced_nodes: 1,
                added_nodes: 1,
                unchanged_internal_nodes: 0,
            }
        );
        assert_eq!(network.names(), vec!["a", "f-without-g", "g"]);
        assert!(network.nodes[1].normalized);
    }

    #[test]
    fn good_node_uses_good_kernel_and_preserves_root_first_order() {
        let mut algebra = TestAlgebra::default();
        let node = TestNode::internal("f")
            .with_quick_divisors(&["quick"])
            .with_good_divisors(&["good"]);

        let decomposition = decompose_good_node(&mut algebra, &node).unwrap();
        let names: Vec<_> = decomposition
            .iter()
            .map(|node| node.name.as_str())
            .collect();

        assert_eq!(names, vec!["f-without-good", "good"]);
    }

    #[test]
    fn single_node_decomposition_is_discarded_for_network_mutation() {
        let mut network = TestNetwork::new(vec![TestNode::internal("f")]);
        let mut algebra = TestAlgebra::default();

        let summary = decompose_quick_network(&mut network, &mut algebra).unwrap();

        assert_eq!(summary.visited_internal_nodes, 1);
        assert_eq!(summary.unchanged_internal_nodes, 1);
        assert_eq!(network.names(), vec!["f"]);
    }

    #[test]
    fn disjoint_network_uses_backend_partition_result() {
        let root = TestNode::internal("or");
        let first = TestNode::internal("p0");
        let second = TestNode::internal("p1");
        let node = TestNode::internal("f").with_disjoint_result(vec![root, first, second]);
        let mut network = TestNetwork::new(vec![node]);
        let mut algebra = TestAlgebra::default();

        let summary = decompose_disjoint_network(&mut network, &mut algebra).unwrap();

        assert_eq!(summary.replaced_nodes, 1);
        assert_eq!(summary.added_nodes, 2);
        assert_eq!(network.names(), vec!["or", "p0", "p1"]);
    }

    #[test]
    fn rejected_substitution_reports_generic_diagnostic() {
        let mut algebra = TestAlgebra {
            reject_substitution: true,
        };
        let node = TestNode::internal("f").with_quick_divisors(&["g"]);

        let error = decompose_quick_node(&mut algebra, &node).unwrap_err();

        assert_eq!(error, DecompositionError::SubstitutionRejected);
        assert_eq!(
            error.to_string(),
            "decomposition divisor could not be substituted"
        );
    }
}
