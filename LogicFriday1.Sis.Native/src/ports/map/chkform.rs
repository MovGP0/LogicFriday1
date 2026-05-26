//! Native Rust form checker for `sis/map/chkform.c`.
//!
//! The legacy routine accepted a SIS `network_t` and verified that every
//! internal non-constant node was already expressed as a 1- or 2-input
//! NAND/NOR primitive in factored form. This native port keeps that behavior
//! over owned Rust data and reports structured diagnostics instead of appending
//! to SIS' process-global error buffer. It intentionally exposes no legacy C
//! ABI entry points.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapFormMode {
    Nand,
    Nor,
}

impl MapFormMode {
    pub fn from_nand_flag(nand_flag: bool) -> Self {
        if nand_flag { Self::Nand } else { Self::Nor }
    }

    pub fn gate_name(self) -> &'static str {
        match self {
            Self::Nand => "nand",
            Self::Nor => "nor",
        }
    }

    fn accepts_function(self, function: MapFormFunction) -> bool {
        match self {
            Self::Nand => matches!(function, MapFormFunction::Inverter | MapFormFunction::Or),
            Self::Nor => matches!(function, MapFormFunction::Inverter | MapFormFunction::And),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapFormNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapFormFunction {
    Zero,
    One,
    Inverter,
    And,
    Or,
    Other,
}

impl MapFormFunction {
    fn is_constant(self) -> bool {
        matches!(self, Self::Zero | Self::One)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MapFormLiteralCount {
    pub positive: usize,
    pub negative: usize,
}

impl MapFormLiteralCount {
    pub fn new(positive: usize, negative: usize) -> Self {
        Self { positive, negative }
    }

    pub fn negative_literal() -> Self {
        Self::new(0, 1)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapFormFanin {
    pub name: String,
    pub literals: MapFormLiteralCount,
}

impl MapFormFanin {
    pub fn new(name: impl Into<String>, literals: MapFormLiteralCount) -> Self {
        Self {
            name: name.into(),
            literals,
        }
    }

    pub fn negative(name: impl Into<String>) -> Self {
        Self::new(name, MapFormLiteralCount::negative_literal())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapFormNode {
    pub name: String,
    pub kind: MapFormNodeKind,
    pub function: MapFormFunction,
    pub fanins: Vec<MapFormFanin>,
}

impl MapFormNode {
    pub fn new(
        name: impl Into<String>,
        kind: MapFormNodeKind,
        function: MapFormFunction,
        fanins: Vec<MapFormFanin>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            function,
            fanins,
        }
    }

    pub fn internal(
        name: impl Into<String>,
        function: MapFormFunction,
        fanins: Vec<MapFormFanin>,
    ) -> Self {
        Self::new(name, MapFormNodeKind::Internal, function, fanins)
    }

    pub fn primary_input(name: impl Into<String>) -> Self {
        Self::new(
            name,
            MapFormNodeKind::PrimaryInput,
            MapFormFunction::Other,
            Vec::new(),
        )
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MapFormNetwork {
    pub name: Option<String>,
    pub nodes: Vec<MapFormNode>,
}

impl MapFormNetwork {
    pub fn new(name: impl Into<Option<String>>, nodes: Vec<MapFormNode>) -> Self {
        Self {
            name: name.into(),
            nodes,
        }
    }

    pub fn unnamed(nodes: Vec<MapFormNode>) -> Self {
        Self::new(None, nodes)
    }

    pub fn network_name(&self) -> &str {
        self.name.as_deref().unwrap_or("<unnamed>")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapFormError {
    InvalidGate {
        network: String,
        node: String,
        mode: MapFormMode,
        reason: MapFormRejectReason,
    },
}

impl fmt::Display for MapFormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGate {
                network,
                node,
                mode,
                reason,
            } => write!(
                f,
                "\"{network}\": '{node}' is not a 1 or 2-input {} gate ({reason})",
                mode.gate_name()
            ),
        }
    }
}

impl Error for MapFormError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapFormRejectReason {
    TooManyFanins { fanins: usize },
    UnsupportedFunction { function: MapFormFunction },
    PositivePhaseLiteral { fanin: String, count: usize },
    NegativePhaseLiteralCount { fanin: String, count: usize },
}

impl fmt::Display for MapFormRejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyFanins { fanins } => write!(f, "{fanins} fanins"),
            Self::UnsupportedFunction { function } => {
                write!(f, "unsupported function {function:?}")
            }
            Self::PositivePhaseLiteral { fanin, count } => {
                write!(f, "fanin '{fanin}' has {count} positive literals")
            }
            Self::NegativePhaseLiteralCount { fanin, count } => {
                write!(f, "fanin '{fanin}' has {count} negative literals")
            }
        }
    }
}

pub fn check_map_form(network: &MapFormNetwork, mode: MapFormMode) -> Result<(), MapFormError> {
    for node in &network.nodes {
        if node.kind != MapFormNodeKind::Internal {
            continue;
        }

        check_internal_node(network, node, mode)?;
    }

    Ok(())
}

pub fn map_check_form(network: &MapFormNetwork, nand_flag: bool) -> Result<(), MapFormError> {
    check_map_form(network, MapFormMode::from_nand_flag(nand_flag))
}

fn check_internal_node(
    network: &MapFormNetwork,
    node: &MapFormNode,
    mode: MapFormMode,
) -> Result<(), MapFormError> {
    if node.fanins.len() > 2 {
        return Err(invalid_gate(
            network,
            node,
            mode,
            MapFormRejectReason::TooManyFanins {
                fanins: node.fanins.len(),
            },
        ));
    }

    if node.function.is_constant() {
        return Ok(());
    }

    if !mode.accepts_function(node.function) {
        return Err(invalid_gate(
            network,
            node,
            mode,
            MapFormRejectReason::UnsupportedFunction {
                function: node.function,
            },
        ));
    }

    for fanin in &node.fanins {
        if fanin.literals.positive != 0 {
            return Err(invalid_gate(
                network,
                node,
                mode,
                MapFormRejectReason::PositivePhaseLiteral {
                    fanin: fanin.name.clone(),
                    count: fanin.literals.positive,
                },
            ));
        }

        if fanin.literals.negative != 1 {
            return Err(invalid_gate(
                network,
                node,
                mode,
                MapFormRejectReason::NegativePhaseLiteralCount {
                    fanin: fanin.name.clone(),
                    count: fanin.literals.negative,
                },
            ));
        }
    }

    Ok(())
}

fn invalid_gate(
    network: &MapFormNetwork,
    node: &MapFormNode,
    mode: MapFormMode,
    reason: MapFormRejectReason,
) -> MapFormError {
    MapFormError::InvalidGate {
        network: network.network_name().to_owned(),
        node: node.name.clone(),
        mode,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network(nodes: Vec<MapFormNode>) -> MapFormNetwork {
        MapFormNetwork::new(Some("demo".to_string()), nodes)
    }

    #[test]
    fn accepts_nand_form_or_of_negative_literals_and_inverters() {
        let network = network(vec![
            MapFormNode::primary_input("a"),
            MapFormNode::primary_input("b"),
            MapFormNode::internal(
                "na",
                MapFormFunction::Inverter,
                vec![MapFormFanin::negative("a")],
            ),
            MapFormNode::internal(
                "f",
                MapFormFunction::Or,
                vec![MapFormFanin::negative("a"), MapFormFanin::negative("b")],
            ),
        ]);

        assert_eq!(check_map_form(&network, MapFormMode::Nand), Ok(()));
    }

    #[test]
    fn accepts_nor_form_and_of_negative_literals_and_constants() {
        let network = network(vec![
            MapFormNode::internal("one", MapFormFunction::One, Vec::new()),
            MapFormNode::internal(
                "f",
                MapFormFunction::And,
                vec![MapFormFanin::negative("a"), MapFormFanin::negative("b")],
            ),
        ]);

        assert_eq!(map_check_form(&network, false), Ok(()));
    }

    #[test]
    fn rejects_too_many_fanins_before_function_checks() {
        let network = network(vec![MapFormNode::internal(
            "f",
            MapFormFunction::Or,
            vec![
                MapFormFanin::negative("a"),
                MapFormFanin::negative("b"),
                MapFormFanin::negative("c"),
            ],
        )]);

        assert_eq!(
            check_map_form(&network, MapFormMode::Nand).unwrap_err(),
            MapFormError::InvalidGate {
                network: "demo".to_string(),
                node: "f".to_string(),
                mode: MapFormMode::Nand,
                reason: MapFormRejectReason::TooManyFanins { fanins: 3 },
            }
        );
    }

    #[test]
    fn rejects_wrong_function_for_selected_mode() {
        let network = network(vec![MapFormNode::internal(
            "f",
            MapFormFunction::And,
            vec![MapFormFanin::negative("a"), MapFormFanin::negative("b")],
        )]);

        assert_eq!(
            check_map_form(&network, MapFormMode::Nand).unwrap_err(),
            MapFormError::InvalidGate {
                network: "demo".to_string(),
                node: "f".to_string(),
                mode: MapFormMode::Nand,
                reason: MapFormRejectReason::UnsupportedFunction {
                    function: MapFormFunction::And,
                },
            }
        );
    }

    #[test]
    fn rejects_positive_or_missing_negative_fanin_literals() {
        let positive = network(vec![MapFormNode::internal(
            "f",
            MapFormFunction::Or,
            vec![MapFormFanin::new("a", MapFormLiteralCount::new(1, 1))],
        )]);
        let missing_negative = network(vec![MapFormNode::internal(
            "g",
            MapFormFunction::Or,
            vec![MapFormFanin::new("a", MapFormLiteralCount::new(0, 0))],
        )]);

        assert!(matches!(
            check_map_form(&positive, MapFormMode::Nand),
            Err(MapFormError::InvalidGate {
                reason: MapFormRejectReason::PositivePhaseLiteral { .. },
                ..
            })
        ));
        assert!(matches!(
            check_map_form(&missing_negative, MapFormMode::Nand),
            Err(MapFormError::InvalidGate {
                reason: MapFormRejectReason::NegativePhaseLiteralCount { .. },
                ..
            })
        ));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("chkform.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
