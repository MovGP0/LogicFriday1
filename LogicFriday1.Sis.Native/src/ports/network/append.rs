//! Native network append support for the SIS network layer.
//!
//! The legacy routine merges a duplicate of the right-hand network into the
//! left-hand network by name.  Matching primary inputs are shared, a right-hand
//! driver can replace a left-hand primary input with the same name, and two
//! driven nodes with the same name are reported as an append conflict.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use super::dfs::{network_dfs, Network as DfsNetwork, NetworkDfsError, NetworkNode as DfsNode};
use super::network_util::{
    BoolExpr, Network, NetworkNode, NetworkUtilError, NetworkUtilResult, NodeId, NodeKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendReport {
    pub copied_nodes: usize,
    pub shared_primary_inputs: usize,
    pub replaced_primary_inputs: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppendError {
    Network(NetworkUtilError),
    Cycle(NetworkDfsError),
    DuplicateDrivenName(String),
    MissingCopiedFanin { node: NodeId, fanin: NodeId },
    MissingExpressionLiteral { node: NodeId },
    InvalidPrimaryOutput(NodeId),
}

impl fmt::Display for AppendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(formatter, "{error}"),
            Self::Cycle(error) => write!(formatter, "{error}"),
            Self::DuplicateDrivenName(name) => {
                write!(formatter, "network_append: node '{name}' already driven")
            }
            Self::MissingCopiedFanin { node, fanin } => write!(
                formatter,
                "node {} references unmapped appended fanin {}",
                node.index(),
                fanin.index()
            ),
            Self::MissingExpressionLiteral { node } => write!(
                formatter,
                "expression references unmapped appended node {}",
                node.index()
            ),
            Self::InvalidPrimaryOutput(node) => write!(
                formatter,
                "primary output {} must have exactly one fanin",
                node.index()
            ),
        }
    }
}

impl Error for AppendError {}

impl From<NetworkUtilError> for AppendError {
    fn from(value: NetworkUtilError) -> Self {
        Self::Network(value)
    }
}

impl From<NetworkDfsError> for AppendError {
    fn from(value: NetworkDfsError) -> Self {
        Self::Cycle(value)
    }
}

pub type AppendResult<T> = Result<T, AppendError>;

pub fn network_append(network: &mut Network, appended: &Network) -> AppendResult<AppendReport> {
    let mut target = network.duplicate()?;
    let source = appended.duplicate()?;
    let original_nodes = target.nodes().map(|(id, _)| id).collect::<Vec<_>>();
    let mut copied_nodes = Vec::new();
    let mut source_to_target = BTreeMap::new();
    let mut replaced_inputs = BTreeMap::new();
    let mut shared_primary_inputs = 0;

    for (source_id, source_node) in source.nodes() {
        match target.find_node(&source_node.name) {
            None => {
                let copied = reserve_node(&mut target, source_node)?;
                source_to_target.insert(source_id, copied);
                copied_nodes.push((source_id, copied, source_node.clone()));
            }
            Some(_) if is_madeup_name(&source_node.name) => {
                let copied = reserve_node(&mut target, source_node)?;
                source_to_target.insert(source_id, copied);
                copied_nodes.push((source_id, copied, source_node.clone()));
            }
            Some(existing) if source_node.kind == NodeKind::PrimaryInput => {
                source_to_target.insert(source_id, existing);
                shared_primary_inputs += 1;
            }
            Some(existing) => {
                let copied = reserve_node_with_unique_name(&mut target, source_node)?;
                source_to_target.insert(source_id, copied);
                copied_nodes.push((source_id, copied, source_node.clone()));

                if target.node(existing)?.kind == NodeKind::PrimaryInput {
                    replaced_inputs.insert(existing, copied);
                    target.swap_names(existing, copied)?;
                } else {
                    return Err(AppendError::DuplicateDrivenName(source_node.name.clone()));
                }
            }
        }
    }

    for (source_id, copied, source_node) in &copied_nodes {
        install_copied_node(
            &mut target,
            *source_id,
            *copied,
            source_node,
            &source_to_target,
        )?;
    }

    rewire_existing_nodes(&mut target, &original_nodes, &replaced_inputs)?;
    bypass_primary_output_fanins(&mut target)?;
    rebuild_fanouts(&mut target)?;

    for replaced_input in replaced_inputs.keys().copied().collect::<Vec<_>>() {
        if target.node(replaced_input)?.kind == NodeKind::PrimaryInput {
            target.delete_node(replaced_input)?;
        }
    }

    rebuild_fanouts(&mut target)?;
    ensure_acyclic(&target)?;

    *network = target;
    Ok(AppendReport {
        copied_nodes: copied_nodes.len(),
        shared_primary_inputs,
        replaced_primary_inputs: replaced_inputs.len(),
    })
}

fn reserve_node(network: &mut Network, source_node: &NetworkNode) -> NetworkUtilResult<NodeId> {
    let mut placeholder = NetworkNode::new(source_node.name.clone(), NodeKind::PrimaryInput);
    placeholder.short_name = source_node.short_name.clone();
    network.add_node(placeholder)
}

fn reserve_node_with_unique_name(
    network: &mut Network,
    source_node: &NetworkNode,
) -> NetworkUtilResult<NodeId> {
    let mut placeholder = NetworkNode::new(
        unique_name(network, &source_node.name),
        NodeKind::PrimaryInput,
    );
    placeholder.short_name = source_node.short_name.clone();
    network.add_node(placeholder)
}

fn install_copied_node(
    target: &mut Network,
    source_id: NodeId,
    copied: NodeId,
    source_node: &NetworkNode,
    source_to_target: &BTreeMap<NodeId, NodeId>,
) -> AppendResult<()> {
    let fanins = source_node
        .fanins
        .iter()
        .map(|fanin| {
            source_to_target
                .get(fanin)
                .copied()
                .ok_or(AppendError::MissingCopiedFanin {
                    node: source_id,
                    fanin: *fanin,
                })
        })
        .collect::<AppendResult<Vec<_>>>()?;
    let expression = source_node
        .expression
        .clone()
        .map(|expression| remap_expression(expression, source_to_target))
        .transpose()?;

    {
        let target_node = target.node_mut(copied)?;
        target_node.fanins = fanins;
        target_node.cover = source_node.cover.clone();
        target_node.expression = expression;
    }

    target.change_node_type(copied, source_node.kind)?;
    Ok(())
}

fn rewire_existing_nodes(
    network: &mut Network,
    original_nodes: &[NodeId],
    replacements: &BTreeMap<NodeId, NodeId>,
) -> NetworkUtilResult<()> {
    for node in original_nodes {
        if network.node(*node).is_err() {
            continue;
        }

        let fanins = network
            .node(*node)?
            .fanins
            .iter()
            .map(|fanin| replacements.get(fanin).copied().unwrap_or(*fanin))
            .collect::<Vec<_>>();
        network.node_mut(*node)?.fanins = fanins;
    }

    Ok(())
}

fn bypass_primary_output_fanins(network: &mut Network) -> AppendResult<()> {
    let nodes = network.nodes().map(|(id, _)| id).collect::<Vec<_>>();
    for node in nodes {
        let fanins = network.node(node)?.fanins.clone();
        let mut patched = Vec::with_capacity(fanins.len());

        for fanin in fanins {
            let fanin_node = network.node(fanin)?;
            if fanin_node.kind == NodeKind::PrimaryOutput {
                let driver = fanin_node
                    .fanins
                    .first()
                    .copied()
                    .ok_or(AppendError::InvalidPrimaryOutput(fanin))?;
                patched.push(driver);
            } else {
                patched.push(fanin);
            }
        }

        network.node_mut(node)?.fanins = patched;
    }

    Ok(())
}

fn rebuild_fanouts(network: &mut Network) -> NetworkUtilResult<()> {
    let nodes = network.nodes().map(|(id, _)| id).collect::<Vec<_>>();
    for node in &nodes {
        network.node_mut(*node)?.fanouts.clear();
    }

    for node in nodes {
        let fanins = network.node(node)?.fanins.clone();
        for fanin in fanins {
            network.node(fanin)?;
            network.node_mut(fanin)?.fanouts.insert(node);
        }
    }

    Ok(())
}

fn ensure_acyclic(network: &Network) -> AppendResult<()> {
    let nodes = network
        .nodes()
        .map(|(id, node)| {
            let kind = match node.kind {
                NodeKind::PrimaryInput => super::dfs::NodeKind::PrimaryInput,
                NodeKind::PrimaryOutput => super::dfs::NodeKind::PrimaryOutput,
                NodeKind::Internal | NodeKind::Unassigned => super::dfs::NodeKind::Internal,
            };
            DfsNode::new(id.index(), node.name.clone(), kind).with_fanins(
                node.fanins
                    .iter()
                    .map(|fanin| super::dfs::NodeId(fanin.index())),
            )
        })
        .collect::<Vec<_>>();
    let dfs_network = DfsNetwork::new(nodes)?;
    network_dfs(&dfs_network)?;
    Ok(())
}

fn unique_name(network: &Network, base: &str) -> String {
    let mut count = 0;
    loop {
        let candidate = format!("{base}-{count}");
        if network.find_node(&candidate).is_none() {
            return candidate;
        }
        count += 1;
    }
}

fn is_madeup_name(name: &str) -> bool {
    name.strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .is_some_and(|value| !value.is_empty() && value.chars().all(|item| item.is_ascii_digit()))
}

fn remap_expression(
    expression: BoolExpr,
    source_to_target: &BTreeMap<NodeId, NodeId>,
) -> AppendResult<BoolExpr> {
    match expression {
        BoolExpr::Constant(value) => Ok(BoolExpr::Constant(value)),
        BoolExpr::Literal { node, phase } => {
            let node = source_to_target
                .get(&node)
                .copied()
                .ok_or(AppendError::MissingExpressionLiteral { node })?;
            Ok(BoolExpr::Literal { node, phase })
        }
        BoolExpr::Not(inner) => Ok(BoolExpr::Not(Box::new(remap_expression(
            *inner,
            source_to_target,
        )?))),
        BoolExpr::And(items) => Ok(BoolExpr::And(
            items
                .into_iter()
                .map(|item| remap_expression(item, source_to_target))
                .collect::<AppendResult<Vec<_>>>()?,
        )),
        BoolExpr::Or(items) => Ok(BoolExpr::Or(
            items
                .into_iter()
                .map(|item| remap_expression(item, source_to_target))
                .collect::<AppendResult<Vec<_>>>()?,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::super::network_util::{CoverValue, Cube, SopCover};
    use super::*;

    fn one_cube(values: &[CoverValue]) -> SopCover {
        SopCover::new([Cube::new(values.to_vec())])
    }

    fn input(network: &mut Network, name: &str) -> NodeId {
        network
            .add_primary_input(NetworkNode::new(name, NodeKind::PrimaryInput))
            .unwrap()
    }

    fn output(network: &mut Network, fanin: NodeId) -> NodeId {
        network.add_primary_output(fanin).unwrap()
    }

    #[test]
    fn append_shares_primary_inputs_and_copies_rhs_logic() {
        let mut left = Network::new();
        let left_a = input(&mut left, "a");
        let left_n = left
            .add_internal("left_n", [left_a], one_cube(&[CoverValue::One]))
            .unwrap();
        output(&mut left, left_n);

        let mut right = Network::new();
        let right_a = input(&mut right, "a");
        let right_n = right
            .add_internal("right_n", [right_a], one_cube(&[CoverValue::Zero]))
            .unwrap();
        output(&mut right, right_n);

        let report = network_append(&mut left, &right).unwrap();
        let copied_output = left.find_node("right_n").unwrap();
        let copied_driver = left.node(copied_output).unwrap().fanins[0];

        assert_eq!(report.shared_primary_inputs, 1);
        assert_eq!(left.node(copied_driver).unwrap().fanins, vec![left_a]);
        assert!(left.node(left_a).unwrap().fanouts.contains(&copied_driver));
    }

    #[test]
    fn append_replaces_matching_primary_input_with_rhs_driver() {
        let mut left = Network::new();
        let old_y = input(&mut left, "y");
        let sink = left
            .add_internal("sink", [old_y], one_cube(&[CoverValue::One]))
            .unwrap();
        output(&mut left, sink);

        let mut right = Network::new();
        let a = input(&mut right, "a");
        let driver = right
            .add_internal("y", [a], one_cube(&[CoverValue::One]))
            .unwrap();
        output(&mut right, driver);

        let report = network_append(&mut left, &right).unwrap();
        let new_y = left.find_node("y").unwrap();
        let new_y_driver = left.node(new_y).unwrap().fanins[0];

        assert_eq!(report.replaced_primary_inputs, 1);
        assert!(left.node(old_y).is_err());
        assert_eq!(left.node(sink).unwrap().fanins, vec![new_y_driver]);
        assert_eq!(left.node(new_y).unwrap().kind, NodeKind::PrimaryOutput);
    }

    #[test]
    fn append_rejects_two_driven_nodes_with_same_name_transactionally() {
        let mut left = Network::new();
        let a = input(&mut left, "a");
        left.add_internal("n", [a], one_cube(&[CoverValue::One]))
            .unwrap();
        let before = left.clone();

        let mut right = Network::new();
        let b = input(&mut right, "b");
        right
            .add_internal("n", [b], one_cube(&[CoverValue::One]))
            .unwrap();

        let error = network_append(&mut left, &right).unwrap_err();

        assert_eq!(error, AppendError::DuplicateDrivenName("n".to_string()));
        assert_eq!(left, before);
    }

    #[test]
    fn append_bypasses_fanins_that_target_primary_outputs() {
        let mut left = Network::new();
        let a = input(&mut left, "a");
        let left_driver = left
            .add_internal("left_driver", [a], one_cube(&[CoverValue::One]))
            .unwrap();
        let left_output = output(&mut left, left_driver);

        let mut right = Network::new();
        let right_output = input(&mut right, left.node(left_output).unwrap().name.as_str());
        let sink = right
            .add_internal("sink", [right_output], one_cube(&[CoverValue::One]))
            .unwrap();
        output(&mut right, sink);

        network_append(&mut left, &right).unwrap();
        let sink_output = left.find_node("sink").unwrap();
        let sink_driver = left.node(sink_output).unwrap().fanins[0];

        assert_eq!(left.node(sink_driver).unwrap().fanins, vec![left_driver]);
    }

    #[test]
    fn append_rejects_resulting_cycles() {
        let mut left = Network::new();
        let a = input(&mut left, "a");
        let y = left
            .add_internal("y", [a], one_cube(&[CoverValue::One]))
            .unwrap();
        output(&mut left, y);

        let mut right = Network::new();
        let y_input = input(&mut right, "y");
        let a_driver = right
            .add_internal("a", [y_input], one_cube(&[CoverValue::One]))
            .unwrap();
        output(&mut right, a_driver);

        assert!(matches!(
            network_append(&mut left, &right),
            Err(AppendError::Cycle(_))
        ));
    }
}
