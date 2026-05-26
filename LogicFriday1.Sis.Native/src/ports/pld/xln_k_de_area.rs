//! Native Rust model for `LogicSynthesis/sis/pld/xln_k_de_area.c`.
//!
//! The C file is an area-oriented driver around Roth-Karp decomposition: walk
//! internal nodes, try every non-trivial fanin partition when the fanin count is
//! bounded, choose the decomposition with the lowest mapped internal-node cost,
//! optionally recurse on replacements, and finally fall back to AO mapping for
//! still-infeasible nodes. Direct SIS `network_t`/`node_t` replacement remains an
//! explicit missing-dependency error.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AreaNodeKind {
    PrimaryInput,
    Internal,
    PrimaryOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AreaNode {
    pub name: String,
    pub kind: AreaNodeKind,
    pub fanins: Vec<String>,
}

impl AreaNode {
    pub fn new(name: impl Into<String>, kind: AreaNodeKind, fanins: Vec<String>) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins,
        }
    }

    pub fn internal(name: impl Into<String>, fanins: &[&str]) -> Self {
        Self::new(
            name,
            AreaNodeKind::Internal,
            fanins.iter().map(|fanin| (*fanin).to_owned()).collect(),
        )
    }

    pub fn fanin_count(&self) -> usize {
        self.fanins.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KDecompAreaOptions {
    pub support: usize,
    pub max_fanins_k_decomp: usize,
    pub recursive: bool,
}

impl KDecompAreaOptions {
    pub const fn new(support: usize, max_fanins_k_decomp: usize, recursive: bool) -> Self {
        Self {
            support,
            max_fanins_k_decomp,
            recursive,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaninPartition {
    pub combination: usize,
    pub y_fanins: Vec<String>,
    pub z_fanins: Vec<String>,
    pub lambda_indices: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateDecomposition {
    pub internal_cost_after_root_ao_map: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedDecomposition {
    pub partition: FaninPartition,
    pub internal_cost_after_root_ao_map: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeAction {
    SkipNonInternal {
        node: String,
    },
    AlreadyFeasible {
        node: String,
        fanins: usize,
    },
    TooManyFaninsForExhaustiveSearch {
        node: String,
        fanins: usize,
        max_fanins: usize,
    },
    ReplaceWithBestDecomposition {
        node: String,
        selected: SelectedDecomposition,
    },
    AoMapFallback {
        node: String,
        fanins: usize,
        support: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XlnKDeAreaError {
    InvalidSupport { support: usize },
    TooManyFaninsForBitMask { fanins: usize },
    PartitionArityMismatch { fanins: usize, encoded_bits: usize },
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for XlnKDeAreaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSupport { support } => {
                write!(f, "K-decomposition support must be positive, got {support}")
            }
            Self::TooManyFaninsForBitMask { fanins } => write!(
                f,
                "cannot enumerate 1 << {fanins} fanin partitions in usize bit-mask space"
            ),
            Self::PartitionArityMismatch {
                fanins,
                encoded_bits,
            } => write!(
                f,
                "partition encoding has {encoded_bits} bits for {fanins} fanins"
            ),
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS C-file dependencies"
            ),
        }
    }
}

impl Error for XlnKDeAreaError {}

pub fn xln_exhaustive_k_decomp_network_blocked<Network>(
    _network: &mut Network,
    _options: KDecompAreaOptions,
) -> Result<(), XlnKDeAreaError> {
    missing_native_ports("xln_exhaustive_k_decomp_network")
}

pub fn xln_exhaustive_k_decomp_node_blocked<Node>(
    _node: &Node,
    _support: usize,
    _max_fanins: usize,
) -> Result<(), XlnKDeAreaError> {
    missing_native_ports("xln_exhaustive_k_decomp_node")
}

fn missing_native_ports(operation: &'static str) -> Result<(), XlnKDeAreaError> {
    Err(XlnKDeAreaError::MissingNativePorts { operation })
}

pub fn generate_fanin_combination(
    fanins: &[String],
    combination: usize,
) -> Result<FaninPartition, XlnKDeAreaError> {
    let bits = binary_encoding(combination, fanins.len())?;
    if bits.len() != fanins.len() {
        return Err(XlnKDeAreaError::PartitionArityMismatch {
            fanins: fanins.len(),
            encoded_bits: bits.len(),
        });
    }

    let mut y_fanins = Vec::new();
    let mut z_fanins = Vec::new();
    let mut lambda_indices = Vec::new();
    for (index, (fanin, bit)) in fanins.iter().zip(bits).enumerate() {
        if bit {
            z_fanins.push(fanin.clone());
        } else {
            y_fanins.push(fanin.clone());
            lambda_indices.push(index);
        }
    }

    Ok(FaninPartition {
        combination,
        y_fanins,
        z_fanins,
        lambda_indices,
    })
}

pub fn valid_exhaustive_partitions(
    node: &AreaNode,
    support: usize,
    max_fanins: usize,
) -> Result<Vec<FaninPartition>, XlnKDeAreaError> {
    validate_support(support)?;
    let num_fanin = node.fanin_count();
    if node.kind != AreaNodeKind::Internal || num_fanin <= support || num_fanin > max_fanins {
        return Ok(Vec::new());
    }
    let num_comb = checked_combination_count(num_fanin)?;
    let mut partitions = Vec::new();

    for combination in 0..num_comb {
        let partition = generate_fanin_combination(&node.fanins, combination)?;
        if partition.y_fanins.len() <= 1
            || partition.y_fanins.len() > support
            || partition.z_fanins.is_empty()
        {
            continue;
        }
        partitions.push(partition);
    }

    Ok(partitions)
}

pub fn select_best_decomposition<F>(
    node: &AreaNode,
    support: usize,
    max_fanins: usize,
    mut decompose: F,
) -> Result<Option<SelectedDecomposition>, XlnKDeAreaError>
where
    F: FnMut(&FaninPartition) -> Option<CandidateDecomposition>,
{
    let mut best: Option<SelectedDecomposition> = None;

    for partition in valid_exhaustive_partitions(node, support, max_fanins)? {
        let Some(candidate) = decompose(&partition) else {
            continue;
        };
        if best.as_ref().is_none_or(|current| {
            candidate.internal_cost_after_root_ao_map < current.internal_cost_after_root_ao_map
        }) {
            best = Some(SelectedDecomposition {
                partition,
                internal_cost_after_root_ao_map: candidate.internal_cost_after_root_ao_map,
            });
        }
    }

    Ok(best)
}

pub fn plan_exhaustive_k_decomp_node<F>(
    node: &AreaNode,
    support: usize,
    max_fanins: usize,
    decompose: F,
) -> Result<NodeAction, XlnKDeAreaError>
where
    F: FnMut(&FaninPartition) -> Option<CandidateDecomposition>,
{
    validate_support(support)?;
    if node.kind != AreaNodeKind::Internal {
        return Ok(NodeAction::SkipNonInternal {
            node: node.name.clone(),
        });
    }
    let fanins = node.fanin_count();
    if fanins <= support {
        return Ok(NodeAction::AlreadyFeasible {
            node: node.name.clone(),
            fanins,
        });
    }
    if fanins > max_fanins {
        return Ok(NodeAction::TooManyFaninsForExhaustiveSearch {
            node: node.name.clone(),
            fanins,
            max_fanins,
        });
    }

    Ok(
        match select_best_decomposition(node, support, max_fanins, decompose)? {
            Some(selected) => NodeAction::ReplaceWithBestDecomposition {
                node: node.name.clone(),
                selected,
            },
            None => NodeAction::AoMapFallback {
                node: node.name.clone(),
                fanins,
                support,
            },
        },
    )
}

pub fn plan_exhaustive_k_decomp_network<F>(
    nodes_in_dfs_order: &[AreaNode],
    options: KDecompAreaOptions,
    mut decompose: F,
) -> Result<Vec<NodeAction>, XlnKDeAreaError>
where
    F: FnMut(&AreaNode, &FaninPartition) -> Option<CandidateDecomposition>,
{
    validate_support(options.support)?;
    let mut actions = Vec::new();

    for node in nodes_in_dfs_order {
        let action = plan_exhaustive_k_decomp_node(
            node,
            options.support,
            options.max_fanins_k_decomp,
            |partition| decompose(node, partition),
        )?;
        actions.push(action);
    }

    Ok(actions)
}

pub fn should_continue_after_replacement(options: KDecompAreaOptions) -> bool {
    options.recursive
}

fn validate_support(support: usize) -> Result<(), XlnKDeAreaError> {
    if support == 0 {
        Err(XlnKDeAreaError::InvalidSupport { support })
    } else {
        Ok(())
    }
}

fn checked_combination_count(fanins: usize) -> Result<usize, XlnKDeAreaError> {
    1usize
        .checked_shl(fanins as u32)
        .ok_or(XlnKDeAreaError::TooManyFaninsForBitMask { fanins })
}

fn binary_encoding(value: usize, length: usize) -> Result<Vec<bool>, XlnKDeAreaError> {
    if length >= usize::BITS as usize && value != 0 {
        return Err(XlnKDeAreaError::TooManyFaninsForBitMask { fanins: length });
    }
    Ok((0..length)
        .rev()
        .map(|bit| ((value >> bit) & 1) == 1)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn fanin_combination_matches_xln_filter_binary_partition_order() {
        let fanins = names(&["a", "b", "c"]);

        let first = generate_fanin_combination(&fanins, 1).unwrap();
        assert_eq!(first.y_fanins, names(&["a", "b"]));
        assert_eq!(first.z_fanins, names(&["c"]));
        assert_eq!(first.lambda_indices, vec![0, 1]);

        let second = generate_fanin_combination(&fanins, 2).unwrap();
        assert_eq!(second.y_fanins, names(&["a", "c"]));
        assert_eq!(second.z_fanins, names(&["b"]));
        assert_eq!(second.lambda_indices, vec![0, 2]);
    }

    #[test]
    fn valid_partitions_skip_trivial_empty_z_and_oversized_y_sets() {
        let node = AreaNode::internal("n", &["a", "b", "c"]);

        let partitions = valid_exhaustive_partitions(&node, 2, 3).unwrap();

        assert_eq!(
            partitions
                .iter()
                .map(|partition| partition.combination)
                .collect::<Vec<_>>(),
            vec![1, 2, 4]
        );
        assert_eq!(
            partitions
                .iter()
                .map(|partition| partition.lambda_indices.clone())
                .collect::<Vec<_>>(),
            vec![vec![0, 1], vec![0, 2], vec![1, 2]]
        );
    }

    #[test]
    fn node_selection_uses_lowest_post_ao_internal_cost() {
        let node = AreaNode::internal("n", &["a", "b", "c"]);

        let selected = select_best_decomposition(&node, 2, 3, |partition| {
            let cost = match partition.lambda_indices.as_slice() {
                [0, 1] => 4,
                [0, 2] => return None,
                [1, 2] => 2,
                _ => 9,
            };
            Some(CandidateDecomposition {
                internal_cost_after_root_ao_map: cost,
            })
        })
        .unwrap()
        .unwrap();

        assert_eq!(selected.partition.lambda_indices, vec![1, 2]);
        assert_eq!(selected.internal_cost_after_root_ao_map, 2);
    }

    #[test]
    fn node_planner_matches_c_early_returns_and_final_ao_fallback() {
        let input = AreaNode::new("pi", AreaNodeKind::PrimaryInput, Vec::new());
        assert_eq!(
            plan_exhaustive_k_decomp_node(&input, 2, 3, |_| unreachable!()).unwrap(),
            NodeAction::SkipNonInternal {
                node: "pi".to_owned()
            }
        );

        let feasible = AreaNode::internal("small", &["a", "b"]);
        assert_eq!(
            plan_exhaustive_k_decomp_node(&feasible, 2, 3, |_| unreachable!()).unwrap(),
            NodeAction::AlreadyFeasible {
                node: "small".to_owned(),
                fanins: 2,
            }
        );

        let too_wide = AreaNode::internal("wide", &["a", "b", "c", "d"]);
        assert_eq!(
            plan_exhaustive_k_decomp_node(&too_wide, 2, 3, |_| unreachable!()).unwrap(),
            NodeAction::TooManyFaninsForExhaustiveSearch {
                node: "wide".to_owned(),
                fanins: 4,
                max_fanins: 3,
            }
        );

        let no_candidate = AreaNode::internal("hard", &["a", "b", "c"]);
        assert_eq!(
            plan_exhaustive_k_decomp_node(&no_candidate, 2, 3, |_| None).unwrap(),
            NodeAction::AoMapFallback {
                node: "hard".to_owned(),
                fanins: 3,
                support: 2,
            }
        );
    }

    #[test]
    fn network_planner_visits_nodes_in_supplied_dfs_order() {
        let nodes = vec![
            AreaNode::new("a", AreaNodeKind::PrimaryInput, Vec::new()),
            AreaNode::internal("n1", &["a", "b", "c"]),
            AreaNode::internal("n2", &["a", "b"]),
        ];

        let actions = plan_exhaustive_k_decomp_network(
            &nodes,
            KDecompAreaOptions::new(2, 3, false),
            |node, partition| {
                if node.name == "n1" && partition.lambda_indices == [0, 1] {
                    Some(CandidateDecomposition {
                        internal_cost_after_root_ao_map: 1,
                    })
                } else {
                    None
                }
            },
        )
        .unwrap();

        assert!(matches!(actions[0], NodeAction::SkipNonInternal { .. }));
        assert!(matches!(
            actions[1],
            NodeAction::ReplaceWithBestDecomposition { .. }
        ));
        assert!(matches!(actions[2], NodeAction::AlreadyFeasible { .. }));
    }

    #[test]
    fn recursive_flag_controls_c_while_loop_continuation() {
        assert!(!should_continue_after_replacement(KDecompAreaOptions::new(
            5, 7, false
        )));
        assert!(should_continue_after_replacement(KDecompAreaOptions::new(
            5, 7, true
        )));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_k_de_area.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
