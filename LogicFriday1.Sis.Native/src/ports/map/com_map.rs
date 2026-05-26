//! Native Rust command/options model for `LogicSynthesis/sis/map/com_map.c`.
//!
//! The SIS C command registers `map`, parses mapper flags, checks that a
//! technology library has been loaded, and then dispatches into the mapper.
//! The network/library mapper is still represented by other native porting
//! beads, so this module intentionally ports the deterministic command model
//! only. It exposes no legacy C ABI entry points.

use std::error::Error;
use std::fmt;

pub const DEFAULT_MAP_MODE: MapCostMode = MapCostMode::Area;
pub const DEFAULT_FANOUT_HANDLING: FanoutHandling = FanoutHandling::BranchesAndInternalCells;
pub const DEFAULT_LOAD_LIMIT_MODE: LoadLimitMode = LoadLimitMode::Enforce;
pub const DEFAULT_LOAD_PENALTY: i32 = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapCostMode {
    Area,
    Delay,
    CriticalPathDelay,
    AreaDelayTradeoff(i32),
}

impl MapCostMode {
    pub fn from_legacy_value(value: i32) -> Self {
        match value {
            0 => Self::Area,
            1 => Self::Delay,
            2 => Self::CriticalPathDelay,
            value => Self::AreaDelayTradeoff(value),
        }
    }

    pub fn legacy_value(self) -> i32 {
        match self {
            Self::Area => 0,
            Self::Delay => 1,
            Self::CriticalPathDelay => 2,
            Self::AreaDelayTradeoff(value) => value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutHandling {
    Disabled,
    BranchCostHeuristic,
    InternalFanoutCells,
    BranchesAndInternalCells,
}

impl FanoutHandling {
    pub fn from_legacy_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Disabled),
            1 => Some(Self::BranchCostHeuristic),
            2 => Some(Self::InternalFanoutCells),
            3 => Some(Self::BranchesAndInternalCells),
            _ => None,
        }
    }

    pub fn legacy_value(self) -> i32 {
        match self {
            Self::Disabled => 0,
            Self::BranchCostHeuristic => 1,
            Self::InternalFanoutCells => 2,
            Self::BranchesAndInternalCells => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadLimitMode {
    Ignore,
    Enforce,
}

impl LoadLimitMode {
    pub fn from_legacy_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Ignore),
            1 => Some(Self::Enforce),
            _ => None,
        }
    }

    pub fn legacy_value(self) -> i32 {
        match self {
            Self::Ignore => 0,
            Self::Enforce => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreeCovering {
    Simple,
    LoadSensitiveDelay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryAvailability {
    Library,
    NoLibrary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapOptions {
    pub cost_mode: MapCostMode,
    pub tree_covering: TreeCovering,
    pub fanout_handling: FanoutHandling,
    pub disable_inverter_at_branch: bool,
    pub raw_mode: bool,
    pub print_statistics: bool,
    pub ignore_delay_constraints: bool,
    pub verbosity: i32,
    pub recover_area_with_buffer_resize: bool,
    pub recover_area_with_gate_resize: bool,
    pub fanout_optimization: bool,
    pub load_limit_mode: LoadLimitMode,
    pub load_penalty: i32,
    pub suppress_warnings: bool,
}

impl Default for MapOptions {
    fn default() -> Self {
        Self {
            cost_mode: DEFAULT_MAP_MODE,
            tree_covering: TreeCovering::Simple,
            fanout_handling: DEFAULT_FANOUT_HANDLING,
            disable_inverter_at_branch: false,
            raw_mode: false,
            print_statistics: false,
            ignore_delay_constraints: false,
            verbosity: 0,
            recover_area_with_buffer_resize: false,
            recover_area_with_gate_resize: false,
            fanout_optimization: false,
            load_limit_mode: DEFAULT_LOAD_LIMIT_MODE,
            load_penalty: DEFAULT_LOAD_PENALTY,
            suppress_warnings: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapCommandPlan {
    pub options: MapOptions,
    pub library: LibraryAvailability,
}

impl MapCommandPlan {
    pub fn new(options: MapOptions, library: LibraryAvailability) -> Self {
        Self { options, library }
    }

    pub fn validate_library(&self) -> Result<(), MapCommandError> {
        match self.library {
            LibraryAvailability::Library => Ok(()),
            LibraryAvailability::NoLibrary => Err(MapCommandError::MissingLibrary),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapParseError {
    MissingOptionValue(char),
    InvalidInteger { option: char, value: String },
    InvalidFanoutHandling(i32),
    InvalidLoadLimitMode(i32),
    InvalidTreeCovering(i32),
    UnsupportedOption(String),
    UnexpectedOperand(String),
}

impl fmt::Display for MapParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::InvalidInteger { option, value } => {
                write!(f, "invalid integer for -{option}: {value}")
            }
            Self::InvalidFanoutHandling(value) => {
                write!(f, "valid range of -f option is 0 to 3, got {value}")
            }
            Self::InvalidLoadLimitMode(value) => {
                write!(f, "valid range of -B option is 0 to 1, got {value}")
            }
            Self::InvalidTreeCovering(value) => {
                write!(
                    f,
                    "-n only accepts 1 for load-sensitive delay mapping, got {value}"
                )
            }
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::UnexpectedOperand(operand) => write!(f, "unexpected operand {operand}"),
        }
    }
}

impl Error for MapParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapCommandError {
    MissingLibrary,
    MissingDependencies,
}

impl fmt::Display for MapCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLibrary => {
                write!(f, "map requires a technology library loaded by read_library")
            }
            Self::MissingDependencies => {
                write!(f, "map requires unavailable native SIS mapper integration")
            }
        }
    }
}

impl Error for MapCommandError {}

pub fn map_usage() -> &'static str {
    "usage: map [-b #][-f #][-i][-m #][-n #][-r][-s][-p][-v #] [-A][-B #][-F][-G][-W]\n"
}

pub fn parse_map_args<I, S>(args: I) -> Result<MapOptions, MapParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = MapOptions::default();
    let operands = parse_options(args, "b:f:im:n:rspv:AB:FGW", |option, value| {
        apply_map_option(&mut options, option, value)
    })?;

    if let Some(operand) = operands.first() {
        return Err(MapParseError::UnexpectedOperand(operand.clone()));
    }

    Ok(options)
}

pub fn parse_map_command<I, S>(
    args: I,
    library: LibraryAvailability,
) -> Result<MapCommandPlan, MapParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    Ok(MapCommandPlan::new(parse_map_args(args)?, library))
}

pub fn execute_map_command<Network>(
    _network: &mut Network,
    plan: &MapCommandPlan,
) -> Result<(), MapCommandError> {
    plan.validate_library()?;
    Err(MapCommandError::MissingDependencies)
}

fn apply_map_option(
    options: &mut MapOptions,
    option: char,
    value: String,
) -> Result<(), MapParseError> {
    match option {
        'b' => {
            options.load_penalty = parse_i32(option, &value)?;
            Ok(())
        }
        'f' => {
            let fanout = parse_i32(option, &value)?;
            options.fanout_handling = FanoutHandling::from_legacy_value(fanout)
                .ok_or(MapParseError::InvalidFanoutHandling(fanout))?;
            Ok(())
        }
        'i' => {
            options.disable_inverter_at_branch = true;
            Ok(())
        }
        'm' => {
            options.cost_mode = MapCostMode::from_legacy_value(parse_i32(option, &value)?);
            options.tree_covering = TreeCovering::Simple;
            Ok(())
        }
        'n' => {
            let mode = parse_i32(option, &value)?;
            if mode != 1 {
                return Err(MapParseError::InvalidTreeCovering(mode));
            }
            options.cost_mode = MapCostMode::Delay;
            options.tree_covering = TreeCovering::LoadSensitiveDelay;
            Ok(())
        }
        'r' => {
            options.raw_mode = true;
            Ok(())
        }
        's' => {
            options.print_statistics = true;
            Ok(())
        }
        'p' => {
            options.ignore_delay_constraints = true;
            Ok(())
        }
        'v' => {
            options.verbosity = parse_i32(option, &value)?;
            Ok(())
        }
        'A' => {
            options.recover_area_with_buffer_resize = true;
            Ok(())
        }
        'B' => {
            let mode = parse_i32(option, &value)?;
            options.load_limit_mode = LoadLimitMode::from_legacy_value(mode)
                .ok_or(MapParseError::InvalidLoadLimitMode(mode))?;
            Ok(())
        }
        'F' => {
            options.fanout_optimization = true;
            options.fanout_handling = FanoutHandling::Disabled;
            Ok(())
        }
        'G' => {
            options.recover_area_with_gate_resize = true;
            Ok(())
        }
        'W' => {
            options.suppress_warnings = true;
            Ok(())
        }
        _ => Err(MapParseError::UnsupportedOption(format!("-{option}"))),
    }
}

fn parse_options<F>(
    args: impl IntoIterator<Item = impl AsRef<str>>,
    spec: &str,
    mut apply: F,
) -> Result<Vec<String>, MapParseError>
where
    F: FnMut(char, String) -> Result<(), MapParseError>,
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
                .ok_or_else(|| MapParseError::UnsupportedOption(format!("-{option}")))?;
            if needs_value {
                let value_start = offset + option.len_utf8();
                let value = if value_start < arg[1..].len() {
                    arg[1 + value_start..].to_owned()
                } else {
                    iter.next()
                        .ok_or(MapParseError::MissingOptionValue(option))?
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

fn parse_i32(option: char, value: &str) -> Result<i32, MapParseError> {
    value.parse().map_err(|_| MapParseError::InvalidInteger {
        option,
        value: value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_defaults_match_documented_command_defaults() {
        let options = parse_map_args(std::iter::empty::<&str>()).unwrap();

        assert_eq!(options.cost_mode, MapCostMode::Area);
        assert_eq!(options.tree_covering, TreeCovering::Simple);
        assert_eq!(
            options.fanout_handling,
            FanoutHandling::BranchesAndInternalCells
        );
        assert_eq!(options.load_limit_mode, LoadLimitMode::Enforce);
        assert_eq!(options.load_penalty, DEFAULT_LOAD_PENALTY);
        assert!(!options.fanout_optimization);
        assert!(!options.recover_area_with_buffer_resize);
        assert!(!options.recover_area_with_gate_resize);
        assert_eq!(options.verbosity, 0);
    }

    #[test]
    fn parses_delay_mode_with_attached_value() {
        let options = parse_map_args(["-m1"]).unwrap();

        assert_eq!(options.cost_mode, MapCostMode::Delay);
        assert_eq!(options.tree_covering, TreeCovering::Simple);
    }

    #[test]
    fn parses_default_fanout_mode_with_attached_value() {
        let options = parse_map_args(["-f3"]).unwrap();

        assert_eq!(
            options.fanout_handling,
            FanoutHandling::BranchesAndInternalCells
        );
        assert_eq!(options.fanout_handling.legacy_value(), 3);
    }

    #[test]
    fn fanout_optimization_disables_internal_fanout_handling() {
        let options = parse_map_args(["-f3", "-AFG", "-B0", "-b", "2500"]).unwrap();

        assert!(options.fanout_optimization);
        assert!(options.recover_area_with_buffer_resize);
        assert!(options.recover_area_with_gate_resize);
        assert_eq!(options.fanout_handling, FanoutHandling::Disabled);
        assert_eq!(options.load_limit_mode, LoadLimitMode::Ignore);
        assert_eq!(options.load_penalty, 2_500);
    }

    #[test]
    fn parses_load_sensitive_delay_mapping() {
        let options = parse_map_args(["-n", "1", "-p", "-r", "-s", "-v2", "-i", "-W"]).unwrap();

        assert_eq!(options.cost_mode, MapCostMode::Delay);
        assert_eq!(options.tree_covering, TreeCovering::LoadSensitiveDelay);
        assert!(options.ignore_delay_constraints);
        assert!(options.raw_mode);
        assert!(options.print_statistics);
        assert!(options.disable_inverter_at_branch);
        assert!(options.suppress_warnings);
        assert_eq!(options.verbosity, 2);
    }

    #[test]
    fn validates_library_and_no_library_command_states() {
        let with_library = parse_map_command(["-m0"], LibraryAvailability::Library).unwrap();
        assert_eq!(with_library.validate_library(), Ok(()));

        let without_library = parse_map_command(["-m0"], LibraryAvailability::NoLibrary).unwrap();
        assert_eq!(
            without_library.validate_library(),
            Err(MapCommandError::MissingLibrary)
        );
    }

    #[test]
    fn rejects_invalid_values_and_operands() {
        assert_eq!(
            parse_map_args(["-f4"]).unwrap_err(),
            MapParseError::InvalidFanoutHandling(4)
        );
        assert_eq!(
            parse_map_args(["-B2"]).unwrap_err(),
            MapParseError::InvalidLoadLimitMode(2)
        );
        assert_eq!(
            parse_map_args(["-n0"]).unwrap_err(),
            MapParseError::InvalidTreeCovering(0)
        );
        assert_eq!(
            parse_map_args(["node"]).unwrap_err(),
            MapParseError::UnexpectedOperand("node".to_owned())
        );
    }
}
