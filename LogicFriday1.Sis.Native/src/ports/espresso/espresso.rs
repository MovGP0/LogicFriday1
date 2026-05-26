//! Native Espresso minimization driver.
//!
//! The original routine coordinates the setup, expand, irredundant, essential,
//! reduce, gasp, and sparse-cleanup passes. Those passes are separate porting
//! units, so this module keeps the orchestration in native Rust and delegates
//! cover transformations through an explicit backend trait.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EspressoOptions
{
    pub trace: bool,
    pub remove_essential: bool,
    pub single_expand: bool,
    pub use_super_gasp: bool,
    pub recompute_onset: bool,
    pub unwrap_onset: bool,
    pub force_irredundant: bool,
    pub skip_make_sparse: bool,
}

impl EspressoOptions
{
    pub const fn sis_defaults() -> Self
    {
        Self
        {
            trace: false,
            remove_essential: true,
            single_expand: false,
            use_super_gasp: false,
            recompute_onset: false,
            unwrap_onset: true,
            force_irredundant: true,
            skip_make_sparse: false,
        }
    }
}

impl Default for EspressoOptions
{
    fn default() -> Self
    {
        Self::sis_defaults()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CoverCost
{
    pub cubes: usize,
    pub input_literals: usize,
    pub output_literals: usize,
    pub total_literals: usize,
}

impl CoverCost
{
    pub const fn new(
        cubes: usize,
        input_literals: usize,
        output_literals: usize,
        total_literals: usize,
    ) -> Self
    {
        Self
        {
            cubes,
            input_literals,
            output_literals,
            total_literals,
        }
    }

    fn is_outer_iteration_better_than(self, other: Self) -> bool
    {
        self.cubes < other.cubes
            || (self.cubes == other.cubes && self.total_literals < other.total_literals)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube
{
    columns: BTreeSet<usize>,
    prime: bool,
}

impl Cube
{
    pub fn empty() -> Self
    {
        Self
        {
            columns: BTreeSet::new(),
            prime: false,
        }
    }

    pub fn from_columns(columns: impl IntoIterator<Item = usize>) -> Self
    {
        Self
        {
            columns: columns.into_iter().collect(),
            prime: false,
        }
    }

    pub fn len(&self) -> usize
    {
        self.columns.len()
    }

    pub fn columns(&self) -> impl Iterator<Item = usize> + '_
    {
        self.columns.iter().copied()
    }

    pub fn is_prime(&self) -> bool
    {
        self.prime
    }

    pub fn set_prime(&mut self, value: bool)
    {
        self.prime = value;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover
{
    set_size: usize,
    output_part_size: usize,
    cubes: Vec<Cube>,
}

impl Cover
{
    pub fn new(set_size: usize) -> Self
    {
        Self
        {
            set_size,
            output_part_size: 1,
            cubes: Vec::new(),
        }
    }

    pub fn with_output_part_size(set_size: usize, output_part_size: usize) -> Self
    {
        Self
        {
            set_size,
            output_part_size: output_part_size.max(1),
            cubes: Vec::new(),
        }
    }

    pub fn from_cubes(set_size: usize, cubes: impl IntoIterator<Item = Cube>) -> Self
    {
        let mut cover = Self::new(set_size);
        for cube in cubes
        {
            cover.push(cube);
        }

        cover
    }

    pub fn from_cubes_with_output_part_size(
        set_size: usize,
        output_part_size: usize,
        cubes: impl IntoIterator<Item = Cube>,
    ) -> Self
    {
        let mut cover = Self::with_output_part_size(set_size, output_part_size);
        for cube in cubes
        {
            cover.push(cube);
        }

        cover
    }

    pub fn push(&mut self, cube: Cube)
    {
        debug_assert!(cube.columns().all(|column| column < self.set_size));
        self.cubes.push(cube);
    }

    pub fn len(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }

    pub fn set_size(&self) -> usize
    {
        self.set_size
    }

    pub fn output_part_size(&self) -> usize
    {
        self.output_part_size
    }

    pub fn cubes(&self) -> &[Cube]
    {
        &self.cubes
    }

    pub fn cubes_mut(&mut self) -> &mut [Cube]
    {
        &mut self.cubes
    }

    pub fn clear_prime_flags(&mut self)
    {
        for cube in &mut self.cubes
        {
            cube.set_prime(false);
        }
    }

    pub fn append(mut self, mut other: Self) -> Self
    {
        debug_assert_eq!(self.set_size, other.set_size);
        self.cubes.append(&mut other.cubes);
        self
    }

    pub fn cost(&self) -> CoverCost
    {
        let total_literals: usize = self.cubes.iter().map(Cube::len).sum();
        let output_literals = output_literal_count(self);
        CoverCost::new(
            self.cubes.len(),
            total_literals.saturating_sub(output_literals),
            output_literals,
            total_literals,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EspressoStep
{
    SaveOriginal,
    CopyDontCare,
    SimplifyRecomputedOnset,
    UnwrapOnset,
    ClearPrimeFlags,
    Expand,
    Irredundant,
    Essential,
    Reduce,
    LastGasp,
    SuperGasp,
    AppendEssential,
    MakeSparse,
    RetryWithoutUnwrap,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoPlan
{
    pub options: EspressoOptions,
    pub steps: Vec<EspressoStep>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EspressoError
{
    MissingSisPorts
    {
        operation: &'static str,
    },
}

impl fmt::Display for EspressoError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingSisPorts
            {
                operation,
            } => write!(
                formatter,
                "{operation} requires native Rust Espresso passes that are not available yet"
            ),
        }
    }
}

impl Error for EspressoError {}

pub trait EspressoBackend
{
    fn simplify_recomputed_onset(&mut self, cover: Cover) -> Result<Cover, EspressoError>;

    fn unwrap_onset(&mut self, cover: Cover) -> Result<Cover, EspressoError>;

    fn expand(&mut self, cover: Cover, off_set: &Cover) -> Result<Cover, EspressoError>;

    fn irredundant(&mut self, cover: Cover, dont_care: &Cover) -> Result<Cover, EspressoError>;

    fn essential(
        &mut self,
        cover: Cover,
        dont_care: Cover,
    ) -> Result<(Cover, Cover, Cover), EspressoError>;

    fn reduce(&mut self, cover: Cover, dont_care: &Cover) -> Result<Cover, EspressoError>;

    fn last_gasp(
        &mut self,
        cover: Cover,
        dont_care: &Cover,
        off_set: &Cover,
    ) -> Result<Cover, EspressoError>;

    fn super_gasp(
        &mut self,
        cover: Cover,
        dont_care: &Cover,
        off_set: &Cover,
    ) -> Result<Cover, EspressoError>;

    fn make_sparse(
        &mut self,
        cover: Cover,
        original_dont_care: &Cover,
        off_set: &Cover,
    ) -> Result<Cover, EspressoError>;
}

pub fn plan_espresso(on_set: &Cover, options: EspressoOptions) -> EspressoPlan
{
    let mut steps = vec![EspressoStep::SaveOriginal, EspressoStep::CopyDontCare];

    if options.recompute_onset
    {
        steps.push(EspressoStep::SimplifyRecomputedOnset);
    }
    if should_unwrap_onset(on_set, options)
    {
        steps.push(EspressoStep::UnwrapOnset);
    }

    steps.extend([
        EspressoStep::ClearPrimeFlags,
        EspressoStep::Expand,
        EspressoStep::Irredundant,
    ]);

    if !options.single_expand
    {
        if options.remove_essential
        {
            steps.push(EspressoStep::Essential);
        }

        steps.extend([
            EspressoStep::Reduce,
            EspressoStep::Expand,
            EspressoStep::Irredundant,
        ]);
        steps.push(if options.use_super_gasp
        {
            EspressoStep::SuperGasp
        }
        else
        {
            EspressoStep::LastGasp
        });
        steps.push(EspressoStep::AppendEssential);
    }

    if !options.skip_make_sparse
    {
        steps.push(EspressoStep::MakeSparse);
    }

    steps.push(EspressoStep::RetryWithoutUnwrap);

    EspressoPlan
    {
        options,
        steps,
    }
}

pub fn espresso<B>(
    backend: &mut B,
    on_set: Cover,
    dont_care: Cover,
    off_set: Cover,
    options: EspressoOptions,
) -> Result<Cover, EspressoError>
where
    B: EspressoBackend,
{
    espresso_inner(backend, on_set, dont_care, off_set, options)
}

fn espresso_inner<B>(
    backend: &mut B,
    original_on_set: Cover,
    original_dont_care: Cover,
    off_set: Cover,
    mut options: EspressoOptions,
) -> Result<Cover, EspressoError>
where
    B: EspressoBackend,
{
    loop
    {
        let saved_on_set = original_on_set.clone();
        let mut current = original_on_set.clone();
        let mut scratch_dont_care = original_dont_care.clone();

        if options.recompute_onset
        {
            current = backend.simplify_recomputed_onset(current)?;
        }
        if should_unwrap_onset(&current, options)
        {
            current = backend.unwrap_onset(current)?;
        }

        current.clear_prime_flags();
        current = backend.expand(current, &off_set)?;
        current = backend.irredundant(current, &scratch_dont_care)?;

        if !options.single_expand
        {
            let essential = if options.remove_essential
            {
                let (reduced_current, reduced_dont_care, essential) =
                    backend.essential(current, scratch_dont_care)?;
                current = reduced_current;
                scratch_dont_care = reduced_dont_care;
                essential
            }
            else
            {
                Cover::new(current.set_size())
            };

            current = iterate_espresso_loop(backend, current, &scratch_dont_care, &off_set, options)?;
            current = current.append(essential);
        }

        if !options.skip_make_sparse
        {
            current = backend.make_sparse(current, &original_dont_care, &off_set)?;
        }

        if saved_on_set.len() < current.len() && options.unwrap_onset
        {
            options.unwrap_onset = false;
            continue;
        }

        if saved_on_set.len() < current.len()
        {
            return Ok(saved_on_set);
        }

        return Ok(current);
    }
}

fn iterate_espresso_loop<B>(
    backend: &mut B,
    mut current: Cover,
    dont_care: &Cover,
    off_set: &Cover,
    options: EspressoOptions,
) -> Result<Cover, EspressoError>
where
    B: EspressoBackend,
{
    let mut cost = current.cost();

    loop
    {
        loop
        {
            let best_cost = cost;
            current = backend.reduce(current, dont_care)?;
            current = backend.expand(current, off_set)?;
            current = backend.irredundant(current, dont_care)?;
            cost = current.cost();

            if cost.cubes >= best_cost.cubes
            {
                break;
            }
        }

        let best_cost = cost;
        current = if options.use_super_gasp
        {
            backend.super_gasp(current, dont_care, off_set)?
        }
        else
        {
            backend.last_gasp(current, dont_care, off_set)?
        };
        cost = current.cost();

        if options.use_super_gasp && cost.cubes >= best_cost.cubes
        {
            break;
        }
        if !cost.is_outer_iteration_better_than(best_cost)
        {
            break;
        }
    }

    Ok(current)
}

pub fn native_espresso_cover_bound_minimize() -> Result<Cover, EspressoError>
{
    Err(EspressoError::MissingSisPorts
    {
        operation: "espresso",
    })
}

fn should_unwrap_onset(cover: &Cover, options: EspressoOptions) -> bool
{
    if !options.unwrap_onset || cover.output_part_size() <= 1
    {
        return false;
    }

    let cost = cover.cost();
    cost.output_literals != cost.cubes * cover.output_part_size() && cost.output_literals < 5000
}

fn output_literal_count(cover: &Cover) -> usize
{
    if cover.output_part_size() == 1 || cover.set_size() < cover.output_part_size()
    {
        return 0;
    }

    let first_output_column = cover.set_size() - cover.output_part_size();
    cover
        .cubes()
        .iter()
        .map(|cube| cube.columns().filter(|column| *column >= first_output_column).count())
        .sum()
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Default)]
    struct RecordingBackend
    {
        calls: Vec<&'static str>,
        reduce_outputs: Vec<Cover>,
        gasp_outputs: Vec<Cover>,
        unwrap_output: Option<Cover>,
        sparse_output: Option<Cover>,
    }

    impl EspressoBackend for RecordingBackend
    {
        fn simplify_recomputed_onset(&mut self, cover: Cover) -> Result<Cover, EspressoError>
        {
            self.calls.push("simplify");
            Ok(cover)
        }

        fn unwrap_onset(&mut self, cover: Cover) -> Result<Cover, EspressoError>
        {
            self.calls.push("unwrap");
            Ok(self.unwrap_output.take().unwrap_or(cover))
        }

        fn expand(&mut self, cover: Cover, _off_set: &Cover) -> Result<Cover, EspressoError>
        {
            self.calls.push("expand");
            Ok(cover)
        }

        fn irredundant(
            &mut self,
            cover: Cover,
            _dont_care: &Cover,
        ) -> Result<Cover, EspressoError>
        {
            self.calls.push("irredundant");
            Ok(cover)
        }

        fn essential(
            &mut self,
            cover: Cover,
            dont_care: Cover,
        ) -> Result<(Cover, Cover, Cover), EspressoError>
        {
            self.calls.push("essential");
            Ok((cover, dont_care, Cover::new(6)))
        }

        fn reduce(&mut self, cover: Cover, _dont_care: &Cover) -> Result<Cover, EspressoError>
        {
            self.calls.push("reduce");
            Ok(if self.reduce_outputs.is_empty()
            {
                cover
            }
            else
            {
                self.reduce_outputs.remove(0)
            })
        }

        fn last_gasp(
            &mut self,
            cover: Cover,
            _dont_care: &Cover,
            _off_set: &Cover,
        ) -> Result<Cover, EspressoError>
        {
            self.calls.push("last_gasp");
            Ok(if self.gasp_outputs.is_empty()
            {
                cover
            }
            else
            {
                self.gasp_outputs.remove(0)
            })
        }

        fn super_gasp(
            &mut self,
            cover: Cover,
            _dont_care: &Cover,
            _off_set: &Cover,
        ) -> Result<Cover, EspressoError>
        {
            self.calls.push("super_gasp");
            Ok(if self.gasp_outputs.is_empty()
            {
                cover
            }
            else
            {
                self.gasp_outputs.remove(0)
            })
        }

        fn make_sparse(
            &mut self,
            cover: Cover,
            _original_dont_care: &Cover,
            _off_set: &Cover,
        ) -> Result<Cover, EspressoError>
        {
            self.calls.push("make_sparse");
            Ok(self.sparse_output.take().unwrap_or(cover))
        }
    }

    fn cube(columns: &[usize]) -> Cube
    {
        Cube::from_columns(columns.iter().copied())
    }

    fn cover(cubes: &[&[usize]]) -> Cover
    {
        Cover::from_cubes(6, cubes.iter().map(|columns| cube(columns)))
    }

    fn mv_cover(cubes: &[&[usize]]) -> Cover
    {
        Cover::from_cubes_with_output_part_size(6, 2, cubes.iter().map(|columns| cube(columns)))
    }

    #[test]
    fn plan_includes_setup_iteration_gasp_and_sparse_steps()
    {
        let input = mv_cover(&[&[0, 4], &[1, 5]]);
        let options = EspressoOptions
        {
            recompute_onset: true,
            ..EspressoOptions::default()
        };

        let plan = plan_espresso(&input, options);

        assert_eq!(plan.options, options);
        assert!(plan.steps.contains(&EspressoStep::SimplifyRecomputedOnset));
        assert!(plan.steps.contains(&EspressoStep::UnwrapOnset));
        assert!(plan.steps.contains(&EspressoStep::Essential));
        assert!(plan.steps.contains(&EspressoStep::LastGasp));
        assert!(plan.steps.contains(&EspressoStep::MakeSparse));
    }

    #[test]
    fn single_expand_runs_only_initial_expand_irredundant_and_sparse()
    {
        let mut backend = RecordingBackend::default();
        let options = EspressoOptions
        {
            single_expand: true,
            unwrap_onset: false,
            ..EspressoOptions::default()
        };

        let result = espresso(
            &mut backend,
            cover(&[&[0, 1], &[2, 3]]),
            Cover::new(6),
            Cover::new(6),
            options,
        )
        .unwrap();

        assert_eq!(result, cover(&[&[0, 1], &[2, 3]]));
        assert_eq!(backend.calls, vec!["expand", "irredundant", "make_sparse"]);
    }

    #[test]
    fn full_driver_runs_inner_loop_until_cube_count_stabilizes()
    {
        let mut backend = RecordingBackend
        {
            reduce_outputs: vec![cover(&[&[0], &[1]]), cover(&[&[0]])],
            gasp_outputs: vec![cover(&[&[0]])],
            ..RecordingBackend::default()
        };
        let options = EspressoOptions
        {
            unwrap_onset: false,
            skip_make_sparse: true,
            ..EspressoOptions::default()
        };

        let result = espresso(
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
    fn super_gasp_stops_when_cube_count_does_not_improve()
    {
        let mut backend = RecordingBackend::default();
        let options = EspressoOptions
        {
            unwrap_onset: false,
            use_super_gasp: true,
            skip_make_sparse: true,
            ..EspressoOptions::default()
        };

        espresso(
            &mut backend,
            cover(&[&[0], &[1]]),
            Cover::new(6),
            Cover::new(6),
            options,
        )
        .unwrap();

        assert_eq!(backend.calls.last(), Some(&"super_gasp"));
    }

    #[test]
    fn retry_without_unwrap_returns_original_when_unwrapped_result_is_larger()
    {
        let mut backend = RecordingBackend
        {
            unwrap_output: Some(mv_cover(&[&[0, 4], &[1, 4], &[2, 5]])),
            ..RecordingBackend::default()
        };

        let result = espresso(
            &mut backend,
            mv_cover(&[&[0, 4], &[1, 5]]),
            Cover::new(6),
            Cover::new(6),
            EspressoOptions::default(),
        )
        .unwrap();

        assert_eq!(result, mv_cover(&[&[0, 4], &[1, 5]]));
        assert_eq!(backend.calls.iter().filter(|call| **call == "unwrap").count(), 1);
    }

    #[test]
    fn clears_prime_flags_before_initial_expand()
    {
        let mut backend = RecordingBackend::default();
        let mut first = cube(&[0, 1]);
        first.set_prime(true);

        let result = espresso(
            &mut backend,
            Cover::from_cubes(6, [first]),
            Cover::new(6),
            Cover::new(6),
            EspressoOptions
            {
                single_expand: true,
                skip_make_sparse: true,
                unwrap_onset: false,
                ..EspressoOptions::default()
            },
        )
        .unwrap();

        assert!(!result.cubes()[0].is_prime());
    }

    #[test]
    fn missing_cover_ports_are_explicit()
    {
        assert_eq!(
            native_espresso_cover_bound_minimize(),
            Err(EspressoError::MissingSisPorts
            {
                operation: "espresso",
            })
        );
    }

    #[test]
    fn no_disallowed_porting_tokens_are_present()
    {
        let text = include_str!("espresso.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday1", "-")));
    }
}
