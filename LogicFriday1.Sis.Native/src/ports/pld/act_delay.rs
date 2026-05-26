//! Owned Rust delay-ordering helpers for Actel PLD mapping.
//!
//! The original routine orders an internal node's fanins by descending arrival
//! time using the mapping cost table. This port keeps that behavior independent
//! of SIS pointer tables by using node names and owned records.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActNodeKind {
    PrimaryInput,
    PrimaryOutput,
    ConstantZero,
    ConstantOne,
    Internal,
}

impl ActNodeKind {
    pub const fn is_delay_ordered(self) -> bool {
        matches!(self, Self::Internal)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActDelayNode {
    pub id: NodeId,
    pub name: String,
    pub kind: ActNodeKind,
    fanins: Vec<NodeId>,
}

impl ActDelayNode {
    pub fn new(
        id: NodeId,
        name: impl Into<String>,
        kind: ActNodeKind,
        fanins: Vec<NodeId>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            fanins,
        }
    }

    pub fn primary_input(id: NodeId, name: impl Into<String>) -> Self {
        Self::new(id, name, ActNodeKind::PrimaryInput, Vec::new())
    }

    pub fn primary_output(id: NodeId, name: impl Into<String>, fanins: Vec<NodeId>) -> Self {
        Self::new(id, name, ActNodeKind::PrimaryOutput, fanins)
    }

    pub fn constant_zero(id: NodeId, name: impl Into<String>) -> Self {
        Self::new(id, name, ActNodeKind::ConstantZero, Vec::new())
    }

    pub fn constant_one(id: NodeId, name: impl Into<String>) -> Self {
        Self::new(id, name, ActNodeKind::ConstantOne, Vec::new())
    }

    pub fn internal(id: NodeId, name: impl Into<String>, fanins: Vec<NodeId>) -> Self {
        Self::new(id, name, ActNodeKind::Internal, fanins)
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActDelayNetwork {
    nodes: Vec<ActDelayNode>,
}

impl ActDelayNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        kind: ActNodeKind,
        fanins: Vec<NodeId>,
    ) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(ActDelayNode::new(id, name, kind, fanins));
        id
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> NodeId {
        self.add_node(name, ActNodeKind::PrimaryInput, Vec::new())
    }

    pub fn add_internal(&mut self, name: impl Into<String>, fanins: Vec<NodeId>) -> NodeId {
        self.add_node(name, ActNodeKind::Internal, fanins)
    }

    pub fn node(&self, id: NodeId) -> ActDelayResult<&ActDelayNode> {
        self.nodes.get(id.0).ok_or(ActDelayError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[ActDelayNode] {
        &self.nodes
    }

    pub fn node_name(&self, id: NodeId) -> ActDelayResult<&str> {
        Ok(&self.node(id)?.name)
    }

    pub fn order_for_delay(
        &self,
        node: NodeId,
        cost_table: &ActCostTable,
    ) -> ActDelayResult<Option<Vec<NodeId>>> {
        act_order_for_delay(self, node, cost_table)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActCost {
    pub node: NodeId,
    pub arrival_time: f64,
}

impl ActCost {
    pub const fn new(node: NodeId, arrival_time: f64) -> Self {
        Self { node, arrival_time }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActCostTable {
    by_name: BTreeMap<String, ActCost>,
}

impl ActCostTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, cost: ActCost) -> Option<ActCost> {
        self.by_name.insert(name.into(), cost)
    }

    pub fn get(&self, name: &str) -> Option<&ActCost> {
        self.by_name.get(name)
    }

    pub fn from_network_arrivals(
        network: &ActDelayNetwork,
        arrivals: impl IntoIterator<Item = (NodeId, f64)>,
    ) -> ActDelayResult<Self> {
        let mut table = Self::new();
        for (node, arrival_time) in arrivals {
            let name = network.node_name(node)?.to_owned();
            table.insert(name, ActCost::new(node, arrival_time));
        }
        Ok(table)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActDelayError {
    UnknownNode(NodeId),
    MissingCost {
        fanin: NodeId,
        name: String,
    },
    CostNodeMismatch {
        name: String,
        expected: NodeId,
        actual: NodeId,
    },
}

impl fmt::Display for ActDelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown act_delay node {:?}", node),
            Self::MissingCost { fanin, name } => {
                write!(
                    f,
                    "missing act_delay cost entry for fanin {:?} named {name}",
                    fanin
                )
            }
            Self::CostNodeMismatch {
                name,
                expected,
                actual,
            } => write!(
                f,
                "act_delay cost entry {name} points to {:?}, expected {:?}",
                actual, expected
            ),
        }
    }
}

impl Error for ActDelayError {}

pub type ActDelayResult<T> = Result<T, ActDelayError>;

pub fn act_order_for_delay(
    network: &ActDelayNetwork,
    node: NodeId,
    cost_table: &ActCostTable,
) -> ActDelayResult<Option<Vec<NodeId>>> {
    let node_ref = network.node(node)?;
    if !node_ref.kind.is_delay_ordered() {
        return Ok(None);
    }

    let mut fanin_costs = Vec::with_capacity(node_ref.fanins().len());
    for fanin in node_ref.fanins() {
        let fanin_ref = network.node(*fanin)?;
        let cost = cost_table
            .get(&fanin_ref.name)
            .ok_or_else(|| ActDelayError::MissingCost {
                fanin: *fanin,
                name: fanin_ref.name.clone(),
            })?;
        if cost.node != *fanin {
            return Err(ActDelayError::CostNodeMismatch {
                name: fanin_ref.name.clone(),
                expected: *fanin,
                actual: cost.node,
            });
        }
        fanin_costs.push((*fanin, cost.arrival_time));
    }

    fanin_costs.sort_by(|left, right| arrival_compare(right.1, left.1));
    Ok(Some(
        fanin_costs
            .into_iter()
            .map(|(fanin, _arrival_time)| fanin)
            .collect(),
    ))
}

pub fn arrival_compare(left: f64, right: f64) -> Ordering {
    if left > right {
        Ordering::Greater
    } else if left < right {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> (ActDelayNetwork, NodeId, NodeId, NodeId, NodeId) {
        let mut network = ActDelayNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let c = network.add_primary_input("c");
        let f = network.add_internal("f", vec![a, b, c]);

        (network, a, b, c, f)
    }

    #[test]
    fn internal_node_fanins_are_ordered_by_descending_arrival_time() {
        let (network, a, b, c, f) = sample_network();
        let cost_table =
            ActCostTable::from_network_arrivals(&network, [(a, 1.0), (b, 4.0), (c, 2.5)]).unwrap();

        let order = act_order_for_delay(&network, f, &cost_table).unwrap();

        assert_eq!(order, Some(vec![b, c, a]));
    }

    #[test]
    fn equal_arrival_times_keep_fanin_order_deterministically() {
        let (network, a, b, c, f) = sample_network();
        let cost_table =
            ActCostTable::from_network_arrivals(&network, [(a, 1.0), (b, 1.0), (c, 0.0)]).unwrap();

        let order = network.order_for_delay(f, &cost_table).unwrap();

        assert_eq!(order, Some(vec![a, b, c]));
    }

    #[test]
    fn non_internal_nodes_have_no_delay_order() {
        let (mut network, a, _b, _c, _f) = sample_network();
        let po = network.add_node("out", ActNodeKind::PrimaryOutput, vec![a]);
        let zero = network.add_node("zero", ActNodeKind::ConstantZero, Vec::new());
        let one = network.add_node("one", ActNodeKind::ConstantOne, Vec::new());
        let cost_table = ActCostTable::new();

        assert_eq!(network.order_for_delay(a, &cost_table).unwrap(), None);
        assert_eq!(network.order_for_delay(po, &cost_table).unwrap(), None);
        assert_eq!(network.order_for_delay(zero, &cost_table).unwrap(), None);
        assert_eq!(network.order_for_delay(one, &cost_table).unwrap(), None);
    }

    #[test]
    fn missing_cost_entry_is_reported_for_named_fanin() {
        let (network, a, b, _c, f) = sample_network();
        let cost_table =
            ActCostTable::from_network_arrivals(&network, [(a, 1.0), (b, 2.0)]).unwrap();

        assert_eq!(
            network.order_for_delay(f, &cost_table),
            Err(ActDelayError::MissingCost {
                fanin: NodeId(2),
                name: "c".to_owned(),
            })
        );
    }

    #[test]
    fn cost_table_entry_must_reference_the_named_fanin() {
        let (network, a, b, c, f) = sample_network();
        let mut cost_table =
            ActCostTable::from_network_arrivals(&network, [(a, 1.0), (c, 3.0)]).unwrap();
        cost_table.insert("b", ActCost::new(c, 2.0));

        assert_eq!(
            network.order_for_delay(f, &cost_table),
            Err(ActDelayError::CostNodeMismatch {
                name: "b".to_owned(),
                expected: b,
                actual: c,
            })
        );
    }

    #[test]
    fn unknown_node_and_unknown_arrival_node_are_diagnostics() {
        let (network, a, _b, _c, _f) = sample_network();
        let cost_table = ActCostTable::new();

        assert_eq!(
            network.order_for_delay(NodeId(99), &cost_table),
            Err(ActDelayError::UnknownNode(NodeId(99)))
        );
        assert_eq!(
            ActCostTable::from_network_arrivals(&network, [(a, 1.0), (NodeId(99), 0.0)]),
            Err(ActDelayError::UnknownNode(NodeId(99)))
        );
    }

    #[test]
    fn arrival_compare_matches_c_comparator_signs() {
        assert_eq!(arrival_compare(3.0, 2.0), Ordering::Greater);
        assert_eq!(arrival_compare(2.0, 3.0), Ordering::Less);
        assert_eq!(arrival_compare(2.0, 2.0), Ordering::Equal);
        assert_eq!(arrival_compare(f64::NAN, 2.0), Ordering::Equal);
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_beads_metadata_are_present_in_this_port() {
        let source = include_str!("act_delay.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
