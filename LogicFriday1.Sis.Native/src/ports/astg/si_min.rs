use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type StateCode = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind
{
    Input,
    Output,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal
{
    pub name: String,
    pub state_bit: StateCode,
    pub kind: SignalKind,
}

impl Signal
{
    pub fn new(name: impl Into<String>, state_bit: StateCode, kind: SignalKind) -> Self
    {
        Self
        {
            name: name.into(),
            state_bit,
            kind,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgState
{
    pub code: StateCode,
    pub enabled: StateCode,
    pub has_free_choice_marking: bool,
}

impl AstgState
{
    pub fn new(code: StateCode, enabled: StateCode) -> Self
    {
        Self
        {
            code,
            enabled,
            has_free_choice_marking: false,
        }
    }

    pub fn with_free_choice_marking(mut self) -> Self
    {
        self.has_free_choice_marking = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgGraph
{
    pub signals: Vec<Signal>,
    pub states: Vec<AstgState>,
    pub is_marked_graph: bool,
}

impl AstgGraph
{
    pub fn new(signals: Vec<Signal>, states: Vec<AstgState>) -> Self
    {
        Self
        {
            signals,
            states,
            is_marked_graph: true,
        }
    }

    pub fn with_free_choice_places(mut self) -> Self
    {
        self.is_marked_graph = false;
        self
    }

    fn signal_by_name(&self, name: &str) -> Option<&Signal>
    {
        self.signals.iter().find(|signal| signal.name == name)
    }

    fn signal_by_bit(&self, state_bit: StateCode) -> Option<&Signal>
    {
        self.signals
            .iter()
            .find(|signal| signal.state_bit == state_bit)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal
{
    Zero,
    One,
    DontCare,
}

impl Literal
{
    fn matches(self, value: bool) -> bool
    {
        match self
        {
            Self::Zero => !value,
            Self::One => value,
            Self::DontCare => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube
{
    pub inputs: Vec<Literal>,
    pub outputs: Vec<bool>,
}

impl Cube
{
    pub fn new(inputs: Vec<Literal>, outputs: Vec<bool>) -> Self
    {
        Self
        {
            inputs,
            outputs,
        }
    }

    pub fn minterm(bits: &[bool], outputs: Vec<bool>) -> Self
    {
        Self
        {
            inputs: bits
                .iter()
                .map(|bit| if *bit { Literal::One } else { Literal::Zero })
                .collect(),
            outputs,
        }
    }

    fn output_count(&self) -> usize
    {
        self.outputs.iter().filter(|output| **output).count()
    }

    fn covers_assignment(&self, assignment: usize, input_count: usize) -> bool
    {
        self.inputs
            .iter()
            .take(input_count)
            .enumerate()
            .all(|(index, literal)| literal.matches(bit_from_assignment(assignment, index)))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pla
{
    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
    pub on_set: Vec<Cube>,
    pub off_set: Vec<Cube>,
    pub dc_set: Vec<Cube>,
}

impl Pla
{
    pub fn new(input_names: Vec<String>, output_names: Vec<String>, on_set: Vec<Cube>) -> Self
    {
        Self
        {
            input_names,
            output_names,
            on_set,
            off_set: Vec::new(),
            dc_set: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SiMinOptions
{
    pub add_redundant_consensus: bool,
    pub reduce_simultaneous_signal_hazards: bool,
}

impl Default for SiMinOptions
{
    fn default() -> Self
    {
        Self
        {
            add_redundant_consensus: true,
            reduce_simultaneous_signal_hazards: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SiMinReport
{
    pub linked_states: usize,
    pub consensus_cubes_added: usize,
    pub reductions_performed: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StateRecord
{
    state: StateCode,
    enabled: StateCode,
    num_fanouts: usize,
    index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SiMinError
{
    InputCountMismatch
    {
        cube: usize,
        expected: usize,
        actual: usize,
    },
    OutputCountMismatch
    {
        cube: usize,
        expected: usize,
        actual: usize,
    },
    MissingSignal
    {
        name: String,
    },
    UnknownEnabledSignal
    {
        state: StateCode,
        bit: StateCode,
    },
    OnSetIntersectsOffSet,
    TooManyInputsForExhaustiveCover
    {
        inputs: usize,
    },
}

impl fmt::Display for SiMinError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::InputCountMismatch
            {
                cube,
                expected,
                actual,
            } => write!(f, "cube {cube} has {actual} inputs, expected {expected}"),
            Self::OutputCountMismatch
            {
                cube,
                expected,
                actual,
            } => write!(f, "cube {cube} has {actual} outputs, expected {expected}"),
            Self::MissingSignal
            {
                name,
            } => write!(f, "missing ASTG signal named {name}"),
            Self::UnknownEnabledSignal
            {
                state,
                bit,
            } => write!(f, "state {state} enables unknown signal bit {bit}"),
            Self::OnSetIntersectsOffSet => write!(f, "PLA on-set and off-set are not disjoint"),
            Self::TooManyInputsForExhaustiveCover
            {
                inputs,
            } => write!(
                f,
                "{inputs} inputs are too many for exhaustive cube coverage validation"
            ),
        }
    }
}

impl Error for SiMinError
{
}

pub fn astg_minimize(
    pla: &mut Pla,
    stg: &AstgGraph,
    options: SiMinOptions,
) -> Result<SiMinReport, SiMinError>
{
    validate_pla(pla)?;
    validate_astg_labels(pla, stg)?;
    validate_on_off_disjoint(pla)?;

    let state_minterms = link_state_minterms(pla, stg)?;
    let consensus_cubes_added = if options.add_redundant_consensus
    {
        add_consensus_cubes(pla, stg, &state_minterms)?
    }
    else
    {
        0
    };

    let reductions_performed = if options.reduce_simultaneous_signal_hazards
    {
        reduce_simultaneous_signal_hazards(pla, stg)?
    }
    else
    {
        0
    };

    Ok(SiMinReport
    {
        linked_states: state_minterms.len(),
        consensus_cubes_added,
        reductions_performed,
    })
}

pub fn link_state_minterms(
    pla: &Pla,
    stg: &AstgGraph,
) -> Result<HashMap<StateCode, Cube>, SiMinError>
{
    let mut table = HashMap::new();

    for state in &stg.states
    {
        if let Some(cube) = find_minterm(pla, stg, state.code)?
        {
            table.insert(state.code, cube);
        }
    }

    Ok(table)
}

pub fn find_minterm(
    pla: &Pla,
    stg: &AstgGraph,
    state: StateCode,
) -> Result<Option<Cube>, SiMinError>
{
    let mut result: Option<Cube> = None;

    for cube in &pla.on_set
    {
        let mut matches = true;

        for (input_index, input_name) in pla.input_names.iter().enumerate().rev()
        {
            let signal = stg
                .signal_by_name(input_name)
                .ok_or_else(|| SiMinError::MissingSignal
                {
                    name: input_name.clone(),
                })?;
            let value = (signal.state_bit & state) != 0;

            if cube.inputs[input_index] == Literal::DontCare
                || !cube.inputs[input_index].matches(value)
            {
                matches = false;
                break;
            }
        }

        if matches
        {
            match &mut result
            {
                Some(existing) =>
                {
                    for (index, output) in cube.outputs.iter().enumerate()
                    {
                        existing.outputs[index] |= *output;
                    }
                }
                None => result = Some(cube.clone()),
            }
        }
    }

    Ok(result)
}

pub fn add_consensus_cubes(
    pla: &mut Pla,
    stg: &AstgGraph,
    state_minterms: &HashMap<StateCode, Cube>,
) -> Result<usize, SiMinError>
{
    let mut added = 0;

    for state in &stg.states
    {
        let Some(cube) = state_minterms.get(&state.code) else
        {
            continue;
        };

        for signal in enabled_signals(stg, state)?
        {
            let next_state = state.code ^ signal.state_bit;
            let Some(next_cube) = state_minterms.get(&next_state) else
            {
                continue;
            };

            if input_distance(cube, next_cube) == 1
            {
                let consensus = consensus_cube(cube, next_cube);

                if consensus.output_count() > 0 && !pla.on_set.contains(&consensus)
                {
                    pla.on_set.push(consensus);
                    added += 1;
                }
            }
        }
    }

    Ok(added)
}

pub fn reduce_simultaneous_signal_hazards(
    pla: &mut Pla,
    stg: &AstgGraph,
) -> Result<usize, SiMinError>
{
    let mut states = sort_states(stg);
    let mut reductions = 0;
    let mut reducing = true;

    while reducing
    {
        reducing = false;

        for state_index in 0..states.len()
        {
            states[state_index].index = state_index;
            let snapshot = pla.on_set.clone();

            for cube_index in 0..snapshot.len()
            {
                let Some(cube) = pla.on_set.get(cube_index).cloned() else
                {
                    continue;
                };

                if possible_glitch(&cube, &states[state_index], stg, &pla.on_set, pla)?
                    && perform_reduction(cube_index, &states[state_index], &states, stg, pla)?
                {
                    reducing = true;
                    reductions += 1;
                }
            }
        }
    }

    Ok(reductions)
}

fn possible_glitch(
    cube: &Cube,
    state_info: &StateRecord,
    stg: &AstgGraph,
    cover: &[Cube],
    pla: &Pla,
) -> Result<bool, SiMinError>
{
    let mut rising = 0;
    let mut falling = 0;

    for (input_index, input_name) in pla.input_names.iter().enumerate().rev()
    {
        let signal = stg
            .signal_by_name(input_name)
            .ok_or_else(|| SiMinError::MissingSignal
            {
                name: input_name.clone(),
            })?;

        let init_val = (signal.state_bit & state_info.state) != 0;
        let changing = (signal.state_bit & state_info.enabled) != 0;

        match cube.inputs[input_index]
        {
            Literal::DontCare =>
            {
            }
            Literal::Zero =>
            {
                if !changing && init_val
                {
                    return Ok(false);
                }

                if changing
                {
                    if init_val
                    {
                        rising += 1;
                    }
                    else
                    {
                        falling += 1;
                    }
                }
            }
            Literal::One =>
            {
                if !changing && !init_val
                {
                    return Ok(false);
                }

                if changing
                {
                    if init_val
                    {
                        falling += 1;
                    }
                    else
                    {
                        rising += 1;
                    }
                }
            }
        }
    }

    if rising > 0 && falling > 0
    {
        return Ok(!constant_one(cube, state_info, stg, cover, pla)?);
    }

    Ok(false)
}

fn validate_pla(pla: &Pla) -> Result<(), SiMinError>
{
    for (index, cube) in pla
        .on_set
        .iter()
        .chain(pla.off_set.iter())
        .chain(pla.dc_set.iter())
        .enumerate()
    {
        if cube.inputs.len() != pla.input_names.len()
        {
            return Err(SiMinError::InputCountMismatch
            {
                cube: index,
                expected: pla.input_names.len(),
                actual: cube.inputs.len(),
            });
        }

        if cube.outputs.len() != pla.output_names.len()
        {
            return Err(SiMinError::OutputCountMismatch
            {
                cube: index,
                expected: pla.output_names.len(),
                actual: cube.outputs.len(),
            });
        }
    }

    Ok(())
}

fn validate_astg_labels(pla: &Pla, stg: &AstgGraph) -> Result<(), SiMinError>
{
    for input_name in &pla.input_names
    {
        stg.signal_by_name(input_name)
            .ok_or_else(|| SiMinError::MissingSignal
            {
                name: input_name.clone(),
            })?;
    }

    Ok(())
}

fn validate_on_off_disjoint(pla: &Pla) -> Result<(), SiMinError>
{
    for on_cube in &pla.on_set
    {
        for off_cube in &pla.off_set
        {
            if cubes_intersect(on_cube, off_cube)
                && on_cube
                    .outputs
                    .iter()
                    .zip(off_cube.outputs.iter())
                    .any(|(on_output, off_output)| *on_output && *off_output)
            {
                return Err(SiMinError::OnSetIntersectsOffSet);
            }
        }
    }

    Ok(())
}

fn enabled_signals<'a>(
    stg: &'a AstgGraph,
    state: &AstgState,
) -> Result<Vec<&'a Signal>, SiMinError>
{
    let mut signals = Vec::new();
    let mut enabled = state.enabled;

    while enabled != 0
    {
        let bit = enabled & enabled.wrapping_neg();
        let signal = stg
            .signal_by_bit(bit)
            .ok_or(SiMinError::UnknownEnabledSignal
            {
                state: state.code,
                bit,
            })?;
        signals.push(signal);
        enabled &= !bit;
    }

    Ok(signals)
}

fn input_distance(left: &Cube, right: &Cube) -> usize
{
    left.inputs
        .iter()
        .zip(right.inputs.iter())
        .filter(|(left, right)| left != right)
        .count()
}

fn consensus_cube(left: &Cube, right: &Cube) -> Cube
{
    Cube
    {
        inputs: left
            .inputs
            .iter()
            .zip(right.inputs.iter())
            .map(|(left, right)| if left == right { *left } else { Literal::DontCare })
            .collect(),
        outputs: left
            .outputs
            .iter()
            .zip(right.outputs.iter())
            .map(|(left, right)| *left && *right)
            .collect(),
    }
}

fn sort_states(stg: &AstgGraph) -> Vec<StateRecord>
{
    let mut states: Vec<StateRecord> = stg
        .states
        .iter()
        .filter_map(|state|
        {
            let num_fanouts = stg
                .signals
                .iter()
                .filter(|signal| (signal.state_bit & state.enabled) != 0)
                .count();

            if num_fanouts > 1 && (stg.is_marked_graph || !state.has_free_choice_marking)
            {
                Some(StateRecord
                {
                    state: state.code,
                    enabled: state.enabled,
                    num_fanouts,
                    index: 0,
                })
            }
            else
            {
                None
            }
        })
        .collect();

    states.sort_by(|left, right| right.num_fanouts.cmp(&left.num_fanouts));

    for (index, state) in states.iter_mut().enumerate()
    {
        state.index = index;
    }

    states
}

fn constant_one(
    cube: &Cube,
    state_info: &StateRecord,
    stg: &AstgGraph,
    cover: &[Cube],
    pla: &Pla,
) -> Result<bool, SiMinError>
{
    let initial = endpoint_cube(state_info.state, pla, stg)?;
    let final_state = state_info.state ^ state_info.enabled;
    let final_cube = endpoint_cube(final_state, pla, stg)?;
    let mut const_one = vec![0usize; pla.output_names.len()];

    for cover_cube in cover
    {
        let initial_hit = cubes_intersect(cover_cube, &initial);
        let final_hit = cubes_intersect(cover_cube, &final_cube);

        for (index, output) in cube.outputs.iter().enumerate().rev()
        {
            if *output && cover_cube.outputs[index] && initial_hit && final_hit
            {
                const_one[index] += 1;
            }
        }
    }

    Ok(cube
        .outputs
        .iter()
        .enumerate()
        .filter(|(_, output)| **output)
        .all(|(index, _)| const_one[index] > 0))
}

fn perform_reduction(
    cube_index: usize,
    state_info: &StateRecord,
    state_array: &[StateRecord],
    stg: &AstgGraph,
    pla: &mut Pla,
) -> Result<bool, SiMinError>
{
    let Some(original_cube) = pla.on_set.get(cube_index).cloned() else
    {
        return Ok(false);
    };

    let cover_without_cube: Vec<Cube> = pla
        .on_set
        .iter()
        .enumerate()
        .filter_map(|(index, cube)| if index == cube_index { None } else { Some(cube.clone()) })
        .chain(pla.dc_set.iter().cloned())
        .collect();
    let mut best_literal = None;
    let mut best_cost = usize::MAX;

    for (input_index, input_name) in pla.input_names.iter().enumerate().rev()
    {
        let signal = stg
            .signal_by_name(input_name)
            .ok_or_else(|| SiMinError::MissingSignal
            {
                name: input_name.clone(),
            })?;

        if (signal.state_bit & state_info.enabled) != 0
            || original_cube.inputs[input_index] != Literal::DontCare
        {
            continue;
        }

        let mut reduced_cube = original_cube.clone();

        if (signal.state_bit & state_info.state) != 0
        {
            if signal.kind == SignalKind::Input
            {
                continue;
            }

            reduced_cube.inputs[input_index] = Literal::Zero;
        }
        else
        {
            reduced_cube.inputs[input_index] = Literal::One;
        }

        let mut opposite_cube = reduced_cube.clone();
        opposite_cube.inputs[input_index] = match reduced_cube.inputs[input_index]
        {
            Literal::Zero => Literal::One,
            Literal::One => Literal::Zero,
            Literal::DontCare => Literal::DontCare,
        };

        if !cube_is_covered(&cover_without_cube, &opposite_cube)?
        {
            continue;
        }

        if logic_hazard(&pla.on_set, pla, signal, stg)?
        {
            continue;
        }

        let mut cost = 0;
        for (index, state) in state_array.iter().enumerate().rev()
        {
            if index == state_info.index
            {
                continue;
            }

            if possible_glitch(&reduced_cube, state, stg, &pla.on_set, pla)?
            {
                if index < state_info.index
                {
                    cost = usize::MAX;
                    break;
                }

                cost += 1;
            }
        }

        if cost < best_cost
        {
            best_literal = Some(input_index);
            best_cost = cost;
        }
    }

    let Some(best_literal) = best_literal else
    {
        return Ok(false);
    };

    let signal = stg
        .signal_by_name(&pla.input_names[best_literal])
        .ok_or_else(|| SiMinError::MissingSignal
        {
            name: pla.input_names[best_literal].clone(),
        })?;

    pla.on_set[cube_index].inputs[best_literal] =
        if (signal.state_bit & state_info.state) != 0
        {
            Literal::Zero
        }
        else
        {
            Literal::One
        };

    Ok(true)
}

fn logic_hazard(
    cover: &[Cube],
    pla: &Pla,
    signal: &Signal,
    stg: &AstgGraph,
) -> Result<bool, SiMinError>
{
    for state in &stg.states
    {
        if (signal.state_bit & state.enabled) == 0
        {
            continue;
        }

        let c1 = endpoint_cube(state.code, pla, stg)?;
        let c2 = endpoint_cube(state.code ^ signal.state_bit, pla, stg)?;
        let mut rising = vec![0usize; pla.output_names.len()];
        let mut falling = vec![0usize; pla.output_names.len()];
        let mut const_one = vec![0usize; pla.output_names.len()];

        for cube in cover
        {
            let p1 = cubes_intersect(cube, &c1);
            let p2 = cubes_intersect(cube, &c2);

            for index in (0..pla.output_names.len()).rev()
            {
                if cube.outputs[index]
                {
                    if !p1 && p2
                    {
                        rising[index] += 1;
                    }
                    else if p1 && !p2
                    {
                        falling[index] += 1;
                    }
                    else if p1 && p2
                    {
                        const_one[index] += 1;
                    }
                }
            }
        }

        if rising
            .iter()
            .zip(falling.iter())
            .zip(const_one.iter())
            .any(|((rising, falling), const_one)| *rising > 0 && *falling > 0 && *const_one == 0)
        {
            return Ok(true);
        }
    }

    Ok(false)
}

fn endpoint_cube(
    state: StateCode,
    pla: &Pla,
    stg: &AstgGraph,
) -> Result<Cube, SiMinError>
{
    let mut inputs = Vec::with_capacity(pla.input_names.len());

    for input_name in &pla.input_names
    {
        let signal = stg
            .signal_by_name(input_name)
            .ok_or_else(|| SiMinError::MissingSignal
            {
                name: input_name.clone(),
            })?;
        inputs.push(if (signal.state_bit & state) != 0
        {
            Literal::One
        }
        else
        {
            Literal::Zero
        });
    }

    Ok(Cube
    {
        inputs,
        outputs: vec![true; pla.output_names.len()],
    })
}

fn cubes_intersect(left: &Cube, right: &Cube) -> bool
{
    left.inputs
        .iter()
        .zip(right.inputs.iter())
        .all(|(left, right)| match (left, right)
        {
            (Literal::Zero, Literal::One) | (Literal::One, Literal::Zero) => false,
            _ => true,
        })
}

fn cube_is_covered(cover: &[Cube], candidate: &Cube) -> Result<bool, SiMinError>
{
    let input_count = candidate.inputs.len();

    if input_count >= usize::BITS as usize
    {
        return Err(SiMinError::TooManyInputsForExhaustiveCover
        {
            inputs: input_count,
        });
    }

    let max_assignment = 1usize << input_count;

    for output_index in 0..candidate.outputs.len()
    {
        if !candidate.outputs[output_index]
        {
            continue;
        }

        for assignment in 0..max_assignment
        {
            if !candidate.covers_assignment(assignment, input_count)
            {
                continue;
            }

            let covered = cover.iter().any(|cube|
            {
                cube.outputs[output_index] && cube.covers_assignment(assignment, input_count)
            });

            if !covered
            {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

fn bit_from_assignment(assignment: usize, index: usize) -> bool
{
    (assignment & (1usize << index)) != 0
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn names(values: &[&str]) -> Vec<String>
    {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn graph() -> AstgGraph
    {
        AstgGraph::new(
            vec![
                Signal::new("a", 0b001, SignalKind::Output),
                Signal::new("b", 0b010, SignalKind::Output),
                Signal::new("c", 0b100, SignalKind::Input),
            ],
            vec![
                AstgState::new(0b000, 0b001),
                AstgState::new(0b001, 0b010),
                AstgState::new(0b011, 0b011),
            ],
        )
    }

    #[test]
    fn find_minterm_merges_outputs_for_same_state()
    {
        let pla = Pla::new(
            names(&["a", "b"]),
            names(&["x", "y"]),
            vec![
                Cube::minterm(&[true, false], vec![true, false]),
                Cube::minterm(&[true, false], vec![false, true]),
            ],
        );

        let cube = find_minterm(&pla, &graph(), 0b001).unwrap().unwrap();

        assert_eq!(cube.outputs, vec![true, true]);
    }

    #[test]
    fn dont_care_cube_is_not_treated_as_state_minterm()
    {
        let pla = Pla::new(
            names(&["a", "b"]),
            names(&["x"]),
            vec![Cube::new(
                vec![Literal::One, Literal::DontCare],
                vec![true],
            )],
        );

        assert_eq!(find_minterm(&pla, &graph(), 0b001).unwrap(), None);
    }

    #[test]
    fn consensus_cube_is_added_for_enabled_adjacent_state_pair()
    {
        let mut pla = Pla::new(
            names(&["a", "b"]),
            names(&["x"]),
            vec![
                Cube::minterm(&[false, false], vec![true]),
                Cube::minterm(&[true, false], vec![true]),
            ],
        );
        let stg = graph();
        let state_minterms = link_state_minterms(&pla, &stg).unwrap();

        let added = add_consensus_cubes(&mut pla, &stg, &state_minterms).unwrap();

        assert_eq!(added, 1);
        assert!(pla.on_set.contains(&Cube::new(
            vec![Literal::DontCare, Literal::Zero],
            vec![true],
        )));
    }

    #[test]
    fn consensus_cube_keeps_only_common_outputs()
    {
        let left = Cube::minterm(&[false, false], vec![true, false, true]);
        let right = Cube::minterm(&[true, false], vec![true, true, false]);

        assert_eq!(
            consensus_cube(&left, &right),
            Cube::new(
                vec![Literal::DontCare, Literal::Zero],
                vec![true, false, false],
            )
        );
    }

    #[test]
    fn sort_states_orders_by_decreasing_fanout_and_skips_free_choice_markings()
    {
        let stg = AstgGraph::new(
            vec![
                Signal::new("a", 0b001, SignalKind::Output),
                Signal::new("b", 0b010, SignalKind::Output),
                Signal::new("c", 0b100, SignalKind::Output),
            ],
            vec![
                AstgState::new(0, 0b001),
                AstgState::new(1, 0b111),
                AstgState::new(2, 0b011).with_free_choice_marking(),
            ],
        )
        .with_free_choice_places();

        let states = sort_states(&stg);

        assert_eq!(states.len(), 1);
        assert_eq!(states[0].state, 1);
        assert_eq!(states[0].num_fanouts, 3);
    }

    #[test]
    fn possible_glitch_detects_mixed_rising_and_falling_enabled_signals()
    {
        let pla = Pla::new(
            names(&["a", "b"]),
            names(&["x"]),
            vec![Cube::new(
                vec![Literal::One, Literal::Zero],
                vec![true],
            )],
        );
        let stg = graph();
        let state = StateRecord
        {
            state: 0b000,
            enabled: 0b011,
            num_fanouts: 2,
            index: 0,
        };

        assert!(possible_glitch(&pla.on_set[0], &state, &stg, &pla.on_set, &pla).unwrap());
    }

    #[test]
    fn possible_glitch_is_masked_by_constant_one_cube()
    {
        let pla = Pla::new(
            names(&["a", "b"]),
            names(&["x"]),
            vec![
                Cube::new(vec![Literal::One, Literal::Zero], vec![true]),
                Cube::new(vec![Literal::DontCare, Literal::DontCare], vec![true]),
            ],
        );
        let stg = graph();
        let state = StateRecord
        {
            state: 0b001,
            enabled: 0b011,
            num_fanouts: 2,
            index: 0,
        };

        assert!(!possible_glitch(&pla.on_set[0], &state, &stg, &pla.on_set, &pla).unwrap());
    }

    #[test]
    fn astg_minimize_reports_consensus_work()
    {
        let mut pla = Pla::new(
            names(&["a", "b"]),
            names(&["x"]),
            vec![
                Cube::minterm(&[false, false], vec![true]),
                Cube::minterm(&[true, false], vec![true]),
            ],
        );

        let report = astg_minimize(
            &mut pla,
            &graph(),
            SiMinOptions
            {
                add_redundant_consensus: true,
                reduce_simultaneous_signal_hazards: false,
            },
        )
        .unwrap();

        assert_eq!(report.linked_states, 2);
        assert_eq!(report.consensus_cubes_added, 1);
        assert_eq!(report.reductions_performed, 0);
    }

    #[test]
    fn astg_minimize_rejects_intersecting_on_and_off_sets()
    {
        let mut pla = Pla::new(
            names(&["a"]),
            names(&["x"]),
            vec![Cube::minterm(&[true], vec![true])],
        );
        pla.off_set.push(Cube::minterm(&[true], vec![true]));

        let error = astg_minimize(&mut pla, &graph(), SiMinOptions::default()).unwrap_err();

        assert_eq!(error, SiMinError::OnSetIntersectsOffSet);
    }
}
