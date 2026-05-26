//! Native Rust port of `LogicSynthesis/sis/test/end_test.c`.
//!
//! The original SIS function is a program shutdown hook with an empty body.
//! Keeping that behavior as an explicit Rust API lets higher-level lifecycle
//! code call the test package teardown step without carrying a per-file legacy
//! C ABI export.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndTestDisposition {
    NoOp,
}

pub const END_TEST_DISPOSITION: EndTestDisposition = EndTestDisposition::NoOp;

pub const END_TEST_RATIONALE: &str =
    "LogicSynthesis/sis/test/end_test.c is an empty shutdown hook.";

pub fn end_test() -> EndTestDisposition {
    END_TEST_DISPOSITION
}

pub fn end_test_is_noop() -> bool {
    end_test() == EndTestDisposition::NoOp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_noop_disposition() {
        assert_eq!(end_test(), EndTestDisposition::NoOp);
        assert!(end_test_is_noop());
    }

    #[test]
    fn rationale_names_source_file() {
        assert!(END_TEST_RATIONALE.contains("end_test.c"));
        assert!(END_TEST_RATIONALE.contains("empty shutdown hook"));
    }
}
