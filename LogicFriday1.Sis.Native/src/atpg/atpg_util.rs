//! Native Rust utilities corresponding to the SIS ATPG utility routines.
//!
//! The original C file mixed pointer sorting, list splicing, vector sequence
//! construction, bit-packed word-vector maintenance, and diagnostic formatting.
//! This module keeps those operations as ordinary Rust data transformations.

use std::collections::HashMap;
use std::fmt::Write;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtpgValue
{
    Zero,
    One,
}

impl AtpgValue
{
    pub fn as_char(self) -> char
    {
        match self
        {
            Self::Zero => '0',
            Self::One => '1',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SatValue
{
    False,
    True,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StuckAt
{
    Zero,
    One,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderedNode<N>
{
    pub node: N,
    pub order: i32,
}

impl<N> OrderedNode<N>
{
    pub fn new(node: N, order: i32) -> Self
    {
        Self {
            node,
            order,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SatInput<K>
{
    pub sat_id: usize,
    pub info: K,
}

impl<K> SatInput<K>
{
    pub fn new(sat_id: usize, info: K) -> Self
    {
        Self {
            sat_id,
            info,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExcitationContext<K>
where
    K: Eq + Hash,
{
    pub n_real_pi: usize,
    pub n_latch: usize,
    pub sat_input_vars: Vec<SatInput<K>>,
    pub pi_po_table: HashMap<K, usize>,
}

impl<K> ExcitationContext<K>
where
    K: Eq + Hash,
{
    pub fn new(
        n_real_pi: usize,
        n_latch: usize,
        sat_input_vars: Vec<SatInput<K>>,
        pi_po_table: HashMap<K, usize>,
    ) -> Self
    {
        Self {
            n_real_pi,
            n_latch,
            sat_input_vars,
            pi_po_table,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sequence
{
    pub vectors: Vec<Vec<AtpgValue>>,
}

impl Sequence
{
    pub fn new(vectors: Vec<Vec<AtpgValue>>) -> Self
    {
        Self {
            vectors,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceInfo
{
    pub just_sequence: Vec<Vec<AtpgValue>>,
    pub prop_sequence: Vec<Vec<AtpgValue>>,
}

impl SequenceInfo
{
    pub fn new(just_sequence: Vec<Vec<AtpgValue>>, prop_sequence: Vec<Vec<AtpgValue>>) -> Self
    {
        Self {
            just_sequence,
            prop_sequence,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SteadyStateInfo
{
    pub n_real_pi: usize,
    pub word_vectors: Vec<Vec<u32>>,
}

impl SteadyStateInfo
{
    pub fn new(n_real_pi: usize) -> Self
    {
        Self {
            n_real_pi,
            word_vectors: Vec::new(),
        }
    }

    pub fn with_word_vectors(n_real_pi: usize, word_vectors: Vec<Vec<u32>>) -> Self
    {
        Self {
            n_real_pi,
            word_vectors,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fault<N>
{
    pub value: StuckAt,
    pub node: N,
    pub fanin: Option<N>,
}

impl<N> Fault<N>
{
    pub fn new(value: StuckAt, node: N, fanin: Option<N>) -> Self
    {
        Self {
            value,
            node,
            fanin,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaultSequence
{
    pub sequence: Sequence,
}

impl FaultSequence
{
    pub fn new(sequence: Sequence) -> Self
    {
        Self {
            sequence,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Statistics
{
    pub initial_faults: usize,
    pub tested_faults: usize,
    pub redundant_faults: usize,
    pub untested_faults: usize,
    pub final_untested_faults: usize,
    pub n_rtg_tested: usize,
    pub sat_red: usize,
    pub verified_red: usize,
    pub n_just_reused: usize,
    pub n_random_propagations: usize,
    pub n_random_propagated: usize,
    pub n_det_propagations: usize,
    pub n_prop_reused: usize,
    pub n_ff_propagated: usize,
    pub n_not_ff_propagated: usize,
    pub n_untested_by_main_loop: usize,
    pub n_verifications: usize,
    pub n_sequences: usize,
    pub n_vectors: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TimeInfo
{
    pub setup: u64,
    pub traverse_stg: u64,
    pub rtg: u64,
    pub sat_clauses: u64,
    pub sat_solve: u64,
    pub justify: u64,
    pub random_propagate: u64,
    pub ff_propagate: u64,
    pub fault_simulate: u64,
    pub product_machine_verify: u64,
    pub reverse_fault_sim: u64,
    pub total_time: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SequentialReachabilityCounts
{
    pub dfs_depth: usize,
    pub states_reached_in_dfs: f64,
    pub justified_states: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResultSummary
{
    pub statistics: Statistics,
    pub time_info: TimeInfo,
    pub verbosity: usize,
    pub sequential_counts: Option<SequentialReachabilityCounts>,
}

pub fn fanin_dfs_sort<N>(fanins: &[OrderedNode<N>]) -> Vec<usize>
{
    let mut fanin_ptr: Vec<_> = fanins
        .iter()
        .enumerate()
        .map(|(index, fanin)| (index, fanin.order))
        .collect();

    fanin_ptr.sort_by(|left, right| left.1.cmp(&right.1));
    fanin_ptr.into_iter().map(|(index, _)| index).collect()
}

pub fn sat_fanout_dfs_sort<N>(fanouts: &[OrderedNode<N>]) -> Vec<N>
where
    N: Clone,
{
    let mut fanout_ptr = fanouts.to_vec();
    fanout_ptr.sort_by(|left, right| left.order.cmp(&right.order));
    fanout_ptr
        .into_iter()
        .map(|fanout| fanout.node)
        .collect()
}

pub fn concat_lists<T>(l1: &mut Vec<T>, l2: &mut Vec<T>)
{
    l1.append(l2);
}

pub fn create_just_sequence(
    seq_info: &mut SequenceInfo,
    n_new_vectors: usize,
    old_vectors: &[Vec<AtpgValue>],
    npi: usize,
)
{
    assert!(seq_info.prop_sequence.len() >= n_new_vectors);
    assert_vectors_have_width(&seq_info.just_sequence, npi);
    assert_vectors_have_width(&seq_info.prop_sequence[..n_new_vectors], npi);
    assert_vectors_have_width(old_vectors, npi);

    let n_old_vectors = old_vectors.len();
    let required_len = n_old_vectors + n_new_vectors;

    while seq_info.just_sequence.len() < required_len
    {
        seq_info.just_sequence.push(vec![AtpgValue::One; npi]);
    }

    for index in (0..n_new_vectors).rev()
    {
        seq_info.prop_sequence[index][..npi]
            .copy_from_slice(&seq_info.just_sequence[index][..npi]);
    }

    for index in (0..n_old_vectors).rev()
    {
        seq_info.just_sequence[index][..npi].copy_from_slice(&old_vectors[index][..npi]);
    }

    for index in (0..n_new_vectors).rev()
    {
        seq_info.just_sequence[index + n_old_vectors][..npi]
            .copy_from_slice(&seq_info.prop_sequence[index][..npi]);
    }
}

pub fn derive_excitation_vector<K>(
    info: &ExcitationContext<K>,
    n_pi_vars: usize,
    sat_value: impl Fn(usize) -> SatValue,
) -> Vec<AtpgValue>
where
    K: Eq + Hash,
{
    assert!(n_pi_vars <= info.sat_input_vars.len());

    let mut vector = vec![AtpgValue::One; info.n_real_pi];

    for sat_input in info.sat_input_vars.iter().take(n_pi_vars)
    {
        let value = sat_value(sat_input.sat_id);
        let index = info
            .pi_po_table
            .get(&sat_input.info)
            .copied()
            .expect("SAT input info must be present in the PI/PO table");

        if value == SatValue::False && index >= info.n_latch
        {
            let real_pi_index = index - info.n_latch;
            assert!(real_pi_index < info.n_real_pi);
            vector[real_pi_index] = AtpgValue::Zero;
        }
    }

    vector
}

pub fn format_fault<N>(fault: &Fault<N>, node_name: impl Fn(&N) -> String) -> String
{
    let mut output = String::new();

    match fault.value
    {
        StuckAt::Zero => write!(output, "S_A_0: NODE: {} ", node_name(&fault.node)).unwrap(),
        StuckAt::One => write!(output, "S_A_1: NODE: {} ", node_name(&fault.node)).unwrap(),
    }

    match &fault.fanin
    {
        Some(fanin) => writeln!(output, "\tINPUT: {}", node_name(fanin)).unwrap(),
        None => writeln!(output, "\tOUTPUT").unwrap(),
    }

    output
}

pub fn format_vectors(vectors: &[Vec<AtpgValue>], n_inputs: usize) -> String
{
    format_some_vectors(vectors, n_inputs, vectors.len())
}

pub fn format_some_vectors(
    vectors: &[Vec<AtpgValue>],
    n_inputs: usize,
    seq_length: usize,
) -> String
{
    assert!(seq_length <= vectors.len());
    assert_vectors_have_width(&vectors[..seq_length], n_inputs);

    let mut output = String::new();

    for vector in vectors.iter().take(seq_length)
    {
        for value in vector.iter().take(n_inputs)
        {
            write!(output, "{} ", value.as_char()).unwrap();
        }

        output.push('\n');
    }

    output.push('\n');
    output
}

pub fn format_results(summary: &ResultSummary) -> String
{
    let stats = &summary.statistics;
    let mut output = String::new();

    writeln!(
        output,
        "faults: {}\ttested: {}\tredundant: {}\tuntested: {}",
        stats.initial_faults,
        stats.tested_faults,
        stats.redundant_faults,
        stats.untested_faults + stats.final_untested_faults,
    )
    .unwrap();

    if summary.verbosity > 1
    {
        writeln!(output, "tested by RTG: {}", stats.n_rtg_tested).unwrap();
        writeln!(
            output,
            "comb. red. or all excite states unreachable: {}\tverified red.: {}",
            stats.sat_red,
            stats.verified_red,
        )
        .unwrap();
        writeln!(
            output,
            "reused justification sequences: {}",
            stats.n_just_reused,
        )
        .unwrap();
        writeln!(
            output,
            "random propagations: {}\tsuccessful: {}",
            stats.n_random_propagations,
            stats.n_random_propagated,
        )
        .unwrap();
        writeln!(
            output,
            "deterministic propagations: {}",
            stats.n_det_propagations,
        )
        .unwrap();
        writeln!(
            output,
            "reused propagation sequences: {}",
            stats.n_prop_reused,
        )
        .unwrap();
        writeln!(
            output,
            "ff prop. sequence was test in faulty machine: {}\twasn't test: {}",
            stats.n_ff_propagated,
            stats.n_not_ff_propagated,
        )
        .unwrap();
        writeln!(
            output,
            "untested by main loop: {}\tverifications performed: {}",
            stats.n_untested_by_main_loop,
            stats.n_verifications,
        )
        .unwrap();

        if let Some(counts) = summary.sequential_counts
        {
            writeln!(
                output,
                "DFS depth: {}\tstates reached in DFS: {:.0}",
                counts.dfs_depth,
                counts.states_reached_in_dfs,
            )
            .unwrap();
            writeln!(output, "justified states: {:.0}", counts.justified_states).unwrap();
        }

        output.push_str(&format_time_info(&summary.time_info));
        writeln!(output, "number of tests in test set: {}", stats.n_sequences).unwrap();

        if summary.sequential_counts.is_some()
        {
            writeln!(output, "number of vectors in test set: {}", stats.n_vectors).unwrap();
        }
    }

    output
}

pub fn format_time_info(time_info: &TimeInfo) -> String
{
    let total_time = time_info.total_time;
    let mut output = String::new();

    write_time_line(&mut output, "setup time:\t\t\t", time_info.setup, total_time);
    write_time_line(
        &mut output,
        "STG traversal time:\t\t",
        time_info.traverse_stg,
        total_time,
    );
    write_time_line(&mut output, "RTG time:\t\t\t", time_info.rtg, total_time);
    write_time_line(
        &mut output,
        "SAT clause setup time:\t\t",
        time_info.sat_clauses,
        total_time,
    );
    write_time_line(
        &mut output,
        "SAT solve time:\t\t\t",
        time_info.sat_solve,
        total_time,
    );
    write_time_line(
        &mut output,
        "justification time:\t\t",
        time_info.justify,
        total_time,
    );
    write_time_line(
        &mut output,
        "random propagation time:\t",
        time_info.random_propagate,
        total_time,
    );
    write_time_line(
        &mut output,
        "fault-free propagation time:\t",
        time_info.ff_propagate,
        total_time,
    );
    write_time_line(
        &mut output,
        "fault simulation time:\t\t",
        time_info.fault_simulate,
        total_time,
    );
    write_time_line(
        &mut output,
        "product machine verification:\t",
        time_info.product_machine_verify,
        total_time,
    );
    write_time_line(
        &mut output,
        "reverse fault simulation:\t",
        time_info.reverse_fault_sim,
        total_time,
    );
    writeln!(output, "total time:\t\t\t{:.2}", total_time as f64 / 1000.0).unwrap();

    output
}

pub fn derive_test_sequence(
    ss_info: &mut SteadyStateInfo,
    seq_info: &SequenceInfo,
    n_just_vectors: usize,
    n_prop_vectors: usize,
    npi: usize,
    bit_index: usize,
) -> Sequence
{
    assert!(bit_index < u32::BITS as usize);
    assert!(npi <= ss_info.n_real_pi);
    assert!(seq_info.just_sequence.len() >= n_just_vectors);
    assert!(seq_info.prop_sequence.len() >= n_prop_vectors);
    assert_vectors_have_width(&seq_info.just_sequence[..n_just_vectors], npi);
    assert_vectors_have_width(&seq_info.prop_sequence[..n_prop_vectors], npi);

    let total_vectors = n_just_vectors + n_prop_vectors;

    if ss_info.word_vectors.len() < total_vectors
    {
        lengthen_word_vectors(
            ss_info,
            total_vectors - ss_info.word_vectors.len(),
            ss_info.n_real_pi,
        );
    }

    let mut vectors = vec![vec![AtpgValue::One; npi]; total_vectors];

    for index in (0..n_just_vectors).rev()
    {
        vectors[index][..npi].copy_from_slice(&seq_info.just_sequence[index][..npi]);
        apply_vector_to_word_vector(
            &vectors[index],
            &mut ss_info.word_vectors[index],
            npi,
            bit_index,
        );
    }

    for index in (0..n_prop_vectors).rev()
    {
        let sequence_index = index + n_just_vectors;
        vectors[sequence_index][..npi].copy_from_slice(&seq_info.prop_sequence[index][..npi]);
        apply_vector_to_word_vector(
            &vectors[sequence_index],
            &mut ss_info.word_vectors[sequence_index],
            npi,
            bit_index,
        );
    }

    Sequence::new(vectors)
}

pub fn reset_word_vectors(info: &mut SteadyStateInfo)
{
    for vector in &mut info.word_vectors
    {
        assert!(vector.len() >= info.n_real_pi);

        for value in vector.iter_mut().take(info.n_real_pi)
        {
            *value = u32::MAX;
        }
    }
}

pub fn lengthen_word_vectors(info: &mut SteadyStateInfo, n_vectors: usize, npi: usize)
{
    assert!(npi <= info.n_real_pi);

    for _ in 0..n_vectors
    {
        info.word_vectors.push(vec![u32::MAX; npi]);
    }
}

pub fn fillin_word_vectors(
    info: &mut SteadyStateInfo,
    fault_sequences: &[Option<FaultSequence>],
    seq_length: usize,
    npi: usize,
)
{
    assert!(npi <= info.n_real_pi);

    if info.word_vectors.len() < seq_length
    {
        lengthen_word_vectors(info, seq_length - info.word_vectors.len(), npi);
    }

    for (bit_index, fault_sequence) in fault_sequences.iter().enumerate()
    {
        assert!(bit_index < u32::BITS as usize);

        let Some(fault_sequence) = fault_sequence else
        {
            continue;
        };

        for (vector_index, vector) in fault_sequence.sequence.vectors.iter().enumerate()
        {
            assert!(vector_index < info.word_vectors.len());
            apply_vector_to_word_vector(
                vector,
                &mut info.word_vectors[vector_index],
                npi,
                bit_index,
            );
        }
    }
}

fn apply_vector_to_word_vector(
    vector: &[AtpgValue],
    word_vector: &mut [u32],
    npi: usize,
    bit_index: usize,
)
{
    assert!(vector.len() >= npi);
    assert!(word_vector.len() >= npi);

    let mask = 1_u32 << bit_index;

    for index in (0..npi).rev()
    {
        match vector[index]
        {
            AtpgValue::Zero => word_vector[index] &= !mask,
            AtpgValue::One => word_vector[index] |= mask,
        }
    }
}

fn assert_vectors_have_width(vectors: &[Vec<AtpgValue>], width: usize)
{
    assert!(vectors.iter().all(|vector| vector.len() >= width));
}

fn write_time_line(output: &mut String, label: &str, value: u64, total_time: u64)
{
    let seconds = value as f64 / 1000.0;
    let percent = if total_time == 0
    {
        0.0
    }
    else
    {
        100.0 * value as f64 / total_time as f64
    };

    writeln!(output, "{}{:.2}\t{:.2}%", label, seconds, percent).unwrap();
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn fanin_dfs_sort_returns_original_indices_in_clause_order()
    {
        let fanins = vec![
            OrderedNode::new("a", 30),
            OrderedNode::new("b", 10),
            OrderedNode::new("c", 20),
        ];

        assert_eq!(fanin_dfs_sort(&fanins), vec![1, 2, 0]);
    }

    #[test]
    fn sat_fanout_dfs_sort_returns_nodes_in_clause_order()
    {
        let fanouts = vec![
            OrderedNode::new("late", 30),
            OrderedNode::new("early", 10),
            OrderedNode::new("middle", 20),
        ];

        assert_eq!(sat_fanout_dfs_sort(&fanouts), vec!["early", "middle", "late"]);
    }

    #[test]
    fn concat_lists_moves_second_list_to_first()
    {
        let mut first = vec![1, 2];
        let mut second = vec![3, 4];

        concat_lists(&mut first, &mut second);

        assert_eq!(first, vec![1, 2, 3, 4]);
        assert!(second.is_empty());
    }

    #[test]
    fn create_just_sequence_preserves_new_vectors_while_prefixing_old_vectors()
    {
        let mut seq_info = SequenceInfo::new(
            vec![
                vec![AtpgValue::Zero, AtpgValue::One],
                vec![AtpgValue::One, AtpgValue::Zero],
            ],
            vec![
                vec![AtpgValue::One, AtpgValue::One],
                vec![AtpgValue::One, AtpgValue::One],
            ],
        );
        let old_vectors = vec![vec![AtpgValue::Zero, AtpgValue::Zero]];

        create_just_sequence(&mut seq_info, 2, &old_vectors, 2);

        assert_eq!(
            seq_info.just_sequence,
            vec![
                vec![AtpgValue::Zero, AtpgValue::Zero],
                vec![AtpgValue::Zero, AtpgValue::One],
                vec![AtpgValue::One, AtpgValue::Zero],
            ],
        );
    }

    #[test]
    fn derive_excitation_vector_defaults_to_one_and_clears_false_real_pis()
    {
        let mut pi_po_table = HashMap::new();
        pi_po_table.insert("latch", 0);
        pi_po_table.insert("pi0", 1);
        pi_po_table.insert("pi1", 2);
        let info = ExcitationContext::new(
            2,
            1,
            vec![
                SatInput::new(10, "latch"),
                SatInput::new(11, "pi0"),
                SatInput::new(12, "pi1"),
            ],
            pi_po_table,
        );

        let vector = derive_excitation_vector(&info, 3, |sat_id| {
            if sat_id == 10 || sat_id == 11
            {
                SatValue::False
            }
            else
            {
                SatValue::Unknown
            }
        });

        assert_eq!(vector, vec![AtpgValue::Zero, AtpgValue::One]);
    }

    #[test]
    fn format_fault_matches_legacy_text_shape()
    {
        let fault = Fault::new(StuckAt::One, 10, Some(20));

        let text = format_fault(&fault, |node| format!("n{}", node));

        assert_eq!(text, "S_A_1: NODE: n10 \tINPUT: n20\n");
    }

    #[test]
    fn format_vectors_prints_values_with_trailing_blank_line()
    {
        let vectors = vec![
            vec![AtpgValue::Zero, AtpgValue::One],
            vec![AtpgValue::One, AtpgValue::Zero],
        ];

        assert_eq!(format_vectors(&vectors, 2), "0 1 \n1 0 \n\n");
    }

    #[test]
    fn derive_test_sequence_copies_vectors_and_updates_packed_words()
    {
        let seq_info = SequenceInfo::new(
            vec![vec![AtpgValue::Zero, AtpgValue::One]],
            vec![vec![AtpgValue::One, AtpgValue::Zero]],
        );
        let mut ss_info = SteadyStateInfo::new(2);

        let sequence = derive_test_sequence(&mut ss_info, &seq_info, 1, 1, 2, 3);

        assert_eq!(
            sequence.vectors,
            vec![
                vec![AtpgValue::Zero, AtpgValue::One],
                vec![AtpgValue::One, AtpgValue::Zero],
            ],
        );
        assert_eq!(ss_info.word_vectors[0], vec![u32::MAX & !(1 << 3), u32::MAX]);
        assert_eq!(ss_info.word_vectors[1], vec![u32::MAX, u32::MAX & !(1 << 3)]);
    }

    #[test]
    fn reset_word_vectors_restores_all_ones()
    {
        let mut ss_info = SteadyStateInfo::with_word_vectors(2, vec![vec![0, 1], vec![2, 3]]);

        reset_word_vectors(&mut ss_info);

        assert_eq!(ss_info.word_vectors, vec![vec![u32::MAX; 2], vec![u32::MAX; 2]]);
    }

    #[test]
    fn fillin_word_vectors_replays_fault_sequences_into_bit_slots()
    {
        let mut ss_info = SteadyStateInfo::new(2);
        let fault_sequences = vec![
            Some(FaultSequence::new(Sequence::new(vec![
                vec![AtpgValue::Zero, AtpgValue::One],
                vec![AtpgValue::One, AtpgValue::Zero],
            ]))),
            None,
            Some(FaultSequence::new(Sequence::new(vec![
                vec![AtpgValue::One, AtpgValue::Zero],
            ]))),
        ];

        fillin_word_vectors(&mut ss_info, &fault_sequences, 2, 2);

        assert_eq!(ss_info.word_vectors[0][0] & 0b101, 0b100);
        assert_eq!(ss_info.word_vectors[0][1] & 0b101, 0b001);
        assert_eq!(ss_info.word_vectors[1][0] & 0b101, 0b101);
        assert_eq!(ss_info.word_vectors[1][1] & 0b101, 0b100);
    }

    #[test]
    fn format_time_info_handles_zero_total_time()
    {
        let text = format_time_info(&TimeInfo {
            setup: 25,
            total_time: 0,
            ..TimeInfo::default()
        });

        assert!(text.contains("setup time:\t\t\t0.03\t0.00%"));
        assert!(text.contains("total time:\t\t\t0.00"));
    }
}
