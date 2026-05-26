//! Native Rust scaffold for `LogicSynthesis/sis/timing/com_timing.c`.
//!
//! The C file provides SIS command entry points for `c_opt` and `c_check`,
//! plus timing package command registration. The graph construction,
//! verification, optimal-clock computation, clock settings, mapped-network
//! checks, and command registry live in other SIS modules that are still being
//! ported, so this module exposes the parsed command intent without recreating
//! per-file legacy C ABI exports.

pub const C_OPT_USAGE: &str = concat!(
    "Usage: c_opt -[nBI] -[dSHmM]# \n",
    "\t -d: Debug option\n",
    "\t -n: Use unmapped circuit\n",
    "\t   : unit delay with 0.2 per fanout\n",
    "\t -S: Set up time \n",
    "\t -H: Hold time \n",
    "\t -m: minimum separation between phases [0, 1)\n",
    "\t -M: Maximum separation between phases [m, 1)\n",
    "\t -B: Use binary search\n",
    "\t -I: 2 phase clock with phi and phibar\n",
    "defaults: no debug, mapped, S = 0, H = 0, m = 0\n",
    "        : M = 1, G = TRUE, I = FALSE\n",
);

pub const C_CHECK_USAGE: &str = concat!(
    "Usage: c_check -[dn] -[SH]# \n",
    "\t -d: Debug option\n",
    "\t -n: Use unmapped circuit\n",
    "\t   : unit delay with 0.2 per fanout\n",
    "\t -S: Set up time \n",
    "\t -H: Hold time \n",
    "defaults: no debug, mapped, S = 0, H = 0\n",
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebugType {
    LGraph,
    CGraph,
    BranchAndBound,
    General,
    None,
    Verify,
    All,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Library,
    UnitFanout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingCommandKind {
    OptimizeClock,
    CheckClock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimingCommandRegistration {
    pub name: &'static str,
    pub kind: TimingCommandKind,
    pub changes_network: bool,
}

pub const TIMING_COMMANDS: &[TimingCommandRegistration] = &[
    TimingCommandRegistration {
        name: "c_opt",
        kind: TimingCommandKind::OptimizeClock,
        changes_network: true,
    },
    TimingCommandRegistration {
        name: "c_check",
        kind: TimingCommandKind::CheckClock,
        changes_network: false,
    },
];

#[derive(Clone, Debug, PartialEq)]
pub struct ClockOptimizeOptions {
    pub debug: DebugType,
    pub model: DelayModel,
    pub set_up: f64,
    pub hold: f64,
    pub min_sep: f64,
    pub max_sep: f64,
    pub use_general_algorithm: bool,
    pub inverted_second_phase: bool,
}

impl Default for ClockOptimizeOptions {
    fn default() -> Self {
        Self {
            debug: DebugType::None,
            model: DelayModel::Library,
            set_up: 0.0,
            hold: 0.0,
            min_sep: 0.0,
            max_sep: 1.0,
            use_general_algorithm: true,
            inverted_second_phase: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockCheckOptions {
    pub debug: DebugType,
    pub model: DelayModel,
    pub set_up: f64,
    pub hold: f64,
}

impl Default for ClockCheckOptions {
    fn default() -> Self {
        Self {
            debug: DebugType::None,
            model: DelayModel::Library,
            set_up: 0.0,
            hold: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimingCommandError {
    MissingOptionValue(char),
    InvalidNumber { option: char, value: String },
    InvalidOptimizeRange,
    UnsupportedOption(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingPortDisposition {
    BlockedByUnportedTimingAndCommandApis,
}

pub fn timing_port_disposition() -> TimingPortDisposition {
    TimingPortDisposition::BlockedByUnportedTimingAndCommandApis
}

pub fn timing_port_is_blocked() -> bool {
    timing_port_disposition() == TimingPortDisposition::BlockedByUnportedTimingAndCommandApis
}

pub fn timing_command_registrations() -> &'static [TimingCommandRegistration] {
    TIMING_COMMANDS
}

pub fn parse_clock_optimize_args<I, S>(args: I) -> Result<ClockOptimizeOptions, TimingCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = ClockOptimizeOptions::default();
    let mut iter = args.into_iter().map(|arg| arg.as_ref().to_owned());

    while let Some(arg) = iter.next() {
        if arg == "-n" {
            options.model = DelayModel::UnitFanout;
        } else if arg == "-B" {
            options.use_general_algorithm = false;
        } else if arg == "-I" {
            options.inverted_second_phase = true;
        } else if let Some(value) = option_value(&arg, 'd', &mut iter)? {
            options.debug = optimize_debug_from_c_value(parse_i32('d', &value)?);
        } else if let Some(value) = option_value(&arg, 'S', &mut iter)? {
            options.set_up = parse_f64('S', &value)?;
        } else if let Some(value) = option_value(&arg, 'H', &mut iter)? {
            options.hold = parse_f64('H', &value)?;
        } else if let Some(value) = option_value(&arg, 'm', &mut iter)? {
            options.min_sep = parse_f64('m', &value)?;
            if options.min_sep > 1.0 {
                return Err(TimingCommandError::InvalidOptimizeRange);
            }
        } else if let Some(value) = option_value(&arg, 'M', &mut iter)? {
            options.max_sep = parse_f64('M', &value)?;
            if options.max_sep < options.min_sep || options.max_sep > 1.0 {
                return Err(TimingCommandError::InvalidOptimizeRange);
            }
        } else {
            return Err(TimingCommandError::UnsupportedOption(arg));
        }
    }

    Ok(options)
}

pub fn parse_clock_check_args<I, S>(args: I) -> Result<ClockCheckOptions, TimingCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = ClockCheckOptions::default();
    let mut iter = args.into_iter().map(|arg| arg.as_ref().to_owned());

    while let Some(arg) = iter.next() {
        if arg == "-n" {
            options.model = DelayModel::UnitFanout;
        } else if let Some(value) = option_value(&arg, 'd', &mut iter)? {
            options.debug = check_debug_from_c_value(parse_i32('d', &value)?);
        } else if let Some(value) = option_value(&arg, 'S', &mut iter)? {
            options.set_up = parse_f64('S', &value)?;
        } else if let Some(value) = option_value(&arg, 'H', &mut iter)? {
            options.hold = parse_f64('H', &value)?;
        } else {
            return Err(TimingCommandError::UnsupportedOption(arg));
        }
    }

    Ok(options)
}

fn option_value<I>(
    arg: &str,
    option: char,
    iter: &mut I,
) -> Result<Option<String>, TimingCommandError>
where
    I: Iterator<Item = String>,
{
    let short = format!("-{option}");
    if arg == short {
        return iter
            .next()
            .map(Some)
            .ok_or(TimingCommandError::MissingOptionValue(option));
    }

    Ok(arg
        .strip_prefix(&short)
        .filter(|value| !value.is_empty())
        .map(str::to_owned))
}

fn parse_i32(option: char, value: &str) -> Result<i32, TimingCommandError> {
    value
        .parse()
        .map_err(|_| TimingCommandError::InvalidNumber {
            option,
            value: value.to_owned(),
        })
}

fn parse_f64(option: char, value: &str) -> Result<f64, TimingCommandError> {
    value
        .parse()
        .map_err(|_| TimingCommandError::InvalidNumber {
            option,
            value: value.to_owned(),
        })
}

fn optimize_debug_from_c_value(value: i32) -> DebugType {
    match value {
        0 => DebugType::All,
        1 => DebugType::LGraph,
        2 => DebugType::CGraph,
        3 => DebugType::BranchAndBound,
        4 => DebugType::General,
        _ => DebugType::None,
    }
}

fn check_debug_from_c_value(value: i32) -> DebugType {
    match value {
        0 => DebugType::All,
        1 => DebugType::LGraph,
        5 => DebugType::Verify,
        _ => DebugType::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clock_optimize_defaults() {
        assert_eq!(
            parse_clock_optimize_args(["-d", "0"]).unwrap().debug,
            DebugType::All
        );
        assert_eq!(
            parse_clock_optimize_args(std::iter::empty::<&str>()).unwrap(),
            ClockOptimizeOptions::default()
        );
    }

    #[test]
    fn parses_clock_optimize_flags_and_values() {
        let options = parse_clock_optimize_args([
            "-n", "-S1.5", "-H", "0.2", "-m", "0.1", "-M0.9", "-B", "-I", "-d3",
        ])
        .unwrap();

        assert_eq!(options.debug, DebugType::BranchAndBound);
        assert_eq!(options.model, DelayModel::UnitFanout);
        assert_eq!(options.set_up, 1.5);
        assert_eq!(options.hold, 0.2);
        assert_eq!(options.min_sep, 0.1);
        assert_eq!(options.max_sep, 0.9);
        assert!(!options.use_general_algorithm);
        assert!(options.inverted_second_phase);
    }

    #[test]
    fn rejects_clock_optimize_bad_ranges() {
        assert_eq!(
            parse_clock_optimize_args(["-m", "1.1"]),
            Err(TimingCommandError::InvalidOptimizeRange)
        );
        assert_eq!(
            parse_clock_optimize_args(["-m", "0.7", "-M", "0.6"]),
            Err(TimingCommandError::InvalidOptimizeRange)
        );
    }

    #[test]
    fn parses_clock_check_options() {
        let options = parse_clock_check_args(["-d5", "-n", "-S", "1.0", "-H0.3"]).unwrap();

        assert_eq!(options.debug, DebugType::Verify);
        assert_eq!(options.model, DelayModel::UnitFanout);
        assert_eq!(options.set_up, 1.0);
        assert_eq!(options.hold, 0.3);
    }

    #[test]
    fn reports_usage_errors_and_dependencies() {
        assert_eq!(
            parse_clock_check_args(["-S"]),
            Err(TimingCommandError::MissingOptionValue('S'))
        );
        assert_eq!(
            parse_clock_check_args(["-S", "soon"]),
            Err(TimingCommandError::InvalidNumber {
                option: 'S',
                value: "soon".to_owned()
            })
        );
        assert_eq!(timing_command_registrations().len(), 2);
        assert!(timing_port_is_blocked());
        assert!(timing_port_is_blocked());
    }
}
