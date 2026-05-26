//! Safe native Rust model for the process globals in `sis/main/sis_glbs.c`.
//!
//! The original C file only defines shared mutable globals:
//! diagnostic/output streams, command history, and the process program name.
//! This port keeps the same runtime state explicit and owned so startup,
//! command execution, history handling, and shutdown code can share it without
//! introducing Rust process globals or a per-file C ABI shim.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

pub const DEFAULT_PROGRAM_NAME: &str = "sis";
pub const DEFAULT_HISTORY_LIMIT: usize = 1_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisGlobals {
    pub program_name: Option<String>,
    pub output: SisStream,
    pub error: SisStream,
    pub history_stream: Option<SisStream>,
    pub command_history: CommandHistory,
}

impl Default for SisGlobals {
    fn default() -> Self {
        Self::new()
    }
}

impl SisGlobals {
    pub fn new() -> Self {
        Self {
            program_name: None,
            output: SisStream::standard_output(),
            error: SisStream::standard_error(),
            history_stream: None,
            command_history: CommandHistory::new(DEFAULT_HISTORY_LIMIT),
        }
    }

    pub fn initialize(program_name: impl Into<String>) -> Self {
        let mut globals = Self::new();
        globals.set_program_name(program_name);
        globals
    }

    pub fn set_program_name(&mut self, program_name: impl Into<String>) {
        let program_name = program_name.into();
        self.program_name = if program_name.is_empty() {
            None
        } else {
            Some(program_name)
        };
    }

    pub fn program_name_or_default(&self) -> &str {
        self.program_name.as_deref().unwrap_or(DEFAULT_PROGRAM_NAME)
    }

    pub fn set_output(&mut self, stream: SisStream) {
        self.output = stream;
    }

    pub fn set_error(&mut self, stream: SisStream) {
        self.error = stream;
    }

    pub fn open_history_stream(&mut self, stream: SisStream) {
        self.history_stream = Some(stream);
    }

    pub fn close_history_stream(&mut self) -> Option<SisStream> {
        self.history_stream.take()
    }

    pub fn record_command(&mut self, command: impl Into<String>) {
        self.command_history.push(command);
    }

    pub fn reset_io_to_standard_streams(&mut self) {
        self.output = SisStream::standard_output();
        self.error = SisStream::standard_error();
        self.history_stream = None;
    }

    pub fn clear_for_shutdown(&mut self) {
        self.program_name = None;
        self.reset_io_to_standard_streams();
        self.command_history.clear();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisStream {
    pub target: SisStreamTarget,
    pub mode: SisStreamMode,
}

impl SisStream {
    pub fn standard_output() -> Self {
        Self {
            target: SisStreamTarget::Stdout,
            mode: SisStreamMode::Append,
        }
    }

    pub fn standard_error() -> Self {
        Self {
            target: SisStreamTarget::Stderr,
            mode: SisStreamMode::Append,
        }
    }

    pub fn file(path: impl Into<String>, mode: SisStreamMode) -> Result<Self, SisGlobalError> {
        let path = path.into();
        if path.trim().is_empty() {
            return Err(SisGlobalError::EmptyPath);
        }

        Ok(Self {
            target: SisStreamTarget::File(path),
            mode,
        })
    }

    pub fn is_standard(&self) -> bool {
        matches!(
            self.target,
            SisStreamTarget::Stdout | SisStreamTarget::Stderr
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SisStreamTarget {
    Stdout,
    Stderr,
    File(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SisStreamMode {
    Read,
    Write,
    Append,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandHistory {
    limit: usize,
    entries: VecDeque<String>,
}

impl CommandHistory {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            entries: VecDeque::new(),
        }
    }

    pub fn push(&mut self, command: impl Into<String>) {
        if self.limit == 0 {
            return;
        }

        let command = command.into();
        if command.is_empty() {
            return;
        }

        while self.entries.len() >= self.limit {
            self.entries.pop_front();
        }
        self.entries.push_back(command);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.entries.iter().map(String::as_str)
    }

    pub fn newest(&self) -> Option<&str> {
        self.entries.back().map(String::as_str)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SisGlobalError {
    EmptyPath,
}

impl fmt::Display for SisGlobalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPath => f.write_str("stream path must not be empty"),
        }
    }
}

impl Error for SisGlobalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_uninitialized_c_globals_with_safe_standard_streams() {
        let globals = SisGlobals::new();

        assert_eq!(globals.program_name, None);
        assert_eq!(globals.program_name_or_default(), DEFAULT_PROGRAM_NAME);
        assert_eq!(globals.output, SisStream::standard_output());
        assert_eq!(globals.error, SisStream::standard_error());
        assert_eq!(globals.history_stream, None);
        assert!(globals.command_history.is_empty());
    }

    #[test]
    fn initialize_records_program_name_without_touching_streams() {
        let globals = SisGlobals::initialize("logicfriday-sis");

        assert_eq!(globals.program_name_or_default(), "logicfriday-sis");
        assert!(globals.output.is_standard());
        assert!(globals.error.is_standard());
    }

    #[test]
    fn empty_program_name_falls_back_to_default_name() {
        let mut globals = SisGlobals::initialize("sis-native");

        globals.set_program_name("");

        assert_eq!(globals.program_name, None);
        assert_eq!(globals.program_name_or_default(), "sis");
    }

    #[test]
    fn file_stream_rejects_empty_paths_and_records_mode() {
        assert_eq!(
            SisStream::file(" ", SisStreamMode::Write).unwrap_err(),
            SisGlobalError::EmptyPath
        );

        assert_eq!(
            SisStream::file("run.log", SisStreamMode::Append).unwrap(),
            SisStream {
                target: SisStreamTarget::File("run.log".to_string()),
                mode: SisStreamMode::Append,
            }
        );
    }

    #[test]
    fn stream_setters_track_nonstandard_output_and_error_targets() {
        let mut globals = SisGlobals::new();

        globals.set_output(SisStream::file("out.txt", SisStreamMode::Write).unwrap());
        globals.set_error(SisStream::file("err.txt", SisStreamMode::Append).unwrap());

        assert!(!globals.output.is_standard());
        assert!(!globals.error.is_standard());
    }

    #[test]
    fn history_stream_can_be_opened_and_closed() {
        let mut globals = SisGlobals::new();
        let stream = SisStream::file("history", SisStreamMode::Append).unwrap();

        globals.open_history_stream(stream.clone());

        assert_eq!(globals.close_history_stream(), Some(stream));
        assert_eq!(globals.close_history_stream(), None);
    }

    #[test]
    fn command_history_preserves_order_and_ignores_empty_commands() {
        let mut history = CommandHistory::new(4);

        history.push("read_blif input.blif");
        history.push("");
        history.push("print_stats");

        assert_eq!(
            history.entries().collect::<Vec<_>>(),
            vec!["read_blif input.blif", "print_stats"]
        );
        assert_eq!(history.newest(), Some("print_stats"));
    }

    #[test]
    fn command_history_honors_limit_by_dropping_oldest_entries() {
        let mut history = CommandHistory::new(2);

        history.push("one");
        history.push("two");
        history.push("three");

        assert_eq!(history.entries().collect::<Vec<_>>(), vec!["two", "three"]);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn zero_length_history_discards_all_commands() {
        let mut history = CommandHistory::new(0);

        history.push("help");

        assert!(history.is_empty());
        assert_eq!(history.limit(), 0);
    }

    #[test]
    fn reset_io_keeps_program_name_and_history() {
        let mut globals = SisGlobals::initialize("sis-native");
        globals.set_output(SisStream::file("out", SisStreamMode::Write).unwrap());
        globals.open_history_stream(SisStream::file("hist", SisStreamMode::Append).unwrap());
        globals.record_command("history");

        globals.reset_io_to_standard_streams();

        assert_eq!(globals.program_name_or_default(), "sis-native");
        assert_eq!(globals.output, SisStream::standard_output());
        assert_eq!(globals.error, SisStream::standard_error());
        assert_eq!(globals.history_stream, None);
        assert_eq!(globals.command_history.newest(), Some("history"));
    }

    #[test]
    fn shutdown_clear_restores_c_global_defaults() {
        let mut globals = SisGlobals::initialize("sis-native");
        globals.set_output(SisStream::file("out", SisStreamMode::Write).unwrap());
        globals.set_error(SisStream::file("err", SisStreamMode::Append).unwrap());
        globals.open_history_stream(SisStream::file("hist", SisStreamMode::Append).unwrap());
        globals.record_command("quit");

        globals.clear_for_shutdown();

        assert_eq!(globals, SisGlobals::new());
    }
}
