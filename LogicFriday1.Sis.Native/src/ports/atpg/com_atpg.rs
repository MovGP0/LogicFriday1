//! Native Rust command model for `LogicSynthesis/sis/atpg/com_atpg.c`.
//!
//! The C command registers ATPG-related SIS commands, parses ATPG options,
//! orchestrates setup, RTG, deterministic test generation, optional product
//! machine verification, reverse fault simulation, result printing, and cleanup.
//! The SIS network, SAT, BDD, sequence, and fault-simulation internals are
//! still separate native ports, so this module exposes deterministic command
//! intent and planning without adding legacy C ABI exports.

use std::error::Error;
use std::fmt;

pub const ATPG_USAGE: &str = concat!(
    "usage: atpg [-dfFhnrRptvy] [file]\n",
    "-d\tdepth of RTG sequences (default is STG depth)\n",
    "-f\tno fault simulation\n",
    "-F\tno reverse fault simulation\n",
    "-h\tuse fast SAT; no non-local implications\n",
    "-n\tnumber of sequences to fault simulate at one time\n",
    "\t(default is system word length; n must be less than this length)\n",
    "-r\tno RTG\n",
    "-R\tno random propagation\n",
    "-p\tno product machines, i.e. no fault-free propagation or \n\tgood/faulty PMT\n",
    "-t\tperform tech decomp of network\n",
    "-v\tverbosity\n",
    "-y\tlength of random prop sequences (default is 20)\n",
    "file\toutput file for test patterns\n",
);

pub const WORD_LENGTH: i32 = u32::BITS as i32;
pub const MAX_TIMEOUT_SECONDS: i32 = 3600 * 24 * 365;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtpgCommandKind
{
    Atpg,
    RedundancyRemoval,
    ShortTests,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtpgCommandRegistration
{
    pub name: &'static str,
    pub kind: AtpgCommandKind,
    pub changes_network: bool,
}

pub const ATPG_COMMANDS: &[AtpgCommandRegistration] = &[
    AtpgCommandRegistration {
        name: "atpg",
        kind: AtpgCommandKind::Atpg,
        changes_network: true,
    },
    AtpgCommandRegistration {
        name: "red_removal",
        kind: AtpgCommandKind::RedundancyRemoval,
        changes_network: true,
    },
    AtpgCommandRegistration {
        name: "short_tests",
        kind: AtpgCommandKind::ShortTests,
        changes_network: true,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgOptions
{
    pub quick_redundancy: bool,
    pub reverse_fault_simulation: bool,
    pub product_machine_only: bool,
    pub use_internal_states: bool,
    pub simulation_batch_size: i32,
    pub deterministic_propagation: bool,
    pub random_propagation: bool,
    pub rtg_depth: i32,
    pub random_propagation_depth: i32,
    pub random_propagation_iterations: i32,
    pub fault_simulation: bool,
    pub fast_sat: bool,
    pub random_test_generation: bool,
    pub build_product_machines: bool,
    pub technology_decomposition: bool,
    pub timeout_seconds: i32,
    pub verbosity: i32,
    pub print_sequences: bool,
    pub output_file: Option<String>,
    pub force_combinational: bool,
}

impl Default for AtpgOptions
{
    fn default() -> Self
    {
        Self {
            quick_redundancy: false,
            reverse_fault_simulation: true,
            product_machine_only: false,
            use_internal_states: false,
            simulation_batch_size: WORD_LENGTH,
            deterministic_propagation: true,
            random_propagation: true,
            rtg_depth: -1,
            random_propagation_depth: 20,
            random_propagation_iterations: 1,
            fault_simulation: true,
            fast_sat: false,
            random_test_generation: true,
            build_product_machines: true,
            technology_decomposition: false,
            timeout_seconds: 0,
            verbosity: 0,
            print_sequences: false,
            output_file: None,
            force_combinational: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtpgNetworkShape
{
    pub internal_nodes: usize,
    pub latches: usize,
    pub has_external_dont_cares: bool,
}

impl AtpgNetworkShape
{
    pub fn is_sequential(self) -> bool
    {
        self.latches != 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtpgAction
{
    NoNetwork,
    NoInternalNodes,
    InitializeInfo,
    ConfigureTimeout { seconds: i32 },
    TechnologyDecomposition,
    InitializeMainSimulationAndSat,
    InitializeSequenceInfo,
    GenerateFaults,
    InitializeExternalDontCareSimulation,
    SequentialSetup,
    ProductMachineSetup,
    RecordResetState,
    TraverseStateGraph,
    RandomTestGeneration { depth: i32 },
    CreateEmptyTestedFaultList,
    DeterministicFaultLoop,
    ProductMachineVerificationLoop,
    ReverseFaultSimulation,
    PrintAndDestroySequences,
    PrintResults,
    Cleanup,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgRunPlan
{
    pub options: AtpgOptions,
    pub sequential: bool,
    pub actions: Vec<AtpgAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtpgCommandError
{
    MissingOptionValue(char),
    UnsupportedOption(String),
    TooManyOutputFiles,
    SimulationBatchTooLarge { maximum: i32, supplied: i32 },
    InvalidTimeout(i32),
    Blocked { command: AtpgCommandKind },
}

impl fmt::Display for AtpgCommandError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::TooManyOutputFiles => write!(f, "atpg accepts at most one output file"),
            Self::SimulationBatchTooLarge { maximum, supplied } => write!(
                f,
                "atpg simulation batch size must be no greater than {maximum}, got {supplied}"
            ),
            Self::InvalidTimeout(seconds) => {
                write!(f, "atpg timeout must be between 0 and {MAX_TIMEOUT_SECONDS}, got {seconds}")
            }
            Self::Blocked { command } => {
                write!(f, "{command:?} requires native Rust ports for SIS ATPG dependencies")
            }
        }
    }
}

impl Error for AtpgCommandError {}

pub fn atpg_command_registrations() -> &'static [AtpgCommandRegistration]
{
    ATPG_COMMANDS
}

pub fn parse_atpg_args<I, S>(args: I) -> Result<AtpgOptions, AtpgCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = AtpgOptions::default();
    let mut iter = args.into_iter().map(|arg| arg.as_ref().to_owned()).peekable();
    let mut operands = Vec::new();
    let mut scanning_options = true;

    while let Some(arg) = iter.next() {
        if !scanning_options || !arg.starts_with('-') || arg == "-" {
            operands.push(arg);
            scanning_options = false;
            continue;
        }

        if arg == "--" {
            scanning_options = false;
            continue;
        }

        let mut flags = arg[1..].chars().peekable();
        while let Some(option) = flags.next() {
            match option {
                'c' => options.force_combinational = true,
                'D' => options.deterministic_propagation = false,
                'f' => options.fault_simulation = false,
                'F' => options.reverse_fault_simulation = false,
                'h' => options.fast_sat = true,
                'q' => {
                    options.quick_redundancy = true;
                    options.build_product_machines = false;
                }
                'r' => options.random_test_generation = false,
                'R' => options.random_propagation = false,
                'p' => options.build_product_machines = false,
                't' => options.technology_decomposition = true,
                'd' | 'n' | 'T' | 'v' | 'y' | 'z' => {
                    let inline: String = flags.collect();
                    let value = if inline.is_empty() {
                        iter.next().ok_or(AtpgCommandError::MissingOptionValue(option))?
                    } else {
                        inline
                    };
                    apply_value_option(&mut options, option, &value)?;
                    break;
                }
                _ => return Err(AtpgCommandError::UnsupportedOption(format!("-{option}"))),
            }
        }
    }

    match operands.as_slice() {
        [] => {}
        [output] => {
            options.print_sequences = true;
            options.output_file = Some(output.clone());
        }
        _ => return Err(AtpgCommandError::TooManyOutputFiles),
    }

    Ok(options)
}

pub fn plan_atpg(
    network: Option<AtpgNetworkShape>,
    mut options: AtpgOptions,
    stg_depth: Option<i32>,
) -> AtpgRunPlan
{
    let Some(shape) = network else {
        return AtpgRunPlan {
            options,
            sequential: false,
            actions: vec![AtpgAction::NoNetwork],
        };
    };

    if shape.internal_nodes == 0 {
        return AtpgRunPlan {
            options,
            sequential: shape.is_sequential(),
            actions: vec![AtpgAction::NoInternalNodes],
        };
    }

    let sequential = shape.is_sequential() && !options.force_combinational;
    let mut actions = vec![AtpgAction::InitializeInfo];

    if options.timeout_seconds > 0 {
        actions.push(AtpgAction::ConfigureTimeout {
            seconds: options.timeout_seconds,
        });
    }

    if options.technology_decomposition {
        actions.push(AtpgAction::TechnologyDecomposition);
    }

    actions.push(AtpgAction::InitializeMainSimulationAndSat);
    if sequential {
        actions.push(AtpgAction::InitializeSequenceInfo);
    }
    actions.push(AtpgAction::GenerateFaults);

    if !sequential && shape.has_external_dont_cares {
        actions.push(AtpgAction::InitializeExternalDontCareSimulation);
    }

    if sequential {
        actions.push(AtpgAction::SequentialSetup);
        if options.build_product_machines {
            actions.push(AtpgAction::ProductMachineSetup);
        }
        actions.push(AtpgAction::RecordResetState);
        actions.push(AtpgAction::TraverseStateGraph);
    }

    if options.random_test_generation {
        if sequential {
            if options.rtg_depth == -1 {
                options.rtg_depth = stg_depth.unwrap_or(0);
            }
        } else {
            options.rtg_depth = 1;
        }
        actions.push(AtpgAction::RandomTestGeneration {
            depth: options.rtg_depth,
        });
    } else {
        actions.push(AtpgAction::CreateEmptyTestedFaultList);
    }

    actions.push(AtpgAction::DeterministicFaultLoop);

    if sequential && options.build_product_machines {
        actions.push(AtpgAction::ProductMachineVerificationLoop);
    }

    if options.reverse_fault_simulation {
        actions.push(AtpgAction::ReverseFaultSimulation);
    }

    actions.push(AtpgAction::PrintAndDestroySequences);
    actions.push(AtpgAction::PrintResults);
    actions.push(AtpgAction::Cleanup);

    AtpgRunPlan {
        options,
        sequential,
        actions,
    }
}

pub fn execute_atpg_command<Network>(
    _network: &mut Network,
    command: AtpgCommandKind,
    _options: &AtpgOptions,
) -> Result<(), AtpgCommandError>
{
    Err(AtpgCommandError::Blocked { command })
}

fn apply_value_option(
    options: &mut AtpgOptions,
    option: char,
    value: &str,
) -> Result<(), AtpgCommandError>
{
    let parsed = c_atoi(value);
    match option {
        'd' => options.rtg_depth = parsed,
        'n' => {
            options.simulation_batch_size = parsed;
            if options.simulation_batch_size > WORD_LENGTH {
                return Err(AtpgCommandError::SimulationBatchTooLarge {
                    maximum: WORD_LENGTH,
                    supplied: options.simulation_batch_size,
                });
            }
        }
        'T' => {
            options.timeout_seconds = parsed;
            if !(0..=MAX_TIMEOUT_SECONDS).contains(&options.timeout_seconds) {
                return Err(AtpgCommandError::InvalidTimeout(options.timeout_seconds));
            }
        }
        'v' => options.verbosity = parsed,
        'y' => options.random_propagation_depth = parsed,
        'z' => options.random_propagation_iterations = parsed,
        _ => unreachable!("caller filters value options"),
    }

    Ok(())
}

fn c_atoi(value: &str) -> i32
{
    let value = value.trim_start();
    let mut end = 0usize;
    for (index, ch) in value.char_indices() {
        let accepted = if index == 0 {
            ch == '+' || ch == '-' || ch.is_ascii_digit()
        } else {
            ch.is_ascii_digit()
        };

        if !accepted {
            break;
        }

        end = index + ch.len_utf8();
    }

    if end == 0 || value[..end].chars().all(|ch| ch == '+' || ch == '-') {
        return 0;
    }

    value[..end].parse().unwrap_or(0)
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn combinational_network() -> AtpgNetworkShape
    {
        AtpgNetworkShape {
            internal_nodes: 4,
            latches: 0,
            has_external_dont_cares: true,
        }
    }

    fn sequential_network() -> AtpgNetworkShape
    {
        AtpgNetworkShape {
            internal_nodes: 6,
            latches: 2,
            has_external_dont_cares: true,
        }
    }

    #[test]
    fn command_registration_matches_init_atpg()
    {
        assert_eq!(
            atpg_command_registrations(),
            &[
                AtpgCommandRegistration {
                    name: "atpg",
                    kind: AtpgCommandKind::Atpg,
                    changes_network: true,
                },
                AtpgCommandRegistration {
                    name: "red_removal",
                    kind: AtpgCommandKind::RedundancyRemoval,
                    changes_network: true,
                },
                AtpgCommandRegistration {
                    name: "short_tests",
                    kind: AtpgCommandKind::ShortTests,
                    changes_network: true,
                },
            ]
        );
    }

    #[test]
    fn parses_atpg_defaults_and_output_file()
    {
        let defaults = parse_atpg_args(std::iter::empty::<&str>()).unwrap();
        assert_eq!(defaults, AtpgOptions::default());

        let with_output = parse_atpg_args(["patterns.out"]).unwrap();
        assert!(with_output.print_sequences);
        assert_eq!(with_output.output_file.as_deref(), Some("patterns.out"));
    }

    #[test]
    fn parses_atpg_flags_and_inline_values()
    {
        let options = parse_atpg_args([
            "-cDfFhqrRpt",
            "-d7",
            "-n",
            "8",
            "-T60",
            "-v2",
            "-y",
            "11",
            "-z3",
        ])
        .unwrap();

        assert!(options.force_combinational);
        assert!(!options.deterministic_propagation);
        assert!(!options.fault_simulation);
        assert!(!options.reverse_fault_simulation);
        assert!(options.fast_sat);
        assert!(options.quick_redundancy);
        assert!(!options.random_test_generation);
        assert!(!options.random_propagation);
        assert!(!options.build_product_machines);
        assert!(options.technology_decomposition);
        assert_eq!(options.rtg_depth, 7);
        assert_eq!(options.simulation_batch_size, 8);
        assert_eq!(options.timeout_seconds, 60);
        assert_eq!(options.verbosity, 2);
        assert_eq!(options.random_propagation_depth, 11);
        assert_eq!(options.random_propagation_iterations, 3);
    }

    #[test]
    fn parser_matches_c_atoi_and_usage_failures()
    {
        assert_eq!(parse_atpg_args(["-v", "loud"]).unwrap().verbosity, 0);
        assert_eq!(parse_atpg_args(["-d-2x"]).unwrap().rtg_depth, -2);

        assert_eq!(
            parse_atpg_args(["-n33"]),
            Err(AtpgCommandError::SimulationBatchTooLarge {
                maximum: WORD_LENGTH,
                supplied: WORD_LENGTH + 1,
            })
        );
        assert_eq!(
            parse_atpg_args(["-T", "-1"]),
            Err(AtpgCommandError::InvalidTimeout(-1))
        );
        assert_eq!(
            parse_atpg_args(["one.out", "two.out"]),
            Err(AtpgCommandError::TooManyOutputFiles)
        );
        assert_eq!(
            parse_atpg_args(["-x"]),
            Err(AtpgCommandError::UnsupportedOption("-x".to_owned()))
        );
        assert_eq!(
            parse_atpg_args(["-d"]),
            Err(AtpgCommandError::MissingOptionValue('d'))
        );
    }

    #[test]
    fn plan_returns_early_for_missing_or_trivial_networks()
    {
        assert_eq!(
            plan_atpg(None, AtpgOptions::default(), None).actions,
            vec![AtpgAction::NoNetwork]
        );
        assert_eq!(
            plan_atpg(
                Some(AtpgNetworkShape {
                    internal_nodes: 0,
                    latches: 1,
                    has_external_dont_cares: false,
                }),
                AtpgOptions::default(),
                None
            )
            .actions,
            vec![AtpgAction::NoInternalNodes]
        );
    }

    #[test]
    fn combinational_plan_uses_exdc_and_forces_rtg_depth_to_one()
    {
        let plan = plan_atpg(Some(combinational_network()), AtpgOptions::default(), None);

        assert!(!plan.sequential);
        assert_eq!(plan.options.rtg_depth, 1);
        assert!(plan.actions.contains(&AtpgAction::InitializeExternalDontCareSimulation));
        assert!(plan.actions.contains(&AtpgAction::RandomTestGeneration { depth: 1 }));
        assert!(!plan.actions.contains(&AtpgAction::TraverseStateGraph));
    }

    #[test]
    fn sequential_plan_uses_stg_depth_and_product_machine_verification()
    {
        let plan = plan_atpg(Some(sequential_network()), AtpgOptions::default(), Some(5));

        assert!(plan.sequential);
        assert_eq!(plan.options.rtg_depth, 5);
        assert!(plan.actions.contains(&AtpgAction::SequentialSetup));
        assert!(plan.actions.contains(&AtpgAction::ProductMachineSetup));
        assert!(plan.actions.contains(&AtpgAction::TraverseStateGraph));
        assert!(plan.actions.contains(&AtpgAction::ProductMachineVerificationLoop));
        assert!(!plan.actions.contains(&AtpgAction::InitializeExternalDontCareSimulation));
    }

    #[test]
    fn plan_respects_disabled_rtg_product_machines_and_reverse_sim()
    {
        let options = parse_atpg_args(["-r", "-p", "-F"]).unwrap();
        let plan = plan_atpg(Some(sequential_network()), options, Some(5));

        assert!(plan.actions.contains(&AtpgAction::CreateEmptyTestedFaultList));
        assert!(!plan.actions.contains(&AtpgAction::RandomTestGeneration { depth: 5 }));
        assert!(!plan.actions.contains(&AtpgAction::ProductMachineSetup));
        assert!(!plan.actions.contains(&AtpgAction::ProductMachineVerificationLoop));
        assert!(!plan.actions.contains(&AtpgAction::ReverseFaultSimulation));
    }

    #[test]
    fn force_combinational_ignores_sequential_setup()
    {
        let options = parse_atpg_args(["-c"]).unwrap();
        let plan = plan_atpg(Some(sequential_network()), options, Some(5));

        assert!(!plan.sequential);
        assert_eq!(plan.options.rtg_depth, 1);
        assert!(plan.actions.contains(&AtpgAction::InitializeExternalDontCareSimulation));
        assert!(!plan.actions.contains(&AtpgAction::SequentialSetup));
    }

    #[test]
    fn dispatch_reports_missing_native_prerequisites()
    {
        let mut network = ();
        let error =
            execute_atpg_command(&mut network, AtpgCommandKind::Atpg, &AtpgOptions::default())
                .unwrap_err();

        assert_eq!(
            error,
            AtpgCommandError::Blocked {
                command: AtpgCommandKind::Atpg,
            }
        );
        assert!(error.to_string().contains("requires native Rust ports"));
    }
}
