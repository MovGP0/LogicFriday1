//! Native Rust package configuration for the SIS UCB BDD port.
//!
//! The legacy `config.c` unit intentionally owns no package globals. Its
//! runtime effect comes from including the BDD headers, whose default build
//! switches keep the inline ITE paths enabled while all debug/statistics
//! instrumentation is disabled. This module makes that configuration explicit
//! as owned Rust data instead of preserving an empty C translation unit.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddDebugConfig {
    pub package_assertions: bool,
    pub age_tracking: bool,
    pub external_pointer_tracking: bool,
    pub external_pointer_full_tracking: bool,
    pub garbage_collection_checks: bool,
    pub garbage_collection_stats: bool,
    pub lifespan_tracking: bool,
    pub safe_frame_checks: bool,
    pub unique_id_tracking: bool,
    pub flight_recorder: bool,
    pub lifespan_tracefile: Option<String>,
    pub flight_recorder_logfile: Option<String>,
}

impl Default for BddDebugConfig {
    fn default() -> Self {
        Self {
            package_assertions: false,
            age_tracking: false,
            external_pointer_tracking: false,
            external_pointer_full_tracking: false,
            garbage_collection_checks: false,
            garbage_collection_stats: false,
            lifespan_tracking: false,
            safe_frame_checks: false,
            unique_id_tracking: false,
            flight_recorder: false,
            lifespan_tracefile: None,
            flight_recorder_logfile: None,
        }
    }
}

impl BddDebugConfig {
    pub fn any_enabled(&self) -> bool {
        self.package_assertions
            || self.age_tracking
            || self.external_pointer_tracking
            || self.external_pointer_full_tracking
            || self.garbage_collection_checks
            || self.garbage_collection_stats
            || self.lifespan_tracking
            || self.safe_frame_checks
            || self.unique_id_tracking
            || self.flight_recorder
            || self.lifespan_tracefile.is_some()
            || self.flight_recorder_logfile.is_some()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddPackageConfig {
    pub automated_statistics_gathering: bool,
    pub inline_ite: bool,
    pub inline_ite_constant: bool,
    pub no_garbage_collection: bool,
    pub statistics: bool,
    pub memory_usage_tracking: bool,
    pub debug: BddDebugConfig,
    global_state_slots: usize,
}

impl Default for BddPackageConfig {
    fn default() -> Self {
        Self {
            automated_statistics_gathering: false,
            inline_ite: true,
            inline_ite_constant: true,
            no_garbage_collection: false,
            statistics: false,
            memory_usage_tracking: false,
            debug: BddDebugConfig::default(),
            global_state_slots: 0,
        }
    }
}

impl BddPackageConfig {
    pub fn legacy_default() -> Self {
        Self::default()
    }

    pub fn global_state_slots(&self) -> usize {
        self.global_state_slots
    }

    pub fn owns_package_globals(&self) -> bool {
        self.global_state_slots != 0
    }

    pub fn uses_debug_node_identity(&self) -> bool {
        self.debug.unique_id_tracking || self.debug.lifespan_tracking
    }

    pub fn uses_debug_node_age(&self) -> bool {
        self.debug.age_tracking || self.debug.lifespan_tracking
    }

    pub fn with_debug(mut self, debug: BddDebugConfig) -> Self {
        self.debug = debug;
        self
    }

    pub fn with_inline_ite(mut self, inline_ite: bool) -> Self {
        self.inline_ite = inline_ite;
        self
    }

    pub fn with_inline_ite_constant(mut self, inline_ite_constant: bool) -> Self {
        self.inline_ite_constant = inline_ite_constant;
        self
    }
}

pub fn legacy_package_config() -> BddPackageConfig {
    BddPackageConfig::legacy_default()
}

pub fn has_legacy_package_globals() -> bool {
    legacy_package_config().owns_package_globals()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_defaults_have_no_package_globals() {
        let config = legacy_package_config();

        assert_eq!(config.global_state_slots(), 0);
        assert!(!config.owns_package_globals());
        assert!(!has_legacy_package_globals());
    }

    #[test]
    fn legacy_defaults_match_header_switches() {
        let config = legacy_package_config();

        assert!(!config.automated_statistics_gathering);
        assert!(config.inline_ite);
        assert!(config.inline_ite_constant);
        assert!(!config.no_garbage_collection);
        assert!(!config.statistics);
        assert!(!config.memory_usage_tracking);
        assert!(!config.debug.any_enabled());
    }

    #[test]
    fn lifespan_debug_implies_age_and_identity_fields() {
        let mut debug = BddDebugConfig::default();
        debug.lifespan_tracking = true;
        let config = BddPackageConfig::legacy_default().with_debug(debug);

        assert!(config.uses_debug_node_age());
        assert!(config.uses_debug_node_identity());
    }

    #[test]
    fn direct_age_and_identity_flags_are_reported_independently() {
        let mut debug = BddDebugConfig::default();
        debug.age_tracking = true;
        let age_config = BddPackageConfig::legacy_default().with_debug(debug.clone());
        assert!(age_config.uses_debug_node_age());
        assert!(!age_config.uses_debug_node_identity());

        debug.age_tracking = false;
        debug.unique_id_tracking = true;
        let identity_config = BddPackageConfig::legacy_default().with_debug(debug);
        assert!(!identity_config.uses_debug_node_age());
        assert!(identity_config.uses_debug_node_identity());
    }

    #[test]
    fn inline_ite_switches_can_be_overridden_for_native_tests() {
        let config = BddPackageConfig::legacy_default()
            .with_inline_ite(false)
            .with_inline_ite_constant(false);

        assert!(!config.inline_ite);
        assert!(!config.inline_ite_constant);
    }
}
