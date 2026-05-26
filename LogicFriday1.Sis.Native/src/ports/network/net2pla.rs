//! Convert native SIS networks to a PLA-shaped cover.
//!
//! The legacy `net2pla.c` duplicates and collapses a network, sets Espresso's
//! global cube layout, then emits one product term per output cover row.  This
//! Rust port keeps that behavior as owned data: inputs and outputs are named in
//! network order, each PLA row carries one output bit, and an attached external
//! don't-care network becomes the optional DC cover.

use super::network_util::{
    BoolExpr, CoverValue, Network, NetworkUtilError, NetworkUtilResult, NodeId, NodeKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pla {
    pub input_labels: Vec<String>,
    pub output_labels: Vec<String>,
    pub on_set: Vec<PlaCube>,
    pub dc_set: Vec<PlaCube>,
}

impl Pla {
    pub fn has_dc_set(&self) -> bool {
        !self.dc_set.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaCube {
    pub inputs: Vec<CoverValue>,
    pub output: usize,
}

impl PlaCube {
    pub fn new(inputs: Vec<CoverValue>, output: usize) -> Self {
        Self { inputs, output }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Sop {
    cubes: Vec<Vec<CoverValue>>,
}

impl Sop {
    fn zero() -> Self {
        Self { cubes: Vec::new() }
    }

    fn one(input_count: usize) -> Self {
        Self {
            cubes: vec![vec![CoverValue::DontCare; input_count]],
        }
    }

    fn literal(input_count: usize, input_index: usize, phase: bool) -> Self {
        let mut cube = vec![CoverValue::DontCare; input_count];
        cube[input_index] = if phase {
            CoverValue::One
        } else {
            CoverValue::Zero
        };

        Self { cubes: vec![cube] }
    }

    fn or(mut self, mut other: Self) -> Self {
        self.cubes.append(&mut other.cubes);
        self
    }

    fn and(self, other: Self) -> Self {
        if self.cubes.is_empty() || other.cubes.is_empty() {
            return Self::zero();
        }

        let mut result = Vec::new();
        for left in &self.cubes {
            for right in &other.cubes {
                if let Some(cube) = merge_cubes(left, right) {
                    result.push(cube);
                }
            }
        }

        Self { cubes: result }
    }

    fn complement(&self, input_count: usize) -> Self {
        let mut result = Self::one(input_count);
        for cube in &self.cubes {
            result = result.and(complement_cube(cube, input_count));
            if result.cubes.is_empty() {
                break;
            }
        }

        result
    }
}

pub fn network_to_pla(network: &Network) -> NetworkUtilResult<Option<Pla>> {
    let input_count = network.num_pi();
    let output_count = network.num_po();
    if input_count == 0 || output_count == 0 {
        return Ok(None);
    }

    let input_ids = network.primary_inputs().to_vec();
    let output_ids = network.primary_outputs().to_vec();
    let input_labels = input_ids
        .iter()
        .map(|input| network.node(*input).map(|node| node.name.clone()))
        .collect::<NetworkUtilResult<Vec<_>>>()?;
    let output_labels = output_ids
        .iter()
        .map(|output| network.node(*output).map(|node| node.name.clone()))
        .collect::<NetworkUtilResult<Vec<_>>>()?;

    let mut converter = PlaConverter::new(network, input_ids);
    let mut on_set = Vec::new();
    for (output_index, output) in output_ids.iter().copied().enumerate() {
        let driver = primary_output_driver(network, output)?;
        let sop = converter.node_sop(driver)?;
        on_set.extend(
            sop.cubes
                .into_iter()
                .map(|cube| PlaCube::new(cube, output_index)),
        );
    }

    let mut dc_set = Vec::new();
    if network.dc_network().is_some() {
        let attachment = network.attach_dc_network()?;
        for (output_index, output) in output_ids.iter().copied().enumerate() {
            let expression = network.find_external_dc(output, &attachment)?;
            let sop = converter.expression_sop(&expression)?;
            dc_set.extend(
                sop.cubes
                    .into_iter()
                    .map(|cube| PlaCube::new(cube, output_index)),
            );
        }
    }

    Ok(Some(Pla {
        input_labels,
        output_labels,
        on_set,
        dc_set,
    }))
}

pub fn discard_pla(_pla: Option<Pla>) {}

struct PlaConverter<'a> {
    network: &'a Network,
    input_order: Vec<NodeId>,
}

impl<'a> PlaConverter<'a> {
    fn new(network: &'a Network, input_order: Vec<NodeId>) -> Self {
        Self {
            network,
            input_order,
        }
    }

    fn node_sop(&mut self, node: NodeId) -> NetworkUtilResult<Sop> {
        let network_node = self.network.node(node)?;
        match network_node.kind {
            NodeKind::PrimaryInput => {
                let input_index = self
                    .input_order
                    .iter()
                    .position(|input| *input == node)
                    .ok_or(NetworkUtilError::NodeBelongsToDifferentNetwork(node))?;
                Ok(Sop::literal(self.input_order.len(), input_index, true))
            }
            NodeKind::PrimaryOutput => self.node_sop(primary_output_driver(self.network, node)?),
            NodeKind::Internal | NodeKind::Unassigned => {
                if let Some(cover) = &network_node.cover {
                    let mut sum = Sop::zero();
                    for (cube_index, cube) in cover.cubes().iter().enumerate() {
                        if cube.values().len() != network_node.fanins.len() {
                            return Err(NetworkUtilError::InvalidCover {
                                node,
                                cube: cube_index,
                            });
                        }

                        let mut product = Sop::one(self.input_order.len());
                        for (fanin_index, value) in cube.values().iter().enumerate() {
                            let fanin = network_node.fanins[fanin_index];
                            let term = match value {
                                CoverValue::DontCare => continue,
                                CoverValue::One => self.node_sop(fanin)?,
                                CoverValue::Zero => {
                                    self.node_sop(fanin)?.complement(self.input_order.len())
                                }
                            };
                            product = product.and(term);
                        }
                        sum = sum.or(product);
                    }

                    return Ok(sum);
                }

                if let Some(expression) = &network_node.expression {
                    return self.expression_sop(expression);
                }

                Ok(Sop::zero())
            }
        }
    }

    fn expression_sop(&mut self, expression: &BoolExpr) -> NetworkUtilResult<Sop> {
        match expression {
            BoolExpr::Constant(false) => Ok(Sop::zero()),
            BoolExpr::Constant(true) => Ok(Sop::one(self.input_order.len())),
            BoolExpr::Literal { node, phase } => {
                let sop = self.node_sop(*node)?;
                if *phase {
                    Ok(sop)
                } else {
                    Ok(sop.complement(self.input_order.len()))
                }
            }
            BoolExpr::Not(inner) => Ok(self
                .expression_sop(inner)?
                .complement(self.input_order.len())),
            BoolExpr::And(items) => {
                let mut product = Sop::one(self.input_order.len());
                for item in items {
                    product = product.and(self.expression_sop(item)?);
                }
                Ok(product)
            }
            BoolExpr::Or(items) => {
                let mut sum = Sop::zero();
                for item in items {
                    sum = sum.or(self.expression_sop(item)?);
                }
                Ok(sum)
            }
        }
    }
}

fn primary_output_driver(network: &Network, output: NodeId) -> NetworkUtilResult<NodeId> {
    let output_node = network.node(output)?;
    if output_node.fanins.len() != 1 {
        return Err(NetworkUtilError::InvalidPrimaryOutput(output));
    }

    Ok(output_node.fanins[0])
}

fn merge_cubes(left: &[CoverValue], right: &[CoverValue]) -> Option<Vec<CoverValue>> {
    left.iter()
        .zip(right)
        .map(
            |(left_value, right_value)| match (*left_value, *right_value) {
                (CoverValue::DontCare, value) | (value, CoverValue::DontCare) => Some(value),
                (CoverValue::Zero, CoverValue::Zero) => Some(CoverValue::Zero),
                (CoverValue::One, CoverValue::One) => Some(CoverValue::One),
                (CoverValue::Zero, CoverValue::One) | (CoverValue::One, CoverValue::Zero) => None,
            },
        )
        .collect()
}

fn complement_cube(cube: &[CoverValue], input_count: usize) -> Sop {
    let mut result = Sop::zero();
    for (input_index, value) in cube.iter().enumerate() {
        let phase = match value {
            CoverValue::DontCare => continue,
            CoverValue::Zero => true,
            CoverValue::One => false,
        };
        result = result.or(Sop::literal(input_count, input_index, phase));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::super::network_util::{NetworkNode, SopCover};
    use super::*;

    fn cube(values: &[CoverValue]) -> super::super::network_util::Cube {
        super::super::network_util::Cube::new(values.to_vec())
    }

    #[test]
    fn returns_none_for_network_without_primary_io() {
        let network = Network::new();

        assert_eq!(network_to_pla(&network).unwrap(), None);
    }

    #[test]
    fn converts_single_output_cover_to_pla_rows() {
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
                SopCover::new([
                    cube(&[CoverValue::One, CoverValue::Zero]),
                    cube(&[CoverValue::DontCare, CoverValue::One]),
                ]),
            )
            .unwrap();
        network.add_primary_output(n).unwrap();

        let pla = network_to_pla(&network).unwrap().unwrap();

        assert_eq!(pla.input_labels, vec!["a", "b"]);
        assert_eq!(pla.output_labels, vec!["n"]);
        assert_eq!(
            pla.on_set,
            vec![
                PlaCube::new(vec![CoverValue::One, CoverValue::Zero], 0),
                PlaCube::new(vec![CoverValue::DontCare, CoverValue::One], 0),
            ]
        );
        assert!(pla.dc_set.is_empty());
    }

    #[test]
    fn treats_output_driven_by_primary_input_as_positive_literal() {
        let mut network = Network::new();
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        network.add_primary_output(a).unwrap();

        let pla = network_to_pla(&network).unwrap().unwrap();

        assert_eq!(pla.on_set, vec![PlaCube::new(vec![CoverValue::One], 0)]);
    }

    #[test]
    fn expands_nested_fanin_covers_to_primary_input_space() {
        let mut network = Network::new();
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let n = network
            .add_internal("n", vec![a], SopCover::new([cube(&[CoverValue::One])]))
            .unwrap();
        let y = network
            .add_internal(
                "y",
                vec![n, b],
                SopCover::new([cube(&[CoverValue::One, CoverValue::One])]),
            )
            .unwrap();
        network.add_primary_output(y).unwrap();

        let pla = network_to_pla(&network).unwrap().unwrap();

        assert_eq!(
            pla.on_set,
            vec![PlaCube::new(vec![CoverValue::One, CoverValue::One], 0)]
        );
    }

    #[test]
    fn includes_external_dc_rows_for_matching_outputs() {
        let mut care = Network::new();
        let a = care
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let n = care
            .add_internal("n", vec![a], SopCover::new([cube(&[CoverValue::One])]))
            .unwrap();
        let y = care.add_primary_output(n).unwrap();

        let mut dc = Network::new();
        let dc_a = dc
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let dc_n = dc
            .add_internal(
                "dc_n",
                vec![dc_a],
                SopCover::new([cube(&[CoverValue::Zero])]),
            )
            .unwrap();
        let dc_y = dc.add_primary_output(dc_n).unwrap();
        dc.change_node_name(dc_y, care.node(y).unwrap().name.clone())
            .unwrap();
        care.set_dc_network(Some(dc));

        let pla = network_to_pla(&care).unwrap().unwrap();

        assert_eq!(pla.on_set, vec![PlaCube::new(vec![CoverValue::One], 0)]);
        assert_eq!(pla.dc_set, vec![PlaCube::new(vec![CoverValue::Zero], 0)]);
        assert!(pla.has_dc_set());
    }

    #[test]
    fn rejects_malformed_cover_width() {
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
            network_to_pla(&network),
            Err(NetworkUtilError::InvalidCover { node, cube: 0 }) if node == n
        ));
    }
}
