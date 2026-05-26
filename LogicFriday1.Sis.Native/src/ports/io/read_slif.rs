//! Native SLIF reader for the SIS IO layer.
//!
//! The legacy reader is a small statement processor around SLIF model
//! declarations. This port keeps the file format behavior in safe owned Rust:
//! comments and semicolon-terminated statements are normalized like the C
//! scanner, models collect primary IO, equations, net aliases, calls, latches,
//! library markers, and delay attributes, and include/search directives are
//! represented explicitly or expanded through a caller-provided resolver.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

const INCLUDE_MAX: usize = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifDesign {
    pub models: BTreeMap<String, SlifModel>,
    pub first_model: Option<String>,
    pub searches: Vec<String>,
}

impl SlifDesign {
    pub fn first_model(&self) -> Option<&SlifModel> {
        self.first_model
            .as_ref()
            .and_then(|name| self.models.get(name))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifModel {
    pub name: String,
    pub library: bool,
    pub primary_inputs: Vec<String>,
    pub primary_outputs: Vec<String>,
    pub nodes: BTreeMap<String, SlifNode>,
    pub equations: Vec<SlifEquation>,
    pub nets: Vec<SlifNet>,
    pub calls: Vec<SlifCall>,
    pub latches: Vec<SlifLatch>,
    pub attributes: Vec<SlifAttribute>,
    pub ignored_commands: Vec<SlifIgnoredCommand>,
    pub depends_on: usize,
}

impl SlifModel {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            library: false,
            primary_inputs: Vec::new(),
            primary_outputs: Vec::new(),
            nodes: BTreeMap::new(),
            equations: Vec::new(),
            nets: Vec::new(),
            calls: Vec::new(),
            latches: Vec::new(),
            attributes: Vec::new(),
            ignored_commands: Vec::new(),
            depends_on: 0,
        }
    }

    fn node_mut(&mut self, signal: &SlifSignal) -> &mut SlifNode {
        let name = signal.storage_name().to_owned();
        let node = self
            .nodes
            .entry(name.clone())
            .or_insert_with(|| SlifNode::new(name));
        if signal.complemented {
            node.complement_references += 1;
        }
        node
    }

    fn add_primary_input(&mut self, name: &str, location: SourceLocation) -> SlifResult<()> {
        let signal = SlifSignal::parse(name);
        let node = self.node_mut(&signal);
        if node.kind != SlifNodeKind::Undefined {
            return Err(SlifError::new(
                location,
                format!("node {name} multiply defined"),
            ));
        }

        node.kind = SlifNodeKind::PrimaryInput;
        push_unique(&mut self.primary_inputs, signal.storage_name());
        Ok(())
    }

    fn add_primary_output(&mut self, name: &str) {
        let signal = SlifSignal::parse(name);
        self.node_mut(&signal);
        push_unique(&mut self.primary_outputs, signal.storage_name());
    }

    fn mark_defined(&mut self, signal: &SlifSignal, definition: SlifDefinition) {
        let node = self.node_mut(signal);
        node.definition = Some(definition);
        if node.kind == SlifNodeKind::Undefined {
            node.kind = SlifNodeKind::Internal;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifNode {
    pub name: String,
    pub kind: SlifNodeKind,
    pub definition: Option<SlifDefinition>,
    pub complement_references: usize,
}

impl SlifNode {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: SlifNodeKind::Undefined,
            definition: None,
            complement_references: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlifNodeKind {
    Undefined,
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlifDefinition {
    Constant(bool),
    Equation(String),
    Alias(SlifSignal),
    LatchOutput { latch: usize },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SlifSignal {
    pub name: String,
    pub complemented: bool,
}

impl SlifSignal {
    pub fn parse(value: &str) -> Self {
        let complemented = value.len() > 1 && value.ends_with('\'');
        let name = if complemented {
            value.trim_end_matches('\'').to_owned()
        } else {
            value.to_owned()
        };

        Self { name, complemented }
    }

    pub fn storage_name(&self) -> &str {
        if self.complemented {
            self.name.as_str()
        } else {
            self.name.as_str()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifEquation {
    pub output: SlifSignal,
    pub expression: String,
    pub dependencies: Vec<SlifSignal>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifNet {
    pub signals: Vec<SlifSignal>,
    pub driver: Option<SlifSignal>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifCall {
    pub model_name: String,
    pub actuals: Vec<SlifSignal>,
    pub inputs: usize,
    pub outputs: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifLatch {
    pub output: SlifSignal,
    pub input: SlifSignal,
    pub clock: SlifSignal,
    pub enable: Option<SlifSignal>,
    pub initial_value: bool,
    pub current_value: bool,
    pub edge: SlifLatchEdge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlifLatchEdge {
    Rising,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifAttribute {
    pub global: bool,
    pub name: String,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifIgnoredCommand {
    pub command: String,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    pub filename: Option<String>,
    pub line: usize,
}

impl SourceLocation {
    fn new(filename: Option<String>, line: usize) -> Self {
        Self { filename, line }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlifError {
    pub location: SourceLocation,
    pub message: String,
}

impl SlifError {
    fn new(location: SourceLocation, message: impl Into<String>) -> Self {
        Self {
            location,
            message: message.into(),
        }
    }
}

impl fmt::Display for SlifError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(filename) = &self.location.filename {
            write!(
                formatter,
                "\"{}\", line {}: {}",
                filename, self.location.line, self.message
            )
        } else {
            write!(formatter, "line {}: {}", self.location.line, self.message)
        }
    }
}

impl Error for SlifError {}

pub type SlifResult<T> = Result<T, SlifError>;

pub fn read_slif(source: &str) -> SlifResult<SlifDesign> {
    read_slif_with_resolver(source, None, |_, _| Ok(None))
}

pub fn read_slif_named(source: &str, filename: impl Into<String>) -> SlifResult<SlifDesign> {
    read_slif_with_resolver(source, Some(filename.into()), |_, _| Ok(None))
}

pub fn read_slif_with_resolver<F>(
    source: &str,
    filename: Option<String>,
    mut resolver: F,
) -> SlifResult<SlifDesign>
where
    F: FnMut(&str, bool) -> SlifResult<Option<String>>,
{
    let mut reader = StatementReader::new(source, filename);
    let mut design = SlifDesign {
        models: BTreeMap::new(),
        first_model: None,
        searches: Vec::new(),
    };
    let mut current_model = None;

    parse_loop(
        &mut reader,
        &mut design,
        &mut resolver,
        0,
        &mut current_model,
    )?;
    if current_model.is_some() {
        return Err(SlifError::new(reader.location(), "Missing .endmodel"));
    }
    Ok(design)
}

fn parse_loop<F>(
    reader: &mut StatementReader,
    design: &mut SlifDesign,
    resolver: &mut F,
    include_depth: usize,
    current_model: &mut Option<SlifModel>,
) -> SlifResult<()>
where
    F: FnMut(&str, bool) -> SlifResult<Option<String>>,
{
    while let Some(statement) = reader.next_statement()? {
        if statement.text == ";" || statement.text.is_empty() {
            return Err(SlifError::new(statement.location, "null statement"));
        }

        if let Some(command_text) = statement.text.strip_prefix('.') {
            let tokens = tokenize(command_text);
            let Some((raw_command, args)) = tokens.split_first() else {
                return Err(SlifError::new(statement.location, "null statement"));
            };
            let command = raw_command.as_str();

            match command {
                "model" => {
                    if current_model.is_some() {
                        return Err(SlifError::new(
                            statement.location,
                            ".model encountered within another .model",
                        ));
                    }
                    let name = args.first().ok_or_else(|| {
                        SlifError::new(
                            statement.location.clone(),
                            "missing name in .model construct",
                        )
                    })?;
                    if design.models.contains_key(name) {
                        return Err(SlifError::new(
                            statement.location,
                            format!("model {name} already defined"),
                        ));
                    }
                    if design.first_model.is_none() {
                        design.first_model = Some(name.clone());
                    }
                    *current_model = Some(SlifModel::new(name));
                }
                "include" => {
                    let name = args.first().ok_or_else(|| {
                        SlifError::new(
                            statement.location.clone(),
                            "no file name specified after .include",
                        )
                    })?;
                    if include_depth >= INCLUDE_MAX {
                        return Err(SlifError::new(
                            statement.location,
                            "maximum include depth exceeded",
                        ));
                    }
                    if let Some(contents) = resolver(name, false)? {
                        let mut included = StatementReader::new(&contents, Some(name.clone()));
                        parse_loop(
                            &mut included,
                            design,
                            resolver,
                            include_depth + 1,
                            current_model,
                        )?;
                    }
                }
                "search" => {
                    let name = args.first().ok_or_else(|| {
                        SlifError::new(
                            statement.location.clone(),
                            "no file name specified after .search",
                        )
                    })?;
                    push_unique(&mut design.searches, name);
                    if let Some(contents) = resolver(name, true)? {
                        let mut searched = StatementReader::new(&contents, Some(name.clone()));
                        parse_loop(
                            &mut searched,
                            design,
                            resolver,
                            include_depth + 1,
                            current_model,
                        )?;
                    }
                }
                _ => {
                    let Some(model) = current_model.as_mut() else {
                        return Err(SlifError::new(
                            statement.location,
                            format!("Illegal statement outside of .model: {}", statement.text),
                        ));
                    };
                    parse_model_command(model, command, args, reader, statement.location)?;
                    if command == "endmodel" {
                        let finished = current_model.take().expect("model");
                        design.models.insert(finished.name.clone(), finished);
                    }
                }
            }
        } else {
            let Some(model) = current_model.as_mut() else {
                return Err(SlifError::new(
                    statement.location,
                    format!("Illegal statement outside of .model: {}", statement.text),
                ));
            };
            parse_model_body_statement(model, &statement)?;
        }
    }

    Ok(())
}

fn parse_model_command(
    model: &mut SlifModel,
    command: &str,
    args: &[String],
    reader: &mut StatementReader,
    location: SourceLocation,
) -> SlifResult<()> {
    match command {
        "attribute" | "global_attribute" => {
            let Some(name) = args.first() else {
                return Err(SlifError::new(location, "no attribute type specified"));
            };
            model.attributes.push(SlifAttribute {
                global: command == "global_attribute",
                name: name.clone(),
                arguments: args.iter().skip(1).cloned().collect(),
            });
        }
        "call" => {
            let call = parse_call(args, reader, location)?;
            for actual in &call.actuals {
                model.node_mut(actual);
            }
            model.depends_on += 1;
            model.calls.push(call);
        }
        "endmodel" => {
            if model.library && model.primary_outputs.len() != 1 {
                return Err(SlifError::new(
                    location,
                    "library module can only have 1 primary output",
                ));
            }
        }
        "inouts" => {
            return Err(SlifError::new(location, "inouts not allowed"));
        }
        "inputs" => {
            for input in args {
                model.add_primary_input(input, location.clone())?;
            }
        }
        "library" => {
            model.library = true;
        }
        "net" => {
            let signals = args
                .iter()
                .map(|arg| SlifSignal::parse(arg))
                .collect::<Vec<_>>();
            let driver = signals
                .iter()
                .find(|signal| {
                    model
                        .nodes
                        .get(signal.storage_name())
                        .and_then(|node| node.definition.as_ref())
                        .is_some()
                })
                .cloned()
                .or_else(|| signals.first().cloned());
            for signal in &signals {
                model.node_mut(signal);
            }
            if let Some(driver) = &driver {
                for signal in &signals {
                    if signal != driver {
                        model.mark_defined(signal, SlifDefinition::Alias(driver.clone()));
                    }
                }
            }
            model.nets.push(SlifNet { signals, driver });
        }
        "outputs" => {
            for output in args {
                model.add_primary_output(output);
            }
        }
        _ => {
            model.ignored_commands.push(SlifIgnoredCommand {
                command: command.to_owned(),
                arguments: args.to_vec(),
            });
        }
    }

    Ok(())
}

fn parse_model_body_statement(model: &mut SlifModel, statement: &Statement) -> SlifResult<()> {
    if statement.text.contains('@') {
        let latch = parse_latch(&statement.text, statement.location.clone())?;
        let latch_index = model.latches.len();
        model.node_mut(&latch.input);
        model.node_mut(&latch.clock);
        if let Some(enable) = &latch.enable {
            model.node_mut(enable);
        }
        model.mark_defined(
            &latch.output,
            SlifDefinition::LatchOutput { latch: latch_index },
        );
        if let Some(node) = model.nodes.get_mut(latch.output.storage_name()) {
            node.kind = SlifNodeKind::PrimaryInput;
        }
        model.latches.push(latch);
        return Ok(());
    }

    let equation = parse_equation(&statement.text, statement.location.clone())?;
    for signal in &equation.dependencies {
        model.node_mut(signal);
    }
    model.mark_defined(
        &equation.output,
        match equation.expression.as_str() {
            "0" => SlifDefinition::Constant(false),
            "1" => SlifDefinition::Constant(true),
            _ => SlifDefinition::Equation(equation.expression.clone()),
        },
    );
    model.equations.push(equation);
    Ok(())
}

fn parse_call(
    args: &[String],
    reader: &mut StatementReader,
    location: SourceLocation,
) -> SlifResult<SlifCall> {
    if args.is_empty() {
        return Err(SlifError::new(location, "missing model name for call"));
    }

    let mut model_name = args[0].clone();
    let mut start = 0;
    for (index, token) in args.iter().enumerate() {
        if token == "(" {
            if index == 0 {
                return Err(SlifError::new(location, "missing model name for call"));
            }
            model_name = args[index - 1].clone();
            start = index + 1;
            break;
        }
    }

    let mut actuals = Vec::new();
    let inputs = read_node_list(&args[start..], &mut actuals);
    let inout_statement = reader
        .next_statement()?
        .ok_or_else(|| SlifError::new(location.clone(), "ran out of arguments in .call"))?;
    let inout_tokens = tokenize(&inout_statement.text);
    let inouts = read_node_list(&inout_tokens, &mut actuals);
    if inouts != 0 {
        return Err(SlifError::new(
            inout_statement.location,
            "no inouts allowed",
        ));
    }

    let output_statement = reader
        .next_statement()?
        .ok_or_else(|| SlifError::new(location, "ran out of arguments in .call"))?;
    let output_tokens = tokenize(&output_statement.text);
    let before_outputs = actuals.len();
    read_node_list(&output_tokens, &mut actuals);
    let outputs = actuals.len() - before_outputs;

    Ok(SlifCall {
        model_name,
        actuals,
        inputs,
        outputs,
    })
}

fn read_node_list(tokens: &[String], actuals: &mut Vec<SlifSignal>) -> usize {
    let mut count = 0;
    for token in tokens {
        if token == ")" || token == ";" {
            break;
        }
        if token == "," || token == "(" {
            continue;
        }
        actuals.push(SlifSignal::parse(token));
        count += 1;
    }

    count
}

fn parse_latch(text: &str, location: SourceLocation) -> SlifResult<SlifLatch> {
    let equals = text
        .find('=')
        .ok_or_else(|| SlifError::new(location.clone(), "bad format for latch"))?;
    let output = text[..equals].trim();
    let rest = text[equals + 1..].trim();
    let at = rest
        .find('@')
        .ok_or_else(|| SlifError::new(location.clone(), "bad format for latch"))?;
    let kind = rest[at + 1..].chars().next().unwrap_or('\0');
    if kind == 'T' {
        return Err(SlifError::new(location, "T flip flop not supported"));
    }
    if kind != 'D' {
        return Err(SlifError::new(
            location,
            format!("unknown flip flop: {}", &rest[at + 1..]),
        ));
    }

    let open = rest
        .find('(')
        .ok_or_else(|| SlifError::new(location.clone(), "bad format for latch"))?;
    let close = rest
        .rfind(')')
        .ok_or_else(|| SlifError::new(location.clone(), "bad format for latch"))?;
    if close <= open {
        return Err(SlifError::new(location, "bad format for latch"));
    }

    let tokens = tokenize(&rest[open + 1..close]);
    let signals = tokens
        .iter()
        .filter(|token| token.as_str() != ",")
        .map(|token| SlifSignal::parse(token))
        .collect::<Vec<_>>();
    if signals.len() < 2 || signals.len() > 3 {
        return Err(SlifError::new(
            location,
            "invalid number of inputs to D flip flop",
        ));
    }

    Ok(SlifLatch {
        output: SlifSignal::parse(output),
        input: signals[0].clone(),
        clock: signals[1].clone(),
        enable: signals.get(2).cloned(),
        initial_value: false,
        current_value: false,
        edge: SlifLatchEdge::Rising,
    })
}

fn parse_equation(text: &str, location: SourceLocation) -> SlifResult<SlifEquation> {
    let line = text.trim_end_matches(';').trim();
    let Some((left, right)) = line.split_once('=') else {
        return Err(SlifError::new(location, format!("{text}")));
    };
    let output = SlifSignal::parse(left.trim());
    let expression = right.trim().to_owned();
    let dependencies = expression_dependencies(&expression, output.storage_name());

    Ok(SlifEquation {
        output,
        expression,
        dependencies,
    })
}

fn expression_dependencies(expression: &str, output_name: &str) -> Vec<SlifSignal> {
    let mut dependencies = BTreeSet::new();
    let mut token = String::new();
    for item in expression.chars() {
        if item.is_ascii_alphanumeric() || item == '_' || item == '\'' || item == '[' || item == ']'
        {
            token.push(item);
        } else {
            insert_dependency(&mut dependencies, &mut token, output_name);
        }
    }
    insert_dependency(&mut dependencies, &mut token, output_name);
    dependencies.into_iter().collect()
}

fn insert_dependency(
    dependencies: &mut BTreeSet<SlifSignal>,
    token: &mut String,
    output_name: &str,
) {
    if token.is_empty() {
        return;
    }
    if token != "0" && token != "1" && token != output_name {
        dependencies.insert(SlifSignal::parse(token));
    }
    token.clear();
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Statement {
    text: String,
    location: SourceLocation,
}

struct StatementReader<'a> {
    input: std::str::Chars<'a>,
    filename: Option<String>,
    line: usize,
}

impl<'a> StatementReader<'a> {
    fn new(input: &'a str, filename: Option<String>) -> Self {
        Self {
            input: input.chars(),
            filename,
            line: 1,
        }
    }

    fn location(&self) -> SourceLocation {
        SourceLocation::new(self.filename.clone(), self.line)
    }

    fn next_statement(&mut self) -> SlifResult<Option<Statement>> {
        let start = self.location();
        let mut text = String::new();
        let mut last = ' ';

        while let Some(mut item) = self.input.next() {
            if item == '#' {
                for comment in self.input.by_ref() {
                    if comment == '\n' {
                        self.line += 1;
                        item = ' ';
                        break;
                    }
                }
            }

            if item.is_whitespace() {
                if item == '\n' {
                    self.line += 1;
                }
                if last == ' ' {
                    continue;
                }
                item = ' ';
            } else if item == ';' {
                if text.starts_with('.') {
                    if last == ' ' {
                        text.pop();
                    }
                } else {
                    text.push(';');
                }
                return Ok(Some(Statement {
                    text,
                    location: start,
                }));
            } else if item == '(' || item == ')' || item == ',' {
                if last != ' ' {
                    text.push(' ');
                }
                text.push(item);
                text.push(' ');
                last = ' ';
                continue;
            }

            text.push(item);
            last = item;
        }

        if text.trim().is_empty() {
            Ok(None)
        } else {
            Err(SlifError::new(start, "incomplete last line"))
        }
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|item| !item.is_empty() && *item != ",")
        .map(ToOwned::to_owned)
        .collect()
}

fn push_unique(items: &mut Vec<String>, value: &str) {
    if !items.iter().any(|item| item == value) {
        items.push(value.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> SlifDesign {
        read_slif(source).expect("valid slif")
    }

    #[test]
    fn statement_reader_collapses_whitespace_comments_and_delimiters() {
        let source = ".model demo; # ignored\n.inputs a,b c;\ny = a*b + c;\n.endmodel demo;";
        let mut reader = StatementReader::new(source, Some("demo.slif".to_owned()));

        assert_eq!(
            reader.next_statement().unwrap().unwrap().text,
            ".model demo"
        );
        assert_eq!(
            reader.next_statement().unwrap().unwrap().text,
            ".inputs a , b c"
        );
        assert_eq!(
            reader.next_statement().unwrap().unwrap().text,
            "y = a*b + c;"
        );
        assert_eq!(
            reader.next_statement().unwrap().unwrap().text,
            ".endmodel demo"
        );
        assert!(reader.next_statement().unwrap().is_none());
    }

    #[test]
    fn parses_model_primary_io_and_equations() {
        let design = parse(
            ".model demo;
             .inputs a b;
             .outputs y;
             y = a * b';
             .endmodel demo;",
        );
        let model = design.first_model().unwrap();

        assert_eq!(model.name, "demo");
        assert_eq!(model.primary_inputs, vec!["a", "b"]);
        assert_eq!(model.primary_outputs, vec!["y"]);
        assert_eq!(model.equations.len(), 1);
        assert_eq!(
            model.equations[0].dependencies,
            vec![SlifSignal::parse("a"), SlifSignal::parse("b'")]
        );
        assert_eq!(
            model.nodes.get("y").unwrap().definition,
            Some(SlifDefinition::Equation("a * b'".to_owned()))
        );
    }

    #[test]
    fn parses_constants_as_node_definitions() {
        let design = parse(
            ".model demo;
             .outputs z o;
             z = 0;
             o = 1;
             .endmodel demo;",
        );
        let model = design.first_model().unwrap();

        assert_eq!(
            model.nodes.get("z").unwrap().definition,
            Some(SlifDefinition::Constant(false))
        );
        assert_eq!(
            model.nodes.get("o").unwrap().definition,
            Some(SlifDefinition::Constant(true))
        );
    }

    #[test]
    fn parses_net_aliases_and_prefers_existing_driver() {
        let design = parse(
            ".model demo;
             a = 1;
             .net b a c;
             .endmodel demo;",
        );
        let model = design.first_model().unwrap();

        assert_eq!(model.nets[0].driver, Some(SlifSignal::parse("a")));
        assert_eq!(
            model.nodes.get("b").unwrap().definition,
            Some(SlifDefinition::Alias(SlifSignal::parse("a")))
        );
        assert_eq!(
            model.nodes.get("c").unwrap().definition,
            Some(SlifDefinition::Alias(SlifSignal::parse("a")))
        );
    }

    #[test]
    fn parses_three_statement_call_without_inouts() {
        let design = parse(
            ".model top;
             .call NAND2 gate0 ( a , b;
             ;
             y );
             .endmodel top;",
        );
        let model = design.first_model().unwrap();

        assert_eq!(model.depends_on, 1);
        assert_eq!(model.calls.len(), 1);
        assert_eq!(model.calls[0].model_name, "gate0");
        assert_eq!(model.calls[0].inputs, 2);
        assert_eq!(model.calls[0].outputs, 1);
        assert_eq!(
            model.calls[0].actuals,
            vec![
                SlifSignal::parse("a"),
                SlifSignal::parse("b"),
                SlifSignal::parse("y")
            ]
        );
    }

    #[test]
    fn rejects_call_inouts() {
        let error = read_slif(
            ".model top;
             .call gate ( a;
             carry;
             y );
             .endmodel top;",
        )
        .unwrap_err();

        assert_eq!(error.message, "no inouts allowed");
    }

    #[test]
    fn parses_d_latch_with_enable_and_marks_output_as_primary_input() {
        let design = parse(
            ".model seq;
             q = @D(d, clk, en);
             .endmodel seq;",
        );
        let model = design.first_model().unwrap();

        assert_eq!(model.latches.len(), 1);
        assert_eq!(model.latches[0].output, SlifSignal::parse("q"));
        assert_eq!(model.latches[0].input, SlifSignal::parse("d"));
        assert_eq!(model.latches[0].clock, SlifSignal::parse("clk"));
        assert_eq!(model.latches[0].enable, Some(SlifSignal::parse("en")));
        assert_eq!(
            model.nodes.get("q").unwrap().kind,
            SlifNodeKind::PrimaryInput
        );
        assert_eq!(
            model.nodes.get("q").unwrap().definition,
            Some(SlifDefinition::LatchOutput { latch: 0 })
        );
    }

    #[test]
    fn rejects_t_latches() {
        let error = read_slif(
            ".model seq;
             q = @T(d, clk);
             .endmodel seq;",
        )
        .unwrap_err();

        assert_eq!(error.message, "T flip flop not supported");
    }

    #[test]
    fn stores_attributes_library_and_ignored_commands() {
        let design = parse(
            ".model inv;
             .library;
             .inputs a;
             .outputs y;
             y = a';
             .global_attribute wire_load_slope 1.5;
             .attribute delay a INV 1 999 1 .2 1 .2;
             .unknown value;
             .endmodel inv;",
        );
        let model = design.first_model().unwrap();

        assert!(model.library);
        assert_eq!(model.attributes.len(), 2);
        assert!(model.attributes[0].global);
        assert_eq!(model.attributes[0].name, "wire_load_slope");
        assert_eq!(model.ignored_commands[0].command, "unknown");
    }

    #[test]
    fn rejects_library_model_with_multiple_outputs() {
        let error = read_slif(
            ".model bad;
             .library;
             .outputs a b;
             .endmodel bad;",
        )
        .unwrap_err();

        assert_eq!(
            error.message,
            "library module can only have 1 primary output"
        );
    }

    #[test]
    fn expands_includes_with_resolver() {
        let design = read_slif_with_resolver(
            ".include child;",
            Some("root.slif".to_owned()),
            |name, _search| {
                assert_eq!(name, "child");
                Ok(Some(
                    ".model child; .outputs y; y = 1; .endmodel child;".to_owned(),
                ))
            },
        )
        .unwrap();

        assert!(design.models.contains_key("child"));
        assert_eq!(design.first_model.as_deref(), Some("child"));
    }

    #[test]
    fn includes_are_parsed_inline_inside_current_model() {
        let design = read_slif_with_resolver(
            ".model top; .include body; .endmodel top;",
            Some("root.slif".to_owned()),
            |name, _search| {
                assert_eq!(name, "body");
                Ok(Some(".inputs a; .outputs y; y = a;".to_owned()))
            },
        )
        .unwrap();
        let model = design.first_model().unwrap();

        assert_eq!(model.name, "top");
        assert_eq!(model.primary_inputs, vec!["a"]);
        assert_eq!(model.primary_outputs, vec!["y"]);
        assert_eq!(model.equations.len(), 1);
    }

    #[test]
    fn records_searches_when_resolver_does_not_expand() {
        let design = parse(".search library.slif; .model top; .endmodel top;");

        assert_eq!(design.searches, vec!["library.slif"]);
    }

    #[test]
    fn reports_illegal_statement_outside_model() {
        let error = read_slif("y = a;").unwrap_err();

        assert_eq!(error.message, "Illegal statement outside of .model: y = a;");
    }

    #[test]
    fn reports_incomplete_last_statement() {
        let error = read_slif(".model missing").unwrap_err();

        assert_eq!(error.message, "incomplete last line");
    }
}
