//! Native Rust decision model for `sis/speed/speed_or.c`.
//!
//! The C routine mutates SIS `network_t` and `node_t` objects directly. Those
//! APIs are not native Rust yet, so this module ports the deterministic
//! decomposition decisions into an inspectable plan and keeps the network-bound
//! entry point as an explicit blocked operation.

use std::error::Error;
use std::fmt;

pub const REQUIRED_PORT_BEADS: &[&str] = &[
    "LogicFriday1-8j8.2.6.473", // speed/speed_and.c: speed_and_decomp
    "LogicFriday1-8j8.2.6.480", // speed/speed_util.c: speed_dec_node_cube
    "LogicFriday1-8j8.2.6.305", // network/network_util.c: network_add_node, network_delete_node
    "LogicFriday1-8j8.2.6.309", // node/collapse.c: node_collapse
    "LogicFriday1-8j8.2.6.313", // node/fan.c: fanin/fanout traversal
    "LogicFriday1-8j8.2.6.318", // node/node.c: node_and, node_literal, node_constant, node_num_cube
    "LogicFriday1-8j8.2.6.321", // node/nodemisc.c: node_replace
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpeedOrOptions {
    pub add_inv: bool,
}

impl SpeedOrOptions {
    pub fn new(add_inv: bool) -> Self {
        Self { add_inv }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CubeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CombinerInput {
    pub cube: CubeId,
    pub phase: LiteralPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndDecompositionTarget {
    OriginalNode,
    Cube(CubeId),
    OrCombiner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndDecompositionRequest {
    pub target: AndDecompositionTarget,
    pub invert_output: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedOrBranch {
    MultiCube,
    SingleCubeOrConstant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeedOrPlan {
    pub branch: SpeedOrBranch,
    pub decoded_cubes: Vec<CubeId>,
    pub combiner_inputs: Vec<CombinerInput>,
    pub and_decompositions: Vec<AndDecompositionRequest>,
    pub collapse_candidates: Vec<CubeId>,
}

impl SpeedOrPlan {
    pub fn uses_or_combiner(&self) -> bool {
        self.branch == SpeedOrBranch::MultiCube
    }
}

pub fn plan_speed_and_or_decomposition(
    cube_fanin_counts_after_decomp: &[usize],
    options: SpeedOrOptions,
) -> SpeedOrPlan {
    if cube_fanin_counts_after_decomp.len() <= 1 {
        return SpeedOrPlan {
            branch: SpeedOrBranch::SingleCubeOrConstant,
            decoded_cubes: Vec::new(),
            combiner_inputs: Vec::new(),
            and_decompositions: vec![AndDecompositionRequest {
                target: AndDecompositionTarget::OriginalNode,
                invert_output: false,
            }],
            collapse_candidates: Vec::new(),
        };
    }

    let decoded_cubes: Vec<_> = (0..cube_fanin_counts_after_decomp.len())
        .map(CubeId)
        .collect();
    let combiner_inputs = decoded_cubes
        .iter()
        .map(|cube| CombinerInput {
            cube: *cube,
            phase: LiteralPhase::Negative,
        })
        .collect();

    let mut and_decompositions: Vec<_> = decoded_cubes
        .iter()
        .map(|cube| AndDecompositionRequest {
            target: AndDecompositionTarget::Cube(*cube),
            invert_output: false,
        })
        .collect();
    and_decompositions.push(AndDecompositionRequest {
        target: AndDecompositionTarget::OrCombiner,
        invert_output: true,
    });

    let collapse_candidates = if options.add_inv {
        Vec::new()
    } else {
        cube_fanin_counts_after_decomp
            .iter()
            .enumerate()
            .filter_map(|(index, fanin_count)| (*fanin_count <= 1).then_some(CubeId(index)))
            .collect()
    };

    SpeedOrPlan {
        branch: SpeedOrBranch::MultiCube,
        decoded_cubes,
        combiner_inputs,
        and_decompositions,
        collapse_candidates,
    }
}

pub fn decompose_node_in_network<Network, Node>(
    _network: &mut Network,
    _node: &mut Node,
    _options: SpeedOrOptions,
) -> Result<(), SpeedOrError> {
    Err(SpeedOrError::MissingNativePorts {
        operation: "speed_and_or_decomp",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedOrError {
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [&'static str],
    },
}

impl fmt::Display for SpeedOrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires native Rust ports for SIS speed/node/network APIs: {}",
                dependencies.join(", ")
            ),
        }
    }
}

impl Error for SpeedOrError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_cube_plan_decodes_each_cube_and_decomposes_combiner_with_inversion() {
        let plan = plan_speed_and_or_decomposition(&[2, 3, 4], SpeedOrOptions::new(true));

        assert_eq!(plan.branch, SpeedOrBranch::MultiCube);
        assert!(plan.uses_or_combiner());
        assert_eq!(plan.decoded_cubes, vec![CubeId(0), CubeId(1), CubeId(2)]);
        assert_eq!(
            plan.combiner_inputs,
            vec![
                CombinerInput {
                    cube: CubeId(0),
                    phase: LiteralPhase::Negative,
                },
                CombinerInput {
                    cube: CubeId(1),
                    phase: LiteralPhase::Negative,
                },
                CombinerInput {
                    cube: CubeId(2),
                    phase: LiteralPhase::Negative,
                },
            ]
        );
        assert_eq!(
            plan.and_decompositions,
            vec![
                AndDecompositionRequest {
                    target: AndDecompositionTarget::Cube(CubeId(0)),
                    invert_output: false,
                },
                AndDecompositionRequest {
                    target: AndDecompositionTarget::Cube(CubeId(1)),
                    invert_output: false,
                },
                AndDecompositionRequest {
                    target: AndDecompositionTarget::Cube(CubeId(2)),
                    invert_output: false,
                },
                AndDecompositionRequest {
                    target: AndDecompositionTarget::OrCombiner,
                    invert_output: true,
                },
            ]
        );
    }

    #[test]
    fn single_cube_or_constant_plan_decomposes_original_node_without_inversion() {
        for cube_fanin_counts in [&[][..], &[3][..]] {
            let plan =
                plan_speed_and_or_decomposition(cube_fanin_counts, SpeedOrOptions::new(false));

            assert_eq!(plan.branch, SpeedOrBranch::SingleCubeOrConstant);
            assert!(!plan.uses_or_combiner());
            assert_eq!(plan.decoded_cubes, Vec::<CubeId>::new());
            assert_eq!(
                plan.and_decompositions,
                vec![AndDecompositionRequest {
                    target: AndDecompositionTarget::OriginalNode,
                    invert_output: false,
                }]
            );
        }
    }

    #[test]
    fn add_inv_false_marks_single_fanin_cubes_for_collapse() {
        let plan = plan_speed_and_or_decomposition(&[0, 1, 2, 1], SpeedOrOptions::new(false));

        assert_eq!(
            plan.collapse_candidates,
            vec![CubeId(0), CubeId(1), CubeId(3)]
        );
    }

    #[test]
    fn add_inv_true_preserves_cube_nodes_even_when_they_have_one_fanin() {
        let plan = plan_speed_and_or_decomposition(&[1, 1, 2], SpeedOrOptions::new(true));

        assert!(plan.collapse_candidates.is_empty());
    }

    #[test]
    fn network_bound_entry_point_reports_blocking_ports() {
        let mut network = ();
        let mut node = ();

        assert_eq!(
            decompose_node_in_network(&mut network, &mut node, SpeedOrOptions::new(false)),
            Err(SpeedOrError::MissingNativePorts {
                operation: "speed_and_or_decomp",
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
    }
}
