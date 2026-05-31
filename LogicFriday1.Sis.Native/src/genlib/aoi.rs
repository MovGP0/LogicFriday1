//! Native Rust generator for alternating AND/OR genlib tree forms.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNodeKind {
    Or,
    And,
}

impl TreeNodeKind {
    pub fn reverse(self) -> Self {
        match self {
            Self::Or => Self::And,
            Self::And => Self::Or,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AoiTree {
    pub kind: TreeNodeKind,
    pub phase: bool,
    pub name: Option<String>,
    pub sons: Vec<AoiTree>,
    pub series: usize,
    pub parallel: usize,
    pub level: usize,
}

impl AoiTree {
    pub fn leaf(kind: TreeNodeKind) -> Self {
        let mut tree = Self {
            kind,
            phase: true,
            name: None,
            sons: Vec::new(),
            series: 1,
            parallel: 1,
            level: 0,
        };
        tree.compute_metrics();
        tree
    }

    pub fn node(kind: TreeNodeKind, sons: Vec<AoiTree>) -> Self {
        let mut tree = Self {
            kind,
            phase: true,
            name: None,
            sons,
            series: 0,
            parallel: 0,
            level: 0,
        };
        tree.canonicalize();
        tree
    }

    pub fn is_leaf(&self) -> bool {
        self.sons.is_empty()
    }

    pub fn literal_count(&self) -> usize {
        if self.is_leaf() {
            1
        } else {
            self.sons.iter().map(Self::literal_count).sum()
        }
    }

    pub fn assign_leaf_names(&mut self) {
        let mut next = 0usize;
        self.assign_leaf_names_recur(&mut next);
    }

    pub fn to_algebraic_string(&self) -> String {
        let mut tree = self.clone();
        tree.assign_leaf_names();
        tree.to_algebraic_string_recur(0)
    }

    fn assign_leaf_names_recur(&mut self, next: &mut usize) {
        if self.is_leaf() {
            if self.name.is_none() {
                let name = if *next < 26 {
                    ((*next as u8) + b'a') as char
                } else {
                    '_'
                };
                self.name = Some(name.to_string());
            }
            *next += 1;
            return;
        }

        for son in &mut self.sons {
            son.assign_leaf_names_recur(next);
        }
    }

    fn to_algebraic_string_recur(&self, depth: usize) -> String {
        if self.is_leaf() {
            let prefix = if self.phase { "" } else { "!" };
            return format!("{prefix}{}", self.name.as_deref().unwrap_or("_"));
        }

        let separator = match self.kind {
            TreeNodeKind::Or => "+",
            TreeNodeKind::And => "*",
        };
        let expression = self
            .sons
            .iter()
            .map(|son| son.to_algebraic_string_recur(depth + 1))
            .collect::<Vec<_>>()
            .join(separator);

        if self.kind == TreeNodeKind::Or && depth > 0 {
            format!("({expression})")
        } else {
            expression
        }
    }

    fn canonicalize(&mut self) {
        for son in &mut self.sons {
            son.canonicalize();
        }
        self.sons.sort_by(shape_cmp);
        self.compute_metrics();
    }

    fn compute_metrics(&mut self) {
        if self.is_leaf() {
            self.series = 1;
            self.parallel = 1;
            self.level = 0;
            return;
        }

        for son in &mut self.sons {
            son.compute_metrics();
        }

        self.level = self.sons.iter().map(|son| son.level).max().unwrap_or(0) + 1;

        match self.kind {
            TreeNodeKind::And => {
                self.series = self.sons.iter().map(|son| son.series).sum();
                self.parallel = self.sons.iter().map(|son| son.parallel).max().unwrap_or(0);
            }
            TreeNodeKind::Or => {
                self.series = self.sons.iter().map(|son| son.series).max().unwrap_or(0);
                self.parallel = self.sons.iter().map(|son| son.parallel).sum();
            }
        }
    }

    fn shape_key(&self) -> String {
        if self.is_leaf() {
            return "l".to_string();
        }

        let children = self
            .sons
            .iter()
            .map(Self::shape_key)
            .collect::<Vec<_>>()
            .join(",");
        format!("n({children})")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AoiError {
    EmptyPartition,
    InvalidLimit { series: usize, parallel: usize },
}

impl fmt::Display for AoiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPartition => write!(f, "cannot generate an empty partition"),
            Self::InvalidLimit { series, parallel } => {
                write!(
                    f,
                    "series and parallel limits must be positive, got {series}/{parallel}"
                )
            }
        }
    }
}

impl std::error::Error for AoiError {}

pub fn generate_complex_gates(
    level: usize,
    series: usize,
    parallel: usize,
) -> Result<Vec<AoiTree>, AoiError> {
    validate_limits(series, parallel)?;

    let mut and_forms = gen_complex_gates(level, series, parallel, TreeNodeKind::And)?;
    let mut or_forms = gen_complex_gates(level, series, parallel, TreeNodeKind::Or)?;

    if !or_forms.is_empty() {
        or_forms.remove(0);
    }

    and_forms.extend(or_forms);
    Ok(and_forms)
}

pub fn gen_complex_gates(
    level: usize,
    series: usize,
    parallel: usize,
    root_kind: TreeNodeKind,
) -> Result<Vec<AoiTree>, AoiError> {
    validate_limits(series, parallel)?;

    if level == 0 || (series <= 1 && parallel <= 1) {
        return Ok(vec![AoiTree::leaf(root_kind)]);
    }

    let mut unique = BTreeMap::<String, AoiTree>::new();
    insert_unique(&mut unique, AoiTree::leaf(root_kind));

    let partition_limit = match root_kind {
        TreeNodeKind::And => series,
        TreeNodeKind::Or => parallel,
    };

    for partition in partitions_descending(partition_limit)? {
        let largest = partition[0];
        let mut forms = match root_kind {
            TreeNodeKind::And => {
                gen_complex_gates(level - 1, largest, parallel, root_kind.reverse())?
            }
            TreeNodeKind::Or => gen_complex_gates(level - 1, series, largest, root_kind.reverse())?,
        };

        match root_kind {
            TreeNodeKind::And => forms.sort_by_key(|tree| tree.series),
            TreeNodeKind::Or => forms.sort_by_key(|tree| tree.parallel),
        }

        let eligible_counts = partition
            .iter()
            .map(|limit| {
                forms
                    .iter()
                    .filter(|tree| match root_kind {
                        TreeNodeKind::And => tree.series <= *limit,
                        TreeNodeKind::Or => tree.parallel <= *limit,
                    })
                    .count()
            })
            .collect::<Vec<_>>();

        for indices in nonincreasing_combinations(&eligible_counts) {
            let sons = indices
                .into_iter()
                .map(|index| forms[index].clone())
                .collect::<Vec<_>>();
            insert_unique(&mut unique, AoiTree::node(root_kind, sons));
        }
    }

    Ok(unique.into_values().collect())
}

fn validate_limits(series: usize, parallel: usize) -> Result<(), AoiError> {
    if series == 0 || parallel == 0 {
        Err(AoiError::InvalidLimit { series, parallel })
    } else {
        Ok(())
    }
}

fn insert_unique(unique: &mut BTreeMap<String, AoiTree>, mut tree: AoiTree) {
    tree.canonicalize();
    unique.entry(tree.shape_key()).or_insert(tree);
}

fn shape_cmp(left: &AoiTree, right: &AoiTree) -> Ordering {
    left.sons.len().cmp(&right.sons.len()).then_with(|| {
        left.sons
            .iter()
            .zip(&right.sons)
            .map(|(left, right)| shape_cmp(left, right))
            .find(|ordering| *ordering != Ordering::Equal)
            .unwrap_or(Ordering::Equal)
    })
}

fn partitions_descending(total: usize) -> Result<Vec<Vec<usize>>, AoiError> {
    if total == 0 {
        return Err(AoiError::EmptyPartition);
    }

    let mut partitions = Vec::new();
    let mut current = Vec::new();
    partitions_descending_recur(total, total, &mut current, &mut partitions);
    Ok(partitions)
}

fn partitions_descending_recur(
    remaining: usize,
    maximum: usize,
    current: &mut Vec<usize>,
    partitions: &mut Vec<Vec<usize>>,
) {
    if remaining == 0 {
        partitions.push(current.clone());
        return;
    }

    for value in (1..=remaining.min(maximum)).rev() {
        current.push(value);
        partitions_descending_recur(remaining - value, value, current, partitions);
        current.pop();
    }
}

fn nonincreasing_combinations(limits: &[usize]) -> Vec<Vec<usize>> {
    if limits.is_empty() || limits.iter().any(|limit| *limit == 0) {
        return Vec::new();
    }

    let mut output = Vec::new();
    let mut current = vec![0usize; limits.len()];
    nonincreasing_combinations_recur(limits, 0, usize::MAX, &mut current, &mut output);
    output
}

fn nonincreasing_combinations_recur(
    limits: &[usize],
    index: usize,
    previous: usize,
    current: &mut [usize],
    output: &mut Vec<Vec<usize>>,
) {
    if index == limits.len() {
        output.push(current.to_vec());
        return;
    }

    let maximum = (limits[index] - 1).min(previous);
    for value in 0..=maximum {
        current[index] = value;
        nonincreasing_combinations_recur(limits, index + 1, value, current, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_zero_generates_single_leaf() {
        let forms = gen_complex_gates(0, 4, 3, TreeNodeKind::And).unwrap();

        assert_eq!(forms.len(), 1);
        assert!(forms[0].is_leaf());
        assert_eq!(forms[0].series, 1);
        assert_eq!(forms[0].parallel, 1);
        assert_eq!(forms[0].level, 0);
    }

    #[test]
    fn one_level_and_forms_include_leaf_unary_and_binary_shapes() {
        let forms = gen_complex_gates(1, 2, 1, TreeNodeKind::And).unwrap();
        let shapes = forms.iter().map(AoiTree::shape_key).collect::<Vec<_>>();

        assert_eq!(shapes, vec!["l", "n(l)", "n(l,l)"]);
        assert_eq!(forms[2].series, 2);
        assert_eq!(forms[2].parallel, 1);
    }

    #[test]
    fn one_level_or_forms_track_parallel_limit() {
        let forms = gen_complex_gates(1, 1, 3, TreeNodeKind::Or).unwrap();
        let metrics = forms
            .iter()
            .map(|tree| (tree.shape_key(), tree.series, tree.parallel))
            .collect::<Vec<_>>();

        assert_eq!(
            metrics,
            vec![
                ("l".to_string(), 1, 1),
                ("n(l)".to_string(), 1, 1),
                ("n(l,l)".to_string(), 1, 2),
                ("n(l,l,l)".to_string(), 1, 3),
            ]
        );
    }

    #[test]
    fn generate_complex_gates_drops_duplicate_or_leaf() {
        let forms = generate_complex_gates(0, 2, 2).unwrap();

        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].kind, TreeNodeKind::And);
        assert!(forms[0].is_leaf());
    }

    #[test]
    fn recursively_builds_alternating_trees() {
        let forms = gen_complex_gates(2, 2, 2, TreeNodeKind::And).unwrap();

        assert!(forms.iter().any(|tree| {
            tree.kind == TreeNodeKind::And
                && tree.level == 2
                && tree.series == 2
                && tree.parallel == 2
        }));
    }

    #[test]
    fn canonicalizes_child_order_and_removes_duplicates() {
        let left = AoiTree::node(TreeNodeKind::Or, vec![AoiTree::leaf(TreeNodeKind::And)]);
        let right = AoiTree::leaf(TreeNodeKind::And);
        let tree = AoiTree::node(TreeNodeKind::And, vec![left.clone(), right.clone()]);
        let reversed = AoiTree::node(TreeNodeKind::And, vec![right, left]);

        assert_eq!(tree.shape_key(), reversed.shape_key());
    }

    #[test]
    fn algebraic_printing_assigns_leaf_names_in_depth_first_order() {
        let tree = AoiTree::node(
            TreeNodeKind::Or,
            vec![
                AoiTree::leaf(TreeNodeKind::And),
                AoiTree::node(
                    TreeNodeKind::And,
                    vec![
                        AoiTree::leaf(TreeNodeKind::Or),
                        AoiTree::leaf(TreeNodeKind::Or),
                    ],
                ),
            ],
        );

        assert_eq!(tree.to_algebraic_string(), "a+b*c");
    }

    #[test]
    fn rejects_zero_limits() {
        assert_eq!(
            gen_complex_gates(1, 0, 1, TreeNodeKind::And),
            Err(AoiError::InvalidLimit {
                series: 0,
                parallel: 1,
            })
        );
    }
}
