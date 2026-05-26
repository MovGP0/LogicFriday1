//! Native orchestration for SIS factor/factor.c.
//!
//! The C file owns the public factoring entry points and delegates the actual
//! kernel extraction to the rest of the factor package. This Rust port keeps the
//! same split: it manages cached factored forms, invalidation, temporary-network
//! factoring, and quick/good backend selection while exposing Rust-owned data.

#[cfg(not(test))]
use super::alg_ft;

#[cfg(not(test))]
use super::alg_ft::{
    AlgebraicFactorNode, Atom, Cover, Cube, FactorId, Literal, LiteralPhase, VariableId,
    factor_recur,
};

#[cfg(not(test))]
use super::ft_util;

#[cfg(not(test))]
use super::ft_util::{
    CubeLiteral, FactorKind, FactorNetwork, FactorResult, FactorTree, NodeFunction, NodeId,
    factor_free, factor_nt_to_ft, factor_set,
};

#[cfg(test)]
#[path = "alg_ft.rs"]
mod alg_ft;

#[cfg(test)]
#[path = "ft_util.rs"]
mod ft_util;

#[cfg(test)]
use alg_ft::{
    AlgebraicFactorNode, Atom, Cover, Cube, FactorId, Literal, LiteralPhase, VariableId,
    factor_recur,
};

#[cfg(test)]
use ft_util::{
    CubeLiteral, FactorError, FactorKind, FactorNetwork, FactorNode, FactorResult, FactorTree,
    NodeFunction, NodeId, factor_free, factor_nt_to_ft, factor_set,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactorMode {
    Quick,
    Good,
}

pub trait FactorBackend {
    fn prepare_node(&mut self, _network: &mut FactorNetwork, _node: NodeId) -> FactorResult<()> {
        Ok(())
    }

    fn quick_kernel(&mut self, _network: &mut FactorNetwork, node: NodeId) -> FactorResult<NodeId> {
        Ok(node)
    }

    fn best_kernel(&mut self, _network: &mut FactorNetwork, node: NodeId) -> FactorResult<NodeId> {
        Ok(node)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeFactorBackend;

impl FactorBackend for NativeFactorBackend {
    fn quick_kernel(&mut self, network: &mut FactorNetwork, node: NodeId) -> FactorResult<NodeId> {
        self.factor_native(network, node, FactorMode::Quick)
    }

    fn best_kernel(&mut self, network: &mut FactorNetwork, node: NodeId) -> FactorResult<NodeId> {
        self.factor_native(network, node, FactorMode::Good)
    }
}

impl NativeFactorBackend {
    fn factor_native(
        &mut self,
        network: &mut FactorNetwork,
        node: NodeId,
        mode: FactorMode,
    ) -> FactorResult<NodeId> {
        if network.node(node)?.function != NodeFunction::SumOfProducts {
            return Ok(node);
        }

        let mut algebraic = node_to_algebraic(network, node)?;
        let mut generator = |cover: &Cover| select_kernel(cover, mode);
        factor_recur(&mut algebraic, &mut generator).map_err(map_alg_error)?;
        let tree = algebraic_to_tree(&algebraic).map_err(map_alg_error)?;
        factor_set(network.node_mut(node)?, tree);
        Ok(node)
    }
}

pub fn factor(network: &mut FactorNetwork, node: NodeId) -> FactorResult<()> {
    let mut backend = NativeFactorBackend;
    factor_with_backend(network, node, &mut backend)
}

pub fn factor_with_backend<B>(
    network: &mut FactorNetwork,
    node: NodeId,
    backend: &mut B,
) -> FactorResult<()>
where
    B: FactorBackend,
{
    if network.node(node)?.factored().is_some() {
        return Ok(());
    }

    factor_quick_with_backend(network, node, backend)
}

pub fn factor_quick(network: &mut FactorNetwork, node: NodeId) -> FactorResult<()> {
    let mut backend = NativeFactorBackend;
    factor_quick_with_backend(network, node, &mut backend)
}

pub fn factor_quick_with_backend<B>(
    network: &mut FactorNetwork,
    node: NodeId,
    backend: &mut B,
) -> FactorResult<()>
where
    B: FactorBackend,
{
    factor_using_backend(network, node, backend, FactorMode::Quick)
}

pub fn factor_good(network: &mut FactorNetwork, node: NodeId) -> FactorResult<()> {
    let mut backend = NativeFactorBackend;
    factor_good_with_backend(network, node, &mut backend)
}

pub fn factor_good_with_backend<B>(
    network: &mut FactorNetwork,
    node: NodeId,
    backend: &mut B,
) -> FactorResult<()>
where
    B: FactorBackend,
{
    factor_using_backend(network, node, backend, FactorMode::Good)
}

pub fn factor_using_backend<B>(
    network: &mut FactorNetwork,
    node: NodeId,
    backend: &mut B,
    mode: FactorMode,
) -> FactorResult<()>
where
    B: FactorBackend,
{
    factor_free(network.node_mut(node)?);

    let mut working = network.clone();
    backend.prepare_node(&mut working, node)?;
    let root = match mode {
        FactorMode::Quick => backend.quick_kernel(&mut working, node)?,
        FactorMode::Good => backend.best_kernel(&mut working, node)?,
    };

    let tree = if let Some(tree) = working.node(root)?.factored().cloned() {
        tree
    } else {
        factor_nt_to_ft(&working, node, root)?
    };
    factor_set(network.node_mut(node)?, tree);
    Ok(())
}

pub fn factor_tree(network: &mut FactorNetwork, node: NodeId) -> FactorResult<&FactorTree> {
    factor(network, node)?;
    Ok(network
        .node(node)?
        .factored()
        .expect("factor() must install a factor tree on success"))
}

fn node_to_algebraic(network: &FactorNetwork, node: NodeId) -> FactorResult<AlgebraicFactorNode> {
    let node_data = network.node(node)?;
    let mut cubes = Vec::new();

    for (cube_index, cube) in node_data.cubes().iter().enumerate() {
        if cube.len() != node_data.fanins().len() {
            return Err(ft_util::FactorError::InvalidCubeWidth {
                node,
                cube: cube_index,
                expected: node_data.fanins().len(),
                actual: cube.len(),
            });
        }

        let mut literals = Vec::new();
        for (index, literal) in cube.iter().enumerate() {
            let Some(phase) = cube_literal_phase(*literal) else {
                continue;
            };
            literals.push(Literal::variable(VariableId(index), phase));
        }

        cubes.push(Cube::new(literals).map_err(map_alg_error)?);
    }

    Ok(AlgebraicFactorNode::new(
        FactorId(node.0),
        Cover::new(cubes),
    ))
}

fn cube_literal_phase(literal: CubeLiteral) -> Option<LiteralPhase> {
    match literal {
        CubeLiteral::Zero => Some(LiteralPhase::Negative),
        CubeLiteral::One => Some(LiteralPhase::Positive),
        CubeLiteral::DontCare => None,
    }
}

fn select_kernel(cover: &Cover, mode: FactorMode) -> Option<Cover> {
    let candidates = kernel_candidates(cover);
    match mode {
        FactorMode::Quick => candidates.into_iter().next(),
        FactorMode::Good => candidates
            .into_iter()
            .max_by_key(|candidate| kernel_score(cover, candidate)),
    }
}

fn kernel_candidates(cover: &Cover) -> Vec<Cover> {
    let mut candidates = Vec::new();
    for atom in cover.atoms() {
        for phase in [LiteralPhase::Positive, LiteralPhase::Negative] {
            let divisor = Cover::literal(Literal { atom, phase });
            let quotient = cover.quotient(&divisor);
            if is_useful_kernel(&quotient) && !candidates.contains(&quotient) {
                candidates.push(quotient);
            }
        }
    }

    candidates
}

fn is_useful_kernel(cover: &Cover) -> bool {
    cover.cube_count() >= 2 && !cover.is_one()
}

fn kernel_score(cover: &Cover, candidate: &Cover) -> usize {
    let co_kernel = cover.quotient(candidate);
    let candidate_literals = literal_count(candidate);
    let co_kernel_literals = literal_count(&co_kernel);
    let extracted_literals = candidate_literals.saturating_mul(co_kernel.cube_count());
    extracted_literals + co_kernel_literals + candidate.cube_count()
}

fn literal_count(cover: &Cover) -> usize {
    cover
        .cubes()
        .iter()
        .map(|cube| cube.literal_count())
        .sum::<usize>()
}

fn algebraic_to_tree(node: &AlgebraicFactorNode) -> Result<FactorTree, alg_ft::FactorError> {
    cover_to_tree(&node.cover, node)
}

fn cover_to_tree(
    cover: &Cover,
    owner: &AlgebraicFactorNode,
) -> Result<FactorTree, alg_ft::FactorError> {
    if cover.is_zero() {
        return Ok(FactorTree::constant(false));
    }

    let mut terms = Vec::new();
    for cube in cover.cubes() {
        terms.push(cube_to_tree(cube, owner)?);
    }

    Ok(nary_tree(FactorKind::Or, terms))
}

fn cube_to_tree(
    cube: &Cube,
    owner: &AlgebraicFactorNode,
) -> Result<FactorTree, alg_ft::FactorError> {
    if cube.literals().is_empty() {
        return Ok(FactorTree::constant(true));
    }

    let mut children = Vec::new();
    for literal in cube.literals() {
        children.push(literal_to_tree(*literal, owner)?);
    }

    Ok(nary_tree(FactorKind::And, children))
}

fn literal_to_tree(
    literal: Literal,
    owner: &AlgebraicFactorNode,
) -> Result<FactorTree, alg_ft::FactorError> {
    let child = match literal.atom {
        Atom::Variable(variable) => FactorTree::leaf(variable.0),
        Atom::Factor(factor) => {
            let child = owner
                .child(factor)
                .ok_or(alg_ft::FactorError::MissingAssignment(Atom::Factor(factor)))?;
            cover_to_tree(&child.cover, child)?
        }
    };

    Ok(match literal.phase {
        LiteralPhase::Positive => child,
        LiteralPhase::Negative => {
            FactorTree::new(FactorKind::Inverter, -1, 0).with_next_level(child)
        }
    })
}

fn nary_tree(kind: FactorKind, mut children: Vec<FactorTree>) -> FactorTree {
    if children.len() == 1 {
        return children.remove(0);
    }

    let mut iter = children.into_iter().rev();
    let mut tree = iter
        .next()
        .expect("nary_tree requires at least one child for non-constant factors");
    for mut child in iter {
        child.same_level = Some(Box::new(tree));
        tree = child;
    }

    FactorTree::new(kind, -1, 0).with_next_level(tree)
}

fn map_alg_error(_error: alg_ft::FactorError) -> ft_util::FactorError {
    ft_util::FactorError::MissingNativeIntegration {
        operation: "algebraic factoring",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        prepared: usize,
        quick: usize,
        best: usize,
        root: Option<NodeId>,
        fail_prepare: bool,
    }

    impl FactorBackend for RecordingBackend {
        fn prepare_node(
            &mut self,
            _network: &mut FactorNetwork,
            _node: NodeId,
        ) -> FactorResult<()> {
            self.prepared += 1;
            if self.fail_prepare {
                Err(FactorError::MissingNativeIntegration {
                    operation: "prepare",
                })
            } else {
                Ok(())
            }
        }

        fn quick_kernel(
            &mut self,
            _network: &mut FactorNetwork,
            node: NodeId,
        ) -> FactorResult<NodeId> {
            self.quick += 1;
            Ok(self.root.unwrap_or(node))
        }

        fn best_kernel(
            &mut self,
            _network: &mut FactorNetwork,
            node: NodeId,
        ) -> FactorResult<NodeId> {
            self.best += 1;
            Ok(self.root.unwrap_or(node))
        }
    }

    fn sample_network() -> (FactorNetwork, NodeId, NodeId, NodeId) {
        let mut network = FactorNetwork::new();
        let a = network.add_node(FactorNode::input());
        let b = network.add_node(FactorNode::input());
        let f = network.add_node(FactorNode::sum_of_products(
            vec![a, b],
            vec![
                vec![CubeLiteral::One, CubeLiteral::DontCare],
                vec![CubeLiteral::DontCare, CubeLiteral::Zero],
            ],
        ));
        (network, a, b, f)
    }

    fn child(tree: &FactorTree) -> &FactorTree {
        tree.next_level.as_deref().unwrap()
    }

    fn sibling(tree: &FactorTree) -> &FactorTree {
        tree.same_level.as_deref().unwrap()
    }

    #[test]
    fn factor_is_noop_when_cached_factor_exists() {
        let (mut network, _a, _b, f) = sample_network();
        factor_set(network.node_mut(f).unwrap(), FactorTree::leaf(0));
        let mut backend = RecordingBackend::default();

        factor_with_backend(&mut network, f, &mut backend).unwrap();

        assert_eq!(backend.prepared, 0);
        assert_eq!(backend.quick, 0);
        assert_eq!(
            network.node(f).unwrap().factored(),
            Some(&FactorTree::leaf(0))
        );
    }

    #[test]
    fn quick_factor_recomputes_and_dispatches_quick_kernel() {
        let (mut network, _a, _b, f) = sample_network();
        factor_set(network.node_mut(f).unwrap(), FactorTree::constant(true));
        let mut backend = RecordingBackend::default();

        factor_quick_with_backend(&mut network, f, &mut backend).unwrap();

        assert_eq!(backend.prepared, 1);
        assert_eq!(backend.quick, 1);
        assert_eq!(backend.best, 0);
        assert_ne!(
            network.node(f).unwrap().factored(),
            Some(&FactorTree::constant(true))
        );
    }

    #[test]
    fn native_quick_factor_extracts_shared_literal_kernel() {
        let mut network = FactorNetwork::new();
        let a = network.add_node(FactorNode::input());
        let b = network.add_node(FactorNode::input());
        let c = network.add_node(FactorNode::input());
        let f = network.add_node(FactorNode::sum_of_products(
            vec![a, b, c],
            vec![
                vec![CubeLiteral::One, CubeLiteral::One, CubeLiteral::DontCare],
                vec![CubeLiteral::One, CubeLiteral::DontCare, CubeLiteral::One],
            ],
        ));

        factor_quick(&mut network, f).unwrap();

        let tree = network.node(f).unwrap().factored().unwrap();
        assert_eq!(tree.kind, FactorKind::And);
        assert_eq!(child(tree).kind, FactorKind::Leaf);
        assert_eq!(child(tree).index, 0);
        assert_eq!(sibling(child(tree)).kind, FactorKind::Or);
    }

    #[test]
    fn native_good_factor_extracts_product_of_two_sums() {
        let mut network = FactorNetwork::new();
        let a = network.add_node(FactorNode::input());
        let b = network.add_node(FactorNode::input());
        let c = network.add_node(FactorNode::input());
        let d = network.add_node(FactorNode::input());
        let f = network.add_node(FactorNode::sum_of_products(
            vec![a, b, c, d],
            vec![
                vec![
                    CubeLiteral::One,
                    CubeLiteral::DontCare,
                    CubeLiteral::One,
                    CubeLiteral::DontCare,
                ],
                vec![
                    CubeLiteral::One,
                    CubeLiteral::DontCare,
                    CubeLiteral::DontCare,
                    CubeLiteral::One,
                ],
                vec![
                    CubeLiteral::DontCare,
                    CubeLiteral::One,
                    CubeLiteral::One,
                    CubeLiteral::DontCare,
                ],
                vec![
                    CubeLiteral::DontCare,
                    CubeLiteral::One,
                    CubeLiteral::DontCare,
                    CubeLiteral::One,
                ],
            ],
        ));

        factor_good(&mut network, f).unwrap();

        let tree = network.node(f).unwrap().factored().unwrap();
        assert_eq!(tree.kind, FactorKind::And);
        assert_eq!(child(tree).kind, FactorKind::Or);
        assert_eq!(sibling(child(tree)).kind, FactorKind::Or);
    }

    #[test]
    fn good_factor_dispatches_best_kernel() {
        let (mut network, _a, _b, f) = sample_network();
        let mut backend = RecordingBackend::default();

        factor_good_with_backend(&mut network, f, &mut backend).unwrap();

        assert_eq!(backend.prepared, 1);
        assert_eq!(backend.quick, 0);
        assert_eq!(backend.best, 1);
        assert!(network.node(f).unwrap().factored().is_some());
    }

    #[test]
    fn backend_can_choose_a_temporary_kernel_root() {
        let (mut network, a, _b, f) = sample_network();
        let mut backend = RecordingBackend {
            root: Some(a),
            ..RecordingBackend::default()
        };

        factor_quick_with_backend(&mut network, f, &mut backend).unwrap();

        assert_eq!(
            network.node(f).unwrap().factored(),
            Some(&FactorTree::leaf(0))
        );
    }

    #[test]
    fn failed_prepare_clears_existing_factor() {
        let (mut network, _a, _b, f) = sample_network();
        factor_set(network.node_mut(f).unwrap(), FactorTree::leaf(0));
        let mut backend = RecordingBackend {
            fail_prepare: true,
            ..RecordingBackend::default()
        };

        let error = factor_good_with_backend(&mut network, f, &mut backend).unwrap_err();

        assert_eq!(
            error,
            FactorError::MissingNativeIntegration {
                operation: "prepare"
            }
        );
        assert!(network.node(f).unwrap().factored().is_none());
    }

    #[test]
    fn invalid_cover_shape_is_reported() {
        let mut network = FactorNetwork::new();
        let a = network.add_node(FactorNode::input());
        let b = network.add_node(FactorNode::input());
        let f = network.add_node(FactorNode::sum_of_products(
            vec![a, b],
            vec![vec![CubeLiteral::One]],
        ));

        assert!(matches!(
            factor_quick(&mut network, f),
            Err(FactorError::InvalidCubeWidth { .. })
        ));
    }

    #[test]
    fn factor_tree_returns_installed_tree() {
        let (mut network, _a, _b, f) = sample_network();

        let tree = factor_tree(&mut network, f).unwrap();

        assert_eq!(tree.kind, FactorKind::Or);
    }

    #[test]
    fn no_legacy_c_abi_or_dependency_metadata_tokens_are_present() {
        let source = include_str!("factor.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("Logic", "Friday", "1", "-", "8", "j", "8")));
    }
}
