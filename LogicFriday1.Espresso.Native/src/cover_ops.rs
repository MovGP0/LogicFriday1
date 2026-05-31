use std::cmp::Ordering;

/// A single Espresso cube represented as a set of part bits.
///
/// Binary variables use two parts (`ZERO`, `ONE`); multiple-valued variables
/// use one part per value. A cube contains a minterm when every variable has at
/// least one part in common with that minterm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeSet {
    size: usize,
    words: Vec<u64>,
    flags: u32,
    cached_order: usize,
}

impl CubeSet {
    const ACTIVE: u32 = 0x2000;

    pub fn empty(size: usize) -> Self {
        Self {
            size,
            words: vec![0; word_count(size)],
            flags: 0,
            cached_order: 0,
        }
    }

    pub fn full(size: usize) -> Self {
        let mut set = Self::empty(size);
        for word in &mut set.words {
            *word = !0;
        }
        set.trim_unused_bits();
        set.cached_order = set.order();
        set
    }

    pub fn from_indices(size: usize, indices: impl IntoIterator<Item = usize>) -> Self {
        let mut set = Self::empty(size);
        for index in indices {
            set.insert(index);
        }
        set
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn contains(&self, index: usize) -> bool {
        assert!(index < self.size, "set bit index out of range");
        self.words[index / 64] & (1u64 << (index % 64)) != 0
    }

    pub fn insert(&mut self, index: usize) {
        assert!(index < self.size, "set bit index out of range");
        self.words[index / 64] |= 1u64 << (index % 64);
    }

    pub fn remove(&mut self, index: usize) {
        assert!(index < self.size, "set bit index out of range");
        self.words[index / 64] &= !(1u64 << (index % 64));
    }

    pub fn clear(&mut self) {
        self.words.fill(0);
        self.cached_order = 0;
    }

    pub fn fill(&mut self) {
        self.words.fill(!0);
        self.trim_unused_bits();
        self.cached_order = self.order();
    }

    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|word| *word == 0)
    }

    pub fn is_full(&self) -> bool {
        self == &Self::full(self.size)
    }

    pub fn order(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    pub fn intersection_count(&self, other: &Self) -> usize {
        self.check_compatible(other);
        self.words
            .iter()
            .zip(&other.words)
            .map(|(a, b)| (a & b).count_ones() as usize)
            .sum()
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.check_compatible(other);
        self.words.iter().zip(&other.words).all(|(a, b)| a & b == 0)
    }

    pub fn implies(&self, container: &Self) -> bool {
        self.check_compatible(container);
        self.words
            .iter()
            .zip(&container.words)
            .all(|(a, b)| a & !b == 0)
    }

    pub fn union(&self, other: &Self) -> Self {
        self.zip_words(other, |a, b| a | b)
    }

    pub fn intersect(&self, other: &Self) -> Self {
        self.zip_words(other, |a, b| a & b)
    }

    pub fn difference(&self, other: &Self) -> Self {
        self.zip_words(other, |a, b| a & !b)
    }

    pub fn xor(&self, other: &Self) -> Self {
        self.zip_words(other, |a, b| a ^ b)
    }

    pub fn merge_masked(&self, other: &Self, mask: &Self) -> Self {
        self.check_compatible(other);
        self.check_compatible(mask);
        let words = self
            .words
            .iter()
            .zip(&other.words)
            .zip(&mask.words)
            .map(|((a, b), m)| (a & m) | (b & !m))
            .collect();
        Self::from_words(self.size, words)
    }

    pub fn set_active(&mut self, active: bool) {
        if active {
            self.flags |= Self::ACTIVE;
        } else {
            self.flags &= !Self::ACTIVE;
        }
    }

    pub fn is_active(&self) -> bool {
        self.flags & Self::ACTIVE != 0
    }

    fn with_cached_order(mut self) -> Self {
        self.cached_order = self.order();
        self
    }

    fn from_words(size: usize, words: Vec<u64>) -> Self {
        let mut set = Self {
            size,
            words,
            flags: 0,
            cached_order: 0,
        };
        set.trim_unused_bits();
        set
    }

    fn zip_words(&self, other: &Self, f: impl Fn(u64, u64) -> u64) -> Self {
        self.check_compatible(other);
        let words = self
            .words
            .iter()
            .zip(&other.words)
            .map(|(a, b)| f(*a, *b))
            .collect();
        Self::from_words(self.size, words)
    }

    fn check_compatible(&self, other: &Self) {
        assert_eq!(self.size, other.size, "set size mismatch");
    }

    fn trim_unused_bits(&mut self) {
        let unused = self.words.len() * 64 - self.size;
        if unused > 0 {
            if let Some(last) = self.words.last_mut() {
                *last &= !0u64 >> unused;
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    size: usize,
    cubes: Vec<CubeSet>,
}

impl Cover {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            cubes: Vec::new(),
        }
    }

    pub fn from_cubes(size: usize, cubes: Vec<CubeSet>) -> Self {
        for cube in &cubes {
            assert_eq!(cube.size(), size, "cover cube size mismatch");
        }
        Self { size, cubes }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn cubes(&self) -> &[CubeSet] {
        &self.cubes
    }

    pub fn push(&mut self, cube: CubeSet) {
        assert_eq!(cube.size(), self.size, "cover cube size mismatch");
        self.cubes.push(cube);
    }

    pub fn append(&mut self, mut other: Cover) {
        assert_eq!(self.size, other.size, "cover size mismatch");
        self.cubes.append(&mut other.cubes);
    }

    pub fn or_all(&self) -> CubeSet {
        self.cubes
            .iter()
            .fold(CubeSet::empty(self.size), |acc, cube| acc.union(cube))
    }

    pub fn and_all(&self) -> CubeSet {
        self.cubes
            .iter()
            .fold(CubeSet::full(self.size), |acc, cube| acc.intersect(cube))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure {
    part_size: Vec<usize>,
    num_binary_vars: usize,
    first_part: Vec<usize>,
    last_part: Vec<usize>,
    var_masks: Vec<CubeSet>,
    fullset: CubeSet,
}

impl CubeStructure {
    pub fn new(num_binary_vars: usize, mut mv_part_sizes: Vec<usize>) -> Self {
        let mut part_size = vec![2; num_binary_vars];
        part_size.append(&mut mv_part_sizes);
        Self::from_part_sizes(num_binary_vars, part_size)
    }

    pub fn from_part_sizes(num_binary_vars: usize, part_size: Vec<usize>) -> Self {
        assert!(
            num_binary_vars <= part_size.len(),
            "binary variable count exceeds variable count"
        );
        assert!(
            part_size[..num_binary_vars].iter().all(|size| *size == 2),
            "binary variables must have two parts"
        );
        assert!(
            part_size.iter().all(|size| *size > 0),
            "variables must have at least one part"
        );

        let mut first_part = Vec::with_capacity(part_size.len());
        let mut last_part = Vec::with_capacity(part_size.len());
        let mut offset = 0;
        for size in &part_size {
            first_part.push(offset);
            offset += *size;
            last_part.push(offset - 1);
        }

        let fullset = CubeSet::full(offset);
        let var_masks = first_part
            .iter()
            .zip(&last_part)
            .map(|(first, last)| CubeSet::from_indices(offset, *first..=*last))
            .collect();

        Self {
            part_size,
            num_binary_vars,
            first_part,
            last_part,
            var_masks,
            fullset,
        }
    }

    pub fn size(&self) -> usize {
        self.fullset.size()
    }

    pub fn num_vars(&self) -> usize {
        self.part_size.len()
    }

    pub fn num_binary_vars(&self) -> usize {
        self.num_binary_vars
    }

    pub fn part_size(&self, var: usize) -> usize {
        self.part_size[var]
    }

    pub fn first_part(&self, var: usize) -> usize {
        self.first_part[var]
    }

    pub fn last_part(&self, var: usize) -> usize {
        self.last_part[var]
    }

    pub fn var_mask(&self, var: usize) -> &CubeSet {
        &self.var_masks[var]
    }

    pub fn fullset(&self) -> &CubeSet {
        &self.fullset
    }

    pub fn full_cube(&self) -> CubeSet {
        self.fullset.clone()
    }

    pub fn empty_cube(&self) -> CubeSet {
        CubeSet::empty(self.size())
    }

    pub fn cube_from_parts(&self, parts: impl IntoIterator<Item = usize>) -> CubeSet {
        CubeSet::from_indices(self.size(), parts)
    }

    pub fn binary_literal(&self, var: usize, value: bool) -> CubeSet {
        assert!(var < self.num_binary_vars);
        let mut cube = self.full_cube();
        cube.remove(self.first_part[var] + usize::from(!value));
        cube
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeList {
    cofactored: CubeSet,
    cubes: Vec<CubeSet>,
}

impl CubeList {
    pub fn new(cofactored: CubeSet, cubes: Vec<CubeSet>) -> Self {
        Self { cofactored, cubes }
    }

    pub fn cofactored(&self) -> &CubeSet {
        &self.cofactored
    }

    pub fn cubes(&self) -> &[CubeSet] {
        &self.cubes
    }

    pub fn len(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cubes.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountData {
    pub part_zeros: Vec<usize>,
    pub var_zeros: Vec<usize>,
    pub parts_active: Vec<usize>,
    pub is_unate: Vec<bool>,
    pub vars_active: usize,
    pub vars_unate: usize,
    pub best: Option<usize>,
}

pub fn cube1list(cover: &Cover, ctx: &CubeStructure) -> CubeList {
    CubeList::new(ctx.empty_cube(), cover.cubes.clone())
}

pub fn cube2list(a: &Cover, b: &Cover, ctx: &CubeStructure) -> CubeList {
    assert_eq!(a.size(), b.size(), "cover size mismatch");
    let mut cubes = Vec::with_capacity(a.len() + b.len());
    cubes.extend_from_slice(a.cubes());
    cubes.extend_from_slice(b.cubes());
    CubeList::new(ctx.empty_cube(), cubes)
}

pub fn cube3list(a: &Cover, b: &Cover, c: &Cover, ctx: &CubeStructure) -> CubeList {
    assert_eq!(a.size(), b.size(), "cover size mismatch");
    assert_eq!(a.size(), c.size(), "cover size mismatch");
    let mut cubes = Vec::with_capacity(a.len() + b.len() + c.len());
    cubes.extend_from_slice(a.cubes());
    cubes.extend_from_slice(b.cubes());
    cubes.extend_from_slice(c.cubes());
    CubeList::new(ctx.empty_cube(), cubes)
}

pub fn cubeunlist(list: &CubeList) -> Cover {
    let cubes = list
        .cubes
        .iter()
        .map(|cube| cube.union(&list.cofactored))
        .collect();
    Cover::from_cubes(list.cofactored.size(), cubes)
}

pub fn cdist0(a: &CubeSet, b: &CubeSet, ctx: &CubeStructure) -> bool {
    (0..ctx.num_vars()).all(|var| !a.intersect(b).is_disjoint(ctx.var_mask(var)))
}

pub fn cdist(a: &CubeSet, b: &CubeSet, ctx: &CubeStructure) -> usize {
    let intersection = a.intersect(b);
    (0..ctx.num_vars())
        .filter(|var| intersection.is_disjoint(ctx.var_mask(*var)))
        .count()
}

pub fn cdist01(a: &CubeSet, b: &CubeSet, ctx: &CubeStructure) -> usize {
    cdist(a, b, ctx).min(2)
}

pub fn full_row(cube: &CubeSet, cofactored: &CubeSet, ctx: &CubeStructure) -> bool {
    cube.union(cofactored) == *ctx.fullset()
}

pub fn cofactor(list: &CubeList, against: &CubeSet, ctx: &CubeStructure) -> CubeList {
    let cofactored = list.cofactored.union(&ctx.fullset().difference(against));
    let cubes = list
        .cubes
        .iter()
        .filter(|cube| *cube != against && cdist0(cube, against, ctx))
        .cloned()
        .collect();
    CubeList::new(cofactored, cubes)
}

pub fn scofactor(list: &CubeList, against: &CubeSet, var: usize, ctx: &CubeStructure) -> CubeList {
    let cofactored = list.cofactored.union(&ctx.fullset().difference(against));
    let mask = ctx.var_mask(var).intersect(against);
    let cubes = list
        .cubes
        .iter()
        .filter(|cube| *cube != against && !cube.is_disjoint(&mask))
        .cloned()
        .collect();
    CubeList::new(cofactored, cubes)
}

pub fn massive_count(list: &CubeList, ctx: &CubeStructure) -> CountData {
    let mut part_zeros = vec![0; ctx.size()];
    for cube in &list.cubes {
        let zeros = ctx.fullset().difference(&cube.union(&list.cofactored));
        for bit in bits(&zeros) {
            part_zeros[bit] += 1;
        }
    }

    let mut var_zeros = vec![0; ctx.num_vars()];
    let mut parts_active = vec![0; ctx.num_vars()];
    let mut is_unate = vec![false; ctx.num_vars()];
    let mut vars_active = 0;
    let mut vars_unate = 0;
    let mut best = None;
    let mut most_active = 0;
    let mut most_zero = 0;
    let mut most_balanced = usize::MAX;

    for var in 0..ctx.num_vars() {
        let counts = ctx.first_part(var)..=ctx.last_part(var);
        let mut active = 0;
        let mut max_zeros_in_part = 0;
        for part in counts {
            let zeros = part_zeros[part];
            var_zeros[var] += zeros;
            active += usize::from(zeros > 0);
            max_zeros_in_part = max_zeros_in_part.max(zeros);
        }

        if active > most_active
            || (active == most_active
                && (var_zeros[var] > most_zero
                    || (var_zeros[var] == most_zero && max_zeros_in_part < most_balanced)))
        {
            best = Some(var);
            most_active = active;
            most_zero = var_zeros[var];
            most_balanced = max_zeros_in_part;
        }

        parts_active[var] = active;
        is_unate[var] = active == 1;
        vars_active += usize::from(active > 0);
        vars_unate += usize::from(active == 1);
    }

    CountData {
        part_zeros,
        var_zeros,
        parts_active,
        is_unate,
        vars_active,
        vars_unate,
        best,
    }
}

pub fn binate_split_select(
    list: &CubeList,
    counts: &CountData,
    ctx: &CubeStructure,
) -> Option<(usize, CubeSet, CubeSet)> {
    let best = counts.best?;
    let mut cleft = ctx.fullset().difference(ctx.var_mask(best));
    let mut cright = cleft.clone();

    let available: Vec<_> = (ctx.first_part(best)..=ctx.last_part(best))
        .filter(|part| !list.cofactored.contains(*part))
        .collect();
    let split = available.len() / 2;
    for part in &available[..split] {
        cleft.insert(*part);
    }
    for part in &available[split..] {
        cright.insert(*part);
    }
    Some((best, cleft, cright))
}

pub fn sf_contain(cover: &Cover) -> Cover {
    contain_with_mode(cover, false, true)
}

pub fn sf_rev_contain(cover: &Cover) -> Cover {
    contain_with_mode(cover, true, true)
}

pub fn sf_dupl(cover: &Cover) -> Cover {
    contain_with_mode(cover, false, false)
}

pub fn sf_union(a: &Cover, b: &Cover) -> Cover {
    assert_eq!(a.size(), b.size(), "cover size mismatch");
    let mut joined = Cover::from_cubes(a.size(), a.cubes.clone());
    joined.cubes.extend_from_slice(b.cubes());
    sf_contain(&joined)
}

pub fn d1merge(cover: &Cover, var: usize, ctx: &CubeStructure) -> Cover {
    let mask = ctx.var_mask(var);
    let mut groups: Vec<CubeSet> = Vec::new();
    for cube in &cover.cubes {
        if let Some(existing) = groups
            .iter_mut()
            .find(|existing| d1_order_key(existing, mask) == d1_order_key(cube, mask))
        {
            *existing = existing.union(cube);
        } else {
            groups.push(cube.clone());
        }
    }
    sf_dupl(&Cover::from_cubes(cover.size(), groups))
}

pub fn cv_intersect(a: &Cover, b: &Cover, ctx: &CubeStructure) -> Cover {
    assert_eq!(a.size(), b.size(), "cover size mismatch");
    let mut result = Cover::new(a.size());
    for left in a.cubes() {
        for right in b.cubes() {
            if cdist0(left, right, ctx) {
                result.push(left.intersect(right));
            }
        }
    }
    sf_contain(&result)
}

pub fn sharp(a: &CubeSet, b: &CubeSet, ctx: &CubeStructure) -> Cover {
    let mut result = Cover::new(ctx.size());
    if cdist0(a, b, ctx) {
        let diff = a.difference(b);
        for var in 0..ctx.num_vars() {
            let active_diff = diff.intersect(ctx.var_mask(var));
            if !active_diff.is_empty() {
                result.push(active_diff.union(&a.difference(ctx.var_mask(var))));
            }
        }
    } else {
        result.push(a.clone());
    }
    result
}

pub fn cb_sharp(cube: &CubeSet, cover: &Cover, ctx: &CubeStructure) -> Cover {
    if cover.is_empty() {
        Cover::from_cubes(ctx.size(), vec![cube.clone()])
    } else {
        cb_recur_sharp(cube, cover, 0, cover.len() - 1, ctx)
    }
}

pub fn cv_sharp(a: &Cover, b: &Cover, ctx: &CubeStructure) -> Cover {
    let mut result = Cover::new(ctx.size());
    for cube in a.cubes() {
        result = sf_union(&result, &cb_sharp(cube, b, ctx));
    }
    result
}

pub fn dsharp(a: &CubeSet, b: &CubeSet, ctx: &CubeStructure) -> Cover {
    let mut result = Cover::new(ctx.size());
    if cdist0(a, b, ctx) {
        let diff = a.difference(b);
        let and = a.intersect(b);
        let mut mask = ctx.empty_cube();
        for var in 0..ctx.num_vars() {
            if !diff.is_disjoint(ctx.var_mask(var)) {
                let coord = diff.intersect(ctx.var_mask(var));
                let before = and.intersect(&mask);
                mask = mask.union(ctx.var_mask(var));
                let after = a.difference(&mask);
                result.push(coord.union(&before).union(&after));
            }
        }
    } else {
        result.push(a.clone());
    }
    result
}

pub fn cb_dsharp(cube: &CubeSet, cover: &Cover, ctx: &CubeStructure) -> Cover {
    if cover.is_empty() {
        Cover::from_cubes(ctx.size(), vec![cube.clone()])
    } else {
        let mut result = Cover::from_cubes(ctx.size(), vec![cube.clone()]);
        for other in cover.cubes() {
            result = cb1_dsharp(&result, other, ctx);
        }
        result
    }
}

pub fn cb1_dsharp(cover: &Cover, cube: &CubeSet, ctx: &CubeStructure) -> Cover {
    let mut result = Cover::new(ctx.size());
    for other in cover.cubes() {
        result = sf_union(&result, &dsharp(other, cube, ctx));
    }
    result
}

pub fn cv_dsharp(a: &Cover, b: &Cover, ctx: &CubeStructure) -> Cover {
    let mut result = Cover::new(ctx.size());
    for cube in a.cubes() {
        result = sf_union(&result, &cb_dsharp(cube, b, ctx));
    }
    result
}

pub fn make_disjoint(cover: &Cover, ctx: &CubeStructure) -> Cover {
    let mut result = Cover::new(ctx.size());
    for cube in cover.cubes() {
        let fresh = cb_dsharp(cube, &result, ctx);
        result.append(fresh);
    }
    result
}

pub fn complement(list: CubeList, ctx: &CubeStructure) -> Cover {
    if list.is_empty() {
        return Cover::from_cubes(ctx.size(), vec![ctx.full_cube()]);
    }
    if list.len() == 1 {
        return compl_cube(&list.cofactored.union(&list.cubes[0]), ctx);
    }
    if list
        .cubes
        .iter()
        .any(|cube| full_row(cube, &list.cofactored, ctx))
    {
        return Cover::new(ctx.size());
    }

    let ceil = list
        .cubes
        .iter()
        .fold(list.cofactored.clone(), |acc, cube| acc.union(cube));
    if ceil != *ctx.fullset() {
        let lifted = CubeList::new(
            list.cofactored.union(&ctx.fullset().difference(&ceil)),
            list.cubes.clone(),
        );
        let mut result = complement(lifted, ctx);
        result.append(compl_cube(&ceil, ctx));
        return result;
    }

    let counts = massive_count(&list, ctx);
    if counts.vars_active == 1 {
        return Cover::new(ctx.size());
    }
    if counts.vars_unate == counts.vars_active {
        let unate = map_cover_to_unate(&list, &counts);
        let unate_compl_cover = unate_compl(&unate);
        return map_unate_to_cover(&unate_compl_cover, &counts, ctx);
    }

    let Some((best, cleft, cright)) = binate_split_select(&list, &counts, ctx) else {
        return Cover::new(ctx.size());
    };
    let left = complement(scofactor(&list, &cleft, best, ctx), ctx);
    let right = complement(scofactor(&list, &cright, best, ctx), ctx);
    compl_merge(&list, left, right, &cleft, &cright, best, ctx)
}

pub fn simplify(list: CubeList, ctx: &CubeStructure) -> Cover {
    if list.is_empty() {
        return Cover::new(ctx.size());
    }
    if list.len() == 1 {
        return Cover::from_cubes(ctx.size(), vec![list.cofactored.union(&list.cubes[0])]);
    }
    if list
        .cubes
        .iter()
        .any(|cube| full_row(cube, &list.cofactored, ctx))
    {
        return Cover::from_cubes(ctx.size(), vec![ctx.full_cube()]);
    }

    let ceil = list
        .cubes
        .iter()
        .fold(list.cofactored.clone(), |acc, cube| acc.union(cube));
    if ceil != *ctx.fullset() {
        let lifted = CubeList::new(
            list.cofactored.union(&ctx.fullset().difference(&ceil)),
            list.cubes.clone(),
        );
        let mut result = simplify(lifted, ctx);
        for cube in &mut result.cubes {
            *cube = cube.intersect(&ceil);
        }
        return result;
    }

    let counts = massive_count(&list, ctx);
    if counts.vars_active == 1 {
        return Cover::from_cubes(ctx.size(), vec![ctx.full_cube()]);
    }
    if counts.vars_unate == counts.vars_active {
        return sf_contain(&cubeunlist(&list));
    }

    let Some((best, cleft, cright)) = binate_split_select(&list, &counts, ctx) else {
        return Cover::new(ctx.size());
    };
    let left = simplify(scofactor(&list, &cleft, best, ctx), ctx);
    let right = simplify(scofactor(&list, &cright, best, ctx), ctx);
    let result = compl_merge(&list, left, right, &cleft, &cright, best, ctx);
    if result.len() > list.len() {
        cubeunlist(&list)
    } else {
        result
    }
}

pub fn map_cover_to_unate(list: &CubeList, counts: &CountData) -> Cover {
    let active_parts: Vec<_> = counts
        .part_zeros
        .iter()
        .enumerate()
        .filter_map(|(part, zeros)| (*zeros > 0).then_some(part))
        .collect();
    let mut result = Cover::new(active_parts.len());
    for cube in list.cubes() {
        let mut row = CubeSet::empty(active_parts.len());
        for (col, part) in active_parts.iter().enumerate() {
            if !cube.contains(*part) {
                row.insert(col);
            }
        }
        result.push(row.with_cached_order());
    }
    result
}

pub fn map_unate_to_cover(unate: &Cover, counts: &CountData, ctx: &CubeStructure) -> Cover {
    let unate_vars: Vec<_> = counts
        .is_unate
        .iter()
        .enumerate()
        .filter_map(|(var, is_unate)| (*is_unate).then_some(var))
        .collect();
    let mut result = Cover::new(ctx.size());
    for row in unate.cubes() {
        let mut cube = ctx.full_cube();
        for (col, var) in unate_vars.iter().enumerate() {
            if row.contains(col) {
                for part in ctx.first_part(*var)..=ctx.last_part(*var) {
                    if counts.part_zeros[part] == 0 {
                        cube.remove(part);
                    }
                }
            }
        }
        result.push(cube);
    }
    result
}

pub fn unate_compl(cover: &Cover) -> Cover {
    let mut with_sizes = cover.clone();
    for cube in &mut with_sizes.cubes {
        cube.cached_order = cube.order();
    }
    sf_rev_contain(&unate_complement(with_sizes))
}

pub fn unate_complement(cover: Cover) -> Cover {
    if cover.is_empty() {
        return Cover::from_cubes(cover.size(), vec![CubeSet::empty(cover.size())]);
    }
    if cover.len() == 1 {
        let mut result = Cover::new(cover.size());
        for bit in bits(&cover.cubes[0]) {
            result.push(CubeSet::from_indices(cover.size(), [bit]));
        }
        return result;
    }

    let min_order = cover
        .cubes
        .iter()
        .map(|cube| cube.cached_order.max(cube.order()))
        .min()
        .unwrap_or(0);
    let restricted = cover
        .cubes
        .iter()
        .filter(|cube| cube.cached_order.max(cube.order()) == min_order)
        .fold(CubeSet::empty(cover.size()), |acc, cube| acc.union(cube));

    if min_order == 0 {
        return Cover::new(cover.size());
    }
    if min_order == 1 {
        let mut result = unate_complement(abs_covered_many(&cover, &restricted));
        for cube in &mut result.cubes {
            *cube = cube.union(&restricted);
        }
        return result;
    }

    let pick = abs_select_restricted(&cover, &restricted);
    let mut picked = unate_complement(abs_covered(&cover, pick));
    for cube in &mut picked.cubes {
        cube.insert(pick);
    }

    let mut without_pick = cover.clone();
    for cube in &mut without_pick.cubes {
        if cube.contains(pick) {
            cube.remove(pick);
            cube.cached_order = cube.cached_order.saturating_sub(1);
        }
    }
    picked.append(unate_complement(without_pick));
    picked
}

pub fn unate_intersect(a: &Cover, b: &Cover, largest_only: bool) -> Cover {
    assert_eq!(a.size(), b.size(), "cover size mismatch");
    let mut result = Cover::new(a.size());
    let mut max_order = 0;

    for left in a.cubes() {
        for right in b.cubes() {
            let intersection = left.intersect(right);
            if intersection.is_empty() {
                continue;
            }

            if largest_only {
                let order = intersection.order();
                match order.cmp(&max_order) {
                    Ordering::Greater => {
                        result.cubes.clear();
                        max_order = order;
                    }
                    Ordering::Less => continue,
                    Ordering::Equal => {}
                }
            }
            result.push(intersection);
        }
    }
    sf_contain(&result)
}

/// Exact minimum-cover helper from `unate.c`: returns every minimum column set
/// that covers all rows in `table`.
pub fn exact_minimum_cover(table: &Cover) -> Cover {
    if table.is_empty() {
        return Cover::new(table.size());
    }

    let mut result = Cover::from_cubes(table.size(), vec![CubeSet::empty(table.size())]);
    for row in table.cubes() {
        let mut choices = Cover::new(table.size());
        for bit in bits(row) {
            choices.push(CubeSet::from_indices(table.size(), [bit]));
        }
        result = unate_intersect(&result, &choices, false);
    }
    sf_rev_contain(&result)
}

pub fn consensus(a: &CubeSet, b: &CubeSet, ctx: &CubeStructure) -> CubeSet {
    let mut result = ctx.empty_cube();
    for var in 0..ctx.num_vars() {
        let mask = ctx.var_mask(var);
        let intersection = a.intersect(b).intersect(mask);
        let contribution = if intersection.is_empty() {
            a.union(b).intersect(mask)
        } else {
            intersection
        };
        result = result.union(&contribution);
    }
    result
}

pub fn force_lower(xlower: &mut CubeSet, a: &CubeSet, b: &CubeSet, ctx: &CubeStructure) {
    for var in 0..ctx.num_vars() {
        let mask = ctx.var_mask(var);
        if a.intersect(b).is_disjoint(mask) {
            *xlower = xlower.union(&a.intersect(mask));
        }
    }
}

pub fn cactive(cube: &CubeSet, ctx: &CubeStructure) -> Option<usize> {
    let mut active = None;
    for var in 0..ctx.num_vars() {
        if !ctx.var_mask(var).difference(cube).is_empty() {
            if active.is_some() {
                return None;
            }
            active = Some(var);
        }
    }
    active
}

pub fn ccommon(a: &CubeSet, b: &CubeSet, cofactored: &CubeSet, ctx: &CubeStructure) -> bool {
    (0..ctx.num_vars()).any(|var| {
        let active_a = !ctx
            .var_mask(var)
            .difference(&a.union(cofactored))
            .is_empty();
        let active_b = !ctx
            .var_mask(var)
            .difference(&b.union(cofactored))
            .is_empty();
        active_a && active_b
    })
}

/// Expands a cover into minterm ordinals in the same mixed-radix order used by
/// Espresso's `map.c`.
pub fn minterms(cover: &Cover, ctx: &CubeStructure) -> CubeSet {
    let size = ctx.part_size.iter().product();
    let mut result = CubeSet::empty(size);
    for cube in cover.cubes() {
        explode_minterms(cube, ctx, ctx.num_vars() - 1, 0, &mut result);
    }
    result
}

fn compl_merge(
    original: &CubeList,
    mut left: Cover,
    mut right: Cover,
    cleft: &CubeSet,
    cright: &CubeSet,
    var: usize,
    ctx: &CubeStructure,
) -> Cover {
    for cube in &mut left.cubes {
        *cube = cube.intersect(cleft);
        cube.set_active(true);
    }
    for cube in &mut right.cubes {
        *cube = cube.intersect(cright);
        cube.set_active(true);
    }

    compl_d1merge(&mut left, &mut right, var, ctx);
    compl_lift(&mut left, &right, cright, var, ctx);
    compl_lift(&mut right, &left, cleft, var, ctx);

    let mut result = Cover::new(ctx.size());
    result.cubes.extend(left.cubes);
    result
        .cubes
        .extend(right.cubes.into_iter().filter(CubeSet::is_active));

    let merged = sf_contain(&result);
    if merged.len() > original.len() {
        cubeunlist(original)
    } else {
        merged
    }
}

fn compl_cube(cube: &CubeSet, ctx: &CubeStructure) -> Cover {
    let mut result = Cover::new(ctx.size());
    let diff = ctx.fullset().difference(cube);
    for var in 0..ctx.num_vars() {
        if !diff.is_disjoint(ctx.var_mask(var)) {
            result.push(diff.merge_masked(ctx.fullset(), ctx.var_mask(var)));
        }
    }
    result
}

fn compl_d1merge(left: &mut Cover, right: &mut Cover, var: usize, ctx: &CubeStructure) {
    let mask = ctx.var_mask(var);
    for left_cube in &mut left.cubes {
        for right_cube in &mut right.cubes {
            if right_cube.is_active()
                && d1_order_key(left_cube, mask) == d1_order_key(right_cube, mask)
            {
                right_cube.set_active(false);
                *left_cube = left_cube.union(right_cube);
            }
        }
    }
}

fn compl_lift(a: &mut Cover, b: &Cover, bcube: &CubeSet, var: usize, ctx: &CubeStructure) {
    let mask = ctx.var_mask(var);
    let liftor = bcube.intersect(mask);
    for cube in &mut a.cubes {
        if !cube.is_active() {
            continue;
        }
        let lift = bcube.merge_masked(cube, mask);
        if b.cubes().iter().any(|other| lift.implies(other)) {
            *cube = cube.union(&liftor);
        }
    }
}

fn cb_recur_sharp(
    cube: &CubeSet,
    cover: &Cover,
    first: usize,
    last: usize,
    ctx: &CubeStructure,
) -> Cover {
    if first == last {
        sharp(cube, &cover.cubes[first], ctx)
    } else {
        let middle = (first + last) / 2;
        let left = cb_recur_sharp(cube, cover, first, middle, ctx);
        let right = cb_recur_sharp(cube, cover, middle + 1, last, ctx);
        cv_intersect(&left, &right, ctx)
    }
}

fn contain_with_mode(cover: &Cover, reverse: bool, remove_contained: bool) -> Cover {
    let mut cubes: Vec<_> = cover
        .cubes
        .iter()
        .cloned()
        .map(CubeSet::with_cached_order)
        .collect();
    cubes.sort_by(|a, b| compare_by_order(a, b, reverse));
    cubes.dedup();
    if !remove_contained {
        return Cover::from_cubes(cover.size(), cubes);
    }

    let mut kept: Vec<CubeSet> = Vec::new();
    'outer: for cube in cubes {
        for previous in &kept {
            let contained = if reverse {
                previous.implies(&cube)
            } else {
                cube.implies(previous)
            };
            if contained {
                continue 'outer;
            }
        }
        kept.push(cube);
    }
    Cover::from_cubes(cover.size(), kept)
}

fn compare_by_order(a: &CubeSet, b: &CubeSet, ascending: bool) -> Ordering {
    let ord = a.order().cmp(&b.order());
    let ord = if ascending { ord } else { ord.reverse() };
    ord.then_with(|| b.words.cmp(&a.words))
}

fn d1_order_key(cube: &CubeSet, mask: &CubeSet) -> Vec<u64> {
    cube.words
        .iter()
        .zip(&mask.words)
        .map(|(cube, mask)| cube | mask)
        .collect()
}

fn abs_covered(cover: &Cover, pick: usize) -> Cover {
    Cover::from_cubes(
        cover.size(),
        cover
            .cubes()
            .iter()
            .filter(|cube| !cube.contains(pick))
            .cloned()
            .collect(),
    )
}

fn abs_covered_many(cover: &Cover, pick_set: &CubeSet) -> Cover {
    Cover::from_cubes(
        cover.size(),
        cover
            .cubes()
            .iter()
            .filter(|cube| cube.is_disjoint(pick_set))
            .cloned()
            .collect(),
    )
}

fn abs_select_restricted(cover: &Cover, restricted: &CubeSet) -> usize {
    let mut count = vec![0usize; cover.size()];
    for cube in cover.cubes() {
        let weight = 1024 / cube.order().saturating_sub(1).max(1);
        for bit in bits(&cube.intersect(restricted)) {
            count[bit] += weight;
        }
    }
    count
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .map(|(bit, _)| bit)
        .expect("restricted set must be nonempty")
}

fn bits(set: &CubeSet) -> impl Iterator<Item = usize> + '_ {
    let size = set.size;
    set.words
        .iter()
        .enumerate()
        .flat_map(move |(word_index, word)| {
            (0..64).filter_map(move |bit| {
                let index = word_index * 64 + bit;
                (index < size && (word & (1u64 << bit)) != 0).then_some(index)
            })
        })
}

fn explode_minterms(
    cube: &CubeSet,
    ctx: &CubeStructure,
    var: usize,
    z: usize,
    output: &mut CubeSet,
) {
    for (offset, part) in (ctx.first_part(var)..=ctx.last_part(var)).enumerate() {
        if cube.contains(part) {
            let next = z * ctx.part_size(var) + offset;
            if var == 0 {
                output.insert(next);
            } else {
                explode_minterms(cube, ctx, var - 1, next, output);
            }
        }
    }
}

fn word_count(size: usize) -> usize {
    size.div_ceil(64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_binary_vars() -> CubeStructure {
        CubeStructure::new(2, vec![])
    }

    #[test]
    fn sharp_splits_a_cube_by_parts_missing_from_subtrahend() {
        let ctx = two_binary_vars();
        let a = ctx.full_cube();
        let b = ctx.binary_literal(0, true);

        let result = sharp(&a, &b, &ctx);

        assert_eq!(result.len(), 1);
        assert!(result.cubes()[0].contains(0));
        assert!(result.cubes()[0].contains(2));
        assert!(result.cubes()[0].contains(3));
        assert!(!result.cubes()[0].contains(1));
    }

    #[test]
    fn complement_of_single_literal_is_opposite_literal() {
        let ctx = two_binary_vars();
        let f = Cover::from_cubes(ctx.size(), vec![ctx.binary_literal(0, true)]);

        let result = complement(cube1list(&f, &ctx), &ctx);

        assert_eq!(result.len(), 1);
        assert!(result.cubes()[0].contains(0));
        assert!(!result.cubes()[0].contains(1));
        assert!(result.cubes()[0].contains(2));
        assert!(result.cubes()[0].contains(3));
    }

    #[test]
    fn containment_removes_cubes_implied_by_larger_cubes() {
        let ctx = two_binary_vars();
        let full = ctx.full_cube();
        let literal = ctx.binary_literal(1, false);
        let cover = Cover::from_cubes(ctx.size(), vec![literal, full.clone()]);

        let result = sf_contain(&cover);

        assert_eq!(result.cubes(), &[full]);
    }

    #[test]
    fn cofactor_keeps_only_intersecting_cubes_and_records_cofactored_parts() {
        let ctx = two_binary_vars();
        let on_x = ctx.binary_literal(0, true);
        let off_x = ctx.binary_literal(0, false);
        let cover = Cover::from_cubes(ctx.size(), vec![on_x.clone(), off_x]);

        let list = cofactor(&cube1list(&cover, &ctx), &on_x, &ctx);

        assert_eq!(list.len(), 0);
        assert!(list.cofactored().contains(0));
        assert!(!list.cofactored().contains(1));
    }

    #[test]
    fn unate_complement_computes_minimal_hitting_sets() {
        let table = Cover::from_cubes(
            3,
            vec![
                CubeSet::from_indices(3, [0, 1]),
                CubeSet::from_indices(3, [1, 2]),
            ],
        );

        let result = unate_compl(&table);

        let expected = Cover::from_cubes(
            3,
            vec![
                CubeSet::from_indices(3, [1]),
                CubeSet::from_indices(3, [0, 2]),
            ],
        );
        assert_eq!(sf_dupl(&result), sf_dupl(&expected));
    }

    #[test]
    fn minterms_uses_espresso_mixed_radix_order() {
        let ctx = two_binary_vars();
        let cover = Cover::from_cubes(ctx.size(), vec![ctx.full_cube()]);

        let result = minterms(&cover, &ctx);

        assert_eq!(result.order(), 4);
        for bit in 0..4 {
            assert!(result.contains(bit));
        }
    }
}
