//! Native Rust model for `sis/minimize/minimize.c`.
//!
//! The C unit is the no-complement minimization driver. It selects between
//! `nocomp`, `snocomp`, and `dcsimp`, drives the Espresso-style reduction /
//! expansion passes, and contains several reduced-offset helpers. Most of the
//! full SIS cover machinery is still represented by other porting units, so
//! this module keeps the driver policy and the self-contained unate-complement
//! routines in native Rust and delegates cover mutation to an explicit backend.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinimizeType {
    NoComp,
    SinglePassNoComp,
    DcSimplify,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MinimizeOptions {
    pub trace: bool,
    pub remove_essential: bool,
    pub single_expand: bool,
    pub use_super_gasp: bool,
    pub recompute_onset: bool,
    pub unwrap_onset: bool,
    pub force_irredundant: bool,
    pub skip_make_sparse: bool,
    pub use_random_order: bool,
}

impl MinimizeOptions {
    pub const fn sis_defaults() -> Self {
        Self {
            trace: false,
            remove_essential: true,
            single_expand: false,
            use_super_gasp: false,
            recompute_onset: false,
            unwrap_onset: true,
            force_irredundant: true,
            skip_make_sparse: false,
            use_random_order: false,
        }
    }

    pub const fn for_single_output_binary(mut self) -> Self {
        self.skip_make_sparse = true;
        self.unwrap_onset = false;
        self
    }
}

impl Default for MinimizeOptions {
    fn default() -> Self {
        Self::sis_defaults()
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
    pub const fn new(
        cubes: usize,
        input_literals: usize,
        output_literals: usize,
        total_literals: usize,
    ) -> Self {
        Self {
            cubes,
            input_literals,
            output_literals,
            total_literals,
        }
    }

    pub fn is_literal_better_than(self, other: Self) -> bool {
        self.cubes < other.cubes
            || (self.cubes == other.cubes && self.input_literals < other.input_literals)
    }

    pub fn is_iteration_better_than(self, other: Self) -> bool {
        self.cubes < other.cubes
            || (self.cubes == other.cubes && self.total_literals < other.total_literals)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    columns: BTreeSet<usize>,
}

impl Cube {
    pub fn empty() -> Self {
        Self {
            columns: BTreeSet::new(),
        }
    }

    pub fn from_columns(columns: impl IntoIterator<Item = usize>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    pub fn contains(&self, column: usize) -> bool {
        self.columns.contains(&column)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.columns.is_disjoint(&other.columns)
    }

    pub fn union(&self, other: &Self) -> Self {
        Self::from_columns(self.columns.union(&other.columns).copied())
    }

    pub fn without(&self, column: usize) -> Self {
        Self::from_columns(self.columns.iter().copied().filter(|item| *item != column))
    }

    pub fn columns(&self) -> impl Iterator<Item = usize> + '_ {
        self.columns.iter().copied()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    sf_size: usize,
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(sf_size: usize) -> Self {
        Self {
            sf_size,
            cubes: Vec::new(),
        }
    }

    pub fn from_cubes(sf_size: usize, cubes: impl IntoIterator<Item = Cube>) -> Self {
        let mut cover = Self::new(sf_size);
        for cube in cubes {
            cover.push(cube);
        }
        cover
    }

    pub fn universe(sf_size: usize) -> Self {
        Self::from_cubes(sf_size, [Cube::empty()])
    }

    pub fn push(&mut self, cube: Cube) {
        debug_assert!(cube.columns().all(|column| column < self.sf_size));
        self.cubes.push(cube);
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn sf_size(&self) -> usize {
        self.sf_size
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn append(mut self, mut other: Self) -> Self {
        debug_assert_eq!(self.sf_size, other.sf_size);
        self.cubes.append(&mut other.cubes);
        self
    }

    pub fn reverse_contain(mut self) -> Self {
        self.cubes.sort_by_key(Cube::len);
        let mut retained: Vec<Cube> = Vec::new();
        for cube in self.cubes {
            if !retained
                .iter()
                .any(|existing| existing.columns.is_subset(&cube.columns))
            {
                retained.push(cube);
            }
        }
        self.cubes = retained;
        self
    }

    pub fn cost(&self) -> CoverCost {
        let literals = self.cubes.iter().map(Cube::len).sum();
        CoverCost::new(self.cubes.len(), literals, 0, literals)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinimizeStep {
    SaveOriginal,
    CopyDontCare,
    InitReducedOffset,
    SimplifyRecomputedOnset,
    UnwrapOnset,
    ClearPrimeFlags,
    FirstExpand,
    Expand,
    Essential,
    Reduce,
    Irredundant,
    LastGasp,
    SuperGasp,
    AppendEssential,
    MakeSparse,
    CloseReducedOffset,
    DcSimplify,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimizePlan {
    pub mode: MinimizeType,
    pub options: MinimizeOptions,
    pub steps: Vec<MinimizeStep>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MinimizeError {
    UnsupportedMode,
    MissingSisPorts { operation: &'static str },
}

impl fmt::Display for MinimizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMode => write!(f, "unsupported SIS minimization mode"),
            Self::MissingSisPorts { operation } => write!(
                f,
                "{operation} requires native Rust SIS cover ports that are not available yet"
            ),
        }
    }
}

impl Error for MinimizeError {}

pub trait MinimizeBackend {
    fn simplify_recomputed_onset(&mut self, cover: Cover) -> Result<Cover, MinimizeError>;

    fn unwrap_onset(&mut self, cover: Cover) -> Result<Cover, MinimizeError>;

    fn first_expand(&mut self, cover: Cover) -> Result<Cover, MinimizeError>;

    fn expand(
        &mut self,
        cover: Cover,
        old_cover: Option<Cover>,
        nonsparse: bool,
    ) -> Result<Cover, MinimizeError>;

    fn essential(
        &mut self,
        cover: Cover,
        dont_care: Cover,
    ) -> Result<(Cover, Cover, Cover), MinimizeError>;

    fn reduce(&mut self, cover: Cover, dont_care: &Cover) -> Result<(Cover, Cover), MinimizeError>;

    fn irredundant(&mut self, cover: Cover, dont_care: &Cover) -> Result<Cover, MinimizeError>;

    fn last_gasp(&mut self, cover: Cover, dont_care: &Cover) -> Result<Cover, MinimizeError>;

    fn super_gasp(&mut self, cover: Cover, dont_care: &Cover) -> Result<Cover, MinimizeError>;

    fn make_sparse(&mut self, cover: Cover, dont_care: &Cover) -> Result<Cover, MinimizeError>;

    fn dc_simplify(&mut self, cover: Cover, dont_care: Cover) -> Result<Cover, MinimizeError>;
}

pub fn minimize<B>(
    backend: &mut B,
    onset: Cover,
    dont_care: Cover,
    mode: MinimizeType,
    options: MinimizeOptions,
) -> Result<Cover, MinimizeError>
where
    B: MinimizeBackend,
{
    match mode {
        MinimizeType::NoComp => nocomp(backend, onset, dont_care, options),
        MinimizeType::SinglePassNoComp => snocomp(backend, onset, dont_care, options),
        MinimizeType::DcSimplify => backend.dc_simplify(onset, dont_care),
    }
}

pub fn plan_minimize(
    mode: MinimizeType,
    options: MinimizeOptions,
    binary_variables: usize,
    cube_size: usize,
    cube_count: usize,
) -> MinimizePlan {
    let options = if cube_size.saturating_sub(2 * binary_variables) <= 1 {
        options.for_single_output_binary()
    } else {
        options
    };

    let steps = match mode {
        MinimizeType::NoComp => plan_nocomp(options, cube_count),
        MinimizeType::SinglePassNoComp => vec![
            MinimizeStep::CopyDontCare,
            MinimizeStep::InitReducedOffset,
            MinimizeStep::Essential,
            MinimizeStep::Reduce,
            MinimizeStep::ClearPrimeFlags,
            MinimizeStep::Expand,
            MinimizeStep::Irredundant,
            MinimizeStep::AppendEssential,
            MinimizeStep::CloseReducedOffset,
        ],
        MinimizeType::DcSimplify => vec![MinimizeStep::DcSimplify],
    };

    MinimizePlan {
        mode,
        options,
        steps,
    }
}

fn plan_nocomp(options: MinimizeOptions, cube_count: usize) -> Vec<MinimizeStep> {
    let mut steps = vec![
        MinimizeStep::SaveOriginal,
        MinimizeStep::CopyDontCare,
        MinimizeStep::InitReducedOffset,
    ];

    if options.recompute_onset {
        steps.push(MinimizeStep::SimplifyRecomputedOnset);
    }
    if options.unwrap_onset {
        steps.push(MinimizeStep::UnwrapOnset);
    }

    steps.push(MinimizeStep::ClearPrimeFlags);
    steps.push(if cube_count > 6 {
        MinimizeStep::Expand
    } else {
        MinimizeStep::FirstExpand
    });
    steps.push(MinimizeStep::Irredundant);

    if !options.single_expand {
        if options.remove_essential {
            steps.push(MinimizeStep::Essential);
        }
        steps.extend([
            MinimizeStep::Reduce,
            MinimizeStep::Expand,
            MinimizeStep::Irredundant,
        ]);
        steps.push(if options.use_super_gasp {
            MinimizeStep::SuperGasp
        } else {
            MinimizeStep::LastGasp
        });
        steps.push(MinimizeStep::AppendEssential);
    }

    if !options.skip_make_sparse {
        steps.push(MinimizeStep::MakeSparse);
    }
    steps.push(MinimizeStep::CloseReducedOffset);
    steps
}

pub fn nocomp<B>(
    backend: &mut B,
    onset: Cover,
    dont_care: Cover,
    options: MinimizeOptions,
) -> Result<Cover, MinimizeError>
where
    B: MinimizeBackend,
{
    let original = onset.clone();
    let mut current = onset;

    if options.recompute_onset {
        current = backend.simplify_recomputed_onset(current)?;
    }
    if options.unwrap_onset {
        current = backend.unwrap_onset(current)?;
    }

    current = if current.len() > 6 {
        backend.expand(current, None, false)?
    } else {
        backend.first_expand(current)?
    };
    current = backend.irredundant(current, &dont_care)?;

    if !options.single_expand {
        let essential = if options.remove_essential {
            let (reduced_current, reduced_dont_care, essential) =
                backend.essential(current, dont_care.clone())?;
            current = reduced_current;
            let _ = reduced_dont_care;
            essential
        } else {
            Cover::new(current.sf_size())
        };

        let mut best = current.clone();
        let mut best_cost = best.cost();

        loop {
            let previous_cost = current.cost();
            let (reduced, old_cover) = backend.reduce(current, &dont_care)?;
            current = backend.expand(reduced, Some(old_cover), false)?;
            current = backend.irredundant(current, &dont_care)?;

            let current_cost = current.cost();
            if current_cost.is_literal_better_than(best_cost) {
                best = current.clone();
                best_cost = current_cost;
            }

            if current.len() == 1 || current_cost.cubes >= previous_cost.cubes {
                break;
            }
        }

        let before_gasp = current.cost();
        current = if options.use_super_gasp {
            backend.super_gasp(current, &dont_care)?
        } else {
            backend.last_gasp(current, &dont_care)?
        };

        if current.cost().is_iteration_better_than(before_gasp) {
            best = current.clone();
        }

        current = best.append(essential);
    }

    if !options.skip_make_sparse {
        current = backend.make_sparse(current, &dont_care)?;
    }

    if original.len() < current.len() && !options.unwrap_onset {
        Ok(original)
    } else {
        Ok(current)
    }
}

pub fn snocomp<B>(
    backend: &mut B,
    onset: Cover,
    dont_care: Cover,
    _options: MinimizeOptions,
) -> Result<Cover, MinimizeError>
where
    B: MinimizeBackend,
{
    let (current, dont_care, essential) = backend.essential(onset, dont_care)?;
    let (current, old_cover) = backend.reduce(current, &dont_care)?;
    let current = backend.expand(current, Some(old_cover), false)?;
    let current = if current.len() > 1 {
        backend.irredundant(current, &dont_care)?
    } else {
        current
    };

    Ok(current.append(essential))
}

pub fn ncp_unate_compl(cover: Cover, max_lit: usize) -> Cover {
    ncp_unate_complement(cover, max_lit).reverse_contain()
}

pub fn ncp_unate_complement(cover: Cover, max_lit: usize) -> Cover {
    if cover.is_empty() {
        return Cover::universe(cover.sf_size());
    }
    if max_lit == 0 {
        return Cover::new(cover.sf_size());
    }
    if cover.len() == 1 {
        return Cover::from_cubes(
            cover.sf_size(),
            cover.cubes()[0]
                .columns()
                .map(|column| Cube::from_columns([column])),
        );
    }

    let restricted = minimum_order_restricted_set(&cover);
    if restricted.is_empty() {
        return Cover::new(cover.sf_size());
    }
    if restricted.len() == 1 {
        if restricted.len() > max_lit {
            return Cover::new(cover.sf_size());
        }

        let tail = ncp_unate_complement(
            ncp_abs_covered_many(&cover, &restricted),
            max_lit - restricted.len(),
        );
        return Cover::from_cubes(
            cover.sf_size(),
            tail.cubes().iter().map(|cube| cube.union(&restricted)),
        );
    }

    let pick = ncp_abs_select_restricted(&cover, &restricted);
    let picked = ncp_unate_complement(ncp_abs_covered(&cover, pick), max_lit - 1);
    let picked = Cover::from_cubes(
        cover.sf_size(),
        picked
            .cubes()
            .iter()
            .map(|cube| cube.union(&Cube::from_columns([pick]))),
    );

    let remainder = Cover::from_cubes(
        cover.sf_size(),
        cover.cubes().iter().map(|cube| cube.without(pick)),
    );
    picked.append(ncp_unate_complement(remainder, max_lit))
}

pub fn ncp_abs_covered_many(cover: &Cover, pick_set: &Cube) -> Cover {
    Cover::from_cubes(
        cover.sf_size(),
        cover
            .cubes()
            .iter()
            .filter(|cube| cube.is_disjoint(pick_set))
            .cloned(),
    )
}

pub fn ncp_abs_covered(cover: &Cover, pick: usize) -> Cover {
    Cover::from_cubes(
        cover.sf_size(),
        cover
            .cubes()
            .iter()
            .filter(|cube| !cube.contains(pick))
            .cloned(),
    )
}

pub fn ncp_abs_select_restricted(cover: &Cover, restricted: &Cube) -> usize {
    let mut counts = vec![0usize; cover.sf_size()];
    for cube in cover.cubes() {
        for column in cube.columns() {
            if restricted.contains(column) {
                counts[column] += 1;
            }
        }
    }

    restricted
        .columns()
        .max_by_key(|column| (counts[*column], std::cmp::Reverse(*column)))
        .expect("restricted set is not empty")
}

fn minimum_order_restricted_set(cover: &Cover) -> Cube {
    let Some(minimum_size) = cover.cubes().iter().map(Cube::len).min() else {
        return Cube::empty();
    };

    Cover::from_cubes(
        cover.sf_size(),
        cover
            .cubes()
            .iter()
            .filter(|cube| cube.len() == minimum_size)
            .cloned(),
    )
    .cubes()
    .iter()
    .fold(Cube::empty(), |accumulator, cube| accumulator.union(cube))
}

pub fn native_cover_bound_minimize() -> Result<Cover, MinimizeError> {
    Err(MinimizeError::MissingSisPorts {
        operation: "minimize",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        calls: Vec<&'static str>,
    }

    impl MinimizeBackend for RecordingBackend {
        fn simplify_recomputed_onset(&mut self, cover: Cover) -> Result<Cover, MinimizeError> {
            self.calls.push("simplify");
            Ok(cover)
        }

        fn unwrap_onset(&mut self, cover: Cover) -> Result<Cover, MinimizeError> {
            self.calls.push("unwrap");
            Ok(cover)
        }

        fn first_expand(&mut self, cover: Cover) -> Result<Cover, MinimizeError> {
            self.calls.push("first_expand");
            Ok(cover)
        }

        fn expand(
            &mut self,
            cover: Cover,
            _old_cover: Option<Cover>,
            _nonsparse: bool,
        ) -> Result<Cover, MinimizeError> {
            self.calls.push("expand");
            Ok(cover)
        }

        fn essential(
            &mut self,
            cover: Cover,
            dont_care: Cover,
        ) -> Result<(Cover, Cover, Cover), MinimizeError> {
            self.calls.push("essential");
            Ok((cover, dont_care, Cover::new(4)))
        }

        fn reduce(
            &mut self,
            cover: Cover,
            _dont_care: &Cover,
        ) -> Result<(Cover, Cover), MinimizeError> {
            self.calls.push("reduce");
            Ok((cover.clone(), cover))
        }

        fn irredundant(
            &mut self,
            cover: Cover,
            _dont_care: &Cover,
        ) -> Result<Cover, MinimizeError> {
            self.calls.push("irredundant");
            Ok(cover)
        }

        fn last_gasp(&mut self, cover: Cover, _dont_care: &Cover) -> Result<Cover, MinimizeError> {
            self.calls.push("last_gasp");
            Ok(cover)
        }

        fn super_gasp(&mut self, cover: Cover, _dont_care: &Cover) -> Result<Cover, MinimizeError> {
            self.calls.push("super_gasp");
            Ok(cover)
        }

        fn make_sparse(
            &mut self,
            cover: Cover,
            _dont_care: &Cover,
        ) -> Result<Cover, MinimizeError> {
            self.calls.push("make_sparse");
            Ok(cover)
        }

        fn dc_simplify(&mut self, cover: Cover, _dont_care: Cover) -> Result<Cover, MinimizeError> {
            self.calls.push("dc_simplify");
            Ok(cover)
        }
    }

    fn cube(columns: &[usize]) -> Cube {
        Cube::from_columns(columns.iter().copied())
    }

    fn cover(cubes: &[&[usize]]) -> Cover {
        Cover::from_cubes(4, cubes.iter().map(|columns| cube(columns)))
    }

    #[test]
    fn plans_select_minimize_driver_mode() {
        let plan = plan_minimize(MinimizeType::NoComp, MinimizeOptions::default(), 2, 6, 7);

        assert_eq!(plan.options, MinimizeOptions::default());
        assert!(plan.steps.contains(&MinimizeStep::Expand));
        assert!(plan.steps.contains(&MinimizeStep::Essential));
        assert!(plan.steps.contains(&MinimizeStep::LastGasp));
        assert!(plan.steps.contains(&MinimizeStep::MakeSparse));
    }

    #[test]
    fn single_output_binary_plan_skips_sparse_pass_and_unwrap() {
        let plan = plan_minimize(MinimizeType::NoComp, MinimizeOptions::default(), 3, 7, 4);

        assert!(plan.options.skip_make_sparse);
        assert!(!plan.options.unwrap_onset);
        assert!(plan.steps.contains(&MinimizeStep::FirstExpand));
        assert!(!plan.steps.contains(&MinimizeStep::MakeSparse));
        assert!(!plan.steps.contains(&MinimizeStep::UnwrapOnset));
    }

    #[test]
    fn dispatcher_uses_dc_simplify_backend_for_dcsimplify() {
        let mut backend = RecordingBackend::default();
        let result = minimize(
            &mut backend,
            cover(&[&[0, 1]]),
            Cover::new(4),
            MinimizeType::DcSimplify,
            MinimizeOptions::default(),
        )
        .unwrap();

        assert_eq!(result, cover(&[&[0, 1]]));
        assert_eq!(backend.calls, vec!["dc_simplify"]);
    }

    #[test]
    fn snocomp_runs_single_pass_sequence() {
        let mut backend = RecordingBackend::default();
        snocomp(
            &mut backend,
            cover(&[&[0, 1], &[2]]),
            Cover::new(4),
            MinimizeOptions::default(),
        )
        .unwrap();

        assert_eq!(
            backend.calls,
            vec!["essential", "reduce", "expand", "irredundant"]
        );
    }

    #[test]
    fn unate_complement_of_empty_cover_is_universe_cube() {
        assert_eq!(ncp_unate_compl(Cover::new(4), 3), Cover::universe(4));
    }

    #[test]
    fn unate_complement_of_single_cube_is_demorgan_columns() {
        assert_eq!(ncp_unate_compl(cover(&[&[1, 3]]), 3), cover(&[&[1], &[3]]));
        assert_eq!(ncp_unate_compl(cover(&[&[1, 3]]), 1), cover(&[&[1], &[3]]));
    }

    #[test]
    fn unate_complement_returns_minimal_hitting_sets() {
        let result = ncp_unate_compl(cover(&[&[0, 1], &[1, 2]]), 3);

        assert_eq!(result, cover(&[&[1], &[0, 2]]));
    }

    #[test]
    fn unate_complement_respects_literal_limit() {
        let result = ncp_unate_compl(cover(&[&[0, 1], &[2, 3]]), 1);

        assert_eq!(result, Cover::new(4));
    }

    #[test]
    fn covered_helpers_match_selected_column_rules() {
        let input = cover(&[&[0, 1], &[2], &[1, 3]]);

        assert_eq!(ncp_abs_covered(&input, 1), cover(&[&[2]]));
        assert_eq!(ncp_abs_covered_many(&input, &cube(&[1, 3])), cover(&[&[2]]));
        assert_eq!(ncp_abs_select_restricted(&input, &cube(&[1, 2, 3])), 1);
    }

    #[test]
    fn missing_cover_ports_are_explicit() {
        assert_eq!(
            native_cover_bound_minimize(),
            Err(MinimizeError::MissingSisPorts {
                operation: "minimize",
            })
        );
    }

    #[test]
    fn no_disallowed_porting_tokens_are_present() {
        let text = include_str!("minimize.rs");

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
