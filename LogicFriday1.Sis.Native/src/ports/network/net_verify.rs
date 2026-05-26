//! Native network equivalence verification for the SIS network layer.
//!
//! The legacy implementation duplicated both networks, checked primary-output
//! name correspondence, then compared each collapsed output function. This port
//! keeps the same externally visible decision points on owned Rust networks:
//! output-name mismatches are reported before functional comparison, external
//! don't-care networks must either be present on both sides or neither side,
//! and don't-care networks are verified before comparing care networks ORed
//! with their don't-care functions.

use super::network_util::{
    BoolExpr, CoverValue, Network, NetworkUtilError, NetworkUtilResult, NodeId, NodeKind,
};

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationMethod {
    Collapse,
    Bdd,
}

impl TryFrom<i32> for VerificationMethod {
    type Error = NetworkVerifyError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Collapse),
            1 => Ok(Self::Bdd),
            _ => Err(NetworkVerifyError::UnknownMethod(value)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub equivalent: bool,
    pub diagnostics: Vec<String>,
}

impl VerificationReport {
    pub fn equivalent() -> Self {
        Self {
            equivalent: true,
            diagnostics: Vec::new(),
        }
    }

    pub fn not_equivalent(diagnostic: impl Into<String>) -> Self {
        Self {
            equivalent: false,
            diagnostics: vec![diagnostic.into()],
        }
    }

    pub fn with_diagnostics(diagnostics: Vec<String>) -> Self {
        Self {
            equivalent: diagnostics.is_empty(),
            diagnostics,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkVerifyError {
    UnknownMethod(i32),
    UnsupportedBddMethod,
    Network(NetworkUtilError),
}

impl fmt::Display for NetworkVerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMethod(method) => {
                write!(formatter, "unknown verification method {method}")
            }
            Self::UnsupportedBddMethod => {
                write!(
                    formatter,
                    "BDD verification requires the native NTBDD verifier port"
                )
            }
            Self::Network(error) => error.fmt(formatter),
        }
    }
}

impl Error for NetworkVerifyError {}

impl From<NetworkUtilError> for NetworkVerifyError {
    fn from(value: NetworkUtilError) -> Self {
        Self::Network(value)
    }
}

pub type NetworkVerifyResult<T> = Result<T, NetworkVerifyError>;

pub fn net_verify_with_dc(
    network1: &Network,
    network2: &Network,
    method: VerificationMethod,
) -> NetworkVerifyResult<VerificationReport> {
    match (network1.dc_network(), network2.dc_network()) {
        (None, None) => network_verify(network1, network2, method),
        (Some(dc1), Some(dc2)) => {
            let dc_report = network_verify(dc1, dc2, method)?;
            if !dc_report.equivalent {
                let mut diagnostics = dc_report.diagnostics;
                diagnostics.push("External don't care networks are not equal.".to_string());
                return Ok(VerificationReport::with_diagnostics(diagnostics));
            }

            let net1 = network1.or_with_dc_network()?;
            let net2 = network2.or_with_dc_network()?;
            network_verify(&net1, &net2, method)
        }
        _ => Ok(VerificationReport::not_equivalent(
            "External don't care networks are not equal.",
        )),
    }
}

pub fn net_verify_with_dc_legacy_method(
    network1: &Network,
    network2: &Network,
    method: i32,
) -> NetworkVerifyResult<VerificationReport> {
    net_verify_with_dc(network1, network2, VerificationMethod::try_from(method)?)
}

pub fn network_verify(
    network1: &Network,
    network2: &Network,
    method: VerificationMethod,
) -> NetworkVerifyResult<VerificationReport> {
    let output_diagnostics = primary_output_name_diagnostics(network1, network2)?;
    if !output_diagnostics.is_empty() {
        return Ok(VerificationReport::with_diagnostics(output_diagnostics));
    }

    match method {
        VerificationMethod::Collapse => verify_by_collapse(network1, network2),
        VerificationMethod::Bdd => Err(NetworkVerifyError::UnsupportedBddMethod),
    }
}

pub fn network_verify_legacy_method(
    network1: &Network,
    network2: &Network,
    method: i32,
) -> NetworkVerifyResult<VerificationReport> {
    network_verify(network1, network2, VerificationMethod::try_from(method)?)
}

pub fn verify_by_collapse(
    network1: &Network,
    network2: &Network,
) -> NetworkVerifyResult<VerificationReport> {
    let input_names = primary_input_names(network1, network2)?;
    let output_names = primary_output_names(network1)?;

    for output_name in output_names {
        let output1 = network1
            .find_node(&output_name)
            .expect("primary output names were validated before collapse verification");
        let output2 = network2
            .find_node(&output_name)
            .expect("primary output names were validated before collapse verification");

        if !outputs_equal(network1, output1, network2, output2, &input_names)? {
            return Ok(VerificationReport::not_equivalent(format!(
                "Networks differ on (at least) primary output {output_name}"
            )));
        }
    }

    Ok(VerificationReport::equivalent())
}

fn primary_output_name_diagnostics(
    network1: &Network,
    network2: &Network,
) -> NetworkVerifyResult<Vec<String>> {
    let mut diagnostics = Vec::new();

    for output in network1.primary_outputs() {
        let output_node = network1.node(*output)?;
        match network2.find_node(&output_node.name) {
            Some(candidate) if network2.node(candidate)?.kind == NodeKind::PrimaryOutput => {}
            _ => diagnostics.push(format!(
                "output '{}' only in network '{}'",
                output_node.name,
                network1.name()
            )),
        }
    }

    for output in network2.primary_outputs() {
        let output_node = network2.node(*output)?;
        match network1.find_node(&output_node.name) {
            Some(candidate) if network1.node(candidate)?.kind == NodeKind::PrimaryOutput => {}
            _ => diagnostics.push(format!(
                "output '{}' only in network '{}'",
                output_node.name,
                network2.name()
            )),
        }
    }

    Ok(diagnostics)
}

fn primary_output_names(network: &Network) -> NetworkVerifyResult<Vec<String>> {
    network
        .primary_outputs()
        .iter()
        .map(|output| network.node(*output).map(|node| node.name.clone()))
        .collect::<NetworkUtilResult<Vec<_>>>()
        .map_err(Into::into)
}

fn primary_input_names(network1: &Network, network2: &Network) -> NetworkVerifyResult<Vec<String>> {
    let mut names = BTreeSet::new();
    for input in network1.primary_inputs() {
        names.insert(network1.node(*input)?.name.clone());
    }
    for input in network2.primary_inputs() {
        names.insert(network2.node(*input)?.name.clone());
    }

    Ok(names.into_iter().collect())
}

fn outputs_equal(
    network1: &Network,
    output1: NodeId,
    network2: &Network,
    output2: NodeId,
    input_names: &[String],
) -> NetworkVerifyResult<bool> {
    visit_assignments(input_names.len(), &mut Vec::new(), &mut |values| {
        let assignment = input_names
            .iter()
            .cloned()
            .zip(values.iter().copied())
            .collect::<BTreeMap<_, _>>();

        let value1 = evaluate_primary_output(network1, output1, &assignment)?;
        let value2 = evaluate_primary_output(network2, output2, &assignment)?;

        Ok(value1 == value2)
    })
}

fn evaluate_primary_output(
    network: &Network,
    output: NodeId,
    assignment: &BTreeMap<String, bool>,
) -> NetworkVerifyResult<bool> {
    let output_node = network.node(output)?;
    if output_node.kind != NodeKind::PrimaryOutput || output_node.fanins.len() != 1 {
        return Err(NetworkUtilError::InvalidPrimaryOutput(output).into());
    }

    evaluate_node(
        network,
        output_node.fanins[0],
        assignment,
        &mut BTreeMap::new(),
        &mut BTreeSet::new(),
    )
}

fn evaluate_node(
    network: &Network,
    node: NodeId,
    assignment: &BTreeMap<String, bool>,
    memo: &mut BTreeMap<NodeId, bool>,
    active: &mut BTreeSet<NodeId>,
) -> NetworkVerifyResult<bool> {
    if let Some(value) = memo.get(&node) {
        return Ok(*value);
    }

    if !active.insert(node) {
        return Err(NetworkUtilError::CycleDetected.into());
    }

    let network_node = network.node(node)?;
    let value = match network_node.kind {
        NodeKind::PrimaryInput => assignment.get(&network_node.name).copied().unwrap_or(false),
        NodeKind::PrimaryOutput => {
            if network_node.fanins.len() != 1 {
                return Err(NetworkUtilError::InvalidPrimaryOutput(node).into());
            }

            evaluate_node(network, network_node.fanins[0], assignment, memo, active)?
        }
        NodeKind::Internal | NodeKind::Unassigned => {
            if let Some(expression) = &network_node.expression {
                evaluate_expression(network, expression, assignment, memo, active)?
            } else if let Some(cover) = &network_node.cover {
                let mut matched = false;
                for (cube_index, cube) in cover.cubes().iter().enumerate() {
                    if cube.values().len() != network_node.fanins.len() {
                        return Err(NetworkUtilError::InvalidCover {
                            node,
                            cube: cube_index,
                        }
                        .into());
                    }

                    let mut cube_matches = true;
                    for (fanin_index, cover_value) in cube.values().iter().enumerate() {
                        if *cover_value == CoverValue::DontCare {
                            continue;
                        }

                        let fanin_value = evaluate_node(
                            network,
                            network_node.fanins[fanin_index],
                            assignment,
                            memo,
                            active,
                        )?;
                        let required = *cover_value == CoverValue::One;
                        if fanin_value != required {
                            cube_matches = false;
                            break;
                        }
                    }

                    if cube_matches {
                        matched = true;
                        break;
                    }
                }

                matched
            } else {
                false
            }
        }
    };

    active.remove(&node);
    memo.insert(node, value);
    Ok(value)
}

fn evaluate_expression(
    network: &Network,
    expression: &BoolExpr,
    assignment: &BTreeMap<String, bool>,
    memo: &mut BTreeMap<NodeId, bool>,
    active: &mut BTreeSet<NodeId>,
) -> NetworkVerifyResult<bool> {
    match expression {
        BoolExpr::Constant(value) => Ok(*value),
        BoolExpr::Literal { node, phase } => {
            Ok(evaluate_node(network, *node, assignment, memo, active)? == *phase)
        }
        BoolExpr::Not(inner) => Ok(!evaluate_expression(
            network, inner, assignment, memo, active,
        )?),
        BoolExpr::And(items) => {
            for item in items {
                if !evaluate_expression(network, item, assignment, memo, active)? {
                    return Ok(false);
                }
            }

            Ok(true)
        }
        BoolExpr::Or(items) => {
            for item in items {
                if evaluate_expression(network, item, assignment, memo, active)? {
                    return Ok(true);
                }
            }

            Ok(false)
        }
    }
}

fn visit_assignments<F>(
    input_count: usize,
    partial: &mut Vec<bool>,
    visit: &mut F,
) -> NetworkVerifyResult<bool>
where
    F: FnMut(&[bool]) -> NetworkVerifyResult<bool>,
{
    if partial.len() == input_count {
        return visit(partial);
    }

    partial.push(false);
    if !visit_assignments(input_count, partial, visit)? {
        partial.pop();
        return Ok(false);
    }
    partial.pop();

    partial.push(true);
    if !visit_assignments(input_count, partial, visit)? {
        partial.pop();
        return Ok(false);
    }
    partial.pop();

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::network_util::{Cube, NetworkNode, SopCover};
    use super::*;

    fn cube(values: &[CoverValue]) -> Cube {
        Cube::new(values.to_vec())
    }

    fn and_network(name: &str, output_name: &str) -> Network {
        let mut network = Network::new();
        network.set_name(name);
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let driver = network
            .add_internal(
                "n",
                vec![a, b],
                SopCover::new([cube(&[CoverValue::One, CoverValue::One])]),
            )
            .unwrap();
        let output = network.add_primary_output(driver).unwrap();
        network.change_node_name(output, output_name).unwrap();
        network
    }

    fn or_network(name: &str, output_name: &str) -> Network {
        let mut network = Network::new();
        network.set_name(name);
        let a = network
            .add_primary_input(NetworkNode::new("a", NodeKind::PrimaryInput))
            .unwrap();
        let b = network
            .add_primary_input(NetworkNode::new("b", NodeKind::PrimaryInput))
            .unwrap();
        let driver = network
            .add_internal(
                "n",
                vec![a, b],
                SopCover::new([
                    cube(&[CoverValue::One, CoverValue::DontCare]),
                    cube(&[CoverValue::DontCare, CoverValue::One]),
                ]),
            )
            .unwrap();
        let output = network.add_primary_output(driver).unwrap();
        network.change_node_name(output, output_name).unwrap();
        network
    }

    #[test]
    fn collapse_verification_accepts_equivalent_outputs() {
        let left = and_network("left", "y");
        let right = and_network("right", "y");

        let report = verify_by_collapse(&left, &right).unwrap();

        assert!(report.equivalent);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn collapse_verification_reports_first_different_output() {
        let left = and_network("left", "y");
        let right = or_network("right", "y");

        let report = verify_by_collapse(&left, &right).unwrap();

        assert!(!report.equivalent);
        assert_eq!(
            report.diagnostics,
            vec!["Networks differ on (at least) primary output y"]
        );
    }

    #[test]
    fn network_verify_reports_primary_output_name_mismatches_before_function_check() {
        let left = and_network("left", "left_y");
        let right = and_network("right", "right_y");

        let report = network_verify(&left, &right, VerificationMethod::Collapse).unwrap();

        assert!(!report.equivalent);
        assert_eq!(
            report.diagnostics,
            vec![
                "output 'left_y' only in network 'left'",
                "output 'right_y' only in network 'right'",
            ]
        );
    }

    #[test]
    fn external_dc_networks_must_match_on_both_sides() {
        let mut left = and_network("left", "y");
        let right = and_network("right", "y");
        left.set_dc_network(Some(or_network("dc", "y")));

        let report = net_verify_with_dc(&left, &right, VerificationMethod::Collapse).unwrap();

        assert!(!report.equivalent);
        assert_eq!(
            report.diagnostics,
            vec!["External don't care networks are not equal."]
        );
    }

    #[test]
    fn external_dc_networks_are_verified_before_care_networks() {
        let mut left = and_network("left", "y");
        let mut right = and_network("right", "y");
        left.set_dc_network(Some(and_network("dc_left", "y")));
        right.set_dc_network(Some(or_network("dc_right", "y")));

        let report = net_verify_with_dc(&left, &right, VerificationMethod::Collapse).unwrap();

        assert!(!report.equivalent);
        assert_eq!(
            report.diagnostics,
            vec![
                "Networks differ on (at least) primary output y",
                "External don't care networks are not equal.",
            ]
        );
    }

    #[test]
    fn bdd_method_reports_missing_native_dependency() {
        let left = and_network("left", "y");
        let right = and_network("right", "y");

        let error = network_verify(&left, &right, VerificationMethod::Bdd).unwrap_err();

        assert_eq!(error, NetworkVerifyError::UnsupportedBddMethod);
    }

    #[test]
    fn legacy_method_rejects_unknown_method() {
        let left = and_network("left", "y");
        let right = and_network("right", "y");

        let error = network_verify_legacy_method(&left, &right, 2).unwrap_err();

        assert_eq!(error, NetworkVerifyError::UnknownMethod(2));
    }

    #[test]
    fn no_source_dependency_metadata_or_legacy_c_abi_tokens_are_present() {
        let source = include_str!("net_verify.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
