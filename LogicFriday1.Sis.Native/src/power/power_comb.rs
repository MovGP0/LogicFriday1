//! Native Rust model for `LogicSynthesis/sis/power/power_comb.c`.
//!
//! The C file estimates static combinational power in two modes. Zero-delay
//! mode evaluates each non-PO node's probability of being one, converts that to
//! a transition probability `2p(1-p)`, adds it to the node switching
//! probability, and accumulates `cap_factor * transition * CAPACITANCE *
//! 250.0`. Arbitrary-delay mode consumes the symbolic transition network built
//! by `power_symbolic_simulate`, evaluates each symbolic PO cone probability,
//! and charges the original node mapped from that symbolic transition. Direct
//! SIS `network_t`, `st_table`, `array_t`, and `ntbdd` integration is reported
//! as explicit dependency errors until the prerequisite ports are available.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const CAPACITANCE: f64 = 0.01;
pub const POWER_SCALE: f64 = 250.0;
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BoolExpr {
    Constant(bool),
    Input(NodeId),
    Not(Box<BoolExpr>),
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
    Xor(Box<BoolExpr>, Box<BoolExpr>),
    Ite {
        variable: NodeId,
        when_one: Box<BoolExpr>,
        when_zero: Box<BoolExpr>,
    },
}

impl BoolExpr {
    pub fn probability_one(
        &self,
        input_probabilities: &HashMap<NodeId, f64>,
    ) -> Result<f64, PowerCombError> {
        match self {
            Self::Constant(value) => Ok(if *value { 1.0 } else { 0.0 }),
            Self::Input(node) => input_probabilities
                .get(node)
                .copied()
                .ok_or(PowerCombError::MissingInputProbability(*node)),
            Self::Not(expr) => Ok(1.0 - expr.probability_one(input_probabilities)?),
            Self::And(terms) => terms.iter().try_fold(1.0, |acc, term| {
                Ok(acc * term.probability_one(input_probabilities)?)
            }),
            Self::Or(terms) => {
                let probability_zero = terms.iter().try_fold(1.0, |acc, term| {
                    Ok(acc * (1.0 - term.probability_one(input_probabilities)?))
                })?;
                Ok(1.0 - probability_zero)
            }
            Self::Xor(left, right) => {
                let left = left.probability_one(input_probabilities)?;
                let right = right.probability_one(input_probabilities)?;
                Ok(left * (1.0 - right) + (1.0 - left) * right)
            }
            Self::Ite {
                variable,
                when_one,
                when_zero,
            } => {
                let variable_probability = input_probabilities
                    .get(variable)
                    .copied()
                    .ok_or(PowerCombError::MissingInputProbability(*variable))?;
                Ok(
                    variable_probability * when_one.probability_one(input_probabilities)?
                        + (1.0 - variable_probability)
                            * when_zero.probability_one(input_probabilities)?,
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeInfo {
    pub probability_one: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub function: Option<BoolExpr>,
    pub cap_factor: f64,
    pub switching_prob: f64,
}

impl CombNode {
    pub fn primary_input(id: usize, name: impl Into<String>) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind: NodeKind::PrimaryInput,
            function: None,
            cap_factor: 0.0,
            switching_prob: 0.0,
        }
    }

    pub fn primary_output(id: usize, name: impl Into<String>, function: BoolExpr) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind: NodeKind::PrimaryOutput,
            function: Some(function),
            cap_factor: 0.0,
            switching_prob: 0.0,
        }
    }

    pub fn internal(
        id: usize,
        name: impl Into<String>,
        function: BoolExpr,
        cap_factor: f64,
    ) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind: NodeKind::Internal,
            function: Some(function),
            cap_factor,
            switching_prob: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombPowerModel {
    pub nodes: Vec<CombNode>,
    pub node_info: HashMap<NodeId, NodeInfo>,
    pub primary_input_order: Vec<NodeId>,
    pub primary_output_order: Vec<NodeId>,
}

impl CombPowerModel {
    pub fn new(
        nodes: Vec<CombNode>,
        node_info: impl IntoIterator<Item = (NodeId, NodeInfo)>,
    ) -> Result<Self, PowerCombError> {
        let mut seen = HashMap::with_capacity(nodes.len());
        for (position, node) in nodes.iter().enumerate() {
            if seen.insert(node.id, position).is_some() {
                return Err(PowerCombError::DuplicateNode(node.id));
            }
        }

        let primary_input_order = nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryInput)
            .map(|node| node.id)
            .collect();
        let primary_output_order = nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryOutput)
            .map(|node| node.id)
            .collect();

        Ok(Self {
            nodes,
            node_info: node_info.into_iter().collect(),
            primary_input_order,
            primary_output_order,
        })
    }

    pub fn evaluate_static_zero(&mut self) -> Result<CombPowerReport, PowerCombError> {
        let mut input_probabilities = HashMap::with_capacity(self.primary_input_order.len());
        for input in &self.primary_input_order {
            let node = self.node(*input)?;
            if node.kind != NodeKind::PrimaryInput {
                return Err(PowerCombError::UnexpectedNodeKind {
                    node: *input,
                    expected: NodeKind::PrimaryInput,
                    actual: node.kind,
                });
            }
            let probability = self
                .node_info
                .get(input)
                .ok_or(PowerCombError::MissingNodeInfo { node: *input })?
                .probability_one;
            validate_probability(probability)?;
            input_probabilities.insert(*input, probability);
        }

        for output in &self.primary_output_order {
            let node = self.node(*output)?;
            if node.kind != NodeKind::PrimaryOutput {
                return Err(PowerCombError::UnexpectedNodeKind {
                    node: *output,
                    expected: NodeKind::PrimaryOutput,
                    actual: node.kind,
                });
            }
            if node.function.is_none() {
                return Err(PowerCombError::MissingNodeFunction(*output));
            }
        }

        let mut contributions = Vec::new();
        for node in &mut self.nodes {
            if node.kind == NodeKind::PrimaryOutput {
                continue;
            }

            let probability_one = match node.kind {
                NodeKind::PrimaryInput => {
                    self.node_info
                        .get(&node.id)
                        .ok_or(PowerCombError::MissingNodeInfo { node: node.id })?
                        .probability_one
                }
                NodeKind::Internal => node
                    .function
                    .as_ref()
                    .ok_or(PowerCombError::MissingNodeFunction(node.id))?
                    .probability_one(&input_probabilities)?,
                NodeKind::PrimaryOutput => unreachable!(),
            };
            validate_probability(probability_one)?;

            let transition_probability = zero_delay_transition_probability(probability_one);
            let unscaled_power = node.cap_factor * transition_probability * CAPACITANCE;
            node.switching_prob += transition_probability;
            contributions.push(CombNodeContribution {
                node: node.id,
                node_name: node.name.clone(),
                probability_one,
                switching_probability: transition_probability,
                cap_factor: node.cap_factor,
                unscaled_power,
                scaled_power: unscaled_power * POWER_SCALE,
                switching_prob_after: node.switching_prob,
            });
        }

        let total_power = contributions
            .iter()
            .map(|contribution| contribution.scaled_power)
            .sum();

        Ok(CombPowerReport {
            mode: CombDelayMode::Zero,
            primary_input_order: self.primary_input_order.clone(),
            primary_output_order: self.primary_output_order.clone(),
            contributions,
            total_power,
        })
    }

    pub fn node(&self, id: NodeId) -> Result<&CombNode, PowerCombError> {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .ok_or(PowerCombError::MissingNode(id))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombDelayMode {
    Zero,
    Arbitrary,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombNodeContribution {
    pub node: NodeId,
    pub node_name: String,
    pub probability_one: f64,
    pub switching_probability: f64,
    pub cap_factor: f64,
    pub unscaled_power: f64,
    pub scaled_power: f64,
    pub switching_prob_after: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombPowerReport {
    pub mode: CombDelayMode,
    pub primary_input_order: Vec<NodeId>,
    pub primary_output_order: Vec<NodeId>,
    pub contributions: Vec<CombNodeContribution>,
    pub total_power: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolicTransitionOutput {
    pub output: NodeId,
    pub original_node: NodeId,
    pub original_name: String,
    pub function: BoolExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolicCombPowerModel {
    pub input_probabilities: HashMap<NodeId, f64>,
    pub power_info: HashMap<NodeId, PowerInfo>,
    pub outputs: Vec<SymbolicTransitionOutput>,
}

impl SymbolicCombPowerModel {
    pub fn evaluate_static_arbitrary(&mut self) -> Result<CombPowerReport, PowerCombError> {
        for probability in self.input_probabilities.values().copied() {
            validate_probability(probability)?;
        }

        let mut contributions = Vec::new();
        for output in &self.outputs {
            let probability_one = output.function.probability_one(&self.input_probabilities)?;
            validate_probability(probability_one)?;
            let power_info = self.power_info.get_mut(&output.original_node).ok_or(
                PowerCombError::MissingPowerInfo {
                    node: output.original_node,
                },
            )?;
            let unscaled_power = power_info.cap_factor * probability_one * CAPACITANCE;
            power_info.switching_prob += probability_one;
            contributions.push(CombNodeContribution {
                node: output.original_node,
                node_name: output.original_name.clone(),
                probability_one,
                switching_probability: probability_one,
                cap_factor: power_info.cap_factor,
                unscaled_power,
                scaled_power: unscaled_power * POWER_SCALE,
                switching_prob_after: power_info.switching_prob,
            });
        }

        let total_power = contributions
            .iter()
            .map(|contribution| contribution.scaled_power)
            .sum();

        Ok(CombPowerReport {
            mode: CombDelayMode::Arbitrary,
            primary_input_order: sorted_node_ids(&self.input_probabilities),
            primary_output_order: self.outputs.iter().map(|output| output.output).collect(),
            contributions,
            total_power,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerInfo {
    pub cap_factor: f64,
    pub switching_prob: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PowerCombError {
    MissingNativePorts {
        operation: &'static str,
    },
    DuplicateNode(NodeId),
    MissingNode(NodeId),
    MissingNodeInfo {
        node: NodeId,
    },
    MissingPowerInfo {
        node: NodeId,
    },
    MissingInputProbability(NodeId),
    MissingNodeFunction(NodeId),
    ProbabilityOutOfRange(f64),
    UnexpectedNodeKind {
        node: NodeId,
        expected: NodeKind,
        actual: NodeKind,
    },
}

impl fmt::Display for PowerCombError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
            Self::DuplicateNode(node) => {
                write!(f, "combinational power model has duplicate node {:?}", node)
            }
            Self::MissingNode(node) => {
                write!(f, "combinational power model is missing node {:?}", node)
            }
            Self::MissingNodeInfo { node, .. } => write!(
                f,
                "node {:?} is missing power node_info_t probability data from the SIS info table",
                node
            ),
            Self::MissingPowerInfo { node, .. } => write!(
                f,
                "node {:?} is missing power_info_t capacitance/switching data",
                node
            ),
            Self::MissingInputProbability(node) => {
                write!(f, "missing probability-one value for input {:?}", node)
            }
            Self::MissingNodeFunction(node) => {
                write!(
                    f,
                    "missing BDD/function for combinational power node {:?}",
                    node
                )
            }
            Self::ProbabilityOutOfRange(probability) => {
                write!(f, "probability {probability} is outside 0.0..=1.0")
            }
            Self::UnexpectedNodeKind {
                node,
                expected,
                actual,
            } => write!(
                f,
                "node {:?} has kind {:?}, expected {:?}",
                node, actual, expected
            ),
        }
    }
}

impl Error for PowerCombError {}

pub fn zero_delay_transition_probability(probability_one: f64) -> f64 {
    2.0 * probability_one * (1.0 - probability_one)
}

pub fn power_comb_static_zero_from_sis_network<Network, InfoTable>(
    _network: &Network,
    _info_table: &InfoTable,
) -> Result<CombPowerReport, PowerCombError> {
    Err(PowerCombError::MissingNativePorts {
        operation: "power_comb_static_zero",
    })
}

pub fn power_comb_static_arbitrary_from_sis_network<Network, InfoTable>(
    _network: &Network,
    _info_table: &InfoTable,
) -> Result<CombPowerReport, PowerCombError> {
    Err(PowerCombError::MissingNativePorts {
        operation: "power_comb_static_arbit",
    })
}

fn validate_probability(probability: f64) -> Result<(), PowerCombError> {
    if (0.0..=1.0).contains(&probability) {
        Ok(())
    } else {
        Err(PowerCombError::ProbabilityOutOfRange(probability))
    }
}

fn sorted_node_ids(map: &HashMap<NodeId, f64>) -> Vec<NodeId> {
    let mut ids: Vec<_> = map.keys().copied().collect();
    ids.sort();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(id: usize, probability_one: f64) -> (CombNode, (NodeId, NodeInfo)) {
        (
            CombNode::primary_input(id, format!("pi{id}")),
            (NodeId(id), NodeInfo { probability_one }),
        )
    }

    #[test]
    fn constants_match_the_c_comb_power_scaling() {
        assert_eq!(CAPACITANCE, 0.01);
        assert_eq!(POWER_SCALE, 250.0);
    }

    #[test]
    fn zero_delay_probability_uses_static_transition_formula() {
        assert_eq!(zero_delay_transition_probability(0.0), 0.0);
        assert_eq!(zero_delay_transition_probability(0.5), 0.5);
        assert_eq!(zero_delay_transition_probability(1.0), 0.0);
    }

    #[test]
    fn zero_delay_charges_primary_inputs_and_internal_nodes_but_skips_outputs() {
        let (a, a_info) = input(0, 0.25);
        let (b, b_info) = input(1, 0.5);
        let mut model = CombPowerModel::new(
            vec![
                CombNode {
                    cap_factor: 1.5,
                    ..a
                },
                b,
                CombNode::internal(
                    2,
                    "and_gate",
                    BoolExpr::And(vec![BoolExpr::Input(NodeId(0)), BoolExpr::Input(NodeId(1))]),
                    4.0,
                ),
                CombNode::primary_output(3, "out", BoolExpr::Input(NodeId(2))),
            ],
            [a_info, b_info],
        )
        .unwrap();

        let report = model.evaluate_static_zero().unwrap();

        assert_eq!(report.mode, CombDelayMode::Zero);
        assert_eq!(report.primary_input_order, vec![NodeId(0), NodeId(1)]);
        assert_eq!(report.primary_output_order, vec![NodeId(3)]);
        assert_eq!(report.contributions.len(), 3);
        assert_eq!(report.contributions[0].probability_one, 0.25);
        assert_eq!(report.contributions[0].switching_probability, 0.375);
        assert_eq!(report.contributions[0].scaled_power, 1.40625);
        assert_eq!(report.contributions[2].probability_one, 0.125);
        assert_eq!(report.contributions[2].switching_probability, 0.21875);
        assert_eq!(report.contributions[2].scaled_power, 2.1875);
        assert_eq!(report.total_power, 3.59375);
        assert_eq!(model.node(NodeId(2)).unwrap().switching_prob, 0.21875);
    }

    #[test]
    fn arbitrary_delay_uses_symbolic_transition_probability_without_zero_delay_formula() {
        let mut model = SymbolicCombPowerModel {
            input_probabilities: HashMap::from([(NodeId(0), 0.25), (NodeId(1), 0.5)]),
            power_info: HashMap::from([(
                NodeId(10),
                PowerInfo {
                    cap_factor: 8.0,
                    switching_prob: 0.1,
                },
            )]),
            outputs: vec![SymbolicTransitionOutput {
                output: NodeId(100),
                original_node: NodeId(10),
                original_name: "orig".to_owned(),
                function: BoolExpr::Xor(
                    Box::new(BoolExpr::Input(NodeId(0))),
                    Box::new(BoolExpr::Input(NodeId(1))),
                ),
            }],
        };

        let report = model.evaluate_static_arbitrary().unwrap();

        assert_eq!(report.mode, CombDelayMode::Arbitrary);
        assert_eq!(report.primary_input_order, vec![NodeId(0), NodeId(1)]);
        assert_eq!(report.primary_output_order, vec![NodeId(100)]);
        assert_eq!(report.contributions[0].probability_one, 0.5);
        assert_eq!(report.contributions[0].switching_probability, 0.5);
        assert_eq!(report.contributions[0].scaled_power, 10.0);
        assert_eq!(
            model.power_info.get(&NodeId(10)).unwrap().switching_prob,
            0.6
        );
    }

    #[test]
    fn ite_expression_models_bdd_probability_with_shared_variables() {
        let expr = BoolExpr::Ite {
            variable: NodeId(0),
            when_one: Box::new(BoolExpr::Input(NodeId(1))),
            when_zero: Box::new(BoolExpr::Not(Box::new(BoolExpr::Input(NodeId(1))))),
        };
        let probabilities = HashMap::from([(NodeId(0), 0.25), (NodeId(1), 0.8)]);

        let probability = expr.probability_one(&probabilities).unwrap();

        assert!((probability - 0.35).abs() < f64::EPSILON);
    }

    #[test]
    fn sis_bound_entries_report_explicit_missing_dependencies() {
        let zero_error = power_comb_static_zero_from_sis_network(&(), &()).unwrap_err();
        let arbitrary_error = power_comb_static_arbitrary_from_sis_network(&(), &()).unwrap_err();

        assert_eq!(
            zero_error,
            PowerCombError::MissingNativePorts {
                operation: "power_comb_static_zero",
            }
        );
        assert_eq!(
            arbitrary_error,
            PowerCombError::MissingNativePorts {
                operation: "power_comb_static_arbit",
            }
        );
        assert!(
            zero_error
                .to_string()
                .contains("requires native SIS prerequisite ports")
        );
        assert!(
            arbitrary_error
                .to_string()
                .contains("requires native SIS prerequisite ports")
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("power_comb.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
