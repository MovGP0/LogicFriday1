//! Native Rust command model for `LogicSynthesis/sis/speed/com_speed.c`.
//!
//! The C file registers the SIS speed commands, owns the command-line option
//! defaults for speed-up and buffering, and dispatches into delay, network,
//! library, buffering, plotting, and speed optimization routines. Those SIS
//! integration layers are not all native Rust yet, so this module ports the
//! deterministic command/default/parsing behavior and exposes network-bound
//! entry points as explicit missing-dependency errors.

use std::error::Error;
use std::fmt;

pub const DEFAULT_SPEED_THRESH: f64 = 0.5;
pub const DEFAULT_SPEED_COEFF: f64 = 0.0;
pub const DEFAULT_SPEED_DIST: i32 = 3;
pub const DEFAULT_MAX_NUM_CUTS: i32 = 50;
pub const DEFAULT_BUFFER_LIMIT: i32 = 2;
pub const DEFAULT_BUFFER_MODE: i32 = 7;
pub const V_SMALL: f64 = 1.0e-9;

pub const REQUIRED_PORTS: &[PortDependency] = &[
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.133",
        source: "LogicSynthesis/sis/delay/delay.c",
        reason: "delay model lookup, delay tracing, slack and arrival data",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.214",
        source: "LogicSynthesis/sis/graphics/com_graphics.c",
        reason: "conditional _speed_plot command registration",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.258",
        source: "LogicSynthesis/sis/map/libutil.c",
        reason: "mapped-network detection",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.257",
        source: "LogicSynthesis/sis/map/library.c",
        reason: "mapped-library presence and gate data",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.305",
        source: "LogicSynthesis/sis/network/network_util.c",
        reason: "network PI count, node selection, and traversal",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.318",
        source: "LogicSynthesis/sis/node/node.c",
        reason: "node type/function/fanin/fanout inspection",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.455",
        source: "LogicSynthesis/sis/simplify/simp.c",
        reason: "redundancy removal during speed-up",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.467",
        source: "LogicSynthesis/sis/speed/new_speed.c",
        reason: "new speed optimization engine",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.468",
        source: "LogicSynthesis/sis/speed/new_wght_util.c",
        reason: "local transform selection and new cutset weighting",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.472",
        source: "LogicSynthesis/sis/speed/speed_and.c",
        reason: "initial two-input NAND decomposition",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.474",
        source: "LogicSynthesis/sis/speed/speed_delay.c",
        reason: "speed delay data setup and tracing",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.475",
        source: "LogicSynthesis/sis/speed/speed_loop.c",
        reason: "speed-up script and repeated optimization loop",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.476",
        source: "LogicSynthesis/sis/speed/speed_net.c",
        reason: "per-node and per-network decomposition",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.479",
        source: "LogicSynthesis/sis/speed/speed_plot.c",
        reason: "graphics command implementation",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.480",
        source: "LogicSynthesis/sis/speed/speed_util.c",
        reason: "thresholds, levels, performance reporting",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.481",
        source: "LogicSynthesis/sis/speed/speedup.c",
        reason: "legacy speed_up_network implementation",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.483",
        source: "LogicSynthesis/sis/speed/weight.c",
        reason: "critical-path weight and cutset computation",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.466",
        source: "LogicSynthesis/sis/speed/gbx.c",
        reason: "buffer allocation/free daemons and mapped buffering helpers",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.470",
        source: "LogicSynthesis/sis/speed/sp_buffer.c",
        reason: "network and node buffering implementation",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.473",
        source: "LogicSynthesis/sis/speed/sp_network.c",
        reason: "PI/PO load and drive default handling",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead: &'static str,
    pub source: &'static str,
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    Library,
    UnitFanout,
    Mapped,
    Tdc,
}

impl DelayModel {
    pub fn from_c_name(name: &str) -> Option<Self> {
        match name {
            "unit" | "DELAY_MODEL_UNIT" => Some(Self::Unit),
            "library" | "DELAY_MODEL_LIBRARY" => Some(Self::Library),
            "unit-fanout" | "unit_fanout" | "DELAY_MODEL_UNIT_FANOUT" => Some(Self::UnitFanout),
            "mapped" | "DELAY_MODEL_MAPPED" => Some(Self::Mapped),
            "tdc" | "DELAY_MODEL_TDC" => Some(Self::Tdc),
            _ => None,
        }
    }

    pub fn command_model(self, interactive: bool) -> DelayModelSelection {
        if self == Self::Library {
            DelayModelSelection {
                model: Self::Mapped,
                notice: interactive.then_some("Using MAPPED model instead of LIBRARY"),
            }
        } else {
            DelayModelSelection {
                model: self,
                notice: None,
            }
        }
    }

    pub fn print_command_model(self) -> DelayModelSelection {
        if self == Self::Library {
            DelayModelSelection {
                model: Self::Mapped,
                notice: Some("Notice: Using the mapped model instead"),
            }
        } else {
            DelayModelSelection {
                model: self,
                notice: None,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelayModelSelection {
    pub model: DelayModel,
    pub notice: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedRegion {
    AlongCriticalPath,
    TransitiveFanin,
    Compromise,
    OnlyTree,
}

impl SpeedRegion {
    pub fn from_c_name(name: &str) -> Option<Self> {
        match name {
            "crit" => Some(Self::AlongCriticalPath),
            "transitive" => Some(Self::TransitiveFanin),
            "compromise" => Some(Self::Compromise),
            "tree" => Some(Self::OnlyTree),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformSelection {
    BestBenefit,
    BestBangForBuck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedObjective {
    AreaBased,
    TransformBased,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedOptions {
    pub area_reclaim: bool,
    pub trace: bool,
    pub debug: i32,
    pub add_inv: bool,
    pub del_crit_cubes: bool,
    pub num_tries: i32,
    pub thresh: f64,
    pub coeff: f64,
    pub dist: i32,
    pub new_mode: bool,
    pub region: SpeedRegion,
    pub transform: TransformSelection,
    pub max_recur: i32,
    pub timeout: f64,
    pub max_num_cuts: i32,
    pub red_removal: bool,
    pub objective: SpeedObjective,
    pub req_times_set: bool,
    pub do_script: bool,
    pub speed_repeat: bool,
    pub only_init_decomp: bool,
    pub model: DelayModel,
    pub pin_cap: f64,
    pub library_acceleration: bool,
    pub interactive: bool,
    pub local_transforms: Vec<String>,
}

impl Default for SpeedOptions {
    fn default() -> Self {
        Self {
            area_reclaim: true,
            trace: false,
            debug: 0,
            add_inv: false,
            del_crit_cubes: true,
            num_tries: 1,
            thresh: DEFAULT_SPEED_THRESH,
            coeff: DEFAULT_SPEED_COEFF,
            dist: DEFAULT_SPEED_DIST,
            new_mode: true,
            region: SpeedRegion::AlongCriticalPath,
            transform: TransformSelection::BestBenefit,
            max_recur: 1,
            timeout: f64::INFINITY,
            max_num_cuts: DEFAULT_MAX_NUM_CUTS,
            red_removal: false,
            objective: SpeedObjective::AreaBased,
            req_times_set: false,
            do_script: true,
            speed_repeat: true,
            only_init_decomp: false,
            model: DelayModel::Unit,
            pin_cap: 0.0,
            library_acceleration: false,
            interactive: false,
            local_transforms: Vec::new(),
        }
    }
}

impl SpeedOptions {
    pub fn interactive_defaults() -> Self {
        Self {
            interactive: true,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedCommandPlan {
    pub options: SpeedOptions,
    pub node_names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintLevelOptions {
    pub model: DelayModel,
    pub thresh: f64,
    pub crit_only: bool,
    pub print_flag: bool,
    pub node_names: Vec<String>,
    pub notice: Option<&'static str>,
}

impl Default for PrintLevelOptions {
    fn default() -> Self {
        Self {
            model: DelayModel::UnitFanout,
            thresh: 0.5,
            crit_only: false,
            print_flag: true,
            node_names: Vec::new(),
            notice: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintFanoutOptions {
    pub limit: i32,
    pub max_only: bool,
    pub thresh: f64,
    pub model: DelayModel,
    pub notice: Option<&'static str>,
}

impl Default for PrintFanoutOptions {
    fn default() -> Self {
        Self {
            limit: 5,
            max_only: false,
            thresh: 0.5,
            model: DelayModel::Mapped,
            notice: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportDelayDataOptions {
    pub explicit_model: Option<DelayModel>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferOptions {
    pub trace: bool,
    pub single_pass: bool,
    pub do_decomp: bool,
    pub debug: i32,
    pub limit: i32,
    pub mode: i32,
    pub thresh: f64,
    pub only_check_max_load: bool,
    pub interactive: bool,
}

impl Default for BufferOptions {
    fn default() -> Self {
        Self {
            trace: false,
            single_pass: false,
            do_decomp: false,
            debug: 0,
            limit: DEFAULT_BUFFER_LIMIT,
            mode: DEFAULT_BUFFER_MODE,
            thresh: 2.0 * V_SMALL,
            only_check_max_load: false,
            interactive: false,
        }
    }
}

impl BufferOptions {
    pub fn interactive_defaults() -> Self {
        Self {
            interactive: true,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferCommandPlan {
    pub options: BufferOptions,
    pub node_names: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeedupAlgorithmState {
    pub transforms: Vec<String>,
}

impl Default for SpeedupAlgorithmState {
    fn default() -> Self {
        Self {
            transforms: default_local_transforms(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeedupAlgorithmCommand {
    pub verbose: bool,
    pub no_options: bool,
    pub transforms: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedCommandKind {
    SpeedUp,
    SpeedupAlg,
    BufferOpt,
    PrintLevel,
    PrintFanout,
    ReportDelayData,
    PartCollapse,
    PrintCutset,
    PrintWeight,
    SpeedDelay,
    IsTwoInputAnd,
    SpeedPlot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: SpeedCommandKind,
    pub changes_network: bool,
}

pub const SPEED_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "speed_up",
        kind: SpeedCommandKind::SpeedUp,
        changes_network: true,
    },
    CommandRegistration {
        name: "speedup_alg",
        kind: SpeedCommandKind::SpeedupAlg,
        changes_network: false,
    },
    CommandRegistration {
        name: "buffer_opt",
        kind: SpeedCommandKind::BufferOpt,
        changes_network: true,
    },
    CommandRegistration {
        name: "print_level",
        kind: SpeedCommandKind::PrintLevel,
        changes_network: false,
    },
    CommandRegistration {
        name: "_print_fanout",
        kind: SpeedCommandKind::PrintFanout,
        changes_network: false,
    },
    CommandRegistration {
        name: "_report_delay_data",
        kind: SpeedCommandKind::ReportDelayData,
        changes_network: false,
    },
    CommandRegistration {
        name: "_part_collapse",
        kind: SpeedCommandKind::PartCollapse,
        changes_network: true,
    },
    CommandRegistration {
        name: "_print_cutset",
        kind: SpeedCommandKind::PrintCutset,
        changes_network: false,
    },
    CommandRegistration {
        name: "_print_weight",
        kind: SpeedCommandKind::PrintWeight,
        changes_network: false,
    },
    CommandRegistration {
        name: "_speed_delay",
        kind: SpeedCommandKind::SpeedDelay,
        changes_network: false,
    },
    CommandRegistration {
        name: "_is_2ip_and",
        kind: SpeedCommandKind::IsTwoInputAnd,
        changes_network: false,
    },
];

pub const SPEED_PLOT_COMMAND: CommandRegistration = CommandRegistration {
    name: "_speed_plot",
    kind: SpeedCommandKind::SpeedPlot,
    changes_network: false,
};

#[derive(Clone, Debug, PartialEq)]
pub enum SpeedCommand {
    SpeedUp(SpeedCommandPlan),
    SpeedupAlg(SpeedupAlgorithmCommand),
    BufferOpt(BufferCommandPlan),
    PrintLevel(PrintLevelOptions),
    PrintFanout(PrintFanoutOptions),
    ReportDelayData(ReportDelayDataOptions),
    PartCollapse(SpeedCommandPlan),
    PrintCutset(SpeedCommandPlan),
    PrintWeight(SpeedCommandPlan),
    SpeedDelay(SpeedCommandPlan),
    IsTwoInputAnd,
    SpeedPlot,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CommandParseError {
    MissingOptionValue(char),
    InvalidInteger { option: char, value: String },
    InvalidFloat { option: char, value: String },
    IllegalRegion(String),
    UnknownDelayModel(String),
    UnsupportedOption(String),
    UnexpectedOperand(String),
    InvalidBufferMode(i32),
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::InvalidInteger { option, value } => {
                write!(f, "invalid integer for -{option}: {value}")
            }
            Self::InvalidFloat { option, value } => {
                write!(f, "invalid float for -{option}: {value}")
            }
            Self::IllegalRegion(region) => write!(f, "illegal argument to the -s flag: {region}"),
            Self::UnknownDelayModel(model) => write!(f, "unknown delay model {model}"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::UnexpectedOperand(operand) => write!(f, "unexpected operand {operand}"),
            Self::InvalidBufferMode(mode) => {
                write!(f, "valid range of -f option is 1 to 7, got {mode}")
            }
        }
    }
}

impl Error for CommandParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedCommandError {
    MissingDependencies {
        command: SpeedCommandKind,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for SpeedCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDependencies {
                command,
                dependencies,
            } => {
                write!(
                    f,
                    "{command:?} requires native Rust ports for SIS dependencies: "
                )?;
                for (index, dependency) in dependencies.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} ({})", dependency.bead, dependency.source)?;
                }
                Ok(())
            }
        }
    }
}

impl Error for SpeedCommandError {}

pub fn required_ports() -> &'static [PortDependency] {
    REQUIRED_PORTS
}

pub fn speed_command_registrations(graphics_enabled: bool) -> Vec<CommandRegistration> {
    let mut commands = SPEED_COMMANDS.to_vec();
    if graphics_enabled {
        commands.push(SPEED_PLOT_COMMAND);
    }
    commands
}

pub fn speed_usage() -> &'static str {
    "usage: speed_up [-cinprvABIRT] [-a n] [-d n] [-w n] [-s method] [-t n.n] [-l n] [-D n] [-m model] [node-list]\n"
}

pub fn part_collapse_usage() -> &'static str {
    "usage: _part_collapse [-m model] [-d n] [-t n.n] list-of-nodes\n"
}

pub fn print_cutset_usage() -> &'static str {
    "usage: _print_cutset [-t n.n] [-d n] [-w n] [-m model]\n"
}

pub fn print_weight_usage() -> &'static str {
    "usage: print_weight [-m model] [-w n.n] [-d n] [-t n.n] list-of-nodes\n"
}

pub fn speed_delay_usage() -> &'static str {
    "Usage:  _speed_delay [-m model]\n"
}

pub fn print_level_usage() -> &'static str {
    "Usage:  print_level [-m model] [-t thresh] [-l] [-c] [node_list]\n"
}

pub fn print_fanout_usage() -> &'static str {
    "Usage: print_fanout [-s] [-t #.#] [-n #]\n"
}

pub fn buffer_usage() -> &'static str {
    "Usage: buffer_opt [-l #] [-cdTDL] [-f #] [-v #] [node_list]\n"
}

pub fn default_local_transforms() -> Vec<String> {
    Vec::new()
}

pub fn parse_speed_command_args<I, S>(args: I) -> Result<SpeedCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = SpeedOptions::interactive_defaults();
    let nodes = parse_options(args, "a:d:l:m:s:w:t:C:D:I:ABRTcnfipvr", |option, value| {
        apply_speed_option(&mut options, option, value)
    })?;

    Ok(SpeedCommandPlan {
        options,
        node_names: nodes,
    })
}

pub fn parse_part_collapse_args<I, S>(args: I) -> Result<SpeedCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_speed_command_args(args)
}

pub fn parse_print_cutset_args<I, S>(args: I) -> Result<SpeedCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_speed_command_args(args)
}

pub fn parse_print_weight_args<I, S>(args: I) -> Result<SpeedCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_speed_command_args(args)
}

pub fn parse_speed_delay_args<I, S>(args: I) -> Result<SpeedCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_speed_command_args(args)
}

pub fn parse_print_level_args<I, S>(args: I) -> Result<PrintLevelOptions, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = PrintLevelOptions::default();
    options.node_names = parse_options(args, "m:t:lc", |option, value| match option {
        'm' => {
            let model = DelayModel::from_c_name(&value)
                .ok_or_else(|| CommandParseError::UnknownDelayModel(value.clone()))?;
            let selection = model.print_command_model();
            options.model = selection.model;
            options.notice = selection.notice;
            Ok(())
        }
        't' => {
            options.thresh = parse_f64(option, &value)?;
            Ok(())
        }
        'l' => {
            options.print_flag = false;
            Ok(())
        }
        'c' => {
            options.crit_only = true;
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;
    if options.crit_only {
        options.print_flag = true;
    }
    Ok(options)
}

pub fn parse_print_fanout_args<I, S>(args: I) -> Result<PrintFanoutOptions, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = PrintFanoutOptions::default();
    let operands = parse_options(args, "sn:t:m:", |option, value| match option {
        'm' => {
            let model = DelayModel::from_c_name(&value)
                .ok_or_else(|| CommandParseError::UnknownDelayModel(value.clone()))?;
            let selection = model.print_command_model();
            options.model = selection.model;
            options.notice = selection.notice;
            Ok(())
        }
        's' => {
            options.max_only = true;
            Ok(())
        }
        't' => {
            options.thresh = parse_f64(option, &value)?;
            Ok(())
        }
        'n' => {
            options.limit = parse_i32(option, &value)?;
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;
    if let Some(operand) = operands.first() {
        return Err(CommandParseError::UnexpectedOperand(operand.clone()));
    }
    Ok(options)
}

pub fn parse_report_delay_data_args<I, S>(
    args: I,
) -> Result<ReportDelayDataOptions, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut explicit_model = None;
    let operands = parse_options(args, "m:", |option, value| match option {
        'm' => {
            explicit_model = Some(
                DelayModel::from_c_name(&value)
                    .ok_or_else(|| CommandParseError::UnknownDelayModel(value.clone()))?,
            );
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;
    if let Some(operand) = operands.first() {
        return Err(CommandParseError::UnexpectedOperand(operand.clone()));
    }
    Ok(ReportDelayDataOptions { explicit_model })
}

pub fn parse_buffer_args<I, S>(args: I) -> Result<BufferCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = BufferOptions::interactive_defaults();
    let nodes = parse_options(args, "cdLTDf:v:l:", |option, value| match option {
        'd' => {
            options.do_decomp = true;
            Ok(())
        }
        'c' => {
            options.single_pass = true;
            Ok(())
        }
        'L' => {
            options.only_check_max_load = true;
            Ok(())
        }
        'T' => {
            options.trace = true;
            Ok(())
        }
        'f' => {
            let mode = parse_i32(option, &value)?;
            if !(1..=7).contains(&mode) {
                return Err(CommandParseError::InvalidBufferMode(mode));
            }
            options.mode = mode;
            Ok(())
        }
        'v' => {
            options.debug = parse_i32(option, &value)?;
            Ok(())
        }
        'l' => {
            options.limit = parse_i32(option, &value)?;
            Ok(())
        }
        'D' => {
            options.debug = 1;
            Ok(())
        }
        _ => Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    })?;

    Ok(BufferCommandPlan {
        options,
        node_names: nodes,
    })
}

pub fn parse_speedup_alg_args<I, S>(
    args: I,
    state: &mut SpeedupAlgorithmState,
) -> Result<SpeedupAlgorithmCommand, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();
    let no_options = args.is_empty();
    let mut verbose = false;

    while args
        .first()
        .is_some_and(|arg| arg.starts_with('-') && arg != "-")
    {
        let arg = args.remove(0);
        if arg == "-v" {
            verbose = true;
        } else {
            return Err(CommandParseError::UnsupportedOption(arg));
        }
    }

    if !no_options {
        state.transforms = args.clone();
    }

    Ok(SpeedupAlgorithmCommand {
        verbose,
        no_options,
        transforms: state.transforms.clone(),
    })
}

pub fn parse_command<I, S>(
    command_name: &str,
    args: I,
    algorithm_state: &mut SpeedupAlgorithmState,
) -> Result<SpeedCommand, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name {
        "speed_up" => parse_speed_command_args(args).map(SpeedCommand::SpeedUp),
        "speedup_alg" => {
            parse_speedup_alg_args(args, algorithm_state).map(SpeedCommand::SpeedupAlg)
        }
        "buffer_opt" => parse_buffer_args(args).map(SpeedCommand::BufferOpt),
        "print_level" => parse_print_level_args(args).map(SpeedCommand::PrintLevel),
        "_print_fanout" => parse_print_fanout_args(args).map(SpeedCommand::PrintFanout),
        "_report_delay_data" => {
            parse_report_delay_data_args(args).map(SpeedCommand::ReportDelayData)
        }
        "_part_collapse" => parse_part_collapse_args(args).map(SpeedCommand::PartCollapse),
        "_print_cutset" => parse_print_cutset_args(args).map(SpeedCommand::PrintCutset),
        "_print_weight" => parse_print_weight_args(args).map(SpeedCommand::PrintWeight),
        "_speed_delay" => parse_speed_delay_args(args).map(SpeedCommand::SpeedDelay),
        "_is_2ip_and" => Ok(SpeedCommand::IsTwoInputAnd),
        "_speed_plot" => Ok(SpeedCommand::SpeedPlot),
        _ => Err(CommandParseError::UnsupportedOption(
            command_name.to_owned(),
        )),
    }
}

pub fn execute_command<Network>(
    _network: &mut Network,
    command: &SpeedCommand,
) -> Result<(), SpeedCommandError> {
    let kind = match command {
        SpeedCommand::SpeedUp(_) => SpeedCommandKind::SpeedUp,
        SpeedCommand::SpeedupAlg(_) => return Ok(()),
        SpeedCommand::BufferOpt(_) => SpeedCommandKind::BufferOpt,
        SpeedCommand::PrintLevel(_) => SpeedCommandKind::PrintLevel,
        SpeedCommand::PrintFanout(_) => SpeedCommandKind::PrintFanout,
        SpeedCommand::ReportDelayData(_) => SpeedCommandKind::ReportDelayData,
        SpeedCommand::PartCollapse(_) => SpeedCommandKind::PartCollapse,
        SpeedCommand::PrintCutset(_) => SpeedCommandKind::PrintCutset,
        SpeedCommand::PrintWeight(_) => SpeedCommandKind::PrintWeight,
        SpeedCommand::SpeedDelay(_) => SpeedCommandKind::SpeedDelay,
        SpeedCommand::IsTwoInputAnd => SpeedCommandKind::IsTwoInputAnd,
        SpeedCommand::SpeedPlot => SpeedCommandKind::SpeedPlot,
    };

    Err(SpeedCommandError::MissingDependencies {
        command: kind,
        dependencies: REQUIRED_PORTS,
    })
}

fn apply_speed_option(
    options: &mut SpeedOptions,
    option: char,
    value: String,
) -> Result<(), CommandParseError> {
    match option {
        'f' => options.new_mode = false,
        'p' => options.add_inv = true,
        'r' => options.area_reclaim = false,
        'c' => {
            options.speed_repeat = false;
            options.do_script = false;
        }
        'i' => {
            options.only_init_decomp = true;
            options.do_script = false;
        }
        'A' => options.del_crit_cubes = false,
        'B' => options.transform = TransformSelection::BestBangForBuck,
        'I' => options.timeout = parse_i32(option, &value)? as f64,
        'R' => options.red_removal = true,
        'n' => options.objective = SpeedObjective::TransformBased,
        'T' => options.trace = true,
        'v' => options.debug = 1,
        'D' => {
            options.debug = parse_i32(option, &value)?;
            if options.debug < 0 {
                options.debug = 0;
            }
        }
        's' => {
            options.region = SpeedRegion::from_c_name(&value)
                .ok_or_else(|| CommandParseError::IllegalRegion(value.clone()))?;
        }
        'w' => options.coeff = parse_f64(option, &value)?.clamp(0.0, 1.0),
        't' => {
            options.thresh = parse_f64(option, &value)?;
            options.do_script = false;
        }
        'a' => {
            let num_tries = parse_i32(option, &value)?;
            if num_tries > 0 {
                options.num_tries = num_tries;
            }
        }
        'C' => {
            let max_cuts = parse_i32(option, &value)?;
            if max_cuts > 0 {
                options.max_num_cuts = max_cuts;
            }
        }
        'l' => {
            let level = parse_i32(option, &value)?;
            if level > 0 {
                options.max_recur = level;
            }
        }
        'd' => {
            let dist = parse_i32(option, &value)?;
            if dist >= 0 {
                options.dist = dist;
            }
            options.do_script = false;
        }
        'm' => {
            let model = DelayModel::from_c_name(&value)
                .ok_or_else(|| CommandParseError::UnknownDelayModel(value.clone()))?;
            options.model = model.command_model(options.interactive).model;
        }
        _ => return Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
    }
    Ok(())
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

fn parse_i32(option: char, value: &str) -> Result<i32, CommandParseError> {
    value
        .parse()
        .map_err(|_| CommandParseError::InvalidInteger {
            option,
            value: value.to_owned(),
        })
}

fn parse_f64(option: char, value: &str) -> Result<f64, CommandParseError> {
    value.parse().map_err(|_| CommandParseError::InvalidFloat {
        option,
        value: value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_defaults_match_c_initializer() {
        let options = SpeedOptions::default();

        assert!(options.area_reclaim);
        assert!(!options.trace);
        assert!(!options.add_inv);
        assert!(options.del_crit_cubes);
        assert_eq!(options.num_tries, 1);
        assert_eq!(options.thresh, DEFAULT_SPEED_THRESH);
        assert_eq!(options.coeff, DEFAULT_SPEED_COEFF);
        assert_eq!(options.dist, DEFAULT_SPEED_DIST);
        assert!(options.new_mode);
        assert_eq!(options.region, SpeedRegion::AlongCriticalPath);
        assert_eq!(options.transform, TransformSelection::BestBenefit);
        assert_eq!(options.max_recur, 1);
        assert!(options.timeout.is_infinite());
        assert_eq!(options.max_num_cuts, DEFAULT_MAX_NUM_CUTS);
        assert!(!options.red_removal);
        assert_eq!(options.objective, SpeedObjective::AreaBased);
        assert!(options.do_script);
        assert!(options.speed_repeat);
        assert!(!options.only_init_decomp);
        assert_eq!(options.model, DelayModel::Unit);
        assert_eq!(options.pin_cap, 0.0);
        assert!(!options.library_acceleration);
    }

    #[test]
    fn parses_speed_flags_and_node_operands() {
        let plan = parse_speed_command_args([
            "-fcipABRnT",
            "-D",
            "-1",
            "-a2",
            "-C",
            "12",
            "-l3",
            "-d",
            "4",
            "-w",
            "1.5",
            "-t0.25",
            "-s",
            "tree",
            "-m",
            "library",
            "n1",
            "n2",
        ])
        .unwrap();

        assert!(!plan.options.new_mode);
        assert!(plan.options.add_inv);
        assert!(plan.options.only_init_decomp);
        assert!(!plan.options.do_script);
        assert!(!plan.options.del_crit_cubes);
        assert_eq!(plan.options.transform, TransformSelection::BestBangForBuck);
        assert!(plan.options.red_removal);
        assert_eq!(plan.options.objective, SpeedObjective::TransformBased);
        assert!(plan.options.trace);
        assert_eq!(plan.options.debug, 0);
        assert_eq!(plan.options.num_tries, 2);
        assert_eq!(plan.options.max_num_cuts, 12);
        assert_eq!(plan.options.max_recur, 3);
        assert_eq!(plan.options.dist, 4);
        assert_eq!(plan.options.coeff, 1.0);
        assert_eq!(plan.options.thresh, 0.25);
        assert_eq!(plan.options.region, SpeedRegion::OnlyTree);
        assert_eq!(plan.options.model, DelayModel::Mapped);
        assert_eq!(plan.node_names, vec!["n1", "n2"]);
    }

    #[test]
    fn speed_parser_rejects_bad_region_and_model() {
        assert_eq!(
            parse_speed_command_args(["-s", "wide"]).unwrap_err(),
            CommandParseError::IllegalRegion("wide".to_owned())
        );
        assert_eq!(
            parse_speed_command_args(["-m", "slow"]).unwrap_err(),
            CommandParseError::UnknownDelayModel("slow".to_owned())
        );
    }

    #[test]
    fn parses_print_level_summary_and_critical_flags() {
        let options = parse_print_level_args(["-lc", "-m", "library", "-t0.75", "out"]).unwrap();

        assert_eq!(options.model, DelayModel::Mapped);
        assert_eq!(
            options.notice,
            Some("Notice: Using the mapped model instead")
        );
        assert_eq!(options.thresh, 0.75);
        assert!(options.crit_only);
        assert!(options.print_flag);
        assert_eq!(options.node_names, vec!["out"]);
    }

    #[test]
    fn parses_print_fanout_options_and_rejects_operands() {
        let options = parse_print_fanout_args(["-s", "-n", "8", "-t0.1", "-m", "unit"]).unwrap();

        assert!(options.max_only);
        assert_eq!(options.limit, 8);
        assert_eq!(options.thresh, 0.1);
        assert_eq!(options.model, DelayModel::Unit);
        assert_eq!(
            parse_print_fanout_args(["node"]).unwrap_err(),
            CommandParseError::UnexpectedOperand("node".to_owned())
        );
    }

    #[test]
    fn buffer_defaults_and_option_validation_match_c() {
        let plan = parse_buffer_args(["-cdLTD", "-f", "3", "-v12", "-l", "9", "x"]).unwrap();

        assert!(plan.options.single_pass);
        assert!(plan.options.do_decomp);
        assert!(plan.options.only_check_max_load);
        assert!(plan.options.trace);
        assert_eq!(plan.options.debug, 12);
        assert_eq!(plan.options.mode, 3);
        assert_eq!(plan.options.limit, 9);
        assert_eq!(plan.node_names, vec!["x"]);
        assert_eq!(
            parse_buffer_args(["-f", "8"]).unwrap_err(),
            CommandParseError::InvalidBufferMode(8)
        );
    }

    #[test]
    fn speedup_alg_tracks_current_transform_list() {
        let mut state = SpeedupAlgorithmState::default();

        let first = parse_speedup_alg_args(["-v", "a", "b"], &mut state).unwrap();
        assert!(first.verbose);
        assert!(!first.no_options);
        assert_eq!(first.transforms, vec!["a", "b"]);

        let second = parse_speedup_alg_args(std::iter::empty::<&str>(), &mut state).unwrap();
        assert!(second.no_options);
        assert_eq!(second.transforms, vec!["a", "b"]);
    }

    #[test]
    fn command_registration_honors_graphics_flag() {
        let without_graphics = speed_command_registrations(false);
        let with_graphics = speed_command_registrations(true);

        assert_eq!(without_graphics.len(), SPEED_COMMANDS.len());
        assert!(
            !without_graphics
                .iter()
                .any(|command| command.name == "_speed_plot")
        );
        assert!(
            with_graphics
                .iter()
                .any(|command| command.kind == SpeedCommandKind::SpeedPlot)
        );
    }

    #[test]
    fn dispatch_reports_native_dependency_blockers() {
        let mut network = ();
        let command = SpeedCommand::SpeedUp(parse_speed_command_args(["-c"]).unwrap());
        let error = execute_command(&mut network, &command).unwrap_err();

        assert_eq!(
            error,
            SpeedCommandError::MissingDependencies {
                command: SpeedCommandKind::SpeedUp,
                dependencies: REQUIRED_PORTS,
            }
        );
        assert!(
            required_ports()
                .iter()
                .any(|dependency| dependency.bead == "LogicFriday1-8j8.2.6.475")
        );
        assert!(
            error
                .to_string()
                .contains("LogicSynthesis/sis/speed/speed_loop.c")
        );
    }

    #[test]
    fn speedup_alg_dispatch_is_native_and_does_not_require_network_ports() {
        let mut network = ();
        let command = SpeedCommand::SpeedupAlg(SpeedupAlgorithmCommand {
            verbose: true,
            no_options: false,
            transforms: vec!["x".to_owned()],
        });

        assert_eq!(execute_command(&mut network, &command), Ok(()));
    }
}
