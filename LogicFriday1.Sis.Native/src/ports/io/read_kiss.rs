//! Native Rust reader for SIS KISS state-transition graphs.
//!
//! The legacy `sis/io/read_kiss.c` skips comments and leading whitespace,
//! reads `.i`, `.o`, optional `.p`, `.s`, and `.r` headers, then builds an STG
//! from four-column transition rows. This port keeps that behavior in an owned
//! Rust model so callers can adapt it to native STG/network structures without
//! adding a per-file C ABI shim.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct KissStateId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KissState {
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KissTransition {
    pub from: KissStateId,
    pub to: KissStateId,
    pub input: String,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KissStateGraph {
    input_count: usize,
    output_count: usize,
    states: Vec<KissState>,
    transitions: Vec<KissTransition>,
    start: KissStateId,
    current: KissStateId,
}

impl KissStateGraph {
    pub fn input_count(&self) -> usize {
        self.input_count
    }

    pub fn output_count(&self) -> usize {
        self.output_count
    }

    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    pub fn product_count(&self) -> usize {
        self.transitions.len()
    }

    pub fn states(&self) -> &[KissState] {
        &self.states
    }

    pub fn transitions(&self) -> &[KissTransition] {
        &self.transitions
    }

    pub fn start(&self) -> KissStateId {
        self.start
    }

    pub fn current(&self) -> KissStateId {
        self.current
    }

    pub fn state_name(&self, state: KissStateId) -> Option<&str> {
        self.states.get(state.0).map(|state| state.name.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KissRead {
    pub graph: KissStateGraph,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KissReadError {
    MissingHeader {
        header: &'static str,
    },
    DuplicateHeader {
        line: usize,
        option: char,
    },
    InvalidHeader {
        line: usize,
        message: String,
    },
    InvalidTransition {
        line: usize,
        text: String,
    },
    InvalidTerminator {
        line: usize,
        text: String,
    },
    InputWidth {
        line: usize,
        expected: usize,
        actual: usize,
        text: String,
    },
    OutputWidth {
        line: usize,
        expected: usize,
        actual: usize,
        text: String,
    },
    NoTransitions,
    InvalidStartState {
        name: String,
    },
}

impl fmt::Display for KissReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader { header } => write!(formatter, "{header} not specified in header"),
            Self::DuplicateHeader { line, option } => {
                write!(
                    formatter,
                    "line {line}: header option .{option} specified twice"
                )
            }
            Self::InvalidHeader { line, message } => write!(formatter, "line {line}: {message}"),
            Self::InvalidTransition { line, text } => {
                write!(formatter, "line {line}: invalid line: {text}")
            }
            Self::InvalidTerminator { line, .. } => write!(
                formatter,
                "line {line}: kiss input must end with .end_kiss; reading aborted"
            ),
            Self::InputWidth {
                line,
                expected,
                actual,
                ..
            } => write!(
                formatter,
                "line {line}: invalid number of inputs: expected {expected}, got {actual}"
            ),
            Self::OutputWidth {
                line,
                expected,
                actual,
                ..
            } => write!(
                formatter,
                "line {line}: invalid number of outputs: expected {expected}, got {actual}"
            ),
            Self::NoTransitions => write!(formatter, "kiss input has no transition rows"),
            Self::InvalidStartState { name } => {
                write!(formatter, "start state {name} is not a valid state")
            }
        }
    }
}

impl Error for KissReadError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct KissLine {
    number: usize,
    text: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Headers {
    inputs: Option<usize>,
    products: Option<usize>,
    states: Option<usize>,
    outputs: Option<usize>,
}

pub fn read_kiss(input: &str) -> Result<KissRead, KissReadError> {
    let lines = scan_kiss_lines(input);
    let mut index = 0;
    let mut headers = Headers::default();
    let mut reset = None;
    let mut warnings = Vec::new();

    while let Some(line) = lines.get(index) {
        if !line.text.starts_with('.') {
            break;
        }

        if line.text.starts_with(".r") {
            let tokens = tokens(&line.text);
            if tokens.len() != 2 {
                return Err(KissReadError::InvalidHeader {
                    line: line.number,
                    message: "missing argument to header option .r".to_owned(),
                });
            }
            reset = Some(tokens[1].to_owned());
        } else if is_count_header(&line.text) {
            let option = line.text.as_bytes()[1] as char;
            let value = parse_count_header(line, option)?;
            match option {
                'i' => set_header(&mut headers.inputs, value, line.number, option)?,
                'p' => set_header(&mut headers.products, value, line.number, option)?,
                's' => set_header(&mut headers.states, value, line.number, option)?,
                'o' => set_header(&mut headers.outputs, value, line.number, option)?,
                _ => {
                    return Err(KissReadError::InvalidHeader {
                        line: line.number,
                        message: format!("invalid header option .{option}"),
                    });
                }
            }
        } else {
            warnings.push(format!(
                "ignored header line {}: {}",
                line.number, line.text
            ));
        }

        index += 1;
    }

    if headers.inputs.is_none() {
        return Err(KissReadError::MissingHeader { header: "inputs" });
    }
    if headers.outputs.is_none() {
        return Err(KissReadError::MissingHeader { header: "outputs" });
    }

    let mut builder = GraphBuilder::default();
    let mut observed_inputs = None;
    let mut observed_outputs = None;

    while let Some(line) = lines.get(index) {
        if line.text.starts_with('.') {
            if line.text.starts_with(".e") {
                break;
            }

            return Err(KissReadError::InvalidTerminator {
                line: line.number,
                text: line.text.clone(),
            });
        }

        let fields = tokens(&line.text);
        if fields.len() != 4 {
            return Err(KissReadError::InvalidTransition {
                line: line.number,
                text: line.text.clone(),
            });
        }

        let input_width = fields[0].len();
        let output_width = fields[3].len();
        if let Some(expected) = observed_inputs {
            if input_width != expected {
                return Err(KissReadError::InputWidth {
                    line: line.number,
                    expected,
                    actual: input_width,
                    text: line.text.clone(),
                });
            }
        } else {
            observed_inputs = Some(input_width);
        }

        if let Some(expected) = observed_outputs {
            if output_width != expected {
                return Err(KissReadError::OutputWidth {
                    line: line.number,
                    expected,
                    actual: output_width,
                    text: line.text.clone(),
                });
            }
        } else {
            observed_outputs = Some(output_width);
        }

        let from = builder.get_or_add_state(fields[1]);
        let to = builder.get_or_add_state(fields[2]);
        if builder.transitions.is_empty() && reset.is_none() {
            reset = Some(fields[1].to_owned());
        }

        builder.transitions.push(KissTransition {
            from,
            to,
            input: fields[0].to_owned(),
            output: fields[3].to_owned(),
        });

        index += 1;
    }

    let input_count = observed_inputs.ok_or(KissReadError::NoTransitions)?;
    let output_count = observed_outputs.expect("transition row sets output count");

    if headers.inputs != Some(input_count) {
        warnings.push("Number of inputs given is not correct.".to_owned());
        warnings.push(".i line ignored".to_owned());
    }
    if headers.outputs != Some(output_count) {
        warnings.push("Number of outputs given is not correct.".to_owned());
        warnings.push(".o line ignored".to_owned());
    }

    let reset = reset.expect("first transition supplies reset when .r is absent");
    let start = resolve_start(&builder, &reset)?;

    Ok(KissRead {
        graph: KissStateGraph {
            input_count,
            output_count,
            states: builder.states,
            transitions: builder.transitions,
            start,
            current: start,
        },
        warnings,
    })
}

pub fn read_kiss_graph(input: &str) -> Result<KissStateGraph, KissReadError> {
    read_kiss(input).map(|read| read.graph)
}

fn scan_kiss_lines(input: &str) -> Vec<KissLine> {
    let mut lines = Vec::new();

    for (offset, raw) in input.lines().enumerate() {
        let text = raw.trim_start();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }

        let text = text.trim_end();
        if text.is_empty() {
            continue;
        }

        lines.push(KissLine {
            number: offset + 1,
            text: text.to_owned(),
        });
    }

    lines
}

fn is_count_header(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() > 2 && bytes[0] == b'.' && bytes[2].is_ascii_whitespace()
}

fn parse_count_header(line: &KissLine, option: char) -> Result<usize, KissReadError> {
    let fields = tokens(&line.text);
    if fields.len() != 2 {
        return Err(KissReadError::InvalidHeader {
            line: line.number,
            message: format!("missing/bad argument to header option .{option}"),
        });
    }

    match fields[1].parse::<usize>() {
        Ok(value) if value > 0 => Ok(value),
        _ => Err(KissReadError::InvalidHeader {
            line: line.number,
            message: format!("missing/bad argument to header option .{option}"),
        }),
    }
}

fn set_header(
    target: &mut Option<usize>,
    value: usize,
    line: usize,
    option: char,
) -> Result<(), KissReadError> {
    if target.is_some() {
        return Err(KissReadError::DuplicateHeader { line, option });
    }

    *target = Some(value);
    Ok(())
}

fn tokens(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

#[derive(Clone, Debug, Default)]
struct GraphBuilder {
    states: Vec<KissState>,
    transitions: Vec<KissTransition>,
    names: HashMap<String, KissStateId>,
}

impl GraphBuilder {
    fn get_or_add_state(&mut self, name: &str) -> KissStateId {
        if let Some(state) = self.names.get(name) {
            return *state;
        }

        let state = KissStateId(self.states.len());
        self.states.push(KissState {
            name: name.to_owned(),
        });
        self.names.insert(name.to_owned(), state);
        state
    }
}

fn resolve_start(builder: &GraphBuilder, name: &str) -> Result<KissStateId, KissReadError> {
    if name == "ANY" || name == "*" {
        builder
            .states
            .iter()
            .enumerate()
            .find(|(_, state)| state.name != "ANY" && state.name != "*")
            .map(|(index, _)| KissStateId(index))
            .ok_or_else(|| KissReadError::InvalidStartState {
                name: name.to_owned(),
            })
    } else {
        builder
            .names
            .get(name)
            .copied()
            .ok_or_else(|| KissReadError::InvalidStartState {
                name: name.to_owned(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headers_reset_and_transition_table() {
        let read = read_kiss(
            "
            .i 2
            .o 1
            .p 2
            .s 2
            .r S0
            00 S0 S1 0
            1- S1 S0 1
            .end_kiss
            ",
        )
        .unwrap();

        assert_eq!(read.graph.input_count(), 2);
        assert_eq!(read.graph.output_count(), 1);
        assert_eq!(read.graph.state_count(), 2);
        assert_eq!(read.graph.product_count(), 2);
        assert_eq!(read.graph.state_name(read.graph.start()), Some("S0"));
        assert_eq!(
            read.graph.transitions()[0],
            KissTransition {
                from: KissStateId(0),
                to: KissStateId(1),
                input: "00".to_owned(),
                output: "0".to_owned()
            }
        );
        assert!(read.warnings.is_empty());
    }

    #[test]
    fn skips_comments_and_uses_first_state_as_default_reset() {
        let graph = read_kiss_graph(
            "
            # comment before header
              .i 1
              .o 1
            # comment before rows
              0 IDLE BUSY 1
              1 BUSY IDLE 0
            ",
        )
        .unwrap();

        assert_eq!(graph.state_name(graph.start()), Some("IDLE"));
        assert_eq!(graph.current(), graph.start());
        assert_eq!(graph.state_name(KissStateId(1)), Some("BUSY"));
    }

    #[test]
    fn resolves_any_reset_to_first_real_state() {
        let graph = read_kiss_graph(
            "
            .i 1
            .o 1
            .r ANY
            0 ANY *
            1 READY DONE 1
            ",
        );

        assert!(matches!(
            graph,
            Err(KissReadError::InvalidTransition { .. })
        ));

        let graph = read_kiss_graph(
            "
            .i 1
            .o 1
            .r ANY
            0 ANY ANY 0
            1 READY DONE 1
            ",
        )
        .unwrap();

        assert_eq!(graph.state_name(graph.start()), Some("READY"));
    }

    #[test]
    fn warns_when_declared_widths_do_not_match_observed_rows() {
        let read = read_kiss(
            "
            .i 4
            .o 3
            01 S0 S1 1
            ",
        )
        .unwrap();

        assert_eq!(read.graph.input_count(), 2);
        assert_eq!(read.graph.output_count(), 1);
        assert_eq!(
            read.warnings,
            vec![
                "Number of inputs given is not correct.",
                ".i line ignored",
                "Number of outputs given is not correct.",
                ".o line ignored"
            ]
        );
    }

    #[test]
    fn rejects_duplicate_count_header() {
        let error = read_kiss(
            "
            .i 1
            .i 2
            .o 1
            0 A B 1
            ",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            KissReadError::DuplicateHeader { option: 'i', .. }
        ));
    }

    #[test]
    fn rejects_missing_required_headers() {
        let error = read_kiss(".i 1\n0 A B 1\n").unwrap_err();

        assert!(matches!(
            error,
            KissReadError::MissingHeader { header: "outputs" }
        ));
    }

    #[test]
    fn rejects_bad_transition_arity() {
        let error = read_kiss(".i 1\n.o 1\n0 A B\n").unwrap_err();

        assert!(matches!(error, KissReadError::InvalidTransition { .. }));
    }

    #[test]
    fn rejects_inconsistent_input_width() {
        let error = read_kiss(
            "
            .i 1
            .o 1
            0 A B 1
            01 B C 0
            ",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            KissReadError::InputWidth {
                expected: 1,
                actual: 2,
                ..
            }
        ));
    }

    #[test]
    fn rejects_inconsistent_output_width() {
        let error = read_kiss(
            "
            .i 1
            .o 1
            0 A B 1
            1 B C 00
            ",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            KissReadError::OutputWidth {
                expected: 1,
                actual: 2,
                ..
            }
        ));
    }

    #[test]
    fn rejects_non_end_dot_line_inside_table() {
        let error = read_kiss(
            "
            .i 1
            .o 1
            0 A B 1
            .bad
            ",
        )
        .unwrap_err();

        assert!(matches!(error, KissReadError::InvalidTerminator { .. }));
    }

    #[test]
    fn rejects_unknown_explicit_reset_state() {
        let error = read_kiss(
            "
            .i 1
            .o 1
            .r MISSING
            0 A B 1
            ",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            KissReadError::InvalidStartState { name } if name == "MISSING"
        ));
    }

    #[test]
    fn keeps_unknown_compact_dot_headers_as_warnings_like_legacy_reader() {
        let read = read_kiss(
            "
            .kiss
            .i 1
            .o 1
            0 A B 1
            ",
        )
        .unwrap();

        assert_eq!(read.warnings, vec!["ignored header line 2: .kiss"]);
    }
}
