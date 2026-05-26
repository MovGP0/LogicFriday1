//! Native Rust no-algorithm fanout construction for `sis/map/noalg.c`.
//!
//! The SIS implementation is the simplest fanout optimizer: for each source and
//! sink polarity, it either drives the sinks directly or uses at most one
//! inverter/buffer to serve the opposite polarity. This port keeps that behavior
//! over owned Rust fanout data and the shared native fanout utility layer. Full
//! SIS `network_t` extraction remains a higher-level integration concern.

use std::error::Error;
use std::fmt;

use super::fanout_tree::{
    FanoutPolarity as TreePolarity, FanoutSink as TreeSink, FanoutTreeError, FanoutTreeForest,
    FanoutTreeNode,
};
use super::fanout_util::{
    self, FanoutGateCatalog, FanoutInfo, FanoutOptimizationResult, FanoutUtilError, MergeTable,
    PLUS_INFINITY, Polarity, SelectedSource, SingleSource,
};
use super::virtual_net::DelayTime;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoAlgEntry {
    pub required: DelayTime,
    pub area: f64,
}

impl NoAlgEntry {
    pub fn initial() -> Self {
        Self {
            required: fanout_util::MINUS_INFINITY,
            area: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoAlgTable {
    entries: Vec<NoAlgEntry>,
    gate_count: usize,
}

impl NoAlgTable {
    pub fn entry(&self, gate_index: usize, polarity: Polarity) -> Result<NoAlgEntry, NoAlgError> {
        let index = self.index(gate_index, polarity)?;
        Ok(self.entries[index])
    }

    fn new(gate_count: usize) -> Self {
        Self {
            entries: vec![NoAlgEntry::initial(); gate_count * 2],
            gate_count,
        }
    }

    fn set(
        &mut self,
        gate_index: usize,
        polarity: Polarity,
        value: NoAlgEntry,
    ) -> Result<(), NoAlgError> {
        let index = self.index(gate_index, polarity)?;
        self.entries[index] = value;
        Ok(())
    }

    fn index(&self, gate_index: usize, polarity: Polarity) -> Result<usize, NoAlgError> {
        if gate_index >= self.gate_count {
            return Err(NoAlgError::GateIndexOutOfRange {
                index: gate_index,
                gate_count: self.gate_count,
            });
        }

        let polarity_index = match polarity {
            Polarity::X => 0,
            Polarity::Y => 1,
        };

        Ok(gate_index * 2 + polarity_index)
    }
}

pub trait NoAlgTiming {
    fn gate_area(&self, gate_index: usize) -> Result<f64, FanoutUtilError>;

    fn buffer_load(&self, gate_index: usize) -> Result<f64, FanoutUtilError>;

    fn backward_intrinsic(
        &self,
        required: DelayTime,
        gate_index: usize,
    ) -> Result<DelayTime, FanoutUtilError>;

    fn backward_load_dependent(
        &self,
        required: DelayTime,
        gate_index: usize,
        load: f64,
    ) -> Result<DelayTime, FanoutUtilError>;

    fn wire_load(&self, fanout_count: usize) -> Result<f64, FanoutUtilError>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum NoAlgError {
    FanoutUtil(FanoutUtilError),
    FanoutTree(FanoutTreeError),
    GateIndexOutOfRange { index: usize, gate_count: usize },
    CannotServeTwoPolaritiesFromSameOutput,
    MissingSisPorts { operation: &'static str },
}

impl fmt::Display for NoAlgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FanoutUtil(error) => write!(f, "{error}"),
            Self::FanoutTree(error) => write!(f, "{error}"),
            Self::GateIndexOutOfRange { index, gate_count } => write!(
                f,
                "noalg gate index {index} is outside the catalog of {gate_count} gates"
            ),
            Self::CannotServeTwoPolaritiesFromSameOutput => {
                write!(
                    f,
                    "noalg cannot service two sink polarities from the same source output"
                )
            }
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for NoAlgError {}

impl From<FanoutUtilError> for NoAlgError {
    fn from(value: FanoutUtilError) -> Self {
        Self::FanoutUtil(value)
    }
}

impl From<FanoutTreeError> for NoAlgError {
    fn from(value: FanoutTreeError) -> Self {
        Self::FanoutTree(value)
    }
}

pub fn full_sis_noalg_unavailable() -> Result<FanoutOptimizationResult<FanoutTreeForest>, NoAlgError>
{
    Err(NoAlgError::MissingSisPorts {
        operation: "noalg full SIS fanout-info extraction",
    })
}

pub fn optimize_noalg(
    fanout_info: &FanoutInfo,
    gates: &FanoutGateCatalog,
    timing: &impl NoAlgTiming,
) -> Result<FanoutOptimizationResult<FanoutTreeForest>, NoAlgError> {
    fanout_info.validate()?;
    let table = optimize_one_source(fanout_info, gates, timing)?;
    let mut merge_table = MergeTable::new(gates.gates().len())?;
    extract_merge_info(fanout_info, gates, timing, &table, &mut merge_table)?;
    let delay_model = |required: DelayTime, gate_index: usize, load: f64| {
        timing.backward_load_dependent(required, gate_index, load)
    };
    let wire_model = |fanout_count: usize| timing.wire_load(fanout_count);
    let (selected_source, cost) =
        fanout_util::select_best_source(gates, &merge_table, &delay_model, &wire_model)?;
    let tree = build_selected_tree(fanout_info, gates, selected_source)?;

    Ok(FanoutOptimizationResult {
        tree,
        selected_source,
        cost,
    })
}

pub fn optimize_one_source(
    fanout_info: &FanoutInfo,
    gates: &FanoutGateCatalog,
    timing: &impl NoAlgTiming,
) -> Result<NoAlgTable, NoAlgError> {
    fanout_info.validate()?;
    let mut table = NoAlgTable::new(gates.gates().len());

    for source_index in 0..gates.gates().len() {
        let gate = gates.gate(source_index)?;
        for source_polarity in [Polarity::X, Polarity::Y] {
            if gate.is_source() && gates.source_polarity(source_index)? != source_polarity {
                continue;
            }

            let entry = compute_best_one_source_one_polarity(
                fanout_info,
                source_polarity,
                source_index,
                timing,
            )?;
            table.set(source_index, source_polarity, entry)?;
        }
    }

    Ok(table)
}

pub fn build_selected_tree(
    fanout_info: &FanoutInfo,
    gates: &FanoutGateCatalog,
    source: SelectedSource,
) -> Result<FanoutTreeForest, NoAlgError> {
    fanout_info.validate()?;
    let p = source.main_source_sink_polarity;
    let q = p.invert();
    let p_sinks = fanout_info.sinks(p);
    let q_sinks = fanout_info.sinks(q);
    let mut nodes = Vec::new();

    if !q_sinks.is_empty() {
        if gates.gate(source.buffer)?.is_source() {
            return Err(NoAlgError::CannotServeTwoPolaritiesFromSameOutput);
        }

        nodes.push(FanoutTreeNode::buffer(
            source.main_source,
            p_sinks.len() + 1,
        ));
        insert_sinks(&mut nodes, p_sinks);
        nodes.push(FanoutTreeNode::buffer(source.buffer, q_sinks.len()));
        insert_sinks(&mut nodes, q_sinks);
    } else {
        nodes.push(FanoutTreeNode::buffer(source.main_source, p_sinks.len()));
        insert_sinks(&mut nodes, p_sinks);
    }

    FanoutTreeForest::from_prefix(nodes).map_err(NoAlgError::from)
}

pub fn insert_sinks(nodes: &mut Vec<FanoutTreeNode>, sinks: &[fanout_util::FanoutSink]) {
    nodes.extend(sinks.iter().map(|sink| {
        FanoutTreeNode::sink(TreeSink::new(
            format!("node{}", sink.sink.index()),
            sink.pin.max(0) as usize,
            to_tree_polarity(sink.polarity),
            sink.load,
            sink.required,
        ))
    }));
}

fn compute_best_one_source_one_polarity(
    fanout_info: &FanoutInfo,
    polarity: Polarity,
    source_index: usize,
    timing: &impl NoAlgTiming,
) -> Result<NoAlgEntry, NoAlgError> {
    let sinks = fanout_info.sinks(polarity);
    if sinks.is_empty() {
        return Ok(NoAlgEntry {
            required: PLUS_INFINITY,
            area: 0.0,
        });
    }

    let required = minimum_required(sinks);
    let load = total_load(sinks) + timing.wire_load(sinks.len())?;
    Ok(NoAlgEntry {
        required: timing.backward_load_dependent(required, source_index, load)?,
        area: timing.gate_area(source_index)?,
    })
}

fn extract_merge_info(
    fanout_info: &FanoutInfo,
    gates: &FanoutGateCatalog,
    timing: &impl NoAlgTiming,
    table: &NoAlgTable,
    merge_table: &mut MergeTable,
) -> Result<(), NoAlgError> {
    for source_index in 0..gates.gates().len() {
        let gate = gates.gate(source_index)?;
        if gate.is_source() {
            let p = gates.source_polarity(source_index)?;
            let sinks = fanout_info.sinks(p);
            let value = if sinks.is_empty() {
                SingleSource {
                    required: PLUS_INFINITY,
                    load: 0.0,
                    fanout_count: 0,
                    area: 0.0,
                }
            } else {
                SingleSource {
                    required: minimum_required(sinks),
                    load: total_load(sinks),
                    fanout_count: sinks.len(),
                    area: 0.0,
                }
            };
            merge_table.set(source_index, p, p, value)?;
            continue;
        }

        for p in [Polarity::X, Polarity::Y] {
            let from = table.entry(source_index, p)?;
            let value = if fanout_info.sinks(p).is_empty() {
                SingleSource {
                    required: PLUS_INFINITY,
                    load: 0.0,
                    fanout_count: 0,
                    area: 0.0,
                }
            } else {
                SingleSource {
                    required: timing.backward_intrinsic(from.required, source_index)?,
                    load: timing.buffer_load(source_index)?,
                    fanout_count: 1,
                    area: from.area,
                }
            };
            merge_table.set(source_index, p, p, value)?;
        }
    }

    Ok(())
}

fn minimum_required(sinks: &[fanout_util::FanoutSink]) -> DelayTime {
    sinks
        .iter()
        .fold(PLUS_INFINITY, |required, sink| required.min(sink.required))
}

fn total_load(sinks: &[fanout_util::FanoutSink]) -> f64 {
    sinks.iter().map(|sink| sink.load).sum()
}

fn to_tree_polarity(polarity: Polarity) -> TreePolarity {
    match polarity {
        Polarity::X => TreePolarity::Positive,
        Polarity::Y => TreePolarity::Negative,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::fanout_util::{FanoutGate, FanoutSink};
    use crate::ports::map::virtual_net::NodeId;

    struct TestTiming {
        areas: Vec<f64>,
        loads: Vec<f64>,
        load_factor: Vec<f64>,
        intrinsic: Vec<f64>,
        wire_load: f64,
    }

    impl NoAlgTiming for TestTiming {
        fn gate_area(&self, gate_index: usize) -> Result<f64, FanoutUtilError> {
            self.value(&self.areas, gate_index)
        }

        fn buffer_load(&self, gate_index: usize) -> Result<f64, FanoutUtilError> {
            self.value(&self.loads, gate_index)
        }

        fn backward_intrinsic(
            &self,
            required: DelayTime,
            gate_index: usize,
        ) -> Result<DelayTime, FanoutUtilError> {
            let intrinsic = self.value(&self.intrinsic, gate_index)?;
            Ok(DelayTime::new(
                required.rise - intrinsic,
                required.fall - intrinsic,
            ))
        }

        fn backward_load_dependent(
            &self,
            required: DelayTime,
            gate_index: usize,
            load: f64,
        ) -> Result<DelayTime, FanoutUtilError> {
            let factor = self.value(&self.load_factor, gate_index)?;
            Ok(DelayTime::new(
                required.rise - load * factor,
                required.fall - load * factor,
            ))
        }

        fn wire_load(&self, fanout_count: usize) -> Result<f64, FanoutUtilError> {
            Ok(fanout_count as f64 * self.wire_load)
        }
    }

    impl TestTiming {
        fn value(&self, values: &[f64], index: usize) -> Result<f64, FanoutUtilError> {
            values
                .get(index)
                .copied()
                .ok_or(FanoutUtilError::GateIndexOutOfRange {
                    index,
                    gate_count: values.len(),
                })
        }
    }

    fn node(index: usize) -> NodeId {
        let mut network = crate::ports::map::virtual_net::VirtualMappedNetwork::new();
        let mut id = None;
        for current in 0..=index {
            id = Some(network.add_primary_input(format!("n{current}")));
        }
        id.expect("loop always creates the requested node")
    }

    fn sink(index: usize, polarity: Polarity, required: f64) -> FanoutSink {
        FanoutSink {
            sink: node(index),
            pin: 0,
            polarity,
            load: 1.0,
            required: DelayTime::new(required, required),
        }
    }

    fn timing() -> TestTiming {
        TestTiming {
            areas: vec![2.0, 3.0, 0.0],
            loads: vec![1.0, 1.0, 0.0],
            load_factor: vec![0.1, 0.1, 0.05],
            intrinsic: vec![0.25, 0.5, 0.0],
            wire_load: 0.0,
        }
    }

    #[test]
    fn builds_direct_tree_for_one_polarity() {
        let gates = FanoutGateCatalog::new(vec![
            FanoutGate::buffer("buf", false),
            FanoutGate::buffer("inv", true),
            FanoutGate::source("src", Polarity::X),
        ])
        .unwrap();
        let info = FanoutInfo::new(
            vec![sink(0, Polarity::X, 10.0), sink(1, Polarity::X, 8.0)],
            Vec::new(),
        )
        .unwrap();

        let result = optimize_noalg(&info, &gates, &timing()).unwrap();

        assert_eq!(result.selected_source.main_source, 2);
        assert_eq!(result.tree.nodes().len(), 3);
        assert_eq!(result.cost.area, 0.0);
        assert_eq!(result.cost.slack, DelayTime::new(7.9, 7.9));
    }

    #[test]
    fn inserts_one_buffer_for_opposite_polarity_sinks() {
        let gates = FanoutGateCatalog::new(vec![
            FanoutGate::buffer("buf", false),
            FanoutGate::buffer("inv", true),
            FanoutGate::source("src", Polarity::X),
        ])
        .unwrap();
        let info = FanoutInfo::new(
            vec![sink(0, Polarity::X, 10.0)],
            vec![sink(1, Polarity::Y, 8.0)],
        )
        .unwrap();

        let result = optimize_noalg(&info, &gates, &timing()).unwrap();

        assert_eq!(result.selected_source.main_source, 2);
        assert_eq!(result.selected_source.buffer, 1);
        assert_eq!(result.tree.nodes().len(), 4);
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("noalg.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
