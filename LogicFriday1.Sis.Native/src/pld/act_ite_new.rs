//! Owned-data Rust port of `LogicSynthesis/sis/pld/act_ite_new.c`.
//!
//! The C file builds ITE vertices for two narrow ACT mapper cases: one cube
//! implemented as an AND of literals, and a cover whose cubes are single
//! literals implemented as an OR. This module preserves that construction over
//! explicit Rust data and returns diagnostics for SIS-only failure cases instead
//! of terminating the process.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase {
    PositiveUnate,
    NegativeUnate,
    Binate,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiteralInput {
    pub name: String,
    pub phase: InputPhase,
}

impl LiteralInput {
    pub fn new(name: impl Into<String>, phase: InputPhase) -> Self {
        Self {
            name: name.into(),
            phase,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LiteralNode {
    fanins: Vec<LiteralInput>,
}

impl LiteralNode {
    pub fn new(fanins: impl IntoIterator<Item = LiteralInput>) -> Self {
        Self {
            fanins: fanins.into_iter().collect(),
        }
    }

    pub fn fanins(&self) -> &[LiteralInput] {
        &self.fanins
    }

    pub fn fanin_count(&self) -> usize {
        self.fanins.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteVertex {
    kind: IteKind,
    index_size: usize,
}

impl IteVertex {
    pub fn terminal(value: bool) -> Self {
        Self {
            kind: IteKind::Terminal(value),
            index_size: 0,
        }
    }

    pub fn literal(name: impl Into<String>) -> Self {
        Self {
            kind: IteKind::Literal { name: name.into() },
            index_size: 1,
        }
    }

    pub fn shannon(condition: Self, then_branch: Self, else_branch: Self) -> Self {
        let index_size = condition.index_size + then_branch.index_size + else_branch.index_size;
        Self {
            kind: IteKind::Shannon {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
            index_size,
        }
    }

    pub fn index_size(&self) -> usize {
        self.index_size
    }

    pub fn kind(&self) -> &IteKind {
        &self.kind
    }

    pub fn evaluate(&self, inputs: &BTreeMap<String, bool>) -> ActIteNewResult<bool> {
        match &self.kind {
            IteKind::Terminal(value) => Ok(*value),
            IteKind::Literal { name } => inputs
                .get(name)
                .copied()
                .ok_or_else(|| ActIteNewError::MissingInput { name: name.clone() }),
            IteKind::Shannon {
                condition,
                then_branch,
                else_branch,
            } => {
                if condition.evaluate(inputs)? {
                    then_branch.evaluate(inputs)
                } else {
                    else_branch.evaluate(inputs)
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteKind {
    Terminal(bool),
    Literal {
        name: String,
    },
    Shannon {
        condition: Box<IteVertex>,
        then_branch: Box<IteVertex>,
        else_branch: Box<IteVertex>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActIteNewError {
    InvalidPhase {
        operation: &'static str,
        fanin: String,
        phase: InputPhase,
    },
    EmptyInputName,
    EmptyNode,
    MissingInput {
        name: String,
    },
    MissingNativePorts {
        operation: &'static str,
    },
}

impl fmt::Display for ActIteNewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPhase {
                operation,
                fanin,
                phase,
            } => write!(
                f,
                "{operation} requires positive or negative unate input; {fanin} is {phase:?}"
            ),
            Self::EmptyInputName => write!(f, "ITE literal input name cannot be empty"),
            Self::EmptyNode => write!(f, "ITE construction requires at least one literal input"),
            Self::MissingInput { name } => write!(f, "missing value for ITE input {name}"),
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} is blocked by unported SIS dependencies")
            }
        }
    }
}

impl Error for ActIteNewError {}

pub type ActIteNewResult<T> = Result<T, ActIteNewError>;

pub fn ite_new_ite_for_cubenode(node: &LiteralNode) -> ActIteNewResult<IteVertex> {
    let order_list = single_cube_order(node)?;
    let mut vertex = None;
    for fanin in order_list.iter().rev() {
        vertex = Some(ite_new_ite_and(vertex, fanin)?);
    }

    let mut vertex = vertex.ok_or(ActIteNewError::EmptyNode)?;
    vertex.index_size = node.fanin_count();
    Ok(vertex)
}

pub fn ite_new_ite_for_single_literal_cubes(node: &LiteralNode) -> ActIteNewResult<IteVertex> {
    let order_list = or_literal_order(node)?;
    let mut vertex = None;
    for fanin in order_list.iter().rev() {
        vertex = Some(ite_new_ite_or(vertex, fanin)?);
    }

    let mut vertex = vertex.ok_or(ActIteNewError::EmptyNode)?;
    vertex.index_size = node.fanin_count();
    Ok(vertex)
}

pub fn ite_new_ite_and(
    vertex: Option<IteVertex>,
    fanin: &LiteralInput,
) -> ActIteNewResult<IteVertex> {
    let vertex_if = ite_new_literal(fanin)?;
    match (vertex, fanin.phase) {
        (None, InputPhase::PositiveUnate) => Ok(my_shannon_ite(vertex_if, ite_one(), ite_zero())),
        (None, InputPhase::NegativeUnate) => Ok(my_shannon_ite(vertex_if, ite_zero(), ite_one())),
        (Some(vertex), InputPhase::PositiveUnate) => {
            Ok(my_shannon_ite(vertex_if, vertex, ite_zero()))
        }
        (Some(vertex), InputPhase::NegativeUnate) => {
            Ok(my_shannon_ite(vertex_if, ite_zero(), vertex))
        }
        (_, phase) => Err(invalid_phase("ite_new_ite_and", fanin, phase)),
    }
}

pub fn ite_new_ite_or(
    vertex: Option<IteVertex>,
    fanin: &LiteralInput,
) -> ActIteNewResult<IteVertex> {
    let vertex_if = ite_new_literal(fanin)?;
    match (vertex, fanin.phase) {
        (None, InputPhase::PositiveUnate) => Ok(my_shannon_ite(vertex_if, ite_one(), ite_zero())),
        (None, InputPhase::NegativeUnate) => Ok(my_shannon_ite(vertex_if, ite_zero(), ite_one())),
        (Some(vertex), InputPhase::PositiveUnate) => {
            Ok(my_shannon_ite(vertex_if, ite_one(), vertex))
        }
        (Some(vertex), InputPhase::NegativeUnate) => {
            Ok(my_shannon_ite(vertex_if, vertex, ite_one()))
        }
        (_, phase) => Err(invalid_phase("ite_new_ite_or", fanin, phase)),
    }
}

pub fn ite_new_literal(fanin: &LiteralInput) -> ActIteNewResult<IteVertex> {
    if fanin.name.is_empty() {
        return Err(ActIteNewError::EmptyInputName);
    }

    Ok(IteVertex::literal(fanin.name.clone()))
}

pub fn single_cube_order(node: &LiteralNode) -> ActIteNewResult<Vec<LiteralInput>> {
    staged_order(
        node,
        &[
            InputPhase::PositiveUnate,
            InputPhase::PositiveUnate,
            InputPhase::NegativeUnate,
            InputPhase::NegativeUnate,
        ],
    )
}

pub fn or_literal_order(node: &LiteralNode) -> ActIteNewResult<Vec<LiteralInput>> {
    staged_order(
        node,
        &[
            InputPhase::PositiveUnate,
            InputPhase::NegativeUnate,
            InputPhase::PositiveUnate,
            InputPhase::PositiveUnate,
        ],
    )
}

pub fn ite_new_ite_for_cubenode_blocked<Node>(_node: &Node) -> ActIteNewResult<IteVertex> {
    Err(missing_native_ports(
        "ite_new_ite_for_cubenode SIS node/order integration",
    ))
}

pub fn ite_new_ite_for_single_literal_cubes_blocked<Node>(
    _node: &Node,
) -> ActIteNewResult<IteVertex> {
    Err(missing_native_ports(
        "ite_new_ite_for_single_literal_cubes SIS node/order integration",
    ))
}

fn staged_order(
    node: &LiteralNode,
    stage_phase: &[InputPhase; 4],
) -> ActIteNewResult<Vec<LiteralInput>> {
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for fanin in node.fanins().iter().rev() {
        match fanin.phase {
            InputPhase::PositiveUnate => positive.push(fanin.clone()),
            InputPhase::NegativeUnate => negative.push(fanin.clone()),
            phase => return Err(invalid_phase("literal ordering", fanin, phase)),
        }
    }

    let mut pointer_pos = 0usize;
    let mut pointer_neg = 0usize;
    let mut stage = 0usize;
    let mut order_rev = Vec::with_capacity(node.fanin_count());

    for i in 0..node.fanin_count() {
        match stage_phase[stage] {
            InputPhase::PositiveUnate => {
                if pointer_pos >= positive.len() {
                    append_remaining(&mut order_rev, &negative, &mut pointer_neg, i, node)?;
                    break;
                }
                order_rev.push(positive[pointer_pos].clone());
                pointer_pos += 1;
            }
            InputPhase::NegativeUnate => {
                if pointer_neg >= negative.len() {
                    append_remaining(&mut order_rev, &positive, &mut pointer_pos, i, node)?;
                    break;
                }
                order_rev.push(negative[pointer_neg].clone());
                pointer_neg += 1;
            }
            phase => {
                return Err(ActIteNewError::InvalidPhase {
                    operation: "literal ordering stage",
                    fanin: String::new(),
                    phase,
                });
            }
        }

        if stage % 3 == 0 {
            stage = 1;
        } else {
            stage += 1;
        }
    }

    if order_rev.len() != node.fanin_count() {
        return Err(ActIteNewError::MissingNativePorts {
            operation: "literal ordering produced an incomplete fanin order",
        });
    }

    order_rev.reverse();
    Ok(order_rev)
}

fn append_remaining(
    order_rev: &mut Vec<LiteralInput>,
    source: &[LiteralInput],
    pointer: &mut usize,
    current_index: usize,
    node: &LiteralNode,
) -> ActIteNewResult<()> {
    for _ in current_index..node.fanin_count() {
        let fanin = source
            .get(*pointer)
            .ok_or(ActIteNewError::MissingNativePorts {
                operation: "literal ordering exhausted one phase class",
            })?;
        order_rev.push(fanin.clone());
        *pointer += 1;
    }
    Ok(())
}

fn ite_zero() -> IteVertex {
    IteVertex::terminal(false)
}

fn ite_one() -> IteVertex {
    IteVertex::terminal(true)
}

fn my_shannon_ite(
    condition: IteVertex,
    then_branch: IteVertex,
    else_branch: IteVertex,
) -> IteVertex {
    IteVertex::shannon(condition, then_branch, else_branch)
}

fn invalid_phase(
    operation: &'static str,
    fanin: &LiteralInput,
    phase: InputPhase,
) -> ActIteNewError {
    ActIteNewError::InvalidPhase {
        operation,
        fanin: fanin.name.clone(),
        phase,
    }
}

fn missing_native_ports(operation: &'static str) -> ActIteNewError {
    ActIteNewError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(name: &str, phase: InputPhase) -> LiteralInput {
        LiteralInput::new(name, phase)
    }

    fn inputs(values: &[(&str, bool)]) -> BTreeMap<String, bool> {
        values
            .iter()
            .map(|(name, value)| ((*name).to_owned(), *value))
            .collect()
    }

    #[test]
    fn cube_order_matches_c_stage_pattern() {
        let node = LiteralNode::new([
            lit("a", InputPhase::PositiveUnate),
            lit("b", InputPhase::NegativeUnate),
            lit("c", InputPhase::PositiveUnate),
            lit("d", InputPhase::NegativeUnate),
            lit("e", InputPhase::PositiveUnate),
        ]);

        let names: Vec<_> = single_cube_order(&node)
            .unwrap()
            .into_iter()
            .map(|fanin| fanin.name)
            .collect();

        assert_eq!(names, ["a", "b", "d", "c", "e"]);
    }

    #[test]
    fn or_literal_order_matches_c_stage_pattern() {
        let node = LiteralNode::new([
            lit("a", InputPhase::PositiveUnate),
            lit("b", InputPhase::NegativeUnate),
            lit("c", InputPhase::PositiveUnate),
            lit("d", InputPhase::NegativeUnate),
            lit("e", InputPhase::PositiveUnate),
        ]);

        let names: Vec<_> = or_literal_order(&node)
            .unwrap()
            .into_iter()
            .map(|fanin| fanin.name)
            .collect();

        assert_eq!(names, ["b", "a", "c", "d", "e"]);
    }

    #[test]
    fn cube_ite_evaluates_as_and_of_phased_literals() {
        let node = LiteralNode::new([
            lit("a", InputPhase::PositiveUnate),
            lit("b", InputPhase::NegativeUnate),
            lit("c", InputPhase::PositiveUnate),
        ]);
        let ite = ite_new_ite_for_cubenode(&node).unwrap();

        assert_eq!(ite.index_size(), 3);
        assert!(
            ite.evaluate(&inputs(&[("a", true), ("b", false), ("c", true)]))
                .unwrap()
        );
        assert!(
            !ite.evaluate(&inputs(&[("a", true), ("b", true), ("c", true)]))
                .unwrap()
        );
        assert!(
            !ite.evaluate(&inputs(&[("a", false), ("b", false), ("c", true)]))
                .unwrap()
        );
    }

    #[test]
    fn single_literal_cube_ite_evaluates_as_or_of_phased_literals() {
        let node = LiteralNode::new([
            lit("a", InputPhase::PositiveUnate),
            lit("b", InputPhase::NegativeUnate),
            lit("c", InputPhase::PositiveUnate),
        ]);
        let ite = ite_new_ite_for_single_literal_cubes(&node).unwrap();

        assert_eq!(ite.index_size(), 3);
        assert!(
            ite.evaluate(&inputs(&[("a", true), ("b", true), ("c", false)]))
                .unwrap()
        );
        assert!(
            ite.evaluate(&inputs(&[("a", false), ("b", false), ("c", false)]))
                .unwrap()
        );
        assert!(
            !ite.evaluate(&inputs(&[("a", false), ("b", true), ("c", false)]))
                .unwrap()
        );
    }

    #[test]
    fn and_and_or_helpers_preserve_terminal_base_cases() {
        let a = lit("a", InputPhase::PositiveUnate);
        let not_b = lit("b", InputPhase::NegativeUnate);

        let and_ite = ite_new_ite_and(Some(ite_new_ite_and(None, &a).unwrap()), &not_b).unwrap();
        let or_ite = ite_new_ite_or(Some(ite_new_ite_or(None, &a).unwrap()), &not_b).unwrap();

        assert!(
            and_ite
                .evaluate(&inputs(&[("a", true), ("b", false)]))
                .unwrap()
        );
        assert!(
            !and_ite
                .evaluate(&inputs(&[("a", true), ("b", true)]))
                .unwrap()
        );
        assert!(
            or_ite
                .evaluate(&inputs(&[("a", false), ("b", false)]))
                .unwrap()
        );
        assert!(
            !or_ite
                .evaluate(&inputs(&[("a", false), ("b", true)]))
                .unwrap()
        );
    }

    #[test]
    fn binate_and_unknown_phases_return_diagnostics() {
        let node = LiteralNode::new([
            lit("a", InputPhase::PositiveUnate),
            lit("x", InputPhase::Binate),
        ]);

        assert!(matches!(
            ite_new_ite_for_cubenode(&node),
            Err(ActIteNewError::InvalidPhase {
                phase: InputPhase::Binate,
                ..
            })
        ));

        assert!(matches!(
            ite_new_ite_or(None, &lit("x", InputPhase::Unknown)),
            Err(ActIteNewError::InvalidPhase {
                phase: InputPhase::Unknown,
                ..
            })
        ));
    }
}
