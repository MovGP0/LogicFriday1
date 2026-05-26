use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind
{
    Input,
    Output,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionPolarity
{
    Rising,
    Falling,
}

impl TransitionPolarity
{
    fn is_opposite(self, other: Self) -> bool
    {
        matches!(
            (self, other),
            (Self::Rising, Self::Falling) | (Self::Falling, Self::Rising)
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal
{
    pub name: String,
    pub kind: SignalKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition
{
    pub name: String,
    pub signal: usize,
    pub polarity: TransitionPolarity,
    pub opposite: Option<VertexId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Place
{
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VertexKind
{
    Transition(Transition),
    Place(Place),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vertex
{
    pub kind: VertexKind,
    pub subset: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistReport
{
    pub constraints_added: usize,
    pub nonpersistent: Vec<(VertexId, VertexId)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PersistError
{
    UnknownVertex(VertexId),
    UnknownSignal(usize),
    ExpectedTransition(VertexId),
    ExpectedPlace(VertexId),
    NoOppositeTransition(VertexId),
    SamePolarityOpposite
    {
        left: VertexId,
        right: VertexId,
    },
    Disconnected
    {
        components: usize,
    },
    MissingNoninputTrigger
    {
        transition: VertexId,
    },
}

impl fmt::Display for PersistError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::UnknownVertex(vertex) => write!(f, "unknown ASTG vertex {}", vertex.0),
            Self::UnknownSignal(signal) => write!(f, "unknown ASTG signal {signal}"),
            Self::ExpectedTransition(vertex) =>
            {
                write!(f, "ASTG vertex {} is not a transition", vertex.0)
            }
            Self::ExpectedPlace(vertex) => write!(f, "ASTG vertex {} is not a place", vertex.0),
            Self::NoOppositeTransition(vertex) =>
            {
                write!(f, "no opposite transition found for vertex {}", vertex.0)
            }
            Self::SamePolarityOpposite
            {
                left,
                right,
            } => write!(
                f,
                "transitions {} and {} have the same polarity",
                left.0, right.0
            ),
            Self::Disconnected
            {
                components,
            } => write!(f, "ASTG graph is disconnected with {components} components"),
            Self::MissingNoninputTrigger
            {
                transition,
            } => write!(
                f,
                "cannot find a non-input transition that enables vertex {}",
                transition.0
            ),
        }
    }
}

impl Error for PersistError
{
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AstgGraph
{
    name: String,
    signals: Vec<Signal>,
    vertices: Vec<Vertex>,
    edges: Vec<(VertexId, VertexId)>,
}

impl AstgGraph
{
    pub fn new(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            signals: Vec::new(),
            vertices: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn name(&self) -> &str
    {
        &self.name
    }

    pub fn signals(&self) -> &[Signal]
    {
        &self.signals
    }

    pub fn vertices(&self) -> &[Vertex]
    {
        &self.vertices
    }

    pub fn edges(&self) -> &[(VertexId, VertexId)]
    {
        &self.edges
    }

    pub fn add_signal(&mut self, name: impl Into<String>, kind: SignalKind) -> usize
    {
        let index = self.signals.len();
        self.signals.push(Signal
        {
            name: name.into(),
            kind,
        });
        index
    }

    pub fn add_transition(
        &mut self,
        name: impl Into<String>,
        signal: usize,
        polarity: TransitionPolarity,
    ) -> Result<VertexId, PersistError>
    {
        if signal >= self.signals.len()
        {
            return Err(PersistError::UnknownSignal(signal));
        }

        let id = VertexId(self.vertices.len());
        self.vertices.push(Vertex
        {
            kind: VertexKind::Transition(Transition
            {
                name: name.into(),
                signal,
                polarity,
                opposite: None,
            }),
            subset: true,
            selected: false,
        });
        Ok(id)
    }

    pub fn add_place(&mut self, name: impl Into<String>) -> VertexId
    {
        let id = VertexId(self.vertices.len());
        self.vertices.push(Vertex
        {
            kind: VertexKind::Place(Place
            {
                name: name.into(),
            }),
            subset: true,
            selected: false,
        });
        id
    }

    pub fn add_edge(&mut self, from: VertexId, to: VertexId) -> Result<(), PersistError>
    {
        self.require_vertex(from)?;
        self.require_vertex(to)?;
        self.edges.push((from, to));
        Ok(())
    }

    pub fn set_subset(&mut self, vertex: VertexId, subset: bool) -> Result<(), PersistError>
    {
        self.require_vertex(vertex)?;
        self.vertices[vertex.0].subset = subset;
        Ok(())
    }

    pub fn selected_transitions(&self) -> Vec<VertexId>
    {
        self.vertices
            .iter()
            .enumerate()
            .filter_map(|(index, vertex)|
            {
                if vertex.selected && matches!(vertex.kind, VertexKind::Transition(_))
                {
                    Some(VertexId(index))
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    pub fn transition(&self, vertex: VertexId) -> Result<&Transition, PersistError>
    {
        self.require_vertex(vertex)?;
        match &self.vertices[vertex.0].kind
        {
            VertexKind::Transition(transition) => Ok(transition),
            VertexKind::Place(_) => Err(PersistError::ExpectedTransition(vertex)),
        }
    }

    fn transition_mut(&mut self, vertex: VertexId) -> Result<&mut Transition, PersistError>
    {
        self.require_vertex(vertex)?;
        match &mut self.vertices[vertex.0].kind
        {
            VertexKind::Transition(transition) => Ok(transition),
            VertexKind::Place(_) => Err(PersistError::ExpectedTransition(vertex)),
        }
    }

    fn require_vertex(&self, vertex: VertexId) -> Result<(), PersistError>
    {
        if vertex.0 < self.vertices.len()
        {
            Ok(())
        }
        else
        {
            Err(PersistError::UnknownVertex(vertex))
        }
    }

    fn transition_ids(&self, subset_only: bool) -> Vec<VertexId>
    {
        self.vertices
            .iter()
            .enumerate()
            .filter_map(|(index, vertex)|
            {
                if matches!(vertex.kind, VertexKind::Transition(_))
                    && (!subset_only || vertex.subset)
                {
                    Some(VertexId(index))
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    fn output_places(&self, transition: VertexId, subset_only: bool) -> Vec<VertexId>
    {
        self.edges
            .iter()
            .filter_map(|(from, to)|
            {
                if *from == transition
                    && self.vertex_is_place(*to)
                    && (!subset_only || self.vertices[to.0].subset)
                {
                    Some(*to)
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    fn first_output_transition(&self, place: VertexId, subset_only: bool) -> Option<VertexId>
    {
        self.edges.iter().find_map(|(from, to)|
        {
            if *from == place
                && self.vertex_is_transition(*to)
                && (!subset_only || self.vertices[to.0].subset)
            {
                Some(*to)
            }
            else
            {
                None
            }
        })
    }

    fn output_degree(&self, vertex: VertexId, subset_only: bool) -> usize
    {
        self.edges
            .iter()
            .filter(|(from, to)|
            {
                *from == vertex && (!subset_only || self.vertices[to.0].subset)
            })
            .count()
    }

    fn vertex_is_transition(&self, vertex: VertexId) -> bool
    {
        matches!(self.vertices[vertex.0].kind, VertexKind::Transition(_))
    }

    fn vertex_is_place(&self, vertex: VertexId) -> bool
    {
        matches!(self.vertices[vertex.0].kind, VertexKind::Place(_))
    }

    fn successors(&self, vertex: VertexId, subset_only: bool) -> Vec<VertexId>
    {
        self.edges
            .iter()
            .filter_map(|(from, to)|
            {
                if *from == vertex && (!subset_only || self.vertices[to.0].subset)
                {
                    Some(*to)
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    fn predecessors(&self, vertex: VertexId, subset_only: bool) -> Vec<VertexId>
    {
        self.edges
            .iter()
            .filter_map(|(from, to)|
            {
                if *to == vertex && (!subset_only || self.vertices[from.0].subset)
                {
                    Some(*from)
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    fn undirected_neighbors(&self, vertex: VertexId) -> Vec<VertexId>
    {
        let mut neighbors = Vec::new();

        for (from, to) in &self.edges
        {
            if *from == vertex
            {
                neighbors.push(*to);
            }
            else if *to == vertex
            {
                neighbors.push(*from);
            }
        }

        neighbors
    }

    fn connected_components(&self) -> usize
    {
        if self.vertices.is_empty()
        {
            return 0;
        }

        let mut visited = vec![false; self.vertices.len()];
        let mut components = 0;

        for start in 0..self.vertices.len()
        {
            if visited[start]
            {
                continue;
            }

            components += 1;
            let mut stack = vec![VertexId(start)];
            visited[start] = true;

            while let Some(vertex) = stack.pop()
            {
                for neighbor in self.undirected_neighbors(vertex)
                {
                    if !visited[neighbor.0]
                    {
                        visited[neighbor.0] = true;
                        stack.push(neighbor);
                    }
                }
            }
        }

        components
    }
}

pub fn set_opposite_transitions(
    graph: &mut AstgGraph,
    subset_only: bool,
) -> Result<(), PersistError>
{
    let transition_ids = graph.transition_ids(subset_only);
    let mut signal_counts = vec![0usize; graph.signals.len()];

    for transition_id in &transition_ids
    {
        let signal = graph.transition(*transition_id)?.signal;
        signal_counts[signal] += 1;
        graph.transition_mut(*transition_id)?.opposite = None;
    }

    let cycles = simple_cycles(graph, subset_only);

    for cycle in cycles
    {
        let cycle_transitions = cycle
            .iter()
            .copied()
            .filter(|vertex| graph.vertex_is_transition(*vertex))
            .collect::<Vec<_>>();
        let mut found = vec![0usize; graph.signals.len()];

        for transition_id in &cycle_transitions
        {
            found[graph.transition(*transition_id)?.signal] += 1;
        }

        for signal in 0..graph.signals.len()
        {
            if signal_counts[signal] == 0 || found[signal] != signal_counts[signal]
            {
                continue;
            }

            let matching = cycle_transitions
                .iter()
                .copied()
                .filter(|transition_id| graph.transition(*transition_id).unwrap().signal == signal)
                .collect::<Vec<_>>();

            if matching.len() != 2
            {
                continue;
            }

            let left = matching[0];
            let right = matching[1];
            let left_polarity = graph.transition(left)?.polarity;
            let right_polarity = graph.transition(right)?.polarity;

            if !left_polarity.is_opposite(right_polarity)
            {
                return Err(PersistError::SamePolarityOpposite
                {
                    left,
                    right,
                });
            }

            graph.transition_mut(left)?.opposite = Some(right);
            graph.transition_mut(right)?.opposite = Some(left);
        }
    }

    for transition_id in transition_ids
    {
        if graph.transition(transition_id)?.opposite.is_none()
        {
            return Err(PersistError::NoOppositeTransition(transition_id));
        }
    }

    Ok(())
}

pub fn make_persistent(
    graph: &mut AstgGraph,
    modify: bool,
) -> Result<PersistReport, PersistError>
{
    let components = graph.connected_components();

    if components != 1
    {
        return Err(PersistError::Disconnected
        {
            components,
        });
    }

    make_marked_component_persistent(graph, modify)
}

fn make_marked_component_persistent(
    graph: &mut AstgGraph,
    modify: bool,
) -> Result<PersistReport, PersistError>
{
    set_opposite_transitions(graph, true)?;

    let mut report = PersistReport
    {
        constraints_added: 0,
        nonpersistent: Vec::new(),
    };
    let mut stack = graph
        .transition_ids(true)
        .into_iter()
        .filter(|transition| graph.output_degree(*transition, true) > 1)
        .collect::<Vec<_>>();

    while let Some(transition) = stack.pop()
    {
        let opposite = graph
            .transition(transition)?
            .opposite
            .ok_or(PersistError::NoOppositeTransition(transition))?;

        for place in graph.output_places(transition, true)
        {
            let successor = graph
                .first_output_transition(place, true)
                .ok_or(PersistError::ExpectedTransition(place))?;

            if contains_cycle_with(graph, successor, opposite, true)
            {
                continue;
            }

            graph.vertices[transition.0].selected = true;
            report.nonpersistent.push((transition, successor));

            if modify
            {
                if let Some(destination) = noninput_trigger(graph, opposite)?
                {
                    add_constraint(graph, successor, destination)?;
                    report.constraints_added += 1;
                    stack.push(successor);
                }
            }
        }
    }

    Ok(report)
}

fn noninput_trigger(
    graph: &AstgGraph,
    transition: VertexId,
) -> Result<Option<VertexId>, PersistError>
{
    graph.transition(transition)?;

    for input_place in graph.predecessors(transition, true)
    {
        if !graph.vertex_is_place(input_place)
        {
            continue;
        }

        for predecessor in graph.predecessors(input_place, true)
        {
            if !graph.vertex_is_transition(predecessor)
            {
                continue;
            }

            let predecessor_transition = graph.transition(predecessor)?;
            let signal = graph
                .signals
                .get(predecessor_transition.signal)
                .ok_or(PersistError::UnknownSignal(predecessor_transition.signal))?;

            if signal.kind != SignalKind::Input
            {
                return Ok(Some(predecessor));
            }
        }
    }

    Ok(None)
}

fn add_constraint(
    graph: &mut AstgGraph,
    from: VertexId,
    to: VertexId,
) -> Result<(), PersistError>
{
    graph.transition(from)?;
    graph.transition(to)?;

    let place = graph.add_place(format!("persist_{}_{}", from.0, to.0));
    graph.add_edge(from, place)?;
    graph.add_edge(place, to)?;
    Ok(())
}

fn contains_cycle_with(
    graph: &AstgGraph,
    left: VertexId,
    right: VertexId,
    subset_only: bool,
) -> bool
{
    simple_cycles(graph, subset_only)
        .iter()
        .any(|cycle| cycle.contains(&left) && cycle.contains(&right))
}

fn simple_cycles(graph: &AstgGraph, subset_only: bool) -> Vec<Vec<VertexId>>
{
    let mut cycles = Vec::new();
    let mut seen = HashSet::new();

    for start in 0..graph.vertices.len()
    {
        if subset_only && !graph.vertices[start].subset
        {
            continue;
        }

        let start_id = VertexId(start);
        let mut path = Vec::new();
        let mut on_path = vec![false; graph.vertices.len()];
        collect_cycles(
            graph,
            start_id,
            start_id,
            subset_only,
            &mut path,
            &mut on_path,
            &mut cycles,
            &mut seen,
        );
    }

    cycles
}

fn collect_cycles(
    graph: &AstgGraph,
    start: VertexId,
    current: VertexId,
    subset_only: bool,
    path: &mut Vec<VertexId>,
    on_path: &mut [bool],
    cycles: &mut Vec<Vec<VertexId>>,
    seen: &mut HashSet<Vec<usize>>,
)
{
    path.push(current);
    on_path[current.0] = true;

    for successor in graph.successors(current, subset_only)
    {
        if successor == start && path.len() > 1
        {
            let mut key = path.iter().map(|vertex| vertex.0).collect::<Vec<_>>();
            key.sort_unstable();

            if seen.insert(key)
            {
                cycles.push(path.clone());
            }
        }
        else if !on_path[successor.0] && successor.0 >= start.0
        {
            collect_cycles(
                graph,
                start,
                successor,
                subset_only,
                path,
                on_path,
                cycles,
                seen,
            );
        }
    }

    on_path[current.0] = false;
    path.pop();
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn add_edge(graph: &mut AstgGraph, from: VertexId, to: VertexId)
    {
        graph.add_edge(from, to).unwrap();
    }

    fn branched_graph() -> (AstgGraph, VertexId, VertexId, VertexId)
    {
        let mut graph = AstgGraph::new("g");
        let a = graph.add_signal("a", SignalKind::Output);
        let b = graph.add_signal("b", SignalKind::Output);
        let c = graph.add_signal("c", SignalKind::Input);
        let a_plus = graph
            .add_transition("a+", a, TransitionPolarity::Rising)
            .unwrap();
        let b_plus = graph
            .add_transition("b+", b, TransitionPolarity::Rising)
            .unwrap();
        let a_minus = graph
            .add_transition("a-", a, TransitionPolarity::Falling)
            .unwrap();
        let b_minus = graph
            .add_transition("b-", b, TransitionPolarity::Falling)
            .unwrap();
        let c_plus = graph
            .add_transition("c+", c, TransitionPolarity::Rising)
            .unwrap();
        let c_minus = graph
            .add_transition("c-", c, TransitionPolarity::Falling)
            .unwrap();
        let p0 = graph.add_place("p0");
        let p1 = graph.add_place("p1");
        let p2 = graph.add_place("p2");
        let p3 = graph.add_place("p3");
        let p4 = graph.add_place("p4");
        let p5 = graph.add_place("p5");
        let p6 = graph.add_place("p6");

        add_edge(&mut graph, a_plus, p0);
        add_edge(&mut graph, p0, b_plus);
        add_edge(&mut graph, b_plus, p1);
        add_edge(&mut graph, p1, a_minus);
        add_edge(&mut graph, a_minus, p2);
        add_edge(&mut graph, p2, b_minus);
        add_edge(&mut graph, b_minus, p3);
        add_edge(&mut graph, p3, a_plus);

        add_edge(&mut graph, a_plus, p4);
        add_edge(&mut graph, p4, c_plus);
        add_edge(&mut graph, c_plus, p5);
        add_edge(&mut graph, p5, c_minus);
        add_edge(&mut graph, c_minus, p6);
        add_edge(&mut graph, p6, a_plus);

        (graph, a_plus, c_plus, a_minus)
    }

    #[test]
    fn set_opposite_transitions_pairs_signal_edges_from_cycles()
    {
        let (mut graph, a_plus, _, a_minus) = branched_graph();

        set_opposite_transitions(&mut graph, true).unwrap();

        assert_eq!(graph.transition(a_plus).unwrap().opposite, Some(a_minus));
        assert_eq!(graph.transition(a_minus).unwrap().opposite, Some(a_plus));
    }

    #[test]
    fn make_persistent_reports_nonpersistent_branch_without_modifying()
    {
        let (mut graph, a_plus, c_plus, _) = branched_graph();

        let report = make_persistent(&mut graph, false).unwrap();

        assert_eq!(report.constraints_added, 0);
        assert!(report.nonpersistent.contains(&(a_plus, c_plus)));
        assert_eq!(graph.selected_transitions(), vec![a_plus]);
    }

    #[test]
    fn make_persistent_adds_constraint_to_noninput_trigger()
    {
        let (mut graph, _, c_plus, a_minus) = branched_graph();
        let initial_edges = graph.edges().len();
        let initial_vertices = graph.vertices().len();

        let report = make_persistent(&mut graph, true).unwrap();

        assert_eq!(report.constraints_added, 1);
        assert_eq!(graph.vertices().len(), initial_vertices + 1);
        assert_eq!(graph.edges().len(), initial_edges + 2);
        assert!(graph.edges().iter().any(|(from, _)| *from == c_plus));
        assert!(graph.edges().iter().any(|(_, to)| *to == a_minus));
    }

    #[test]
    fn make_persistent_rejects_disconnected_graph()
    {
        let mut graph = AstgGraph::new("disconnected");
        let signal = graph.add_signal("a", SignalKind::Output);
        graph
            .add_transition("a+", signal, TransitionPolarity::Rising)
            .unwrap();
        graph.add_place("p");

        let error = make_persistent(&mut graph, false).unwrap_err();

        assert_eq!(error, PersistError::Disconnected
        {
            components: 2,
        });
    }

    #[test]
    fn metadata_and_legacy_abi_are_absent()
    {
        let source = include_str!("astg_persist.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
