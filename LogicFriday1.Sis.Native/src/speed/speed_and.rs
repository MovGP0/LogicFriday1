//! Native Rust orchestration for `sis/speed/speed_and.c`.
//!
//! The original routine decomposes a single-cube node into an AND tree by
//! repeatedly pairing the two earliest arriving literals. This module exposes
//! that behavior through a Rust backend contract instead of legacy C ABI entry
//! points.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpeedAndOptions {
    pub invert_output: bool,
}

impl SpeedAndOptions {
    pub const fn new(invert_output: bool) -> Self {
        Self { invert_output }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LiteralValue {
    Zero,
    One,
    DontCare,
}

impl LiteralValue {
    pub fn phase(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            Self::DontCare => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn arrival(self) -> f64 {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndLiteral {
    pub fanin_index: usize,
    pub literal: LiteralValue,
    pub arrival: DelayTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedLiteral {
    pub fanin_index: usize,
    pub phase: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AndDecompStep {
    SingleLevelUpdate,
    InvertConstantOrSingleLiteral,
    CreateAndNode {
        first: SelectedLiteral,
        second: SelectedLiteral,
    },
    SubstituteAndRecurse,
    AddExplicitInverterForMultiLiteralNode,
}

pub trait SpeedAndBackend {
    type Node: Clone + Eq;
    type Error: Error + Send + Sync + 'static;

    fn literal_count(&self, node: &Self::Node) -> Result<usize, Self::Error>;

    fn cube_literals(
        &self,
        node: &Self::Node,
        cube_index: usize,
    ) -> Result<Vec<LiteralValue>, Self::Error>;

    fn fanins(&self, node: &Self::Node) -> Result<Vec<Self::Node>, Self::Error>;

    fn arrival_time(&self, node: &Self::Node) -> Result<DelayTime, Self::Error>;

    fn make_literal(&mut self, fanin: &Self::Node, phase: bool) -> Result<Self::Node, Self::Error>;

    fn make_and(
        &mut self,
        left: &Self::Node,
        right: &Self::Node,
    ) -> Result<Self::Node, Self::Error>;

    fn add_node(&mut self, node: Self::Node) -> Result<(), Self::Error>;

    fn update_single_level(&mut self, node: &Self::Node) -> Result<DelayTime, Self::Error>;

    fn substitute(
        &mut self,
        node: &Self::Node,
        new_node: &Self::Node,
        positive_phase: bool,
    ) -> Result<bool, Self::Error>;

    fn replace_with_not(&mut self, node: &Self::Node) -> Result<(), Self::Error>;

    fn duplicate_node(&mut self, node: &Self::Node) -> Result<Self::Node, Self::Error>;

    fn replace_with_literal(
        &mut self,
        node: &Self::Node,
        fanin: &Self::Node,
        phase: bool,
    ) -> Result<(), Self::Error>;
}

pub fn speed_and_decompose<B>(
    backend: &mut B,
    node: B::Node,
    options: SpeedAndOptions,
) -> Result<bool, SpeedAndError<B::Error>>
where
    B: SpeedAndBackend,
{
    decompose_node(backend, &node, options.invert_output)
}

fn decompose_node<B>(
    backend: &mut B,
    node: &B::Node,
    invert_output: bool,
) -> Result<bool, SpeedAndError<B::Error>>
where
    B: SpeedAndBackend,
{
    let literal_count = backend
        .literal_count(node)
        .map_err(SpeedAndError::Backend)?;
    if literal_count == 0 {
        if invert_output {
            backend
                .replace_with_not(node)
                .map_err(SpeedAndError::Backend)?;
        }
        backend
            .update_single_level(node)
            .map_err(SpeedAndError::Backend)?;
        return Ok(true);
    }

    let fanins = backend.fanins(node).map_err(SpeedAndError::Backend)?;
    let cube = backend
        .cube_literals(node, 0)
        .map_err(SpeedAndError::Backend)?;
    if cube.len() != fanins.len() {
        return Err(SpeedAndError::CubeFaninMismatch {
            literals: cube.len(),
            fanins: fanins.len(),
        });
    }

    let literals = timed_literals(backend, &fanins, &cube)?;
    if literals.len() > 2 {
        let selected = select_two_earliest_literals(&literals);
        if selected.len() != 2 {
            return Err(SpeedAndError::NoSubstitutablePair);
        }

        let first = &selected[0];
        let second = &selected[1];
        let nlit = backend
            .make_literal(&fanins[first.fanin_index], first.phase)
            .map_err(SpeedAndError::Backend)?;
        let mlit = backend
            .make_literal(&fanins[second.fanin_index], second.phase)
            .map_err(SpeedAndError::Backend)?;
        let new_node = backend
            .make_and(&nlit, &mlit)
            .map_err(SpeedAndError::Backend)?;
        backend
            .add_node(new_node.clone())
            .map_err(SpeedAndError::Backend)?;
        backend
            .update_single_level(&new_node)
            .map_err(SpeedAndError::Backend)?;

        if !backend
            .substitute(node, &new_node, true)
            .map_err(SpeedAndError::Backend)?
        {
            return Err(SpeedAndError::SubstituteFailed);
        }

        return decompose_node(backend, node, invert_output);
    }

    if invert_output {
        if literals.len() > 1 {
            let temp_node = backend
                .duplicate_node(node)
                .map_err(SpeedAndError::Backend)?;
            backend
                .add_node(temp_node.clone())
                .map_err(SpeedAndError::Backend)?;
            backend
                .replace_with_literal(node, &temp_node, false)
                .map_err(SpeedAndError::Backend)?;
            backend
                .update_single_level(&temp_node)
                .map_err(SpeedAndError::Backend)?;
        } else {
            backend
                .replace_with_not(node)
                .map_err(SpeedAndError::Backend)?;
        }
    }

    backend
        .update_single_level(node)
        .map_err(SpeedAndError::Backend)?;
    Ok(true)
}

fn timed_literals<B>(
    backend: &B,
    fanins: &[B::Node],
    cube: &[LiteralValue],
) -> Result<Vec<AndLiteral>, SpeedAndError<B::Error>>
where
    B: SpeedAndBackend,
{
    let mut literals = Vec::new();
    for (fanin_index, literal) in cube.iter().copied().enumerate() {
        if literal.phase().is_none() {
            continue;
        }

        let arrival = backend
            .arrival_time(&fanins[fanin_index])
            .map_err(SpeedAndError::Backend)?;
        if !arrival.rise.is_finite() || !arrival.fall.is_finite() {
            return Err(SpeedAndError::NonFiniteArrival);
        }

        literals.push(AndLiteral {
            fanin_index,
            literal,
            arrival,
        });
    }

    Ok(literals)
}

pub fn select_two_earliest_literals(literals: &[AndLiteral]) -> Vec<SelectedLiteral> {
    let mut selected: Vec<(f64, SelectedLiteral)> = literals
        .iter()
        .filter_map(|literal| {
            literal.literal.phase().map(|phase| {
                (
                    literal.arrival.arrival(),
                    SelectedLiteral {
                        fanin_index: literal.fanin_index,
                        phase,
                    },
                )
            })
        })
        .collect();

    selected.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then(left.1.fanin_index.cmp(&right.1.fanin_index))
    });
    selected
        .into_iter()
        .take(2)
        .map(|(_, literal)| literal)
        .collect()
}

pub fn plan_and_decomposition(
    literals: &[AndLiteral],
    invert_result: bool,
) -> Result<Vec<AndDecompStep>, SpeedAndPlanError> {
    let literal_count = literals
        .iter()
        .filter(|literal| literal.literal.phase().is_some())
        .count();

    if literal_count == 0 {
        return Ok(if invert_result {
            vec![
                AndDecompStep::InvertConstantOrSingleLiteral,
                AndDecompStep::SingleLevelUpdate,
            ]
        } else {
            vec![AndDecompStep::SingleLevelUpdate]
        });
    }

    if literal_count > 2 {
        let selected = select_two_earliest_literals(literals);
        if selected.len() != 2 {
            return Err(SpeedAndPlanError::NoSubstitutablePair);
        }
        return Ok(vec![
            AndDecompStep::CreateAndNode {
                first: selected[0].clone(),
                second: selected[1].clone(),
            },
            AndDecompStep::SingleLevelUpdate,
            AndDecompStep::SubstituteAndRecurse,
        ]);
    }

    let mut steps = Vec::new();
    if invert_result {
        if literal_count > 1 {
            steps.push(AndDecompStep::AddExplicitInverterForMultiLiteralNode);
        } else {
            steps.push(AndDecompStep::InvertConstantOrSingleLiteral);
        }
    }
    steps.push(AndDecompStep::SingleLevelUpdate);
    Ok(steps)
}

#[derive(Debug)]
pub enum SpeedAndError<E> {
    Backend(E),
    NoSubstitutablePair,
    NonFiniteArrival,
    CubeFaninMismatch { literals: usize, fanins: usize },
    SubstituteFailed,
}

impl<E> fmt::Display for SpeedAndError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(f, "{error}"),
            Self::NoSubstitutablePair => write!(f, "no two substitutable literals were found"),
            Self::NonFiniteArrival => write!(f, "fanin arrival time is not finite"),
            Self::CubeFaninMismatch { literals, fanins } => {
                write!(f, "cube has {literals} literals for {fanins} fanins")
            }
            Self::SubstituteFailed => write!(f, "substitute failed in speed_and_decomp"),
        }
    }
}

impl<E> Error for SpeedAndError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::NoSubstitutablePair
            | Self::NonFiniteArrival
            | Self::CubeFaninMismatch { .. }
            | Self::SubstituteFailed => None,
        }
    }
}

impl<E> PartialEq for SpeedAndError<E>
where
    E: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Backend(left), Self::Backend(right)) => left == right,
            (Self::NoSubstitutablePair, Self::NoSubstitutablePair)
            | (Self::NonFiniteArrival, Self::NonFiniteArrival)
            | (Self::SubstituteFailed, Self::SubstituteFailed) => true,
            (
                Self::CubeFaninMismatch {
                    literals: left_literals,
                    fanins: left_fanins,
                },
                Self::CubeFaninMismatch {
                    literals: right_literals,
                    fanins: right_fanins,
                },
            ) => left_literals == right_literals && left_fanins == right_fanins,
            _ => false,
        }
    }
}

impl<E> Eq for SpeedAndError<E> where E: Eq {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedAndPlanError {
    NoSubstitutablePair,
}

impl fmt::Display for SpeedAndPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSubstitutablePair => write!(f, "no two substitutable literals were found"),
        }
    }
}

impl Error for SpeedAndPlanError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    struct NodeId(usize);

    #[derive(Clone, Debug, PartialEq)]
    struct TestNode {
        fanins: Vec<NodeId>,
        literals: Vec<LiteralValue>,
        arrival: DelayTime,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        MakeLiteral(NodeId, bool),
        MakeAnd(NodeId, NodeId, NodeId),
        AddNode(NodeId),
        Update(NodeId),
        Substitute(NodeId, NodeId, bool),
        Duplicate(NodeId, NodeId),
        ReplaceWithLiteral(NodeId, NodeId, bool),
        ReplaceWithNot(NodeId),
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
        next_node: usize,
        events: Vec<Event>,
        fail_substitute: bool,
    }

    impl TestBackend {
        fn new(root_literals: Vec<LiteralValue>, arrivals: &[f64]) -> Self {
            let root_fanins = (0..arrivals.len())
                .map(|index| NodeId(100 + index))
                .collect::<Vec<_>>();
            let mut nodes = BTreeMap::new();
            nodes.insert(
                NodeId(0),
                TestNode {
                    fanins: root_fanins,
                    literals: root_literals,
                    arrival: delay(0.0),
                },
            );
            for (index, arrival) in arrivals.iter().copied().enumerate() {
                nodes.insert(
                    NodeId(100 + index),
                    TestNode {
                        fanins: Vec::new(),
                        literals: Vec::new(),
                        arrival: delay(arrival),
                    },
                );
            }

            Self {
                nodes,
                next_node: 1,
                events: Vec::new(),
                fail_substitute: false,
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

        fn allocate(&mut self, node: TestNode) -> NodeId {
            let id = NodeId(self.next_node);
            self.next_node += 1;
            self.nodes.insert(id, node);
            id
        }
    }

    impl SpeedAndBackend for TestBackend {
        type Node = NodeId;
        type Error = TestError;

        fn literal_count(&self, node: &Self::Node) -> Result<usize, Self::Error> {
            Ok(self
                .node(*node)?
                .literals
                .iter()
                .filter(|literal| literal.phase().is_some())
                .count())
        }

        fn cube_literals(
            &self,
            node: &Self::Node,
            cube_index: usize,
        ) -> Result<Vec<LiteralValue>, Self::Error> {
            assert_eq!(cube_index, 0);
            Ok(self.node(*node)?.literals.clone())
        }

        fn fanins(&self, node: &Self::Node) -> Result<Vec<Self::Node>, Self::Error> {
            Ok(self.node(*node)?.fanins.clone())
        }

        fn arrival_time(&self, node: &Self::Node) -> Result<DelayTime, Self::Error> {
            Ok(self.node(*node)?.arrival)
        }

        fn make_literal(
            &mut self,
            fanin: &Self::Node,
            phase: bool,
        ) -> Result<Self::Node, Self::Error> {
            let literal = self.allocate(TestNode {
                fanins: vec![*fanin],
                literals: vec![if phase {
                    LiteralValue::One
                } else {
                    LiteralValue::Zero
                }],
                arrival: self.node(*fanin)?.arrival,
            });
            self.events.push(Event::MakeLiteral(*fanin, phase));
            Ok(literal)
        }

        fn make_and(
            &mut self,
            left: &Self::Node,
            right: &Self::Node,
        ) -> Result<Self::Node, Self::Error> {
            let left_node = self.node(*left)?.clone();
            let right_node = self.node(*right)?.clone();
            let arrival = delay(
                left_node
                    .arrival
                    .arrival()
                    .max(right_node.arrival.arrival()),
            );
            let node = self.allocate(TestNode {
                fanins: [left_node.fanins, right_node.fanins].concat(),
                literals: [left_node.literals, right_node.literals].concat(),
                arrival,
            });
            self.events.push(Event::MakeAnd(*left, *right, node));
            Ok(node)
        }

        fn add_node(&mut self, node: Self::Node) -> Result<(), Self::Error> {
            self.node(node)?;
            self.events.push(Event::AddNode(node));
            Ok(())
        }

        fn update_single_level(&mut self, node: &Self::Node) -> Result<DelayTime, Self::Error> {
            let arrival = self.node(*node)?.arrival;
            self.events.push(Event::Update(*node));
            Ok(arrival)
        }

        fn substitute(
            &mut self,
            node: &Self::Node,
            new_node: &Self::Node,
            positive_phase: bool,
        ) -> Result<bool, Self::Error> {
            self.events
                .push(Event::Substitute(*node, *new_node, positive_phase));
            if self.fail_substitute {
                return Ok(false);
            }

            let replacement = self.node(*new_node)?.clone();
            let selected = replacement
                .fanins
                .iter()
                .copied()
                .zip(replacement.literals.iter().copied())
                .collect::<BTreeSet<_>>();
            let root = self.node_mut(*node)?;
            let mut fanins = Vec::new();
            let mut literals = Vec::new();
            let mut replaced = false;

            for (fanin, literal) in root
                .fanins
                .iter()
                .copied()
                .zip(root.literals.iter().copied())
            {
                if selected.contains(&(fanin, literal)) {
                    if !replaced {
                        fanins.push(*new_node);
                        literals.push(LiteralValue::One);
                        replaced = true;
                    }
                } else {
                    fanins.push(fanin);
                    literals.push(literal);
                }
            }

            root.fanins = fanins;
            root.literals = literals;
            Ok(replaced)
        }

        fn replace_with_not(&mut self, node: &Self::Node) -> Result<(), Self::Error> {
            let root = self.node_mut(*node)?;
            for literal in &mut root.literals {
                *literal = match literal {
                    LiteralValue::Zero => LiteralValue::One,
                    LiteralValue::One => LiteralValue::Zero,
                    LiteralValue::DontCare => LiteralValue::DontCare,
                };
            }
            self.events.push(Event::ReplaceWithNot(*node));
            Ok(())
        }

        fn duplicate_node(&mut self, node: &Self::Node) -> Result<Self::Node, Self::Error> {
            let duplicate = self.allocate(self.node(*node)?.clone());
            self.events.push(Event::Duplicate(*node, duplicate));
            Ok(duplicate)
        }

        fn replace_with_literal(
            &mut self,
            node: &Self::Node,
            fanin: &Self::Node,
            phase: bool,
        ) -> Result<(), Self::Error> {
            let root = self.node_mut(*node)?;
            root.fanins = vec![*fanin];
            root.literals = vec![if phase {
                LiteralValue::One
            } else {
                LiteralValue::Zero
            }];
            self.events
                .push(Event::ReplaceWithLiteral(*node, *fanin, phase));
            Ok(())
        }
    }

    fn lit(fanin_index: usize, literal: LiteralValue, rise: f64, fall: f64) -> AndLiteral {
        AndLiteral {
            fanin_index,
            literal,
            arrival: DelayTime { rise, fall },
        }
    }

    fn delay(time: f64) -> DelayTime {
        DelayTime {
            rise: time,
            fall: time,
        }
    }

    #[test]
    fn selects_two_earliest_care_literals_by_max_arrival() {
        let selected = select_two_earliest_literals(&[
            lit(0, LiteralValue::One, 5.0, 2.0),
            lit(1, LiteralValue::DontCare, 1.0, 1.0),
            lit(2, LiteralValue::Zero, 3.0, 4.0),
            lit(3, LiteralValue::One, 1.0, 2.0),
        ]);

        assert_eq!(
            selected,
            vec![
                SelectedLiteral {
                    fanin_index: 3,
                    phase: true,
                },
                SelectedLiteral {
                    fanin_index: 2,
                    phase: false,
                },
            ]
        );
    }

    #[test]
    fn plans_recursive_substitution_for_more_than_two_literals() {
        let plan = plan_and_decomposition(
            &[
                lit(0, LiteralValue::One, 5.0, 2.0),
                lit(1, LiteralValue::Zero, 1.0, 1.0),
                lit(2, LiteralValue::One, 3.0, 3.0),
            ],
            false,
        )
        .unwrap();

        assert_eq!(
            plan,
            vec![
                AndDecompStep::CreateAndNode {
                    first: SelectedLiteral {
                        fanin_index: 1,
                        phase: false,
                    },
                    second: SelectedLiteral {
                        fanin_index: 2,
                        phase: true,
                    },
                },
                AndDecompStep::SingleLevelUpdate,
                AndDecompStep::SubstituteAndRecurse,
            ]
        );
    }

    #[test]
    fn recursive_decomposition_pairs_earliest_literals_and_updates_new_nodes() {
        let mut backend = TestBackend::new(
            vec![
                LiteralValue::One,
                LiteralValue::Zero,
                LiteralValue::One,
                LiteralValue::One,
            ],
            &[7.0, 1.0, 3.0, 5.0],
        );

        assert_eq!(
            speed_and_decompose(&mut backend, NodeId(0), SpeedAndOptions::new(false)),
            Ok(true)
        );

        assert_eq!(
            &backend.events[0..2],
            &[
                Event::MakeLiteral(NodeId(101), false),
                Event::MakeLiteral(NodeId(102), true)
            ]
        );
        assert!(
            backend
                .events
                .iter()
                .filter(|event| matches!(event, Event::Substitute(NodeId(0), _, true)))
                .count()
                >= 2
        );
        assert_eq!(
            backend.node(NodeId(0)).unwrap().literals,
            vec![LiteralValue::One, LiteralValue::One]
        );
    }

    #[test]
    fn inverted_two_literal_terminal_case_adds_explicit_inverter() {
        let mut backend =
            TestBackend::new(vec![LiteralValue::One, LiteralValue::Zero], &[1.0, 2.0]);

        assert_eq!(
            speed_and_decompose(&mut backend, NodeId(0), SpeedAndOptions::new(true)),
            Ok(true)
        );

        assert_eq!(
            backend.events,
            vec![
                Event::Duplicate(NodeId(0), NodeId(1)),
                Event::AddNode(NodeId(1)),
                Event::ReplaceWithLiteral(NodeId(0), NodeId(1), false),
                Event::Update(NodeId(1)),
                Event::Update(NodeId(0)),
            ]
        );
    }

    #[test]
    fn inverted_constant_and_single_literal_cases_replace_with_not() {
        for literals in [Vec::new(), vec![LiteralValue::One]] {
            let mut backend = TestBackend::new(literals, &[1.0]);

            assert_eq!(
                speed_and_decompose(&mut backend, NodeId(0), SpeedAndOptions::new(true)),
                Ok(true)
            );

            assert!(backend.events.contains(&Event::ReplaceWithNot(NodeId(0))));
            assert!(backend.events.contains(&Event::Update(NodeId(0))));
        }
    }

    #[test]
    fn reports_substitution_failure_and_shape_errors() {
        let mut backend = TestBackend::new(
            vec![LiteralValue::One, LiteralValue::Zero, LiteralValue::One],
            &[1.0, 2.0, 3.0],
        );
        backend.fail_substitute = true;

        assert_eq!(
            speed_and_decompose(&mut backend, NodeId(0), SpeedAndOptions::new(false)),
            Err(SpeedAndError::SubstituteFailed)
        );

        let mut backend = TestBackend::new(vec![LiteralValue::One, LiteralValue::Zero], &[1.0]);
        assert_eq!(
            speed_and_decompose(&mut backend, NodeId(0), SpeedAndOptions::new(false)),
            Err(SpeedAndError::CubeFaninMismatch {
                literals: 2,
                fanins: 1,
            })
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let text = include_str!("speed_and.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
    }
}
