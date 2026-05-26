//! Native Rust port of the Espresso expansion step.
//!
//! The expansion pass raises each non-prime ON-set cube into a prime implicant
//! while preserving orthogonality against the OFF-set. This module keeps the
//! Espresso cube model, but represents covers as owned Rust values and reports
//! invalid ON/OFF overlap as a typed error.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpandOptions {
    pub nonsparse: bool,
}

impl ExpandOptions {
    pub const fn all_variables() -> Self {
        Self { nonsparse: false }
    }

    pub const fn non_sparse_variables_only() -> Self {
        Self { nonsparse: true }
    }
}

impl Default for ExpandOptions {
    fn default() -> Self {
        Self::all_variables()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Variable {
    first_part: usize,
    last_part: usize,
    sparse: bool,
}

impl Variable {
    pub const fn new(first_part: usize, last_part: usize, sparse: bool) -> Self {
        Self {
            first_part,
            last_part,
            sparse,
        }
    }

    pub const fn first_part(self) -> usize {
        self.first_part
    }

    pub const fn last_part(self) -> usize {
        self.last_part
    }

    pub const fn is_sparse(self) -> bool {
        self.sparse
    }

    fn parts(self) -> impl Iterator<Item = usize> {
        self.first_part..=self.last_part
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure {
    variables: Vec<Variable>,
    part_count: usize,
}

impl CubeStructure {
    pub fn new(variables: impl IntoIterator<Item = Variable>) -> Result<Self, ExpandError> {
        let variables = variables.into_iter().collect::<Vec<_>>();
        let mut next_part = 0;

        for variable in &variables {
            if variable.first_part > variable.last_part {
                return Err(ExpandError::InvalidVariableRange {
                    first_part: variable.first_part,
                    last_part: variable.last_part,
                });
            }

            if variable.first_part != next_part {
                return Err(ExpandError::NonContiguousVariableParts {
                    expected_first_part: next_part,
                    actual_first_part: variable.first_part,
                });
            }

            next_part = variable.last_part + 1;
        }

        Ok(Self {
            variables,
            part_count: next_part,
        })
    }

    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    pub fn part_count(&self) -> usize {
        self.part_count
    }

    pub fn full_cube(&self) -> Cube {
        Cube::from_parts(0..self.part_count)
    }

    fn validate_cube(&self, cube: &Cube) -> Result<(), ExpandError> {
        if let Some(part) = cube.parts().find(|part| *part >= self.part_count) {
            return Err(ExpandError::PartOutOfRange {
                part,
                part_count: self.part_count,
            });
        }

        for variable in &self.variables {
            if cube.part_count_for(*variable) == 0 {
                return Err(ExpandError::EmptyVariable {
                    first_part: variable.first_part,
                    last_part: variable.last_part,
                });
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    parts: BTreeSet<usize>,
}

impl Cube {
    pub fn empty() -> Self {
        Self {
            parts: BTreeSet::new(),
        }
    }

    pub fn from_parts(parts: impl IntoIterator<Item = usize>) -> Self {
        Self {
            parts: parts.into_iter().collect(),
        }
    }

    pub fn parts(&self) -> impl Iterator<Item = usize> + '_ {
        self.parts.iter().copied()
    }

    pub fn contains(&self, part: usize) -> bool {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> bool {
        self.parts.insert(part)
    }

    pub fn remove(&mut self, part: usize) -> bool {
        self.parts.remove(&part)
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.parts.is_subset(&other.parts)
    }

    pub fn is_disjoint_from(&self, other: &Self) -> bool {
        self.parts.is_disjoint(&other.parts)
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            parts: self.parts.union(&other.parts).copied().collect(),
        }
    }

    pub fn difference(&self, other: &Self) -> Self {
        Self {
            parts: self.parts.difference(&other.parts).copied().collect(),
        }
    }

    pub fn part_count_for(&self, variable: Variable) -> usize {
        variable.parts().filter(|part| self.contains(*part)).count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Implicant {
    cube: Cube,
    prime: bool,
    covered: bool,
    nonessential: bool,
}

impl Implicant {
    pub fn new(cube: Cube) -> Self {
        Self {
            cube,
            prime: false,
            covered: false,
            nonessential: false,
        }
    }

    pub fn prime(cube: Cube) -> Self {
        Self {
            cube,
            prime: true,
            covered: false,
            nonessential: false,
        }
    }

    pub fn cube(&self) -> &Cube {
        &self.cube
    }

    pub fn is_prime(&self) -> bool {
        self.prime
    }

    pub fn is_covered(&self) -> bool {
        self.covered
    }

    pub fn is_nonessential(&self) -> bool {
        self.nonessential
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    implicants: Vec<Implicant>,
}

impl Cover {
    pub fn new(implicants: impl IntoIterator<Item = Implicant>) -> Self {
        Self {
            implicants: implicants.into_iter().collect(),
        }
    }

    pub fn from_cubes(cubes: impl IntoIterator<Item = Cube>) -> Self {
        Self::new(cubes.into_iter().map(Implicant::new))
    }

    pub fn empty() -> Self {
        Self {
            implicants: Vec::new(),
        }
    }

    pub fn implicants(&self) -> &[Implicant] {
        &self.implicants
    }

    pub fn cubes(&self) -> impl Iterator<Item = &Cube> {
        self.implicants.iter().map(Implicant::cube)
    }

    pub fn len(&self) -> usize {
        self.implicants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.implicants.is_empty()
    }

    pub fn push(&mut self, implicant: Implicant) {
        self.implicants.push(implicant);
    }

    fn validate(&self, structure: &CubeStructure) -> Result<(), ExpandError> {
        for implicant in &self.implicants {
            structure.validate_cube(&implicant.cube)?;
        }

        Ok(())
    }

    fn sort_for_expansion(&mut self) {
        let mut counts = Vec::new();
        for implicant in &self.implicants {
            for part in implicant.cube.parts() {
                if part >= counts.len() {
                    counts.resize(part + 1, 0usize);
                }

                counts[part] += 1;
            }
        }

        self.implicants.sort_by_key(|implicant| {
            implicant
                .cube
                .parts()
                .map(|part| counts.get(part).copied().unwrap_or(0))
                .sum::<usize>()
        });
    }

    fn retain_uncovered(&mut self) {
        self.implicants.retain(|implicant| !implicant.covered);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandReport {
    pub cover: Cover,
    pub expanded_cubes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpandError {
    InvalidVariableRange {
        first_part: usize,
        last_part: usize,
    },
    NonContiguousVariableParts {
        expected_first_part: usize,
        actual_first_part: usize,
    },
    PartOutOfRange {
        part: usize,
        part_count: usize,
    },
    EmptyVariable {
        first_part: usize,
        last_part: usize,
    },
    OnSetIntersectsOffSet,
}

impl fmt::Display for ExpandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVariableRange {
                first_part,
                last_part,
            } => write!(
                formatter,
                "invalid variable range {first_part}..={last_part}"
            ),
            Self::NonContiguousVariableParts {
                expected_first_part,
                actual_first_part,
            } => write!(
                formatter,
                "variable starts at part {actual_first_part}, expected {expected_first_part}"
            ),
            Self::PartOutOfRange { part, part_count } => {
                write!(formatter, "part {part} is outside 0..{part_count}")
            }
            Self::EmptyVariable {
                first_part,
                last_part,
            } => write!(
                formatter,
                "cube has no parts in variable range {first_part}..={last_part}"
            ),
            Self::OnSetIntersectsOffSet => {
                write!(formatter, "ON-set and OFF-set are not orthogonal")
            }
        }
    }
}

impl Error for ExpandError {}

pub type ExpandResult<T> = Result<T, ExpandError>;

pub fn expand(
    mut on_set: Cover,
    off_set: &Cover,
    structure: &CubeStructure,
    options: ExpandOptions,
) -> ExpandResult<ExpandReport> {
    on_set.validate(structure)?;
    off_set.validate(structure)?;
    on_set.sort_for_expansion();

    let mut init_lower = Cube::empty();
    if options.nonsparse {
        for variable in structure
            .variables()
            .iter()
            .copied()
            .filter(|variable| variable.is_sparse())
        {
            init_lower = init_lower.union(&Cube::from_parts(variable.parts()));
        }
    }

    for implicant in &mut on_set.implicants {
        implicant.covered = false;
        implicant.nonessential = false;
    }

    let mut expanded_cubes = 0;
    let mut index = 0;
    while index < on_set.implicants.len() {
        let should_expand = {
            let implicant = &on_set.implicants[index];
            !implicant.prime && !implicant.covered
        };

        if should_expand {
            let result = expand_one(index, &mut on_set, off_set, structure, &init_lower)?;
            let implicant = &mut on_set.implicants[index];
            implicant.cube = result.raise;
            implicant.prime = true;
            implicant.covered = false;
            implicant.nonessential =
                result.num_covered == 0 && implicant.cube != result.overexpanded_cube;
            expanded_cubes += 1;
        }

        index += 1;
    }

    on_set.retain_uncovered();

    Ok(ExpandReport {
        cover: on_set,
        expanded_cubes,
    })
}

pub fn all_primes(
    on_set: &Cover,
    off_set: &Cover,
    structure: &CubeStructure,
) -> ExpandResult<Cover> {
    on_set.validate(structure)?;
    off_set.validate(structure)?;

    let mut result = Cover::empty();
    for implicant in &on_set.implicants {
        if implicant.prime {
            result.push(implicant.clone());
            continue;
        }

        let mut raise = implicant.cube.clone();
        let mut freeset = structure.full_cube().difference(&raise);
        let mut bb_active = vec![true; off_set.len()];
        essen_parts(
            off_set,
            None,
            &mut raise,
            &mut freeset,
            &mut bb_active,
            None,
            structure,
        )?;

        for prime in find_all_primes(off_set, &bb_active, &raise, &freeset, structure)? {
            result.push(Implicant::prime(prime));
        }
    }

    Ok(result)
}

struct ExpandOneResult {
    raise: Cube,
    overexpanded_cube: Cube,
    num_covered: usize,
}

fn expand_one(
    cube_index: usize,
    on_set: &mut Cover,
    off_set: &Cover,
    structure: &CubeStructure,
    init_lower: &Cube,
) -> ExpandResult<ExpandOneResult> {
    on_set.implicants[cube_index].prime = true;

    let mut bb_active = vec![true; off_set.len()];
    let mut cc_active = on_set
        .implicants
        .iter()
        .map(|implicant| !implicant.covered && !implicant.prime)
        .collect::<Vec<_>>();

    let mut num_covered = 0;
    let mut super_cube = on_set.implicants[cube_index].cube.clone();
    let mut raise = on_set.implicants[cube_index].cube.clone();
    let mut freeset = structure.full_cube().difference(&raise);

    if !init_lower.is_empty() {
        freeset = freeset.difference(init_lower);
        elim_lowering(
            off_set,
            Some(on_set),
            &raise,
            &freeset,
            &mut bb_active,
            Some(&mut cc_active),
            structure,
        );
    }

    essen_parts(
        off_set,
        Some(on_set),
        &mut raise,
        &mut freeset,
        &mut bb_active,
        Some(&mut cc_active),
        structure,
    )?;
    let overexpanded_cube = raise.union(&freeset);

    if cc_active.iter().any(|active| *active) {
        select_feasible(
            off_set,
            on_set,
            &mut raise,
            &mut freeset,
            &mut super_cube,
            &mut num_covered,
            &mut bb_active,
            &mut cc_active,
            structure,
        )?;
    }

    while cc_active.iter().any(|active| *active) {
        let best_part = most_frequent(Some(on_set), Some(&cc_active), &freeset)
            .expect("active covering cubes imply a free part");
        raise.insert(best_part);
        freeset.remove(best_part);
        essen_parts(
            off_set,
            Some(on_set),
            &mut raise,
            &mut freeset,
            &mut bb_active,
            Some(&mut cc_active),
            structure,
        )?;
    }

    while bb_active.iter().any(|active| *active) {
        mincov(off_set, &mut raise, &mut freeset, &mut bb_active, structure)?;
    }

    raise = raise.union(&freeset);

    Ok(ExpandOneResult {
        raise,
        overexpanded_cube,
        num_covered,
    })
}

fn essen_parts(
    off_set: &Cover,
    mut on_set: Option<&mut Cover>,
    raise: &mut Cube,
    freeset: &mut Cube,
    bb_active: &mut [bool],
    cc_active: Option<&mut [bool]>,
    structure: &CubeStructure,
) -> ExpandResult<()> {
    let mut xlower = Cube::empty();

    for (index, off_cube) in off_set.cubes().enumerate() {
        if !bb_active[index] {
            continue;
        }

        match cube_distance_limited(off_cube, raise, structure) {
            0 => return Err(ExpandError::OnSetIntersectsOffSet),
            1 => {
                xlower = xlower.union(&force_lower(off_cube, raise, structure));
                bb_active[index] = false;
            }
            _ => {}
        }
    }

    if !xlower.is_empty() {
        *freeset = freeset.difference(&xlower);
        elim_lowering(
            off_set,
            on_set.as_deref_mut(),
            raise,
            freeset,
            bb_active,
            cc_active,
            structure,
        );
    }

    Ok(())
}

fn essen_raising(off_set: &Cover, raise: &mut Cube, freeset: &mut Cube, bb_active: &[bool]) {
    let mut blocked = Cube::empty();
    for (index, off_cube) in off_set.cubes().enumerate() {
        if bb_active[index] {
            blocked = blocked.union(off_cube);
        }
    }

    let xraise = freeset.difference(&blocked);
    *raise = raise.union(&xraise);
    *freeset = freeset.difference(&xraise);
}

fn elim_lowering(
    off_set: &Cover,
    on_set: Option<&mut Cover>,
    raise: &Cube,
    freeset: &Cube,
    bb_active: &mut [bool],
    cc_active: Option<&mut [bool]>,
    structure: &CubeStructure,
) {
    let overexpanded = raise.union(freeset);
    for (index, off_cube) in off_set.cubes().enumerate() {
        if bb_active[index] && cube_distance(off_cube, &overexpanded, structure) > 0 {
            bb_active[index] = false;
        }
    }

    if let (Some(on_set), Some(cc_active)) = (on_set, cc_active) {
        for (index, implicant) in on_set.implicants.iter().enumerate() {
            if cc_active[index] && !implicant.cube.is_subset_of(&overexpanded) {
                cc_active[index] = false;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn select_feasible(
    off_set: &Cover,
    on_set: &mut Cover,
    raise: &mut Cube,
    freeset: &mut Cube,
    super_cube: &mut Cube,
    num_covered: &mut usize,
    bb_active: &mut [bool],
    cc_active: &mut [bool],
    structure: &CubeStructure,
) -> ExpandResult<()> {
    let mut feasible = cc_active
        .iter()
        .enumerate()
        .filter_map(|(index, active)| active.then_some(index))
        .collect::<Vec<_>>();

    loop {
        essen_raising(off_set, raise, freeset, bb_active);

        let previous = feasible;
        feasible = Vec::new();
        let mut feasible_new_lower = Vec::new();

        for cube_index in previous {
            if !cc_active[cube_index] {
                continue;
            }

            let cube = &on_set.implicants[cube_index].cube;
            if cube.is_subset_of(raise) {
                *num_covered += 1;
                *super_cube = super_cube.union(cube);
                cc_active[cube_index] = false;
                on_set.implicants[cube_index].covered = true;
            } else if let Some(new_lower) =
                feasibly_covered(off_set, cube, raise, bb_active, structure)?
            {
                feasible.push(cube_index);
                feasible_new_lower.push(new_lower);
            }
        }

        if feasible.is_empty() {
            return Ok(());
        }

        let mut best_index = 0;
        let mut best_count = 0;
        let mut best_size = usize::MAX;

        for (candidate_position, cube_index) in feasible.iter().copied().enumerate() {
            let candidate = &on_set.implicants[cube_index].cube;
            let size = candidate.parts.intersection(&freeset.parts).count();
            let count = feasible
                .iter()
                .filter(|other_index| {
                    feasible_new_lower[candidate_position]
                        .is_disjoint_from(&on_set.implicants[**other_index].cube)
                })
                .count();

            if count > best_count || count == best_count && size < best_size {
                best_index = candidate_position;
                best_count = count;
                best_size = size;
            }
        }

        *raise = raise.union(&on_set.implicants[feasible[best_index]].cube);
        *freeset = freeset.difference(raise);
        essen_parts(
            off_set,
            Some(on_set),
            raise,
            freeset,
            bb_active,
            Some(cc_active),
            structure,
        )?;
    }
}

fn feasibly_covered(
    off_set: &Cover,
    cube: &Cube,
    raise: &Cube,
    bb_active: &[bool],
    structure: &CubeStructure,
) -> ExpandResult<Option<Cube>> {
    let combined = raise.union(cube);
    let mut new_lower = Cube::empty();

    for (index, off_cube) in off_set.cubes().enumerate() {
        if !bb_active[index] {
            continue;
        }

        match cube_distance_limited(off_cube, &combined, structure) {
            0 => return Ok(None),
            1 => new_lower = new_lower.union(&force_lower(off_cube, &combined, structure)),
            _ => {}
        }
    }

    Ok(Some(new_lower))
}

fn mincov(
    off_set: &Cover,
    raise: &mut Cube,
    freeset: &mut Cube,
    bb_active: &mut [bool],
    structure: &CubeStructure,
) -> ExpandResult<()> {
    let mut lowered_rows = Vec::new();
    for (index, off_cube) in off_set.cubes().enumerate() {
        if bb_active[index] {
            lowered_rows.push(force_lower(off_cube, raise, structure));
        }
    }

    if lowered_rows.is_empty() {
        bb_active.fill(false);
        return Ok(());
    }

    let expansion = unraveled_size(&lowered_rows, structure);
    if expansion <= 500 {
        let unraveled = unravel(&lowered_rows, structure);
        let xlower = minimum_hitting_set(&unraveled);
        *raise = raise.union(&freeset.difference(&xlower));
        *freeset = Cube::empty();
        bb_active.fill(false);
        return Ok(());
    }

    let Some(part) = most_frequent(None, None, freeset) else {
        bb_active.fill(false);
        return Ok(());
    };
    raise.insert(part);
    *freeset = freeset.difference(raise);
    essen_parts(off_set, None, raise, freeset, bb_active, None, structure)
}

fn find_all_primes(
    off_set: &Cover,
    bb_active: &[bool],
    raise: &Cube,
    freeset: &Cube,
    structure: &CubeStructure,
) -> ExpandResult<Vec<Cube>> {
    let mut lowered_rows = Vec::new();
    for (index, off_cube) in off_set.cubes().enumerate() {
        if bb_active[index] {
            lowered_rows.push(force_lower(off_cube, raise, structure));
        }
    }

    if lowered_rows.is_empty() {
        return Ok(vec![raise.union(freeset)]);
    }

    let unraveled = retain_maximal(unravel(&lowered_rows, structure));
    Ok(minimum_hitting_sets(&unraveled)
        .into_iter()
        .map(|lowered| raise.union(&freeset.difference(&lowered)))
        .collect())
}

fn most_frequent(on_set: Option<&Cover>, active: Option<&[bool]>, freeset: &Cube) -> Option<usize> {
    let mut counts = Vec::new();

    if let (Some(on_set), Some(active)) = (on_set, active) {
        for (index, implicant) in on_set.implicants.iter().enumerate() {
            if !active[index] {
                continue;
            }

            for part in implicant.cube.parts() {
                if part >= counts.len() {
                    counts.resize(part + 1, 0usize);
                }

                counts[part] += 1;
            }
        }
    }

    freeset
        .parts()
        .max_by_key(|part| (counts.get(*part).copied().unwrap_or(0), usize::MAX - *part))
}

fn cube_distance_limited(left: &Cube, right: &Cube, structure: &CubeStructure) -> usize {
    let mut distance = 0;
    for variable in structure.variables() {
        if variable
            .parts()
            .all(|part| !left.contains(part) || !right.contains(part))
        {
            distance += 1;
            if distance > 1 {
                return 2;
            }
        }
    }

    distance
}

fn cube_distance(left: &Cube, right: &Cube, structure: &CubeStructure) -> usize {
    structure
        .variables()
        .iter()
        .filter(|variable| {
            variable
                .parts()
                .all(|part| !left.contains(part) || !right.contains(part))
        })
        .count()
}

fn force_lower(off_cube: &Cube, raise: &Cube, structure: &CubeStructure) -> Cube {
    let mut lowered = Cube::empty();
    for variable in structure.variables() {
        let intersects = variable
            .parts()
            .any(|part| off_cube.contains(part) && raise.contains(part));
        if !intersects {
            lowered = lowered.union(&Cube::from_parts(
                variable.parts().filter(|part| off_cube.contains(*part)),
            ));
        }
    }

    lowered
}

fn unraveled_size(rows: &[Cube], structure: &CubeStructure) -> usize {
    let mut total = 0usize;
    for row in rows {
        let mut expansion = 1usize;
        for variable in structure.variables() {
            let count = row.part_count_for(*variable);
            if count > 1 {
                expansion = expansion.saturating_mul(count);
            }
        }

        total = total.saturating_add(expansion);
    }

    total
}

fn unravel(rows: &[Cube], structure: &CubeStructure) -> Vec<Cube> {
    let mut result = Vec::new();
    for row in rows {
        let mut partial = vec![Cube::empty()];

        for variable in structure.variables() {
            let selected = variable
                .parts()
                .filter(|part| row.contains(*part))
                .collect::<Vec<_>>();

            if selected.len() <= 1 {
                for cube in &mut partial {
                    for part in &selected {
                        cube.insert(*part);
                    }
                }
                continue;
            }

            let mut next = Vec::new();
            for cube in &partial {
                for part in &selected {
                    let mut expanded = cube.clone();
                    expanded.insert(*part);
                    next.push(expanded);
                }
            }
            partial = next;
        }

        result.append(&mut partial);
    }

    result
}

fn minimum_hitting_set(rows: &[Cube]) -> Cube {
    minimum_hitting_sets(rows)
        .into_iter()
        .next()
        .unwrap_or_else(Cube::empty)
}

fn minimum_hitting_sets(rows: &[Cube]) -> Vec<Cube> {
    let mut rows = retain_minimal(rows.to_vec());
    rows.retain(|row| !row.is_empty());

    if rows.is_empty() {
        return vec![Cube::empty()];
    }

    let universe = rows
        .iter()
        .flat_map(Cube::parts)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut best_len = usize::MAX;
    let mut best = Vec::new();
    search_hitting_sets(
        &rows,
        &universe,
        0,
        &mut Cube::empty(),
        &mut best_len,
        &mut best,
    );
    best.sort_by_key(|cube| cube.parts.clone());
    best
}

fn search_hitting_sets(
    rows: &[Cube],
    universe: &[usize],
    start: usize,
    current: &mut Cube,
    best_len: &mut usize,
    best: &mut Vec<Cube>,
) {
    if current.len() > *best_len {
        return;
    }

    if rows
        .iter()
        .all(|row| row.parts().any(|part| current.contains(part)))
    {
        match current.len().cmp(best_len) {
            std::cmp::Ordering::Less => {
                *best_len = current.len();
                best.clear();
                best.push(current.clone());
            }
            std::cmp::Ordering::Equal => best.push(current.clone()),
            std::cmp::Ordering::Greater => {}
        }
        return;
    }

    for index in start..universe.len() {
        current.insert(universe[index]);
        search_hitting_sets(rows, universe, index + 1, current, best_len, best);
        current.remove(universe[index]);
    }
}

fn retain_minimal(mut rows: Vec<Cube>) -> Vec<Cube> {
    rows.sort_by_key(|row| row.parts.clone());
    rows.dedup();
    let original = rows.clone();
    rows.retain(|row| {
        !original
            .iter()
            .any(|candidate| candidate != row && candidate.is_subset_of(row))
    });
    rows
}

fn retain_maximal(mut rows: Vec<Cube>) -> Vec<Cube> {
    rows.sort_by_key(|row| row.parts.clone());
    rows.dedup();
    let original = rows.clone();
    rows.retain(|row| {
        !original
            .iter()
            .any(|candidate| candidate != row && row.is_subset_of(candidate))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structure() -> CubeStructure {
        CubeStructure::new([
            Variable::new(0, 1, false),
            Variable::new(2, 3, false),
            Variable::new(4, 5, true),
        ])
        .unwrap()
    }

    fn cube(parts: &[usize]) -> Cube {
        Cube::from_parts(parts.iter().copied())
    }

    fn cover(cubes: &[&[usize]]) -> Cover {
        Cover::from_cubes(cubes.iter().map(|parts| cube(parts)))
    }

    fn cube_parts(cover: &Cover) -> Vec<BTreeSet<usize>> {
        cover.cubes().map(|cube| cube.parts().collect()).collect()
    }

    #[test]
    fn expands_single_cube_to_universe_when_off_set_is_empty() {
        let structure = structure();
        let on_set = cover(&[&[0, 2, 4]]);
        let off_set = Cover::empty();

        let report = expand(on_set, &off_set, &structure, ExpandOptions::all_variables()).unwrap();

        assert_eq!(report.expanded_cubes, 1);
        assert_eq!(cube_parts(&report.cover), vec![set(&[0, 1, 2, 3, 4, 5])]);
        assert!(report.cover.implicants()[0].is_prime());
    }

    #[test]
    fn expansion_stays_orthogonal_to_off_set() {
        let structure = structure();
        let on_set = cover(&[&[0, 2, 4]]);
        let off_set = cover(&[&[1, 2, 4]]);

        let report = expand(on_set, &off_set, &structure, ExpandOptions::all_variables()).unwrap();

        assert_eq!(cube_parts(&report.cover), vec![set(&[0, 2, 3, 4, 5])]);
    }

    #[test]
    fn expansion_marks_newly_covered_cubes_inactive() {
        let structure = structure();
        let on_set = cover(&[&[0, 2, 4], &[0, 3, 5]]);
        let off_set = cover(&[&[1, 2, 4]]);

        let report = expand(on_set, &off_set, &structure, ExpandOptions::all_variables()).unwrap();

        assert_eq!(report.cover.len(), 1);
        assert_eq!(cube_parts(&report.cover), vec![set(&[0, 2, 3, 4, 5])]);
    }

    #[test]
    fn nonsparse_mode_keeps_sparse_variables_lowered() {
        let structure = structure();
        let on_set = cover(&[&[0, 2, 4]]);
        let off_set = Cover::empty();

        let report = expand(
            on_set,
            &off_set,
            &structure,
            ExpandOptions::non_sparse_variables_only(),
        )
        .unwrap();

        assert_eq!(cube_parts(&report.cover), vec![set(&[0, 1, 2, 3, 4])]);
    }

    #[test]
    fn overlap_between_on_and_off_set_is_reported() {
        let structure = structure();
        let on_set = cover(&[&[0, 2, 4]]);
        let off_set = cover(&[&[0, 2, 4]]);

        let error =
            expand(on_set, &off_set, &structure, ExpandOptions::all_variables()).unwrap_err();

        assert_eq!(error, ExpandError::OnSetIntersectsOffSet);
    }

    #[test]
    fn all_primes_returns_prime_expansions_for_each_nonprime_cube() {
        let structure = structure();
        let on_set = cover(&[&[0, 2, 4]]);
        let off_set = cover(&[&[1, 2, 4]]);

        let primes = all_primes(&on_set, &off_set, &structure).unwrap();

        assert_eq!(cube_parts(&primes), vec![set(&[0, 2, 3, 4, 5])]);
        assert!(primes.implicants()[0].is_prime());
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("expand.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }

    fn set(parts: &[usize]) -> BTreeSet<usize> {
        parts.iter().copied().collect()
    }
}
