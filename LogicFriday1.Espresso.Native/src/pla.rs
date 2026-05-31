use std::collections::HashSet;
use std::fmt;

pub use crate::getopt::{GetOpt, GetOptItem};
pub use crate::verify::{ConsistencyReport, VerifyReport, verify_covers};

pub const F_TYPE: u16 = 1;
pub const D_TYPE: u16 = 2;
pub const R_TYPE: u16 = 4;
pub const FD_TYPE: u16 = F_TYPE | D_TYPE;
pub const FR_TYPE: u16 = F_TYPE | R_TYPE;
pub const DR_TYPE: u16 = D_TYPE | R_TYPE;
pub const FDR_TYPE: u16 = F_TYPE | D_TYPE | R_TYPE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pla {
    pub input_count: usize,
    pub output_count: usize,
    pub pla_type: u16,
    pub input_labels: Vec<String>,
    pub output_labels: Vec<String>,
    pub phase: Option<Vec<bool>>,
    pub f: Cover,
    pub d: Cover,
    pub r: Cover,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cover {
    pub cubes: Vec<Cube>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Cube {
    pub inputs: Vec<InputPart>,
    pub outputs: Vec<bool>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InputPart {
    Zero,
    One,
    Dash,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaError {
    MissingInputCount,
    MissingOutputCount,
    InvalidDirective { line: usize, directive: String },
    InvalidNumber { line: usize, token: String },
    InvalidPlaType { line: usize, token: String },
    InvalidCube { line: usize, message: String },
    Unsupported { line: usize, directive: String },
}

impl fmt::Display for PlaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInputCount => write!(f, "PLA is missing .i input count"),
            Self::MissingOutputCount => write!(f, "PLA is missing .o output count"),
            Self::InvalidDirective { line, directive } => {
                write!(f, "invalid PLA directive on line {line}: {directive}")
            }
            Self::InvalidNumber { line, token } => {
                write!(f, "invalid number on line {line}: {token}")
            }
            Self::InvalidPlaType { line, token } => {
                write!(f, "invalid PLA type on line {line}: {token}")
            }
            Self::InvalidCube { line, message } => {
                write!(f, "invalid cube on line {line}: {message}")
            }
            Self::Unsupported { line, directive } => {
                write!(f, "unsupported PLA directive on line {line}: {directive}")
            }
        }
    }
}

impl std::error::Error for PlaError {}

impl Pla {
    pub fn new(input_count: usize, output_count: usize) -> Self {
        Self {
            input_count,
            output_count,
            pla_type: F_TYPE,
            input_labels: Vec::new(),
            output_labels: Vec::new(),
            phase: None,
            f: Cover::default(),
            d: Cover::default(),
            r: Cover::default(),
        }
    }

    pub fn parse(input: &str) -> Result<Self, PlaError> {
        let mut input_count = None;
        let mut output_count = None;
        let mut pla_type = FD_TYPE;
        let mut input_labels = Vec::new();
        let mut output_labels = Vec::new();
        let mut phase = None;
        let mut f = Cover::default();
        let mut d = Cover::default();
        let mut r = Cover::default();
        let mut pending_cube: Option<(usize, String)> = None;

        for (line_index, raw_line) in input.lines().enumerate() {
            let line_no = line_index + 1;
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let first = parts.next().unwrap();
            if first.starts_with('.') {
                match first {
                    ".i" => input_count = Some(parse_usize(parts.next(), line_no)?),
                    ".o" => output_count = Some(parse_usize(parts.next(), line_no)?),
                    ".p" => {}
                    ".e" | ".end" => break,
                    ".type" => {
                        let token = parts.next().ok_or_else(|| PlaError::InvalidDirective {
                            line: line_no,
                            directive: first.to_string(),
                        })?;
                        pla_type =
                            parse_pla_type(token).ok_or_else(|| PlaError::InvalidPlaType {
                                line: line_no,
                                token: token.to_string(),
                            })?;
                    }
                    ".ilb" => input_labels = parts.map(str::to_string).collect(),
                    ".ob" => output_labels = parts.map(str::to_string).collect(),
                    ".phase" => {
                        let token = parts.next().ok_or_else(|| PlaError::InvalidDirective {
                            line: line_no,
                            directive: first.to_string(),
                        })?;
                        phase = Some(parse_phase(token, line_no)?);
                    }
                    ".mv" | ".label" | ".symbolic" | ".symbolic-output" | ".pair" | ".kiss" => {
                        return Err(PlaError::Unsupported {
                            line: line_no,
                            directive: first.to_string(),
                        });
                    }
                    _ => {
                        return Err(PlaError::InvalidDirective {
                            line: line_no,
                            directive: first.to_string(),
                        });
                    }
                }
                continue;
            }

            let ni = input_count.ok_or(PlaError::MissingInputCount)?;
            let no = output_count.ok_or(PlaError::MissingOutputCount)?;
            let mut cube_tokens = Vec::with_capacity(2);
            cube_tokens.push(first);
            cube_tokens.extend(parts);
            let compact = compact_cube_tokens(&cube_tokens);
            let (cube_line, cube_text) = if let Some((cube_line, mut pending)) = pending_cube.take()
            {
                pending.push_str(&compact);
                (cube_line, pending)
            } else {
                (line_no, compact)
            };

            if cube_text.len() < ni + no {
                pending_cube = Some((cube_line, cube_text));
                continue;
            }

            let (input_token, output_token) = if cube_text.len() == ni + no {
                (
                    cube_text[..ni].to_string(),
                    cube_text[ni..ni + no].to_string(),
                )
            } else {
                split_cube_tokens(&cube_tokens, ni, no, line_no)?
            };
            let input_parts = parse_input_pattern(&input_token, ni, cube_line)?;
            let output_chars: Vec<char> = output_token.chars().collect();

            add_cube_by_output(
                &mut f,
                &input_parts,
                &output_chars,
                no,
                pla_type,
                F_TYPE,
                &['1', '4'],
            );
            add_cube_by_output(
                &mut d,
                &input_parts,
                &output_chars,
                no,
                pla_type,
                D_TYPE,
                &['2', '-'],
            );
            add_cube_by_output(
                &mut r,
                &input_parts,
                &output_chars,
                no,
                pla_type,
                R_TYPE,
                &['0', '3'],
            );
        }

        if let Some((line, _)) = pending_cube {
            return Err(PlaError::InvalidCube {
                line,
                message: "cube is missing wrapped input/output parts".to_string(),
            });
        }

        let input_count = input_count.ok_or(PlaError::MissingInputCount)?;
        let output_count = output_count.ok_or(PlaError::MissingOutputCount)?;

        Ok(Self {
            input_count,
            output_count,
            pla_type,
            input_labels,
            output_labels,
            phase,
            f,
            d,
            r,
        })
    }

    pub fn write(&self, output_type: u16) -> String {
        let mut out = String::new();
        self.write_header(output_type, &mut out);

        let mut rows = 0;
        if output_type & F_TYPE != 0 {
            rows += self.f.cubes.len();
        }
        if output_type & D_TYPE != 0 {
            rows += self.d.cubes.len();
        }
        if output_type & R_TYPE != 0 {
            rows += self.r.cubes.len();
        }
        out.push_str(&format!(".p {rows}\n"));

        if output_type == F_TYPE {
            for cube in &self.f.cubes {
                out.push_str(&format!("{}\n", cube.format("01")));
            }
            out.push_str(".e\n");
            return out;
        }

        if output_type & F_TYPE != 0 {
            for cube in &self.f.cubes {
                out.push_str(&format!("{}\n", cube.format("~1")));
            }
        }
        if output_type & D_TYPE != 0 {
            for cube in &self.d.cubes {
                out.push_str(&format!("{}\n", cube.format("~2")));
            }
        }
        if output_type & R_TYPE != 0 {
            for cube in &self.r.cubes {
                out.push_str(&format!("{}\n", cube.format("~0")));
            }
        }
        out.push_str(".end\n");
        out
    }

    pub fn write_header(&self, output_type: u16, out: &mut String) {
        if output_type != F_TYPE {
            out.push_str(".type ");
            if output_type & F_TYPE != 0 {
                out.push('f');
            }
            if output_type & D_TYPE != 0 {
                out.push('d');
            }
            if output_type & R_TYPE != 0 {
                out.push('r');
            }
            out.push('\n');
        }

        out.push_str(&format!(
            ".i {}\n.o {}\n",
            self.input_count, self.output_count
        ));

        if !self.input_labels.is_empty() {
            out.push_str(".ilb");
            for label in &self.input_labels {
                out.push(' ');
                out.push_str(label);
            }
            out.push('\n');
        }
        if !self.output_labels.is_empty() {
            out.push_str(".ob");
            for label in &self.output_labels {
                out.push(' ');
                out.push_str(label);
            }
            out.push('\n');
        }
        if let Some(phase) = &self.phase {
            out.push_str("#.phase ");
            for bit in phase {
                out.push(if *bit { '1' } else { '0' });
            }
            out.push('\n');
        }
    }
}

impl Cover {
    pub fn cost(&self, input_count: usize, output_count: usize) -> CoverCost {
        let mut cost = CoverCost {
            cubes: self.cubes.len(),
            ..CoverCost::default()
        };

        for cube in &self.cubes {
            cost.inputs += cube
                .inputs
                .iter()
                .filter(|part| matches!(part, InputPart::Zero | InputPart::One))
                .count();
            cost.outputs += cube.outputs.iter().filter(|selected| **selected).count();
        }
        cost.total = cost.inputs + cost.outputs;
        cost.unused_inputs = input_count.saturating_sub(
            self.cubes
                .iter()
                .flat_map(|cube| cube.inputs.iter().enumerate())
                .filter(|(_, part)| matches!(part, InputPart::Zero | InputPart::One))
                .map(|(index, _)| index)
                .collect::<HashSet<_>>()
                .len(),
        );
        cost.unused_outputs = output_count.saturating_sub(
            self.cubes
                .iter()
                .flat_map(|cube| cube.outputs.iter().enumerate())
                .filter(|(_, selected)| **selected)
                .map(|(index, _)| index)
                .collect::<HashSet<_>>()
                .len(),
        );
        cost
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct CoverCost {
    pub cubes: usize,
    pub inputs: usize,
    pub outputs: usize,
    pub total: usize,
    pub unused_inputs: usize,
    pub unused_outputs: usize,
}

impl fmt::Display for CoverCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "c={} in={} out={} tot={}",
            self.cubes, self.inputs, self.outputs, self.total
        )
    }
}

impl Cube {
    pub fn format(&self, out_map: &str) -> String {
        let out_map: Vec<char> = out_map.chars().collect();
        let mut out = String::with_capacity(self.inputs.len() + self.outputs.len() + 1);
        for input in &self.inputs {
            out.push(match input {
                InputPart::Empty => '?',
                InputPart::Zero => '0',
                InputPart::One => '1',
                InputPart::Dash => '-',
            });
        }
        out.push(' ');
        for selected in &self.outputs {
            out.push(out_map[usize::from(*selected)]);
        }
        out
    }

    pub(crate) fn covered_minterms(&self, input_count: usize) -> Vec<usize> {
        let mut minterms = vec![0usize];
        for (index, part) in self.inputs.iter().enumerate().take(input_count) {
            let bit = 1usize << (input_count - index - 1);
            match part {
                InputPart::Zero | InputPart::Empty => {}
                InputPart::One => {
                    for minterm in &mut minterms {
                        *minterm |= bit;
                    }
                }
                InputPart::Dash => {
                    let mut ones = minterms.clone();
                    for minterm in &mut ones {
                        *minterm |= bit;
                    }
                    minterms.extend(ones);
                }
            }
        }
        minterms
    }
}

fn parse_usize(token: Option<&str>, line: usize) -> Result<usize, PlaError> {
    let token = token.ok_or_else(|| PlaError::InvalidNumber {
        line,
        token: String::new(),
    })?;
    token.parse().map_err(|_| PlaError::InvalidNumber {
        line,
        token: token.to_string(),
    })
}

fn parse_pla_type(token: &str) -> Option<u16> {
    let mut value = 0;
    for ch in token.chars() {
        value |= match ch {
            'f' | 'F' => F_TYPE,
            'd' | 'D' => D_TYPE,
            'r' | 'R' => R_TYPE,
            _ => return None,
        };
    }
    Some(value)
}

fn parse_phase(token: &str, line: usize) -> Result<Vec<bool>, PlaError> {
    token
        .chars()
        .map(|ch| match ch {
            '0' => Ok(false),
            '1' => Ok(true),
            _ => Err(PlaError::InvalidCube {
                line,
                message: "only 0 or 1 allowed in phase description".to_string(),
            }),
        })
        .collect()
}

fn split_cube_tokens(
    tokens: &[&str],
    input_count: usize,
    output_count: usize,
    line: usize,
) -> Result<(String, String), PlaError> {
    if tokens.len() >= 2 {
        let input = tokens[0]
            .chars()
            .filter(|ch| *ch != '|')
            .collect::<String>();
        let output = tokens[1..]
            .iter()
            .flat_map(|token| token.chars())
            .filter(|ch| *ch != '|')
            .collect::<String>();
        if input.len() == input_count && output.len() == output_count {
            return Ok((input, output));
        }
    }

    let compact = compact_cube_tokens(tokens);
    if compact.len() == input_count + output_count {
        return Ok((
            compact[..input_count].to_string(),
            compact[input_count..].to_string(),
        ));
    }

    let input = tokens
        .iter()
        .take_while(|token| {
            token
                .chars()
                .all(|ch| matches!(ch, '0' | '1' | '2' | '-' | '?' | '|'))
        })
        .flat_map(|token| token.chars())
        .filter(|ch| *ch != '|')
        .collect::<String>();
    let consumed_input_tokens = tokens
        .iter()
        .take_while(|token| {
            token
                .chars()
                .all(|ch| matches!(ch, '0' | '1' | '2' | '-' | '?' | '|'))
        })
        .count();
    let output = tokens[consumed_input_tokens..]
        .iter()
        .flat_map(|token| token.chars())
        .filter(|ch| *ch != '|')
        .collect::<String>();

    if input.len() != input_count || output.len() != output_count {
        return Err(PlaError::InvalidCube {
            line,
            message: format!(
                "expected {input_count} input and {output_count} output characters, got {} and {}",
                input.len(),
                output.len()
            ),
        });
    }
    Ok((input, output))
}

fn compact_cube_tokens(tokens: &[&str]) -> String {
    tokens
        .iter()
        .flat_map(|token| token.chars())
        .filter(|ch| *ch != '|')
        .collect()
}

fn parse_input_pattern(
    token: &str,
    input_count: usize,
    line: usize,
) -> Result<Vec<InputPart>, PlaError> {
    if token.len() != input_count {
        return Err(PlaError::InvalidCube {
            line,
            message: format!("expected {input_count} inputs, got {}", token.len()),
        });
    }
    token
        .chars()
        .map(|ch| match ch {
            '0' => Ok(InputPart::Zero),
            '1' => Ok(InputPart::One),
            '2' | '-' => Ok(InputPart::Dash),
            '?' => Ok(InputPart::Empty),
            _ => Err(PlaError::InvalidCube {
                line,
                message: format!("bad input character {ch}"),
            }),
        })
        .collect()
}

fn add_cube_by_output(
    cover: &mut Cover,
    input_parts: &[InputPart],
    output_chars: &[char],
    output_count: usize,
    pla_type: u16,
    required_type: u16,
    on_chars: &[char],
) {
    if pla_type & required_type == 0 {
        return;
    }
    let mut outputs = vec![false; output_count];
    let mut save = false;
    for (index, ch) in output_chars.iter().enumerate().take(output_count) {
        if on_chars.contains(ch) {
            outputs[index] = true;
            save = true;
        }
    }
    if save {
        cover.cubes.push(Cube {
            inputs: input_parts.to_vec(),
            outputs,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_binary_pla_into_on_dc_off_covers() {
        let pla = Pla::parse(
            r#"
.i 2
.o 2
.type fdr
.ilb a b
.ob y z
00 10
01 2-
1- 01
.e
"#,
        )
        .unwrap();

        assert_eq!(pla.input_count, 2);
        assert_eq!(pla.output_count, 2);
        assert_eq!(pla.input_labels, ["a", "b"]);
        assert_eq!(pla.output_labels, ["y", "z"]);
        assert_eq!(pla.f.cubes.len(), 2);
        assert_eq!(pla.d.cubes.len(), 1);
        assert_eq!(pla.r.cubes.len(), 2);
        assert_eq!(pla.f.cubes[0].format("~1"), "00 1~");
    }

    #[test]
    fn writes_pla_header_and_rows_like_espresso_fdr_output() {
        let pla = Pla::parse(
            r#"
.i 2
.o 1
.type fd
.ilb a b
.ob y
0- 1
10 -
.end
"#,
        )
        .unwrap();

        assert_eq!(
            pla.write(FD_TYPE),
            ".type fd\n.i 2\n.o 1\n.ilb a b\n.ob y\n.p 2\n0- 1\n10 2\n.end\n"
        );
    }

    #[test]
    fn verifies_equivalence_with_original_dc_set() {
        let original = Pla::parse(
            r#"
.i 2
.o 1
.type fd
00 1
01 -
.e
"#,
        )
        .unwrap();
        let minimized = Pla::parse(
            r#"
.i 2
.o 1
0- 1
.e
"#,
        )
        .unwrap();

        assert!(original.verify_minimized(&minimized).is_equivalent());
    }

    #[test]
    fn reports_non_equivalent_minimized_cover() {
        let original = Pla::parse(".i 2\n.o 1\n00 1\n.e\n").unwrap();
        let minimized = Pla::parse(".i 2\n.o 1\n01 1\n.e\n").unwrap();

        let report = original.verify_minimized(&minimized);

        assert_eq!(report.minimized_not_covered_by_original, [(1, 0)]);
        assert_eq!(report.original_not_covered_by_minimized, [(0, 0)]);
    }

    #[test]
    fn checks_partition_consistency() {
        let pla = Pla::parse(
            r#"
.i 1
.o 1
.type fdr
0 1
1 0
.e
"#,
        )
        .unwrap();

        assert!(pla.check_consistency().is_consistent());
    }

    #[test]
    fn pla_type_parser_accepts_espresso_type_tokens() {
        assert_eq!(parse_pla_type("fdr"), Some(FDR_TYPE));
        assert_eq!(parse_pla_type("x"), None);
    }
}
