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
