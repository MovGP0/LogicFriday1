//! Owned Rust reader for Actel PLD fanout delay tables.
//!
//! The delay table format starts with the number of fanout delay entries,
//! followed by whitespace-separated `<fanout> <delay>` pairs. Index zero is
//! always present and represents zero fanout delay.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct ActDelayValues {
    values: Vec<f64>,
}

impl ActDelayValues {
    pub fn new(entries: impl IntoIterator<Item = (usize, f64)>) -> ActReadResult<Self> {
        let mut values = vec![0.0];
        let mut count = 0usize;

        for (fanout, delay) in entries {
            count += 1;
            validate_entry(count, fanout, delay)?;

            if fanout >= values.len() {
                values.resize(fanout + 1, 0.0);
            }
            values[fanout] = delay;
        }

        if count == 0 {
            return Err(ActReadError::InvalidEntryCount(0));
        }

        Ok(Self { values })
    }

    pub fn parse(text: &str) -> ActReadResult<Self> {
        parse_delay_values(text)
    }

    pub fn read_from_path(path: impl AsRef<Path>) -> ActReadResult<Self> {
        let text = fs::read_to_string(path).map_err(ActReadError::from)?;
        Self::parse(&text)
    }

    pub fn values(&self) -> &[f64] {
        &self.values
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn delay_for_fanout(&self, fanout: usize) -> Option<f64> {
        self.values.get(fanout).copied()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActReadError {
    Io(String),
    MissingEntryCount,
    InvalidEntryCount(i64),
    InvalidEntryCountToken(String),
    MissingFanout {
        entry_index: usize,
    },
    InvalidFanout {
        entry_index: usize,
        token: String,
    },
    NonPositiveFanout {
        entry_index: usize,
        fanout: i64,
    },
    MissingDelay {
        entry_index: usize,
        fanout: usize,
    },
    InvalidDelay {
        entry_index: usize,
        fanout: usize,
        token: String,
    },
    NegativeDelay {
        entry_index: usize,
        fanout: usize,
        delay: f64,
    },
    EntryCountMismatch {
        declared: usize,
        actual: usize,
    },
}

impl fmt::Display for ActReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(f, "failed to read act delay file: {message}"),
            Self::MissingEntryCount => write!(f, "missing act delay entry count"),
            Self::InvalidEntryCount(count) => {
                write!(
                    f,
                    "act delay entry count must be greater than zero, got {count}"
                )
            }
            Self::InvalidEntryCountToken(token) => {
                write!(f, "invalid act delay entry count token {token:?}")
            }
            Self::MissingFanout { entry_index } => {
                write!(f, "missing act delay fanout for entry {entry_index}")
            }
            Self::InvalidFanout { entry_index, token } => {
                write!(
                    f,
                    "invalid act delay fanout token {token:?} for entry {entry_index}"
                )
            }
            Self::NonPositiveFanout {
                entry_index,
                fanout,
            } => write!(
                f,
                "act delay fanout must be greater than zero for entry {entry_index}, got {fanout}"
            ),
            Self::MissingDelay {
                entry_index,
                fanout,
            } => write!(
                f,
                "missing act delay value for entry {entry_index} with fanout {fanout}"
            ),
            Self::InvalidDelay {
                entry_index,
                fanout,
                token,
            } => write!(
                f,
                "invalid act delay value token {token:?} for entry {entry_index} with fanout {fanout}"
            ),
            Self::NegativeDelay {
                entry_index,
                fanout,
                delay,
            } => write!(
                f,
                "act delay value must be non-negative for entry {entry_index} with fanout {fanout}, got {delay}"
            ),
            Self::EntryCountMismatch { declared, actual } => write!(
                f,
                "act delay entry count mismatch: declared {declared}, parsed {actual}"
            ),
        }
    }
}

impl Error for ActReadError {}

impl From<io::Error> for ActReadError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub type ActReadResult<T> = Result<T, ActReadError>;

pub fn parse_delay_values(text: &str) -> ActReadResult<ActDelayValues> {
    let mut tokens = text.split_whitespace();
    let declared = parse_entry_count(tokens.next())?;
    let mut entries = Vec::with_capacity(declared);

    for entry_index in 1..=declared {
        let fanout = parse_fanout(tokens.next(), entry_index)?;
        let delay = parse_delay(tokens.next(), entry_index, fanout)?;
        entries.push((fanout, delay));
    }

    if tokens.next().is_some() {
        return Err(ActReadError::EntryCountMismatch {
            declared,
            actual: declared + 1,
        });
    }

    ActDelayValues::new(entries)
}

pub fn read_delay_values(path: impl AsRef<Path>) -> ActReadResult<ActDelayValues> {
    ActDelayValues::read_from_path(path)
}

pub fn format_delay_values(delay_values: &ActDelayValues) -> String {
    let mut output = String::from("printing actel delay info for number of fanouts...\n");

    for (fanout, delay) in delay_values.values().iter().enumerate().skip(1) {
        output.push_str(&format!(" delay[{fanout}] = {delay:.6}\n"));
    }

    output.push('\n');
    output
}

fn parse_entry_count(token: Option<&str>) -> ActReadResult<usize> {
    let token = token.ok_or(ActReadError::MissingEntryCount)?;
    let count = token
        .parse::<i64>()
        .map_err(|_| ActReadError::InvalidEntryCountToken(token.to_owned()))?;

    if count < 1 {
        return Err(ActReadError::InvalidEntryCount(count));
    }

    Ok(count as usize)
}

fn parse_fanout(token: Option<&str>, entry_index: usize) -> ActReadResult<usize> {
    let token = token.ok_or(ActReadError::MissingFanout { entry_index })?;
    let fanout = token
        .parse::<i64>()
        .map_err(|_| ActReadError::InvalidFanout {
            entry_index,
            token: token.to_owned(),
        })?;

    if fanout < 1 {
        return Err(ActReadError::NonPositiveFanout {
            entry_index,
            fanout,
        });
    }

    Ok(fanout as usize)
}

fn parse_delay(token: Option<&str>, entry_index: usize, fanout: usize) -> ActReadResult<f64> {
    let token = token.ok_or(ActReadError::MissingDelay {
        entry_index,
        fanout,
    })?;
    let delay = token
        .parse::<f64>()
        .map_err(|_| ActReadError::InvalidDelay {
            entry_index,
            fanout,
            token: token.to_owned(),
        })?;

    validate_entry(entry_index, fanout, delay)?;
    Ok(delay)
}

fn validate_entry(entry_index: usize, fanout: usize, delay: f64) -> ActReadResult<()> {
    if fanout == 0 {
        return Err(ActReadError::NonPositiveFanout {
            entry_index,
            fanout: 0,
        });
    }

    if delay < 0.0 {
        return Err(ActReadError::NegativeDelay {
            entry_index,
            fanout,
            delay,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_delay_table_and_inserts_zero_fanout_delay() {
        let values = parse_delay_values(
            "\
4
1 2.3
2 3.0
3 3.9
4 5.4
",
        )
        .unwrap();

        assert_eq!(values.values(), &[0.0, 2.3, 3.0, 3.9, 5.4]);
        assert_eq!(values.delay_for_fanout(3), Some(3.9));
    }

    #[test]
    fn accepts_sparse_fanout_entries_as_owned_indexed_values() {
        let values = ActDelayValues::new([(1, 1.5), (3, 4.5)]).unwrap();

        assert_eq!(values.values(), &[0.0, 1.5, 0.0, 4.5]);
    }

    #[test]
    fn formats_delay_values_like_debug_printer() {
        let values = ActDelayValues::new([(1, 2.0), (2, 3.25)]).unwrap();

        assert_eq!(
            format_delay_values(&values),
            "printing actel delay info for number of fanouts...\n delay[1] = 2.000000\n delay[2] = 3.250000\n\n"
        );
    }

    #[test]
    fn rejects_missing_or_invalid_entry_count() {
        assert_eq!(parse_delay_values(""), Err(ActReadError::MissingEntryCount));
        assert_eq!(
            parse_delay_values("0"),
            Err(ActReadError::InvalidEntryCount(0))
        );
        assert_eq!(
            parse_delay_values("x"),
            Err(ActReadError::InvalidEntryCountToken("x".to_owned()))
        );
    }

    #[test]
    fn rejects_non_positive_fanout_and_negative_delay() {
        assert_eq!(
            parse_delay_values("1\n0 2.0"),
            Err(ActReadError::NonPositiveFanout {
                entry_index: 1,
                fanout: 0,
            })
        );
        assert_eq!(
            parse_delay_values("1\n1 -2.0"),
            Err(ActReadError::NegativeDelay {
                entry_index: 1,
                fanout: 1,
                delay: -2.0,
            })
        );
    }

    #[test]
    fn rejects_missing_fanout_or_delay() {
        assert_eq!(
            parse_delay_values("1"),
            Err(ActReadError::MissingFanout { entry_index: 1 })
        );
        assert_eq!(
            parse_delay_values("1\n2"),
            Err(ActReadError::MissingDelay {
                entry_index: 1,
                fanout: 2,
            })
        );
    }

    #[test]
    fn rejects_extra_entries_beyond_declared_count() {
        assert_eq!(
            parse_delay_values("1\n1 2.0\n2 3.0"),
            Err(ActReadError::EntryCountMismatch {
                declared: 1,
                actual: 2,
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_beads_metadata_are_present_in_this_port() {
        let source = include_str!("act_read.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
