//! Native Rust command planning and execution for SIS decomposition commands.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecompositionCommandKind
{
    Decomp,
    TechDecomp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration
{
    pub name: &'static str,
    pub kind: DecompositionCommandKind,
    pub changes_network: bool,
}

pub const DECOMPOSITION_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "decomp",
        kind: DecompositionCommandKind::Decomp,
        changes_network: true,
    },
    CommandRegistration {
        name: "tech_decomp",
        kind: DecompositionCommandKind::TechDecomp,
        changes_network: true,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind
{
    Internal,
    Input,
    Output,
    Constant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecompositionMethod
{
    Quick,
    Good,
    Disjoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecompCommandPlan
{
    pub method: DecompositionMethod,
    pub node_names: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TechDecompCommandPlan
{
    pub and_limit: i32,
    pub or_limit: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecompositionCommand
{
    Decomp(DecompCommandPlan),
    TechDecomp(TechDecompCommandPlan),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecompExecutionSummary
{
    pub selected_nodes: usize,
    pub decomposed_internal_nodes: usize,
    pub skipped_non_internal_nodes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandParseError
{
    MissingOptionValue(char),
    UnsupportedOption(String),
    InvalidLimit
    {
        option: char,
        value: String,
    },
    MissingTechLimit,
    UnexpectedOperand(String),
    UnknownCommand(String),
}

impl fmt::Display for CommandParseError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::InvalidLimit { option, value } => {
                write!(f, "invalid -{option} decomposition limit {value}")
            }
            Self::MissingTechLimit => f.write_str("tech_decomp requires -a or -o with a limit of at least 2"),
            Self::UnexpectedOperand(operand) => write!(f, "unexpected operand {operand}"),
            Self::UnknownCommand(command) => write!(f, "unknown decomposition command {command}"),
        }
    }
}

impl Error for CommandParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecompositionCommandError
{
    Selection(String),
    Backend(String),
}

impl fmt::Display for DecompositionCommandError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::Selection(message) | Self::Backend(message) => f.write_str(message),
        }
    }
}

impl Error for DecompositionCommandError {}

pub trait DecompCommandNetwork
{
    type NodeId: Clone;

    fn select_nodes(&self, node_names: &[String]) -> Result<Vec<Self::NodeId>, DecompositionCommandError>;

    fn node_kind(&self, node: &Self::NodeId) -> Result<NodeKind, DecompositionCommandError>;

    fn decompose_node(
        &mut self,
        node: &Self::NodeId,
        method: DecompositionMethod,
    ) -> Result<(), DecompositionCommandError>;
}

pub trait TechDecompCommandNetwork
{
    fn tech_decompose(
        &mut self,
        and_limit: i32,
        or_limit: i32,
    ) -> Result<(), DecompositionCommandError>;
}

pub fn decomposition_command_registrations() -> &'static [CommandRegistration]
{
    DECOMPOSITION_COMMANDS
}

pub fn decomp_usage() -> &'static str
{
    "usage: decomp [-dqg] [node-list]\n    -q\t\tQuick decomposition (default)\n    -g\t\tGood decomposition\n    -d\t\tDisjoint decomposition\n"
}

pub fn tech_decomp_usage() -> &'static str
{
    "usage: tech_decomp [-a and] [-o or]\n    -a and \tAnd gate with fanin limit 'and'\n    -o or \tOr gate with fanin limit 'or'\n"
}

pub fn parse_decomp_args<I, S>(args: I) -> Result<DecompCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut method = DecompositionMethod::Quick;
    let node_names = parse_options(args, "dgq", |option, _value| {
        method = match option
        {
            'q' => DecompositionMethod::Quick,
            'g' => DecompositionMethod::Good,
            'd' => DecompositionMethod::Disjoint,
            _ => return Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
        };

        Ok(())
    })?;

    Ok(DecompCommandPlan {
        method,
        node_names,
    })
}

pub fn parse_tech_decomp_args<I, S>(args: I) -> Result<TechDecompCommandPlan, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut and_limit = 0;
    let mut or_limit = 0;
    let operands = parse_options(args, "a:o:", |option, value| {
        let limit = c_atoi(&value);
        if limit < 2
        {
            return Err(CommandParseError::InvalidLimit { option, value });
        }

        match option
        {
            'a' => and_limit = limit,
            'o' => or_limit = limit,
            _ => return Err(CommandParseError::UnsupportedOption(format!("-{option}"))),
        }

        Ok(())
    })?;

    if and_limit < 2 && or_limit < 2
    {
        return Err(CommandParseError::MissingTechLimit);
    }

    if let Some(operand) = operands.into_iter().next()
    {
        return Err(CommandParseError::UnexpectedOperand(operand));
    }

    Ok(TechDecompCommandPlan {
        and_limit,
        or_limit,
    })
}

pub fn parse_decomposition_command<I, S>(
    command_name: &str,
    args: I,
) -> Result<DecompositionCommand, CommandParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name
    {
        "decomp" => parse_decomp_args(args).map(DecompositionCommand::Decomp),
        "tech_decomp" => parse_tech_decomp_args(args).map(DecompositionCommand::TechDecomp),
        _ => Err(CommandParseError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn execute_decomp_command<N>(
    network: &mut N,
    plan: &DecompCommandPlan,
) -> Result<DecompExecutionSummary, DecompositionCommandError>
where
    N: DecompCommandNetwork,
{
    let nodes = network.select_nodes(&plan.node_names)?;
    let mut summary = DecompExecutionSummary {
        selected_nodes: nodes.len(),
        decomposed_internal_nodes: 0,
        skipped_non_internal_nodes: 0,
    };

    for node in nodes
    {
        if network.node_kind(&node)? == NodeKind::Internal
        {
            network.decompose_node(&node, plan.method)?;
            summary.decomposed_internal_nodes += 1;
        }
        else
        {
            summary.skipped_non_internal_nodes += 1;
        }
    }

    Ok(summary)
}

pub fn execute_tech_decomp_command<N>(
    network: &mut N,
    plan: &TechDecompCommandPlan,
) -> Result<(), DecompositionCommandError>
where
    N: TechDecompCommandNetwork,
{
    network.tech_decompose(plan.and_limit, plan.or_limit)
}

fn parse_options<F>(
    args: impl IntoIterator<Item = impl AsRef<str>>,
    spec: &str,
    mut apply: F,
) -> Result<Vec<String>, CommandParseError>
where
    F: FnMut(char, String) -> Result<(), CommandParseError>,
{
    let mut iter = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .peekable();
    let mut operands = Vec::new();
    let mut scanning_options = true;

    while let Some(arg) = iter.next()
    {
        if !scanning_options || !arg.starts_with('-') || arg == "-"
        {
            operands.push(arg);
            operands.extend(iter);
            break;
        }

        if arg == "--"
        {
            scanning_options = false;
            continue;
        }

        let mut chars = arg[1..].char_indices().peekable();
        while let Some((offset, option)) = chars.next()
        {
            let needs_value = option_needs_value(spec, option)
                .ok_or_else(|| CommandParseError::UnsupportedOption(format!("-{option}")))?;
            if needs_value
            {
                let value_start = offset + option.len_utf8();
                let value = if value_start < arg[1..].len()
                {
                    arg[1 + value_start..].to_owned()
                }
                else
                {
                    iter.next()
                        .ok_or(CommandParseError::MissingOptionValue(option))?
                };
                apply(option, value)?;
                break;
            }
            else
            {
                apply(option, String::new())?;
            }
        }
    }

    Ok(operands)
}

fn option_needs_value(spec: &str, option: char) -> Option<bool>
{
    let mut chars = spec.chars().peekable();
    while let Some(candidate) = chars.next()
    {
        if candidate == ':'
        {
            continue;
        }

        let has_value = chars.peek() == Some(&':');
        if candidate == option
        {
            return Some(has_value);
        }
    }

    None
}

fn c_atoi(value: &str) -> i32
{
    let trimmed = value.trim_start();
    let mut chars = trimmed.chars().peekable();
    let mut sign = 1;

    match chars.peek().copied()
    {
        Some('-') => {
            sign = -1;
            chars.next();
        }
        Some('+') => {
            chars.next();
        }
        _ => {}
    }

    let mut result = 0i32;
    let mut saw_digit = false;
    for ch in chars
    {
        let Some(digit) = ch.to_digit(10) else
        {
            break;
        };
        saw_digit = true;
        result = result.saturating_mul(10).saturating_add(digit as i32);
    }

    if saw_digit
    {
        result.saturating_mul(sign)
    }
    else
    {
        0
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNode
    {
        name: String,
        kind: NodeKind,
    }

    impl TestNode
    {
        fn new(name: &str, kind: NodeKind) -> Self
        {
            Self {
                name: name.to_owned(),
                kind,
            }
        }
    }

    #[derive(Default)]
    struct TestNetwork
    {
        nodes: Vec<TestNode>,
        decomposed: Vec<(usize, DecompositionMethod)>,
        tech_calls: Vec<(i32, i32)>,
    }

    impl TestNetwork
    {
        fn sample() -> Self
        {
            Self {
                nodes: vec![
                    TestNode::new("a", NodeKind::Input),
                    TestNode::new("f", NodeKind::Internal),
                    TestNode::new("g", NodeKind::Internal),
                    TestNode::new("y", NodeKind::Output),
                ],
                decomposed: Vec::new(),
                tech_calls: Vec::new(),
            }
        }
    }

    impl DecompCommandNetwork for TestNetwork
    {
        type NodeId = usize;

        fn select_nodes(&self, node_names: &[String]) -> Result<Vec<Self::NodeId>, DecompositionCommandError>
        {
            if node_names.is_empty()
            {
                return Ok((0..self.nodes.len()).collect());
            }

            node_names
                .iter()
                .map(|name| {
                    self.nodes
                        .iter()
                        .position(|node| node.name == *name)
                        .ok_or_else(|| DecompositionCommandError::Selection(format!("node '{name}' was not found")))
                })
                .collect()
        }

        fn node_kind(&self, node: &Self::NodeId) -> Result<NodeKind, DecompositionCommandError>
        {
            self.nodes
                .get(*node)
                .map(|node| node.kind)
                .ok_or_else(|| DecompositionCommandError::Backend("node id is outside the network".to_owned()))
        }

        fn decompose_node(
            &mut self,
            node: &Self::NodeId,
            method: DecompositionMethod,
        ) -> Result<(), DecompositionCommandError>
        {
            self.decomposed.push((*node, method));
            Ok(())
        }
    }

    impl TechDecompCommandNetwork for TestNetwork
    {
        fn tech_decompose(
            &mut self,
            and_limit: i32,
            or_limit: i32,
        ) -> Result<(), DecompositionCommandError>
        {
            self.tech_calls.push((and_limit, or_limit));
            Ok(())
        }
    }

    #[test]
    fn registrations_match_decomposition_commands()
    {
        assert_eq!(
            decomposition_command_registrations(),
            &[
                CommandRegistration {
                    name: "decomp",
                    kind: DecompositionCommandKind::Decomp,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "tech_decomp",
                    kind: DecompositionCommandKind::TechDecomp,
                    changes_network: true,
                },
            ]
        );
    }

    #[test]
    fn decomp_defaults_to_quick_and_keeps_node_operands()
    {
        let plan = parse_decomp_args(["f", "g"]).unwrap();

        assert_eq!(plan.method, DecompositionMethod::Quick);
        assert_eq!(plan.node_names, vec!["f", "g"]);
    }

    #[test]
    fn decomp_uses_last_method_option_like_getopt_loop()
    {
        let plan = parse_decomp_args(["-qgd", "f"]).unwrap();

        assert_eq!(plan.method, DecompositionMethod::Disjoint);
        assert_eq!(plan.node_names, vec!["f"]);
    }

    #[test]
    fn tech_decomp_accepts_and_or_limits_and_rejects_extra_operands()
    {
        let plan = parse_tech_decomp_args(["-a", "4", "-o3"]).unwrap();

        assert_eq!(plan.and_limit, 4);
        assert_eq!(plan.or_limit, 3);
        assert_eq!(
            parse_tech_decomp_args(["-a", "4", "n"]).unwrap_err(),
            CommandParseError::UnexpectedOperand("n".to_owned())
        );
    }

    #[test]
    fn tech_decomp_rejects_limits_below_two_or_missing_limits()
    {
        assert_eq!(
            parse_tech_decomp_args(["-a", "1"]).unwrap_err(),
            CommandParseError::InvalidLimit {
                option: 'a',
                value: "1".to_owned(),
            }
        );
        assert_eq!(
            parse_tech_decomp_args(std::iter::empty::<&str>()).unwrap_err(),
            CommandParseError::MissingTechLimit
        );
    }

    #[test]
    fn decomp_execution_only_invokes_internal_nodes()
    {
        let mut network = TestNetwork::sample();
        let plan = DecompCommandPlan {
            method: DecompositionMethod::Good,
            node_names: Vec::new(),
        };

        let summary = execute_decomp_command(&mut network, &plan).unwrap();

        assert_eq!(
            summary,
            DecompExecutionSummary {
                selected_nodes: 4,
                decomposed_internal_nodes: 2,
                skipped_non_internal_nodes: 2,
            }
        );
        assert_eq!(
            network.decomposed,
            vec![
                (1, DecompositionMethod::Good),
                (2, DecompositionMethod::Good),
            ]
        );
    }

    #[test]
    fn tech_decomp_execution_delegates_to_native_backend()
    {
        let mut network = TestNetwork::sample();
        let plan = TechDecompCommandPlan {
            and_limit: 2,
            or_limit: 5,
        };

        execute_tech_decomp_command(&mut network, &plan).unwrap();

        assert_eq!(network.tech_calls, vec![(2, 5)]);
    }

    #[test]
    fn command_dispatch_parses_both_command_names()
    {
        assert_eq!(
            parse_decomposition_command("decomp", ["-g"]).unwrap(),
            DecompositionCommand::Decomp(DecompCommandPlan {
                method: DecompositionMethod::Good,
                node_names: Vec::new(),
            })
        );
        assert_eq!(
            parse_decomposition_command("tech_decomp", ["-o", "2"]).unwrap(),
            DecompositionCommand::TechDecomp(TechDecompCommandPlan {
                and_limit: 0,
                or_limit: 2,
            })
        );
    }
}
