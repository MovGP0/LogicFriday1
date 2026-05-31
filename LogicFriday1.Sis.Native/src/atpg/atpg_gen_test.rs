use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sequence {
    vectors: Vec<Vec<u32>>,
}

impl Sequence {
    pub fn new(vectors: Vec<Vec<u32>>) -> Self
    {
        Self { vectors }
    }

    pub fn vectors(&self) -> &[Vec<u32>]
    {
        &self.vectors
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultStatus {
    Unknown,
    Tested,
    Redundant,
    Aborted,
}

impl Default for FaultStatus {
    fn default() -> Self
    {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RedundancyType {
    Control,
    Observe,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Fault {
    status: FaultStatus,
    redundancy_type: Option<RedundancyType>,
    is_covered: bool,
}

impl Fault {
    pub fn status(&self) -> FaultStatus
    {
        self.status
    }

    pub fn redundancy_type(&self) -> Option<RedundancyType>
    {
        self.redundancy_type
    }

    pub fn is_covered(&self) -> bool
    {
        self.is_covered
    }

    pub fn set_covered(&mut self, is_covered: bool)
    {
        self.is_covered = is_covered;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SatResult {
    Absurd,
    GaveUp,
    Solved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgOptions {
    pub fast_sat: bool,
    pub verbosity: usize,
    pub use_internal_states: bool,
    pub random_prop: bool,
    pub deterministic_prop: bool,
    pub build_product_machines: bool,
}

impl Default for AtpgOptions {
    fn default() -> Self
    {
        Self {
            fast_sat: false,
            verbosity: 0,
            use_internal_states: false,
            random_prop: false,
            deterministic_prop: true,
            build_product_machines: true,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Statistics {
    pub sat_red: usize,
    pub n_just_reused: usize,
    pub n_random_propagations: usize,
    pub n_random_propagated: usize,
    pub n_det_propagations: usize,
    pub n_prop_reused: usize,
    pub n_ff_propagated: usize,
    pub n_not_ff_propagated: usize,
    pub n_verifications: usize,
    pub verified_red: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TimeInfo {
    pub sat_clauses: u64,
    pub sat_solve: u64,
    pub justify: u64,
    pub random_propagate: u64,
    pub ff_propagate: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgInfo {
    pub sequential: bool,
    pub n_real_pi: usize,
    pub n_latch: usize,
    pub options: AtpgOptions,
    pub statistics: Statistics,
    pub time_info: TimeInfo,
}

impl AtpgInfo {
    pub fn combinational(n_real_pi: usize) -> Self
    {
        Self {
            sequential: false,
            n_real_pi,
            n_latch: 0,
            options: AtpgOptions::default(),
            statistics: Statistics::default(),
            time_info: TimeInfo::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SequentialInfo {
    pub product_machine_built: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExcitationStates {
    pub has_already_justified_state: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredSequence {
    pub key: i32,
    pub vectors: Vec<Vec<u32>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtpgGenTestError {
    UnreachableExcitationStates,
    MissingStateSequence,
    InvalidJustificationLength,
    Backend(String),
}

impl fmt::Display for AtpgGenTestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::UnreachableExcitationStates => {
                formatter.write_str("SAT returned unreachable excitation states")
            }
            Self::MissingStateSequence => {
                formatter.write_str("internal-state justification did not find a stored sequence")
            }
            Self::InvalidJustificationLength => {
                formatter.write_str("justification did not produce any usable vectors")
            }
            Self::Backend(message) => formatter.write_str(message),
        }
    }
}

impl Error for AtpgGenTestError {}

pub trait AtpgGenTestEngine {
    fn cpu_time(&mut self) -> u64
    {
        0
    }

    fn reset_sat(&mut self) -> Result<(), AtpgGenTestError>;

    fn network_fault_clauses(
        &mut self,
        fault: &Fault,
        seq_info: &SequentialInfo,
    ) -> Result<usize, AtpgGenTestError>;

    fn solve_sat(
        &mut self,
        fast_sat: bool,
        verbosity: usize,
    ) -> Result<SatResult, AtpgGenTestError>;

    fn derive_excitation_vector(
        &mut self,
        n_pi_vars: usize,
        width: usize,
    ) -> Result<Vec<u32>, AtpgGenTestError>;

    fn derive_excitation_states(
        &mut self,
        n_pi_vars: usize,
    ) -> Result<ExcitationStates, AtpgGenTestError>;

    fn excitation_states_are_reachable(
        &mut self,
        _states: &ExcitationStates,
    ) -> Result<bool, AtpgGenTestError>
    {
        Ok(true)
    }

    fn reuse_just_sequence(
        &mut self,
        n_real_pi: usize,
        n_latch: usize,
        states: &ExcitationStates,
    ) -> Result<usize, AtpgGenTestError>;

    fn state_justify(
        &mut self,
        info: &AtpgInfo,
        states: &ExcitationStates,
    ) -> Result<usize, AtpgGenTestError>;

    fn internal_state_justify(
        &mut self,
        info: &AtpgInfo,
        states: &ExcitationStates,
    ) -> Result<usize, AtpgGenTestError>;

    fn store_excitation_vector(
        &mut self,
        n_just_vectors: usize,
        vector: Vec<u32>,
    ) -> Result<(), AtpgGenTestError>;

    fn minimum_just_sequence(
        &mut self,
        fault: &mut Fault,
        seq_info: &SequentialInfo,
    ) -> Result<usize, AtpgGenTestError>;

    fn derive_test_sequence(
        &mut self,
        actual_n_just_vectors: usize,
        n_prop_vectors: usize,
        n_real_pi: usize,
        count: usize,
    ) -> Result<Sequence, AtpgGenTestError>;

    fn random_propagate(
        &mut self,
        fault: &Fault,
        info: &AtpgInfo,
        seq_info: &SequentialInfo,
    ) -> Result<usize, AtpgGenTestError>;

    fn prop_key(&mut self, seq_info: &SequentialInfo) -> Result<i32, AtpgGenTestError>;

    fn inverted_prop_key(&mut self, seq_info: &SequentialInfo) -> Result<i32, AtpgGenTestError>;

    fn has_prop_sequence(&mut self, key: i32) -> Result<bool, AtpgGenTestError>;

    fn reuse_prop_sequence(
        &mut self,
        n_real_pi: usize,
        key: i32,
    ) -> Result<usize, AtpgGenTestError>;

    fn fault_free_propagate(
        &mut self,
        info: &AtpgInfo,
        seq_info: &SequentialInfo,
    ) -> Result<usize, AtpgGenTestError>;

    fn verify_test(
        &mut self,
        fault: &Fault,
        test_sequence: &Sequence,
    ) -> Result<bool, AtpgGenTestError>;

    fn start_state_is_initial(&mut self, _seq_info: &SequentialInfo) -> Result<bool, AtpgGenTestError>
    {
        Ok(true)
    }

    fn take_start_state_sequence(
        &mut self,
        _seq_info: &SequentialInfo,
    ) -> Result<Option<StoredSequence>, AtpgGenTestError>
    {
        Ok(None)
    }

    fn prepend_old_vectors_to_just_sequence(
        &mut self,
        _n_just_vectors: usize,
        _old_vectors: &[Vec<u32>],
        _n_real_pi: usize,
    ) -> Result<(), AtpgGenTestError>
    {
        Ok(())
    }

    fn simulate_entire_sequence(
        &mut self,
        _fault: &mut Fault,
        _seq_info: &SequentialInfo,
        _old_vectors: &[Vec<u32>],
    ) -> Result<(), AtpgGenTestError>
    {
        Ok(())
    }

    fn remove_used_state_sequence(
        &mut self,
        _key: i32,
        _sequence: StoredSequence,
    ) -> Result<(), AtpgGenTestError>
    {
        Ok(())
    }

    fn release_start_state_used(&mut self) -> Result<(), AtpgGenTestError>
    {
        Ok(())
    }

    fn good_faulty_product_machine_test(
        &mut self,
        fault: &Fault,
        info: &AtpgInfo,
        seq_info: &SequentialInfo,
    ) -> Result<usize, AtpgGenTestError>;

    fn product_start_state_is_initial(
        &mut self,
        _seq_info: &SequentialInfo,
    ) -> Result<bool, AtpgGenTestError>
    {
        Ok(true)
    }

    fn take_product_start_state_sequence(
        &mut self,
        _info: &AtpgInfo,
        _seq_info: &SequentialInfo,
    ) -> Result<Option<StoredSequence>, AtpgGenTestError>
    {
        Ok(None)
    }

    fn copy_old_vectors_to_just_sequence(
        &mut self,
        _old_vectors: &[Vec<u32>],
        _n_real_pi: usize,
    ) -> Result<(), AtpgGenTestError>
    {
        Ok(())
    }

    fn remove_used_product_state_sequence(
        &mut self,
        _key: i32,
        _sequence: StoredSequence,
    ) -> Result<(), AtpgGenTestError>
    {
        Ok(())
    }

    fn release_product_start_state_used(&mut self) -> Result<(), AtpgGenTestError>
    {
        Ok(())
    }
}

pub fn generate_test(
    fault: &mut Fault,
    info: &mut AtpgInfo,
    seq_info: &mut SequentialInfo,
    engine: &mut impl AtpgGenTestEngine,
    count: usize,
) -> Result<Option<Sequence>, AtpgGenTestError>
{
    let mut last_time = engine.cpu_time();
    engine.reset_sat()?;
    let n_pi_vars = engine.network_fault_clauses(fault, seq_info)?;
    let mut time = engine.cpu_time();
    info.time_info.sat_clauses += time.saturating_sub(last_time);
    last_time = time;

    let sat_value = engine.solve_sat(info.options.fast_sat, info.options.verbosity)?;
    time = engine.cpu_time();
    info.time_info.sat_solve += time.saturating_sub(last_time);

    match sat_value
    {
        SatResult::Absurd => {
            fault.status = FaultStatus::Redundant;
            fault.redundancy_type = Some(RedundancyType::Control);
            info.statistics.sat_red += 1;
            Ok(None)
        }
        SatResult::GaveUp => {
            fault.status = FaultStatus::Aborted;
            Ok(None)
        }
        SatResult::Solved => {
            let test_sequence = if info.sequential
            {
                if info.options.use_internal_states
                {
                    internal_states_justify_and_ff_propagate(
                        fault,
                        info,
                        seq_info,
                        engine,
                        n_pi_vars,
                        count,
                    )?
                }
                else
                {
                    justify_and_ff_propagate(
                        fault,
                        info,
                        seq_info,
                        engine,
                        n_pi_vars,
                        count,
                    )?
                }
            }
            else
            {
                fault.is_covered = true;
                Some(Sequence::new(vec![engine.derive_excitation_vector(
                    n_pi_vars,
                    info.n_real_pi,
                )?]))
            };

            if fault.is_covered
            {
                fault.status = FaultStatus::Tested;
            }

            Ok(test_sequence)
        }
    }
}

fn justify_and_ff_propagate(
    fault: &mut Fault,
    info: &mut AtpgInfo,
    seq_info: &mut SequentialInfo,
    engine: &mut impl AtpgGenTestEngine,
    n_pi_vars: usize,
    count: usize,
) -> Result<Option<Sequence>, AtpgGenTestError>
{
    let last_time = engine.cpu_time();
    let excite_states = engine.derive_excitation_states(n_pi_vars)?;

    if !engine.excitation_states_are_reachable(&excite_states)?
    {
        return Err(AtpgGenTestError::UnreachableExcitationStates);
    }

    let mut n_just_vectors = if excite_states.has_already_justified_state
    {
        info.statistics.n_just_reused += 1;
        engine.reuse_just_sequence(info.n_real_pi, info.n_latch, &excite_states)?
    }
    else
    {
        engine.state_justify(info, &excite_states)?
    };

    let excitation_vector = engine.derive_excitation_vector(n_pi_vars, info.n_real_pi)?;
    engine.store_excitation_vector(n_just_vectors, excitation_vector)?;

    let mut actual_n_just_vectors = engine.minimum_just_sequence(fault, seq_info)?;
    if excite_states.has_already_justified_state && actual_n_just_vectors < 1
    {
        n_just_vectors = engine.state_justify(info, &excite_states)?;
        let excitation_vector = engine.derive_excitation_vector(n_pi_vars, info.n_real_pi)?;
        engine.store_excitation_vector(n_just_vectors, excitation_vector)?;
        actual_n_just_vectors = engine.minimum_just_sequence(fault, seq_info)?;
    }

    ensure_justified(actual_n_just_vectors)?;
    let time = engine.cpu_time();
    info.time_info.justify += time.saturating_sub(last_time);

    if fault.is_covered
    {
        return Ok(Some(engine.derive_test_sequence(
            actual_n_just_vectors,
            0,
            info.n_real_pi,
            count,
        )?));
    }

    propagate_fault(fault, info, seq_info, engine, actual_n_just_vectors, count)
}

fn internal_states_justify_and_ff_propagate(
    fault: &mut Fault,
    info: &mut AtpgInfo,
    seq_info: &mut SequentialInfo,
    engine: &mut impl AtpgGenTestEngine,
    n_pi_vars: usize,
    count: usize,
) -> Result<Option<Sequence>, AtpgGenTestError>
{
    let last_time = engine.cpu_time();
    let excite_states = engine.derive_excitation_states(n_pi_vars)?;

    if !engine.excitation_states_are_reachable(&excite_states)?
    {
        return Err(AtpgGenTestError::UnreachableExcitationStates);
    }

    let n_just_vectors = engine.internal_state_justify(info, &excite_states)?;
    let excitation_vector = engine.derive_excitation_vector(n_pi_vars, info.n_real_pi)?;
    engine.store_excitation_vector(n_just_vectors, excitation_vector)?;

    let old_sequence = if engine.start_state_is_initial(seq_info)?
    {
        None
    }
    else
    {
        let sequence = engine
            .take_start_state_sequence(seq_info)?
            .ok_or(AtpgGenTestError::MissingStateSequence)?;
        engine.prepend_old_vectors_to_just_sequence(
            n_just_vectors + 1,
            &sequence.vectors,
            info.n_real_pi,
        )?;
        Some(sequence)
    };

    let n_old_vectors = old_sequence
        .as_ref()
        .map(|sequence| sequence.vectors.len())
        .unwrap_or(0);
    let mut actual_n_just_vectors = engine.minimum_just_sequence(fault, seq_info)?;

    if actual_n_just_vectors < n_old_vectors
    {
        fault.is_covered = false;
        let old_vectors = old_sequence
            .as_ref()
            .map(|sequence| sequence.vectors.as_slice())
            .unwrap_or(&[]);
        engine.simulate_entire_sequence(fault, seq_info, old_vectors)?;
        actual_n_just_vectors = n_old_vectors;
    }

    ensure_justified(actual_n_just_vectors)?;
    let time = engine.cpu_time();
    info.time_info.justify += time.saturating_sub(last_time);

    let test_sequence = if fault.is_covered
    {
        Some(engine.derive_test_sequence(
            actual_n_just_vectors,
            0,
            info.n_real_pi,
            count,
        )?)
    }
    else
    {
        propagate_fault(fault, info, seq_info, engine, actual_n_just_vectors, count)?
    };

    if let Some(sequence) = old_sequence
    {
        if fault.is_covered
        {
            engine.remove_used_state_sequence(sequence.key, sequence)?;
        }
    }

    engine.release_start_state_used()?;
    Ok(test_sequence)
}

fn propagate_fault(
    fault: &mut Fault,
    info: &mut AtpgInfo,
    seq_info: &mut SequentialInfo,
    engine: &mut impl AtpgGenTestEngine,
    actual_n_just_vectors: usize,
    count: usize,
) -> Result<Option<Sequence>, AtpgGenTestError>
{
    let mut guaranteed_test = false;
    let mut n_prop_vectors = 0;

    if info.options.random_prop
    {
        info.statistics.n_random_propagations += 1;
        let last_time = engine.cpu_time();
        n_prop_vectors = engine.random_propagate(fault, info, seq_info)?;
        let time = engine.cpu_time();
        info.time_info.random_propagate += time.saturating_sub(last_time);

        if n_prop_vectors > 0
        {
            guaranteed_test = true;
            fault.is_covered = true;
            info.statistics.n_random_propagated += 1;
        }
    }

    if n_prop_vectors == 0
        && info.options.deterministic_prop
        && info.options.build_product_machines
        && seq_info.product_machine_built
    {
        let last_time = engine.cpu_time();
        info.statistics.n_det_propagations += 1;
        let key = engine.prop_key(seq_info)?;

        if engine.has_prop_sequence(key)?
        {
            info.statistics.n_prop_reused += 1;
            n_prop_vectors = engine.reuse_prop_sequence(info.n_real_pi, key)?;
        }
        else
        {
            let inverted_key = engine.inverted_prop_key(seq_info)?;
            if engine.has_prop_sequence(inverted_key)?
            {
                info.statistics.n_prop_reused += 1;
                n_prop_vectors = engine.reuse_prop_sequence(info.n_real_pi, inverted_key)?;
            }
            else
            {
                n_prop_vectors = engine.fault_free_propagate(info, seq_info)?;
            }
        }

        let time = engine.cpu_time();
        info.time_info.ff_propagate += time.saturating_sub(last_time);
    }

    if n_prop_vectors == 0
    {
        return Ok(None);
    }

    let test_sequence = engine.derive_test_sequence(
        actual_n_just_vectors,
        n_prop_vectors,
        info.n_real_pi,
        count,
    )?;

    if guaranteed_test
    {
        return Ok(Some(test_sequence));
    }

    let last_time = engine.cpu_time();
    let verified = engine.verify_test(fault, &test_sequence)?;
    let time = engine.cpu_time();
    info.time_info.ff_propagate += time.saturating_sub(last_time);

    if verified
    {
        fault.is_covered = true;
        info.statistics.n_ff_propagated += 1;
        Ok(Some(test_sequence))
    }
    else
    {
        info.statistics.n_not_ff_propagated += 1;
        Ok(None)
    }
}

pub fn generate_test_using_verification(
    fault: &mut Fault,
    info: &mut AtpgInfo,
    seq_info: &mut SequentialInfo,
    engine: &mut impl AtpgGenTestEngine,
    count: usize,
) -> Result<Option<Sequence>, AtpgGenTestError>
{
    assert!(seq_info.product_machine_built);
    info.statistics.n_verifications += 1;

    let n_test_vectors = engine.good_faulty_product_machine_test(fault, info, seq_info)?;
    if n_test_vectors == 0
    {
        fault.status = FaultStatus::Redundant;
        fault.redundancy_type = Some(RedundancyType::Observe);
        info.statistics.verified_red += 1;
        return Ok(None);
    }

    let mut tested = true;
    let mut internal_state_sequence = None;
    let mut n_old_vectors = 0;

    if info.options.use_internal_states
        && !engine.product_start_state_is_initial(seq_info)?
    {
        let sequence = engine
            .take_product_start_state_sequence(info, seq_info)?
            .ok_or(AtpgGenTestError::MissingStateSequence)?;
        n_old_vectors = sequence.vectors.len();
        engine.copy_old_vectors_to_just_sequence(&sequence.vectors, info.n_real_pi)?;
        internal_state_sequence = Some(sequence);
    }

    let test_sequence = engine.derive_test_sequence(
        n_old_vectors,
        n_test_vectors,
        info.n_real_pi,
        count,
    )?;

    if internal_state_sequence.is_some()
    {
        tested = engine.verify_test(fault, &test_sequence)?;
    }

    if tested
    {
        fault.is_covered = true;
        fault.status = FaultStatus::Tested;

        if let Some(sequence) = internal_state_sequence
        {
            engine.remove_used_product_state_sequence(sequence.key, sequence)?;
        }

        if info.options.use_internal_states
        {
            engine.release_product_start_state_used()?;
        }

        Ok(Some(test_sequence))
    }
    else
    {
        if info.options.use_internal_states
        {
            engine.release_product_start_state_used()?;
        }

        Ok(None)
    }
}

fn ensure_justified(actual_n_just_vectors: usize) -> Result<(), AtpgGenTestError>
{
    if actual_n_just_vectors == 0
    {
        Err(AtpgGenTestError::InvalidJustificationLength)
    }
    else
    {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, VecDeque};

    #[derive(Clone, Debug)]
    struct FakeEngine {
        sat_result: SatResult,
        n_pi_vars: usize,
        excitation_vector: Vec<u32>,
        excitation_states: ExcitationStates,
        reachable: bool,
        reused_just_vectors: usize,
        state_just_vectors: VecDeque<usize>,
        internal_just_vectors: usize,
        minimum_just_vectors: VecDeque<usize>,
        random_prop_vectors: usize,
        prop_key: i32,
        inverted_prop_key: i32,
        prop_sequences: HashMap<i32, usize>,
        fault_free_prop_vectors: usize,
        verify_result: bool,
        start_initial: bool,
        product_start_initial: bool,
        start_sequence: Option<StoredSequence>,
        product_start_sequence: Option<StoredSequence>,
        good_faulty_vectors: usize,
        stored_excitation: Vec<(usize, Vec<u32>)>,
        removed_state_keys: Vec<i32>,
        removed_product_keys: Vec<i32>,
        copied_old_vectors: Vec<Vec<u32>>,
        released_start_state: bool,
        released_product_start_state: bool,
    }

    impl Default for FakeEngine {
        fn default() -> Self
        {
            Self {
                sat_result: SatResult::Solved,
                n_pi_vars: 2,
                excitation_vector: vec![1, 0],
                excitation_states: ExcitationStates {
                    has_already_justified_state: false,
                },
                reachable: true,
                reused_just_vectors: 1,
                state_just_vectors: VecDeque::from([1]),
                internal_just_vectors: 1,
                minimum_just_vectors: VecDeque::from([1]),
                random_prop_vectors: 0,
                prop_key: 7,
                inverted_prop_key: -7,
                prop_sequences: HashMap::new(),
                fault_free_prop_vectors: 0,
                verify_result: true,
                start_initial: true,
                product_start_initial: true,
                start_sequence: None,
                product_start_sequence: None,
                good_faulty_vectors: 0,
                stored_excitation: Vec::new(),
                removed_state_keys: Vec::new(),
                removed_product_keys: Vec::new(),
                copied_old_vectors: Vec::new(),
                released_start_state: false,
                released_product_start_state: false,
            }
        }
    }

    impl AtpgGenTestEngine for FakeEngine {
        fn reset_sat(&mut self) -> Result<(), AtpgGenTestError>
        {
            Ok(())
        }

        fn network_fault_clauses(
            &mut self,
            _fault: &Fault,
            _seq_info: &SequentialInfo,
        ) -> Result<usize, AtpgGenTestError>
        {
            Ok(self.n_pi_vars)
        }

        fn solve_sat(
            &mut self,
            _fast_sat: bool,
            _verbosity: usize,
        ) -> Result<SatResult, AtpgGenTestError>
        {
            Ok(self.sat_result)
        }

        fn derive_excitation_vector(
            &mut self,
            _n_pi_vars: usize,
            width: usize,
        ) -> Result<Vec<u32>, AtpgGenTestError>
        {
            Ok(self.excitation_vector.iter().copied().take(width).collect())
        }

        fn derive_excitation_states(
            &mut self,
            _n_pi_vars: usize,
        ) -> Result<ExcitationStates, AtpgGenTestError>
        {
            Ok(self.excitation_states.clone())
        }

        fn excitation_states_are_reachable(
            &mut self,
            _states: &ExcitationStates,
        ) -> Result<bool, AtpgGenTestError>
        {
            Ok(self.reachable)
        }

        fn reuse_just_sequence(
            &mut self,
            _n_real_pi: usize,
            _n_latch: usize,
            _states: &ExcitationStates,
        ) -> Result<usize, AtpgGenTestError>
        {
            Ok(self.reused_just_vectors)
        }

        fn state_justify(
            &mut self,
            _info: &AtpgInfo,
            _states: &ExcitationStates,
        ) -> Result<usize, AtpgGenTestError>
        {
            Ok(self.state_just_vectors.pop_front().unwrap_or(1))
        }

        fn internal_state_justify(
            &mut self,
            _info: &AtpgInfo,
            _states: &ExcitationStates,
        ) -> Result<usize, AtpgGenTestError>
        {
            Ok(self.internal_just_vectors)
        }

        fn store_excitation_vector(
            &mut self,
            n_just_vectors: usize,
            vector: Vec<u32>,
        ) -> Result<(), AtpgGenTestError>
        {
            self.stored_excitation.push((n_just_vectors, vector));
            Ok(())
        }

        fn minimum_just_sequence(
            &mut self,
            fault: &mut Fault,
            _seq_info: &SequentialInfo,
        ) -> Result<usize, AtpgGenTestError>
        {
            let value = self.minimum_just_vectors.pop_front().unwrap_or(1);
            if value > 0 && self.fault_free_prop_vectors == 0 && self.random_prop_vectors == 0
            {
                fault.set_covered(true);
            }
            Ok(value)
        }

        fn derive_test_sequence(
            &mut self,
            actual_n_just_vectors: usize,
            n_prop_vectors: usize,
            n_real_pi: usize,
            count: usize,
        ) -> Result<Sequence, AtpgGenTestError>
        {
            let total = actual_n_just_vectors + n_prop_vectors;
            Ok(Sequence::new(
                (0..total)
                    .map(|index| vec![count as u32, index as u32, n_real_pi as u32])
                    .collect(),
            ))
        }

        fn random_propagate(
            &mut self,
            _fault: &Fault,
            _info: &AtpgInfo,
            _seq_info: &SequentialInfo,
        ) -> Result<usize, AtpgGenTestError>
        {
            Ok(self.random_prop_vectors)
        }

        fn prop_key(&mut self, _seq_info: &SequentialInfo) -> Result<i32, AtpgGenTestError>
        {
            Ok(self.prop_key)
        }

        fn inverted_prop_key(&mut self, _seq_info: &SequentialInfo) -> Result<i32, AtpgGenTestError>
        {
            Ok(self.inverted_prop_key)
        }

        fn has_prop_sequence(&mut self, key: i32) -> Result<bool, AtpgGenTestError>
        {
            Ok(self.prop_sequences.contains_key(&key))
        }

        fn reuse_prop_sequence(
            &mut self,
            _n_real_pi: usize,
            key: i32,
        ) -> Result<usize, AtpgGenTestError>
        {
            Ok(*self.prop_sequences.get(&key).unwrap_or(&0))
        }

        fn fault_free_propagate(
            &mut self,
            _info: &AtpgInfo,
            _seq_info: &SequentialInfo,
        ) -> Result<usize, AtpgGenTestError>
        {
            Ok(self.fault_free_prop_vectors)
        }

        fn verify_test(
            &mut self,
            _fault: &Fault,
            _test_sequence: &Sequence,
        ) -> Result<bool, AtpgGenTestError>
        {
            Ok(self.verify_result)
        }

        fn start_state_is_initial(
            &mut self,
            _seq_info: &SequentialInfo,
        ) -> Result<bool, AtpgGenTestError>
        {
            Ok(self.start_initial)
        }

        fn take_start_state_sequence(
            &mut self,
            _seq_info: &SequentialInfo,
        ) -> Result<Option<StoredSequence>, AtpgGenTestError>
        {
            Ok(self.start_sequence.take())
        }

        fn prepend_old_vectors_to_just_sequence(
            &mut self,
            _n_just_vectors: usize,
            old_vectors: &[Vec<u32>],
            _n_real_pi: usize,
        ) -> Result<(), AtpgGenTestError>
        {
            self.copied_old_vectors = old_vectors.to_vec();
            Ok(())
        }

        fn simulate_entire_sequence(
            &mut self,
            fault: &mut Fault,
            _seq_info: &SequentialInfo,
            _old_vectors: &[Vec<u32>],
        ) -> Result<(), AtpgGenTestError>
        {
            fault.set_covered(true);
            Ok(())
        }

        fn remove_used_state_sequence(
            &mut self,
            key: i32,
            _sequence: StoredSequence,
        ) -> Result<(), AtpgGenTestError>
        {
            self.removed_state_keys.push(key);
            Ok(())
        }

        fn release_start_state_used(&mut self) -> Result<(), AtpgGenTestError>
        {
            self.released_start_state = true;
            Ok(())
        }

        fn good_faulty_product_machine_test(
            &mut self,
            _fault: &Fault,
            _info: &AtpgInfo,
            _seq_info: &SequentialInfo,
        ) -> Result<usize, AtpgGenTestError>
        {
            Ok(self.good_faulty_vectors)
        }

        fn product_start_state_is_initial(
            &mut self,
            _seq_info: &SequentialInfo,
        ) -> Result<bool, AtpgGenTestError>
        {
            Ok(self.product_start_initial)
        }

        fn take_product_start_state_sequence(
            &mut self,
            _info: &AtpgInfo,
            _seq_info: &SequentialInfo,
        ) -> Result<Option<StoredSequence>, AtpgGenTestError>
        {
            Ok(self.product_start_sequence.take())
        }

        fn copy_old_vectors_to_just_sequence(
            &mut self,
            old_vectors: &[Vec<u32>],
            _n_real_pi: usize,
        ) -> Result<(), AtpgGenTestError>
        {
            self.copied_old_vectors = old_vectors.to_vec();
            Ok(())
        }

        fn remove_used_product_state_sequence(
            &mut self,
            key: i32,
            _sequence: StoredSequence,
        ) -> Result<(), AtpgGenTestError>
        {
            self.removed_product_keys.push(key);
            Ok(())
        }

        fn release_product_start_state_used(&mut self) -> Result<(), AtpgGenTestError>
        {
            self.released_product_start_state = true;
            Ok(())
        }
    }

    #[test]
    fn absurd_sat_marks_fault_control_redundant()
    {
        let mut engine = FakeEngine {
            sat_result: SatResult::Absurd,
            ..FakeEngine::default()
        };
        let mut fault = Fault::default();
        let mut info = AtpgInfo::combinational(2);
        let mut seq_info = SequentialInfo::default();

        let result = generate_test(&mut fault, &mut info, &mut seq_info, &mut engine, 0).unwrap();

        assert_eq!(result, None);
        assert_eq!(fault.status(), FaultStatus::Redundant);
        assert_eq!(fault.redundancy_type(), Some(RedundancyType::Control));
        assert_eq!(info.statistics.sat_red, 1);
    }

    #[test]
    fn combinational_solved_sat_returns_excitation_vector_and_tests_fault()
    {
        let mut engine = FakeEngine {
            excitation_vector: vec![1, 0, 1],
            ..FakeEngine::default()
        };
        let mut fault = Fault::default();
        let mut info = AtpgInfo::combinational(3);
        let mut seq_info = SequentialInfo::default();

        let result = generate_test(&mut fault, &mut info, &mut seq_info, &mut engine, 12).unwrap();

        assert_eq!(result.unwrap().vectors(), &[vec![1, 0, 1]]);
        assert!(fault.is_covered());
        assert_eq!(fault.status(), FaultStatus::Tested);
    }

    #[test]
    fn sequential_reuses_justification_and_falls_back_when_minimum_is_empty()
    {
        let mut engine = FakeEngine {
            excitation_states: ExcitationStates {
                has_already_justified_state: true,
            },
            state_just_vectors: VecDeque::from([3]),
            minimum_just_vectors: VecDeque::from([0, 2]),
            ..FakeEngine::default()
        };
        let mut fault = Fault::default();
        let mut info = AtpgInfo::combinational(2);
        info.sequential = true;
        let mut seq_info = SequentialInfo::default();

        let result = generate_test(&mut fault, &mut info, &mut seq_info, &mut engine, 4).unwrap();

        assert_eq!(result.unwrap().vectors().len(), 2);
        assert_eq!(info.statistics.n_just_reused, 1);
        assert_eq!(
            engine.stored_excitation,
            vec![(1, vec![1, 0]), (3, vec![1, 0])]
        );
    }

    #[test]
    fn deterministic_propagation_reuses_inverted_sequence_and_verifies()
    {
        let mut prop_sequences = HashMap::new();
        prop_sequences.insert(-7, 2);
        let mut engine = FakeEngine {
            fault_free_prop_vectors: 1,
            prop_sequences,
            ..FakeEngine::default()
        };
        let mut fault = Fault::default();
        let mut info = AtpgInfo::combinational(2);
        info.sequential = true;
        let mut seq_info = SequentialInfo {
            product_machine_built: true,
        };

        let result = generate_test(&mut fault, &mut info, &mut seq_info, &mut engine, 9).unwrap();

        assert_eq!(result.unwrap().vectors().len(), 3);
        assert!(fault.is_covered());
        assert_eq!(info.statistics.n_det_propagations, 1);
        assert_eq!(info.statistics.n_prop_reused, 1);
        assert_eq!(info.statistics.n_ff_propagated, 1);
    }

    #[test]
    fn failed_fault_free_verification_discards_sequence()
    {
        let mut engine = FakeEngine {
            fault_free_prop_vectors: 1,
            verify_result: false,
            ..FakeEngine::default()
        };
        let mut fault = Fault::default();
        let mut info = AtpgInfo::combinational(2);
        info.sequential = true;
        let mut seq_info = SequentialInfo {
            product_machine_built: true,
        };

        let result = generate_test(&mut fault, &mut info, &mut seq_info, &mut engine, 0).unwrap();

        assert_eq!(result, None);
        assert!(!fault.is_covered());
        assert_eq!(info.statistics.n_not_ff_propagated, 1);
    }

    #[test]
    fn internal_state_generation_removes_consumed_start_sequence()
    {
        let old_sequence = StoredSequence {
            key: 42,
            vectors: vec![vec![0, 0], vec![1, 1]],
        };
        let mut engine = FakeEngine {
            start_initial: false,
            start_sequence: Some(old_sequence.clone()),
            minimum_just_vectors: VecDeque::from([1]),
            ..FakeEngine::default()
        };
        let mut fault = Fault::default();
        let mut info = AtpgInfo::combinational(2);
        info.sequential = true;
        info.options.use_internal_states = true;
        let mut seq_info = SequentialInfo::default();

        let result = generate_test(&mut fault, &mut info, &mut seq_info, &mut engine, 0).unwrap();

        assert_eq!(result.unwrap().vectors().len(), 2);
        assert_eq!(engine.copied_old_vectors, old_sequence.vectors);
        assert_eq!(engine.removed_state_keys, vec![42]);
        assert!(engine.released_start_state);
    }

    #[test]
    fn verification_redundant_path_marks_observe_redundancy()
    {
        let mut engine = FakeEngine::default();
        let mut fault = Fault::default();
        let mut info = AtpgInfo::combinational(2);
        let mut seq_info = SequentialInfo {
            product_machine_built: true,
        };

        let result = generate_test_using_verification(
            &mut fault,
            &mut info,
            &mut seq_info,
            &mut engine,
            0,
        )
        .unwrap();

        assert_eq!(result, None);
        assert_eq!(fault.status(), FaultStatus::Redundant);
        assert_eq!(fault.redundancy_type(), Some(RedundancyType::Observe));
        assert_eq!(info.statistics.n_verifications, 1);
        assert_eq!(info.statistics.verified_red, 1);
    }

    #[test]
    fn verification_with_internal_state_copies_and_removes_old_sequence()
    {
        let old_sequence = StoredSequence {
            key: 5,
            vectors: vec![vec![1, 0], vec![0, 1]],
        };
        let mut engine = FakeEngine {
            good_faulty_vectors: 3,
            product_start_initial: false,
            product_start_sequence: Some(old_sequence.clone()),
            ..FakeEngine::default()
        };
        let mut fault = Fault::default();
        let mut info = AtpgInfo::combinational(2);
        info.options.use_internal_states = true;
        let mut seq_info = SequentialInfo {
            product_machine_built: true,
        };

        let result = generate_test_using_verification(
            &mut fault,
            &mut info,
            &mut seq_info,
            &mut engine,
            8,
        )
        .unwrap();

        assert_eq!(result.unwrap().vectors().len(), 5);
        assert_eq!(engine.copied_old_vectors, old_sequence.vectors);
        assert_eq!(engine.removed_product_keys, vec![5]);
        assert!(engine.released_product_start_state);
        assert_eq!(fault.status(), FaultStatus::Tested);
    }
}
