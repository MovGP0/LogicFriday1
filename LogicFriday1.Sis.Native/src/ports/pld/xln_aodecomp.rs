//! Native Rust model for `LogicSynthesis/sis/pld/xln_aodecomp.c`.
//!
//! The C file rewrites SIS `node_t` objects in-place: AND/OR nodes are split
//! through `balanced_tree`, and complex SOP nodes are converted into ORs of
//! cube AND terms before any oversized term/root is split. This module ports
//! those deterministic planning rules to owned Rust data. Direct `network_t`
//! and `node_t` mutation remains an explicit missing-dependency error until
//! the lower-level SIS node/network ports are native.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AoOperation {
    PldDecompAndOr,
    DecompAnd,
    DecompOr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    And,
    Or,
    Complex,
    Constant0,
    Constant1,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase {
    Positive,
    Negative,
    Binate,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CubeLiteral {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Positive,
    Negative,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AoLiteral {
    pub fanin: String,
    pub phase: LiteralPhase,
}

impl AoLiteral {
    pub fn new(fanin: impl Into<String>, phase: LiteralPhase) -> Self {
        Self {
            fanin: fanin.into(),
            phase,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AoNode {
    pub name: String,
    pub function: NodeFunction,
    pub fanins: Vec<String>,
    pub input_phases: Vec<InputPhase>,
    pub cubes: Vec<Vec<CubeLiteral>>,
}

impl AoNode {
    pub fn new(name: impl Into<String>, function: NodeFunction, fanins: Vec<String>) -> Self {
        let input_phases = vec![InputPhase::Positive; fanins.len()];
        Self {
            name: name.into(),
            function,
            fanins,
            input_phases,
            cubes: Vec::new(),
        }
    }

    pub fn with_input_phases(mut self, input_phases: Vec<InputPhase>) -> Self {
        self.input_phases = input_phases;
        self
    }

    pub fn with_cubes(mut self, cubes: Vec<Vec<CubeLiteral>>) -> Self {
        self.cubes = cubes;
        self
    }

    pub fn fanin_count(&self) -> usize {
        self.fanins.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateKind {
    And,
    Or,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeNode {
    pub id: usize,
    pub kind: TreeNodeKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreeNodeKind {
    Leaf {
        leaf_index: usize,
    },
    Gate {
        gate: GateKind,
        children: Vec<usize>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalancedTreePlan {
    pub gate: GateKind,
    pub leaf_count: usize,
    pub branching_factor: usize,
    pub root: usize,
    pub nodes: Vec<TreeNode>,
    pub leaves: Vec<usize>,
}

impl BalancedTreePlan {
    pub fn gate_fanin(&self, id: usize) -> Option<usize> {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .and_then(|node| match &node.kind {
                TreeNodeKind::Gate { children, .. } => Some(children.len()),
                TreeNodeKind::Leaf { .. } => None,
            })
    }

    pub fn max_gate_fanin(&self) -> usize {
        self.nodes
            .iter()
            .filter_map(|node| match &node.kind {
                TreeNodeKind::Gate { children, .. } => Some(children.len()),
                TreeNodeKind::Leaf { .. } => None,
            })
            .max()
            .unwrap_or(0)
    }

    pub fn internal_gate_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| matches!(node.kind, TreeNodeKind::Gate { .. }))
            .count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateDecompositionPlan {
    pub gate: GateKind,
    pub literals: Vec<AoLiteral>,
    pub tree: BalancedTreePlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeTermPlan {
    pub cube_index: usize,
    pub literals: Vec<AoLiteral>,
    pub and_decomposition: Option<GateDecompositionPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AoDecompositionPlan {
    And(GateDecompositionPlan),
    Or(GateDecompositionPlan),
    Complex {
        cube_terms: Vec<CubeTermPlan>,
        root_or_decomposition: Option<BalancedTreePlan>,
    },
    ConstantAfterMerge {
        function: NodeFunction,
    },
    Noop {
        function: NodeFunction,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnAoDecompError {
    InvalidFaninLimit {
        limit: usize,
    },
    NonTerminatingBranchingFactor {
        leaves: usize,
        branching_factor: usize,
    },
    InvalidLeafCount {
        leaves: usize,
    },
    FunctionTypeMismatch {
        expected: GateKind,
        actual: NodeFunction,
    },
    PhaseArityMismatch {
        fanins: usize,
        phases: usize,
    },
    CubeArityMismatch {
        cube_index: usize,
        fanins: usize,
        literals: usize,
    },
    InvalidInputPhase {
        fanin: String,
        phase: InputPhase,
        gate: GateKind,
    },
    MissingNativePorts {
        operation: AoOperation,
    },
}

impl fmt::Display for XlnAoDecompError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFaninLimit { limit } => {
                write!(
                    f,
                    "AO decomposition fanin limit must be positive, got {limit}"
                )
            }
            Self::NonTerminatingBranchingFactor {
                leaves,
                branching_factor,
            } => write!(
                f,
                "balanced_tree with {leaves} leaves cannot terminate with branching factor {branching_factor}"
            ),
            Self::InvalidLeafCount { leaves } => {
                write!(f, "balanced_tree requires at least one leaf, got {leaves}")
            }
            Self::FunctionTypeMismatch { expected, actual } => {
                write!(
                    f,
                    "expected {expected:?} node for AO decomposition, got {actual:?}"
                )
            }
            Self::PhaseArityMismatch { fanins, phases } => {
                write!(f, "node has {fanins} fanins but {phases} input phases")
            }
            Self::CubeArityMismatch {
                cube_index,
                fanins,
                literals,
            } => write!(
                f,
                "cube {cube_index} has {literals} literals for {fanins} fanins"
            ),
            Self::InvalidInputPhase { fanin, phase, gate } => {
                write!(
                    f,
                    "{gate:?} decomposition cannot wire fanin {fanin} with phase {phase:?}"
                )
            }
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation:?} requires native Rust ports for SIS dependencies"
            ),
        }
    }
}

impl Error for XlnAoDecompError {}

pub fn pld_decomp_and_or_blocked<Network, Node>(
    _network: &mut Network,
    _node: &mut Node,
    _size: usize,
) -> Result<(), XlnAoDecompError> {
    missing_native_ports(AoOperation::PldDecompAndOr)
}

pub fn decomp_and_blocked<Network, Node>(
    _network: &mut Network,
    _node: &mut Node,
    _and_limit: usize,
) -> Result<(), XlnAoDecompError> {
    missing_native_ports(AoOperation::DecompAnd)
}

pub fn decomp_or_blocked<Network, Node>(
    _network: &mut Network,
    _node: &mut Node,
    _or_limit: usize,
) -> Result<(), XlnAoDecompError> {
    missing_native_ports(AoOperation::DecompOr)
}

fn missing_native_ports<T>(operation: AoOperation) -> Result<T, XlnAoDecompError> {
    Err(XlnAoDecompError::MissingNativePorts { operation })
}

pub fn pld_decomp_and_or_plan(
    node: &AoNode,
    size: usize,
) -> Result<AoDecompositionPlan, XlnAoDecompError> {
    validate_limit(size)?;
    match node.function {
        NodeFunction::And => decomp_and_plan(node, size).map(AoDecompositionPlan::And),
        NodeFunction::Or => decomp_or_plan(node, size).map(AoDecompositionPlan::Or),
        NodeFunction::Complex => decomp_complex_plan(node, size),
        NodeFunction::Constant0 | NodeFunction::Constant1 => {
            Ok(AoDecompositionPlan::ConstantAfterMerge {
                function: node.function,
            })
        }
        NodeFunction::Other => Ok(AoDecompositionPlan::Noop {
            function: node.function,
        }),
    }
}

pub fn decomp_and_plan(
    node: &AoNode,
    and_limit: usize,
) -> Result<GateDecompositionPlan, XlnAoDecompError> {
    decomp_gate_plan(node, and_limit, GateKind::And, NodeFunction::And)
}

pub fn decomp_or_plan(
    node: &AoNode,
    or_limit: usize,
) -> Result<GateDecompositionPlan, XlnAoDecompError> {
    decomp_gate_plan(node, or_limit, GateKind::Or, NodeFunction::Or)
}

pub fn balanced_tree_plan(
    leaves: usize,
    branching_factor: usize,
    gate: GateKind,
) -> Result<BalancedTreePlan, XlnAoDecompError> {
    validate_limit(branching_factor)?;
    if leaves == 0 {
        return Err(XlnAoDecompError::InvalidLeafCount { leaves });
    }
    if leaves > 1 && branching_factor <= 1 {
        return Err(XlnAoDecompError::NonTerminatingBranchingFactor {
            leaves,
            branching_factor,
        });
    }

    let mut builder = TreeBuilder::new(gate, branching_factor);
    let root = builder.build(leaves);
    Ok(BalancedTreePlan {
        gate,
        leaf_count: leaves,
        branching_factor,
        root,
        nodes: builder.nodes,
        leaves: builder.leaves,
    })
}

pub fn dec_node_cube_plan(
    node: &AoNode,
    cube_index: usize,
) -> Result<Vec<AoLiteral>, XlnAoDecompError> {
    let cube = node
        .cubes
        .get(cube_index)
        .ok_or(XlnAoDecompError::CubeArityMismatch {
            cube_index,
            fanins: node.fanins.len(),
            literals: 0,
        })?;
    if cube.len() != node.fanins.len() {
        return Err(XlnAoDecompError::CubeArityMismatch {
            cube_index,
            fanins: node.fanins.len(),
            literals: cube.len(),
        });
    }

    let mut literals = Vec::new();
    for (fanin, literal) in node.fanins.iter().zip(cube) {
        match literal {
            CubeLiteral::Zero => literals.push(AoLiteral::new(fanin, LiteralPhase::Negative)),
            CubeLiteral::One => literals.push(AoLiteral::new(fanin, LiteralPhase::Positive)),
            CubeLiteral::DontCare => {}
        }
    }
    Ok(literals)
}

fn decomp_gate_plan(
    node: &AoNode,
    limit: usize,
    gate: GateKind,
    expected: NodeFunction,
) -> Result<GateDecompositionPlan, XlnAoDecompError> {
    validate_limit(limit)?;
    if node.function != expected {
        return Err(XlnAoDecompError::FunctionTypeMismatch {
            expected: gate,
            actual: node.function,
        });
    }
    if node.fanins.len() != node.input_phases.len() {
        return Err(XlnAoDecompError::PhaseArityMismatch {
            fanins: node.fanins.len(),
            phases: node.input_phases.len(),
        });
    }

    let mut literals = Vec::with_capacity(node.fanins.len());
    for (fanin, phase) in node.fanins.iter().zip(&node.input_phases) {
        let phase = match phase {
            InputPhase::Positive => LiteralPhase::Positive,
            InputPhase::Negative => LiteralPhase::Negative,
            InputPhase::Binate | InputPhase::Unknown => {
                return Err(XlnAoDecompError::InvalidInputPhase {
                    fanin: fanin.clone(),
                    phase: *phase,
                    gate,
                });
            }
        };
        literals.push(AoLiteral::new(fanin, phase));
    }

    Ok(GateDecompositionPlan {
        gate,
        literals,
        tree: balanced_tree_plan(node.fanin_count(), limit, gate)?,
    })
}

fn decomp_complex_plan(
    node: &AoNode,
    size: usize,
) -> Result<AoDecompositionPlan, XlnAoDecompError> {
    let mut cube_terms = Vec::with_capacity(node.cubes.len());

    for cube_index in (0..node.cubes.len()).rev() {
        let literals = dec_node_cube_plan(node, cube_index)?;
        let and_decomposition = if literals.len() > size {
            let cube_node = AoNode::new(
                format!("{}_cube_{cube_index}", node.name),
                NodeFunction::And,
                literals
                    .iter()
                    .map(|literal| literal.fanin.clone())
                    .collect(),
            )
            .with_input_phases(
                literals
                    .iter()
                    .map(|literal| match literal.phase {
                        LiteralPhase::Positive => InputPhase::Positive,
                        LiteralPhase::Negative => InputPhase::Negative,
                    })
                    .collect(),
            );
            Some(decomp_and_plan(&cube_node, size)?)
        } else {
            None
        };
        cube_terms.push(CubeTermPlan {
            cube_index,
            literals,
            and_decomposition,
        });
    }

    let root_or_decomposition = if cube_terms.len() > size {
        Some(balanced_tree_plan(cube_terms.len(), size, GateKind::Or)?)
    } else {
        None
    };

    Ok(AoDecompositionPlan::Complex {
        cube_terms,
        root_or_decomposition,
    })
}

fn validate_limit(limit: usize) -> Result<(), XlnAoDecompError> {
    if limit == 0 {
        Err(XlnAoDecompError::InvalidFaninLimit { limit })
    } else {
        Ok(())
    }
}

struct TreeBuilder {
    gate: GateKind,
    branching_factor: usize,
    nodes: Vec<TreeNode>,
    leaves: Vec<usize>,
    next_leaf: usize,
}

impl TreeBuilder {
    fn new(gate: GateKind, branching_factor: usize) -> Self {
        Self {
            gate,
            branching_factor,
            nodes: Vec::new(),
            leaves: Vec::new(),
            next_leaf: 0,
        }
    }

    fn build(&mut self, leaves: usize) -> usize {
        if leaves == 1 {
            let id = self.push(TreeNodeKind::Leaf {
                leaf_index: self.next_leaf,
            });
            self.next_leaf += 1;
            self.leaves.push(id);
            return id;
        }

        let split_size = leaves.div_ceil(self.branching_factor);
        let mut remaining = leaves;
        let mut children = Vec::new();
        while remaining > 0 {
            let child_leaves = split_size.min(remaining);
            remaining = remaining.saturating_sub(split_size);
            children.push(self.build(child_leaves));
        }

        self.push(TreeNodeKind::Gate {
            gate: self.gate,
            children,
        })
    }

    fn push(&mut self, kind: TreeNodeKind) -> usize {
        let id = self.nodes.len();
        self.nodes.push(TreeNode { id, kind });
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn and_node() -> AoNode {
        AoNode::new("f", NodeFunction::And, names(&["a", "b", "c", "d"])).with_input_phases(vec![
            InputPhase::Positive,
            InputPhase::Negative,
            InputPhase::Positive,
            InputPhase::Negative,
        ])
    }

    #[test]
    fn balanced_tree_keeps_every_gate_within_the_requested_fanin_limit() {
        let tree = balanced_tree_plan(7, 3, GateKind::And).unwrap();

        assert_eq!(tree.leaf_count, 7);
        assert_eq!(tree.leaves.len(), 7);
        assert!(tree.max_gate_fanin() <= 3);
        assert_eq!(tree.gate_fanin(tree.root), Some(3));
    }

    #[test]
    fn decomp_and_wires_positive_and_negative_unate_inputs_as_literals() {
        let plan = decomp_and_plan(&and_node(), 2).unwrap();

        assert_eq!(plan.gate, GateKind::And);
        assert_eq!(
            plan.literals,
            vec![
                AoLiteral::new("a", LiteralPhase::Positive),
                AoLiteral::new("b", LiteralPhase::Negative),
                AoLiteral::new("c", LiteralPhase::Positive),
                AoLiteral::new("d", LiteralPhase::Negative),
            ]
        );
        assert!(plan.tree.max_gate_fanin() <= 2);
    }

    #[test]
    fn decomp_or_rejects_binate_input_phase_like_c_debug_path() {
        let node = AoNode::new("f", NodeFunction::Or, names(&["a", "b"]))
            .with_input_phases(vec![InputPhase::Positive, InputPhase::Binate]);

        assert_eq!(
            decomp_or_plan(&node, 2),
            Err(XlnAoDecompError::InvalidInputPhase {
                fanin: "b".to_owned(),
                phase: InputPhase::Binate,
                gate: GateKind::Or,
            })
        );
    }

    #[test]
    fn dec_node_cube_plan_matches_zero_one_and_dont_care_literals() {
        let node =
            AoNode::new("f", NodeFunction::Complex, names(&["a", "b", "c"])).with_cubes(vec![
                vec![CubeLiteral::One, CubeLiteral::DontCare, CubeLiteral::Zero],
            ]);

        assert_eq!(
            dec_node_cube_plan(&node, 0).unwrap(),
            vec![
                AoLiteral::new("a", LiteralPhase::Positive),
                AoLiteral::new("c", LiteralPhase::Negative),
            ]
        );
    }

    #[test]
    fn complex_plan_visits_cubes_in_reverse_order_and_splits_oversized_terms() {
        let node =
            AoNode::new("f", NodeFunction::Complex, names(&["a", "b", "c"])).with_cubes(vec![
                vec![CubeLiteral::One, CubeLiteral::DontCare, CubeLiteral::Zero],
                vec![CubeLiteral::One, CubeLiteral::One, CubeLiteral::Zero],
                vec![CubeLiteral::DontCare, CubeLiteral::One, CubeLiteral::Zero],
            ]);

        let AoDecompositionPlan::Complex {
            cube_terms,
            root_or_decomposition,
        } = pld_decomp_and_or_plan(&node, 2).unwrap()
        else {
            panic!("expected complex decomposition plan");
        };

        assert_eq!(
            cube_terms
                .iter()
                .map(|term| term.cube_index)
                .collect::<Vec<_>>(),
            vec![2, 1, 0]
        );
        assert!(cube_terms[0].and_decomposition.is_none());
        assert!(cube_terms[1].and_decomposition.is_some());
        assert!(root_or_decomposition.is_some());
    }

    #[test]
    fn invalid_limits_and_cube_arity_are_explicit() {
        assert_eq!(
            pld_decomp_and_or_plan(&and_node(), 0),
            Err(XlnAoDecompError::InvalidFaninLimit { limit: 0 })
        );
        assert_eq!(
            balanced_tree_plan(2, 1, GateKind::And),
            Err(XlnAoDecompError::NonTerminatingBranchingFactor {
                leaves: 2,
                branching_factor: 1,
            })
        );

        let node = AoNode::new("f", NodeFunction::Complex, names(&["a", "b"]))
            .with_cubes(vec![vec![CubeLiteral::One]]);
        assert_eq!(
            dec_node_cube_plan(&node, 0),
            Err(XlnAoDecompError::CubeArityMismatch {
                cube_index: 0,
                fanins: 2,
                literals: 1,
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_aodecomp.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
