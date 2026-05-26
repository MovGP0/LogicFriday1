//! Native Rust model for `LogicSynthesis/sis/resub/bresub.c`.
//!
//! SIS never implemented a separate boolean resubstitution engine in this file.
//! Both boolean entry points print a warning and delegate to algebraic
//! resubstitution with complement use forced on.

use std::error::Error;
use std::fmt;

pub const BOOLEAN_RESUB_WARNING: &str =
    "Warning!: Boolean resub has not been implemented, algebraic resub is used.\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanResubOperation {
    BooleanNode,
    BooleanNetwork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BooleanResubPlan {
    pub operation: BooleanResubOperation,
    pub algebraic_use_complement: bool,
    pub warning: &'static str,
}

impl BooleanResubPlan {
    pub const fn for_node() -> Self {
        Self {
            operation: BooleanResubOperation::BooleanNode,
            algebraic_use_complement: true,
            warning: BOOLEAN_RESUB_WARNING,
        }
    }

    pub const fn for_network() -> Self {
        Self {
            operation: BooleanResubOperation::BooleanNetwork,
            algebraic_use_complement: true,
            warning: BOOLEAN_RESUB_WARNING,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BooleanResubError {
    MissingNativePorts { operation: BooleanResubOperation },
    AlgebraicFallback(String),
}

impl fmt::Display for BooleanResubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation:?} requires unavailable native Rust SIS support"
            ),
            Self::AlgebraicFallback(message) => f.write_str(message),
        }
    }
}

impl Error for BooleanResubError {}

pub trait BooleanResubWarningSink {
    fn warn(&mut self, message: &str);
}

impl BooleanResubWarningSink for String {
    fn warn(&mut self, message: &str) {
        self.push_str(message);
    }
}

impl BooleanResubWarningSink for Vec<String> {
    fn warn(&mut self, message: &str) {
        self.push(message.to_owned());
    }
}

pub trait AlgebraicResubBackend {
    type Node;
    type Network;

    fn resub_algebraic_node(
        &mut self,
        node: Self::Node,
        use_complement: bool,
    ) -> Result<bool, BooleanResubError>;

    fn resub_algebraic_network(
        &mut self,
        network: Self::Network,
        use_complement: bool,
    ) -> Result<(), BooleanResubError>;
}

pub fn resub_bool_node<B, W>(
    backend: &mut B,
    warnings: &mut W,
    node: B::Node,
) -> Result<bool, BooleanResubError>
where
    B: AlgebraicResubBackend,
    W: BooleanResubWarningSink,
{
    let plan = BooleanResubPlan::for_node();
    warnings.warn(plan.warning);
    backend.resub_algebraic_node(node, plan.algebraic_use_complement)
}

pub fn resub_bool_network<B, W>(
    backend: &mut B,
    warnings: &mut W,
    network: B::Network,
) -> Result<(), BooleanResubError>
where
    B: AlgebraicResubBackend,
    W: BooleanResubWarningSink,
{
    let plan = BooleanResubPlan::for_network();
    warnings.warn(plan.warning);
    backend.resub_algebraic_network(network, plan.algebraic_use_complement)
}

#[derive(Default)]
pub struct MissingAlgebraicResubBackend;

impl AlgebraicResubBackend for MissingAlgebraicResubBackend {
    type Node = String;
    type Network = String;

    fn resub_algebraic_node(
        &mut self,
        _node: Self::Node,
        _use_complement: bool,
    ) -> Result<bool, BooleanResubError> {
        Err(missing(BooleanResubOperation::BooleanNode))
    }

    fn resub_algebraic_network(
        &mut self,
        _network: Self::Network,
        _use_complement: bool,
    ) -> Result<(), BooleanResubError> {
        Err(missing(BooleanResubOperation::BooleanNetwork))
    }
}

pub fn execute_node_with_missing_dependencies(
    node: impl Into<String>,
) -> Result<bool, BooleanResubError> {
    let mut warnings = String::new();
    resub_bool_node(
        &mut MissingAlgebraicResubBackend,
        &mut warnings,
        node.into(),
    )
}

pub fn execute_network_with_missing_dependencies(
    network: impl Into<String>,
) -> Result<(), BooleanResubError> {
    let mut warnings = String::new();
    resub_bool_network(
        &mut MissingAlgebraicResubBackend,
        &mut warnings,
        network.into(),
    )
}

fn missing(operation: BooleanResubOperation) -> BooleanResubError {
    BooleanResubError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        calls: Vec<String>,
    }

    impl AlgebraicResubBackend for RecordingBackend {
        type Node = String;
        type Network = String;

        fn resub_algebraic_node(
            &mut self,
            node: Self::Node,
            use_complement: bool,
        ) -> Result<bool, BooleanResubError> {
            self.calls
                .push(format!("algebraic_node:{node}:{use_complement}"));
            Ok(true)
        }

        fn resub_algebraic_network(
            &mut self,
            network: Self::Network,
            use_complement: bool,
        ) -> Result<(), BooleanResubError> {
            self.calls
                .push(format!("algebraic_network:{network}:{use_complement}"));
            Ok(())
        }
    }

    #[test]
    fn boolean_plans_match_the_c_fallback_contract() {
        assert_eq!(
            BooleanResubPlan::for_node(),
            BooleanResubPlan {
                operation: BooleanResubOperation::BooleanNode,
                algebraic_use_complement: true,
                warning: BOOLEAN_RESUB_WARNING,
            }
        );
        assert_eq!(
            BooleanResubPlan::for_network().algebraic_use_complement,
            true
        );
    }

    #[test]
    fn boolean_node_warns_and_delegates_to_algebraic_node_with_complement() {
        let mut backend = RecordingBackend::default();
        let mut warnings = String::new();

        let changed = resub_bool_node(&mut backend, &mut warnings, "n1".to_owned()).unwrap();

        assert!(changed);
        assert_eq!(warnings, BOOLEAN_RESUB_WARNING);
        assert_eq!(backend.calls, vec!["algebraic_node:n1:true"]);
    }

    #[test]
    fn boolean_network_warns_and_delegates_to_algebraic_network_with_complement() {
        let mut backend = RecordingBackend::default();
        let mut warnings = Vec::new();

        resub_bool_network(&mut backend, &mut warnings, "net".to_owned()).unwrap();

        assert_eq!(warnings, vec![BOOLEAN_RESUB_WARNING.to_owned()]);
        assert_eq!(backend.calls, vec!["algebraic_network:net:true"]);
    }

    #[test]
    fn missing_node_fallback_reports_failed_operation() {
        let Err(BooleanResubError::MissingNativePorts { operation }) =
            execute_node_with_missing_dependencies("n1")
        else {
            panic!("expected missing native port error");
        };

        assert_eq!(operation, BooleanResubOperation::BooleanNode);
    }

    #[test]
    fn missing_network_fallback_reports_failed_operation() {
        let Err(BooleanResubError::MissingNativePorts { operation }) =
            execute_network_with_missing_dependencies("net")
        else {
            panic!("expected missing native port error");
        };

        assert_eq!(operation, BooleanResubOperation::BooleanNetwork);
    }

    #[test]
    fn missing_dependency_display_is_generic() {
        let error = execute_network_with_missing_dependencies("net").unwrap_err();

        assert_eq!(
            error.to_string(),
            "BooleanNetwork requires unavailable native Rust SIS support"
        );
    }
}
