//! Native Rust port scaffold for `sis/stg/enumerate.c`.
//!
//! The original file mixes two responsibilities: packed latch-state storage
//! and recursive sequential circuit enumeration over SIS `network_t`, `node_t`,
//! `graph_t`, and STG simulation globals. The packed-state logic is ported here
//! as an owned Rust type. The full recursive enumeration remains blocked until
//! the graph, network, node, and STG simulation ports are available.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

pub const STG_CHUNK_SIZE: usize = 1000;
pub const MAX_ELENGTH: usize = 36;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackedStateTable {
    bits_per_word: usize,
    latch_count: usize,
    words_per_state: usize,
    states: HashSet<Vec<u32>>,
}

impl PackedStateTable {
    pub fn new(latch_count: usize, bits_per_word: usize) -> Result<Self, EnumerateError> {
        if bits_per_word == 0 || bits_per_word > u32::BITS as usize {
            return Err(EnumerateError::InvalidBitsPerWord(bits_per_word));
        }

        let words_per_state = if latch_count == 0 {
            0
        } else {
            latch_count.div_ceil(bits_per_word)
        };

        Ok(Self {
            bits_per_word,
            latch_count,
            words_per_state,
            states: HashSet::new(),
        })
    }

    pub fn bits_per_word(&self) -> usize {
        self.bits_per_word
    }

    pub fn latch_count(&self) -> usize {
        self.latch_count
    }

    pub fn words_per_state(&self) -> usize {
        self.words_per_state
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn hash_code(&mut self, estate: &[u8]) -> Result<Vec<u32>, EnumerateError> {
        let packed = self.pack_state(estate)?;
        self.states.insert(packed.clone());
        Ok(packed)
    }

    pub fn contains_packed(&self, packed: &[u32]) -> bool {
        self.states.contains(packed)
    }

    pub fn pack_state(&self, estate: &[u8]) -> Result<Vec<u32>, EnumerateError> {
        if estate.len() != self.latch_count {
            return Err(EnumerateError::StateLength {
                expected: self.latch_count,
                actual: estate.len(),
            });
        }
        if let Some((index, value)) = estate
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| *value > 1)
        {
            return Err(EnumerateError::InvalidStateBit { index, value });
        }

        let mut hashed = vec![0u32; self.words_per_state];
        let mut next_width = self.latch_count % self.bits_per_word;
        if next_width == 0 {
            next_width = self.bits_per_word;
        }

        let mut k = self.latch_count;
        for i in (0..self.words_per_state).rev() {
            let mut state = 0u32;
            for _ in 0..next_width {
                k -= 1;
                state = (state << 1) + estate[k] as u32;
            }
            hashed[i] = state;
            next_width = self.bits_per_word;
        }

        Ok(hashed)
    }

    pub fn translate_hashed_code(&self, h_state: &[u32]) -> Result<Vec<u8>, EnumerateError> {
        if h_state.len() != self.words_per_state {
            return Err(EnumerateError::PackedLength {
                expected: self.words_per_state,
                actual: h_state.len(),
            });
        }

        let mut stg_state = Vec::with_capacity(self.latch_count);
        for compact in h_state.iter().copied() {
            let mut compact_state = compact;
            for _ in 0..self.bits_per_word {
                if stg_state.len() == self.latch_count {
                    return Ok(stg_state);
                }
                stg_state.push((compact_state & 1) as u8);
                compact_state >>= 1;
            }
        }

        Ok(stg_state)
    }

    pub fn state_hash(&self, packed: &[u32], modulus: u32) -> Result<u32, EnumerateError> {
        if modulus == 0 {
            return Err(EnumerateError::ZeroModulus);
        }
        if packed.len() != self.words_per_state {
            return Err(EnumerateError::PackedLength {
                expected: self.words_per_state,
                actual: packed.len(),
            });
        }

        Ok(packed.first().copied().unwrap_or_default() % modulus)
    }

    pub fn compare_states(&self, left: &[u32], right: &[u32]) -> Result<i32, EnumerateError> {
        if left.len() != self.words_per_state {
            return Err(EnumerateError::PackedLength {
                expected: self.words_per_state,
                actual: left.len(),
            });
        }
        if right.len() != self.words_per_state {
            return Err(EnumerateError::PackedLength {
                expected: self.words_per_state,
                actual: right.len(),
            });
        }

        for i in (0..self.words_per_state).rev() {
            if left[i] > right[i] {
                return Ok(1);
            }
            if left[i] < right[i] {
                return Ok(-1);
            }
        }
        Ok(0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumerationRequest {
    pub latch_count: usize,
    pub primary_input_count: usize,
    pub primary_output_count: usize,
    pub max_depth: usize,
}

pub fn enumerate_sequential_circuit(request: &EnumerationRequest) -> Result<(), EnumerateError> {
    if request.max_depth > MAX_ELENGTH {
        return Err(EnumerateError::DepthLimit {
            requested: request.max_depth,
            max: MAX_ELENGTH,
        });
    }

    Err(EnumerateError::MissingDependency(
        "recursive STG enumeration requires graph.c, node/*.c, network/*.c, stg/stg.c, stg/stg_sc_sim.c, stg/level_c.c, and stg/senum_main.c ports",
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnumerateError {
    InvalidBitsPerWord(usize),
    StateLength { expected: usize, actual: usize },
    PackedLength { expected: usize, actual: usize },
    InvalidStateBit { index: usize, value: u8 },
    ZeroModulus,
    DepthLimit { requested: usize, max: usize },
    MissingDependency(&'static str),
}

impl fmt::Display for EnumerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBitsPerWord(value) => {
                write!(f, "invalid bits-per-word value {value}; expected 1..=32")
            }
            Self::StateLength { expected, actual } => {
                write!(
                    f,
                    "state length {actual} does not match latch count {expected}"
                )
            }
            Self::PackedLength { expected, actual } => {
                write!(
                    f,
                    "packed state length {actual} does not match word count {expected}"
                )
            }
            Self::InvalidStateBit { index, value } => {
                write!(f, "state bit at index {index} has invalid value {value}")
            }
            Self::ZeroModulus => write!(f, "hash modulus must be non-zero"),
            Self::DepthLimit { requested, max } => {
                write!(
                    f,
                    "requested enumeration depth {requested} exceeds MAX_ELENGTH {max}"
                )
            }
            Self::MissingDependency(message) => write!(f, "{message}"),
        }
    }
}

impl Error for EnumerateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_state_matches_c_shashcode_word_order() {
        let table = PackedStateTable::new(5, 4).unwrap();

        assert_eq!(table.pack_state(&[1, 0, 1, 1, 0]).unwrap(), vec![0b1101, 0]);
        assert_eq!(
            table.translate_hashed_code(&[0b1101, 0]).unwrap(),
            vec![1, 0, 1, 1, 0]
        );
    }

    #[test]
    fn pack_state_handles_partial_most_significant_word_first() {
        let table = PackedStateTable::new(10, 4).unwrap();

        let packed = table.pack_state(&[1, 0, 1, 1, 0, 0, 1, 0, 1, 1]).unwrap();

        assert_eq!(packed, vec![0b1101, 0b0100, 0b11]);
        assert_eq!(
            table.translate_hashed_code(&packed).unwrap(),
            vec![1, 0, 1, 1, 0, 0, 1, 0, 1, 1]
        );
    }

    #[test]
    fn hash_code_interns_packed_states_like_st_storelist() {
        let mut table = PackedStateTable::new(4, 4).unwrap();

        let first = table.hash_code(&[1, 0, 0, 1]).unwrap();
        let second = table.hash_code(&[1, 0, 0, 1]).unwrap();
        let third = table.hash_code(&[0, 0, 0, 1]).unwrap();

        assert_eq!(first, second);
        assert_ne!(first, third);
        assert_eq!(table.len(), 2);
        assert!(table.contains_packed(&first));
    }

    #[test]
    fn compare_and_hash_match_c_helpers() {
        let table = PackedStateTable::new(8, 4).unwrap();

        assert_eq!(table.compare_states(&[3, 1], &[3, 2]).unwrap(), -1);
        assert_eq!(table.compare_states(&[9, 2], &[3, 2]).unwrap(), 1);
        assert_eq!(table.compare_states(&[3, 2], &[3, 2]).unwrap(), 0);
        assert_eq!(table.state_hash(&[17, 2], 5).unwrap(), 2);
    }

    #[test]
    fn recursive_enumeration_reports_unported_dependencies() {
        let request = EnumerationRequest {
            latch_count: 2,
            primary_input_count: 1,
            primary_output_count: 1,
            max_depth: MAX_ELENGTH,
        };

        assert_eq!(
            enumerate_sequential_circuit(&request),
            Err(EnumerateError::MissingDependency(
                "recursive STG enumeration requires graph.c, node/*.c, network/*.c, stg/stg.c, stg/stg_sc_sim.c, stg/level_c.c, and stg/senum_main.c ports",
            ))
        );
    }
}
