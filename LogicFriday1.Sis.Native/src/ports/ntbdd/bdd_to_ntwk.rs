use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NetworkNodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NetworkSignal {
    pub node: NetworkNodeId,
    pub phase: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddFunction {
    pub node: BddNodeId,
    pub complemented: bool,
}

impl BddFunction {
    pub fn new(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: false,
        }
    }

    pub fn complemented(node: BddNodeId) -> Self {
        Self {
            node,
            complemented: true,
        }
    }

    pub fn not(self) -> Self {
        Self {
            node: self.node,
            complemented: !self.complemented,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddNode {
    Zero,
    One,
    Branch {
        variable: usize,
        else_branch: BddFunction,
        then_branch: BddFunction,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddArena {
    nodes: Vec<BddNode>,
}

impl Default for BddArena {
    fn default() -> Self {
        Self::with_constants()
    }
}

impl BddArena {
    pub fn with_constants() -> Self {
        Self {
            nodes: vec![BddNode::Zero, BddNode::One],
        }
    }

    pub fn zero() -> BddFunction {
        BddFunction::new(BddNodeId(0))
    }

    pub fn one() -> BddFunction {
        BddFunction::new(BddNodeId(1))
    }

    pub fn add_branch(
        &mut self,
        variable: usize,
        else_branch: BddFunction,
        then_branch: BddFunction,
    ) -> BddFunction {
        let node = BddNodeId(self.nodes.len());
        self.nodes.push(BddNode::Branch {
            variable,
            else_branch,
            then_branch,
        });

        BddFunction::new(node)
    }

    pub fn node(&self, id: BddNodeId) -> Option<&BddNode> {
        self.nodes.get(id.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LogicExpression {
    Constant(bool),
    Literal(NetworkSignal),
    And(NetworkSignal, NetworkSignal),
    Or(NetworkSignal, NetworkSignal),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkNodeKind {
    PrimaryInput,
    Internal,
    PrimaryOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkNode {
    pub name: String,
    pub kind: NetworkNodeKind,
    pub expression: LogicExpression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNetwork {
    nodes: Vec<NetworkNode>,
}

impl BddNetwork {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn nodes(&self) -> &[NetworkNode] {
        &self.nodes
    }

    pub fn node(&self, id: NetworkNodeId) -> Option<&NetworkNode> {
        self.nodes.get(id.0)
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = (NetworkNodeId, &NetworkNode)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.kind == NetworkNodeKind::PrimaryInput)
            .map(|(index, node)| (NetworkNodeId(index), node))
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = (NetworkNodeId, &NetworkNode)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.kind == NetworkNodeKind::PrimaryOutput)
            .map(|(index, node)| (NetworkNodeId(index), node))
    }

    fn add_node(
        &mut self,
        name: impl Into<String>,
        kind: NetworkNodeKind,
        expression: LogicExpression,
    ) -> NetworkNodeId {
        let id = NetworkNodeId(self.nodes.len());
        self.nodes.push(NetworkNode {
            name: name.into(),
            kind,
            expression,
        });
        id
    }
}

impl Default for BddNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddToNetworkError {
    OutputCountMismatch {
        functions: usize,
        output_names: usize,
    },
    MissingBddNode(BddNodeId),
    MissingVariableName {
        variable: usize,
    },
    ConstantRootReachedInRecursiveBuild(BddFunction),
}

impl fmt::Display for BddToNetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputCountMismatch {
                functions,
                output_names,
            } => write!(
                formatter,
                "BDD function count {functions} does not match output name count {output_names}"
            ),
            Self::MissingBddNode(node) => write!(formatter, "BDD node {} is not present", node.0),
            Self::MissingVariableName { variable } => {
                write!(formatter, "BDD variable {variable} has no input name")
            }
            Self::ConstantRootReachedInRecursiveBuild(function) => write!(
                formatter,
                "constant BDD {:?} reached the recursive network builder",
                function
            ),
        }
    }
}

impl Error for BddToNetworkError {}

pub type BddToNetworkResult<T> = Result<T, BddToNetworkError>;

pub fn bdd_single_to_network(
    arena: &BddArena,
    function: BddFunction,
    output_name: impl Into<String>,
    variable_names: &[String],
) -> BddToNetworkResult<BddNetwork> {
    bdd_array_to_network(
        arena,
        &[Some(function)],
        &[output_name.into()],
        variable_names,
    )
}

pub fn bdd_array_to_network(
    arena: &BddArena,
    functions: &[Option<BddFunction>],
    output_names: &[String],
    variable_names: &[String],
) -> BddToNetworkResult<BddNetwork> {
    if functions.len() != output_names.len() {
        return Err(BddToNetworkError::OutputCountMismatch {
            functions: functions.len(),
            output_names: output_names.len(),
        });
    }

    let mut builder = BddNetworkBuilder {
        arena,
        variable_names,
        network: BddNetwork::new(),
        primary_inputs: HashMap::new(),
        built_nodes: HashMap::new(),
        next_internal: 0,
    };

    for (function, output_name) in functions.iter().zip(output_names) {
        let Some(function) = function else {
            continue;
        };

        builder.build_output(*function, output_name)?;
    }

    Ok(builder.network)
}

struct BddNetworkBuilder<'a> {
    arena: &'a BddArena,
    variable_names: &'a [String],
    network: BddNetwork,
    primary_inputs: HashMap<usize, NetworkNodeId>,
    built_nodes: HashMap<BddNodeId, NetworkNodeId>,
    next_internal: usize,
}

impl BddNetworkBuilder<'_> {
    fn build_output(
        &mut self,
        function: BddFunction,
        output_name: &str,
    ) -> BddToNetworkResult<NetworkNodeId> {
        let expression = match self.constant_value(function)? {
            Some(value) => LogicExpression::Constant(value),
            None => LogicExpression::Literal(self.build_rec(function)?),
        };

        Ok(self
            .network
            .add_node(output_name, NetworkNodeKind::PrimaryOutput, expression))
    }

    fn build_rec(&mut self, function: BddFunction) -> BddToNetworkResult<NetworkSignal> {
        if self.constant_value(function)?.is_some() {
            return Err(BddToNetworkError::ConstantRootReachedInRecursiveBuild(
                function,
            ));
        }

        if let Some(node) = self.built_nodes.get(&function.node) {
            return Ok(NetworkSignal {
                node: *node,
                phase: !function.complemented,
            });
        }

        let (variable, else_branch, then_branch) = match self
            .arena
            .node(function.node)
            .ok_or(BddToNetworkError::MissingBddNode(function.node))?
        {
            BddNode::Branch {
                variable,
                else_branch,
                then_branch,
            } => (*variable, *else_branch, *then_branch),
            BddNode::Zero | BddNode::One => {
                return Err(BddToNetworkError::ConstantRootReachedInRecursiveBuild(
                    function,
                ));
            }
        };

        let pi = self.primary_input(variable)?;
        let else_literal = NetworkSignal {
            node: pi,
            phase: false,
        };
        let then_literal = NetworkSignal {
            node: pi,
            phase: true,
        };
        let else_constant = self.constant_value(else_branch)?;
        let then_constant = self.constant_value(then_branch)?;

        let result = match (else_constant, then_constant) {
            (Some(else_value), Some(then_value)) => {
                debug_assert_ne!(else_value, then_value);
                if then_value {
                    then_literal
                } else {
                    else_literal
                }
            }
            (Some(constant), None) => {
                let active = self.build_rec(then_branch)?;
                if constant {
                    self.make_or(active, else_literal)?
                } else {
                    self.make_and(active, then_literal)?
                }
            }
            (None, Some(constant)) => {
                let active = self.build_rec(else_branch)?;
                if constant {
                    self.make_or(active, then_literal)?
                } else {
                    self.make_and(active, else_literal)?
                }
            }
            (None, None) => {
                let else_node = self.build_rec(else_branch)?;
                let then_node = self.build_rec(then_branch)?;
                let else_and = self.make_and(else_node, else_literal)?;
                let then_and = self.make_and(then_node, then_literal)?;
                self.make_or(else_and, then_and)?
            }
        };

        self.built_nodes.insert(function.node, result.node);

        Ok(NetworkSignal {
            node: result.node,
            phase: result.phase ^ function.complemented,
        })
    }

    fn primary_input(&mut self, variable: usize) -> BddToNetworkResult<NetworkNodeId> {
        if let Some(node) = self.primary_inputs.get(&variable) {
            return Ok(*node);
        }

        let name = self
            .variable_names
            .get(variable)
            .ok_or(BddToNetworkError::MissingVariableName { variable })?
            .clone();
        let node = self.network.add_node(
            name.clone(),
            NetworkNodeKind::PrimaryInput,
            LogicExpression::Constant(true),
        );
        self.primary_inputs.insert(variable, node);

        Ok(node)
    }

    fn constant_value(&self, function: BddFunction) -> BddToNetworkResult<Option<bool>> {
        let value = match self
            .arena
            .node(function.node)
            .ok_or(BddToNetworkError::MissingBddNode(function.node))?
        {
            BddNode::Zero => Some(false),
            BddNode::One => Some(true),
            BddNode::Branch { .. } => None,
        };

        Ok(value.map(|value| value ^ function.complemented))
    }

    fn make_and(
        &mut self,
        left: NetworkSignal,
        right: NetworkSignal,
    ) -> BddToNetworkResult<NetworkSignal> {
        let node = self.add_internal(LogicExpression::And(left, right));
        Ok(NetworkSignal { node, phase: true })
    }

    fn make_or(
        &mut self,
        left: NetworkSignal,
        right: NetworkSignal,
    ) -> BddToNetworkResult<NetworkSignal> {
        let node = self.add_internal(LogicExpression::Or(left, right));
        Ok(NetworkSignal { node, phase: true })
    }

    fn add_internal(&mut self, expression: LogicExpression) -> NetworkNodeId {
        let name = format!("n{}", self.next_internal);
        self.next_internal += 1;
        self.network
            .add_node(name, NetworkNodeKind::Internal, expression)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn evaluate(network: &BddNetwork, output: &str, inputs: &[(&str, bool)]) -> bool {
        let output = network
            .primary_outputs()
            .find(|(_, node)| node.name == output)
            .map(|(id, _)| id)
            .unwrap();

        evaluate_node(network, output, inputs)
    }

    fn evaluate_node(network: &BddNetwork, node: NetworkNodeId, inputs: &[(&str, bool)]) -> bool {
        let node = network.node(node).unwrap();
        match node.kind {
            NetworkNodeKind::PrimaryInput => inputs
                .iter()
                .find(|(name, _)| *name == node.name)
                .map(|(_, value)| *value)
                .unwrap(),
            NetworkNodeKind::Internal | NetworkNodeKind::PrimaryOutput => {
                evaluate_expression(network, &node.expression, inputs)
            }
        }
    }

    fn evaluate_expression(
        network: &BddNetwork,
        expression: &LogicExpression,
        inputs: &[(&str, bool)],
    ) -> bool {
        match expression {
            LogicExpression::Constant(value) => *value,
            LogicExpression::Literal(signal) => evaluate_signal(network, *signal, inputs),
            LogicExpression::And(left, right) => {
                evaluate_signal(network, *left, inputs) && evaluate_signal(network, *right, inputs)
            }
            LogicExpression::Or(left, right) => {
                evaluate_signal(network, *left, inputs) || evaluate_signal(network, *right, inputs)
            }
        }
    }

    fn evaluate_signal(
        network: &BddNetwork,
        signal: NetworkSignal,
        inputs: &[(&str, bool)],
    ) -> bool {
        let value = evaluate_node(network, signal.node, inputs);
        if signal.phase { value } else { !value }
    }

    #[test]
    fn converts_constant_roots_to_primary_outputs() {
        let arena = BddArena::default();
        let network = bdd_single_to_network(&arena, BddArena::one(), "y", &names(&[])).unwrap();

        assert_eq!(network.nodes().len(), 1);
        assert_eq!(
            network.primary_outputs().next().unwrap().1.expression,
            LogicExpression::Constant(true)
        );
    }

    #[test]
    fn converts_single_variable_bdd_to_literal_output() {
        let mut arena = BddArena::default();
        let root = arena.add_branch(0, BddArena::zero(), BddArena::one());

        let network = bdd_single_to_network(&arena, root, "y", &names(&["a"])).unwrap();

        assert_eq!(network.primary_inputs().count(), 1);
        let output = network.primary_outputs().next().unwrap().1;
        assert_eq!(output.name, "y");
        assert_eq!(
            output.expression,
            LogicExpression::Literal(NetworkSignal {
                node: NetworkNodeId(0),
                phase: true,
            })
        );
    }

    #[test]
    fn converts_one_constant_branch_with_legacy_simplification() {
        let mut arena = BddArena::default();
        let b = arena.add_branch(1, BddArena::zero(), BddArena::one());
        let root = arena.add_branch(0, BddArena::zero(), b);

        let network = bdd_single_to_network(&arena, root, "y", &names(&["a", "b"])).unwrap();

        assert_eq!(network.primary_inputs().count(), 2);
        assert_eq!(network.primary_outputs().count(), 1);
        assert!(
            network
                .nodes()
                .iter()
                .any(|node| matches!(node.expression, LogicExpression::And(_, _)))
        );
    }

    #[test]
    fn preserves_complemented_root_phase() {
        let mut arena = BddArena::default();
        let root = arena.add_branch(0, BddArena::zero(), BddArena::one()).not();

        let network = bdd_single_to_network(&arena, root, "y", &names(&["a"])).unwrap();

        assert!(!evaluate(&network, "y", &[("a", true)]));
        assert!(evaluate(&network, "y", &[("a", false)]));
    }

    #[test]
    fn shares_internal_network_node_for_reused_bdd_branch() {
        let mut arena = BddArena::default();
        let shared = arena.add_branch(1, BddArena::zero(), BddArena::one());
        let root = arena.add_branch(0, shared, shared);

        let network = bdd_single_to_network(&arena, root, "y", &names(&["a", "b"])).unwrap();

        assert_eq!(network.primary_inputs().count(), 2);
        assert_eq!(network.primary_outputs().count(), 1);
        assert_eq!(
            network
                .nodes()
                .iter()
                .filter(|node| node.kind == NetworkNodeKind::Internal)
                .count(),
            3
        );
    }

    #[test]
    fn skips_absent_functions_in_multi_output_conversion() {
        let arena = BddArena::default();
        let network = bdd_array_to_network(
            &arena,
            &[None, Some(BddArena::zero())],
            &names(&["skip", "z"]),
            &names(&[]),
        )
        .unwrap();

        assert_eq!(network.primary_outputs().count(), 1);
        assert_eq!(network.primary_outputs().next().unwrap().1.name, "z");
    }

    #[test]
    fn reports_output_count_mismatch() {
        let arena = BddArena::default();

        let error =
            bdd_array_to_network(&arena, &[Some(BddArena::one())], &[], &names(&[])).unwrap_err();

        assert_eq!(
            error,
            BddToNetworkError::OutputCountMismatch {
                functions: 1,
                output_names: 0,
            }
        );
    }

    #[test]
    fn reports_missing_variable_name() {
        let mut arena = BddArena::default();
        let root = arena.add_branch(2, BddArena::zero(), BddArena::one());

        let error = bdd_single_to_network(&arena, root, "y", &names(&["a"])).unwrap_err();

        assert_eq!(
            error,
            BddToNetworkError::MissingVariableName { variable: 2 }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("bdd_to_ntwk.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
    }
}
