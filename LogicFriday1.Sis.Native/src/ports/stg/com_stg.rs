//! Native Rust scaffold for `LogicSynthesis/sis/stg/com_stg.c`.
//!
//! The C file is mostly SIS command plumbing around STG extraction, external
//! state assignment/minimization tools, Espresso-based STG-to-network
//! conversion, and clock/network mutation. This module ports the command option
//! models plus independent STG formatting/encoding behavior. Operations that
//! still require SIS `network_t`, command registration, PLA/KISS readers, latch
//! wiring, or Espresso integration report generic missing-dependency errors.

use std::error::Error;
use std::fmt;

use super::stg::{StateId, Stg, StgError};

pub const STG_LATCH_LIMIT: usize = 16;
pub const STG_CHECK_EDGE_LIMIT: usize = 500;

pub const STG_EXTRACT_USAGE: &str = concat!(
    "Usage: stg_extract [-a] [-e] [-c]\n",
    "\t-a: find for all possible start states\n",
    "\t    (only if there are no more than 16 latches)\n",
    "\t-e: extract even if the number of latches exceeds 16\n",
    "\t-c: always check that the network covers the STG\n",
);

pub const STG_TO_NETWORK_USAGE: &str = "usage: stg_to_network [-e option]\n";
pub const STATE_ASSIGN_USAGE: &str = "usage: state_assign program_name options\n";
pub const STATE_MINIMIZE_USAGE: &str = "usage: state_mininimize program_name options\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComStgPortDisposition {
    BlockedByUnportedCommandNetworkIoAndEspressoApis,
}

pub fn com_stg_port_disposition() -> ComStgPortDisposition {
    ComStgPortDisposition::BlockedByUnportedCommandNetworkIoAndEspressoApis
}

pub fn com_stg_port_is_blocked() -> bool {
    com_stg_port_disposition()
        == ComStgPortDisposition::BlockedByUnportedCommandNetworkIoAndEspressoApis
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StgCommandKind {
    StgExtract,
    StgToNetwork,
    StateAssign,
    StateMinimize,
    StgCover,
    OneHot,
    StgDumpGraph,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StgCommandRegistration {
    pub name: &'static str,
    pub kind: StgCommandKind,
    pub changes_network: bool,
}

pub const STG_COMMANDS: &[StgCommandRegistration] = &[
    StgCommandRegistration {
        name: "stg_extract",
        kind: StgCommandKind::StgExtract,
        changes_network: true,
    },
    StgCommandRegistration {
        name: "stg_to_network",
        kind: StgCommandKind::StgToNetwork,
        changes_network: true,
    },
    StgCommandRegistration {
        name: "state_assign",
        kind: StgCommandKind::StateAssign,
        changes_network: true,
    },
    StgCommandRegistration {
        name: "state_minimize",
        kind: StgCommandKind::StateMinimize,
        changes_network: true,
    },
    StgCommandRegistration {
        name: "stg_cover",
        kind: StgCommandKind::StgCover,
        changes_network: false,
    },
    StgCommandRegistration {
        name: "one_hot",
        kind: StgCommandKind::OneHot,
        changes_network: true,
    },
    StgCommandRegistration {
        name: "_stg_dump_graph",
        kind: StgCommandKind::StgDumpGraph,
        changes_network: false,
    },
];

pub fn stg_command_registrations() -> &'static [StgCommandRegistration] {
    STG_COMMANDS
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StgExtractOptions {
    pub all_start_states: bool,
    pub override_latch_limit: bool,
    pub check_containment: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetworkLatchSummary {
    pub latch_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StgExtractPlan {
    pub options: StgExtractOptions,
    pub latch_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StgExtractionTotals {
    pub states: usize,
    pub edges: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StgToNetworkOptions {
    pub encoding_option: u8,
}

impl Default for StgToNetworkOptions {
    fn default() -> Self {
        Self { encoding_option: 1 }
    }
}

impl StgToNetworkOptions {
    pub fn stg_single(self) -> bool {
        self.encoding_option != 0
    }

    pub fn espresso_mode(self) -> EspressoMode {
        if self.encoding_option == 2 {
            EspressoMode::FdSo
        } else {
            EspressoMode::Fd
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EspressoMode {
    Fd,
    FdSo,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalStateToolInvocation {
    pub program: String,
    pub options: Vec<String>,
    pub help_requested: bool,
    pub notice: Option<ExternalToolNotice>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalToolNotice {
    Nova,
    Jedi,
    Stamina,
    Freduce,
    Sred,
}

impl ExternalToolNotice {
    pub fn message(self) -> &'static str {
        match self {
            Self::Nova => "Running nova, written by Tiziano Villa,  UC Berkeley\n",
            Self::Jedi => "Running jedi, written by Bill Lin,  UC Berkeley\n",
            Self::Stamina => {
                "Running stamina, written by June Rho, University of Colorado at Boulder\n"
            }
            Self::Freduce => "Running freduce, written by Bill Lin,  UC Berkeley\n",
            Self::Sred => "Running sred, written by Tiziano Villa,  UC Berkeley\n",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComStgError {
    UnsupportedOption(String),
    MissingOptionValue(char),
    InvalidNumber { option: char, value: String },
    InvalidEncodingOption(i32),
    StateToolUsage { usage: &'static str },
    NoLatches,
    TooManyLatchesForAllStartStates { latches: usize, limit: usize },
    TooManyLatchesWithoutOverride { latches: usize, limit: usize },
    MissingStartState,
    MissingStartEncoding,
    MissingStateEncoding { state: StateId },
    StgModel(StgError),
    Blocked { command: StgCommandKind },
}

impl fmt::Display for ComStgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::MissingOptionValue(option) => write!(f, "missing value for option -{option}"),
            Self::InvalidNumber { option, value } => {
                write!(f, "invalid numeric value for option -{option}: {value}")
            }
            Self::InvalidEncodingOption(value) => {
                write!(
                    f,
                    "invalid stg_to_network encoding option {value}; expected 0, 1, or 2"
                )
            }
            Self::StateToolUsage { usage } => write!(f, "{usage}"),
            Self::NoLatches => write!(f, "network has no latches"),
            Self::TooManyLatchesForAllStartStates { latches, limit } => write!(
                f,
                "network has {latches} latches; all-start-state extraction is limited to {limit}"
            ),
            Self::TooManyLatchesWithoutOverride { latches, limit } => write!(
                f,
                "network has {latches} latches; use override for extraction above {limit}"
            ),
            Self::MissingStartState => write!(f, "STG has no start state"),
            Self::MissingStartEncoding => write!(f, "FSM has no encoding"),
            Self::MissingStateEncoding { state } => {
                write!(f, "STG state {:?} has no encoding", state)
            }
            Self::StgModel(error) => write!(f, "{error}"),
            Self::Blocked { command } => {
                write!(f, "{command:?} is blocked by unported SIS dependencies")
            }
        }
    }
}

impl Error for ComStgError {}

impl From<StgError> for ComStgError {
    fn from(value: StgError) -> Self {
        Self::StgModel(value)
    }
}

pub fn parse_stg_extract_args<I, S>(args: I) -> Result<StgExtractOptions, ComStgError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = StgExtractOptions::default();
    for arg in args {
        let arg = arg.as_ref();
        let Some(flags) = arg.strip_prefix('-') else {
            return Err(ComStgError::UnsupportedOption(arg.to_owned()));
        };
        if flags.is_empty() {
            return Err(ComStgError::UnsupportedOption(arg.to_owned()));
        }
        for flag in flags.chars() {
            match flag {
                'a' => options.all_start_states = true,
                'e' => options.override_latch_limit = true,
                'c' => options.check_containment = true,
                _ => return Err(ComStgError::UnsupportedOption(format!("-{flag}"))),
            }
        }
    }

    Ok(options)
}

pub fn plan_stg_extract(
    network: NetworkLatchSummary,
    options: StgExtractOptions,
) -> Result<StgExtractPlan, ComStgError> {
    if network.latch_count == 0 {
        return Err(ComStgError::NoLatches);
    }
    if options.all_start_states && network.latch_count > STG_LATCH_LIMIT {
        return Err(ComStgError::TooManyLatchesForAllStartStates {
            latches: network.latch_count,
            limit: STG_LATCH_LIMIT,
        });
    }
    if network.latch_count > STG_LATCH_LIMIT && !options.override_latch_limit {
        return Err(ComStgError::TooManyLatchesWithoutOverride {
            latches: network.latch_count,
            limit: STG_LATCH_LIMIT,
        });
    }

    Ok(StgExtractPlan {
        options,
        latch_count: network.latch_count,
    })
}

pub fn should_check_network_stg_cover(
    options: StgExtractOptions,
    totals: StgExtractionTotals,
) -> bool {
    options.check_containment || totals.edges <= STG_CHECK_EDGE_LIMIT
}

pub fn stg_extract(_plan: StgExtractPlan) -> Result<(), ComStgError> {
    Err(blocked(StgCommandKind::StgExtract))
}

pub fn parse_stg_to_network_args<I, S>(args: I) -> Result<StgToNetworkOptions, ComStgError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = StgToNetworkOptions::default();
    let mut iter = args.into_iter().map(|arg| arg.as_ref().to_owned());

    while let Some(arg) = iter.next() {
        let Some(value) = option_value(&arg, 'e', &mut iter)? else {
            return Err(ComStgError::UnsupportedOption(arg));
        };
        let parsed = parse_i32('e', &value)?;
        if !(0..=2).contains(&parsed) {
            return Err(ComStgError::InvalidEncodingOption(parsed));
        }
        options.encoding_option = parsed as u8;
    }

    Ok(options)
}

pub fn stg_to_network(_stg: &Stg, _options: StgToNetworkOptions) -> Result<(), ComStgError> {
    Err(blocked(StgCommandKind::StgToNetwork))
}

pub fn parse_state_assign_args<I, S>(args: I) -> Result<ExternalStateToolInvocation, ComStgError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_external_state_tool_args(args, "nova", true, STATE_ASSIGN_USAGE)
}

pub fn parse_state_minimize_args<I, S>(args: I) -> Result<ExternalStateToolInvocation, ComStgError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_external_state_tool_args(args, "stamina", false, STATE_MINIMIZE_USAGE)
}

pub fn state_assign(_invocation: ExternalStateToolInvocation) -> Result<(), ComStgError> {
    Err(blocked(StgCommandKind::StateAssign))
}

pub fn state_minimize(_invocation: ExternalStateToolInvocation) -> Result<(), ComStgError> {
    Err(blocked(StgCommandKind::StateMinimize))
}

pub fn stg_cover() -> Result<(), ComStgError> {
    Err(blocked(StgCommandKind::StgCover))
}

pub fn one_hot(_stg: &mut Stg) -> Result<(), ComStgError> {
    Err(blocked(StgCommandKind::OneHot))
}

pub fn stg_dump_graph() -> Result<(), ComStgError> {
    Err(blocked(StgCommandKind::StgDumpGraph))
}

pub fn assign_one_hot_encodings(stg: &mut Stg) -> Result<Vec<String>, ComStgError> {
    let state_count = stg.num_states();
    let mut encodings = Vec::with_capacity(state_count);
    for index in 0..state_count {
        let mut code = vec!['0'; state_count];
        if let Some(bit) = code.get_mut(index) {
            *bit = '1';
        }
        let code: String = code.into_iter().collect();
        stg.set_state_encoding(StateId(index), code.clone())?;
        encodings.push(code);
    }
    Ok(encodings)
}

pub fn write_encoded_espresso_format(stg: &Stg) -> Result<String, ComStgError> {
    let start = stg.start().ok_or(ComStgError::MissingStartState)?;
    let state_bits = stg
        .state_encoding(start)
        .ok_or(ComStgError::MissingStartEncoding)?
        .len();

    let mut output = String::new();
    output.push_str(".type fr\n");
    output.push_str(&format!(".i {}\n", stg.num_inputs() + state_bits));
    output.push_str(&format!(".o {}\n", stg.num_outputs() + state_bits));

    for transition in stg.transitions() {
        let from_encoding =
            stg.state_encoding(transition.from)
                .ok_or(ComStgError::MissingStateEncoding {
                    state: transition.from,
                })?;
        let to_encoding =
            stg.state_encoding(transition.to)
                .ok_or(ComStgError::MissingStateEncoding {
                    state: transition.to,
                })?;

        output.push_str(&format!(
            "{} {} {} {}\n",
            transition.input, from_encoding, to_encoding, transition.output
        ));
    }

    output.push_str(".e\n");
    Ok(output)
}

fn parse_external_state_tool_args<I, S>(
    args: I,
    default_program: &str,
    accept_help_word: bool,
    usage: &'static str,
) -> Result<ExternalStateToolInvocation, ComStgError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();
    if args.iter().any(|arg| arg == "-x") {
        return Err(ComStgError::StateToolUsage { usage });
    }

    let (program, options) = match args.split_first() {
        Some((program, options)) => (program.clone(), options.to_vec()),
        None => (default_program.to_owned(), Vec::new()),
    };
    let help_requested = args
        .iter()
        .any(|arg| arg == "-h" || (accept_help_word && arg == "-help"));

    Ok(ExternalStateToolInvocation {
        notice: external_tool_notice(&program),
        program,
        options,
        help_requested,
    })
}

fn external_tool_notice(program: &str) -> Option<ExternalToolNotice> {
    match program {
        "nova" => Some(ExternalToolNotice::Nova),
        "jedi" => Some(ExternalToolNotice::Jedi),
        "stamina" => Some(ExternalToolNotice::Stamina),
        "freduce" => Some(ExternalToolNotice::Freduce),
        "sred" => Some(ExternalToolNotice::Sred),
        _ => None,
    }
}

fn option_value<I>(arg: &str, option: char, iter: &mut I) -> Result<Option<String>, ComStgError>
where
    I: Iterator<Item = String>,
{
    let short = format!("-{option}");
    if arg == short {
        return iter
            .next()
            .map(Some)
            .ok_or(ComStgError::MissingOptionValue(option));
    }

    Ok(arg
        .strip_prefix(&short)
        .filter(|value| !value.is_empty())
        .map(str::to_owned))
}

fn parse_i32(option: char, value: &str) -> Result<i32, ComStgError> {
    value.parse().map_err(|_| ComStgError::InvalidNumber {
        option,
        value: value.to_owned(),
    })
}

fn blocked(command: StgCommandKind) -> ComStgError {
    ComStgError::Blocked { command }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stg_extract_options_and_latch_guards() {
        let options = parse_stg_extract_args(["-ac"]).unwrap();
        assert_eq!(
            options,
            StgExtractOptions {
                all_start_states: true,
                override_latch_limit: false,
                check_containment: true,
            }
        );
        assert_eq!(
            plan_stg_extract(NetworkLatchSummary { latch_count: 0 }, options),
            Err(ComStgError::NoLatches)
        );
        assert_eq!(
            plan_stg_extract(NetworkLatchSummary { latch_count: 17 }, options),
            Err(ComStgError::TooManyLatchesForAllStartStates {
                latches: 17,
                limit: STG_LATCH_LIMIT,
            })
        );

        let override_options = parse_stg_extract_args(["-e"]).unwrap();
        assert_eq!(
            plan_stg_extract(NetworkLatchSummary { latch_count: 17 }, override_options)
                .unwrap()
                .latch_count,
            17
        );
    }

    #[test]
    fn cover_check_limit_matches_c_skip_rule() {
        let defaults = StgExtractOptions::default();
        assert!(should_check_network_stg_cover(
            defaults,
            StgExtractionTotals {
                states: 10,
                edges: STG_CHECK_EDGE_LIMIT,
            }
        ));
        assert!(!should_check_network_stg_cover(
            defaults,
            StgExtractionTotals {
                states: 10,
                edges: STG_CHECK_EDGE_LIMIT + 1,
            }
        ));
        assert!(should_check_network_stg_cover(
            StgExtractOptions {
                check_containment: true,
                ..StgExtractOptions::default()
            },
            StgExtractionTotals {
                states: 10,
                edges: STG_CHECK_EDGE_LIMIT + 1,
            }
        ));
    }

    #[test]
    fn parses_stg_to_network_option_values() {
        assert_eq!(
            parse_stg_to_network_args(std::iter::empty::<&str>()).unwrap(),
            StgToNetworkOptions { encoding_option: 1 }
        );
        let options = parse_stg_to_network_args(["-e2"]).unwrap();
        assert_eq!(options.encoding_option, 2);
        assert!(options.stg_single());
        assert_eq!(options.espresso_mode(), EspressoMode::FdSo);
        assert_eq!(
            parse_stg_to_network_args(["-e", "3"]),
            Err(ComStgError::InvalidEncodingOption(3))
        );
        assert_eq!(
            parse_stg_to_network_args(["-e"]),
            Err(ComStgError::MissingOptionValue('e'))
        );
    }

    #[test]
    fn parses_external_state_tool_invocations() {
        assert_eq!(
            parse_state_assign_args(std::iter::empty::<&str>()).unwrap(),
            ExternalStateToolInvocation {
                program: "nova".to_owned(),
                options: Vec::new(),
                help_requested: false,
                notice: Some(ExternalToolNotice::Nova),
            }
        );

        assert_eq!(
            parse_state_assign_args(["jedi", "-help"]).unwrap(),
            ExternalStateToolInvocation {
                program: "jedi".to_owned(),
                options: vec!["-help".to_owned()],
                help_requested: true,
                notice: Some(ExternalToolNotice::Jedi),
            }
        );

        assert_eq!(
            parse_state_minimize_args(["freduce", "-help"])
                .unwrap()
                .help_requested,
            false
        );
        assert_eq!(
            parse_state_minimize_args(["-x"]),
            Err(ComStgError::StateToolUsage {
                usage: STATE_MINIMIZE_USAGE
            })
        );
    }

    #[test]
    fn assigns_one_hot_encodings_in_state_order() {
        let mut stg = Stg::with_dimensions(1, 1);
        let s0 = stg.create_state(Some("s0"), Some("old"));
        let s1 = stg.create_state(Some("s1"), Some("old"));
        let s2 = stg.create_state(Some("s2"), Some("old"));

        assert_eq!(
            assign_one_hot_encodings(&mut stg).unwrap(),
            ["100", "010", "001"]
        );
        assert_eq!(stg.state_encoding(s0), Some("100"));
        assert_eq!(stg.state_encoding(s1), Some("010"));
        assert_eq!(stg.state_encoding(s2), Some("001"));
    }

    #[test]
    fn writes_encoded_espresso_format_like_com_stg_c() {
        let mut stg = Stg::with_dimensions(2, 1);
        let s0 = stg.create_state(Some("s0"), Some("00"));
        let s1 = stg.create_state(Some("s1"), Some("01"));
        stg.set_start(s0).unwrap();
        stg.create_transition(s0, s1, "1-", "0").unwrap();
        stg.create_transition(s1, s0, "0-", "1").unwrap();

        assert_eq!(
            write_encoded_espresso_format(&stg).unwrap(),
            ".type fr\n.i 4\n.o 3\n1- 00 01 0\n0- 01 00 1\n.e\n"
        );
    }

    #[test]
    fn reports_missing_encoding_and_generic_blocked_error() {
        let mut stg = Stg::with_dimensions(1, 1);
        let s0 = stg.create_state(Some("s0"), None::<String>);
        stg.set_start(s0).unwrap();
        assert_eq!(
            write_encoded_espresso_format(&stg),
            Err(ComStgError::MissingStartEncoding)
        );

        assert!(com_stg_port_is_blocked());
        assert_eq!(stg_command_registrations().len(), 7);

        let error = stg_extract(StgExtractPlan {
            options: StgExtractOptions::default(),
            latch_count: 1,
        })
        .expect_err("native network extraction should be blocked");
        let ComStgError::Blocked { command } = error else {
            panic!("unexpected error kind");
        };
        assert_eq!(command, StgCommandKind::StgExtract);
    }
}
