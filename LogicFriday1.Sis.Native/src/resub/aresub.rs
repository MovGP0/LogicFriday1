//! Native Rust model for `LogicSynthesis/sis/resub/aresub.c`.
//!
//! The C implementation repeatedly substitutes one algebraic node into the
//! one-level transitive fanout of each of its fanins. The SIS `network_t` and
//! `node_t` integration points are still native-port blockers, so this module
//! ports the traversal/substitution orchestration behind a Rust trait and
//! reports unavailable native support explicitly.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

impl NodeKind {
    pub fn is_substitution_source(self) -> bool {
        matches!(self, Self::Internal)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AresubOperation {
    NodeKind,
    Fanins,
    OneLevelTransitiveFanout,
    Substitute,
    NetworkNodes,
    AlgebraicNode,
    AlgebraicNetwork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AresubError {
    MissingNativePorts { operation: AresubOperation },
    Backend(String),
}

impl fmt::Display for AresubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation:?} requires unavailable native Rust SIS support"
            ),
            Self::Backend(message) => f.write_str(message),
        }
    }
}

impl Error for AresubError {}

pub type AresubResult<T> = Result<T, AresubError>;

pub trait AlgebraicResubGraph {
    type Node: Clone + Ord;

    fn node_kind(&self, node: &Self::Node) -> AresubResult<NodeKind>;

    fn fanins(&self, node: &Self::Node) -> AresubResult<Vec<Self::Node>>;

    fn one_level_transitive_fanout(&self, node: &Self::Node) -> AresubResult<Vec<Self::Node>>;

    fn substitute(
        &mut self,
        source: &Self::Node,
        target: &Self::Node,
        use_complement: bool,
    ) -> AresubResult<bool>;

    fn nodes(&self) -> AresubResult<Vec<Self::Node>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlgebraicNodePlan<N> {
    pub source: N,
    pub targets: Vec<N>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlgebraicNetworkStats {
    pub passes: usize,
    pub changed_nodes: usize,
}

pub fn algebraic_resubstitution_targets<G>(
    graph: &G,
    source: &G::Node,
) -> AresubResult<Vec<G::Node>>
where
    G: AlgebraicResubGraph,
{
    if !graph.node_kind(source)?.is_substitution_source() {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();
    for fanin in graph.fanins(source)? {
        targets.extend(graph.one_level_transitive_fanout(&fanin)?);
    }
    targets.sort();
    targets.dedup();
    Ok(targets)
}

pub fn plan_resub_alge_node<G>(
    graph: &G,
    source: G::Node,
) -> AresubResult<AlgebraicNodePlan<G::Node>>
where
    G: AlgebraicResubGraph,
{
    let targets = algebraic_resubstitution_targets(graph, &source)?;
    Ok(AlgebraicNodePlan { source, targets })
}

pub fn resub_alge_node<G>(
    graph: &mut G,
    source: &G::Node,
    use_complement: bool,
) -> AresubResult<bool>
where
    G: AlgebraicResubGraph,
{
    let targets = algebraic_resubstitution_targets(graph, source)?;
    let mut changed = false;

    for target in targets {
        if graph.substitute(source, &target, use_complement)? {
            changed = true;
        }
    }

    Ok(changed)
}

pub fn resub_alge_network<G>(
    graph: &mut G,
    use_complement: bool,
) -> AresubResult<AlgebraicNetworkStats>
where
    G: AlgebraicResubGraph,
{
    let mut stats = AlgebraicNetworkStats {
        passes: 0,
        changed_nodes: 0,
    };

    loop {
        stats.passes += 1;
        let mut changed_this_pass = false;
        for node in graph.nodes()? {
            if resub_alge_node(graph, &node, use_complement)? {
                stats.changed_nodes += 1;
                changed_this_pass = true;
            }
        }

        if !changed_this_pass {
            return Ok(stats);
        }
    }
}

#[derive(Default)]
pub struct MissingAlgebraicResubGraph;

impl AlgebraicResubGraph for MissingAlgebraicResubGraph {
    type Node = String;

    fn node_kind(&self, _node: &Self::Node) -> AresubResult<NodeKind> {
        Err(missing(AresubOperation::NodeKind))
    }

    fn fanins(&self, _node: &Self::Node) -> AresubResult<Vec<Self::Node>> {
        Err(missing(AresubOperation::Fanins))
    }

    fn one_level_transitive_fanout(&self, _node: &Self::Node) -> AresubResult<Vec<Self::Node>> {
        Err(missing(AresubOperation::OneLevelTransitiveFanout))
    }

    fn substitute(
        &mut self,
        _source: &Self::Node,
        _target: &Self::Node,
        _use_complement: bool,
    ) -> AresubResult<bool> {
        Err(missing(AresubOperation::Substitute))
    }

    fn nodes(&self) -> AresubResult<Vec<Self::Node>> {
        Err(missing(AresubOperation::NetworkNodes))
    }
}

pub fn resub_alge_node_bound<Node>(_source: &Node, _use_complement: bool) -> AresubResult<bool> {
    Err(missing(AresubOperation::AlgebraicNode))
}

pub fn resub_alge_network_bound<Network>(
    _network: &mut Network,
    _use_complement: bool,
) -> AresubResult<AlgebraicNetworkStats> {
    Err(missing(AresubOperation::AlgebraicNetwork))
}

fn missing(operation: AresubOperation) -> AresubError {
    AresubError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[derive(Clone, Debug)]
    struct TestNode {
        kind: NodeKind,
        fanins: Vec<&'static str>,
        fanouts: Vec<&'static str>,
    }

    #[derive(Default)]
    struct TestGraph {
        nodes: HashMap<&'static str, TestNode>,
        changed_once: HashSet<(&'static str, &'static str)>,
        calls: Vec<(&'static str, &'static str, bool)>,
    }

    impl TestGraph {
        fn with_node(
            mut self,
            id: &'static str,
            kind: NodeKind,
            fanins: &[&'static str],
            fanouts: &[&'static str],
        ) -> Self {
            self.nodes.insert(
                id,
                TestNode {
                    kind,
                    fanins: fanins.to_vec(),
                    fanouts: fanouts.to_vec(),
                },
            );
            self
        }

        fn changing(mut self, source: &'static str, target: &'static str) -> Self {
            self.changed_once.insert((source, target));
            self
        }
    }

    impl AlgebraicResubGraph for TestGraph {
        type Node = &'static str;

        fn node_kind(&self, node: &Self::Node) -> AresubResult<NodeKind> {
            Ok(self.nodes[node].kind)
        }

        fn fanins(&self, node: &Self::Node) -> AresubResult<Vec<Self::Node>> {
            Ok(self.nodes[node].fanins.clone())
        }

        fn one_level_transitive_fanout(&self, node: &Self::Node) -> AresubResult<Vec<Self::Node>> {
            Ok(self.nodes[node].fanouts.clone())
        }

        fn substitute(
            &mut self,
            source: &Self::Node,
            target: &Self::Node,
            use_complement: bool,
        ) -> AresubResult<bool> {
            self.calls.push((*source, *target, use_complement));
            Ok(self.changed_once.remove(&(*source, *target)))
        }

        fn nodes(&self) -> AresubResult<Vec<Self::Node>> {
            let mut nodes = self.nodes.keys().copied().collect::<Vec<_>>();
            nodes.sort();
            Ok(nodes)
        }
    }

    fn sample_graph() -> TestGraph {
        TestGraph::default()
            .with_node("f", NodeKind::Internal, &["b", "a"], &[])
            .with_node("a", NodeKind::Internal, &[], &["z", "x"])
            .with_node("b", NodeKind::Internal, &[], &["x", "y"])
            .with_node("x", NodeKind::Internal, &[], &[])
            .with_node("y", NodeKind::Internal, &[], &[])
            .with_node("z", NodeKind::Internal, &[], &[])
    }

    #[test]
    fn primary_inputs_and_outputs_are_not_substitution_sources() {
        let graph = TestGraph::default()
            .with_node("pi", NodeKind::PrimaryInput, &["a"], &[])
            .with_node("po", NodeKind::PrimaryOutput, &["a"], &[])
            .with_node("a", NodeKind::Internal, &[], &["target"]);

        assert_eq!(algebraic_resubstitution_targets(&graph, &"pi"), Ok(vec![]));
        assert_eq!(algebraic_resubstitution_targets(&graph, &"po"), Ok(vec![]));
    }

    #[test]
    fn targets_are_collected_from_each_fanin_then_sorted_and_deduplicated() {
        let graph = sample_graph();

        assert_eq!(
            algebraic_resubstitution_targets(&graph, &"f").unwrap(),
            vec!["x", "y", "z"]
        );
        assert_eq!(
            plan_resub_alge_node(&graph, "f").unwrap(),
            AlgebraicNodePlan {
                source: "f",
                targets: vec!["x", "y", "z"],
            }
        );
    }

    #[test]
    fn node_resubstitution_tries_each_target_and_reports_any_change() {
        let mut graph = sample_graph().changing("f", "y");

        assert_eq!(resub_alge_node(&mut graph, &"f", false), Ok(true));
        assert_eq!(
            graph.calls,
            vec![("f", "x", false), ("f", "y", false), ("f", "z", false)]
        );
    }

    #[test]
    fn node_resubstitution_returns_false_when_no_target_changes() {
        let mut graph = sample_graph();

        assert_eq!(resub_alge_node(&mut graph, &"f", true), Ok(false));
        assert_eq!(
            graph.calls,
            vec![("f", "x", true), ("f", "y", true), ("f", "z", true)]
        );
    }

    #[test]
    fn network_resubstitution_repeats_until_a_pass_has_no_changes() {
        let mut graph = sample_graph()
            .changing("f", "x")
            .changing("f", "z")
            .with_node("pi", NodeKind::PrimaryInput, &[], &["f"]);

        assert_eq!(
            resub_alge_network(&mut graph, true),
            Ok(AlgebraicNetworkStats {
                passes: 2,
                changed_nodes: 1,
            })
        );

        let f_calls = graph
            .calls
            .iter()
            .filter(|(source, _, _)| *source == "f")
            .count();
        assert_eq!(f_calls, 6);
    }

    #[test]
    fn network_resubstitution_still_performs_one_pass_when_nothing_changes() {
        let mut graph = sample_graph();

        assert_eq!(
            resub_alge_network(&mut graph, false),
            Ok(AlgebraicNetworkStats {
                passes: 1,
                changed_nodes: 0,
            })
        );
    }

    #[test]
    fn missing_dependency_errors_identify_failed_operation() {
        let error = resub_alge_network_bound(&mut (), true).unwrap_err();
        let AresubError::MissingNativePorts { operation } = error else {
            panic!("expected missing native ports");
        };

        assert_eq!(operation, AresubOperation::AlgebraicNetwork);
        assert_eq!(
            error.to_string(),
            "AlgebraicNetwork requires unavailable native Rust SIS support"
        );
    }

    #[test]
    fn missing_backend_reports_node_kind_dependency_before_other_graph_calls() {
        let graph = MissingAlgebraicResubGraph;
        assert_eq!(
            algebraic_resubstitution_targets(&graph, &"f".to_owned()),
            Err(AresubError::MissingNativePorts {
                operation: AresubOperation::NodeKind,
            })
        );
    }
}
