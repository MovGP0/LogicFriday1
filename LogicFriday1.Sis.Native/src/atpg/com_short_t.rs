//! Native Rust command model for SIS `short_tests`.
//!
//! The original command performs sequential ATPG through many SIS subsystems
//! that are ported independently. This module keeps the deterministic command
//! behavior in Rust: option parsing, early network checks, execution planning,
//! usage text, and the start-state bookkeeping used after generated tests.

use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

const WORD_LENGTH: usize = usize::BITS as usize;
const MAX_TIMEOUT_SECONDS: u32 = 3600 * 24 * 365;
const DEFAULT_PROP_RTG_DEPTH: i32 = 10;
const DEFAULT_RANDOM_PROP_ITERATIONS: usize = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortTestsOptions {
    pub quick_redundancy: bool,
    pub reverse_fault_simulation: bool,
    pub product_machine_traversal_only: bool,
    pub use_internal_states: bool,
    pub simulation_sequence_count: usize,
    pub deterministic_propagation: bool,
    pub random_propagation: bool,
    pub random_test_depth: i32,
    pub fault_simulation: bool,
    pub fast_sat: bool,
    pub random_test_generation: bool,
    pub build_product_machines: bool,
    pub technology_decomposition: bool,
    pub timeout_seconds: u32,
    pub verbosity: i32,
    pub propagation_random_test_depth: i32,
    pub random_propagation_iterations: usize,
    pub print_sequences: bool,
    pub output_file: Option<String>,
}

impl Default for ShortTestsOptions {
    fn default() -> Self {
        Self {
            quick_redundancy: false,
            reverse_fault_simulation: true,
            product_machine_traversal_only: false,
            use_internal_states: true,
            simulation_sequence_count: WORD_LENGTH,
            deterministic_propagation: true,
            random_propagation: false,
            random_test_depth: -1,
            fault_simulation: true,
            fast_sat: false,
            random_test_generation: false,
            build_product_machines: true,
            technology_decomposition: false,
            timeout_seconds: 0,
            verbosity: 0,
            propagation_random_test_depth: DEFAULT_PROP_RTG_DEPTH,
            random_propagation_iterations: DEFAULT_RANDOM_PROP_ITERATIONS,
            print_sequences: false,
            output_file: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortTestsParseResult {
    pub options: ShortTestsOptions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShortTestsParseError {
    MissingOptionValue(char),
    InvalidInteger { option: char, value: String },
    TimeoutOutOfRange(i64),
    UnknownOption(char),
    TooManyOperands(Vec<String>),
}

impl fmt::Display for ShortTestsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::InvalidInteger { option, value } => {
                write!(f, "invalid integer for -{option}: {value}")
            }
            Self::TimeoutOutOfRange(value) => {
                write!(
                    f,
                    "timeout must be in range 0..={MAX_TIMEOUT_SECONDS}, got {value}"
                )
            }
            Self::UnknownOption(option) => write!(f, "unknown option -{option}"),
            Self::TooManyOperands(operands) => write!(
                f,
                "short_tests accepts at most one output file, got {} operands",
                operands.len()
            ),
        }
    }
}

impl Error for ShortTestsParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkStats {
    pub is_present: bool,
    pub internal_count: usize,
    pub latch_count: usize,
}

impl NetworkStats {
    pub fn absent() -> Self {
        Self {
            is_present: false,
            internal_count: 0,
            latch_count: 0,
        }
    }

    pub fn present(internal_count: usize, latch_count: usize) -> Self {
        Self {
            is_present: true,
            internal_count,
            latch_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShortTestsPlan {
    NothingToDo,
    UseCombinationalAtpg,
    ExecuteSisBound { options: ShortTestsOptions },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShortTestsExecutionError {
    MissingNativeAtpgPorts,
}

impl fmt::Display for ShortTestsExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativeAtpgPorts => {
                write!(
                    f,
                    "short_tests requires native ATPG, SAT, BDD, sequence, and fault ports"
                )
            }
        }
    }
}

impl Error for ShortTestsExecutionError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartStateUpdate {
    pub key: u64,
    pub inserted_new_state: bool,
    pub appended_sequence: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartStateStore<Sequence> {
    reset_state_keys: BTreeSet<u64>,
    state_sequences: HashMap<u64, Vec<Sequence>>,
    start_state_keys: BTreeSet<u64>,
    product_start_state_keys: BTreeSet<u64>,
}

impl<Sequence> Default for StartStateStore<Sequence> {
    fn default() -> Self {
        Self {
            reset_state_keys: BTreeSet::new(),
            state_sequences: HashMap::new(),
            start_state_keys: BTreeSet::new(),
            product_start_state_keys: BTreeSet::new(),
        }
    }
}

impl<Sequence> StartStateStore<Sequence> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_reset_state(
        &mut self,
        good_state: &[bool],
        latch_to_pi_ordering: &[usize],
    ) -> Result<u64, StartStateError> {
        let key = state_key(good_state, latch_to_pi_ordering)?;
        self.reset_state_keys.insert(key);
        self.state_sequences.entry(key).or_default();
        Ok(key)
    }

    pub fn start_state_keys(&self) -> &BTreeSet<u64> {
        &self.start_state_keys
    }

    pub fn product_start_state_keys(&self) -> &BTreeSet<u64> {
        &self.product_start_state_keys
    }

    pub fn sequences_for_key(&self, key: u64) -> Option<&[Sequence]> {
        self.state_sequences.get(&key).map(Vec::as_slice)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StartStateError {
    MismatchedLatchVectors {
        state_len: usize,
        ordering_len: usize,
    },
    OrderingPositionOutOfRange {
        position: usize,
    },
}

impl fmt::Display for StartStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedLatchVectors {
                state_len,
                ordering_len,
            } => write!(
                f,
                "good_state length {state_len} does not match latch ordering length {ordering_len}"
            ),
            Self::OrderingPositionOutOfRange { position } => {
                write!(
                    f,
                    "latch ordering position {position} does not fit in a u64 key"
                )
            }
        }
    }
}

impl Error for StartStateError {}

pub fn short_tests_usage() -> &'static str {
    "usage: short_tests [-DfFhirtV] [-T seconds] [-v level] [file]\n\
     -D\tno deterministic propagation\n\
     -f\tno fault simulation\n\
     -F\tno reverse fault simulation\n\
     -h\tuse fast SAT; no non-local implications\n\
     -i\tdo not use internal states as start states for tests\n\
     -r\tuse random test generation and propagation\n\
     -t\tperform tech decomp of network\n\
     -T\ttimeout in seconds\n\
     -v\tverbosity\n\
     -V\tall tests generated by product machine traversal\n\
     file\toutput file for test patterns\n"
}

pub fn parse_short_tests_args<I, S>(args: I) -> Result<ShortTestsParseResult, ShortTestsParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = ShortTestsOptions::default();
    let operands = parse_options(args, "DfFhirtT:v:V", |option, value| {
        apply_short_tests_option(&mut options, option, value)
    })?;

    match operands.as_slice() {
        [] => {}
        [output_file] => {
            options.print_sequences = true;
            options.output_file = Some(output_file.clone());
        }
        _ => return Err(ShortTestsParseError::TooManyOperands(operands)),
    }

    Ok(ShortTestsParseResult { options })
}

pub fn plan_short_tests(
    args: &[&str],
    network: &NetworkStats,
) -> Result<ShortTestsPlan, ShortTestsParseError> {
    if !network.is_present || network.internal_count == 0 {
        return Ok(ShortTestsPlan::NothingToDo);
    }
    if network.latch_count == 0 {
        return Ok(ShortTestsPlan::UseCombinationalAtpg);
    }

    let parsed = parse_short_tests_args(args)?;
    Ok(ShortTestsPlan::ExecuteSisBound {
        options: parsed.options,
    })
}

pub fn execute_short_tests<Network>(
    _network: &mut Network,
    _options: &ShortTestsOptions,
) -> Result<(), ShortTestsExecutionError> {
    Err(ShortTestsExecutionError::MissingNativeAtpgPorts)
}

pub fn update_start_states<Sequence>(
    store: &mut StartStateStore<Sequence>,
    good_state: &[bool],
    latch_to_pi_ordering: &[usize],
    sequence: Sequence,
) -> Result<StartStateUpdate, StartStateError> {
    update_start_state_set(
        store,
        good_state,
        latch_to_pi_ordering,
        sequence,
        StartStateKind::Regular,
    )
}

pub fn update_product_start_states<Sequence>(
    store: &mut StartStateStore<Sequence>,
    good_state: &[bool],
    latch_to_pi_ordering: &[usize],
    sequence: Sequence,
) -> Result<StartStateUpdate, StartStateError> {
    update_start_state_set(
        store,
        good_state,
        latch_to_pi_ordering,
        sequence,
        StartStateKind::Product,
    )
}

fn apply_short_tests_option(
    options: &mut ShortTestsOptions,
    option: char,
    value: String,
) -> Result<(), ShortTestsParseError> {
    match option {
        'D' => {
            options.deterministic_propagation = false;
            Ok(())
        }
        'f' => {
            options.fault_simulation = false;
            Ok(())
        }
        'F' => {
            options.reverse_fault_simulation = false;
            Ok(())
        }
        'h' => {
            options.fast_sat = true;
            Ok(())
        }
        'i' => {
            options.use_internal_states = false;
            Ok(())
        }
        'r' => {
            options.random_test_generation = true;
            options.random_propagation = true;
            Ok(())
        }
        't' => {
            options.technology_decomposition = true;
            Ok(())
        }
        'T' => {
            let timeout = parse_i64(option, &value)?;
            if timeout < 0 || timeout > i64::from(MAX_TIMEOUT_SECONDS) {
                return Err(ShortTestsParseError::TimeoutOutOfRange(timeout));
            }
            options.timeout_seconds = timeout as u32;
            Ok(())
        }
        'v' => {
            options.verbosity = parse_i32(option, &value)?;
            Ok(())
        }
        'V' => {
            options.product_machine_traversal_only = true;
            Ok(())
        }
        _ => Err(ShortTestsParseError::UnknownOption(option)),
    }
}

fn parse_options<F>(
    args: impl IntoIterator<Item = impl AsRef<str>>,
    spec: &str,
    mut apply: F,
) -> Result<Vec<String>, ShortTestsParseError>
where
    F: FnMut(char, String) -> Result<(), ShortTestsParseError>,
{
    let mut iter = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .peekable();
    let mut operands = Vec::new();
    let mut scanning_options = true;

    while let Some(arg) = iter.next() {
        if !scanning_options || !arg.starts_with('-') || arg == "-" {
            operands.push(arg);
            operands.extend(iter);
            break;
        }
        if arg == "--" {
            scanning_options = false;
            continue;
        }

        let mut chars = arg[1..].char_indices().peekable();
        while let Some((offset, option)) = chars.next() {
            let needs_value = option_needs_value(spec, option)
                .ok_or(ShortTestsParseError::UnknownOption(option))?;
            if needs_value {
                let value_start = offset + option.len_utf8();
                let value = if value_start < arg[1..].len() {
                    arg[1 + value_start..].to_owned()
                } else {
                    iter.next()
                        .ok_or(ShortTestsParseError::MissingOptionValue(option))?
                };
                apply(option, value)?;
                break;
            } else {
                apply(option, String::new())?;
            }
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
        if candidate == option {
            return Some(chars.peek() == Some(&':'));
        }
    }

    None
}

fn parse_i32(option: char, value: &str) -> Result<i32, ShortTestsParseError> {
    value
        .parse::<i32>()
        .map_err(|_| ShortTestsParseError::InvalidInteger {
            option,
            value: value.to_owned(),
        })
}

fn parse_i64(option: char, value: &str) -> Result<i64, ShortTestsParseError> {
    value
        .parse::<i64>()
        .map_err(|_| ShortTestsParseError::InvalidInteger {
            option,
            value: value.to_owned(),
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartStateKind {
    Regular,
    Product,
}

fn update_start_state_set<Sequence>(
    store: &mut StartStateStore<Sequence>,
    good_state: &[bool],
    latch_to_pi_ordering: &[usize],
    sequence: Sequence,
    kind: StartStateKind,
) -> Result<StartStateUpdate, StartStateError> {
    let key = state_key(good_state, latch_to_pi_ordering)?;
    if store.reset_state_keys.contains(&key) {
        return Ok(StartStateUpdate {
            key,
            inserted_new_state: false,
            appended_sequence: false,
        });
    }

    let inserted_new_state = !store.state_sequences.contains_key(&key);
    store
        .state_sequences
        .entry(key)
        .or_default()
        .insert(0, sequence);

    if inserted_new_state {
        match kind {
            StartStateKind::Regular => {
                store.start_state_keys.insert(key);
            }
            StartStateKind::Product => {
                store.product_start_state_keys.insert(key);
            }
        }
    }

    Ok(StartStateUpdate {
        key,
        inserted_new_state,
        appended_sequence: true,
    })
}

pub fn state_key(
    good_state: &[bool],
    latch_to_pi_ordering: &[usize],
) -> Result<u64, StartStateError> {
    if good_state.len() != latch_to_pi_ordering.len() {
        return Err(StartStateError::MismatchedLatchVectors {
            state_len: good_state.len(),
            ordering_len: latch_to_pi_ordering.len(),
        });
    }

    let mut key = 0_u64;
    for (value, position) in good_state.iter().zip(latch_to_pi_ordering.iter()).rev() {
        if *position >= u64::BITS as usize {
            return Err(StartStateError::OrderingPositionOutOfRange {
                position: *position,
            });
        }
        if *value {
            key |= 1_u64 << position;
        }
    }

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_match_c_initialization() {
        let parsed = parse_short_tests_args::<_, &str>([]).unwrap();

        assert_eq!(parsed.options, ShortTestsOptions::default());
        assert_eq!(parsed.options.simulation_sequence_count, WORD_LENGTH);
        assert!(parsed.options.reverse_fault_simulation);
        assert!(parsed.options.use_internal_states);
        assert!(parsed.options.deterministic_propagation);
        assert!(parsed.options.fault_simulation);
        assert!(parsed.options.build_product_machines);
    }

    #[test]
    fn parse_combined_flags_and_operands() {
        let parsed =
            parse_short_tests_args(["-DfFhir", "-t", "-T", "25", "-v3", "-V", "patterns.out"])
                .unwrap();

        assert!(!parsed.options.deterministic_propagation);
        assert!(!parsed.options.fault_simulation);
        assert!(!parsed.options.reverse_fault_simulation);
        assert!(parsed.options.fast_sat);
        assert!(!parsed.options.use_internal_states);
        assert!(parsed.options.random_test_generation);
        assert!(parsed.options.random_propagation);
        assert!(parsed.options.technology_decomposition);
        assert_eq!(parsed.options.timeout_seconds, 25);
        assert_eq!(parsed.options.verbosity, 3);
        assert!(parsed.options.product_machine_traversal_only);
        assert!(parsed.options.print_sequences);
        assert_eq!(parsed.options.output_file.as_deref(), Some("patterns.out"));
    }

    #[test]
    fn rejects_invalid_timeout_and_extra_operands() {
        assert_eq!(
            parse_short_tests_args(["-T", "-1"]).unwrap_err(),
            ShortTestsParseError::TimeoutOutOfRange(-1)
        );
        assert_eq!(
            parse_short_tests_args(["-T", "31536001"]).unwrap_err(),
            ShortTestsParseError::TimeoutOutOfRange(31_536_001)
        );
        assert_eq!(
            parse_short_tests_args(["one", "two"]).unwrap_err(),
            ShortTestsParseError::TooManyOperands(vec!["one".to_owned(), "two".to_owned()])
        );
    }

    #[test]
    fn plans_early_network_exits_before_parsing() {
        assert_eq!(
            plan_short_tests(&["-Z"], &NetworkStats::absent()).unwrap(),
            ShortTestsPlan::NothingToDo
        );
        assert_eq!(
            plan_short_tests(&["-Z"], &NetworkStats::present(0, 1)).unwrap(),
            ShortTestsPlan::NothingToDo
        );
        assert_eq!(
            plan_short_tests(&["-Z"], &NetworkStats::present(1, 0)).unwrap(),
            ShortTestsPlan::UseCombinationalAtpg
        );
        assert!(matches!(
            plan_short_tests(&["-Z"], &NetworkStats::present(1, 1)).unwrap_err(),
            ShortTestsParseError::UnknownOption('Z')
        ));
    }

    #[test]
    fn computes_state_key_from_latch_ordering() {
        let key = state_key(&[true, false, true, true], &[3, 1, 0, 2]).unwrap();

        assert_eq!(key, 0b1101);
    }

    #[test]
    fn updates_regular_start_states_like_c_state_table() {
        let mut store = StartStateStore::new();
        let first = update_start_states(&mut store, &[true, false], &[0, 1], "first").unwrap();
        let second = update_start_states(&mut store, &[true, false], &[0, 1], "second").unwrap();

        assert_eq!(
            first,
            StartStateUpdate {
                key: 1,
                inserted_new_state: true,
                appended_sequence: true,
            }
        );
        assert_eq!(
            second,
            StartStateUpdate {
                key: 1,
                inserted_new_state: false,
                appended_sequence: true,
            }
        );
        assert_eq!(
            store.start_state_keys().iter().copied().collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(store.sequences_for_key(1).unwrap(), &["second", "first"]);
    }

    #[test]
    fn reset_states_are_not_added_as_start_states() {
        let mut store = StartStateStore::new();
        store.mark_reset_state(&[false, true], &[0, 1]).unwrap();

        let update = update_start_states(&mut store, &[false, true], &[0, 1], "ignored").unwrap();

        assert_eq!(
            update,
            StartStateUpdate {
                key: 2,
                inserted_new_state: false,
                appended_sequence: false,
            }
        );
        assert!(store.start_state_keys().is_empty());
        assert_eq!(store.sequences_for_key(2).unwrap(), &[] as &[&str]);
    }

    #[test]
    fn product_updates_have_separate_start_state_set() {
        let mut store = StartStateStore::new();
        update_product_start_states(&mut store, &[true, true], &[0, 1], "seq").unwrap();

        assert!(store.start_state_keys().is_empty());
        assert_eq!(
            store
                .product_start_state_keys()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![3]
        );
    }
}
