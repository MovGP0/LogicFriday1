use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VariablePair {
    first: usize,
    second: usize,
}

impl VariablePair {
    pub const fn new(first: usize, second: usize) -> Self {
        Self { first, second }
    }

    pub const fn first(self) -> usize {
        self.first
    }

    pub const fn second(self) -> usize {
        self.second
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pairing {
    pairs: Vec<VariablePair>,
}

impl Pairing {
    pub fn new(pairs: impl IntoIterator<Item = VariablePair>) -> Self {
        Self {
            pairs: pairs.into_iter().collect(),
        }
    }

    pub fn empty() -> Self {
        Self { pairs: Vec::new() }
    }

    pub fn pairs(&self) -> &[VariablePair] {
        &self.pairs
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub fn push(&mut self, pair: VariablePair) {
        self.pairs.push(pair);
    }
}

impl fmt::Display for Pairing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("pair is")?;
        for pair in &self.pairs {
            write!(formatter, " ({} {})", pair.first + 1, pair.second + 1)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure {
    binary_variable_count: usize,
    part_sizes: Vec<usize>,
    sparse: Vec<bool>,
}

impl CubeStructure {
    pub fn new(
        binary_variable_count: usize,
        part_sizes: Vec<usize>,
        sparse: Vec<bool>,
    ) -> PairResult<Self> {
        if binary_variable_count > part_sizes.len() {
            return Err(PairError::BinaryVariableCountOutOfRange {
                binary_variable_count,
                variable_count: part_sizes.len(),
            });
        }

        if sparse.len() != part_sizes.len() {
            return Err(PairError::SparseCountMismatch {
                expected: part_sizes.len(),
                actual: sparse.len(),
            });
        }

        for (index, part_size) in part_sizes.iter().copied().enumerate() {
            if part_size == 0 {
                return Err(PairError::EmptyVariable { variable: index });
            }

            if index < binary_variable_count && part_size != 2 {
                return Err(PairError::BinaryVariableNotTwoValued {
                    variable: index,
                    part_size,
                });
            }
        }

        Ok(Self {
            binary_variable_count,
            part_sizes,
            sparse,
        })
    }

    pub fn binary(binary_variable_count: usize) -> Self {
        Self {
            binary_variable_count,
            part_sizes: vec![2; binary_variable_count],
            sparse: vec![false; binary_variable_count],
        }
    }

    pub fn binary_variable_count(&self) -> usize {
        self.binary_variable_count
    }

    pub fn variable_count(&self) -> usize {
        self.part_sizes.len()
    }

    pub fn part_sizes(&self) -> &[usize] {
        &self.part_sizes
    }

    pub fn sparse(&self) -> &[bool] {
        &self.sparse
    }

    pub fn set_sparse(&mut self, variable: usize, sparse: bool) -> PairResult<()> {
        let Some(slot) = self.sparse.get_mut(variable) else {
            return Err(PairError::VariableOutOfRange {
                variable,
                binary_variable_count: self.binary_variable_count,
            });
        };

        *slot = sparse;
        Ok(())
    }

    pub fn first_part(&self, variable: usize) -> PairResult<usize> {
        if variable >= self.variable_count() {
            return Err(PairError::VariableOutOfRange {
                variable,
                binary_variable_count: self.variable_count(),
            });
        }

        Ok(self.part_sizes[..variable].iter().sum())
    }

    pub fn part_count(&self) -> usize {
        self.part_sizes.iter().sum()
    }

    fn output_insert_column(&self) -> usize {
        self.variable_count()
            .checked_sub(1)
            .map(|variable| self.part_sizes[..variable].iter().sum())
            .unwrap_or_else(|| self.part_count())
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

    pub fn contains(&self, part: usize) -> bool {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize) -> bool {
        self.parts.insert(part)
    }

    pub fn parts(&self) -> &BTreeSet<usize> {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    part_count: usize,
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(part_count: usize, cubes: Vec<Cube>) -> PairResult<Self> {
        for cube in &cubes {
            if let Some(part) = cube.parts().iter().find(|part| **part >= part_count) {
                return Err(PairError::PartOutOfRange {
                    part: *part,
                    part_count,
                });
            }
        }

        Ok(Self { part_count, cubes })
    }

    pub fn from_rows<I, R>(part_count: usize, rows: I) -> PairResult<Self>
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = usize>,
    {
        Self::new(part_count, rows.into_iter().map(Cube::from_parts).collect())
    }

    pub fn part_count(&self) -> usize {
        self.part_count
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    fn map_cubes(
        &self,
        new_part_count: usize,
        mut map_cube: impl FnMut(&Cube) -> Cube,
    ) -> PairResult<Self> {
        Self::new(
            new_part_count,
            self.cubes.iter().map(|cube| map_cube(cube)).collect(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pla {
    on_set: Cover,
    off_set: Cover,
    dont_care_set: Cover,
    labels: Option<Vec<String>>,
    structure: CubeStructure,
}

impl Pla {
    pub fn new(
        on_set: Cover,
        off_set: Cover,
        dont_care_set: Cover,
        labels: Option<Vec<String>>,
        structure: CubeStructure,
    ) -> PairResult<Self> {
        let part_count = structure.part_count();
        for actual in [
            on_set.part_count(),
            off_set.part_count(),
            dont_care_set.part_count(),
        ] {
            if actual != part_count {
                return Err(PairError::CoverPartCountMismatch {
                    expected: part_count,
                    actual,
                });
            }
        }

        if let Some(labels) = &labels {
            if labels.len() != part_count {
                return Err(PairError::LabelCountMismatch {
                    expected: part_count,
                    actual: labels.len(),
                });
            }
        }

        Ok(Self {
            on_set,
            off_set,
            dont_care_set,
            labels,
            structure,
        })
    }

    pub fn on_set(&self) -> &Cover {
        &self.on_set
    }

    pub fn off_set(&self) -> &Cover {
        &self.off_set
    }

    pub fn dont_care_set(&self) -> &Cover {
        &self.dont_care_set
    }

    pub fn labels(&self) -> Option<&[String]> {
        self.labels.as_deref()
    }

    pub fn structure(&self) -> &CubeStructure {
        &self.structure
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PairError {
    BinaryVariableCountOutOfRange {
        binary_variable_count: usize,
        variable_count: usize,
    },
    BinaryVariableNotTwoValued {
        variable: usize,
        part_size: usize,
    },
    CoverPartCountMismatch {
        expected: usize,
        actual: usize,
    },
    DuplicatePairedVariable {
        variable: usize,
    },
    EmptyVariable {
        variable: usize,
    },
    LabelCountMismatch {
        expected: usize,
        actual: usize,
    },
    PartOutOfRange {
        part: usize,
        part_count: usize,
    },
    PairMatrixNotSquare {
        rows: usize,
        columns: usize,
    },
    SelfPair {
        variable: usize,
    },
    SparseCountMismatch {
        expected: usize,
        actual: usize,
    },
    VariableOutOfRange {
        variable: usize,
        binary_variable_count: usize,
    },
}

impl fmt::Display for PairError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BinaryVariableCountOutOfRange {
                binary_variable_count,
                variable_count,
            } => write!(
                formatter,
                "{binary_variable_count} binary variables exceeds {variable_count} total variables"
            ),
            Self::BinaryVariableNotTwoValued {
                variable,
                part_size,
            } => write!(
                formatter,
                "binary variable {variable} has {part_size} parts instead of 2"
            ),
            Self::CoverPartCountMismatch { expected, actual } => {
                write!(formatter, "cover has {actual} parts; expected {expected}")
            }
            Self::DuplicatePairedVariable { variable } => {
                write!(formatter, "binary variable {variable} appears in more than one pair")
            }
            Self::EmptyVariable { variable } => write!(formatter, "variable {variable} has no parts"),
            Self::LabelCountMismatch { expected, actual } => {
                write!(formatter, "{actual} labels supplied for {expected} parts")
            }
            Self::PartOutOfRange { part, part_count } => {
                write!(formatter, "part {part} is outside 0..{part_count}")
            }
            Self::PairMatrixNotSquare { rows, columns } => {
                write!(formatter, "pair cost matrix is {rows}x{columns}, expected square")
            }
            Self::SelfPair { variable } => write!(formatter, "variable {variable} is paired with itself"),
            Self::SparseCountMismatch { expected, actual } => {
                write!(formatter, "{actual} sparse flags supplied for {expected} variables")
            }
            Self::VariableOutOfRange {
                variable,
                binary_variable_count,
            } => write!(
                formatter,
                "paired variable {variable} is outside binary variable range 0..{binary_variable_count}"
            ),
        }
    }
}

impl Error for PairError {}

pub type PairResult<T> = Result<T, PairError>;

pub fn set_pair(pla: Pla, pairing: &Pairing) -> PairResult<Pla> {
    let paired = paired_variables(pairing, pla.structure())?;
    let transformed_structure = paired_structure(pla.structure(), pairing, &paired);
    let transformed_part_count = transformed_structure.part_count();

    let on_set = apply_pairing_to_cover(pla.on_set(), pla.structure(), pairing, &paired)?;
    let off_set = apply_pairing_to_cover(pla.off_set(), pla.structure(), pairing, &paired)?;
    let dont_care_set =
        apply_pairing_to_cover(pla.dont_care_set(), pla.structure(), pairing, &paired)?;
    let labels = match pla.labels() {
        Some(labels) => Some(paired_labels(labels, pla.structure(), pairing, &paired)?),
        None => None,
    };

    Pla::new(
        Cover::new(transformed_part_count, on_set.cubes().to_vec())?,
        Cover::new(transformed_part_count, off_set.cubes().to_vec())?,
        Cover::new(transformed_part_count, dont_care_set.cubes().to_vec())?,
        labels,
        transformed_structure,
    )
}

pub fn apply_pairing_to_cover(
    cover: &Cover,
    structure: &CubeStructure,
    pairing: &Pairing,
    paired: &[bool],
) -> PairResult<Cover> {
    let with_pair_columns = pair_variables_in_cover(cover, structure, pairing)?;
    delete_paired_binary_variables(&with_pair_columns, structure, paired)
}

pub fn pair_variables_in_cover(
    cover: &Cover,
    structure: &CubeStructure,
    pairing: &Pairing,
) -> PairResult<Cover> {
    let paired = paired_variables(pairing, structure)?;
    let insert_column = structure.output_insert_column();
    let inserted_width = pairing.len() * 4;
    let new_part_count = cover.part_count() + inserted_width;

    cover
        .map_cubes(new_part_count, |cube| {
            let mut result = Cube::from_parts(cube.parts().iter().map(|part| {
                if *part >= insert_column {
                    *part + inserted_width
                } else {
                    *part
                }
            }));

            for (pair_index, pair) in pairing.pairs().iter().copied().enumerate() {
                let first_part = structure.first_part(pair.first()).expect("validated pair");
                let second_part = structure.first_part(pair.second()).expect("validated pair");
                let second_zero = cube.contains(second_part);
                let second_one = cube.contains(second_part + 1);
                let value = insert_column + pair_index * 4;

                if cube.contains(first_part) {
                    if second_zero {
                        result.insert(value + 3);
                    }

                    if second_one {
                        result.insert(value + 2);
                    }
                }

                if cube.contains(first_part + 1) {
                    if second_zero {
                        result.insert(value + 1);
                    }

                    if second_one {
                        result.insert(value);
                    }
                }
            }

            result
        })
        .and_then(|cover| {
            if paired.len() == structure.binary_variable_count() {
                Ok(cover)
            } else {
                Err(PairError::SparseCountMismatch {
                    expected: structure.binary_variable_count(),
                    actual: paired.len(),
                })
            }
        })
}

pub fn delete_paired_binary_variables(
    cover: &Cover,
    structure: &CubeStructure,
    paired: &[bool],
) -> PairResult<Cover> {
    if paired.len() != structure.binary_variable_count() {
        return Err(PairError::SparseCountMismatch {
            expected: structure.binary_variable_count(),
            actual: paired.len(),
        });
    }

    let mut removed_before = vec![0usize; cover.part_count() + 1];
    let mut removed = 0;
    for (part, slot) in removed_before
        .iter_mut()
        .enumerate()
        .take(cover.part_count())
    {
        *slot = removed;
        if is_paired_binary_part(part, structure, paired) {
            removed += 1;
        }
    }
    removed_before[cover.part_count()] = removed;

    cover.map_cubes(cover.part_count() - removed, |cube| {
        Cube::from_parts(cube.parts().iter().filter_map(|part| {
            if is_paired_binary_part(*part, structure, paired) {
                None
            } else {
                Some(*part - removed_before[*part])
            }
        }))
    })
}

pub fn greedy_best_cost(costs: &[Vec<isize>]) -> PairResult<(Pairing, isize)> {
    validate_square_costs(costs)?;

    let mut candidates = BTreeSet::from_iter(0..costs.len());
    let mut pairing = Pairing::empty();
    let mut total = 0;

    while candidates.len() >= 2 {
        let mut best = None;
        for &first in &candidates {
            for &second in candidates.range(first + 1..) {
                let cost = costs[first][second];
                if best
                    .map(|(_, _, best_cost)| cost > best_cost)
                    .unwrap_or(true)
                {
                    best = Some((first, second, cost));
                }
            }
        }

        let Some((first, second, cost)) = best else {
            break;
        };

        pairing.push(VariablePair::new(first, second));
        candidates.remove(&first);
        candidates.remove(&second);
        total += cost;
    }

    Ok((pairing, total))
}

pub fn pair_best_cost(costs: &[Vec<isize>]) -> PairResult<(Pairing, isize)> {
    validate_square_costs(costs)?;

    let mut best_pairing = Pairing::empty();
    let mut best_cost = isize::MIN;
    let mut current = Pairing::empty();
    let mut candidates = BTreeSet::from_iter(0..costs.len());

    generate_all_pairs_inner(&mut current, &mut candidates, costs.len(), &mut |pairing| {
        let cost = pairing
            .pairs()
            .iter()
            .map(|pair| costs[pair.first()][pair.second()])
            .sum();
        if cost > best_cost {
            best_cost = cost;
            best_pairing = pairing.clone();
        }
    });

    Ok((best_pairing, best_cost))
}

pub fn generate_all_pairs(variable_count: usize) -> Vec<Pairing> {
    let mut pairings = Vec::new();
    let mut current = Pairing::empty();
    let mut candidates = BTreeSet::from_iter(0..variable_count);

    generate_all_pairs_inner(
        &mut current,
        &mut candidates,
        variable_count,
        &mut |pairing| {
            pairings.push(pairing.clone());
        },
    );

    pairings
}

fn paired_variables(pairing: &Pairing, structure: &CubeStructure) -> PairResult<Vec<bool>> {
    let mut paired = vec![false; structure.binary_variable_count()];
    for pair in pairing.pairs().iter().copied() {
        validate_pair_variable(pair.first(), structure)?;
        validate_pair_variable(pair.second(), structure)?;

        if pair.first() == pair.second() {
            return Err(PairError::SelfPair {
                variable: pair.first(),
            });
        }

        for variable in [pair.first(), pair.second()] {
            if paired[variable] {
                return Err(PairError::DuplicatePairedVariable { variable });
            }

            paired[variable] = true;
        }
    }

    Ok(paired)
}

fn validate_pair_variable(variable: usize, structure: &CubeStructure) -> PairResult<()> {
    if variable >= structure.binary_variable_count() {
        return Err(PairError::VariableOutOfRange {
            variable,
            binary_variable_count: structure.binary_variable_count(),
        });
    }

    Ok(())
}

fn paired_structure(
    structure: &CubeStructure,
    pairing: &Pairing,
    paired: &[bool],
) -> CubeStructure {
    let unpaired_binary_count = paired.iter().filter(|is_paired| !**is_paired).count();
    let mut part_sizes = Vec::with_capacity(
        unpaired_binary_count + pairing.len() + structure.variable_count() - paired.len(),
    );
    let mut sparse = Vec::with_capacity(part_sizes.capacity());

    for &is_paired in paired {
        if !is_paired {
            part_sizes.push(2);
            sparse.push(false);
        }
    }

    for _ in pairing.pairs() {
        part_sizes.push(4);
        sparse.push(false);
    }

    for variable in structure.binary_variable_count()..structure.variable_count() {
        part_sizes.push(structure.part_sizes()[variable]);
        sparse.push(structure.sparse()[variable]);
    }

    CubeStructure {
        binary_variable_count: unpaired_binary_count,
        part_sizes,
        sparse,
    }
}

fn paired_labels(
    labels: &[String],
    structure: &CubeStructure,
    pairing: &Pairing,
    paired: &[bool],
) -> PairResult<Vec<String>> {
    if labels.len() != structure.part_count() {
        return Err(PairError::LabelCountMismatch {
            expected: structure.part_count(),
            actual: labels.len(),
        });
    }

    let mut result = Vec::with_capacity(
        labels.len() - paired.iter().filter(|value| **value).count() * 2 + pairing.len() * 4,
    );

    for (variable, is_paired) in paired.iter().copied().enumerate() {
        if !is_paired {
            let start = structure.first_part(variable)?;
            result.push(labels[start].clone());
            result.push(labels[start + 1].clone());
        }
    }

    for pair in pairing.pairs().iter().copied() {
        let first = structure.first_part(pair.first())?;
        let second = structure.first_part(pair.second())?;
        let first_zero = &labels[first];
        let first_one = &labels[first + 1];
        let second_zero = &labels[second];
        let second_one = &labels[second + 1];
        result.push(format!("{first_zero}+{second_zero}"));
        result.push(format!("{first_zero}+{second_one}"));
        result.push(format!("{first_one}+{second_zero}"));
        result.push(format!("{first_one}+{second_one}"));
    }

    let mv_start = structure.first_part(structure.binary_variable_count())?;
    result.extend(labels[mv_start..].iter().cloned());
    Ok(result)
}

fn is_paired_binary_part(part: usize, structure: &CubeStructure, paired: &[bool]) -> bool {
    let mut first_part = 0;
    for (variable, part_size) in structure.part_sizes().iter().copied().enumerate() {
        if part < first_part + part_size {
            return variable < paired.len() && paired[variable];
        }

        first_part += part_size;
    }

    false
}

fn validate_square_costs(costs: &[Vec<isize>]) -> PairResult<()> {
    for row in costs {
        if row.len() != costs.len() {
            return Err(PairError::PairMatrixNotSquare {
                rows: costs.len(),
                columns: row.len(),
            });
        }
    }

    Ok(())
}

fn generate_all_pairs_inner(
    current: &mut Pairing,
    candidates: &mut BTreeSet<usize>,
    variable_count: usize,
    action: &mut impl FnMut(&Pairing),
) {
    if candidates.len() < 2 {
        action(current);
        return;
    }

    let first = (0..variable_count)
        .find(|variable| candidates.contains(variable))
        .expect("at least one candidate");
    let partners = candidates.range(first + 1..).copied().collect::<Vec<_>>();

    for second in partners {
        candidates.remove(&first);
        candidates.remove(&second);
        current.push(VariablePair::new(first, second));

        generate_all_pairs_inner(current, candidates, variable_count, action);

        current.pairs.pop();
        candidates.insert(first);
        candidates.insert(second);
    }

    if candidates.len() % 2 == 1 {
        candidates.remove(&first);
        generate_all_pairs_inner(current, candidates, variable_count, action);
        candidates.insert(first);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cover(part_count: usize, rows: &[&[usize]]) -> Cover {
        Cover::from_rows(part_count, rows.iter().map(|row| row.iter().copied())).unwrap()
    }

    fn labels(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn pair_variables_inserts_legacy_ordered_pair_columns() {
        let structure = CubeStructure::binary(3);
        let input = cover(6, &[&[0, 2, 4], &[1, 3, 5], &[0, 3]]);
        let pairing = Pairing::new([VariablePair::new(0, 1)]);

        let result = pair_variables_in_cover(&input, &structure, &pairing).unwrap();

        assert_eq!(
            result,
            cover(10, &[&[0, 2, 7, 8], &[1, 3, 4, 9], &[0, 3, 6]])
        );
    }

    #[test]
    fn delete_paired_binary_variables_removes_runs_with_minimal_shifts() {
        let structure = CubeStructure::binary(4);
        let input = cover(8, &[&[0, 1, 4, 6], &[2, 3, 5, 7]]);
        let paired = vec![true, false, true, false];

        let result = delete_paired_binary_variables(&input, &structure, &paired).unwrap();

        assert_eq!(result, cover(4, &[&[2], &[0, 1, 3]]));
    }

    #[test]
    fn set_pair_updates_covers_structure_and_labels() {
        let structure =
            CubeStructure::new(3, vec![2, 2, 2, 3], vec![false, true, false, true]).unwrap();
        let on_set = cover(9, &[&[0, 2, 6], &[1, 3, 7]]);
        let off_set = cover(9, &[&[4, 6], &[5, 8]]);
        let dont_care_set = cover(9, &[&[0, 3, 6]]);
        let pla = Pla::new(
            on_set,
            off_set,
            dont_care_set,
            Some(labels(&[
                "a0", "a1", "b0", "b1", "c0", "c1", "out0", "out1", "out2",
            ])),
            structure,
        )
        .unwrap();
        let pairing = Pairing::new([VariablePair::new(0, 1)]);

        let result = set_pair(pla, &pairing).unwrap();

        assert_eq!(result.structure().binary_variable_count(), 1);
        assert_eq!(result.structure().part_sizes(), &[2, 4, 3]);
        assert_eq!(result.structure().sparse(), &[false, false, true]);
        assert_eq!(result.on_set(), &cover(9, &[&[5, 6], &[2, 7]]));
        assert_eq!(
            result.labels().unwrap(),
            &labels(&["c0", "c1", "a0+b0", "a0+b1", "a1+b0", "a1+b1", "out0", "out1", "out2",])
        );
    }

    #[test]
    fn set_pair_rejects_non_binary_pair_targets() {
        let structure = CubeStructure::new(1, vec![2, 3], vec![false, false]).unwrap();
        let pla = Pla::new(
            cover(5, &[&[0, 2]]),
            cover(5, &[]),
            cover(5, &[]),
            None,
            structure,
        )
        .unwrap();

        assert_eq!(
            set_pair(pla, &Pairing::new([VariablePair::new(0, 1)])).unwrap_err(),
            PairError::VariableOutOfRange {
                variable: 1,
                binary_variable_count: 1,
            }
        );
    }

    #[test]
    fn set_pair_rejects_overlapping_pairs() {
        let structure = CubeStructure::binary(3);
        let pla = Pla::new(cover(6, &[]), cover(6, &[]), cover(6, &[]), None, structure).unwrap();

        assert_eq!(
            set_pair(
                pla,
                &Pairing::new([VariablePair::new(0, 1), VariablePair::new(1, 2)])
            )
            .unwrap_err(),
            PairError::DuplicatePairedVariable { variable: 1 }
        );
    }

    #[test]
    fn generate_all_pairs_matches_exhaustive_pairing_counts() {
        assert_eq!(generate_all_pairs(1), vec![Pairing::empty()]);
        assert_eq!(generate_all_pairs(2).len(), 1);
        assert_eq!(generate_all_pairs(4).len(), 3);
        assert_eq!(generate_all_pairs(6).len(), 15);
        assert_eq!(generate_all_pairs(5).len(), 15);
    }

    #[test]
    fn pair_best_cost_selects_maximum_total_cost() {
        let costs = vec![
            vec![0, 7, 1, 1],
            vec![0, 0, 4, 4],
            vec![0, 0, 0, 9],
            vec![0, 0, 0, 0],
        ];

        let (pairing, cost) = pair_best_cost(&costs).unwrap();

        assert_eq!(cost, 16);
        assert_eq!(
            pairing,
            Pairing::new([VariablePair::new(0, 1), VariablePair::new(2, 3)])
        );
    }

    #[test]
    fn greedy_best_cost_takes_best_available_disjoint_edges() {
        let costs = vec![
            vec![0, 9, 8, 1],
            vec![0, 0, 7, 2],
            vec![0, 0, 0, 6],
            vec![0, 0, 0, 0],
        ];

        let (pairing, cost) = greedy_best_cost(&costs).unwrap();

        assert_eq!(cost, 15);
        assert_eq!(
            pairing,
            Pairing::new([VariablePair::new(0, 1), VariablePair::new(2, 3)])
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("pair.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
    }
}
