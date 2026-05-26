pub mod ports;

/// Returns the native SIS interop ABI version.
///
/// This is intentionally small bootstrap surface. Real SIS functions should be
/// added as the corresponding C modules are ported.
pub fn abi_version() -> i32 {
    1
}
