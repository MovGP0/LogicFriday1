//! Native Rust reduced-offset support for `LogicSynthesis/sis/minimize/ros.c`.
//!
//! The original SIS implementation stores Espresso cubes in process-wide
//! globals and computes reduced offsets through a static/dynamic cofactoring
//! tree. This port keeps the same set-family responsibilities in an owned,
//! Boolean cube model: cache lookup and replacement, overexpanded cube
//! calculation, reduced-offset construction, cover multiplication, cube
//! complement, non-cover filtering, and independent-cover partitioning.

use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

pub const ROS_ZEROS: usize = 20;
pub const ROS_MEM_SIZE: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum LiteralValue {
    Zero,
    One,
    DontCare,
}

impl LiteralValue {
    pub fn complement(self) -> Self {
        match self {
            Self::Zero => Self::One,
            Self::One => Self::Zero,
            Self::DontCare => Self::DontCare,
        }
    }

    pub fn is_specified(self) -> bool {
        self != Self::DontCare
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RosCube {
    literals: Vec<LiteralValue>,
}

impl RosCube {
    pub fn new(literals: impl Into<Vec<LiteralValue>>) -> Self {
        Self {
            literals: literals.into(),
        }
    }

    pub fn full(width: usize) -> Self {
        Self {
            literals: vec![LiteralValue::DontCare; width],
        }
    }

    pub fn width(&self) -> usize {
        self.literals.len()
    }

    pub fn literals(&self) -> &[LiteralValue] {
        &self.literals
    }

    pub fn literal(&self, index: usize) -> Option<LiteralValue> {
        self.literals.get(index).copied()
    }

    pub fn set_literal(&mut self, index: usize, value: LiteralValue) -> RosResult<()> {
        if index >= self.width() {
            return Err(RosError::VariableOutOfRange {
                index,
                width: self.width(),
            });
        }

        self.literals[index] = value;
        Ok(())
    }

    pub fn is_full(&self) -> bool {
        self.literals
            .iter()
            .all(|literal| *literal == LiteralValue::DontCare)
    }

    pub fn specified_count(&self) -> usize {
        self.literals
            .iter()
            .filter(|literal| literal.is_specified())
            .count()
    }

    pub fn zero_count(&self) -> usize {
        self.width() - self.specified_count()
    }

    pub fn implies(&self, other: &Self) -> RosResult<bool> {
        ensure_same_width(self, other)?;
        Ok(self
            .literals
            .iter()
            .zip(other.literals.iter())
            .all(|(left, right)| *right == LiteralValue::DontCare || left == right))
    }

    pub fn intersects(&self, other: &Self) -> RosResult<bool> {
        ensure_same_width(self, other)?;
        Ok(self
            .literals
            .iter()
            .zip(other.literals.iter())
            .all(|(left, right)| {
                *left == LiteralValue::DontCare || *right == LiteralValue::DontCare || left == right
            }))
    }

    pub fn intersection(&self, other: &Self) -> RosResult<Option<Self>> {
        ensure_same_width(self, other)?;
        let mut literals = Vec::with_capacity(self.width());
        for (left, right) in self.literals.iter().zip(other.literals.iter()) {
            let literal = match (*left, *right) {
                (LiteralValue::DontCare, value) | (value, LiteralValue::DontCare) => value,
                (left_value, right_value) if left_value == right_value => left_value,
                _ => return Ok(None),
            };
            literals.push(literal);
        }

        Ok(Some(Self { literals }))
    }

    pub fn supercube(&self, other: &Self) -> RosResult<Self> {
        ensure_same_width(self, other)?;
        Ok(Self {
            literals: self
                .literals
                .iter()
                .zip(other.literals.iter())
                .map(|(left, right)| {
                    if left == right {
                        *left
                    } else {
                        LiteralValue::DontCare
                    }
                })
                .collect(),
        })
    }

    pub fn differs_only_in(&self, other: &Self, variable: usize) -> RosResult<bool> {
        ensure_same_width(self, other)?;
        if variable >= self.width() {
            return Err(RosError::VariableOutOfRange {
                index: variable,
                width: self.width(),
            });
        }

        Ok(self
            .literals
            .iter()
            .zip(other.literals.iter())
            .enumerate()
            .all(|(index, (left, right))| index == variable || left == right))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RosCover {
    width: usize,
    cubes: Vec<RosCube>,
}

impl RosCover {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            cubes: Vec::new(),
        }
    }

    pub fn from_cubes(cubes: impl IntoIterator<Item = RosCube>) -> RosResult<Self> {
        let mut iter = cubes.into_iter();
        let Some(first) = iter.next() else {
            return Ok(Self::new(0));
        };

        let mut cover = Self::new(first.width());
        cover.add_cube(first)?;
        for cube in iter {
            cover.add_cube(cube)?;
        }

        Ok(cover)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn cubes(&self) -> &[RosCube] {
        &self.cubes
    }

    pub fn add_cube(&mut self, cube: RosCube) -> RosResult<()> {
        if self.width == 0 && self.cubes.is_empty() {
            self.width = cube.width();
        }
        ensure_width(cube.width(), self.width)?;

        if self
            .cubes
            .iter()
            .any(|existing| cube.implies(existing).unwrap_or(false))
        {
            return Ok(());
        }

        self.cubes
            .retain(|existing| !existing.implies(&cube).unwrap_or(false));
        self.cubes.push(cube);
        self.cubes.sort();
        Ok(())
    }

    pub fn contains_cube(&self, cube: &RosCube) -> RosResult<bool> {
        ensure_width(cube.width(), self.width)?;
        self.cubes.iter().try_fold(false, |found, existing| {
            if found {
                Ok(true)
            } else {
                cube.implies(existing)
            }
        })
    }

    pub fn intersects_cube(&self, cube: &RosCube) -> RosResult<bool> {
        ensure_width(cube.width(), self.width)?;
        self.cubes.iter().try_fold(false, |found, existing| {
            if found {
                Ok(true)
            } else {
                existing.intersects(cube)
            }
        })
    }

    pub fn append(&mut self, other: Self) -> RosResult<()> {
        ensure_width(other.width, self.width)?;
        for cube in other.cubes {
            self.add_cube(cube)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RosError {
    WidthMismatch { expected: usize, actual: usize },
    VariableOutOfRange { index: usize, width: usize },
    ExactComplementTooWide { width: usize },
}

impl fmt::Display for RosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WidthMismatch { expected, actual } => {
                write!(f, "cube width mismatch: expected {expected}, got {actual}")
            }
            Self::VariableOutOfRange { index, width } => {
                write!(f, "variable index {index} is outside cube width {width}")
            }
            Self::ExactComplementTooWide { width } => {
                write!(
                    f,
                    "exact cover complement is limited to 16 variables, got {width}"
                )
            }
        }
    }
}

impl Error for RosError {}

pub type RosResult<T> = Result<T, RosError>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RosStats {
    pub reduced_offset_count: usize,
    pub overexpanded_cube_count: usize,
    pub max_reduced_offset_size: usize,
    pub cache_hits: usize,
}

#[derive(Clone, Debug)]
pub struct RosOptions {
    pub max_level: usize,
    pub filter_level: usize,
    pub ros_zeros: usize,
    pub max_store: usize,
    pub max_size_store: usize,
}

impl RosOptions {
    pub fn for_cover_size(cover_size: usize) -> Self {
        let max_store = (2 * cover_size).max(1);
        Self {
            max_level: 0,
            filter_level: 1,
            ros_zeros: ROS_ZEROS,
            max_store,
            max_size_store: ROS_MEM_SIZE / max_store,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RosComputation {
    pub reduced_offset: RosCover,
    pub overexpanded_cube: RosCube,
    pub cache_hit: bool,
}

#[derive(Clone, Debug)]
struct RosEntry {
    ros_cube: RosCube,
    reduced_offset: RosCover,
    use_count: usize,
}

#[derive(Clone, Debug)]
pub struct ReducedOffsetSession {
    blocking_cover: RosCover,
    options: RosOptions,
    cache: Vec<RosEntry>,
    stats: RosStats,
}

impl ReducedOffsetSession {
    pub fn new(blocking_cover: RosCover, options: RosOptions) -> Self {
        Self {
            blocking_cover,
            options,
            cache: Vec::new(),
            stats: RosStats::default(),
        }
    }

    pub fn for_cover(blocking_cover: RosCover) -> Self {
        let options = RosOptions::for_cover_size(blocking_cover.len());
        Self::new(blocking_cover, options)
    }

    pub fn stats(&self) -> &RosStats {
        &self.stats
    }

    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    pub fn get_reduced_offset(&mut self, cube: &RosCube) -> RosResult<RosComputation> {
        self.get_reduced_offset_with_old_cube(cube, None)
    }

    pub fn get_reduced_offset_with_old_cube(
        &mut self,
        cube: &RosCube,
        old_cube: Option<&RosCube>,
    ) -> RosResult<RosComputation> {
        ensure_width(cube.width(), self.blocking_cover.width())?;
        if let Some(cached) = self.find_cached(cube)? {
            self.stats.cache_hits += 1;
            return Ok(RosComputation {
                reduced_offset: cached.reduced_offset,
                overexpanded_cube: cached.ros_cube,
                cache_hit: true,
            });
        }

        let overexpanded_cube = self.overexpand_cube(cube, old_cube)?;
        let mut reduced_offset = self.blocking_matrix(cube, &overexpanded_cube)?;
        if !overexpanded_cube.is_full() {
            reduced_offset.append(complement_cube(&overexpanded_cube)?)?;
        }

        self.stats.reduced_offset_count += 1;
        self.stats.overexpanded_cube_count += 1;
        self.stats.max_reduced_offset_size =
            self.stats.max_reduced_offset_size.max(reduced_offset.len());

        self.store(overexpanded_cube.clone(), reduced_offset.clone());

        Ok(RosComputation {
            reduced_offset,
            overexpanded_cube,
            cache_hit: false,
        })
    }

    fn find_cached(&mut self, cube: &RosCube) -> RosResult<Option<RosEntry>> {
        for entry in &mut self.cache {
            if cube.implies(&entry.ros_cube)? {
                entry.use_count += 1;
                return Ok(Some(entry.clone()));
            }
        }

        Ok(None)
    }

    fn store(&mut self, ros_cube: RosCube, reduced_offset: RosCover) {
        if reduced_offset.len() > self.options.max_size_store {
            return;
        }

        let entry = RosEntry {
            ros_cube,
            reduced_offset,
            use_count: 1,
        };

        if self.cache.len() < self.options.max_store {
            self.cache.push(entry);
            return;
        }

        if let Some((index, _)) = self
            .cache
            .iter()
            .enumerate()
            .min_by_key(|(_, entry)| entry.use_count)
        {
            self.cache[index] = entry;
        }
    }

    fn overexpand_cube(&self, cube: &RosCube, old_cube: Option<&RosCube>) -> RosResult<RosCube> {
        if let Some(old_cube) = old_cube {
            ensure_same_width(cube, old_cube)?;
        }

        let mut expanded = old_cube.cloned().unwrap_or_else(|| cube.clone());
        for variable in 0..cube.width() {
            if cube.literals[variable] == LiteralValue::DontCare {
                continue;
            }

            let saved = expanded.literals[variable];
            expanded.literals[variable] = LiteralValue::DontCare;
            if self.blocking_cover.intersects_cube(&expanded)? {
                expanded.literals[variable] = saved;
            }
        }

        Ok(expanded)
    }

    fn blocking_matrix(&self, cube: &RosCube, overexpanded_cube: &RosCube) -> RosResult<RosCover> {
        let mut matrix = RosCover::new(cube.width());
        for blocker in self.blocking_cover.cubes() {
            if blocker.intersects(overexpanded_cube)? && !blocker.intersects(cube)? {
                if let Some(intersection) = blocker.intersection(overexpanded_cube)? {
                    matrix.add_cube(intersection)?;
                }
            }
        }

        Ok(matrix)
    }
}

pub fn complement_cube(cube: &RosCube) -> RosResult<RosCover> {
    let mut result = RosCover::new(cube.width());
    for (index, literal) in cube.literals.iter().copied().enumerate() {
        if literal.is_specified() {
            let mut complement = RosCube::full(cube.width());
            complement.set_literal(index, literal.complement())?;
            result.add_cube(complement)?;
        }
    }

    Ok(result)
}

pub fn multiply_covers(left: RosCover, right: RosCover) -> RosResult<RosCover> {
    ensure_width(left.width(), right.width())?;
    if left.is_empty() || right.is_empty() {
        return Ok(RosCover::new(left.width()));
    }

    let mut result = RosCover::new(left.width());
    let (smaller, larger) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };

    for left_cube in smaller.cubes() {
        for right_cube in larger.cubes() {
            if let Some(product) = left_cube.intersection(right_cube)? {
                result.add_cube(product)?;
            }
        }
    }

    Ok(result)
}

pub fn distance_zero_in_variable(
    left: &RosCube,
    right: &RosCube,
    variable: usize,
) -> RosResult<bool> {
    ensure_same_width(left, right)?;
    if variable >= left.width() {
        return Err(RosError::VariableOutOfRange {
            index: variable,
            width: left.width(),
        });
    }

    Ok(left.literals[variable] == LiteralValue::DontCare
        || right.literals[variable] == LiteralValue::DontCare
        || left.literals[variable] == right.literals[variable])
}

pub fn remove_noncovering_cubes(cube: &RosCube, cover: &RosCover) -> RosResult<RosCover> {
    ensure_width(cube.width(), cover.width())?;
    let mut result = RosCover::new(cover.width());
    for row in cover.cubes() {
        if cube.implies(row)? {
            result.add_cube(row.clone())?;
        }
    }

    Ok(result)
}

pub fn or_noncovering_cubes(cube: &RosCube, cover: &RosCover) -> RosResult<RosCube> {
    ensure_width(cube.width(), cover.width())?;
    let mut retained = remove_noncovering_cubes(cube, cover)?;
    if retained.is_empty() {
        return Ok(RosCube::full(cube.width()));
    }

    let mut iter = retained.cubes.drain(..);
    let mut result = iter.next().expect("retained cover is non-empty");
    for row in iter {
        result = result.supercube(&row)?;
    }

    Ok(result)
}

pub fn partition_cover(cover: &RosCover) -> RosResult<Option<(RosCover, RosCover)>> {
    if cover.len() <= 1 {
        return Ok(None);
    }

    let supports = cover.cubes().iter().map(active_support).collect::<Vec<_>>();
    let mut visited = vec![false; cover.len()];
    let mut queue = VecDeque::from([0]);
    visited[0] = true;

    while let Some(index) = queue.pop_front() {
        for candidate in 0..cover.len() {
            if !visited[candidate] && !supports[index].is_disjoint(&supports[candidate]) {
                visited[candidate] = true;
                queue.push_back(candidate);
            }
        }
    }

    if visited.iter().all(|was_visited| *was_visited) {
        return Ok(None);
    }

    let mut first = RosCover::new(cover.width());
    let mut second = RosCover::new(cover.width());
    for (index, cube) in cover.cubes().iter().cloned().enumerate() {
        if visited[index] {
            first.add_cube(cube)?;
        } else {
            second.add_cube(cube)?;
        }
    }

    Ok(Some((first, second)))
}

pub fn exact_complement(cover: &RosCover) -> RosResult<RosCover> {
    if cover.width() > 16 {
        return Err(RosError::ExactComplementTooWide {
            width: cover.width(),
        });
    }

    let mut result = RosCover::new(cover.width());
    let minterm_count = 1_usize << cover.width();
    for minterm in 0..minterm_count {
        let cube = minterm_cube(minterm, cover.width());
        if !cover.contains_cube(&cube)? {
            result.add_cube(cube)?;
        }
    }

    Ok(result)
}

fn minterm_cube(minterm: usize, width: usize) -> RosCube {
    RosCube::new(
        (0..width)
            .map(|index| {
                if (minterm & (1 << index)) == 0 {
                    LiteralValue::Zero
                } else {
                    LiteralValue::One
                }
            })
            .collect::<Vec<_>>(),
    )
}

fn active_support(cube: &RosCube) -> BTreeSet<usize> {
    cube.literals()
        .iter()
        .enumerate()
        .filter_map(|(index, literal)| literal.is_specified().then_some(index))
        .collect()
}

fn ensure_same_width(left: &RosCube, right: &RosCube) -> RosResult<()> {
    ensure_width(left.width(), right.width())
}

fn ensure_width(actual: usize, expected: usize) -> RosResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(RosError::WidthMismatch { expected, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(values: &[LiteralValue]) -> RosCube {
        RosCube::new(values.to_vec())
    }

    fn z() -> LiteralValue {
        LiteralValue::Zero
    }

    fn o() -> LiteralValue {
        LiteralValue::One
    }

    fn x() -> LiteralValue {
        LiteralValue::DontCare
    }

    #[test]
    fn complement_cube_uses_one_clause_per_specified_literal() {
        let cube = c(&[o(), x(), z()]);
        let complement = complement_cube(&cube).unwrap();

        assert_eq!(complement.len(), 2);
        assert!(complement.contains_cube(&c(&[z(), x(), x()])).unwrap());
        assert!(complement.contains_cube(&c(&[x(), x(), o()])).unwrap());
    }

    #[test]
    fn multiply_covers_forms_intersections_and_drops_conflicts() {
        let left = RosCover::from_cubes([c(&[o(), x(), x()]), c(&[x(), z(), x()])]).unwrap();
        let right = RosCover::from_cubes([c(&[x(), o(), x()]), c(&[x(), x(), z()])]).unwrap();

        let product = multiply_covers(left, right).unwrap();

        assert_eq!(product.len(), 3);
        assert!(product.contains_cube(&c(&[o(), o(), x()])).unwrap());
        assert!(product.contains_cube(&c(&[o(), x(), z()])).unwrap());
        assert!(product.contains_cube(&c(&[x(), z(), z()])).unwrap());
    }

    #[test]
    fn reduced_offset_session_overexpands_until_a_blocker_would_be_hit() {
        let blockers = RosCover::from_cubes([c(&[z(), o(), x()])]).unwrap();
        let mut session = ReducedOffsetSession::for_cover(blockers);

        let result = session.get_reduced_offset(&c(&[o(), o(), z()])).unwrap();

        assert_eq!(result.overexpanded_cube, c(&[o(), x(), x()]));
        assert!(!result.cache_hit);
        assert!(
            result
                .reduced_offset
                .contains_cube(&c(&[z(), x(), x()]))
                .unwrap()
        );
        assert_eq!(session.stats().reduced_offset_count, 1);
    }

    #[test]
    fn cache_hit_reuses_a_stored_reduced_offset_for_contained_cube() {
        let blockers = RosCover::from_cubes([c(&[z(), o(), x()])]).unwrap();
        let mut session = ReducedOffsetSession::for_cover(blockers);

        session.get_reduced_offset(&c(&[o(), o(), z()])).unwrap();
        let result = session.get_reduced_offset(&c(&[o(), x(), z()])).unwrap();

        assert!(result.cache_hit);
        assert_eq!(session.stats().cache_hits, 1);
        assert_eq!(session.cache_len(), 1);
    }

    #[test]
    fn cache_replaces_the_least_used_entry_when_full() {
        let blockers = RosCover::from_cubes([c(&[z(), x()])]).unwrap();
        let options = RosOptions {
            max_level: 0,
            filter_level: 1,
            ros_zeros: ROS_ZEROS,
            max_store: 1,
            max_size_store: ROS_MEM_SIZE,
        };
        let mut session = ReducedOffsetSession::new(blockers, options);

        session.get_reduced_offset(&c(&[o(), z()])).unwrap();
        session.get_reduced_offset(&c(&[z(), o()])).unwrap();

        assert_eq!(session.cache_len(), 1);
        assert_eq!(session.stats().reduced_offset_count, 2);
    }

    #[test]
    fn remove_noncovering_cubes_keeps_rows_that_contain_the_cube() {
        let cover = RosCover::from_cubes([
            c(&[o(), x(), z()]),
            c(&[o(), o(), z()]),
            c(&[z(), x(), z()]),
        ])
        .unwrap();

        let retained = remove_noncovering_cubes(&c(&[o(), o(), z()]), &cover).unwrap();

        assert_eq!(retained.len(), 1);
        assert!(retained.contains_cube(&c(&[o(), x(), z()])).unwrap());
        assert!(retained.contains_cube(&c(&[o(), o(), z()])).unwrap());
    }

    #[test]
    fn partition_cover_splits_independent_support_components() {
        let cover = RosCover::from_cubes([
            c(&[o(), z(), x(), x()]),
            c(&[z(), o(), x(), x()]),
            c(&[x(), x(), o(), z()]),
        ])
        .unwrap();

        let (first, second) = partition_cover(&cover).unwrap().unwrap();

        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 1);
    }

    #[test]
    fn exact_complement_enumerates_uncovered_minterms_for_small_covers() {
        let cover = RosCover::from_cubes([c(&[o(), x()])]).unwrap();

        let complement = exact_complement(&cover).unwrap();

        assert_eq!(complement.len(), 2);
        assert!(complement.contains_cube(&c(&[z(), z()])).unwrap());
        assert!(complement.contains_cube(&c(&[z(), o()])).unwrap());
    }

    #[test]
    fn no_legacy_c_abi_or_source_dependency_metadata_is_present() {
        let source = include_str!("ros.rs");

        let forbidden = [
            concat!("extern ", "\"C\""),
            concat!("no", "_mangle"),
            concat!("REQUIRED", "_"),
            concat!("Port", "Dependency"),
            concat!("LogicFriday1", "-8j8"),
        ];

        for token in forbidden {
            assert!(!source.contains(token), "{token}");
        }
    }
}
