#![allow(special_module_name)]

mod facade;

pub mod array;
pub mod astg;
pub mod atpg;
pub mod avl;
pub mod bdd_cmu;
pub mod bdd_ucb;
pub mod clock;
pub mod command;
pub mod decomp;
pub mod delay;
pub mod enc;
pub mod error;
pub mod espresso;
pub mod extract;
pub mod factor;
pub mod gcd;
pub mod genlib;
pub mod graph;
pub mod graphics;
pub mod io;
pub mod latch;
pub mod linsolv;
pub mod list;
pub mod main;
pub mod map;
pub mod maxflow;
pub mod mincov;
pub mod minimize;
pub mod network;
pub mod node;
pub mod ntbdd;
pub mod octio;
pub mod order;
pub mod phase;
pub mod pld;
pub mod power;
pub mod resub;
pub mod retime;
pub mod seqbdd;
pub mod sim;
pub mod simplify;
pub mod sparse;
pub mod speed;
pub mod st;
pub mod stg;
pub mod test;
pub mod timing;
pub mod util;
pub mod var_set;

/// Returns the native SIS interop ABI version.
///
/// This is intentionally small bootstrap surface. Real SIS functions should be
/// added as the corresponding C modules are ported.
pub fn abi_version() -> i32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn logicfriday1_sis_abi_version() -> i32 {
    abi_version()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn logicfriday1_sis_map_blif_to_json(
    blif_ptr: *const u8,
    blif_len: usize,
    options: u32,
    output_ptr: *mut u8,
    output_len: usize,
) -> usize {
    unsafe { facade::map_blif_to_json(blif_ptr, blif_len, options, output_ptr, output_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn logicfriday1_sis_map_blif_genlib_to_json(
    blif_ptr: *const u8,
    blif_len: usize,
    genlib_ptr: *const u8,
    genlib_len: usize,
    options: u32,
    output_ptr: *mut u8,
    output_len: usize,
) -> usize {
    unsafe {
        facade::map_blif_genlib_to_json(
            blif_ptr, blif_len, genlib_ptr, genlib_len, options, output_ptr, output_len,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn logicfriday1_sis_last_error(
    output_ptr: *mut u8,
    output_len: usize,
) -> usize {
    unsafe { facade::last_error(output_ptr, output_len) }
}
