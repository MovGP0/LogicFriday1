//! Native EQN writer for SIS networks.
//!
//! The legacy writer emits primary-input/output order declarations followed by
//! equations for nodes that survive SIS output-name folding. This port keeps
//! the formatting and printable-node rules as owned Rust APIs.

use crate::ports::network::network_util::{
    BoolExpr, CoverValue, Network, NetworkUtilError, NodeId, NodeKind,
};

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EqnWriteOptions {
    pub short_names: bool,
    pub slif_literals: bool,
    pub line_width: usize,
}

impl Default for EqnWriteOptions {
    fn default() -> Self {
        Self {
            short_names: false,
            slif_literals: false,
            line_width: 78,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EqnWriteError {
    Network(NetworkUtilError),
    PrimaryOutputWithoutFanin(NodeId),
    InvalidCoverWidth {
        node: NodeId,
        cube: usize,
        expected: usize,
        actual: usize,
    },
    UnsupportedNodeFunction(NodeId),
}

impl fmt::Display for EqnWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(f, "{error}"),
            Self::PrimaryOutputWithoutFanin(node) => {
                write!(f, "primary output {} has no fanin", node.index())
            }
            Self::InvalidCoverWidth {
                node,
                cube,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "node {} cube {cube} has {actual} literals, expected {expected}",
                    node.index()
                )
            }
            Self::UnsupportedNodeFunction(node) => {
                write!(
                    f,
                    "node {} has no printable SOP or expression",
                    node.index()
                )
            }
        }
    }
}

impl Error for EqnWriteError {}

impl From<NetworkUtilError> for EqnWriteError {
    fn from(value: NetworkUtilError) -> Self {
        Self::Network(value)
    }
}

pub type EqnWriteResult<T> = Result<T, EqnWriteError>;

pub fn write_eqn(network: &Network, short_names: bool) -> EqnWriteResult<String> {
    write_eqn_with_options(
        network,
        EqnWriteOptions {
            short_names,
            ..EqnWriteOptions::default()
        },
    )
}

pub fn write_eqn_with_options(
    network: &Network,
    options: EqnWriteOptions,
) -> EqnWriteResult<String> {
    let mut writer = BreakingWriter::new(options.line_width, "\n");

    writer.push("\n");
    writer.push("INORDER =");
    for input in network.primary_inputs() {
        writer.push_char(' ');
        write_name(&mut writer, network, *input, options.short_names)?;
    }
    writer.push(";\n");

    writer.push("OUTORDER =");
    for output in network.primary_outputs() {
        writer.push_char(' ');
        write_name(&mut writer, network, *output, options.short_names)?;
    }
    writer.push(";\n");

    for (id, _) in network.nodes() {
        write_sop_to(&mut writer, network, id, options)?;
    }

    Ok(writer.finish())
}

pub fn write_sop(
    network: &Network,
    node: NodeId,
    short_names: bool,
    slif_literals: bool,
) -> EqnWriteResult<String> {
    let mut writer = BreakingWriter::new(78, "\n");
    write_sop_to(
        &mut writer,
        network,
        node,
        EqnWriteOptions {
            short_names,
            slif_literals,
            line_width: 78,
        },
    )?;
    Ok(writer.finish())
}

fn write_sop_to(
    writer: &mut BreakingWriter,
    network: &Network,
    node: NodeId,
    options: EqnWriteOptions,
) -> EqnWriteResult<()> {
    if !node_should_be_printed(network, node)? {
        return Ok(());
    }

    write_name(writer, network, node, options.short_names)?;
    writer.push(" = ");

    let data = network.node(node)?;
    if data.kind == NodeKind::PrimaryOutput {
        let fanin = data
            .fanins
            .first()
            .copied()
            .ok_or(EqnWriteError::PrimaryOutputWithoutFanin(node))?;
        write_name(writer, network, fanin, options.short_names)?;
        writer.push(";\n");
        return Ok(());
    }

    if let Some(cover) = &data.cover {
        if cover.is_empty() {
            writer.push("0;\n");
            return Ok(());
        }

        if cover.cubes().len() == 1
            && cover.cubes()[0]
                .values()
                .iter()
                .all(|value| *value == CoverValue::DontCare)
        {
            writer.push("1;\n");
            return Ok(());
        }

        for (cube_index, cube) in cover.cubes().iter().enumerate().rev() {
            if cube.values().len() != data.fanins.len() {
                return Err(EqnWriteError::InvalidCoverWidth {
                    node,
                    cube: cube_index,
                    expected: data.fanins.len(),
                    actual: cube.values().len(),
                });
            }

            if cube_index + 1 != cover.cubes().len() {
                writer.push(" + ");
            }

            let mut first_literal = true;
            for (fanin_index, value) in cube.values().iter().enumerate() {
                if *value == CoverValue::DontCare {
                    continue;
                }

                write_literal_separator(writer, &mut first_literal, options.slif_literals);

                if !options.slif_literals && *value == CoverValue::Zero {
                    writer.push_char('!');
                }

                write_name(
                    writer,
                    network,
                    data.fanins[fanin_index],
                    options.short_names,
                )?;

                if options.slif_literals && *value == CoverValue::Zero {
                    writer.push_char('\'');
                }
            }
        }

        writer.push(";\n");
        return Ok(());
    }

    if let Some(expression) = &data.expression {
        write_expression(writer, network, expression, options.short_names)?;
        writer.push(";\n");
        return Ok(());
    }

    Err(EqnWriteError::UnsupportedNodeFunction(node))
}

fn write_literal_separator(
    writer: &mut BreakingWriter,
    first_literal: &mut bool,
    slif_literals: bool,
) {
    if *first_literal {
        *first_literal = false;
        return;
    }

    writer.push_char(if slif_literals { ' ' } else { '*' });
}

fn write_expression(
    writer: &mut BreakingWriter,
    network: &Network,
    expression: &BoolExpr,
    short_names: bool,
) -> EqnWriteResult<()> {
    match expression {
        BoolExpr::Constant(false) => writer.push("0"),
        BoolExpr::Constant(true) => writer.push("1"),
        BoolExpr::Literal { node, phase } => {
            if !*phase {
                writer.push_char('!');
            }
            write_name(writer, network, *node, short_names)?;
        }
        BoolExpr::Not(inner) => {
            writer.push_char('!');
            write_parenthesized_expression(writer, network, inner, short_names)?;
        }
        BoolExpr::And(items) => {
            write_joined_expression(writer, network, items, "*", short_names)?;
        }
        BoolExpr::Or(items) => {
            write_joined_expression(writer, network, items, " + ", short_names)?;
        }
    }

    Ok(())
}

fn write_joined_expression(
    writer: &mut BreakingWriter,
    network: &Network,
    items: &[BoolExpr],
    separator: &str,
    short_names: bool,
) -> EqnWriteResult<()> {
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            writer.push(separator);
        }
        write_parenthesized_expression(writer, network, item, short_names)?;
    }

    Ok(())
}

fn write_parenthesized_expression(
    writer: &mut BreakingWriter,
    network: &Network,
    expression: &BoolExpr,
    short_names: bool,
) -> EqnWriteResult<()> {
    match expression {
        BoolExpr::Constant(_) | BoolExpr::Literal { .. } => {
            write_expression(writer, network, expression, short_names)?;
        }
        _ => {
            writer.push_char('(');
            write_expression(writer, network, expression, short_names)?;
            writer.push_char(')');
        }
    }

    Ok(())
}

fn write_name(
    writer: &mut BreakingWriter,
    network: &Network,
    node: NodeId,
    short_names: bool,
) -> EqnWriteResult<()> {
    writer.push(&io_name(network, node, short_names)?);
    Ok(())
}

fn io_name(network: &Network, node: NodeId, short_names: bool) -> EqnWriteResult<String> {
    let mut printable = node;
    let data = network.node(node)?;

    if data.kind == NodeKind::PrimaryOutput {
        printable = node;
    } else if data.kind != NodeKind::PrimaryInput {
        if let Some(only_output) = single_primary_output_fanout(network, node)? {
            printable = only_output;
        }
    }

    let data = network.node(printable)?;
    Ok(if short_names {
        data.short_name.clone()
    } else {
        data.name.clone()
    })
}

fn node_should_be_printed(network: &Network, node: NodeId) -> EqnWriteResult<bool> {
    let data = network.node(node)?;

    if data.kind == NodeKind::PrimaryInput {
        return Ok(false);
    }

    if data.kind == NodeKind::PrimaryOutput {
        let fanin = data
            .fanins
            .first()
            .copied()
            .ok_or(EqnWriteError::PrimaryOutputWithoutFanin(node))?;
        let fanin_data = network.node(fanin)?;
        let real_output_fanouts = primary_output_fanout_count(network, fanin)?;

        if fanin_data.kind != NodeKind::PrimaryInput && real_output_fanouts == 1 {
            return Ok(false);
        }
    }

    Ok(true)
}

fn primary_output_fanout_count(network: &Network, node: NodeId) -> EqnWriteResult<usize> {
    let data = network.node(node)?;
    let mut count = 0;

    for fanout in &data.fanouts {
        if network.node(*fanout)?.kind == NodeKind::PrimaryOutput {
            count += 1;
        }
    }

    Ok(count)
}

fn single_primary_output_fanout(network: &Network, node: NodeId) -> EqnWriteResult<Option<NodeId>> {
    let data = network.node(node)?;
    let mut output = None;

    for fanout in &data.fanouts {
        if network.node(*fanout)?.kind != NodeKind::PrimaryOutput {
            continue;
        }

        if output.is_some() {
            return Ok(None);
        }
        output = Some(*fanout);
    }

    Ok(output)
}

#[derive(Clone, Debug)]
struct BreakingWriter {
    output: String,
    column: usize,
    break_column: usize,
    break_string: &'static str,
}

impl BreakingWriter {
    fn new(break_column: usize, break_string: &'static str) -> Self {
        Self {
            output: String::new(),
            column: 0,
            break_column,
            break_string,
        }
    }

    fn push(&mut self, value: &str) {
        if self.column + value.len() > self.break_column {
            self.push_raw(self.break_string);
        }
        self.push_raw(value);
    }

    fn push_char(&mut self, value: char) {
        self.output.push(value);
        if value == '\n' {
            self.column = 0;
        } else {
            self.column += value.len_utf8();
        }
    }

    fn push_raw(&mut self, value: &str) {
        for character in value.chars() {
            self.push_char(character);
        }
    }

    fn finish(self) -> String {
        self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::network::network_util::{Cube, NetworkNode, SopCover};

    fn cube(values: &[CoverValue]) -> Cube {
        Cube::new(values.to_vec())
    }

    fn input(network: &mut Network, name: &str) -> NodeId {
        network
            .add_primary_input(NetworkNode::new(name, NodeKind::PrimaryInput))
            .unwrap()
    }

    #[test]
    fn writes_primary_orders_and_sop_equation() {
        let mut network = Network::new();
        let a = input(&mut network, "a");
        let b = input(&mut network, "b");
        let y = network
            .add_internal(
                "n",
                [a, b],
                SopCover::new([
                    cube(&[CoverValue::One, CoverValue::DontCare]),
                    cube(&[CoverValue::Zero, CoverValue::One]),
                ]),
            )
            .unwrap();
        network.add_primary_output(y).unwrap();

        let actual = write_eqn(&network, false).unwrap();

        assert_eq!(actual, "\nINORDER = a b;\nOUTORDER = n;\nn = !a*b + a;\n");
    }

    #[test]
    fn folds_single_output_internal_node_to_output_name() {
        let mut network = Network::new();
        let a = input(&mut network, "a");
        let n = network
            .add_internal("n", [a], SopCover::new([cube(&[CoverValue::Zero])]))
            .unwrap();
        let output = network.add_primary_output(n).unwrap();
        network.change_node_name(output, "y").unwrap();

        let actual = write_eqn(&network, false).unwrap();

        assert_eq!(actual, "\nINORDER = a;\nOUTORDER = y;\ny = !a;\n");
    }

    #[test]
    fn prints_primary_output_when_it_is_driven_by_primary_input() {
        let mut network = Network::new();
        let a = input(&mut network, "a");
        let output = network.add_primary_output(a).unwrap();
        network.change_node_name(output, "y").unwrap();
        network.change_node_name(a, "a").unwrap();

        let actual = write_eqn(&network, false).unwrap();

        assert_eq!(actual, "\nINORDER = a;\nOUTORDER = y;\ny = a;\n");
    }

    #[test]
    fn prints_internal_and_outputs_when_internal_feeds_multiple_outputs() {
        let mut network = Network::new();
        let a = input(&mut network, "a");
        let n = network
            .add_internal("n", [a], SopCover::new([cube(&[CoverValue::One])]))
            .unwrap();
        let y0 = network.add_primary_output(n).unwrap();
        let y1 = network.add_primary_output(n).unwrap();
        network.change_node_name(y0, "y0").unwrap();
        network.change_node_name(y1, "y1").unwrap();
        network.change_node_name(n, "n").unwrap();

        let actual = write_eqn(&network, false).unwrap();

        assert_eq!(
            actual,
            "\nINORDER = a;\nOUTORDER = y0 y1;\nn = a;\ny0 = n;\ny1 = n;\n"
        );
    }

    #[test]
    fn writes_constants() {
        let mut network = Network::new();
        let zero = network.add_internal("zero", [], SopCover::new([])).unwrap();
        let one = network
            .add_internal("one", [], SopCover::new([cube(&[])]))
            .unwrap();
        network.add_primary_output(zero).unwrap();
        network.add_primary_output(one).unwrap();

        let actual = write_eqn(&network, false).unwrap();

        assert!(actual.contains("zero = 0;\n"));
        assert!(actual.contains("one = 1;\n"));
    }

    #[test]
    fn writes_short_names_and_slif_literals() {
        let mut network = Network::new();
        let a = input(&mut network, "alpha");
        let b = input(&mut network, "beta");
        network.change_node_short_name(a, "a").unwrap();
        network.change_node_short_name(b, "b").unwrap();
        let n = network
            .add_internal(
                "node",
                [a, b],
                SopCover::new([cube(&[CoverValue::Zero, CoverValue::One])]),
            )
            .unwrap();
        network.change_node_short_name(n, "n").unwrap();

        let actual = write_sop(&network, n, true, true).unwrap();

        assert_eq!(actual, "n = a' b;\n");
    }

    #[test]
    fn wraps_long_lines_before_string_writes() {
        let mut network = Network::new();
        let a = input(&mut network, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let b = input(&mut network, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let n = network
            .add_internal(
                "nnnnnnnnnn",
                [a, b],
                SopCover::new([cube(&[CoverValue::One, CoverValue::One])]),
            )
            .unwrap();
        network.add_primary_output(n).unwrap();

        let actual = write_eqn_with_options(
            &network,
            EqnWriteOptions {
                line_width: 42,
                ..EqnWriteOptions::default()
            },
        )
        .unwrap();

        assert!(actual.contains("INORDER = \naaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(actual.contains("*\nbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
    }

    #[test]
    fn reports_invalid_cover_width() {
        let mut network = Network::new();
        let a = input(&mut network, "a");
        let node = network
            .add_internal(
                "n",
                [a],
                SopCover::new([cube(&[CoverValue::One, CoverValue::Zero])]),
            )
            .unwrap();

        let error = write_sop(&network, node, false, false).unwrap_err();

        assert_eq!(
            error,
            EqnWriteError::InvalidCoverWidth {
                node,
                cube: 0,
                expected: 1,
                actual: 2,
            }
        );
    }
}
