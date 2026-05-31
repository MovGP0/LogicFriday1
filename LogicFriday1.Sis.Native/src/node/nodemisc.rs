//! Native Rust support routines for SIS-style node cover base changes.
//!
//! The routines in this file keep the legacy behavior in owned Rust data:
//! merge fanin bases, remap covers onto a new base, drop contradictory cubes
//! introduced by duplicate fanins, and reduce nodes to their minimum base.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaninRef {
    pub id: NodeId,
    pub name: String,
}

impl FaninRef {
    pub fn new(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Literal {
    Zero,
    One,
    DontCare,
}

impl Literal {
    fn as_value(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            Self::DontCare => None,
        }
    }

    fn from_value(value: Option<bool>) -> Self {
        match value {
            Some(false) => Self::Zero,
            Some(true) => Self::One,
            None => Self::DontCare,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    literals: Vec<Literal>,
}

impl Cube {
    pub fn new(literals: Vec<Literal>) -> Self {
        Self { literals }
    }

    pub fn one(input_count: usize) -> Self {
        Self {
            literals: vec![Literal::DontCare; input_count],
        }
    }

    pub fn literals(&self) -> &[Literal] {
        &self.literals
    }

    fn literal_count(&self) -> usize {
        self.literals
            .iter()
            .filter(|literal| **literal != Literal::DontCare)
            .count()
    }

    fn contains(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(left, right)| *left == Literal::DontCare || left == right)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    input_count: usize,
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(input_count: usize, cubes: Vec<Cube>) -> NodeMiscResult<Self> {
        for cube in &cubes {
            if cube.literals.len() != input_count {
                return Err(NodeMiscError::CoverArityMismatch {
                    expected: input_count,
                    actual: cube.literals.len(),
                });
            }
        }

        Ok(Self { input_count, cubes }.contained())
    }

    pub fn zero(input_count: usize) -> Self {
        Self {
            input_count,
            cubes: Vec::new(),
        }
    }

    pub fn one(input_count: usize) -> Self {
        Self {
            input_count,
            cubes: vec![Cube::one(input_count)],
        }
    }

    pub fn input_count(&self) -> usize {
        self.input_count
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn is_zero(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn is_one(&self) -> bool {
        self.cubes.len() == 1 && self.cubes[0].literal_count() == 0
    }

    fn remap(
        &self,
        old_fanins: &[FaninRef],
        new_fanins: &[FaninRef],
        compare: FaninCompare,
    ) -> Self {
        let permutation = old_fanins
            .iter()
            .map(|old_fanin| {
                new_fanins
                    .iter()
                    .position(|new_fanin| compare(old_fanin, new_fanin) == Ordering::Equal)
            })
            .collect::<Vec<_>>();

        self.permute(&permutation, new_fanins.len())
    }

    fn permute(&self, permutation: &[Option<usize>], new_input_count: usize) -> Self {
        let mut cubes = Vec::new();
        for cube in &self.cubes {
            let mut literals = vec![None; new_input_count];
            let mut contradictory = false;

            for (old_index, new_index) in permutation.iter().enumerate() {
                let Some(new_index) = new_index else {
                    continue;
                };
                let Some(value) = cube.literals[old_index].as_value() else {
                    continue;
                };

                match literals[*new_index] {
                    Some(existing) if existing != value => {
                        contradictory = true;
                        break;
                    }
                    _ => literals[*new_index] = Some(value),
                }
            }

            if !contradictory {
                cubes.push(Cube::new(
                    literals.into_iter().map(Literal::from_value).collect(),
                ));
            }
        }

        Self {
            input_count: new_input_count,
            cubes,
        }
        .contained()
    }

    fn used_inputs(&self) -> Vec<usize> {
        (0..self.input_count)
            .filter(|index| {
                self.cubes
                    .iter()
                    .any(|cube| cube.literals[*index] != Literal::DontCare)
            })
            .collect()
    }

    fn project(&self, used_inputs: &[usize]) -> Self {
        Self {
            input_count: used_inputs.len(),
            cubes: self
                .cubes
                .iter()
                .map(|cube| {
                    Cube::new(
                        used_inputs
                            .iter()
                            .map(|index| cube.literals[*index])
                            .collect(),
                    )
                })
                .collect(),
        }
        .contained()
    }

    fn contained(mut self) -> Self {
        let mut unique = Vec::new();
        for cube in self.cubes.drain(..) {
            if !unique.contains(&cube) {
                unique.push(cube);
            }
        }

        let mut reduced = Vec::new();
        'cubes: for (index, cube) in unique.iter().enumerate() {
            for (other_index, other) in unique.iter().enumerate() {
                if index != other_index && other.contains(cube) {
                    continue 'cubes;
                }
            }

            reduced.push(cube.clone());
        }

        Self {
            input_count: self.input_count,
            cubes: reduced,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiscNode {
    pub id: NodeId,
    pub name: Option<String>,
    pub kind: NodeKind,
    pub fanins: Vec<FaninRef>,
    pub function: Option<Cover>,
    pub is_dup_free: bool,
    pub is_scc_minimal: bool,
    pub fanin_changed: bool,
}

impl MiscNode {
    pub fn new(id: NodeId, function: Cover, fanins: Vec<FaninRef>) -> NodeMiscResult<Self> {
        if function.input_count() != fanins.len() {
            return Err(NodeMiscError::CoverArityMismatch {
                expected: fanins.len(),
                actual: function.input_count(),
            });
        }

        Ok(Self {
            id,
            name: None,
            kind: NodeKind::Internal,
            fanins,
            function: Some(function),
            is_dup_free: false,
            is_scc_minimal: false,
            fanin_changed: false,
        })
    }

    pub fn primary_input(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: Some(name.into()),
            kind: NodeKind::PrimaryInput,
            fanins: Vec::new(),
            function: None,
            is_dup_free: true,
            is_scc_minimal: true,
            fanin_changed: false,
        }
    }

    pub fn primary_output(id: NodeId, name: impl Into<String>, fanin: FaninRef) -> Self {
        Self {
            id,
            name: Some(name.into()),
            kind: NodeKind::PrimaryOutput,
            fanins: vec![fanin],
            function: None,
            is_dup_free: true,
            is_scc_minimal: true,
            fanin_changed: false,
        }
    }

    fn required_function(&self, operation: &'static str) -> NodeMiscResult<&Cover> {
        self.function
            .as_ref()
            .ok_or(NodeMiscError::MissingFunction { operation })
    }
}

type FaninCompare = fn(&FaninRef, &FaninRef) -> Ordering;
pub type NodeMiscResult<T> = Result<T, NodeMiscError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeMiscError {
    MissingFunction { operation: &'static str },
    CoverArityMismatch { expected: usize, actual: usize },
}

impl fmt::Display for NodeMiscError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFunction { operation } => {
                write!(f, "{operation} requires a node with a Boolean function")
            }
            Self::CoverArityMismatch { expected, actual } => {
                write!(f, "cover has {actual} inputs, expected {expected}")
            }
        }
    }
}

impl Error for NodeMiscError {}

pub fn compare_fanin_by_id(left: &FaninRef, right: &FaninRef) -> Ordering {
    left.id.cmp(&right.id)
}

pub fn compare_fanin_by_name(left: &FaninRef, right: &FaninRef) -> Ordering {
    left.name.cmp(&right.name)
}

pub fn make_common_base(
    f: &MiscNode,
    g: &MiscNode,
) -> NodeMiscResult<(Vec<FaninRef>, Cover, Cover)> {
    make_common_base_with_compare(f, g, compare_fanin_by_id)
}

pub fn make_common_base_by_name(
    f: &MiscNode,
    g: &MiscNode,
) -> NodeMiscResult<(Vec<FaninRef>, Cover, Cover)> {
    make_common_base_with_compare(f, g, compare_fanin_by_name)
}

pub fn adjust_cover_to_base(node: &mut MiscNode, new_fanins: &[FaninRef]) -> NodeMiscResult<Cover> {
    adjust_cover_to_base_with_compare(node, new_fanins, compare_fanin_by_id)
}

pub fn adjust_cover_to_base_by_name(
    node: &mut MiscNode,
    new_fanins: &[FaninRef],
) -> NodeMiscResult<Cover> {
    adjust_cover_to_base_with_compare(node, new_fanins, compare_fanin_by_name)
}

pub fn node_minimum_base(node: &mut MiscNode) -> NodeMiscResult<()> {
    if matches!(node.kind, NodeKind::PrimaryInput | NodeKind::PrimaryOutput) {
        return Ok(());
    }

    node.required_function("node_minimum_base")?;
    node_remove_dup_fanin(node)?;

    let function = node.required_function("node_minimum_base")?;
    let is_zero = function.is_zero();
    let is_one = function.is_one();
    if is_zero || is_one {
        node.fanins.clear();
        node.function = Some(if is_zero {
            Cover::zero(0)
        } else {
            Cover::one(0)
        });
        node.is_dup_free = true;
        node.is_scc_minimal = true;
        return Ok(());
    }

    let used_inputs = function.used_inputs();
    if used_inputs.len() != function.input_count() {
        let new_fanins = used_inputs
            .iter()
            .map(|index| node.fanins[*index].clone())
            .collect::<Vec<_>>();
        let new_function = function.project(&used_inputs);
        node_replace_internal(node, new_fanins, new_function)?;
    }

    node.is_dup_free = true;
    node.is_scc_minimal = true;
    Ok(())
}

pub fn node_create(id: NodeId, function: Cover, fanins: Vec<FaninRef>) -> NodeMiscResult<MiscNode> {
    MiscNode::new(id, function, fanins)
}

pub fn node_replace_internal(
    node: &mut MiscNode,
    fanins: Vec<FaninRef>,
    function: Cover,
) -> NodeMiscResult<()> {
    if function.input_count() != fanins.len() {
        return Err(NodeMiscError::CoverArityMismatch {
            expected: fanins.len(),
            actual: function.input_count(),
        });
    }

    if node.kind == NodeKind::PrimaryInput {
        node.kind = NodeKind::Internal;
    }

    node.fanins = fanins;
    node.function = Some(function);
    node.fanin_changed = true;
    node.is_dup_free = has_unique_fanins(&node.fanins, compare_fanin_by_id);
    node.is_scc_minimal = false;
    Ok(())
}

pub fn node_replace(node: &mut MiscNode, replacement: MiscNode) -> NodeMiscResult<()> {
    let function = replacement
        .function
        .clone()
        .ok_or(NodeMiscError::MissingFunction {
            operation: "node_replace",
        })?;

    node_replace_internal(node, replacement.fanins, function)?;
    node.is_dup_free = replacement.is_dup_free;
    node.is_scc_minimal = replacement.is_scc_minimal;
    Ok(())
}

pub fn node_base_contains(f: &MiscNode, g: &MiscNode) -> bool {
    if f.fanins.len() < g.fanins.len() {
        return false;
    }

    let mut left = f.fanins.iter().map(|fanin| fanin.id).collect::<Vec<_>>();
    let mut right = g.fanins.iter().map(|fanin| fanin.id).collect::<Vec<_>>();
    left.sort();
    right.sort();

    let mut left_index = left.len();
    for right_fanin in right.iter().rev() {
        while left_index > 0 && left[left_index - 1] > *right_fanin {
            left_index -= 1;
        }

        if left_index == 0 || left[left_index - 1] < *right_fanin {
            return false;
        }

        left_index -= 1;
    }

    true
}

pub fn node_remove_dup_fanin(node: &mut MiscNode) -> NodeMiscResult<()> {
    let new_fanins = merge_fanin_list(std::iter::once(&*node), compare_fanin_by_id);
    if new_fanins.len() != node.fanins.len() {
        let new_function = adjust_cover_to_base(node, &new_fanins)?;
        node_replace_internal(node, new_fanins, new_function)?;
    }

    node.is_dup_free = true;
    Ok(())
}

fn make_common_base_with_compare(
    f: &MiscNode,
    g: &MiscNode,
    compare: FaninCompare,
) -> NodeMiscResult<(Vec<FaninRef>, Cover, Cover)> {
    let fanins = merge_fanin_list([f, g].into_iter(), compare);
    let mut left_node = f.clone();
    let mut right_node = g.clone();
    let left = adjust_cover_to_base_with_compare(&mut left_node, &fanins, compare)?;
    let right = adjust_cover_to_base_with_compare(&mut right_node, &fanins, compare)?;

    Ok((fanins, left, right))
}

fn adjust_cover_to_base_with_compare(
    node: &mut MiscNode,
    new_fanins: &[FaninRef],
    compare: FaninCompare,
) -> NodeMiscResult<Cover> {
    let has_dup = if node.is_dup_free {
        false
    } else {
        let has_dup = !has_unique_fanins(&node.fanins, compare);
        node.is_dup_free = !has_dup;
        has_dup
    };
    let function = node.required_function("adjust_cover_to_base")?;

    if new_fanins.is_empty() {
        return Ok(if function.is_zero() {
            Cover::zero(0)
        } else {
            Cover::one(0)
        });
    }

    if node.fanins.len() == new_fanins.len()
        && node
            .fanins
            .iter()
            .zip(new_fanins)
            .all(|(left, right)| compare(left, right) == Ordering::Equal)
    {
        return Ok(function.clone());
    }

    let adjusted = function.remap(&node.fanins, new_fanins, compare);
    if has_dup {
        Ok(adjusted.contained())
    } else {
        Ok(adjusted)
    }
}

fn merge_fanin_list<'a>(
    nodes: impl Iterator<Item = &'a MiscNode>,
    compare: FaninCompare,
) -> Vec<FaninRef> {
    let mut fanins = nodes
        .flat_map(|node| node.fanins.iter().rev().cloned())
        .collect::<Vec<_>>();
    fanins.sort_by(compare);
    fanins.dedup_by(|left, right| compare(left, right) == Ordering::Equal);
    fanins
}

fn has_unique_fanins(fanins: &[FaninRef], compare: FaninCompare) -> bool {
    let mut fanins = fanins.to_vec();
    fanins.sort_by(compare);
    fanins
        .windows(2)
        .all(|window| compare(&window[0], &window[1]) != Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fanin(id: usize, name: &str) -> FaninRef {
        FaninRef::new(NodeId(id), name)
    }

    fn node(id: usize, fanins: Vec<FaninRef>, cubes: Vec<Cube>) -> MiscNode {
        let function = Cover::new(fanins.len(), cubes).unwrap();
        MiscNode::new(NodeId(id), function, fanins).unwrap()
    }

    #[test]
    fn common_base_merges_by_id_and_remaps_covers() {
        let a = fanin(1, "a");
        let b = fanin(2, "b");
        let c = fanin(3, "c");
        let f = node(
            10,
            vec![c.clone(), a.clone()],
            vec![Cube::new(vec![Literal::One, Literal::Zero])],
        );
        let g = node(
            11,
            vec![b.clone(), a.clone()],
            vec![Cube::new(vec![Literal::Zero, Literal::One])],
        );

        let (base, f_cover, g_cover) = make_common_base(&f, &g).unwrap();

        assert_eq!(base, vec![a, b, c]);
        assert_eq!(
            f_cover.cubes(),
            &[Cube::new(vec![
                Literal::Zero,
                Literal::DontCare,
                Literal::One
            ])]
        );
        assert_eq!(
            g_cover.cubes(),
            &[Cube::new(vec![
                Literal::One,
                Literal::Zero,
                Literal::DontCare
            ])]
        );
    }

    #[test]
    fn common_base_by_name_uses_names_for_order_and_identity() {
        let left_a = fanin(1, "a");
        let left_b = fanin(2, "b");
        let right_a = fanin(20, "a");
        let right_c = fanin(3, "c");
        let f = node(
            10,
            vec![left_b, left_a.clone()],
            vec![Cube::new(vec![Literal::One, Literal::Zero])],
        );
        let g = node(
            11,
            vec![right_c, right_a],
            vec![Cube::new(vec![Literal::Zero, Literal::One])],
        );

        let (base, _, g_cover) = make_common_base_by_name(&f, &g).unwrap();

        assert_eq!(
            base.iter()
                .map(|fanin| fanin.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
        assert_eq!(base[0], left_a);
        assert_eq!(
            g_cover.cubes(),
            &[Cube::new(vec![
                Literal::One,
                Literal::DontCare,
                Literal::Zero
            ])]
        );
    }

    #[test]
    fn duplicate_fanin_removal_drops_contradictory_cubes() {
        let a = fanin(1, "a");
        let mut f = node(
            10,
            vec![a.clone(), a.clone()],
            vec![
                Cube::new(vec![Literal::One, Literal::Zero]),
                Cube::new(vec![Literal::One, Literal::One]),
            ],
        );

        node_remove_dup_fanin(&mut f).unwrap();

        assert_eq!(f.fanins, vec![a]);
        assert_eq!(
            f.function.as_ref().unwrap().cubes(),
            &[Cube::new(vec![Literal::One])]
        );
        assert!(f.is_dup_free);
        assert!(f.fanin_changed);
    }

    #[test]
    fn minimum_base_removes_unused_inputs_and_constants_have_empty_base() {
        let a = fanin(1, "a");
        let b = fanin(2, "b");
        let mut f = node(
            10,
            vec![a.clone(), b],
            vec![Cube::new(vec![Literal::Zero, Literal::DontCare])],
        );
        let mut one = node(11, vec![a], vec![Cube::one(1)]);

        node_minimum_base(&mut f).unwrap();
        node_minimum_base(&mut one).unwrap();

        assert_eq!(f.fanins, vec![fanin(1, "a")]);
        assert_eq!(f.function.as_ref().unwrap().input_count(), 1);
        assert_eq!(one.fanins, Vec::<FaninRef>::new());
        assert!(one.function.as_ref().unwrap().is_one());
    }

    #[test]
    fn base_contains_uses_sorted_id_multiset_behavior() {
        let a = fanin(1, "a");
        let b = fanin(2, "b");
        let c = fanin(3, "c");
        let f = node(10, vec![a.clone(), b.clone(), c], vec![Cube::one(3)]);
        let g = node(11, vec![b, a], vec![Cube::one(2)]);
        let h = node(12, vec![fanin(4, "d")], vec![Cube::one(1)]);

        assert!(node_base_contains(&f, &g));
        assert!(!node_base_contains(&f, &h));
    }

    #[test]
    fn replace_internal_turns_primary_input_into_internal_node() {
        let a = fanin(1, "a");
        let mut f = MiscNode::primary_input(NodeId(10), "f");
        let function = Cover::new(1, vec![Cube::new(vec![Literal::One])]).unwrap();

        node_replace_internal(&mut f, vec![a], function).unwrap();

        assert_eq!(f.kind, NodeKind::Internal);
        assert!(f.fanin_changed);
    }
}
