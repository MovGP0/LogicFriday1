use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::slice;
use std::str;

use crate::ports::map::com_map;
use crate::ports::map::library as genlib_library;
use crate::ports::map::map_interface;
use crate::ports::map::two_level;

thread_local! {
    static LAST_ERROR: RefCell<String> = RefCell::new(String::new());
}

const OPTION_INVERTED_OUTPUTS: u32 = 1;
const OPTION_READ_LIBRARY_NO_DECOMP: u32 = 2;
const OPTION_MAP_M1: u32 = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BlifModel {
    inputs: Vec<String>,
    outputs: Vec<String>,
    nodes: Vec<BlifNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BlifNode {
    fanins: Vec<String>,
    output: String,
    covers: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Gate {
    id: String,
    kind: GateKind,
    inputs: Vec<String>,
    output: String,
    level: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LibraryGate {
    name: String,
    area_text: String,
    expression: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GateKind {
    And,
    Not,
    Or,
    Const0,
    Const1,
    Buf,
}

pub unsafe fn map_blif_to_json(
    blif_ptr: *const u8,
    blif_len: usize,
    options: u32,
    output_ptr: *mut u8,
    output_len: usize,
) -> usize {
    match unsafe { map_blif_genlib_to_json_core(blif_ptr, blif_len, std::ptr::null(), 0, options) }
    {
        Ok(json) => {
            set_last_error("");
            unsafe { write_buffer(json.as_bytes(), output_ptr, output_len) }
        }
        Err(error) => {
            set_last_error(&error);
            0
        }
    }
}

pub unsafe fn map_blif_genlib_to_json(
    blif_ptr: *const u8,
    blif_len: usize,
    genlib_ptr: *const u8,
    genlib_len: usize,
    options: u32,
    output_ptr: *mut u8,
    output_len: usize,
) -> usize {
    match unsafe {
        map_blif_genlib_to_json_core(blif_ptr, blif_len, genlib_ptr, genlib_len, options)
    } {
        Ok(json) => {
            set_last_error("");
            unsafe { write_buffer(json.as_bytes(), output_ptr, output_len) }
        }
        Err(error) => {
            set_last_error(&error);
            0
        }
    }
}

pub unsafe fn last_error(output_ptr: *mut u8, output_len: usize) -> usize {
    LAST_ERROR
        .with(|error| unsafe { write_buffer(error.borrow().as_bytes(), output_ptr, output_len) })
}

unsafe fn map_blif_genlib_to_json_core(
    blif_ptr: *const u8,
    blif_len: usize,
    genlib_ptr: *const u8,
    genlib_len: usize,
    options: u32,
) -> Result<String, String> {
    let blif = read_utf8("BLIF input", blif_ptr, blif_len)?;
    let genlib = read_utf8("genlib input", genlib_ptr, genlib_len)?;
    let two_level_model = two_level::parse_blif(blif, two_level::ParseLimits::default())
        .map_err(|error| format!("BLIF mapper input is invalid: {error}"))?;
    let genlib_options = if options & OPTION_READ_LIBRARY_NO_DECOMP != 0 {
        genlib_library::ReadLibraryOptions::read_library_dash_n()
    } else {
        genlib_library::ReadLibraryOptions::read_library()
    };
    let parsed_genlib = if genlib.trim().is_empty() {
        None
    } else {
        Some(
            genlib_library::parse_genlib_with_options(
                genlib,
                genlib_options,
                genlib_library::ParseLimits::default(),
            )
            .map_err(|error| format!("genlib mapper input is invalid: {error}"))?,
        )
    };
    let interface_result = map_interface::map_two_level_with_genlib_to_virtual_network(
        &two_level_model,
        parsed_genlib.as_ref(),
        &facade_map_options(options),
    )
    .ok();
    let model = parse_blif(blif)?;
    let library = parse_genlib(genlib)?;
    let gates = synthesize_gates(&model, options);

    Ok(write_mapping_json(
        &model,
        parsed_genlib
            .as_ref()
            .map_or(library.len(), |item| item.gates.len()),
        &library,
        &gates,
        interface_result.as_ref(),
        options,
    ))
}

fn read_utf8<'a>(name: &str, ptr: *const u8, len: usize) -> Result<&'a str, String> {
    if ptr.is_null() && len != 0 {
        return Err(format!("{name} pointer is null"));
    }

    let bytes = if len == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(ptr, len) }
    };
    str::from_utf8(bytes).map_err(|error| format!("{name} is not UTF-8: {error}"))
}

fn parse_blif(blif: &str) -> Result<BlifModel, String> {
    let mut model = BlifModel {
        inputs: Vec::new(),
        outputs: Vec::new(),
        nodes: Vec::new(),
    };
    let mut current_node: Option<BlifNode> = None;

    for (line_index, raw_line) in blif.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('.') {
            if let Some(node) = current_node.take() {
                model.nodes.push(node);
            }

            let fields: Vec<&str> = line.split_whitespace().collect();
            match fields.as_slice() {
                [".model", ..] => {}
                [".inputs", values @ ..] => extend_unique(&mut model.inputs, values),
                [".outputs", values @ ..] => extend_unique(&mut model.outputs, values),
                [".names", values @ ..] if !values.is_empty() => {
                    let output = values[values.len() - 1].to_string();
                    let fanins = values[..values.len() - 1]
                        .iter()
                        .map(|value| (*value).to_string())
                        .collect();
                    current_node = Some(BlifNode {
                        fanins,
                        output,
                        covers: Vec::new(),
                    });
                }
                [".end"] => break,
                [directive, ..] => {
                    return Err(format!(
                        "unsupported BLIF directive {directive} at line {}",
                        line_index + 1
                    ));
                }
                [] => {}
            }
        } else if let Some(node) = current_node.as_mut() {
            node.covers.push(line.to_string());
        } else {
            return Err(format!(
                "cover row appears before .names at line {}",
                line_index + 1
            ));
        }
    }

    if let Some(node) = current_node {
        model.nodes.push(node);
    }

    if model.outputs.is_empty() {
        return Err("BLIF input does not declare .outputs".to_string());
    }

    Ok(model)
}

fn parse_genlib(genlib: &str) -> Result<Vec<LibraryGate>, String> {
    let mut gates = Vec::new();

    for (line_index, raw_line) in genlib.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        let fields = line.split_whitespace().collect::<Vec<_>>();
        match fields.as_slice() {
            ["GATE", name, area, expression, ..] => gates.push(LibraryGate {
                name: (*name).to_string(),
                area_text: (*area).to_string(),
                expression: (*expression).to_string(),
            }),
            ["PIN", ..] => {}
            [directive, ..] => {
                return Err(format!(
                    "unsupported genlib directive {directive} at line {}",
                    line_index + 1
                ));
            }
            [] => {}
        }
    }

    Ok(gates)
}

fn facade_map_options(options: u32) -> com_map::MapOptions {
    let mut map_options = com_map::MapOptions::default();
    if options & OPTION_MAP_M1 != 0 {
        map_options.cost_mode = com_map::MapCostMode::Delay;
    }
    map_options
}

fn synthesize_gates(model: &BlifModel, options: u32) -> Vec<Gate> {
    let mut gates = Vec::new();
    let mut levels = HashMap::new();

    for input in &model.inputs {
        levels.insert(input.clone(), 0);
    }

    for node in &model.nodes {
        let mut realized = synthesize_node(node, &levels, gates.len());
        if options & OPTION_INVERTED_OUTPUTS != 0 && model.outputs.contains(&node.output) {
            let source = realized
                .last()
                .map(|gate| gate.output.clone())
                .unwrap_or_else(|| node.output.clone());
            let level = level_of(&source, &levels) + 1;
            realized.push(Gate {
                id: format!("g{}", gates.len() + realized.len()),
                kind: GateKind::Not,
                inputs: vec![source],
                output: format!("{}_inv", node.output),
                level,
            });
        }

        if let Some(last) = realized.last() {
            levels.insert(node.output.clone(), last.level);
        } else {
            levels.insert(node.output.clone(), 0);
        }

        gates.extend(realized);
    }

    gates
}

fn synthesize_node(
    node: &BlifNode,
    levels: &HashMap<String, usize>,
    gate_offset: usize,
) -> Vec<Gate> {
    if node.fanins.is_empty() {
        return vec![constant_gate(node, gate_offset)];
    }

    if let Some(gate) = single_literal_gate(node, levels, gate_offset) {
        return vec![gate];
    }

    let mut gates = Vec::new();
    let mut product_outputs = Vec::new();

    for cover in &node.covers {
        let Some(pattern) = cover.split_whitespace().next() else {
            continue;
        };
        if pattern.len() != node.fanins.len() {
            continue;
        }

        let mut product_inputs = Vec::new();
        for (index, bit) in pattern.chars().enumerate() {
            let input = &node.fanins[index];
            match bit {
                '1' => product_inputs.push(input.clone()),
                '0' => {
                    let not_output = format!("{}_{}_not", node.output, input);
                    let level = level_of(input, levels) + 1;
                    gates.push(Gate {
                        id: format!("g{}", gate_offset + gates.len()),
                        kind: GateKind::Not,
                        inputs: vec![input.clone()],
                        output: not_output.clone(),
                        level,
                    });
                    product_inputs.push(not_output);
                }
                '-' => {}
                _ => {}
            }
        }

        let product_output = if product_inputs.len() == 1 {
            product_inputs[0].clone()
        } else {
            let output = format!("{}_p{}", node.output, product_outputs.len());
            let level = product_inputs
                .iter()
                .map(|input| level_of(input, levels))
                .max()
                .unwrap_or(0)
                + 1;
            gates.push(Gate {
                id: format!("g{}", gate_offset + gates.len()),
                kind: GateKind::And,
                inputs: product_inputs,
                output: output.clone(),
                level,
            });
            output
        };

        product_outputs.push(product_output);
    }

    if product_outputs.is_empty() {
        gates.push(constant_gate(node, gate_offset + gates.len()));
    } else if product_outputs.len() == 1 {
        let source = product_outputs.remove(0);
        if let Some(last) = gates.last_mut() {
            if last.output == source {
                last.output = node.output.clone();
            } else {
                gates.push(Gate {
                    id: format!("g{}", gate_offset + gates.len()),
                    kind: GateKind::Buf,
                    level: level_of(&source, levels) + 1,
                    inputs: vec![source],
                    output: node.output.clone(),
                });
            }
        } else {
            gates.push(Gate {
                id: format!("g{}", gate_offset + gates.len()),
                kind: GateKind::Buf,
                level: level_of(&source, levels) + 1,
                inputs: vec![source],
                output: node.output.clone(),
            });
        }
    } else {
        let level = product_outputs
            .iter()
            .map(|input| level_of(input, levels))
            .max()
            .unwrap_or(0)
            + 1;
        gates.push(Gate {
            id: format!("g{}", gate_offset + gates.len()),
            kind: GateKind::Or,
            inputs: product_outputs,
            output: node.output.clone(),
            level,
        });
    }

    gates
}

fn constant_gate(node: &BlifNode, gate_index: usize) -> Gate {
    let is_one = node.covers.iter().any(|cover| cover.trim() == "1");
    Gate {
        id: format!("g{gate_index}"),
        kind: if is_one {
            GateKind::Const1
        } else {
            GateKind::Const0
        },
        inputs: Vec::new(),
        output: node.output.clone(),
        level: 0,
    }
}

fn single_literal_gate(
    node: &BlifNode,
    levels: &HashMap<String, usize>,
    gate_index: usize,
) -> Option<Gate> {
    if node.fanins.len() != 1 || node.covers.len() != 1 {
        return None;
    }

    let pattern = node.covers[0].split_whitespace().next()?;
    let input = node.fanins[0].clone();
    let level = level_of(&input, levels) + 1;
    match pattern {
        "1" | "1 1" => Some(Gate {
            id: format!("g{gate_index}"),
            kind: GateKind::Buf,
            inputs: vec![input],
            output: node.output.clone(),
            level,
        }),
        "0" | "0 1" => Some(Gate {
            id: format!("g{gate_index}"),
            kind: GateKind::Not,
            inputs: vec![input],
            output: node.output.clone(),
            level,
        }),
        _ => None,
    }
}

fn level_of(signal: &str, levels: &HashMap<String, usize>) -> usize {
    levels.get(signal).copied().unwrap_or(0)
}

fn write_mapping_json(
    model: &BlifModel,
    library_gate_count: usize,
    library: &[LibraryGate],
    gates: &[Gate],
    interface_result: Option<&map_interface::MapInterfaceResult>,
    options: u32,
) -> String {
    let mut json = String::from("{\"inputs\":");
    write_string_array(&mut json, &model.inputs);
    json.push_str(",\"outputs\":");
    write_string_array(&mut json, &model.outputs);
    json.push_str(",\"libraryGateCount\":");
    json.push_str(&library_gate_count.to_string());
    json.push_str(",\"libraryGates\":");
    write_library_gates(&mut json, library);
    json.push_str(",\"readLibraryNoDecomp\":");
    json.push_str(if options & OPTION_READ_LIBRARY_NO_DECOMP != 0 {
        "true"
    } else {
        "false"
    });
    json.push_str(",\"mapMode\":\"");
    json.push_str(if options & OPTION_MAP_M1 != 0 {
        "m1"
    } else {
        "default"
    });
    json.push('"');
    json.push_str(",\"printGate\":\"");
    let print_gate = interface_result
        .and_then(|result| result.network.format_print_gate().ok())
        .unwrap_or_else(|| format_print_gate(gates));
    push_json_string(&mut json, &print_gate);
    json.push('"');
    json.push_str(",\"printLevelSummary\":\"");
    let print_level_summary = interface_result
        .and_then(|result| result.network.format_print_level_summary().ok())
        .unwrap_or_else(|| format!("{}\n", max_level(gates)));
    push_json_string(&mut json, &print_level_summary);
    json.push('"');
    json.push_str(",\"printLevel\":\"");
    let print_level = interface_result
        .and_then(|result| result.network.format_print_level().ok())
        .unwrap_or_else(|| format_print_level(model, gates));
    push_json_string(&mut json, &print_level);
    json.push('"');
    json.push_str(",\"gates\":[");

    for (index, gate) in gates.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        json.push_str("\"id\":\"");
        push_json_string(&mut json, &gate.id);
        json.push_str("\",\"kind\":\"");
        json.push_str(gate.kind.as_str());
        json.push_str("\",\"inputs\":");
        write_string_array(&mut json, &gate.inputs);
        json.push_str(",\"output\":\"");
        push_json_string(&mut json, &gate.output);
        json.push_str("\",\"level\":");
        json.push_str(&gate.level.to_string());
        json.push('}');
    }

    json.push_str("]}");
    json
}

fn write_library_gates(json: &mut String, library: &[LibraryGate]) {
    json.push('[');
    for (index, gate) in library.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str("{\"name\":\"");
        push_json_string(json, &gate.name);
        json.push_str("\",\"areaText\":\"");
        push_json_string(json, &gate.area_text);
        json.push_str("\",\"expression\":\"");
        push_json_string(json, &gate.expression);
        json.push_str("\"}");
    }
    json.push(']');
}

fn format_print_gate(gates: &[Gate]) -> String {
    let mut output = format!("nodes={}\n", gates.len());
    for (index, gate) in gates.iter().enumerate() {
        let display = if gate.inputs.is_empty() {
            String::new()
        } else {
            gate.inputs
                .iter()
                .enumerate()
                .map(|(pin, input)| format!(" pin{pin}={input}"))
                .collect::<String>()
        };
        output.push_str(&format!(
            "[{index}] {} {}{display}\n",
            gate.kind.print_gate_name(),
            gate.inputs.len()
        ));
    }
    output
}

fn format_print_level(model: &BlifModel, gates: &[Gate]) -> String {
    let mut levels = BTreeMap::<usize, Vec<String>>::new();
    for gate in gates {
        levels.entry(gate.level).or_default().push(gate.id.clone());
    }

    let mut output = format!("Total number of levels = {}\n", max_level(gates));
    for input in &model.inputs {
        output.push('{');
        output.push_str(input);
        output.push_str("} ");
    }
    output.push('\n');

    for (_level, items) in levels {
        for item in items {
            output.push('{');
            output.push_str(&item);
            output.push_str("} ");
        }
        output.push('\n');
    }

    output
}

fn max_level(gates: &[Gate]) -> usize {
    gates.iter().map(|gate| gate.level).max().unwrap_or(0)
}

fn write_string_array(json: &mut String, values: &[String]) {
    json.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('"');
        push_json_string(json, value);
        json.push('"');
    }
    json.push(']');
}

fn push_json_string(json: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            _ => json.push(ch),
        }
    }
}

fn extend_unique(target: &mut Vec<String>, values: &[&str]) {
    for value in values {
        if !target.iter().any(|existing| existing == value) {
            target.push((*value).to_string());
        }
    }
}

fn set_last_error(error: &str) {
    LAST_ERROR.with(|last_error| {
        last_error.replace(error.to_string());
    });
}

unsafe fn write_buffer(bytes: &[u8], output_ptr: *mut u8, output_len: usize) -> usize {
    if output_ptr.is_null() || output_len == 0 {
        return bytes.len();
    }

    let write_len = bytes.len().min(output_len.saturating_sub(1));
    if write_len > 0 {
        unsafe {
            output_ptr.copy_from_nonoverlapping(bytes.as_ptr(), write_len);
        }
    }
    unsafe {
        *output_ptr.add(write_len) = 0;
    }

    bytes.len()
}

impl GateKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Not => "not",
            Self::Or => "or",
            Self::Const0 => "const0",
            Self::Const1 => "const1",
            Self::Buf => "buf",
        }
    }

    fn print_gate_name(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Not => "inv",
            Self::Or => "or",
            Self::Const0 => "zer",
            Self::Const1 => "one",
            Self::Buf => "wire",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_blif_and_builds_sop_gate_network() {
        let blif = b".model f\n.inputs a b\n.outputs y\n.names a b y\n11 1\n.end\n";
        let json = unsafe {
            map_blif_genlib_to_json_core(blif.as_ptr(), blif.len(), std::ptr::null(), 0, 0).unwrap()
        };

        assert!(json.contains("\"inputs\":[\"a\",\"b\"]"));
        assert!(json.contains("\"kind\":\"and\""));
        assert!(json.contains("\"output\":\"y\""));
        assert!(json.contains("\"level\":1"));
        assert!(json.contains("\"printGate\":\"nodes=1\\n"));
        assert!(json.contains("and 2 pin0=a pin1=b"));
    }

    #[test]
    fn maps_inverted_single_literal_to_not_gate() {
        let blif = b".inputs a\n.outputs y\n.names a y\n0 1\n.end\n";
        let json = unsafe {
            map_blif_genlib_to_json_core(blif.as_ptr(), blif.len(), std::ptr::null(), 0, 0).unwrap()
        };

        assert!(json.contains("\"kind\":\"not\""));
        assert!(json.contains("\"inputs\":[\"a\"]"));
    }

    #[test]
    fn reports_error_for_unsupported_directive() {
        let blif = b".outputs y\n.latch a y\n.end\n";
        let error = unsafe {
            map_blif_genlib_to_json_core(blif.as_ptr(), blif.len(), std::ptr::null(), 0, 0)
                .unwrap_err()
        };

        assert!(error.contains("unsupported directive"));
        assert!(error.contains(".latch"));
    }

    #[test]
    fn parses_genlib_and_preserves_map_command_flags() {
        let blif = b".inputs a\n.outputs y\n.names a y\n1 1\n.end\n";
        let genlib = b"GATE inv 1 O=!a;\nPIN * INV 1 999 1 0 1 0\n";
        let json = unsafe {
            map_blif_genlib_to_json_core(
                blif.as_ptr(),
                blif.len(),
                genlib.as_ptr(),
                genlib.len(),
                OPTION_READ_LIBRARY_NO_DECOMP | OPTION_MAP_M1,
            )
            .unwrap()
        };

        assert!(json.contains("\"libraryGateCount\":1"));
        assert!(json.contains("\"name\":\"inv\""));
        assert!(json.contains("\"readLibraryNoDecomp\":true"));
        assert!(json.contains("\"mapMode\":\"m1\""));
        assert!(json.contains("\"printLevelSummary\":\""));
    }

    #[test]
    fn buffer_api_returns_required_length_and_preserves_last_error() {
        let blif = b".inputs a\n.outputs y\n.names a y\n1 1\n.end\n";
        let mut buffer = [0_u8; 8];
        let required = unsafe {
            map_blif_to_json(
                blif.as_ptr(),
                blif.len(),
                0,
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        };

        assert!(required > buffer.len());
        assert_eq!(buffer[buffer.len() - 1], 0);

        let bad = b".inputs a\n.end\n";
        let failed = unsafe {
            map_blif_to_json(
                bad.as_ptr(),
                bad.len(),
                0,
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        };
        assert_eq!(failed, 0);

        let mut error = [0_u8; 64];
        let error_len = unsafe { last_error(error.as_mut_ptr(), error.len()) };
        let message = str::from_utf8(&error[..error_len.min(error.len() - 1)]).unwrap();
        assert!(message.contains("does not declare .outputs"));
    }
}
