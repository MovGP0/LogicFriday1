use std::collections::BTreeMap;

pub type CommandStatus = i32;

pub const SUCCESS: CommandStatus = 0;
pub const FAILURE: CommandStatus = 1;
pub const SET_USAGE: &str = "usage: set [name] [value]";
pub const UNSET_USAGE: &str = "usage: unset val1 val2 ...";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FlagStore {
    flags: BTreeMap<String, String>,
}

impl FlagStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }

    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.flags.insert(name.into(), value.into())
    }

    pub fn unset(&mut self, name: &str) -> Option<String> {
        self.flags.remove(name)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&str, &str)> {
        self.flags
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    pub fn len(&self) -> usize {
        self.flags.len()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SetCommandOptions {
    pub graphics_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlagAction {
    GraphicsSet { payload: String },
    ReopenOutput { target: String },
    ReopenError { target: String },
    DisableHistory,
    ReopenHistory { target: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlagCommandReport {
    pub status: CommandStatus,
    pub output: Vec<String>,
    pub diagnostics: Vec<String>,
    pub actions: Vec<FlagAction>,
    pub previous_value: Option<String>,
}

impl FlagCommandReport {
    fn success() -> Self {
        Self {
            status: SUCCESS,
            output: Vec::new(),
            diagnostics: Vec::new(),
            actions: Vec::new(),
            previous_value: None,
        }
    }

    fn failure(message: impl Into<String>) -> Self {
        Self {
            status: FAILURE,
            output: Vec::new(),
            diagnostics: vec![message.into()],
            actions: Vec::new(),
            previous_value: None,
        }
    }
}

pub fn set_variable<I, S>(
    flags: &mut FlagStore,
    argv: I,
    options: SetCommandOptions,
) -> FlagCommandReport
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let argv = argv
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();

    if argv.is_empty() || argv.len() > 3 {
        return FlagCommandReport::failure(SET_USAGE);
    }

    if argv.len() == 1 {
        let mut report = FlagCommandReport::success();
        report.output = flags
            .iter()
            .map(|(name, value)| format!("{name}\t{value}"))
            .collect();
        return report;
    }

    let name = &argv[1];
    let value = argv.get(2).cloned().unwrap_or_default();
    let previous_value = flags.set(name.clone(), value.clone());

    let mut report = FlagCommandReport::success();
    report.previous_value = previous_value;

    if options.graphics_enabled {
        report.actions.push(FlagAction::GraphicsSet {
            payload: format!("{name}\t{value}"),
        });
    }

    match name.as_str() {
        "sisout" | "misout" => report.actions.push(FlagAction::ReopenOutput {
            target: redirect_target(&value),
        }),
        "siserr" | "miserr" => report.actions.push(FlagAction::ReopenError {
            target: redirect_target(&value),
        }),
        "history" if value.is_empty() => report.actions.push(FlagAction::DisableHistory),
        "history" => report
            .actions
            .push(FlagAction::ReopenHistory { target: value }),
        _ => {}
    }

    report
}

pub fn unset_variable<I, S>(flags: &mut FlagStore, argv: I) -> FlagCommandReport
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let argv = argv
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();

    if argv.len() < 2 {
        return FlagCommandReport::failure(UNSET_USAGE);
    }

    for name in argv.iter().skip(1) {
        flags.unset(name);
    }

    FlagCommandReport::success()
}

pub fn get_flag<'a>(flags: &'a FlagStore, name: &str) -> Option<&'a str> {
    flags.get(name)
}

fn redirect_target(value: &str) -> String {
    if value.is_empty() {
        "-".to_owned()
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_without_name_lists_flags_in_sorted_order() {
        let mut flags = FlagStore::new();
        flags.set("zeta", "last");
        flags.set("alpha", "first");

        let report = set_variable(&mut flags, ["set"], SetCommandOptions::default());

        assert_eq!(report.status, SUCCESS);
        assert_eq!(report.output, ["alpha\tfirst", "zeta\tlast"]);
        assert!(report.actions.is_empty());
    }

    #[test]
    fn set_rejects_missing_command_or_too_many_arguments() {
        let mut flags = FlagStore::new();

        let missing = set_variable(
            &mut flags,
            std::iter::empty::<&str>(),
            SetCommandOptions::default(),
        );
        let too_many = set_variable(
            &mut flags,
            ["set", "a", "b", "c"],
            SetCommandOptions::default(),
        );

        assert_eq!(missing.status, FAILURE);
        assert_eq!(missing.diagnostics, [SET_USAGE]);
        assert_eq!(too_many.status, FAILURE);
        assert_eq!(too_many.diagnostics, [SET_USAGE]);
        assert!(flags.is_empty());
    }

    #[test]
    fn set_name_without_value_stores_empty_value() {
        let mut flags = FlagStore::new();

        let report = set_variable(&mut flags, ["set", "prompt"], SetCommandOptions::default());

        assert_eq!(report.status, SUCCESS);
        assert_eq!(get_flag(&flags, "prompt"), Some(""));
        assert_eq!(report.previous_value, None);
    }

    #[test]
    fn set_replaces_existing_value_and_reports_previous_value() {
        let mut flags = FlagStore::new();
        flags.set("shell_char", "!");

        let report = set_variable(
            &mut flags,
            ["set", "shell_char", "$"],
            SetCommandOptions::default(),
        );

        assert_eq!(report.status, SUCCESS);
        assert_eq!(report.previous_value, Some("!".to_owned()));
        assert_eq!(get_flag(&flags, "shell_char"), Some("$"));
    }

    #[test]
    fn graphics_enabled_records_legacy_payload() {
        let mut flags = FlagStore::new();

        let report = set_variable(
            &mut flags,
            ["set", "open_path", ".:/tmp"],
            SetCommandOptions {
                graphics_enabled: true,
            },
        );

        assert_eq!(
            report.actions,
            [FlagAction::GraphicsSet {
                payload: "open_path\t.:/tmp".to_owned()
            }]
        );
    }

    #[test]
    fn output_and_error_redirect_empty_values_to_dash() {
        let mut flags = FlagStore::new();

        let output = set_variable(&mut flags, ["set", "sisout"], SetCommandOptions::default());
        let error = set_variable(&mut flags, ["set", "miserr"], SetCommandOptions::default());

        assert_eq!(
            output.actions,
            [FlagAction::ReopenOutput {
                target: "-".to_owned()
            }]
        );
        assert_eq!(
            error.actions,
            [FlagAction::ReopenError {
                target: "-".to_owned()
            }]
        );
    }

    #[test]
    fn history_empty_disables_history_and_nonempty_reopens_it() {
        let mut flags = FlagStore::new();

        let disabled = set_variable(&mut flags, ["set", "history"], SetCommandOptions::default());
        let reopened = set_variable(
            &mut flags,
            ["set", "history", "sis.hist"],
            SetCommandOptions::default(),
        );

        assert_eq!(disabled.actions, [FlagAction::DisableHistory]);
        assert_eq!(
            reopened.actions,
            [FlagAction::ReopenHistory {
                target: "sis.hist".to_owned()
            }]
        );
    }

    #[test]
    fn unset_requires_at_least_one_name_and_ignores_missing_flags() {
        let mut flags = FlagStore::new();
        flags.set("a", "1");
        flags.set("b", "2");

        let missing = unset_variable(&mut flags, ["unset"]);
        let removed = unset_variable(&mut flags, ["unset", "a", "missing"]);

        assert_eq!(missing.status, FAILURE);
        assert_eq!(missing.diagnostics, [UNSET_USAGE]);
        assert_eq!(removed.status, SUCCESS);
        assert_eq!(get_flag(&flags, "a"), None);
        assert_eq!(get_flag(&flags, "b"), Some("2"));
    }

    #[test]
    fn source_has_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("cmd_set.rs");

        assert!(!source.contains(&format!("{}_{}", "no", "mangle")));
        assert!(!source.contains(&format!("{} \"{}\"", "extern", "C")));
        assert!(!source.contains(&format!("{}_", "REQUIRED")));
        assert!(!source.contains(&format!("{}-{}", "LogicFriday1", "8j8")));
    }
}
