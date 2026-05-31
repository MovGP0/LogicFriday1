//! Native BDNET writer for mapped SIS networks.
//!
//! The legacy writer emits a textual BDNET netlist from a mapped SIS network and
//! depends on command flags, library gates, and latch metadata.  This port keeps
//! those inputs as owned Rust data so integration can feed the writer from the
//! native network, mapper, command, and latch layers without adding C entry
//! points.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fmt::Write;

pub const OCT_CELL_PATH: &str = "OCT-CELL-PATH";
pub const OCT_CELL_VIEW: &str = "OCT-CELL-VIEW";
pub const OCT_TECHNOLOGY: &str = "OCT-TECHNOLOGY";
pub const OCT_VIEWTYPE: &str = "OCT-VIEWTYPE";
pub const OCT_EDITSTYLE: &str = "OCT-EDITSTYLE";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BdnetNetwork {
    pub name: String,
    pub is_mapped: bool,
    pub flags: BTreeMap<String, String>,
    pub primary_inputs: Vec<BdnetPrimaryInput>,
    pub primary_outputs: Vec<BdnetPrimaryOutput>,
    pub instances: Vec<BdnetInstance>,
}

impl BdnetNetwork {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_mapped: true,
            flags: BTreeMap::new(),
            primary_inputs: Vec::new(),
            primary_outputs: Vec::new(),
            instances: Vec::new(),
        }
    }

    pub fn with_flag(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.flags.insert(name.into(), value.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BdnetPrimaryInput {
    pub name: String,
    pub is_real: bool,
}

impl BdnetPrimaryInput {
    pub fn real(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_real: true,
        }
    }

    pub fn pseudo(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_real: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BdnetPrimaryOutput {
    pub name: String,
    pub driver: String,
    pub is_real: bool,
}

impl BdnetPrimaryOutput {
    pub fn real(name: impl Into<String>, driver: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            driver: driver.into(),
            is_real: true,
        }
    }

    pub fn pseudo(name: impl Into<String>, driver: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            driver: driver.into(),
            is_real: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BdnetGateKind {
    Combinational,
    Sequential,
    Asynchronous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BdnetGate {
    pub name: String,
    pub kind: BdnetGateKind,
    pub input_pins: Vec<String>,
    pub output_pin: String,
    pub control_pin: Option<String>,
    pub latch_pin: Option<usize>,
}

impl BdnetGate {
    pub fn combinational(
        name: impl Into<String>,
        input_pins: impl Into<Vec<String>>,
        output_pin: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: BdnetGateKind::Combinational,
            input_pins: input_pins.into(),
            output_pin: output_pin.into(),
            control_pin: None,
            latch_pin: None,
        }
    }

    pub fn sequential(
        name: impl Into<String>,
        input_pins: impl Into<Vec<String>>,
        output_pin: impl Into<String>,
        control_pin: impl Into<String>,
        latch_pin: usize,
    ) -> Self {
        Self {
            name: name.into(),
            kind: BdnetGateKind::Sequential,
            input_pins: input_pins.into(),
            output_pin: output_pin.into(),
            control_pin: Some(control_pin.into()),
            latch_pin: Some(latch_pin),
        }
    }

    pub fn asynchronous(
        name: impl Into<String>,
        input_pins: impl Into<Vec<String>>,
        output_pin: impl Into<String>,
        latch_pin: usize,
    ) -> Self {
        Self {
            name: name.into(),
            kind: BdnetGateKind::Asynchronous,
            input_pins: input_pins.into(),
            output_pin: output_pin.into(),
            control_pin: None,
            latch_pin: Some(latch_pin),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BdnetInstance {
    pub gate: BdnetGate,
    pub fanins: Vec<String>,
    pub output_signal: String,
    pub latch: Option<BdnetLatch>,
}

impl BdnetInstance {
    pub fn combinational(
        gate: BdnetGate,
        fanins: impl Into<Vec<String>>,
        output_signal: impl Into<String>,
    ) -> Self {
        Self {
            gate,
            fanins: fanins.into(),
            output_signal: output_signal.into(),
            latch: None,
        }
    }

    pub fn sequential(gate: BdnetGate, fanins: impl Into<Vec<String>>, latch: BdnetLatch) -> Self {
        Self {
            gate,
            fanins: fanins.into(),
            output_signal: String::new(),
            latch: Some(latch),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BdnetLatch {
    pub output_signal: String,
    pub control_signal: Option<String>,
}

impl BdnetLatch {
    pub fn new(output_signal: impl Into<String>, control_signal: Option<String>) -> Self {
        Self {
            output_signal: output_signal.into(),
            control_signal,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BdnetWriteError {
    NetworkNotMapped,
    MissingInputPin { gate: String, pin: usize },
    MissingLatch { gate: String },
    MissingLatchPin { gate: String },
    MissingControlPin { gate: String },
}

impl fmt::Display for BdnetWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkNotMapped => {
                write!(
                    formatter,
                    "write_bdnet: network not mapped, cannot write netlist"
                )
            }
            Self::MissingInputPin { gate, pin } => {
                write!(formatter, "gate {gate} is missing input pin {pin}")
            }
            Self::MissingLatch { gate } => {
                write!(formatter, "sequential gate {gate} is missing latch data")
            }
            Self::MissingLatchPin { gate } => {
                write!(formatter, "sequential gate {gate} is missing latch pin")
            }
            Self::MissingControlPin { gate } => {
                write!(formatter, "sequential gate {gate} is missing control pin")
            }
        }
    }
}

impl Error for BdnetWriteError {}

pub fn write_bdnet(network: &BdnetNetwork) -> Result<String, BdnetWriteError> {
    if !network.is_mapped {
        return Err(BdnetWriteError::NetworkNotMapped);
    }

    let mut output = String::new();
    writeln!(output, "MODEL \"{}\";", network.name).unwrap();
    write_optional_flag(&mut output, &network.flags, OCT_TECHNOLOGY, "TECHNOLOGY");
    write_optional_flag(&mut output, &network.flags, OCT_VIEWTYPE, "VIEWTYPE");
    write_optional_flag(&mut output, &network.flags, OCT_EDITSTYLE, "EDITSTYLE");
    output.push('\n');

    output.push_str("INPUT");
    for input in network.primary_inputs.iter().filter(|input| input.is_real) {
        writeln!(output, "\n\t\"{}\"\t:\t\"{}\"", input.name, input.name).unwrap();
    }
    output.push_str(";\n\n");

    output.push_str("OUTPUT");
    for primary_output in network
        .primary_outputs
        .iter()
        .filter(|primary_output| primary_output.is_real)
    {
        writeln!(
            output,
            "\n\t\"{}\"\t:\t\"{}\"",
            primary_output.name, primary_output.driver
        )
        .unwrap();
    }
    output.push_str(";\n\n");

    let path = network.flags.get(OCT_CELL_PATH).map(String::as_str);
    let viewname = network
        .flags
        .get(OCT_CELL_VIEW)
        .map(String::as_str)
        .unwrap_or("physical");

    for instance in &network.instances {
        write_instance(&mut output, path, viewname, instance)?;
    }

    output.push_str("ENDMODEL;\n");
    Ok(output)
}

fn write_optional_flag(
    output: &mut String,
    flags: &BTreeMap<String, String>,
    flag_name: &str,
    bdnet_name: &str,
) {
    if let Some(value) = flags.get(flag_name) {
        writeln!(output, "    {bdnet_name} {value};").unwrap();
    }
}

fn write_instance(
    output: &mut String,
    path: Option<&str>,
    default_view: &str,
    instance: &BdnetInstance,
) -> Result<(), BdnetWriteError> {
    write_instance_name(output, path, default_view, &instance.gate.name);
    match instance.gate.kind {
        BdnetGateKind::Combinational => {
            for (pin_index, fanin) in instance.fanins.iter().enumerate() {
                let pin = input_pin(&instance.gate, pin_index)?;
                write_pin_connection(output, pin, fanin);
            }
            write_pin_connection(output, &instance.gate.output_pin, &instance.output_signal);
            output.push('\n');
        }
        BdnetGateKind::Sequential | BdnetGateKind::Asynchronous => {
            let latch_pin =
                instance
                    .gate
                    .latch_pin
                    .ok_or_else(|| BdnetWriteError::MissingLatchPin {
                        gate: instance.gate.name.clone(),
                    })?;
            for (pin_index, fanin) in instance.fanins.iter().enumerate() {
                if pin_index == latch_pin {
                    continue;
                }

                let pin = input_pin(&instance.gate, pin_index)?;
                write_pin_connection(output, pin, fanin);
            }

            let latch = instance
                .latch
                .as_ref()
                .ok_or_else(|| BdnetWriteError::MissingLatch {
                    gate: instance.gate.name.clone(),
                })?;

            if instance.gate.kind != BdnetGateKind::Asynchronous {
                let control_pin = instance.gate.control_pin.as_deref().ok_or_else(|| {
                    BdnetWriteError::MissingControlPin {
                        gate: instance.gate.name.clone(),
                    }
                })?;
                match &latch.control_signal {
                    Some(control) => write_pin_connection(output, control_pin, control),
                    None => write_unconnected_pin(output, control_pin),
                }
            }

            write_pin_connection(output, &instance.gate.output_pin, &latch.output_signal);
            output.push('\n');
        }
    }

    Ok(())
}

fn write_instance_name(
    output: &mut String,
    path: Option<&str>,
    default_view: &str,
    gate_name: &str,
) {
    let (gate, view) = gate_name
        .split_once(':')
        .map(|(gate, view)| (gate, view))
        .unwrap_or((gate_name, default_view));

    if path.is_none() || gate.starts_with('/') || gate.starts_with('~') {
        writeln!(output, "INSTANCE \"{gate}\":\"{view}\"").unwrap();
    } else {
        writeln!(output, "INSTANCE \"{}/{}\":\"{view}\"", path.unwrap(), gate).unwrap();
    }
}

fn input_pin(gate: &BdnetGate, pin_index: usize) -> Result<&str, BdnetWriteError> {
    gate.input_pins
        .get(pin_index)
        .map(String::as_str)
        .ok_or_else(|| BdnetWriteError::MissingInputPin {
            gate: gate.name.clone(),
            pin: pin_index,
        })
}

fn write_pin_connection(output: &mut String, pin: &str, signal: &str) {
    writeln!(output, "\t\"{pin}\"\t:\t\"{signal}\";").unwrap();
}

fn write_unconnected_pin(output: &mut String, pin: &str) {
    writeln!(output, "\t\"{pin}\"\t:\tUNCONNECTED;").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn and_gate() -> BdnetGate {
        BdnetGate::combinational("std/and2", vec!["A".to_owned(), "B".to_owned()], "Y")
    }

    #[test]
    fn rejects_unmapped_networks() {
        let mut network = BdnetNetwork::new("demo");
        network.is_mapped = false;

        assert_eq!(
            write_bdnet(&network).unwrap_err(),
            BdnetWriteError::NetworkNotMapped
        );
    }

    #[test]
    fn writes_header_flags_and_real_io_only() {
        let network = BdnetNetwork::new("alu")
            .with_flag(OCT_TECHNOLOGY, "cmos")
            .with_flag(OCT_VIEWTYPE, "NETLIST")
            .with_flag(OCT_EDITSTYLE, "SYMBOLIC");
        let mut network = network;
        network.primary_inputs = vec![BdnetPrimaryInput::real("a"), BdnetPrimaryInput::pseudo("s")];
        network.primary_outputs = vec![
            BdnetPrimaryOutput::real("y", "n1"),
            BdnetPrimaryOutput::pseudo("state", "q"),
        ];

        let output = write_bdnet(&network).unwrap();

        assert!(output.starts_with(
            "MODEL \"alu\";\n    TECHNOLOGY cmos;\n    VIEWTYPE NETLIST;\n    EDITSTYLE SYMBOLIC;\n\n"
        ));
        assert!(output.contains("INPUT\n\t\"a\"\t:\t\"a\"\n;\n\n"));
        assert!(!output.contains("\"s\""));
        assert!(output.contains("OUTPUT\n\t\"y\"\t:\t\"n1\"\n;\n\n"));
        assert!(!output.contains("\"state\""));
    }

    #[test]
    fn writes_combinational_instances_with_default_view() {
        let mut network = BdnetNetwork::new("mapped").with_flag(OCT_CELL_PATH, "/lib");
        network.instances.push(BdnetInstance::combinational(
            and_gate(),
            vec!["a".to_owned(), "b".to_owned()],
            "n1",
        ));

        let output = write_bdnet(&network).unwrap();

        assert!(output.contains(
            "INSTANCE \"/lib/std/and2\":\"physical\"\n\t\"A\"\t:\t\"a\";\n\t\"B\"\t:\t\"b\";\n\t\"Y\"\t:\t\"n1\";\n\n"
        ));
    }

    #[test]
    fn gate_view_overrides_default_view_and_absolute_paths_are_not_prefixed() {
        let mut network = BdnetNetwork::new("mapped")
            .with_flag(OCT_CELL_PATH, "/cells")
            .with_flag(OCT_CELL_VIEW, "symbolic");
        network.instances.push(BdnetInstance::combinational(
            BdnetGate::combinational("/vendor/nand2:layout", vec!["A".to_owned()], "Y"),
            vec!["a".to_owned()],
            "n",
        ));

        let output = write_bdnet(&network).unwrap();

        assert!(output.contains("INSTANCE \"/vendor/nand2\":\"layout\"\n"));
    }

    #[test]
    fn sequential_instances_skip_latch_data_pin_and_write_control() {
        let mut network = BdnetNetwork::new("seq");
        network.instances.push(BdnetInstance::sequential(
            BdnetGate::sequential(
                "dff",
                vec!["D".to_owned(), "SCAN".to_owned()],
                "Q",
                "CLK",
                0,
            ),
            vec!["d".to_owned(), "scan".to_owned()],
            BdnetLatch::new("state", Some("clk".to_owned())),
        ));

        let output = write_bdnet(&network).unwrap();

        assert!(output.contains(
            "INSTANCE \"dff\":\"physical\"\n\t\"SCAN\"\t:\t\"scan\";\n\t\"CLK\"\t:\t\"clk\";\n\t\"Q\"\t:\t\"state\";\n\n"
        ));
        assert!(!output.contains("\"D\"\t:\t\"d\""));
    }

    #[test]
    fn sequential_instances_write_unconnected_missing_control() {
        let mut network = BdnetNetwork::new("seq");
        network.instances.push(BdnetInstance::sequential(
            BdnetGate::sequential("dff", vec!["D".to_owned()], "Q", "CLK", 0),
            vec!["d".to_owned()],
            BdnetLatch::new("state", None),
        ));

        let output = write_bdnet(&network).unwrap();

        assert!(output.contains("\t\"CLK\"\t:\tUNCONNECTED;\n\t\"Q\"\t:\t\"state\";\n\n"));
    }

    #[test]
    fn asynchronous_instances_do_not_write_control_pin() {
        let mut network = BdnetNetwork::new("seq");
        network.instances.push(BdnetInstance::sequential(
            BdnetGate::asynchronous("latch", vec!["D".to_owned(), "RST".to_owned()], "Q", 0),
            vec!["d".to_owned(), "reset".to_owned()],
            BdnetLatch::new("state", Some("clk".to_owned())),
        ));

        let output = write_bdnet(&network).unwrap();

        assert!(output.contains(
            "INSTANCE \"latch\":\"physical\"\n\t\"RST\"\t:\t\"reset\";\n\t\"Q\"\t:\t\"state\";\n\n"
        ));
        assert!(!output.contains("clk"));
    }

    #[test]
    fn reports_missing_pin_metadata() {
        let mut network = BdnetNetwork::new("bad");
        network.instances.push(BdnetInstance::combinational(
            BdnetGate::combinational("bad_gate", vec!["A".to_owned()], "Y"),
            vec!["a".to_owned(), "b".to_owned()],
            "n",
        ));

        assert_eq!(
            write_bdnet(&network).unwrap_err(),
            BdnetWriteError::MissingInputPin {
                gate: "bad_gate".to_owned(),
                pin: 1
            }
        );
    }

    #[test]
    fn source_contains_no_dependency_metadata_or_c_abi_export() {
        let source = include_str!("write_bdnet.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
