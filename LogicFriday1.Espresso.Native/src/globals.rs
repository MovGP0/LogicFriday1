//! Global constants and configuration state ported from Espresso's `globals.c`.
//!
//! The original C file defines process-global mutable variables. This Rust port
//! keeps the same configuration surface in an owned `EspressoGlobals` value so
//! callers can pass state explicitly and avoid global mutable aliasing.

pub const BPI: usize = 32;
pub const LOGBPI: usize = 5;
pub const DISJOINT: u32 = 0x5555_5555;
pub const CUBE_TEMP: usize = 10;

pub const PRIME: u32 = 0x8000;
pub const NONESSEN: u32 = 0x4000;
pub const ACTIVE: u32 = 0x2000;
pub const REDUND: u32 = 0x1000;
pub const COVERED: u32 = 0x0800;
pub const RELESSEN: u32 = 0x0400;

pub const F_TYPE: i32 = 1;
pub const D_TYPE: i32 = 2;
pub const R_TYPE: i32 = 4;
pub const PLEASURE_TYPE: i32 = 8;
pub const EQNTOTT_TYPE: i32 = 16;
pub const KISS_TYPE: i32 = 128;
pub const CONSTRAINTS_TYPE: i32 = 256;
pub const SYMBOLIC_CONSTRAINTS_TYPE: i32 = 512;
pub const FD_TYPE: i32 = F_TYPE | D_TYPE;
pub const FR_TYPE: i32 = F_TYPE | R_TYPE;
pub const DR_TYPE: i32 = D_TYPE | R_TYPE;
pub const FDR_TYPE: i32 = F_TYPE | D_TYPE | R_TYPE;

pub const TWO: u8 = 3;
pub const DASH: u8 = 3;
pub const ONE: u8 = 2;
pub const ZERO: u8 = 1;

pub const VERSION: &str = "UC Berkeley, Espresso Version #2.3, Release date 01/31/88";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaType {
    pub key: &'static str,
    pub value: i32,
}

pub const PLA_TYPES: &[PlaType] = &[
    PlaType {
        key: "-f",
        value: F_TYPE,
    },
    PlaType {
        key: "-r",
        value: R_TYPE,
    },
    PlaType {
        key: "-d",
        value: D_TYPE,
    },
    PlaType {
        key: "-fd",
        value: FD_TYPE,
    },
    PlaType {
        key: "-fr",
        value: FR_TYPE,
    },
    PlaType {
        key: "-dr",
        value: DR_TYPE,
    },
    PlaType {
        key: "-fdr",
        value: FDR_TYPE,
    },
    PlaType {
        key: "-fc",
        value: F_TYPE | CONSTRAINTS_TYPE,
    },
    PlaType {
        key: "-rc",
        value: R_TYPE | CONSTRAINTS_TYPE,
    },
    PlaType {
        key: "-dc",
        value: D_TYPE | CONSTRAINTS_TYPE,
    },
    PlaType {
        key: "-fdc",
        value: FD_TYPE | CONSTRAINTS_TYPE,
    },
    PlaType {
        key: "-frc",
        value: FR_TYPE | CONSTRAINTS_TYPE,
    },
    PlaType {
        key: "-drc",
        value: DR_TYPE | CONSTRAINTS_TYPE,
    },
    PlaType {
        key: "-fdrc",
        value: FDR_TYPE | CONSTRAINTS_TYPE,
    },
    PlaType {
        key: "-pleasure",
        value: PLEASURE_TYPE,
    },
    PlaType {
        key: "-eqn",
        value: EQNTOTT_TYPE,
    },
    PlaType {
        key: "-eqntott",
        value: EQNTOTT_TYPE,
    },
    PlaType {
        key: "-kiss",
        value: KISS_TYPE,
    },
    PlaType {
        key: "-cons",
        value: CONSTRAINTS_TYPE,
    },
    PlaType {
        key: "-scons",
        value: SYMBOLIC_CONSTRAINTS_TYPE,
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EspressoGlobals {
    pub debug: u32,
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
    pub debug_exact_minimization: bool,
    pub filename: Option<String>,
}

pub const BIT_COUNT: [u8; 256] = build_bit_count();

pub const fn byte_bit_count(byte: u8) -> u8 {
    BIT_COUNT[byte as usize]
}

const fn build_bit_count() -> [u8; 256] {
    let mut counts = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        counts[i] = (i as u8).count_ones() as u8;
        i += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pla_type_table_matches_globals_c_tokens() {
        assert_eq!(PLA_TYPES.first().unwrap().key, "-f");
        assert_eq!(PLA_TYPES.last().unwrap().key, "-scons");
        assert_eq!(
            PLA_TYPES
                .iter()
                .find(|entry| entry.key == "-fdrc")
                .unwrap()
                .value,
            FDR_TYPE | CONSTRAINTS_TYPE
        );
        assert_eq!(
            PLA_TYPES
                .iter()
                .find(|entry| entry.key == "-eqntott")
                .unwrap()
                .value,
            EQNTOTT_TYPE
        );
    }

    #[test]
    fn byte_bit_count_matches_globals_c_lookup_table() {
        assert_eq!(byte_bit_count(0), 0);
        assert_eq!(byte_bit_count(1), 1);
        assert_eq!(byte_bit_count(0b1010_1010), 4);
        assert_eq!(byte_bit_count(u8::MAX), 8);
        for byte in 0u8..=u8::MAX {
            assert_eq!(byte_bit_count(byte), byte.count_ones() as u8);
        }
    }

    #[test]
    fn globals_are_owned_state_instead_of_mutable_process_globals() {
        let globals = EspressoGlobals {
            verbose_debug: true,
            filename: Some("input.pla".to_string()),
            ..EspressoGlobals::default()
        };

        assert!(globals.verbose_debug);
        assert_eq!(globals.filename.as_deref(), Some("input.pla"));
        assert!(!globals.force_irredundant);
    }
}
