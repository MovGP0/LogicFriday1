//! Backward hazard detection for ASTG-derived two-level next-state functions.
//!
//! The code models the SIS `bwd_hazard.c` algorithm with owned Rust data:
//! cubes are binary-input product terms, covers are vectors of cubes, and the
//! state graph is supplied as plain native structs.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

pub type StateCode = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind
{
    Input,
    Output,
    Internal,
    Dummy,
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
        Self {
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
}

impl AstgState
{
    pub const fn new(code: StateCode, enabled: StateCode) -> Self
    {
        Self { code, enabled }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgGraph
{
    pub signals: Vec<Signal>,
    pub states: Vec<AstgState>,
}

impl AstgGraph
{
    pub fn new(signals: Vec<Signal>, states: Vec<AstgState>) -> Self
    {
        Self { signals, states }
    }

    fn signal_by_name(&self, name: &str) -> Option<&Signal>
    {
        self.signals.iter().find(|signal| signal.name == name)
    }

    fn signal_by_bit(&self, state_bit: StateCode) -> Option<&Signal>
    {
        self.signals
            .iter()
            .find(|signal| signal.kind != SignalKind::Dummy && signal.state_bit == state_bit)
    }

    fn state(&self, code: StateCode) -> Option<&AstgState>
    {
        self.states.iter().find(|state| state.code == code)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CubeValue
{
    Zero,
    One,
    DontCare,
}

impl CubeValue
{
    fn from_bit(value: bool) -> Self
    {
        if value {
            Self::One
        } else {
            Self::Zero
        }
    }

    fn matches(self, value: bool) -> bool
    {
        match self {
            Self::Zero => !value,
            Self::One => value,
            Self::DontCare => true,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cube
{
    values: Vec<CubeValue>,
}

impl Cube
{
    pub fn new(values: impl Into<Vec<CubeValue>>) -> Self
    {
        Self {
            values: values.into(),
        }
    }

    pub fn dont_care(input_count: usize) -> Self
    {
        Self {
            values: vec![CubeValue::DontCare; input_count],
        }
    }

    pub fn values(&self) -> &[CubeValue]
    {
        &self.values
    }

    pub fn len(&self) -> usize
    {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.values.is_empty()
    }

    pub fn covers_assignment(&self, assignment: StateCode, bits: &[StateCode]) -> bool
    {
        self.values
            .iter()
            .zip(bits.iter())
            .all(|(value, bit)| value.matches((assignment & bit) != 0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover
{
    input_count: usize,
    cubes: Vec<Cube>,
}

impl Cover
{
    pub fn new(input_count: usize, cubes: impl Into<Vec<Cube>>) -> HazardResult<Self>
    {
        let cubes = cubes.into();
        for (index, cube) in cubes.iter().enumerate() {
            if cube.len() != input_count {
                return Err(HazardError::CubeWidthMismatch {
                    cube: index,
                    expected: input_count,
                    actual: cube.len(),
                });
            }
        }

        Ok(Self { input_count, cubes })
    }

    pub fn empty(input_count: usize) -> Self
    {
        Self {
            input_count,
            cubes: Vec::new(),
        }
    }

    pub fn input_count(&self) -> usize
    {
        self.input_count
    }

    pub fn cubes(&self) -> &[Cube]
    {
        &self.cubes
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }

    pub fn push(&mut self, cube: Cube) -> HazardResult<()>
    {
        if cube.len() != self.input_count {
            return Err(HazardError::CubeWidthMismatch {
                cube: self.cubes.len(),
                expected: self.input_count,
                actual: cube.len(),
            });
        }

        self.cubes.push(cube);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionDirection
{
    Rising,
    Falling,
}

impl TransitionDirection
{
    fn from_top_side(value: CubeValue) -> Self
    {
        if value == CubeValue::Zero {
            Self::Rising
        } else {
            Self::Falling
        }
    }

    fn from_bottom_side(value: CubeValue) -> Self
    {
        if value == CubeValue::One {
            Self::Rising
        } else {
            Self::Falling
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hazard
{
    pub first_signal: String,
    pub second_signal: String,
    pub first_direction: TransitionDirection,
    pub second_direction: TransitionDirection,
    pub onset: bool,
    pub on_cube1: Cube,
    pub off_cube1: Cube,
    pub on_cube2: Cube,
    pub off_cube2: Cube,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HazardSearchMode
{
    Tcad,
    Dac,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HazardError
{
    CubeWidthMismatch
    {
        cube: usize,
        expected: usize,
        actual: usize,
    },
    CoverWidthMismatch
    {
        expected: usize, actual: usize
    },
    MissingSignal
    {
        name: String
    },
    UnknownEnabledSignal
    {
        state: StateCode, bit: StateCode
    },
    MissingState
    {
        state: StateCode
    },
    TooManyInputsForExactComplement
    {
        inputs: usize
    },
}

impl fmt::Display for HazardError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::CubeWidthMismatch {
                cube,
                expected,
                actual,
            } => write!(
                formatter,
                "cube {cube} has width {actual}, expected {expected}"
            ),
            Self::CoverWidthMismatch { expected, actual } => write!(
                formatter,
                "cover width {actual} does not match expected width {expected}"
            ),
            Self::MissingSignal { name } => write!(formatter, "missing ASTG signal named {name}"),
            Self::UnknownEnabledSignal { state, bit } => {
                write!(formatter, "state {state} enables unknown signal bit {bit}")
            }
            Self::MissingState { state } => write!(formatter, "missing ASTG state {state}"),
            Self::TooManyInputsForExactComplement { inputs } => write!(
                formatter,
                "{inputs} inputs are too many for exact complement enumeration"
            ),
        }
    }
}

impl Error for HazardError {}

pub type HazardResult<T> = Result<T, HazardError>;

pub fn find_backward_hazards(
    stg: &AstgGraph,
    output_signal: &str,
    fanin_names: &[String],
    on_set: &Cover,
    off_set: &Cover,
    mode: HazardSearchMode,
) -> HazardResult<Vec<Hazard>>
{
    if on_set.input_count() != fanin_names.len() {
        return Err(HazardError::CoverWidthMismatch {
            expected: fanin_names.len(),
            actual: on_set.input_count(),
        });
    }

    if off_set.input_count() != fanin_names.len() {
        return Err(HazardError::CoverWidthMismatch {
            expected: fanin_names.len(),
            actual: off_set.input_count(),
        });
    }

    let signal = stg
        .signal_by_name(output_signal)
        .ok_or_else(|| HazardError::MissingSignal {
            name: output_signal.to_string(),
        })?;
    let fanin_bits = fanin_state_bits(stg, fanin_names)?;
    let mut hazards = Vec::new();
    let mut not_top = HashSet::new();

    for state in &stg.states {
        if (state.enabled & signal.state_bit) == 0 {
            continue;
        }

        let mut reached = HashSet::from([state.code]);
        let mut bot = bit_to_cube(state.code, &fanin_bits);
        let new_state = state.code ^ signal.state_bit;
        let current_onset = (new_state & signal.state_bit) != 0;
        let active_on_set = if current_onset { on_set } else { off_set };

        find_not_top(
            active_on_set,
            stg,
            state.code,
            &fanin_bits,
            &mut reached,
            &mut not_top,
            &mut bot,
        )?;
    }

    for state in &stg.states {
        if not_top.contains(&state.code) {
            continue;
        }

        let transition_cube = bit_to_cube(state.code, &fanin_bits);
        let top = transition_cube.clone();
        let mut bot = transition_cube.clone();
        let mut reached = HashSet::from([state.code]);
        let new_state = if (state.enabled & signal.state_bit) != 0 {
            state.code ^ signal.state_bit
        } else {
            state.code
        };
        let current_onset = (new_state & signal.state_bit) != 0;
        let active_on_set = if current_onset { on_set } else { off_set };
        let active_off_set = complement_cover(active_on_set, &fanin_bits)?;
        let mut context = SearchContext {
            fanin_names,
            fanin_bits: &fanin_bits,
            stg,
            on_set: active_on_set,
            off_set: &active_off_set,
            hazards: &mut hazards,
            onset: current_onset,
            mode,
        };

        find_bot(
            &mut context,
            state.code,
            &mut reached,
            transition_cube,
            &top,
            &mut bot,
        )?;
    }

    Ok(hazards)
}

pub fn restrict_cover_to_cube(cover: &Cover, cube: &Cube) -> HazardResult<Cover>
{
    if cover.input_count() != cube.len() {
        return Err(HazardError::CoverWidthMismatch {
            expected: cover.input_count(),
            actual: cube.len(),
        });
    }

    let mut restricted = Cover::empty(cover.input_count());
    for row in cover.cubes() {
        if let Some(intersection) = cube_intersection(row, cube) {
            restricted.push(intersection)?;
        }
    }

    Ok(contain_cover(restricted))
}

struct SearchContext<'a>
{
    fanin_names: &'a [String],
    fanin_bits: &'a [StateCode],
    stg: &'a AstgGraph,
    on_set: &'a Cover,
    off_set: &'a Cover,
    hazards: &'a mut Vec<Hazard>,
    onset: bool,
    mode: HazardSearchMode,
}

fn find_bot(
    context: &mut SearchContext<'_>,
    state_code: StateCode,
    reached: &mut HashSet<StateCode>,
    mut transition_cube: Cube,
    top: &Cube,
    bot: &mut Cube,
) -> HazardResult<()>
{
    let state = context
        .stg
        .state(state_code)
        .ok_or(HazardError::MissingState { state: state_code })?;
    let mut recursions = 0;

    for signal in enabled_signals(context.stg, state)? {
        let Some(fanin_index) = context
            .fanin_bits
            .iter()
            .position(|bit| *bit == signal.state_bit)
        else {
            continue;
        };
        let new_state = state_code ^ signal.state_bit;
        let saved = transition_cube.values[fanin_index];

        transition_cube.values[fanin_index] = CubeValue::DontCare;
        *bot = bit_to_cube(new_state, context.fanin_bits);

        if !func_value(context.on_set, bot) {
            transition_cube.values[fanin_index] = saved;
            continue;
        }

        recursions += 1;
        if reached.insert(new_state) {
            find_bot(
                context,
                new_state,
                reached,
                transition_cube.clone(),
                top,
                bot,
            )?;
        }

        transition_cube.values[fanin_index] = saved;
    }

    if recursions == 0 {
        *bot = bit_to_cube(state_code, context.fanin_bits);
        match context.mode {
            HazardSearchMode::Tcad => store_hazards(context, &transition_cube, top, bot)?,
            HazardSearchMode::Dac => store_hazards_old(context, &transition_cube, top, bot)?,
        }
    }

    Ok(())
}

fn find_not_top(
    active_on_set: &Cover,
    stg: &AstgGraph,
    state_code: StateCode,
    fanin_bits: &[StateCode],
    reached: &mut HashSet<StateCode>,
    not_top: &mut HashSet<StateCode>,
    bot: &mut Cube,
) -> HazardResult<()>
{
    let state = stg
        .state(state_code)
        .ok_or(HazardError::MissingState { state: state_code })?;

    for signal in enabled_signals(stg, state)? {
        let new_state = state_code ^ signal.state_bit;
        if !reached.insert(new_state) {
            continue;
        }

        *bot = bit_to_cube(new_state, fanin_bits);
        if !func_value(active_on_set, bot) {
            continue;
        }

        not_top.insert(new_state);
        find_not_top(
            active_on_set,
            stg,
            new_state,
            fanin_bits,
            reached,
            not_top,
            bot,
        )?;
    }

    Ok(())
}

fn store_hazards(
    context: &mut SearchContext<'_>,
    transition_cube: &Cube,
    top: &Cube,
    bot: &Cube,
) -> HazardResult<()>
{
    let off_restricted = restrict_cover_to_cube(context.off_set, transition_cube)?;
    if off_restricted.is_empty() {
        return Ok(());
    }

    let on_restricted = restrict_cover_to_cube(context.on_set, transition_cube)?;
    if on_restricted.is_empty() {
        return Ok(());
    }

    for off_cube1 in off_restricted.cubes() {
        let top_to_off1 = cube_distance(top, off_cube1);
        let bot_to_off1 = cube_distance(bot, off_cube1);

        for off_cube2 in off_restricted.cubes() {
            let top_to_off2 = cube_distance(top, off_cube2);
            let bot_to_off2 = cube_distance(bot, off_cube2);
            if top_to_off2 < top_to_off1 || bot_to_off1 < bot_to_off2 {
                continue;
            }

            for (on_index1, on_cube1) in on_restricted.cubes().iter().enumerate() {
                if cube_distance_limit_one(on_cube1, off_cube1) != 1 {
                    continue;
                }

                let top_to_on1 = cube_distance(top, on_cube1);
                let bot_to_on1 = cube_distance(bot, on_cube1);
                let Some((var1, dir1)) = changed_variable(on_cube1, off_cube1, true) else {
                    continue;
                };

                for (on_index2, on_cube2) in on_restricted.cubes().iter().enumerate() {
                    if on_index1 == on_index2 || cube_distance_limit_one(on_cube2, off_cube2) != 1 {
                        continue;
                    }

                    let top_to_on2 = cube_distance(top, on_cube2);
                    let bot_to_on2 = cube_distance(bot, on_cube2);
                    if top_to_on2 < top_to_on1 || bot_to_on1 < bot_to_on2 {
                        continue;
                    }

                    let Some((var2, dir2)) = changed_variable(on_cube2, off_cube2, false) else {
                        continue;
                    };

                    push_unique_hazard(
                        context,
                        Hazard {
                            first_signal: context.fanin_names[var1].clone(),
                            second_signal: context.fanin_names[var2].clone(),
                            first_direction: dir1,
                            second_direction: dir2,
                            onset: context.onset,
                            on_cube1: on_cube1.clone(),
                            off_cube1: off_cube1.clone(),
                            on_cube2: on_cube2.clone(),
                            off_cube2: off_cube2.clone(),
                        },
                    );
                }
            }
        }
    }

    Ok(())
}

fn store_hazards_old(
    context: &mut SearchContext<'_>,
    transition_cube: &Cube,
    top: &Cube,
    bot: &Cube,
) -> HazardResult<()>
{
    let off_restricted = restrict_cover_to_cube(context.off_set, transition_cube)?;
    let on_restricted = restrict_cover_to_cube(context.on_set, transition_cube)?;

    for off_cube in off_restricted.cubes() {
        let top_distance = cube_distance(off_cube, top);
        let bot_distance = cube_distance(off_cube, bot);
        let top_side = on_restricted
            .cubes()
            .iter()
            .enumerate()
            .filter(|(_, cube)| cube_distance(cube, top) + 1 == top_distance)
            .collect::<Vec<_>>();
        let bottom_side = on_restricted
            .cubes()
            .iter()
            .enumerate()
            .filter(|(_, cube)| cube_distance(cube, bot) + 1 == bot_distance)
            .collect::<Vec<_>>();

        for (top_index, on_cube1) in &top_side {
            if cube_distance_limit_one(on_cube1, off_cube) != 1 {
                continue;
            }

            let Some((var1, dir1)) = changed_variable(on_cube1, off_cube, true) else {
                continue;
            };

            for (bottom_index, on_cube2) in &bottom_side {
                if top_index == bottom_index || cube_distance_limit_one(on_cube2, off_cube) != 1 {
                    continue;
                }

                let Some((var2, dir2)) = changed_variable(on_cube2, off_cube, false) else {
                    continue;
                };

                push_unique_hazard(
                    context,
                    Hazard {
                        first_signal: context.fanin_names[var1].clone(),
                        second_signal: context.fanin_names[var2].clone(),
                        first_direction: dir1,
                        second_direction: dir2,
                        onset: context.onset,
                        on_cube1: (*on_cube1).clone(),
                        off_cube1: off_cube.clone(),
                        on_cube2: (*on_cube2).clone(),
                        off_cube2: off_cube.clone(),
                    },
                );
            }
        }
    }

    Ok(())
}

fn push_unique_hazard(context: &mut SearchContext<'_>, hazard: Hazard)
{
    if !context.hazards.contains(&hazard) {
        context.hazards.push(hazard);
    }
}

fn changed_variable(
    on_cube: &Cube,
    off_cube: &Cube,
    top_side: bool,
) -> Option<(usize, TransitionDirection)>
{
    let mut changed = None;

    for (index, (on_value, off_value)) in on_cube
        .values()
        .iter()
        .copied()
        .zip(off_cube.values().iter().copied())
        .enumerate()
    {
        if literal_conflicts(on_value, off_value) {
            if changed.is_some() {
                return None;
            }

            let direction = if top_side {
                TransitionDirection::from_top_side(on_value)
            } else {
                TransitionDirection::from_bottom_side(on_value)
            };
            changed = Some((index, direction));
        }
    }

    changed
}

fn enabled_signals<'a>(stg: &'a AstgGraph, state: &AstgState) -> HazardResult<Vec<&'a Signal>>
{
    let mut signals = Vec::new();
    let mut enabled = state.enabled;

    while enabled != 0 {
        let bit = enabled & enabled.wrapping_neg();
        let signal = stg
            .signal_by_bit(bit)
            .ok_or(HazardError::UnknownEnabledSignal {
                state: state.code,
                bit,
            })?;
        signals.push(signal);
        enabled &= !bit;
    }

    Ok(signals)
}

fn fanin_state_bits(stg: &AstgGraph, fanin_names: &[String]) -> HazardResult<Vec<StateCode>>
{
    fanin_names
        .iter()
        .map(|name| {
            stg.signal_by_name(name)
                .map(|signal| signal.state_bit)
                .ok_or_else(|| HazardError::MissingSignal { name: name.clone() })
        })
        .collect()
}

fn bit_to_cube(state: StateCode, bits: &[StateCode]) -> Cube
{
    Cube::new(
        bits.iter()
            .map(|bit| CubeValue::from_bit((state & bit) != 0))
            .collect::<Vec<_>>(),
    )
}

fn func_value(cover: &Cover, vector: &Cube) -> bool
{
    cover
        .cubes()
        .iter()
        .any(|cube| cube_intersection(cube, vector).is_some())
}

fn complement_cover(cover: &Cover, bits: &[StateCode]) -> HazardResult<Cover>
{
    let input_count = cover.input_count();
    if input_count >= usize::BITS as usize {
        return Err(HazardError::TooManyInputsForExactComplement {
            inputs: input_count,
        });
    }

    let mut result = Cover::empty(input_count);
    let assignments = 1usize << input_count;

    for assignment in 0..assignments {
        let state = bits.iter().enumerate().fold(0, |state, (index, bit)| {
            if (assignment & (1usize << index)) != 0 {
                state | bit
            } else {
                state
            }
        });

        let cube = Cube::new(
            (0..input_count)
                .map(|index| CubeValue::from_bit((assignment & (1usize << index)) != 0))
                .collect::<Vec<_>>(),
        );

        if !cover
            .cubes()
            .iter()
            .any(|row| row.covers_assignment(state, bits))
        {
            result.push(cube)?;
        }
    }

    Ok(contain_cover(result))
}

fn cube_intersection(left: &Cube, right: &Cube) -> Option<Cube>
{
    if left.len() != right.len() {
        return None;
    }

    let mut values = Vec::with_capacity(left.len());
    for (left_value, right_value) in left.values().iter().zip(right.values()) {
        let value = match (*left_value, *right_value) {
            (CubeValue::Zero, CubeValue::One) | (CubeValue::One, CubeValue::Zero) => return None,
            (CubeValue::DontCare, value) | (value, CubeValue::DontCare) => value,
            (value, _) => value,
        };
        values.push(value);
    }

    Some(Cube::new(values))
}

fn contain_cover(cover: Cover) -> Cover
{
    let mut cubes = cover.cubes;
    cubes.sort_by(|left, right| {
        left.values()
            .iter()
            .filter(|value| **value != CubeValue::DontCare)
            .count()
            .cmp(
                &right
                    .values()
                    .iter()
                    .filter(|value| **value != CubeValue::DontCare)
                    .count(),
            )
            .then_with(|| left.cmp(right))
    });
    cubes.dedup();

    let original = cubes.clone();
    cubes.retain(|cube| {
        !original
            .iter()
            .any(|candidate| candidate != cube && cube_is_contained_by(cube, candidate))
    });

    Cover {
        input_count: cover.input_count,
        cubes,
    }
}

fn cube_is_contained_by(cube: &Cube, candidate: &Cube) -> bool
{
    cube.values()
        .iter()
        .zip(candidate.values())
        .all(|(cube_value, candidate_value)| {
            *candidate_value == CubeValue::DontCare || cube_value == candidate_value
        })
}

fn cube_distance(left: &Cube, right: &Cube) -> usize
{
    left.values()
        .iter()
        .zip(right.values())
        .filter(|(left_value, right_value)| literal_conflicts(**left_value, **right_value))
        .count()
}

fn cube_distance_limit_one(left: &Cube, right: &Cube) -> usize
{
    let mut distance = 0;
    for (left_value, right_value) in left.values().iter().zip(right.values()) {
        if literal_conflicts(*left_value, *right_value) {
            distance += 1;
            if distance > 1 {
                return distance;
            }
        }
    }

    distance
}

fn literal_conflicts(left: CubeValue, right: CubeValue) -> bool
{
    matches!(
        (left, right),
        (CubeValue::Zero, CubeValue::One) | (CubeValue::One, CubeValue::Zero)
    )
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn names(values: &[&str]) -> Vec<String>
    {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn cube(values: &[CubeValue]) -> Cube
    {
        Cube::new(values.to_vec())
    }

    fn cover(rows: &[&[CubeValue]]) -> Cover
    {
        Cover::new(3, rows.iter().map(|row| cube(row)).collect::<Vec<_>>()).unwrap()
    }

    fn graph() -> AstgGraph
    {
        AstgGraph::new(
            vec![
                Signal::new("a", 0b001, SignalKind::Output),
                Signal::new("b", 0b010, SignalKind::Output),
                Signal::new("y", 0b100, SignalKind::Output),
            ],
            vec![
                AstgState::new(0b000, 0b001),
                AstgState::new(0b001, 0b010),
                AstgState::new(0b011, 0b100),
                AstgState::new(0b111, 0),
            ],
        )
    }

    #[test]
    fn restrict_cover_intersects_rows_and_removes_contained_cubes()
    {
        let source = cover(&[
            &[CubeValue::One, CubeValue::DontCare, CubeValue::Zero],
            &[CubeValue::One, CubeValue::Zero, CubeValue::Zero],
            &[CubeValue::Zero, CubeValue::One, CubeValue::Zero],
        ]);
        let transition = cube(&[CubeValue::One, CubeValue::DontCare, CubeValue::DontCare]);

        let result = restrict_cover_to_cube(&source, &transition).unwrap();

        assert_eq!(
            result.cubes(),
            &[cube(&[
                CubeValue::One,
                CubeValue::DontCare,
                CubeValue::Zero,
            ])]
        );
    }

    #[test]
    fn changed_variable_reports_direction_for_top_and_bottom_sides()
    {
        let on_top = cube(&[CubeValue::Zero, CubeValue::One]);
        let off_top = cube(&[CubeValue::One, CubeValue::One]);
        let on_bottom = cube(&[CubeValue::Zero, CubeValue::One]);
        let off_bottom = cube(&[CubeValue::Zero, CubeValue::Zero]);

        assert_eq!(
            changed_variable(&on_top, &off_top, true),
            Some((0, TransitionDirection::Rising))
        );
        assert_eq!(
            changed_variable(&on_bottom, &off_bottom, false),
            Some((1, TransitionDirection::Rising))
        );
    }

    #[test]
    fn complement_cover_enumerates_uncovered_minterms()
    {
        let source = Cover::new(2, [cube(&[CubeValue::Zero, CubeValue::DontCare])]).unwrap();

        let result = complement_cover(&source, &[0b01, 0b10]).unwrap();

        assert_eq!(
            result.cubes(),
            &[
                cube(&[CubeValue::One, CubeValue::Zero]),
                cube(&[CubeValue::One, CubeValue::One]),
            ]
        );
    }

    #[test]
    fn tcad_store_hazards_records_unique_signal_pair()
    {
        let fanins = names(&["a", "b"]);
        let on_set = Cover::new(
            2,
            [
                cube(&[CubeValue::Zero, CubeValue::Zero]),
                cube(&[CubeValue::One, CubeValue::One]),
            ],
        )
        .unwrap();
        let off_set = Cover::new(
            2,
            [
                cube(&[CubeValue::One, CubeValue::Zero]),
                cube(&[CubeValue::Zero, CubeValue::One]),
            ],
        )
        .unwrap();
        let mut hazards = Vec::new();
        let stg = graph();
        let mut context = SearchContext {
            fanin_names: &fanins,
            fanin_bits: &[0b001, 0b010],
            stg: &stg,
            on_set: &on_set,
            off_set: &off_set,
            hazards: &mut hazards,
            onset: true,
            mode: HazardSearchMode::Tcad,
        };
        let top = cube(&[CubeValue::Zero, CubeValue::Zero]);
        let bot = cube(&[CubeValue::One, CubeValue::One]);
        let transition = Cube::dont_care(2);

        store_hazards(&mut context, &transition, &top, &bot).unwrap();
        store_hazards(&mut context, &transition, &top, &bot).unwrap();

        assert_eq!(context.hazards.len(), 4);
        assert!(context.hazards.iter().any(|hazard| {
            hazard.first_signal == "a"
                && hazard.second_signal == "b"
                && hazard.first_direction == TransitionDirection::Rising
                && hazard.second_direction == TransitionDirection::Rising
        }));
    }

    #[test]
    fn find_backward_hazards_traverses_from_top_to_bottom_state()
    {
        let stg = graph();
        let fanins = names(&["a", "b", "y"]);
        let off_set = cover(&[
            &[CubeValue::Zero, CubeValue::Zero, CubeValue::Zero],
            &[CubeValue::One, CubeValue::Zero, CubeValue::Zero],
            &[CubeValue::One, CubeValue::One, CubeValue::Zero],
        ]);
        let on_set = complement_cover(&off_set, &[0b001, 0b010, 0b100]).unwrap();

        let hazards = find_backward_hazards(
            &stg,
            "y",
            &fanins,
            &on_set,
            &off_set,
            HazardSearchMode::Tcad,
        )
        .unwrap();

        assert!(!hazards.is_empty());
    }

    #[test]
    fn unknown_enabled_signal_is_reported()
    {
        let stg = AstgGraph::new(
            vec![Signal::new("a", 0b001, SignalKind::Output)],
            vec![AstgState::new(0, 0b010)],
        );
        let cover = Cover::new(1, [cube(&[CubeValue::Zero])]).unwrap();
        let error = find_backward_hazards(
            &stg,
            "a",
            &names(&["a"]),
            &cover,
            &Cover::empty(1),
            HazardSearchMode::Tcad,
        )
        .unwrap_err();

        assert_eq!(
            error,
            HazardError::UnknownEnabledSignal {
                state: 0,
                bit: 0b010,
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("bwd_hazard.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }
}
