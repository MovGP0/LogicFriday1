//! Native Rust model for `LogicSynthesis/sis/seqbdd/prl_util.c`.
//!
//! The original C file contains small network/DC-network helpers, elapsed-time
//! reporting, and BDD array/minterm utilities. This module ports those behaviors
//! onto owned Rust data structures. Direct SIS `network_t`, `node_t`, `array_t`,
//! `st_table`, and BDD-manager integration remains blocked on the native ports
//! reported as missing native SIS ports.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrlNode {
    pub id: NodeId,
    pub name: Option<String>,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub copy: Option<NodeId>,
    pub is_real_pi: bool,
    pub is_real_po: bool,
    pub logic: LogicFunction,
}

impl PrlNode {
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            name: Some(name.into()),
            kind,
            fanins: Vec::new(),
            copy: None,
            is_real_pi: kind == NodeKind::PrimaryInput,
            is_real_po: kind == NodeKind::PrimaryOutput,
            logic: LogicFunction::one(),
        }
    }

    pub fn anonymous(id: NodeId, kind: NodeKind) -> Self {
        Self {
            id,
            name: None,
            kind,
            fanins: Vec::new(),
            copy: None,
            is_real_pi: kind == NodeKind::PrimaryInput,
            is_real_po: kind == NodeKind::PrimaryOutput,
            logic: LogicFunction::one(),
        }
    }

    pub fn with_fanins(mut self, fanins: impl Into<Vec<NodeId>>) -> Self {
        self.fanins = fanins.into();
        self
    }

    pub fn with_copy(mut self, copy: NodeId) -> Self {
        self.copy = Some(copy);
        self
    }

    pub fn with_real_pi(mut self, is_real_pi: bool) -> Self {
        self.is_real_pi = is_real_pi;
        self
    }

    pub fn with_real_po(mut self, is_real_po: bool) -> Self {
        self.is_real_po = is_real_po;
        self
    }

    pub fn with_logic(mut self, logic: LogicFunction) -> Self {
        self.logic = logic;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrlNetwork {
    name: String,
    nodes: Vec<PrlNode>,
    dc_network: Option<Box<PrlNetwork>>,
}

impl PrlNetwork {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            dc_network: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn nodes(&self) -> &[PrlNode] {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> Option<&PrlNode> {
        self.nodes.get(id.0).filter(|node| node.id == id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut PrlNode> {
        self.nodes.get_mut(id.0).filter(|node| node.id == id)
    }

    pub fn add_node(&mut self, mut node: PrlNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.id = id;
        self.nodes.push(node);
        id
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> NodeId {
        self.add_node(PrlNode::new(
            NodeId(self.nodes.len()),
            name,
            NodeKind::PrimaryInput,
        ))
    }

    pub fn add_primary_output(&mut self, name: impl Into<String>, fanin: NodeId) -> NodeId {
        self.add_node(
            PrlNode::new(NodeId(self.nodes.len()), name, NodeKind::PrimaryOutput)
                .with_fanins(vec![fanin]),
        )
    }

    pub fn find_node(&self, name: &str) -> Option<NodeId> {
        self.nodes
            .iter()
            .find(|node| node.name.as_deref() == Some(name))
            .map(|node| node.id)
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryInput)
            .map(|node| node.id)
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryOutput)
            .map(|node| node.id)
    }

    pub fn primary_output_count(&self) -> usize {
        self.primary_outputs().count()
    }

    pub fn primary_output(&self, index: usize) -> Option<NodeId> {
        self.primary_outputs().nth(index)
    }

    pub fn set_dc_network(&mut self, dc_network: PrlNetwork) {
        self.dc_network = Some(Box::new(dc_network));
    }

    pub fn dc_network(&self) -> Option<&PrlNetwork> {
        self.dc_network.as_deref()
    }

    pub fn take_dc_network(&mut self) -> Option<PrlNetwork> {
        self.dc_network.take().map(|network| *network)
    }

    pub fn fanout_count(&self, id: NodeId) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.fanins.contains(&id))
            .count()
    }

    fn delete_nodes(&mut self, removed: &BTreeSet<NodeId>) {
        let mut old_to_new = HashMap::new();
        let mut new_nodes = Vec::new();

        for node in &self.nodes {
            if !removed.contains(&node.id) {
                let new_id = NodeId(new_nodes.len());
                old_to_new.insert(node.id, new_id);
                let mut new_node = node.clone();
                new_node.id = new_id;
                new_nodes.push(new_node);
            }
        }

        for node in &mut new_nodes {
            node.fanins = node
                .fanins
                .iter()
                .filter_map(|fanin| old_to_new.get(fanin).copied())
                .collect();
            node.copy = node.copy.and_then(|copy| old_to_new.get(&copy).copied());
            node.logic.remap_variables(&old_to_new);
        }

        self.nodes = new_nodes;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrlUtilError {
    UnknownNode(NodeId),
    MissingNodeName(NodeId),
    MissingPrimaryOutputFanin(NodeId),
    MissingCopyForPrimaryInput(NodeId),
    EmptyFunction,
    InvalidSingleOutputDcNetwork { outputs: usize },
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for PrlUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown prl_util node {:?}", node),
            Self::MissingNodeName(node) => write!(f, "node {:?} has no name", node),
            Self::MissingPrimaryOutputFanin(node) => {
                write!(f, "primary output {:?} has no fanin", node)
            }
            Self::MissingCopyForPrimaryInput(node) => {
                write!(f, "primary input {:?} has no initialized copy field", node)
            }
            Self::EmptyFunction => write!(f, "cannot extract an edge from the zero function"),
            Self::InvalidSingleOutputDcNetwork { outputs } => write!(
                f,
                "single-output dc network must contain exactly one PO, found {outputs}"
            ),
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by missing native SIS ports")
            }
        }
    }
}

impl Error for PrlUtilError {}

pub fn sis_remove_dc_network() -> Result<(), PrlUtilError> {
    missing_native_ports("Prl_RemoveDcNetwork")
}

pub fn sis_setup_copy_fields() -> Result<(), PrlUtilError> {
    missing_native_ports("Prl_SetupCopyFields")
}

pub fn sis_copy_subnetwork() -> Result<(), PrlUtilError> {
    missing_native_ports("Prl_CopySubnetwork")
}

pub fn sis_store_as_single_output_dc_network() -> Result<(), PrlUtilError> {
    missing_native_ports("Prl_StoreAsSingleOutputDcNetwork")
}

pub fn sis_cleanup_dc_network() -> Result<(), PrlUtilError> {
    missing_native_ports("Prl_CleanupDcNetwork")
}

pub fn sis_get_simple_dc() -> Result<(), PrlUtilError> {
    missing_native_ports("Prl_GetSimpleDc")
}

pub fn sis_get_one_edge() -> Result<(), PrlUtilError> {
    missing_native_ports("Prl_GetOneEdge")
}

pub fn sis_get_pi_to_var_table() -> Result<(), PrlUtilError> {
    missing_native_ports("Prl_GetPiToVarTable")
}

fn missing_native_ports<T>(operation: &'static str) -> Result<T, PrlUtilError> {
    Err(PrlUtilError::MissingNativePorts { operation })
}

pub fn remove_dc_network(network: &mut PrlNetwork) -> Option<PrlNetwork> {
    network.take_dc_network()
}

pub fn setup_copy_fields(
    to_network: &mut PrlNetwork,
    from_network: &mut PrlNetwork,
) -> Result<(), PrlUtilError> {
    for node in &mut from_network.nodes {
        node.copy = None;
    }

    let from_inputs = from_network.primary_inputs().collect::<Vec<_>>();
    for from_input in from_inputs {
        let (from_name, from_is_real_pi) = {
            let node = from_network
                .node(from_input)
                .ok_or(PrlUtilError::UnknownNode(from_input))?;
            (
                node.name
                    .clone()
                    .ok_or(PrlUtilError::MissingNodeName(from_input))?,
                node.is_real_pi,
            )
        };

        let mut copy = None;
        if from_is_real_pi {
            if let Some(to_input) = to_network.find_node(&from_name) {
                let target = to_network
                    .node(to_input)
                    .ok_or(PrlUtilError::UnknownNode(to_input))?;
                if target.is_real_po {
                    copy = Some(
                        *target
                            .fanins
                            .first()
                            .ok_or(PrlUtilError::MissingPrimaryOutputFanin(to_input))?,
                    );
                } else if target.kind == NodeKind::PrimaryInput {
                    copy = Some(to_input);
                }
            }
        }

        let copy = match copy {
            Some(copy) => copy,
            None => {
                let name = disambiguate_name(to_network, &from_name, None);
                to_network.add_primary_input(name)
            }
        };

        from_network
            .node_mut(from_input)
            .ok_or(PrlUtilError::UnknownNode(from_input))?
            .copy = Some(copy);
    }

    Ok(())
}

pub fn disambiguate_name(network: &PrlNetwork, name: &str, node: Option<NodeId>) -> String {
    let mut candidate = name.to_string();
    while let Some(matching_node) = network.find_node(&candidate) {
        if Some(matching_node) == node {
            return candidate;
        }
        candidate.push_str(":dup");
    }
    candidate
}

pub fn copy_subnetwork(
    network: &mut PrlNetwork,
    source_network: &mut PrlNetwork,
    node: NodeId,
) -> Result<NodeId, PrlUtilError> {
    if let Some(copy) = source_network
        .node(node)
        .ok_or(PrlUtilError::UnknownNode(node))?
        .copy
    {
        return Ok(copy);
    }

    let source_node = source_network
        .node(node)
        .ok_or(PrlUtilError::UnknownNode(node))?
        .clone();

    if source_node.kind == NodeKind::PrimaryInput {
        return Err(PrlUtilError::MissingCopyForPrimaryInput(node));
    }

    let mut copied_fanins = Vec::with_capacity(source_node.fanins.len());
    for fanin in &source_node.fanins {
        copied_fanins.push(copy_subnetwork(network, source_network, *fanin)?);
    }

    let copied_name = match &source_node.name {
        Some(name) if network.find_node(name).is_some() => {
            Some(disambiguate_name(network, name, None))
        }
        Some(name) => Some(name.clone()),
        None => None,
    };

    let mut copied_node = source_node;
    copied_node.id = NodeId(usize::MAX);
    copied_node.fanins = copied_fanins;
    copied_node.copy = None;
    copied_node.name = copied_name;
    let copied = network.add_node(copied_node);
    source_network
        .node_mut(node)
        .ok_or(PrlUtilError::UnknownNode(node))?
        .copy = Some(copied);
    Ok(copied)
}

pub fn store_as_single_output_dc_network(
    network: &mut PrlNetwork,
    dc_network: Option<PrlNetwork>,
) -> Result<(), PrlUtilError> {
    let Some(mut dc_network) = dc_network else {
        return Ok(());
    };

    let outputs = dc_network.primary_output_count();
    if outputs != 1 {
        return Err(PrlUtilError::InvalidSingleOutputDcNetwork { outputs });
    }

    remove_dc_network(network);
    let old_po = dc_network
        .primary_output(0)
        .ok_or(PrlUtilError::InvalidSingleOutputDcNetwork { outputs: 0 })?;
    let fanin = *dc_network
        .node(old_po)
        .ok_or(PrlUtilError::UnknownNode(old_po))?
        .fanins
        .first()
        .ok_or(PrlUtilError::MissingPrimaryOutputFanin(old_po))?;
    let fanin_logic = dc_network
        .node(fanin)
        .ok_or(PrlUtilError::UnknownNode(fanin))?
        .logic
        .clone();

    let output_names = network
        .primary_outputs()
        .map(|po| {
            network
                .node(po)
                .ok_or(PrlUtilError::UnknownNode(po))?
                .name
                .clone()
                .ok_or(PrlUtilError::MissingNodeName(po))
        })
        .collect::<Result<Vec<_>, PrlUtilError>>()?;

    for po_name in output_names {
        let literal = PrlNode::new(
            NodeId(dc_network.nodes.len()),
            po_name.clone(),
            NodeKind::Internal,
        )
        .with_fanins(vec![fanin])
        .with_logic(fanin_logic.clone());
        let literal_id = dc_network.add_node(literal);
        dc_network.add_primary_output(po_name, literal_id);
    }

    dc_network.delete_nodes(&BTreeSet::from([old_po]));
    dc_network.set_name(format!("{}.dc", network.name()));
    network.set_dc_network(dc_network);
    Ok(())
}

pub fn cleanup_dc_network(network: &mut PrlNetwork) {
    let Some(mut dc_network) = network.take_dc_network() else {
        return;
    };
    let removed = dc_network
        .primary_inputs()
        .filter(|pi| dc_network.fanout_count(*pi) == 0)
        .collect::<BTreeSet<_>>();
    dc_network.delete_nodes(&removed);
    network.set_dc_network(dc_network);
}

pub fn free_bdd_array(array: Vec<LogicFunction>) {
    drop(array);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrlOptions {
    pub verbose: bool,
    pub method_name: String,
    pub last_time_ms: u64,
    pub total_time_ms: u64,
}

impl PrlOptions {
    pub fn new(method_name: impl Into<String>) -> Self {
        Self {
            verbose: false,
            method_name: method_name.into(),
            last_time_ms: 0,
            total_time_ms: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElapsedTimeReport {
    pub method_name: String,
    pub elapsed: Duration,
    pub total: Duration,
    pub comment: Option<String>,
}

pub fn report_elapsed_time(
    options: &mut PrlOptions,
    new_time_ms: u64,
    comment: Option<&str>,
) -> Option<ElapsedTimeReport> {
    if !options.verbose {
        return None;
    }

    let elapsed_ms = new_time_ms.saturating_sub(options.last_time_ms);
    options.last_time_ms = new_time_ms;
    options.total_time_ms = options.total_time_ms.saturating_add(elapsed_ms);

    Some(ElapsedTimeReport {
        method_name: options.method_name.clone(),
        elapsed: Duration::from_millis(elapsed_ms),
        total: Duration::from_millis(options.total_time_ms),
        comment: comment.map(str::to_string),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    literals: BTreeMap<usize, bool>,
}

impl Cube {
    pub fn one() -> Self {
        Self {
            literals: BTreeMap::new(),
        }
    }

    pub fn literal(variable: usize, polarity: bool) -> Self {
        Self {
            literals: BTreeMap::from([(variable, polarity)]),
        }
    }

    pub fn literals(&self) -> &BTreeMap<usize, bool> {
        &self.literals
    }

    fn and(&self, other: &Self) -> Option<Self> {
        let mut literals = self.literals.clone();
        for (variable, polarity) in &other.literals {
            match literals.get(variable) {
                Some(existing) if existing != polarity => return None,
                _ => {
                    literals.insert(*variable, *polarity);
                }
            }
        }
        Some(Self { literals })
    }

    fn smooth(&self, variables: &BTreeSet<usize>) -> Self {
        Self {
            literals: self
                .literals
                .iter()
                .filter(|(variable, _)| !variables.contains(variable))
                .map(|(variable, polarity)| (*variable, *polarity))
                .collect(),
        }
    }

    fn remap_variables(&mut self, old_to_new: &HashMap<NodeId, NodeId>) {
        self.literals = self
            .literals
            .iter()
            .map(|(variable, polarity)| {
                let remapped = old_to_new
                    .get(&NodeId(*variable))
                    .map_or(*variable, |id| id.0);
                (remapped, *polarity)
            })
            .collect();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicFunction {
    cubes: Vec<Cube>,
}

impl LogicFunction {
    pub fn zero() -> Self {
        Self { cubes: Vec::new() }
    }

    pub fn one() -> Self {
        Self {
            cubes: vec![Cube::one()],
        }
    }

    pub fn literal(variable: usize, polarity: bool) -> Self {
        Self {
            cubes: vec![Cube::literal(variable, polarity)],
        }
    }

    pub fn from_cubes(cubes: impl IntoIterator<Item = Cube>) -> Self {
        Self {
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn is_zero(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn and(&self, other: &Self) -> Self {
        let cubes = self
            .cubes
            .iter()
            .flat_map(|left| other.cubes.iter().filter_map(|right| left.and(right)))
            .collect();
        Self { cubes }
    }

    pub fn smooth(&self, variables: impl IntoIterator<Item = usize>) -> Self {
        let variables = variables.into_iter().collect::<BTreeSet<_>>();
        let mut seen = BTreeSet::new();
        let mut cubes = Vec::new();
        for cube in &self.cubes {
            let smoothed = cube.smooth(&variables);
            if seen.insert(smoothed.literals.clone()) {
                cubes.push(smoothed);
            }
        }
        Self { cubes }
    }

    fn remap_variables(&mut self, old_to_new: &HashMap<NodeId, NodeId>) {
        for cube in &mut self.cubes {
            cube.remap_variables(old_to_new);
        }
    }
}

pub fn array_bdd_and(functions: &[LogicFunction]) -> LogicFunction {
    functions
        .iter()
        .fold(LogicFunction::one(), |result, function| {
            result.and(function)
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqInfo {
    pub next_state_dc: Vec<LogicFunction>,
    pub ext_output_dc: Vec<LogicFunction>,
    pub present_state_vars: Vec<usize>,
    pub ext_input_vars: Vec<usize>,
    pub input_nodes: Vec<NodeId>,
    pub input_vars: Vec<usize>,
}

impl SeqInfo {
    pub fn new() -> Self {
        Self {
            next_state_dc: Vec::new(),
            ext_output_dc: Vec::new(),
            present_state_vars: Vec::new(),
            ext_input_vars: Vec::new(),
            input_nodes: Vec::new(),
            input_vars: Vec::new(),
        }
    }
}

impl Default for SeqInfo {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_simple_dc(seq_info: &SeqInfo) -> LogicFunction {
    array_bdd_and(&seq_info.next_state_dc).and(&array_bdd_and(&seq_info.ext_output_dc))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OneEdge {
    pub state: LogicFunction,
    pub input: LogicFunction,
}

pub fn get_one_edge(from: &LogicFunction, seq_info: &SeqInfo) -> Result<OneEdge, PrlUtilError> {
    let vars = seq_info
        .present_state_vars
        .iter()
        .chain(seq_info.ext_input_vars.iter())
        .copied()
        .collect::<BTreeSet<_>>();
    let minterm = get_one_minterm(from, &vars)?;
    Ok(OneEdge {
        state: minterm.smooth(seq_info.ext_input_vars.iter().copied()),
        input: minterm.smooth(seq_info.present_state_vars.iter().copied()),
    })
}

pub fn get_one_minterm(
    function: &LogicFunction,
    variables: &BTreeSet<usize>,
) -> Result<LogicFunction, PrlUtilError> {
    let cube = function.cubes.first().ok_or(PrlUtilError::EmptyFunction)?;
    let mut minterm = LogicFunction::one();
    for variable in variables {
        let polarity = cube.literals.get(variable).copied().unwrap_or(false);
        minterm = minterm.and(&LogicFunction::literal(*variable, polarity));
    }
    Ok(minterm)
}

pub fn get_pi_to_var_table(seq_info: &SeqInfo) -> HashMap<NodeId, usize> {
    seq_info
        .input_nodes
        .iter()
        .copied()
        .zip(seq_info.input_vars.iter().copied())
        .collect()
}

pub fn store_var_ids_in_table(vars: impl IntoIterator<Item = usize>) -> BTreeSet<usize> {
    vars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disambiguate_name_reuses_matching_node_or_appends_dup_until_unique() {
        let mut network = PrlNetwork::new("n");
        let a = network.add_primary_input("a");
        network.add_primary_input("a:dup");

        assert_eq!(disambiguate_name(&network, "a", Some(a)), "a");
        assert_eq!(disambiguate_name(&network, "a", None), "a:dup:dup");
        assert_eq!(disambiguate_name(&network, "b", None), "b");
    }

    #[test]
    fn setup_copy_fields_matches_real_pi_to_target_pi_or_po_fanin() {
        let mut target = PrlNetwork::new("target");
        let a = target.add_primary_input("a");
        let b_driver = target.add_node(PrlNode::new(NodeId(0), "b_driver", NodeKind::Internal));
        target.add_primary_output("b", b_driver);

        let mut source = PrlNetwork::new("source");
        let source_a = source.add_primary_input("a");
        let source_b = source.add_primary_input("b");
        let source_c = source.add_primary_input("c");

        setup_copy_fields(&mut target, &mut source).unwrap();

        assert_eq!(source.node(source_a).unwrap().copy, Some(a));
        assert_eq!(source.node(source_b).unwrap().copy, Some(b_driver));
        let c_copy = source.node(source_c).unwrap().copy.unwrap();
        assert_eq!(target.node(c_copy).unwrap().name.as_deref(), Some("c"));
        assert_eq!(target.node(c_copy).unwrap().kind, NodeKind::PrimaryInput);
    }

    #[test]
    fn copy_subnetwork_recursively_duplicates_fanin_cone_and_disambiguates_names() {
        let mut target = PrlNetwork::new("target");
        let target_a = target.add_primary_input("a");
        target.add_node(PrlNode::new(NodeId(0), "n", NodeKind::Internal));

        let mut source = PrlNetwork::new("source");
        let source_a = source
            .add_node(PrlNode::new(NodeId(0), "a", NodeKind::PrimaryInput).with_copy(target_a));
        let source_n = source
            .add_node(PrlNode::new(NodeId(0), "n", NodeKind::Internal).with_fanins(vec![source_a]));

        let copied = copy_subnetwork(&mut target, &mut source, source_n).unwrap();

        let copied_node = target.node(copied).unwrap();
        assert_eq!(copied_node.name.as_deref(), Some("n:dup"));
        assert_eq!(copied_node.fanins, vec![target_a]);
        assert_eq!(source.node(source_n).unwrap().copy, Some(copied));
    }

    #[test]
    fn store_single_output_dc_network_replaces_old_dc_with_one_output_per_network_output() {
        let mut network = PrlNetwork::new("main");
        let a = network.add_primary_input("a");
        network.add_primary_output("out0", a);
        network.add_primary_output("out1", a);
        network.set_dc_network(PrlNetwork::new("old"));

        let mut dc = PrlNetwork::new("candidate");
        let dc_a = dc.add_primary_input("a");
        let condition = dc.add_node(
            PrlNode::new(NodeId(0), "cond", NodeKind::Internal)
                .with_fanins(vec![dc_a])
                .with_logic(LogicFunction::literal(0, true)),
        );
        dc.add_primary_output("old_dc_po", condition);

        store_as_single_output_dc_network(&mut network, Some(dc)).unwrap();

        let dc = network.dc_network().unwrap();
        assert_eq!(dc.name(), "main.dc");
        let output_names = dc
            .primary_outputs()
            .map(|po| dc.node(po).unwrap().name.as_deref().unwrap().to_string())
            .collect::<Vec<_>>();
        assert_eq!(output_names, vec!["out0", "out1"]);
    }

    #[test]
    fn cleanup_dc_network_removes_unconnected_primary_inputs() {
        let mut network = PrlNetwork::new("main");
        let mut dc = PrlNetwork::new("dc");
        let used = dc.add_primary_input("used");
        dc.add_primary_input("unused");
        dc.add_primary_output("out", used);
        network.set_dc_network(dc);

        cleanup_dc_network(&mut network);

        let dc = network.dc_network().unwrap();
        let inputs = dc
            .primary_inputs()
            .map(|pi| dc.node(pi).unwrap().name.as_deref().unwrap().to_string())
            .collect::<Vec<_>>();
        assert_eq!(inputs, vec!["used"]);
    }

    #[test]
    fn simple_dc_ands_next_state_and_ext_output_dc_arrays() {
        let seq_info = SeqInfo {
            next_state_dc: vec![
                LogicFunction::literal(0, true),
                LogicFunction::literal(1, false),
            ],
            ext_output_dc: vec![LogicFunction::literal(2, true)],
            ..SeqInfo::new()
        };

        let simple = get_simple_dc(&seq_info);

        assert_eq!(
            simple,
            LogicFunction::from_cubes([Cube {
                literals: BTreeMap::from([(0, true), (1, false), (2, true)])
            }])
        );
    }

    #[test]
    fn get_one_edge_selects_false_for_dont_care_vars_and_smooths_state_and_input() {
        let from = LogicFunction::from_cubes([Cube {
            literals: BTreeMap::from([(0, true), (2, true)]),
        }]);
        let seq_info = SeqInfo {
            present_state_vars: vec![0, 1],
            ext_input_vars: vec![2],
            ..SeqInfo::new()
        };

        let edge = get_one_edge(&from, &seq_info).unwrap();

        assert_eq!(
            edge.state,
            LogicFunction::from_cubes([Cube {
                literals: BTreeMap::from([(0, true), (1, false)])
            }])
        );
        assert_eq!(
            edge.input,
            LogicFunction::from_cubes([Cube {
                literals: BTreeMap::from([(2, true)])
            }])
        );
    }

    #[test]
    fn pi_to_var_table_zips_input_nodes_to_input_vars() {
        let seq_info = SeqInfo {
            input_nodes: vec![NodeId(10), NodeId(11)],
            input_vars: vec![3, 4],
            ..SeqInfo::new()
        };

        assert_eq!(
            get_pi_to_var_table(&seq_info),
            HashMap::from([(NodeId(10), 3), (NodeId(11), 4)])
        );
    }

    #[test]
    fn elapsed_time_report_updates_last_and_total_time_only_when_verbose() {
        let mut options = PrlOptions::new("product");
        assert_eq!(report_elapsed_time(&mut options, 100, Some("quiet")), None);
        assert_eq!(options.last_time_ms, 0);

        options.verbose = true;
        options.last_time_ms = 100;
        let report = report_elapsed_time(&mut options, 350, Some("done")).unwrap();

        assert_eq!(options.last_time_ms, 350);
        assert_eq!(options.total_time_ms, 250);
        assert_eq!(report.method_name, "product");
        assert_eq!(report.elapsed, Duration::from_millis(250));
        assert_eq!(report.total, Duration::from_millis(250));
        assert_eq!(report.comment.as_deref(), Some("done"));
    }
}
