//! Native Rust library-preparation pass for SIS-style Boolean networks.
//!
//! The pass mirrors the legacy library-package preparation flow with owned
//! Rust data: collapse internal nodes to primary-input support, minimize the
//! resulting on-set into prime implicants, then rewrite every internal node to
//! use the full primary-input list in primary-input order.

use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    fn from_bool(value: bool) -> Self {
        if value { Self::One } else { Self::Zero }
    }

    fn required_value(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            Self::DontCare => None,
        }
    }

    fn matches_value(self, value: bool) -> bool {
        self.required_value()
            .map_or(true, |required| required == value)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }

    pub fn one(input_count: usize) -> Self {
        Self {
            literals: vec![Literal::DontCare; input_count],
        }
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    fn from_assignment(assignment: &[bool]) -> Self {
        Self::new(assignment.iter().copied().map(Literal::from_bool).collect())
    }

    fn is_superset_of(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| *left == Literal::DontCare || left == right)
    }

    fn matches_assignment(&self, assignment: &[bool]) -> bool {
        self.literals
            .iter()
            .zip(assignment)
            .all(|(literal, value)| literal.matches_value(*value))
    }

    fn merge_candidate(&self, other: &Self) -> Option<Self> {
        let mut difference = None;
        let mut literals = Vec::with_capacity(self.literals.len());

        for (index, (left, right)) in self.literals.iter().zip(&other.literals).enumerate() {
            if left == right {
                literals.push(*left);
                continue;
            }

            if *left == Literal::DontCare || *right == Literal::DontCare || difference.is_some() {
                return None;
            }

            difference = Some(index);
            literals.push(Literal::DontCare);
        }

        difference.map(|_| Self::new(literals))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub cover: Vec<Cube>,
}

impl LibNode {
    pub fn primary_input(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            kind: NodeKind::PrimaryInput,
            fanins: Vec::new(),
            cover: Vec::new(),
        }
    }

    pub fn primary_output(id: NodeId, name: impl Into<String>, fanin: NodeId) -> Self {
        Self {
            id,
            name: name.into(),
            kind: NodeKind::PrimaryOutput,
            fanins: vec![fanin],
            cover: Vec::new(),
        }
    }

    pub fn internal(
        id: NodeId,
        name: impl Into<String>,
        fanins: Vec<NodeId>,
        cover: Vec<Cube>,
    ) -> LibHackResult<Self> {
        let node = Self {
            id,
            name: name.into(),
            kind: NodeKind::Internal,
            fanins,
            cover,
        };
        node.validate_cover()?;
        Ok(node)
    }

    fn validate_cover(&self) -> LibHackResult<()> {
        for cube in &self.cover {
            if cube.literals.len() != self.fanins.len() {
                return Err(LibHackError::CoverArityMismatch {
                    node: self.id,
                    fanins: self.fanins.len(),
                    literals: cube.literals.len(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibNetwork {
    nodes: Vec<LibNode>,
    primary_inputs: Vec<NodeId>,
}

impl LibNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: LibNode) -> LibHackResult<NodeId> {
        node.validate_cover()?;
        if node.id.0 != self.nodes.len() {
            return Err(LibHackError::UnexpectedNodeId {
                expected: NodeId(self.nodes.len()),
                actual: node.id,
            });
        }

        for fanin in &node.fanins {
            self.node(*fanin)?;
        }

        if node.kind == NodeKind::PrimaryInput {
            self.primary_inputs.push(node.id);
        }

        let id = node.id;
        self.nodes.push(node);
        Ok(id)
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> LibHackResult<NodeId> {
        let id = NodeId(self.nodes.len());
        self.add_node(LibNode::primary_input(id, name))
    }

    pub fn add_internal(
        &mut self,
        name: impl Into<String>,
        fanins: Vec<NodeId>,
        cover: Vec<Cube>,
    ) -> LibHackResult<NodeId> {
        let id = NodeId(self.nodes.len());
        self.add_node(LibNode::internal(id, name, fanins, cover)?)
    }

    pub fn add_primary_output(
        &mut self,
        name: impl Into<String>,
        fanin: NodeId,
    ) -> LibHackResult<NodeId> {
        let id = NodeId(self.nodes.len());
        self.add_node(LibNode::primary_output(id, name, fanin))
    }

    pub fn node(&self, id: NodeId) -> LibHackResult<&LibNode> {
        self.nodes.get(id.0).ok_or(LibHackError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> LibHackResult<&mut LibNode> {
        self.nodes
            .get_mut(id.0)
            .ok_or(LibHackError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[LibNode] {
        &self.nodes
    }

    pub fn primary_inputs(&self) -> &[NodeId] {
        &self.primary_inputs
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibHackSummary {
    pub collapsed_internal_nodes: usize,
    pub minimized_internal_nodes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LibHackError {
    CoverArityMismatch {
        node: NodeId,
        fanins: usize,
        literals: usize,
    },
    UnexpectedNodeId {
        expected: NodeId,
        actual: NodeId,
    },
    UnknownNode(NodeId),
    UnknownFanin(NodeId),
    CombinationalCycle(NodeId),
}

impl fmt::Display for LibHackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoverArityMismatch {
                node,
                fanins,
                literals,
            } => write!(
                f,
                "node {:?} has {fanins} fanins but a cube with {literals} literals",
                node
            ),
            Self::UnexpectedNodeId { expected, actual } => {
                write!(f, "expected node id {:?}, got {:?}", expected, actual)
            }
            Self::UnknownNode(node) => write!(f, "unknown node {:?}", node),
            Self::UnknownFanin(node) => write!(f, "unknown fanin {:?}", node),
            Self::CombinationalCycle(node) => {
                write!(f, "combinational cycle reaches node {:?}", node)
            }
        }
    }
}

impl Error for LibHackError {}

pub type LibHackResult<T> = Result<T, LibHackError>;

pub fn process_library_network(network: &mut LibNetwork) -> LibHackResult<LibHackSummary> {
    let primary_inputs = network.primary_inputs.clone();
    let mut summary = LibHackSummary::default();
    let internal_nodes = network
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Internal)
        .map(|node| node.id)
        .collect::<Vec<_>>();

    for node in internal_nodes {
        let on_set = collapsed_on_set(network, node, &primary_inputs)?;
        let cover = prime_cover(primary_inputs.len(), &on_set);
        let target = network.node_mut(node)?;
        target.fanins = primary_inputs.clone();
        target.cover = cover;
        summary.collapsed_internal_nodes += 1;
        summary.minimized_internal_nodes += 1;
    }

    Ok(summary)
}

pub fn evaluate_node(
    network: &LibNetwork,
    node: NodeId,
    primary_input_values: &[(NodeId, bool)],
) -> LibHackResult<bool> {
    let assignment = primary_input_values
        .iter()
        .map(|(id, value)| (*id, *value))
        .collect::<Vec<_>>();
    let mut visiting = BTreeSet::new();
    evaluate_node_inner(network, node, &assignment, &mut visiting)
}

fn collapsed_on_set(
    network: &LibNetwork,
    node: NodeId,
    primary_inputs: &[NodeId],
) -> LibHackResult<HashSet<Vec<bool>>> {
    let mut on_set = HashSet::new();
    let mut assignment = vec![false; primary_inputs.len()];

    visit_assignments(0, &mut assignment, &mut |values| {
        let input_values = primary_inputs
            .iter()
            .copied()
            .zip(values.iter().copied())
            .collect::<Vec<_>>();
        if evaluate_node(network, node, &input_values)? {
            on_set.insert(values.to_vec());
        }
        Ok(())
    })?;

    Ok(on_set)
}

fn evaluate_node_inner(
    network: &LibNetwork,
    node: NodeId,
    primary_input_values: &[(NodeId, bool)],
    visiting: &mut BTreeSet<NodeId>,
) -> LibHackResult<bool> {
    let node_ref = network.node(node)?;
    match node_ref.kind {
        NodeKind::PrimaryInput => primary_input_values
            .iter()
            .find(|(id, _)| *id == node)
            .map(|(_, value)| *value)
            .ok_or(LibHackError::UnknownFanin(node)),
        NodeKind::PrimaryOutput => {
            let Some(fanin) = node_ref.fanins.first() else {
                return Ok(false);
            };
            evaluate_node_inner(network, *fanin, primary_input_values, visiting)
        }
        NodeKind::Internal => {
            if !visiting.insert(node) {
                return Err(LibHackError::CombinationalCycle(node));
            }

            let values = node_ref
                .fanins
                .iter()
                .map(|fanin| evaluate_node_inner(network, *fanin, primary_input_values, visiting))
                .collect::<LibHackResult<Vec<_>>>()?;
            let value = evaluate_cover(&node_ref.cover, &values);
            visiting.remove(&node);
            Ok(value)
        }
    }
}

fn evaluate_cover(cover: &[Cube], assignment: &[bool]) -> bool {
    cover.iter().any(|cube| cube.matches_assignment(assignment))
}

fn prime_cover(input_count: usize, on_set: &HashSet<Vec<bool>>) -> Vec<Cube> {
    if on_set.is_empty() {
        return Vec::new();
    }

    if Some(on_set.len()) == 1usize.checked_shl(input_count as u32) {
        return vec![Cube::one(input_count)];
    }

    let mut current = on_set
        .iter()
        .map(|assignment| Cube::from_assignment(assignment))
        .collect::<BTreeSet<_>>();
    let mut primes = BTreeSet::new();

    loop {
        let mut used = BTreeSet::new();
        let mut next = BTreeSet::new();
        let current_vec = current.iter().cloned().collect::<Vec<_>>();

        for left_index in 0..current_vec.len() {
            for right_index in (left_index + 1)..current_vec.len() {
                let Some(merged) =
                    current_vec[left_index].merge_candidate(&current_vec[right_index])
                else {
                    continue;
                };

                if cube_covers_only_on_set(&merged, input_count, on_set) {
                    used.insert(current_vec[left_index].clone());
                    used.insert(current_vec[right_index].clone());
                    next.insert(merged);
                }
            }
        }

        for cube in &current {
            if !used.contains(cube) {
                primes.insert(cube.clone());
            }
        }

        if next.is_empty() {
            break;
        }

        current = next;
    }

    contain(primes.into_iter().collect())
}

fn cube_covers_only_on_set(cube: &Cube, input_count: usize, on_set: &HashSet<Vec<bool>>) -> bool {
    let mut valid = true;
    let mut assignment = vec![false; input_count];
    let _ = visit_assignments(0, &mut assignment, &mut |values| {
        if cube.matches_assignment(values) && !on_set.contains(values) {
            valid = false;
        }
        Ok(())
    });

    valid
}

fn contain(mut cover: Vec<Cube>) -> Vec<Cube> {
    cover.sort();
    let mut result: Vec<Cube> = Vec::new();

    'candidate: for candidate in cover {
        for other in &result {
            if other.is_superset_of(&candidate) {
                continue 'candidate;
            }
        }

        result.retain(|other: &Cube| !candidate.is_superset_of(other));
        result.push(candidate);
    }

    result
}

fn visit_assignments<F>(index: usize, assignment: &mut [bool], visit: &mut F) -> LibHackResult<()>
where
    F: FnMut(&[bool]) -> LibHackResult<()>,
{
    if index == assignment.len() {
        return visit(assignment);
    }

    assignment[index] = false;
    visit_assignments(index + 1, assignment, visit)?;
    assignment[index] = true;
    visit_assignments(index + 1, assignment, visit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(literals: &[Literal]) -> Cube {
        Cube::new(literals.to_vec())
    }

    fn values(values: &[(NodeId, bool)]) -> Vec<(NodeId, bool)> {
        values.to_vec()
    }

    #[test]
    fn process_rewrites_internal_nodes_to_full_primary_input_order() {
        let mut network = LibNetwork::new();
        let a = network.add_primary_input("a").unwrap();
        let b = network.add_primary_input("b").unwrap();
        let c = network.add_primary_input("c").unwrap();
        let n = network
            .add_internal("n", vec![b], vec![cube(&[Literal::One])])
            .unwrap();
        network
            .add_internal(
                "out",
                vec![n, a],
                vec![cube(&[Literal::One, Literal::Zero])],
            )
            .unwrap();
        network.add_primary_output("po", n).unwrap();

        let summary = process_library_network(&mut network).unwrap();

        assert_eq!(summary.collapsed_internal_nodes, 2);
        assert_eq!(network.node(n).unwrap().fanins, vec![a, b, c]);
        assert_eq!(
            network.node(n).unwrap().cover,
            vec![cube(&[Literal::DontCare, Literal::One, Literal::DontCare,])]
        );
    }

    #[test]
    fn process_collapses_internal_fanins_before_adjusting_support() {
        let mut network = LibNetwork::new();
        let a = network.add_primary_input("a").unwrap();
        let b = network.add_primary_input("b").unwrap();
        let and = network
            .add_internal("and", vec![a, b], vec![cube(&[Literal::One, Literal::One])])
            .unwrap();
        let not_and = network
            .add_internal("not_and", vec![and], vec![cube(&[Literal::Zero])])
            .unwrap();

        process_library_network(&mut network).unwrap();

        assert_eq!(network.node(not_and).unwrap().fanins, vec![a, b]);
        assert_eq!(
            evaluate_node(&network, not_and, &values(&[(a, true), (b, true)])).unwrap(),
            false
        );
        assert_eq!(
            evaluate_node(&network, not_and, &values(&[(a, true), (b, false)])).unwrap(),
            true
        );
        assert_eq!(
            evaluate_node(&network, not_and, &values(&[(a, false), (b, true)])).unwrap(),
            true
        );
    }

    #[test]
    fn process_reduces_adjacent_minterms_to_prime_implicant() {
        let mut network = LibNetwork::new();
        let a = network.add_primary_input("a").unwrap();
        let b = network.add_primary_input("b").unwrap();
        let node = network
            .add_internal(
                "f",
                vec![a, b],
                vec![
                    cube(&[Literal::One, Literal::One]),
                    cube(&[Literal::One, Literal::Zero]),
                ],
            )
            .unwrap();

        process_library_network(&mut network).unwrap();

        assert_eq!(
            network.node(node).unwrap().cover,
            vec![cube(&[Literal::One, Literal::DontCare])]
        );
    }

    #[test]
    fn evaluate_reports_combinational_cycles() {
        let mut network = LibNetwork::new();
        let a = network.add_primary_input("a").unwrap();
        let n = network
            .add_internal("n", vec![a], vec![cube(&[Literal::One])])
            .unwrap();
        network.node_mut(n).unwrap().fanins = vec![n];

        assert_eq!(
            process_library_network(&mut network),
            Err(LibHackError::CombinationalCycle(n))
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_tokens_are_present() {
        let text = include_str!("libhack.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
    }
}
