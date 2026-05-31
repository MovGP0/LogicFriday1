//! Native lifecycle model for `LogicSynthesis/sis/graphics/com_graphics.c`.
//!
//! The original C file intentionally registers no general-purpose graphics
//! commands. SIS graphics data is emitted through the command package helper
//! routines; this module only represents the graphics package init/end hooks in
//! a native, testable form.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GraphicsLifecycle {
    initialized: bool,
    init_count: usize,
    end_count: usize,
}

impl GraphicsLifecycle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialized(&self) -> bool {
        self.initialized
    }

    pub fn init_count(&self) -> usize {
        self.init_count
    }

    pub fn end_count(&self) -> usize {
        self.end_count
    }

    pub fn init_graphics(&mut self) {
        self.initialized = true;
        self.init_count += 1;
    }

    pub fn end_graphics(&mut self) {
        self.initialized = false;
        self.end_count += 1;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub changes_network: bool,
}

pub const GRAPHICS_COMMANDS: &[CommandRegistration] = &[];

pub fn graphics_command_registrations() -> &'static [CommandRegistration] {
    GRAPHICS_COMMANDS
}

pub fn init_graphics(lifecycle: &mut GraphicsLifecycle) {
    lifecycle.init_graphics();
}

pub fn end_graphics(lifecycle: &mut GraphicsLifecycle) {
    lifecycle.end_graphics();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_registration_list_is_empty_like_the_c_module() {
        assert_eq!(graphics_command_registrations(), &[]);
    }

    #[test]
    fn lifecycle_starts_uninitialized() {
        let lifecycle = GraphicsLifecycle::new();

        assert!(!lifecycle.initialized());
        assert_eq!(lifecycle.init_count(), 0);
        assert_eq!(lifecycle.end_count(), 0);
    }

    #[test]
    fn init_graphics_marks_lifecycle_initialized() {
        let mut lifecycle = GraphicsLifecycle::new();

        init_graphics(&mut lifecycle);

        assert!(lifecycle.initialized());
        assert_eq!(lifecycle.init_count(), 1);
        assert_eq!(lifecycle.end_count(), 0);
    }

    #[test]
    fn end_graphics_marks_lifecycle_stopped() {
        let mut lifecycle = GraphicsLifecycle::new();

        init_graphics(&mut lifecycle);
        end_graphics(&mut lifecycle);

        assert!(!lifecycle.initialized());
        assert_eq!(lifecycle.init_count(), 1);
        assert_eq!(lifecycle.end_count(), 1);
    }

    #[test]
    fn lifecycle_hooks_are_idempotent_from_a_resource_perspective() {
        let mut lifecycle = GraphicsLifecycle::new();

        init_graphics(&mut lifecycle);
        init_graphics(&mut lifecycle);
        end_graphics(&mut lifecycle);
        end_graphics(&mut lifecycle);

        assert!(!lifecycle.initialized());
        assert_eq!(lifecycle.init_count(), 2);
        assert_eq!(lifecycle.end_count(), 2);
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_c_abi_exports() {
        let source = include_str!("com_graphics.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
