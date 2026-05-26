//! Retired port of `LogicSynthesis/sis/util/state.c`.
//!
//! The C source is wholly guarded by `#ifdef notdef`, so normal SIS builds do
//! not compile or export any of its symbols. The code that remains inside that
//! disabled block is machine-specific stack/register checkpointing for VAX and
//! early Sun 68k restart support. Recreating that behavior would require
//! unsupported inline assembly and the equally disabled `util/restart.c` path.
//!
//! This module intentionally preserves only the portable lint fallback shape:
//! saving restart state reports the initial path (`0`) and restoring state is a
//! no-op. It gives future `restart.c` work a documented scaffold without
//! reviving obsolete platform-specific control flow.

use std::ffi::c_int;

pub const UTIL_RESTART_SAVE_INITIAL: c_int = 0;

/// Save process restart state.
///
/// Returns `0`, matching the only portable fallback in the disabled C source.
pub fn restart_save_state() -> c_int {
    UTIL_RESTART_SAVE_INITIAL
}

/// Restore process restart state.
///
/// This is intentionally a no-op because the active SIS source does not compile
/// the architecture-specific state restoration routines.
pub fn restart_restore_state() {}

// TODO: keep this retired scaffold native Rust only.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_state_reports_initial_execution_path() {
        assert_eq!(restart_save_state(), UTIL_RESTART_SAVE_INITIAL);
    }

    #[test]
    fn restore_state_is_a_noop() {
        restart_restore_state();
    }
}
