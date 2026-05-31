//! Native Rust timing-driven cofactor delay helpers.
//!
//! The original SIS implementation computes pin delay parameters from live
//! `node_t`, `network_t`, and BDD manager state. This port keeps the portable
//! behavior as owned Rust data: parameter parsing, the TDC logarithmic delay
//! equation, BDD cut metrics, arrival-ordered select grouping, delay-to-output
//! accumulation, and per-pin delay parameter generation. Full SIS graph/Bdd
//! conversion remains a higher-level integration concern.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TdcParameters {
    pub k0: f64,
    pub k1: f64,
    pub k2: f64,
}

impl Default for TdcParameters {
    fn default() -> Self {
        Self {
            k0: 0.35,
            k1: 0.6,
            k2: 0.5,
        }
    }
}

impl TdcParameters {
    pub fn parse_line(line: &str) -> Option<Self> {
        let mut values = line
            .split_whitespace()
            .filter_map(|value| value.parse().ok());
        let k0 = values.next()?;
        let k1 = values.next()?;
        let k2 = values.next()?;

        Some(Self { k0, k1, k2 })
    }

    pub fn parse_first(content: &str) -> Self {
        content
            .lines()
            .find_map(Self::parse_line)
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TdcModelLevel {
    Basic,
    Fanout,
    ComplexityAndFanout,
}

impl TdcModelLevel {
    fn uses_fanout(self) -> bool {
        matches!(self, Self::Fanout | Self::ComplexityAndFanout)
    }

    fn uses_complexity(self) -> bool {
        self == Self::ComplexityAndFanout
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TdcInput {
    pub name: String,
    pub arrival_rise: f64,
}

impl TdcInput {
    pub fn new(name: impl Into<String>, arrival_rise: f64) -> Self {
        Self {
            name: name.into(),
            arrival_rise,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TdcPinPhase {
    Inverting,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TdcPinDelay {
    pub input_name: String,
    pub original_index: usize,
    pub sorted_pin_number: usize,
    pub block_rise: f64,
    pub block_fall: f64,
    pub drive_rise: f64,
    pub drive_fall: f64,
    pub phase: TdcPinPhase,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TdcGroup {
    pub first: usize,
    pub last: usize,
    pub member_indices: Vec<usize>,
    pub function_line_count: usize,
    pub girdle: usize,
    pub group_delay: f64,
    pub delay_to_output: f64,
    pub arrival_estimate: f64,
}

impl TdcGroup {
    pub fn group_size(&self) -> usize {
        self.member_indices.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TdcComputation {
    pub groups: Vec<TdcGroup>,
    pub pin_delays: Vec<TdcPinDelay>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TdcError {
    InvalidInputArrival { input: String, arrival: f64 },
    InvalidFanoutCount { fanout_count: usize },
    MissingBddNode { node: BddRef },
    MissingBddVariable { var_id: usize },
    InvalidBddVariable { var_id: usize, input_count: usize },
    EmptyBdd,
    MissingSisPorts { operation: &'static str },
}

impl fmt::Display for TdcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInputArrival { input, arrival } => {
                write!(f, "TDC input '{input}' has invalid arrival {arrival}")
            }
            Self::InvalidFanoutCount { fanout_count } => {
                write!(f, "TDC fanout count {fanout_count} is invalid")
            }
            Self::MissingBddNode { node } => write!(f, "TDC BDD node {:?} does not exist", node),
            Self::MissingBddVariable { var_id } => {
                write!(f, "TDC BDD variable {var_id} is not present in the diagram")
            }
            Self::InvalidBddVariable {
                var_id,
                input_count,
            } => write!(
                f,
                "TDC BDD variable {var_id} is outside the {input_count} available inputs"
            ),
            Self::EmptyBdd => write!(f, "TDC BDD contains no root"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for TdcError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BddRef(usize);

impl BddRef {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddNode {
    Terminal(bool),
    Branch {
        var_id: usize,
        then_ref: BddRef,
        else_ref: BddRef,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TdcBdd {
    nodes: Vec<BddNode>,
    root: Option<BddRef>,
}

impl TdcBdd {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
        }
    }

    pub fn add_terminal(&mut self, value: bool) -> BddRef {
        self.push(BddNode::Terminal(value))
    }

    pub fn add_branch(&mut self, var_id: usize, then_ref: BddRef, else_ref: BddRef) -> BddRef {
        self.push(BddNode::Branch {
            var_id,
            then_ref,
            else_ref,
        })
    }

    pub fn set_root(&mut self, root: BddRef) {
        self.root = Some(root);
    }

    pub fn root(&self) -> Option<BddRef> {
        self.root
    }

    pub fn node(&self, node: BddRef) -> Option<&BddNode> {
        self.nodes.get(node.index())
    }

    fn push(&mut self, node: BddNode) -> BddRef {
        let node_ref = BddRef(self.nodes.len());
        self.nodes.push(node);
        node_ref
    }
}

impl Default for TdcBdd {
    fn default() -> Self {
        Self::new()
    }
}

pub fn compute_tdc_pin_delays(
    inputs: &[TdcInput],
    bdd: &TdcBdd,
    fanout_count: usize,
    level: TdcModelLevel,
    parameters: TdcParameters,
) -> Result<TdcComputation, TdcError> {
    validate_inputs(inputs)?;
    validate_bdd_variables(bdd, inputs.len())?;

    let mut members = sorted_members(inputs);
    let mut groups = build_initial_groups(&members);
    if groups.is_empty() {
        return Ok(TdcComputation {
            groups,
            pin_delays: Vec::new(),
        });
    }

    let root = bdd.root().ok_or(TdcError::EmptyBdd)?;
    recompute_group_timing(&mut groups, &members, bdd, root, level, parameters)?;
    regroup_inputs(&mut groups, &members, bdd, root, level, parameters)?;
    recompute_group_timing(&mut groups, &members, bdd, root, level, parameters)?;
    accumulate_delay_to_output(&mut groups);

    let fanout_delay = if level.uses_fanout() {
        validate_fanout_count(fanout_count)?;
        fanout_count as f64 * 0.2
    } else {
        0.0
    };

    members.sort_by_key(|member| member.original_index);
    let mut pin_delays = Vec::with_capacity(members.len());
    for member in members {
        let group = groups
            .iter()
            .find(|group| group.member_indices.contains(&member.sorted_index))
            .expect("group membership was created from sorted members");
        let delay = group.delay_to_output + fanout_delay;
        pin_delays.push(TdcPinDelay {
            input_name: member.input.name.clone(),
            original_index: member.original_index,
            sorted_pin_number: member.pin_number,
            block_rise: delay,
            block_fall: delay,
            drive_rise: 1.0,
            drive_fall: 1.0,
            phase: TdcPinPhase::Inverting,
        });
    }

    Ok(TdcComputation { groups, pin_delays })
}

pub fn delay_equation(count: usize, complexity: usize, parameters: TdcParameters) -> f64 {
    let count_term = if count > 1 {
        parameters.k1 * log2(count as f64)
    } else {
        0.0
    };
    let complexity_term = if complexity > 1 {
        parameters.k2 * log2(complexity as f64)
    } else {
        0.0
    };

    parameters.k0 + count_term + complexity_term
}

pub fn count_function_lines(
    bdd: &TdcBdd,
    source_group: &TdcGroup,
    next_group: Option<&TdcGroup>,
) -> Result<usize, TdcError> {
    let Some(next_group) = next_group else {
        return Ok(0);
    };
    let root = bdd.root().ok_or(TdcError::EmptyBdd)?;
    let mut visited = BTreeSet::new();

    count_function_lines_rec(
        bdd,
        root,
        source_group.first,
        source_group.last,
        next_group.first,
        next_group.last,
        false,
        &mut visited,
    )
}

pub fn girdle_width(bdd: &TdcBdd, group: &TdcGroup) -> Result<usize, TdcError> {
    let root = bdd.root().ok_or(TdcError::EmptyBdd)?;
    let mut visited = BTreeSet::new();
    let mut counter = BTreeMap::new();
    girdle_rec(
        bdd,
        root,
        group.first.max(group.last),
        &mut visited,
        &mut counter,
    )?;

    let mut widest = 0;
    for var_id in group_var_range(group) {
        let count = counter.get(&var_id).copied().unwrap_or(0);
        widest = widest.max(count);
    }

    Ok(widest)
}

pub fn tdc_factor_network_unavailable() -> Result<(), TdcError> {
    Err(TdcError::MissingSisPorts {
        operation: "tdc_factor_network BDD-to-SIS-network conversion",
    })
}

fn regroup_inputs(
    groups: &mut Vec<TdcGroup>,
    members: &[SortedMember<'_>],
    bdd: &TdcBdd,
    root: BddRef,
    level: TdcModelLevel,
    parameters: TdcParameters,
) -> Result<(), TdcError> {
    if members.len() <= 1 {
        return Ok(());
    }

    let mut final_groups = vec![single_member_group(&members[0])];
    recompute_group_timing(&mut final_groups, members, bdd, root, level, parameters)?;

    for member in members.iter().skip(1) {
        let current = final_groups
            .last_mut()
            .expect("final_groups starts with one group");
        if member.input.arrival_rise > current.arrival_estimate {
            final_groups.push(single_member_group(member));
        } else {
            current.last = member.pin_number;
            current.member_indices.push(member.sorted_index);
        }
        recompute_group_timing(&mut final_groups, members, bdd, root, level, parameters)?;
    }

    *groups = final_groups;
    Ok(())
}

fn build_initial_groups(members: &[SortedMember<'_>]) -> Vec<TdcGroup> {
    let Some(first) = members.first() else {
        return Vec::new();
    };
    let mut groups = vec![single_member_group(first)];

    if members.len() > 1 {
        let rest = &members[1..];
        groups.push(TdcGroup {
            first: rest.first().unwrap().pin_number,
            last: rest.last().unwrap().pin_number,
            member_indices: rest.iter().map(|member| member.sorted_index).collect(),
            function_line_count: 0,
            girdle: 1,
            group_delay: 0.0,
            delay_to_output: 0.0,
            arrival_estimate: 0.0,
        });
    }

    groups
}

fn single_member_group(member: &SortedMember<'_>) -> TdcGroup {
    TdcGroup {
        first: member.pin_number,
        last: member.pin_number,
        member_indices: vec![member.sorted_index],
        function_line_count: 0,
        girdle: 1,
        group_delay: 0.0,
        delay_to_output: 0.0,
        arrival_estimate: 0.0,
    }
}

fn recompute_group_timing(
    groups: &mut [TdcGroup],
    members: &[SortedMember<'_>],
    bdd: &TdcBdd,
    root: BddRef,
    level: TdcModelLevel,
    parameters: TdcParameters,
) -> Result<(), TdcError> {
    for index in 0..groups.len() {
        let previous = index.checked_sub(1).map(|previous| &groups[previous]);
        let function_line_count = match previous {
            Some(previous) => count_function_lines_from_root(bdd, root, previous, &groups[index])?,
            None => 0,
        };
        let outgoing_line_count = groups
            .get(index + 1)
            .map(|next| count_function_lines_from_root(bdd, root, &groups[index], next))
            .transpose()?
            .unwrap_or(0);
        let girdle = if level.uses_complexity() && index == 0 {
            girdle_width_from_root(bdd, root, &groups[index])?
        } else {
            1
        };
        let count = groups[index].group_size() + function_line_count;
        let group_delay = delay_equation(count, girdle, parameters);
        let latest_arrival = groups[index]
            .member_indices
            .iter()
            .filter_map(|sorted_index| members.get(*sorted_index))
            .map(|member| member.input.arrival_rise)
            .fold(f64::NEG_INFINITY, f64::max);

        groups[index].function_line_count = outgoing_line_count;
        groups[index].girdle = girdle;
        groups[index].group_delay = group_delay;
        groups[index].arrival_estimate = latest_arrival + group_delay;
    }

    Ok(())
}

fn accumulate_delay_to_output(groups: &mut [TdcGroup]) {
    let mut delay = 0.0;
    for group in groups.iter_mut().rev() {
        delay += group.group_delay;
        group.delay_to_output = delay;
    }
}

fn count_function_lines_from_root(
    bdd: &TdcBdd,
    root: BddRef,
    source_group: &TdcGroup,
    next_group: &TdcGroup,
) -> Result<usize, TdcError> {
    let mut visited = BTreeSet::new();
    count_function_lines_rec(
        bdd,
        root,
        source_group.first,
        source_group.last,
        next_group.first,
        next_group.last,
        false,
        &mut visited,
    )
}

fn count_function_lines_rec(
    bdd: &TdcBdd,
    node_ref: BddRef,
    first_src: usize,
    last_src: usize,
    first_dest: usize,
    last_dest: usize,
    source_group: bool,
    visited: &mut BTreeSet<BddRef>,
) -> Result<usize, TdcError> {
    let node = bdd
        .node(node_ref)
        .ok_or(TdcError::MissingBddNode { node: node_ref })?;
    let BddNode::Branch {
        var_id,
        then_ref,
        else_ref,
    } = *node
    else {
        return Ok(0);
    };

    if visited.contains(&node_ref) {
        return Ok(0);
    }
    if contains_var(first_dest, last_dest, var_id) && source_group {
        visited.insert(node_ref);
        return Ok(1);
    }

    let in_source_group = contains_var(first_src, last_src, var_id);
    let then_count = count_function_lines_rec(
        bdd,
        then_ref,
        first_src,
        last_src,
        first_dest,
        last_dest,
        in_source_group,
        visited,
    )?;
    let else_count = count_function_lines_rec(
        bdd,
        else_ref,
        first_src,
        last_src,
        first_dest,
        last_dest,
        in_source_group,
        visited,
    )?;

    Ok(then_count + else_count)
}

fn girdle_width_from_root(bdd: &TdcBdd, root: BddRef, group: &TdcGroup) -> Result<usize, TdcError> {
    let mut visited = BTreeSet::new();
    let mut counter = BTreeMap::new();
    girdle_rec(
        bdd,
        root,
        group.first.max(group.last),
        &mut visited,
        &mut counter,
    )?;

    let mut widest = 0;
    for var_id in group_var_range(group) {
        let count = counter.get(&var_id).copied().unwrap_or(0);
        widest = widest.max(count);
    }

    Ok(widest)
}

fn girdle_rec(
    bdd: &TdcBdd,
    node_ref: BddRef,
    highest_group_var: usize,
    visited: &mut BTreeSet<BddRef>,
    counter: &mut BTreeMap<usize, usize>,
) -> Result<(), TdcError> {
    let node = bdd
        .node(node_ref)
        .ok_or(TdcError::MissingBddNode { node: node_ref })?;
    let BddNode::Branch {
        var_id,
        then_ref,
        else_ref,
    } = *node
    else {
        return Ok(());
    };

    if visited.contains(&node_ref) || var_id > highest_group_var {
        return Ok(());
    }

    visited.insert(node_ref);
    *counter.entry(var_id).or_insert(0) += 1;
    girdle_rec(bdd, then_ref, highest_group_var, visited, counter)?;
    girdle_rec(bdd, else_ref, highest_group_var, visited, counter)
}

fn sorted_members(inputs: &[TdcInput]) -> Vec<SortedMember<'_>> {
    let mut members = inputs
        .iter()
        .enumerate()
        .map(|(original_index, input)| SortedMember {
            input,
            original_index,
            sorted_index: 0,
            pin_number: 0,
        })
        .collect::<Vec<_>>();
    members.sort_by(|left, right| {
        left.input
            .arrival_rise
            .total_cmp(&right.input.arrival_rise)
            .then_with(|| left.original_index.cmp(&right.original_index))
    });
    let count = members.len();
    for (sorted_index, member) in members.iter_mut().enumerate() {
        member.sorted_index = sorted_index;
        member.pin_number = count - sorted_index - 1;
    }

    members
}

fn validate_inputs(inputs: &[TdcInput]) -> Result<(), TdcError> {
    for input in inputs {
        if !input.arrival_rise.is_finite() {
            return Err(TdcError::InvalidInputArrival {
                input: input.name.clone(),
                arrival: input.arrival_rise,
            });
        }
    }

    Ok(())
}

fn validate_fanout_count(fanout_count: usize) -> Result<(), TdcError> {
    if fanout_count > isize::MAX as usize {
        return Err(TdcError::InvalidFanoutCount { fanout_count });
    }

    Ok(())
}

fn validate_bdd_variables(bdd: &TdcBdd, input_count: usize) -> Result<(), TdcError> {
    for node in &bdd.nodes {
        if let BddNode::Branch { var_id, .. } = node {
            if *var_id >= input_count {
                return Err(TdcError::InvalidBddVariable {
                    var_id: *var_id,
                    input_count,
                });
            }
        }
    }

    Ok(())
}

fn contains_var(first: usize, last: usize, var_id: usize) -> bool {
    let low = first.min(last);
    let high = first.max(last);

    var_id >= low && var_id <= high
}

fn group_var_range(group: &TdcGroup) -> std::ops::RangeInclusive<usize> {
    group.first.min(group.last)..=group.first.max(group.last)
}

fn log2(value: f64) -> f64 {
    value.log10() / 0.30103
}

#[derive(Clone, Debug)]
struct SortedMember<'a> {
    input: &'a TdcInput,
    original_index: usize,
    sorted_index: usize,
    pin_number: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bdd() -> TdcBdd {
        let mut bdd = TdcBdd::new();
        let zero = bdd.add_terminal(false);
        let one = bdd.add_terminal(true);
        let x0_else = bdd.add_branch(0, one, zero);
        let x0_then = bdd.add_branch(0, one, zero);
        let x1 = bdd.add_branch(1, x0_then, x0_else);
        bdd.set_root(x1);
        bdd
    }

    #[test]
    fn parses_first_parameter_triple_or_defaults() {
        assert_eq!(
            TdcParameters::parse_first("# ignored\n0.1 0.2 0.3\n"),
            TdcParameters {
                k0: 0.1,
                k1: 0.2,
                k2: 0.3,
            }
        );
        assert_eq!(
            TdcParameters::parse_first("nothing useful"),
            TdcParameters::default()
        );
    }

    #[test]
    fn delay_equation_matches_tdc_log_model() {
        let delay = delay_equation(4, 2, TdcParameters::default());

        assert!((delay - 2.05).abs() < 0.00001);
        assert_eq!(delay_equation(1, 1, TdcParameters::default()), 0.35);
    }

    #[test]
    fn counts_function_lines_into_next_group_once_per_destination_node() {
        let bdd = sample_bdd();
        let source = TdcGroup {
            first: 1,
            last: 1,
            member_indices: vec![0],
            function_line_count: 0,
            girdle: 1,
            group_delay: 0.0,
            delay_to_output: 0.0,
            arrival_estimate: 0.0,
        };
        let destination = TdcGroup {
            first: 0,
            last: 0,
            member_indices: vec![1],
            function_line_count: 0,
            girdle: 1,
            group_delay: 0.0,
            delay_to_output: 0.0,
            arrival_estimate: 0.0,
        };

        assert_eq!(
            count_function_lines(&bdd, &source, Some(&destination)).unwrap(),
            2
        );
        assert_eq!(count_function_lines(&bdd, &destination, None).unwrap(), 0);
    }

    #[test]
    fn computes_girdle_width_for_grouped_bdd_levels() {
        let bdd = sample_bdd();
        let group = TdcGroup {
            first: 1,
            last: 0,
            member_indices: vec![0, 1],
            function_line_count: 0,
            girdle: 1,
            group_delay: 0.0,
            delay_to_output: 0.0,
            arrival_estimate: 0.0,
        };

        assert_eq!(girdle_width(&bdd, &group).unwrap(), 2);
    }

    #[test]
    fn computes_pin_delays_from_group_delay_and_fanout() {
        let bdd = sample_bdd();
        let inputs = vec![
            TdcInput::new("late", 5.0),
            TdcInput::new("early", 0.0),
            TdcInput::new("middle", 0.1),
        ];

        let result = compute_tdc_pin_delays(
            &inputs,
            &bdd,
            3,
            TdcModelLevel::Fanout,
            TdcParameters::default(),
        )
        .unwrap();

        assert_eq!(result.groups.len(), 2);
        assert_eq!(result.pin_delays[0].input_name, "late");
        assert_eq!(result.pin_delays[0].sorted_pin_number, 0);
        assert_eq!(result.pin_delays[0].drive_rise, 1.0);
        assert_eq!(result.pin_delays[0].phase, TdcPinPhase::Inverting);
        assert!(
            (result.pin_delays[1].block_rise - result.pin_delays[1].block_fall).abs()
                < f64::EPSILON
        );
        assert!(result.pin_delays[1].block_rise > result.pin_delays[0].block_rise);
    }

    #[test]
    fn unavailable_network_conversion_is_reported_without_legacy_abi() {
        assert!(matches!(
            tdc_factor_network_unavailable().unwrap_err(),
            TdcError::MissingSisPorts { .. }
        ));

        let source = include_str!("tdc_delay.rs");
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
