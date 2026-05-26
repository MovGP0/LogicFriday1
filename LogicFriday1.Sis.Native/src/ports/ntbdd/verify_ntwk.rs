//! Native Boolean-network verification for the ntbdd port.
//!
//! The legacy implementation builds BDDs for each primary-output function and
//! compares canonical forms. This port keeps the same observable verification
//! rules over owned Rust data: primary outputs are matched by name, primary
//! inputs with the same name share a variable, unmatched inputs are still part
//! of the counterexample space, and verification may run one output at a time
//! or as a single pass.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

const MAX_EXHAUSTIVE_INPUTS: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderMethod {
    Dfs,
    Random,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifyMethod {
    OneAtATime,
    AllTogether,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BooleanNetwork {
    inputs: Vec<String>,
    outputs: Vec<NetworkOutput>,
}

impl BooleanNetwork {
    pub fn new(
        inputs: impl IntoIterator<Item = impl Into<String>>,
        outputs: impl IntoIterator<Item = NetworkOutput>,
    ) -> Self {
        Self {
            inputs: inputs.into_iter().map(Into::into).collect(),
            outputs: outputs.into_iter().collect(),
        }
    }

    pub fn inputs(&self) -> &[String] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[NetworkOutput] {
        &self.outputs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkOutput {
    name: String,
    function: BoolExpr,
}

impl NetworkOutput {
    pub fn new(name: impl Into<String>, function: BoolExpr) -> Self {
        Self {
            name: name.into(),
            function,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn function(&self) -> &BoolExpr {
        &self.function
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoolExpr {
    Const(bool),
    Input(String),
    Not(Box<BoolExpr>),
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
    Xor(Box<BoolExpr>, Box<BoolExpr>),
}

impl BoolExpr {
    pub fn input(name: impl Into<String>) -> Self {
        Self::Input(name.into())
    }

    pub fn logical_not(expr: BoolExpr) -> Self {
        Self::Not(Box::new(expr))
    }

    pub fn and(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::And(exprs.into_iter().collect())
    }

    pub fn or(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::Or(exprs.into_iter().collect())
    }

    pub fn xor(left: BoolExpr, right: BoolExpr) -> Self {
        Self::Xor(Box::new(left), Box::new(right))
    }

    fn eval(&self, assignment: &BTreeMap<String, bool>) -> Result<bool, VerifyError> {
        match self {
            Self::Const(value) => Ok(*value),
            Self::Input(name) => assignment
                .get(name)
                .copied()
                .ok_or_else(|| VerifyError::UnknownInputInFunction { name: name.clone() }),
            Self::Not(expr) => Ok(!expr.eval(assignment)?),
            Self::And(exprs) => {
                for expr in exprs {
                    if !expr.eval(assignment)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            Self::Or(exprs) => {
                for expr in exprs {
                    if expr.eval(assignment)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            Self::Xor(left, right) => Ok(left.eval(assignment)? ^ right.eval(assignment)?),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationFailure {
    pub output_name: String,
    pub counterexample: Vec<InputValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputValue {
    pub name: String,
    pub value: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationResult {
    Equivalent,
    Different(VerificationFailure),
}

impl VerificationResult {
    pub fn is_equivalent(&self) -> bool {
        matches!(self, Self::Equivalent)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifyError {
    OutputSetMismatch {
        left_count: usize,
        right_count: usize,
        missing_in_left: Vec<String>,
        missing_in_right: Vec<String>,
    },
    DuplicateOutputName {
        name: String,
    },
    DuplicateInputName {
        name: String,
    },
    UnknownInputInFunction {
        name: String,
    },
    TooManyInputs {
        count: usize,
    },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputSetMismatch {
                left_count,
                right_count,
                missing_in_left,
                missing_in_right,
            } => write!(
                f,
                "network output sets differ: left has {left_count}, right has {right_count}, missing in left {:?}, missing in right {:?}",
                missing_in_left, missing_in_right
            ),
            Self::DuplicateOutputName { name } => {
                write!(f, "duplicate primary output name {name}")
            }
            Self::DuplicateInputName { name } => {
                write!(f, "duplicate primary input name {name}")
            }
            Self::UnknownInputInFunction { name } => {
                write!(f, "function references unknown primary input {name}")
            }
            Self::TooManyInputs { count } => {
                write!(
                    f,
                    "network verification needs {count} inputs; exhaustive comparison supports at most {MAX_EXHAUSTIVE_INPUTS}"
                )
            }
        }
    }
}

impl Error for VerifyError {}

pub fn verify_networks(
    left: &BooleanNetwork,
    right: &BooleanNetwork,
    order_method: OrderMethod,
    verify_method: VerifyMethod,
) -> Result<VerificationResult, VerifyError> {
    let paired_outputs = same_output_order(left.outputs(), right.outputs())?;
    let input_order = build_input_order(left.inputs(), right.inputs(), order_method)?;

    match verify_method {
        VerifyMethod::OneAtATime => verify_one_at_a_time(&paired_outputs, &input_order),
        VerifyMethod::AllTogether => verify_all_together(&paired_outputs, &input_order),
    }
}

fn verify_one_at_a_time(
    paired_outputs: &[(NetworkOutput, NetworkOutput)],
    input_order: &[String],
) -> Result<VerificationResult, VerifyError> {
    for pair in paired_outputs {
        let single = [pair.clone()];
        let result = verify_all_together(&single, input_order)?;
        if !result.is_equivalent() {
            return Ok(result);
        }
    }

    Ok(VerificationResult::Equivalent)
}

fn verify_all_together(
    paired_outputs: &[(NetworkOutput, NetworkOutput)],
    input_order: &[String],
) -> Result<VerificationResult, VerifyError> {
    if input_order.len() > MAX_EXHAUSTIVE_INPUTS {
        return Err(VerifyError::TooManyInputs {
            count: input_order.len(),
        });
    }

    let assignment_count = 1_u64 << input_order.len();
    for bits in 0..assignment_count {
        let assignment = assignment_from_bits(input_order, bits);
        for (left_output, right_output) in paired_outputs {
            let left_value = left_output.function.eval(&assignment)?;
            let right_value = right_output.function.eval(&assignment)?;
            if left_value != right_value {
                return Ok(VerificationResult::Different(VerificationFailure {
                    output_name: left_output.name.clone(),
                    counterexample: input_order
                        .iter()
                        .map(|name| InputValue {
                            name: name.clone(),
                            value: assignment[name],
                        })
                        .collect(),
                }));
            }
        }
    }

    Ok(VerificationResult::Equivalent)
}

fn same_output_order(
    left: &[NetworkOutput],
    right: &[NetworkOutput],
) -> Result<Vec<(NetworkOutput, NetworkOutput)>, VerifyError> {
    validate_unique_outputs(left)?;
    validate_unique_outputs(right)?;

    let left_by_name = left
        .iter()
        .enumerate()
        .map(|(index, output)| (output.name.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let right_by_name = right
        .iter()
        .enumerate()
        .map(|(index, output)| (output.name.clone(), index))
        .collect::<BTreeMap<_, _>>();

    if left_by_name.keys().ne(right_by_name.keys()) {
        return Err(VerifyError::OutputSetMismatch {
            left_count: left.len(),
            right_count: right.len(),
            missing_in_left: right_by_name
                .keys()
                .filter(|name| !left_by_name.contains_key(*name))
                .cloned()
                .collect(),
            missing_in_right: left_by_name
                .keys()
                .filter(|name| !right_by_name.contains_key(*name))
                .cloned()
                .collect(),
        });
    }

    let mut pairs = Vec::with_capacity(left.len());
    for left_output in left {
        let right_index = right_by_name[&left_output.name];
        pairs.push((left_output.clone(), right[right_index].clone()));
    }

    Ok(pairs)
}

fn validate_unique_outputs(outputs: &[NetworkOutput]) -> Result<(), VerifyError> {
    let mut seen = BTreeSet::new();
    for output in outputs {
        if !seen.insert(output.name.clone()) {
            return Err(VerifyError::DuplicateOutputName {
                name: output.name.clone(),
            });
        }
    }

    Ok(())
}

fn build_input_order(
    left_inputs: &[String],
    right_inputs: &[String],
    order_method: OrderMethod,
) -> Result<Vec<String>, VerifyError> {
    validate_unique_inputs(left_inputs)?;
    validate_unique_inputs(right_inputs)?;

    let mut order = match order_method {
        OrderMethod::Dfs => left_inputs.to_vec(),
        OrderMethod::Random => {
            let mut inputs = left_inputs.to_vec();
            inputs.sort();
            inputs
        }
    };
    let mut present = order.iter().cloned().collect::<BTreeSet<_>>();

    for input in right_inputs {
        if present.insert(input.clone()) {
            order.push(input.clone());
        }
    }

    Ok(order)
}

fn validate_unique_inputs(inputs: &[String]) -> Result<(), VerifyError> {
    let mut seen = BTreeSet::new();
    for input in inputs {
        if !seen.insert(input.clone()) {
            return Err(VerifyError::DuplicateInputName {
                name: input.clone(),
            });
        }
    }

    Ok(())
}

fn assignment_from_bits(input_order: &[String], bits: u64) -> BTreeMap<String, bool> {
    input_order
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), ((bits >> index) & 1) != 0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(name: &str, function: BoolExpr) -> NetworkOutput {
        NetworkOutput::new(name, function)
    }

    fn input(name: &str) -> BoolExpr {
        BoolExpr::input(name)
    }

    #[test]
    fn reordered_outputs_are_matched_by_name() {
        let left = BooleanNetwork::new(
            ["a", "b"],
            [
                output("sum", BoolExpr::xor(input("a"), input("b"))),
                output("carry", BoolExpr::and([input("a"), input("b")])),
            ],
        );
        let right = BooleanNetwork::new(
            ["b", "a"],
            [
                output("carry", BoolExpr::and([input("b"), input("a")])),
                output("sum", BoolExpr::xor(input("b"), input("a"))),
            ],
        );

        assert_eq!(
            verify_networks(&left, &right, OrderMethod::Dfs, VerifyMethod::AllTogether).unwrap(),
            VerificationResult::Equivalent
        );
    }

    #[test]
    fn output_set_mismatch_is_reported_before_function_comparison() {
        let left = BooleanNetwork::new(["a"], [output("z", input("a"))]);
        let right = BooleanNetwork::new(["a"], [output("other", input("a"))]);

        assert_eq!(
            verify_networks(&left, &right, OrderMethod::Dfs, VerifyMethod::AllTogether),
            Err(VerifyError::OutputSetMismatch {
                left_count: 1,
                right_count: 1,
                missing_in_left: vec!["other".to_owned()],
                missing_in_right: vec!["z".to_owned()],
            })
        );
    }

    #[test]
    fn extra_unused_inputs_do_not_make_equivalent_constants_different() {
        let left = BooleanNetwork::new(["a"], [output("z", BoolExpr::Const(true))]);
        let right = BooleanNetwork::new(["b"], [output("z", BoolExpr::Const(true))]);

        assert_eq!(
            verify_networks(&left, &right, OrderMethod::Dfs, VerifyMethod::AllTogether).unwrap(),
            VerificationResult::Equivalent
        );
    }

    #[test]
    fn differing_output_reports_first_counterexample_with_all_input_names() {
        let left = BooleanNetwork::new(["a"], [output("z", input("a"))]);
        let right = BooleanNetwork::new(["b"], [output("z", input("b"))]);

        assert_eq!(
            verify_networks(&left, &right, OrderMethod::Dfs, VerifyMethod::OneAtATime).unwrap(),
            VerificationResult::Different(VerificationFailure {
                output_name: "z".to_owned(),
                counterexample: vec![
                    InputValue {
                        name: "a".to_owned(),
                        value: true,
                    },
                    InputValue {
                        name: "b".to_owned(),
                        value: false,
                    },
                ],
            })
        );
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let left = BooleanNetwork::new(
            ["a"],
            [output("z", input("a")), output("z", BoolExpr::Const(false))],
        );
        let right = BooleanNetwork::new(["a"], [output("z", input("a"))]);

        assert_eq!(
            verify_networks(&left, &right, OrderMethod::Dfs, VerifyMethod::AllTogether),
            Err(VerifyError::DuplicateOutputName {
                name: "z".to_owned(),
            })
        );

        let left = BooleanNetwork::new(["a", "a"], [output("z", input("a"))]);
        assert_eq!(
            verify_networks(&left, &right, OrderMethod::Dfs, VerifyMethod::AllTogether),
            Err(VerifyError::DuplicateInputName {
                name: "a".to_owned(),
            })
        );
    }
}
