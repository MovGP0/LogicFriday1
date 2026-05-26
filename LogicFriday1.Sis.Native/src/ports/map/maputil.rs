//! Native Rust mapper utilities for `sis/map/maputil.c`.
//!
//! The SIS utility file mixes deterministic reporting helpers with direct
//! `network_t` mutation and cleanup. This native port keeps the owned-data
//! behavior available to the current mapper facade: option summaries,
//! diagnostic summaries, level and gate accounting over `VirtualMappedNetwork`,
//! and explicit dependency errors for the remaining full-network mutation path.
//! It intentionally exposes no legacy C ABI entry points.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use super::com_map::{FanoutHandling, LoadLimitMode, MapCostMode, MapOptions, TreeCovering};
use super::map_interface::{MapDiagnostic, MapInterfaceResult, MapInterfaceStrategy};
use super::virtual_net::{
    GateKind, NodeId, NodeKind, SourceRef, VirtualMappedNetwork, VirtualNetworkError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapperOptionSummary {
    pub cost_mode: &'static str,
    pub tree_covering: &'static str,
    pub fanout_handling: &'static str,
    pub load_limit_mode: &'static str,
    pub load_penalty: i32,
    pub verbosity: i32,
    pub raw_mode: bool,
    pub print_statistics: bool,
    pub ignore_delay_constraints: bool,
    pub disable_inverter_at_branch: bool,
    pub recover_area_with_buffer_resize: bool,
    pub recover_area_with_gate_resize: bool,
    pub fanout_optimization: bool,
    pub suppress_warnings: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapperDiagnosticSummary {
    pub strategy: &'static str,
    pub replacement_path: bool,
    pub false_output_rows_ignored: usize,
    pub library_primitive_selections: usize,
    pub library_gate_use: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapperLevelEntry {
    pub level: usize,
    pub nodes: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapperGateAccounting {
    pub node_count: usize,
    pub primary_input_count: usize,
    pub primary_output_count: usize,
    pub active_internal_count: usize,
    pub inactive_internal_count: usize,
    pub mapped_gate_count: usize,
    pub wire_count: usize,
    pub constant_count: usize,
    pub edge_count: usize,
    pub maximum_fanin: usize,
    pub maximum_fanout: usize,
    pub level_count: usize,
    pub gates_by_kind: BTreeMap<String, usize>,
    pub levels: Vec<MapperLevelEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapperStateSummary {
    pub options: MapperOptionSummary,
    pub diagnostics: MapperDiagnosticSummary,
    pub accounting: MapperGateAccounting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapUtilError {
    VirtualNetwork(VirtualNetworkError),
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for MapUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => write!(f, "{operation} requires unavailable native SIS integration"),
        }
    }
}

impl Error for MapUtilError {}

impl From<VirtualNetworkError> for MapUtilError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

pub fn full_sis_network_mutation_unavailable() -> Result<(), MapUtilError> {
    Err(MapUtilError::MissingSisPorts {
        operation: "maputil full SIS network mutation",
    })
}

pub fn summarize_options(options: &MapOptions) -> MapperOptionSummary {
    MapperOptionSummary {
        cost_mode: cost_mode_name(options.cost_mode),
        tree_covering: tree_covering_name(options.tree_covering),
        fanout_handling: fanout_handling_name(options.fanout_handling),
        load_limit_mode: load_limit_mode_name(options.load_limit_mode),
        load_penalty: options.load_penalty,
        verbosity: options.verbosity,
        raw_mode: options.raw_mode,
        print_statistics: options.print_statistics,
        ignore_delay_constraints: options.ignore_delay_constraints,
        disable_inverter_at_branch: options.disable_inverter_at_branch,
        recover_area_with_buffer_resize: options.recover_area_with_buffer_resize,
        recover_area_with_gate_resize: options.recover_area_with_gate_resize,
        fanout_optimization: options.fanout_optimization,
        suppress_warnings: options.suppress_warnings,
    }
}

pub fn summarize_diagnostics(result: &MapInterfaceResult) -> MapperDiagnosticSummary {
    let mut summary = MapperDiagnosticSummary {
        strategy: strategy_name(result.strategy),
        replacement_path: false,
        false_output_rows_ignored: 0,
        library_primitive_selections: 0,
        library_gate_use: BTreeMap::new(),
    };

    for diagnostic in &result.diagnostics {
        match diagnostic {
            MapDiagnostic::SopPrimitiveReplacementPath => {
                summary.replacement_path = true;
            }
            MapDiagnostic::FalseOutputRowsIgnored { count, .. } => {
                summary.false_output_rows_ignored += count;
            }
            MapDiagnostic::LibraryPrimitiveSelected { gate, .. } => {
                summary.library_primitive_selections += 1;
                *summary.library_gate_use.entry(gate.clone()).or_default() += 1;
            }
        }
    }

    summary
}

pub fn account_virtual_network(
    network: &VirtualMappedNetwork,
) -> Result<MapperGateAccounting, MapUtilError> {
    let levels = network
        .levels()?
        .into_iter()
        .enumerate()
        .map(|(level, nodes)| MapperLevelEntry { level, nodes })
        .collect::<Vec<_>>();
    let level_count = levels.len().saturating_sub(1);

    let mut accounting = MapperGateAccounting {
        node_count: network.nodes().len(),
        primary_input_count: network.inputs().len(),
        primary_output_count: network.outputs().len(),
        active_internal_count: 0,
        inactive_internal_count: 0,
        mapped_gate_count: 0,
        wire_count: 0,
        constant_count: 0,
        edge_count: 0,
        maximum_fanin: 0,
        maximum_fanout: 0,
        level_count,
        gates_by_kind: BTreeMap::new(),
        levels,
    };

    for node in network.nodes() {
        accounting.edge_count += node
            .save_binding
            .iter()
            .filter(|source| matches!(source, SourceRef::Node(_)))
            .count();
        accounting.maximum_fanin = accounting.maximum_fanin.max(node.save_binding.len());
        accounting.maximum_fanout = accounting.maximum_fanout.max(node.gate_links().count());

        if node.kind == NodeKind::Internal {
            if let Some(gate) = &node.gate {
                accounting.active_internal_count += 1;
                accounting.mapped_gate_count += (!gate.is_wire()) as usize;

                if gate.is_wire() {
                    accounting.wire_count += 1;
                }

                if matches!(gate, GateKind::One | GateKind::Zero) {
                    accounting.constant_count += 1;
                }

                *accounting
                    .gates_by_kind
                    .entry(gate_accounting_name(gate).to_owned())
                    .or_default() += 1;
            } else {
                accounting.inactive_internal_count += 1;
            }
        }
    }

    Ok(accounting)
}

pub fn summarize_mapping_result(
    result: &MapInterfaceResult,
    options: &MapOptions,
) -> Result<MapperStateSummary, MapUtilError> {
    Ok(MapperStateSummary {
        options: summarize_options(options),
        diagnostics: summarize_diagnostics(result),
        accounting: account_virtual_network(&result.network)?,
    })
}

fn cost_mode_name(mode: MapCostMode) -> &'static str {
    match mode {
        MapCostMode::Area => "area",
        MapCostMode::Delay => "delay",
        MapCostMode::CriticalPathDelay => "critical-path-delay",
        MapCostMode::AreaDelayTradeoff(_) => "area-delay-tradeoff",
    }
}

fn tree_covering_name(value: TreeCovering) -> &'static str {
    match value {
        TreeCovering::Simple => "simple",
        TreeCovering::LoadSensitiveDelay => "load-sensitive-delay",
    }
}

fn fanout_handling_name(value: FanoutHandling) -> &'static str {
    match value {
        FanoutHandling::Disabled => "disabled",
        FanoutHandling::BranchCostHeuristic => "branch-cost-heuristic",
        FanoutHandling::InternalFanoutCells => "internal-fanout-cells",
        FanoutHandling::BranchesAndInternalCells => "branches-and-internal-cells",
    }
}

fn load_limit_mode_name(value: LoadLimitMode) -> &'static str {
    match value {
        LoadLimitMode::Ignore => "ignore",
        LoadLimitMode::Enforce => "enforce",
    }
}

fn strategy_name(value: MapInterfaceStrategy) -> &'static str {
    match value {
        MapInterfaceStrategy::SopPrimitiveLowering => "sop-primitive-lowering",
    }
}

fn gate_accounting_name(gate: &GateKind) -> &str {
    match gate {
        GateKind::Library(name) => name.as_str(),
        _ => gate.mnemonic(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ports::map::com_map::MapOptions;
    use crate::ports::map::map_interface::{
        ComMapOptions, MapInterfaceResult, MapInterfaceStrategy,
    };

    fn sample_network() -> VirtualMappedNetwork {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let and = network.add_gate(
            "n1",
            GateKind::And,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        let inv = network.add_gate("n2", GateKind::Inverter, vec![SourceRef::Node(and)]);
        let wire = network.add_gate("w", GateKind::Wire, vec![SourceRef::Node(inv)]);
        network
            .add_primary_output("f", SourceRef::Node(wire))
            .unwrap();
        network.setup_gate_links().unwrap();
        network
    }

    #[test]
    fn accounts_virtual_network_levels_and_gate_kinds() {
        let network = sample_network();
        let accounting = account_virtual_network(&network).unwrap();

        assert_eq!(accounting.node_count, 6);
        assert_eq!(accounting.primary_input_count, 2);
        assert_eq!(accounting.primary_output_count, 1);
        assert_eq!(accounting.active_internal_count, 3);
        assert_eq!(accounting.mapped_gate_count, 2);
        assert_eq!(accounting.wire_count, 1);
        assert_eq!(accounting.edge_count, 5);
        assert_eq!(accounting.maximum_fanin, 2);
        assert_eq!(accounting.maximum_fanout, 1);
        assert_eq!(accounting.level_count, 3);
        assert_eq!(accounting.gates_by_kind["and"], 1);
        assert_eq!(accounting.gates_by_kind["inv"], 1);
        assert_eq!(accounting.gates_by_kind["wire"], 1);
        assert_eq!(accounting.levels[1].nodes.len(), 1);
    }

    #[test]
    fn summarizes_options_with_legacy_modes() {
        let options = MapOptions {
            cost_mode: MapCostMode::Delay,
            tree_covering: TreeCovering::LoadSensitiveDelay,
            fanout_handling: FanoutHandling::Disabled,
            raw_mode: true,
            print_statistics: true,
            verbosity: 2,
            ..MapOptions::default()
        };

        let summary = summarize_options(&options);

        assert_eq!(summary.cost_mode, "delay");
        assert_eq!(summary.tree_covering, "load-sensitive-delay");
        assert_eq!(summary.fanout_handling, "disabled");
        assert_eq!(summary.load_limit_mode, "enforce");
        assert!(summary.raw_mode);
        assert!(summary.print_statistics);
        assert_eq!(summary.verbosity, 2);
    }

    #[test]
    fn summarizes_mapping_diagnostics_and_library_gate_use() {
        let result = MapInterfaceResult {
            network: sample_network(),
            strategy: MapInterfaceStrategy::SopPrimitiveLowering,
            options: ComMapOptions::default(),
            diagnostics: vec![
                MapDiagnostic::SopPrimitiveReplacementPath,
                MapDiagnostic::FalseOutputRowsIgnored {
                    node: "n1".to_owned(),
                    count: 2,
                },
                MapDiagnostic::LibraryPrimitiveSelected {
                    primitive: super::super::map_interface::MapPrimitive::And,
                    gate: "nand2".to_owned(),
                    input_count: 2,
                },
                MapDiagnostic::LibraryPrimitiveSelected {
                    primitive: super::super::map_interface::MapPrimitive::And,
                    gate: "nand2".to_owned(),
                    input_count: 2,
                },
            ],
        };

        let summary = summarize_diagnostics(&result);

        assert_eq!(summary.strategy, "sop-primitive-lowering");
        assert!(summary.replacement_path);
        assert_eq!(summary.false_output_rows_ignored, 2);
        assert_eq!(summary.library_primitive_selections, 2);
        assert_eq!(summary.library_gate_use["nand2"], 2);
    }

    #[test]
    fn combines_result_options_diagnostics_and_accounting() {
        let result = MapInterfaceResult {
            network: sample_network(),
            strategy: MapInterfaceStrategy::SopPrimitiveLowering,
            options: ComMapOptions::default(),
            diagnostics: vec![MapDiagnostic::SopPrimitiveReplacementPath],
        };

        let summary = summarize_mapping_result(&result, &MapOptions::default()).unwrap();

        assert_eq!(summary.options.cost_mode, "area");
        assert!(summary.diagnostics.replacement_path);
        assert_eq!(summary.accounting.mapped_gate_count, 2);
    }
}
