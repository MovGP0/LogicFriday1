use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure
{
    output_first_part: usize,
    output_count: usize,
    total_parts: usize,
}

impl CubeStructure
{
    pub fn new(output_first_part: usize, output_count: usize, total_parts: usize) -> Result<Self, OpoError>
    {
        if output_count == 0
        {
            return Err(OpoError::MissingOutput);
        }

        let Some(output_end) = output_first_part.checked_add(output_count) else
        {
            return Err(OpoError::OutputRangeOutOfBounds);
        };

        if output_end > total_parts
        {
            return Err(OpoError::OutputRangeOutOfBounds);
        }

        Ok(Self {
            output_first_part,
            output_count,
            total_parts,
        })
    }

    pub fn output_first_part(&self) -> usize
    {
        self.output_first_part
    }

    pub fn output_count(&self) -> usize
    {
        self.output_count
    }

    pub fn total_parts(&self) -> usize
    {
        self.total_parts
    }

    fn output_part(&self, output: usize) -> Result<usize, OpoError>
    {
        if output >= self.output_count
        {
            return Err(OpoError::OutputIndexOutOfRange {
                output,
                output_count: self.output_count,
            });
        }

        Ok(self.output_first_part + output)
    }

    fn full_cube(&self) -> Cube
    {
        Cube::from_parts(0..self.total_parts)
    }

    fn output_mask(&self) -> Cube
    {
        Cube::from_parts(self.output_first_part..self.output_first_part + self.output_count)
    }

    fn phase_complement(&self, phase: &Cube) -> Cube
    {
        let output_mask = self.output_mask();
        let non_outputs = self.full_cube().difference(&output_mask);
        non_outputs.union(&output_mask.difference(phase))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Cube
{
    parts: BTreeSet<usize>,
}

impl Cube
{
    pub fn empty() -> Self
    {
        Self {
            parts: BTreeSet::new(),
        }
    }

    pub fn from_parts(parts: impl IntoIterator<Item = usize>) -> Self
    {
        Self {
            parts: parts.into_iter().collect(),
        }
    }

    pub fn contains(&self, part: usize) -> bool
    {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize)
    {
        self.parts.insert(part);
    }

    pub fn remove(&mut self, part: usize)
    {
        self.parts.remove(&part);
    }

    pub fn is_empty(&self) -> bool
    {
        self.parts.is_empty()
    }

    pub fn len(&self) -> usize
    {
        self.parts.len()
    }

    pub fn parts(&self) -> &BTreeSet<usize>
    {
        &self.parts
    }

    pub fn union(&self, other: &Self) -> Self
    {
        Self {
            parts: self.parts.union(&other.parts).copied().collect(),
        }
    }

    pub fn intersection(&self, other: &Self) -> Self
    {
        Self {
            parts: self.parts.intersection(&other.parts).copied().collect(),
        }
    }

    pub fn difference(&self, other: &Self) -> Self
    {
        Self {
            parts: self.parts.difference(&other.parts).copied().collect(),
        }
    }

    pub fn is_disjoint(&self, other: &Self) -> bool
    {
        self.parts.is_disjoint(&other.parts)
    }

    pub fn is_subset(&self, other: &Self) -> bool
    {
        self.parts.is_subset(&other.parts)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover
{
    part_count: usize,
    cubes: Vec<Cube>,
}

impl Cover
{
    pub fn new(part_count: usize, cubes: impl IntoIterator<Item = Cube>) -> Result<Self, OpoError>
    {
        let cubes = cubes.into_iter().collect::<Vec<_>>();
        for cube in &cubes
        {
            if let Some(part) = cube.parts().iter().find(|part| **part >= part_count)
            {
                return Err(OpoError::PartOutOfRange {
                    part: *part,
                    part_count,
                });
            }
        }

        Ok(Self {
            part_count,
            cubes,
        })
    }

    pub fn empty(part_count: usize) -> Self
    {
        Self {
            part_count,
            cubes: Vec::new(),
        }
    }

    pub fn part_count(&self) -> usize
    {
        self.part_count
    }

    pub fn len(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }

    pub fn cubes(&self) -> &[Cube]
    {
        &self.cubes
    }

    pub fn push(&mut self, cube: Cube) -> Result<(), OpoError>
    {
        for part in cube.parts()
        {
            if *part >= self.part_count
            {
                return Err(OpoError::PartOutOfRange {
                    part: *part,
                    part_count: self.part_count,
                });
            }
        }

        self.cubes.push(cube);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pla
{
    pub on_set: Cover,
    pub off_set: Cover,
    pub dont_care_set: Cover,
    pub phase: Option<Cube>,
}

impl Pla
{
    pub fn new(on_set: Cover, off_set: Cover, dont_care_set: Cover, phase: Option<Cube>) -> Result<Self, OpoError>
    {
        ensure_same_part_count(&on_set, &off_set)?;
        ensure_same_part_count(&on_set, &dont_care_set)?;

        Ok(Self {
            on_set,
            off_set,
            dont_care_set,
            phase,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhaseAssignmentOptions
{
    pub repeated: bool,
}

impl PhaseAssignmentOptions
{
    pub const fn single_pass() -> Self
    {
        Self {
            repeated: false,
        }
    }

    pub const fn repeated() -> Self
    {
        Self {
            repeated: true,
        }
    }
}

impl Default for PhaseAssignmentOptions
{
    fn default() -> Self
    {
        Self::single_pass()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OpoError
{
    MissingOutput,
    OutputRangeOutOfBounds,
    OutputIndexOutOfRange { output: usize, output_count: usize },
    FirstOutputOutOfRange { first_output: usize, output_count: usize },
    PartOutOfRange { part: usize, part_count: usize },
    CoverPartCountMismatch { left: usize, right: usize },
    NoPhaseChoices,
    AmbiguousPhase { output: usize },
}

impl fmt::Display for OpoError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingOutput => write!(formatter, "phase assignment requires at least one output"),
            Self::OutputRangeOutOfBounds => write!(formatter, "output range exceeds cube part count"),
            Self::OutputIndexOutOfRange {
                output,
                output_count,
            } => write!(formatter, "output {output} is outside 0..{output_count}"),
            Self::FirstOutputOutOfRange {
                first_output,
                output_count,
            } => write!(formatter, "first output {first_output} is outside 0..{output_count}"),
            Self::PartOutOfRange { part, part_count } => {
                write!(formatter, "part {part} is outside 0..{part_count}")
            }
            Self::CoverPartCountMismatch { left, right } => {
                write!(formatter, "cover part counts differ: {left} != {right}")
            }
            Self::NoPhaseChoices => write!(formatter, "phase selection produced no candidate rows"),
            Self::AmbiguousPhase { output } => write!(formatter, "both phases remained uncovered for output {output}"),
        }
    }
}

impl Error for OpoError {}

pub type OpoResult<T> = Result<T, OpoError>;

pub fn phase_assignment<M>(
    pla: &mut Pla,
    structure: &CubeStructure,
    options: PhaseAssignmentOptions,
    mut minimize: M,
) -> OpoResult<()>
where
    M: FnMut(&mut Pla) -> OpoResult<()>,
{
    if options.repeated
    {
        pla.phase = Some(structure.full_cube());
        repeated_phase_assignment(pla, structure, &mut minimize)?;
    }
    else
    {
        pla.phase = Some(find_phase(pla, structure, 0, None, &mut minimize)?);
    }

    set_phase(pla, structure)?;
    minimize(pla)
}

pub fn repeated_phase_assignment<M>(
    pla: &mut Pla,
    structure: &CubeStructure,
    minimize: &mut M,
) -> OpoResult<()>
where
    M: FnMut(&mut Pla) -> OpoResult<()>,
{
    let mut phase = pla.phase.clone().unwrap_or_else(|| structure.full_cube());
    for output in 0..structure.output_count()
    {
        let candidate = find_phase(pla, structure, output, Some(&phase), minimize)?;
        let output_part = structure.output_part(output)?;
        if !candidate.contains(output_part)
        {
            phase.remove(output_part);
        }
    }

    pla.phase = Some(phase);
    Ok(())
}

pub fn find_phase<M>(
    pla: &Pla,
    structure: &CubeStructure,
    first_output: usize,
    phase: Option<&Cube>,
    minimize: &mut M,
) -> OpoResult<Cube>
where
    M: FnMut(&mut Pla) -> OpoResult<()>,
{
    if first_output >= structure.output_count()
    {
        return Err(OpoError::FirstOutputOutOfRange {
            first_output,
            output_count: structure.output_count(),
        });
    }

    let mut working = Pla::new(
        pla.on_set.clone(),
        pla.off_set.clone(),
        pla.dont_care_set.clone(),
        phase.cloned(),
    )?;

    if phase.is_some()
    {
        set_phase(&mut working, structure)?;
    }

    let doubled_structure = output_phase_setup(&mut working, structure, first_output)?;
    minimize(&mut working)?;
    select_output_phase(&working.on_set, &doubled_structure, first_output)
}

pub fn output_phase_setup(
    pla: &mut Pla,
    structure: &CubeStructure,
    first_output: usize,
) -> OpoResult<CubeStructure>
{
    if first_output >= structure.output_count()
    {
        return Err(OpoError::FirstOutputOutOfRange {
            first_output,
            output_count: structure.output_count(),
        });
    }

    let old_part_count = structure.total_parts();
    let output_first = structure.output_first_part();
    let first_part = structure.output_part(first_output)?;
    let offset = structure.output_count() - first_output;
    let new_part_count = old_part_count + offset;
    let unchanged_mask = Cube::from_parts((0..old_part_count).filter(|part| *part < first_part));
    let input_mask = Cube::from_parts((0..old_part_count).filter(|part| *part < output_first));

    let old_on = pla.on_set.clone();
    let old_off = pla.off_set.clone();
    let old_dc = pla.dont_care_set.clone();
    let mut on_set = Cover::empty(new_part_count);
    let mut off_set = Cover::empty(new_part_count);
    let mut dont_care_set = Cover::empty(new_part_count);

    for cube in old_on.cubes()
    {
        let mut on_cube = cube.intersection(&unchanged_mask);
        let mut off_cube = cube.intersection(&input_mask);
        let mut save_off = false;

        for part in first_part..output_first + structure.output_count()
        {
            if cube.contains(part)
            {
                on_cube.insert(part);
                off_cube.insert(part + offset);
                save_off = true;
            }
        }

        on_set.push(on_cube)?;
        if save_off
        {
            off_set.push(off_cube)?;
        }
    }

    for cube in old_off.cubes()
    {
        let mut on_cube = cube.intersection(&input_mask);
        let mut off_cube = cube.intersection(&unchanged_mask);
        let mut save_on = false;

        for part in first_part..output_first + structure.output_count()
        {
            if cube.contains(part)
            {
                on_cube.insert(part + offset);
                off_cube.insert(part);
                save_on = true;
            }
        }

        if save_on
        {
            on_set.push(on_cube)?;
        }
        off_set.push(off_cube)?;
    }

    for cube in old_dc.cubes()
    {
        let mut dc_cube = cube.intersection(&unchanged_mask);
        for part in first_part..output_first + structure.output_count()
        {
            if cube.contains(part)
            {
                dc_cube.insert(part);
                dc_cube.insert(part + offset);
            }
        }

        dont_care_set.push(dc_cube)?;
    }

    pla.on_set = on_set;
    pla.off_set = off_set;
    pla.dont_care_set = dont_care_set;
    pla.phase = None;

    CubeStructure::new(output_first, structure.output_count() + offset, new_part_count)
}

pub fn set_phase(pla: &mut Pla, structure: &CubeStructure) -> OpoResult<()>
{
    let phase = pla.phase.clone().unwrap_or_else(|| structure.full_cube());
    let inverse_phase = structure.phase_complement(&phase);
    let output_mask = structure.output_mask();
    let mut on_set = Cover::empty(pla.on_set.part_count());
    let mut off_set = Cover::empty(pla.off_set.part_count());

    for cube in pla.on_set.cubes()
    {
        let selected = cube.intersection(&phase);
        if !selected.is_disjoint(&output_mask)
        {
            on_set.push(selected)?;
        }

        let unselected = cube.intersection(&inverse_phase);
        if !unselected.is_disjoint(&output_mask)
        {
            off_set.push(unselected)?;
        }
    }

    for cube in pla.off_set.cubes()
    {
        let selected = cube.intersection(&phase);
        if !selected.is_disjoint(&output_mask)
        {
            off_set.push(selected)?;
        }

        let unselected = cube.intersection(&inverse_phase);
        if !unselected.is_disjoint(&output_mask)
        {
            on_set.push(unselected)?;
        }
    }

    pla.on_set = on_set;
    pla.off_set = off_set;
    Ok(())
}

pub fn select_output_phase(
    minimized: &Cover,
    doubled_structure: &CubeStructure,
    first_output: usize,
) -> OpoResult<Cube>
{
    if first_output >= doubled_structure.output_count()
    {
        return Err(OpoError::FirstOutputOutOfRange {
            first_output,
            output_count: doubled_structure.output_count(),
        });
    }

    let offset = (doubled_structure.output_count() - first_output) / 2;
    if offset == 0
    {
        return Err(OpoError::NoPhaseChoices);
    }

    let last_output = first_output + offset - 1;
    let last_part = doubled_structure.output_part(last_output)?;
    let first_part = doubled_structure.output_part(first_output)?;
    let original_part_count = doubled_structure.total_parts() - offset;
    let mut phase = Cube::from_parts(0..original_part_count);
    let selected = opo_choice_row(minimized, first_part, last_part, offset)?;

    for output in first_output..=last_output
    {
        let positive = doubled_structure.output_part(output)?;
        let negative = doubled_structure.output_part(output + offset)?;
        let positive_covered = minimized
            .cubes()
            .iter()
            .enumerate()
            .any(|(index, cube)| !selected.contains(index) && cube.contains(positive));
        let negative_covered = minimized
            .cubes()
            .iter()
            .enumerate()
            .any(|(index, cube)| !selected.contains(index) && cube.contains(negative));

        match (positive_covered, negative_covered)
        {
            (true, false) | (true, true) => {}
            (false, true) => phase.remove(positive),
            (false, false) => {
                return Err(OpoError::AmbiguousPhase {
                    output,
                });
            }
        }
    }

    Ok(phase)
}

pub fn opo_choice_row(
    cover: &Cover,
    first_output: usize,
    last_output: usize,
    offset: usize,
) -> OpoResult<Cube>
{
    if first_output > last_output
    {
        return Err(OpoError::NoPhaseChoices);
    }

    let select = Cube::from_parts(0..cover.len());
    let choices = opo_recur(cover, &select, offset, first_output, last_output, true)?;
    choices
        .cubes()
        .first()
        .cloned()
        .ok_or(OpoError::NoPhaseChoices)
}

pub fn opo_recur(
    cover: &Cover,
    select: &Cube,
    offset: usize,
    first: usize,
    last: usize,
    largest_only: bool,
) -> OpoResult<Cover>
{
    if first == last
    {
        return opo_leaf(cover, select, first, first + offset);
    }

    let middle = (first + last) / 2;
    let left = opo_recur(cover, select, offset, first, middle, false)?;
    let right = opo_recur(cover, select, offset, middle + 1, last, false)?;

    intersect_choice_covers(&left, &right, largest_only)
}

pub fn opo_leaf(cover: &Cover, select: &Cube, out1: usize, out2: usize) -> OpoResult<Cover>
{
    let mut rows = Vec::with_capacity(2);
    for output in [out1, out2]
    {
        let mut row = select.clone();
        for (index, cube) in cover.cubes().iter().enumerate()
        {
            if cube.contains(output)
            {
                row.remove(index);
            }
        }

        rows.push(row);
    }

    Cover::new(cover.len(), rows)
}

pub fn intersect_choice_covers(left: &Cover, right: &Cover, largest_only: bool) -> OpoResult<Cover>
{
    ensure_same_part_count(left, right)?;

    let mut rows = Vec::new();
    let mut largest = 0;
    for left_row in left.cubes()
    {
        for right_row in right.cubes()
        {
            let row = left_row.intersection(right_row);
            if row.is_empty()
            {
                continue;
            }

            if largest_only
            {
                let row_size = row.len();
                if row_size > largest
                {
                    rows.clear();
                    largest = row_size;
                }

                if row_size < largest
                {
                    continue;
                }
            }

            rows.push(row);
        }
    }

    retain_maximal(&mut rows);
    Cover::new(left.part_count(), rows)
}

fn ensure_same_part_count(left: &Cover, right: &Cover) -> OpoResult<()>
{
    if left.part_count() != right.part_count()
    {
        return Err(OpoError::CoverPartCountMismatch {
            left: left.part_count(),
            right: right.part_count(),
        });
    }

    Ok(())
}

fn retain_maximal(rows: &mut Vec<Cube>)
{
    rows.sort();
    rows.dedup();
    let original = rows.clone();
    rows.retain(|row| {
        !original
            .iter()
            .any(|candidate| candidate != row && row.is_subset(candidate))
    });
    rows.sort_by_key(|row| (usize::MAX - row.len(), row.parts().clone()));
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn cube(parts: &[usize]) -> Cube
    {
        Cube::from_parts(parts.iter().copied())
    }

    fn cover(part_count: usize, cubes: &[&[usize]]) -> Cover
    {
        Cover::new(part_count, cubes.iter().map(|parts| cube(parts))).unwrap()
    }

    #[test]
    fn output_phase_setup_duplicates_undecided_outputs()
    {
        let structure = CubeStructure::new(2, 2, 4).unwrap();
        let mut pla = Pla::new(
            cover(4, &[&[0, 2], &[1, 3]]),
            cover(4, &[&[0, 3]]),
            cover(4, &[&[1, 2, 3]]),
            None,
        )
        .unwrap();

        let doubled = output_phase_setup(&mut pla, &structure, 0).unwrap();

        assert_eq!(doubled, CubeStructure::new(2, 4, 6).unwrap());
        assert_eq!(pla.on_set, cover(6, &[&[0, 2], &[1, 3], &[0, 5]]));
        assert_eq!(pla.off_set, cover(6, &[&[0, 4], &[1, 5], &[0, 3]]));
        assert_eq!(pla.dont_care_set, cover(6, &[&[1, 2, 3, 4, 5]]));
    }

    #[test]
    fn set_phase_routes_selected_and_unselected_outputs()
    {
        let structure = CubeStructure::new(2, 2, 4).unwrap();
        let mut pla = Pla::new(
            cover(4, &[&[0, 2], &[1, 3]]),
            cover(4, &[&[0, 2], &[1, 3]]),
            Cover::empty(4),
            Some(cube(&[0, 1, 2])),
        )
        .unwrap();

        set_phase(&mut pla, &structure).unwrap();

        assert_eq!(pla.on_set, cover(4, &[&[0, 2], &[1, 3]]));
        assert_eq!(pla.off_set, cover(4, &[&[1, 3], &[0, 2]]));
    }

    #[test]
    fn opo_leaf_records_primes_not_needed_by_each_phase()
    {
        let input = cover(5, &[&[0, 2], &[1, 3], &[1, 4]]);
        let select = cube(&[0, 1, 2]);

        let result = opo_leaf(&input, &select, 3, 4).unwrap();

        assert_eq!(result, cover(3, &[&[0, 2], &[0, 1]]));
    }

    #[test]
    fn opo_choice_row_multiplies_phase_choices_and_keeps_largest()
    {
        let cover = cover(6, &[&[0, 2, 4], &[1, 3], &[1, 5]]);

        let result = opo_choice_row(&cover, 2, 3, 2).unwrap();

        assert_eq!(result, cube(&[1]));
    }

    #[test]
    fn select_output_phase_removes_outputs_implemented_by_complement()
    {
        let doubled = CubeStructure::new(2, 4, 6).unwrap();
        let minimized = cover(6, &[&[0, 2, 4], &[1, 3], &[1, 5]]);

        let phase = select_output_phase(&minimized, &doubled, 0).unwrap();

        assert!(phase.contains(2));
        assert!(!phase.contains(3));
    }

    #[test]
    fn phase_assignment_uses_minimizer_callback_and_applies_phase()
    {
        let structure = CubeStructure::new(2, 1, 3).unwrap();
        let mut pla = Pla::new(
            cover(3, &[&[0, 2]]),
            cover(3, &[&[1, 2]]),
            Cover::empty(3),
            None,
        )
        .unwrap();
        let mut calls = 0;

        phase_assignment(&mut pla, &structure, PhaseAssignmentOptions::default(), |pla| {
            calls += 1;
            if pla.on_set.part_count() == 4
            {
                pla.on_set = cover(4, &[&[0, 2], &[1, 3]]);
            }

            Ok(())
        })
        .unwrap();

        assert_eq!(calls, 2);
        assert_eq!(pla.phase, Some(cube(&[0, 1, 2])));
        assert_eq!(pla.on_set, cover(3, &[&[0, 2]]));
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("opo.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }
}
