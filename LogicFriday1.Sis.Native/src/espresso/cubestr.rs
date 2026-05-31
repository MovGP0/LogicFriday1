//! Native Rust cube-structure setup for Espresso.
//!
//! The original implementation initialized global `cube` and `cdata` records
//! from a partially-filled parser state. This module keeps the same derived
//! layout data in owned Rust structures and models save/restore through normal
//! ownership instead of process-wide globals.

use super::set::Set;
use std::error::Error;
use std::fmt;

pub const CUBE_TEMP_COUNT: usize = 10;

const BITS_PER_WORD: usize = u32::BITS as usize;
const DISJOINT_MASK: u32 = 0x5555_5555;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeSetupInput
{
    num_vars: usize,
    num_binary_vars: usize,
    part_size: Vec<isize>,
}

impl CubeSetupInput
{
    pub fn new(
        num_vars: usize,
        num_binary_vars: usize,
        part_size: impl IntoIterator<Item = isize>,
    ) -> Result<Self, CubestrError>
    {
        let part_size = part_size.into_iter().collect::<Vec<_>>();
        if num_binary_vars > num_vars
        {
            return Err(CubestrError::BinaryVariableCountExceedsVariableCount {
                num_vars,
                num_binary_vars,
            });
        }

        if part_size.len() < num_vars
        {
            return Err(CubestrError::PartSizeCountTooSmall {
                expected: num_vars,
                actual: part_size.len(),
            });
        }

        for (variable, size) in part_size.iter().copied().enumerate().skip(num_binary_vars)
        {
            if size == 0
            {
                return Err(CubestrError::EmptyPartSize { variable });
            }
        }

        Ok(Self {
            num_vars,
            num_binary_vars,
            part_size,
        })
    }

    pub const fn num_vars(&self) -> usize
    {
        self.num_vars
    }

    pub const fn num_binary_vars(&self) -> usize
    {
        self.num_binary_vars
    }

    pub fn part_size(&self) -> &[isize]
    {
        &self.part_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure
{
    size: usize,
    num_vars: usize,
    num_binary_vars: usize,
    num_mv_vars: usize,
    output: Option<usize>,
    first_part: Vec<usize>,
    last_part: Vec<usize>,
    part_size: Vec<usize>,
    symbolic_part: Vec<bool>,
    first_word: Vec<usize>,
    last_word: Vec<usize>,
    binary_mask: Set,
    mv_mask: Set,
    var_mask: Vec<Set>,
    temp: Vec<Set>,
    fullset: Set,
    emptyset: Set,
    inmask: u32,
    inword: Option<usize>,
    sparse: Vec<bool>,
}

impl CubeStructure
{
    pub fn setup(input: CubeSetupInput) -> Self
    {
        let num_mv_vars = input.num_vars - input.num_binary_vars;
        let output = if num_mv_vars > 0
        {
            Some(input.num_vars - 1)
        }
        else
        {
            None
        };

        let mut size = 0;
        let mut first_part = Vec::with_capacity(input.num_vars);
        let mut last_part = Vec::with_capacity(input.num_vars);
        let mut part_size = Vec::with_capacity(input.num_vars);
        let mut symbolic_part = Vec::with_capacity(input.num_vars);
        let mut first_word = Vec::with_capacity(input.num_vars);
        let mut last_word = Vec::with_capacity(input.num_vars);

        for variable in 0..input.num_vars
        {
            let raw_size = if variable < input.num_binary_vars
            {
                2
            }
            else
            {
                input.part_size[variable]
            };
            let variable_part_size = raw_size.unsigned_abs();

            first_part.push(size);
            first_word.push(which_word(size));
            size += variable_part_size;
            last_part.push(size - 1);
            last_word.push(which_word(size - 1));
            part_size.push(variable_part_size);
            symbolic_part.push(variable >= input.num_binary_vars && raw_size < 0);
        }

        let mut binary_mask = Set::empty(size);
        let mut mv_mask = Set::empty(size);
        let mut var_mask = Vec::with_capacity(input.num_vars);
        let mut sparse = Vec::with_capacity(input.num_vars);

        for variable in 0..input.num_vars
        {
            let mask = Set::from_elements(size, first_part[variable]..=last_part[variable]);
            if variable < input.num_binary_vars
            {
                binary_mask = binary_mask.union(&mask);
                sparse.push(false);
            }
            else
            {
                mv_mask = mv_mask.union(&mask);
                sparse.push(true);
            }

            var_mask.push(mask);
        }

        let inword = input
            .num_binary_vars
            .checked_sub(1)
            .map(|last_binary_variable| last_word[last_binary_variable]);
        let inmask = inword
            .map(|word| binary_mask.words()[word] & DISJOINT_MASK)
            .unwrap_or(0);
        let temp = (0..CUBE_TEMP_COUNT).map(|_| Set::empty(size)).collect();

        Self {
            size,
            num_vars: input.num_vars,
            num_binary_vars: input.num_binary_vars,
            num_mv_vars,
            output,
            first_part,
            last_part,
            part_size,
            symbolic_part,
            first_word,
            last_word,
            binary_mask,
            mv_mask,
            var_mask,
            temp,
            fullset: Set::full(size),
            emptyset: Set::empty(size),
            inmask,
            inword,
            sparse,
        }
    }

    pub const fn size(&self) -> usize
    {
        self.size
    }

    pub const fn num_vars(&self) -> usize
    {
        self.num_vars
    }

    pub const fn num_binary_vars(&self) -> usize
    {
        self.num_binary_vars
    }

    pub const fn num_mv_vars(&self) -> usize
    {
        self.num_mv_vars
    }

    pub const fn output(&self) -> Option<usize>
    {
        self.output
    }

    pub fn first_part(&self) -> &[usize]
    {
        &self.first_part
    }

    pub fn last_part(&self) -> &[usize]
    {
        &self.last_part
    }

    pub fn part_size(&self) -> &[usize]
    {
        &self.part_size
    }

    pub fn symbolic_part(&self) -> &[bool]
    {
        &self.symbolic_part
    }

    pub fn first_word(&self) -> &[usize]
    {
        &self.first_word
    }

    pub fn last_word(&self) -> &[usize]
    {
        &self.last_word
    }

    pub fn binary_mask(&self) -> &Set
    {
        &self.binary_mask
    }

    pub fn mv_mask(&self) -> &Set
    {
        &self.mv_mask
    }

    pub fn var_mask(&self) -> &[Set]
    {
        &self.var_mask
    }

    pub fn temp(&self) -> &[Set]
    {
        &self.temp
    }

    pub fn fullset(&self) -> &Set
    {
        &self.fullset
    }

    pub fn emptyset(&self) -> &Set
    {
        &self.emptyset
    }

    pub const fn inmask(&self) -> u32
    {
        self.inmask
    }

    pub const fn inword(&self) -> Option<usize>
    {
        self.inword
    }

    pub fn sparse(&self) -> &[bool]
    {
        &self.sparse
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeData
{
    part_zeros: Vec<usize>,
    var_zeros: Vec<usize>,
    parts_active: Vec<usize>,
    is_unate: Vec<bool>,
    vars_active: usize,
    vars_unate: usize,
}

impl CubeData
{
    pub fn new(cube: &CubeStructure) -> Self
    {
        Self {
            part_zeros: vec![0; cube.size()],
            var_zeros: vec![0; cube.num_vars()],
            parts_active: vec![0; cube.num_vars()],
            is_unate: vec![false; cube.num_vars()],
            vars_active: 0,
            vars_unate: 0,
        }
    }

    pub fn part_zeros(&self) -> &[usize]
    {
        &self.part_zeros
    }

    pub fn var_zeros(&self) -> &[usize]
    {
        &self.var_zeros
    }

    pub fn parts_active(&self) -> &[usize]
    {
        &self.parts_active
    }

    pub fn is_unate(&self) -> &[bool]
    {
        &self.is_unate
    }

    pub const fn vars_active(&self) -> usize
    {
        self.vars_active
    }

    pub const fn vars_unate(&self) -> usize
    {
        self.vars_unate
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeContext
{
    cube: Option<CubeStructure>,
    cdata: Option<CubeData>,
}

impl CubeContext
{
    pub fn setup(input: CubeSetupInput) -> Self
    {
        let cube = CubeStructure::setup(input);
        let cdata = CubeData::new(&cube);

        Self {
            cube: Some(cube),
            cdata: Some(cdata),
        }
    }

    pub fn cube(&self) -> Option<&CubeStructure>
    {
        self.cube.as_ref()
    }

    pub fn cdata(&self) -> Option<&CubeData>
    {
        self.cdata.as_ref()
    }

    pub fn setdown(&mut self)
    {
        self.cube = None;
        self.cdata = None;
    }

    pub fn save(&mut self) -> SavedCubeContext
    {
        SavedCubeContext {
            cube: self.cube.take(),
            cdata: self.cdata.take(),
        }
    }

    pub fn restore(&mut self, saved: SavedCubeContext)
    {
        self.cube = saved.cube;
        self.cdata = saved.cdata;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedCubeContext
{
    cube: Option<CubeStructure>,
    cdata: Option<CubeData>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CubestrError
{
    BinaryVariableCountExceedsVariableCount
    {
        num_vars: usize,
        num_binary_vars: usize,
    },
    PartSizeCountTooSmall
    {
        expected: usize,
        actual: usize,
    },
    EmptyPartSize
    {
        variable: usize,
    },
}

impl fmt::Display for CubestrError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::BinaryVariableCountExceedsVariableCount {
                num_vars,
                num_binary_vars,
            } => write!(
                formatter,
                "binary variable count {num_binary_vars} exceeds variable count {num_vars}"
            ),
            Self::PartSizeCountTooSmall { expected, actual } => write!(
                formatter,
                "part-size table has {actual} entries for {expected} variables"
            ),
            Self::EmptyPartSize { variable } => {
                write!(formatter, "variable {variable} has no parts")
            }
        }
    }
}

impl Error for CubestrError {}

pub type CubestrResult<T> = Result<T, CubestrError>;

pub fn cube_setup(input: CubeSetupInput) -> CubeContext
{
    CubeContext::setup(input)
}

pub fn setdown_cube(context: &mut CubeContext)
{
    context.setdown();
}

pub fn save_cube_struct(context: &mut CubeContext) -> SavedCubeContext
{
    context.save()
}

pub fn restore_cube_struct(context: &mut CubeContext, saved: SavedCubeContext)
{
    context.restore(saved);
}

fn which_word(element: usize) -> usize
{
    (element / BITS_PER_WORD) + 1
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn input() -> CubeSetupInput
    {
        CubeSetupInput::new(4, 2, [0, 0, 3, -4]).unwrap()
    }

    #[test]
    fn setup_derives_part_ranges_words_masks_and_sparse_flags()
    {
        let context = cube_setup(input());
        let cube = context.cube().unwrap();

        assert_eq!(cube.size(), 11);
        assert_eq!(cube.num_vars(), 4);
        assert_eq!(cube.num_binary_vars(), 2);
        assert_eq!(cube.num_mv_vars(), 2);
        assert_eq!(cube.output(), Some(3));
        assert_eq!(cube.first_part(), &[0, 2, 4, 7]);
        assert_eq!(cube.last_part(), &[1, 3, 6, 10]);
        assert_eq!(cube.part_size(), &[2, 2, 3, 4]);
        assert_eq!(cube.symbolic_part(), &[false, false, false, true]);
        assert_eq!(cube.first_word(), &[1, 1, 1, 1]);
        assert_eq!(cube.last_word(), &[1, 1, 1, 1]);
        assert_eq!(cube.sparse(), &[false, false, true, true]);
        assert_eq!(cube.binary_mask().to_bit_string(cube.size()), "11110000000");
        assert_eq!(cube.mv_mask().to_bit_string(cube.size()), "00001111111");
        assert_eq!(cube.var_mask()[2].to_bit_string(cube.size()), "00001110000");
        assert_eq!(cube.fullset().cardinality(), cube.size());
        assert!(cube.emptyset().is_empty());
        assert_eq!(cube.temp().len(), CUBE_TEMP_COUNT);
        assert!(cube.temp().iter().all(Set::is_empty));
    }

    #[test]
    fn setup_computes_input_word_and_disjoint_mask_for_binary_variables()
    {
        let cube = cube_setup(input()).cube().unwrap().clone();

        assert_eq!(cube.inword(), Some(1));
        assert_eq!(cube.inmask(), 0b0101);

        let no_binary = cube_setup(CubeSetupInput::new(2, 0, [3, 2]).unwrap());
        let no_binary_cube = no_binary.cube().unwrap();

        assert_eq!(no_binary_cube.inword(), None);
        assert_eq!(no_binary_cube.inmask(), 0);
    }

    #[test]
    fn cdata_tables_are_allocated_from_cube_dimensions()
    {
        let context = cube_setup(input());
        let cdata = context.cdata().unwrap();

        assert_eq!(cdata.part_zeros(), vec![0; 11]);
        assert_eq!(cdata.var_zeros(), vec![0; 4]);
        assert_eq!(cdata.parts_active(), vec![0; 4]);
        assert_eq!(cdata.is_unate(), vec![false; 4]);
        assert_eq!(cdata.vars_active(), 0);
        assert_eq!(cdata.vars_unate(), 0);
    }

    #[test]
    fn setdown_drops_cube_and_cdata_state()
    {
        let mut context = cube_setup(input());

        setdown_cube(&mut context);

        assert_eq!(context.cube(), None);
        assert_eq!(context.cdata(), None);
    }

    #[test]
    fn save_moves_state_out_and_restore_puts_it_back()
    {
        let mut context = cube_setup(input());
        let saved = save_cube_struct(&mut context);

        assert_eq!(context.cube(), None);
        assert_eq!(context.cdata(), None);

        restore_cube_struct(&mut context, saved);

        assert_eq!(context.cube().unwrap().size(), 11);
        assert_eq!(context.cdata().unwrap().part_zeros().len(), 11);
    }

    #[test]
    fn input_validation_rejects_silly_cube_shapes()
    {
        assert_eq!(
            CubeSetupInput::new(1, 2, [0, 0]).unwrap_err(),
            CubestrError::BinaryVariableCountExceedsVariableCount {
                num_vars: 1,
                num_binary_vars: 2,
            }
        );
        assert_eq!(
            CubeSetupInput::new(3, 1, [0, 2]).unwrap_err(),
            CubestrError::PartSizeCountTooSmall {
                expected: 3,
                actual: 2,
            }
        );
        assert_eq!(
            CubeSetupInput::new(2, 1, [0, 0]).unwrap_err(),
            CubestrError::EmptyPartSize { variable: 1 }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("cubestr.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }
}
