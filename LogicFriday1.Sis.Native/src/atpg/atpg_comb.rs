use std::collections::HashSet;
use std::fmt;

pub type PackedValue = u32;

const PACKED_BITS: usize = PackedValue::BITS as usize;
const ALL_ZERO: PackedValue = 0;
const ALL_ONE: PackedValue = !0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sequence
{
    vectors: Vec<Vec<PackedValue>>,
    n_covers: usize,
    index: usize,
}

impl Sequence
{
    pub fn new(vectors: Vec<Vec<PackedValue>>) -> Self
    {
        Self {
            vectors,
            n_covers: 0,
            index: 0,
        }
    }

    pub fn vectors(&self) -> &[Vec<PackedValue>]
    {
        &self.vectors
    }

    pub fn n_covers(&self) -> usize
    {
        self.n_covers
    }

    pub fn index(&self) -> usize
    {
        self.index
    }

    pub fn set_index(&mut self, index: usize)
    {
        self.index = index;
    }

    pub fn increment_covers(&mut self)
    {
        self.n_covers += 1;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SatInputValue
{
    pub pi_index: usize,
    pub value: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtpgCombError
{
    EmptyPatternSet,
    InvalidBitPosition
    {
        position: usize,
    },
    InvalidPrimaryInput
    {
        index: usize,
        input_count: usize,
    },
    InvalidPrimaryOutput
    {
        index: usize,
        output_count: usize,
    },
    InvalidNode
    {
        index: usize,
        node_count: usize,
    },
    InvalidFanin
    {
        node: usize,
        fanin: usize,
        fanin_count: usize,
    },
    PatternInputCount
    {
        expected: usize,
        actual: usize,
    },
    OutputBufferCount
    {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for AtpgCombError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyPatternSet => formatter.write_str("atpg combinational simulation needs at least one pattern"),
            Self::InvalidBitPosition
            {
                position,
            } => write!(formatter, "packed bit position {position} is outside {PACKED_BITS} bits"),
            Self::InvalidPrimaryInput
            {
                index,
                input_count,
            } => write!(formatter, "primary input index {index} is outside {input_count} inputs"),
            Self::InvalidPrimaryOutput
            {
                index,
                output_count,
            } => write!(formatter, "primary output index {index} is outside {output_count} outputs"),
            Self::InvalidNode
            {
                index,
                node_count,
            } => write!(formatter, "node index {index} is outside {node_count} nodes"),
            Self::InvalidFanin
            {
                node,
                fanin,
                fanin_count,
            } => write!(
                formatter,
                "node {node} fanin index {fanin} is outside {fanin_count} fanins"
            ),
            Self::PatternInputCount
            {
                expected,
                actual,
            } => write!(formatter, "pattern has {actual} inputs, expected {expected}"),
            Self::OutputBufferCount
            {
                expected,
                actual,
            } => write!(formatter, "output buffer has {actual} entries, expected {expected}"),
        }
    }
}

impl std::error::Error for AtpgCombError {}

pub type AtpgCombResult<T> = Result<T, AtpgCombError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeFunction
{
    PrimaryInput,
    Buffer
    {
        input: usize,
    },
    Not
    {
        input: usize,
    },
    And
    {
        inputs: Vec<usize>,
    },
    Or
    {
        inputs: Vec<usize>,
    },
    Xor
    {
        inputs: Vec<usize>,
    },
}

impl NodeFunction
{
    fn fanins(&self) -> Vec<usize>
    {
        match self
        {
            Self::PrimaryInput => Vec::new(),
            Self::Buffer
            {
                input,
            }
            | Self::Not
            {
                input,
            } => vec![*input],
            Self::And
            {
                inputs,
            }
            | Self::Or
            {
                inputs,
            }
            | Self::Xor
            {
                inputs,
            } => inputs.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimNode
{
    function: NodeFunction,
    fanout: Vec<usize>,
    value: PackedValue,
    or_input_masks: Vec<PackedValue>,
    and_input_masks: Vec<PackedValue>,
    or_output_mask: PackedValue,
    and_output_mask: PackedValue,
}

impl SimNode
{
    pub fn new(function: NodeFunction) -> Self
    {
        let fanin_count = function.fanins().len();

        Self {
            function,
            fanout: Vec::new(),
            value: ALL_ONE,
            or_input_masks: vec![ALL_ZERO; fanin_count],
            and_input_masks: vec![ALL_ONE; fanin_count],
            or_output_mask: ALL_ZERO,
            and_output_mask: ALL_ONE,
        }
    }

    pub fn value(&self) -> PackedValue
    {
        self.value
    }

    pub fn fanout(&self) -> &[usize]
    {
        &self.fanout
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StuckAt
{
    Zero,
    One,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fault
{
    pub node: usize,
    pub fanin: Option<usize>,
    pub stuck_at: StuckAt,
    pub is_covered: bool,
    pub sequence_index: Option<usize>,
    pub sequence: Option<Sequence>,
}

impl Fault
{
    pub fn output(node: usize, stuck_at: StuckAt) -> Self
    {
        Self {
            node,
            fanin: None,
            stuck_at,
            is_covered: false,
            sequence_index: None,
            sequence: None,
        }
    }

    pub fn input(node: usize, fanin: usize, stuck_at: StuckAt) -> Self
    {
        Self {
            node,
            fanin: Some(fanin),
            stuck_at,
            is_covered: false,
            sequence_index: None,
            sequence: None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FaultPattern
{
    pub node: usize,
    pub fanin: Option<usize>,
    pub value: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgCombSimulator
{
    nodes: Vec<SimNode>,
    pi_uid: Vec<usize>,
    po_uid: Vec<usize>,
    tfo: Vec<usize>,
}

impl AtpgCombSimulator
{
    pub fn new(
        mut nodes: Vec<SimNode>,
        pi_uid: Vec<usize>,
        po_uid: Vec<usize>,
    ) -> AtpgCombResult<Self>
    {
        let node_count = nodes.len();

        for &index in &pi_uid
        {
            validate_node_index(index, node_count)?;
        }

        for &index in &po_uid
        {
            validate_node_index(index, node_count)?;
        }

        for index in 0..node_count
        {
            let fanins = nodes[index].function.fanins();

            for (fanin_index, &fanin) in fanins.iter().enumerate()
            {
                validate_node_index(fanin, node_count).map_err(|_| AtpgCombError::InvalidFanin {
                    node: index,
                    fanin: fanin_index,
                    fanin_count: fanins.len(),
                })?;
                nodes[fanin].fanout.push(index);
            }
        }

        Ok(Self {
            nodes,
            pi_uid,
            po_uid,
            tfo: Vec::new(),
        })
    }

    pub fn nodes(&self) -> &[SimNode]
    {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut [SimNode]
    {
        &mut self.nodes
    }

    pub fn primary_input_count(&self) -> usize
    {
        self.pi_uid.len()
    }

    pub fn primary_output_count(&self) -> usize
    {
        self.po_uid.len()
    }

    pub fn simulate_network(&mut self, pi_values: &[PackedValue]) -> AtpgCombResult<Vec<PackedValue>>
    {
        self.set_primary_inputs(pi_values)?;

        for index in 0..self.nodes.len()
        {
            self.evaluate_node(index);
        }

        Ok(self.primary_output_values())
    }

    pub fn set_primary_inputs(&mut self, pi_values: &[PackedValue]) -> AtpgCombResult<()>
    {
        if pi_values.len() != self.pi_uid.len()
        {
            return Err(AtpgCombError::PatternInputCount {
                expected: self.pi_uid.len(),
                actual: pi_values.len(),
            });
        }

        for (index, &node_index) in self.pi_uid.iter().enumerate()
        {
            self.nodes[node_index].value = pi_values[index];
        }

        Ok(())
    }

    pub fn primary_output_values(&self) -> Vec<PackedValue>
    {
        self.po_uid
            .iter()
            .map(|&node_index| self.nodes[node_index].value)
            .collect()
    }

    pub fn simulate_tfo(&mut self, node_index: usize) -> AtpgCombResult<()>
    {
        validate_node_index(node_index, self.nodes.len())?;

        self.tfo.clear();
        let mut visited = HashSet::new();
        self.collect_tfo(node_index, &mut visited);

        for node_index in self.tfo.clone().into_iter().rev()
        {
            self.evaluate_node(node_index);
        }

        Ok(())
    }

    pub fn single_fault_simulate(
        &mut self,
        exdc: Option<&mut Self>,
        word_vectors: &[Vec<PackedValue>],
        faults: &mut Vec<Fault>,
    ) -> AtpgCombResult<Vec<Fault>>
    {
        let pattern = word_vectors.first().ok_or(AtpgCombError::EmptyPatternSet)?;
        let true_values = self.simulate_network(pattern)?;
        let exdc_values = if let Some(exdc) = exdc
        {
            Some(exdc.simulate_network(pattern)?)
        }
        else
        {
            None
        };

        let mut covered_faults = Vec::new();
        let mut remaining_faults = Vec::new();
        let mut previous_fault: Option<Fault> = None;

        for mut fault in faults.drain(..)
        {
            self.set_single_fault_masks(&fault)?;

            if let Some(previous) = &previous_fault
            {
                if matches!(self.nodes[previous.node].function, NodeFunction::PrimaryInput)
                {
                    self.set_primary_inputs(pattern)?;
                }

                self.simulate_tfo(previous.node)?;
            }

            self.simulate_tfo(fault.node)?;
            let po_values = self.primary_output_values();
            self.reset_single_fault_masks(&fault)?;

            let covered = record_single_fault_coverage(
                &mut fault,
                &true_values,
                &po_values,
                exdc_values.as_deref(),
            )?;
            previous_fault = Some(fault.clone());

            if covered
            {
                covered_faults.push(fault);
            }
            else
            {
                remaining_faults.push(fault);
            }
        }

        *faults = remaining_faults;
        Ok(covered_faults)
    }

    pub fn fault_pattern_simulate(&mut self, faults: &mut Vec<Fault>) -> AtpgCombResult<()>
    {
        for chunk_start in (0..faults.len()).step_by(PACKED_BITS)
        {
            let chunk_end = usize::min(chunk_start + PACKED_BITS, faults.len());
            let chunk = &mut faults[chunk_start..chunk_end];
            let mut faults_ptr: Vec<Option<Fault>> = vec![None; PACKED_BITS];

            for (index, fault) in chunk.iter().cloned().enumerate()
            {
                if fault.sequence.is_some()
                {
                    faults_ptr[index] = Some(fault);
                }
            }

            let mut word_vectors = Vec::new();
            fillin_word_vectors_from_faults(&faults_ptr, &mut word_vectors, self.pi_uid.len())?;

            let Some(pattern) = word_vectors.first()
            else
            {
                continue;
            };

            let true_outputs = self.simulate_network(pattern)?;
            self.set_parallel_fault_masks(&faults_ptr)?;
            let faulty_outputs = self.simulate_network(pattern)?;
            self.reset_parallel_fault_masks(&faults_ptr)?;

            record_fault_pattern_coverage(&mut faults_ptr, &true_outputs, &faulty_outputs)?;

            for (index, fault) in faults_ptr.into_iter().enumerate().take(chunk.len())
            {
                if let Some(fault) = fault
                {
                    chunk[index].is_covered = fault.is_covered;
                }
            }
        }

        faults.retain(|fault| !fault.is_covered);
        Ok(())
    }

    fn collect_tfo(&mut self, node_index: usize, visited: &mut HashSet<usize>)
    {
        if !visited.insert(node_index)
        {
            return;
        }

        for fanout in self.nodes[node_index].fanout.clone()
        {
            self.collect_tfo(fanout, visited);
        }

        self.tfo.push(node_index);
    }

    fn evaluate_node(&mut self, node_index: usize)
    {
        let value = match self.nodes[node_index].function.clone()
        {
            NodeFunction::PrimaryInput => self.nodes[node_index].value,
            NodeFunction::Buffer
            {
                input,
            } => self.input_value(node_index, 0, input),
            NodeFunction::Not
            {
                input,
            } => !self.input_value(node_index, 0, input),
            NodeFunction::And
            {
                inputs,
            } => inputs
                .iter()
                .enumerate()
                .fold(ALL_ONE, |acc, (fanin_index, &input)| {
                    acc & self.input_value(node_index, fanin_index, input)
                }),
            NodeFunction::Or
            {
                inputs,
            } => inputs
                .iter()
                .enumerate()
                .fold(ALL_ZERO, |acc, (fanin_index, &input)| {
                    acc | self.input_value(node_index, fanin_index, input)
                }),
            NodeFunction::Xor
            {
                inputs,
            } => inputs
                .iter()
                .enumerate()
                .fold(ALL_ZERO, |acc, (fanin_index, &input)| {
                    acc ^ self.input_value(node_index, fanin_index, input)
                }),
        };

        self.nodes[node_index].value =
            (value | self.nodes[node_index].or_output_mask) & self.nodes[node_index].and_output_mask;
    }

    fn input_value(&self, node_index: usize, fanin_index: usize, input: usize) -> PackedValue
    {
        (self.nodes[input].value | self.nodes[node_index].or_input_masks[fanin_index])
            & self.nodes[node_index].and_input_masks[fanin_index]
    }

    fn set_single_fault_masks(&mut self, fault: &Fault) -> AtpgCombResult<()>
    {
        self.set_fault_masks(fault, ALL_ONE)
    }

    fn reset_single_fault_masks(&mut self, fault: &Fault) -> AtpgCombResult<()>
    {
        self.reset_fault_masks(fault)
    }

    fn set_parallel_fault_masks(&mut self, faults: &[Option<Fault>]) -> AtpgCombResult<()>
    {
        for (bit, fault) in faults.iter().enumerate()
        {
            if let Some(fault) = fault
            {
                self.set_fault_masks(fault, 1 << bit)?;
            }
        }

        Ok(())
    }

    fn reset_parallel_fault_masks(&mut self, faults: &[Option<Fault>]) -> AtpgCombResult<()>
    {
        for fault in faults.iter().flatten()
        {
            self.reset_fault_masks(fault)?;
        }

        Ok(())
    }

    fn set_fault_masks(&mut self, fault: &Fault, bit_mask: PackedValue) -> AtpgCombResult<()>
    {
        validate_node_index(fault.node, self.nodes.len())?;

        match fault.fanin
        {
            Some(fanin) =>
            {
                validate_fanin_index(fault.node, fanin, self.nodes[fault.node].or_input_masks.len())?;

                match fault.stuck_at
                {
                    StuckAt::One => self.nodes[fault.node].or_input_masks[fanin] |= bit_mask,
                    StuckAt::Zero => self.nodes[fault.node].and_input_masks[fanin] &= !bit_mask,
                }
            }
            None => match fault.stuck_at
            {
                StuckAt::One => self.nodes[fault.node].or_output_mask |= bit_mask,
                StuckAt::Zero => self.nodes[fault.node].and_output_mask &= !bit_mask,
            },
        }

        Ok(())
    }

    fn reset_fault_masks(&mut self, fault: &Fault) -> AtpgCombResult<()>
    {
        validate_node_index(fault.node, self.nodes.len())?;

        match fault.fanin
        {
            Some(fanin) =>
            {
                validate_fanin_index(fault.node, fanin, self.nodes[fault.node].or_input_masks.len())?;
                self.nodes[fault.node].or_input_masks[fanin] = ALL_ZERO;
                self.nodes[fault.node].and_input_masks[fanin] = ALL_ONE;
            }
            None =>
            {
                self.nodes[fault.node].or_output_mask = ALL_ZERO;
                self.nodes[fault.node].and_output_mask = ALL_ONE;
            }
        }

        Ok(())
    }
}

pub fn derive_comb_test(
    n_pi: usize,
    bit_position: usize,
    sat_inputs: &[SatInputValue],
    word_vectors: &mut Vec<Vec<PackedValue>>,
) -> AtpgCombResult<Sequence>
{
    if bit_position >= PACKED_BITS
    {
        return Err(AtpgCombError::InvalidBitPosition {
            position: bit_position,
        });
    }

    if word_vectors.is_empty()
    {
        lengthen_word_vectors(word_vectors, 1, n_pi);
    }

    let mut vector = vec![ALL_ONE; n_pi];

    for sat_input in sat_inputs
    {
        if sat_input.pi_index >= n_pi
        {
            return Err(AtpgCombError::InvalidPrimaryInput {
                index: sat_input.pi_index,
                input_count: n_pi,
            });
        }

        if sat_input.value
        {
            vector[sat_input.pi_index] = ALL_ONE;
            word_vectors[0][sat_input.pi_index] |= 1 << bit_position;
        }
        else
        {
            vector[sat_input.pi_index] = ALL_ZERO;
            word_vectors[0][sat_input.pi_index] &= !(1 << bit_position);
        }
    }

    Ok(Sequence::new(vec![vector]))
}

pub fn lengthen_word_vectors(
    word_vectors: &mut Vec<Vec<PackedValue>>,
    n_vectors: usize,
    n_pi: usize,
)
{
    word_vectors.extend((0..n_vectors).map(|_| vec![ALL_ONE; n_pi]));
}

pub fn reset_word_vectors(word_vectors: &mut [Vec<PackedValue>], n_pi: usize)
{
    for vector in word_vectors
    {
        vector.resize(n_pi, ALL_ONE);
        vector.fill(ALL_ONE);
    }
}

pub fn fillin_word_vectors_from_faults(
    faults: &[Option<Fault>],
    word_vectors: &mut Vec<Vec<PackedValue>>,
    n_pi: usize,
) -> AtpgCombResult<()>
{
    for (bit, fault) in faults.iter().enumerate()
    {
        if bit >= PACKED_BITS
        {
            return Err(AtpgCombError::InvalidBitPosition {
                position: bit,
            });
        }

        let Some(fault) = fault
        else
        {
            continue;
        };

        let Some(sequence) = &fault.sequence
        else
        {
            continue;
        };

        if word_vectors.len() < sequence.vectors.len()
        {
            lengthen_word_vectors(word_vectors, sequence.vectors.len() - word_vectors.len(), n_pi);
        }

        for (vector_index, vector) in sequence.vectors.iter().enumerate()
        {
            if vector.len() != n_pi
            {
                return Err(AtpgCombError::PatternInputCount {
                    expected: n_pi,
                    actual: vector.len(),
                });
            }

            for (pi_index, &value) in vector.iter().enumerate()
            {
                if value == ALL_ZERO
                {
                    word_vectors[vector_index][pi_index] &= !(1 << bit);
                }
                else
                {
                    word_vectors[vector_index][pi_index] |= 1 << bit;
                }
            }
        }
    }

    Ok(())
}

pub fn record_single_fault_coverage(
    fault: &mut Fault,
    true_values: &[PackedValue],
    po_values: &[PackedValue],
    exdc_values: Option<&[PackedValue]>,
) -> AtpgCombResult<bool>
{
    if po_values.len() != true_values.len()
    {
        return Err(AtpgCombError::OutputBufferCount {
            expected: true_values.len(),
            actual: po_values.len(),
        });
    }

    if let Some(exdc_values) = exdc_values
    {
        if exdc_values.len() != true_values.len()
        {
            return Err(AtpgCombError::OutputBufferCount {
                expected: true_values.len(),
                actual: exdc_values.len(),
            });
        }
    }

    for output_index in (0..true_values.len()).rev()
    {
        let true_value = true_values[output_index];
        let faulty_value = po_values[output_index];
        let dc_value = exdc_values.map_or(ALL_ZERO, |values| values[output_index]);

        if (true_value ^ faulty_value) & !dc_value != 0
        {
            for bit in 0..PACKED_BITS
            {
                if !extract_bit(dc_value, bit) && extract_bit(true_value, bit) != extract_bit(faulty_value, bit)
                {
                    fault.sequence_index = Some(bit);
                    fault.is_covered = true;
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

pub fn record_fault_pattern_coverage(
    faults: &mut [Option<Fault>],
    true_outputs: &[PackedValue],
    faulty_outputs: &[PackedValue],
) -> AtpgCombResult<()>
{
    if faulty_outputs.len() != true_outputs.len()
    {
        return Err(AtpgCombError::OutputBufferCount {
            expected: true_outputs.len(),
            actual: faulty_outputs.len(),
        });
    }

    for (bit, fault) in faults.iter_mut().enumerate()
    {
        let Some(fault) = fault
        else
        {
            continue;
        };

        for output_index in 0..true_outputs.len()
        {
            if extract_bit(true_outputs[output_index], bit) != extract_bit(faulty_outputs[output_index], bit)
            {
                fault.is_covered = true;
                break;
            }
        }
    }

    Ok(())
}

fn validate_node_index(index: usize, node_count: usize) -> AtpgCombResult<()>
{
    if index >= node_count
    {
        return Err(AtpgCombError::InvalidNode {
            index,
            node_count,
        });
    }

    Ok(())
}

fn validate_fanin_index(node: usize, fanin: usize, fanin_count: usize) -> AtpgCombResult<()>
{
    if fanin >= fanin_count
    {
        return Err(AtpgCombError::InvalidFanin {
            node,
            fanin,
            fanin_count,
        });
    }

    Ok(())
}

fn extract_bit(value: PackedValue, bit: usize) -> bool
{
    (value & (1 << bit)) != 0
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn derive_comb_test_defaults_unassigned_inputs_to_one_and_updates_word_vector()
    {
        let mut word_vectors = Vec::new();

        let sequence = derive_comb_test(
            3,
            2,
            &[
                SatInputValue {
                    pi_index: 0,
                    value: false,
                },
                SatInputValue {
                    pi_index: 2,
                    value: true,
                },
            ],
            &mut word_vectors,
        )
        .unwrap();

        assert_eq!(sequence.vectors(), &[vec![ALL_ZERO, ALL_ONE, ALL_ONE]]);
        assert_eq!(word_vectors, vec![vec![!4, ALL_ONE, ALL_ONE]]);
    }

    #[test]
    fn simulate_network_evaluates_combinational_nodes()
    {
        let mut simulator = xor_with_buffer_output().unwrap();
        let outputs = simulator.simulate_network(&[0b1010, 0b1100]).unwrap();

        assert_eq!(outputs, vec![0b0110]);
    }

    #[test]
    fn single_fault_simulate_removes_detected_fault_and_records_bit_index()
    {
        let mut simulator = xor_with_buffer_output().unwrap();
        let mut faults = vec![Fault::output(2, StuckAt::Zero)];

        let covered = simulator
            .single_fault_simulate(None, &[vec![0b1010, 0b1100]], &mut faults)
            .unwrap();

        assert!(faults.is_empty());
        assert_eq!(covered.len(), 1);
        assert_eq!(covered[0].sequence_index, Some(1));
        assert!(covered[0].is_covered);
    }

    #[test]
    fn single_fault_coverage_respects_external_dont_cares()
    {
        let mut fault = Fault::output(0, StuckAt::Zero);

        let covered = record_single_fault_coverage(
            &mut fault,
            &[0b0010],
            &[0b0000],
            Some(&[0b0010]),
        )
        .unwrap();

        assert!(!covered);
        assert_eq!(fault.sequence_index, None);
    }

    #[test]
    fn tfo_resimulation_restores_previous_fault_before_next_fault()
    {
        let mut simulator = xor_with_buffer_output().unwrap();
        let mut faults = vec![Fault::output(2, StuckAt::Zero), Fault::output(1, StuckAt::Zero)];

        let covered = simulator
            .single_fault_simulate(None, &[vec![0b1010, 0b1100]], &mut faults)
            .unwrap();

        assert_eq!(covered.len(), 2);
        assert!(faults.is_empty());
        assert_eq!(covered[0].sequence_index, Some(1));
        assert_eq!(covered[1].sequence_index, Some(2));
    }

    #[test]
    fn fillin_word_vectors_packs_fault_sequences_by_fault_slot()
    {
        let mut faults = vec![None; 3];
        let mut fault = Fault::output(1, StuckAt::Zero);
        fault.sequence = Some(Sequence::new(vec![vec![ALL_ZERO, ALL_ONE]]));
        faults[2] = Some(fault);

        let mut word_vectors = Vec::new();
        fillin_word_vectors_from_faults(&faults, &mut word_vectors, 2).unwrap();

        assert_eq!(word_vectors, vec![vec![!4, ALL_ONE]]);
    }

    #[test]
    fn record_fault_pattern_coverage_marks_only_different_slots()
    {
        let mut faults = vec![
            Some(Fault::output(0, StuckAt::Zero)),
            Some(Fault::output(1, StuckAt::One)),
            Some(Fault::output(2, StuckAt::Zero)),
        ];

        record_fault_pattern_coverage(&mut faults, &[0b101], &[0b100]).unwrap();

        assert!(faults[0].as_ref().unwrap().is_covered);
        assert!(!faults[1].as_ref().unwrap().is_covered);
        assert!(!faults[2].as_ref().unwrap().is_covered);
    }

    fn xor_with_buffer_output() -> AtpgCombResult<AtpgCombSimulator>
    {
        AtpgCombSimulator::new(
            vec![
                SimNode::new(NodeFunction::PrimaryInput),
                SimNode::new(NodeFunction::PrimaryInput),
                SimNode::new(NodeFunction::Xor {
                    inputs: vec![0, 1],
                }),
                SimNode::new(NodeFunction::Buffer {
                    input: 2,
                }),
            ],
            vec![0, 1],
            vec![3],
        )
    }
}
