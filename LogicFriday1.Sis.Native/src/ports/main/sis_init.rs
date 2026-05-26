//! Native Rust lifecycle model for SIS startup and shutdown.
//!
//! The original implementation initializes all packages in a fixed order,
//! tears them down in reverse package order, restores the standard diagnostic
//! streams, and performs shared cleanup. This port captures that behavior as
//! typed Rust data and small orchestration helpers. Actual package-specific
//! startup functions are wired as their packages gain native registries.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SisFlavor {
    Sis,
    Mis,
}

impl SisFlavor {
    pub fn program_name(self) -> &'static str {
        match self {
            Self::Sis => "SIS - Version 1.2",
            Self::Mis => "MIS - Version 2.2",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SisBuildOptions {
    pub flavor: SisFlavor,
    pub oct_enabled: bool,
}

impl Default for SisBuildOptions {
    fn default() -> Self {
        Self {
            flavor: SisFlavor::Sis,
            oct_enabled: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SisPackage {
    Command,
    Node,
    Network,
    Io,
    Extract,
    Factor,
    Decomp,
    Resub,
    Delay,
    Map,
    Genlib,
    Phase,
    Pld,
    Sim,
    Simplify,
    Gcd,
    OctIo,
    NtBdd,
    MaxFlow,
    Speed,
    Atpg,
    Graphics,
    Latch,
    Power,
    Retime,
    Graph,
    SeqBdd,
    Stg,
    Clock,
    Astg,
    Timing,
    Test,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SisLifecycleAction {
    SetProgramName(&'static str),
    EnableCoreDumps,
    AttachStandardStreams,
    Initialize(SisPackage),
    RegisterGraphicsHelp,
    End(SisPackage),
    CloseOutputStream,
    CloseErrorStream,
    CloseHistoryStream,
    ResetStandardStreams,
    SharedCleanup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SisStreams {
    pub output_is_standard: bool,
    pub error_is_standard: bool,
    pub history_is_open: bool,
}

impl Default for SisStreams {
    fn default() -> Self {
        Self {
            output_is_standard: true,
            error_is_standard: true,
            history_is_open: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisRuntimeState {
    pub streams: SisStreams,
    pub initialized: Vec<SisPackage>,
}

impl Default for SisRuntimeState {
    fn default() -> Self {
        Self {
            streams: SisStreams::default(),
            initialized: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SisInitError {
    MissingPackagePort(SisPackage),
}

impl fmt::Display for SisInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPackagePort(package) => {
                write!(
                    f,
                    "native package lifecycle is not available for {package:?}"
                )
            }
        }
    }
}

impl Error for SisInitError {}

pub fn sis_startup_plan(options: SisBuildOptions) -> Vec<SisLifecycleAction> {
    let mut actions = Vec::new();

    if options.oct_enabled {
        actions.push(SisLifecycleAction::SetProgramName(
            options.flavor.program_name(),
        ));
        actions.push(SisLifecycleAction::EnableCoreDumps);
    }

    actions.push(SisLifecycleAction::AttachStandardStreams);

    for package in startup_packages(options) {
        actions.push(SisLifecycleAction::Initialize(package));
    }

    actions.push(SisLifecycleAction::RegisterGraphicsHelp);
    actions
}

pub fn sis_shutdown_plan(options: SisBuildOptions, streams: SisStreams) -> Vec<SisLifecycleAction> {
    let mut actions = Vec::new();

    for package in shutdown_packages(options) {
        actions.push(SisLifecycleAction::End(package));
    }

    if !streams.output_is_standard {
        actions.push(SisLifecycleAction::CloseOutputStream);
    }
    if !streams.error_is_standard {
        actions.push(SisLifecycleAction::CloseErrorStream);
    }
    if streams.history_is_open {
        actions.push(SisLifecycleAction::CloseHistoryStream);
    }

    actions.push(SisLifecycleAction::ResetStandardStreams);
    actions.push(SisLifecycleAction::SharedCleanup);
    actions
}

pub fn startup_packages(options: SisBuildOptions) -> Vec<SisPackage> {
    let mut packages = vec![
        SisPackage::Command,
        SisPackage::Node,
        SisPackage::Network,
        SisPackage::Io,
        SisPackage::Extract,
        SisPackage::Factor,
        SisPackage::Decomp,
        SisPackage::Resub,
        SisPackage::Delay,
        SisPackage::Map,
        SisPackage::Genlib,
        SisPackage::Phase,
        SisPackage::Pld,
        SisPackage::Sim,
        SisPackage::Simplify,
        SisPackage::Gcd,
    ];

    if options.oct_enabled {
        packages.push(SisPackage::OctIo);
    }

    packages.extend([
        SisPackage::NtBdd,
        SisPackage::MaxFlow,
        SisPackage::Speed,
        SisPackage::Atpg,
        SisPackage::Graphics,
        SisPackage::Latch,
        SisPackage::Power,
        SisPackage::Retime,
        SisPackage::Graph,
        SisPackage::SeqBdd,
        SisPackage::Stg,
        SisPackage::Clock,
        SisPackage::Astg,
        SisPackage::Timing,
        SisPackage::Test,
    ]);

    packages
}

pub fn shutdown_packages(options: SisBuildOptions) -> Vec<SisPackage> {
    let mut packages = startup_packages(options);
    packages.reverse();
    packages
}

pub fn init_sis(state: &mut SisRuntimeState, options: SisBuildOptions) {
    state.streams = SisStreams::default();
    state.initialized = startup_packages(options);
}

pub fn end_sis(state: &mut SisRuntimeState, options: SisBuildOptions) -> Vec<SisLifecycleAction> {
    let streams = state.streams;
    let actions = sis_shutdown_plan(options, streams);

    state.streams = SisStreams::default();
    state.initialized.clear();

    actions
}

pub fn execute_startup_plan(actions: &[SisLifecycleAction]) -> Result<(), SisInitError> {
    for action in actions {
        if let SisLifecycleAction::Initialize(package) = action {
            return Err(SisInitError::MissingPackagePort(*package));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_packages() -> Vec<SisPackage> {
        vec![
            SisPackage::Command,
            SisPackage::Node,
            SisPackage::Network,
            SisPackage::Io,
            SisPackage::Extract,
            SisPackage::Factor,
            SisPackage::Decomp,
            SisPackage::Resub,
            SisPackage::Delay,
            SisPackage::Map,
            SisPackage::Genlib,
            SisPackage::Phase,
            SisPackage::Pld,
            SisPackage::Sim,
            SisPackage::Simplify,
            SisPackage::Gcd,
            SisPackage::NtBdd,
            SisPackage::MaxFlow,
            SisPackage::Speed,
            SisPackage::Atpg,
            SisPackage::Graphics,
            SisPackage::Latch,
            SisPackage::Power,
            SisPackage::Retime,
            SisPackage::Graph,
            SisPackage::SeqBdd,
            SisPackage::Stg,
            SisPackage::Clock,
            SisPackage::Astg,
            SisPackage::Timing,
            SisPackage::Test,
        ]
    }

    #[test]
    fn startup_packages_match_default_sis_order() {
        assert_eq!(
            startup_packages(SisBuildOptions::default()),
            default_packages()
        );
    }

    #[test]
    fn startup_plan_attaches_streams_before_initializing_command_package() {
        let plan = sis_startup_plan(SisBuildOptions::default());

        assert_eq!(plan[0], SisLifecycleAction::AttachStandardStreams);
        assert_eq!(plan[1], SisLifecycleAction::Initialize(SisPackage::Command));
        assert_eq!(plan.last(), Some(&SisLifecycleAction::RegisterGraphicsHelp));
    }

    #[test]
    fn oct_startup_records_program_name_and_oct_package() {
        let options = SisBuildOptions {
            flavor: SisFlavor::Mis,
            oct_enabled: true,
        };
        let plan = sis_startup_plan(options);

        assert_eq!(
            plan[0],
            SisLifecycleAction::SetProgramName("MIS - Version 2.2")
        );
        assert_eq!(plan[1], SisLifecycleAction::EnableCoreDumps);
        assert!(plan.contains(&SisLifecycleAction::Initialize(SisPackage::OctIo)));
    }

    #[test]
    fn shutdown_packages_reverse_startup_packages() {
        let options = SisBuildOptions::default();
        let mut expected = default_packages();
        expected.reverse();

        assert_eq!(shutdown_packages(options), expected);
    }

    #[test]
    fn shutdown_closes_only_nonstandard_streams_and_resets_state() {
        let streams = SisStreams {
            output_is_standard: false,
            error_is_standard: true,
            history_is_open: true,
        };
        let plan = sis_shutdown_plan(SisBuildOptions::default(), streams);

        assert!(plan.contains(&SisLifecycleAction::CloseOutputStream));
        assert!(!plan.contains(&SisLifecycleAction::CloseErrorStream));
        assert!(plan.contains(&SisLifecycleAction::CloseHistoryStream));
        assert_eq!(
            plan[plan.len() - 2],
            SisLifecycleAction::ResetStandardStreams
        );
        assert_eq!(plan[plan.len() - 1], SisLifecycleAction::SharedCleanup);
    }

    #[test]
    fn runtime_helpers_initialize_and_clear_state() {
        let mut state = SisRuntimeState {
            streams: SisStreams {
                output_is_standard: false,
                error_is_standard: false,
                history_is_open: true,
            },
            initialized: Vec::new(),
        };

        init_sis(&mut state, SisBuildOptions::default());

        assert_eq!(state.streams, SisStreams::default());
        assert_eq!(state.initialized, default_packages());

        state.streams = SisStreams {
            output_is_standard: false,
            error_is_standard: false,
            history_is_open: true,
        };
        let shutdown = end_sis(&mut state, SisBuildOptions::default());

        assert!(shutdown.contains(&SisLifecycleAction::CloseOutputStream));
        assert!(shutdown.contains(&SisLifecycleAction::CloseErrorStream));
        assert!(shutdown.contains(&SisLifecycleAction::CloseHistoryStream));
        assert_eq!(state, SisRuntimeState::default());
    }

    #[test]
    fn missing_package_diagnostic_is_generic() {
        let error = execute_startup_plan(&[
            SisLifecycleAction::AttachStandardStreams,
            SisLifecycleAction::Initialize(SisPackage::Command),
        ])
        .unwrap_err();

        assert_eq!(error, SisInitError::MissingPackagePort(SisPackage::Command));
        assert_eq!(
            error.to_string(),
            "native package lifecycle is not available for Command"
        );
    }
}
