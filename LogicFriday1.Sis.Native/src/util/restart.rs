//! Retired port of `LogicSynthesis/sis/util/restart.c`.
//!
//! The C source is wholly guarded by `#ifdef notdef`, so normal SIS builds do
//! not compile or export `util_restart`. The disabled implementation attempted
//! to checkpoint and later resume a process image by copying stack memory,
//! installing legacy Unix signal handlers, calling `sbrk`, and delegating image
//! writing to the equally obsolete `util_save_image` path.
//!
//! Recreating that behavior in the native Rust port would require reviving
//! unsupported VAX/SunOS stack-pointer manipulation and process-image rewriting.
//! The behavior is intentionally retired here with a documented, idiomatic Rust
//! API that reports restart as unavailable.

use std::path::{Path, PathBuf};

pub const RESTART_UNAVAILABLE_MESSAGE: &str =
    "util_restart: not supported on your operating system/hardware";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RestartRequest {
    old_executable: PathBuf,
    new_executable: PathBuf,
    interval_seconds: u64,
}

impl RestartRequest {
    pub fn new(
        old_executable: impl Into<PathBuf>,
        new_executable: impl Into<PathBuf>,
        interval_seconds: u64,
    ) -> Self {
        Self {
            old_executable: old_executable.into(),
            new_executable: new_executable.into(),
            interval_seconds,
        }
    }

    pub fn old_executable(&self) -> &Path {
        &self.old_executable
    }

    pub fn new_executable(&self) -> &Path {
        &self.new_executable
    }

    pub fn interval_seconds(&self) -> u64 {
        self.interval_seconds
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RestartUnavailable;

/// Configure process restart checkpointing.
///
/// Always returns `RestartUnavailable` because the only implementation in the
/// SIS source is disabled and depends on obsolete machine-specific stack image
/// manipulation.
pub fn restart(_request: &RestartRequest) -> Result<(), RestartUnavailable> {
    report_unavailable();
    Err(RestartUnavailable)
}

fn report_unavailable() {
    eprintln!("{RESTART_UNAVAILABLE_MESSAGE}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_request_preserves_inputs() {
        let request = RestartRequest::new("sis", "sis.chkpt", 3600);

        assert_eq!(request.old_executable(), Path::new("sis"));
        assert_eq!(request.new_executable(), Path::new("sis.chkpt"));
        assert_eq!(request.interval_seconds(), 3600);
    }

    #[test]
    fn restart_reports_unavailable() {
        let request = RestartRequest::new("sis", "sis.chkpt", 0);

        assert_eq!(restart(&request), Err(RestartUnavailable));
    }
}
