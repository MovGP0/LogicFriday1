use std::collections::VecDeque;

pub type SimWord = u32;

const WORD_LENGTH: usize = SimWord::BITS as usize;
const ALL_ZERO: SimWord = 0;
const ALL_ONE: SimWord = !0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultValue
{
    StuckAtZero,
    StuckAtOne,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultSite
{
    Output,
    Input(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fault
{
    pub node_id: usize,
    pub site: FaultSite,
    pub value: FaultValue,
    pub current_state: Vec<SimWord>,
    pub sequence_index: usize,
    pub sequence: Option<TestSequence>,
    pub is_covered: bool,
    pub status: FaultStatus,
}

impl Fault
{
    pub fn output(node_id: usize, value: FaultValue, latch_count: usize) -> Self
    {
        Self {
            node_id,
            site: FaultSite::Output,
            value,
            current_state: vec![ALL_ZERO; latch_count],
            sequence_index: 0,
            sequence: None,
            is_covered: false,
            status: FaultStatus::Untested,
        }
    }

    pub fn input(node_id: usize, input_index: usize, value: FaultValue, latch_count: usize)
        -> Self
    {
        Self {
            node_id,
            site: FaultSite::Input(input_index),
            value,
            current_state: vec![ALL_ZERO; latch_count],
            sequence_index: 0,
            sequence: None,
            is_covered: false,
            status: FaultStatus::Untested,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultStatus
{
    Untested,
    Tested,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestSequence
{
    pub vectors: Vec<Vec<SimWord>>,
    pub index: usize,
    pub cover_count: usize,
}

impl TestSequence
{
    pub fn new(vectors: Vec<Vec<SimWord>>) -> Self
    {
        Self {
            vectors,
            index: 0,
            cover_count: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SequenceInfo
{
    pub good_state: Vec<i32>,
    pub faulty_state: Vec<i32>,
    pub just_sequence: Vec<Vec<SimWord>>,
    pub prop_sequence: Vec<Vec<SimWord>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimOp
{
    PrimaryInput,
    ConstZero,
    ConstOne,
    Buffer,
    Not,
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Xnor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimNode
{
    pub fanins: Vec<usize>,
    pub fanouts: Vec<usize>,
    pub op: SimOp,
    pub value: SimWord,
    and_input_masks: Vec<SimWord>,
    or_input_masks: Vec<SimWord>,
    and_output_mask: SimWord,
    or_output_mask: SimWord,
    visited: bool,
}

impl SimNode
{
    pub fn new(op: SimOp, fanins: Vec<usize>) -> Self
    {
        let mask_count = fanins.len();
        Self {
            fanins,
            fanouts: Vec::new(),
            op,
            value: ALL_ZERO,
            and_input_masks: vec![ALL_ONE; mask_count],
            or_input_masks: vec![ALL_ZERO; mask_count],
            and_output_mask: ALL_ONE,
            or_output_mask: ALL_ZERO,
            visited: false,
        }
    }

    pub fn input() -> Self
    {
        Self::new(SimOp::PrimaryInput, Vec::new())
    }

    fn evaluate_from_snapshot(&self, snapshot: &[SimWord]) -> SimWord
    {
        let input = |index: usize| -> SimWord {
            let fanin = self.fanins[index];
            (snapshot[fanin] & self.and_input_masks[index]) | self.or_input_masks[index]
        };

        let value = match self.op {
            SimOp::PrimaryInput => self.value,
            SimOp::ConstZero => ALL_ZERO,
            SimOp::ConstOne => ALL_ONE,
            SimOp::Buffer => input(0),
            SimOp::Not => !input(0),
            SimOp::And => (0..self.fanins.len()).fold(ALL_ONE, |acc, index| acc & input(index)),
            SimOp::Nand => !(0..self.fanins.len()).fold(ALL_ONE, |acc, index| acc & input(index)),
            SimOp::Or => (0..self.fanins.len()).fold(ALL_ZERO, |acc, index| acc | input(index)),
            SimOp::Nor => !(0..self.fanins.len()).fold(ALL_ZERO, |acc, index| acc | input(index)),
            SimOp::Xor => (0..self.fanins.len()).fold(ALL_ZERO, |acc, index| acc ^ input(index)),
            SimOp::Xnor => !(0..self.fanins.len()).fold(ALL_ZERO, |acc, index| acc ^ input(index)),
        };

        (value & self.and_output_mask) | self.or_output_mask
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgSimNetwork
{
    pub nodes: Vec<SimNode>,
    pub pi_uid: Vec<usize>,
    pub po_uid: Vec<usize>,
    pub n_latch: usize,
    pub n_real_pi: usize,
    pub n_real_po: usize,
    pub reset_state: Vec<SimWord>,
    pub true_value: Vec<SimWord>,
    pub true_state: Vec<SimWord>,
    pub real_po_values: Vec<SimWord>,
    pub faulty_state: Vec<SimWord>,
    pub faults_ptr: Vec<Option<usize>>,
    changed_node_indices: Vec<usize>,
    tfo: Vec<usize>,
}

impl AtpgSimNetwork
{
    pub fn new(
        mut nodes: Vec<SimNode>,
        pi_uid: Vec<usize>,
        po_uid: Vec<usize>,
        n_latch: usize,
        n_real_pi: usize,
        n_real_po: usize,
    ) -> Self
    {
        for node in &mut nodes {
            node.fanouts.clear();
        }

        for id in 0..nodes.len() {
            let fanins = nodes[id].fanins.clone();
            for fanin in fanins {
                nodes[fanin].fanouts.push(id);
            }
        }

        Self {
            nodes,
            pi_uid,
            po_uid,
            n_latch,
            n_real_pi,
            n_real_po,
            reset_state: vec![ALL_ZERO; n_latch],
            true_value: vec![ALL_ZERO; n_real_po],
            true_state: vec![ALL_ZERO; n_latch],
            real_po_values: vec![ALL_ZERO; n_real_po],
            faulty_state: vec![ALL_ZERO; n_latch],
            faults_ptr: vec![None; WORD_LENGTH],
            changed_node_indices: Vec::new(),
            tfo: Vec::new(),
        }
    }

    pub fn network_simulate(
        &mut self,
        real_pi_values: &[SimWord],
        state: &mut [SimWord],
        real_po_values: &mut [SimWord],
    )
    {
        assert_eq!(real_pi_values.len(), self.n_real_pi);
        assert_eq!(state.len(), self.n_latch);
        assert_eq!(real_po_values.len(), self.n_real_po);

        for i in 0..self.n_latch {
            self.nodes[self.pi_uid[i]].value = state[i];
        }

        for i in 0..self.n_real_pi {
            self.nodes[self.pi_uid[i + self.n_latch]].value = real_pi_values[i];
        }

        self.evaluate_all();
        self.copy_outputs(real_po_values, state);
    }

    pub fn set_single_fault_masks(&mut self, fault: &Fault)
    {
        self.set_fault_mask(fault, ALL_ONE);
    }

    pub fn reset_single_fault_masks(&mut self, fault: &Fault)
    {
        let node = &mut self.nodes[fault.node_id];
        match fault.site {
            FaultSite::Output => {
                node.or_output_mask = ALL_ZERO;
                node.and_output_mask = ALL_ONE;
            }
            FaultSite::Input(index) => {
                node.or_input_masks[index] = ALL_ZERO;
                node.and_input_masks[index] = ALL_ONE;
            }
        }
    }

    pub fn set_parallel_fault_masks(&mut self, faults: &[Fault], active_faults: &[Option<usize>])
    {
        for (bit, fault_index) in active_faults.iter().enumerate() {
            if let Some(fault_index) = *fault_index {
                self.set_fault_mask_bit(&faults[fault_index], bit);
            }
        }
    }

    pub fn reset_parallel_fault_masks(&mut self, faults: &[Fault], active_faults: &[Option<usize>])
    {
        for fault_index in active_faults.iter().flatten() {
            self.reset_single_fault_masks(&faults[*fault_index]);
        }
    }

    pub fn seq_single_fault_simulate(
        &mut self,
        sequences: &[Vec<SimWord>],
        faults: &mut Vec<Fault>,
        exdc: Option<&mut AtpgSimNetwork>,
        original_sequences: Option<&[TestSequence]>,
        cnt: usize,
    ) -> Vec<Fault>
    {
        let mut true_state = self.reset_state.clone();
        let mut covered = Vec::new();
        let mut remaining = Vec::with_capacity(faults.len());
        let mut exdc = exdc;

        for (vector_number, vector) in sequences.iter().enumerate() {
            let mut true_value = vec![ALL_ZERO; self.n_real_po];
            self.network_simulate(vector, &mut true_state, &mut true_value);

            let exdc_value = if let Some(exdc_info) = exdc.as_deref_mut() {
                let mut exdc_state = exdc_info.reset_state.clone();
                let mut value = vec![ALL_ZERO; exdc_info.n_real_po];
                exdc_info.network_simulate(vector, &mut exdc_state, &mut value);
                Some(value)
            } else {
                None
            };

            let current_faults = std::mem::take(faults);
            let mut prev_fault: Option<Fault> = None;

            for mut fault in current_faults {
                let mut changed = Vec::new();
                self.set_single_fault_masks(&fault);

                if let Some(previous) = prev_fault.as_ref() {
                    changed.push(previous.node_id);
                }

                if vector_number > 0 {
                    for latch in 0..self.n_latch {
                        let pi = self.pi_uid[latch];
                        if self.nodes[pi].value != fault.current_state[latch] {
                            self.nodes[pi].value = fault.current_state[latch];
                            changed.push(pi);
                        }
                    }
                }

                changed.push(fault.node_id);
                self.simulate_tfo_array(&changed);
                let mut po_values = vec![ALL_ZERO; self.n_real_po];
                let mut next_state = vec![ALL_ZERO; self.n_latch];
                self.copy_outputs(&mut po_values, &mut next_state);
                fault.current_state = next_state;
                self.reset_single_fault_masks(&fault);
                prev_fault = Some(fault.clone());

                if record_single_fault_coverage(
                    &mut fault,
                    &true_value,
                    &po_values,
                    exdc_value.as_deref(),
                    self.n_real_po,
                    vector_number,
                    original_sequences,
                    cnt,
                ) {
                    covered.push(fault);
                } else {
                    remaining.push(fault);
                }
            }

            *faults = remaining;
            remaining = Vec::with_capacity(faults.len());
        }

        covered
    }

    pub fn fault_simulate(
        &mut self,
        sequences: &[Vec<SimWord>],
        faults: &mut Vec<Fault>,
        untested_faults: &mut Vec<Fault>,
        alloc_sequences: &[TestSequence],
        exdc: Option<&mut AtpgSimNetwork>,
    ) -> Vec<Fault>
    {
        let mut exdc = exdc;
        let mut covered = self.seq_single_fault_simulate(
            sequences,
            faults,
            exdc.as_deref_mut(),
            Some(alloc_sequences),
            alloc_sequences.len(),
        );
        covered.extend(self.seq_single_fault_simulate(
            sequences,
            untested_faults,
            exdc.as_deref_mut(),
            Some(alloc_sequences),
            alloc_sequences.len(),
        ));

        for fault in &mut covered {
            fault.sequence = alloc_sequences.get(fault.sequence_index).cloned();
            fault.is_covered = true;
            fault.status = FaultStatus::Tested;
        }

        covered
    }

    pub fn extract_test_sequences(
        &self,
        faults: &mut [Fault],
        sequences: &[Vec<SimWord>],
        next_test_index: &mut usize,
        npi: usize,
    ) -> Vec<Option<TestSequence>>
    {
        let mut used = [false; WORD_LENGTH];
        for fault in faults.iter() {
            used[fault.sequence_index] = true;
        }

        let mut alloc_sequences = vec![None; WORD_LENGTH];
        for bit in 0..WORD_LENGTH {
            if used[bit] {
                *next_test_index += 1;
                let vectors = sequences
                    .iter()
                    .map(|word_vector| {
                        (0..npi)
                            .map(|input| {
                                if extract_bit(word_vector[input], bit) {
                                    ALL_ONE
                                } else {
                                    ALL_ZERO
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

                alloc_sequences[bit] = Some(TestSequence {
                    vectors,
                    index: *next_test_index,
                    cover_count: 0,
                });
            }
        }

        for fault in faults {
            if let Some(sequence) = alloc_sequences[fault.sequence_index].as_mut() {
                sequence.cover_count += 1;
                fault.sequence = Some(sequence.clone());
                fault.is_covered = true;
                fault.status = FaultStatus::Tested;
            }
        }

        alloc_sequences
    }

    pub fn verify_test(&mut self, fault: &Fault, test_sequence: &TestSequence) -> bool
    {
        let mut state = self.reset_state.clone();
        let mut po_values = vec![ALL_ZERO; self.n_real_po];
        let active_faults = vec![Some(0)];
        let faults = vec![fault.clone()];

        self.set_parallel_fault_masks(&faults, &active_faults);
        for vector in &test_sequence.vectors {
            self.network_simulate(vector, &mut state, &mut po_values);
            if po_values
                .iter()
                .any(|po| extract_bit(*po, 1) != extract_bit(*po, 0))
            {
                self.reset_parallel_fault_masks(&faults, &active_faults);
                return true;
            }
        }

        self.reset_parallel_fault_masks(&faults, &active_faults);
        false
    }

    pub fn sequence_simulate(
        &mut self,
        test_sequence: &mut TestSequence,
        faults: &mut Vec<Fault>,
    ) -> Vec<Fault>
    {
        let mut covered_indices = Vec::new();
        let mut chunk_start = 0;

        while chunk_start < faults.len() {
            let chunk_end = usize::min(chunk_start + WORD_LENGTH - 1, faults.len());
            let active_faults = (chunk_start..chunk_end)
                .map(Some)
                .chain(std::iter::once(None))
                .collect::<Vec<_>>();
            let mut state = self.reset_state.clone();
            let mut po_values = vec![ALL_ZERO; self.n_real_po];

            self.set_parallel_fault_masks(faults, &active_faults);
            for vector in &test_sequence.vectors {
                self.network_simulate(vector, &mut state, &mut po_values);
                record_parallel_fault_coverage(&po_values, faults, &active_faults, self.n_real_po);
            }
            self.reset_parallel_fault_masks(faults, &active_faults);

            for index in chunk_start..chunk_end {
                if faults[index].is_covered {
                    covered_indices.push(index);
                }
            }

            chunk_start = chunk_end;
        }

        let mut covered = Vec::new();
        let mut remaining = Vec::with_capacity(faults.len().saturating_sub(covered_indices.len()));
        for (index, mut fault) in std::mem::take(faults).into_iter().enumerate() {
            if covered_indices.contains(&index) {
                fault.status = FaultStatus::Tested;
                fault.sequence = Some(test_sequence.clone());
                test_sequence.cover_count += 1;
                covered.push(fault);
            } else {
                remaining.push(fault);
            }
        }

        *faults = remaining;
        covered
    }

    pub fn final_state(&mut self, test_sequence: &TestSequence, seq_info: &mut SequenceInfo)
    {
        let mut state = self.reset_state.clone();
        let mut po_values = vec![ALL_ZERO; self.n_real_po];

        for vector in &test_sequence.vectors {
            self.network_simulate(vector, &mut state, &mut po_values);
        }

        seq_info.good_state = state
            .iter()
            .map(|latch| {
                if *latch == ALL_ZERO {
                    0
                } else {
                    assert_eq!(*latch, ALL_ONE);
                    1
                }
            })
            .collect();
    }

    pub fn simulate_entire_sequence(
        &mut self,
        fault: &Fault,
        seq_info: &mut SequenceInfo,
        vectors: &[Vec<SimWord>],
    )
    {
        let mut state = self.reset_state.clone();
        let mut po_values = vec![ALL_ZERO; self.n_real_po];
        let active_faults = vec![Some(0)];
        let faults = vec![fault.clone()];

        self.set_parallel_fault_masks(&faults, &active_faults);
        for vector in vectors {
            self.network_simulate(vector, &mut state, &mut po_values);
        }
        self.reset_parallel_fault_masks(&faults, &active_faults);
        record_different_states(seq_info, &state, self.n_latch);
    }

    pub fn min_just_sequence(&mut self, fault: &mut Fault, seq_info: &mut SequenceInfo) -> isize
    {
        let mut state = self.reset_state.clone();
        let mut po_values = vec![ALL_ZERO; self.n_real_po];
        let active_faults = vec![Some(0)];
        let faults = vec![fault.clone()];

        self.set_parallel_fault_masks(&faults, &active_faults);
        for index in 0..seq_info.just_sequence.len() {
            let vector = seq_info.just_sequence[index].clone();
            self.network_simulate(&vector, &mut state, &mut po_values);

            if po_values
                .iter()
                .any(|po| extract_bit(*po, 1) != extract_bit(*po, 0))
            {
                self.reset_parallel_fault_masks(&faults, &active_faults);
                fault.is_covered = true;
                return (index + 1) as isize;
            }

            if state
                .iter()
                .any(|latch| extract_bit(*latch, 1) != extract_bit(*latch, 0))
            {
                self.reset_parallel_fault_masks(&faults, &active_faults);
                record_different_states(seq_info, &state, self.n_latch);
                return (index + 1) as isize;
            }
        }

        self.reset_parallel_fault_masks(&faults, &active_faults);
        -1
    }

    pub fn extract_sequences(
        &self,
        sequences: &[TestSequence],
        from: usize,
        to: usize,
        seq_length: usize,
        npi: usize,
    ) -> Vec<Vec<SimWord>>
    {
        let mut word_vectors = vec![vec![ALL_ZERO; npi]; seq_length];
        for h in from..to {
            let bit = h - from;
            for (i, vector) in sequences[h].vectors.iter().enumerate() {
                for j in 0..npi {
                    if vector[j] == ALL_ZERO {
                        word_vectors[i][j] &= !(1 << bit);
                    } else {
                        assert_eq!(vector[j], ALL_ONE);
                        word_vectors[i][j] |= 1 << bit;
                    }
                }
            }
        }

        word_vectors
    }

    fn evaluate_all(&mut self)
    {
        for id in 0..self.nodes.len() {
            self.evaluate_node(id);
        }
    }

    fn evaluate_node(&mut self, id: usize)
    {
        let snapshot = self.nodes.iter().map(|node| node.value).collect::<Vec<_>>();
        let value = self.nodes[id].evaluate_from_snapshot(&snapshot);
        self.nodes[id].value = value;
    }

    fn copy_outputs(&self, real_po_values: &mut [SimWord], next_state: &mut [SimWord])
    {
        for i in 0..self.n_latch {
            next_state[i] = self.nodes[self.po_uid[i]].value;
        }

        for i in 0..self.n_real_po {
            real_po_values[i] = self.nodes[self.po_uid[i + self.n_latch]].value;
        }
    }

    fn set_fault_mask(&mut self, fault: &Fault, mask: SimWord)
    {
        let node = &mut self.nodes[fault.node_id];
        match (fault.site, fault.value) {
            (FaultSite::Output, FaultValue::StuckAtOne) => node.or_output_mask = mask,
            (FaultSite::Output, FaultValue::StuckAtZero) => node.and_output_mask = !mask,
            (FaultSite::Input(index), FaultValue::StuckAtOne) => node.or_input_masks[index] = mask,
            (FaultSite::Input(index), FaultValue::StuckAtZero) => {
                node.and_input_masks[index] = !mask
            }
        }
    }

    fn set_fault_mask_bit(&mut self, fault: &Fault, bit: usize)
    {
        let node = &mut self.nodes[fault.node_id];
        let mask = 1 << bit;
        match (fault.site, fault.value) {
            (FaultSite::Output, FaultValue::StuckAtOne) => node.or_output_mask |= mask,
            (FaultSite::Output, FaultValue::StuckAtZero) => node.and_output_mask &= !mask,
            (FaultSite::Input(index), FaultValue::StuckAtOne) => node.or_input_masks[index] |= mask,
            (FaultSite::Input(index), FaultValue::StuckAtZero) => {
                node.and_input_masks[index] &= !mask
            }
        }
    }

    fn simulate_tfo_array(&mut self, changed_nodes: &[usize])
    {
        self.changed_node_indices.clear();
        self.changed_node_indices.extend_from_slice(changed_nodes);
        self.tfo.clear();

        let mut stack = VecDeque::new();
        for id in changed_nodes.iter().rev().copied() {
            stack.push_back((id, false));
        }

        while let Some((id, expanded)) = stack.pop_back() {
            if expanded {
                self.tfo.push(id);
                continue;
            }

            if self.nodes[id].visited {
                continue;
            }

            self.nodes[id].visited = true;
            stack.push_back((id, true));
            for fanout in self.nodes[id].fanouts.iter().rev().copied() {
                stack.push_back((fanout, false));
            }
        }

        while let Some(id) = self.tfo.pop() {
            self.evaluate_node(id);
            self.nodes[id].visited = false;
        }
    }
}

pub fn record_single_fault_coverage(
    fault: &mut Fault,
    true_value: &[SimWord],
    po_values: &[SimWord],
    exdc_value: Option<&[SimWord]>,
    npo: usize,
    vector_number: usize,
    original_sequences: Option<&[TestSequence]>,
    cnt: usize,
) -> bool
{
    for i in (0..npo).rev() {
        let tval = true_value[i];
        let fval = po_values[i];
        let dcval = exdc_value.map_or(ALL_ZERO, |values| values[i]);

        if ((tval ^ fval) & !dcval) != 0 {
            match original_sequences {
                None => {
                    for bit in 0..WORD_LENGTH {
                        if !extract_bit(dcval, bit)
                            && extract_bit(tval, bit) != extract_bit(fval, bit)
                        {
                            fault.sequence_index = bit;
                            return true;
                        }
                    }
                }
                Some(sequences) => {
                    for bit in 0..cnt {
                        if !extract_bit(dcval, bit)
                            && extract_bit(tval, bit) != extract_bit(fval, bit)
                            && vector_number < sequences[bit].vectors.len()
                        {
                            fault.sequence_index = bit;
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

pub fn record_parallel_fault_coverage(
    po_values: &[SimWord],
    faults: &mut [Fault],
    active_faults: &[Option<usize>],
    npo: usize,
)
{
    let true_value = po_values
        .iter()
        .take(npo)
        .map(|value| extract_bit(*value, WORD_LENGTH - 1))
        .collect::<Vec<_>>();

    for bit in 0..WORD_LENGTH - 1 {
        if let Some(fault_index) = active_faults.get(bit).and_then(|index| *index) {
            for j in 0..npo {
                if extract_bit(po_values[j], bit) != true_value[j] {
                    faults[fault_index].is_covered = true;
                    break;
                }
            }
        }
    }
}

pub fn record_different_states(seq_info: &mut SequenceInfo, state: &[SimWord], n_latch: usize)
{
    seq_info.good_state.resize(n_latch, 0);
    seq_info.faulty_state.resize(n_latch, 0);

    for i in (0..n_latch).rev() {
        seq_info.good_state[i] = if extract_bit(state[i], 1) { 1 } else { 0 };
        seq_info.faulty_state[i] = if extract_bit(state[i], 0) { 1 } else { 0 };
    }
}

pub fn sequence_length_desc(left: &TestSequence, right: &TestSequence) -> std::cmp::Ordering
{
    right.vectors.len().cmp(&left.vectors.len())
}

pub fn sequence_index_desc(left: &TestSequence, right: &TestSequence) -> std::cmp::Ordering
{
    right.index.cmp(&left.index)
}

fn extract_bit(value: SimWord, bit: usize) -> bool
{
    ((value >> bit) & 1) != 0
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn simple_and_network() -> AtpgSimNetwork
    {
        let nodes = vec![
            SimNode::input(),
            SimNode::input(),
            SimNode::new(SimOp::And, vec![0, 1]),
        ];
        AtpgSimNetwork::new(nodes, vec![0, 1], vec![2], 0, 2, 1)
    }

    #[test]
    fn network_simulate_sets_inputs_and_reads_primary_outputs()
    {
        let mut network = simple_and_network();
        let mut state = Vec::new();
        let mut outputs = vec![0];

        network.network_simulate(&[ALL_ONE, 0b1010], &mut state, &mut outputs);

        assert_eq!(outputs, vec![0b1010]);
    }

    #[test]
    fn single_fault_output_mask_forces_stuck_value_and_resets()
    {
        let mut network = simple_and_network();
        let fault = Fault::output(2, FaultValue::StuckAtZero, 0);
        let mut state = Vec::new();
        let mut outputs = vec![0];

        network.set_single_fault_masks(&fault);
        network.network_simulate(&[ALL_ONE, ALL_ONE], &mut state, &mut outputs);

        assert_eq!(outputs, vec![ALL_ZERO]);

        network.reset_single_fault_masks(&fault);
        network.network_simulate(&[ALL_ONE, ALL_ONE], &mut state, &mut outputs);

        assert_eq!(outputs, vec![ALL_ONE]);
    }

    #[test]
    fn record_single_fault_coverage_ignores_exdc_bits()
    {
        let mut fault = Fault::output(0, FaultValue::StuckAtOne, 0);

        let covered = record_single_fault_coverage(
            &mut fault,
            &[0b0010],
            &[0b1011],
            Some(&[0b0001]),
            1,
            0,
            None,
            0,
        );

        assert!(covered);
        assert_eq!(fault.sequence_index, 3);
    }

    #[test]
    fn seq_single_fault_simulate_removes_covered_faults()
    {
        let mut network = simple_and_network();
        let mut faults = vec![Fault::output(2, FaultValue::StuckAtZero, 0)];
        let sequences = vec![vec![ALL_ONE, ALL_ONE]];

        let covered = network.seq_single_fault_simulate(&sequences, &mut faults, None, None, 0);

        assert_eq!(covered.len(), 1);
        assert!(faults.is_empty());
        assert_eq!(covered[0].sequence_index, 0);
    }

    #[test]
    fn extract_test_sequences_expands_word_bits_into_binary_vectors()
    {
        let network = simple_and_network();
        let mut faults = vec![Fault::output(2, FaultValue::StuckAtZero, 0)];
        faults[0].sequence_index = 1;
        let sequences = vec![vec![0b10, 0b01]];
        let mut next_test_index = 7;

        let alloc =
            network.extract_test_sequences(&mut faults, &sequences, &mut next_test_index, 2);
        let sequence = alloc[1].as_ref().unwrap();

        assert_eq!(next_test_index, 8);
        assert_eq!(sequence.index, 8);
        assert_eq!(sequence.vectors, vec![vec![ALL_ONE, ALL_ZERO]]);
        assert!(faults[0].is_covered);
    }

    #[test]
    fn parallel_sequence_simulation_marks_faults_against_good_machine_bit()
    {
        let mut network = simple_and_network();
        let mut sequence = TestSequence::new(vec![vec![ALL_ONE, ALL_ONE]]);
        let mut faults = vec![Fault::output(2, FaultValue::StuckAtZero, 0)];

        let covered = network.sequence_simulate(&mut sequence, &mut faults);

        assert_eq!(covered.len(), 1);
        assert_eq!(sequence.cover_count, 1);
        assert!(faults.is_empty());
    }

    #[test]
    fn final_state_records_latch_values_after_sequence()
    {
        let nodes = vec![
            SimNode::input(),
            SimNode::input(),
            SimNode::new(SimOp::Buffer, vec![1]),
        ];
        let mut network = AtpgSimNetwork::new(nodes, vec![0, 1], vec![2], 1, 1, 0);
        let sequence = TestSequence::new(vec![vec![ALL_ONE]]);
        let mut seq_info = SequenceInfo::default();

        network.final_state(&sequence, &mut seq_info);

        assert_eq!(seq_info.good_state, vec![1]);
    }

    #[test]
    fn tfo_simulation_recomputes_only_reachable_fanout()
    {
        let mut network = simple_and_network();
        network.nodes[0].value = ALL_ONE;
        network.nodes[1].value = ALL_ONE;
        network.evaluate_all();
        network.nodes[1].value = ALL_ZERO;

        network.simulate_tfo_array(&[1]);

        assert_eq!(network.nodes[2].value, ALL_ZERO);
        assert!(network.nodes.iter().all(|node| !node.visited));
    }
}
