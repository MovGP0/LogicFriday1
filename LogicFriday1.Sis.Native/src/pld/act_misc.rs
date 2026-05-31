//! Native Rust model for `LogicSynthesis/sis/pld/act_misc.c`.
//!
//! The C file contains small ACT DAG helpers around pointer-owned vertices:
//! freeing a node's global ACT, cloning a rooted DAG, complementing terminal
//! values while preserving the DAG shape, selecting an ACT construction mode,
//! and computing the legacy ACT size code. This port keeps those behaviors in
//! owned Rust data structures. Direct construction from SIS `node_t` data is
//! reported as a runtime prerequisite error until the native ACT builders exist.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub const NO_VALUE: i32 = 4;

pub type ActMiscResult<T> = Result<T, ActMiscError>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActValue {
    Zero,
    One,
    NoValue,
}

impl ActValue {
    pub fn from_c_value(value: i32) -> ActMiscResult<Self> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            NO_VALUE => Ok(Self::NoValue),
            _ => Err(ActMiscError::InvalidActValue(value)),
        }
    }

    pub fn as_c_value(self) -> i32 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::NoValue => NO_VALUE,
        }
    }

    pub fn complemented(self) -> Self {
        match self {
            Self::Zero => Self::One,
            Self::One => Self::Zero,
            Self::NoValue => Self::NoValue,
        }
    }

    pub fn is_internal(self) -> bool {
        self == Self::NoValue
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActVertex {
    pub id: i32,
    pub value: ActValue,
    pub index: i32,
    pub index_size: i32,
    pub mark: i32,
    pub low: Option<ActVertexId>,
    pub high: Option<ActVertexId>,
}

impl ActVertex {
    pub fn terminal(id: i32, value: bool, index_size: i32) -> Self {
        Self {
            id,
            value: if value { ActValue::One } else { ActValue::Zero },
            index: index_size,
            index_size,
            mark: 0,
            low: None,
            high: None,
        }
    }

    pub fn internal(
        id: i32,
        index: i32,
        index_size: i32,
        low: ActVertexId,
        high: ActVertexId,
    ) -> Self {
        Self {
            id,
            value: ActValue::NoValue,
            index,
            index_size,
            mark: 0,
            low: Some(low),
            high: Some(high),
        }
    }

    pub fn from_c_fields(
        id: i32,
        value: i32,
        index: i32,
        index_size: i32,
        mark: i32,
        low: Option<ActVertexId>,
        high: Option<ActVertexId>,
    ) -> ActMiscResult<Self> {
        Ok(Self {
            id,
            value: ActValue::from_c_value(value)?,
            index,
            index_size,
            mark,
            low,
            high,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActDag {
    vertices: Vec<ActVertex>,
    root: ActVertexId,
}

impl ActDag {
    pub fn new(root: ActVertex) -> Self {
        Self {
            vertices: vec![root],
            root: ActVertexId(0),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(capacity),
            root: ActVertexId(0),
        }
    }

    pub fn add_vertex(&mut self, vertex: ActVertex) -> ActVertexId {
        let id = ActVertexId(self.vertices.len());
        self.vertices.push(vertex);
        id
    }

    pub fn set_root(&mut self, root: ActVertexId) -> ActMiscResult<()> {
        self.vertex(root)?;
        self.root = root;
        Ok(())
    }

    pub fn root(&self) -> ActVertexId {
        self.root
    }

    pub fn vertex(&self, id: ActVertexId) -> ActMiscResult<&ActVertex> {
        self.vertices
            .get(id.0)
            .ok_or(ActMiscError::MissingVertex(id))
    }

    pub fn vertex_mut(&mut self, id: ActVertexId) -> ActMiscResult<&mut ActVertex> {
        self.vertices
            .get_mut(id.0)
            .ok_or(ActMiscError::MissingVertex(id))
    }

    pub fn vertices(&self) -> &[ActVertex] {
        &self.vertices
    }

    pub fn root_copy(&self) -> ActMiscResult<Self> {
        self.copy_from(self.root)
    }

    pub fn root_complement(&self) -> ActMiscResult<Self> {
        self.complement_from(self.root)
    }

    pub fn copy_from(&self, root: ActVertexId) -> ActMiscResult<Self> {
        self.transform_from(root, |value| value)
    }

    pub fn complement_from(&self, root: ActVertexId) -> ActMiscResult<Self> {
        self.transform_from(root, ActValue::complemented)
    }

    fn transform_from<F>(&self, root: ActVertexId, transform_value: F) -> ActMiscResult<Self>
    where
        F: Fn(ActValue) -> ActValue + Copy,
    {
        let mut output = Self::with_capacity(self.vertices.len());
        let mut copied = HashMap::new();
        let new_root = self.transform_vertex(root, &mut output, &mut copied, transform_value)?;
        output.set_root(new_root)?;
        Ok(output)
    }

    fn transform_vertex<F>(
        &self,
        source_id: ActVertexId,
        output: &mut ActDag,
        copied: &mut HashMap<ActVertexId, ActVertexId>,
        transform_value: F,
    ) -> ActMiscResult<ActVertexId>
    where
        F: Fn(ActValue) -> ActValue + Copy,
    {
        if let Some(copied_id) = copied.get(&source_id) {
            return Ok(*copied_id);
        }

        let source = self.vertex(source_id)?;
        let (low, high) = if source.value.is_internal() {
            (
                Some(self.transform_child(
                    source_id,
                    source.low,
                    output,
                    copied,
                    transform_value,
                )?),
                Some(self.transform_child(
                    source_id,
                    source.high,
                    output,
                    copied,
                    transform_value,
                )?),
            )
        } else {
            (None, None)
        };

        let copied_id = output.add_vertex(ActVertex {
            id: source.id,
            value: transform_value(source.value),
            index: source.index,
            index_size: source.index_size,
            mark: source.mark,
            low,
            high,
        });
        copied.insert(source_id, copied_id);
        Ok(copied_id)
    }

    fn transform_child<F>(
        &self,
        parent: ActVertexId,
        child: Option<ActVertexId>,
        output: &mut ActDag,
        copied: &mut HashMap<ActVertexId, ActVertexId>,
        transform_value: F,
    ) -> ActMiscResult<ActVertexId>
    where
        F: Fn(ActValue) -> ActValue + Copy,
    {
        let child = child.ok_or(ActMiscError::MissingChild { parent })?;
        self.transform_vertex(child, output, copied, transform_value)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActNode {
    pub global_act: Option<ActDag>,
    pub local_act: Option<ActDag>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActConstructionMode {
    Global,
    Local,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActMiscError {
    InvalidActValue(i32),
    MissingVertex(ActVertexId),
    MissingChild { parent: ActVertexId },
    MissingConstructedAct { mode: ActConstructionMode },
    MissingNativePorts { operation: &'static str },
}

impl fmt::Display for ActMiscError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidActValue(value) => write!(f, "invalid ACT vertex value {value}"),
            Self::MissingVertex(vertex) => write!(f, "missing ACT vertex {}", vertex.0),
            Self::MissingChild { parent } => {
                write!(f, "ACT vertex {} is missing a required child", parent.0)
            }
            Self::MissingConstructedAct { mode } => {
                write!(f, "ACT construction did not produce a {mode:?} root")
            }
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
        }
    }
}

impl Error for ActMiscError {}

pub fn p_act_node_free(node: &mut ActNode) -> bool {
    node.global_act.take().is_some()
}

pub fn p_act_size(act: &ActVertex) -> i32 {
    if act.id == 0 {
        act.value.as_c_value()
    } else {
        act.id + 1
    }
}

pub fn p_root_copy(act: &ActDag) -> ActMiscResult<ActDag> {
    act.root_copy()
}

pub fn p_dag_copy(act: &ActDag, root: ActVertexId) -> ActMiscResult<ActDag> {
    act.copy_from(root)
}

pub fn p_root_complement(act: &ActDag) -> ActMiscResult<ActDag> {
    act.root_complement()
}

pub fn p_dag_complement(act: &ActDag, root: ActVertexId) -> ActMiscResult<ActDag> {
    act.complement_from(root)
}

pub fn p_act_construct_blocked() -> ActMiscResult<ActVertexId> {
    Err(ActMiscError::MissingNativePorts {
        operation: "p_act_construct",
    })
}

pub fn p_act_construct_with<F>(
    node: &mut ActNode,
    mode: ActConstructionMode,
    mut construct: F,
) -> ActMiscResult<ActVertexId>
where
    F: FnMut(&mut ActNode, ActConstructionMode) -> ActMiscResult<ActDag>,
{
    let act = construct(node, mode)?;
    let root = act.root();
    match mode {
        ActConstructionMode::Global => {
            node.global_act = Some(act);
        }
        ActConstructionMode::Local => {
            node.local_act = Some(act);
        }
    }
    Ok(root)
}

pub fn constructed_root(node: &ActNode, mode: ActConstructionMode) -> ActMiscResult<ActVertexId> {
    match mode {
        ActConstructionMode::Global => node
            .global_act
            .as_ref()
            .map(ActDag::root)
            .ok_or(ActMiscError::MissingConstructedAct { mode }),
        ActConstructionMode::Local => node
            .local_act
            .as_ref()
            .map(ActDag::root)
            .ok_or(ActMiscError::MissingConstructedAct { mode }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared_child_dag() -> ActDag {
        let mut dag = ActDag::with_capacity(3);
        let zero = dag.add_vertex(ActVertex::terminal(0, false, 2));
        let one = dag.add_vertex(ActVertex::terminal(0, true, 2));
        let root = dag.add_vertex(ActVertex::internal(2, 0, 2, zero, one));
        dag.vertex_mut(root).unwrap().mark = 17;
        dag.set_root(root).unwrap();
        dag
    }

    #[test]
    fn root_copy_preserves_values_fields_and_shape() {
        let dag = shared_child_dag();

        let copy = p_root_copy(&dag).unwrap();

        assert_eq!(copy.root(), ActVertexId(2));
        assert_eq!(copy.vertices().len(), 3);
        let root = copy.vertex(copy.root()).unwrap();
        assert_eq!(root.id, 2);
        assert_eq!(root.value, ActValue::NoValue);
        assert_eq!(root.index, 0);
        assert_eq!(root.index_size, 2);
        assert_eq!(root.mark, 17);
        assert_eq!(
            copy.vertex(root.low.unwrap()).unwrap().value,
            ActValue::Zero
        );
        assert_eq!(
            copy.vertex(root.high.unwrap()).unwrap().value,
            ActValue::One
        );
    }

    #[test]
    fn root_copy_preserves_shared_children() {
        let mut dag = ActDag::with_capacity(2);
        let terminal = dag.add_vertex(ActVertex::terminal(0, true, 1));
        let root = dag.add_vertex(ActVertex::internal(1, 0, 1, terminal, terminal));
        dag.set_root(root).unwrap();

        let copy = p_root_copy(&dag).unwrap();

        let copied_root = copy.vertex(copy.root()).unwrap();
        assert_eq!(copied_root.low, copied_root.high);
        assert_eq!(copy.vertices().len(), 2);
    }

    #[test]
    fn root_complement_flips_terminals_and_keeps_internal_values() {
        let dag = shared_child_dag();

        let complement = p_root_complement(&dag).unwrap();

        let root = complement.vertex(complement.root()).unwrap();
        assert_eq!(root.value, ActValue::NoValue);
        assert_eq!(
            complement.vertex(root.low.unwrap()).unwrap().value,
            ActValue::One
        );
        assert_eq!(
            complement.vertex(root.high.unwrap()).unwrap().value,
            ActValue::Zero
        );
    }

    #[test]
    fn dag_copy_can_start_from_non_root() {
        let dag = shared_child_dag();

        let copy = p_dag_copy(&dag, ActVertexId(0)).unwrap();

        assert_eq!(copy.root(), ActVertexId(0));
        assert_eq!(copy.vertices(), &[ActVertex::terminal(0, false, 2)]);
    }

    #[test]
    fn p_act_size_matches_legacy_cases() {
        assert_eq!(p_act_size(&ActVertex::terminal(0, false, 2)), 0);
        assert_eq!(p_act_size(&ActVertex::terminal(0, true, 2)), 1);
        assert_eq!(
            p_act_size(&ActVertex::internal(
                3,
                0,
                2,
                ActVertexId(0),
                ActVertexId(1)
            )),
            4
        );
    }

    #[test]
    fn node_free_only_drops_global_act() {
        let global = ActDag::new(ActVertex::terminal(0, true, 0));
        let local = ActDag::new(ActVertex::terminal(0, false, 0));
        let mut node = ActNode {
            global_act: Some(global),
            local_act: Some(local.clone()),
        };

        assert!(p_act_node_free(&mut node));
        assert!(node.global_act.is_none());
        assert_eq!(node.local_act, Some(local));
        assert!(!p_act_node_free(&mut node));
    }

    #[test]
    fn construct_with_stores_graph_by_requested_mode() {
        let mut node = ActNode::default();

        let root = p_act_construct_with(&mut node, ActConstructionMode::Local, |_node, mode| {
            assert_eq!(mode, ActConstructionMode::Local);
            Ok(ActDag::new(ActVertex::terminal(0, true, 0)))
        })
        .unwrap();

        assert_eq!(root, ActVertexId(0));
        assert!(node.global_act.is_none());
        assert_eq!(
            constructed_root(&node, ActConstructionMode::Local),
            Ok(root)
        );
    }

    #[test]
    fn blocked_construct_reports_generic_runtime_diagnostic() {
        assert_eq!(
            p_act_construct_blocked(),
            Err(ActMiscError::MissingNativePorts {
                operation: "p_act_construct",
            })
        );
    }

    #[test]
    fn invalid_value_and_missing_children_are_errors() {
        assert_eq!(
            ActVertex::from_c_fields(0, 9, 0, 0, 0, None, None),
            Err(ActMiscError::InvalidActValue(9))
        );

        let dag = ActDag::new(
            ActVertex::from_c_fields(1, NO_VALUE, 0, 1, 0, None, Some(ActVertexId(0))).unwrap(),
        );

        assert_eq!(
            p_root_copy(&dag),
            Err(ActMiscError::MissingChild {
                parent: ActVertexId(0),
            })
        );
    }
}
