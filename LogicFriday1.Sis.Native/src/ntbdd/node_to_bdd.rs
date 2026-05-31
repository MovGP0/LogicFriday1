//! Native Rust node-to-BDD construction for the SIS ntbdd package.
//!
//! The port models the original local and recursive global conversion rules
//! without exposing legacy per-file C entry points. Nodes are represented by
//! owned Rust data, and BDDs are built with a small canonical ordered manager
//! that is sufficient for deterministic conversion and focused tests.

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_MANAGER_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddFunction {
    manager_id: u64,
    root: usize,
}

impl BddFunction {
    pub fn root(&self) -> usize {
        self.root
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct BddNode {
    variable: usize,
    low: usize,
    high: usize,
}

#[derive(Clone, Debug)]
pub struct BddManager {
    id: u64,
    nodes: Vec<BddNode>,
    unique: HashMap<BddNode, usize>,
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            id: NEXT_MANAGER_ID.fetch_add(1, Ordering::Relaxed),
            nodes: Vec::new(),
            unique: HashMap::new(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn zero(&self) -> BddFunction {
        BddFunction {
            manager_id: self.id,
            root: 0,
        }
    }

    pub fn one(&self) -> BddFunction {
        BddFunction {
            manager_id: self.id,
            root: 1,
        }
    }

    pub fn variable(&mut self, variable: usize) -> BddFunction {
        let root = self.mk(variable, 0, 1);
        BddFunction {
            manager_id: self.id,
            root,
        }
    }

    pub fn not(&mut self, function: BddFunction) -> NtBddResult<BddFunction> {
        self.ensure_function(function)?;
        let mut memo = HashMap::new();
        let root = self.not_root(function.root, &mut memo);
        Ok(BddFunction {
            manager_id: self.id,
            root,
        })
    }

    pub fn and(&mut self, left: BddFunction, right: BddFunction) -> NtBddResult<BddFunction> {
        self.apply(BddOp::And, left, right)
    }

    pub fn or(&mut self, left: BddFunction, right: BddFunction) -> NtBddResult<BddFunction> {
        self.apply(BddOp::Or, left, right)
    }

    pub fn equal(&self, left: BddFunction, right: BddFunction) -> NtBddResult<bool> {
        self.ensure_function(left)?;
        self.ensure_function(right)?;
        Ok(left.root == right.root)
    }

    pub fn evaluate(
        &self,
        function: BddFunction,
        assignment: &BTreeMap<usize, bool>,
    ) -> NtBddResult<bool> {
        self.ensure_function(function)?;
        let mut root = function.root;
        loop {
            match root {
                0 => return Ok(false),
                1 => return Ok(true),
                _ => {
                    let node = &self.nodes[root - 2];
                    root = if assignment.get(&node.variable).copied().unwrap_or(false) {
                        node.high
                    } else {
                        node.low
                    };
                }
            }
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn apply(
        &mut self,
        operation: BddOp,
        left: BddFunction,
        right: BddFunction,
    ) -> NtBddResult<BddFunction> {
        self.ensure_function(left)?;
        self.ensure_function(right)?;
        let mut memo = HashMap::new();
        let root = self.apply_root(operation, left.root, right.root, &mut memo);
        Ok(BddFunction {
            manager_id: self.id,
            root,
        })
    }

    fn ensure_function(&self, function: BddFunction) -> NtBddResult<()> {
        if function.manager_id != self.id {
            return Err(NtBddError::ManagerMismatch);
        }

        if function.root >= self.nodes.len() + 2 {
            return Err(NtBddError::UnknownBddRoot(function.root));
        }

        Ok(())
    }

    fn mk(&mut self, variable: usize, low: usize, high: usize) -> usize {
        if low == high {
            return low;
        }

        let node = BddNode {
            variable,
            low,
            high,
        };
        match self.unique.entry(node.clone()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let root = self.nodes.len() + 2;
                self.nodes.push(node);
                entry.insert(root);
                root
            }
        }
    }

    fn not_root(&mut self, root: usize, memo: &mut HashMap<usize, usize>) -> usize {
        match root {
            0 => 1,
            1 => 0,
            _ => {
                if let Some(result) = memo.get(&root) {
                    return *result;
                }

                let node = self.nodes[root - 2].clone();
                let low = self.not_root(node.low, memo);
                let high = self.not_root(node.high, memo);
                let result = self.mk(node.variable, low, high);
                memo.insert(root, result);
                result
            }
        }
    }

    fn apply_root(
        &mut self,
        operation: BddOp,
        left: usize,
        right: usize,
        memo: &mut HashMap<(BddOp, usize, usize), usize>,
    ) -> usize {
        if let Some(terminal) = operation.terminal(left, right) {
            return terminal;
        }

        let key = (operation, left, right);
        if let Some(result) = memo.get(&key) {
            return *result;
        }

        let variable = match (self.variable_of(left), self.variable_of(right)) {
            (Some(left_var), Some(right_var)) => left_var.min(right_var),
            (Some(left_var), None) => left_var,
            (None, Some(right_var)) => right_var,
            (None, None) => unreachable!("terminal case handled above"),
        };
        let (left_low, left_high) = self.cofactors(left, variable);
        let (right_low, right_high) = self.cofactors(right, variable);
        let low = self.apply_root(operation, left_low, right_low, memo);
        let high = self.apply_root(operation, left_high, right_high, memo);
        let result = self.mk(variable, low, high);
        memo.insert(key, result);
        result
    }

    fn variable_of(&self, root: usize) -> Option<usize> {
        if root < 2 {
            None
        } else {
            Some(self.nodes[root - 2].variable)
        }
    }

    fn cofactors(&self, root: usize, variable: usize) -> (usize, usize) {
        if root < 2 {
            return (root, root);
        }

        let node = &self.nodes[root - 2];
        if node.variable == variable {
            (node.low, node.high)
        } else {
            (root, root)
        }
    }
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BddOp {
    And,
    Or,
}

impl BddOp {
    fn terminal(self, left: usize, right: usize) -> Option<usize> {
        match self {
            Self::And => {
                if left == 0 || right == 0 {
                    Some(0)
                } else if left == 1 {
                    Some(right)
                } else if right == 1 || left == right {
                    Some(left)
                } else {
                    None
                }
            }
            Self::Or => {
                if left == 1 || right == 1 {
                    Some(1)
                } else if left == 0 {
                    Some(right)
                } else if right == 0 || left == right {
                    Some(left)
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicNodeKind {
    Internal,
    PrimaryInput,
    PrimaryOutput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicConstant {
    Zero,
    One,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralValue {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube {
    literals: Vec<LiteralValue>,
}

impl Cube {
    pub fn new(literals: Vec<LiteralValue>) -> Self {
        Self { literals }
    }

    pub fn literals(&self) -> &[LiteralValue] {
        &self.literals
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LogicFunction {
    Constant(LogicConstant),
    SumOfProducts(Vec<Cube>),
    Unspecified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicNode {
    pub name: String,
    pub kind: LogicNodeKind,
    pub fanins: Vec<String>,
    pub function: LogicFunction,
}

impl LogicNode {
    pub fn primary_input(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name,
            kind: LogicNodeKind::PrimaryInput,
            fanins: Vec::new(),
            function: LogicFunction::Unspecified,
        }
    }

    pub fn primary_output(name: impl Into<String>, fanin: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: LogicNodeKind::PrimaryOutput,
            fanins: vec![fanin.into()],
            function: LogicFunction::Unspecified,
        }
    }

    pub fn constant(name: impl Into<String>, constant: LogicConstant) -> Self {
        Self {
            name: name.into(),
            kind: LogicNodeKind::Internal,
            fanins: Vec::new(),
            function: LogicFunction::Constant(constant),
        }
    }

    pub fn sum_of_products(
        name: impl Into<String>,
        fanins: impl IntoIterator<Item = impl Into<String>>,
        cubes: Vec<Cube>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: LogicNodeKind::Internal,
            fanins: fanins.into_iter().map(Into::into).collect(),
            function: LogicFunction::SumOfProducts(cubes),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LogicNetwork {
    nodes: BTreeMap<String, LogicNode>,
    bdds: BTreeMap<String, BddFunction>,
}

impl LogicNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, node: LogicNode) {
        self.bdds.remove(&node.name);
        self.nodes.insert(node.name.clone(), node);
    }

    pub fn cached_bdd(&self, node: &str) -> Option<BddFunction> {
        self.bdds.get(node).copied()
    }

    pub fn clear_cached_bdd(&mut self, node: &str) {
        self.bdds.remove(node);
    }

    fn node(&self, node: &str) -> NtBddResult<&LogicNode> {
        self.nodes
            .get(node)
            .ok_or_else(|| NtBddError::UnknownNode(node.to_string()))
    }

    fn set_cached_bdd(&mut self, node: &str, function: BddFunction) {
        self.bdds.insert(node.to_string(), function);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConversionScope {
    Local,
    Global,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NtBddError {
    MissingLeaf {
        node: String,
    },
    UnknownNode(String),
    UnknownFanin {
        node: String,
        fanin: String,
    },
    MissingVariable {
        node: String,
    },
    CubeWidthMismatch {
        node: String,
        expected: usize,
        actual: usize,
    },
    ManagerMismatch,
    UnknownBddRoot(usize),
}

impl fmt::Display for NtBddError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLeaf { node } => write!(f, "node {node} is not listed in leaves"),
            Self::UnknownNode(node) => write!(f, "node {node} is not present in the network"),
            Self::UnknownFanin { node, fanin } => {
                write!(f, "node {node} references unknown fanin {fanin}")
            }
            Self::MissingVariable { node } => {
                write!(f, "variable node {node} is not listed in leaves")
            }
            Self::CubeWidthMismatch {
                node,
                expected,
                actual,
            } => write!(
                f,
                "cube for node {node} has {actual} literals, expected {expected}"
            ),
            Self::ManagerMismatch => write!(f, "BDD belongs to a different manager"),
            Self::UnknownBddRoot(root) => write!(f, "BDD root {root} is not owned by manager"),
        }
    }
}

impl Error for NtBddError {}

pub type NtBddResult<T> = Result<T, NtBddError>;

pub fn node_to_local_bdd(
    node: &LogicNode,
    manager: &mut BddManager,
    leaves: &BTreeMap<String, usize>,
) -> NtBddResult<BddFunction> {
    for fanin in &node.fanins {
        if !leaves.contains_key(fanin) {
            return Err(NtBddError::MissingLeaf {
                node: fanin.clone(),
            });
        }
    }

    construct_node_bdd(
        node,
        manager,
        leaves,
        ConversionScope::Local,
        &mut LogicNetwork::new(),
    )
}

pub fn node_to_bdd(
    network: &mut LogicNetwork,
    node: &str,
    manager: &mut BddManager,
    leaves: &BTreeMap<String, usize>,
) -> NtBddResult<BddFunction> {
    if let Some(cached) = network.cached_bdd(node) {
        if let Some(variable) = leaves.get(node) {
            let single_variable = manager.variable(*variable);
            if manager.equal(cached, single_variable)? {
                return Ok(cached);
            }

            network.set_cached_bdd(node, single_variable);
            return Ok(single_variable);
        }

        if cached.manager_id == manager.id() {
            manager.ensure_function(cached)?;
            return Ok(cached);
        }
    } else if let Some(variable) = leaves.get(node) {
        let single_variable = manager.variable(*variable);
        network.set_cached_bdd(node, single_variable);
        return Ok(single_variable);
    }

    let node_data = network.node(node)?.clone();
    let function = construct_node_bdd(
        &node_data,
        manager,
        leaves,
        ConversionScope::Global,
        network,
    )?;
    network.set_cached_bdd(node, function);
    Ok(function)
}

fn construct_node_bdd(
    node: &LogicNode,
    manager: &mut BddManager,
    leaves: &BTreeMap<String, usize>,
    scope: ConversionScope,
    network: &mut LogicNetwork,
) -> NtBddResult<BddFunction> {
    if node.fanins.is_empty() {
        return match node.function {
            LogicFunction::Constant(LogicConstant::Zero) => Ok(manager.zero()),
            LogicFunction::Constant(LogicConstant::One) => Ok(manager.one()),
            _ => leaves
                .get(&node.name)
                .map(|variable| manager.variable(*variable))
                .ok_or_else(|| NtBddError::MissingVariable {
                    node: node.name.clone(),
                }),
        };
    }

    if node.fanins.len() == 1 && node.kind == LogicNodeKind::PrimaryOutput {
        let fanin = &node.fanins[0];
        return match scope {
            ConversionScope::Local => leaves
                .get(fanin)
                .map(|variable| manager.variable(*variable))
                .ok_or_else(|| NtBddError::MissingLeaf {
                    node: fanin.clone(),
                }),
            ConversionScope::Global => node_to_bdd(network, fanin, manager, leaves),
        };
    }

    sop(node, manager, leaves, scope, network)
}

fn sop(
    node: &LogicNode,
    manager: &mut BddManager,
    leaves: &BTreeMap<String, usize>,
    scope: ConversionScope,
    network: &mut LogicNetwork,
) -> NtBddResult<BddFunction> {
    let cubes = match &node.function {
        LogicFunction::SumOfProducts(cubes) => cubes,
        LogicFunction::Constant(LogicConstant::Zero) => return Ok(manager.zero()),
        LogicFunction::Constant(LogicConstant::One) => return Ok(manager.one()),
        LogicFunction::Unspecified => {
            return Err(NtBddError::MissingVariable {
                node: node.name.clone(),
            });
        }
    };

    let mut current = manager.zero();
    for cube in cubes {
        let product = product(node, cube, manager, leaves, scope, network)?;
        current = manager.or(current, product)?;
    }

    Ok(current)
}

fn product(
    node: &LogicNode,
    cube: &Cube,
    manager: &mut BddManager,
    leaves: &BTreeMap<String, usize>,
    scope: ConversionScope,
    network: &mut LogicNetwork,
) -> NtBddResult<BddFunction> {
    if cube.literals.len() != node.fanins.len() {
        return Err(NtBddError::CubeWidthMismatch {
            node: node.name.clone(),
            expected: node.fanins.len(),
            actual: cube.literals.len(),
        });
    }

    let mut current = manager.one();
    for (literal, fanin) in cube.literals.iter().zip(&node.fanins) {
        let fanin_bdd = match literal {
            LiteralValue::DontCare => continue,
            LiteralValue::Zero | LiteralValue::One => {
                get_node_bdd(node, fanin, manager, leaves, scope, network)?
            }
        };
        let literal_bdd = match literal {
            LiteralValue::Zero => manager.not(fanin_bdd)?,
            LiteralValue::One => fanin_bdd,
            LiteralValue::DontCare => unreachable!("handled before BDD lookup"),
        };
        current = manager.and(current, literal_bdd)?;
    }

    Ok(current)
}

fn get_node_bdd(
    owner: &LogicNode,
    node: &str,
    manager: &mut BddManager,
    leaves: &BTreeMap<String, usize>,
    scope: ConversionScope,
    network: &mut LogicNetwork,
) -> NtBddResult<BddFunction> {
    match scope {
        ConversionScope::Local => leaves
            .get(node)
            .map(|variable| manager.variable(*variable))
            .ok_or_else(|| NtBddError::MissingLeaf {
                node: node.to_string(),
            }),
        ConversionScope::Global => {
            if network.node(node).is_err() {
                return Err(NtBddError::UnknownFanin {
                    node: owner.name.clone(),
                    fanin: node.to_string(),
                });
            }

            node_to_bdd(network, node, manager, leaves)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaves(names: &[&str]) -> BTreeMap<String, usize> {
        names
            .iter()
            .enumerate()
            .map(|(index, name)| ((*name).to_string(), index))
            .collect()
    }

    fn assignment(values: &[(usize, bool)]) -> BTreeMap<usize, bool> {
        values.iter().copied().collect()
    }

    fn and_node(name: &str, a: &str, b: &str) -> LogicNode {
        LogicNode::sum_of_products(
            name,
            [a, b],
            vec![Cube::new(vec![LiteralValue::One, LiteralValue::One])],
        )
    }

    #[test]
    fn local_conversion_builds_sum_of_products_from_leaf_order() {
        let node = LogicNode::sum_of_products(
            "f",
            ["a", "b"],
            vec![
                Cube::new(vec![LiteralValue::One, LiteralValue::Zero]),
                Cube::new(vec![LiteralValue::Zero, LiteralValue::One]),
            ],
        );
        let mut manager = BddManager::new();
        let bdd = node_to_local_bdd(&node, &mut manager, &leaves(&["a", "b"]))
            .expect("local BDD should be constructed");

        assert!(
            !manager
                .evaluate(bdd, &assignment(&[(0, false), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .evaluate(bdd, &assignment(&[(0, false), (1, true)]))
                .unwrap()
        );
        assert!(
            manager
                .evaluate(bdd, &assignment(&[(0, true), (1, false)]))
                .unwrap()
        );
        assert!(
            !manager
                .evaluate(bdd, &assignment(&[(0, true), (1, true)]))
                .unwrap()
        );
    }

    #[test]
    fn global_conversion_recursively_uses_fanin_functions() {
        let mut network = LogicNetwork::new();
        network.insert(LogicNode::primary_input("a"));
        network.insert(LogicNode::primary_input("b"));
        network.insert(and_node("g", "a", "b"));
        network.insert(LogicNode::sum_of_products(
            "f",
            ["g", "a"],
            vec![Cube::new(vec![LiteralValue::Zero, LiteralValue::One])],
        ));
        let mut manager = BddManager::new();
        let bdd = node_to_bdd(&mut network, "f", &mut manager, &leaves(&["a", "b"]))
            .expect("global BDD should be constructed");

        assert!(
            !manager
                .evaluate(bdd, &assignment(&[(0, false), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .evaluate(bdd, &assignment(&[(0, true), (1, false)]))
                .unwrap()
        );
        assert!(
            !manager
                .evaluate(bdd, &assignment(&[(0, true), (1, true)]))
                .unwrap()
        );
    }

    #[test]
    fn primary_output_local_conversion_returns_fanin_variable() {
        let output = LogicNode::primary_output("out", "a");
        let mut manager = BddManager::new();
        let bdd = node_to_local_bdd(&output, &mut manager, &leaves(&["a"]))
            .expect("primary output should map to fanin variable");

        assert!(!manager.evaluate(bdd, &assignment(&[(0, false)])).unwrap());
        assert!(manager.evaluate(bdd, &assignment(&[(0, true)])).unwrap());
    }

    #[test]
    fn global_conversion_caches_bdds_at_nodes() {
        let mut network = LogicNetwork::new();
        network.insert(LogicNode::primary_input("a"));
        network.insert(LogicNode::primary_input("b"));
        network.insert(and_node("f", "a", "b"));
        let mut manager = BddManager::new();
        let leaves = leaves(&["a", "b"]);

        let first = node_to_bdd(&mut network, "f", &mut manager, &leaves).unwrap();
        let second = node_to_bdd(&mut network, "f", &mut manager, &leaves).unwrap();

        assert_eq!(first, second);
        assert_eq!(network.cached_bdd("f"), Some(first));
    }

    #[test]
    fn leaf_nodes_replace_stale_cached_functions() {
        let mut network = LogicNetwork::new();
        network.insert(LogicNode::primary_input("a"));
        let mut manager = BddManager::new();
        let stale = manager.one();
        network.set_cached_bdd("a", stale);

        let bdd = node_to_bdd(&mut network, "a", &mut manager, &leaves(&["a"]))
            .expect("leaf should be converted to single variable");

        assert_ne!(bdd, stale);
        assert_eq!(network.cached_bdd("a"), Some(bdd));
        assert!(!manager.evaluate(bdd, &assignment(&[(0, false)])).unwrap());
        assert!(manager.evaluate(bdd, &assignment(&[(0, true)])).unwrap());
    }

    #[test]
    fn constants_are_handled_without_leaf_entries() {
        let mut manager = BddManager::new();
        let zero = node_to_local_bdd(
            &LogicNode::constant("zero", LogicConstant::Zero),
            &mut manager,
            &BTreeMap::new(),
        )
        .unwrap();
        let one = node_to_local_bdd(
            &LogicNode::constant("one", LogicConstant::One),
            &mut manager,
            &BTreeMap::new(),
        )
        .unwrap();

        assert!(!manager.evaluate(zero, &BTreeMap::new()).unwrap());
        assert!(manager.evaluate(one, &BTreeMap::new()).unwrap());
    }

    #[test]
    fn invalid_local_fanin_reports_missing_leaf() {
        let node = and_node("f", "a", "b");
        let mut manager = BddManager::new();

        assert_eq!(
            node_to_local_bdd(&node, &mut manager, &leaves(&["a"])),
            Err(NtBddError::MissingLeaf {
                node: "b".to_string(),
            })
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("node_to_bdd.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
    }
}
