//! Native command interpreter support for the SIS command package.
//!
//! The legacy implementation combines command dispatch, command-line tokenizing,
//! alias expansion, shell escapes, and passive auto-execution. This module keeps
//! those behaviors in safe Rust and leaves process-level signal handling and
//! external facade wiring to higher integration layers.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::process::Command;

pub type CommandStatus = i32;
pub type CommandResult = Result<CommandStatus, CommandError>;

pub const SUCCESS: CommandStatus = 0;
pub const FAILURE: CommandStatus = 1;
pub const DEFAULT_SHELL_CHAR: char = '!';
pub const MAX_ALIAS_EXPANSIONS: usize = 20;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandError {
    Interrupted,
    AliasLoop,
    HistoryExpansion(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interrupted => write!(f, "command interrupted"),
            Self::AliasLoop => write!(f, "alias expansion loop"),
            Self::HistoryExpansion(message) => write!(f, "{message}"),
        }
    }
}

impl Error for CommandError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitLine {
    pub argv: Vec<String>,
    pub rest: String,
    pub unbalanced_quote: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Alias {
    argv: Vec<String>,
}

impl Alias {
    pub fn new<I, S>(argv: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            argv: argv.into_iter().map(Into::into).collect(),
        }
    }

    pub fn argv(&self) -> &[String] {
        &self.argv
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryExpansion {
    pub value: String,
    pub substituted: bool,
}

impl HistoryExpansion {
    pub fn unchanged(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            substituted: false,
        }
    }

    pub fn substituted(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            substituted: true,
        }
    }
}

pub trait HistoryExpander {
    fn expand(&mut self, value: &str) -> Result<HistoryExpansion, CommandError>;
}

#[derive(Default)]
pub struct IdentityHistoryExpander;

impl HistoryExpander for IdentityHistoryExpander {
    fn expand(&mut self, value: &str) -> Result<HistoryExpansion, CommandError> {
        Ok(HistoryExpansion::unchanged(value))
    }
}

pub trait ShellExecutor {
    fn execute(&mut self, command: &str) -> CommandStatus;
}

#[derive(Default)]
pub struct SystemShellExecutor;

impl ShellExecutor for SystemShellExecutor {
    fn execute(&mut self, command: &str) -> CommandStatus {
        #[cfg(windows)]
        let status = Command::new("cmd").args(["/C", command]).status();

        #[cfg(not(windows))]
        let status = Command::new("sh").args(["-c", command]).status();

        status.map_or(FAILURE, |status| status.code().unwrap_or(FAILURE))
    }
}

struct CommandDescriptor<N> {
    changes_network: bool,
    action: Box<dyn FnMut(&mut Option<N>, &[String]) -> CommandResult>,
}

pub struct CommandInterpreter<N> {
    commands: HashMap<String, CommandDescriptor<N>>,
    aliases: HashMap<String, Alias>,
    flags: HashMap<String, String>,
    diagnostics: Vec<String>,
    shell_char: char,
    autoexec: bool,
    history_expander: Box<dyn HistoryExpander>,
    shell_executor: Box<dyn ShellExecutor>,
}

impl<N> Default for CommandInterpreter<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N> CommandInterpreter<N> {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
            flags: HashMap::new(),
            diagnostics: Vec::new(),
            shell_char: DEFAULT_SHELL_CHAR,
            autoexec: false,
            history_expander: Box::new(IdentityHistoryExpander),
            shell_executor: Box::new(SystemShellExecutor),
        }
    }

    pub fn with_history_expander(mut self, history_expander: Box<dyn HistoryExpander>) -> Self {
        self.history_expander = history_expander;
        self
    }

    pub fn with_shell_executor(mut self, shell_executor: Box<dyn ShellExecutor>) -> Self {
        self.shell_executor = shell_executor;
        self
    }

    pub fn register_command<F>(&mut self, name: impl Into<String>, changes_network: bool, action: F)
    where
        F: FnMut(&mut Option<N>, &[String]) -> CommandResult + 'static,
    {
        self.commands.insert(
            name.into(),
            CommandDescriptor {
                changes_network,
                action: Box::new(action),
            },
        );
    }

    pub fn set_alias<I, S>(&mut self, name: impl Into<String>, argv: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.aliases.insert(name.into(), Alias::new(argv));
    }

    pub fn remove_alias(&mut self, name: &str) -> Option<Alias> {
        self.aliases.remove(name)
    }

    pub fn set_flag(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.flags.insert(name.into(), value.into());
    }

    pub fn remove_flag(&mut self, name: &str) -> Option<String> {
        self.flags.remove(name)
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    pub fn execute(&mut self, network: &mut Option<N>, command: &str) -> CommandResult
    where
        N: Clone,
    {
        let mut commandp = command.to_owned();

        loop {
            if let Some(shell_status) = self.check_shell_escape(&commandp) {
                return Ok(shell_status);
            }

            let split = split_command_line(&commandp);
            self.record_split_diagnostics(&split);
            let mut argv = split.argv;
            let mut loop_count = 0;

            let mut status = self.apply_alias(network, &mut argv, &mut loop_count)?;
            if status == SUCCESS {
                status = self.dispatch(network, &argv)?;
            }

            commandp = split.rest;
            if status != SUCCESS || commandp.is_empty() {
                return Ok(status);
            }
        }
    }

    fn apply_alias(
        &mut self,
        network: &mut Option<N>,
        argv: &mut Vec<String>,
        loop_count: &mut usize,
    ) -> CommandResult
    where
        N: Clone,
    {
        let mut stop_expanding_current_name = false;

        while *loop_count < MAX_ALIAS_EXPANSIONS {
            if argv.is_empty() {
                return Ok(SUCCESS);
            }

            let Some(alias) = self.aliases.get(&argv[0]).cloned() else {
                return Ok(SUCCESS);
            };

            if stop_expanding_current_name {
                return Ok(SUCCESS);
            }

            if alias.argv.first() == argv.first() {
                stop_expanding_current_name = true;
            }

            *loop_count += 1;

            let trailing_args = argv.iter().skip(1).cloned().collect::<Vec<_>>();
            let mut expanded_args = Vec::new();
            let mut did_substitute = false;

            for alias_arg in alias.argv {
                let expansion = self.history_expander.expand(&alias_arg)?;
                did_substitute |= expansion.substituted;

                let mut remaining = expansion.value;
                loop {
                    let split = split_command_line(&remaining);
                    self.record_split_diagnostics(&split);

                    if split.rest.is_empty() {
                        expanded_args.extend(split.argv);
                        break;
                    }

                    let mut nested_argv = split.argv;
                    let status = self.apply_alias(network, &mut nested_argv, loop_count)?;
                    if status == SUCCESS {
                        let status = self.dispatch(network, &nested_argv)?;
                        if status != SUCCESS {
                            return Ok(status);
                        }
                    } else {
                        return Ok(status);
                    }

                    remaining = split.rest;
                }
            }

            if !did_substitute {
                expanded_args.extend(trailing_args);
            }

            *argv = expanded_args;
        }

        self.diagnostics
            .push("error: alias expansion loop".to_owned());
        Err(CommandError::AliasLoop)
    }

    fn dispatch(&mut self, network: &mut Option<N>, argv: &[String]) -> CommandResult
    where
        N: Clone,
    {
        let Some(command_name) = argv.first() else {
            return Ok(SUCCESS);
        };

        let (changes_network, backup_network, result) = {
            let Some(descriptor) = self.commands.get_mut(command_name) else {
                self.diagnostics
                    .push(format!("unknown command '{}'", command_name));
                return Ok(FAILURE);
            };

            let changes_network = descriptor.changes_network;
            let backup_network = changes_network.then(|| network.clone());
            let result = (descriptor.action)(network, argv);
            (changes_network, backup_network, result)
        };

        let mut status = match result {
            Ok(status) => status,
            Err(CommandError::Interrupted) if changes_network => {
                if let Some(backup_network) = backup_network {
                    *network = backup_network;
                }
                return Err(CommandError::Interrupted);
            }
            Err(error) => return Err(error),
        };

        if status == SUCCESS && !self.autoexec {
            if let Some(autoexec_command) = self.flags.get("autoexec").cloned() {
                self.autoexec = true;
                let autoexec_result = self.execute(network, &autoexec_command);
                self.autoexec = false;
                status = autoexec_result?;
            }
        }

        Ok(status)
    }

    fn check_shell_escape(&mut self, command: &str) -> Option<CommandStatus> {
        if let Some(value) = self.flags.get("shell_char") {
            if let Some(shell_char) = value.chars().next() {
                self.shell_char = shell_char;
            }
        }

        let trimmed = command.trim_start();
        trimmed
            .strip_prefix(self.shell_char)
            .map(|shell_command| self.shell_executor.execute(shell_command))
    }

    fn record_split_diagnostics(&mut self, split: &SplitLine) {
        if split.unbalanced_quote {
            self.diagnostics
                .push("ignoring unbalanced quote".to_owned());
        }
    }
}

pub fn split_command_line(command: &str) -> SplitLine {
    let chars = command.char_indices().collect::<Vec<_>>();
    let mut index = 0;
    let mut argv = Vec::new();
    let mut unbalanced_quote = false;

    loop {
        while let Some((_, c)) = chars.get(index) {
            if !c.is_whitespace() {
                break;
            }
            index += 1;
        }

        let start = index;
        let mut single_quote = false;
        let mut double_quote = false;

        while let Some((_, c)) = chars.get(index) {
            if (*c == ';' || *c == '#' || c.is_whitespace()) && !single_quote && !double_quote {
                break;
            }

            if *c == '\'' {
                single_quote = !single_quote;
            }

            if *c == '"' {
                double_quote = !double_quote;
            }

            index += 1;
        }

        if single_quote || double_quote {
            unbalanced_quote = true;
        }

        if start == index {
            break;
        }

        let mut argument = String::new();
        for (_, c) in &chars[start..index] {
            if *c != '\'' && *c != '"' {
                argument.push(if c.is_whitespace() { ' ' } else { *c });
            }
        }
        argv.push(argument);
    }

    let rest = match chars.get(index) {
        Some((_, ';')) => chars
            .get(index + 1)
            .map_or_else(String::new, |(byte_index, _)| {
                command[*byte_index..].to_owned()
            }),
        Some((byte_index, '#')) => command[*byte_index..]
            .chars()
            .next()
            .map_or_else(String::new, |_| String::new()),
        Some((byte_index, _)) => command[*byte_index..].to_owned(),
        None => String::new(),
    };

    SplitLine {
        argv,
        rest,
        unbalanced_quote,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct RecordingShell {
        commands: Rc<RefCell<Vec<String>>>,
        status: CommandStatus,
    }

    impl ShellExecutor for RecordingShell {
        fn execute(&mut self, command: &str) -> CommandStatus {
            self.commands.borrow_mut().push(command.to_owned());
            self.status
        }
    }

    struct BangHistory;

    impl HistoryExpander for BangHistory {
        fn expand(&mut self, value: &str) -> Result<HistoryExpansion, CommandError> {
            if value == "!:1" {
                Ok(HistoryExpansion::substituted("lion.blif"))
            } else {
                Ok(HistoryExpansion::unchanged(value))
            }
        }
    }

    #[test]
    fn splits_one_command_and_returns_remainder() {
        let split = split_command_line("  read \"a b\" 'c d'; write # ignored");

        assert_eq!(split.argv, ["read", "a b", "c d"]);
        assert_eq!(split.rest, " write # ignored");
        assert!(!split.unbalanced_quote);
    }

    #[test]
    fn splits_comment_as_end_of_command() {
        let split = split_command_line("read file # trailing");

        assert_eq!(split.argv, ["read", "file"]);
        assert_eq!(split.rest, "");
    }

    #[test]
    fn records_unbalanced_quote_without_rejecting_token() {
        let split = split_command_line("read \"file");

        assert_eq!(split.argv, ["read", "file"]);
        assert!(split.unbalanced_quote);
    }

    #[test]
    fn dispatches_commands_until_failure() {
        let calls = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut interpreter = CommandInterpreter::<String>::new();

        for name in ["first", "second"] {
            let calls = Rc::clone(&calls);
            interpreter.register_command(name, false, move |_, argv| {
                calls.borrow_mut().push(argv[0].clone());
                Ok(SUCCESS)
            });
        }

        let status = interpreter.execute(&mut None, "first; second").unwrap();

        assert_eq!(status, SUCCESS);
        assert_eq!(&*calls.borrow(), &["first", "second"]);
    }

    #[test]
    fn unknown_command_returns_failure_and_diagnostic() {
        let mut interpreter = CommandInterpreter::<String>::new();

        let status = interpreter.execute(&mut None, "missing").unwrap();

        assert_eq!(status, FAILURE);
        assert_eq!(interpreter.diagnostics(), ["unknown command 'missing'"]);
    }

    #[test]
    fn expands_alias_and_keeps_trailing_arguments() {
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut interpreter = CommandInterpreter::<String>::new();
        interpreter.set_alias("r", ["read", "-m"]);

        let seen_command = Rc::clone(&seen);
        interpreter.register_command("read", false, move |_, argv| {
            *seen_command.borrow_mut() = argv.to_vec();
            Ok(SUCCESS)
        });

        let status = interpreter.execute(&mut None, "r input.blif").unwrap();

        assert_eq!(status, SUCCESS);
        assert_eq!(&*seen.borrow(), &["read", "-m", "input.blif"]);
    }

    #[test]
    fn history_substitution_drops_original_trailing_arguments() {
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut interpreter =
            CommandInterpreter::<String>::new().with_history_expander(Box::new(BangHistory));
        interpreter.set_alias("t", ["read", "!:1"]);

        let seen_command = Rc::clone(&seen);
        interpreter.register_command("read", false, move |_, argv| {
            *seen_command.borrow_mut() = argv.to_vec();
            Ok(SUCCESS)
        });

        let status = interpreter.execute(&mut None, "t tiger.blif").unwrap();

        assert_eq!(status, SUCCESS);
        assert_eq!(&*seen.borrow(), &["read", "lion.blif"]);
    }

    #[test]
    fn alias_arg_with_semicolon_dispatches_complete_prefix() {
        let calls = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut interpreter = CommandInterpreter::<String>::new();
        interpreter.set_alias("combo", ["first; second"]);

        for name in ["first", "second"] {
            let calls = Rc::clone(&calls);
            interpreter.register_command(name, false, move |_, argv| {
                calls.borrow_mut().push(argv[0].clone());
                Ok(SUCCESS)
            });
        }

        let status = interpreter.execute(&mut None, "combo").unwrap();

        assert_eq!(status, SUCCESS);
        assert_eq!(&*calls.borrow(), &["first", "second"]);
    }

    #[test]
    fn autoexec_runs_after_successful_command_once() {
        let calls = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut interpreter = CommandInterpreter::<String>::new();
        interpreter.set_flag("autoexec", "after");

        for name in ["main", "after"] {
            let calls = Rc::clone(&calls);
            interpreter.register_command(name, false, move |_, argv| {
                calls.borrow_mut().push(argv[0].clone());
                Ok(SUCCESS)
            });
        }

        let status = interpreter.execute(&mut None, "main").unwrap();

        assert_eq!(status, SUCCESS);
        assert_eq!(&*calls.borrow(), &["main", "after"]);
    }

    #[test]
    fn shell_escape_uses_configured_shell_character() {
        let commands = Rc::new(RefCell::new(Vec::<String>::new()));
        let shell = RecordingShell {
            commands: Rc::clone(&commands),
            status: 7,
        };
        let mut interpreter =
            CommandInterpreter::<String>::new().with_shell_executor(Box::new(shell));
        interpreter.set_flag("shell_char", "$");

        let status = interpreter.execute(&mut None, "  $echo ok").unwrap();

        assert_eq!(status, 7);
        assert_eq!(&*commands.borrow(), &["echo ok"]);
    }

    #[test]
    fn interrupted_network_changing_command_restores_backup() {
        let mut interpreter = CommandInterpreter::<String>::new();
        interpreter.register_command("change", true, |network, _| {
            *network = Some("changed".to_owned());
            Err(CommandError::Interrupted)
        });

        let mut network = Some("original".to_owned());
        let result = interpreter.execute(&mut network, "change");

        assert_eq!(result, Err(CommandError::Interrupted));
        assert_eq!(network, Some("original".to_owned()));
    }
}
