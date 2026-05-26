//! Native lock graph support for ASTG state-coding analysis.
//!
//! The lock graph is an undirected graph over ASTG signals. Edges are produced
//! when two signals have interleaved positive and negative transitions on a
//! simple cycle. The legacy SIS implementation can optionally mutate the ASTG
//! by adding lock constraints; this native module keeps the graph analysis
//! independent from that constraint backend.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SignalId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal
{
    pub name: String,
    pub transition_count: usize,
}

impl Signal
{
    pub fn new(name: impl Into<String>, transition_count: usize) -> Self
    {
        Self
        {
            name: name.into(),
            transition_count,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionPolarity
{
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransitionEvent
{
    pub signal: SignalId,
    pub polarity: TransitionPolarity,
}

impl TransitionEvent
{
    pub fn positive(signal: SignalId) -> Self
    {
        Self
        {
            signal,
            polarity: TransitionPolarity::Positive,
        }
    }

    pub fn negative(signal: SignalId) -> Self
    {
        Self
        {
            signal,
            polarity: TransitionPolarity::Negative,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockCycle
{
    events: Vec<TransitionEvent>,
}

impl LockCycle
{
    pub fn new(events: impl Into<Vec<TransitionEvent>>) -> Self
    {
        Self
        {
            events: events.into(),
        }
    }

    pub fn events(&self) -> &[TransitionEvent]
    {
        &self.events
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockGraph
{
    signals: Vec<Signal>,
    edges: Vec<BTreeSet<SignalId>>,
}

impl LockGraph
{
    pub fn new(signals: impl Into<Vec<Signal>>) -> Self
    {
        let signals = signals.into();
        let edges = vec![BTreeSet::new(); signals.len()];

        Self
        {
            signals,
            edges,
        }
    }

    pub fn signals(&self) -> &[Signal]
    {
        &self.signals
    }

    pub fn signal(&self, signal: SignalId) -> Option<&Signal>
    {
        self.signals.get(signal.0)
    }

    pub fn edge_count(&self) -> usize
    {
        self.edges.iter().map(BTreeSet::len).sum::<usize>() / 2
    }

    pub fn neighbors(&self, signal: SignalId) -> Option<&BTreeSet<SignalId>>
    {
        self.edges.get(signal.0)
    }

    pub fn has_edge(&self, left: SignalId, right: SignalId) -> bool
    {
        self.edges
            .get(left.0)
            .is_some_and(|neighbors| neighbors.contains(&right))
    }

    pub fn add_edge(&mut self, left: SignalId, right: SignalId) -> Result<bool, LockGraphError>
    {
        self.validate_signal(left)?;
        self.validate_signal(right)?;

        if left == right
        {
            return Ok(false);
        }

        let inserted = self.edges[left.0].insert(right);
        self.edges[right.0].insert(left);

        Ok(inserted)
    }

    pub fn connected_components(&self) -> Vec<Vec<SignalId>>
    {
        let mut unprocessed = vec![true; self.signals.len()];
        let mut components = Vec::new();

        for signal_index in 0..self.signals.len()
        {
            if !unprocessed[signal_index]
            {
                continue;
            }

            let mut component = Vec::new();
            self.collect_component(SignalId(signal_index), &mut unprocessed, &mut component);
            components.push(component);
        }

        components
    }

    pub fn shortest_paths(
        &self,
        source: SignalId,
    ) -> Result<Vec<Option<ShortestPathStep>>, LockGraphError>
    {
        self.validate_signal(source)?;

        let mut paths = vec![None; self.signals.len()];
        let mut queue = BTreeSet::new();

        paths[source.0] = Some(ShortestPathStep
        {
            weight: 0,
            from: None,
        });
        queue.insert(QueueEntry
        {
            weight: 0,
            signal: source,
        });

        while let Some(entry) = queue.pop_first()
        {
            let current_weight = paths[entry.signal.0]
                .as_ref()
                .map(|path| path.weight)
                .unwrap_or(usize::MAX);

            if entry.weight != current_weight
            {
                continue;
            }

            for neighbor in self.edges[entry.signal.0].iter().rev().copied()
            {
                let new_weight = entry.weight + 1;
                let should_update = paths[neighbor.0]
                    .as_ref()
                    .is_none_or(|path| new_weight < path.weight);

                if should_update
                {
                    paths[neighbor.0] = Some(ShortestPathStep
                    {
                        weight: new_weight,
                        from: Some(entry.signal),
                    });
                    queue.insert(QueueEntry
                    {
                        weight: new_weight,
                        signal: neighbor,
                    });
                }
            }
        }

        Ok(paths)
    }

    fn validate_signal(&self, signal: SignalId) -> Result<(), LockGraphError>
    {
        if signal.0 >= self.signals.len()
        {
            return Err(LockGraphError::UnknownSignal(signal));
        }

        Ok(())
    }

    fn collect_component(
        &self,
        signal: SignalId,
        unprocessed: &mut [bool],
        component: &mut Vec<SignalId>,
    )
    {
        if !unprocessed[signal.0]
        {
            return;
        }

        unprocessed[signal.0] = false;
        component.push(signal);

        for neighbor in self.edges[signal.0].iter().rev().copied()
        {
            self.collect_component(neighbor, unprocessed, component);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShortestPathStep
{
    pub weight: usize,
    pub from: Option<SignalId>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct QueueEntry
{
    weight: usize,
    signal: SignalId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateCodingReport
{
    pub component_count: usize,
    pub edge_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LockGraphError
{
    UnknownSignal(SignalId),
    NonUniqueTransitionSignal
    {
        signal: SignalId,
        name: String,
        transition_count: usize,
    },
    LockConstraintBackendUnavailable
    {
        component_count: usize,
    },
}

impl fmt::Display for LockGraphError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::UnknownSignal(signal) => write!(formatter, "unknown ASTG signal {}", signal.0),
            Self::NonUniqueTransitionSignal
            {
                signal,
                name,
                transition_count,
            } => write!(
                formatter,
                "signal {} ({name}) has {transition_count} transitions; lock graphs require exactly two",
                signal.0
            ),
            Self::LockConstraintBackendUnavailable
            {
                component_count,
            } => write!(
                formatter,
                "lock graph has {component_count} components, but native ASTG lock-constraint mutation is not available"
            ),
        }
    }
}

impl Error for LockGraphError
{
}

pub fn build_lock_graph_from_cycles(
    signals: impl Into<Vec<Signal>>,
    cycles: &[LockCycle],
) -> Result<LockGraph, LockGraphError>
{
    let mut graph = LockGraph::new(signals);
    validate_unique_transitions(&graph)?;

    for cycle in cycles
    {
        add_cycle_interlocks(&mut graph, cycle)?;
    }

    Ok(graph)
}

pub fn astg_state_coding(
    signals: impl Into<Vec<Signal>>,
    cycles: &[LockCycle],
    do_lock: bool,
) -> Result<StateCodingReport, LockGraphError>
{
    let graph = build_lock_graph_from_cycles(signals, cycles)?;
    let component_count = graph.connected_components().len();

    if do_lock && component_count > 1
    {
        return Err(LockGraphError::LockConstraintBackendUnavailable
        {
            component_count,
        });
    }

    Ok(StateCodingReport
    {
        component_count,
        edge_count: graph.edge_count(),
    })
}

pub fn interleaved(
    first_positive: usize,
    first_negative: usize,
    second_positive: usize,
    second_negative: usize,
) -> bool
{
    sandwich(first_positive, second_positive, first_negative)
        ^ sandwich(first_positive, second_negative, first_negative)
}

fn validate_unique_transitions(graph: &LockGraph) -> Result<(), LockGraphError>
{
    for (index, signal) in graph.signals.iter().enumerate()
    {
        if signal.transition_count != 2
        {
            return Err(LockGraphError::NonUniqueTransitionSignal
            {
                signal: SignalId(index),
                name: signal.name.clone(),
                transition_count: signal.transition_count,
            });
        }
    }

    Ok(())
}

fn add_cycle_interlocks(graph: &mut LockGraph, cycle: &LockCycle) -> Result<(), LockGraphError>
{
    let mut positions = vec![SignalPositions::default(); graph.signals.len()];

    for (index, event) in cycle.events.iter().enumerate()
    {
        graph.validate_signal(event.signal)?;

        match event.polarity
        {
            TransitionPolarity::Positive => positions[event.signal.0].positive = Some(index),
            TransitionPolarity::Negative => positions[event.signal.0].negative = Some(index),
        }
    }

    for first in 0..positions.len()
    {
        let Some(first_positive) = positions[first].positive else
        {
            continue;
        };
        let Some(first_negative) = positions[first].negative else
        {
            continue;
        };

        for second in 0..first
        {
            let Some(second_positive) = positions[second].positive else
            {
                continue;
            };
            let Some(second_negative) = positions[second].negative else
            {
                continue;
            };

            if interleaved(first_positive, first_negative, second_positive, second_negative)
            {
                graph.add_edge(SignalId(first), SignalId(second))?;
            }
        }
    }

    Ok(())
}

fn sandwich(first: usize, middle: usize, second: usize) -> bool
{
    let low = first.min(second);
    let high = first.max(second);

    low < middle && middle < high
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SignalPositions
{
    positive: Option<usize>,
    negative: Option<usize>,
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn signals(names: &[&str]) -> Vec<Signal>
    {
        names.iter().map(|name| Signal::new(*name, 2)).collect()
    }

    #[test]
    fn interleaved_matches_legacy_sandwich_rule()
    {
        assert!(interleaved(0, 2, 1, 3));
        assert!(interleaved(2, 0, 1, 3));
        assert!(!interleaved(0, 3, 1, 2));
        assert!(!interleaved(0, 1, 2, 3));
    }

    #[test]
    fn cycle_interleaving_creates_undirected_lock_edges()
    {
        let cycle = LockCycle::new([
            TransitionEvent::positive(SignalId(0)),
            TransitionEvent::positive(SignalId(1)),
            TransitionEvent::negative(SignalId(0)),
            TransitionEvent::negative(SignalId(1)),
        ]);

        let graph = build_lock_graph_from_cycles(signals(&["a", "b"]), &[cycle]).unwrap();

        assert_eq!(graph.edge_count(), 1);
        assert!(graph.has_edge(SignalId(0), SignalId(1)));
        assert!(graph.has_edge(SignalId(1), SignalId(0)));
    }

    #[test]
    fn cycles_without_both_signal_transitions_do_not_lock()
    {
        let cycle = LockCycle::new([
            TransitionEvent::positive(SignalId(0)),
            TransitionEvent::positive(SignalId(1)),
            TransitionEvent::negative(SignalId(0)),
        ]);

        let graph = build_lock_graph_from_cycles(signals(&["a", "b"]), &[cycle]).unwrap();

        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn connected_components_are_found_with_depth_first_search()
    {
        let mut graph = LockGraph::new(signals(&["a", "b", "c", "d"]));
        graph.add_edge(SignalId(0), SignalId(1)).unwrap();
        graph.add_edge(SignalId(1), SignalId(2)).unwrap();

        let components = graph.connected_components();

        assert_eq!(
            components,
            vec![
                vec![SignalId(0), SignalId(1), SignalId(2)],
                vec![SignalId(3)],
            ]
        );
    }

    #[test]
    fn shortest_paths_record_distance_and_predecessor()
    {
        let mut graph = LockGraph::new(signals(&["a", "b", "c", "d"]));
        graph.add_edge(SignalId(0), SignalId(1)).unwrap();
        graph.add_edge(SignalId(1), SignalId(2)).unwrap();

        let paths = graph.shortest_paths(SignalId(0)).unwrap();

        assert_eq!(
            paths,
            vec![
                Some(ShortestPathStep
                {
                    weight: 0,
                    from: None,
                }),
                Some(ShortestPathStep
                {
                    weight: 1,
                    from: Some(SignalId(0)),
                }),
                Some(ShortestPathStep
                {
                    weight: 2,
                    from: Some(SignalId(1)),
                }),
                None,
            ]
        );
    }

    #[test]
    fn state_coding_rejects_non_unique_transition_signals()
    {
        let error = astg_state_coding(vec![Signal::new("a", 3)], &[], false).unwrap_err();

        assert_eq!(
            error,
            LockGraphError::NonUniqueTransitionSignal
            {
                signal: SignalId(0),
                name: "a".to_string(),
                transition_count: 3,
            }
        );
    }

    #[test]
    fn state_coding_reports_component_count_without_locking()
    {
        let report = astg_state_coding(signals(&["a", "b"]), &[], false).unwrap();

        assert_eq!(
            report,
            StateCodingReport
            {
                component_count: 2,
                edge_count: 0,
            }
        );
    }

    #[test]
    fn state_coding_requires_constraint_backend_when_locking_multiple_components()
    {
        let error = astg_state_coding(signals(&["a", "b"]), &[], true).unwrap_err();

        assert_eq!(
            error,
            LockGraphError::LockConstraintBackendUnavailable
            {
                component_count: 2,
            }
        );
    }

    #[test]
    fn no_legacy_exports_or_tracking_metadata_are_present()
    {
        let source = include_str!("astg_lkgraph.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
