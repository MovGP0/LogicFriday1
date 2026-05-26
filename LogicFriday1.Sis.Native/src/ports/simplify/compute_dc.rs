//! Native Rust model for `LogicSynthesis/sis/simplify/compute_dc.c`.
//!
//! The original SIS module computes CSPF/ODC metadata through `network_t`,
//! `node_t`, `array_t`, `st_table`, and BDD APIs. This file ports the
//! deterministic Boolean and ordering behavior onto an owned Rust graph. Direct
//! SIS-bound entry points report generic missing-port diagnostics until the
//! native node, network, BDD, and simplify integration ports are available.

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const INFINITY_VALUE: usize = usize::MAX;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComputeNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputeNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CubeLiteral {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverCube {
    pub inputs: Vec<CubeLiteral>,
}

impl CoverCube {
    pub fn new(inputs: impl Into<Vec<CubeLiteral>>) -> Self {
        Self {
            inputs: inputs.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoolExpr {
    Const(bool),
    Literal { node: ComputeNodeId, positive: bool },
    Not(Box<BoolExpr>),
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
    Xnor(Box<BoolExpr>, Box<BoolExpr>),
}

impl BoolExpr {
    pub const fn zero() -> Self {
        Self::Const(false)
    }

    pub const fn one() -> Self {
        Self::Const(true)
    }

    pub const fn literal(node: ComputeNodeId, positive: bool) -> Self {
        Self::Literal { node, positive }
    }

    pub fn not(expr: Self) -> Self {
        match expr {
            Self::Const(value) => Self::Const(!value),
            Self::Not(inner) => *inner,
            other => Self::Not(Box::new(other)),
        }
    }

    pub fn and(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::Const(false), _) | (_, Self::Const(false)) => Self::Const(false),
            (Self::Const(true), expr) | (expr, Self::Const(true)) => expr,
            (Self::And(mut left), Self::And(right)) => {
                left.extend(right);
                Self::And(left)
            }
            (Self::And(mut terms), expr) | (expr, Self::And(mut terms)) => {
                terms.push(expr);
                Self::And(terms)
            }
            (left, right) => Self::And(vec![left, right]),
        }
    }

    pub fn or(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::Const(true), _) | (_, Self::Const(true)) => Self::Const(true),
            (Self::Const(false), expr) | (expr, Self::Const(false)) => expr,
            (Self::Or(mut left), Self::Or(right)) => {
                left.extend(right);
                Self::Or(left)
            }
            (Self::Or(mut terms), expr) | (expr, Self::Or(mut terms)) => {
                terms.push(expr);
                Self::Or(terms)
            }
            (left, right) => Self::Or(vec![left, right]),
        }
    }

    pub fn xnor(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::Const(left), Self::Const(right)) => Self::Const(left == right),
            (left, right) if left == right => Self::Const(true),
            (left, right) => Self::Xnor(Box::new(left), Box::new(right)),
        }
    }

    pub fn cofactor(&self, variable: ComputeNodeId, value: bool) -> Self {
        match self {
            Self::Const(value) => Self::Const(*value),
            Self::Literal { node, positive } if *node == variable => {
                Self::Const(if *positive { value } else { !value })
            }
            Self::Literal { node, positive } => Self::Literal {
                node: *node,
                positive: *positive,
            },
            Self::Not(expr) => Self::not(expr.cofactor(variable, value)),
            Self::And(terms) => terms
                .iter()
                .map(|term| term.cofactor(variable, value))
                .fold(Self::one(), Self::and),
            Self::Or(terms) => terms
                .iter()
                .map(|term| term.cofactor(variable, value))
                .fold(Self::zero(), Self::or),
            Self::Xnor(left, right) => Self::xnor(
                left.cofactor(variable, value),
                right.cofactor(variable, value),
            ),
        }
    }

    pub fn is_zero(&self) -> bool {
        matches!(self, Self::Const(false))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoubleNode {
    pub pos: BoolExpr,
    pub neg: BoolExpr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CspfSlot {
    pub level: i32,
    pub node: Option<ComputeNodeId>,
    pub list: Vec<DoubleNode>,
    pub bdd: Option<BoolExpr>,
    pub set: BTreeSet<ComputeNodeId>,
}

impl CspfSlot {
    pub fn allocated() -> Self {
        Self {
            level: 0,
            node: None,
            list: Vec::new(),
            bdd: None,
            set: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OdcSlot {
    pub order: i32,
    pub level: i32,
    pub value: usize,
    pub f: Option<BoolExpr>,
    pub var: Option<BoolExpr>,
    pub vodc: Vec<BoolExpr>,
}

impl OdcSlot {
    pub fn allocated() -> Self {
        Self {
            order: 0,
            level: 0,
            value: 0,
            f: None,
            var: None,
            vodc: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeNode {
    pub name: String,
    pub kind: ComputeNodeKind,
    pub fanins: Vec<ComputeNodeId>,
    pub fanouts: Vec<ComputeNodeId>,
    pub cover: Vec<CoverCube>,
    pub cspf: Option<CspfSlot>,
    pub odc: Option<OdcSlot>,
}

impl ComputeNode {
    pub fn new(name: impl Into<String>, kind: ComputeNodeKind, cover: Vec<CoverCube>) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            cover,
            cspf: None,
            odc: None,
        }
    }

    pub fn primary_input(name: impl Into<String>) -> Self {
        Self::new(name, ComputeNodeKind::PrimaryInput, Vec::new())
    }

    pub fn primary_output(name: impl Into<String>) -> Self {
        Self::new(name, ComputeNodeKind::PrimaryOutput, Vec::new())
    }

    pub fn internal(name: impl Into<String>, cover: Vec<CoverCube>) -> Self {
        Self::new(name, ComputeNodeKind::Internal, cover)
    }

    pub fn cube_count(&self) -> usize {
        self.cover.len()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComputeNetwork {
    nodes: Vec<ComputeNode>,
}

impl ComputeNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: ComputeNode) -> ComputeNodeId {
        let id = ComputeNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn connect(
        &mut self,
        fanin: ComputeNodeId,
        fanout: ComputeNodeId,
    ) -> Result<(), ComputeDcError> {
        self.node(fanin)?;
        self.node(fanout)?;
        if !self.nodes[fanout.0].fanins.contains(&fanin) {
            self.nodes[fanout.0].fanins.push(fanin);
        }
        if !self.nodes[fanin.0].fanouts.contains(&fanout) {
            self.nodes[fanin.0].fanouts.push(fanout);
        }
        Ok(())
    }

    pub fn node(&self, id: ComputeNodeId) -> Result<&ComputeNode, ComputeDcError> {
        self.nodes.get(id.0).ok_or(ComputeDcError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: ComputeNodeId) -> Result<&mut ComputeNode, ComputeDcError> {
        self.nodes
            .get_mut(id.0)
            .ok_or(ComputeDcError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[ComputeNode] {
        &self.nodes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComputeDcError {
    UnknownNode(ComputeNodeId),
    MissingFanin {
        node: ComputeNodeId,
        fanin: ComputeNodeId,
    },
    CubeArityMismatch {
        node: ComputeNodeId,
        cube_index: usize,
        fanins: usize,
        literals: usize,
    },
    MissingCspf(ComputeNodeId),
    MissingOdc(ComputeNodeId),
}

impl fmt::Display for ComputeDcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown compute-dc node {:?}", node),
            Self::MissingFanin { node, fanin } => {
                write!(f, "{fanin:?} is not a fanin of {node:?}")
            }
            Self::CubeArityMismatch {
                node,
                cube_index,
                fanins,
                literals,
            } => write!(
                f,
                "cube {cube_index} of {node:?} has {literals} literals for {fanins} fanins",
            ),
            Self::MissingCspf(node) => write!(f, "CSPF slot does not exist for {node:?}"),
            Self::MissingOdc(node) => write!(f, "ODC slot does not exist for {node:?}"),
        }
    }
}

impl Error for ComputeDcError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputeDcMode {
    WithoutOdc,
    WithOdc,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComputeDcReport {
    pub visited_nodes: Vec<ComputeNodeId>,
    pub cspf_nodes: Vec<ComputeNodeId>,
    pub simplified_nodes: Vec<ComputeNodeId>,
    pub mspf_edges: Vec<(ComputeNodeId, ComputeNodeId)>,
    pub updated_fanins: Vec<ComputeNodeId>,
}

impl ComputeDcReport {
    fn visit(&mut self, node: ComputeNodeId) {
        self.visited_nodes.push(node);
    }

    fn cspf(&mut self, node: ComputeNodeId) {
        self.cspf_nodes.push(node);
    }

    fn simplified(&mut self, node: ComputeNodeId) {
        self.simplified_nodes.push(node);
    }

    fn mspf(&mut self, node: ComputeNodeId, fanin: ComputeNodeId) {
        self.mspf_edges.push((node, fanin));
    }

    fn updated_fanin(&mut self, fanin: ComputeNodeId) {
        if !self.updated_fanins.contains(&fanin) {
            self.updated_fanins.push(fanin);
        }
    }
}

pub fn cspf_alloc(network: &mut ComputeNetwork, node: ComputeNodeId) -> Result<(), ComputeDcError> {
    network.node_mut(node)?.cspf = Some(CspfSlot::allocated());
    Ok(())
}

pub fn cspf_free(network: &mut ComputeNetwork, node: ComputeNodeId) -> Result<(), ComputeDcError> {
    network.node_mut(node)?.cspf = None;
    Ok(())
}

pub fn odc_alloc(network: &mut ComputeNetwork, node: ComputeNodeId) -> Result<(), ComputeDcError> {
    network.node_mut(node)?.odc = Some(OdcSlot::allocated());
    Ok(())
}

pub fn odc_free(network: &mut ComputeNetwork, node: ComputeNodeId) -> Result<(), ComputeDcError> {
    network.node_mut(node)?.odc = None;
    Ok(())
}

pub fn cspf_bdd_dc(
    network: &ComputeNetwork,
    node: ComputeNodeId,
) -> Result<BoolExpr, ComputeDcError> {
    let Some(cspf) = &network.node(node)?.cspf else {
        return Err(ComputeDcError::MissingCspf(node));
    };
    Ok(cspf.bdd.clone().unwrap_or_else(BoolExpr::zero))
}

pub fn compute_mspf(
    network: &ComputeNetwork,
    node: ComputeNodeId,
    fanin: ComputeNodeId,
) -> Result<DoubleNode, ComputeDcError> {
    let node_data = network.node(node)?;
    let fanin_data = network.node(fanin)?;
    if node_data.kind == ComputeNodeKind::PrimaryOutput
        || fanin_data.kind == ComputeNodeKind::PrimaryInput
    {
        return Ok(DoubleNode {
            pos: BoolExpr::zero(),
            neg: BoolExpr::one(),
        });
    }

    let index = node_data
        .fanins
        .iter()
        .position(|candidate| *candidate == fanin)
        .ok_or(ComputeDcError::MissingFanin { node, fanin })?;

    let mut f_x = BoolExpr::zero();
    let mut f_xbar = BoolExpr::zero();
    let mut h = BoolExpr::zero();
    for (cube_index, cube) in node_data.cover.iter().enumerate() {
        if cube.inputs.len() != node_data.fanins.len() {
            return Err(ComputeDcError::CubeArityMismatch {
                node,
                cube_index,
                fanins: node_data.fanins.len(),
                literals: cube.inputs.len(),
            });
        }

        let cube_expr = cube_to_expr(network, node, cube, Some(index))?;
        match cube.inputs[index] {
            CubeLiteral::DontCare => h = BoolExpr::or(h, cube_expr),
            CubeLiteral::One => f_x = BoolExpr::or(f_x, cube_expr),
            CubeLiteral::Zero => f_xbar = BoolExpr::or(f_xbar, cube_expr),
        }
    }

    let p = BoolExpr::xnor(f_x, f_xbar);
    let pnot = BoolExpr::not(p.clone());
    let pos = BoolExpr::or(p, h.clone());
    let neg = BoolExpr::and(pnot, BoolExpr::not(h));
    Ok(DoubleNode { pos, neg })
}

pub fn cube_to_expr(
    network: &ComputeNetwork,
    node: ComputeNodeId,
    cube: &CoverCube,
    smooth_index: Option<usize>,
) -> Result<BoolExpr, ComputeDcError> {
    let node_data = network.node(node)?;
    if cube.inputs.len() != node_data.fanins.len() {
        return Err(ComputeDcError::CubeArityMismatch {
            node,
            cube_index: 0,
            fanins: node_data.fanins.len(),
            literals: cube.inputs.len(),
        });
    }

    let mut expr = BoolExpr::one();
    for (index, literal) in cube.inputs.iter().copied().enumerate() {
        if smooth_index == Some(index) || literal == CubeLiteral::DontCare {
            continue;
        }
        let fanin = node_data.fanins[index];
        expr = BoolExpr::and(expr, BoolExpr::literal(fanin, literal == CubeLiteral::One));
    }
    Ok(expr)
}

pub fn compute_cspf_compatible(
    dc: BoolExpr,
    pfanin: ComputeNodeId,
    bool_diff: BoolExpr,
    previous_cspf: Option<&BoolExpr>,
) -> BoolExpr {
    let Some(previous_cspf) = previous_cspf else {
        return dc;
    };
    if previous_cspf.is_zero() {
        return dc;
    }

    let both_cofactors = BoolExpr::and(dc.cofactor(pfanin, false), dc.cofactor(pfanin, true));
    let compatible_previous_edge = BoolExpr::and(dc, bool_diff);
    BoolExpr::or(both_cofactors, compatible_previous_edge)
}

pub fn cspf_inout_table(
    network: &ComputeNetwork,
    node: ComputeNodeId,
    input_level: usize,
    output_level: usize,
) -> Result<BTreeSet<ComputeNodeId>, ComputeDcError> {
    let mut table = BTreeSet::new();
    for fanin in transitive_fanin(network, node, input_level)? {
        table.insert(fanin);
        for fanout in transitive_fanout(network, fanin, output_level)? {
            table.insert(fanout);
        }
    }
    Ok(table)
}

pub fn find_odc_level(network: &mut ComputeNetwork) -> Result<(), ComputeDcError> {
    let order = dfs_from_inputs(network)?;
    for node in order {
        let level = if network.node(node)?.kind == ComputeNodeKind::PrimaryInput {
            0
        } else {
            network
                .node(node)?
                .fanins
                .iter()
                .map(|fanin| {
                    network
                        .node(*fanin)?
                        .odc
                        .as_ref()
                        .map(|odc| odc.level)
                        .ok_or(ComputeDcError::MissingOdc(*fanin))
                })
                .try_fold(0, |max_level, level| Ok(max_level.max(level?)))?
                + 1
        };
        let Some(odc) = &mut network.node_mut(node)?.odc else {
            return Err(ComputeDcError::MissingOdc(node));
        };
        odc.level = level;
    }
    Ok(())
}

pub fn level_node_cmp1(
    network: &ComputeNetwork,
    left: ComputeNodeId,
    right: ComputeNodeId,
) -> Result<Ordering, ComputeDcError> {
    let left_odc = network
        .node(left)?
        .odc
        .as_ref()
        .ok_or(ComputeDcError::MissingOdc(left))?;
    let right_odc = network
        .node(right)?
        .odc
        .as_ref()
        .ok_or(ComputeDcError::MissingOdc(right))?;
    Ok(right_odc
        .level
        .cmp(&left_odc.level)
        .then_with(|| right_odc.value.cmp(&left_odc.value))
        .then_with(|| right_odc.order.cmp(&left_odc.order)))
}

pub fn level_node_cmp2(
    network: &ComputeNetwork,
    left: ComputeNodeId,
    right: ComputeNodeId,
) -> Result<Ordering, ComputeDcError> {
    let left_odc = network
        .node(left)?
        .odc
        .as_ref()
        .ok_or(ComputeDcError::MissingOdc(left))?;
    let right_odc = network
        .node(right)?
        .odc
        .as_ref()
        .ok_or(ComputeDcError::MissingOdc(right))?;
    Ok(right_odc
        .level
        .cmp(&left_odc.level)
        .then_with(|| right_odc.order.cmp(&left_odc.order)))
}

pub fn level_node_cmp3(
    network: &ComputeNetwork,
    left: ComputeNodeId,
    right: ComputeNodeId,
) -> Result<Ordering, ComputeDcError> {
    let left_odc = network
        .node(left)?
        .odc
        .as_ref()
        .ok_or(ComputeDcError::MissingOdc(left))?;
    let right_odc = network
        .node(right)?
        .odc
        .as_ref()
        .ok_or(ComputeDcError::MissingOdc(right))?;
    Ok(left_odc.level.cmp(&right_odc.level).then_with(|| {
        network
            .node(left)
            .map(ComputeNode::cube_count)
            .unwrap_or(usize::MAX)
            .cmp(
                &network
                    .node(right)
                    .map(ComputeNode::cube_count)
                    .unwrap_or(usize::MAX),
            )
    }))
}

pub fn odc_value(network: &ComputeNetwork, node: ComputeNodeId) -> Result<usize, ComputeDcError> {
    let node_data = network.node(node)?;
    if node_data.kind == ComputeNodeKind::PrimaryOutput {
        return Ok(INFINITY_VALUE);
    }
    if node_data.fanouts.iter().all(|fanout| {
        network
            .node(*fanout)
            .is_ok_and(|np| np.kind == ComputeNodeKind::PrimaryOutput)
    }) {
        return Ok(INFINITY_VALUE);
    }

    let mut unique_fanouts = BTreeSet::new();
    for fanout in &node_data.fanouts {
        unique_fanouts.insert(*fanout);
    }

    let mut value = 0usize;
    for fanout in unique_fanouts {
        let fanout_node = network.node(fanout)?;
        if fanout_node.kind != ComputeNodeKind::PrimaryOutput {
            value += fanout_node
                .fanins
                .iter()
                .filter(|fanin| **fanin == node)
                .count();
        }
    }
    Ok(value)
}

pub fn transitive_fanin(
    network: &ComputeNetwork,
    node: ComputeNodeId,
    limit: usize,
) -> Result<Vec<ComputeNodeId>, ComputeDcError> {
    walk_cone(network, node, limit, ConeDirection::Fanin)
}

pub fn transitive_fanout(
    network: &ComputeNetwork,
    node: ComputeNodeId,
    limit: usize,
) -> Result<Vec<ComputeNodeId>, ComputeDcError> {
    walk_cone(network, node, limit, ConeDirection::Fanout)
}

pub fn simplify_without_odc_native(
    network: &mut ComputeNetwork,
) -> Result<ComputeDcReport, ComputeDcError> {
    ensure_cspf_slots(network)?;

    let mut report = ComputeDcReport::default();
    for node in dfs_from_inputs(network)? {
        report.visit(node);
        let node_data = network.node(node)?;
        if matches!(
            node_data.kind,
            ComputeNodeKind::PrimaryInput | ComputeNodeKind::PrimaryOutput
        ) {
            continue;
        }

        let cspf_bdd = if node_data.fanins.is_empty() {
            BoolExpr::zero()
        } else {
            let mut bdd = BoolExpr::one();
            for fanout in transitive_fanout(network, node, 1)? {
                bdd = BoolExpr::and(bdd, cspf_bdd_dc(network, fanout)?);
            }
            bdd
        };

        network
            .node_mut(node)?
            .cspf
            .as_mut()
            .ok_or(ComputeDcError::MissingCspf(node))?
            .bdd = Some(cspf_bdd);
        report.cspf(node);

        if network.node(node)?.kind == ComputeNodeKind::Internal {
            report.simplified(node);
        }
    }

    Ok(report)
}

pub fn simplify_with_odc_native(
    network: &mut ComputeNetwork,
) -> Result<ComputeDcReport, ComputeDcError> {
    ensure_cspf_slots(network)?;

    let mut report = ComputeDcReport::default();
    for node in dfs_from_inputs(network)? {
        report.visit(node);
        let node_data = network.node(node)?;
        if matches!(
            node_data.kind,
            ComputeNodeKind::PrimaryInput | ComputeNodeKind::PrimaryOutput
        ) {
            continue;
        }

        if node_data.fanins.is_empty() {
            network
                .node_mut(node)?
                .cspf
                .as_mut()
                .ok_or(ComputeDcError::MissingCspf(node))?
                .bdd = Some(BoolExpr::zero());
            report.cspf(node);
            continue;
        }

        let fanouts = transitive_fanout(network, node, 1)?;
        let mut fdc = None::<BoolExpr>;
        for fanout in fanouts {
            let fanout_data = network.node(fanout)?;
            let mut edge_dc = if fanout_data.kind == ComputeNodeKind::PrimaryOutput {
                BoolExpr::zero()
            } else {
                let index = fanout_data
                    .fanins
                    .iter()
                    .position(|fanin| *fanin == node)
                    .ok_or(ComputeDcError::MissingFanin {
                        node: fanout,
                        fanin: node,
                    })?;
                fanout_data
                    .cspf
                    .as_ref()
                    .and_then(|cspf| cspf.list.get(index))
                    .map(|mspf| mspf.pos.clone())
                    .unwrap_or_else(BoolExpr::zero)
            };

            if fanout_data.kind != ComputeNodeKind::PrimaryOutput || !fanout_data.fanouts.is_empty()
            {
                let fanins = fanout_data.fanins.clone();
                for pfanin in fanins {
                    if network.node(pfanin)?.kind == ComputeNodeKind::PrimaryInput {
                        continue;
                    }
                    let Some(index) = network
                        .node(fanout)?
                        .fanins
                        .iter()
                        .position(|candidate| *candidate == pfanin)
                    else {
                        continue;
                    };
                    let Some(bool_diff) = network
                        .node(fanout)?
                        .cspf
                        .as_ref()
                        .and_then(|cspf| cspf.list.get(index))
                        .map(|mspf| mspf.neg.clone())
                    else {
                        continue;
                    };
                    let previous = network
                        .node(pfanin)?
                        .cspf
                        .as_ref()
                        .and_then(|cspf| cspf.bdd.as_ref());
                    edge_dc = compute_cspf_compatible(edge_dc, pfanin, bool_diff, previous);
                }
            }

            let fanout_cspf = network
                .node(fanout)?
                .cspf
                .as_ref()
                .and_then(|cspf| cspf.bdd.clone());
            if let Some(fanout_cspf) = fanout_cspf {
                edge_dc = BoolExpr::or(fanout_cspf, edge_dc);
            }
            fdc = Some(match fdc {
                Some(current) => BoolExpr::and(current, edge_dc),
                None => edge_dc,
            });
        }

        network
            .node_mut(node)?
            .cspf
            .as_mut()
            .ok_or(ComputeDcError::MissingCspf(node))?
            .bdd = Some(fdc.unwrap_or_else(BoolExpr::zero));
        report.cspf(node);

        if network.node(node)?.kind == ComputeNodeKind::Internal {
            report.simplified(node);
            rebuild_mspf_list(network, node, &mut report)?;
            update_cspf_of_fanins_native(network, node, &mut report)?;
        }
    }

    Ok(report)
}

pub fn compute_dc_in_sis_network(
    network: &mut ComputeNetwork,
    mode: ComputeDcMode,
) -> Result<ComputeDcReport, ComputeDcError> {
    match mode {
        ComputeDcMode::WithoutOdc => simplify_without_odc_native(network),
        ComputeDcMode::WithOdc => simplify_with_odc_native(network),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConeDirection {
    Fanin,
    Fanout,
}

fn walk_cone(
    network: &ComputeNetwork,
    node: ComputeNodeId,
    limit: usize,
    direction: ConeDirection,
) -> Result<Vec<ComputeNodeId>, ComputeDcError> {
    network.node(node)?;
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    let mut queue = VecDeque::from([(node, 0usize)]);
    while let Some((current, depth)) = queue.pop_front() {
        if depth == limit {
            continue;
        }
        let neighbors = match direction {
            ConeDirection::Fanin => &network.node(current)?.fanins,
            ConeDirection::Fanout => &network.node(current)?.fanouts,
        };
        for neighbor in neighbors.iter().copied() {
            network.node(neighbor)?;
            if seen.insert(neighbor) {
                ordered.push(neighbor);
                queue.push_back((neighbor, depth.saturating_add(1)));
            }
        }
    }
    Ok(ordered)
}

fn dfs_from_inputs(network: &ComputeNetwork) -> Result<Vec<ComputeNodeId>, ComputeDcError> {
    let mut indegree = network
        .nodes()
        .iter()
        .map(|node| node.fanins.len())
        .collect::<Vec<_>>();
    let mut queue = VecDeque::new();
    for (index, node) in network.nodes().iter().enumerate() {
        if node.kind == ComputeNodeKind::PrimaryInput || indegree[index] == 0 {
            queue.push_back(ComputeNodeId(index));
        }
    }

    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    while let Some(node) = queue.pop_front() {
        if !seen.insert(node) {
            continue;
        }
        ordered.push(node);
        for fanout in network.node(node)?.fanouts.iter().copied() {
            indegree[fanout.0] = indegree[fanout.0].saturating_sub(1);
            if indegree[fanout.0] == 0 {
                queue.push_back(fanout);
            }
        }
    }

    for index in 0..network.nodes().len() {
        let node = ComputeNodeId(index);
        if seen.insert(node) {
            ordered.push(node);
        }
    }
    Ok(ordered)
}

fn ensure_cspf_slots(network: &mut ComputeNetwork) -> Result<(), ComputeDcError> {
    for index in 0..network.nodes().len() {
        let node = ComputeNodeId(index);
        if network.node(node)?.cspf.is_none() {
            cspf_alloc(network, node)?;
        }
    }
    Ok(())
}

fn rebuild_mspf_list(
    network: &mut ComputeNetwork,
    node: ComputeNodeId,
    report: &mut ComputeDcReport,
) -> Result<(), ComputeDcError> {
    let node_data = network.node(node)?;
    let use_constant_mspf = node_data.fanins.len() > 15
        && node_data.fanins.len().saturating_mul(node_data.cover.len()) >= 300;
    let fanins = node_data.fanins.clone();

    let mut list = Vec::with_capacity(fanins.len());
    for fanin in fanins {
        let mspf =
            if network.node(fanin)?.kind == ComputeNodeKind::PrimaryInput || use_constant_mspf {
                DoubleNode {
                    pos: BoolExpr::zero(),
                    neg: BoolExpr::one(),
                }
            } else {
                compute_mspf(network, node, fanin)?
            };
        report.mspf(node, fanin);
        list.push(mspf);
    }

    network
        .node_mut(node)?
        .cspf
        .as_mut()
        .ok_or(ComputeDcError::MissingCspf(node))?
        .list = list;
    Ok(())
}

fn update_cspf_of_fanins_native(
    network: &mut ComputeNetwork,
    node: ComputeNodeId,
    report: &mut ComputeDcReport,
) -> Result<(), ComputeDcError> {
    let fanins = network.node(node)?.fanins.clone();
    let cspf_list = network
        .node(node)?
        .cspf
        .as_ref()
        .ok_or(ComputeDcError::MissingCspf(node))?
        .list
        .clone();
    let node_cspf = network
        .node(node)?
        .cspf
        .as_ref()
        .and_then(|cspf| cspf.bdd.clone());

    for (index, fanin) in fanins.iter().copied().enumerate() {
        if network.node(fanin)?.kind == ComputeNodeKind::PrimaryInput {
            continue;
        }
        let Some(fanin_cspf) = network
            .node(fanin)?
            .cspf
            .as_ref()
            .and_then(|cspf| cspf.bdd.clone())
        else {
            continue;
        };

        let Some(mspf) = cspf_list.get(index) else {
            continue;
        };
        let mut edge_dc = mspf.pos.clone();
        for previous_index in 0..index {
            let pfanin = fanins[previous_index];
            if network.node(pfanin)?.kind == ComputeNodeKind::PrimaryInput {
                continue;
            }
            let Some(previous_mspf) = cspf_list.get(previous_index) else {
                continue;
            };
            let previous = network
                .node(pfanin)?
                .cspf
                .as_ref()
                .and_then(|cspf| cspf.bdd.as_ref());
            edge_dc = compute_cspf_compatible(edge_dc, pfanin, previous_mspf.neg.clone(), previous);
        }

        if let Some(node_cspf) = node_cspf.clone() {
            edge_dc = BoolExpr::or(node_cspf, edge_dc);
        }
        let updated = BoolExpr::and(fanin_cspf, edge_dc);
        network
            .node_mut(fanin)?
            .cspf
            .as_mut()
            .ok_or(ComputeDcError::MissingCspf(fanin))?
            .bdd = Some(updated);
        report.updated_fanin(fanin);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(node: ComputeNodeId) -> BoolExpr {
        BoolExpr::literal(node, true)
    }

    fn cube(inputs: &[CubeLiteral]) -> CoverCube {
        CoverCube::new(inputs.to_vec())
    }

    fn allocated_two_input_network() -> (ComputeNetwork, ComputeNodeId, ComputeNodeId, ComputeNodeId)
    {
        let mut network = ComputeNetwork::new();
        let a = network.add_node(ComputeNode::primary_input("a"));
        let b = network.add_node(ComputeNode::primary_input("b"));
        let f = network.add_node(ComputeNode::internal(
            "f",
            vec![
                cube(&[CubeLiteral::One, CubeLiteral::DontCare]),
                cube(&[CubeLiteral::Zero, CubeLiteral::One]),
            ],
        ));
        network.connect(a, f).unwrap();
        network.connect(b, f).unwrap();
        (network, a, b, f)
    }

    #[test]
    fn alloc_and_free_match_cspf_and_odc_initializers() {
        let (mut network, _, _, f) = allocated_two_input_network();

        cspf_alloc(&mut network, f).unwrap();
        odc_alloc(&mut network, f).unwrap();

        assert_eq!(network.node(f).unwrap().cspf, Some(CspfSlot::allocated()));
        assert_eq!(network.node(f).unwrap().odc, Some(OdcSlot::allocated()));

        cspf_free(&mut network, f).unwrap();
        odc_free(&mut network, f).unwrap();

        assert!(network.node(f).unwrap().cspf.is_none());
        assert!(network.node(f).unwrap().odc.is_none());
    }

    #[test]
    fn cube_to_expr_smooths_selected_fanin_and_dont_cares() {
        let (network, a, b, f) = allocated_two_input_network();

        assert_eq!(
            cube_to_expr(
                &network,
                f,
                &cube(&[CubeLiteral::Zero, CubeLiteral::One]),
                Some(0),
            ),
            Ok(lit(b))
        );
        assert_eq!(
            cube_to_expr(
                &network,
                f,
                &cube(&[CubeLiteral::Zero, CubeLiteral::One]),
                None,
            ),
            Ok(BoolExpr::and(BoolExpr::literal(a, false), lit(b)))
        );
    }

    #[test]
    fn compute_mspf_partitions_cover_by_fanin_literal() {
        let mut network = ComputeNetwork::new();
        let a = network.add_node(ComputeNode::primary_input("a"));
        let x = network.add_node(ComputeNode::internal("x", vec![cube(&[CubeLiteral::One])]));
        let f = network.add_node(ComputeNode::internal(
            "f",
            vec![
                cube(&[CubeLiteral::One, CubeLiteral::DontCare]),
                cube(&[CubeLiteral::Zero, CubeLiteral::One]),
            ],
        ));
        network.connect(a, x).unwrap();
        network.connect(x, f).unwrap();
        network.connect(a, f).unwrap();

        assert_eq!(
            compute_mspf(&network, f, x).unwrap(),
            DoubleNode {
                pos: BoolExpr::xnor(BoolExpr::one(), lit(a)),
                neg: BoolExpr::not(BoolExpr::xnor(BoolExpr::one(), lit(a))),
            }
        );
        assert_eq!(
            compute_mspf(&network, f, a).unwrap(),
            DoubleNode {
                pos: BoolExpr::zero(),
                neg: BoolExpr::one(),
            }
        );
    }

    #[test]
    fn compute_cspf_compatible_matches_c_formula() {
        let x = ComputeNodeId(7);
        let y = ComputeNodeId(8);
        let dc = BoolExpr::or(lit(x), lit(y));
        let diff = BoolExpr::literal(y, false);

        assert_eq!(
            compute_cspf_compatible(dc.clone(), x, diff.clone(), None),
            dc
        );
        assert_eq!(
            compute_cspf_compatible(dc.clone(), x, diff.clone(), Some(&BoolExpr::zero())),
            dc
        );
        assert_eq!(
            compute_cspf_compatible(dc, x, diff.clone(), Some(&BoolExpr::one())),
            BoolExpr::or(lit(y), BoolExpr::and(BoolExpr::or(lit(x), lit(y)), diff))
        );
    }

    #[test]
    fn inout_table_contains_transitive_fanins_and_their_fanouts() {
        let mut network = ComputeNetwork::new();
        let a = network.add_node(ComputeNode::primary_input("a"));
        let b = network.add_node(ComputeNode::primary_input("b"));
        let x = network.add_node(ComputeNode::internal("x", vec![]));
        let y = network.add_node(ComputeNode::internal("y", vec![]));
        let f = network.add_node(ComputeNode::internal("f", vec![]));
        network.connect(a, x).unwrap();
        network.connect(x, f).unwrap();
        network.connect(b, y).unwrap();
        network.connect(y, f).unwrap();

        assert_eq!(
            cspf_inout_table(&network, f, 1, 1).unwrap(),
            BTreeSet::from([x, y, f])
        );
    }

    #[test]
    fn find_odc_level_sets_pi_zero_and_internal_max_fanin_plus_one() {
        let mut network = ComputeNetwork::new();
        let a = network.add_node(ComputeNode::primary_input("a"));
        let b = network.add_node(ComputeNode::primary_input("b"));
        let x = network.add_node(ComputeNode::internal("x", vec![]));
        let f = network.add_node(ComputeNode::internal("f", vec![]));
        for node in [a, b, x, f] {
            odc_alloc(&mut network, node).unwrap();
        }
        network.connect(a, x).unwrap();
        network.connect(b, x).unwrap();
        network.connect(x, f).unwrap();

        find_odc_level(&mut network).unwrap();

        assert_eq!(network.node(a).unwrap().odc.as_ref().unwrap().level, 0);
        assert_eq!(network.node(x).unwrap().odc.as_ref().unwrap().level, 1);
        assert_eq!(network.node(f).unwrap().odc.as_ref().unwrap().level, 2);
    }

    #[test]
    fn level_comparators_match_c_sort_directions() {
        let mut network = ComputeNetwork::new();
        let low = network.add_node(ComputeNode::internal("low", vec![cube(&[])]));
        let high = network.add_node(ComputeNode::internal("high", vec![cube(&[]), cube(&[])]));
        for node in [low, high] {
            odc_alloc(&mut network, node).unwrap();
        }
        network.node_mut(low).unwrap().odc.as_mut().unwrap().level = 1;
        network.node_mut(low).unwrap().odc.as_mut().unwrap().value = 10;
        network.node_mut(low).unwrap().odc.as_mut().unwrap().order = 1;
        network.node_mut(high).unwrap().odc.as_mut().unwrap().level = 2;
        network.node_mut(high).unwrap().odc.as_mut().unwrap().value = 1;
        network.node_mut(high).unwrap().odc.as_mut().unwrap().order = 0;

        assert_eq!(level_node_cmp1(&network, high, low), Ok(Ordering::Less));
        assert_eq!(level_node_cmp2(&network, high, low), Ok(Ordering::Less));
        assert_eq!(level_node_cmp3(&network, low, high), Ok(Ordering::Less));
    }

    #[test]
    fn odc_value_is_infinite_for_output_only_fanout_otherwise_counts_uses() {
        let mut network = ComputeNetwork::new();
        let f = network.add_node(ComputeNode::internal("f", vec![]));
        let g = network.add_node(ComputeNode::internal("g", vec![]));
        let out = network.add_node(ComputeNode::primary_output("out"));
        network.connect(f, out).unwrap();

        assert_eq!(odc_value(&network, out), Ok(INFINITY_VALUE));
        assert_eq!(odc_value(&network, f), Ok(INFINITY_VALUE));

        network.connect(f, g).unwrap();
        assert_eq!(odc_value(&network, f), Ok(1));
    }

    #[test]
    fn cspf_bdd_dc_returns_stored_bdd_or_zero_for_allocated_slot() {
        let (mut network, _, b, f) = allocated_two_input_network();
        cspf_alloc(&mut network, f).unwrap();

        assert_eq!(cspf_bdd_dc(&network, f), Ok(BoolExpr::zero()));
        network.node_mut(f).unwrap().cspf.as_mut().unwrap().bdd = Some(lit(b));
        assert_eq!(cspf_bdd_dc(&network, f), Ok(lit(b)));
    }

    #[test]
    fn simplify_without_odc_sets_cspf_from_immediate_fanouts() {
        let (mut network, _a, _b, f) = allocated_two_input_network();
        let out = network.add_node(ComputeNode::primary_output("out"));
        network.connect(f, out).unwrap();
        cspf_alloc(&mut network, out).unwrap();
        network.node_mut(out).unwrap().cspf.as_mut().unwrap().bdd = Some(BoolExpr::one());

        let report = simplify_without_odc_native(&mut network).unwrap();

        assert!(report.visited_nodes.contains(&f));
        assert!(report.simplified_nodes.contains(&f));
        assert_eq!(cspf_bdd_dc(&network, f), Ok(BoolExpr::one()));
    }

    #[test]
    fn simplify_with_odc_builds_mspf_lists_and_updates_fanin_cspf() {
        let mut network = ComputeNetwork::new();
        let a = network.add_node(ComputeNode::primary_input("a"));
        let x = network.add_node(ComputeNode::internal("x", vec![cube(&[CubeLiteral::One])]));
        let f = network.add_node(ComputeNode::internal(
            "f",
            vec![
                cube(&[CubeLiteral::One, CubeLiteral::DontCare]),
                cube(&[CubeLiteral::Zero, CubeLiteral::One]),
            ],
        ));
        let out = network.add_node(ComputeNode::primary_output("out"));
        network.connect(a, x).unwrap();
        network.connect(x, f).unwrap();
        network.connect(a, f).unwrap();
        network.connect(f, out).unwrap();

        let report = compute_dc_in_sis_network(&mut network, ComputeDcMode::WithOdc).unwrap();

        assert!(report.simplified_nodes.contains(&x));
        assert!(report.simplified_nodes.contains(&f));
        assert!(report.mspf_edges.contains(&(f, x)));
        assert_eq!(
            network.node(f).unwrap().cspf.as_ref().unwrap().list.len(),
            2
        );
        assert!(
            network
                .node(x)
                .unwrap()
                .cspf
                .as_ref()
                .unwrap()
                .bdd
                .is_some()
        );
    }
}
