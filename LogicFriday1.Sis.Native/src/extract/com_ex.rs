//! Native Rust command model for SIS extraction commands.
//!
//! The legacy module registers `gkx`, `gcx`, `print_kernel`, `_gdiv`, `_qdiv`,
//! and `fx`, then delegates almost all work to extraction, sparse-matrix,
//! network, node, and command-table APIs. This port keeps the deterministic
//! command surface native: option parsing, command registration metadata,
//! dispatch intent, and fast-extract default state. The network-mutating
//! algorithms remain behind a Rust backend trait until the surrounding SIS
//! extraction and network ports are wired together.

use std::error::Error;
use std::fmt;

pub const CUBE_EXTRACT_USAGE: &str = "usage: gcx [-bcdf] [-t thresh]";
pub const KERNEL_EXTRACT_USAGE: &str = "usage: gkx [-1abcdfo] [-t thresh]";
pub const PRINT_KERNEL_USAGE: &str = "usage: print_kernel [-as] n1 n2 ...";
pub const GDIV_USAGE: &str = "usage: _gdiv n1 n2 ...";
pub const QDIV_USAGE: &str = "usage: _qdiv n1 n2 ...";
pub const FAST_EXTRACT_USAGE: &str = "usage: fx [-o] [-b limit] [-l] [-z]";

pub const FAST_EXTRACT_DEFAULT_LENGTH1: i32 = 5;
pub const FAST_EXTRACT_DEFAULT_LENGTH2: i32 = 5;
pub const FAST_EXTRACT_DEFAULT_OBJECT_SIZE: i32 = 50_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtractCommandKind {
    KernelExtract,
    CubeExtract,
    PrintKernel,
    GoodDivisor,
    QuickDivisor,
    FastExtract,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: ExtractCommandKind,
    pub changes_network: bool,
}

pub const EXTRACT_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "gkx",
        kind: ExtractCommandKind::KernelExtract,
        changes_network: true,
    },
    CommandRegistration {
        name: "gcx",
        kind: ExtractCommandKind::CubeExtract,
        changes_network: true,
    },
    CommandRegistration {
        name: "print_kernel",
        kind: ExtractCommandKind::PrintKernel,
        changes_network: true,
    },
    CommandRegistration {
        name: "_gdiv",
        kind: ExtractCommandKind::GoodDivisor,
        changes_network: false,
    },
    CommandRegistration {
        name: "_qdiv",
        kind: ExtractCommandKind::QuickDivisor,
        changes_network: false,
    },
    CommandRegistration {
        name: "fx",
        kind: ExtractCommandKind::FastExtract,
        changes_network: true,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubcubeSelection {
    PingPong,
    BestSubcube,
    FactoredLiteralValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubkernelSelection {
    PingPong,
    BestSubkernel,
    FactoredLiteralValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeExtractOptions {
    pub selection: SubcubeSelection,
    pub use_complement: bool,
    pub debug_level: i32,
    pub threshold: i32,
}

impl Default for CubeExtractOptions {
    fn default() -> Self {
        Self {
            selection: SubcubeSelection::PingPong,
            use_complement: false,
            debug_level: 0,
            threshold: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelExtractOptions {
    pub one_pass: bool,
    pub use_all_kernels: bool,
    pub selection: SubkernelSelection,
    pub use_complement: bool,
    pub debug_level: i32,
    pub use_overlap: bool,
    pub threshold: i32,
}

impl Default for KernelExtractOptions {
    fn default() -> Self {
        Self {
            one_pass: false,
            use_all_kernels: false,
            selection: SubkernelSelection::PingPong,
            use_complement: false,
            debug_level: 0,
            use_overlap: false,
            threshold: 0,
        }
    }
}

impl KernelExtractOptions {
    pub fn needs_duplicate_network(&self) -> bool {
        self.use_overlap
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrintKernelOptions {
    pub include_all_levels: bool,
    pub print_subkernels: bool,
    pub nodes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DivisorKind {
    Good,
    Quick,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DivisorOptions {
    pub kind: DivisorKind,
    pub nodes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FastExtractOptions {
    pub one_pass: bool,
    pub delete_when_large: bool,
    pub preserve_level: bool,
    pub length1: i32,
    pub length2: i32,
    pub object_size: i32,
    pub dont_use_weight_zero: bool,
}

impl Default for FastExtractOptions {
    fn default() -> Self {
        Self {
            one_pass: false,
            delete_when_large: false,
            preserve_level: false,
            length1: FAST_EXTRACT_DEFAULT_LENGTH1,
            length2: FAST_EXTRACT_DEFAULT_LENGTH2,
            object_size: FAST_EXTRACT_DEFAULT_OBJECT_SIZE,
            dont_use_weight_zero: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtractCommand {
    CubeExtract(CubeExtractOptions),
    KernelExtract(KernelExtractOptions),
    PrintKernel(PrintKernelOptions),
    Divisor(DivisorOptions),
    FastExtract(FastExtractOptions),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtractOperation {
    RegisterCommands,
    CleanupSparseMatrixPackage,
    KernelExtract,
    CubeExtract,
    PrintKernel,
    GoodDivisor,
    QuickDivisor,
    FastExtract,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtractCommandError {
    UnknownCommand(String),
    MissingOptionValue(char),
    UnsupportedOption(String),
    UnexpectedOperands {
        command: ExtractCommandKind,
        operands: Vec<String>,
        usage: &'static str,
    },
    MissingOperands {
        command: ExtractCommandKind,
        usage: &'static str,
    },
    MissingNativePorts {
        operation: ExtractOperation,
    },
    Backend(String),
}

impl fmt::Display for ExtractCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(formatter, "unknown extract command {command}"),
            Self::MissingOptionValue(option) => write!(formatter, "-{option} requires an argument"),
            Self::UnsupportedOption(option) => write!(formatter, "unsupported option {option}"),
            Self::UnexpectedOperands { usage, .. } => formatter.write_str(usage),
            Self::MissingOperands { usage, .. } => formatter.write_str(usage),
            Self::MissingNativePorts { operation } => {
                write!(
                    formatter,
                    "operation {operation:?} requires native SIS prerequisite ports"
                )
            }
            Self::Backend(message) => formatter.write_str(message),
        }
    }
}

impl Error for ExtractCommandError {}

pub trait ExtractBackend {
    fn cube_extract(&mut self, options: &CubeExtractOptions) -> Result<i32, ExtractCommandError>;

    fn kernel_extract(
        &mut self,
        options: &KernelExtractOptions,
    ) -> Result<i32, ExtractCommandError>;

    fn print_kernel(&mut self, options: &PrintKernelOptions) -> Result<i32, ExtractCommandError>;

    fn find_divisor(&mut self, options: &DivisorOptions) -> Result<i32, ExtractCommandError>;

    fn fast_extract(&mut self, options: &FastExtractOptions) -> Result<i32, ExtractCommandError>;
}

#[derive(Default)]
pub struct MissingExtractBackend;

impl ExtractBackend for MissingExtractBackend {
    fn cube_extract(&mut self, _options: &CubeExtractOptions) -> Result<i32, ExtractCommandError> {
        Err(missing(ExtractOperation::CubeExtract))
    }

    fn kernel_extract(
        &mut self,
        _options: &KernelExtractOptions,
    ) -> Result<i32, ExtractCommandError> {
        Err(missing(ExtractOperation::KernelExtract))
    }

    fn print_kernel(&mut self, _options: &PrintKernelOptions) -> Result<i32, ExtractCommandError> {
        Err(missing(ExtractOperation::PrintKernel))
    }

    fn find_divisor(&mut self, options: &DivisorOptions) -> Result<i32, ExtractCommandError> {
        let operation = match options.kind {
            DivisorKind::Good => ExtractOperation::GoodDivisor,
            DivisorKind::Quick => ExtractOperation::QuickDivisor,
        };

        Err(missing(operation))
    }

    fn fast_extract(&mut self, _options: &FastExtractOptions) -> Result<i32, ExtractCommandError> {
        Err(missing(ExtractOperation::FastExtract))
    }
}

pub fn extract_command_registrations() -> &'static [CommandRegistration] {
    EXTRACT_COMMANDS
}

pub fn register_extract_commands() -> Result<&'static [CommandRegistration], ExtractCommandError> {
    Err(missing(ExtractOperation::RegisterCommands))
}

pub fn cleanup_extract_package() -> Result<(), ExtractCommandError> {
    Err(missing(ExtractOperation::CleanupSparseMatrixPackage))
}

pub fn parse_extract_command<I, S>(
    command_name: &str,
    args: I,
) -> Result<ExtractCommand, ExtractCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match command_name {
        "gcx" => parse_cube_extract_args(args).map(ExtractCommand::CubeExtract),
        "gkx" => parse_kernel_extract_args(args).map(ExtractCommand::KernelExtract),
        "print_kernel" => parse_print_kernel_args(args).map(ExtractCommand::PrintKernel),
        "_gdiv" => parse_divisor_args(DivisorKind::Good, args).map(ExtractCommand::Divisor),
        "_qdiv" => parse_divisor_args(DivisorKind::Quick, args).map(ExtractCommand::Divisor),
        "fx" => parse_fast_extract_args(args).map(ExtractCommand::FastExtract),
        _ => Err(ExtractCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn parse_cube_extract_args<I, S>(args: I) -> Result<CubeExtractOptions, ExtractCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = CubeExtractOptions::default();
    let operands = parse_options(args, "bcdft:v:", |option, value| match option {
        'b' => {
            options.selection = SubcubeSelection::BestSubcube;
            Ok(())
        }
        'c' => {
            options.use_complement = true;
            Ok(())
        }
        'd' => {
            options.debug_level = 1;
            Ok(())
        }
        'f' => {
            options.selection = SubcubeSelection::FactoredLiteralValue;
            Ok(())
        }
        't' => {
            options.threshold = c_atoi(&value);
            Ok(())
        }
        'v' => {
            options.debug_level = c_atoi(&value);
            Ok(())
        }
        _ => Err(ExtractCommandError::UnsupportedOption(format!("-{option}"))),
    })?;

    reject_operands(
        ExtractCommandKind::CubeExtract,
        operands,
        CUBE_EXTRACT_USAGE,
    )?;
    Ok(options)
}

pub fn parse_kernel_extract_args<I, S>(args: I) -> Result<KernelExtractOptions, ExtractCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = KernelExtractOptions::default();
    let operands = parse_options(args, "1abcdfot:v:", |option, value| match option {
        '1' => {
            options.one_pass = true;
            Ok(())
        }
        'a' => {
            options.use_all_kernels = true;
            Ok(())
        }
        'b' => {
            options.selection = SubkernelSelection::BestSubkernel;
            Ok(())
        }
        'c' => {
            options.use_complement = true;
            Ok(())
        }
        'd' => {
            options.debug_level = 1;
            Ok(())
        }
        'f' => {
            options.selection = SubkernelSelection::FactoredLiteralValue;
            Ok(())
        }
        'o' => {
            options.use_overlap = true;
            Ok(())
        }
        't' => {
            options.threshold = c_atoi(&value);
            Ok(())
        }
        'v' => {
            options.debug_level = c_atoi(&value);
            Ok(())
        }
        _ => Err(ExtractCommandError::UnsupportedOption(format!("-{option}"))),
    })?;

    reject_operands(
        ExtractCommandKind::KernelExtract,
        operands,
        KERNEL_EXTRACT_USAGE,
    )?;
    Ok(options)
}

pub fn parse_print_kernel_args<I, S>(args: I) -> Result<PrintKernelOptions, ExtractCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut include_all_levels = false;
    let mut print_subkernels = false;
    let nodes = parse_options(args, "as", |option, _value| match option {
        'a' => {
            include_all_levels = true;
            Ok(())
        }
        's' => {
            print_subkernels = true;
            Ok(())
        }
        _ => Err(ExtractCommandError::UnsupportedOption(format!("-{option}"))),
    })?;

    Ok(PrintKernelOptions {
        include_all_levels,
        print_subkernels,
        nodes,
    })
}

pub fn parse_divisor_args<I, S>(
    kind: DivisorKind,
    args: I,
) -> Result<DivisorOptions, ExtractCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let nodes = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect::<Vec<_>>();

    if nodes.is_empty() {
        let (command, usage) = match kind {
            DivisorKind::Good => (ExtractCommandKind::GoodDivisor, GDIV_USAGE),
            DivisorKind::Quick => (ExtractCommandKind::QuickDivisor, QDIV_USAGE),
        };

        return Err(ExtractCommandError::MissingOperands { command, usage });
    }

    Ok(DivisorOptions { kind, nodes })
}

pub fn parse_fast_extract_args<I, S>(args: I) -> Result<FastExtractOptions, ExtractCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = FastExtractOptions::default();
    let operands = parse_options(args, "b:f:s:loz", |option, value| match option {
        'o' => {
            options.one_pass = true;
            Ok(())
        }
        'l' => {
            options.preserve_level = true;
            Ok(())
        }
        'b' => {
            options.one_pass = true;
            options.delete_when_large = true;
            options.object_size = c_atoi(&value);
            Ok(())
        }
        'f' => {
            if options.delete_when_large {
                options.length1 = c_atoi(&value);
            }
            Ok(())
        }
        's' => {
            if options.delete_when_large {
                options.length2 = c_atoi(&value);
            }
            Ok(())
        }
        'z' => {
            options.dont_use_weight_zero = false;
            Ok(())
        }
        _ => Err(ExtractCommandError::UnsupportedOption(format!("-{option}"))),
    })?;

    reject_operands(
        ExtractCommandKind::FastExtract,
        operands,
        FAST_EXTRACT_USAGE,
    )?;
    Ok(options)
}

pub fn dispatch_extract_command<B>(
    backend: &mut B,
    command: &ExtractCommand,
) -> Result<i32, ExtractCommandError>
where
    B: ExtractBackend,
{
    match command {
        ExtractCommand::CubeExtract(options) => backend.cube_extract(options),
        ExtractCommand::KernelExtract(options) => backend.kernel_extract(options),
        ExtractCommand::PrintKernel(options) => backend.print_kernel(options),
        ExtractCommand::Divisor(options) => backend.find_divisor(options),
        ExtractCommand::FastExtract(options) => backend.fast_extract(options),
    }
}

pub fn execute_with_missing_dependencies(
    command: &ExtractCommand,
) -> Result<i32, ExtractCommandError> {
    dispatch_extract_command(&mut MissingExtractBackend, command)
}

fn parse_options<F>(
    args: impl IntoIterator<Item = impl AsRef<str>>,
    spec: &str,
    mut apply: F,
) -> Result<Vec<String>, ExtractCommandError>
where
    F: FnMut(char, String) -> Result<(), ExtractCommandError>,
{
    let mut operands = Vec::new();
    let mut iter = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .peekable();
    while let Some(arg) = iter.next() {
        if !arg.starts_with('-') || arg == "-" {
            operands.push(arg);
            operands.extend(iter);
            break;
        }

        if arg == "--" {
            operands.extend(iter);
            break;
        }

        let option_text = &arg[1..];
        let mut chars = option_text.char_indices().peekable();
        while let Some((offset, option)) = chars.next() {
            let needs_value = option_needs_value(spec, option)
                .ok_or_else(|| ExtractCommandError::UnsupportedOption(format!("-{option}")))?;
            if needs_value {
                let value_start = offset + option.len_utf8();
                let value = if value_start < option_text.len() {
                    option_text[value_start..].to_owned()
                } else {
                    iter.next()
                        .ok_or(ExtractCommandError::MissingOptionValue(option))?
                };

                apply(option, value)?;
                break;
            }

            apply(option, String::new())?;
        }
    }

    Ok(operands)
}

fn option_needs_value(spec: &str, option: char) -> Option<bool> {
    let mut chars = spec.chars().peekable();
    while let Some(candidate) = chars.next() {
        if candidate == ':' {
            continue;
        }

        let has_value = chars.peek() == Some(&':');
        if candidate == option {
            return Some(has_value);
        }
    }

    None
}

fn reject_operands(
    command: ExtractCommandKind,
    operands: Vec<String>,
    usage: &'static str,
) -> Result<(), ExtractCommandError> {
    if operands.is_empty() {
        Ok(())
    } else {
        Err(ExtractCommandError::UnexpectedOperands {
            command,
            operands,
            usage,
        })
    }
}

fn c_atoi(text: &str) -> i32 {
    let trimmed = text.trim_start();
    let mut chars = trimmed.chars().peekable();
    let sign = match chars.peek() {
        Some('-') => {
            chars.next();
            -1
        }
        Some('+') => {
            chars.next();
            1
        }
        _ => 1,
    };
    let mut value = 0_i32;
    let mut saw_digit = false;

    while let Some(digit) = chars.peek().and_then(|candidate| candidate.to_digit(10)) {
        saw_digit = true;
        value = value.saturating_mul(10).saturating_add(digit as i32);
        chars.next();
    }

    if saw_digit {
        value.saturating_mul(sign)
    } else {
        0
    }
}

fn missing(operation: ExtractOperation) -> ExtractCommandError {
    ExtractCommandError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        calls: Vec<String>,
    }

    impl ExtractBackend for RecordingBackend {
        fn cube_extract(
            &mut self,
            options: &CubeExtractOptions,
        ) -> Result<i32, ExtractCommandError> {
            self.calls.push(format!("gcx:{options:?}"));
            Ok(0)
        }

        fn kernel_extract(
            &mut self,
            options: &KernelExtractOptions,
        ) -> Result<i32, ExtractCommandError> {
            self.calls.push(format!("gkx:{options:?}"));
            Ok(0)
        }

        fn print_kernel(
            &mut self,
            options: &PrintKernelOptions,
        ) -> Result<i32, ExtractCommandError> {
            self.calls.push(format!("print:{options:?}"));
            Ok(0)
        }

        fn find_divisor(&mut self, options: &DivisorOptions) -> Result<i32, ExtractCommandError> {
            self.calls.push(format!("div:{options:?}"));
            Ok(0)
        }

        fn fast_extract(
            &mut self,
            options: &FastExtractOptions,
        ) -> Result<i32, ExtractCommandError> {
            self.calls.push(format!("fx:{options:?}"));
            Ok(0)
        }
    }

    #[test]
    fn command_registrations_match_legacy_init_order() {
        assert_eq!(
            extract_command_registrations(),
            &[
                CommandRegistration {
                    name: "gkx",
                    kind: ExtractCommandKind::KernelExtract,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "gcx",
                    kind: ExtractCommandKind::CubeExtract,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "print_kernel",
                    kind: ExtractCommandKind::PrintKernel,
                    changes_network: true,
                },
                CommandRegistration {
                    name: "_gdiv",
                    kind: ExtractCommandKind::GoodDivisor,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "_qdiv",
                    kind: ExtractCommandKind::QuickDivisor,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "fx",
                    kind: ExtractCommandKind::FastExtract,
                    changes_network: true,
                },
            ]
        );
    }

    #[test]
    fn parses_cube_extract_options() {
        assert_eq!(
            parse_cube_extract_args(["-cd", "-t", "7", "-v3", "-f"]).unwrap(),
            CubeExtractOptions {
                selection: SubcubeSelection::FactoredLiteralValue,
                use_complement: true,
                debug_level: 3,
                threshold: 7,
            }
        );
    }

    #[test]
    fn parses_kernel_extract_options_and_overlap_duplicate_requirement() {
        let options = parse_kernel_extract_args(["-1aco", "-t-2", "-v", "4", "-b"]).unwrap();

        assert_eq!(
            options,
            KernelExtractOptions {
                one_pass: true,
                use_all_kernels: true,
                selection: SubkernelSelection::BestSubkernel,
                use_complement: true,
                debug_level: 4,
                use_overlap: true,
                threshold: -2,
            }
        );
        assert!(options.needs_duplicate_network());
    }

    #[test]
    fn parses_print_kernel_options_and_node_operands() {
        assert_eq!(
            parse_print_kernel_args(["-as", "n1", "n2"]).unwrap(),
            PrintKernelOptions {
                include_all_levels: true,
                print_subkernels: true,
                nodes: vec!["n1".to_owned(), "n2".to_owned()],
            }
        );
    }

    #[test]
    fn divisor_commands_require_at_least_one_node() {
        assert_eq!(
            parse_divisor_args(DivisorKind::Good, Vec::<String>::new()),
            Err(ExtractCommandError::MissingOperands {
                command: ExtractCommandKind::GoodDivisor,
                usage: GDIV_USAGE,
            })
        );

        assert_eq!(
            parse_divisor_args(DivisorKind::Quick, ["n1"]).unwrap(),
            DivisorOptions {
                kind: DivisorKind::Quick,
                nodes: vec!["n1".to_owned()],
            }
        );
    }

    #[test]
    fn parses_fast_extract_defaults_and_order_sensitive_delete_options() {
        assert_eq!(
            parse_fast_extract_args(["-f", "9", "-s9", "-b", "100", "-f", "2", "-s", "3"]).unwrap(),
            FastExtractOptions {
                one_pass: true,
                delete_when_large: true,
                preserve_level: false,
                length1: 2,
                length2: 3,
                object_size: 100,
                dont_use_weight_zero: true,
            }
        );
    }

    #[test]
    fn parses_fast_extract_level_and_zero_weight_flags() {
        assert_eq!(
            parse_fast_extract_args(["-olz"]).unwrap(),
            FastExtractOptions {
                one_pass: true,
                preserve_level: true,
                dont_use_weight_zero: false,
                ..FastExtractOptions::default()
            }
        );
    }

    #[test]
    fn rejects_operands_for_commands_that_do_not_accept_nodes() {
        assert_eq!(
            parse_cube_extract_args(["node"]).unwrap_err(),
            ExtractCommandError::UnexpectedOperands {
                command: ExtractCommandKind::CubeExtract,
                operands: vec!["node".to_owned()],
                usage: CUBE_EXTRACT_USAGE,
            }
        );
    }

    #[test]
    fn dispatch_uses_native_backend_trait_without_abi_shims() {
        let mut backend = RecordingBackend::default();
        let commands = [
            parse_extract_command("gcx", ["-b"]).unwrap(),
            parse_extract_command("gkx", ["-f"]).unwrap(),
            parse_extract_command("print_kernel", ["n1"]).unwrap(),
            parse_extract_command("_gdiv", ["n2"]).unwrap(),
            parse_extract_command("fx", ["-b10"]).unwrap(),
        ];

        for command in &commands {
            assert_eq!(dispatch_extract_command(&mut backend, command), Ok(0));
        }

        assert_eq!(backend.calls.len(), commands.len());
        assert!(backend.calls[0].starts_with("gcx:"));
        assert!(backend.calls[1].starts_with("gkx:"));
        assert!(backend.calls[2].starts_with("print:"));
        assert!(backend.calls[3].starts_with("div:"));
        assert!(backend.calls[4].starts_with("fx:"));
    }

    #[test]
    fn missing_backend_reports_explicit_prerequisite_error() {
        let command = parse_extract_command("fx", Vec::<String>::new()).unwrap();

        assert_eq!(
            execute_with_missing_dependencies(&command),
            Err(ExtractCommandError::MissingNativePorts {
                operation: ExtractOperation::FastExtract,
            })
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present() {
        let source = include_str!("com_ex.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday", "1-", "8j8")));
    }
}
