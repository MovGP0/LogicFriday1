//! Native phase-inverter insertion.
//!
//! The original routine walks a SIS network in depth-first order. For each
//! node it looks for fanout logic that uses the node in positive phase, creates
//! or reuses a shared inverter fanout, and rewrites those positive literals as
//! a complemented use of that inverter. This module models the same rewrite on
//! an owned sum-of-products graph.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PhaseNodeId(pub usize);

impl PhaseNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiteralPhase {
    Positive,
    Negative,
}

impl LiteralPhase {
    pub fn inverted(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PhaseLiteral {
    pub node: PhaseNodeId,
    pub phase: LiteralPhase,
}

impl PhaseLiteral {
    pub fn positive(node: PhaseNodeId) -> Self {
        Self {
            node,
            phase: LiteralPhase::Positive,
        }
    }

    pub fn negative(node: PhaseNodeId) -> Self {
        Self {
            node,
            phase: LiteralPhase::Negative,
        }
    }

    pub fn inverted(self) -> Self {
        Self {
            node: self.node,
            phase: self.phase.inverted(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhaseCube {
    literals: Vec<PhaseLiteral>,
}

impl PhaseCube {
    pub fn one() -> Self {
        Self {
            literals: Vec::new(),
        }
    }

    pub fn from_literals(literals: impl IntoIterator<Item = PhaseLiteral>) -> Option<Self> {
        let mut normalized = literals.into_iter().collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();

        for literal in &normalized {
            if normalized.binary_search(&literal.inverted()).is_ok() {
                return None;
            }
        }

        Some(Self {
            literals: normalized,
        })
    }

    pub fn literals(&self) -> &[PhaseLiteral] {
        &self.literals
    }

    fn contains(&self, literal: PhaseLiteral) -> bool {
        self.literals.binary_search(&literal).is_ok()
    }

    fn without(&self, literal: PhaseLiteral) -> Self {
        let literals = self
            .literals
            .iter()
            .copied()
            .filter(|item| *item != literal)
            .collect::<Vec<_>>();

        Self { literals }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhaseCover {
    cubes: Vec<PhaseCube>,
}

impl PhaseCover {
    pub fn zero() -> Self {
        Self { cubes: Vec::new() }
    }

    pub fn one() -> Self {
        Self {
            cubes: vec![PhaseCube::one()],
        }
    }

    pub fn literal(literal: PhaseLiteral) -> Self {
        Self::from_cubes([PhaseCube::from_literals([literal]).expect("single literal is valid")])
    }

    pub fn from_cubes(cubes: impl IntoIterator<Item = PhaseCube>) -> Self {
        let mut normalized = cubes.into_iter().collect::<Vec<_>>();
        normalized.sort_by(|left, right| left.literals.cmp(&right.literals));
        normalized.dedup();

        Self { cubes: normalized }
    }

    pub fn cubes(&self) -> &[PhaseCube] {
        &self.cubes
    }

    pub fn is_zero(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn or(&self, other: &Self) -> Self {
        Self::from_cubes(
            self.cubes
                .iter()
                .cloned()
                .chain(other.cubes.iter().cloned()),
        )
    }

    pub fn and(&self, other: &Self) -> Self {
        let mut cubes = Vec::new();

        for left in &self.cubes {
            for right in &other.cubes {
                let merged = left
                    .literals
                    .iter()
                    .copied()
                    .chain(right.literals.iter().copied());

                if let Some(cube) = PhaseCube::from_literals(merged) {
                    cubes.push(cube);
                }
            }
        }

        Self::from_cubes(cubes)
    }

    fn divide_by_literal(&self, literal: PhaseLiteral) -> (Self, Self) {
        let mut quotient = Vec::new();
        let mut remainder = Vec::new();

        for cube in &self.cubes {
            if cube.contains(literal) {
                quotient.push(cube.without(literal));
            } else {
                remainder.push(cube.clone());
            }
        }

        (Self::from_cubes(quotient), Self::from_cubes(remainder))
    }

    fn referenced_nodes(&self) -> BTreeSet<PhaseNodeId> {
        self.cubes
            .iter()
            .flat_map(|cube| cube.literals.iter().map(|literal| literal.node))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
    Inverter,
    Constant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseNode {
    pub name: String,
    pub kind: PhaseNodeKind,
    pub function: PhaseCover,
}

impl PhaseNode {
    pub fn new(name: impl Into<String>, kind: PhaseNodeKind, function: PhaseCover) -> Self {
        Self {
            name: name.into(),
            kind,
            function,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhaseNetwork {
    nodes: Vec<PhaseNode>,
}

impl PhaseNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: PhaseNode) -> PhaseNodeId {
        let id = PhaseNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> PhaseNodeId {
        self.add_node(PhaseNode::new(
            name,
            PhaseNodeKind::PrimaryInput,
            PhaseCover::one(),
        ))
    }

    pub fn add_primary_output(
        &mut self,
        name: impl Into<String>,
        fanin: PhaseNodeId,
    ) -> PhaseNodeId {
        self.add_node(PhaseNode::new(
            name,
            PhaseNodeKind::PrimaryOutput,
            PhaseCover::literal(PhaseLiteral::positive(fanin)),
        ))
    }

    pub fn add_internal(&mut self, name: impl Into<String>, function: PhaseCover) -> PhaseNodeId {
        self.add_node(PhaseNode::new(name, PhaseNodeKind::Internal, function))
    }

    pub fn add_constant(&mut self, name: impl Into<String>, value: bool) -> PhaseNodeId {
        let function = if value {
            PhaseCover::one()
        } else {
            PhaseCover::zero()
        };

        self.add_node(PhaseNode::new(name, PhaseNodeKind::Constant, function))
    }

    pub fn add_inverter(&mut self, fanin: PhaseNodeId) -> PhaseNodeId {
        let name = format!("{}_inv{}", self.node_name(fanin), self.nodes.len());
        self.add_node(PhaseNode::new(
            name,
            PhaseNodeKind::Inverter,
            PhaseCover::literal(PhaseLiteral::negative(fanin)),
        ))
    }

    pub fn node(&self, id: PhaseNodeId) -> Option<&PhaseNode> {
        self.nodes.get(id.index())
    }

    pub fn nodes(&self) -> &[PhaseNode] {
        &self.nodes
    }

    pub fn fanouts(&self, node: PhaseNodeId) -> Result<Vec<PhaseNodeId>, PhaseError> {
        self.ensure_node(node)?;

        Ok(self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate.function.referenced_nodes().contains(&node))
            .map(|(index, _)| PhaseNodeId(index))
            .collect())
    }

    fn replace_function(
        &mut self,
        node: PhaseNodeId,
        function: PhaseCover,
    ) -> Result<(), PhaseError> {
        self.validate_cover(&function)?;

        let item = self
            .nodes
            .get_mut(node.index())
            .ok_or(PhaseError::MissingNode { node })?;
        item.function = function;
        Ok(())
    }

    fn ensure_node(&self, node: PhaseNodeId) -> Result<(), PhaseError> {
        self.node(node)
            .map(|_| ())
            .ok_or(PhaseError::MissingNode { node })
    }

    fn validate(&self) -> Result<(), PhaseError> {
        for node in &self.nodes {
            self.validate_cover(&node.function)?;
        }

        Ok(())
    }

    fn validate_cover(&self, cover: &PhaseCover) -> Result<(), PhaseError> {
        for referenced in cover.referenced_nodes() {
            self.ensure_node(referenced)?;
        }

        Ok(())
    }

    fn node_name(&self, node: PhaseNodeId) -> &str {
        self.nodes
            .get(node.index())
            .map(|node| node.name.as_str())
            .unwrap_or("node")
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhaseInsertReport {
    pub changed_nodes: Vec<PhaseNodeId>,
    pub created_inverters: Vec<PhaseNodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhaseError {
    MissingNode { node: PhaseNodeId },
    CycleDetected { node: PhaseNodeId },
}

impl fmt::Display for PhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode { node } => {
                write!(f, "phase network references missing node {}", node.index())
            }
            Self::CycleDetected { node } => {
                write!(
                    f,
                    "phase network contains a cycle through node {}",
                    node.index()
                )
            }
        }
    }
}

impl Error for PhaseError {}

pub fn add_inverters_to_network(
    network: &mut PhaseNetwork,
) -> Result<PhaseInsertReport, PhaseError> {
    network.validate()?;
    let mut report = PhaseInsertReport::default();
    let order = network_dfs(network)?;

    for node in order {
        if add_inverter_to_node(network, node, &mut report)? {
            report.changed_nodes.push(node);
        }
    }

    Ok(report)
}

pub fn add_inverter_to_node(
    network: &mut PhaseNetwork,
    node: PhaseNodeId,
    report: &mut PhaseInsertReport,
) -> Result<bool, PhaseError> {
    network.ensure_node(node)?;

    let positive_literal = PhaseLiteral::positive(node);
    let inverter = find_existing_inverter(network, node)?;
    let fanouts = network.fanouts(node)?;
    let mut inverter = inverter;
    let mut changed = false;

    for fanout in fanouts {
        let fanout_node = network
            .node(fanout)
            .ok_or(PhaseError::MissingNode { node: fanout })?;

        if fanout_node.kind == PhaseNodeKind::PrimaryOutput {
            continue;
        }

        let (quotient, remainder) = fanout_node.function.divide_by_literal(positive_literal);

        if quotient.is_zero() {
            continue;
        }

        let inverter = match inverter {
            Some(id) => id,
            None => {
                let id = network.add_inverter(node);
                report.created_inverters.push(id);
                inverter = Some(id);
                id
            }
        };

        let inverter_as_original_phase = PhaseCover::literal(PhaseLiteral::negative(inverter));
        let replacement = quotient.and(&inverter_as_original_phase).or(&remainder);
        network.replace_function(fanout, replacement)?;
        changed = true;
    }

    Ok(changed)
}

fn find_existing_inverter(
    network: &PhaseNetwork,
    node: PhaseNodeId,
) -> Result<Option<PhaseNodeId>, PhaseError> {
    for fanout in network.fanouts(node)? {
        let fanout_node = network
            .node(fanout)
            .ok_or(PhaseError::MissingNode { node: fanout })?;

        if fanout_node.kind == PhaseNodeKind::Inverter {
            return Ok(Some(fanout));
        }
    }

    Ok(None)
}

fn network_dfs(network: &PhaseNetwork) -> Result<Vec<PhaseNodeId>, PhaseError> {
    let mut roots = network
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            (node.kind == PhaseNodeKind::PrimaryOutput).then_some(PhaseNodeId(index))
        })
        .collect::<Vec<_>>();

    roots.extend(
        network
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, _)| {
                let id = PhaseNodeId(index);
                network
                    .fanouts(id)
                    .is_ok_and(|fanouts| fanouts.is_empty())
                    .then_some(id)
            })
            .filter(|id| {
                network
                    .node(*id)
                    .is_some_and(|node| node.kind != PhaseNodeKind::PrimaryOutput)
            }),
    );

    let mut visited = Vec::new();
    let mut active = BTreeSet::new();
    let mut done = BTreeSet::new();

    for root in roots {
        dfs_visit(network, root, &mut active, &mut done, &mut visited)?;
    }

    Ok(visited)
}

fn dfs_visit(
    network: &PhaseNetwork,
    node: PhaseNodeId,
    active: &mut BTreeSet<PhaseNodeId>,
    done: &mut BTreeSet<PhaseNodeId>,
    visited: &mut Vec<PhaseNodeId>,
) -> Result<(), PhaseError> {
    network.ensure_node(node)?;

    if done.contains(&node) {
        return Ok(());
    }

    if !active.insert(node) {
        return Err(PhaseError::CycleDetected { node });
    }

    let fanins = network
        .node(node)
        .ok_or(PhaseError::MissingNode { node })?
        .function
        .referenced_nodes()
        .into_iter()
        .collect::<Vec<_>>();

    for fanin in fanins {
        dfs_visit(network, fanin, active, done, visited)?;
    }

    active.remove(&node);
    done.insert(node);
    visited.push(node);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cover(cubes: &[&[PhaseLiteral]]) -> PhaseCover {
        PhaseCover::from_cubes(
            cubes
                .iter()
                .filter_map(|cube| PhaseCube::from_literals(cube.iter().copied())),
        )
    }

    #[test]
    fn inserts_shared_inverter_and_rewrites_positive_fanout_literals() {
        let mut network = PhaseNetwork::new();
        let f = network.add_primary_input("f");
        let a = network.add_primary_input("a");
        let g = network.add_internal(
            "g",
            cover(&[
                &[PhaseLiteral::positive(f), PhaseLiteral::positive(a)],
                &[PhaseLiteral::negative(a)],
            ]),
        );
        network.add_primary_output("out", g);

        let mut report = PhaseInsertReport::default();
        let changed = add_inverter_to_node(&mut network, f, &mut report).unwrap();

        assert!(changed);
        assert_eq!(report.created_inverters.len(), 1);
        let inverter = report.created_inverters[0];
        assert_eq!(
            network.node(inverter).unwrap().function,
            PhaseCover::literal(PhaseLiteral::negative(f))
        );
        assert_eq!(
            network.node(g).unwrap().function,
            cover(&[
                &[PhaseLiteral::positive(a), PhaseLiteral::negative(inverter)],
                &[PhaseLiteral::negative(a)],
            ])
        );
    }

    #[test]
    fn reuses_existing_inverter_fanout() {
        let mut network = PhaseNetwork::new();
        let f = network.add_primary_input("f");
        let existing = network.add_inverter(f);
        let g = network.add_internal("g", PhaseCover::literal(PhaseLiteral::positive(f)));
        network.add_primary_output("out", g);

        let report = add_inverters_to_network(&mut network).unwrap();

        assert!(report.created_inverters.is_empty());
        assert_eq!(
            network.node(g).unwrap().function,
            PhaseCover::literal(PhaseLiteral::negative(existing))
        );
    }

    #[test]
    fn skips_primary_output_fanouts() {
        let mut network = PhaseNetwork::new();
        let f = network.add_primary_input("f");
        let out = network.add_primary_output("out", f);

        let report = add_inverters_to_network(&mut network).unwrap();

        assert!(report.created_inverters.is_empty());
        assert_eq!(
            network.node(out).unwrap().function,
            PhaseCover::literal(PhaseLiteral::positive(f))
        );
    }

    #[test]
    fn leaves_negative_occurrences_of_rewritten_node_unchanged() {
        let mut network = PhaseNetwork::new();
        let f = network.add_primary_input("f");
        let g = network.add_internal(
            "g",
            cover(&[&[PhaseLiteral::positive(f)], &[PhaseLiteral::negative(f)]]),
        );
        network.add_primary_output("out", g);

        let report = add_inverters_to_network(&mut network).unwrap();
        let inverter = report.created_inverters[0];

        assert_eq!(
            network.node(g).unwrap().function,
            cover(&[
                &[PhaseLiteral::negative(inverter)],
                &[PhaseLiteral::negative(f)],
            ])
        );
    }

    #[test]
    fn reports_missing_referenced_nodes() {
        let mut network = PhaseNetwork::new();
        network.add_internal(
            "bad",
            PhaseCover::literal(PhaseLiteral::positive(PhaseNodeId(9))),
        );

        assert!(matches!(
            add_inverters_to_network(&mut network),
            Err(PhaseError::MissingNode { .. })
        ));
    }

    #[test]
    fn reports_cycles() {
        let mut network = PhaseNetwork::new();
        let a = network.add_internal("a", PhaseCover::zero());
        let b = network.add_internal("b", PhaseCover::literal(PhaseLiteral::positive(a)));
        network
            .replace_function(a, PhaseCover::literal(PhaseLiteral::positive(b)))
            .unwrap();
        network.add_primary_output("out", a);

        assert!(matches!(
            add_inverters_to_network(&mut network),
            Err(PhaseError::CycleDetected { .. })
        ));
    }
}
