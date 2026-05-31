#![allow(special_module_name)]

pub mod cols;
pub mod cofactor;
pub mod compl;
pub mod contain;
pub mod cover_ops;
pub mod cubestr;
pub mod cvrin;
pub mod cvrmisc;
pub mod cvrm;
pub mod cvrout;
pub mod dominate;
pub mod driver;
pub mod equiv;
pub mod espresso;
pub mod essen;
pub mod exact;
pub mod expand;
pub mod foundation;
pub mod gasp;
pub mod gimpel;
pub mod getopt;
pub mod globals;
pub mod hack;
pub mod indep;
pub mod irred;
pub mod main;
pub mod map;
pub mod matrix;
pub mod mincov;
pub mod minimize;
pub mod opo;
pub mod pair;
pub mod part;
pub mod pla;
pub mod primes;
pub mod reduce;
pub mod rows;
pub mod set;
pub mod setc;
pub mod sminterf;
pub mod solution;
pub mod sparse;
pub mod unate;
pub mod verify;

/// Returns the native Espresso interop ABI version.
///
/// This bootstrap surface keeps the native crate buildable while the Espresso
/// C modules are ported to pure Rust.

pub fn abi_version() -> i32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn logicfriday1_espresso_abi_version() -> i32 {
    abi_version()
}
