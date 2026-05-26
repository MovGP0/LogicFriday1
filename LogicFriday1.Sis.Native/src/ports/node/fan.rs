use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FanoutRecord {
    pub fanout: NodeId,
    pub pin: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanNode {
    pub name: String,
    pub kind: NodeKind,
    fanins: Vec<NodeId>,
    fanouts: Vec<FanoutRecord>,
    duplicate_free: bool,
}

impl FanNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            duplicate_free: true,
        }
    }

    pub fn with_fanins(mut self, fanins: Vec<NodeId>) -> Self {
        self.fanins = fanins;
        self.duplicate_free = false;
        self
    }

    pub fn fanins(&self) -> &[NodeId] {
        &self.fanins
    }

    pub fn fanouts(&self) -> &[FanoutRecord] {
        &self.fanouts
    }

    pub fn is_duplicate_free(&self) -> bool {
        self.duplicate_free
    }

    pub fn mark_duplicate_free(&mut self) {
        self.duplicate_free = true;
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FanNetwork {
    nodes: Vec<FanNode>,
}

impl FanNetwork {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: FanNode) -> Result<NodeId, FanError> {
        for fanin in &node.fanins {
            self.node(*fanin)?;
        }

        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        self.add_fanout_records(id)?;
        Ok(id)
    }

    pub fn node(&self, id: NodeId) -> Result<&FanNode, FanError> {
        self.nodes.get(id.0).ok_or(FanError::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut FanNode, FanError> {
        self.nodes.get_mut(id.0).ok_or(FanError::UnknownNode(id))
    }

    pub fn nodes(&self) -> &[FanNode] {
        &self.nodes
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_fanin(&self, node: NodeId, index: usize) -> Result<NodeId, FanError> {
        self.node(node)?
            .fanins
            .get(index)
            .copied()
            .ok_or(FanError::BadFaninIndex { node, index })
    }

    pub fn get_fanout(&self, node: NodeId, index: usize) -> Result<FanoutRecord, FanError> {
        self.node(node)?
            .fanouts
            .get(index)
            .copied()
            .ok_or(FanError::BadFanoutIndex { node, index })
    }

    pub fn get_fanin_index(&self, node: NodeId, fanin: NodeId) -> Result<Option<usize>, FanError> {
        Ok(self
            .node(node)?
            .fanins
            .iter()
            .position(|candidate| *candidate == fanin))
    }

    pub fn num_fanin(&self, node: NodeId) -> Result<usize, FanError> {
        Ok(self.node(node)?.fanins.len())
    }

    pub fn num_fanout(&self, node: NodeId) -> Result<usize, FanError> {
        Ok(self.node(node)?.fanouts.len())
    }

    pub fn fanout_iter(
        &self,
        node: NodeId,
    ) -> Result<impl Iterator<Item = FanoutRecord> + '_, FanError> {
        Ok(self.node(node)?.fanouts.iter().copied())
    }

    pub fn patch_fanin(
        &mut self,
        node: NodeId,
        fanin: NodeId,
        new_fanin: NodeId,
    ) -> Result<bool, FanError> {
        let Some(index) = self.get_fanin_index(node, fanin)? else {
            return Ok(false);
        };

        self.patch_fanin_index(node, index, new_fanin)
    }

    pub fn patch_fanin_index(
        &mut self,
        node: NodeId,
        fanin_index: usize,
        new_fanin: NodeId,
    ) -> Result<bool, FanError> {
        self.node(new_fanin)?;
        if fanin_index >= self.node(node)?.fanins.len() {
            return Ok(false);
        }

        self.remove_single_fanout_record(node, fanin_index)?;
        self.nodes[node.0].fanins[fanin_index] = new_fanin;
        self.replace_single_fanout_record(node, fanin_index)?;
        self.nodes[node.0].duplicate_free = false;
        Ok(true)
    }

    pub fn rebuild_fanout_records(&mut self, node: NodeId) -> Result<(), FanError> {
        self.remove_fanout_records(node)?;
        self.add_fanout_records(node)
    }

    fn add_fanout_records(&mut self, node: NodeId) -> Result<(), FanError> {
        let fanin_count = self.node(node)?.fanins.len();
        for index in (0..fanin_count).rev() {
            self.replace_single_fanout_record(node, index)?;
        }

        Ok(())
    }

    fn remove_fanout_records(&mut self, node: NodeId) -> Result<(), FanError> {
        let fanin_count = self.node(node)?.fanins.len();
        for index in (0..fanin_count).rev() {
            self.remove_single_fanout_record(node, index)?;
        }

        Ok(())
    }

    fn replace_single_fanout_record(&mut self, node: NodeId, index: usize) -> Result<(), FanError> {
        let fanin = self.get_fanin(node, index)?;
        let fanin_node = self.node_mut(fanin)?;
        fanin_node.fanouts.push(FanoutRecord {
            fanout: node,
            pin: index,
        });
        Ok(())
    }

    fn remove_single_fanout_record(&mut self, node: NodeId, index: usize) -> Result<(), FanError> {
        let fanin = self.get_fanin(node, index)?;
        let fanouts = &mut self.node_mut(fanin)?.fanouts;
        let Some(position) = fanouts
            .iter()
            .position(|record| record.fanout == node && record.pin == index)
        else {
            return Err(FanError::MissingFanoutRecord {
                fanin,
                fanout: node,
                pin: index,
            });
        };

        fanouts.remove(position);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FanError {
    UnknownNode(NodeId),
    BadFaninIndex {
        node: NodeId,
        index: usize,
    },
    BadFanoutIndex {
        node: NodeId,
        index: usize,
    },
    MissingFanoutRecord {
        fanin: NodeId,
        fanout: NodeId,
        pin: usize,
    },
}

impl fmt::Display for FanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown node {:?}", node),
            Self::BadFaninIndex { node, index } => {
                write!(f, "bad fanin index {index} for node {:?}", node)
            }
            Self::BadFanoutIndex { node, index } => {
                write!(f, "bad fanout index {index} for node {:?}", node)
            }
            Self::MissingFanoutRecord { fanin, fanout, pin } => write!(
                f,
                "missing fanout record from {:?} to {:?} at pin {pin}",
                fanin, fanout
            ),
        }
    }
}

impl Error for FanError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> FanNetwork {
        let mut network = FanNetwork::new();
        let a = network
            .add_node(FanNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_node(FanNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let n = network
            .add_node(FanNode::new("n", NodeKind::Internal).with_fanins(vec![a, b]))
            .unwrap();
        network
            .add_node(FanNode::new("out", NodeKind::PrimaryOutput).with_fanins(vec![n]))
            .unwrap();

        network
    }

    #[test]
    fn add_node_registers_reverse_fanout_records() {
        let network = sample_network();

        assert_eq!(network.num_fanin(NodeId(2)).unwrap(), 2);
        assert_eq!(network.num_fanout(NodeId(0)).unwrap(), 1);
        assert_eq!(
            network.get_fanout(NodeId(0), 0).unwrap(),
            FanoutRecord {
                fanout: NodeId(2),
                pin: 0,
            }
        );
        assert_eq!(
            network.get_fanout(NodeId(1), 0).unwrap(),
            FanoutRecord {
                fanout: NodeId(2),
                pin: 1,
            }
        );
    }

    #[test]
    fn fanout_iterator_returns_fanout_records_in_list_order() {
        let network = sample_network();

        let fanouts: Vec<_> = network.fanout_iter(NodeId(2)).unwrap().collect();

        assert_eq!(
            fanouts,
            vec![FanoutRecord {
                fanout: NodeId(3),
                pin: 0,
            }]
        );
    }

    #[test]
    fn get_fanin_index_reports_first_matching_fanin() {
        let mut network = FanNetwork::new();
        let a = network
            .add_node(FanNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let n = network
            .add_node(FanNode::new("n", NodeKind::Internal).with_fanins(vec![a, a]))
            .unwrap();

        assert_eq!(network.get_fanin_index(n, a).unwrap(), Some(0));
        assert_eq!(network.num_fanout(a).unwrap(), 2);
    }

    #[test]
    fn patch_fanin_moves_reverse_record_to_new_fanin() {
        let mut network = sample_network();
        network.node_mut(NodeId(2)).unwrap().mark_duplicate_free();

        assert!(
            network
                .patch_fanin(NodeId(2), NodeId(0), NodeId(1))
                .unwrap()
        );

        assert_eq!(
            network.node(NodeId(2)).unwrap().fanins(),
            &[NodeId(1), NodeId(1)]
        );
        assert_eq!(network.num_fanout(NodeId(0)).unwrap(), 0);
        assert_eq!(
            network.node(NodeId(1)).unwrap().fanouts(),
            &[
                FanoutRecord {
                    fanout: NodeId(2),
                    pin: 1,
                },
                FanoutRecord {
                    fanout: NodeId(2),
                    pin: 0,
                },
            ]
        );
        assert!(!network.node(NodeId(2)).unwrap().is_duplicate_free());
    }

    #[test]
    fn patch_fanin_returns_false_when_fanin_is_absent() {
        let mut network = sample_network();

        assert!(
            !network
                .patch_fanin(NodeId(2), NodeId(3), NodeId(1))
                .unwrap()
        );
    }

    #[test]
    fn patch_fanin_index_returns_false_for_invalid_index() {
        let mut network = sample_network();

        assert!(!network.patch_fanin_index(NodeId(2), 4, NodeId(1)).unwrap());
    }

    #[test]
    fn patch_fanin_index_rejects_unknown_replacement_node() {
        let mut network = sample_network();

        assert_eq!(
            network.patch_fanin_index(NodeId(2), 0, NodeId(99)),
            Err(FanError::UnknownNode(NodeId(99)))
        );
    }

    #[test]
    fn add_node_rejects_unknown_fanin_without_mutating_network() {
        let mut network = FanNetwork::new();

        assert_eq!(
            network.add_node(FanNode::new("bad", NodeKind::Internal).with_fanins(vec![NodeId(10)])),
            Err(FanError::UnknownNode(NodeId(10)))
        );
        assert_eq!(network.node_count(), 0);
    }

    #[test]
    fn fanin_accessors_report_bad_indices() {
        let network = sample_network();

        assert_eq!(
            network.get_fanin(NodeId(2), 2),
            Err(FanError::BadFaninIndex {
                node: NodeId(2),
                index: 2,
            })
        );
        assert_eq!(
            network.get_fanout(NodeId(0), 1),
            Err(FanError::BadFanoutIndex {
                node: NodeId(0),
                index: 1,
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("fan.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
