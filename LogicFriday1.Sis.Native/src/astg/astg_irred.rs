use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AstgVertexId(usize);

impl AstgVertexId
{
    pub fn index(self) -> usize
    {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AstgVertexKind
{
    Place,
    Transition
    {
        delay: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AstgVertex
{
    pub name: String,
    pub kind: AstgVertexKind,
    selected: bool,
    subset: bool,
    active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgEdge
{
    pub tail: AstgVertexId,
    pub head: AstgVertexId,
    active: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AstgGraph
{
    vertices: Vec<AstgVertex>,
    edges: Vec<AstgEdge>,
    selection_name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgIrredError
{
    MissingVertex
    {
        index: usize
    },
}

impl fmt::Display for AstgIrredError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingVertex { index } => write!(f, "missing ASTG vertex {index}"),
        }
    }
}

impl Error for AstgIrredError {}

#[derive(Clone, Debug, PartialEq)]
struct CycleInfo
{
    find_longest: bool,
    which_cycle: usize,
    cycle_n: usize,
    longest_cycle: Vec<AstgVertexId>,
    its_delay: f32,
}

impl AstgGraph
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn add_place(&mut self, name: impl Into<String>) -> AstgVertexId
    {
        self.add_vertex(name, AstgVertexKind::Place)
    }

    pub fn add_transition(&mut self, name: impl Into<String>, delay: f32) -> AstgVertexId
    {
        self.add_vertex(name, AstgVertexKind::Transition { delay })
    }

    pub fn add_edge(&mut self, tail: AstgVertexId, head: AstgVertexId)
    -> Result<(), AstgIrredError>
    {
        self.require_vertex(tail)?;
        self.require_vertex(head)?;
        self.edges.push(AstgEdge {
            tail,
            head,
            active: true,
        });
        Ok(())
    }

    pub fn vertices(&self) -> &[AstgVertex]
    {
        &self.vertices
    }

    pub fn edges(&self) -> &[AstgEdge]
    {
        &self.edges
    }

    pub fn vertex_name(&self, vertex: AstgVertexId) -> Option<&str>
    {
        self.vertices
            .get(vertex.index())
            .filter(|candidate| candidate.active)
            .map(|candidate| candidate.name.as_str())
    }

    pub fn selected_vertices(&self) -> Vec<AstgVertexId>
    {
        self.vertices
            .iter()
            .enumerate()
            .filter_map(|(index, vertex)| {
                if vertex.active && vertex.selected
                {
                    Some(AstgVertexId(index))
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    pub fn selection_name(&self) -> Option<&str>
    {
        self.selection_name.as_deref()
    }

    pub fn irredundant_constraints(&mut self, modify: bool) -> Result<usize, AstgIrredError>
    {
        for vertex in &mut self.vertices
        {
            if vertex.active && matches!(vertex.kind, AstgVertexKind::Place)
            {
                vertex.subset = true;
            }
        }

        let places = self.place_ids();
        let mut redundant = Vec::new();

        for place in places
        {
            if !self.active_place(place)
            {
                continue;
            }

            let Some((tail, head)) = self.unique_constraint_endpoints(place)
            else
            {
                continue;
            };

            if self.has_alternate_cycle(tail, head, place)
            {
                redundant.push(place);

                if let Some(vertex) = self.vertices.get_mut(place.index())
                {
                    vertex.subset = false;
                }

                if modify
                {
                    self.delete_place(place);
                }
            }
        }

        Ok(redundant.len())
    }

    pub fn longest_cycle_delay(&self) -> f32
    {
        let mut cinfo = CycleInfo {
            find_longest: true,
            which_cycle: 0,
            cycle_n: 0,
            longest_cycle: Vec::new(),
            its_delay: 0.0,
        };
        let mut graph = self.clone();

        graph.check_cycles(None, &mut cinfo);
        cinfo.its_delay
    }

    pub fn select_cycle(
        &mut self,
        thru_transition: Option<AstgVertexId>,
        cycle_n: usize,
        find_longest: bool,
        add_to_set: bool,
    ) -> usize
    {
        let mut cinfo = CycleInfo {
            find_longest,
            which_cycle: cycle_n,
            cycle_n: 0,
            longest_cycle: Vec::new(),
            its_delay: 0.0,
        };

        if !find_longest && cycle_n == 0 && !add_to_set
        {
            self.select_new("simple cycles");
        }
        else if !find_longest && !add_to_set
        {
            self.select_new(format!("simple cycle {cycle_n}"));
        }

        let n_cycle = self.check_cycles(thru_transition, &mut cinfo);

        if find_longest && !cinfo.longest_cycle.is_empty()
        {
            self.select_new(format!("longest cycle {:.1}", cinfo.its_delay));

            for vertex in cinfo.longest_cycle
            {
                self.select_vertex(vertex, true);
            }
        }

        n_cycle
    }

    fn add_vertex(&mut self, name: impl Into<String>, kind: AstgVertexKind) -> AstgVertexId
    {
        let id = AstgVertexId(self.vertices.len());
        self.vertices.push(AstgVertex {
            name: name.into(),
            kind,
            selected: false,
            subset: true,
            active: true,
        });
        id
    }

    fn require_vertex(&self, vertex: AstgVertexId) -> Result<(), AstgIrredError>
    {
        match self.vertices.get(vertex.index())
        {
            Some(candidate) if candidate.active => Ok(()),
            _ => Err(AstgIrredError::MissingVertex {
                index: vertex.index(),
            }),
        }
    }

    fn place_ids(&self) -> Vec<AstgVertexId>
    {
        self.vertices
            .iter()
            .enumerate()
            .filter_map(|(index, vertex)| {
                if vertex.active && matches!(vertex.kind, AstgVertexKind::Place)
                {
                    Some(AstgVertexId(index))
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    fn active_place(&self, place: AstgVertexId) -> bool
    {
        self.vertices
            .get(place.index())
            .is_some_and(|vertex| vertex.active && matches!(vertex.kind, AstgVertexKind::Place))
    }

    fn unique_constraint_endpoints(
        &self,
        place: AstgVertexId,
    ) -> Option<(AstgVertexId, AstgVertexId)>
    {
        let incoming = self
            .edges
            .iter()
            .filter(|edge| edge.active && edge.head == place && self.edge_endpoints_active(edge))
            .collect::<Vec<_>>();
        let outgoing = self
            .edges
            .iter()
            .filter(|edge| edge.active && edge.tail == place && self.edge_endpoints_active(edge))
            .collect::<Vec<_>>();

        if incoming.len() == 1 && outgoing.len() == 1
        {
            Some((incoming[0].tail, outgoing[0].head))
        }
        else
        {
            None
        }
    }

    fn edge_endpoints_active(&self, edge: &AstgEdge) -> bool
    {
        self.vertices
            .get(edge.tail.index())
            .is_some_and(|vertex| vertex.active)
            && self
                .vertices
                .get(edge.head.index())
                .is_some_and(|vertex| vertex.active)
    }

    fn has_alternate_cycle(
        &self,
        tail: AstgVertexId,
        head: AstgVertexId,
        place: AstgVertexId,
    ) -> bool
    {
        self.simple_cycles(Some(tail), CycleFilter::Subset)
            .into_iter()
            .any(|cycle| cycle.contains(&head) && !cycle.contains(&place))
    }

    fn delete_place(&mut self, place: AstgVertexId)
    {
        if let Some(vertex) = self.vertices.get_mut(place.index())
        {
            vertex.active = false;
            vertex.selected = false;
            vertex.subset = false;
        }

        for edge in &mut self.edges
        {
            if edge.tail == place || edge.head == place
            {
                edge.active = false;
            }
        }
    }

    fn check_cycles(&mut self, start: Option<AstgVertexId>, cinfo: &mut CycleInfo) -> usize
    {
        let cycles = self.simple_cycles(start, CycleFilter::All);
        let mut n_found = 0;

        for cycle in cycles
        {
            cinfo.cycle_n += 1;

            if cinfo.find_longest
            {
                let cycle_delay = self.calculate_delay(&cycle);
                n_found += 1;

                if cinfo.longest_cycle.is_empty() || cycle_delay > cinfo.its_delay
                {
                    cinfo.longest_cycle = cycle;
                    cinfo.its_delay = cycle_delay;
                }
            }
            else if cinfo.cycle_n == cinfo.which_cycle || cinfo.which_cycle == 0
            {
                n_found += 1;

                for vertex in cycle
                {
                    self.select_vertex(vertex, true);
                }
            }
        }

        n_found
    }

    fn calculate_delay(&self, cycle: &[AstgVertexId]) -> f32
    {
        cycle
            .iter()
            .filter_map(|vertex| self.vertices.get(vertex.index()))
            .map(|vertex| match vertex.kind
            {
                AstgVertexKind::Transition { delay } => delay,
                AstgVertexKind::Place => 0.0,
            })
            .sum()
    }

    fn select_new(&mut self, name: impl Into<String>)
    {
        self.selection_name = Some(name.into());

        for vertex in &mut self.vertices
        {
            vertex.selected = false;
        }
    }

    fn select_vertex(&mut self, vertex: AstgVertexId, selected: bool)
    {
        if let Some(candidate) = self.vertices.get_mut(vertex.index())
        {
            if candidate.active
            {
                candidate.selected = selected;
            }
        }
    }

    fn simple_cycles(
        &self,
        start: Option<AstgVertexId>,
        filter: CycleFilter,
    ) -> Vec<Vec<AstgVertexId>>
    {
        let starts = match start
        {
            Some(vertex) if self.vertex_allowed(vertex, filter) => vec![vertex],
            Some(_) => Vec::new(),
            None => self
                .vertices
                .iter()
                .enumerate()
                .filter_map(|(index, _)| {
                    let vertex = AstgVertexId(index);

                    if self.vertex_allowed(vertex, filter)
                    {
                        Some(vertex)
                    }
                    else
                    {
                        None
                    }
                })
                .collect(),
        };
        let mut cycles = Vec::new();
        let mut seen = HashSet::new();

        for root in starts
        {
            let mut path = vec![root];
            let mut on_path = HashSet::from([root]);
            self.visit_cycles(
                root,
                root,
                filter,
                &mut path,
                &mut on_path,
                &mut cycles,
                &mut seen,
            );
        }

        cycles
    }

    fn visit_cycles(
        &self,
        root: AstgVertexId,
        current: AstgVertexId,
        filter: CycleFilter,
        path: &mut Vec<AstgVertexId>,
        on_path: &mut HashSet<AstgVertexId>,
        cycles: &mut Vec<Vec<AstgVertexId>>,
        seen: &mut HashSet<Vec<usize>>,
    )
    {
        for next in self.successors(current, filter)
        {
            if next == root && path.len() > 1
            {
                let key = canonical_cycle_key(path);

                if seen.insert(key)
                {
                    cycles.push(path.clone());
                }
            }
            else if !on_path.contains(&next)
            {
                path.push(next);
                on_path.insert(next);
                self.visit_cycles(root, next, filter, path, on_path, cycles, seen);
                on_path.remove(&next);
                path.pop();
            }
        }
    }

    fn successors(&self, vertex: AstgVertexId, filter: CycleFilter) -> Vec<AstgVertexId>
    {
        self.edges
            .iter()
            .filter_map(|edge| {
                if edge.active && edge.tail == vertex && self.vertex_allowed(edge.head, filter)
                {
                    Some(edge.head)
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    fn vertex_allowed(&self, vertex: AstgVertexId, filter: CycleFilter) -> bool
    {
        self.vertices.get(vertex.index()).is_some_and(|candidate| {
            candidate.active
                && match filter
                {
                    CycleFilter::All => true,
                    CycleFilter::Subset => match candidate.kind
                    {
                        AstgVertexKind::Place => candidate.subset,
                        AstgVertexKind::Transition { .. } => true,
                    },
                }
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CycleFilter
{
    All,
    Subset,
}

fn canonical_cycle_key(cycle: &[AstgVertexId]) -> Vec<usize>
{
    let indexes = cycle
        .iter()
        .map(|vertex| vertex.index())
        .collect::<Vec<_>>();
    let best_start = (0..indexes.len())
        .min_by_key(|index| {
            (0..indexes.len())
                .map(|offset| indexes[(index + offset) % indexes.len()])
                .collect::<Vec<_>>()
        })
        .unwrap_or(0);

    (0..indexes.len())
        .map(|offset| indexes[(best_start + offset) % indexes.len()])
        .collect()
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn add_edge(graph: &mut AstgGraph, tail: AstgVertexId, head: AstgVertexId)
    {
        graph.add_edge(tail, head).unwrap();
    }

    fn redundant_constraint_graph() -> (AstgGraph, AstgVertexId, AstgVertexId)
    {
        let mut graph = AstgGraph::new();
        let t0 = graph.add_transition("t0", 1.0);
        let p0 = graph.add_place("p0");
        let t1 = graph.add_transition("t1", 2.0);
        let p1 = graph.add_place("p1");
        let p2 = graph.add_place("p2");

        add_edge(&mut graph, t0, p0);
        add_edge(&mut graph, p0, t1);
        add_edge(&mut graph, t1, p1);
        add_edge(&mut graph, p1, t0);
        add_edge(&mut graph, t0, p2);
        add_edge(&mut graph, p2, t1);

        (graph, p0, p2)
    }

    #[test]
    fn irredundant_constraints_counts_alternate_simple_cycle_without_modifying_graph()
    {
        let (mut graph, redundant_place, _) = redundant_constraint_graph();

        let count = graph.irredundant_constraints(false).unwrap();

        assert_eq!(count, 1);
        assert!(graph.vertices()[redundant_place.index()].active);
    }

    #[test]
    fn irredundant_constraints_deletes_redundant_place_when_requested()
    {
        let (mut graph, redundant_place, kept_place) = redundant_constraint_graph();

        let count = graph.irredundant_constraints(true).unwrap();

        assert_eq!(count, 1);
        assert!(!graph.vertices()[redundant_place.index()].active);
        assert!(graph.vertices()[kept_place.index()].active);
        assert!(
            graph
                .edges()
                .iter()
                .filter(|edge| edge.tail == redundant_place || edge.head == redundant_place)
                .all(|edge| !edge.active)
        );
    }

    #[test]
    fn irredundant_constraints_requires_single_input_and_output_place()
    {
        let mut graph = AstgGraph::new();
        let t0 = graph.add_transition("t0", 1.0);
        let t1 = graph.add_transition("t1", 1.0);
        let t2 = graph.add_transition("t2", 1.0);
        let p0 = graph.add_place("p0");
        let p1 = graph.add_place("p1");

        add_edge(&mut graph, t0, p0);
        add_edge(&mut graph, p0, t1);
        add_edge(&mut graph, p0, t2);
        add_edge(&mut graph, t1, p1);
        add_edge(&mut graph, p1, t0);

        assert_eq!(graph.irredundant_constraints(true).unwrap(), 0);
        assert!(graph.vertices()[p0.index()].active);
    }

    #[test]
    fn longest_cycle_delay_sums_transition_delays()
    {
        let mut graph = AstgGraph::new();
        let t0 = graph.add_transition("t0", 1.25);
        let p0 = graph.add_place("p0");
        let t1 = graph.add_transition("t1", 2.5);
        let p1 = graph.add_place("p1");
        let t2 = graph.add_transition("t2", 4.0);
        let p2 = graph.add_place("p2");

        add_edge(&mut graph, t0, p0);
        add_edge(&mut graph, p0, t1);
        add_edge(&mut graph, t1, p1);
        add_edge(&mut graph, p1, t0);
        add_edge(&mut graph, t1, p2);
        add_edge(&mut graph, p2, t2);
        add_edge(&mut graph, t2, p1);

        assert_eq!(graph.longest_cycle_delay(), 7.75);
    }

    #[test]
    fn select_cycle_marks_requested_cycle_or_longest_cycle()
    {
        let mut graph = AstgGraph::new();
        let t0 = graph.add_transition("t0", 1.0);
        let p0 = graph.add_place("p0");
        let t1 = graph.add_transition("t1", 5.0);
        let p1 = graph.add_place("p1");

        add_edge(&mut graph, t0, p0);
        add_edge(&mut graph, p0, t1);
        add_edge(&mut graph, t1, p1);
        add_edge(&mut graph, p1, t0);

        assert_eq!(graph.select_cycle(None, 1, false, false), 1);
        assert_eq!(graph.selection_name(), Some("simple cycle 1"));
        assert_eq!(graph.selected_vertices().len(), 4);

        assert_eq!(graph.select_cycle(None, 0, true, false), 1);
        assert_eq!(graph.selection_name(), Some("longest cycle 6.0"));
        assert_eq!(graph.selected_vertices().len(), 4);
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_c_abi_exports()
    {
        let source = include_str!("astg_irred.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
