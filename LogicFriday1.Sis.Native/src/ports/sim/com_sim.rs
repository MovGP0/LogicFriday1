//! Native Rust command model for `LogicSynthesis/sis/sim/com_sim.c`.
//!
//! The C file registers `simulate`, `set_state`, `print_state`, and
//! `sim_verify`, parses their command-line options, formats simulation state,
//! and dispatches into SIS network, latch, STG, BLIF reader, and simulation
//! internals. This port keeps the deterministic command/parser/formatting
//! behavior native. Entry points that still require unported SIS data
//! structures report explicit dependency blockers instead of exposing legacy C
//! ABI shims.

use std::error::Error;
use std::fmt;

pub const SIMULATE_USAGE: &str = "usage: simulate [-s] [-i] in1 in2 in3 ...\n";
pub const SET_STATE_USAGE: &str = "usage: set_state [-s] [-i] [state_name]\n";
pub const PRINT_STATE_USAGE: &str = "usage:  print_state\n";
pub const SIM_VERIFY_USAGE: &str = "usage: sim_verify [-n n_patterns] network2.blif\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimCommandKind {
    Simulate,
    SetState,
    PrintState,
    SimVerify,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: SimCommandKind,
    pub changes_network: bool,
}

pub const SIS_SIM_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "simulate",
        kind: SimCommandKind::Simulate,
        changes_network: false,
    },
    CommandRegistration {
        name: "set_state",
        kind: SimCommandKind::SetState,
        changes_network: true,
    },
    CommandRegistration {
        name: "print_state",
        kind: SimCommandKind::PrintState,
        changes_network: false,
    },
    CommandRegistration {
        name: "sim_verify",
        kind: SimCommandKind::SimVerify,
        changes_network: false,
    },
];

pub fn sim_command_registrations() -> &'static [CommandRegistration] {
    SIS_SIM_COMMANDS
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimTargets {
    pub logic: bool,
    pub stg: bool,
}

impl Default for SimTargets {
    fn default() -> Self {
        Self {
            logic: true,
            stg: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkSimulationShape {
    pub primary_inputs: usize,
    pub latches: usize,
    pub has_stg: bool,
    pub stg_inputs: usize,
}

impl NetworkSimulationShape {
    pub fn external_logic_inputs(&self) -> usize {
        self.primary_inputs.saturating_sub(self.latches)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulatePlan {
    pub targets: SimTargets,
    pub input_values: Vec<Bit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetStatePlan {
    pub targets: SimTargets,
    pub state_name: Option<String>,
    pub stg_lookup: StgLookupMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StgLookupMode {
    SymbolicName,
    Encoding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimVerifyPlan {
    pub patterns: usize,
    pub filename: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimCommand {
    Simulate(SimulatePlan),
    SetState(SetStatePlan),
    PrintState,
    SimVerify(SimVerifyPlan),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Bit {
    Zero,
    One,
}

impl Bit {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "0" => Some(Self::Zero),
            "1" => Some(Self::One),
            _ => None,
        }
    }

    pub fn as_i32(self) -> i32 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
        }
    }

    pub fn as_char(self) -> char {
        match self {
            Self::Zero => '0',
            Self::One => '1',
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComSimError {
    UnsupportedOption(String),
    MissingOptionValue(char),
    InvalidInteger {
        option: char,
        value: String,
    },
    BadBit {
        command: SimCommandKind,
        value: String,
    },
    WrongLogicInputCount {
        expected: usize,
        supplied: usize,
    },
    WrongStgInputCount {
        expected: usize,
        supplied: usize,
    },
    SetStateNeedsOneState,
    WrongLatchStateLength {
        expected: usize,
        supplied: usize,
    },
    BadStateBit {
        value: char,
    },
    InvalidPatternCount(usize),
    SimVerifyNeedsNetwork,
    PrintStateTakesNoArguments,
    Blocked {
        command: SimCommandKind,
    },
}

impl fmt::Display for ComSimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::InvalidInteger { option, value } => {
                write!(f, "invalid integer for -{option}: {value}")
            }
            Self::BadBit { command, value } => {
                let prefix = match command {
                    SimCommandKind::Simulate => "simulate",
                    SimCommandKind::SetState => "set_state",
                    SimCommandKind::PrintState => "print_state",
                    SimCommandKind::SimVerify => "sim_verify",
                };
                write!(f, "{prefix}: bad value '{value}' -- should be 0 or 1")
            }
            Self::WrongLogicInputCount { expected, supplied } => write!(
                f,
                "simulate network: network has {expected} inputs; {supplied} values were supplied."
            ),
            Self::WrongStgInputCount { expected, supplied } => write!(
                f,
                "simulate stg: stg has {expected} inputs; {supplied} values were supplied."
            ),
            Self::SetStateNeedsOneState => write!(f, "set_state: one state must be given"),
            Self::WrongLatchStateLength { expected, supplied } => write!(
                f,
                "set_state: network had {expected} latches; {supplied} values were supplied."
            ),
            Self::BadStateBit { value } => {
                write!(f, "set_state: bad value '{value}' -- should be 0 or 1")
            }
            Self::InvalidPatternCount(patterns) => {
                write!(
                    f,
                    "sim_verify pattern count must be at least 1, got {patterns}"
                )
            }
            Self::SimVerifyNeedsNetwork => write!(f, "sim_verify requires one network filename"),
            Self::PrintStateTakesNoArguments => write!(f, "print_state takes no arguments"),
            Self::Blocked { command } => write!(
                f,
                "{command:?} requires native Rust ports for SIS dependencies"
            ),
        }
    }
}

impl Error for ComSimError {}

pub fn parse_simulate_args<I, S>(
    args: I,
    shape: &NetworkSimulationShape,
) -> Result<SimulatePlan, ComSimError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let (targets, operands) = parse_target_options(args, SimCommandKind::Simulate)?;

    if targets.logic && shape.primary_inputs != 0 {
        let expected = shape.external_logic_inputs();
        if operands.len() != expected {
            return Err(ComSimError::WrongLogicInputCount {
                expected,
                supplied: operands.len(),
            });
        }
    }

    if targets.stg && shape.has_stg && operands.len() != shape.stg_inputs {
        return Err(ComSimError::WrongStgInputCount {
            expected: shape.stg_inputs,
            supplied: operands.len(),
        });
    }

    let input_values = parse_bit_operands(&operands, SimCommandKind::Simulate)?;
    Ok(SimulatePlan {
        targets,
        input_values,
    })
}

pub fn parse_set_state_args<I, S>(
    args: I,
    shape: &NetworkSimulationShape,
) -> Result<SetStatePlan, ComSimError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    if shape.primary_inputs == 0 && !shape.has_stg {
        return Ok(SetStatePlan {
            targets: SimTargets::default(),
            state_name: None,
            stg_lookup: StgLookupMode::SymbolicName,
        });
    }

    let (targets, operands) = parse_target_options(args, SimCommandKind::SetState)?;
    let stg_lookup = if targets.logic && shape.primary_inputs != 0 {
        StgLookupMode::Encoding
    } else {
        StgLookupMode::SymbolicName
    };

    match operands.as_slice() {
        [] => Ok(SetStatePlan {
            targets,
            state_name: None,
            stg_lookup,
        }),
        [state] => {
            if targets.logic && shape.latches != 0 {
                validate_latch_state(state, shape.latches)?;
            }
            Ok(SetStatePlan {
                targets,
                state_name: Some(state.clone()),
                stg_lookup,
            })
        }
        _ => Err(ComSimError::SetStateNeedsOneState),
    }
}

pub fn parse_print_state_args<I, S>(args: I) -> Result<(), ComSimError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = args.into_iter().count();
    if args == 0 {
        Ok(())
    } else {
        Err(ComSimError::PrintStateTakesNoArguments)
    }
}

pub fn parse_sim_verify_args<I, S>(args: I) -> Result<SimVerifyPlan, ComSimError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut iter = args.into_iter().map(|arg| arg.as_ref().to_owned());
    let mut patterns = 1024usize;
    let mut operands = Vec::new();

    while let Some(arg) = iter.next() {
        if arg == "-n" || arg.starts_with("-n") && arg.len() > 2 {
            let value = if arg == "-n" {
                iter.next().ok_or(ComSimError::MissingOptionValue('n'))?
            } else {
                arg[2..].to_owned()
            };
            patterns = value
                .parse()
                .map_err(|_| ComSimError::InvalidInteger { option: 'n', value })?;
            if patterns < 1 {
                return Err(ComSimError::InvalidPatternCount(patterns));
            }
            if patterns % 32 != 0 {
                patterns += 32 - patterns % 32;
            }
        } else if arg.starts_with('-') {
            return Err(ComSimError::UnsupportedOption(arg));
        } else {
            operands.push(arg);
            operands.extend(iter);
            break;
        }
    }

    match operands.as_slice() {
        [filename] => Ok(SimVerifyPlan {
            patterns,
            filename: filename.clone(),
        }),
        _ => Err(ComSimError::SimVerifyNeedsNetwork),
    }
}

pub fn parse_command<I, S>(
    command_name: &str,
    args: I,
    shape: &NetworkSimulationShape,
) -> Result<SimCommand, ComSimError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name {
        "simulate" => parse_simulate_args(args, shape).map(SimCommand::Simulate),
        "set_state" => parse_set_state_args(args, shape).map(SimCommand::SetState),
        "print_state" => {
            parse_print_state_args(args)?;
            Ok(SimCommand::PrintState)
        }
        "sim_verify" => parse_sim_verify_args(args).map(SimCommand::SimVerify),
        _ => Err(ComSimError::UnsupportedOption(command_name.to_owned())),
    }
}

pub fn execute_command<Network>(
    _network: &mut Network,
    command: &SimCommand,
) -> Result<(), ComSimError> {
    let kind = match command {
        SimCommand::Simulate(_) => SimCommandKind::Simulate,
        SimCommand::SetState(_) => SimCommandKind::SetState,
        SimCommand::PrintState => SimCommandKind::PrintState,
        SimCommand::SimVerify(_) => SimCommandKind::SimVerify,
    };

    Err(blocked(kind))
}

pub fn format_network_simulation(
    output_values: &[Bit],
    real_output_mask: &[bool],
    next_state: &[Bit],
) -> String {
    let mut output = String::new();
    output.push_str("\nNetwork simulation:\n");
    output.push_str("Outputs:");
    for (value, is_real_po) in output_values.iter().zip(real_output_mask) {
        if *is_real_po {
            output.push(' ');
            output.push_str(&value.as_i32().to_string());
        }
    }
    output.push_str("\nNext state: ");
    for value in next_state {
        output.push(value.as_char());
    }
    output.push('\n');
    output
}

pub fn format_stg_simulation(step: Option<StgSimulationStep>) -> String {
    let mut output = String::new();
    output.push_str("\nSTG simulation:\n");
    let Some(step) = step else {
        output.push_str("Next state cannot be determined\n");
        return output;
    };

    output.push_str("Outputs:");
    for value in step.outputs.chars() {
        output.push(' ');
        output.push(value);
    }
    output.push_str("\nNext state: ");
    output.push_str(&step.next_state_name);
    output.push_str(" (");
    output.push_str(&step.next_state_encoding);
    output.push_str(")\n\n");
    output
}

pub fn format_print_state(network_state: Option<&str>, stg_state: Option<(&str, &str)>) -> String {
    let mut output = String::from("\n");
    if let Some(state) = network_state {
        output.push_str("Network state: ");
        output.push_str(state);
        output.push_str("\n\n");
    }
    if let Some((name, encoding)) = stg_state {
        output.push_str("STG state: ");
        output.push_str(name);
        output.push_str(" (");
        output.push_str(encoding);
        output.push_str(")\n\n");
    }
    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StgSimulationStep {
    pub outputs: String,
    pub next_state_name: String,
    pub next_state_encoding: String,
}

pub fn transition_matches(pattern: &str, vector: &str) -> bool {
    let mut vector_chars = vector.chars();
    for pattern_char in pattern.chars() {
        let vector_char = vector_chars.next().unwrap_or('\0');
        if pattern_char != '-' && vector_char != '-' && pattern_char != vector_char {
            return false;
        }
    }
    true
}

pub fn format_verification_pass(patterns: usize) -> String {
    format!("Passed verification with {patterns} random input vectors\n")
}

pub fn format_verification_failure(
    input_values: &[u32],
    internal_outputs: &[u32],
    read_in_outputs: &[u32],
    faulty_output: usize,
) -> String {
    let diff = internal_outputs[faulty_output] ^ read_in_outputs[faulty_output];
    let mask = diff & diff.wrapping_neg();

    let mut output = String::new();
    output.push_str("verification failed on input value: ");
    push_masked_pattern(&mut output, input_values, mask);
    output.push_str("\ninternal network outputs: ");
    push_masked_pattern(&mut output, internal_outputs, mask);
    output.push_str("\nread-in network outputs: ");
    push_masked_pattern(&mut output, read_in_outputs, mask);
    output.push_str("\nNote that the inputs and outputs are matched up by order, not by name\n");
    output
}

fn parse_target_options<I, S>(
    args: I,
    command: SimCommandKind,
) -> Result<(SimTargets, Vec<String>), ComSimError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut targets = SimTargets::default();
    let mut operands = Vec::new();
    let mut scanning_options = true;

    for arg in args {
        let arg = arg.as_ref().to_owned();
        if !scanning_options || !arg.starts_with('-') || arg == "-" {
            operands.push(arg);
            scanning_options = false;
            continue;
        }
        if arg == "--" {
            scanning_options = false;
            continue;
        }
        let flags = &arg[1..];
        if flags.is_empty() {
            operands.push(arg);
            scanning_options = false;
            continue;
        }
        for flag in flags.chars() {
            match flag {
                'i' => targets.stg = false,
                's' => targets.logic = false,
                _ => return Err(ComSimError::UnsupportedOption(format!("-{flag}"))),
            }
        }
    }

    if !targets.logic && !targets.stg {
        targets = SimTargets::default();
    }

    if command == SimCommandKind::SetState && operands.len() > 1 {
        return Err(ComSimError::SetStateNeedsOneState);
    }

    Ok((targets, operands))
}

fn parse_bit_operands(
    operands: &[String],
    command: SimCommandKind,
) -> Result<Vec<Bit>, ComSimError> {
    operands
        .iter()
        .map(|operand| {
            Bit::parse(operand).ok_or_else(|| ComSimError::BadBit {
                command,
                value: operand.clone(),
            })
        })
        .collect()
}

fn validate_latch_state(state: &str, latches: usize) -> Result<(), ComSimError> {
    if state.len() != latches {
        return Err(ComSimError::WrongLatchStateLength {
            expected: latches,
            supplied: state.len(),
        });
    }

    for value in state.chars() {
        if value != '0' && value != '1' {
            return Err(ComSimError::BadStateBit { value });
        }
    }

    Ok(())
}

fn push_masked_pattern(output: &mut String, values: &[u32], mask: u32) {
    for value in values {
        output.push_str(if value & mask != 0 { "1 " } else { "0 " });
    }
}

fn blocked(command: SimCommandKind) -> ComSimError {
    ComSimError::Blocked { command }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequential_shape() -> NetworkSimulationShape {
        NetworkSimulationShape {
            primary_inputs: 5,
            latches: 2,
            has_stg: true,
            stg_inputs: 3,
        }
    }

    #[test]
    fn command_registration_matches_init_sim_under_sis() {
        assert_eq!(
            sim_command_registrations(),
            &[
                CommandRegistration {
                    name: "simulate",
                    kind: SimCommandKind::Simulate,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "set_state",
                    kind: SimCommandKind::SetState,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "print_state",
                    kind: SimCommandKind::PrintState,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "sim_verify",
                    kind: SimCommandKind::SimVerify,
                    changes_network: false,
                },
            ]
        );
    }

    #[test]
    fn parses_simulate_targets_and_input_counts_like_c() {
        let plan = parse_simulate_args(["-i", "1", "0", "1"], &sequential_shape()).unwrap();
        assert_eq!(
            plan,
            SimulatePlan {
                targets: SimTargets {
                    logic: true,
                    stg: false,
                },
                input_values: vec![Bit::One, Bit::Zero, Bit::One],
            }
        );

        let both = parse_simulate_args(["-is", "1", "0", "1"], &sequential_shape()).unwrap();
        assert_eq!(both.targets, SimTargets::default());
        assert_eq!(
            parse_simulate_args(["1", "0"], &sequential_shape()),
            Err(ComSimError::WrongLogicInputCount {
                expected: 3,
                supplied: 2
            })
        );
        assert_eq!(
            parse_simulate_args(["-s", "1", "0"], &sequential_shape()),
            Err(ComSimError::WrongStgInputCount {
                expected: 3,
                supplied: 2
            })
        );
    }

    #[test]
    fn parse_simulate_rejects_non_binary_values() {
        assert_eq!(
            parse_simulate_args(["1", "x", "0"], &sequential_shape()),
            Err(ComSimError::BadBit {
                command: SimCommandKind::Simulate,
                value: "x".to_owned(),
            })
        );
    }

    #[test]
    fn parses_set_state_reset_logic_and_stg_lookup_modes() {
        let reset = parse_set_state_args(std::iter::empty::<&str>(), &sequential_shape()).unwrap();
        assert_eq!(reset.state_name, None);
        assert_eq!(reset.stg_lookup, StgLookupMode::Encoding);

        let stg_only = parse_set_state_args(["-s", "idle"], &sequential_shape()).unwrap();
        assert_eq!(stg_only.targets.logic, false);
        assert_eq!(stg_only.stg_lookup, StgLookupMode::SymbolicName);
        assert_eq!(stg_only.state_name.as_deref(), Some("idle"));

        assert_eq!(
            parse_set_state_args(["101"], &sequential_shape()),
            Err(ComSimError::WrongLatchStateLength {
                expected: 2,
                supplied: 3
            })
        );
        assert_eq!(
            parse_set_state_args(["0x"], &sequential_shape()),
            Err(ComSimError::BadStateBit { value: 'x' })
        );
    }

    #[test]
    fn parses_sim_verify_default_and_rounding() {
        assert_eq!(
            parse_sim_verify_args(["-n", "33", "other.blif"]).unwrap(),
            SimVerifyPlan {
                patterns: 64,
                filename: "other.blif".to_owned(),
            }
        );
        assert_eq!(
            parse_sim_verify_args(["network.blif"]).unwrap(),
            SimVerifyPlan {
                patterns: 1024,
                filename: "network.blif".to_owned(),
            }
        );
        assert_eq!(
            parse_sim_verify_args(["-n0", "network.blif"]),
            Err(ComSimError::InvalidPatternCount(0))
        );
    }

    #[test]
    fn formatting_matches_com_sim_output_shape() {
        assert_eq!(
            format_network_simulation(
                &[Bit::One, Bit::Zero, Bit::One],
                &[true, false, true],
                &[Bit::Zero, Bit::One],
            ),
            "\nNetwork simulation:\nOutputs: 1 1\nNext state: 01\n"
        );

        assert_eq!(
            format_stg_simulation(Some(StgSimulationStep {
                outputs: "10".to_owned(),
                next_state_name: "S1".to_owned(),
                next_state_encoding: "01".to_owned(),
            })),
            "\nSTG simulation:\nOutputs: 1 0\nNext state: S1 (01)\n\n"
        );

        assert_eq!(
            format_print_state(Some("10"), Some(("S1", "01"))),
            "\nNetwork state: 10\n\nSTG state: S1 (01)\n\n"
        );
    }

    #[test]
    fn transition_matching_and_verification_messages_match_interpret_helpers_used_by_com_sim() {
        assert!(transition_matches("1-0", "110"));
        assert!(transition_matches("---", "010"));
        assert!(transition_matches("01", "010"));
        assert!(!transition_matches("010", "01"));

        assert_eq!(
            format_verification_pass(64),
            "Passed verification with 64 random input vectors\n"
        );
        assert_eq!(
            format_verification_failure(&[0b0010, 0], &[0b0100], &[0], 0),
            concat!(
                "verification failed on input value: 0 0 \n",
                "internal network outputs: 1 \n",
                "read-in network outputs: 0 \n",
                "Note that the inputs and outputs are matched up by order, not by name\n",
            )
        );
    }

    #[test]
    fn dispatch_reports_missing_native_prerequisites() {
        let mut network = ();
        let command = SimCommand::SimVerify(SimVerifyPlan {
            patterns: 32,
            filename: "other.blif".to_owned(),
        });
        let error = execute_command(&mut network, &command).unwrap_err();

        assert_eq!(
            error,
            ComSimError::Blocked {
                command: SimCommandKind::SimVerify,
            }
        );
        assert!(
            error
                .to_string()
                .contains("requires native Rust ports for SIS dependencies")
        );
    }
}
