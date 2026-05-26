//! Native Rust support for SIS genlib series/parallel trees.
//!
//! The original `sis/genlib/sptree.c` owns tree allocation, copying,
//! normalization, shape canonicalization, level metric computation, and
//! expression rendering for the genlib NAND/NOR form generator. This port keeps
//! those operations as safe owned-data APIs and intentionally exposes no legacy
//! C ABI entry points.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreeNodeType {
    Or,
    And,
    Nor,
    Nand,
    Zero,
    One,
    Leaf,
}

impl TreeNodeType {
    pub fn reversed(self) -> Self {
        match self {
            Self::Or => Self::And,
            Self::And => Self::Or,
            other => other,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeMetrics {
    pub series: usize,
    pub parallel: usize,
    pub level: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeNode {
    pub node_type: TreeNodeType,
    pub phase: bool,
    pub children: Vec<TreeNode>,
    pub name: Option<String>,
    pub series: usize,
    pub parallel: usize,
    pub level: usize,
}

impl TreeNode {
    pub fn new(children: Vec<TreeNode>) -> Self {
        Self {
            node_type: TreeNodeType::Or,
            phase: true,
            children,
            name: None,
            series: 0,
            parallel: 0,
            level: 0,
        }
    }

    pub fn with_type(node_type: TreeNodeType, children: Vec<TreeNode>) -> Self {
        Self {
            node_type,
            ..Self::new(children)
        }
    }

    pub fn leaf(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::new(Vec::new())
        }
    }

    pub fn unnamed_leaf() -> Self {
        Self::new(Vec::new())
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn dualize_node(&mut self) {
        self.node_type = self.node_type.reversed();
    }

    pub fn invert_tree(&mut self) {
        if self.phase {
            if self.is_leaf() {
                self.phase = false;
            } else {
                for child in &mut self.children {
                    child.invert_tree();
                }
                self.dualize_node();
            }
        } else {
            self.phase = true;
        }
    }

    pub fn dualize_tree(&mut self) {
        for child in &mut self.children {
            child.dualize_tree();
        }
        self.dualize_node();
    }

    pub fn make_well_formed(&mut self) {
        if !self.phase && !self.is_leaf() {
            self.phase = true;
            self.invert_tree();
        }

        let mut index = 0;
        while index < self.children.len() {
            if !self.children[index].phase && !self.children[index].is_leaf() {
                self.children[index].phase = true;
                self.children[index].invert_tree();
            }

            if self.children[index].is_leaf() {
                self.children[index].node_type = self.node_type.reversed();
                index += 1;
                continue;
            }

            if self.children[index].node_type == self.node_type {
                let mut child = self.children.remove(index);
                self.children.splice(index..index, child.children.drain(..));
                continue;
            }

            index += 1;
        }

        for child in &mut self.children {
            child.make_well_formed();
        }
    }

    pub fn compute_level(&mut self) -> TreeMetrics {
        if self.is_leaf() {
            self.series = 1;
            self.parallel = 1;
            self.level = 0;
        } else {
            let mut series = 0;
            let mut parallel = 0;
            let mut level = 0;

            for child in &mut self.children {
                let child_metrics = child.compute_level();
                match self.node_type {
                    TreeNodeType::And => {
                        series += child_metrics.series;
                        parallel = parallel.max(child_metrics.parallel);
                    }
                    _ => {
                        parallel += child_metrics.parallel;
                        series = series.max(child_metrics.series);
                    }
                }
                level = level.max(child_metrics.level);
            }

            self.series = series;
            self.parallel = parallel;
            self.level = level + 1;
        }

        TreeMetrics {
            series: self.series,
            parallel: self.parallel,
            level: self.level,
        }
    }

    pub fn canonicalize_shape(&mut self) {
        for child in &mut self.children {
            child.canonicalize_shape();
        }
        self.children.sort_by(compare_tree_shape);
    }

    pub fn compare_shape(&self, other: &Self) -> Ordering {
        compare_tree_shape(self, other)
    }

    pub fn render(&mut self) -> String {
        self.assign_leaf_names();
        if self.phase || self.is_leaf() {
            self.render_recur(0, false)
        } else {
            format!("({})'", self.render_recur(0, false))
        }
    }

    pub fn render_algebraic(&mut self) -> String {
        self.assign_leaf_names();
        if self.phase || self.is_leaf() {
            self.render_recur(0, true)
        } else {
            format!("!({})", self.render_recur(0, true))
        }
    }

    pub fn assign_node_names(&mut self) {
        let mut count = 0;
        self.assign_node_names_recur(&mut count);
    }

    pub fn assign_leaf_names(&mut self) {
        let mut count = 0;
        self.assign_leaf_names_recur(&mut count);
    }

    pub fn unique_leaf_names(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        self.collect_leaf_names(&mut names);
        names.into_iter().collect()
    }

    pub fn unique_leaf_representatives(&self) -> Vec<&TreeNode> {
        let mut leaves = BTreeMap::new();
        self.collect_leaf_representatives(&mut leaves);
        leaves.into_values().collect()
    }

    pub fn dump(&self) -> String {
        let mut lines = Vec::new();
        self.dump_recur(&mut lines);
        lines.join("\n")
    }

    fn render_recur(&self, level: usize, algebraic: bool) -> String {
        if self.is_leaf() {
            let mut text = String::new();
            if !self.phase {
                text.push('!');
            }
            text.push_str(self.name.as_deref().unwrap_or(""));
            return text;
        }

        let separator = match (self.node_type, algebraic) {
            (TreeNodeType::Or, _) => "+",
            (TreeNodeType::And, true) => "*",
            _ => "",
        };
        let mut text = self
            .children
            .iter()
            .map(|child| child.render_recur(level + 1, algebraic))
            .collect::<Vec<_>>()
            .join(separator);

        if self.node_type == TreeNodeType::Or && level > 0 {
            text = format!("({text})");
        }

        text
    }

    fn assign_node_names_recur(&mut self, count: &mut usize) {
        for child in &mut self.children {
            child.assign_node_names_recur(count);
        }

        if self.name.is_none() {
            self.name = Some(format!("_{count}"));
            *count += 1;
        }
    }

    fn assign_leaf_names_recur(&mut self, count: &mut usize) {
        if self.is_leaf() {
            if self.name.is_none() {
                self.name = Some(leaf_name(*count));
                *count += 1;
            }
            return;
        }

        for child in &mut self.children {
            child.assign_leaf_names_recur(count);
        }
    }

    fn collect_leaf_names(&self, names: &mut BTreeSet<String>) {
        if self.is_leaf() {
            if let Some(name) = &self.name {
                names.insert(name.clone());
            }
            return;
        }

        for child in &self.children {
            child.collect_leaf_names(names);
        }
    }

    fn collect_leaf_representatives<'a>(&'a self, leaves: &mut BTreeMap<String, &'a TreeNode>) {
        if self.is_leaf() {
            if let Some(name) = &self.name {
                leaves.insert(name.clone(), self);
            }
            return;
        }

        for child in &self.children {
            child.collect_leaf_representatives(leaves);
        }
    }

    fn dump_recur(&self, lines: &mut Vec<String>) {
        if self.is_leaf() {
            lines.push(format!(
                "LEAF : {} {}-NODE",
                self.name.as_deref().unwrap_or(""),
                type_name(self.node_type)
            ));
            return;
        }

        lines.push(format!(
            "nsons={} type={}",
            self.children.len(),
            type_name(self.node_type)
        ));
        for child in &self.children {
            child.dump_recur(lines);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TreeFormSet {
    entries: BTreeMap<ShapeKey, TreeNode>,
}

impl TreeFormSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn find_or_add(&mut self, mut tree: TreeNode) -> bool {
        tree.canonicalize_shape();
        tree.compute_level();
        let key = ShapeKey::from_tree(&tree);
        match self.entries.entry(key) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(tree);
                true
            }
            std::collections::btree_map::Entry::Occupied(_) => false,
        }
    }

    pub fn forms(&self) -> Vec<&TreeNode> {
        self.entries.values().collect()
    }

    pub fn into_forms(self) -> Vec<TreeNode> {
        self.entries.into_values().collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ShapeKey(Vec<usize>);

impl ShapeKey {
    fn from_tree(tree: &TreeNode) -> Self {
        let mut key = Vec::new();
        push_shape_key(tree, &mut key);
        Self(key)
    }
}

pub fn reverse_type(node_type: TreeNodeType) -> TreeNodeType {
    node_type.reversed()
}

pub fn compare_tree_shape(left: &TreeNode, right: &TreeNode) -> Ordering {
    match left.children.len().cmp(&right.children.len()) {
        Ordering::Equal => {
            for (left_child, right_child) in left.children.iter().zip(&right.children) {
                let result = compare_tree_shape(left_child, right_child);
                if result != Ordering::Equal {
                    return result;
                }
            }
            Ordering::Equal
        }
        other => other,
    }
}

fn push_shape_key(tree: &TreeNode, key: &mut Vec<usize>) {
    key.push(tree.children.len());
    for child in &tree.children {
        push_shape_key(child, key);
    }
}

fn leaf_name(index: usize) -> String {
    if index < 26 {
        ((b'a' + index as u8) as char).to_string()
    } else {
        format!("x{index}")
    }
}

fn type_name(node_type: TreeNodeType) -> &'static str {
    match node_type {
        TreeNodeType::Or | TreeNodeType::Nor => "OR",
        TreeNodeType::And | TreeNodeType::Nand => "AND",
        TreeNodeType::Zero => "ZERO",
        TreeNodeType::One => "ONE",
        TreeNodeType::Leaf => "LEAF",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(name: &str) -> TreeNode {
        TreeNode::leaf(name)
    }

    fn or(children: Vec<TreeNode>) -> TreeNode {
        TreeNode::with_type(TreeNodeType::Or, children)
    }

    fn and(children: Vec<TreeNode>) -> TreeNode {
        TreeNode::with_type(TreeNodeType::And, children)
    }

    #[test]
    fn invert_tree_pushes_negation_through_internal_nodes() {
        let mut tree = or(vec![leaf("a"), leaf("b")]);

        tree.invert_tree();

        assert_eq!(tree.node_type, TreeNodeType::And);
        assert_eq!(tree.children[0].phase, false);
        assert_eq!(tree.children[1].phase, false);
    }

    #[test]
    fn invert_tree_toggles_leaf_phase() {
        let mut tree = leaf("a");

        tree.invert_tree();
        assert!(!tree.phase);

        tree.invert_tree();
        assert!(tree.phase);
    }

    #[test]
    fn make_well_formed_flattens_equal_type_children() {
        let mut tree = or(vec![leaf("a"), or(vec![leaf("b"), leaf("c")])]);

        tree.make_well_formed();

        assert_eq!(tree.children.len(), 3);
        assert_eq!(tree.render(), "a+b+c");
    }

    #[test]
    fn make_well_formed_marks_leaf_type_as_opposite_parent_type() {
        let mut tree = and(vec![leaf("a"), leaf("b")]);

        tree.make_well_formed();

        assert_eq!(tree.children[0].node_type, TreeNodeType::Or);
        assert_eq!(tree.children[1].node_type, TreeNodeType::Or);
    }

    #[test]
    fn compute_level_matches_series_parallel_rules() {
        let mut tree = or(vec![and(vec![leaf("a"), leaf("b")]), leaf("c")]);

        let metrics = tree.compute_level();

        assert_eq!(
            metrics,
            TreeMetrics {
                series: 2,
                parallel: 2,
                level: 2
            }
        );
    }

    #[test]
    fn render_matches_legacy_print_tree_precedence() {
        let mut tree = and(vec![or(vec![leaf("a"), leaf("b")]), leaf("c")]);

        assert_eq!(tree.render(), "(a+b)c");
    }

    #[test]
    fn algebraic_render_prints_explicit_and_operator() {
        let mut tree = and(vec![or(vec![leaf("a"), leaf("b")]), leaf("c")]);

        assert_eq!(tree.render_algebraic(), "(a+b)*c");
    }

    #[test]
    fn negative_root_rendering_matches_legacy_modes() {
        let mut tree = or(vec![leaf("a"), leaf("b")]);
        tree.phase = false;
        let mut algebraic = tree.clone();

        assert_eq!(tree.render(), "(a+b)'");
        assert_eq!(algebraic.render_algebraic(), "!(a+b)");
    }

    #[test]
    fn assigns_missing_leaf_names_in_depth_first_order() {
        let mut tree = or(vec![
            TreeNode::unnamed_leaf(),
            and(vec![TreeNode::unnamed_leaf()]),
        ]);

        assert_eq!(tree.render(), "a+b");
        assert_eq!(
            tree.unique_leaf_names(),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn assigns_node_names_post_order() {
        let mut tree = or(vec![and(vec![leaf("a"), leaf("b")]), leaf("c")]);

        tree.assign_node_names();

        assert_eq!(tree.children[0].name.as_deref(), Some("_0"));
        assert_eq!(tree.children[1].name.as_deref(), Some("c"));
        assert_eq!(tree.name.as_deref(), Some("_1"));
    }

    #[test]
    fn canonicalize_and_compare_use_shape_only() {
        let mut left = or(vec![and(vec![leaf("a"), leaf("b")]), leaf("c")]);
        let mut right = and(vec![leaf("x"), or(vec![leaf("y"), leaf("z")])]);

        left.canonicalize_shape();
        right.canonicalize_shape();

        assert_eq!(left.compare_shape(&right), Ordering::Equal);
    }

    #[test]
    fn form_set_rejects_duplicate_shapes_and_updates_metrics() {
        let mut forms = TreeFormSet::new();
        let tree = or(vec![leaf("a"), and(vec![leaf("b"), leaf("c")])]);

        assert!(forms.find_or_add(tree.clone()));
        assert!(!forms.find_or_add(tree));
        assert_eq!(forms.len(), 1);

        let form = forms.forms()[0];
        assert_eq!(form.series, 2);
        assert_eq!(form.parallel, 2);
        assert_eq!(form.level, 2);
    }

    #[test]
    fn unique_leaf_representatives_keep_last_leaf_for_each_name() {
        let tree = or(vec![leaf("a"), and(vec![leaf("b"), leaf("a")])]);

        let names = tree
            .unique_leaf_representatives()
            .into_iter()
            .map(|node| node.name.clone().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn dump_matches_legacy_shape_text() {
        let tree = or(vec![leaf("a")]);

        assert_eq!(tree.dump(), "nsons=1 type=OR\nLEAF : a OR-NODE");
    }
}
