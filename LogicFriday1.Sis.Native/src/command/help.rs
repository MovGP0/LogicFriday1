//! Native Rust help command planning for the SIS command package.
//!
//! The legacy command either opens graphical help, lists command names, or
//! invokes a pager for a formatted help topic. This port keeps those decisions
//! as typed data so the caller can wire graphics, terminal output, and process
//! execution at an integration boundary.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

pub const HELP_USAGE: &str = "usage: help [-a] [command]\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration
{
    pub name: &'static str,
    pub changes_network: bool,
}

pub const HELP_COMMAND: CommandRegistration = CommandRegistration {
    name: "help",
    changes_network: false,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HelpOptions
{
    pub all: bool,
    pub geometry: Option<String>,
    pub operands: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpContext
{
    pub graphics_enabled: bool,
    pub library_path: PathBuf,
    pub commands: BTreeSet<String>,
    pub aliases: BTreeMap<String, Vec<String>>,
}

impl HelpContext
{
    pub fn new(library_path: impl Into<PathBuf>) -> Self
    {
        Self {
            graphics_enabled: false,
            library_path: library_path.into(),
            commands: BTreeSet::new(),
            aliases: BTreeMap::new(),
        }
    }

    pub fn with_graphics_enabled(mut self, graphics_enabled: bool) -> Self
    {
        self.graphics_enabled = graphics_enabled;
        self
    }

    pub fn with_commands<I, S>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.commands = commands.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_alias<I, S>(mut self, name: impl Into<String>, argv: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.aliases
            .insert(name.into(), argv.into_iter().map(Into::into).collect());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphicsHelpDocument
{
    pub geometry: Option<String>,
    pub topic: String,
}

impl GraphicsHelpDocument
{
    pub fn render(&self) -> String
    {
        let mut output = String::new();

        if let Some(geometry) = &self.geometry
        {
            output.push_str(".geometry\t");
            output.push_str(geometry);
            output.push('\n');
        }

        output.push_str(".topic\t");
        output.push_str(&self.topic);
        output.push('\n');

        output
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HelpAction
{
    OpenGraphics(GraphicsHelpDocument),
    ListCommands
    {
        commands: Vec<String>,
        output: String,
    },
    PageTopic
    {
        topic: String,
        pager: String,
        help_file: PathBuf,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HelpError
{
    Usage,
}

impl fmt::Display for HelpError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::Usage => f.write_str(HELP_USAGE),
        }
    }
}

impl Error for HelpError {}

pub fn help_command_registration() -> CommandRegistration
{
    HELP_COMMAND
}

pub fn parse_help_options<I, S>(args: I) -> Result<HelpOptions, HelpError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut options = HelpOptions::default();
    let mut iter = args.into_iter().map(Into::into).peekable();
    let mut parse_options = true;

    while let Some(arg) = iter.next()
    {
        if parse_options && arg == "--"
        {
            parse_options = false;
            continue;
        }

        if parse_options && arg == "-a"
        {
            options.all = true;
            continue;
        }

        if parse_options && arg == "-g"
        {
            let Some(geometry) = iter.next() else
            {
                return Err(HelpError::Usage);
            };

            options.geometry = Some(geometry);
            continue;
        }

        if parse_options && arg.starts_with("-g") && arg.len() > 2
        {
            options.geometry = Some(arg[2..].to_owned());
            continue;
        }

        if parse_options && arg.starts_with('-') && arg.len() > 1
        {
            return Err(HelpError::Usage);
        }

        parse_options = false;
        options.operands.push(arg);
    }

    Ok(options)
}

pub fn plan_help_command<I, S>(args: I, context: &HelpContext) -> Result<HelpAction, HelpError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let options = parse_help_options(args)?;

    if !options.all && context.graphics_enabled
    {
        let topic = if options.operands.len() == 1
        {
            resolve_alias_topic(&options.operands[0], &context.aliases)
        }
        else
        {
            "help".to_owned()
        };

        return Ok(HelpAction::OpenGraphics(GraphicsHelpDocument {
            geometry: options.geometry,
            topic,
        }));
    }

    match options.operands.as_slice()
    {
        [] =>
        {
            let commands = visible_commands(&context.commands, options.all);
            let output = render_command_listing(&commands);

            Ok(HelpAction::ListCommands {
                commands,
                output,
            })
        }
        [command] =>
        {
            let topic = resolve_alias_topic(command, &context.aliases);
            let help_file = help_file_path(&context.library_path, &topic);

            Ok(HelpAction::PageTopic {
                topic,
                pager: default_pager().to_owned(),
                help_file,
            })
        }
        _ => Err(HelpError::Usage),
    }
}

pub fn resolve_alias_topic(
    command: &str,
    aliases: &BTreeMap<String, Vec<String>>,
) -> String
{
    aliases
        .get(command)
        .and_then(|argv| argv.first())
        .cloned()
        .unwrap_or_else(|| command.to_owned())
}

pub fn visible_commands(commands: &BTreeSet<String>, all: bool) -> Vec<String>
{
    commands
        .iter()
        .filter(|command| command.starts_with('_') == all)
        .cloned()
        .collect()
}

pub fn render_command_listing(commands: &[String]) -> String
{
    let mut output = String::new();

    for (index, command) in commands.iter().enumerate()
    {
        output.push_str(&format!("{command:<15}"));

        if (index + 1) % 5 == 0
        {
            output.push('\n');
        }
    }

    if !commands.is_empty() && commands.len() % 5 != 0
    {
        output.push('\n');
    }

    output
}

pub fn help_file_path(library_path: &Path, command: &str) -> PathBuf
{
    library_path.join("help").join(format!("{command}.fmt"))
}

pub fn default_pager() -> &'static str
{
    if cfg!(unix)
    {
        "less"
    }
    else
    {
        "more"
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn sample_context() -> HelpContext
    {
        HelpContext::new("/sis/lib")
            .with_commands(["_internal", "alias", "help", "print", "_trace", "write"])
            .with_alias("h", ["help"])
            .with_alias("ps", ["print_stats", "-v"])
    }

    #[test]
    fn command_registration_is_non_mutating_help_command()
    {
        assert_eq!(
            help_command_registration(),
            CommandRegistration {
                name: "help",
                changes_network: false,
            }
        );
    }

    #[test]
    fn parses_all_and_geometry_options()
    {
        let options = parse_help_options(["-a", "-g", "80x24+1+2", "write"]).unwrap();

        assert!(options.all);
        assert_eq!(options.geometry.as_deref(), Some("80x24+1+2"));
        assert_eq!(options.operands, ["write"]);
    }

    #[test]
    fn rejects_unknown_options_with_legacy_usage()
    {
        let error = parse_help_options(["-x"]).unwrap_err();

        assert_eq!(error, HelpError::Usage);
        assert_eq!(error.to_string(), HELP_USAGE);
    }

    #[test]
    fn graphics_help_uses_alias_topic_for_one_operand()
    {
        let context = sample_context().with_graphics_enabled(true);
        let action = plan_help_command(["-g80x24", "ps"], &context).unwrap();

        assert_eq!(
            action,
            HelpAction::OpenGraphics(GraphicsHelpDocument {
                geometry: Some("80x24".to_owned()),
                topic: "print_stats".to_owned(),
            })
        );
    }

    #[test]
    fn graphics_help_defaults_to_help_topic_for_zero_or_many_operands()
    {
        let context = sample_context().with_graphics_enabled(true);

        assert_eq!(
            plan_help_command(Vec::<String>::new(), &context).unwrap(),
            HelpAction::OpenGraphics(GraphicsHelpDocument {
                geometry: None,
                topic: "help".to_owned(),
            })
        );
        assert_eq!(
            plan_help_command(["read", "write"], &context).unwrap(),
            HelpAction::OpenGraphics(GraphicsHelpDocument {
                geometry: None,
                topic: "help".to_owned(),
            })
        );
    }

    #[test]
    fn all_option_bypasses_graphics_and_lists_internal_commands()
    {
        let context = sample_context().with_graphics_enabled(true);
        let action = plan_help_command(["-a"], &context).unwrap();

        assert_eq!(
            action,
            HelpAction::ListCommands {
                commands: vec!["_internal".to_owned(), "_trace".to_owned()],
                output: "_internal      _trace         \n".to_owned(),
            }
        );
    }

    #[test]
    fn public_listing_is_sorted_and_wrapped_after_five_entries()
    {
        let context = HelpContext::new("/sis/lib").with_commands([
            "write", "read", "_hidden", "help", "alias", "print", "quit", "source",
        ]);
        let action = plan_help_command(Vec::<String>::new(), &context).unwrap();

        assert_eq!(
            action,
            HelpAction::ListCommands {
                commands: vec![
                    "alias".to_owned(),
                    "help".to_owned(),
                    "print".to_owned(),
                    "quit".to_owned(),
                    "read".to_owned(),
                    "source".to_owned(),
                    "write".to_owned(),
                ],
                output:
                    "alias          help           print          quit           read           \nsource         write          \n"
                        .to_owned(),
            }
        );
    }

    #[test]
    fn one_terminal_operand_pages_resolved_help_file()
    {
        let context = sample_context();
        let action = plan_help_command(["h"], &context).unwrap();

        assert_eq!(
            action,
            HelpAction::PageTopic {
                topic: "help".to_owned(),
                pager: default_pager().to_owned(),
                help_file: PathBuf::from("/sis/lib").join("help").join("help.fmt"),
            }
        );
    }

    #[test]
    fn many_terminal_operands_are_usage_errors()
    {
        let context = sample_context();

        assert_eq!(
            plan_help_command(["read", "write"], &context),
            Err(HelpError::Usage)
        );
    }

    #[test]
    fn graphics_document_renders_legacy_directives()
    {
        let document = GraphicsHelpDocument {
            geometry: Some("+1+2".to_owned()),
            topic: "read".to_owned(),
        };

        assert_eq!(document.render(), ".geometry\t+1+2\n.topic\tread\n");
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_c_abi_exports()
    {
        let source = include_str!("help.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
