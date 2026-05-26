//! Native Rust command-driver model for `LogicSynthesis/sis/main/sis.c`.
//!
//! The C entry point owns process setup, option parsing, initial rc-file
//! sourcing, batch command execution, interactive command completion, and
//! shutdown decisions. This module keeps those decisions explicit and testable
//! without adding a process-level entry point or a per-file ABI shim.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

pub const DEFAULT_SIS_VERSION: &str = "SIS - Version 1.2";
pub const DEFAULT_SIS_LIBRARY: &str = "/projects/sis/sis/common/sis_lib";

pub const COMMANDS: &[&str] = &[
    "act_map",
    "add_inverter",
    "alias",
    "astg_add_state",
    "astg_contract",
    "astg_current",
    "astg_encode",
    "astg_lockgraph",
    "astg_marking",
    "astg_persist",
    "astg_print_sg",
    "astg_print_statastg_slow",
    "astg_state_min",
    "astg_stg_scr",
    "astg_syn",
    "astg_to_f",
    "astg_to_stg",
    "atpg",
    "bdsyn",
    "buffer_opt",
    "c_check",
    "c_opt",
    "chng_clock",
    "chng_name",
    "collapse",
    "constraints",
    "decomp",
    "echo",
    "eliminate",
    "env_seq_dc",
    "env_verify_fsm",
    "equiv_nets",
    "espresso",
    "extract_seq_dc",
    "factor",
    "fanout_alg",
    "fanout_param",
    "force_init_0",
    "free_dc",
    "full_simplify",
    "fx",
    "gcx",
    "gkx",
    "help",
    "history",
    "invert",
    "invert_io",
    "ite_map",
    "latch_output",
    "map",
    "one_hot",
    "phase",
    "power_estimate",
    "power_free_info",
    "power_print",
    "print",
    "print_altname",
    "print_clock",
    "print_delay",
    "print_factor",
    "print_gate",
    "print_io",
    "print_kernel",
    "print_latch",
    "print_level",
    "print_library",
    "print_map_statsprint_state",
    "print_stats",
    "print_value",
    "quit",
    "read_astg",
    "read_blif",
    "read_eqn",
    "read_kiss",
    "read_library",
    "read_pla",
    "read_slif",
    "red_removal",
    "reduce_depth",
    "remove_dep",
    "remove_latches",
    "replace",
    "reset_name",
    "resub",
    "retime",
    "save",
    "set",
    "set_delay",
    "set_state",
    "short_tests",
    "sim_verify",
    "simplify",
    "simulate",
    "source",
    "speed_up",
    "speedup_alg",
    "state_assign",
    "state_minimize",
    "stg_cover",
    "stg_extract",
    "stg_to_astg",
    "stg_to_network",
    "sweep",
    "tech_decomp",
    "time",
    "timeout",
    "unalias",
    "undo",
    "unset",
    "usage",
    "verify",
    "verify_fsm",
    "wd",
    "write_astg",
    "write_bdnet",
    "write_blif",
    "write_eqn",
    "write_kiss",
    "write_pds",
    "write_pla",
    "write_slif",
    "xl_absorb",
    "xl_ao",
    "xl_coll_ck",
    "xl_cover",
    "xl_decomp_two",
    "xl_imp",
    "xl_k_decomp",
    "xl_merge",
    "xl_part_coll",
    "xl_partition",
    "xl_rl",
    "xl_split",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SisAction {
    InitializeSis,
    AllocateNetwork,
    PrintVersion(String),
    WarnTrailingArguments,
    Source(String),
    Read { command: String, file: String },
    Execute(String),
    Write { command: String, file: String },
    InteractivePrompt { prompt: String },
    FreeNetwork,
    EndSis,
    FreeCommandHistory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisMainPlan {
    pub mode: SisMode,
    pub status: i32,
    pub actions: Vec<SisAction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SisMode {
    Interactive,
    Batch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SisFileType {
    Bdnet,
    Blif,
    Eqn,
    Kiss,
    Oct,
    Pla,
    Slif,
    None,
}

impl SisFileType {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "bdnet" => Some(Self::Bdnet),
            "blif" => Some(Self::Blif),
            "eqn" => Some(Self::Eqn),
            "kiss" => Some(Self::Kiss),
            "oct" => Some(Self::Oct),
            "pla" => Some(Self::Pla),
            "slif" => Some(Self::Slif),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    pub fn read_command(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Bdnet => Some("read_bdnet"),
            Self::Blif => Some("read_blif"),
            Self::Eqn => Some("read_eqn"),
            Self::Kiss => Some("read_kiss"),
            Self::Oct => Some("read_oct"),
            Self::Pla => Some("read_pla"),
            Self::Slif => Some("read_slif"),
        }
    }

    pub fn write_command(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Bdnet => Some("write_bdnet"),
            Self::Blif => Some("write_blif"),
            Self::Eqn => Some("write_eqn"),
            Self::Kiss => Some("write_kiss"),
            Self::Oct => Some("write_oct"),
            Self::Pla => Some("write_pla"),
            Self::Slif => Some("write_slif"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisMainOptions {
    pub program_name: String,
    pub command: String,
    pub read_command: Option<String>,
    pub write_command: Option<String>,
    pub input_file: String,
    pub output_file: String,
    pub initial_source: bool,
    pub batch: bool,
}

impl Default for SisMainOptions {
    fn default() -> Self {
        Self {
            program_name: "sis".to_string(),
            command: String::new(),
            read_command: Some("read_blif".to_string()),
            write_command: Some("write_blif".to_string()),
            input_file: "-".to_string(),
            output_file: "-".to_string(),
            initial_source: true,
            batch: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SisMainError {
    MissingOptionValue { option: String },
    UnknownOption(String),
    UnknownType(String),
    TooManyInputFiles,
}

impl fmt::Display for SisMainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue { option } => write!(f, "option {option} requires a value"),
            Self::UnknownOption(option) => write!(f, "unknown option {option}"),
            Self::UnknownType(value) => write!(f, "unknown type {value}"),
            Self::TooManyInputFiles => write!(f, "too many input files"),
        }
    }
}

impl Error for SisMainError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RcFileState {
    pub home_misrc: Option<PathBuf>,
    pub current_misrc: Option<PathBuf>,
    pub home_sisrc: Option<PathBuf>,
    pub current_sisrc: Option<PathBuf>,
}

impl RcFileState {
    pub fn empty() -> Self {
        Self {
            home_misrc: None,
            current_misrc: None,
            home_sisrc: None,
            current_sisrc: None,
        }
    }
}

pub fn command_name_completion(prefix: &str) -> Vec<&'static str> {
    COMMANDS
        .iter()
        .copied()
        .filter(|command| command.starts_with(prefix))
        .collect()
}

pub fn check_type(value: &str) -> bool {
    SisFileType::parse(value).is_some()
}

pub fn usage(program_name: &str, version: &str, library: &str) -> String {
    [
        version.to_string(),
        format!(
            "usage: {program_name} [-sx] [-c cmd] [-f script] [-o file] [-t type] [-T type] [file]"
        ),
        "    -c cmd\texecute SIS commands `cmd'".to_string(),
        "    -f file\texecute SIS commands from a file".to_string(),
        "    -o file\tspecify output filename (default is -)".to_string(),
        format!("    -s\t\tsuppress initial 'source {library}/.sisrc'"),
        "    -t type\tspecify input type (blif, eqn, kiss, oct, pla, slif, or none)".to_string(),
        "    -T type\tspecify output type (blif, eqn, kiss, oct, pla, slif, or none)".to_string(),
        "    -x\t\tequivalent to '-t none -T none'".to_string(),
    ]
    .join("\n")
}

pub fn parse_main_options<I, S>(args: I) -> Result<SisMainOptions, SisMainError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let mut options = SisMainOptions::default();

    if let Some(program_name) = args.next() {
        options.program_name = program_name;
    }

    let mut operands = Vec::new();
    while let Some(arg) = args.next() {
        if !arg.starts_with('-') || arg == "-" {
            operands.push(arg);
            continue;
        }

        if arg == "--" {
            operands.extend(args);
            break;
        }

        let mut chars = arg[1..].chars().peekable();
        while let Some(option) = chars.next() {
            match option {
                'c' | 'f' | 'o' | 't' | 'T' | 'X' => {
                    let inline: String = chars.collect();
                    let value = if inline.is_empty() {
                        args.next()
                            .ok_or_else(|| SisMainError::MissingOptionValue {
                                option: format!("-{option}"),
                            })?
                    } else {
                        inline
                    };

                    apply_value_option(&mut options, option, value)?;
                    break;
                }
                's' => {
                    options.initial_source = false;
                }
                'x' => {
                    options.read_command = None;
                    options.write_command = None;
                    options.batch = true;
                }
                _ => {
                    return Err(SisMainError::UnknownOption(format!("-{option}")));
                }
            }
        }
    }

    match operands.as_slice() {
        [] => {}
        [input] => {
            options.input_file = input.clone();
        }
        _ => {
            return Err(SisMainError::TooManyInputFiles);
        }
    }

    Ok(options)
}

pub fn source_sisrc_plan(library: &str, state: &RcFileState) -> Vec<SisAction> {
    let mut actions = vec![
        SisAction::Source(format!("{library}/.misrc")),
        SisAction::Source(format!("{library}/.sisrc")),
        SisAction::Source("~/.sisrc".to_string()),
    ];

    push_home_current_sources(
        &mut actions,
        "misrc",
        "~/.misrc",
        ".misrc",
        state.home_misrc.as_deref(),
        state.current_misrc.as_deref(),
    );

    push_home_current_sources(
        &mut actions,
        "sisrc",
        "~/.sisrc",
        ".sisrc",
        state.home_sisrc.as_deref(),
        state.current_sisrc.as_deref(),
    );

    actions
}

pub fn plan_main(options: SisMainOptions, library: &str, rc_files: &RcFileState) -> SisMainPlan {
    let mut actions = vec![SisAction::InitializeSis, SisAction::AllocateNetwork];

    if options.batch {
        if options.initial_source {
            actions.extend(source_sisrc_plan(library, rc_files));
        }

        if let Some(read_command) = options.read_command {
            actions.push(SisAction::Read {
                command: read_command,
                file: options.input_file,
            });
        }

        actions.push(SisAction::Execute(options.command));

        if let Some(write_command) = options.write_command {
            actions.push(SisAction::Write {
                command: write_command,
                file: options.output_file,
            });
        }

        actions.push(SisAction::FreeCommandHistory);
        SisMainPlan {
            mode: SisMode::Batch,
            status: 0,
            actions,
        }
    } else {
        if options.input_file != "-" {
            actions.push(SisAction::WarnTrailingArguments);
        }

        actions.push(SisAction::PrintVersion(DEFAULT_SIS_VERSION.to_string()));
        if options.initial_source {
            actions.extend(source_sisrc_plan(library, rc_files));
        }
        actions.push(SisAction::InteractivePrompt {
            prompt: "sis> ".to_string(),
        });
        actions.push(SisAction::FreeCommandHistory);

        SisMainPlan {
            mode: SisMode::Interactive,
            status: 0,
            actions,
        }
    }
}

pub fn finish_for_quit_flag(plan: &mut SisMainPlan, quit_flag: i32, command_status: i32) {
    plan.status = if quit_flag == -1 || quit_flag == -2 {
        0
    } else {
        command_status
    };

    if quit_flag == -2 {
        plan.actions.push(SisAction::FreeNetwork);
        plan.actions.push(SisAction::EndSis);
    }
}

fn apply_value_option(
    options: &mut SisMainOptions,
    option: char,
    value: String,
) -> Result<(), SisMainError> {
    match option {
        'c' => {
            options.command = value;
            options.batch = true;
        }
        'f' => {
            options.command = format!("source {value}");
            options.batch = true;
        }
        'o' => {
            options.output_file = value;
        }
        't' => {
            let file_type =
                SisFileType::parse(&value).ok_or_else(|| SisMainError::UnknownType(value))?;
            options.read_command = file_type.read_command().map(str::to_string);
            options.batch = true;
        }
        'T' => {
            let file_type =
                SisFileType::parse(&value).ok_or_else(|| SisMainError::UnknownType(value))?;
            options.write_command = file_type.write_command().map(str::to_string);
            options.batch = true;
        }
        'X' => {}
        _ => unreachable!("caller filters valid value options"),
    }

    Ok(())
}

fn push_home_current_sources(
    actions: &mut Vec<SisAction>,
    name: &str,
    home_command: &str,
    current_command: &str,
    home: Option<&Path>,
    current: Option<&Path>,
) {
    match (home, current) {
        (Some(home), Some(current)) if same_file_identity(home, current) => {
            actions.push(SisAction::Source(home_command.to_string()));
        }
        (Some(_), Some(_)) => {
            actions.push(SisAction::Source(home_command.to_string()));
            actions.push(SisAction::Source(current_command.to_string()));
        }
        (Some(_), None) => {
            actions.push(SisAction::Source(home_command.to_string()));
        }
        (None, Some(_)) => {
            actions.push(SisAction::Source(current_command.to_string()));
        }
        (None, None) => {
            let _ = name;
        }
    }
}

fn same_file_identity(left: &Path, right: &Path) -> bool {
    normalize_path(left) == normalize_path(right)
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

pub fn unique_commands() -> BTreeSet<&'static str> {
    COMMANDS.iter().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_rc_files() -> RcFileState {
        RcFileState::empty()
    }

    #[test]
    fn command_completion_matches_prefix_in_c_order() {
        assert_eq!(
            command_name_completion("read_"),
            vec![
                "read_astg",
                "read_blif",
                "read_eqn",
                "read_kiss",
                "read_library",
                "read_pla",
                "read_slif"
            ]
        );

        assert_eq!(command_name_completion("xl_m"), vec!["xl_merge"]);
    }

    #[test]
    fn file_type_check_accepts_sis_types_and_rejects_unknown_values() {
        for value in ["bdnet", "blif", "eqn", "kiss", "oct", "pla", "slif", "none"] {
            assert!(check_type(value), "{value}");
        }

        assert!(!check_type("verilog"));
    }

    #[test]
    fn parse_defaults_to_interactive_blif_stdin_stdout() {
        let options = parse_main_options(["sis"]).unwrap();

        assert_eq!(options, SisMainOptions::default());
    }

    #[test]
    fn parse_command_file_and_output_options_match_c_behavior() {
        let options =
            parse_main_options(["sis", "-f", "run.scr", "-oout.blif", "in.blif"]).unwrap();

        assert!(options.batch);
        assert_eq!(options.command, "source run.scr");
        assert_eq!(options.output_file, "out.blif");
        assert_eq!(options.input_file, "in.blif");
        assert_eq!(options.read_command.as_deref(), Some("read_blif"));
        assert_eq!(options.write_command.as_deref(), Some("write_blif"));
    }

    #[test]
    fn parse_types_and_x_option_control_read_write() {
        let typed = parse_main_options(["sis", "-t", "pla", "-Tslif", "input.pla"]).unwrap();
        assert_eq!(typed.read_command.as_deref(), Some("read_pla"));
        assert_eq!(typed.write_command.as_deref(), Some("write_slif"));

        let no_io = parse_main_options(["sis", "-x", "-c", "print_stats"]).unwrap();
        assert_eq!(no_io.read_command, None);
        assert_eq!(no_io.write_command, None);
        assert_eq!(no_io.command, "print_stats");
    }

    #[test]
    fn parse_reports_usage_errors() {
        assert_eq!(
            parse_main_options(["sis", "-t", "verilog"]).unwrap_err(),
            SisMainError::UnknownType("verilog".to_string())
        );
        assert_eq!(
            parse_main_options(["sis", "-c"]).unwrap_err(),
            SisMainError::MissingOptionValue {
                option: "-c".to_string()
            }
        );
        assert_eq!(
            parse_main_options(["sis", "-c", "help", "a.blif", "b.blif"]).unwrap_err(),
            SisMainError::TooManyInputFiles
        );
    }

    #[test]
    fn source_plan_always_sources_library_files_and_default_home_sisrc() {
        let plan = source_sisrc_plan("/lib/sis", &no_rc_files());

        assert_eq!(
            plan,
            vec![
                SisAction::Source("/lib/sis/.misrc".to_string()),
                SisAction::Source("/lib/sis/.sisrc".to_string()),
                SisAction::Source("~/.sisrc".to_string()),
            ]
        );
    }

    #[test]
    fn source_plan_avoids_double_sourcing_same_home_and_current_file() {
        let state = RcFileState {
            home_misrc: Some(PathBuf::from("C:/home/.misrc")),
            current_misrc: Some(PathBuf::from("c:\\home\\.misrc")),
            home_sisrc: Some(PathBuf::from("C:/home/.sisrc")),
            current_sisrc: Some(PathBuf::from("D:/repo/.sisrc")),
        };

        let plan = source_sisrc_plan("/lib/sis", &state);

        assert_eq!(
            plan,
            vec![
                SisAction::Source("/lib/sis/.misrc".to_string()),
                SisAction::Source("/lib/sis/.sisrc".to_string()),
                SisAction::Source("~/.sisrc".to_string()),
                SisAction::Source("~/.misrc".to_string()),
                SisAction::Source("~/.sisrc".to_string()),
                SisAction::Source(".sisrc".to_string()),
            ]
        );
    }

    #[test]
    fn interactive_plan_prints_version_sources_rc_and_prompts() {
        let plan = plan_main(
            parse_main_options(["sis"]).unwrap(),
            "/lib/sis",
            &no_rc_files(),
        );

        assert_eq!(plan.mode, SisMode::Interactive);
        assert_eq!(
            plan.actions,
            vec![
                SisAction::InitializeSis,
                SisAction::AllocateNetwork,
                SisAction::PrintVersion(DEFAULT_SIS_VERSION.to_string()),
                SisAction::Source("/lib/sis/.misrc".to_string()),
                SisAction::Source("/lib/sis/.sisrc".to_string()),
                SisAction::Source("~/.sisrc".to_string()),
                SisAction::InteractivePrompt {
                    prompt: "sis> ".to_string()
                },
                SisAction::FreeCommandHistory,
            ]
        );
    }

    #[test]
    fn interactive_plan_warns_about_trailing_operand() {
        let plan = plan_main(
            parse_main_options(["sis", "ignored.blif"]).unwrap(),
            "/lib/sis",
            &no_rc_files(),
        );

        assert!(plan.actions.contains(&SisAction::WarnTrailingArguments));
    }

    #[test]
    fn batch_plan_sources_reads_executes_and_writes_in_order() {
        let options = parse_main_options([
            "sis",
            "-c",
            "sweep; print_stats",
            "-t",
            "pla",
            "-T",
            "eqn",
            "in.pla",
        ])
        .unwrap();
        let plan = plan_main(options, "/lib/sis", &no_rc_files());

        assert_eq!(plan.mode, SisMode::Batch);
        assert_eq!(
            plan.actions,
            vec![
                SisAction::InitializeSis,
                SisAction::AllocateNetwork,
                SisAction::Source("/lib/sis/.misrc".to_string()),
                SisAction::Source("/lib/sis/.sisrc".to_string()),
                SisAction::Source("~/.sisrc".to_string()),
                SisAction::Read {
                    command: "read_pla".to_string(),
                    file: "in.pla".to_string()
                },
                SisAction::Execute("sweep; print_stats".to_string()),
                SisAction::Write {
                    command: "write_eqn".to_string(),
                    file: "-".to_string()
                },
                SisAction::FreeCommandHistory,
            ]
        );
    }

    #[test]
    fn batch_plan_respects_s_and_none_io_options() {
        let options = parse_main_options(["sis", "-s", "-x", "-c", "quit"]).unwrap();
        let plan = plan_main(options, "/lib/sis", &no_rc_files());

        assert_eq!(
            plan.actions,
            vec![
                SisAction::InitializeSis,
                SisAction::AllocateNetwork,
                SisAction::Execute("quit".to_string()),
                SisAction::FreeCommandHistory,
            ]
        );
    }

    #[test]
    fn quit_minus_two_requests_network_free_and_shutdown() {
        let mut plan = plan_main(
            parse_main_options(["sis", "-c", "quit"]).unwrap(),
            "/lib/sis",
            &no_rc_files(),
        );

        finish_for_quit_flag(&mut plan, -2, 7);

        assert_eq!(plan.status, 0);
        assert_eq!(plan.actions[plan.actions.len() - 2], SisAction::FreeNetwork);
        assert_eq!(plan.actions[plan.actions.len() - 1], SisAction::EndSis);
    }

    #[test]
    fn usage_mentions_sis_options() {
        let text = usage("sis", "SIS v", "/cad/lib");

        assert!(text.contains("usage: sis [-sx]"));
        assert!(text.contains("execute SIS commands"));
        assert!(text.contains("source /cad/lib/.sisrc"));
        assert!(text.contains("blif, eqn, kiss, oct, pla, slif, or none"));
    }

    #[test]
    fn command_table_has_no_duplicates() {
        assert_eq!(unique_commands().len(), COMMANDS.len());
    }
}
