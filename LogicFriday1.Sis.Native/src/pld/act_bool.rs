//! Owned Rust Boolean matching for ACT-style PLD blocks.
//!
//! The original implementation works on SIS `network_t` and `node_t` objects.
//! This port keeps the matching behavior over an owned sum-of-products model.
//! Direct SIS graph integration is represented by generic runtime diagnostics
//! until the native node, network, cofactor, and simplification ports are
//! available.

use std::collections::BTreeSet;
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
pub enum LiteralPhase {
    Positive,
    Negative,
}

impl LiteralPhase {
    fn accepts(self, value: bool) -> bool {
        match self {
            Self::Positive => value,
            Self::Negative => !value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Literal {
    pub node: NodeId,
    pub phase: LiteralPhase,
}

impl Literal {
    pub const fn positive(node: NodeId) -> Self {
        Self {
            node,
            phase: LiteralPhase::Positive,
        }
    }

    pub const fn negative(node: NodeId) -> Self {
        Self {
            node,
            phase: LiteralPhase::Negative,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: impl IntoIterator<Item = Literal>) -> Self {
        let mut literals: Vec<_> = literals.into_iter().collect();
        literals.sort_unstable();
        literals.dedup();
        Self { literals }
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    fn cofactor(&self, literal: Literal) -> Option<Self> {
        let mut result = Vec::with_capacity(self.literals.len());
        for cube_literal in &self.literals {
            if cube_literal.node != literal.node {
                result.push(*cube_literal);
                continue;
            }

            if cube_literal.phase != literal.phase {
                return None;
            }
        }
        Some(Self::new(result))
    }

    fn is_subset_of(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .all(|literal| other.literals.contains(literal))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoolNode {
    pub name: String,
    pub kind: NodeKind,
    fanins: Vec<NodeId>,
    cubes: Vec<Cube>,
}

impl BoolNode {
    pub fn new(
        name: impl Into<String>,
        kind: NodeKind,
        fanins: Vec<NodeId>,
        cubes: impl IntoIterator<Item = Cube>,
    ) -> Self {
        let mut node = Self {
            name: name.into(),
            kind,
            fanins,
            cubes: cubes.into_iter().collect(),
        };
        node.simplify();
        node
    }

    pub fn primary_input(name: impl Into<String>) -> Self {
        Self::new(name, NodeKind::PrimaryInput, Vec::new(), Vec::new())
    }

    pub fn primary_output(
        name: impl Into<String>,
        fanins: Vec<NodeId>,
        cubes: impl IntoIterator<Item = Cube>,
    ) -> Self {
        Self::new(name, NodeKind::PrimaryOutput, fanins, cubes)
    }

    pub fn internal(
        name: impl Into<String>,
        fanins: Vec<NodeId>,
        cubes: impl IntoIterator<Item = Cube>,
    ) -> Self {
        Self::new(name, NodeKind::Internal, fanins, cubes)
    }

    pub fn constant(value: bool) -> Self {
        if value {
            Self::new("$one", NodeKind::Internal, Vec::new(), [Cube::new([])])
        } else {
            Self::new("$zero", NodeKind::Internal, Vec::new(), Vec::new())
        }
    }

    pub fn literal(node: NodeId, phase: LiteralPhase) -> Self {
        Self::new(
            format!("lit_{}", node.0),
            NodeKind::Internal,
            vec![node],
            [Cube::new([Literal { node, phase }])],
        )
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn fanin_count(&self) -> usize {
        self.fanins.len()
    }

    pub fn cube_count(&self) -> usize {
        self.cubes.len()
    }

    pub fn function_kind(&self) -> NodeFunction {
        if self.cubes.is_empty() {
            NodeFunction::Zero
        } else if self.fanins.is_empty()
            && self.cubes.len() == 1
            && self.cubes[0].literals.is_empty()
        {
            NodeFunction::One
        } else if self.is_single_positive_literal() {
            NodeFunction::Buffer
        } else if self.is_single_negative_literal() {
            NodeFunction::Inverter
        } else {
            NodeFunction::Other
        }
    }

    pub fn cofactor(&self, literal: Literal) -> Self {
        let mut fanins = self.fanins.clone();
        fanins.retain(|fanin| *fanin != literal.node);
        let cubes: Vec<_> = self
            .cubes
            .iter()
            .filter_map(|cube| cube.cofactor(literal))
            .collect();
        Self::new(
            format!("{}_cof", self.name),
            NodeKind::Internal,
            fanins,
            cubes,
        )
    }

    pub fn and(&self, other: &Self) -> Self {
        if self.is_constant(false) || other.is_constant(false) {
            return Self::constant(false);
        }
        if self.is_constant(true) {
            return other.clone();
        }
        if other.is_constant(true) {
            return self.clone();
        }

        let fanins = union_fanins(&self.fanins, &other.fanins);
        let mut cubes = Vec::new();
        for left in &self.cubes {
            for right in &other.cubes {
                if let Some(cube) = merge_cubes(left, right) {
                    cubes.push(cube);
                }
            }
        }
        Self::new("and", NodeKind::Internal, fanins, cubes)
    }

    pub fn or(&self, other: &Self) -> Self {
        if self.is_constant(true) || other.is_constant(true) {
            return Self::constant(true);
        }
        if self.is_constant(false) {
            return other.clone();
        }
        if other.is_constant(false) {
            return self.clone();
        }

        let fanins = union_fanins(&self.fanins, &other.fanins);
        let mut cubes = self.cubes.clone();
        cubes.extend(other.cubes.clone());
        Self::new("or", NodeKind::Internal, fanins, cubes)
    }

    pub fn eval(&self, assignment: &[(NodeId, bool)]) -> bool {
        self.cubes.iter().any(|cube| {
            cube.literals().iter().all(|literal| {
                assignment
                    .iter()
                    .find_map(|(node, value)| (*node == literal.node).then_some(*value))
                    .is_some_and(|value| literal.phase.accepts(value))
            })
        })
    }

    pub fn is_constant(&self, value: bool) -> bool {
        match (value, self.function_kind()) {
            (false, NodeFunction::Zero) | (true, NodeFunction::One) => true,
            _ => false,
        }
    }

    fn is_single_positive_literal(&self) -> bool {
        self.fanins.len() == 1
            && self.cubes.len() == 1
            && self.cubes[0].literals == vec![Literal::positive(self.fanins[0])]
    }

    fn is_single_negative_literal(&self) -> bool {
        self.fanins.len() == 1
            && self.cubes.len() == 1
            && self.cubes[0].literals == vec![Literal::negative(self.fanins[0])]
    }

    fn simplify(&mut self) {
        let mut normalized = Vec::new();
        for cube in &self.cubes {
            if cube.literals.iter().any(|literal| {
                cube.literals.contains(&Literal {
                    node: literal.node,
                    phase: opposite_phase(literal.phase),
                })
            }) {
                continue;
            }
            normalized.push(cube.clone());
        }

        normalized.sort_by(|left, right| left.literals.cmp(&right.literals));
        normalized.dedup();

        if normalized.iter().any(|cube| cube.literals.is_empty()) {
            self.name = "$one".to_string();
            self.fanins.clear();
            self.cubes = vec![Cube::new([])];
            return;
        }

        let mut irredundant = Vec::new();
        for (index, cube) in normalized.iter().enumerate() {
            let covered = normalized
                .iter()
                .enumerate()
                .any(|(other_index, other)| other_index != index && other.is_subset_of(cube));
            if !covered {
                irredundant.push(cube.clone());
            }
        }

        let used_fanins = irredundant
            .iter()
            .flat_map(|cube| cube.literals().iter().map(|literal| literal.node))
            .collect::<BTreeSet<_>>();
        self.fanins.retain(|fanin| used_fanins.contains(fanin));
        self.cubes = irredundant;
        if self.cubes.is_empty() {
            self.name = "$zero".to_string();
        } else if self.is_single_positive_literal() || self.is_single_negative_literal() {
            self.name = format!("lit_{}", self.fanins[0].0);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Buffer,
    Inverter,
    Other,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BoolNetwork {
    nodes: Vec<BoolNode>,
    dfs_order: Vec<NodeId>,
}

impl BoolNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: BoolNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        self.dfs_order.push(id);
        id
    }

    pub fn node(&self, id: NodeId) -> ActBoolResult<&BoolNode> {
        self.nodes.get(id.0).ok_or(ActBoolError::UnknownNode(id))
    }

    pub fn set_dfs_order(&mut self, order: Vec<NodeId>) {
        self.dfs_order = order;
    }

    pub fn dfs_order(&self) -> &[NodeId] {
        &self.dfs_order
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MuxMatch {
    pub zero: BoolNode,
    pub one: BoolNode,
    pub select: BoolNode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActMatch {
    pub a0: BoolNode,
    pub a1: BoolNode,
    pub sa: BoolNode,
    pub b0: BoolNode,
    pub b1: BoolNode,
    pub sb: BoolNode,
    pub s0: BoolNode,
    pub s1: BoolNode,
}

impl ActMatch {
    pub fn uses_or_gate(&self) -> bool {
        !self.s0.is_constant(false)
            && !self.s0.is_constant(true)
            && !self.s1.is_constant(false)
            && !self.s1.is_constant(true)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapReport {
    pub matches: usize,
    pub or_gate_matches: usize,
    pub entries: Vec<MapEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapEntry {
    pub node: NodeId,
    pub matched: bool,
    pub uses_or_gate: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActBoolError {
    UnknownNode(NodeId),
    TooManyFanins {
        node: String,
        fanins: usize,
        limit: usize,
    },
    MissingNativePorts {
        operation: &'static str,
    },
}

impl fmt::Display for ActBoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown ACT Boolean node {}", node.0),
            Self::TooManyFanins {
                node,
                fanins,
                limit,
            } => write!(
                f,
                "node {node} has {fanins} fanins; ACT Boolean matching supports at most {limit}"
            ),
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
        }
    }
}

impl Error for ActBoolError {}

pub type ActBoolResult<T> = Result<T, ActBoolError>;

pub fn sis_bound_operation_unavailable(operation: &'static str) -> ActBoolResult<()> {
    Err(ActBoolError::MissingNativePorts { operation })
}

pub fn act_bool_map_network(
    network: &BoolNetwork,
    map_alg: bool,
    act_is_or_used: bool,
) -> ActBoolResult<MapReport> {
    let mut report = MapReport {
        matches: 0,
        or_gate_matches: 0,
        entries: Vec::new(),
    };

    for node_id in network.dfs_order() {
        let node = network.node(*node_id)?;
        let match_info = act_is_act_function(node, map_alg, act_is_or_used)?;
        if let Some(match_info) = match_info {
            report.matches += 1;
            let uses_or_gate = match_info.uses_or_gate();
            report.or_gate_matches += usize::from(uses_or_gate);
            report.entries.push(MapEntry {
                node: *node_id,
                matched: true,
                uses_or_gate,
            });
        } else if node.kind != NodeKind::PrimaryInput && node.kind != NodeKind::PrimaryOutput {
            report.entries.push(MapEntry {
                node: *node_id,
                matched: false,
                uses_or_gate: false,
            });
        }
    }

    Ok(report)
}

pub fn act_is_act_function(
    function: &BoolNode,
    map_alg: bool,
    act_is_or_used: bool,
) -> ActBoolResult<Option<ActMatch>> {
    if function.kind == NodeKind::PrimaryInput || function.kind == NodeKind::PrimaryOutput {
        return Ok(None);
    }

    match function.function_kind() {
        NodeFunction::Zero => {
            return Ok(Some(constant_act_match(false)));
        }
        NodeFunction::One => {
            return Ok(Some(constant_act_match(true)));
        }
        _ => {}
    }

    let limit = if act_is_or_used { 8 } else { 7 };
    if function.fanin_count() > limit {
        return Ok(None);
    }

    for s0 in function.fanins().iter().copied() {
        if let Some(match_info) =
            match_for_selects(function, Literal::positive(s0), None, MatchMode::Exact)?
        {
            return Ok(Some(match_info));
        }
    }

    if !act_is_or_used {
        return Ok(None);
    }

    for (index, s0) in function.fanins().iter().copied().enumerate() {
        for s1 in function.fanins().iter().copied().skip(index + 1) {
            let mode = if map_alg {
                MatchMode::Exact
            } else {
                MatchMode::CareSet
            };
            if let Some(match_info) = match_for_selects(
                function,
                Literal::positive(s0),
                Some(Literal::positive(s1)),
                mode,
            )? {
                return Ok(Some(match_info));
            }
        }
    }

    Ok(None)
}

pub fn act_is_mux_function(function: &BoolNode) -> Option<MuxMatch> {
    if function.fanin_count() > 3 || function.cube_count() > 2 {
        return None;
    }

    match function.function_kind() {
        NodeFunction::Zero => {
            let zero = BoolNode::constant(false);
            return Some(MuxMatch {
                zero: zero.clone(),
                one: zero.clone(),
                select: zero,
            });
        }
        NodeFunction::One => {
            let one = BoolNode::constant(true);
            return Some(MuxMatch {
                zero: one.clone(),
                one: one.clone(),
                select: one,
            });
        }
        NodeFunction::Buffer => {
            let fanin = function.fanins()[0];
            return Some(MuxMatch {
                zero: BoolNode::constant(false),
                one: BoolNode::constant(true),
                select: BoolNode::literal(fanin, LiteralPhase::Positive),
            });
        }
        NodeFunction::Inverter => {
            let fanin = function.fanins()[0];
            return Some(MuxMatch {
                zero: BoolNode::constant(true),
                one: BoolNode::constant(false),
                select: BoolNode::literal(fanin, LiteralPhase::Positive),
            });
        }
        NodeFunction::Other => {}
    }

    for select in function.fanins().iter().copied() {
        let zero = function.cofactor(Literal::negative(select));
        let one = function.cofactor(Literal::positive(select));
        if is_mux_pin(&zero) && is_mux_pin(&one) {
            return Some(MuxMatch {
                zero,
                one,
                select: BoolNode::literal(select, LiteralPhase::Positive),
            });
        }
    }

    None
}

pub fn act_form_g(a: &BoolNode, ac_b: &BoolNode, a_pos: NodeId, b_pos: NodeId) -> BoolNode {
    let a_term = a.and(&BoolNode::literal(a_pos, LiteralPhase::Positive));
    let ac_b_select = BoolNode::literal(a_pos, LiteralPhase::Negative)
        .and(&BoolNode::literal(b_pos, LiteralPhase::Positive));
    a_term.or(&ac_b.and(&ac_b_select))
}

pub fn act_find_g_and_match(
    g: &BoolNode,
    c: &BoolNode,
    d: &BoolNode,
    ac_bc: &BoolNode,
) -> Option<MuxMatch> {
    let h = c.and(d);
    let candidate = g.or(&h.and(ac_bc));
    act_is_mux_function(&candidate)
}

pub fn act_is_two_input_h(node: &BoolNode) -> bool {
    if node.fanin_count() != 2 || node.cube_count() != 1 {
        return false;
    }
    let cube = &node.cubes()[0];
    let negative_literals = cube
        .literals()
        .iter()
        .filter(|literal| literal.phase == LiteralPhase::Negative)
        .count();
    negative_literals < 2
}

pub fn act_bool_map_network_from_sis<Network>(
    _network: &Network,
    _map_alg: bool,
    _act_is_or_used: bool,
) -> ActBoolResult<MapReport> {
    Err(ActBoolError::MissingNativePorts {
        operation: "act_bool_map_network SIS integration",
    })
}

pub fn act_is_act_function_from_sis<Node>(
    _node: &Node,
    _map_alg: bool,
    _act_is_or_used: bool,
) -> ActBoolResult<Option<ActMatch>> {
    Err(ActBoolError::MissingNativePorts {
        operation: "act_is_act_function SIS integration",
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MatchMode {
    Exact,
    CareSet,
}

fn match_for_selects(
    function: &BoolNode,
    s0: Literal,
    s1: Option<Literal>,
    mode: MatchMode,
) -> ActBoolResult<Option<ActMatch>> {
    let a_function = match s1 {
        Some(s1) => function
            .cofactor(opposite_literal(s0))
            .cofactor(opposite_literal(s1)),
        None => function.cofactor(opposite_literal(s0)),
    };
    let Some(a_mux) = act_is_mux_function(&a_function) else {
        return Ok(None);
    };

    let b_candidates = enumerate_mux_candidates(function.fanins());
    for b_mux in b_candidates {
        let match_info = ActMatch {
            a0: a_mux.zero.clone(),
            a1: a_mux.one.clone(),
            sa: a_mux.select.clone(),
            b0: b_mux.zero.clone(),
            b1: b_mux.one.clone(),
            sb: b_mux.select.clone(),
            s0: BoolNode::literal(s0.node, s0.phase),
            s1: s1
                .map(|literal| BoolNode::literal(literal.node, literal.phase))
                .unwrap_or_else(|| BoolNode::constant(false)),
        };
        if candidate_matches(function, &match_info, mode) {
            return Ok(Some(match_info));
        }
    }

    Ok(None)
}

fn candidate_matches(function: &BoolNode, match_info: &ActMatch, mode: MatchMode) -> bool {
    let fanins = function.fanins();
    if fanins.len() > usize::BITS as usize {
        return false;
    }

    for mask in 0usize..(1usize << fanins.len()) {
        let assignment = assignment_from_mask(fanins, mask);
        let s0 = match_info.s0.eval(&assignment);
        let s1 = match_info.s1.eval(&assignment);
        let care = mode == MatchMode::CareSet || !s0 || !s1 || function.eval(&assignment);
        if !care && s0 && s1 {
            continue;
        }

        let select = s0 || s1;
        let a_value = mux_value(&match_info.a0, &match_info.a1, &match_info.sa, &assignment);
        let b_value = mux_value(&match_info.b0, &match_info.b1, &match_info.sb, &assignment);
        let candidate = if select { b_value } else { a_value };
        if candidate != function.eval(&assignment) {
            return false;
        }
    }

    true
}

fn enumerate_mux_candidates(fanins: &[NodeId]) -> Vec<MuxMatch> {
    let mut pins = vec![BoolNode::constant(false), BoolNode::constant(true)];
    pins.extend(
        fanins
            .iter()
            .copied()
            .map(|fanin| BoolNode::literal(fanin, LiteralPhase::Positive)),
    );

    let mut candidates = Vec::new();
    for zero in &pins {
        for one in &pins {
            for select in &pins {
                if mux_support_count(zero, one, select) <= 3 {
                    candidates.push(MuxMatch {
                        zero: zero.clone(),
                        one: one.clone(),
                        select: select.clone(),
                    });
                }
            }
        }
    }
    candidates
}

fn is_mux_pin(node: &BoolNode) -> bool {
    matches!(
        node.function_kind(),
        NodeFunction::Zero | NodeFunction::One | NodeFunction::Buffer
    )
}

fn mux_support_count(zero: &BoolNode, one: &BoolNode, select: &BoolNode) -> usize {
    zero.fanins()
        .iter()
        .chain(one.fanins())
        .chain(select.fanins())
        .copied()
        .collect::<BTreeSet<_>>()
        .len()
}

fn mux_value(
    zero: &BoolNode,
    one: &BoolNode,
    select: &BoolNode,
    assignment: &[(NodeId, bool)],
) -> bool {
    if select.eval(assignment) {
        one.eval(assignment)
    } else {
        zero.eval(assignment)
    }
}

fn constant_act_match(value: bool) -> ActMatch {
    let constant = BoolNode::constant(value);
    ActMatch {
        a0: constant.clone(),
        a1: constant.clone(),
        sa: constant.clone(),
        b0: constant.clone(),
        b1: constant.clone(),
        sb: constant.clone(),
        s0: constant.clone(),
        s1: constant,
    }
}

fn merge_cubes(left: &Cube, right: &Cube) -> Option<Cube> {
    let mut literals = left.literals.clone();
    for literal in &right.literals {
        if literals.contains(&opposite_literal(*literal)) {
            return None;
        }
        literals.push(*literal);
    }
    Some(Cube::new(literals))
}

fn union_fanins(left: &[NodeId], right: &[NodeId]) -> Vec<NodeId> {
    left.iter()
        .chain(right)
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn assignment_from_mask(fanins: &[NodeId], mask: usize) -> Vec<(NodeId, bool)> {
    fanins
        .iter()
        .enumerate()
        .map(|(index, node)| (*node, (mask & (1usize << index)) != 0))
        .collect()
}

fn opposite_literal(literal: Literal) -> Literal {
    Literal {
        node: literal.node,
        phase: opposite_phase(literal.phase),
    }
}

fn opposite_phase(phase: LiteralPhase) -> LiteralPhase {
    match phase {
        LiteralPhase::Positive => LiteralPhase::Negative,
        LiteralPhase::Negative => LiteralPhase::Positive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn and_node(a: NodeId, b: NodeId) -> BoolNode {
        BoolNode::internal(
            "and",
            vec![a, b],
            [Cube::new([Literal::positive(a), Literal::positive(b)])],
        )
    }

    fn or_node(a: NodeId, b: NodeId) -> BoolNode {
        BoolNode::internal(
            "or",
            vec![a, b],
            [
                Cube::new([Literal::positive(a)]),
                Cube::new([Literal::positive(b)]),
            ],
        )
    }

    #[test]
    fn mux_detection_handles_constants_literals_and_two_input_gates() {
        let a = NodeId(0);
        let b = NodeId(1);

        let inv = BoolNode::literal(a, LiteralPhase::Negative);
        let inv_match = act_is_mux_function(&inv).unwrap();
        assert_eq!(inv_match.zero, BoolNode::constant(true));
        assert_eq!(inv_match.one, BoolNode::constant(false));
        assert_eq!(
            inv_match.select,
            BoolNode::literal(a, LiteralPhase::Positive)
        );

        let and_match = act_is_mux_function(&and_node(a, b)).unwrap();
        assert_eq!(and_match.zero, BoolNode::constant(false));
        assert_eq!(and_match.one, BoolNode::literal(b, LiteralPhase::Positive));

        let or_match = act_is_mux_function(&or_node(a, b)).unwrap();
        assert_eq!(or_match.zero, BoolNode::literal(b, LiteralPhase::Positive));
        assert_eq!(or_match.one, BoolNode::constant(true));
    }

    #[test]
    fn mux_detection_handles_three_fanin_ite_shape() {
        let a = NodeId(0);
        let b = NodeId(1);
        let s = NodeId(2);
        let node = BoolNode::internal(
            "mux",
            vec![a, b, s],
            [
                Cube::new([Literal::positive(a), Literal::negative(s)]),
                Cube::new([Literal::positive(b), Literal::positive(s)]),
            ],
        );

        let match_info = act_is_mux_function(&node).unwrap();

        assert_eq!(
            match_info.zero,
            BoolNode::literal(a, LiteralPhase::Positive)
        );
        assert_eq!(match_info.one, BoolNode::literal(b, LiteralPhase::Positive));
        assert_eq!(
            match_info.select,
            BoolNode::literal(s, LiteralPhase::Positive)
        );
    }

    #[test]
    fn act_match_without_or_gate_maps_single_select_block() {
        let a = NodeId(0);
        let b = NodeId(1);
        let s = NodeId(2);
        let function = BoolNode::internal(
            "f",
            vec![a, b, s],
            [
                Cube::new([Literal::positive(a), Literal::negative(s)]),
                Cube::new([Literal::positive(b), Literal::positive(s)]),
            ],
        );

        let match_info = act_is_act_function(&function, false, false)
            .unwrap()
            .unwrap();

        assert!(!match_info.uses_or_gate());
        assert!(candidate_matches(&function, &match_info, MatchMode::Exact));
    }

    #[test]
    fn act_match_with_or_gate_counts_two_selects() {
        let a = NodeId(0);
        let b = NodeId(1);
        let c = NodeId(2);
        let function = BoolNode::internal(
            "f",
            vec![a, b, c],
            [
                Cube::new([Literal::positive(a), Literal::positive(c)]),
                Cube::new([Literal::positive(b), Literal::positive(c)]),
            ],
        );

        let match_info = act_is_act_function(&function, false, true)
            .unwrap()
            .unwrap();

        assert!(candidate_matches(
            &function,
            &match_info,
            MatchMode::CareSet
        ));
    }

    #[test]
    fn map_network_skips_inputs_and_outputs_and_counts_matches() {
        let mut network = BoolNetwork::new();
        let a = network.add_node(BoolNode::primary_input("a"));
        let b = network.add_node(BoolNode::primary_input("b"));
        let f = network.add_node(or_node(a, b));
        let po = network.add_node(BoolNode::primary_output(
            "po",
            vec![f],
            [Cube::new([Literal::positive(f)])],
        ));
        network.set_dfs_order(vec![a, b, f, po]);

        let report = act_bool_map_network(&network, false, true).unwrap();

        assert_eq!(report.matches, 1);
        assert_eq!(report.or_gate_matches, 0);
        assert_eq!(
            report.entries,
            vec![MapEntry {
                node: f,
                matched: true,
                uses_or_gate: false,
            }]
        );
    }

    #[test]
    fn two_input_h_rejects_double_negative_product() {
        let a = NodeId(0);
        let b = NodeId(1);

        assert!(act_is_two_input_h(&and_node(a, b)));
        assert!(!act_is_two_input_h(&BoolNode::internal(
            "neg",
            vec![a, b],
            [Cube::new([Literal::negative(a), Literal::negative(b)])],
        )));
    }

    #[test]
    fn blocked_sis_entry_points_return_generic_diagnostics() {
        let err = act_bool_map_network_from_sis(&(), false, false).unwrap_err();

        assert_eq!(
            err,
            ActBoolError::MissingNativePorts {
                operation: "act_bool_map_network SIS integration",
            }
        );
        assert!(!err.to_string().contains(concat!("Logic", "Friday1", "-")));
    }

    #[test]
    fn no_legacy_c_abi_or_beads_metadata_tokens_are_present() {
        let source = include_str!("act_bool.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-")));
    }
}
