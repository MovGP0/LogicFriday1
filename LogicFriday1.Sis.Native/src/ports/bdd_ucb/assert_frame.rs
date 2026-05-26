//! Native Rust safe-frame validation for the SIS UCB BDD package.
//!
//! The legacy routine is a debug assertion pass over the manager's internal
//! safe-frame stack. Each safe node can protect either a direct slot or the
//! current value of a linked argument. This port keeps that behavior explicit
//! and returns typed errors instead of aborting through C assertion macros.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddNodeId(usize);

impl BddNodeId {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn value(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddPointer {
    node: BddNodeId,
    complemented: bool,
}

impl BddPointer {
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

    pub const fn regularized(self) -> Self {
        Self::regular(self.node)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddNode {
    halfspace: usize,
    broken_heart: bool,
}

impl BddNode {
    pub const fn new(halfspace: usize) -> Self {
        Self {
            halfspace,
            broken_heart: false,
        }
    }

    pub const fn broken_heart(halfspace: usize) -> Self {
        Self {
            halfspace,
            broken_heart: true,
        }
    }

    pub const fn halfspace(self) -> usize {
        self.halfspace
    }

    pub const fn is_broken_heart(self) -> bool {
        self.broken_heart
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafeNodeSource {
    LinkedArgument,
    NodeSlot,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddSafeNode {
    node: Option<BddPointer>,
    linked_argument: Option<Option<BddPointer>>,
}

impl BddSafeNode {
    pub const fn declared(node: Option<BddPointer>) -> Self {
        Self {
            node,
            linked_argument: None,
        }
    }

    pub const fn linked_argument(argument: Option<BddPointer>) -> Self {
        Self {
            node: None,
            linked_argument: Some(argument),
        }
    }

    pub const fn with_node(mut self, node: Option<BddPointer>) -> Self {
        self.node = node;
        self
    }

    pub const fn node(&self) -> Option<BddPointer> {
        self.node
    }

    pub const fn linked_argument_value(&self) -> Option<Option<BddPointer>> {
        self.linked_argument
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddSafeFrame {
    nodes: Vec<BddSafeNode>,
}

impl BddSafeFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_nodes(nodes: impl IntoIterator<Item = BddSafeNode>) -> Self {
        Self {
            nodes: nodes.into_iter().collect(),
        }
    }

    pub fn push_node(&mut self, node: BddSafeNode) {
        self.nodes.push(node);
    }

    pub fn nodes(&self) -> &[BddSafeNode] {
        &self.nodes
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameAssertionReport {
    pub frames_checked: usize,
    pub safe_nodes_checked: usize,
    pub pointers_checked: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddFrameAssertManager {
    active_halfspace: usize,
    check_halfspace: bool,
    nodes: Vec<Option<BddNode>>,
    frames: Vec<BddSafeFrame>,
}

impl BddFrameAssertManager {
    pub fn new(active_halfspace: usize) -> Self {
        Self {
            active_halfspace,
            check_halfspace: false,
            nodes: Vec::new(),
            frames: Vec::new(),
        }
    }

    pub fn with_halfspace_check(mut self, check_halfspace: bool) -> Self {
        self.check_halfspace = check_halfspace;
        self
    }

    pub fn active_halfspace(&self) -> usize {
        self.active_halfspace
    }

    pub fn set_active_halfspace(&mut self, active_halfspace: usize) {
        self.active_halfspace = active_halfspace;
    }

    pub fn check_halfspace(&self) -> bool {
        self.check_halfspace
    }

    pub fn add_node(&mut self, node: BddNode) -> BddNodeId {
        let id = BddNodeId::new(self.nodes.len());
        self.nodes.push(Some(node));
        id
    }

    pub fn remove_node(&mut self, id: BddNodeId) -> Result<(), BddFrameAssertionError> {
        let slot = self
            .nodes
            .get_mut(id.value())
            .ok_or(BddFrameAssertionError::MissingNode {
                node: id,
                source: SafeNodeSource::NodeSlot,
            })?;
        *slot = None;
        Ok(())
    }

    pub fn node(&self, id: BddNodeId) -> Option<BddNode> {
        self.nodes.get(id.value()).copied().flatten()
    }

    pub fn push_frame(&mut self, frame: BddSafeFrame) {
        self.frames.push(frame);
    }

    pub fn pop_frame(&mut self) -> Option<BddSafeFrame> {
        self.frames.pop()
    }

    pub fn frames(&self) -> &[BddSafeFrame] {
        &self.frames
    }

    pub fn assert_frames_correct(&self) -> Result<FrameAssertionReport, BddFrameAssertionError> {
        bdd_assert_frames_correct(self)
    }
}

pub fn bdd_assert_frames_correct(
    manager: &BddFrameAssertManager,
) -> Result<FrameAssertionReport, BddFrameAssertionError> {
    let mut report = FrameAssertionReport::default();

    for frame in manager.frames.iter().rev() {
        report.frames_checked += 1;

        for safe_node in frame.nodes() {
            report.safe_nodes_checked += 1;

            if let Some(argument) = safe_node.linked_argument_value() {
                assert_pointer_okay(manager, argument, SafeNodeSource::LinkedArgument)?;
                if argument.is_some() {
                    report.pointers_checked += 1;
                }
            }

            assert_pointer_okay(manager, safe_node.node(), SafeNodeSource::NodeSlot)?;
            if safe_node.node().is_some() {
                report.pointers_checked += 1;
            }
        }
    }

    Ok(report)
}

fn assert_pointer_okay(
    manager: &BddFrameAssertManager,
    pointer: Option<BddPointer>,
    source: SafeNodeSource,
) -> Result<(), BddFrameAssertionError> {
    let Some(pointer) = pointer else {
        return Ok(());
    };

    let regular = pointer.regularized().node();
    let node = manager
        .node(regular)
        .ok_or(BddFrameAssertionError::MissingNode {
            node: regular,
            source,
        })?;

    if manager.check_halfspace && node.halfspace() != manager.active_halfspace {
        return Err(BddFrameAssertionError::HalfspaceMismatch {
            node: regular,
            expected: manager.active_halfspace,
            actual: node.halfspace(),
            source,
        });
    }

    if node.is_broken_heart() {
        return Err(BddFrameAssertionError::BrokenHeartNode {
            node: regular,
            source,
        });
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddFrameAssertionError {
    MissingNode {
        node: BddNodeId,
        source: SafeNodeSource,
    },
    HalfspaceMismatch {
        node: BddNodeId,
        expected: usize,
        actual: usize,
        source: SafeNodeSource,
    },
    BrokenHeartNode {
        node: BddNodeId,
        source: SafeNodeSource,
    },
}

impl fmt::Display for BddFrameAssertionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode { node, source } => {
                write!(
                    formatter,
                    "BDD safe-frame {:?} pointer references missing node {}",
                    source,
                    node.value()
                )
            }
            Self::HalfspaceMismatch {
                node,
                expected,
                actual,
                source,
            } => {
                write!(
                    formatter,
                    "BDD safe-frame {:?} pointer references node {} in halfspace {}, expected {}",
                    source,
                    node.value(),
                    actual,
                    expected
                )
            }
            Self::BrokenHeartNode { node, source } => {
                write!(
                    formatter,
                    "BDD safe-frame {:?} pointer references broken-heart node {}",
                    source,
                    node.value()
                )
            }
        }
    }
}

impl Error for BddFrameAssertionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn ptr(node: BddNodeId) -> BddPointer {
        BddPointer::regular(node)
    }

    fn cptr(node: BddNodeId) -> BddPointer {
        BddPointer::complemented(node)
    }

    #[test]
    fn empty_stack_is_correct() {
        let manager = BddFrameAssertManager::new(0);

        let report = manager.assert_frames_correct().unwrap();

        assert_eq!(report, FrameAssertionReport::default());
    }

    #[test]
    fn nil_slots_and_nil_linked_arguments_are_accepted() {
        let mut manager = BddFrameAssertManager::new(0);
        manager.push_frame(BddSafeFrame::with_nodes([
            BddSafeNode::declared(None),
            BddSafeNode::linked_argument(None),
        ]));

        let report = bdd_assert_frames_correct(&manager).unwrap();

        assert_eq!(report.frames_checked, 1);
        assert_eq!(report.safe_nodes_checked, 2);
        assert_eq!(report.pointers_checked, 0);
    }

    #[test]
    fn validates_linked_argument_before_node_slot() {
        let mut manager = BddFrameAssertManager::new(0);
        let good = manager.add_node(BddNode::new(0));
        let bad = manager.add_node(BddNode::broken_heart(0));
        manager.push_frame(BddSafeFrame::with_nodes([BddSafeNode::linked_argument(
            Some(ptr(bad)),
        )
        .with_node(Some(ptr(good)))]));

        let error = manager.assert_frames_correct().unwrap_err();

        assert_eq!(
            error,
            BddFrameAssertionError::BrokenHeartNode {
                node: bad,
                source: SafeNodeSource::LinkedArgument,
            }
        );
    }

    #[test]
    fn complemented_pointers_are_regularized_for_validation() {
        let mut manager = BddFrameAssertManager::new(0);
        let node = manager.add_node(BddNode::new(0));
        manager.push_frame(BddSafeFrame::with_nodes([BddSafeNode::declared(Some(
            cptr(node),
        ))]));

        let report = manager.assert_frames_correct().unwrap();

        assert_eq!(report.pointers_checked, 1);
    }

    #[test]
    fn missing_nodes_are_reported_with_source() {
        let mut manager = BddFrameAssertManager::new(0);
        let missing = BddNodeId::new(4);
        manager.push_frame(BddSafeFrame::with_nodes([BddSafeNode::declared(Some(
            ptr(missing),
        ))]));

        let error = manager.assert_frames_correct().unwrap_err();

        assert_eq!(
            error,
            BddFrameAssertionError::MissingNode {
                node: missing,
                source: SafeNodeSource::NodeSlot,
            }
        );
    }

    #[test]
    fn halfspace_check_is_optional_like_legacy_debug_gc() {
        let mut manager = BddFrameAssertManager::new(0);
        let node = manager.add_node(BddNode::new(1));
        manager.push_frame(BddSafeFrame::with_nodes([BddSafeNode::declared(Some(
            ptr(node),
        ))]));

        assert!(manager.assert_frames_correct().is_ok());

        let manager = manager.with_halfspace_check(true);
        assert_eq!(
            manager.assert_frames_correct(),
            Err(BddFrameAssertionError::HalfspaceMismatch {
                node,
                expected: 0,
                actual: 1,
                source: SafeNodeSource::NodeSlot,
            })
        );
    }

    #[test]
    fn pop_frame_restores_previous_stack_top() {
        let mut manager = BddFrameAssertManager::new(0);
        let first = manager.add_node(BddNode::new(0));
        let second = manager.add_node(BddNode::new(0));
        manager.push_frame(BddSafeFrame::with_nodes([BddSafeNode::declared(Some(
            ptr(first),
        ))]));
        manager.push_frame(BddSafeFrame::with_nodes([BddSafeNode::declared(Some(
            ptr(second),
        ))]));

        let popped = manager.pop_frame().unwrap();
        let report = manager.assert_frames_correct().unwrap();

        assert_eq!(popped.nodes()[0].node(), Some(ptr(second)));
        assert_eq!(report.frames_checked, 1);
        assert_eq!(report.pointers_checked, 1);
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present() {
        let source = include_str!("assert_frame.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
