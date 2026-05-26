//! Bounded native helper for two-level BLIF-style mapper inputs.
//!
//! `LogicSynthesis/sis/map/two_level.c` is not present in this checkout. This
//! module therefore captures the bounded owned-data portion needed by mapper
//! input preparation: parse `.inputs`, `.outputs`, and `.names` tables into a
//! validated two-level model, then emit the same constrained BLIF subset. Direct
//! SIS `network_t` integration is intentionally left to the native network/node
//! ports.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseLimits {
    pub max_lines: usize,
    pub max_line_length: usize,
    pub max_name_length: usize,
    pub max_inputs: usize,
    pub max_outputs: usize,
    pub max_nodes: usize,
    pub max_cubes_per_node: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_lines: 16_384,
            max_line_length: 8_192,
            max_name_length: 256,
            max_inputs: 4_096,
            max_outputs: 4_096,
            max_nodes: 16_384,
            max_cubes_per_node: 262_144,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlifLiteral {
    Zero,
    One,
    DontCare,
}

impl BlifLiteral {
    fn parse(value: u8) -> Option<Self> {
        match value {
            b'0' => Some(Self::Zero),
            b'1' => Some(Self::One),
            b'-' => Some(Self::DontCare),
            _ => None,
        }
    }

    fn as_char(self) -> char {
        match self {
            Self::Zero => '0',
            Self::One => '1',
            Self::DontCare => '-',
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlifCube {
    pub literals: Vec<BlifLiteral>,
    pub output_value: bool,
}

impl BlifCube {
    pub fn new(literals: Vec<BlifLiteral>, output_value: bool) -> Self {
        Self {
            literals,
            output_value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoLevelNode {
    pub fanins: Vec<String>,
    pub output: String,
    pub cubes: Vec<BlifCube>,
}

impl TwoLevelNode {
    pub fn new(
        fanins: Vec<String>,
        output: impl Into<String>,
        cubes: Vec<BlifCube>,
    ) -> Result<Self, TwoLevelError> {
        let output = output.into();
        let node = Self {
            fanins,
            output,
            cubes,
        };
        node.validate()?;
        Ok(node)
    }

    pub fn constant(output: impl Into<String>, value: bool) -> Self {
        let cubes = if value {
            vec![BlifCube::new(Vec::new(), true)]
        } else {
            Vec::new()
        };
        Self {
            fanins: Vec::new(),
            output: output.into(),
            cubes,
        }
    }

    fn validate(&self) -> Result<(), TwoLevelError> {
        validate_name(
            &self.output,
            "node output",
            ParseLimits::default().max_name_length,
        )?;
        validate_unique_names(&self.fanins, "fanin")?;
        for (cube_index, cube) in self.cubes.iter().enumerate() {
            if cube.literals.len() != self.fanins.len() {
                return Err(TwoLevelError::CubeWidthMismatch {
                    node: self.output.clone(),
                    cube_index,
                    expected: self.fanins.len(),
                    actual: cube.literals.len(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoLevelModel {
    pub model_name: Option<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub nodes: Vec<TwoLevelNode>,
}

impl TwoLevelModel {
    pub fn new(
        model_name: Option<String>,
        inputs: Vec<String>,
        outputs: Vec<String>,
        nodes: Vec<TwoLevelNode>,
    ) -> Result<Self, TwoLevelError> {
        let model = Self {
            model_name,
            inputs,
            outputs,
            nodes,
        };
        model.validate()?;
        Ok(model)
    }

    pub fn validate(&self) -> Result<(), TwoLevelError> {
        if let Some(model_name) = &self.model_name {
            validate_name(model_name, "model", ParseLimits::default().max_name_length)?;
        }
        validate_unique_names(&self.inputs, "input")?;
        validate_unique_names(&self.outputs, "output")?;

        let input_set = self.inputs.iter().collect::<HashSet<_>>();
        let mut produced = HashSet::new();
        for node in &self.nodes {
            node.validate()?;
            if !produced.insert(&node.output) {
                return Err(TwoLevelError::DuplicateName {
                    kind: "node output",
                    name: node.output.clone(),
                });
            }
            for fanin in &node.fanins {
                if !input_set.contains(fanin) && !produced.contains(fanin) {
                    return Err(TwoLevelError::UnknownFanin {
                        node: node.output.clone(),
                        fanin: fanin.clone(),
                    });
                }
            }
        }

        for output in &self.outputs {
            if !input_set.contains(output) && !produced.contains(output) {
                return Err(TwoLevelError::MissingOutputDriver {
                    output: output.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn to_blif(&self) -> Result<String, TwoLevelError> {
        self.validate()?;

        let mut output = String::new();
        if let Some(model_name) = &self.model_name {
            output.push_str(".model ");
            output.push_str(model_name);
            output.push('\n');
        }
        push_directive(&mut output, ".inputs", &self.inputs);
        push_directive(&mut output, ".outputs", &self.outputs);
        for node in &self.nodes {
            output.push_str(".names");
            for fanin in &node.fanins {
                output.push(' ');
                output.push_str(fanin);
            }
            output.push(' ');
            output.push_str(&node.output);
            output.push('\n');
            for cube in &node.cubes {
                for literal in &cube.literals {
                    output.push(literal.as_char());
                }
                if !cube.literals.is_empty() {
                    output.push(' ');
                }
                output.push(if cube.output_value { '1' } else { '0' });
                output.push('\n');
            }
        }
        output.push_str(".end\n");
        Ok(output)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TwoLevelError {
    LineLimitExceeded {
        max_lines: usize,
    },
    LineTooLong {
        line: usize,
        max_line_length: usize,
    },
    NameTooLong {
        kind: &'static str,
        name: String,
        max_name_length: usize,
    },
    EmptyName {
        kind: &'static str,
    },
    DuplicateName {
        kind: &'static str,
        name: String,
    },
    TooManyNames {
        kind: &'static str,
        max: usize,
    },
    TooManyNodes {
        max: usize,
    },
    TooManyCubes {
        node: String,
        max: usize,
    },
    UnknownDirective {
        line: usize,
        directive: String,
    },
    MissingDirectiveValue {
        line: usize,
        directive: &'static str,
    },
    CubeWithoutNames {
        line: usize,
    },
    InvalidCubePattern {
        line: usize,
        pattern: String,
    },
    InvalidCubeOutput {
        line: usize,
        value: String,
    },
    CubeWidthMismatch {
        node: String,
        cube_index: usize,
        expected: usize,
        actual: usize,
    },
    UnknownFanin {
        node: String,
        fanin: String,
    },
    MissingOutputDriver {
        output: String,
    },
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for TwoLevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineLimitExceeded { max_lines } => {
                write!(f, "BLIF input exceeds {max_lines} logical lines")
            }
            Self::LineTooLong {
                line,
                max_line_length,
            } => write!(f, "line {line} exceeds {max_line_length} characters"),
            Self::NameTooLong {
                kind,
                name,
                max_name_length,
            } => write!(
                f,
                "{kind} name '{name}' exceeds {max_name_length} characters"
            ),
            Self::EmptyName { kind } => write!(f, "{kind} name cannot be empty"),
            Self::DuplicateName { kind, name } => write!(f, "duplicate {kind} name '{name}'"),
            Self::TooManyNames { kind, max } => write!(f, "too many {kind} names; max is {max}"),
            Self::TooManyNodes { max } => write!(f, "too many .names nodes; max is {max}"),
            Self::TooManyCubes { node, max } => {
                write!(f, "node '{node}' has too many cubes; max is {max}")
            }
            Self::UnknownDirective { line, directive } => {
                write!(
                    f,
                    "line {line} contains unsupported directive '{directive}'"
                )
            }
            Self::MissingDirectiveValue { line, directive } => {
                write!(f, "line {line} has no values for {directive}")
            }
            Self::CubeWithoutNames { line } => write!(f, "line {line} has a cube before .names"),
            Self::InvalidCubePattern { line, pattern } => {
                write!(f, "line {line} has invalid cube pattern '{pattern}'")
            }
            Self::InvalidCubeOutput { line, value } => {
                write!(f, "line {line} has invalid cube output '{value}'")
            }
            Self::CubeWidthMismatch {
                node,
                cube_index,
                expected,
                actual,
            } => write!(
                f,
                "node '{node}' cube {cube_index} has width {actual}, expected {expected}"
            ),
            Self::UnknownFanin { node, fanin } => {
                write!(f, "node '{node}' references unknown fanin '{fanin}'")
            }
            Self::MissingOutputDriver { output } => {
                write!(
                    f,
                    "output '{output}' is not driven by an input or .names node"
                )
            }
            Self::MissingSisPorts { operation } => write!(f, "{operation} requires unavailable native SIS integration"),
        }
    }
}

impl Error for TwoLevelError {}

pub fn parse_blif(input: &str, limits: ParseLimits) -> Result<TwoLevelModel, TwoLevelError> {
    let lines = logical_lines(input, limits)?;
    let mut model_name = None;
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut nodes = Vec::new();
    let mut current_node: Option<TwoLevelNode> = None;

    for (line_number, line) in lines {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.is_empty() {
            continue;
        }

        if fields[0].starts_with('.') {
            if let Some(node) = current_node.take() {
                nodes.push(node);
            }

            match fields[0] {
                ".model" => {
                    let value = only_directive_value(&fields, line_number, ".model")?;
                    validate_name(value, "model", limits.max_name_length)?;
                    model_name = Some(value.to_string());
                }
                ".inputs" => {
                    extend_names(
                        &mut inputs,
                        &fields[1..],
                        "input",
                        limits.max_inputs,
                        limits,
                    )?;
                }
                ".outputs" => {
                    extend_names(
                        &mut outputs,
                        &fields[1..],
                        "output",
                        limits.max_outputs,
                        limits,
                    )?;
                }
                ".names" => {
                    if nodes.len() >= limits.max_nodes {
                        return Err(TwoLevelError::TooManyNodes {
                            max: limits.max_nodes,
                        });
                    }
                    if fields.len() < 2 {
                        return Err(TwoLevelError::MissingDirectiveValue {
                            line: line_number,
                            directive: ".names",
                        });
                    }
                    let output = fields[fields.len() - 1];
                    validate_name(output, "node output", limits.max_name_length)?;
                    let fanins = fields[1..fields.len() - 1]
                        .iter()
                        .map(|name| {
                            validate_name(name, "fanin", limits.max_name_length)?;
                            Ok((*name).to_string())
                        })
                        .collect::<Result<Vec<_>, TwoLevelError>>()?;
                    validate_unique_names(&fanins, "fanin")?;
                    current_node = Some(TwoLevelNode {
                        fanins,
                        output: output.to_string(),
                        cubes: Vec::new(),
                    });
                }
                ".end" => break,
                directive => {
                    return Err(TwoLevelError::UnknownDirective {
                        line: line_number,
                        directive: directive.to_string(),
                    });
                }
            }
            continue;
        }

        let Some(node) = current_node.as_mut() else {
            return Err(TwoLevelError::CubeWithoutNames { line: line_number });
        };
        if node.cubes.len() >= limits.max_cubes_per_node {
            return Err(TwoLevelError::TooManyCubes {
                node: node.output.clone(),
                max: limits.max_cubes_per_node,
            });
        }
        node.cubes
            .push(parse_cube(&fields, node.fanins.len(), line_number)?);
    }

    if let Some(node) = current_node.take() {
        nodes.push(node);
    }

    if inputs.len() > limits.max_inputs {
        return Err(TwoLevelError::TooManyNames {
            kind: "input",
            max: limits.max_inputs,
        });
    }
    if outputs.len() > limits.max_outputs {
        return Err(TwoLevelError::TooManyNames {
            kind: "output",
            max: limits.max_outputs,
        });
    }

    TwoLevelModel::new(model_name, inputs, outputs, nodes)
}

pub fn synthesize_blif(model: &TwoLevelModel) -> Result<String, TwoLevelError> {
    model.to_blif()
}

pub fn network_to_two_level_unavailable() -> Result<TwoLevelModel, TwoLevelError> {
    Err(TwoLevelError::MissingSisPorts {
        operation: "network_to_two_level native SIS integration",
    })
}

fn logical_lines(input: &str, limits: ParseLimits) -> Result<Vec<(usize, String)>, TwoLevelError> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut start_line = 1;

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        if line_number > limits.max_lines {
            return Err(TwoLevelError::LineLimitExceeded {
                max_lines: limits.max_lines,
            });
        }
        if raw_line.len() > limits.max_line_length {
            return Err(TwoLevelError::LineTooLong {
                line: line_number,
                max_line_length: limits.max_line_length,
            });
        }

        let line = strip_comment(raw_line).trim_end();
        let continued = line.ends_with('\\');
        let piece = if continued {
            line[..line.len() - 1].trim_end()
        } else {
            line
        };

        if current.is_empty() {
            start_line = line_number;
            current.push_str(piece.trim_start());
        } else {
            current.push(' ');
            current.push_str(piece.trim());
        }

        if !continued {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                lines.push((start_line, trimmed.to_string()));
            }
            current.clear();
        }
    }

    if !current.trim().is_empty() {
        lines.push((start_line, current.trim().to_string()));
    }

    Ok(lines)
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .map(|(prefix, _)| prefix)
        .unwrap_or(line)
}

fn only_directive_value<'a>(
    fields: &'a [&str],
    line: usize,
    directive: &'static str,
) -> Result<&'a str, TwoLevelError> {
    if fields.len() < 2 {
        return Err(TwoLevelError::MissingDirectiveValue { line, directive });
    }
    Ok(fields[1])
}

fn extend_names(
    target: &mut Vec<String>,
    values: &[&str],
    kind: &'static str,
    max: usize,
    limits: ParseLimits,
) -> Result<(), TwoLevelError> {
    if values.is_empty() {
        return Err(TwoLevelError::MissingDirectiveValue {
            line: 0,
            directive: if kind == "input" {
                ".inputs"
            } else {
                ".outputs"
            },
        });
    }
    for value in values {
        if target.len() >= max {
            return Err(TwoLevelError::TooManyNames { kind, max });
        }
        validate_name(value, kind, limits.max_name_length)?;
        target.push((*value).to_string());
    }
    validate_unique_names(target, kind)
}

fn parse_cube(fields: &[&str], fanin_count: usize, line: usize) -> Result<BlifCube, TwoLevelError> {
    let (pattern, output_value) = match fields {
        [value] if fanin_count == 0 => ("", *value),
        [pattern, value] => (*pattern, *value),
        [pattern] => (*pattern, "1"),
        _ => {
            return Err(TwoLevelError::InvalidCubePattern {
                line,
                pattern: fields.join(" "),
            });
        }
    };

    if pattern.len() != fanin_count {
        return Err(TwoLevelError::CubeWidthMismatch {
            node: "<parse>".to_string(),
            cube_index: 0,
            expected: fanin_count,
            actual: pattern.len(),
        });
    }

    let literals = pattern
        .bytes()
        .map(|value| {
            BlifLiteral::parse(value).ok_or_else(|| TwoLevelError::InvalidCubePattern {
                line,
                pattern: pattern.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let output_value = match output_value {
        "0" => false,
        "1" => true,
        value => {
            return Err(TwoLevelError::InvalidCubeOutput {
                line,
                value: value.to_string(),
            });
        }
    };

    Ok(BlifCube {
        literals,
        output_value,
    })
}

fn validate_name(
    name: &str,
    kind: &'static str,
    max_name_length: usize,
) -> Result<(), TwoLevelError> {
    if name.is_empty() {
        return Err(TwoLevelError::EmptyName { kind });
    }
    if name.len() > max_name_length {
        return Err(TwoLevelError::NameTooLong {
            kind,
            name: name.to_string(),
            max_name_length,
        });
    }
    Ok(())
}

fn validate_unique_names(values: &[String], kind: &'static str) -> Result<(), TwoLevelError> {
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(TwoLevelError::DuplicateName {
                kind,
                name: value.clone(),
            });
        }
    }
    Ok(())
}

fn push_directive(output: &mut String, directive: &str, names: &[String]) {
    output.push_str(directive);
    for name in names {
        output.push(' ');
        output.push_str(name);
    }
    output.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(value: char) -> BlifLiteral {
        match value {
            '0' => BlifLiteral::Zero,
            '1' => BlifLiteral::One,
            '-' => BlifLiteral::DontCare,
            other => panic!("unexpected literal {other}"),
        }
    }

    #[test]
    fn parses_two_level_blif_subset() {
        let model = parse_blif(
            concat!(
                ".model demo\n",
                ".inputs a b c\n",
                ".outputs f\n",
                ".names a b c f\n",
                "1-0 1\n",
                "01- 1\n",
                ".end\n"
            ),
            ParseLimits::default(),
        )
        .unwrap();

        assert_eq!(model.model_name, Some("demo".to_string()));
        assert_eq!(model.inputs, vec!["a", "b", "c"]);
        assert_eq!(model.outputs, vec!["f"]);
        assert_eq!(model.nodes.len(), 1);
        assert_eq!(model.nodes[0].fanins, vec!["a", "b", "c"]);
        assert_eq!(model.nodes[0].output, "f");
        assert_eq!(
            model.nodes[0].cubes,
            vec![
                BlifCube::new(vec![lit('1'), lit('-'), lit('0')], true),
                BlifCube::new(vec![lit('0'), lit('1'), lit('-')], true),
            ]
        );
    }

    #[test]
    fn emits_canonical_blif_subset() {
        let model = TwoLevelModel::new(
            Some("demo".to_string()),
            vec!["a".to_string(), "b".to_string()],
            vec!["f".to_string()],
            vec![
                TwoLevelNode::new(
                    vec!["a".to_string(), "b".to_string()],
                    "f",
                    vec![
                        BlifCube::new(vec![lit('1'), lit('-')], true),
                        BlifCube::new(vec![lit('0'), lit('1')], true),
                    ],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(
            synthesize_blif(&model).unwrap(),
            concat!(
                ".model demo\n",
                ".inputs a b\n",
                ".outputs f\n",
                ".names a b f\n",
                "1- 1\n",
                "01 1\n",
                ".end\n"
            )
        );
    }

    #[test]
    fn parses_constant_one_and_constant_zero_nodes() {
        let model = parse_blif(
            concat!(
                ".outputs one zero\n",
                ".names one\n",
                "1\n",
                ".names zero\n",
                ".end\n"
            ),
            ParseLimits::default(),
        )
        .unwrap();

        assert_eq!(model.nodes[0], TwoLevelNode::constant("one", true));
        assert_eq!(model.nodes[1], TwoLevelNode::constant("zero", false));
    }

    #[test]
    fn bounds_lines_names_nodes_and_cubes() {
        let err = parse_blif(
            ".inputs abc\n",
            ParseLimits {
                max_name_length: 2,
                ..Default::default()
            },
        )
        .expect_err("name should exceed configured limit");
        assert!(matches!(err, TwoLevelError::NameTooLong { .. }));

        let err = parse_blif(
            ".outputs f\n.names f\n1\n",
            ParseLimits {
                max_cubes_per_node: 0,
                ..Default::default()
            },
        )
        .expect_err("cube count should exceed configured limit");
        assert_eq!(
            err,
            TwoLevelError::TooManyCubes {
                node: "f".to_string(),
                max: 0,
            }
        );
    }

    #[test]
    fn rejects_unknown_fanins_and_missing_output_drivers() {
        let err = parse_blif(
            concat!(".inputs a\n", ".outputs f\n", ".names missing f\n", "1 1\n"),
            ParseLimits::default(),
        )
        .expect_err("unknown fanin should be rejected");
        assert_eq!(
            err,
            TwoLevelError::UnknownFanin {
                node: "f".to_string(),
                fanin: "missing".to_string(),
            }
        );

        let err = parse_blif(".inputs a\n.outputs f\n", ParseLimits::default())
            .expect_err("undriven output should be rejected");
        assert_eq!(
            err,
            TwoLevelError::MissingOutputDriver {
                output: "f".to_string(),
            }
        );
    }
}
