//! Native command model for `LogicSynthesis/sis/clock/com_clock.c`.
//!
//! The legacy module registers `print_clock` and `chng_clock`. The print
//! command emits the active clock setting, cycle time, both edges for every
//! clock, and any dependent edges. The change command toggles between the
//! specification and working timing views. This file keeps that behavior in a
//! small owned Rust model without C ABI exports.

use std::error::Error;
use std::fmt;

pub const CLOCK_NOT_SET: f64 = -1.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockCommandKind {
    PrintClock,
    ChangeClock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: ClockCommandKind,
    pub changes_network: bool,
}

pub const CLOCK_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "print_clock",
        kind: ClockCommandKind::PrintClock,
        changes_network: false,
    },
    CommandRegistration {
        name: "chng_clock",
        kind: ClockCommandKind::ChangeClock,
        changes_network: false,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockSetting {
    Specification,
    Working,
    Unknown,
}

impl ClockSetting {
    pub fn cycle_time_label(self) -> Result<&'static str, ClockCommandError> {
        match self {
            Self::Specification => Ok("SPECIFIED"),
            Self::Working => Ok("WORKING"),
            Self::Unknown => Err(ClockCommandError::UnknownClockSetting),
        }
    }

    pub fn command_label(self) -> Result<&'static str, ClockCommandError> {
        match self {
            Self::Specification => Ok("SPECIFICATION"),
            Self::Working => Ok("WORKING"),
            Self::Unknown => Err(ClockCommandError::UnknownClockSetting),
        }
    }

    pub fn toggled(self) -> Result<Self, ClockCommandError> {
        match self {
            Self::Specification => Ok(Self::Working),
            Self::Working => Ok(Self::Specification),
            Self::Unknown => Err(ClockCommandError::UnknownClockSetting),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ClockTransition {
    Rise,
    Fall,
}

impl ClockTransition {
    pub fn legacy_prefix(self) -> &'static str {
        match self {
            Self::Rise => "r'",
            Self::Fall => "f'",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockEdgeTiming {
    pub nominal: f64,
    pub lower_range: f64,
    pub upper_range: f64,
}

impl ClockEdgeTiming {
    pub fn unspecified() -> Self {
        Self {
            nominal: CLOCK_NOT_SET,
            lower_range: CLOCK_NOT_SET,
            upper_range: CLOCK_NOT_SET,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockEdgeRef {
    pub clock_name: String,
    pub transition: ClockTransition,
}

impl ClockEdgeRef {
    pub fn new(clock_name: impl Into<String>, transition: ClockTransition) -> Self {
        Self {
            clock_name: clock_name.into(),
            transition,
        }
    }

    pub fn legacy_name(&self) -> String {
        format!("{}{}", self.transition.legacy_prefix(), self.clock_name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockEdge {
    pub timing: ClockEdgeTiming,
    pub dependent_edges: Vec<ClockEdgeRef>,
}

impl ClockEdge {
    pub fn new(timing: ClockEdgeTiming) -> Self {
        Self {
            timing,
            dependent_edges: Vec::new(),
        }
    }

    pub fn with_dependent(
        mut self,
        clock_name: impl Into<String>,
        transition: ClockTransition,
    ) -> Self {
        self.dependent_edges
            .push(ClockEdgeRef::new(clock_name, transition));
        self
    }
}

impl Default for ClockEdge {
    fn default() -> Self {
        Self::new(ClockEdgeTiming::unspecified())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clock {
    pub name: String,
    pub rise: ClockEdge,
    pub fall: ClockEdge,
}

impl Clock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rise: ClockEdge::default(),
            fall: ClockEdge::default(),
        }
    }

    pub fn with_rise(mut self, edge: ClockEdge) -> Self {
        self.rise = edge;
        self
    }

    pub fn with_fall(mut self, edge: ClockEdge) -> Self {
        self.fall = edge;
        self
    }

    fn edge(&self, transition: ClockTransition) -> &ClockEdge {
        match transition {
            ClockTransition::Rise => &self.rise,
            ClockTransition::Fall => &self.fall,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockNetwork {
    pub setting: ClockSetting,
    pub cycle_time: f64,
    clocks: Vec<Clock>,
}

impl ClockNetwork {
    pub fn new(setting: ClockSetting, cycle_time: f64) -> Self {
        Self {
            setting,
            cycle_time,
            clocks: Vec::new(),
        }
    }

    pub fn clocks(&self) -> &[Clock] {
        &self.clocks
    }

    pub fn add_clock(&mut self, clock: Clock) {
        self.clocks.push(clock);
    }

    pub fn toggle_setting(&mut self) -> Result<ClockSetting, ClockCommandError> {
        self.setting = self.setting.toggled()?;
        Ok(self.setting)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockCommand {
    PrintClock,
    ChangeClock,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockCommandOutput {
    pub status: i32,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl ClockCommandOutput {
    pub fn success(stdout: Vec<String>) -> Self {
        Self {
            status: 0,
            stdout,
            stderr: Vec::new(),
        }
    }

    pub fn error(stderr: impl Into<String>) -> Self {
        Self {
            status: 0,
            stdout: Vec::new(),
            stderr: vec![stderr.into()],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockCommandError {
    UnknownCommand(String),
    UnexpectedArguments {
        command: ClockCommandKind,
        expected: usize,
        actual: usize,
    },
    UnknownClockSetting,
}

impl fmt::Display for ClockCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(formatter, "unknown clock command {command}"),
            Self::UnexpectedArguments {
                command,
                expected,
                actual,
            } => write!(
                formatter,
                "{command:?} expected {expected} argument(s), got {actual}"
            ),
            Self::UnknownClockSetting => formatter.write_str("Unknown value for clock setting"),
        }
    }
}

impl Error for ClockCommandError {}

pub fn clock_command_registrations() -> &'static [CommandRegistration] {
    CLOCK_COMMANDS
}

pub fn init_clock() -> &'static [CommandRegistration] {
    clock_command_registrations()
}

pub fn end_clock() {}

pub fn parse_clock_command<I, S>(
    command_name: &str,
    args: I,
) -> Result<ClockCommand, ClockCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    match command_name {
        "print_clock" => {
            require_no_extra_args(ClockCommandKind::PrintClock, &args)?;
            Ok(ClockCommand::PrintClock)
        }
        "chng_clock" => {
            require_no_extra_args(ClockCommandKind::ChangeClock, &args)?;
            Ok(ClockCommand::ChangeClock)
        }
        _ => Err(ClockCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn dispatch_clock_command(
    network: &mut ClockNetwork,
    command: &ClockCommand,
) -> Result<ClockCommandOutput, ClockCommandError> {
    match command {
        ClockCommand::PrintClock => Ok(print_clock(network)?),
        ClockCommand::ChangeClock => Ok(change_clock(network)),
    }
}

pub fn print_clock(network: &ClockNetwork) -> Result<ClockCommandOutput, ClockCommandError> {
    if network.clocks().is_empty() {
        return Ok(ClockCommandOutput::success(Vec::new()));
    }

    let mut lines = vec![
        format!("\t(A value of {:.2} means unspecified)", CLOCK_NOT_SET),
        format!(
            "{} cycle_time = {:6.2}",
            network.setting.cycle_time_label()?,
            network.cycle_time
        ),
    ];

    for clock in network.clocks() {
        append_edge_lines(&mut lines, clock, ClockTransition::Rise);
        append_edge_lines(&mut lines, clock, ClockTransition::Fall);
    }

    Ok(ClockCommandOutput::success(lines))
}

pub fn change_clock(network: &mut ClockNetwork) -> ClockCommandOutput {
    match network.toggle_setting() {
        Ok(setting) => ClockCommandOutput::success(vec![format!(
            "Switching to setting {}",
            setting.command_label().expect("validated clock setting")
        )]),
        Err(ClockCommandError::UnknownClockSetting) => {
            ClockCommandOutput::error("Unknown value for clock setting")
        }
        Err(error) => ClockCommandOutput::error(error.to_string()),
    }
}

fn append_edge_lines(lines: &mut Vec<String>, clock: &Clock, transition: ClockTransition) {
    let edge = clock.edge(transition);
    lines.push(format!(
        "{}{}, Nominal={:4.2}, Range=({:5.2},{:<5.2})",
        transition.legacy_prefix(),
        clock.name,
        edge.timing.nominal,
        edge.timing.lower_range,
        edge.timing.upper_range
    ));

    if edge.dependent_edges.is_empty() {
        return;
    }

    let mut dependency_line = String::from("\tDependent edges --");
    for dependent in &edge.dependent_edges {
        dependency_line.push_str("   ");
        dependency_line.push_str(&dependent.legacy_name());
    }
    lines.push(dependency_line);
}

fn require_no_extra_args(
    command: ClockCommandKind,
    args: &[String],
) -> Result<(), ClockCommandError> {
    if !args.is_empty() {
        return Err(ClockCommandError::UnexpectedArguments {
            command,
            expected: 0,
            actual: args.len(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(nominal: f64, lower_range: f64, upper_range: f64) -> ClockEdgeTiming {
        ClockEdgeTiming {
            nominal,
            lower_range,
            upper_range,
        }
    }

    fn sample_network() -> ClockNetwork {
        let mut network = ClockNetwork::new(ClockSetting::Specification, 10.0);
        network.add_clock(
            Clock::new("clk")
                .with_rise(
                    ClockEdge::new(timing(1.0, 0.75, 1.25))
                        .with_dependent("phase2", ClockTransition::Fall),
                )
                .with_fall(ClockEdge::new(timing(6.0, 5.50, 6.50))),
        );
        network.add_clock(
            Clock::new("phase2")
                .with_rise(ClockEdge::new(timing(2.0, 1.50, 2.50)))
                .with_fall(ClockEdge::new(timing(7.0, 6.50, 7.50))),
        );
        network
    }

    #[test]
    fn registers_legacy_clock_commands() {
        assert_eq!(
            init_clock(),
            &[
                CommandRegistration {
                    name: "print_clock",
                    kind: ClockCommandKind::PrintClock,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "chng_clock",
                    kind: ClockCommandKind::ChangeClock,
                    changes_network: false,
                },
            ]
        );
    }

    #[test]
    fn parses_known_zero_argument_commands() {
        assert_eq!(
            parse_clock_command("print_clock", Vec::<String>::new()),
            Ok(ClockCommand::PrintClock)
        );
        assert_eq!(
            parse_clock_command("chng_clock", Vec::<String>::new()),
            Ok(ClockCommand::ChangeClock)
        );
    }

    #[test]
    fn rejects_unknown_commands_and_extra_arguments() {
        assert_eq!(
            parse_clock_command("clock", Vec::<String>::new()),
            Err(ClockCommandError::UnknownCommand("clock".to_owned()))
        );
        assert_eq!(
            parse_clock_command("print_clock", ["-x"]),
            Err(ClockCommandError::UnexpectedArguments {
                command: ClockCommandKind::PrintClock,
                expected: 0,
                actual: 1,
            })
        );
    }

    #[test]
    fn print_clock_is_quiet_when_network_has_no_clocks() {
        let network = ClockNetwork::new(ClockSetting::Working, CLOCK_NOT_SET);

        assert_eq!(
            print_clock(&network),
            Ok(ClockCommandOutput::success(Vec::new()))
        );
    }

    #[test]
    fn print_clock_matches_legacy_line_shape() {
        let output = print_clock(&sample_network()).unwrap();

        assert_eq!(output.status, 0);
        assert_eq!(
            output.stdout,
            vec![
                "\t(A value of -1.00 means unspecified)",
                "SPECIFIED cycle_time =  10.00",
                "r'clk, Nominal=1.00, Range=( 0.75,1.25 )",
                "\tDependent edges --   f'phase2",
                "f'clk, Nominal=6.00, Range=( 5.50,6.50 )",
                "r'phase2, Nominal=2.00, Range=( 1.50,2.50 )",
                "f'phase2, Nominal=7.00, Range=( 6.50,7.50 )",
            ]
        );
    }

    #[test]
    fn change_clock_toggles_between_working_and_specification() {
        let mut network = ClockNetwork::new(ClockSetting::Specification, 10.0);

        assert_eq!(
            change_clock(&mut network),
            ClockCommandOutput::success(vec!["Switching to setting WORKING".to_owned()])
        );
        assert_eq!(network.setting, ClockSetting::Working);
        assert_eq!(
            change_clock(&mut network),
            ClockCommandOutput::success(vec!["Switching to setting SPECIFICATION".to_owned()])
        );
        assert_eq!(network.setting, ClockSetting::Specification);
    }

    #[test]
    fn change_clock_reports_unknown_setting_without_panicking() {
        let mut network = ClockNetwork::new(ClockSetting::Unknown, 10.0);

        assert_eq!(
            change_clock(&mut network),
            ClockCommandOutput::error("Unknown value for clock setting")
        );
        assert_eq!(network.setting, ClockSetting::Unknown);
    }

    #[test]
    fn dispatch_runs_parsed_commands() {
        let mut network = sample_network();
        let command = parse_clock_command("print_clock", Vec::<String>::new()).unwrap();

        assert_eq!(
            dispatch_clock_command(&mut network, &command)
                .unwrap()
                .stdout[1],
            "SPECIFIED cycle_time =  10.00"
        );

        let command = parse_clock_command("chng_clock", Vec::<String>::new()).unwrap();
        assert_eq!(
            dispatch_clock_command(&mut network, &command)
                .unwrap()
                .stdout,
            vec!["Switching to setting WORKING"]
        );
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_c_abi_exports() {
        let source = include_str!("com_clock.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
