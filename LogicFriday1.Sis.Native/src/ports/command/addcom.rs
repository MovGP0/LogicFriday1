use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandDescriptor<Handler> {
    pub name: String,
    pub handler: Handler,
    pub changes_network: bool,
}

impl<Handler> CommandDescriptor<Handler> {
    pub fn new(name: impl Into<String>, handler: Handler, changes_network: bool) -> Self {
        let name = name.into();

        Self {
            name,
            handler,
            changes_network,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandDiagnostic {
    RedefinedCommand { name: String },
}

impl fmt::Display for CommandDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RedefinedCommand { name } => write!(f, "warning: redefining '{name}'"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandTableError {
    NameMismatch {
        key: String,
        descriptor_name: String,
    },
}

impl fmt::Display for CommandTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameMismatch {
                key,
                descriptor_name,
            } => write!(
                f,
                "command key '{key}' does not match descriptor name '{descriptor_name}'"
            ),
        }
    }
}

impl Error for CommandTableError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandAddReport<Handler> {
    pub replaced: Option<CommandDescriptor<Handler>>,
    pub diagnostic: Option<CommandDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandTable<Handler> {
    commands: BTreeMap<String, CommandDescriptor<Handler>>,
}

impl<Handler> Default for CommandTable<Handler> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Handler> CommandTable<Handler> {
    pub fn new() -> Self {
        Self {
            commands: BTreeMap::new(),
        }
    }

    pub fn add_command(
        &mut self,
        name: impl Into<String>,
        handler: Handler,
        changes_network: bool,
    ) -> CommandAddReport<Handler> {
        let descriptor = CommandDescriptor::new(name, handler, changes_network);
        let descriptor_name = descriptor.name.clone();
        let replaced = self.commands.insert(descriptor_name.clone(), descriptor);
        let diagnostic = replaced
            .as_ref()
            .map(|old| CommandDiagnostic::RedefinedCommand {
                name: old.name.clone(),
            });

        CommandAddReport {
            replaced,
            diagnostic,
        }
    }

    pub fn insert_descriptor(
        &mut self,
        key: impl Into<String>,
        descriptor: CommandDescriptor<Handler>,
    ) -> Result<Option<CommandDescriptor<Handler>>, CommandTableError> {
        let key = key.into();
        if key != descriptor.name {
            return Err(CommandTableError::NameMismatch {
                key,
                descriptor_name: descriptor.name,
            });
        }

        Ok(self.commands.insert(key, descriptor))
    }

    pub fn get(&self, name: &str) -> Option<&CommandDescriptor<Handler>> {
        self.commands.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<CommandDescriptor<Handler>> {
        self.commands.remove(name)
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &CommandDescriptor<Handler>> {
        self.commands.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler_a() -> i32 {
        1
    }

    fn handler_b() -> i32 {
        2
    }

    #[test]
    fn add_command_inserts_owned_descriptor() {
        let mut table = CommandTable::new();
        let report = table.add_command("read", handler_a as fn() -> i32, true);

        assert_eq!(report.replaced, None);
        assert_eq!(report.diagnostic, None);
        assert_eq!(table.len(), 1);

        let descriptor = table.get("read").unwrap();
        assert_eq!(descriptor.name, "read");
        assert_eq!((descriptor.handler)(), 1);
        assert!(descriptor.changes_network);
    }

    #[test]
    fn add_command_replaces_existing_descriptor_and_reports_warning() {
        let mut table = CommandTable::new();
        table.add_command("print", handler_a as fn() -> i32, false);

        let report = table.add_command("print", handler_b as fn() -> i32, true);

        assert_eq!(
            report.diagnostic,
            Some(CommandDiagnostic::RedefinedCommand {
                name: "print".to_owned()
            })
        );
        assert_eq!(
            report.diagnostic.as_ref().unwrap().to_string(),
            "warning: redefining 'print'"
        );
        assert_eq!(report.replaced.unwrap().changes_network, false);

        let descriptor = table.get("print").unwrap();
        assert_eq!((descriptor.handler)(), 2);
        assert!(descriptor.changes_network);
    }

    #[test]
    fn iteration_is_sorted_by_command_name() {
        let mut table = CommandTable::new();
        table.add_command("write", handler_a as fn() -> i32, false);
        table.add_command("alias", handler_a as fn() -> i32, false);
        table.add_command("quit", handler_a as fn() -> i32, false);

        let names = table
            .iter()
            .map(|descriptor| descriptor.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, ["alias", "quit", "write"]);
    }

    #[test]
    fn remove_drops_descriptor_from_table() {
        let mut table = CommandTable::new();
        table.add_command("undo", handler_a as fn() -> i32, false);

        let removed = table.remove("undo").unwrap();

        assert_eq!(removed.name, "undo");
        assert!(!table.contains("undo"));
        assert!(table.is_empty());
    }

    #[test]
    fn descriptor_insert_rejects_mismatched_key() {
        let mut table = CommandTable::new();
        let descriptor = CommandDescriptor::new("source", handler_a as fn() -> i32, false);

        assert_eq!(
            table.insert_descriptor("save", descriptor),
            Err(CommandTableError::NameMismatch {
                key: "save".to_owned(),
                descriptor_name: "source".to_owned()
            })
        );
    }
}
