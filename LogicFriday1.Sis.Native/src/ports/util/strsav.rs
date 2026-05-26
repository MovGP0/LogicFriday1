//! Port of `sis/util/strsav.c`.
//!
//! The original routine duplicates a NUL-terminated C string with SIS'
//! `ALLOC` macro. This Rust port keeps the same C-string contract and returns
//! owned memory to callers through `CString::into_raw`.

use std::ffi::{CStr, CString, NulError};

pub fn strsav(source: &CStr) -> CString {
    source.to_owned()
}

pub fn strsav_bytes(source: &[u8]) -> Result<CString, NulError> {
    CString::new(source)
}

// TODO(LogicFriday1-8j8.2.6.513): expose this module through the crate module
// tree once translated Rust callers need it.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicates_c_string() {
        let source = c"abc";
        let saved = strsav(source);

        assert_eq!(saved.as_c_str(), source);
        assert_ne!(saved.as_ptr(), source.as_ptr());
    }

    #[test]
    fn rejects_interior_nul_for_bytes() {
        let err = strsav_bytes(b"a\0b").unwrap_err();

        assert_eq!(err.nul_position(), 1);
    }
}
