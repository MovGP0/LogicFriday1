//! Native Rust scaffold for `LogicSynthesis/sis/test/example.c`.
//!
//! The C source is sample SIS command code. It parses `test [-c] n1 n2`,
//! resolves exactly two network nodes through `com_get_nodes`, rewrites `n1`
//! as `(quotient(n1 / n2) & n2) | remainder`, and optionally repeats the
//! rewrite using the complement of `n2`.
//!
//! The command cannot be completed as native Rust until the SIS command,
//! network, and node layers have native APIs. This module intentionally avoids
//! legacy per-file C ABI exports and records the blocked behavior in a small
//! Rust API that can be replaced by the real implementation once those layers
//! are available.

pub const USAGE: &str = "usage: test [-c] n1 n2\n    -c\t\tuse complement of n2 in division\n";

pub const REQUIRED_PORT_BEADS: &[&str] = &[
    "LogicFriday1-8j8.2.6.118", // command/get_nodes.c: com_get_nodes
    "LogicFriday1-8j8.2.6.312", // node/divide.c: node_div
    "LogicFriday1-8j8.2.6.314", // node/invert.c: node_not
    "LogicFriday1-8j8.2.6.318", // node/node.c: node_and, node_or, node_literal
    "LogicFriday1-8j8.2.6.321", // node/nodemisc.c: node_free
    "LogicFriday1-8j8.2.6.322", // node/nodeutil.c and shared node helpers
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExampleCommand {
    pub target_node: String,
    pub divisor_node: String,
    pub use_complement: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExampleCommandError {
    UnsupportedOption(String),
    WrongArity { actual: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExamplePortDisposition {
    BlockedByUnportedNodeAndCommandApis,
}

pub fn example_port_disposition() -> ExamplePortDisposition {
    ExamplePortDisposition::BlockedByUnportedNodeAndCommandApis
}

pub fn example_port_is_blocked() -> bool {
    example_port_disposition() == ExamplePortDisposition::BlockedByUnportedNodeAndCommandApis
}

pub fn parse_example_command_args<I, S>(args: I) -> Result<ExampleCommand, ExampleCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut use_complement = false;
    let mut operands = Vec::new();

    for arg in args {
        let arg = arg.as_ref();
        match arg {
            "-c" => use_complement = true,
            option if option.starts_with('-') => {
                return Err(ExampleCommandError::UnsupportedOption(option.to_owned()));
            }
            operand => operands.push(operand.to_owned()),
        }
    }

    match operands.as_slice() {
        [target_node, divisor_node] => Ok(ExampleCommand {
            target_node: target_node.clone(),
            divisor_node: divisor_node.clone(),
            use_complement,
        }),
        _ => Err(ExampleCommandError::WrongArity {
            actual: operands.len(),
        }),
    }
}

pub fn required_port_beads() -> &'static [&'static str] {
    REQUIRED_PORT_BEADS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_required_nodes_without_complement() {
        assert_eq!(
            parse_example_command_args(["n1", "n2"]),
            Ok(ExampleCommand {
                target_node: "n1".to_owned(),
                divisor_node: "n2".to_owned(),
                use_complement: false,
            })
        );
    }

    #[test]
    fn parses_complement_flag() {
        assert_eq!(
            parse_example_command_args(["-c", "n1", "n2"])
                .unwrap()
                .use_complement,
            true
        );
    }

    #[test]
    fn rejects_unknown_options_and_wrong_arity() {
        assert_eq!(
            parse_example_command_args(["-x", "n1", "n2"]),
            Err(ExampleCommandError::UnsupportedOption("-x".to_owned()))
        );
        assert_eq!(
            parse_example_command_args(["n1"]),
            Err(ExampleCommandError::WrongArity { actual: 1 })
        );
    }

    #[test]
    fn reports_blocked_native_port_dependencies() {
        assert!(example_port_is_blocked());
        assert!(required_port_beads().contains(&"LogicFriday1-8j8.2.6.118"));
        assert!(required_port_beads().contains(&"LogicFriday1-8j8.2.6.312"));
    }
}
