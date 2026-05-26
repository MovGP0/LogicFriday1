//! Port of `LogicSynthesis/sis/util/cpu_time.c`.

use std::ffi::c_void;
use std::sync::OnceLock;
use std::time::Instant;

/// Return elapsed processor time in milliseconds.
///
/// The original SIS routine reports user CPU time where the platform provides
/// it. On Windows, use `GetProcessTimes` and match that user-time behavior.
/// Other targets use a monotonic elapsed-time fallback until their process CPU
/// clock is needed by a consuming port.
pub fn cpu_time_milliseconds() -> i64 {
    platform_cpu_time_ms().unwrap_or_else(fallback_elapsed_ms)
}

#[cfg(windows)]
fn platform_cpu_time_ms() -> Option<i64> {
    let mut creation_time = FileTime::default();
    let mut exit_time = FileTime::default();
    let mut kernel_time = FileTime::default();
    let mut user_time = FileTime::default();

    let ok = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        )
    };

    if ok == 0 {
        return None;
    }

    let user_time_100ns =
        ((user_time.dw_high_date_time as u64) << 32) | (user_time.dw_low_date_time as u64);
    Some(saturating_i64(user_time_100ns / 10_000))
}

#[cfg(not(windows))]
fn platform_cpu_time_ms() -> Option<i64> {
    None
}

fn fallback_elapsed_ms() -> i64 {
    static START: OnceLock<Instant> = OnceLock::new();
    saturating_i64(START.get_or_init(Instant::now).elapsed().as_millis())
}

fn saturating_i64<T>(value: T) -> i64
where
    T: TryInto<i64>,
{
    value.try_into().unwrap_or(i64::MAX)
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct FileTime {
    dw_low_date_time: u32,
    dw_high_date_time: u32,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut c_void;

    fn GetProcessTimes(
        process: *mut c_void,
        creation_time: *mut FileTime,
        exit_time: *mut FileTime,
        kernel_time: *mut FileTime,
        user_time: *mut FileTime,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::cpu_time_milliseconds;

    #[test]
    fn cpu_time_is_non_negative_and_non_decreasing() {
        let first = cpu_time_milliseconds();
        let second = cpu_time_milliseconds();

        assert!(first >= 0);
        assert!(second >= first);
    }
}
