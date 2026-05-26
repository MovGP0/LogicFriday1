//! Native Rust command model for ASTG speed-independent synthesis commands.
//!
//! This module keeps the legacy command behavior testable without exporting C
//! ABI symbols. The synthesis and graph operations are represented by a Rust
//! backend trait so native ASTG ports can plug in as they become available.

use std::error::Error;
use std::fmt;

pub const ASTG_SYN_USAGE: &str = concat!(
    "usage: astg_syn [-m] [-r] [-v debug_level] [-x] \n",
    "       -m   : don't remove MIC/MOC-related hazards\n",
    "       -r   : don't add redundancy (run espresso)\n",
    "       -v debug_level : print debug info\n",
    "       -x   : skip token flow and synthesize direclty\n",
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgCommandKind
{
    Synthesize,
    PrintStateGraph,
    PrintStatistics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration
{
    pub name: &'static str,
    pub kind: AstgCommandKind,
    pub changes_network: bool,
}

pub const ASTG_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration
    {
        name: "astg_syn",
        kind: AstgCommandKind::Synthesize,
        changes_network: true,
    },
    CommandRegistration
    {
        name: "astg_print_sg",
        kind: AstgCommandKind::PrintStateGraph,
        changes_network: false,
    },
    CommandRegistration
    {
        name: "astg_print_stat",
        kind: AstgCommandKind::PrintStatistics,
        changes_network: false,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AstgSynOptions
{
    pub remove_mic_moc_hazards: bool,
    pub add_redundancy: bool,
    pub debug_level: i32,
    pub skip_token_flow: bool,
}

impl Default for AstgSynOptions
{
    fn default() -> Self
    {
        Self
        {
            remove_mic_moc_hazards: true,
            add_redundancy: true,
            debug_level: 0,
            skip_token_flow: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgCommand
{
    Synthesize(AstgSynOptions),
    PrintStateGraph,
    PrintStatistics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgCommandOutput
{
    pub status: i32,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl AstgCommandOutput
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

    pub fn failure(stderr: impl Into<String>) -> Self
    {
        Self
        {
            status: 1,
            stdout: Vec::new(),
            stderr: vec![stderr.into()],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgCommandError
{
    UnknownCommand(String),
    UnsupportedOption(String),
    MissingOptionValue(char),
    InvalidDebugLevel(String),
    UnexpectedArguments
    {
        command: AstgCommandKind,
        actual: usize,
    },
    MissingCurrentGraph,
    MissingInitialState,
    CscViolation,
    Backend(String),
}

impl fmt::Display for AstgCommandError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::UnknownCommand(command) => write!(formatter, "unknown ASTG command {command}"),
            Self::UnsupportedOption(option) => {
                write!(formatter, "unsupported ASTG option {option}\n{ASTG_SYN_USAGE}")
            }
            Self::MissingOptionValue(option) => write!(formatter, "missing value for -{option}"),
            Self::InvalidDebugLevel(value) => write!(formatter, "invalid debug level {value}"),
            Self::UnexpectedArguments { command, actual } => {
                write!(formatter, "{command:?} expected no arguments, got {actual}")
            }
            Self::MissingCurrentGraph => formatter.write_str("no current ASTG graph"),
            Self::MissingInitialState => {
                formatter.write_str("can't find live, safe initial marking")
            }
            Self::CscViolation => formatter.write_str("CSC violation during token flow"),
            Self::Backend(message) => formatter.write_str(message),
        }
    }
}

impl Error for AstgCommandError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgGraphSnapshot
{
    pub file_name: String,
    pub signal_count: usize,
    pub output_count: usize,
    pub initial_state: Option<AstgStateSnapshot>,
    pub state_count: Option<usize>,
    pub state_graph_lines: Vec<String>,
}

impl AstgGraphSnapshot
{
    pub fn input_count(&self) -> usize
    {
        self.signal_count.saturating_sub(self.output_count)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgStateSnapshot
{
    pub state: String,
    pub enabled: String,
}

pub trait AstgBackend
{
    fn run_token_flow_command(&mut self) -> Result<(), AstgCommandError>;

    fn synthesize(&mut self, options: AstgSynOptions) -> Result<AstgSynthesisResult, AstgCommandError>;

    fn current_graph(&self) -> Option<AstgGraphSnapshot>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgSynthesisResult
{
    ReplacedNetwork,
    NoNetworkProduced,
}

pub fn astg_command_registrations() -> &'static [CommandRegistration]
{
    ASTG_COMMANDS
}

pub fn si_cmds() -> &'static [CommandRegistration]
{
    astg_command_registrations()
}

pub fn parse_astg_command<I, S>(
    command_name: &str,
    args: I,
) -> Result<AstgCommand, AstgCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name
    {
        "astg_syn" => parse_astg_syn_args(args).map(AstgCommand::Synthesize),
        "astg_print_sg" => {
            require_no_args(AstgCommandKind::PrintStateGraph, args)?;
            Ok(AstgCommand::PrintStateGraph)
        }
        "astg_print_stat" => {
            require_no_args(AstgCommandKind::PrintStatistics, args)?;
            Ok(AstgCommand::PrintStatistics)
        }
        _ => Err(AstgCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn parse_astg_syn_args<I, S>(args: I) -> Result<AstgSynOptions, AstgCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = AstgSynOptions::default();
    let mut iter = args.into_iter().map(|arg| arg.as_ref().to_owned());

    while let Some(arg) = iter.next()
    {
        if arg == "--"
        {
            if let Some(extra) = iter.next()
            {
                return Err(AstgCommandError::UnsupportedOption(extra));
            }

            break;
        }

        let Some(flags) = arg.strip_prefix('-') else
        {
            return Err(AstgCommandError::UnsupportedOption(arg));
        };

        if flags.is_empty()
        {
            return Err(AstgCommandError::UnsupportedOption(arg));
        }

        let mut chars = flags.chars().peekable();
        while let Some(flag) = chars.next()
        {
            match flag
            {
                'm' => options.remove_mic_moc_hazards = false,
                'r' => options.add_redundancy = false,
                'x' => options.skip_token_flow = true,
                'v' => {
                    let inline: String = chars.collect();
                    let value = if inline.is_empty()
                    {
                        iter.next().ok_or(AstgCommandError::MissingOptionValue('v'))?
                    }
                    else
                    {
                        inline
                    };
                    options.debug_level = value
                        .parse()
                        .map_err(|_| AstgCommandError::InvalidDebugLevel(value))?;
                    break;
                }
                _ => return Err(AstgCommandError::UnsupportedOption(format!("-{flag}"))),
            }
        }
    }

    Ok(options)
}

pub fn dispatch_astg_command<B>(
    backend: &mut B,
    command: &AstgCommand,
) -> Result<AstgCommandOutput, AstgCommandError>
where
    B: AstgBackend,
{
    match command
    {
        AstgCommand::Synthesize(options) => astg_syn(backend, *options),
        AstgCommand::PrintStateGraph => astg_print_sg(backend),
        AstgCommand::PrintStatistics => astg_print_stat(backend),
    }
}

pub fn astg_syn<B>(
    backend: &mut B,
    options: AstgSynOptions,
) -> Result<AstgCommandOutput, AstgCommandError>
where
    B: AstgBackend,
{
    if !options.skip_token_flow
    {
        backend.run_token_flow_command()?;
    }

    let _result = backend.synthesize(options)?;
    Ok(AstgCommandOutput::success(Vec::new()))
}

pub fn astg_print_stat<B>(backend: &B) -> Result<AstgCommandOutput, AstgCommandError>
where
    B: AstgBackend,
{
    let graph = backend
        .current_graph()
        .ok_or(AstgCommandError::MissingCurrentGraph)?;

    Ok(AstgCommandOutput::success(format_statistics(&graph)))
}

pub fn astg_print_sg<B>(backend: &B) -> Result<AstgCommandOutput, AstgCommandError>
where
    B: AstgBackend,
{
    let graph = backend
        .current_graph()
        .ok_or(AstgCommandError::MissingCurrentGraph)?;

    Ok(AstgCommandOutput::success(graph.state_graph_lines))
}

pub fn format_statistics(graph: &AstgGraphSnapshot) -> Vec<String>
{
    let mut lines = vec![
        format!("File Name = {}", graph.file_name),
        format!(
            "Total Number of Signals = {} (I = {}/O = {})",
            graph.signal_count,
            graph.input_count(),
            graph.output_count
        ),
    ];

    match &graph.initial_state
    {
        Some(initial) =>
        {
            lines.push(format!("Initial State = {}", initial.state));
            if !initial.enabled.is_empty()
            {
                lines.push(initial.enabled.clone());
            }
        }
        None =>
        {
            lines.push("Initial State = ?? (can't find live, safe initial marking".to_owned());
            lines.push("Total Number of States = ??".to_owned());
            return lines;
        }
    }

    match graph.state_count
    {
        Some(count) => lines.push(format!("Total Number of States = {count}")),
        None => lines.push("Total Number of States = ?? (csc violation)".to_owned()),
    }

    lines
}

fn require_no_args<I, S>(command: AstgCommandKind, args: I) -> Result<(), AstgCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let actual = args.into_iter().count();
    if actual != 0
    {
        return Err(AstgCommandError::UnexpectedArguments
        {
            command,
            actual,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Default)]
    struct RecordingBackend
    {
        calls: Vec<String>,
        graph: Option<AstgGraphSnapshot>,
        flow_error: Option<AstgCommandError>,
    }

    impl AstgBackend for RecordingBackend
    {
        fn run_token_flow_command(&mut self) -> Result<(), AstgCommandError>
        {
            self.calls.push("flow".to_owned());
            if let Some(error) = self.flow_error.clone()
            {
                return Err(error);
            }

            Ok(())
        }

        fn synthesize(
            &mut self,
            options: AstgSynOptions,
        ) -> Result<AstgSynthesisResult, AstgCommandError>
        {
            self.calls.push(format!(
                "synth:moc={}:red={}:debug={}:skip={}",
                options.remove_mic_moc_hazards,
                options.add_redundancy,
                options.debug_level,
                options.skip_token_flow
            ));
            Ok(AstgSynthesisResult::ReplacedNetwork)
        }

        fn current_graph(&self) -> Option<AstgGraphSnapshot>
        {
            self.graph.clone()
        }
    }

    fn sample_graph() -> AstgGraphSnapshot
    {
        AstgGraphSnapshot
        {
            file_name: "sample.g".to_owned(),
            signal_count: 5,
            output_count: 2,
            initial_state: Some(AstgStateSnapshot
            {
                state: "01010".to_owned(),
                enabled: " enabled: a+ b-".to_owned(),
            }),
            state_count: Some(12),
            state_graph_lines: vec!["s0 -> s1".to_owned(), "s1 -> s0".to_owned()],
        }
    }

    #[test]
    fn registers_astg_commands_with_legacy_change_flags()
    {
        assert_eq!(
            si_cmds(),
            &[
                CommandRegistration
                {
                    name: "astg_syn",
                    kind: AstgCommandKind::Synthesize,
                    changes_network: true,
                },
                CommandRegistration
                {
                    name: "astg_print_sg",
                    kind: AstgCommandKind::PrintStateGraph,
                    changes_network: false,
                },
                CommandRegistration
                {
                    name: "astg_print_stat",
                    kind: AstgCommandKind::PrintStatistics,
                    changes_network: false,
                },
            ]
        );
    }

    #[test]
    fn parses_syn_defaults_and_flags()
    {
        assert_eq!(parse_astg_syn_args(std::iter::empty::<&str>()).unwrap(), AstgSynOptions::default());

        assert_eq!(
            parse_astg_syn_args(["-mrx", "-v", "3"]).unwrap(),
            AstgSynOptions
            {
                remove_mic_moc_hazards: false,
                add_redundancy: false,
                debug_level: 3,
                skip_token_flow: true,
            }
        );

        assert_eq!(parse_astg_syn_args(["-v7"]).unwrap().debug_level, 7);
    }

    #[test]
    fn reports_syn_usage_errors()
    {
        assert_eq!(
            parse_astg_syn_args(["-v"]),
            Err(AstgCommandError::MissingOptionValue('v'))
        );
        assert_eq!(
            parse_astg_syn_args(["-v", "loud"]),
            Err(AstgCommandError::InvalidDebugLevel("loud".to_owned()))
        );
        assert_eq!(
            parse_astg_syn_args(["-z"]),
            Err(AstgCommandError::UnsupportedOption("-z".to_owned()))
        );
    }

    #[test]
    fn parses_command_names_and_rejects_extra_print_args()
    {
        assert_eq!(
            parse_astg_command("astg_syn", ["-x"]).unwrap(),
            AstgCommand::Synthesize(AstgSynOptions
            {
                skip_token_flow: true,
                ..AstgSynOptions::default()
            })
        );
        assert_eq!(
            parse_astg_command("astg_print_sg", Vec::<&str>::new()).unwrap(),
            AstgCommand::PrintStateGraph
        );
        assert_eq!(
            parse_astg_command("astg_print_stat", ["extra"]),
            Err(AstgCommandError::UnexpectedArguments
            {
                command: AstgCommandKind::PrintStatistics,
                actual: 1,
            })
        );
    }

    #[test]
    fn synthesis_runs_flow_before_minimization_unless_skipped()
    {
        let mut backend = RecordingBackend::default();
        let output = astg_syn(&mut backend, AstgSynOptions::default()).unwrap();

        assert_eq!(output, AstgCommandOutput::success(Vec::new()));
        assert_eq!(
            backend.calls,
            vec!["flow", "synth:moc=true:red=true:debug=0:skip=false"]
        );

        let mut backend = RecordingBackend::default();
        astg_syn(
            &mut backend,
            AstgSynOptions
            {
                skip_token_flow: true,
                ..AstgSynOptions::default()
            },
        )
        .unwrap();

        assert_eq!(backend.calls, vec!["synth:moc=true:red=true:debug=0:skip=true"]);
    }

    #[test]
    fn synthesis_returns_flow_failure_without_synthesizing()
    {
        let mut backend = RecordingBackend
        {
            flow_error: Some(AstgCommandError::Backend("flow failed".to_owned())),
            ..RecordingBackend::default()
        };

        assert_eq!(
            astg_syn(&mut backend, AstgSynOptions::default()),
            Err(AstgCommandError::Backend("flow failed".to_owned()))
        );
        assert_eq!(backend.calls, vec!["flow"]);
    }

    #[test]
    fn formats_statistics_like_legacy_command_output()
    {
        assert_eq!(
            format_statistics(&sample_graph()),
            vec![
                "File Name = sample.g",
                "Total Number of Signals = 5 (I = 3/O = 2)",
                "Initial State = 01010",
                " enabled: a+ b-",
                "Total Number of States = 12",
            ]
        );
    }

    #[test]
    fn formats_initial_state_and_csc_failure_cases()
    {
        let mut graph = sample_graph();
        graph.initial_state = None;
        assert_eq!(
            format_statistics(&graph),
            vec![
                "File Name = sample.g",
                "Total Number of Signals = 5 (I = 3/O = 2)",
                "Initial State = ?? (can't find live, safe initial marking",
                "Total Number of States = ??",
            ]
        );

        let mut graph = sample_graph();
        graph.state_count = None;
        assert_eq!(
            format_statistics(&graph).last().unwrap(),
            "Total Number of States = ?? (csc violation)"
        );
    }

    #[test]
    fn print_commands_read_the_current_graph_snapshot()
    {
        let backend = RecordingBackend
        {
            graph: Some(sample_graph()),
            ..RecordingBackend::default()
        };

        assert_eq!(
            astg_print_sg(&backend).unwrap().stdout,
            vec!["s0 -> s1", "s1 -> s0"]
        );
        assert_eq!(
            astg_print_stat(&backend).unwrap().stdout[0],
            "File Name = sample.g"
        );
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_c_abi_exports()
    {
        let source = include_str!("si_com.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
