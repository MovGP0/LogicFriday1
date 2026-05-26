//! Native Rust port surface for `sis/atpg/com_redund.c`.
//!
//! The original file combines SIS command handling, ATPG execution, fault
//! pattern bookkeeping, and direct network mutation. The complete ATPG engine is
//! still outside this module, so this port exposes the pieces that are local to
//! `com_redund.c`: fault-pattern table behavior and redundancy patching through
//! a native Rust network trait.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FaultValue
{
    StuckAtZero,
    StuckAtOne,
}

impl FaultValue
{
    pub fn bit(self) -> u8
    {
        match self
        {
            Self::StuckAtZero => 0,
            Self::StuckAtOne => 1,
        }
    }

    pub fn constant_value(self) -> bool
    {
        self == Self::StuckAtOne
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FaultPattern
{
    pub node: usize,
    pub fanin: Option<usize>,
    pub value: FaultValue,
}

impl FaultPattern
{
    pub fn new(node: usize, fanin: Option<usize>, value: FaultValue) -> Self
    {
        Self { node, fanin, value }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RedundancyType
{
    Control,
    Observe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RedundantFault
{
    pub pattern: FaultPattern,
    pub redundancy_type: RedundancyType,
}

impl RedundantFault
{
    pub fn new(pattern: FaultPattern, redundancy_type: RedundancyType) -> Self
    {
        Self {
            pattern,
            redundancy_type,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedundancyRemovalOptions
{
    pub force_comb: bool,
    pub timeout_seconds: Option<u32>,
    pub tech_decomp: bool,
    pub quick_redundancy: bool,
    pub rtg_enabled: bool,
    pub random_propagation: bool,
    pub build_product_machines: bool,
    pub deterministic_propagation: bool,
    pub fast_sat: bool,
    pub rtg_depth: Option<usize>,
    pub random_propagation_length: usize,
    pub n_sim_sequences: usize,
    pub verbosity: u8,
}

impl Default for RedundancyRemovalOptions
{
    fn default() -> Self
    {
        Self {
            force_comb: false,
            timeout_seconds: None,
            tech_decomp: false,
            quick_redundancy: false,
            rtg_enabled: true,
            random_propagation: true,
            build_product_machines: true,
            deterministic_propagation: true,
            fast_sat: false,
            rtg_depth: None,
            random_propagation_length: 20,
            n_sim_sequences: usize::BITS as usize,
            verbosity: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedundancyRemovalReport
{
    pub redundant_faults_removed: usize,
    pub setup_count: usize,
    pub output_functions_changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RedundancyRemovalError
{
    AtpgEngineUnavailable,
}

impl fmt::Display for RedundancyRemovalError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::AtpgEngineUnavailable => {
                write!(f, "SIS ATPG redundancy removal is not fully ported to Rust yet")
            }
        }
    }
}

impl Error for RedundancyRemovalError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaultPatternTable<S>
{
    sequences: HashMap<FaultPattern, S>,
    previous_faults: HashSet<FaultPattern>,
}

impl<S> Default for FaultPatternTable<S>
{
    fn default() -> Self
    {
        Self {
            sequences: HashMap::new(),
            previous_faults: HashSet::new(),
        }
    }
}

impl<S> FaultPatternTable<S>
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn len(&self) -> usize
    {
        self.sequences.len()
    }

    pub fn previous_fault_count(&self) -> usize
    {
        self.previous_faults.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.sequences.is_empty()
    }

    pub fn sequence_for(&self, pattern: &FaultPattern) -> Option<&S>
    {
        self.sequences.get(pattern)
    }

    pub fn has_previous_fault(&self, pattern: &FaultPattern) -> bool
    {
        self.previous_faults.contains(pattern)
    }

    pub fn save_fault_pattern(&mut self, pattern: FaultPattern, sequence: S) -> Option<S>
    {
        self.previous_faults.insert(pattern);
        self.sequences.insert(pattern, sequence)
    }

    pub fn record_previous_fault(&mut self, pattern: FaultPattern) -> bool
    {
        self.previous_faults.insert(pattern)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbortedFaultTable
{
    retry_flags: HashMap<FaultPattern, bool>,
}

impl Default for AbortedFaultTable
{
    fn default() -> Self
    {
        Self {
            retry_flags: HashMap::new(),
        }
    }
}

impl AbortedFaultTable
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn len(&self) -> usize
    {
        self.retry_flags.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.retry_flags.is_empty()
    }

    pub fn reset_retry_flags(&mut self)
    {
        for retry_flag in self.retry_flags.values_mut()
        {
            *retry_flag = false;
        }
    }

    pub fn record_first_abort(&mut self, pattern: FaultPattern) -> bool
    {
        self.retry_flags.insert(pattern, false).is_none()
    }

    pub fn record_later_abort(&mut self, pattern: FaultPattern) -> bool
    {
        self.retry_flags.insert(pattern, true).is_none()
    }

    pub fn should_defer_once(&mut self, pattern: &FaultPattern) -> bool
    {
        match self.retry_flags.get_mut(pattern)
        {
            Some(retry_flag) if !*retry_flag => {
                *retry_flag = true;
                true
            }
            _ => false,
        }
    }

    pub fn retry_flag(&self, pattern: &FaultPattern) -> Option<bool>
    {
        self.retry_flags.get(pattern).copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RedundancyPatch
{
    None,
    AllFanouts {
        node: usize,
        replacement: usize,
    },
    Fannin {
        node: usize,
        fanin: usize,
        replacement: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RedundancyPatchResult
{
    pub pi_seen_before: bool,
    pub output_functions_changed: bool,
    pub patch: RedundancyPatch,
}

pub trait RedundancyNetwork
{
    fn is_primary_input(&self, node: usize) -> bool;

    fn fanout_count(&self, node: usize) -> usize;

    fn add_constant(&mut self, value: bool) -> usize;

    fn patch_all_fanouts(&mut self, node: usize, replacement: usize);

    fn patch_fanin(&mut self, node: usize, fanin: usize, replacement: usize);

    fn sweep(&mut self);
}

pub fn fault_pattern_compare(left: &FaultPattern, right: &FaultPattern) -> Ordering
{
    left.node
        .cmp(&right.node)
        .then_with(|| fanin_sort_value(left.fanin).cmp(&fanin_sort_value(right.fanin)))
        .then_with(|| left.value.bit().cmp(&right.value.bit()))
}

pub fn fault_pattern_hash(pattern: &FaultPattern, modulus: usize) -> usize
{
    assert!(modulus > 0, "hash modulus must be non-zero");

    pattern
        .node
        .wrapping_add(fanin_sort_value(pattern.fanin))
        .wrapping_add(pattern.value.bit() as usize + 1)
        % modulus
}

pub fn remove_redundancy<N>(
    network: &mut N,
    fault: &RedundantFault,
) -> RedundancyPatchResult
where
    N: RedundancyNetwork,
{
    let node = fault.pattern.node;
    let fanout_count = network.fanout_count(node);

    if network.is_primary_input(node) && fanout_count == 0
    {
        return RedundancyPatchResult {
            pi_seen_before: true,
            output_functions_changed: false,
            patch: RedundancyPatch::None,
        };
    }

    let replacement = network.add_constant(fault.pattern.value.constant_value());
    let patch = match fault.pattern.fanin
    {
        Some(fanin) => {
            network.patch_fanin(node, fanin, replacement);
            RedundancyPatch::Fannin {
                node,
                fanin,
                replacement,
            }
        }
        None => {
            network.patch_all_fanouts(node, replacement);
            RedundancyPatch::AllFanouts { node, replacement }
        }
    };

    network.sweep();

    RedundancyPatchResult {
        pi_seen_before: false,
        output_functions_changed: fault.redundancy_type == RedundancyType::Observe,
        patch,
    }
}

pub fn redundancy_removal_network_bound<Network>(
    _network: &mut Network,
    _options: &RedundancyRemovalOptions,
) -> Result<RedundancyRemovalReport, RedundancyRemovalError>
{
    Err(RedundancyRemovalError::AtpgEngineUnavailable)
}

fn fanin_sort_value(fanin: Option<usize>) -> usize
{
    fanin.unwrap_or(0)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Default)]
    struct MockNetwork
    {
        primary_inputs: HashSet<usize>,
        fanout_counts: HashMap<usize, usize>,
        constants: Vec<bool>,
        patches: Vec<RedundancyPatch>,
        sweep_count: usize,
    }

    impl MockNetwork
    {
        fn with_primary_input(mut self, node: usize) -> Self
        {
            self.primary_inputs.insert(node);
            self
        }

        fn with_fanouts(mut self, node: usize, fanout_count: usize) -> Self
        {
            self.fanout_counts.insert(node, fanout_count);
            self
        }
    }

    impl RedundancyNetwork for MockNetwork
    {
        fn is_primary_input(&self, node: usize) -> bool
        {
            self.primary_inputs.contains(&node)
        }

        fn fanout_count(&self, node: usize) -> usize
        {
            self.fanout_counts.get(&node).copied().unwrap_or(0)
        }

        fn add_constant(&mut self, value: bool) -> usize
        {
            self.constants.push(value);
            10_000 + self.constants.len() - 1
        }

        fn patch_all_fanouts(&mut self, node: usize, replacement: usize)
        {
            self.patches
                .push(RedundancyPatch::AllFanouts { node, replacement });
        }

        fn patch_fanin(&mut self, node: usize, fanin: usize, replacement: usize)
        {
            self.patches.push(RedundancyPatch::Fannin {
                node,
                fanin,
                replacement,
            });
        }

        fn sweep(&mut self)
        {
            self.sweep_count += 1;
        }
    }

    #[test]
    fn fault_pattern_order_matches_c_compare_priority()
    {
        let low_node = FaultPattern::new(1, Some(9), FaultValue::StuckAtOne);
        let high_node = FaultPattern::new(2, None, FaultValue::StuckAtZero);
        let low_fanin = FaultPattern::new(4, Some(3), FaultValue::StuckAtOne);
        let high_fanin = FaultPattern::new(4, Some(5), FaultValue::StuckAtZero);
        let low_value = FaultPattern::new(4, Some(5), FaultValue::StuckAtZero);
        let high_value = FaultPattern::new(4, Some(5), FaultValue::StuckAtOne);

        assert_eq!(fault_pattern_compare(&low_node, &high_node), Ordering::Less);
        assert_eq!(fault_pattern_compare(&low_fanin, &high_fanin), Ordering::Less);
        assert_eq!(fault_pattern_compare(&low_value, &high_value), Ordering::Less);
        assert_eq!(fault_pattern_compare(&high_value, &low_value), Ordering::Greater);
    }

    #[test]
    fn fault_pattern_hash_matches_c_sum_formula()
    {
        let pattern = FaultPattern::new(0x1000, Some(0x40), FaultValue::StuckAtOne);

        assert_eq!(fault_pattern_hash(&pattern, 97), (0x1000 + 0x40 + 2) % 97);
        assert_eq!(
            fault_pattern_hash(
                &FaultPattern::new(7, None, FaultValue::StuckAtZero),
                11,
            ),
            8
        );
    }

    #[test]
    fn save_fault_pattern_inserts_previous_fault_and_updates_sequence()
    {
        let pattern = FaultPattern::new(7, Some(3), FaultValue::StuckAtZero);
        let mut table = FaultPatternTable::new();

        assert_eq!(table.save_fault_pattern(pattern, "first"), None);
        assert_eq!(table.sequence_for(&pattern), Some(&"first"));
        assert!(table.has_previous_fault(&pattern));
        assert_eq!(table.previous_fault_count(), 1);

        assert_eq!(table.save_fault_pattern(pattern, "second"), Some("first"));
        assert_eq!(table.sequence_for(&pattern), Some(&"second"));
        assert_eq!(table.len(), 1);
        assert_eq!(table.previous_fault_count(), 1);
    }

    #[test]
    fn aborted_fault_table_defers_each_reset_fault_once()
    {
        let first = FaultPattern::new(1, None, FaultValue::StuckAtZero);
        let second = FaultPattern::new(2, None, FaultValue::StuckAtOne);
        let mut table = AbortedFaultTable::new();

        assert!(table.record_first_abort(first));
        assert!(table.record_later_abort(second));
        assert_eq!(table.retry_flag(&first), Some(false));
        assert_eq!(table.retry_flag(&second), Some(true));

        assert!(table.should_defer_once(&first));
        assert!(!table.should_defer_once(&first));
        assert!(!table.should_defer_once(&second));

        table.reset_retry_flags();

        assert_eq!(table.retry_flag(&first), Some(false));
        assert_eq!(table.retry_flag(&second), Some(false));
        assert!(table.should_defer_once(&second));
    }

    #[test]
    fn dangling_primary_input_redundancy_is_reported_as_seen_before()
    {
        let mut network = MockNetwork::default().with_primary_input(3);
        let fault = RedundantFault::new(
            FaultPattern::new(3, None, FaultValue::StuckAtZero),
            RedundancyType::Control,
        );

        let result = remove_redundancy(&mut network, &fault);

        assert_eq!(
            result,
            RedundancyPatchResult {
                pi_seen_before: true,
                output_functions_changed: false,
                patch: RedundancyPatch::None,
            }
        );
        assert!(network.constants.is_empty());
        assert!(network.patches.is_empty());
        assert_eq!(network.sweep_count, 0);
    }

    #[test]
    fn node_fault_patches_all_fanouts_to_constant()
    {
        let mut network = MockNetwork::default().with_fanouts(5, 2);
        let fault = RedundantFault::new(
            FaultPattern::new(5, None, FaultValue::StuckAtOne),
            RedundancyType::Control,
        );

        let result = remove_redundancy(&mut network, &fault);

        assert_eq!(network.constants, vec![true]);
        assert_eq!(
            result.patch,
            RedundancyPatch::AllFanouts {
                node: 5,
                replacement: 10_000,
            }
        );
        assert_eq!(network.patches, vec![result.patch]);
        assert!(!result.output_functions_changed);
        assert_eq!(network.sweep_count, 1);
    }

    #[test]
    fn fanin_fault_patches_one_input_and_marks_observe_change()
    {
        let mut network = MockNetwork::default();
        let fault = RedundantFault::new(
            FaultPattern::new(9, Some(4), FaultValue::StuckAtZero),
            RedundancyType::Observe,
        );

        let result = remove_redundancy(&mut network, &fault);

        assert_eq!(network.constants, vec![false]);
        assert_eq!(
            result.patch,
            RedundancyPatch::Fannin {
                node: 9,
                fanin: 4,
                replacement: 10_000,
            }
        );
        assert!(result.output_functions_changed);
        assert_eq!(network.sweep_count, 1);
    }

    #[test]
    fn network_bound_entry_reports_missing_atpg_engine()
    {
        let mut network = ();

        assert_eq!(
            redundancy_removal_network_bound(&mut network, &RedundancyRemovalOptions::default()),
            Err(RedundancyRemovalError::AtpgEngineUnavailable)
        );
    }
}
