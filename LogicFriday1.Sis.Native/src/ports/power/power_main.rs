//! Native Rust model for `LogicSynthesis/sis/power/power_main.c`.
//!
//! The C file owns the power-estimation command defaults, option parsing,
//! construction of per-node capacitance/probability/delay tables, mapped-delay
//! normalization, and dispatch to the concrete combinational/sequential/
//! pipeline/dynamic engines. This port keeps the table-building behavior on
//! explicit Rust data structures. Direct dispatch from a legacy SIS `network_t`
//! remains gated with bead/source dependency errors until the callee ports and
//! SIS object model are available.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const BDD_MODE: i32 = 100;
pub const SAMPLE_MODE: i32 = 101;

pub const DEFAULT_MAX_INPUT_SIZING: usize = 4;
pub const DEFAULT_PS_MAX_ALLOWED_ERROR: f64 = 0.01;
pub const CAP_IN_LATCH: i32 = 4;
pub const CAP_OUT_LATCH: i32 = 20;
pub fn sis_power_estimate_blocked() -> Result<(), PowerMainError> {
    missing_sis_dependencies(PowerMainOperation::PowerEstimate)
}

pub fn sis_power_main_driver_blocked() -> Result<(), PowerMainError> {
    missing_sis_dependencies(PowerMainOperation::PowerMainDriver)
}

pub fn sis_power_command_line_interface_blocked() -> Result<(), PowerMainError> {
    missing_sis_dependencies(PowerMainOperation::PowerCommandLineInterface)
}

pub fn sis_power_get_node_info_blocked() -> Result<(), PowerMainError> {
    missing_sis_dependencies(PowerMainOperation::PowerGetNodeInfo)
}

fn missing_sis_dependencies(operation: PowerMainOperation) -> Result<(), PowerMainError> {
    Err(PowerMainError::MissingSisDependencies { operation })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerMainOperation {
    PowerCommandLineInterface,
    PowerEstimate,
    PowerMainDriver,
    PowerGetNodeInfo,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EstimationMode {
    Bdd,
    Sample,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Zero,
    Unit,
    General,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CircuitType {
    Combinational,
    Sequential,
    Pipeline,
    Dynamic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentStateProbability {
    Approximation,
    Exact,
    StateLine,
    Uniform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicForm {
    FactoredForm,
    SumOfProducts,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerGlobals {
    pub set_size: usize,
    pub delta: f64,
    pub verbose: i32,
    pub cap_in_latch: i32,
    pub cap_out_latch: i32,
}

impl Default for PowerGlobals {
    fn default() -> Self {
        Self {
            set_size: 1,
            delta: DEFAULT_PS_MAX_ALLOWED_ERROR,
            verbose: 0,
            cap_in_latch: CAP_IN_LATCH,
            cap_out_latch: CAP_OUT_LATCH,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerCommandOptions {
    pub mode: EstimationMode,
    pub delay: DelayModel,
    pub circuit_type: CircuitType,
    pub ps_probability: PresentStateProbability,
    pub num_samples: usize,
    pub info_file: Option<String>,
    pub logic_form: LogicForm,
    pub input_sizing: usize,
    pub sample_gap: Option<usize>,
    pub globals: PowerGlobals,
}

impl Default for PowerCommandOptions {
    fn default() -> Self {
        Self {
            mode: EstimationMode::Bdd,
            delay: DelayModel::Zero,
            circuit_type: CircuitType::Combinational,
            ps_probability: PresentStateProbability::Approximation,
            num_samples: 100,
            info_file: None,
            logic_form: LogicForm::FactoredForm,
            input_sizing: DEFAULT_MAX_INPUT_SIZING,
            sample_gap: None,
            globals: PowerGlobals::default(),
        }
    }
}

impl PowerCommandOptions {
    pub fn parse_argv(
        args: &[impl AsRef<str>],
        latch_count: usize,
    ) -> Result<Self, PowerMainError> {
        let mut options = Self::default();
        let mut index = 1;

        while index < args.len() {
            let arg = args[index].as_ref();
            if !arg.starts_with('-') || arg.len() < 2 {
                return Err(PowerMainError::OptionNotUnderstood(arg.to_owned()));
            }

            let option = arg.as_bytes()[1] as char;
            index += 1;
            match option {
                'm' => {
                    let value = required_value(args, index, "-m")?;
                    options.mode = match first_char(value) {
                        Some('b') | Some('B') => EstimationMode::Bdd,
                        Some('s') | Some('S') => EstimationMode::Sample,
                        _ => return Err(PowerMainError::InvalidMode(value.to_owned())),
                    };
                    index += 1;
                }
                'd' => {
                    let value = required_value(args, index, "-d")?;
                    options.delay = match first_char(value) {
                        Some('z') | Some('Z') => DelayModel::Zero,
                        Some('u') | Some('U') => DelayModel::Unit,
                        Some('g') | Some('G') => DelayModel::General,
                        _ => return Err(PowerMainError::InvalidDelay(value.to_owned())),
                    };
                    index += 1;
                }
                't' => {
                    let value = required_value(args, index, "-t")?;
                    options.circuit_type = match first_char(value) {
                        Some('c') | Some('C') => CircuitType::Combinational,
                        Some('s') | Some('S') if latch_count > 0 => CircuitType::Sequential,
                        Some('s') | Some('S') => CircuitType::Combinational,
                        Some('p') | Some('P') if latch_count > 0 => CircuitType::Pipeline,
                        Some('p') | Some('P') => CircuitType::Combinational,
                        Some('d') | Some('D') => CircuitType::Dynamic,
                        _ => return Err(PowerMainError::InvalidType(value.to_owned())),
                    };
                    index += 1;
                }
                's' => {
                    let value = required_value(args, index, "-s")?;
                    options.ps_probability = match first_char(value) {
                        Some('a') | Some('A') => PresentStateProbability::Approximation,
                        Some('e') | Some('E') if latch_count <= 16 => {
                            PresentStateProbability::Exact
                        }
                        Some('e') | Some('E') => return Err(PowerMainError::TooManyExactStates),
                        Some('u') | Some('U') => PresentStateProbability::Uniform,
                        _ => {
                            return Err(PowerMainError::InvalidPresentStateProbability(
                                value.to_owned(),
                            ));
                        }
                    };
                    index += 1;
                }
                'a' => {
                    let value = required_value(args, index, "-a")?;
                    options.globals.set_size = parse_usize(value, "-a")?;
                    if !(1..=16).contains(&options.globals.set_size) {
                        return Err(PowerMainError::InvalidSetSize(options.globals.set_size));
                    }
                    index += 1;
                }
                'e' => {
                    let value = required_value(args, index, "-e")?;
                    options.globals.delta = parse_f64(value, "-e")?;
                    if options.globals.delta <= 0.0 {
                        return Err(PowerMainError::InvalidDelta(options.globals.delta));
                    }
                    index += 1;
                }
                'n' => {
                    let value = required_value(args, index, "-n")?;
                    options.num_samples = parse_usize(value, "-n")?;
                    index += 1;
                }
                'f' => {
                    options.info_file = Some(required_value(args, index, "-f")?.to_owned());
                    index += 1;
                }
                'h' => return Err(PowerMainError::UsageRequested),
                'R' => {
                    options.globals.cap_in_latch = 0;
                    options.globals.cap_out_latch = 0;
                }
                'M' => {
                    let value = required_value(args, index, "-M")?;
                    options.input_sizing = parse_usize(value, "-M")?;
                    index += 1;
                }
                'S' => options.logic_form = LogicForm::SumOfProducts,
                'N' => {
                    let value = required_value(args, index, "-N")?;
                    options.sample_gap = Some(parse_usize(value, "-N")?);
                    index += 1;
                }
                'V' => {
                    let value = required_value(args, index, "-V")?;
                    options.globals.verbose = parse_i32(value, "-V")?;
                    index += 1;
                }
                _ => return Err(PowerMainError::OptionNotUnderstood(arg.to_owned())),
            }
        }

        Ok(options)
    }
}

fn required_value<'a>(
    args: &'a [impl AsRef<str>],
    index: usize,
    option: &'static str,
) -> Result<&'a str, PowerMainError> {
    args.get(index)
        .map(AsRef::as_ref)
        .ok_or(PowerMainError::MissingOptionValue(option))
}

fn first_char(value: &str) -> Option<char> {
    value.chars().next()
}

fn parse_usize(value: &str, option: &'static str) -> Result<usize, PowerMainError> {
    value
        .parse()
        .map_err(|_| PowerMainError::InvalidNumericValue {
            option,
            value: value.to_owned(),
        })
}

fn parse_i32(value: &str, option: &'static str) -> Result<i32, PowerMainError> {
    value
        .parse()
        .map_err(|_| PowerMainError::InvalidNumericValue {
            option,
            value: value.to_owned(),
        })
}

fn parse_f64(value: &str, option: &'static str) -> Result<f64, PowerMainError> {
    value
        .parse()
        .map_err(|_| PowerMainError::InvalidNumericValue {
            option,
            value: value.to_owned(),
        })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput { real: bool },
    PrimaryOutput { real: bool },
    Internal,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PolarityLiteralCount {
    pub negative: i32,
    pub positive: i32,
}

impl PolarityLiteralCount {
    pub fn total(self) -> i32 {
        self.negative + self.positive
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn max(self) -> f64 {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
    pub sop_literal_counts_by_fanin: Vec<PolarityLiteralCount>,
    pub factored_uses_by_fanin: Vec<i32>,
    pub sop_literal_total: i32,
    pub factored_literal_total: i32,
    pub mapped_arrival: Option<DelayTime>,
}

impl PowerNode {
    pub fn new(id: usize, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind,
            fanins: Vec::new(),
            sop_literal_counts_by_fanin: Vec::new(),
            factored_uses_by_fanin: Vec::new(),
            sop_literal_total: 0,
            factored_literal_total: 0,
            mapped_arrival: None,
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_sop_literal_counts(mut self, counts: Vec<PolarityLiteralCount>) -> Self {
        self.sop_literal_total = counts.iter().map(|count| count.total()).sum();
        self.sop_literal_counts_by_fanin = counts;
        self
    }

    pub fn with_factored_uses(mut self, uses: Vec<i32>) -> Self {
        self.factored_literal_total = uses.iter().sum();
        self.factored_uses_by_fanin = uses;
        self
    }

    pub fn with_literal_totals(mut self, sop_total: i32, factored_total: i32) -> Self {
        self.sop_literal_total = sop_total;
        self.factored_literal_total = factored_total;
        self
    }

    pub fn with_mapped_arrival(mut self, rise: f64, fall: f64) -> Self {
        self.mapped_arrival = Some(DelayTime { rise, fall });
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerNetwork {
    name: String,
    nodes: Vec<PowerNode>,
    positions: HashMap<NodeId, usize>,
}

impl PowerNetwork {
    pub fn new(name: impl Into<String>, nodes: Vec<PowerNode>) -> Result<Self, PowerMainError> {
        let mut positions = HashMap::with_capacity(nodes.len());
        for (position, node) in nodes.iter().enumerate() {
            if positions.insert(node.id, position).is_some() {
                return Err(PowerMainError::DuplicateNode(node.id));
            }
        }

        for node in &nodes {
            if !node.sop_literal_counts_by_fanin.is_empty()
                && node.sop_literal_counts_by_fanin.len() != node.fanins.len()
            {
                return Err(PowerMainError::FaninDataArityMismatch {
                    node_id: node.id,
                    fanins: node.fanins.len(),
                    data: node.sop_literal_counts_by_fanin.len(),
                });
            }
            if !node.factored_uses_by_fanin.is_empty()
                && node.factored_uses_by_fanin.len() != node.fanins.len()
            {
                return Err(PowerMainError::FaninDataArityMismatch {
                    node_id: node.id,
                    fanins: node.fanins.len(),
                    data: node.factored_uses_by_fanin.len(),
                });
            }
            for fanin in &node.fanins {
                if !positions.contains_key(fanin) {
                    return Err(PowerMainError::MissingFanin {
                        node_id: node.id,
                        fanin_id: *fanin,
                    });
                }
            }
        }

        Ok(Self {
            name: name.into(),
            nodes,
            positions,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn nodes(&self) -> &[PowerNode] {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> Result<&PowerNode, PowerMainError> {
        self.positions
            .get(&id)
            .map(|position| &self.nodes[*position])
            .ok_or(PowerMainError::MissingNode(id))
    }

    fn fanouts(&self, id: NodeId) -> impl Iterator<Item = &PowerNode> {
        self.nodes
            .iter()
            .filter(move |node| node.fanins.iter().any(|fanin| *fanin == id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerInfo {
    pub cap_factor: i32,
    pub switching_prob: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeInfo {
    pub delay: i32,
    pub prob_one: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PowerTables {
    pub power_info: HashMap<NodeId, PowerInfo>,
    pub node_info: HashMap<NodeId, NodeInfo>,
}

impl PowerTables {
    pub fn power_info(&self, id: NodeId) -> Result<PowerInfo, PowerMainError> {
        self.power_info
            .get(&id)
            .copied()
            .ok_or(PowerMainError::MissingNode(id))
    }

    pub fn node_info(&self, id: NodeId) -> Result<NodeInfo, PowerMainError> {
        self.node_info
            .get(&id)
            .copied()
            .ok_or(PowerMainError::MissingNode(id))
    }

    pub fn apply_delay_model(
        &mut self,
        network: &PowerNetwork,
        delay: DelayModel,
    ) -> Result<(), PowerMainError> {
        for node in network.nodes() {
            let Some(info) = self.node_info.get_mut(&node.id) else {
                return Err(PowerMainError::MissingNode(node.id));
            };
            if info.delay != -1 {
                continue;
            }
            info.delay = match delay {
                DelayModel::Zero => 0,
                DelayModel::Unit => 1,
                DelayModel::General => power_get_mapped_delay(network, node.id)?,
            };
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerInfoOverride {
    pub node_name: String,
    pub cap_factor: Option<i32>,
    pub delay: Option<i32>,
    pub prob_one: Option<f64>,
}

pub fn parse_power_info_overrides(input: &str) -> Result<Vec<PowerInfoOverride>, PowerMainError> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let mut overrides = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if tokens[index] != "name" {
            return Err(PowerMainError::InvalidInfoFileToken(
                tokens[index].to_owned(),
            ));
        }
        index += 1;
        let Some(name) = tokens.get(index) else {
            return Err(PowerMainError::SuddenEndOfInfoFile);
        };
        index += 1;

        let mut record = PowerInfoOverride {
            node_name: (*name).to_owned(),
            cap_factor: None,
            delay: None,
            prob_one: None,
        };

        while index < tokens.len() && tokens[index] != "name" {
            let key = tokens[index];
            index += 1;
            let Some(value) = tokens.get(index) else {
                return Err(PowerMainError::SuddenEndOfInfoFile);
            };
            match key {
                "cap_factor" => record.cap_factor = Some(parse_info_i32(value, "cap_factor")?),
                "delay" => record.delay = Some(parse_info_i32(value, "delay")?),
                "p0" => record.prob_one = Some(1.0 - parse_info_f64(value, "p0")?),
                "p1" => record.prob_one = Some(parse_info_f64(value, "p1")?),
                _ => return Err(PowerMainError::InvalidInfoFileToken(key.to_owned())),
            }
            index += 1;
        }

        overrides.push(record);
    }

    Ok(overrides)
}

fn parse_info_i32(value: &str, key: &'static str) -> Result<i32, PowerMainError> {
    value
        .parse()
        .map_err(|_| PowerMainError::InvalidInfoFileValue {
            key,
            value: value.to_owned(),
        })
}

fn parse_info_f64(value: &str, key: &'static str) -> Result<f64, PowerMainError> {
    value
        .parse()
        .map_err(|_| PowerMainError::InvalidInfoFileValue {
            key,
            value: value.to_owned(),
        })
}

pub fn power_get_node_info(
    network: &PowerNetwork,
    overrides: &[PowerInfoOverride],
    logic_form: LogicForm,
    input_sizing: usize,
    globals: &PowerGlobals,
) -> Result<PowerTables, PowerMainError> {
    let mut tables = PowerTables::default();

    for record in overrides {
        let Some(node) = network
            .nodes()
            .iter()
            .find(|node| node.name == record.node_name)
        else {
            continue;
        };
        let (mut power_info, mut node_info) =
            default_node_info(network, node, logic_form, input_sizing, globals)?;
        if let Some(cap_factor) = record.cap_factor {
            power_info.cap_factor = cap_factor;
        }
        if let Some(delay) = record.delay {
            node_info.delay = delay;
        }
        if let Some(prob_one) = record.prob_one {
            node_info.prob_one = prob_one;
        } else if matches!(node.kind, NodeKind::PrimaryInput { .. }) {
            node_info.prob_one = 0.0;
        }
        tables.power_info.insert(node.id, power_info);
        tables.node_info.insert(node.id, node_info);
    }

    for node in network.nodes() {
        if tables.power_info.contains_key(&node.id) {
            continue;
        }
        let (power_info, node_info) =
            default_node_info(network, node, logic_form, input_sizing, globals)?;
        tables.power_info.insert(node.id, power_info);
        tables.node_info.insert(node.id, node_info);
    }

    Ok(tables)
}

fn default_node_info(
    network: &PowerNetwork,
    node: &PowerNode,
    logic_form: LogicForm,
    input_sizing: usize,
    globals: &PowerGlobals,
) -> Result<(PowerInfo, NodeInfo), PowerMainError> {
    let (cap_factor, delay, prob_one) = match node.kind {
        NodeKind::PrimaryInput { real } => {
            let base = if real { 0 } else { globals.cap_out_latch };
            (
                base + output_load(network, node.id, logic_form, input_sizing, globals)?,
                -1,
                0.5,
            )
        }
        NodeKind::PrimaryOutput { .. } => (0, -1, 0.0),
        NodeKind::Internal => {
            let internal = match logic_form {
                LogicForm::SumOfProducts => node.sop_literal_total / 2,
                LogicForm::FactoredForm => node.factored_literal_total / 2,
            };
            (
                output_load(network, node.id, logic_form, input_sizing, globals)? + internal,
                -1,
                0.0,
            )
        }
    };

    Ok((
        PowerInfo {
            cap_factor,
            switching_prob: 0.0,
        },
        NodeInfo { delay, prob_one },
    ))
}

fn output_load(
    network: &PowerNetwork,
    node_id: NodeId,
    logic_form: LogicForm,
    input_sizing: usize,
    globals: &PowerGlobals,
) -> Result<i32, PowerMainError> {
    let mut total = 0;

    for fanout in network.fanouts(node_id) {
        match fanout.kind {
            NodeKind::PrimaryOutput { real } if real => {
                total += 0;
            }
            NodeKind::PrimaryOutput { real: false } => {
                total += globals.cap_in_latch;
            }
            _ => {
                let fanin_index = fanout
                    .fanins
                    .iter()
                    .position(|fanin| *fanin == node_id)
                    .ok_or(PowerMainError::MissingFanin {
                        node_id: fanout.id,
                        fanin_id: node_id,
                    })?;
                let sized_inputs = fanout.fanins.len().min(input_sizing) as i32;
                let uses = match logic_form {
                    LogicForm::SumOfProducts => fanout
                        .sop_literal_counts_by_fanin
                        .get(fanin_index)
                        .copied()
                        .unwrap_or_default()
                        .total(),
                    LogicForm::FactoredForm => fanout
                        .factored_uses_by_fanin
                        .get(fanin_index)
                        .copied()
                        .unwrap_or(0),
                };
                total += sized_inputs * uses;
            }
        }
    }

    Ok(total)
}

pub fn power_get_mapped_delay(
    network: &PowerNetwork,
    node_id: NodeId,
) -> Result<i32, PowerMainError> {
    let node = network.node(node_id)?;
    let mut max_fanin_arrival = -1.0;
    for fanin_id in &node.fanins {
        let arrival = network
            .node(*fanin_id)?
            .mapped_arrival
            .ok_or(PowerMainError::MissingMappedArrival(*fanin_id))?;
        max_fanin_arrival = arrival.max().max(max_fanin_arrival);
    }

    let arrival = node
        .mapped_arrival
        .ok_or(PowerMainError::MissingMappedArrival(node_id))?;
    let actual_delay = arrival.max() - max_fanin_arrival;
    Ok(((actual_delay + 0.1) / 0.2) as i32)
}

pub fn power_estimate_defaults(
    circuit_type: CircuitType,
    delay: DelayModel,
) -> PowerCommandOptions {
    PowerCommandOptions {
        circuit_type,
        delay,
        ps_probability: PresentStateProbability::Approximation,
        logic_form: LogicForm::FactoredForm,
        input_sizing: DEFAULT_MAX_INPUT_SIZING,
        globals: PowerGlobals::default(),
        ..PowerCommandOptions::default()
    }
}

pub fn command_summary(
    network_name: &str,
    options: &PowerCommandOptions,
    total_power: f64,
    word_bits: usize,
) -> String {
    let mut output = String::new();
    match options.circuit_type {
        CircuitType::Combinational => output.push_str("Combinational power estimation, "),
        CircuitType::Sequential => {
            output.push_str("Sequential power estimation");
            match options.ps_probability {
                PresentStateProbability::Approximation => {
                    if options.globals.set_size != 1 {
                        output.push_str(&format!(" (setsize = {}", options.globals.set_size));
                        if options.globals.delta != DEFAULT_PS_MAX_ALLOWED_ERROR {
                            output.push_str(&format!(", maxerror = {}), ", options.globals.delta));
                        } else {
                            output.push_str("), ");
                        }
                    } else if options.globals.delta != DEFAULT_PS_MAX_ALLOWED_ERROR {
                        output.push_str(&format!(" (maxerror = {}), ", options.globals.delta));
                    } else {
                        output.push_str(", ");
                    }
                }
                PresentStateProbability::Exact => output.push_str(" (Exact method), "),
                PresentStateProbability::StateLine => output.push_str(" (State-line method), "),
                PresentStateProbability::Uniform => output.push_str(" (Uniform method), "),
            }
        }
        CircuitType::Pipeline => output.push_str("Pipeline power estimation, "),
        CircuitType::Dynamic => output.push_str("Dynamic power estimation.\n"),
    }

    if options.circuit_type != CircuitType::Dynamic {
        match options.delay {
            DelayModel::Zero => output.push_str("with Zero delay model.\n"),
            DelayModel::Unit => output.push_str("with Unit delay model.\n"),
            DelayModel::General => output.push_str("with General delay model.\n"),
        }
    }

    if options.mode == EstimationMode::Sample {
        output.push_str(&format!(
            "Using Sampling, #vectors = {}\n",
            options.num_samples * word_bits
        ));
    }

    output.push_str(&format!(
        "Network: {}, Power = {:.1} uW assuming 20 MHz clock and Vdd = 5V\n",
        network_name, total_power
    ));
    output
}

#[derive(Clone, Debug, PartialEq)]
pub enum PowerMainError {
    MissingSisDependencies {
        operation: PowerMainOperation,
    },
    MissingOptionValue(&'static str),
    InvalidNumericValue {
        option: &'static str,
        value: String,
    },
    InvalidMode(String),
    InvalidDelay(String),
    InvalidType(String),
    InvalidPresentStateProbability(String),
    InvalidSetSize(usize),
    InvalidDelta(f64),
    TooManyExactStates,
    OptionNotUnderstood(String),
    UsageRequested,
    DuplicateNode(NodeId),
    MissingNode(NodeId),
    MissingFanin {
        node_id: NodeId,
        fanin_id: NodeId,
    },
    FaninDataArityMismatch {
        node_id: NodeId,
        fanins: usize,
        data: usize,
    },
    MissingMappedArrival(NodeId),
    InvalidInfoFileToken(String),
    InvalidInfoFileValue {
        key: &'static str,
        value: String,
    },
    SuddenEndOfInfoFile,
}

impl fmt::Display for PowerMainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
            Self::MissingOptionValue(option) => write!(f, "missing value for option {option}"),
            Self::InvalidNumericValue { option, value } => {
                write!(f, "invalid numeric value {value:?} for option {option}")
            }
            Self::InvalidMode(value) => write!(f, "invalid mode: {value}. Only BDD or SAMPLE"),
            Self::InvalidDelay(value) => {
                write!(f, "invalid delay: {value}. One of ZERO, UNIT or GENERAL")
            }
            Self::InvalidType(value) => {
                write!(
                    f,
                    "invalid type: {value}. One of COMBINATIONAL, SEQUENTIAL, PIPELINE or DYNAMIC"
                )
            }
            Self::InvalidPresentStateProbability(value) => {
                write!(f, "invalid present-state probability calculation: {value}")
            }
            Self::InvalidSetSize(value) => {
                write!(f, "invalid PS lines set size: {value}. Limits are 1 to 16")
            }
            Self::InvalidDelta(value) => {
                write!(f, "invalid error: {value}. Must be a positive value")
            }
            Self::TooManyExactStates => {
                write!(
                    f,
                    "too many states; exact method supports at most 16 latches"
                )
            }
            Self::OptionNotUnderstood(option) => write!(f, "option {option} not understood"),
            Self::UsageRequested => write!(f, "power_estimate usage requested"),
            Self::DuplicateNode(node_id) => write!(f, "duplicate node id {node_id:?}"),
            Self::MissingNode(node_id) => write!(f, "missing node {node_id:?}"),
            Self::MissingFanin { node_id, fanin_id } => {
                write!(f, "node {node_id:?} references missing fanin {fanin_id:?}")
            }
            Self::FaninDataArityMismatch {
                node_id,
                fanins,
                data,
            } => write!(
                f,
                "node {node_id:?} has {fanins} fanins but {data} fanin data entries"
            ),
            Self::MissingMappedArrival(node_id) => {
                write!(f, "node {node_id:?} has no mapped arrival time")
            }
            Self::InvalidInfoFileToken(token) => {
                write!(f, "invalid information file token {token}")
            }
            Self::InvalidInfoFileValue { key, value } => {
                write!(f, "invalid information file value {value:?} for {key}")
            }
            Self::SuddenEndOfInfoFile => write!(f, "sudden end of information file"),
        }
    }
}

impl Error for PowerMainError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> PowerNetwork {
        PowerNetwork::new(
            "sample",
            vec![
                PowerNode::new(0, "a", NodeKind::PrimaryInput { real: true })
                    .with_mapped_arrival(0.0, 0.0),
                PowerNode::new(1, "b", NodeKind::PrimaryInput { real: true })
                    .with_mapped_arrival(0.0, 0.0),
                PowerNode::new(2, "n1", NodeKind::Internal)
                    .with_fanins([NodeId(0), NodeId(1)])
                    .with_sop_literal_counts(vec![
                        PolarityLiteralCount {
                            negative: 1,
                            positive: 0,
                        },
                        PolarityLiteralCount {
                            negative: 0,
                            positive: 2,
                        },
                    ])
                    .with_factored_uses(vec![1, 3])
                    .with_mapped_arrival(0.61, 0.42),
                PowerNode::new(3, "out", NodeKind::PrimaryOutput { real: true })
                    .with_fanins([NodeId(2)])
                    .with_mapped_arrival(0.61, 0.42),
                PowerNode::new(4, "latch_in", NodeKind::PrimaryOutput { real: false })
                    .with_fanins([NodeId(0)])
                    .with_mapped_arrival(0.0, 0.0),
            ],
        )
        .unwrap()
    }

    #[test]
    fn parses_command_line_defaults_and_legacy_options() {
        let args = [
            "power_estimate",
            "-m",
            "sampling",
            "-d",
            "general",
            "-t",
            "sequential",
            "-s",
            "exact",
            "-n",
            "12",
            "-M",
            "3",
            "-S",
            "-N",
            "5",
            "-V",
            "2",
        ];

        let options = PowerCommandOptions::parse_argv(&args, 2).unwrap();

        assert_eq!(options.mode, EstimationMode::Sample);
        assert_eq!(options.delay, DelayModel::General);
        assert_eq!(options.circuit_type, CircuitType::Sequential);
        assert_eq!(options.ps_probability, PresentStateProbability::Exact);
        assert_eq!(options.num_samples, 12);
        assert_eq!(options.input_sizing, 3);
        assert_eq!(options.logic_form, LogicForm::SumOfProducts);
        assert_eq!(options.sample_gap, Some(5));
        assert_eq!(options.globals.verbose, 2);
    }

    #[test]
    fn non_sequential_network_falls_back_to_combinational() {
        let args = ["power_estimate", "-t", "pipeline"];
        let options = PowerCommandOptions::parse_argv(&args, 0).unwrap();

        assert_eq!(options.circuit_type, CircuitType::Combinational);
    }

    #[test]
    fn rejects_exact_state_probability_above_sixteen_latches() {
        let args = ["power_estimate", "-s", "exact"];

        assert_eq!(
            PowerCommandOptions::parse_argv(&args, 17),
            Err(PowerMainError::TooManyExactStates)
        );
    }

    #[test]
    fn builds_factored_node_info_like_power_get_node_info() {
        let network = sample_network();
        let globals = PowerGlobals::default();
        let tables =
            power_get_node_info(&network, &[], LogicForm::FactoredForm, 4, &globals).unwrap();

        assert_eq!(
            tables.power_info(NodeId(0)).unwrap(),
            PowerInfo {
                cap_factor: 6,
                switching_prob: 0.0
            }
        );
        assert_eq!(
            tables.power_info(NodeId(2)).unwrap(),
            PowerInfo {
                cap_factor: 2,
                switching_prob: 0.0
            }
        );
        assert_eq!(
            tables.node_info(NodeId(0)).unwrap(),
            NodeInfo {
                delay: -1,
                prob_one: 0.5
            }
        );
    }

    #[test]
    fn builds_sum_of_products_loads_and_internal_dissipation() {
        let network = sample_network();
        let tables = power_get_node_info(
            &network,
            &[],
            LogicForm::SumOfProducts,
            4,
            &PowerGlobals::default(),
        )
        .unwrap();

        assert_eq!(tables.power_info(NodeId(0)).unwrap().cap_factor, 6);
        assert_eq!(tables.power_info(NodeId(1)).unwrap().cap_factor, 4);
        assert_eq!(tables.power_info(NodeId(2)).unwrap().cap_factor, 1);
    }

    #[test]
    fn applies_information_file_overrides() {
        let network = sample_network();
        let overrides = parse_power_info_overrides("name a p0 0.25 delay 7 cap_factor 9").unwrap();
        let tables = power_get_node_info(
            &network,
            &overrides,
            LogicForm::FactoredForm,
            4,
            &PowerGlobals::default(),
        )
        .unwrap();

        assert_eq!(tables.power_info(NodeId(0)).unwrap().cap_factor, 9);
        assert_eq!(
            tables.node_info(NodeId(0)).unwrap(),
            NodeInfo {
                delay: 7,
                prob_one: 0.75
            }
        );
    }

    #[test]
    fn applies_zero_unit_and_mapped_delays_only_when_uninitialized() {
        let network = sample_network();
        let mut tables = power_get_node_info(
            &network,
            &parse_power_info_overrides("name a delay 8").unwrap(),
            LogicForm::FactoredForm,
            4,
            &PowerGlobals::default(),
        )
        .unwrap();

        tables
            .apply_delay_model(&network, DelayModel::General)
            .unwrap();

        assert_eq!(tables.node_info(NodeId(0)).unwrap().delay, 8);
        assert_eq!(tables.node_info(NodeId(2)).unwrap().delay, 3);
    }

    #[test]
    fn formats_command_summary() {
        let options = PowerCommandOptions {
            mode: EstimationMode::Sample,
            num_samples: 3,
            ..PowerCommandOptions::default()
        };

        let summary = command_summary("demo", &options, 12.34, 64);

        assert!(summary.contains("Combinational power estimation, with Zero delay model."));
        assert!(summary.contains("Using Sampling, #vectors = 192"));
        assert!(summary.contains("Network: demo, Power = 12.3 uW"));
    }
}
