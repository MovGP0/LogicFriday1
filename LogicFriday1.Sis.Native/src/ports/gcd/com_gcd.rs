//! Native Rust command model for `LogicSynthesis/sis/gcd/com_gcd.c`.
//!
//! The legacy file registers two diagnostic commands around the GCD package.
//! This port keeps command registration, validation, and output formatting in
//! Rust while the actual algebraic GCD/factorization implementation is supplied
//! by a native backend.

use std::error::Error;
use std::fmt;

pub const GCD_USAGE: &str = "_gcd n1 n2 ...\n   where none of n1, n2,..., nn are primary inputs\n";
pub const PRIME_FACTOR_USAGE: &str = "_prime_factor n1 n2 ...\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GcdCommandKind {
    Gcd,
    PrimeFactor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandRegistration {
    pub name: &'static str,
    pub kind: GcdCommandKind,
    pub changes_network: bool,
}

pub const GCD_COMMANDS: &[CommandRegistration] = &[
    CommandRegistration {
        name: "_gcd",
        kind: GcdCommandKind::Gcd,
        changes_network: false,
    },
    CommandRegistration {
        name: "_prime_factor",
        kind: GcdCommandKind::PrimeFactor,
        changes_network: false,
    },
];

pub fn gcd_command_registrations() -> &'static [CommandRegistration] {
    GCD_COMMANDS
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GcdNodeFunction {
    PrimaryInput,
    One,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GcdNodeSelection {
    pub name: String,
    pub function: GcdNodeFunction,
}

impl GcdNodeSelection {
    pub fn new(name: impl Into<String>, function: GcdNodeFunction) -> Self {
        Self {
            name: name.into(),
            function,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GcdExpression {
    rhs: String,
}

impl GcdExpression {
    pub fn new(rhs: impl Into<String>) -> Self {
        Self { rhs: rhs.into() }
    }

    pub fn rhs(&self) -> &str {
        &self.rhs
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GcdCommandOutput {
    pub stdout: String,
}

impl GcdCommandOutput {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn stdout(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
        }
    }
}

pub trait GcdBackend {
    type Error;

    fn gcd_nodevec(&mut self, nodes: &[GcdNodeSelection]) -> Result<GcdExpression, Self::Error>;

    fn prime_factorize(
        &mut self,
        node: &GcdNodeSelection,
    ) -> Result<Vec<GcdExpression>, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GcdCommandError<E> {
    Usage(&'static str),
    UnknownCommand(String),
    MissingNativeBackend { command: GcdCommandKind },
    Backend(E),
}

impl<E> fmt::Display for GcdCommandError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(usage) => write!(f, "{usage}"),
            Self::UnknownCommand(command) => write!(f, "unknown gcd command {command}"),
            Self::MissingNativeBackend { command } => {
                write!(f, "{command:?} requires a native gcd backend")
            }
            Self::Backend(error) => write!(f, "{error}"),
        }
    }
}

impl<E> Error for GcdCommandError<E> where E: Error + 'static {}

pub fn parse_gcd_command(command_name: &str) -> Result<GcdCommandKind, GcdCommandError<String>> {
    match command_name {
        "_gcd" => Ok(GcdCommandKind::Gcd),
        "_prime_factor" => Ok(GcdCommandKind::PrimeFactor),
        _ => Err(GcdCommandError::UnknownCommand(command_name.to_owned())),
    }
}

pub fn execute_gcd<B>(
    backend: &mut B,
    nodes: &[GcdNodeSelection],
) -> Result<GcdCommandOutput, GcdCommandError<B::Error>>
where
    B: GcdBackend,
{
    validate_gcd_selection(nodes)?;

    if nodes
        .iter()
        .any(|node| node.function == GcdNodeFunction::One)
    {
        return Ok(GcdCommandOutput::stdout("-1-\n"));
    }

    let gcd = backend
        .gcd_nodevec(nodes)
        .map_err(GcdCommandError::Backend)?;
    Ok(GcdCommandOutput::stdout(format!("{}\n", gcd.rhs())))
}

pub fn execute_prime_factor<B>(
    backend: &mut B,
    nodes: &[GcdNodeSelection],
) -> Result<GcdCommandOutput, GcdCommandError<B::Error>>
where
    B: GcdBackend,
{
    if nodes.is_empty() {
        return Err(GcdCommandError::Usage(PRIME_FACTOR_USAGE));
    }

    let mut stdout = String::new();
    for node in nodes {
        let factors = backend
            .prime_factorize(node)
            .map_err(GcdCommandError::Backend)?;
        stdout.push_str(&format!("Prime factorization for node {}\n", node.name));
        for factor in factors {
            stdout.push_str(factor.rhs());
            stdout.push('\n');
        }
    }

    Ok(GcdCommandOutput::stdout(stdout))
}

pub fn execute_command<B>(
    backend: &mut B,
    command: GcdCommandKind,
    nodes: &[GcdNodeSelection],
) -> Result<GcdCommandOutput, GcdCommandError<B::Error>>
where
    B: GcdBackend,
{
    match command {
        GcdCommandKind::Gcd => execute_gcd(backend, nodes),
        GcdCommandKind::PrimeFactor => execute_prime_factor(backend, nodes),
    }
}

pub fn execute_command_without_backend<E>(
    command: GcdCommandKind,
) -> Result<GcdCommandOutput, GcdCommandError<E>> {
    Err(GcdCommandError::MissingNativeBackend { command })
}

fn validate_gcd_selection<E>(nodes: &[GcdNodeSelection]) -> Result<(), GcdCommandError<E>> {
    if nodes.is_empty()
        || nodes
            .iter()
            .any(|node| node.function == GcdNodeFunction::PrimaryInput)
    {
        return Err(GcdCommandError::Usage(GCD_USAGE));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        gcd_calls: Vec<Vec<String>>,
        factor_calls: Vec<String>,
    }

    impl GcdBackend for RecordingBackend {
        type Error = String;

        fn gcd_nodevec(
            &mut self,
            nodes: &[GcdNodeSelection],
        ) -> Result<GcdExpression, Self::Error> {
            self.gcd_calls
                .push(nodes.iter().map(|node| node.name.clone()).collect());
            Ok(GcdExpression::new("a b + c"))
        }

        fn prime_factorize(
            &mut self,
            node: &GcdNodeSelection,
        ) -> Result<Vec<GcdExpression>, Self::Error> {
            self.factor_calls.push(node.name.clone());
            Ok(vec![
                GcdExpression::new(format!("{}_p0", node.name)),
                GcdExpression::new(format!("{}_p1", node.name)),
            ])
        }
    }

    fn node(name: &str) -> GcdNodeSelection {
        GcdNodeSelection::new(name, GcdNodeFunction::Other)
    }

    #[test]
    fn command_registration_matches_init_gcd() {
        assert_eq!(
            gcd_command_registrations(),
            &[
                CommandRegistration {
                    name: "_gcd",
                    kind: GcdCommandKind::Gcd,
                    changes_network: false,
                },
                CommandRegistration {
                    name: "_prime_factor",
                    kind: GcdCommandKind::PrimeFactor,
                    changes_network: false,
                },
            ]
        );
    }

    #[test]
    fn parses_known_command_names() {
        assert_eq!(parse_gcd_command("_gcd"), Ok(GcdCommandKind::Gcd));
        assert_eq!(
            parse_gcd_command("_prime_factor"),
            Ok(GcdCommandKind::PrimeFactor)
        );
        assert_eq!(
            parse_gcd_command("missing"),
            Err(GcdCommandError::UnknownCommand("missing".to_owned()))
        );
    }

    #[test]
    fn gcd_rejects_empty_selection_and_primary_inputs() {
        let mut backend = RecordingBackend::default();
        assert_eq!(
            execute_gcd(&mut backend, &[]),
            Err(GcdCommandError::Usage(GCD_USAGE))
        );
        assert_eq!(
            execute_gcd(
                &mut backend,
                &[GcdNodeSelection::new("a", GcdNodeFunction::PrimaryInput)]
            ),
            Err(GcdCommandError::Usage(GCD_USAGE))
        );
        assert!(backend.gcd_calls.is_empty());
    }

    #[test]
    fn gcd_constant_one_short_circuits_like_c_command() {
        let mut backend = RecordingBackend::default();
        let output = execute_gcd(
            &mut backend,
            &[
                node("f"),
                GcdNodeSelection::new("one", GcdNodeFunction::One),
                node("g"),
            ],
        )
        .unwrap();

        assert_eq!(output.stdout, "-1-\n");
        assert!(backend.gcd_calls.is_empty());
    }

    #[test]
    fn gcd_delegates_selected_nodes_and_prints_rhs() {
        let mut backend = RecordingBackend::default();
        let output = execute_gcd(&mut backend, &[node("f"), node("g")]).unwrap();

        assert_eq!(output.stdout, "a b + c\n");
        assert_eq!(
            backend.gcd_calls,
            vec![vec!["f".to_owned(), "g".to_owned()]]
        );
    }

    #[test]
    fn prime_factor_rejects_empty_selection() {
        let mut backend = RecordingBackend::default();

        assert_eq!(
            execute_prime_factor(&mut backend, &[]),
            Err(GcdCommandError::Usage(PRIME_FACTOR_USAGE))
        );
        assert!(backend.factor_calls.is_empty());
    }

    #[test]
    fn prime_factor_delegates_each_node_and_formats_legacy_headers() {
        let mut backend = RecordingBackend::default();
        let output = execute_prime_factor(&mut backend, &[node("f"), node("g")]).unwrap();

        assert_eq!(
            output.stdout,
            concat!(
                "Prime factorization for node f\n",
                "f_p0\n",
                "f_p1\n",
                "Prime factorization for node g\n",
                "g_p0\n",
                "g_p1\n"
            )
        );
        assert_eq!(backend.factor_calls, vec!["f".to_owned(), "g".to_owned()]);
    }

    #[test]
    fn command_dispatch_uses_requested_command_kind() {
        let mut backend = RecordingBackend::default();

        assert_eq!(
            execute_command(&mut backend, GcdCommandKind::PrimeFactor, &[node("f")])
                .unwrap()
                .stdout,
            "Prime factorization for node f\nf_p0\nf_p1\n"
        );
        assert_eq!(backend.factor_calls, vec!["f".to_owned()]);
    }

    #[test]
    fn reports_missing_backend_for_later_integration_layers() {
        let error = execute_command_without_backend::<String>(GcdCommandKind::Gcd).unwrap_err();

        assert_eq!(
            error,
            GcdCommandError::MissingNativeBackend {
                command: GcdCommandKind::Gcd
            }
        );
        assert!(error.to_string().contains("native gcd backend"));
    }

    #[test]
    fn no_dependency_metadata_or_legacy_abi_tokens_are_present() {
        let source = include_str!("com_gcd.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
