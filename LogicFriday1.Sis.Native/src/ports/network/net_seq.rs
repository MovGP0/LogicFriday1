//! Native Rust sequential-network helpers for the SIS network layer.
//!
//! This module ports the behavior of `sis/network/net_seq.c` onto an owned
//! graph model: latch endpoint bookkeeping, latch insertion/removal, PI/PO
//! reconnect/disconnect helpers, STG attachment/checking, control-output
//! classification, fake output naming, and sequential transitive-fanin
//! traversal.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LatchId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoolExpr {
    Constant(bool),
    Literal(NodeId),
    Not(Box<BoolExpr>),
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
    Xor(Vec<BoolExpr>),
}

impl BoolExpr {
    pub fn literal(node: NodeId) -> Self {
        Self::Literal(node)
    }

    pub fn evaluate<F>(&self, value_of: &mut F) -> Result<bool, NetSeqError>
    where
        F: FnMut(NodeId) -> Result<bool, NetSeqError>,
    {
        match self {
            Self::Constant(value) => Ok(*value),
            Self::Literal(node) => value_of(*node),
            Self::Not(expr) => Ok(!expr.evaluate(value_of)?),
            Self::And(terms) => {
                for term in terms {
                    if !term.evaluate(value_of)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Self::Or(terms) => {
                for term in terms {
                    if term.evaluate(value_of)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Self::Xor(terms) => {
                let mut value = false;
                for term in terms {
                    value ^= term.evaluate(value_of)?;
                }
                Ok(value)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequentialNode {
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub fanouts: BTreeSet<NodeId>,
    pub expression: Option<BoolExpr>,
}

impl SequentialNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: BTreeSet::new(),
            expression: None,
        }
    }

    pub fn with_expression(mut self, expression: BoolExpr) -> Self {
        self.expression = Some(expression);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Latch {
    pub input: NodeId,
    pub output: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub name: String,
    pub encoding: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub from_state: String,
    pub to_state: String,
    pub input: String,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateTransitionGraph {
    start_state: String,
    states: BTreeMap<String, State>,
    transitions: Vec<Transition>,
    saved_names: BTreeMap<String, String>,
}

impl StateTransitionGraph {
    pub fn new(start_state: impl Into<String>) -> Self {
        Self {
            start_state: start_state.into(),
            states: BTreeMap::new(),
            transitions: Vec::new(),
            saved_names: BTreeMap::new(),
        }
    }

    pub fn add_state(
        &mut self,
        name: impl Into<String>,
        encoding: Option<impl Into<String>>,
    ) -> &mut Self {
        let name = name.into();
        self.states.insert(
            name.clone(),
            State {
                name,
                encoding: encoding.map(Into::into),
            },
        );
        self
    }

    pub fn add_transition(
        &mut self,
        from_state: impl Into<String>,
        to_state: impl Into<String>,
        input: impl Into<String>,
        output: impl Into<String>,
    ) -> &mut Self {
        self.transitions.push(Transition {
            from_state: from_state.into(),
            to_state: to_state.into(),
            input: input.into(),
            output: output.into(),
        });
        self
    }

    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    pub fn saved_names(&self) -> &BTreeMap<String, String> {
        &self.saved_names
    }

    fn state_encoding(&self, name: &str) -> Result<&str, NetSeqError> {
        self.states
            .get(name)
            .ok_or_else(|| NetSeqError::MissingState(name.to_owned()))?
            .encoding
            .as_deref()
            .ok_or_else(|| NetSeqError::MissingStateEncoding(name.to_owned()))
    }

    fn start_encoding(&self) -> Result<&str, NetSeqError> {
        self.state_encoding(&self.start_state)
    }

    fn save_names_from_network(&mut self, network: &SequentialNetwork) {
        self.saved_names.clear();
        for (node_id, node) in network.live_nodes() {
            self.saved_names
                .insert(node_id.0.to_string(), node.name.clone());
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StgCheckFailure {
    Output {
        node: NodeId,
        expected: bool,
        present_state: String,
        input: String,
    },
    LatchOutput {
        node: NodeId,
        expected: bool,
        present_state: String,
        input: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetSeqError {
    MissingNode(NodeId),
    MissingLatch(LatchId),
    MissingFanin {
        node: NodeId,
        fanin: NodeId,
    },
    WrongNodeKind {
        node: NodeId,
        expected: NodeKind,
    },
    InvalidPrimaryOutput(NodeId),
    AlreadyLatched(NodeId),
    InvalidLatchEndpoints {
        input: NodeId,
        output: NodeId,
    },
    MissingLatchEndpoint(NodeId),
    MissingState(String),
    MissingStateEncoding(String),
    MissingStg,
    InvalidStgEncodingWidth {
        latches: usize,
        bits: usize,
    },
    InvalidTransitionVector {
        vector: &'static str,
        expected: usize,
        actual: usize,
    },
    MissingInputValue(NodeId),
    CombinationalCycle {
        from: NodeId,
        to: NodeId,
    },
    EvaluationCycle(NodeId),
    StgMismatch(StgCheckFailure),
}

impl fmt::Display for NetSeqError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "missing node {}", node.0),
            Self::MissingLatch(latch) => write!(formatter, "missing latch {}", latch.0),
            Self::MissingFanin { node, fanin } => {
                write!(
                    formatter,
                    "node {} references missing fanin {}",
                    node.0, fanin.0
                )
            }
            Self::WrongNodeKind { node, expected } => {
                write!(formatter, "node {} is not a {expected:?}", node.0)
            }
            Self::InvalidPrimaryOutput(node) => {
                write!(formatter, "primary output {} must have one fanin", node.0)
            }
            Self::AlreadyLatched(node) => write!(formatter, "node {} is already latched", node.0),
            Self::InvalidLatchEndpoints { input, output } => write!(
                formatter,
                "cannot create latch between nodes {} and {}",
                input.0, output.0
            ),
            Self::MissingLatchEndpoint(node) => {
                write!(formatter, "node {} is not part of a latch", node.0)
            }
            Self::MissingState(state) => write!(formatter, "missing STG state {state}"),
            Self::MissingStateEncoding(state) => {
                write!(formatter, "STG state {state} is missing its encoding")
            }
            Self::MissingStg => write!(formatter, "network has no STG"),
            Self::InvalidStgEncodingWidth { latches, bits } => write!(
                formatter,
                "network has {latches} latches but the STG start encoding has {bits} bits"
            ),
            Self::InvalidTransitionVector {
                vector,
                expected,
                actual,
            } => write!(
                formatter,
                "transition {vector} vector has {actual} bits; expected {expected}"
            ),
            Self::MissingInputValue(node) => write!(formatter, "missing value for node {}", node.0),
            Self::CombinationalCycle { from, to } => {
                write!(
                    formatter,
                    "connection from {} to {} would create a cycle",
                    from.0, to.0
                )
            }
            Self::EvaluationCycle(node) => {
                write!(formatter, "cycle while evaluating node {}", node.0)
            }
            Self::StgMismatch(failure) => write!(formatter, "STG mismatch: {failure:?}"),
        }
    }
}

impl Error for NetSeqError {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SequentialNetwork {
    nodes: Vec<Option<SequentialNode>>,
    inputs: Vec<NodeId>,
    outputs: Vec<NodeId>,
    latch_order: Vec<LatchId>,
    latches: Vec<Option<Latch>>,
    latch_by_node: BTreeMap<NodeId, LatchId>,
    control_outputs: BTreeSet<NodeId>,
    stg: Option<StateTransitionGraph>,
}

impl SequentialNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> NodeId {
        let node = self.add_node(SequentialNode::new(name, NodeKind::PrimaryInput));
        self.inputs.push(node);
        node
    }

    pub fn add_internal(
        &mut self,
        name: impl Into<String>,
        fanins: impl IntoIterator<Item = NodeId>,
        expression: BoolExpr,
    ) -> Result<NodeId, NetSeqError> {
        let fanins = fanins.into_iter().collect::<Vec<_>>();
        for fanin in &fanins {
            self.node(*fanin)?;
        }

        let mut node = SequentialNode::new(name, NodeKind::Internal).with_expression(expression);
        node.fanins = fanins;
        let node_id = self.add_node(node);
        self.attach_fanins(node_id)?;
        Ok(node_id)
    }

    pub fn add_primary_output(
        &mut self,
        name: impl Into<String>,
        fanin: NodeId,
    ) -> Result<NodeId, NetSeqError> {
        self.node(fanin)?;
        let mut node = SequentialNode::new(name, NodeKind::PrimaryOutput);
        node.fanins = vec![fanin];
        node.expression = Some(BoolExpr::Literal(fanin));
        let node_id = self.add_node(node);
        self.outputs.push(node_id);
        self.attach_fanins(node_id)?;
        Ok(node_id)
    }

    pub fn add_fake_primary_output(&mut self, fanin: NodeId) -> Result<NodeId, NetSeqError> {
        let output = self.add_primary_output(format!(" {}", self.outputs.len()), fanin)?;
        self.swap_names(fanin, output)?;
        Ok(output)
    }

    pub fn add_control_output(
        &mut self,
        name: impl Into<String>,
        fanin: NodeId,
    ) -> Result<NodeId, NetSeqError> {
        let output = self.add_primary_output(name, fanin)?;
        self.control_outputs.insert(output);
        Ok(output)
    }

    pub fn node(&self, node: NodeId) -> Result<&SequentialNode, NetSeqError> {
        self.nodes
            .get(node.0)
            .and_then(Option::as_ref)
            .ok_or(NetSeqError::MissingNode(node))
    }

    pub fn latch(&self, latch: LatchId) -> Result<&Latch, NetSeqError> {
        self.latches
            .get(latch.0)
            .and_then(Option::as_ref)
            .ok_or(NetSeqError::MissingLatch(latch))
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.inputs
            .iter()
            .copied()
            .filter(|node| self.node(*node).is_ok())
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.outputs
            .iter()
            .copied()
            .filter(|node| self.node(*node).is_ok())
    }

    pub fn latch_order(&self) -> &[LatchId] {
        &self.latch_order
    }

    pub fn stg(&self) -> Option<&StateTransitionGraph> {
        self.stg.as_ref()
    }

    pub fn set_stg(&mut self, mut stg: StateTransitionGraph) {
        stg.save_names_from_network(self);
        self.stg = Some(stg);
    }

    pub fn network_latch_end(&self, node: NodeId) -> Result<Option<NodeId>, NetSeqError> {
        let kind = self.node(node)?.kind;
        if kind != NodeKind::PrimaryInput && kind != NodeKind::PrimaryOutput {
            return Ok(None);
        }

        let Some(latch_id) = self.latch_by_node.get(&node).copied() else {
            return Ok(None);
        };
        let latch = self.latch(latch_id)?;

        if latch.input == node {
            Ok(Some(latch.output))
        } else {
            Ok(Some(latch.input))
        }
    }

    pub fn create_latch(&mut self, n1: NodeId, n2: NodeId) -> Result<LatchId, NetSeqError> {
        self.node(n1)?;
        self.node(n2)?;

        if self.node(n1)?.fanouts.contains(&n2) {
            let (po, pi) = self.disconnect(n1, n2)?;
            return self.insert_latch(po, pi);
        }

        if self.node(n1)?.kind == NodeKind::PrimaryOutput
            && self.node(n2)?.kind == NodeKind::PrimaryInput
        {
            if self.network_latch_end(n1)? == Some(n2) {
                return self
                    .latch_by_node
                    .get(&n1)
                    .copied()
                    .ok_or(NetSeqError::MissingLatchEndpoint(n1));
            }
            if self.network_latch_end(n1)?.is_some() {
                return Err(NetSeqError::AlreadyLatched(n1));
            }
            if self.network_latch_end(n2)?.is_some() {
                return Err(NetSeqError::AlreadyLatched(n2));
            }
            return self.insert_latch(n1, n2);
        }

        Err(NetSeqError::InvalidLatchEndpoints {
            input: n1,
            output: n2,
        })
    }

    pub fn delete_latch(&mut self, latch_id: LatchId) -> Result<Latch, NetSeqError> {
        let latch = self
            .latches
            .get_mut(latch_id.0)
            .and_then(Option::take)
            .ok_or(NetSeqError::MissingLatch(latch_id))?;
        self.latch_by_node.remove(&latch.input);
        self.latch_by_node.remove(&latch.output);
        self.latch_order.retain(|entry| *entry != latch_id);
        Ok(latch)
    }

    pub fn disconnect(
        &mut self,
        node1: NodeId,
        node2: NodeId,
    ) -> Result<(NodeId, NodeId), NetSeqError> {
        if !self.node(node1)?.fanouts.contains(&node2) {
            return Err(NetSeqError::MissingFanin {
                node: node2,
                fanin: node1,
            });
        }

        let po = self.add_fake_primary_output(node1)?;
        let pi = self.add_primary_input(format!(" {}", self.inputs.len()));
        self.patch_fanin(node2, node1, pi)?;
        Ok((po, pi))
    }

    pub fn connect(&mut self, node1: NodeId, node2: NodeId) -> Result<(), NetSeqError> {
        self.require_kind(node1, NodeKind::PrimaryOutput)?;
        self.require_kind(node2, NodeKind::PrimaryInput)?;

        let fanin = match self.node(node1)?.fanins.as_slice() {
            [fanin] => *fanin,
            _ => return Err(NetSeqError::InvalidPrimaryOutput(node1)),
        };

        for fanout in self.node(node2)?.fanouts.clone() {
            let tfo = self.transitive_fanout(fanout)?;
            if tfo.contains(&fanin) || fanout == fanin {
                return Err(NetSeqError::CombinationalCycle {
                    from: fanin,
                    to: fanout,
                });
            }
        }

        for fanout in self.node(node2)?.fanouts.clone() {
            self.patch_fanin(fanout, node2, fanin)?;
        }
        self.delete_node(node1)?;
        self.delete_node(node2)?;
        Ok(())
    }

    pub fn stg_check(&self) -> Result<bool, NetSeqError> {
        if self.primary_inputs().count() == 0 {
            return Ok(true);
        }

        let Some(stg) = &self.stg else {
            return Ok(true);
        };

        let start_bits = stg.start_encoding()?.chars().count();
        if self.latch_order.len() != start_bits {
            return Err(NetSeqError::InvalidStgEncodingWidth {
                latches: self.latch_order.len(),
                bits: start_bits,
            });
        }

        let real_inputs = self.real_primary_inputs().collect::<Vec<_>>();
        let real_outputs = self.real_primary_outputs().collect::<Vec<_>>();

        for transition in &stg.transitions {
            let from_encoding = stg.state_encoding(&transition.from_state)?;
            let to_encoding = stg.state_encoding(&transition.to_state)?;
            self.require_vector_width("input", &transition.input, real_inputs.len())?;
            self.require_vector_width("output", &transition.output, real_outputs.len())?;
            self.require_vector_width("present-state", from_encoding, self.latch_order.len())?;
            self.require_vector_width("next-state", to_encoding, self.latch_order.len())?;

            let mut values = BTreeMap::new();
            for (node, bit) in real_inputs.iter().zip(transition.input.chars()) {
                if let Some(value) = bit_value(bit) {
                    values.insert(*node, value);
                }
            }
            for (latch_id, bit) in self.latch_order.iter().zip(from_encoding.chars()) {
                if let Some(value) = bit_value(bit) {
                    values.insert(self.latch(*latch_id)?.output, value);
                }
            }

            for (node, bit) in real_outputs.iter().zip(transition.output.chars()) {
                if let Some(expected) = bit_value(bit) {
                    let actual = self.evaluate_node(*node, &values)?;
                    if actual != expected {
                        return Err(NetSeqError::StgMismatch(StgCheckFailure::Output {
                            node: *node,
                            expected,
                            present_state: from_encoding.to_owned(),
                            input: transition.input.clone(),
                        }));
                    }
                }
            }

            for (latch_id, bit) in self.latch_order.iter().zip(to_encoding.chars()) {
                if let Some(expected) = bit_value(bit) {
                    let node = self.latch(*latch_id)?.input;
                    let actual = self.evaluate_node(node, &values)?;
                    if actual != expected {
                        return Err(NetSeqError::StgMismatch(StgCheckFailure::LatchOutput {
                            node,
                            expected,
                            present_state: from_encoding.to_owned(),
                            input: transition.input.clone(),
                        }));
                    }
                }
            }
        }

        Ok(true)
    }

    pub fn is_real_po(&self, node: NodeId) -> Result<bool, NetSeqError> {
        Ok(self.node(node)?.kind == NodeKind::PrimaryOutput && self.is_real_pio(node)?)
    }

    pub fn is_real_pi(&self, node: NodeId) -> Result<bool, NetSeqError> {
        Ok(self.node(node)?.kind == NodeKind::PrimaryInput && self.is_real_pio(node)?)
    }

    pub fn is_control(&self, node: NodeId) -> Result<bool, NetSeqError> {
        self.node(node)?;
        Ok(self.control_outputs.contains(&node))
    }

    pub fn get_control(&self, control: NodeId) -> Result<Option<NodeId>, NetSeqError> {
        let source = if self.node(control)?.kind == NodeKind::PrimaryOutput {
            self.node(control)?
                .fanins
                .first()
                .copied()
                .ok_or(NetSeqError::InvalidPrimaryOutput(control))?
        } else {
            control
        };

        for fanout in &self.node(source)?.fanouts {
            if self.node(*fanout)?.kind == NodeKind::PrimaryOutput && self.is_control(*fanout)? {
                return Ok(Some(*fanout));
            }
        }

        Ok(None)
    }

    pub fn replace_io_fake_names(&mut self) {
        let mut used = self
            .live_nodes()
            .filter(|(_, node)| !node.name.starts_with(' '))
            .map(|(_, node)| node.name.clone())
            .collect::<BTreeSet<_>>();
        let fake_nodes = self
            .live_nodes()
            .filter(|(_, node)| node.name.starts_with(' '))
            .map(|(node, _)| node)
            .collect::<Vec<_>>();

        for node_id in fake_nodes {
            let mut index = node_id.0;
            let name = loop {
                let candidate = format!("n{index}");
                if used.insert(candidate.clone()) {
                    break candidate;
                }
                index += 1;
            };
            if let Some(node) = self.nodes.get_mut(node_id.0).and_then(Option::as_mut) {
                node.name = name;
            }
        }
    }

    pub fn sequential_tfi_of_real_pos(&self) -> Result<BTreeSet<NodeId>, NetSeqError> {
        let mut visited = BTreeSet::new();
        for po in self.real_primary_outputs() {
            self.sequential_dfs_recur(po, &mut visited)?;
        }
        Ok(visited)
    }

    fn add_node(&mut self, node: SequentialNode) -> NodeId {
        let node_id = NodeId(self.nodes.len());
        self.nodes.push(Some(node));
        node_id
    }

    fn attach_fanins(&mut self, node: NodeId) -> Result<(), NetSeqError> {
        for fanin in self.node(node)?.fanins.clone() {
            self.node(fanin)?;
            self.nodes[fanin.0]
                .as_mut()
                .ok_or(NetSeqError::MissingNode(fanin))?
                .fanouts
                .insert(node);
        }
        Ok(())
    }

    fn insert_latch(&mut self, input: NodeId, output: NodeId) -> Result<LatchId, NetSeqError> {
        self.require_kind(input, NodeKind::PrimaryOutput)?;
        self.require_kind(output, NodeKind::PrimaryInput)?;
        if self.latch_by_node.contains_key(&input) {
            return Err(NetSeqError::AlreadyLatched(input));
        }
        if self.latch_by_node.contains_key(&output) {
            return Err(NetSeqError::AlreadyLatched(output));
        }

        let latch_id = LatchId(self.latches.len());
        self.latches.push(Some(Latch { input, output }));
        self.latch_order.push(latch_id);
        self.latch_by_node.insert(input, latch_id);
        self.latch_by_node.insert(output, latch_id);
        Ok(latch_id)
    }

    fn require_kind(&self, node: NodeId, expected: NodeKind) -> Result<(), NetSeqError> {
        if self.node(node)?.kind == expected {
            Ok(())
        } else {
            Err(NetSeqError::WrongNodeKind { node, expected })
        }
    }

    fn patch_fanin(
        &mut self,
        node: NodeId,
        old_fanin: NodeId,
        new_fanin: NodeId,
    ) -> Result<(), NetSeqError> {
        self.node(new_fanin)?;
        let target = self.nodes[node.0]
            .as_mut()
            .ok_or(NetSeqError::MissingNode(node))?;
        let mut replaced = false;
        for fanin in &mut target.fanins {
            if *fanin == old_fanin {
                *fanin = new_fanin;
                replaced = true;
            }
        }
        if !replaced {
            return Err(NetSeqError::MissingFanin {
                node,
                fanin: old_fanin,
            });
        }

        if let Some(old_node) = self.nodes.get_mut(old_fanin.0).and_then(Option::as_mut) {
            old_node.fanouts.remove(&node);
        }
        self.nodes[new_fanin.0]
            .as_mut()
            .ok_or(NetSeqError::MissingNode(new_fanin))?
            .fanouts
            .insert(node);
        Ok(())
    }

    fn swap_names(&mut self, left: NodeId, right: NodeId) -> Result<(), NetSeqError> {
        self.node(left)?;
        self.node(right)?;
        let left_name = self.nodes[left.0]
            .as_ref()
            .ok_or(NetSeqError::MissingNode(left))?
            .name
            .clone();
        let right_name = self.nodes[right.0]
            .as_ref()
            .ok_or(NetSeqError::MissingNode(right))?
            .name
            .clone();
        self.nodes[left.0]
            .as_mut()
            .ok_or(NetSeqError::MissingNode(left))?
            .name = right_name;
        self.nodes[right.0]
            .as_mut()
            .ok_or(NetSeqError::MissingNode(right))?
            .name = left_name;
        Ok(())
    }

    fn delete_node(&mut self, node: NodeId) -> Result<SequentialNode, NetSeqError> {
        if self.latch_by_node.contains_key(&node) {
            return Err(NetSeqError::AlreadyLatched(node));
        }

        let removed = self
            .nodes
            .get_mut(node.0)
            .and_then(Option::take)
            .ok_or(NetSeqError::MissingNode(node))?;
        for fanin in &removed.fanins {
            if let Some(fanin_node) = self.nodes.get_mut(fanin.0).and_then(Option::as_mut) {
                fanin_node.fanouts.remove(&node);
            }
        }
        for fanout in &removed.fanouts {
            if let Some(fanout_node) = self.nodes.get_mut(fanout.0).and_then(Option::as_mut) {
                fanout_node.fanins.retain(|fanin| *fanin != node);
            }
        }
        self.inputs.retain(|entry| *entry != node);
        self.outputs.retain(|entry| *entry != node);
        self.control_outputs.remove(&node);
        Ok(removed)
    }

    fn transitive_fanout(&self, root: NodeId) -> Result<BTreeSet<NodeId>, NetSeqError> {
        self.node(root)?;
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::from([root]);
        while let Some(node) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            for fanout in &self.node(node)?.fanouts {
                queue.push_back(*fanout);
            }
        }
        Ok(visited)
    }

    fn is_real_pio(&self, node: NodeId) -> Result<bool, NetSeqError> {
        self.node(node)?;
        Ok(!self.latch_by_node.contains_key(&node) && !self.control_outputs.contains(&node))
    }

    fn real_primary_inputs(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.primary_inputs()
            .filter(|node| self.is_real_pi(*node).unwrap_or(false))
    }

    fn real_primary_outputs(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.primary_outputs()
            .filter(|node| self.is_real_po(*node).unwrap_or(false))
    }

    fn require_vector_width(
        &self,
        vector: &'static str,
        value: &str,
        expected: usize,
    ) -> Result<(), NetSeqError> {
        let actual = value.chars().count();
        if actual == expected {
            Ok(())
        } else {
            Err(NetSeqError::InvalidTransitionVector {
                vector,
                expected,
                actual,
            })
        }
    }

    fn evaluate_node(
        &self,
        node: NodeId,
        values: &BTreeMap<NodeId, bool>,
    ) -> Result<bool, NetSeqError> {
        let mut active = BTreeSet::new();
        self.evaluate_node_recur(node, values, &mut active)
    }

    fn evaluate_node_recur(
        &self,
        node: NodeId,
        values: &BTreeMap<NodeId, bool>,
        active: &mut BTreeSet<NodeId>,
    ) -> Result<bool, NetSeqError> {
        if let Some(value) = values.get(&node) {
            return Ok(*value);
        }
        if !active.insert(node) {
            return Err(NetSeqError::EvaluationCycle(node));
        }

        let result = match self.node(node)?.kind {
            NodeKind::PrimaryInput => Err(NetSeqError::MissingInputValue(node)),
            NodeKind::PrimaryOutput => {
                let fanin = self
                    .node(node)?
                    .fanins
                    .first()
                    .copied()
                    .ok_or(NetSeqError::InvalidPrimaryOutput(node))?;
                self.evaluate_node_recur(fanin, values, active)
            }
            NodeKind::Internal => {
                let expr = self
                    .node(node)?
                    .expression
                    .as_ref()
                    .ok_or(NetSeqError::MissingInputValue(node))?;
                expr.evaluate(&mut |input| self.evaluate_node_recur(input, values, active))
            }
        };

        active.remove(&node);
        result
    }

    fn sequential_dfs_recur(
        &self,
        node: NodeId,
        visited: &mut BTreeSet<NodeId>,
    ) -> Result<(), NetSeqError> {
        if !visited.insert(node) {
            return Ok(());
        }
        for fanin in &self.node(node)?.fanins {
            self.sequential_dfs_recur(*fanin, visited)?;
        }
        if self.node(node)?.kind == NodeKind::PrimaryInput {
            if let Some(latch_input) = self.network_latch_end(node)? {
                self.sequential_dfs_recur(latch_input, visited)?;
            }
        }
        Ok(())
    }

    fn live_nodes(&self) -> impl Iterator<Item = (NodeId, &SequentialNode)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| node.as_ref().map(|node| (NodeId(index), node)))
    }
}

fn bit_value(bit: char) -> Option<bool> {
    match bit {
        '0' => Some(false),
        '1' => Some(true),
        '2' | '-' => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_latch_by_splitting_existing_edge() {
        let mut network = SequentialNetwork::new();
        let a = network.add_primary_input("a");
        let y = network
            .add_internal("y", [a], BoolExpr::literal(a))
            .expect("internal");

        let latch = network.create_latch(a, y).expect("latch");
        let stored = network.latch(latch).expect("stored latch");

        assert_eq!(
            network.node(stored.input).unwrap().kind,
            NodeKind::PrimaryOutput
        );
        assert_eq!(
            network.node(stored.output).unwrap().kind,
            NodeKind::PrimaryInput
        );
        assert_eq!(
            network.network_latch_end(stored.input).unwrap(),
            Some(stored.output)
        );
        assert_eq!(
            network.network_latch_end(stored.output).unwrap(),
            Some(stored.input)
        );
        assert!(network.node(y).unwrap().fanins.contains(&stored.output));
    }

    #[test]
    fn preserves_latch_order_and_deletes_table_entries() {
        let mut network = SequentialNetwork::new();
        let state0 = network.add_primary_input("s0");
        let state1 = network.add_primary_input("s1");
        let next0 = network.add_primary_output("n0", state0).unwrap();
        let next1 = network.add_primary_output("n1", state1).unwrap();

        let first = network.create_latch(next0, state0).unwrap();
        let second = network.create_latch(next1, state1).unwrap();

        assert_eq!(network.latch_order(), &[first, second]);
        assert_eq!(network.delete_latch(first).unwrap().input, next0);
        assert_eq!(network.latch_order(), &[second]);
        assert_eq!(network.network_latch_end(next0).unwrap(), None);
    }

    #[test]
    fn reconnect_rejects_combinational_cycle() {
        let mut network = SequentialNetwork::new();
        let a = network.add_primary_input("a");
        let b = network
            .add_internal("b", [a], BoolExpr::literal(a))
            .unwrap();
        let c = network
            .add_internal("c", [b], BoolExpr::literal(b))
            .unwrap();
        let po = network.add_primary_output("po", c).unwrap();
        let pi = network.add_primary_input("pi");
        network.patch_fanin(b, a, pi).unwrap();

        let err = network.connect(po, pi).unwrap_err();

        assert_eq!(err, NetSeqError::CombinationalCycle { from: c, to: b });
    }

    #[test]
    fn connect_replaces_latch_boundary_with_direct_fanin() {
        let mut network = SequentialNetwork::new();
        let a = network.add_primary_input("a");
        let po = network.add_primary_output("po", a).unwrap();
        let pi = network.add_primary_input("pi");
        let use_pi = network
            .add_internal("use_pi", [pi], BoolExpr::literal(pi))
            .unwrap();

        network.connect(po, pi).unwrap();

        assert!(network.node(po).is_err());
        assert!(network.node(pi).is_err());
        assert_eq!(network.node(use_pi).unwrap().fanins, vec![a]);
        assert!(network.node(a).unwrap().fanouts.contains(&use_pi));
    }

    #[test]
    fn stg_check_accepts_matching_transition() {
        let mut network = SequentialNetwork::new();
        let input = network.add_primary_input("i");
        let state = network.add_primary_input("s");
        let next_expr = BoolExpr::Xor(vec![BoolExpr::literal(input), BoolExpr::literal(state)]);
        let next = network
            .add_internal("next", [input, state], next_expr)
            .unwrap();
        let latch_input = network.add_primary_output("next_po", next).unwrap();
        let output = network.add_primary_output("out", state).unwrap();
        network.create_latch(latch_input, state).unwrap();

        let mut stg = StateTransitionGraph::new("S0");
        stg.add_state("S0", Some("0"));
        stg.add_state("S1", Some("1"));
        stg.add_transition("S0", "S1", "1", "0");
        stg.add_transition("S1", "S1", "0", "1");
        network.set_stg(stg);

        assert_eq!(network.stg_check().unwrap(), true);
        assert!(network.is_real_po(output).unwrap());
    }

    #[test]
    fn stg_check_reports_output_mismatch() {
        let mut network = SequentialNetwork::new();
        let state = network.add_primary_input("s");
        let latch_input = network.add_primary_output("next", state).unwrap();
        network.add_primary_output("out", state).unwrap();
        network.create_latch(latch_input, state).unwrap();

        let mut stg = StateTransitionGraph::new("S0");
        stg.add_state("S0", Some("0"));
        stg.add_transition("S0", "S0", "", "1");
        network.set_stg(stg);

        let err = network.stg_check().unwrap_err();

        assert!(matches!(
            err,
            NetSeqError::StgMismatch(StgCheckFailure::Output { expected: true, .. })
        ));
    }

    #[test]
    fn control_output_is_not_real_and_can_be_found() {
        let mut network = SequentialNetwork::new();
        let clock = network.add_primary_input("clock");
        let control = network.add_control_output("clock_po", clock).unwrap();

        assert!(!network.is_real_po(control).unwrap());
        assert!(network.is_control(control).unwrap());
        assert_eq!(network.get_control(clock).unwrap(), Some(control));
    }

    #[test]
    fn sequential_tfi_jumps_back_across_latches() {
        let mut network = SequentialNetwork::new();
        let input = network.add_primary_input("i");
        let state = network.add_primary_input("s");
        let next = network
            .add_internal("next", [input], BoolExpr::literal(input))
            .unwrap();
        let latch_input = network.add_primary_output("next_po", next).unwrap();
        let output = network.add_primary_output("out", state).unwrap();
        network.create_latch(latch_input, state).unwrap();

        let tfi = network.sequential_tfi_of_real_pos().unwrap();

        assert!(tfi.contains(&output));
        assert!(tfi.contains(&state));
        assert!(tfi.contains(&latch_input));
        assert!(tfi.contains(&next));
        assert!(tfi.contains(&input));
    }

    #[test]
    fn fake_names_are_replaced_with_unique_regular_names() {
        let mut network = SequentialNetwork::new();
        let input = network.add_primary_input("a");
        let output = network.add_fake_primary_output(input).unwrap();

        assert!(network.node(input).unwrap().name.starts_with(' '));
        network.replace_io_fake_names();

        assert!(!network.node(input).unwrap().name.starts_with(' '));
        assert_eq!(network.node(output).unwrap().name, "a");
    }
}
