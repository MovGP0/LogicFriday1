//! Native BLIF reader.
//!
//! The legacy reader tokenizes BLIF input, builds SIS networks from `.inputs`,
//! `.outputs`, `.names`/`.cover`, `.latch`, `.subckt`, `.gate`, clock metadata,
//! and attaches an `.exdc` network. This port keeps those semantics in an owned
//! Rust model so command and network ports can integrate without per-file C ABI
//! entry points.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub type BlifResult<T> = Result<T, BlifReadError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlifReadLimits {
    pub max_lines: usize,
    pub max_line_length: usize,
    pub max_name_length: usize,
    pub max_models: usize,
    pub max_nodes: usize,
    pub max_cover_rows: usize,
}

impl Default for BlifReadLimits {
    fn default() -> Self {
        Self {
            max_lines: 262_144,
            max_line_length: 16_384,
            max_name_length: 1_024,
            max_models: 4_096,
            max_nodes: 262_144,
            max_cover_rows: 1_048_576,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlifLiteral {
    Zero,
    One,
    DontCare,
}

impl BlifLiteral {
    fn parse(value: char) -> Option<Self> {
        match value {
            '0' => Some(Self::Zero),
            '1' => Some(Self::One),
            '-' | '2' => Some(Self::DontCare),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifCoverRow {
    pub literals: Vec<BlifLiteral>,
    pub output_value: bool,
}

impl BlifCoverRow {
    pub fn new(literals: Vec<BlifLiteral>, output_value: bool) -> Self {
        Self {
            literals,
            output_value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifNode {
    pub fanins: Vec<String>,
    pub output: String,
    pub cover: Vec<BlifCoverRow>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatchType {
    Unknown,
    FallingEdge,
    RisingEdge,
    ActiveHigh,
    ActiveLow,
    Asynchronous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifLatch {
    pub input: String,
    pub output: String,
    pub latch_type: LatchType,
    pub control: Option<String>,
    pub initial_value: Option<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifSubckt {
    pub model: String,
    pub connections: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifGate {
    pub model: String,
    pub connections: Vec<(String, String)>,
    pub latch_control: Option<String>,
    pub latch_initial_value: Option<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KissBlock {
    pub lines: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockTransition {
    Rise,
    Fall,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockEventEdge {
    pub transition: ClockTransition,
    pub clock: String,
    pub lower_range: Option<f64>,
    pub upper_range: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockEvent {
    pub nominal_position: f64,
    pub edges: Vec<ClockEventEdge>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlifNetwork {
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub nodes: Vec<BlifNode>,
    pub latches: Vec<BlifLatch>,
    pub latch_order: Vec<String>,
    pub gates: Vec<BlifGate>,
    pub subckts: Vec<BlifSubckt>,
    pub kiss_blocks: Vec<KissBlock>,
    pub clocks: Vec<String>,
    pub clock_events: Vec<ClockEvent>,
    pub state_codes: Vec<(String, String)>,
    pub cycle_time: Option<f64>,
    pub external_dc: Option<Box<BlifNetwork>>,
}

impl BlifNetwork {
    fn new(name: String) -> Self {
        Self {
            name,
            inputs: Vec::new(),
            outputs: Vec::new(),
            nodes: Vec::new(),
            latches: Vec::new(),
            latch_order: Vec::new(),
            gates: Vec::new(),
            subckts: Vec::new(),
            kiss_blocks: Vec::new(),
            clocks: Vec::new(),
            clock_events: Vec::new(),
            state_codes: Vec::new(),
            cycle_time: None,
            external_dc: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlifRead {
    pub networks: Vec<BlifNetwork>,
    pub search_files: Vec<String>,
    pub warnings: Vec<String>,
}

impl BlifRead {
    pub fn first_network(&self) -> Option<&BlifNetwork> {
        self.networks.first()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlifReadError {
    NoNetwork,
    LimitExceeded {
        kind: &'static str,
        limit: usize,
    },
    MissingArgument {
        directive: &'static str,
        line: usize,
    },
    UnexpectedStatement {
        line: usize,
        statement: String,
    },
    DuplicateModel {
        line: usize,
        name: String,
    },
    DuplicateName {
        line: usize,
        kind: &'static str,
        name: String,
    },
    MultiplyDefinedOutput {
        line: usize,
        name: String,
    },
    InvalidCoverHeader {
        line: usize,
    },
    MultiOutputCover {
        line: usize,
        outputs: usize,
    },
    InvalidCoverRow {
        line: usize,
        row: String,
    },
    MixedCoverPolarity {
        line: usize,
    },
    BadAssignment {
        line: usize,
        directive: &'static str,
        value: String,
    },
    InvalidLatch {
        line: usize,
        reason: String,
    },
    InvalidClockEvent {
        line: usize,
        reason: String,
    },
    InvalidNumber {
        line: usize,
        directive: &'static str,
        value: String,
    },
}

impl fmt::Display for BlifReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoNetwork => write!(f, "no BLIF network found"),
            Self::LimitExceeded { kind, limit } => write!(f, "{kind} limit exceeded: {limit}"),
            Self::MissingArgument { directive, line } => {
                write!(f, "line {line}: {directive} is missing a required argument")
            }
            Self::UnexpectedStatement { line, statement } => {
                write!(f, "line {line}: unexpected BLIF statement `{statement}`")
            }
            Self::DuplicateModel { line, name } => {
                write!(f, "line {line}: model `{name}` is already defined")
            }
            Self::DuplicateName { line, kind, name } => {
                write!(f, "line {line}: duplicate {kind} `{name}`")
            }
            Self::MultiplyDefinedOutput { line, name } => {
                write!(f, "line {line}: output `{name}` is multiply defined")
            }
            Self::InvalidCoverHeader { line } => {
                write!(f, "line {line}: .cover requires nin, nout, and nterm")
            }
            Self::MultiOutputCover { line, outputs } => {
                write!(
                    f,
                    "line {line}: only single-output .cover is supported, got {outputs}"
                )
            }
            Self::InvalidCoverRow { line, row } => {
                write!(f, "line {line}: invalid BLIF cover row `{row}`")
            }
            Self::MixedCoverPolarity { line } => {
                write!(f, "line {line}: cover cannot mix ON-set and OFF-set rows")
            }
            Self::BadAssignment {
                line,
                directive,
                value,
            } => {
                write!(f, "line {line}: bad {directive} assignment `{value}`")
            }
            Self::InvalidLatch { line, reason } => {
                write!(f, "line {line}: invalid latch: {reason}")
            }
            Self::InvalidClockEvent { line, reason } => {
                write!(f, "line {line}: invalid clock event: {reason}")
            }
            Self::InvalidNumber {
                line,
                directive,
                value,
            } => {
                write!(
                    f,
                    "line {line}: invalid numeric argument for {directive}: `{value}`"
                )
            }
        }
    }
}

impl Error for BlifReadError {}

#[derive(Clone, Debug)]
struct LogicalLine {
    number: usize,
    text: String,
}

#[derive(Default)]
struct ModelState {
    inputs: BTreeSet<String>,
    outputs: BTreeSet<String>,
    driven: BTreeSet<String>,
    symbols: BTreeSet<String>,
}

pub fn read_blif(input: &str) -> BlifResult<BlifRead> {
    read_blif_with_limits(input, BlifReadLimits::default())
}

pub fn read_blif_first(input: &str) -> BlifResult<BlifNetwork> {
    let read = read_blif_first_with_limits(input, BlifReadLimits::default())?;
    read.networks
        .into_iter()
        .next()
        .ok_or(BlifReadError::NoNetwork)
}

pub fn read_blif_with_limits(input: &str, limits: BlifReadLimits) -> BlifResult<BlifRead> {
    let lines = preprocess_lines(input, limits)?;
    let mut parser = Parser::new(lines, limits, false);
    parser.parse()
}

pub fn read_blif_first_with_limits(input: &str, limits: BlifReadLimits) -> BlifResult<BlifRead> {
    let lines = preprocess_lines(input, limits)?;
    let mut parser = Parser::new(lines, limits, true);
    parser.parse()
}

pub fn preprocess_blif_lines(input: &str) -> BlifResult<Vec<String>> {
    Ok(preprocess_lines(input, BlifReadLimits::default())?
        .into_iter()
        .map(|line| line.text)
        .collect())
}

struct Parser {
    lines: Vec<LogicalLine>,
    limits: BlifReadLimits,
    only_first: bool,
    index: usize,
    model_names: BTreeSet<String>,
    fake_models: usize,
    search_files: Vec<String>,
    warnings: Vec<String>,
}

impl Parser {
    fn new(lines: Vec<LogicalLine>, limits: BlifReadLimits, only_first: bool) -> Self {
        Self {
            lines,
            limits,
            only_first,
            index: 0,
            model_names: BTreeSet::new(),
            fake_models: 0,
            search_files: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn parse(&mut self) -> BlifResult<BlifRead> {
        let mut networks = Vec::new();
        let mut current = None;
        let mut state = ModelState::default();

        while let Some(line) = self.current().cloned() {
            let tokens = tokenize(&line.text);
            if tokens.is_empty() {
                self.index += 1;
                continue;
            }

            let head = tokens[0];
            if !head.starts_with('.') {
                return Err(BlifReadError::UnexpectedStatement {
                    line: line.number,
                    statement: line.text,
                });
            }

            match head {
                ".model" | ".circuit" => {
                    if current.is_some() && self.only_first {
                        break;
                    }
                    if let Some(network) = current.take() {
                        networks.push(network);
                        state = ModelState::default();
                    }
                    let name = required_token(&tokens, 1, ".model", line.number)?;
                    current = Some(self.start_model(name, line.number)?);
                    self.index += 1;
                }
                ".search" => {
                    let value = required_token(&tokens, 1, ".search", line.number)?;
                    self.search_files.push(value.to_string());
                    self.index += 1;
                }
                ".end" => {
                    if let Some(network) = current.take() {
                        networks.push(network);
                    }
                    if self.only_first {
                        self.index += 1;
                        break;
                    }
                    state = ModelState::default();
                    self.index += 1;
                }
                ".exdc" => {
                    let network =
                        current
                            .as_mut()
                            .ok_or_else(|| BlifReadError::UnexpectedStatement {
                                line: line.number,
                                statement: line.text.clone(),
                            })?;
                    self.index += 1;
                    let dc = self.parse_external_dc(network.name.clone())?;
                    network.external_dc = Some(Box::new(dc));
                }
                _ => {
                    if current.is_none() {
                        current = Some(self.start_implicit_model(line.number)?);
                    }
                    let network = current.as_mut().expect("current network");
                    self.parse_model_statement(network, &mut state, &line, &tokens)?;
                }
            }

            if networks.len() > self.limits.max_models {
                return Err(BlifReadError::LimitExceeded {
                    kind: "model",
                    limit: self.limits.max_models,
                });
            }
        }

        if let Some(network) = current {
            networks.push(network);
        }
        if networks.is_empty() && self.search_files.is_empty() {
            return Err(BlifReadError::NoNetwork);
        }

        Ok(BlifRead {
            networks,
            search_files: self.search_files.clone(),
            warnings: self.warnings.clone(),
        })
    }

    fn parse_external_dc(&mut self, parent_name: String) -> BlifResult<BlifNetwork> {
        let name = format!("{parent_name}.exdc");
        let mut network = BlifNetwork::new(name);
        let mut state = ModelState::default();

        while let Some(line) = self.current().cloned() {
            let tokens = tokenize(&line.text);
            if tokens.is_empty() {
                self.index += 1;
                continue;
            }
            let head = tokens[0];
            if head == ".end" {
                self.index += 1;
                break;
            }
            if head == ".model" || head == ".circuit" {
                let model_name = required_token(&tokens, 1, ".model", line.number)?;
                network.name = model_name.to_string();
                self.index += 1;
                continue;
            }
            if head == ".exdc" {
                return Err(BlifReadError::UnexpectedStatement {
                    line: line.number,
                    statement: line.text,
                });
            }
            self.parse_model_statement(&mut network, &mut state, &line, &tokens)?;
        }

        Ok(network)
    }

    fn parse_model_statement(
        &mut self,
        network: &mut BlifNetwork,
        state: &mut ModelState,
        line: &LogicalLine,
        tokens: &[&str],
    ) -> BlifResult<()> {
        match tokens[0] {
            ".inputs" => {
                for token in &tokens[1..] {
                    validate_name(token, self.limits, line.number)?;
                    insert_unique(&mut state.inputs, token, "input", line.number)?;
                    state.symbols.insert((*token).to_string());
                    network.inputs.push((*token).to_string());
                }
                self.index += 1;
            }
            ".outputs" => {
                for token in &tokens[1..] {
                    validate_name(token, self.limits, line.number)?;
                    insert_unique(&mut state.outputs, token, "output", line.number)?;
                    state.symbols.insert((*token).to_string());
                    network.outputs.push((*token).to_string());
                }
                self.index += 1;
            }
            ".names" => {
                self.parse_names(network, state, line, tokens, 0)?;
            }
            ".cover" => {
                self.parse_cover(network, state, line, tokens)?;
            }
            ".latch" => {
                network.latches.push(parse_latch(tokens, line.number)?);
                self.index += 1;
            }
            ".mlatch" => {
                network.gates.push(parse_gate(tokens, line.number, true)?);
                self.index += 1;
            }
            ".gate" => {
                network.gates.push(parse_gate(tokens, line.number, false)?);
                self.index += 1;
            }
            ".subckt" => {
                network.subckts.push(parse_subckt(tokens, line.number)?);
                self.index += 1;
            }
            ".start_kiss" => {
                network.kiss_blocks.push(self.parse_kiss_block()?);
            }
            ".clock" | ".clocks" => {
                let mut existing_clocks = network.clocks.iter().cloned().collect::<BTreeSet<_>>();
                for token in &tokens[1..] {
                    validate_name(token, self.limits, line.number)?;
                    insert_unique(&mut existing_clocks, token, "clock", line.number)?;
                    if state.inputs.insert((*token).to_string()) {
                        network.inputs.push((*token).to_string());
                    }
                    state.symbols.insert((*token).to_string());
                    network.clocks.push((*token).to_string());
                }
                self.index += 1;
            }
            ".clock_event" => {
                network
                    .clock_events
                    .push(parse_clock_event(tokens, line.number)?);
                self.index += 1;
            }
            ".latch_order" => {
                network
                    .latch_order
                    .extend(tokens[1..].iter().map(|token| (*token).to_string()));
                self.index += 1;
            }
            ".code" => {
                if tokens.len() != 3 {
                    return Err(BlifReadError::MissingArgument {
                        directive: ".code",
                        line: line.number,
                    });
                }
                network
                    .state_codes
                    .push((tokens[1].to_string(), tokens[2].to_string()));
                self.index += 1;
            }
            ".cycle" => {
                let value = required_token(tokens, 1, ".cycle", line.number)?;
                network.cycle_time = Some(parse_f64(value, ".cycle", line.number)?);
                self.index += 1;
            }
            _ => {
                self.warnings.push(format!(
                    "line {}: skipped unsupported directive `{}`",
                    line.number, tokens[0]
                ));
                self.index += 1;
            }
        }

        Ok(())
    }

    fn parse_kiss_block(&mut self) -> BlifResult<KissBlock> {
        self.index += 1;
        let mut lines = Vec::new();
        while let Some(line) = self.current().cloned() {
            if line.text == ".end_kiss" {
                self.index += 1;
                return Ok(KissBlock { lines });
            }
            lines.push(line.text);
            self.index += 1;
        }

        Ok(KissBlock { lines })
    }

    fn parse_names(
        &mut self,
        network: &mut BlifNetwork,
        state: &mut ModelState,
        line: &LogicalLine,
        tokens: &[&str],
        declared_terms: usize,
    ) -> BlifResult<()> {
        if tokens.len() < 2 {
            return Err(BlifReadError::MissingArgument {
                directive: ".names",
                line: line.number,
            });
        }

        let output = tokens[tokens.len() - 1].to_string();
        validate_output_definition(&output, state, line.number)?;
        let fanins = tokens[1..tokens.len() - 1]
            .iter()
            .map(|token| (*token).to_string())
            .collect::<Vec<_>>();
        for fanin in &fanins {
            validate_name(fanin, self.limits, line.number)?;
            state.symbols.insert(fanin.clone());
        }

        let mut cover = Vec::new();
        self.index += 1;
        while let Some(row_line) = self.current().cloned() {
            if row_line.text.starts_with('.') {
                break;
            }
            let row_tokens = tokenize(&row_line.text);
            parse_cover_row(
                &row_tokens,
                fanins.len(),
                &mut cover,
                row_line.number,
                &row_line.text,
            )?;
            if cover.len() > self.limits.max_cover_rows {
                return Err(BlifReadError::LimitExceeded {
                    kind: "cover row",
                    limit: self.limits.max_cover_rows,
                });
            }
            self.index += 1;
        }

        if declared_terms != 0 && declared_terms != cover.len() {
            self.warnings.push(format!(
                "line {}: .cover declared {} rows but parsed {}",
                line.number,
                declared_terms,
                cover.len()
            ));
        }

        state.driven.insert(output.clone());
        state.symbols.insert(output.clone());
        network.nodes.push(BlifNode {
            fanins,
            output,
            cover,
        });
        if network.nodes.len() > self.limits.max_nodes {
            return Err(BlifReadError::LimitExceeded {
                kind: "node",
                limit: self.limits.max_nodes,
            });
        }

        Ok(())
    }

    fn parse_cover(
        &mut self,
        network: &mut BlifNetwork,
        state: &mut ModelState,
        line: &LogicalLine,
        tokens: &[&str],
    ) -> BlifResult<()> {
        if tokens.len() < 5 {
            return Err(BlifReadError::InvalidCoverHeader { line: line.number });
        }
        let nin = parse_usize(tokens[1], ".cover", line.number)?;
        let nout = parse_usize(tokens[2], ".cover", line.number)?;
        let nterm = parse_usize(tokens[3], ".cover", line.number)?;
        if nout != 1 {
            return Err(BlifReadError::MultiOutputCover {
                line: line.number,
                outputs: nout,
            });
        }
        if tokens.len() != 4 + nin + nout {
            return Err(BlifReadError::InvalidCoverHeader { line: line.number });
        }

        let names = [&[".names"], &tokens[4..]].concat();
        self.parse_names(network, state, line, &names, nterm)
    }

    fn start_model(&mut self, name: &str, line: usize) -> BlifResult<BlifNetwork> {
        validate_name(name, self.limits, line)?;
        if !self.model_names.insert(name.to_string()) {
            return Err(BlifReadError::DuplicateModel {
                line,
                name: name.to_string(),
            });
        }
        Ok(BlifNetwork::new(name.to_string()))
    }

    fn start_implicit_model(&mut self, line: usize) -> BlifResult<BlifNetwork> {
        if self.fake_models != 0 {
            return Err(BlifReadError::DuplicateModel {
                line,
                name: "implicit".to_string(),
            });
        }
        self.fake_models += 1;
        self.start_model("implicit", line)
    }

    fn current(&self) -> Option<&LogicalLine> {
        self.lines.get(self.index)
    }
}

fn preprocess_lines(input: &str, limits: BlifReadLimits) -> BlifResult<Vec<LogicalLine>> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_start = 1;

    for (physical_index, raw) in input.lines().enumerate() {
        let line_number = physical_index + 1;
        if lines.len() >= limits.max_lines {
            return Err(BlifReadError::LimitExceeded {
                kind: "line",
                limit: limits.max_lines,
            });
        }
        if raw.len() > limits.max_line_length {
            return Err(BlifReadError::LimitExceeded {
                kind: "line length",
                limit: limits.max_line_length,
            });
        }

        if current.is_empty() {
            current_start = line_number;
        }
        let mut text = raw;
        let continued = text.ends_with('\\');
        if continued {
            text = &text[..text.len() - 1];
        }
        if let Some(comment) = text.find('#') {
            text = &text[..comment];
        }
        current.push_str(text);
        if continued {
            current.push(' ');
            continue;
        }

        if let Some(normalized) = normalize_blif_line(&current) {
            lines.push(LogicalLine {
                number: current_start,
                text: normalized,
            });
        }
        current.clear();
    }

    if !current.is_empty() {
        if let Some(normalized) = normalize_blif_line(&current) {
            lines.push(LogicalLine {
                number: current_start,
                text: normalized,
            });
        }
    }

    Ok(lines)
}

fn normalize_blif_line(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn tokenize(value: &str) -> Vec<&str> {
    value.split_whitespace().collect()
}

fn required_token<'a>(
    tokens: &'a [&'a str],
    index: usize,
    directive: &'static str,
    line: usize,
) -> BlifResult<&'a str> {
    tokens
        .get(index)
        .copied()
        .ok_or(BlifReadError::MissingArgument { directive, line })
}

fn validate_name(name: &str, limits: BlifReadLimits, line: usize) -> BlifResult<()> {
    if name.is_empty() {
        return Err(BlifReadError::MissingArgument {
            directive: "name",
            line,
        });
    }
    if name.len() > limits.max_name_length {
        return Err(BlifReadError::LimitExceeded {
            kind: "name length",
            limit: limits.max_name_length,
        });
    }
    Ok(())
}

fn insert_unique(
    values: &mut BTreeSet<String>,
    value: &str,
    kind: &'static str,
    line: usize,
) -> BlifResult<()> {
    if !values.insert(value.to_string()) {
        return Err(BlifReadError::DuplicateName {
            line,
            kind,
            name: value.to_string(),
        });
    }
    Ok(())
}

fn validate_output_definition(output: &str, state: &ModelState, line: usize) -> BlifResult<()> {
    if state.inputs.contains(output) || state.driven.contains(output) {
        return Err(BlifReadError::MultiplyDefinedOutput {
            line,
            name: output.to_string(),
        });
    }
    Ok(())
}

fn parse_cover_row(
    tokens: &[&str],
    fanin_count: usize,
    cover: &mut Vec<BlifCoverRow>,
    line: usize,
    raw: &str,
) -> BlifResult<()> {
    if fanin_count == 0 {
        if tokens.len() != 1 {
            return Err(BlifReadError::InvalidCoverRow {
                line,
                row: raw.to_string(),
            });
        }
        match tokens[0] {
            "1" => cover.push(BlifCoverRow::new(Vec::new(), true)),
            "0" => {}
            _ => {
                return Err(BlifReadError::InvalidCoverRow {
                    line,
                    row: raw.to_string(),
                });
            }
        }
        return Ok(());
    }

    if tokens.len() != 2 || tokens[0].chars().count() != fanin_count {
        return Err(BlifReadError::InvalidCoverRow {
            line,
            row: raw.to_string(),
        });
    }

    let output_value = match tokens[1] {
        "1" => true,
        "0" => false,
        _ => {
            return Err(BlifReadError::InvalidCoverRow {
                line,
                row: raw.to_string(),
            });
        }
    };
    if cover
        .iter()
        .any(|existing| existing.output_value != output_value)
    {
        return Err(BlifReadError::MixedCoverPolarity { line });
    }

    let mut literals = Vec::with_capacity(fanin_count);
    for value in tokens[0].chars() {
        literals.push(
            BlifLiteral::parse(value).ok_or_else(|| BlifReadError::InvalidCoverRow {
                line,
                row: raw.to_string(),
            })?,
        );
    }
    cover.push(BlifCoverRow::new(literals, output_value));
    Ok(())
}

fn parse_latch(tokens: &[&str], line: usize) -> BlifResult<BlifLatch> {
    if tokens.len() < 3 {
        return Err(BlifReadError::InvalidLatch {
            line,
            reason: "input and output must be specified".to_string(),
        });
    }
    if tokens.len() > 6 {
        return Err(BlifReadError::InvalidLatch {
            line,
            reason: "too many parameters".to_string(),
        });
    }

    let mut latch_type = LatchType::Unknown;
    let mut control = None;
    let mut initial_value = None;

    match tokens.len() {
        3 => {}
        4 => {
            initial_value = Some(parse_latch_value(tokens[3], line)?);
        }
        5 => {
            latch_type = parse_latch_type(tokens[3], line)?;
            control = parse_optional_node(tokens[4]);
        }
        6 => {
            latch_type = parse_latch_type(tokens[3], line)?;
            control = parse_optional_node(tokens[4]);
            initial_value = Some(parse_latch_value(tokens[5], line)?);
        }
        _ => {}
    }

    Ok(BlifLatch {
        input: tokens[1].to_string(),
        output: tokens[2].to_string(),
        latch_type,
        control,
        initial_value,
    })
}

fn parse_latch_type(value: &str, line: usize) -> BlifResult<LatchType> {
    match value {
        "fe" => Ok(LatchType::FallingEdge),
        "re" => Ok(LatchType::RisingEdge),
        "ah" => Ok(LatchType::ActiveHigh),
        "al" => Ok(LatchType::ActiveLow),
        "as" => Ok(LatchType::Asynchronous),
        _ => Err(BlifReadError::InvalidLatch {
            line,
            reason: "latch type must be re, fe, ah, al, or as".to_string(),
        }),
    }
}

fn parse_latch_value(value: &str, line: usize) -> BlifResult<u8> {
    let parsed = value
        .parse::<u8>()
        .map_err(|_| BlifReadError::InvalidLatch {
            line,
            reason: "latch value must be 0, 1, 2, or 3".to_string(),
        })?;
    if parsed > 3 {
        return Err(BlifReadError::InvalidLatch {
            line,
            reason: "latch value must be 0, 1, 2, or 3".to_string(),
        });
    }
    Ok(parsed)
}

fn parse_optional_node(value: &str) -> Option<String> {
    if value == "NIL" {
        None
    } else {
        Some(value.to_string())
    }
}

fn parse_subckt(tokens: &[&str], line: usize) -> BlifResult<BlifSubckt> {
    let model = required_token(tokens, 1, ".subckt", line)?.to_string();
    Ok(BlifSubckt {
        model,
        connections: parse_assignments(&tokens[2..], ".subckt", line)?,
    })
}

fn parse_gate(tokens: &[&str], line: usize, mlatch: bool) -> BlifResult<BlifGate> {
    let model = required_token(tokens, 1, ".gate", line)?.to_string();
    let mut end = tokens.len();
    let mut latch_control = None;
    let mut latch_initial_value = None;
    if mlatch {
        if tokens.len() < 5 {
            return Err(BlifReadError::MissingArgument {
                directive: ".mlatch",
                line,
            });
        }
        latch_initial_value = Some(parse_latch_value(tokens[end - 1], line)?);
        end -= 1;
        latch_control = parse_optional_node(tokens[end - 1]);
        end -= 1;
    }

    Ok(BlifGate {
        model,
        connections: parse_assignments(&tokens[2..end], ".gate", line)?,
        latch_control,
        latch_initial_value,
    })
}

fn parse_assignments(
    tokens: &[&str],
    directive: &'static str,
    line: usize,
) -> BlifResult<Vec<(String, String)>> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut formals = BTreeMap::new();
    for token in tokens {
        let Some((formal, actual)) = token.split_once('=') else {
            return Err(BlifReadError::BadAssignment {
                line,
                directive,
                value: (*token).to_string(),
            });
        };
        if formal.is_empty() || actual.is_empty() {
            return Err(BlifReadError::BadAssignment {
                line,
                directive,
                value: (*token).to_string(),
            });
        }
        if formals.insert(formal.to_string(), ()).is_some() {
            return Err(BlifReadError::DuplicateName {
                line,
                kind: "formal",
                name: formal.to_string(),
            });
        }
        result.push((formal.to_string(), actual.to_string()));
    }
    Ok(result)
}

fn parse_clock_event(tokens: &[&str], line: usize) -> BlifResult<ClockEvent> {
    let nominal = parse_f64(
        required_token(tokens, 1, ".clock_event", line)?,
        ".clock_event",
        line,
    )?;
    let mut edges = Vec::new();
    let mut index = 2;
    while index < tokens.len() {
        let token = tokens[index];
        let (lower_range, upper_range);
        let edge_token;
        if token == "(" {
            if index + 3 >= tokens.len() || tokens[index + 3] != ")" {
                return Err(BlifReadError::InvalidClockEvent {
                    line,
                    reason: "range must be `( min max )`".to_string(),
                });
            }
            lower_range = Some(parse_f64(tokens[index + 1], ".clock_event", line)?);
            upper_range = Some(parse_f64(tokens[index + 2], ".clock_event", line)?);
            edge_token = required_token(tokens, index + 4, ".clock_event", line)?;
            index += 5;
        } else {
            lower_range = None;
            upper_range = None;
            edge_token = token;
            index += 1;
        }

        let mut chars = edge_token.chars();
        let transition = match chars.next() {
            Some('r') => ClockTransition::Rise,
            Some('f') => ClockTransition::Fall,
            _ => {
                return Err(BlifReadError::InvalidClockEvent {
                    line,
                    reason: "transition must start with r or f".to_string(),
                });
            }
        };
        let clock = edge_token
            .strip_prefix("r:")
            .or_else(|| edge_token.strip_prefix("f:"))
            .ok_or_else(|| BlifReadError::InvalidClockEvent {
                line,
                reason: "clock edge must have form r:name or f:name".to_string(),
            })?;
        if clock.is_empty() {
            return Err(BlifReadError::InvalidClockEvent {
                line,
                reason: "clock name is missing".to_string(),
            });
        }
        edges.push(ClockEventEdge {
            transition,
            clock: clock.to_string(),
            lower_range,
            upper_range,
        });
    }

    Ok(ClockEvent {
        nominal_position: nominal,
        edges,
    })
}

fn parse_usize(value: &str, directive: &'static str, line: usize) -> BlifResult<usize> {
    value.parse().map_err(|_| BlifReadError::InvalidNumber {
        line,
        directive,
        value: value.to_string(),
    })
}

fn parse_f64(value: &str, directive: &'static str, line: usize) -> BlifResult<f64> {
    value.parse().map_err(|_| BlifReadError::InvalidNumber {
        line,
        directive,
        value: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(values: &[BlifLiteral]) -> Vec<BlifLiteral> {
        values.to_vec()
    }

    #[test]
    fn preprocesses_comments_whitespace_and_continuations() {
        let lines = preprocess_blif_lines(
            "  .model   demo   # comment\n.inputs a \\\n b\n\n.names a b f\n1- 1\n",
        )
        .expect("preprocessed");

        assert_eq!(
            lines,
            vec![".model demo", ".inputs a b", ".names a b f", "1- 1"]
        );
    }

    #[test]
    fn parses_combinational_model() {
        let read = read_blif(concat!(
            ".model demo\n",
            ".inputs a b\n",
            ".outputs f\n",
            ".names a b f\n",
            "11 1\n",
            "0- 1\n",
            ".end\n"
        ))
        .expect("parsed");

        let network = read.first_network().expect("network");
        assert_eq!(network.name, "demo");
        assert_eq!(network.inputs, ["a", "b"]);
        assert_eq!(network.outputs, ["f"]);
        assert_eq!(network.nodes.len(), 1);
        assert_eq!(network.nodes[0].fanin_count(), 2);
        assert_eq!(
            network.nodes[0].cover[1],
            BlifCoverRow::new(lit(&[BlifLiteral::Zero, BlifLiteral::DontCare]), true)
        );
    }

    #[test]
    fn parses_cover_directive_with_declared_rows() {
        let read = read_blif(concat!(
            ".model demo\n",
            ".inputs a b\n",
            ".outputs f\n",
            ".cover 2 1 2 a b f\n",
            "10 1\n",
            "2- 1\n",
            ".end\n"
        ))
        .expect("parsed");

        let cover = &read.networks[0].nodes[0].cover;
        assert_eq!(cover.len(), 2);
        assert_eq!(
            cover[1].literals,
            lit(&[BlifLiteral::DontCare, BlifLiteral::DontCare])
        );
    }

    #[test]
    fn parses_constant_nodes() {
        let read = read_blif(".outputs one zero\n.names one\n1\n.names zero\n0\n").expect("parsed");

        assert_eq!(read.networks[0].nodes.len(), 2);
        assert_eq!(read.networks[0].nodes[0].cover.len(), 1);
        assert!(read.networks[0].nodes[1].cover.is_empty());
    }

    #[test]
    fn rejects_mixed_on_and_off_set_rows() {
        let err =
            read_blif(".model demo\n.names a f\n1 1\n0 0\n").expect_err("mixed polarity rejected");

        assert!(matches!(err, BlifReadError::MixedCoverPolarity { .. }));
    }

    #[test]
    fn rejects_multiply_defined_outputs() {
        let err = read_blif(".model demo\n.inputs a\n.names a\n1\n")
            .expect_err("input redefinition rejected");

        assert!(matches!(err, BlifReadError::MultiplyDefinedOutput { .. }));
    }

    #[test]
    fn parses_latches_and_sequential_metadata() {
        let read = read_blif(concat!(
            ".model seq\n",
            ".inputs d clk\n",
            ".outputs q\n",
            ".latch d q re clk 1\n",
            ".latch_order q\n",
            ".cycle 4.5\n",
            ".clock clk\n",
            ".clock_event 1.0 r:clk f:clk\n",
            ".code s0 0\n",
            ".end\n"
        ))
        .expect("parsed");

        let network = &read.networks[0];
        assert_eq!(network.latches[0].latch_type, LatchType::RisingEdge);
        assert_eq!(network.latches[0].control.as_deref(), Some("clk"));
        assert_eq!(network.latches[0].initial_value, Some(1));
        assert_eq!(network.latch_order, ["q"]);
        assert_eq!(network.cycle_time, Some(4.5));
        assert_eq!(network.clock_events[0].edges.len(), 2);
        assert_eq!(network.state_codes[0], ("s0".to_string(), "0".to_string()));
    }

    #[test]
    fn parses_subckt_gate_and_mlatch_assignments() {
        let read = read_blif(concat!(
            ".model top\n",
            ".subckt child in=a out=f\n",
            ".gate nand2 A=a B=b Y=n\n",
            ".mlatch latch D=d Q=q clk 3\n",
            ".end\n"
        ))
        .expect("parsed");

        let network = &read.networks[0];
        assert_eq!(network.subckts[0].model, "child");
        assert_eq!(
            network.gates[0].connections[2],
            ("Y".to_string(), "n".to_string())
        );
        assert_eq!(network.gates[1].latch_control.as_deref(), Some("clk"));
        assert_eq!(network.gates[1].latch_initial_value, Some(3));
    }

    #[test]
    fn attaches_external_dc_network() {
        let read = read_blif(concat!(
            ".model top\n",
            ".inputs a\n",
            ".outputs f\n",
            ".names a f\n",
            "1 1\n",
            ".exdc\n",
            ".inputs a\n",
            ".outputs f\n",
            ".names a f\n",
            "0 1\n",
            ".end\n"
        ))
        .expect("parsed");

        let dc = read.networks[0].external_dc.as_ref().expect("dc network");
        assert_eq!(dc.name, "top.exdc");
        assert_eq!(dc.nodes[0].cover[0].literals, lit(&[BlifLiteral::Zero]));
    }

    #[test]
    fn preserves_embedded_kiss_block() {
        let read = read_blif(concat!(
            ".model seq\n",
            ".start_kiss\n",
            ".i 1\n",
            ".o 1\n",
            "0 s0 s1 1\n",
            ".end_kiss\n",
            ".end\n"
        ))
        .expect("parsed");

        assert_eq!(
            read.networks[0].kiss_blocks[0].lines,
            [".i 1", ".o 1", "0 s0 s1 1"]
        );
    }

    #[test]
    fn read_first_stops_before_later_models() {
        let network = read_blif_first(".model one\n.end\n.model two\n.end\n").expect("first");

        assert_eq!(network.name, "one");
    }

    #[test]
    fn records_search_files_without_loading_them() {
        let read = read_blif(".search library.blif\n.model demo\n.end\n").expect("parsed");

        assert_eq!(read.search_files, ["library.blif"]);
    }

    trait NodeTestExt {
        fn fanin_count(&self) -> usize;
    }

    impl NodeTestExt for BlifNode {
        fn fanin_count(&self) -> usize {
            self.fanins.len()
        }
    }
}
