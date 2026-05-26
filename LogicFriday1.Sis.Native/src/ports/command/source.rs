use std::error::Error;
use std::fmt;
use std::io::BufRead;

pub const SUCCESS: i32 = 0;
pub const FAILURE: i32 = 1;
pub const QUIT: i32 = -1;
pub const DEFAULT_PROMPT: &str = "sis> ";

const DELAY_EQUAL_EPSILON: f64 = 0.001;
const INTERACTIVE_EOF_LIMIT: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    UnitFanout,
    Library,
    Mapped,
    Tdc,
}

impl DelayModel {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "unit" | "DELAY_MODEL_UNIT" => Some(Self::Unit),
            "unit-fanout" | "unit_fanout" | "DELAY_MODEL_UNIT_FANOUT" => Some(Self::UnitFanout),
            "library" | "DELAY_MODEL_LIBRARY" => Some(Self::Library),
            "mapped" | "DELAY_MODEL_MAPPED" => Some(Self::Mapped),
            "tdc" | "DELAY_MODEL_TDC" => Some(Self::Tdc),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceOptions {
    pub interactive: bool,
    pub prompt: bool,
    pub silent: bool,
    pub echo: bool,
    pub loop_literals: bool,
    pub loop_time: bool,
    pub delay_model: DelayModel,
}

impl Default for SourceOptions {
    fn default() -> Self {
        Self {
            interactive: false,
            prompt: false,
            silent: false,
            echo: false,
            loop_literals: false,
            loop_time: false,
            delay_model: DelayModel::Unit,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceInvocation {
    pub options: SourceOptions,
    pub filename: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceError {
    MissingFilename,
    MissingDelayModelName,
    UnknownDelayModel(String),
    UnknownOption(char),
    OpenFailed(String),
    ReadFailed(String),
    History(String),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFilename => write!(f, "source [-psx] filename"),
            Self::MissingDelayModelName => write!(f, "missing delay model name after -m"),
            Self::UnknownDelayModel(name) => write!(f, "Unknown delay model {name}"),
            Self::UnknownOption(option) => write!(f, "unknown option -{option}"),
            Self::OpenFailed(message) => f.write_str(message),
            Self::ReadFailed(message) => f.write_str(message),
            Self::History(message) => f.write_str(message),
        }
    }
}

impl Error for SourceError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistorySubstitution {
    pub command: String,
    pub substituted: bool,
}

impl HistorySubstitution {
    pub fn unchanged(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            substituted: false,
        }
    }

    pub fn substituted(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            substituted: true,
        }
    }
}

pub trait SourceProvider {
    type Reader: BufRead;

    fn open(&mut self, filename: &str, silent: bool) -> Result<Option<Self::Reader>, SourceError>;
}

pub trait SourceExecutor<Network> {
    fn execute(&mut self, network: &mut Network, command: &str) -> i32;
}

pub trait SourceHistory {
    fn substitute(&mut self, line: &str) -> Result<HistorySubstitution, SourceError>;
}

#[derive(Default)]
pub struct IdentitySourceHistory;

impl SourceHistory for IdentitySourceHistory {
    fn substitute(&mut self, line: &str) -> Result<HistorySubstitution, SourceError> {
        Ok(HistorySubstitution::unchanged(line))
    }
}

pub trait SourceUi {
    fn prompt(&mut self) -> String {
        DEFAULT_PROMPT.to_owned()
    }

    fn echo(&mut self, _line: &str) {}

    fn substituted_command(&mut self, _command: &str) {}

    fn interactive_eof(&mut self) {}

    fn history_line(&mut self, _line: &str) {}

    fn diagnostic(&mut self, _message: &str) {}
}

#[derive(Default)]
pub struct SilentSourceUi;

impl SourceUi for SilentSourceUi {}

pub trait SourceNetworkMetrics<Network> {
    fn literal_count(&mut self, network: &Network) -> usize;

    fn latest_output_arrival(&mut self, network: &Network, model: DelayModel) -> f64;
}

#[derive(Default)]
pub struct NoSourceNetworkMetrics;

impl<Network> SourceNetworkMetrics<Network> for NoSourceNetworkMetrics {
    fn literal_count(&mut self, _network: &Network) -> usize {
        0
    }

    fn latest_output_arrival(&mut self, _network: &Network, _model: DelayModel) -> f64 {
        0.0
    }
}

#[derive(Clone, Debug)]
struct LoopState<Network> {
    previous_network: Option<Network>,
    previous_count: Option<usize>,
    previous_delay: Option<f64>,
}

impl<Network> Default for LoopState<Network> {
    fn default() -> Self {
        Self {
            previous_network: None,
            previous_count: None,
            previous_delay: None,
        }
    }
}

pub fn parse_source_invocation<I, S>(argv: I) -> Result<SourceInvocation, SourceError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = argv
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();

    if args.first().is_some_and(|arg| arg == "source") {
        args.remove(0);
    }

    let mut options = SourceOptions::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        if arg == "--" {
            index += 1;
            break;
        }

        if !arg.starts_with('-') || arg == "-" {
            break;
        }

        let mut chars = arg[1..].chars().peekable();
        while let Some(option) = chars.next() {
            match option {
                'i' => options.interactive = true,
                'p' => options.prompt = true,
                's' => options.silent = true,
                'x' => options.echo = true,
                'l' => options.loop_literals = true,
                't' => options.loop_time = true,
                'm' => {
                    let rest = chars.collect::<String>();
                    let model_name = if rest.is_empty() {
                        index += 1;
                        args.get(index)
                            .ok_or(SourceError::MissingDelayModelName)?
                            .as_str()
                    } else {
                        rest.as_str()
                    };
                    options.delay_model = DelayModel::from_name(model_name)
                        .ok_or_else(|| SourceError::UnknownDelayModel(model_name.to_owned()))?;
                    break;
                }
                other => return Err(SourceError::UnknownOption(other)),
            }
        }

        index += 1;
    }

    let filename = args
        .get(index)
        .ok_or(SourceError::MissingFilename)?
        .to_owned();

    Ok(SourceInvocation { options, filename })
}

pub fn run_source<Network, Provider, Executor, History, Ui, Metrics>(
    network: &mut Network,
    invocation: &SourceInvocation,
    provider: &mut Provider,
    executor: &mut Executor,
    history: &mut History,
    ui: &mut Ui,
    metrics: &mut Metrics,
) -> Result<i32, SourceError>
where
    Network: Clone,
    Provider: SourceProvider,
    Executor: SourceExecutor<Network>,
    History: SourceHistory,
    Ui: SourceUi,
    Metrics: SourceNetworkMetrics<Network>,
{
    let mut loop_count = 0;
    let mut loop_state = LoopState::default();

    loop {
        let Some(mut reader) = provider.open(&invocation.filename, invocation.options.silent)?
        else {
            return Ok(if invocation.options.silent {
                SUCCESS
            } else {
                FAILURE
            });
        };

        let status = run_open_source(network, invocation, &mut reader, executor, history, ui)?;
        if status > SUCCESS {
            ui.diagnostic(&format!("aborting 'source {}'", invocation.filename));
        }

        if status != SUCCESS {
            return Ok(status);
        }

        let should_loop = invocation.options.loop_literals || invocation.options.loop_time;
        if !should_loop {
            return Ok(SUCCESS);
        }

        let end_loop = loop_count > 0
            && update_loop_state(network, &invocation.options, metrics, &mut loop_state);
        loop_count += 1;

        if end_loop {
            return Ok(SUCCESS);
        }
    }
}

fn run_open_source<Network, Reader, Executor, History, Ui>(
    network: &mut Network,
    invocation: &SourceInvocation,
    reader: &mut Reader,
    executor: &mut Executor,
    history: &mut History,
    ui: &mut Ui,
) -> Result<i32, SourceError>
where
    Reader: BufRead,
    Executor: SourceExecutor<Network>,
    History: SourceHistory,
    Ui: SourceUi,
{
    let mut quit_count = 0;
    let mut line = String::new();

    loop {
        if invocation.options.prompt {
            let _ = ui.prompt();
        }

        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| SourceError::ReadFailed(error.to_string()))?;

        if bytes == 0 {
            if invocation.options.interactive {
                if quit_count < INTERACTIVE_EOF_LIMIT {
                    quit_count += 1;
                    ui.interactive_eof();
                    continue;
                }

                return Ok(QUIT);
            }

            return Ok(SUCCESS);
        }

        quit_count = 0;

        if invocation.options.echo {
            ui.echo(&line);
        }

        let substitution = history.substitute(&line)?;
        let command = substitution.command;
        if substitution.substituted && invocation.options.interactive {
            ui.substituted_command(&command);
        }

        if invocation.options.interactive && !command.is_empty() {
            let mut history_line = command.clone();
            trim_record_separator(&mut history_line);
            ui.history_line(&history_line);
        }

        let status = executor.execute(network, &command);
        if status != SUCCESS {
            return Ok(status);
        }
    }
}

fn update_loop_state<Network, Metrics>(
    network: &mut Network,
    options: &SourceOptions,
    metrics: &mut Metrics,
    state: &mut LoopState<Network>,
) -> bool
where
    Network: Clone,
    Metrics: SourceNetworkMetrics<Network>,
{
    let count = metrics.literal_count(network);

    if options.loop_literals {
        if state.previous_count.is_none() {
            state.previous_network = Some(network.clone());
            state.previous_count = Some(count);
            return false;
        }

        if count < state.previous_count.unwrap_or(usize::MAX) {
            state.previous_network = Some(network.clone());
            state.previous_count = Some(count);
            return false;
        }

        restore_previous_network(network, state);
        return true;
    }

    if options.loop_time {
        let delay = metrics.latest_output_arrival(network, options.delay_model);
        if state.previous_delay.is_none() {
            state.previous_network = Some(network.clone());
            state.previous_delay = Some(delay);
            state.previous_count = Some(count);
            return false;
        }

        let previous_delay = state.previous_delay.unwrap_or(f64::INFINITY);
        let previous_count = state.previous_count.unwrap_or(usize::MAX);
        if delay < previous_delay || (same_delay(delay, previous_delay) && count < previous_count) {
            state.previous_network = Some(network.clone());
            state.previous_delay = Some(delay);
            state.previous_count = Some(count);
            return false;
        }

        restore_previous_network(network, state);
        return true;
    }

    true
}

fn restore_previous_network<Network>(network: &mut Network, state: &mut LoopState<Network>) {
    if let Some(previous_network) = state.previous_network.take() {
        *network = previous_network;
    }

    state.previous_count = None;
    state.previous_delay = None;
}

fn same_delay(left: f64, right: f64) -> bool {
    (left - right).abs() < DELAY_EQUAL_EPSILON
}

fn trim_record_separator(line: &mut String) {
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io::Cursor;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNetwork {
        literals: usize,
        delay_millis: i32,
    }

    #[derive(Default)]
    struct TestProvider {
        reads: VecDeque<String>,
    }

    impl TestProvider {
        fn with_reads<I, S>(reads: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            Self {
                reads: reads.into_iter().map(Into::into).collect(),
            }
        }
    }

    impl SourceProvider for TestProvider {
        type Reader = Cursor<Vec<u8>>;

        fn open(
            &mut self,
            _filename: &str,
            silent: bool,
        ) -> Result<Option<Self::Reader>, SourceError> {
            let Some(text) = self.reads.pop_front() else {
                return Ok(if silent {
                    None
                } else {
                    Some(Cursor::new(Vec::new()))
                });
            };

            Ok(Some(Cursor::new(text.into_bytes())))
        }
    }

    #[derive(Default)]
    struct RecordingExecutor {
        commands: Vec<String>,
        statuses: VecDeque<i32>,
        next_literal_counts: VecDeque<usize>,
        next_delay_millis: VecDeque<i32>,
    }

    impl SourceExecutor<TestNetwork> for RecordingExecutor {
        fn execute(&mut self, network: &mut TestNetwork, command: &str) -> i32 {
            self.commands.push(command.to_owned());

            if let Some(literals) = self.next_literal_counts.pop_front() {
                network.literals = literals;
            }

            if let Some(delay_millis) = self.next_delay_millis.pop_front() {
                network.delay_millis = delay_millis;
            }

            self.statuses.pop_front().unwrap_or(SUCCESS)
        }
    }

    #[derive(Default)]
    struct BangHistory;

    impl SourceHistory for BangHistory {
        fn substitute(&mut self, line: &str) -> Result<HistorySubstitution, SourceError> {
            if line == "again\n" {
                Ok(HistorySubstitution::substituted("read cached.blif\n"))
            } else {
                Ok(HistorySubstitution::unchanged(line))
            }
        }
    }

    #[derive(Default)]
    struct RecordingUi {
        echoes: Vec<String>,
        substitutions: Vec<String>,
        history: Vec<String>,
        diagnostics: Vec<String>,
        eof_count: usize,
        prompts: usize,
    }

    impl SourceUi for RecordingUi {
        fn prompt(&mut self) -> String {
            self.prompts += 1;
            "prompt> ".to_owned()
        }

        fn echo(&mut self, line: &str) {
            self.echoes.push(line.to_owned());
        }

        fn substituted_command(&mut self, command: &str) {
            self.substitutions.push(command.to_owned());
        }

        fn interactive_eof(&mut self) {
            self.eof_count += 1;
        }

        fn history_line(&mut self, line: &str) {
            self.history.push(line.to_owned());
        }

        fn diagnostic(&mut self, message: &str) {
            self.diagnostics.push(message.to_owned());
        }
    }

    #[derive(Default)]
    struct TestMetrics;

    impl SourceNetworkMetrics<TestNetwork> for TestMetrics {
        fn literal_count(&mut self, network: &TestNetwork) -> usize {
            network.literals
        }

        fn latest_output_arrival(&mut self, network: &TestNetwork, _model: DelayModel) -> f64 {
            f64::from(network.delay_millis) / 1000.0
        }
    }

    #[test]
    fn parses_compact_options_and_model() {
        let invocation =
            parse_source_invocation(["source", "-ipxlt", "-m", "mapped", "script.sis"]).unwrap();

        assert_eq!(invocation.filename, "script.sis");
        assert!(invocation.options.interactive);
        assert!(invocation.options.prompt);
        assert!(invocation.options.echo);
        assert!(invocation.options.loop_literals);
        assert!(invocation.options.loop_time);
        assert_eq!(invocation.options.delay_model, DelayModel::Mapped);
    }

    #[test]
    fn rejects_unknown_delay_model() {
        assert_eq!(
            parse_source_invocation(["source", "-m", "bad", "script.sis"]),
            Err(SourceError::UnknownDelayModel("bad".to_owned()))
        );
    }

    #[test]
    fn executes_lines_until_end_of_file() {
        let invocation = parse_source_invocation(["source", "script.sis"]).unwrap();
        let mut network = TestNetwork {
            literals: 4,
            delay_millis: 100,
        };
        let mut provider = TestProvider::with_reads(["read a.blif\nprint_stats\n"]);
        let mut executor = RecordingExecutor::default();
        let mut history = IdentitySourceHistory;
        let mut ui = RecordingUi::default();
        let mut metrics = TestMetrics;

        let status = run_source(
            &mut network,
            &invocation,
            &mut provider,
            &mut executor,
            &mut history,
            &mut ui,
            &mut metrics,
        )
        .unwrap();

        assert_eq!(status, SUCCESS);
        assert_eq!(executor.commands, ["read a.blif\n", "print_stats\n"]);
    }

    #[test]
    fn echoes_lines_and_records_interactive_history_after_substitution() {
        let invocation = parse_source_invocation(["source", "-ixp", "script.sis"]).unwrap();
        let mut network = TestNetwork {
            literals: 4,
            delay_millis: 100,
        };
        let mut provider = TestProvider::with_reads(["again\n"]);
        let mut executor = RecordingExecutor::default();
        let mut history = BangHistory;
        let mut ui = RecordingUi::default();
        let mut metrics = TestMetrics;

        let status = run_source(
            &mut network,
            &invocation,
            &mut provider,
            &mut executor,
            &mut history,
            &mut ui,
            &mut metrics,
        )
        .unwrap();

        assert_eq!(status, QUIT);
        assert_eq!(ui.prompts, 7);
        assert_eq!(ui.eof_count, INTERACTIVE_EOF_LIMIT);
        assert_eq!(ui.echoes, ["again\n"]);
        assert_eq!(ui.substitutions, ["read cached.blif\n"]);
        assert_eq!(ui.history, ["read cached.blif"]);
        assert_eq!(executor.commands, ["read cached.blif\n"]);
    }

    #[test]
    fn failure_aborts_and_reports_filename() {
        let invocation = parse_source_invocation(["source", "script.sis"]).unwrap();
        let mut network = TestNetwork {
            literals: 4,
            delay_millis: 100,
        };
        let mut provider = TestProvider::with_reads(["ok\nfail\nignored\n"]);
        let mut executor = RecordingExecutor {
            statuses: VecDeque::from([SUCCESS, FAILURE]),
            ..Default::default()
        };
        let mut history = IdentitySourceHistory;
        let mut ui = RecordingUi::default();
        let mut metrics = TestMetrics;

        let status = run_source(
            &mut network,
            &invocation,
            &mut provider,
            &mut executor,
            &mut history,
            &mut ui,
            &mut metrics,
        )
        .unwrap();

        assert_eq!(status, FAILURE);
        assert_eq!(executor.commands, ["ok\n", "fail\n"]);
        assert_eq!(ui.diagnostics, ["aborting 'source script.sis'"]);
    }

    #[test]
    fn interactive_end_of_file_requires_repeated_quits() {
        let invocation = parse_source_invocation(["source", "-i", "script.sis"]).unwrap();
        let mut network = TestNetwork {
            literals: 4,
            delay_millis: 100,
        };
        let mut provider = TestProvider::with_reads([""]);
        let mut executor = RecordingExecutor::default();
        let mut history = IdentitySourceHistory;
        let mut ui = RecordingUi::default();
        let mut metrics = TestMetrics;

        let status = run_source(
            &mut network,
            &invocation,
            &mut provider,
            &mut executor,
            &mut history,
            &mut ui,
            &mut metrics,
        )
        .unwrap();

        assert_eq!(status, QUIT);
        assert_eq!(ui.eof_count, INTERACTIVE_EOF_LIMIT);
    }

    #[test]
    fn silent_missing_file_returns_success() {
        let invocation = parse_source_invocation(["source", "-s", "missing.sis"]).unwrap();
        let mut network = TestNetwork {
            literals: 4,
            delay_millis: 100,
        };
        let mut provider = TestProvider::default();
        let mut executor = RecordingExecutor::default();
        let mut history = IdentitySourceHistory;
        let mut ui = RecordingUi::default();
        let mut metrics = TestMetrics;

        let status = run_source(
            &mut network,
            &invocation,
            &mut provider,
            &mut executor,
            &mut history,
            &mut ui,
            &mut metrics,
        )
        .unwrap();

        assert_eq!(status, SUCCESS);
        assert!(executor.commands.is_empty());
    }

    #[test]
    fn literal_loop_repeats_while_count_decreases_and_restores_best_network() {
        let invocation = parse_source_invocation(["source", "-l", "script.sis"]).unwrap();
        let mut network = TestNetwork {
            literals: 10,
            delay_millis: 100,
        };
        let mut provider = TestProvider::with_reads(["opt\n", "opt\n", "opt\n"]);
        let mut executor = RecordingExecutor {
            next_literal_counts: VecDeque::from([8, 6, 7]),
            ..Default::default()
        };
        let mut history = IdentitySourceHistory;
        let mut ui = RecordingUi::default();
        let mut metrics = TestMetrics;

        let status = run_source(
            &mut network,
            &invocation,
            &mut provider,
            &mut executor,
            &mut history,
            &mut ui,
            &mut metrics,
        )
        .unwrap();

        assert_eq!(status, SUCCESS);
        assert_eq!(executor.commands, ["opt\n", "opt\n", "opt\n"]);
        assert_eq!(network.literals, 6);
    }

    #[test]
    fn time_loop_accepts_equal_delay_only_when_literals_decrease() {
        let invocation =
            parse_source_invocation(["source", "-t", "-m", "unit", "script.sis"]).unwrap();
        let mut network = TestNetwork {
            literals: 10,
            delay_millis: 1000,
        };
        let mut provider = TestProvider::with_reads(["opt\n", "opt\n", "opt\n"]);
        let mut executor = RecordingExecutor {
            next_literal_counts: VecDeque::from([9, 8, 9]),
            next_delay_millis: VecDeque::from([900, 900, 900]),
            ..Default::default()
        };
        let mut history = IdentitySourceHistory;
        let mut ui = RecordingUi::default();
        let mut metrics = TestMetrics;

        let status = run_source(
            &mut network,
            &invocation,
            &mut provider,
            &mut executor,
            &mut history,
            &mut ui,
            &mut metrics,
        )
        .unwrap();

        assert_eq!(status, SUCCESS);
        assert_eq!(network.literals, 8);
        assert_eq!(network.delay_millis, 900);
    }

    #[test]
    fn text_contains_no_dependency_metadata_or_c_abi_tokens() {
        let text = include_str!("source.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("be", "ad", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }
}
