use std::collections::BTreeSet;
use std::fmt;

const EPSILON: f64 = 1.0e-12;

pub const CLOCK_NOT_SET: f64 = -1.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockSetting {
    Specification,
    Working,
}

impl ClockSetting {
    fn index(self) -> usize {
        match self {
            Self::Specification => 0,
            Self::Working => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ClockTransition {
    Rise,
    Fall,
}

impl ClockTransition {
    fn index(self) -> usize {
        match self {
            Self::Rise => 0,
            Self::Fall => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockParameter {
    NominalPosition,
    AbsoluteValue,
    LowerRange,
    UpperRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClockEdgeRef {
    pub clock: usize,
    pub transition: ClockTransition,
}

impl ClockEdgeRef {
    pub fn new(clock: usize, transition: ClockTransition) -> Self {
        Self { clock, transition }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockValue {
    pub nominal: f64,
    pub lower_range: f64,
    pub upper_range: f64,
}

impl Default for ClockValue {
    fn default() -> Self {
        Self {
            nominal: CLOCK_NOT_SET,
            lower_range: CLOCK_NOT_SET,
            upper_range: CLOCK_NOT_SET,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockError {
    ClockNotFound(String),
    DuplicateClock(String),
    InvalidClockIndex(usize),
    IncompatibleNominalPositions,
}

impl fmt::Display for ClockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClockNotFound(name) => write!(formatter, "clock '{name}' was not found"),
            Self::DuplicateClock(name) => write!(formatter, "clock '{name}' already exists"),
            Self::InvalidClockIndex(index) => write!(formatter, "clock index {index} is invalid"),
            Self::IncompatibleNominalPositions => {
                write!(formatter, "clock edges have incompatible nominal positions")
            }
        }
    }
}

impl std::error::Error for ClockError {}

#[derive(Clone, Debug, PartialEq)]
pub struct SisClock {
    name: String,
    values: [[ClockValue; 2]; 2],
    dependencies: [[BTreeSet<ClockEdgeRef>; 2]; 2],
}

impl SisClock {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: [[ClockValue::default(); 2]; 2],
            dependencies: std::array::from_fn(|_| std::array::from_fn(|_| BTreeSet::new())),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockNetwork {
    current_setting: ClockSetting,
    cycle_time: [f64; 2],
    clocks: Vec<SisClock>,
}

impl Default for ClockNetwork {
    fn default() -> Self {
        Self {
            current_setting: ClockSetting::Specification,
            cycle_time: [CLOCK_NOT_SET, CLOCK_NOT_SET],
            clocks: Vec::new(),
        }
    }
}

impl ClockNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_clock(&mut self, clock: SisClock) -> Result<usize, ClockError> {
        if self.find_clock(clock.name()).is_some() {
            return Err(ClockError::DuplicateClock(clock.name().to_owned()));
        }

        let index = self.clocks.len();
        self.clocks.push(clock);
        Ok(index)
    }

    pub fn add_clock_named(&mut self, name: impl Into<String>) -> Result<usize, ClockError> {
        self.add_clock(SisClock::new(name))
    }

    pub fn delete_clock(&mut self, clock: usize) -> Result<SisClock, ClockError> {
        self.require_clock(clock)?;

        for source in 0..self.clocks.len() {
            for transition in [ClockTransition::Rise, ClockTransition::Fall] {
                for setting in [ClockSetting::Specification, ClockSetting::Working] {
                    self.dependencies_mut(ClockEdgeRef::new(source, transition), setting)
                        .retain(|edge| edge.clock != clock);
                }
            }
        }

        let removed = self.clocks.remove(clock);
        for source in 0..self.clocks.len() {
            for transition in [ClockTransition::Rise, ClockTransition::Fall] {
                for setting in [ClockSetting::Specification, ClockSetting::Working] {
                    let adjusted = self
                        .dependencies(ClockEdgeRef::new(source, transition), setting)
                        .iter()
                        .map(|edge| {
                            let adjusted_clock = if edge.clock > clock {
                                edge.clock - 1
                            } else {
                                edge.clock
                            };

                            ClockEdgeRef::new(adjusted_clock, edge.transition)
                        })
                        .collect();

                    *self.dependencies_mut(ClockEdgeRef::new(source, transition), setting) =
                        adjusted;
                }
            }
        }

        Ok(removed)
    }

    pub fn clocks(&self) -> &[SisClock] {
        &self.clocks
    }

    pub fn num_clocks(&self) -> usize {
        self.clocks.len()
    }

    pub fn find_clock(&self, name: &str) -> Option<usize> {
        self.clocks.iter().position(|clock| clock.name() == name)
    }

    pub fn clock_name(&self, clock: usize) -> Result<&str, ClockError> {
        self.require_clock(clock)?;
        Ok(self.clocks[clock].name())
    }

    pub fn set_current_setting(&mut self, setting: ClockSetting) {
        self.current_setting = setting;
    }

    pub fn current_setting(&self) -> ClockSetting {
        self.current_setting
    }

    pub fn set_cycle_time(&mut self, value: f64) {
        self.cycle_time[self.current_setting.index()] = value;
    }

    pub fn cycle_time(&self) -> f64 {
        self.cycle_time[self.current_setting.index()]
    }

    pub fn set_parameter(
        &mut self,
        edge: ClockEdgeRef,
        parameter: ClockParameter,
        value: f64,
    ) -> Result<(), ClockError> {
        self.require_edge(edge)?;
        let setting = self.current_setting;
        self.set_edge_parameter(edge, setting, parameter, value);

        if parameter == ClockParameter::NominalPosition {
            let dependent_edges = self
                .dependent_edges(edge)?
                .iter()
                .copied()
                .collect::<Vec<_>>();
            for dependent_edge in dependent_edges {
                self.set_edge_parameter(
                    dependent_edge,
                    setting,
                    ClockParameter::NominalPosition,
                    value,
                );
            }
        }

        Ok(())
    }

    pub fn parameter(
        &self,
        edge: ClockEdgeRef,
        parameter: ClockParameter,
    ) -> Result<f64, ClockError> {
        self.require_edge(edge)?;
        let value = self.value(edge, self.current_setting);
        let result = match parameter {
            ClockParameter::NominalPosition => value.nominal,
            ClockParameter::LowerRange => value.lower_range,
            ClockParameter::UpperRange => value.upper_range,
            ClockParameter::AbsoluteValue => {
                let cycle = self.cycle_time();
                if cycle == CLOCK_NOT_SET {
                    CLOCK_NOT_SET
                } else {
                    value.nominal * cycle / 100.0
                }
            }
        };

        Ok(result)
    }

    pub fn add_dependency(
        &mut self,
        edge1: ClockEdgeRef,
        edge2: ClockEdgeRef,
    ) -> Result<(), ClockError> {
        self.require_edge(edge1)?;
        self.require_edge(edge2)?;

        if edge1 == edge2 {
            return Ok(());
        }

        let nominal1 = self.parameter(edge1, ClockParameter::NominalPosition)?;
        let nominal2 = self.parameter(edge2, ClockParameter::NominalPosition)?;
        if (nominal1 - nominal2).abs() > EPSILON {
            return Err(ClockError::IncompatibleNominalPositions);
        }

        if self.is_dependent(edge1, edge2)? {
            return Ok(());
        }

        let setting = self.current_setting;
        let mut component = BTreeSet::new();
        component.insert(edge1);
        component.insert(edge2);
        component.extend(self.dependencies(edge1, setting).iter().copied());
        component.extend(self.dependencies(edge2, setting).iter().copied());

        let edges = component.into_iter().collect::<Vec<_>>();
        for source in &edges {
            for target in &edges {
                if source != target {
                    self.dependencies_mut(*source, setting).insert(*target);
                }
            }
        }

        Ok(())
    }

    pub fn remove_dependency(
        &mut self,
        edge1: ClockEdgeRef,
        edge2: ClockEdgeRef,
    ) -> Result<(), ClockError> {
        self.require_edge(edge1)?;
        self.require_edge(edge2)?;

        if edge1 == edge2 {
            return Ok(());
        }

        let setting = self.current_setting;
        self.dependencies_mut(edge1, setting).remove(&edge2);
        self.dependencies_mut(edge2, setting).remove(&edge1);
        Ok(())
    }

    pub fn is_dependent(
        &self,
        edge1: ClockEdgeRef,
        edge2: ClockEdgeRef,
    ) -> Result<bool, ClockError> {
        self.require_edge(edge1)?;
        self.require_edge(edge2)?;

        if edge1 == edge2 {
            return Ok(true);
        }

        Ok(self
            .dependencies(edge1, self.current_setting)
            .contains(&edge2))
    }

    pub fn dependent_edges(
        &self,
        edge: ClockEdgeRef,
    ) -> Result<&BTreeSet<ClockEdgeRef>, ClockError> {
        self.require_edge(edge)?;
        Ok(self.dependencies(edge, self.current_setting))
    }

    pub fn num_dependent_edges(&self, edge: ClockEdgeRef) -> Result<usize, ClockError> {
        self.dependent_edges(edge).map(BTreeSet::len)
    }

    pub fn duplicate(&self) -> Self {
        self.clone()
    }

    fn require_clock(&self, clock: usize) -> Result<(), ClockError> {
        if clock < self.clocks.len() {
            Ok(())
        } else {
            Err(ClockError::InvalidClockIndex(clock))
        }
    }

    fn require_edge(&self, edge: ClockEdgeRef) -> Result<(), ClockError> {
        self.require_clock(edge.clock)
    }

    fn value(&self, edge: ClockEdgeRef, setting: ClockSetting) -> ClockValue {
        self.clocks[edge.clock].values[edge.transition.index()][setting.index()]
    }

    fn value_mut(&mut self, edge: ClockEdgeRef, setting: ClockSetting) -> &mut ClockValue {
        &mut self.clocks[edge.clock].values[edge.transition.index()][setting.index()]
    }

    fn set_edge_parameter(
        &mut self,
        edge: ClockEdgeRef,
        setting: ClockSetting,
        parameter: ClockParameter,
        value: f64,
    ) {
        match parameter {
            ClockParameter::NominalPosition => self.value_mut(edge, setting).nominal = value,
            ClockParameter::LowerRange => self.value_mut(edge, setting).lower_range = value,
            ClockParameter::UpperRange => self.value_mut(edge, setting).upper_range = value,
            ClockParameter::AbsoluteValue => {
                let cycle = self.cycle_time[setting.index()];
                self.value_mut(edge, setting).nominal = if cycle == CLOCK_NOT_SET {
                    CLOCK_NOT_SET
                } else {
                    value * 100.0 / cycle
                };
            }
        }
    }

    fn dependencies(&self, edge: ClockEdgeRef, setting: ClockSetting) -> &BTreeSet<ClockEdgeRef> {
        &self.clocks[edge.clock].dependencies[edge.transition.index()][setting.index()]
    }

    fn dependencies_mut(
        &mut self,
        edge: ClockEdgeRef,
        setting: ClockSetting,
    ) -> &mut BTreeSet<ClockEdgeRef> {
        &mut self.clocks[edge.clock].dependencies[edge.transition.index()][setting.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(clock: usize, transition: ClockTransition) -> ClockEdgeRef {
        ClockEdgeRef::new(clock, transition)
    }

    #[test]
    fn new_network_starts_with_specification_setting_and_no_cycle() {
        let network = ClockNetwork::new();

        assert_eq!(network.current_setting(), ClockSetting::Specification);
        assert_eq!(network.cycle_time(), CLOCK_NOT_SET);
        assert_eq!(network.num_clocks(), 0);
    }

    #[test]
    fn add_clock_rejects_duplicate_names() {
        let mut network = ClockNetwork::new();

        assert_eq!(network.add_clock_named("clk").unwrap(), 0);
        assert_eq!(
            network.add_clock_named("clk"),
            Err(ClockError::DuplicateClock("clk".to_owned()))
        );
        assert_eq!(network.find_clock("clk"), Some(0));
    }

    #[test]
    fn setting_selects_independent_cycle_and_edge_parameters() {
        let mut network = ClockNetwork::new();
        let clk = network.add_clock_named("clk").unwrap();
        let rise = edge(clk, ClockTransition::Rise);

        network.set_cycle_time(100.0);
        network
            .set_parameter(rise, ClockParameter::NominalPosition, 25.0)
            .unwrap();
        network
            .set_parameter(rise, ClockParameter::LowerRange, 2.0)
            .unwrap();
        network.set_current_setting(ClockSetting::Working);
        network.set_cycle_time(80.0);
        network
            .set_parameter(rise, ClockParameter::NominalPosition, 50.0)
            .unwrap();

        assert_eq!(
            network
                .parameter(rise, ClockParameter::AbsoluteValue)
                .unwrap(),
            40.0
        );
        network.set_current_setting(ClockSetting::Specification);
        assert_eq!(
            network
                .parameter(rise, ClockParameter::NominalPosition)
                .unwrap(),
            25.0
        );
        assert_eq!(
            network.parameter(rise, ClockParameter::LowerRange).unwrap(),
            2.0
        );
        assert_eq!(
            network
                .parameter(rise, ClockParameter::AbsoluteValue)
                .unwrap(),
            25.0
        );
    }

    #[test]
    fn dependency_addition_builds_transitive_symmetric_groups() {
        let mut network = ClockNetwork::new();
        let a = network.add_clock_named("a").unwrap();
        let b = network.add_clock_named("b").unwrap();
        let c = network.add_clock_named("c").unwrap();
        let a_rise = edge(a, ClockTransition::Rise);
        let b_fall = edge(b, ClockTransition::Fall);
        let c_rise = edge(c, ClockTransition::Rise);

        for clock_edge in [a_rise, b_fall, c_rise] {
            network
                .set_parameter(clock_edge, ClockParameter::NominalPosition, 10.0)
                .unwrap();
        }

        network.add_dependency(a_rise, b_fall).unwrap();
        network.add_dependency(b_fall, c_rise).unwrap();

        assert!(network.is_dependent(a_rise, c_rise).unwrap());
        assert!(network.is_dependent(c_rise, a_rise).unwrap());
        assert_eq!(network.num_dependent_edges(b_fall).unwrap(), 2);
    }

    #[test]
    fn dependency_rejects_incompatible_nominal_positions() {
        let mut network = ClockNetwork::new();
        let a = network.add_clock_named("a").unwrap();
        let b = network.add_clock_named("b").unwrap();
        let a_rise = edge(a, ClockTransition::Rise);
        let b_rise = edge(b, ClockTransition::Rise);

        network
            .set_parameter(a_rise, ClockParameter::NominalPosition, 10.0)
            .unwrap();
        network
            .set_parameter(b_rise, ClockParameter::NominalPosition, 11.0)
            .unwrap();

        assert_eq!(
            network.add_dependency(a_rise, b_rise),
            Err(ClockError::IncompatibleNominalPositions)
        );
    }

    #[test]
    fn setting_nominal_position_updates_dependent_edges() {
        let mut network = ClockNetwork::new();
        let a = network.add_clock_named("a").unwrap();
        let b = network.add_clock_named("b").unwrap();
        let a_rise = edge(a, ClockTransition::Rise);
        let b_rise = edge(b, ClockTransition::Rise);

        network
            .set_parameter(a_rise, ClockParameter::NominalPosition, 0.0)
            .unwrap();
        network
            .set_parameter(b_rise, ClockParameter::NominalPosition, 0.0)
            .unwrap();
        network.add_dependency(a_rise, b_rise).unwrap();
        network
            .set_parameter(a_rise, ClockParameter::NominalPosition, 45.0)
            .unwrap();

        assert_eq!(
            network
                .parameter(b_rise, ClockParameter::NominalPosition)
                .unwrap(),
            45.0
        );
    }

    #[test]
    fn remove_dependency_removes_symmetric_pair() {
        let mut network = ClockNetwork::new();
        let a = network.add_clock_named("a").unwrap();
        let b = network.add_clock_named("b").unwrap();
        let a_rise = edge(a, ClockTransition::Rise);
        let b_rise = edge(b, ClockTransition::Rise);

        network.add_dependency(a_rise, b_rise).unwrap();
        network.remove_dependency(a_rise, b_rise).unwrap();

        assert!(!network.is_dependent(a_rise, b_rise).unwrap());
        assert!(!network.is_dependent(b_rise, a_rise).unwrap());
    }

    #[test]
    fn delete_clock_removes_dependencies_and_reindexes_remaining_edges() {
        let mut network = ClockNetwork::new();
        let a = network.add_clock_named("a").unwrap();
        let b = network.add_clock_named("b").unwrap();
        let c = network.add_clock_named("c").unwrap();
        let a_rise = edge(a, ClockTransition::Rise);
        let b_rise = edge(b, ClockTransition::Rise);
        let c_rise = edge(c, ClockTransition::Rise);

        network.add_dependency(a_rise, c_rise).unwrap();
        network.delete_clock(b).unwrap();

        let new_c_rise = edge(1, ClockTransition::Rise);
        assert_eq!(network.clock_name(1).unwrap(), "c");
        assert!(
            network
                .is_dependent(edge(0, ClockTransition::Rise), new_c_rise)
                .unwrap()
        );
        assert_eq!(network.num_dependent_edges(new_c_rise).unwrap(), 1);
        assert_eq!(network.num_clocks(), 2);
        assert_eq!(network.clock_name(b_rise.clock), Ok("c"));
    }

    #[test]
    fn duplicate_preserves_clock_data_without_sharing_storage() {
        let mut network = ClockNetwork::new();
        let a = network.add_clock_named("a").unwrap();
        let b = network.add_clock_named("b").unwrap();
        let a_fall = edge(a, ClockTransition::Fall);
        let b_fall = edge(b, ClockTransition::Fall);

        network.set_cycle_time(200.0);
        network
            .set_parameter(a_fall, ClockParameter::NominalPosition, 40.0)
            .unwrap();
        network
            .set_parameter(b_fall, ClockParameter::NominalPosition, 40.0)
            .unwrap();
        network.add_dependency(a_fall, b_fall).unwrap();

        let mut duplicate = network.duplicate();
        duplicate
            .set_parameter(a_fall, ClockParameter::NominalPosition, 60.0)
            .unwrap();

        assert_eq!(
            network
                .parameter(a_fall, ClockParameter::NominalPosition)
                .unwrap(),
            40.0
        );
        assert_eq!(
            duplicate
                .parameter(b_fall, ClockParameter::NominalPosition)
                .unwrap(),
            60.0
        );
        assert!(duplicate.is_dependent(a_fall, b_fall).unwrap());
    }

    #[test]
    fn source_does_not_contain_legacy_abi_or_tracking_tokens() {
        let source = include_str!("clock.rs");

        let forbidden_tokens = [
            concat!("no", "_mangle"),
            concat!("extern ", "\"", "C", "\""),
            concat!("REQUIRED", "_"),
            concat!("REQUIRED", "_PORT", "_BEADS"),
            concat!("bead", "_id"),
            concat!("source", "_file"),
        ];

        for forbidden in forbidden_tokens {
            assert!(!source.contains(forbidden), "{forbidden}");
        }
    }
}
