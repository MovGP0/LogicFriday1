//! Native Rust facade for the CMU-backed SIS BDD port.
//!
//! The legacy `bddport.c` file translated the UCB BDD API onto CMU BDD
//! manager calls.  This module keeps that boundary as ordinary Rust: a manager
//! owns reduced ordered BDD nodes, public handles carry roots, and operations
//! return typed errors instead of fatal process exits.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_MANAGER_ID: AtomicUsize = AtomicUsize::new(1);

pub type BddVariableId = u32;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddEdge {
    node: usize,
    complemented: bool,
}

impl BddEdge {
    pub const fn regular(node: usize) -> Self {
        Self {
            node,
            complemented: false,
        }
    }

    pub const fn node(self) -> usize {
        self.node
    }

    pub const fn is_complemented(self) -> bool {
        self.complemented
    }

    pub const fn complement(self) -> Self {
        if self.node == 0 && !self.complemented {
            Self {
                node: 1,
                complemented: false,
            }
        } else if self.node == 1 && !self.complemented {
            Self {
                node: 0,
                complemented: false,
            }
        } else {
            Self {
                node: self.node,
                complemented: !self.complemented,
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddNode {
    Constant(bool),
    Branch {
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddFunction {
    manager_id: usize,
    root: BddEdge,
    freed: bool,
}

impl BddFunction {
    pub const fn root(&self) -> BddEdge {
        self.root
    }

    pub const fn is_freed(&self) -> bool {
        self.freed
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddExternalHooks {
    pub network: Option<String>,
    pub mdd: Option<String>,
    pub undef1: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddReorderType {
    Sift,
    Window,
    None,
}

impl Default for BddReorderType {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddMinMatchType {
    TwoSide,
    OneSide,
    OneSideDontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddStats {
    pub variables: usize,
    pub nodes_total: usize,
    pub nodes_used: usize,
    pub operations: usize,
    pub cache_entries: usize,
    pub reorder: BddReorderType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddPortError {
    DifferentManagers,
    EmptyVariableSet(&'static str),
    InvalidFunction,
    InvalidNode(usize),
    InvalidVariable(BddVariableId),
    MismatchedSubstitution,
    NotVariable,
    ZeroConstraint,
}

impl fmt::Display for BddPortError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DifferentManagers => {
                formatter.write_str("BDD operands belong to different managers")
            }
            Self::EmptyVariableSet(operation) => {
                write!(formatter, "{operation}: no variables supplied")
            }
            Self::InvalidFunction => formatter.write_str("invalid or freed BDD function handle"),
            Self::InvalidNode(node) => write!(formatter, "BDD node {node} is not present"),
            Self::InvalidVariable(variable) => {
                write!(formatter, "BDD variable {variable} is not present")
            }
            Self::MismatchedSubstitution => {
                formatter.write_str("bdd_substitute: mismatch of number of new and old variables")
            }
            Self::NotVariable => formatter.write_str("BDD handle is not a positive variable"),
            Self::ZeroConstraint => {
                formatter.write_str("bdd_cofactor: zero constraint is undefined")
            }
        }
    }
}

impl Error for BddPortError {}

#[derive(Clone, Debug)]
pub struct BddManager {
    id: usize,
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariableId, BddEdge, BddEdge), BddEdge>,
    ite_cache: HashMap<(BddEdge, BddEdge, BddEdge), BddEdge>,
    variables: Vec<BddEdge>,
    hooks: BddExternalHooks,
    reorder: BddReorderType,
    operations: usize,
}

impl BddManager {
    pub fn new(variable_count: usize) -> Self {
        let mut manager = Self {
            id: NEXT_MANAGER_ID.fetch_add(1, Ordering::Relaxed),
            nodes: vec![BddNode::Constant(false), BddNode::Constant(true)],
            unique_table: HashMap::new(),
            ite_cache: HashMap::new(),
            variables: Vec::new(),
            hooks: BddExternalHooks::default(),
            reorder: BddReorderType::None,
            operations: 0,
        };

        for _ in 0..variable_count {
            manager.create_variable();
        }

        manager
    }

    pub fn start_with_params(variable_count: usize, _params: BddManagerInit) -> Self {
        Self::new(variable_count)
    }

    pub fn set_manager_init_defaults(_params: &mut BddManagerInit) {}

    pub fn zero(&self) -> BddFunction {
        self.handle(BddEdge::regular(0))
    }

    pub fn one(&self) -> BddFunction {
        self.handle(BddEdge::regular(1))
    }

    pub fn create_variable(&mut self) -> BddFunction {
        let variable = self.variables.len() as BddVariableId;
        let edge = self.find_or_add_unchecked(variable, BddEdge::regular(1), BddEdge::regular(0));
        self.variables.push(edge);
        self.handle(edge)
    }

    pub fn create_variable_after(
        &mut self,
        after: BddVariableId,
    ) -> Result<BddFunction, BddPortError> {
        if after as usize >= self.variables.len() {
            return Err(BddPortError::InvalidVariable(after));
        }

        Ok(self.create_variable())
    }

    pub fn get_variable(&self, variable: BddVariableId) -> Result<BddFunction, BddPortError> {
        self.variables
            .get(variable as usize)
            .copied()
            .map(|edge| self.handle(edge))
            .ok_or(BddPortError::InvalidVariable(variable))
    }

    pub fn duplicate(&self, function: &BddFunction) -> Result<BddFunction, BddPortError> {
        self.validate_function(function)?;
        Ok(self.handle(function.root))
    }

    pub fn free(&self, function: &mut BddFunction) -> Result<(), BddPortError> {
        self.validate_function(function)?;
        function.freed = true;
        Ok(())
    }

    pub fn not(&mut self, function: &BddFunction) -> Result<BddFunction, BddPortError> {
        self.validate_function(function)?;
        Ok(self.handle(function.root.complement()))
    }

    pub fn and(
        &mut self,
        left: &BddFunction,
        right: &BddFunction,
        left_phase: bool,
        right_phase: bool,
    ) -> Result<BddFunction, BddPortError> {
        let left = self.phase(left, left_phase)?;
        let right = self.phase(right, right_phase)?;
        let root = self.ite_inner(left, right, BddEdge::regular(0))?;

        Ok(self.handle(root))
    }

    pub fn or(
        &mut self,
        left: &BddFunction,
        right: &BddFunction,
        left_phase: bool,
        right_phase: bool,
    ) -> Result<BddFunction, BddPortError> {
        let left = self.phase(left, left_phase)?;
        let right = self.phase(right, right_phase)?;
        let root = self.ite_inner(left, BddEdge::regular(1), right)?;

        Ok(self.handle(root))
    }

    pub fn xor(
        &mut self,
        left: &BddFunction,
        right: &BddFunction,
    ) -> Result<BddFunction, BddPortError> {
        self.validate_pair(left, right)?;
        let root = self.ite_inner(left.root, right.root.complement(), right.root)?;

        Ok(self.handle(root))
    }

    pub fn xnor(
        &mut self,
        left: &BddFunction,
        right: &BddFunction,
    ) -> Result<BddFunction, BddPortError> {
        let xor = self.xor(left, right)?;
        self.not(&xor)
    }

    pub fn ite(
        &mut self,
        condition: &BddFunction,
        then_function: &BddFunction,
        else_function: &BddFunction,
        condition_phase: bool,
        then_phase: bool,
        else_phase: bool,
    ) -> Result<BddFunction, BddPortError> {
        self.validate_function(condition)?;
        self.validate_function(then_function)?;
        self.validate_function(else_function)?;
        self.validate_same_manager(then_function)?;
        self.validate_same_manager(else_function)?;

        let condition = apply_phase(condition.root, condition_phase);
        let then_edge = apply_phase(then_function.root, then_phase);
        let else_edge = apply_phase(else_function.root, else_phase);
        let root = self.ite_inner(condition, then_edge, else_edge)?;

        Ok(self.handle(root))
    }

    pub fn cofactor(
        &mut self,
        function: &BddFunction,
        constraint: &BddFunction,
    ) -> Result<BddFunction, BddPortError> {
        self.validate_pair(function, constraint)?;

        if self.is_zero(constraint.root)? {
            return Err(BddPortError::ZeroConstraint);
        }

        let root = self.cofactor_inner(function.root, constraint.root)?;
        Ok(self.handle(root))
    }

    pub fn compose(
        &mut self,
        function: &BddFunction,
        variable: &BddFunction,
        replacement: &BddFunction,
    ) -> Result<BddFunction, BddPortError> {
        self.validate_function(function)?;
        self.validate_function(variable)?;
        self.validate_function(replacement)?;
        self.validate_same_manager(variable)?;
        self.validate_same_manager(replacement)?;

        let variable = self.variable_from_function(variable)?;
        let mut replacements = BTreeMap::new();
        replacements.insert(variable, replacement.root);
        let root = self.substitute_inner(function.root, &replacements)?;

        Ok(self.handle(root))
    }

    pub fn substitute(
        &mut self,
        function: &BddFunction,
        old_variables: &[BddFunction],
        new_variables: &[BddFunction],
    ) -> Result<BddFunction, BddPortError> {
        if old_variables.len() != new_variables.len() {
            return Err(BddPortError::MismatchedSubstitution);
        }

        self.validate_function(function)?;

        let mut replacements = BTreeMap::new();
        for (old, new) in old_variables.iter().zip(new_variables) {
            self.validate_function(old)?;
            self.validate_function(new)?;
            replacements.insert(self.variable_from_function(old)?, new.root);
        }

        let root = self.substitute_inner(function.root, &replacements)?;
        Ok(self.handle(root))
    }

    pub fn smooth(
        &mut self,
        function: &BddFunction,
        variables: &[BddFunction],
    ) -> Result<BddFunction, BddPortError> {
        let variables = self.variable_set("bdd_smooth", variables)?;
        let mut root = function.root;
        self.validate_function(function)?;

        for variable in variables {
            let with_one = self.restrict(root, variable, true)?;
            let with_zero = self.restrict(root, variable, false)?;
            root = self.ite_inner(with_one, BddEdge::regular(1), with_zero)?;
        }

        Ok(self.handle(root))
    }

    pub fn consensus(
        &mut self,
        function: &BddFunction,
        variables: &[BddFunction],
    ) -> Result<BddFunction, BddPortError> {
        let variables = self.variable_set("bdd_consensus", variables)?;
        let mut root = function.root;
        self.validate_function(function)?;

        for variable in variables {
            let with_one = self.restrict(root, variable, true)?;
            let with_zero = self.restrict(root, variable, false)?;
            root = self.ite_inner(with_one, with_zero, BddEdge::regular(0))?;
        }

        Ok(self.handle(root))
    }

    pub fn and_smooth(
        &mut self,
        left: &BddFunction,
        right: &BddFunction,
        variables: &[BddFunction],
    ) -> Result<BddFunction, BddPortError> {
        let product = self.and(left, right, true, true)?;
        self.smooth(&product, variables)
    }

    pub fn cproject(
        &mut self,
        function: &BddFunction,
        variables: &[BddFunction],
    ) -> Result<BddFunction, BddPortError> {
        self.smooth(function, variables)
    }

    pub fn minimize(
        &mut self,
        function: &BddFunction,
        care: &BddFunction,
    ) -> Result<BddFunction, BddPortError> {
        self.validate_pair(function, care)?;
        let outside_care = care.root.complement();
        let root = self.ite_inner(outside_care, BddEdge::regular(0), function.root)?;

        Ok(self.handle(root))
    }

    pub fn minimize_with_params(
        &mut self,
        function: &BddFunction,
        care: &BddFunction,
        _match_type: BddMinMatchType,
        _complement: bool,
        _no_new_variables: bool,
        _return_minimum: bool,
    ) -> Result<BddFunction, BddPortError> {
        self.minimize(function, care)
    }

    pub fn between(
        &mut self,
        lower: &BddFunction,
        upper: &BddFunction,
    ) -> Result<BddFunction, BddPortError> {
        let care = self.or(lower, upper, true, false)?;
        self.minimize(lower, &care)
    }

    pub fn then_branch(&self, function: &BddFunction) -> Result<BddFunction, BddPortError> {
        self.validate_function(function)?;
        let (then_edge, _) = self.branches(function.root)?;
        Ok(self.handle(then_edge))
    }

    pub fn else_branch(&self, function: &BddFunction) -> Result<BddFunction, BddPortError> {
        self.validate_function(function)?;
        let (_, else_edge) = self.branches(function.root)?;
        Ok(self.handle(else_edge))
    }

    pub fn top_variable(&self, function: &BddFunction) -> Result<BddFunction, BddPortError> {
        let variable = self.top_variable_id(function)?;
        self.get_variable(variable)
    }

    pub fn top_variable_id(&self, function: &BddFunction) -> Result<BddVariableId, BddPortError> {
        self.validate_function(function)?;
        self.edge_variable(function.root)
            .ok_or(BddPortError::NotVariable)
    }

    pub fn equal(&self, left: &BddFunction, right: &BddFunction) -> Result<bool, BddPortError> {
        self.validate_pair(left, right)?;
        Ok(left.root == right.root)
    }

    pub fn intersects(
        &mut self,
        left: &BddFunction,
        right: &BddFunction,
    ) -> Result<BddFunction, BddPortError> {
        self.and(left, right, true, true)
    }

    pub fn is_tautology(&self, function: &BddFunction, phase: bool) -> Result<bool, BddPortError> {
        self.validate_function(function)?;
        Ok(function.root
            == if phase {
                BddEdge::regular(1)
            } else {
                BddEdge::regular(0)
            })
    }

    pub fn leq(
        &mut self,
        left: &BddFunction,
        right: &BddFunction,
        left_phase: bool,
        right_phase: bool,
    ) -> Result<bool, BddPortError> {
        let left = self.phase(left, left_phase)?;
        let right = self.phase(right, right_phase)?;
        let counterexample = self.ite_inner(left, right.complement(), BddEdge::regular(0))?;

        self.is_zero(counterexample)
    }

    pub fn count_onset(
        &self,
        function: &BddFunction,
        variables: &[BddFunction],
    ) -> Result<f64, BddPortError> {
        self.validate_function(function)?;
        let variable_ids = variables
            .iter()
            .map(|variable| self.variable_from_function(variable))
            .collect::<Result<Vec<_>, _>>()?;

        let mut count = 0usize;
        for mask in 0..(1usize << variable_ids.len()) {
            let assignment = variable_ids
                .iter()
                .enumerate()
                .map(|(index, variable)| (*variable, (mask & (1usize << index)) != 0))
                .collect::<BTreeMap<_, _>>();

            if self.evaluate_edge(function.root, &assignment)? {
                count += 1;
            }
        }

        Ok(count as f64)
    }

    pub fn manager_id(&self, function: &BddFunction) -> Result<usize, BddPortError> {
        self.validate_function(function)?;
        Ok(function.manager_id)
    }

    pub fn get_node(&self, function: &BddFunction) -> Result<(usize, bool), BddPortError> {
        self.validate_function(function)?;
        Ok((function.root.node(), function.root.is_complemented()))
    }

    pub fn stats(&self) -> BddStats {
        BddStats {
            variables: self.variables.len(),
            nodes_total: self.nodes.len(),
            nodes_used: self.reachable_nodes(),
            operations: self.operations,
            cache_entries: self.ite_cache.len(),
            reorder: self.reorder,
        }
    }

    pub fn support(&self, function: &BddFunction) -> Result<BTreeSet<BddVariableId>, BddPortError> {
        self.validate_function(function)?;
        let mut support = BTreeSet::new();
        self.collect_support(function.root, &mut support)?;
        Ok(support)
    }

    pub fn varids(&self, variables: &[BddFunction]) -> Result<Vec<BddVariableId>, BddPortError> {
        variables
            .iter()
            .map(|variable| self.variable_from_function(variable))
            .collect()
    }

    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn size(&self, function: &BddFunction) -> Result<usize, BddPortError> {
        self.validate_function(function)?;
        let mut visited = BTreeSet::new();
        self.collect_reachable(function.root, &mut visited)?;
        Ok(visited.len())
    }

    pub fn evaluate(
        &self,
        function: &BddFunction,
        assignment: &BTreeMap<BddVariableId, bool>,
    ) -> Result<bool, BddPortError> {
        self.validate_function(function)?;
        self.evaluate_edge(function.root, assignment)
    }

    pub fn external_hooks(&self) -> &BddExternalHooks {
        &self.hooks
    }

    pub fn external_hooks_mut(&mut self) -> &mut BddExternalHooks {
        &mut self.hooks
    }

    pub fn register_daemon(&self) {}

    pub fn set_gc_mode(&self, _disabled: bool) {}

    pub fn dynamic_reordering(&mut self, reorder: BddReorderType) {
        self.reorder = reorder;
    }

    pub fn reorder(&mut self) {
        self.ite_cache.clear();
    }

    fn handle(&self, root: BddEdge) -> BddFunction {
        BddFunction {
            manager_id: self.id,
            root,
            freed: false,
        }
    }

    fn validate_pair(&self, left: &BddFunction, right: &BddFunction) -> Result<(), BddPortError> {
        self.validate_function(left)?;
        self.validate_function(right)?;
        self.validate_same_manager(right)
    }

    fn validate_same_manager(&self, function: &BddFunction) -> Result<(), BddPortError> {
        if function.manager_id != self.id {
            return Err(BddPortError::DifferentManagers);
        }

        Ok(())
    }

    fn validate_function(&self, function: &BddFunction) -> Result<(), BddPortError> {
        if function.freed {
            return Err(BddPortError::InvalidFunction);
        }

        self.validate_same_manager(function)?;
        self.validate_edge(function.root)
    }

    fn validate_edge(&self, edge: BddEdge) -> Result<(), BddPortError> {
        if edge.node >= self.nodes.len() {
            return Err(BddPortError::InvalidNode(edge.node));
        }

        Ok(())
    }

    fn phase(&self, function: &BddFunction, phase: bool) -> Result<BddEdge, BddPortError> {
        self.validate_function(function)?;
        Ok(apply_phase(function.root, phase))
    }

    fn variable_from_function(
        &self,
        function: &BddFunction,
    ) -> Result<BddVariableId, BddPortError> {
        self.validate_function(function)?;

        if function.root.is_complemented() {
            return Err(BddPortError::NotVariable);
        }

        match self.nodes.get(function.root.node) {
            Some(BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            }) if *then_edge == BddEdge::regular(1) && *else_edge == BddEdge::regular(0) => {
                Ok(*variable)
            }
            Some(_) => Err(BddPortError::NotVariable),
            None => Err(BddPortError::InvalidNode(function.root.node)),
        }
    }

    fn variable_set(
        &self,
        operation: &'static str,
        variables: &[BddFunction],
    ) -> Result<Vec<BddVariableId>, BddPortError> {
        if variables.is_empty() {
            return Err(BddPortError::EmptyVariableSet(operation));
        }

        variables
            .iter()
            .map(|variable| self.variable_from_function(variable))
            .collect()
    }

    fn edge_variable(&self, edge: BddEdge) -> Option<BddVariableId> {
        match self.nodes.get(edge.node) {
            Some(BddNode::Branch { variable, .. }) => Some(*variable),
            _ => None,
        }
    }

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), BddPortError> {
        match self.nodes.get(edge.node) {
            Some(BddNode::Branch {
                then_edge,
                else_edge,
                ..
            }) if edge.is_complemented() => Ok((then_edge.complement(), else_edge.complement())),
            Some(BddNode::Branch {
                then_edge,
                else_edge,
                ..
            }) => Ok((*then_edge, *else_edge)),
            Some(BddNode::Constant(_)) => Err(BddPortError::NotVariable),
            None => Err(BddPortError::InvalidNode(edge.node)),
        }
    }

    fn find_or_add_unchecked(
        &mut self,
        variable: BddVariableId,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> BddEdge {
        if then_edge == else_edge {
            return then_edge;
        }

        let key = (variable, then_edge, else_edge);
        if let Some(edge) = self.unique_table.get(&key) {
            return *edge;
        }

        let edge = BddEdge::regular(self.nodes.len());
        self.nodes.push(BddNode::Branch {
            variable,
            then_edge,
            else_edge,
        });
        self.unique_table.insert(key, edge);
        edge
    }

    fn ite_inner(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, BddPortError> {
        self.operations += 1;
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;

        if condition == BddEdge::regular(1) {
            return Ok(then_edge);
        }

        if condition == BddEdge::regular(0) {
            return Ok(else_edge);
        }

        if then_edge == else_edge {
            return Ok(then_edge);
        }

        if then_edge == BddEdge::regular(1) && else_edge == BddEdge::regular(0) {
            return Ok(condition);
        }

        let key = (condition, then_edge, else_edge);
        if let Some(edge) = self.ite_cache.get(&key) {
            return Ok(*edge);
        }

        let variable = self
            .min_top_variable([condition, then_edge, else_edge])
            .expect("non-terminal ITE has at least one branch operand");
        let (condition_then, condition_else) = self.cofactor_by_variable(condition, variable)?;
        let (then_then, then_else) = self.cofactor_by_variable(then_edge, variable)?;
        let (else_then, else_else) = self.cofactor_by_variable(else_edge, variable)?;
        let high = self.ite_inner(condition_then, then_then, else_then)?;
        let low = self.ite_inner(condition_else, then_else, else_else)?;
        let result = self.find_or_add_unchecked(variable, high, low);
        self.ite_cache.insert(key, result);

        Ok(result)
    }

    fn min_top_variable<const N: usize>(&self, edges: [BddEdge; N]) -> Option<BddVariableId> {
        edges
            .iter()
            .filter_map(|edge| self.edge_variable(*edge))
            .min()
    }

    fn cofactor_by_variable(
        &self,
        edge: BddEdge,
        variable: BddVariableId,
    ) -> Result<(BddEdge, BddEdge), BddPortError> {
        match self.nodes.get(edge.node) {
            Some(BddNode::Branch {
                variable: node_variable,
                then_edge,
                else_edge,
            }) if *node_variable == variable && edge.is_complemented() => {
                Ok((then_edge.complement(), else_edge.complement()))
            }
            Some(BddNode::Branch {
                variable: node_variable,
                then_edge,
                else_edge,
            }) if *node_variable == variable => Ok((*then_edge, *else_edge)),
            Some(_) => Ok((edge, edge)),
            None => Err(BddPortError::InvalidNode(edge.node)),
        }
    }

    fn restrict(
        &mut self,
        edge: BddEdge,
        variable: BddVariableId,
        value: bool,
    ) -> Result<BddEdge, BddPortError> {
        match self.nodes.get(edge.node).cloned() {
            Some(BddNode::Constant(_)) => Ok(edge),
            Some(BddNode::Branch {
                variable: node_variable,
                then_edge,
                else_edge,
            }) if node_variable == variable => {
                Ok(if value { then_edge } else { else_edge }
                    .with_complement(edge.is_complemented()))
            }
            Some(BddNode::Branch {
                variable: node_variable,
                then_edge,
                else_edge,
            }) => {
                let high = self.restrict(then_edge, variable, value)?;
                let low = self.restrict(else_edge, variable, value)?;
                Ok(self
                    .find_or_add_unchecked(node_variable, high, low)
                    .with_complement(edge.is_complemented()))
            }
            None => Err(BddPortError::InvalidNode(edge.node)),
        }
    }

    fn cofactor_inner(
        &mut self,
        function: BddEdge,
        constraint: BddEdge,
    ) -> Result<BddEdge, BddPortError> {
        if constraint == BddEdge::regular(1)
            || matches!(self.nodes[function.node], BddNode::Constant(_))
        {
            return Ok(function);
        }

        let variable = self
            .min_top_variable([function, constraint])
            .expect("non-trivial cofactor has at least one branch");
        let (function_then, function_else) = self.cofactor_by_variable(function, variable)?;
        let (constraint_then, constraint_else) = self.cofactor_by_variable(constraint, variable)?;
        let then_zero = self.is_zero(constraint_then)?;
        let else_zero = self.is_zero(constraint_else)?;

        match (then_zero, else_zero) {
            (true, true) => Err(BddPortError::ZeroConstraint),
            (false, true) => self.cofactor_inner(function_then, constraint_then),
            (true, false) => self.cofactor_inner(function_else, constraint_else),
            (false, false) => {
                let high = self.cofactor_inner(function_then, constraint_then)?;
                let low = self.cofactor_inner(function_else, constraint_else)?;
                Ok(self.find_or_add_unchecked(variable, high, low))
            }
        }
    }

    fn substitute_inner(
        &mut self,
        edge: BddEdge,
        replacements: &BTreeMap<BddVariableId, BddEdge>,
    ) -> Result<BddEdge, BddPortError> {
        match self.nodes.get(edge.node).cloned() {
            Some(BddNode::Constant(_)) => Ok(edge),
            Some(BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            }) => {
                let high = self.substitute_inner(then_edge, replacements)?;
                let low = self.substitute_inner(else_edge, replacements)?;
                let result = if let Some(replacement) = replacements.get(&variable) {
                    self.ite_inner(*replacement, high, low)?
                } else {
                    self.find_or_add_unchecked(variable, high, low)
                };

                Ok(result.with_complement(edge.is_complemented()))
            }
            None => Err(BddPortError::InvalidNode(edge.node)),
        }
    }

    fn is_zero(&self, edge: BddEdge) -> Result<bool, BddPortError> {
        self.validate_edge(edge)?;
        Ok(edge == BddEdge::regular(0))
    }

    fn evaluate_edge(
        &self,
        edge: BddEdge,
        assignment: &BTreeMap<BddVariableId, bool>,
    ) -> Result<bool, BddPortError> {
        let mut edge = edge;
        let mut complemented = false;

        loop {
            self.validate_edge(edge)?;
            complemented ^= edge.is_complemented();

            match self.nodes.get(edge.node) {
                Some(BddNode::Constant(value)) => return Ok(*value ^ complemented),
                Some(BddNode::Branch {
                    variable,
                    then_edge,
                    else_edge,
                }) => {
                    edge = if assignment.get(variable).copied().unwrap_or(false) {
                        *then_edge
                    } else {
                        *else_edge
                    };
                }
                None => return Err(BddPortError::InvalidNode(edge.node)),
            }
        }
    }

    fn collect_support(
        &self,
        edge: BddEdge,
        support: &mut BTreeSet<BddVariableId>,
    ) -> Result<(), BddPortError> {
        match self.nodes.get(edge.node) {
            Some(BddNode::Constant(_)) => Ok(()),
            Some(BddNode::Branch {
                variable,
                then_edge,
                else_edge,
            }) => {
                support.insert(*variable);
                self.collect_support(*then_edge, support)?;
                self.collect_support(*else_edge, support)
            }
            None => Err(BddPortError::InvalidNode(edge.node)),
        }
    }

    fn collect_reachable(
        &self,
        edge: BddEdge,
        visited: &mut BTreeSet<usize>,
    ) -> Result<(), BddPortError> {
        self.validate_edge(edge)?;

        if !visited.insert(edge.node) {
            return Ok(());
        }

        if let BddNode::Branch {
            then_edge,
            else_edge,
            ..
        } = self.nodes[edge.node]
        {
            self.collect_reachable(then_edge, visited)?;
            self.collect_reachable(else_edge, visited)?;
        }

        Ok(())
    }

    fn reachable_nodes(&self) -> usize {
        let mut visited = BTreeSet::new();
        for variable in &self.variables {
            let _ = self.collect_reachable(*variable, &mut visited);
        }

        visited.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddManagerInit {
    pub cache_on: bool,
    pub garbage_collection_on: bool,
    pub memory_limit: usize,
}

impl Default for BddManagerInit {
    fn default() -> Self {
        Self {
            cache_on: true,
            garbage_collection_on: true,
            memory_limit: (1usize << 30) - 2,
        }
    }
}

trait ComplementExt {
    fn with_complement(self, complemented: bool) -> Self;
}

impl ComplementExt for BddEdge {
    fn with_complement(self, complemented: bool) -> Self {
        if complemented {
            self.complement()
        } else {
            self
        }
    }
}

fn apply_phase(edge: BddEdge, phase: bool) -> BddEdge {
    if phase {
        edge
    } else {
        edge.complement()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assignment(values: &[(BddVariableId, bool)]) -> BTreeMap<BddVariableId, bool> {
        values.iter().copied().collect()
    }

    #[test]
    fn start_preallocates_variable_functions() {
        let manager = BddManager::new(3);

        assert_eq!(manager.variable_count(), 3);
        assert_eq!(
            manager.top_variable_id(&manager.get_variable(2).unwrap()),
            Ok(2)
        );
    }

    #[test]
    fn boolean_operations_obey_requested_phases() {
        let mut manager = BddManager::new(2);
        let x = manager.get_variable(0).unwrap();
        let y = manager.get_variable(1).unwrap();

        let expression = manager.and(&x, &y, true, false).unwrap();

        assert!(manager
            .evaluate(&expression, &assignment(&[(0, true), (1, false)]))
            .unwrap());
        assert!(!manager
            .evaluate(&expression, &assignment(&[(0, true), (1, true)]))
            .unwrap());
        assert!(!manager
            .evaluate(&expression, &assignment(&[(0, false), (1, false)]))
            .unwrap());
    }

    #[test]
    fn ite_xor_xnor_and_order_queries_match_bdd_semantics() {
        let mut manager = BddManager::new(2);
        let x = manager.get_variable(0).unwrap();
        let y = manager.get_variable(1).unwrap();
        let one = manager.one();
        let zero = manager.zero();

        let mux = manager.ite(&x, &one, &y, true, true, true).unwrap();
        let xor = manager.xor(&x, &y).unwrap();
        let xnor = manager.xnor(&x, &y).unwrap();

        assert!(manager
            .evaluate(&mux, &assignment(&[(0, true), (1, false)]))
            .unwrap());
        assert!(manager
            .evaluate(&mux, &assignment(&[(0, false), (1, true)]))
            .unwrap());
        assert!(manager
            .evaluate(&xor, &assignment(&[(0, true), (1, false)]))
            .unwrap());
        assert!(manager
            .evaluate(&xnor, &assignment(&[(0, true), (1, true)]))
            .unwrap());
        assert!(manager.leq(&zero, &mux, true, true).unwrap());
        assert!(manager.is_tautology(&one, true).unwrap());
    }

    #[test]
    fn cofactor_compose_and_substitute_rewrite_variables() {
        let mut manager = BddManager::new(3);
        let x = manager.get_variable(0).unwrap();
        let y = manager.get_variable(1).unwrap();
        let z = manager.get_variable(2).unwrap();
        let function = manager.or(&x, &y, true, true).unwrap();

        let cofactor = manager.cofactor(&function, &x).unwrap();
        let composed = manager.compose(&function, &x, &z).unwrap();
        let substituted = manager
            .substitute(
                &function,
                std::slice::from_ref(&y),
                std::slice::from_ref(&z),
            )
            .unwrap();

        assert!(manager
            .evaluate(&cofactor, &assignment(&[(0, true), (1, false)]))
            .unwrap());
        assert!(!manager
            .evaluate(
                &composed,
                &assignment(&[(0, false), (1, false), (2, false)])
            )
            .unwrap());
        assert!(manager
            .evaluate(&composed, &assignment(&[(0, false), (1, false), (2, true)]))
            .unwrap());
        assert!(manager
            .evaluate(
                &substituted,
                &assignment(&[(0, false), (1, false), (2, true)])
            )
            .unwrap());
    }

    #[test]
    fn quantification_support_and_onset_count_use_variable_arrays() {
        let mut manager = BddManager::new(2);
        let x = manager.get_variable(0).unwrap();
        let y = manager.get_variable(1).unwrap();
        let product = manager.and(&x, &y, true, true).unwrap();
        let smoothed = manager.smooth(&product, std::slice::from_ref(&y)).unwrap();
        let consensus = manager
            .consensus(&product, std::slice::from_ref(&y))
            .unwrap();

        assert_eq!(manager.support(&product).unwrap(), BTreeSet::from([0, 1]));
        assert!(manager
            .evaluate(&smoothed, &assignment(&[(0, true), (1, false)]))
            .unwrap());
        assert!(!manager
            .evaluate(&consensus, &assignment(&[(0, true), (1, true)]))
            .unwrap());
        assert_eq!(manager.count_onset(&product, &[x, y]).unwrap(), 1.0);
    }

    #[test]
    fn duplicate_free_and_cross_manager_validation_are_explicit() {
        let manager = BddManager::new(1);
        let other = BddManager::new(1);
        let x = manager.get_variable(0).unwrap();
        let mut duplicate = manager.duplicate(&x).unwrap();
        let foreign = other.get_variable(0).unwrap();

        assert_eq!(manager.equal(&x, &duplicate), Ok(true));
        manager.free(&mut duplicate).unwrap();
        assert_eq!(
            manager.equal(&x, &duplicate),
            Err(BddPortError::InvalidFunction)
        );
        assert_eq!(
            manager.equal(&x, &foreign),
            Err(BddPortError::DifferentManagers)
        );
    }

    #[test]
    fn stats_hooks_and_reordering_are_owned_by_manager() {
        let mut manager = BddManager::new(1);
        manager.external_hooks_mut().network = Some("network-slot".to_string());
        manager.dynamic_reordering(BddReorderType::Sift);
        manager.reorder();

        let stats = manager.stats();

        assert_eq!(
            manager.external_hooks().network.as_deref(),
            Some("network-slot")
        );
        assert_eq!(stats.variables, 1);
        assert_eq!(stats.reorder, BddReorderType::Sift);
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_metadata_tokens_are_present() {
        let source = include_str!("bddport.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
