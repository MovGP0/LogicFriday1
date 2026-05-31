//! Source-aligned PLA input routines from Espresso `cvrin.c`.
//!
//! The original C module owns token scanning, PLA allocation, `.i/.o/.type`
//! parsing, cube reads, and optional cover completion through complement
//! operations. The native Rust parser in `pla.rs` already performs the binary
//! PLA tokenization and cover population; this module provides the
//! source-aligned entry points and read-time massaging around that type.

use std::collections::HashSet;
use std::fmt;

use crate::pla::{
    Cover, Cube, D_TYPE, DR_TYPE, F_TYPE, FD_TYPE, FR_TYPE, InputPart, Pla, PlaError, R_TYPE,
};

/// Options corresponding to `read_pla` parameters.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ReadPlaOptions {
    pub needs_dcset: bool,
    pub needs_offset: bool,
    pub pla_type: u16,
}

impl Default for ReadPlaOptions {
    fn default() -> Self {
        Self {
            needs_dcset: false,
            needs_offset: false,
            pla_type: FD_TYPE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadPlaError {
    Parse(PlaError),
    Unsupported(&'static str),
}

impl fmt::Display for ReadPlaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::Unsupported(feature) => write!(f, "unsupported PLA input feature: {feature}"),
        }
    }
}

impl std::error::Error for ReadPlaError {}

impl From<PlaError> for ReadPlaError {
    fn from(value: PlaError) -> Self {
        Self::Parse(value)
    }
}

/// `new_PLA` equivalent for the native representation.
pub fn new_pla(input_count: usize, output_count: usize) -> Pla {
    Pla::new(input_count, output_count)
}

/// `parse_pla` equivalent for in-memory PLA text.
pub fn parse_pla(input: &str) -> Result<Pla, ReadPlaError> {
    Ok(Pla::parse(input)?)
}

/// `read_pla` equivalent for in-memory PLA text.
///
/// Binary `.i/.o` PLAs are parsed through `Pla::parse`. If the source text does
/// not contain a `.type` directive, `options.pla_type` supplies the C
/// `read_pla` default. Complement completion is performed by minterm expansion
/// for the binary native representation.
pub fn read_pla(input: &str, options: ReadPlaOptions) -> Result<Option<Pla>, ReadPlaError> {
    if input.trim().is_empty() {
        return Ok(None);
    }

    let parse_input;
    let input = if has_type_directive(input) {
        input
    } else {
        parse_input = format!(".type {}\n{input}", pla_type_token(options.pla_type));
        &parse_input
    };

    let mut pla = Pla::parse(input)?;
    complete_covers(&mut pla, options);
    Ok(Some(pla))
}

/// Parse one product term into the F/D/R covers using the same output maps as
/// `read_cube`. The returned PLA contains only that row.
pub fn read_cube(
    input_pattern: &str,
    output_pattern: &str,
    input_count: usize,
    output_count: usize,
    pla_type: u16,
) -> Result<Pla, ReadPlaError> {
    let text = format!(
        ".type {}\n.i {input_count}\n.o {output_count}\n{input_pattern} {output_pattern}\n.e\n",
        pla_type_token(pla_type)
    );
    Ok(Pla::parse(&text)?)
}

/// `skip_line` equivalent for string scanners.
pub fn skip_line(input: &str, start: usize, echo: bool) -> (usize, Option<String>) {
    let tail = input.get(start..).unwrap_or_default();
    let line_len = tail.find('\n').map(|index| index + 1).unwrap_or(tail.len());
    let next = start.saturating_add(line_len);
    if echo {
        (next, Some(tail[..line_len].to_string()))
    } else {
        (next, None)
    }
}

/// `get_word` equivalent for string scanners.
pub fn get_word(input: &str, start: usize) -> Option<(&str, usize)> {
    let tail = input.get(start..)?;
    let leading = tail.len() - tail.trim_start().len();
    let tail = &tail[leading..];
    if tail.is_empty() {
        return None;
    }
    let word_len = tail.find(char::is_whitespace).unwrap_or(tail.len());
    Some((&tail[..word_len], start + leading + word_len))
}

/// `PLA_summary` equivalent returning the summary text.
pub fn pla_summary(pla: &Pla) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# PLA has {} inputs and {} outputs\n",
        pla.input_count, pla.output_count
    ));
    out.push_str(&format!(
        "# ON-set cubes  {}\n# OFF-set cubes {}\n# DC-set cubes  {}\n",
        pla.f.cubes.len(),
        pla.r.cubes.len(),
        pla.d.cubes.len()
    ));
    if let Some(phase) = &pla.phase {
        out.push_str("# phase is ");
        for bit in phase {
            out.push(if *bit { '1' } else { '0' });
        }
        out.push('\n');
    }
    out
}

/// `label_index` equivalent over compact native labels.
pub fn label_index(pla: &Pla, word: &str) -> Option<(usize, usize)> {
    if pla.input_labels.is_empty() && pla.output_labels.is_empty() {
        let index = word.parse::<usize>().ok()?;
        return Some((index, index));
    }

    if let Some(index) = pla.input_labels.iter().position(|label| label == word) {
        return Some((index, 1));
    }
    if let Some(index) = pla
        .input_labels
        .iter()
        .position(|label| word == format!("{label}.bar"))
    {
        return Some((index, 0));
    }
    pla.output_labels
        .iter()
        .position(|label| label == word)
        .map(|index| (pla.input_count, index))
}

/// Source-aligned representation of a `.symbolic` or `.symbolic-output` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolicSpec {
    pub symbolic_list: Vec<(usize, usize)>,
    pub symbolic_labels: Vec<String>,
}

/// `read_symbolic` equivalent over pre-tokenized input.
///
/// Tokens before the first semicolon are resolved with [`label_index`]. Tokens
/// after the first semicolon become symbolic labels until the next semicolon.
pub fn read_symbolic(tokens: &[&str], pla: &Pla) -> Option<SymbolicSpec> {
    let first_semicolon = tokens.iter().position(|token| *token == ";")?;
    let second_semicolon = tokens
        .iter()
        .enumerate()
        .skip(first_semicolon + 1)
        .find_map(|(index, token)| (*token == ";").then_some(index))?;

    let symbolic_list = tokens[..first_semicolon]
        .iter()
        .map(|token| label_index(pla, token))
        .collect::<Option<Vec<_>>>()?;
    let symbolic_labels = tokens[first_semicolon + 1..second_semicolon]
        .iter()
        .map(|token| (*token).to_string())
        .collect();

    Some(SymbolicSpec {
        symbolic_list,
        symbolic_labels,
    })
}

fn complete_covers(pla: &mut Pla, options: ReadPlaOptions) {
    let needs_offset = options.needs_offset || pla.phase.is_some();
    match pla.pla_type {
        F_TYPE | FD_TYPE if needs_offset => {
            pla.r = complement_of(&[&pla.f, &pla.d], pla.input_count, pla.output_count);
            pla.pla_type |= R_TYPE;
        }
        FR_TYPE if options.needs_dcset => {
            pla.d = complement_of(&[&pla.f, &pla.r], pla.input_count, pla.output_count);
            pla.pla_type |= D_TYPE;
        }
        R_TYPE | DR_TYPE => {
            pla.f = complement_of(&[&pla.d, &pla.r], pla.input_count, pla.output_count);
            pla.pla_type |= F_TYPE;
        }
        _ => {}
    }
}

fn complement_of(covers: &[&Cover], input_count: usize, output_count: usize) -> Cover {
    let covered = covers
        .iter()
        .flat_map(|cover| covered_points(cover, input_count, output_count))
        .collect::<HashSet<_>>();

    let mut by_minterm = vec![vec![false; output_count]; 1usize << input_count];
    for (minterm, outputs) in by_minterm.iter_mut().enumerate() {
        for (output, selected) in outputs.iter_mut().enumerate() {
            *selected = !covered.contains(&(minterm, output));
        }
    }

    Cover {
        cubes: by_minterm
            .into_iter()
            .enumerate()
            .filter_map(|(minterm, outputs)| {
                if outputs.iter().any(|selected| *selected) {
                    Some(Cube {
                        inputs: minterm_inputs(minterm, input_count),
                        outputs,
                    })
                } else {
                    None
                }
            })
            .collect(),
    }
}

fn covered_points(cover: &Cover, input_count: usize, output_count: usize) -> Vec<(usize, usize)> {
    cover
        .cubes
        .iter()
        .flat_map(|cube| {
            cube.covered_minterms(input_count)
                .into_iter()
                .flat_map(move |minterm| {
                    cube.outputs
                        .iter()
                        .enumerate()
                        .take(output_count)
                        .filter_map(move |(output, selected)| selected.then_some((minterm, output)))
                })
        })
        .collect()
}

fn minterm_inputs(minterm: usize, input_count: usize) -> Vec<InputPart> {
    (0..input_count)
        .map(|index| {
            let bit = 1usize << (input_count - index - 1);
            if minterm & bit == 0 {
                InputPart::Zero
            } else {
                InputPart::One
            }
        })
        .collect()
}

fn has_type_directive(input: &str) -> bool {
    input
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim_start())
        .any(|line| {
            line.starts_with(".type") && line[5..].chars().next().is_none_or(char::is_whitespace)
        })
}

fn pla_type_token(pla_type: u16) -> String {
    let mut token = String::new();
    if pla_type & F_TYPE != 0 {
        token.push('f');
    }
    if pla_type & D_TYPE != 0 {
        token.push('d');
    }
    if pla_type & R_TYPE != 0 {
        token.push('r');
    }
    if token.is_empty() {
        token.push('f');
    }
    token
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pla::{FDR_TYPE, FR_TYPE};

    #[test]
    fn read_pla_uses_supplied_type_when_directive_is_absent() {
        let pla = read_pla(
            ".i 1\n.o 1\n0 1\n1 0\n.e\n",
            ReadPlaOptions {
                pla_type: FDR_TYPE,
                ..ReadPlaOptions::default()
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(pla.f.cubes.len(), 1);
        assert_eq!(pla.r.cubes.len(), 1);
        assert_eq!(pla.d.cubes.len(), 0);
    }

    #[test]
    fn read_pla_computes_offset_when_requested_for_f_type() {
        let pla = read_pla(
            ".i 1\n.o 1\n.type f\n0 1\n.e\n",
            ReadPlaOptions {
                needs_offset: true,
                pla_type: F_TYPE,
                ..ReadPlaOptions::default()
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(pla.f.cubes.len(), 1);
        assert_eq!(pla.r.cubes.len(), 1);
        assert_eq!(pla.r.cubes[0].format("01"), "1 1");
    }

    #[test]
    fn read_pla_computes_dcset_for_fr_type() {
        let pla = read_pla(
            ".i 2\n.o 1\n.type fr\n00 1\n01 0\n.e\n",
            ReadPlaOptions {
                needs_dcset: true,
                pla_type: FR_TYPE,
                ..ReadPlaOptions::default()
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(pla.d.cubes.len(), 2);
        assert_eq!(pla.d.cubes[0].format("01"), "10 1");
        assert_eq!(pla.d.cubes[1].format("01"), "11 1");
    }

    #[test]
    fn read_cube_populates_requested_cover_sets() {
        let pla = read_cube("0-", "1-", 2, 2, FDR_TYPE).unwrap();

        assert_eq!(pla.f.cubes[0].format("~1"), "0- 1~");
        assert_eq!(pla.d.cubes[0].format("~2"), "0- ~2");
    }

    #[test]
    fn scanner_helpers_match_line_and_word_behavior() {
        assert_eq!(
            skip_line("abc\ndef", 0, true),
            (4, Some("abc\n".to_string()))
        );
        assert_eq!(get_word(" \t.type fd", 0), Some((".type", 7)));
    }

    #[test]
    fn label_lookup_supports_inputs_bars_outputs_and_numeric_fallback() {
        let pla = Pla::parse(".i 1\n.o 1\n.ilb a\n.ob y\n0 1\n.e\n").unwrap();
        assert_eq!(label_index(&pla, "a.bar"), Some((0, 0)));
        assert_eq!(label_index(&pla, "a"), Some((0, 1)));
        assert_eq!(label_index(&pla, "y"), Some((1, 0)));

        let unlabeled = Pla::parse(".i 1\n.o 1\n0 1\n.e\n").unwrap();
        assert_eq!(label_index(&unlabeled, "3"), Some((3, 3)));
    }

    #[test]
    fn read_symbolic_resolves_label_list_and_symbolic_labels() {
        let pla = Pla::parse(".i 1\n.o 1\n.ilb a\n.ob y\n0 1\n.e\n").unwrap();

        assert_eq!(
            read_symbolic(&["a", "y", ";", "low", "high", ";"], &pla),
            Some(SymbolicSpec {
                symbolic_list: vec![(0, 1), (1, 0)],
                symbolic_labels: vec!["low".to_string(), "high".to_string()],
            })
        );
    }
}
