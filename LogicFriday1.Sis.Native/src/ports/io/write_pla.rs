//! Native PLA writer for SIS networks.
//!
//! The legacy `write_pla.c` converts the current network to an Espresso PLA
//! and delegates the text formatting to `fprint_pla`.  This Rust port keeps the
//! same split: `network_to_pla` owns network conversion, while this module owns
//! deterministic Espresso PLA text output.

use std::error::Error;
use std::fmt;
use std::io::{self, Write};

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
    Io(io::Error),
}

impl fmt::Display for WritePlaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl Error for WritePlaError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Network(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<NetworkUtilError> for WritePlaError {
    fn from(value: NetworkUtilError) -> Self {
        Self::Network(value)
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

pub fn write_pla_from_pla<W>(writer: &mut W, pla: &Pla) -> io::Result<()>
where
    W: Write,
{
    let output_type = output_type_for_pla(pla);
    write_header(writer, pla, output_type)?;

    let product_count = match output_type {
        PlaOutputType::OnSet => pla.on_set.len(),
        PlaOutputType::OnSetAndDontCare => pla.on_set.len() + pla.dc_set.len(),
    };
    writeln!(writer, ".p {product_count}")?;

    match output_type {
        PlaOutputType::OnSet => {
            write_cubes(writer, &pla.on_set, pla.output_labels.len(), '0', '1')?;
            writeln!(writer, ".e")?;
        }
        PlaOutputType::OnSetAndDontCare => {
            write_cubes(writer, &pla.on_set, pla.output_labels.len(), '~', '1')?;
            write_cubes(writer, &pla.dc_set, pla.output_labels.len(), '~', '2')?;
            writeln!(writer, ".end")?;
        }
    }

    Ok(())
}

pub fn format_pla(pla: &Pla) -> String {
    let mut output = Vec::new();
    write_pla_from_pla(&mut output, pla).expect("writing to an in-memory buffer cannot fail");
    String::from_utf8(output).expect("PLA output is ASCII")
}

pub fn output_type_for_pla(pla: &Pla) -> PlaOutputType {
    if pla.has_dc_set() {
        PlaOutputType::OnSetAndDontCare
    } else {
        PlaOutputType::OnSet
    }
}

fn write_header<W>(writer: &mut W, pla: &Pla, output_type: PlaOutputType) -> io::Result<()>
where
    W: Write,
{
    if output_type == PlaOutputType::OnSetAndDontCare {
        writeln!(writer, ".type fd")?;
    }

    writeln!(writer, ".i {}", pla.input_labels.len())?;
    writeln!(writer, ".o {}", pla.output_labels.len())?;

    if !pla.input_labels.is_empty() {
        writeln!(writer, ".ilb {}", pla.input_labels.join(" "))?;
    }

    if !pla.output_labels.is_empty() {
        writeln!(writer, ".ob {}", pla.output_labels.join(" "))?;
    }

    Ok(())
}

fn write_cubes<W>(
    writer: &mut W,
    cubes: &[PlaCube],
    output_count: usize,
    output_absent: char,
    output_present: char,
) -> io::Result<()>
where
    W: Write,
{
    for cube in cubes {
        write_inputs(writer, &cube.inputs)?;
        write!(writer, " ")?;
        for output_index in 0..output_count {
            let value = if output_index == cube.output {
                output_present
            } else {
                output_absent
            };
            write!(writer, "{value}")?;
        }
        writeln!(writer)?;
    }

    Ok(())
}

fn write_inputs<W>(writer: &mut W, inputs: &[CoverValue]) -> io::Result<()>
where
    W: Write,
{
    for input in inputs {
        write!(writer, "{}", cover_value_char(*input))?;
    }

    Ok(())
}

fn cover_value_char(value: CoverValue) -> char {
    match value {
        CoverValue::Zero => '0',
        CoverValue::One => '1',
        CoverValue::DontCare => '-',
    }
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
        let output = format_pla(&pla_without_dc());

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

        let output = format_pla(&pla);

        assert_eq!(
            output,
            ".type fd\n.i 2\n.o 2\n.ilb a b\n.ob y z\n.p 3\n10 1~\n-1 ~1\n0- ~2\n.end\n"
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
