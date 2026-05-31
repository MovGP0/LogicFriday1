//! Native Rust utilities for the legacy SIS miscellaneous command file.
//!
//! The original module registers small command handlers, owns graphics stream
//! framing state, prints process statistics and history, and keeps a best
//! network snapshot. This port keeps those behaviors as explicit Rust data and
//! traits so integration layers can provide file, process, and network hooks.

use std::error::Error;
use std::fmt;
use std::time::Duration;

pub const TIME_USAGE: &str = "usage: time\n";
pub const USAGE_USAGE: &str = "usage: usage -- give CPU usage statistics\n";
pub const SAVE_USAGE: &str = "usage: save filename\n";
pub const GRAPHICS_MSG_START: &str = "\u{1b}SIS_GRAPHICS_START\n";
pub const GRAPHICS_MSG_END: &str = "\u{1b}SIS_GRAPHICS_END\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration
{
    pub name: &'static str,
    pub changes_network: bool,
    pub hidden: bool,
}

pub const MISC_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "echo",
        changes_network: false,
        hidden: false,
    },
    CommandRegistration {
        name: "quit",
        changes_network: false,
        hidden: false,
    },
    CommandRegistration {
        name: "save",
        changes_network: false,
        hidden: false,
    },
    CommandRegistration {
        name: "_iloop",
        changes_network: true,
        hidden: true,
    },
    CommandRegistration {
        name: "time",
        changes_network: false,
        hidden: false,
    },
    CommandRegistration {
        name: "usage",
        changes_network: false,
        hidden: false,
    },
    CommandRegistration {
        name: "history",
        changes_network: false,
        hidden: false,
    },
    CommandRegistration {
        name: "_which",
        changes_network: false,
        hidden: false,
    },
    CommandRegistration {
        name: "_best",
        changes_network: true,
        hidden: true,
    },
];

pub fn misc_command_registrations() -> &'static [CommandRegistration]
{
    MISC_COMMANDS
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MiscCommandError
{
    TimeUsage,
    UsageUsage,
    SaveUsage,
    ProgramNotFound,
    SaveFailed,
    MissingNetwork,
}

impl fmt::Display for MiscCommandError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::TimeUsage => formatter.write_str(TIME_USAGE),
            Self::UsageUsage => formatter.write_str(USAGE_USAGE),
            Self::SaveUsage => formatter.write_str(SAVE_USAGE),
            Self::ProgramNotFound => formatter.write_str("cannot locate current executable\n"),
            Self::SaveFailed => formatter.write_str("error occured during save ...\n"),
            Self::MissingNetwork => formatter.write_str("no current network"),
        }
    }
}

impl Error for MiscCommandError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeCommand
{
    last_time: Duration,
}

impl Default for TimeCommand
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl TimeCommand
{
    pub fn new() -> Self
    {
        Self {
            last_time: Duration::ZERO,
        }
    }

    pub fn run(&mut self, argc: usize, current_time: Duration) -> Result<String, MiscCommandError>
    {
        if argc != 1
        {
            return Err(MiscCommandError::TimeUsage);
        }

        let elapsed = current_time.saturating_sub(self.last_time);
        self.last_time = current_time;

        Ok(format!(
            "elapse: {:2.1} seconds, total: {:2.1} seconds\n",
            elapsed.as_secs_f64(),
            current_time.as_secs_f64()
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuUsageReport
{
    pub text: String,
}

impl CpuUsageReport
{
    pub fn new(text: impl Into<String>) -> Self
    {
        Self { text: text.into() }
    }
}

pub fn usage_command(argc: usize, report: &CpuUsageReport) -> Result<String, MiscCommandError>
{
    if argc != 1
    {
        return Err(MiscCommandError::UsageUsage);
    }

    Ok(report.text.clone())
}

pub fn echo_command<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut output = String::new();

    for arg in args.into_iter().skip(1)
    {
        output.push_str(arg.as_ref());
        output.push(' ');
    }

    output.push('\n');
    output
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuitAction
{
    QuickQuit,
    SaveAndQuit,
}

pub fn quit_command<I, S>(args: I) -> QuitAction
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let argv = args.into_iter().map(|arg| arg.as_ref().to_owned()).collect::<Vec<_>>();

    if argv.len() == 2 && argv[1].starts_with("-s")
    {
        QuitAction::SaveAndQuit
    }
    else
    {
        QuitAction::QuickQuit
    }
}

pub fn history_command<I, S>(entries: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut output = String::from("\nCommand history:\n");

    for entry in entries
    {
        output.push('\t');
        output.push_str(entry.as_ref());
        output.push('\n');
    }

    output.push('\n');
    output
}

pub trait CommandFileSystem
{
    fn path_search(&mut self, program_name: &str) -> Option<String>;

    fn open_file(&mut self, path: &str, mode: &str, silent: bool) -> Option<String>;

    fn save_image(&mut self, source_executable: &str, target_executable: &str) -> bool;
}

pub fn save_image_command<F>(
    args: &[String],
    program_name: &str,
    file_system: &mut F,
) -> Result<(), MiscCommandError>
where
    F: CommandFileSystem,
{
    if args.len() != 2
    {
        return Err(MiscCommandError::SaveUsage);
    }

    let source = file_system
        .path_search(program_name)
        .ok_or(MiscCommandError::ProgramNotFound)?;
    let target = file_system
        .open_file(&args[1], "w", false)
        .ok_or(MiscCommandError::SaveFailed)?;

    if file_system.save_image(&source, &target)
    {
        Ok(())
    }
    else
    {
        Err(MiscCommandError::SaveFailed)
    }
}

pub fn which_command<F>(args: &[String], file_system: &mut F) -> Option<String>
where
    F: CommandFileSystem,
{
    if args.len() != 2
    {
        return None;
    }

    file_system.open_file(&args[1], "r", false)
}

pub trait LiteralCountNetwork: Clone
{
    fn internal_literal_count(&self) -> usize;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BestNetworkTracker<N>
{
    best_network: Option<N>,
    best_count: Option<usize>,
}

impl<N> Default for BestNetworkTracker<N>
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl<N> BestNetworkTracker<N>
{
    pub fn new() -> Self
    {
        Self {
            best_network: None,
            best_count: None,
        }
    }

    pub fn best_count(&self) -> Option<usize>
    {
        self.best_count
    }

    pub fn has_snapshot(&self) -> bool
    {
        self.best_network.is_some()
    }
}

impl<N> BestNetworkTracker<N>
where
    N: LiteralCountNetwork,
{
    pub fn run(&mut self, current: &mut Option<N>) -> Result<BestDecision, MiscCommandError>
    {
        let network = current.as_ref().ok_or(MiscCommandError::MissingNetwork)?;
        let count = network.internal_literal_count();

        match self.best_count
        {
            None =>
            {
                self.best_network = Some(network.clone());
                self.best_count = Some(count);
                Ok(BestDecision::Initialized { count })
            }
            Some(best_count) if count <= best_count =>
            {
                self.best_network = Some(network.clone());
                self.best_count = Some(count);
                Ok(BestDecision::Improved {
                    old_count: best_count,
                    new_count: count,
                })
            }
            Some(best_count) =>
            {
                let Some(best_network) = &self.best_network else
                {
                    return Err(MiscCommandError::MissingNetwork);
                };

                *current = Some(best_network.clone());
                Ok(BestDecision::Restored {
                    current_count: count,
                    best_count,
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BestDecision
{
    Initialized
    {
        count: usize,
    },
    Improved
    {
        old_count: usize,
        new_count: usize,
    },
    Restored
    {
        current_count: usize,
        best_count: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphicsController
{
    enabled: bool,
    opened: bool,
}

impl Default for GraphicsController
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl GraphicsController
{
    pub fn new() -> Self
    {
        Self {
            enabled: false,
            opened: false,
        }
    }

    pub fn enable(&mut self)
    {
        self.enabled = true;
    }

    pub fn disable(&mut self)
    {
        self.enabled = false;
        self.opened = false;
    }

    pub fn is_enabled(&self) -> bool
    {
        self.enabled
    }

    pub fn is_opened(&self) -> bool
    {
        self.opened
    }

    pub fn open(
        &mut self,
        message_type: impl AsRef<str>,
        title: impl AsRef<str>,
        command: impl AsRef<str>,
    ) -> Result<GraphicsFrame, GraphicsError>
    {
        if !self.enabled
        {
            return Err(GraphicsError::Disabled);
        }

        if self.opened
        {
            return Err(GraphicsError::AlreadyOpened);
        }

        self.opened = true;

        let mut header = String::from(GRAPHICS_MSG_START);
        header.push_str(message_type.as_ref());
        header.push('\t');
        header.push_str(command.as_ref());
        header.push('\t');
        header.push_str(title.as_ref());
        header.push('\n');

        Ok(GraphicsFrame {
            header,
            data: String::new(),
        })
    }

    pub fn close(&mut self, mut frame: GraphicsFrame) -> Result<String, GraphicsError>
    {
        if !self.opened
        {
            return Err(GraphicsError::NotOpened);
        }

        frame.data.push_str(GRAPHICS_MSG_END);
        self.opened = false;
        Ok(frame.render())
    }

    pub fn exec(
        &mut self,
        message_type: impl AsRef<str>,
        title: impl AsRef<str>,
        command: impl AsRef<str>,
        data: impl AsRef<str>,
    ) -> Option<String>
    {
        if !self.enabled
        {
            return None;
        }

        let mut frame = self.open(message_type, title, command).ok()?;
        frame.write_line(data.as_ref());
        self.close(frame).ok()
    }

    pub fn help<I, S>(&mut self, commands: I) -> Option<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if !self.enabled
        {
            return None;
        }

        let mut frame = self.open("sis", "sis", "commands").ok()?;

        for command in commands
        {
            frame.write_line(command.as_ref());
        }

        self.close(frame).ok()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphicsFrame
{
    header: String,
    data: String,
}

impl GraphicsFrame
{
    pub fn write(&mut self, data: impl AsRef<str>)
    {
        self.data.push_str(data.as_ref());
    }

    pub fn write_line(&mut self, data: impl AsRef<str>)
    {
        self.data.push_str(data.as_ref());
        self.data.push('\n');
    }

    pub fn render(self) -> String
    {
        let mut output = self.header;
        output.push_str(&self.data);
        output
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphicsError
{
    Disabled,
    AlreadyOpened,
    NotOpened,
}

impl fmt::Display for GraphicsError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::Disabled => formatter.write_str("graphics stream is disabled"),
            Self::AlreadyOpened => formatter.write_str("graphics stream is already open"),
            Self::NotOpened => formatter.write_str("graphics stream is not open"),
        }
    }
}

impl Error for GraphicsError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandInitializationPlan
{
    pub registrations: Vec<CommandRegistration>,
    pub open_path_command: String,
}

pub fn init_command_plan(library_path: impl AsRef<str>) -> CommandInitializationPlan
{
    CommandInitializationPlan {
        registrations: MISC_COMMANDS.to_vec(),
        open_path_command: format!("set open_path .:{}", library_path.as_ref()),
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandShutdownPlan
{
    pub clear_flags: bool,
    pub clear_commands: bool,
    pub clear_aliases: bool,
    pub free_backup_network: bool,
    pub cleanup_error_buffer: bool,
}

pub fn end_command_plan(has_backup_network: bool) -> CommandShutdownPlan
{
    CommandShutdownPlan {
        clear_flags: true,
        clear_commands: true,
        clear_aliases: true,
        free_backup_network: has_backup_network,
        cleanup_error_buffer: true,
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNetwork
    {
        literals: usize,
        name: &'static str,
    }

    impl LiteralCountNetwork for TestNetwork
    {
        fn internal_literal_count(&self) -> usize
        {
            self.literals
        }
    }

    #[derive(Default)]
    struct TestFileSystem
    {
        searched: Vec<String>,
        opened: Vec<(String, String, bool)>,
        saves: Vec<(String, String)>,
        found_program: Option<String>,
        opened_file: Option<String>,
        save_ok: bool,
    }

    impl CommandFileSystem for TestFileSystem
    {
        fn path_search(&mut self, program_name: &str) -> Option<String>
        {
            self.searched.push(program_name.to_owned());
            self.found_program.clone()
        }

        fn open_file(&mut self, path: &str, mode: &str, silent: bool) -> Option<String>
        {
            self.opened
                .push((path.to_owned(), mode.to_owned(), silent));
            self.opened_file.clone()
        }

        fn save_image(&mut self, source_executable: &str, target_executable: &str) -> bool
        {
            self.saves
                .push((source_executable.to_owned(), target_executable.to_owned()));
            self.save_ok
        }
    }

    #[test]
    fn time_reports_elapsed_and_total_then_updates_last_time()
    {
        let mut command = TimeCommand::new();

        let first = command.run(1, Duration::from_millis(1500)).unwrap();
        let second = command.run(1, Duration::from_millis(4000)).unwrap();

        assert_eq!(first, "elapse: 1.5 seconds, total: 1.5 seconds\n");
        assert_eq!(second, "elapse: 2.5 seconds, total: 4.0 seconds\n");
    }

    #[test]
    fn time_rejects_extra_arguments_with_legacy_usage()
    {
        let mut command = TimeCommand::new();

        assert_eq!(
            command.run(2, Duration::ZERO).unwrap_err().to_string(),
            TIME_USAGE
        );
    }

    #[test]
    fn usage_returns_injected_cpu_report()
    {
        let report = CpuUsageReport::new("cpu stats\n");

        assert_eq!(usage_command(1, &report).unwrap(), "cpu stats\n");
        assert_eq!(usage_command(2, &report).unwrap_err().to_string(), USAGE_USAGE);
    }

    #[test]
    fn echo_preserves_legacy_trailing_space()
    {
        let output = echo_command(["echo", "alpha", "beta"]);

        assert_eq!(output, "alpha beta \n");
    }

    #[test]
    fn quit_maps_save_option_to_distinct_action()
    {
        assert_eq!(quit_command(["quit"]), QuitAction::QuickQuit);
        assert_eq!(quit_command(["quit", "-s"]), QuitAction::SaveAndQuit);
        assert_eq!(quit_command(["quit", "-silent"]), QuitAction::SaveAndQuit);
    }

    #[test]
    fn history_matches_legacy_heading_and_tabs()
    {
        let output = history_command(["read a.blif", "write b.blif"]);

        assert_eq!(
            output,
            "\nCommand history:\n\tread a.blif\n\twrite b.blif\n\n"
        );
    }

    #[test]
    fn save_image_searches_current_program_and_saves_opened_target()
    {
        let mut file_system = TestFileSystem {
            found_program: Some("sis.exe".to_owned()),
            opened_file: Some("copy.exe".to_owned()),
            save_ok: true,
            ..Default::default()
        };

        let args = vec!["save".to_owned(), "~/copy.exe".to_owned()];
        save_image_command(&args, "sis", &mut file_system).unwrap();

        assert_eq!(file_system.searched, ["sis"]);
        assert_eq!(
            file_system.opened,
            [("~/copy.exe".to_owned(), "w".to_owned(), false)]
        );
        assert_eq!(
            file_system.saves,
            [("sis.exe".to_owned(), "copy.exe".to_owned())]
        );
    }

    #[test]
    fn which_returns_opened_filename_only_for_one_operand()
    {
        let mut file_system = TestFileSystem {
            opened_file: Some("lib/script.rugged".to_owned()),
            ..Default::default()
        };

        let args = vec!["_which".to_owned(), "script.rugged".to_owned()];

        assert_eq!(
            which_command(&args, &mut file_system),
            Some("lib/script.rugged".to_owned())
        );
        assert_eq!(which_command(&["_which".to_owned()], &mut file_system), None);
    }

    #[test]
    fn best_tracker_keeps_non_worse_network_and_restores_previous_best()
    {
        let mut tracker = BestNetworkTracker::new();
        let mut network = Some(TestNetwork {
            literals: 5,
            name: "first",
        });

        assert_eq!(
            tracker.run(&mut network).unwrap(),
            BestDecision::Initialized { count: 5 }
        );

        network = Some(TestNetwork {
            literals: 4,
            name: "better",
        });
        assert_eq!(
            tracker.run(&mut network).unwrap(),
            BestDecision::Improved {
                old_count: 5,
                new_count: 4
            }
        );

        network = Some(TestNetwork {
            literals: 7,
            name: "worse",
        });
        assert_eq!(
            tracker.run(&mut network).unwrap(),
            BestDecision::Restored {
                current_count: 7,
                best_count: 4
            }
        );
        assert_eq!(network.unwrap().name, "better");
    }

    #[test]
    fn graphics_exec_wraps_data_in_legacy_markers()
    {
        let mut graphics = GraphicsController::new();
        graphics.enable();

        let output = graphics.exec("sis", "title", "cmd", "payload").unwrap();

        assert_eq!(
            output,
            format!("{GRAPHICS_MSG_START}sis\tcmd\ttitle\npayload\n{GRAPHICS_MSG_END}")
        );
        assert!(!graphics.is_opened());
    }

    #[test]
    fn graphics_rejects_nested_open()
    {
        let mut graphics = GraphicsController::new();
        graphics.enable();

        let _frame = graphics.open("sis", "title", "cmd").unwrap();

        assert_eq!(
            graphics.open("sis", "again", "cmd").unwrap_err(),
            GraphicsError::AlreadyOpened
        );
    }

    #[test]
    fn graphics_help_lists_commands_when_enabled()
    {
        let mut graphics = GraphicsController::new();
        graphics.enable();

        let output = graphics.help(["echo", "help"]).unwrap();

        assert_eq!(
            output,
            format!("{GRAPHICS_MSG_START}sis\tcommands\tsis\necho\nhelp\n{GRAPHICS_MSG_END}")
        );
    }

    #[test]
    fn init_and_shutdown_plans_preserve_legacy_table_shape()
    {
        let init = init_command_plan("/usr/local/sis");

        assert!(init.registrations.contains(&CommandRegistration {
            name: "_best",
            changes_network: true,
            hidden: true,
        }));
        assert_eq!(init.open_path_command, "set open_path .:/usr/local/sis");

        assert_eq!(
            end_command_plan(true),
            CommandShutdownPlan {
                clear_flags: true,
                clear_commands: true,
                clear_aliases: true,
                free_backup_network: true,
                cleanup_error_buffer: true,
            }
        );
    }

}
