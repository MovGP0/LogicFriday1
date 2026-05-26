//! Port of `sis/util/cpu_stats.c`.
//!
//! The original SIS source has its detailed BSD `getrusage` implementation
//! compiled out with `&& 0`, so the active behavior is the fallback message.
//! This module preserves that behavior without depending on unported SIS code.

use std::io::{self, Write};

pub const CPU_STATS_UNAVAILABLE_MESSAGE: &str = "Usage statistics not available\n";

pub fn cpu_stats_report() -> &'static str {
    CPU_STATS_UNAVAILABLE_MESSAGE
}

pub fn write_cpu_stats(mut writer: impl Write) -> io::Result<()> {
    writer.write_all(cpu_stats_report().as_bytes())
}

// TODO(LogicFriday1-8j8.2.6.503): expose this module through the crate module
// tree and csbindgen surface once this worker is allowed to edit `src/lib.rs`.
