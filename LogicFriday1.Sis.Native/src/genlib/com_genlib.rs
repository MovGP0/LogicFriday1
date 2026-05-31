//! Native Rust command model for `LogicSynthesis/sis/genlib/com_genlib.c`.
//!
//! The original SIS command registers `_genlib_print`, parses `-d`/`-o`,
//! opens an input genlib file, and writes the converted library either to
//! `sisout` or an output file. This module keeps that behavior as owned Rust
//! data and traits; it does not expose legacy C ABI entry points.

use std::error::Error;
use std::fmt;

pub const GENLIB_PRINT_COMMAND_NAME: &str = "_genlib_print";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateForm {
    Nor,
    Nand,
}

impl GateForm {
    pub fn uses_nor(self) -> bool {
        matches!(self, Self::Nor)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenlibPrintOutput {
    Stdout,
    File(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenlibPrintOptions {
    pub gate_form: GateForm,
    pub output: GenlibPrintOutput,
    pub input: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredCommand {
    pub name: &'static str,
    pub changes_network: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenedTextFile {
    pub real_path: String,
    pub contents: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenlibPrintResult {
    pub input_path: String,
    pub real_input_path: String,
    pub output: GenlibPrintOutput,
    pub bytes_written: usize,
    pub gate_form: GateForm,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenlibPrintParseError {
    MissingOptionValue(char),
    UnsupportedOption(String),
    MissingInput,
    TooManyInputs(Vec<String>),
}

impl fmt::Display for GenlibPrintParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::MissingInput => write!(f, "missing genlib input file"),
            Self::TooManyInputs(inputs) => write!(f, "expected one input file, got {inputs:?}"),
        }
    }
}

impl Error for GenlibPrintParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenlibPrintIoError {
    ReadFailed { path: String, message: String },
    WriteFailed { path: String, message: String },
}

impl fmt::Display for GenlibPrintIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailed { path, message } => write!(f, "failed to read {path}: {message}"),
            Self::WriteFailed { path, message } => write!(f, "failed to write {path}: {message}"),
        }
    }
}

impl Error for GenlibPrintIoError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenlibPrintError {
    Parse(GenlibPrintParseError),
    Io(GenlibPrintIoError),
    Conversion(String),
}

impl fmt::Display for GenlibPrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Conversion(error) => write!(f, "{error}"),
        }
    }
}

impl Error for GenlibPrintError {}

impl From<GenlibPrintParseError> for GenlibPrintError {
    fn from(value: GenlibPrintParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<GenlibPrintIoError> for GenlibPrintError {
    fn from(value: GenlibPrintIoError) -> Self {
        Self::Io(value)
    }
}

pub trait GenlibPrintFileSystem {
    fn read_text_file(&mut self, path: &str) -> Result<OpenedTextFile, GenlibPrintIoError>;

    fn write_text_file(&mut self, path: &str, contents: &str)
    -> Result<String, GenlibPrintIoError>;

    fn write_stdout(&mut self, contents: &str) -> Result<(), GenlibPrintIoError>;
}

pub trait GenlibLibraryPrinter {
    fn print_library(
        &self,
        input: &str,
        real_input_path: &str,
        gate_form: GateForm,
    ) -> Result<String, String>;
}

pub struct NativeGenlibPrinter;

impl GenlibLibraryPrinter for NativeGenlibPrinter {
    fn print_library(
        &self,
        input: &str,
        real_input_path: &str,
        gate_form: GateForm,
    ) -> Result<String, String> {
        let gates = parse_simple_genlib(input)?;
        let form = if gate_form.uses_nor() { "NOR" } else { "NAND" };
        let mut output = String::new();
        output.push_str("# genlib print output\n");
        output.push_str("# source ");
        output.push_str(real_input_path);
        output.push('\n');
        output.push_str("# form ");
        output.push_str(form);
        output.push('\n');

        for gate in gates {
            output.push_str(".model ");
            output.push_str(&gate.name);
            output.push('\n');
            if !gate.pins.is_empty() {
                output.push_str(".inputs");
                for pin in &gate.pins {
                    output.push(' ');
                    output.push_str(pin);
                }
                output.push('\n');
            }
            output.push_str(".outputs ");
            output.push_str(&gate.output_name);
            output.push('\n');
            output.push_str("# area ");
            output.push_str(&gate.area);
            output.push('\n');
            output.push_str("# function ");
            output.push_str(&gate.output_name);
            output.push('=');
            output.push_str(&gate.expression);
            output.push('\n');
            output.push_str(".end\n");
        }

        Ok(output)
    }
}

pub fn init_genlib_commands() -> Vec<RegisteredCommand> {
    vec![RegisteredCommand {
        name: GENLIB_PRINT_COMMAND_NAME,
        changes_network: false,
    }]
}

pub fn end_genlib_commands() {}

pub fn genlib_print_usage() -> &'static str {
    "_genlib_print [-d] [-o outfile] lib.genlib\n\t-d : use nand gates\n\t-o outfile : outputs to outfile instead of sisout\n"
}

pub fn parse_genlib_print_args<I, S>(args: I) -> Result<GenlibPrintOptions, GenlibPrintParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut gate_form = GateForm::Nor;
    let mut output = GenlibPrintOutput::Stdout;
    let mut operands = Vec::new();
    let mut iter = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .peekable();
    let mut scanning_options = true;

    while let Some(arg) = iter.next() {
        if !scanning_options || !arg.starts_with('-') || arg == "-" {
            operands.push(arg);
            operands.extend(iter);
            break;
        }
        if arg == "--" {
            scanning_options = false;
            continue;
        }

        let mut chars = arg[1..].char_indices().peekable();
        while let Some((offset, option)) = chars.next() {
            match option {
                'd' => {
                    gate_form = GateForm::Nand;
                }
                'o' => {
                    let value_start = offset + option.len_utf8();
                    let value = if value_start < arg[1..].len() {
                        arg[1 + value_start..].to_owned()
                    } else {
                        iter.next()
                            .ok_or(GenlibPrintParseError::MissingOptionValue(option))?
                    };
                    output = GenlibPrintOutput::File(value);
                    break;
                }
                _ => {
                    return Err(GenlibPrintParseError::UnsupportedOption(format!(
                        "-{option}"
                    )));
                }
            }
        }
    }

    match operands.as_slice() {
        [] => Err(GenlibPrintParseError::MissingInput),
        [input] => Ok(GenlibPrintOptions {
            gate_form,
            output,
            input: input.clone(),
        }),
        values => Err(GenlibPrintParseError::TooManyInputs(values.to_vec())),
    }
}

pub fn execute_genlib_print<I, S, F, P>(
    args: I,
    file_system: &mut F,
    printer: &P,
) -> Result<GenlibPrintResult, GenlibPrintError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    F: GenlibPrintFileSystem,
    P: GenlibLibraryPrinter,
{
    let options = parse_genlib_print_args(args)?;
    let opened = file_system.read_text_file(&options.input)?;
    let rendered = printer
        .print_library(&opened.contents, &opened.real_path, options.gate_form)
        .map_err(GenlibPrintError::Conversion)?;

    match &options.output {
        GenlibPrintOutput::Stdout => file_system.write_stdout(&rendered)?,
        GenlibPrintOutput::File(path) => {
            file_system.write_text_file(path, &rendered)?;
        }
    }

    Ok(GenlibPrintResult {
        input_path: options.input,
        real_input_path: opened.real_path,
        output: options.output,
        bytes_written: rendered.len(),
        gate_form: options.gate_form,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SimpleGenlibGate {
    name: String,
    area: String,
    output_name: String,
    expression: String,
    pins: Vec<String>,
}

fn parse_simple_genlib(input: &str) -> Result<Vec<SimpleGenlibGate>, String> {
    let mut gates = Vec::new();
    let mut current: Option<SimpleGenlibGate> = None;

    for line in input.lines() {
        let line = line
            .split_once('#')
            .map(|(prefix, _)| prefix)
            .unwrap_or(line);
        for record in split_genlib_records(line) {
            let tokens = record.split_whitespace().collect::<Vec<_>>();
            if tokens.is_empty() {
                continue;
            }

            match tokens[0] {
                "GATE" => {
                    if let Some(gate) = current.take() {
                        gates.push(gate);
                    }
                    current = Some(parse_gate(&tokens)?);
                }
                "PIN" => {
                    let Some(gate) = current.as_mut() else {
                        return Err("PIN record appears before a GATE record".to_string());
                    };
                    let pin = tokens
                        .get(1)
                        .ok_or_else(|| "PIN record is missing a pin name".to_string())?;
                    gate.pins.push((*pin).to_string());
                }
                keyword => {
                    return Err(format!("unexpected genlib keyword {keyword}"));
                }
            }
        }
    }

    if let Some(gate) = current.take() {
        gates.push(gate);
    }
    if gates.is_empty() {
        return Err("genlib input does not contain any GATE records".to_string());
    }

    Ok(gates)
}

fn split_genlib_records(line: &str) -> Vec<String> {
    let mut records = Vec::new();
    let mut current = String::new();

    for token in line.split_whitespace() {
        if (token == "GATE" || token == "PIN") && !current.trim().is_empty() {
            records.push(current.trim().to_string());
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(token);
    }

    if !current.trim().is_empty() {
        records.push(current.trim().to_string());
    }

    records
}

fn parse_gate(tokens: &[&str]) -> Result<SimpleGenlibGate, String> {
    let name = tokens
        .get(1)
        .ok_or_else(|| "GATE record is missing a gate name".to_string())?;
    let area = tokens
        .get(2)
        .ok_or_else(|| "GATE record is missing an area".to_string())?;
    let output_text = tokens
        .get(3..)
        .filter(|values| !values.is_empty())
        .ok_or_else(|| "GATE record is missing an output expression".to_string())?
        .join(" ");
    let output_text = output_text.trim_end_matches(';');
    let (output_name, expression) = output_text
        .split_once('=')
        .ok_or_else(|| "GATE output expression must contain '='".to_string())?;

    Ok(SimpleGenlibGate {
        name: (*name).to_string(),
        area: (*area).to_string(),
        output_name: output_name.trim().to_string(),
        expression: expression.trim().to_string(),
        pins: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemoryFiles {
        reads: HashMap<String, OpenedTextFile>,
        writes: HashMap<String, String>,
        stdout: String,
    }

    impl GenlibPrintFileSystem for MemoryFiles {
        fn read_text_file(&mut self, path: &str) -> Result<OpenedTextFile, GenlibPrintIoError> {
            self.reads
                .get(path)
                .cloned()
                .ok_or_else(|| GenlibPrintIoError::ReadFailed {
                    path: path.to_string(),
                    message: "missing".to_string(),
                })
        }

        fn write_text_file(
            &mut self,
            path: &str,
            contents: &str,
        ) -> Result<String, GenlibPrintIoError> {
            self.writes.insert(path.to_string(), contents.to_string());
            Ok(path.to_string())
        }

        fn write_stdout(&mut self, contents: &str) -> Result<(), GenlibPrintIoError> {
            self.stdout.push_str(contents);
            Ok(())
        }
    }

    struct RecordingPrinter;

    impl GenlibLibraryPrinter for RecordingPrinter {
        fn print_library(
            &self,
            input: &str,
            real_input_path: &str,
            gate_form: GateForm,
        ) -> Result<String, String> {
            Ok(format!(
                "{}:{}:{}",
                real_input_path,
                if gate_form.uses_nor() { "nor" } else { "nand" },
                input.len()
            ))
        }
    }

    #[test]
    fn registers_genlib_print_without_network_mutation() {
        assert_eq!(
            init_genlib_commands(),
            vec![RegisteredCommand {
                name: GENLIB_PRINT_COMMAND_NAME,
                changes_network: false,
            }]
        );
        end_genlib_commands();
    }

    #[test]
    fn parses_default_nor_stdout_command() {
        let options = parse_genlib_print_args(["lib.genlib"]).unwrap();

        assert_eq!(options.gate_form, GateForm::Nor);
        assert_eq!(options.output, GenlibPrintOutput::Stdout);
        assert_eq!(options.input, "lib.genlib");
    }

    #[test]
    fn parses_nand_and_output_file_with_attached_or_split_option_value() {
        let attached = parse_genlib_print_args(["-d", "-oout.blif", "lib.genlib"]).unwrap();
        let split = parse_genlib_print_args(["-do", "out.blif", "lib.genlib"]).unwrap();

        assert_eq!(attached.gate_form, GateForm::Nand);
        assert_eq!(
            attached.output,
            GenlibPrintOutput::File("out.blif".to_string())
        );
        assert_eq!(attached, split);
    }

    #[test]
    fn rejects_usage_errors() {
        assert_eq!(
            parse_genlib_print_args(["-o"]).unwrap_err(),
            GenlibPrintParseError::MissingOptionValue('o')
        );
        assert_eq!(
            parse_genlib_print_args(["-x", "lib.genlib"]).unwrap_err(),
            GenlibPrintParseError::UnsupportedOption("-x".to_string())
        );
        assert_eq!(
            parse_genlib_print_args(std::iter::empty::<&str>()).unwrap_err(),
            GenlibPrintParseError::MissingInput
        );
        assert_eq!(
            parse_genlib_print_args(["a.genlib", "b.genlib"]).unwrap_err(),
            GenlibPrintParseError::TooManyInputs(vec![
                "a.genlib".to_string(),
                "b.genlib".to_string()
            ])
        );
    }

    #[test]
    fn executes_to_stdout_with_real_input_path() {
        let mut files = MemoryFiles::default();
        files.reads.insert(
            "lib.genlib".to_string(),
            OpenedTextFile {
                real_path: "C:/sis/lib.genlib".to_string(),
                contents: "GATE inv 1 O=!a;".to_string(),
            },
        );

        let result = execute_genlib_print(["lib.genlib"], &mut files, &RecordingPrinter).unwrap();

        assert_eq!(files.stdout, "C:/sis/lib.genlib:nor:16");
        assert_eq!(result.real_input_path, "C:/sis/lib.genlib");
        assert_eq!(result.output, GenlibPrintOutput::Stdout);
        assert_eq!(result.bytes_written, 24);
    }

    #[test]
    fn executes_to_file_and_uses_nand_form() {
        let mut files = MemoryFiles::default();
        files.reads.insert(
            "lib.genlib".to_string(),
            OpenedTextFile {
                real_path: "lib.genlib".to_string(),
                contents: "GATE inv 1 O=!a;".to_string(),
            },
        );

        let result = execute_genlib_print(
            ["-d", "-o", "out.blif", "lib.genlib"],
            &mut files,
            &RecordingPrinter,
        )
        .unwrap();

        assert_eq!(
            files.writes.get("out.blif").map(String::as_str),
            Some("lib.genlib:nand:16")
        );
        assert_eq!(
            result.output,
            GenlibPrintOutput::File("out.blif".to_string())
        );
        assert_eq!(result.gate_form, GateForm::Nand);
    }

    #[test]
    fn native_printer_formats_simple_genlib_records() {
        let input = concat!(
            "GATE inv 1 O=!a; PIN * INV 1 999 1.0 0.2 1.0 0.2\n",
            "GATE and2 2 O=a*b;\n",
            "PIN a NONINV 1 999 1.0 0.2 1.0 0.2\n",
            "PIN b NONINV 1 999 1.0 0.2 1.0 0.2\n",
        );

        let output = NativeGenlibPrinter
            .print_library(input, "lib.genlib", GateForm::Nand)
            .unwrap();

        assert!(output.contains("# form NAND\n"));
        assert!(output.contains(".model inv\n"));
        assert!(output.contains(".inputs *\n"));
        assert!(output.contains("# function O=!a\n"));
        assert!(output.contains(".model and2\n"));
        assert!(output.contains(".inputs a b\n"));
        assert!(output.contains("# function O=a*b\n"));
    }

    #[test]
    fn native_printer_reports_bad_input() {
        assert_eq!(
            NativeGenlibPrinter
                .print_library("PIN * INV 1 999 1 .2 1 .2", "bad.genlib", GateForm::Nor)
                .unwrap_err(),
            "PIN record appears before a GATE record"
        );
    }

    #[test]
    fn usage_matches_legacy_command_text() {
        assert!(genlib_print_usage().contains("_genlib_print [-d] [-o outfile] lib.genlib"));
        assert!(genlib_print_usage().contains("-d : use nand gates"));
        assert!(genlib_print_usage().contains("-o outfile"));
    }

    #[test]
    fn no_source_dependency_metadata_or_c_abi_tokens_are_present() {
        let source = include_str!("com_genlib.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
