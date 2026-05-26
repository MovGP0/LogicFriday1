//! Port scaffold for `LogicSynthesis/sis/util/pipefork.c`.
//!
//! The C source only implements `util_pipefork` when SIS is built with the
//! legacy `UNIX` define. All other builds print a diagnostic and return
//! failure. LogicFriday currently builds the native Rust library for Windows,
//! and this bead is scoped away from `Cargo.toml`/module registration changes,
//! so the exported ABI below intentionally preserves the non-UNIX behavior.
//!
//! A full Unix port needs a crate-level libc surface for `FILE *`, `pipe`,
//! `dup2`, `fdopen`, `execvp`, and child-process status handling.

pub const PIPEFORK_UNAVAILABLE_MESSAGE: &str =
    "util_pipefork: not implemented on your operating system";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PipeforkUnavailable;

/// Fork a command with pipes connected to its standard input and output.
pub fn pipefork(_argv: &[String]) -> Result<(), PipeforkUnavailable> {
    report_not_implemented();
    Err(PipeforkUnavailable)
}

fn report_not_implemented() {
    eprintln!("{PIPEFORK_UNAVAILABLE_MESSAGE}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipefork_reports_unavailable() {
        assert_eq!(pipefork(&[]), Err(PipeforkUnavailable));
    }
}
