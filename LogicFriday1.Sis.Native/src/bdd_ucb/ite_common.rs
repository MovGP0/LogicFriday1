use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

pub type BddVariableId = u32;
pub type BddNodeId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BddEdge {
    node: BddNodeId,
    complemented: bool,
}

impl BddEdge {
    pub const fn regular(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: false,
        }
    }

    pub const fn complemented(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: true,
        }
    }

    pub const fn node(self) -> BddNodeId {
        self.node
    }

    pub const fn is_complemented(self) -> bool {
        self.complemented
    }

    pub const fn not(self) -> Self {
        Self {
            node: self.node,
            complemented: !self.complemented,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BddNode {
    Constant(bool),
    Branch {
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    },
}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    one: BddEdge,
    zero: BddEdge,
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            zero: BddEdge::regular(0),
            one: BddEdge::regular(1),
        }
    }

    pub fn zero(&self) -> BddEdge {
        self.zero
    }

    pub fn one(&self) -> BddEdge {
        self.one
    }

    pub fn add_branch(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> BddEdge {
        let edge = BddEdge::regular(self.nodes.len());
        self.nodes.push(BddNode::Branch {
            variable,
            then_edge,
            else_edge,
        });
        edge
    }

    pub fn node(&self, edge: BddEdge) -> Result<&BddNode, IteCommonError> {
        self.nodes
            .get(edge.node)
            .ok_or(IteCommonError::MissingNode(edge.node))
    }

    pub fn is_constant(&self, edge: BddEdge) -> Result<bool, IteCommonError> {
        Ok(matches!(self.node(edge)?, BddNode::Constant(_)))
    }

    pub fn variable(&self, edge: BddEdge) -> Result<BddVariableId, IteCommonError> {
        match self.node(edge)? {
            BddNode::Constant(_) => Err(IteCommonError::ExpectedBranch(edge.node)),
            BddNode::Branch { variable, .. } => Ok(*variable),
        }
    }
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IteInputs {
    pub f: BddEdge,
    pub g: BddEdge,
    pub h: BddEdge,
}

impl IteInputs {
    pub const fn new(f: BddEdge, g: BddEdge, h: BddEdge) -> Self {
        Self { f, g, h }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanonicalIteInputs {
    pub inputs: IteInputs,
    pub complemented: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IteCommonError {
    MissingNode(BddNodeId),
    ExpectedBranch(BddNodeId),
}

impl fmt::Display for IteCommonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::ExpectedBranch(node) => write!(formatter, "BDD node {node} is not a branch node"),
        }
    }
}

impl Error for IteCommonError {}

pub fn quick_cofactor(
    manager: &BddManager,
    f: BddEdge,
    variable: BddVariableId,
) -> Result<(BddEdge, BddEdge), IteCommonError> {
    let regular = BddEdge::regular(f.node);
    match manager.node(regular)? {
        BddNode::Branch {
            variable: node_variable,
            then_edge,
            else_edge,
        } if *node_variable == variable => {
            if f.is_complemented() {
                Ok((then_edge.not(), else_edge.not()))
            } else {
                Ok((*then_edge, *else_edge))
            }
        }
        _ => Ok((f, f)),
    }
}

pub fn var_to_const(manager: &BddManager, f: BddEdge, g: &mut BddEdge, h: &mut BddEdge) {
    if f == *g {
        *g = manager.one();
    } else if f == g.not() {
        *g = manager.zero();
    }

    if f == *h {
        *h = manager.zero();
    } else if f == h.not() {
        *h = manager.one();
    }
}

pub fn canonicalize_ite_inputs(
    manager: &BddManager,
    mut inputs: IteInputs,
) -> Result<CanonicalIteInputs, IteCommonError> {
    if manager.is_constant(inputs.g)? {
        if greater_than_or_equal(manager, inputs.f, inputs.h)? {
            if inputs.g == manager.one() {
                swap(&mut inputs.h, &mut inputs.f);
            } else {
                swap(&mut inputs.h, &mut inputs.f);
                inputs.f = inputs.f.not();
                inputs.h = inputs.h.not();
            }
        }
    } else if manager.is_constant(inputs.h)? {
        if greater_than_or_equal(manager, inputs.f, inputs.g)? {
            if inputs.h == manager.one() {
                swap(&mut inputs.g, &mut inputs.f);
                inputs.f = inputs.f.not();
                inputs.g = inputs.g.not();
            } else {
                swap(&mut inputs.g, &mut inputs.f);
            }
        }
    } else if inputs.g == inputs.h.not() && greater_than_or_equal(manager, inputs.f, inputs.g)? {
        swap(&mut inputs.f, &mut inputs.g);
        inputs.h = inputs.g.not();
    }

    if inputs.f.is_complemented() {
        inputs.f = inputs.f.not();
        swap(&mut inputs.g, &mut inputs.h);
    }

    let complemented = inputs.g.is_complemented();
    if complemented {
        inputs.g = inputs.g.not();
        inputs.h = inputs.h.not();
    }

    Ok(CanonicalIteInputs {
        inputs,
        complemented,
    })
}

pub fn greater_than_or_equal(
    manager: &BddManager,
    f: BddEdge,
    g: BddEdge,
) -> Result<bool, IteCommonError> {
    let f_variable = sort_variable(manager, f)?;
    let g_variable = sort_variable(manager, g)?;

    Ok(match f_variable.cmp(&g_variable) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => {
            f.node > g.node || f.node == g.node && f.is_complemented() >= g.is_complemented()
        }
    })
}

fn sort_variable(manager: &BddManager, edge: BddEdge) -> Result<BddVariableId, IteCommonError> {
    match manager.node(BddEdge::regular(edge.node))? {
        BddNode::Constant(false) => Ok(BddVariableId::MAX - 1),
        BddNode::Constant(true) => Ok(BddVariableId::MAX),
        BddNode::Branch { variable, .. } => Ok(*variable),
    }
}

fn swap<T>(left: &mut T, right: &mut T) {
    std::mem::swap(left, right);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manager() -> (BddManager, BddEdge, BddEdge, BddEdge) {
        let mut manager = BddManager::new();
        let x = manager.add_branch(1, manager.one(), manager.zero());
        let y = manager.add_branch(2, manager.one(), x);
        let z = manager.add_branch(3, y, manager.zero());

        (manager, x, y, z)
    }

    #[test]
    fn quick_cofactor_returns_children_for_matching_regular_variable() {
        let (manager, x, _, _) = sample_manager();

        let (positive, negative) = quick_cofactor(&manager, x, 1).unwrap();

        assert_eq!(positive, manager.one());
        assert_eq!(negative, manager.zero());
    }

    #[test]
    fn quick_cofactor_complements_children_for_matching_complemented_variable() {
        let (manager, x, _, _) = sample_manager();

        let (positive, negative) = quick_cofactor(&manager, x.not(), 1).unwrap();

        assert_eq!(positive, manager.one().not());
        assert_eq!(negative, manager.zero().not());
    }

    #[test]
    fn quick_cofactor_returns_original_when_top_variable_differs() {
        let (manager, x, _, _) = sample_manager();

        assert_eq!(quick_cofactor(&manager, x, 2).unwrap(), (x, x));
    }

    #[test]
    fn var_to_const_rewrites_matching_then_and_else_arguments() {
        let (manager, x, y, _) = sample_manager();
        let mut g = x;
        let mut h = x.not();

        var_to_const(&manager, x, &mut g, &mut h);

        assert_eq!(g, manager.one());
        assert_eq!(h, manager.one());

        g = x.not();
        h = x;
        var_to_const(&manager, x, &mut g, &mut h);

        assert_eq!(g, manager.zero());
        assert_eq!(h, manager.zero());

        g = y;
        h = y;
        var_to_const(&manager, x, &mut g, &mut h);

        assert_eq!(g, y);
        assert_eq!(h, y);
    }

    #[test]
    fn canonicalize_swaps_when_then_branch_is_one_and_f_orders_after_h() {
        let (manager, x, _, z) = sample_manager();

        let canonical =
            canonicalize_ite_inputs(&manager, IteInputs::new(z, manager.one(), x)).unwrap();

        assert_eq!(canonical.inputs, IteInputs::new(x, manager.one(), z));
        assert!(!canonical.complemented);
    }

    #[test]
    fn canonicalize_complements_swap_when_then_branch_is_zero() {
        let (manager, x, _, z) = sample_manager();

        let canonical =
            canonicalize_ite_inputs(&manager, IteInputs::new(z, manager.zero(), x)).unwrap();

        assert_eq!(canonical.inputs, IteInputs::new(x, z, manager.zero().not()));
        assert!(canonical.complemented);
    }

    #[test]
    fn canonicalize_swaps_and_complements_when_else_branch_is_one() {
        let (manager, x, _, z) = sample_manager();

        let canonical =
            canonicalize_ite_inputs(&manager, IteInputs::new(z, x, manager.one())).unwrap();

        assert_eq!(canonical.inputs, IteInputs::new(x, manager.one(), z.not()));
        assert!(!canonical.complemented);
    }

    #[test]
    fn canonicalize_swaps_when_else_branch_is_zero() {
        let (manager, x, _, z) = sample_manager();

        let canonical =
            canonicalize_ite_inputs(&manager, IteInputs::new(z, x, manager.zero())).unwrap();

        assert_eq!(canonical.inputs, IteInputs::new(x, z, manager.zero()));
        assert!(!canonical.complemented);
    }

    #[test]
    fn canonicalize_xor_like_inputs_puts_g_first() {
        let (manager, x, y, _) = sample_manager();

        let canonical = canonicalize_ite_inputs(&manager, IteInputs::new(y, x, x.not())).unwrap();

        assert_eq!(canonical.inputs, IteInputs::new(x, y, y.not()));
        assert!(!canonical.complemented);
    }

    #[test]
    fn canonicalize_regularizes_complemented_condition() {
        let (manager, x, y, z) = sample_manager();

        let canonical = canonicalize_ite_inputs(&manager, IteInputs::new(x.not(), y, z)).unwrap();

        assert_eq!(canonical.inputs, IteInputs::new(x, z, y));
        assert!(!canonical.complemented);
    }

    #[test]
    fn canonicalize_regularizes_complemented_then_branch() {
        let (manager, x, y, z) = sample_manager();

        let canonical = canonicalize_ite_inputs(&manager, IteInputs::new(x, y.not(), z)).unwrap();

        assert_eq!(canonical.inputs, IteInputs::new(x, y, z.not()));
        assert!(canonical.complemented);
    }

    #[test]
    fn ordering_uses_variable_then_node_identity_then_complement() {
        let (manager, x, y, _) = sample_manager();

        assert!(greater_than_or_equal(&manager, y, x).unwrap());
        assert!(!greater_than_or_equal(&manager, x, y).unwrap());
        assert!(greater_than_or_equal(&manager, x.not(), x).unwrap());
        assert!(!greater_than_or_equal(&manager, x, x.not()).unwrap());
    }
}
