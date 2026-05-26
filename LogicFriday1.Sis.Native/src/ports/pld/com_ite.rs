//! Native Rust command/options model for the SIS ITE PLD commands.
//!
//! The original C unit owns command parsing for `ite_map`, `ite_map_mroot`,
//! `_ite_mux`, and `_act_bool`, initializes ACT/ITE mapper parameters, installs
//! command registrations, and allocates per-node cost slots. This Rust port
//! keeps those deterministic pieces as owned data. The actual SIS network
//! mapping calls are represented by hook traits so callers can wire them to
//! native ports once those dependencies are available.

use std::error::Error;
use std::fmt;

pub const DEFAULT_ACT_ITE_ALPHA: i32 = 2;
pub const DEFAULT_ACT_ITE_GAMMA: i32 = 1;
pub const DEFAULT_MAX_OPTIMAL: i32 = 6;
pub const DEFAULT_FAC_TO_SOP_RATIO: f32 = 0.7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollapseUpdate {
    Inexpensive,
    Expensive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollapseMethod {
    Old,
    New,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecompMethod {
    UseGoodDecomp,
    UseFactor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapMethod {
    Old,
    New,
    WithIter,
    WithJustDecomp,
    Other(i32),
}

impl MapMethod {
    pub fn from_legacy_value(value: i32) -> Self {
        match value {
            0 => Self::Old,
            1 => Self::New,
            2 => Self::WithIter,
            3 => Self::WithJustDecomp,
            other => Self::Other(other),
        }
    }

    pub fn legacy_value(self) -> i32 {
        match self {
            Self::Old => 0,
            Self::New => 1,
            Self::WithIter => 2,
            Self::WithJustDecomp => 3,
            Self::Other(value) => value,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActInitParam {
    pub heuristic_num: i32,
    pub fanin_collapse: i32,
    pub collapse_fanins_of_fanout: i32,
    pub decomp_fanin: i32,
    pub num_iter: i32,
    pub cost_limit: i32,
    pub last_gasp: bool,
    pub map_alg: i32,
    pub lit_bound: i32,
    pub ite_fanin_limit_for_bdd: i32,
    pub collapse_update: CollapseUpdate,
    pub collapse_method: CollapseMethod,
    pub decomp_method: DecompMethod,
    pub alternate_rep: bool,
    pub map_method: MapMethod,
    pub var_selection_lit: i32,
    pub break_after_map: bool,
    pub mode: f64,
    pub gain_factor: f64,
}

impl Default for ActInitParam {
    fn default() -> Self {
        Self {
            heuristic_num: 0,
            fanin_collapse: 3,
            collapse_fanins_of_fanout: 15,
            decomp_fanin: 4,
            num_iter: 0,
            cost_limit: 3,
            last_gasp: false,
            map_alg: 1,
            lit_bound: 200,
            ite_fanin_limit_for_bdd: 40,
            collapse_update: CollapseUpdate::Inexpensive,
            collapse_method: CollapseMethod::Old,
            decomp_method: DecompMethod::UseGoodDecomp,
            alternate_rep: false,
            map_method: MapMethod::New,
            var_selection_lit: 15,
            break_after_map: false,
            mode: 0.0,
            gain_factor: 0.01,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IteCommandState {
    pub act_ite_alpha: i32,
    pub act_ite_gamma: i32,
    pub unate_select: bool,
    pub act_ite_debug: i32,
    pub act_ite_statistics: bool,
    pub use_fac_when_unate: bool,
    pub act_is_or_used: bool,
    pub max_optimal: i32,
}

impl Default for IteCommandState {
    fn default() -> Self {
        Self {
            act_ite_alpha: DEFAULT_ACT_ITE_ALPHA,
            act_ite_gamma: DEFAULT_ACT_ITE_GAMMA,
            unate_select: false,
            act_ite_debug: 0,
            act_ite_statistics: false,
            use_fac_when_unate: true,
            act_is_or_used: true,
            max_optimal: DEFAULT_MAX_OPTIMAL,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IteCommandKind {
    IteMap,
    IteMapMroot,
    IteMux,
    ActBool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IteCommandPlan {
    IteMap {
        state: IteCommandState,
        init_param: ActInitParam,
    },
    IteMapMroot {
        state: IteCommandState,
        init_param: ActInitParam,
    },
    IteMux {
        state: IteCommandState,
        init_param: ActInitParam,
        fac_to_sop_ratio: f32,
    },
    ActBool {
        map_alg: i32,
        print_flag: bool,
        act_is_or_used: bool,
        act_ite_debug: i32,
    },
}

impl IteCommandPlan {
    pub fn kind(&self) -> IteCommandKind {
        match self {
            Self::IteMap { .. } => IteCommandKind::IteMap,
            Self::IteMapMroot { .. } => IteCommandKind::IteMapMroot,
            Self::IteMux { .. } => IteCommandKind::IteMux,
            Self::ActBool { .. } => IteCommandKind::ActBool,
        }
    }
}

pub trait IteCommandHooks<Network> {
    fn map_network_with_iter(
        &mut self,
        network: &mut Network,
        init_param: &ActInitParam,
        state: &IteCommandState,
    ) -> IteCommandResult<()>;

    fn create_and_map_mroot_network(
        &mut self,
        network: &mut Network,
        init_param: &ActInitParam,
        state: &IteCommandState,
    ) -> IteCommandResult<()>;

    fn mux_network(
        &mut self,
        network: &mut Network,
        init_param: &ActInitParam,
        state: &IteCommandState,
        fac_to_sop_ratio: f32,
    ) -> IteCommandResult<()>;

    fn bool_map_network(
        &mut self,
        network: &mut Network,
        map_alg: i32,
        act_is_or_used: bool,
        print_flag: bool,
    ) -> IteCommandResult<()>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MissingIteCommandHooks;

impl<Network> IteCommandHooks<Network> for MissingIteCommandHooks {
    fn map_network_with_iter(
        &mut self,
        _network: &mut Network,
        _init_param: &ActInitParam,
        _state: &IteCommandState,
    ) -> IteCommandResult<()> {
        Err(IteCommandError::MissingNativePorts {
            operation: "ITE map network",
        })
    }

    fn create_and_map_mroot_network(
        &mut self,
        _network: &mut Network,
        _init_param: &ActInitParam,
        _state: &IteCommandState,
    ) -> IteCommandResult<()> {
        Err(IteCommandError::MissingNativePorts {
            operation: "ITE multiple-root map network",
        })
    }

    fn mux_network(
        &mut self,
        _network: &mut Network,
        _init_param: &ActInitParam,
        _state: &IteCommandState,
        _fac_to_sop_ratio: f32,
    ) -> IteCommandResult<()> {
        Err(IteCommandError::MissingNativePorts {
            operation: "ITE mux network",
        })
    }

    fn bool_map_network(
        &mut self,
        _network: &mut Network,
        _map_alg: i32,
        _act_is_or_used: bool,
        _print_flag: bool,
    ) -> IteCommandResult<()> {
        Err(IteCommandError::MissingNativePorts {
            operation: "ACT boolean map network",
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum IteCommandError {
    Parse(IteParseError),
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for IteCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for IteCommandError {}

impl From<IteParseError> for IteCommandError {
    fn from(value: IteParseError) -> Self {
        Self::Parse(value)
    }
}

pub type IteCommandResult<T> = Result<T, IteCommandError>;

#[derive(Clone, Debug, PartialEq)]
pub enum IteParseError {
    MissingOptionValue(char),
    InvalidInteger { option: char, value: String },
    InvalidFloat { option: char, value: String },
    UnsupportedHeuristic(i32),
    UnsupportedOption(String),
    UnexpectedOperand(String),
}

impl fmt::Display for IteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::InvalidInteger { option, value } => {
                write!(f, "invalid integer for -{option}: {value}")
            }
            Self::InvalidFloat { option, value } => {
                write!(f, "invalid floating point value for -{option}: {value}")
            }
            Self::UnsupportedHeuristic(value) => write!(f, "only -h 0 is supported, got {value}"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::UnexpectedOperand(operand) => write!(f, "unexpected operand {operand}"),
        }
    }
}

impl Error for IteParseError {}

#[derive(Clone, Debug, PartialEq)]
pub struct ActIteCostSlot<NodeId, IteId, ActId, MatchId, NetworkId> {
    pub node: NodeId,
    pub cost: i32,
    pub arrival_time: f64,
    pub required_time: f64,
    pub slack: f64,
    pub is_critical: bool,
    pub area_weight: f64,
    pub cost_and_arrival_time: f64,
    pub ite: Option<IteId>,
    pub will_ite: Option<IteId>,
    pub act: Option<ActId>,
    pub match_id: Option<MatchId>,
    pub network: Option<NetworkId>,
}

impl<NodeId, IteId, ActId, MatchId, NetworkId>
    ActIteCostSlot<NodeId, IteId, ActId, MatchId, NetworkId>
{
    pub fn new(node: NodeId) -> Self {
        Self {
            node,
            cost: 0,
            arrival_time: 0.0,
            required_time: 0.0,
            slack: 0.0,
            is_critical: false,
            area_weight: 0.0,
            cost_and_arrival_time: 0.0,
            ite: None,
            will_ite: None,
            act: None,
            match_id: None,
            network: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IteCommandRegistry {
    commands: Vec<RegisteredCommand>,
    daemons: Vec<RegisteredDaemon>,
}

impl IteCommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> &[RegisteredCommand] {
        &self.commands
    }

    pub fn daemons(&self) -> &[RegisteredDaemon] {
        &self.daemons
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredCommand {
    pub name: &'static str,
    pub changes_network: bool,
    pub kind: IteCommandKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegisteredDaemon {
    AllocActIteCost,
    FreeActIteCost,
}

pub fn init_ite(registry: &mut IteCommandRegistry) {
    registry.commands.push(RegisteredCommand {
        name: "ite_map",
        changes_network: true,
        kind: IteCommandKind::IteMap,
    });
    registry.commands.push(RegisteredCommand {
        name: "_ite_mux",
        changes_network: true,
        kind: IteCommandKind::IteMux,
    });
    registry.commands.push(RegisteredCommand {
        name: "_act_bool",
        changes_network: false,
        kind: IteCommandKind::ActBool,
    });
    registry.daemons.push(RegisteredDaemon::AllocActIteCost);
    registry.daemons.push(RegisteredDaemon::FreeActIteCost);
}

pub fn end_ite(_registry: &mut IteCommandRegistry) {}

pub fn act_ite_alloc<NodeId, IteId, ActId, MatchId, NetworkId>(
    node: NodeId,
) -> ActIteCostSlot<NodeId, IteId, ActId, MatchId, NetworkId> {
    ActIteCostSlot::new(node)
}

pub fn act_ite_free<NodeId, IteId, ActId, MatchId, NetworkId>(
    slot: &mut Option<ActIteCostSlot<NodeId, IteId, ActId, MatchId, NetworkId>>,
) -> bool {
    slot.take().is_some()
}

pub fn parse_ite_map_args<I, S>(args: I) -> Result<IteCommandPlan, IteParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_ite_mapping_command(args, IteMappingCommand::IteMap)
}

pub fn parse_ite_map_mroot_args<I, S>(args: I) -> Result<IteCommandPlan, IteParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_ite_mapping_command(args, IteMappingCommand::IteMapMroot)
}

pub fn parse_ite_mux_args<I, S>(args: I) -> Result<IteCommandPlan, IteParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_ite_mapping_command(args, IteMappingCommand::IteMux)
}

pub fn parse_act_bool_args<I, S>(args: I) -> Result<IteCommandPlan, IteParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut map_alg = 1;
    let mut print_flag = true;
    let mut act_is_or_used = true;
    let operands = parse_options(args, "cop", |option, _value| match option {
        'c' => {
            map_alg = 0;
            Ok(())
        }
        'o' => {
            act_is_or_used = false;
            Ok(())
        }
        'p' => {
            print_flag = false;
            Ok(())
        }
        _ => Err(IteParseError::UnsupportedOption(format!("-{option}"))),
    })?;

    reject_operands(operands)?;
    Ok(IteCommandPlan::ActBool {
        map_alg,
        print_flag,
        act_is_or_used,
        act_ite_debug: i32::from(print_flag),
    })
}

pub fn execute_ite_command<Network, Hooks>(
    network: &mut Network,
    plan: &IteCommandPlan,
    hooks: &mut Hooks,
) -> IteCommandResult<()>
where
    Hooks: IteCommandHooks<Network>,
{
    match plan {
        IteCommandPlan::IteMap { state, init_param } => {
            hooks.map_network_with_iter(network, init_param, state)
        }
        IteCommandPlan::IteMapMroot { state, init_param } => {
            hooks.create_and_map_mroot_network(network, init_param, state)
        }
        IteCommandPlan::IteMux {
            state,
            init_param,
            fac_to_sop_ratio,
        } => hooks.mux_network(network, init_param, state, *fac_to_sop_ratio),
        IteCommandPlan::ActBool {
            map_alg,
            print_flag,
            act_is_or_used,
            ..
        } => hooks.bool_map_network(network, *map_alg, *act_is_or_used, *print_flag),
    }
}

pub fn execute_ite_command_blocked<Network>(
    network: &mut Network,
    plan: &IteCommandPlan,
) -> IteCommandResult<()> {
    execute_ite_command(network, plan, &mut MissingIteCommandHooks)
}

pub fn ite_map_usage() -> &'static str {
    "usage: ite_map [-o] [-h HEURISTIC_NUM] [-d DECOMP_FANIN] [-c] [-f FANIN_COLLAPSE] [-F COLLAPSE_FANINS_OF_FANOUT] [-n NUM_ITER] [-s] [-L] [-N] [-a] [-U] [-v LEVEL] [-m MAP_METHOD] [-V LIT_COUNT] [-D] [-w]\n"
}

pub fn ite_map_mroot_usage() -> &'static str {
    "usage: ite_map_mroot [-o] [-h HEURISTIC_NUM] [-d DECOMP_FANIN] [-c] [-f FANIN_COLLAPSE] [-F COLLAPSE_FANINS_OF_FANOUT] [-n NUM_ITER] [-s] [-L] [-N] [-a] [-U] [-v LEVEL] [-m MAP_METHOD] [-V LIT_COUNT] [-D]\n"
}

pub fn ite_mux_usage() -> &'static str {
    "usage: ite_mux [-o] [-h HEURISTIC_NUM] [-d DECOMP_FANIN] [-c] [-f FANIN_COLLAPSE] [-F COLLAPSE_FANINS_OF_FANOUT] [-n NUM_ITER] [-s] [-L] [-N] [-a] [-U] [-v LEVEL] [-m MAP_METHOD] [-V LIT_COUNT] [-D] [-R FAC_TO_SOP_RATIO]\n"
}

pub fn act_bool_usage() -> &'static str {
    "usage: act_bool [-c] [-o] [-p]\n"
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IteMappingCommand {
    IteMap,
    IteMapMroot,
    IteMux,
}

fn parse_ite_mapping_command<I, S>(
    args: I,
    command: IteMappingCommand,
) -> Result<IteCommandPlan, IteParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut state = IteCommandState::default();
    let mut init_param = ActInitParam::default();
    let mut fac_to_sop_ratio = DEFAULT_FAC_TO_SOP_RATIO;
    let spec = match command {
        IteMappingCommand::IteMap => "A:C:F:G:M:V:b:d:f:h:l:m:n:v:DLNUacorsuw",
        IteMappingCommand::IteMapMroot => "A:C:F:G:M:V:b:d:f:h:l:m:n:v:DLNUacorsu",
        IteMappingCommand::IteMux => "A:C:F:G:M:R:V:b:d:f:h:l:m:n:v:DLNUacorsu",
    };

    let operands = parse_options(args, spec, |option, value| {
        apply_ite_mapping_option(
            command,
            &mut state,
            &mut init_param,
            &mut fac_to_sop_ratio,
            option,
            &value,
        )
    })?;
    reject_operands(operands)?;

    match command {
        IteMappingCommand::IteMap => Ok(IteCommandPlan::IteMap { state, init_param }),
        IteMappingCommand::IteMapMroot => Ok(IteCommandPlan::IteMapMroot { state, init_param }),
        IteMappingCommand::IteMux => Ok(IteCommandPlan::IteMux {
            state,
            init_param,
            fac_to_sop_ratio,
        }),
    }
}

fn apply_ite_mapping_option(
    command: IteMappingCommand,
    state: &mut IteCommandState,
    init_param: &mut ActInitParam,
    fac_to_sop_ratio: &mut f32,
    option: char,
    value: &str,
) -> Result<(), IteParseError> {
    match option {
        'A' => state.act_ite_alpha = parse_i32(option, value)?,
        'C' => init_param.cost_limit = parse_i32(option, value)?,
        'D' => init_param.decomp_method = DecompMethod::UseFactor,
        'F' => init_param.collapse_fanins_of_fanout = parse_i32(option, value)?,
        'G' => state.act_ite_gamma = parse_i32(option, value)?,
        'L' => init_param.last_gasp = true,
        'M' => state.max_optimal = parse_i32(option, value)?,
        'N' => init_param.collapse_method = CollapseMethod::New,
        'R' if command == IteMappingCommand::IteMux => {
            *fac_to_sop_ratio = parse_f32(option, value)?
        }
        'U' => init_param.collapse_update = CollapseUpdate::Expensive,
        'V' => init_param.var_selection_lit = parse_i32(option, value)?,
        'a' => init_param.alternate_rep = true,
        'b' => init_param.ite_fanin_limit_for_bdd = parse_i32(option, value)?,
        'c' => init_param.map_alg = 0,
        'd' => init_param.decomp_fanin = parse_i32(option, value)?,
        'f' => init_param.fanin_collapse = parse_i32(option, value)?,
        'h' => {
            init_param.heuristic_num = parse_i32(option, value)?;
            if init_param.heuristic_num != 0 {
                return Err(IteParseError::UnsupportedHeuristic(
                    init_param.heuristic_num,
                ));
            }
        }
        'l' => init_param.lit_bound = parse_i32(option, value)?,
        'm' => init_param.map_method = MapMethod::from_legacy_value(parse_i32(option, value)?),
        'n' => init_param.num_iter = parse_i32(option, value)?,
        'o' => state.act_is_or_used = false,
        'r' => init_param.break_after_map = true,
        's' => state.act_ite_statistics = true,
        'u' => state.use_fac_when_unate = false,
        'v' => state.act_ite_debug = parse_i32(option, value)?,
        'w' if command == IteMappingCommand::IteMap => state.unate_select = true,
        _ => return Err(IteParseError::UnsupportedOption(format!("-{option}"))),
    }

    Ok(())
}

fn parse_options<F>(
    args: impl IntoIterator<Item = impl AsRef<str>>,
    spec: &str,
    mut apply: F,
) -> Result<Vec<String>, IteParseError>
where
    F: FnMut(char, String) -> Result<(), IteParseError>,
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
                .ok_or_else(|| IteParseError::UnsupportedOption(format!("-{option}")))?;
            if needs_value {
                let value_start = offset + option.len_utf8();
                let value = if value_start < arg[1..].len() {
                    arg[1 + value_start..].to_owned()
                } else {
                    iter.next()
                        .ok_or(IteParseError::MissingOptionValue(option))?
                };
                apply(option, value)?;
                break;
            }

            apply(option, String::new())?;
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

fn reject_operands(operands: Vec<String>) -> Result<(), IteParseError> {
    if let Some(operand) = operands.first() {
        return Err(IteParseError::UnexpectedOperand(operand.clone()));
    }

    Ok(())
}

fn parse_i32(option: char, value: &str) -> Result<i32, IteParseError> {
    value.parse().map_err(|_| IteParseError::InvalidInteger {
        option,
        value: value.to_owned(),
    })
}

fn parse_f32(option: char, value: &str) -> Result<f32, IteParseError> {
    value.parse().map_err(|_| IteParseError::InvalidFloat {
        option,
        value: value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingHooks {
        calls: Vec<String>,
    }

    impl IteCommandHooks<i32> for RecordingHooks {
        fn map_network_with_iter(
            &mut self,
            network: &mut i32,
            init_param: &ActInitParam,
            state: &IteCommandState,
        ) -> IteCommandResult<()> {
            *network += init_param.cost_limit + state.act_ite_alpha;
            self.calls.push("map".to_owned());
            Ok(())
        }

        fn create_and_map_mroot_network(
            &mut self,
            network: &mut i32,
            init_param: &ActInitParam,
            _state: &IteCommandState,
        ) -> IteCommandResult<()> {
            *network += init_param.decomp_fanin;
            self.calls.push("mroot".to_owned());
            Ok(())
        }

        fn mux_network(
            &mut self,
            network: &mut i32,
            _init_param: &ActInitParam,
            _state: &IteCommandState,
            fac_to_sop_ratio: f32,
        ) -> IteCommandResult<()> {
            *network += (fac_to_sop_ratio * 10.0) as i32;
            self.calls.push("mux".to_owned());
            Ok(())
        }

        fn bool_map_network(
            &mut self,
            network: &mut i32,
            map_alg: i32,
            act_is_or_used: bool,
            print_flag: bool,
        ) -> IteCommandResult<()> {
            *network += map_alg + i32::from(act_is_or_used) + i32::from(print_flag);
            self.calls.push("bool".to_owned());
            Ok(())
        }
    }

    #[test]
    fn ite_map_defaults_match_c_command_initialization() {
        let plan = parse_ite_map_args(std::iter::empty::<&str>()).unwrap();
        let IteCommandPlan::IteMap { state, init_param } = plan else {
            panic!("expected ite map plan");
        };

        assert_eq!(state, IteCommandState::default());
        assert_eq!(init_param, ActInitParam::default());
        assert_eq!(init_param.map_method, MapMethod::New);
        assert_eq!(init_param.mode, 0.0);
        assert_eq!(init_param.gain_factor, 0.01);
    }

    #[test]
    fn ite_map_parses_every_legacy_switch_shape() {
        let plan = parse_ite_map_args([
            "-A4", "-C", "5", "-D", "-F8", "-G3", "-L", "-M", "9", "-N", "-U", "-V17", "-a",
            "-b41", "-c", "-d6", "-f7", "-h0", "-l250", "-m2", "-n3", "-o", "-r", "-s", "-u",
            "-v4", "-w",
        ])
        .unwrap();
        let IteCommandPlan::IteMap { state, init_param } = plan else {
            panic!("expected ite map plan");
        };

        assert_eq!(state.act_ite_alpha, 4);
        assert_eq!(state.act_ite_gamma, 3);
        assert_eq!(state.max_optimal, 9);
        assert!(state.unate_select);
        assert_eq!(state.act_ite_debug, 4);
        assert!(state.act_ite_statistics);
        assert!(!state.use_fac_when_unate);
        assert!(!state.act_is_or_used);
        assert_eq!(init_param.cost_limit, 5);
        assert_eq!(init_param.decomp_method, DecompMethod::UseFactor);
        assert_eq!(init_param.collapse_fanins_of_fanout, 8);
        assert!(init_param.last_gasp);
        assert_eq!(init_param.collapse_method, CollapseMethod::New);
        assert_eq!(init_param.collapse_update, CollapseUpdate::Expensive);
        assert_eq!(init_param.var_selection_lit, 17);
        assert!(init_param.alternate_rep);
        assert_eq!(init_param.ite_fanin_limit_for_bdd, 41);
        assert_eq!(init_param.map_alg, 0);
        assert_eq!(init_param.decomp_fanin, 6);
        assert_eq!(init_param.fanin_collapse, 7);
        assert_eq!(init_param.lit_bound, 250);
        assert_eq!(init_param.map_method, MapMethod::WithIter);
        assert_eq!(init_param.num_iter, 3);
        assert!(init_param.break_after_map);
    }

    #[test]
    fn mroot_rejects_unate_select_switch_that_only_ite_map_accepts() {
        assert_eq!(
            parse_ite_map_mroot_args(["-w"]).unwrap_err(),
            IteParseError::UnsupportedOption("-w".to_owned())
        );
    }

    #[test]
    fn mux_parses_ratio_and_rejects_bad_float() {
        let plan = parse_ite_mux_args(["-R", "0.5"]).unwrap();
        let IteCommandPlan::IteMux {
            fac_to_sop_ratio, ..
        } = plan
        else {
            panic!("expected mux plan");
        };

        assert_eq!(fac_to_sop_ratio, 0.5);
        assert_eq!(
            parse_ite_mux_args(["-Rnope"]).unwrap_err(),
            IteParseError::InvalidFloat {
                option: 'R',
                value: "nope".to_owned()
            }
        );
    }

    #[test]
    fn heuristic_one_is_reported_as_unsupported() {
        assert_eq!(
            parse_ite_map_args(["-h", "1"]).unwrap_err(),
            IteParseError::UnsupportedHeuristic(1)
        );
    }

    #[test]
    fn act_bool_parses_complete_match_no_or_and_no_print() {
        let plan = parse_act_bool_args(["-cop"]).unwrap();
        assert_eq!(
            plan,
            IteCommandPlan::ActBool {
                map_alg: 0,
                print_flag: false,
                act_is_or_used: false,
                act_ite_debug: 0,
            }
        );
    }

    #[test]
    fn execute_dispatches_to_matching_hook() {
        let mut network = 0;
        let mut hooks = RecordingHooks::default();

        execute_ite_command(
            &mut network,
            &parse_ite_map_args(["-A4", "-C5"]).unwrap(),
            &mut hooks,
        )
        .unwrap();
        execute_ite_command(
            &mut network,
            &parse_ite_map_mroot_args(["-d6"]).unwrap(),
            &mut hooks,
        )
        .unwrap();
        execute_ite_command(
            &mut network,
            &parse_ite_mux_args(["-R0.5"]).unwrap(),
            &mut hooks,
        )
        .unwrap();
        execute_ite_command(
            &mut network,
            &parse_act_bool_args(["-c"]).unwrap(),
            &mut hooks,
        )
        .unwrap();

        assert_eq!(hooks.calls, ["map", "mroot", "mux", "bool"]);
        assert_eq!(network, 4 + 5 + 6 + 5 + 0 + 1 + 1);
    }

    #[test]
    fn blocked_executor_returns_generic_runtime_diagnostic() {
        let mut network = ();
        let error =
            execute_ite_command_blocked(&mut network, &parse_ite_mux_args(["-R0.6"]).unwrap())
                .unwrap_err();

        assert_eq!(
            error,
            IteCommandError::MissingNativePorts {
                operation: "ITE mux network"
            }
        );
        assert_eq!(
            error.to_string(),
            "ITE mux network requires unavailable native SIS integration"
        );
    }

    #[test]
    fn init_registers_commands_and_daemons() {
        let mut registry = IteCommandRegistry::new();

        init_ite(&mut registry);
        end_ite(&mut registry);

        assert_eq!(
            registry.commands(),
            [
                RegisteredCommand {
                    name: "ite_map",
                    changes_network: true,
                    kind: IteCommandKind::IteMap,
                },
                RegisteredCommand {
                    name: "_ite_mux",
                    changes_network: true,
                    kind: IteCommandKind::IteMux,
                },
                RegisteredCommand {
                    name: "_act_bool",
                    changes_network: false,
                    kind: IteCommandKind::ActBool,
                },
            ]
        );
        assert_eq!(
            registry.daemons(),
            [
                RegisteredDaemon::AllocActIteCost,
                RegisteredDaemon::FreeActIteCost,
            ]
        );
    }

    #[test]
    fn cost_slot_allocation_and_free_match_c_defaults() {
        let mut slot = Some(act_ite_alloc::<_, usize, usize, usize, usize>(7usize));

        let allocated = slot.as_ref().unwrap();
        assert_eq!(allocated.node, 7);
        assert_eq!(allocated.cost, 0);
        assert_eq!(allocated.arrival_time, 0.0);
        assert_eq!(allocated.required_time, 0.0);
        assert_eq!(allocated.slack, 0.0);
        assert!(!allocated.is_critical);
        assert_eq!(allocated.area_weight, 0.0);
        assert_eq!(allocated.cost_and_arrival_time, 0.0);
        assert_eq!(allocated.ite, None);
        assert_eq!(allocated.will_ite, None);
        assert_eq!(allocated.act, None);
        assert_eq!(allocated.match_id, None);
        assert_eq!(allocated.network, None);

        assert!(act_ite_free(&mut slot));
        assert!(slot.is_none());
        assert!(!act_ite_free(&mut slot));
    }

    #[test]
    fn no_disallowed_metadata_or_legacy_abi_tokens_are_present() {
        let source = include_str!("com_ite.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
