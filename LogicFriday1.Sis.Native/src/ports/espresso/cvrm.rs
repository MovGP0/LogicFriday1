use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Variable
{
    first_part: usize,
    last_part: usize,
}

impl Variable
{
    pub const fn new(first_part: usize, last_part: usize) -> Self
    {
        Self {
            first_part,
            last_part,
        }
    }

    pub const fn first_part(self) -> usize
    {
        self.first_part
    }

    pub const fn last_part(self) -> usize
    {
        self.last_part
    }

    pub fn parts(self) -> std::ops::RangeInclusive<usize>
    {
        self.first_part..=self.last_part
    }

    pub fn len(self) -> usize
    {
        self.last_part - self.first_part + 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure
{
    variables: Vec<Variable>,
    set_size: usize,
    output: Option<usize>,
}

impl CubeStructure
{
    pub fn new(variables: impl IntoIterator<Item = Variable>) -> CvrmResult<Self>
    {
        let variables = variables.into_iter().collect::<Vec<_>>();
        let mut previous_last = None;
        for variable in &variables
        {
            if variable.last_part < variable.first_part
            {
                return Err(CvrmError::InvalidVariable {
                    first_part: variable.first_part,
                    last_part: variable.last_part,
                });
            }

            if let Some(previous_last) = previous_last
            {
                if variable.first_part <= previous_last
                {
                    return Err(CvrmError::OverlappingVariables);
                }
            }

            previous_last = Some(variable.last_part);
        }

        let set_size = variables
            .last()
            .map(|variable| variable.last_part + 1)
            .unwrap_or(0);

        Ok(Self {
            variables,
            set_size,
            output: None,
        })
    }

    pub fn with_output(mut self, output: usize) -> CvrmResult<Self>
    {
        if output >= self.variables.len()
        {
            return Err(CvrmError::VariableOutOfRange {
                variable: output,
                variable_count: self.variables.len(),
            });
        }

        self.output = Some(output);
        Ok(self)
    }

    pub fn variable(&self, index: usize) -> CvrmResult<Variable>
    {
        self.variables
            .get(index)
            .copied()
            .ok_or(CvrmError::VariableOutOfRange {
                variable: index,
                variable_count: self.variables.len(),
            })
    }

    pub fn variables(&self) -> &[Variable]
    {
        &self.variables
    }

    pub fn variable_count(&self) -> usize
    {
        self.variables.len()
    }

    pub fn set_size(&self) -> usize
    {
        self.set_size
    }

    pub fn output_variable(&self) -> Option<Variable>
    {
        self.output.and_then(|index| self.variables.get(index).copied())
    }

    fn full_cube(&self) -> Cube
    {
        Cube::from_parts(0..self.set_size)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube
{
    parts: BTreeSet<usize>,
    prime: bool,
}

impl Cube
{
    pub fn empty() -> Self
    {
        Self {
            parts: BTreeSet::new(),
            prime: false,
        }
    }

    pub fn from_parts(parts: impl IntoIterator<Item = usize>) -> Self
    {
        Self {
            parts: parts.into_iter().collect(),
            prime: false,
        }
    }

    pub fn parts(&self) -> &BTreeSet<usize>
    {
        &self.parts
    }

    pub fn contains(&self, part: usize) -> bool
    {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> bool
    {
        self.parts.insert(part)
    }

    pub fn remove(&mut self, part: usize) -> bool
    {
        self.parts.remove(&part)
    }

    pub fn len(&self) -> usize
    {
        self.parts.len()
    }

    pub fn is_prime(&self) -> bool
    {
        self.prime
    }

    pub fn set_prime(&mut self, prime: bool)
    {
        self.prime = prime;
    }

    pub fn part_count_for(&self, variable: Variable) -> usize
    {
        variable.parts().filter(|part| self.contains(*part)).count()
    }

    pub fn contains_any_for(&self, variable: Variable) -> bool
    {
        variable.parts().any(|part| self.contains(part))
    }

    pub fn contains_all_for(&self, variable: Variable) -> bool
    {
        variable.parts().all(|part| self.contains(part))
    }

    pub fn union(&self, other: &Self) -> Self
    {
        Self {
            parts: self.parts.union(&other.parts).copied().collect(),
            prime: self.prime || other.prime,
        }
    }

    pub fn intersection(&self, other: &Self) -> Self
    {
        Self {
            parts: self.parts.intersection(&other.parts).copied().collect(),
            prime: self.prime && other.prime,
        }
    }

    pub fn difference(&self, other: &Self) -> Self
    {
        Self {
            parts: self.parts.difference(&other.parts).copied().collect(),
            prime: self.prime,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover
{
    set_size: usize,
    cubes: Vec<Cube>,
}

impl Cover
{
    pub fn new(set_size: usize, cubes: impl IntoIterator<Item = Cube>) -> CvrmResult<Self>
    {
        let cubes = cubes.into_iter().collect::<Vec<_>>();
        for cube in &cubes
        {
            if let Some(part) = cube.parts().iter().find(|part| **part >= set_size)
            {
                return Err(CvrmError::PartOutOfRange {
                    part: *part,
                    set_size,
                });
            }
        }

        Ok(Self {
            set_size,
            cubes,
        })
    }

    pub fn empty(set_size: usize) -> Self
    {
        Self {
            set_size,
            cubes: Vec::new(),
        }
    }

    pub fn set_size(&self) -> usize
    {
        self.set_size
    }

    pub fn cubes(&self) -> &[Cube]
    {
        &self.cubes
    }

    pub fn cubes_mut(&mut self) -> &mut [Cube]
    {
        &mut self.cubes
    }

    pub fn into_cubes(self) -> Vec<Cube>
    {
        self.cubes
    }

    pub fn len(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }

    pub fn push(&mut self, cube: Cube) -> CvrmResult<()>
    {
        if let Some(part) = cube.parts().iter().find(|part| **part >= self.set_size)
        {
            return Err(CvrmError::PartOutOfRange {
                part: *part,
                set_size: self.set_size,
            });
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
    pub fn new(on_set: Cover, off_set: Cover, dont_care_set: Cover) -> CvrmResult<Self>
    {
        ensure_same_set_size(&on_set, &off_set)?;
        ensure_same_set_size(&on_set, &dont_care_set)?;

        Ok(Self {
            on_set,
            off_set,
            dont_care_set,
            phase: None,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubelistPartition
{
    pub selected: Vec<Cube>,
    pub remainder: Vec<Cube>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CvrmError
{
    InvalidVariable { first_part: usize, last_part: usize },
    OverlappingVariables,
    VariableOutOfRange { variable: usize, variable_count: usize },
    RangeOrder { start: usize, end: usize },
    CoverSetSizeMismatch { left: usize, right: usize },
    PartOutOfRange { part: usize, set_size: usize },
    ExpansionTooLarge { expansion: usize, limit: usize },
    MissingOutputVariable,
    EmptyCubelist,
}

impl fmt::Display for CvrmError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::InvalidVariable {
                first_part,
                last_part,
            } => write!(
                formatter,
                "variable range {first_part}..={last_part} is invalid"
            ),
            Self::OverlappingVariables => write!(formatter, "variable part ranges overlap"),
            Self::VariableOutOfRange {
                variable,
                variable_count,
            } => write!(
                formatter,
                "variable {variable} is outside 0..{variable_count}"
            ),
            Self::RangeOrder {
                start,
                end,
            } => write!(formatter, "range start {start} is after end {end}"),
            Self::CoverSetSizeMismatch {
                left,
                right,
            } => write!(formatter, "cover set sizes differ: {left} != {right}"),
            Self::PartOutOfRange {
                part,
                set_size,
            } => write!(formatter, "part {part} is outside 0..{set_size}"),
            Self::ExpansionTooLarge {
                expansion,
                limit,
            } => write!(
                formatter,
                "unravel expansion {expansion} exceeds limit {limit}"
            ),
            Self::MissingOutputVariable => write!(formatter, "cube structure has no output variable"),
            Self::EmptyCubelist => write!(formatter, "cubelist partition requires at least one cube"),
        }
    }
}

impl Error for CvrmError {}

pub type CvrmResult<T> = Result<T, CvrmError>;

pub fn unravel(cover: &Cover, structure: &CubeStructure, start: usize) -> CvrmResult<Cover>
{
    let end = structure.variable_count().saturating_sub(1);
    unravel_range(cover, structure, start, end)
}

pub fn unravel_range(
    cover: &Cover,
    structure: &CubeStructure,
    start: usize,
    end: usize,
) -> CvrmResult<Cover>
{
    validate_structure_cover(structure, cover)?;
    if start > end
    {
        return Err(CvrmError::RangeOrder {
            start,
            end,
        });
    }

    structure.variable(start)?;
    structure.variable(end)?;

    let mut startbase = Cube::empty();
    for variable in structure.variables().iter().take(start)
    {
        for part in variable.parts()
        {
            startbase.insert(part);
        }
    }

    for variable in structure.variables().iter().skip(end + 1)
    {
        for part in variable.parts()
        {
            startbase.insert(part);
        }
    }

    let mut output = Cover::empty(cover.set_size());
    for cube in cover.cubes()
    {
        cb_unravel(cube, start, end, &startbase, &mut output, structure)?;
    }

    Ok(output)
}

pub fn lex_sort(mut cover: Cover) -> Cover
{
    cover.cubes.sort_by(|left, right| left.parts.cmp(&right.parts));
    cover
}

pub fn size_sort(mut cover: Cover) -> Cover
{
    cover.cubes.sort_by_key(|cube| Reverse(cube.len()));
    cover
}

pub fn mini_sort_by(mut cover: Cover, compare_descending: bool) -> Cover
{
    let weights = column_counts(&cover);
    cover.cubes.sort_by_key(|cube| {
        let score = cube
            .parts()
            .iter()
            .map(|part| weights.get(*part).copied().unwrap_or(0))
            .sum::<usize>();

        if compare_descending
        {
            (Reverse(score), cube.parts().clone())
        }
        else
        {
            (Reverse(usize::MAX - score), cube.parts().clone())
        }
    });

    cover
}

pub fn sort_reduce(mut cover: Cover, structure: &CubeStructure) -> CvrmResult<Cover>
{
    validate_structure_cover(structure, &cover)?;
    if cover.is_empty()
    {
        return Ok(cover);
    }

    let largest = cover
        .cubes()
        .iter()
        .max_by_key(|cube| cube.len())
        .cloned()
        .expect("non-empty cover has largest cube");
    let variable_count = structure.variable_count();

    cover.cubes.sort_by_key(|cube| {
        let distance = cube_distance(&largest, cube, structure);
        let score = ((variable_count - distance) << 7) + cube.len().min(127);
        (Reverse(score), cube.parts().clone())
    });

    Ok(cover)
}

pub fn random_order(mut cover: Cover) -> Cover
{
    for index in (1..cover.cubes.len()).rev()
    {
        let swap_index = (index * 23 + 997) % index;
        cover.cubes.swap(index, swap_index);
    }

    cover
}

pub fn cubelist_partition(
    cofactor: &Cube,
    cubes: &[Cube],
    structure: &CubeStructure,
) -> CvrmResult<Option<CubelistPartition>>
{
    if cubes.is_empty()
    {
        return Err(CvrmError::EmptyCubelist);
    }

    let mut covered = vec![false; cubes.len()];
    let mut seed = cubes[0].clone();
    covered[0] = true;

    loop
    {
        let mut changed = false;
        for (index, cube) in cubes.iter().enumerate()
        {
            if !covered[index] && cubes_share_component(cube, &seed, cofactor, structure)
            {
                seed = seed.intersection(cube);
                covered[index] = true;
                changed = true;
            }
        }

        if !changed
        {
            break;
        }
    }

    if covered.iter().all(|value| *value)
    {
        return Ok(None);
    }

    let mut selected = Vec::new();
    let mut remainder = Vec::new();
    for (cube, is_covered) in cubes.iter().cloned().zip(covered)
    {
        if is_covered
        {
            selected.push(cube);
        }
        else
        {
            remainder.push(cube);
        }
    }

    Ok(Some(CubelistPartition {
        selected,
        remainder,
    }))
}

pub fn cof_output(cover: &Cover, structure: &CubeStructure, output_part: usize) -> CvrmResult<Cover>
{
    let output = structure
        .output_variable()
        .ok_or(CvrmError::MissingOutputVariable)?;
    validate_structure_cover(structure, cover)?;

    let output_mask = Cube::from_parts(output.parts());
    let mut result = Cover::empty(cover.set_size());
    for cube in cover.cubes()
    {
        if cube.contains(output_part)
        {
            let mut cofactored = cube.union(&output_mask);
            cofactored.set_prime(false);
            result.push(cofactored)?;
        }
    }

    Ok(result)
}

pub fn uncof_output(
    cover: Option<Cover>,
    structure: &CubeStructure,
    output_part: usize,
) -> CvrmResult<Option<Cover>>
{
    let output = structure
        .output_variable()
        .ok_or(CvrmError::MissingOutputVariable)?;
    let Some(mut cover) = cover
    else
    {
        return Ok(None);
    };

    validate_structure_cover(structure, &cover)?;
    let output_mask = Cube::from_parts(output.parts());
    for cube in cover.cubes_mut()
    {
        *cube = cube.difference(&output_mask);
        cube.insert(output_part);
    }

    Ok(Some(cover))
}

pub fn foreach_output_function<P, S>(
    pla: &Pla,
    structure: &CubeStructure,
    mut process: P,
    mut save: S,
) -> CvrmResult<()>
where
    P: FnMut(&mut Pla, usize) -> CvrmResult<bool>,
    S: FnMut(&mut Pla, usize) -> CvrmResult<bool>,
{
    let output = structure
        .output_variable()
        .ok_or(CvrmError::MissingOutputVariable)?;

    for (index, output_part) in output.parts().enumerate()
    {
        let mut single_output = Pla::new(
            cof_output(&pla.on_set, structure, output_part)?,
            cof_output(&pla.off_set, structure, output_part)?,
            cof_output(&pla.dont_care_set, structure, output_part)?,
        )?;

        if !process(&mut single_output, index)?
        {
            return Ok(());
        }

        single_output.on_set =
            uncof_output(Some(single_output.on_set), structure, output_part)?.unwrap();
        single_output.off_set =
            uncof_output(Some(single_output.off_set), structure, output_part)?.unwrap();
        single_output.dont_care_set =
            uncof_output(Some(single_output.dont_care_set), structure, output_part)?.unwrap();

        if !save(&mut single_output, index)?
        {
            return Ok(());
        }
    }

    Ok(())
}

pub fn so_espresso<M>(
    pla: &mut Pla,
    structure: &CubeStructure,
    mut minimize: M,
) -> CvrmResult<()>
where
    M: FnMut(Cover, Cover, Cover, usize) -> CvrmResult<Cover>,
{
    let mut minimized = Cover::empty(pla.on_set.set_size());
    foreach_output_function(
        pla,
        structure,
        |single_output, index| {
            single_output.on_set = minimize(
                single_output.on_set.clone(),
                single_output.dont_care_set.clone(),
                single_output.off_set.clone(),
                index,
            )?;
            Ok(true)
        },
        |single_output, _index| {
            append_cover(&mut minimized, single_output.on_set.clone())?;
            Ok(true)
        },
    )?;

    pla.on_set = minimized;
    Ok(())
}

pub fn so_both_espresso<M>(
    pla: &mut Pla,
    structure: &CubeStructure,
    mut minimize: M,
) -> CvrmResult<()>
where
    M: FnMut(Cover, Cover, Cover, bool, usize) -> CvrmResult<Cover>,
{
    let mut phase = structure.full_cube();
    let mut minimized = Cover::empty(pla.on_set.set_size());
    let output = structure
        .output_variable()
        .ok_or(CvrmError::MissingOutputVariable)?;

    foreach_output_function(
        pla,
        structure,
        |single_output, index| {
            single_output.on_set = minimize(
                single_output.on_set.clone(),
                single_output.dont_care_set.clone(),
                single_output.off_set.clone(),
                true,
                index,
            )?;
            single_output.off_set = minimize(
                single_output.off_set.clone(),
                single_output.dont_care_set.clone(),
                single_output.on_set.clone(),
                false,
                index,
            )?;
            Ok(true)
        },
        |single_output, index| {
            if single_output.on_set.len() > single_output.off_set.len()
            {
                let output_part = output.first_part() + index;
                phase.remove(output_part);
                append_cover(&mut minimized, single_output.off_set.clone())?;
            }
            else
            {
                append_cover(&mut minimized, single_output.on_set.clone())?;
            }

            Ok(true)
        },
    )?;

    pla.on_set = minimized;
    pla.phase = Some(phase);
    Ok(())
}

fn cb_unravel(
    cube: &Cube,
    start: usize,
    end: usize,
    startbase: &Cube,
    output: &mut Cover,
    structure: &CubeStructure,
) -> CvrmResult<()>
{
    let mut expansion = 1usize;
    let mut base = startbase.clone();
    for variable_index in start..=end
    {
        let variable = structure.variable(variable_index)?;
        let size = cube.part_count_for(variable);
        if size < 2
        {
            for part in variable.parts()
            {
                base.insert(part);
            }
        }
        else
        {
            expansion = expansion
                .checked_mul(size)
                .ok_or(CvrmError::ExpansionTooLarge {
                    expansion: usize::MAX,
                    limit: 1_000_000,
                })?;
            if expansion > 1_000_000
            {
                return Err(CvrmError::ExpansionTooLarge {
                    expansion,
                    limit: 1_000_000,
                });
            }
        }
    }

    base = cube.intersection(&base);
    let offset = output.len();
    for _ in 0..expansion
    {
        output.push(base.clone())?;
    }

    let mut place = expansion;
    for variable_index in start..=end
    {
        let variable = structure.variable(variable_index)?;
        let size = cube.part_count_for(variable);
        if size > 1
        {
            let skip = place;
            place /= size;
            let mut n = 0;
            for part in variable.parts()
            {
                if cube.contains(part)
                {
                    for j in (n..expansion).step_by(skip)
                    {
                        for k in 0..place
                        {
                            output.cubes_mut()[offset + j + k].insert(part);
                        }
                    }

                    n += place;
                }
            }
        }
    }

    Ok(())
}

fn column_counts(cover: &Cover) -> Vec<usize>
{
    let mut counts = vec![0; cover.set_size()];
    for cube in cover.cubes()
    {
        for part in cube.parts()
        {
            counts[*part] += 1;
        }
    }

    counts
}

fn cube_distance(left: &Cube, right: &Cube, structure: &CubeStructure) -> usize
{
    structure
        .variables()
        .iter()
        .filter(|variable| {
            !variable
                .parts()
                .any(|part| left.contains(part) && right.contains(part))
        })
        .count()
}

fn cubes_share_component(
    cube: &Cube,
    seed: &Cube,
    cofactor: &Cube,
    structure: &CubeStructure,
) -> bool
{
    structure.variables().iter().any(|variable| {
        if cofactor.contains_all_for(*variable)
        {
            return false;
        }

        variable
            .parts()
            .any(|part| cube.contains(part) && seed.contains(part))
    })
}

fn append_cover(target: &mut Cover, source: Cover) -> CvrmResult<()>
{
    ensure_same_set_size(target, &source)?;
    for cube in source.into_cubes()
    {
        target.push(cube)?;
    }

    Ok(())
}

fn validate_structure_cover(structure: &CubeStructure, cover: &Cover) -> CvrmResult<()>
{
    if structure.set_size() != cover.set_size()
    {
        return Err(CvrmError::CoverSetSizeMismatch {
            left: structure.set_size(),
            right: cover.set_size(),
        });
    }

    Ok(())
}

fn ensure_same_set_size(left: &Cover, right: &Cover) -> CvrmResult<()>
{
    if left.set_size() != right.set_size()
    {
        return Err(CvrmError::CoverSetSizeMismatch {
            left: left.set_size(),
            right: right.set_size(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn structure() -> CubeStructure
    {
        CubeStructure::new([
            Variable::new(0, 1),
            Variable::new(2, 4),
            Variable::new(5, 6),
        ])
        .unwrap()
        .with_output(2)
        .unwrap()
    }

    fn cube(parts: &[usize]) -> Cube
    {
        Cube::from_parts(parts.iter().copied())
    }

    fn cover(rows: &[&[usize]]) -> Cover
    {
        Cover::new(7, rows.iter().map(|parts| cube(parts))).unwrap()
    }

    fn rows(cover: Cover) -> Vec<BTreeSet<usize>>
    {
        cover
            .into_cubes()
            .into_iter()
            .map(|cube| cube.parts().clone())
            .collect()
    }

    #[test]
    fn unravel_range_expands_multiple_valued_parts()
    {
        let structure = structure();
        let input = cover(&[&[0, 2, 3, 5], &[1, 4, 6]]);

        let result = unravel_range(&input, &structure, 1, 1).unwrap();

        assert_eq!(
            rows(result),
            vec![
                set(&[0, 2, 5]),
                set(&[0, 3, 5]),
                set(&[1, 4, 6]),
            ]
        );
    }

    #[test]
    fn lex_and_size_sorts_match_espresso_cover_ordering_helpers()
    {
        let input = cover(&[&[3], &[0, 1, 2], &[0]]);

        assert_eq!(
            rows(lex_sort(input.clone())),
            vec![set(&[0]), set(&[0, 1, 2]), set(&[3])]
        );
        assert_eq!(
            rows(size_sort(input)),
            vec![set(&[0, 1, 2]), set(&[3]), set(&[0])]
        );
    }

    #[test]
    fn mini_sort_weights_cubes_by_column_counts()
    {
        let input = cover(&[&[0, 1], &[1, 2], &[1, 3], &[4]]);

        let result = mini_sort_by(input, true);

        assert_eq!(rows(result)[0], set(&[0, 1]));
    }

    #[test]
    fn sort_reduce_prefers_cubes_close_to_largest_cube()
    {
        let structure = structure();
        let input = cover(&[&[1, 4, 6], &[0, 2, 5], &[0, 2, 3, 5]]);

        let result = sort_reduce(input, &structure).unwrap();

        assert_eq!(
            rows(result),
            vec![set(&[0, 2, 3, 5]), set(&[0, 2, 5]), set(&[1, 4, 6])]
        );
    }

    #[test]
    fn deterministic_random_order_uses_legacy_fallback_sequence()
    {
        let input = cover(&[&[0], &[1], &[2], &[3]]);

        let result = random_order(input);

        assert_eq!(rows(result), vec![set(&[2]), set(&[0]), set(&[3]), set(&[1])]);
    }

    #[test]
    fn cubelist_partition_splits_disconnected_components()
    {
        let structure = structure();
        let partition = cubelist_partition(
            &Cube::empty(),
            &[cube(&[0, 2, 5]), cube(&[0, 3, 5]), cube(&[1, 4, 6])],
            &structure,
        )
        .unwrap()
        .unwrap();

        assert_eq!(partition.selected, vec![cube(&[0, 2, 5]), cube(&[0, 3, 5])]);
        assert_eq!(partition.remainder, vec![cube(&[1, 4, 6])]);
    }

    #[test]
    fn output_cofactor_and_uncofactor_restore_selected_output()
    {
        let structure = structure();
        let input = cover(&[&[0, 2, 5], &[1, 3, 6], &[0, 4, 5, 6]]);

        let cofactored = cof_output(&input, &structure, 5).unwrap();
        let restored = uncof_output(Some(cofactored), &structure, 6).unwrap().unwrap();

        assert_eq!(rows(restored), vec![set(&[0, 2, 6]), set(&[0, 4, 6])]);
    }

    #[test]
    fn foreach_output_function_visits_each_output_part()
    {
        let structure = structure();
        let pla = Pla::new(cover(&[&[0, 2, 5], &[1, 3, 6]]), Cover::empty(7), Cover::empty(7))
            .unwrap();
        let mut seen = Vec::new();

        foreach_output_function(
            &pla,
            &structure,
            |single_output, index| {
                seen.push((index, single_output.on_set.len()));
                Ok(true)
            },
            |_single_output, _index| Ok(true),
        )
        .unwrap();

        assert_eq!(seen, vec![(0, 1), (1, 1)]);
    }

    #[test]
    fn so_espresso_appends_minimized_single_output_covers()
    {
        let structure = structure();
        let mut pla = Pla::new(
            cover(&[&[0, 2, 5], &[1, 3, 6]]),
            Cover::empty(7),
            Cover::empty(7),
        )
        .unwrap();

        so_espresso(&mut pla, &structure, |on_set, _dc, _off, _index| {
            Ok(Cover::new(7, on_set.into_cubes().into_iter().take(1)).unwrap())
        })
        .unwrap();

        assert_eq!(rows(pla.on_set), vec![set(&[0, 2, 5]), set(&[1, 3, 6])]);
    }

    #[test]
    fn so_both_espresso_selects_smaller_phase()
    {
        let structure = structure();
        let mut pla = Pla::new(
            cover(&[&[0, 2, 5], &[1, 3, 6]]),
            cover(&[&[0, 4, 5], &[1, 4, 5], &[1, 2, 6]]),
            Cover::empty(7),
        )
        .unwrap();

        so_both_espresso(&mut pla, &structure, |cover, _dc, _other, positive, index| {
            if positive && index == 0
            {
                Ok(cover)
            }
            else if positive
            {
                Ok(Cover::new(7, [cube(&[1, 3, 6]), cube(&[0, 3, 6])]).unwrap())
            }
            else
            {
                Ok(Cover::new(7, [cube(&[1, 2, 6])]).unwrap())
            }
        })
        .unwrap();

        assert_eq!(rows(pla.on_set), vec![set(&[0, 2, 5]), set(&[1, 2, 6])]);
        assert!(!pla.phase.unwrap().contains(6));
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("cvrm.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }

    fn set(parts: &[usize]) -> BTreeSet<usize>
    {
        parts.iter().copied().collect()
    }
}
