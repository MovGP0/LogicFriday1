use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;

fn word_length() -> usize {
    usize::BITS as usize
}

fn all_zero() -> usize {
    0
}

fn all_one() -> usize {
    usize::MAX
}

fn max_timeout_seconds() -> i32 {
    3_600 * 24 * 365
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgOptions {
    pub quick_redund: bool,
    pub reverse_fault_sim: bool,
    pub pmt_only: bool,
    pub use_internal_states: bool,
    pub n_sim_sequences: usize,
    pub deterministic_prop: bool,
    pub random_prop: bool,
    pub rtg_depth: i32,
    pub fault_simulate: bool,
    pub fast_sat: bool,
    pub rtg: bool,
    pub build_product_machines: bool,
    pub tech_decomp: bool,
    pub timeout: i32,
    pub verbosity: i32,
    pub prop_rtg_depth: usize,
    pub n_random_prop_iter: usize,
    pub print_sequences: bool,
    pub force_comb: bool,
    pub sequence_output_path: Option<PathBuf>,
}

impl Default for AtpgOptions {
    fn default() -> Self {
        Self {
            quick_redund: false,
            reverse_fault_sim: true,
            pmt_only: false,
            use_internal_states: false,
            n_sim_sequences: word_length(),
            deterministic_prop: true,
            random_prop: true,
            rtg_depth: -1,
            fault_simulate: true,
            fast_sat: false,
            rtg: true,
            build_product_machines: true,
            tech_decomp: false,
            timeout: 0,
            verbosity: 0,
            prop_rtg_depth: 20,
            n_random_prop_iter: 1,
            print_sequences: false,
            force_comb: false,
            sequence_output_path: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtpgOptionError {
    MissingArgument(char),
    UnknownOption(char),
    InvalidInteger { option: char, value: String },
    TooManySimulationSequences { requested: usize, maximum: usize },
    TimeoutOutOfRange(i32),
    TooManyTrailingArguments(Vec<String>),
}

impl fmt::Display for AtpgOptionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArgument(option) => write!(formatter, "-{option} requires an argument"),
            Self::UnknownOption(option) => write!(formatter, "unknown ATPG option -{option}"),
            Self::InvalidInteger { option, value } => {
                write!(formatter, "-{option} expects an integer, got {value:?}")
            }
            Self::TooManySimulationSequences { requested, maximum } => write!(
                formatter,
                "-n requested {requested} simulation sequences, maximum is {maximum}"
            ),
            Self::TimeoutOutOfRange(timeout) => {
                write!(
                    formatter,
                    "-T timeout {timeout} is outside the accepted range"
                )
            }
            Self::TooManyTrailingArguments(arguments) => {
                write!(
                    formatter,
                    "unexpected trailing arguments: {}",
                    arguments.join(" ")
                )
            }
        }
    }
}

impl std::error::Error for AtpgOptionError {}

pub fn parse_atpg_options(arguments: &[impl AsRef<str>]) -> Result<AtpgOptions, AtpgOptionError> {
    let mut options = AtpgOptions::default();
    let mut trailing = Vec::new();
    let mut index = 0;

    while index < arguments.len() {
        let argument = arguments[index].as_ref();

        if argument == "--" {
            trailing.extend(
                arguments[index + 1..]
                    .iter()
                    .map(|item| item.as_ref().to_owned()),
            );
            break;
        }

        if !argument.starts_with('-') || argument == "-" {
            trailing.push(argument.to_owned());
            index += 1;
            continue;
        }

        let mut chars = argument[1..].chars().peekable();

        while let Some(option) = chars.next() {
            match option {
                'c' => options.force_comb = true,
                'D' => options.deterministic_prop = false,
                'f' => options.fault_simulate = false,
                'F' => options.reverse_fault_sim = false,
                'h' => options.fast_sat = true,
                'q' => {
                    options.quick_redund = true;
                    options.build_product_machines = false;
                }
                'r' => options.rtg = false,
                'R' => options.random_prop = false,
                'p' => options.build_product_machines = false,
                't' => options.tech_decomp = true,
                'd' | 'n' | 'T' | 'v' | 'y' | 'z' => {
                    let value = if chars.peek().is_some() {
                        chars.collect::<String>()
                    } else {
                        index += 1;
                        if index >= arguments.len() {
                            return Err(AtpgOptionError::MissingArgument(option));
                        }
                        arguments[index].as_ref().to_owned()
                    };

                    apply_numeric_option(&mut options, option, &value)?;
                    break;
                }
                _ => return Err(AtpgOptionError::UnknownOption(option)),
            }
        }

        index += 1;
    }

    match trailing.len() {
        0 => Ok(options),
        1 => {
            options.print_sequences = true;
            options.sequence_output_path = Some(PathBuf::from(&trailing[0]));
            Ok(options)
        }
        _ => Err(AtpgOptionError::TooManyTrailingArguments(trailing)),
    }
}

fn apply_numeric_option(
    options: &mut AtpgOptions,
    option: char,
    value: &str,
) -> Result<(), AtpgOptionError> {
    let parsed = value
        .parse::<i32>()
        .map_err(|_| AtpgOptionError::InvalidInteger {
            option,
            value: value.to_owned(),
        })?;

    match option {
        'd' => options.rtg_depth = parsed,
        'n' => {
            if parsed < 0 {
                return Err(AtpgOptionError::InvalidInteger {
                    option,
                    value: value.to_owned(),
                });
            }

            let requested = parsed as usize;
            if requested > word_length() {
                return Err(AtpgOptionError::TooManySimulationSequences {
                    requested,
                    maximum: word_length(),
                });
            }

            options.n_sim_sequences = requested;
        }
        'T' => {
            if !(0..=max_timeout_seconds()).contains(&parsed) {
                return Err(AtpgOptionError::TimeoutOutOfRange(parsed));
            }

            options.timeout = parsed;
        }
        'v' => options.verbosity = parsed,
        'y' => {
            if parsed < 0 {
                return Err(AtpgOptionError::InvalidInteger {
                    option,
                    value: value.to_owned(),
                });
            }

            options.prop_rtg_depth = parsed as usize;
        }
        'z' => {
            if parsed < 0 {
                return Err(AtpgOptionError::InvalidInteger {
                    option,
                    value: value.to_owned(),
                });
            }

            options.n_random_prop_iter = parsed as usize;
        }
        _ => unreachable!("numeric ATPG option dispatch should filter option letters"),
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TimeInfo {
    pub setup: u64,
    pub deterministic: u64,
    pub random: u64,
    pub fault_simulation: u64,
    pub redundancy: u64,
    pub product_machine: u64,
    pub sequential: u64,
    pub sat: u64,
    pub implication: u64,
    pub justification: u64,
    pub propagation: u64,
    pub total: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Statistics {
    pub n_detected_faults: usize,
    pub n_redundant_faults: usize,
    pub n_untested_faults: usize,
    pub n_aborted_faults: usize,
    pub n_faults: usize,
    pub n_patterns: usize,
    pub n_sequences: usize,
    pub n_vectors: usize,
    pub n_sat_calls: usize,
    pub n_sat_failures: usize,
    pub n_simulated_faults: usize,
    pub n_random_patterns: usize,
    pub n_deterministic_patterns: usize,
    pub n_product_machines: usize,
    pub n_state_justifications: usize,
    pub n_state_propagations: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgInfo {
    pub network_name: String,
    pub n_pi: usize,
    pub n_po: usize,
    pub n_latch: usize,
    pub n_real_pi: usize,
    pub n_real_po: usize,
    pub control_nodes: BTreeSet<String>,
    pub options: AtpgOptions,
    pub statistics: Statistics,
    pub time_info: TimeInfo,
    sequence_table: HashMap<String, Sequence>,
    redundant_faults: Vec<Fault>,
    untested_faults: Vec<Fault>,
    final_untested_faults: Vec<Fault>,
}

impl AtpgInfo {
    pub fn new(summary: NetworkSummary) -> Self {
        Self {
            network_name: summary.name,
            n_pi: summary.n_pi,
            n_po: summary.n_po,
            n_latch: summary.n_latch,
            n_real_pi: summary.n_real_pi,
            n_real_po: summary.n_real_po,
            control_nodes: summary.control_nodes,
            options: AtpgOptions::default(),
            statistics: Statistics::default(),
            time_info: TimeInfo::default(),
            sequence_table: HashMap::new(),
            redundant_faults: Vec::new(),
            untested_faults: Vec::new(),
            final_untested_faults: Vec::new(),
        }
    }

    pub fn has_control_fault_note(&self) -> bool {
        !self.control_nodes.is_empty()
    }

    pub fn add_sequence(&mut self, key: impl Into<String>, sequence: Sequence) {
        self.sequence_table.insert(key.into(), sequence);
    }

    pub fn sequence_count(&self) -> usize {
        self.sequence_table.len()
    }

    pub fn redundant_faults(&self) -> &[Fault] {
        &self.redundant_faults
    }

    pub fn untested_faults(&self) -> &[Fault] {
        &self.untested_faults
    }

    pub fn final_untested_faults(&self) -> &[Fault] {
        &self.final_untested_faults
    }

    pub fn take_sequence_printout(&mut self) -> Option<String> {
        let mut printout = if self.options.print_sequences {
            let mut text = format!("atpg test sequences for {}\n", self.network_name);

            if self.has_control_fault_note() {
                text.push_str(
                    "\nNOTE: Ignore setting of clock and other latch control inputs.\n\n",
                );
            }

            text.push_str("inputs:\n\n\n");
            Some(text)
        } else {
            None
        };

        let sequences = std::mem::take(&mut self.sequence_table);

        for sequence in sequences.values() {
            self.statistics.n_vectors += sequence.vectors().len();
            self.statistics.n_sequences += 1;

            if let Some(text) = printout.as_mut() {
                text.push_str(&sequence.format_vectors(self.n_real_pi));
            }
        }

        printout
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkSummary {
    pub name: String,
    pub n_pi: usize,
    pub n_po: usize,
    pub n_latch: usize,
    pub n_real_pi: usize,
    pub n_real_po: usize,
    pub control_nodes: BTreeSet<String>,
}

impl NetworkSummary {
    pub fn combinational(name: impl Into<String>, n_real_pi: usize, n_real_po: usize) -> Self {
        Self {
            name: name.into(),
            n_pi: n_real_pi,
            n_po: n_real_po,
            n_latch: 0,
            n_real_pi,
            n_real_po,
            control_nodes: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fault {
    pub node: String,
    pub stuck_at: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sequence {
    vectors: Vec<Vec<bool>>,
}

impl Sequence {
    pub fn new(vectors: Vec<Vec<bool>>) -> Self {
        Self { vectors }
    }

    pub fn vectors(&self) -> &[Vec<bool>] {
        &self.vectors
    }

    pub fn format_vectors(&self, n_real_pi: usize) -> String {
        let mut output = String::new();

        for vector in &self.vectors {
            let mut bits = vector
                .iter()
                .take(n_real_pi)
                .map(|bit| if *bit { '1' } else { '0' })
                .collect::<String>();

            if vector.len() < n_real_pi {
                bits.extend(std::iter::repeat_n('0', n_real_pi - vector.len()));
            }

            output.push_str(&bits);
            output.push('\n');
        }

        output
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimSatInfo {
    pub network_name: String,
    pub n_real_pi: usize,
    pub n_real_po: usize,
    pub used: Vec<i32>,
    pub word_vectors: Vec<Vec<usize>>,
    pub prop_word_vectors: Vec<Vec<usize>>,
    pub real_po_values: Vec<usize>,
    pub true_value: Vec<usize>,
    pub faults_ptr: Vec<Option<Fault>>,
}

impl SimSatInfo {
    pub fn new(summary: &NetworkSummary, options: &AtpgOptions) -> Self {
        Self {
            network_name: summary.name.clone(),
            n_real_pi: summary.n_real_pi,
            n_real_po: summary.n_real_po,
            used: vec![0; word_length()],
            word_vectors: Vec::new(),
            prop_word_vectors: vec![vec![all_zero(); summary.n_real_pi]; options.prop_rtg_depth],
            real_po_values: vec![all_zero(); summary.n_real_po],
            true_value: vec![all_zero(); summary.n_real_po],
            faults_ptr: vec![None; word_length()],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqInfo {
    pub build_product_machines: bool,
    pub start_states_known: bool,
    pub product_start_states_known: bool,
    pub reached_sets: Vec<String>,
    pub product_reached_sets: Option<Vec<String>>,
    pub input_trace: Vec<Vec<bool>>,
    pub product_machine_built: bool,
    pub just_sequence: Vec<Vec<usize>>,
    pub prop_sequence: Vec<Vec<usize>>,
}

impl SeqInfo {
    pub fn new(options: &AtpgOptions) -> Self {
        Self {
            build_product_machines: options.build_product_machines,
            start_states_known: false,
            product_start_states_known: false,
            reached_sets: Vec::new(),
            product_reached_sets: options.build_product_machines.then(Vec::new),
            input_trace: Vec::new(),
            product_machine_built: false,
            just_sequence: Vec::new(),
            prop_sequence: Vec::new(),
        }
    }

    pub fn setup_sequences(&mut self, depth: usize, n_real_pi: usize) {
        self.just_sequence = vec![vec![all_zero(); n_real_pi]; depth + 1];
        self.prop_sequence = vec![vec![all_zero(); n_real_pi]; depth + 1];
    }

    pub fn mark_product_machine_built(&mut self) {
        self.product_machine_built = true;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    Zero,
    One,
    PrimaryInput,
    PrimaryOutput,
    Buffer,
    Inverter,
    And,
    Or,
    Complex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    fn input_row(self) -> usize {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::DontCare => 2,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimNode {
    pub name: String,
    pub uid: usize,
    pub kind: NodeKind,
    pub fanins: Vec<usize>,
    pub fanout: Vec<usize>,
    pub visited: bool,
    pub value: usize,
    pub or_output_mask: usize,
    pub and_output_mask: usize,
    pub or_input_masks: Vec<usize>,
    pub and_input_masks: Vec<usize>,
    function: Vec<Vec<Literal>>,
}

impl SimNode {
    pub fn new(
        name: impl Into<String>,
        uid: usize,
        kind: NodeKind,
        fanins: Vec<usize>,
        mut fanout: Vec<usize>,
        function: Vec<Vec<Literal>>,
    ) -> Self {
        fanout.sort_unstable();

        let n_inputs = fanins.len();
        let value = match kind {
            NodeKind::Zero => all_zero(),
            NodeKind::One => all_one(),
            _ => all_zero(),
        };

        Self {
            name: name.into(),
            uid,
            kind,
            fanins,
            fanout,
            visited: false,
            value,
            or_output_mask: all_zero(),
            and_output_mask: all_one(),
            or_input_masks: vec![all_zero(); n_inputs],
            and_input_masks: vec![all_one(); n_inputs],
            function,
        }
    }

    pub fn function(&self) -> &[Vec<Literal>] {
        &self.function
    }
}

pub fn evaluate_sim_node(nodes: &mut [SimNode], uid: usize) {
    let value = match nodes[uid].kind {
        NodeKind::Zero | NodeKind::One | NodeKind::PrimaryInput => evaluate_pi(nodes, uid),
        NodeKind::PrimaryOutput => evaluate_po(nodes, uid),
        NodeKind::Buffer
        | NodeKind::Inverter
        | NodeKind::And
        | NodeKind::Or
        | NodeKind::Complex => evaluate_logic_node(nodes, uid),
    };

    nodes[uid].value = value;
}

pub fn evaluate_sim_nodes_in_uid_order(nodes: &mut [SimNode]) {
    for uid in 0..nodes.len() {
        evaluate_sim_node(nodes, uid);
    }
}

fn evaluate_pi(nodes: &[SimNode], uid: usize) -> usize {
    let node = &nodes[uid];
    (node.value & node.and_output_mask) | node.or_output_mask
}

fn evaluate_po(nodes: &[SimNode], uid: usize) -> usize {
    let node = &nodes[uid];
    let fanin_value = nodes[node.fanins[0]].value;
    let result = fanin_value & (node.and_input_masks[0] & node.and_output_mask);
    result | (node.or_input_masks[0] | node.or_output_mask)
}

fn evaluate_logic_node(nodes: &[SimNode], uid: usize) -> usize {
    let node = &nodes[uid];
    let mut complemented_inputs = Vec::with_capacity(node.fanins.len());
    let mut asserted_inputs = Vec::with_capacity(node.fanins.len());

    for (index, fanin_uid) in node.fanins.iter().copied().enumerate() {
        let result =
            (nodes[fanin_uid].value & node.and_input_masks[index]) | node.or_input_masks[index];
        asserted_inputs.push(result);
        complemented_inputs.push(!result);
    }

    let mut result = node.or_output_mask;

    for cube in &node.function {
        let mut and_result = node.and_output_mask;

        for (index, literal) in cube.iter().copied().enumerate() {
            and_result &= match literal.input_row() {
                0 => complemented_inputs[index],
                1 => asserted_inputs[index],
                2 => all_one(),
                _ => unreachable!("literal input row is limited to the extracted cube rows"),
            };
        }

        result |= and_result;
    }

    result
}

pub fn validate_sim_nodes(nodes: &[SimNode]) -> Result<(), SimNodeError> {
    let mut seen = HashSet::new();

    for (index, node) in nodes.iter().enumerate() {
        if node.uid != index {
            return Err(SimNodeError::UidMismatch {
                index,
                uid: node.uid,
            });
        }

        if !seen.insert(node.uid) {
            return Err(SimNodeError::DuplicateUid(node.uid));
        }

        for fanin in &node.fanins {
            if *fanin >= nodes.len() {
                return Err(SimNodeError::MissingFanin {
                    uid: node.uid,
                    fanin: *fanin,
                });
            }
        }

        if matches!(node.kind, NodeKind::PrimaryOutput) && node.fanins.len() != 1 {
            return Err(SimNodeError::PrimaryOutputFaninCount {
                uid: node.uid,
                count: node.fanins.len(),
            });
        }

        if node.or_input_masks.len() != node.fanins.len()
            || node.and_input_masks.len() != node.fanins.len()
        {
            return Err(SimNodeError::InputMaskCount {
                uid: node.uid,
                count: node.fanins.len(),
            });
        }

        for cube in &node.function {
            if cube.len() != node.fanins.len() {
                return Err(SimNodeError::CubeLiteralCount {
                    uid: node.uid,
                    count: cube.len(),
                    expected: node.fanins.len(),
                });
            }
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimNodeError {
    UidMismatch {
        index: usize,
        uid: usize,
    },
    DuplicateUid(usize),
    MissingFanin {
        uid: usize,
        fanin: usize,
    },
    PrimaryOutputFaninCount {
        uid: usize,
        count: usize,
    },
    InputMaskCount {
        uid: usize,
        count: usize,
    },
    CubeLiteralCount {
        uid: usize,
        count: usize,
        expected: usize,
    },
}

impl fmt::Display for SimNodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UidMismatch { index, uid } => {
                write!(formatter, "simulation node at index {index} has uid {uid}")
            }
            Self::DuplicateUid(uid) => write!(formatter, "duplicate simulation node uid {uid}"),
            Self::MissingFanin { uid, fanin } => write!(
                formatter,
                "simulation node {uid} references missing fanin {fanin}"
            ),
            Self::PrimaryOutputFaninCount { uid, count } => write!(
                formatter,
                "primary output simulation node {uid} has {count} fanins"
            ),
            Self::InputMaskCount { uid, count } => write!(
                formatter,
                "simulation node {uid} masks do not match {count} fanins"
            ),
            Self::CubeLiteralCount {
                uid,
                count,
                expected,
            } => write!(
                formatter,
                "simulation node {uid} cube has {count} literals, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for SimNodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_match_initialization_defaults() {
        let options = AtpgOptions::default();

        assert!(!options.quick_redund);
        assert!(options.reverse_fault_sim);
        assert!(!options.pmt_only);
        assert!(!options.use_internal_states);
        assert_eq!(options.n_sim_sequences, word_length());
        assert!(options.deterministic_prop);
        assert!(options.random_prop);
        assert_eq!(options.rtg_depth, -1);
        assert!(options.fault_simulate);
        assert!(!options.fast_sat);
        assert!(options.rtg);
        assert!(options.build_product_machines);
        assert!(!options.tech_decomp);
        assert_eq!(options.timeout, 0);
        assert_eq!(options.verbosity, 0);
        assert_eq!(options.prop_rtg_depth, 20);
        assert_eq!(options.n_random_prop_iter, 1);
        assert!(!options.print_sequences);
        assert!(!options.force_comb);
    }

    #[test]
    fn parses_flags_and_numeric_options() {
        let options = parse_atpg_options(&[
            "-cDfFhqrRpt",
            "-d",
            "4",
            "-n8",
            "-T",
            "120",
            "-v",
            "2",
            "-y",
            "3",
            "-z",
            "5",
            "seq.out",
        ])
        .unwrap();

        assert!(options.force_comb);
        assert!(!options.deterministic_prop);
        assert!(!options.fault_simulate);
        assert!(!options.reverse_fault_sim);
        assert!(options.fast_sat);
        assert!(options.quick_redund);
        assert!(!options.rtg);
        assert!(!options.random_prop);
        assert!(!options.build_product_machines);
        assert!(options.tech_decomp);
        assert_eq!(options.rtg_depth, 4);
        assert_eq!(options.n_sim_sequences, 8);
        assert_eq!(options.timeout, 120);
        assert_eq!(options.verbosity, 2);
        assert_eq!(options.prop_rtg_depth, 3);
        assert_eq!(options.n_random_prop_iter, 5);
        assert!(options.print_sequences);
        assert_eq!(options.sequence_output_path, Some(PathBuf::from("seq.out")));
    }

    #[test]
    fn rejects_too_many_simulation_sequences() {
        let arguments = vec!["-n".to_owned(), (word_length() + 1).to_string()];
        let error = parse_atpg_options(&arguments).unwrap_err();

        assert_eq!(
            error,
            AtpgOptionError::TooManySimulationSequences {
                requested: word_length() + 1,
                maximum: word_length(),
            }
        );
    }

    #[test]
    fn rejects_timeout_outside_original_range() {
        let error = parse_atpg_options(&["-T", "-1"]).unwrap_err();

        assert_eq!(error, AtpgOptionError::TimeoutOutOfRange(-1));
    }

    #[test]
    fn initializes_atpg_info_and_sequence_tables() {
        let mut control_nodes = BTreeSet::new();
        control_nodes.insert("clock".to_owned());
        let summary = NetworkSummary {
            name: "net".to_owned(),
            n_pi: 3,
            n_po: 2,
            n_latch: 1,
            n_real_pi: 2,
            n_real_po: 1,
            control_nodes,
        };
        let info = AtpgInfo::new(summary);

        assert_eq!(info.network_name, "net");
        assert_eq!(info.n_pi, 3);
        assert_eq!(info.n_po, 2);
        assert_eq!(info.n_latch, 1);
        assert_eq!(info.n_real_pi, 2);
        assert_eq!(info.n_real_po, 1);
        assert!(info.has_control_fault_note());
        assert_eq!(info.statistics, Statistics::default());
        assert_eq!(info.time_info, TimeInfo::default());
        assert_eq!(info.sequence_count(), 0);
        assert!(info.redundant_faults().is_empty());
        assert!(info.untested_faults().is_empty());
        assert!(info.final_untested_faults().is_empty());
    }

    #[test]
    fn sequence_printout_counts_and_clears_sequences() {
        let mut info = AtpgInfo::new(NetworkSummary::combinational("demo", 3, 1));
        info.options.print_sequences = true;
        info.add_sequence(
            "a",
            Sequence::new(vec![vec![true, false, true], vec![false, true]]),
        );

        let printout = info.take_sequence_printout().unwrap();

        assert!(printout.starts_with("atpg test sequences for demo\ninputs:\n\n\n"));
        assert!(printout.contains("101\n010\n"));
        assert_eq!(info.statistics.n_sequences, 1);
        assert_eq!(info.statistics.n_vectors, 2);
        assert_eq!(info.sequence_count(), 0);
    }

    #[test]
    fn sim_sat_info_allocates_initial_vectors() {
        let summary = NetworkSummary::combinational("demo", 2, 3);
        let mut options = AtpgOptions::default();
        options.prop_rtg_depth = 4;

        let info = SimSatInfo::new(&summary, &options);

        assert_eq!(info.used, vec![0; word_length()]);
        assert_eq!(info.prop_word_vectors, vec![vec![all_zero(); 2]; 4]);
        assert_eq!(info.real_po_values, vec![all_zero(); 3]);
        assert_eq!(info.true_value, vec![all_zero(); 3]);
        assert_eq!(info.faults_ptr, vec![None; word_length()]);
    }

    #[test]
    fn seq_info_allocates_depth_plus_one_sequences() {
        let options = AtpgOptions::default();
        let mut seq_info = SeqInfo::new(&options);

        seq_info.setup_sequences(2, 3);
        seq_info.mark_product_machine_built();

        assert_eq!(seq_info.just_sequence, vec![vec![all_zero(); 3]; 3]);
        assert_eq!(seq_info.prop_sequence, vec![vec![all_zero(); 3]; 3]);
        assert!(seq_info.product_reached_sets.is_some());
        assert!(seq_info.product_machine_built);
    }

    #[test]
    fn evaluates_primary_input_and_primary_output_masks() {
        let mut nodes = vec![
            SimNode::new("pi", 0, NodeKind::PrimaryInput, vec![], vec![1], vec![]),
            SimNode::new("po", 1, NodeKind::PrimaryOutput, vec![0], vec![], vec![]),
        ];
        nodes[0].value = 0b1010;
        nodes[0].and_output_mask = 0b1110;
        nodes[0].or_output_mask = 0b0001;
        nodes[1].and_input_masks[0] = 0b0111;
        nodes[1].or_output_mask = 0b1000;

        validate_sim_nodes(&nodes).unwrap();
        evaluate_sim_nodes_in_uid_order(&mut nodes);

        assert_eq!(nodes[0].value, 0b1011);
        assert_eq!(nodes[1].value, 0b1011);
    }

    #[test]
    fn evaluates_sum_of_product_cubes_like_original_loop() {
        let mut nodes = vec![
            SimNode::new("a", 0, NodeKind::PrimaryInput, vec![], vec![2], vec![]),
            SimNode::new("b", 1, NodeKind::PrimaryInput, vec![], vec![2], vec![]),
            SimNode::new(
                "logic",
                2,
                NodeKind::Complex,
                vec![0, 1],
                vec![],
                vec![
                    vec![Literal::One, Literal::One],
                    vec![Literal::Zero, Literal::DontCare],
                ],
            ),
        ];
        nodes[0].value = 0b1100;
        nodes[1].value = 0b1010;

        validate_sim_nodes(&nodes).unwrap();
        evaluate_sim_nodes_in_uid_order(&mut nodes);

        assert_eq!(nodes[2].value & 0b1111, 0b1011);
    }

    #[test]
    fn constructor_sorts_fanout_by_uid() {
        let node = SimNode::new(
            "node",
            0,
            NodeKind::PrimaryInput,
            vec![],
            vec![4, 1, 3],
            vec![],
        );

        assert_eq!(node.fanout, vec![1, 3, 4]);
    }

    #[test]
    fn validates_sim_node_shape() {
        let nodes = vec![SimNode::new(
            "bad",
            0,
            NodeKind::Complex,
            vec![2],
            vec![],
            vec![vec![Literal::One]],
        )];

        assert_eq!(
            validate_sim_nodes(&nodes).unwrap_err(),
            SimNodeError::MissingFanin { uid: 0, fanin: 2 }
        );
    }
}
