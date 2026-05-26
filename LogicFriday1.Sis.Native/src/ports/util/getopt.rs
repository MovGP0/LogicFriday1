//! Port of `sis/util/getopt.c`.
//!
//! This is Henry Spencer's small `getopt` implementation as carried by SIS,
//! including the SIS-specific `util_` globals and reset entry point. The module
//! is intentionally self-contained: the C source only depends on standard C
//! string/stderr behavior plus declarations from `util.h`.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

const EOF: c_int = -1;

/// Global argument pointer set when an option consumes an argument.
pub static mut UTIL_OPTARG: *mut c_char = ptr::null_mut();

/// Global argv index used by `util_getopt`.
pub static mut UTIL_OPTIND: c_int = 0;

static mut SCAN: *mut c_char = ptr::null_mut();

/// Reset SIS getopt parsing state.
pub unsafe fn util_getopt_reset() {
    unsafe {
        UTIL_OPTARG = ptr::null_mut();
        UTIL_OPTIND = 0;
        SCAN = ptr::null_mut();
    }
}

/// Get the next option from `argv`.
///
/// This follows `sis/util/getopt.c`:
/// - returns the option character as an `int`
/// - returns `EOF` (`-1`) when option scanning is complete
/// - returns `'?'` for unknown options or missing required arguments
/// - sets `UTIL_OPTARG` when an option with `:` in `optstring` has an argument
pub unsafe fn util_getopt(argc: c_int, argv: *mut *mut c_char, optstring: *mut c_char) -> c_int {
    unsafe {
        UTIL_OPTARG = ptr::null_mut();

        if argv.is_null() || optstring.is_null() {
            return EOF;
        }

        if SCAN.is_null() || *SCAN == 0 {
            if UTIL_OPTIND == 0 {
                UTIL_OPTIND += 1;
            }
            if UTIL_OPTIND >= argc {
                return EOF;
            }

            let place = *argv.add(UTIL_OPTIND as usize);
            if place.is_null() || *place != b'-' as c_char || *place.add(1) == 0 {
                return EOF;
            }

            UTIL_OPTIND += 1;
            if *place.add(1) == b'-' as c_char && *place.add(2) == 0 {
                return EOF;
            }

            SCAN = place.add(1);
        }

        let option = *SCAN as u8;
        SCAN = SCAN.add(1);

        let mut place = find_option(optstring, option);
        if place.is_null() || option == b':' {
            eprintln!("{}: unknown option {}", program_name(argv), option as char);
            return b'?' as c_int;
        }

        place = place.add(1);
        if *place == b':' as c_char {
            if !SCAN.is_null() && *SCAN != 0 {
                UTIL_OPTARG = SCAN;
                SCAN = ptr::null_mut();
            } else {
                if UTIL_OPTIND >= argc {
                    eprintln!(
                        "{}: {} requires an argument",
                        program_name(argv),
                        option as char
                    );
                    return b'?' as c_int;
                }
                UTIL_OPTARG = *argv.add(UTIL_OPTIND as usize);
                UTIL_OPTIND += 1;
            }
        }

        option as c_int
    }
}

unsafe fn find_option(mut optstring: *mut c_char, option: u8) -> *mut c_char {
    unsafe {
        while !optstring.is_null() && *optstring != 0 {
            if *optstring as u8 == option {
                return optstring;
            }
            optstring = optstring.add(1);
        }
        ptr::null_mut()
    }
}

unsafe fn program_name(argv: *mut *mut c_char) -> String {
    unsafe {
        if argv.is_null() || (*argv).is_null() {
            return String::from("program");
        }
        CStr::from_ptr(*argv).to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn argv(values: &[&str]) -> (Vec<CString>, Vec<*mut c_char>) {
        let strings = values
            .iter()
            .map(|value| CString::new(*value).unwrap())
            .collect::<Vec<_>>();
        let mut pointers = strings
            .iter()
            .map(|value| value.as_ptr() as *mut c_char)
            .collect::<Vec<_>>();
        pointers.push(ptr::null_mut());
        (strings, pointers)
    }

    #[test]
    fn parses_clustered_options_and_stops_at_operand() {
        let _guard = TEST_LOCK.lock().unwrap();
        let (_strings, mut pointers) = argv(&["sis", "-ab", "file"]);
        let optstring = CString::new("ab").unwrap();

        unsafe {
            util_getopt_reset();
            assert_eq!(
                util_getopt(3, pointers.as_mut_ptr(), optstring.as_ptr() as *mut c_char),
                b'a' as c_int
            );
            assert_eq!(
                util_getopt(3, pointers.as_mut_ptr(), optstring.as_ptr() as *mut c_char),
                b'b' as c_int
            );
            assert_eq!(
                util_getopt(3, pointers.as_mut_ptr(), optstring.as_ptr() as *mut c_char),
                EOF
            );
            let optind = UTIL_OPTIND;
            assert_eq!(optind, 2);
        }
    }

    #[test]
    fn parses_attached_and_separate_option_arguments() {
        let _guard = TEST_LOCK.lock().unwrap();
        let (_strings, mut pointers) = argv(&["sis", "-ofile", "-n", "42"]);
        let optstring = CString::new("o:n:").unwrap();

        unsafe {
            util_getopt_reset();
            assert_eq!(
                util_getopt(4, pointers.as_mut_ptr(), optstring.as_ptr() as *mut c_char),
                b'o' as c_int
            );
            assert_eq!(CStr::from_ptr(UTIL_OPTARG).to_str().unwrap(), "file");
            assert_eq!(
                util_getopt(4, pointers.as_mut_ptr(), optstring.as_ptr() as *mut c_char),
                b'n' as c_int
            );
            assert_eq!(CStr::from_ptr(UTIL_OPTARG).to_str().unwrap(), "42");
            assert_eq!(
                util_getopt(4, pointers.as_mut_ptr(), optstring.as_ptr() as *mut c_char),
                EOF
            );
        }
    }

    #[test]
    fn reports_unknown_and_missing_arguments_as_question_mark() {
        let _guard = TEST_LOCK.lock().unwrap();
        let (_strings, mut unknown_pointers) = argv(&["sis", "-x"]);
        let (_strings, mut missing_pointers) = argv(&["sis", "-o"]);
        let no_x = CString::new("ab").unwrap();
        let requires_argument = CString::new("o:").unwrap();

        unsafe {
            util_getopt_reset();
            assert_eq!(
                util_getopt(
                    2,
                    unknown_pointers.as_mut_ptr(),
                    no_x.as_ptr() as *mut c_char
                ),
                b'?' as c_int
            );

            util_getopt_reset();
            assert_eq!(
                util_getopt(
                    2,
                    missing_pointers.as_mut_ptr(),
                    requires_argument.as_ptr() as *mut c_char
                ),
                b'?' as c_int
            );
        }
    }
}
