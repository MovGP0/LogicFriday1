use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type StateCode = u64;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexKind
{
    Place,
    Transition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgVertex
{
    pub name: String,
    pub kind: VertexKind,
    pub initial_token: bool,
}

impl AstgVertex
{
    pub fn place(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            kind: VertexKind::Place,
            initial_token: false,
        }
    }

    pub fn marked_place(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            kind: VertexKind::Place,
            initial_token: true,
        }
    }

    pub fn transition(name: impl Into<String>) -> Self
    {
        Self
        {
            name: name.into(),
            kind: VertexKind::Transition,
            initial_token: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgMarking
{
    pub places: Vec<VertexId>,
}

impl AstgMarking
{
    pub fn new(places: Vec<VertexId>) -> Self
    {
        Self
        {
            places,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgState
{
    pub code: StateCode,
    pub markings: Vec<AstgMarking>,
}

impl AstgState
{
    pub fn new(code: StateCode, markings: Vec<AstgMarking>) -> Self
    {
        Self
        {
            code,
            markings,
        }
    }

    pub fn marking_count(&self) -> usize
    {
        self.markings.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgSignal
{
    pub name: String,
    pub bit: StateCode,
}

impl AstgSignal
{
    pub fn new(name: impl Into<String>, bit: StateCode) -> Self
    {
        Self
        {
            name: name.into(),
            bit,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenFlowStatus
{
    Ok,
    NotUsc,
    NotSafe,
    NoMarking,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgGraph
{
    pub vertices: Vec<AstgVertex>,
    pub state_machine_components: Vec<Vec<VertexId>>,
    pub has_marking: bool,
    pub change_count: usize,
    pub states: Vec<AstgState>,
    pub initial_state: Option<usize>,
    pub signals: Vec<AstgSignal>,
    pub token_flow_status: TokenFlowStatus,
    pub code_adjustment: StateCode,
}

impl Default for AstgGraph
{
    fn default() -> Self
    {
        Self
        {
            vertices: Vec::new(),
            state_machine_components: Vec::new(),
            has_marking: false,
            change_count: 0,
            states: Vec::new(),
            initial_state: None,
            signals: Vec::new(),
            token_flow_status: TokenFlowStatus::Ok,
            code_adjustment: 0,
        }
    }
}

impl AstgGraph
{
    pub fn add_vertex(&mut self, vertex: AstgVertex) -> VertexId
    {
        let id = VertexId(self.vertices.len());
        self.vertices.push(vertex);
        id
    }

    pub fn add_state_machine_component(&mut self, vertices: Vec<VertexId>)
    {
        self.state_machine_components.push(vertices);
    }

    pub fn add_signal(&mut self, signal: AstgSignal)
    {
        self.signals.push(signal);
    }

    pub fn add_state(&mut self, state: AstgState) -> usize
    {
        let index = self.states.len();
        self.states.push(state);
        index
    }

    pub fn marking(&self) -> Vec<VertexId>
    {
        self.vertices
            .iter()
            .enumerate()
            .filter_map(|(index, vertex)|
            {
                (vertex.kind == VertexKind::Place && vertex.initial_token)
                    .then_some(VertexId(index))
            })
            .collect()
    }

    pub fn set_marking(&mut self, marking: &AstgMarking) -> Result<(), AstgMarkingError>
    {
        for vertex in &mut self.vertices
        {
            if vertex.kind == VertexKind::Place
            {
                vertex.initial_token = false;
            }
        }

        for place in &marking.places
        {
            let vertex = self
                .vertices
                .get_mut(place.0)
                .ok_or(AstgMarkingError::UnknownVertex(*place))?;

            if vertex.kind != VertexKind::Place
            {
                return Err(AstgMarkingError::NotAPlace(*place));
            }

            vertex.initial_token = true;
        }

        self.has_marking = true;
        self.change_count += 1;
        Ok(())
    }

    fn place_has_token(&self, vertex: VertexId) -> bool
    {
        self.vertices
            .get(vertex.0)
            .is_some_and(|item| item.kind == VertexKind::Place && item.initial_token)
    }

    fn signal_by_name(&self, name: &str) -> Option<&AstgSignal>
    {
        self.signals.iter().find(|signal| signal.name == name)
    }

    fn adjusted_code(&self, state_code: StateCode) -> StateCode
    {
        state_code ^ self.code_adjustment
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgMarkingError
{
    UnknownVertex(VertexId),
    NotAPlace(VertexId),
    InvalidStateMachineComponent
    {
        component: usize,
        token_count: usize,
    },
    NoOneTokenStateMachineMarking,
    NoInitialState,
    NoUniqueStateCode,
    BadSignalName(String),
    TokenFlow(TokenFlowStatus),
}

impl fmt::Display for AstgMarkingError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::UnknownVertex(vertex) => write!(formatter, "unknown ASTG vertex {}", vertex.0),
            Self::NotAPlace(vertex) => write!(formatter, "ASTG vertex {} is not a place", vertex.0),
            Self::InvalidStateMachineComponent
            {
                component,
                token_count,
            } => write!(
                formatter,
                "state machine component {} has {} tokens",
                component + 1,
                token_count,
            ),
            Self::NoOneTokenStateMachineMarking =>
            {
                formatter.write_str("no 1-token state machine marking")
            }
            Self::NoInitialState => formatter.write_str("no initial state code"),
            Self::NoUniqueStateCode => formatter.write_str("no unique state code was found"),
            Self::BadSignalName(name) => write!(formatter, "no signal {name:?}"),
            Self::TokenFlow(status) => write!(formatter, "token flow failed with {status:?}"),
        }
    }
}

impl Error for AstgMarkingError
{
}

pub fn valid_marking(stg: &AstgGraph) -> Result<(), AstgMarkingError>
{
    for (component_index, component) in stg.state_machine_components.iter().enumerate()
    {
        let token_count = component
            .iter()
            .filter(|vertex| stg.place_has_token(**vertex))
            .count();

        if token_count != 1
        {
            return Err(AstgMarkingError::InvalidStateMachineComponent
            {
                component: component_index,
                token_count,
            });
        }
    }

    Ok(())
}

pub fn one_sm_token(stg: &mut AstgGraph) -> Result<(), AstgMarkingError>
{
    if stg.has_marking
    {
        valid_marking(stg)
    }
    else
    {
        find_marking(stg)
    }
}

pub fn find_marking(stg: &mut AstgGraph) -> Result<(), AstgMarkingError>
{
    let mut vertex_is_available = vec![true; stg.vertices.len()];

    stg.has_marking = false;
    stg.change_count += 1;
    for vertex in &mut stg.vertices
    {
        if vertex.kind == VertexKind::Place
        {
            vertex.initial_token = false;
        }
    }

    let mut unmarked_components = (0..stg.state_machine_components.len()).collect::<Vec<_>>();

    while let Some(component_index) = unmarked_components.first().copied()
    {
        unmarked_components.remove(0);

        let component = &stg.state_machine_components[component_index];
        let Some(new_marked) = component.iter().copied().find(|vertex|
        {
            stg.vertices
                .get(vertex.0)
                .is_some_and(|item| item.kind == VertexKind::Place && vertex_is_available[vertex.0])
        })
        else
        {
            return Err(AstgMarkingError::NoOneTokenStateMachineMarking);
        };

        stg.vertices[new_marked.0].initial_token = true;

        for vertex in component
        {
            if stg
                .vertices
                .get(vertex.0)
                .is_some_and(|item| item.kind == VertexKind::Place)
            {
                vertex_is_available[vertex.0] = false;
            }
        }

        unmarked_components.retain(|candidate_index|
        {
            let has_new_token = stg.state_machine_components[*candidate_index]
                .iter()
                .any(|vertex| *vertex == new_marked);

            if has_new_token
            {
                for vertex in &stg.state_machine_components[*candidate_index]
                {
                    if stg
                        .vertices
                        .get(vertex.0)
                        .is_some_and(|item| item.kind == VertexKind::Place)
                    {
                        vertex_is_available[vertex.0] = false;
                    }
                }
            }

            !has_new_token
        });
    }

    stg.has_marking = true;
    Ok(())
}

pub fn initial_state(stg: &mut AstgGraph) -> Result<StateCode, AstgMarkingError>
{
    if !stg.has_marking
    {
        one_sm_token(stg)?;
    }

    run_token_flow(stg)?;

    let state_index = stg.initial_state.ok_or(AstgMarkingError::NoInitialState)?;
    let state = stg
        .states
        .get(state_index)
        .ok_or(AstgMarkingError::NoInitialState)?;

    Ok(stg.adjusted_code(state.code))
}

pub fn unique_state(stg: &mut AstgGraph) -> Result<StateCode, AstgMarkingError>
{
    match stg.token_flow_status
    {
        TokenFlowStatus::Ok =>
        {
            run_token_flow(stg)?;
            let state_index = stg.initial_state.ok_or(AstgMarkingError::NoInitialState)?;
            let state = stg
                .states
                .get(state_index)
                .ok_or(AstgMarkingError::NoInitialState)?;

            Ok(stg.adjusted_code(state.code))
        }
        TokenFlowStatus::NotUsc =>
        {
            run_token_flow(stg)?;
            stg.states
                .iter()
                .find(|state| state.marking_count() == 1)
                .map(|state| state.code)
                .ok_or(AstgMarkingError::NoUniqueStateCode)
        }
        status => Err(AstgMarkingError::TokenFlow(status)),
    }
}

pub fn set_marking_by_code(
    stg: &mut AstgGraph,
    state_code: StateCode,
    signal_mask: StateCode,
) -> Result<(), AstgMarkingError>
{
    run_token_flow_allowing_not_usc(stg)?;

    let marking = stg
        .states
        .iter()
        .find(|state| ((!signal_mask) & (state_code ^ state.code)) == 0)
        .and_then(|state| state.markings.first())
        .cloned()
        .ok_or(AstgMarkingError::NoOneTokenStateMachineMarking)?;

    stg.set_marking(&marking)
}

pub fn set_marking_by_name(
    stg: &mut AstgGraph,
    signal_values: &HashMap<String, bool>,
) -> Result<(), AstgMarkingError>
{
    run_token_flow_allowing_not_usc(stg)?;

    let mut state_code = 0;
    let mut signal_mask = 0;

    for (name, value) in signal_values
    {
        let signal = stg
            .signal_by_name(name)
            .ok_or_else(|| AstgMarkingError::BadSignalName(name.clone()))?;

        signal_mask |= signal.bit;
        if *value
        {
            state_code |= signal.bit;
        }
    }

    set_marking_by_code(stg, state_code, signal_mask)
}

fn run_token_flow(stg: &AstgGraph) -> Result<(), AstgMarkingError>
{
    match stg.token_flow_status
    {
        TokenFlowStatus::Ok | TokenFlowStatus::NotUsc => Ok(()),
        status => Err(AstgMarkingError::TokenFlow(status)),
    }
}

fn run_token_flow_allowing_not_usc(stg: &AstgGraph) -> Result<(), AstgMarkingError>
{
    run_token_flow(stg)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn valid_marking_requires_one_token_in_each_component()
    {
        let mut stg = AstgGraph::default();
        let p0 = stg.add_vertex(AstgVertex::marked_place("p0"));
        let p1 = stg.add_vertex(AstgVertex::place("p1"));
        let p2 = stg.add_vertex(AstgVertex::place("p2"));
        stg.add_state_machine_component(vec![p0, p1]);
        stg.add_state_machine_component(vec![p1, p2]);
        stg.has_marking = true;

        assert_eq!(
            valid_marking(&stg),
            Err(AstgMarkingError::InvalidStateMachineComponent
            {
                component: 1,
                token_count: 0,
            }),
        );

        stg.vertices[p2.0].initial_token = true;

        assert_eq!(valid_marking(&stg), Ok(()));
    }

    #[test]
    fn find_marking_places_tokens_and_removes_covered_components()
    {
        let mut stg = AstgGraph::default();
        let p0 = stg.add_vertex(AstgVertex::place("p0"));
        let p1 = stg.add_vertex(AstgVertex::place("p1"));
        let p2 = stg.add_vertex(AstgVertex::place("p2"));
        let t0 = stg.add_vertex(AstgVertex::transition("t0"));
        stg.add_state_machine_component(vec![p0, p1, t0]);
        stg.add_state_machine_component(vec![p1, p2]);
        stg.add_state_machine_component(vec![p2]);

        assert_eq!(find_marking(&mut stg), Ok(()));
        assert_eq!(stg.marking(), vec![p0, p2]);
        assert!(stg.has_marking);
        assert_eq!(valid_marking(&stg), Ok(()));
    }

    #[test]
    fn find_marking_reports_when_greedy_choice_leaves_component_without_place()
    {
        let mut stg = AstgGraph::default();
        let p0 = stg.add_vertex(AstgVertex::place("p0"));
        let p1 = stg.add_vertex(AstgVertex::place("p1"));
        stg.add_state_machine_component(vec![p0, p1]);
        stg.add_state_machine_component(vec![p0]);
        stg.add_state_machine_component(vec![p1]);

        assert_eq!(
            find_marking(&mut stg),
            Err(AstgMarkingError::NoOneTokenStateMachineMarking),
        );
    }

    #[test]
    fn set_marking_by_code_applies_masked_state_marking()
    {
        let mut stg = AstgGraph::default();
        let p0 = stg.add_vertex(AstgVertex::place("p0"));
        let p1 = stg.add_vertex(AstgVertex::place("p1"));
        stg.add_state(AstgState::new(0b01, vec![AstgMarking::new(vec![p0])]));
        stg.add_state(AstgState::new(0b11, vec![AstgMarking::new(vec![p1])]));

        assert_eq!(set_marking_by_code(&mut stg, 0b10, 0b01), Ok(()));
        assert_eq!(stg.marking(), vec![p1]);
    }

    #[test]
    fn set_marking_by_name_combines_signal_values_into_masked_code()
    {
        let mut stg = AstgGraph::default();
        let p0 = stg.add_vertex(AstgVertex::place("p0"));
        let p1 = stg.add_vertex(AstgVertex::place("p1"));
        stg.add_signal(AstgSignal::new("a", 0b01));
        stg.add_signal(AstgSignal::new("b", 0b10));
        stg.add_state(AstgState::new(0b01, vec![AstgMarking::new(vec![p0])]));
        stg.add_state(AstgState::new(0b10, vec![AstgMarking::new(vec![p1])]));

        let signal_values = HashMap::from([("a".to_owned(), false), ("b".to_owned(), true)]);

        assert_eq!(set_marking_by_name(&mut stg, &signal_values), Ok(()));
        assert_eq!(stg.marking(), vec![p0]);
    }

    #[test]
    fn unique_state_falls_back_to_single_marking_state_when_state_codes_collide()
    {
        let mut stg = AstgGraph
        {
            token_flow_status: TokenFlowStatus::NotUsc,
            ..AstgGraph::default()
        };

        stg.add_state(AstgState::new(
            0b01,
            vec![
                AstgMarking::new(vec![VertexId(0)]),
                AstgMarking::new(vec![VertexId(1)]),
            ],
        ));
        stg.add_state(AstgState::new(
            0b10,
            vec![AstgMarking::new(vec![VertexId(2)])],
        ));

        assert_eq!(unique_state(&mut stg), Ok(0b10));
    }

    #[test]
    fn initial_state_returns_adjusted_initial_code_after_marking_exists()
    {
        let mut stg = AstgGraph
        {
            has_marking: true,
            initial_state: Some(0),
            code_adjustment: 0b10,
            ..AstgGraph::default()
        };

        stg.add_state(AstgState::new(0b01, vec![AstgMarking::new(Vec::new())]));

        assert_eq!(initial_state(&mut stg), Ok(0b11));
    }

    #[test]
    fn source_contains_no_legacy_exports_or_tracking_metadata()
    {
        let source = include_str!("astg_marking.rs");
        let forbidden = [
            concat!("no", "_mangle"),
            concat!("extern ", "\"C\""),
            concat!("REQUIRED", "_"),
            concat!("Port", "Dependency"),
            concat!("source", "_file"),
            concat!("bead", "_id"),
        ];

        for token in forbidden
        {
            assert!(!source.contains(token), "{token}");
        }
    }
}
