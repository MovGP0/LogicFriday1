//! Native Rust model for `LogicSynthesis/sis/power/power_dynamic.c`.
//!
//! The C routine estimates dynamic-logic power by optionally refreshing
//! present-state probabilities for sequential networks, ordering primary inputs,
//! converting primary-output cones to BDDs, evaluating each internal node's
//! probability of being one, adding that probability into the node switching
//! probability, and accumulating `cap_factor * probability * CAPACITANCE *
//! 250.0`. This module keeps that behavior on owned Rust data. Direct SIS
//! `network_t`, `st_table`, `array_t`, and `ntbdd` integration is reported as
//! explicit dependency errors until the prerequisite ports are available.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const CAPACITANCE: f64 = 0.01;
pub const POWER_SCALE: f64 = 250.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.402",
        source_file: "LogicSynthesis/sis/power/power_psAppr.c",
        reason: "power_direct_PS_lines_prob refreshes present-state line probabilities",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.398",
        source_file: "LogicSynthesis/sis/power/power_comp.c",
        reason: "power_calc_func_prob evaluates BDD probability from PI probabilities",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.400",
        source_file: "LogicSynthesis/sis/power/power_main.c",
        reason: "allocates power_info_table and node_info_t probability records",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.329",
        source_file: "LogicSynthesis/sis/ntbdd/manager.c",
        reason: "ntbdd_start_manager and ntbdd_end_manager lifetime",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.330",
        source_file: "LogicSynthesis/sis/ntbdd/node_to_bdd.c",
        reason: "ntbdd_node_to_bdd and ntbdd_at_node provide per-node BDDs",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.442",
        source_file: "LogicSynthesis/sis/seqbdd/verif_util.c",
        reason: "order_nodes supplies the PI order used by the BDD manager",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.299",
        source_file: "LogicSynthesis/sis/network/net_seq.c",
        reason: "network_num_latch detects dynamic sequential circuits",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "primary-input, primary-output, and node iteration over network_t",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node function classification and node names",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "info_table, leaves, and power_info_table are st_table instances",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        reason: "poArray, piOrder, and psOrder are legacy array_t values",
    },
];

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

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
    ) -> Result<f64, PowerDynamicError> {
        match self {
            Self::Constant(value) => Ok(if *value { 1.0 } else { 0.0 }),
            Self::Input(node) => input_probabilities
                .get(node)
                .copied()
                .ok_or(PowerDynamicError::MissingInputProbability(*node)),
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
                    .ok_or(PowerDynamicError::MissingInputProbability(*variable))?;
                Ok(
                    variable_probability * when_one.probability_one(input_probabilities)?
                        + (1.0 - variable_probability)
                            * when_zero.probability_one(input_probabilities)?,
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub probability_one: Option<f64>,
    pub function: Option<BoolExpr>,
    pub cap_factor: f64,
    pub switching_prob: f64,
}

impl DynamicNode {
    pub fn primary_input(id: usize, name: impl Into<String>, probability_one: f64) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind: NodeKind::PrimaryInput,
            probability_one: Some(probability_one),
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
            probability_one: None,
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
            probability_one: None,
            function: Some(function),
            cap_factor,
            switching_prob: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicNodeContribution {
    pub node: NodeId,
    pub node_name: String,
    pub probability_one: f64,
    pub cap_factor: f64,
    pub unscaled_power: f64,
    pub scaled_power: f64,
    pub switching_prob_after: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicPowerReport {
    pub sequential_probability_refresh: bool,
    pub primary_input_order: Vec<NodeId>,
    pub bdd_output_roots: Vec<NodeId>,
    pub contributions: Vec<DynamicNodeContribution>,
    pub total_power: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicPowerModel {
    pub nodes: Vec<DynamicNode>,
    pub primary_input_order: Vec<NodeId>,
    pub primary_output_order: Vec<NodeId>,
    pub latch_count: usize,
}

impl DynamicPowerModel {
    pub fn new(nodes: Vec<DynamicNode>) -> Result<Self, PowerDynamicError> {
        let mut seen = HashMap::with_capacity(nodes.len());
        for (position, node) in nodes.iter().enumerate() {
            if seen.insert(node.id, position).is_some() {
                return Err(PowerDynamicError::DuplicateNode(node.id));
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
            primary_input_order,
            primary_output_order,
            latch_count: 0,
        })
    }

    pub fn with_latch_count(mut self, latch_count: usize) -> Self {
        self.latch_count = latch_count;
        self
    }

    pub fn evaluate(&mut self) -> Result<DynamicPowerReport, PowerDynamicError> {
        let mut input_probabilities = HashMap::with_capacity(self.primary_input_order.len());
        for input in &self.primary_input_order {
            let node = self.node(*input)?;
            if node.kind != NodeKind::PrimaryInput {
                return Err(PowerDynamicError::UnexpectedNodeKind {
                    node: *input,
                    expected: NodeKind::PrimaryInput,
                    actual: node.kind,
                });
            }
            let probability = node
                .probability_one
                .ok_or(PowerDynamicError::MissingInputProbability(*input))?;
            validate_probability(probability)?;
            input_probabilities.insert(*input, probability);
        }

        for output in &self.primary_output_order {
            let node = self.node(*output)?;
            if node.kind != NodeKind::PrimaryOutput {
                return Err(PowerDynamicError::UnexpectedNodeKind {
                    node: *output,
                    expected: NodeKind::PrimaryOutput,
                    actual: node.kind,
                });
            }
            if node.function.is_none() {
                return Err(PowerDynamicError::MissingNodeFunction(*output));
            }
        }

        let mut contributions = Vec::new();
        for node in &mut self.nodes {
            if node.kind != NodeKind::Internal {
                continue;
            }

            let function = node
                .function
                .as_ref()
                .ok_or(PowerDynamicError::MissingNodeFunction(node.id))?;
            let probability_one = function.probability_one(&input_probabilities)?;
            validate_probability(probability_one)?;
            let unscaled_power = node.cap_factor * probability_one * CAPACITANCE;
            node.switching_prob += probability_one;
            contributions.push(DynamicNodeContribution {
                node: node.id,
                node_name: node.name.clone(),
                probability_one,
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

        Ok(DynamicPowerReport {
            sequential_probability_refresh: self.latch_count > 0,
            primary_input_order: self.primary_input_order.clone(),
            bdd_output_roots: self.primary_output_order.clone(),
            contributions,
            total_power,
        })
    }

    pub fn node(&self, id: NodeId) -> Result<&DynamicNode, PowerDynamicError> {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .ok_or(PowerDynamicError::MissingNode(id))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PowerDynamicError {
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    DuplicateNode(NodeId),
    MissingNode(NodeId),
    MissingInputProbability(NodeId),
    MissingNodeFunction(NodeId),
    ProbabilityOutOfRange(f64),
    UnexpectedNodeKind {
        node: NodeId,
        expected: NodeKind,
        actual: NodeKind,
    },
}

impl fmt::Display for PowerDynamicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => {
                write!(f, "{operation} requires native SIS prerequisite ports: ")?;
                for (index, dependency) in dependencies.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} ({})", dependency.bead_id, dependency.source_file)?;
                }
                Ok(())
            }
            Self::DuplicateNode(node) => {
                write!(f, "dynamic power model has duplicate node {:?}", node)
            }
            Self::MissingNode(node) => write!(f, "dynamic power model is missing node {:?}", node),
            Self::MissingInputProbability(node) => {
                write!(
                    f,
                    "missing probability-one value for primary input {:?}",
                    node
                )
            }
            Self::MissingNodeFunction(node) => {
                write!(f, "missing BDD/function for dynamic power node {:?}", node)
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

impl Error for PowerDynamicError {}

pub fn evaluate_sis_dynamic_power<Network>(
    _network: &Network,
) -> Result<DynamicPowerReport, PowerDynamicError> {
    Err(PowerDynamicError::MissingNativePorts {
        operation: "SIS power_dynamic",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

fn validate_probability(probability: f64) -> Result<(), PowerDynamicError> {
    if (0.0..=1.0).contains(&probability) {
        Ok(())
    } else {
        Err(PowerDynamicError::ProbabilityOutOfRange(probability))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(id: usize, probability: f64) -> DynamicNode {
        DynamicNode::primary_input(id, format!("pi{id}"), probability)
    }

    #[test]
    fn constants_match_the_c_dynamic_power_scaling() {
        assert_eq!(CAPACITANCE, 0.01);
        assert_eq!(POWER_SCALE, 250.0);
    }

    #[test]
    fn evaluates_internal_nodes_and_updates_switching_probability() {
        let mut model = DynamicPowerModel::new(vec![
            input(0, 0.25),
            input(1, 0.5),
            DynamicNode::internal(
                2,
                "and_gate",
                BoolExpr::And(vec![BoolExpr::Input(NodeId(0)), BoolExpr::Input(NodeId(1))]),
                4.0,
            ),
            DynamicNode::internal(
                3,
                "xor_gate",
                BoolExpr::Xor(
                    Box::new(BoolExpr::Input(NodeId(0))),
                    Box::new(BoolExpr::Input(NodeId(1))),
                ),
                2.0,
            ),
            DynamicNode::primary_output(4, "out", BoolExpr::Input(NodeId(3))),
        ])
        .unwrap();

        let report = model.evaluate().unwrap();

        assert_eq!(report.primary_input_order, vec![NodeId(0), NodeId(1)]);
        assert_eq!(report.bdd_output_roots, vec![NodeId(4)]);
        assert_eq!(report.contributions[0].probability_one, 0.125);
        assert_eq!(report.contributions[0].scaled_power, 1.25);
        assert_eq!(report.contributions[0].switching_prob_after, 0.125);
        assert_eq!(report.contributions[1].probability_one, 0.5);
        assert_eq!(report.contributions[1].scaled_power, 2.5);
        assert_eq!(report.total_power, 3.75);
        assert_eq!(model.node(NodeId(3)).unwrap().switching_prob, 0.5);
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
    fn primary_outputs_are_converted_but_not_charged_as_internal_nodes() {
        let mut model = DynamicPowerModel::new(vec![
            input(0, 0.5),
            DynamicNode::primary_output(1, "out", BoolExpr::Input(NodeId(0))),
        ])
        .unwrap();

        let report = model.evaluate().unwrap();

        assert!(report.contributions.is_empty());
        assert_eq!(report.total_power, 0.0);
        assert_eq!(report.bdd_output_roots, vec![NodeId(1)]);
    }

    #[test]
    fn sequential_networks_record_the_present_state_probability_refresh() {
        let mut model = DynamicPowerModel::new(vec![input(0, 0.5)])
            .unwrap()
            .with_latch_count(1);

        let report = model.evaluate().unwrap();

        assert!(report.sequential_probability_refresh);
    }

    #[test]
    fn invalid_or_missing_probabilities_are_reported() {
        let mut model = DynamicPowerModel::new(vec![input(0, 1.5)]).unwrap();

        assert_eq!(
            model.evaluate().unwrap_err(),
            PowerDynamicError::ProbabilityOutOfRange(1.5)
        );

        let expr = BoolExpr::Input(NodeId(99));
        let probabilities = HashMap::new();
        assert_eq!(
            expr.probability_one(&probabilities).unwrap_err(),
            PowerDynamicError::MissingInputProbability(NodeId(99))
        );
    }

    #[test]
    fn sis_entry_reports_dependency_beads_and_sources() {
        let error = evaluate_sis_dynamic_power(&()).unwrap_err();
        let PowerDynamicError::MissingNativePorts {
            operation,
            dependencies,
        } = error
        else {
            panic!("expected missing native port error");
        };

        assert_eq!(operation, "SIS power_dynamic");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.402"
                && dependency.source_file == "LogicSynthesis/sis/power/power_psAppr.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.398"
                && dependency.source_file == "LogicSynthesis/sis/power/power_comp.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.330"
                && dependency.source_file == "LogicSynthesis/sis/ntbdd/node_to_bdd.c"
        }));
        assert!(format!("{error}").contains("LogicFriday1-8j8.2.6.485"));
    }

    #[test]
    fn no_legacy_abi_tokens_are_present_in_this_port() {
        let source = include_str!("power_dynamic.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
