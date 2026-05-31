//! Source-aligned facade for Espresso `main.c`.
//!
//! The original C file owns the process-level CLI entry point. Logic Friday
//! uses this crate as a library, so `driver.rs` contains the reusable command
//! parsing and mode mapping while this module preserves the `main.c` port
//! boundary for Beads/source tracking.

pub use crate::driver::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cvrout::OutputFormat;
    use crate::minimize::MinimizeMode;
    use crate::pla::FDR_TYPE;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn main_c_facade_exposes_driver_defaults() {
        let command = parse_driver_args(&args(&["espresso"])).unwrap();

        assert_eq!(command.subcommand, EspressoSubcommand::Espresso);
        assert_eq!(
            command.logic_friday_options().unwrap().mode,
            MinimizeMode::FastJoint
        );
    }

    #[test]
    fn main_c_facade_exposes_exact_and_output_mode_mapping() {
        let command = parse_driver_args(&args(&["espresso", "-Dexact", "-ofdr", "-x"])).unwrap();

        assert_eq!(
            command.logic_friday_options().unwrap().mode,
            MinimizeMode::ExactJoint
        );
        assert_eq!(command.output, OutputFormat::Pla(FDR_TYPE));
        assert!(!command.print_solution);
    }
}
