//! Native Rust Espresso minimization orchestration.
//!
//! This module ports the algorithm layer from Espresso's `espresso.c` and
//! `exact.c`: mode dispatch, the heuristic expand/reduce/irredundant loop,
//! and exact prime-table covering. The lower cover transformations are exposed
//! as Rust traits because their native ports are separate work items.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinimizeMode {
    FastJoint,
    FastIndependentOutput,
    ExactJoint,
    ExactIndependentOutput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MinimizeOptions {
    pub mode: MinimizeMode,
    pub heuristic: HeuristicOptions,
    pub exact: ExactOptions,
}

impl MinimizeOptions {
    pub const fn fast_joint() -> Self {
        Self {
            mode: MinimizeMode::FastJoint,
            heuristic: HeuristicOptions::logic_friday_fast(),
            exact: ExactOptions::exact_cover(),
        }
    }

    pub const fn fast_independent_output() -> Self {
        Self {
            mode: MinimizeMode::FastIndependentOutput,
            heuristic: HeuristicOptions::logic_friday_fast(),
            exact: ExactOptions::exact_cover(),
        }
    }

    pub const fn exact_joint() -> Self {
        Self {
            mode: MinimizeMode::ExactJoint,
            heuristic: HeuristicOptions::logic_friday_fast(),
            exact: ExactOptions::exact_cover(),
        }
    }

    pub const fn exact_independent_output() -> Self {
        Self {
            mode: MinimizeMode::ExactIndependentOutput,
            heuristic: HeuristicOptions::logic_friday_fast(),
            exact: ExactOptions::exact_cover(),
        }
    }
}

impl Default for MinimizeOptions {
    fn default() -> Self {
        Self::fast_joint()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeuristicOptions {
    pub remove_essential: bool,
    pub single_expand: bool,
    pub use_super_gasp: bool,
    pub recompute_onset: bool,
    pub unwrap_onset: bool,
    pub skip_make_sparse: bool,
}

impl HeuristicOptions {
    pub const fn logic_friday_fast() -> Self {
        Self {
            remove_essential: true,
            single_expand: false,
            use_super_gasp: false,
            recompute_onset: false,
            unwrap_onset: true,
            skip_make_sparse: false,
        }
    }
}

impl Default for HeuristicOptions {
    fn default() -> Self {
        Self::logic_friday_fast()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactOptions {
    pub exact_cover: bool,
    pub weighted_literals: bool,
    pub skip_make_sparse: bool,
}

impl ExactOptions {
    pub const fn exact_cover() -> Self {
        Self {
            exact_cover: true,
            weighted_literals: false,
            skip_make_sparse: false,
        }
    }

    pub const fn heuristic_cover() -> Self {
        Self {
            exact_cover: false,
            weighted_literals: false,
            skip_make_sparse: false,
        }
    }

    pub const fn exact_literal_cover() -> Self {
        Self {
            exact_cover: true,
            weighted_literals: true,
            skip_make_sparse: false,
        }
    }
}

impl Default for ExactOptions {
    fn default() -> Self {
        Self::exact_cover()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    columns: BTreeSet<usize>,
    prime: bool,
    covered: bool,
    nonessential: bool,
    relatively_essential: bool,
}

impl Cube {
    pub fn empty() -> Self {
        Self {
            columns: BTreeSet::new(),
            prime: false,
            covered: false,
            nonessential: false,
            relatively_essential: true,
        }
    }

    pub fn from_columns(columns: impl IntoIterator<Item = usize>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            prime: false,
            covered: false,
            nonessential: false,
            relatively_essential: true,
        }
    }

    pub fn columns(&self) -> impl Iterator<Item = usize> + '_ {
        self.columns.iter().copied()
    }

    pub fn literal_count(&self) -> usize {
        self.columns.len()
    }

    pub fn is_prime(&self) -> bool {
        self.prime
    }

    pub fn set_prime(&mut self, value: bool) {
        self.prime = value;
    }

    pub fn is_covered(&self) -> bool {
        self.covered
    }

    pub fn set_covered(&mut self, value: bool) {
        self.covered = value;
    }

    pub fn is_nonessential(&self) -> bool {
        self.nonessential
    }

    pub fn set_nonessential(&mut self, value: bool) {
        self.nonessential = value;
    }

    pub fn is_relatively_essential(&self) -> bool {
        self.relatively_essential
    }

    pub fn set_relatively_essential(&mut self, value: bool) {
        self.relatively_essential = value;
    }

    fn contains(&self, column: usize) -> bool {
        self.columns.contains(&column)
    }

    fn insert(&mut self, column: usize) -> bool {
        self.columns.insert(column)
    }

    fn remove(&mut self, column: usize) -> bool {
        self.columns.remove(&column)
    }

    fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    fn is_subset_of(&self, other: &Self) -> bool {
        self.columns.is_subset(&other.columns)
    }

    fn is_disjoint_from(&self, other: &Self) -> bool {
        self.columns.is_disjoint(&other.columns)
    }

    fn union(&self, other: &Self) -> Self {
        let mut cube = Self::from_columns(self.columns.union(&other.columns).copied());
        cube.prime = self.prime && other.prime;
        cube
    }

    fn intersect(&self, other: &Self) -> Self {
        Self::from_columns(self.columns.intersection(&other.columns).copied())
    }

    fn difference(&self, other: &Self) -> Self {
        Self::from_columns(self.columns.difference(&other.columns).copied())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    set_size: usize,
    output_part_size: usize,
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(set_size: usize) -> Self {
        Self::with_output_part_size(set_size, 1)
    }

    pub fn with_output_part_size(set_size: usize, output_part_size: usize) -> Self {
        Self {
            set_size,
            output_part_size: output_part_size.max(1),
            cubes: Vec::new(),
        }
    }

    pub fn from_cubes(set_size: usize, cubes: impl IntoIterator<Item = Cube>) -> Self {
        let mut cover = Self::new(set_size);
        for cube in cubes {
            cover.push(cube);
        }
        cover
    }

    pub fn from_cubes_with_output_part_size(
        set_size: usize,
        output_part_size: usize,
        cubes: impl IntoIterator<Item = Cube>,
    ) -> Self {
        let mut cover = Self::with_output_part_size(set_size, output_part_size);
        for cube in cubes {
            cover.push(cube);
        }
        cover
    }

    pub fn push(&mut self, cube: Cube) {
        debug_assert!(cube.columns().all(|column| column < self.set_size));
        self.cubes.push(cube);
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn set_size(&self) -> usize {
        self.set_size
    }

    pub fn output_part_size(&self) -> usize {
        self.output_part_size
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn cubes_mut(&mut self) -> &mut [Cube] {
        &mut self.cubes
    }

    pub fn retain(&mut self, predicate: impl FnMut(&Cube) -> bool) {
        self.cubes.retain(predicate);
    }

    pub fn clear_prime_flags(&mut self) {
        for cube in &mut self.cubes {
            cube.set_prime(false);
        }
    }

    pub fn append(mut self, mut other: Self) -> Self {
        debug_assert_eq!(self.set_size, other.set_size);
        self.cubes.append(&mut other.cubes);
        self
    }

    pub fn cost(&self) -> CoverCost {
        let total_literals = self.cubes.iter().map(Cube::literal_count).sum();
        let output_literals = output_literal_count(self);
        CoverCost {
            cubes: self.cubes.len(),
            input_literals: total_literals - output_literals,
            output_literals,
            total_literals,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CoverCost {
    pub cubes: usize,
    pub input_literals: usize,
    pub output_literals: usize,
    pub total_literals: usize,
}

impl CoverCost {
    fn improves_outer_loop(self, previous: Self) -> bool {
        self.cubes < previous.cubes
            || (self.cubes == previous.cubes && self.total_literals < previous.total_literals)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pla {
    pub on_set: Cover,
    pub dont_care: Cover,
    pub off_set: Cover,
}

impl Pla {
    pub fn new(on_set: Cover, dont_care: Cover, off_set: Cover) -> Self {
        Self {
            on_set,
            dont_care,
            off_set,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimizeResult {
    pub cover: Cover,
    pub mode: MinimizeMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LowerPort {
    RecomputeOnset,
    UnwrapOnset,
    Expand,
    Irredundant,
    Essential,
    Reduce,
    LastGasp,
    SuperGasp,
    MakeSparse,
    GeneratePrimes,
    SplitIrredundant,
    DerivePrimeTable,
    SplitOutputs,
    MergeOutputs,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MinimizeError {
    MissingLowerPort(LowerPort),
    EmptyPrimeTableRow {
        row: usize,
    },
    MissingPrimeIndex {
        index: usize,
    },
    InvalidOutputPart {
        set_size: usize,
        output_part_size: usize,
    },
    InvalidWeights {
        expected: usize,
        actual: usize,
    },
    InvalidVariableRange {
        first_column: usize,
        last_column: usize,
    },
    NonContiguousVariableColumns {
        expected_first_column: usize,
        actual_first_column: usize,
    },
    ColumnOutOfRange {
        column: usize,
        set_size: usize,
    },
    EmptyVariable {
        first_column: usize,
        last_column: usize,
    },
    OnSetIntersectsOffSet,
    MintermEnumerationTooLarge {
        requested: usize,
        limit: usize,
    },
}

impl fmt::Display for MinimizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLowerPort(port) => {
                write!(
                    formatter,
                    "native Espresso lower port is not available: {port:?}"
                )
            }
            Self::EmptyPrimeTableRow { row } => {
                write!(
                    formatter,
                    "exact minimization prime table row {row} has no covering primes"
                )
            }
            Self::MissingPrimeIndex { index } => {
                write!(
                    formatter,
                    "exact minimization selected missing prime index {index}"
                )
            }
            Self::InvalidOutputPart {
                set_size,
                output_part_size,
            } => write!(
                formatter,
                "output part size {output_part_size} is not valid for set size {set_size}"
            ),
            Self::InvalidWeights { expected, actual } => write!(
                formatter,
                "exact minimization got {actual} weights; expected at least {expected}"
            ),
            Self::InvalidVariableRange {
                first_column,
                last_column,
            } => write!(
                formatter,
                "invalid Espresso variable range {first_column}..={last_column}"
            ),
            Self::NonContiguousVariableColumns {
                expected_first_column,
                actual_first_column,
            } => write!(
                formatter,
                "variable starts at column {actual_first_column}, expected {expected_first_column}"
            ),
            Self::ColumnOutOfRange { column, set_size } => {
                write!(formatter, "column {column} is outside 0..{set_size}")
            }
            Self::EmptyVariable {
                first_column,
                last_column,
            } => write!(
                formatter,
                "cube has no parts in variable range {first_column}..={last_column}"
            ),
            Self::OnSetIntersectsOffSet => {
                write!(formatter, "ON-set and OFF-set are not orthogonal")
            }
            Self::MintermEnumerationTooLarge { requested, limit } => write!(
                formatter,
                "minterm enumeration requested {requested} points, over limit {limit}"
            ),
        }
    }
}

impl Error for MinimizeError {}

pub trait HeuristicBackend {
    fn simplify_recomputed_onset(&mut self, _cover: Cover) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::RecomputeOnset))
    }

    fn unwrap_onset(&mut self, _cover: Cover) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::UnwrapOnset))
    }

    fn expand(&mut self, _cover: Cover, _off_set: &Cover) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::Expand))
    }

    fn irredundant(&mut self, _cover: Cover, _dont_care: &Cover) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::Irredundant))
    }

    fn essential(
        &mut self,
        _cover: Cover,
        _dont_care: Cover,
    ) -> Result<(Cover, Cover, Cover), MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::Essential))
    }

    fn reduce(&mut self, _cover: Cover, _dont_care: &Cover) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::Reduce))
    }

    fn last_gasp(
        &mut self,
        _cover: Cover,
        _dont_care: &Cover,
        _off_set: &Cover,
    ) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::LastGasp))
    }

    fn super_gasp(
        &mut self,
        _cover: Cover,
        _dont_care: &Cover,
        _off_set: &Cover,
    ) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::SuperGasp))
    }

    fn make_sparse(
        &mut self,
        _cover: Cover,
        _original_dont_care: &Cover,
        _off_set: &Cover,
    ) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::MakeSparse))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedCube {
    index: usize,
    cube: Cube,
}

impl IndexedCube {
    pub fn new(index: usize, cube: Cube) -> Self {
        Self { index, cube }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn cube(&self) -> &Cube {
        &self.cube
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedCover {
    set_size: usize,
    output_part_size: usize,
    cubes: Vec<IndexedCube>,
}

impl IndexedCover {
    pub fn from_cover(cover: Cover) -> Self {
        let set_size = cover.set_size;
        let output_part_size = cover.output_part_size;
        let cubes = cover
            .cubes
            .into_iter()
            .enumerate()
            .map(|(index, cube)| IndexedCube::new(index, cube))
            .collect();

        Self {
            set_size,
            output_part_size,
            cubes,
        }
    }

    pub fn from_indexed_cubes(
        set_size: usize,
        output_part_size: usize,
        cubes: impl IntoIterator<Item = IndexedCube>,
    ) -> Self {
        Self {
            set_size,
            output_part_size,
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn empty(set_size: usize, output_part_size: usize) -> Self {
        Self {
            set_size,
            output_part_size,
            cubes: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn cubes(&self) -> &[IndexedCube] {
        &self.cubes
    }

    fn lookup(&self) -> BTreeMap<usize, Cube> {
        self.cubes
            .iter()
            .map(|cube| (cube.index(), cube.cube().clone()))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrredundantSplit {
    pub essential: IndexedCover,
    pub totally_redundant: IndexedCover,
    pub partially_redundant: IndexedCover,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimeTable {
    rows: Vec<BTreeSet<usize>>,
}

impl PrimeTable {
    pub fn new(rows: impl IntoIterator<Item = BTreeSet<usize>>) -> Self {
        Self {
            rows: rows.into_iter().collect(),
        }
    }

    pub fn rows(&self) -> &[BTreeSet<usize>] {
        &self.rows
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactResult {
    pub cover: Cover,
    pub selected_prime_indices: Vec<usize>,
    pub essential_count: usize,
    pub totally_redundant_count: usize,
    pub partially_redundant_count: usize,
    pub sparse_cleanup_applied: bool,
}

pub trait ExactBackend: HeuristicBackend {
    fn generate_primes(
        &mut self,
        _on_set: &Cover,
        _dont_care: &Cover,
    ) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::GeneratePrimes))
    }

    fn split_irredundant(
        &mut self,
        _primes: &IndexedCover,
        _dont_care: &Cover,
    ) -> Result<IrredundantSplit, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::SplitIrredundant))
    }

    fn derive_prime_table(
        &mut self,
        _dont_care: &Cover,
        _split: &IrredundantSplit,
    ) -> Result<PrimeTable, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::DerivePrimeTable))
    }

    fn exact_make_sparse(
        &mut self,
        cover: Cover,
        dont_care: &Cover,
        off_set: &Cover,
    ) -> Result<Cover, MinimizeError> {
        HeuristicBackend::make_sparse(self, cover, dont_care, off_set)
    }
}

pub trait OutputPartitionBackend {
    fn split_outputs(&mut self, _problem: Pla) -> Result<Vec<Pla>, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::SplitOutputs))
    }

    fn merge_outputs(&mut self, _covers: Vec<Cover>) -> Result<Cover, MinimizeError> {
        Err(MinimizeError::MissingLowerPort(LowerPort::MergeOutputs))
    }
}

pub trait MinimizationBackend: HeuristicBackend + ExactBackend + OutputPartitionBackend {}

impl<T> MinimizationBackend for T where T: HeuristicBackend + ExactBackend + OutputPartitionBackend {}

pub struct MissingPortsBackend;

impl HeuristicBackend for MissingPortsBackend {}
impl ExactBackend for MissingPortsBackend {}
impl OutputPartitionBackend for MissingPortsBackend {}

pub fn minimize<B>(
    backend: &mut B,
    problem: Pla,
    options: MinimizeOptions,
) -> Result<MinimizeResult, MinimizeError>
where
    B: MinimizationBackend,
{
    let cover = match options.mode {
        MinimizeMode::FastJoint => heuristic_minimize(
            backend,
            problem.on_set,
            problem.dont_care,
            problem.off_set,
            options.heuristic,
        )?,
        MinimizeMode::FastIndependentOutput => {
            let subproblems = backend.split_outputs(problem)?;
            let heuristic_options = HeuristicOptions {
                skip_make_sparse: true,
                ..options.heuristic
            };
            let mut covers = Vec::with_capacity(subproblems.len());
            for subproblem in subproblems {
                covers.push(heuristic_minimize(
                    backend,
                    subproblem.on_set,
                    subproblem.dont_care,
                    subproblem.off_set,
                    heuristic_options,
                )?);
            }
            backend.merge_outputs(covers)?
        }
        MinimizeMode::ExactJoint => {
            exact_minimize(
                backend,
                &problem.on_set,
                &problem.dont_care,
                Some(&problem.off_set),
                options.exact,
            )?
            .cover
        }
        MinimizeMode::ExactIndependentOutput => {
            let subproblems = backend.split_outputs(problem)?;
            let exact_options = ExactOptions {
                skip_make_sparse: true,
                ..options.exact
            };
            let mut covers = Vec::with_capacity(subproblems.len());
            for subproblem in subproblems {
                covers.push(
                    exact_minimize(
                        backend,
                        &subproblem.on_set,
                        &subproblem.dont_care,
                        Some(&subproblem.off_set),
                        exact_options,
                    )?
                    .cover,
                );
            }
            backend.merge_outputs(covers)?
        }
    };

    Ok(MinimizeResult {
        cover,
        mode: options.mode,
    })
}

pub fn heuristic_minimize<B>(
    backend: &mut B,
    on_set: Cover,
    dont_care: Cover,
    off_set: Cover,
    mut options: HeuristicOptions,
) -> Result<Cover, MinimizeError>
where
    B: HeuristicBackend,
{
    loop {
        let saved_on_set = on_set.clone();
        let mut current = on_set.clone();
        let mut scratch_dont_care = dont_care.clone();

        if options.recompute_onset {
            current = backend.simplify_recomputed_onset(current)?;
        }
        if should_unwrap_onset(&current, options) {
            current = backend.unwrap_onset(current)?;
        }

        current.clear_prime_flags();
        current = backend.expand(current, &off_set)?;
        current = backend.irredundant(current, &scratch_dont_care)?;

        if !options.single_expand {
            let essential = if options.remove_essential {
                let (reduced_current, reduced_dont_care, essential) =
                    backend.essential(current, scratch_dont_care)?;
                current = reduced_current;
                scratch_dont_care = reduced_dont_care;
                essential
            } else {
                Cover::new(current.set_size())
            };

            current =
                iterate_heuristic_loop(backend, current, &scratch_dont_care, &off_set, options)?;
            current = current.append(essential);
        }

        if !options.skip_make_sparse {
            current = backend.make_sparse(current, &dont_care, &off_set)?;
        }

        if saved_on_set.len() < current.len() && options.unwrap_onset {
            options.unwrap_onset = false;
            continue;
        }

        return Ok(if saved_on_set.len() < current.len() {
            saved_on_set
        } else {
            current
        });
    }
}

fn iterate_heuristic_loop<B>(
    backend: &mut B,
    mut current: Cover,
    dont_care: &Cover,
    off_set: &Cover,
    options: HeuristicOptions,
) -> Result<Cover, MinimizeError>
where
    B: HeuristicBackend,
{
    let mut cost = current.cost();

    loop {
        loop {
            let best_cost = cost;
            current = backend.reduce(current, dont_care)?;
            current = backend.expand(current, off_set)?;
            current = backend.irredundant(current, dont_care)?;
            cost = current.cost();

            if cost.cubes >= best_cost.cubes {
                break;
            }
        }

        let best_cost = cost;
        current = if options.use_super_gasp {
            backend.super_gasp(current, dont_care, off_set)?
        } else {
            backend.last_gasp(current, dont_care, off_set)?
        };
        cost = current.cost();

        if options.use_super_gasp && cost.cubes >= best_cost.cubes {
            break;
        }
        if !cost.improves_outer_loop(best_cost) {
            break;
        }
    }

    Ok(current)
}

pub fn exact_minimize<B>(
    backend: &mut B,
    on_set: &Cover,
    dont_care: &Cover,
    off_set: Option<&Cover>,
    options: ExactOptions,
) -> Result<ExactResult, MinimizeError>
where
    B: ExactBackend,
{
    let generated_primes = backend.generate_primes(on_set, dont_care)?;
    let primes = IndexedCover::from_cover(generated_primes);
    let split = backend.split_irredundant(&primes, dont_care)?;
    let table = backend.derive_prime_table(dont_care, &split)?;
    let weights = if options.weighted_literals {
        Some(literal_weights(&primes))
    } else {
        None
    };

    let selected_prime_indices =
        minimum_cover(table.rows(), weights.as_deref(), !options.exact_cover)?;
    let mut cover = form_exact_result_cover(&primes, &split, &selected_prime_indices)?;
    let mut sparse_cleanup_applied = false;

    if !options.skip_make_sparse {
        if let Some(off_set) = off_set {
            cover = backend.exact_make_sparse(cover, dont_care, off_set)?;
            sparse_cleanup_applied = true;
        }
    }

    Ok(ExactResult {
        cover,
        selected_prime_indices,
        essential_count: split.essential.len(),
        totally_redundant_count: split.totally_redundant.len(),
        partially_redundant_count: split.partially_redundant.len(),
        sparse_cleanup_applied,
    })
}

pub fn form_exact_result_cover(
    primes: &IndexedCover,
    split: &IrredundantSplit,
    selected_prime_indices: &[usize],
) -> Result<Cover, MinimizeError> {
    let lookup = primes.lookup();
    let mut result = Cover::with_output_part_size(primes.set_size, primes.output_part_size);

    for cube in split.essential.cubes() {
        result.push(cube.cube().clone());
    }

    for index in selected_prime_indices {
        let Some(cube) = lookup.get(index) else {
            return Err(MinimizeError::MissingPrimeIndex { index: *index });
        };
        result.push(cube.clone());
    }

    Ok(result)
}

pub fn literal_weights(primes: &IndexedCover) -> Vec<usize> {
    let Some(max_index) = primes.cubes().iter().map(IndexedCube::index).max() else {
        return Vec::new();
    };

    let mut weights = vec![1; max_index + 1];
    for prime in primes.cubes() {
        weights[prime.index()] =
            literal_weight(prime.cube(), primes.set_size, primes.output_part_size);
    }

    weights
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Variable {
    first_column: usize,
    last_column: usize,
    sparse: bool,
}

impl Variable {
    pub const fn new(first_column: usize, last_column: usize, sparse: bool) -> Self {
        Self {
            first_column,
            last_column,
            sparse,
        }
    }

    pub const fn first_column(self) -> usize {
        self.first_column
    }

    pub const fn last_column(self) -> usize {
        self.last_column
    }

    pub const fn is_sparse(&self) -> bool {
        self.sparse
    }

    fn columns(self) -> impl Iterator<Item = usize> {
        self.first_column..=self.last_column
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeLayout {
    variables: Vec<Variable>,
    set_size: usize,
    enumeration_limit: usize,
}

impl CubeLayout {
    pub fn new(variables: impl IntoIterator<Item = Variable>) -> Result<Self, MinimizeError> {
        Self::with_enumeration_limit(variables, 1_000_000)
    }

    pub fn with_enumeration_limit(
        variables: impl IntoIterator<Item = Variable>,
        enumeration_limit: usize,
    ) -> Result<Self, MinimizeError> {
        let variables = variables.into_iter().collect::<Vec<_>>();
        let mut next_column = 0;

        for variable in &variables {
            if variable.first_column > variable.last_column {
                return Err(MinimizeError::InvalidVariableRange {
                    first_column: variable.first_column,
                    last_column: variable.last_column,
                });
            }
            if variable.first_column != next_column {
                return Err(MinimizeError::NonContiguousVariableColumns {
                    expected_first_column: next_column,
                    actual_first_column: variable.first_column,
                });
            }
            next_column = variable.last_column + 1;
        }

        Ok(Self {
            variables,
            set_size: next_column,
            enumeration_limit,
        })
    }

    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    pub fn set_size(&self) -> usize {
        self.set_size
    }

    pub fn full_cube(&self) -> Cube {
        Cube::from_columns(0..self.set_size)
    }

    pub fn validate_cover(&self, cover: &Cover) -> Result<(), MinimizeError> {
        if cover.set_size() != self.set_size {
            return Err(MinimizeError::ColumnOutOfRange {
                column: self.set_size,
                set_size: cover.set_size(),
            });
        }
        for cube in cover.cubes() {
            self.validate_cube(cube)?;
        }
        Ok(())
    }

    pub fn validate_cube(&self, cube: &Cube) -> Result<(), MinimizeError> {
        if let Some(column) = cube.columns().find(|column| *column >= self.set_size) {
            return Err(MinimizeError::ColumnOutOfRange {
                column,
                set_size: self.set_size,
            });
        }

        for variable in &self.variables {
            if cube_column_count(cube, *variable) == 0 {
                return Err(MinimizeError::EmptyVariable {
                    first_column: variable.first_column,
                    last_column: variable.last_column,
                });
            }
        }

        Ok(())
    }

    fn minterm_count(&self) -> Result<usize, MinimizeError> {
        let requested = self.variables.iter().try_fold(1usize, |acc, variable| {
            acc.checked_mul(variable.last_column - variable.first_column + 1)
        });
        let requested = requested.unwrap_or(usize::MAX);
        if requested > self.enumeration_limit {
            return Err(MinimizeError::MintermEnumerationTooLarge {
                requested,
                limit: self.enumeration_limit,
            });
        }
        Ok(requested)
    }
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandReport {
    pub cover: Cover,
    pub expanded_cubes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeMinimizeBackend {
    layout: CubeLayout,
    expand_options: ExpandOptions,
    reduce_toggle: bool,
}

impl NativeMinimizeBackend {
    pub fn new(layout: CubeLayout) -> Self {
        Self {
            layout,
            expand_options: ExpandOptions::all_variables(),
            reduce_toggle: true,
        }
    }

    pub fn with_expand_options(mut self, expand_options: ExpandOptions) -> Self {
        self.expand_options = expand_options;
        self
    }
}

impl HeuristicBackend for NativeMinimizeBackend {
    fn expand(&mut self, cover: Cover, off_set: &Cover) -> Result<Cover, MinimizeError> {
        Ok(expand_native(cover, off_set, &self.layout, self.expand_options)?.cover)
    }

    fn irredundant(&mut self, cover: Cover, dont_care: &Cover) -> Result<Cover, MinimizeError> {
        irredundant_native(cover, dont_care, &self.layout)
    }

    fn essential(
        &mut self,
        cover: Cover,
        dont_care: Cover,
    ) -> Result<(Cover, Cover, Cover), MinimizeError> {
        essential_native(cover, dont_care, &self.layout)
    }

    fn reduce(&mut self, cover: Cover, dont_care: &Cover) -> Result<Cover, MinimizeError> {
        let result = reduce_native(cover, dont_care, &self.layout, self.reduce_toggle)?;
        self.reduce_toggle = !self.reduce_toggle;
        Ok(result)
    }

    fn make_sparse(
        &mut self,
        cover: Cover,
        original_dont_care: &Cover,
        off_set: &Cover,
    ) -> Result<Cover, MinimizeError> {
        self.layout.validate_cover(original_dont_care)?;
        self.layout.validate_cover(off_set)?;
        mv_reduce_native(cover, original_dont_care, &self.layout)
    }

    fn last_gasp(
        &mut self,
        cover: Cover,
        dont_care: &Cover,
        off_set: &Cover,
    ) -> Result<Cover, MinimizeError> {
        last_gasp_native(cover, dont_care, off_set, &self.layout, self.expand_options)
    }

    fn super_gasp(
        &mut self,
        cover: Cover,
        dont_care: &Cover,
        off_set: &Cover,
    ) -> Result<Cover, MinimizeError> {
        super_gasp_native(cover, dont_care, off_set, &self.layout)
    }
}

impl ExactBackend for NativeMinimizeBackend {
    fn generate_primes(
        &mut self,
        on_set: &Cover,
        dont_care: &Cover,
    ) -> Result<Cover, MinimizeError> {
        all_primes_native(on_set, dont_care, &self.layout)
    }

    fn split_irredundant(
        &mut self,
        primes: &IndexedCover,
        dont_care: &Cover,
    ) -> Result<IrredundantSplit, MinimizeError> {
        split_irredundant_native(primes, dont_care, &self.layout)
    }

    fn derive_prime_table(
        &mut self,
        dont_care: &Cover,
        split: &IrredundantSplit,
    ) -> Result<PrimeTable, MinimizeError> {
        derive_prime_table_native(dont_care, split, &self.layout)
    }
}

impl OutputPartitionBackend for NativeMinimizeBackend {
    fn split_outputs(&mut self, problem: Pla) -> Result<Vec<Pla>, MinimizeError> {
        self.layout.validate_cover(&problem.on_set)?;
        self.layout.validate_cover(&problem.dont_care)?;
        self.layout.validate_cover(&problem.off_set)?;

        let output_columns = output_columns(&problem.on_set)?;
        Ok(output_columns
            .iter()
            .map(|output_column| {
                Pla::new(
                    cof_output(&problem.on_set, *output_column),
                    cof_output(&problem.dont_care, *output_column),
                    cof_output(&problem.off_set, *output_column),
                )
            })
            .collect())
    }

    fn merge_outputs(&mut self, covers: Vec<Cover>) -> Result<Cover, MinimizeError> {
        let mut merged =
            Cover::with_output_part_size(self.layout.set_size(), output_part_size(&covers)?);
        for (index, cover) in covers.into_iter().enumerate() {
            let output_column = output_columns(&cover)?.get(index).copied().ok_or(
                MinimizeError::InvalidOutputPart {
                    set_size: cover.set_size(),
                    output_part_size: cover.output_part_size(),
                },
            )?;
            merged = merged.append(uncof_output(cover, output_column)?);
        }
        Ok(merged)
    }
}

#[path = "minimize_native.rs"]
mod minimize_native;
use minimize_native::cube_column_count;
pub use minimize_native::*;

pub fn last_gasp_native(
    cover: Cover,
    dont_care: &Cover,
    off_set: &Cover,
    layout: &CubeLayout,
    expand_options: ExpandOptions,
) -> Result<Cover, MinimizeError> {
    let reduced = reduce_gasp_native(&cover, dont_care, layout)?;
    let candidates =
        expand_gasp_native(&reduced, dont_care, off_set, &cover, layout, expand_options)?;

    if candidates.is_empty() {
        Ok(cover)
    } else {
        irredundant_native(cover.append(candidates), dont_care, layout)
    }
}

pub fn super_gasp_native(
    cover: Cover,
    dont_care: &Cover,
    off_set: &Cover,
    layout: &CubeLayout,
) -> Result<Cover, MinimizeError> {
    let reduced = reduce_gasp_native(&cover, dont_care, layout)?;
    let new_primes = all_primes_native(&reduced, off_set, layout)?;
    irredundant_native(cover.append(new_primes), dont_care, layout)
}

pub fn reduce_gasp_native(
    cover: &Cover,
    dont_care: &Cover,
    layout: &CubeLayout,
) -> Result<Cover, MinimizeError> {
    layout.validate_cover(cover)?;
    layout.validate_cover(dont_care)?;
    layout.minterm_count()?;

    let mut reduced = Cover::with_output_part_size(cover.set_size(), cover.output_part_size());
    for index in 0..cover.len() {
        let cube = &cover.cubes()[index];
        let essential_points = essential_points_for_cube(cover, dont_care, index, layout);
        if essential_points.is_empty() {
            let mut unchanged = cube.clone();
            unchanged.set_prime(true);
            reduced.push(unchanged);
            continue;
        }

        let mut reduced_cube = cube_from_minterms_gasp(&essential_points, layout);
        if reduced_cube == *cube {
            reduced_cube.set_prime(true);
        } else {
            reduced_cube.set_prime(false);
        }
        reduced.push(reduced_cube);
    }

    Ok(reduced)
}

pub fn expand_gasp_native(
    reduced: &Cover,
    dont_care: &Cover,
    off_set: &Cover,
    original: &Cover,
    layout: &CubeLayout,
    expand_options: ExpandOptions,
) -> Result<Cover, MinimizeError> {
    layout.validate_cover(reduced)?;
    layout.validate_cover(dont_care)?;
    layout.validate_cover(off_set)?;
    layout.validate_cover(original)?;

    let mut candidates =
        Cover::with_output_part_size(reduced.set_size(), reduced.output_part_size());
    for c1_index in 0..reduced.len() {
        let c1_under = &reduced.cubes()[c1_index];
        let mut seed = c1_under.clone();
        seed.set_prime(false);
        let c1_cover = Cover::from_cubes_with_output_part_size(
            reduced.set_size(),
            reduced.output_part_size(),
            [seed],
        );
        let c1_primes = all_primes_native(&c1_cover, off_set, layout)?;

        for (c2_index, c2_under) in reduced.cubes().iter().enumerate() {
            if c1_index == c2_index || c2_under.is_prime() {
                continue;
            }
            if !c1_primes
                .cubes()
                .iter()
                .any(|prime| c2_under.is_subset_of(prime))
            {
                continue;
            }

            let mut replaced = original.clone();
            replaced.cubes[c1_index] = c1_under.clone();
            let c2_points = essential_points_for_cube(&replaced, dont_care, c2_index, layout);
            if c2_points.is_empty() {
                continue;
            }

            let c2_essential = cube_from_minterms_gasp(&c2_points, layout);
            if c1_primes
                .cubes()
                .iter()
                .any(|prime| c2_essential.is_subset_of(prime))
            {
                let mut candidate = c1_under.union(&c2_essential);
                candidate.set_prime(false);
                candidates.push(candidate);
            }
        }
    }

    let candidates = contain_gasp_cover(candidates);
    if candidates.is_empty() {
        Ok(candidates)
    } else {
        Ok(expand_native(candidates, off_set, layout, expand_options)?.cover)
    }
}

fn essential_points_for_cube(
    cover: &Cover,
    dont_care: &Cover,
    cube_index: usize,
    layout: &CubeLayout,
) -> Vec<Cube> {
    enumerate_gasp_minterms(layout)
        .into_iter()
        .filter(|point| {
            cube_covers_gasp_minterm(&cover.cubes()[cube_index], point)
                && !cover.cubes().iter().enumerate().any(|(other, cube)| {
                    other != cube_index && cube_covers_gasp_minterm(cube, point)
                })
                && !dont_care
                    .cubes()
                    .iter()
                    .any(|cube| cube_covers_gasp_minterm(cube, point))
        })
        .collect()
}

fn enumerate_gasp_minterms(layout: &CubeLayout) -> Vec<Cube> {
    fn walk(
        layout: &CubeLayout,
        variable_index: usize,
        current: &mut Cube,
        output: &mut Vec<Cube>,
    ) {
        if variable_index == layout.variables().len() {
            output.push(current.clone());
            return;
        }

        for column in layout.variables()[variable_index].columns() {
            current.insert(column);
            walk(layout, variable_index + 1, current, output);
            current.remove(column);
        }
    }

    let mut output = Vec::new();
    walk(layout, 0, &mut Cube::empty(), &mut output);
    output
}

fn cube_covers_gasp_minterm(cube: &Cube, minterm: &Cube) -> bool {
    minterm.is_subset_of(cube)
}

fn cube_from_minterms_gasp(minterms: &[Cube], layout: &CubeLayout) -> Cube {
    let mut result = Cube::empty();
    for variable in layout.variables() {
        for column in variable.columns() {
            if minterms.iter().any(|minterm| minterm.contains(column)) {
                result.insert(column);
            }
        }
    }
    result
}

fn contain_gasp_cover(cover: Cover) -> Cover {
    let mut cubes = cover.cubes;
    cubes.sort_by_key(|cube| {
        (
            std::cmp::Reverse(cube.literal_count()),
            cube.columns.clone(),
        )
    });
    cubes.dedup();

    let mut kept: Vec<Cube> = Vec::new();
    'outer: for cube in cubes {
        for previous in &kept {
            if cube.is_subset_of(previous) {
                continue 'outer;
            }
        }
        kept.push(cube);
    }

    Cover::from_cubes_with_output_part_size(cover.set_size, cover.output_part_size, kept)
}

pub fn native_minimize_unavailable(
    problem: Pla,
    mode: MinimizeMode,
) -> Result<MinimizeResult, MinimizeError> {
    let mut backend = MissingPortsBackend;
    minimize(
        &mut backend,
        problem,
        MinimizeOptions {
            mode,
            ..MinimizeOptions::default()
        },
    )
}

fn minimum_cover(
    rows: &[BTreeSet<usize>],
    weights: Option<&[usize]>,
    heuristic: bool,
) -> Result<Vec<usize>, MinimizeError> {
    for (row_index, row) in rows.iter().enumerate() {
        if row.is_empty() {
            return Err(MinimizeError::EmptyPrimeTableRow { row: row_index });
        }
    }

    let max_column = rows.iter().flat_map(|row| row.iter().copied()).max();
    if let (Some(weights), Some(max_column)) = (weights, max_column) {
        if weights.len() <= max_column {
            return Err(MinimizeError::InvalidWeights {
                expected: max_column + 1,
                actual: weights.len(),
            });
        }
    }

    let mut selected = rows
        .iter()
        .filter(|row| row.len() == 1)
        .flat_map(|row| row.iter().copied())
        .collect::<BTreeSet<_>>();
    let remaining = uncovered_rows(rows, &selected);

    if heuristic {
        greedy_cover(&remaining, &mut selected, weights);
    } else {
        let mut best = None;
        search_exact_cover(&remaining, &mut selected, &mut best, weights);
        if let Some(best_cover) = best {
            selected = best_cover;
        }
    }

    Ok(selected.into_iter().collect())
}

fn search_exact_cover(
    rows: &[BTreeSet<usize>],
    selected: &mut BTreeSet<usize>,
    best: &mut Option<BTreeSet<usize>>,
    weights: Option<&[usize]>,
) {
    if rows.is_empty() {
        if best
            .as_ref()
            .is_none_or(|candidate| cover_is_better(selected, candidate, weights))
        {
            *best = Some(selected.clone());
        }
        return;
    }

    if best
        .as_ref()
        .is_some_and(|candidate| !cover_is_better(selected, candidate, weights))
    {
        return;
    }

    let row = rows
        .iter()
        .min_by_key(|row| row.len())
        .expect("nonempty rows");
    for column in row {
        selected.insert(*column);
        let next_rows = uncovered_rows(rows, selected);
        search_exact_cover(&next_rows, selected, best, weights);
        selected.remove(column);
    }
}

fn greedy_cover(
    rows: &[BTreeSet<usize>],
    selected: &mut BTreeSet<usize>,
    weights: Option<&[usize]>,
) {
    let mut remaining = uncovered_rows(rows, selected);
    while !remaining.is_empty() {
        let mut candidates = BTreeSet::new();
        for row in &remaining {
            candidates.extend(row.iter().copied());
        }

        let column = candidates
            .into_iter()
            .max_by_key(|column| {
                let covered = remaining.iter().filter(|row| row.contains(column)).count();
                let weight = weights.and_then(|w| w.get(*column)).copied().unwrap_or(1);
                (covered, usize::MAX - weight, usize::MAX - *column)
            })
            .expect("remaining rows have candidates");
        selected.insert(column);
        remaining = uncovered_rows(rows, selected);
    }
}

fn cover_is_better(
    candidate: &BTreeSet<usize>,
    incumbent: &BTreeSet<usize>,
    weights: Option<&[usize]>,
) -> bool {
    let candidate_weight = cover_weight(candidate, weights);
    let incumbent_weight = cover_weight(incumbent, weights);

    candidate_weight < incumbent_weight
        || (candidate_weight == incumbent_weight
            && (candidate.len() < incumbent.len()
                || (candidate.len() == incumbent.len() && candidate < incumbent)))
}

fn cover_weight(cover: &BTreeSet<usize>, weights: Option<&[usize]>) -> usize {
    cover
        .iter()
        .map(|column| weights.and_then(|w| w.get(*column)).copied().unwrap_or(1))
        .sum()
}

fn uncovered_rows(rows: &[BTreeSet<usize>], selected: &BTreeSet<usize>) -> Vec<BTreeSet<usize>> {
    rows.iter()
        .filter(|row| row.is_disjoint(selected))
        .cloned()
        .collect()
}

fn should_unwrap_onset(cover: &Cover, options: HeuristicOptions) -> bool {
    if !options.unwrap_onset || cover.output_part_size() <= 1 {
        return false;
    }

    let cost = cover.cost();
    cost.output_literals != cost.cubes * cover.output_part_size() && cost.output_literals < 5000
}

fn output_literal_count(cover: &Cover) -> usize {
    if cover.output_part_size() == 1 || cover.set_size() < cover.output_part_size() {
        return 0;
    }

    let first_output_column = cover.set_size() - cover.output_part_size();
    cover
        .cubes()
        .iter()
        .map(|cube| {
            cube.columns()
                .filter(|column| *column >= first_output_column)
                .count()
        })
        .sum()
}

fn literal_weight(cube: &Cube, set_size: usize, output_part_size: usize) -> usize {
    let output_literals = if output_part_size > 1 && set_size >= output_part_size {
        let first_output_column = set_size - output_part_size;
        cube.columns()
            .filter(|column| *column >= first_output_column)
            .count()
    } else {
        0
    };

    let base = set_size.saturating_sub(cube.literal_count());
    base + output_literals
}

fn output_part_size(covers: &[Cover]) -> Result<usize, MinimizeError> {
    covers
        .first()
        .map(|cover| {
            validate_output_part(cover)?;
            Ok(cover.output_part_size())
        })
        .unwrap_or(Ok(1))
}

fn output_columns(cover: &Cover) -> Result<Vec<usize>, MinimizeError> {
    validate_output_part(cover)?;
    Ok((cover.set_size() - cover.output_part_size()..cover.set_size()).collect())
}

fn validate_output_part(cover: &Cover) -> Result<(), MinimizeError> {
    if cover.output_part_size() == 0 || cover.output_part_size() > cover.set_size() {
        return Err(MinimizeError::InvalidOutputPart {
            set_size: cover.set_size(),
            output_part_size: cover.output_part_size(),
        });
    }
    Ok(())
}

fn output_mask(cover: &Cover) -> Result<Cube, MinimizeError> {
    Ok(Cube::from_columns(output_columns(cover)?))
}

fn cof_output(cover: &Cover, output_column: usize) -> Cover {
    let mask = output_mask(cover).expect("validated output mask");
    let mut result = Cover::with_output_part_size(cover.set_size(), cover.output_part_size());
    for cube in cover
        .cubes()
        .iter()
        .filter(|cube| cube.contains(output_column))
    {
        let mut cofactored = cube.union(&mask);
        cofactored.set_prime(false);
        result.push(cofactored);
    }
    result
}

fn uncof_output(cover: Cover, output_column: usize) -> Result<Cover, MinimizeError> {
    let mask = output_mask(&cover)?;
    let mut result = Cover::with_output_part_size(cover.set_size(), cover.output_part_size());
    for cube in cover.cubes {
        let mut restored = cube.difference(&mask);
        restored.insert(output_column);
        result.push(restored);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pla as pla_format;

    #[derive(Default)]
    struct RecordingBackend {
        calls: Vec<&'static str>,
        reduce_outputs: Vec<Cover>,
        gasp_outputs: Vec<Cover>,
        primes: Option<Cover>,
        split: Option<IrredundantSplit>,
        table: Option<PrimeTable>,
        split_outputs: Vec<Pla>,
    }

    impl HeuristicBackend for RecordingBackend {
        fn unwrap_onset(&mut self, cover: Cover) -> Result<Cover, MinimizeError> {
            self.calls.push("unwrap");
            Ok(cover)
        }

        fn expand(&mut self, cover: Cover, _off_set: &Cover) -> Result<Cover, MinimizeError> {
            self.calls.push("expand");
            Ok(cover)
        }

        fn irredundant(
            &mut self,
            cover: Cover,
            _dont_care: &Cover,
        ) -> Result<Cover, MinimizeError> {
            self.calls.push("irredundant");
            Ok(cover)
        }

        fn essential(
            &mut self,
            cover: Cover,
            dont_care: Cover,
        ) -> Result<(Cover, Cover, Cover), MinimizeError> {
            self.calls.push("essential");
            Ok((cover, dont_care, Cover::new(6)))
        }

        fn reduce(&mut self, cover: Cover, _dont_care: &Cover) -> Result<Cover, MinimizeError> {
            self.calls.push("reduce");
            Ok(if self.reduce_outputs.is_empty() {
                cover
            } else {
                self.reduce_outputs.remove(0)
            })
        }

        fn last_gasp(
            &mut self,
            cover: Cover,
            _dont_care: &Cover,
            _off_set: &Cover,
        ) -> Result<Cover, MinimizeError> {
            self.calls.push("last_gasp");
            Ok(if self.gasp_outputs.is_empty() {
                cover
            } else {
                self.gasp_outputs.remove(0)
            })
        }

        fn make_sparse(
            &mut self,
            cover: Cover,
            _original_dont_care: &Cover,
            _off_set: &Cover,
        ) -> Result<Cover, MinimizeError> {
            self.calls.push("make_sparse");
            Ok(cover)
        }
    }

    impl ExactBackend for RecordingBackend {
        fn generate_primes(
            &mut self,
            _on_set: &Cover,
            _dont_care: &Cover,
        ) -> Result<Cover, MinimizeError> {
            self.calls.push("primes");
            Ok(self.primes.take().expect("test primes"))
        }

        fn split_irredundant(
            &mut self,
            _primes: &IndexedCover,
            _dont_care: &Cover,
        ) -> Result<IrredundantSplit, MinimizeError> {
            self.calls.push("split");
            Ok(self.split.take().expect("test split"))
        }

        fn derive_prime_table(
            &mut self,
            _dont_care: &Cover,
            _split: &IrredundantSplit,
        ) -> Result<PrimeTable, MinimizeError> {
            self.calls.push("table");
            Ok(self.table.take().expect("test table"))
        }
    }

    impl OutputPartitionBackend for RecordingBackend {
        fn split_outputs(&mut self, _problem: Pla) -> Result<Vec<Pla>, MinimizeError> {
            self.calls.push("split_outputs");
            Ok(std::mem::take(&mut self.split_outputs))
        }

        fn merge_outputs(&mut self, covers: Vec<Cover>) -> Result<Cover, MinimizeError> {
            self.calls.push("merge_outputs");
            Ok(covers
                .into_iter()
                .reduce(Cover::append)
                .unwrap_or_else(|| Cover::new(6)))
        }
    }

    fn cube(columns: &[usize]) -> Cube {
        Cube::from_columns(columns.iter().copied())
    }

    fn cover(cubes: &[&[usize]]) -> Cover {
        Cover::from_cubes(6, cubes.iter().map(|columns| cube(columns)))
    }

    fn row(columns: &[usize]) -> BTreeSet<usize> {
        columns.iter().copied().collect()
    }

    fn problem(cubes: &[&[usize]]) -> Pla {
        Pla::new(cover(cubes), Cover::new(6), Cover::new(6))
    }

    fn binary_example_layout(input_count: usize, output_count: usize) -> CubeLayout {
        let mut variables = Vec::with_capacity(input_count + 1);
        for input in 0..input_count {
            variables.push(Variable::new(input * 2, input * 2 + 1, false));
        }
        let output_base = input_count * 2;
        variables.push(Variable::new(
            output_base,
            output_base + output_count - 1,
            true,
        ));
        CubeLayout::new(variables).unwrap()
    }

    fn binary_example_problem(parsed: &pla_format::Pla) -> Pla {
        let set_size = parsed.input_count * 2 + parsed.output_count;
        Pla::new(
            binary_example_cover(&parsed.f, parsed, set_size),
            binary_example_cover(&parsed.d, parsed, set_size),
            binary_example_cover(&parsed.r, parsed, set_size),
        )
    }

    fn binary_example_cover(
        cover: &pla_format::Cover,
        parsed: &pla_format::Pla,
        set_size: usize,
    ) -> Cover {
        Cover::from_cubes_with_output_part_size(
            set_size,
            parsed.output_count,
            cover
                .cubes
                .iter()
                .map(|cube| binary_example_cube(cube, parsed)),
        )
    }

    fn binary_example_cube(cube: &pla_format::Cube, parsed: &pla_format::Pla) -> Cube {
        let mut columns = BTreeSet::new();
        for (index, part) in cube.inputs.iter().enumerate() {
            match part {
                pla_format::InputPart::Zero => {
                    columns.insert(index * 2);
                }
                pla_format::InputPart::One => {
                    columns.insert(index * 2 + 1);
                }
                pla_format::InputPart::Dash => {
                    columns.insert(index * 2);
                    columns.insert(index * 2 + 1);
                }
                pla_format::InputPart::Empty => {}
            }
        }

        let output_base = parsed.input_count * 2;
        for (index, selected) in cube.outputs.iter().enumerate() {
            if *selected {
                columns.insert(output_base + index);
            }
        }

        Cube::from_columns(columns)
    }

    fn binary_example_result_pla(original: &pla_format::Pla, cover: &Cover) -> pla_format::Pla {
        pla_format::Pla {
            input_count: original.input_count,
            output_count: original.output_count,
            pla_type: pla_format::F_TYPE,
            input_labels: original.input_labels.clone(),
            output_labels: original.output_labels.clone(),
            phase: original.phase.clone(),
            f: pla_format::Cover {
                cubes: cover
                    .cubes()
                    .iter()
                    .map(|cube| binary_example_result_cube(original, cube))
                    .collect(),
            },
            d: pla_format::Cover::default(),
            r: pla_format::Cover::default(),
        }
    }

    fn binary_example_result_cube(original: &pla_format::Pla, cube: &Cube) -> pla_format::Cube {
        let inputs = (0..original.input_count)
            .map(|index| {
                let has_zero = cube.contains(index * 2);
                let has_one = cube.contains(index * 2 + 1);
                match (has_zero, has_one) {
                    (true, true) => pla_format::InputPart::Dash,
                    (true, false) => pla_format::InputPart::Zero,
                    (false, true) => pla_format::InputPart::One,
                    (false, false) => pla_format::InputPart::Empty,
                }
            })
            .collect();
        let output_base = original.input_count * 2;
        let outputs = (0..original.output_count)
            .map(|index| cube.contains(output_base + index))
            .collect();

        pla_format::Cube { inputs, outputs }
    }

    fn complete_binary_off_set(mut pla: pla_format::Pla) -> pla_format::Pla {
        for input in 0..(1usize << pla.input_count) {
            let inputs = (0..pla.input_count)
                .map(|index| {
                    if (input & (1 << (pla.input_count - index - 1))) == 0 {
                        pla_format::InputPart::Zero
                    } else {
                        pla_format::InputPart::One
                    }
                })
                .collect::<Vec<_>>();

            for output in 0..pla.output_count {
                if binary_pla_cover_has_point(&pla.f, &inputs, output)
                    || binary_pla_cover_has_point(&pla.d, &inputs, output)
                    || binary_pla_cover_has_point(&pla.r, &inputs, output)
                {
                    continue;
                }

                let mut outputs = vec![false; pla.output_count];
                outputs[output] = true;
                pla.r.cubes.push(pla_format::Cube {
                    inputs: inputs.clone(),
                    outputs,
                });
            }
        }

        pla.pla_type |= pla_format::R_TYPE;
        pla
    }

    fn binary_pla_cover_has_point(
        cover: &pla_format::Cover,
        inputs: &[pla_format::InputPart],
        output: usize,
    ) -> bool {
        cover.cubes.iter().any(|cube| {
            cube.outputs.get(output).copied().unwrap_or(false)
                && cube
                    .inputs
                    .iter()
                    .zip(inputs)
                    .all(|(cube_part, point_part)| match cube_part {
                        pla_format::InputPart::Dash => true,
                        pla_format::InputPart::Zero | pla_format::InputPart::One => {
                            cube_part == point_part
                        }
                        pla_format::InputPart::Empty => false,
                    })
        })
    }

    fn layout() -> CubeLayout {
        CubeLayout::new([
            Variable::new(0, 1, false),
            Variable::new(2, 3, false),
            Variable::new(4, 5, true),
        ])
        .unwrap()
    }

    fn cube_columns(cover: &Cover) -> Vec<BTreeSet<usize>> {
        cover
            .cubes()
            .iter()
            .map(|cube| cube.columns().collect())
            .collect()
    }

    #[test]
    fn fast_joint_runs_espresso_loop_until_cube_count_stabilizes() {
        let mut backend = RecordingBackend {
            reduce_outputs: vec![cover(&[&[0], &[1]]), cover(&[&[0]])],
            gasp_outputs: vec![cover(&[&[0]])],
            ..RecordingBackend::default()
        };
        let options = HeuristicOptions {
            unwrap_onset: false,
            skip_make_sparse: true,
            ..HeuristicOptions::default()
        };

        let result = heuristic_minimize(
            &mut backend,
            cover(&[&[0], &[1], &[2]]),
            Cover::new(6),
            Cover::new(6),
            options,
        )
        .unwrap();

        assert_eq!(result, cover(&[&[0]]));
        assert_eq!(
            backend.calls,
            vec![
                "expand",
                "irredundant",
                "essential",
                "reduce",
                "expand",
                "irredundant",
                "reduce",
                "expand",
                "irredundant",
                "reduce",
                "expand",
                "irredundant",
                "last_gasp"
            ]
        );
    }

    #[test]
    fn exact_joint_solves_prime_table_without_heuristic_substitution() {
        let primes = cover(&[&[0], &[0, 1, 2], &[2]]);
        let split = IrredundantSplit {
            essential: IndexedCover::from_indexed_cubes(6, 1, [IndexedCube::new(2, cube(&[2]))]),
            totally_redundant: IndexedCover::empty(6, 1),
            partially_redundant: IndexedCover::from_indexed_cubes(
                6,
                1,
                [
                    IndexedCube::new(0, cube(&[0])),
                    IndexedCube::new(1, cube(&[0, 1, 2])),
                ],
            ),
        };
        let mut backend = RecordingBackend {
            primes: Some(primes),
            split: Some(split),
            table: Some(PrimeTable::new([row(&[0, 1])])),
            ..RecordingBackend::default()
        };

        let result = exact_minimize(
            &mut backend,
            &Cover::new(6),
            &Cover::new(6),
            None,
            ExactOptions::exact_literal_cover(),
        )
        .unwrap();

        assert_eq!(backend.calls, vec!["primes", "split", "table"]);
        assert_eq!(result.selected_prime_indices, vec![1]);
        assert_eq!(result.cover, cover(&[&[2], &[0, 1, 2]]));
        assert!(!result.sparse_cleanup_applied);
    }

    #[test]
    fn exact_heuristic_mode_uses_greedy_cover_only_when_requested() {
        let selected = minimum_cover(&[row(&[0, 1]), row(&[1, 2])], None, true).unwrap();

        assert!(!selected.is_empty());
    }

    #[test]
    fn independent_modes_split_and_merge_outputs() {
        let mut backend = RecordingBackend {
            split_outputs: vec![problem(&[&[0]]), problem(&[&[1]])],
            ..RecordingBackend::default()
        };

        let result = minimize(
            &mut backend,
            problem(&[&[0], &[1]]),
            MinimizeOptions {
                mode: MinimizeMode::FastIndependentOutput,
                heuristic: HeuristicOptions {
                    single_expand: true,
                    unwrap_onset: false,
                    ..HeuristicOptions::default()
                },
                ..MinimizeOptions::default()
            },
        )
        .unwrap();

        assert_eq!(result.cover, cover(&[&[0], &[1]]));
        assert_eq!(
            backend.calls,
            vec![
                "split_outputs",
                "expand",
                "irredundant",
                "expand",
                "irredundant",
                "merge_outputs"
            ]
        );
    }

    #[test]
    fn native_output_split_cofactors_each_output_and_clears_prime_flags() {
        let mut backend = NativeMinimizeBackend::new(layout());
        let mut first = cube(&[0, 2, 4]);
        first.set_prime(true);
        let problem = Pla::new(
            Cover::from_cubes_with_output_part_size(
                6,
                2,
                [first, cube(&[1, 3, 5]), cube(&[0, 2, 4, 5])],
            ),
            Cover::from_cubes_with_output_part_size(6, 2, [cube(&[0, 3, 4])]),
            Cover::from_cubes_with_output_part_size(6, 2, [cube(&[1, 2, 5])]),
        );

        let outputs = backend.split_outputs(problem).unwrap();

        assert_eq!(outputs.len(), 2);
        assert_eq!(
            cube_columns(&outputs[0].on_set),
            vec![row(&[0, 2, 4, 5]), row(&[0, 2, 4, 5])]
        );
        assert_eq!(
            cube_columns(&outputs[1].on_set),
            vec![row(&[1, 3, 4, 5]), row(&[0, 2, 4, 5])]
        );
        assert_eq!(
            cube_columns(&outputs[0].dont_care),
            vec![row(&[0, 3, 4, 5])]
        );
        assert!(outputs[1].dont_care.is_empty());
        assert!(outputs[0].off_set.is_empty());
        assert_eq!(cube_columns(&outputs[1].off_set), vec![row(&[1, 2, 4, 5])]);
        assert!(!outputs[0].on_set.cubes()[0].is_prime());
    }

    #[test]
    fn native_output_merge_reinserts_one_output_part_per_subcover() {
        let mut backend = NativeMinimizeBackend::new(layout());
        let covers = vec![
            Cover::from_cubes_with_output_part_size(6, 2, [cube(&[0, 2, 4, 5])]),
            Cover::from_cubes_with_output_part_size(6, 2, [cube(&[1, 3, 4, 5])]),
        ];

        let merged = backend.merge_outputs(covers).unwrap();

        assert_eq!(merged.output_part_size(), 2);
        assert_eq!(
            cube_columns(&merged),
            vec![row(&[0, 2, 4]), row(&[1, 3, 5])]
        );
    }

    #[test]
    fn missing_native_ports_report_the_first_required_lower_port() {
        let error =
            native_minimize_unavailable(problem(&[&[0]]), MinimizeMode::ExactJoint).unwrap_err();

        assert_eq!(
            error,
            MinimizeError::MissingLowerPort(LowerPort::GeneratePrimes)
        );
    }

    #[test]
    fn native_expand_preserves_off_set_orthogonality_and_covers_subsumed_cubes() {
        let layout = layout();
        let on_set = cover(&[&[0, 2, 4], &[0, 3, 5]]);
        let off_set = cover(&[&[1, 2, 4]]);

        let report =
            expand_native(on_set, &off_set, &layout, ExpandOptions::all_variables()).unwrap();

        assert_eq!(report.expanded_cubes, 1);
        assert_eq!(report.cover.len(), 1);
        assert_eq!(cube_columns(&report.cover), vec![row(&[0, 2, 3, 4, 5])]);
        assert!(report.cover.cubes()[0].is_prime());
    }

    #[test]
    fn native_all_primes_generates_prime_expansions_for_nonprime_cubes() {
        let layout = layout();
        let on_set = cover(&[&[0, 2, 4]]);
        let off_set = cover(&[&[1, 2, 4]]);

        let primes = all_primes_native(&on_set, &off_set, &layout).unwrap();

        assert_eq!(cube_columns(&primes), vec![row(&[0, 2, 3, 4, 5])]);
        assert!(primes.cubes()[0].is_prime());
    }

    #[test]
    fn native_essential_moves_uniquely_covering_primes_to_dont_care() {
        let layout = layout();
        let on_set = cover(&[&[0, 2, 4], &[0, 2, 4, 5]]);

        let (remaining, dont_care, essential) =
            essential_native(on_set, Cover::new(6), &layout).unwrap();

        assert_eq!(cube_columns(&remaining), vec![row(&[0, 2, 4])]);
        assert_eq!(cube_columns(&essential), vec![row(&[0, 2, 4, 5])]);
        assert_eq!(cube_columns(&dont_care), vec![row(&[0, 2, 4, 5])]);
    }

    #[test]
    fn native_reduce_shrinks_cube_to_points_not_covered_elsewhere() {
        let layout = layout();
        let on_set = cover(&[&[0, 1, 2, 3, 4, 5], &[1, 2, 3, 4, 5]]);

        let reduced = reduce_native(on_set, &Cover::new(6), &layout, true).unwrap();

        assert_eq!(cube_columns(&reduced), vec![row(&[0, 2, 3, 4, 5])]);
    }

    #[test]
    fn native_reduce_gasp_preserves_order_and_marks_reduced_cubes_nonprime() {
        let layout = layout();
        let on_set = cover(&[&[0, 1, 2, 3, 4, 5], &[1, 2, 3, 4, 5]]);

        let reduced = reduce_gasp_native(&on_set, &Cover::new(6), &layout).unwrap();

        assert_eq!(
            cube_columns(&reduced),
            vec![row(&[0, 2, 3, 4, 5]), row(&[1, 2, 3, 4, 5])]
        );
        assert!(!reduced.cubes()[0].is_prime());
        assert!(reduced.cubes()[1].is_prime());
    }

    #[test]
    fn native_super_gasp_uses_native_path_instead_of_missing_port() {
        let layout = layout();
        let mut backend = NativeMinimizeBackend::new(layout);
        let on_set = cover(&[&[0, 2, 4], &[0, 3, 4]]);
        let off_set = cover(&[&[1, 2, 4]]);

        let result =
            HeuristicBackend::super_gasp(&mut backend, on_set, &Cover::new(6), &off_set).unwrap();

        assert!(!result.is_empty());
        assert!(result.len() <= 2);
    }

    #[test]
    fn native_last_gasp_uses_native_gasp_path_instead_of_missing_port() {
        let layout = layout();
        let mut backend = NativeMinimizeBackend::new(layout);
        let on_set = cover(&[&[0, 2, 4], &[0, 3, 4]]);
        let off_set = cover(&[&[1, 2, 4]]);

        let result =
            HeuristicBackend::last_gasp(&mut backend, on_set.clone(), &Cover::new(6), &off_set)
                .unwrap();

        assert!(!result.is_empty());
        assert!(result.len() <= on_set.len());
    }

    #[test]
    fn native_irredundant_removes_cubes_covered_by_essential_cubes() {
        let layout = layout();
        let on_set = cover(&[&[0, 1, 2, 3, 4, 5], &[0, 2, 4]]);

        let reduced = irredundant_native(on_set, &Cover::new(6), &layout).unwrap();

        assert_eq!(cube_columns(&reduced), vec![row(&[0, 1, 2, 3, 4, 5])]);
    }

    #[test]
    fn native_mv_reduce_removes_redundant_sparse_output_parts() {
        let layout = layout();
        let on_set = cover(&[&[0, 2, 4, 5], &[0, 2, 4]]);

        let reduced = mv_reduce_native(on_set, &Cover::new(6), &layout).unwrap();

        assert_eq!(cube_columns(&reduced), vec![row(&[0, 2, 4, 5])]);
    }

    #[test]
    fn native_make_sparse_uses_sparse_reduce_cleanup() {
        let layout = layout();
        let mut backend = NativeMinimizeBackend::new(layout);
        let on_set = cover(&[&[0, 2, 4, 5], &[0, 2, 4]]);

        let reduced = backend
            .make_sparse(on_set, &Cover::new(6), &Cover::new(6))
            .unwrap();

        assert_eq!(cube_columns(&reduced), vec![row(&[0, 2, 4, 5])]);
    }

    #[test]
    fn original_example_fast_joint_outputs_equivalent_cover() {
        let fixtures = [
            (
                "simple_binary_joint",
                ".i 2\n.o 1\n.type fr\n00 1\n01 1\n10 0\n11 0\n.e\n",
            ),
            (
                "examples/indust/dc1",
                include_str!("../../LogicSynthesis/espresso/examples/indust/dc1"),
            ),
            (
                "examples/math/rd53",
                include_str!("../../LogicSynthesis/espresso/examples/math/rd53"),
            ),
        ];

        for (name, text) in fixtures {
            let original =
                complete_binary_off_set(pla_format::Pla::parse(text).unwrap_or_else(|error| {
                    panic!("{name} should parse as a supported binary PLA: {error}")
                }));
            let layout = binary_example_layout(original.input_count, original.output_count);
            let mut backend = NativeMinimizeBackend::new(layout);
            let result = minimize(
                &mut backend,
                binary_example_problem(&original),
                MinimizeOptions {
                    mode: MinimizeMode::FastJoint,
                    heuristic: HeuristicOptions {
                        unwrap_onset: false,
                        ..HeuristicOptions::logic_friday_fast()
                    },
                    exact: ExactOptions::exact_cover(),
                },
            )
            .unwrap_or_else(|error| panic!("{name} should minimize with native Rust: {error}"));

            let minimized = binary_example_result_pla(&original, &result.cover);
            let verify = original.verify_minimized(&minimized);
            assert!(
                verify.is_equivalent(),
                "{name} minimized cover should be equivalent: {verify:?}"
            );
            assert_eq!(minimized.input_count, original.input_count);
            assert_eq!(minimized.output_count, original.output_count);
            assert!(
                result.cover.cost().cubes <= original.f.cubes.len(),
                "{name} should not increase product term count"
            );
        }
    }

    #[test]
    fn original_random_example_parses_output_dont_cares_like_fd_input() {
        let random = pla_format::Pla::parse(include_str!(
            "../../LogicSynthesis/espresso/examples/random/fout"
        ))
        .unwrap();

        assert_eq!(random.input_count, 6);
        assert_eq!(random.output_count, 10);
        assert!(!random.f.cubes.is_empty());
        assert!(!random.d.cubes.is_empty());
        assert!(random.r.cubes.is_empty());
    }

    #[test]
    fn native_split_and_prime_table_classify_essential_and_redundant_primes() {
        let layout = layout();
        let primes = IndexedCover::from_cover(cover(&[&[0, 2, 4], &[0, 2, 4, 5], &[1, 2, 4]]));

        let split = split_irredundant_native(&primes, &Cover::new(6), &layout).unwrap();
        let table = derive_prime_table_native(&Cover::new(6), &split, &layout).unwrap();

        assert_eq!(split.essential.cubes().len(), 2);
        assert_eq!(
            split
                .essential
                .cubes()
                .iter()
                .map(IndexedCube::index)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(split.totally_redundant.cubes().len(), 1);
        assert_eq!(split.totally_redundant.cubes()[0].index(), 0);
        assert!(table.rows().is_empty());
    }

    #[test]
    fn no_legacy_c_abi_or_bead_metadata_tokens_are_present() {
        let source = include_str!("minimize.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-")));
    }
}
