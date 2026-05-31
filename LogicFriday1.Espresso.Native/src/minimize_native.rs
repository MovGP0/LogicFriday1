use super::*;
use std::collections::BTreeSet;

pub fn expand_native(
    mut on_set: Cover,
    off_set: &Cover,
    layout: &CubeLayout,
    options: ExpandOptions,
) -> Result<ExpandReport, MinimizeError> {
    layout.validate_cover(&on_set)?;
    layout.validate_cover(off_set)?;
    sort_for_expansion(&mut on_set);

    let mut init_lower = Cube::empty();
    if options.nonsparse {
        for variable in layout.variables().iter().copied().filter(|v| v.is_sparse()) {
            init_lower = init_lower.union(&Cube::from_columns(variable.columns()));
        }
    }

    for cube in on_set.cubes_mut() {
        cube.set_covered(false);
        cube.set_nonessential(false);
    }

    let mut expanded_cubes = 0;
    let mut index = 0;
    while index < on_set.len() {
        let should_expand = {
            let cube = &on_set.cubes()[index];
            !cube.is_prime() && !cube.is_covered()
        };

        if should_expand {
            let result = expand_one(index, &mut on_set, off_set, layout, &init_lower)?;
            let cube = &mut on_set.cubes_mut()[index];
            cube.columns = result.raise.columns;
            cube.set_prime(true);
            cube.set_covered(false);
            cube.set_nonessential(result.num_covered == 0 && *cube != result.overexpanded_cube);
            expanded_cubes += 1;
        }
        index += 1;
    }

    on_set.retain(|cube| !cube.is_covered());
    Ok(ExpandReport {
        cover: on_set,
        expanded_cubes,
    })
}

pub fn all_primes_native(
    on_set: &Cover,
    off_set: &Cover,
    layout: &CubeLayout,
) -> Result<Cover, MinimizeError> {
    layout.validate_cover(on_set)?;
    layout.validate_cover(off_set)?;

    let mut result = Cover::with_output_part_size(on_set.set_size(), on_set.output_part_size());
    for cube in on_set.cubes() {
        if cube.is_prime() {
            result.push(cube.clone());
            continue;
        }

        let mut raise = cube.clone();
        let mut freeset = layout.full_cube().difference(&raise);
        let mut bb_active = vec![true; off_set.len()];
        essen_parts_native(
            off_set,
            None,
            &mut raise,
            &mut freeset,
            &mut bb_active,
            None,
            layout,
        )?;

        for mut prime in find_all_primes(off_set, &bb_active, &raise, &freeset, layout) {
            prime.set_prime(true);
            result.push(prime);
        }
    }

    Ok(contain_cover(result))
}

pub fn primes_consensus_native(cover: &Cover, layout: &CubeLayout) -> Result<Cover, MinimizeError> {
    layout.validate_cover(cover)?;
    let mut primes = contain_cover(cover.clone());
    let mut changed = true;

    while changed {
        changed = false;
        let snapshot = primes.cubes().to_vec();
        for left in 0..snapshot.len() {
            for right in (left + 1)..snapshot.len() {
                if cube_distance_limited(&snapshot[left], &snapshot[right], layout) == 1 {
                    let mut candidate = consensus_cube(&snapshot[left], &snapshot[right], layout);
                    candidate.set_prime(true);
                    if !primes.cubes().iter().any(|cube| cube == &candidate) {
                        primes.push(candidate);
                        changed = true;
                    }
                }
            }
        }
        primes = contain_cover(primes);
    }

    for cube in primes.cubes_mut() {
        cube.set_prime(true);
    }
    Ok(primes)
}

pub fn essential_native(
    cover: Cover,
    mut dont_care: Cover,
    layout: &CubeLayout,
) -> Result<(Cover, Cover, Cover), MinimizeError> {
    layout.validate_cover(&cover)?;
    layout.validate_cover(&dont_care)?;
    layout.minterm_count()?;

    let all_points = enumerate_minterms(layout);
    let mut essential_indices = BTreeSet::new();

    for (index, cube) in cover.cubes().iter().enumerate() {
        if cube.is_nonessential() || !cube.is_relatively_essential() {
            continue;
        }

        if all_points.iter().any(|point| {
            cube_covers_minterm(cube, point)
                && !cover.cubes().iter().enumerate().any(|(other, other_cube)| {
                    other != index && cube_covers_minterm(other_cube, point)
                })
                && !dont_care
                    .cubes()
                    .iter()
                    .any(|dc_cube| cube_covers_minterm(dc_cube, point))
        }) {
            essential_indices.insert(index);
        }
    }

    let mut remaining = Cover::with_output_part_size(cover.set_size(), cover.output_part_size());
    let mut essential = Cover::with_output_part_size(cover.set_size(), cover.output_part_size());
    for (index, cube) in cover.cubes().iter().cloned().enumerate() {
        if essential_indices.contains(&index) {
            essential.push(cube);
        } else {
            remaining.push(cube);
        }
    }

    dont_care = dont_care.append(essential.clone());
    Ok((remaining, dont_care, essential))
}

pub fn reduce_native(
    mut cover: Cover,
    dont_care: &Cover,
    layout: &CubeLayout,
    use_distance_sort: bool,
) -> Result<Cover, MinimizeError> {
    layout.validate_cover(&cover)?;
    layout.validate_cover(dont_care)?;
    layout.minterm_count()?;

    if use_distance_sort {
        sort_reduce(&mut cover, layout);
    } else {
        mini_sort_descending(&mut cover);
    }

    let original = cover.clone();
    let all_points = enumerate_minterms(layout);
    let mut reduced = Cover::with_output_part_size(cover.set_size(), cover.output_part_size());

    for (index, cube) in original.cubes().iter().enumerate() {
        let essential_points = all_points
            .iter()
            .filter(|point| {
                cube_covers_minterm(cube, point)
                    && !original
                        .cubes()
                        .iter()
                        .enumerate()
                        .any(|(other, other_cube)| {
                            other != index && cube_covers_minterm(other_cube, point)
                        })
                    && !dont_care
                        .cubes()
                        .iter()
                        .any(|dc_cube| cube_covers_minterm(dc_cube, point))
            })
            .cloned()
            .collect::<Vec<_>>();

        if essential_points.is_empty() {
            continue;
        }

        let mut reduced_cube = cube_from_minterms(&essential_points, layout);
        reduced_cube.set_prime(reduced_cube == *cube);
        reduced.push(reduced_cube);
    }

    Ok(contain_cover(reduced))
}

pub fn make_sparse_native(
    cover: Cover,
    dont_care: &Cover,
    off_set: &Cover,
    layout: &CubeLayout,
) -> Result<Cover, MinimizeError> {
    layout.validate_cover(&cover)?;
    layout.validate_cover(dont_care)?;
    layout.validate_cover(off_set)?;

    let mut best_cover = cover;
    let mut best_cost = best_cover.cost();
    let mut current = mv_reduce_native(best_cover.clone(), dont_care, layout)?;
    let mut cost = current.cost();
    if cost.total_literals == best_cost.total_literals {
        return Ok(current);
    }
    if cost.total_literals > best_cost.total_literals {
        return Ok(best_cover);
    }

    best_cover = current.clone();
    best_cost = cost;

    current = expand_native(
        current,
        off_set,
        layout,
        ExpandOptions::non_sparse_variables_only(),
    )?
    .cover;
    cost = current.cost();
    if cost.total_literals <= best_cost.total_literals {
        return Ok(current);
    }

    Ok(best_cover)
}

pub fn mv_reduce_native(
    mut cover: Cover,
    dont_care: &Cover,
    layout: &CubeLayout,
) -> Result<Cover, MinimizeError> {
    layout.validate_cover(&cover)?;
    layout.validate_cover(dont_care)?;

    let variables = layout.variables().to_vec();
    for (variable_index, variable) in variables.iter().copied().enumerate() {
        if !variable.is_sparse() {
            continue;
        }

        for part in variable.columns() {
            let mut mapped_indices = Vec::new();
            let mut part_cover =
                Cover::with_output_part_size(cover.set_size(), cover.output_part_size());
            for (cube_index, cube) in cover.cubes().iter().enumerate() {
                if cube.contains(part) {
                    mapped_indices.push(cube_index);
                    part_cover.push(cofactor_sparse_part(cube, variable, part));
                }
            }

            if part_cover.is_empty() {
                continue;
            }

            let mut part_dont_care =
                Cover::with_output_part_size(dont_care.set_size(), dont_care.output_part_size());
            for cube in dont_care.cubes() {
                if cube.contains(part) {
                    part_dont_care.push(cofactor_sparse_part(cube, variable, part));
                }
            }

            let active = irredundant_active_indices(&part_cover, &part_dont_care, layout)?;
            for (part_index, original_index) in mapped_indices.into_iter().enumerate() {
                if active.contains(&part_index) {
                    continue;
                }

                let cube = &mut cover.cubes_mut()[original_index];
                let is_output_variable = variable_index + 1 == variables.len();
                if is_output_variable || !variable_is_full(cube, variable) {
                    cube.remove(part);
                }
                cube.set_prime(false);
            }
        }
    }

    cover.retain(|cube| {
        variables
            .iter()
            .copied()
            .filter(Variable::is_sparse)
            .all(|variable| variable_has_part(cube, variable))
    });

    Ok(cover)
}

pub fn irredundant_native(
    cover: Cover,
    dont_care: &Cover,
    layout: &CubeLayout,
) -> Result<Cover, MinimizeError> {
    layout.validate_cover(&cover)?;
    let indexed = IndexedCover::from_cover(cover);
    let split = split_irredundant_native(&indexed, dont_care, layout)?;
    let table = derive_prime_table_native(dont_care, &split, layout)?;
    let selected = minimum_cover(table.rows(), None, true)?;

    form_exact_result_cover(&indexed, &split, &selected).map(contain_cover)
}

fn irredundant_active_indices(
    cover: &Cover,
    dont_care: &Cover,
    layout: &CubeLayout,
) -> Result<BTreeSet<usize>, MinimizeError> {
    let indexed = IndexedCover::from_cover(cover.clone());
    irredundant_active_indices_indexed(&indexed, dont_care, layout)
}

fn irredundant_active_indices_indexed(
    indexed: &IndexedCover,
    dont_care: &Cover,
    layout: &CubeLayout,
) -> Result<BTreeSet<usize>, MinimizeError> {
    let split = split_irredundant_native(indexed, dont_care, layout)?;
    let table = derive_prime_table_native(dont_care, &split, layout)?;
    let mut active = split
        .essential
        .cubes()
        .iter()
        .map(IndexedCube::index)
        .collect::<BTreeSet<_>>();
    active.extend(minimum_cover(table.rows(), None, true)?);
    Ok(active)
}

fn cofactor_sparse_part(cube: &Cube, variable: Variable, part: usize) -> Cube {
    let mask = Cube::from_columns(variable.columns());
    cube.difference(&mask).union(&Cube::from_columns([part]))
}

fn variable_is_full(cube: &Cube, variable: Variable) -> bool {
    variable.columns().all(|part| cube.contains(part))
}

fn variable_has_part(cube: &Cube, variable: Variable) -> bool {
    variable.columns().any(|part| cube.contains(part))
}

pub fn split_irredundant_native(
    primes: &IndexedCover,
    dont_care: &Cover,
    layout: &CubeLayout,
) -> Result<IrredundantSplit, MinimizeError> {
    layout.validate_cover(dont_care)?;
    layout.minterm_count()?;

    let points = enumerate_minterms(layout)
        .into_iter()
        .filter(|point| {
            !dont_care
                .cubes()
                .iter()
                .any(|cube| cube_covers_minterm(cube, point))
        })
        .collect::<Vec<_>>();

    let mut coverage = Vec::new();
    let mut essential_indices = BTreeSet::new();

    for candidate in primes.cubes() {
        layout.validate_cube(candidate.cube())?;
        let covered_points = points
            .iter()
            .filter(|point| cube_covers_minterm(candidate.cube(), point))
            .cloned()
            .collect::<Vec<_>>();

        if covered_points.is_empty() {
            coverage.push((candidate.clone(), covered_points));
            continue;
        }

        let has_unique_point = covered_points.iter().any(|point| {
            !primes.cubes().iter().any(|other| {
                other.index() != candidate.index() && cube_covers_minterm(other.cube(), point)
            })
        });

        if has_unique_point {
            essential_indices.insert(candidate.index());
        }
        coverage.push((candidate.clone(), covered_points));
    }

    let essential = coverage
        .iter()
        .filter_map(|(candidate, _)| {
            essential_indices
                .contains(&candidate.index())
                .then_some(candidate.clone())
        })
        .collect::<Vec<_>>();
    let mut totally_redundant = Vec::new();
    let mut partially_redundant = Vec::new();

    for (candidate, covered_points) in coverage {
        if essential_indices.contains(&candidate.index()) {
            continue;
        }
        let covered_by_essential = covered_points.iter().all(|point| {
            essential
                .iter()
                .any(|essential| cube_covers_minterm(essential.cube(), point))
        });
        if covered_points.is_empty() || covered_by_essential {
            totally_redundant.push(candidate.clone());
        } else {
            partially_redundant.push(candidate.clone());
        }
    }

    Ok(IrredundantSplit {
        essential: IndexedCover::from_indexed_cubes(
            primes.set_size,
            primes.output_part_size,
            essential,
        ),
        totally_redundant: IndexedCover::from_indexed_cubes(
            primes.set_size,
            primes.output_part_size,
            totally_redundant,
        ),
        partially_redundant: IndexedCover::from_indexed_cubes(
            primes.set_size,
            primes.output_part_size,
            partially_redundant,
        ),
    })
}

pub fn derive_prime_table_native(
    dont_care: &Cover,
    split: &IrredundantSplit,
    layout: &CubeLayout,
) -> Result<PrimeTable, MinimizeError> {
    layout.validate_cover(dont_care)?;
    layout.minterm_count()?;

    let rows = enumerate_minterms(layout)
        .into_iter()
        .filter(|point| {
            !dont_care
                .cubes()
                .iter()
                .any(|cube| cube_covers_minterm(cube, point))
        })
        .filter(|point| {
            !split
                .essential
                .cubes()
                .iter()
                .any(|cube| cube_covers_minterm(cube.cube(), point))
        })
        .filter_map(|point| {
            let row = split
                .partially_redundant
                .cubes()
                .iter()
                .filter_map(|cube| cube_covers_minterm(cube.cube(), &point).then_some(cube.index()))
                .collect::<BTreeSet<_>>();
            (!row.is_empty()).then_some(row)
        })
        .collect::<Vec<_>>();

    Ok(PrimeTable::new(rows))
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
    layout: &CubeLayout,
    init_lower: &Cube,
) -> Result<ExpandOneResult, MinimizeError> {
    on_set.cubes_mut()[cube_index].set_prime(true);

    let mut bb_active = vec![true; off_set.len()];
    let mut cc_active = on_set
        .cubes()
        .iter()
        .map(|cube| !cube.is_covered() && !cube.is_prime())
        .collect::<Vec<_>>();

    let mut num_covered = 0;
    let mut super_cube = on_set.cubes()[cube_index].clone();
    let mut raise = on_set.cubes()[cube_index].clone();
    let mut freeset = layout.full_cube().difference(&raise);

    if !init_lower.is_empty() {
        freeset = freeset.difference(init_lower);
        elim_lowering(
            off_set,
            Some(on_set),
            &raise,
            &freeset,
            &mut bb_active,
            Some(&mut cc_active),
            layout,
        );
    }

    essen_parts_native(
        off_set,
        Some(on_set),
        &mut raise,
        &mut freeset,
        &mut bb_active,
        Some(&mut cc_active),
        layout,
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
            layout,
        )?;
    }

    while cc_active.iter().any(|active| *active) {
        let best_part = most_frequent(Some(on_set), Some(&cc_active), &freeset)
            .expect("active covering cubes imply a free part");
        raise.insert(best_part);
        freeset.remove(best_part);
        essen_parts_native(
            off_set,
            Some(on_set),
            &mut raise,
            &mut freeset,
            &mut bb_active,
            Some(&mut cc_active),
            layout,
        )?;
    }

    while bb_active.iter().any(|active| *active) {
        mincov(off_set, &mut raise, &mut freeset, &mut bb_active, layout)?;
    }

    raise = raise.union(&freeset);
    Ok(ExpandOneResult {
        raise,
        overexpanded_cube,
        num_covered,
    })
}

fn essen_parts_native(
    off_set: &Cover,
    mut on_set: Option<&mut Cover>,
    raise: &mut Cube,
    freeset: &mut Cube,
    bb_active: &mut [bool],
    cc_active: Option<&mut [bool]>,
    layout: &CubeLayout,
) -> Result<(), MinimizeError> {
    let mut xlower = Cube::empty();

    for (index, off_cube) in off_set.cubes().iter().enumerate() {
        if !bb_active[index] {
            continue;
        }

        match cube_distance_limited(off_cube, raise, layout) {
            0 => return Err(MinimizeError::OnSetIntersectsOffSet),
            1 => {
                xlower = xlower.union(&force_lower(off_cube, raise, layout));
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
            layout,
        );
    }

    Ok(())
}

fn essen_raising(off_set: &Cover, raise: &mut Cube, freeset: &mut Cube, bb_active: &[bool]) {
    let mut blocked = Cube::empty();
    for (index, off_cube) in off_set.cubes().iter().enumerate() {
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
    layout: &CubeLayout,
) {
    let overexpanded = raise.union(freeset);
    for (index, off_cube) in off_set.cubes().iter().enumerate() {
        if bb_active[index] && cube_distance(off_cube, &overexpanded, layout) > 0 {
            bb_active[index] = false;
        }
    }

    if let (Some(on_set), Some(cc_active)) = (on_set, cc_active) {
        for (index, cube) in on_set.cubes().iter().enumerate() {
            if cc_active[index] && !cube.is_subset_of(&overexpanded) {
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
    layout: &CubeLayout,
) -> Result<(), MinimizeError> {
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

            let cube = &on_set.cubes()[cube_index];
            if cube.is_subset_of(raise) {
                *num_covered += 1;
                *super_cube = super_cube.union(cube);
                cc_active[cube_index] = false;
                on_set.cubes_mut()[cube_index].set_covered(true);
            } else if let Some(new_lower) =
                feasibly_covered(off_set, cube, raise, bb_active, layout)?
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
            let candidate = &on_set.cubes()[cube_index];
            let size = candidate.columns.intersection(&freeset.columns).count();
            let count = feasible
                .iter()
                .filter(|other_index| {
                    feasible_new_lower[candidate_position]
                        .is_disjoint_from(&on_set.cubes()[**other_index])
                })
                .count();

            if count > best_count || count == best_count && size < best_size {
                best_index = candidate_position;
                best_count = count;
                best_size = size;
            }
        }

        *raise = raise.union(&on_set.cubes()[feasible[best_index]]);
        *freeset = freeset.difference(raise);
        essen_parts_native(
            off_set,
            Some(on_set),
            raise,
            freeset,
            bb_active,
            Some(cc_active),
            layout,
        )?;
    }
}

fn feasibly_covered(
    off_set: &Cover,
    cube: &Cube,
    raise: &Cube,
    bb_active: &[bool],
    layout: &CubeLayout,
) -> Result<Option<Cube>, MinimizeError> {
    let combined = raise.union(cube);
    let mut new_lower = Cube::empty();

    for (index, off_cube) in off_set.cubes().iter().enumerate() {
        if !bb_active[index] {
            continue;
        }

        match cube_distance_limited(off_cube, &combined, layout) {
            0 => return Ok(None),
            1 => new_lower = new_lower.union(&force_lower(off_cube, &combined, layout)),
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
    layout: &CubeLayout,
) -> Result<(), MinimizeError> {
    let mut lowered_rows = Vec::new();
    for (index, off_cube) in off_set.cubes().iter().enumerate() {
        if bb_active[index] {
            lowered_rows.push(force_lower(off_cube, raise, layout));
        }
    }

    if lowered_rows.is_empty() {
        bb_active.fill(false);
        return Ok(());
    }

    let expansion = unraveled_size(&lowered_rows, layout);
    if expansion <= 500 {
        let unraveled = unravel(&lowered_rows, layout);
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
    essen_parts_native(off_set, None, raise, freeset, bb_active, None, layout)
}

fn find_all_primes(
    off_set: &Cover,
    bb_active: &[bool],
    raise: &Cube,
    freeset: &Cube,
    layout: &CubeLayout,
) -> Vec<Cube> {
    let mut lowered_rows = Vec::new();
    for (index, off_cube) in off_set.cubes().iter().enumerate() {
        if bb_active[index] {
            lowered_rows.push(force_lower(off_cube, raise, layout));
        }
    }

    if lowered_rows.is_empty() {
        return vec![raise.union(freeset)];
    }

    let unraveled = retain_maximal(unravel(&lowered_rows, layout));
    minimum_hitting_sets(&unraveled)
        .into_iter()
        .map(|lowered| raise.union(&freeset.difference(&lowered)))
        .collect()
}

fn sort_for_expansion(cover: &mut Cover) {
    let mut counts = vec![0usize; cover.set_size()];
    for cube in cover.cubes() {
        for column in cube.columns() {
            counts[column] += 1;
        }
    }

    cover.cubes.sort_by_key(|cube| {
        cube.columns()
            .map(|column| counts.get(column).copied().unwrap_or(0))
            .sum::<usize>()
    });
}

fn sort_reduce(cover: &mut Cover, layout: &CubeLayout) {
    let largest = cover
        .cubes()
        .iter()
        .max_by_key(|cube| cube.literal_count())
        .cloned()
        .unwrap_or_else(Cube::empty);
    cover.cubes.sort_by_key(|cube| {
        (
            cube_distance(cube, &largest, layout),
            usize::MAX - cube.literal_count(),
        )
    });
}

fn mini_sort_descending(cover: &mut Cover) {
    let mut counts = vec![0usize; cover.set_size()];
    for cube in cover.cubes() {
        for column in cube.columns() {
            counts[column] += 1;
        }
    }
    cover.cubes.sort_by_key(|cube| {
        std::cmp::Reverse(
            cube.columns()
                .map(|column| counts.get(column).copied().unwrap_or(0))
                .sum::<usize>(),
        )
    });
}

fn most_frequent(on_set: Option<&Cover>, active: Option<&[bool]>, freeset: &Cube) -> Option<usize> {
    let mut counts = Vec::new();

    if let (Some(on_set), Some(active)) = (on_set, active) {
        for (index, cube) in on_set.cubes().iter().enumerate() {
            if !active[index] {
                continue;
            }
            for column in cube.columns() {
                if column >= counts.len() {
                    counts.resize(column + 1, 0usize);
                }
                counts[column] += 1;
            }
        }
    }

    freeset.columns().max_by_key(|column| {
        (
            counts.get(*column).copied().unwrap_or(0),
            usize::MAX - *column,
        )
    })
}

fn cube_distance_limited(left: &Cube, right: &Cube, layout: &CubeLayout) -> usize {
    let mut distance = 0;
    for variable in layout.variables() {
        if variable
            .columns()
            .all(|column| !left.contains(column) || !right.contains(column))
        {
            distance += 1;
            if distance > 1 {
                return 2;
            }
        }
    }
    distance
}

fn cube_distance(left: &Cube, right: &Cube, layout: &CubeLayout) -> usize {
    layout
        .variables()
        .iter()
        .filter(|variable| {
            variable
                .columns()
                .all(|column| !left.contains(column) || !right.contains(column))
        })
        .count()
}

fn force_lower(off_cube: &Cube, raise: &Cube, layout: &CubeLayout) -> Cube {
    let mut lowered = Cube::empty();
    for variable in layout.variables() {
        let intersects = variable
            .columns()
            .any(|column| off_cube.contains(column) && raise.contains(column));
        if !intersects {
            lowered = lowered.union(&Cube::from_columns(
                variable
                    .columns()
                    .filter(|column| off_cube.contains(*column)),
            ));
        }
    }
    lowered
}

fn consensus_cube(left: &Cube, right: &Cube, layout: &CubeLayout) -> Cube {
    let mut result = Cube::empty();
    for variable in layout.variables() {
        let left_part =
            Cube::from_columns(variable.columns().filter(|column| left.contains(*column)));
        let right_part =
            Cube::from_columns(variable.columns().filter(|column| right.contains(*column)));
        let intersection = left_part.intersect(&right_part);
        let contribution = if intersection.is_empty() {
            left_part.union(&right_part)
        } else {
            intersection
        };
        result = result.union(&contribution);
    }
    result
}

pub(super) fn cube_column_count(cube: &Cube, variable: Variable) -> usize {
    variable
        .columns()
        .filter(|column| cube.contains(*column))
        .count()
}

fn unraveled_size(rows: &[Cube], layout: &CubeLayout) -> usize {
    let mut total = 0usize;
    for row in rows {
        let mut expansion = 1usize;
        for variable in layout.variables() {
            let count = cube_column_count(row, *variable);
            if count > 1 {
                expansion = expansion.saturating_mul(count);
            }
        }
        total = total.saturating_add(expansion);
    }
    total
}

fn unravel(rows: &[Cube], layout: &CubeLayout) -> Vec<Cube> {
    let mut result = Vec::new();
    for row in rows {
        let mut partial = vec![Cube::empty()];
        for variable in layout.variables() {
            let selected = variable
                .columns()
                .filter(|column| row.contains(*column))
                .collect::<Vec<_>>();

            if selected.len() <= 1 {
                for cube in &mut partial {
                    for column in &selected {
                        cube.insert(*column);
                    }
                }
                continue;
            }

            let mut next = Vec::new();
            for cube in &partial {
                for column in &selected {
                    let mut expanded = cube.clone();
                    expanded.insert(*column);
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
        .flat_map(Cube::columns)
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
    best.sort_by_key(|cube| cube.columns.clone());
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
    if current.literal_count() > *best_len {
        return;
    }

    if rows
        .iter()
        .all(|row| row.columns().any(|column| current.contains(column)))
    {
        match current.literal_count().cmp(best_len) {
            std::cmp::Ordering::Less => {
                *best_len = current.literal_count();
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
    rows.sort_by_key(|row| row.columns.clone());
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
    rows.sort_by_key(|row| row.columns.clone());
    rows.dedup();
    let original = rows.clone();
    rows.retain(|row| {
        !original
            .iter()
            .any(|candidate| candidate != row && row.is_subset_of(candidate))
    });
    rows
}

fn contain_cover(cover: Cover) -> Cover {
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

fn enumerate_minterms(layout: &CubeLayout) -> Vec<Cube> {
    fn walk(layout: &CubeLayout, var: usize, current: &mut Cube, output: &mut Vec<Cube>) {
        if var == layout.variables().len() {
            output.push(current.clone());
            return;
        }

        for column in layout.variables()[var].columns() {
            current.insert(column);
            walk(layout, var + 1, current, output);
            current.remove(column);
        }
    }

    let mut output = Vec::new();
    walk(layout, 0, &mut Cube::empty(), &mut output);
    output
}

fn cube_covers_minterm(cube: &Cube, minterm: &Cube) -> bool {
    minterm.is_subset_of(cube)
}

fn cube_from_minterms(minterms: &[Cube], layout: &CubeLayout) -> Cube {
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
