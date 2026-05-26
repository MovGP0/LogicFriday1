//! Native replacement-plan model for `sis/map/replace.c`.
//!
//! Legacy SIS `replace.c` mutates a full `network_t`: mapped covers are turned
//! into gate nodes, those gates are ordered before their users, and the original
//! network nodes are substituted or deleted. The native Rust mapper does not yet
//! own the complete SIS network mutation layer, so this module records the same
//! replacement intent against `VirtualMappedNetwork` nodes and validates the
//! order that a later mutator must follow.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use super::two_level::PortDependency;
use super::virtual_net::{
    GateKind, NodeId, NodeKind, SourceRef, VirtualMappedNetwork, VirtualNetworkError,
};

pub const REQUIRED_FULL_NETWORK_MUTATION_BEADS: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "native node creation, duplication, substitution, and deletion",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.304",
        source_file: "LogicSynthesis/sis/network/netmake.c",
        note: "native network construction and fanin/fanout rewiring",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "native network mutation utilities used by replacement",
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappedGateReplacement {
    pub output: NodeId,
    pub gate: GateKind,
    pub fanins: Vec<SourceRef>,
}

impl MappedGateReplacement {
    pub fn new(output: NodeId, gate: GateKind, fanins: Vec<SourceRef>) -> Self {
        Self {
            output,
            gate,
            fanins,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeReplacement {
    pub original: NodeId,
    pub gates: Vec<MappedGateReplacement>,
}

impl NodeReplacement {
    pub fn new(original: NodeId, gates: Vec<MappedGateReplacement>) -> Self {
        Self { original, gates }
    }

    pub fn root_gate(&self) -> Option<&MappedGateReplacement> {
        self.gates.last()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReplacementPlan {
    replacements: Vec<NodeReplacement>,
}

impl ReplacementPlan {
    pub fn new(replacements: Vec<NodeReplacement>) -> Self {
        Self { replacements }
    }

    pub fn replacements(&self) -> &[NodeReplacement] {
        &self.replacements
    }

    pub fn push(&mut self, replacement: NodeReplacement) {
        self.replacements.push(replacement);
    }

    pub fn validate(&self, network: &VirtualMappedNetwork) -> Result<(), ReplacementPlanError> {
        let mut seen_originals = BTreeSet::new();
        let mut produced_gates = BTreeSet::new();
        let mut replacement_index_by_original = BTreeMap::new();

        for (index, replacement) in self.replacements.iter().enumerate() {
            if replacement_index_by_original
                .insert(replacement.original, index)
                .is_some()
            {
                return Err(ReplacementPlanError::DuplicateOriginalNode(
                    replacement.original,
                ));
            }
        }

        for (index, replacement) in self.replacements.iter().enumerate() {
            let original_node =
                network
                    .node(replacement.original)
                    .ok_or(ReplacementPlanError::VirtualNetwork(
                        VirtualNetworkError::MissingNode(replacement.original),
                    ))?;

            if original_node.kind != NodeKind::Internal {
                return Err(ReplacementPlanError::CannotReplaceExternalNode(
                    replacement.original,
                ));
            }

            if !seen_originals.insert(replacement.original) {
                return Err(ReplacementPlanError::DuplicateOriginalNode(
                    replacement.original,
                ));
            }

            if replacement.gates.is_empty() {
                return Err(ReplacementPlanError::EmptyCover(replacement.original));
            }

            let mut local_outputs = BTreeSet::new();
            for gate in &replacement.gates {
                if network.node(gate.output).is_none() {
                    return Err(ReplacementPlanError::VirtualNetwork(
                        VirtualNetworkError::MissingNode(gate.output),
                    ));
                }

                if !produced_gates.insert(gate.output) {
                    return Err(ReplacementPlanError::DuplicateGateOutput(gate.output));
                }

                for source in &gate.fanins {
                    self.validate_source(
                        network,
                        replacement.original,
                        *source,
                        &local_outputs,
                        &replacement_index_by_original,
                        index,
                    )?;
                }

                local_outputs.insert(gate.output);
            }

            let root = replacement
                .root_gate()
                .expect("non-empty replacement covers must have a root gate");
            if root.output != replacement.original {
                return Err(ReplacementPlanError::RootDoesNotReplaceOriginal {
                    original: replacement.original,
                    root: root.output,
                });
            }
        }

        Ok(())
    }

    fn validate_source(
        &self,
        network: &VirtualMappedNetwork,
        original: NodeId,
        source: SourceRef,
        local_outputs: &BTreeSet<NodeId>,
        replacement_index_by_original: &BTreeMap<NodeId, usize>,
        replacement_index: usize,
    ) -> Result<(), ReplacementPlanError> {
        let SourceRef::Node(source) = source else {
            return Ok(());
        };

        if network.node(source).is_none() {
            return Err(ReplacementPlanError::VirtualNetwork(
                VirtualNetworkError::MissingNode(source),
            ));
        }

        if local_outputs.contains(&source) {
            return Ok(());
        }

        if source == original {
            return Err(ReplacementPlanError::SelfDependency(original));
        }

        if let Some(source_replacement_index) = replacement_index_by_original.get(&source) {
            if *source_replacement_index >= replacement_index {
                return Err(ReplacementPlanError::ReplacementOrder {
                    user: original,
                    dependency: source,
                });
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplacementPlanError {
    VirtualNetwork(VirtualNetworkError),
    CannotReplaceExternalNode(NodeId),
    DuplicateOriginalNode(NodeId),
    DuplicateGateOutput(NodeId),
    EmptyCover(NodeId),
    RootDoesNotReplaceOriginal {
        original: NodeId,
        root: NodeId,
    },
    SelfDependency(NodeId),
    ReplacementOrder {
        user: NodeId,
        dependency: NodeId,
    },
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for ReplacementPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::CannotReplaceExternalNode(node) => {
                write!(f, "cannot replace external virtual node {}", node.index())
            }
            Self::DuplicateOriginalNode(node) => {
                write!(f, "replacement plan repeats original node {}", node.index())
            }
            Self::DuplicateGateOutput(node) => {
                write!(f, "replacement plan repeats gate output {}", node.index())
            }
            Self::EmptyCover(node) => {
                write!(
                    f,
                    "replacement for node {} has no mapped gates",
                    node.index()
                )
            }
            Self::RootDoesNotReplaceOriginal { original, root } => write!(
                f,
                "replacement for node {} must end with root gate output {}, got {}",
                original.index(),
                original.index(),
                root.index()
            ),
            Self::SelfDependency(node) => {
                write!(f, "replacement for node {} depends on itself", node.index())
            }
            Self::ReplacementOrder { user, dependency } => write!(
                f,
                "replacement for node {} must appear after dependency {}",
                user.index(),
                dependency.index()
            ),
            Self::MissingSisPorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} native SIS prerequisite ports",
                dependencies.len()
            ),
        }
    }
}

impl Error for ReplacementPlanError {}

impl From<VirtualNetworkError> for ReplacementPlanError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

pub fn required_full_network_mutation_beads() -> &'static [PortDependency] {
    REQUIRED_FULL_NETWORK_MUTATION_BEADS
}

pub fn full_sis_network_mutation_unavailable() -> Result<(), ReplacementPlanError> {
    Err(ReplacementPlanError::MissingSisPorts {
        operation: "replace mapped covers in a full SIS network",
        dependencies: REQUIRED_FULL_NETWORK_MUTATION_BEADS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network_with_two_internal_nodes() -> (VirtualMappedNetwork, NodeId, NodeId, NodeId) {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let n1 = network.add_gate(
            "n1",
            GateKind::And,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        let n2 = network.add_gate("n2", GateKind::Or, vec![SourceRef::Node(n1)]);
        network
            .add_primary_output("f", SourceRef::Node(n2))
            .unwrap();
        (network, a, n1, n2)
    }

    #[test]
    fn validates_ordered_mapped_cover_replacements() {
        let (network, a, n1, n2) = network_with_two_internal_nodes();
        let plan = ReplacementPlan::new(vec![
            NodeReplacement::new(
                n1,
                vec![MappedGateReplacement::new(
                    n1,
                    GateKind::Nand,
                    vec![SourceRef::Node(a), SourceRef::ConstantOne],
                )],
            ),
            NodeReplacement::new(
                n2,
                vec![
                    MappedGateReplacement::new(
                        a,
                        GateKind::Inverter,
                        vec![SourceRef::ConstantZero],
                    ),
                    MappedGateReplacement::new(n2, GateKind::Or, vec![SourceRef::Node(n1)]),
                ],
            ),
        ]);

        plan.validate(&network).unwrap();
    }

    #[test]
    fn rejects_replacement_that_uses_later_original() {
        let (network, _, n1, n2) = network_with_two_internal_nodes();
        let plan = ReplacementPlan::new(vec![
            NodeReplacement::new(
                n2,
                vec![MappedGateReplacement::new(
                    n2,
                    GateKind::Or,
                    vec![SourceRef::Node(n1)],
                )],
            ),
            NodeReplacement::new(
                n1,
                vec![MappedGateReplacement::new(
                    n1,
                    GateKind::And,
                    vec![SourceRef::ConstantOne],
                )],
            ),
        ]);

        assert_eq!(
            plan.validate(&network),
            Err(ReplacementPlanError::ReplacementOrder {
                user: n2,
                dependency: n1,
            })
        );
    }

    #[test]
    fn rejects_cover_whose_root_does_not_replace_original() {
        let (network, a, n1, _) = network_with_two_internal_nodes();
        let plan = ReplacementPlan::new(vec![NodeReplacement::new(
            n1,
            vec![MappedGateReplacement::new(
                a,
                GateKind::Inverter,
                vec![SourceRef::ConstantOne],
            )],
        )]);

        assert_eq!(
            plan.validate(&network),
            Err(ReplacementPlanError::RootDoesNotReplaceOriginal {
                original: n1,
                root: a,
            })
        );
    }

    #[test]
    fn reports_dependency_error_for_full_network_mutation() {
        assert_eq!(
            full_sis_network_mutation_unavailable(),
            Err(ReplacementPlanError::MissingSisPorts {
                operation: "replace mapped covers in a full SIS network",
                dependencies: REQUIRED_FULL_NETWORK_MUTATION_BEADS,
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("replace.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
