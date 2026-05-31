//! Native Rust model for `LogicSynthesis/sis/pld/ite_pld.c`.
//!
//! The C file owns the low-level ITE vertex allocator plus DAG numbering,
//! printing, and mark clearing. This port keeps those behaviors on an owned
//! Rust graph. SIS-backed printing that must call `node_long_name(node_t *)`
//! remains represented as an explicit dependency error until the node/name
//! ports are wired into the native model.

use std::error::Error;
use std::fmt;

const ITE_TERMINAL_TABLE_CAPACITY: usize = 100;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IteVertexId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutIte {
    pub parent_ite_ptr: Option<IteVertexId>,
    pub next: Option<Box<FanoutIte>>,
}

impl FanoutIte {
    pub fn allocated_default() -> Self {
        Self {
            parent_ite_ptr: None,
            next: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteVertex {
    pub id: IteVertexId,
    pub if_child: Option<IteVertexId>,
    pub then_child: Option<IteVertexId>,
    pub else_child: Option<IteVertexId>,
    pub multiple_fo: bool,
    pub mark: i32,
    pub index_size: i32,
    pub print_mark: bool,
    pub fanout: Option<FanoutIte>,
    pub fanin: Option<NodeId>,
    pub value: i32,
    pub cost: i32,
    pub pattern_num: i32,
    pub phase: i32,
}

impl IteVertex {
    pub fn allocated_default(id: usize) -> Self {
        Self {
            id: IteVertexId(id),
            if_child: None,
            then_child: None,
            else_child: None,
            multiple_fo: false,
            mark: 0,
            index_size: 0,
            print_mark: false,
            fanout: Some(FanoutIte::allocated_default()),
            fanin: None,
            value: 0,
            cost: 0,
            pattern_num: 0,
            phase: 0,
        }
    }

    pub fn zero(id: usize) -> Self {
        Self {
            value: 0,
            ..Self::allocated_default(id)
        }
    }

    pub fn one(id: usize) -> Self {
        Self {
            value: 1,
            ..Self::allocated_default(id)
        }
    }

    pub fn literal(id: usize, fanin: NodeId, phase: i32) -> Self {
        Self {
            value: 2,
            fanin: Some(fanin),
            phase,
            ..Self::allocated_default(id)
        }
    }

    pub fn branch(
        id: usize,
        if_child: IteVertexId,
        then_child: IteVertexId,
        else_child: IteVertexId,
    ) -> Self {
        Self {
            value: 3,
            if_child: Some(if_child),
            then_child: Some(then_child),
            else_child: Some(else_child),
            ..Self::allocated_default(id)
        }
    }

    fn is_terminal(&self) -> bool {
        (0..=2).contains(&self.value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteTerminal {
    pub fanin: NodeId,
    pub node_num: i32,
    pub ite_node: IteVertexId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItePldError {
    MissingNativePorts {
        operation: &'static str,
    },
    MissingVertex(IteVertexId),
    MissingChild {
        parent: IteVertexId,
        child_name: &'static str,
    },
    MissingFanin(IteVertexId),
    MissingNodeName(NodeId),
    TerminalTableOverflow {
        capacity: usize,
    },
}

impl fmt::Display for ItePldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} is blocked by unported SIS C-file dependencies"
            ),
            Self::MissingVertex(vertex) => write!(f, "missing ITE vertex {}", vertex.0),
            Self::MissingChild { parent, child_name } => {
                write!(f, "ITE vertex {} is missing {child_name} child", parent.0)
            }
            Self::MissingFanin(vertex) => write!(f, "literal ITE vertex {} has no fanin", vertex.0),
            Self::MissingNodeName(node) => write!(f, "missing long name for SIS node {}", node.0),
            Self::TerminalTableOverflow { capacity } => {
                write!(f, "ITE terminal table exceeded fixed C capacity {capacity}")
            }
        }
    }
}

impl Error for ItePldError {}

pub type ItePldResult<T> = Result<T, ItePldError>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IteGraph {
    vertices: Vec<IteVertex>,
}

impl IteGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ite_alloc(&mut self) -> IteVertexId {
        let id = IteVertexId(self.vertices.len());
        self.vertices.push(IteVertex::allocated_default(id.0));
        id
    }

    pub fn add_vertex(&mut self, mut vertex: IteVertex) -> IteVertexId {
        let id = IteVertexId(self.vertices.len());
        vertex.id = id;
        self.vertices.push(vertex);
        id
    }

    pub fn vertex(&self, id: IteVertexId) -> ItePldResult<&IteVertex> {
        self.vertices
            .get(id.0)
            .ok_or(ItePldError::MissingVertex(id))
    }

    pub fn vertex_mut(&mut self, id: IteVertexId) -> ItePldResult<&mut IteVertex> {
        self.vertices
            .get_mut(id.0)
            .ok_or(ItePldError::MissingVertex(id))
    }

    pub fn assign_numbers(&mut self, root: IteVertexId) -> ItePldResult<Vec<IteTerminal>> {
        let mut next_node_num = 1;
        let mut terminals = Vec::new();
        self.assign_numbers_from(root, &mut next_node_num, &mut terminals)?;
        Ok(terminals)
    }

    pub fn print_lines<F>(
        &mut self,
        root: IteVertexId,
        mut node_long_name: F,
    ) -> ItePldResult<Vec<String>>
    where
        F: FnMut(NodeId) -> Option<String>,
    {
        let terminals = self.assign_numbers(root)?;
        let mut lines = Vec::new();
        self.print_from(root, &terminals, &mut node_long_name, &mut lines)?;
        self.clear_dag(root)?;
        Ok(lines)
    }

    pub fn print_out_lines<F>(
        &mut self,
        root: IteVertexId,
        node_long_name: F,
    ) -> ItePldResult<Vec<String>>
    where
        F: FnMut(NodeId) -> Option<String>,
    {
        self.print_lines(root, node_long_name)
    }

    pub fn clear_dag(&mut self, root: IteVertexId) -> ItePldResult<()> {
        if self.vertex(root)?.mark == 0 {
            return Ok(());
        }

        {
            let vertex = self.vertex_mut(root)?;
            vertex.mark = 0;
            vertex.print_mark = false;
        }

        let (if_child, then_child, else_child) = {
            let vertex = self.vertex(root)?;
            (vertex.if_child, vertex.then_child, vertex.else_child)
        };

        if let Some(if_child) = if_child {
            self.clear_dag(if_child)?;
            self.clear_dag(required_child(self.vertex(root)?, then_child, "THEN")?)?;
            self.clear_dag(required_child(self.vertex(root)?, else_child, "ELSE")?)?;
        }

        Ok(())
    }

    fn assign_numbers_from(
        &mut self,
        id: IteVertexId,
        next_node_num: &mut i32,
        terminals: &mut Vec<IteTerminal>,
    ) -> ItePldResult<()> {
        if self.vertex(id)?.mark != 0 {
            return Ok(());
        }

        let (value, phase, fanin, if_child, then_child, else_child) = {
            let vertex = self.vertex(id)?;
            (
                vertex.value,
                vertex.phase,
                vertex.fanin,
                vertex.if_child,
                vertex.then_child,
                vertex.else_child,
            )
        };

        if value == 2 && phase == 1 {
            if terminals.len() == ITE_TERMINAL_TABLE_CAPACITY {
                return Err(ItePldError::TerminalTableOverflow {
                    capacity: ITE_TERMINAL_TABLE_CAPACITY,
                });
            }
            terminals.push(IteTerminal {
                fanin: fanin.ok_or(ItePldError::MissingFanin(id))?,
                node_num: *next_node_num,
                ite_node: id,
            });
        }

        self.vertex_mut(id)?.mark = *next_node_num;
        *next_node_num += 1;

        if !(0..=2).contains(&value) {
            if let Some(if_child) = if_child {
                self.assign_numbers_from(if_child, next_node_num, terminals)?;
                self.assign_numbers_from(
                    required_child(self.vertex(id)?, then_child, "THEN")?,
                    next_node_num,
                    terminals,
                )?;
                self.assign_numbers_from(
                    required_child(self.vertex(id)?, else_child, "ELSE")?,
                    next_node_num,
                    terminals,
                )?;
            }
        }

        Ok(())
    }

    fn print_from<F>(
        &mut self,
        id: IteVertexId,
        terminals: &[IteTerminal],
        node_long_name: &mut F,
        lines: &mut Vec<String>,
    ) -> ItePldResult<()>
    where
        F: FnMut(NodeId) -> Option<String>,
    {
        if self.vertex(id)?.print_mark {
            return Ok(());
        }

        let vertex = self.vertex(id)?.clone();
        if vertex.is_terminal() {
            lines.push(self.format_terminal(&vertex, terminals, node_long_name)?);
            self.vertex_mut(id)?.print_mark = true;
            return Ok(());
        }

        let if_child = required_child(&vertex, vertex.if_child, "IF")?;
        let then_child = required_child(&vertex, vertex.then_child, "THEN")?;
        let else_child = required_child(&vertex, vertex.else_child, "ELSE")?;
        let if_mark = self.vertex(if_child)?.mark;
        let then_mark = self.vertex(then_child)?.mark;
        let else_mark = self.vertex(else_child)?.mark;

        lines.push(format!(
            "[{}]=[{}, {}, {}], cost = {}, pattern_num = {}",
            vertex.mark, if_mark, then_mark, else_mark, vertex.cost, vertex.pattern_num
        ));
        self.vertex_mut(id)?.print_mark = true;
        self.print_from(if_child, terminals, node_long_name, lines)?;
        self.print_from(then_child, terminals, node_long_name, lines)?;
        self.print_from(else_child, terminals, node_long_name, lines)?;
        Ok(())
    }

    fn format_terminal<F>(
        &self,
        vertex: &IteVertex,
        terminals: &[IteTerminal],
        node_long_name: &mut F,
    ) -> ItePldResult<String>
    where
        F: FnMut(NodeId) -> Option<String>,
    {
        match vertex.value {
            0 => Ok(format!("[{}]=0", vertex.mark)),
            1 => Ok(format!("[{}]=1", vertex.mark)),
            2 => {
                let fanin = vertex.fanin.ok_or(ItePldError::MissingFanin(vertex.id))?;
                let name = node_long_name(fanin).ok_or(ItePldError::MissingNodeName(fanin))?;
                if vertex.phase == 0 {
                    let positive_seen = terminals.iter().any(|terminal| terminal.fanin == fanin);
                    if positive_seen {
                        Ok(format!("[{}]={}'", vertex.mark, name))
                    } else {
                        Ok(format!("[{}]'={}'", vertex.mark, name))
                    }
                } else {
                    Ok(format!("[{}]={}", vertex.mark, name))
                }
            }
            _ => unreachable!("format_terminal called for nonterminal ITE value"),
        }
    }
}

pub fn ite_print_dag_blocked<LegacyIte>(_ite: &LegacyIte) -> ItePldResult<Vec<String>> {
    Err(missing_native_ports(
        "ite_print_dag SIS node_long_name integration",
    ))
}

pub fn ite_print_out_blocked<LegacyIte>(_ite: &LegacyIte) -> ItePldResult<Vec<String>> {
    Err(missing_native_ports(
        "ite_print_out SIS node_long_name integration",
    ))
}

fn missing_native_ports(operation: &'static str) -> ItePldError {
    ItePldError::MissingNativePorts { operation }
}

fn required_child(
    vertex: &IteVertex,
    child: Option<IteVertexId>,
    child_name: &'static str,
) -> ItePldResult<IteVertexId> {
    child.ok_or(ItePldError::MissingChild {
        parent: vertex.id,
        child_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name_for(node: NodeId) -> Option<String> {
        match node.0 {
            1 => Some("a".to_owned()),
            2 => Some("b".to_owned()),
            _ => None,
        }
    }

    #[test]
    fn ite_alloc_matches_c_initializer_defaults() {
        let mut graph = IteGraph::new();
        let id = graph.ite_alloc();
        let vertex = graph.vertex(id).unwrap();

        assert_eq!(vertex.if_child, None);
        assert_eq!(vertex.then_child, None);
        assert_eq!(vertex.else_child, None);
        assert!(!vertex.multiple_fo);
        assert_eq!(vertex.mark, 0);
        assert_eq!(vertex.index_size, 0);
        assert!(!vertex.print_mark);
        assert_eq!(vertex.fanout, Some(FanoutIte::allocated_default()));
    }

    #[test]
    fn assign_numbers_is_preorder_and_skips_shared_vertices() {
        let mut graph = IteGraph::new();
        let lit = graph.add_vertex(IteVertex::literal(0, NodeId(1), 1));
        let one = graph.add_vertex(IteVertex::one(0));
        let root = graph.add_vertex(IteVertex::branch(0, lit, one, lit));

        let terminals = graph.assign_numbers(root).unwrap();

        assert_eq!(graph.vertex(root).unwrap().mark, 1);
        assert_eq!(graph.vertex(lit).unwrap().mark, 2);
        assert_eq!(graph.vertex(one).unwrap().mark, 3);
        assert_eq!(
            terminals,
            vec![IteTerminal {
                fanin: NodeId(1),
                node_num: 2,
                ite_node: lit,
            }]
        );
    }

    #[test]
    fn print_lines_matches_c_order_and_clears_marks_afterward() {
        let mut graph = IteGraph::new();
        let condition = graph.add_vertex(IteVertex::literal(0, NodeId(1), 1));
        let then_child = graph.add_vertex(IteVertex::one(0));
        let else_child = graph.add_vertex(IteVertex::literal(0, NodeId(2), 0));
        let root = graph.add_vertex(IteVertex {
            cost: 7,
            pattern_num: 11,
            ..IteVertex::branch(0, condition, then_child, else_child)
        });

        let lines = graph.print_lines(root, name_for).unwrap();

        assert_eq!(
            lines,
            vec![
                "[1]=[2, 3, 4], cost = 7, pattern_num = 11",
                "[2]=a",
                "[3]=1",
                "[4]'=b'",
            ]
        );
        assert_eq!(graph.vertex(root).unwrap().mark, 0);
        assert!(!graph.vertex(condition).unwrap().print_mark);
    }

    #[test]
    fn negative_literal_uses_positive_terminal_table_when_seen() {
        let mut graph = IteGraph::new();
        let positive = graph.add_vertex(IteVertex::literal(0, NodeId(1), 1));
        let negative = graph.add_vertex(IteVertex::literal(0, NodeId(1), 0));
        let root = graph.add_vertex(IteVertex::branch(0, positive, negative, positive));

        let lines = graph.print_lines(root, name_for).unwrap();

        assert_eq!(
            lines,
            vec![
                "[1]=[2, 3, 2], cost = 0, pattern_num = 0",
                "[2]=a",
                "[3]=a'",
            ]
        );
    }

    #[test]
    fn clear_dag_respects_already_clear_roots() {
        let mut graph = IteGraph::new();
        let root = graph.add_vertex(IteVertex::one(0));

        graph.clear_dag(root).unwrap();

        assert_eq!(graph.vertex(root).unwrap().mark, 0);
        assert!(!graph.vertex(root).unwrap().print_mark);
    }
}
