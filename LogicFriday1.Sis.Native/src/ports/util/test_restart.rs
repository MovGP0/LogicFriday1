//! Retired port of `LogicSynthesis/sis/util/test-restart.c`.
//!
//! The original source is completely guarded by `#ifdef notdef`, so it is not
//! part of normal SIS builds. The disabled code was an ad-hoc executable test
//! harness for the obsolete `util_restart` mechanism: it found the current
//! program, attempted a restart, printed argv/environ addresses, and returned a
//! fixed status.
//!
//! Native Rust intentionally does not recreate that standalone process-restart
//! test driver. The restart support it exercised is itself platform-specific
//! and retired in the native port. This module keeps the port decision
//! explicit for future callers without adding legacy C ABI exports.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestartTestDisposition {
    Retired,
}

pub const RESTART_TEST_DISPOSITION: RestartTestDisposition = RestartTestDisposition::Retired;

pub const RETIREMENT_RATIONALE: &str = "LogicSynthesis/sis/util/test-restart.c is fully disabled by #ifdef notdef and only contained an obsolete standalone util_restart test driver.";

pub fn restart_test_disposition() -> RestartTestDisposition {
    RESTART_TEST_DISPOSITION
}

pub fn restart_test_is_retired() -> bool {
    restart_test_disposition() == RestartTestDisposition::Retired
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_retired_disposition() {
        assert_eq!(restart_test_disposition(), RestartTestDisposition::Retired);
        assert!(restart_test_is_retired());
    }

    #[test]
    fn rationale_names_disabled_c_source() {
        assert!(RETIREMENT_RATIONALE.contains("test-restart.c"));
        assert!(RETIREMENT_RATIONALE.contains("#ifdef notdef"));
    }
}
