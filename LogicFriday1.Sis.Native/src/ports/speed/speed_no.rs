//! Native Rust port for `sis/speed/speed_no.c`.
//!
//! The original file drives speed decomposition by creating a temporary network
//! from one node, repeatedly trying alternative decompositions, keeping the
//! network with the smallest output arrival time, optionally inserting
//! inverters, then converting the result back to an array of nodes. This module
//! exposes that flow through a native Rust backend trait so graph implementations
//! can plug in without preserving per-file C ABI entry points.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Buffer,
    Inverter,
    PrimaryOutput,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub fn worst(self) -> f64 {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedDecompOptions {
    pub coeff: f64,
    pub model: DelayModel,
    pub num_tries: usize,
    pub debug: bool,
    pub add_inv: bool,
}

impl Default for SpeedDecompOptions {
    fn default() -> Self {
        Self {
            coeff: 0.0,
            model: DelayModel::Unit,
            num_tries: 1,
            debug: false,
            add_inv: false,
        }
    }
}

impl SpeedDecompOptions {
    pub fn for_interface(coeff: f64, model: DelayModel) -> Self {
        Self {
            coeff,
            model,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    UnitFanout,
    Mapped,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecompositionAttempt {
    pub attempt_index: usize,
    pub output_arrival: DelayTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BestAttempt {
    pub attempt_index: usize,
    pub delay: f64,
}

pub fn select_best_attempt(attempts: &[DecompositionAttempt]) -> Result<BestAttempt, SpeedNoError> {
    let mut best: Option<BestAttempt> = None;

    for attempt in attempts {
        let delay = attempt.output_arrival.worst();
        if best.as_ref().is_none_or(|current| delay < current.delay) {
            best = Some(BestAttempt {
                attempt_index: attempt.attempt_index,
                delay,
            });
        }
    }

    best.ok_or(SpeedNoError::NoAttempts)
}

pub fn format_attempt_trace(attempts: &[DecompositionAttempt]) -> String {
    if attempts.len() <= 1 {
        return String::new();
    }

    let best = select_best_attempt(attempts).ok();
    let mut trace = String::new();
    for attempt in attempts {
        trace.push_str(&format!(
            "{} => {:.2}\t",
            attempt.attempt_index,
            attempt.output_arrival.worst()
        ));
    }
    if let Some(best) = best {
        trace.push_str(&format!(" BEST is {}\n", best.attempt_index));
    }
    trace
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedDecompOutcome<T> {
    pub nodes: T,
    pub attempts: Vec<DecompositionAttempt>,
    pub best_attempt: BestAttempt,
    pub debug_trace: String,
}

pub trait SpeedDecompBackend {
    type Network: Clone;
    type Node: Clone;
    type Output;

    fn create_network_from_node(
        &mut self,
        node_name: &str,
        options: &SpeedDecompOptions,
        delay_flag: bool,
    ) -> Result<Self::Network, SpeedNoError>;

    fn delay_trace(
        &mut self,
        network: &mut Self::Network,
        options: &SpeedDecompOptions,
    ) -> Result<(), SpeedNoError>;

    fn first_internal_node(&mut self, network: &Self::Network) -> Result<Self::Node, SpeedNoError>;

    fn set_library_acceleration(
        &mut self,
        options: &mut SpeedDecompOptions,
        enabled: bool,
    ) -> Result<(), SpeedNoError>;

    fn collapse_network(&mut self, network: &mut Self::Network) -> Result<(), SpeedNoError>;

    fn decompose_network(
        &mut self,
        network: &mut Self::Network,
        node: &Self::Node,
        options: &SpeedDecompOptions,
        attempt_index: usize,
    ) -> Result<(), SpeedNoError>;

    fn primary_output_arrival(
        &mut self,
        network: &Self::Network,
        options: &SpeedDecompOptions,
    ) -> Result<DelayTime, SpeedNoError>;

    fn add_inverters(&mut self, network: &mut Self::Network) -> Result<(), SpeedNoError>;

    fn network_to_nodes(
        &mut self,
        network: Self::Network,
        original_node_name: &str,
    ) -> Result<Self::Output, SpeedNoError>;
}

pub fn speed_decomp<B>(
    node_name: &str,
    options: &SpeedDecompOptions,
    delay_flag: bool,
    backend: &mut B,
) -> Result<SpeedDecompOutcome<B::Output>, SpeedNoError>
where
    B: SpeedDecompBackend,
{
    if options.num_tries == 0 {
        return Err(SpeedNoError::NoAttempts);
    }

    let mut working_options = options.clone();
    let mut network = backend.create_network_from_node(node_name, &working_options, delay_flag)?;
    backend.delay_trace(&mut network, &working_options)?;
    let node = backend.first_internal_node(&network)?;
    backend.set_library_acceleration(&mut working_options, true)?;

    let mut attempts = Vec::with_capacity(working_options.num_tries);
    let mut best_network = None;
    let mut best_attempt = None;

    for attempt_index in 0..working_options.num_tries {
        backend.collapse_network(&mut network)?;
        backend.decompose_network(&mut network, &node, &working_options, attempt_index)?;

        let output_arrival = backend.primary_output_arrival(&network, &working_options)?;
        let delay = output_arrival.worst();
        attempts.push(DecompositionAttempt {
            attempt_index,
            output_arrival,
        });

        if best_attempt
            .as_ref()
            .is_none_or(|current: &BestAttempt| delay < current.delay)
        {
            best_network = Some(network.clone());
            best_attempt = Some(BestAttempt {
                attempt_index,
                delay,
            });
        }
    }

    let mut network = best_network.ok_or(SpeedNoError::NoAttempts)?;
    let best_attempt = best_attempt.ok_or(SpeedNoError::NoAttempts)?;
    backend.set_library_acceleration(&mut working_options, false)?;

    if working_options.add_inv {
        backend.add_inverters(&mut network)?;
        backend.delay_trace(&mut network, &working_options)?;
    }

    let debug_trace = if working_options.debug && working_options.num_tries > 1 {
        format_attempt_trace(&attempts)
    } else {
        String::new()
    };
    let nodes = backend.network_to_nodes(network, node_name)?;

    Ok(SpeedDecompOutcome {
        nodes,
        attempts,
        best_attempt,
        debug_trace,
    })
}

pub fn speed_decomp_interface_with_backend<B>(
    node_name: &str,
    coeff: f64,
    model: DelayModel,
    backend: &mut B,
) -> Result<SpeedDecompOutcome<B::Output>, SpeedNoError>
where
    B: SpeedDecompBackend,
{
    let options = SpeedDecompOptions::for_interface(coeff, model);
    speed_decomp(node_name, &options, false, backend)
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseNode {
    pub id: usize,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub fanouts: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhaseAction {
    CollapseIntoFanout { node: usize, fanout: usize },
    DeleteIfFanoutless { node: usize },
}

pub fn phase_adjustment_actions(nodes: &[PhaseNode]) -> Vec<PhaseAction> {
    let mut actions = Vec::new();

    for node in nodes {
        if node.kind != NodeKind::Internal {
            continue;
        }
        if !matches!(node.function, NodeFunction::Buffer | NodeFunction::Inverter) {
            continue;
        }

        let mut collapsible_fanouts = 0usize;
        for fanout in &node.fanouts {
            if nodes
                .iter()
                .find(|candidate| candidate.id == *fanout)
                .is_some_and(|candidate| candidate.function != NodeFunction::PrimaryOutput)
            {
                actions.push(PhaseAction::CollapseIntoFanout {
                    node: node.id,
                    fanout: *fanout,
                });
                collapsible_fanouts += 1;
            }
        }

        if node.fanouts.is_empty() || collapsible_fanouts == node.fanouts.len() {
            actions.push(PhaseAction::DeleteIfFanoutless { node: node.id });
        }
    }

    actions
}

pub fn speed_decomp_interface(
    _node_name: &str,
    _options: &SpeedDecompOptions,
) -> Result<(), SpeedNoError> {
    Err(SpeedNoError::MissingDependency(
        "speed_decomp_interface requires a native speed decomposition backend",
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpeedNoError {
    NoAttempts,
    MissingInternalNode,
    MissingDependency(&'static str),
    BackendFailure {
        operation: &'static str,
        message: String,
    },
}

impl fmt::Display for SpeedNoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAttempts => write!(f, "speed decomposition had no attempts to compare"),
            Self::MissingInternalNode => {
                write!(f, "speed decomposition network has no internal node")
            }
            Self::MissingDependency(message) => write!(f, "{message}"),
            Self::BackendFailure { operation, message } => {
                write!(f, "{operation} failed: {message}")
            }
        }
    }
}

impl Error for SpeedNoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct MockNetwork {
        id: usize,
        collapsed: usize,
        decomposed_attempt: Option<usize>,
        inverters_added: bool,
    }

    #[derive(Default)]
    struct MockBackend {
        arrivals: Vec<DelayTime>,
        events: Vec<String>,
        next_network_id: usize,
    }

    impl MockBackend {
        fn new(arrivals: Vec<DelayTime>) -> Self {
            Self {
                arrivals,
                events: Vec::new(),
                next_network_id: 1,
            }
        }
    }

    impl SpeedDecompBackend for MockBackend {
        type Network = MockNetwork;
        type Node = &'static str;
        type Output = MockNetwork;

        fn create_network_from_node(
            &mut self,
            node_name: &str,
            _options: &SpeedDecompOptions,
            delay_flag: bool,
        ) -> Result<Self::Network, SpeedNoError> {
            self.events
                .push(format!("create:{node_name}:delay={delay_flag}"));
            let network = MockNetwork {
                id: self.next_network_id,
                collapsed: 0,
                decomposed_attempt: None,
                inverters_added: false,
            };
            self.next_network_id += 1;
            Ok(network)
        }

        fn delay_trace(
            &mut self,
            network: &mut Self::Network,
            _options: &SpeedDecompOptions,
        ) -> Result<(), SpeedNoError> {
            self.events.push(format!("trace:{}", network.id));
            Ok(())
        }

        fn first_internal_node(
            &mut self,
            _network: &Self::Network,
        ) -> Result<Self::Node, SpeedNoError> {
            self.events.push("first-internal".to_owned());
            Ok("internal")
        }

        fn set_library_acceleration(
            &mut self,
            _options: &mut SpeedDecompOptions,
            enabled: bool,
        ) -> Result<(), SpeedNoError> {
            self.events.push(format!("accl:{enabled}"));
            Ok(())
        }

        fn collapse_network(&mut self, network: &mut Self::Network) -> Result<(), SpeedNoError> {
            network.collapsed += 1;
            self.events
                .push(format!("collapse:{}:{}", network.id, network.collapsed));
            Ok(())
        }

        fn decompose_network(
            &mut self,
            network: &mut Self::Network,
            node: &Self::Node,
            _options: &SpeedDecompOptions,
            attempt_index: usize,
        ) -> Result<(), SpeedNoError> {
            network.decomposed_attempt = Some(attempt_index);
            self.events
                .push(format!("decompose:{node}:{attempt_index}"));
            Ok(())
        }

        fn primary_output_arrival(
            &mut self,
            network: &Self::Network,
            _options: &SpeedDecompOptions,
        ) -> Result<DelayTime, SpeedNoError> {
            let attempt = network.decomposed_attempt.unwrap();
            self.events.push(format!("arrival:{attempt}"));
            Ok(self.arrivals[attempt])
        }

        fn add_inverters(&mut self, network: &mut Self::Network) -> Result<(), SpeedNoError> {
            network.inverters_added = true;
            self.events.push(format!("add-inverters:{}", network.id));
            Ok(())
        }

        fn network_to_nodes(
            &mut self,
            network: Self::Network,
            original_node_name: &str,
        ) -> Result<Self::Output, SpeedNoError> {
            self.events.push(format!("to-nodes:{original_node_name}"));
            Ok(network)
        }
    }

    #[test]
    fn selects_lowest_max_rise_fall_delay_like_c_loop() {
        let attempts = vec![
            DecompositionAttempt {
                attempt_index: 0,
                output_arrival: DelayTime {
                    rise: 4.0,
                    fall: 7.0,
                },
            },
            DecompositionAttempt {
                attempt_index: 1,
                output_arrival: DelayTime {
                    rise: 5.0,
                    fall: 3.0,
                },
            },
            DecompositionAttempt {
                attempt_index: 2,
                output_arrival: DelayTime {
                    rise: 6.0,
                    fall: 6.5,
                },
            },
        ];

        assert_eq!(
            select_best_attempt(&attempts).unwrap(),
            BestAttempt {
                attempt_index: 1,
                delay: 5.0,
            }
        );
    }

    #[test]
    fn formats_debug_attempt_trace_with_best_index() {
        let attempts = vec![
            DecompositionAttempt {
                attempt_index: 0,
                output_arrival: DelayTime {
                    rise: 4.0,
                    fall: 7.0,
                },
            },
            DecompositionAttempt {
                attempt_index: 1,
                output_arrival: DelayTime {
                    rise: 5.0,
                    fall: 3.0,
                },
            },
        ];

        assert_eq!(
            format_attempt_trace(&attempts),
            "0 => 7.00\t1 => 5.00\t BEST is 1\n"
        );
    }

    #[test]
    fn speed_decomp_runs_attempt_loop_and_keeps_best_network() {
        let mut backend = MockBackend::new(vec![
            DelayTime {
                rise: 9.0,
                fall: 4.0,
            },
            DelayTime {
                rise: 3.0,
                fall: 6.0,
            },
            DelayTime {
                rise: 7.0,
                fall: 7.5,
            },
        ]);
        let options = SpeedDecompOptions {
            num_tries: 3,
            debug: true,
            add_inv: true,
            ..SpeedDecompOptions::default()
        };

        let outcome = speed_decomp("f", &options, true, &mut backend).unwrap();

        assert_eq!(
            outcome.best_attempt,
            BestAttempt {
                attempt_index: 1,
                delay: 6.0,
            }
        );
        assert_eq!(outcome.nodes.decomposed_attempt, Some(1));
        assert!(outcome.nodes.inverters_added);
        assert_eq!(
            outcome.debug_trace,
            "0 => 9.00\t1 => 6.00\t2 => 7.50\t BEST is 1\n"
        );
        assert_eq!(
            backend.events,
            vec![
                "create:f:delay=true",
                "trace:1",
                "first-internal",
                "accl:true",
                "collapse:1:1",
                "decompose:internal:0",
                "arrival:0",
                "collapse:1:2",
                "decompose:internal:1",
                "arrival:1",
                "collapse:1:3",
                "decompose:internal:2",
                "arrival:2",
                "accl:false",
                "add-inverters:1",
                "trace:1",
                "to-nodes:f",
            ]
        );
    }

    #[test]
    fn interface_helper_uses_default_options_without_fanin_delay_flag() {
        let mut backend = MockBackend::new(vec![DelayTime {
            rise: 2.0,
            fall: 3.0,
        }]);

        let outcome =
            speed_decomp_interface_with_backend("f", 0.75, DelayModel::Mapped, &mut backend)
                .unwrap();

        assert_eq!(outcome.best_attempt.delay, 3.0);
        assert_eq!(backend.events[0], "create:f:delay=false");
    }

    #[test]
    fn speed_decomp_rejects_zero_attempts() {
        let mut backend = MockBackend::new(Vec::new());
        let options = SpeedDecompOptions {
            num_tries: 0,
            ..SpeedDecompOptions::default()
        };

        assert_eq!(
            speed_decomp("f", &options, false, &mut backend).unwrap_err(),
            SpeedNoError::NoAttempts
        );
        assert!(backend.events.is_empty());
    }

    #[test]
    fn phase_adjustment_collapses_buffers_and_inverters_except_into_pos() {
        let nodes = vec![
            PhaseNode {
                id: 1,
                kind: NodeKind::Internal,
                function: NodeFunction::Buffer,
                fanouts: vec![2, 3],
            },
            PhaseNode {
                id: 2,
                kind: NodeKind::Internal,
                function: NodeFunction::Other,
                fanouts: vec![],
            },
            PhaseNode {
                id: 3,
                kind: NodeKind::PrimaryOutput,
                function: NodeFunction::PrimaryOutput,
                fanouts: vec![],
            },
            PhaseNode {
                id: 4,
                kind: NodeKind::Internal,
                function: NodeFunction::Inverter,
                fanouts: vec![],
            },
        ];

        assert_eq!(
            phase_adjustment_actions(&nodes),
            vec![
                PhaseAction::CollapseIntoFanout { node: 1, fanout: 2 },
                PhaseAction::DeleteIfFanoutless { node: 4 },
            ]
        );
    }

    #[test]
    fn network_bound_entry_point_reports_missing_dependencies() {
        let options = SpeedDecompOptions {
            coeff: 0.0,
            model: DelayModel::UnitFanout,
            num_tries: 1,
            debug: false,
            add_inv: false,
        };

        assert_eq!(
            speed_decomp_interface("n1", &options),
            Err(SpeedNoError::MissingDependency(
                "speed_decomp_interface requires a native speed decomposition backend",
            ))
        );
    }

    #[test]
    fn no_legacy_c_abi_or_tracking_tokens_are_present() {
        let source = include_str!("speed_no.rs");

        assert!(!source.contains(&["no", "_mangle"].concat()));
        assert!(!source.contains(&["extern", " \"", "C", "\""].concat()));
        assert!(!source.contains(&["REQUIRED", "_"].concat()));
        assert!(!source.contains(&["Port", "Dependency"].concat()));
        assert!(!source.contains(&["source", "_file"].concat()));
        assert!(!source.contains(&["be", "ad"].concat()));
    }
}
