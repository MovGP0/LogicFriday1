//! Native Rust alias command support for the SIS command package.
//!
//! The legacy implementation stores command aliases in a process-global AVL
//! table. This port keeps the same sorted listing, replacement, lookup, and
//! deletion behavior behind an owned table that higher command integration can
//! compose without C ABI shims.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub const UNALIAS_USAGE: &str = "usage: unalias name1 name2 ...\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub changes_network: bool,
}

pub const ALIAS_COMMAND: CommandRegistration = CommandRegistration {
    name: "alias",
    changes_network: false,
};

pub const UNALIAS_COMMAND: CommandRegistration = CommandRegistration {
    name: "unalias",
    changes_network: false,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AliasDescriptor {
    name: String,
    argv: Vec<String>,
}

impl AliasDescriptor {
    pub fn new<I, S>(name: impl Into<String>, argv: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            name: name.into(),
            argv: argv.into_iter().map(Into::into).collect(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    pub fn into_argv(self) -> Vec<String> {
        self.argv
    }

    pub fn render(&self) -> String {
        render_alias(self)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AliasTable {
    aliases: BTreeMap<String, AliasDescriptor>,
}

impl AliasTable {
    pub fn new() -> Self {
        Self {
            aliases: BTreeMap::new(),
        }
    }

    pub fn set<I, S>(&mut self, name: impl Into<String>, argv: I) -> Option<AliasDescriptor>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let name = name.into();
        let descriptor = AliasDescriptor::new(name.clone(), argv);

        self.aliases.insert(name, descriptor)
    }

    pub fn get(&self, name: &str) -> Option<&AliasDescriptor> {
        self.aliases.get(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<AliasDescriptor> {
        self.aliases.remove(name)
    }

    pub fn clear(&mut self) {
        self.aliases.clear();
    }

    pub fn len(&self) -> usize {
        self.aliases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.aliases.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &AliasDescriptor> {
        self.aliases.values()
    }

    pub fn render_all(&self) -> String {
        let mut output = String::new();

        for alias in self.iter() {
            output.push_str(&alias.render());
        }

        output
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AliasCommandAction {
    ListedAll(String),
    ListedOne(Option<String>),
    Set {
        name: String,
        argv: Vec<String>,
        replaced: Option<AliasDescriptor>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnaliasCommandAction {
    Removed(Vec<AliasDescriptor>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnaliasError {
    Usage,
}

impl fmt::Display for UnaliasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => f.write_str(UNALIAS_USAGE),
        }
    }
}

impl Error for UnaliasError {}

pub fn alias_command_registration() -> CommandRegistration {
    ALIAS_COMMAND
}

pub fn unalias_command_registration() -> CommandRegistration {
    UNALIAS_COMMAND
}

pub fn run_alias_command<I, S>(args: I, aliases: &mut AliasTable) -> AliasCommandAction
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();

    match args.as_slice() {
        [] | [_] => AliasCommandAction::ListedAll(aliases.render_all()),
        [_, name] => AliasCommandAction::ListedOne(aliases.get(name).map(render_alias)),
        [_, name, argv @ ..] => {
            let alias_argv = argv.to_vec();
            let replaced = aliases.set(name.clone(), alias_argv.clone());

            AliasCommandAction::Set {
                name: name.clone(),
                argv: alias_argv,
                replaced,
            }
        }
    }
}

pub fn run_unalias_command<I, S>(
    args: I,
    aliases: &mut AliasTable,
) -> Result<UnaliasCommandAction, UnaliasError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();

    if args.len() < 2 {
        return Err(UnaliasError::Usage);
    }

    let removed = args
        .iter()
        .skip(1)
        .filter_map(|name| aliases.remove(name))
        .collect();

    Ok(UnaliasCommandAction::Removed(removed))
}

pub fn render_alias(alias: &AliasDescriptor) -> String {
    let mut output = String::new();

    output.push_str(alias.name());
    output.push('\t');

    for arg in alias.argv() {
        output.push(' ');
        output.push_str(arg);
    }

    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_registrations_are_non_mutating() {
        assert_eq!(
            alias_command_registration(),
            CommandRegistration {
                name: "alias",
                changes_network: false,
            }
        );
        assert_eq!(
            unalias_command_registration(),
            CommandRegistration {
                name: "unalias",
                changes_network: false,
            }
        );
    }

    #[test]
    fn alias_without_operands_lists_all_aliases_in_sorted_order() {
        let mut aliases = AliasTable::new();
        aliases.set("w", ["write_blif"]);
        aliases.set("r", ["read_blif", "-s"]);

        let action = run_alias_command(["alias"], &mut aliases);

        assert_eq!(
            action,
            AliasCommandAction::ListedAll("r\t read_blif -s\nw\t write_blif\n".to_owned())
        );
    }

    #[test]
    fn alias_with_one_operand_prints_matching_alias() {
        let mut aliases = AliasTable::new();
        aliases.set("ps", ["print_stats", "-v"]);

        let action = run_alias_command(["alias", "ps"], &mut aliases);

        assert_eq!(
            action,
            AliasCommandAction::ListedOne(Some("ps\t print_stats -v\n".to_owned()))
        );
    }

    #[test]
    fn alias_with_missing_name_succeeds_without_output() {
        let mut aliases = AliasTable::new();

        let action = run_alias_command(["alias", "missing"], &mut aliases);

        assert_eq!(action, AliasCommandAction::ListedOne(None));
    }

    #[test]
    fn alias_with_two_or_more_operands_replaces_existing_alias() {
        let mut aliases = AliasTable::new();
        aliases.set("r", ["read_blif"]);

        let action = run_alias_command(["alias", "r", "read_eqn", "-a"], &mut aliases);

        assert_eq!(
            action,
            AliasCommandAction::Set {
                name: "r".to_owned(),
                argv: vec!["read_eqn".to_owned(), "-a".to_owned()],
                replaced: Some(AliasDescriptor::new("r", ["read_blif"])),
            }
        );
        assert_eq!(aliases.get("r").unwrap().argv(), ["read_eqn", "-a"]);
    }

    #[test]
    fn unalias_removes_each_named_alias_and_ignores_missing_names() {
        let mut aliases = AliasTable::new();
        aliases.set("a", ["first"]);
        aliases.set("b", ["second"]);

        let action = run_unalias_command(["unalias", "missing", "a", "b"], &mut aliases).unwrap();

        assert_eq!(
            action,
            UnaliasCommandAction::Removed(vec![
                AliasDescriptor::new("a", ["first"]),
                AliasDescriptor::new("b", ["second"]),
            ])
        );
        assert!(aliases.is_empty());
    }

    #[test]
    fn unalias_requires_at_least_one_name() {
        let mut aliases = AliasTable::new();

        let error = run_unalias_command(["unalias"], &mut aliases).unwrap_err();

        assert_eq!(error, UnaliasError::Usage);
        assert_eq!(error.to_string(), UNALIAS_USAGE);
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_c_abi_exports() {
        let source = include_str!("alias.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
