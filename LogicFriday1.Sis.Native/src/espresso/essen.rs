use std::collections::BTreeSet;

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

    fn parts(self) -> impl Iterator<Item = usize>
    {
        self.first_part..=self.last_part
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure
{
    variables: Vec<Variable>,
    binary_variable_count: usize,
}

impl CubeStructure
{
    pub fn new(binary_variable_count: usize, variables: impl IntoIterator<Item = Variable>) -> Self
    {
        let variables = variables.into_iter().collect::<Vec<_>>();
        debug_assert!(binary_variable_count <= variables.len());

        for window in variables.windows(2)
        {
            debug_assert!(window[0].last_part < window[1].first_part);
        }

        Self {
            variables,
            binary_variable_count,
        }
    }

    pub fn variables(&self) -> &[Variable]
    {
        &self.variables
    }

    pub fn variable(&self, index: usize) -> Option<Variable>
    {
        self.variables.get(index).copied()
    }

    pub fn variable_count(&self) -> usize
    {
        self.variables.len()
    }

    pub fn binary_variable_count(&self) -> usize
    {
        self.binary_variable_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube
{
    parts: BTreeSet<usize>,
    nonessential: bool,
    relatively_essential: bool,
}

impl Cube
{
    pub fn empty() -> Self
    {
        Self {
            parts: BTreeSet::new(),
            nonessential: false,
            relatively_essential: false,
        }
    }

    pub fn from_parts(parts: impl IntoIterator<Item = usize>) -> Self
    {
        Self {
            parts: parts.into_iter().collect(),
            nonessential: false,
            relatively_essential: false,
        }
    }

    pub fn contains(&self, part: usize) -> bool
    {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> bool
    {
        self.parts.insert(part)
    }

    pub fn parts(&self) -> impl Iterator<Item = usize> + '_
    {
        self.parts.iter().copied()
    }

    pub fn is_nonessential(&self) -> bool
    {
        self.nonessential
    }

    pub fn set_nonessential(&mut self, value: bool)
    {
        self.nonessential = value;
    }

    pub fn is_relatively_essential(&self) -> bool
    {
        self.relatively_essential
    }

    pub fn set_relatively_essential(&mut self, value: bool)
    {
        self.relatively_essential = value;
    }

    fn variable_parts(&self, variable: Variable) -> BTreeSet<usize>
    {
        variable
            .parts()
            .filter(|part| self.contains(*part))
            .collect()
    }

    fn with_variable_parts(&self, variable: Variable, parts: &BTreeSet<usize>) -> Self
    {
        let mut output = Self::from_parts(
            self.parts()
                .filter(|part| !variable.parts().any(|candidate| candidate == *part)),
        );
        output.nonessential = self.nonessential;
        output.relatively_essential = self.relatively_essential;

        for part in parts
        {
            output.insert(*part);
        }

        output
    }

    fn contains_assignment(&self, assignment: &[usize]) -> bool
    {
        assignment.iter().all(|part| self.contains(*part))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover
{
    cubes: Vec<Cube>,
}

impl Cover
{
    pub fn new(cubes: impl IntoIterator<Item = Cube>) -> Self
    {
        Self {
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn empty() -> Self
    {
        Self { cubes: Vec::new() }
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

    pub fn cubes_mut(&mut self) -> &mut [Cube]
    {
        &mut self.cubes
    }

    pub fn push(&mut self, cube: Cube)
    {
        self.cubes.push(cube);
    }

    pub fn append(&mut self, other: &Cover)
    {
        self.cubes.extend(other.cubes.iter().cloned());
    }

    fn joined(&self, other: &Cover) -> Self
    {
        Self::new(self.cubes.iter().chain(&other.cubes).cloned())
    }

    fn covers_cube(&self, cube: &Cube, structure: &CubeStructure) -> bool
    {
        let mut assignment = Vec::with_capacity(structure.variable_count());
        every_assignment_in_cube(cube, structure, 0, &mut assignment, &mut |assignment| {
            self.cubes
                .iter()
                .any(|candidate| candidate.contains_assignment(assignment))
        })
    }
}

pub fn essential(on_set: &mut Cover, dont_care: &mut Cover, structure: &CubeStructure) -> Cover
{
    let mut essential = Cover::empty();
    let mut active = vec![true; on_set.len()];

    for (index, cube) in on_set.cubes().iter().enumerate()
    {
        if cube.is_nonessential() || !cube.is_relatively_essential()
        {
            continue;
        }

        if essen_cube(on_set, dont_care, cube, structure)
        {
            essential.push(cube.clone());
            active[index] = false;
        }
    }

    let mut active_index = 0;
    on_set.cubes.retain(|_| {
        let keep = active[active_index];
        active_index += 1;
        keep
    });
    dont_care.append(&essential);

    essential
}

pub fn essen_cube(on_set: &Cover, dont_care: &Cover, cube: &Cube, structure: &CubeStructure) -> bool
{
    let function_and_dont_care = on_set.joined(dont_care);
    let consensus = cb_consensus(&function_and_dont_care, cube, structure);
    let consensus_and_dont_care = consensus.joined(dont_care);

    !consensus_and_dont_care.covers_cube(cube, structure)
}

pub fn cb_consensus(cover: &Cover, cube: &Cube, structure: &CubeStructure) -> Cover
{
    let mut output = Cover::empty();
    let mut skipped_test_cube = false;

    for candidate in cover.cubes()
    {
        if std::ptr::eq(candidate, cube) || (!skipped_test_cube && candidate == cube)
        {
            skipped_test_cube = true;
            continue;
        }

        match cube_distance_zero_or_one(candidate, cube, structure)
        {
            0 => cb_consensus_dist0(&mut output, candidate, cube, structure),
            1 => output.push(consensus(candidate, cube, structure)),
            _ => {}
        }
    }

    output
}

pub fn cb_consensus_dist0(
    output: &mut Cover,
    candidate: &Cube,
    cube: &Cube,
    structure: &CubeStructure,
)
{
    if cube_implies(candidate, cube, structure)
    {
        return;
    }

    let mut got_one = false;
    for variable in structure
        .variables()
        .iter()
        .copied()
        .skip(structure.binary_variable_count())
    {
        let candidate_minus_cube = variable
            .parts()
            .any(|part| candidate.contains(part) && !cube.contains(part));

        if candidate_minus_cube
        {
            output.push(merge_cube_with_intersection_on_variable(
                candidate,
                cube,
                variable,
            ));
            got_one = true;
        }
    }

    if !got_one && structure.binary_variable_count() > 0
    {
        output.push(intersection(candidate, cube, structure));
    }
}

fn cube_distance_zero_or_one(left: &Cube, right: &Cube, structure: &CubeStructure) -> usize
{
    let mut distance = 0;

    for variable in structure.variables()
    {
        let intersects = variable
            .parts()
            .any(|part| left.contains(part) && right.contains(part));

        if !intersects
        {
            distance += 1;
            if distance > 1
            {
                return 2;
            }
        }
    }

    distance
}

fn consensus(left: &Cube, right: &Cube, structure: &CubeStructure) -> Cube
{
    let mut output = Cube::empty();

    for variable in structure.variables()
    {
        let left_parts = left.variable_parts(*variable);
        let right_parts = right.variable_parts(*variable);
        let intersection = left_parts
            .intersection(&right_parts)
            .copied()
            .collect::<BTreeSet<_>>();

        let parts = if intersection.is_empty()
        {
            left_parts.union(&right_parts).copied().collect::<BTreeSet<_>>()
        }
        else
        {
            intersection
        };

        for part in parts
        {
            output.insert(part);
        }
    }

    output
}

fn merge_cube_with_intersection_on_variable(
    candidate: &Cube,
    cube: &Cube,
    variable: Variable,
) -> Cube
{
    let candidate_parts = candidate.variable_parts(variable);
    let cube_parts = cube.variable_parts(variable);
    let intersection = candidate_parts
        .intersection(&cube_parts)
        .copied()
        .collect::<BTreeSet<_>>();

    cube.with_variable_parts(variable, &intersection)
}

fn intersection(left: &Cube, right: &Cube, structure: &CubeStructure) -> Cube
{
    let mut output = Cube::empty();

    for variable in structure.variables()
    {
        let left_parts = left.variable_parts(*variable);
        let right_parts = right.variable_parts(*variable);

        for part in left_parts.intersection(&right_parts)
        {
            output.insert(*part);
        }
    }

    output
}

fn cube_implies(left: &Cube, right: &Cube, structure: &CubeStructure) -> bool
{
    structure.variables().iter().copied().all(|variable| {
        let left_parts = left.variable_parts(variable);
        let right_parts = right.variable_parts(variable);

        left_parts.is_subset(&right_parts)
    })
}

fn every_assignment_in_cube<P>(
    cube: &Cube,
    structure: &CubeStructure,
    variable_index: usize,
    assignment: &mut Vec<usize>,
    predicate: &mut P,
) -> bool
where
    P: FnMut(&[usize]) -> bool,
{
    if variable_index == structure.variable_count()
    {
        return predicate(assignment);
    }

    let variable = structure
        .variable(variable_index)
        .expect("valid variable index");
    for part in variable.parts()
    {
        if !cube.contains(part)
        {
            continue;
        }

        assignment.push(part);
        if !every_assignment_in_cube(cube, structure, variable_index + 1, assignment, predicate)
        {
            assignment.pop();
            return false;
        }
        assignment.pop();
    }

    true
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn structure() -> CubeStructure
    {
        CubeStructure::new(
            1,
            [
                Variable::new(0, 1),
                Variable::new(2, 3),
                Variable::new(4, 5),
            ],
        )
    }

    fn cube(parts: &[usize]) -> Cube
    {
        Cube::from_parts(parts.iter().copied())
    }

    fn relatively_essential_cube(parts: &[usize]) -> Cube
    {
        let mut cube = cube(parts);
        cube.set_relatively_essential(true);
        cube
    }

    fn cover(cubes: &[&[usize]]) -> Cover
    {
        Cover::new(cubes.iter().map(|parts| cube(parts)))
    }

    #[test]
    fn consensus_for_distance_one_uses_union_in_empty_variable()
    {
        let structure = structure();
        let left = cube(&[0, 2, 4]);
        let right = cube(&[1, 2, 4]);

        let result = consensus(&left, &right, &structure);

        assert_eq!(result, cube(&[0, 1, 2, 4]));
    }

    #[test]
    fn distance_zero_consensus_skips_candidate_implied_by_test_cube()
    {
        let structure = structure();
        let candidate = cube(&[0, 2, 4]);
        let test_cube = cube(&[0, 1, 2, 4]);
        let mut output = Cover::empty();

        cb_consensus_dist0(&mut output, &candidate, &test_cube, &structure);

        assert!(output.is_empty());
    }

    #[test]
    fn distance_zero_consensus_adds_each_multiple_valued_difference()
    {
        let structure = structure();
        let candidate = cube(&[0, 2, 3, 4, 5]);
        let test_cube = cube(&[0, 2, 4]);
        let mut output = Cover::empty();

        cb_consensus_dist0(&mut output, &candidate, &test_cube, &structure);

        assert_eq!(output, Cover::new([cube(&[0, 2, 4]), cube(&[0, 2, 4])]));
    }

    #[test]
    fn essential_cube_detects_private_care_assignment()
    {
        let structure = structure();
        let on_set = cover(&[&[0, 2, 4], &[1, 3, 4]]);
        let dont_care = Cover::empty();

        assert!(essen_cube(
            &on_set,
            &dont_care,
            &cube(&[0, 2, 4]),
            &structure
        ));
    }

    #[test]
    fn essential_cube_rejects_cube_covered_by_consensus_and_dont_care()
    {
        let structure = structure();
        let on_set = cover(&[&[0, 2, 4], &[0, 3, 4]]);
        let dont_care = Cover::empty();

        assert!(!essen_cube(
            &on_set,
            &dont_care,
            &cube(&[0, 2, 4]),
            &structure
        ));
    }

    #[test]
    fn essential_moves_only_candidate_cubes_with_required_flags()
    {
        let structure = structure();
        let mut nonessential = relatively_essential_cube(&[1, 3, 5]);
        nonessential.set_nonessential(true);
        let mut on_set = Cover::new([
            relatively_essential_cube(&[0, 2, 4]),
            cube(&[1, 3, 4]),
            nonessential,
        ]);
        let mut dont_care = Cover::empty();

        let result = essential(&mut on_set, &mut dont_care, &structure);

        assert_eq!(result, Cover::new([relatively_essential_cube(&[0, 2, 4])]));
        assert_eq!(on_set, Cover::new([cube(&[1, 3, 4]), {
            let mut cube = relatively_essential_cube(&[1, 3, 5]);
            cube.set_nonessential(true);
            cube
        }]));
        assert_eq!(
            dont_care,
            Cover::new([relatively_essential_cube(&[0, 2, 4])])
        );
    }
}
