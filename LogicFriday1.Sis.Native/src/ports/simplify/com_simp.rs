//! Native Rust command scaffold for `LogicSynthesis/sis/simplify/com_simp.c`.
//!
//! The C file registers the SIS simplify commands and translates command-line
//! options into simplify-node passes. Direct mutation of `network_t`/`node_t`,
//! BDD construction, CSPF cleanup, and timeout signal handling are delegated to
//! a native Rust backend. This module ports the deterministic command
//! registration, option parsing, command planning, `get_cone_levels`, and
//! `find_node_level` behavior.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const BDD_NODE_LIMIT: usize = 480_000;
pub const BDD_NODE_LOW: usize = 50_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimplifyCommandKind {
    Simplify,
    InternalSimp,
    FullSimplify,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: SimplifyCommandKind,
    pub changes_network: bool,
}

pub const SIMPLIFY_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "simplify",
        kind: SimplifyCommandKind::Simplify,
        changes_network: true,
    },
    CommandRegistration {
        name: "_simp",
        kind: SimplifyCommandKind::InternalSimp,
        changes_network: true,
    },
    CommandRegistration {
        name: "full_simplify",
        kind: SimplifyCommandKind::FullSimplify,
        changes_network: false,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimMethod {
    SimpComp,
    Espresso,
    Exact,
    ExactLits,
    DcSimp,
    NoComp,
    SNoComp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimAccept {
    FactoredLiterals,
    SopLiterals,
    Cubes,
    Always,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimDcType {
    None,
    Fanin,
    Fanout,
    Inout,
    All,
    SubFanin,
    Level,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimFilter {
    None,
    Exact,
    DisjointSupport,
    Size,
    FirstDistance,
    SecondDistance,
    Level,
    All,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplifyOptions {
    pub method: SimMethod,
    pub accept: SimAccept,
    pub dctype: SimDcType,
    pub filter: SimFilter,
    pub trace: bool,
    pub debug: bool,
    pub fanin_level: i32,
    pub fanin_fanout_level: i32,
    pub node_names: Vec<String>,
}

impl SimplifyOptions {
    pub fn simplify_defaults() -> Self {
        Self {
            method: SimMethod::SNoComp,
            accept: SimAccept::FactoredLiterals,
            dctype: SimDcType::SubFanin,
            filter: SimFilter::Exact,
            trace: false,
            debug: false,
            fanin_level: 1,
            fanin_fanout_level: 0,
            node_names: Vec::new(),
        }
    }

    pub fn internal_simp_defaults() -> Self {
        Self {
            dctype: SimDcType::Fanin,
            ..Self::simplify_defaults()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FullSimplifyOptions {
    pub method: SimMethod,
    pub accept: SimAccept,
    pub dctype: SimDcType,
    pub filter: SimFilter,
    pub ordering: i32,
    pub timeout_seconds: i32,
    pub verbose: bool,
    pub trace: bool,
    pub fanin_level: i32,
    pub fanin_fanout_level: i32,
    pub ignored_operands: Vec<String>,
}

impl Default for FullSimplifyOptions {
    fn default() -> Self {
        Self {
            method: SimMethod::SNoComp,
            accept: SimAccept::FactoredLiterals,
            dctype: SimDcType::All,
            filter: SimFilter::Exact,
            ordering: 0,
            timeout_seconds: 0,
            verbose: false,
            trace: false,
            fanin_level: 1,
            fanin_fanout_level: 0,
            ignored_operands: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimplifyCommand {
    Simplify(SimplifyCommandPlan),
    InternalSimp(SimplifyCommandPlan),
    FullSimplify(FullSimplifyCommandPlan),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimplifyAction {
    ResetGlobalFlags {
        trace: bool,
        debug: bool,
        fanin_level: i32,
        fanin_fanout_level: i32,
    },
    ComputeNodeLevels,
    SelectNodes(Vec<String>),
    OrderNodesForSimplify,
    SimplifyInternalNodes {
        method: SimMethod,
        dctype: SimDcType,
        accept: SimAccept,
        filter: SimFilter,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplifyCommandPlan {
    pub command: SimplifyCommandKind,
    pub options: SimplifyOptions,
    pub actions: Vec<SimplifyAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FullSimplifyAction {
    ResetGlobalFlags {
        verbose: bool,
        trace: bool,
        fanin_level: i32,
        fanin_fanout_level: i32,
    },
    InstallTimeoutHandler {
        seconds: i32,
    },
    ComputeLevelFilter,
    CollectPrimaryOutputs,
    CopyAndAttachDcNetwork,
    OrderPrimaryInputsByDepth,
    OrderPrimaryInputsByLevel,
    StartBddManager,
    ConvertNodesToBdds {
        low_node_threshold: usize,
        hard_node_limit: usize,
    },
    SimplifyWithoutOdc,
    SimplifyWithOdc,
    CleanupBddCspfAndDcNetwork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FullSimplifyCommandPlan {
    pub options: FullSimplifyOptions,
    pub actions: Vec<FullSimplifyAction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimplifyNodeKind {
    PrimaryInput,
    Internal,
    PrimaryOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplifyGraphNode {
    pub name: String,
    pub kind: SimplifyNodeKind,
    pub fanins: Vec<String>,
}

impl SimplifyGraphNode {
    pub fn new(name: impl Into<String>, kind: SimplifyNodeKind, fanins: Vec<String>) -> Self {
        Self {
            name: name.into(),
            kind,
            fanins,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandParseError {
    MissingOptionValue(char),
    UnsupportedOption(String),
    InvalidArgument { option: char, value: String },
    InvalidConeLevels(String),
    UnknownAccept(String),
    UnknownDontCareType(String),
    UnknownMethod(String),
    UnknownFilter(String),
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::InvalidArgument { option, value } => {
                write!(f, "invalid argument to -{option}: {value}")
            }
            Self::InvalidConeLevels(value) => write!(f, "invalid cone level argument {value}"),
            Self::UnknownAccept(value) => write!(f, "unknown simplify accept mode {value}"),
            Self::UnknownDontCareType(value) => {
                write!(f, "unknown simplify don't-care type {value}")
            }
            Self::UnknownMethod(value) => write!(f, "unknown simplify method {value}"),
            Self::UnknownFilter(value) => write!(f, "unknown simplify filter {value}"),
        }
    }
}

impl Error for CommandParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimplifyCommandError {
    Backend(String),
}

impl fmt::Display for SimplifyCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(message) => write!(f, "{message}"),
        }
    }
}

impl Error for SimplifyCommandError {}

pub fn simplify_command_registrations() -> &'static [CommandRegistration] {
    SIMPLIFY_COMMANDS
}

pub fn simplify_usage(command: &str) -> String {
    format!("usage: {command} [-d][-i <num>[:<num>]] [-m method] [-f filter] [node-list]\n")
}

pub fn full_simplify_usage(command: &str) -> String {
    format!("usage: {command} [-d][-o ordering] [-m method] [-l] [-t time] [-v]\n")
}

pub fn parse_simplify_args<I, S>(args: I) -> Result<SimplifyCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = SimplifyOptions::simplify_defaults();
    options.node_names = parse_options(args, "m:i:f:tdl", |option, value| match option {
        'l' => {
            options.dctype = SimDcType::Level;
            Ok(())
        }
        'd' => {
            options.dctype = SimDcType::None;
            Ok(())
        }
        'm' => {
            options.method = parse_public_method(&value)?;
            Ok(())
        }
        'f' => {
            options.filter = parse_public_filter(&value)?;
            Ok(())
        }
        'i' => {
            let (fanin, fanout) = get_cone_levels(&value)
                .ok_or_else(|| CommandParseError::InvalidConeLevels(value.clone()))?;
            options.dctype = SimDcType::Fanin;
            options.fanin_level = fanin;
            options.fanin_fanout_level = fanout;
            Ok(())
        }
        't' => {
            options.trace = true;
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;

    Ok(plan_simplify_command(
        SimplifyCommandKind::Simplify,
        options,
        true,
    ))
}

pub fn parse_internal_simp_args<I, S>(args: I) -> Result<SimplifyCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = SimplifyOptions::internal_simp_defaults();
    options.node_names = parse_options(args, "a:d:m:i:o:f:Dt", |option, value| match option {
        'a' => {
            options.accept = parse_accept(&value)?;
            Ok(())
        }
        'd' => {
            options.dctype = parse_internal_dctype(&value)?;
            Ok(())
        }
        'm' => {
            options.method = parse_internal_method(&value)?;
            Ok(())
        }
        'f' => {
            options.filter = parse_internal_filter(&value)?;
            Ok(())
        }
        'i' => {
            options.fanin_level = c_atoi(&value);
            merge_fanin_or_fanout_level_option(&mut options.dctype);
            Ok(())
        }
        'o' => {
            options.fanin_fanout_level = c_atoi(&value);
            merge_fanin_or_fanout_level_option(&mut options.dctype);
            Ok(())
        }
        't' => {
            options.trace = true;
            Ok(())
        }
        'D' => {
            options.debug = true;
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;

    Ok(plan_simplify_command(
        SimplifyCommandKind::InternalSimp,
        options,
        false,
    ))
}

pub fn parse_full_simplify_args<I, S>(args: I) -> Result<FullSimplifyCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = FullSimplifyOptions::default();
    options.ignored_operands = parse_options(args, "f:o:m:t:vdl", |option, value| match option {
        'o' => {
            options.ordering = c_atoi(&value);
            Ok(())
        }
        't' => {
            options.timeout_seconds = c_atoi(&value);
            Ok(())
        }
        'l' => {
            options.filter = SimFilter::Level;
            Ok(())
        }
        'f' => {
            options.filter = parse_full_filter(&value)?;
            Ok(())
        }
        'd' => {
            options.dctype = SimDcType::Fanin;
            Ok(())
        }
        'm' => {
            options.method = parse_public_method(&value)?;
            Ok(())
        }
        'v' => {
            options.verbose = true;
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;

    Ok(plan_full_simplify_command(options))
}

pub fn parse_simplify_command<I, S>(
    command_name: &str,
    args: I,
) -> Result<SimplifyCommand, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name {
        "simplify" => parse_simplify_args(args).map(SimplifyCommand::Simplify),
        "_simp" => parse_internal_simp_args(args).map(SimplifyCommand::InternalSimp),
        "full_simplify" => parse_full_simplify_args(args).map(SimplifyCommand::FullSimplify),
        _ => Err(CommandParseError::UnsupportedOption(
            command_name.to_owned(),
        )),
    }
}

pub fn execute_simplify_command<Network>(
    network: &mut Network,
    command: &SimplifyCommand,
) -> Result<(), SimplifyCommandError>
where
    Network: SimplifyCommandBackend,
{
    execute_simplify_command_with_backend(network, command)
}

pub trait SimplifyCommandBackend {
    fn simplify(&mut self, _plan: &SimplifyCommandPlan) -> Result<(), SimplifyCommandError> {
        Ok(())
    }

    fn internal_simp(&mut self, _plan: &SimplifyCommandPlan) -> Result<(), SimplifyCommandError> {
        Ok(())
    }

    fn full_simplify(
        &mut self,
        _plan: &FullSimplifyCommandPlan,
    ) -> Result<(), SimplifyCommandError> {
        Ok(())
    }
}

impl SimplifyCommandBackend for () {}

pub fn execute_simplify_command_with_backend<Network>(
    network: &mut Network,
    command: &SimplifyCommand,
) -> Result<(), SimplifyCommandError>
where
    Network: SimplifyCommandBackend,
{
    match command {
        SimplifyCommand::Simplify(plan) => network.simplify(plan),
        SimplifyCommand::InternalSimp(plan) => network.internal_simp(plan),
        SimplifyCommand::FullSimplify(plan) => network.full_simplify(plan),
    }
}

pub fn get_cone_levels(value: &str) -> Option<(i32, i32)> {
    if value.is_empty() || value.contains(' ') {
        return None;
    }

    let mut fanin = 1;
    let mut fanin_fanout = 0;
    let (left, right) = match value.split_once(':') {
        Some(parts) => parts,
        None => (value, ""),
    };

    if !value.starts_with(':') {
        fanin = c_atoi(left);
    }
    if value.contains(':') {
        fanin_fanout = c_atoi(right);
    }

    Some((fanin, fanin_fanout))
}

pub fn find_node_levels(nodes_in_dfs_order: &[SimplifyGraphNode]) -> HashMap<String, i32> {
    let mut levels = HashMap::new();

    for node in nodes_in_dfs_order {
        if node.kind == SimplifyNodeKind::PrimaryInput {
            continue;
        }

        let level = node
            .fanins
            .iter()
            .filter_map(|fanin| levels.get(fanin))
            .copied()
            .max()
            .unwrap_or(0)
            + 1;
        levels.insert(node.name.clone(), level);
    }

    levels
}

fn plan_simplify_command(
    command: SimplifyCommandKind,
    options: SimplifyOptions,
    order_nodes: bool,
) -> SimplifyCommandPlan {
    let mut actions = vec![SimplifyAction::ResetGlobalFlags {
        trace: options.trace,
        debug: options.debug,
        fanin_level: options.fanin_level,
        fanin_fanout_level: options.fanin_fanout_level,
    }];
    if options.dctype == SimDcType::Level {
        actions.push(SimplifyAction::ComputeNodeLevels);
    }
    actions.push(SimplifyAction::SelectNodes(options.node_names.clone()));
    if order_nodes {
        actions.push(SimplifyAction::OrderNodesForSimplify);
    }
    actions.push(SimplifyAction::SimplifyInternalNodes {
        method: options.method,
        dctype: options.dctype,
        accept: options.accept,
        filter: options.filter,
    });

    SimplifyCommandPlan {
        command,
        options,
        actions,
    }
}

fn plan_full_simplify_command(options: FullSimplifyOptions) -> FullSimplifyCommandPlan {
    let mut actions = vec![FullSimplifyAction::ResetGlobalFlags {
        verbose: options.verbose,
        trace: options.trace,
        fanin_level: options.fanin_level,
        fanin_fanout_level: options.fanin_fanout_level,
    }];
    if options.timeout_seconds > 0 {
        actions.push(FullSimplifyAction::InstallTimeoutHandler {
            seconds: options.timeout_seconds,
        });
    }
    if options.filter == SimFilter::Level {
        actions.push(FullSimplifyAction::ComputeLevelFilter);
    }
    actions.extend([
        FullSimplifyAction::CollectPrimaryOutputs,
        FullSimplifyAction::CopyAndAttachDcNetwork,
    ]);
    if options.ordering == 0 {
        actions.push(FullSimplifyAction::OrderPrimaryInputsByDepth);
    } else {
        actions.push(FullSimplifyAction::OrderPrimaryInputsByLevel);
    }
    actions.extend([
        FullSimplifyAction::StartBddManager,
        FullSimplifyAction::ConvertNodesToBdds {
            low_node_threshold: BDD_NODE_LOW,
            hard_node_limit: BDD_NODE_LIMIT,
        },
    ]);
    if options.dctype == SimDcType::Fanin {
        actions.push(FullSimplifyAction::SimplifyWithoutOdc);
    } else {
        actions.push(FullSimplifyAction::SimplifyWithOdc);
    }
    actions.push(FullSimplifyAction::CleanupBddCspfAndDcNetwork);

    FullSimplifyCommandPlan { options, actions }
}

fn parse_accept(value: &str) -> Result<SimAccept, CommandParseError> {
    match value {
        "fct_lits" => Ok(SimAccept::FactoredLiterals),
        "sop_lits" => Ok(SimAccept::SopLiterals),
        "cubes" => Ok(SimAccept::Cubes),
        "always" => Ok(SimAccept::Always),
        _ => Err(CommandParseError::UnknownAccept(value.to_owned())),
    }
}

fn parse_internal_dctype(value: &str) -> Result<SimDcType, CommandParseError> {
    match value {
        "none" => Ok(SimDcType::None),
        "fanin" => Ok(SimDcType::Fanin),
        "fanout" => Ok(SimDcType::Fanout),
        "inout" => Ok(SimDcType::Inout),
        "support" => Ok(SimDcType::SubFanin),
        "all" => Ok(SimDcType::All),
        _ => Err(CommandParseError::UnknownDontCareType(value.to_owned())),
    }
}

fn parse_public_method(value: &str) -> Result<SimMethod, CommandParseError> {
    match value {
        "nocomp" => Ok(SimMethod::NoComp),
        "snocomp" => Ok(SimMethod::SNoComp),
        "dcsimp" => Ok(SimMethod::DcSimp),
        _ => Err(CommandParseError::UnknownMethod(value.to_owned())),
    }
}

fn parse_internal_method(value: &str) -> Result<SimMethod, CommandParseError> {
    match value {
        "simpcomp" => Ok(SimMethod::SimpComp),
        "espresso" => Ok(SimMethod::Espresso),
        "exact" => Ok(SimMethod::Exact),
        "min_lit" => Ok(SimMethod::ExactLits),
        _ => parse_public_method(value),
    }
}

fn parse_public_filter(value: &str) -> Result<SimFilter, CommandParseError> {
    match value {
        "exact" => Ok(SimFilter::Exact),
        "disj_sup" => Ok(SimFilter::DisjointSupport),
        _ => Err(CommandParseError::UnknownFilter(value.to_owned())),
    }
}

fn parse_internal_filter(value: &str) -> Result<SimFilter, CommandParseError> {
    match value {
        "size" => Ok(SimFilter::Size),
        "fdist" => Ok(SimFilter::FirstDistance),
        "sdist" => Ok(SimFilter::SecondDistance),
        "none" => Ok(SimFilter::None),
        "all" => Ok(SimFilter::All),
        _ => parse_public_filter(value),
    }
}

fn parse_full_filter(value: &str) -> Result<SimFilter, CommandParseError> {
    match value {
        "level" => Ok(SimFilter::Level),
        "all" => Ok(SimFilter::All),
        _ => Err(CommandParseError::UnknownFilter(value.to_owned())),
    }
}

fn merge_fanin_or_fanout_level_option(dctype: &mut SimDcType) {
    *dctype = match dctype {
        SimDcType::Fanout => SimDcType::Inout,
        SimDcType::Inout | SimDcType::All => *dctype,
        _ => SimDcType::Fanin,
    };
}

fn parse_options<F>(
    args: impl IntoIterator<Item = impl AsRef<str>>,
    spec: &str,
    mut apply: F,
) -> Result<Vec<String>, CommandParseError>
where
    F: FnMut(char, String) -> Result<(), CommandParseError>,
{
    let mut iter = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .peekable();
    let mut operands = Vec::new();
    let mut scanning_options = true;

    while let Some(arg) = iter.next() {
        if !scanning_options || !arg.starts_with('-') || arg == "-" {
            operands.push(arg);
            operands.extend(iter);
            break;
        }
        if arg == "--" {
            scanning_options = false;
            continue;
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
            } else {
                apply(option, String::new())?;
            }
        }
    }

    Ok(operands)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrations_match_init_simplify_commands() {
        assert_eq!(
            simplify_command_registrations(),
            &[
                CommandRegistration {
                    name: "simplify",
                    kind: SimplifyCommandKind::Simplify,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "_simp",
                    kind: SimplifyCommandKind::InternalSimp,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "full_simplify",
                    kind: SimplifyCommandKind::FullSimplify,
                    changes_network: false,
                },
            ]
        );
    }

    #[test]
    fn parses_simplify_defaults_and_cone_levels() {
        let plan =
            parse_simplify_args(["-i", "2:3", "-m", "dcsimp", "-fdisj_sup", "-t", "n1"]).unwrap();

        assert_eq!(plan.command, SimplifyCommandKind::Simplify);
        assert_eq!(plan.options.method, SimMethod::DcSimp);
        assert_eq!(plan.options.dctype, SimDcType::Fanin);
        assert_eq!(plan.options.filter, SimFilter::DisjointSupport);
        assert!(plan.options.trace);
        assert_eq!(plan.options.fanin_level, 2);
        assert_eq!(plan.options.fanin_fanout_level, 3);
        assert_eq!(plan.options.node_names, vec!["n1"]);
        assert!(
            plan.actions
                .contains(&SimplifyAction::OrderNodesForSimplify)
        );
    }

    #[test]
    fn parses_internal_simp_accept_dctype_method_filter_and_levels() {
        let plan = parse_internal_simp_args([
            "-a", "sop_lits", "-d", "fanout", "-i2", "-o", "4", "-m", "min_lit", "-f", "sdist",
            "-Dt", "x", "y",
        ])
        .unwrap();

        assert_eq!(plan.command, SimplifyCommandKind::InternalSimp);
        assert_eq!(plan.options.accept, SimAccept::SopLiterals);
        assert_eq!(plan.options.dctype, SimDcType::Inout);
        assert_eq!(plan.options.method, SimMethod::ExactLits);
        assert_eq!(plan.options.filter, SimFilter::SecondDistance);
        assert_eq!(plan.options.fanin_level, 2);
        assert_eq!(plan.options.fanin_fanout_level, 4);
        assert!(plan.options.debug);
        assert!(plan.options.trace);
        assert_eq!(plan.options.node_names, vec!["x", "y"]);
        assert!(
            !plan
                .actions
                .contains(&SimplifyAction::OrderNodesForSimplify)
        );
    }

    #[test]
    fn parses_full_simplify_and_models_major_driver_order() {
        let plan = parse_full_simplify_args(["-o1", "-t", "12", "-l", "-d", "-m", "nocomp", "-v"])
            .unwrap();

        assert_eq!(plan.options.ordering, 1);
        assert_eq!(plan.options.timeout_seconds, 12);
        assert_eq!(plan.options.filter, SimFilter::Level);
        assert_eq!(plan.options.dctype, SimDcType::Fanin);
        assert_eq!(plan.options.method, SimMethod::NoComp);
        assert!(plan.options.verbose);
        assert_eq!(
            plan.actions,
            vec![
                FullSimplifyAction::ResetGlobalFlags {
                    verbose: true,
                    trace: false,
                    fanin_level: 1,
                    fanin_fanout_level: 0,
                },
                FullSimplifyAction::InstallTimeoutHandler { seconds: 12 },
                FullSimplifyAction::ComputeLevelFilter,
                FullSimplifyAction::CollectPrimaryOutputs,
                FullSimplifyAction::CopyAndAttachDcNetwork,
                FullSimplifyAction::OrderPrimaryInputsByLevel,
                FullSimplifyAction::StartBddManager,
                FullSimplifyAction::ConvertNodesToBdds {
                    low_node_threshold: BDD_NODE_LOW,
                    hard_node_limit: BDD_NODE_LIMIT,
                },
                FullSimplifyAction::SimplifyWithoutOdc,
                FullSimplifyAction::CleanupBddCspfAndDcNetwork,
            ]
        );
    }

    #[test]
    fn cone_level_parser_keeps_c_atoi_edge_cases() {
        assert_eq!(get_cone_levels(""), None);
        assert_eq!(get_cone_levels("1 2"), None);
        assert_eq!(get_cone_levels("5"), Some((5, 0)));
        assert_eq!(get_cone_levels(":7"), Some((1, 7)));
        assert_eq!(get_cone_levels("8:"), Some((8, 0)));
        assert_eq!(get_cone_levels("bad:9x"), Some((0, 9)));
        assert_eq!(get_cone_levels("-2:+3"), Some((-2, 3)));
    }

    #[test]
    fn find_node_levels_matches_c_dfs_scan_rules() {
        let nodes = vec![
            SimplifyGraphNode::new("a", SimplifyNodeKind::PrimaryInput, vec![]),
            SimplifyGraphNode::new("b", SimplifyNodeKind::PrimaryInput, vec![]),
            SimplifyGraphNode::new(
                "n1",
                SimplifyNodeKind::Internal,
                vec!["a".to_owned(), "b".to_owned()],
            ),
            SimplifyGraphNode::new(
                "n2",
                SimplifyNodeKind::Internal,
                vec!["n1".to_owned(), "missing".to_owned()],
            ),
            SimplifyGraphNode::new("po", SimplifyNodeKind::PrimaryOutput, vec!["n2".to_owned()]),
        ];

        let levels = find_node_levels(&nodes);
        assert_eq!(levels.get("a"), None);
        assert_eq!(levels.get("n1"), Some(&1));
        assert_eq!(levels.get("n2"), Some(&2));
        assert_eq!(levels.get("po"), Some(&3));
    }

    #[test]
    fn rejects_unknown_options_and_values_like_usage_failures() {
        assert_eq!(
            parse_simplify_args(["-m", "espresso"]).unwrap_err(),
            CommandParseError::UnknownMethod("espresso".to_owned())
        );
        assert_eq!(
            parse_internal_simp_args(["-a", "cost"]).unwrap_err(),
            CommandParseError::UnknownAccept("cost".to_owned())
        );
        assert_eq!(
            parse_full_simplify_args(["-f", "exact"]).unwrap_err(),
            CommandParseError::UnknownFilter("exact".to_owned())
        );
        assert_eq!(
            parse_simplify_args(["-i"]).unwrap_err(),
            CommandParseError::MissingOptionValue('i')
        );
    }

    #[derive(Default)]
    struct RecordingBackend {
        simplified: Vec<SimplifyCommandPlan>,
    }

    impl SimplifyCommandBackend for RecordingBackend {
        fn simplify(&mut self, plan: &SimplifyCommandPlan) -> Result<(), SimplifyCommandError> {
            self.simplified.push(plan.clone());
            Ok(())
        }
    }

    #[test]
    fn dispatch_invokes_native_backend() {
        let mut network = RecordingBackend::default();
        let command = SimplifyCommand::Simplify(parse_simplify_args(["n"]).unwrap());

        execute_simplify_command(&mut network, &command).unwrap();

        assert_eq!(
            network.simplified,
            vec![parse_simplify_args(["n"]).unwrap()]
        );
    }
}
