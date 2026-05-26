//! Native ASTG text reader and writer.
//!
//! This module ports the STG text syntax handled by the SIS `astg_read.c`
//! unit into an idiomatic Rust data model. It deliberately exposes native Rust
//! APIs only; higher-level integration can adapt these structures when the rest
//! of the ASTG graph package is available.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalType
{
    Input,
    Output,
    Internal,
    Dummy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionKind
{
    Positive,
    Negative,
    Toggle,
    Hatch,
    Dummy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal
{
    pub name: String,
    pub signal_type: SignalType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transition
{
    pub name: String,
    pub signal: String,
    pub kind: TransitionKind,
    pub copy_number: usize,
    pub delay: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Place
{
    pub name: String,
    pub user_named: bool,
    pub initial_token: bool,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge
{
    pub from: VertexRef,
    pub to: VertexRef,
    pub guard: Option<String>,
    pub spline_points: Vec<f64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VertexRef
{
    Place(usize),
    Transition(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AstgGraph
{
    pub name: String,
    pub filename: Option<String>,
    pub comments: Vec<String>,
    pub signals: Vec<Signal>,
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub edges: Vec<Edge>,
    pub has_marking: bool,
}

impl AstgGraph
{
    pub fn new(name: impl Into<String>) -> Self
    {
        Self {
            name: name.into(),
            filename: None,
            comments: Vec::new(),
            signals: Vec::new(),
            places: Vec::new(),
            transitions: Vec::new(),
            edges: Vec::new(),
            has_marking: false,
        }
    }

    pub fn signal(&self, name: &str) -> Option<&Signal>
    {
        self.signals.iter().find(|signal| signal.name == name)
    }

    pub fn transition(&self, name: &str) -> Option<&Transition>
    {
        self.transitions
            .iter()
            .find(|transition| transition.name == name)
    }

    pub fn place(&self, name: &str) -> Option<&Place>
    {
        self.places.iter().find(|place| place.name == name)
    }

    pub fn outgoing_edges(&self, vertex: VertexRef) -> impl Iterator<Item = &Edge>
    {
        self.edges.iter().filter(move |edge| edge.from == vertex)
    }

    pub fn incoming_edges(&self, vertex: VertexRef) -> impl Iterator<Item = &Edge>
    {
        self.edges.iter().filter(move |edge| edge.to == vertex)
    }

    fn find_or_create_signal(&mut self, name: &str, signal_type: SignalType) -> usize
    {
        if let Some(index) = self.signals.iter().position(|signal| signal.name == name) {
            return index;
        }

        self.signals.push(Signal {
            name: name.to_owned(),
            signal_type,
        });
        self.signals.len() - 1
    }

    fn find_or_create_place(&mut self, name: Option<&str>) -> usize
    {
        if let Some(name) = name {
            if let Some(index) = self.places.iter().position(|place| place.name == name) {
                return index;
            }
        }

        let user_named = name.is_some();
        let name = name
            .map(str::to_owned)
            .unwrap_or_else(|| self.next_implicit_place_name());
        self.places.push(Place {
            name,
            user_named,
            initial_token: false,
            x: 0.0,
            y: 0.0,
        });
        self.places.len() - 1
    }

    fn next_implicit_place_name(&self) -> String
    {
        let existing = self
            .places
            .iter()
            .map(|place| place.name.as_str())
            .collect::<BTreeSet<_>>();
        let mut number = 0;
        loop {
            let name = format!("p{number}");
            if !existing.contains(name.as_str()) {
                return name;
            }

            number += 1;
        }
    }

    fn find_or_create_transition(
        &mut self,
        signal_name: &str,
        kind: TransitionKind,
        copy_number: usize,
        create_signal: bool,
    ) -> Result<usize, AstgReadError>
    {
        let name = make_transition_name(signal_name, kind, copy_number);
        if let Some(index) = self
            .transitions
            .iter()
            .position(|transition| transition.name == name)
        {
            return Ok(index);
        }

        if self.signal(signal_name).is_none() {
            if create_signal {
                self.find_or_create_signal(signal_name, SignalType::Dummy);
            } else {
                return Err(AstgReadError::NoSuchSignal(signal_name.to_owned()));
            }
        }

        self.transitions.push(Transition {
            name,
            signal: signal_name.to_owned(),
            kind,
            copy_number,
            delay: 0.0,
            x: 0.0,
            y: 0.0,
        });
        Ok(self.transitions.len() - 1)
    }

    fn add_edge(
        &mut self,
        from: VertexRef,
        to: VertexRef,
        source: &mut InputSource,
    ) -> Option<usize>
    {
        if self
            .edges
            .iter()
            .any(|edge| edge.from == from && edge.to == to)
        {
            source.warn("Repeated edge is ignored.");
            return None;
        }

        self.edges.push(Edge {
            from,
            to,
            guard: None,
            spline_points: Vec::new(),
        });
        Some(self.edges.len() - 1)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgParseDiagnostic
{
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgReadError
{
    Parse(AstgParseDiagnostic),
    NoSuchSignal(String),
    NoSuchTransition(String),
}

impl fmt::Display for AstgReadError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::Parse(diagnostic) => {
                write!(
                    formatter,
                    "{} at line {}, column {}",
                    diagnostic.message, diagnostic.line, diagnostic.column
                )
            }
            Self::NoSuchSignal(name) => write!(formatter, "no such signal: {name}"),
            Self::NoSuchTransition(name) => write!(formatter, "no such transition: {name}"),
        }
    }
}

impl Error for AstgReadError {}

#[derive(Clone, Debug, PartialEq)]
pub struct AstgWriteOptions
{
    pub hide_places: bool,
}

impl Default for AstgWriteOptions
{
    fn default() -> Self
    {
        Self { hide_places: false }
    }
}

pub fn make_transition_name(signal_name: &str, kind: TransitionKind, copy_number: usize) -> String
{
    let suffix = match kind {
        TransitionKind::Positive => "+",
        TransitionKind::Negative => "-",
        TransitionKind::Toggle => "~",
        TransitionKind::Hatch => "*",
        TransitionKind::Dummy => "",
    };

    if copy_number > 0 {
        format!("{signal_name}{suffix}/{copy_number}")
    } else {
        format!("{signal_name}{suffix}")
    }
}

pub fn parse_transition_name(name: &str) -> Result<(String, TransitionKind, usize), AstgReadError>
{
    let (base, copy_number) = if let Some((left, right)) = name.rsplit_once('/') {
        let copy_number = right.parse::<usize>().map_err(|_| {
            AstgReadError::Parse(AstgParseDiagnostic {
                line: 0,
                column: 0,
                message: "unrecognizable copy number".to_owned(),
            })
        })?;
        (left, copy_number)
    } else {
        (name, 0)
    };

    if let Some(signal_name) = base.strip_suffix('+') {
        Ok((
            signal_name.to_owned(),
            TransitionKind::Positive,
            copy_number,
        ))
    } else if let Some(signal_name) = base.strip_suffix('-') {
        Ok((
            signal_name.to_owned(),
            TransitionKind::Negative,
            copy_number,
        ))
    } else if let Some(signal_name) = base.strip_suffix('~') {
        Ok((signal_name.to_owned(), TransitionKind::Toggle, copy_number))
    } else if let Some(signal_name) = base.strip_suffix('*') {
        Ok((signal_name.to_owned(), TransitionKind::Hatch, copy_number))
    } else {
        Ok((base.to_owned(), TransitionKind::Dummy, copy_number))
    }
}

pub fn read_astg(
    input: &str,
    filename: impl Into<Option<String>>,
) -> Result<AstgGraph, AstgReadError>
{
    let mut source = InputSource::new(input);
    let mut graph = AstgGraph::new(String::new());
    graph.filename = filename.into();

    read_stg(&mut source, &mut graph)?;
    read_graph_check(&mut graph);

    if graph.name.is_empty() {
        graph.name = graph.filename.clone().unwrap_or_else(|| "astg".to_owned());
    }

    Ok(graph)
}

pub fn write_astg(graph: &AstgGraph, options: &AstgWriteOptions) -> String
{
    let mut output = String::new();
    output.push_str(".model ");
    output.push_str(&graph.name);
    output.push('\n');

    for comment in &graph.comments {
        output.push_str(".note");
        output.push_str(comment);
        output.push('\n');
    }

    write_signals(&mut output, ".inputs", graph, SignalType::Input);
    write_signals(&mut output, ".outputs", graph, SignalType::Output);
    write_optional_signals(&mut output, ".internal", graph, SignalType::Internal);
    write_optional_signals(&mut output, ".dummy", graph, SignalType::Dummy);

    output.push_str(".graph\n");

    for (place_index, place) in graph.places.iter().enumerate() {
        if options.hide_places && is_boring_place(graph, place_index) {
            continue;
        }

        output.push_str(&place.name);
        write_position(&mut output, place.x, place.y);

        for edge in graph.outgoing_edges(VertexRef::Place(place_index)) {
            if let VertexRef::Transition(transition_index) = edge.to {
                output.push(' ');
                output.push_str(&graph.transitions[transition_index].name);
                if let Some(guard) = &edge.guard {
                    output.push_str(" ?");
                    output.push_str(guard);
                }
            }
        }

        output.push('\n');
    }

    for (transition_index, transition) in graph.transitions.iter().enumerate() {
        output.push_str(&transition.name);
        if transition.delay != 0.0 {
            output.push(' ');
            output.push_str(&trim_float(transition.delay));
        }

        write_position(&mut output, transition.x, transition.y);

        for edge in graph.outgoing_edges(VertexRef::Transition(transition_index)) {
            if let VertexRef::Place(place_index) = edge.to {
                output.push(' ');
                if options.hide_places && is_boring_place(graph, place_index) {
                    if let Some(target_transition) =
                        sole_place_output_transition(graph, place_index)
                    {
                        output.push_str(&graph.transitions[target_transition].name);
                    } else {
                        output.push_str(&graph.places[place_index].name);
                    }
                } else {
                    output.push_str(&graph.places[place_index].name);
                }
            }
        }

        output.push('\n');
    }

    if graph.has_marking {
        output.push_str(".marking ");
        write_marking(graph, &mut output);
    }

    output.push_str(".end\n");
    output
}

fn read_stg(source: &mut InputSource, graph: &mut AstgGraph) -> Result<(), AstgReadError>
{
    loop {
        let token = source.token()?;
        match token.as_str() {
            "" => return Ok(()),
            ".end" => return Ok(()),
            ".model" | ".name" => {
                let model_name = source.token()?;
                if model_name == "\n" || model_name.is_empty() {
                    return source.error("no model name specified");
                }

                graph.name = model_name;
                source.must_be("\n")?;
            }
            ".note" => graph.comments.push(source.read_note()),
            ".graph" => {
                source.must_be("\n")?;
                read_graph(source, graph)?;
            }
            ".inputs" => read_signals(source, graph, SignalType::Input)?,
            ".internal" => read_signals(source, graph, SignalType::Internal)?,
            ".outputs" => read_signals(source, graph, SignalType::Output)?,
            ".dummy" => read_signals(source, graph, SignalType::Dummy)?,
            ".marking" => read_marking(source, graph)?,
            "\n" => {}
            token if token.starts_with('.') => return source.error("unrecognized keyword"),
            _ => return source.error("what is this supposed to be?"),
        }
    }
}

fn read_signals(
    source: &mut InputSource,
    graph: &mut AstgGraph,
    signal_type: SignalType,
) -> Result<(), AstgReadError>
{
    loop {
        let token = source.token()?;
        if token == "\n" || token.is_empty() {
            return Ok(());
        }

        if is_signal_name(&token) {
            graph.find_or_create_signal(&token, signal_type);
        } else {
            return source.error("invalid signal name");
        }
    }
}

fn read_graph(source: &mut InputSource, graph: &mut AstgGraph) -> Result<(), AstgReadError>
{
    loop {
        let next = source.peek_nonblank()?;
        if next.is_empty() || next.starts_with('.') {
            return Ok(());
        }

        if next == "\n" {
            source.token()?;
        } else if next
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
        {
            read_vertex(source, graph)?;
        } else {
            return source.error("bad vertex description");
        }
    }
}

fn read_vertex(source: &mut InputSource, graph: &mut AstgGraph) -> Result<(), AstgReadError>
{
    let name = source.token()?;
    if is_place_name(graph, &name) {
        let place_index = graph.find_or_create_place(Some(&name));
        graph.places[place_index].user_named = true;
        read_position(source, |x, y| {
            graph.places[place_index].x = x;
            graph.places[place_index].y = y;
        })?;

        while source.peek_nonblank()? != "\n" && !source.peek_nonblank()?.is_empty() {
            read_place_fanout(source, graph, place_index)?;
        }

        source.consume_line_end()?;
    } else {
        let transition_index = find_transition_by_name(source, graph, &name, true)?;
        read_delay(source, graph, transition_index)?;
        read_position(source, |x, y| {
            graph.transitions[transition_index].x = x;
            graph.transitions[transition_index].y = y;
        })?;

        while source.peek_nonblank()? != "\n" && !source.peek_nonblank()?.is_empty() {
            read_transition_fanout(source, graph, transition_index)?;
        }

        source.consume_line_end()?;
    }

    Ok(())
}

fn read_place_fanout(
    source: &mut InputSource,
    graph: &mut AstgGraph,
    place_index: usize,
) -> Result<(), AstgReadError>
{
    let name = source.token()?;
    let transition_index = find_transition_by_name(source, graph, &name, true)?;
    if let Some(edge_index) = graph.add_edge(
        VertexRef::Place(place_index),
        VertexRef::Transition(transition_index),
        source,
    ) {
        if source.maybe("?")? {
            graph.edges[edge_index].guard = Some(source.token()?);
        }

        graph.edges[edge_index].spline_points = read_point_list(source)?;
    }

    Ok(())
}

fn read_transition_fanout(
    source: &mut InputSource,
    graph: &mut AstgGraph,
    transition_index: usize,
) -> Result<(), AstgReadError>
{
    let name = source.token()?;
    if !name
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
    {
        return source.error("bad fanout place");
    }

    if is_place_name(graph, &name) {
        let place_index = graph.find_or_create_place(Some(&name));
        if let Some(edge_index) = graph.add_edge(
            VertexRef::Transition(transition_index),
            VertexRef::Place(place_index),
            source,
        ) {
            graph.edges[edge_index].spline_points = read_point_list(source)?;
        }
    } else {
        let target_index = find_transition_by_name(source, graph, &name, true)?;
        let place_index = graph.find_or_create_place(None);
        graph.add_edge(
            VertexRef::Transition(transition_index),
            VertexRef::Place(place_index),
            source,
        );
        graph.add_edge(
            VertexRef::Place(place_index),
            VertexRef::Transition(target_index),
            source,
        );
    }

    Ok(())
}

fn read_marking(source: &mut InputSource, graph: &mut AstgGraph) -> Result<(), AstgReadError>
{
    graph.has_marking = true;
    source.must_be("{")?;

    while !source.maybe("}")? {
        if source.maybe("<")? {
            let first = source.token()?;
            let first_index = find_transition_by_name(source, graph, &first, false)?;
            source.must_be(",")?;
            let second = source.token()?;
            let second_index = find_transition_by_name(source, graph, &second, false)?;
            source.must_be(">")?;

            let place_index = graph.edges.iter().find_map(|edge| {
                if edge.from == VertexRef::Transition(first_index) {
                    if let VertexRef::Place(place_index) = edge.to {
                        if graph.edges.iter().any(|next_edge| {
                            next_edge.from == VertexRef::Place(place_index)
                                && next_edge.to == VertexRef::Transition(second_index)
                        }) {
                            return Some(place_index);
                        }
                    }
                }

                None
            });

            if let Some(place_index) = place_index {
                graph.places[place_index].initial_token = true;
            } else {
                return source.error("couldn't find this edge");
            }
        } else {
            let place_name = source.token()?;
            if let Some(place_index) = graph
                .places
                .iter()
                .position(|place| place.name == place_name)
            {
                graph.places[place_index].initial_token = true;
            } else {
                return source.error("no place with this name");
            }
        }
    }

    source.consume_line_end()?;
    Ok(())
}

fn find_transition_by_name(
    source: &mut InputSource,
    graph: &mut AstgGraph,
    name: &str,
    create: bool,
) -> Result<usize, AstgReadError>
{
    let (signal_name, kind, copy_number) =
        parse_transition_name(name).map_err(|error| source.with(error))?;
    Ok(graph
        .find_or_create_transition(&signal_name, kind, copy_number, create)
        .map_err(|_| {
            if create {
                source.diagnostic_error("no such signal")
            } else {
                source.diagnostic_error("no such transition")
            }
        })?)
}

fn read_delay(
    source: &mut InputSource,
    graph: &mut AstgGraph,
    transition_index: usize,
) -> Result<(), AstgReadError>
{
    if source
        .peek_nonblank()?
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        let token = source.token()?;
        graph.transitions[transition_index].delay = token
            .parse::<f64>()
            .map_err(|_| source.diagnostic_error("invalid transition delay value"))?;
    }

    Ok(())
}

fn read_position<F>(source: &mut InputSource, mut assign: F) -> Result<(), AstgReadError>
where
    F: FnMut(f64, f64),
{
    if !source.maybe("(")? {
        return Ok(());
    }

    let x = source
        .token()?
        .parse::<f64>()
        .map_err(|_| source.diagnostic_error("bad x coordinate"))?;
    source.must_be(",")?;
    let y = source
        .token()?
        .parse::<f64>()
        .map_err(|_| source.diagnostic_error("bad y coordinate"))?;
    source.must_be(")")?;
    assign(x, y);
    Ok(())
}

fn read_point_list(source: &mut InputSource) -> Result<Vec<f64>, AstgReadError>
{
    if !source.maybe("(")? {
        return Ok(Vec::new());
    }

    let mut points = Vec::new();
    loop {
        let value = source
            .token()?
            .parse::<f64>()
            .map_err(|_| source.diagnostic_error("bad coordinate"))?;
        points.push(value);

        if !source.maybe(",")? {
            break;
        }
    }

    source.must_be(")")?;
    Ok(points)
}

fn read_graph_check(graph: &mut AstgGraph)
{
    let hatch_indices = graph
        .transitions
        .iter()
        .enumerate()
        .filter_map(|(index, transition)| {
            (transition.kind == TransitionKind::Hatch).then_some(index)
        })
        .collect::<Vec<_>>();

    for hatch_index in hatch_indices.into_iter().rev() {
        expand_hatch_transition(graph, hatch_index);
    }
}

fn expand_hatch_transition(graph: &mut AstgGraph, hatch_index: usize)
{
    let hatch = graph.transitions[hatch_index].clone();
    let dummy_signal = format!("dummy{}", graph.signals.len());
    graph.find_or_create_signal(&dummy_signal, SignalType::Dummy);
    let center_place = graph.find_or_create_place(None);
    let dummy_one = graph
        .find_or_create_transition(&dummy_signal, TransitionKind::Dummy, 1, true)
        .expect("dummy signal was just inserted");
    let dummy_two = graph
        .find_or_create_transition(&dummy_signal, TransitionKind::Dummy, 2, true)
        .expect("dummy signal was just inserted");
    let toggle = graph
        .find_or_create_transition(
            &hatch.signal,
            TransitionKind::Toggle,
            hatch.copy_number,
            true,
        )
        .expect("hatch signal already exists");

    let input_places = graph
        .edges
        .iter()
        .filter_map(|edge| {
            (edge.to == VertexRef::Transition(hatch_index)).then_some(edge.from.clone())
        })
        .collect::<Vec<_>>();
    let output_places = graph
        .edges
        .iter()
        .filter_map(|edge| {
            (edge.from == VertexRef::Transition(hatch_index)).then_some(edge.to.clone())
        })
        .collect::<Vec<_>>();

    graph.edges.retain(|edge| {
        edge.from != VertexRef::Transition(hatch_index)
            && edge.to != VertexRef::Transition(hatch_index)
    });

    for input_place in input_places {
        graph.edges.push(Edge {
            from: input_place,
            to: VertexRef::Transition(dummy_one),
            guard: None,
            spline_points: Vec::new(),
        });
    }

    graph.edges.push(Edge {
        from: VertexRef::Transition(dummy_one),
        to: VertexRef::Place(center_place),
        guard: None,
        spline_points: Vec::new(),
    });
    graph.edges.push(Edge {
        from: VertexRef::Place(center_place),
        to: VertexRef::Transition(dummy_two),
        guard: None,
        spline_points: Vec::new(),
    });

    for output_place in output_places {
        graph.edges.push(Edge {
            from: VertexRef::Transition(dummy_two),
            to: output_place,
            guard: None,
            spline_points: Vec::new(),
        });
    }

    graph.edges.push(Edge {
        from: VertexRef::Place(center_place),
        to: VertexRef::Transition(toggle),
        guard: None,
        spline_points: Vec::new(),
    });
    graph.edges.push(Edge {
        from: VertexRef::Transition(toggle),
        to: VertexRef::Place(center_place),
        guard: None,
        spline_points: Vec::new(),
    });

    graph.transitions.remove(hatch_index);
    for edge in &mut graph.edges {
        renumber_transition_ref(&mut edge.from, hatch_index);
        renumber_transition_ref(&mut edge.to, hatch_index);
    }
}

fn renumber_transition_ref(vertex: &mut VertexRef, removed_index: usize)
{
    if let VertexRef::Transition(index) = vertex {
        if *index > removed_index {
            *index -= 1;
        }
    }
}

fn is_signal_name(name: &str) -> bool
{
    name.chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
        && !name.contains('/')
        && !name.contains('-')
        && !name.contains('+')
}

fn is_place_name(graph: &AstgGraph, name: &str) -> bool
{
    if name.contains('/') {
        return false;
    }

    if graph.signal(name).is_some() {
        return false;
    }

    name.chars()
        .last()
        .is_none_or(|character| !matches!(character, '+' | '-' | '*' | '~'))
}

fn write_signals(output: &mut String, label: &str, graph: &AstgGraph, signal_type: SignalType)
{
    output.push_str(label);
    for signal in graph
        .signals
        .iter()
        .filter(|signal| signal.signal_type == signal_type)
    {
        output.push(' ');
        output.push_str(&signal.name);
    }

    output.push('\n');
}

fn write_optional_signals(
    output: &mut String,
    label: &str,
    graph: &AstgGraph,
    signal_type: SignalType,
)
{
    if graph
        .signals
        .iter()
        .any(|signal| signal.signal_type == signal_type)
    {
        write_signals(output, label, graph, signal_type);
    }
}

fn write_position(output: &mut String, x: f64, y: f64)
{
    if x != 0.0 || y != 0.0 {
        output.push_str(" (");
        output.push_str(&trim_float(x));
        output.push(',');
        output.push_str(&trim_float(y));
        output.push(')');
    }
}

fn write_marking(graph: &AstgGraph, output: &mut String)
{
    let marked_places = graph
        .places
        .iter()
        .filter(|place| place.initial_token)
        .map(|place| place.name.as_str())
        .collect::<Vec<_>>();

    if marked_places.is_empty() {
        output.push_str("{ }\n");
        return;
    }

    output.push('{');
    for (index, place_name) in marked_places.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }

        output.push_str(place_name);
    }

    output.push_str("}\n");
}

fn trim_float(value: f64) -> String
{
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }

    if text.ends_with('.') {
        text.pop();
    }

    text
}

fn is_boring_place(graph: &AstgGraph, place_index: usize) -> bool
{
    !graph.places[place_index].user_named
        && !graph.places[place_index].initial_token
        && graph.incoming_edges(VertexRef::Place(place_index)).count() == 1
        && graph.outgoing_edges(VertexRef::Place(place_index)).count() == 1
}

fn sole_place_output_transition(graph: &AstgGraph, place_index: usize) -> Option<usize>
{
    graph
        .outgoing_edges(VertexRef::Place(place_index))
        .find_map(|edge| {
            if let VertexRef::Transition(index) = edge.to {
                Some(index)
            } else {
                None
            }
        })
}

#[derive(Clone, Debug)]
struct InputSource
{
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    saved: Option<String>,
    warnings: Vec<AstgParseDiagnostic>,
}

impl InputSource
{
    fn new(input: &str) -> Self
    {
        Self {
            chars: input.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            saved: None,
            warnings: Vec::new(),
        }
    }

    fn token(&mut self) -> Result<String, AstgReadError>
    {
        if let Some(token) = self.saved.take() {
            return Ok(token);
        }

        self.skip_inline_space_and_comments();
        let Some(first) = self.next_char() else {
            return Ok(String::new());
        };

        if first == '\n' {
            return Ok("\n".to_owned());
        }

        if "{}<,>".contains(first) {
            return Ok(first.to_string());
        }

        if first == '(' {
            return self.balanced_token(first, ')');
        }

        if first == '"' {
            return self.balanced_token(first, '"');
        }

        let mut token = first.to_string();
        while let Some(next) = self.peek_char() {
            if next.is_whitespace() || ",>)}?".contains(next) {
                break;
            }

            token.push(self.next_char().expect("peeked character must exist"));
        }

        Ok(token)
    }

    fn balanced_token(&mut self, start: char, end: char) -> Result<String, AstgReadError>
    {
        let mut token = String::new();
        token.push(start);
        let mut depth = 1;

        while let Some(character) = self.next_char() {
            token.push(character);
            if character == end {
                depth -= 1;
                if depth == 0 {
                    return Ok(token);
                }
            } else if character == start {
                depth += 1;
            }
        }

        self.error("unexpected end-of-file")
    }

    fn peek_nonblank(&mut self) -> Result<String, AstgReadError>
    {
        let token = self.token()?;
        self.saved = Some(token.clone());
        Ok(token)
    }

    fn maybe(&mut self, expected: &str) -> Result<bool, AstgReadError>
    {
        if expected.chars().count() == 1 && self.saved.is_none() {
            self.skip_inline_space_and_comments();
            if self.peek_char() == expected.chars().next() {
                self.next_char();
                return Ok(true);
            }

            return Ok(false);
        }

        let token = self.token()?;
        if token == expected {
            Ok(true)
        } else {
            self.saved = Some(token);
            Ok(false)
        }
    }

    fn must_be(&mut self, expected: &str) -> Result<(), AstgReadError>
    {
        let token = self.token()?;
        if token == expected {
            Ok(())
        } else if expected == "\n" {
            self.error("expecting 'end of line' here")
        } else {
            self.error(&format!("expecting '{expected}' here"))
        }
    }

    fn read_note(&mut self) -> String
    {
        let mut note = String::new();
        while let Some(character) = self.next_char() {
            if character == '\n' {
                break;
            }

            note.push(character);
        }

        note
    }

    fn consume_line_end(&mut self) -> Result<(), AstgReadError>
    {
        if self.peek_nonblank()? == "\n" {
            self.token()?;
        }

        Ok(())
    }

    fn warn(&mut self, message: &str)
    {
        self.warnings.push(AstgParseDiagnostic {
            line: self.line,
            column: self.column,
            message: message.to_owned(),
        });
    }

    fn error<T>(&self, message: &str) -> Result<T, AstgReadError>
    {
        Err(self.diagnostic_error(message))
    }

    fn diagnostic_error(&self, message: &str) -> AstgReadError
    {
        AstgReadError::Parse(AstgParseDiagnostic {
            line: self.line,
            column: self.column,
            message: message.to_owned(),
        })
    }

    fn with(&self, error: AstgReadError) -> AstgReadError
    {
        match error {
            AstgReadError::Parse(mut diagnostic) => {
                if diagnostic.line == 0 {
                    diagnostic.line = self.line;
                    diagnostic.column = self.column;
                }

                AstgReadError::Parse(diagnostic)
            }
            other => other,
        }
    }

    fn skip_inline_space_and_comments(&mut self)
    {
        loop {
            while self
                .peek_char()
                .is_some_and(|character| character != '\n' && character.is_whitespace())
            {
                self.next_char();
            }

            if self.peek_char() == Some('#') {
                while self.peek_char().is_some_and(|character| character != '\n') {
                    self.next_char();
                }
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char>
    {
        self.chars.get(self.index).copied()
    }

    fn next_char(&mut self) -> Option<char>
    {
        let character = self.chars.get(self.index).copied()?;
        self.index += 1;
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(character)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn transition_names_round_trip()
    {
        assert_eq!(
            make_transition_name("req", TransitionKind::Positive, 0),
            "req+"
        );
        assert_eq!(
            make_transition_name("ack", TransitionKind::Negative, 2),
            "ack-/2"
        );
        assert_eq!(
            parse_transition_name("done~/4").unwrap(),
            ("done".to_owned(), TransitionKind::Toggle, 4)
        );
        assert_eq!(
            parse_transition_name("tau/3").unwrap(),
            ("tau".to_owned(), TransitionKind::Dummy, 3)
        );
    }

    #[test]
    fn reads_signals_places_edges_guards_and_marking()
    {
        let graph = read_astg(
            ".model sample\n\
             .inputs req\n\
             .outputs ack\n\
             .internal int\n\
             .dummy tau\n\
             .graph\n\
             p0 (1,2) req+ ?ack (3,4)\n\
             req+ 0.5 (5,6) p1\n\
             p1 ack-\n\
             ack- p0\n\
             .marking { <ack-,req+> }\n\
             .end\n",
            Some("sample.astg".to_owned()),
        )
        .unwrap();

        assert_eq!(graph.name, "sample");
        assert_eq!(graph.filename.as_deref(), Some("sample.astg"));
        assert_eq!(graph.signals.len(), 4);
        assert_eq!(graph.transitions.len(), 2);
        assert_eq!(graph.places.len(), 2);
        assert!(graph.has_marking);
        assert!(graph.place("p0").unwrap().initial_token);
        assert_eq!(graph.transition("req+").unwrap().delay, 0.5);
        assert_eq!(graph.place("p0").unwrap().x, 1.0);

        let guarded = graph
            .edges
            .iter()
            .find(|edge| edge.guard.as_deref() == Some("ack"))
            .unwrap();
        assert_eq!(guarded.spline_points, vec![3.0, 4.0]);
    }

    #[test]
    fn inserts_implicit_places_between_adjacent_transitions()
    {
        let graph = read_astg(
            ".model sample\n\
             .inputs a b\n\
             .graph\n\
             a+ b+\n\
             b+ a+\n\
             .end\n",
            None,
        )
        .unwrap();

        assert_eq!(graph.places.len(), 2);
        assert_eq!(graph.edges.len(), 4);
        assert!(graph.place("p0").is_some());
        assert!(graph.transition("a+").is_some());
        assert!(graph.transition("b+").is_some());
    }

    #[test]
    fn expands_hatch_transitions_to_dummy_and_toggle_cycle()
    {
        let graph = read_astg(
            ".model sample\n\
             .inputs a\n\
             .graph\n\
             p0 a*\n\
             a* p1\n\
             .end\n",
            None,
        )
        .unwrap();

        assert!(graph.transition("a*").is_none());
        assert!(graph.transition("a~").is_some());
        assert!(
            graph
                .transitions
                .iter()
                .any(|transition| transition.kind == TransitionKind::Dummy)
        );
        assert!(
            graph
                .signals
                .iter()
                .any(|signal| signal.signal_type == SignalType::Dummy)
        );
    }

    #[test]
    fn write_astg_preserves_readable_text()
    {
        let graph = read_astg(
            ".model sample\n\
             .inputs req\n\
             .outputs ack\n\
             .graph\n\
             p0 req+\n\
             req+ p1\n\
             p1 ack-\n\
             ack- p0\n\
             .marking { p0 }\n\
             .end\n",
            None,
        )
        .unwrap();

        let text = write_astg(&graph, &AstgWriteOptions::default());
        assert!(text.contains(".model sample\n"));
        assert!(text.contains(".inputs req\n"));
        assert!(text.contains(".outputs ack\n"));
        assert!(text.contains(".marking {p0}\n"));

        let reparsed = read_astg(&text, None).unwrap();
        assert_eq!(reparsed.signals.len(), graph.signals.len());
        assert_eq!(reparsed.transitions.len(), graph.transitions.len());
        assert_eq!(reparsed.places.len(), graph.places.len());
    }

    #[test]
    fn rejects_invalid_signal_names()
    {
        let error = read_astg(".inputs a+\n.end\n", None).unwrap_err();
        assert!(error.to_string().contains("invalid signal name"));
    }

    #[test]
    fn reports_unknown_marked_place()
    {
        let error = read_astg(
            ".model sample\n\
             .inputs a\n\
             .graph\n\
             p0 a+\n\
             .marking { missing }\n\
             .end\n",
            None,
        )
        .unwrap_err();

        assert!(error.to_string().contains("no place with this name"));
    }
}
