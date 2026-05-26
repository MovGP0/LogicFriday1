//! Native Rust port scaffold for `sis/speed/sp_network.c`.
//!
//! The C helpers build a temporary network from a node, copy delay parameters
//! between original and temporary PI/PO nodes, then duplicate network nodes into
//! arrays while rewiring fanin pointers. This module ports those rules into
//! explicit Rust copy plans. Applying the plans to real SIS nodes remains
//! blocked until native network/node/delay ports are available.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DelayParameter {
    BlockRise,
    DriveRise,
    BlockFall,
    DriveFall,
    MaxInputLoad,
    ArrivalRise,
    ArrivalFall,
    OutputLoad,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelaySnapshot {
    pub params: HashMap<DelayParameter, f64>,
    pub arrival: Option<DelayTime>,
    pub speed_arrival: Option<DelayTime>,
    pub load: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayCopyPlan {
    pub copied_parameters: Vec<(DelayParameter, f64)>,
}

pub fn plan_delay_parameter_duplication(
    from: &DelaySnapshot,
    to_kind: NodeKind,
    delay_flag: bool,
) -> Result<DelayCopyPlan, SpNetworkError> {
    let base_params = [
        DelayParameter::BlockRise,
        DelayParameter::DriveRise,
        DelayParameter::BlockFall,
        DelayParameter::DriveFall,
        DelayParameter::MaxInputLoad,
    ];
    let mut copied_parameters = Vec::new();
    for parameter in base_params {
        let value = from
            .params
            .get(&parameter)
            .copied()
            .ok_or(SpNetworkError::MissingDelayParameter(parameter))?;
        copied_parameters.push((parameter, value));
    }

    match to_kind {
        NodeKind::PrimaryInput => {
            let time = if delay_flag {
                from.speed_arrival
                    .ok_or(SpNetworkError::MissingSpeedArrival)?
            } else {
                from.arrival.ok_or(SpNetworkError::MissingArrival)?
            };
            copied_parameters.push((DelayParameter::ArrivalRise, time.rise));
            copied_parameters.push((DelayParameter::ArrivalFall, time.fall));
        }
        NodeKind::PrimaryOutput => {
            copied_parameters.push((
                DelayParameter::OutputLoad,
                from.load.ok_or(SpNetworkError::MissingLoad)?,
            ));
        }
        NodeKind::Internal => {}
    }

    Ok(DelayCopyPlan { copied_parameters })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub id: usize,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<usize>,
    pub library_gate: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayNodeCopy {
    pub original_id: usize,
    pub copied_id: usize,
    pub copied_fanins: Vec<usize>,
    pub library_gate: Option<String>,
}

pub fn plan_network_to_array(nodes_in_dfs_order: &[NetworkNode]) -> Vec<ArrayNodeCopy> {
    let id_to_copy: HashMap<usize, usize> = nodes_in_dfs_order
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id, index))
        .collect();

    nodes_in_dfs_order
        .iter()
        .enumerate()
        .map(|(copy_id, node)| ArrayNodeCopy {
            original_id: node.id,
            copied_id: copy_id,
            copied_fanins: node
                .fanins
                .iter()
                .filter_map(|fanin| id_to_copy.get(fanin).copied())
                .collect(),
            library_gate: None,
        })
        .collect()
}

pub fn plan_network_and_node_to_array(
    nodes_in_dfs_order: &[NetworkNode],
    original_inputs_by_name: &HashMap<String, usize>,
) -> Result<Vec<ArrayNodeCopy>, SpNetworkError> {
    let id_to_copy: HashMap<usize, usize> = nodes_in_dfs_order
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id, index))
        .collect();

    let mut copies = Vec::new();
    for (copy_id, node) in nodes_in_dfs_order.iter().enumerate() {
        let mut copied_fanins = Vec::new();
        for fanin_id in &node.fanins {
            let Some(fanin) = nodes_in_dfs_order
                .iter()
                .find(|candidate| candidate.id == *fanin_id)
            else {
                return Err(SpNetworkError::UnknownFanin(*fanin_id));
            };

            if fanin.kind == NodeKind::PrimaryInput {
                let original = original_inputs_by_name
                    .get(&fanin.name)
                    .copied()
                    .ok_or_else(|| SpNetworkError::MissingOriginalInput(fanin.name.clone()))?;
                copied_fanins.push(original);
            } else {
                copied_fanins.push(
                    id_to_copy
                        .get(fanin_id)
                        .copied()
                        .ok_or(SpNetworkError::UnknownFanin(*fanin_id))?,
                );
            }
        }

        copies.push(ArrayNodeCopy {
            original_id: node.id,
            copied_id: copy_id,
            copied_fanins,
            library_gate: node.library_gate.clone(),
        });
    }
    Ok(copies)
}

pub fn speed_network_create_from_node_bound() -> Result<(), SpNetworkError> {
    Err(SpNetworkError::MissingDependency(
        "speed_network_create_from_node requires native network_create_from_node, PI/PO traversal, node lookup, and delay parameter APIs",
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpNetworkError {
    MissingDelayParameter(DelayParameter),
    MissingArrival,
    MissingSpeedArrival,
    MissingLoad,
    UnknownFanin(usize),
    MissingOriginalInput(String),
    MissingDependency(&'static str),
}

impl fmt::Display for SpNetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDelayParameter(parameter) => {
                write!(f, "missing delay parameter {parameter:?}")
            }
            Self::MissingArrival => write!(f, "missing ordinary arrival time"),
            Self::MissingSpeedArrival => write!(f, "missing speed arrival time"),
            Self::MissingLoad => write!(f, "missing output load"),
            Self::UnknownFanin(id) => write!(f, "unknown fanin node id {id}"),
            Self::MissingOriginalInput(name) => {
                write!(f, "failed to retrieve original input node named {name}")
            }
            Self::MissingDependency(message) => write!(f, "{message}"),
        }
    }
}

impl Error for SpNetworkError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> DelaySnapshot {
        let mut params = HashMap::new();
        params.insert(DelayParameter::BlockRise, 1.0);
        params.insert(DelayParameter::DriveRise, 2.0);
        params.insert(DelayParameter::BlockFall, 3.0);
        params.insert(DelayParameter::DriveFall, 4.0);
        params.insert(DelayParameter::MaxInputLoad, 5.0);
        DelaySnapshot {
            params,
            arrival: Some(DelayTime {
                rise: 6.0,
                fall: 7.0,
            }),
            speed_arrival: Some(DelayTime {
                rise: 8.0,
                fall: 9.0,
            }),
            load: Some(10.0),
        }
    }

    #[test]
    fn duplicates_base_delay_parameters_and_pi_arrival_from_selected_source() {
        let normal =
            plan_delay_parameter_duplication(&snapshot(), NodeKind::PrimaryInput, false).unwrap();
        assert!(
            normal
                .copied_parameters
                .contains(&(DelayParameter::ArrivalRise, 6.0))
        );
        assert!(
            normal
                .copied_parameters
                .contains(&(DelayParameter::ArrivalFall, 7.0))
        );

        let speed =
            plan_delay_parameter_duplication(&snapshot(), NodeKind::PrimaryInput, true).unwrap();
        assert!(
            speed
                .copied_parameters
                .contains(&(DelayParameter::ArrivalRise, 8.0))
        );
        assert!(
            speed
                .copied_parameters
                .contains(&(DelayParameter::ArrivalFall, 9.0))
        );
    }

    #[test]
    fn duplicates_output_load_for_primary_outputs() {
        let plan =
            plan_delay_parameter_duplication(&snapshot(), NodeKind::PrimaryOutput, false).unwrap();

        assert!(
            plan.copied_parameters
                .contains(&(DelayParameter::OutputLoad, 10.0))
        );
    }

    #[test]
    fn network_to_array_rewrites_fanins_to_copied_nodes() {
        let nodes = vec![
            NetworkNode {
                id: 10,
                name: "a".to_string(),
                kind: NodeKind::PrimaryInput,
                fanins: vec![],
                library_gate: None,
            },
            NetworkNode {
                id: 20,
                name: "n".to_string(),
                kind: NodeKind::Internal,
                fanins: vec![10],
                library_gate: Some("g1".to_string()),
            },
        ];

        assert_eq!(
            plan_network_to_array(&nodes),
            vec![
                ArrayNodeCopy {
                    original_id: 10,
                    copied_id: 0,
                    copied_fanins: vec![],
                    library_gate: None,
                },
                ArrayNodeCopy {
                    original_id: 20,
                    copied_id: 1,
                    copied_fanins: vec![0],
                    library_gate: None,
                },
            ]
        );
    }

    #[test]
    fn network_and_node_to_array_patches_primary_inputs_to_original_nodes() {
        let nodes = vec![
            NetworkNode {
                id: 10,
                name: "a".to_string(),
                kind: NodeKind::PrimaryInput,
                fanins: vec![],
                library_gate: None,
            },
            NetworkNode {
                id: 20,
                name: "n".to_string(),
                kind: NodeKind::Internal,
                fanins: vec![10],
                library_gate: Some("g1".to_string()),
            },
        ];
        let originals = HashMap::from([("a".to_string(), 99)]);

        assert_eq!(
            plan_network_and_node_to_array(&nodes, &originals).unwrap()[1],
            ArrayNodeCopy {
                original_id: 20,
                copied_id: 1,
                copied_fanins: vec![99],
                library_gate: Some("g1".to_string()),
            }
        );
    }

    #[test]
    fn network_bound_entry_point_reports_missing_dependencies() {
        assert_eq!(
            speed_network_create_from_node_bound(),
            Err(SpNetworkError::MissingDependency(
                "speed_network_create_from_node requires native network_create_from_node, PI/PO traversal, node lookup, and delay parameter APIs",
            ))
        );
    }
}
