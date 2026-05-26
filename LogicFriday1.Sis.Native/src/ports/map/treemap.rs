//! Bounded native Rust tree mapper for `sis/map/treemap.c`.
//!
//! Full SIS tree mapping still depends on native ports for phase-aware genlib
//! matching, top-down commitment, and network replacement. This module keeps the
//! native owned-data core available now: enumerate candidate library matches on
//! the mapper tree model, price each match, and select a deterministic
//! lowest-cost cover.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use super::library::{GenlibGate, GenlibLibrary};
use super::tree::{
    MapperTree, MapperTreeError, MapperTreeNode, MapperTreeNodeId, PrimitiveGateKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TreeMapLimits {
    pub max_candidates: usize,
    pub max_pattern_nodes: usize,
    pub max_cover_nodes: usize,
    pub max_name_length: usize,
}

impl Default for TreeMapLimits {
    fn default() -> Self {
        Self {
            max_candidates: 16_384,
            max_pattern_nodes: 256,
            max_cover_nodes: 16_384,
            max_name_length: 256,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreePattern {
    Boundary(String),
    Gate {
        kind: PrimitiveGateKind,
        fanins: Vec<TreePattern>,
    },
}

impl TreePattern {
    pub fn boundary(name: impl Into<String>) -> Self {
        Self::Boundary(name.into())
    }

    pub fn gate(kind: PrimitiveGateKind, fanins: Vec<TreePattern>) -> Self {
        Self::Gate { kind, fanins }
    }

    fn node_count(&self) -> usize {
        match self {
            Self::Boundary(_) => 1,
            Self::Gate { fanins, .. } => 1 + fanins.iter().map(Self::node_count).sum::<usize>(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateGate {
    pub name: String,
    pub pattern: TreePattern,
    pub cost: f64,
}

impl CandidateGate {
    pub fn new(
        name: impl Into<String>,
        pattern: TreePattern,
        cost: f64,
    ) -> Result<Self, TreeMapError> {
        let candidate = Self {
            name: name.into(),
            pattern,
            cost,
        };
        candidate.validate(TreeMapLimits::default())?;
        Ok(candidate)
    }

    fn validate(&self, limits: TreeMapLimits) -> Result<(), TreeMapError> {
        validate_name(&self.name, "candidate gate", limits.max_name_length)?;
        if !self.cost.is_finite() || self.cost < 0.0 {
            return Err(TreeMapError::InvalidCost {
                gate: self.name.clone(),
                cost: self.cost,
            });
        }
        if self.pattern.node_count() > limits.max_pattern_nodes {
            return Err(TreeMapError::PatternTooLarge {
                gate: self.name.clone(),
                max: limits.max_pattern_nodes,
            });
        }
        validate_pattern(&self.name, &self.pattern, limits)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CandidateLibrary {
    gates: Vec<CandidateGate>,
}

impl CandidateLibrary {
    pub fn new(gates: Vec<CandidateGate>) -> Result<Self, TreeMapError> {
        Self::with_limits(gates, TreeMapLimits::default())
    }

    pub fn with_limits(
        gates: Vec<CandidateGate>,
        limits: TreeMapLimits,
    ) -> Result<Self, TreeMapError> {
        if gates.len() > limits.max_candidates {
            return Err(TreeMapError::TooManyCandidates {
                max: limits.max_candidates,
            });
        }
        for gate in &gates {
            gate.validate(limits)?;
        }
        Ok(Self { gates })
    }

    pub fn from_genlib(library: &GenlibLibrary) -> Result<Self, TreeMapError> {
        let gates = library
            .gates
            .iter()
            .filter_map(classify_genlib_gate)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(gates)
    }

    pub fn gates(&self) -> &[CandidateGate] {
        &self.gates
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateMatch {
    pub gate: String,
    pub root: MapperTreeNodeId,
    pub frontier: Vec<MapperTreeNodeId>,
    pub local_cost: f64,
    pub total_cost: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoverNode {
    pub root: MapperTreeNodeId,
    pub gate: String,
    pub frontier: Vec<MapperTreeNodeId>,
    pub local_cost: f64,
    pub total_cost: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeCover {
    pub root: MapperTreeNodeId,
    pub total_cost: f64,
    pub nodes: Vec<CoverNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TreeMapError {
    Tree(MapperTreeError),
    MissingSisPorts {
        operation: &'static str,
    },
    TooManyCandidates {
        max: usize,
    },
    PatternTooLarge {
        gate: String,
        max: usize,
    },
    TooManyCoverNodes {
        max: usize,
    },
    EmptyName {
        kind: &'static str,
    },
    NameTooLong {
        kind: &'static str,
        name: String,
        max_name_length: usize,
    },
    InvalidPattern {
        gate: String,
        reason: &'static str,
    },
    InvalidCost {
        gate: String,
        cost: f64,
    },
    UncoverableNode {
        node: MapperTreeNodeId,
    },
}

impl fmt::Display for TreeMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tree(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => write!(f, "{operation} requires unavailable native SIS integration"),
            Self::TooManyCandidates { max } => write!(f, "too many candidate gates; max is {max}"),
            Self::PatternTooLarge { gate, max } => {
                write!(f, "candidate gate '{gate}' pattern exceeds {max} nodes")
            }
            Self::TooManyCoverNodes { max } => {
                write!(f, "too many selected cover nodes; max is {max}")
            }
            Self::EmptyName { kind } => write!(f, "{kind} name cannot be empty"),
            Self::NameTooLong {
                kind,
                name,
                max_name_length,
            } => write!(
                f,
                "{kind} name '{name}' exceeds {max_name_length} characters"
            ),
            Self::InvalidPattern { gate, reason } => {
                write!(f, "candidate gate '{gate}' has invalid pattern: {reason}")
            }
            Self::InvalidCost { gate, cost } => {
                write!(f, "candidate gate '{gate}' has invalid cost {cost}")
            }
            Self::UncoverableNode { node } => {
                write!(f, "tree node {} cannot be covered", node.index())
            }
        }
    }
}

impl Error for TreeMapError {}

impl From<MapperTreeError> for TreeMapError {
    fn from(value: MapperTreeError) -> Self {
        Self::Tree(value)
    }
}

pub fn full_sis_tree_mapping_unavailable() -> Result<TreeCover, TreeMapError> {
    Err(TreeMapError::MissingSisPorts {
        operation: "treemap full SIS tree mapping",
    })
}

pub fn candidate_matches_at(
    tree: &MapperTree,
    library: &CandidateLibrary,
    root: MapperTreeNodeId,
    limits: TreeMapLimits,
) -> Result<Vec<CandidateMatch>, TreeMapError> {
    tree.validate()?;
    CandidateLibrary::with_limits(library.gates.clone(), limits)?;

    let mut mapper = CoverMapper::new(tree, library, limits);
    let matches = mapper.matches_at(root)?;
    Ok(matches
        .into_iter()
        .map(|item| CandidateMatch {
            gate: item.gate,
            root: item.root,
            frontier: item.frontier,
            local_cost: item.local_cost,
            total_cost: item.total_cost,
        })
        .collect())
}

pub fn select_lowest_cost_cover(
    tree: &MapperTree,
    library: &CandidateLibrary,
    limits: TreeMapLimits,
) -> Result<TreeCover, TreeMapError> {
    tree.validate()?;
    CandidateLibrary::with_limits(library.gates.clone(), limits)?;

    let mut mapper = CoverMapper::new(tree, library, limits);
    let root_choice = mapper.best_cover(tree.root())?.clone();
    let mut nodes = Vec::new();
    mapper.collect_cover(&root_choice, &mut nodes)?;

    Ok(TreeCover {
        root: tree.root(),
        total_cost: root_choice.total_cost,
        nodes,
    })
}

#[derive(Clone, Debug, PartialEq)]
struct CoverChoice {
    gate: String,
    root: MapperTreeNodeId,
    frontier: Vec<MapperTreeNodeId>,
    local_cost: f64,
    total_cost: f64,
}

struct CoverMapper<'a> {
    tree: &'a MapperTree,
    library: &'a CandidateLibrary,
    limits: TreeMapLimits,
    best: HashMap<MapperTreeNodeId, CoverChoice>,
}

impl<'a> CoverMapper<'a> {
    fn new(tree: &'a MapperTree, library: &'a CandidateLibrary, limits: TreeMapLimits) -> Self {
        Self {
            tree,
            library,
            limits,
            best: HashMap::new(),
        }
    }

    fn best_cover(&mut self, root: MapperTreeNodeId) -> Result<&CoverChoice, TreeMapError> {
        if !self.best.contains_key(&root) {
            let best = self
                .matches_at(root)?
                .into_iter()
                .min_by(compare_choices)
                .ok_or(TreeMapError::UncoverableNode { node: root })?;
            self.best.insert(root, best);
        }

        Ok(self
            .best
            .get(&root)
            .expect("best cover was inserted before lookup"))
    }

    fn matches_at(&mut self, root: MapperTreeNodeId) -> Result<Vec<CoverChoice>, TreeMapError> {
        let Some(node) = self.tree.node(root) else {
            return Err(MapperTreeError::MissingNode { node: root }.into());
        };
        if matches!(node, MapperTreeNode::Leaf { .. }) {
            return Ok(Vec::new());
        }

        let mut matches = Vec::new();
        for gate in self.library.gates() {
            let mut frontier = Vec::new();
            if !match_pattern(self.tree, root, &gate.pattern, &mut frontier)? {
                continue;
            }
            frontier.sort();
            frontier.dedup();

            let mut total_cost = gate.cost;
            for child in &frontier {
                if !is_boundary(self.tree, *child)? {
                    total_cost += self.best_cover(*child)?.total_cost;
                }
            }

            matches.push(CoverChoice {
                gate: gate.name.clone(),
                root,
                frontier,
                local_cost: gate.cost,
                total_cost,
            });
        }

        matches.sort_by(compare_choices);
        Ok(matches)
    }

    fn collect_cover(
        &mut self,
        choice: &CoverChoice,
        nodes: &mut Vec<CoverNode>,
    ) -> Result<(), TreeMapError> {
        if nodes.len() >= self.limits.max_cover_nodes {
            return Err(TreeMapError::TooManyCoverNodes {
                max: self.limits.max_cover_nodes,
            });
        }

        nodes.push(CoverNode {
            root: choice.root,
            gate: choice.gate.clone(),
            frontier: choice.frontier.clone(),
            local_cost: choice.local_cost,
            total_cost: choice.total_cost,
        });

        for child in &choice.frontier {
            if !is_boundary(self.tree, *child)? {
                let child_choice = self.best_cover(*child)?.clone();
                self.collect_cover(&child_choice, nodes)?;
            }
        }

        Ok(())
    }
}

fn match_pattern(
    tree: &MapperTree,
    root: MapperTreeNodeId,
    pattern: &TreePattern,
    frontier: &mut Vec<MapperTreeNodeId>,
) -> Result<bool, TreeMapError> {
    let node = tree
        .node(root)
        .ok_or(MapperTreeError::MissingNode { node: root })?;
    match pattern {
        TreePattern::Boundary(_) => {
            frontier.push(root);
            Ok(true)
        }
        TreePattern::Gate { kind, fanins } => {
            let MapperTreeNode::Gate {
                kind: node_kind,
                fanins: node_fanins,
            } = node
            else {
                return Ok(false);
            };
            if node_kind != kind || node_fanins.len() != fanins.len() {
                return Ok(false);
            }
            for (fanin, fanin_pattern) in node_fanins.iter().zip(fanins) {
                if fanin.inverted {
                    return Ok(false);
                }
                if !match_pattern(tree, fanin.node, fanin_pattern, frontier)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

fn is_boundary(tree: &MapperTree, node: MapperTreeNodeId) -> Result<bool, TreeMapError> {
    Ok(matches!(
        tree.node(node)
            .ok_or(MapperTreeError::MissingNode { node })?,
        MapperTreeNode::Leaf { .. }
            | MapperTreeNode::Gate {
                kind: PrimitiveGateKind::One | PrimitiveGateKind::Zero,
                ..
            }
    ))
}

fn compare_choices(left: &CoverChoice, right: &CoverChoice) -> std::cmp::Ordering {
    left.total_cost
        .total_cmp(&right.total_cost)
        .then_with(|| left.gate.cmp(&right.gate))
        .then_with(|| left.root.index().cmp(&right.root.index()))
        .then_with(|| {
            left.frontier
                .iter()
                .map(|node| node.index())
                .cmp(right.frontier.iter().map(|node| node.index()))
        })
}

fn validate_pattern(
    gate: &str,
    pattern: &TreePattern,
    limits: TreeMapLimits,
) -> Result<(), TreeMapError> {
    match pattern {
        TreePattern::Boundary(name) => {
            validate_name(name, "pattern boundary", limits.max_name_length)
        }
        TreePattern::Gate { kind, fanins } => {
            if kind.is_constant() {
                return Err(TreeMapError::InvalidPattern {
                    gate: gate.to_string(),
                    reason: "constant gates are treated as tree boundaries",
                });
            }
            if matches!(
                kind,
                PrimitiveGateKind::Buffer | PrimitiveGateKind::Inverter
            ) && fanins.len() != 1
            {
                return Err(TreeMapError::InvalidPattern {
                    gate: gate.to_string(),
                    reason: "unary gate pattern must have one fanin",
                });
            }
            if matches!(
                kind,
                PrimitiveGateKind::And
                    | PrimitiveGateKind::Nand
                    | PrimitiveGateKind::Or
                    | PrimitiveGateKind::Nor
                    | PrimitiveGateKind::Xor
                    | PrimitiveGateKind::Xnor
            ) && fanins.len() < 2
            {
                return Err(TreeMapError::InvalidPattern {
                    gate: gate.to_string(),
                    reason: "multi-input gate pattern must have at least two fanins",
                });
            }
            for fanin in fanins {
                validate_pattern(gate, fanin, limits)?;
            }
            Ok(())
        }
    }
}

fn validate_name(
    name: &str,
    kind: &'static str,
    max_name_length: usize,
) -> Result<(), TreeMapError> {
    if name.is_empty() {
        return Err(TreeMapError::EmptyName { kind });
    }
    if name.len() > max_name_length {
        return Err(TreeMapError::NameTooLong {
            kind,
            name: name.to_string(),
            max_name_length,
        });
    }
    Ok(())
}

fn classify_genlib_gate(gate: &GenlibGate) -> Option<Result<CandidateGate, TreeMapError>> {
    let expression = gate
        .output
        .expression
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    let leaf = || TreePattern::boundary("a");
    let binary = |kind| TreePattern::gate(kind, vec![leaf(), TreePattern::boundary("b")]);
    let pattern = match expression.as_str() {
        _ if gate.pins.len() == 1 && expression.contains('!') => {
            TreePattern::gate(PrimitiveGateKind::Inverter, vec![leaf()])
        }
        _ if gate.pins.len() == 1 => TreePattern::gate(PrimitiveGateKind::Buffer, vec![leaf()]),
        _ if gate.pins.len() == 2 && expression.contains('*') && !expression.contains('+') => {
            binary(PrimitiveGateKind::And)
        }
        _ if gate.pins.len() == 2 && expression.contains('+') && !expression.contains('*') => {
            binary(PrimitiveGateKind::Or)
        }
        _ => return None,
    };

    Some(CandidateGate::new(gate.name.clone(), pattern, gate.area))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::tree::{MapperTreeFanin, PrimitiveGateKind};

    fn leaf(name: &str) -> TreePattern {
        TreePattern::boundary(name)
    }

    fn pattern(kind: PrimitiveGateKind, fanins: Vec<TreePattern>) -> TreePattern {
        TreePattern::gate(kind, fanins)
    }

    fn sample_tree() -> MapperTree {
        let mut tree = MapperTree::empty();
        let a = tree.add_leaf("a");
        let b = tree.add_leaf("b");
        let c = tree.add_leaf("c");
        let and = tree.add_gate(
            PrimitiveGateKind::And,
            vec![MapperTreeFanin::new(a), MapperTreeFanin::new(b)],
        );
        let root = tree.add_gate(
            PrimitiveGateKind::Or,
            vec![MapperTreeFanin::new(and), MapperTreeFanin::new(c)],
        );
        tree.set_root(root);
        tree.validate().unwrap();
        tree
    }

    #[test]
    fn selects_lowest_cost_cover_deterministically() {
        let library = CandidateLibrary::new(vec![
            CandidateGate::new(
                "or2",
                pattern(PrimitiveGateKind::Or, vec![leaf("x"), leaf("y")]),
                2.0,
            )
            .unwrap(),
            CandidateGate::new(
                "and2_b",
                pattern(PrimitiveGateKind::And, vec![leaf("x"), leaf("y")]),
                1.0,
            )
            .unwrap(),
            CandidateGate::new(
                "and2_a",
                pattern(PrimitiveGateKind::And, vec![leaf("x"), leaf("y")]),
                1.0,
            )
            .unwrap(),
        ])
        .unwrap();

        let cover =
            select_lowest_cost_cover(&sample_tree(), &library, TreeMapLimits::default()).unwrap();

        assert_eq!(cover.total_cost, 3.0);
        assert_eq!(cover.nodes.len(), 2);
        assert_eq!(cover.nodes[0].gate, "or2");
        assert_eq!(cover.nodes[1].gate, "and2_a");
        assert_eq!(
            cover.nodes[0]
                .frontier
                .iter()
                .map(|node| node.index())
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn multi_level_candidate_covers_internal_cone_without_child_gate() {
        let library = CandidateLibrary::new(vec![
            CandidateGate::new(
                "or_and2",
                pattern(
                    PrimitiveGateKind::Or,
                    vec![
                        pattern(PrimitiveGateKind::And, vec![leaf("a"), leaf("b")]),
                        leaf("c"),
                    ],
                ),
                1.5,
            )
            .unwrap(),
            CandidateGate::new(
                "or2",
                pattern(PrimitiveGateKind::Or, vec![leaf("x"), leaf("y")]),
                2.0,
            )
            .unwrap(),
            CandidateGate::new(
                "and2",
                pattern(PrimitiveGateKind::And, vec![leaf("x"), leaf("y")]),
                1.0,
            )
            .unwrap(),
        ])
        .unwrap();

        let cover =
            select_lowest_cost_cover(&sample_tree(), &library, TreeMapLimits::default()).unwrap();

        assert_eq!(cover.total_cost, 1.5);
        assert_eq!(cover.nodes.len(), 1);
        assert_eq!(cover.nodes[0].gate, "or_and2");
        assert_eq!(
            cover.nodes[0]
                .frontier
                .iter()
                .map(|node| node.index())
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn creates_candidate_library_from_simple_genlib_gates() {
        let library = crate::ports::map::library::parse_genlib(concat!(
            "GATE and2 1 O=a*b;\n",
            "PIN a NONINV 1 999 1 .2 1 .2\n",
            "PIN b NONINV 1 999 1 .2 1 .2\n",
            "GATE complex 9 O=a*b+c;\n",
            "PIN a NONINV 1 999 1 .2 1 .2\n",
            "PIN b NONINV 1 999 1 .2 1 .2\n",
            "PIN c NONINV 1 999 1 .2 1 .2\n",
        ))
        .unwrap();

        let candidates = CandidateLibrary::from_genlib(&library).unwrap();

        assert_eq!(candidates.gates().len(), 1);
        assert_eq!(candidates.gates()[0].name, "and2");
        assert_eq!(candidates.gates()[0].cost, 1.0);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("treemap.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
