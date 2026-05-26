//! Native Rust fanout utility helpers for `sis/map/fanout_util.c`.
//!
//! The original C file is the common merge layer used by several fanout
//! optimization algorithms. This port keeps that behavior over owned Rust
//! data: callers fill a merge table, the utility code selects the best
//! main-source/buffer combination, and cost computation is driven by explicit
//! delay and wire-load providers instead of hidden C globals.
//! It intentionally exposes no legacy C ABI entry points.

use std::error::Error;
use std::fmt;

use super::virtual_net::{DelayTime, GateLink, NodeId, VirtualMappedNetwork, VirtualNetworkError};

pub const MINUS_INFINITY: DelayTime = DelayTime {
    rise: f64::NEG_INFINITY,
    fall: f64::NEG_INFINITY,
};
pub const PLUS_INFINITY: DelayTime = DelayTime {
    rise: f64::INFINITY,
    fall: f64::INFINITY,
};
pub const ZERO_DELAY: DelayTime = DelayTime {
    rise: 0.0,
    fall: 0.0,
};
pub const SINGLE_SOURCE_INIT_VALUE: SingleSource = SingleSource {
    required: MINUS_INFINITY,
    load: 0.0,
    fanout_count: 0,
    area: 0.0,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Polarity {
    X,
    Y,
}

impl Polarity {
    pub fn invert(self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::X,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
        }
    }

    fn all() -> [Self; 2] {
        [Self::X, Self::Y]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SingleSource {
    pub required: DelayTime,
    pub load: f64,
    pub fanout_count: usize,
    pub area: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectedSource {
    pub main_source: usize,
    pub main_source_sink_polarity: Polarity,
    pub buffer: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutCost {
    pub slack: DelayTime,
    pub area: f64,
}

impl FanoutCost {
    pub fn infeasible() -> Self {
        Self {
            slack: MINUS_INFINITY,
            area: f64::INFINITY,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FanoutGateRole {
    Source { polarity: Polarity },
    Buffer { inverting: bool },
}

impl FanoutGateRole {
    fn buffer_output_polarity(&self, source_polarity: Polarity) -> Option<Polarity> {
        match self {
            Self::Source { polarity } => Some(*polarity),
            Self::Buffer { inverting } => Some(if *inverting {
                source_polarity.invert()
            } else {
                source_polarity
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutGate {
    pub name: String,
    pub role: FanoutGateRole,
}

impl FanoutGate {
    pub fn source(name: impl Into<String>, polarity: Polarity) -> Self {
        Self {
            name: name.into(),
            role: FanoutGateRole::Source { polarity },
        }
    }

    pub fn buffer(name: impl Into<String>, inverting: bool) -> Self {
        Self {
            name: name.into(),
            role: FanoutGateRole::Buffer { inverting },
        }
    }

    pub fn is_source(&self) -> bool {
        matches!(self.role, FanoutGateRole::Source { .. })
    }

    pub fn source_polarity(&self) -> Option<Polarity> {
        match self.role {
            FanoutGateRole::Source { polarity } => Some(polarity),
            FanoutGateRole::Buffer { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutGateCatalog {
    gates: Vec<FanoutGate>,
}

impl FanoutGateCatalog {
    pub fn new(gates: Vec<FanoutGate>) -> Result<Self, FanoutUtilError> {
        if gates.is_empty() {
            return Err(FanoutUtilError::EmptyGateCatalog);
        }
        if !gates.iter().any(FanoutGate::is_source) {
            return Err(FanoutUtilError::NoSourceGates);
        }

        Ok(Self { gates })
    }

    pub fn gates(&self) -> &[FanoutGate] {
        &self.gates
    }

    pub fn gate(&self, index: usize) -> Result<&FanoutGate, FanoutUtilError> {
        self.gates
            .get(index)
            .ok_or(FanoutUtilError::GateIndexOutOfRange {
                index,
                gate_count: self.gates.len(),
            })
    }

    pub fn source_polarity(&self, index: usize) -> Result<Polarity, FanoutUtilError> {
        self.gate(index)?
            .source_polarity()
            .ok_or(FanoutUtilError::GateIsNotSource { index })
    }

    fn len(&self) -> usize {
        self.gates.len()
    }
}

pub trait FanoutDelayModel {
    fn backward_load_dependent(
        &self,
        required: DelayTime,
        gate_index: usize,
        load: f64,
    ) -> Result<DelayTime, FanoutUtilError>;
}

impl<F> FanoutDelayModel for F
where
    F: Fn(DelayTime, usize, f64) -> Result<DelayTime, FanoutUtilError>,
{
    fn backward_load_dependent(
        &self,
        required: DelayTime,
        gate_index: usize,
        load: f64,
    ) -> Result<DelayTime, FanoutUtilError> {
        self(required, gate_index, load)
    }
}

pub trait WireLoadModel {
    fn wire_load(&self, fanout_count: usize) -> Result<f64, FanoutUtilError>;
}

impl<F> WireLoadModel for F
where
    F: Fn(usize) -> Result<f64, FanoutUtilError>,
{
    fn wire_load(&self, fanout_count: usize) -> Result<f64, FanoutUtilError> {
        self(fanout_count)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MergeTable {
    entries: Vec<SingleSource>,
    gate_count: usize,
}

impl MergeTable {
    pub fn new(gate_count: usize) -> Result<Self, FanoutUtilError> {
        if gate_count == 0 {
            return Err(FanoutUtilError::EmptyGateCatalog);
        }

        Ok(Self {
            entries: vec![SINGLE_SOURCE_INIT_VALUE; gate_count * 4],
            gate_count,
        })
    }

    pub fn gate_count(&self) -> usize {
        self.gate_count
    }

    pub fn get(
        &self,
        gate_index: usize,
        source_polarity: Polarity,
        sink_polarity: Polarity,
    ) -> Result<SingleSource, FanoutUtilError> {
        let index = self.index(gate_index, source_polarity, sink_polarity)?;
        Ok(self.entries[index])
    }

    pub fn set(
        &mut self,
        gate_index: usize,
        source_polarity: Polarity,
        sink_polarity: Polarity,
        value: SingleSource,
    ) -> Result<(), FanoutUtilError> {
        validate_single_source(value)?;
        let index = self.index(gate_index, source_polarity, sink_polarity)?;
        self.entries[index] = value;
        Ok(())
    }

    fn index(
        &self,
        gate_index: usize,
        source_polarity: Polarity,
        sink_polarity: Polarity,
    ) -> Result<usize, FanoutUtilError> {
        if gate_index >= self.gate_count {
            return Err(FanoutUtilError::GateIndexOutOfRange {
                index: gate_index,
                gate_count: self.gate_count,
            });
        }

        Ok((gate_index * 4) + (source_polarity.index() * 2) + sink_polarity.index())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSink {
    pub sink: NodeId,
    pub pin: isize,
    pub polarity: Polarity,
    pub load: f64,
    pub required: DelayTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutInfo {
    pub sinks: [Vec<FanoutSink>; 2],
}

impl FanoutInfo {
    pub fn new(
        x_sinks: Vec<FanoutSink>,
        y_sinks: Vec<FanoutSink>,
    ) -> Result<Self, FanoutUtilError> {
        let info = Self {
            sinks: [x_sinks, y_sinks],
        };
        info.validate()?;
        Ok(info)
    }

    pub fn empty() -> Self {
        Self {
            sinks: [Vec::new(), Vec::new()],
        }
    }

    pub fn sinks(&self, polarity: Polarity) -> &[FanoutSink] {
        &self.sinks[polarity.index()]
    }

    pub fn sinks_mut(&mut self, polarity: Polarity) -> &mut Vec<FanoutSink> {
        &mut self.sinks[polarity.index()]
    }

    pub fn total_count(&self) -> usize {
        self.sinks.iter().map(Vec::len).sum()
    }

    pub fn validate(&self) -> Result<(), FanoutUtilError> {
        for polarity in Polarity::all() {
            for sink in self.sinks(polarity) {
                if sink.polarity != polarity {
                    return Err(FanoutUtilError::SinkPolarityMismatch {
                        sink: sink.sink,
                        expected: polarity,
                        actual: sink.polarity,
                    });
                }
                validate_load(sink.load)?;
                validate_delay_time(sink.required)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutOptimizationResult<Tree> {
    pub tree: Tree,
    pub selected_source: SelectedSource,
    pub cost: FanoutCost,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FanoutUtilError {
    EmptyDelayArray,
    EmptyGateCatalog,
    NoSourceGates,
    GateIndexOutOfRange {
        index: usize,
        gate_count: usize,
    },
    GateIsNotSource {
        index: usize,
    },
    InvalidLoad(f64),
    InvalidFanoutCount(usize),
    InvalidDelayTime(DelayTime),
    NoFeasibleSource,
    SinkPolarityMismatch {
        sink: NodeId,
        expected: Polarity,
        actual: Polarity,
    },
    VirtualNetwork(VirtualNetworkError),
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for FanoutUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDelayArray => write!(f, "cannot select a minimum index from an empty array"),
            Self::EmptyGateCatalog => write!(f, "fanout gate catalog cannot be empty"),
            Self::NoSourceGates => {
                write!(f, "fanout gate catalog must contain at least one source")
            }
            Self::GateIndexOutOfRange { index, gate_count } => write!(
                f,
                "fanout gate index {index} is outside the catalog of {gate_count} gates"
            ),
            Self::GateIsNotSource { index } => write!(f, "fanout gate {index} is not a source"),
            Self::InvalidLoad(load) => write!(f, "invalid fanout load {load}"),
            Self::InvalidFanoutCount(count) => write!(f, "invalid fanout count {count}"),
            Self::InvalidDelayTime(time) => {
                write!(f, "invalid delay time ({}, {})", time.rise, time.fall)
            }
            Self::NoFeasibleSource => write!(f, "fanout merge table has no feasible source"),
            Self::SinkPolarityMismatch {
                sink,
                expected,
                actual,
            } => write!(
                f,
                "sink {} was placed in {expected:?} fanout info but is marked {actual:?}",
                sink.index()
            ),
            Self::VirtualNetwork(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => write!(f, "{operation} requires unavailable native SIS integration"),
        }
    }
}

impl Error for FanoutUtilError {}

impl From<VirtualNetworkError> for FanoutUtilError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

pub fn full_sis_fanout_optimization_unavailable<Tree>()
-> Result<FanoutOptimizationResult<Tree>, FanoutUtilError> {
    Err(FanoutUtilError::MissingSisPorts {
        operation: "fanout_util full SIS fanout optimization",
    })
}

pub fn find_minimum_index(array: &[DelayTime]) -> Result<usize, FanoutUtilError> {
    array
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| delay_min(**left).total_cmp(&delay_min(**right)))
        .map(|(index, _)| index)
        .ok_or(FanoutUtilError::EmptyDelayArray)
}

pub fn compute_merge_cost(
    source_index: usize,
    entry_x: SingleSource,
    entry_y: SingleSource,
    delay_model: &impl FanoutDelayModel,
    wire_load: &impl WireLoadModel,
) -> Result<FanoutCost, FanoutUtilError> {
    validate_single_source(entry_x)?;
    validate_single_source(entry_y)?;

    let required = min_delay_time(entry_x.required, entry_y.required);
    let fanout_count = entry_x
        .fanout_count
        .checked_add(entry_y.fanout_count)
        .ok_or(FanoutUtilError::InvalidFanoutCount(usize::MAX))?;
    let load = entry_x.load + entry_y.load + wire_load.wire_load(fanout_count)?;
    validate_load(load)?;

    Ok(FanoutCost {
        slack: delay_model.backward_load_dependent(required, source_index, load)?,
        area: entry_x.area + entry_y.area,
    })
}

pub fn select_best_source(
    gates: &FanoutGateCatalog,
    merge_table: &MergeTable,
    delay_model: &impl FanoutDelayModel,
    wire_load: &impl WireLoadModel,
) -> Result<(SelectedSource, FanoutCost), FanoutUtilError> {
    if gates.len() != merge_table.gate_count() {
        return Err(FanoutUtilError::GateIndexOutOfRange {
            index: merge_table.gate_count(),
            gate_count: gates.len(),
        });
    }

    let mut best_source = None;
    let mut best_cost = FanoutCost::infeasible();

    for source_index in 0..gates.len() {
        if !gates.gate(source_index)?.is_source() {
            continue;
        }

        let (source, cost) =
            get_best_one_source(gates, merge_table, source_index, delay_model, wire_load)?;
        if is_better_cost(cost, best_cost) {
            best_source = Some(source);
            best_cost = cost;
        }
    }

    best_source
        .map(|source| (source, best_cost))
        .ok_or(FanoutUtilError::NoFeasibleSource)
}

pub fn generic_fanout_optimizer<Table, Tree>(
    fanout_info: &FanoutInfo,
    gates: &FanoutGateCatalog,
    delay_model: &impl FanoutDelayModel,
    wire_load: &impl WireLoadModel,
    optimize_one_source: impl FnOnce(&FanoutInfo) -> Result<Table, FanoutUtilError>,
    extract_merge_info: impl FnOnce(&FanoutInfo, &Table, &mut MergeTable) -> Result<(), FanoutUtilError>,
    build_tree: impl FnOnce(&FanoutInfo, SelectedSource, &Table) -> Result<Tree, FanoutUtilError>,
) -> Result<FanoutOptimizationResult<Tree>, FanoutUtilError> {
    fanout_info.validate()?;
    let table = optimize_one_source(fanout_info)?;
    let mut merge_table = MergeTable::new(gates.len())?;
    extract_merge_info(fanout_info, &table, &mut merge_table)?;
    let (selected_source, cost) = select_best_source(gates, &merge_table, delay_model, wire_load)?;
    let tree = build_tree(fanout_info, selected_source, &table)?;

    Ok(FanoutOptimizationResult {
        tree,
        selected_source,
        cost,
    })
}

pub fn fanout_info_from_virtual_network(
    network: &VirtualMappedNetwork,
    source: NodeId,
    polarity_for_link: impl Fn(&GateLink) -> Polarity,
) -> Result<FanoutInfo, FanoutUtilError> {
    let mut info = FanoutInfo::empty();
    let node = network
        .node(source)
        .ok_or(VirtualNetworkError::MissingNode(source))?;

    for link in node.gate_links() {
        let polarity = polarity_for_link(link);
        info.sinks_mut(polarity).push(FanoutSink {
            sink: link.node,
            pin: link.pin,
            polarity,
            load: link.load,
            required: link.required,
        });
    }

    info.validate()?;
    Ok(info)
}

fn get_best_one_source(
    gates: &FanoutGateCatalog,
    merge_table: &MergeTable,
    source_index: usize,
    delay_model: &impl FanoutDelayModel,
    wire_load: &impl WireLoadModel,
) -> Result<(SelectedSource, FanoutCost), FanoutUtilError> {
    let source_polarity = gates.source_polarity(source_index)?;
    let mut best_source = None;
    let mut best_cost = FanoutCost::infeasible();

    for sink_polarity in Polarity::all() {
        let source_entry = merge_table.get(source_index, source_polarity, sink_polarity)?;

        for buffer_index in 0..gates.len() {
            let buffer_gate = gates.gate(buffer_index)?;
            let Some(buffer_polarity) = buffer_gate.role.buffer_output_polarity(source_polarity)
            else {
                continue;
            };
            if buffer_gate.is_source() && buffer_index != source_index {
                continue;
            }

            let buffer_entry =
                merge_table.get(buffer_index, buffer_polarity, sink_polarity.invert())?;
            let cost = compute_merge_cost(
                source_index,
                source_entry,
                buffer_entry,
                delay_model,
                wire_load,
            )?;
            if is_better_cost(cost, best_cost) {
                best_source = Some(SelectedSource {
                    main_source: source_index,
                    main_source_sink_polarity: sink_polarity,
                    buffer: buffer_index,
                });
                best_cost = cost;
            }
        }
    }

    best_source
        .map(|source| (source, best_cost))
        .ok_or(FanoutUtilError::NoFeasibleSource)
}

fn validate_single_source(value: SingleSource) -> Result<(), FanoutUtilError> {
    validate_delay_time(value.required)?;
    validate_load(value.load)?;
    if !value.area.is_finite() || value.area < 0.0 {
        return Err(FanoutUtilError::InvalidLoad(value.area));
    }

    Ok(())
}

fn validate_load(load: f64) -> Result<(), FanoutUtilError> {
    if !load.is_finite() || load < 0.0 {
        return Err(FanoutUtilError::InvalidLoad(load));
    }

    Ok(())
}

fn validate_delay_time(time: DelayTime) -> Result<(), FanoutUtilError> {
    if time.rise.is_nan() || time.fall.is_nan() {
        return Err(FanoutUtilError::InvalidDelayTime(time));
    }

    Ok(())
}

fn is_better_cost(candidate: FanoutCost, incumbent: FanoutCost) -> bool {
    delay_min(candidate.slack) > delay_min(incumbent.slack)
        || (delay_min(candidate.slack) == delay_min(incumbent.slack)
            && candidate.area < incumbent.area)
}

fn delay_min(value: DelayTime) -> f64 {
    value.rise.min(value.fall)
}

fn min_delay_time(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise.min(right.rise), left.fall.min(right.fall))
}

#[cfg(test)]
mod tests {
    use super::super::virtual_net::{GateKind, SourceRef};
    use super::*;

    fn identity_delay(
        required: DelayTime,
        gate_index: usize,
        load: f64,
    ) -> Result<DelayTime, FanoutUtilError> {
        Ok(DelayTime::new(
            required.rise - gate_index as f64 - load,
            required.fall - gate_index as f64 - load,
        ))
    }

    fn zero_wire_load(_: usize) -> Result<f64, FanoutUtilError> {
        Ok(0.0)
    }

    fn entry(required: f64, load: f64, fanout_count: usize, area: f64) -> SingleSource {
        SingleSource {
            required: DelayTime::new(required, required + 1.0),
            load,
            fanout_count,
            area,
        }
    }

    #[test]
    fn find_minimum_index_matches_c_getmin_selection() {
        let values = [
            DelayTime::new(2.0, 9.0),
            DelayTime::new(5.0, 6.0),
            DelayTime::new(4.0, 12.0),
        ];

        assert_eq!(find_minimum_index(&values).unwrap(), 1);
    }

    #[test]
    fn merge_cost_combines_required_load_wire_and_area() {
        let cost = compute_merge_cost(
            2,
            entry(10.0, 1.5, 2, 3.0),
            entry(8.0, 2.0, 1, 4.0),
            &identity_delay,
            &|fanouts| Ok(fanouts as f64 * 0.25),
        )
        .unwrap();

        assert_eq!(cost.slack, DelayTime::new(1.75, 2.75));
        assert_eq!(cost.area, 7.0);
    }

    #[test]
    fn select_best_source_checks_source_and_buffer_polarities() {
        let gates = FanoutGateCatalog::new(vec![
            FanoutGate::source("src", Polarity::X),
            FanoutGate::buffer("buf", false),
            FanoutGate::buffer("inv", true),
        ])
        .unwrap();
        let mut merge_table = MergeTable::new(gates.len()).unwrap();

        merge_table
            .set(0, Polarity::X, Polarity::X, entry(10.0, 1.0, 1, 1.0))
            .unwrap();
        merge_table
            .set(2, Polarity::Y, Polarity::Y, entry(9.0, 1.0, 1, 2.0))
            .unwrap();
        merge_table
            .set(0, Polarity::X, Polarity::Y, entry(1.0, 1.0, 1, 1.0))
            .unwrap();

        let (source, cost) =
            select_best_source(&gates, &merge_table, &identity_delay, &zero_wire_load).unwrap();

        assert_eq!(
            source,
            SelectedSource {
                main_source: 0,
                main_source_sink_polarity: Polarity::X,
                buffer: 2,
            }
        );
        assert_eq!(cost.slack, DelayTime::new(7.0, 8.0));
    }

    #[test]
    fn generic_optimizer_threads_owned_table_and_tree_callbacks() {
        let gates = FanoutGateCatalog::new(vec![FanoutGate::source("src", Polarity::X)]).unwrap();
        let fanout_info = FanoutInfo::empty();

        let result = generic_fanout_optimizer(
            &fanout_info,
            &gates,
            &identity_delay,
            &zero_wire_load,
            |_| Ok(vec![42usize]),
            |_, table, merge_table| {
                assert_eq!(table, &[42usize]);
                merge_table.set(0, Polarity::X, Polarity::X, entry(5.0, 0.0, 0, 1.0))?;
                merge_table.set(0, Polarity::X, Polarity::Y, entry(4.0, 0.0, 0, 2.0))
            },
            |_, selected, table| Ok((selected, table[0])),
        )
        .unwrap();

        assert_eq!(result.tree.1, 42);
        assert_eq!(result.selected_source.main_source, 0);
        assert_eq!(result.cost.slack, DelayTime::new(4.0, 5.0));
    }

    #[test]
    fn extracts_owned_fanout_info_from_virtual_network_links() {
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let gate = network.add_gate(
            "n1",
            GateKind::And,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        network
            .add_primary_output("y", SourceRef::Node(gate))
            .unwrap();
        network.setup_gate_links().unwrap();

        let info = fanout_info_from_virtual_network(&network, a, |_| Polarity::X).unwrap();

        assert_eq!(info.total_count(), 1);
        assert_eq!(info.sinks(Polarity::X)[0].sink, gate);
        assert_eq!(info.sinks(Polarity::X)[0].pin, 0);
    }
}
