//! Native Rust command model for the SIS ntbdd command package.
//!
//! This module keeps the command registration metadata, argument parsing, and
//! deterministic dispatch planning for the legacy ntbdd commands in safe Rust.
//! The actual network and BDD algorithms are injected through a Rust backend so
//! this file stays free of per-file C ABI entry points.

use std::error::Error;
use std::fmt;

pub const MAX_TIMEOUT_SECONDS: u32 = 3600 * 24 * 365;

pub const BDD_TEST_USAGE: &str = "usage: _bdd_test [-v]\n";
pub const BDD_CREATE_USAGE: &str =
    "usage: bdd_create [-l] [-o ordering_style] [-t time_limit] node1 node2 ...\n";
pub const BDD_PRINT_USAGE: &str = "usage: bdd_print n1 n2 ...\n";
pub const BDD_SIZE_USAGE: &str = "usage: bdd_size n1 n2 ...\n";
pub const BDD_IMPLIES_USAGE: &str = "usage: bdd_implies n1 n2 [phase1 phase2]\n\tIf phase1 & phase2 are left out they are assumed both 1\n";
pub const BDD_COFACTOR_USAGE: &str =
    "_bdd_cofactor [-b] node_fn node_factor\n               -b: print as BDD\n";
pub const BDD_SMOOTH_USAGE: &str =
    "_bdd_smooth [-b] node_fn node_var1 node_var2 ...\n               -b: print as BDD\n";
pub const BDD_COMPOSE_USAGE: &str = "usage: _bdd_compose node_fn node_var node_replacement\n";
pub const BDD_VERIFY_USAGE: &str = "usage: _bdd_verify blif_file1 blif_file2\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NtbddCommandKind {
    Stats,
    Test,
    Create,
    Print,
    Size,
    Implies,
    Cofactor,
    Compose,
    Verify,
    Smooth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: NtbddCommandKind,
    pub changes_network: bool,
}

pub const NTBDD_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "_bdd_stats",
        kind: NtbddCommandKind::Stats,
        changes_network: false,
    },
    CommandRegistration {
        name: "_bdd_test",
        kind: NtbddCommandKind::Test,
        changes_network: false,
    },
    CommandRegistration {
        name: "_bdd_create",
        kind: NtbddCommandKind::Create,
        changes_network: false,
    },
    CommandRegistration {
        name: "_bdd_print",
        kind: NtbddCommandKind::Print,
        changes_network: false,
    },
    CommandRegistration {
        name: "_bdd_size",
        kind: NtbddCommandKind::Size,
        changes_network: false,
    },
    CommandRegistration {
        name: "_bdd_implies",
        kind: NtbddCommandKind::Implies,
        changes_network: false,
    },
    CommandRegistration {
        name: "_bdd_cofactor",
        kind: NtbddCommandKind::Cofactor,
        changes_network: true,
    },
    CommandRegistration {
        name: "_bdd_compose",
        kind: NtbddCommandKind::Compose,
        changes_network: false,
    },
    CommandRegistration {
        name: "_bdd_verify",
        kind: NtbddCommandKind::Verify,
        changes_network: false,
    },
    CommandRegistration {
        name: "_bdd_smooth",
        kind: NtbddCommandKind::Smooth,
        changes_network: false,
    },
];

pub fn ntbdd_command_registrations() -> &'static [CommandRegistration] {
    NTBDD_COMMANDS
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddLocality {
    Global,
    Local,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddOrdering {
    Dfs,
    Random,
}

impl BddOrdering {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "dfs" => Some(Self::Dfs),
            "random" => Some(Self::Random),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddCreatePlan {
    pub locality: BddLocality,
    pub ordering: BddOrdering,
    pub timeout_seconds: u32,
    pub nodes: Vec<String>,
}

impl Default for BddCreatePlan {
    fn default() -> Self {
        Self {
            locality: BddLocality::Global,
            ordering: BddOrdering::Dfs,
            timeout_seconds: 0,
            nodes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeListPlan {
    pub nodes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddImpliesPlan {
    pub left_node: String,
    pub right_node: String,
    pub left_phase: bool,
    pub right_phase: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddCofactorPlan {
    pub function_node: String,
    pub factor_node: String,
    pub print_as_network: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddSmoothPlan {
    pub function_node: String,
    pub smoothing_nodes: Vec<String>,
    pub print_as_network: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddComposePlan {
    pub function_node: String,
    pub variable_node: String,
    pub replacement_node: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddVerifyPlan {
    pub left_blif: String,
    pub right_blif: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NtbddCommand {
    Stats,
    Test { verbose: bool },
    Create(BddCreatePlan),
    Print(NodeListPlan),
    Size(NodeListPlan),
    Implies(BddImpliesPlan),
    Cofactor(BddCofactorPlan),
    Compose(BddComposePlan),
    Verify(BddVerifyPlan),
    Smooth(BddSmoothPlan),
}

impl NtbddCommand {
    pub fn kind(&self) -> NtbddCommandKind {
        match self {
            Self::Stats => NtbddCommandKind::Stats,
            Self::Test { .. } => NtbddCommandKind::Test,
            Self::Create(_) => NtbddCommandKind::Create,
            Self::Print(_) => NtbddCommandKind::Print,
            Self::Size(_) => NtbddCommandKind::Size,
            Self::Implies(_) => NtbddCommandKind::Implies,
            Self::Cofactor(_) => NtbddCommandKind::Cofactor,
            Self::Compose(_) => NtbddCommandKind::Compose,
            Self::Verify(_) => NtbddCommandKind::Verify,
            Self::Smooth(_) => NtbddCommandKind::Smooth,
        }
    }
}

pub trait NtbddBackend {
    fn init_ntbdd(&mut self) -> Result<(), NtbddError>;
    fn end_ntbdd(&mut self) -> Result<(), NtbddError>;
    fn execute_ntbdd_command(&mut self, command: &NtbddCommand) -> Result<i32, NtbddError>;
}

#[derive(Default)]
pub struct MissingNtbddBackend;

impl NtbddBackend for MissingNtbddBackend {
    fn init_ntbdd(&mut self) -> Result<(), NtbddError> {
        Err(NtbddError::RequiresNativeSisPort {
            operation: "init_ntbdd",
        })
    }

    fn end_ntbdd(&mut self) -> Result<(), NtbddError> {
        Ok(())
    }

    fn execute_ntbdd_command(&mut self, command: &NtbddCommand) -> Result<i32, NtbddError> {
        Err(NtbddError::RequiresNativeSisPort {
            operation: command.kind().operation_name(),
        })
    }
}

impl NtbddCommandKind {
    fn operation_name(self) -> &'static str {
        match self {
            Self::Stats => "_bdd_stats",
            Self::Test => "_bdd_test",
            Self::Create => "_bdd_create",
            Self::Print => "_bdd_print",
            Self::Size => "_bdd_size",
            Self::Implies => "_bdd_implies",
            Self::Cofactor => "_bdd_cofactor",
            Self::Compose => "_bdd_compose",
            Self::Verify => "_bdd_verify",
            Self::Smooth => "_bdd_smooth",
        }
    }
}

pub fn init_ntbdd<B>(backend: &mut B) -> Result<&'static [CommandRegistration], NtbddError>
where
    B: NtbddBackend,
{
    backend.init_ntbdd()?;
    Ok(ntbdd_command_registrations())
}

pub fn end_ntbdd<B>(backend: &mut B) -> Result<(), NtbddError>
where
    B: NtbddBackend,
{
    backend.end_ntbdd()
}

pub fn dispatch_ntbdd_command<B>(backend: &mut B, command: &NtbddCommand) -> Result<i32, NtbddError>
where
    B: NtbddBackend,
{
    backend.execute_ntbdd_command(command)
}

pub fn execute_with_missing_backend(command: &NtbddCommand) -> Result<i32, NtbddError> {
    dispatch_ntbdd_command(&mut MissingNtbddBackend, command)
}

pub fn parse_ntbdd_command<I, S>(command_name: &str, args: I) -> Result<NtbddCommand, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name {
        "_bdd_stats" => parse_bdd_stats_args(args),
        "_bdd_test" => parse_bdd_test_args(args),
        "_bdd_create" => parse_bdd_create_args(args).map(NtbddCommand::Create),
        "_bdd_print" => parse_node_list_args(args, BDD_PRINT_USAGE).map(NtbddCommand::Print),
        "_bdd_size" => parse_node_list_args(args, BDD_SIZE_USAGE).map(NtbddCommand::Size),
        "_bdd_implies" => parse_bdd_implies_args(args).map(NtbddCommand::Implies),
        "_bdd_cofactor" => parse_bdd_cofactor_args(args).map(NtbddCommand::Cofactor),
        "_bdd_compose" => parse_bdd_compose_args(args).map(NtbddCommand::Compose),
        "_bdd_verify" => parse_bdd_verify_args(args).map(NtbddCommand::Verify),
        "_bdd_smooth" => parse_bdd_smooth_args(args).map(NtbddCommand::Smooth),
        _ => Err(NtbddError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn parse_bdd_stats_args<I, S>(args: I) -> Result<NtbddCommand, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    if args.is_empty() {
        Ok(NtbddCommand::Stats)
    } else {
        Err(NtbddError::WrongArity {
            usage: "usage: _bdd_stats\n",
            actual: args.len(),
        })
    }
}

pub fn parse_bdd_test_args<I, S>(args: I) -> Result<NtbddCommand, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    let mut verbose = false;
    for arg in &args {
        match arg.as_str() {
            "-v" => verbose = true,
            _ => {
                return Err(NtbddError::Usage {
                    usage: BDD_TEST_USAGE,
                });
            }
        }
    }

    Ok(NtbddCommand::Test { verbose })
}

pub fn parse_bdd_create_args<I, S>(args: I) -> Result<BddCreatePlan, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    let mut plan = BddCreatePlan::default();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            plan.nodes.extend(args[index + 1..].iter().cloned());
            break;
        }
        if !arg.starts_with('-') || arg == "-" {
            plan.nodes.extend(args[index..].iter().cloned());
            break;
        }

        if arg == "-l" {
            plan.locality = BddLocality::Local;
        } else if arg == "-o" || arg.starts_with("-o") && arg.len() > 2 {
            let value = option_value(&args, &mut index, arg, 'o')?;
            plan.ordering = BddOrdering::parse(&value)
                .ok_or_else(|| NtbddError::UnknownOrdering(value.clone()))?;
        } else if arg == "-t" || arg.starts_with("-t") && arg.len() > 2 {
            let value = option_value(&args, &mut index, arg, 't')?;
            plan.timeout_seconds = parse_timeout(&value)?;
        } else {
            return Err(NtbddError::Usage {
                usage: BDD_CREATE_USAGE,
            });
        }

        index += 1;
    }

    if plan.nodes.is_empty() {
        return Err(NtbddError::Usage {
            usage: BDD_CREATE_USAGE,
        });
    }

    Ok(plan)
}

pub fn parse_node_list_args<I, S>(args: I, usage: &'static str) -> Result<NodeListPlan, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let nodes = collect_args(args);
    if nodes.is_empty() {
        Err(NtbddError::Usage { usage })
    } else {
        Ok(NodeListPlan { nodes })
    }
}

pub fn parse_bdd_implies_args<I, S>(args: I) -> Result<BddImpliesPlan, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    match args.as_slice() {
        [left_node, right_node] => Ok(BddImpliesPlan {
            left_node: left_node.clone(),
            right_node: right_node.clone(),
            left_phase: true,
            right_phase: true,
        }),
        [left_node, right_node, left_phase, right_phase] => Ok(BddImpliesPlan {
            left_node: left_node.clone(),
            right_node: right_node.clone(),
            left_phase: parse_phase(left_phase)?,
            right_phase: parse_phase(right_phase)?,
        }),
        _ => Err(NtbddError::Usage {
            usage: BDD_IMPLIES_USAGE,
        }),
    }
}

pub fn parse_bdd_cofactor_args<I, S>(args: I) -> Result<BddCofactorPlan, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let (print_as_network, operands) = parse_b_option(args, BDD_COFACTOR_USAGE)?;
    match operands.as_slice() {
        [function_node, factor_node] => Ok(BddCofactorPlan {
            function_node: function_node.clone(),
            factor_node: factor_node.clone(),
            print_as_network,
        }),
        _ => Err(NtbddError::Usage {
            usage: BDD_COFACTOR_USAGE,
        }),
    }
}

pub fn parse_bdd_smooth_args<I, S>(args: I) -> Result<BddSmoothPlan, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let (print_as_network, operands) = parse_b_option(args, BDD_SMOOTH_USAGE)?;
    if operands.len() < 2 {
        return Err(NtbddError::Usage {
            usage: BDD_SMOOTH_USAGE,
        });
    }

    Ok(BddSmoothPlan {
        function_node: operands[0].clone(),
        smoothing_nodes: operands[1..].to_vec(),
        print_as_network,
    })
}

pub fn parse_bdd_compose_args<I, S>(args: I) -> Result<BddComposePlan, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    match args.as_slice() {
        [function_node, variable_node, replacement_node] => Ok(BddComposePlan {
            function_node: function_node.clone(),
            variable_node: variable_node.clone(),
            replacement_node: replacement_node.clone(),
        }),
        _ => Err(NtbddError::Usage {
            usage: BDD_COMPOSE_USAGE,
        }),
    }
}

pub fn parse_bdd_verify_args<I, S>(args: I) -> Result<BddVerifyPlan, NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    match args.as_slice() {
        [left_blif, right_blif] => Ok(BddVerifyPlan {
            left_blif: left_blif.clone(),
            right_blif: right_blif.clone(),
        }),
        _ => Err(NtbddError::Usage {
            usage: BDD_VERIFY_USAGE,
        }),
    }
}

fn parse_b_option<I, S>(args: I, usage: &'static str) -> Result<(bool, Vec<String>), NtbddError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    let mut print_as_network = true;
    let mut operands = Vec::new();
    let mut scanning_options = true;

    for arg in args {
        if scanning_options && arg == "--" {
            scanning_options = false;
            continue;
        }
        if scanning_options && arg.starts_with('-') && arg != "-" {
            if arg == "-b" {
                print_as_network = false;
            } else {
                return Err(NtbddError::Usage { usage });
            }
        } else {
            scanning_options = false;
            operands.push(arg);
        }
    }

    Ok((print_as_network, operands))
}

fn collect_args<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect()
}

fn option_value(
    args: &[String],
    index: &mut usize,
    arg: &str,
    option: char,
) -> Result<String, NtbddError> {
    if arg.len() > 2 {
        Ok(arg[2..].to_owned())
    } else {
        *index += 1;
        args.get(*index)
            .cloned()
            .ok_or(NtbddError::MissingOptionValue(option))
    }
}

fn parse_timeout(value: &str) -> Result<u32, NtbddError> {
    let parsed = value
        .parse::<i64>()
        .map_err(|_| NtbddError::InvalidTimeout(value.to_owned()))?;
    if parsed < 0 || parsed > i64::from(MAX_TIMEOUT_SECONDS) {
        return Err(NtbddError::InvalidTimeout(value.to_owned()));
    }

    Ok(parsed as u32)
}

fn parse_phase(value: &str) -> Result<bool, NtbddError> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(NtbddError::InvalidPhase(value.to_owned())),
    }
}

pub fn format_implies_result(plan: &BddImpliesPlan, implies: bool) -> String {
    format!(
        "{} set to {} {} {} to {}\n",
        plan.left_node,
        i32::from(plan.left_phase),
        if implies { "forces" } else { "does not force" },
        plan.right_node,
        i32::from(plan.right_phase),
    )
}

pub fn format_verification_result(succeeds: bool) -> String {
    format!(
        "verification {}\n",
        if succeeds { "succeeds" } else { "fails" }
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NtbddError {
    UnknownCommand(String),
    Usage { usage: &'static str },
    WrongArity { usage: &'static str, actual: usize },
    MissingOptionValue(char),
    UnknownOrdering(String),
    InvalidTimeout(String),
    InvalidPhase(String),
    RequiresNativeSisPort { operation: &'static str },
}

impl fmt::Display for NtbddError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(f, "unknown ntbdd command {command}"),
            Self::Usage { usage } => write!(f, "{usage}"),
            Self::WrongArity { usage, actual } => {
                write!(f, "{usage}received {actual} argument(s)")
            }
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::UnknownOrdering(ordering) => write!(f, "unknown ordering method: {ordering}"),
            Self::InvalidTimeout(value) => write!(f, "invalid timeout value: {value}"),
            Self::InvalidPhase(value) => write!(f, "phase must be 0 or 1, got {value}"),
            Self::RequiresNativeSisPort { operation } => {
                write!(f, "{operation} requires native SIS network and BDD support")
            }
        }
    }
}

impl Error for NtbddError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        initialized: bool,
        ended: bool,
        executed: Vec<NtbddCommandKind>,
    }

    impl NtbddBackend for RecordingBackend {
        fn init_ntbdd(&mut self) -> Result<(), NtbddError> {
            self.initialized = true;
            Ok(())
        }

        fn end_ntbdd(&mut self) -> Result<(), NtbddError> {
            self.ended = true;
            Ok(())
        }

        fn execute_ntbdd_command(&mut self, command: &NtbddCommand) -> Result<i32, NtbddError> {
            self.executed.push(command.kind());
            Ok(0)
        }
    }

    #[test]
    fn command_table_matches_legacy_registration_order_and_mutation_flags() {
        let table = ntbdd_command_registrations()
            .iter()
            .map(|registration| (registration.name, registration.changes_network))
            .collect::<Vec<_>>();

        assert_eq!(
            table,
            vec![
                ("_bdd_stats", false),
                ("_bdd_test", false),
                ("_bdd_create", false),
                ("_bdd_print", false),
                ("_bdd_size", false),
                ("_bdd_implies", false),
                ("_bdd_cofactor", true),
                ("_bdd_compose", false),
                ("_bdd_verify", false),
                ("_bdd_smooth", false),
            ]
        );
    }

    #[test]
    fn create_parser_preserves_defaults_and_accepts_all_options() {
        let plan = parse_bdd_create_args(["-l", "-o", "random", "-t30", "n1", "n2"]).unwrap();

        assert_eq!(plan.locality, BddLocality::Local);
        assert_eq!(plan.ordering, BddOrdering::Random);
        assert_eq!(plan.timeout_seconds, 30);
        assert_eq!(plan.nodes, vec!["n1", "n2"]);

        let defaults = parse_bdd_create_args(["n1"]).unwrap();
        assert_eq!(defaults.locality, BddLocality::Global);
        assert_eq!(defaults.ordering, BddOrdering::Dfs);
        assert_eq!(defaults.timeout_seconds, 0);
    }

    #[test]
    fn create_parser_rejects_unknown_ordering_bad_timeout_and_empty_node_list() {
        assert_eq!(
            parse_bdd_create_args(["-o", "breadth", "n"]).unwrap_err(),
            NtbddError::UnknownOrdering("breadth".to_owned())
        );
        assert_eq!(
            parse_bdd_create_args(["-t", "31536001", "n"]).unwrap_err(),
            NtbddError::InvalidTimeout("31536001".to_owned())
        );
        assert_eq!(
            parse_bdd_create_args(["-l"]).unwrap_err(),
            NtbddError::Usage {
                usage: BDD_CREATE_USAGE,
            }
        );
    }

    #[test]
    fn bdd_test_and_node_list_parsers_enforce_legacy_usage() {
        assert_eq!(
            parse_bdd_test_args(["-v"]).unwrap(),
            NtbddCommand::Test { verbose: true }
        );
        assert_eq!(
            parse_bdd_test_args(["-x"]).unwrap_err(),
            NtbddError::Usage {
                usage: BDD_TEST_USAGE,
            }
        );
        assert_eq!(
            parse_node_list_args(["a", "b"], BDD_PRINT_USAGE).unwrap(),
            NodeListPlan {
                nodes: vec!["a".to_owned(), "b".to_owned()],
            }
        );
        assert_eq!(
            parse_node_list_args(std::iter::empty::<&str>(), BDD_SIZE_USAGE).unwrap_err(),
            NtbddError::Usage {
                usage: BDD_SIZE_USAGE,
            }
        );
    }

    #[test]
    fn implies_parser_defaults_phases_and_formats_result() {
        let plan = parse_bdd_implies_args(["n1", "n2"]).unwrap();
        assert_eq!(
            plan,
            BddImpliesPlan {
                left_node: "n1".to_owned(),
                right_node: "n2".to_owned(),
                left_phase: true,
                right_phase: true,
            }
        );
        assert_eq!(
            format_implies_result(&plan, true),
            "n1 set to 1 forces n2 to 1\n"
        );

        let inverted = parse_bdd_implies_args(["n1", "n2", "0", "1"]).unwrap();
        assert!(!inverted.left_phase);
        assert_eq!(
            parse_bdd_implies_args(["n1", "n2", "2", "1"]).unwrap_err(),
            NtbddError::InvalidPhase("2".to_owned())
        );
    }

    #[test]
    fn cofactor_and_smooth_parse_bdd_output_switch() {
        assert_eq!(
            parse_bdd_cofactor_args(["-b", "f", "x"]).unwrap(),
            BddCofactorPlan {
                function_node: "f".to_owned(),
                factor_node: "x".to_owned(),
                print_as_network: false,
            }
        );
        assert_eq!(
            parse_bdd_smooth_args(["f", "x", "y"]).unwrap(),
            BddSmoothPlan {
                function_node: "f".to_owned(),
                smoothing_nodes: vec!["x".to_owned(), "y".to_owned()],
                print_as_network: true,
            }
        );
        assert_eq!(
            parse_bdd_smooth_args(["f"]).unwrap_err(),
            NtbddError::Usage {
                usage: BDD_SMOOTH_USAGE,
            }
        );
    }

    #[test]
    fn compose_and_verify_require_exact_operands() {
        assert_eq!(
            parse_bdd_compose_args(["f", "x", "g"]).unwrap(),
            BddComposePlan {
                function_node: "f".to_owned(),
                variable_node: "x".to_owned(),
                replacement_node: "g".to_owned(),
            }
        );
        assert_eq!(
            parse_bdd_verify_args(["left.blif", "right.blif"]).unwrap(),
            BddVerifyPlan {
                left_blif: "left.blif".to_owned(),
                right_blif: "right.blif".to_owned(),
            }
        );
        assert_eq!(
            parse_bdd_verify_args(["left.blif"]).unwrap_err(),
            NtbddError::Usage {
                usage: BDD_VERIFY_USAGE,
            }
        );
    }

    #[test]
    fn parse_ntbdd_command_selects_command_variant() {
        assert!(matches!(
            parse_ntbdd_command("_bdd_stats", std::iter::empty::<&str>()).unwrap(),
            NtbddCommand::Stats
        ));
        assert!(matches!(
            parse_ntbdd_command("_bdd_create", ["n"]).unwrap(),
            NtbddCommand::Create(_)
        ));
        assert_eq!(
            parse_ntbdd_command("_missing", std::iter::empty::<&str>()).unwrap_err(),
            NtbddError::UnknownCommand("_missing".to_owned())
        );
    }

    #[test]
    fn backend_initialization_and_dispatch_are_plain_rust_calls() {
        let mut backend = RecordingBackend::default();
        let command = parse_ntbdd_command("_bdd_print", ["n1"]).unwrap();

        assert_eq!(
            init_ntbdd(&mut backend).unwrap(),
            ntbdd_command_registrations()
        );
        assert_eq!(dispatch_ntbdd_command(&mut backend, &command).unwrap(), 0);
        end_ntbdd(&mut backend).unwrap();

        assert!(backend.initialized);
        assert!(backend.ended);
        assert_eq!(backend.executed, vec![NtbddCommandKind::Print]);
    }

    #[test]
    fn missing_backend_reports_generic_native_requirement() {
        let command = parse_ntbdd_command("_bdd_verify", ["a.blif", "b.blif"]).unwrap();
        let error = execute_with_missing_backend(&command).unwrap_err();

        assert_eq!(
            error,
            NtbddError::RequiresNativeSisPort {
                operation: "_bdd_verify",
            }
        );
        assert!(
            error
                .to_string()
                .contains("requires native SIS network and BDD support")
        );
    }

    #[test]
    fn helper_output_keeps_legacy_verification_message() {
        assert_eq!(format_verification_result(true), "verification succeeds\n");
        assert_eq!(format_verification_result(false), "verification fails\n");
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_dependency_metadata_are_present_in_this_port() {
        let source = include_str!("com_ntbdd.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
