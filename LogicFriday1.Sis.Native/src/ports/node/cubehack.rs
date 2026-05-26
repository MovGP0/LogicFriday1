//! Native cube sizing state for SIS node minimization helpers.
//!
//! The node package used Espresso's global cube descriptor as a reusable cache.
//! This port keeps the same sizing behavior in owned Rust data so callers can
//! manage contexts explicitly instead of mutating process-wide globals.

use std::error::Error;
use std::fmt;

const BITS_PER_WORD: usize = u32::BITS as usize;
const DISJOINT_MASK: u32 = 0x5555_5555;
const TEMP_SET_COUNT: usize = 10;
const SMALL_INPUT_CACHE_SIZE: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CubeHackError {
    InvalidCubeSize {
        binary_variables: usize,
        variables: usize,
    },
}

impl fmt::Display for CubeHackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCubeSize {
                binary_variables,
                variables,
            } => write!(
                f,
                "cube size is invalid: {binary_variables} binary variables in {variables} total variables"
            ),
        }
    }
}

impl Error for CubeHackError {}

pub type CubeHackResult<T> = Result<T, CubeHackError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackedSet {
    words: Vec<u32>,
}

impl PackedSet {
    pub fn clear(size: usize) -> Self {
        let mut words = vec![0; loop_init(size) + 1];
        words[0] = loop_init(size) as u32;
        Self { words }
    }

    pub fn full(size: usize) -> Self {
        let loop_count = loop_init(size);
        let mut words = vec![0; loop_count + 1];
        words[0] = loop_count as u32;

        if size > 0 {
            for word in words.iter_mut().take(loop_count).skip(1) {
                *word = u32::MAX;
            }

            let used_bits = size - ((loop_count - 1) * BITS_PER_WORD);
            words[loop_count] = if used_bits == BITS_PER_WORD {
                u32::MAX
            } else {
                (1u32 << used_bits) - 1
            };
        }

        Self { words }
    }

    pub fn insert(&mut self, element: usize) {
        self.words[which_word(element)] |= 1u32 << which_bit(element);
    }

    pub fn contains(&self, element: usize) -> bool {
        (self.words[which_word(element)] & (1u32 << which_bit(element))) != 0
    }

    pub fn words(&self) -> &[u32] {
        &self.words
    }

    fn union_assign(&mut self, other: &Self) {
        for index in 1..self.words.len().min(other.words.len()) {
            self.words[index] |= other.words[index];
        }
    }

    fn resize_full(&mut self, size: usize) {
        *self = Self::full(size);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeDescriptor {
    pub size: usize,
    pub num_vars: usize,
    pub num_binary_vars: usize,
    pub num_mv_vars: usize,
    pub output: Option<usize>,
    pub first_part: Vec<usize>,
    pub last_part: Vec<usize>,
    pub part_size: Vec<usize>,
    pub first_word: Vec<usize>,
    pub last_word: Vec<usize>,
    pub binary_mask: PackedSet,
    pub mv_mask: PackedSet,
    pub var_mask: Vec<PackedSet>,
    pub temp: Vec<PackedSet>,
    pub fullset: PackedSet,
    pub emptyset: PackedSet,
    pub inmask: u32,
    pub inword: Option<usize>,
    pub sparse: Vec<bool>,
}

impl CubeDescriptor {
    pub fn binary(input_count: usize) -> CubeHackResult<Self> {
        Self::new(input_count, input_count, vec![2; input_count])
    }

    pub fn new(
        num_vars: usize,
        num_binary_vars: usize,
        mut part_size: Vec<usize>,
    ) -> CubeHackResult<Self> {
        if num_binary_vars > num_vars {
            return Err(CubeHackError::InvalidCubeSize {
                binary_variables: num_binary_vars,
                variables: num_vars,
            });
        }

        if part_size.len() < num_vars {
            part_size.resize(num_vars, 2);
        }

        let num_mv_vars = num_vars - num_binary_vars;
        let output = if num_mv_vars > 0 {
            Some(num_vars - 1)
        } else {
            None
        };
        let mut size = 0;
        let mut first_part = Vec::with_capacity(num_vars);
        let mut last_part = Vec::with_capacity(num_vars);
        let mut first_word = Vec::with_capacity(num_vars);
        let mut last_word = Vec::with_capacity(num_vars);

        for variable in 0..num_vars {
            if variable < num_binary_vars {
                part_size[variable] = 2;
            }

            first_part.push(size);
            first_word.push(which_word(size));
            size += part_size[variable];
            last_part.push(size - 1);
            last_word.push(which_word(size - 1));
        }

        let mut descriptor = Self {
            size,
            num_vars,
            num_binary_vars,
            num_mv_vars,
            output,
            first_part,
            last_part,
            part_size,
            first_word,
            last_word,
            binary_mask: PackedSet::clear(size),
            mv_mask: PackedSet::clear(size),
            var_mask: Vec::with_capacity(num_vars),
            temp: (0..TEMP_SET_COUNT)
                .map(|_| PackedSet::clear(size))
                .collect(),
            fullset: PackedSet::full(size),
            emptyset: PackedSet::clear(size),
            inmask: 0,
            inword: None,
            sparse: vec![false; num_vars],
        };

        descriptor.rebuild_masks();
        Ok(descriptor)
    }

    fn shrink_binary_prefix(&mut self, input_count: usize) {
        self.num_vars = input_count;
        self.num_binary_vars = input_count;
        self.num_mv_vars = 0;
        self.output = None;
        self.size = input_count * 2;

        self.fullset.resize_full(self.size);
        self.binary_mask.resize_full(self.size);
        let loop_count = self.fullset.words[0];

        for mask in self.var_mask.iter_mut().take(input_count) {
            mask.words[0] = loop_count;
        }

        for temp in &mut self.temp {
            temp.words[0] = loop_count;
        }

        self.emptyset.words[0] = loop_count;
        self.mv_mask.words[0] = loop_count;

        if input_count == 0 {
            self.inword = None;
            self.inmask = 0;
        } else {
            let inword = self.last_word[input_count - 1];
            self.inword = Some(inword);
            self.inmask = self.binary_mask.words[inword] & DISJOINT_MASK;
        }
    }

    fn rebuild_masks(&mut self) {
        self.binary_mask = PackedSet::clear(self.size);
        self.mv_mask = PackedSet::clear(self.size);
        self.var_mask.clear();

        for variable in 0..self.num_vars {
            let mut mask = PackedSet::clear(self.size);
            for part in self.first_part[variable]..=self.last_part[variable] {
                mask.insert(part);
            }

            if variable < self.num_binary_vars {
                self.binary_mask.union_assign(&mask);
                self.sparse[variable] = false;
            } else {
                self.mv_mask.union_assign(&mask);
                self.sparse[variable] = true;
            }

            self.var_mask.push(mask);
        }

        if self.num_binary_vars == 0 {
            self.inword = None;
            self.inmask = 0;
        } else {
            let inword = self.last_word[self.num_binary_vars - 1];
            self.inword = Some(inword);
            self.inmask = self.binary_mask.words[inword] & DISJOINT_MASK;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeHackState {
    cube: Option<CubeDescriptor>,
    called_before: bool,
}

impl CubeHackState {
    pub fn new() -> Self {
        Self {
            cube: None,
            called_before: false,
        }
    }

    pub fn cube(&self) -> Option<&CubeDescriptor> {
        self.cube.as_ref()
    }

    pub fn cautious_define_cube_size(
        &mut self,
        input_count: usize,
    ) -> CubeHackResult<&CubeDescriptor> {
        if matches!(
            self.cube.as_ref(),
            Some(cube) if cube.num_binary_vars == input_count
        ) {
            return Ok(self.cube.as_ref().unwrap());
        }

        self.cube = Some(CubeDescriptor::binary(input_count)?);
        Ok(self.cube.as_ref().unwrap())
    }

    pub fn define_cube_size(&mut self, input_count: usize) -> CubeHackResult<&CubeDescriptor> {
        if matches!(
            self.cube.as_ref(),
            Some(cube) if cube.num_binary_vars == input_count && cube.num_vars == input_count
        ) {
            return Ok(self.cube.as_ref().unwrap());
        }

        if input_count > SMALL_INPUT_CACHE_SIZE {
            self.cautious_define_cube_size(input_count)?;
            self.called_before = false;
            return Ok(self.cube.as_ref().unwrap());
        }

        if self.cube.is_none() || !self.called_before {
            self.cautious_define_cube_size(SMALL_INPUT_CACHE_SIZE)?;
            self.called_before = true;
        }

        self.cube
            .as_mut()
            .expect("cube is initialized above")
            .shrink_binary_prefix(input_count);

        Ok(self.cube.as_ref().unwrap())
    }

    pub fn undefine_cube_size(&mut self) {
        if let Some(cube) = self.cube.as_mut() {
            if cube.num_binary_vars <= SMALL_INPUT_CACHE_SIZE {
                cube.num_vars = SMALL_INPUT_CACHE_SIZE;
                cube.num_binary_vars = SMALL_INPUT_CACHE_SIZE;
            }
        }

        self.cube = None;
    }
}

impl Default for CubeHackState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoFlags {
    pub summary: bool,
    pub trace: bool,
    pub remove_essential: bool,
    pub force_irredundant: bool,
    pub unwrap_onset: bool,
    pub single_expand: bool,
    pub pos: bool,
    pub recompute_onset: bool,
    pub use_super_gasp: bool,
    pub use_random_order: bool,
}

impl EspressoFlags {
    pub fn node_defaults() -> Self {
        Self {
            summary: false,
            trace: false,
            remove_essential: true,
            force_irredundant: true,
            unwrap_onset: true,
            single_expand: false,
            pos: false,
            recompute_onset: false,
            use_super_gasp: false,
            use_random_order: false,
        }
    }
}

impl Default for EspressoFlags {
    fn default() -> Self {
        Self::node_defaults()
    }
}

pub fn set_espresso_flags(flags: &mut EspressoFlags) {
    *flags = EspressoFlags::node_defaults();
}

fn which_word(element: usize) -> usize {
    (element / BITS_PER_WORD) + 1
}

fn which_bit(element: usize) -> usize {
    element % BITS_PER_WORD
}

fn loop_init(size: usize) -> usize {
    if size <= BITS_PER_WORD {
        1
    } else {
        which_word(size - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cautious_definition_allocates_exact_binary_context() {
        let mut state = CubeHackState::new();
        let cube = state.cautious_define_cube_size(4).unwrap();

        assert_eq!(cube.num_vars, 4);
        assert_eq!(cube.num_binary_vars, 4);
        assert_eq!(cube.num_mv_vars, 0);
        assert_eq!(cube.output, None);
        assert_eq!(cube.size, 8);
        assert_eq!(cube.part_size, vec![2, 2, 2, 2]);
        assert_eq!(cube.first_part, vec![0, 2, 4, 6]);
        assert_eq!(cube.last_part, vec![1, 3, 5, 7]);
        assert_eq!(cube.first_word, vec![1, 1, 1, 1]);
        assert_eq!(cube.last_word, vec![1, 1, 1, 1]);
        assert_eq!(cube.inword, Some(1));
        assert_eq!(cube.inmask, 0x55);
        assert_eq!(cube.fullset.words(), &[1, 0xff]);
        assert_eq!(cube.binary_mask.words(), &[1, 0xff]);
        assert_eq!(cube.mv_mask.words(), &[1, 0]);
        assert!(cube.var_mask[2].contains(4));
        assert!(cube.var_mask[2].contains(5));
        assert!(!cube.var_mask[2].contains(6));
    }

    #[test]
    fn small_define_reuses_one_hundred_input_context() {
        let mut state = CubeHackState::new();
        let cube = state.define_cube_size(3).unwrap();

        assert_eq!(cube.num_vars, 3);
        assert_eq!(cube.num_binary_vars, 3);
        assert_eq!(cube.size, 6);
        assert_eq!(cube.first_part[3], 6);
        assert_eq!(cube.part_size.len(), SMALL_INPUT_CACHE_SIZE);
        assert_eq!(cube.var_mask.len(), SMALL_INPUT_CACHE_SIZE);
        assert_eq!(cube.fullset.words(), &[1, 0x3f]);
        assert_eq!(cube.binary_mask.words(), &[1, 0x3f]);
        assert_eq!(cube.emptyset.words()[0], 1);
        assert_eq!(cube.temp.len(), TEMP_SET_COUNT);
        assert!(cube.temp.iter().all(|temp| temp.words()[0] == 1));
    }

    #[test]
    fn small_define_refreshes_cached_headers_across_word_boundaries() {
        let mut state = CubeHackState::new();
        state.define_cube_size(3).unwrap();
        let cube = state.define_cube_size(20).unwrap();

        assert_eq!(cube.size, 40);
        assert_eq!(cube.fullset.words()[0], 2);
        assert_eq!(cube.fullset.words()[1], u32::MAX);
        assert_eq!(cube.fullset.words()[2], 0xff);
        assert_eq!(cube.inword, Some(2));
        assert_eq!(cube.inmask, 0x55);
        assert!(
            cube.var_mask
                .iter()
                .take(20)
                .all(|mask| mask.words()[0] == 2)
        );
    }

    #[test]
    fn large_define_uses_exact_context_and_resets_small_cache_path() {
        let mut state = CubeHackState::new();
        state.define_cube_size(101).unwrap();
        let large = state.cube().unwrap();

        assert_eq!(large.num_vars, 101);
        assert_eq!(large.num_binary_vars, 101);
        assert_eq!(large.var_mask.len(), 101);
        assert_eq!(large.fullset.words()[0], 7);

        let small = state.define_cube_size(2).unwrap();
        assert_eq!(small.num_vars, 2);
        assert_eq!(small.num_binary_vars, 2);
        assert_eq!(small.part_size.len(), SMALL_INPUT_CACHE_SIZE);
    }

    #[test]
    fn undefine_releases_context() {
        let mut state = CubeHackState::new();
        state.define_cube_size(2).unwrap();

        state.undefine_cube_size();

        assert!(state.cube().is_none());
    }

    #[test]
    fn flags_match_node_minimization_defaults() {
        let mut flags = EspressoFlags {
            summary: true,
            trace: true,
            remove_essential: false,
            force_irredundant: false,
            unwrap_onset: false,
            single_expand: true,
            pos: true,
            recompute_onset: true,
            use_super_gasp: true,
            use_random_order: true,
        };

        set_espresso_flags(&mut flags);

        assert_eq!(flags, EspressoFlags::node_defaults());
        assert!(!flags.summary);
        assert!(!flags.trace);
        assert!(flags.remove_essential);
        assert!(flags.force_irredundant);
        assert!(flags.unwrap_onset);
        assert!(!flags.single_expand);
        assert!(!flags.pos);
        assert!(!flags.recompute_onset);
        assert!(!flags.use_super_gasp);
        assert!(!flags.use_random_order);
    }

    #[test]
    fn no_disallowed_porting_tokens_are_present() {
        let text = include_str!("cubehack.rs");

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
