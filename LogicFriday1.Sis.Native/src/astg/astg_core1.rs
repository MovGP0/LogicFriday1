//! Native Rust ASTG core graph primitives.
//!
//! This ports the low-level graph, marking, state, signal, and PLA helpers from
//! the SIS `astg_core1.c` unit without exposing legacy C ABI entry points.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub type AstgScode = u64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AstgCoreError {
    BitIndexOutOfRange {
        index: usize,
        len: usize,
    },
    MissingPlace(PlaceId),
    MissingTransition(TransitionId),
    MissingSignal(SignalId),
    MissingEdge(EdgeId),
    IncompatibleEdge {
        tail: VertexKind,
        head: VertexKind,
    },
    SignalTypeMismatch {
        name: String,
        expected: SignalKind,
        actual: SignalKind,
    },
    InvalidTransitionForSignal {
        signal: String,
        signal_kind: SignalKind,
        transition_kind: TransitionKind,
    },
    UnsafeMarking(PlaceId),
    EmptyState,
    TokenFlowUnavailable,
}

impl fmt::Display for AstgCoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BitIndexOutOfRange { index, len } => {
                write!(formatter, "bit index {index} is outside 0..{len}")
            }
            Self::MissingPlace(id) => write!(formatter, "missing ASTG place {}", id.0),
            Self::MissingTransition(id) => write!(formatter, "missing ASTG transition {}", id.0),
            Self::MissingSignal(id) => write!(formatter, "missing ASTG signal {}", id.0),
            Self::MissingEdge(id) => write!(formatter, "missing ASTG edge {}", id.0),
            Self::IncompatibleEdge { tail, head } => {
                write!(formatter, "cannot connect {tail:?} to {head:?}")
            }
            Self::SignalTypeMismatch {
                name,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "signal {name} has type {actual:?}, expected {expected:?}"
                )
            }
            Self::InvalidTransitionForSignal {
                signal,
                signal_kind,
                transition_kind,
            } => {
                write!(
                    formatter,
                    "transition {transition_kind:?} is invalid for signal {signal} ({signal_kind:?})"
                )
            }
            Self::UnsafeMarking(id) => write!(formatter, "unsafe marking at place {}", id.0),
            Self::EmptyState => formatter.write_str("state has no markings"),
            Self::TokenFlowUnavailable => {
                formatter.write_str("state graph is empty; run token flow before writing PLA")
            }
        }
    }
}

impl Error for AstgCoreError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BitArray {
    bits: Vec<u64>,
    len: usize,
}

impl BitArray {
    pub fn new(len: usize) -> Self {
        Self {
            bits: vec![0; len.div_ceil(u64::BITS as usize)],
            len,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn set(&mut self, index: usize) -> Result<(), AstgCoreError> {
        self.check_index(index)?;
        self.bits[index / 64] |= 1_u64 << (index % 64);
        Ok(())
    }

    pub fn clear(&mut self, index: usize) -> Result<(), AstgCoreError> {
        self.check_index(index)?;
        self.bits[index / 64] &= !(1_u64 << (index % 64));
        Ok(())
    }

    pub fn get(&self, index: usize) -> Result<bool, AstgCoreError> {
        self.check_index(index)?;
        Ok((self.bits[index / 64] & (1_u64 << (index % 64))) != 0)
    }

    fn check_index(&self, index: usize) -> Result<(), AstgCoreError> {
        if index < self.len {
            Ok(())
        } else {
            Err(AstgCoreError::BitIndexOutOfRange {
                index,
                len: self.len,
            })
        }
    }
}

impl Ord for BitArray {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.len
            .cmp(&other.len)
            .then_with(|| self.bits.cmp(&other.bits))
    }
}

impl PartialOrd for BitArray {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexKind {
    Place,
    Transition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PlaceId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TransitionId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SignalId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EdgeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexId {
    Place(PlaceId),
    Transition(TransitionId),
}

impl VertexId {
    pub fn kind(self) -> VertexKind {
        match self {
            Self::Place(_) => VertexKind::Place,
            Self::Transition(_) => VertexKind::Transition,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind {
    Input,
    Output,
    Internal,
    Dummy,
}

impl SignalKind {
    pub fn is_input(self) -> bool {
        self == Self::Input
    }

    pub fn is_noninput(self) -> bool {
        matches!(self, Self::Output | Self::Internal)
    }

    pub fn is_dummy(self) -> bool {
        self == Self::Dummy
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalLookupKind {
    Any,
    Exact(SignalKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionKind {
    Positive,
    Negative,
    Toggle,
    Dummy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub id: PlaceId,
    name: Option<String>,
    pub x: f32,
    pub y: f32,
    pub subset: bool,
    pub selected: bool,
    pub useful: bool,
    pub hilited: bool,
    pub initial_token: bool,
    pub user_named: bool,
    flow_id: Option<usize>,
    in_edges: Vec<EdgeId>,
    out_edges: Vec<EdgeId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub id: TransitionId,
    name: String,
    alpha_name: String,
    pub x: f32,
    pub y: f32,
    pub subset: bool,
    pub selected: bool,
    pub useful: bool,
    pub hilited: bool,
    pub delay: f32,
    pub signal: SignalId,
    pub kind: TransitionKind,
    pub copy_n: i32,
    in_edges: Vec<EdgeId>,
    out_edges: Vec<EdgeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal {
    pub id: SignalId,
    pub name: String,
    pub kind: SignalKind,
    pub state_bit: AstgScode,
    pub can_elim: bool,
    transitions: Vec<TransitionId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardExpr {
    Const(bool),
    Signal(String),
    Not(Box<GuardExpr>),
    And(Vec<GuardExpr>),
    Or(Vec<GuardExpr>),
}

impl GuardExpr {
    pub fn eval<F>(&self, signal_level: &mut F) -> bool
    where
        F: FnMut(&str) -> bool,
    {
        match self {
            Self::Const(value) => *value,
            Self::Signal(name) => signal_level(name),
            Self::Not(expr) => !expr.eval(signal_level),
            Self::And(exprs) => exprs.iter().all(|expr| expr.eval(signal_level)),
            Self::Or(exprs) => exprs.iter().any(|expr| expr.eval(signal_level)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: EdgeId,
    pub tail: VertexId,
    pub head: VertexId,
    pub selected: bool,
    pub guard_eqn: Option<String>,
    pub guard: Option<GuardExpr>,
    pub spline_points: Vec<(f32, f32)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Marking {
    pub marked_places: BitArray,
    pub enabled: AstgScode,
    pub state_code: AstgScode,
    pub is_dummy: bool,
}

impl Marking {
    pub fn new(place_count: usize) -> Self {
        Self {
            marked_places: BitArray::new(place_count),
            enabled: 0,
            state_code: 0,
            is_dummy: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub markings: Vec<Marking>,
}

impl State {
    pub fn new() -> Self {
        Self {
            markings: Vec::new(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowInfo {
    pub status: AstgFlowStatus,
    pub change_count: u64,
    pub phase_adj: AstgScode,
    pub initial_state: AstgScode,
    pub in_width: usize,
    pub out_width: usize,
    pub state_list: BTreeMap<AstgScode, State>,
}

impl Default for FlowInfo {
    fn default() -> Self {
        Self {
            status: AstgFlowStatus::Ok,
            change_count: 0,
            phase_adj: 0,
            initial_state: 0,
            in_width: 0,
            out_width: 0,
            state_list: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AstgFlowStatus {
    Ok,
    NotSafe,
    NotCsa,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AstgGraph {
    name: String,
    change_count: u64,
    places: Vec<Place>,
    transitions: Vec<Transition>,
    signals: Vec<Signal>,
    edges: Vec<Edge>,
    signal_by_name: BTreeMap<String, SignalId>,
    has_marking: bool,
    next_place: usize,
    next_sig: usize,
    pub flow_info: FlowInfo,
    pub selection_name: Option<String>,
    pub comments: Vec<String>,
}

impl AstgGraph {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            change_count: 1,
            places: Vec::new(),
            transitions: Vec::new(),
            signals: Vec::new(),
            edges: Vec::new(),
            signal_by_name: BTreeMap::new(),
            has_marking: false,
            next_place: 0,
            next_sig: 0,
            flow_info: FlowInfo::default(),
            selection_name: None,
            comments: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn change_count(&self) -> u64 {
        self.change_count
    }

    pub fn num_vertices(&self) -> usize {
        self.places.len() + self.transitions.len()
    }

    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    pub fn places(&self) -> impl Iterator<Item = &Place> {
        self.places.iter()
    }

    pub fn transitions(&self) -> impl Iterator<Item = &Transition> {
        self.transitions.iter()
    }

    pub fn signals(&self) -> impl Iterator<Item = &Signal> {
        self.signals.iter()
    }

    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter()
    }

    pub fn place(&self, id: PlaceId) -> Result<&Place, AstgCoreError> {
        self.places.get(id.0).ok_or(AstgCoreError::MissingPlace(id))
    }

    pub fn place_mut(&mut self, id: PlaceId) -> Result<&mut Place, AstgCoreError> {
        self.places
            .get_mut(id.0)
            .ok_or(AstgCoreError::MissingPlace(id))
    }

    pub fn transition(&self, id: TransitionId) -> Result<&Transition, AstgCoreError> {
        self.transitions
            .get(id.0)
            .ok_or(AstgCoreError::MissingTransition(id))
    }

    pub fn transition_mut(&mut self, id: TransitionId) -> Result<&mut Transition, AstgCoreError> {
        self.transitions
            .get_mut(id.0)
            .ok_or(AstgCoreError::MissingTransition(id))
    }

    pub fn signal(&self, id: SignalId) -> Result<&Signal, AstgCoreError> {
        self.signals
            .get(id.0)
            .ok_or(AstgCoreError::MissingSignal(id))
    }

    pub fn edge(&self, id: EdgeId) -> Result<&Edge, AstgCoreError> {
        self.edges.get(id.0).ok_or(AstgCoreError::MissingEdge(id))
    }

    pub fn new_place(&mut self, name: Option<&str>) -> PlaceId {
        let id = PlaceId(self.places.len());
        self.places.push(Place {
            id,
            name: name.map(str::to_owned),
            x: 0.0,
            y: 0.0,
            subset: true,
            selected: false,
            useful: false,
            hilited: false,
            initial_token: false,
            user_named: false,
            flow_id: None,
            in_edges: Vec::new(),
            out_edges: Vec::new(),
        });
        self.change_count += 1;
        id
    }

    pub fn find_place(&mut self, name: Option<&str>, create: bool) -> Option<PlaceId> {
        if let Some(name) = name {
            if let Some(place) = self
                .places
                .iter()
                .find(|place| place.name.as_deref() == Some(name))
            {
                return Some(place.id);
            }
        }

        create.then(|| self.new_place(name))
    }

    pub fn make_place_name(&mut self, id: PlaceId) -> Result<String, AstgCoreError> {
        if let Some(name) = self.place(id)?.name.clone() {
            return Ok(name);
        }

        loop {
            let name = format!("pin{}", self.next_place);
            self.next_place += 1;
            if self.find_place(Some(&name), false).is_none() {
                self.place_mut(id)?.name = Some(name.clone());
                self.change_count += 1;
                return Ok(name);
            }
        }
    }

    pub fn place_name(&self, id: PlaceId) -> Result<String, AstgCoreError> {
        let place = self.place(id)?;
        if self.is_boring_place(id)? {
            let input = self.transition_name(self.edge(place.in_edges[0])?.tail_transition()?)?;
            let output = self.transition_name(self.edge(place.out_edges[0])?.head_transition()?)?;
            Ok(format!("<{input},{output}>"))
        } else {
            Ok(place.name.clone().unwrap_or_else(|| "*unnamed*".to_owned()))
        }
    }

    pub fn is_boring_place(&self, id: PlaceId) -> Result<bool, AstgCoreError> {
        let place = self.place(id)?;
        Ok(!place.user_named && place.in_edges.len() == 1 && place.out_edges.len() == 1)
    }

    pub fn find_signal(
        &mut self,
        name: &str,
        kind: SignalLookupKind,
        create: bool,
    ) -> Result<Option<SignalId>, AstgCoreError> {
        if let Some(id) = self.signal_by_name.get(name).copied() {
            let signal_kind = self.signal(id)?.kind;
            return match kind {
                SignalLookupKind::Any => Ok(Some(id)),
                SignalLookupKind::Exact(expected) if expected == signal_kind => Ok(Some(id)),
                SignalLookupKind::Exact(expected) => Err(AstgCoreError::SignalTypeMismatch {
                    name: name.to_owned(),
                    expected,
                    actual: signal_kind,
                }),
            };
        }

        if !create {
            return Ok(None);
        }

        let SignalLookupKind::Exact(signal_kind) = kind else {
            return Ok(None);
        };

        let id = SignalId(self.signals.len());
        self.signals.push(Signal {
            id,
            name: name.to_owned(),
            kind: signal_kind,
            state_bit: 0,
            can_elim: false,
            transitions: Vec::new(),
        });
        self.signal_by_name.insert(name.to_owned(), id);
        self.change_count += 1;
        Ok(Some(id))
    }

    pub fn find_named_signal(&self, name: &str) -> Option<SignalId> {
        self.signal_by_name.get(name).copied()
    }

    pub fn new_signal(&mut self, kind: SignalKind) -> SignalId {
        loop {
            let name = format!("is{}", self.next_sig);
            self.next_sig += 1;
            if !self.signal_by_name.contains_key(&name) {
                return self
                    .find_signal(&name, SignalLookupKind::Exact(kind), true)
                    .expect("new signal insertion cannot fail")
                    .expect("new signal insertion returns an id");
            }
        }
    }

    pub fn find_transition(
        &mut self,
        signal_name: &str,
        kind: TransitionKind,
        copy_n: i32,
        create: bool,
    ) -> Result<Option<TransitionId>, AstgCoreError> {
        if let Some(transition) = self.transitions.iter().find(|transition| {
            transition.kind == kind
                && transition.copy_n == copy_n
                && self.signals[transition.signal.0].name == signal_name
        }) {
            return Ok(Some(transition.id));
        }

        if !create {
            return Ok(None);
        }

        let Some(signal_id) = self.find_named_signal(signal_name) else {
            return Ok(None);
        };
        let signal = self.signal(signal_id)?;
        let signal_kind = signal.kind;
        if (kind == TransitionKind::Dummy) != (signal_kind == SignalKind::Dummy) {
            return Err(AstgCoreError::InvalidTransitionForSignal {
                signal: signal_name.to_owned(),
                signal_kind,
                transition_kind: kind,
            });
        }

        let name = make_transition_name(signal_name, kind, copy_n);
        let id = TransitionId(self.transitions.len());
        self.transitions.push(Transition {
            id,
            alpha_name: sanitize_name(&name),
            name,
            x: 0.0,
            y: 0.0,
            subset: true,
            selected: false,
            useful: false,
            hilited: false,
            delay: 0.0,
            signal: signal_id,
            kind,
            copy_n,
            in_edges: Vec::new(),
            out_edges: Vec::new(),
        });
        self.signals[signal_id.0].transitions.insert(0, id);
        self.change_count += 1;
        Ok(Some(id))
    }

    pub fn transition_name(&self, id: TransitionId) -> Result<&str, AstgCoreError> {
        Ok(&self.transition(id)?.name)
    }

    pub fn transition_alpha_name(&self, id: TransitionId) -> Result<&str, AstgCoreError> {
        Ok(&self.transition(id)?.alpha_name)
    }

    pub fn new_edge(&mut self, tail: VertexId, head: VertexId) -> Result<EdgeId, AstgCoreError> {
        if tail.kind() == head.kind() {
            return Err(AstgCoreError::IncompatibleEdge {
                tail: tail.kind(),
                head: head.kind(),
            });
        }

        self.vertex_exists(tail)?;
        self.vertex_exists(head)?;

        let id = EdgeId(self.edges.len());
        self.edges.push(Edge {
            id,
            tail,
            head,
            selected: false,
            guard_eqn: None,
            guard: None,
            spline_points: Vec::new(),
        });
        self.out_edges_mut(tail)?.push(id);
        self.in_edges_mut(head)?.push(id);
        self.change_count += 1;
        Ok(id)
    }

    pub fn find_edge(
        &mut self,
        tail: VertexId,
        head: VertexId,
        create: bool,
    ) -> Result<Option<EdgeId>, AstgCoreError> {
        self.vertex_exists(tail)?;
        self.vertex_exists(head)?;

        let existing = self
            .out_edges(tail)?
            .iter()
            .copied()
            .find(|edge| self.edges[edge.0].head == head);

        match (existing, create) {
            (Some(id), _) => Ok(Some(id)),
            (None, true) => self.new_edge(tail, head).map(Some),
            (None, false) => Ok(None),
        }
    }

    pub fn edge_name(&self, id: EdgeId) -> Result<String, AstgCoreError> {
        let edge = self.edge(id)?;
        Ok(format!(
            "({},{})",
            self.vertex_name(edge.tail)?,
            self.vertex_name(edge.head)?
        ))
    }

    pub fn set_guard(
        &mut self,
        id: EdgeId,
        guard_eqn: Option<String>,
        guard: Option<GuardExpr>,
    ) -> Result<(), AstgCoreError> {
        let edge = self
            .edges
            .get_mut(id.0)
            .ok_or(AstgCoreError::MissingEdge(id))?;
        edge.guard_eqn = guard_eqn;
        edge.guard = guard;
        self.change_count += 1;
        Ok(())
    }

    pub fn add_constraint(
        &mut self,
        first: TransitionId,
        second: TransitionId,
        check: bool,
    ) -> Result<Option<PlaceId>, AstgCoreError> {
        if check && self.has_constraint(first, second)? {
            return Ok(None);
        }

        let name = format!("p_{}", self.next_place);
        self.next_place += 1;
        let place = self.new_place(Some(&name));
        self.new_edge(VertexId::Transition(first), VertexId::Place(place))?;
        self.new_edge(VertexId::Place(place), VertexId::Transition(second))?;
        self.has_marking = false;
        Ok(Some(place))
    }

    pub fn has_constraint(
        &self,
        first: TransitionId,
        second: TransitionId,
    ) -> Result<bool, AstgCoreError> {
        for place in self.output_places(first)? {
            if self.output_transitions(place)?.contains(&second) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn is_trigger(&self, signal1: SignalId, signal2: SignalId) -> Result<bool, AstgCoreError> {
        self.signal(signal1)?;
        self.signal(signal2)?;

        for transition in &self.signals[signal1.0].transitions {
            if !matches!(
                self.transitions[transition.0].kind,
                TransitionKind::Positive | TransitionKind::Negative
            ) {
                continue;
            }

            for place in self.output_places(*transition)? {
                for output_transition in self.output_transitions(place)? {
                    if self.transition(output_transition)?.signal == signal2 {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    pub fn noninput_trigger(
        &self,
        transition_id: TransitionId,
    ) -> Result<Option<TransitionId>, AstgCoreError> {
        let start = transition_id;
        let mut current = transition_id;

        while self
            .signal(self.transition(current)?.signal)?
            .kind
            .is_input()
        {
            let Some(input_place_edge) = self.transition(current)?.in_edges.first().copied() else {
                return Ok(None);
            };
            let input_place = self.edge(input_place_edge)?.tail_place()?;
            let Some(input_trans_edge) = self.place(input_place)?.in_edges.first().copied() else {
                return Ok(None);
            };
            current = self.edge(input_trans_edge)?.tail_transition()?;
            if current == start {
                return Ok(None);
            }
        }

        Ok(Some(current))
    }

    pub fn reset_state_graph(&mut self) -> bool {
        if self.change_count == self.flow_info.change_count {
            return false;
        }

        self.flow_info = FlowInfo {
            status: AstgFlowStatus::Ok,
            change_count: self.change_count,
            phase_adj: 0,
            initial_state: 0,
            in_width: 0,
            out_width: 0,
            state_list: BTreeMap::new(),
        };

        let mut state_bit = 1;
        for signal in &mut self.signals {
            if signal.kind.is_noninput() {
                self.flow_info.in_width += 1;
                self.flow_info.out_width += 1;
                signal.state_bit = state_bit;
                state_bit <<= 1;
            }
        }

        for signal in &mut self.signals {
            if signal.kind.is_input() {
                self.flow_info.in_width += 1;
                signal.state_bit = state_bit;
                state_bit <<= 1;
            }
        }

        for transition in &mut self.transitions {
            transition.useful = false;
        }

        for (flow_id, place) in self.places.iter_mut().enumerate() {
            place.useful = false;
            place.flow_id = Some(flow_id);
        }

        true
    }

    pub fn find_state(&mut self, real_code: AstgScode, create: bool) -> Option<&mut State> {
        let code = real_code ^ self.flow_info.phase_adj;
        if create {
            self.flow_info.state_list.entry(code).or_default();
        }
        self.flow_info.state_list.get_mut(&code)
    }

    pub fn state_count(&self) -> usize {
        self.flow_info.state_list.len()
    }

    pub fn mark(&mut self, place: PlaceId, marking: &mut Marking) -> Result<(), AstgCoreError> {
        if self.get_marked(marking, place)? {
            self.select_new("unsafe place", false);
            self.place_mut(place)?.selected = true;
            self.flow_info.status = AstgFlowStatus::NotSafe;
            Err(AstgCoreError::UnsafeMarking(place))
        } else {
            self.set_marked(marking, place, true)
        }
    }

    pub fn unmark(&self, place: PlaceId, marking: &mut Marking) -> Result<(), AstgCoreError> {
        let flow_id = self.place(place)?.flow_id;
        if let Some(flow_id) = flow_id {
            marking.marked_places.clear(flow_id)?;
        }
        Ok(())
    }

    pub fn set_marked(
        &self,
        marking: &mut Marking,
        place: PlaceId,
        value: bool,
    ) -> Result<(), AstgCoreError> {
        let Some(flow_id) = self.place(place)?.flow_id else {
            return Ok(());
        };

        if value {
            marking.marked_places.set(flow_id)
        } else {
            marking.marked_places.clear(flow_id)
        }
    }

    pub fn get_marked(&self, marking: &Marking, place: PlaceId) -> Result<bool, AstgCoreError> {
        let Some(flow_id) = self.place(place)?.flow_id else {
            return Ok(true);
        };

        marking.marked_places.get(flow_id)
    }

    pub fn initial_marking(&mut self) -> Result<Marking, AstgCoreError> {
        let mut marking = Marking::new(self.places.len());
        for place_id in (0..self.places.len()).map(PlaceId) {
            if self.place(place_id)?.initial_token {
                self.mark(place_id, &mut marking)?;
            }
        }
        Ok(marking)
    }

    pub fn set_marking(&mut self, marking: &Marking) -> Result<(), AstgCoreError> {
        let mut changed = !self.has_marking;
        for place_id in (0..self.places.len()).map(PlaceId) {
            let marked = self.get_marked(marking, place_id)?;
            let place = self.place_mut(place_id)?;
            if place.initial_token != marked {
                place.initial_token = marked;
                changed = true;
            }
        }

        self.flow_info.initial_state = marking.state_code;
        self.has_marking = true;
        if changed {
            self.change_count += 1;
        }
        Ok(())
    }

    pub fn disabled_count(
        &self,
        transition: TransitionId,
        marking: &Marking,
    ) -> Result<usize, AstgCoreError> {
        let mut enabled_input_count = 0;
        for edge_id in self.transition(transition)?.in_edges.iter().copied() {
            let edge = self.edge(edge_id)?;
            let place = edge.tail_place()?;
            if self.get_marked(marking, place)? && self.guard_enabled(edge, marking)? {
                enabled_input_count += 1;
            }
        }

        Ok(self.transition(transition)?.in_edges.len() - enabled_input_count)
    }

    pub fn add_marking(
        &self,
        state: &mut State,
        marking: &mut Marking,
    ) -> Result<(), AstgCoreError> {
        let mut enabled = 0;
        marking.is_dummy = false;

        for transition in &self.transitions {
            if self.disabled_count(transition.id, marking)? == 0 {
                let signal = self.signal(transition.signal)?;
                enabled |= signal.state_bit;
                if signal.kind == SignalKind::Dummy {
                    marking.is_dummy = true;
                }
            }
        }

        marking.enabled = enabled;
        state.markings.push(marking.clone());
        Ok(())
    }

    pub fn fire(
        &mut self,
        marking: &mut Marking,
        transition: TransitionId,
    ) -> Result<AstgScode, AstgCoreError> {
        let transition_data = self.transition(transition)?.clone();
        let state_bit = self.signal(transition_data.signal)?.state_bit;
        let new_state = marking.state_code ^ state_bit;

        if matches!(
            transition_data.kind,
            TransitionKind::Positive | TransitionKind::Negative
        ) {
            let real_state = new_state ^ self.flow_info.phase_adj;
            let should_be_set = transition_data.kind == TransitionKind::Positive;
            let is_set = has_bit(real_state, state_bit);
            if should_be_set != is_set {
                if has_bit(self.flow_info.phase_adj, state_bit) {
                    self.flow_info.status = AstgFlowStatus::NotCsa;
                    self.select_new("Inconsistent state assignment", false);
                    self.transition_mut(transition)?.selected = true;
                }
                set_bit(&mut self.flow_info.phase_adj, state_bit);
            }
        }

        for edge_id in transition_data.in_edges {
            self.unmark(self.edge(edge_id)?.tail_place()?, marking)?;
        }

        marking.state_code ^= state_bit;

        for edge_id in transition_data.out_edges {
            self.mark(self.edge(edge_id)?.head_place()?, marking)?;
        }

        Ok(new_state)
    }

    pub fn unfire(
        &mut self,
        marking: &mut Marking,
        transition: TransitionId,
    ) -> Result<AstgScode, AstgCoreError> {
        let transition_data = self.transition(transition)?.clone();
        let state_bit = self.signal(transition_data.signal)?.state_bit;
        let previous_state = marking.state_code ^ state_bit;

        for edge_id in transition_data.out_edges {
            self.unmark(self.edge(edge_id)?.head_place()?, marking)?;
        }

        marking.state_code ^= state_bit;

        for edge_id in transition_data.in_edges {
            self.mark(self.edge(edge_id)?.tail_place()?, marking)?;
        }

        Ok(previous_state)
    }

    pub fn adjusted_code(&self, state_code: AstgScode) -> AstgScode {
        state_code ^ self.flow_info.phase_adj
    }

    pub fn state_code(&self, state: &State) -> Result<AstgScode, AstgCoreError> {
        let marking = state.markings.first().ok_or(AstgCoreError::EmptyState)?;
        Ok(marking.state_code ^ self.flow_info.phase_adj)
    }

    pub fn state_enabled(state: &State) -> AstgScode {
        state
            .markings
            .iter()
            .fold(0, |enabled, marking| enabled | marking.enabled)
    }

    pub fn cmp_marking(left: &Marking, right: &Marking) -> bool {
        left.state_code != right.state_code || left.marked_places != right.marked_places
    }

    pub fn print_state(&self, state: AstgScode) -> String {
        let mut text = String::new();
        for signal in &self.signals {
            text.push_str(&format!(
                " {}={}",
                signal.name,
                usize::from(has_bit(state, signal.state_bit))
            ));
        }
        text.push('\n');
        text
    }

    pub fn print_marking(&self, index: usize, marking: &Marking) -> Result<String, AstgCoreError> {
        let mut text = format!(
            "marking {index}\n  state 0x{:X}, enabled 0x{:X}\n  marked:",
            marking.state_code, marking.enabled
        );
        for place in &self.places {
            if self.get_marked(marking, place.id)? {
                text.push(' ');
                text.push_str(&self.place_name(place.id)?);
            }
        }
        text.push('\n');
        Ok(text)
    }

    pub fn select_new(&mut self, name: impl Into<String>, value: bool) {
        self.selection_name = Some(name.into());
        for place in &mut self.places {
            place.selected = value;
        }
        for transition in &mut self.transitions {
            transition.selected = value;
        }
        for edge in &mut self.edges {
            edge.selected = value;
        }
    }

    pub fn select_clear(&mut self) {
        self.selection_name = None;
    }

    pub fn selected_summary(&self) -> Result<Option<String>, AstgCoreError> {
        let Some(name) = &self.selection_name else {
            return Ok(None);
        };

        let transitions = self
            .transitions
            .iter()
            .filter(|transition| transition.selected)
            .map(|transition| transition.name.clone())
            .collect::<Vec<_>>();
        let places = self
            .places
            .iter()
            .filter(|place| place.selected)
            .map(|place| self.place_name(place.id))
            .collect::<Result<Vec<_>, _>>()?;

        if transitions.is_empty() && places.is_empty() {
            return Ok(None);
        }

        let mut text = format!("{name}:\n");
        if !transitions.is_empty() {
            text.push_str(&transitions.join(" "));
            text.push('\n');
        }
        if !places.is_empty() {
            text.push_str(&places.join(" "));
            text.push('\n');
        }
        Ok(Some(text))
    }

    pub fn write_pla(&self) -> Result<String, AstgCoreError> {
        if self.flow_info.state_list.is_empty() {
            return Err(AstgCoreError::TokenFlowUnavailable);
        }

        let mut signals = self
            .signals
            .iter()
            .filter(|signal| signal.kind != SignalKind::Dummy)
            .collect::<Vec<_>>();
        signals.sort_by_key(|signal| signal.state_bit);

        let mut in_mask = 0;
        let mut out_mask = 0;
        let mut output = String::new();
        output.push_str(&format!("# {}\n", self.name));
        output.push_str(&format!(".i {}\n.o {}", signals.len(), self.output_count()));
        output.push_str("\n.ilb");
        for signal in &signals {
            output.push_str(&format!(" {}", signal.name));
            in_mask |= signal.state_bit;
        }

        output.push_str("\n.ob");
        for signal in &signals {
            if signal.kind.is_noninput() {
                output.push_str(&format!(" {}_out", signal.name));
                out_mask |= signal.state_bit;
            }
        }

        output.push_str("\n.type fr\n");
        for state in self.flow_info.state_list.values() {
            output.push_str(&self.espresso_row(
                in_mask,
                out_mask,
                self.state_code(state)?,
                Self::state_enabled(state),
            ));
        }
        output.push_str(".e\n");
        Ok(output)
    }

    fn output_count(&self) -> usize {
        self.signals
            .iter()
            .filter(|signal| signal.kind.is_noninput())
            .count()
    }

    fn espresso_row(
        &self,
        in_mask: AstgScode,
        out_mask: AstgScode,
        new_state: AstgScode,
        delta_ns: AstgScode,
    ) -> String {
        let mut row = String::new();
        let mut s = new_state;
        let mut v = in_mask;
        for _ in 0..self.flow_info.in_width {
            row.push(if v & 1 != 0 {
                if s & 1 != 0 { '1' } else { '0' }
            } else {
                '-'
            });
            s >>= 1;
            v >>= 1;
        }

        row.push(' ');
        s = new_state ^ delta_ns;
        v = out_mask;
        for _ in 0..self.flow_info.out_width {
            row.push(if v & 1 != 0 {
                if s & 1 != 0 { '1' } else { '0' }
            } else {
                '-'
            });
            s >>= 1;
            v >>= 1;
        }
        row.push('\n');
        row
    }

    fn guard_enabled(&self, edge: &Edge, marking: &Marking) -> Result<bool, AstgCoreError> {
        let Some(guard) = &edge.guard else {
            return Ok(true);
        };

        let mut missing_signal = None;
        let enabled = guard.eval(&mut |name| {
            let Some(signal_id) = self.find_named_signal(name) else {
                missing_signal = Some(name.to_owned());
                return false;
            };
            let signal = &self.signals[signal_id.0];
            has_bit(marking.state_code, signal.state_bit)
        });

        if let Some(name) = missing_signal {
            return Err(AstgCoreError::SignalTypeMismatch {
                name,
                expected: SignalKind::Input,
                actual: SignalKind::Dummy,
            });
        }

        Ok(enabled)
    }

    fn vertex_exists(&self, vertex: VertexId) -> Result<(), AstgCoreError> {
        match vertex {
            VertexId::Place(id) => self.place(id).map(|_| ()),
            VertexId::Transition(id) => self.transition(id).map(|_| ()),
        }
    }

    fn vertex_name(&self, vertex: VertexId) -> Result<String, AstgCoreError> {
        match vertex {
            VertexId::Place(id) => self.place_name(id),
            VertexId::Transition(id) => self.transition_name(id).map(str::to_owned),
        }
    }

    fn in_edges_mut(&mut self, vertex: VertexId) -> Result<&mut Vec<EdgeId>, AstgCoreError> {
        match vertex {
            VertexId::Place(id) => Ok(&mut self.place_mut(id)?.in_edges),
            VertexId::Transition(id) => Ok(&mut self.transition_mut(id)?.in_edges),
        }
    }

    fn out_edges_mut(&mut self, vertex: VertexId) -> Result<&mut Vec<EdgeId>, AstgCoreError> {
        match vertex {
            VertexId::Place(id) => Ok(&mut self.place_mut(id)?.out_edges),
            VertexId::Transition(id) => Ok(&mut self.transition_mut(id)?.out_edges),
        }
    }

    fn out_edges(&self, vertex: VertexId) -> Result<&[EdgeId], AstgCoreError> {
        match vertex {
            VertexId::Place(id) => Ok(&self.place(id)?.out_edges),
            VertexId::Transition(id) => Ok(&self.transition(id)?.out_edges),
        }
    }

    fn output_places(&self, transition: TransitionId) -> Result<Vec<PlaceId>, AstgCoreError> {
        self.transition(transition)?
            .out_edges
            .iter()
            .map(|edge| self.edge(*edge)?.head_place())
            .collect()
    }

    fn output_transitions(&self, place: PlaceId) -> Result<Vec<TransitionId>, AstgCoreError> {
        self.place(place)?
            .out_edges
            .iter()
            .map(|edge| self.edge(*edge)?.head_transition())
            .collect()
    }
}

impl Edge {
    fn tail_place(&self) -> Result<PlaceId, AstgCoreError> {
        match self.tail {
            VertexId::Place(id) => Ok(id),
            VertexId::Transition(_) => Err(AstgCoreError::IncompatibleEdge {
                tail: VertexKind::Transition,
                head: VertexKind::Place,
            }),
        }
    }

    fn head_place(&self) -> Result<PlaceId, AstgCoreError> {
        match self.head {
            VertexId::Place(id) => Ok(id),
            VertexId::Transition(_) => Err(AstgCoreError::IncompatibleEdge {
                tail: VertexKind::Place,
                head: VertexKind::Transition,
            }),
        }
    }

    fn tail_transition(&self) -> Result<TransitionId, AstgCoreError> {
        match self.tail {
            VertexId::Transition(id) => Ok(id),
            VertexId::Place(_) => Err(AstgCoreError::IncompatibleEdge {
                tail: VertexKind::Place,
                head: VertexKind::Transition,
            }),
        }
    }

    fn head_transition(&self) -> Result<TransitionId, AstgCoreError> {
        match self.head {
            VertexId::Transition(id) => Ok(id),
            VertexId::Place(_) => Err(AstgCoreError::IncompatibleEdge {
                tail: VertexKind::Transition,
                head: VertexKind::Place,
            }),
        }
    }
}

pub fn sanitize_name(name: &str) -> String {
    let mut result = String::new();
    for value in name.chars() {
        match value {
            c if c.is_ascii_alphabetic() => result.push(c),
            '+' => result.push_str("_plus"),
            '-' => result.push_str("_minus"),
            '~' => result.push_str("_toggle"),
            _ => result.push('x'),
        }
    }
    result
}

pub fn make_transition_name(signal_name: &str, kind: TransitionKind, copy_n: i32) -> String {
    let suffix = match kind {
        TransitionKind::Positive => "+",
        TransitionKind::Negative => "-",
        TransitionKind::Toggle => "~",
        TransitionKind::Dummy => "",
    };

    if copy_n == 0 {
        format!("{signal_name}{suffix}")
    } else {
        format!("{signal_name}{suffix}/{copy_n}")
    }
}

pub fn has_bit(value: AstgScode, bit: AstgScode) -> bool {
    value & bit != 0
}

pub fn set_bit(value: &mut AstgScode, bit: AstgScode) {
    *value |= bit;
}

pub fn clear_bit(value: &mut AstgScode, bit: AstgScode) {
    *value &= !bit;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_array_sets_clears_and_compares_bits() {
        let mut bits = BitArray::new(65);

        assert!(!bits.get(64).unwrap());
        bits.set(64).unwrap();
        assert!(bits.get(64).unwrap());
        bits.clear(64).unwrap();
        assert!(!bits.get(64).unwrap());

        assert_eq!(
            bits.set(65),
            Err(AstgCoreError::BitIndexOutOfRange { index: 65, len: 65 })
        );
        assert_eq!(bits, BitArray::new(65));
    }

    #[test]
    fn graph_creates_signals_places_transitions_and_constraints() {
        let mut graph = AstgGraph::new("g");
        graph
            .find_signal("a", SignalLookupKind::Exact(SignalKind::Output), true)
            .unwrap();
        graph
            .find_signal("b", SignalLookupKind::Exact(SignalKind::Input), true)
            .unwrap();
        let a_plus = graph
            .find_transition("a", TransitionKind::Positive, 0, true)
            .unwrap()
            .unwrap();
        let b_plus = graph
            .find_transition("b", TransitionKind::Positive, 0, true)
            .unwrap()
            .unwrap();

        let place = graph.add_constraint(a_plus, b_plus, true).unwrap().unwrap();

        assert!(graph.has_constraint(a_plus, b_plus).unwrap());
        assert_eq!(graph.add_constraint(a_plus, b_plus, true).unwrap(), None);
        assert_eq!(graph.place(place).unwrap().name.as_deref(), Some("p_0"));
        assert_eq!(graph.num_edges(), 2);
    }

    #[test]
    fn reset_state_graph_assigns_output_bits_before_input_bits() {
        let mut graph = AstgGraph::new("g");
        let input = graph
            .find_signal("i", SignalLookupKind::Exact(SignalKind::Input), true)
            .unwrap()
            .unwrap();
        let output = graph
            .find_signal("o", SignalLookupKind::Exact(SignalKind::Output), true)
            .unwrap()
            .unwrap();

        assert!(graph.reset_state_graph());
        assert_eq!(graph.signal(output).unwrap().state_bit, 1);
        assert_eq!(graph.signal(input).unwrap().state_bit, 2);
        assert_eq!(graph.flow_info.in_width, 2);
        assert_eq!(graph.flow_info.out_width, 1);
        assert!(!graph.reset_state_graph());
    }

    #[test]
    fn fire_and_unfire_move_tokens_and_update_state() {
        let mut graph = AstgGraph::new("g");
        graph
            .find_signal("o", SignalLookupKind::Exact(SignalKind::Output), true)
            .unwrap();
        let t = graph
            .find_transition("o", TransitionKind::Positive, 0, true)
            .unwrap()
            .unwrap();
        let p0 = graph.new_place(Some("p0"));
        let p1 = graph.new_place(Some("p1"));
        graph
            .new_edge(VertexId::Place(p0), VertexId::Transition(t))
            .unwrap();
        graph
            .new_edge(VertexId::Transition(t), VertexId::Place(p1))
            .unwrap();
        graph.place_mut(p0).unwrap().initial_token = true;
        graph.reset_state_graph();

        let mut marking = graph.initial_marking().unwrap();
        assert!(graph.get_marked(&marking, p0).unwrap());
        assert!(!graph.get_marked(&marking, p1).unwrap());

        assert_eq!(graph.fire(&mut marking, t).unwrap(), 1);
        assert_eq!(marking.state_code, 1);
        assert!(!graph.get_marked(&marking, p0).unwrap());
        assert!(graph.get_marked(&marking, p1).unwrap());

        assert_eq!(graph.unfire(&mut marking, t).unwrap(), 0);
        assert_eq!(marking.state_code, 0);
        assert!(graph.get_marked(&marking, p0).unwrap());
        assert!(!graph.get_marked(&marking, p1).unwrap());
    }

    #[test]
    fn disabled_count_honors_guards() {
        let mut graph = AstgGraph::new("g");
        graph
            .find_signal("o", SignalLookupKind::Exact(SignalKind::Output), true)
            .unwrap();
        graph
            .find_signal("i", SignalLookupKind::Exact(SignalKind::Input), true)
            .unwrap();
        let t = graph
            .find_transition("o", TransitionKind::Positive, 0, true)
            .unwrap()
            .unwrap();
        let p = graph.new_place(Some("p"));
        let edge = graph
            .new_edge(VertexId::Place(p), VertexId::Transition(t))
            .unwrap();
        graph
            .set_guard(
                edge,
                Some("i".to_owned()),
                Some(GuardExpr::Signal("i".to_owned())),
            )
            .unwrap();
        graph.place_mut(p).unwrap().initial_token = true;
        graph.reset_state_graph();

        let input_bit = graph
            .signal(graph.find_named_signal("i").unwrap())
            .unwrap()
            .state_bit;
        let mut marking = graph.initial_marking().unwrap();
        assert_eq!(graph.disabled_count(t, &marking).unwrap(), 1);

        marking.state_code |= input_bit;
        assert_eq!(graph.disabled_count(t, &marking).unwrap(), 0);
    }

    #[test]
    fn write_pla_formats_state_rows() {
        let mut graph = AstgGraph::new("g");
        graph
            .find_signal("o", SignalLookupKind::Exact(SignalKind::Output), true)
            .unwrap();
        graph
            .find_signal("i", SignalLookupKind::Exact(SignalKind::Input), true)
            .unwrap();
        graph.reset_state_graph();

        let mut marking = Marking::new(0);
        marking.state_code = 0b10;
        marking.enabled = 0b01;
        let mut state = State::new();
        state.markings.push(marking);
        graph.flow_info.state_list.insert(0b10, state);

        let pla = graph.write_pla().unwrap();

        assert_eq!(
            pla,
            "# g\n.i 2\n.o 1\n.ilb o i\n.ob o_out\n.type fr\n01 1\n.e\n"
        );
    }
}
