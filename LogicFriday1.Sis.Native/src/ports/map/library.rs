//! Native Rust genlib parser for `sis/map/library.c`.
//!
//! The original SIS mapper reads `.genlib` gate libraries with commands such
//! as `read_library file.genlib` and `read_library -n file.genlib`. This module
//! keeps that behavior as owned Rust data: callers choose a pin-name policy in
//! `ReadLibraryOptions`, and parsing produces a self-contained `GenlibLibrary`
//! without legacy C ABI entry points.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseLimits {
    pub max_records: usize,
    pub max_record_length: usize,
    pub max_name_length: usize,
    pub max_gates: usize,
    pub max_pins_per_gate: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_records: 65_536,
            max_record_length: 16_384,
            max_name_length: 256,
            max_gates: 16_384,
            max_pins_per_gate: 256,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinNamePolicy {
    PreserveDeclared,
    GenerateByPosition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadLibraryOptions {
    pub pin_name_policy: PinNamePolicy,
}

impl ReadLibraryOptions {
    pub fn read_library() -> Self {
        Self {
            pin_name_policy: PinNamePolicy::PreserveDeclared,
        }
    }

    pub fn read_library_dash_n() -> Self {
        Self {
            pin_name_policy: PinNamePolicy::GenerateByPosition,
        }
    }
}

impl Default for ReadLibraryOptions {
    fn default() -> Self {
        Self::read_library()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenlibLibrary {
    pub gates: Vec<GenlibGate>,
    pub options: ReadLibraryOptions,
}

impl GenlibLibrary {
    pub fn new(gates: Vec<GenlibGate>, options: ReadLibraryOptions) -> Result<Self, GenlibError> {
        let library = Self { gates, options };
        library.validate()?;
        Ok(library)
    }

    pub fn validate(&self) -> Result<(), GenlibError> {
        let mut seen = HashSet::new();
        for gate in &self.gates {
            if !seen.insert(&gate.name) {
                return Err(GenlibError::DuplicateGate {
                    name: gate.name.clone(),
                });
            }
            gate.validate()?;
        }

        Ok(())
    }

    pub fn gate(&self, name: &str) -> Option<&GenlibGate> {
        self.gates.iter().find(|gate| gate.name == name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenlibGate {
    pub name: String,
    pub area: f64,
    pub output: GenlibOutput,
    pub pins: Vec<GenlibPin>,
}

impl GenlibGate {
    pub fn new(
        name: impl Into<String>,
        area: f64,
        output: GenlibOutput,
        pins: Vec<GenlibPin>,
    ) -> Result<Self, GenlibError> {
        let gate = Self {
            name: name.into(),
            area,
            output,
            pins,
        };
        gate.validate()?;
        Ok(gate)
    }

    pub fn validate(&self) -> Result<(), GenlibError> {
        validate_name(&self.name, "gate", ParseLimits::default().max_name_length)?;
        validate_name(
            &self.output.name,
            "gate output",
            ParseLimits::default().max_name_length,
        )?;
        if !self.area.is_finite() || self.area < 0.0 {
            return Err(GenlibError::InvalidNumber {
                record: 0,
                value: self.area.to_string(),
            });
        }
        for pin in &self.pins {
            pin.validate()?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenlibOutput {
    pub name: String,
    pub expression: String,
}

impl GenlibOutput {
    pub fn new(name: impl Into<String>, expression: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            expression: expression.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenlibPin {
    pub name: GenlibPinName,
    pub declared_name: String,
    pub phase: PinPhase,
    pub input_load: f64,
    pub max_load: f64,
    pub rise_block_delay: f64,
    pub rise_fanout_delay: f64,
    pub fall_block_delay: f64,
    pub fall_fanout_delay: f64,
}

impl GenlibPin {
    pub fn validate(&self) -> Result<(), GenlibError> {
        validate_name(
            &self.declared_name,
            "pin",
            ParseLimits::default().max_name_length,
        )?;
        let values = [
            self.input_load,
            self.max_load,
            self.rise_block_delay,
            self.rise_fanout_delay,
            self.fall_block_delay,
            self.fall_fanout_delay,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(GenlibError::InvalidPinTiming {
                pin: self.declared_name.clone(),
            });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenlibPinName {
    Declared(String),
    Wildcard,
    Generated(usize),
}

impl GenlibPinName {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Declared(name) => Some(name.as_str()),
            Self::Wildcard => Some("*"),
            Self::Generated(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinPhase {
    Inv,
    NonInv,
    Unknown,
}

impl PinPhase {
    fn parse(value: &str, record: usize) -> Result<Self, GenlibError> {
        match value {
            "INV" => Ok(Self::Inv),
            "NONINV" => Ok(Self::NonInv),
            "UNKNOWN" => Ok(Self::Unknown),
            _ => Err(GenlibError::InvalidPinPhase {
                record,
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GenlibError {
    RecordLimitExceeded {
        max_records: usize,
    },
    RecordTooLong {
        record: usize,
        max_record_length: usize,
    },
    ExpectedKeyword {
        record: usize,
        keyword: &'static str,
    },
    UnexpectedKeyword {
        record: usize,
        keyword: String,
    },
    MissingField {
        record: usize,
        field: &'static str,
    },
    EmptyName {
        kind: &'static str,
    },
    NameTooLong {
        kind: &'static str,
        name: String,
        max_name_length: usize,
    },
    InvalidNumber {
        record: usize,
        value: String,
    },
    InvalidOutput {
        record: usize,
        value: String,
    },
    InvalidPinPhase {
        record: usize,
        value: String,
    },
    InvalidPinTiming {
        pin: String,
    },
    TooManyGates {
        max: usize,
    },
    TooManyPins {
        gate: String,
        max: usize,
    },
    PinWithoutGate {
        record: usize,
    },
    DuplicateGate {
        name: String,
    },
}

impl fmt::Display for GenlibError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecordLimitExceeded { max_records } => {
                write!(f, "genlib input exceeds {max_records} records")
            }
            Self::RecordTooLong {
                record,
                max_record_length,
            } => write!(f, "record {record} exceeds {max_record_length} characters"),
            Self::ExpectedKeyword { record, keyword } => {
                write!(f, "record {record} expected {keyword}")
            }
            Self::UnexpectedKeyword { record, keyword } => {
                write!(f, "record {record} has unexpected keyword '{keyword}'")
            }
            Self::MissingField { record, field } => write!(f, "record {record} is missing {field}"),
            Self::EmptyName { kind } => write!(f, "{kind} name cannot be empty"),
            Self::NameTooLong {
                kind,
                name,
                max_name_length,
            } => write!(
                f,
                "{kind} name '{name}' exceeds {max_name_length} characters"
            ),
            Self::InvalidNumber { record, value } => {
                write!(f, "record {record} has invalid number '{value}'")
            }
            Self::InvalidOutput { record, value } => {
                write!(f, "record {record} has invalid gate output '{value}'")
            }
            Self::InvalidPinPhase { record, value } => {
                write!(f, "record {record} has invalid pin phase '{value}'")
            }
            Self::InvalidPinTiming { pin } => {
                write!(f, "pin '{pin}' has invalid timing or load fields")
            }
            Self::TooManyGates { max } => write!(f, "too many genlib gates; max is {max}"),
            Self::TooManyPins { gate, max } => {
                write!(f, "gate '{gate}' has too many pins; max is {max}")
            }
            Self::PinWithoutGate { record } => {
                write!(f, "record {record} contains PIN before any GATE")
            }
            Self::DuplicateGate { name } => write!(f, "duplicate gate name '{name}'"),
        }
    }
}

impl Error for GenlibError {}

pub fn parse_genlib(input: &str) -> Result<GenlibLibrary, GenlibError> {
    parse_genlib_with_options(input, ReadLibraryOptions::default(), ParseLimits::default())
}

pub fn parse_genlib_with_options(
    input: &str,
    options: ReadLibraryOptions,
    limits: ParseLimits,
) -> Result<GenlibLibrary, GenlibError> {
    let records = records(input, limits)?;
    let mut gates = Vec::<GenlibGate>::new();
    let mut current_gate: Option<GenlibGate> = None;

    for (record_number, record) in records {
        let tokens = record.split_whitespace().collect::<Vec<_>>();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "GATE" => {
                if gates.len() >= limits.max_gates {
                    return Err(GenlibError::TooManyGates {
                        max: limits.max_gates,
                    });
                }
                if let Some(gate) = current_gate.take() {
                    gates.push(gate);
                }
                current_gate = Some(parse_gate_record(&tokens, record_number, limits)?);
            }
            "PIN" => {
                let Some(gate) = current_gate.as_mut() else {
                    return Err(GenlibError::PinWithoutGate {
                        record: record_number,
                    });
                };
                if gate.pins.len() >= limits.max_pins_per_gate {
                    return Err(GenlibError::TooManyPins {
                        gate: gate.name.clone(),
                        max: limits.max_pins_per_gate,
                    });
                }
                let pin = parse_pin_record(&tokens, record_number, gate.pins.len(), options)?;
                gate.pins.push(pin);
            }
            keyword => {
                return Err(GenlibError::UnexpectedKeyword {
                    record: record_number,
                    keyword: keyword.to_string(),
                });
            }
        }
    }

    if let Some(gate) = current_gate.take() {
        gates.push(gate);
    }

    GenlibLibrary::new(gates, options)
}

fn parse_gate_record(
    tokens: &[&str],
    record: usize,
    limits: ParseLimits,
) -> Result<GenlibGate, GenlibError> {
    expect_keyword(tokens, record, "GATE")?;
    let name = field(tokens, record, 1, "gate name")?;
    validate_name(name, "gate", limits.max_name_length)?;
    let area = parse_f64(field(tokens, record, 2, "gate area")?, record)?;
    let output_text = tokens
        .get(3..)
        .filter(|values| !values.is_empty())
        .ok_or(GenlibError::MissingField {
            record,
            field: "gate output",
        })?
        .join(" ");
    let output = parse_output_assignment(&output_text, record, limits)?;

    Ok(GenlibGate {
        name: name.to_string(),
        area,
        output,
        pins: Vec::new(),
    })
}

fn parse_pin_record(
    tokens: &[&str],
    record: usize,
    position: usize,
    options: ReadLibraryOptions,
) -> Result<GenlibPin, GenlibError> {
    expect_keyword(tokens, record, "PIN")?;
    if tokens.len() != 9 {
        return Err(GenlibError::MissingField {
            record,
            field: "complete PIN fields",
        });
    }

    let declared_name = tokens[1].to_string();
    let name = match options.pin_name_policy {
        PinNamePolicy::PreserveDeclared if declared_name == "*" => GenlibPinName::Wildcard,
        PinNamePolicy::PreserveDeclared => GenlibPinName::Declared(declared_name.clone()),
        PinNamePolicy::GenerateByPosition => GenlibPinName::Generated(position),
    };

    Ok(GenlibPin {
        name,
        declared_name,
        phase: PinPhase::parse(tokens[2], record)?,
        input_load: parse_f64(tokens[3], record)?,
        max_load: parse_f64(tokens[4], record)?,
        rise_block_delay: parse_f64(tokens[5], record)?,
        rise_fanout_delay: parse_f64(tokens[6], record)?,
        fall_block_delay: parse_f64(tokens[7], record)?,
        fall_fanout_delay: parse_f64(tokens[8], record)?,
    })
}

fn records(input: &str, limits: ParseLimits) -> Result<Vec<(usize, String)>, GenlibError> {
    let mut records = Vec::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line_number = line_index + 1;
        if records.len() >= limits.max_records {
            return Err(GenlibError::RecordLimitExceeded {
                max_records: limits.max_records,
            });
        }
        if raw_line.len() > limits.max_record_length {
            return Err(GenlibError::RecordTooLong {
                record: line_number,
                max_record_length: limits.max_record_length,
            });
        }

        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        split_records(line, line_number, &mut records);
    }

    Ok(records)
}

fn split_records(line: &str, line_number: usize, records: &mut Vec<(usize, String)>) {
    let mut current = String::new();
    for token in line.split_whitespace() {
        if (token == "GATE" || token == "PIN") && !current.trim().is_empty() {
            records.push((line_number, current.trim().to_string()));
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(token);
    }

    if !current.trim().is_empty() {
        records.push((line_number, current.trim().to_string()));
    }
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .map(|(prefix, _)| prefix)
        .unwrap_or(line)
}

fn expect_keyword(
    tokens: &[&str],
    record: usize,
    keyword: &'static str,
) -> Result<(), GenlibError> {
    if tokens.first().copied() != Some(keyword) {
        return Err(GenlibError::ExpectedKeyword { record, keyword });
    }

    Ok(())
}

fn field<'a>(
    tokens: &'a [&str],
    record: usize,
    index: usize,
    name: &'static str,
) -> Result<&'a str, GenlibError> {
    tokens.get(index).copied().ok_or(GenlibError::MissingField {
        record,
        field: name,
    })
}

fn parse_output_assignment(
    value: &str,
    record: usize,
    limits: ParseLimits,
) -> Result<GenlibOutput, GenlibError> {
    let value = value.trim().trim_end_matches(';').trim();
    let Some((name, expression)) = value.split_once('=') else {
        return Err(GenlibError::InvalidOutput {
            record,
            value: value.to_string(),
        });
    };
    let name = name.trim();
    let expression = expression.trim();
    validate_name(name, "gate output", limits.max_name_length)?;
    if expression.is_empty() {
        return Err(GenlibError::InvalidOutput {
            record,
            value: value.to_string(),
        });
    }

    Ok(GenlibOutput::new(name, expression))
}

fn parse_f64(value: &str, record: usize) -> Result<f64, GenlibError> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| GenlibError::InvalidNumber {
            record,
            value: value.to_string(),
        })?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err(GenlibError::InvalidNumber {
            record,
            value: value.to_string(),
        });
    }

    Ok(parsed)
}

fn validate_name(
    name: &str,
    kind: &'static str,
    max_name_length: usize,
) -> Result<(), GenlibError> {
    if name.is_empty() {
        return Err(GenlibError::EmptyName { kind });
    }
    if name.len() > max_name_length {
        return Err(GenlibError::NameTooLong {
            kind,
            name: name.to_string(),
            max_name_length,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gate_declarations_and_wildcard_pin() {
        let library = parse_genlib(concat!(
            "GATE zero0 0 O=CONST0;\n",
            "GATE inv1 1 O=!a; PIN * INV 1 999 0.9 0.3 0.9 0.3\n",
        ))
        .unwrap();

        assert_eq!(library.gates.len(), 2);
        assert_eq!(library.gates[0].name, "zero0");
        assert_eq!(library.gates[0].output, GenlibOutput::new("O", "CONST0"));
        assert_eq!(library.gates[1].name, "inv1");
        assert_eq!(library.gates[1].area, 1.0);
        assert_eq!(library.gates[1].output, GenlibOutput::new("O", "!a"));
        assert_eq!(library.gates[1].pins.len(), 1);
        assert_eq!(library.gates[1].pins[0].name, GenlibPinName::Wildcard);
        assert_eq!(library.gates[1].pins[0].phase, PinPhase::Inv);
    }

    #[test]
    fn parses_explicit_pin_load_and_delay_fields() {
        let library = parse_genlib(concat!(
            "GATE mux2 4 O=1D1*!3SEL+2D2*3SEL;\n",
            "PIN 1D1 NONINV 1 999 1 .2 1 .2\n",
            "PIN 2D2 NONINV 1.5 50 1.1 .25 1.2 .3\n",
            "PIN 3SEL UNKNOWN 2 75 1.3 .4 1.5 .6\n",
        ))
        .unwrap();

        let gate = library.gate("mux2").unwrap();
        assert_eq!(gate.pins.len(), 3);
        assert_eq!(
            gate.pins[0].name,
            GenlibPinName::Declared("1D1".to_string())
        );
        assert_eq!(gate.pins[0].input_load, 1.0);
        assert_eq!(gate.pins[0].max_load, 999.0);
        assert_eq!(gate.pins[1].rise_block_delay, 1.1);
        assert_eq!(gate.pins[1].rise_fanout_delay, 0.25);
        assert_eq!(gate.pins[2].fall_block_delay, 1.5);
        assert_eq!(gate.pins[2].fall_fanout_delay, 0.6);
    }

    #[test]
    fn read_library_dash_n_uses_owned_generated_pin_names() {
        let library = parse_genlib_with_options(
            concat!(
                "GATE mux2 4 O=1D1*!3SEL+2D2*3SEL;\n",
                "PIN 1D1 NONINV 1 999 1 .2 1 .2\n",
                "PIN 2D2 NONINV 1 999 1 .2 1 .2\n",
                "PIN 3SEL UNKNOWN 1 999 1 .2 1 .2\n",
            ),
            ReadLibraryOptions::read_library_dash_n(),
            ParseLimits::default(),
        )
        .unwrap();

        let pins = &library.gate("mux2").unwrap().pins;
        assert_eq!(library.options, ReadLibraryOptions::read_library_dash_n());
        assert_eq!(pins[0].declared_name, "1D1");
        assert_eq!(pins[0].name, GenlibPinName::Generated(0));
        assert_eq!(pins[1].name, GenlibPinName::Generated(1));
        assert_eq!(pins[2].name, GenlibPinName::Generated(2));
    }

    #[test]
    fn rejects_pin_before_gate_duplicate_gate_and_bad_numbers() {
        assert_eq!(
            parse_genlib("PIN * INV 1 999 1 .2 1 .2\n").unwrap_err(),
            GenlibError::PinWithoutGate { record: 1 }
        );

        assert_eq!(
            parse_genlib("GATE inv1 1 O=!a;\nGATE inv1 2 O=!b;\n").unwrap_err(),
            GenlibError::DuplicateGate {
                name: "inv1".to_string(),
            }
        );

        assert!(matches!(
            parse_genlib("GATE inv1 nope O=!a;\n").unwrap_err(),
            GenlibError::InvalidNumber { .. }
        ));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("library.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
