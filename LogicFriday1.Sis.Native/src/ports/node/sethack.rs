//! Native Rust ordering helper for packed SIS set words.
//!
//! The legacy node package used this comparator to sort sum-of-products cubes
//! for printing. Each set is represented as a header word followed by packed
//! 32-bit payload words; the low ten bits of the header encode the number of
//! payload words to compare.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

const LOOP_MASK: u32 = 0x03ff;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SetHackError {
    MissingHeader,
    MissingPayload {
        loop_count: usize,
        actual_words: usize,
    },
    MismatchedLoopCount {
        left: usize,
        right: usize,
    },
}

impl fmt::Display for SetHackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => write!(f, "set has no header word"),
            Self::MissingPayload {
                loop_count,
                actual_words,
            } => write!(
                f,
                "set loop count {loop_count} requires {} words, got {actual_words}",
                loop_count + 1
            ),
            Self::MismatchedLoopCount { left, right } => {
                write!(f, "set loop counts differ: {left} and {right}")
            }
        }
    }
}

impl Error for SetHackError {}

pub type SetHackResult<T> = Result<T, SetHackError>;

pub fn payload_word_count(set: &[u32]) -> SetHackResult<usize> {
    let header = set.first().ok_or(SetHackError::MissingHeader)?;
    let count = (header & LOOP_MASK) as usize;
    let required_words = count + 1;
    if set.len() < required_words {
        return Err(SetHackError::MissingPayload {
            loop_count: count,
            actual_words: set.len(),
        });
    }

    Ok(count)
}

pub fn reverse_word_bits(value: u32) -> u32 {
    value.reverse_bits()
}

pub fn fancy_lex_order(left: &[u32], right: &[u32]) -> SetHackResult<Ordering> {
    let left_count = payload_word_count(left)?;
    let right_count = payload_word_count(right)?;
    if left_count != right_count {
        return Err(SetHackError::MismatchedLoopCount {
            left: left_count,
            right: right_count,
        });
    }

    for index in 1..=left_count {
        let left_key = reverse_word_bits(!left[index]);
        let right_key = reverse_word_bits(!right[index]);
        match left_key.cmp(&right_key) {
            Ordering::Greater => return Ok(Ordering::Less),
            Ordering::Less => return Ok(Ordering::Greater),
            Ordering::Equal => {}
        }
    }

    Ok(Ordering::Equal)
}

pub fn fancy_lex_compare(left: &[u32], right: &[u32]) -> SetHackResult<i32> {
    Ok(match fancy_lex_order(left, right)? {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(words: &[u32]) -> Vec<u32> {
        let mut result = Vec::with_capacity(words.len() + 1);
        result.push(words.len() as u32);
        result.extend_from_slice(words);
        result
    }

    #[test]
    fn reverses_word_bits() {
        assert_eq!(reverse_word_bits(0x0000_0001), 0x8000_0000);
        assert_eq!(reverse_word_bits(0x0123_4567), 0xe6a2_c480);
        assert_eq!(reverse_word_bits(0xffff_fff0), 0x0fff_ffff);
    }

    #[test]
    fn orders_by_reversed_complemented_payload_words() {
        let all_but_high_bit = set(&[0x7fff_ffff]);
        let all_but_low_bit = set(&[0xffff_fffe]);

        assert_eq!(
            fancy_lex_order(&all_but_high_bit, &all_but_low_bit),
            Ok(Ordering::Greater)
        );
        assert_eq!(
            fancy_lex_compare(&all_but_high_bit, &all_but_low_bit),
            Ok(1)
        );
        assert_eq!(
            fancy_lex_compare(&all_but_low_bit, &all_but_high_bit),
            Ok(-1)
        );
    }

    #[test]
    fn advances_to_later_words_after_equal_prefix() {
        let left = set(&[0xffff_ffff, 0x7fff_ffff]);
        let right = set(&[0xffff_ffff, 0xffff_fffe]);

        assert_eq!(fancy_lex_order(&left, &right), Ok(Ordering::Greater));
    }

    #[test]
    fn equal_payload_words_compare_equal() {
        let left = set(&[0xaaaa_5555, 0x1234_5678]);
        let right = set(&[0xaaaa_5555, 0x1234_5678]);

        assert_eq!(fancy_lex_order(&left, &right), Ok(Ordering::Equal));
        assert_eq!(fancy_lex_compare(&left, &right), Ok(0));
    }

    #[test]
    fn validates_set_shapes() {
        assert_eq!(payload_word_count(&[]), Err(SetHackError::MissingHeader));
        assert_eq!(
            payload_word_count(&[2, 0]),
            Err(SetHackError::MissingPayload {
                loop_count: 2,
                actual_words: 2
            })
        );
        assert_eq!(
            fancy_lex_order(&set(&[0]), &set(&[0, 0])),
            Err(SetHackError::MismatchedLoopCount { left: 1, right: 2 })
        );
    }

    #[test]
    fn header_uses_low_ten_bits_for_loop_count() {
        let with_flags = [0xfc00 | 1, 0x7fff_ffff];
        let plain = [1, 0xffff_fffe];

        assert_eq!(payload_word_count(&with_flags), Ok(1));
        assert_eq!(fancy_lex_order(&with_flags, &plain), Ok(Ordering::Greater));
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let text = include_str!("sethack.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!text.contains(concat!("REQUIRED", "_")));
        assert!(!text.contains(concat!("Port", "Dependency")));
        assert!(!text.contains(concat!("bead", "_", "id")));
        assert!(!text.contains(concat!("source", "_", "file")));
        assert!(!text.contains(concat!("Logic", "Friday1", "-")));
    }
}
