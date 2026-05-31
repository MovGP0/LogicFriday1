//! Source-aligned command driver pieces from Espresso `main.c`.
//!
//! Logic Friday calls the Rust library directly, so the native port does not
//! need a process-level `main`. This module keeps the command option mapping
//! from `main.c`: `-D` subcommands, `-S` strategy values, output selectors, and
//! the four minimization modes used by Logic Friday.

use crate::cvrout::OutputFormat;
use crate::getopt::{GetOpt, GetOptItem};
use crate::minimize::MinimizeOptions;
use crate::pla::{F_TYPE, FD_TYPE, FDR_TYPE, FR_TYPE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspressoCommand {
    pub subcommand: EspressoSubcommand,
    pub strategy: i32,
    pub output: OutputFormat,
    pub print_solution: bool,
}

impl Default for EspressoCommand {
    fn default() -> Self {
        Self {
            subcommand: EspressoSubcommand::Espresso,
            strategy: 0,
            output: OutputFormat::Pla(F_TYPE),
            print_solution: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EspressoSubcommand {
    Espresso,
    SingleOutput,
    Exact,
    Verify,
    Phase,
    Pair,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverError {
    UnknownOption(char),
    MissingOptionArgument(char),
    UnknownSubcommand(String),
    UnknownOutput(String),
}

impl EspressoCommand {
    pub fn logic_friday_options(&self) -> Option<MinimizeOptions> {
        match self.subcommand {
            EspressoSubcommand::Espresso => Some(MinimizeOptions::fast_joint()),
            EspressoSubcommand::SingleOutput if self.strategy == 1 => {
                Some(MinimizeOptions::exact_independent_output())
            }
            EspressoSubcommand::SingleOutput => Some(MinimizeOptions::fast_independent_output()),
            EspressoSubcommand::Exact => Some(MinimizeOptions::exact_joint()),
            _ => None,
        }
    }
}

pub fn parse_driver_args(argv: &[String]) -> Result<EspressoCommand, DriverError> {
    let mut command = EspressoCommand::default();
    let mut getopt = GetOpt::default();

    loop {
        match getopt.next(argv, "D:S:o:x") {
            GetOptItem::End => return Ok(command),
            GetOptItem::Unknown(option) => return Err(DriverError::UnknownOption(option)),
            GetOptItem::Option('x') => command.print_solution = false,
            GetOptItem::Option('S') => {
                let value = getopt
                    .optarg()
                    .ok_or(DriverError::MissingOptionArgument('S'))?;
                command.strategy = value
                    .parse()
                    .map_err(|_| DriverError::MissingOptionArgument('S'))?;
            }
            GetOptItem::Option('D') => {
                let value = getopt
                    .optarg()
                    .ok_or(DriverError::MissingOptionArgument('D'))?;
                command.subcommand = parse_subcommand(value)?;
            }
            GetOptItem::Option('o') => {
                let value = getopt
                    .optarg()
                    .ok_or(DriverError::MissingOptionArgument('o'))?;
                command.output = parse_output(value)?;
            }
            GetOptItem::Option(option) => return Err(DriverError::UnknownOption(option)),
        }
    }
}

fn parse_subcommand(value: &str) -> Result<EspressoSubcommand, DriverError> {
    match value {
        "espresso" => Ok(EspressoSubcommand::Espresso),
        "so" => Ok(EspressoSubcommand::SingleOutput),
        "exact" => Ok(EspressoSubcommand::Exact),
        "verify" => Ok(EspressoSubcommand::Verify),
        "opo" => Ok(EspressoSubcommand::Phase),
        "pair" => Ok(EspressoSubcommand::Pair),
        _ => Err(DriverError::UnknownSubcommand(value.to_string())),
    }
}

fn parse_output(value: &str) -> Result<OutputFormat, DriverError> {
    match value {
        "f" => Ok(OutputFormat::Pla(F_TYPE)),
        "fd" => Ok(OutputFormat::Pla(FD_TYPE)),
        "fr" => Ok(OutputFormat::Pla(FR_TYPE)),
        "fdr" => Ok(OutputFormat::Pla(FDR_TYPE)),
        "eqntott" => Ok(OutputFormat::EqnTott),
        "pleasure" => Ok(OutputFormat::Pleasure),
        "kiss" => Ok(OutputFormat::Kiss),
        _ => Err(DriverError::UnknownOutput(value.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minimize::MinimizeMode;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn default_driver_matches_main_c_espresso_defaults() {
        let command = parse_driver_args(&args(&["espresso"])).unwrap();

        assert_eq!(command.subcommand, EspressoSubcommand::Espresso);
        assert_eq!(command.output, OutputFormat::Pla(F_TYPE));
        assert_eq!(
            command.logic_friday_options().unwrap().mode,
            MinimizeMode::FastJoint
        );
    }

    #[test]
    fn single_output_strategy_maps_to_logic_friday_modes() {
        let fast = parse_driver_args(&args(&["espresso", "-Dso"])).unwrap();
        let exact = parse_driver_args(&args(&["espresso", "-Dso", "-S1"])).unwrap();

        assert_eq!(
            fast.logic_friday_options().unwrap().mode,
            MinimizeMode::FastIndependentOutput
        );
        assert_eq!(
            exact.logic_friday_options().unwrap().mode,
            MinimizeMode::ExactIndependentOutput
        );
    }

    #[test]
    fn exact_and_output_options_parse_like_main_c_tables() {
        let command = parse_driver_args(&args(&["espresso", "-Dexact", "-ofdr", "-x"])).unwrap();

        assert_eq!(
            command.logic_friday_options().unwrap().mode,
            MinimizeMode::ExactJoint
        );
        assert_eq!(command.output, OutputFormat::Pla(FDR_TYPE));
        assert!(!command.print_solution);
    }
}
