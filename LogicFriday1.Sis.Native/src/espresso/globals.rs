//! Native Rust state for Espresso's former process globals.
//!
//! The C unit stores command flags, timing counters, PLA type lookup entries,
//! the active cube descriptor, and a byte popcount table as writable globals.
//! This port keeps the lookup data immutable and moves the mutable data into an
//! explicit `EspressoGlobals` value that callers can own, clone, and test.

use std::ops::{BitOr, BitOrAssign};

pub const TIME_COUNT: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeBucket
{
    Read = 0,
    Complement = 1,
    Onset = 2,
    Essential = 3,
    Expand = 4,
    Irredundant = 5,
    Reduce = 6,
    GaspExpand = 7,
    GaspIrredundant = 8,
    GaspReduce = 9,
    Primes = 10,
    MinimumCover = 11,
    MultipleValueReduce = 12,
    RaiseInput = 13,
    Verify = 14,
    Write = 15,
}

impl TimeBucket
{
    pub const ALL: [Self; TIME_COUNT] = [
        Self::Read,
        Self::Complement,
        Self::Onset,
        Self::Essential,
        Self::Expand,
        Self::Irredundant,
        Self::Reduce,
        Self::GaspExpand,
        Self::GaspIrredundant,
        Self::GaspReduce,
        Self::Primes,
        Self::MinimumCover,
        Self::MultipleValueReduce,
        Self::RaiseInput,
        Self::Verify,
        Self::Write,
    ];

    pub const fn index(self) -> usize
    {
        self as usize
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoTiming
{
    names: [Option<&'static str>; TIME_COUNT],
    elapsed_ticks: [i64; TIME_COUNT],
    calls: [usize; TIME_COUNT],
}

impl EspressoTiming
{
    pub const fn new() -> Self
    {
        Self {
            names: [None; TIME_COUNT],
            elapsed_ticks: [0; TIME_COUNT],
            calls: [0; TIME_COUNT],
        }
    }

    pub fn set_name(&mut self, bucket: TimeBucket, name: &'static str)
    {
        self.names[bucket.index()] = Some(name);
    }

    pub fn name(&self, bucket: TimeBucket) -> Option<&'static str>
    {
        self.names[bucket.index()]
    }

    pub fn elapsed_ticks(&self, bucket: TimeBucket) -> i64
    {
        self.elapsed_ticks[bucket.index()]
    }

    pub fn calls(&self, bucket: TimeBucket) -> usize
    {
        self.calls[bucket.index()]
    }

    pub fn record(&mut self, bucket: TimeBucket, elapsed_ticks: i64)
    {
        let index = bucket.index();
        self.elapsed_ticks[index] = self.elapsed_ticks[index].saturating_add(elapsed_ticks);
        self.calls[index] = self.calls[index].saturating_add(1);
    }

    pub fn reset(&mut self)
    {
        self.elapsed_ticks = [0; TIME_COUNT];
        self.calls = [0; TIME_COUNT];
    }
}

impl Default for EspressoTiming
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DebugFlags(u32);

impl DebugFlags
{
    pub const NONE: Self = Self(0);
    pub const COMPLEMENT: Self = Self(0x0001);
    pub const ESSENTIAL: Self = Self(0x0002);
    pub const EXPAND: Self = Self(0x0004);
    pub const EXPAND_STEP: Self = Self(0x0008);
    pub const GASP: Self = Self(0x0010);
    pub const IRREDUNDANT: Self = Self(0x0020);
    pub const REDUCE: Self = Self(0x0040);
    pub const REDUCE_STEP: Self = Self(0x0080);
    pub const SPARSE: Self = Self(0x0100);
    pub const TAUTOLOGY: Self = Self(0x0200);
    pub const EXACT: Self = Self(0x0400);
    pub const MINIMUM_COVER: Self = Self(0x0800);
    pub const MINIMUM_COVER_STEP: Self = Self(0x1000);
    pub const SHARP: Self = Self(0x2000);
    pub const IRREDUNDANT_STEP: Self = Self(0x4000);

    pub const fn bits(self) -> u32
    {
        self.0
    }

    pub const fn contains(self, flag: Self) -> bool
    {
        self.0 & flag.0 == flag.0
    }
}

impl BitOr for DebugFlags
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output
    {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for DebugFlags
{
    fn bitor_assign(&mut self, rhs: Self)
    {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlaType(u16);

impl PlaType
{
    pub const EMPTY: Self = Self(0);
    pub const F: Self = Self(1);
    pub const D: Self = Self(2);
    pub const R: Self = Self(4);
    pub const PLEASURE: Self = Self(8);
    pub const EQN_TO_TT: Self = Self(16);
    pub const KISS: Self = Self(128);
    pub const CONSTRAINTS: Self = Self(256);
    pub const SYMBOLIC_CONSTRAINTS: Self = Self(512);
    pub const FD: Self = Self(Self::F.0 | Self::D.0);
    pub const FR: Self = Self(Self::F.0 | Self::R.0);
    pub const DR: Self = Self(Self::D.0 | Self::R.0);
    pub const FDR: Self = Self(Self::F.0 | Self::D.0 | Self::R.0);

    pub const fn bits(self) -> u16
    {
        self.0
    }

    pub const fn contains(self, flag: Self) -> bool
    {
        self.0 & flag.0 == flag.0
    }
}

impl BitOr for PlaType
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output
    {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for PlaType
{
    fn bitor_assign(&mut self, rhs: Self)
    {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlaTypeEntry
{
    pub key: &'static str,
    pub value: PlaType,
}

pub const PLA_TYPES: [PlaTypeEntry; 20] = [
    PlaTypeEntry { key: "-f", value: PlaType::F },
    PlaTypeEntry { key: "-r", value: PlaType::R },
    PlaTypeEntry { key: "-d", value: PlaType::D },
    PlaTypeEntry { key: "-fd", value: PlaType::FD },
    PlaTypeEntry { key: "-fr", value: PlaType::FR },
    PlaTypeEntry { key: "-dr", value: PlaType::DR },
    PlaTypeEntry { key: "-fdr", value: PlaType::FDR },
    PlaTypeEntry { key: "-fc", value: PlaType(PlaType::F.0 | PlaType::CONSTRAINTS.0) },
    PlaTypeEntry { key: "-rc", value: PlaType(PlaType::R.0 | PlaType::CONSTRAINTS.0) },
    PlaTypeEntry { key: "-dc", value: PlaType(PlaType::D.0 | PlaType::CONSTRAINTS.0) },
    PlaTypeEntry { key: "-fdc", value: PlaType(PlaType::FD.0 | PlaType::CONSTRAINTS.0) },
    PlaTypeEntry { key: "-frc", value: PlaType(PlaType::FR.0 | PlaType::CONSTRAINTS.0) },
    PlaTypeEntry { key: "-drc", value: PlaType(PlaType::DR.0 | PlaType::CONSTRAINTS.0) },
    PlaTypeEntry { key: "-fdrc", value: PlaType(PlaType::FDR.0 | PlaType::CONSTRAINTS.0) },
    PlaTypeEntry { key: "-pleasure", value: PlaType::PLEASURE },
    PlaTypeEntry { key: "-eqn", value: PlaType::EQN_TO_TT },
    PlaTypeEntry { key: "-eqntott", value: PlaType::EQN_TO_TT },
    PlaTypeEntry { key: "-kiss", value: PlaType::KISS },
    PlaTypeEntry { key: "-cons", value: PlaType::CONSTRAINTS },
    PlaTypeEntry { key: "-scons", value: PlaType::SYMBOLIC_CONSTRAINTS },
];

pub fn pla_type_for_key(key: &str) -> Option<PlaType>
{
    PLA_TYPES
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.value)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoOptions
{
    pub debug: DebugFlags,
    pub verbose_debug: bool,
    pub echo_comments: bool,
    pub echo_unknown_commands: bool,
    pub force_irredundant: bool,
    pub skip_make_sparse: bool,
    pub kiss: bool,
    pub pos: bool,
    pub print_solution: bool,
    pub recompute_onset: bool,
    pub remove_essential: bool,
    pub single_expand: bool,
    pub summary: bool,
    pub trace: bool,
    pub unwrap_onset: bool,
    pub use_random_order: bool,
    pub use_super_gasp: bool,
    pub filename: Option<String>,
}

impl EspressoOptions
{
    pub fn c_zeroed() -> Self
    {
        Self {
            debug: DebugFlags::NONE,
            verbose_debug: false,
            echo_comments: false,
            echo_unknown_commands: false,
            force_irredundant: false,
            skip_make_sparse: false,
            kiss: false,
            pos: false,
            print_solution: false,
            recompute_onset: false,
            remove_essential: false,
            single_expand: false,
            summary: false,
            trace: false,
            unwrap_onset: false,
            use_random_order: false,
            use_super_gasp: false,
            filename: None,
        }
    }

    pub fn sis_minimization_defaults() -> Self
    {
        Self {
            remove_essential: true,
            force_irredundant: true,
            unwrap_onset: true,
            ..Self::c_zeroed()
        }
    }
}

impl Default for EspressoOptions
{
    fn default() -> Self
    {
        Self::c_zeroed()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CubeGlobals
{
    pub size: usize,
    pub num_vars: usize,
    pub num_binary_vars: usize,
    pub first_part: Vec<usize>,
    pub last_part: Vec<usize>,
    pub part_size: Vec<isize>,
    pub first_word: Vec<usize>,
    pub last_word: Vec<usize>,
    pub sparse: Vec<bool>,
    pub num_mv_vars: usize,
    pub output: Option<usize>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CoverDataGlobals
{
    pub part_zeros: Vec<usize>,
    pub var_zeros: Vec<usize>,
    pub parts_active: Vec<usize>,
    pub is_unate: Vec<bool>,
    pub vars_active: usize,
    pub vars_unate: usize,
    pub best: Option<usize>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EspressoGlobals
{
    pub options: EspressoOptions,
    pub timing: EspressoTiming,
    pub cube: CubeGlobals,
    pub saved_cube: CubeGlobals,
    pub cover_data: CoverDataGlobals,
    pub saved_cover_data: CoverDataGlobals,
}

impl EspressoGlobals
{
    pub fn c_zeroed() -> Self
    {
        Self::default()
    }

    pub fn sis_minimization_defaults() -> Self
    {
        Self {
            options: EspressoOptions::sis_minimization_defaults(),
            ..Self::default()
        }
    }
}

pub const BIT_COUNT: [u8; 256] = [
    0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4,
    1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5,
    1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5,
    2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6,
    1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5,
    2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6,
    2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6,
    3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6, 5, 6, 6, 7,
    1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5,
    2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6,
    2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6,
    3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6, 5, 6, 6, 7,
    2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6,
    3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6, 5, 6, 6, 7,
    3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6, 5, 6, 6, 7,
    4, 5, 5, 6, 5, 6, 6, 7, 5, 6, 6, 7, 6, 7, 7, 8,
];

pub const fn count_ones_byte(value: u8) -> u8
{
    BIT_COUNT[value as usize]
}

pub fn count_ones_word(value: u32) -> u8
{
    count_ones_byte((value & 0xff) as u8)
        + count_ones_byte(((value >> 8) & 0xff) as u8)
        + count_ones_byte(((value >> 16) & 0xff) as u8)
        + count_ones_byte(((value >> 24) & 0xff) as u8)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn pla_type_lookup_matches_espresso_table()
    {
        assert_eq!(PLA_TYPES.len(), 20);
        assert_eq!(pla_type_for_key("-f"), Some(PlaType::F));
        assert_eq!(pla_type_for_key("-fd"), Some(PlaType::FD));
        assert_eq!(
            pla_type_for_key("-fdrc"),
            Some(PlaType::FDR | PlaType::CONSTRAINTS)
        );
        assert_eq!(pla_type_for_key("-eqn"), Some(PlaType::EQN_TO_TT));
        assert_eq!(pla_type_for_key("-eqntott"), Some(PlaType::EQN_TO_TT));
        assert_eq!(pla_type_for_key("-missing"), None);
    }

    #[test]
    fn c_zeroed_options_match_static_storage_initialization()
    {
        let options = EspressoOptions::c_zeroed();

        assert_eq!(options.debug, DebugFlags::NONE);
        assert!(!options.trace);
        assert!(!options.force_irredundant);
        assert!(!options.unwrap_onset);
        assert_eq!(options.filename, None);
    }

    #[test]
    fn sis_minimization_defaults_match_set_espresso_flags()
    {
        let options = EspressoOptions::sis_minimization_defaults();

        assert!(options.remove_essential);
        assert!(options.force_irredundant);
        assert!(options.unwrap_onset);
        assert!(!options.summary);
        assert!(!options.trace);
        assert!(!options.single_expand);
        assert!(!options.pos);
        assert!(!options.recompute_onset);
        assert!(!options.use_super_gasp);
        assert!(!options.use_random_order);
    }

    #[test]
    fn timing_records_names_elapsed_ticks_and_call_counts()
    {
        let mut timing = EspressoTiming::new();

        timing.set_name(TimeBucket::Expand, "EXPAND");
        timing.record(TimeBucket::Expand, 7);
        timing.record(TimeBucket::Expand, 11);

        assert_eq!(timing.name(TimeBucket::Expand), Some("EXPAND"));
        assert_eq!(timing.elapsed_ticks(TimeBucket::Expand), 18);
        assert_eq!(timing.calls(TimeBucket::Expand), 2);

        timing.reset();

        assert_eq!(timing.name(TimeBucket::Expand), Some("EXPAND"));
        assert_eq!(timing.elapsed_ticks(TimeBucket::Expand), 0);
        assert_eq!(timing.calls(TimeBucket::Expand), 0);
    }

    #[test]
    fn bit_count_table_matches_native_popcount_for_every_byte()
    {
        for value in u8::MIN..=u8::MAX
        {
            assert_eq!(count_ones_byte(value), value.count_ones() as u8);
        }

        assert_eq!(count_ones_word(0x5555_ffff), 24);
    }
}
