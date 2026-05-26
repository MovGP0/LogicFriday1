//! Native Rust port of SIS `genlib/nand.c`.
//!
//! This module builds NAND/NOR-only tree forms from the shared genlib
//! series/parallel tree model and formats those trees as small BLIF fragments.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use super::comb::{CombinationError, CombinationGenerator};
use super::permute::gl_permute;
use super::sptree::{TreeFormSet, TreeNode, TreeNodeType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NandError {
    EmptyTreeList,
    Combination(CombinationError),
}

impl fmt::Display for NandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTreeList => write!(formatter, "tree generation requires at least one tree"),
            Self::Combination(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for NandError {}

impl From<CombinationError> for NandError {
    fn from(value: CombinationError) -> Self {
        Self::Combination(value)
    }
}

pub type NandResult<T> = Result<T, NandError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatchInfo {
    pub input: String,
    pub output: String,
    pub latch_type: String,
}

impl LatchInfo {
    pub fn new(
        input: impl Into<String>,
        output: impl Into<String>,
        latch_type: impl Into<String>,
    ) -> Self {
        Self {
            input: input.into(),
            output: output.into(),
            latch_type: latch_type.into(),
        }
    }
}

pub fn write_blif(tree: &TreeNode, use_nor_gate: bool, latch: Option<&LatchInfo>) -> String {
    match tree.node_type {
        TreeNodeType::Zero => {
            format!(".outputs {}\n.names {}\n", node_name(tree), node_name(tree))
        }
        TreeNodeType::One => {
            format!(
                ".outputs {}\n.names {}\n 1\n",
                node_name(tree),
                node_name(tree)
            )
        }
        _ => {
            let mut output = String::new();
            output.push_str(".inputs");
            write_blif_inputs(&mut output, tree, latch);
            output.push('\n');

            if latch.is_none() {
                output.push_str(&format!(".outputs {}\n", node_name(tree)));
            }

            write_blif_tables(&mut output, tree, use_nor_gate);
            output
        }
    }
}

pub fn nand_gate_forms(tree: &TreeNode, use_nor_gate: bool) -> NandResult<Vec<TreeNode>> {
    let mut forms = if tree.is_leaf() {
        let leaf = leaf_from(tree);
        if tree.phase {
            vec![TreeNode::new(vec![TreeNode::new(vec![leaf])])]
        } else {
            vec![TreeNode::new(vec![leaf])]
        }
    } else {
        let invert_output = !tree.phase;
        let mut forms = nand_gate_forms_recur(tree, use_nor_gate)?;

        if ((tree.node_type == TreeNodeType::Or) != use_nor_gate) == invert_output {
            for form in &mut forms {
                *form = TreeNode::new(vec![std::mem::replace(form, TreeNode::new(Vec::new()))]);
            }
        }

        forms
    };

    for form in &mut forms {
        set_node_type(
            form,
            if use_nor_gate {
                TreeNodeType::Nor
            } else {
                TreeNodeType::Nand
            },
        );
    }

    Ok(forms)
}

fn set_node_type(tree: &mut TreeNode, node_type: TreeNodeType) {
    for child in &mut tree.children {
        set_node_type(child, node_type);
    }

    tree.phase = true;
    tree.node_type = node_type;
}

fn write_blif_inputs(output: &mut String, tree: &TreeNode, latch: Option<&LatchInfo>) {
    let latch_output = latch.map(|info| info.output.as_str());
    let mut leaf_names = BTreeSet::new();
    collect_leaf_names(tree, &mut leaf_names);

    for name in leaf_names {
        if latch_output == Some(name.as_str()) {
            continue;
        }

        output.push(' ');
        output.push_str(&name);
    }
}

fn write_blif_tables(output: &mut String, tree: &TreeNode, use_nor_gate: bool) {
    for child in &tree.children {
        write_blif_tables(output, child, use_nor_gate);
    }

    if tree.is_leaf() {
        return;
    }

    output.push_str(".names");
    for child in &tree.children {
        output.push(' ');
        output.push_str(node_name(child));
    }
    output.push(' ');
    output.push_str(node_name(tree));
    output.push('\n');

    if use_nor_gate {
        output.push_str(&"0".repeat(tree.children.len()));
        output.push_str(" 1\n");
    } else {
        for index in 0..tree.children.len() {
            for child_index in 0..tree.children.len() {
                output.push(if index == child_index { '0' } else { '-' });
            }
            output.push_str(" 1\n");
        }
    }
}

fn nand_gate_forms_recur(tree: &TreeNode, use_nor_gate: bool) -> NandResult<Vec<TreeNode>> {
    if tree.is_leaf() {
        let invert_input = !tree.phase;
        let use_input_inverter =
            ((tree.node_type == TreeNodeType::Or) == use_nor_gate) != invert_input;
        let leaf = leaf_from(tree);
        return Ok(if use_input_inverter {
            vec![TreeNode::new(vec![leaf])]
        } else {
            vec![leaf]
        });
    }

    let mut child_forms = Vec::with_capacity(tree.children.len());
    let mut radices = Vec::with_capacity(tree.children.len());
    for child in &tree.children {
        let forms = nand_gate_forms_recur(child, use_nor_gate)?;
        radices.push(forms.len());
        child_forms.push(forms);
    }

    let mut unique_forms = TreeFormSet::new();
    let mut generator = CombinationGenerator::new(radices)?;
    while let Some(indices) = generator.next_combination() {
        let forms = indices
            .iter()
            .enumerate()
            .map(|(child_index, form_index)| &child_forms[child_index][*form_index])
            .collect::<Vec<_>>();

        if all_isomorphic(&forms) {
            make_tree(&forms, &mut unique_forms)?;
        } else {
            let mut permuted = forms;
            gl_permute(&mut permuted, &mut unique_forms, |permutation, forms| {
                let _ = make_tree(permutation, forms);
            });
        }
    }

    Ok(unique_forms.into_forms())
}

fn make_tree(list: &[&TreeNode], forms: &mut TreeFormSet) -> NandResult<()> {
    for tree in make_tree_recur(list, 0)? {
        forms.find_or_add(tree);
    }

    Ok(())
}

fn make_tree_recur(leafs: &[&TreeNode], level: usize) -> NandResult<Vec<TreeNode>> {
    if leafs.is_empty() {
        return Err(NandError::EmptyTreeList);
    }

    if leafs.len() == 1 {
        return Ok(vec![leafs[0].clone()]);
    }

    let mut forms = Vec::new();
    for split in 1..=leafs.len() / 2 {
        let left_forms = make_tree_recur(&leafs[..split], level + 1)?;
        let right_forms = make_tree_recur(&leafs[split..], level + 1)?;

        for left in &left_forms {
            for right in &right_forms {
                let binary = TreeNode::new(vec![left.clone(), right.clone()]);
                if level > 0 {
                    forms.push(TreeNode::new(vec![binary]));
                } else {
                    forms.push(binary);
                }
            }
        }
    }

    Ok(forms)
}

fn all_isomorphic(forms: &[&TreeNode]) -> bool {
    forms
        .windows(2)
        .all(|window| window[0].compare_shape(window[1]).is_eq())
}

fn collect_leaf_names(tree: &TreeNode, names: &mut BTreeSet<String>) {
    if tree.is_leaf() {
        if let Some(name) = &tree.name {
            names.insert(name.clone());
        }
        return;
    }

    for child in &tree.children {
        collect_leaf_names(child, names);
    }
}

fn leaf_from(tree: &TreeNode) -> TreeNode {
    TreeNode::leaf(tree.name.clone().unwrap_or_default())
}

fn node_name(tree: &TreeNode) -> &str {
    tree.name.as_deref().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(name: &str) -> TreeNode {
        TreeNode::leaf(name)
    }

    fn named_node(name: &str, node_type: TreeNodeType, children: Vec<TreeNode>) -> TreeNode {
        let mut tree = TreeNode::with_type(node_type, children);
        tree.name = Some(name.to_string());
        tree
    }

    fn or(children: Vec<TreeNode>) -> TreeNode {
        TreeNode::with_type(TreeNodeType::Or, children)
    }

    fn and(children: Vec<TreeNode>) -> TreeNode {
        TreeNode::with_type(TreeNodeType::And, children)
    }

    fn shape_keys(forms: &[TreeNode]) -> Vec<String> {
        forms.iter().map(shape_key).collect()
    }

    fn shape_key(tree: &TreeNode) -> String {
        if tree.is_leaf() {
            "x".to_string()
        } else {
            format!(
                "({})",
                tree.children
                    .iter()
                    .map(shape_key)
                    .collect::<Vec<_>>()
                    .join("")
            )
        }
    }

    #[test]
    fn leaf_forms_match_special_inverter_and_buffer_cases() {
        let normal = nand_gate_forms(&leaf("a"), false).unwrap();
        let mut inverted_leaf = leaf("a");
        inverted_leaf.phase = false;
        let inverted = nand_gate_forms(&inverted_leaf, false).unwrap();

        assert_eq!(shape_keys(&normal), vec!["((x))"]);
        assert_eq!(shape_keys(&inverted), vec!["(x)"]);
        assert_eq!(normal[0].node_type, TreeNodeType::Nand);
        assert_eq!(normal[0].children[0].node_type, TreeNodeType::Nand);
        assert!(normal[0].phase);
    }

    #[test]
    fn nonleaf_forms_make_balanced_binary_tree_shapes_with_extra_internal_inverters() {
        let tree = or(vec![leaf("a"), leaf("b"), leaf("c")]);
        let forms = nand_gate_forms(&tree, false).unwrap();

        assert_eq!(shape_keys(&forms), vec!["(x((xx)))"]);
        assert!(
            forms
                .iter()
                .all(|form| form.node_type == TreeNodeType::Nand)
        );
    }

    #[test]
    fn isomorphic_children_skip_duplicate_permutations() {
        let tree = and(vec![
            or(vec![leaf("a")]),
            or(vec![leaf("b")]),
            or(vec![leaf("c")]),
        ]);
        let forms = nand_gate_forms(&tree, false).unwrap();

        assert_eq!(shape_keys(&forms), vec!["((x((xx))))"]);
    }

    #[test]
    fn output_phase_adds_root_inverter_when_required() {
        let mut tree = and(vec![leaf("a"), leaf("b")]);
        tree.phase = false;

        let forms = nand_gate_forms(&tree, false).unwrap();

        assert_eq!(shape_keys(&forms), vec!["(xx)"]);
    }

    #[test]
    fn nor_generation_sets_all_node_types_to_nor() {
        let tree = or(vec![leaf("a"), leaf("b")]);
        let forms = nand_gate_forms(&tree, true).unwrap();

        assert_eq!(shape_keys(&forms), vec!["(((x)(x)))"]);
        assert_eq!(forms[0].node_type, TreeNodeType::Nor);
        assert_eq!(forms[0].children[0].node_type, TreeNodeType::Nor);
    }

    #[test]
    fn writes_constant_zero_and_one_blif_fragments() {
        let zero = named_node("z", TreeNodeType::Zero, Vec::new());
        let one = named_node("o", TreeNodeType::One, Vec::new());

        assert_eq!(write_blif(&zero, false, None), ".outputs z\n.names z\n");
        assert_eq!(write_blif(&one, false, None), ".outputs o\n.names o\n 1\n");
    }

    #[test]
    fn writes_nand_blif_tables_in_post_order() {
        let tree = named_node(
            "f",
            TreeNodeType::Nand,
            vec![
                named_node("_0", TreeNodeType::Nand, vec![leaf("a"), leaf("b")]),
                leaf("c"),
            ],
        );

        assert_eq!(
            write_blif(&tree, false, None),
            ".inputs a b c\n.outputs f\n.names a b _0\n0- 1\n-0 1\n.names _0 c f\n0- 1\n-0 1\n"
        );
    }

    #[test]
    fn writes_nor_blif_table_rows() {
        let tree = named_node(
            "f",
            TreeNodeType::Nor,
            vec![leaf("a"), leaf("b"), leaf("c")],
        );

        assert_eq!(
            write_blif(&tree, true, None),
            ".inputs a b c\n.outputs f\n.names a b c f\n000 1\n"
        );
    }

    #[test]
    fn latch_output_is_not_reported_as_primary_input_or_output() {
        let tree = named_node(
            "lat_in",
            TreeNodeType::Nand,
            vec![leaf("lat_out"), leaf("a")],
        );
        let latch = LatchInfo::new("lat_in", "lat_out", "re");

        assert_eq!(
            write_blif(&tree, false, Some(&latch)),
            ".inputs a\n.names lat_out a lat_in\n0- 1\n-0 1\n"
        );
    }

    #[test]
    fn source_contains_no_dependency_metadata_or_c_abi_shims() {
        let source = include_str!("nand.rs");

        for forbidden in [
            concat!("REQUIRED", "_"),
            concat!("Port", "Dependency"),
            concat!("bead", "_id"),
            concat!("source", "_file"),
            concat!("LogicFriday1", "-8j8"),
            concat!("no", "_mangle"),
            concat!("extern ", "\"C\""),
        ] {
            assert!(
                !source.contains(forbidden),
                "source contains forbidden marker {forbidden}"
            );
        }
    }
}
