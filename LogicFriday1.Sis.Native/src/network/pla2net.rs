//! Native PLA-to-network construction for the SIS network layer.
//!
//! The C implementation converted Espresso `pPLA` structures into SIS
//! `network_t` values. This port keeps the construction rules in safe Rust:
//! primary inputs come from the PLA input labels, primary outputs come from the
//! PLA output labels, and PLA rows become SOP covers on native network nodes.

use std::error::Error;
use std::fmt;

use super::network_util::{
    CoverValue, Cube, Network, NetworkNode, NetworkUtilError, NodeId, NodeKind, SopCover,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaInputValue {
    Zero,
    One,
    DontCare,
}

impl From<PlaInputValue> for CoverValue {
    fn from(value: PlaInputValue) -> Self {
        match value {
            PlaInputValue::Zero => Self::Zero,
            PlaInputValue::One => Self::One,
            PlaInputValue::DontCare => Self::DontCare,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaRow {
    pub inputs: Vec<PlaInputValue>,
    pub outputs: Vec<bool>,
}

impl PlaRow {
    pub fn new(inputs: impl Into<Vec<PlaInputValue>>, outputs: impl Into<Vec<bool>>) -> Self {
        Self {
            inputs: inputs.into(),
            outputs: outputs.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Pla {
    pub binary_valued: bool,
    pub input_labels: Vec<String>,
    pub output_labels: Vec<String>,
    pub on_set: Vec<PlaRow>,
    pub dc_set: Vec<PlaRow>,
}

impl Pla {
    pub fn new(
        input_labels: impl Into<Vec<String>>,
        output_labels: impl Into<Vec<String>>,
        on_set: impl Into<Vec<PlaRow>>,
        dc_set: impl Into<Vec<PlaRow>>,
    ) -> Self {
        Self {
            binary_valued: true,
            input_labels: input_labels.into(),
            output_labels: output_labels.into(),
            on_set: on_set.into(),
            dc_set: dc_set.into(),
        }
    }

    pub fn with_binary_valued(mut self, binary_valued: bool) -> Self {
        self.binary_valued = binary_valued;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlaToNetworkError {
    NonBinaryValued,
    InvalidRowInputCount {
        row: usize,
        expected: usize,
        actual: usize,
    },
    InvalidRowOutputCount {
        row: usize,
        expected: usize,
        actual: usize,
    },
    EmptyDontCareSet,
    Network(NetworkUtilError),
}

impl fmt::Display for PlaToNetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonBinaryValued => write!(formatter, "only binary-valued PLAs are supported"),
            Self::InvalidRowInputCount {
                row,
                expected,
                actual,
            } => write!(
                formatter,
                "PLA row {row} has {actual} input values but {expected} were expected"
            ),
            Self::InvalidRowOutputCount {
                row,
                expected,
                actual,
            } => write!(
                formatter,
                "PLA row {row} has {actual} output values but {expected} were expected"
            ),
            Self::EmptyDontCareSet => write!(formatter, "PLA don't-care set is empty"),
            Self::Network(error) => error.fmt(formatter),
        }
    }
}

impl Error for PlaToNetworkError {}

impl From<NetworkUtilError> for PlaToNetworkError {
    fn from(value: NetworkUtilError) -> Self {
        Self::Network(value)
    }
}

pub type PlaToNetworkResult<T> = Result<T, PlaToNetworkError>;

pub fn pla_to_network(pla: &Pla) -> PlaToNetworkResult<Network> {
    validate_pla(pla)?;

    let mut network = Network::new();
    let inputs = add_primary_inputs(&mut network, &pla.input_labels)?;
    let pterms = add_product_terms(&mut network, &inputs, &pla.on_set)?;

    for (output_index, output_label) in pla.output_labels.iter().enumerate() {
        let cover = output_cover_from_product_terms(&pla.on_set, output_index, pterms.len());
        let driver = network.add_internal("", pterms.clone(), cover)?;
        network.change_node_name(driver, output_label.clone())?;
        network.add_primary_output(driver)?;
    }

    Ok(network)
}

pub fn pla_to_network_single(pla: &Pla) -> PlaToNetworkResult<Network> {
    validate_pla(pla)?;
    network_from_rows(pla, &pla.on_set, false)
}

pub fn pla_to_dcnetwork_single(pla: &Pla) -> PlaToNetworkResult<Option<Network>> {
    validate_pla(pla)?;
    if pla.dc_set.is_empty() {
        return Ok(None);
    }

    Ok(Some(network_from_rows(pla, &pla.dc_set, true)?))
}

fn validate_pla(pla: &Pla) -> PlaToNetworkResult<()> {
    if !pla.binary_valued {
        return Err(PlaToNetworkError::NonBinaryValued);
    }

    validate_rows(&pla.on_set, pla.input_labels.len(), pla.output_labels.len())?;
    validate_rows(&pla.dc_set, pla.input_labels.len(), pla.output_labels.len())?;
    Ok(())
}

fn validate_rows(
    rows: &[PlaRow],
    input_count: usize,
    output_count: usize,
) -> PlaToNetworkResult<()> {
    for (row_index, row) in rows.iter().enumerate() {
        if row.inputs.len() != input_count {
            return Err(PlaToNetworkError::InvalidRowInputCount {
                row: row_index,
                expected: input_count,
                actual: row.inputs.len(),
            });
        }

        if row.outputs.len() != output_count {
            return Err(PlaToNetworkError::InvalidRowOutputCount {
                row: row_index,
                expected: output_count,
                actual: row.outputs.len(),
            });
        }
    }

    Ok(())
}

fn add_primary_inputs(network: &mut Network, labels: &[String]) -> PlaToNetworkResult<Vec<NodeId>> {
    labels
        .iter()
        .map(|label| {
            network
                .add_primary_input(NetworkNode::new(label.clone(), NodeKind::PrimaryInput))
                .map_err(PlaToNetworkError::from)
        })
        .collect()
}

fn add_product_terms(
    network: &mut Network,
    inputs: &[NodeId],
    rows: &[PlaRow],
) -> PlaToNetworkResult<Vec<NodeId>> {
    let mut pterms = Vec::with_capacity(rows.len());

    for row in rows {
        let cover = SopCover::new([cube_from_inputs(&row.inputs)]);
        let pterm = network.add_internal("", inputs.to_vec(), cover)?;
        pterms.push(pterm);
    }

    Ok(pterms)
}

fn network_from_rows(
    pla: &Pla,
    rows: &[PlaRow],
    empty_rows_are_error: bool,
) -> PlaToNetworkResult<Network> {
    if empty_rows_are_error && rows.is_empty() {
        return Err(PlaToNetworkError::EmptyDontCareSet);
    }

    let mut network = Network::new();
    let inputs = add_primary_inputs(&mut network, &pla.input_labels)?;

    for (output_index, output_label) in pla.output_labels.iter().enumerate() {
        let cover = output_cover_from_input_rows(rows, output_index);
        let driver = network.add_internal(output_label.clone(), inputs.clone(), cover)?;
        network.add_primary_output(driver)?;
    }

    Ok(network)
}

fn output_cover_from_product_terms(
    rows: &[PlaRow],
    output_index: usize,
    product_term_count: usize,
) -> SopCover {
    let cubes = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.outputs[output_index])
        .map(|(row_index, _)| {
            let mut values = vec![CoverValue::DontCare; product_term_count];
            values[row_index] = CoverValue::One;
            Cube::new(values)
        })
        .collect::<Vec<_>>();

    SopCover::new(cubes)
}

fn output_cover_from_input_rows(rows: &[PlaRow], output_index: usize) -> SopCover {
    let cubes = rows
        .iter()
        .filter(|row| row.outputs[output_index])
        .map(|row| cube_from_inputs(&row.inputs))
        .collect::<Vec<_>>();

    SopCover::new(cubes)
}

fn cube_from_inputs(inputs: &[PlaInputValue]) -> Cube {
    Cube::new(
        inputs
            .iter()
            .copied()
            .map(CoverValue::from)
            .collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pla() -> Pla {
        Pla::new(
            vec!["a".to_owned(), "b".to_owned()],
            vec!["y".to_owned(), "z".to_owned()],
            vec![
                PlaRow::new(
                    vec![PlaInputValue::One, PlaInputValue::Zero],
                    vec![true, false],
                ),
                PlaRow::new(
                    vec![PlaInputValue::DontCare, PlaInputValue::One],
                    vec![true, true],
                ),
            ],
            vec![PlaRow::new(
                vec![PlaInputValue::Zero, PlaInputValue::DontCare],
                vec![false, true],
            )],
        )
    }

    #[test]
    fn two_level_network_creates_inputs_product_terms_and_outputs() {
        let network = pla_to_network(&pla()).unwrap();

        assert_eq!(network.num_pi(), 2);
        assert_eq!(network.num_po(), 2);
        assert_eq!(network.num_internal(), 4);
        assert!(network.find_node("a").is_some());
        assert!(network.find_node("b").is_some());
        assert!(network.find_node("y").is_some());
        assert!(network.find_node("z").is_some());

        let y = network.find_node("y").unwrap();
        let y_driver = network.node(y).unwrap().fanins[0];
        let y_cover = network.node(y_driver).unwrap().cover.as_ref().unwrap();
        assert_eq!(y_cover.cubes().len(), 2);
        assert_eq!(
            y_cover.cubes()[0].values(),
            &[CoverValue::One, CoverValue::DontCare]
        );
        assert_eq!(
            y_cover.cubes()[1].values(),
            &[CoverValue::DontCare, CoverValue::One]
        );
    }

    #[test]
    fn single_level_network_copies_matching_on_set_rows_to_each_output() {
        let network = pla_to_network_single(&pla()).unwrap();

        assert_eq!(network.num_pi(), 2);
        assert_eq!(network.num_po(), 2);
        assert_eq!(network.num_internal(), 2);

        let y = network.find_node("y").unwrap();
        let y_driver = network.node(y).unwrap().fanins[0];
        let y_cover = network.node(y_driver).unwrap().cover.as_ref().unwrap();
        assert_eq!(y_cover.cubes().len(), 2);
        assert_eq!(
            y_cover.cubes()[0].values(),
            &[CoverValue::One, CoverValue::Zero]
        );
        assert_eq!(
            y_cover.cubes()[1].values(),
            &[CoverValue::DontCare, CoverValue::One]
        );
    }

    #[test]
    fn dc_network_uses_dc_rows_and_returns_none_for_empty_dc_set() {
        let network = pla_to_dcnetwork_single(&pla()).unwrap().unwrap();
        let z = network.find_node("z").unwrap();
        let z_driver = network.node(z).unwrap().fanins[0];
        let z_cover = network.node(z_driver).unwrap().cover.as_ref().unwrap();

        assert_eq!(z_cover.cubes().len(), 1);
        assert_eq!(
            z_cover.cubes()[0].values(),
            &[CoverValue::Zero, CoverValue::DontCare]
        );

        let mut without_dc = pla();
        without_dc.dc_set.clear();
        assert_eq!(pla_to_dcnetwork_single(&without_dc).unwrap(), None);
    }

    #[test]
    fn validates_row_shape_and_duplicate_labels() {
        assert_eq!(
            pla_to_network_single(&pla().with_binary_valued(false)).unwrap_err(),
            PlaToNetworkError::NonBinaryValued
        );

        let invalid = Pla::new(
            vec!["a".to_owned()],
            vec!["y".to_owned()],
            vec![PlaRow::new(
                vec![PlaInputValue::One, PlaInputValue::Zero],
                vec![true],
            )],
            Vec::new(),
        );
        assert_eq!(
            pla_to_network_single(&invalid).unwrap_err(),
            PlaToNetworkError::InvalidRowInputCount {
                row: 0,
                expected: 1,
                actual: 2,
            }
        );

        let duplicate = Pla::new(
            vec!["a".to_owned(), "a".to_owned()],
            vec!["y".to_owned()],
            vec![PlaRow::new(
                vec![PlaInputValue::One, PlaInputValue::Zero],
                vec![true],
            )],
            Vec::new(),
        );
        assert!(matches!(
            pla_to_network_single(&duplicate),
            Err(PlaToNetworkError::Network(NetworkUtilError::DuplicateName(
                _
            )))
        ));
    }
}
