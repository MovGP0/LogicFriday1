//! Native formatting for SIS Boolean nodes.
//!
//! The legacy printer wrote directly to a `FILE *`. This port returns owned
//! strings and keeps the graph-dependent primary-output skip rule explicit.

#[cfg(test)]
#[path = "node.rs"]
mod native_node;

#[cfg(test)]
use self::native_node::{
    Node, NodeError, NodeFunction, NodeResult, NodeType, node_and, node_constant, node_function,
    node_literal, node_not, node_or, node_sort_for_printing,
};

#[cfg(not(test))]
use super::node::{
    Node, NodeError, NodeFunction, NodeResult, NodeType, node_function, node_not,
    node_sort_for_printing,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimaryOutputFaninKind {
    PrimaryInput,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodePrintOptions {
    pub primary_output_fanin: Option<PrimaryOutputFaninKind>,
}

impl NodePrintOptions {
    pub const fn new() -> Self {
        Self {
            primary_output_fanin: None,
        }
    }

    pub const fn with_primary_output_fanin(primary_output_fanin: PrimaryOutputFaninKind) -> Self {
        Self {
            primary_output_fanin: Some(primary_output_fanin),
        }
    }
}

impl Default for NodePrintOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_node_definition(node: &Node) -> NodeResult<Option<String>> {
    format_node_definition_with_options(node, NodePrintOptions::default())
}

pub fn format_node_definition_with_options(
    node: &Node,
    options: NodePrintOptions,
) -> NodeResult<Option<String>> {
    if should_skip_definition(node, options) {
        return Ok(None);
    }

    let sorted = node_sort_for_printing(node)?;
    let mut output = String::new();
    output.push_str("     ");
    output.push_str(node_name(node));
    output.push_str(" = ");
    write_node_expression(&mut output, &sorted, true)?;
    output.push('\n');

    Ok(Some(output))
}

pub fn format_node_negative_definition(node: &Node) -> NodeResult<Option<String>> {
    format_node_negative_definition_with_options(node, NodePrintOptions::default())
}

pub fn format_node_negative_definition_with_options(
    node: &Node,
    options: NodePrintOptions,
) -> NodeResult<Option<String>> {
    if should_skip_definition(node, options) {
        return Ok(None);
    }

    let node_to_print = if node.node_type == NodeType::PrimaryOutput {
        node.clone()
    } else {
        node_not(node)?
    };

    let sorted = node_sort_for_printing(&node_to_print)?;
    let mut output = String::new();
    output.push_str("     ");
    output.push_str(node_name(node));
    output.push_str(" = ");
    write_node_expression(&mut output, &sorted, false)?;
    output.push('\n');

    Ok(Some(output))
}

pub fn format_node_rhs(node: &Node) -> NodeResult<String> {
    let sorted = node_sort_for_printing(node)?;
    let mut output = String::new();
    write_node_expression(&mut output, &sorted, true)?;
    Ok(output)
}

fn should_skip_definition(node: &Node, options: NodePrintOptions) -> bool {
    match node.node_type {
        NodeType::PrimaryInput => true,
        NodeType::PrimaryOutput => {
            options.primary_output_fanin == Some(PrimaryOutputFaninKind::Other)
        }
        NodeType::Internal => false,
    }
}

fn write_node_expression(output: &mut String, node: &Node, phase: bool) -> NodeResult<()> {
    if write_simple_expression(output, node, phase)? {
        return Ok(());
    }

    let function = node.function().ok_or(NodeError::MissingFunction {
        operation: "node print",
    })?;

    if !phase {
        output.push('(');
    }

    for (cube_index, cube) in function.cubes().iter().enumerate() {
        if cube_index != 0 {
            output.push_str(" + ");
        }

        let mut first_literal = true;
        for (input_index, input) in cube.inputs().iter().enumerate() {
            let Some(input_phase) = input else {
                continue;
            };

            if !first_literal {
                output.push(' ');
            }

            first_literal = false;
            if let Some(fanin) = node.fanins.get(input_index) {
                output.push_str(fanin);
            }

            if !input_phase {
                output.push('\'');
            }
        }
    }

    if !phase {
        output.push_str(")'");
    }

    Ok(())
}

fn write_simple_expression(output: &mut String, node: &Node, phase: bool) -> NodeResult<bool> {
    match node.node_type {
        NodeType::PrimaryInput => Ok(true),
        NodeType::PrimaryOutput => {
            if let Some(fanin) = node.fanins.first() {
                output.push_str(fanin);
            }

            Ok(true)
        }
        NodeType::Internal => write_simple_internal_expression(output, node, phase),
    }
}

fn write_simple_internal_expression(
    output: &mut String,
    node: &Node,
    phase: bool,
) -> NodeResult<bool> {
    match node_function(node)? {
        NodeFunction::Zero => {
            output.push_str(if phase { "-0-" } else { "-1-" });
            Ok(true)
        }
        NodeFunction::One => {
            output.push_str(if phase { "-1-" } else { "-0-" });
            Ok(true)
        }
        NodeFunction::Buffer => {
            write_single_fanin_expression(output, node, !phase);
            Ok(true)
        }
        NodeFunction::Inverter => {
            write_single_fanin_expression(output, node, phase);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn write_single_fanin_expression(output: &mut String, node: &Node, inverted: bool) {
    if let Some(fanin) = node.fanins.first() {
        output.push_str(fanin);
    }

    if inverted {
        output.push('\'');
    }
}

fn node_name(node: &Node) -> &str {
    node.name
        .as_deref()
        .or(node.short_name.as_deref())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(mut node: Node, name: &str) -> Node {
        node.name = Some(name.to_owned());
        node.short_name = Some(name.to_owned());
        node
    }

    fn literal(name: &str, phase: i32) -> Node {
        node_literal(name, phase).unwrap()
    }

    #[test]
    fn definitions_skip_primary_inputs() {
        assert_eq!(
            format_node_definition(&Node::primary_input("a")).unwrap(),
            None
        );
    }

    #[test]
    fn definitions_print_simple_constants_and_literals() {
        assert_eq!(
            format_node_definition(&named(node_constant(0).unwrap(), "f")).unwrap(),
            Some("     f = -0-\n".to_owned())
        );
        assert_eq!(
            format_node_negative_definition(&named(node_constant(0).unwrap(), "f")).unwrap(),
            Some("     f = -0-\n".to_owned())
        );
        assert_eq!(
            format_node_definition(&named(literal("a", 1), "f")).unwrap(),
            Some("     f = a\n".to_owned())
        );
        assert_eq!(
            format_node_negative_definition(&named(literal("a", 1), "f")).unwrap(),
            Some("     f = a\n".to_owned())
        );
    }

    #[test]
    fn rhs_sorts_fanins_and_cubes_for_stable_output() {
        let left = node_and(&literal("b", 1), &literal("a", 0)).unwrap();
        let right = node_and(&literal("a", 1), &literal("b", 0)).unwrap();
        let sum = node_or(&left, &right).unwrap();

        assert_eq!(format_node_rhs(&sum).unwrap(), "a' b + a b'");
    }

    #[test]
    fn negative_complex_expression_wraps_complemented_cover() {
        let left = node_and(&literal("a", 1), &literal("b", 1)).unwrap();
        let right = node_and(&literal("a", 0), &literal("b", 0)).unwrap();
        let xnor = named(node_or(&left, &right).unwrap(), "f");

        assert_eq!(
            format_node_negative_definition(&xnor).unwrap(),
            Some("     f = (a' b + a b')'\n".to_owned())
        );
    }

    #[test]
    fn primary_outputs_print_only_when_fanin_is_a_primary_input() {
        let output = Node::primary_output("out", "a");
        let options =
            NodePrintOptions::with_primary_output_fanin(PrimaryOutputFaninKind::PrimaryInput);

        assert_eq!(
            format_node_definition_with_options(&output, options).unwrap(),
            Some("     out = a\n".to_owned())
        );
        assert_eq!(
            format_node_definition_with_options(
                &output,
                NodePrintOptions::with_primary_output_fanin(PrimaryOutputFaninKind::Other),
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("print.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
