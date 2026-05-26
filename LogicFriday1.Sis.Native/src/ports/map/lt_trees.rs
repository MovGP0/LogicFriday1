//! Native Rust load-threshold fanout-tree planning for `sis/map/lt_trees.c`.
//!
//! The original SIS implementation stores its dynamic-programming table in
//! `multidim_t` and delegates source selection to the generic fanout optimizer.
//! This port keeps the same LT-tree recurrence over owned Rust data. Callers
//! provide gate timing metadata, polarity-partitioned sinks, and the two-level
//! candidates produced by the sibling two-level planner; the result is a
//! reusable table plus tree-building helpers. No legacy per-file C ABI exports
//! are added here.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use super::virtual_net::DelayTime;

pub const PLUS_INFINITY: DelayTime = DelayTime {
    rise: f64::INFINITY,
    fall: f64::INFINITY,
};

pub const MINUS_INFINITY: DelayTime = DelayTime {
    rise: f64::NEG_INFINITY,
    fall: f64::NEG_INFINITY,
};

pub const ZERO_DELAY: DelayTime = DelayTime {
    rise: 0.0,
    fall: 0.0,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Polarity {
    Positive,
    Negative,
}

impl Polarity {
    pub fn inverted(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Positive => 0,
            Self::Negative => 1,
        }
    }

    fn all() -> [Self; 2] {
        [Self::Positive, Self::Negative]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateKind {
    Source { polarity: Polarity },
    Buffer,
    Inverter,
}

impl GateKind {
    fn output_polarity(self, input: Polarity) -> Polarity {
        match self {
            Self::Inverter => input.inverted(),
            Self::Source { polarity } => polarity,
            Self::Buffer => input,
        }
    }

    fn is_source(self) -> bool {
        matches!(self, Self::Source { .. })
    }

    fn is_buffer(self) -> bool {
        matches!(self, Self::Buffer | Self::Inverter)
    }

    fn source_polarity(self) -> Option<Polarity> {
        match self {
            Self::Source { polarity } => Some(polarity),
            Self::Buffer | Self::Inverter => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GateTiming {
    pub intrinsic: DelayTime,
    pub load_slope: DelayTime,
}

impl GateTiming {
    pub fn new(intrinsic: DelayTime, load_slope: DelayTime) -> Self {
        Self {
            intrinsic,
            load_slope,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutGate {
    pub name: String,
    pub kind: GateKind,
    pub area: f64,
    pub input_load: f64,
    pub timing: GateTiming,
}

impl FanoutGate {
    pub fn new(
        name: impl Into<String>,
        kind: GateKind,
        area: f64,
        input_load: f64,
        timing: GateTiming,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            area,
            input_load,
            timing,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSink {
    pub name: String,
    pub load: f64,
    pub min_required: DelayTime,
}

impl FanoutSink {
    pub fn new(name: impl Into<String>, load: f64, min_required: DelayTime) -> Self {
        Self {
            name: name.into(),
            load,
            min_required,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutSinks {
    pub polarity: Polarity,
    pub sinks: Vec<FanoutSink>,
}

impl FanoutSinks {
    pub fn new(polarity: Polarity, sinks: Vec<FanoutSink>) -> Self {
        Self { polarity, sinks }
    }

    pub fn len(&self) -> usize {
        self.sinks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }

    fn cumulative_loads(&self) -> Vec<f64> {
        let mut result = Vec::with_capacity(self.sinks.len() + 1);
        result.push(0.0);
        for sink in &self.sinks {
            result.push(result[result.len() - 1] + sink.load);
        }

        result
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutData {
    by_polarity: [FanoutSinks; 2],
    cumulative_loads: [Vec<f64>; 2],
}

impl FanoutData {
    pub fn new(positive: Vec<FanoutSink>, negative: Vec<FanoutSink>) -> Self {
        let by_polarity = [
            FanoutSinks::new(Polarity::Positive, positive),
            FanoutSinks::new(Polarity::Negative, negative),
        ];
        let cumulative_loads = [
            by_polarity[0].cumulative_loads(),
            by_polarity[1].cumulative_loads(),
        ];

        Self {
            by_polarity,
            cumulative_loads,
        }
    }

    pub fn sinks(&self, polarity: Polarity) -> &FanoutSinks {
        &self.by_polarity[polarity.index()]
    }

    fn cumulative_load(&self, polarity: Polarity, index: usize) -> Result<f64, LtTreeError> {
        self.cumulative_loads[polarity.index()]
            .get(index)
            .copied()
            .ok_or(LtTreeError::InvalidSinkIndex { polarity, index })
    }

    fn interval_load(
        &self,
        polarity: Polarity,
        start: usize,
        end: usize,
    ) -> Result<f64, LtTreeError> {
        Ok(self.cumulative_load(polarity, end)? - self.cumulative_load(polarity, start)?)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WireLoadModel {
    pub per_fanout: f64,
}

impl Default for WireLoadModel {
    fn default() -> Self {
        Self { per_fanout: 0.0 }
    }
}

impl WireLoadModel {
    pub fn load_for_fanout_count(self, fanout_count: usize) -> f64 {
        self.per_fanout * fanout_count as f64
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoLevelCandidate {
    pub gate_index: usize,
    pub gate_count: usize,
    pub required: DelayTime,
    pub area: f64,
}

impl TwoLevelCandidate {
    pub fn new(gate_index: usize, gate_count: usize, required: DelayTime, area: f64) -> Self {
        Self {
            gate_index,
            gate_count,
            required,
            area,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TwoLevelKey {
    pub source_index: usize,
    pub source_polarity: Polarity,
    pub sink_index: usize,
    pub sink_polarity: Polarity,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TwoLevelTable {
    entries: HashMap<TwoLevelKey, TwoLevelCandidate>,
}

impl TwoLevelTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: TwoLevelKey, candidate: TwoLevelCandidate) {
        self.entries.insert(key, candidate);
    }

    pub fn get(&self, key: TwoLevelKey) -> Option<&TwoLevelCandidate> {
        self.entries.get(&key)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct LtTreeKey {
    pub source_index: usize,
    pub source_polarity: Polarity,
    pub sink_index: usize,
    pub sink_polarity: Polarity,
    pub gaps_remaining: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LtTreeChoice {
    TwoLevel(TwoLevelCandidate),
    Split {
        buffer_index: usize,
        sink_index: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LtTreeEntry {
    pub choice: LtTreeChoice,
    pub required: DelayTime,
    pub area: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LtTreeOptions {
    pub max_gaps: usize,
    pub wire_load: WireLoadModel,
}

impl Default for LtTreeOptions {
    fn default() -> Self {
        Self {
            max_gaps: 5,
            wire_load: WireLoadModel::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LtTreeInput {
    pub gates: Vec<FanoutGate>,
    pub fanout_data: FanoutData,
    pub two_level: TwoLevelTable,
    pub options: LtTreeOptions,
}

impl LtTreeInput {
    pub fn new(
        gates: Vec<FanoutGate>,
        fanout_data: FanoutData,
        two_level: TwoLevelTable,
        options: LtTreeOptions,
    ) -> Result<Self, LtTreeError> {
        let input = Self {
            gates,
            fanout_data,
            two_level,
            options,
        };
        input.validate()?;
        Ok(input)
    }

    pub fn validate(&self) -> Result<(), LtTreeError> {
        if self.gates.is_empty() {
            return Err(LtTreeError::NoGates);
        }
        if self.options.max_gaps == 0 {
            return Err(LtTreeError::InvalidMaxGaps);
        }
        if !self.options.wire_load.per_fanout.is_finite()
            || self.options.wire_load.per_fanout < 0.0
        {
            return Err(LtTreeError::InvalidWireLoad);
        }
        if !self.gates.iter().any(|gate| gate.kind.is_source()) {
            return Err(LtTreeError::NoSources);
        }
        if !self.gates.iter().any(|gate| gate.kind.is_buffer()) {
            return Err(LtTreeError::NoBuffers);
        }
        for (index, gate) in self.gates.iter().enumerate() {
            validate_gate(index, gate)?;
        }
        for polarity in Polarity::all() {
            for (index, sink) in self.fanout_data.sinks(polarity).sinks.iter().enumerate() {
                validate_sink(polarity, index, sink)?;
            }
        }
        for (key, candidate) in &self.two_level.entries {
            if key.source_index >= self.gates.len() {
                return Err(LtTreeError::InvalidGateIndex {
                    gate_index: key.source_index,
                });
            }
            if key.sink_index > self.fanout_data.sinks(key.sink_polarity).len() {
                return Err(LtTreeError::InvalidSinkIndex {
                    polarity: key.sink_polarity,
                    index: key.sink_index,
                });
            }
            if candidate.gate_index >= self.gates.len() {
                return Err(LtTreeError::InvalidGateIndex {
                    gate_index: candidate.gate_index,
                });
            }
            if candidate.gate_count == 0 {
                return Err(LtTreeError::InvalidTwoLevelGateCount { key: *key });
            }
            validate_delay(candidate.required, "two-level required")?;
            if !candidate.area.is_finite() || candidate.area < 0.0 {
                return Err(LtTreeError::InvalidArea {
                    gate_index: candidate.gate_index,
                });
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LtTreePlan {
    pub input: LtTreeInput,
    pub table: HashMap<LtTreeKey, LtTreeEntry>,
}

impl LtTreePlan {
    pub fn entry(&self, key: LtTreeKey) -> Option<&LtTreeEntry> {
        self.table.get(&key)
    }

    pub fn merge_info(&self) -> Result<Vec<SingleSourceMerge>, LtTreeError> {
        let mut result = Vec::new();
        let gaps_remaining = self.input.options.max_gaps - 1;

        for source_index in 0..self.input.gates.len() {
            for sink_polarity in Polarity::all() {
                for source_polarity in Polarity::all() {
                    if self.input.fanout_data.sinks(sink_polarity).is_empty() {
                        result.push(SingleSourceMerge {
                            source_index,
                            source_polarity,
                            sink_polarity,
                            required: PLUS_INFINITY,
                            area: 0.0,
                            load: 0.0,
                            fanout_count: 0,
                        });
                        continue;
                    }

                    let gate = &self.input.gates[source_index];
                    let key = LtTreeKey {
                        source_index,
                        source_polarity,
                        sink_index: 0,
                        sink_polarity,
                        gaps_remaining,
                    };
                    let Some(from) = self.entry(key) else {
                        continue;
                    };

                    if gate.kind.is_buffer() {
                        result.push(SingleSourceMerge {
                            source_index,
                            source_polarity,
                            sink_polarity,
                            required: backward_intrinsic(from.required, gate),
                            area: from.area,
                            load: gate.input_load,
                            fanout_count: 1,
                        });
                    } else if gate.kind.source_polarity() == Some(source_polarity) {
                        let (load, fanout_count) =
                            self.root_load_and_fanout(from, sink_polarity)?;
                        let origin = backward_load_dependent(
                            ZERO_DELAY,
                            gate,
                            load + self.input.options.wire_load.load_for_fanout_count(fanout_count),
                        );
                        result.push(SingleSourceMerge {
                            source_index,
                            source_polarity,
                            sink_polarity,
                            required: subtract_delay(from.required, origin),
                            area: from.area,
                            load,
                            fanout_count,
                        });
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn build_tree(&self, source: SelectedSource) -> Result<Vec<FanoutTreeAction>, LtTreeError> {
        let source_gate = self
            .input
            .gates
            .get(source.main_source)
            .ok_or(LtTreeError::InvalidGateIndex {
                gate_index: source.main_source,
            })?;
        let source_polarity =
            source_gate
                .kind
                .source_polarity()
                .ok_or(LtTreeError::SourcePolarityRequired {
                    source_index: source.main_source,
                })?;
        let primary_sink_polarity = source.main_source_sink_polarity;
        let secondary_sink_polarity = primary_sink_polarity.inverted();
        let gaps_remaining = self.input.options.max_gaps - 1;
        let mut tree = Vec::new();

        if !self
            .input
            .fanout_data
            .sinks(secondary_sink_polarity)
            .is_empty()
        {
            if self.input.gates[source.buffer].kind.is_source() {
                if source.buffer != source.main_source {
                    return Err(LtTreeError::InvalidSelectedSource);
                }
                let primary_count = self.fanout_count(
                    source.main_source,
                    source_polarity,
                    0,
                    primary_sink_polarity,
                    gaps_remaining,
                )?;
                let secondary_count = self.fanout_count(
                    source.main_source,
                    source_polarity,
                    0,
                    secondary_sink_polarity,
                    gaps_remaining,
                )?;
                tree.push(FanoutTreeAction::Gate {
                    gate_index: source.main_source,
                    fanout_count: primary_count + secondary_count,
                });
                self.insert_lt_tree(
                    &mut tree,
                    source.main_source,
                    source_polarity,
                    0,
                    primary_sink_polarity,
                    gaps_remaining,
                )?;
                self.insert_lt_tree(
                    &mut tree,
                    source.main_source,
                    source_polarity,
                    0,
                    secondary_sink_polarity,
                    gaps_remaining,
                )?;
            } else {
                let primary_count = self.fanout_count(
                    source.main_source,
                    source_polarity,
                    0,
                    primary_sink_polarity,
                    gaps_remaining,
                )?;
                tree.push(FanoutTreeAction::Gate {
                    gate_index: source.main_source,
                    fanout_count: primary_count + 1,
                });
                self.insert_lt_tree(
                    &mut tree,
                    source.main_source,
                    source_polarity,
                    0,
                    primary_sink_polarity,
                    gaps_remaining,
                )?;

                let buffer_polarity = self.input.gates[source.buffer]
                    .kind
                    .output_polarity(source_polarity);
                let secondary_count = self.fanout_count(
                    source.buffer,
                    buffer_polarity,
                    0,
                    secondary_sink_polarity,
                    gaps_remaining,
                )?;
                tree.push(FanoutTreeAction::Gate {
                    gate_index: source.buffer,
                    fanout_count: secondary_count,
                });
                self.insert_lt_tree(
                    &mut tree,
                    source.buffer,
                    buffer_polarity,
                    0,
                    secondary_sink_polarity,
                    gaps_remaining,
                )?;
            }
        } else {
            let primary_count = self.fanout_count(
                source.main_source,
                source_polarity,
                0,
                primary_sink_polarity,
                gaps_remaining,
            )?;
            tree.push(FanoutTreeAction::Gate {
                gate_index: source.main_source,
                fanout_count: primary_count,
            });
            self.insert_lt_tree(
                &mut tree,
                source.main_source,
                source_polarity,
                0,
                primary_sink_polarity,
                gaps_remaining,
            )?;
        }

        Ok(tree)
    }

    fn root_load_and_fanout(
        &self,
        entry: &LtTreeEntry,
        sink_polarity: Polarity,
    ) -> Result<(f64, usize), LtTreeError> {
        match &entry.choice {
            LtTreeChoice::TwoLevel(candidate) => Ok((
                self.input.gates[candidate.gate_index].input_load * candidate.gate_count as f64,
                candidate.gate_count,
            )),
            LtTreeChoice::Split {
                buffer_index,
                sink_index,
            } => {
                let sinks = self.input.fanout_data.sinks(sink_polarity);
                if *sink_index < sinks.len() {
                    Ok((
                        self.input.fanout_data.cumulative_load(sink_polarity, *sink_index)?
                            + self.input.gates[*buffer_index].input_load,
                        *sink_index + 1,
                    ))
                } else {
                    Ok((
                        self.input.fanout_data.cumulative_load(sink_polarity, sinks.len())?,
                        sinks.len(),
                    ))
                }
            }
        }
    }

    fn insert_lt_tree(
        &self,
        tree: &mut Vec<FanoutTreeAction>,
        source_index: usize,
        source_polarity: Polarity,
        sink_index: usize,
        sink_polarity: Polarity,
        gaps_remaining: usize,
    ) -> Result<(), LtTreeError> {
        let sink_count = self.input.fanout_data.sinks(sink_polarity).len();
        if sink_index == sink_count {
            return Ok(());
        }

        let key = LtTreeKey {
            source_index,
            source_polarity,
            sink_index,
            sink_polarity,
            gaps_remaining,
        };
        let entry = self
            .entry(key)
            .ok_or(LtTreeError::MissingLtTreeEntry { key })?;
        match entry.choice {
            LtTreeChoice::Split {
                buffer_index,
                sink_index: next_sink_index,
            } => {
                self.insert_sinks(tree, sink_polarity, sink_index, next_sink_index)?;
                if next_sink_index == sink_count {
                    return Ok(());
                }
                let buffer_polarity = self.input.gates[buffer_index]
                    .kind
                    .output_polarity(source_polarity);
                let buffer_gaps = if next_sink_index == sink_index
                    && buffer_polarity == sink_polarity
                {
                    gaps_remaining.checked_sub(1).ok_or(LtTreeError::GapUnderflow)?
                } else {
                    gaps_remaining
                };
                let fanout_count = self.fanout_count(
                    buffer_index,
                    buffer_polarity,
                    next_sink_index,
                    sink_polarity,
                    buffer_gaps,
                )?;
                tree.push(FanoutTreeAction::Gate {
                    gate_index: buffer_index,
                    fanout_count,
                });
                self.insert_lt_tree(
                    tree,
                    buffer_index,
                    buffer_polarity,
                    next_sink_index,
                    sink_polarity,
                    buffer_gaps,
                )?;
            }
            LtTreeChoice::TwoLevel(candidate) => {
                tree.push(FanoutTreeAction::TwoLevel {
                    source_index,
                    sink_polarity,
                    sink_start: sink_index,
                    candidate,
                });
            }
        }

        Ok(())
    }

    fn insert_sinks(
        &self,
        tree: &mut Vec<FanoutTreeAction>,
        polarity: Polarity,
        start: usize,
        end: usize,
    ) -> Result<(), LtTreeError> {
        let sinks = self.input.fanout_data.sinks(polarity);
        if start > end || end > sinks.len() {
            return Err(LtTreeError::InvalidSinkRange {
                polarity,
                start,
                end,
            });
        }
        for sink in &sinks.sinks[start..end] {
            tree.push(FanoutTreeAction::Sink {
                polarity,
                sink_name: sink.name.clone(),
            });
        }

        Ok(())
    }

    fn fanout_count(
        &self,
        source_index: usize,
        source_polarity: Polarity,
        sink_index: usize,
        sink_polarity: Polarity,
        gaps_remaining: usize,
    ) -> Result<usize, LtTreeError> {
        let sink_count = self.input.fanout_data.sinks(sink_polarity).len();
        if sink_count == sink_index {
            return Ok(0);
        }

        let key = LtTreeKey {
            source_index,
            source_polarity,
            sink_index,
            sink_polarity,
            gaps_remaining,
        };
        let entry = self
            .entry(key)
            .ok_or(LtTreeError::MissingLtTreeEntry { key })?;
        match entry.choice {
            LtTreeChoice::TwoLevel(candidate) => Ok(candidate.gate_count),
            LtTreeChoice::Split {
                sink_index: next_sink_index,
                ..
            } => {
                if sink_count == next_sink_index {
                    Ok(next_sink_index - sink_index)
                } else {
                    Ok(next_sink_index - sink_index + 1)
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SingleSourceMerge {
    pub source_index: usize,
    pub source_polarity: Polarity,
    pub sink_polarity: Polarity,
    pub required: DelayTime,
    pub area: f64,
    pub load: f64,
    pub fanout_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectedSource {
    pub main_source: usize,
    pub buffer: usize,
    pub main_source_sink_polarity: Polarity,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FanoutTreeAction {
    Gate {
        gate_index: usize,
        fanout_count: usize,
    },
    Sink {
        polarity: Polarity,
        sink_name: String,
    },
    TwoLevel {
        source_index: usize,
        sink_polarity: Polarity,
        sink_start: usize,
        candidate: TwoLevelCandidate,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum LtTreeError {
    MissingSisPorts {
        operation: &'static str,
    },
    NoGates,
    NoSources,
    NoBuffers,
    InvalidMaxGaps,
    InvalidWireLoad,
    InvalidGateIndex {
        gate_index: usize,
    },
    InvalidArea {
        gate_index: usize,
    },
    InvalidInputLoad {
        gate_index: usize,
    },
    InvalidDelay {
        context: &'static str,
    },
    InvalidSinkLoad {
        polarity: Polarity,
        index: usize,
    },
    InvalidSinkIndex {
        polarity: Polarity,
        index: usize,
    },
    InvalidSinkRange {
        polarity: Polarity,
        start: usize,
        end: usize,
    },
    InvalidTwoLevelGateCount {
        key: TwoLevelKey,
    },
    MissingTwoLevelCandidate {
        key: TwoLevelKey,
    },
    MissingLtTreeEntry {
        key: LtTreeKey,
    },
    SourcePolarityRequired {
        source_index: usize,
    },
    InvalidSelectedSource,
    GapUnderflow,
}

impl fmt::Display for LtTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts { operation } => write!(f, "{operation} requires unavailable native SIS integration"),
            Self::NoGates => write!(f, "LT-tree planning requires at least one gate"),
            Self::NoSources => write!(f, "LT-tree planning requires at least one source gate"),
            Self::NoBuffers => write!(f, "LT-tree planning requires at least one buffer gate"),
            Self::InvalidMaxGaps => write!(f, "max_gaps must be greater than zero"),
            Self::InvalidWireLoad => write!(f, "wire load per fanout must be finite and non-negative"),
            Self::InvalidGateIndex { gate_index } => write!(f, "invalid gate index {gate_index}"),
            Self::InvalidArea { gate_index } => {
                write!(f, "gate {gate_index} has invalid area")
            }
            Self::InvalidInputLoad { gate_index } => {
                write!(f, "gate {gate_index} has invalid input load")
            }
            Self::InvalidDelay { context } => write!(f, "{context} has invalid delay values"),
            Self::InvalidSinkLoad { polarity, index } => {
                write!(f, "{polarity:?} sink {index} has invalid load")
            }
            Self::InvalidSinkIndex { polarity, index } => {
                write!(f, "invalid {polarity:?} sink index {index}")
            }
            Self::InvalidSinkRange {
                polarity,
                start,
                end,
            } => write!(f, "invalid {polarity:?} sink range {start}..{end}"),
            Self::InvalidTwoLevelGateCount { key } => {
                write!(f, "two-level candidate {key:?} must use at least one gate")
            }
            Self::MissingTwoLevelCandidate { key } => {
                write!(f, "missing two-level candidate for {key:?}")
            }
            Self::MissingLtTreeEntry { key } => write!(f, "missing LT-tree entry for {key:?}"),
            Self::SourcePolarityRequired { source_index } => {
                write!(f, "selected main source {source_index} is not a source gate")
            }
            Self::InvalidSelectedSource => write!(f, "selected source and buffer are inconsistent"),
            Self::GapUnderflow => write!(f, "LT-tree gap counter underflowed"),
        }
    }
}

impl Error for LtTreeError {}

pub fn plan_from_native_network_unavailable() -> Result<LtTreePlan, LtTreeError> {
    Err(LtTreeError::MissingSisPorts {
        operation: "LT-tree planning from native SIS network",
    })
}

pub fn optimize_lt_trees(input: LtTreeInput) -> Result<LtTreePlan, LtTreeError> {
    input.validate()?;

    let max_sinks = Polarity::all()
        .into_iter()
        .map(|polarity| input.fanout_data.sinks(polarity).len())
        .max()
        .unwrap_or(0);
    let mut table = HashMap::new();

    for sink_polarity in Polarity::all() {
        let sink_count = input.fanout_data.sinks(sink_polarity).len();
        if sink_count == 0 {
            continue;
        }
        for sink_index in (0..sink_count).rev() {
            for gaps_remaining in 0..input.options.max_gaps {
                for source_index in 0..input.gates.len() {
                    for source_polarity in Polarity::all() {
                        let source_gate = &input.gates[source_index];
                        if source_gate.kind.source_polarity().is_some()
                            && source_gate.kind.source_polarity() != Some(source_polarity)
                        {
                            continue;
                        }

                        let key = LtTreeKey {
                            source_index,
                            source_polarity,
                            sink_index,
                            sink_polarity,
                            gaps_remaining,
                        };
                        let entry = best_one_source(&input, max_sinks, key, &table)?;
                        table.insert(key, entry);
                    }
                }
            }
        }
    }

    Ok(LtTreePlan { input, table })
}

fn best_one_source(
    input: &LtTreeInput,
    _max_sinks: usize,
    key: LtTreeKey,
    table: &HashMap<LtTreeKey, LtTreeEntry>,
) -> Result<LtTreeEntry, LtTreeError> {
    let two_level_key = TwoLevelKey {
        source_index: key.source_index,
        source_polarity: key.source_polarity,
        sink_index: key.sink_index,
        sink_polarity: key.sink_polarity,
    };
    let two_level = input
        .two_level
        .get(two_level_key)
        .ok_or(LtTreeError::MissingTwoLevelCandidate { key: two_level_key })?
        .clone();
    let mut best = LtTreeEntry {
        choice: LtTreeChoice::TwoLevel(two_level.clone()),
        required: two_level.required,
        area: two_level.area,
    };
    let sink_count = input.fanout_data.sinks(key.sink_polarity).len();

    for sink in key.sink_index..=sink_count {
        if key.source_polarity != key.sink_polarity && sink > key.sink_index {
            continue;
        }
        for buffer_index in buffer_indices(&input.gates) {
            let source_gate = &input.gates[key.source_index];
            let buffer_gate = &input.gates[buffer_index];
            let buffer_polarity = buffer_gate.kind.output_polarity(key.source_polarity);
            let Some(buffer_gaps) = buffer_gaps_for_split(
                sink,
                key.sink_index,
                buffer_polarity,
                key.sink_polarity,
                key.gaps_remaining,
            ) else {
                continue;
            };

            let (mut local_area, mut fanout_count, mut load, mut local_required) =
                if sink < sink_count {
                    let buffer_key = LtTreeKey {
                        source_index: buffer_index,
                        source_polarity: buffer_polarity,
                        sink_index: sink,
                        sink_polarity: key.sink_polarity,
                        gaps_remaining: buffer_gaps,
                    };
                    let Some(buffer_entry) = table.get(&buffer_key) else {
                        continue;
                    };
                    (
                        buffer_entry.area,
                        1,
                        buffer_gate.input_load,
                        backward_intrinsic(buffer_entry.required, buffer_gate),
                    )
                } else {
                    (0.0, 0, 0.0, PLUS_INFINITY)
                };

            local_area += source_gate.area;
            fanout_count += sink - key.sink_index;
            load += input
                .fanout_data
                .interval_load(key.sink_polarity, key.sink_index, sink)?;
            load += input.options.wire_load.load_for_fanout_count(fanout_count);
            if sink > key.sink_index {
                local_required = min_delay(
                    local_required,
                    input.fanout_data.sinks(key.sink_polarity).sinks[key.sink_index]
                        .min_required,
                );
            }
            local_required = backward_load_dependent(local_required, source_gate, load);

            if min_component(best.required) < min_component(local_required) {
                best = LtTreeEntry {
                    choice: LtTreeChoice::Split {
                        buffer_index,
                        sink_index: sink,
                    },
                    required: local_required,
                    area: local_area,
                };
            }
        }
    }

    Ok(best)
}

fn buffer_indices(gates: &[FanoutGate]) -> impl Iterator<Item = usize> + '_ {
    gates
        .iter()
        .enumerate()
        .filter(|(_, gate)| gate.kind.is_buffer())
        .map(|(index, _)| index)
}

fn buffer_gaps_for_split(
    sink: usize,
    sink_index: usize,
    buffer_polarity: Polarity,
    sink_polarity: Polarity,
    gaps_remaining: usize,
) -> Option<usize> {
    if sink == sink_index && buffer_polarity == sink_polarity {
        gaps_remaining.checked_sub(1)
    } else {
        Some(gaps_remaining)
    }
}

fn backward_intrinsic(required: DelayTime, gate: &FanoutGate) -> DelayTime {
    DelayTime::new(
        required.rise - gate.timing.intrinsic.rise,
        required.fall - gate.timing.intrinsic.fall,
    )
}

fn backward_load_dependent(required: DelayTime, gate: &FanoutGate, load: f64) -> DelayTime {
    DelayTime::new(
        required.rise - gate.timing.intrinsic.rise - gate.timing.load_slope.rise * load,
        required.fall - gate.timing.intrinsic.fall - gate.timing.load_slope.fall * load,
    )
}

fn subtract_delay(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise - right.rise, left.fall - right.fall)
}

fn min_delay(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise.min(right.rise), left.fall.min(right.fall))
}

fn min_component(value: DelayTime) -> f64 {
    value.rise.min(value.fall)
}

fn validate_gate(index: usize, gate: &FanoutGate) -> Result<(), LtTreeError> {
    if !gate.area.is_finite() || gate.area < 0.0 {
        return Err(LtTreeError::InvalidArea { gate_index: index });
    }
    if !gate.input_load.is_finite() || gate.input_load < 0.0 {
        return Err(LtTreeError::InvalidInputLoad { gate_index: index });
    }
    validate_delay(gate.timing.intrinsic, "gate intrinsic")?;
    validate_delay(gate.timing.load_slope, "gate load slope")?;
    if gate.timing.load_slope.rise < 0.0 || gate.timing.load_slope.fall < 0.0 {
        return Err(LtTreeError::InvalidDelay {
            context: "gate load slope",
        });
    }

    Ok(())
}

fn validate_sink(
    polarity: Polarity,
    index: usize,
    sink: &FanoutSink,
) -> Result<(), LtTreeError> {
    if !sink.load.is_finite() || sink.load < 0.0 {
        return Err(LtTreeError::InvalidSinkLoad { polarity, index });
    }
    validate_delay(sink.min_required, "sink required")
}

fn validate_delay(value: DelayTime, context: &'static str) -> Result<(), LtTreeError> {
    if value.rise.is_nan() || value.fall.is_nan() {
        return Err(LtTreeError::InvalidDelay { context });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_two_level_candidate_as_initial_solution() {
        let input = sample_input(DelayTime::new(10.0, 10.0), DelayTime::new(0.0, 0.0), 5.0);
        let plan = optimize_lt_trees(input).unwrap();
        let entry = plan
            .entry(LtTreeKey {
                source_index: 0,
                source_polarity: Polarity::Positive,
                sink_index: 0,
                sink_polarity: Polarity::Positive,
                gaps_remaining: 0,
            })
            .unwrap();

        assert!(matches!(entry.choice, LtTreeChoice::TwoLevel(_)));
        assert_eq!(entry.required, DelayTime::new(10.0, 10.0));
        assert_eq!(entry.area, 5.0);
    }

    #[test]
    fn split_can_beat_two_level_candidate() {
        let input = sample_input(DelayTime::new(1.0, 1.0), DelayTime::new(0.0, 0.0), 20.0);
        let plan = optimize_lt_trees(input).unwrap();
        let entry = plan
            .entry(LtTreeKey {
                source_index: 0,
                source_polarity: Polarity::Positive,
                sink_index: 0,
                sink_polarity: Polarity::Positive,
                gaps_remaining: 0,
            })
            .unwrap();

        assert_eq!(
            entry.choice,
            LtTreeChoice::Split {
                buffer_index: 1,
                sink_index: 1
            }
        );
        assert_eq!(entry.required, DelayTime::new(7.5, 7.5));
        assert_eq!(entry.area, 2.0);
    }

    #[test]
    fn same_polarity_zero_sink_split_consumes_gap() {
        let input = sample_negative_gap_input();

        let plan = optimize_lt_trees(input).unwrap();
        let without_gap = plan.entry(LtTreeKey {
            source_index: 0,
            source_polarity: Polarity::Positive,
            sink_index: 0,
            sink_polarity: Polarity::Negative,
            gaps_remaining: 0,
        });
        let with_gap = plan
            .entry(LtTreeKey {
                source_index: 0,
                source_polarity: Polarity::Positive,
                sink_index: 0,
                sink_polarity: Polarity::Negative,
                gaps_remaining: 1,
            })
            .unwrap();

        assert!(matches!(
            without_gap.unwrap().choice,
            LtTreeChoice::TwoLevel(_)
        ));
        assert_eq!(
            with_gap.choice,
            LtTreeChoice::Split {
                buffer_index: 1,
                sink_index: 0
            }
        );
    }

    #[test]
    fn builds_tree_actions_for_split_plan() {
        let input = sample_input(DelayTime::new(1.0, 1.0), DelayTime::new(0.0, 0.0), 20.0);
        let plan = optimize_lt_trees(input).unwrap();
        let actions = plan
            .build_tree(SelectedSource {
                main_source: 0,
                buffer: 0,
                main_source_sink_polarity: Polarity::Positive,
            })
            .unwrap();

        assert_eq!(
            actions,
            vec![
                FanoutTreeAction::Gate {
                    gate_index: 0,
                    fanout_count: 2
                },
                FanoutTreeAction::Sink {
                    polarity: Polarity::Positive,
                    sink_name: "a".to_string()
                },
                FanoutTreeAction::Gate {
                    gate_index: 1,
                    fanout_count: 1
                },
                FanoutTreeAction::TwoLevel {
                    source_index: 1,
                    sink_polarity: Polarity::Positive,
                    sink_start: 1,
                    candidate: TwoLevelCandidate {
                        gate_index: 1,
                        gate_count: 1,
                        required: DelayTime::new(8.0, 8.0),
                        area: 1.0
                    }
                }
            ]
        );
    }

    #[test]
    fn merge_info_removes_source_drive_at_root() {
        let input = sample_input(DelayTime::new(1.0, 1.0), DelayTime::new(0.0, 0.0), 20.0);
        let plan = optimize_lt_trees(input).unwrap();
        let merges = plan.merge_info().unwrap();
        let merge = merges
            .iter()
            .find(|merge| {
                merge.source_index == 0
                    && merge.source_polarity == Polarity::Positive
                    && merge.sink_polarity == Polarity::Positive
            })
            .unwrap();

        assert_eq!(merge.load, 2.0);
        assert_eq!(merge.fanout_count, 2);
        assert_eq!(merge.required, DelayTime::new(8.0, 8.0));
    }

    fn sample_input(
        root_two_level_required: DelayTime,
        source_intrinsic: DelayTime,
        root_two_level_area: f64,
    ) -> LtTreeInput {
        let gates = vec![
            FanoutGate::new(
                "source",
                GateKind::Source {
                    polarity: Polarity::Positive,
                },
                1.0,
                0.0,
                GateTiming::new(source_intrinsic, DelayTime::new(0.25, 0.25)),
            ),
            FanoutGate::new(
                "buffer",
                GateKind::Buffer,
                1.0,
                1.0,
                GateTiming::new(DelayTime::new(0.0, 0.0), DelayTime::new(0.0, 0.0)),
            ),
        ];
        let fanout_data = FanoutData::new(
            vec![
                FanoutSink::new("a", 1.0, DelayTime::new(8.0, 8.0)),
                FanoutSink::new("b", 1.0, DelayTime::new(8.0, 8.0)),
            ],
            Vec::new(),
        );
        let mut two_level = TwoLevelTable::new();
        for source_index in 0..gates.len() {
            for source_polarity in Polarity::all() {
                if gates[source_index].kind.source_polarity().is_some()
                    && gates[source_index].kind.source_polarity() != Some(source_polarity)
                {
                    continue;
                }
                for sink_index in 0..fanout_data.sinks(Polarity::Positive).len() {
                    two_level.insert(
                        TwoLevelKey {
                            source_index,
                            source_polarity,
                            sink_index,
                            sink_polarity: Polarity::Positive,
                        },
                        TwoLevelCandidate::new(
                            1,
                            1,
                            if source_index == 0 && sink_index == 0 {
                                root_two_level_required
                            } else {
                                DelayTime::new(8.0, 8.0)
                            },
                            if source_index == 0 && sink_index == 0 {
                                root_two_level_area
                            } else {
                                1.0
                            },
                        ),
                    );
                }
            }
        }

        LtTreeInput::new(
            gates,
            fanout_data,
            two_level,
            LtTreeOptions {
                max_gaps: 1,
                wire_load: WireLoadModel::default(),
            },
        )
        .unwrap()
    }

    fn sample_negative_gap_input() -> LtTreeInput {
        let gates = vec![
            FanoutGate::new(
                "source",
                GateKind::Source {
                    polarity: Polarity::Positive,
                },
                1.0,
                0.0,
                GateTiming::new(DelayTime::new(0.0, 0.0), DelayTime::new(0.0, 0.0)),
            ),
            FanoutGate::new(
                "inverter",
                GateKind::Inverter,
                1.0,
                1.0,
                GateTiming::new(DelayTime::new(0.0, 0.0), DelayTime::new(0.0, 0.0)),
            ),
        ];
        let fanout_data = FanoutData::new(
            Vec::new(),
            vec![FanoutSink::new("n", 1.0, DelayTime::new(8.0, 8.0))],
        );
        let mut two_level = TwoLevelTable::new();
        for source_index in 0..gates.len() {
            for source_polarity in Polarity::all() {
                if gates[source_index].kind.source_polarity().is_some()
                    && gates[source_index].kind.source_polarity() != Some(source_polarity)
                {
                    continue;
                }
                two_level.insert(
                    TwoLevelKey {
                        source_index,
                        source_polarity,
                        sink_index: 0,
                        sink_polarity: Polarity::Negative,
                    },
                    TwoLevelCandidate::new(
                        1,
                        1,
                        if source_index == 1 && source_polarity == Polarity::Negative {
                            DelayTime::new(9.0, 9.0)
                        } else {
                            DelayTime::new(0.0, 0.0)
                        },
                        1.0,
                    ),
                );
            }
        }

        LtTreeInput::new(
            gates,
            fanout_data,
            two_level,
            LtTreeOptions {
                max_gaps: 2,
                wire_load: WireLoadModel::default(),
            },
        )
        .unwrap()
    }
}
