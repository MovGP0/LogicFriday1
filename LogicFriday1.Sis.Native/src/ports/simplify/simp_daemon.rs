//! Native Rust model for `LogicSynthesis/sis/simplify/simp_daemon.c`.
//!
//! The C daemon manages the `node_t->simplify` slot: allocate a
//! `sim_flag_t`, reset it to unknown values, free it, and copy it during node
//! duplication. This port keeps that lifecycle in an owned Rust slot. Direct
//! attachment to legacy `node_t` is intentionally left as an explicit missing
//! dependency until the native node representation is available.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_SIS_NODE_PORTS: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "native node storage is needed before the simplify slot can be attached to SIS nodes",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.455",
        source_file: "LogicSynthesis/sis/simplify/simp.c",
        reason: "native simplify-node execution is the main consumer of the stored method, accept, and don't-care type flags",
    },
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SimMethod {
    Simpcomp,
    Espresso,
    Exact,
    ExactLits,
    Dcsimp,
    Nocomp,
    Snocomp,
    #[default]
    Unknown,
}

impl SimMethod {
    pub const fn c_discriminant(self) -> i32 {
        match self {
            Self::Simpcomp => 0,
            Self::Espresso => 1,
            Self::Exact => 2,
            Self::ExactLits => 3,
            Self::Dcsimp => 4,
            Self::Nocomp => 5,
            Self::Snocomp => 6,
            Self::Unknown => 7,
        }
    }

    pub const fn from_c_discriminant(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Simpcomp),
            1 => Some(Self::Espresso),
            2 => Some(Self::Exact),
            3 => Some(Self::ExactLits),
            4 => Some(Self::Dcsimp),
            5 => Some(Self::Nocomp),
            6 => Some(Self::Snocomp),
            7 => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SimAccept {
    FactoredLiterals,
    Cubes,
    SopLiterals,
    Always,
    #[default]
    Unknown,
}

impl SimAccept {
    pub const fn c_discriminant(self) -> i32 {
        match self {
            Self::FactoredLiterals => 0,
            Self::Cubes => 1,
            Self::SopLiterals => 2,
            Self::Always => 3,
            Self::Unknown => 4,
        }
    }

    pub const fn from_c_discriminant(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::FactoredLiterals),
            1 => Some(Self::Cubes),
            2 => Some(Self::SopLiterals),
            3 => Some(Self::Always),
            4 => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SimDcType {
    None,
    Fanin,
    Fanout,
    Inout,
    All,
    SubFanin,
    Level,
    #[default]
    Unknown,
    X,
}

impl SimDcType {
    pub const fn c_discriminant(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Fanin => 1,
            Self::Fanout => 2,
            Self::Inout => 3,
            Self::All => 4,
            Self::SubFanin => 5,
            Self::Level => 6,
            Self::Unknown => 7,
            Self::X => 8,
        }
    }

    pub const fn from_c_discriminant(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Fanin),
            2 => Some(Self::Fanout),
            3 => Some(Self::Inout),
            4 => Some(Self::All),
            5 => Some(Self::SubFanin),
            6 => Some(Self::Level),
            7 => Some(Self::Unknown),
            8 => Some(Self::X),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SimFlag {
    pub method: SimMethod,
    pub accept: SimAccept,
    pub dctype: SimDcType,
}

impl SimFlag {
    pub const fn new(method: SimMethod, accept: SimAccept, dctype: SimDcType) -> Self {
        Self {
            method,
            accept,
            dctype,
        }
    }

    pub const fn unknown() -> Self {
        Self::new(SimMethod::Unknown, SimAccept::Unknown, SimDcType::Unknown)
    }

    pub fn invalidate(&mut self) {
        *self = Self::unknown();
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimplifySlot {
    flag: Option<SimFlag>,
}

impl SimplifySlot {
    pub const fn empty() -> Self {
        Self { flag: None }
    }

    pub const fn allocated_unknown() -> Self {
        Self {
            flag: Some(SimFlag::unknown()),
        }
    }

    pub fn is_allocated(&self) -> bool {
        self.flag.is_some()
    }

    pub fn flag(&self) -> Option<SimFlag> {
        self.flag
    }

    pub fn flag_mut(&mut self) -> Result<&mut SimFlag, SimDaemonError> {
        self.flag
            .as_mut()
            .ok_or(SimDaemonError::MissingSimplifySlot {
                operation: "SIM_FLAG mutation",
            })
    }

    pub fn allocate(&mut self) {
        self.flag = Some(SimFlag::unknown());
    }

    pub fn free(&mut self) {
        self.flag = None;
    }

    pub fn invalidate(&mut self) -> Result<(), SimDaemonError> {
        self.flag_mut()?.invalidate();
        Ok(())
    }

    pub fn duplicate_from(&mut self, old: &Self) -> Result<(), SimDaemonError> {
        let old_flag = old.flag.ok_or(SimDaemonError::MissingSimplifySlot {
            operation: "simp_dup source",
        })?;
        let new_flag = self
            .flag_mut()
            .map_err(|_| SimDaemonError::MissingSimplifySlot {
                operation: "simp_dup destination",
            })?;

        *new_flag = old_flag;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimDaemonError {
    MissingSimplifySlot {
        operation: &'static str,
    },
    MissingSisNodePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for SimDaemonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSimplifySlot { operation } => {
                write!(f, "{operation} requires an allocated simplify slot")
            }
            Self::MissingSisNodePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} requires {} unported native SIS dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for SimDaemonError {}

pub fn required_sis_node_ports() -> &'static [PortDependency] {
    REQUIRED_SIS_NODE_PORTS
}

pub fn attach_to_sis_node() -> Result<(), SimDaemonError> {
    Err(SimDaemonError::MissingSisNodePorts {
        operation: "simp_alloc on native SIS node",
        dependencies: REQUIRED_SIS_NODE_PORTS,
    })
}

pub fn detach_from_sis_node() -> Result<(), SimDaemonError> {
    Err(SimDaemonError::MissingSisNodePorts {
        operation: "simp_free on native SIS node",
        dependencies: REQUIRED_SIS_NODE_PORTS,
    })
}

pub fn invalidate_sis_node() -> Result<(), SimDaemonError> {
    Err(SimDaemonError::MissingSisNodePorts {
        operation: "simp_invalid on native SIS node",
        dependencies: REQUIRED_SIS_NODE_PORTS,
    })
}

pub fn duplicate_sis_node_slot() -> Result<(), SimDaemonError> {
    Err(SimDaemonError::MissingSisNodePorts {
        operation: "simp_dup on native SIS nodes",
        dependencies: REQUIRED_SIS_NODE_PORTS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_ordinals_match_simplify_header() {
        assert_eq!(SimMethod::Simpcomp.c_discriminant(), 0);
        assert_eq!(SimMethod::Snocomp.c_discriminant(), 6);
        assert_eq!(SimMethod::Unknown.c_discriminant(), 7);
        assert_eq!(SimMethod::from_c_discriminant(4), Some(SimMethod::Dcsimp));
        assert_eq!(SimMethod::from_c_discriminant(99), None);

        assert_eq!(SimAccept::FactoredLiterals.c_discriminant(), 0);
        assert_eq!(SimAccept::Unknown.c_discriminant(), 4);
        assert_eq!(
            SimAccept::from_c_discriminant(2),
            Some(SimAccept::SopLiterals)
        );
        assert_eq!(SimAccept::from_c_discriminant(-1), None);

        assert_eq!(SimDcType::None.c_discriminant(), 0);
        assert_eq!(SimDcType::Unknown.c_discriminant(), 7);
        assert_eq!(SimDcType::X.c_discriminant(), 8);
        assert_eq!(SimDcType::from_c_discriminant(5), Some(SimDcType::SubFanin));
        assert_eq!(SimDcType::from_c_discriminant(9), None);
    }

    #[test]
    fn allocate_sets_unknown_flag_values() {
        let mut slot = SimplifySlot::empty();

        assert!(!slot.is_allocated());
        slot.allocate();

        assert!(slot.is_allocated());
        assert_eq!(slot.flag(), Some(SimFlag::unknown()));
    }

    #[test]
    fn free_discards_the_simplify_slot() {
        let mut slot = SimplifySlot::allocated_unknown();

        slot.flag_mut().unwrap().method = SimMethod::Exact;
        slot.free();

        assert!(!slot.is_allocated());
        assert_eq!(slot.flag(), None);
    }

    #[test]
    fn invalidate_resets_existing_slot_to_unknowns() {
        let mut slot = SimplifySlot::allocated_unknown();
        *slot.flag_mut().unwrap() =
            SimFlag::new(SimMethod::Espresso, SimAccept::Always, SimDcType::Fanin);

        slot.invalidate().unwrap();

        assert_eq!(slot.flag(), Some(SimFlag::unknown()));
    }

    #[test]
    fn duplicate_copies_flags_into_an_allocated_destination() {
        let mut old = SimplifySlot::allocated_unknown();
        let mut new = SimplifySlot::allocated_unknown();
        *old.flag_mut().unwrap() =
            SimFlag::new(SimMethod::Nocomp, SimAccept::Cubes, SimDcType::Inout);

        new.duplicate_from(&old).unwrap();

        assert_eq!(new.flag(), old.flag());
    }

    #[test]
    fn slot_operations_report_missing_allocation() {
        let old = SimplifySlot::empty();
        let mut new = SimplifySlot::allocated_unknown();

        assert_eq!(
            SimplifySlot::empty().invalidate(),
            Err(SimDaemonError::MissingSimplifySlot {
                operation: "SIM_FLAG mutation",
            })
        );
        assert_eq!(
            new.duplicate_from(&old),
            Err(SimDaemonError::MissingSimplifySlot {
                operation: "simp_dup source",
            })
        );
        assert_eq!(
            SimplifySlot::empty().duplicate_from(&SimplifySlot::allocated_unknown()),
            Err(SimDaemonError::MissingSimplifySlot {
                operation: "simp_dup destination",
            })
        );
    }

    #[test]
    fn sis_node_entry_points_report_dependency_beads_and_source_files() {
        let dependencies = required_sis_node_ports();

        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.318"
                && dependency.source_file == "LogicSynthesis/sis/node/node.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.455"
                && dependency.source_file == "LogicSynthesis/sis/simplify/simp.c"
        }));
        assert_eq!(
            attach_to_sis_node(),
            Err(SimDaemonError::MissingSisNodePorts {
                operation: "simp_alloc on native SIS node",
                dependencies: REQUIRED_SIS_NODE_PORTS,
            })
        );
        assert_eq!(
            duplicate_sis_node_slot(),
            Err(SimDaemonError::MissingSisNodePorts {
                operation: "simp_dup on native SIS nodes",
                dependencies: REQUIRED_SIS_NODE_PORTS,
            })
        );
    }
}
