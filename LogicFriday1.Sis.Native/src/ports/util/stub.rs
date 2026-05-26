// Port of LogicSynthesis/sis/util/stub.c.
//
// The original C file mostly provided libc compatibility fallbacks behind
// config.h feature gates: memcpy, memset, strchr, strrchr, popen, and pclose.
// Those are supplied by the platform C runtime for the Rust native library, so
// this port intentionally exposes only the SIS-owned symbol from the source.

pub fn do_nothing() -> i32 {
    1
}
