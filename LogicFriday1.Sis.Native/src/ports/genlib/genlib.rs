//! Native Rust conversion support for `sis/genlib/genlib.c`.
//!
//! The C implementation owns the final step of genlib parsing: it validates a
//! parsed function tree, converts that tree to NAND or NOR BLIF, and emits
//! delay/latch timing records. This module keeps that behavior as owned Rust
//! data and text-producing APIs without exposing legacy per-file C ABI symbols.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateForm {
    Nand,
    Nor,
}

impl GateForm {
    fn node_type(self) -> GenlibNodeType {
        match self {
            Self::Nand => GenlibNodeType::Nand,
            Self::Nor => GenlibNodeType::Nor,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Nand => "NAND",
            Self::Nor => "NOR",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenlibNodeType {
    Or,
    And,
    Nor,
    Nand,
    Zero,
    One,
    Leaf,
}

impl GenlibNodeType {
    fn reverse(self) -> Self {
        match self {
            Self::Or => Self::And,
            Self::And => Self::Or,
            value => value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenlibTree {
    pub node_type: GenlibNodeType,
    pub phase: bool,
    pub name: Option<String>,
    pub sons: Vec<GenlibTree>,
}

impl GenlibTree {
    pub fn leaf(name: impl Into<String>) -> Self {
        Self {
            node_type: GenlibNodeType::Leaf,
            phase: true,
            name: Some(name.into()),
            sons: Vec::new(),
        }
    }

    pub fn inverted_leaf(name: impl Into<String>) -> Self {
        Self {
            phase: false,
            ..Self::leaf(name)
        }
    }

    pub fn zero() -> Self {
        Self {
            node_type: GenlibNodeType::Zero,
            phase: true,
            name: None,
            sons: Vec::new(),
        }
    }

    pub fn one() -> Self {
        Self {
            node_type: GenlibNodeType::One,
            phase: true,
            name: None,
            sons: Vec::new(),
        }
    }

    pub fn and(sons: Vec<Self>) -> Self {
        Self::branch(GenlibNodeType::And, sons)
    }

    pub fn or(sons: Vec<Self>) -> Self {
        Self::branch(GenlibNodeType::Or, sons)
    }

    pub fn branch(node_type: GenlibNodeType, sons: Vec<Self>) -> Self {
        Self {
            node_type,
            phase: true,
            name: None,
            sons,
        }
    }

    pub fn with_phase(mut self, phase: bool) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn is_leaf(&self) -> bool {
        self.sons.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenlibFunction {
    pub name: String,
    pub tree: GenlibTree,
}

impl GenlibFunction {
    pub fn new(name: impl Into<String>, tree: GenlibTree) -> Self {
        Self {
            name: name.into(),
            tree,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinInfo {
    pub name: String,
    pub phase: String,
    pub values: [f64; 6],
}

impl PinInfo {
    pub fn new(name: impl Into<String>, phase: impl Into<String>, values: [f64; 6]) -> Self {
        Self {
            name: name.into(),
            phase: phase.into(),
            values,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatchInfo {
    pub input: String,
    pub output: String,
    pub latch_type: String,
}

impl LatchInfo {
    pub fn new(
        input: impl Into<String>,
        output: impl Into<String>,
        latch_type: impl Into<String>,
    ) -> Self {
        Self {
            input: input.into(),
            output: output.into(),
            latch_type: latch_type.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintInfo {
    pub name: String,
    pub setup: f64,
    pub hold: f64,
}

impl ConstraintInfo {
    pub fn new(name: impl Into<String>, setup: f64, hold: f64) -> Self {
        Self {
            name: name.into(),
            setup,
            hold,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GateConversion {
    pub model_name: String,
    pub area: f64,
    pub function: GenlibFunction,
    pub pins: Vec<PinInfo>,
    pub form: GateForm,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatchConversion {
    pub model_name: String,
    pub area: f64,
    pub function: GenlibFunction,
    pub pins: Vec<PinInfo>,
    pub form: GateForm,
    pub latch: LatchInfo,
    pub clock: Option<PinInfo>,
    pub constraints: Vec<ConstraintInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenlibConvertError {
    InternalConstant,
    MissingLeafName,
    InvalidModelName,
    InvalidFunctionName,
    InvalidArea,
    InvalidPinTiming { name: String },
    InvalidClockTiming { name: String },
    InvalidConstraintTiming { name: String },
    ImproperWildcard,
    PinNotFound { name: String },
    ConstraintNotFound { name: String },
    MissingSynchronousClock,
    InvalidLatch,
}

impl fmt::Display for GenlibConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InternalConstant => {
                write!(f, "0 and 1 are allowed only at the root of a function")
            }
            Self::MissingLeafName => write!(f, "function leaf is missing a name"),
            Self::InvalidModelName => write!(f, "model name cannot be empty"),
            Self::InvalidFunctionName => write!(f, "function output name cannot be empty"),
            Self::InvalidArea => write!(f, "gate area must be finite and non-negative"),
            Self::InvalidPinTiming { name } => write!(f, "pin '{name}' has invalid timing values"),
            Self::InvalidClockTiming { name } => {
                write!(f, "clock '{name}' has invalid timing values")
            }
            Self::InvalidConstraintTiming { name } => {
                write!(f, "constraint '{name}' has invalid setup or hold values")
            }
            Self::ImproperWildcard => write!(f, "improper use of pin wildcard '*'"),
            Self::PinNotFound { name } => write!(f, "pin '{name}' was not found in the function"),
            Self::ConstraintNotFound { name } => {
                write!(f, "constraint '{name}' was not found in the function")
            }
            Self::MissingSynchronousClock => {
                write!(f, "no clock delay info found for synchronous latch")
            }
            Self::InvalidLatch => write!(f, "latch input, output, and type must be non-empty"),
        }
    }
}

impl Error for GenlibConvertError {}

pub fn convert_gate_to_blif(conversion: GateConversion) -> Result<String, GenlibConvertError> {
    validate_common(
        &conversion.model_name,
        conversion.area,
        &conversion.function,
        &conversion.pins,
    )?;

    let mut tree = conversion.function.tree.clone();
    check_internal_phase(&tree, 0)?;
    let original = print_tree(&tree)?;
    let leafs = sorted_leaf_names(&tree)?;
    let forms = conversion_forms(&mut tree, conversion.form);
    let mut output = String::new();

    for (index, mut form_tree) in forms.into_iter().enumerate() {
        assign_node_names(&mut form_tree);
        form_tree.name = Some(conversion.function.name.clone());

        output.push_str(&format!(
            "# ({} of 1) {}-FORM of {}\n",
            index + 1,
            conversion.form.label(),
            original
        ));
        output.push_str(&format!(".model {}\n", conversion.model_name));
        write_blif(&mut output, &form_tree, conversion.form, None)?;
        output.push_str(&format!(".area {:.2}\n", conversion.area));
        write_pin_delays(&mut output, &conversion.pins, &leafs, None)?;
        output.push_str(".end\n\n");
    }

    Ok(output)
}

pub fn convert_latch_to_blif(conversion: LatchConversion) -> Result<String, GenlibConvertError> {
    validate_common(
        &conversion.model_name,
        conversion.area,
        &conversion.function,
        &conversion.pins,
    )?;
    validate_latch(&conversion.latch)?;
    if let Some(clock) = &conversion.clock {
        validate_pin(
            clock,
            GenlibConvertError::InvalidClockTiming {
                name: clock.name.clone(),
            },
        )?;
    } else if conversion.latch.latch_type != "as" {
        return Err(GenlibConvertError::MissingSynchronousClock);
    }
    for constraint in &conversion.constraints {
        validate_constraint(constraint)?;
    }

    let mut tree = conversion.function.tree.clone();
    check_internal_phase(&tree, 0)?;
    let original = print_tree(&tree)?;
    let leafs = sorted_leaf_names(&tree)?;
    let forms = conversion_forms(&mut tree, conversion.form);
    let mut output = String::new();

    for (index, mut form_tree) in forms.into_iter().enumerate() {
        assign_node_names(&mut form_tree);
        form_tree.name = Some(conversion.function.name.clone());

        output.push_str(&format!(
            "# ({} of 1) {}-FORM of {}\n",
            index + 1,
            conversion.form.label(),
            original
        ));
        output.push_str(&format!(".model {}\n", conversion.model_name));
        write_blif(
            &mut output,
            &form_tree,
            conversion.form,
            Some(&conversion.latch),
        )?;
        output.push_str(&format!(".area {:.2}\n", conversion.area));
        write_pin_delays(
            &mut output,
            &conversion.pins,
            &leafs,
            Some(conversion.latch.output.as_str()),
        )?;
        if let Some(clock) = &conversion.clock {
            output.push_str(&format!(".clock {}\n", clock.name));
            write_delay_line(&mut output, clock);
        }
        write_constraints(
            &mut output,
            &conversion.constraints,
            &leafs,
            conversion.latch.output.as_str(),
        )?;
        output.push_str(&format!(
            ".latch {} {} {} {} 0\n",
            conversion.latch.input,
            conversion.latch.output,
            conversion.latch.latch_type,
            conversion
                .clock
                .as_ref()
                .map(|clock| clock.name.as_str())
                .unwrap_or("NIL")
        ));
        output.push_str(".end\n\n");
    }

    Ok(output)
}

pub fn check_tree_for_genlib_conversion(tree: &GenlibTree) -> Result<(), GenlibConvertError> {
    check_internal_phase(tree, 0)
}

pub fn set_tree_functions(tree: &mut GenlibTree, form: GateForm) {
    set_functions(tree, form);
}

fn validate_common(
    model_name: &str,
    area: f64,
    function: &GenlibFunction,
    pins: &[PinInfo],
) -> Result<(), GenlibConvertError> {
    if model_name.is_empty() {
        return Err(GenlibConvertError::InvalidModelName);
    }
    if function.name.is_empty() {
        return Err(GenlibConvertError::InvalidFunctionName);
    }
    if !area.is_finite() || area < 0.0 {
        return Err(GenlibConvertError::InvalidArea);
    }
    for pin in pins {
        validate_pin(
            pin,
            GenlibConvertError::InvalidPinTiming {
                name: pin.name.clone(),
            },
        )?;
    }

    Ok(())
}

fn validate_pin(pin: &PinInfo, error: GenlibConvertError) -> Result<(), GenlibConvertError> {
    if pin.name.is_empty()
        || pin.phase.is_empty()
        || pin
            .values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(error);
    }

    Ok(())
}

fn validate_constraint(constraint: &ConstraintInfo) -> Result<(), GenlibConvertError> {
    if constraint.name.is_empty()
        || !constraint.setup.is_finite()
        || !constraint.hold.is_finite()
        || constraint.setup < 0.0
        || constraint.hold < 0.0
    {
        return Err(GenlibConvertError::InvalidConstraintTiming {
            name: constraint.name.clone(),
        });
    }

    Ok(())
}

fn validate_latch(latch: &LatchInfo) -> Result<(), GenlibConvertError> {
    if latch.input.is_empty() || latch.output.is_empty() || latch.latch_type.is_empty() {
        return Err(GenlibConvertError::InvalidLatch);
    }

    Ok(())
}

fn check_internal_phase(tree: &GenlibTree, level: usize) -> Result<(), GenlibConvertError> {
    if level > 0
        && (tree.node_type == GenlibNodeType::Zero || tree.node_type == GenlibNodeType::One)
    {
        return Err(GenlibConvertError::InternalConstant);
    }

    if !tree.sons.is_empty() {
        for son in &tree.sons {
            check_internal_phase(son, level + 1)?;
        }
    }

    Ok(())
}

fn conversion_forms(tree: &mut GenlibTree, form: GateForm) -> Vec<GenlibTree> {
    if tree.node_type != GenlibNodeType::Zero && tree.node_type != GenlibNodeType::One {
        make_well_formed(tree);
    }

    let mut converted = tree.clone();
    set_functions(&mut converted, form);
    vec![converted]
}

fn set_functions(tree: &mut GenlibTree, form: GateForm) {
    for son in &mut tree.sons {
        set_functions(son, form);
    }
    if !tree.sons.is_empty() {
        tree.node_type = form.node_type();
        tree.phase = true;
    }
}

fn make_well_formed(tree: &mut GenlibTree) {
    if !tree.phase && !tree.sons.is_empty() {
        tree.phase = true;
        invert_tree(tree);
    }

    loop {
        let mut flattened = false;
        let mut index = 0usize;
        while index < tree.sons.len() {
            if !tree.sons[index].phase && !tree.sons[index].sons.is_empty() {
                tree.sons[index].phase = true;
                invert_tree(&mut tree.sons[index]);
            }
            if tree.sons[index].is_leaf() {
                tree.sons[index].node_type = tree.node_type.reverse();
            } else if tree.node_type == tree.sons[index].node_type {
                let son = tree.sons.remove(index);
                tree.sons.splice(index..index, son.sons);
                flattened = true;
                continue;
            }
            index += 1;
        }
        if !flattened {
            break;
        }
    }

    for son in &mut tree.sons {
        make_well_formed(son);
    }
}

fn invert_tree(tree: &mut GenlibTree) {
    if tree.phase {
        if tree.sons.is_empty() {
            tree.phase = false;
        } else {
            for son in &mut tree.sons {
                invert_tree(son);
            }
            tree.node_type = tree.node_type.reverse();
        }
    } else {
        tree.phase = true;
    }
}

fn write_blif(
    output: &mut String,
    tree: &GenlibTree,
    form: GateForm,
    latch: Option<&LatchInfo>,
) -> Result<(), GenlibConvertError> {
    match tree.node_type {
        GenlibNodeType::Zero => {
            let name = tree
                .name
                .as_deref()
                .ok_or(GenlibConvertError::MissingLeafName)?;
            output.push_str(&format!(".outputs {name}\n"));
            output.push_str(&format!(".names {name}\n"));
        }
        GenlibNodeType::One => {
            let name = tree
                .name
                .as_deref()
                .ok_or(GenlibConvertError::MissingLeafName)?;
            output.push_str(&format!(".outputs {name}\n"));
            output.push_str(&format!(".names {name}\n 1\n"));
        }
        _ => {
            let leafs = sorted_leaf_names(tree)?;
            output.push_str(".inputs");
            for leaf in leafs {
                if latch.is_some_and(|latch| latch.output == leaf) {
                    continue;
                }
                output.push(' ');
                output.push_str(&leaf);
            }
            output.push('\n');
            if latch.is_none() {
                let name = tree
                    .name
                    .as_deref()
                    .ok_or(GenlibConvertError::MissingLeafName)?;
                output.push_str(&format!(".outputs {name}\n"));
            }
            write_blif_tables(output, tree, form)?;
        }
    }

    Ok(())
}

fn write_blif_tables(
    output: &mut String,
    tree: &GenlibTree,
    form: GateForm,
) -> Result<(), GenlibConvertError> {
    for son in &tree.sons {
        write_blif_tables(output, son, form)?;
    }

    if !tree.sons.is_empty() {
        output.push_str(".names");
        for son in &tree.sons {
            output.push(' ');
            output.push_str(
                son.name
                    .as_deref()
                    .ok_or(GenlibConvertError::MissingLeafName)?,
            );
        }
        output.push(' ');
        output.push_str(
            tree.name
                .as_deref()
                .ok_or(GenlibConvertError::MissingLeafName)?,
        );
        output.push('\n');

        match form {
            GateForm::Nor => {
                output.push_str(&"0".repeat(tree.sons.len()));
                output.push_str(" 1\n");
            }
            GateForm::Nand => {
                for index in 0..tree.sons.len() {
                    for column in 0..tree.sons.len() {
                        output.push(if index == column { '0' } else { '-' });
                    }
                    output.push_str(" 1\n");
                }
            }
        }
    }

    Ok(())
}

fn write_pin_delays(
    output: &mut String,
    pins: &[PinInfo],
    leafs: &[String],
    skip_name: Option<&str>,
) -> Result<(), GenlibConvertError> {
    if pins.len() == 1 && pins[0].name == "*" {
        for leaf in leafs {
            if Some(leaf.as_str()) == skip_name {
                continue;
            }
            let pin = PinInfo::new(leaf, pins[0].phase.clone(), pins[0].values);
            write_delay_line(output, &pin);
        }
        return Ok(());
    }

    if pins.iter().any(|pin| pin.name == "*") {
        return Err(GenlibConvertError::ImproperWildcard);
    }

    for pin in pins {
        if !leafs.iter().any(|leaf| leaf == &pin.name) {
            return Err(GenlibConvertError::PinNotFound {
                name: pin.name.clone(),
            });
        }
        if Some(pin.name.as_str()) != skip_name {
            write_delay_line(output, pin);
        }
    }

    Ok(())
}

fn write_delay_line(output: &mut String, pin: &PinInfo) {
    output.push_str(&format!(
        ".delay {} {} {:.3} {:.3} {:.3} {:.3} {:.3} {:.3}\n",
        pin.name,
        pin.phase,
        pin.values[0],
        pin.values[1],
        pin.values[2],
        pin.values[3],
        pin.values[4],
        pin.values[5]
    ));
}

fn write_constraints(
    output: &mut String,
    constraints: &[ConstraintInfo],
    leafs: &[String],
    latch_output: &str,
) -> Result<(), GenlibConvertError> {
    if constraints.is_empty() {
        for leaf in leafs {
            if leaf != latch_output {
                output.push_str(&format!(".input_arrival {leaf} 0.0 0.0\n"));
            }
        }
        return Ok(());
    }

    if constraints.len() == 1 && constraints[0].name == "*" {
        for leaf in leafs {
            if leaf != latch_output {
                output.push_str(&format!(
                    ".input_arrival {} {:.3} {:.3}\n",
                    leaf, constraints[0].setup, constraints[0].hold
                ));
            }
        }
        return Ok(());
    }

    if constraints.iter().any(|constraint| constraint.name == "*") {
        return Err(GenlibConvertError::ImproperWildcard);
    }

    for constraint in constraints {
        if !leafs.iter().any(|leaf| leaf == &constraint.name) {
            return Err(GenlibConvertError::ConstraintNotFound {
                name: constraint.name.clone(),
            });
        }
        if constraint.name != latch_output {
            output.push_str(&format!(
                ".input_arrival {} {:.3} {:.3}\n",
                constraint.name, constraint.setup, constraint.hold
            ));
        }
    }

    Ok(())
}

fn sorted_leaf_names(tree: &GenlibTree) -> Result<Vec<String>, GenlibConvertError> {
    let mut names = BTreeSet::new();
    collect_leaf_names(tree, &mut names)?;
    Ok(names.into_iter().collect())
}

fn collect_leaf_names(
    tree: &GenlibTree,
    names: &mut BTreeSet<String>,
) -> Result<(), GenlibConvertError> {
    if tree.sons.is_empty() {
        match tree.node_type {
            GenlibNodeType::Zero | GenlibNodeType::One => {}
            _ => {
                names.insert(
                    tree.name
                        .as_ref()
                        .ok_or(GenlibConvertError::MissingLeafName)?
                        .clone(),
                );
            }
        }
        return Ok(());
    }

    for son in &tree.sons {
        collect_leaf_names(son, names)?;
    }

    Ok(())
}

fn assign_node_names(tree: &mut GenlibTree) {
    let mut count = 0usize;
    assign_node_names_recur(tree, &mut count);
}

fn assign_node_names_recur(tree: &mut GenlibTree, count: &mut usize) {
    for son in &mut tree.sons {
        assign_node_names_recur(son, count);
    }
    if tree.name.is_none() {
        tree.name = Some(format!("_{count}"));
        *count += 1;
    }
}

fn print_tree(tree: &GenlibTree) -> Result<String, GenlibConvertError> {
    if tree.phase || tree.sons.is_empty() {
        print_tree_recur(tree, 0)
    } else {
        Ok(format!("({})'", print_tree_recur(tree, 0)?))
    }
}

fn print_tree_recur(tree: &GenlibTree, level: usize) -> Result<String, GenlibConvertError> {
    if tree.sons.is_empty() {
        let name = match tree.node_type {
            GenlibNodeType::Zero => "0",
            GenlibNodeType::One => "1",
            _ => tree
                .name
                .as_deref()
                .ok_or(GenlibConvertError::MissingLeafName)?,
        };
        return Ok(format!("{}{name}", if tree.phase { "" } else { "!" }));
    }

    let mut text = String::new();
    if tree.node_type == GenlibNodeType::Or && level > 0 {
        text.push('(');
    }
    for (index, son) in tree.sons.iter().enumerate() {
        text.push_str(&print_tree_recur(son, level + 1)?);
        if tree.node_type == GenlibNodeType::Or && index + 1 != tree.sons.len() {
            text.push('+');
        }
    }
    if tree.node_type == GenlibNodeType::Or && level > 0 {
        text.push(')');
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(name: &str) -> PinInfo {
        PinInfo::new(name, "INV", [1.0, 999.0, 1.0, 0.2, 1.5, 0.3])
    }

    fn sample_gate() -> GateConversion {
        GateConversion {
            model_name: "nand2".to_string(),
            area: 2.0,
            function: GenlibFunction::new(
                "O",
                GenlibTree::and(vec![GenlibTree::leaf("a"), GenlibTree::leaf("b")]),
            ),
            pins: vec![pin("*")],
            form: GateForm::Nand,
        }
    }

    #[test]
    fn converts_combinational_gate_to_blif_with_sorted_wildcard_delays() {
        let blif = convert_gate_to_blif(sample_gate()).unwrap();

        assert!(blif.contains("# (1 of 1) NAND-FORM of ab\n"));
        assert!(blif.contains(".model nand2\n"));
        assert!(blif.contains(".inputs a b\n"));
        assert!(blif.contains(".outputs O\n"));
        assert!(blif.contains(".names a b O\n"));
        assert!(blif.contains("0- 1\n"));
        assert!(blif.contains("-0 1\n"));
        assert!(blif.contains(".area 2.00\n"));
        assert!(blif.contains(".delay a INV 1.000 999.000 1.000 0.200 1.500 0.300\n"));
        assert!(blif.contains(".delay b INV 1.000 999.000 1.000 0.200 1.500 0.300\n"));
        assert!(blif.ends_with(".end\n\n"));
    }

    #[test]
    fn converts_constants_without_inputs() {
        let mut conversion = sample_gate();
        conversion.function.tree = GenlibTree::one();
        conversion.pins.clear();

        let blif = convert_gate_to_blif(conversion).unwrap();

        assert!(blif.contains(".outputs O\n"));
        assert!(blif.contains(".names O\n 1\n"));
        assert!(!blif.contains(".inputs"));
    }

    #[test]
    fn rejects_improper_wildcard_and_missing_pin() {
        let mut conversion = sample_gate();
        conversion.pins = vec![pin("*"), pin("a")];

        assert_eq!(
            convert_gate_to_blif(conversion).unwrap_err(),
            GenlibConvertError::ImproperWildcard
        );

        let mut conversion = sample_gate();
        conversion.pins = vec![pin("missing")];

        assert_eq!(
            convert_gate_to_blif(conversion).unwrap_err(),
            GenlibConvertError::PinNotFound {
                name: "missing".to_string()
            }
        );
    }

    #[test]
    fn rejects_constants_below_function_root() {
        let mut conversion = sample_gate();
        conversion.function.tree = GenlibTree::or(vec![GenlibTree::one(), GenlibTree::leaf("a")]);

        assert_eq!(
            convert_gate_to_blif(conversion).unwrap_err(),
            GenlibConvertError::InternalConstant
        );
    }

    #[test]
    fn latch_conversion_skips_latch_output_timing_and_writes_clock() {
        let conversion = LatchConversion {
            model_name: "dff".to_string(),
            area: 5.0,
            function: GenlibFunction::new(
                "Qnext",
                GenlibTree::and(vec![GenlibTree::leaf("D"), GenlibTree::leaf("Q")]),
            ),
            pins: vec![pin("*")],
            form: GateForm::Nor,
            latch: LatchInfo::new("D", "Q", "re"),
            clock: Some(PinInfo::new(
                "clk",
                "NONINV",
                [0.0, 0.0, 0.4, 0.1, 0.5, 0.1],
            )),
            constraints: vec![ConstraintInfo::new("*", 0.7, 0.2)],
        };

        let blif = convert_latch_to_blif(conversion).unwrap();

        assert!(blif.contains(".inputs D\n"));
        assert!(!blif.contains(".outputs Qnext\n"));
        assert!(blif.contains(".delay D INV 1.000 999.000 1.000 0.200 1.500 0.300\n"));
        assert!(!blif.contains(".delay Q INV"));
        assert!(blif.contains(".clock clk\n"));
        assert!(blif.contains(".delay clk NONINV 0.000 0.000 0.400 0.100 0.500 0.100\n"));
        assert!(blif.contains(".input_arrival D 0.700 0.200\n"));
        assert!(!blif.contains(".input_arrival Q"));
        assert!(blif.contains(".latch D Q re clk 0\n"));
    }

    #[test]
    fn latch_requires_clock_unless_asynchronous() {
        let conversion = LatchConversion {
            model_name: "dff".to_string(),
            area: 5.0,
            function: GenlibFunction::new("Qnext", GenlibTree::leaf("D")),
            pins: vec![pin("D")],
            form: GateForm::Nand,
            latch: LatchInfo::new("D", "Q", "re"),
            clock: None,
            constraints: Vec::new(),
        };

        assert_eq!(
            convert_latch_to_blif(conversion).unwrap_err(),
            GenlibConvertError::MissingSynchronousClock
        );
    }

    #[test]
    fn asynchronous_latch_uses_nil_clock_and_default_constraints() {
        let conversion = LatchConversion {
            model_name: "alat".to_string(),
            area: 3.0,
            function: GenlibFunction::new("Qnext", GenlibTree::leaf("D")),
            pins: vec![pin("D")],
            form: GateForm::Nand,
            latch: LatchInfo::new("D", "Q", "as"),
            clock: None,
            constraints: Vec::new(),
        };

        let blif = convert_latch_to_blif(conversion).unwrap();

        assert!(blif.contains(".input_arrival D 0.0 0.0\n"));
        assert!(blif.contains(".latch D Q as NIL 0\n"));
    }

    #[test]
    fn set_tree_functions_marks_internal_nodes_only() {
        let mut tree = GenlibTree::or(vec![GenlibTree::leaf("a"), GenlibTree::leaf("b")]);

        set_tree_functions(&mut tree, GateForm::Nor);

        assert_eq!(tree.node_type, GenlibNodeType::Nor);
        assert_eq!(tree.sons[0].node_type, GenlibNodeType::Leaf);
    }

    #[test]
    fn source_does_not_contain_dependency_metadata_or_c_abi_exports() {
        let source = include_str!("genlib.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
