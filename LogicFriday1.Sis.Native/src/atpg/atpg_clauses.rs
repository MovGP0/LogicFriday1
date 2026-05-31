//! Native Rust ATPG clause generation helpers.
//!
//! This module models the clause-building behavior of the SIS ATPG clause
//! generator with Rust-owned network and SAT data structures. It deliberately
//! exposes ordinary Rust APIs only; higher-level facade layers can adapt these
//! structures when integration boundaries are introduced.

use std::error::Error;
use std::fmt;

pub type SatVariable = i32;
pub type SatLiteral = i32;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SatProblem
{
    next_variable: SatVariable,
    clauses: Vec<Vec<SatLiteral>>,
}

impl SatProblem
{
    pub fn new() -> Self
    {
        Self
        {
            next_variable: 1,
            clauses: Vec::new(),
        }
    }

    pub fn new_variable(&mut self) -> SatVariable
    {
        let variable = self.next_variable;
        self.next_variable += 1;
        variable
    }

    pub fn add_clause(&mut self, literals: impl IntoIterator<Item = SatLiteral>)
    {
        self.clauses.push(literals.into_iter().collect());
    }

    pub fn clauses(&self) -> &[Vec<SatLiteral>]
    {
        &self.clauses
    }
}

pub const fn sat_neg(literal: SatLiteral) -> SatLiteral
{
    -literal
}

pub fn add_sum_clause(problem: &mut SatProblem, literals: &[SatLiteral], output: SatLiteral)
{
    for literal in literals.iter().rev()
    {
        problem.add_clause([sat_neg(*literal), output]);
    }

    match literals
    {
        [] => problem.add_clause([sat_neg(output)]),
        [literal] => problem.add_clause([*literal, sat_neg(output)]),
        _ =>
        {
            let mut clause = literals.to_vec();
            clause.push(sat_neg(output));
            problem.add_clause(clause);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction
{
    Zero,
    One,
    PrimaryInput,
    PrimaryOutput,
    Buffer,
    Inverter,
    And,
    Or,
    Complex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CubeLiteral
{
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgClauseState
{
    visited: bool,
    true_id: Option<SatVariable>,
    fault_id: Option<SatVariable>,
    active_id: Option<SatVariable>,
    current_id: Option<SatVariable>,
}

impl AtpgClauseState
{
    pub fn new() -> Self
    {
        Self
        {
            visited: false,
            true_id: None,
            fault_id: None,
            active_id: None,
            current_id: None,
        }
    }

    pub fn true_id(&self) -> Option<SatVariable>
    {
        self.true_id
    }

    pub fn fault_id(&self) -> Option<SatVariable>
    {
        self.fault_id
    }

    pub fn active_id(&self) -> Option<SatVariable>
    {
        self.active_id
    }
}

impl Default for AtpgClauseState
{
    fn default() -> Self
    {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtpgNode
{
    name: String,
    function: NodeFunction,
    fanins: Vec<NodeId>,
    fanouts: Vec<NodeId>,
    cubes: Vec<Vec<CubeLiteral>>,
    clause: AtpgClauseState,
}

impl AtpgNode
{
    pub fn new(name: impl Into<String>, function: NodeFunction) -> Self
    {
        Self
        {
            name: name.into(),
            function,
            fanins: Vec::new(),
            fanouts: Vec::new(),
            cubes: Vec::new(),
            clause: AtpgClauseState::new(),
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self
    {
        self.fanins = fanins.into_iter().collect();
        self
    }

    pub fn with_cubes(mut self, cubes: impl IntoIterator<Item = impl Into<Vec<CubeLiteral>>>) -> Self
    {
        self.cubes = cubes.into_iter().map(Into::into).collect();
        self
    }

    pub fn clause(&self) -> &AtpgClauseState
    {
        &self.clause
    }

    pub fn name(&self) -> &str
    {
        &self.name
    }

    pub fn function(&self) -> NodeFunction
    {
        self.function
    }

    pub fn fanins(&self) -> &[NodeId]
    {
        &self.fanins
    }

    pub fn fanouts(&self) -> &[NodeId]
    {
        &self.fanouts
    }

    pub fn cubes(&self) -> &[Vec<CubeLiteral>]
    {
        &self.cubes
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AtpgNetwork
{
    nodes: Vec<AtpgNode>,
}

impl AtpgNetwork
{
    pub fn new() -> Self
    {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: AtpgNode) -> NodeId
    {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn rebuild_fanouts(&mut self) -> AtpgResult<()>
    {
        for node in &mut self.nodes
        {
            node.fanouts.clear();
        }

        for index in 0..self.nodes.len()
        {
            let fanins = self.nodes[index].fanins.clone();
            for fanin in fanins
            {
                self.node_mut(fanin)?.fanouts.push(NodeId(index));
            }
        }

        Ok(())
    }

    pub fn node(&self, id: NodeId) -> AtpgResult<&AtpgNode>
    {
        self.nodes.get(id.0).ok_or(AtpgError::MissingNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> AtpgResult<&mut AtpgNode>
    {
        self.nodes.get_mut(id.0).ok_or(AtpgError::MissingNode(id))
    }

    pub fn nodes(&self) -> &[AtpgNode]
    {
        &self.nodes
    }

    pub fn find_node(&self, name: &str) -> Option<NodeId>
    {
        self.nodes
            .iter()
            .position(|node| node.name == name)
            .map(NodeId)
    }

    fn clear_visit_marks(&mut self)
    {
        for node in &mut self.nodes
        {
            node.clause.visited = false;
        }
    }

    fn reset_clause_state(&mut self)
    {
        for node in &mut self.nodes
        {
            node.clause = AtpgClauseState::new();
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StuckAtValue
{
    Zero,
    One,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fault
{
    node: NodeId,
    fanin_index: Option<usize>,
    value: StuckAtValue,
}

impl Fault
{
    pub fn output(node: NodeId, value: StuckAtValue) -> Self
    {
        Self
        {
            node,
            fanin_index: None,
            value,
        }
    }

    pub fn input(node: NodeId, fanin_index: usize, value: StuckAtValue) -> Self
    {
        Self
        {
            node,
            fanin_index: Some(fanin_index),
            value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaultClauseResult
{
    primary_inputs: Vec<NodeId>,
    cone_heads: Vec<NodeId>,
    fault_shadow: Vec<NodeId>,
}

impl FaultClauseResult
{
    pub fn primary_inputs(&self) -> &[NodeId]
    {
        &self.primary_inputs
    }

    pub fn cone_heads(&self) -> &[NodeId]
    {
        &self.cone_heads
    }

    pub fn fault_shadow(&self) -> &[NodeId]
    {
        &self.fault_shadow
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtpgError
{
    MissingNode(NodeId),
    MissingFanin
    {
        node: NodeId,
        index: usize,
    },
    MissingClauseVariable
    {
        node: NodeId,
        kind: ClauseVariableKind,
    },
    UnsupportedNodeFunction(NodeFunction),
    CubeWidthMismatch
    {
        node: NodeId,
        cube_width: usize,
        fanin_count: usize,
    },
}

impl fmt::Display for AtpgError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingNode(node) => write!(formatter, "ATPG node {} is not present", node.0),
            Self::MissingFanin
            {
                node,
                index,
            } => write!(formatter, "ATPG node {} has no fanin at index {index}", node.0),
            Self::MissingClauseVariable
            {
                node,
                kind,
            } => write!(formatter, "ATPG node {} has no {kind:?} clause variable", node.0),
            Self::UnsupportedNodeFunction(function) =>
            {
                write!(formatter, "ATPG node function {function:?} cannot be encoded")
            }
            Self::CubeWidthMismatch
            {
                node,
                cube_width,
                fanin_count,
            } => write!(
                formatter,
                "ATPG node {} cube has {cube_width} literals for {fanin_count} fanins",
                node.0
            ),
        }
    }
}

impl Error for AtpgError {}

pub type AtpgResult<T> = Result<T, AtpgError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseVariableKind
{
    Current,
    True,
    Fault,
    Active,
}

pub fn add_node_clause(
    network: &AtpgNetwork,
    problem: &mut SatProblem,
    node: NodeId,
) -> AtpgResult<()>
{
    let data = network.node(node)?;
    let output = clause_variable(data, node, ClauseVariableKind::Current)?;

    match data.function
    {
        NodeFunction::Zero => problem.add_clause([sat_neg(output)]),
        NodeFunction::One => problem.add_clause([output]),
        NodeFunction::PrimaryInput => {}
        NodeFunction::PrimaryOutput | NodeFunction::Buffer =>
        {
            let input = current_fanin_variable(network, node, 0)?;
            problem.add_clause([input, sat_neg(output)]);
            problem.add_clause([sat_neg(input), output]);
        }
        NodeFunction::Inverter =>
        {
            let input = current_fanin_variable(network, node, 0)?;
            problem.add_clause([input, output]);
            problem.add_clause([sat_neg(input), sat_neg(output)]);
        }
        NodeFunction::And | NodeFunction::Or | NodeFunction::Complex =>
        {
            add_sop_clauses(network, problem, node, output)?;
        }
    }

    Ok(())
}

pub fn generate_network_fault_clauses(
    network: &mut AtpgNetwork,
    problem: &mut SatProblem,
    fault: Fault,
) -> AtpgResult<FaultClauseResult>
{
    network.rebuild_fanouts()?;
    network.reset_clause_state();

    let mut cone_heads = Vec::new();
    let mut fault_shadow = Vec::new();
    fault_path_outputs(network, fault.node, &mut fault_shadow, &mut cone_heads)?;

    network.clear_visit_marks();
    let mut primary_inputs = Vec::new();
    for cone_head in cone_heads.iter().copied()
    {
        transitive_fanin_cone(network, problem, cone_head, &mut primary_inputs)?;
    }

    generate_fault_clauses(network, problem, &fault_shadow)?;
    generate_fault_site(network, problem, fault)?;
    generate_active_clauses(network, problem, &fault_shadow)?;

    let test_literals = cone_heads
        .iter()
        .copied()
        .filter(|node| {
            network
                .node(*node)
                .is_ok_and(|data| data.function == NodeFunction::PrimaryOutput)
        })
        .map(|node| {
            clause_variable(
                network.node(node)?,
                node,
                ClauseVariableKind::Active,
            )
        })
        .collect::<AtpgResult<Vec<_>>>()?;
    problem.add_clause(test_literals);

    Ok(FaultClauseResult
    {
        primary_inputs,
        cone_heads,
        fault_shadow,
    })
}

fn add_sop_clauses(
    network: &AtpgNetwork,
    problem: &mut SatProblem,
    node: NodeId,
    output: SatLiteral,
) -> AtpgResult<()>
{
    let data = network.node(node)?;
    let mut sum_literals = Vec::with_capacity(data.cubes.len());

    for cube in data.cubes.iter().rev()
    {
        if cube.len() != data.fanins.len()
        {
            return Err(AtpgError::CubeWidthMismatch
            {
                node,
                cube_width: cube.len(),
                fanin_count: data.fanins.len(),
            });
        }

        let mut cube_literals = Vec::new();
        for (index, value) in cube.iter().enumerate()
        {
            let fanin = network.node(data.fanins[index])?;
            let variable = clause_variable(fanin, data.fanins[index], ClauseVariableKind::Current)?;
            match value
            {
                CubeLiteral::Zero => cube_literals.push(variable),
                CubeLiteral::One => cube_literals.push(sat_neg(variable)),
                CubeLiteral::DontCare => {}
            }
        }

        if cube_literals.len() == 1
        {
            sum_literals.push(sat_neg(cube_literals[0]));
        }
        else
        {
            let literal = if data.cubes.len() == 1
            {
                output
            }
            else
            {
                problem.new_variable()
            };
            sum_literals.push(literal);
            add_sum_clause(problem, &cube_literals, sat_neg(literal));
        }
    }

    if data.cubes.len() > 1
    {
        add_sum_clause(problem, &sum_literals, output);
    }

    Ok(())
}

fn fault_path_outputs(
    network: &mut AtpgNetwork,
    node: NodeId,
    fault_shadow: &mut Vec<NodeId>,
    cone_heads: &mut Vec<NodeId>,
) -> AtpgResult<()>
{
    if network.node(node)?.clause.visited
    {
        return Ok(());
    }

    network.node_mut(node)?.clause.visited = true;
    let fanouts = network.node(node)?.fanouts.clone();
    for fanout in fanouts
    {
        fault_path_outputs(network, fanout, fault_shadow, cone_heads)?;
    }

    if network.node(node)?.fanouts.is_empty()
    {
        cone_heads.push(node);
    }
    fault_shadow.push(node);
    Ok(())
}

fn transitive_fanin_cone(
    network: &mut AtpgNetwork,
    problem: &mut SatProblem,
    node: NodeId,
    primary_inputs: &mut Vec<NodeId>,
) -> AtpgResult<()>
{
    if network.node(node)?.clause.visited
    {
        return Ok(());
    }

    network.node_mut(node)?.clause.visited = true;
    if network.node(node)?.function == NodeFunction::PrimaryInput
    {
        primary_inputs.push(node);
    }

    let true_id = problem.new_variable();
    {
        let data = network.node_mut(node)?;
        data.clause.true_id = Some(true_id);
        data.clause.current_id = Some(true_id);
    }

    let fanins = network.node(node)?.fanins.clone();
    for fanin in fanins
    {
        transitive_fanin_cone(network, problem, fanin, primary_inputs)?;
    }

    add_node_clause(network, problem, node)
}

fn generate_fault_clauses(
    network: &mut AtpgNetwork,
    problem: &mut SatProblem,
    fault_shadow: &[NodeId],
) -> AtpgResult<()>
{
    for (offset, node) in fault_shadow.iter().copied().rev().enumerate()
    {
        let fault_id = problem.new_variable();
        {
            let data = network.node_mut(node)?;
            data.clause.fault_id = Some(fault_id);
            data.clause.current_id = Some(fault_id);
        }

        if offset != 0
        {
            add_node_clause(network, problem, node)?;
        }
    }

    Ok(())
}

fn generate_fault_site(
    network: &mut AtpgNetwork,
    problem: &mut SatProblem,
    fault: Fault,
) -> AtpgResult<()>
{
    let (faulty_site, good_site) = if let Some(fanin_index) = fault.fanin_index
    {
        let fanin = *network
            .node(fault.node)?
            .fanins
            .get(fanin_index)
            .ok_or(AtpgError::MissingFanin
            {
                node: fault.node,
                index: fanin_index,
            })?;
        let faulty_site = problem.new_variable();
        let good_site = clause_variable(network.node(fanin)?, fanin, ClauseVariableKind::True)?;
        {
            let fanin_data = network.node_mut(fanin)?;
            fanin_data.clause.fault_id = Some(faulty_site);
            fanin_data.clause.current_id = Some(faulty_site);
        }
        add_node_clause(network, problem, fault.node)?;
        (faulty_site, good_site)
    }
    else
    {
        let data = network.node(fault.node)?;
        (
            clause_variable(data, fault.node, ClauseVariableKind::Fault)?,
            clause_variable(data, fault.node, ClauseVariableKind::True)?,
        )
    };

    match fault.value
    {
        StuckAtValue::Zero =>
        {
            problem.add_clause([good_site]);
            problem.add_clause([sat_neg(faulty_site)]);
        }
        StuckAtValue::One =>
        {
            problem.add_clause([sat_neg(good_site)]);
            problem.add_clause([faulty_site]);
        }
    }

    Ok(())
}

fn generate_active_clauses(
    network: &mut AtpgNetwork,
    problem: &mut SatProblem,
    fault_shadow: &[NodeId],
) -> AtpgResult<()>
{
    let last = fault_shadow.len().saturating_sub(1);
    for (index, node) in fault_shadow.iter().copied().enumerate()
    {
        let active_id = problem.new_variable();
        network.node_mut(node)?.clause.active_id = Some(active_id);

        let data = network.node(node)?;
        let true_id = clause_variable(data, node, ClauseVariableKind::True)?;
        let fault_id = clause_variable(data, node, ClauseVariableKind::Fault)?;
        problem.add_clause([sat_neg(active_id), true_id, fault_id]);
        problem.add_clause([sat_neg(active_id), sat_neg(true_id), sat_neg(fault_id)]);

        if data.function != NodeFunction::PrimaryOutput
        {
            let fanout_active = data
                .fanouts
                .iter()
                .copied()
                .map(|fanout| {
                    clause_variable(
                        network.node(fanout)?,
                        fanout,
                        ClauseVariableKind::Active,
                    )
                })
                .collect::<AtpgResult<Vec<_>>>()?;
            let mut clause = vec![sat_neg(active_id)];
            clause.extend(fanout_active);
            problem.add_clause(clause);
        }

        if index == last
        {
            problem.add_clause([active_id]);
        }
    }

    Ok(())
}

fn current_fanin_variable(
    network: &AtpgNetwork,
    node: NodeId,
    index: usize,
) -> AtpgResult<SatVariable>
{
    let fanin = *network
        .node(node)?
        .fanins
        .get(index)
        .ok_or(AtpgError::MissingFanin { node, index })?;
    clause_variable(network.node(fanin)?, fanin, ClauseVariableKind::Current)
}

fn clause_variable(
    node: &AtpgNode,
    node_id: NodeId,
    kind: ClauseVariableKind,
) -> AtpgResult<SatVariable>
{
    let variable = match kind
    {
        ClauseVariableKind::Current => node.clause.current_id,
        ClauseVariableKind::True => node.clause.true_id,
        ClauseVariableKind::Fault => node.clause.fault_id,
        ClauseVariableKind::Active => node.clause.active_id,
    };

    variable.ok_or(AtpgError::MissingClauseVariable
    {
        node: node_id,
        kind,
    })
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::collections::HashSet;

    fn is_satisfiable(clauses: &[Vec<SatLiteral>]) -> bool
    {
        let variables = clauses
            .iter()
            .flat_map(|clause| clause.iter())
            .map(|literal| literal.abs())
            .collect::<HashSet<_>>();
        let mut variables = variables.into_iter().collect::<Vec<_>>();
        variables.sort_unstable();

        for mask in 0..(1usize << variables.len())
        {
            if clauses.iter().all(|clause| {
                clause.iter().any(|literal| {
                    let index = variables.iter().position(|variable| variable == &literal.abs()).unwrap();
                    let value = mask & (1usize << index) != 0;
                    if *literal > 0
                    {
                        value
                    }
                    else
                    {
                        !value
                    }
                })
            })
            {
                return true;
            }
        }

        false
    }

    #[test]
    fn add_sum_clause_encodes_output_as_or_of_literals()
    {
        let mut problem = SatProblem::new();

        add_sum_clause(&mut problem, &[1, -2], 3);

        assert_eq!(problem.clauses(), &[vec![2, 3], vec![-1, 3], vec![1, -2, -3]]);
    }

    #[test]
    fn add_node_clause_encodes_buffer_and_inverter()
    {
        let mut network = AtpgNetwork::new();
        let input = network.add_node(AtpgNode::new("a", NodeFunction::PrimaryInput));
        let buffer = network.add_node(AtpgNode::new("b", NodeFunction::Buffer).with_fanins([input]));
        let inverter = network.add_node(AtpgNode::new("n", NodeFunction::Inverter).with_fanins([input]));
        network.node_mut(input).unwrap().clause.current_id = Some(1);
        network.node_mut(buffer).unwrap().clause.current_id = Some(2);
        network.node_mut(inverter).unwrap().clause.current_id = Some(3);
        let mut problem = SatProblem::new();

        add_node_clause(&network, &mut problem, buffer).unwrap();
        add_node_clause(&network, &mut problem, inverter).unwrap();

        assert_eq!(
            problem.clauses(),
            &[vec![1, -2], vec![-1, 2], vec![1, 3], vec![-1, -3]]
        );
    }

    #[test]
    fn sop_clauses_encode_two_input_and()
    {
        let mut network = AtpgNetwork::new();
        let a = network.add_node(AtpgNode::new("a", NodeFunction::PrimaryInput));
        let b = network.add_node(AtpgNode::new("b", NodeFunction::PrimaryInput));
        let and = network.add_node(
            AtpgNode::new("and", NodeFunction::And)
                .with_fanins([a, b])
                .with_cubes([[CubeLiteral::One, CubeLiteral::One]]),
        );
        network.node_mut(a).unwrap().clause.current_id = Some(1);
        network.node_mut(b).unwrap().clause.current_id = Some(2);
        network.node_mut(and).unwrap().clause.current_id = Some(3);
        let mut problem = SatProblem::new();

        add_node_clause(&network, &mut problem, and).unwrap();

        assert_eq!(problem.clauses(), &[vec![2, -3], vec![1, -3], vec![-1, -2, 3]]);
    }

    #[test]
    fn fault_clauses_generate_observable_path_for_output_stuck_at_zero()
    {
        let mut network = AtpgNetwork::new();
        let input = network.add_node(AtpgNode::new("a", NodeFunction::PrimaryInput));
        let output = network.add_node(
            AtpgNode::new("po", NodeFunction::PrimaryOutput).with_fanins([input]),
        );
        let mut problem = SatProblem::new();

        let result = generate_network_fault_clauses(
            &mut network,
            &mut problem,
            Fault::output(input, StuckAtValue::Zero),
        )
        .unwrap();

        assert_eq!(result.primary_inputs(), &[input]);
        assert_eq!(result.cone_heads(), &[output]);
        assert_eq!(result.fault_shadow(), &[output, input]);
        assert!(problem.clauses().contains(&vec![
            network.node(output).unwrap().clause().active_id().unwrap()
        ]));
        assert!(problem.clauses().contains(&vec![
            network.node(input).unwrap().clause().true_id().unwrap()
        ]));
        assert!(problem.clauses().contains(&vec![
            sat_neg(network.node(input).unwrap().clause().fault_id().unwrap())
        ]));
        assert!(is_satisfiable(problem.clauses()));
    }

    #[test]
    fn input_fault_site_regenerates_faulty_gate_clause()
    {
        let mut network = AtpgNetwork::new();
        let a = network.add_node(AtpgNode::new("a", NodeFunction::PrimaryInput));
        let b = network.add_node(AtpgNode::new("b", NodeFunction::PrimaryInput));
        let and = network.add_node(
            AtpgNode::new("and", NodeFunction::And)
                .with_fanins([a, b])
                .with_cubes([[CubeLiteral::One, CubeLiteral::One]]),
        );
        let _output = network.add_node(
            AtpgNode::new("po", NodeFunction::PrimaryOutput).with_fanins([and]),
        );
        let mut problem = SatProblem::new();

        let result = generate_network_fault_clauses(
            &mut network,
            &mut problem,
            Fault::input(and, 1, StuckAtValue::One),
        )
        .unwrap();

        assert_eq!(result.primary_inputs(), &[a, b]);
        assert!(problem.clauses().iter().any(|clause| clause.len() == 3));
        assert!(is_satisfiable(problem.clauses()));
    }

    #[test]
    fn source_contains_no_legacy_exports_or_embedded_tracking_metadata()
    {
        let source = include_str!("atpg_clauses.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
