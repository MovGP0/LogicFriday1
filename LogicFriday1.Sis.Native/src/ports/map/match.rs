//! Bounded native Rust matcher for `sis/map/match.c`.
//!
//! The original SIS implementation recursively binds `node_t` objects to
//! primitive graph nodes and invokes a callback for every complete match. This
//! port keeps the owned-data part available to the mapper ports: validate
//! primitive match patterns, match them against `MapperTree` nodes with
//! polarity and arity constraints, derive simple candidates from genlib gates,
//! and return deterministic, cost-ordered results. Complete SIS `node_t` /
//! `prim_t` graph matching is reported as an explicit dependency until the
//! native network and primitive graph ports exist.

use std::error::Error;
use std::fmt;

use super::library::{GenlibGate, GenlibLibrary};
use super::tree::{
    MapperTree, MapperTreeError, MapperTreeNode, MapperTreeNodeId, PrimitiveGateKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MatchLimits {
    pub max_patterns: usize,
    pub max_pattern_nodes: usize,
    pub max_name_length: usize,
    pub max_matches: usize,
}

impl Default for MatchLimits {
    fn default() -> Self {
        Self {
            max_patterns: 16_384,
            max_pattern_nodes: 256,
            max_name_length: 256,
            max_matches: 65_536,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchPolarity {
    NonInverting,
    Inverting,
    Any,
}

impl MatchPolarity {
    pub fn accepts(self, inverted: bool) -> bool {
        match self {
            Self::NonInverting => !inverted,
            Self::Inverting => inverted,
            Self::Any => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchArity {
    Exact(usize),
    AtLeast(usize),
    Any,
}

impl MatchArity {
    pub fn accepts(self, value: usize) -> bool {
        match self {
            Self::Exact(expected) => value == expected,
            Self::AtLeast(expected) => value >= expected,
            Self::Any => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchEdge {
    pub polarity: MatchPolarity,
    pub pattern: Box<MatchPattern>,
}

impl MatchEdge {
    pub fn new(polarity: MatchPolarity, pattern: MatchPattern) -> Self {
        Self {
            polarity,
            pattern: Box::new(pattern),
        }
    }

    pub fn non_inverting(pattern: MatchPattern) -> Self {
        Self::new(MatchPolarity::NonInverting, pattern)
    }

    pub fn inverting(pattern: MatchPattern) -> Self {
        Self::new(MatchPolarity::Inverting, pattern)
    }

    pub fn any(pattern: MatchPattern) -> Self {
        Self::new(MatchPolarity::Any, pattern)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchPattern {
    Boundary {
        name: String,
    },
    Gate {
        kind: PrimitiveGateKind,
        arity: MatchArity,
        fanins: Vec<MatchEdge>,
    },
}

impl MatchPattern {
    pub fn boundary(name: impl Into<String>) -> Self {
        Self::Boundary { name: name.into() }
    }

    pub fn gate(kind: PrimitiveGateKind, fanins: Vec<MatchEdge>) -> Self {
        Self::Gate {
            kind,
            arity: MatchArity::Exact(fanins.len()),
            fanins,
        }
    }

    pub fn gate_with_arity(
        kind: PrimitiveGateKind,
        arity: MatchArity,
        fanins: Vec<MatchEdge>,
    ) -> Self {
        Self::Gate {
            kind,
            arity,
            fanins,
        }
    }

    fn node_count(&self) -> usize {
        match self {
            Self::Boundary { .. } => 1,
            Self::Gate { fanins, .. } => {
                1 + fanins
                    .iter()
                    .map(|edge| edge.pattern.node_count())
                    .sum::<usize>()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchGate {
    pub name: String,
    pub pattern: MatchPattern,
    pub cost: f64,
}

impl MatchGate {
    pub fn new(
        name: impl Into<String>,
        pattern: MatchPattern,
        cost: f64,
    ) -> Result<Self, MatchError> {
        let gate = Self {
            name: name.into(),
            pattern,
            cost,
        };
        gate.validate(MatchLimits::default())?;
        Ok(gate)
    }

    fn validate(&self, limits: MatchLimits) -> Result<(), MatchError> {
        validate_name(&self.name, "match gate", limits.max_name_length)?;
        if !self.cost.is_finite() || self.cost < 0.0 {
            return Err(MatchError::InvalidCost {
                gate: self.name.clone(),
                cost: self.cost,
            });
        }
        if self.pattern.node_count() > limits.max_pattern_nodes {
            return Err(MatchError::PatternTooLarge {
                gate: self.name.clone(),
                max: limits.max_pattern_nodes,
            });
        }
        validate_pattern(&self.name, &self.pattern, limits)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MatchLibrary {
    gates: Vec<MatchGate>,
}

impl MatchLibrary {
    pub fn new(gates: Vec<MatchGate>) -> Result<Self, MatchError> {
        Self::with_limits(gates, MatchLimits::default())
    }

    pub fn with_limits(gates: Vec<MatchGate>, limits: MatchLimits) -> Result<Self, MatchError> {
        if gates.len() > limits.max_patterns {
            return Err(MatchError::TooManyPatterns {
                max: limits.max_patterns,
            });
        }
        for gate in &gates {
            gate.validate(limits)?;
        }
        Ok(Self { gates })
    }

    pub fn from_genlib(library: &GenlibLibrary) -> Result<Self, MatchError> {
        let gates = library
            .gates
            .iter()
            .filter_map(match_gate_from_genlib)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(gates)
    }

    pub fn gates(&self) -> &[MatchGate] {
        &self.gates
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeMatch {
    pub gate: String,
    pub root: MapperTreeNodeId,
    pub frontier: Vec<MapperTreeNodeId>,
    pub cost: f64,
    pub pattern_nodes: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatchError {
    Tree(MapperTreeError),
    MissingSisPorts {
        operation: &'static str,
    },
    TooManyPatterns {
        max: usize,
    },
    TooManyMatches {
        max: usize,
    },
    PatternTooLarge {
        gate: String,
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
    InvalidArity {
        gate: String,
        reason: &'static str,
    },
    InvalidPattern {
        gate: String,
        reason: &'static str,
    },
    InvalidCost {
        gate: String,
        cost: f64,
    },
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tree(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
            Self::TooManyPatterns { max } => write!(f, "too many match patterns; max is {max}"),
            Self::TooManyMatches { max } => write!(f, "too many tree matches; max is {max}"),
            Self::PatternTooLarge { gate, max } => {
                write!(f, "match gate '{gate}' pattern exceeds {max} nodes")
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
            Self::InvalidArity { gate, reason } => {
                write!(f, "match gate '{gate}' has invalid arity: {reason}")
            }
            Self::InvalidPattern { gate, reason } => {
                write!(f, "match gate '{gate}' has invalid pattern: {reason}")
            }
            Self::InvalidCost { gate, cost } => {
                write!(f, "match gate '{gate}' has invalid cost {cost}")
            }
        }
    }
}

impl Error for MatchError {}

impl From<MapperTreeError> for MatchError {
    fn from(value: MapperTreeError) -> Self {
        Self::Tree(value)
    }
}

pub fn full_sis_graph_matching_unavailable() -> Result<Vec<TreeMatch>, MatchError> {
    Err(MatchError::MissingSisPorts {
        operation: "match full SIS graph matching",
    })
}

pub fn enumerate_tree_matches(
    tree: &MapperTree,
    library: &MatchLibrary,
    limits: MatchLimits,
) -> Result<Vec<TreeMatch>, MatchError> {
    tree.validate()?;
    MatchLibrary::with_limits(library.gates.clone(), limits)?;

    let mut matches = Vec::new();
    for root in tree.preorder()? {
        for gate in library.gates() {
            let mut frontier = Vec::new();
            if !match_at(tree, root, &gate.pattern, &mut frontier)? {
                continue;
            }
            if matches.len() >= limits.max_matches {
                return Err(MatchError::TooManyMatches {
                    max: limits.max_matches,
                });
            }
            frontier.sort();
            frontier.dedup();
            matches.push(TreeMatch {
                gate: gate.name.clone(),
                root,
                frontier,
                cost: gate.cost,
                pattern_nodes: gate.pattern.node_count(),
            });
        }
    }

    matches.sort_by(compare_tree_matches);
    Ok(matches)
}

pub fn matches_at_root(
    tree: &MapperTree,
    library: &MatchLibrary,
    root: MapperTreeNodeId,
    limits: MatchLimits,
) -> Result<Vec<TreeMatch>, MatchError> {
    tree.validate()?;
    MatchLibrary::with_limits(library.gates.clone(), limits)?;

    let mut matches = Vec::new();
    for gate in library.gates() {
        let mut frontier = Vec::new();
        if !match_at(tree, root, &gate.pattern, &mut frontier)? {
            continue;
        }
        if matches.len() >= limits.max_matches {
            return Err(MatchError::TooManyMatches {
                max: limits.max_matches,
            });
        }
        frontier.sort();
        frontier.dedup();
        matches.push(TreeMatch {
            gate: gate.name.clone(),
            root,
            frontier,
            cost: gate.cost,
            pattern_nodes: gate.pattern.node_count(),
        });
    }

    matches.sort_by(compare_tree_matches);
    Ok(matches)
}

fn match_at(
    tree: &MapperTree,
    root: MapperTreeNodeId,
    pattern: &MatchPattern,
    frontier: &mut Vec<MapperTreeNodeId>,
) -> Result<bool, MatchError> {
    let node = tree
        .node(root)
        .ok_or(MapperTreeError::MissingNode { node: root })?;
    match pattern {
        MatchPattern::Boundary { .. } => {
            frontier.push(root);
            Ok(true)
        }
        MatchPattern::Gate {
            kind,
            arity,
            fanins,
        } => {
            let MapperTreeNode::Gate {
                kind: node_kind,
                fanins: node_fanins,
            } = node
            else {
                return Ok(false);
            };
            if node_kind != kind || !arity.accepts(node_fanins.len()) {
                return Ok(false);
            }
            if fanins.is_empty() {
                frontier.extend(node_fanins.iter().map(|fanin| fanin.node));
                return Ok(true);
            }
            if fanins.len() != node_fanins.len() {
                return Ok(false);
            }
            for (node_fanin, pattern_fanin) in node_fanins.iter().zip(fanins) {
                if !pattern_fanin.polarity.accepts(node_fanin.inverted) {
                    return Ok(false);
                }
                if !match_at(tree, node_fanin.node, &pattern_fanin.pattern, frontier)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

fn compare_tree_matches(left: &TreeMatch, right: &TreeMatch) -> std::cmp::Ordering {
    left.cost
        .total_cmp(&right.cost)
        .then_with(|| right.pattern_nodes.cmp(&left.pattern_nodes))
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
    pattern: &MatchPattern,
    limits: MatchLimits,
) -> Result<(), MatchError> {
    match pattern {
        MatchPattern::Boundary { name } => {
            validate_name(name, "match boundary", limits.max_name_length)
        }
        MatchPattern::Gate {
            kind,
            arity,
            fanins,
        } => {
            validate_gate_arity(gate, *kind, *arity, fanins)?;
            for fanin in fanins {
                validate_pattern(gate, &fanin.pattern, limits)?;
            }
            Ok(())
        }
    }
}

fn validate_gate_arity(
    gate: &str,
    kind: PrimitiveGateKind,
    arity: MatchArity,
    fanins: &[MatchEdge],
) -> Result<(), MatchError> {
    if matches!(kind, PrimitiveGateKind::One | PrimitiveGateKind::Zero) && !fanins.is_empty() {
        return Err(MatchError::InvalidPattern {
            gate: gate.to_string(),
            reason: "constant match patterns cannot have fanins",
        });
    }
    match arity {
        MatchArity::Exact(value) if !primitive_accepts_arity(kind, value) => {
            Err(MatchError::InvalidArity {
                gate: gate.to_string(),
                reason: "exact arity is not valid for primitive kind",
            })
        }
        MatchArity::AtLeast(value)
            if matches!(
                kind,
                PrimitiveGateKind::Buffer
                    | PrimitiveGateKind::Inverter
                    | PrimitiveGateKind::One
                    | PrimitiveGateKind::Zero
            ) && value != primitive_min_arity(kind) =>
        {
            Err(MatchError::InvalidArity {
                gate: gate.to_string(),
                reason: "minimum arity is not valid for primitive kind",
            })
        }
        MatchArity::Any if !fanins.is_empty() => Err(MatchError::InvalidArity {
            gate: gate.to_string(),
            reason: "wildcard arity cannot also provide explicit fanin patterns",
        }),
        MatchArity::Exact(value) if !fanins.is_empty() && value != fanins.len() => {
            Err(MatchError::InvalidArity {
                gate: gate.to_string(),
                reason: "exact arity must match explicit fanin pattern count",
            })
        }
        MatchArity::AtLeast(value) if !fanins.is_empty() && fanins.len() < value => {
            Err(MatchError::InvalidArity {
                gate: gate.to_string(),
                reason: "explicit fanin pattern count is below minimum arity",
            })
        }
        _ => Ok(()),
    }
}

fn primitive_accepts_arity(kind: PrimitiveGateKind, value: usize) -> bool {
    match kind {
        PrimitiveGateKind::One | PrimitiveGateKind::Zero => value == 0,
        PrimitiveGateKind::Buffer | PrimitiveGateKind::Inverter => value == 1,
        PrimitiveGateKind::And
        | PrimitiveGateKind::Nand
        | PrimitiveGateKind::Or
        | PrimitiveGateKind::Nor
        | PrimitiveGateKind::Xor
        | PrimitiveGateKind::Xnor => value >= 2,
    }
}

fn primitive_min_arity(kind: PrimitiveGateKind) -> usize {
    match kind {
        PrimitiveGateKind::One | PrimitiveGateKind::Zero => 0,
        PrimitiveGateKind::Buffer | PrimitiveGateKind::Inverter => 1,
        PrimitiveGateKind::And
        | PrimitiveGateKind::Nand
        | PrimitiveGateKind::Or
        | PrimitiveGateKind::Nor
        | PrimitiveGateKind::Xor
        | PrimitiveGateKind::Xnor => 2,
    }
}

fn validate_name(name: &str, kind: &'static str, max_name_length: usize) -> Result<(), MatchError> {
    if name.is_empty() {
        return Err(MatchError::EmptyName { kind });
    }
    if name.len() > max_name_length {
        return Err(MatchError::NameTooLong {
            kind,
            name: name.to_string(),
            max_name_length,
        });
    }
    Ok(())
}

fn match_gate_from_genlib(gate: &GenlibGate) -> Option<Result<MatchGate, MatchError>> {
    let expression = gate
        .output
        .expression
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != ';')
        .collect::<String>();
    let pattern = parse_genlib_pattern(&expression, &gate.pins)?;
    Some(MatchGate::new(gate.name.clone(), pattern, gate.area))
}

fn parse_genlib_pattern(
    expression: &str,
    pins: &[super::library::GenlibPin],
) -> Option<MatchPattern> {
    let expression = strip_wrapping_parentheses(expression);
    if expression == "CONST0" || expression == "0" {
        return Some(MatchPattern::gate(PrimitiveGateKind::Zero, Vec::new()));
    }
    if expression == "CONST1" || expression == "1" {
        return Some(MatchPattern::gate(PrimitiveGateKind::One, Vec::new()));
    }

    if pins.len() == 1 {
        let pin = pins[0].declared_name.as_str();
        if expression == pin {
            return Some(MatchPattern::gate(
                PrimitiveGateKind::Buffer,
                vec![MatchEdge::non_inverting(MatchPattern::boundary(pin))],
            ));
        }
        if expression == format!("!{pin}") {
            return Some(MatchPattern::gate(
                PrimitiveGateKind::Inverter,
                vec![MatchEdge::non_inverting(MatchPattern::boundary(pin))],
            ));
        }
    }

    parse_binary_genlib_pattern(expression, pins, '*', PrimitiveGateKind::And)
        .or_else(|| parse_binary_genlib_pattern(expression, pins, '+', PrimitiveGateKind::Or))
}

fn parse_binary_genlib_pattern(
    expression: &str,
    pins: &[super::library::GenlibPin],
    operator: char,
    kind: PrimitiveGateKind,
) -> Option<MatchPattern> {
    if pins.len() != 2 {
        return None;
    }
    let (left, right) = expression.split_once(operator)?;
    if left.contains(other_binary_operator(operator))
        || right.contains(other_binary_operator(operator))
    {
        return None;
    }

    let left = parse_pin_term(left, pins)?;
    let right = parse_pin_term(right, pins)?;
    Some(MatchPattern::gate(
        kind,
        vec![
            MatchEdge::new(left.0, MatchPattern::boundary(left.1)),
            MatchEdge::new(right.0, MatchPattern::boundary(right.1)),
        ],
    ))
}

fn other_binary_operator(operator: char) -> char {
    if operator == '*' { '+' } else { '*' }
}

fn parse_pin_term<'a>(
    term: &'a str,
    pins: &'a [super::library::GenlibPin],
) -> Option<(MatchPolarity, String)> {
    let (polarity, name) = term
        .strip_prefix('!')
        .map(|name| (MatchPolarity::Inverting, name))
        .unwrap_or((MatchPolarity::NonInverting, term));
    if pins.iter().any(|pin| pin.declared_name == name) {
        Some((polarity, name.to_string()))
    } else {
        None
    }
}

fn strip_wrapping_parentheses(value: &str) -> &str {
    let mut current = value;
    while current.starts_with('(') && current.ends_with(')') && current.len() >= 2 {
        current = &current[1..current.len() - 1];
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::tree::{MapperTreeFanin, PrimitiveGateKind};

    fn leaf(name: &str) -> MatchPattern {
        MatchPattern::boundary(name)
    }

    fn sample_tree() -> MapperTree {
        let mut tree = MapperTree::empty();
        let a = tree.add_leaf("a");
        let b = tree.add_leaf("b");
        let c = tree.add_leaf("c");
        let and = tree.add_gate(
            PrimitiveGateKind::And,
            vec![MapperTreeFanin::new(a), MapperTreeFanin::inverted(b)],
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
    fn matches_polarity_and_arity_against_mapper_tree() {
        let library = MatchLibrary::new(vec![
            MatchGate::new(
                "or2",
                MatchPattern::gate(
                    PrimitiveGateKind::Or,
                    vec![
                        MatchEdge::non_inverting(MatchPattern::gate(
                            PrimitiveGateKind::And,
                            vec![
                                MatchEdge::non_inverting(leaf("a")),
                                MatchEdge::inverting(leaf("b")),
                            ],
                        )),
                        MatchEdge::non_inverting(leaf("c")),
                    ],
                ),
                2.0,
            )
            .unwrap(),
            MatchGate::new(
                "wrong_polarity",
                MatchPattern::gate(
                    PrimitiveGateKind::And,
                    vec![
                        MatchEdge::non_inverting(leaf("a")),
                        MatchEdge::non_inverting(leaf("b")),
                    ],
                ),
                1.0,
            )
            .unwrap(),
        ])
        .unwrap();

        let tree = sample_tree();
        let matches =
            matches_at_root(&tree, &library, tree.root(), MatchLimits::default()).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].gate, "or2");
        assert_eq!(
            matches[0]
                .frontier
                .iter()
                .map(|node| node.index())
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn orders_matches_by_cost_specificity_name_and_root() {
        let library = MatchLibrary::new(vec![
            MatchGate::new(
                "z_and",
                MatchPattern::gate(
                    PrimitiveGateKind::And,
                    vec![MatchEdge::any(leaf("a")), MatchEdge::any(leaf("b"))],
                ),
                1.0,
            )
            .unwrap(),
            MatchGate::new(
                "a_any_and",
                MatchPattern::gate_with_arity(
                    PrimitiveGateKind::And,
                    MatchArity::Exact(2),
                    Vec::new(),
                ),
                1.0,
            )
            .unwrap(),
            MatchGate::new(
                "cheap_or",
                MatchPattern::gate_with_arity(
                    PrimitiveGateKind::Or,
                    MatchArity::AtLeast(2),
                    Vec::new(),
                ),
                0.5,
            )
            .unwrap(),
        ])
        .unwrap();

        let matches = enumerate_tree_matches(&sample_tree(), &library, MatchLimits::default())
            .unwrap()
            .into_iter()
            .map(|item| item.gate)
            .collect::<Vec<_>>();

        assert_eq!(matches, vec!["cheap_or", "z_and", "a_any_and"]);
    }

    #[test]
    fn derives_simple_match_patterns_from_genlib() {
        let genlib = crate::ports::map::library::parse_genlib(concat!(
            "GATE and_inv 3 O=a*!b;\n",
            "PIN a NONINV 1 999 1 .2 1 .2\n",
            "PIN b INV 1 999 1 .2 1 .2\n",
            "GATE complex 9 O=a*b+c;\n",
            "PIN a NONINV 1 999 1 .2 1 .2\n",
            "PIN b NONINV 1 999 1 .2 1 .2\n",
            "PIN c NONINV 1 999 1 .2 1 .2\n",
        ))
        .unwrap();

        let library = MatchLibrary::from_genlib(&genlib).unwrap();

        assert_eq!(library.gates().len(), 1);
        assert_eq!(library.gates()[0].name, "and_inv");
        let matches =
            enumerate_tree_matches(&sample_tree(), &library, MatchLimits::default()).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].gate, "and_inv");
        assert_eq!(matches[0].root.index(), 3);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("match.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
