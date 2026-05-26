//! Native Rust model for `LogicSynthesis/sis/seqbdd/network_info.c`.
//!
//! The C file extracts sequential-network metadata, builds initial-state and
//! equivalence nodes, creates synthetic PIs for next-state outputs, and mutates
//! product networks by renaming/copying nodes. This module ports the owned-data
//! parts of that behavior. Direct integration with SIS `network_t`, `node_t`,
//! latches, `array_t`, `st_table`, and the ordering helpers remains blocked on
//! the dependency beads reported by the SIS-bound entry points.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;

pub const INIT_STATE_OUTPUT_NAME: &str = "initial_state";
pub const EXTERNAL_OUTPUT_NAME: &str = "equiv:output";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub note: &'static str,
}

pub const REQUIRED_NETWORK_INFO_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        note: "legacy array_t allocation, ordering arrays, and node-list ownership",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.230",
        source_file: "LogicSynthesis/sis/latch/latch.c",
        note: "latch traversal, latch endpoint access, and initial values",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.299",
        source_file: "LogicSynthesis/sis/network/net_seq.c",
        note: "network_is_real_pi/network_is_real_po and latch/PIO classification",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "network add/delete/find/change-name/copy-subnetwork operations",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        note: "node_get_fanin and fanin traversal",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "node allocation, constants, literals, boolean construction, and names",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.428",
        source_file: "LogicSynthesis/sis/seqbdd/manual_order.c",
        note: "manual PI-order overrides requested through verif_options_t",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.442",
        source_file: "LogicSynthesis/sis/seqbdd/verif_util.c",
        note: "get_po_ordering/get_pi_ordering support-driven ordering helpers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        note: "legacy st_table name maps used while building product networks",
    },
];

pub fn required_network_info_dependencies() -> &'static [PortDependency] {
    REQUIRED_NETWORK_INFO_DEPENDENCIES
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LogicExpr {
    Const(bool),
    Ref(NodeId),
    Not(Box<LogicExpr>),
    And(Box<LogicExpr>, Box<LogicExpr>),
    Xnor(Box<LogicExpr>, Box<LogicExpr>),
}

impl LogicExpr {
    pub fn literal(node: NodeId, polarity: bool) -> Self {
        if polarity {
            Self::Ref(node)
        } else {
            Self::Not(Box::new(Self::Ref(node)))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub logic: LogicExpr,
    pub is_real_pi: bool,
    pub is_real_po: bool,
}

impl NetworkNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        let is_real_pi = kind == NodeKind::PrimaryInput;
        let is_real_po = kind == NodeKind::PrimaryOutput;
        Self {
            id: NodeId(usize::MAX),
            name: name.into(),
            kind,
            fanins: Vec::new(),
            logic: LogicExpr::Const(true),
            is_real_pi,
            is_real_po,
        }
    }

    pub fn with_fanins(mut self, fanins: impl Into<Vec<NodeId>>) -> Self {
        self.fanins = fanins.into();
        self
    }

    pub fn with_logic(mut self, logic: LogicExpr) -> Self {
        self.logic = logic;
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchInitialValue {
    Zero,
    One,
    DontCare,
}

impl LatchInitialValue {
    pub fn from_sis_value(value: i32) -> Result<Self, NetworkInfoError> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::DontCare),
            other => Err(NetworkInfoError::InvalidLatchInitialValue(other)),
        }
    }

    fn literal_value(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            Self::DontCare => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Latch {
    pub input: NodeId,
    pub output: NodeId,
    pub initial_value: LatchInitialValue,
}

impl Latch {
    pub fn new(input: NodeId, output: NodeId, initial_value: LatchInitialValue) -> Self {
        Self {
            input,
            output,
            initial_value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Network {
    nodes: Vec<NetworkNode>,
    latches: Vec<Latch>,
    dc_network: Option<Box<Network>>,
}

impl Network {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            latches: Vec::new(),
            dc_network: None,
        }
    }

    pub fn nodes(&self) -> &[NetworkNode] {
        &self.nodes
    }

    pub fn latches(&self) -> &[Latch] {
        &self.latches
    }

    pub fn node(&self, id: NodeId) -> Result<&NetworkNode, NetworkInfoError> {
        self.nodes
            .get(id.0)
            .filter(|node| node.id == id)
            .ok_or(NetworkInfoError::UnknownNode(id))
    }

    pub fn add_node(&mut self, mut node: NetworkNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.id = id;
        self.nodes.push(node);
        id
    }

    pub fn add_primary_input(&mut self, name: impl Into<String>) -> NodeId {
        self.add_node(NetworkNode::new(name, NodeKind::PrimaryInput))
    }

    pub fn add_internal(&mut self, name: impl Into<String>, logic: LogicExpr) -> NodeId {
        self.add_node(NetworkNode::new(name, NodeKind::Internal).with_logic(logic))
    }

    pub fn add_primary_output(&mut self, name: impl Into<String>, fanin: NodeId) -> NodeId {
        self.add_node(
            NetworkNode::new(name, NodeKind::PrimaryOutput)
                .with_fanins([fanin])
                .with_logic(LogicExpr::Ref(fanin)),
        )
    }

    pub fn add_latch(&mut self, input: NodeId, output: NodeId, initial_value: LatchInitialValue) {
        self.latches.push(Latch::new(input, output, initial_value));
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

    pub fn find_node(&self, name: &str) -> Option<NodeId> {
        self.nodes
            .iter()
            .find(|node| node.name == name)
            .map(|node| node.id)
    }

    pub fn set_dc_network(&mut self, dc_network: Network) {
        self.dc_network = Some(Box::new(dc_network));
    }

    pub fn dc_network(&self) -> Option<&Network> {
        self.dc_network.as_deref()
    }

    fn node_mut(&mut self, id: NodeId) -> Result<&mut NetworkNode, NetworkInfoError> {
        self.nodes
            .get_mut(id.0)
            .filter(|node| node.id == id)
            .ok_or(NetworkInfoError::UnknownNode(id))
    }

    fn change_node_name(
        &mut self,
        id: NodeId,
        name: impl Into<String>,
    ) -> Result<(), NetworkInfoError> {
        self.node_mut(id)?.name = name.into();
        Ok(())
    }

    fn is_latch_output(&self, node: NodeId) -> bool {
        self.latches.iter().any(|latch| latch.output == node)
    }

    fn primary_output_fanin(&self, node: NodeId) -> Result<NodeId, NetworkInfoError> {
        let node_ref = self.node(node)?;
        node_ref
            .fanins
            .first()
            .copied()
            .ok_or(NetworkInfoError::MissingPrimaryOutputFanin(node))
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationMethod {
    Consistency,
    Bull,
    Product,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationOptions {
    pub method: VerificationMethod,
    pub use_manual_order: bool,
    pub verbose: bool,
}

impl VerificationOptions {
    pub fn new(method: VerificationMethod) -> Self {
        Self {
            method,
            use_manual_order: false,
            verbose: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputInfo {
    pub org_pi: Vec<NodeId>,
    pub ext_pi: Vec<NodeId>,
    pub init_node: NodeId,
    pub po_ordering: Vec<NodeId>,
    pub new_pi: Vec<NodeId>,
    pub pi_ordering: BTreeMap<NodeId, usize>,
    pub transition_nodes: Vec<NodeId>,
    pub main_node: Option<NodeId>,
    pub main_nodes: [Option<NodeId>; 2],
    pub name_table: HashMap<String, usize>,
    pub xnor_nodes: Vec<NodeId>,
    pub output_node: Option<NodeId>,
    pub is_product_network: bool,
    pub generate_global_output: bool,
}

impl OutputInfo {
    pub fn new() -> Self {
        Self {
            org_pi: Vec::new(),
            ext_pi: Vec::new(),
            init_node: NodeId(usize::MAX),
            po_ordering: Vec::new(),
            new_pi: Vec::new(),
            pi_ordering: BTreeMap::new(),
            transition_nodes: Vec::new(),
            main_node: None,
            main_nodes: [None, None],
            name_table: HashMap::new(),
            xnor_nodes: Vec::new(),
            output_node: None,
            is_product_network: false,
            generate_global_output: true,
        }
    }
}

impl Default for OutputInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductNetworkPlan {
    pub renamed_primary_outputs: Vec<(String, String)>,
    pub renamed_internal_nodes: Vec<(String, String)>,
    pub copied_latches: Vec<Latch>,
    pub merged_real_outputs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkInfoError {
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    UnknownNode(NodeId),
    MissingNodeName(NodeId),
    MissingPrimaryOutputFanin(NodeId),
    MismatchedEquivalenceInputs {
        left: usize,
        right: usize,
    },
    InvalidLatchInitialValue(i32),
    ProductInputMissingInTarget {
        name: String,
    },
    OutputMissingInProduct {
        name: String,
    },
}

impl fmt::Display for NetworkInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires native Rust ports for {} SIS dependencies",
                dependencies.len()
            ),
            Self::UnknownNode(node) => write!(f, "unknown network_info node {:?}", node),
            Self::MissingNodeName(node) => write!(f, "node {:?} has no name", node),
            Self::MissingPrimaryOutputFanin(node) => {
                write!(f, "primary output {:?} has no fanin", node)
            }
            Self::MismatchedEquivalenceInputs { left, right } => write!(
                f,
                "equivalence node input lists differ in length: {left} vs {right}"
            ),
            Self::InvalidLatchInitialValue(value) => {
                write!(f, "invalid SIS latch initial value {value}")
            }
            Self::ProductInputMissingInTarget { name } => {
                write!(f, "product target network is missing primary input {name}")
            }
            Self::OutputMissingInProduct { name } => {
                write!(f, "product target network is missing copied output {name}")
            }
        }
    }
}

impl Error for NetworkInfoError {}

pub fn extract_network_info_from_sis() -> Result<OutputInfo, NetworkInfoError> {
    missing_native_ports("extract_network_info SIS network_t entry")
}

pub fn extract_product_network_info_from_sis() -> Result<OutputInfo, NetworkInfoError> {
    missing_native_ports("extract_product_network_info SIS network_t entry")
}

pub fn compute_product_network_from_sis() -> Result<ProductNetworkPlan, NetworkInfoError> {
    missing_native_ports("compute_product_network SIS network_t entry")
}

fn missing_native_ports<T>(operation: &'static str) -> Result<T, NetworkInfoError> {
    Err(NetworkInfoError::MissingNativePorts {
        operation,
        dependencies: REQUIRED_NETWORK_INFO_DEPENDENCIES,
    })
}

pub fn extract_network_info(
    network: &mut Network,
    options: &VerificationOptions,
    mut output_info: OutputInfo,
) -> Result<OutputInfo, NetworkInfoError> {
    output_info.org_pi = network_extract_pi(network);
    output_info.ext_pi = network_extract_extern_pi(network)?;
    output_info.init_node = copy_init_state_constraint(network)?;
    let next_state_po = network_extract_next_state_po(network);

    match options.method {
        VerificationMethod::Bull => {
            output_info.po_ordering = next_state_po;
        }
        VerificationMethod::Consistency => {
            output_info.po_ordering = next_state_po;
            output_info.new_pi = create_new_pi(network, &output_info.po_ordering)?;
            if output_info.is_product_network {
                extract_product_pi_info(network, &mut output_info, "consistency:output")?;
            } else {
                let main = build_equivalence_node(
                    network,
                    &output_info.new_pi,
                    &output_info.po_ordering,
                    Some("consistency:output"),
                    None,
                )?;
                output_info.main_node =
                    Some(network.add_primary_output("consistency:output", main));
            }
            output_info.pi_ordering =
                get_native_pi_ordering(network, &output_info.po_ordering, &output_info.new_pi);
        }
        VerificationMethod::Product => {
            output_info.po_ordering = next_state_po;
            output_info.new_pi = create_new_pi(network, &output_info.po_ordering)?;
            let mut transition_nodes = Vec::new();
            let main = build_equivalence_node(
                network,
                &output_info.new_pi,
                &output_info.po_ordering,
                None,
                Some(&mut transition_nodes),
            )?;
            output_info.transition_nodes = transition_nodes;
            output_info.main_node = None;
            output_info.pi_ordering =
                get_native_pi_ordering(network, &output_info.po_ordering, &output_info.new_pi);
            let _ = main;
        }
    }

    if options.use_manual_order {
        return missing_native_ports("get_manual_order");
    }

    Ok(output_info)
}

pub fn network_extract_next_state_po(network: &Network) -> Vec<NodeId> {
    network.latches.iter().map(|latch| latch.input).collect()
}

pub fn network_extract_pi(network: &Network) -> Vec<NodeId> {
    network.primary_inputs().collect()
}

pub fn network_extract_extern_pi(network: &Network) -> Result<Vec<NodeId>, NetworkInfoError> {
    network
        .primary_inputs()
        .filter_map(|input| {
            network
                .node(input)
                .ok()
                .filter(|node| node.is_real_pi)
                .map(|_| Ok(input))
        })
        .collect()
}

pub fn copy_init_state_constraint(network: &mut Network) -> Result<NodeId, NetworkInfoError> {
    let mut init_state = LogicExpr::Const(true);

    for latch in &network.latches {
        let Some(value) = latch.initial_value.literal_value() else {
            continue;
        };
        let literal = LogicExpr::literal(latch.output, value);
        init_state = LogicExpr::And(Box::new(init_state), Box::new(literal));
    }

    let internal = network.add_internal(INIT_STATE_OUTPUT_NAME, init_state);
    Ok(network.add_primary_output(INIT_STATE_OUTPUT_NAME, internal))
}

pub fn create_new_pi(
    network: &mut Network,
    outputs: &[NodeId],
) -> Result<Vec<NodeId>, NetworkInfoError> {
    let mut new_pi = Vec::with_capacity(outputs.len());
    for (index, output) in outputs.iter().enumerate() {
        let output_name = network.node(*output)?.name.clone();
        let name = format!("{output_name}:y{index}");
        new_pi.push(network.add_primary_input(name));
    }
    Ok(new_pi)
}

pub fn build_equivalence_node(
    network: &mut Network,
    nodes1: &[NodeId],
    nodes2: &[NodeId],
    output_name: Option<&str>,
    mut xnor_array: Option<&mut Vec<NodeId>>,
) -> Result<NodeId, NetworkInfoError> {
    if nodes1.len() != nodes2.len() {
        return Err(NetworkInfoError::MismatchedEquivalenceInputs {
            left: nodes1.len(),
            right: nodes2.len(),
        });
    }

    let mut xnor_nodes = Vec::with_capacity(nodes1.len());
    for (left, right) in nodes1.iter().zip(nodes2) {
        let left = equivalence_operand(network, *left)?;
        let right = equivalence_operand(network, *right)?;
        let logic = LogicExpr::Xnor(
            Box::new(LogicExpr::literal(left, true)),
            Box::new(LogicExpr::literal(right, true)),
        );
        let name = format!(
            "xnor:{}:{}",
            network.node(left)?.name,
            network.node(right)?.name
        );
        let xnor = network.add_internal(name, logic);
        xnor_nodes.push(xnor);
    }

    if let Some(array) = xnor_array.as_deref_mut() {
        array.extend(xnor_nodes.iter().copied());
    }

    let logic = xnor_nodes
        .iter()
        .copied()
        .map(|node| LogicExpr::literal(node, true))
        .reduce(|left, right| LogicExpr::And(Box::new(left), Box::new(right)))
        .unwrap_or(LogicExpr::Const(true));

    let name = output_name
        .map(str::to_string)
        .unwrap_or_else(|| format!("equiv:{}", network.nodes.len()));
    Ok(network.add_internal(name, logic))
}

pub fn compute_product_network(
    network1: &mut Network,
    network2: &mut Network,
    output_info: &mut OutputInfo,
) -> Result<ProductNetworkPlan, NetworkInfoError> {
    allocate_and_rename_primary_inputs(network1, network2, ":0")?;
    let renamed_primary_outputs = rename_primary_outputs(network2, ":0")?;
    let renamed_internal_nodes = rename_internal_nodes(network2, ":0")?;
    hack_rename_internal_nodes(network1, network2, ":0")?;
    remember_primary_output_names(network1, output_info, 0)?;
    remember_primary_output_names(network2, output_info, 1)?;
    copy_primary_outputs_by_name(network1, network2)?;
    let merged_real_outputs = merge_io_outputs(network1, network2, output_info)?;

    let copied_latches = network1.latches.clone();
    network2.latches.extend(copied_latches.iter().cloned());

    Ok(ProductNetworkPlan {
        renamed_primary_outputs,
        renamed_internal_nodes,
        copied_latches,
        merged_real_outputs,
    })
}

fn allocate_and_rename_primary_inputs(
    network1: &Network,
    network2: &mut Network,
    postfix: &str,
) -> Result<(), NetworkInfoError> {
    for input in network1.primary_inputs() {
        let input_node = network1.node(input)?;
        if network1.is_latch_output(input) {
            if let Some(existing) = network2.find_node(&input_node.name) {
                let new_name = format!("{}{postfix}", input_node.name);
                network2.change_node_name(existing, &new_name)?;
                if let Some(dc_network) = network2.dc_network.as_deref_mut() {
                    if let Some(dc_existing) = dc_network.find_node(&input_node.name) {
                        dc_network.change_node_name(dc_existing, new_name)?;
                    }
                }
            }
            network2.add_primary_input(input_node.name.clone());
        } else if network2.find_node(&input_node.name).is_none() {
            return Err(NetworkInfoError::ProductInputMissingInTarget {
                name: input_node.name.clone(),
            });
        }
    }

    Ok(())
}

fn rename_primary_outputs(
    network: &mut Network,
    postfix: &str,
) -> Result<Vec<(String, String)>, NetworkInfoError> {
    let outputs = network.primary_outputs().collect::<Vec<_>>();
    let mut renamed = Vec::with_capacity(outputs.len());
    for output in outputs {
        let old_name = network.node(output)?.name.clone();
        let new_name = format!("{old_name}{postfix}");
        network.change_node_name(output, new_name.clone())?;
        renamed.push((old_name, new_name));
    }
    Ok(renamed)
}

fn rename_internal_nodes(
    network: &mut Network,
    postfix: &str,
) -> Result<Vec<(String, String)>, NetworkInfoError> {
    let internals = network
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Internal)
        .map(|node| node.id)
        .collect::<Vec<_>>();
    let mut renamed = Vec::with_capacity(internals.len());
    for node in internals {
        let old_name = network.node(node)?.name.clone();
        let new_name = format!("{old_name}{postfix}");
        network.change_node_name(node, new_name.clone())?;
        renamed.push((old_name, new_name));
    }
    Ok(renamed)
}

fn hack_rename_internal_nodes(
    network1: &mut Network,
    network2: &Network,
    postfix: &str,
) -> Result<(), NetworkInfoError> {
    let conflicts = network1
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Internal && network2.find_node(&node.name).is_some())
        .map(|node| node.id)
        .collect::<Vec<_>>();

    for node in conflicts {
        let new_name = format!("{}{postfix}", network1.node(node)?.name);
        network1.change_node_name(node, new_name)?;
    }

    Ok(())
}

fn copy_primary_outputs_by_name(
    network1: &Network,
    network2: &mut Network,
) -> Result<(), NetworkInfoError> {
    for output in network1.primary_outputs() {
        let output_node = network1.node(output)?;
        if network2.find_node(&output_node.name).is_some() {
            continue;
        }
        let source_fanin = network1.primary_output_fanin(output)?;
        let source_fanin_name = network1.node(source_fanin)?.name.clone();
        let target_fanin = network2.find_node(&source_fanin_name).ok_or_else(|| {
            NetworkInfoError::ProductInputMissingInTarget {
                name: source_fanin_name,
            }
        })?;
        let copied = network2.add_primary_output(output_node.name.clone(), target_fanin);
        network2.node_mut(copied)?.is_real_po = output_node.is_real_po;
    }

    Ok(())
}

fn remember_primary_output_names(
    network: &Network,
    info: &mut OutputInfo,
    value: usize,
) -> Result<(), NetworkInfoError> {
    for output in network.primary_outputs() {
        info.name_table
            .insert(network.node(output)?.name.clone(), value);
    }
    Ok(())
}

fn merge_io_outputs(
    network1: &Network,
    network2: &mut Network,
    output_info: &mut OutputInfo,
) -> Result<Vec<String>, NetworkInfoError> {
    let mut io_fanin1 = Vec::new();
    let mut io_fanin2 = Vec::new();
    let mut merged_names = Vec::new();

    for output in network1.primary_outputs() {
        let output_node = network1.node(output)?;
        if !output_node.is_real_po {
            continue;
        }
        let matching = network2.find_node(&output_node.name).ok_or_else(|| {
            NetworkInfoError::OutputMissingInProduct {
                name: output_node.name.clone(),
            }
        })?;
        let renamed = network2
            .find_node(&format!("{}:0", output_node.name))
            .ok_or_else(|| NetworkInfoError::OutputMissingInProduct {
                name: format!("{}:0", output_node.name),
            })?;
        io_fanin1.push(network2.primary_output_fanin(matching)?);
        io_fanin2.push(network2.primary_output_fanin(renamed)?);
        merged_names.push(output_node.name.clone());
    }

    if io_fanin1.is_empty() {
        return Ok(merged_names);
    }

    output_info.xnor_nodes.clear();
    let output = build_equivalence_node(
        network2,
        &io_fanin1,
        &io_fanin2,
        Some(EXTERNAL_OUTPUT_NAME),
        Some(&mut output_info.xnor_nodes),
    )?;
    if output_info.generate_global_output {
        output_info.output_node = Some(network2.add_primary_output(EXTERNAL_OUTPUT_NAME, output));
    } else {
        output_info.output_node = None;
        for xnor in &output_info.xnor_nodes {
            let name = network2.node(*xnor)?.name.clone();
            network2.add_primary_output(name, *xnor);
        }
    }

    Ok(merged_names)
}

fn extract_product_pi_info(
    network: &mut Network,
    output_info: &mut OutputInfo,
    name: &str,
) -> Result<(), NetworkInfoError> {
    for netid in 0..2 {
        let (local_pi, local_po) = extract_local_pipo(network, output_info, netid)?;
        let node = build_equivalence_node(network, &local_pi, &local_po, None, None)?;
        output_info.main_nodes[netid] = Some(node);
    }
    let left =
        output_info.main_nodes[0].ok_or(NetworkInfoError::UnknownNode(NodeId(usize::MAX)))?;
    let right =
        output_info.main_nodes[1].ok_or(NetworkInfoError::UnknownNode(NodeId(usize::MAX)))?;
    let node = network.add_internal(
        name,
        LogicExpr::And(
            Box::new(LogicExpr::literal(left, true)),
            Box::new(LogicExpr::literal(right, true)),
        ),
    );
    output_info.main_node = Some(network.add_primary_output(name, node));
    Ok(())
}

fn extract_local_pipo(
    network: &Network,
    info: &OutputInfo,
    netid: usize,
) -> Result<(Vec<NodeId>, Vec<NodeId>), NetworkInfoError> {
    let mut local_pi = Vec::new();
    let mut local_po = Vec::new();
    for (index, po) in info.po_ordering.iter().enumerate() {
        let owner = info
            .name_table
            .get(&network.node(*po)?.name)
            .copied()
            .ok_or(NetworkInfoError::MissingNodeName(*po))?;
        if owner != netid {
            continue;
        }
        if index < info.new_pi.len() {
            local_po.push(*po);
            local_pi.push(info.new_pi[index]);
        }
    }
    Ok((local_pi, local_po))
}

fn get_native_pi_ordering(
    network: &Network,
    _po_ordering: &[NodeId],
    new_pi: &[NodeId],
) -> BTreeMap<NodeId, usize> {
    let mut ordered = Vec::new();
    ordered.extend(network.primary_inputs());
    for pi in new_pi {
        if !ordered.contains(pi) {
            ordered.push(*pi);
        }
    }
    ordered
        .into_iter()
        .enumerate()
        .map(|(index, node)| (node, index))
        .collect()
}

fn equivalence_operand(network: &Network, node: NodeId) -> Result<NodeId, NetworkInfoError> {
    let node_ref = network.node(node)?;
    if node_ref.kind == NodeKind::PrimaryOutput {
        network.primary_output_fanin(node)
    } else {
        Ok(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(network: &Network, ids: &[NodeId]) -> Vec<String> {
        ids.iter()
            .map(|id| network.node(*id).unwrap().name.clone())
            .collect()
    }

    #[test]
    fn extracts_all_pi_external_pi_and_next_state_outputs_in_network_order() {
        let mut network = Network::new();
        let a = network.add_primary_input("a");
        let pseudo =
            network.add_node(NetworkNode::new("ps", NodeKind::PrimaryInput).with_real_pi(false));
        let ns = network.add_internal("ns", LogicExpr::Ref(a));
        network.add_latch(ns, pseudo, LatchInitialValue::One);

        assert_eq!(network_extract_pi(&network), vec![a, pseudo]);
        assert_eq!(network_extract_extern_pi(&network).unwrap(), vec![a]);
        assert_eq!(network_extract_next_state_po(&network), vec![ns]);
    }

    #[test]
    fn init_state_constraint_ands_initialized_latch_outputs_and_skips_dont_care() {
        let mut network = Network::new();
        let ps0 = network.add_primary_input("ps0");
        let ps1 = network.add_primary_input("ps1");
        let ps2 = network.add_primary_input("ps2");
        let ns0 = network.add_internal("ns0", LogicExpr::Ref(ps0));
        let ns1 = network.add_internal("ns1", LogicExpr::Ref(ps1));
        let ns2 = network.add_internal("ns2", LogicExpr::Ref(ps2));
        network.add_latch(ns0, ps0, LatchInitialValue::Zero);
        network.add_latch(ns1, ps1, LatchInitialValue::One);
        network.add_latch(ns2, ps2, LatchInitialValue::DontCare);

        let init_po = copy_init_state_constraint(&mut network).unwrap();
        let init_internal = network.primary_output_fanin(init_po).unwrap();

        assert_eq!(network.node(init_po).unwrap().name, INIT_STATE_OUTPUT_NAME);
        assert_eq!(
            network.node(init_internal).unwrap().name,
            INIT_STATE_OUTPUT_NAME
        );
        assert_eq!(
            network.node(init_internal).unwrap().logic,
            LogicExpr::And(
                Box::new(LogicExpr::And(
                    Box::new(LogicExpr::Const(true)),
                    Box::new(LogicExpr::literal(ps0, false))
                )),
                Box::new(LogicExpr::literal(ps1, true))
            )
        );
    }

    #[test]
    fn creates_one_new_pi_per_ordered_output_using_c_name_format() {
        let mut network = Network::new();
        let out0 = network.add_internal("state_a", LogicExpr::Const(true));
        let out1 = network.add_internal("state_b", LogicExpr::Const(true));

        let new_pi = create_new_pi(&mut network, &[out0, out1]).unwrap();

        assert_eq!(names(&network, &new_pi), vec!["state_a:y0", "state_b:y1"]);
        assert!(
            new_pi
                .iter()
                .all(|id| network.node(*id).unwrap().kind == NodeKind::PrimaryInput)
        );
    }

    #[test]
    fn equivalence_node_replaces_po_operands_with_fanins_and_records_xnors() {
        let mut network = Network::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let po = network.add_primary_output("out", b);
        let mut xnors = Vec::new();

        let eq = build_equivalence_node(&mut network, &[a], &[po], Some("eq"), Some(&mut xnors))
            .unwrap();

        assert_eq!(xnors.len(), 1);
        assert_eq!(network.node(eq).unwrap().name, "eq");
        assert_eq!(
            network.node(xnors[0]).unwrap().logic,
            LogicExpr::Xnor(
                Box::new(LogicExpr::literal(a, true)),
                Box::new(LogicExpr::literal(b, true))
            )
        );
    }

    #[test]
    fn extract_network_info_ports_consistency_method_shape() {
        let mut network = Network::new();
        let ps = network
            .add_primary_input("ps")
            .with_real_pi_marker(&mut network, false);
        let ns = network.add_internal("ns", LogicExpr::Ref(ps));
        network.add_latch(ns, ps, LatchInitialValue::One);
        let output_info = OutputInfo::new();

        let info = extract_network_info(
            &mut network,
            &VerificationOptions::new(VerificationMethod::Consistency),
            output_info,
        )
        .unwrap();

        assert_eq!(info.po_ordering, vec![ns]);
        assert_eq!(names(&network, &info.new_pi), vec!["ns:y0"]);
        assert!(info.main_node.is_some());
        assert!(info.pi_ordering.contains_key(&info.new_pi[0]));
    }

    #[test]
    fn product_network_renames_target_outputs_and_merges_real_outputs() {
        let mut network1 = Network::new();
        let a1 = network1.add_primary_input("a");
        let out1 = network1.add_primary_output("out", a1);
        let ps1 = network1.add_primary_input("ps");
        let ns1 = network1.add_internal("ns", LogicExpr::Ref(ps1));
        network1.add_latch(ns1, ps1, LatchInitialValue::Zero);

        let mut network2 = Network::new();
        let a2 = network2.add_primary_input("a");
        let out_fanin = network2.add_internal("out_logic", LogicExpr::Ref(a2));
        network2.add_primary_output("out", out_fanin);
        network2.add_primary_input("ps");
        let mut output_info = OutputInfo::new();

        let plan = compute_product_network(&mut network1, &mut network2, &mut output_info).unwrap();

        assert_eq!(
            plan.renamed_primary_outputs,
            vec![("out".to_string(), "out:0".to_string())]
        );
        assert_eq!(plan.copied_latches, network1.latches().to_vec());
        assert_eq!(plan.merged_real_outputs, vec!["out"]);
        assert!(network2.find_node("ps:0").is_some());
        assert!(network2.find_node("ps").is_some());
        assert!(output_info.output_node.is_some());
        assert_eq!(network1.node(out1).unwrap().name, "out");
    }

    #[test]
    fn sis_bound_operations_report_dependency_beads_and_sources() {
        let error = extract_product_network_info_from_sis().unwrap_err();

        match error {
            NetworkInfoError::MissingNativePorts {
                operation,
                dependencies,
            } => {
                assert_eq!(
                    operation,
                    "extract_product_network_info SIS network_t entry"
                );
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.230"
                        && dependency.source_file == "LogicSynthesis/sis/latch/latch.c"
                }));
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.442"
                        && dependency.source_file == "LogicSynthesis/sis/seqbdd/verif_util.c"
                }));
                assert!(dependencies.iter().any(|dependency| {
                    dependency.bead_id == "LogicFriday1-8j8.2.6.485"
                        && dependency.source_file == "LogicSynthesis/sis/st/st.c"
                }));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("network_info.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }

    trait TestNodeMarker {
        fn with_real_pi_marker(self, network: &mut Network, is_real_pi: bool) -> Self;
    }

    impl TestNodeMarker for NodeId {
        fn with_real_pi_marker(self, network: &mut Network, is_real_pi: bool) -> Self {
            network.node_mut(self).unwrap().is_real_pi = is_real_pi;
            self
        }
    }
}
