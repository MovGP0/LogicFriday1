//! Native Rust model for `LogicSynthesis/sis/seqbdd/com_verify.c`.
//!
//! The C file is the command and option layer for sequential BDD verification,
//! range computation, environment-constrained don't-care extraction, latch
//! removal, latch-output retiming, equivalent-net merging, dependency removal,
//! and DC-network cleanup. This port keeps the deterministic command table,
//! option parsing, status mapping, and early-exit behavior in Rust. Operations
//! that still require SIS command execution, `network_t`, `array_t`, BDDs, or
//! PRL mutation routines return explicit missing-port errors.

use std::error::Error;
use std::fmt;

const INFINITY: u32 = u32::MAX;
const MAX_TIMEOUT_SECONDS: u32 = 3600 * 24 * 365;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeqBddCommand {
    VerifyFsm,
    ExtractSeqDc,
    EnvSeqDc,
    EnvVerifyFsm,
    RemoveLatches,
    LatchOutput,
    EquivNets,
    RemoveDependencies,
    FreeDc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub command: SeqBddCommand,
    pub changes_network: bool,
}

pub const COMMAND_TABLE: &[CommandRegistration] = &[
    CommandRegistration {
        name: "verify_fsm",
        command: SeqBddCommand::VerifyFsm,
        changes_network: true,
    },
    CommandRegistration {
        name: "extract_seq_dc",
        command: SeqBddCommand::ExtractSeqDc,
        changes_network: true,
    },
    CommandRegistration {
        name: "env_seq_dc",
        command: SeqBddCommand::EnvSeqDc,
        changes_network: true,
    },
    CommandRegistration {
        name: "env_verify_fsm",
        command: SeqBddCommand::EnvVerifyFsm,
        changes_network: false,
    },
    CommandRegistration {
        name: "remove_latches",
        command: SeqBddCommand::RemoveLatches,
        changes_network: true,
    },
    CommandRegistration {
        name: "latch_output",
        command: SeqBddCommand::LatchOutput,
        changes_network: true,
    },
    CommandRegistration {
        name: "equiv_nets",
        command: SeqBddCommand::EquivNets,
        changes_network: true,
    },
    CommandRegistration {
        name: "remove_dep",
        command: SeqBddCommand::RemoveDependencies,
        changes_network: true,
    },
    CommandRegistration {
        name: "free_dc",
        command: SeqBddCommand::FreeDc,
        changes_network: true,
    },
];

pub fn init_seqbdd_registry() -> Result<&'static [CommandRegistration], ComVerifyError> {
    Err(ComVerifyError::MissingNativePorts {
        operation: "init_seqbdd",
    })
}

pub fn native_command_table() -> &'static [CommandRegistration] {
    COMMAND_TABLE
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RangeMethod {
    Consistency,
    Bull,
    Product,
}

impl RangeMethod {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "consistency" => Some(Self::Consistency),
            "bull" => Some(Self::Bull),
            "product" => Some(Self::Product),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Consistency => "consistency",
            Self::Bull => "bull",
            Self::Product => "product",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeOptions {
    pub timeout_seconds: u32,
    pub keep_old_network: bool,
    pub n_iter: u32,
    pub verbose: i32,
    pub use_manual_order: bool,
    pub order_network_name: Option<String>,
    pub ordering_depth: i32,
    pub sim_file: Option<String>,
    pub stop_if_verify: bool,
    pub method: RangeMethod,
}

impl Default for RangeOptions {
    fn default() -> Self {
        Self {
            timeout_seconds: 0,
            keep_old_network: true,
            n_iter: 1,
            verbose: 0,
            use_manual_order: false,
            order_network_name: None,
            ordering_depth: 2,
            sim_file: None,
            stop_if_verify: false,
            method: RangeMethod::Product,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeParseResult {
    pub options: RangeOptions,
    pub operands: Vec<String>,
}

pub fn parse_range_options(args: &[&str]) -> Result<RangeParseResult, ComVerifyError> {
    let mut options = RangeOptions::default();
    let operands = parse_options(args, "imoOtvs", |option, value| match option {
        'i' => {
            let n_iter = parse_u32_option(option, value)?;
            if n_iter > INFINITY {
                return Err(ComVerifyError::InvalidOptionValue {
                    option,
                    value: value.to_owned(),
                });
            }
            options.n_iter = n_iter;
            Ok(())
        }
        'm' => {
            options.method =
                RangeMethod::parse(value).ok_or_else(|| ComVerifyError::UnknownRangeMethod {
                    method: value.to_owned(),
                })?;
            Ok(())
        }
        'o' => {
            options.ordering_depth = parse_i32_option(option, value)?;
            Ok(())
        }
        'O' => {
            options.use_manual_order = true;
            options.order_network_name = Some(value.to_owned());
            Ok(())
        }
        't' => {
            let timeout = parse_u32_option(option, value)?;
            if timeout > MAX_TIMEOUT_SECONDS {
                return Err(ComVerifyError::InvalidOptionValue {
                    option,
                    value: value.to_owned(),
                });
            }
            options.timeout_seconds = timeout;
            Ok(())
        }
        'v' => {
            options.verbose = parse_i32_option(option, value)?;
            Ok(())
        }
        's' => {
            options.sim_file = Some(value.to_owned());
            Ok(())
        }
        'V' => {
            options.stop_if_verify = true;
            Ok(())
        }
        _ => Err(ComVerifyError::UnknownOption(option)),
    })?;

    Ok(RangeParseResult { options, operands })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrlRemoveLatchOptions {
    pub max_cost: u32,
    pub max_level: u32,
    pub local_retiming: bool,
    pub remove_boot: bool,
}

impl Default for PrlRemoveLatchOptions {
    fn default() -> Self {
        Self {
            max_cost: INFINITY,
            max_level: INFINITY,
            local_retiming: true,
            remove_boot: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrlOptions {
    pub verbose: i32,
    pub ordering_depth: i32,
    pub timeout_seconds: u32,
    pub stop_if_verify: bool,
    pub method: RangeMethod,
    pub remlatch: PrlRemoveLatchOptions,
}

impl Default for PrlOptions {
    fn default() -> Self {
        Self {
            verbose: 0,
            ordering_depth: 1,
            timeout_seconds: 0,
            stop_if_verify: false,
            method: RangeMethod::Product,
            remlatch: PrlRemoveLatchOptions::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrlParseResult {
    pub options: PrlOptions,
    pub operands: Vec<String>,
}

pub fn parse_prl_options(args: &[&str]) -> Result<PrlParseResult, ComVerifyError> {
    let mut options = PrlOptions::default();
    let operands = parse_options(args, "otvlf", |option, value| match option {
        'o' => {
            options.ordering_depth = parse_i32_option(option, value)?;
            Ok(())
        }
        't' => {
            let timeout = parse_u32_option(option, value)?;
            if timeout > MAX_TIMEOUT_SECONDS {
                return Err(ComVerifyError::InvalidOptionValue {
                    option,
                    value: value.to_owned(),
                });
            }
            options.timeout_seconds = timeout;
            Ok(())
        }
        'v' => {
            options.verbose = parse_i32_option(option, value)?;
            Ok(())
        }
        'l' => {
            let max_level = parse_u32_option(option, value)?;
            if max_level <= 1 {
                return Err(ComVerifyError::InvalidOptionValue {
                    option,
                    value: value.to_owned(),
                });
            }
            options.remlatch.max_level = max_level;
            Ok(())
        }
        'f' => {
            let max_cost = parse_u32_option(option, value)?;
            if max_cost < 1 {
                return Err(ComVerifyError::InvalidOptionValue {
                    option,
                    value: value.to_owned(),
                });
            }
            options.remlatch.max_cost = max_cost;
            Ok(())
        }
        'r' => {
            options.remlatch.local_retiming = false;
            Ok(())
        }
        'i' => {
            options.remlatch.remove_boot = false;
            Ok(())
        }
        'V' => {
            options.stop_if_verify = true;
            Ok(())
        }
        'p' => Err(ComVerifyError::UnsupportedPrlOption('p')),
        _ => Err(ComVerifyError::UnknownOption(option)),
    })?;

    Ok(PrlParseResult { options, operands })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveDepOptions {
    pub verbosity: i32,
    pub perform_check: bool,
    pub insert_a_one: bool,
}

impl Default for RemoveDepOptions {
    fn default() -> Self {
        Self {
            verbosity: 0,
            perform_check: false,
            insert_a_one: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveDepParseResult {
    pub options: RemoveDepOptions,
    pub nodes: Vec<String>,
}

pub fn parse_remove_dep_options(args: &[&str]) -> Result<RemoveDepParseResult, ComVerifyError> {
    let mut options = RemoveDepOptions::default();
    let nodes = parse_options(args, "v", |option, value| match option {
        'o' => {
            options.insert_a_one = true;
            Ok(())
        }
        'p' => {
            options.perform_check = true;
            Ok(())
        }
        'v' => {
            options.verbosity = parse_i32_option(option, value)?;
            Ok(())
        }
        _ => Err(ComVerifyError::UnknownOption(option)),
    })?;

    Ok(RemoveDepParseResult { options, nodes })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatchOutputParseResult {
    pub verbosity: i32,
    pub nodes: Vec<String>,
}

pub fn parse_latch_output_options(args: &[&str]) -> Result<LatchOutputParseResult, ComVerifyError> {
    let mut verbosity = 0;
    let nodes = parse_options(args, "v", |option, value| match option {
        'v' => {
            verbosity = parse_i32_option(option, value)?;
            Ok(())
        }
        _ => Err(ComVerifyError::UnknownOption(option)),
    })?;

    Ok(LatchOutputParseResult { verbosity, nodes })
}

fn parse_options<F>(
    args: &[&str],
    value_options: &str,
    mut apply_option: F,
) -> Result<Vec<String>, ComVerifyError>
where
    F: FnMut(char, &str) -> Result<(), ComVerifyError>,
{
    let mut operands = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = args[index];
        if arg == "--" {
            operands.extend(args[index + 1..].iter().map(|arg| (*arg).to_owned()));
            break;
        }
        if !arg.starts_with('-') || arg == "-" {
            operands.push(arg.to_owned());
            index += 1;
            continue;
        }

        let mut chars = arg[1..].chars().peekable();
        while let Some(option) = chars.next() {
            if value_options.contains(option) {
                let inline = chars.collect::<String>();
                let value = if inline.is_empty() {
                    index += 1;
                    args.get(index)
                        .copied()
                        .ok_or(ComVerifyError::MissingOptionValue(option))?
                        .to_owned()
                } else {
                    inline
                };
                apply_option(option, &value)?;
                break;
            }
            apply_option(option, "")?;
        }
        index += 1;
    }

    Ok(operands)
}

fn parse_u32_option(option: char, value: &str) -> Result<u32, ComVerifyError> {
    value
        .parse::<u32>()
        .map_err(|_| ComVerifyError::InvalidOptionValue {
            option,
            value: value.to_owned(),
        })
}

fn parse_i32_option(option: char, value: &str) -> Result<i32, ComVerifyError> {
    value
        .parse::<i32>()
        .map_err(|_| ComVerifyError::InvalidOptionValue {
            option,
            value: value.to_owned(),
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkStats {
    pub is_present: bool,
    pub internal_count: usize,
    pub latch_count: usize,
}

impl NetworkStats {
    pub fn absent() -> Self {
        Self {
            is_present: false,
            internal_count: 0,
            latch_count: 0,
        }
    }

    pub fn present(internal_count: usize, latch_count: usize) -> Self {
        Self {
            is_present: true,
            internal_count,
            latch_count,
        }
    }

    fn has_seq_logic(&self) -> bool {
        self.is_present && self.internal_count > 0 && self.latch_count > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandPlan {
    NoNetwork,
    NothingToDo,
    NeedsNetworkRead {
        command: &'static str,
        filename: String,
    },
    ExecuteSisBound {
        operation: &'static str,
    },
    Delegate {
        command_line: &'static str,
    },
}

pub fn plan_verify_fsm(
    args: &[&str],
    network: &NetworkStats,
) -> Result<CommandPlan, ComVerifyError> {
    if !network.is_present {
        return Ok(CommandPlan::NoNetwork);
    }
    let parsed = parse_range_options(args)?;
    require_operand_count("verify_fsm", &parsed.operands, 1)?;
    Ok(CommandPlan::NeedsNetworkRead {
        command: "read_blif",
        filename: parsed.operands[0].clone(),
    })
}

pub fn plan_extract_seq_dc(
    args: &[&str],
    network: &NetworkStats,
) -> Result<CommandPlan, ComVerifyError> {
    let mut parsed = parse_range_options(args)?;
    require_operand_count("extract_seq_dc", &parsed.operands, 0)?;
    parsed.options.keep_old_network = false;
    parsed.options.n_iter = INFINITY;

    if !network.has_seq_logic() {
        return Ok(CommandPlan::NothingToDo);
    }
    Ok(CommandPlan::ExecuteSisBound {
        operation: "range_computation_interface",
    })
}

pub fn plan_env_seq_dc(
    args: &[&str],
    network: &NetworkStats,
) -> Result<CommandPlan, ComVerifyError> {
    if !network.has_seq_logic() {
        return Ok(CommandPlan::NothingToDo);
    }

    let parsed = parse_prl_options(args)?;
    if parsed.operands.is_empty() {
        return Ok(CommandPlan::Delegate {
            command_line: "extract_seq_dc",
        });
    }
    require_operand_count("env_seq_dc", &parsed.operands, 1)?;
    Ok(CommandPlan::NeedsNetworkRead {
        command: "read_blif",
        filename: parsed.operands[0].clone(),
    })
}

pub fn plan_env_verify_fsm(
    args: &[&str],
    network: &NetworkStats,
) -> Result<CommandPlan, ComVerifyError> {
    if !network.is_present {
        return Ok(CommandPlan::NothingToDo);
    }
    let parsed = parse_prl_options(args)?;
    require_operand_count("env_verify_fsm", &parsed.operands, 2)?;
    Ok(CommandPlan::NeedsNetworkRead {
        command: "read_blif",
        filename: parsed.operands[1].clone(),
    })
}

pub fn plan_remove_latches(
    args: &[&str],
    network: &NetworkStats,
) -> Result<CommandPlan, ComVerifyError> {
    let parsed = parse_prl_options(args)?;
    require_operand_count("remove_latches", &parsed.operands, 0)?;
    if !network.is_present {
        return Err(ComVerifyError::NoNetworkSpecified);
    }
    if network.latch_count == 0 {
        return Ok(CommandPlan::NothingToDo);
    }
    Ok(CommandPlan::ExecuteSisBound {
        operation: "Prl_RemoveLatches",
    })
}

pub fn plan_equiv_nets(
    args: &[&str],
    network: &NetworkStats,
) -> Result<CommandPlan, ComVerifyError> {
    let parsed = parse_prl_options(args)?;
    require_operand_count("equiv_nets", &parsed.operands, 0)?;
    if !network.is_present {
        return Err(ComVerifyError::NoNetworkSpecified);
    }
    Ok(CommandPlan::ExecuteSisBound {
        operation: "Prl_EquivNets",
    })
}

pub fn plan_latch_output(args: &[&str]) -> Result<CommandPlan, ComVerifyError> {
    let parsed = parse_latch_output_options(args)?;
    if parsed.nodes.is_empty() {
        return Err(ComVerifyError::WrongOperandCount {
            command: "latch_output",
            expected: "one or more nodes",
            actual: 0,
        });
    }
    Ok(CommandPlan::ExecuteSisBound {
        operation: "Prl_LatchOutput",
    })
}

pub fn plan_remove_dependencies(args: &[&str]) -> Result<CommandPlan, ComVerifyError> {
    let parsed = parse_remove_dep_options(args)?;
    if parsed.nodes.len() < 2 {
        return Err(ComVerifyError::WrongOperandCount {
            command: "remove_dep",
            expected: "input and at least one output",
            actual: parsed.nodes.len(),
        });
    }
    Ok(CommandPlan::ExecuteSisBound {
        operation: "Prl_RemoveDependencies",
    })
}

pub fn plan_free_dc() -> CommandPlan {
    CommandPlan::ExecuteSisBound {
        operation: "Prl_RemoveDcNetwork",
    }
}

pub fn sis_bound_result<T>(operation: &'static str) -> Result<T, ComVerifyError> {
    Err(ComVerifyError::MissingNativePorts { operation })
}

pub fn read_optional_network_plan(filename: &str) -> Result<CommandPlan, ComVerifyError> {
    if filename.is_empty() {
        return Err(ComVerifyError::EmptyNetworkFilename);
    }
    Ok(CommandPlan::NeedsNetworkRead {
        command: "read_blif",
        filename: filename.to_owned(),
    })
}

pub fn map_stop_if_verify_status(raw_status: bool, stop_if_verify: bool) -> bool {
    if stop_if_verify {
        !raw_status
    } else {
        raw_status
    }
}

pub fn range_usage(command: &str, unique_options: &str) -> String {
    let suffix = if unique_options.is_empty() {
        String::new()
    } else {
        format!(" {unique_options}")
    };
    format!(
        "usage: {command} [-o d] [-t s] [-v n] [-V] -m method{suffix}\n    method is one of: consistency bull product \n"
    )
}

pub fn prl_usage(command: &str, message: &str) -> String {
    format!("usage: {command} [-o d] [-t s] [-v n] [-l m] [-f n] [-r] [-b] [-V] {message}\n")
}

fn require_operand_count(
    command: &'static str,
    operands: &[String],
    expected: usize,
) -> Result<(), ComVerifyError> {
    if operands.len() == expected {
        Ok(())
    } else {
        Err(ComVerifyError::WrongOperandCount {
            command,
            expected: match expected {
                0 => "no operands",
                1 => "one operand",
                2 => "two operands",
                _ => "fixed operand count",
            },
            actual: operands.len(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComVerifyError {
    MissingNativePorts {
        operation: &'static str,
    },
    MissingOptionValue(char),
    InvalidOptionValue {
        option: char,
        value: String,
    },
    UnknownOption(char),
    UnknownRangeMethod {
        method: String,
    },
    UnsupportedPrlOption(char),
    WrongOperandCount {
        command: &'static str,
        expected: &'static str,
        actual: usize,
    },
    NoNetworkSpecified,
    EmptyNetworkFilename,
}

impl fmt::Display for ComVerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by missing native SIS ports")
            }
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::InvalidOptionValue { option, value } => {
                write!(f, "invalid value \"{value}\" for -{option}")
            }
            Self::UnknownOption(option) => write!(f, "unknown option -{option}"),
            Self::UnknownRangeMethod { method } => {
                write!(f, "unknown sequential BDD range method \"{method}\"")
            }
            Self::UnsupportedPrlOption(option) => {
                write!(
                    f,
                    "option -{option} is parsed by C but has no implemented PRL behavior"
                )
            }
            Self::WrongOperandCount {
                command,
                expected,
                actual,
            } => write!(
                f,
                "{command} expects {expected}; received {actual} operand(s)"
            ),
            Self::NoNetworkSpecified => write!(f, "no network specified"),
            Self::EmptyNetworkFilename => write!(f, "network filename must not be empty"),
        }
    }
}

impl Error for ComVerifyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_table_matches_c_registration_order_and_mutation_flags() {
        let names = native_command_table()
            .iter()
            .map(|registration| (registration.name, registration.changes_network))
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                ("verify_fsm", true),
                ("extract_seq_dc", true),
                ("env_seq_dc", true),
                ("env_verify_fsm", false),
                ("remove_latches", true),
                ("latch_output", true),
                ("equiv_nets", true),
                ("remove_dep", true),
                ("free_dc", true),
            ]
        );
    }

    #[test]
    fn range_options_preserve_defaults_and_parse_c_switches() {
        let parsed = parse_range_options(&[
            "-i",
            "12",
            "-m",
            "bull",
            "-o",
            "3",
            "-O",
            "order.eqn",
            "-t",
            "30",
            "-v",
            "2",
            "-s",
            "trace.sim",
            "-V",
            "other.blif",
        ])
        .unwrap();

        assert_eq!(parsed.operands, vec!["other.blif"]);
        assert_eq!(parsed.options.method, RangeMethod::Bull);
        assert_eq!(parsed.options.n_iter, 12);
        assert_eq!(parsed.options.ordering_depth, 3);
        assert_eq!(
            parsed.options.order_network_name.as_deref(),
            Some("order.eqn")
        );
        assert_eq!(parsed.options.timeout_seconds, 30);
        assert_eq!(parsed.options.verbose, 2);
        assert_eq!(parsed.options.sim_file.as_deref(), Some("trace.sim"));
        assert!(parsed.options.stop_if_verify);
        assert!(parsed.options.keep_old_network);
    }

    #[test]
    fn range_options_reject_unknown_method_and_bad_timeout() {
        assert_eq!(
            parse_range_options(&["-m", "bad"]).unwrap_err(),
            ComVerifyError::UnknownRangeMethod {
                method: "bad".to_owned()
            }
        );
        assert_eq!(
            parse_range_options(&["-t", "31536001"]).unwrap_err(),
            ComVerifyError::InvalidOptionValue {
                option: 't',
                value: "31536001".to_owned(),
            }
        );
    }

    #[test]
    fn prl_options_parse_remove_latch_flags_and_bounds() {
        let parsed = parse_prl_options(&[
            "-o", "0", "-t", "4", "-v", "7", "-l", "3", "-f", "2", "-r", "-i", "-V",
        ])
        .unwrap();

        assert_eq!(parsed.options.ordering_depth, 0);
        assert_eq!(parsed.options.timeout_seconds, 4);
        assert_eq!(parsed.options.verbose, 7);
        assert_eq!(parsed.options.remlatch.max_level, 3);
        assert_eq!(parsed.options.remlatch.max_cost, 2);
        assert!(!parsed.options.remlatch.local_retiming);
        assert!(!parsed.options.remlatch.remove_boot);
        assert!(parsed.options.stop_if_verify);
        assert_eq!(parsed.options.method, RangeMethod::Product);
    }

    #[test]
    fn prl_options_preserve_c_disabled_p_option_as_error() {
        assert_eq!(
            parse_prl_options(&["-p", "ignored"]).unwrap_err(),
            ComVerifyError::UnsupportedPrlOption('p')
        );
        assert_eq!(
            parse_prl_options(&["-l", "1"]).unwrap_err(),
            ComVerifyError::InvalidOptionValue {
                option: 'l',
                value: "1".to_owned(),
            }
        );
    }

    #[test]
    fn command_plans_model_c_early_exits_and_delegation() {
        assert_eq!(
            plan_verify_fsm(&["other.blif"], &NetworkStats::absent()).unwrap(),
            CommandPlan::NoNetwork
        );
        assert_eq!(
            plan_extract_seq_dc(&[], &NetworkStats::present(0, 1)).unwrap(),
            CommandPlan::NothingToDo
        );
        assert_eq!(
            plan_env_seq_dc(&[], &NetworkStats::present(2, 1)).unwrap(),
            CommandPlan::Delegate {
                command_line: "extract_seq_dc"
            }
        );
        assert_eq!(
            plan_remove_latches(&[], &NetworkStats::present(2, 0)).unwrap(),
            CommandPlan::NothingToDo
        );
    }

    #[test]
    fn command_plans_require_c_operand_counts() {
        assert_eq!(
            plan_verify_fsm(&[], &NetworkStats::present(1, 1)).unwrap_err(),
            ComVerifyError::WrongOperandCount {
                command: "verify_fsm",
                expected: "one operand",
                actual: 0,
            }
        );
        assert_eq!(
            plan_env_verify_fsm(&["check.blif"], &NetworkStats::present(1, 1)).unwrap_err(),
            ComVerifyError::WrongOperandCount {
                command: "env_verify_fsm",
                expected: "two operands",
                actual: 1,
            }
        );
        assert!(matches!(
            plan_latch_output(&[]),
            Err(ComVerifyError::WrongOperandCount {
                command: "latch_output",
                ..
            })
        ));
        assert!(matches!(
            plan_remove_dependencies(&["input"]),
            Err(ComVerifyError::WrongOperandCount {
                command: "remove_dep",
                ..
            })
        ));
    }

    #[test]
    fn stop_if_verify_status_mapping_matches_c_return_expression() {
        assert!(map_stop_if_verify_status(true, false));
        assert!(!map_stop_if_verify_status(false, false));
        assert!(!map_stop_if_verify_status(true, true));
        assert!(map_stop_if_verify_status(false, true));
    }
    #[test]
    fn usage_text_keeps_c_method_listing_and_prl_banner_shape() {
        assert!(range_usage("verify_fsm", "network2.blif").contains("consistency bull product"));
        assert!(prl_usage("remove_latches", "").contains("[-l m] [-f n]"));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("com_verify.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
