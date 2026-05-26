//! Native Rust printing helpers for SIS factor trees.
//!
//! The original SIS code prints factored forms by resolving leaf indices through
//! the owning node's fanin list. The native factor tree model stores only the
//! fanin index, so this module accepts the fanin display names explicitly.

use std::error::Error;
use std::fmt;

use super::ft_util::{FactorKind, FactorNetwork, FactorTree, NodeId, factor_nt_to_ft};

pub const DEFAULT_LINE_WIDTH: usize = 60;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FactorPrintOptions {
    pub line_width: usize,
}

impl Default for FactorPrintOptions {
    fn default() -> Self {
        Self {
            line_width: DEFAULT_LINE_WIDTH,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorPrintError {
    MissingChild(FactorKind),
    UnknownFactorKind,
    InvalidLeafIndex(isize),
    UnknownFaninIndex(usize),
    FactorBuild(String),
}

impl fmt::Display for FactorPrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingChild(kind) => write!(f, "factor {:?} is missing its child", kind),
            Self::UnknownFactorKind => write!(f, "unknown factor kind"),
            Self::InvalidLeafIndex(index) => write!(f, "invalid factor leaf index {}", index),
            Self::UnknownFaninIndex(index) => write!(f, "unknown factor fanin index {}", index),
            Self::FactorBuild(message) => write!(f, "{message}"),
        }
    }
}

impl Error for FactorPrintError {}

impl From<super::ft_util::FactorError> for FactorPrintError {
    fn from(value: super::ft_util::FactorError) -> Self {
        Self::FactorBuild(value.to_string())
    }
}

pub type FactorPrintResult<T> = Result<T, FactorPrintError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorTreeLine {
    pub level: usize,
    pub node: String,
    pub children: Vec<String>,
}

pub fn factor_print(
    network: &FactorNetwork,
    node: NodeId,
    fanin_names: &[impl AsRef<str>],
) -> FactorPrintResult<String> {
    factor_print_with_options(network, node, fanin_names, FactorPrintOptions::default())
}

pub fn factor_print_with_options(
    network: &FactorNetwork,
    node: NodeId,
    fanin_names: &[impl AsRef<str>],
    options: FactorPrintOptions,
) -> FactorPrintResult<String> {
    let tree = match network.node(node)?.factored() {
        Some(tree) => tree.clone(),
        None => factor_nt_to_ft(network, node, node)?,
    };

    format_factor_tree(&tree, fanin_names, options)
}

pub fn format_factor_tree(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
    options: FactorPrintOptions,
) -> FactorPrintResult<String> {
    let mut output = String::new();
    let mut line_len = 0_usize;
    print_tree(
        tree,
        fanin_names,
        options.line_width,
        &mut line_len,
        &mut output,
    )?;
    Ok(output)
}

pub fn debug_tree_lines(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
) -> FactorPrintResult<Vec<FactorTreeLine>> {
    let mut lines = Vec::new();
    collect_debug_lines(tree, fanin_names, 0, &mut lines)?;
    Ok(lines)
}

pub fn format_debug_tree(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
) -> FactorPrintResult<String> {
    let mut output = String::new();
    for line in debug_tree_lines(tree, fanin_names)? {
        output.push_str(&format!("{}: {}\t", line.level, line.node));
        for child in line.children {
            output.push('\t');
            output.push_str(&child);
        }
        output.push('\n');
    }

    Ok(output)
}

pub fn factor_lengths(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
) -> FactorPrintResult<Vec<usize>> {
    let mut lengths = Vec::new();
    collect_lengths(tree, fanin_names, &mut lengths)?;
    Ok(lengths)
}

pub fn factor_len_init(
    tree: &mut FactorTree,
    fanin_names: &[impl AsRef<str>],
) -> FactorPrintResult<()> {
    assign_lengths(tree, fanin_names)?;
    Ok(())
}

pub fn factor_len(tree: &FactorTree, fanin_names: &[impl AsRef<str>]) -> FactorPrintResult<usize> {
    tree_len(tree, fanin_names)
}

fn print_tree(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
    line_width: usize,
    line_len: &mut usize,
    output: &mut String,
) -> FactorPrintResult<()> {
    match tree.kind {
        FactorKind::Zero => {
            output.push_str("-0-");
            Ok(())
        }
        FactorKind::One => {
            output.push_str("-1-");
            Ok(())
        }
        FactorKind::And => print_and(tree, fanin_names, line_width, line_len, output),
        FactorKind::Or => print_or(tree, fanin_names, line_width, line_len, output),
        FactorKind::Inverter => {
            let child = tree
                .next_level
                .as_deref()
                .ok_or(FactorPrintError::MissingChild(FactorKind::Inverter))?;
            print_tree(child, fanin_names, line_width, line_len, output)?;
            output.push('\'');
            *line_len += 1;
            Ok(())
        }
        FactorKind::Leaf => {
            let name = leaf_name(tree, fanin_names)?;
            if line_width > 0 && name.len() + *line_len > line_width {
                output.push_str("\n\t");
                *line_len = 0;
            }

            output.push_str(name);
            *line_len += name.len();
            Ok(())
        }
        FactorKind::Unknown => Err(FactorPrintError::UnknownFactorKind),
    }
}

fn print_or(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
    line_width: usize,
    line_len: &mut usize,
    output: &mut String,
) -> FactorPrintResult<()> {
    let mut child = tree.next_level.as_deref();
    let mut first = true;
    while let Some(node) = child {
        if first {
            first = false;
        } else {
            output.push_str(" + ");
            *line_len += 3;
        }

        print_tree(node, fanin_names, line_width, line_len, output)?;
        child = node.same_level.as_deref();
    }

    Ok(())
}

fn print_and(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
    line_width: usize,
    line_len: &mut usize,
    output: &mut String,
) -> FactorPrintResult<()> {
    let mut child = tree.next_level.as_deref();
    let mut first = true;
    while let Some(node) = child {
        if first {
            first = false;
        } else {
            output.push(' ');
            *line_len += 1;
        }

        if node.kind == FactorKind::Or {
            output.push('(');
            print_tree(node, fanin_names, line_width, line_len, output)?;
            output.push(')');
        } else {
            print_tree(node, fanin_names, line_width, line_len, output)?;
        }

        child = node.same_level.as_deref();
    }

    Ok(())
}

fn collect_debug_lines(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
    level: usize,
    lines: &mut Vec<FactorTreeLine>,
) -> FactorPrintResult<()> {
    let mut children = Vec::new();
    let mut child = tree.next_level.as_deref();
    while let Some(node) = child {
        children.push(tree_name(node, fanin_names)?.to_string());
        child = node.same_level.as_deref();
    }

    lines.push(FactorTreeLine {
        level,
        node: tree_name(tree, fanin_names)?.to_string(),
        children,
    });

    let mut child = tree.next_level.as_deref();
    while let Some(node) = child {
        collect_debug_lines(node, fanin_names, level + 1, lines)?;
        child = node.same_level.as_deref();
    }

    Ok(())
}

fn collect_lengths(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
    lengths: &mut Vec<usize>,
) -> FactorPrintResult<usize> {
    let length = tree_len(tree, fanin_names)?;
    lengths.push(length);

    let mut child = tree.next_level.as_deref();
    while let Some(node) = child {
        collect_lengths(node, fanin_names, lengths)?;
        child = node.same_level.as_deref();
    }

    Ok(length)
}

fn assign_lengths(
    tree: &mut FactorTree,
    fanin_names: &[impl AsRef<str>],
) -> FactorPrintResult<usize> {
    if let Some(child) = tree.next_level.as_deref_mut() {
        assign_lengths(child, fanin_names)?;
    }

    tree.len = tree_len_from_child_lengths(tree, fanin_names)?;

    if let Some(sibling) = tree.same_level.as_deref_mut() {
        assign_lengths(sibling, fanin_names)?;
    }

    Ok(tree.len)
}

fn tree_len(tree: &FactorTree, fanin_names: &[impl AsRef<str>]) -> FactorPrintResult<usize> {
    match tree.kind {
        FactorKind::Zero | FactorKind::One => Ok(3),
        FactorKind::Leaf => Ok(leaf_name(tree, fanin_names)?.len()),
        FactorKind::Inverter => {
            let child = tree
                .next_level
                .as_deref()
                .ok_or(FactorPrintError::MissingChild(FactorKind::Inverter))?;
            Ok(tree_len(child, fanin_names)? + 1)
        }
        FactorKind::And => nary_len(tree, fanin_names, 1),
        FactorKind::Or => nary_len(tree, fanin_names, 3),
        FactorKind::Unknown => Err(FactorPrintError::UnknownFactorKind),
    }
}

fn nary_len(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
    child_extra: usize,
) -> FactorPrintResult<usize> {
    let mut total = None::<isize>;
    let mut child = tree.next_level.as_deref();
    while let Some(node) = child {
        let child_len = isize::try_from(tree_len(node, fanin_names)?).unwrap_or(isize::MAX);
        let extra = isize::try_from(child_extra).unwrap_or(isize::MAX);
        total = Some(
            total
                .unwrap_or(-1)
                .saturating_add(child_len)
                .saturating_add(extra),
        );
        child = node.same_level.as_deref();
    }

    Ok(total.unwrap_or(0).max(0) as usize)
}

fn tree_len_from_child_lengths(
    tree: &FactorTree,
    fanin_names: &[impl AsRef<str>],
) -> FactorPrintResult<usize> {
    match tree.kind {
        FactorKind::Zero | FactorKind::One => Ok(3),
        FactorKind::Leaf => Ok(leaf_name(tree, fanin_names)?.len()),
        FactorKind::Inverter => {
            let child = tree
                .next_level
                .as_deref()
                .ok_or(FactorPrintError::MissingChild(FactorKind::Inverter))?;
            Ok(child.len + 1)
        }
        FactorKind::And => nary_len_from_child_lengths(tree, 1),
        FactorKind::Or => nary_len_from_child_lengths(tree, 3),
        FactorKind::Unknown => Err(FactorPrintError::UnknownFactorKind),
    }
}

fn nary_len_from_child_lengths(tree: &FactorTree, child_extra: usize) -> FactorPrintResult<usize> {
    let mut total = None::<isize>;
    let mut child = tree.next_level.as_deref();
    while let Some(node) = child {
        let child_len = isize::try_from(node.len).unwrap_or(isize::MAX);
        let extra = isize::try_from(child_extra).unwrap_or(isize::MAX);
        total = Some(
            total
                .unwrap_or(-1)
                .saturating_add(child_len)
                .saturating_add(extra),
        );
        child = node.same_level.as_deref();
    }

    Ok(total.unwrap_or(0).max(0) as usize)
}

fn tree_name<'a>(
    tree: &'a FactorTree,
    fanin_names: &'a [impl AsRef<str>],
) -> FactorPrintResult<&'a str> {
    match tree.kind {
        FactorKind::Zero => Ok("-0-"),
        FactorKind::One => Ok("-1-"),
        FactorKind::And => Ok("AND"),
        FactorKind::Or => Ok("OR"),
        FactorKind::Inverter => Ok("INV"),
        FactorKind::Leaf => leaf_name(tree, fanin_names),
        FactorKind::Unknown => Ok("???"),
    }
}

fn leaf_name<'a>(
    tree: &FactorTree,
    fanin_names: &'a [impl AsRef<str>],
) -> FactorPrintResult<&'a str> {
    let index =
        usize::try_from(tree.index).map_err(|_| FactorPrintError::InvalidLeafIndex(tree.index))?;
    fanin_names
        .get(index)
        .map(AsRef::as_ref)
        .ok_or(FactorPrintError::UnknownFaninIndex(index))
}

#[cfg(test)]
mod tests {
    use super::super::ft_util::{CubeLiteral, FactorNode, factor_set};
    use super::*;

    fn leaf(index: usize) -> FactorTree {
        FactorTree::leaf(index)
    }

    fn inv(child: FactorTree) -> FactorTree {
        FactorTree::new(FactorKind::Inverter, -1, 0).with_next_level(child)
    }

    fn nary(kind: FactorKind, mut children: Vec<FactorTree>) -> FactorTree {
        let mut chain = None;
        while let Some(mut child) = children.pop() {
            child.same_level = chain;
            chain = Some(Box::new(child));
        }

        FactorTree::new(kind, -1, 0).with_next_level(*chain.unwrap())
    }

    #[test]
    fn prints_constants_and_leaf_names() {
        assert_eq!(
            format_factor_tree(
                &FactorTree::constant(false),
                &["a", "b"],
                FactorPrintOptions::default()
            )
            .unwrap(),
            "-0-"
        );
        assert_eq!(
            format_factor_tree(&leaf(1), &["a", "b"], FactorPrintOptions::default()).unwrap(),
            "b"
        );
    }

    #[test]
    fn prints_and_or_and_inverter_precedence_like_sis() {
        let tree = nary(
            FactorKind::And,
            vec![nary(FactorKind::Or, vec![leaf(0), inv(leaf(1))]), leaf(2)],
        );

        let text =
            format_factor_tree(&tree, &["a", "b", "c"], FactorPrintOptions::default()).unwrap();

        assert_eq!(text, "(a + b') c");
    }

    #[test]
    fn wraps_before_leaf_names_when_line_width_is_exceeded() {
        let tree = nary(FactorKind::And, vec![leaf(0), leaf(1), leaf(2)]);

        let text = format_factor_tree(
            &tree,
            &["alpha", "beta", "gamma"],
            FactorPrintOptions { line_width: 9 },
        )
        .unwrap();

        assert_eq!(text, "alpha \n\tbeta \n\tgamma");
    }

    #[test]
    fn debug_tree_matches_legacy_preorder_listing() {
        let tree = nary(FactorKind::Or, vec![leaf(0), inv(leaf(1))]);

        let text = format_debug_tree(&tree, &["a", "b"]).unwrap();

        assert_eq!(text, "0: OR\t\ta\tINV\n1: a\t\n1: INV\t\tb\n2: b\t\n");
    }

    #[test]
    fn computes_lengths_with_legacy_separator_widths() {
        let tree = nary(
            FactorKind::Or,
            vec![nary(FactorKind::And, vec![leaf(0), leaf(1)]), inv(leaf(2))],
        );

        assert_eq!(factor_len(&tree, &["aa", "bbb", "c"]).unwrap(), 13);
        assert_eq!(factor_lengths(&tree, &["aa", "bbb", "c"]).unwrap()[0], 13);
    }

    #[test]
    fn initializes_cached_lengths_in_postorder_like_legacy_ft_len_init() {
        let mut tree = nary(
            FactorKind::Or,
            vec![nary(FactorKind::And, vec![leaf(0), leaf(1)]), inv(leaf(2))],
        );

        factor_len_init(&mut tree, &["aa", "bbb", "c"]).unwrap();

        assert_eq!(tree.len, 13);
        let and = tree.next_level.as_deref().unwrap();
        assert_eq!(and.len, 6);
        let inv = and.same_level.as_deref().unwrap();
        assert_eq!(inv.len, 2);
    }

    #[test]
    fn formats_existing_or_computed_factor_for_network_node() {
        let mut network = FactorNetwork::new();
        let a = network.add_node(FactorNode::input());
        let b = network.add_node(FactorNode::input());
        let f = network.add_node(FactorNode::sum_of_products(
            vec![a, b],
            vec![vec![CubeLiteral::One, CubeLiteral::Zero]],
        ));

        let text = factor_print(&network, f, &["a", "b"]).unwrap();

        assert_eq!(text, "a b'");
    }

    #[test]
    fn prefers_cached_factored_tree_when_present() {
        let mut network = FactorNetwork::new();
        let a = network.add_node(FactorNode::input());
        let b = network.add_node(FactorNode::input());
        let f = network.add_node(FactorNode::sum_of_products(
            vec![a, b],
            vec![vec![CubeLiteral::One, CubeLiteral::One]],
        ));
        factor_set(network.node_mut(f).unwrap(), inv(leaf(1)));

        let text = factor_print(&network, f, &["a", "b"]).unwrap();

        assert_eq!(text, "b'");
    }

    #[test]
    fn reports_invalid_leaf_indices_and_unknown_types() {
        let invalid_leaf = FactorTree::new(FactorKind::Leaf, -1, 0);
        let unknown = FactorTree::new(FactorKind::Unknown, -1, 0);

        assert_eq!(
            format_factor_tree(&invalid_leaf, &["a"], FactorPrintOptions::default()).unwrap_err(),
            FactorPrintError::InvalidLeafIndex(-1)
        );
        assert_eq!(
            format_factor_tree(&unknown, &["a"], FactorPrintOptions::default()).unwrap_err(),
            FactorPrintError::UnknownFactorKind
        );
    }

    #[test]
    fn no_c_abi_exports_are_present_in_this_port() {
        let text = include_str!("ft_print.rs");

        assert!(!text.contains(concat!("no", "_", "mangle")));
        assert!(!text.contains(concat!("pub ", "extern")));
        assert!(!text.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
