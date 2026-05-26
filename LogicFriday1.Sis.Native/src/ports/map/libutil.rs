//! Native Rust library utilities for `sis/map/libutil.c`.
//!
//! The C file is mostly an accessor layer around global `library_t` state,
//! `lib_gate_t` objects, mapped-node annotations, and full `network_t`
//! mutation. This port keeps the useful mapper-facing behavior as explicit
//! owned-data helpers over `GenlibLibrary` and `VirtualMappedNetwork`: gate
//! lookup, deterministic class grouping, pin/load/delay access, and mapped gate
//! inspection. Operations that require the legacy SIS `network_t` graph return
//! dependency errors instead of preserving hidden global state or per-file C ABI
//! entry points.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;

use super::library::{GenlibGate, GenlibLibrary, GenlibPin, PinPhase};
use super::virtual_net::{GateKind, NodeId, NodeKind, VirtualMappedNetwork};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinDirection {
    Output,
    Input,
    Control,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transition {
    Rise,
    Fall,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinDelay {
    pub phase: PinPhase,
    pub input_load: f64,
    pub max_load: f64,
    pub rise_block_delay: f64,
    pub rise_fanout_delay: f64,
    pub fall_block_delay: f64,
    pub fall_fanout_delay: f64,
}

impl PinDelay {
    pub fn delay_at_load(self, transition: Transition, load: f64) -> Result<f64, LibUtilError> {
        if !load.is_finite() || load < 0.0 {
            return Err(LibUtilError::InvalidLoad { load });
        }

        let delay = match transition {
            Transition::Rise => self.rise_block_delay + self.rise_fanout_delay * load,
            Transition::Fall => self.fall_block_delay + self.fall_fanout_delay * load,
        };

        Ok(delay)
    }

    pub fn worst_delay_at_load(self, load: f64) -> Result<f64, LibUtilError> {
        Ok(self
            .delay_at_load(Transition::Rise, load)?
            .max(self.delay_at_load(Transition::Fall, load)?))
    }
}

impl From<&GenlibPin> for PinDelay {
    fn from(value: &GenlibPin) -> Self {
        Self {
            phase: value.phase,
            input_load: value.input_load,
            max_load: value.max_load,
            rise_block_delay: value.rise_block_delay,
            rise_fanout_delay: value.rise_fanout_delay,
            fall_block_delay: value.fall_block_delay,
            fall_fanout_delay: value.fall_fanout_delay,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LibraryClassId(usize);

impl LibraryClassId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryClass {
    id: LibraryClassId,
    signature: String,
    gate_names: Vec<String>,
    dual: Option<LibraryClassId>,
}

impl LibraryClass {
    pub fn id(&self) -> LibraryClassId {
        self.id
    }

    pub fn signature(&self) -> &str {
        &self.signature
    }

    pub fn gate_names(&self) -> &[String] {
        &self.gate_names
    }

    pub fn name(&self) -> Option<&str> {
        self.gate_names.first().map(String::as_str)
    }

    pub fn dual(&self) -> Option<LibraryClassId> {
        self.dual
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryIndex {
    classes: Vec<LibraryClass>,
    gate_to_class: HashMap<String, LibraryClassId>,
}

impl LibraryIndex {
    pub fn from_genlib(library: &GenlibLibrary) -> Self {
        let mut class_by_signature = BTreeMap::<String, Vec<String>>::new();
        for gate in &library.gates {
            class_by_signature
                .entry(class_signature(gate))
                .or_default()
                .push(gate.name.clone());
        }

        let mut classes = class_by_signature
            .into_iter()
            .enumerate()
            .map(|(index, (signature, gate_names))| LibraryClass {
                id: LibraryClassId(index),
                signature,
                gate_names,
                dual: None,
            })
            .collect::<Vec<_>>();

        let class_by_signature = classes
            .iter()
            .map(|class| (class.signature.clone(), class.id))
            .collect::<HashMap<_, _>>();

        for class in &mut classes {
            let dual_signature = dual_signature(&class.signature);
            class.dual = class_by_signature.get(&dual_signature).copied();
        }

        let mut gate_to_class = HashMap::new();
        for class in &classes {
            for gate in &class.gate_names {
                gate_to_class.insert(gate.clone(), class.id);
            }
        }

        Self {
            classes,
            gate_to_class,
        }
    }

    pub fn classes(&self) -> &[LibraryClass] {
        &self.classes
    }

    pub fn class(&self, id: LibraryClassId) -> Option<&LibraryClass> {
        self.classes.get(id.index())
    }

    pub fn class_for_gate(&self, gate: &GenlibGate) -> Option<&LibraryClass> {
        self.gate_to_class
            .get(&gate.name)
            .and_then(|id| self.class(*id))
    }

    pub fn class_for_gate_name(&self, name: &str) -> Option<&LibraryClass> {
        self.gate_to_class.get(name).and_then(|id| self.class(*id))
    }

    pub fn gates_in_class<'a>(
        &'a self,
        library: &'a GenlibLibrary,
        class: LibraryClassId,
    ) -> Result<Vec<&'a GenlibGate>, LibUtilError> {
        let class = self
            .class(class)
            .ok_or(LibUtilError::MissingClass { class })?;
        class
            .gate_names
            .iter()
            .map(|name| {
                get_gate(library, name)
                    .ok_or_else(|| LibUtilError::MissingGate { name: name.clone() })
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GateLoadDelay {
    pub gate: String,
    pub area: f64,
    pub input_load: f64,
    pub max_load: f64,
    pub worst_block_delay: f64,
    pub worst_fanout_delay: f64,
    pub worst_delay_at_load: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappedGateInfo {
    pub node: NodeId,
    pub node_name: String,
    pub gate_name: String,
    pub input_count: usize,
    pub load: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LibUtilError {
    MissingGate {
        name: String,
    },
    MissingClass {
        class: LibraryClassId,
    },
    MissingNode {
        node: NodeId,
    },
    UnmappedNode {
        node: NodeId,
    },
    InvalidPin {
        gate: String,
        direction: PinDirection,
        pin: usize,
    },
    InvalidLoad {
        load: f64,
    },
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for LibUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingGate { name } => write!(f, "library gate '{name}' was not found"),
            Self::MissingClass { class } => {
                write!(f, "library class {} was not found", class.index())
            }
            Self::MissingNode { node } => {
                write!(f, "virtual network node {} was not found", node.index())
            }
            Self::UnmappedNode { node } => {
                write!(
                    f,
                    "virtual network node {} has no mapped gate",
                    node.index()
                )
            }
            Self::InvalidPin {
                gate,
                direction,
                pin,
            } => write!(
                f,
                "gate '{gate}' has no {direction:?} pin at position {pin}"
            ),
            Self::InvalidLoad { load } => write!(f, "invalid output load {load}"),
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for LibUtilError {}

pub fn get_gate<'a>(library: &'a GenlibLibrary, name: &str) -> Option<&'a GenlibGate> {
    library.gate(name)
}

pub fn class_name(class: &LibraryClass) -> Option<&str> {
    class.name()
}

pub fn class_dual(index: &LibraryIndex, class: LibraryClassId) -> Option<&LibraryClass> {
    index
        .class(class)?
        .dual()
        .and_then(|dual| index.class(dual))
}

pub fn gate_name(gate: Option<&GenlibGate>) -> Option<&str> {
    gate.map(|gate| gate.name.as_str())
}

pub fn gate_area(gate: Option<&GenlibGate>) -> f64 {
    gate.map_or(0.0, |gate| gate.area)
}

pub fn gate_class<'a>(index: &'a LibraryIndex, gate: &GenlibGate) -> Option<&'a LibraryClass> {
    index.class_for_gate(gate)
}

pub fn gate_num_inputs(gate: Option<&GenlibGate>) -> isize {
    gate.map_or(-1, |gate| gate.pins.len() as isize)
}

pub fn gate_num_outputs(gate: Option<&GenlibGate>) -> isize {
    gate.map_or(-1, |_| 1)
}

pub fn gate_pin_name(
    gate: &GenlibGate,
    pin: usize,
    direction: PinDirection,
) -> Result<&str, LibUtilError> {
    match direction {
        PinDirection::Output if pin == 0 => Ok(gate.output.name.as_str()),
        PinDirection::Output => Err(LibUtilError::InvalidPin {
            gate: gate.name.clone(),
            direction,
            pin,
        }),
        PinDirection::Input => gate
            .pins
            .get(pin)
            .map(|pin| pin.declared_name.as_str())
            .ok_or_else(|| LibUtilError::InvalidPin {
                gate: gate.name.clone(),
                direction,
                pin,
            }),
        PinDirection::Control => Err(LibUtilError::MissingSisPorts {
            operation: "lib_gate_pin_name control latch pin lookup",
        }),
    }
}

pub fn pin_delay(gate: &GenlibGate, pin: usize) -> Result<PinDelay, LibUtilError> {
    gate.pins
        .get(pin)
        .map(PinDelay::from)
        .ok_or_else(|| LibUtilError::InvalidPin {
            gate: gate.name.clone(),
            direction: PinDirection::Input,
            pin,
        })
}

pub fn pin_delay_at_load(
    gate: &GenlibGate,
    pin: usize,
    transition: Transition,
    load: f64,
) -> Result<f64, LibUtilError> {
    pin_delay(gate, pin)?.delay_at_load(transition, load)
}

pub fn gate_load_delay(gate: &GenlibGate, load: f64) -> Result<GateLoadDelay, LibUtilError> {
    if !load.is_finite() || load < 0.0 {
        return Err(LibUtilError::InvalidLoad { load });
    }

    let mut input_load = 0.0;
    let mut max_load = f64::INFINITY;
    let mut worst_block_delay: f64 = 0.0;
    let mut worst_fanout_delay: f64 = 0.0;
    let mut worst_delay_at_load: f64 = 0.0;

    for pin in &gate.pins {
        let delay = PinDelay::from(pin);
        input_load += delay.input_load;
        max_load = max_load.min(delay.max_load);
        worst_block_delay =
            worst_block_delay.max(delay.rise_block_delay.max(delay.fall_block_delay));
        worst_fanout_delay =
            worst_fanout_delay.max(delay.rise_fanout_delay.max(delay.fall_fanout_delay));
        worst_delay_at_load = worst_delay_at_load.max(delay.worst_delay_at_load(load)?);
    }

    if gate.pins.is_empty() {
        max_load = 0.0;
    }

    Ok(GateLoadDelay {
        gate: gate.name.clone(),
        area: gate.area,
        input_load,
        max_load,
        worst_block_delay,
        worst_fanout_delay,
        worst_delay_at_load,
    })
}

pub fn choose_smallest_gate<'a>(
    gates: impl IntoIterator<Item = &'a GenlibGate>,
) -> Option<&'a GenlibGate> {
    gates.into_iter().min_by(|left, right| {
        left.area
            .total_cmp(&right.area)
            .then_with(|| left.name.cmp(&right.name))
    })
}

pub fn choose_smallest_gate_in_class<'a>(
    library: &'a GenlibLibrary,
    index: &'a LibraryIndex,
    class: LibraryClassId,
) -> Result<Option<&'a GenlibGate>, LibUtilError> {
    Ok(choose_smallest_gate(index.gates_in_class(library, class)?))
}

pub fn mapped_gate_info(
    network: &VirtualMappedNetwork,
    node: NodeId,
) -> Result<MappedGateInfo, LibUtilError> {
    let mapped_node = network
        .node(node)
        .ok_or(LibUtilError::MissingNode { node })?;
    let gate = mapped_node
        .gate
        .as_ref()
        .ok_or(LibUtilError::UnmappedNode { node })?;

    Ok(MappedGateInfo {
        node,
        node_name: mapped_node.name.clone(),
        gate_name: mapped_gate_name(gate).to_string(),
        input_count: mapped_node.save_binding.len(),
        load: mapped_node.load,
    })
}

pub fn mapped_library_gate<'a>(
    library: &'a GenlibLibrary,
    network: &VirtualMappedNetwork,
    node: NodeId,
) -> Result<Option<&'a GenlibGate>, LibUtilError> {
    let info = mapped_gate_info(network, node)?;
    Ok(get_gate(library, &info.gate_name))
}

pub fn network_is_mapped(network: &VirtualMappedNetwork) -> bool {
    network
        .nodes()
        .iter()
        .any(|node| node.kind == NodeKind::Internal && node.gate.is_some())
}

pub fn get_class_by_type_unavailable() -> Result<LibraryClassId, LibUtilError> {
    Err(LibUtilError::MissingSisPorts {
        operation: "lib_get_class_by_type full SIS network pattern matching",
    })
}

pub fn set_gate_unavailable() -> Result<(), LibUtilError> {
    Err(LibUtilError::MissingSisPorts {
        operation: "lib_set_gate full SIS network replacement",
    })
}

pub fn free_library_unavailable() -> Result<(), LibUtilError> {
    Err(LibUtilError::MissingSisPorts {
        operation: "lib_free legacy primitive/network ownership cleanup",
    })
}

fn mapped_gate_name(gate: &GateKind) -> &str {
    match gate {
        GateKind::Library(name) => name.as_str(),
        _ => gate.mnemonic(),
    }
}

fn class_signature(gate: &GenlibGate) -> String {
    let expression = normalize_expression(&gate.output.expression);
    format!("{}:{}", gate.pins.len(), expression)
}

fn dual_signature(signature: &str) -> String {
    let Some((arity, expression)) = signature.split_once(':') else {
        return signature.to_string();
    };
    let dual = if let Some(stripped) = expression
        .strip_prefix("!(")
        .and_then(|value| value.strip_suffix(')'))
    {
        stripped.to_string()
    } else if let Some(stripped) = expression.strip_prefix('!') {
        stripped.to_string()
    } else {
        format!("!({expression})")
    };

    format!("{arity}:{dual}")
}

fn normalize_expression(expression: &str) -> String {
    expression
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != ';')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::library::parse_genlib;
    use crate::ports::map::virtual_net::{SourceRef, VirtualMappedNetwork};

    fn sample_library() -> GenlibLibrary {
        parse_genlib(concat!(
            "GATE and2_slow 5 O=a*b;\n",
            "PIN a NONINV 1 10 2 .5 3 .7\n",
            "PIN b NONINV 2 8 1 .25 4 .75\n",
            "GATE and2_fast 2 O=a*b;\n",
            "PIN a NONINV 1 10 1 .2 1 .2\n",
            "PIN b NONINV 1 10 1 .2 1 .2\n",
            "GATE nand2 3 O=!(a*b);\n",
            "PIN a INV 1 9 1 .3 1 .3\n",
            "PIN b INV 1 9 1 .3 1 .3\n",
        ))
        .unwrap()
    }

    #[test]
    fn indexes_classes_and_selects_smallest_gate_by_area() {
        let library = sample_library();
        let index = LibraryIndex::from_genlib(&library);
        let class = index.class_for_gate_name("and2_slow").unwrap();

        assert_eq!(class_name(class), Some("and2_slow"));
        assert_eq!(
            class.gate_names(),
            &["and2_slow".to_string(), "and2_fast".to_string()]
        );
        assert_eq!(
            choose_smallest_gate_in_class(&library, &index, class.id())
                .unwrap()
                .unwrap()
                .name,
            "and2_fast"
        );
        assert_eq!(
            class_dual(&index, class.id()).and_then(LibraryClass::name),
            Some("nand2")
        );
    }

    #[test]
    fn exposes_gate_pin_area_and_delay_accessors() {
        let library = sample_library();
        let gate = get_gate(&library, "and2_slow").unwrap();

        assert_eq!(gate_name(Some(gate)), Some("and2_slow"));
        assert_eq!(gate_area(Some(gate)), 5.0);
        assert_eq!(gate_num_inputs(Some(gate)), 2);
        assert_eq!(gate_num_outputs(Some(gate)), 1);
        assert_eq!(gate_pin_name(gate, 0, PinDirection::Output).unwrap(), "O");
        assert_eq!(gate_pin_name(gate, 1, PinDirection::Input).unwrap(), "b");
        assert_eq!(
            pin_delay_at_load(gate, 1, Transition::Fall, 2.0).unwrap(),
            5.5
        );

        let summary = gate_load_delay(gate, 2.0).unwrap();
        assert_eq!(summary.input_load, 3.0);
        assert_eq!(summary.max_load, 8.0);
        assert_eq!(summary.worst_block_delay, 4.0);
        assert_eq!(summary.worst_fanout_delay, 0.75);
        assert_eq!(summary.worst_delay_at_load, 5.5);
    }

    #[test]
    fn inspects_virtual_mapped_gate_information() {
        let library = sample_library();
        let mut network = VirtualMappedNetwork::new();
        let a = network.add_primary_input("a");
        let b = network.add_primary_input("b");
        let n1 = network.add_gate(
            "n1",
            GateKind::Library("and2_fast".to_string()),
            vec![SourceRef::Node(a), SourceRef::Node(b)],
        );

        assert!(network_is_mapped(&network));

        let info = mapped_gate_info(&network, n1).unwrap();
        assert_eq!(info.gate_name, "and2_fast");
        assert_eq!(info.input_count, 2);
        assert_eq!(
            mapped_library_gate(&library, &network, n1)
                .unwrap()
                .unwrap()
                .name,
            "and2_fast"
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("libutil.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
