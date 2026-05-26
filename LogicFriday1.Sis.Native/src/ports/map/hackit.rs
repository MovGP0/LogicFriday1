//! Native Rust helpers for `sis/map/hackit.c`.
//!
//! The original file chose the largest inverter cell in the current genlib and
//! inserted inverter pairs at primary inputs so both phases were available to
//! downstream mapping. This port keeps that behavior over `VirtualMappedNetwork`
//! and `GenlibLibrary` without legacy C ABI exports.

use std::error::Error;
use std::fmt;

use super::library::{GenlibGate, GenlibLibrary};
use super::virtual_net::{
    GateKind, GateLink, NodeId, NodeKind, SourceRef, VirtualMappedNetwork, VirtualNetworkError,
};

#[derive(Clone, Debug, PartialEq)]
pub struct BufferInputsReport {
    pub inverter_gate: String,
    pub processed_primary_inputs: usize,
    pub created_inverters: usize,
    pub reused_existing_inverters: usize,
    pub patched_positive_fanouts: usize,
    pub patched_negative_fanouts: usize,
}

impl BufferInputsReport {
    fn new(inverter_gate: &GenlibGate, processed_primary_inputs: usize) -> Self {
        Self {
            inverter_gate: inverter_gate.name.clone(),
            processed_primary_inputs,
            created_inverters: 0,
            reused_existing_inverters: 0,
            patched_positive_fanouts: 0,
            patched_negative_fanouts: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HackitError {
    MissingInverterGate,
    InvalidInverterGate { gate: String },
    MissingNode { node: NodeId },
    VirtualNetwork(VirtualNetworkError),
}

impl fmt::Display for HackitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInverterGate => {
                write!(
                    f,
                    "buffer input insertion requires an inverter gate in the library"
                )
            }
            Self::InvalidInverterGate { gate } => {
                write!(f, "library gate '{gate}' is not a single-input inverter")
            }
            Self::MissingNode { node } => {
                write!(f, "virtual network node {} was not found", node.index())
            }
            Self::VirtualNetwork(error) => write!(f, "{error}"),
        }
    }
}

impl Error for HackitError {}

impl From<VirtualNetworkError> for HackitError {
    fn from(value: VirtualNetworkError) -> Self {
        Self::VirtualNetwork(value)
    }
}

pub fn choose_largest_inverter_gate(library: &GenlibLibrary) -> Option<&GenlibGate> {
    library
        .gates
        .iter()
        .filter(|gate| is_inverter_gate(gate))
        .max_by(|left, right| {
            left.area
                .total_cmp(&right.area)
                .then_with(|| right.name.cmp(&left.name))
        })
}

pub fn buffer_inputs(
    network: &mut VirtualMappedNetwork,
    library: &GenlibLibrary,
) -> Result<BufferInputsReport, HackitError> {
    let inverter_gate =
        choose_largest_inverter_gate(library).ok_or(HackitError::MissingInverterGate)?;
    buffer_inputs_with_gate(network, library, inverter_gate)
}

pub fn buffer_inputs_with_gate(
    network: &mut VirtualMappedNetwork,
    library: &GenlibLibrary,
    inverter_gate: &GenlibGate,
) -> Result<BufferInputsReport, HackitError> {
    if !is_inverter_gate(inverter_gate) {
        return Err(HackitError::InvalidInverterGate {
            gate: inverter_gate.name.clone(),
        });
    }

    network.setup_gate_links()?;
    let primary_inputs = network.inputs().to_vec();
    let mut report = BufferInputsReport::new(inverter_gate, primary_inputs.len());
    let mut name_counter = 0usize;

    for primary_input in primary_inputs {
        add_input_inverters(
            network,
            library,
            primary_input,
            inverter_gate,
            &mut name_counter,
            &mut report,
        )?;
    }

    network.setup_gate_links()?;
    Ok(report)
}

fn add_input_inverters(
    network: &mut VirtualMappedNetwork,
    library: &GenlibLibrary,
    node: NodeId,
    inverter_gate: &GenlibGate,
    name_counter: &mut usize,
    report: &mut BufferInputsReport,
) -> Result<(), HackitError> {
    let existing_inverter = first_inverter_fanout(network, library, node)?;
    let inverter = if let Some(inverter) = existing_inverter {
        report.reused_existing_inverters += 1;
        inverter
    } else {
        report.created_inverters += 1;
        create_inverter(network, node, inverter_gate, name_counter)?
    };

    report.created_inverters += 1;
    let positive_driver = create_inverter(network, inverter, inverter_gate, name_counter)?;
    report.patched_positive_fanouts +=
        patch_non_inverter_fanouts(network, library, node, positive_driver)?;

    if existing_inverter.is_some() {
        report.created_inverters += 1;
        let negative_driver =
            create_inverter(network, positive_driver, inverter_gate, name_counter)?;
        report.patched_negative_fanouts +=
            patch_non_inverter_fanouts(network, library, inverter, negative_driver)?;
    }

    Ok(())
}

fn create_inverter(
    network: &mut VirtualMappedNetwork,
    fanin: NodeId,
    inverter_gate: &GenlibGate,
    name_counter: &mut usize,
) -> Result<NodeId, HackitError> {
    let fanin_name = network
        .node(fanin)
        .ok_or(HackitError::MissingNode { node: fanin })?
        .name
        .clone();
    let name = loop {
        let candidate = format!("{fanin_name}_hackit_inv_{name_counter}");
        *name_counter += 1;
        if network.nodes().iter().all(|node| node.name != candidate) {
            break candidate;
        }
    };

    let inverter = network.add_gate(
        name,
        GateKind::Library(inverter_gate.name.clone()),
        vec![SourceRef::Node(fanin)],
    );
    network.add_to_gate_link(SourceRef::Node(fanin), GateLink::new(inverter, 0))?;

    Ok(inverter)
}

fn patch_non_inverter_fanouts(
    network: &mut VirtualMappedNetwork,
    library: &GenlibLibrary,
    source: NodeId,
    replacement: NodeId,
) -> Result<usize, HackitError> {
    let links = network
        .node(source)
        .ok_or(HackitError::MissingNode { node: source })?
        .gate_links()
        .copied()
        .collect::<Vec<_>>();
    let mut patched = 0usize;

    for link in links {
        if link.node == replacement || is_inverter_node(network, library, link.node)? {
            continue;
        }

        network.remove_gate_link(source, link.node, link.pin);
        network.add_to_gate_link(SourceRef::Node(replacement), link)?;
        patched += 1;
    }

    Ok(patched)
}

fn first_inverter_fanout(
    network: &VirtualMappedNetwork,
    library: &GenlibLibrary,
    source: NodeId,
) -> Result<Option<NodeId>, HackitError> {
    let source_node = network
        .node(source)
        .ok_or(HackitError::MissingNode { node: source })?;

    for link in source_node.gate_links() {
        if is_inverter_node(network, library, link.node)? {
            return Ok(Some(link.node));
        }
    }

    Ok(None)
}

fn is_inverter_node(
    network: &VirtualMappedNetwork,
    library: &GenlibLibrary,
    node: NodeId,
) -> Result<bool, HackitError> {
    let node = network
        .node(node)
        .ok_or(HackitError::MissingNode { node })?;
    if node.kind != NodeKind::Internal {
        return Ok(false);
    }

    Ok(match &node.gate {
        Some(GateKind::Inverter) => true,
        Some(GateKind::Library(name)) => library.gate(name).is_some_and(is_inverter_gate),
        _ => false,
    })
}

fn is_inverter_gate(gate: &GenlibGate) -> bool {
    gate.pins.len() == 1 && is_single_input_inverter_expression(&gate.output.expression)
}

fn is_single_input_inverter_expression(expression: &str) -> bool {
    let mut expression = normalize_expression(expression);
    if !expression.starts_with('!') {
        return false;
    }

    expression.remove(0);
    let variable = expression
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(&expression);

    !variable.is_empty()
        && variable
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

fn normalize_expression(expression: &str) -> String {
    expression
        .trim()
        .trim_end_matches(';')
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::library::parse_genlib;

    fn sample_library() -> GenlibLibrary {
        parse_genlib(concat!(
            "GATE inv_small 1 O=!a;\n",
            "PIN a INV 1 10 1 .2 1 .2\n",
            "GATE inv_large 5 O=!(a);\n",
            "PIN a INV 2 10 1 .2 1 .2\n",
            "GATE and2 3 O=a*b;\n",
            "PIN a NONINV 1 10 1 .2 1 .2\n",
            "PIN b NONINV 1 10 1 .2 1 .2\n",
        ))
        .unwrap()
    }

    #[test]
    fn chooses_largest_single_input_inverter() {
        let library = sample_library();

        assert_eq!(
            choose_largest_inverter_gate(&library).map(|gate| gate.name.as_str()),
            Some("inv_large")
        );
    }

    #[test]
    fn buffers_primary_input_fanouts_with_library_inverters() {
        let library = sample_library();
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let n1 = network.add_gate(
            "n1",
            GateKind::And,
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );
        network
            .add_primary_output("f", SourceRef::Node(n1))
            .unwrap();

        let report = buffer_inputs(&mut network, &library).unwrap();

        assert_eq!(report.processed_primary_inputs, 2);
        assert_eq!(report.created_inverters, 4);
        assert_ne!(
            network.node(n1).unwrap().save_binding[0],
            SourceRef::Node(a)
        );
        assert_ne!(
            network.node(n1).unwrap().save_binding[1],
            SourceRef::Node(b)
        );
        assert_eq!(
            network
                .nodes()
                .iter()
                .filter(|node| node.gate == Some(GateKind::Library("inv_large".to_string())))
                .count(),
            4
        );
    }

    #[test]
    fn reuses_existing_input_inverter_and_rebuffers_its_positive_fanouts() {
        let library = sample_library();
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let inv = network.add_gate(
            "existing_inv",
            GateKind::Library("inv_small".to_string()),
            vec![SourceRef::Node(a)],
        );
        let consumer = network.add_gate("consumer", GateKind::Or, vec![SourceRef::Node(inv)]);
        network
            .add_primary_output("f", SourceRef::Node(consumer))
            .unwrap();

        let report = buffer_inputs(&mut network, &library).unwrap();

        assert_eq!(report.reused_existing_inverters, 1);
        assert_eq!(report.created_inverters, 2);
        assert_ne!(
            network.node(consumer).unwrap().save_binding,
            vec![SourceRef::Node(inv)]
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("hackit.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
