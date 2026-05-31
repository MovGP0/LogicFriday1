//! Native network sweeping for the SIS network layer.
//!
//! The sweep pass repeatedly folds constants, buffers, and inverters into their
//! fanouts, optionally removes sequential state that has become irrelevant, then
//! deletes unobserved internal nodes.  The API is intentionally Rust-native and
//! operates on an owned graph model instead of exposing legacy entry points.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SweepNodeId(pub usize);

impl SweepNodeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SweepLatchId(pub usize);

impl SweepLatchId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SweepNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SweepExpr {
    Constant(bool),
    Literal { node: SweepNodeId, phase: bool },
    Not(Box<SweepExpr>),
    And(Vec<SweepExpr>),
    Or(Vec<SweepExpr>),
}

impl SweepExpr {
    pub fn literal(node: SweepNodeId) -> Self {
        Self::Literal { node, phase: true }
    }

    pub fn negate(self) -> Self {
        match self {
            Self::Constant(value) => Self::Constant(!value),
            Self::Literal { node, phase } => Self::Literal {
                node,
                phase: !phase,
            },
            Self::Not(inner) => *inner,
            value => Self::Not(Box::new(value)),
        }
    }

    fn substitute(&mut self, old_node: SweepNodeId, replacement: &SweepExpr) {
        match self {
            Self::Literal { node, phase } if *node == old_node => {
                let mut next = replacement.clone();
                if !*phase {
                    next = next.negate();
                }
                *self = next;
            }
            Self::Literal { .. } | Self::Constant(_) => {}
            Self::Not(inner) => {
                inner.substitute(old_node, replacement);
            }
            Self::And(items) | Self::Or(items) => {
                for item in items {
                    item.substitute(old_node, replacement);
                }
            }
        }
    }

    fn referenced_nodes(&self, nodes: &mut BTreeSet<SweepNodeId>) {
        match self {
            Self::Literal { node, .. } => {
                nodes.insert(*node);
            }
            Self::Constant(_) => {}
            Self::Not(inner) => {
                inner.referenced_nodes(nodes);
            }
            Self::And(items) | Self::Or(items) => {
                for item in items {
                    item.referenced_nodes(nodes);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SweepFunction {
    Zero,
    One,
    Buffer,
    Inverter,
    Logic(SweepExpr),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepNode {
    pub name: String,
    pub kind: SweepNodeKind,
    pub fanins: Vec<SweepNodeId>,
    pub fanouts: BTreeSet<SweepNodeId>,
    pub function: SweepFunction,
}

impl SweepNode {
    pub fn new(name: impl Into<String>, kind: SweepNodeKind, function: SweepFunction) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins: Vec::new(),
            fanouts: BTreeSet::new(),
            function,
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = SweepNodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SweepLatchInitialValue {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepLatch {
    pub input: SweepNodeId,
    pub output: SweepNodeId,
    pub initial_value: SweepLatchInitialValue,
}

impl SweepLatch {
    pub fn new(
        input: SweepNodeId,
        output: SweepNodeId,
        initial_value: SweepLatchInitialValue,
    ) -> Self {
        Self {
            input,
            output,
            initial_value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepReport {
    pub changed: bool,
    pub latch_removed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SweepError {
    MissingNode(SweepNodeId),
    MissingLatch(SweepLatchId),
    DuplicateFanin {
        node: SweepNodeId,
        fanin: SweepNodeId,
    },
    InvalidPrimaryOutput(SweepNodeId),
    InvalidBuffer(SweepNodeId),
    InvalidLatchInput(SweepLatchId),
}

impl fmt::Display for SweepError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(node) => write!(formatter, "missing sweep node {}", node.index()),
            Self::MissingLatch(latch) => write!(formatter, "missing sweep latch {}", latch.index()),
            Self::DuplicateFanin { node, fanin } => write!(
                formatter,
                "sweep node {} references fanin {} more than once",
                node.index(),
                fanin.index()
            ),
            Self::InvalidPrimaryOutput(node) => write!(
                formatter,
                "primary output {} must have exactly one fanin",
                node.index()
            ),
            Self::InvalidBuffer(node) => write!(
                formatter,
                "trivial node {} must have exactly one fanin",
                node.index()
            ),
            Self::InvalidLatchInput(latch) => write!(
                formatter,
                "latch {} input must have exactly one fanin",
                latch.index()
            ),
        }
    }
}

impl Error for SweepError {}

pub type SweepResult<T> = Result<T, SweepError>;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct SweepNetwork {
    nodes: Vec<Option<SweepNode>>,
    order: Vec<SweepNodeId>,
    primary_outputs: Vec<SweepNodeId>,
    latches: Vec<Option<SweepLatch>>,
    dc_network: Option<Box<SweepNetwork>>,
    next_constant_name: usize,
}

impl SweepNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node(&self, node: SweepNodeId) -> SweepResult<&SweepNode> {
        self.nodes
            .get(node.index())
            .and_then(Option::as_ref)
            .ok_or(SweepError::MissingNode(node))
    }

    pub fn node_mut(&mut self, node: SweepNodeId) -> SweepResult<&mut SweepNode> {
        self.nodes
            .get_mut(node.index())
            .and_then(Option::as_mut)
            .ok_or(SweepError::MissingNode(node))
    }

    pub fn nodes(&self) -> impl Iterator<Item = (SweepNodeId, &SweepNode)> {
        self.order.iter().filter_map(|id| {
            self.nodes
                .get(id.index())
                .and_then(Option::as_ref)
                .map(|node| (*id, node))
        })
    }

    pub fn latch(&self, latch: SweepLatchId) -> SweepResult<&SweepLatch> {
        self.latches
            .get(latch.index())
            .and_then(Option::as_ref)
            .ok_or(SweepError::MissingLatch(latch))
    }

    pub fn latches(&self) -> impl Iterator<Item = (SweepLatchId, &SweepLatch)> {
        self.latches
            .iter()
            .enumerate()
            .filter_map(|(index, latch)| latch.as_ref().map(|latch| (SweepLatchId(index), latch)))
    }

    pub fn dc_network(&self) -> Option<&SweepNetwork> {
        self.dc_network.as_deref()
    }

    pub fn set_dc_network(&mut self, dc_network: Option<SweepNetwork>) {
        self.dc_network = dc_network.map(Box::new);
    }

    pub fn add_node(&mut self, mut node: SweepNode) -> SweepResult<SweepNodeId> {
        let id = SweepNodeId(self.nodes.len());
        let mut seen = BTreeSet::new();
        for fanin in &node.fanins {
            self.node(*fanin)?;
            if !seen.insert(*fanin) {
                return Err(SweepError::DuplicateFanin {
                    node: id,
                    fanin: *fanin,
                });
            }
        }

        if node.kind == SweepNodeKind::PrimaryOutput && node.fanins.len() != 1 {
            return Err(SweepError::InvalidPrimaryOutput(id));
        }

        if node.kind == SweepNodeKind::PrimaryOutput {
            self.primary_outputs.push(id);
        }

        let fanins = node.fanins.clone();
        node.fanouts.clear();
        self.nodes.push(Some(node));
        self.order.push(id);

        for fanin in fanins {
            self.node_mut(fanin)?.fanouts.insert(id);
        }

        Ok(id)
    }

    pub fn add_latch(&mut self, latch: SweepLatch) -> SweepResult<SweepLatchId> {
        self.node(latch.input)?;
        self.node(latch.output)?;
        let id = SweepLatchId(self.latches.len());
        self.latches.push(Some(latch));
        Ok(id)
    }

    pub fn sweep(&mut self) -> SweepResult<SweepReport> {
        let report = self.sweep_util(true)?;
        if report.latch_removed {
            self.dc_network = None;
        }

        Ok(report)
    }

    pub fn combinational_sweep(&mut self) -> SweepResult<bool> {
        self.sweep_util(false).map(|report| report.changed)
    }

    fn sweep_util(&mut self, sweep_latches: bool) -> SweepResult<SweepReport> {
        let mut changed = false;
        let mut latch_removed = false;

        loop {
            let mut some_change = false;
            let nodes = self.dfs_order()?;
            for node in nodes {
                if self.sweep_node(node)? {
                    some_change = true;
                    changed = true;
                }
            }

            if sweep_latches {
                let reached = self.reached_from_primary_outputs()?;
                let latches = self.latches().map(|(id, _)| id).collect::<Vec<_>>();
                for latch in latches {
                    if self.sweep_latch(latch, &reached)? {
                        some_change = true;
                        changed = true;
                        latch_removed = true;
                    }
                }
            }

            if !some_change {
                break;
            }
        }

        if let Some(dc_network) = &mut self.dc_network {
            dc_network.sweep()?;
        }

        if sweep_latches {
            while self.merge_redundant_latches()? {
                changed = true;
                latch_removed = true;
            }
        }

        if self.cleanup(sweep_latches)? {
            changed = true;
        }

        Ok(SweepReport {
            changed,
            latch_removed,
        })
    }

    fn sweep_node(&mut self, node: SweepNodeId) -> SweepResult<bool> {
        if self.is_latch_endpoint(node)
        {
            return Ok(false);
        }

        if self.node(node)?.kind == SweepNodeKind::PrimaryOutput {
            let fanin = *self
                .node(node)?
                .fanins
                .first()
                .ok_or(SweepError::InvalidPrimaryOutput(node))?;
            let fanin_node = self.node(fanin)?;
            if fanin_node.kind == SweepNodeKind::Internal
                && !self.is_latch_endpoint(fanin)
                && fanin_node.function == SweepFunction::Buffer
                && fanin_node.fanins.len() == 1
            {
                let source = self.single_fanin(fanin)?;
                self.patch_fanin(node, fanin, source)?;
                return Ok(true);
            }

            return Ok(false);
        }

        let mut changed = false;
        loop {
            let Some(fanin) = self
                .node(node)?
                .fanins
                .iter()
                .copied()
                .find(|fanin| self.is_collapsible(*fanin))
            else {
                break;
            };

            self.collapse_fanin(node, fanin)?;
            changed = true;
        }

        Ok(changed)
    }

    fn sweep_latch(
        &mut self,
        latch: SweepLatchId,
        reached: &BTreeSet<SweepNodeId>,
    ) -> SweepResult<bool> {
        let latch_data = self.latch(latch)?.clone();
        let input_driver = self.single_latch_input(latch, latch_data.input)?;
        let driver_function = self.node(input_driver)?.function.clone();

        if latch_initial_matches_function(latch_data.initial_value, &driver_function) {
            self.patch_all_fanouts(latch_data.output, input_driver)?;
            self.delete_latch_with_nodes(latch, latch_data.input, latch_data.output)?;
            return Ok(true);
        }

        if !reached.contains(&latch_data.output) {
            let zero = self.add_constant(false)?;
            self.patch_all_fanouts(latch_data.output, zero)?;
            self.delete_latch_with_nodes(latch, latch_data.input, latch_data.output)?;
            return Ok(true);
        }

        Ok(false)
    }

    fn merge_redundant_latches(&mut self) -> SweepResult<bool> {
        let latches = self
            .latches()
            .map(|(id, latch)| (id, latch.clone()))
            .collect::<Vec<_>>();

        for (index, (first_id, first)) in latches.iter().enumerate() {
            let first_driver = self.single_latch_input(*first_id, first.input)?;
            for (second_id, second) in latches.iter().skip(index + 1) {
                if self
                    .latches
                    .get(second_id.index())
                    .and_then(Option::as_ref)
                    .is_none()
                {
                    continue;
                }

                let second_driver = self.single_latch_input(*second_id, second.input)?;
                if first_driver == second_driver && first.initial_value == second.initial_value {
                    self.patch_all_fanouts(second.output, first.output)?;
                    self.delete_latch_with_nodes(*second_id, second.input, second.output)?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn collapse_fanin(&mut self, node: SweepNodeId, fanin: SweepNodeId) -> SweepResult<()> {
        let replacement = self.replacement_expr(fanin)?;
        {
            let target = self.node_mut(node)?;
            target.function = match target.function.clone() {
                SweepFunction::Logic(mut expression) => {
                    expression.substitute(fanin, &replacement);
                    SweepFunction::Logic(expression)
                }
                SweepFunction::Buffer => function_from_expr(replacement.clone())?,
                SweepFunction::Inverter => function_from_expr(replacement.clone().negate())?,
                other => other,
            };
        }

        let replacement_nodes = nodes_in_expr(&replacement);
        if replacement_nodes.is_empty() {
            self.remove_fanin(node, fanin)?;
        } else {
            let existing = self.node(node)?.fanins.clone();
            for replacement_node in &replacement_nodes {
                if !existing.contains(replacement_node) {
                    self.add_fanin(node, *replacement_node)?;
                }
            }
            self.remove_fanin(node, fanin)?;
        }

        Ok(())
    }

    fn replacement_expr(&self, node: SweepNodeId) -> SweepResult<SweepExpr> {
        match &self.node(node)?.function {
            SweepFunction::Zero => Ok(SweepExpr::Constant(false)),
            SweepFunction::One => Ok(SweepExpr::Constant(true)),
            SweepFunction::Buffer => Ok(SweepExpr::literal(self.single_fanin(node)?)),
            SweepFunction::Inverter => Ok(SweepExpr::literal(self.single_fanin(node)?).negate()),
            SweepFunction::Logic(_) => Ok(SweepExpr::literal(node)),
        }
    }

    fn is_collapsible(&self, node: SweepNodeId) -> bool {
        self.node(node).is_ok_and(|node_data| {
            node_data.kind == SweepNodeKind::Internal
                && !self.is_latch_endpoint(node)
                && match node_data.function {
                    SweepFunction::Zero | SweepFunction::One => true,
                    SweepFunction::Buffer | SweepFunction::Inverter => node_data.fanins.len() == 1,
                    SweepFunction::Logic(_) => false,
                }
        })
    }

    fn single_fanin(&self, node: SweepNodeId) -> SweepResult<SweepNodeId> {
        let node_data = self.node(node)?;
        if node_data.fanins.len() != 1 {
            return Err(SweepError::InvalidBuffer(node));
        }

        Ok(node_data.fanins[0])
    }

    fn single_latch_input(
        &self,
        latch: SweepLatchId,
        input: SweepNodeId,
    ) -> SweepResult<SweepNodeId> {
        let input_node = self.node(input)?;
        if input_node.fanins.len() != 1 {
            return Err(SweepError::InvalidLatchInput(latch));
        }

        Ok(input_node.fanins[0])
    }

    fn patch_fanin(
        &mut self,
        node: SweepNodeId,
        old_fanin: SweepNodeId,
        new_fanin: SweepNodeId,
    ) -> SweepResult<bool> {
        self.node(old_fanin)?;
        self.node(new_fanin)?;
        let replaced = {
            let target = self.node_mut(node)?;
            let mut replaced = false;
            for fanin in &mut target.fanins {
                if *fanin == old_fanin {
                    *fanin = new_fanin;
                    replaced = true;
                }
            }
            replaced
        };

        if replaced {
            self.node_mut(old_fanin)?.fanouts.remove(&node);
            self.node_mut(new_fanin)?.fanouts.insert(node);
        }

        Ok(replaced)
    }

    fn patch_all_fanouts(
        &mut self,
        old_fanin: SweepNodeId,
        new_fanin: SweepNodeId,
    ) -> SweepResult<()> {
        let fanouts = self
            .node(old_fanin)?
            .fanouts
            .iter()
            .copied()
            .collect::<Vec<_>>();
        for fanout in fanouts {
            self.patch_fanin(fanout, old_fanin, new_fanin)?;
        }

        Ok(())
    }

    fn add_fanin(&mut self, node: SweepNodeId, fanin: SweepNodeId) -> SweepResult<()> {
        self.node(fanin)?;
        let already_present = self.node(node)?.fanins.contains(&fanin);
        if !already_present {
            self.node_mut(node)?.fanins.push(fanin);
            self.node_mut(fanin)?.fanouts.insert(node);
        }

        Ok(())
    }

    fn remove_fanin(&mut self, node: SweepNodeId, fanin: SweepNodeId) -> SweepResult<()> {
        self.node(fanin)?;
        self.node_mut(node)?
            .fanins
            .retain(|candidate| *candidate != fanin);
        self.node_mut(fanin)?.fanouts.remove(&node);
        Ok(())
    }

    fn delete_node(&mut self, node: SweepNodeId) -> SweepResult<SweepNode> {
        self.node(node)?;
        let removed = self.nodes[node.index()]
            .take()
            .ok_or(SweepError::MissingNode(node))?;
        self.order.retain(|candidate| *candidate != node);
        self.primary_outputs.retain(|candidate| *candidate != node);

        for fanin in &removed.fanins {
            if let Some(fanin_node) = self.nodes.get_mut(fanin.index()).and_then(Option::as_mut) {
                fanin_node.fanouts.remove(&node);
            }
        }

        for fanout in &removed.fanouts {
            if let Some(fanout_node) = self.nodes.get_mut(fanout.index()).and_then(Option::as_mut) {
                fanout_node.fanins.retain(|fanin| *fanin != node);
            }
        }

        Ok(removed)
    }

    fn delete_latch_with_nodes(
        &mut self,
        latch: SweepLatchId,
        input: SweepNodeId,
        output: SweepNodeId,
    ) -> SweepResult<()> {
        if self
            .latches
            .get(latch.index())
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(SweepError::MissingLatch(latch));
        }

        self.latches[latch.index()] = None;
        self.delete_node(input)?;
        self.delete_node(output)?;
        Ok(())
    }

    fn cleanup(&mut self, sweep_latches: bool) -> SweepResult<bool> {
        let mut changed = false;
        loop {
            let removable = self.nodes().find_map(|(id, node)| {
                (node.kind == SweepNodeKind::Internal
                    && node.fanouts.is_empty()
                    && (sweep_latches || !self.is_latch_endpoint(id)))
                .then_some(id)
            });

            let Some(node) = removable else {
                break;
            };

            self.delete_node(node)?;
            changed = true;
        }

        Ok(changed)
    }

    fn is_latch_endpoint(&self, node: SweepNodeId) -> bool {
        self.latches()
            .any(|(_, latch)| latch.input == node || latch.output == node)
    }

    fn add_constant(&mut self, value: bool) -> SweepResult<SweepNodeId> {
        let name = format!("const_{}", self.next_constant_name);
        self.next_constant_name += 1;
        self.add_node(SweepNode::new(
            name,
            SweepNodeKind::Internal,
            if value {
                SweepFunction::One
            } else {
                SweepFunction::Zero
            },
        ))
    }

    fn dfs_order(&self) -> SweepResult<Vec<SweepNodeId>> {
        let mut active = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut order = Vec::new();

        for root in &self.primary_outputs {
            self.dfs_recur(*root, &mut active, &mut visited, &mut order)?;
        }

        for (node, node_data) in self.nodes() {
            if node_data.fanouts.is_empty() {
                self.dfs_recur(node, &mut active, &mut visited, &mut order)?;
            }
        }

        Ok(order)
    }

    fn dfs_recur(
        &self,
        node: SweepNodeId,
        active: &mut BTreeSet<SweepNodeId>,
        visited: &mut BTreeSet<SweepNodeId>,
        order: &mut Vec<SweepNodeId>,
    ) -> SweepResult<()> {
        if visited.contains(&node) {
            return Ok(());
        }

        if !active.insert(node) {
            return Ok(());
        }

        for fanin in &self.node(node)?.fanins {
            self.dfs_recur(*fanin, active, visited, order)?;
        }

        active.remove(&node);
        visited.insert(node);
        order.push(node);
        Ok(())
    }

    fn reached_from_primary_outputs(&self) -> SweepResult<BTreeSet<SweepNodeId>> {
        let mut reached = BTreeSet::new();
        for output in &self.primary_outputs {
            self.collect_fanins(*output, &mut reached)?;
        }

        Ok(reached)
    }

    fn collect_fanins(
        &self,
        node: SweepNodeId,
        reached: &mut BTreeSet<SweepNodeId>,
    ) -> SweepResult<()> {
        if !reached.insert(node) {
            return Ok(());
        }

        for fanin in &self.node(node)?.fanins {
            self.collect_fanins(*fanin, reached)?;
        }

        Ok(())
    }
}

fn latch_initial_matches_function(
    initial_value: SweepLatchInitialValue,
    function: &SweepFunction,
) -> bool {
    matches!(
        (initial_value, function),
        (
            SweepLatchInitialValue::Zero | SweepLatchInitialValue::DontCare,
            SweepFunction::Zero
        ) | (
            SweepLatchInitialValue::One | SweepLatchInitialValue::DontCare,
            SweepFunction::One
        )
    )
}

fn function_from_expr(expression: SweepExpr) -> SweepResult<SweepFunction> {
    match expression {
        SweepExpr::Constant(false) => Ok(SweepFunction::Zero),
        SweepExpr::Constant(true) => Ok(SweepFunction::One),
        SweepExpr::Literal { .. } => Ok(SweepFunction::Buffer),
        SweepExpr::Not(inner) if matches!(*inner, SweepExpr::Literal { .. }) => {
            Ok(SweepFunction::Inverter)
        }
        expression => Ok(SweepFunction::Logic(expression)),
    }
}

fn nodes_in_expr(expression: &SweepExpr) -> BTreeSet<SweepNodeId> {
    let mut nodes = BTreeSet::new();
    expression.referenced_nodes(&mut nodes);
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primary_input(name: &str) -> SweepNode {
        SweepNode::new(name, SweepNodeKind::PrimaryInput, SweepFunction::Buffer)
    }

    fn logic(name: &str, fanins: &[SweepNodeId], expression: SweepExpr) -> SweepNode {
        SweepNode::new(
            name,
            SweepNodeKind::Internal,
            SweepFunction::Logic(expression),
        )
        .with_fanins(fanins.iter().copied())
    }

    fn output(name: &str, fanin: SweepNodeId) -> SweepNode {
        SweepNode::new(name, SweepNodeKind::PrimaryOutput, SweepFunction::Buffer)
            .with_fanins([fanin])
    }

    #[test]
    fn primary_output_bypasses_buffer_driver() {
        let mut network = SweepNetwork::new();
        let input = network.add_node(primary_input("a")).unwrap();
        let buffer = network
            .add_node(
                SweepNode::new("buf", SweepNodeKind::Internal, SweepFunction::Buffer)
                    .with_fanins([input]),
            )
            .unwrap();
        let output = network.add_node(output("y", buffer)).unwrap();

        let changed = network.combinational_sweep().unwrap();

        assert!(changed);
        assert_eq!(network.node(output).unwrap().fanins, vec![input]);
        assert!(network.node(input).unwrap().fanouts.contains(&output));
        assert!(matches!(
            network.node(buffer),
            Err(SweepError::MissingNode(_))
        ));
    }

    #[test]
    fn internal_node_collapses_constant_and_inverter_fanins() {
        let mut network = SweepNetwork::new();
        let input = network.add_node(primary_input("a")).unwrap();
        let zero = network
            .add_node(SweepNode::new(
                "z",
                SweepNodeKind::Internal,
                SweepFunction::Zero,
            ))
            .unwrap();
        let inverter = network
            .add_node(
                SweepNode::new("inv", SweepNodeKind::Internal, SweepFunction::Inverter)
                    .with_fanins([input]),
            )
            .unwrap();
        let logic_node = network
            .add_node(logic(
                "n",
                &[zero, inverter],
                SweepExpr::And(vec![SweepExpr::literal(zero), SweepExpr::literal(inverter)]),
            ))
            .unwrap();
        network.add_node(output("y", logic_node)).unwrap();

        let changed = network.combinational_sweep().unwrap();

        assert!(changed);
        assert_eq!(network.node(logic_node).unwrap().fanins, vec![input]);
        assert_eq!(
            network.node(logic_node).unwrap().function,
            SweepFunction::Logic(SweepExpr::And(vec![
                SweepExpr::Constant(false),
                SweepExpr::Literal {
                    node: input,
                    phase: false,
                },
            ]))
        );
    }

    #[test]
    fn combinational_sweep_keeps_latches_and_dc_network() {
        let mut network = SweepNetwork::new();
        let constant = network
            .add_node(SweepNode::new(
                "one",
                SweepNodeKind::Internal,
                SweepFunction::One,
            ))
            .unwrap();
        let latch_input = network
            .add_node(
                SweepNode::new("li", SweepNodeKind::Internal, SweepFunction::Buffer)
                    .with_fanins([constant]),
            )
            .unwrap();
        let latch_output = network
            .add_node(SweepNode::new(
                "lo",
                SweepNodeKind::Internal,
                SweepFunction::Buffer,
            ))
            .unwrap();
        network
            .add_latch(SweepLatch::new(
                latch_input,
                latch_output,
                SweepLatchInitialValue::One,
            ))
            .unwrap();
        network.add_node(output("y", latch_output)).unwrap();
        network.set_dc_network(Some(SweepNetwork::new()));

        let changed = network.combinational_sweep().unwrap();

        assert!(!changed);
        assert!(network.dc_network().is_some());
        assert_eq!(network.latches().count(), 1);
        assert!(network.node(latch_output).is_ok());
    }

    #[test]
    fn latch_with_matching_constant_driver_is_removed_and_discards_dc_network() {
        let mut network = SweepNetwork::new();
        let constant = network
            .add_node(SweepNode::new(
                "one",
                SweepNodeKind::Internal,
                SweepFunction::One,
            ))
            .unwrap();
        let latch_input = network
            .add_node(
                SweepNode::new("li", SweepNodeKind::Internal, SweepFunction::Buffer)
                    .with_fanins([constant]),
            )
            .unwrap();
        let latch_output = network
            .add_node(SweepNode::new(
                "lo",
                SweepNodeKind::Internal,
                SweepFunction::Buffer,
            ))
            .unwrap();
        network
            .add_latch(SweepLatch::new(
                latch_input,
                latch_output,
                SweepLatchInitialValue::One,
            ))
            .unwrap();
        let output = network.add_node(output("y", latch_output)).unwrap();
        network.set_dc_network(Some(SweepNetwork::new()));

        let report = network.sweep().unwrap();

        assert_eq!(
            report,
            SweepReport {
                changed: true,
                latch_removed: true,
            }
        );
        assert!(network.dc_network().is_none());
        assert_eq!(network.node(output).unwrap().fanins, vec![constant]);
        assert_eq!(network.latches().count(), 0);
        assert!(network.node(latch_input).is_err());
        assert!(network.node(latch_output).is_err());
    }

    #[test]
    fn unreachable_latch_output_is_replaced_by_zero() {
        let mut network = SweepNetwork::new();
        let input = network.add_node(primary_input("a")).unwrap();
        let visible = network
            .add_node(logic("n", &[input], SweepExpr::literal(input)))
            .unwrap();
        network.add_node(output("y", visible)).unwrap();
        let latch_input = network
            .add_node(
                SweepNode::new("li", SweepNodeKind::Internal, SweepFunction::Buffer)
                    .with_fanins([input]),
            )
            .unwrap();
        let latch_output = network
            .add_node(SweepNode::new(
                "lo",
                SweepNodeKind::Internal,
                SweepFunction::Buffer,
            ))
            .unwrap();
        network
            .add_latch(SweepLatch::new(
                latch_input,
                latch_output,
                SweepLatchInitialValue::Zero,
            ))
            .unwrap();
        let sink = network
            .add_node(logic(
                "sink",
                &[latch_output],
                SweepExpr::literal(latch_output),
            ))
            .unwrap();

        let report = network.sweep().unwrap();

        assert!(report.latch_removed);
        assert!(network.node(sink).is_err());
        assert_eq!(network.latches().count(), 0);
    }

    #[test]
    fn redundant_latches_share_the_first_output() {
        let mut network = SweepNetwork::new();
        let input = network.add_node(primary_input("a")).unwrap();
        let latch_input_a = network
            .add_node(
                SweepNode::new("li_a", SweepNodeKind::Internal, SweepFunction::Buffer)
                    .with_fanins([input]),
            )
            .unwrap();
        let latch_output_a = network
            .add_node(SweepNode::new(
                "lo_a",
                SweepNodeKind::Internal,
                SweepFunction::Buffer,
            ))
            .unwrap();
        network
            .add_latch(SweepLatch::new(
                latch_input_a,
                latch_output_a,
                SweepLatchInitialValue::DontCare,
            ))
            .unwrap();
        let latch_input_b = network
            .add_node(
                SweepNode::new("li_b", SweepNodeKind::Internal, SweepFunction::Buffer)
                    .with_fanins([input]),
            )
            .unwrap();
        let latch_output_b = network
            .add_node(SweepNode::new(
                "lo_b",
                SweepNodeKind::Internal,
                SweepFunction::Buffer,
            ))
            .unwrap();
        network
            .add_latch(SweepLatch::new(
                latch_input_b,
                latch_output_b,
                SweepLatchInitialValue::DontCare,
            ))
            .unwrap();
        let out_a = network.add_node(output("ya", latch_output_a)).unwrap();
        let out_b = network.add_node(output("yb", latch_output_b)).unwrap();

        let report = network.sweep().unwrap();

        assert!(report.latch_removed);
        assert_eq!(network.latches().count(), 1);
        assert_eq!(network.node(out_a).unwrap().fanins, vec![latch_output_a]);
        assert_eq!(network.node(out_b).unwrap().fanins, vec![latch_output_a]);
        assert!(network.node(latch_output_b).is_err());
    }
}
