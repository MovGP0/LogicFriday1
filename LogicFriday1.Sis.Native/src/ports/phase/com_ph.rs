//! Native Rust command model for SIS phase commands.
//!
//! This module keeps the command parsing and command intent for phase
//! assignment and inverter insertion in Rust. Integration with the command
//! interpreter and concrete network mutation can be supplied by higher-level
//! native backends.

use std::error::Error;
use std::fmt;

pub const PHASE_USAGE: &str = concat!(
    "usage: phase [-gqst] [-r n]\n",
    "       -g Good phase\n",
    "       -q Quick phase\n",
    "       -s Simulated annealing\n",
    "       -t Trace\n",
    "       -r n Random greedy (n > 0)\n",
);

pub const SIMULATED_ANNEALING_NOT_IMPLEMENTED: &str =
    "simulated annealing method has not been implemented\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseCommandKind {
    Phase,
    AddInverter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: PhaseCommandKind,
    pub changes_network: bool,
}

pub const PHASE_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "phase",
        kind: PhaseCommandKind::Phase,
        changes_network: true,
    },
    CommandRegistration {
        name: "add_inverter",
        kind: PhaseCommandKind::AddInverter,
        changes_network: true,
    },
];

pub fn phase_command_registrations() -> &'static [CommandRegistration] {
    PHASE_COMMANDS
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseMethod {
    Quick,
    Good,
    SimulatedAnnealing,
    RandomGreedy { iterations: usize },
}

impl Default for PhaseMethod {
    fn default() -> Self {
        Self::Quick
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PhaseAssignment {
    pub method: PhaseMethod,
    pub trace: bool,
    pub check: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AddInverterPlan {
    Network,
    Nodes(Vec<String>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryOutput,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeSelection {
    pub name: String,
    pub kind: NodeKind,
}

impl NodeSelection {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhaseCommand {
    Phase(PhaseAssignment),
    AddInverter(AddInverterPlan),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhaseCommandError {
    UnsupportedOption(String),
    MissingOptionValue(char),
    InvalidInteger { option: char, value: String },
    NonPositiveRandomGreedyCount { value: String },
    ExtraArguments(Vec<String>),
    UnknownCommand(String),
    MissingNativeBackend { command: PhaseCommandKind },
}

impl fmt::Display for PhaseCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::InvalidInteger { option, value } => {
                write!(f, "invalid integer for -{option}: {value}")
            }
            Self::NonPositiveRandomGreedyCount { value } => {
                write!(f, "random greedy count must be greater than zero: {value}")
            }
            Self::ExtraArguments(args) => {
                write!(f, "unexpected phase command arguments: {}", args.join(" "))
            }
            Self::UnknownCommand(command) => write!(f, "unknown phase command {command}"),
            Self::MissingNativeBackend { command } => {
                write!(f, "{command:?} requires a native phase backend")
            }
        }
    }
}

impl Error for PhaseCommandError {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhaseCommandOutput {
    pub messages: Vec<String>,
}

impl PhaseCommandOutput {
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            messages: vec![message.into()],
        }
    }
}

pub trait PhaseBackend {
    type Error;

    fn quick(&mut self, assignment: PhaseAssignment) -> Result<(), Self::Error>;

    fn good(&mut self, assignment: PhaseAssignment) -> Result<(), Self::Error>;

    fn random_greedy(
        &mut self,
        assignment: PhaseAssignment,
        iterations: usize,
    ) -> Result<(), Self::Error>;

    fn add_inverter_to_network(&mut self) -> Result<(), Self::Error>;

    fn add_inverter_to_node(&mut self, node: &str) -> Result<(), Self::Error>;
}

pub fn parse_phase_args<I, S>(args: I) -> Result<PhaseAssignment, PhaseCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut parser = ShortOptionParser::new(args);
    let mut assignment = PhaseAssignment::default();
    let mut operands = Vec::new();

    while let Some(token) = parser.next_token() {
        if token == "--" {
            operands.extend(parser.remaining_args());
            break;
        }

        if !token.starts_with('-') || token == "-" {
            operands.push(token);
            operands.extend(parser.remaining_args());
            break;
        }

        let mut chars = token[1..].char_indices().peekable();
        while let Some((index, option)) = chars.next() {
            match option {
                'c' => assignment.check = true,
                'q' => assignment.method = PhaseMethod::Quick,
                'g' => assignment.method = PhaseMethod::Good,
                's' => assignment.method = PhaseMethod::SimulatedAnnealing,
                't' => assignment.trace = true,
                'r' => {
                    let value_start = index + option.len_utf8();
                    let value = if value_start < token.len() - 1 {
                        token[(value_start + 1)..].to_owned()
                    } else {
                        parser
                            .next_token()
                            .ok_or(PhaseCommandError::MissingOptionValue('r'))?
                    };
                    let iterations = parse_positive_c_integer('r', &value)?;
                    assignment.method = PhaseMethod::RandomGreedy { iterations };
                    break;
                }
                _ => return Err(PhaseCommandError::UnsupportedOption(format!("-{option}"))),
            }
        }
    }

    if operands.is_empty() {
        Ok(assignment)
    } else {
        Err(PhaseCommandError::ExtraArguments(operands))
    }
}

pub fn parse_add_inverter_args<I, S>(args: I) -> AddInverterPlan
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let nodes: Vec<_> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();

    if nodes.is_empty() {
        AddInverterPlan::Network
    } else {
        AddInverterPlan::Nodes(nodes)
    }
}

pub fn select_add_inverter_targets(nodes: &[NodeSelection]) -> Vec<String> {
    nodes
        .iter()
        .filter(|node| node.kind != NodeKind::PrimaryOutput)
        .map(|node| node.name.clone())
        .collect()
}

pub fn parse_command<I, S>(command_name: &str, args: I) -> Result<PhaseCommand, PhaseCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name {
        "phase" => parse_phase_args(args).map(PhaseCommand::Phase),
        "add_inverter" => Ok(PhaseCommand::AddInverter(parse_add_inverter_args(args))),
        _ => Err(PhaseCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn execute_phase_assignment<B>(
    backend: &mut B,
    assignment: PhaseAssignment,
) -> Result<PhaseCommandOutput, B::Error>
where
    B: PhaseBackend,
{
    match assignment.method {
        PhaseMethod::Quick => backend.quick(assignment)?,
        PhaseMethod::Good => backend.good(assignment)?,
        PhaseMethod::SimulatedAnnealing => {
            return Ok(PhaseCommandOutput::message(
                SIMULATED_ANNEALING_NOT_IMPLEMENTED,
            ));
        }
        PhaseMethod::RandomGreedy { iterations } => {
            backend.random_greedy(assignment, iterations)?;
        }
    }

    Ok(PhaseCommandOutput::default())
}

pub fn execute_add_inverter<B>(
    backend: &mut B,
    plan: &AddInverterPlan,
) -> Result<PhaseCommandOutput, B::Error>
where
    B: PhaseBackend,
{
    match plan {
        AddInverterPlan::Network => backend.add_inverter_to_network()?,
        AddInverterPlan::Nodes(nodes) => {
            for node in nodes {
                backend.add_inverter_to_node(node)?;
            }
        }
    }

    Ok(PhaseCommandOutput::default())
}

pub fn execute_command_without_backend(
    command: &PhaseCommand,
) -> Result<PhaseCommandOutput, PhaseCommandError> {
    let command = match command {
        PhaseCommand::Phase(_) => PhaseCommandKind::Phase,
        PhaseCommand::AddInverter(_) => PhaseCommandKind::AddInverter,
    };

    Err(PhaseCommandError::MissingNativeBackend { command })
}

fn parse_positive_c_integer(option: char, value: &str) -> Result<usize, PhaseCommandError> {
    let parsed = c_atoi(value).ok_or_else(|| PhaseCommandError::InvalidInteger {
        option,
        value: value.to_owned(),
    })?;

    if parsed <= 0 {
        return Err(PhaseCommandError::NonPositiveRandomGreedyCount {
            value: value.to_owned(),
        });
    }

    Ok(parsed as usize)
}

fn c_atoi(value: &str) -> Option<i32> {
    let value = value.trim_start();
    let mut chars = value.chars().peekable();
    let sign = match chars.peek() {
        Some('-') => {
            chars.next();
            -1_i64
        }
        Some('+') => {
            chars.next();
            1_i64
        }
        _ => 1_i64,
    };

    let mut found_digit = false;
    let mut number = 0_i64;
    for character in chars {
        let Some(digit) = character.to_digit(10) else {
            break;
        };
        found_digit = true;
        number = number.saturating_mul(10).saturating_add(i64::from(digit));
    }

    if found_digit {
        Some((number * sign).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
    } else {
        None
    }
}

struct ShortOptionParser {
    args: std::vec::IntoIter<String>,
}

impl ShortOptionParser {
    fn new<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            args: args
                .into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }

    fn next_token(&mut self) -> Option<String> {
        self.args.next()
    }

    fn remaining_args(&mut self) -> Vec<String> {
        self.args.by_ref().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        calls: Vec<String>,
    }

    impl PhaseBackend for RecordingBackend {
        type Error = String;

        fn quick(&mut self, assignment: PhaseAssignment) -> Result<(), Self::Error> {
            self.calls
                .push(format!("quick:{}:{}", assignment.trace, assignment.check));
            Ok(())
        }

        fn good(&mut self, assignment: PhaseAssignment) -> Result<(), Self::Error> {
            self.calls
                .push(format!("good:{}:{}", assignment.trace, assignment.check));
            Ok(())
        }

        fn random_greedy(
            &mut self,
            assignment: PhaseAssignment,
            iterations: usize,
        ) -> Result<(), Self::Error> {
            self.calls.push(format!(
                "random:{iterations}:{}:{}",
                assignment.trace, assignment.check
            ));
            Ok(())
        }

        fn add_inverter_to_network(&mut self) -> Result<(), Self::Error> {
            self.calls.push("network".to_owned());
            Ok(())
        }

        fn add_inverter_to_node(&mut self, node: &str) -> Result<(), Self::Error> {
            self.calls.push(format!("node:{node}"));
            Ok(())
        }
    }

    #[test]
    fn command_registration_matches_phase_init() {
        assert_eq!(
            phase_command_registrations(),
            &[
                CommandRegistration {
                    name: "phase",
                    kind: PhaseCommandKind::Phase,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "add_inverter",
                    kind: PhaseCommandKind::AddInverter,
                    changes_network: true,
                },
            ]
        );
    }

    #[test]
    fn parses_phase_default_and_flags() {
        assert_eq!(
            parse_phase_args(std::iter::empty::<&str>()).unwrap(),
            PhaseAssignment {
                method: PhaseMethod::Quick,
                trace: false,
                check: false,
            }
        );

        assert_eq!(
            parse_phase_args(["-ctg"]).unwrap(),
            PhaseAssignment {
                method: PhaseMethod::Good,
                trace: true,
                check: true,
            }
        );
    }

    #[test]
    fn later_phase_method_option_wins() {
        assert_eq!(
            parse_phase_args(["-g", "-q"]).unwrap().method,
            PhaseMethod::Quick
        );
        assert_eq!(parse_phase_args(["-qg"]).unwrap().method, PhaseMethod::Good);
        assert_eq!(
            parse_phase_args(["-s"]).unwrap().method,
            PhaseMethod::SimulatedAnnealing
        );
    }

    #[test]
    fn parses_random_greedy_count_like_c_option_parsing() {
        assert_eq!(
            parse_phase_args(["-r", "12"]).unwrap().method,
            PhaseMethod::RandomGreedy { iterations: 12 }
        );
        assert_eq!(
            parse_phase_args(["-r12"]).unwrap().method,
            PhaseMethod::RandomGreedy { iterations: 12 }
        );
        assert_eq!(
            parse_phase_args(["-r", "12xyz"]).unwrap().method,
            PhaseMethod::RandomGreedy { iterations: 12 }
        );
        assert_eq!(
            parse_phase_args(["-r", "0"]),
            Err(PhaseCommandError::NonPositiveRandomGreedyCount {
                value: "0".to_owned()
            })
        );
        assert_eq!(
            parse_phase_args(["-r"]),
            Err(PhaseCommandError::MissingOptionValue('r'))
        );
    }

    #[test]
    fn rejects_phase_operands_and_unknown_options() {
        assert_eq!(
            parse_phase_args(["-x"]),
            Err(PhaseCommandError::UnsupportedOption("-x".to_owned()))
        );
        assert_eq!(
            parse_phase_args(["node"]),
            Err(PhaseCommandError::ExtraArguments(vec!["node".to_owned()]))
        );
        assert_eq!(
            parse_phase_args(["--", "node"]),
            Err(PhaseCommandError::ExtraArguments(vec!["node".to_owned()]))
        );
    }

    #[test]
    fn parses_add_inverter_plan() {
        assert_eq!(
            parse_add_inverter_args(std::iter::empty::<&str>()),
            AddInverterPlan::Network
        );
        assert_eq!(
            parse_add_inverter_args(["a", "b"]),
            AddInverterPlan::Nodes(vec!["a".to_owned(), "b".to_owned()])
        );
    }

    #[test]
    fn skips_primary_outputs_when_selecting_add_inverter_targets() {
        let nodes = [
            NodeSelection::new("a", NodeKind::Other),
            NodeSelection::new("out", NodeKind::PrimaryOutput),
            NodeSelection::new("b", NodeKind::Other),
        ];

        assert_eq!(
            select_add_inverter_targets(&nodes),
            vec!["a".to_owned(), "b".to_owned()]
        );
    }

    #[test]
    fn dispatches_phase_assignment_to_backend() {
        let mut backend = RecordingBackend::default();
        let assignment = PhaseAssignment {
            method: PhaseMethod::RandomGreedy { iterations: 4 },
            trace: true,
            check: false,
        };

        let output = execute_phase_assignment(&mut backend, assignment).unwrap();

        assert_eq!(output, PhaseCommandOutput::default());
        assert_eq!(backend.calls, vec!["random:4:true:false"]);
    }

    #[test]
    fn simulated_annealing_reports_legacy_message_without_backend_call() {
        let mut backend = RecordingBackend::default();
        let output = execute_phase_assignment(
            &mut backend,
            PhaseAssignment {
                method: PhaseMethod::SimulatedAnnealing,
                trace: false,
                check: false,
            },
        )
        .unwrap();

        assert_eq!(
            output,
            PhaseCommandOutput::message(SIMULATED_ANNEALING_NOT_IMPLEMENTED)
        );
        assert!(backend.calls.is_empty());
    }

    #[test]
    fn dispatches_add_inverter_plan_to_backend() {
        let mut backend = RecordingBackend::default();
        let plan = AddInverterPlan::Nodes(vec!["a".to_owned(), "b".to_owned()]);

        execute_add_inverter(&mut backend, &plan).unwrap();

        assert_eq!(backend.calls, vec!["node:a", "node:b"]);
    }

    #[test]
    fn reports_missing_backend_with_generic_diagnostic() {
        let command = PhaseCommand::Phase(PhaseAssignment::default());
        let error = execute_command_without_backend(&command).unwrap_err();

        assert_eq!(
            error,
            PhaseCommandError::MissingNativeBackend {
                command: PhaseCommandKind::Phase,
            }
        );
        assert!(error.to_string().contains("native phase backend"));
    }
}
