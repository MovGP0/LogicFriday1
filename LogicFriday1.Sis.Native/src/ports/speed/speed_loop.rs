//! Native Rust planning port for `LogicSynthesis/sis/speed/speed_loop.c`.
//!
//! The C file owns the outer optimization loop: initialize speed parameters,
//! trace the initial delay, repeatedly run either `new_speed` or
//! `speed_up_network`, accept only improved networks, try mapped-gate downsizing
//! as a perturbation, and vary collapse distance for the old script mode. The
//! network mutation, delay tracing, and SIS signal-handler paths are still
//! blocked on other porting beads, so this module exposes the loop/script
//! decision rules and trace formatting as native Rust data structures.

use std::error::Error;
use std::fmt;

pub const DEFAULT_SPEED_THRESH: f64 = 0.5;
pub const DEFAULT_SPEED_COEFF: f64 = 0.0;
pub const DEFAULT_SPEED_DIST: i32 = 3;
pub const NSP_EPSILON: f64 = 1.0e-6;

pub const REQUIRED_PORT_BEADS: &[&str] = &[
    "LogicFriday1-8j8.2.6.133", // delay/delay.c: delay_trace and delay_latest_output
    "LogicFriday1-8j8.2.6.258", // map/libutil.c: lib_network_is_mapped
    "LogicFriday1-8j8.2.6.299", // network/net_seq.c: network duplication/free traversal substrate
    "LogicFriday1-8j8.2.6.305", // network/network_util.c: network_num_pi, network_name
    "LogicFriday1-8j8.2.6.455", // simplify/simp.c: com_redundancy_removal
    "LogicFriday1-8j8.2.6.465", // speed/com_speed.c: speed_fill_options
    "LogicFriday1-8j8.2.6.467", // speed/new_speed.c: new_speed loop body
    "LogicFriday1-8j8.2.6.468", // speed/new_wght_util.c: new-speed local transform setup
    "LogicFriday1-8j8.2.6.474", // speed/speed_delay.c: speed_set_delay_data
    "LogicFriday1-8j8.2.6.480", // speed/speed_util.c: SP_GET_PERFORMANCE helpers
    "LogicFriday1-8j8.2.6.481", // speed/speedup.c: speed_up_network
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    Library,
    UnitFanout,
    Mapped,
    Tdc,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedRegion {
    AlongCriticalPath,
    TransitiveFanin,
    Compromise,
    OnlyTree,
    Unknown,
}

impl SpeedRegion {
    pub fn c_method_name(self) -> &'static str {
        match self {
            Self::AlongCriticalPath => "CRITICAL",
            Self::TransitiveFanin => "TRANSITIVE",
            Self::Compromise => "COMPROMISE",
            Self::OnlyTree => "TREE",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformSelection {
    BestBenefit,
    BestBangForBuck,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalTransform {
    pub name: String,
    pub enabled: bool,
}

impl LocalTransform {
    pub fn enabled(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
        }
    }

    pub fn disabled(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedLoopOptions {
    pub threshold: f64,
    pub coeff: f64,
    pub dist: i32,
    pub model: DelayModel,
    pub trace: bool,
    pub req_times_set: bool,
    pub interactive: bool,
    pub new_mode: bool,
    pub red_removal: bool,
    pub del_crit_cubes: bool,
    pub region: SpeedRegion,
    pub transform_selection: TransformSelection,
    pub local_transforms: Vec<LocalTransform>,
}

impl Default for SpeedLoopOptions {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_SPEED_THRESH,
            coeff: DEFAULT_SPEED_COEFF,
            dist: DEFAULT_SPEED_DIST,
            model: DelayModel::Unit,
            trace: false,
            req_times_set: false,
            interactive: false,
            new_mode: false,
            red_removal: false,
            del_crit_cubes: true,
            region: SpeedRegion::AlongCriticalPath,
            transform_selection: TransformSelection::BestBenefit,
            local_transforms: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedLoopInterfaceSetup {
    pub options: SpeedLoopOptions,
    pub library_acceleration: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetworkPerformance {
    pub value: f64,
    pub area: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedPerformance {
    pub value: f64,
    pub area: f64,
    pub node_name: String,
}

impl NamedPerformance {
    pub fn new(value: f64, area: f64, node_name: impl Into<String>) -> Self {
        Self {
            value,
            area,
            node_name: node_name.into(),
        }
    }
}

impl From<&NamedPerformance> for NetworkPerformance {
    fn from(performance: &NamedPerformance) -> Self {
        Self {
            value: performance.value,
            area: performance.area,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformRun {
    NewSpeed,
    LegacySpeedUpNetwork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceptedNetwork {
    FirstTransform,
    DownsizedTransformed,
    DownsizedSaved,
    SecondTransform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoopStopReason {
    TrivialNetwork,
    TimingConstraintsMet,
    NewSpeedMadeNoChange,
    NoImprovement,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoopOutcome {
    Accept {
        source: AcceptedNetwork,
        timing_constraints_met: bool,
    },
    Stop(LoopStopReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoopAction {
    TraceInitialDelay,
    MeasureCurrentPerformance,
    DuplicateCurrentNetwork,
    RunRedundancyRemoval,
    RunNewSpeed,
    RunLegacySpeedUpNetwork,
    TraceTransformedDelay,
    MeasureTransformedPerformance,
    DownsizeTransformedMappedNetwork,
    DuplicateSavedNetwork,
    DownsizeSavedMappedNetwork,
    RetryNewSpeed,
    RetryLegacySpeedUpNetwork,
    RestoreSignalHandler,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IterationObservation {
    pub primary_input_count: usize,
    pub current: NetworkPerformance,
    pub transformed: NetworkPerformance,
    pub mapped_network: bool,
    pub new_speed_status: Option<i32>,
    pub downsized_transformed: Option<NetworkPerformance>,
    pub downsized_saved: Option<NetworkPerformance>,
    pub second_transform: Option<NetworkPerformance>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoopIterationPlan {
    pub actions: Vec<LoopAction>,
    pub outcome: LoopOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedLoopDependency {
    DelayTrace,
    DelayDataDefaults,
    NetworkInspection,
    NetworkDuplication,
    NetworkMutation,
    RedundancyRemoval,
    PerformanceLookup,
    NewSpeed,
    LegacySpeedUpNetwork,
    MappedNetworkDetection,
    MappedAreaRecovery,
    SignalHandler,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedLoopError {
    MissingDependency(SpeedLoopDependency),
}

impl fmt::Display for SpeedLoopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDependency(dependency) => match dependency {
                SpeedLoopDependency::DelayTrace => {
                    write!(f, "SIS delay tracing is not ported to Rust yet")
                }
                SpeedLoopDependency::DelayDataDefaults => {
                    write!(f, "SIS speed delay-data setup is not ported to Rust yet")
                }
                SpeedLoopDependency::NetworkInspection => {
                    write!(f, "SIS network inspection is not ported to Rust yet")
                }
                SpeedLoopDependency::NetworkDuplication => {
                    write!(
                        f,
                        "SIS network duplication/free APIs are not ported to Rust yet"
                    )
                }
                SpeedLoopDependency::NetworkMutation => {
                    write!(f, "SIS network replacement APIs are not ported to Rust yet")
                }
                SpeedLoopDependency::RedundancyRemoval => {
                    write!(f, "SIS redundancy removal is not ported to Rust yet")
                }
                SpeedLoopDependency::PerformanceLookup => {
                    write!(f, "SIS speed performance lookup is not ported to Rust yet")
                }
                SpeedLoopDependency::NewSpeed => {
                    write!(f, "SIS new_speed is not ported to Rust yet")
                }
                SpeedLoopDependency::LegacySpeedUpNetwork => {
                    write!(f, "SIS speed_up_network is not ported to Rust yet")
                }
                SpeedLoopDependency::MappedNetworkDetection => {
                    write!(f, "SIS mapped-network detection is not ported to Rust yet")
                }
                SpeedLoopDependency::MappedAreaRecovery => {
                    write!(f, "SIS mapped-gate area recovery is not ported to Rust yet")
                }
                SpeedLoopDependency::SignalHandler => {
                    write!(f, "SIS CPU-limit signal handling is not ported to Rust yet")
                }
            },
        }
    }
}

impl Error for SpeedLoopError {}

pub fn required_port_beads() -> &'static [&'static str] {
    REQUIRED_PORT_BEADS
}

pub fn speed_loop_interface_setup(
    threshold: f64,
    coeff: f64,
    dist: i32,
    model: DelayModel,
    trace: bool,
) -> SpeedLoopInterfaceSetup {
    let options = SpeedLoopOptions {
        threshold,
        coeff,
        dist,
        model,
        trace,
        ..SpeedLoopOptions::default()
    };

    SpeedLoopInterfaceSetup {
        options,
        library_acceleration: false,
    }
}

pub fn transform_run(options: &SpeedLoopOptions) -> TransformRun {
    if options.new_mode {
        TransformRun::NewSpeed
    } else {
        TransformRun::LegacySpeedUpNetwork
    }
}

pub fn script_distances(new_mode: bool) -> Vec<i32> {
    if new_mode {
        vec![DEFAULT_SPEED_DIST]
    } else {
        vec![
            DEFAULT_SPEED_DIST,
            DEFAULT_SPEED_DIST + 1,
            DEFAULT_SPEED_DIST + 2,
            DEFAULT_SPEED_DIST + 1,
            DEFAULT_SPEED_DIST,
        ]
    }
}

pub fn speed_up_script_plan(options: &SpeedLoopOptions) -> Vec<SpeedLoopOptions> {
    script_distances(options.new_mode)
        .into_iter()
        .map(|dist| SpeedLoopOptions {
            dist,
            ..options.clone()
        })
        .collect()
}

pub fn metric_label(req_times_set: bool) -> &'static str {
    if req_times_set { "Slack" } else { "Delay" }
}

pub fn is_improved(req_times_set: bool, best: f64, cur: f64) -> bool {
    if req_times_set {
        best > cur + NSP_EPSILON
    } else {
        best < cur - NSP_EPSILON
    }
}

pub fn timing_constraints_met(options: &SpeedLoopOptions, performance: NetworkPerformance) -> bool {
    options.req_times_set && performance.value > NSP_EPSILON
}

pub fn format_loop_trace_header(options: &SpeedLoopOptions) -> String {
    if options.new_mode {
        let mut result = format!(
            "distance = {:<2}, Selection = {}, {}AGG-B{}, Transforms: ",
            options.dist,
            options.region.c_method_name(),
            if options.del_crit_cubes { "" } else { "NON" },
            if options.transform_selection == TransformSelection::BestBangForBuck {
                "/C"
            } else {
                ""
            }
        );

        for transform in &options.local_transforms {
            if transform.enabled {
                result.push_str(&format!(" \"{}\"", transform.name));
            }
        }
        result.push('\n');
        result
    } else {
        format!(
            "distance = {:<2}  threshold = {:3.1}\n",
            options.dist, options.threshold
        )
    }
}

pub fn format_performance_transition(
    req_times_set: bool,
    current: &NamedPerformance,
    best: &NamedPerformance,
) -> String {
    format!(
        "\t{}  {:5.2} -> {:5.2} Area {:.1} -> {:.1}  {} -> {}\n",
        metric_label(req_times_set),
        current.value,
        best.value,
        current.area,
        best.area,
        current.node_name,
        best.node_name
    )
}

pub fn decide_iteration(
    options: &SpeedLoopOptions,
    observation: &IterationObservation,
) -> LoopIterationPlan {
    let mut actions = vec![
        LoopAction::TraceInitialDelay,
        LoopAction::MeasureCurrentPerformance,
    ];

    if observation.primary_input_count == 0 {
        return LoopIterationPlan {
            actions,
            outcome: LoopOutcome::Stop(LoopStopReason::TrivialNetwork),
        };
    }

    if timing_constraints_met(options, observation.current) {
        return LoopIterationPlan {
            actions,
            outcome: LoopOutcome::Stop(LoopStopReason::TimingConstraintsMet),
        };
    }

    actions.push(LoopAction::DuplicateCurrentNetwork);
    if options.red_removal && options.model != DelayModel::Mapped {
        actions.push(LoopAction::RunRedundancyRemoval);
    }

    match transform_run(options) {
        TransformRun::NewSpeed => actions.push(LoopAction::RunNewSpeed),
        TransformRun::LegacySpeedUpNetwork => actions.push(LoopAction::RunLegacySpeedUpNetwork),
    }
    actions.push(LoopAction::TraceTransformedDelay);
    actions.push(LoopAction::MeasureTransformedPerformance);

    if is_improved(
        options.req_times_set,
        observation.transformed.value,
        observation.current.value,
    ) {
        return accept_plan(
            actions,
            options,
            observation.transformed,
            AcceptedNetwork::FirstTransform,
        );
    }

    if observation.mapped_network {
        actions.push(LoopAction::DownsizeTransformedMappedNetwork);
        if let Some(performance) = observation.downsized_transformed {
            if is_improved(
                options.req_times_set,
                performance.value,
                observation.current.value,
            ) {
                return accept_plan(
                    actions,
                    options,
                    performance,
                    AcceptedNetwork::DownsizedTransformed,
                );
            }
        }

        actions.push(LoopAction::DuplicateSavedNetwork);
        actions.push(LoopAction::DownsizeSavedMappedNetwork);
        if let Some(performance) = observation.downsized_saved {
            if is_improved(
                options.req_times_set,
                performance.value,
                observation.current.value,
            ) {
                return accept_plan(
                    actions,
                    options,
                    performance,
                    AcceptedNetwork::DownsizedSaved,
                );
            }
        }
    }

    if options
        .new_mode
        .then_some(observation.new_speed_status)
        .flatten()
        .is_some_and(|status| status < 0)
    {
        actions.push(LoopAction::RestoreSignalHandler);
        return LoopIterationPlan {
            actions,
            outcome: LoopOutcome::Stop(LoopStopReason::NewSpeedMadeNoChange),
        };
    }

    if options.new_mode {
        actions.push(LoopAction::RetryNewSpeed);
    } else {
        actions.push(LoopAction::RetryLegacySpeedUpNetwork);
    }

    if let Some(performance) = observation.second_transform {
        if is_improved(
            options.req_times_set,
            performance.value,
            observation.current.value,
        ) {
            return accept_plan(
                actions,
                options,
                performance,
                AcceptedNetwork::SecondTransform,
            );
        }
    }

    actions.push(LoopAction::RestoreSignalHandler);
    LoopIterationPlan {
        actions,
        outcome: LoopOutcome::Stop(LoopStopReason::NoImprovement),
    }
}

pub fn speed_up_loop_network_bound<Network>(
    _network: &mut Network,
    _options: &SpeedLoopOptions,
) -> Result<(), SpeedLoopError> {
    Err(SpeedLoopError::MissingDependency(
        SpeedLoopDependency::DelayTrace,
    ))
}

pub fn speed_loop_interface_network_bound<Network>(
    _network: &mut Network,
    _setup: &SpeedLoopInterfaceSetup,
) -> Result<(), SpeedLoopError> {
    Err(SpeedLoopError::MissingDependency(
        SpeedLoopDependency::DelayDataDefaults,
    ))
}

fn accept_plan(
    mut actions: Vec<LoopAction>,
    options: &SpeedLoopOptions,
    performance: NetworkPerformance,
    source: AcceptedNetwork,
) -> LoopIterationPlan {
    actions.push(LoopAction::RestoreSignalHandler);
    LoopIterationPlan {
        actions,
        outcome: LoopOutcome::Accept {
            source,
            timing_constraints_met: timing_constraints_met(options, performance),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perf(value: f64) -> NetworkPerformance {
        NetworkPerformance { value, area: 10.0 }
    }

    #[test]
    fn interface_setup_overrides_c_options_and_disables_library_acceleration() {
        let setup = speed_loop_interface_setup(0.25, 0.75, 6, DelayModel::Mapped, true);

        assert_eq!(setup.options.threshold, 0.25);
        assert_eq!(setup.options.coeff, 0.75);
        assert_eq!(setup.options.dist, 6);
        assert_eq!(setup.options.model, DelayModel::Mapped);
        assert!(setup.options.trace);
        assert!(!setup.library_acceleration);
    }

    #[test]
    fn script_distances_match_c_mutation_sequence() {
        assert_eq!(script_distances(true), vec![3]);
        assert_eq!(script_distances(false), vec![3, 4, 5, 4, 3]);

        let options = SpeedLoopOptions {
            dist: 99,
            threshold: 0.9,
            new_mode: false,
            ..SpeedLoopOptions::default()
        };
        let plan = speed_up_script_plan(&options);

        assert_eq!(
            plan.iter().map(|options| options.dist).collect::<Vec<_>>(),
            vec![3, 4, 5, 4, 3]
        );
        assert!(plan.iter().all(|options| options.threshold == 0.9));
    }

    #[test]
    fn improvement_predicate_matches_slack_vs_delay_modes() {
        assert!(is_improved(false, 9.0, 10.0));
        assert!(!is_improved(false, 10.0 - NSP_EPSILON / 2.0, 10.0));
        assert!(is_improved(true, 2.0, 1.0));
        assert!(!is_improved(true, 1.0 + NSP_EPSILON / 2.0, 1.0));
    }

    #[test]
    fn formats_old_and_new_loop_trace_headers_like_c() {
        let old_options = SpeedLoopOptions {
            dist: 3,
            threshold: 0.5,
            ..SpeedLoopOptions::default()
        };

        assert_eq!(
            format_loop_trace_header(&old_options),
            "distance = 3   threshold = 0.5\n"
        );

        let new_options = SpeedLoopOptions {
            dist: 4,
            new_mode: true,
            region: SpeedRegion::Compromise,
            del_crit_cubes: false,
            transform_selection: TransformSelection::BestBangForBuck,
            local_transforms: vec![
                LocalTransform::enabled("collapse"),
                LocalTransform::disabled("fanout"),
                LocalTransform::enabled("dual"),
            ],
            ..SpeedLoopOptions::default()
        };

        assert_eq!(
            format_loop_trace_header(&new_options),
            "distance = 4 , Selection = COMPROMISE, NONAGG-B/C, Transforms:  \"collapse\" \"dual\"\n"
        );
    }

    #[test]
    fn formats_performance_transition_like_sp_print_macro() {
        assert_eq!(
            format_performance_transition(
                false,
                &NamedPerformance::new(12.0, 44.25, "old"),
                &NamedPerformance::new(9.5, 40.0, "new"),
            ),
            "\tDelay  12.00 ->  9.50 Area 44.2 -> 40.0  old -> new\n"
        );
    }

    #[test]
    fn chooses_first_improved_transform_and_marks_met_slack_constraints() {
        let options = SpeedLoopOptions {
            req_times_set: true,
            new_mode: true,
            ..SpeedLoopOptions::default()
        };
        let plan = decide_iteration(
            &options,
            &IterationObservation {
                primary_input_count: 2,
                current: perf(-0.5),
                transformed: perf(0.25),
                mapped_network: false,
                new_speed_status: Some(1),
                downsized_transformed: None,
                downsized_saved: None,
                second_transform: None,
            },
        );

        assert_eq!(
            plan.outcome,
            LoopOutcome::Accept {
                source: AcceptedNetwork::FirstTransform,
                timing_constraints_met: true,
            }
        );
        assert!(plan.actions.contains(&LoopAction::RunNewSpeed));
    }

    #[test]
    fn plans_mapped_downsize_and_second_legacy_attempt_when_needed() {
        let options = SpeedLoopOptions::default();
        let plan = decide_iteration(
            &options,
            &IterationObservation {
                primary_input_count: 1,
                current: perf(10.0),
                transformed: perf(10.5),
                mapped_network: true,
                new_speed_status: None,
                downsized_transformed: Some(perf(10.2)),
                downsized_saved: Some(perf(10.1)),
                second_transform: Some(perf(8.0)),
            },
        );

        assert_eq!(
            plan.outcome,
            LoopOutcome::Accept {
                source: AcceptedNetwork::SecondTransform,
                timing_constraints_met: false,
            }
        );
        assert!(
            plan.actions
                .contains(&LoopAction::DownsizeTransformedMappedNetwork)
        );
        assert!(
            plan.actions
                .contains(&LoopAction::DownsizeSavedMappedNetwork)
        );
        assert!(
            plan.actions
                .contains(&LoopAction::RetryLegacySpeedUpNetwork)
        );
    }

    #[test]
    fn blocked_network_entries_report_explicit_missing_dependencies() {
        let mut network = ();
        assert!(required_port_beads().contains(&"LogicFriday1-8j8.2.6.481"));
        assert_eq!(
            speed_up_loop_network_bound(&mut network, &SpeedLoopOptions::default()),
            Err(SpeedLoopError::MissingDependency(
                SpeedLoopDependency::DelayTrace
            ))
        );
        assert_eq!(
            speed_loop_interface_network_bound(
                &mut network,
                &speed_loop_interface_setup(0.5, 0.0, 3, DelayModel::Unit, false),
            ),
            Err(SpeedLoopError::MissingDependency(
                SpeedLoopDependency::DelayDataDefaults
            ))
        );
    }
}
