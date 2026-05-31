//! Native bottom-up fanout-tree construction for `sis/map/bottom_up.c`.
//!
//! The SIS implementation builds a buffer tree by repeatedly grouping the latest
//! required sinks and feeding the grouped subtree back into the opposite or same
//! polarity list depending on the selected buffer. This port keeps that algorithm
//! in owned Rust data and returns the native `FanoutTreeForest` representation.
//! Full `network_t` extraction is not part of this file; callers must provide
//! the already-collected sink groups and timing model.

use std::error::Error;
use std::fmt;
use std::rc::Rc;

use super::fanout_delay::FanoutPolarity as TimingPolarity;
use super::fanout_tree::{
    FanoutCost, FanoutPolarity as TreePolarity, FanoutSink, FanoutTreeError, FanoutTreeForest,
    FanoutTreeNode,
};
use super::virtual_net::{DelayTime, MINUS_INFINITY};

#[derive(Clone, Debug, PartialEq)]
pub struct BottomUpSink {
    pub name: String,
    pub pin: usize,
    pub load: f64,
    pub required: DelayTime,
}

impl BottomUpSink {
    pub fn new(name: impl Into<String>, pin: usize, load: f64, required: DelayTime) -> Self {
        Self {
            name: name.into(),
            pin,
            load,
            required,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BottomUpFanoutInfo {
    positive: Vec<BottomUpSink>,
    negative: Vec<BottomUpSink>,
}

impl BottomUpFanoutInfo {
    pub fn new(positive: Vec<BottomUpSink>, negative: Vec<BottomUpSink>) -> Self {
        Self { positive, negative }
    }

    pub fn positive(&self) -> &[BottomUpSink] {
        &self.positive
    }

    pub fn negative(&self) -> &[BottomUpSink] {
        &self.negative
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BottomUpPlan {
    pub source_index: usize,
    pub tree: FanoutTreeForest,
    pub cost: FanoutCost,
}

pub trait BottomUpTiming {
    fn source_indices(&self) -> &[usize];

    fn buffer_indices(&self) -> &[usize];

    fn source_polarity(&self, source_index: usize) -> Result<TimingPolarity, BottomUpError>;

    fn buffer_polarity(&self, buffer_index: usize) -> Result<TimingPolarity, BottomUpError>;

    fn is_buffer(&self, gate_index: usize) -> bool;

    fn buffer_load(&self, buffer_index: usize) -> Result<f64, BottomUpError>;

    fn gate_area(&self, gate_index: usize) -> Result<f64, BottomUpError>;

    fn wire_load(&self, fanout_count: usize) -> Result<f64, BottomUpError>;

    fn backward_intrinsic(
        &self,
        required: DelayTime,
        gate_index: usize,
    ) -> Result<DelayTime, BottomUpError>;

    fn backward_load_dependent(
        &self,
        required: DelayTime,
        gate_index: usize,
        load: f64,
    ) -> Result<DelayTime, BottomUpError>;

    fn best_number_of_inverters(
        &self,
        source_index: usize,
        buffer_index: usize,
        load: f64,
        max_count: usize,
    ) -> Result<usize, BottomUpError>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum BottomUpError {
    EmptyFanout,
    EmptySourceSet,
    EmptySinkName,
    InvalidSink {
        name: String,
    },
    InvalidMetric {
        metric: &'static str,
        value: f64,
    },
    InvalidGateIndex {
        index: usize,
    },
    MissingUsableBuffer {
        source_index: usize,
        polarity: TimingPolarity,
    },
    MissingBestSource,
    Tree(FanoutTreeError),
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for BottomUpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFanout => write!(f, "bottom-up fanout optimization has no sinks"),
            Self::EmptySourceSet => write!(f, "bottom-up fanout optimization has no sources"),
            Self::EmptySinkName => write!(f, "bottom-up fanout sink name cannot be empty"),
            Self::InvalidSink { name } => write!(f, "bottom-up fanout sink '{name}' is invalid"),
            Self::InvalidMetric { metric, value } => {
                write!(f, "bottom-up fanout {metric} has invalid value {value}")
            }
            Self::InvalidGateIndex { index } => {
                write!(f, "bottom-up fanout gate index {index} is invalid")
            }
            Self::MissingUsableBuffer {
                source_index,
                polarity,
            } => write!(
                f,
                "source {source_index} has no usable bottom-up buffer for {polarity:?} sinks"
            ),
            Self::MissingBestSource => write!(f, "bottom-up fanout did not select a source"),
            Self::Tree(error) => write!(f, "{error}"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for BottomUpError {}

impl From<FanoutTreeError> for BottomUpError {
    fn from(value: FanoutTreeError) -> Self {
        Self::Tree(value)
    }
}

pub fn full_sis_bottom_up_unavailable() -> Result<BottomUpPlan, BottomUpError> {
    Err(BottomUpError::MissingSisPorts {
        operation: "bottom_up full SIS fanout-info extraction",
    })
}

pub fn optimize_bottom_up(
    fanout_info: &BottomUpFanoutInfo,
    timing: &impl BottomUpTiming,
) -> Result<BottomUpPlan, BottomUpError> {
    validate_fanout_info(fanout_info)?;
    if timing.source_indices().is_empty() {
        return Err(BottomUpError::EmptySourceSet);
    }

    let mut best: Option<BottomUpPlan> = None;
    for source_index in timing.source_indices() {
        let plan = build_tree_bottom_up(fanout_info, *source_index, timing)?;
        if best.as_ref().is_none_or(|current| {
            min_component(current.cost.slack) < min_component(plan.cost.slack)
        }) {
            best = Some(plan);
        }
    }

    best.ok_or(BottomUpError::MissingBestSource)
}

pub fn build_tree_bottom_up(
    fanout_info: &BottomUpFanoutInfo,
    source_index: usize,
    timing: &impl BottomUpTiming,
) -> Result<BottomUpPlan, BottomUpError> {
    validate_fanout_info(fanout_info)?;

    let mut groups = [
        FanoutGroup::from_sinks(&fanout_info.positive, TimingPolarity::Positive)?,
        FanoutGroup::from_sinks(&fanout_info.negative, TimingPolarity::Negative)?,
    ];
    let root = build_tree_bottom_up_rec(&mut groups, source_index, timing)?;
    let tree = tree_from_root(&root)?;
    let cost = FanoutCost {
        slack: root.required,
        area: root.area,
    };

    Ok(BottomUpPlan {
        source_index,
        tree,
        cost,
    })
}

fn build_tree_bottom_up_rec(
    fanout_info: &mut [FanoutGroup; 2],
    source_index: usize,
    timing: &impl BottomUpTiming,
) -> Result<Rc<BuildNode>, BottomUpError> {
    let source_polarity = timing.source_polarity(source_index)?;
    let source_slot = polarity_index(source_polarity);
    let other_slot = polarity_index(source_polarity.inverted());

    if fanout_info[source_slot].len() == 1 && fanout_info[other_slot].is_empty() {
        return create_tree_node(&fanout_info[source_slot], 0, source_index, timing)
            .map(|candidate| candidate.node);
    }

    let polarity = latest_required_polarity(fanout_info)?;
    let polarity_slot = polarity_index(polarity);
    let inverted_slot = polarity_index(polarity.inverted());
    let no_buffer_ok = source_polarity == polarity && fanout_info[inverted_slot].is_empty();
    let subgroup = select_best_subgroup(
        &fanout_info[polarity_slot],
        source_index,
        no_buffer_ok,
        timing,
    )?;

    let Some(buffer_index) = subgroup.buffer_index else {
        return create_tree_node(&fanout_info[polarity_slot], 0, source_index, timing)
            .map(|candidate| candidate.node);
    };

    let node = create_tree_node(
        &fanout_info[polarity_slot],
        subgroup.sink_index,
        buffer_index,
        timing,
    )?;
    let mut next = [
        fanout_info[polarity_index(TimingPolarity::Positive)].extract(0, 0),
        fanout_info[polarity_index(TimingPolarity::Negative)].extract(0, 0),
    ];
    next[polarity_slot] = fanout_info[polarity_slot].extract(0, subgroup.sink_index);
    next[inverted_slot] = fanout_info[inverted_slot].extract(0, fanout_info[inverted_slot].len());

    let target = if timing.buffer_polarity(buffer_index)? == TimingPolarity::Negative {
        inverted_slot
    } else {
        polarity_slot
    };
    next[target].push(node)?;
    build_tree_bottom_up_rec(&mut next, source_index, timing)
}

fn select_best_subgroup(
    fanout_info: &FanoutGroup,
    source_index: usize,
    no_buffer_ok: bool,
    timing: &impl BottomUpTiming,
) -> Result<SelectedSubgroup, BottomUpError> {
    let load = fanout_info.total_load + timing.wire_load(fanout_info.len())?;
    let mut best_buffer = None;
    let mut best_sink = 0;
    let mut best_count = 0;
    let mut best_required = MINUS_INFINITY;

    for buffer_index in timing.buffer_indices() {
        if fanout_info.len() == 1
            && timing.buffer_polarity(*buffer_index)? != TimingPolarity::Negative
        {
            continue;
        }

        let count = timing.best_number_of_inverters(
            source_index,
            *buffer_index,
            load,
            fanout_info.len(),
        )?;
        let sink_index = compute_sink_index(fanout_info, count);
        let mut local_load = fanout_info.total_load - fanout_info.cumulative_load[sink_index];
        local_load += timing.wire_load(fanout_info.len() - sink_index)?;
        let mut local_required = fanout_info.minimum_required[sink_index];
        local_required =
            timing.backward_load_dependent(local_required, *buffer_index, local_load)?;
        local_required = timing.backward_intrinsic(local_required, *buffer_index)?;
        local_load = timing.buffer_load(*buffer_index)? * count as f64;
        local_load += timing.wire_load(count)?;
        local_required =
            timing.backward_load_dependent(local_required, source_index, local_load)?;

        if min_component(best_required) < min_component(local_required) {
            best_count = count;
            best_required = local_required;
            best_buffer = Some(*buffer_index);
            best_sink = sink_index;
        }
    }

    if no_buffer_ok && best_count == 1 && best_sink == 0 {
        let local_required =
            timing.backward_load_dependent(fanout_info.minimum_required[0], source_index, load)?;
        if min_component(best_required) < min_component(local_required) {
            best_buffer = None;
        }
    }

    if best_buffer.is_none() && !no_buffer_ok {
        return Err(BottomUpError::MissingUsableBuffer {
            source_index,
            polarity: fanout_info.polarity,
        });
    }

    Ok(SelectedSubgroup {
        buffer_index: best_buffer,
        sink_index: best_sink,
    })
}

fn create_tree_node(
    fanout_info: &FanoutGroup,
    sink_index: usize,
    source_index: usize,
    timing: &impl BottomUpTiming,
) -> Result<BottomUpCandidate, BottomUpError> {
    if sink_index >= fanout_info.len() {
        return Err(BottomUpError::InvalidGateIndex { index: sink_index });
    }

    let child_count = fanout_info.len() - sink_index;
    let mut load = fanout_info.total_load - fanout_info.cumulative_load[sink_index];
    load += timing.wire_load(child_count)?;
    let mut required = fanout_info.minimum_required[sink_index];
    required = timing.backward_load_dependent(required, source_index, load)?;

    let link = if timing.is_buffer(source_index) {
        BottomUpSink {
            name: format!("bottom_up:{source_index}:{sink_index}"),
            pin: 0,
            load: timing.buffer_load(source_index)?,
            required: timing.backward_intrinsic(required, source_index)?,
        }
    } else {
        BottomUpSink {
            name: format!("bottom_up:source:{source_index}"),
            pin: 0,
            load: 0.0,
            required,
        }
    };

    let children = fanout_info.items[sink_index..]
        .iter()
        .map(|candidate| Rc::clone(&candidate.node))
        .collect::<Vec<_>>();
    let area =
        children.iter().map(|child| child.area).sum::<f64>() + timing.gate_area(source_index)?;
    let node = Rc::new(BuildNode {
        gate_index: Some(source_index),
        sink: link.clone(),
        polarity: fanout_info.polarity,
        children,
        required: link.required,
        area,
    });

    Ok(BottomUpCandidate { sink: link, node })
}

fn compute_sink_index(fanout_info: &FanoutGroup, count: usize) -> usize {
    if fanout_info.len() == 1 {
        return 0;
    }

    let load_threshold = fanout_info.total_load / count as f64;
    for index in (0..=(fanout_info.len() - 2)).rev() {
        let load = fanout_info.total_load - fanout_info.cumulative_load[index];
        if load > load_threshold {
            return index;
        }
    }

    0
}

fn latest_required_polarity(
    fanout_info: &[FanoutGroup; 2],
) -> Result<TimingPolarity, BottomUpError> {
    if fanout_info[0].is_empty() && fanout_info[1].is_empty() {
        return Err(BottomUpError::EmptyFanout);
    }
    if fanout_info[0].is_empty() {
        return Ok(TimingPolarity::Negative);
    }
    if fanout_info[1].is_empty() {
        return Ok(TimingPolarity::Positive);
    }

    let positive = fanout_info[0].items.last().expect("checked non-empty");
    let negative = fanout_info[1].items.last().expect("checked non-empty");
    if min_component(positive.sink.required) > min_component(negative.sink.required) {
        Ok(TimingPolarity::Positive)
    } else {
        Ok(TimingPolarity::Negative)
    }
}

fn tree_from_root(root: &Rc<BuildNode>) -> Result<FanoutTreeForest, BottomUpError> {
    let mut nodes = Vec::new();
    push_prefix_nodes(root, &mut nodes);
    FanoutTreeForest::from_prefix(nodes).map_err(BottomUpError::from)
}

fn push_prefix_nodes(node: &Rc<BuildNode>, nodes: &mut Vec<FanoutTreeNode>) {
    if node.children.is_empty() {
        nodes.push(FanoutTreeNode::sink(FanoutSink::new(
            node.sink.name.clone(),
            node.sink.pin,
            to_tree_polarity(node.polarity),
            node.sink.load,
            node.sink.required,
        )));
        return;
    }

    nodes.push(FanoutTreeNode::buffer(
        node.gate_index
            .expect("non-leaf bottom-up nodes have a gate"),
        node.children.len(),
    ));
    for child in &node.children {
        push_prefix_nodes(child, nodes);
    }
}

fn validate_fanout_info(fanout_info: &BottomUpFanoutInfo) -> Result<(), BottomUpError> {
    if fanout_info.positive.is_empty() && fanout_info.negative.is_empty() {
        return Err(BottomUpError::EmptyFanout);
    }
    for sink in fanout_info
        .positive
        .iter()
        .chain(fanout_info.negative.iter())
    {
        validate_sink(sink)?;
    }

    Ok(())
}

fn validate_sink(sink: &BottomUpSink) -> Result<(), BottomUpError> {
    if sink.name.is_empty() {
        return Err(BottomUpError::EmptySinkName);
    }
    if !sink.load.is_finite() || sink.load < 0.0 {
        return Err(BottomUpError::InvalidMetric {
            metric: "sink.load",
            value: sink.load,
        });
    }
    if sink.required.rise.is_nan() || sink.required.fall.is_nan() {
        return Err(BottomUpError::InvalidSink {
            name: sink.name.clone(),
        });
    }

    Ok(())
}

fn polarity_index(polarity: TimingPolarity) -> usize {
    match polarity {
        TimingPolarity::Positive => 0,
        TimingPolarity::Negative => 1,
    }
}

fn to_tree_polarity(polarity: TimingPolarity) -> TreePolarity {
    match polarity {
        TimingPolarity::Positive => TreePolarity::Positive,
        TimingPolarity::Negative => TreePolarity::Negative,
    }
}

fn min_component(delay: DelayTime) -> f64 {
    delay.rise.min(delay.fall)
}

#[derive(Clone, Debug)]
struct FanoutGroup {
    polarity: TimingPolarity,
    items: Vec<BottomUpCandidate>,
    total_load: f64,
    cumulative_load: Vec<f64>,
    minimum_required: Vec<DelayTime>,
}

impl FanoutGroup {
    fn from_sinks(sinks: &[BottomUpSink], polarity: TimingPolarity) -> Result<Self, BottomUpError> {
        let items = sinks
            .iter()
            .cloned()
            .map(|sink| {
                validate_sink(&sink)?;
                Ok(BottomUpCandidate {
                    node: Rc::new(BuildNode {
                        gate_index: None,
                        sink: sink.clone(),
                        polarity,
                        children: Vec::new(),
                        required: sink.required,
                        area: 0.0,
                    }),
                    sink,
                })
            })
            .collect::<Result<Vec<_>, BottomUpError>>()?;
        Self::from_items(items, polarity)
    }

    fn from_items(
        mut items: Vec<BottomUpCandidate>,
        polarity: TimingPolarity,
    ) -> Result<Self, BottomUpError> {
        items.sort_by(|left, right| {
            min_component(left.sink.required)
                .total_cmp(&min_component(right.sink.required))
                .then_with(|| left.sink.name.cmp(&right.sink.name))
                .then_with(|| left.sink.pin.cmp(&right.sink.pin))
        });

        let total_load = items.iter().map(|item| item.sink.load).sum::<f64>();
        if !total_load.is_finite() || total_load < 0.0 {
            return Err(BottomUpError::InvalidMetric {
                metric: "group.total_load",
                value: total_load,
            });
        }

        let mut cumulative_load = Vec::with_capacity(items.len());
        let mut load = 0.0;
        for item in &items {
            cumulative_load.push(load);
            load += item.sink.load;
        }

        let mut minimum_required = vec![MINUS_INFINITY; items.len()];
        let mut suffix = DelayTime::new(f64::INFINITY, f64::INFINITY);
        for index in (0..items.len()).rev() {
            suffix = suffix.min(items[index].sink.required);
            minimum_required[index] = suffix;
        }

        Ok(Self {
            polarity,
            items,
            total_load,
            cumulative_load,
            minimum_required,
        })
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn extract(&self, from: usize, to: usize) -> Self {
        Self::from_items(self.items[from..to].to_vec(), self.polarity)
            .expect("extracting an already validated fanout group should remain valid")
    }

    fn push(&mut self, item: BottomUpCandidate) -> Result<(), BottomUpError> {
        let mut items = self.items.clone();
        items.push(item);
        *self = Self::from_items(items, self.polarity)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct BottomUpCandidate {
    sink: BottomUpSink,
    node: Rc<BuildNode>,
}

#[derive(Clone, Debug)]
struct BuildNode {
    gate_index: Option<usize>,
    sink: BottomUpSink,
    polarity: TimingPolarity,
    children: Vec<Rc<BuildNode>>,
    required: DelayTime,
    area: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SelectedSubgroup {
    buffer_index: Option<usize>,
    sink_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTiming {
        sources: Vec<usize>,
        buffers: Vec<usize>,
        polarities: Vec<TimingPolarity>,
        loads: Vec<f64>,
        areas: Vec<f64>,
        load_factor: Vec<f64>,
        intrinsic: Vec<f64>,
        wire_load: f64,
    }

    impl BottomUpTiming for TestTiming {
        fn source_indices(&self) -> &[usize] {
            &self.sources
        }

        fn buffer_indices(&self) -> &[usize] {
            &self.buffers
        }

        fn source_polarity(&self, source_index: usize) -> Result<TimingPolarity, BottomUpError> {
            self.polarity(source_index)
        }

        fn buffer_polarity(&self, buffer_index: usize) -> Result<TimingPolarity, BottomUpError> {
            self.polarity(buffer_index)
        }

        fn is_buffer(&self, gate_index: usize) -> bool {
            self.buffers.contains(&gate_index)
        }

        fn buffer_load(&self, buffer_index: usize) -> Result<f64, BottomUpError> {
            self.loads
                .get(buffer_index)
                .copied()
                .ok_or(BottomUpError::InvalidGateIndex {
                    index: buffer_index,
                })
        }

        fn gate_area(&self, gate_index: usize) -> Result<f64, BottomUpError> {
            self.areas
                .get(gate_index)
                .copied()
                .ok_or(BottomUpError::InvalidGateIndex { index: gate_index })
        }

        fn wire_load(&self, fanout_count: usize) -> Result<f64, BottomUpError> {
            Ok(fanout_count as f64 * self.wire_load)
        }

        fn backward_intrinsic(
            &self,
            required: DelayTime,
            gate_index: usize,
        ) -> Result<DelayTime, BottomUpError> {
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
        ) -> Result<DelayTime, BottomUpError> {
            let factor = self.value(&self.load_factor, gate_index)?;
            Ok(DelayTime::new(
                required.rise - load * factor,
                required.fall - load * factor,
            ))
        }

        fn best_number_of_inverters(
            &self,
            _source_index: usize,
            _buffer_index: usize,
            _load: f64,
            max_count: usize,
        ) -> Result<usize, BottomUpError> {
            Ok(max_count.clamp(1, 2))
        }
    }

    impl TestTiming {
        fn polarity(&self, index: usize) -> Result<TimingPolarity, BottomUpError> {
            self.polarities
                .get(index)
                .copied()
                .ok_or(BottomUpError::InvalidGateIndex { index })
        }

        fn value(&self, values: &[f64], index: usize) -> Result<f64, BottomUpError> {
            values
                .get(index)
                .copied()
                .ok_or(BottomUpError::InvalidGateIndex { index })
        }
    }

    fn timing() -> TestTiming {
        TestTiming {
            sources: vec![2],
            buffers: vec![0, 1],
            polarities: vec![
                TimingPolarity::Positive,
                TimingPolarity::Negative,
                TimingPolarity::Positive,
            ],
            loads: vec![1.0, 1.0, 0.0],
            areas: vec![2.0, 3.0, 0.0],
            load_factor: vec![0.1, 0.1, 0.05],
            intrinsic: vec![0.5, 0.5, 0.0],
            wire_load: 0.0,
        }
    }

    #[test]
    fn builds_single_polarity_tree_in_prefix_order() {
        let info = BottomUpFanoutInfo::new(
            vec![
                BottomUpSink::new("a", 0, 1.0, DelayTime::new(10.0, 10.0)),
                BottomUpSink::new("b", 0, 1.0, DelayTime::new(8.0, 8.0)),
                BottomUpSink::new("c", 0, 1.0, DelayTime::new(7.0, 7.0)),
            ],
            Vec::new(),
        );

        let plan = optimize_bottom_up(&info, &timing()).unwrap();

        assert_eq!(plan.source_index, 2);
        assert_eq!(plan.tree.nodes().len(), 6);
        assert_eq!(plan.tree.roots().len(), 1);
        assert_eq!(plan.cost.area, 4.0);
        assert!(plan.cost.slack.rise < 7.0);
    }

    #[test]
    fn can_skip_buffer_when_one_source_polarity_sink_remains() {
        let info = BottomUpFanoutInfo::new(
            vec![BottomUpSink::new("out", 0, 2.0, DelayTime::new(5.0, 5.0))],
            Vec::new(),
        );

        let plan = optimize_bottom_up(&info, &timing()).unwrap();

        assert_eq!(plan.tree.nodes().len(), 2);
        assert_eq!(plan.cost.area, 0.0);
        assert_eq!(plan.cost.slack, DelayTime::new(4.9, 4.9));
    }

    #[test]
    fn rejects_missing_sources_and_legacy_c_abi_tokens() {
        let info = BottomUpFanoutInfo::new(
            vec![BottomUpSink::new("out", 0, 1.0, DelayTime::new(1.0, 1.0))],
            Vec::new(),
        );
        let mut timing = timing();
        timing.sources.clear();

        assert_eq!(
            optimize_bottom_up(&info, &timing).unwrap_err(),
            BottomUpError::EmptySourceSet
        );

        let source = include_str!("bottom_up.rs");
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
