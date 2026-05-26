//! Retired port of `LogicSynthesis/sis/util/saveimage.c`.
//!
//! The historic executable-image writer is compiled only under the disabled
//! `#ifdef notdef` BSD `a.out` path. The active SIS source prints a diagnostic
//! and returns failure on every platform, ignoring both path arguments.

use std::path::Path;

pub const SAVE_IMAGE_UNAVAILABLE_MESSAGE: &str =
    "util_save_image: not implemented on your operating system";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SaveImageUnavailable;

/// Save an executable copy of the current process image.
///
/// This intentionally preserves the active SIS behavior: the operation is not
/// implemented and always reports failure. The disabled C implementation
/// depended on obsolete BSD `a.out` process-image internals.
pub fn save_image(
    _original_file_name: impl AsRef<Path>,
    _save_file_name: impl AsRef<Path>,
) -> Result<(), SaveImageUnavailable> {
    report_not_implemented();
    Err(SaveImageUnavailable)
}

fn report_not_implemented() {
    eprintln!("{SAVE_IMAGE_UNAVAILABLE_MESSAGE}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_image_reports_unavailable() {
        assert_eq!(save_image("sis", "sis.saved"), Err(SaveImageUnavailable));
    }

    #[test]
    fn unavailable_message_matches_c_fallback() {
        assert_eq!(
            SAVE_IMAGE_UNAVAILABLE_MESSAGE,
            "util_save_image: not implemented on your operating system"
        );
    }
}
