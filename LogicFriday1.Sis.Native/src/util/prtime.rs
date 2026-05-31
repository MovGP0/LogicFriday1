//! Port of `LogicSynthesis/sis/util/prtime.c`.

pub fn print_time(t: i64) -> String {
    format!("{}.{:02} sec", t / 1000, (t % 1000) / 10)
}

// TODO: expose this module through the crate module tree once native Rust
// callers need it.

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats_milliseconds_as_seconds_with_two_decimal_places() {
        assert_eq!(print_time(0), "0.00 sec");
        assert_eq!(print_time(1_234), "1.23 sec");
        assert_eq!(print_time(1_999), "1.99 sec");
    }

    #[test]
    fn matches_c_integer_behavior_for_negative_values() {
        assert_eq!(print_time(-10), "0.-1 sec");
        assert_eq!(print_time(-1_234), "-1.-23 sec");
    }
}
