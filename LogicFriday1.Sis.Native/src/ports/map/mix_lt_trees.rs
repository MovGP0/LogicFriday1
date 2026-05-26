//! Native Rust mixed-polarity load-threshold fanout-tree planning for
//! `sis/map/mix_lt_trees.c`.
//!
//! The C implementation combines positive and negative sink lists in one
//! dynamic-programming table, using two-level trees when only one polarity
//! remains and ordinary LT-tree splits otherwise. This port keeps that behavior
//! over owned Rust data and deliberately avoids legacy per-file C ABI exports.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use super::lt_trees::{
    FanoutData, FanoutGate, FanoutTreeAction, GateKind, LtTreeOptions, MINUS_INFINITY,
    PLUS_INFINITY, Polarity, TwoLevelCandidate, TwoLevelKey, TwoLevelTable, WireLoadModel,
};
use super::virtual_net::DelayTime;

#[derive(Clone, Debug, PartialEq)]
pub struct MixedLtTreeOptions {
    pub max_gaps: usize,
    pub max_x_index: usize,
    pub max_y_index: usize,
    pub wire_load: WireLoadModel,
}

impl Default for MixedLtTreeOptions {
    fn default() -> Self {
        Self {
            max_gaps: 5,
            max_x_index: 15,
            max_y_index: 15,
            wire_load: WireLoadModel::default(),
        }
    }
}

impl From<LtTreeOptions> for MixedLtTreeOptions {
    fn from(value: LtTreeOptions) -> Self {
        Self {
            max_gaps: value.max_gaps,
            max_x_index: Self::default().max_x_index,
            max_y_index: Self::default().max_y_index,
            wire_load: value.wire_load,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MixedLtTreeInput {
    pub gates: Vec<FanoutGate>,
    pub fanout_data: FanoutData,
    pub two_level: TwoLevelTable,
    pub options: MixedLtTreeOptions,
}

impl MixedLtTreeInput {
    pub fn new(
        gates: Vec<FanoutGate>,
        fanout_data: FanoutData,
        two_level: TwoLevelTable,
        options: MixedLtTreeOptions,
    ) -> Result<Self, MixedLtTreeError> {
        let input = Self {
            gates,
            fanout_data,
            two_level,
            options,
        };
        input.validate()?;
        Ok(input)
    }

    pub fn validate(&self) -> Result<(), MixedLtTreeError> {
        if self.gates.is_empty() {
            return Err(MixedLtTreeError::NoGates);
        }
        if self.options.max_gaps == 0 {
            return Err(MixedLtTreeError::InvalidMaxGaps);
        }
        if self.options.max_x_index == 0 || self.options.max_y_index == 0 {
            return Err(MixedLtTreeError::InvalidSinkLimit);
        }
        if !self.options.wire_load.per_fanout.is_finite() || self.options.wire_load.per_fanout < 0.0
        {
            return Err(MixedLtTreeError::InvalidWireLoad);
        }
        if !self.gates.iter().any(|gate| is_source(gate.kind)) {
            return Err(MixedLtTreeError::NoSources);
        }
        if !self.gates.iter().any(|gate| is_buffer(gate.kind)) {
            return Err(MixedLtTreeError::NoBuffers);
        }
        for (index, gate) in self.gates.iter().enumerate() {
            validate_gate(index, gate)?;
        }
        for polarity in all_polarities() {
            for (index, sink) in self.fanout_data.sinks(polarity).sinks.iter().enumerate() {
                if !sink.load.is_finite() || sink.load < 0.0 {
                    return Err(MixedLtTreeError::InvalidSinkLoad { polarity, index });
                }
                validate_delay(sink.min_required, "sink required")?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct MixedLtTreeKey {
    pub source_index: usize,
    pub source_polarity: Polarity,
    pub x_index: usize,
    pub y_index: usize,
    pub gaps_remaining: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MixedLtTreeChoice {
    Done,
    TwoLevel {
        sink_polarity: Polarity,
        candidate: TwoLevelCandidate,
    },
    Split {
        buffer_index: usize,
        next_x_index: usize,
        next_y_index: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct MixedLtTreeEntry {
    pub choice: MixedLtTreeChoice,
    pub required: DelayTime,
    pub area: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixedLtTreeCost {
    pub slack: DelayTime,
    pub area: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectedMixedSource {
    pub main_source: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MixedLtTreePlan {
    pub input: MixedLtTreeInput,
    pub table: HashMap<MixedLtTreeKey, MixedLtTreeEntry>,
}

impl MixedLtTreePlan {
    pub fn entry(&self, key: MixedLtTreeKey) -> Option<&MixedLtTreeEntry> {
        self.table.get(&key)
    }

    pub fn select_best_source(
        &self,
    ) -> Result<(SelectedMixedSource, MixedLtTreeCost), MixedLtTreeError> {
        let mut selected = None;
        let mut cost = MixedLtTreeCost {
            slack: MINUS_INFINITY,
            area: f64::INFINITY,
        };
        let gaps_remaining = self.input.options.max_gaps - 1;

        for (source_index, gate) in self.input.gates.iter().enumerate() {
            let Some(source_polarity) = source_polarity(gate.kind) else {
                continue;
            };
            let key = MixedLtTreeKey {
                source_index,
                source_polarity,
                x_index: 0,
                y_index: 0,
                gaps_remaining,
            };
            let entry = self
                .entry(key)
                .ok_or(MixedLtTreeError::MissingMixedLtTreeEntry { key })?;
            if min_component(cost.slack) < min_component(entry.required) {
                selected = Some(SelectedMixedSource {
                    main_source: source_index,
                });
                cost = MixedLtTreeCost {
                    slack: entry.required,
                    area: entry.area,
                };
            }
        }

        let selected = selected.ok_or(MixedLtTreeError::NoUsableSource)?;
        if min_component(cost.slack) == f64::NEG_INFINITY {
            return Err(MixedLtTreeError::NoUsableSource);
        }

        Ok((selected, cost))
    }

    pub fn build_tree(
        &self,
        source: SelectedMixedSource,
    ) -> Result<Vec<FanoutTreeAction>, MixedLtTreeError> {
        let source_gate =
            self.input
                .gates
                .get(source.main_source)
                .ok_or(MixedLtTreeError::InvalidGateIndex {
                    gate_index: source.main_source,
                })?;
        let source_polarity =
            source_polarity(source_gate.kind).ok_or(MixedLtTreeError::SourcePolarityRequired {
                source_index: source.main_source,
            })?;
        let gaps_remaining = self.input.options.max_gaps - 1;
        let mut tree = Vec::new();
        let n_fanouts =
            self.fanout_count(source.main_source, source_polarity, 0, 0, gaps_remaining)?;

        tree.push(FanoutTreeAction::Gate {
            gate_index: source.main_source,
            fanout_count: n_fanouts,
        });
        self.insert_mixed_lt_tree(
            &mut tree,
            source.main_source,
            source_polarity,
            0,
            0,
            gaps_remaining,
        )?;

        Ok(tree)
    }

    fn insert_mixed_lt_tree(
        &self,
        tree: &mut Vec<FanoutTreeAction>,
        source_index: usize,
        source_polarity: Polarity,
        x_index: usize,
        y_index: usize,
        gaps_remaining: usize,
    ) -> Result<(), MixedLtTreeError> {
        if self.is_terminal(x_index, y_index) {
            return Ok(());
        }

        let key = MixedLtTreeKey {
            source_index,
            source_polarity,
            x_index,
            y_index,
            gaps_remaining,
        };
        let entry = self
            .entry(key)
            .ok_or(MixedLtTreeError::MissingMixedLtTreeEntry { key })?;
        let p = source_polarity;
        let q = p.inverted();
        let p_sink = self.index_for_polarity(p, x_index, y_index);
        let q_sink = self.index_for_polarity(q, x_index, y_index);

        match entry.choice {
            MixedLtTreeChoice::Done => Ok(()),
            MixedLtTreeChoice::TwoLevel {
                sink_polarity,
                candidate,
            } => {
                let sink_start = self.index_for_polarity(sink_polarity, x_index, y_index);
                tree.push(FanoutTreeAction::TwoLevel {
                    source_index,
                    sink_polarity,
                    sink_start,
                    candidate,
                });
                Ok(())
            }
            MixedLtTreeChoice::Split {
                buffer_index,
                next_x_index,
                next_y_index,
            } => {
                let sink = self.index_for_polarity(p, next_x_index, next_y_index);
                if q_sink != self.index_for_polarity(q, next_x_index, next_y_index) {
                    return Err(MixedLtTreeError::InvalidSplit);
                }
                self.insert_sinks(tree, p, p_sink, sink)?;
                if self.is_terminal(next_x_index, next_y_index) {
                    return Ok(());
                }

                let buffer_polarity = output_polarity(self.input.gates[buffer_index].kind, p);
                let buffer_gaps = if sink == p_sink && sink < self.sink_count(p) {
                    gaps_remaining
                        .checked_sub(1)
                        .ok_or(MixedLtTreeError::GapUnderflow)?
                } else {
                    gaps_remaining
                };
                let n_fanouts = self.fanout_count(
                    buffer_index,
                    buffer_polarity,
                    next_x_index,
                    next_y_index,
                    buffer_gaps,
                )?;
                tree.push(FanoutTreeAction::Gate {
                    gate_index: buffer_index,
                    fanout_count: n_fanouts,
                });
                self.insert_mixed_lt_tree(
                    tree,
                    buffer_index,
                    buffer_polarity,
                    next_x_index,
                    next_y_index,
                    buffer_gaps,
                )
            }
        }
    }

    fn fanout_count(
        &self,
        source_index: usize,
        source_polarity: Polarity,
        x_index: usize,
        y_index: usize,
        gaps_remaining: usize,
    ) -> Result<usize, MixedLtTreeError> {
        if self.is_terminal(x_index, y_index) {
            return Ok(0);
        }

        let key = MixedLtTreeKey {
            source_index,
            source_polarity,
            x_index,
            y_index,
            gaps_remaining,
        };
        let entry = self
            .entry(key)
            .ok_or(MixedLtTreeError::MissingMixedLtTreeEntry { key })?;
        match entry.choice {
            MixedLtTreeChoice::Done => Ok(0),
            MixedLtTreeChoice::TwoLevel { candidate, .. } => Ok(candidate.gate_count),
            MixedLtTreeChoice::Split {
                next_x_index,
                next_y_index,
                ..
            } => {
                let p_sink = self.index_for_polarity(source_polarity, x_index, y_index);
                let q = source_polarity.inverted();
                let q_sink = self.index_for_polarity(q, x_index, y_index);
                let sink = self.index_for_polarity(source_polarity, next_x_index, next_y_index);
                if sink == self.sink_count(source_polarity) && q_sink == self.sink_count(q) {
                    Ok(sink - p_sink)
                } else {
                    Ok(sink - p_sink + 1)
                }
            }
        }
    }

    fn insert_sinks(
        &self,
        tree: &mut Vec<FanoutTreeAction>,
        polarity: Polarity,
        start: usize,
        end: usize,
    ) -> Result<(), MixedLtTreeError> {
        let sinks = self.input.fanout_data.sinks(polarity);
        if start > end || end > sinks.len() {
            return Err(MixedLtTreeError::InvalidSinkRange {
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

    fn is_terminal(&self, x_index: usize, y_index: usize) -> bool {
        x_index == self.sink_count(Polarity::Positive)
            && y_index == self.sink_count(Polarity::Negative)
    }

    fn sink_count(&self, polarity: Polarity) -> usize {
        self.input.fanout_data.sinks(polarity).len()
    }

    fn index_for_polarity(&self, polarity: Polarity, x_index: usize, y_index: usize) -> usize {
        match polarity {
            Polarity::Positive => x_index,
            Polarity::Negative => y_index,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MixedLtTreeError {
    MissingSisPorts {
        operation: &'static str,
    },
    NoGates,
    NoSources,
    NoBuffers,
    NoUsableSource,
    InvalidMaxGaps,
    InvalidSinkLimit,
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
    InvalidSinkRange {
        polarity: Polarity,
        start: usize,
        end: usize,
    },
    MissingTwoLevelCandidate {
        key: TwoLevelKey,
    },
    MissingMixedLtTreeEntry {
        key: MixedLtTreeKey,
    },
    SourcePolarityRequired {
        source_index: usize,
    },
    InvalidSplit,
    GapUnderflow,
}

impl fmt::Display for MixedLtTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
            Self::NoGates => write!(f, "mixed LT-tree planning requires at least one gate"),
            Self::NoSources => write!(
                f,
                "mixed LT-tree planning requires at least one source gate"
            ),
            Self::NoBuffers => write!(
                f,
                "mixed LT-tree planning requires at least one buffer gate"
            ),
            Self::NoUsableSource => write!(f, "mixed LT-tree planning found no usable source"),
            Self::InvalidMaxGaps => write!(f, "max_gaps must be greater than zero"),
            Self::InvalidSinkLimit => write!(f, "sink limits must be greater than zero"),
            Self::InvalidWireLoad => {
                write!(f, "wire load per fanout must be finite and non-negative")
            }
            Self::InvalidGateIndex { gate_index } => write!(f, "invalid gate index {gate_index}"),
            Self::InvalidArea { gate_index } => write!(f, "gate {gate_index} has invalid area"),
            Self::InvalidInputLoad { gate_index } => {
                write!(f, "gate {gate_index} has invalid input load")
            }
            Self::InvalidDelay { context } => write!(f, "{context} has invalid delay values"),
            Self::InvalidSinkLoad { polarity, index } => {
                write!(f, "{polarity:?} sink {index} has invalid load")
            }
            Self::InvalidSinkRange {
                polarity,
                start,
                end,
            } => write!(f, "invalid {polarity:?} sink range {start}..{end}"),
            Self::MissingTwoLevelCandidate { key } => {
                write!(
                    f,
                    "missing two-level candidate for mixed LT-tree key {key:?}"
                )
            }
            Self::MissingMixedLtTreeEntry { key } => {
                write!(f, "missing mixed LT-tree entry for {key:?}")
            }
            Self::SourcePolarityRequired { source_index } => {
                write!(
                    f,
                    "selected main source {source_index} is not a source gate"
                )
            }
            Self::InvalidSplit => {
                write!(f, "mixed LT-tree split changes the opposite sink frontier")
            }
            Self::GapUnderflow => write!(f, "mixed LT-tree gap counter underflowed"),
        }
    }
}

impl Error for MixedLtTreeError {}

pub fn plan_from_native_network_unavailable() -> Result<MixedLtTreePlan, MixedLtTreeError> {
    Err(MixedLtTreeError::MissingSisPorts {
        operation: "mixed LT-tree planning from native SIS network",
    })
}

pub fn optimize_mixed_lt_trees(
    input: MixedLtTreeInput,
) -> Result<Option<MixedLtTreePlan>, MixedLtTreeError> {
    input.validate()?;

    let x_count = input.fanout_data.sinks(Polarity::Positive).len();
    let y_count = input.fanout_data.sinks(Polarity::Negative).len();
    if x_count == 0 || y_count == 0 {
        return Ok(None);
    }
    if x_count > input.options.max_x_index && y_count > input.options.max_y_index {
        return Ok(None);
    }

    let mut table = HashMap::new();
    for x_index in (0..=x_count).rev() {
        for y_index in (0..=y_count).rev() {
            for source_index in 0..input.gates.len() {
                for gaps_remaining in 0..input.options.max_gaps {
                    for source_polarity_value in all_polarities() {
                        if source_polarity(input.gates[source_index].kind)
                            .is_some_and(|polarity| polarity != source_polarity_value)
                        {
                            continue;
                        }
                        let key = MixedLtTreeKey {
                            source_index,
                            source_polarity: source_polarity_value,
                            x_index,
                            y_index,
                            gaps_remaining,
                        };
                        let entry = best_one_source(&input, key, &table)?;
                        table.insert(key, entry);
                    }
                }
            }
        }
    }

    Ok(Some(MixedLtTreePlan { input, table }))
}

fn best_one_source(
    input: &MixedLtTreeInput,
    key: MixedLtTreeKey,
    table: &HashMap<MixedLtTreeKey, MixedLtTreeEntry>,
) -> Result<MixedLtTreeEntry, MixedLtTreeError> {
    let x_count = input.fanout_data.sinks(Polarity::Positive).len();
    let y_count = input.fanout_data.sinks(Polarity::Negative).len();
    if key.x_index == x_count && key.y_index == y_count {
        return Ok(MixedLtTreeEntry {
            choice: MixedLtTreeChoice::Done,
            required: PLUS_INFINITY,
            area: 0.0,
        });
    }

    let mut best = two_level_boundary_entry(input, key)?;
    let p = key.source_polarity;
    let q = p.inverted();
    let p_sink = index_for_polarity(p, key.x_index, key.y_index);
    let q_sink = index_for_polarity(q, key.x_index, key.y_index);

    for sink in p_sink..=sink_count(input, p) {
        for buffer_index in buffer_indices(&input.gates) {
            let source_gate = &input.gates[key.source_index];
            let buffer_gate = &input.gates[buffer_index];
            let buffer_polarity = output_polarity(buffer_gate.kind, p);
            let buffer_gaps = if sink == p_sink && sink < sink_count(input, p) {
                let Some(value) = key.gaps_remaining.checked_sub(1) else {
                    continue;
                };
                value
            } else {
                key.gaps_remaining
            };
            let next_x = if p == Polarity::Positive {
                sink
            } else {
                q_sink
            };
            let next_y = if p == Polarity::Negative {
                sink
            } else {
                q_sink
            };

            let (mut local_area, mut fanout_count, mut load, mut local_required) =
                if sink == sink_count(input, p) && q_sink == sink_count(input, q) {
                    (0.0, 0, 0.0, PLUS_INFINITY)
                } else {
                    let buffer_key = MixedLtTreeKey {
                        source_index: buffer_index,
                        source_polarity: buffer_polarity,
                        x_index: next_x,
                        y_index: next_y,
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
                };

            local_area += source_gate.area;
            fanout_count += sink - p_sink;
            load += interval_load(input, p, p_sink, sink)?;
            load += input.options.wire_load.load_for_fanout_count(fanout_count);
            if sink > p_sink {
                local_required = min_delay(
                    local_required,
                    input.fanout_data.sinks(p).sinks[p_sink].min_required,
                );
            }
            local_required = backward_load_dependent(local_required, source_gate, load);

            if best
                .as_ref()
                .is_none_or(|entry| min_component(entry.required) < min_component(local_required))
            {
                best = Some(MixedLtTreeEntry {
                    choice: MixedLtTreeChoice::Split {
                        buffer_index,
                        next_x_index: next_x,
                        next_y_index: next_y,
                    },
                    required: local_required,
                    area: local_area,
                });
            }
        }
    }

    best.ok_or_else(|| {
        let two_level_key = TwoLevelKey {
            source_index: key.source_index,
            source_polarity: key.source_polarity,
            sink_index: if key.x_index == x_count {
                key.y_index
            } else {
                key.x_index
            },
            sink_polarity: if key.x_index == x_count {
                Polarity::Negative
            } else {
                Polarity::Positive
            },
        };
        MixedLtTreeError::MissingTwoLevelCandidate { key: two_level_key }
    })
}

fn two_level_boundary_entry(
    input: &MixedLtTreeInput,
    key: MixedLtTreeKey,
) -> Result<Option<MixedLtTreeEntry>, MixedLtTreeError> {
    let x_count = input.fanout_data.sinks(Polarity::Positive).len();
    let y_count = input.fanout_data.sinks(Polarity::Negative).len();
    let two_level_key = if key.x_index == x_count {
        Some(TwoLevelKey {
            source_index: key.source_index,
            source_polarity: key.source_polarity,
            sink_index: key.y_index,
            sink_polarity: Polarity::Negative,
        })
    } else if key.y_index == y_count {
        Some(TwoLevelKey {
            source_index: key.source_index,
            source_polarity: key.source_polarity,
            sink_index: key.x_index,
            sink_polarity: Polarity::Positive,
        })
    } else {
        None
    };

    let Some(two_level_key) = two_level_key else {
        return Ok(None);
    };
    let candidate = input
        .two_level
        .get(two_level_key)
        .ok_or(MixedLtTreeError::MissingTwoLevelCandidate { key: two_level_key })?;

    Ok(Some(MixedLtTreeEntry {
        choice: MixedLtTreeChoice::TwoLevel {
            sink_polarity: two_level_key.sink_polarity,
            candidate: *candidate,
        },
        required: candidate.required,
        area: candidate.area,
    }))
}

fn all_polarities() -> [Polarity; 2] {
    [Polarity::Positive, Polarity::Negative]
}

fn source_polarity(kind: GateKind) -> Option<Polarity> {
    match kind {
        GateKind::Source { polarity } => Some(polarity),
        GateKind::Buffer | GateKind::Inverter => None,
    }
}

fn is_source(kind: GateKind) -> bool {
    matches!(kind, GateKind::Source { .. })
}

fn is_buffer(kind: GateKind) -> bool {
    matches!(kind, GateKind::Buffer | GateKind::Inverter)
}

fn output_polarity(kind: GateKind, input: Polarity) -> Polarity {
    match kind {
        GateKind::Inverter => input.inverted(),
        GateKind::Source { polarity } => polarity,
        GateKind::Buffer => input,
    }
}

fn index_for_polarity(polarity: Polarity, x_index: usize, y_index: usize) -> usize {
    match polarity {
        Polarity::Positive => x_index,
        Polarity::Negative => y_index,
    }
}

fn sink_count(input: &MixedLtTreeInput, polarity: Polarity) -> usize {
    input.fanout_data.sinks(polarity).len()
}

fn interval_load(
    input: &MixedLtTreeInput,
    polarity: Polarity,
    start: usize,
    end: usize,
) -> Result<f64, MixedLtTreeError> {
    let sinks = input.fanout_data.sinks(polarity);
    if start > end || end > sinks.len() {
        return Err(MixedLtTreeError::InvalidSinkRange {
            polarity,
            start,
            end,
        });
    }
    Ok(sinks.sinks[start..end].iter().map(|sink| sink.load).sum())
}

fn buffer_indices(gates: &[FanoutGate]) -> impl Iterator<Item = usize> + '_ {
    gates
        .iter()
        .enumerate()
        .filter(|(_, gate)| is_buffer(gate.kind))
        .map(|(index, _)| index)
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

fn min_delay(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise.min(right.rise), left.fall.min(right.fall))
}

fn min_component(value: DelayTime) -> f64 {
    value.rise.min(value.fall)
}

fn validate_gate(index: usize, gate: &FanoutGate) -> Result<(), MixedLtTreeError> {
    if !gate.area.is_finite() || gate.area < 0.0 {
        return Err(MixedLtTreeError::InvalidArea { gate_index: index });
    }
    if !gate.input_load.is_finite() || gate.input_load < 0.0 {
        return Err(MixedLtTreeError::InvalidInputLoad { gate_index: index });
    }
    validate_delay(gate.timing.intrinsic, "gate intrinsic")?;
    validate_delay(gate.timing.load_slope, "gate load slope")?;
    if gate.timing.load_slope.rise < 0.0 || gate.timing.load_slope.fall < 0.0 {
        return Err(MixedLtTreeError::InvalidDelay {
            context: "gate load slope",
        });
    }

    Ok(())
}

fn validate_delay(value: DelayTime, context: &'static str) -> Result<(), MixedLtTreeError> {
    if value.rise.is_nan() || value.fall.is_nan() {
        return Err(MixedLtTreeError::InvalidDelay { context });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::lt_trees::{FanoutSink, GateTiming};
    use super::*;

    #[test]
    fn declines_when_one_polarity_has_no_sinks() {
        let mut input = sample_input();
        input.fanout_data = FanoutData::new(
            vec![FanoutSink::new("x0", 1.0, DelayTime::new(9.0, 9.0))],
            Vec::new(),
        );

        assert_eq!(optimize_mixed_lt_trees(input).unwrap(), None);
    }

    #[test]
    fn uses_two_level_when_one_polarity_frontier_is_complete() {
        let input = sample_input();
        let plan = optimize_mixed_lt_trees(input).unwrap().unwrap();
        let entry = plan
            .entry(MixedLtTreeKey {
                source_index: 0,
                source_polarity: Polarity::Positive,
                x_index: 1,
                y_index: 0,
                gaps_remaining: 0,
            })
            .unwrap();

        assert!(matches!(
            entry.choice,
            MixedLtTreeChoice::TwoLevel {
                sink_polarity: Polarity::Negative,
                ..
            }
        ));
    }

    #[test]
    fn builds_tree_for_best_mixed_split() {
        let input = sample_input();
        let plan = optimize_mixed_lt_trees(input).unwrap().unwrap();
        let (source, _) = plan.select_best_source().unwrap();
        let actions = plan.build_tree(source).unwrap();

        assert_eq!(
            actions,
            vec![
                FanoutTreeAction::Gate {
                    gate_index: 0,
                    fanout_count: 2
                },
                FanoutTreeAction::Sink {
                    polarity: Polarity::Positive,
                    sink_name: "x0".to_string()
                },
                FanoutTreeAction::Gate {
                    gate_index: 2,
                    fanout_count: 1
                },
                FanoutTreeAction::TwoLevel {
                    source_index: 2,
                    sink_polarity: Polarity::Negative,
                    sink_start: 0,
                    candidate: TwoLevelCandidate::new(2, 1, DelayTime::new(9.0, 9.0), 1.0)
                }
            ]
        );
    }

    fn sample_input() -> MixedLtTreeInput {
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
                "buffer",
                GateKind::Buffer,
                1.0,
                1.0,
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
            vec![FanoutSink::new("x0", 1.0, DelayTime::new(8.0, 8.0))],
            vec![FanoutSink::new("y0", 1.0, DelayTime::new(8.0, 8.0))],
        );
        let mut two_level = TwoLevelTable::new();
        for source_index in 0..gates.len() {
            for source_polarity_value in all_polarities() {
                if source_polarity(gates[source_index].kind)
                    .is_some_and(|polarity| polarity != source_polarity_value)
                {
                    continue;
                }
                for sink_polarity in all_polarities() {
                    two_level.insert(
                        TwoLevelKey {
                            source_index,
                            source_polarity: source_polarity_value,
                            sink_index: 0,
                            sink_polarity,
                        },
                        TwoLevelCandidate::new(
                            source_index,
                            1,
                            if source_index == 2 && sink_polarity == Polarity::Negative {
                                DelayTime::new(9.0, 9.0)
                            } else {
                                DelayTime::new(1.0, 1.0)
                            },
                            1.0,
                        ),
                    );
                }
            }
        }

        MixedLtTreeInput::new(
            gates,
            fanout_data,
            two_level,
            MixedLtTreeOptions {
                max_gaps: 1,
                max_x_index: 15,
                max_y_index: 15,
                wire_load: WireLoadModel::default(),
            },
        )
        .unwrap()
    }
}
