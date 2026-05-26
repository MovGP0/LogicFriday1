//! Native Rust writer for SIS KISS state-transition graphs.
//!
//! The original `sis/io/write_kiss.c` prints the dimensions, reset state, and
//! then each outgoing transition grouped by source-state iteration order. This
//! port keeps that behavior on an owned Rust model so callers can adapt either
//! the native STG port or command-layer structures without going through a C
//! ABI shim.

use std::error::Error;
use std::fmt;
use std::io;
use std::io::Write;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct KissStateId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KissState {
    pub name: String,
}

impl KissState {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KissTransition {
    pub from: KissStateId,
    pub to: KissStateId,
    pub input: String,
    pub output: String,
}

impl KissTransition {
    pub fn new(
        from: KissStateId,
        to: KissStateId,
        input: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            from,
            to,
            input: input.into(),
            output: output.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KissStateGraph {
    input_count: usize,
    output_count: usize,
    states: Vec<KissState>,
    transitions: Vec<KissTransition>,
    start: Option<KissStateId>,
}

impl KissStateGraph {
    pub fn new(input_count: usize, output_count: usize) -> Self {
        Self {
            input_count,
            output_count,
            states: Vec::new(),
            transitions: Vec::new(),
            start: None,
        }
    }

    pub fn input_count(&self) -> usize {
        self.input_count
    }

    pub fn output_count(&self) -> usize {
        self.output_count
    }

    pub fn states(&self) -> &[KissState] {
        &self.states
    }

    pub fn transitions(&self) -> &[KissTransition] {
        &self.transitions
    }

    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    pub fn product_count(&self) -> usize {
        self.transitions.len()
    }

    pub fn start(&self) -> Option<KissStateId> {
        self.start
    }

    pub fn add_state(&mut self, name: impl Into<String>) -> KissStateId {
        let id = KissStateId(self.states.len());
        self.states.push(KissState::new(name));
        id
    }

    pub fn set_start(&mut self, state: KissStateId) -> Result<(), WriteKissError> {
        self.require_state(state)?;
        self.start = Some(state);
        Ok(())
    }

    pub fn add_transition(
        &mut self,
        from: KissStateId,
        to: KissStateId,
        input: impl Into<String>,
        output: impl Into<String>,
    ) -> Result<(), WriteKissError> {
        self.require_state(from)?;
        self.require_state(to)?;

        let input = input.into();
        let output = output.into();
        if input.len() != self.input_count {
            return Err(WriteKissError::InputWidth {
                expected: self.input_count,
                actual: input.len(),
            });
        }
        if output.len() != self.output_count {
            return Err(WriteKissError::OutputWidth {
                expected: self.output_count,
                actual: output.len(),
            });
        }

        self.transitions
            .push(KissTransition::new(from, to, input, output));
        Ok(())
    }

    pub fn state_name(&self, state: KissStateId) -> Result<&str, WriteKissError> {
        self.states
            .get(state.0)
            .map(|state| state.name.as_str())
            .ok_or(WriteKissError::UnknownState(state))
    }

    fn require_state(&self, state: KissStateId) -> Result<(), WriteKissError> {
        if state.0 < self.states.len() {
            Ok(())
        } else {
            Err(WriteKissError::UnknownState(state))
        }
    }
}

#[derive(Debug)]
pub enum WriteKissError {
    MissingGraph,
    MissingStartState,
    UnknownState(KissStateId),
    InputWidth { expected: usize, actual: usize },
    OutputWidth { expected: usize, actual: usize },
    Io(io::Error),
}

impl fmt::Display for WriteKissError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingGraph => write!(formatter, "write_kiss: no STG specified"),
            Self::MissingStartState => write!(formatter, "write_kiss: no reset state specified"),
            Self::UnknownState(state) => write!(formatter, "write_kiss: unknown state {:?}", state),
            Self::InputWidth { expected, actual } => write!(
                formatter,
                "write_kiss: transition input width {actual} does not match {expected}"
            ),
            Self::OutputWidth { expected, actual } => write!(
                formatter,
                "write_kiss: transition output width {actual} does not match {expected}"
            ),
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl Error for WriteKissError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for WriteKissError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn write_optional_kiss<W>(
    writer: &mut W,
    graph: Option<&KissStateGraph>,
) -> Result<(), WriteKissError>
where
    W: Write,
{
    let graph = graph.ok_or(WriteKissError::MissingGraph)?;
    write_kiss(writer, graph)
}

pub fn write_kiss<W>(writer: &mut W, graph: &KissStateGraph) -> Result<(), WriteKissError>
where
    W: Write,
{
    let start = graph.start().ok_or(WriteKissError::MissingStartState)?;
    let start_name = graph.state_name(start)?;

    writeln!(writer, ".i {}", graph.input_count())?;
    writeln!(writer, ".o {}", graph.output_count())?;
    writeln!(writer, ".p {}", graph.product_count())?;
    writeln!(writer, ".s {}", graph.state_count())?;
    writeln!(writer, ".r {start_name}")?;

    for (state_index, state) in graph.states().iter().enumerate() {
        let from = KissStateId(state_index);
        for transition in graph
            .transitions()
            .iter()
            .filter(|transition| transition.from == from)
        {
            let to_name = graph.state_name(transition.to)?;
            writeln!(
                writer,
                "{} {} {} {}",
                transition.input, state.name, to_name, transition.output
            )?;
        }
    }

    Ok(())
}

pub fn write_kiss_to_string(graph: &KissStateGraph) -> Result<String, WriteKissError> {
    let mut bytes = Vec::new();
    write_kiss(&mut bytes, graph)?;
    String::from_utf8(bytes)
        .map_err(|error| WriteKissError::Io(io::Error::new(io::ErrorKind::InvalidData, error)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_header_reset_and_transitions() {
        let mut graph = KissStateGraph::new(2, 1);
        let s0 = graph.add_state("S0");
        let s1 = graph.add_state("S1");
        graph.set_start(s0).unwrap();
        graph.add_transition(s0, s1, "00", "0").unwrap();
        graph.add_transition(s1, s0, "1-", "1").unwrap();

        let output = write_kiss_to_string(&graph).unwrap();

        assert_eq!(
            output,
            ".i 2\n.o 1\n.p 2\n.s 2\n.r S0\n00 S0 S1 0\n1- S1 S0 1\n"
        );
    }

    #[test]
    fn groups_edges_by_state_iteration_order() {
        let mut graph = KissStateGraph::new(1, 1);
        let s0 = graph.add_state("A");
        let s1 = graph.add_state("B");
        graph.set_start(s0).unwrap();
        graph.add_transition(s1, s0, "1", "1").unwrap();
        graph.add_transition(s0, s1, "0", "0").unwrap();

        let output = write_kiss_to_string(&graph).unwrap();

        assert!(output.find("0 A B 0").unwrap() < output.find("1 B A 1").unwrap());
    }

    #[test]
    fn preserves_multiple_outgoing_edge_order_for_one_state() {
        let mut graph = KissStateGraph::new(2, 2);
        let s0 = graph.add_state("idle");
        let s1 = graph.add_state("busy");
        graph.set_start(s0).unwrap();
        graph.add_transition(s0, s1, "00", "01").unwrap();
        graph.add_transition(s0, s0, "11", "10").unwrap();

        let output = write_kiss_to_string(&graph).unwrap();

        assert!(output.find("00 idle busy 01").unwrap() < output.find("11 idle idle 10").unwrap());
    }

    #[test]
    fn reports_missing_graph_like_c_null_stg_guard() {
        let mut output = Vec::new();

        let error = write_optional_kiss(&mut output, None).unwrap_err();

        assert!(matches!(error, WriteKissError::MissingGraph));
        assert!(output.is_empty());
    }

    #[test]
    fn reports_missing_start_state() {
        let mut graph = KissStateGraph::new(1, 1);
        graph.add_state("S0");

        let error = write_kiss_to_string(&graph).unwrap_err();

        assert!(matches!(error, WriteKissError::MissingStartState));
    }

    #[test]
    fn validates_transition_input_width() {
        let mut graph = KissStateGraph::new(2, 1);
        let s0 = graph.add_state("S0");
        graph.set_start(s0).unwrap();

        let error = graph.add_transition(s0, s0, "0", "1").unwrap_err();

        assert!(matches!(
            error,
            WriteKissError::InputWidth {
                expected: 2,
                actual: 1
            }
        ));
    }

    #[test]
    fn validates_transition_output_width() {
        let mut graph = KissStateGraph::new(1, 2);
        let s0 = graph.add_state("S0");
        graph.set_start(s0).unwrap();

        let error = graph.add_transition(s0, s0, "0", "1").unwrap_err();

        assert!(matches!(
            error,
            WriteKissError::OutputWidth {
                expected: 2,
                actual: 1
            }
        ));
    }

    #[test]
    fn validates_state_references() {
        let mut graph = KissStateGraph::new(1, 1);

        let error = graph.set_start(KissStateId(7)).unwrap_err();

        assert!(matches!(
            error,
            WriteKissError::UnknownState(KissStateId(7))
        ));
    }
}
