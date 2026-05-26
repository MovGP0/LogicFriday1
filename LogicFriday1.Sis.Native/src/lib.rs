mod facade;

pub mod ports;

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
