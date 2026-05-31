//! Native ASTG package initialization model.
//!
//! The legacy unit is only responsible for package startup and shutdown
//! ordering. It delegates command registration to the basic ASTG,
//! speed-independent, and bounded-wire-delay subpackages, then discards ASTG
//! daemons during shutdown. This port keeps that behavior as data and a small
//! lifecycle type without exposing legacy C ABI symbols.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgCommandGroup
{
    Basic,
    SpeedIndependent,
    BoundedWireDelay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgCommandKind
{
    ReadAstg,
    Flow,
    Current,
    Persist,
    LockGraph,
    Cycle,
    WriteAstg,
    Marking,
    Irredundant,
    Contract,
    StateMachineComponents,
    MarkedGraphComponents,
    Synthesize,
    PrintStateGraph,
    PrintStatistics,
    ToFunctions,
    ToStateGraph,
    SlowDown,
    StateGraphSingleCubeRestriction,
    StateGraphToAstg,
    StateMinimize,
    AddState,
    Encode,
    StateGraphStrongComponents,
    WriteStateGraph,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration
{
    pub name: &'static str,
    pub group: AstgCommandGroup,
    pub kind: AstgCommandKind,
    pub changes_network: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgDaemonKind
{
    Alloc,
    Duplicate,
    Invalid,
    Free,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DaemonRegistration
{
    pub kind: AstgDaemonKind,
}

pub const BASIC_ASTG_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "read_astg",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::ReadAstg,
        changes_network: true,
    },
    CommandRegistration {
        name: "_astg_flow",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::Flow,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_current",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::Current,
        changes_network: false,
    },
    CommandRegistration {
        name: "astg_persist",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::Persist,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_lockgraph",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::LockGraph,
        changes_network: true,
    },
    CommandRegistration {
        name: "_astg_cycle",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::Cycle,
        changes_network: false,
    },
    CommandRegistration {
        name: "write_astg",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::WriteAstg,
        changes_network: false,
    },
    CommandRegistration {
        name: "astg_marking",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::Marking,
        changes_network: true,
    },
    CommandRegistration {
        name: "_astg_irred",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::Irredundant,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_contract",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::Contract,
        changes_network: true,
    },
    CommandRegistration {
        name: "_astg_smc",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::StateMachineComponents,
        changes_network: false,
    },
    CommandRegistration {
        name: "_astg_mgc",
        group: AstgCommandGroup::Basic,
        kind: AstgCommandKind::MarkedGraphComponents,
        changes_network: false,
    },
];

pub const SPEED_INDEPENDENT_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "astg_syn",
        group: AstgCommandGroup::SpeedIndependent,
        kind: AstgCommandKind::Synthesize,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_print_sg",
        group: AstgCommandGroup::SpeedIndependent,
        kind: AstgCommandKind::PrintStateGraph,
        changes_network: false,
    },
    CommandRegistration {
        name: "astg_print_stat",
        group: AstgCommandGroup::SpeedIndependent,
        kind: AstgCommandKind::PrintStatistics,
        changes_network: false,
    },
];

pub const BOUNDED_WIRE_DELAY_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "astg_to_f",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::ToFunctions,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_to_stg",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::ToStateGraph,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_slow",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::SlowDown,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_stg_scr",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::StateGraphSingleCubeRestriction,
        changes_network: true,
    },
    CommandRegistration {
        name: "stg_to_astg",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::StateGraphToAstg,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_state_min",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::StateMinimize,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_add_state",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::AddState,
        changes_network: true,
    },
    CommandRegistration {
        name: "astg_encode",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::Encode,
        changes_network: true,
    },
    CommandRegistration {
        name: "_stg_scc",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::StateGraphStrongComponents,
        changes_network: false,
    },
    CommandRegistration {
        name: "_write_sg",
        group: AstgCommandGroup::BoundedWireDelay,
        kind: AstgCommandKind::WriteStateGraph,
        changes_network: false,
    },
];

pub const BOUNDED_WIRE_DELAY_DAEMONS: &[DaemonRegistration] = &[
    DaemonRegistration {
        kind: AstgDaemonKind::Alloc,
    },
    DaemonRegistration {
        kind: AstgDaemonKind::Duplicate,
    },
    DaemonRegistration {
        kind: AstgDaemonKind::Invalid,
    },
    DaemonRegistration {
        kind: AstgDaemonKind::Free,
    },
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AstgInitializationPlan
{
    pub commands: Vec<CommandRegistration>,
    pub daemons: Vec<DaemonRegistration>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AstgLifecycle
{
    initialized: bool,
    commands: Vec<CommandRegistration>,
    daemons: Vec<DaemonRegistration>,
    init_count: usize,
    daemon_discard_count: usize,
}

impl AstgLifecycle
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn initialized(&self) -> bool
    {
        self.initialized
    }

    pub fn commands(&self) -> &[CommandRegistration]
    {
        &self.commands
    }

    pub fn daemons(&self) -> &[DaemonRegistration]
    {
        &self.daemons
    }

    pub fn init_count(&self) -> usize
    {
        self.init_count
    }

    pub fn daemon_discard_count(&self) -> usize
    {
        self.daemon_discard_count
    }

    pub fn init_astg(&mut self)
    {
        let plan = astg_initialization_plan();
        self.commands = plan.commands;
        self.daemons = plan.daemons;
        self.initialized = true;
        self.init_count += 1;
    }

    pub fn end_astg(&mut self)
    {
        self.daemons.clear();
        self.initialized = false;
        self.daemon_discard_count += 1;
    }
}

pub fn basic_astg_command_registrations() -> &'static [CommandRegistration]
{
    BASIC_ASTG_COMMANDS
}

pub fn speed_independent_command_registrations() -> &'static [CommandRegistration]
{
    SPEED_INDEPENDENT_COMMANDS
}

pub fn bounded_wire_delay_command_registrations() -> &'static [CommandRegistration]
{
    BOUNDED_WIRE_DELAY_COMMANDS
}

pub fn bounded_wire_delay_daemon_registrations() -> &'static [DaemonRegistration]
{
    BOUNDED_WIRE_DELAY_DAEMONS
}

pub fn astg_command_registrations() -> Vec<CommandRegistration>
{
    let mut commands = Vec::with_capacity(
        BASIC_ASTG_COMMANDS.len()
            + SPEED_INDEPENDENT_COMMANDS.len()
            + BOUNDED_WIRE_DELAY_COMMANDS.len(),
    );
    commands.extend_from_slice(BASIC_ASTG_COMMANDS);
    commands.extend_from_slice(SPEED_INDEPENDENT_COMMANDS);
    commands.extend_from_slice(BOUNDED_WIRE_DELAY_COMMANDS);
    commands
}

pub fn astg_initialization_plan() -> AstgInitializationPlan
{
    AstgInitializationPlan {
        commands: astg_command_registrations(),
        daemons: BOUNDED_WIRE_DELAY_DAEMONS.to_vec(),
    }
}

pub fn init_astg(lifecycle: &mut AstgLifecycle) -> AstgInitializationPlan
{
    lifecycle.init_astg();
    AstgInitializationPlan {
        commands: lifecycle.commands.clone(),
        daemons: lifecycle.daemons.clone(),
    }
}

pub fn end_astg(lifecycle: &mut AstgLifecycle)
{
    lifecycle.end_astg();
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn init_plan_preserves_legacy_registration_order()
    {
        let names = astg_command_registrations()
            .iter()
            .map(|registration| registration.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "read_astg",
                "_astg_flow",
                "astg_current",
                "astg_persist",
                "astg_lockgraph",
                "_astg_cycle",
                "write_astg",
                "astg_marking",
                "_astg_irred",
                "astg_contract",
                "_astg_smc",
                "_astg_mgc",
                "astg_syn",
                "astg_print_sg",
                "astg_print_stat",
                "astg_to_f",
                "astg_to_stg",
                "astg_slow",
                "astg_stg_scr",
                "stg_to_astg",
                "astg_state_min",
                "astg_add_state",
                "astg_encode",
                "_stg_scc",
                "_write_sg",
            ]
        );
    }

    #[test]
    fn command_mutability_matches_delegated_command_tables()
    {
        let commands = astg_command_registrations();
        let read_only = commands
            .iter()
            .filter(|registration| !registration.changes_network)
            .map(|registration| registration.name)
            .collect::<Vec<_>>();

        assert_eq!(
            read_only,
            vec![
                "astg_current",
                "_astg_cycle",
                "write_astg",
                "_astg_smc",
                "_astg_mgc",
                "astg_print_sg",
                "astg_print_stat",
                "_stg_scc",
                "_write_sg",
            ]
        );
    }

    #[test]
    fn init_registers_bounded_wire_delay_daemons()
    {
        assert_eq!(
            astg_initialization_plan().daemons,
            vec![
                DaemonRegistration {
                    kind: AstgDaemonKind::Alloc,
                },
                DaemonRegistration {
                    kind: AstgDaemonKind::Duplicate,
                },
                DaemonRegistration {
                    kind: AstgDaemonKind::Invalid,
                },
                DaemonRegistration {
                    kind: AstgDaemonKind::Free,
                },
            ]
        );
    }

    #[test]
    fn lifecycle_init_and_end_match_package_behavior()
    {
        let mut lifecycle = AstgLifecycle::new();

        let plan = init_astg(&mut lifecycle);

        assert!(lifecycle.initialized());
        assert_eq!(lifecycle.init_count(), 1);
        assert_eq!(lifecycle.commands(), plan.commands);
        assert_eq!(lifecycle.daemons(), plan.daemons);
        assert_eq!(lifecycle.commands().len(), 25);
        assert_eq!(lifecycle.daemons().len(), 4);

        end_astg(&mut lifecycle);

        assert!(!lifecycle.initialized());
        assert_eq!(lifecycle.daemon_discard_count(), 1);
        assert_eq!(lifecycle.commands().len(), 25);
        assert!(lifecycle.daemons().is_empty());
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_c_abi_exports()
    {
        let source = include_str!("com_astg.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
