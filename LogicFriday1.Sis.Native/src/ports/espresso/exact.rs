//! Native Rust exact-minimization driver for Espresso covers.
//!
//! The legacy C unit coordinates prime generation, irredundant splitting,
//! sparse minimum-cover solving, result reconstruction, and the optional sparse
//! cleanup pass. The cover-analysis routines are owned by separate Espresso
//! ports, so this module keeps the exact driver native and delegates those
//! stages through an explicit Rust backend.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::ports::mincov::mincov::{MinCoverError, MinCoverResult, sm_minimum_cover};
use crate::ports::sparse::matrix::SparseMatrix;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactOptions
{
    pub exact_cover: bool,
    pub weighted_literals: bool,
    pub skip_make_sparse: bool,
    pub mincov_debug: bool,
}

impl ExactOptions
{
    pub const fn exact_cover() -> Self
    {
        Self {
            exact_cover: true,
            weighted_literals: false,
            skip_make_sparse: false,
            mincov_debug: false,
        }
    }

    pub const fn heuristic_cover() -> Self
    {
        Self {
            exact_cover: false,
            weighted_literals: false,
            skip_make_sparse: false,
            mincov_debug: false,
        }
    }

    pub const fn exact_literal_cover() -> Self
    {
        Self {
            exact_cover: true,
            weighted_literals: true,
            skip_make_sparse: false,
            mincov_debug: false,
        }
    }
}

impl Default for ExactOptions
{
    fn default() -> Self
    {
        Self::exact_cover()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputRange
{
    first_part: usize,
    last_part: usize,
}

impl OutputRange
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
pub struct CubeLayout
{
    set_size: usize,
    output_range: Option<OutputRange>,
}

impl CubeLayout
{
    pub const fn new(set_size: usize) -> Self
    {
        Self {
            set_size,
            output_range: None,
        }
    }

    pub const fn with_output_range(set_size: usize, output_range: OutputRange) -> Self
    {
        Self {
            set_size,
            output_range: Some(output_range),
        }
    }

    pub const fn set_size(&self) -> usize
    {
        self.set_size
    }

    pub const fn output_range(&self) -> Option<OutputRange>
    {
        self.output_range
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactCube
{
    parts: BTreeSet<usize>,
}

impl ExactCube
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

    pub fn parts(&self) -> &BTreeSet<usize>
    {
        &self.parts
    }

    pub fn contains(&self, part: usize) -> bool
    {
        self.parts.contains(&part)
    }

    pub fn literal_weight(&self, layout: &CubeLayout) -> Result<i32, ExactError>
    {
        validate_cube_parts(self, layout.set_size())?;

        let mut weight = layout.set_size() as i32 - self.parts.len() as i32;
        if let Some(output_range) = layout.output_range()
        {
            validate_output_range(output_range, layout.set_size())?;
            for part in output_range.parts()
            {
                if self.contains(part)
                {
                    weight += 1;
                }
                else
                {
                    weight -= 1;
                }
            }
        }

        if weight <= 0
        {
            return Err(ExactError::NonPositiveLiteralWeight { weight });
        }

        Ok(weight)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactCover
{
    layout: CubeLayout,
    cubes: Vec<ExactCube>,
}

impl ExactCover
{
    pub fn new(layout: CubeLayout) -> Self
    {
        Self {
            layout,
            cubes: Vec::new(),
        }
    }

    pub fn from_cubes(layout: CubeLayout, cubes: impl IntoIterator<Item = ExactCube>) -> Self
    {
        Self {
            layout,
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn layout(&self) -> &CubeLayout
    {
        &self.layout
    }

    pub fn len(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }

    pub fn cubes(&self) -> &[ExactCube]
    {
        &self.cubes
    }

    pub fn push(&mut self, cube: ExactCube)
    {
        debug_assert!(cube.parts().iter().all(|part| *part < self.layout.set_size()));
        self.cubes.push(cube);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedCube
{
    index: usize,
    cube: ExactCube,
}

impl IndexedCube
{
    pub const fn index(&self) -> usize
    {
        self.index
    }

    pub const fn cube(&self) -> &ExactCube
    {
        &self.cube
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedCover
{
    layout: CubeLayout,
    cubes: Vec<IndexedCube>,
}

impl IndexedCover
{
    pub fn from_cover(cover: ExactCover) -> Self
    {
        let layout = cover.layout;
        let cubes = cover
            .cubes
            .into_iter()
            .enumerate()
            .map(|(index, cube)| IndexedCube { index, cube })
            .collect();

        Self {
            layout,
            cubes,
        }
    }

    pub fn from_indexed_cubes(
        layout: CubeLayout,
        cubes: impl IntoIterator<Item = IndexedCube>,
    ) -> Self
    {
        Self {
            layout,
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn empty(layout: CubeLayout) -> Self
    {
        Self {
            layout,
            cubes: Vec::new(),
        }
    }

    pub fn layout(&self) -> &CubeLayout
    {
        &self.layout
    }

    pub fn len(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }

    pub fn cubes(&self) -> &[IndexedCube]
    {
        &self.cubes
    }

    pub fn selected_by_indices(&self, indices: &BTreeSet<usize>) -> ExactCover
    {
        ExactCover::from_cubes(
            self.layout.clone(),
            self.cubes
                .iter()
                .filter(|cube| indices.contains(&cube.index()))
                .map(|cube| cube.cube().clone()),
        )
    }

    fn lookup(&self) -> BTreeMap<usize, ExactCube>
    {
        self.cubes
            .iter()
            .map(|cube| (cube.index(), cube.cube().clone()))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrredundantSplit
{
    pub essential: IndexedCover,
    pub totally_redundant: IndexedCover,
    pub partially_redundant: IndexedCover,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactResult
{
    pub cover: ExactCover,
    pub min_cover: MinCoverResult,
    pub essential_count: usize,
    pub totally_redundant_count: usize,
    pub partially_redundant_count: usize,
    pub sparse_cleanup_applied: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactError
{
    CubePartOutOfRange { part: usize, set_size: usize },
    OutputRangeOutOfRange { range: OutputRange, set_size: usize },
    NonPositiveLiteralWeight { weight: i32 },
    MissingPrimeIndex { index: usize },
    Backend(String),
    MinimumCover(MinCoverError),
}

impl fmt::Display for ExactError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::CubePartOutOfRange { part, set_size } => {
                write!(f, "espresso cube part {part} is outside set size {set_size}")
            }
            Self::OutputRangeOutOfRange { range, set_size } => write!(
                f,
                "espresso output range {}..={} is outside set size {set_size}",
                range.first_part(),
                range.last_part()
            ),
            Self::NonPositiveLiteralWeight { weight } => {
                write!(f, "espresso exact literal weight {weight} is not positive")
            }
            Self::MissingPrimeIndex { index } => {
                write!(f, "espresso exact table selected missing prime index {index}")
            }
            Self::Backend(message) => write!(f, "{message}"),
            Self::MinimumCover(error) => write!(f, "{error}"),
        }
    }
}

impl Error for ExactError
{
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match self
        {
            Self::MinimumCover(error) => Some(error),
            _ => None,
        }
    }
}

impl From<MinCoverError> for ExactError
{
    fn from(value: MinCoverError) -> Self
    {
        Self::MinimumCover(value)
    }
}

pub trait ExactBackend
{
    fn generate_primes(
        &mut self,
        onset: &ExactCover,
        dont_care: &ExactCover,
    ) -> Result<ExactCover, ExactError>;

    fn split_irredundant(
        &mut self,
        primes: &IndexedCover,
        dont_care: &ExactCover,
    ) -> Result<IrredundantSplit, ExactError>;

    fn derive_prime_table(
        &mut self,
        dont_care: &ExactCover,
        split: &IrredundantSplit,
    ) -> Result<SparseMatrix, ExactError>;

    fn make_sparse(
        &mut self,
        cover: ExactCover,
        dont_care: &ExactCover,
        off_set: &ExactCover,
    ) -> Result<ExactCover, ExactError>;
}

pub fn minimize_exact<B>(
    backend: &mut B,
    onset: &ExactCover,
    dont_care: &ExactCover,
    off_set: Option<&ExactCover>,
    options: ExactOptions,
) -> Result<ExactResult, ExactError>
where
    B: ExactBackend,
{
    let generated_primes = backend.generate_primes(onset, dont_care)?;
    let primes = IndexedCover::from_cover(generated_primes);
    let split = backend.split_irredundant(&primes, dont_care)?;
    let table = backend.derive_prime_table(dont_care, &split)?;
    let weights = if options.weighted_literals
    {
        Some(literal_weights(&primes)?)
    }
    else
    {
        None
    };
    let min_cover = sm_minimum_cover(
        &table,
        weights.as_deref(),
        !options.exact_cover,
        if options.mincov_debug { 4 } else { 0 },
    )?;

    let mut cover = form_result_cover(&primes, &split, &min_cover.cover)?;
    let mut sparse_cleanup_applied = false;
    if !options.skip_make_sparse
    {
        if let Some(off_set) = off_set
        {
            cover = backend.make_sparse(cover, dont_care, off_set)?;
            sparse_cleanup_applied = true;
        }
    }

    Ok(ExactResult {
        cover,
        min_cover,
        essential_count: split.essential.len(),
        totally_redundant_count: split.totally_redundant.len(),
        partially_redundant_count: split.partially_redundant.len(),
        sparse_cleanup_applied,
    })
}

pub fn minimize_exact_literals<B>(
    backend: &mut B,
    onset: &ExactCover,
    dont_care: &ExactCover,
    off_set: Option<&ExactCover>,
    mut options: ExactOptions,
) -> Result<ExactResult, ExactError>
where
    B: ExactBackend,
{
    options.weighted_literals = true;
    minimize_exact(backend, onset, dont_care, off_set, options)
}

pub fn literal_weights(primes: &IndexedCover) -> Result<Vec<i32>, ExactError>
{
    let Some(max_index) = primes.cubes().iter().map(IndexedCube::index).max()
    else
    {
        return Ok(Vec::new());
    };

    let mut weights = vec![1; max_index + 1];
    for prime in primes.cubes()
    {
        weights[prime.index()] = prime.cube().literal_weight(primes.layout())?;
    }

    Ok(weights)
}

pub fn form_result_cover(
    primes: &IndexedCover,
    split: &IrredundantSplit,
    selected_prime_indices: &[usize],
) -> Result<ExactCover, ExactError>
{
    let selected_prime_indices = selected_prime_indices.iter().copied().collect::<BTreeSet<_>>();
    let lookup = primes.lookup();
    let mut result = ExactCover::new(primes.layout().clone());

    for cube in split.essential.cubes()
    {
        result.push(cube.cube().clone());
    }

    for index in selected_prime_indices
    {
        let Some(cube) = lookup.get(&index)
        else
        {
            return Err(ExactError::MissingPrimeIndex { index });
        };
        result.push(cube.clone());
    }

    Ok(result)
}

fn validate_cube_parts(cube: &ExactCube, set_size: usize) -> Result<(), ExactError>
{
    for part in cube.parts()
    {
        if *part >= set_size
        {
            return Err(ExactError::CubePartOutOfRange {
                part: *part,
                set_size,
            });
        }
    }

    Ok(())
}

fn validate_output_range(range: OutputRange, set_size: usize) -> Result<(), ExactError>
{
    if range.first_part() > range.last_part() || range.last_part() >= set_size
    {
        return Err(ExactError::OutputRangeOutOfRange { range, set_size });
    }

    Ok(())
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Default)]
    struct RecordingBackend
    {
        primes: Option<ExactCover>,
        split: Option<IrredundantSplit>,
        table: SparseMatrix,
        calls: Vec<&'static str>,
    }

    impl RecordingBackend
    {
        fn new(primes: ExactCover, split: IrredundantSplit, table: SparseMatrix) -> Self
        {
            Self {
                primes: Some(primes),
                split: Some(split),
                table,
                calls: Vec::new(),
            }
        }
    }

    impl ExactBackend for RecordingBackend
    {
        fn generate_primes(
            &mut self,
            _onset: &ExactCover,
            _dont_care: &ExactCover,
        ) -> Result<ExactCover, ExactError>
        {
            self.calls.push("primes");
            self.primes
                .clone()
                .ok_or_else(|| ExactError::Backend("missing test primes".to_string()))
        }

        fn split_irredundant(
            &mut self,
            _primes: &IndexedCover,
            _dont_care: &ExactCover,
        ) -> Result<IrredundantSplit, ExactError>
        {
            self.calls.push("split");
            self.split
                .clone()
                .ok_or_else(|| ExactError::Backend("missing test split".to_string()))
        }

        fn derive_prime_table(
            &mut self,
            _dont_care: &ExactCover,
            _split: &IrredundantSplit,
        ) -> Result<SparseMatrix, ExactError>
        {
            self.calls.push("table");
            Ok(self.table.clone())
        }

        fn make_sparse(
            &mut self,
            mut cover: ExactCover,
            _dont_care: &ExactCover,
            _off_set: &ExactCover,
        ) -> Result<ExactCover, ExactError>
        {
            self.calls.push("sparse");
            cover.push(ExactCube::from_parts([3]));
            Ok(cover)
        }
    }

    fn cube(parts: &[usize]) -> ExactCube
    {
        ExactCube::from_parts(parts.iter().copied())
    }

    fn cover(layout: CubeLayout, cubes: &[&[usize]]) -> ExactCover
    {
        ExactCover::from_cubes(layout, cubes.iter().map(|parts| cube(parts)))
    }

    fn indexed(layout: CubeLayout, entries: &[(usize, &[usize])]) -> IndexedCover
    {
        IndexedCover::from_indexed_cubes(
            layout,
            entries.iter().map(|(index, parts)| IndexedCube {
                index: *index,
                cube: cube(parts),
            }),
        )
    }

    #[test]
    fn literal_weights_match_exact_c_literal_count_correction()
    {
        let layout = CubeLayout::with_output_range(6, OutputRange::new(4, 5));
        let primes = indexed(layout, &[(0, &[0, 4]), (3, &[0, 1, 5])]);

        let weights = literal_weights(&primes).unwrap();

        assert_eq!(weights, vec![4, 1, 1, 3]);
    }

    #[test]
    fn exact_driver_solves_prime_table_and_rebuilds_cover()
    {
        let layout = CubeLayout::new(4);
        let primes = cover(layout.clone(), &[&[0], &[0, 1, 2], &[2]]);
        let split = IrredundantSplit {
            essential: indexed(layout.clone(), &[(2, &[2])]),
            totally_redundant: IndexedCover::empty(layout.clone()),
            partially_redundant: indexed(layout.clone(), &[(0, &[0]), (1, &[0, 1, 2])]),
        };
        let mut table = SparseMatrix::new();
        table.insert(0, 0);
        table.insert(0, 1);
        let mut backend = RecordingBackend::new(primes, split, table);
        let onset = ExactCover::new(layout.clone());
        let dont_care = ExactCover::new(layout.clone());

        let result = minimize_exact(
            &mut backend,
            &onset,
            &dont_care,
            None,
            ExactOptions::exact_literal_cover(),
        )
        .unwrap();

        assert_eq!(backend.calls, vec!["primes", "split", "table"]);
        assert_eq!(result.min_cover.cover, vec![1]);
        assert_eq!(result.cover, cover(layout, &[&[2], &[0, 1, 2]]));
        assert_eq!(result.essential_count, 1);
        assert_eq!(result.partially_redundant_count, 2);
        assert!(!result.sparse_cleanup_applied);
    }

    #[test]
    fn exact_driver_uses_heuristic_when_exact_cover_is_false()
    {
        let layout = CubeLayout::new(4);
        let primes = cover(layout.clone(), &[&[0], &[1], &[2]]);
        let split = IrredundantSplit {
            essential: IndexedCover::empty(layout.clone()),
            totally_redundant: IndexedCover::empty(layout.clone()),
            partially_redundant: indexed(layout.clone(), &[(0, &[0]), (1, &[1]), (2, &[2])]),
        };
        let mut table = SparseMatrix::new();
        table.insert(0, 0);
        table.insert(0, 1);
        table.insert(1, 1);
        table.insert(1, 2);
        let mut backend = RecordingBackend::new(primes, split, table);

        let result = minimize_exact(
            &mut backend,
            &ExactCover::new(layout.clone()),
            &ExactCover::new(layout.clone()),
            None,
            ExactOptions::heuristic_cover(),
        )
        .unwrap();

        assert!(!result.min_cover.cover.is_empty());
    }

    #[test]
    fn sparse_cleanup_runs_only_when_enabled_and_offset_exists()
    {
        let layout = CubeLayout::new(4);
        let primes = cover(layout.clone(), &[&[0]]);
        let split = IrredundantSplit {
            essential: IndexedCover::empty(layout.clone()),
            totally_redundant: IndexedCover::empty(layout.clone()),
            partially_redundant: indexed(layout.clone(), &[(0, &[0])]),
        };
        let mut table = SparseMatrix::new();
        table.insert(0, 0);
        let mut backend = RecordingBackend::new(primes, split, table);

        let result = minimize_exact(
            &mut backend,
            &ExactCover::new(layout.clone()),
            &ExactCover::new(layout.clone()),
            Some(&ExactCover::new(layout.clone())),
            ExactOptions::exact_cover(),
        )
        .unwrap();

        assert_eq!(backend.calls, vec!["primes", "split", "table", "sparse"]);
        assert!(result.sparse_cleanup_applied);
        assert_eq!(result.cover, cover(layout, &[&[0], &[3]]));
    }

    #[test]
    fn selected_unknown_prime_index_is_reported()
    {
        let layout = CubeLayout::new(4);
        let primes = indexed(layout.clone(), &[(0, &[0])]);
        let split = IrredundantSplit {
            essential: IndexedCover::empty(layout),
            totally_redundant: IndexedCover::empty(CubeLayout::new(4)),
            partially_redundant: primes.clone(),
        };

        assert_eq!(
            form_result_cover(&primes, &split, &[7]).unwrap_err(),
            ExactError::MissingPrimeIndex { index: 7 }
        );
    }

    #[test]
    fn no_disallowed_porting_tokens_are_present()
    {
        let source = include_str!("exact.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-")));
    }
}
