//! Native Rust command planning for the SIS PLD command module.
//!
//! The original C module registers PLD/ACT/Xilinx commands, parses command-line
//! options, toggles a small set of global flags, and dispatches into many SIS
//! network algorithms. This port keeps that deterministic command surface on
//! owned Rust data. Live command-table registration and `network_t` mutation are
//! represented by an explicit backend trait so callers get runtime diagnostics
//! until those integration ports are available.

use std::error::Error;
use std::fmt;

pub const AREA_MODE: f64 = 0.0;
pub const DELAY_MODE: f64 = 1.0;
pub const MAX_FANINS_CAP: i32 = 31;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PldCommandKind {
    ActMap,
    XlPartition,
    XlMerge,
    XlCover,
    XlKDecomp,
    XlSplit,
    XlNodeValue,
    XlCountNets,
    XlEstimateClb,
    XlDoClb,
    XlImprove,
    XlReduceLevels,
    XlAo,
    XlDecompTwo,
    XlPartialCollapse,
    XlAbsorb,
    XlCollapseCheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: PldCommandKind,
    pub changes_network: bool,
}

pub const PLD_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "act_map",
        kind: PldCommandKind::ActMap,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_partition",
        kind: PldCommandKind::XlPartition,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_merge",
        kind: PldCommandKind::XlMerge,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_cover",
        kind: PldCommandKind::XlCover,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_k_decomp",
        kind: PldCommandKind::XlKDecomp,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_split",
        kind: PldCommandKind::XlSplit,
        changes_network: true,
    },
    CommandRegistration {
        name: "_xl_nodevalue",
        kind: PldCommandKind::XlNodeValue,
        changes_network: false,
    },
    CommandRegistration {
        name: "_xl_cnets",
        kind: PldCommandKind::XlCountNets,
        changes_network: false,
    },
    CommandRegistration {
        name: "_xl_clb",
        kind: PldCommandKind::XlEstimateClb,
        changes_network: false,
    },
    CommandRegistration {
        name: "_xl_do_clb",
        kind: PldCommandKind::XlDoClb,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_imp",
        kind: PldCommandKind::XlImprove,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_rl",
        kind: PldCommandKind::XlReduceLevels,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_ao",
        kind: PldCommandKind::XlAo,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_decomp_two",
        kind: PldCommandKind::XlDecompTwo,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_part_coll",
        kind: PldCommandKind::XlPartialCollapse,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_absorb",
        kind: PldCommandKind::XlAbsorb,
        changes_network: true,
    },
    CommandRegistration {
        name: "xl_coll_ck",
        kind: PldCommandKind::XlCollapseCheck,
        changes_network: true,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecompMergeHeuristic {
    AllCubes,
    PairNodes,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActMapOptions {
    pub heuristic_num: i32,
    pub max_iterations: i32,
    pub fanin_collapse: i32,
    pub decomp_fanin: i32,
    pub gain_factor: f64,
    pub max_optimal: i32,
    pub mode: f64,
    pub bdnet_file: Option<String>,
    pub delay_file: Option<String>,
    pub quick_phase: bool,
    pub ignore_or_gate: bool,
    pub last_gasp: bool,
    pub disjoint_decomp: bool,
    pub statistics: bool,
    pub debug: bool,
}

impl Default for ActMapOptions {
    fn default() -> Self {
        Self {
            heuristic_num: 2,
            max_iterations: 0,
            fanin_collapse: 3,
            decomp_fanin: 4,
            gain_factor: 0.01,
            max_optimal: 6,
            mode: AREA_MODE,
            bdnet_file: None,
            delay_file: None,
            quick_phase: false,
            ignore_or_gate: false,
            last_gasp: false,
            disjoint_decomp: false,
            statistics: false,
            debug: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeOptions {
    pub max_fanin: i32,
    pub max_common_fanin: i32,
    pub max_union_fanin: i32,
    pub support: i32,
    pub output_file: String,
    pub verbose: bool,
    pub use_lindo: bool,
    pub final_collapse_after_merge: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoverOptions {
    pub support: i32,
    pub heuristic: i32,
    pub exact_node_limit: i32,
    pub heuristic_node_limit_upper: i32,
    pub debug: bool,
}

impl Default for CoverOptions {
    fn default() -> Self {
        Self {
            support: 5,
            heuristic: 3,
            exact_node_limit: 30,
            heuristic_node_limit_upper: 200,
            debug: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PartitionOptions {
    pub support: i32,
    pub move_fanins: bool,
    pub max_fanins: i32,
    pub trivial_collapse_only: bool,
    pub debug: i32,
}

impl Default for PartitionOptions {
    fn default() -> Self {
        Self {
            support: 5,
            move_fanins: false,
            max_fanins: 15,
            trivial_collapse_only: false,
            debug: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KDecompOptions {
    pub support: i32,
    pub node_name: Option<String>,
    pub max_fanins_k_decomp: i32,
    pub exhaustive: bool,
    pub recursive: bool,
    pub desperate: bool,
    pub debug: i32,
}

impl Default for KDecompOptions {
    fn default() -> Self {
        Self {
            support: 5,
            node_name: None,
            max_fanins_k_decomp: 7,
            exhaustive: false,
            recursive: false,
            desperate: true,
            debug: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SplitOptions {
    pub size: i32,
    pub debug: bool,
}

impl Default for SplitOptions {
    fn default() -> Self {
        Self {
            size: 5,
            debug: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XlnImproveOptions {
    pub support: i32,
    pub cover_node_limit: i32,
    pub lit_bound: i32,
    pub flag_decomp_good: i32,
    pub good_or_fast: i32,
    pub absorb: bool,
    pub recursive: bool,
    pub desperate: bool,
    pub max_fanins_k_decomp: i32,
    pub move_fanins: bool,
    pub max_fanins: i32,
    pub debug: i32,
    pub use_best: bool,
}

impl Default for XlnImproveOptions {
    fn default() -> Self {
        Self {
            support: 5,
            cover_node_limit: 25,
            lit_bound: 50,
            flag_decomp_good: 0,
            good_or_fast: 0,
            absorb: true,
            recursive: false,
            desperate: true,
            max_fanins_k_decomp: 7,
            move_fanins: false,
            max_fanins: 15,
            debug: 0,
            use_best: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReduceLevelsOptions {
    pub support: i32,
    pub heuristic: i32,
    pub max_fanins: i32,
    pub move_fanins: bool,
    pub bound_alphas: i32,
    pub collapse_input_limit: i32,
    pub traversal_method: i32,
    pub debug: i32,
}

impl Default for ReduceLevelsOptions {
    fn default() -> Self {
        Self {
            support: 5,
            heuristic: 1,
            max_fanins: 15,
            move_fanins: true,
            bound_alphas: 1,
            collapse_input_limit: 10,
            traversal_method: 1,
            debug: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PartialCollapseOptions {
    pub support: i32,
    pub cost_limit: i32,
    pub cover_node_limit: i32,
    pub lit_bound: i32,
    pub flag_decomp_good: i32,
    pub good_or_fast: i32,
    pub absorb: bool,
    pub move_fanins: bool,
    pub max_fanins: i32,
    pub debug: i32,
    pub use_best: bool,
}

impl Default for PartialCollapseOptions {
    fn default() -> Self {
        Self {
            support: 5,
            cost_limit: 1,
            cover_node_limit: 15,
            lit_bound: 50,
            flag_decomp_good: 0,
            good_or_fast: 1,
            absorb: false,
            move_fanins: false,
            max_fanins: 15,
            debug: 0,
            use_best: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecompForMergingOptions {
    pub max_fanin: i32,
    pub max_common_fanin: i32,
    pub max_union_fanin: i32,
    pub support: i32,
    pub heuristic: DecompMergeHeuristic,
    pub common_lower_bound: i32,
    pub cube_support_lower_bound: i32,
    pub debug: bool,
}

impl Default for DecompForMergingOptions {
    fn default() -> Self {
        Self {
            max_fanin: 4,
            max_common_fanin: 4,
            max_union_fanin: 5,
            support: 5,
            heuristic: DecompMergeHeuristic::AllCubes,
            common_lower_bound: 2,
            cube_support_lower_bound: 4,
            debug: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbsorbOptions {
    pub support: i32,
    pub method: i32,
    pub max_fanins: i32,
    pub debug: bool,
}

impl Default for AbsorbOptions {
    fn default() -> Self {
        Self {
            support: 5,
            method: 1,
            max_fanins: 15,
            debug: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollapseCheckOptions {
    pub support: i32,
    pub collapse_input_limit: i32,
    pub use_roth_karp: bool,
    pub debug: bool,
}

impl Default for CollapseCheckOptions {
    fn default() -> Self {
        Self {
            support: 5,
            collapse_input_limit: 9,
            use_roth_karp: true,
            debug: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PldCommandPlan {
    ActMap(ActMapOptions),
    Merge(MergeOptions),
    Cover(CoverOptions),
    NodeValue { fanin_limit: i32 },
    Partition(PartitionOptions),
    CountNets,
    EstimateClb { max_inputs: i32 },
    AndOrMap { size: i32 },
    KDecomp(KDecompOptions),
    Split(SplitOptions),
    XlnAo { support: i32, debug: i32 },
    XlnImprove(XlnImproveOptions),
    ReduceLevels(ReduceLevelsOptions),
    PartialCollapse(PartialCollapseOptions),
    DecompForMerging(DecompForMergingOptions),
    Absorb(AbsorbOptions),
    CollapseCheck(CollapseCheckOptions),
}

impl PldCommandPlan {
    pub fn kind(&self) -> PldCommandKind {
        match self {
            Self::ActMap(_) => PldCommandKind::ActMap,
            Self::Merge(_) => PldCommandKind::XlMerge,
            Self::Cover(_) => PldCommandKind::XlCover,
            Self::NodeValue { .. } => PldCommandKind::XlNodeValue,
            Self::Partition(_) => PldCommandKind::XlPartition,
            Self::CountNets => PldCommandKind::XlCountNets,
            Self::EstimateClb { .. } => PldCommandKind::XlEstimateClb,
            Self::AndOrMap { .. } => PldCommandKind::XlDoClb,
            Self::KDecomp(_) => PldCommandKind::XlKDecomp,
            Self::Split(_) => PldCommandKind::XlSplit,
            Self::XlnAo { .. } => PldCommandKind::XlAo,
            Self::XlnImprove(_) => PldCommandKind::XlImprove,
            Self::ReduceLevels(_) => PldCommandKind::XlReduceLevels,
            Self::PartialCollapse(_) => PldCommandKind::XlPartialCollapse,
            Self::DecompForMerging(_) => PldCommandKind::XlDecompTwo,
            Self::Absorb(_) => PldCommandKind::XlAbsorb,
            Self::CollapseCheck(_) => PldCommandKind::XlCollapseCheck,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PldCommandParseError {
    UnknownCommand(String),
    MissingOptionValue(char),
    UnsupportedOption(String),
    UnexpectedOperands {
        command: PldCommandKind,
        operands: Vec<String>,
    },
    MissingRequiredOption {
        command: PldCommandKind,
        option: char,
    },
    WrongArity {
        command: PldCommandKind,
        expected: usize,
        actual: usize,
    },
    InvalidValue {
        option: char,
        value: String,
        message: &'static str,
    },
    InvalidCombination(&'static str),
}

impl fmt::Display for PldCommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(f, "unknown PLD command {command}"),
            Self::MissingOptionValue(option) => write!(f, "missing value for -{option}"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::UnexpectedOperands { command, operands } => {
                write!(
                    f,
                    "command {command:?} does not accept operands: {operands:?}"
                )
            }
            Self::MissingRequiredOption { command, option } => {
                write!(f, "command {command:?} requires -{option}")
            }
            Self::WrongArity {
                command,
                expected,
                actual,
            } => write!(
                f,
                "command {command:?} expects {expected} argument(s), got {actual}"
            ),
            Self::InvalidValue {
                option,
                value,
                message,
            } => write!(f, "invalid value {value:?} for -{option}: {message}"),
            Self::InvalidCombination(message) => f.write_str(message),
        }
    }
}

impl Error for PldCommandParseError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PldOperation {
    RegisterCommands,
    InitIte,
    EndIte,
    Execute(PldCommandKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PldCommandError {
    MissingNativePorts { operation: PldOperation },
    Backend(String),
}

impl fmt::Display for PldCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
            Self::Backend(message) => f.write_str(message),
        }
    }
}

impl Error for PldCommandError {}

pub trait PldCommandBackend {
    fn execute_pld_command(&mut self, plan: &PldCommandPlan) -> Result<i32, PldCommandError>;

    fn init_ite(&mut self) -> Result<(), PldCommandError>;

    fn end_ite(&mut self) -> Result<(), PldCommandError>;
}

#[derive(Default)]
pub struct MissingPldCommandBackend;

impl PldCommandBackend for MissingPldCommandBackend {
    fn execute_pld_command(&mut self, plan: &PldCommandPlan) -> Result<i32, PldCommandError> {
        Err(missing(PldOperation::Execute(plan.kind())))
    }

    fn init_ite(&mut self) -> Result<(), PldCommandError> {
        Err(missing(PldOperation::InitIte))
    }

    fn end_ite(&mut self) -> Result<(), PldCommandError> {
        Err(missing(PldOperation::EndIte))
    }
}

pub fn pld_command_registrations() -> &'static [CommandRegistration] {
    PLD_COMMANDS
}

pub fn register_pld_commands() -> Result<&'static [CommandRegistration], PldCommandError> {
    Err(missing(PldOperation::RegisterCommands))
}

pub fn init_pld<B>(backend: &mut B) -> Result<&'static [CommandRegistration], PldCommandError>
where
    B: PldCommandBackend,
{
    backend.init_ite()?;
    Ok(PLD_COMMANDS)
}

pub fn end_pld<B>(backend: &mut B) -> Result<(), PldCommandError>
where
    B: PldCommandBackend,
{
    backend.end_ite()
}

pub fn dispatch_pld_command<B>(
    backend: &mut B,
    plan: &PldCommandPlan,
) -> Result<i32, PldCommandError>
where
    B: PldCommandBackend,
{
    backend.execute_pld_command(plan)
}

pub fn execute_with_missing_dependencies(plan: &PldCommandPlan) -> Result<i32, PldCommandError> {
    dispatch_pld_command(&mut MissingPldCommandBackend, plan)
}

pub fn parse_pld_command<I, S>(
    command_name: &str,
    args: I,
) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name {
        "act_map" => parse_act_map_args(args),
        "xl_partition" => parse_partition_args(args),
        "xl_merge" => parse_merge_args(args),
        "xl_cover" => parse_cover_args(args),
        "xl_k_decomp" => parse_k_decomp_args(args),
        "xl_split" => parse_split_args(args),
        "_xl_nodevalue" => parse_node_value_args(args),
        "_xl_cnets" => parse_count_nets_args(args),
        "_xl_clb" => parse_estimate_clb_args(args),
        "_xl_do_clb" => parse_and_or_map_args(args),
        "xl_imp" => parse_xln_improve_args(args),
        "xl_rl" => parse_reduce_levels_args(args),
        "xl_ao" => parse_xln_ao_args(args),
        "xl_decomp_two" => parse_decomp_for_merging_args(args),
        "xl_part_coll" => parse_partial_collapse_args(args),
        "xl_absorb" => parse_absorb_args(args),
        "xl_coll_ck" => parse_collapse_check_args(args),
        _ => Err(PldCommandParseError::UnknownCommand(
            command_name.to_owned(),
        )),
    }
}

pub fn parse_act_map_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = ActMapOptions::default();
    let operands = parse_options(args, "M:h:n:f:d:g:r:m:i:oqDlsv", |option, value| {
        match option {
            'M' => options.max_optimal = c_atoi(&value),
            'h' => {
                options.heuristic_num = c_atoi(&value);
                validate_range(option, &value, options.heuristic_num, 1, 4)?;
            }
            'n' => {
                options.max_iterations = c_atoi(&value);
                validate_min(option, &value, options.max_iterations, 0)?;
            }
            'f' => {
                options.fanin_collapse = c_atoi(&value);
                validate_min(option, &value, options.fanin_collapse, 0)?;
            }
            'd' => {
                options.decomp_fanin = c_atoi(&value);
                validate_min(option, &value, options.decomp_fanin, 1)?;
            }
            'g' => {
                options.gain_factor = c_atof(&value);
                if !(0.0..=1.0).contains(&options.gain_factor) {
                    return Err(invalid(option, value, "expected value from 0.0 to 1.0"));
                }
            }
            'r' => options.bdnet_file = Some(value),
            'm' => {
                options.mode = c_atof(&value);
                if !(AREA_MODE..=DELAY_MODE).contains(&options.mode) {
                    return Err(invalid(option, value, "expected value from 0.0 to 1.0"));
                }
            }
            'i' => options.delay_file = Some(value),
            'q' => options.quick_phase = true,
            'o' => options.ignore_or_gate = true,
            'l' => options.last_gasp = true,
            'D' => options.disjoint_decomp = true,
            's' => options.statistics = true,
            'v' => {
                options.debug = true;
                options.statistics = true;
            }
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::ActMap, operands)?;
    if options.delay_file.is_none() && options.mode != AREA_MODE {
        return Err(PldCommandParseError::MissingRequiredOption {
            command: PldCommandKind::ActMap,
            option: 'i',
        });
    }
    Ok(PldCommandPlan::ActMap(options))
}

pub fn parse_merge_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut max_fanin = 4;
    let mut max_common_fanin = 4;
    let mut max_union_fanin = 5;
    let mut support = 5;
    let mut output_file = None;
    let mut verbose = false;
    let mut use_lindo = true;
    let mut final_collapse_after_merge = true;

    let operands = parse_options(args, "f:c:n:u:o:Flv", |option, value| {
        match option {
            'f' => {
                max_fanin = c_atoi(&value);
                validate_min(option, &value, max_fanin, 0)?;
            }
            'c' => {
                max_common_fanin = c_atoi(&value);
                validate_min(option, &value, max_common_fanin, 0)?;
            }
            'u' => {
                max_union_fanin = c_atoi(&value);
                validate_min(option, &value, max_union_fanin, 0)?;
            }
            'n' => {
                support = c_atoi(&value);
                validate_min(option, &value, support, 2)?;
            }
            'o' => output_file = Some(value),
            'F' => final_collapse_after_merge = false,
            'l' => use_lindo = false,
            'v' => verbose = true,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlMerge, operands)?;
    let output_file = output_file.ok_or(PldCommandParseError::MissingRequiredOption {
        command: PldCommandKind::XlMerge,
        option: 'o',
    })?;

    Ok(PldCommandPlan::Merge(MergeOptions {
        max_fanin,
        max_common_fanin,
        max_union_fanin,
        support,
        output_file,
        verbose,
        use_lindo,
        final_collapse_after_merge,
    }))
}

pub fn parse_cover_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = CoverOptions::default();
    let operands = parse_options(args, "n:h:e:u:d", |option, value| {
        match option {
            'n' => options.support = c_atoi(&value),
            'h' => options.heuristic = c_atoi(&value),
            'e' => options.exact_node_limit = c_atoi(&value),
            'u' => options.heuristic_node_limit_upper = c_atoi(&value),
            'd' => options.debug = true,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlCover, operands)?;
    Ok(PldCommandPlan::Cover(options))
}

pub fn parse_partition_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = PartitionOptions::default();
    let operands = parse_options(args, "M:n:v:mt", |option, value| {
        match option {
            'm' => options.move_fanins = true,
            'M' => options.max_fanins = cap_max_fanins(c_atoi(&value)),
            'n' => options.support = c_atoi(&value),
            't' => options.trivial_collapse_only = true,
            'v' => options.debug = c_atoi(&value),
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlPartition, operands)?;
    validate_min('n', &options.support.to_string(), options.support, 2)?;
    Ok(PldCommandPlan::Partition(options))
}

pub fn parse_node_value_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut value = -1;
    let _operands = parse_options(args, "v:", |option, option_value| {
        match option {
            'v' => value = c_atoi(&option_value),
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    validate_min('v', &value.to_string(), value, 1)?;
    Ok(PldCommandPlan::NodeValue { fanin_limit: value })
}

pub fn parse_count_nets_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let operands = parse_options(args, "", |option, _value| Err(unsupported(option)))?;
    require_no_operands(PldCommandKind::XlCountNets, operands)?;
    Ok(PldCommandPlan::CountNets)
}

pub fn parse_estimate_clb_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let operands = collect_operands(args)?;
    let max_inputs = parse_single_operand(PldCommandKind::XlEstimateClb, operands)?;
    Ok(PldCommandPlan::EstimateClb { max_inputs })
}

pub fn parse_and_or_map_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let operands = collect_operands(args)?;
    let size = parse_single_operand(PldCommandKind::XlDoClb, operands)?;
    Ok(PldCommandPlan::AndOrMap { size })
}

pub fn parse_k_decomp_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = KDecompOptions::default();
    let operands = parse_options(args, "n:f:p:v:der", |option, value| {
        match option {
            'n' => {
                options.support = c_atoi(&value);
                validate_min(option, &value, options.support, 2)?;
            }
            'f' => options.max_fanins_k_decomp = c_atoi(&value),
            'p' => options.node_name = Some(value),
            'v' => options.debug = c_atoi(&value),
            'd' => options.desperate = false,
            'e' => options.exhaustive = true,
            'r' => options.recursive = true,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlKDecomp, operands)?;
    if options.exhaustive && options.node_name.is_some() {
        return Err(PldCommandParseError::InvalidCombination(
            "xl_k_decomp cannot combine exhaustive mode with a single node",
        ));
    }
    Ok(PldCommandPlan::KDecomp(options))
}

pub fn parse_split_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = SplitOptions::default();
    let operands = parse_options(args, "n:d", |option, value| {
        match option {
            'n' => options.size = c_atoi(&value),
            'd' => options.debug = true,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlSplit, operands)?;
    validate_min('n', &options.size.to_string(), options.size, 1)?;
    Ok(PldCommandPlan::Split(options))
}

pub fn parse_xln_ao_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut support = 5;
    let mut debug = 0;
    let operands = parse_options(args, "n:v:", |option, value| {
        match option {
            'n' => {
                support = c_atoi(&value);
                validate_min(option, &value, support, 2)?;
            }
            'v' => debug = c_atoi(&value),
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlAo, operands)?;
    Ok(PldCommandPlan::XlnAo { support, debug })
}

pub fn parse_xln_improve_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = XlnImproveOptions::default();
    let operands = parse_options(args, "M:n:c:f:g:l:v:Aabdmr", |option, value| {
        match option {
            'M' => options.max_fanins = cap_max_fanins(c_atoi(&value)),
            'n' => {
                options.support = c_atoi(&value);
                validate_min(option, &value, options.support, 2)?;
            }
            'c' => options.cover_node_limit = c_atoi(&value),
            'f' => options.max_fanins_k_decomp = c_atoi(&value),
            'g' => options.flag_decomp_good = c_atoi(&value),
            'l' => options.lit_bound = c_atoi(&value),
            'v' => options.debug = c_atoi(&value),
            'A' => options.absorb = false,
            'a' => options.good_or_fast = 1,
            'b' => options.use_best = true,
            'd' => options.desperate = false,
            'm' => options.move_fanins = true,
            'r' => options.recursive = true,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlImprove, operands)?;
    Ok(PldCommandPlan::XlnImprove(options))
}

pub fn parse_reduce_levels_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = ReduceLevelsOptions::default();
    let operands = parse_options(args, "c:n:v:h:M:A:mt", |option, value| {
        match option {
            'c' => options.collapse_input_limit = c_atoi(&value),
            'n' => {
                options.support = c_atoi(&value);
                validate_min(option, &value, options.support, 2)?;
            }
            'v' => options.debug = c_atoi(&value),
            'h' => options.heuristic = c_atoi(&value),
            'M' => options.max_fanins = c_atoi(&value),
            'A' => options.bound_alphas = c_atoi(&value),
            'm' => options.move_fanins = false,
            't' => options.traversal_method = 0,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlReduceLevels, operands)?;
    Ok(PldCommandPlan::ReduceLevels(options))
}

pub fn parse_partial_collapse_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = PartialCollapseOptions::default();
    let operands = parse_options(args, "M:n:C:v:c:g:l:bamA", |option, value| {
        match option {
            'M' => options.max_fanins = cap_max_fanins(c_atoi(&value)),
            'n' => {
                options.support = c_atoi(&value);
                validate_min(option, &value, options.support, 2)?;
            }
            'C' => options.cost_limit = c_atoi(&value),
            'v' => options.debug = c_atoi(&value),
            'c' => options.cover_node_limit = c_atoi(&value),
            'g' => options.flag_decomp_good = c_atoi(&value),
            'l' => options.lit_bound = c_atoi(&value),
            'b' => options.use_best = true,
            'a' => options.good_or_fast = 1,
            'm' => options.move_fanins = true,
            'A' => options.absorb = true,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlPartialCollapse, operands)?;
    Ok(PldCommandPlan::PartialCollapse(options))
}

pub fn parse_decomp_for_merging_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = DecompForMergingOptions::default();
    let operands = parse_options(args, "f:c:n:u:h:l:L:v", |option, value| {
        match option {
            'f' => {
                options.max_fanin = c_atoi(&value);
                validate_min(option, &value, options.max_fanin, 0)?;
            }
            'c' => {
                options.max_common_fanin = c_atoi(&value);
                validate_min(option, &value, options.max_common_fanin, 0)?;
            }
            'u' => {
                options.max_union_fanin = c_atoi(&value);
                validate_min(option, &value, options.max_union_fanin, 0)?;
            }
            'n' => {
                options.support = c_atoi(&value);
                validate_min(option, &value, options.support, 0)?;
            }
            'h' => {
                options.heuristic = match c_atoi(&value) {
                    1 => DecompMergeHeuristic::AllCubes,
                    2 => DecompMergeHeuristic::PairNodes,
                    _ => return Err(invalid(option, value, "expected 1 or 2")),
                };
            }
            'l' => options.common_lower_bound = c_atoi(&value),
            'L' => options.cube_support_lower_bound = c_atoi(&value),
            'v' => options.debug = true,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlDecompTwo, operands)?;
    Ok(PldCommandPlan::DecompForMerging(options))
}

pub fn parse_absorb_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = AbsorbOptions::default();
    let operands = parse_options(args, "f:m:n:v", |option, value| {
        match option {
            'f' => options.max_fanins = cap_max_fanins(c_atoi(&value)),
            'm' => {
                options.method = c_atoi(&value);
                validate_range(option, &value, options.method, 0, 1)?;
            }
            'n' => {
                options.support = c_atoi(&value);
                validate_min(option, &value, options.support, 2)?;
            }
            'v' => options.debug = true,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlAbsorb, operands)?;
    Ok(PldCommandPlan::Absorb(options))
}

pub fn parse_collapse_check_args<I, S>(args: I) -> Result<PldCommandPlan, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = CollapseCheckOptions::default();
    let operands = parse_options(args, "c:n:kv", |option, value| {
        match option {
            'c' => options.collapse_input_limit = c_atoi(&value),
            'n' => {
                options.support = c_atoi(&value);
                validate_min(option, &value, options.support, 2)?;
            }
            'k' => options.use_roth_karp = false,
            'v' => options.debug = true,
            _ => return Err(unsupported(option)),
        }
        Ok(())
    })?;
    require_no_operands(PldCommandKind::XlCollapseCheck, operands)?;
    Ok(PldCommandPlan::CollapseCheck(options))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActVertex {
    pub mark: bool,
    pub index: usize,
    pub index_size: usize,
    pub low: Option<usize>,
    pub high: Option<usize>,
}

impl ActVertex {
    pub fn terminal(mark: bool, index_size: usize) -> Self {
        Self {
            mark,
            index: index_size,
            index_size,
            low: None,
            high: None,
        }
    }

    pub fn branch(mark: bool, index: usize, index_size: usize, low: usize, high: usize) -> Self {
        Self {
            mark,
            index,
            index_size,
            low: Some(low),
            high: Some(high),
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.index == self.index_size
    }
}

pub fn traverse_act_vertices<F>(
    vertices: &mut [ActVertex],
    root: usize,
    mut manipulate: F,
) -> Result<(), TraverseError>
where
    F: FnMut(usize, &mut ActVertex),
{
    traverse_act_vertices_inner(vertices, root, &mut manipulate)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraverseError {
    UnknownVertex(usize),
    MissingLowChild(usize),
    MissingHighChild(usize),
}

impl fmt::Display for TraverseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVertex(index) => write!(f, "unknown ACT vertex {index}"),
            Self::MissingLowChild(index) => write!(f, "ACT vertex {index} has no low child"),
            Self::MissingHighChild(index) => write!(f, "ACT vertex {index} has no high child"),
        }
    }
}

impl Error for TraverseError {}

fn traverse_act_vertices_inner<F>(
    vertices: &mut [ActVertex],
    index: usize,
    manipulate: &mut F,
) -> Result<(), TraverseError>
where
    F: FnMut(usize, &mut ActVertex),
{
    let vertex = vertices
        .get_mut(index)
        .ok_or(TraverseError::UnknownVertex(index))?;
    vertex.mark = !vertex.mark;
    let mark = vertex.mark;
    manipulate(index, vertex);

    if vertices[index].is_terminal() {
        return Ok(());
    }

    let low = vertices[index]
        .low
        .ok_or(TraverseError::MissingLowChild(index))?;
    if mark
        != vertices
            .get(low)
            .ok_or(TraverseError::UnknownVertex(low))?
            .mark
    {
        traverse_act_vertices_inner(vertices, low, manipulate)?;
    }

    let high = vertices[index]
        .high
        .ok_or(TraverseError::MissingHighChild(index))?;
    if mark
        != vertices
            .get(high)
            .ok_or(TraverseError::UnknownVertex(high))?
            .mark
    {
        traverse_act_vertices_inner(vertices, high, manipulate)?;
    }

    Ok(())
}

fn parse_options<F>(
    args: impl IntoIterator<Item = impl AsRef<str>>,
    spec: &str,
    mut apply: F,
) -> Result<Vec<String>, PldCommandParseError>
where
    F: FnMut(char, String) -> Result<(), PldCommandParseError>,
{
    let mut iter = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .peekable();
    let mut operands = Vec::new();

    while let Some(arg) = iter.next() {
        if !arg.starts_with('-') || arg == "-" {
            operands.push(arg);
            operands.extend(iter);
            break;
        }
        if arg == "--" {
            operands.extend(iter);
            break;
        }

        let mut chars = arg[1..].char_indices().peekable();
        while let Some((offset, option)) = chars.next() {
            let needs_value = option_needs_value(spec, option)
                .ok_or_else(|| PldCommandParseError::UnsupportedOption(format!("-{option}")))?;
            if needs_value {
                let value_start = offset + option.len_utf8();
                let value = if value_start < arg[1..].len() {
                    arg[1 + value_start..].to_owned()
                } else {
                    iter.next()
                        .ok_or(PldCommandParseError::MissingOptionValue(option))?
                };
                apply(option, value)?;
                break;
            }
            apply(option, String::new())?;
        }
    }

    Ok(operands)
}

fn option_needs_value(spec: &str, option: char) -> Option<bool> {
    let mut chars = spec.chars().peekable();
    while let Some(candidate) = chars.next() {
        if candidate == ':' {
            continue;
        }
        let has_value = chars.peek() == Some(&':');
        if candidate == option {
            return Some(has_value);
        }
    }
    None
}

fn collect_operands<I, S>(args: I) -> Result<Vec<String>, PldCommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_options(args, "", |option, _value| Err(unsupported(option)))
}

fn parse_single_operand(
    command: PldCommandKind,
    operands: Vec<String>,
) -> Result<i32, PldCommandParseError> {
    if operands.len() != 1 {
        return Err(PldCommandParseError::WrongArity {
            command,
            expected: 1,
            actual: operands.len(),
        });
    }
    Ok(c_atoi(&operands[0]))
}

fn require_no_operands(
    command: PldCommandKind,
    operands: Vec<String>,
) -> Result<(), PldCommandParseError> {
    if !operands.is_empty() {
        return Err(PldCommandParseError::UnexpectedOperands { command, operands });
    }
    Ok(())
}

fn validate_min(
    option: char,
    value_text: &str,
    value: i32,
    min: i32,
) -> Result<(), PldCommandParseError> {
    if value < min {
        return Err(invalid(
            option,
            value_text.to_owned(),
            "value is below minimum",
        ));
    }
    Ok(())
}

fn validate_range(
    option: char,
    value_text: &str,
    value: i32,
    min: i32,
    max: i32,
) -> Result<(), PldCommandParseError> {
    if value < min || value > max {
        return Err(invalid(
            option,
            value_text.to_owned(),
            "value is outside range",
        ));
    }
    Ok(())
}

fn cap_max_fanins(value: i32) -> i32 {
    value.min(MAX_FANINS_CAP)
}

fn invalid(option: char, value: String, message: &'static str) -> PldCommandParseError {
    PldCommandParseError::InvalidValue {
        option,
        value,
        message,
    }
}

fn unsupported(option: char) -> PldCommandParseError {
    PldCommandParseError::UnsupportedOption(format!("-{option}"))
}

fn missing(operation: PldOperation) -> PldCommandError {
    PldCommandError::MissingNativePorts { operation }
}

fn c_atoi(value: &str) -> i32 {
    let trimmed = value.trim_start();
    let mut chars = trimmed.chars().peekable();
    let mut sign = 1;

    match chars.peek() {
        Some('-') => {
            sign = -1;
            chars.next();
        }
        Some('+') => {
            chars.next();
        }
        _ => {}
    }

    let mut result = 0_i32;
    let mut found_digit = false;
    while let Some(ch) = chars.peek().copied() {
        if let Some(digit) = ch.to_digit(10) {
            found_digit = true;
            result = result.saturating_mul(10).saturating_add(digit as i32);
            chars.next();
        } else {
            break;
        }
    }

    if found_digit {
        result.saturating_mul(sign)
    } else {
        0
    }
}

fn c_atof(value: &str) -> f64 {
    let trimmed = value.trim_start();
    let mut end = 0;
    let mut seen_digit = false;

    for (index, ch) in trimmed.char_indices() {
        let allowed = ch.is_ascii_digit()
            || matches!(ch, '+' | '-' | '.')
            || ((ch == 'e' || ch == 'E') && seen_digit);
        if !allowed {
            break;
        }
        if ch.is_ascii_digit() {
            seen_digit = true;
        }
        end = index + ch.len_utf8();
    }

    if !seen_digit {
        return 0.0;
    }

    trimmed[..end].parse::<f64>().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        executed: Vec<PldCommandKind>,
        init_count: usize,
        end_count: usize,
    }

    impl PldCommandBackend for RecordingBackend {
        fn execute_pld_command(&mut self, plan: &PldCommandPlan) -> Result<i32, PldCommandError> {
            self.executed.push(plan.kind());
            Ok(0)
        }

        fn init_ite(&mut self) -> Result<(), PldCommandError> {
            self.init_count += 1;
            Ok(())
        }

        fn end_ite(&mut self) -> Result<(), PldCommandError> {
            self.end_count += 1;
            Ok(())
        }
    }

    #[test]
    fn command_registrations_match_init_pld_table_order() {
        let names: Vec<_> = pld_command_registrations()
            .iter()
            .map(|registration| registration.name)
            .collect();

        assert_eq!(
            names,
            vec![
                "act_map",
                "xl_partition",
                "xl_merge",
                "xl_cover",
                "xl_k_decomp",
                "xl_split",
                "_xl_nodevalue",
                "_xl_cnets",
                "_xl_clb",
                "_xl_do_clb",
                "xl_imp",
                "xl_rl",
                "xl_ao",
                "xl_decomp_two",
                "xl_part_coll",
                "xl_absorb",
                "xl_coll_ck",
            ]
        );
    }

    #[test]
    fn parses_act_map_options_and_requires_delay_file_for_delay_mode() {
        let plan = parse_act_map_args([
            "-h",
            "4",
            "-m",
            "0.5",
            "-i",
            "delay.tbl",
            "-rbdnet.txt",
            "-qolDsv",
        ])
        .unwrap();

        assert_eq!(
            plan,
            PldCommandPlan::ActMap(ActMapOptions {
                heuristic_num: 4,
                mode: 0.5,
                delay_file: Some("delay.tbl".to_owned()),
                bdnet_file: Some("bdnet.txt".to_owned()),
                quick_phase: true,
                ignore_or_gate: true,
                last_gasp: true,
                disjoint_decomp: true,
                statistics: true,
                debug: true,
                ..ActMapOptions::default()
            })
        );
        assert_eq!(
            parse_act_map_args(["-m", "1.0"]).unwrap_err(),
            PldCommandParseError::MissingRequiredOption {
                command: PldCommandKind::ActMap,
                option: 'i',
            }
        );
    }

    #[test]
    fn parses_merge_required_output_and_flags() {
        let plan = parse_merge_args([
            "-f", "8", "-c3", "-u", "6", "-n", "4", "-o", "m.out", "-lFv",
        ])
        .unwrap();

        assert_eq!(
            plan,
            PldCommandPlan::Merge(MergeOptions {
                max_fanin: 8,
                max_common_fanin: 3,
                max_union_fanin: 6,
                support: 4,
                output_file: "m.out".to_owned(),
                verbose: true,
                use_lindo: false,
                final_collapse_after_merge: false,
            })
        );
        assert_eq!(
            parse_merge_args(["-f", "1"]).unwrap_err(),
            PldCommandParseError::MissingRequiredOption {
                command: PldCommandKind::XlMerge,
                option: 'o',
            }
        );
    }

    #[test]
    fn parses_partition_and_caps_move_fanin_limit() {
        assert_eq!(
            parse_partition_args(["-m", "-M", "99", "-n", "6", "-v", "2", "-t"]).unwrap(),
            PldCommandPlan::Partition(PartitionOptions {
                support: 6,
                move_fanins: true,
                max_fanins: 31,
                trivial_collapse_only: true,
                debug: 2,
            })
        );
    }

    #[test]
    fn parses_xln_family_option_defaults_and_validation() {
        assert_eq!(
            parse_xln_improve_args(["-n", "7", "-aAbdmr", "-M40"]).unwrap(),
            PldCommandPlan::XlnImprove(XlnImproveOptions {
                support: 7,
                good_or_fast: 1,
                absorb: false,
                use_best: true,
                desperate: false,
                move_fanins: true,
                recursive: true,
                max_fanins: 31,
                ..XlnImproveOptions::default()
            })
        );
        assert_eq!(
            parse_absorb_args(["-m", "2"]).unwrap_err(),
            PldCommandParseError::InvalidValue {
                option: 'm',
                value: "2".to_owned(),
                message: "value is outside range",
            }
        );
        assert_eq!(
            parse_decomp_for_merging_args(["-h", "2", "-v"]).unwrap(),
            PldCommandPlan::DecompForMerging(DecompForMergingOptions {
                heuristic: DecompMergeHeuristic::PairNodes,
                debug: true,
                ..DecompForMergingOptions::default()
            })
        );
    }

    #[test]
    fn parses_single_argument_utility_commands() {
        assert_eq!(
            parse_estimate_clb_args(["5"]).unwrap(),
            PldCommandPlan::EstimateClb { max_inputs: 5 }
        );
        assert_eq!(
            parse_and_or_map_args(["6"]).unwrap(),
            PldCommandPlan::AndOrMap { size: 6 }
        );
        assert_eq!(
            parse_count_nets_args(std::iter::empty::<&str>()).unwrap(),
            PldCommandPlan::CountNets
        );
    }

    #[test]
    fn k_decomp_rejects_exhaustive_single_node_combination() {
        assert_eq!(
            parse_k_decomp_args(["-e", "-p", "n1"]).unwrap_err(),
            PldCommandParseError::InvalidCombination(
                "xl_k_decomp cannot combine exhaustive mode with a single node",
            )
        );
    }

    #[test]
    fn dispatch_and_lifecycle_use_backend_trait() {
        let mut backend = RecordingBackend::default();
        let plan = PldCommandPlan::CountNets;

        assert_eq!(init_pld(&mut backend).unwrap(), PLD_COMMANDS);
        assert_eq!(dispatch_pld_command(&mut backend, &plan), Ok(0));
        assert_eq!(end_pld(&mut backend), Ok(()));
        assert_eq!(backend.init_count, 1);
        assert_eq!(backend.end_count, 1);
        assert_eq!(backend.executed, vec![PldCommandKind::XlCountNets]);
    }

    #[test]
    fn missing_backend_reports_generic_runtime_diagnostic() {
        assert_eq!(
            execute_with_missing_dependencies(&PldCommandPlan::CountNets),
            Err(PldCommandError::MissingNativePorts {
                operation: PldOperation::Execute(PldCommandKind::XlCountNets),
            })
        );
        assert_eq!(
            register_pld_commands(),
            Err(PldCommandError::MissingNativePorts {
                operation: PldOperation::RegisterCommands,
            })
        );
    }

    #[test]
    fn traverse_toggles_marks_and_skips_already_matching_children() {
        let mut vertices = vec![
            ActVertex::branch(false, 0, 2, 1, 2),
            ActVertex::terminal(false, 2),
            ActVertex::terminal(true, 2),
        ];
        let mut visited = Vec::new();

        traverse_act_vertices(&mut vertices, 0, |index, _vertex| visited.push(index)).unwrap();

        assert_eq!(visited, vec![0, 1]);
        assert_eq!(vertices[0].mark, true);
        assert_eq!(vertices[1].mark, true);
        assert_eq!(vertices[2].mark, true);
    }
}
