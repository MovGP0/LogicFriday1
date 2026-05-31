//! Native Boolean node operations for the SIS node package.
//!
//! This module models the behavior of the legacy `node_t` Boolean cover
//! operations with owned Rust data. It deliberately exposes Rust APIs only;
//! interop belongs at a higher integration boundary.

#![allow(dead_code)]

use std::error::Error;
use std::fmt;

pub type NodeResult<T> = Result<T, NodeError>;

const MAX_NATIVE_SNOCOMP_INPUTS: usize = 12;
const MAX_EXACT_SNOCOMP_PRIMES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeType {
    Internal,
    PrimaryInput,
    PrimaryOutput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    PrimaryInput,
    PrimaryOutput,
    Undefined,
    Zero,
    One,
    Buffer,
    Inverter,
    And,
    Or,
    Complex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimplifyMode {
    SingleCubeContainment,
    SimpleComplement,
    Espresso,
    Exact,
    ExactLiterals,
    DontCareSimplify,
    NoComplement,
    SingleOutputNoComplement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeError {
    MissingFunction {
        operation: &'static str,
    },
    InvalidConstantPhase(i32),
    InvalidLiteralPhase(i32),
    IncompatibleCubeSize {
        expected: usize,
        actual: usize,
    },
    FunctionIsNotMinimumBase,
    NativeMinimizationTooLarge {
        operation: &'static str,
        inputs: usize,
        max_inputs: usize,
    },
    NativeSupportUnavailable {
        operation: &'static str,
    },
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFunction { operation } => {
                write!(f, "{operation} requires a node with a Boolean function")
            }
            Self::InvalidConstantPhase(phase) => {
                write!(f, "constant phase must be 0 or 1, got {phase}")
            }
            Self::InvalidLiteralPhase(phase) => {
                write!(f, "literal phase must be 0 or 1, got {phase}")
            }
            Self::IncompatibleCubeSize { expected, actual } => {
                write!(f, "cube has {actual} inputs, expected {expected}")
            }
            Self::FunctionIsNotMinimumBase => {
                write!(f, "node function is not in minimum base")
            }
            Self::NativeMinimizationTooLarge {
                operation,
                inputs,
                max_inputs,
            } => {
                write!(
                    f,
                    "{operation} supports at most {max_inputs} inputs, got {inputs}"
                )
            }
            Self::NativeSupportUnavailable { operation } => {
                write!(f, "{operation} requires native minimization support")
            }
        }
    }
}

impl Error for NodeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    inputs: Vec<Option<bool>>,
}

impl Cube {
    pub fn new(inputs: Vec<Option<bool>>) -> Self {
        Self { inputs }
    }

    pub fn literal(input: usize, input_count: usize, phase: bool) -> Self {
        let mut inputs = vec![None; input_count];
        inputs[input] = Some(phase);
        Self { inputs }
    }

    pub fn tautology(input_count: usize) -> Self {
        Self {
            inputs: vec![None; input_count],
        }
    }

    pub fn inputs(&self) -> &[Option<bool>] {
        &self.inputs
    }

    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    pub fn literal_count(&self) -> usize {
        self.inputs.iter().filter(|input| input.is_some()).count()
    }

    pub fn covers(&self, other: &Self) -> bool {
        self.inputs
            .iter()
            .zip(&other.inputs)
            .all(|(left, right)| left.is_none() || left == right)
    }

    fn intersect(&self, other: &Self) -> Option<Self> {
        let mut inputs = Vec::with_capacity(self.inputs.len());
        for (left, right) in self.inputs.iter().zip(&other.inputs) {
            match (left, right) {
                (Some(left), Some(right)) if left != right => return None,
                (Some(value), _) | (_, Some(value)) => inputs.push(Some(*value)),
                (None, None) => inputs.push(None),
            }
        }

        Some(Self { inputs })
    }

    fn merge_distance_one(&self, other: &Self) -> Option<Self> {
        let mut difference = None;
        let mut inputs = self.inputs.clone();

        for (index, (left, right)) in self.inputs.iter().zip(&other.inputs).enumerate() {
            if left == right {
                continue;
            }

            match (left, right) {
                (Some(left), Some(right)) if left != right && difference.is_none() => {
                    difference = Some(index);
                    inputs[index] = None;
                }
                _ => return None,
            }
        }

        difference.map(|_| Self { inputs })
    }

    fn evaluate(&self, assignment: &[bool]) -> bool {
        self.inputs
            .iter()
            .zip(assignment)
            .all(|(input, value)| input.map_or(true, |required| required == *value))
    }

    fn remap(&self, old_fanins: &[String], new_fanins: &[String]) -> Self {
        let mut inputs = vec![None; new_fanins.len()];
        for (old_index, old_name) in old_fanins.iter().enumerate() {
            if let Some(new_index) = new_fanins.iter().position(|new_name| new_name == old_name) {
                inputs[new_index] = self.inputs[old_index];
            }
        }

        Self { inputs }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover {
    input_count: usize,
    cubes: Vec<Cube>,
}

impl Cover {
    pub fn new(input_count: usize, cubes: Vec<Cube>) -> NodeResult<Self> {
        for cube in &cubes {
            if cube.input_count() != input_count {
                return Err(NodeError::IncompatibleCubeSize {
                    expected: input_count,
                    actual: cube.input_count(),
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
            cubes: vec![Cube::tautology(input_count)],
        }
    }

    pub fn cubes(&self) -> &[Cube] {
        &self.cubes
    }

    pub fn input_count(&self) -> usize {
        self.input_count
    }

    pub fn cube_count(&self) -> usize {
        self.cubes.len()
    }

    pub fn is_zero(&self) -> bool {
        self.cubes.is_empty()
    }

    pub fn is_one(&self) -> bool {
        self.cubes.len() == 1 && self.cubes[0].literal_count() == 0
    }

    pub fn literal_count(&self) -> usize {
        self.cubes.iter().map(Cube::literal_count).sum()
    }

    pub fn literal_counts(&self) -> Vec<usize> {
        let mut counts = vec![0; self.input_count * 2];
        for cube in &self.cubes {
            for (index, input) in cube.inputs.iter().enumerate() {
                if let Some(value) = input {
                    counts[index * 2 + usize::from(*value)] += 1;
                }
            }
        }

        counts
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut cubes = self.cubes.clone();
        cubes.extend(other.cubes.iter().cloned());
        Self {
            input_count: self.input_count,
            cubes,
        }
        .contained()
    }

    pub fn intersection(&self, other: &Self) -> Self {
        let mut cubes = Vec::new();
        for left in &self.cubes {
            for right in &other.cubes {
                if let Some(cube) = left.intersect(right) {
                    cubes.push(cube);
                }
            }
        }

        Self {
            input_count: self.input_count,
            cubes,
        }
        .contained()
    }

    pub fn complement(&self) -> Self {
        let mut cubes = Vec::new();
        visit_assignments(self.input_count, &mut Vec::new(), &mut |assignment| {
            if !self.evaluate(assignment) {
                cubes.push(Cube::new(assignment.iter().copied().map(Some).collect()));
            }
        });

        Self {
            input_count: self.input_count,
            cubes,
        }
        .distance_one_merged()
        .contained()
    }

    pub fn contains(&self, other: &Self) -> bool {
        let mut contains = true;
        visit_assignments(self.input_count, &mut Vec::new(), &mut |assignment| {
            if other.evaluate(assignment) && !self.evaluate(assignment) {
                contains = false;
            }
        });

        contains
    }

    pub fn equals(&self, other: &Self) -> bool {
        self.contains(other) && other.contains(self)
    }

    pub fn distance_one_merged(&self) -> Self {
        let mut current = self.clone().contained();
        loop {
            let mut changed = false;
            let mut next = Vec::new();
            let mut used = vec![false; current.cubes.len()];

            'outer: for left_index in 0..current.cubes.len() {
                if used[left_index] {
                    continue;
                }

                for right_index in (left_index + 1)..current.cubes.len() {
                    if used[right_index] {
                        continue;
                    }

                    if let Some(merged) =
                        current.cubes[left_index].merge_distance_one(&current.cubes[right_index])
                    {
                        next.push(merged);
                        used[left_index] = true;
                        used[right_index] = true;
                        changed = true;
                        continue 'outer;
                    }
                }

                next.push(current.cubes[left_index].clone());
                used[left_index] = true;
            }

            current = Self {
                input_count: current.input_count,
                cubes: next,
            }
            .contained();

            if !changed {
                return current;
            }
        }
    }

    fn evaluate(&self, assignment: &[bool]) -> bool {
        self.cubes.iter().any(|cube| cube.evaluate(assignment))
    }

    fn remap(&self, old_fanins: &[String], new_fanins: &[String]) -> Self {
        Self {
            input_count: new_fanins.len(),
            cubes: self
                .cubes
                .iter()
                .map(|cube| cube.remap(old_fanins, new_fanins))
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
        'cube: for (index, cube) in unique.iter().enumerate() {
            for (other_index, other) in unique.iter().enumerate() {
                if index != other_index && other.covers(cube) {
                    continue 'cube;
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
pub struct Node {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub node_type: NodeType,
    pub fanins: Vec<String>,
    function: Option<Cover>,
    complement: Option<Cover>,
    pub is_dup_free: bool,
    pub is_scc_minimal: bool,
}

impl Node {
    pub fn new(function: Cover, fanins: Vec<String>) -> Self {
        let mut node = Self {
            name: None,
            short_name: None,
            node_type: NodeType::Internal,
            fanins,
            function: Some(function),
            complement: None,
            is_dup_free: false,
            is_scc_minimal: false,
        };
        node.minimum_base();
        node
    }

    pub fn primary_input(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: Some(name.clone()),
            short_name: Some(name),
            node_type: NodeType::PrimaryInput,
            fanins: Vec::new(),
            function: None,
            complement: None,
            is_dup_free: true,
            is_scc_minimal: true,
        }
    }

    pub fn primary_output(name: impl Into<String>, fanin: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: Some(name.clone()),
            short_name: Some(name),
            node_type: NodeType::PrimaryOutput,
            fanins: vec![fanin.into()],
            function: None,
            complement: None,
            is_dup_free: true,
            is_scc_minimal: true,
        }
    }

    pub fn function(&self) -> Option<&Cover> {
        self.function.as_ref()
    }

    pub fn clear_names(&mut self) {
        self.name = None;
        self.short_name = None;
    }

    pub fn replace_with(&mut self, other: Node) {
        *self = other;
    }

    fn required_function(&self, operation: &'static str) -> NodeResult<&Cover> {
        self.function
            .as_ref()
            .ok_or(NodeError::MissingFunction { operation })
    }

    fn required_function_mut(&mut self, operation: &'static str) -> NodeResult<&mut Cover> {
        self.function
            .as_mut()
            .ok_or(NodeError::MissingFunction { operation })
    }

    fn minimum_base(&mut self) {
        let Some(function) = self.function.as_ref() else {
            return;
        };

        if function.is_zero() || function.is_one() {
            self.fanins.clear();
            self.function = Some(if function.is_zero() {
                Cover::zero(0)
            } else {
                Cover::one(0)
            });
            self.complement = None;
            return;
        }

        let used = (0..function.input_count)
            .filter(|index| {
                function
                    .cubes
                    .iter()
                    .any(|cube| cube.inputs[*index].is_some())
            })
            .collect::<Vec<_>>();

        if used.len() == function.input_count {
            return;
        }

        let fanins = used
            .iter()
            .filter_map(|index| self.fanins.get(*index).cloned())
            .collect::<Vec<_>>();
        let cubes = function
            .cubes
            .iter()
            .map(|cube| Cube::new(used.iter().map(|index| cube.inputs[*index]).collect()))
            .collect::<Vec<_>>();

        self.fanins = fanins;
        self.function = Some(
            Cover {
                input_count: used.len(),
                cubes,
            }
            .contained(),
        );
        self.complement = None;
    }
}

pub fn node_constant(phase: i32) -> NodeResult<Node> {
    let function = match phase {
        0 => Cover::zero(0),
        1 => Cover::one(0),
        value => return Err(NodeError::InvalidConstantPhase(value)),
    };

    let mut node = Node::new(function, Vec::new());
    node.is_dup_free = true;
    node.is_scc_minimal = true;
    Ok(node)
}

pub fn node_literal(fanin: impl Into<String>, phase: i32) -> NodeResult<Node> {
    let phase = match phase {
        0 => false,
        1 => true,
        value => return Err(NodeError::InvalidLiteralPhase(value)),
    };

    let fanins = vec![fanin.into()];
    let function = Cover::new(1, vec![Cube::literal(0, 1, phase)])?;
    let mut node = Node::new(function, fanins);
    node.is_dup_free = true;
    node.is_scc_minimal = true;
    Ok(node)
}

pub fn node_and(f: &Node, g: &Node) -> NodeResult<Node> {
    let f_function = f.required_function("node_and")?;
    let g_function = g.required_function("node_and")?;

    if f.fanins.is_empty() {
        return if f_function.is_zero() {
            node_constant(0)
        } else {
            Ok(dup_without_names(g))
        };
    }

    if g.fanins.is_empty() {
        return if g_function.is_zero() {
            node_constant(0)
        } else {
            Ok(dup_without_names(f))
        };
    }

    let (fanins, left, right) = common_base(f, g, false)?;
    let mut node = Node::new(left.intersection(&right), fanins);
    node.is_dup_free = true;
    node.is_scc_minimal = true;
    node.minimum_base();
    Ok(node)
}

pub fn node_or(f: &Node, g: &Node) -> NodeResult<Node> {
    let f_function = f.required_function("node_or")?;
    let g_function = g.required_function("node_or")?;

    if f.fanins.is_empty() {
        return if f_function.is_zero() {
            Ok(dup_without_names(g))
        } else {
            node_constant(1)
        };
    }

    if g.fanins.is_empty() {
        return if g_function.is_zero() {
            Ok(dup_without_names(f))
        } else {
            node_constant(1)
        };
    }

    let (fanins, left, right) = common_base(f, g, false)?;
    let mut node = Node::new(left.union(&right), fanins);
    node.is_dup_free = true;
    node.is_scc_minimal = true;
    node.minimum_base();
    Ok(node)
}

pub fn node_not(f: &Node) -> NodeResult<Node> {
    let function = f.required_function("node_not")?;
    if f.fanins.is_empty() {
        return node_constant(i32::from(function.is_zero()));
    }

    let mut node = Node::new(function.complement(), f.fanins.clone());
    node.is_dup_free = f.is_dup_free;
    node.is_scc_minimal = true;
    node.minimum_base();
    Ok(node)
}

pub fn node_xor(f: &Node, g: &Node) -> NodeResult<Node> {
    let fbar = node_not(f)?;
    let gbar = node_not(g)?;
    let t0 = node_and(&fbar, g)?;
    let t1 = node_and(f, &gbar)?;
    node_or(&t0, &t1)
}

pub fn node_xnor(f: &Node, g: &Node) -> NodeResult<Node> {
    let fbar = node_not(f)?;
    let gbar = node_not(g)?;
    let t0 = node_and(f, g)?;
    let t1 = node_and(&fbar, &gbar)?;
    node_or(&t0, &t1)
}

pub fn node_largest_cube_divisor(f: &Node) -> NodeResult<Node> {
    let function = f.required_function("node_largest_cube_divisor")?;
    if f.fanins.is_empty() {
        return node_constant(1);
    }

    let mut inputs = vec![None; function.input_count()];
    for index in 0..function.input_count() {
        let mut value = None;
        let mut fixed = true;
        for cube in function.cubes() {
            match (value, cube.inputs[index]) {
                (None, Some(input)) => value = Some(input),
                (Some(left), Some(right)) if left == right => {}
                _ => {
                    fixed = false;
                    break;
                }
            }
        }

        if fixed {
            inputs[index] = value;
        }
    }

    let mut node = Node::new(
        Cover::new(function.input_count(), vec![Cube::new(inputs)])?,
        f.fanins.clone(),
    );
    node.is_dup_free = f.is_dup_free;
    node.is_scc_minimal = true;
    node.minimum_base();
    Ok(node)
}

pub fn node_contains(f: &Node, g: &Node) -> NodeResult<bool> {
    f.required_function("node_contains")?;
    g.required_function("node_contains")?;
    let (_, left, right) = common_base(f, g, false)?;
    Ok(left.contains(&right))
}

pub fn node_equal(f: &Node, g: &Node) -> NodeResult<bool> {
    Ok(node_contains(f, g)? && node_contains(g, f)?)
}

pub fn node_equal_by_name(f: &Node, g: &Node) -> NodeResult<bool> {
    if matches!(
        f.node_type,
        NodeType::PrimaryInput | NodeType::PrimaryOutput
    ) {
        return Ok(f.node_type == g.node_type);
    }

    if matches!(
        g.node_type,
        NodeType::PrimaryInput | NodeType::PrimaryOutput
    ) {
        return Ok(f.node_type == g.node_type);
    }

    f.required_function("node_equal_by_name")?;
    g.required_function("node_equal_by_name")?;
    let (_, left, right) = common_base(f, g, true)?;
    Ok(left.equals(&right))
}

pub fn node_sort_for_printing(f: &Node) -> NodeResult<Node> {
    if matches!(
        f.node_type,
        NodeType::PrimaryInput | NodeType::PrimaryOutput
    ) {
        return Ok(f.clone());
    }

    let function = f.required_function("node_sort_for_printing")?;
    if f.fanins.is_empty() || function.is_zero() {
        return Ok(f.clone());
    }

    let mut fanins = f.fanins.clone();
    fanins.sort();
    let mut cover = function.remap(&f.fanins, &fanins);
    cover
        .cubes
        .sort_by(|left, right| left.inputs.cmp(&right.inputs));

    let mut node = Node::new(cover, fanins);
    node.is_dup_free = true;
    node.is_scc_minimal = true;
    Ok(node)
}

pub fn node_function(node: &Node) -> NodeResult<NodeFunction> {
    match node.node_type {
        NodeType::PrimaryInput => return Ok(NodeFunction::PrimaryInput),
        NodeType::PrimaryOutput => return Ok(NodeFunction::PrimaryOutput),
        NodeType::Internal => {}
    }

    let function = match node.function.as_ref() {
        Some(function) => function,
        None => return Ok(NodeFunction::Undefined),
    };

    if function.is_zero() {
        if !node.fanins.is_empty() {
            return Err(NodeError::FunctionIsNotMinimumBase);
        }

        return Ok(NodeFunction::Zero);
    }

    if function.cube_count() == 1 {
        let cube = &function.cubes()[0];
        if node.fanins.is_empty() {
            return Ok(NodeFunction::One);
        }

        if node.fanins.len() == 1 {
            return match cube.inputs[0] {
                Some(true) => Ok(NodeFunction::Buffer),
                Some(false) => Ok(NodeFunction::Inverter),
                None => Err(NodeError::FunctionIsNotMinimumBase),
            };
        }

        if cube.literal_count() != node.fanins.len() {
            return Err(NodeError::FunctionIsNotMinimumBase);
        }

        return Ok(NodeFunction::And);
    }

    if function
        .cubes()
        .iter()
        .all(|cube| cube.literal_count() == 1)
    {
        Ok(NodeFunction::Or)
    } else {
        Ok(NodeFunction::Complex)
    }
}

pub fn node_type(node: &Node) -> NodeType {
    node.node_type
}

pub fn node_simplify_replace(
    f: &mut Node,
    d: Option<&Node>,
    mode: SimplifyMode,
) -> NodeResult<bool> {
    let new_f = node_simplify(f, d, mode)?;
    if node_num_literal(&new_f)? < node_num_literal(f)? {
        f.replace_with(new_f);
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn node_simplify(f: &Node, d: Option<&Node>, mode: SimplifyMode) -> NodeResult<Node> {
    let function = f.required_function("node_simplify")?;
    if function.is_zero() || f.fanins.is_empty() {
        return Ok(f.clone());
    }

    if let Some(d) = d {
        d.required_function("node_simplify")?;
    }

    match mode {
        SimplifyMode::SingleCubeContainment | SimplifyMode::SimpleComplement => {
            let mut node = Node::new(function.clone().contained(), f.fanins.clone());
            node.is_dup_free = true;
            node.is_scc_minimal = true;
            node.minimum_base();
            Ok(node)
        }
        SimplifyMode::SingleOutputNoComplement => simplify_single_output_no_complement(f, d),
        SimplifyMode::Espresso
        | SimplifyMode::Exact
        | SimplifyMode::ExactLiterals
        | SimplifyMode::DontCareSimplify
        | SimplifyMode::NoComplement => Err(NodeError::NativeSupportUnavailable {
            operation: "cover minimization",
        }),
    }
}

pub fn node_scc(node: &mut Node) {
    node.is_scc_minimal = false;
    node.is_dup_free = false;
    node.minimum_base();
}

pub fn node_num_literal(node: &Node) -> NodeResult<usize> {
    if matches!(
        node.node_type,
        NodeType::PrimaryInput | NodeType::PrimaryOutput
    ) {
        return Ok(0);
    }

    let function = node.required_function("node_num_literal")?;
    if function.is_zero() || node.fanins.is_empty() {
        return Ok(0);
    }

    Ok(function.literal_count())
}

pub fn node_num_cube(node: &Node) -> NodeResult<usize> {
    if matches!(
        node.node_type,
        NodeType::PrimaryInput | NodeType::PrimaryOutput
    ) {
        return Ok(0);
    }

    Ok(node.required_function("node_num_cube")?.cube_count())
}

pub fn node_literal_count(node: &Node) -> NodeResult<Vec<usize>> {
    if matches!(
        node.node_type,
        NodeType::PrimaryInput | NodeType::PrimaryOutput
    ) {
        return Ok(Vec::new());
    }

    let function = node.required_function("node_literal_count")?;
    if node.fanins.is_empty() {
        return Ok(vec![0]);
    }

    Ok(function.literal_counts())
}

pub fn node_complement(node: &mut Node) -> NodeResult<()> {
    if node.complement.is_some() {
        return Ok(());
    }

    if matches!(
        node.node_type,
        NodeType::PrimaryInput | NodeType::PrimaryOutput
    ) {
        return Ok(());
    }

    let function = node.required_function("node_complement")?;
    node.complement = Some(function.complement());
    Ok(())
}

pub fn node_d1merge(node: &mut Node) -> NodeResult<()> {
    let function = node.required_function_mut("node_d1merge")?;
    *function = function.distance_one_merged();
    node.is_dup_free = true;
    node.is_scc_minimal = true;
    node.minimum_base();
    Ok(())
}

fn dup_without_names(node: &Node) -> Node {
    let mut node = node.clone();
    node.clear_names();
    node
}

fn common_base(f: &Node, g: &Node, sort_by_name: bool) -> NodeResult<(Vec<String>, Cover, Cover)> {
    let left = f.required_function("common_base")?;
    let right = g.required_function("common_base")?;
    let mut fanins = f.fanins.clone();

    for fanin in &g.fanins {
        if !fanins.contains(fanin) {
            fanins.push(fanin.clone());
        }
    }

    if sort_by_name {
        fanins.sort();
    }

    Ok((
        fanins.clone(),
        left.remap(&f.fanins, &fanins),
        right.remap(&g.fanins, &fanins),
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeImplicant {
    cube: Cube,
    covered_onset: Vec<usize>,
}

fn simplify_single_output_no_complement(f: &Node, d: Option<&Node>) -> NodeResult<Node> {
    let function = f.required_function("node_simplify")?;
    let (fanins, onset, dont_care) = if let Some(d) = d {
        let (fanins, onset, dont_care) = common_base(f, d, false)?;
        (fanins, onset, dont_care)
    } else {
        (
            f.fanins.clone(),
            function.clone(),
            Cover::zero(function.input_count()),
        )
    };

    let minimized = minimize_single_output_cover(&onset, &dont_care)?;
    let mut node = Node::new(minimized, fanins);
    node.is_dup_free = true;
    node.is_scc_minimal = true;
    node.minimum_base();
    Ok(node)
}

fn minimize_single_output_cover(onset: &Cover, dont_care: &Cover) -> NodeResult<Cover> {
    let input_count = onset.input_count();
    if input_count != dont_care.input_count() {
        return Err(NodeError::IncompatibleCubeSize {
            expected: input_count,
            actual: dont_care.input_count(),
        });
    }
    if input_count > MAX_NATIVE_SNOCOMP_INPUTS {
        return Err(NodeError::NativeMinimizationTooLarge {
            operation: "single-output no-complement minimization",
            inputs: input_count,
            max_inputs: MAX_NATIVE_SNOCOMP_INPUTS,
        });
    }

    let (onset_minterms, allowed_minterms) = collect_minterms(onset, dont_care);
    if onset_minterms.is_empty() {
        return Ok(Cover::zero(input_count));
    }
    if allowed_minterms.len() == (1usize << input_count) {
        return Ok(Cover::one(input_count));
    }

    let primes = prime_implicants(input_count, &onset_minterms, &allowed_minterms);
    if primes.is_empty() {
        return Ok(onset.clone().contained());
    }

    let selected = select_implicants(&primes, &onset_minterms);
    Cover::new(
        input_count,
        selected
            .into_iter()
            .map(|index| primes[index].cube.clone())
            .collect(),
    )
}

fn collect_minterms(onset: &Cover, dont_care: &Cover) -> (Vec<usize>, Vec<usize>) {
    let mut onset_minterms = Vec::new();
    let mut allowed_minterms = Vec::new();
    visit_assignments(onset.input_count(), &mut Vec::new(), &mut |assignment| {
        let index = assignment_index(assignment);
        let in_onset = onset.evaluate(assignment);
        if in_onset {
            onset_minterms.push(index);
            allowed_minterms.push(index);
        } else if dont_care.evaluate(assignment) {
            allowed_minterms.push(index);
        }
    });
    (onset_minterms, allowed_minterms)
}

fn prime_implicants(
    input_count: usize,
    onset_minterms: &[usize],
    allowed_minterms: &[usize],
) -> Vec<NativeImplicant> {
    let mut current = allowed_minterms
        .iter()
        .copied()
        .map(|minterm| cube_from_minterm(minterm, input_count))
        .collect::<Vec<_>>();
    let mut primes = Vec::new();

    loop {
        let mut used = vec![false; current.len()];
        let mut next = Vec::new();
        for left_index in 0..current.len() {
            for right_index in (left_index + 1)..current.len() {
                if let Some(merged) = current[left_index].merge_distance_one(&current[right_index])
                {
                    if cube_is_allowed(&merged, input_count, allowed_minterms)
                        && !next.contains(&merged)
                    {
                        next.push(merged);
                    }
                    used[left_index] = true;
                    used[right_index] = true;
                }
            }
        }

        for (cube, was_used) in current.into_iter().zip(used) {
            let covered_onset = covered_onset_minterms(&cube, input_count, onset_minterms);
            if !was_used
                && !covered_onset.is_empty()
                && !primes
                    .iter()
                    .any(|prime: &NativeImplicant| prime.cube == cube)
            {
                primes.push(NativeImplicant {
                    cube,
                    covered_onset,
                });
            }
        }

        if next.is_empty() {
            break;
        }
        current = next;
    }

    primes
}

fn select_implicants(primes: &[NativeImplicant], onset_minterms: &[usize]) -> Vec<usize> {
    let mut uncovered = onset_minterms.to_vec();
    let mut selected = Vec::new();

    loop {
        let mut changed = false;
        for minterm in uncovered.clone() {
            let covering = primes
                .iter()
                .enumerate()
                .filter(|(_, prime)| prime.covered_onset.contains(&minterm))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if covering.len() == 1 && !selected.contains(&covering[0]) {
                selected.push(covering[0]);
                changed = true;
            }
        }
        if !changed {
            break;
        }
        remove_covered_minterms(&mut uncovered, primes, &selected);
    }

    if uncovered.is_empty() {
        return selected;
    }

    let candidates = primes
        .iter()
        .enumerate()
        .filter(|(index, prime)| {
            !selected.contains(index)
                && prime
                    .covered_onset
                    .iter()
                    .any(|minterm| uncovered.contains(minterm))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    let mut extra = if candidates.len() <= MAX_EXACT_SNOCOMP_PRIMES {
        exact_cover_selection(primes, &candidates, &uncovered)
    } else {
        greedy_cover_selection(primes, &candidates, &uncovered)
    };
    selected.append(&mut extra);
    remove_redundant_selected(primes, onset_minterms, &mut selected);
    selected
}

fn exact_cover_selection(
    primes: &[NativeImplicant],
    candidates: &[usize],
    uncovered: &[usize],
) -> Vec<usize> {
    let mut best: Option<Vec<usize>> = None;
    let mut current = Vec::new();
    exact_cover_branch(primes, candidates, uncovered, &mut current, &mut best);
    best.unwrap_or_else(|| greedy_cover_selection(primes, candidates, uncovered))
}

fn exact_cover_branch(
    primes: &[NativeImplicant],
    candidates: &[usize],
    uncovered: &[usize],
    current: &mut Vec<usize>,
    best: &mut Option<Vec<usize>>,
) {
    if uncovered.is_empty() {
        if best
            .as_ref()
            .is_none_or(|best_selection| selection_is_better(primes, current, best_selection))
        {
            *best = Some(current.clone());
        }
        return;
    }
    if best
        .as_ref()
        .is_some_and(|best_selection| current.len() >= best_selection.len())
    {
        return;
    }

    let target = uncovered[0];
    let mut covering = candidates
        .iter()
        .copied()
        .filter(|index| !current.contains(index) && primes[*index].covered_onset.contains(&target))
        .collect::<Vec<_>>();
    covering.sort_by_key(|index| {
        (
            std::cmp::Reverse(primes[*index].covered_onset.len()),
            primes[*index].cube.literal_count(),
        )
    });

    for index in covering {
        current.push(index);
        let remaining = uncovered
            .iter()
            .copied()
            .filter(|minterm| !primes[index].covered_onset.contains(minterm))
            .collect::<Vec<_>>();
        exact_cover_branch(primes, candidates, &remaining, current, best);
        current.pop();
    }
}

fn greedy_cover_selection(
    primes: &[NativeImplicant],
    candidates: &[usize],
    uncovered: &[usize],
) -> Vec<usize> {
    let mut selected = Vec::new();
    let mut uncovered = uncovered.to_vec();
    while !uncovered.is_empty() {
        let Some(best) = candidates
            .iter()
            .copied()
            .filter(|index| !selected.contains(index))
            .max_by_key(|index| {
                let covered = primes[*index]
                    .covered_onset
                    .iter()
                    .filter(|minterm| uncovered.contains(minterm))
                    .count();
                (
                    covered,
                    std::cmp::Reverse(primes[*index].cube.literal_count()),
                )
            })
        else {
            break;
        };
        selected.push(best);
        uncovered.retain(|minterm| !primes[best].covered_onset.contains(minterm));
    }
    selected
}

fn remove_covered_minterms(
    uncovered: &mut Vec<usize>,
    primes: &[NativeImplicant],
    selected: &[usize],
) {
    uncovered.retain(|minterm| {
        !selected
            .iter()
            .any(|index| primes[*index].covered_onset.contains(minterm))
    });
}

fn remove_redundant_selected(
    primes: &[NativeImplicant],
    onset_minterms: &[usize],
    selected: &mut Vec<usize>,
) {
    selected.sort_unstable();
    selected.dedup();
    let mut index = 0;
    while index < selected.len() {
        let candidate = selected[index];
        let covered_without = onset_minterms.iter().all(|minterm| {
            selected.iter().copied().any(|selected_index| {
                selected_index != candidate
                    && primes[selected_index].covered_onset.contains(minterm)
            })
        });
        if covered_without {
            selected.remove(index);
        } else {
            index += 1;
        }
    }
}

fn selection_is_better(
    primes: &[NativeImplicant],
    candidate: &[usize],
    incumbent: &[usize],
) -> bool {
    let candidate_literals = selection_literal_count(primes, candidate);
    let incumbent_literals = selection_literal_count(primes, incumbent);
    candidate.len() < incumbent.len()
        || (candidate.len() == incumbent.len() && candidate_literals < incumbent_literals)
}

fn selection_literal_count(primes: &[NativeImplicant], selected: &[usize]) -> usize {
    selected
        .iter()
        .map(|index| primes[*index].cube.literal_count())
        .sum()
}

fn covered_onset_minterms(cube: &Cube, input_count: usize, onset_minterms: &[usize]) -> Vec<usize> {
    onset_minterms
        .iter()
        .copied()
        .filter(|minterm| cube.evaluate(&assignment_from_index(*minterm, input_count)))
        .collect()
}

fn cube_is_allowed(cube: &Cube, input_count: usize, allowed_minterms: &[usize]) -> bool {
    let mut allowed = true;
    visit_assignments(input_count, &mut Vec::new(), &mut |assignment| {
        if cube.evaluate(assignment) && !allowed_minterms.contains(&assignment_index(assignment)) {
            allowed = false;
        }
    });
    allowed
}

fn cube_from_minterm(minterm: usize, input_count: usize) -> Cube {
    Cube::new(
        assignment_from_index(minterm, input_count)
            .into_iter()
            .map(Some)
            .collect(),
    )
}

fn assignment_from_index(minterm: usize, input_count: usize) -> Vec<bool> {
    (0..input_count)
        .map(|index| minterm & (1usize << (input_count - index - 1)) != 0)
        .collect()
}

fn assignment_index(assignment: &[bool]) -> usize {
    assignment
        .iter()
        .fold(0usize, |index, value| (index << 1) | usize::from(*value))
}

fn visit_assignments<F>(input_count: usize, partial: &mut Vec<bool>, visit: &mut F)
where
    F: FnMut(&[bool]),
{
    if partial.len() == input_count {
        visit(partial);
        return;
    }

    partial.push(false);
    visit_assignments(input_count, partial, visit);
    partial.pop();
    partial.push(true);
    visit_assignments(input_count, partial, visit);
    partial.pop();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(name: &str, phase: i32) -> Node {
        node_literal(name, phase).unwrap()
    }

    #[test]
    fn constants_and_literals_are_classified() {
        assert_eq!(
            node_function(&node_constant(0).unwrap()).unwrap(),
            NodeFunction::Zero
        );
        assert_eq!(
            node_function(&node_constant(1).unwrap()).unwrap(),
            NodeFunction::One
        );
        assert_eq!(node_function(&lit("a", 1)).unwrap(), NodeFunction::Buffer);
        assert_eq!(node_function(&lit("a", 0)).unwrap(), NodeFunction::Inverter);
    }

    #[test]
    fn and_or_not_preserve_boolean_meaning() {
        let a = lit("a", 1);
        let b = lit("b", 1);
        let and = node_and(&a, &b).unwrap();
        let or = node_or(&a, &b).unwrap();
        let not = node_not(&a).unwrap();

        assert_eq!(node_function(&and).unwrap(), NodeFunction::And);
        assert_eq!(node_function(&or).unwrap(), NodeFunction::Or);
        assert!(node_contains(&or, &and).unwrap());
        assert!(!node_contains(&and, &or).unwrap());
        assert_eq!(node_function(&not).unwrap(), NodeFunction::Inverter);
    }

    #[test]
    fn xor_and_xnor_are_complements() {
        let a = lit("a", 1);
        let b = lit("b", 1);
        let xor = node_xor(&a, &b).unwrap();
        let xnor = node_xnor(&a, &b).unwrap();
        let not_xor = node_not(&xor).unwrap();

        assert!(node_equal(&not_xor, &xnor).unwrap());
        assert_eq!(node_num_cube(&xor).unwrap(), 2);
    }

    #[test]
    fn equality_by_name_ignores_fanin_order() {
        let a = lit("a", 1);
        let b = lit("b", 1);
        let left = node_and(&a, &b).unwrap();
        let right = node_and(&b, &a).unwrap();

        assert!(node_equal_by_name(&left, &right).unwrap());
        assert_eq!(
            node_sort_for_printing(&right).unwrap().fanins,
            vec!["a", "b"]
        );
    }

    #[test]
    fn largest_cube_divisor_keeps_common_literals() {
        let a = lit("a", 1);
        let b = lit("b", 1);
        let c = lit("c", 1);
        let ab = node_and(&a, &b).unwrap();
        let ac = node_and(&a, &c).unwrap();
        let sum = node_or(&ab, &ac).unwrap();
        let divisor = node_largest_cube_divisor(&sum).unwrap();

        assert_eq!(divisor.fanins, vec!["a"]);
        assert_eq!(node_function(&divisor).unwrap(), NodeFunction::Buffer);
    }

    #[test]
    fn distance_one_merge_reduces_adjacent_cubes() {
        let a = lit("a", 1);
        let b = lit("b", 1);
        let not_b = lit("b", 0);
        let ab = node_and(&a, &b).unwrap();
        let anb = node_and(&a, &not_b).unwrap();
        let mut sum = node_or(&ab, &anb).unwrap();

        node_d1merge(&mut sum).unwrap();

        assert_eq!(sum.fanins, vec!["a"]);
        assert_eq!(node_function(&sum).unwrap(), NodeFunction::Buffer);
    }

    #[test]
    fn single_output_no_complement_minimizes_cover_without_dont_cares() {
        let function = Cover::new(
            2,
            vec![
                Cube::new(vec![Some(true), Some(true)]),
                Cube::new(vec![Some(true), Some(false)]),
            ],
        )
        .unwrap();
        let node = Node::new(function, vec!["a".to_string(), "b".to_string()]);

        let simplified =
            node_simplify(&node, None, SimplifyMode::SingleOutputNoComplement).unwrap();

        assert_eq!(simplified.fanins, vec!["a"]);
        assert_eq!(node_function(&simplified).unwrap(), NodeFunction::Buffer);
        assert!(node_equal(&node, &simplified).unwrap());
    }

    #[test]
    fn single_output_no_complement_uses_dont_cares_for_larger_implicant() {
        let function = Cover::new(2, vec![Cube::new(vec![Some(true), Some(true)])]).unwrap();
        let dont_care = Node::new(
            Cover::new(2, vec![Cube::new(vec![Some(true), Some(false)])]).unwrap(),
            vec!["a".to_string(), "b".to_string()],
        );
        let node = Node::new(function, vec!["a".to_string(), "b".to_string()]);

        let simplified = node_simplify(
            &node,
            Some(&dont_care),
            SimplifyMode::SingleOutputNoComplement,
        )
        .unwrap();

        assert_eq!(simplified.fanins, vec!["a"]);
        assert_eq!(node_function(&simplified).unwrap(), NodeFunction::Buffer);
        assert!(node_contains(&simplified, &node).unwrap());
    }

    #[test]
    fn unsupported_minimization_returns_generic_diagnostic() {
        let a = lit("a", 1);
        let error = node_simplify(&a, None, SimplifyMode::Espresso).unwrap_err();

        assert_eq!(
            error.to_string(),
            "cover minimization requires native minimization support"
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("node.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
