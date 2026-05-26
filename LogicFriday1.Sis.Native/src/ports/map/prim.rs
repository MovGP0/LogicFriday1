//! Native Rust primitive-gate support for `sis/map/prim.c`.
//!
//! The original file built the mapper's `prim_t` graph from SIS `network_t`
//! objects. The native port keeps the behavior needed by the Rust mapper as
//! owned primitive descriptors: classify genlib functions, construct virtual
//! gates, and keep explicit fanin arity validation. It intentionally exposes no
//! legacy C ABI entry points.

use std::error::Error;
use std::fmt;

use super::library::GenlibGate;
use super::virtual_net::{GateKind, SourceRef, VirtualMappedNetwork};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PrimitiveKind {
    Inverter,
    Nand,
    Nor,
    Xor,
    Xnor,
    Mux,
    And,
    Or,
    One,
    Zero,
    Wire,
}

impl PrimitiveKind {
    pub fn gate_kind(self) -> GateKind {
        match self {
            Self::Inverter => GateKind::Inverter,
            Self::Nand => GateKind::Nand,
            Self::Nor => GateKind::Nor,
            Self::Xor => GateKind::Xor,
            Self::Xnor => GateKind::Xnor,
            Self::Mux => GateKind::Mux,
            Self::And => GateKind::And,
            Self::Or => GateKind::Or,
            Self::One => GateKind::One,
            Self::Zero => GateKind::Zero,
            Self::Wire => GateKind::Wire,
        }
    }

    pub fn arity_rule(self) -> PrimitiveArity {
        match self {
            Self::One | Self::Zero => PrimitiveArity::Exact(0),
            Self::Inverter | Self::Wire => PrimitiveArity::Exact(1),
            Self::Mux => PrimitiveArity::Exact(3),
            Self::Nand | Self::Nor | Self::Xor | Self::Xnor | Self::And | Self::Or => {
                PrimitiveArity::AtLeast(2)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveArity {
    Exact(usize),
    AtLeast(usize),
}

impl PrimitiveArity {
    pub fn accepts(self, arity: usize) -> bool {
        match self {
            Self::Exact(expected) => arity == expected,
            Self::AtLeast(minimum) => arity >= minimum,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveGate {
    pub name: String,
    pub kind: PrimitiveKind,
    pub input_names: Vec<String>,
    pub area: Option<f64>,
}

impl PrimitiveGate {
    pub fn new(
        name: impl Into<String>,
        kind: PrimitiveKind,
        input_names: Vec<String>,
    ) -> Result<Self, PrimitiveError> {
        Self::with_area(name, kind, input_names, None)
    }

    pub fn with_area(
        name: impl Into<String>,
        kind: PrimitiveKind,
        input_names: Vec<String>,
        area: Option<f64>,
    ) -> Result<Self, PrimitiveError> {
        let gate = Self {
            name: name.into(),
            kind,
            input_names,
            area,
        };
        gate.validate()?;
        Ok(gate)
    }

    pub fn from_genlib(gate: &GenlibGate) -> Result<Self, PrimitiveError> {
        let pin_names = gate
            .pins
            .iter()
            .map(|pin| pin.declared_name.clone())
            .collect::<Vec<_>>();
        let kind = infer_genlib_kind(gate)?;
        Self::with_area(gate.name.clone(), kind, pin_names, Some(gate.area))
    }

    pub fn validate(&self) -> Result<(), PrimitiveError> {
        if self.name.trim().is_empty() {
            return Err(PrimitiveError::EmptyName);
        }
        if let Some(area) = self.area {
            if !area.is_finite() || area < 0.0 {
                return Err(PrimitiveError::InvalidArea { area });
            }
        }
        if !self.kind.arity_rule().accepts(self.input_names.len()) {
            return Err(PrimitiveError::InvalidArity {
                kind: self.kind,
                arity: self.input_names.len(),
            });
        }
        if self.input_names.iter().any(|name| name.trim().is_empty()) {
            return Err(PrimitiveError::EmptyInputName);
        }

        Ok(())
    }

    pub fn add_to_virtual_network(
        &self,
        network: &mut VirtualMappedNetwork,
        output_name: impl Into<String>,
        fanins: Vec<SourceRef>,
    ) -> Result<super::virtual_net::NodeId, PrimitiveError> {
        if !self.kind.arity_rule().accepts(fanins.len()) {
            return Err(PrimitiveError::InvalidArity {
                kind: self.kind,
                arity: fanins.len(),
            });
        }

        Ok(network.add_gate(output_name, self.kind.gate_kind(), fanins))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveError {
    EmptyName,
    EmptyInputName,
    InvalidArea { area: f64 },
    InvalidArity { kind: PrimitiveKind, arity: usize },
    ParseExpression { expression: String, message: String },
    UnsupportedFunction { gate: String, expression: String },
}

impl fmt::Display for PrimitiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "primitive gate name must not be empty"),
            Self::EmptyInputName => write!(f, "primitive gate input name must not be empty"),
            Self::InvalidArea { area } => write!(f, "primitive gate area {area} is invalid"),
            Self::InvalidArity { kind, arity } => {
                write!(f, "primitive gate {kind:?} does not accept {arity} inputs")
            }
            Self::ParseExpression {
                expression,
                message,
            } => {
                write!(
                    f,
                    "could not parse genlib expression '{expression}': {message}"
                )
            }
            Self::UnsupportedFunction { gate, expression } => {
                write!(
                    f,
                    "gate '{gate}' has unsupported primitive function '{expression}'"
                )
            }
        }
    }
}

impl Error for PrimitiveError {}

pub fn classify_genlib_gate(gate: &GenlibGate) -> Result<PrimitiveKind, PrimitiveError> {
    infer_genlib_kind(gate)
}

pub fn classify_expression(
    expression: &str,
    input_names: &[String],
) -> Result<PrimitiveKind, PrimitiveError> {
    let parsed =
        Parser::new(expression)
            .parse()
            .map_err(|message| PrimitiveError::ParseExpression {
                expression: expression.to_string(),
                message,
            })?;
    infer_expression_kind(&parsed, input_names).ok_or_else(|| PrimitiveError::UnsupportedFunction {
        gate: "<expression>".to_string(),
        expression: expression.to_string(),
    })
}

fn infer_genlib_kind(gate: &GenlibGate) -> Result<PrimitiveKind, PrimitiveError> {
    let input_names = gate
        .pins
        .iter()
        .map(|pin| pin.declared_name.clone())
        .collect::<Vec<_>>();

    classify_expression(&gate.output.expression, &input_names).map_err(|error| match error {
        PrimitiveError::UnsupportedFunction { .. } => PrimitiveError::UnsupportedFunction {
            gate: gate.name.clone(),
            expression: gate.output.expression.clone(),
        },
        other => other,
    })
}

fn infer_expression_kind(expression: &Expression, input_names: &[String]) -> Option<PrimitiveKind> {
    match expression {
        Expression::Const(false) => Some(PrimitiveKind::Zero),
        Expression::Const(true) => Some(PrimitiveKind::One),
        Expression::Variable(name) if input_names.len() == 1 && input_names[0] == *name => {
            Some(PrimitiveKind::Wire)
        }
        Expression::Not(inner) => match inner.as_ref() {
            Expression::Variable(name) if input_names.len() == 1 && input_names[0] == *name => {
                Some(PrimitiveKind::Inverter)
            }
            Expression::And(terms) if terms_cover_inputs(terms, input_names) => {
                Some(PrimitiveKind::Nand)
            }
            Expression::Or(terms) if terms_cover_inputs(terms, input_names) => {
                Some(PrimitiveKind::Nor)
            }
            Expression::Xor(terms) if terms_cover_inputs(terms, input_names) => {
                Some(PrimitiveKind::Xnor)
            }
            _ => None,
        },
        Expression::And(terms) if terms_cover_inputs(terms, input_names) => {
            Some(PrimitiveKind::And)
        }
        Expression::Or(terms) if terms_cover_inputs(terms, input_names) => Some(PrimitiveKind::Or),
        Expression::Xor(terms) if terms_cover_inputs(terms, input_names) => {
            Some(PrimitiveKind::Xor)
        }
        Expression::Or(terms) if is_mux(terms, input_names) => Some(PrimitiveKind::Mux),
        _ => None,
    }
}

fn terms_cover_inputs(terms: &[Expression], input_names: &[String]) -> bool {
    if terms.len() != input_names.len() || terms.len() < 2 {
        return false;
    }

    input_names.iter().all(|input| {
        terms
            .iter()
            .any(|term| matches!(term, Expression::Variable(name) if name == input))
    })
}

fn is_mux(terms: &[Expression], input_names: &[String]) -> bool {
    if input_names.len() != 3 || terms.len() != 2 {
        return false;
    }

    mux_product_candidates(&terms[0])
        .iter()
        .any(|(select, left_data, left_inverted)| {
            mux_product_candidates(&terms[1]).iter().any(
                |(right_select, right_data, right_inverted)| {
                    if select != right_select || left_inverted == right_inverted {
                        return false;
                    }

                    let select_present = input_names.iter().any(|name| name == select);
                    let left_present = input_names.iter().any(|name| name == left_data);
                    let right_present = input_names.iter().any(|name| name == right_data);

                    select_present && left_present && right_present && left_data != right_data
                },
            )
        })
}

fn mux_product_candidates(expression: &Expression) -> Vec<(&str, &str, bool)> {
    let Expression::And(terms) = expression else {
        return Vec::new();
    };
    if terms.len() != 2 {
        return Vec::new();
    }

    match (&terms[0], &terms[1]) {
        (Expression::Variable(data), Expression::Not(select))
        | (Expression::Not(select), Expression::Variable(data)) => match select.as_ref() {
            Expression::Variable(select) => vec![(select.as_str(), data.as_str(), true)],
            _ => Vec::new(),
        },
        (Expression::Variable(left), Expression::Variable(right)) => vec![
            (left.as_str(), right.as_str(), false),
            (right.as_str(), left.as_str(), false),
        ],
        _ => Vec::new(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Expression {
    Const(bool),
    Variable(String),
    Not(Box<Expression>),
    And(Vec<Expression>),
    Or(Vec<Expression>),
    Xor(Vec<Expression>),
}

struct Parser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn parse(mut self) -> Result<Expression, String> {
        let expression = self.parse_or()?;
        self.skip_whitespace();
        if self.peek().is_some() {
            return Err(format!("unexpected token at byte {}", self.position));
        }

        Ok(expression)
    }

    fn parse_or(&mut self) -> Result<Expression, String> {
        let mut terms = vec![self.parse_xor()?];
        while self.consume('+') || self.consume('|') {
            terms.push(self.parse_xor()?);
        }

        Ok(flatten_or(terms))
    }

    fn parse_xor(&mut self) -> Result<Expression, String> {
        let mut terms = vec![self.parse_and()?];
        while self.consume('^') {
            terms.push(self.parse_and()?);
        }

        Ok(flatten_xor(terms))
    }

    fn parse_and(&mut self) -> Result<Expression, String> {
        let mut terms = vec![self.parse_not()?];
        while self.consume('*') || self.consume('&') {
            terms.push(self.parse_not()?);
        }

        Ok(flatten_and(terms))
    }

    fn parse_not(&mut self) -> Result<Expression, String> {
        if self.consume('!') || self.consume('~') {
            return Ok(Expression::Not(Box::new(self.parse_not()?)));
        }

        let mut expression = self.parse_primary()?;
        while self.consume('\'') {
            expression = Expression::Not(Box::new(expression));
        }

        Ok(expression)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();
        if self.consume('(') {
            let expression = self.parse_or()?;
            if !self.consume(')') {
                return Err(format!("expected ')' at byte {}", self.position));
            }
            return Ok(expression);
        }

        let token = self.parse_token()?;
        match token.as_str() {
            "0" | "CONST0" | "const0" => Ok(Expression::Const(false)),
            "1" | "CONST1" | "const1" => Ok(Expression::Const(true)),
            _ => Ok(Expression::Variable(token)),
        }
    }

    fn parse_token(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        let start = self.position;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '$' {
                self.position += ch.len_utf8();
            } else {
                break;
            }
        }

        if self.position == start {
            Err(format!("expected variable or constant at byte {start}"))
        } else {
            Ok(self.input[start..self.position].to_string())
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        self.skip_whitespace();
        if self.peek() == Some(expected) {
            self.position += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.position += 1;
        }
    }
}

fn flatten_and(terms: Vec<Expression>) -> Expression {
    flatten(terms, Expression::And)
}

fn flatten_or(terms: Vec<Expression>) -> Expression {
    flatten(terms, Expression::Or)
}

fn flatten_xor(terms: Vec<Expression>) -> Expression {
    flatten(terms, Expression::Xor)
}

fn flatten(
    terms: Vec<Expression>,
    constructor: impl FnOnce(Vec<Expression>) -> Expression,
) -> Expression {
    if terms.len() == 1 {
        terms.into_iter().next().unwrap()
    } else {
        constructor(terms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::library::{GenlibOutput, GenlibPin, GenlibPinName, PinPhase};
    use crate::ports::map::virtual_net::SourceRef;

    fn pins(names: &[&str]) -> Vec<GenlibPin> {
        names
            .iter()
            .map(|name| GenlibPin {
                name: GenlibPinName::Declared((*name).to_string()),
                declared_name: (*name).to_string(),
                phase: PinPhase::Unknown,
                input_load: 1.0,
                max_load: 999.0,
                rise_block_delay: 1.0,
                rise_fanout_delay: 0.2,
                fall_block_delay: 1.0,
                fall_fanout_delay: 0.2,
            })
            .collect()
    }

    fn genlib_gate(name: &str, expression: &str, inputs: &[&str]) -> GenlibGate {
        GenlibGate::new(name, 1.0, GenlibOutput::new("O", expression), pins(inputs)).unwrap()
    }

    #[test]
    fn classifies_common_genlib_primitives() {
        let cases = [
            ("inv", "!a", &["a"][..], PrimitiveKind::Inverter),
            ("nand", "!(a*b)", &["a", "b"][..], PrimitiveKind::Nand),
            ("nor", "!(a+b)", &["a", "b"][..], PrimitiveKind::Nor),
            ("xor", "a^b", &["a", "b"][..], PrimitiveKind::Xor),
            ("xnor", "!(a^b)", &["a", "b"][..], PrimitiveKind::Xnor),
            ("and", "a*b", &["a", "b"][..], PrimitiveKind::And),
            ("or", "a+b", &["a", "b"][..], PrimitiveKind::Or),
            ("one", "CONST1", &[][..], PrimitiveKind::One),
            ("zero", "0", &[][..], PrimitiveKind::Zero),
            ("wire", "a", &["a"][..], PrimitiveKind::Wire),
        ];

        for (name, expression, inputs, expected) in cases {
            let gate = genlib_gate(name, expression, inputs);

            assert_eq!(classify_genlib_gate(&gate).unwrap(), expected);
            assert_eq!(PrimitiveGate::from_genlib(&gate).unwrap().kind, expected);
        }
    }

    #[test]
    fn classifies_two_product_mux() {
        let gate = genlib_gate("mux2", "a*!s+b*s", &["s", "a", "b"]);

        assert_eq!(classify_genlib_gate(&gate).unwrap(), PrimitiveKind::Mux);
    }

    #[test]
    fn rejects_unsupported_compound_function() {
        let gate = genlib_gate("complex", "a*b+c", &["a", "b", "c"]);

        assert!(matches!(
            classify_genlib_gate(&gate),
            Err(PrimitiveError::UnsupportedFunction { .. })
        ));
    }

    #[test]
    fn validates_primitive_arity() {
        assert!(PrimitiveGate::new("bad_inv", PrimitiveKind::Inverter, vec![]).is_err());
        assert!(
            PrimitiveGate::new(
                "and2",
                PrimitiveKind::And,
                vec!["a".to_string(), "b".to_string()]
            )
            .is_ok()
        );
    }

    #[test]
    fn constructs_virtual_network_gate() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let gate =
            PrimitiveGate::new("inv", PrimitiveKind::Inverter, vec!["a".to_string()]).unwrap();

        let node = gate
            .add_to_virtual_network(&mut network, "not_a", vec![SourceRef::Node(a)])
            .unwrap();

        assert_eq!(network.node(node).unwrap().gate, Some(GateKind::Inverter));
    }
}
