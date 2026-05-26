//! Native Rust orchestration for `sis/speed/speed_or.c`.
//!
//! The original routine rewrites a sum-of-products node into a NAND-style OR
//! combiner over decoded cube nodes, speed-decomposes each cube, optionally
//! collapses one-input cube nodes back into the combiner, then speed-decomposes
//! the combiner itself. This module keeps that behavior as a Rust backend
//! contract rather than exporting legacy C ABI entry points.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpeedOrOptions {
    pub add_inv: bool,
}

impl SpeedOrOptions {
    pub fn new(add_inv: bool) -> Self {
        Self { add_inv }
    }
}

pub trait SpeedOrBackend {
    type Node: Clone + Eq;
    type Error: Error + Send + Sync + 'static;

    fn cube_count(&self, node: &Self::Node) -> Result<usize, Self::Error>;

    fn decode_cube_node(
        &mut self,
        node: &Self::Node,
        cube_index: usize,
    ) -> Result<Self::Node, Self::Error>;

    fn add_node(&mut self, node: Self::Node) -> Result<(), Self::Error>;

    fn replace_with_negative_cube_combiner(
        &mut self,
        node: &Self::Node,
        cubes: &[Self::Node],
    ) -> Result<(), Self::Error>;

    fn fanins(&self, node: &Self::Node) -> Result<Vec<Self::Node>, Self::Error>;

    fn fanin_count(&self, node: &Self::Node) -> Result<usize, Self::Error>;

    fn fanout_count(&self, node: &Self::Node) -> Result<usize, Self::Error>;

    fn collapse_node(&mut self, fanout: &Self::Node, fanin: &Self::Node)
    -> Result<(), Self::Error>;

    fn delete_node(&mut self, node: &Self::Node) -> Result<(), Self::Error>;

    fn speed_and_decompose(
        &mut self,
        node: &Self::Node,
        invert_output: bool,
    ) -> Result<bool, Self::Error>;
}

pub fn speed_and_or_decompose<B>(
    backend: &mut B,
    node: B::Node,
    options: SpeedOrOptions,
) -> Result<bool, SpeedOrError<B::Error>>
where
    B: SpeedOrBackend,
{
    let cube_count = backend.cube_count(&node).map_err(SpeedOrError::Backend)?;

    if cube_count <= 1 {
        return speed_and_decompose(backend, &node, false, "single cube");
    }

    let mut cubes = Vec::with_capacity(cube_count);
    for cube_index in 0..cube_count {
        let cube = backend
            .decode_cube_node(&node, cube_index)
            .map_err(SpeedOrError::Backend)?;
        backend
            .add_node(cube.clone())
            .map_err(SpeedOrError::Backend)?;
        cubes.push(cube);
    }

    backend
        .replace_with_negative_cube_combiner(&node, &cubes)
        .map_err(SpeedOrError::Backend)?;

    for cube in backend.fanins(&node).map_err(SpeedOrError::Backend)? {
        speed_and_decompose(backend, &cube, false, "cube")?;
    }

    if !options.add_inv {
        collapse_single_fanin_cubes(backend, &node)?;
    }

    speed_and_decompose(backend, &node, true, "node combining cubes")
}

fn collapse_single_fanin_cubes<B>(
    backend: &mut B,
    node: &B::Node,
) -> Result<(), SpeedOrError<B::Error>>
where
    B: SpeedOrBackend,
{
    let mut collapse_candidates = Vec::new();
    for cube in backend.fanins(node).map_err(SpeedOrError::Backend)? {
        let fanin_count = backend.fanin_count(&cube).map_err(SpeedOrError::Backend)?;
        if fanin_count <= 1 {
            collapse_candidates.push(cube);
        }
    }

    for cube in collapse_candidates {
        backend
            .collapse_node(node, &cube)
            .map_err(SpeedOrError::Backend)?;
        let fanout_count = backend.fanout_count(&cube).map_err(SpeedOrError::Backend)?;
        if fanout_count == 0 {
            backend.delete_node(&cube).map_err(SpeedOrError::Backend)?;
        }
    }

    Ok(())
}

fn speed_and_decompose<B>(
    backend: &mut B,
    node: &B::Node,
    invert_output: bool,
    target: &'static str,
) -> Result<bool, SpeedOrError<B::Error>>
where
    B: SpeedOrBackend,
{
    if backend
        .speed_and_decompose(node, invert_output)
        .map_err(SpeedOrError::Backend)?
    {
        return Ok(true);
    }

    Err(SpeedOrError::SpeedAndFailed { target })
}

#[derive(Debug)]
pub enum SpeedOrError<E> {
    Backend(E),
    SpeedAndFailed { target: &'static str },
}

impl<E> fmt::Display for SpeedOrError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(f, "{error}"),
            Self::SpeedAndFailed { target } => {
                write!(f, "failed to decompose {target}")
            }
        }
    }
}

impl<E> Error for SpeedOrError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::SpeedAndFailed { .. } => None,
        }
    }
}

impl<E> PartialEq for SpeedOrError<E>
where
    E: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Backend(left), Self::Backend(right)) => left == right,
            (Self::SpeedAndFailed { target: left }, Self::SpeedAndFailed { target: right }) => {
                left == right
            }
            _ => false,
        }
    }
}

impl<E> Eq for SpeedOrError<E> where E: Eq {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    struct NodeId(usize);

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNode {
        cube_count: usize,
        fanins: Vec<NodeId>,
        fanouts: BTreeSet<NodeId>,
    }

    impl TestNode {
        fn new(cube_count: usize, fanins: Vec<NodeId>) -> Self {
            Self {
                cube_count,
                fanins,
                fanouts: BTreeSet::new(),
            }
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        DecodeCube {
            node: NodeId,
            cube_index: usize,
            cube: NodeId,
        },
        AddNode(NodeId),
        ReplaceWithCombiner {
            node: NodeId,
            cubes: Vec<NodeId>,
        },
        SpeedAnd {
            node: NodeId,
            invert_output: bool,
        },
        Collapse {
            fanout: NodeId,
            fanin: NodeId,
        },
        Delete(NodeId),
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum TestError {
        MissingNode(NodeId),
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::MissingNode(node) => write!(f, "missing node {node:?}"),
            }
        }
    }

    impl Error for TestError {}

    struct TestBackend {
        nodes: BTreeMap<NodeId, TestNode>,
        cube_fanin_counts: Vec<usize>,
        next_node: usize,
        events: Vec<Event>,
        failed_speed_and: Option<NodeId>,
    }

    impl TestBackend {
        fn new(root: TestNode, cube_fanin_counts: Vec<usize>) -> Self {
            let mut nodes = BTreeMap::new();
            nodes.insert(NodeId(0), root);
            Self {
                nodes,
                cube_fanin_counts,
                next_node: 1,
                events: Vec::new(),
                failed_speed_and: None,
            }
        }

        fn node(&self, node: NodeId) -> Result<&TestNode, TestError> {
            self.nodes.get(&node).ok_or(TestError::MissingNode(node))
        }

        fn node_mut(&mut self, node: NodeId) -> Result<&mut TestNode, TestError> {
            self.nodes
                .get_mut(&node)
                .ok_or(TestError::MissingNode(node))
        }
    }

    impl SpeedOrBackend for TestBackend {
        type Node = NodeId;
        type Error = TestError;

        fn cube_count(&self, node: &Self::Node) -> Result<usize, Self::Error> {
            Ok(self.node(*node)?.cube_count)
        }

        fn decode_cube_node(
            &mut self,
            node: &Self::Node,
            cube_index: usize,
        ) -> Result<Self::Node, Self::Error> {
            let cube = NodeId(self.next_node);
            self.next_node += 1;
            let fanins = (0..self.cube_fanin_counts[cube_index])
                .map(|offset| NodeId(100 + cube_index * 10 + offset))
                .collect();
            self.nodes.insert(cube, TestNode::new(1, fanins));
            self.events.push(Event::DecodeCube {
                node: *node,
                cube_index,
                cube,
            });
            Ok(cube)
        }

        fn add_node(&mut self, node: Self::Node) -> Result<(), Self::Error> {
            self.node(node)?;
            self.events.push(Event::AddNode(node));
            Ok(())
        }

        fn replace_with_negative_cube_combiner(
            &mut self,
            node: &Self::Node,
            cubes: &[Self::Node],
        ) -> Result<(), Self::Error> {
            for cube in cubes {
                self.node_mut(*cube)?.fanouts.insert(*node);
            }

            let root = self.node_mut(*node)?;
            root.fanins = cubes.to_vec();
            root.cube_count = 1;
            self.events.push(Event::ReplaceWithCombiner {
                node: *node,
                cubes: cubes.to_vec(),
            });
            Ok(())
        }

        fn fanins(&self, node: &Self::Node) -> Result<Vec<Self::Node>, Self::Error> {
            Ok(self.node(*node)?.fanins.clone())
        }

        fn fanin_count(&self, node: &Self::Node) -> Result<usize, Self::Error> {
            Ok(self.node(*node)?.fanins.len())
        }

        fn fanout_count(&self, node: &Self::Node) -> Result<usize, Self::Error> {
            Ok(self.node(*node)?.fanouts.len())
        }

        fn collapse_node(
            &mut self,
            fanout: &Self::Node,
            fanin: &Self::Node,
        ) -> Result<(), Self::Error> {
            self.node_mut(*fanout)?
                .fanins
                .retain(|candidate| candidate != fanin);
            self.node_mut(*fanin)?.fanouts.remove(fanout);
            self.events.push(Event::Collapse {
                fanout: *fanout,
                fanin: *fanin,
            });
            Ok(())
        }

        fn delete_node(&mut self, node: &Self::Node) -> Result<(), Self::Error> {
            self.nodes
                .remove(node)
                .ok_or(TestError::MissingNode(*node))?;
            self.events.push(Event::Delete(*node));
            Ok(())
        }

        fn speed_and_decompose(
            &mut self,
            node: &Self::Node,
            invert_output: bool,
        ) -> Result<bool, Self::Error> {
            self.node(*node)?;
            self.events.push(Event::SpeedAnd {
                node: *node,
                invert_output,
            });
            Ok(self.failed_speed_and != Some(*node))
        }
    }

    #[test]
    fn multi_cube_decomposition_matches_legacy_operation_order() {
        let mut backend = TestBackend::new(TestNode::new(3, Vec::new()), vec![2, 3, 4]);

        assert_eq!(
            speed_and_or_decompose(&mut backend, NodeId(0), SpeedOrOptions::new(true)),
            Ok(true)
        );

        assert_eq!(
            backend.events,
            vec![
                Event::DecodeCube {
                    node: NodeId(0),
                    cube_index: 0,
                    cube: NodeId(1),
                },
                Event::AddNode(NodeId(1)),
                Event::DecodeCube {
                    node: NodeId(0),
                    cube_index: 1,
                    cube: NodeId(2),
                },
                Event::AddNode(NodeId(2)),
                Event::DecodeCube {
                    node: NodeId(0),
                    cube_index: 2,
                    cube: NodeId(3),
                },
                Event::AddNode(NodeId(3)),
                Event::ReplaceWithCombiner {
                    node: NodeId(0),
                    cubes: vec![NodeId(1), NodeId(2), NodeId(3)],
                },
                Event::SpeedAnd {
                    node: NodeId(1),
                    invert_output: false,
                },
                Event::SpeedAnd {
                    node: NodeId(2),
                    invert_output: false,
                },
                Event::SpeedAnd {
                    node: NodeId(3),
                    invert_output: false,
                },
                Event::SpeedAnd {
                    node: NodeId(0),
                    invert_output: true,
                },
            ]
        );
    }

    #[test]
    fn add_inv_false_collapses_and_deletes_single_fanin_cube_nodes() {
        let mut backend = TestBackend::new(TestNode::new(4, Vec::new()), vec![0, 1, 2, 1]);

        assert_eq!(
            speed_and_or_decompose(&mut backend, NodeId(0), SpeedOrOptions::new(false)),
            Ok(true)
        );

        assert_eq!(
            backend
                .events
                .iter()
                .filter(|event| matches!(event, Event::Collapse { .. } | Event::Delete(_)))
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Event::Collapse {
                    fanout: NodeId(0),
                    fanin: NodeId(1),
                },
                Event::Delete(NodeId(1)),
                Event::Collapse {
                    fanout: NodeId(0),
                    fanin: NodeId(2),
                },
                Event::Delete(NodeId(2)),
                Event::Collapse {
                    fanout: NodeId(0),
                    fanin: NodeId(4),
                },
                Event::Delete(NodeId(4)),
            ]
        );
    }

    #[test]
    fn single_cube_or_constant_decomposes_original_node_without_replacement() {
        for cube_count in [0, 1] {
            let mut backend = TestBackend::new(TestNode::new(cube_count, Vec::new()), Vec::new());

            assert_eq!(
                speed_and_or_decompose(&mut backend, NodeId(0), SpeedOrOptions::new(false)),
                Ok(true)
            );

            assert_eq!(
                backend.events,
                vec![Event::SpeedAnd {
                    node: NodeId(0),
                    invert_output: false,
                }]
            );
        }
    }

    #[test]
    fn failed_speed_and_reports_the_matching_legacy_failure_context() {
        let mut backend = TestBackend::new(TestNode::new(2, Vec::new()), vec![2, 2]);
        backend.failed_speed_and = Some(NodeId(1));

        assert_eq!(
            speed_and_or_decompose(&mut backend, NodeId(0), SpeedOrOptions::new(true)),
            Err(SpeedOrError::SpeedAndFailed { target: "cube" })
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let text = include_str!("speed_or.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
    }
}
