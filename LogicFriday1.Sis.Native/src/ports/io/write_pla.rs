//! Native PLA writer for SIS networks.
//!
//! The SIS writer converts the current network to an Espresso PLA and delegates
//! text formatting to Espresso's PLA output routine. This module keeps that
//! same boundary in Rust: network conversion comes from `network_to_pla`, and
//! final text generation is handled by the native `cvrout` port.

use std::error::Error;
use std::fmt;
use std::io::{self, Write};

use crate::ports::espresso::cvrout::{
    self, CubeStructure, CvroutError, EspressoCover, EspressoCube, EspressoPla, OutputFormat,
    OutputSelection, Variable,
};
use crate::ports::network::net2pla::{Pla, PlaCube, network_to_pla};
use crate::ports::network::network_util::{CoverValue, Network, NetworkUtilError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaOutputType {
    OnSet,
    OnSetAndDontCare,
}

#[derive(Debug)]
pub enum WritePlaError {
    Network(NetworkUtilError),
    Espresso(CvroutError),
    Io(io::Error),
}

impl fmt::Display for WritePlaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => error.fmt(formatter),
            Self::Espresso(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl Error for WritePlaError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Network(error) => Some(error),
            Self::Espresso(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<NetworkUtilError> for WritePlaError {
    fn from(value: NetworkUtilError) -> Self {
        Self::Network(value)
    }
}

impl From<CvroutError> for WritePlaError {
    fn from(value: CvroutError) -> Self {
        Self::Espresso(value)
    }
}

impl From<io::Error> for WritePlaError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub type WritePlaResult<T> = Result<T, WritePlaError>;

pub fn write_pla<W>(writer: &mut W, network: &Network) -> WritePlaResult<bool>
where
    W: Write,
{
    let Some(pla) = network_to_pla(network)? else {
        return Ok(false);
    };

    write_pla_from_pla(writer, &pla)?;
    Ok(true)
}

pub fn write_pla_from_pla<W>(writer: &mut W, pla: &Pla) -> WritePlaResult<()>
where
    W: Write,
{
    let output = format_pla(pla)?;
    writer.write_all(output.as_bytes())?;

    Ok(())
}

pub fn format_pla(pla: &Pla) -> WritePlaResult<String> {
    let espresso_pla = to_espresso_pla(pla)?;
    let selection = match output_type_for_pla(pla) {
        PlaOutputType::OnSet => OutputSelection::on(),
        PlaOutputType::OnSetAndDontCare => OutputSelection {
            on_set: true,
            dont_care_set: true,
            off_set: false,
        },
    };

    Ok(cvrout::format_pla(
        &espresso_pla,
        OutputFormat::Pla(selection),
        [],
    )?)
}

pub fn output_type_for_pla(pla: &Pla) -> PlaOutputType {
    if pla.has_dc_set() {
        PlaOutputType::OnSetAndDontCare
    } else {
        PlaOutputType::OnSet
    }
}

fn to_espresso_pla(pla: &Pla) -> Result<EspressoPla, CvroutError> {
    let input_count = pla.input_labels.len();
    let output_count = pla.output_labels.len();
    let output_first_part = input_count * 2;
    let mut variables = Vec::with_capacity(input_count + usize::from(output_count > 0));

    for input_index in 0..input_count {
        let first_part = input_index * 2;
        variables.push(Variable::new(first_part, first_part + 1));
    }

    let output_variable = if output_count > 0 {
        variables.push(Variable::new(
            output_first_part,
            output_first_part + output_count - 1,
        ));
        Some(input_count)
    } else {
        None
    };

    let structure = CubeStructure::new(variables, input_count, output_variable)?;
    let labels = espresso_labels(pla);

    EspressoPla::new(
        structure,
        espresso_cover(&pla.on_set, input_count, output_first_part),
        espresso_cover(&pla.dc_set, input_count, output_first_part),
        EspressoCover::empty(),
        labels,
        None,
    )
}

fn espresso_labels(pla: &Pla) -> Vec<Option<String>> {
    let mut labels = vec![None; pla.input_labels.len() * 2 + pla.output_labels.len()];

    for (input_index, label) in pla.input_labels.iter().enumerate() {
        labels[input_index * 2 + 1] = Some(label.clone());
    }

    let output_first_part = pla.input_labels.len() * 2;
    for (output_index, label) in pla.output_labels.iter().enumerate() {
        labels[output_first_part + output_index] = Some(label.clone());
    }

    labels
}

fn espresso_cover(
    cubes: &[PlaCube],
    input_count: usize,
    output_first_part: usize,
) -> EspressoCover {
    EspressoCover::new(
        cubes
            .iter()
            .map(|cube| espresso_cube(cube, input_count, output_first_part)),
    )
}

fn espresso_cube(cube: &PlaCube, input_count: usize, output_first_part: usize) -> EspressoCube {
    let mut parts = Vec::with_capacity(input_count * 2 + 1);
    for input_index in 0..input_count {
        let first_part = input_index * 2;
        match cube.inputs[input_index] {
            CoverValue::Zero => {
                parts.push(first_part);
            }
            CoverValue::One => {
                parts.push(first_part + 1);
            }
            CoverValue::DontCare => {
                parts.push(first_part);
                parts.push(first_part + 1);
            }
        }
    }

    parts.push(output_first_part + cube.output);
    EspressoCube::from_parts(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::network::network_util::{Cube, NetworkNode, NodeKind, SopCover};

    fn cube(values: &[CoverValue]) -> Cube {
        Cube::new(values.to_vec())
    }

    fn pla_without_dc() -> Pla {
        Pla {
            input_labels: vec!["a".to_owned(), "b".to_owned()],
            output_labels: vec!["y".to_owned(), "z".to_owned()],
            on_set: vec![
                PlaCube::new(vec![CoverValue::One, CoverValue::Zero], 0),
                PlaCube::new(vec![CoverValue::DontCare, CoverValue::One], 1),
            ],
            dc_set: Vec::new(),
        }
    }

    #[test]
    fn formats_on_set_only_pla_like_f_type_output() {
        let output = format_pla(&pla_without_dc()).unwrap();

        assert_eq!(
            output,
            ".i 2\n.o 2\n.ilb a b\n.ob y z\n.p 2\n10 10\n-1 01\n.e\n"
        );
    }

    #[test]
    fn formats_dc_set_as_fd_output() {
        let mut pla = pla_without_dc();
        pla.dc_set.push(PlaCube::new(
            vec![CoverValue::Zero, CoverValue::DontCare],
            1,
        ));

        let output = format_pla(&pla).unwrap();

        assert_eq!(
            output,
            ".type fd\n.i 2\n.o 2\n.ilb a b\n.ob y z\n.p 3\n10 1~\n-1 ~1\n0- ~2\n.end\n"
        );
    }

    #[test]
    fn write_pla_from_pla_streams_cvrout_output() {
        let mut output = Vec::new();

        write_pla_from_pla(&mut output, &pla_without_dc()).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            ".i 2\n.o 2\n.ilb a b\n.ob y z\n.p 2\n10 10\n-1 01\n.e\n"
        );
    }

    #[test]
    fn write_pla_returns_false_when_network_has_no_pla_shape() {
        let network = Network::new();
        let mut output = Vec::new();

        assert!(!write_pla(&mut output, &network).unwrap());
        assert!(output.is_empty());
    }

    #[test]
    fn write_pla_converts_network_before_formatting() {
        let mut network = Network::new();
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let n = network
            .add_internal(
                "n",
                vec![a, b],
                SopCover::new([cube(&[CoverValue::One, CoverValue::Zero])]),
            )
            .unwrap();
        network.add_primary_output(n).unwrap();

        let mut output = Vec::new();
        assert!(write_pla(&mut output, &network).unwrap());

        assert_eq!(
            String::from_utf8(output).unwrap(),
            ".i 2\n.o 1\n.ilb a b\n.ob n\n.p 1\n10 1\n.e\n"
        );
    }

    #[test]
    fn write_pla_propagates_network_conversion_errors() {
        let mut network = Network::new();
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let n = network
            .add_internal(
                "n",
                vec![a],
                SopCover::new([cube(&[CoverValue::One, CoverValue::Zero])]),
            )
            .unwrap();
        network.add_primary_output(n).unwrap();

        assert!(matches!(
            write_pla(&mut Vec::new(), &network),
            Err(WritePlaError::Network(NetworkUtilError::InvalidCover { node, cube: 0 }))
                if node == n
        ));
    }
}
