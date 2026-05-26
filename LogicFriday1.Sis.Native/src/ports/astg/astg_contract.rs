//! Native Rust model of ASTG net contraction.
//!
//! The legacy implementation contracts a duplicated STG for one output signal
//! by removing eliminable input-signal transitions when Chu's 6.2 and 6.8
//! restrictions allow it. This module keeps that behavior in safe Rust data
//! structures without exposing C ABI symbols.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AstgVertexId {
    Place(usize),
    Transition(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgSignalKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgTransitionPolarity {
    Positive,
    Negative,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgSignal {
    pub name: String,
    pub kind: AstgSignalKind,
    pub can_eliminate: bool,
    pub logic_from: Option<String>,
}

impl AstgSignal {
    pub fn new(name: impl Into<String>, kind: AstgSignalKind) -> Self {
        Self {
            name: name.into(),
            kind,
            can_eliminate: true,
            logic_from: None,
        }
    }

    pub fn with_logic_from(mut self, signal_name: impl Into<String>) -> Self {
        self.logic_from = Some(signal_name.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgPlace {
    pub name: String,
    pub initial_token: bool,
    deleted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgTransition {
    pub name: String,
    pub signal_name: String,
    pub polarity: AstgTransitionPolarity,
    deleted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgEdge {
    pub tail: AstgVertexId,
    pub head: AstgVertexId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgGraph {
    pub name: String,
    pub signals: Vec<AstgSignal>,
    pub places: Vec<AstgPlace>,
    pub transitions: Vec<AstgTransition>,
    pub edges: Vec<AstgEdge>,
}

impl AstgGraph {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            signals: Vec::new(),
            places: Vec::new(),
            transitions: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_signal(&mut self, signal: AstgSignal) {
        self.signals.push(signal);
    }

    pub fn add_place(&mut self, name: impl Into<String>, initial_token: bool) -> AstgVertexId {
        let id = self.places.len();
        self.places.push(AstgPlace {
            name: name.into(),
            initial_token,
            deleted: false,
        });
        AstgVertexId::Place(id)
    }

    pub fn add_transition(
        &mut self,
        name: impl Into<String>,
        signal_name: impl Into<String>,
        polarity: AstgTransitionPolarity,
    ) -> AstgVertexId {
        let id = self.transitions.len();
        self.transitions.push(AstgTransition {
            name: name.into(),
            signal_name: signal_name.into(),
            polarity,
            deleted: false,
        });
        AstgVertexId::Transition(id)
    }

    pub fn add_edge(&mut self, tail: AstgVertexId, head: AstgVertexId) {
        self.edges.push(AstgEdge { tail, head });
    }

    pub fn output_count(&self) -> usize {
        self.signals
            .iter()
            .filter(|signal| signal.kind == AstgSignalKind::Output)
            .count()
    }

    pub fn active_place_names(&self) -> Vec<&str> {
        self.places
            .iter()
            .filter(|place| !place.deleted)
            .map(|place| place.name.as_str())
            .collect()
    }

    pub fn active_transition_names(&self) -> Vec<&str> {
        self.transitions
            .iter()
            .filter(|transition| !transition.deleted)
            .map(|transition| transition.name.as_str())
            .collect()
    }

    pub fn signal(&self, name: &str) -> Option<&AstgSignal> {
        self.signals.iter().find(|signal| signal.name == name)
    }

    fn signal_mut(&mut self, name: &str) -> Option<&mut AstgSignal> {
        self.signals.iter_mut().find(|signal| signal.name == name)
    }

    fn signal_index(&self, name: &str) -> Option<usize> {
        self.signals.iter().position(|signal| signal.name == name)
    }

    fn transition(&self, id: usize) -> Option<&AstgTransition> {
        self.transitions
            .get(id)
            .filter(|transition| !transition.deleted)
    }

    fn place(&self, id: usize) -> Option<&AstgPlace> {
        self.places.get(id).filter(|place| !place.deleted)
    }

    fn is_active_vertex(&self, vertex: AstgVertexId) -> bool {
        match vertex {
            AstgVertexId::Place(id) => self.place(id).is_some(),
            AstgVertexId::Transition(id) => self.transition(id).is_some(),
        }
    }

    fn in_edges(&self, vertex: AstgVertexId) -> Vec<AstgEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.head == vertex && self.is_active_vertex(edge.tail))
            .cloned()
            .collect()
    }

    fn out_edges(&self, vertex: AstgVertexId) -> Vec<AstgEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.tail == vertex && self.is_active_vertex(edge.head))
            .cloned()
            .collect()
    }

    fn in_degree(&self, vertex: AstgVertexId) -> usize {
        self.in_edges(vertex).len()
    }

    fn out_degree(&self, vertex: AstgVertexId) -> usize {
        self.out_edges(vertex).len()
    }

    fn input_places(&self, transition: AstgVertexId) -> Vec<AstgVertexId> {
        self.in_edges(transition)
            .into_iter()
            .map(|edge| edge.tail)
            .filter(|vertex| matches!(vertex, AstgVertexId::Place(_)))
            .collect()
    }

    fn output_places(&self, transition: AstgVertexId) -> Vec<AstgVertexId> {
        self.out_edges(transition)
            .into_iter()
            .map(|edge| edge.head)
            .filter(|vertex| matches!(vertex, AstgVertexId::Place(_)))
            .collect()
    }

    fn input_transitions(&self, place: AstgVertexId) -> Vec<AstgVertexId> {
        self.in_edges(place)
            .into_iter()
            .map(|edge| edge.tail)
            .filter(|vertex| matches!(vertex, AstgVertexId::Transition(_)))
            .collect()
    }

    fn output_transitions(&self, place: AstgVertexId) -> Vec<AstgVertexId> {
        self.out_edges(place)
            .into_iter()
            .map(|edge| edge.head)
            .filter(|vertex| matches!(vertex, AstgVertexId::Transition(_)))
            .collect()
    }

    fn transitions_for_signal(&self, signal_name: &str) -> Vec<usize> {
        self.transitions
            .iter()
            .enumerate()
            .filter(|(_, transition)| !transition.deleted && transition.signal_name == signal_name)
            .map(|(index, _)| index)
            .collect()
    }

    fn delete_place(&mut self, id: usize) {
        if let Some(place) = self.places.get_mut(id) {
            place.deleted = true;
        }
        self.remove_dead_edges();
    }

    fn delete_transition(&mut self, id: usize) {
        if let Some(transition) = self.transitions.get_mut(id) {
            transition.deleted = true;
        }
        self.remove_dead_edges();
    }

    fn remove_dead_edges(&mut self) {
        let active_places = self
            .places
            .iter()
            .map(|place| !place.deleted)
            .collect::<Vec<_>>();
        let active_transitions = self
            .transitions
            .iter()
            .map(|transition| !transition.deleted)
            .collect::<Vec<_>>();

        self.edges.retain(|edge| {
            vertex_active_in(edge.tail, &active_places, &active_transitions)
                && vertex_active_in(edge.head, &active_places, &active_transitions)
        });
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgContractError {
    UnknownOutputSignal(String),
    OutputSignalRequired(String),
}

impl fmt::Display for AstgContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOutputSignal(signal) => write!(formatter, "unknown ASTG signal {signal}"),
            Self::OutputSignalRequired(signal) => {
                write!(formatter, "ASTG signal {signal} is not an output signal")
            }
        }
    }
}

impl Error for AstgContractError {}

pub fn astg_contract(
    stg: &AstgGraph,
    output_signal_name: &str,
    keep_free_choice: bool,
) -> Result<AstgGraph, AstgContractError> {
    let output_signal = stg
        .signal(output_signal_name)
        .ok_or_else(|| AstgContractError::UnknownOutputSignal(output_signal_name.to_owned()))?;

    if output_signal.kind != AstgSignalKind::Output {
        return Err(AstgContractError::OutputSignalRequired(
            output_signal_name.to_owned(),
        ));
    }

    let mut eligibility = stg.clone();
    for signal in &mut eligibility.signals {
        signal.can_eliminate = true;
    }
    eligibility
        .signal_mut(output_signal_name)
        .expect("output signal was already found")
        .can_eliminate = false;

    for transition in eligibility.transitions_for_signal(output_signal_name) {
        keep_in_sig(&mut eligibility, transition);
    }

    let mut contracted = eligibility.clone();
    contracted.name = format!("net_{output_signal_name}");
    for signal in &mut contracted.signals {
        signal.kind = AstgSignalKind::Input;
    }
    contracted
        .signal_mut(output_signal_name)
        .expect("output signal was already found")
        .kind = AstgSignalKind::Output;

    let signal_names = contracted
        .signals
        .iter()
        .map(|signal| signal.name.clone())
        .collect::<Vec<_>>();
    for signal_name in signal_names {
        if can_contract_signal(&contracted, &signal_name, keep_free_choice) {
            eliminate_signal(&mut contracted, &signal_name);
        }
    }

    Ok(contracted)
}

pub fn can_contract_signal(stg: &AstgGraph, signal_name: &str, keep_free_choice: bool) -> bool {
    let Some(signal) = stg.signal(signal_name) else {
        return false;
    };

    signal.can_eliminate
        && stg
            .transitions_for_signal(signal_name)
            .into_iter()
            .all(|transition| can_contract_transition(stg, transition, keep_free_choice))
}

fn can_contract_transition(stg: &AstgGraph, transition: usize, keep_free_choice: bool) -> bool {
    let vertex = AstgVertexId::Transition(transition);
    if keep_free_choice {
        check_62(stg, vertex) && check_68(stg, vertex)
    } else {
        check_68(stg, vertex)
    }
}

fn check_62(stg: &AstgGraph, transition: AstgVertexId) -> bool {
    let mut marked = HashSet::new();

    for place in stg.input_places(transition) {
        marked.insert(place);
        for input_transition in stg.input_transitions(place) {
            marked.insert(input_transition);
        }
    }

    for place in stg.output_places(transition) {
        for output_transition in stg.output_transitions(place) {
            if marked.contains(&output_transition) {
                return false;
            }

            for output_place in stg.output_places(output_transition) {
                if marked.contains(&output_place) {
                    return false;
                }
            }
        }
    }

    true
}

fn check_68(stg: &AstgGraph, transition: AstgVertexId) -> bool {
    check_68a(stg, transition) && check_68b(stg, transition)
}

fn check_68a(stg: &AstgGraph, transition: AstgVertexId) -> bool {
    check_68a1(stg, transition) || check_68a2(stg, transition)
}

fn check_68a1(stg: &AstgGraph, transition: AstgVertexId) -> bool {
    stg.input_places(transition)
        .into_iter()
        .all(|place| stg.out_degree(place) <= 1)
}

fn check_68a2(stg: &AstgGraph, transition: AstgVertexId) -> bool {
    stg.output_places(transition)
        .into_iter()
        .all(|place| stg.in_degree(place) <= 1)
}

fn check_68b(stg: &AstgGraph, transition: AstgVertexId) -> bool {
    if stg.in_degree(transition) <= 1 || stg.out_degree(transition) <= 1 {
        return true;
    }

    for input_place in stg.input_places(transition) {
        for input_transition in stg.input_transitions(input_place) {
            let input_cycle_vertices = cycle_vertices_containing(stg, input_transition);
            for output_place in stg.output_places(transition) {
                for output_transition in stg.output_transitions(output_place) {
                    let has_overlap = simple_cycles_containing(stg, output_transition)
                        .into_iter()
                        .any(|cycle| {
                            cycle
                                .iter()
                                .any(|vertex| input_cycle_vertices.contains(vertex))
                        });

                    if !has_overlap {
                        return false;
                    }
                }
            }
        }
    }

    true
}

fn keep_in_sig(stg: &mut AstgGraph, transition: usize) {
    for input_place in stg.input_places(AstgVertexId::Transition(transition)) {
        for input_transition in stg.input_transitions(input_place) {
            let AstgVertexId::Transition(input_transition_id) = input_transition else {
                continue;
            };

            let signal_name = stg.transitions[input_transition_id].signal_name.clone();
            mark_signal_and_context(stg, &signal_name);
        }
    }
}

fn mark_signal_and_context(stg: &mut AstgGraph, signal_name: &str) {
    let mut current = Some(signal_name.to_owned());
    let mut visited = HashSet::new();

    while let Some(name) = current {
        if !visited.insert(name.clone()) {
            break;
        }

        current = if let Some(signal) = stg.signal_mut(&name) {
            signal.can_eliminate = false;
            signal.logic_from.clone()
        } else {
            None
        };
    }
}

fn eliminate_signal(stg: &mut AstgGraph, signal_name: &str) {
    let transitions = stg.transitions_for_signal(signal_name);
    for transition in transitions {
        if stg.transition(transition).is_some() {
            eliminate_transition(stg, transition);
        }
    }

    if let Some(index) = stg.signal_index(signal_name) {
        stg.signals.remove(index);
    }
}

fn eliminate_transition(stg: &mut AstgGraph, transition: usize) {
    let transition_vertex = AstgVertexId::Transition(transition);
    let input_places = stg.input_places(transition_vertex);
    let output_places = stg.output_places(transition_vertex);

    for input_place in &input_places {
        for output_place in &output_places {
            let AstgVertexId::Place(input_place_id) = *input_place else {
                continue;
            };
            let AstgVertexId::Place(output_place_id) = *output_place else {
                continue;
            };

            let name = format!("pc_{}", stg.places.len());
            let initial_token = stg.places[input_place_id].initial_token
                || stg.places[output_place_id].initial_token;
            let new_place = stg.add_place(name, initial_token);
            let input_transitions = stg.input_transitions(*input_place);
            let output_transitions = stg.output_transitions(*output_place);

            for input_transition in input_transitions {
                stg.add_edge(input_transition, new_place);
            }
            for output_transition in output_transitions {
                stg.add_edge(new_place, output_transition);
            }

            if !pure_place_simple(stg, new_place) {
                let AstgVertexId::Place(new_place_id) = new_place else {
                    continue;
                };
                stg.delete_place(new_place_id);
            }
        }
    }

    for input_place in input_places {
        let AstgVertexId::Place(place_id) = input_place else {
            continue;
        };
        if stg.out_degree(input_place) == 1 {
            stg.delete_place(place_id);
        }
    }

    for output_place in output_places {
        let AstgVertexId::Place(place_id) = output_place else {
            continue;
        };
        if stg.in_degree(output_place) == 1 {
            stg.delete_place(place_id);
        }
    }

    stg.delete_transition(transition);
}

fn pure_place_simple(stg: &AstgGraph, place: AstgVertexId) -> bool {
    if stg.in_degree(place) != 1 || stg.out_degree(place) != 1 {
        return true;
    }

    let input_transition = stg.input_transitions(place)[0];
    let output_transition = stg.output_transitions(place)[0];

    if input_transition == output_transition {
        return false;
    }

    for alternate_place in stg.output_places(input_transition) {
        if alternate_place == place {
            continue;
        }

        if stg.in_degree(alternate_place) == 1
            && stg.out_degree(alternate_place) == 1
            && stg.output_transitions(alternate_place)[0] == output_transition
        {
            return false;
        }
    }

    true
}

fn cycle_vertices_containing(stg: &AstgGraph, start: AstgVertexId) -> HashSet<AstgVertexId> {
    simple_cycles_containing(stg, start)
        .into_iter()
        .flatten()
        .collect()
}

fn simple_cycles_containing(stg: &AstgGraph, start: AstgVertexId) -> Vec<Vec<AstgVertexId>> {
    if !stg.is_active_vertex(start) {
        return Vec::new();
    }

    let mut cycles = Vec::new();
    let mut seen = HashSet::new();
    let mut path = vec![start];

    for edge in stg.out_edges(start) {
        path.push(edge.head);
        search_cycle(stg, start, edge.head, &mut path, &mut seen, &mut cycles);
        path.pop();
    }

    cycles
}

fn search_cycle(
    stg: &AstgGraph,
    start: AstgVertexId,
    current: AstgVertexId,
    path: &mut Vec<AstgVertexId>,
    seen: &mut HashSet<Vec<AstgVertexId>>,
    cycles: &mut Vec<Vec<AstgVertexId>>,
) {
    if current == start {
        let mut cycle = path.clone();
        cycle.pop();
        if seen.insert(cycle.clone()) {
            cycles.push(cycle);
        }
        return;
    }

    if path[..path.len() - 1].contains(&current) {
        return;
    }

    for edge in stg.out_edges(current) {
        path.push(edge.head);
        search_cycle(stg, start, edge.head, path, seen, cycles);
        path.pop();
    }
}

fn vertex_active_in(
    vertex: AstgVertexId,
    active_places: &[bool],
    active_transitions: &[bool],
) -> bool {
    match vertex {
        AstgVertexId::Place(id) => active_places.get(id).copied().unwrap_or(false),
        AstgVertexId::Transition(id) => active_transitions.get(id).copied().unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_signal(graph: &mut AstgGraph, name: &str, kind: AstgSignalKind) {
        graph.add_signal(AstgSignal::new(name, kind));
    }

    fn add_transition(graph: &mut AstgGraph, name: &str, signal: &str) -> AstgVertexId {
        graph.add_transition(name, signal, AstgTransitionPolarity::Positive)
    }

    #[test]
    fn contract_duplicates_graph_as_single_output_and_eliminates_unprotected_input_signal() {
        let mut graph = AstgGraph::new("sample");
        add_signal(&mut graph, "a", AstgSignalKind::Input);
        add_signal(&mut graph, "x", AstgSignalKind::Input);
        add_signal(&mut graph, "z", AstgSignalKind::Output);

        let a = add_transition(&mut graph, "a+", "a");
        let x0 = add_transition(&mut graph, "x0+", "x");
        let x1 = add_transition(&mut graph, "x1+", "x");
        let z = add_transition(&mut graph, "z+", "z");
        let p0 = graph.add_place("p0", true);
        let p1 = graph.add_place("p1", false);
        let pz = graph.add_place("pz", false);
        graph.add_edge(x0, p0);
        graph.add_edge(p0, a);
        graph.add_edge(a, p1);
        graph.add_edge(p1, x1);
        graph.add_edge(x1, pz);
        graph.add_edge(pz, z);

        let contracted = astg_contract(&graph, "z", true).unwrap();

        assert_eq!(contracted.name, "net_z");
        assert_eq!(contracted.output_count(), 1);
        assert_eq!(contracted.signal("z").unwrap().kind, AstgSignalKind::Output);
        assert!(contracted.signal("a").is_none());
        assert!(contracted.signal("x").is_some());
        assert_eq!(contracted.active_transition_names(), vec!["x0+", "x1+", "z+"]);
        assert!(contracted.active_place_names().contains(&"pc_3"));
        assert!(contracted.places[3].initial_token);
    }

    #[test]
    fn output_trigger_and_logic_context_are_not_eliminated() {
        let mut graph = AstgGraph::new("sample");
        graph.add_signal(AstgSignal::new("ctx", AstgSignalKind::Input));
        graph.add_signal(AstgSignal::new("trigger", AstgSignalKind::Input).with_logic_from("ctx"));
        add_signal(&mut graph, "z", AstgSignalKind::Output);

        let ctx = add_transition(&mut graph, "ctx+", "ctx");
        let trigger = add_transition(&mut graph, "trigger+", "trigger");
        let z = add_transition(&mut graph, "z+", "z");
        let p0 = graph.add_place("p0", false);
        let p1 = graph.add_place("p1", false);
        graph.add_edge(ctx, p0);
        graph.add_edge(p0, trigger);
        graph.add_edge(trigger, p1);
        graph.add_edge(p1, z);

        let contracted = astg_contract(&graph, "z", false).unwrap();

        assert!(contracted.signal("trigger").is_some());
        assert!(contracted.signal("ctx").is_some());
        assert!(contracted.signal("z").is_some());
    }

    #[test]
    fn keep_free_choice_enforces_check_62() {
        let mut graph = AstgGraph::new("sample");
        add_signal(&mut graph, "a", AstgSignalKind::Input);
        add_signal(&mut graph, "z", AstgSignalKind::Output);

        let a = add_transition(&mut graph, "a+", "a");
        let t_in = add_transition(&mut graph, "tin+", "z");
        let t_out = add_transition(&mut graph, "tout+", "z");
        let p_in = graph.add_place("p_in", false);
        let p_out = graph.add_place("p_out", false);
        graph.add_edge(t_in, p_in);
        graph.add_edge(p_in, a);
        graph.add_edge(a, p_out);
        graph.add_edge(p_out, t_in);
        graph.add_edge(p_out, t_out);

        assert!(!can_contract_signal(&graph, "a", true));
        assert!(can_contract_signal(&graph, "a", false));
    }

    #[test]
    fn check_68a_requires_simple_input_or_output_places() {
        let mut graph = AstgGraph::new("sample");
        add_signal(&mut graph, "a", AstgSignalKind::Input);
        add_signal(&mut graph, "z", AstgSignalKind::Output);

        let a = add_transition(&mut graph, "a+", "a");
        let i0 = add_transition(&mut graph, "i0+", "z");
        let o0 = add_transition(&mut graph, "o0+", "z");
        let side = add_transition(&mut graph, "side+", "z");
        let p_in = graph.add_place("p_in", false);
        let p_out = graph.add_place("p_out", false);
        graph.add_edge(i0, p_in);
        graph.add_edge(p_in, a);
        graph.add_edge(p_in, side);
        graph.add_edge(a, p_out);
        graph.add_edge(side, p_out);
        graph.add_edge(p_out, o0);

        assert!(!can_contract_signal(&graph, "a", false));
    }

    #[test]
    fn check_68b_rejects_predecessor_successor_pairs_without_cycle_overlap() {
        let mut graph = AstgGraph::new("sample");
        add_signal(&mut graph, "a", AstgSignalKind::Input);
        add_signal(&mut graph, "z", AstgSignalKind::Output);

        let a = add_transition(&mut graph, "a+", "a");
        let ti0 = add_transition(&mut graph, "ti0+", "z");
        let ti1 = add_transition(&mut graph, "ti1+", "z");
        let to0 = add_transition(&mut graph, "to0+", "z");
        let to1 = add_transition(&mut graph, "to1+", "z");
        let p_in0 = graph.add_place("p_in0", false);
        let p_in1 = graph.add_place("p_in1", false);
        let p_out0 = graph.add_place("p_out0", false);
        let p_out1 = graph.add_place("p_out1", false);
        let ti_cycle = graph.add_place("ti_cycle", false);
        let to_cycle = graph.add_place("to_cycle", false);

        graph.add_edge(ti0, p_in0);
        graph.add_edge(p_in0, a);
        graph.add_edge(ti1, p_in1);
        graph.add_edge(p_in1, a);
        graph.add_edge(a, p_out0);
        graph.add_edge(p_out0, to0);
        graph.add_edge(a, p_out1);
        graph.add_edge(p_out1, to1);
        graph.add_edge(ti0, ti_cycle);
        graph.add_edge(ti_cycle, ti0);
        graph.add_edge(to0, to_cycle);
        graph.add_edge(to_cycle, to0);

        assert!(!can_contract_signal(&graph, "a", false));
    }

    #[test]
    fn pure_and_place_simple_check_removes_duplicate_replacement_place() {
        let mut graph = AstgGraph::new("sample");
        add_signal(&mut graph, "a", AstgSignalKind::Input);
        add_signal(&mut graph, "z", AstgSignalKind::Output);

        let a = add_transition(&mut graph, "a+", "a");
        let ti = add_transition(&mut graph, "ti+", "z");
        let to = add_transition(&mut graph, "to+", "z");
        let p_in = graph.add_place("p_in", false);
        let p_out = graph.add_place("p_out", false);
        let duplicate = graph.add_place("duplicate", false);
        graph.add_edge(ti, p_in);
        graph.add_edge(p_in, a);
        graph.add_edge(a, p_out);
        graph.add_edge(p_out, to);
        graph.add_edge(ti, duplicate);
        graph.add_edge(duplicate, to);

        let AstgVertexId::Transition(a_id) = a else {
            unreachable!();
        };
        eliminate_transition(&mut graph, a_id);

        assert_eq!(graph.active_place_names(), vec!["duplicate"]);
        assert_eq!(graph.active_transition_names(), vec!["ti+", "to+"]);
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_c_abi_exports() {
        let source = include_str!("astg_contract.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
