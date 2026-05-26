//! Native Rust irredundant-cover utilities for Espresso-style Boolean covers.
//!
//! The legacy routine partitions an ON cover into relatively essential,
//! totally redundant, and partially redundant cubes, then solves a covering
//! table for the partially redundant cubes. This port keeps the same behavior
//! on owned Rust values and returns the selected cube indices instead of
//! mutating Espresso set flags.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal
{
    Zero,
    One,
    DontCare,
}

impl Literal
{
    fn conflicts_with(self, other: Self) -> bool
    {
        matches!(
            (self, other),
            (Self::Zero, Self::One) | (Self::One, Self::Zero)
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube
{
    literals: Vec<Literal>,
}

impl Cube
{
    pub fn new(literals: Vec<Literal>) -> Self
    {
        Self { literals }
    }

    pub fn universe(width: usize) -> Self
    {
        Self {
            literals: vec![Literal::DontCare; width],
        }
    }

    pub fn width(&self) -> usize
    {
        self.literals.len()
    }

    pub fn literals(&self) -> &[Literal]
    {
        &self.literals
    }

    pub fn contains_cube(&self, other: &Self) -> bool
    {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| *left == Literal::DontCare || left == right)
    }

    pub fn intersects(&self, other: &Self) -> bool
    {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| !left.conflicts_with(*right))
    }

    pub fn contains_assignment(&self, assignment: usize) -> bool
    {
        self.literals.iter().enumerate().all(|(index, literal)| {
            let value = ((assignment >> index) & 1) != 0;
            match literal
            {
                Literal::Zero => !value,
                Literal::One => value,
                Literal::DontCare => true,
            }
        })
    }

    fn split_on(&self, variable: usize) -> [Self; 2]
    {
        let mut zero = self.clone();
        let mut one = self.clone();
        zero.literals[variable] = Literal::Zero;
        one.literals[variable] = Literal::One;
        [zero, one]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover
{
    width: usize,
    cubes: Vec<Cube>,
}

impl Cover
{
    pub fn new(width: usize, cubes: Vec<Cube>) -> IrredundantResult<Self>
    {
        for cube in &cubes
        {
            if cube.width() != width
            {
                return Err(IrredundantError::CubeWidth {
                    expected: width,
                    actual: cube.width(),
                });
            }
        }

        Ok(Self { width, cubes })
    }

    pub fn empty(width: usize) -> Self
    {
        Self {
            width,
            cubes: Vec::new(),
        }
    }

    pub fn width(&self) -> usize
    {
        self.width
    }

    pub fn cubes(&self) -> &[Cube]
    {
        &self.cubes
    }

    pub fn cube_count(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn evaluates(&self, assignment: usize) -> bool
    {
        self.cubes
            .iter()
            .any(|cube| cube.contains_assignment(assignment))
    }

    fn from_indices(&self, indices: &BTreeSet<usize>) -> Self
    {
        Self {
            width: self.width,
            cubes: indices
                .iter()
                .filter_map(|index| self.cubes.get(*index))
                .cloned()
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrredundantSplit
{
    relatively_essential: BTreeSet<usize>,
    totally_redundant: BTreeSet<usize>,
    partially_redundant: BTreeSet<usize>,
}

impl IrredundantSplit
{
    pub fn relatively_essential(&self) -> &BTreeSet<usize>
    {
        &self.relatively_essential
    }

    pub fn totally_redundant(&self) -> &BTreeSet<usize>
    {
        &self.totally_redundant
    }

    pub fn partially_redundant(&self) -> &BTreeSet<usize>
    {
        &self.partially_redundant
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrredundantReport
{
    split: IrredundantSplit,
    selected_partials: BTreeSet<usize>,
    covering_table: Vec<BTreeSet<usize>>,
}

impl IrredundantReport
{
    pub fn split(&self) -> &IrredundantSplit
    {
        &self.split
    }

    pub fn selected_partials(&self) -> &BTreeSet<usize>
    {
        &self.selected_partials
    }

    pub fn covering_table(&self) -> &[BTreeSet<usize>]
    {
        &self.covering_table
    }

    pub fn active_indices(&self) -> BTreeSet<usize>
    {
        self.split
            .relatively_essential
            .union(&self.selected_partials)
            .copied()
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IrredundantError
{
    CubeWidth { expected: usize, actual: usize },
    CoverWidthMismatch { on_set: usize, dont_care: usize },
    UncoverableResidualCube { cube: Cube },
}

impl fmt::Display for IrredundantError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::CubeWidth { expected, actual } => {
                write!(formatter, "cube has width {actual}; expected {expected}")
            }
            Self::CoverWidthMismatch { on_set, dont_care } => {
                write!(formatter, "cover widths differ: ON={on_set}, DC={dont_care}")
            }
            Self::UncoverableResidualCube { cube } => {
                write!(formatter, "residual cube is not covered by any partial cube: {cube:?}")
            }
        }
    }
}

impl Error for IrredundantError {}

pub type IrredundantResult<T> = Result<T, IrredundantError>;

pub fn irredundant(on_set: &Cover, dont_care: &Cover) -> IrredundantResult<Cover>
{
    let report = mark_irredundant(on_set, dont_care)?;
    Ok(on_set.from_indices(&report.active_indices()))
}

pub fn mark_irredundant(on_set: &Cover, dont_care: &Cover) -> IrredundantResult<IrredundantReport>
{
    ensure_same_width(on_set, dont_care)?;

    let split = irred_split_cover(on_set, dont_care)?;
    let table = irred_derive_table(on_set, dont_care, &split)?;
    let selected_partials = minimum_cover(&table);

    Ok(IrredundantReport {
        split,
        selected_partials,
        covering_table: table,
    })
}

pub fn irred_split_cover(on_set: &Cover, dont_care: &Cover) -> IrredundantResult<IrredundantSplit>
{
    ensure_same_width(on_set, dont_care)?;

    let mut relatively_essential = BTreeSet::new();
    let mut redundant = BTreeSet::new();

    for (index, cube) in on_set.cubes().iter().enumerate()
    {
        if cube_is_covered_except(on_set, dont_care, cube, Some(index))?
        {
            redundant.insert(index);
        }
        else
        {
            relatively_essential.insert(index);
        }
    }

    let fixed_cover = cover_from_index_sets(on_set, dont_care, &relatively_essential, None);
    let mut totally_redundant = BTreeSet::new();
    let mut partially_redundant = BTreeSet::new();

    for index in redundant
    {
        let cube = &on_set.cubes()[index];
        if cube_is_covered_by(cube, &fixed_cover)
        {
            totally_redundant.insert(index);
        }
        else
        {
            partially_redundant.insert(index);
        }
    }

    Ok(IrredundantSplit {
        relatively_essential,
        totally_redundant,
        partially_redundant,
    })
}

pub fn irred_derive_table(
    on_set: &Cover,
    dont_care: &Cover,
    split: &IrredundantSplit,
) -> IrredundantResult<Vec<BTreeSet<usize>>>
{
    ensure_same_width(on_set, dont_care)?;

    let fixed_cover = cover_from_index_sets(on_set, dont_care, split.relatively_essential(), None);
    let partial_indices = split.partially_redundant().iter().copied().collect::<Vec<_>>();
    let partial_cubes = partial_indices
        .iter()
        .map(|index| (*index, on_set.cubes()[*index].clone()))
        .collect::<Vec<_>>();
    let mut rows = Vec::new();

    for current_index in &partial_indices
    {
        let current = &on_set.cubes()[*current_index];
        for residual in subtract_cover(current.clone(), fixed_cover.cubes())
        {
            derive_rows_for_fragment(residual, &partial_cubes, &mut rows)?;
        }
    }

    rows.sort();
    rows.dedup();
    Ok(rows)
}

pub fn cube_is_covered(cover: &Cover, cube: &Cube) -> IrredundantResult<bool>
{
    if cover.width() != cube.width()
    {
        return Err(IrredundantError::CubeWidth {
            expected: cover.width(),
            actual: cube.width(),
        });
    }

    Ok(cube_is_covered_by(cube, cover))
}

pub fn tautology(cover: &Cover) -> bool
{
    cube_is_covered_by(&Cube::universe(cover.width()), cover)
}

fn ensure_same_width(on_set: &Cover, dont_care: &Cover) -> IrredundantResult<()>
{
    if on_set.width() != dont_care.width()
    {
        return Err(IrredundantError::CoverWidthMismatch {
            on_set: on_set.width(),
            dont_care: dont_care.width(),
        });
    }

    Ok(())
}

fn cube_is_covered_except(
    on_set: &Cover,
    dont_care: &Cover,
    cube: &Cube,
    excluded_on_index: Option<usize>,
) -> IrredundantResult<bool>
{
    let cover = cover_from_index_sets(on_set, dont_care, &(0..on_set.cube_count()).collect(), excluded_on_index);
    Ok(cube_is_covered_by(cube, &cover))
}

fn cube_is_covered_by(cube: &Cube, cover: &Cover) -> bool
{
    subtract_cover(cube.clone(), cover.cubes()).is_empty()
}

fn cover_from_index_sets(
    on_set: &Cover,
    dont_care: &Cover,
    on_indices: &BTreeSet<usize>,
    excluded_on_index: Option<usize>,
) -> Cover
{
    let mut cubes = dont_care.cubes().to_vec();
    cubes.extend(
        on_indices
            .iter()
            .filter(|index| Some(**index) != excluded_on_index)
            .filter_map(|index| on_set.cubes().get(*index))
            .cloned(),
    );

    Cover {
        width: on_set.width(),
        cubes,
    }
}

fn subtract_cover(cube: Cube, cover: &[Cube]) -> Vec<Cube>
{
    let mut residual = vec![cube];
    for covering_cube in cover
    {
        residual = residual
            .into_iter()
            .flat_map(|remaining| subtract_cube(remaining, covering_cube))
            .collect();

        if residual.is_empty()
        {
            break;
        }
    }

    residual
}

fn subtract_cube(base: Cube, covering_cube: &Cube) -> Vec<Cube>
{
    if !base.intersects(covering_cube)
    {
        return vec![base];
    }

    if covering_cube.contains_cube(&base)
    {
        return Vec::new();
    }

    let split_variable = base
        .literals()
        .iter()
        .zip(covering_cube.literals())
        .enumerate()
        .find_map(|(index, (left, right))| {
            if *left == Literal::DontCare && *right != Literal::DontCare
            {
                Some(index)
            }
            else
            {
                None
            }
        });

    match split_variable
    {
        Some(variable) => base
            .split_on(variable)
            .into_iter()
            .flat_map(|part| subtract_cube(part, covering_cube))
            .collect(),
        None => vec![base],
    }
}

fn derive_rows_for_fragment(
    fragment: Cube,
    candidates: &[(usize, Cube)],
    rows: &mut Vec<BTreeSet<usize>>,
) -> IrredundantResult<()>
{
    let intersecting = candidates
        .iter()
        .filter(|(_, candidate)| candidate.intersects(&fragment))
        .collect::<Vec<_>>();

    let split_variable = intersecting.iter().find_map(|(_, candidate)| {
        if candidate.contains_cube(&fragment)
        {
            None
        }
        else
        {
            fragment
                .literals()
                .iter()
                .zip(candidate.literals())
                .enumerate()
                .find_map(|(index, (left, right))| {
                    if *left == Literal::DontCare && *right != Literal::DontCare
                    {
                        Some(index)
                    }
                    else
                    {
                        None
                    }
                })
        }
    });

    if let Some(variable) = split_variable
    {
        for part in fragment.split_on(variable)
        {
            derive_rows_for_fragment(part, candidates, rows)?;
        }

        return Ok(());
    }

    let row = intersecting
        .into_iter()
        .filter(|(_, candidate)| candidate.contains_cube(&fragment))
        .map(|(index, _)| *index)
        .collect::<BTreeSet<_>>();

    if row.is_empty()
    {
        return Err(IrredundantError::UncoverableResidualCube { cube: fragment });
    }

    rows.push(row);
    Ok(())
}

fn minimum_cover(rows: &[BTreeSet<usize>]) -> BTreeSet<usize>
{
    let mut selected = rows
        .iter()
        .filter(|row| row.len() == 1)
        .flat_map(|row| row.iter().copied())
        .collect::<BTreeSet<_>>();
    let remaining = uncovered_rows(rows, &selected);
    let mut best = None;
    search_minimum_cover(&remaining, &mut selected, &mut best);
    best.unwrap_or(selected)
}

fn search_minimum_cover(
    rows: &[BTreeSet<usize>],
    selected: &mut BTreeSet<usize>,
    best: &mut Option<BTreeSet<usize>>,
)
{
    if rows.is_empty()
    {
        if best.as_ref().is_none_or(|candidate| {
            selected.len() < candidate.len() || (selected.len() == candidate.len() && &*selected < candidate)
        })
        {
            *best = Some(selected.clone());
        }

        return;
    }

    if best
        .as_ref()
        .is_some_and(|candidate| selected.len() >= candidate.len())
    {
        return;
    }

    let row = rows.iter().min_by_key(|row| row.len()).unwrap();
    for column in row
    {
        selected.insert(*column);
        let next_rows = uncovered_rows(rows, selected);
        search_minimum_cover(&next_rows, selected, best);
        selected.remove(column);
    }
}

fn uncovered_rows(rows: &[BTreeSet<usize>], selected: &BTreeSet<usize>) -> Vec<BTreeSet<usize>>
{
    rows.iter()
        .filter(|row| row.is_disjoint(selected))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn cube_coverage_excludes_the_cube_under_test()
    {
        let on_set = cover(2, [cube([dc(), one()])]);
        let dont_care = Cover::empty(2);

        let split = irred_split_cover(&on_set, &dont_care).unwrap();

        assert_eq!(split.relatively_essential(), &set([0]));
        assert!(split.totally_redundant().is_empty());
        assert!(split.partially_redundant().is_empty());
    }

    #[test]
    fn removes_cube_covered_by_another_on_cube()
    {
        let on_set = cover(2, [cube([dc(), dc()]), cube([one(), one()])]);
        let dont_care = Cover::empty(2);

        let result = irredundant(&on_set, &dont_care).unwrap();
        let report = mark_irredundant(&on_set, &dont_care).unwrap();

        assert_eq!(report.split().relatively_essential(), &set([0]));
        assert_eq!(report.split().totally_redundant(), &set([1]));
        assert_eq!(result.cubes(), &[cube([dc(), dc()])]);
    }

    #[test]
    fn selects_minimum_subset_of_partial_redundant_cubes()
    {
        let on_set = cover(
            2,
            [
                cube([dc(), zero()]),
                cube([dc(), one()]),
                cube([zero(), dc()]),
                cube([one(), dc()]),
            ],
        );
        let dont_care = Cover::empty(2);

        let report = mark_irredundant(&on_set, &dont_care).unwrap();
        let result = irredundant(&on_set, &dont_care).unwrap();

        assert!(report.split().relatively_essential().is_empty());
        assert_eq!(report.split().partially_redundant(), &set([0, 1, 2, 3]));
        assert_eq!(report.selected_partials(), &set([0, 1]));
        assert_eq!(result.cubes(), &[cube([dc(), zero()]), cube([dc(), one()])]);
    }

    #[test]
    fn dont_care_cover_can_make_a_cube_totally_redundant()
    {
        let on_set = cover(2, [cube([zero(), zero()])]);
        let dont_care = cover(2, [cube([zero(), dc()])]);

        let report = mark_irredundant(&on_set, &dont_care).unwrap();
        let result = irredundant(&on_set, &dont_care).unwrap();

        assert_eq!(report.split().totally_redundant(), &set([0]));
        assert!(result.cubes().is_empty());
    }

    #[test]
    fn tautology_detects_cover_of_the_boolean_space()
    {
        let complete = cover(1, [cube([zero()]), cube([one()])]);
        let incomplete = cover(2, [cube([zero(), dc()])]);

        assert!(tautology(&complete));
        assert!(!tautology(&incomplete));
    }

    #[test]
    fn rejects_mismatched_cover_widths()
    {
        let on_set = cover(1, [cube([one()])]);
        let dont_care = Cover::empty(2);

        assert_eq!(
            mark_irredundant(&on_set, &dont_care).unwrap_err(),
            IrredundantError::CoverWidthMismatch {
                on_set: 1,
                dont_care: 2
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("irred.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-")));
    }

    fn cover<const N: usize>(width: usize, cubes: [Cube; N]) -> Cover
    {
        Cover::new(width, cubes.into()).unwrap()
    }

    fn cube<const N: usize>(literals: [Literal; N]) -> Cube
    {
        Cube::new(literals.into())
    }

    fn zero() -> Literal
    {
        Literal::Zero
    }

    fn one() -> Literal
    {
        Literal::One
    }

    fn dc() -> Literal
    {
        Literal::DontCare
    }

    fn set<const N: usize>(values: [usize; N]) -> BTreeSet<usize>
    {
        values.into_iter().collect()
    }
}
