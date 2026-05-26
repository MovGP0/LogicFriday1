//! Native Rust port scaffold for `LogicSynthesis/sis/stg/stg_util.c`.
//!
//! The C file has three kinds of behavior:
//! - independent STG name defaults and dump formatting;
//! - copying PI/PO/clock names between `network_t` and `graph_t`;
//! - renaming SIS network nodes to match names stored on an STG.
//!
//! This module ports the independent behavior into a small Rust STG model. The
//! routines that require the native SIS graph, network, node, and clock layers
//! report explicit errors until those prerequisite C files have Rust ports.

use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct StgGraph {
    pub start_state: String,
    pub states: Vec<StgState>,
    pub input_names: Option<Vec<String>>,
    pub output_names: Option<Vec<String>>,
    pub clock: Option<StgClockData>,
}

impl StgGraph {
    pub fn new(start_state: impl Into<String>) -> Self {
        Self {
            start_state: start_state.into(),
            states: Vec::new(),
            input_names: None,
            output_names: None,
            clock: None,
        }
    }

    pub fn with_state(mut self, state: StgState) -> Self {
        self.states.push(state);
        self
    }

    pub fn set_names(&mut self, kind: StgSignalKind, names: Vec<String>) {
        match kind {
            StgSignalKind::Input => self.input_names = Some(names),
            StgSignalKind::Output => self.output_names = Some(names),
        }
    }

    pub fn names(&self, kind: StgSignalKind) -> Option<&[String]> {
        match kind {
            StgSignalKind::Input => self.input_names.as_deref(),
            StgSignalKind::Output => self.output_names.as_deref(),
        }
    }

    pub fn input_count(&self) -> usize {
        self.input_names.as_ref().map_or(0, Vec::len)
    }

    pub fn output_count(&self) -> usize {
        self.output_names.as_ref().map_or(0, Vec::len)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StgState {
    pub name: String,
    pub transitions: Vec<StgTransition>,
}

impl StgState {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transitions: Vec::new(),
        }
    }

    pub fn with_transition(mut self, transition: StgTransition) -> Self {
        self.transitions.push(transition);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StgTransition {
    pub input: String,
    pub output: String,
    pub next_state: String,
}

impl StgTransition {
    pub fn new(
        input: impl Into<String>,
        output: impl Into<String>,
        next_state: impl Into<String>,
    ) -> Self {
        Self {
            input: input.into(),
            output: output.into(),
            next_state: next_state.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StgClockData {
    pub name: String,
    pub cycle_time: f64,
    pub nominal_rise: f64,
    pub nominal_fall: f64,
    pub min_rise: f64,
    pub min_fall: f64,
    pub max_rise: f64,
    pub max_fall: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StgSignalKind {
    Input,
    Output,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkIoSummary {
    pub inputs: Vec<NetworkSignal>,
    pub outputs: Vec<NetworkSignal>,
    pub latch_count: usize,
    pub clock_count: usize,
}

impl NetworkIoSummary {
    pub fn empty() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            latch_count: 0,
            clock_count: 0,
        }
    }

    pub fn real_input_names_without_clocks(&self) -> Vec<String> {
        self.inputs
            .iter()
            .filter(|signal| signal.is_real && !signal.is_clock)
            .map(|signal| signal.name.clone())
            .collect()
    }

    pub fn real_output_names(&self) -> Vec<String> {
        self.outputs
            .iter()
            .filter(|signal| signal.is_real)
            .map(|signal| signal.name.clone())
            .collect()
    }

    pub fn true_input_count(&self) -> usize {
        self.inputs
            .len()
            .saturating_sub(self.latch_count)
            .saturating_sub(self.clock_count)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkSignal {
    pub name: String,
    pub is_real: bool,
    pub is_clock: bool,
    pub is_latch_endpoint: bool,
}

impl NetworkSignal {
    pub fn real(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_real: true,
            is_clock: false,
            is_latch_endpoint: false,
        }
    }

    pub fn clock(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_real: true,
            is_clock: true,
            is_latch_endpoint: false,
        }
    }

    pub fn latch_endpoint(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_real: false,
            is_clock: false,
            is_latch_endpoint: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveNameOptions {
    pub print_input_mismatch: bool,
}

impl Default for SaveNameOptions {
    fn default() -> Self {
        Self {
            print_input_mismatch: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveNameReport {
    pub saved_inputs: bool,
    pub saved_outputs: bool,
    pub diagnostic: Option<SaveNameDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveNameDiagnostic {
    InputCountMismatch {
        network_count: usize,
        stg_count: usize,
        message: Option<&'static str>,
    },
    OutputCountMismatch {
        network_count: usize,
        stg_count: usize,
        message: &'static str,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum StgUtilError {
    MissingStgGraphPort,
    MissingGraphTraversalPort,
    MissingNetworkPort,
    MissingClockPort,
    MissingNodePort,
}

impl fmt::Display for StgUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingStgGraphPort => write!(f, "SIS STG graph APIs are not ported to Rust yet"),
            Self::MissingGraphTraversalPort => {
                write!(f, "SIS graph traversal APIs are not ported to Rust yet")
            }
            Self::MissingNetworkPort => write!(f, "SIS network APIs are not ported to Rust yet"),
            Self::MissingClockPort => write!(f, "SIS clock APIs are not ported to Rust yet"),
            Self::MissingNodePort => write!(f, "SIS node APIs are not ported to Rust yet"),
        }
    }
}

impl Error for StgUtilError {}

pub fn assign_names(kind: StgSignalKind, count: usize) -> Option<Vec<String>> {
    (count > 0).then(|| {
        let prefix = match kind {
            StgSignalKind::Input => "IN",
            StgSignalKind::Output => "OUT",
        };

        (0..count)
            .map(|index| format!("{prefix}_{index}"))
            .collect()
    })
}

pub fn assign_input_names(count: usize) -> Option<Vec<String>> {
    assign_names(StgSignalKind::Input, count)
}

pub fn assign_output_names(count: usize) -> Option<Vec<String>> {
    assign_names(StgSignalKind::Output, count)
}

pub fn save_names_from_summary(
    network: &NetworkIoSummary,
    stg: &mut StgGraph,
    stg_input_count: usize,
    stg_output_count: usize,
    options: SaveNameOptions,
) -> SaveNameReport {
    let mut saved_outputs = false;
    let mut saved_inputs = false;

    if stg.output_names.is_none() {
        let network_outputs = network.real_output_names();
        if network_outputs.is_empty() {
            stg.output_names = assign_output_names(stg_output_count);
            saved_outputs = stg.output_names.is_some();
        } else if network_outputs.len() != stg_output_count {
            return SaveNameReport {
                saved_inputs,
                saved_outputs,
                diagnostic: Some(SaveNameDiagnostic::OutputCountMismatch {
                    network_count: network_outputs.len(),
                    stg_count: stg_output_count,
                    message: "Number of outputs in the STG and the BLIF file do not match. Output names from the BLIF file are ignored.",
                }),
            };
        } else {
            stg.output_names = Some(network_outputs);
            saved_outputs = true;
        }
    }

    if stg.input_names.is_none() {
        let network_input_count = network.true_input_count();
        if network_input_count == 0 {
            stg.input_names = assign_input_names(stg_input_count);
            saved_inputs = stg.input_names.is_some();
        } else if network_input_count != stg_input_count {
            return SaveNameReport {
                saved_inputs,
                saved_outputs,
                diagnostic: Some(SaveNameDiagnostic::InputCountMismatch {
                    network_count: network_input_count,
                    stg_count: stg_input_count,
                    message: options.print_input_mismatch.then_some(
                        "Number of inputs in the STG and the BLIF file do not match. Input names from the BLIF file are ignored.",
                    ),
                }),
            };
        } else {
            stg.input_names = Some(network.real_input_names_without_clocks());
            saved_inputs = true;
        }
    }

    SaveNameReport {
        saved_inputs,
        saved_outputs,
        diagnostic: None,
    }
}

pub fn dump_graph(graph: Option<&StgGraph>, network: Option<&NetworkIoSummary>) -> String {
    let Some(graph) = graph else {
        return "NIL stg\n".to_owned();
    };

    let mut text = String::new();
    text.push_str(&format!("\nInitial state {}\n", graph.start_state));

    if let Some(network) = network {
        text.push_str("PI list: ");
        for input in network
            .inputs
            .iter()
            .filter(|signal| !signal.is_latch_endpoint)
        {
            text.push_str(&input.name);
            text.push_str("  ");
        }
        text.push_str("\nPO list: ");
        for output in network
            .outputs
            .iter()
            .filter(|signal| !signal.is_latch_endpoint)
        {
            text.push_str(&output.name);
            text.push_str("  ");
        }
        text.push('\n');
    }

    text.push_str("PresentState input  output NextState\n");
    for state in &graph.states {
        for transition in &state.transitions {
            text.push_str(&format!(
                "{} {} {} {}\n",
                state.name, transition.input, transition.output, transition.next_state
            ));
        }
    }
    text.push('\n');
    text
}

pub fn save_names_from_sis_network() -> Result<(), StgUtilError> {
    Err(StgUtilError::MissingNetworkPort)
}

pub fn copy_clock_data_from_sis_network() -> Result<StgClockData, StgUtilError> {
    Err(StgUtilError::MissingClockPort)
}

pub fn set_sis_network_pipo_names() -> Result<(), StgUtilError> {
    Err(StgUtilError::MissingNetworkPort)
}

pub fn change_sis_dc_node_name() -> Result<(), StgUtilError> {
    Err(StgUtilError::MissingNodePort)
}

pub fn traverse_sis_stg_graph() -> Result<(), StgUtilError> {
    Err(StgUtilError::MissingGraphTraversalPort)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_c_style_default_names() {
        assert_eq!(assign_input_names(0), None);
        assert_eq!(
            assign_input_names(3),
            Some(vec![
                "IN_0".to_owned(),
                "IN_1".to_owned(),
                "IN_2".to_owned()
            ])
        );
        assert_eq!(
            assign_output_names(2),
            Some(vec!["OUT_0".to_owned(), "OUT_1".to_owned()])
        );
    }

    #[test]
    fn saves_network_names_when_counts_match() {
        let network = NetworkIoSummary {
            inputs: vec![
                NetworkSignal::real("a"),
                NetworkSignal::clock("clk"),
                NetworkSignal::latch_endpoint("state"),
                NetworkSignal::real("b"),
            ],
            outputs: vec![NetworkSignal::real("z")],
            latch_count: 1,
            clock_count: 1,
        };
        let mut stg = StgGraph::new("s0");

        let report = save_names_from_summary(&network, &mut stg, 2, 1, SaveNameOptions::default());

        assert_eq!(
            report,
            SaveNameReport {
                saved_inputs: true,
                saved_outputs: true,
                diagnostic: None,
            }
        );
        assert_eq!(stg.names(StgSignalKind::Input).unwrap(), ["a", "b"]);
        assert_eq!(stg.names(StgSignalKind::Output).unwrap(), ["z"]);
    }

    #[test]
    fn falls_back_to_generated_names_without_network_pios() {
        let network = NetworkIoSummary::empty();
        let mut stg = StgGraph::new("reset");

        let report = save_names_from_summary(
            &network,
            &mut stg,
            2,
            1,
            SaveNameOptions {
                print_input_mismatch: false,
            },
        );

        assert_eq!(report.diagnostic, None);
        assert_eq!(stg.names(StgSignalKind::Input).unwrap(), ["IN_0", "IN_1"]);
        assert_eq!(stg.names(StgSignalKind::Output).unwrap(), ["OUT_0"]);
    }

    #[test]
    fn reports_mismatches_and_honors_input_print_option() {
        let network = NetworkIoSummary {
            inputs: vec![NetworkSignal::real("a"), NetworkSignal::real("b")],
            outputs: Vec::new(),
            latch_count: 0,
            clock_count: 0,
        };
        let mut stg = StgGraph::new("s0");

        let report = save_names_from_summary(
            &network,
            &mut stg,
            1,
            0,
            SaveNameOptions {
                print_input_mismatch: false,
            },
        );

        assert_eq!(
            report.diagnostic,
            Some(SaveNameDiagnostic::InputCountMismatch {
                network_count: 2,
                stg_count: 1,
                message: None,
            })
        );
    }

    #[test]
    fn formats_dump_graph_like_c_output() {
        let graph = StgGraph::new("s0")
            .with_state(StgState::new("s0").with_transition(StgTransition::new("1-", "0", "s1")))
            .with_state(StgState::new("s1").with_transition(StgTransition::new("0-", "1", "s0")));
        let network = NetworkIoSummary {
            inputs: vec![
                NetworkSignal::real("a"),
                NetworkSignal::latch_endpoint("latch_out"),
            ],
            outputs: vec![
                NetworkSignal::real("z"),
                NetworkSignal::latch_endpoint("latch_in"),
            ],
            latch_count: 1,
            clock_count: 0,
        };

        assert_eq!(
            dump_graph(Some(&graph), Some(&network)),
            "\nInitial state s0\nPI list: a  \nPO list: z  \nPresentState input  output NextState\ns0 1- 0 s1\ns1 0- 1 s0\n\n"
        );
        assert_eq!(dump_graph(None, None), "NIL stg\n");
    }

    #[test]
    fn blocked_sis_entry_points_report_missing_ports() {
        assert_eq!(
            save_names_from_sis_network(),
            Err(StgUtilError::MissingNetworkPort)
        );
        assert_eq!(
            copy_clock_data_from_sis_network(),
            Err(StgUtilError::MissingClockPort)
        );
        assert_eq!(
            set_sis_network_pipo_names(),
            Err(StgUtilError::MissingNetworkPort)
        );
        assert_eq!(
            change_sis_dc_node_name(),
            Err(StgUtilError::MissingNodePort)
        );
        assert_eq!(
            traverse_sis_stg_graph(),
            Err(StgUtilError::MissingGraphTraversalPort)
        );
    }
}
