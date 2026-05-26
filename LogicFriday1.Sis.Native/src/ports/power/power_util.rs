//! Native Rust model for `LogicSynthesis/sis/power/power_util.c`.
//!
//! The C file owns auxiliary power-estimation helpers: clearing the global
//! `power_info_table`, printing per-node power data, producing the power package
//! DFS order, and filtering correlated present-state line sets.  This Rust port
//! keeps those behaviors on explicit owned data structures.  Entry points that
//! would require live SIS `network_t`, `node_t`, `array_t`, `st_table`, or
//! Espresso `pset` objects fail with native integration errors.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

pub const CAPACITANCE: f64 = 0.01;
pub const POWER_SCALE: f64 = 250.0 * CAPACITANCE;
pub fn sis_power_info_table_blocked() -> Result<(), PowerUtilError> {
    missing_sis_dependencies("SIS power_info_table access")
}

pub fn sis_power_network_print_blocked() -> Result<(), PowerUtilError> {
    missing_sis_dependencies("SIS power_network_print")
}

pub fn sis_power_lines_in_set_blocked() -> Result<(), PowerUtilError> {
    missing_sis_dependencies("SIS pset power_lines_in_set")
}

fn missing_sis_dependencies(operation: &'static str) -> Result<(), PowerUtilError> {
    Err(PowerUtilError::MissingSisDependencies { operation })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerNode {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<NodeId>,
}

impl PowerNode {
    pub fn new(id: usize, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: NodeId(id),
            name: name.into(),
            kind,
            fanins: Vec::new(),
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = NodeId>) -> Self {
        self.fanins = fanins.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PowerNetwork {
    nodes: Vec<PowerNode>,
    positions: HashMap<NodeId, usize>,
}

impl PowerNetwork {
    pub fn new(nodes: Vec<PowerNode>) -> Result<Self, PowerUtilError> {
        let mut positions = HashMap::with_capacity(nodes.len());
        for (position, node) in nodes.iter().enumerate() {
            if positions.insert(node.id, position).is_some() {
                return Err(PowerUtilError::DuplicateNode { node_id: node.id });
            }
        }

        for node in &nodes {
            for fanin in &node.fanins {
                if !positions.contains_key(fanin) {
                    return Err(PowerUtilError::MissingFanin {
                        node_id: node.id,
                        fanin_id: *fanin,
                    });
                }
            }
        }

        Ok(Self { nodes, positions })
    }

    pub fn nodes(&self) -> &[PowerNode] {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> Result<&PowerNode, PowerUtilError> {
        self.positions
            .get(&id)
            .map(|position| &self.nodes[*position])
            .ok_or(PowerUtilError::MissingNode { node_id: id })
    }

    pub fn primary_inputs(&self) -> impl Iterator<Item = &PowerNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryInput)
    }

    pub fn primary_outputs(&self) -> impl Iterator<Item = &PowerNode> {
        self.nodes
            .iter()
            .filter(|node| node.kind == NodeKind::PrimaryOutput)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerInfo {
    pub cap_factor: i32,
    pub switching_prob: f64,
}

impl PowerInfo {
    pub fn node_power(self) -> f64 {
        POWER_SCALE * f64::from(self.cap_factor) * self.switching_prob
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PowerInfoTable {
    entries: HashMap<NodeId, PowerInfo>,
}

impl PowerInfoTable {
    pub fn new(entries: impl IntoIterator<Item = (NodeId, PowerInfo)>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }

    pub fn insert(&mut self, node_id: NodeId, info: PowerInfo) -> Option<PowerInfo> {
        self.entries.insert(node_id, info)
    }

    pub fn get(&self, node_id: NodeId) -> Option<PowerInfo> {
        self.entries.get(&node_id).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerReportRow {
    pub node_id: NodeId,
    pub node_name: String,
    pub cap_factor: i32,
    pub switching_prob: f64,
    pub power: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerReport {
    pub rows: Vec<PowerReportRow>,
    pub total_power: f64,
    pub primary_input_power: f64,
}

impl fmt::Display for PowerReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.rows {
            writeln!(
                f,
                "Node {:<5}\tCap. = {}\tSwitch Prob. = {:.2}\tPower = {:.1}",
                row.node_name, row.cap_factor, row.switching_prob, row.power
            )?;
        }
        write!(
            f,
            "Total Power:\t{}\nPIs Power:\t{}",
            self.total_power, self.primary_input_power
        )
    }
}

pub fn free_power_info(table: &mut Option<PowerInfoTable>) {
    if let Some(table) = table {
        table.clear();
    }
    *table = None;
}

pub fn power_report(
    network: &PowerNetwork,
    table: &mut Option<PowerInfoTable>,
) -> Result<PowerReport, PowerUtilError> {
    let Some(info_table) = table.as_ref() else {
        return Err(PowerUtilError::PowerNotEstimated);
    };

    let mut rows = Vec::new();
    let mut total_power = 0.0;
    let mut primary_input_power = 0.0;

    for node in network.nodes() {
        if node.kind == NodeKind::PrimaryOutput {
            continue;
        }

        let Some(info) = info_table.get(node.id) else {
            free_power_info(table);
            return Err(PowerUtilError::PowerNotEstimated);
        };

        let power = info.node_power();
        total_power += power;
        if node.kind == NodeKind::PrimaryInput {
            primary_input_power += power;
        }

        rows.push(PowerReportRow {
            node_id: node.id,
            node_name: node.name.clone(),
            cap_factor: info.cap_factor,
            switching_prob: info.switching_prob,
            power,
        });
    }

    Ok(PowerReport {
        rows,
        total_power,
        primary_input_power,
    })
}

pub fn power_network_dfs(network: &PowerNetwork) -> Result<Vec<NodeId>, PowerUtilError> {
    let mut result = Vec::new();
    let mut input = HashSet::new();
    let mut output = HashSet::new();

    for node in network.primary_inputs() {
        result.push(node.id);
        input.insert(node.id);
    }
    for node in network.primary_outputs() {
        output.insert(node.id);
    }

    for node_id in network_dfs(network)? {
        if !input.contains(&node_id) && !output.contains(&node_id) {
            result.push(node_id);
        }
    }

    for node in network.primary_outputs() {
        result.push(node.id);
    }

    Ok(result)
}

fn network_dfs(network: &PowerNetwork) -> Result<Vec<NodeId>, PowerUtilError> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();

    for node in network.primary_outputs() {
        dfs_visit(network, node.id, &mut visited, &mut order)?;
    }

    Ok(order)
}

fn dfs_visit(
    network: &PowerNetwork,
    node_id: NodeId,
    visited: &mut HashSet<NodeId>,
    order: &mut Vec<NodeId>,
) -> Result<(), PowerUtilError> {
    if !visited.insert(node_id) {
        return Ok(());
    }

    let node = network.node(node_id)?;
    for fanin in &node.fanins {
        dfs_visit(network, *fanin, visited, order)?;
    }
    order.push(node_id);
    Ok(())
}

pub fn power_network_print(network: &PowerNetwork) -> Result<String, PowerUtilError> {
    let mut output = String::new();
    for node_id in power_network_dfs(network)? {
        let node = network.node(node_id)?;
        output.push_str(&format_power_node(node));
        output.push('\n');
    }
    Ok(output)
}

pub fn format_power_node(node: &PowerNode) -> String {
    let kind = match node.kind {
        NodeKind::PrimaryInput => "PI",
        NodeKind::PrimaryOutput => "PO",
        NodeKind::Internal => "INTERNAL",
    };
    format!("{kind} {}", node.name)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineSet {
    lines: Vec<bool>,
}

impl LineSet {
    pub fn new(line_count: usize) -> Self {
        Self {
            lines: vec![false; line_count],
        }
    }

    pub fn from_lines(line_count: usize, lines: impl IntoIterator<Item = usize>) -> Self {
        let mut set = Self::new(line_count);
        for line in lines {
            set.insert(line);
        }
        set
    }

    pub fn full(line_count: usize) -> Self {
        Self {
            lines: vec![true; line_count],
        }
    }

    pub fn insert(&mut self, line: usize) {
        if let Some(slot) = self.lines.get_mut(line) {
            *slot = true;
        }
    }

    pub fn contains(&self, line: usize) -> bool {
        self.lines.get(line).copied().unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        !self.lines.iter().any(|line| *line)
    }

    pub fn included_indices(&self) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, included)| included.then_some(index))
            .collect()
    }
}

pub fn power_lines_in_set(
    set: &LineSet,
    set_size: usize,
    power_set_size: usize,
) -> Result<LineSet, PowerUtilError> {
    let required_lines = power_set_size
        .checked_mul(2)
        .ok_or(PowerUtilError::SetSizeOverflow)?;
    if set.len() < required_lines {
        return Err(PowerUtilError::InsufficientLineSet {
            expected: required_lines,
            actual: set.len(),
        });
    }
    if power_set_size >= usize::BITS as usize {
        return Err(PowerUtilError::SetSizeOverflow);
    }

    let mut included_sets = LineSet::full(set_size);
    for set_index in 0..set_size {
        for bit_index in 0..power_set_size {
            let required_line = if (set_index & (1usize << bit_index)) != 0 {
                2 * bit_index + 1
            } else {
                2 * bit_index
            };

            if !set.contains(required_line) {
                included_sets.lines[set_index] = false;
                break;
            }
        }
    }

    Ok(included_sets)
}

#[derive(Clone, Debug, PartialEq)]
pub enum PowerUtilError {
    MissingSisDependencies { operation: &'static str },
    PowerNotEstimated,
    DuplicateNode { node_id: NodeId },
    MissingNode { node_id: NodeId },
    MissingFanin { node_id: NodeId, fanin_id: NodeId },
    InsufficientLineSet { expected: usize, actual: usize },
    SetSizeOverflow,
}

impl fmt::Display for PowerUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => write!(
                f,
                "operation {:?} requires native SIS prerequisite ports",
                operation
            ),
            Self::PowerNotEstimated => write!(f, "Power for this network not estimated yet!"),
            Self::DuplicateNode { node_id } => {
                write!(f, "duplicate power network node {}", node_id.0)
            }
            Self::MissingNode { node_id } => write!(f, "missing power network node {}", node_id.0),
            Self::MissingFanin { node_id, fanin_id } => write!(
                f,
                "power network node {} references missing fanin {}",
                node_id.0, fanin_id.0
            ),
            Self::InsufficientLineSet { expected, actual } => write!(
                f,
                "line set has {actual} lines but power_set_size requires {expected}"
            ),
            Self::SetSizeOverflow => write!(f, "power set size is too large"),
        }
    }
}

impl Error for PowerUtilError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> PowerNetwork {
        PowerNetwork::new(vec![
            PowerNode::new(1, "a", NodeKind::PrimaryInput),
            PowerNode::new(2, "b", NodeKind::PrimaryInput),
            PowerNode::new(3, "n1", NodeKind::Internal).with_fanins([NodeId(1), NodeId(2)]),
            PowerNode::new(4, "n2", NodeKind::Internal).with_fanins([NodeId(3)]),
            PowerNode::new(5, "out", NodeKind::PrimaryOutput).with_fanins([NodeId(4)]),
        ])
        .unwrap()
    }

    #[test]
    fn free_power_info_clears_and_drops_the_table_like_c_global_cleanup() {
        let mut table = Some(PowerInfoTable::new([(
            NodeId(1),
            PowerInfo {
                cap_factor: 4,
                switching_prob: 0.5,
            },
        )]));

        free_power_info(&mut table);

        assert_eq!(table, None);
    }

    #[test]
    fn power_report_matches_c_scaling_and_skips_primary_outputs() {
        let network = sample_network();
        let mut table = Some(PowerInfoTable::new([
            (
                NodeId(1),
                PowerInfo {
                    cap_factor: 2,
                    switching_prob: 0.25,
                },
            ),
            (
                NodeId(2),
                PowerInfo {
                    cap_factor: 3,
                    switching_prob: 0.50,
                },
            ),
            (
                NodeId(3),
                PowerInfo {
                    cap_factor: 4,
                    switching_prob: 0.75,
                },
            ),
            (
                NodeId(4),
                PowerInfo {
                    cap_factor: 5,
                    switching_prob: 1.00,
                },
            ),
        ]));

        let report = power_report(&network, &mut table).unwrap();

        assert_eq!(
            report
                .rows
                .iter()
                .map(|row| row.node_id)
                .collect::<Vec<_>>(),
            vec![NodeId(1), NodeId(2), NodeId(3), NodeId(4)]
        );
        assert_eq!(report.rows[0].power, 1.25);
        assert_eq!(report.primary_input_power, 5.0);
        assert_eq!(report.total_power, 25.0);
        assert!(format!("{report}").contains("Node a    \tCap. = 2"));
    }

    #[test]
    fn missing_report_entry_frees_table_and_reports_not_estimated() {
        let network = sample_network();
        let mut table = Some(PowerInfoTable::new([(
            NodeId(1),
            PowerInfo {
                cap_factor: 2,
                switching_prob: 0.25,
            },
        )]));

        let error = power_report(&network, &mut table).unwrap_err();

        assert_eq!(error, PowerUtilError::PowerNotEstimated);
        assert_eq!(table, None);
    }

    #[test]
    fn power_network_dfs_places_inputs_first_intermediates_next_outputs_last() {
        let network = sample_network();

        assert_eq!(
            power_network_dfs(&network).unwrap(),
            vec![NodeId(1), NodeId(2), NodeId(3), NodeId(4), NodeId(5)]
        );
        assert_eq!(
            power_network_print(&network).unwrap(),
            "PI a\nPI b\nINTERNAL n1\nINTERNAL n2\nPO out\n"
        );
    }

    #[test]
    fn power_lines_in_set_matches_legacy_even_odd_line_encoding() {
        let line_set = LineSet::from_lines(6, [0, 2, 3, 5]);

        let included = power_lines_in_set(&line_set, 8, 3).unwrap();

        assert_eq!(included.included_indices(), vec![4, 6]);
    }
}
