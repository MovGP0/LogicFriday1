//! Native Rust command model for `LogicSynthesis/sis/retime/com_retime.c`.
//!
//! The C file owns SIS command registration for `retime`, `_print_stats`, and
//! `_check_tr`, command-line option defaults, and the top-level retiming
//! workflow. The actual network mutation still depends on native ports for SIS
//! command, clock, delay, library, latch, network, node, STG, and retime graph
//! APIs. This module ports the deterministic command surface and workflow
//! scaffold, and reports those network-bound calls as explicit missing
//! dependency errors instead of exposing legacy C ABI entry points.

use std::error::Error;
use std::fmt;

pub const RETIME_DEFAULT_REG_AREA: f64 = 1.0;
pub const RETIME_DEFAULT_REG_DELAY: f64 = 0.0;
pub const RETIME_DEFAULT_TOLERANCE: f64 = 0.1;

pub const RETIME_USAGE: &str = concat!(
    "retime [-fimn] [-c #.#] [-t #.#] [-d #.#] [-a #.#] [-v #]\n",
    "-i\t\t: Do not recompute the initial states\n",
    "-n\t\t: Use delay/area of unmapped network\n",
    "-m\t\t: Minimize registers subject to given cycle time (-c option)",
    "\t\t: May be very slow for large circuits\n",
    "-f\t\t: Use MILP formulation (default is Saxe's relaxation alg)\n",
    "-c\t#.#\t: Set the desired clock period\n",
    "-t\t#.#\t: Set tolerance for binary search (default = 0.1)\n",
    "-d\t#.#\t: Set the delay thru register (with -n option only)\n",
    "-a\t#.#\t: Set the area of a register (with -n option only)\n",
    "-v\t#\t: Set the verbosity level (0-100)\n",
);

pub const PRINT_STATS_UNKNOWN_OPTION: &str = "Unknown option";
pub const CHECK_TR_UNKNOWN_OPTION: &str = "Unknown option";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Mapped,
    UnitFanout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimeCommandKind {
    Retime,
    CheckTranslation,
    PrintStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: RetimeCommandKind,
    pub changes_network: bool,
}

pub const RETIME_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "retime",
        kind: RetimeCommandKind::Retime,
        changes_network: true,
    },
    CommandRegistration {
        name: "_check_tr",
        kind: RetimeCommandKind::CheckTranslation,
        changes_network: true,
    },
    CommandRegistration {
        name: "_print_stats",
        kind: RetimeCommandKind::PrintStats,
        changes_network: false,
    },
];

#[derive(Clone, Debug, PartialEq)]
pub struct RetimeOptions {
    pub milp_formulation: bool,
    pub minimize_registers: bool,
    pub should_initialize: bool,
    pub delay_model: DelayModel,
    pub use_mapped_network: bool,
    pub desired_cycle_time: Option<f64>,
    pub retime_tolerance: f64,
    pub register_delay: f64,
    pub register_area: f64,
    pub debug_level: i32,
}

impl Default for RetimeOptions {
    fn default() -> Self {
        Self {
            milp_formulation: false,
            minimize_registers: false,
            should_initialize: true,
            delay_model: DelayModel::Mapped,
            use_mapped_network: true,
            desired_cycle_time: None,
            retime_tolerance: RETIME_DEFAULT_TOLERANCE,
            register_delay: RETIME_DEFAULT_REG_DELAY,
            register_area: RETIME_DEFAULT_REG_AREA,
            debug_level: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintStatsOptions {
    pub register_delay: f64,
    pub register_area: f64,
}

impl Default for PrintStatsOptions {
    fn default() -> Self {
        Self {
            register_delay: RETIME_DEFAULT_REG_DELAY,
            register_area: RETIME_DEFAULT_REG_AREA,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckTranslationOptions {
    pub use_mapped_network: bool,
    pub debug_level: i32,
}

impl Default for CheckTranslationOptions {
    fn default() -> Self {
        Self {
            use_mapped_network: true,
            debug_level: 100,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetimeCommand {
    Retime(RetimePlan),
    CheckTranslation(CheckTranslationPlan),
    PrintStats(PrintStatsPlan),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetimePlan {
    pub options: RetimeOptions,
    pub actions: Vec<RetimeAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintStatsPlan {
    pub options: PrintStatsOptions,
    pub actions: Vec<PrintStatsAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckTranslationPlan {
    pub options: CheckTranslationOptions,
    pub actions: Vec<CheckTranslationAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetimeAction {
    ReturnIfEmptyOrCombinational,
    RequireMappedNetworkAndLibrary,
    SweepUnmappedNetwork,
    TraceDelay,
    ReadClockData,
    UseSpecifiedOrClockCycleTime,
    RejectCycleTimeBelowRegisterDelay,
    BuildRetimeGraph,
    OptionallyCheckAndDumpGraph,
    UseLowerBoundWhenNoCycleSpecified,
    ComputeInitialCycleDelay,
    ReturnIfSpecificationAlreadyMet,
    OptimizeGraph,
    ReturnIfNoLatchesMoved,
    ReconstructNetworkOnSuccess,
    SetWorkingCycleTime,
    CleanupTemporaryNodesAndGraph,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrintStatsAction {
    PrintStgDataIfEmptyNetwork,
    FallBackToUnitFanoutWhenUnmappedOrNoLibrary,
    BuildRetimeGraph,
    CleanupTemporaryNodes,
    ReadClockData,
    ReportCycleRegisterAndAreaStats,
    PrintStgData,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckTranslationAction {
    ReturnIfEmptyNetwork,
    PrintLatchTrueInputs,
    PrintInitialNetwork,
    FallBackToUnitFanoutWhenUnmapped,
    BuildRetimeGraph,
    DumpAndCheckGraph,
    ReadClockData,
    ReconstructNetwork,
    RequireMappedNetworkAfterTranslation,
    DuplicateClockAndDelayDefaults,
    ReplaceNetworkAndCleanup,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandParseError {
    MissingOptionValue(char),
    UnsupportedOption(String),
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
        }
    }
}

impl Error for CommandParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetimeCommandError {
    MissingSisPorts { command: RetimeCommandKind },
}

impl fmt::Display for RetimeCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts { command } => {
                write!(f, "{command:?} requires native Rust prerequisite ports")
            }
        }
    }
}

impl Error for RetimeCommandError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetimeNetworkSnapshot {
    pub is_empty: bool,
    pub latch_count: usize,
    pub is_mapped: bool,
    pub has_library: bool,
    pub specified_cycle_time: Option<f64>,
    pub initial_cycle_delay: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetimePreflightDecision {
    NoOpEmptyOrCombinational,
    NeedUnmappedSweep,
    MissingMappedNetwork,
    MissingLibrary,
    DelayThroughRegisterExceedsDesiredCycle,
    SpecificationAlreadyMet,
    ReadyToBuildGraph,
}

pub fn retime_command_registrations() -> &'static [CommandRegistration] {
    RETIME_COMMANDS
}

pub fn parse_command<I, S>(command_name: &str, args: I) -> Result<RetimeCommand, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name {
        "retime" => parse_retime_args(args).map(RetimeCommand::Retime),
        "_check_tr" => parse_check_translation_args(args).map(RetimeCommand::CheckTranslation),
        "_print_stats" => parse_print_stats_args(args).map(RetimeCommand::PrintStats),
        _ => Err(CommandParseError::UnsupportedOption(
            command_name.to_owned(),
        )),
    }
}

pub fn parse_retime_args<I, S>(args: I) -> Result<RetimePlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = RetimeOptions::default();
    parse_options(args, "fimnv:c:d:a:t:", |option, value| match option {
        'n' => {
            options.use_mapped_network = false;
            options.delay_model = DelayModel::UnitFanout;
            Ok(())
        }
        'i' => {
            options.should_initialize = false;
            Ok(())
        }
        'm' => {
            options.minimize_registers = true;
            Ok(())
        }
        'f' => {
            options.milp_formulation = true;
            Ok(())
        }
        't' => {
            options.retime_tolerance = c_atof(&value);
            Ok(())
        }
        'd' => {
            options.register_delay = c_atof(&value);
            Ok(())
        }
        'a' => {
            options.register_area = c_atof(&value);
            Ok(())
        }
        'c' => {
            options.desired_cycle_time = Some(c_atof(&value));
            Ok(())
        }
        'v' => {
            options.debug_level = c_atoi(&value);
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;

    Ok(plan_retime(options))
}

pub fn parse_print_stats_args<I, S>(args: I) -> Result<PrintStatsPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = PrintStatsOptions::default();
    parse_options(args, "d:a:", |option, value| match option {
        'a' => {
            options.register_area = c_atof(&value);
            Ok(())
        }
        'd' => {
            options.register_delay = c_atof(&value);
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;

    Ok(PrintStatsPlan {
        options,
        actions: vec![
            PrintStatsAction::PrintStgDataIfEmptyNetwork,
            PrintStatsAction::FallBackToUnitFanoutWhenUnmappedOrNoLibrary,
            PrintStatsAction::BuildRetimeGraph,
            PrintStatsAction::CleanupTemporaryNodes,
            PrintStatsAction::ReadClockData,
            PrintStatsAction::ReportCycleRegisterAndAreaStats,
            PrintStatsAction::PrintStgData,
        ],
    })
}

pub fn parse_check_translation_args<I, S>(
    args: I,
) -> Result<CheckTranslationPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = CheckTranslationOptions::default();
    parse_options(args, "n", |option, _value| match option {
        'n' => {
            options.use_mapped_network = false;
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;

    Ok(CheckTranslationPlan {
        options,
        actions: vec![
            CheckTranslationAction::ReturnIfEmptyNetwork,
            CheckTranslationAction::PrintLatchTrueInputs,
            CheckTranslationAction::PrintInitialNetwork,
            CheckTranslationAction::FallBackToUnitFanoutWhenUnmapped,
            CheckTranslationAction::BuildRetimeGraph,
            CheckTranslationAction::DumpAndCheckGraph,
            CheckTranslationAction::ReadClockData,
            CheckTranslationAction::ReconstructNetwork,
            CheckTranslationAction::RequireMappedNetworkAfterTranslation,
            CheckTranslationAction::DuplicateClockAndDelayDefaults,
            CheckTranslationAction::ReplaceNetworkAndCleanup,
        ],
    })
}

pub fn plan_retime(options: RetimeOptions) -> RetimePlan {
    RetimePlan {
        options,
        actions: vec![
            RetimeAction::ReturnIfEmptyOrCombinational,
            RetimeAction::RequireMappedNetworkAndLibrary,
            RetimeAction::SweepUnmappedNetwork,
            RetimeAction::TraceDelay,
            RetimeAction::ReadClockData,
            RetimeAction::UseSpecifiedOrClockCycleTime,
            RetimeAction::RejectCycleTimeBelowRegisterDelay,
            RetimeAction::BuildRetimeGraph,
            RetimeAction::OptionallyCheckAndDumpGraph,
            RetimeAction::UseLowerBoundWhenNoCycleSpecified,
            RetimeAction::ComputeInitialCycleDelay,
            RetimeAction::ReturnIfSpecificationAlreadyMet,
            RetimeAction::OptimizeGraph,
            RetimeAction::ReturnIfNoLatchesMoved,
            RetimeAction::ReconstructNetworkOnSuccess,
            RetimeAction::SetWorkingCycleTime,
            RetimeAction::CleanupTemporaryNodesAndGraph,
        ],
    }
}

pub fn preflight_retime(
    options: &RetimeOptions,
    network: RetimeNetworkSnapshot,
) -> RetimePreflightDecision {
    if network.is_empty || network.latch_count == 0 {
        return RetimePreflightDecision::NoOpEmptyOrCombinational;
    }
    if options.use_mapped_network && !network.is_mapped {
        return RetimePreflightDecision::MissingMappedNetwork;
    }
    if options.use_mapped_network && !network.has_library {
        return RetimePreflightDecision::MissingLibrary;
    }
    if !options.use_mapped_network {
        return RetimePreflightDecision::NeedUnmappedSweep;
    }

    let desired = options
        .desired_cycle_time
        .or(network.specified_cycle_time.filter(|cycle| *cycle > 0.0));
    if desired.is_some_and(|cycle| cycle > 0.0 && cycle < options.register_delay) {
        return RetimePreflightDecision::DelayThroughRegisterExceedsDesiredCycle;
    }
    if !options.minimize_registers
        && desired.is_some_and(|cycle| network.initial_cycle_delay <= cycle)
    {
        return RetimePreflightDecision::SpecificationAlreadyMet;
    }

    RetimePreflightDecision::ReadyToBuildGraph
}

pub fn execute_command<Network>(
    _network: &mut Network,
    command: &RetimeCommand,
) -> Result<(), RetimeCommandError> {
    let kind = match command {
        RetimeCommand::Retime(_) => RetimeCommandKind::Retime,
        RetimeCommand::CheckTranslation(_) => RetimeCommandKind::CheckTranslation,
        RetimeCommand::PrintStats(_) => RetimeCommandKind::PrintStats,
    };

    Err(RetimeCommandError::MissingSisPorts { command: kind })
}

fn parse_options<F>(
    args: impl IntoIterator<Item = impl AsRef<str>>,
    spec: &str,
    mut apply: F,
) -> Result<(), CommandParseError>
where
    F: FnMut(char, String) -> Result<(), CommandParseError>,
{
    let mut iter = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .peekable();

    while let Some(arg) = iter.next() {
        if !arg.starts_with('-') || arg == "-" {
            return Err(CommandParseError::UnsupportedOption(arg));
        }
        if arg == "--" {
            if let Some(operand) = iter.next() {
                return Err(CommandParseError::UnsupportedOption(operand));
            }
            return Ok(());
        }

        let mut chars = arg[1..].char_indices().peekable();
        while let Some((offset, option)) = chars.next() {
            let needs_value = option_needs_value(spec, option)
                .ok_or_else(|| CommandParseError::UnsupportedOption(format!("-{option}")))?;
            if needs_value {
                let value_start = offset + option.len_utf8();
                let value = if value_start < arg[1..].len() {
                    arg[1 + value_start..].to_owned()
                } else {
                    iter.next()
                        .ok_or(CommandParseError::MissingOptionValue(option))?
                };
                apply(option, value)?;
                break;
            }
            apply(option, String::new())?;
        }
    }

    Ok(())
}

fn option_needs_value(spec: &str, option: char) -> Option<bool> {
    let mut chars = spec.chars().peekable();
    while let Some(candidate) = chars.next() {
        if candidate == ':' {
            continue;
        }
        let has_value = chars.peek() == Some(&':');
        if candidate == option {
            return Some(has_value);
        }
    }
    None
}

fn c_atoi(value: &str) -> i32 {
    let trimmed = value.trim_start();
    let mut chars = trimmed.chars().peekable();
    let mut sign = 1;

    match chars.peek().copied() {
        Some('-') => {
            sign = -1;
            chars.next();
        }
        Some('+') => {
            chars.next();
        }
        _ => {}
    }

    let mut result = 0i32;
    let mut saw_digit = false;
    for ch in chars {
        let Some(digit) = ch.to_digit(10) else {
            break;
        };
        saw_digit = true;
        result = result.saturating_mul(10).saturating_add(digit as i32);
    }

    if saw_digit {
        result.saturating_mul(sign)
    } else {
        0
    }
}

fn c_atof(value: &str) -> f64 {
    parse_c_float_prefix(value).unwrap_or(0.0)
}

fn parse_c_float_prefix(value: &str) -> Option<f64> {
    let trimmed = value.trim_start();
    let mut end = 0;
    let mut saw_digit = false;
    let mut chars = trimmed.char_indices().peekable();

    if matches!(chars.peek().map(|(_, ch)| *ch), Some('+') | Some('-')) {
        let (index, ch) = chars.next().unwrap();
        end = index + ch.len_utf8();
    }

    while let Some((index, ch)) = chars.peek().copied() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            end = index + ch.len_utf8();
            chars.next();
        } else {
            break;
        }
    }

    if matches!(chars.peek().map(|(_, ch)| *ch), Some('.')) {
        let (index, ch) = chars.next().unwrap();
        end = index + ch.len_utf8();
        while let Some((index, ch)) = chars.peek().copied() {
            if ch.is_ascii_digit() {
                saw_digit = true;
                end = index + ch.len_utf8();
                chars.next();
            } else {
                break;
            }
        }
    }

    if !saw_digit {
        return None;
    }

    if matches!(chars.peek().map(|(_, ch)| *ch), Some('e') | Some('E')) {
        let exponent_start = end;
        let (index, ch) = chars.next().unwrap();
        let mut exponent_end = index + ch.len_utf8();
        if matches!(chars.peek().map(|(_, ch)| *ch), Some('+') | Some('-')) {
            let (index, ch) = chars.next().unwrap();
            exponent_end = index + ch.len_utf8();
        }

        let mut saw_exponent_digit = false;
        while let Some((index, ch)) = chars.peek().copied() {
            if ch.is_ascii_digit() {
                saw_exponent_digit = true;
                exponent_end = index + ch.len_utf8();
                chars.next();
            } else {
                break;
            }
        }

        end = if saw_exponent_digit {
            exponent_end
        } else {
            exponent_start
        };
    }

    trimmed[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrations_match_init_retime() {
        assert_eq!(
            retime_command_registrations(),
            &[
                CommandRegistration {
                    name: "retime",
                    kind: RetimeCommandKind::Retime,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "_check_tr",
                    kind: RetimeCommandKind::CheckTranslation,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "_print_stats",
                    kind: RetimeCommandKind::PrintStats,
                    changes_network: false,
                },
            ]
        );
    }

    #[test]
    fn parses_retime_defaults_and_c_option_effects() {
        let plan = parse_retime_args([
            "-fimn", "-v", "21x", "-c3.5", "-t", ".25", "-d", "0.4ms", "-a2",
        ])
        .unwrap();

        assert!(plan.options.milp_formulation);
        assert!(plan.options.minimize_registers);
        assert!(!plan.options.should_initialize);
        assert_eq!(plan.options.delay_model, DelayModel::UnitFanout);
        assert!(!plan.options.use_mapped_network);
        assert_eq!(plan.options.debug_level, 21);
        assert_eq!(plan.options.desired_cycle_time, Some(3.5));
        assert_eq!(plan.options.retime_tolerance, 0.25);
        assert_eq!(plan.options.register_delay, 0.4);
        assert_eq!(plan.options.register_area, 2.0);
        assert_eq!(
            plan.actions.first(),
            Some(&RetimeAction::ReturnIfEmptyOrCombinational)
        );
        assert!(plan.actions.contains(&RetimeAction::OptimizeGraph));
    }

    #[test]
    fn parses_print_stats_and_check_translation_options() {
        let stats = parse_print_stats_args(["-d1.25", "-a", "3.0"]).unwrap();
        assert_eq!(stats.options.register_delay, 1.25);
        assert_eq!(stats.options.register_area, 3.0);
        assert!(stats.actions.contains(&PrintStatsAction::PrintStgData));

        let check = parse_check_translation_args(["-n"]).unwrap();
        assert!(!check.options.use_mapped_network);
        assert_eq!(check.options.debug_level, 100);
        assert!(
            check
                .actions
                .contains(&CheckTranslationAction::DumpAndCheckGraph)
        );
    }

    #[test]
    fn rejects_unknown_options_and_missing_values() {
        assert_eq!(
            parse_retime_args(["-c"]).unwrap_err(),
            CommandParseError::MissingOptionValue('c')
        );
        assert_eq!(
            parse_print_stats_args(["-x"]).unwrap_err(),
            CommandParseError::UnsupportedOption("-x".to_owned())
        );
        assert_eq!(
            parse_check_translation_args(["operand"]).unwrap_err(),
            CommandParseError::UnsupportedOption("operand".to_owned())
        );
    }

    #[test]
    fn preflight_matches_c_early_return_conditions() {
        let options = RetimeOptions::default();
        let mut network = RetimeNetworkSnapshot {
            is_empty: false,
            latch_count: 1,
            is_mapped: true,
            has_library: true,
            specified_cycle_time: Some(5.0),
            initial_cycle_delay: 4.0,
        };

        network.latch_count = 0;
        assert_eq!(
            preflight_retime(&options, network),
            RetimePreflightDecision::NoOpEmptyOrCombinational
        );

        network.latch_count = 1;
        network.is_mapped = false;
        assert_eq!(
            preflight_retime(&options, network),
            RetimePreflightDecision::MissingMappedNetwork
        );

        let unmapped = RetimeOptions {
            use_mapped_network: false,
            delay_model: DelayModel::UnitFanout,
            ..RetimeOptions::default()
        };
        assert_eq!(
            preflight_retime(&unmapped, network),
            RetimePreflightDecision::NeedUnmappedSweep
        );

        network.is_mapped = true;
        network.has_library = false;
        assert_eq!(
            preflight_retime(&options, network),
            RetimePreflightDecision::MissingLibrary
        );
    }

    #[test]
    fn preflight_checks_cycle_delay_and_specification_met() {
        let mut options = RetimeOptions {
            desired_cycle_time: Some(0.5),
            register_delay: 1.0,
            ..RetimeOptions::default()
        };
        let network = RetimeNetworkSnapshot {
            is_empty: false,
            latch_count: 2,
            is_mapped: true,
            has_library: true,
            specified_cycle_time: None,
            initial_cycle_delay: 2.0,
        };
        assert_eq!(
            preflight_retime(&options, network),
            RetimePreflightDecision::DelayThroughRegisterExceedsDesiredCycle
        );

        options.desired_cycle_time = Some(3.0);
        assert_eq!(
            preflight_retime(&options, network),
            RetimePreflightDecision::SpecificationAlreadyMet
        );

        options.minimize_registers = true;
        assert_eq!(
            preflight_retime(&options, network),
            RetimePreflightDecision::ReadyToBuildGraph
        );
    }
    #[test]
    fn command_dispatch_selects_all_three_c_commands() {
        assert!(matches!(
            parse_command("retime", ["-m"]).unwrap(),
            RetimeCommand::Retime(_)
        ));
        assert!(matches!(
            parse_command("_check_tr", ["-n"]).unwrap(),
            RetimeCommand::CheckTranslation(_)
        ));
        assert!(matches!(
            parse_command("_print_stats", ["-a2"]).unwrap(),
            RetimeCommand::PrintStats(_)
        ));
    }

    #[test]
    fn c_number_helpers_keep_atof_and_atoi_prefix_behavior() {
        assert_eq!(c_atoi("  -42z"), -42);
        assert_eq!(c_atoi("word"), 0);
        assert_eq!(c_atof("  +1.25e2ms"), 125.0);
        assert_eq!(c_atof("bad"), 0.0);
        assert_eq!(c_atof("1e+"), 1.0);
    }
}
