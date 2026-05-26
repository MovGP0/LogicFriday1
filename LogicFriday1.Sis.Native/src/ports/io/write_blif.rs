//! Native Rust BLIF writer for SIS-style networks.
//!
//! The legacy `write_blif.c` streams SIS `network_t` objects directly through
//! shared I/O helpers. This port keeps the observable write decisions in owned
//! Rust data: model headers, real primary interfaces, clocks, delay metadata,
//! latches, mapped gates, `.names` covers, optional `.exdc` sections, and line
//! continuation behavior.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifWriteOptions {
    pub short_names: bool,
    pub netlist: bool,
    pub break_column: usize,
}

impl Default for BlifWriteOptions {
    fn default() -> Self {
        Self {
            short_names: false,
            netlist: false,
            break_column: 78,
        }
    }
}

impl BlifWriteOptions {
    pub fn for_bdsyn() -> Self {
        Self {
            short_names: false,
            netlist: false,
            break_column: 32_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifNetwork {
    pub name: String,
    pub nodes: Vec<BlifNode>,
    pub latches: Vec<BlifLatch>,
    pub clocks: Vec<BlifClock>,
    pub clock_cycle: Option<OrderedReal>,
    pub delays: BlifDelayModel,
    pub kiss: Option<BlifKiss>,
    pub dc_network: Option<Box<BlifNetwork>>,
}

impl BlifNetwork {
    pub fn new(name: impl Into<String>, nodes: Vec<BlifNode>) -> Self {
        Self {
            name: name.into(),
            nodes,
            latches: Vec::new(),
            clocks: Vec::new(),
            clock_cycle: None,
            delays: BlifDelayModel::default(),
            kiss: None,
            dc_network: None,
        }
    }

    pub fn with_latches(mut self, latches: Vec<BlifLatch>) -> Self {
        self.latches = latches;
        self
    }

    pub fn with_dc_network(mut self, dc_network: BlifNetwork) -> Self {
        self.dc_network = Some(Box::new(dc_network));
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifNode {
    pub name: String,
    pub short_name: String,
    pub kind: BlifNodeKind,
    pub fanins: Vec<String>,
    pub cover: Vec<BlifCube>,
    pub gate: Option<BlifGate>,
    pub is_real_interface: bool,
    pub is_clock_input: bool,
}

impl BlifNode {
    pub fn primary_input(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            short_name: name.clone(),
            name,
            kind: BlifNodeKind::PrimaryInput,
            fanins: Vec::new(),
            cover: Vec::new(),
            gate: None,
            is_real_interface: true,
            is_clock_input: false,
        }
    }

    pub fn primary_output(name: impl Into<String>, fanin: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            short_name: name.clone(),
            name,
            kind: BlifNodeKind::PrimaryOutput,
            fanins: vec![fanin.into()],
            cover: Vec::new(),
            gate: None,
            is_real_interface: true,
            is_clock_input: false,
        }
    }

    pub fn internal(
        name: impl Into<String>,
        fanins: impl IntoIterator<Item = impl Into<String>>,
        cover: impl Into<Vec<BlifCube>>,
    ) -> Self {
        let name = name.into();
        Self {
            short_name: name.clone(),
            name,
            kind: BlifNodeKind::Internal,
            fanins: fanins.into_iter().map(Into::into).collect(),
            cover: cover.into(),
            gate: None,
            is_real_interface: true,
            is_clock_input: false,
        }
    }

    pub fn with_short_name(mut self, short_name: impl Into<String>) -> Self {
        self.short_name = short_name.into();
        self
    }

    pub fn with_gate(mut self, gate: BlifGate) -> Self {
        self.gate = Some(gate);
        self
    }

    pub fn with_real_interface(mut self, is_real_interface: bool) -> Self {
        self.is_real_interface = is_real_interface;
        self
    }

    pub fn with_clock_input(mut self, is_clock_input: bool) -> Self {
        self.is_clock_input = is_clock_input;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlifNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifCube {
    pub pattern: Vec<BlifLiteral>,
    pub output_value: bool,
}

impl BlifCube {
    pub fn new(pattern: Vec<BlifLiteral>) -> Self {
        Self {
            pattern,
            output_value: true,
        }
    }

    pub fn with_output(pattern: Vec<BlifLiteral>, output_value: bool) -> Self {
        Self {
            pattern,
            output_value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlifLiteral {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifGate {
    pub name: String,
    pub input_pins: Vec<String>,
    pub output_pin: String,
}

impl BlifGate {
    pub fn new(
        name: impl Into<String>,
        input_pins: impl IntoIterator<Item = impl Into<String>>,
        output_pin: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            input_pins: input_pins.into_iter().map(Into::into).collect(),
            output_pin: output_pin.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifLatch {
    pub input: String,
    pub output: String,
    pub latch_type: BlifLatchType,
    pub control: Option<String>,
    pub initial_value: i32,
    pub gate: Option<BlifGate>,
}

impl BlifLatch {
    pub fn new(input: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            output: output.into(),
            latch_type: BlifLatchType::Unknown,
            control: None,
            initial_value: 0,
            gate: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlifLatchType {
    Unknown,
    RisingEdge,
    FallingEdge,
    ActiveHigh,
    ActiveLow,
    Asynch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifClock {
    pub name: String,
    pub rise: Option<BlifClockEvent>,
    pub fall: Option<BlifClockEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifClockEvent {
    pub nominal_position: OrderedReal,
    pub lower_range: OrderedReal,
    pub upper_range: OrderedReal,
    pub dependencies: Vec<BlifClockEdge>,
}

impl BlifClockEvent {
    pub fn new(nominal_position: f64, lower_range: f64, upper_range: f64) -> Self {
        Self {
            nominal_position: OrderedReal(nominal_position),
            lower_range: OrderedReal(lower_range),
            upper_range: OrderedReal(upper_range),
            dependencies: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifClockEdge {
    pub clock: String,
    pub transition: BlifClockTransition,
    pub lower_range: OrderedReal,
    pub upper_range: OrderedReal,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BlifClockTransition {
    Rise,
    Fall,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct BlifDelayModel {
    pub defaults: BTreeMap<BlifDelayKind, BlifDelayValue>,
    pub node_delays: BTreeMap<String, BTreeMap<BlifDelayKind, BlifDelayValue>>,
    pub wire_load_slope: Option<OrderedReal>,
    pub wire_loads: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BlifDelayKind {
    InputArrival,
    OutputRequired,
    InputDrive,
    OutputLoad,
    MaxInputLoad,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlifDelayValue {
    Pair(OrderedReal, OrderedReal),
    Single(OrderedReal),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifKiss {
    pub body: String,
    pub state_codes: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct OrderedReal(pub f64);

impl Eq for OrderedReal {}

pub fn write_blif(network: &BlifNetwork, options: &BlifWriteOptions) -> Result<String, BlifError> {
    BlifWriter::new(network, options).write()
}

pub fn write_blif_for_bdsyn(network: &BlifNetwork) -> Result<String, BlifError> {
    write_blif(network, &BlifWriteOptions::for_bdsyn())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlifError {
    DuplicateNode {
        node: String,
    },
    MissingNode {
        node: String,
    },
    MissingFanin {
        node: String,
        fanin: String,
    },
    PrimaryOutputFaninCount {
        node: String,
        count: usize,
    },
    CoverWidthMismatch {
        node: String,
        cube: usize,
        expected: usize,
        actual: usize,
    },
    GatePinCountMismatch {
        node: String,
        pins: usize,
        fanins: usize,
    },
}

impl fmt::Display for BlifError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode { node } => write!(f, "duplicate BLIF node '{node}'"),
            Self::MissingNode { node } => write!(f, "missing BLIF node '{node}'"),
            Self::MissingFanin { node, fanin } => {
                write!(f, "BLIF node '{node}' references missing fanin '{fanin}'")
            }
            Self::PrimaryOutputFaninCount { node, count } => {
                write!(f, "primary output '{node}' has {count} fanins")
            }
            Self::CoverWidthMismatch {
                node,
                cube,
                expected,
                actual,
            } => write!(
                f,
                "node '{node}' cube {cube} has width {actual}, expected {expected}"
            ),
            Self::GatePinCountMismatch { node, pins, fanins } => write!(
                f,
                "mapped node '{node}' has {fanins} fanins but {pins} input pins"
            ),
        }
    }
}

impl Error for BlifError {}

struct BlifWriter<'a> {
    network: &'a BlifNetwork,
    options: &'a BlifWriteOptions,
    nodes: HashMap<&'a str, &'a BlifNode>,
    real_po_fanouts: HashMap<&'a str, Vec<&'a BlifNode>>,
    output: BreakWriter,
}

impl<'a> BlifWriter<'a> {
    fn new(network: &'a BlifNetwork, options: &'a BlifWriteOptions) -> Self {
        Self {
            network,
            options,
            nodes: HashMap::new(),
            real_po_fanouts: HashMap::new(),
            output: BreakWriter::new(options.break_column),
        }
    }

    fn write(mut self) -> Result<String, BlifError> {
        self.index_network(self.network)?;
        self.write_main_network()?;
        if let Some(dc_network) = self.network.dc_network.as_deref() {
            self.write_dc_network(dc_network)?;
        }
        self.output.push_str(".end\n");
        Ok(self.output.into_string())
    }

    fn write_main_network(&mut self) -> Result<(), BlifError> {
        self.output
            .push_str(&format!(".model {}\n.inputs", self.network.name));
        for node in self.real_inputs(self.network) {
            self.output.push_char(' ');
            self.write_name(node);
        }
        self.output.push_char('\n');

        self.output.push_str(".outputs");
        for node in self.real_outputs(self.network) {
            self.output.push_char(' ');
            self.write_name(node);
        }
        self.output.push_char('\n');

        self.write_clocks(self.network)?;
        self.write_delays(self.network);
        self.write_latches(self.network)?;
        self.write_kiss(self.network);
        self.write_nodes(self.network)
    }

    fn write_dc_network(&mut self, network: &'a BlifNetwork) -> Result<(), BlifError> {
        self.index_network(network)?;
        self.output.push_str(".exdc \n.inputs");
        for node in self.real_inputs(network) {
            self.output.push_char(' ');
            self.write_name(node);
        }
        self.output.push_char('\n');

        self.output.push_str(".outputs");
        for node in self.real_outputs(network) {
            self.output.push_char(' ');
            self.write_name(node);
        }
        self.output.push_char('\n');

        self.write_nodes(network)
    }

    fn index_network(&mut self, network: &'a BlifNetwork) -> Result<(), BlifError> {
        self.nodes.clear();
        self.real_po_fanouts.clear();
        let mut names = HashSet::new();
        for node in &network.nodes {
            if !names.insert(node.name.as_str()) {
                return Err(BlifError::DuplicateNode {
                    node: node.name.clone(),
                });
            }
            self.nodes.insert(node.name.as_str(), node);
        }

        for node in &network.nodes {
            match node.kind {
                BlifNodeKind::PrimaryOutput => {
                    if node.fanins.len() != 1 {
                        return Err(BlifError::PrimaryOutputFaninCount {
                            node: node.name.clone(),
                            count: node.fanins.len(),
                        });
                    }
                    let fanin = node.fanins[0].as_str();
                    if !self.nodes.contains_key(fanin) {
                        return Err(BlifError::MissingFanin {
                            node: node.name.clone(),
                            fanin: fanin.to_string(),
                        });
                    }
                    if node.is_real_interface {
                        self.real_po_fanouts.entry(fanin).or_default().push(node);
                    }
                }
                BlifNodeKind::Internal => {
                    for fanin in &node.fanins {
                        if !self.nodes.contains_key(fanin.as_str()) {
                            return Err(BlifError::MissingFanin {
                                node: node.name.clone(),
                                fanin: fanin.clone(),
                            });
                        }
                    }
                    for (cube_index, cube) in node.cover.iter().enumerate() {
                        if cube.pattern.len() != node.fanins.len() {
                            return Err(BlifError::CoverWidthMismatch {
                                node: node.name.clone(),
                                cube: cube_index,
                                expected: node.fanins.len(),
                                actual: cube.pattern.len(),
                            });
                        }
                    }
                    if let Some(gate) = &node.gate {
                        if gate.input_pins.len() < node.fanins.len() {
                            return Err(BlifError::GatePinCountMismatch {
                                node: node.name.clone(),
                                pins: gate.input_pins.len(),
                                fanins: node.fanins.len(),
                            });
                        }
                    }
                }
                BlifNodeKind::PrimaryInput => {}
            }
        }

        for latch in &network.latches {
            self.require_node(&latch.input)?;
            self.require_node(&latch.output)?;
            if let Some(control) = &latch.control {
                self.require_node(control)?;
            }
        }

        for clock in &network.clocks {
            self.require_node(&clock.name)?;
        }

        Ok(())
    }

    fn require_node(&self, name: &str) -> Result<(), BlifError> {
        if self.nodes.contains_key(name) {
            Ok(())
        } else {
            Err(BlifError::MissingNode {
                node: name.to_string(),
            })
        }
    }

    fn real_inputs(&self, network: &'a BlifNetwork) -> Vec<&'a BlifNode> {
        network
            .nodes
            .iter()
            .filter(|node| {
                node.kind == BlifNodeKind::PrimaryInput
                    && node.is_real_interface
                    && !node.is_clock_input
            })
            .collect()
    }

    fn real_outputs(&self, network: &'a BlifNetwork) -> Vec<&'a BlifNode> {
        network
            .nodes
            .iter()
            .filter(|node| node.kind == BlifNodeKind::PrimaryOutput && node.is_real_interface)
            .collect()
    }

    fn write_clocks(&mut self, network: &BlifNetwork) -> Result<(), BlifError> {
        if !network.clocks.is_empty() {
            self.output.push_str(".clock");
            for clock in &network.clocks {
                let node =
                    self.nodes
                        .get(clock.name.as_str())
                        .ok_or_else(|| BlifError::MissingNode {
                            node: clock.name.clone(),
                        })?;
                self.output.push_char(' ');
                self.write_name(node);
            }
            self.output.push_char('\n');
        }

        if let Some(cycle) = network.clock_cycle {
            if cycle.0 > 0.0 {
                self.output.push_str(&format!(".cycle {:.2}\n", cycle.0));
            }
        }

        let mut done = BTreeSet::new();
        for clock in &network.clocks {
            if let Some(event) = &clock.rise {
                self.write_clock_event(clock, BlifClockTransition::Rise, event, &mut done)?;
            }
            if let Some(event) = &clock.fall {
                self.write_clock_event(clock, BlifClockTransition::Fall, event, &mut done)?;
            }
        }
        Ok(())
    }

    fn write_clock_event(
        &mut self,
        clock: &BlifClock,
        transition: BlifClockTransition,
        event: &BlifClockEvent,
        done: &mut BTreeSet<(String, BlifClockTransition)>,
    ) -> Result<(), BlifError> {
        if !done.insert((clock.name.clone(), transition)) {
            return Ok(());
        }

        self.output
            .push_str(&format!(".clock_event {:.2}", event.nominal_position.0));
        self.write_clock_edge(
            &clock.name,
            transition,
            event.lower_range,
            event.upper_range,
        )?;
        for dependency in &event.dependencies {
            if done.insert((dependency.clock.clone(), dependency.transition)) {
                self.write_clock_edge(
                    &dependency.clock,
                    dependency.transition,
                    dependency.lower_range,
                    dependency.upper_range,
                )?;
            }
        }
        self.output.push_char('\n');
        Ok(())
    }

    fn write_clock_edge(
        &mut self,
        clock: &str,
        transition: BlifClockTransition,
        lower: OrderedReal,
        upper: OrderedReal,
    ) -> Result<(), BlifError> {
        let node = self
            .nodes
            .get(clock)
            .ok_or_else(|| BlifError::MissingNode {
                node: clock.to_string(),
            })?;
        let prefix = match transition {
            BlifClockTransition::Rise => 'r',
            BlifClockTransition::Fall => 'f',
        };
        self.output.push_str(&format!(
            " ({prefix}'{} {:.2} {:.2})",
            self.name_of(node),
            lower.0,
            upper.0
        ));
        Ok(())
    }

    fn write_delays(&mut self, network: &BlifNetwork) {
        for (kind, value) in &network.delays.defaults {
            self.write_delay_line(kind.default_directive(), None, *value);
        }
        if let Some(slope) = network.delays.wire_load_slope {
            self.output
                .push_str(&format!(".wire_load_slope {:.2}\n", slope.0));
        }
        for wire_load in &network.delays.wire_loads {
            self.output.push_str(wire_load);
            if !wire_load.ends_with('\n') {
                self.output.push_char('\n');
            }
        }

        for (node_name, delays) in &network.delays.node_delays {
            if let Some(node) = self.nodes.get(node_name.as_str()) {
                if !node.is_real_interface {
                    continue;
                }
                let printed_name = self.name_of(node);
                for (kind, value) in delays {
                    self.write_delay_line(
                        kind.node_directive(),
                        Some(printed_name.as_str()),
                        *value,
                    );
                }
            }
        }
    }

    fn write_delay_line(
        &mut self,
        directive: &str,
        node_name: Option<&str>,
        value: BlifDelayValue,
    ) {
        self.output.push_str(directive);
        if let Some(node_name) = node_name {
            self.output.push_char(' ');
            self.output.push_str(node_name);
        }
        match value {
            BlifDelayValue::Pair(rise, fall) => {
                self.output
                    .push_str(&format!(" {:.2} {:.2}\n", rise.0, fall.0));
            }
            BlifDelayValue::Single(value) => {
                self.output.push_str(&format!(" {:.2}\n", value.0));
            }
        }
    }

    fn write_latches(&mut self, network: &BlifNetwork) -> Result<(), BlifError> {
        for latch in &network.latches {
            if self.options.netlist {
                if let Some(gate) = &latch.gate {
                    self.output.push_str(&format!(".gate {}", gate.name));
                    for (pin, fanin) in gate.input_pins.iter().zip([latch.input.as_str()]) {
                        let fanin =
                            self.nodes
                                .get(fanin)
                                .ok_or_else(|| BlifError::MissingNode {
                                    node: fanin.to_string(),
                                })?;
                        self.output
                            .push_str(&format!(" {pin}={}", self.name_of(fanin)));
                    }
                    self.output.push_str(&format!(
                        " {}={}",
                        gate.output_pin,
                        self.name_by_lookup(&latch.output)?
                    ));
                    if let Some(control) = &latch.control {
                        self.output
                            .push_str(&format!(" {}", self.name_by_lookup(control)?));
                    } else {
                        self.output.push_str(" NIL");
                    }
                    self.output.push_str(&format!(" {}\n", latch.initial_value));
                    continue;
                }
            }

            self.output.push_str(".latch    ");
            self.output.push_str(&self.name_by_lookup(&latch.input)?);
            self.output.push_char(' ');
            self.output.push_str(&self.name_by_lookup(&latch.output)?);
            if let Some(kind) = latch_type_token(latch.latch_type) {
                self.output.push_char(' ');
                self.output.push_str(kind);
            }
            if let Some(control) = &latch.control {
                self.output.push_char(' ');
                self.output.push_str(&self.name_by_lookup(control)?);
            } else if latch.latch_type != BlifLatchType::Unknown {
                self.output.push_str(" NIL");
            }
            self.output.push_str(&format!(" {}\n", latch.initial_value));
        }
        Ok(())
    }

    fn write_kiss(&mut self, network: &BlifNetwork) {
        if let Some(kiss) = &network.kiss {
            self.output.push_str(".start_kiss\n");
            self.output.push_str(&kiss.body);
            if !kiss.body.ends_with('\n') {
                self.output.push_char('\n');
            }
            self.output.push_str(".end_kiss\n.latch_order");
            for latch in &network.latches {
                self.output.push_char(' ');
                self.output.push_str(
                    &self
                        .name_by_lookup(&latch.output)
                        .unwrap_or_else(|_| latch.output.clone()),
                );
            }
            self.output.push_char('\n');
            for (state, code) in &kiss.state_codes {
                self.output.push_str(&format!(".code {state} {code}\n"));
            }
        }
    }

    fn write_nodes(&mut self, network: &'a BlifNetwork) -> Result<(), BlifError> {
        for node in &network.nodes {
            if self.options.netlist && node.gate.is_some() {
                self.write_gate(node)?;
            } else {
                self.write_node(node)?;
            }
        }
        Ok(())
    }

    fn write_gate(&mut self, node: &'a BlifNode) -> Result<(), BlifError> {
        if !self.node_should_be_printed(node) {
            return Ok(());
        }
        let Some(gate) = &node.gate else {
            return self.write_node(node);
        };
        self.output.push_str(&format!(".gate {}", gate.name));
        for (index, fanin) in node.fanins.iter().enumerate() {
            let pin =
                gate.input_pins
                    .get(index)
                    .ok_or_else(|| BlifError::GatePinCountMismatch {
                        node: node.name.clone(),
                        pins: gate.input_pins.len(),
                        fanins: node.fanins.len(),
                    })?;
            self.output
                .push_str(&format!(" {pin}={}", self.name_by_lookup(fanin)?));
        }
        self.output
            .push_str(&format!(" {}={}\n", gate.output_pin, self.name_of(node)));
        Ok(())
    }

    fn write_node(&mut self, node: &'a BlifNode) -> Result<(), BlifError> {
        if !self.node_should_be_printed(node) {
            return Ok(());
        }

        self.output.push_str(".names");
        for fanin in &node.fanins {
            self.output.push_char(' ');
            self.output.push_str(&self.name_by_lookup(fanin)?);
        }
        self.output.push_char(' ');
        self.write_name(node);
        self.output.push_char('\n');

        if node.kind == BlifNodeKind::PrimaryOutput {
            self.output.push_str("1 1\n");
        } else {
            for cube in &node.cover {
                if !cube.output_value {
                    continue;
                }
                for literal in &cube.pattern {
                    self.output.push_char(literal.as_char());
                }
                self.output.push_str(" 1\n");
            }
        }
        Ok(())
    }

    fn node_should_be_printed(&self, node: &'a BlifNode) -> bool {
        match node.kind {
            BlifNodeKind::PrimaryInput => false,
            BlifNodeKind::Internal => true,
            BlifNodeKind::PrimaryOutput => {
                if !node.is_real_interface {
                    return false;
                }
                let fanin = &node.fanins[0];
                let Some(driver) = self.nodes.get(fanin.as_str()) else {
                    return true;
                };
                let real_po_count = self.real_po_fanouts.get(fanin.as_str()).map_or(0, Vec::len);
                driver.kind == BlifNodeKind::PrimaryInput || real_po_count > 1
            }
        }
    }

    fn name_by_lookup(&self, name: &str) -> Result<String, BlifError> {
        let node = self.nodes.get(name).ok_or_else(|| BlifError::MissingNode {
            node: name.to_string(),
        })?;
        Ok(self.name_of(node))
    }

    fn write_name(&mut self, node: &BlifNode) {
        self.output.push_str(&self.name_of(node));
    }

    fn name_of(&self, node: &BlifNode) -> String {
        if node.kind == BlifNodeKind::Internal {
            if let Some(outputs) = self.real_po_fanouts.get(node.name.as_str()) {
                if outputs.len() == 1 {
                    return selected_name(outputs[0], self.options.short_names);
                }
            }
        }
        selected_name(node, self.options.short_names)
    }
}

impl BlifLiteral {
    fn as_char(self) -> char {
        match self {
            Self::Zero => '0',
            Self::One => '1',
            Self::DontCare => '-',
        }
    }
}

impl BlifDelayKind {
    fn default_directive(self) -> &'static str {
        match self {
            Self::InputArrival => ".default_input_arrival",
            Self::OutputRequired => ".default_output_required",
            Self::InputDrive => ".default_input_drive",
            Self::OutputLoad => ".default_output_load",
            Self::MaxInputLoad => ".default_max_input_load",
        }
    }

    fn node_directive(self) -> &'static str {
        match self {
            Self::InputArrival => ".input_arrival",
            Self::OutputRequired => ".output_required",
            Self::InputDrive => ".input_drive",
            Self::OutputLoad => ".output_load",
            Self::MaxInputLoad => ".max_input_load",
        }
    }
}

fn selected_name(node: &BlifNode, short_names: bool) -> String {
    if short_names {
        node.short_name.clone()
    } else {
        node.name.clone()
    }
}

fn latch_type_token(latch_type: BlifLatchType) -> Option<&'static str> {
    match latch_type {
        BlifLatchType::Unknown => None,
        BlifLatchType::RisingEdge => Some("re"),
        BlifLatchType::FallingEdge => Some("fe"),
        BlifLatchType::ActiveHigh => Some("ah"),
        BlifLatchType::ActiveLow => Some("al"),
        BlifLatchType::Asynch => Some("as"),
    }
}

struct BreakWriter {
    output: String,
    break_column: usize,
    column: usize,
}

impl BreakWriter {
    fn new(break_column: usize) -> Self {
        Self {
            output: String::new(),
            break_column,
            column: 0,
        }
    }

    fn push_str(&mut self, value: &str) {
        if self.column + value.len() > self.break_column && self.break_column < 32_000 {
            self.push_raw("\\\n");
        }
        self.push_raw(value);
    }

    fn push_char(&mut self, value: char) {
        self.push_str(&value.to_string());
    }

    fn push_raw(&mut self, value: &str) {
        for character in value.chars() {
            self.output.push(character);
            if character == '\n' {
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
    }

    fn into_string(self) -> String {
        self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(value: char) -> BlifLiteral {
        match value {
            '0' => BlifLiteral::Zero,
            '1' => BlifLiteral::One,
            '-' => BlifLiteral::DontCare,
            other => panic!("unexpected literal {other}"),
        }
    }

    fn sample_network() -> BlifNetwork {
        BlifNetwork::new(
            "demo",
            vec![
                BlifNode::primary_input("a"),
                BlifNode::primary_input("b"),
                BlifNode::internal(
                    "n",
                    ["a", "b"],
                    vec![
                        BlifCube::new(vec![lit('1'), lit('-')]),
                        BlifCube::new(vec![lit('0'), lit('1')]),
                    ],
                ),
                BlifNode::primary_output("y", "n"),
            ],
        )
    }

    #[test]
    fn writes_basic_blif_and_uses_output_name_for_single_fanout_driver() {
        let blif = write_blif(&sample_network(), &BlifWriteOptions::default()).unwrap();

        assert_eq!(
            blif,
            concat!(
                ".model demo\n",
                ".inputs a b\n",
                ".outputs y\n",
                ".names a b y\n",
                "1- 1\n",
                "01 1\n",
                ".end\n"
            )
        );
    }

    #[test]
    fn writes_primary_output_node_when_driver_feeds_multiple_outputs() {
        let network = BlifNetwork::new(
            "fanout",
            vec![
                BlifNode::primary_input("a"),
                BlifNode::internal("n", ["a"], vec![BlifCube::new(vec![lit('1')])]),
                BlifNode::primary_output("y0", "n"),
                BlifNode::primary_output("y1", "n"),
            ],
        );

        let blif = write_blif(&network, &BlifWriteOptions::default()).unwrap();

        assert!(blif.contains(".names a n\n1 1\n"));
        assert!(blif.contains(".names n y0\n1 1\n"));
        assert!(blif.contains(".names n y1\n1 1\n"));
    }

    #[test]
    fn omits_clock_inputs_and_synthetic_interfaces() {
        let mut clk = BlifNode::primary_input("clk").with_clock_input(true);
        clk.short_name = "c".to_string();
        let network = BlifNetwork {
            clocks: vec![BlifClock {
                name: "clk".to_string(),
                rise: Some(BlifClockEvent::new(5.0, 4.5, 5.5)),
                fall: None,
            }],
            ..BlifNetwork::new(
                "clocked",
                vec![
                    clk,
                    BlifNode::primary_input("a"),
                    BlifNode::primary_output("fake", "a").with_real_interface(false),
                    BlifNode::primary_output("y", "a"),
                ],
            )
        };

        let blif = write_blif(
            &network,
            &BlifWriteOptions {
                short_names: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(blif.contains(".inputs a\n"));
        assert!(blif.contains(".outputs y\n"));
        assert!(blif.contains(".clock c\n"));
        assert!(blif.contains(".clock_event 5.00 (r'c 4.50 5.50)\n"));
        assert!(!blif.contains("fake"));
    }

    #[test]
    fn writes_latches_with_type_control_and_initial_value() {
        let network = BlifNetwork::new(
            "seq",
            vec![
                BlifNode::primary_input("d"),
                BlifNode::primary_input("clk").with_clock_input(true),
                BlifNode::primary_input("q"),
                BlifNode::primary_output("din", "d").with_real_interface(false),
                BlifNode::primary_output("y", "q"),
            ],
        )
        .with_latches(vec![BlifLatch {
            input: "din".to_string(),
            output: "q".to_string(),
            latch_type: BlifLatchType::RisingEdge,
            control: Some("clk".to_string()),
            initial_value: 1,
            gate: None,
        }]);

        let blif = write_blif(&network, &BlifWriteOptions::default()).unwrap();

        assert!(blif.contains(".latch    din q re clk 1\n"));
    }

    #[test]
    fn writes_mapped_gates_when_netlist_mode_is_enabled() {
        let network = BlifNetwork::new(
            "mapped",
            vec![
                BlifNode::primary_input("a"),
                BlifNode::primary_input("b"),
                BlifNode::internal("n", ["a", "b"], Vec::new()).with_gate(BlifGate::new(
                    "nand2",
                    ["A", "B"],
                    "Y",
                )),
                BlifNode::primary_output("y", "n"),
            ],
        );

        let blif = write_blif(
            &network,
            &BlifWriteOptions {
                netlist: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(blif.contains(".gate nand2 A=a B=b Y=y\n"));
    }

    #[test]
    fn writes_delay_metadata() {
        let mut network = sample_network();
        network.delays.defaults.insert(
            BlifDelayKind::InputArrival,
            BlifDelayValue::Pair(OrderedReal(1.0), OrderedReal(2.0)),
        );
        network.delays.node_delays.insert(
            "y".to_string(),
            BTreeMap::from([(
                BlifDelayKind::OutputLoad,
                BlifDelayValue::Single(OrderedReal(3.5)),
            )]),
        );
        network.delays.wire_load_slope = Some(OrderedReal(0.25));

        let blif = write_blif(&network, &BlifWriteOptions::default()).unwrap();

        assert!(blif.contains(".default_input_arrival 1.00 2.00\n"));
        assert!(blif.contains(".wire_load_slope 0.25\n"));
        assert!(blif.contains(".output_load y 3.50\n"));
    }

    #[test]
    fn writes_exdc_section() {
        let dc = BlifNetwork::new(
            "dc",
            vec![
                BlifNode::primary_input("a"),
                BlifNode::internal("dc_n", ["a"], vec![BlifCube::new(vec![lit('0')])]),
                BlifNode::primary_output("y", "dc_n"),
            ],
        );
        let network = sample_network().with_dc_network(dc);

        let blif = write_blif(&network, &BlifWriteOptions::default()).unwrap();

        assert!(blif.contains(".exdc \n.inputs a\n.outputs y\n"));
        assert!(blif.contains(".names a y\n0 1\n"));
    }

    #[test]
    fn writes_kiss_latch_order_and_codes() {
        let mut network = BlifNetwork::new(
            "fsm",
            vec![
                BlifNode::primary_input("d"),
                BlifNode::primary_input("q"),
                BlifNode::primary_output("li", "d").with_real_interface(false),
                BlifNode::primary_output("y", "q"),
            ],
        )
        .with_latches(vec![BlifLatch::new("li", "q")]);
        network.kiss = Some(BlifKiss {
            body: ".i 1\n.o 1\n".to_string(),
            state_codes: BTreeMap::from([("s0".to_string(), "0".to_string())]),
        });

        let blif = write_blif(&network, &BlifWriteOptions::default()).unwrap();

        assert!(blif.contains(".start_kiss\n.i 1\n.o 1\n.end_kiss\n"));
        assert!(blif.contains(".latch_order q\n"));
        assert!(blif.contains(".code s0 0\n"));
    }

    #[test]
    fn validates_duplicate_names_missing_fanins_and_cover_widths() {
        let duplicate = BlifNetwork::new(
            "bad",
            vec![BlifNode::primary_input("a"), BlifNode::primary_input("a")],
        );
        assert!(matches!(
            write_blif(&duplicate, &BlifWriteOptions::default()),
            Err(BlifError::DuplicateNode { .. })
        ));

        let missing = BlifNetwork::new("bad", vec![BlifNode::primary_output("y", "missing")]);
        assert!(matches!(
            write_blif(&missing, &BlifWriteOptions::default()),
            Err(BlifError::MissingFanin { .. })
        ));

        let width = BlifNetwork::new(
            "bad",
            vec![
                BlifNode::primary_input("a"),
                BlifNode::internal("n", ["a"], vec![BlifCube::new(vec![lit('1'), lit('0')])]),
            ],
        );
        assert!(matches!(
            write_blif(&width, &BlifWriteOptions::default()),
            Err(BlifError::CoverWidthMismatch { .. })
        ));
    }

    #[test]
    fn no_legacy_c_abi_or_dependency_metadata_tokens_are_present() {
        let source = include_str!("write_blif.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("be", "ad", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1", "-", "8", "j", "8")));
    }
}
