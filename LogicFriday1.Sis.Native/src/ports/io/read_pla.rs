//! Native PLA reader for SIS IO.
//!
//! The legacy `sis_read_pla` path read an Espresso PLA, converted it through
//! the network PLA helpers, named both care and don't-care networks from the
//! current input filename, and attached the optional external don't-care
//! network. This module keeps that flow as a Rust-native parser and owned
//! network result.

use std::error::Error;
use std::fmt;

use crate::ports::network::network_util::Network;
use crate::ports::network::pla2net::{
    Pla, PlaInputValue, PlaRow, PlaToNetworkError, pla_to_dcnetwork_single, pla_to_network,
    pla_to_network_single,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaCoverType {
    On,
    DontCare,
    Off,
    Unspecified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadPlaError {
    MissingInputCount,
    MissingOutputCount,
    InvalidDirective {
        line: usize,
        directive: String,
    },
    InvalidCount {
        line: usize,
        directive: &'static str,
        value: String,
    },
    WrongLabelCount {
        line: usize,
        directive: &'static str,
        expected: usize,
        actual: usize,
    },
    InvalidCube {
        line: usize,
        reason: String,
    },
    Network(PlaToNetworkError),
}

impl fmt::Display for ReadPlaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInputCount => write!(formatter, "PLA is missing an .i declaration"),
            Self::MissingOutputCount => write!(formatter, "PLA is missing an .o declaration"),
            Self::InvalidDirective { line, directive } => {
                write!(
                    formatter,
                    "unsupported PLA directive {directive} on line {line}"
                )
            }
            Self::InvalidCount {
                line,
                directive,
                value,
            } => write!(
                formatter,
                "invalid {directive} count {value:?} on line {line}"
            ),
            Self::WrongLabelCount {
                line,
                directive,
                expected,
                actual,
            } => write!(
                formatter,
                "{directive} on line {line} has {actual} labels but {expected} were expected"
            ),
            Self::InvalidCube { line, reason } => {
                write!(formatter, "invalid PLA cube on line {line}: {reason}")
            }
            Self::Network(error) => error.fmt(formatter),
        }
    }
}

impl Error for ReadPlaError {}

impl From<PlaToNetworkError> for ReadPlaError {
    fn from(value: PlaToNetworkError) -> Self {
        Self::Network(value)
    }
}

pub type ReadPlaResult<T> = Result<T, ReadPlaError>;

pub fn sis_read_pla(
    input: &str,
    single_output: bool,
    read_filename: Option<&str>,
) -> ReadPlaResult<Network> {
    let pla = espresso_read_pla(input)?;
    let mut network = if single_output {
        pla_to_network_single(&pla)?
    } else {
        pla_to_network(&pla)?
    };

    if let Some(name) = read_filename.map(read_filename_to_netname) {
        network.set_name(name.clone());
    }

    if let Some(mut dc_network) = pla_to_dcnetwork_single(&pla)? {
        if let Some(name) = read_filename.map(read_filename_to_netname) {
            dc_network.set_name(name);
        }
        network.set_dc_network(Some(dc_network));
    }

    Ok(network)
}

pub fn espresso_read_pla(input: &str) -> ReadPlaResult<Pla> {
    let mut parser = PlaParser::default();

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('.') {
            if parser.parse_directive(line_number, line)? {
                break;
            }
        } else {
            parser.parse_cube(line_number, line)?;
        }
    }

    parser.finish()
}

pub fn read_filename_to_netname(filename: &str) -> String {
    filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .to_owned()
}

#[derive(Clone, Debug)]
struct ParsedCube {
    inputs: Vec<PlaInputValue>,
    outputs: Vec<PlaCoverType>,
}

#[derive(Default)]
struct PlaParser {
    input_count: Option<usize>,
    output_count: Option<usize>,
    input_labels: Option<Vec<String>>,
    output_labels: Option<Vec<String>>,
    cubes: Vec<ParsedCube>,
}

impl PlaParser {
    fn parse_directive(&mut self, line: usize, text: &str) -> ReadPlaResult<bool> {
        let mut words = text.split_whitespace();
        let directive = words.next().unwrap_or_default();
        let values = words.collect::<Vec<_>>();

        match directive {
            ".e" | ".end" => Ok(true),
            ".i" => {
                self.input_count = Some(parse_count(line, ".i", values.first())?);
                Ok(false)
            }
            ".o" => {
                self.output_count = Some(parse_count(line, ".o", values.first())?);
                Ok(false)
            }
            ".ilb" => {
                let expected = self.input_count.ok_or(ReadPlaError::MissingInputCount)?;
                self.input_labels = Some(parse_labels(line, ".ilb", expected, &values)?);
                Ok(false)
            }
            ".ob" => {
                let expected = self.output_count.ok_or(ReadPlaError::MissingOutputCount)?;
                self.output_labels = Some(parse_labels(line, ".ob", expected, &values)?);
                Ok(false)
            }
            ".type" => {
                validate_type(line, &values)?;
                Ok(false)
            }
            ".p" | ".phase" => Ok(false),
            _ => Err(ReadPlaError::InvalidDirective {
                line,
                directive: directive.to_owned(),
            }),
        }
    }

    fn parse_cube(&mut self, line: usize, text: &str) -> ReadPlaResult<()> {
        let input_count = self.input_count.ok_or(ReadPlaError::MissingInputCount)?;
        let output_count = self.output_count.ok_or(ReadPlaError::MissingOutputCount)?;
        let compact = text
            .chars()
            .filter(|ch| !ch.is_whitespace() && *ch != '|')
            .collect::<String>();
        let expected = input_count + output_count;
        if compact.chars().count() != expected {
            return Err(ReadPlaError::InvalidCube {
                line,
                reason: format!("expected {expected} symbols but found {}", compact.len()),
            });
        }

        let inputs = compact
            .chars()
            .take(input_count)
            .map(|ch| parse_input_value(line, ch))
            .collect::<ReadPlaResult<Vec<_>>>()?;
        let outputs = compact
            .chars()
            .skip(input_count)
            .map(|ch| parse_output_value(line, ch))
            .collect::<ReadPlaResult<Vec<_>>>()?;

        self.cubes.push(ParsedCube { inputs, outputs });
        Ok(())
    }

    fn finish(self) -> ReadPlaResult<Pla> {
        let input_count = self.input_count.ok_or(ReadPlaError::MissingInputCount)?;
        let output_count = self.output_count.ok_or(ReadPlaError::MissingOutputCount)?;
        let input_labels = self
            .input_labels
            .unwrap_or_else(|| default_input_labels(input_count));
        let output_labels = self
            .output_labels
            .unwrap_or_else(|| default_output_labels(output_count));

        let mut on_set = Vec::new();
        let mut dc_set = Vec::new();
        for cube in self.cubes {
            let on_outputs = cube
                .outputs
                .iter()
                .map(|value| *value == PlaCoverType::On)
                .collect::<Vec<_>>();
            if on_outputs.iter().any(|value| *value) {
                on_set.push(PlaRow::new(cube.inputs.clone(), on_outputs));
            }

            let dc_outputs = cube
                .outputs
                .iter()
                .map(|value| *value == PlaCoverType::DontCare)
                .collect::<Vec<_>>();
            if dc_outputs.iter().any(|value| *value) {
                dc_set.push(PlaRow::new(cube.inputs, dc_outputs));
            }
        }

        Ok(Pla::new(input_labels, output_labels, on_set, dc_set))
    }
}

fn parse_count(line: usize, directive: &'static str, value: Option<&&str>) -> ReadPlaResult<usize> {
    let Some(value) = value else {
        return Err(ReadPlaError::InvalidCount {
            line,
            directive,
            value: String::new(),
        });
    };

    value
        .parse::<usize>()
        .map_err(|_| ReadPlaError::InvalidCount {
            line,
            directive,
            value: (*value).to_owned(),
        })
}

fn parse_labels(
    line: usize,
    directive: &'static str,
    expected: usize,
    values: &[&str],
) -> ReadPlaResult<Vec<String>> {
    if values.len() != expected {
        return Err(ReadPlaError::WrongLabelCount {
            line,
            directive,
            expected,
            actual: values.len(),
        });
    }

    Ok(values.iter().map(|value| (*value).to_owned()).collect())
}

fn validate_type(line: usize, values: &[&str]) -> ReadPlaResult<()> {
    match values {
        ["f" | "fd" | "fr" | "fdr" | "d" | "dr" | "r"] => Ok(()),
        [value] => Err(ReadPlaError::InvalidDirective {
            line,
            directive: format!(".type {value}"),
        }),
        _ => Err(ReadPlaError::InvalidDirective {
            line,
            directive: ".type".to_owned(),
        }),
    }
}

fn parse_input_value(line: usize, value: char) -> ReadPlaResult<PlaInputValue> {
    match value {
        '0' => Ok(PlaInputValue::Zero),
        '1' => Ok(PlaInputValue::One),
        '-' | '2' | '?' => Ok(PlaInputValue::DontCare),
        _ => Err(ReadPlaError::InvalidCube {
            line,
            reason: format!("invalid input symbol {value:?}"),
        }),
    }
}

fn parse_output_value(line: usize, value: char) -> ReadPlaResult<PlaCoverType> {
    match value {
        '1' | '4' => Ok(PlaCoverType::On),
        '-' | '2' => Ok(PlaCoverType::DontCare),
        '0' | '3' => Ok(PlaCoverType::Off),
        '~' => Ok(PlaCoverType::Unspecified),
        _ => Err(ReadPlaError::InvalidCube {
            line,
            reason: format!("invalid output symbol {value:?}"),
        }),
    }
}

fn default_input_labels(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("v{index}")).collect()
}

fn default_output_labels(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| format!("v{index}.{index}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::network::network_util::CoverValue;

    const PLA_TEXT: &str = "\
.i 2
.o 2
.ilb a b
.ob y z
.type fd
10 10
-1 1-
0- 02
.e
";

    #[test]
    fn parses_binary_pla_into_on_and_dc_sets() {
        let pla = espresso_read_pla(PLA_TEXT).unwrap();

        assert_eq!(pla.input_labels, ["a", "b"]);
        assert_eq!(pla.output_labels, ["y", "z"]);
        assert_eq!(pla.on_set.len(), 2);
        assert_eq!(pla.dc_set.len(), 2);
        assert_eq!(
            pla.on_set[0].inputs,
            [PlaInputValue::One, PlaInputValue::Zero]
        );
        assert_eq!(pla.on_set[1].outputs, [true, false]);
        assert_eq!(pla.dc_set[0].outputs, [false, true]);
        assert_eq!(pla.dc_set[1].outputs, [false, true]);
    }

    #[test]
    fn single_output_read_attaches_named_dc_network() {
        let network = sis_read_pla(PLA_TEXT, true, Some("examples/demo.pla")).unwrap();

        assert_eq!(network.name(), "demo.pla");
        assert_eq!(network.num_pi(), 2);
        assert_eq!(network.num_po(), 2);
        assert!(network.dc_network().is_some());
        assert_eq!(network.dc_network().unwrap().name(), "demo.pla");

        let y = network.find_node("y").unwrap();
        let y_driver = network.node(y).unwrap().fanins[0];
        let y_cover = network.node(y_driver).unwrap().cover.as_ref().unwrap();
        assert_eq!(y_cover.cubes().len(), 2);
        assert_eq!(
            y_cover.cubes()[0].values(),
            &[CoverValue::One, CoverValue::Zero]
        );
    }

    #[test]
    fn two_level_read_uses_product_terms_and_attaches_dc_network() {
        let network = sis_read_pla(PLA_TEXT, false, Some("C:\\tmp\\demo.pla")).unwrap();

        assert_eq!(network.name(), "demo.pla");
        assert_eq!(network.num_pi(), 2);
        assert_eq!(network.num_po(), 2);
        assert_eq!(network.num_internal(), 4);
        assert!(network.dc_network().is_some());
    }

    #[test]
    fn defaults_missing_labels_to_espresso_style_names() {
        let pla = espresso_read_pla(".i 2\n.o 1\n11 1\n.e\n").unwrap();

        assert_eq!(pla.input_labels, ["v0", "v1"]);
        assert_eq!(pla.output_labels, ["v0.0"]);
    }

    #[test]
    fn rejects_cubes_before_shape_is_declared() {
        assert_eq!(
            espresso_read_pla("11 1\n").unwrap_err(),
            ReadPlaError::MissingInputCount
        );
    }

    #[test]
    fn rejects_wrong_label_count() {
        assert_eq!(
            espresso_read_pla(".i 2\n.o 1\n.ilb a\n.e\n").unwrap_err(),
            ReadPlaError::WrongLabelCount {
                line: 3,
                directive: ".ilb",
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn rejects_invalid_cube_symbols_and_lengths() {
        assert!(matches!(
            espresso_read_pla(".i 1\n.o 1\nx1\n.e\n"),
            Err(ReadPlaError::InvalidCube { line: 3, .. })
        ));
        assert!(matches!(
            espresso_read_pla(".i 1\n.o 1\n111\n.e\n"),
            Err(ReadPlaError::InvalidCube { line: 3, .. })
        ));
    }

    #[test]
    fn read_without_dc_set_has_no_attached_dc_network() {
        let network = sis_read_pla(".i 1\n.o 1\n1 1\n.e\n", true, None).unwrap();

        assert!(network.dc_network().is_none());
        assert_eq!(network.name(), "unknown");
    }
}
