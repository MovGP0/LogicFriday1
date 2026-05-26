//! Native orchestration for SIS factor/factor.c.
//!
//! The C file owns the public factoring entry points and delegates the actual
//! kernel extraction to the rest of the factor package. This Rust port keeps the
//! same split: it manages cached factored forms, invalidation, temporary-network
//! factoring, and quick/good backend selection while exposing Rust-owned data.

#[cfg(not(test))]
use super::ft_util::{
    FactorNetwork, FactorResult, FactorTree, NodeId, factor_free, factor_nt_to_ft, factor_set,
};

#[cfg(test)]
#[path = "ft_util.rs"]
mod ft_util;

#[cfg(test)]
use ft_util::{
    CubeLiteral, FactorError, FactorKind, FactorNetwork, FactorNode, FactorResult, FactorTree,
    NodeId, factor_free, factor_nt_to_ft, factor_set,
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

impl FactorBackend for NativeFactorBackend {}

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

    let tree = factor_nt_to_ft(&working, node, root)?;
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
        assert!(!source.contains(concat!("be", "ad", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1", "-", "8", "j", "8")));
    }
}
