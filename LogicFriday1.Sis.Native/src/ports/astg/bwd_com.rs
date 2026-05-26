//! Native Rust command model for bounded-wire-delay ASTG commands.
//!
//! The legacy unit registers command handlers and ASTG daemons, parses command
//! options, maintains per-graph bounded-wire-delay command state, and delegates
//! synthesis work to other bounded-wire-delay routines. This port keeps those
//! responsibilities as Rust data and backend dispatch points without exporting
//! legacy ABI symbols.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BwdCommandKind
{
    ToFunctions,
    ToStateGraph,
    SlowDown,
    StateGraphSingleCubeRestriction,
    StateGraphToAstg,
    StateMinimize,
    AddState,
    Encode,
    StateGraphStrongComponents,
    WriteStateGraph,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration
{
    pub name: &'static str,
    pub kind: BwdCommandKind,
    pub changes_network: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BwdDaemonKind
{
    Alloc,
    Duplicate,
    Invalid,
    Free,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DaemonRegistration
{
    pub kind: BwdDaemonKind,
}

pub const BWD_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration
    {
        name: "astg_to_f",
        kind: BwdCommandKind::ToFunctions,
        changes_network: true,
    },
    CommandRegistration
    {
        name: "astg_to_stg",
        kind: BwdCommandKind::ToStateGraph,
        changes_network: true,
    },
    CommandRegistration
    {
        name: "astg_slow",
        kind: BwdCommandKind::SlowDown,
        changes_network: true,
    },
    CommandRegistration
    {
        name: "astg_stg_scr",
        kind: BwdCommandKind::StateGraphSingleCubeRestriction,
        changes_network: true,
    },
    CommandRegistration
    {
        name: "stg_to_astg",
        kind: BwdCommandKind::StateGraphToAstg,
        changes_network: true,
    },
    CommandRegistration
    {
        name: "astg_state_min",
        kind: BwdCommandKind::StateMinimize,
        changes_network: true,
    },
    CommandRegistration
    {
        name: "astg_add_state",
        kind: BwdCommandKind::AddState,
        changes_network: true,
    },
    CommandRegistration
    {
        name: "astg_encode",
        kind: BwdCommandKind::Encode,
        changes_network: true,
    },
    CommandRegistration
    {
        name: "_stg_scc",
        kind: BwdCommandKind::StateGraphStrongComponents,
        changes_network: false,
    },
    CommandRegistration
    {
        name: "_write_sg",
        kind: BwdCommandKind::WriteStateGraph,
        changes_network: false,
    },
];

pub const BWD_DAEMONS: &[DaemonRegistration] = &[
    DaemonRegistration
    {
        kind: BwdDaemonKind::Alloc,
    },
    DaemonRegistration
    {
        kind: BwdDaemonKind::Duplicate,
    },
    DaemonRegistration
    {
        kind: BwdDaemonKind::Invalid,
    },
    DaemonRegistration
    {
        kind: BwdDaemonKind::Free,
    },
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BwdGraphState
{
    pub change_count: usize,
    pub hazards_by_signal: HashMap<String, Vec<Hazard>>,
    pub slowed_amounts: HashMap<String, f64>,
    pub primary_input_names: Vec<String>,
}

impl BwdGraphState
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn duplicate_for_change_count(&self, change_count: usize) -> Self
    {
        Self
        {
            change_count,
            hazards_by_signal: self.hazards_by_signal.clone(),
            slowed_amounts: self.slowed_amounts.clone(),
            primary_input_names: self.primary_input_names.clone(),
        }
    }

    pub fn invalidate(&mut self)
    {
        *self = Self::default();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hazard
{
    pub signal: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BwdCommand
{
    ToFunctions(ToFunctionsOptions),
    ToStateGraph(ToStateGraphOptions),
    SlowDown(SlowDownOptions),
    StateGraphSingleCubeRestriction(DebugOptions),
    StateGraphToAstg(DebugOptions),
    StateMinimize(StateMinimizeOptions),
    AddState(AddStateOptions),
    Encode(EncodeOptions),
    StateGraphStrongComponents(DebugOptions),
    WriteStateGraph(WriteStateGraphOptions),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugOptions
{
    pub debug_level: i32,
}

impl Default for DebugOptions
{
    fn default() -> Self
    {
        Self { debug_level: 0 }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToFunctionsOptions
{
    pub debug_level: i32,
    pub force_csc: bool,
    pub use_old_algorithm: bool,
    pub keep_redundant_inputs: bool,
    pub create_latches: bool,
    pub make_set_reset_disjoint: bool,
    pub find_hazards: bool,
    pub set_reset_signals: Vec<String>,
}

impl Default for ToFunctionsOptions
{
    fn default() -> Self
    {
        Self
        {
            debug_level: 0,
            force_csc: false,
            use_old_algorithm: false,
            keep_redundant_inputs: false,
            create_latches: true,
            make_set_reset_disjoint: false,
            find_hazards: true,
            set_reset_signals: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToStateGraphOptions
{
    pub debug_level: i32,
    pub use_outputs_as_inputs: bool,
    pub pre_minimize: bool,
}

impl Default for ToStateGraphOptions
{
    fn default() -> Self
    {
        Self
        {
            debug_level: 0,
            use_outputs_as_inputs: true,
            pre_minimize: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SlowDownOptions
{
    pub debug_level: i32,
    pub delay_file: Option<DelayFile>,
    pub default_delay: f64,
    pub shortest_path: bool,
    pub iterate: bool,
    pub update_only: bool,
    pub min_delay_factor: f64,
    pub tolerance: f64,
    pub linear_programming: bool,
    pub lindo: bool,
    pub bound_linear_program: bool,
}

impl Default for SlowDownOptions
{
    fn default() -> Self
    {
        Self
        {
            debug_level: 0,
            delay_file: None,
            default_delay: 0.0,
            shortest_path: true,
            iterate: false,
            update_only: false,
            min_delay_factor: 1.0,
            tolerance: 0.0,
            linear_programming: false,
            lindo: false,
            bound_linear_program: false,
        }
    }
}

impl Eq for SlowDownOptions {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelayFile
{
    pub path: String,
    pub update: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateMinimizeCover
{
    IterativeImprove,
    InitialMdd,
    ImproveWithMdd,
    InitialMinCover,
    ImproveWithMinCover,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateMinimizeGreedy
{
    Exhaustive,
    Greedy,
    VeryGreedy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PartitionDirection
{
    RemoveSignals,
    AddSignals,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateMinimizeOptions
{
    pub debug_level: i32,
    pub command: String,
    pub preminimized_file: Option<String>,
    pub signal_cost_file: Option<String>,
    pub cover: StateMinimizeCover,
    pub greedy: StateMinimizeGreedy,
    pub direction: PartitionDirection,
    pub mincov_option: i32,
}

impl Default for StateMinimizeOptions
{
    fn default() -> Self
    {
        Self
        {
            debug_level: 0,
            command: "sred -c".to_owned(),
            preminimized_file: None,
            signal_cost_file: None,
            cover: StateMinimizeCover::IterativeImprove,
            greedy: StateMinimizeGreedy::Exhaustive,
            direction: PartitionDirection::RemoveSignals,
            mincov_option: 4096,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddStateOptions
{
    pub debug_level: i32,
    pub restore_marking: bool,
}

impl Default for AddStateOptions
{
    fn default() -> Self
    {
        Self
        {
            debug_level: 0,
            restore_marking: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodeOptions
{
    pub debug_level: i32,
    pub user_defined_codes: bool,
    pub print_summary: bool,
    pub heuristic: bool,
}

impl Default for EncodeOptions
{
    fn default() -> Self
    {
        Self
        {
            debug_level: 0,
            user_defined_codes: false,
            print_summary: false,
            heuristic: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteStateGraphOptions
{
    pub debug_level: i32,
    pub filename: String,
}

impl Default for WriteStateGraphOptions
{
    fn default() -> Self
    {
        Self
        {
            debug_level: 0,
            filename: "-".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BwdCommandOutput
{
    pub status: i32,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl BwdCommandOutput
{
    pub fn success(stdout: Vec<String>) -> Self
    {
        Self
        {
            status: 0,
            stdout,
            stderr: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BwdCommandError
{
    UnknownCommand(String),
    UnsupportedOption(String),
    MissingOptionValue(char),
    InvalidInteger
    {
        option: char,
        value: String,
    },
    InvalidFloat
    {
        option: char,
        value: String,
    },
    InvalidMinDelayFactor(f64),
    UnexpectedArgument(String),
    TooManyArguments
    {
        command: BwdCommandKind,
        max: usize,
        actual: usize,
    },
    IncompatibleStateMinimizeOptions,
    Backend(String),
}

impl Eq for BwdCommandError {}

impl fmt::Display for BwdCommandError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::UnknownCommand(command) => write!(
                formatter,
                "unknown bounded-wire-delay ASTG command {command}"
            ),
            Self::UnsupportedOption(option) => write!(
                formatter,
                "unsupported bounded-wire-delay ASTG option {option}"
            ),
            Self::MissingOptionValue(option) => write!(formatter, "missing value for -{option}"),
            Self::InvalidInteger { option, value } =>
            {
                write!(formatter, "invalid integer for -{option}: {value}")
            }
            Self::InvalidFloat { option, value } => write!(
                formatter,
                "invalid floating-point value for -{option}: {value}"
            ),
            Self::InvalidMinDelayFactor(value) =>
            {
                write!(
                    formatter,
                    "min delay factor must be greater than 0 and at most 1, got {value}"
                )
            }
            Self::UnexpectedArgument(argument) =>
            {
                write!(formatter, "unexpected argument {argument}")
            }
            Self::TooManyArguments
            {
                command,
                max,
                actual,
            } => write!(
                formatter,
                "{command:?} accepts at most {max} arguments, got {actual}"
            ),
            Self::IncompatibleStateMinimizeOptions => formatter
                .write_str("greedy and min-cover state minimization only support removing signals"),
            Self::Backend(message) => formatter.write_str(message),
        }
    }
}

impl Error for BwdCommandError {}

pub trait BwdBackend
{
    fn astg_to_functions(
        &mut self,
        options: &ToFunctionsOptions,
    ) -> Result<BwdCommandOutput, BwdCommandError>;

    fn astg_to_state_graph(
        &mut self,
        options: &ToStateGraphOptions,
    ) -> Result<BwdCommandOutput, BwdCommandError>;

    fn slow_down(&mut self, options: &SlowDownOptions)
        -> Result<BwdCommandOutput, BwdCommandError>;

    fn state_graph_single_cube_restriction(
        &mut self,
        options: &DebugOptions,
    ) -> Result<BwdCommandOutput, BwdCommandError>;

    fn state_graph_to_astg(
        &mut self,
        options: &DebugOptions,
    ) -> Result<BwdCommandOutput, BwdCommandError>;

    fn state_minimize(
        &mut self,
        options: &StateMinimizeOptions,
    ) -> Result<BwdCommandOutput, BwdCommandError>;

    fn add_state(&mut self, options: &AddStateOptions)
        -> Result<BwdCommandOutput, BwdCommandError>;

    fn encode(&mut self, options: &EncodeOptions) -> Result<BwdCommandOutput, BwdCommandError>;

    fn state_graph_strong_components(
        &mut self,
        options: &DebugOptions,
    ) -> Result<BwdCommandOutput, BwdCommandError>;

    fn write_state_graph(
        &mut self,
        options: &WriteStateGraphOptions,
    ) -> Result<BwdCommandOutput, BwdCommandError>;
}

pub fn bwd_command_registrations() -> &'static [CommandRegistration]
{
    BWD_COMMANDS
}

pub fn bwd_daemon_registrations() -> &'static [DaemonRegistration]
{
    BWD_DAEMONS
}

pub fn bwd_alloc_daemon() -> BwdGraphState
{
    BwdGraphState::new()
}

pub fn bwd_dup_daemon(old_state: &BwdGraphState, new_change_count: usize) -> BwdGraphState
{
    old_state.duplicate_for_change_count(new_change_count)
}

pub fn bwd_invalid_daemon(state: &mut BwdGraphState)
{
    state.invalidate();
}

pub fn bwd_free_daemon(state: &mut Option<BwdGraphState>)
{
    *state = None;
}

pub fn parse_bwd_command<I, S>(command_name: &str, args: I) -> Result<BwdCommand, BwdCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();

    match command_name
    {
        "astg_to_f" => parse_to_functions_args(args).map(BwdCommand::ToFunctions),
        "astg_to_stg" => parse_to_state_graph_args(args).map(BwdCommand::ToStateGraph),
        "astg_slow" => parse_slow_down_args(args).map(BwdCommand::SlowDown),
        "astg_stg_scr" => parse_debug_args(BwdCommandKind::StateGraphSingleCubeRestriction, args)
            .map(BwdCommand::StateGraphSingleCubeRestriction),
        "stg_to_astg" => parse_debug_args(BwdCommandKind::StateGraphToAstg, args)
            .map(BwdCommand::StateGraphToAstg),
        "astg_state_min" => parse_state_minimize_args(args).map(BwdCommand::StateMinimize),
        "astg_add_state" => parse_add_state_args(args).map(BwdCommand::AddState),
        "astg_encode" => parse_encode_args(args).map(BwdCommand::Encode),
        "_stg_scc" => parse_debug_args(BwdCommandKind::StateGraphStrongComponents, args)
            .map(BwdCommand::StateGraphStrongComponents),
        "_write_sg" => parse_write_state_graph_args(args).map(BwdCommand::WriteStateGraph),
        _ => Err(BwdCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn parse_to_functions_args<I, S>(args: I) -> Result<ToFunctionsOptions, BwdCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = ToFunctionsOptions::default();
    let args = collect_args(args);
    let mut index = 0;

    while index < args.len()
    {
        let arg = &args[index];
        if !arg.starts_with('-') || arg == "-"
        {
            options.set_reset_signals.push(arg.clone());
            index += 1;
            continue;
        }

        index = parse_short_flags(&args, index, "v", |flag, value| match flag
        {
            'f' =>
            {
                options.force_csc = true;
                Ok(())
            }
            'o' =>
            {
                options.use_old_algorithm = true;
                Ok(())
            }
            'r' =>
            {
                options.keep_redundant_inputs = true;
                Ok(())
            }
            'l' =>
            {
                options.create_latches = false;
                Ok(())
            }
            'd' =>
            {
                options.make_set_reset_disjoint = true;
                Ok(())
            }
            'h' =>
            {
                options.find_hazards = false;
                Ok(())
            }
            'v' =>
            {
                options.debug_level = parse_i32('v', value?)?;
                Ok(())
            }
            _ => Err(BwdCommandError::UnsupportedOption(format!("-{flag}"))),
        })?;
    }

    Ok(options)
}

pub fn parse_to_state_graph_args<I, S>(args: I) -> Result<ToStateGraphOptions, BwdCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = ToStateGraphOptions::default();
    parse_debug_like_args(
        collect_args(args),
        "om",
        |flag| match flag
        {
            'o' =>
            {
                options.use_outputs_as_inputs = false;
                Ok(())
            }
            'm' =>
            {
                options.pre_minimize = true;
                Ok(())
            }
            _ => Err(BwdCommandError::UnsupportedOption(format!("-{flag}"))),
        },
        |debug| options.debug_level = debug,
    )?;

    Ok(options)
}

pub fn parse_slow_down_args<I, S>(args: I) -> Result<SlowDownOptions, BwdCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = SlowDownOptions::default();
    let args = collect_args(args);
    let mut index = 0;

    while index < args.len()
    {
        let arg = &args[index];
        if !arg.starts_with('-') || arg == "-"
        {
            return Err(BwdCommandError::UnexpectedArgument(arg.clone()));
        }

        index = parse_short_flags(&args, index, "vdfFmt", |flag, value| match flag
        {
            'b' =>
            {
                options.bound_linear_program = true;
                Ok(())
            }
            'l' =>
            {
                options.linear_programming = true;
                Ok(())
            }
            'L' =>
            {
                options.linear_programming = true;
                options.lindo = true;
                Ok(())
            }
            'v' =>
            {
                options.debug_level = parse_i32('v', value?)?;
                Ok(())
            }
            'd' =>
            {
                options.default_delay = parse_f64('d', value?)?;
                Ok(())
            }
            'i' =>
            {
                options.iterate = true;
                Ok(())
            }
            'm' =>
            {
                options.min_delay_factor = parse_f64('m', value?)?;
                Ok(())
            }
            's' =>
            {
                options.shortest_path = false;
                Ok(())
            }
            't' =>
            {
                options.tolerance = parse_f64('t', value?)?;
                Ok(())
            }
            'f' =>
            {
                options.delay_file = Some(DelayFile
                {
                    path: value?.to_owned(),
                    update: false,
                });
                Ok(())
            }
            'F' =>
            {
                options.delay_file = Some(DelayFile
                {
                    path: value?.to_owned(),
                    update: true,
                });
                Ok(())
            }
            'u' =>
            {
                options.update_only = true;
                Ok(())
            }
            _ => Err(BwdCommandError::UnsupportedOption(format!("-{flag}"))),
        })?;
    }

    if !(options.min_delay_factor > 0.0 && options.min_delay_factor <= 1.0)
    {
        return Err(BwdCommandError::InvalidMinDelayFactor(
            options.min_delay_factor,
        ));
    }

    Ok(options)
}

pub fn parse_state_minimize_args<I, S>(args: I) -> Result<StateMinimizeOptions, BwdCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = StateMinimizeOptions::default();
    let args = collect_args(args);
    let mut index = 0;

    while index < args.len()
    {
        let arg = &args[index];
        if !arg.starts_with('-') || arg == "-"
        {
            return Err(BwdCommandError::UnexpectedArgument(arg.clone()));
        }

        index = parse_short_flags(&args, index, "fovcp", |flag, value| match flag
        {
            'f' =>
            {
                options.signal_cost_file = Some(value?.to_owned());
                Ok(())
            }
            'c' =>
            {
                options.command = value?.to_owned();
                Ok(())
            }
            'p' =>
            {
                options.preminimized_file = Some(value?.to_owned());
                Ok(())
            }
            'b' =>
            {
                options.cover = StateMinimizeCover::InitialMdd;
                Ok(())
            }
            'B' =>
            {
                options.cover = StateMinimizeCover::ImproveWithMdd;
                Ok(())
            }
            'u' =>
            {
                options.direction = PartitionDirection::AddSignals;
                Ok(())
            }
            'g' =>
            {
                options.greedy = StateMinimizeGreedy::Greedy;
                Ok(())
            }
            'G' =>
            {
                options.greedy = StateMinimizeGreedy::VeryGreedy;
                Ok(())
            }
            'm' =>
            {
                options.cover = StateMinimizeCover::InitialMinCover;
                Ok(())
            }
            'M' =>
            {
                options.cover = StateMinimizeCover::ImproveWithMinCover;
                Ok(())
            }
            'o' =>
            {
                options.mincov_option = parse_i32('o', value?)?;
                Ok(())
            }
            'v' =>
            {
                options.debug_level = parse_i32('v', value?)?;
                Ok(())
            }
            _ => Err(BwdCommandError::UnsupportedOption(format!("-{flag}"))),
        })?;
    }

    if options.direction == PartitionDirection::AddSignals
        && (options.greedy != StateMinimizeGreedy::Exhaustive
            || matches!(
                options.cover,
                StateMinimizeCover::InitialMinCover | StateMinimizeCover::ImproveWithMinCover
            ))
    {
        return Err(BwdCommandError::IncompatibleStateMinimizeOptions);
    }

    Ok(options)
}

pub fn parse_add_state_args<I, S>(args: I) -> Result<AddStateOptions, BwdCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = AddStateOptions::default();
    parse_debug_like_args(
        collect_args(args),
        "m",
        |flag| match flag
        {
            'm' =>
            {
                options.restore_marking = false;
                Ok(())
            }
            _ => Err(BwdCommandError::UnsupportedOption(format!("-{flag}"))),
        },
        |debug| options.debug_level = debug,
    )?;

    Ok(options)
}

pub fn parse_encode_args<I, S>(args: I) -> Result<EncodeOptions, BwdCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = EncodeOptions::default();
    parse_debug_like_args(
        collect_args(args),
        "ush",
        |flag| match flag
        {
            'u' =>
            {
                options.user_defined_codes = true;
                Ok(())
            }
            's' =>
            {
                options.print_summary = true;
                Ok(())
            }
            'h' =>
            {
                options.heuristic = true;
                Ok(())
            }
            _ => Err(BwdCommandError::UnsupportedOption(format!("-{flag}"))),
        },
        |debug| options.debug_level = debug,
    )?;

    Ok(options)
}

pub fn parse_write_state_graph_args<I, S>(
    args: I,
) -> Result<WriteStateGraphOptions, BwdCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = WriteStateGraphOptions::default();
    let args = collect_args(args);
    let mut positional = Vec::new();
    let mut index = 0;

    while index < args.len()
    {
        let arg = &args[index];
        if !arg.starts_with('-') || arg == "-"
        {
            positional.push(arg.clone());
            index += 1;
            continue;
        }

        index = parse_short_flags(&args, index, "v", |flag, value| match flag
        {
            'v' =>
            {
                options.debug_level = parse_i32('v', value?)?;
                Ok(())
            }
            _ => Err(BwdCommandError::UnsupportedOption(format!("-{flag}"))),
        })?;
    }

    if positional.len() > 1
    {
        return Err(BwdCommandError::TooManyArguments
        {
            command: BwdCommandKind::WriteStateGraph,
            max: 1,
            actual: positional.len(),
        });
    }

    if let Some(filename) = positional.pop()
    {
        options.filename = filename;
    }

    Ok(options)
}

pub fn dispatch_bwd_command<B>(
    backend: &mut B,
    command: &BwdCommand,
) -> Result<BwdCommandOutput, BwdCommandError>
where
    B: BwdBackend,
{
    match command
    {
        BwdCommand::ToFunctions(options) => backend.astg_to_functions(options),
        BwdCommand::ToStateGraph(options) => backend.astg_to_state_graph(options),
        BwdCommand::SlowDown(options) => backend.slow_down(options),
        BwdCommand::StateGraphSingleCubeRestriction(options) =>
        {
            backend.state_graph_single_cube_restriction(options)
        }
        BwdCommand::StateGraphToAstg(options) => backend.state_graph_to_astg(options),
        BwdCommand::StateMinimize(options) => backend.state_minimize(options),
        BwdCommand::AddState(options) => backend.add_state(options),
        BwdCommand::Encode(options) => backend.encode(options),
        BwdCommand::StateGraphStrongComponents(options) =>
        {
            backend.state_graph_strong_components(options)
        }
        BwdCommand::WriteStateGraph(options) => backend.write_state_graph(options),
    }
}

fn parse_debug_args(
    command: BwdCommandKind,
    args: Vec<String>,
) -> Result<DebugOptions, BwdCommandError>
{
    let mut options = DebugOptions::default();
    parse_debug_like_args(
        args,
        "",
        |_| unreachable!(),
        |debug| options.debug_level = debug,
    )?;

    let _ = command;
    Ok(options)
}

fn parse_debug_like_args<Flag, SetDebug>(
    args: Vec<String>,
    value_less_flags: &str,
    mut flag_action: Flag,
    mut set_debug: SetDebug,
) -> Result<(), BwdCommandError>
where
    Flag: FnMut(char) -> Result<(), BwdCommandError>,
    SetDebug: FnMut(i32),
{
    let mut index = 0;
    while index < args.len()
    {
        let arg = &args[index];
        if !arg.starts_with('-') || arg == "-"
        {
            return Err(BwdCommandError::UnexpectedArgument(arg.clone()));
        }

        index = parse_short_flags(&args, index, "v", |flag, value|
        {
            if flag == 'v'
            {
                set_debug(parse_i32('v', value?)?);
                Ok(())
            }
else if value_less_flags.contains(flag)
{
                flag_action(flag)
            }
else
{
                Err(BwdCommandError::UnsupportedOption(format!("-{flag}")))
            }
        })?;
    }

    Ok(())
}

fn parse_short_flags<F>(
    args: &[String],
    index: usize,
    value_flags: &str,
    mut action: F,
) -> Result<usize, BwdCommandError>
where
    F: FnMut(char, Result<&str, BwdCommandError>) -> Result<(), BwdCommandError>,
{
    let arg = &args[index];
    let flags = arg
        .strip_prefix('-')
        .ok_or_else(|| BwdCommandError::UnexpectedArgument(arg.clone()))?;

    if flags.is_empty()
    {
        return Err(BwdCommandError::UnexpectedArgument(arg.clone()));
    }

    let chars = flags.chars().collect::<Vec<_>>();
    let mut flag_index = 0;
    while flag_index < chars.len()
    {
        let flag = chars[flag_index];
        if value_flags.contains(flag)
        {
            let inline = chars[(flag_index + 1)..].iter().collect::<String>();
            if inline.is_empty()
            {
                let value = args
                    .get(index + 1)
                    .ok_or(BwdCommandError::MissingOptionValue(flag))?;
                action(flag, Ok(value))?;
                return Ok(index + 2);
            }

            action(flag, Ok(&inline))?;
            return Ok(index + 1);
        }

        action(flag, Err(BwdCommandError::MissingOptionValue(flag)))?;
        flag_index += 1;
    }

    Ok(index + 1)
}

fn collect_args<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect()
}

fn parse_i32(option: char, value: &str) -> Result<i32, BwdCommandError>
{
    value.parse().map_err(|_| BwdCommandError::InvalidInteger
    {
        option,
        value: value.to_owned(),
    })
}

fn parse_f64(option: char, value: &str) -> Result<f64, BwdCommandError>
{
    value.parse().map_err(|_| BwdCommandError::InvalidFloat
    {
        option,
        value: value.to_owned(),
    })
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Default)]
    struct RecordingBackend
    {
        calls: Vec<String>,
    }

    impl BwdBackend for RecordingBackend
    {
        fn astg_to_functions(
            &mut self,
            options: &ToFunctionsOptions,
        ) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls.push(format!(
                "to_f:{}:{}:{}",
                options.debug_level,
                options.create_latches,
                options.set_reset_signals.join(",")
            ));
            Ok(BwdCommandOutput::success(Vec::new()))
        }

        fn astg_to_state_graph(
            &mut self,
            options: &ToStateGraphOptions,
        ) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls.push(format!(
                "to_stg:{}:{}:{}",
                options.debug_level, options.use_outputs_as_inputs, options.pre_minimize
            ));
            Ok(BwdCommandOutput::success(Vec::new()))
        }

        fn slow_down(
            &mut self,
            options: &SlowDownOptions,
        ) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls.push(format!(
                "slow:{}:{}:{}",
                options.debug_level,
                options.linear_programming,
                options
                    .delay_file
                    .as_ref()
                    .map(|file| file.path.as_str())
                    .unwrap_or("")
            ));
            Ok(BwdCommandOutput::success(Vec::new()))
        }

        fn state_graph_single_cube_restriction(
            &mut self,
            options: &DebugOptions,
        ) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls.push(format!("scr:{}", options.debug_level));
            Ok(BwdCommandOutput::success(Vec::new()))
        }

        fn state_graph_to_astg(
            &mut self,
            options: &DebugOptions,
        ) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls
                .push(format!("stg_to_astg:{}", options.debug_level));
            Ok(BwdCommandOutput::success(Vec::new()))
        }

        fn state_minimize(
            &mut self,
            options: &StateMinimizeOptions,
        ) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls.push(format!(
                "state_min:{}:{}",
                options.debug_level, options.command
            ));
            Ok(BwdCommandOutput::success(Vec::new()))
        }

        fn add_state(
            &mut self,
            options: &AddStateOptions,
        ) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls.push(format!(
                "add_state:{}:{}",
                options.debug_level, options.restore_marking
            ));
            Ok(BwdCommandOutput::success(Vec::new()))
        }

        fn encode(&mut self, options: &EncodeOptions) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls.push(format!(
                "encode:{}:{}:{}:{}",
                options.debug_level,
                options.user_defined_codes,
                options.print_summary,
                options.heuristic
            ));
            Ok(BwdCommandOutput::success(Vec::new()))
        }

        fn state_graph_strong_components(
            &mut self,
            options: &DebugOptions,
        ) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls.push(format!("scc:{}", options.debug_level));
            Ok(BwdCommandOutput::success(Vec::new()))
        }

        fn write_state_graph(
            &mut self,
            options: &WriteStateGraphOptions,
        ) -> Result<BwdCommandOutput, BwdCommandError>
        {
            self.calls.push(format!(
                "write:{}:{}",
                options.debug_level, options.filename
            ));
            Ok(BwdCommandOutput::success(Vec::new()))
        }
    }

    #[test]
    fn registers_legacy_commands_and_daemons()
    {
        assert_eq!(
            bwd_command_registrations()
                .iter()
                .map(|registration| registration.name)
                .collect::<Vec<_>>(),
            vec![
                "astg_to_f",
                "astg_to_stg",
                "astg_slow",
                "astg_stg_scr",
                "stg_to_astg",
                "astg_state_min",
                "astg_add_state",
                "astg_encode",
                "_stg_scc",
                "_write_sg",
            ]
        );
        assert_eq!(
            bwd_daemon_registrations()
                .iter()
                .map(|registration| registration.kind)
                .collect::<Vec<_>>(),
            vec![
                BwdDaemonKind::Alloc,
                BwdDaemonKind::Duplicate,
                BwdDaemonKind::Invalid,
                BwdDaemonKind::Free,
            ]
        );
        assert_eq!(
            bwd_command_registrations()
                .iter()
                .filter(|registration| !registration.changes_network)
                .map(|registration| registration.name)
                .collect::<Vec<_>>(),
            vec!["_stg_scc", "_write_sg"]
        );
    }

    #[test]
    fn daemon_state_duplicates_and_invalidates_owned_data()
    {
        let mut state = bwd_alloc_daemon();
        state.change_count = 7;
        state.primary_input_names.push("a".to_owned());
        state.slowed_amounts.insert("a".to_owned(), 1.25);
        state.hazards_by_signal.insert(
            "a".to_owned(),
            vec![Hazard
            {
                signal: "a".to_owned(),
                description: "late reset".to_owned(),
            }],
        );

        let duplicated = bwd_dup_daemon(&state, 9);

        assert_eq!(duplicated.change_count, 9);
        assert_eq!(duplicated.primary_input_names, vec!["a"]);
        assert_eq!(duplicated.slowed_amounts["a"], 1.25);
        assert_eq!(
            duplicated.hazards_by_signal["a"][0].description,
            "late reset"
        );

        bwd_invalid_daemon(&mut state);
        assert_eq!(state, BwdGraphState::new());

        let mut optional = Some(duplicated);
        bwd_free_daemon(&mut optional);
        assert_eq!(optional, None);
    }

    #[test]
    fn parses_to_functions_legacy_flags_and_sr_signal_names()
    {
        assert_eq!(
            parse_to_functions_args(["-forv3", "-dlh", "ack", "*"]).unwrap(),
            ToFunctionsOptions
            {
                debug_level: 3,
                force_csc: true,
                use_old_algorithm: true,
                keep_redundant_inputs: true,
                create_latches: false,
                make_set_reset_disjoint: true,
                find_hazards: false,
                set_reset_signals: vec!["ack".to_owned(), "*".to_owned()],
            }
        );
    }

    #[test]
    fn parses_to_state_graph_and_simple_debug_commands()
    {
        assert_eq!(
            parse_to_state_graph_args(["-v", "2", "-om"]).unwrap(),
            ToStateGraphOptions
            {
                debug_level: 2,
                use_outputs_as_inputs: false,
                pre_minimize: true,
            }
        );
        assert_eq!(
            parse_bwd_command("astg_stg_scr", ["-v4"]).unwrap(),
            BwdCommand::StateGraphSingleCubeRestriction(DebugOptions { debug_level: 4 })
        );
        assert_eq!(
            parse_bwd_command("_stg_scc", ["operand"]),
            Err(BwdCommandError::UnexpectedArgument("operand".to_owned()))
        );
    }

    #[test]
    fn parses_slow_down_delay_modes_and_validates_factor()
    {
        let options = parse_slow_down_args([
            "-b",
            "-L",
            "-Fdelays.txt",
            "-d",
            "0.5",
            "-im0.75",
            "-s",
            "-t-0.1",
            "-u",
            "-v2",
        ])
        .unwrap();

        assert_eq!(options.debug_level, 2);
        assert_eq!(
            options.delay_file.unwrap(),
            DelayFile
            {
                path: "delays.txt".to_owned(),
                update: true,
            }
        );
        assert_eq!(options.default_delay, 0.5);
        assert_eq!(options.min_delay_factor, 0.75);
        assert_eq!(options.tolerance, -0.1);
        assert!(options.iterate);
        assert!(options.update_only);
        assert!(options.linear_programming);
        assert!(options.lindo);
        assert!(options.bound_linear_program);
        assert!(!options.shortest_path);

        assert_eq!(
            parse_slow_down_args(["-m", "1.5"]),
            Err(BwdCommandError::InvalidMinDelayFactor(1.5))
        );
    }

    #[test]
    fn parses_state_minimize_modes_and_rejects_legacy_invalid_combo()
    {
        assert_eq!(
            parse_state_minimize_args([
                "-B",
                "-G",
                "-o32",
                "-f",
                "costs.txt",
                "-c",
                "custom",
                "-pmin.kiss",
                "-v",
                "5",
            ])
            .unwrap(),
            StateMinimizeOptions
            {
                debug_level: 5,
                command: "custom".to_owned(),
                preminimized_file: Some("min.kiss".to_owned()),
                signal_cost_file: Some("costs.txt".to_owned()),
                cover: StateMinimizeCover::ImproveWithMdd,
                greedy: StateMinimizeGreedy::VeryGreedy,
                direction: PartitionDirection::RemoveSignals,
                mincov_option: 32,
            }
        );
        assert_eq!(
            parse_state_minimize_args(["-u", "-m"]),
            Err(BwdCommandError::IncompatibleStateMinimizeOptions)
        );
        assert_eq!(
            parse_state_minimize_args(["-u", "-g"]),
            Err(BwdCommandError::IncompatibleStateMinimizeOptions)
        );
    }

    #[test]
    fn parses_add_state_encode_and_write_state_graph()
    {
        assert_eq!(
            parse_add_state_args(["-v1", "-m"]).unwrap(),
            AddStateOptions
            {
                debug_level: 1,
                restore_marking: false,
            }
        );
        assert_eq!(
            parse_encode_args(["-ush", "-v", "6"]).unwrap(),
            EncodeOptions
            {
                debug_level: 6,
                user_defined_codes: true,
                print_summary: true,
                heuristic: true,
            }
        );
        assert_eq!(
            parse_write_state_graph_args(["-v3", "out.sg"]).unwrap(),
            WriteStateGraphOptions
            {
                debug_level: 3,
                filename: "out.sg".to_owned(),
            }
        );
        assert_eq!(
            parse_write_state_graph_args(["a", "b"]),
            Err(BwdCommandError::TooManyArguments
            {
                command: BwdCommandKind::WriteStateGraph,
                max: 1,
                actual: 2,
            })
        );
    }

    #[test]
    fn dispatches_parsed_commands_to_native_backend()
    {
        let mut backend = RecordingBackend::default();

        for command in [
            parse_bwd_command("astg_to_f", ["-l", "req"]).unwrap(),
            parse_bwd_command("astg_to_stg", ["-om"]).unwrap(),
            parse_bwd_command("astg_slow", ["-l", "-f", "d.txt"]).unwrap(),
            parse_bwd_command("astg_stg_scr", ["-v1"]).unwrap(),
            parse_bwd_command("stg_to_astg", ["-v2"]).unwrap(),
            parse_bwd_command("astg_state_min", ["-c", "sred -c"]).unwrap(),
            parse_bwd_command("astg_add_state", ["-m"]).unwrap(),
            parse_bwd_command("astg_encode", ["-s"]).unwrap(),
            parse_bwd_command("_stg_scc", ["-v8"]).unwrap(),
            parse_bwd_command("_write_sg", ["out.sg"]).unwrap(),
        ]
        {
            dispatch_bwd_command(&mut backend, &command).unwrap();
        }

        assert_eq!(
            backend.calls,
            vec![
                "to_f:0:false:req",
                "to_stg:0:false:true",
                "slow:0:true:d.txt",
                "scr:1",
                "stg_to_astg:2",
                "state_min:0:sred -c",
                "add_state:0:false",
                "encode:0:false:true:false",
                "scc:8",
                "write:0:out.sg",
            ]
        );
    }

    #[test]
    fn reports_option_value_errors()
    {
        assert_eq!(
            parse_to_functions_args(["-v"]),
            Err(BwdCommandError::MissingOptionValue('v'))
        );
        assert_eq!(
            parse_slow_down_args(["-d", "soon"]),
            Err(BwdCommandError::InvalidFloat
            {
                option: 'd',
                value: "soon".to_owned(),
            })
        );
        assert_eq!(
            parse_encode_args(["-x"]),
            Err(BwdCommandError::UnsupportedOption("-x".to_owned()))
        );
        assert_eq!(
            parse_bwd_command("missing", Vec::<&str>::new()),
            Err(BwdCommandError::UnknownCommand("missing".to_owned()))
        );
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_abi_exports()
    {
        let text = include_str!("bwd_com.rs");

        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains("extern \"C\""));
    }
}

