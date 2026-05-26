//! Native Rust model for `LogicSynthesis/sis/pld/ite_map.c`.
//!
//! The C file maps SIS ITE DAGs into ACTEL mux-pattern costs, coordinates
//! network/node mapping methods, and delegates several paths to still-unported
//! SIS graph, ACT, BDD, and decomposition code. This module ports the
//! deterministic ITE mapping behavior to owned Rust data and reports direct SIS
//! integration points as explicit dependency errors.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

pub const MAX_COST: i32 = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.297",
        source_file: "LogicSynthesis/sis/network/dfs.c",
        reason: "act_ite_preprocess and act_ite_map_network traverse networks with network_dfs",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node type, node function, literal count, fanin count, and node storage drive node mapping",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.317",
        source_file: "LogicSynthesis/sis/node/names.c",
        reason: "debug and diagnostics use node_long_name",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.344",
        source_file: "LogicSynthesis/sis/pld/act_bool.c",
        reason: "act_is_act_function detects nodes realizable by one ACTEL block",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.351",
        source_file: "LogicSynthesis/sis/pld/act_ite.c",
        reason: "legacy ACT/ITE support routines provide ACTEL node construction helpers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.353",
        source_file: "LogicSynthesis/sis/pld/act_map.c",
        reason: "my_create_act, act_init_multiple_fo_array, map_act, ACT globals, and ACT cleanup back BDD mapping",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.360",
        source_file: "LogicSynthesis/sis/pld/act_util.c",
        reason: "act_initialize_act_area initializes ACT DAG multiple-fanout mapping state",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.361",
        source_file: "LogicSynthesis/sis/pld/com_ite.c",
        reason: "command-level globals such as ACT_ITE_DEBUG, statistics, and act_is_or_used configure mapping",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.363",
        source_file: "LogicSynthesis/sis/pld/ite_break.c",
        reason: "act_ite_map_network_with_iter optionally breaks the final network into mapped blocks",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.365",
        source_file: "LogicSynthesis/sis/pld/ite_factor.c",
        reason: "factored-form ITE construction is one source of ACT_ITE_ite(node)",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.366",
        source_file: "LogicSynthesis/sis/pld/ite_imp.c",
        reason: "act_ite_iterative_improvement and alternate BDD remapping paths are invoked by this mapper",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.367",
        source_file: "LogicSynthesis/sis/pld/ite_leaf.c",
        reason: "ITE leaf handling participates in make_ite and canonical ITE construction",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.369",
        source_file: "LogicSynthesis/sis/pld/ite_mroot.c",
        reason: "multi-root ITE mapping shares the same act_map_ite pattern mapper",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.370",
        source_file: "LogicSynthesis/sis/pld/ite_new_map.c",
        reason: "MAP_WITH_ITER and MAP_WITH_JUST_DECOMP delegate to act_ite_map_node_with_iter_imp",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.371",
        source_file: "LogicSynthesis/sis/pld/ite_new_urp.c",
        reason: "NEW map method delegates to act_ite_new_map_node",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.372",
        source_file: "LogicSynthesis/sis/pld/ite_pld.c",
        reason: "ite_clear_dag and canonical ite_get storage are used after tree mapping",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.466",
        source_file: "LogicSynthesis/sis/decomp/decomp.c",
        reason: "act_ite_preprocess invokes decomp_quick_node for large literal-count nodes",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Buffer,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapNode {
    pub name: String,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub literal_count: usize,
    pub fanin_count: usize,
    pub cost: ActIteCost,
}

impl MapNode {
    pub fn internal(name: impl Into<String>, function: NodeFunction) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Internal,
            function,
            literal_count: 0,
            fanin_count: 0,
            cost: ActIteCost::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActIteCost {
    pub cost: i32,
    pub arrival_time: f64,
    pub has_match: bool,
    pub ite_root: Option<IteVertexId>,
    pub act_root_available: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapMethod {
    Old,
    New,
    WithIter,
    WithJustDecomp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActInitParam {
    pub heuristic_num: i32,
    pub map_method: MapMethod,
    pub break_network: bool,
    pub map_alg: i32,
    pub lit_bound: usize,
    pub ite_fanin_limit_for_bdd: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapOptions {
    pub use_or_patterns: bool,
}

impl Default for MapOptions {
    fn default() -> Self {
        Self {
            use_or_patterns: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteMapError {
    UnknownVertex(IteVertexId),
    MissingChild {
        vertex: IteVertexId,
        child: &'static str,
    },
    MissingIte {
        node: String,
    },
    UnknownMapMethod,
    HeuristicOutOfRange(i32),
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for IteMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVertex(vertex) => write!(f, "unknown ITE vertex {}", vertex.0),
            Self::MissingChild { vertex, child } => {
                write!(f, "ITE vertex {} is missing {child} child", vertex.0)
            }
            Self::MissingIte { node } => write!(f, "node {node} has no ITE root to map"),
            Self::UnknownMapMethod => write!(f, "mapping method is not known"),
            Self::HeuristicOutOfRange(heuristic) => {
                write!(f, "heuristic number {heuristic} is out of range")
            }
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => {
                write!(
                    f,
                    "{operation} requires native Rust ports for SIS dependencies: "
                )?;
                for (index, dependency) in dependencies.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} ({})", dependency.bead_id, dependency.source_file)?;
                }
                Ok(())
            }
        }
    }
}

impl Error for IteMapError {}

pub type IteMapResult<T> = Result<T, IteMapError>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IteVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IteValue {
    Zero,
    One,
    Literal,
    IfThenElse,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IteVertex {
    pub value: IteValue,
    pub phase: bool,
    pub if_child: Option<IteVertexId>,
    pub then_child: Option<IteVertexId>,
    pub else_child: Option<IteVertexId>,
    pub mark: bool,
    pub multiple_fo: usize,
    pub multiple_fo_for_mapping: usize,
    pub mapped: bool,
    pub cost: i32,
    pub pattern_num: Option<usize>,
    pub arrival_time: f64,
}

impl IteVertex {
    pub fn terminal(value: bool) -> Self {
        Self {
            value: if value { IteValue::One } else { IteValue::Zero },
            phase: true,
            if_child: None,
            then_child: None,
            else_child: None,
            mark: false,
            multiple_fo: 0,
            multiple_fo_for_mapping: 0,
            mapped: false,
            cost: 0,
            pattern_num: None,
            arrival_time: 0.0,
        }
    }

    pub fn literal(phase: bool) -> Self {
        Self {
            value: IteValue::Literal,
            phase,
            ..Self::terminal(false)
        }
    }

    pub fn ite(if_child: IteVertexId, then_child: IteVertexId, else_child: IteVertexId) -> Self {
        Self {
            value: IteValue::IfThenElse,
            if_child: Some(if_child),
            then_child: Some(then_child),
            else_child: Some(else_child),
            ..Self::terminal(false)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IteGraph {
    vertices: Vec<IteVertex>,
}

impl IteGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vertex(&mut self, vertex: IteVertex) -> IteVertexId {
        let id = IteVertexId(self.vertices.len());
        self.vertices.push(vertex);
        id
    }

    pub fn vertex(&self, id: IteVertexId) -> IteMapResult<&IteVertex> {
        self.vertices
            .get(id.0)
            .ok_or(IteMapError::UnknownVertex(id))
    }

    pub fn vertex_mut(&mut self, id: IteVertexId) -> IteMapResult<&mut IteVertex> {
        self.vertices
            .get_mut(id.0)
            .ok_or(IteMapError::UnknownVertex(id))
    }

    pub fn clear_dag_marks(&mut self, root: IteVertexId) -> IteMapResult<()> {
        let mut seen = HashSet::new();
        self.clear_dag_marks_from(root, &mut seen)
    }

    pub fn initialize_ite_area(&mut self, root: IteVertexId) -> IteMapResult<Vec<IteVertexId>> {
        let mut multiple_fo_roots = vec![root];
        self.initialize_ite_area_from(root, &mut multiple_fo_roots)?;
        Ok(multiple_fo_roots)
    }

    pub fn make_tree_and_map(
        &mut self,
        root: IteVertexId,
        options: &MapOptions,
    ) -> IteMapResult<i32> {
        let multiple_fo_roots = self.initialize_ite_area(root)?;
        let mut total = 0;
        for present in multiple_fo_roots.into_iter().rev() {
            total = saturated_add(total, self.map_ite(present, present, options)?);
        }
        self.clear_dag_marks(root)?;
        Ok(total)
    }

    pub fn map_ite(
        &mut self,
        vertex: IteVertexId,
        present_ite: IteVertexId,
        options: &MapOptions,
    ) -> IteMapResult<i32> {
        if self.vertex(vertex)?.multiple_fo > 0 && vertex != present_ite {
            return Ok(0);
        }
        if self.vertex(vertex)?.mapped {
            return Ok(self.vertex(vertex)?.cost);
        }

        self.vertex_mut(vertex)?.mapped = true;
        let snapshot = self.vertex(vertex)?.clone();
        match snapshot.value {
            IteValue::Zero | IteValue::One => {
                self.set_cost(vertex, 0, None)?;
                return Ok(0);
            }
            IteValue::Literal => {
                if snapshot.phase {
                    self.set_cost(vertex, 0, None)?;
                    return Ok(0);
                }
                self.set_cost(vertex, 1, Some(0))?;
                return Ok(1);
            }
            IteValue::IfThenElse => {}
        }

        let if_child = required_child(vertex, "IF", snapshot.if_child)?;
        let then_child = required_child(vertex, "THEN", snapshot.then_child)?;
        let else_child = required_child(vertex, "ELSE", snapshot.else_child)?;

        if self.is_positive_input_mux(if_child, then_child, else_child)? {
            self.set_cost(vertex, 0, None)?;
            return Ok(0);
        }

        let total_patterns = if options.use_or_patterns { 10 } else { 4 };
        let mut costs = [MAX_COST; 10];
        let if_cost = self.map_ite(if_child, present_ite, options)?;
        let then_cost = self.map_ite(then_child, present_ite, options)?;
        let else_cost = self.map_ite(else_child, present_ite, options)?;

        let mut temp_if_cost = MAX_COST;
        let mut or_pattern1 = false;
        let if_snapshot = self.vertex(if_child)?.clone();
        if if_snapshot.value == IteValue::IfThenElse {
            let if_then = required_child(if_child, "THEN", if_snapshot.then_child)?;
            if self.vertex(if_then)?.value == IteValue::One {
                or_pattern1 = true;
                if if_snapshot.multiple_fo == 0 {
                    let if_if = required_child(if_child, "IF", if_snapshot.if_child)?;
                    let if_else = required_child(if_child, "ELSE", if_snapshot.else_child)?;
                    temp_if_cost = saturated_add(
                        self.map_ite(if_if, present_ite, options)?,
                        self.map_ite(if_else, present_ite, options)?,
                    );
                }
            }
        }

        let then_complemented = self.is_complemented_literal(then_child)?;
        let else_complemented = self.is_complemented_literal(else_child)?;
        let then_is_ite = self.vertex(then_child)?.value == IteValue::IfThenElse;
        let else_is_ite = self.vertex(else_child)?.value == IteValue::IfThenElse;
        let cond_then = then_is_ite || then_complemented;
        let cond_else = else_is_ite || else_complemented;

        let temp_then_cost =
            self.embedded_branch_cost(then_child, then_complemented, present_ite, options)?;
        let temp_else_cost =
            self.embedded_branch_cost(else_child, else_complemented, present_ite, options)?;

        costs[0] = saturated_sum([if_cost, then_cost, else_cost, 1]);
        if cond_then {
            costs[1] = saturated_sum([if_cost, temp_then_cost, else_cost, 1]);
        }
        if cond_else {
            costs[2] = saturated_sum([if_cost, then_cost, temp_else_cost, 1]);
        }
        if cond_then && cond_else {
            costs[3] = saturated_sum([if_cost, temp_then_cost, temp_else_cost, 1]);
        }

        if options.use_or_patterns {
            if or_pattern1 && if_snapshot.multiple_fo == 0 {
                costs[4] = saturated_sum([temp_if_cost, then_cost, else_cost, 1]);
                if cond_then {
                    costs[5] = saturated_sum([temp_if_cost, temp_then_cost, else_cost, 1]);
                }
                if cond_else {
                    costs[6] = saturated_sum([temp_if_cost, then_cost, temp_else_cost, 1]);
                }
                if cond_then && cond_else {
                    costs[7] = saturated_sum([temp_if_cost, temp_then_cost, temp_else_cost, 1]);
                }
            }

            self.fill_or_pattern2_costs(
                &mut costs,
                vertex,
                if_cost,
                then_cost,
                temp_else_cost,
                present_ite,
                options,
            )?;
        }

        let best = minimum_cost_index(&costs[..total_patterns]);
        self.set_cost(vertex, costs[best], Some(best))?;
        Ok(costs[best])
    }

    fn initialize_ite_area_from(
        &mut self,
        vertex: IteVertexId,
        multiple_fo_roots: &mut Vec<IteVertexId>,
    ) -> IteMapResult<()> {
        let new_mark = !self.vertex(vertex)?.mark;
        {
            let vertex_ref = self.vertex_mut(vertex)?;
            vertex_ref.mark = new_mark;
            vertex_ref.pattern_num = None;
            vertex_ref.cost = 0;
            vertex_ref.arrival_time = 0.0;
            vertex_ref.mapped = false;
            vertex_ref.multiple_fo = 0;
            vertex_ref.multiple_fo_for_mapping = 0;
        }

        if self.vertex(vertex)?.value != IteValue::IfThenElse {
            return Ok(());
        }

        let snapshot = self.vertex(vertex)?.clone();
        for (name, child) in [
            ("IF", snapshot.if_child),
            ("THEN", snapshot.then_child),
            ("ELSE", snapshot.else_child),
        ] {
            let child = required_child(vertex, name, child)?;
            if self.vertex(vertex)?.mark != self.vertex(child)?.mark {
                self.initialize_ite_area_from(child, multiple_fo_roots)?;
            } else {
                if self.vertex(child)?.multiple_fo == 0 {
                    multiple_fo_roots.push(child);
                }
                let child_ref = self.vertex_mut(child)?;
                child_ref.multiple_fo += 1;
                child_ref.multiple_fo_for_mapping += 1;
            }
        }
        Ok(())
    }

    fn clear_dag_marks_from(
        &mut self,
        vertex: IteVertexId,
        seen: &mut HashSet<IteVertexId>,
    ) -> IteMapResult<()> {
        if !seen.insert(vertex) {
            return Ok(());
        }
        self.vertex_mut(vertex)?.mark = false;
        let snapshot = self.vertex(vertex)?.clone();
        if snapshot.value == IteValue::IfThenElse {
            for (name, child) in [
                ("IF", snapshot.if_child),
                ("THEN", snapshot.then_child),
                ("ELSE", snapshot.else_child),
            ] {
                self.clear_dag_marks_from(required_child(vertex, name, child)?, seen)?;
            }
        }
        Ok(())
    }

    fn set_cost(
        &mut self,
        vertex: IteVertexId,
        cost: i32,
        pattern_num: Option<usize>,
    ) -> IteMapResult<()> {
        let vertex_ref = self.vertex_mut(vertex)?;
        vertex_ref.cost = cost;
        vertex_ref.pattern_num = pattern_num;
        Ok(())
    }

    fn is_positive_input_mux(
        &self,
        if_child: IteVertexId,
        then_child: IteVertexId,
        else_child: IteVertexId,
    ) -> IteMapResult<bool> {
        let if_vertex = self.vertex(if_child)?;
        Ok(if_vertex.value == IteValue::Literal
            && if_vertex.phase
            && self.vertex(then_child)?.value == IteValue::One
            && self.vertex(else_child)?.value == IteValue::Zero)
    }

    fn is_complemented_literal(&self, vertex: IteVertexId) -> IteMapResult<bool> {
        let vertex_ref = self.vertex(vertex)?;
        Ok(vertex_ref.value == IteValue::Literal && !vertex_ref.phase)
    }

    fn embedded_branch_cost(
        &mut self,
        vertex: IteVertexId,
        complemented: bool,
        present_ite: IteVertexId,
        options: &MapOptions,
    ) -> IteMapResult<i32> {
        if complemented {
            return Ok(0);
        }
        if self.vertex(vertex)?.value != IteValue::IfThenElse {
            return Ok(MAX_COST);
        }
        if self.vertex(vertex)?.multiple_fo > 0 {
            return Ok(MAX_COST);
        }
        let snapshot = self.vertex(vertex)?.clone();
        let if_child = required_child(vertex, "IF", snapshot.if_child)?;
        let then_child = required_child(vertex, "THEN", snapshot.then_child)?;
        let else_child = required_child(vertex, "ELSE", snapshot.else_child)?;
        Ok(saturated_sum([
            self.map_ite(if_child, present_ite, options)?,
            self.map_ite(then_child, present_ite, options)?,
            self.map_ite(else_child, present_ite, options)?,
        ]))
    }

    fn fill_or_pattern2_costs(
        &mut self,
        costs: &mut [i32; 10],
        vertex: IteVertexId,
        if_cost: i32,
        then_cost: i32,
        temp_else_cost: i32,
        present_ite: IteVertexId,
        options: &MapOptions,
    ) -> IteMapResult<()> {
        let snapshot = self.vertex(vertex)?.clone();
        let then_child = required_child(vertex, "THEN", snapshot.then_child)?;
        let else_child = required_child(vertex, "ELSE", snapshot.else_child)?;
        let else_snapshot = self.vertex(else_child)?.clone();
        if else_snapshot.value != IteValue::IfThenElse || else_snapshot.multiple_fo > 0 {
            return Ok(());
        }

        let else_then = required_child(else_child, "THEN", else_snapshot.then_child)?;
        if !same_terminal_or_vertex(self, then_child, else_then)? {
            return Ok(());
        }

        costs[8] = saturated_sum([if_cost, temp_else_cost, 1]);
        let else_else = required_child(else_child, "ELSE", else_snapshot.else_child)?;
        if self.vertex(else_else)?.multiple_fo > 0 {
            return Ok(());
        }

        let else_else_complemented = self.is_complemented_literal(else_else)?;
        let else_else_is_ite = self.vertex(else_else)?.value == IteValue::IfThenElse;
        if !(else_else_complemented || else_else_is_ite) {
            return Ok(());
        }

        let else_if = required_child(else_child, "IF", else_snapshot.if_child)?;
        let else_if_cost = self.map_ite(else_if, present_ite, options)?;
        let temp_else_else_cost = if else_else_complemented {
            0
        } else {
            self.embedded_branch_cost(else_else, false, present_ite, options)?
        };
        costs[9] = saturated_sum([if_cost, then_cost, else_if_cost, temp_else_else_cost, 1]);
        Ok(())
    }
}

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn sis_bound_operation_unavailable(operation: &'static str) -> IteMapResult<()> {
    Err(missing_native_ports(operation))
}

pub fn act_vertex_ite_complemented(graph: &IteGraph, vertex: IteVertexId) -> IteMapResult<bool> {
    graph.is_complemented_literal(vertex)
}

pub fn act_ite_make_tree_and_map(
    graph: &mut IteGraph,
    root: IteVertexId,
    options: &MapOptions,
) -> IteMapResult<i32> {
    graph.make_tree_and_map(root, options)
}

pub fn act_map_ite(
    graph: &mut IteGraph,
    vertex: IteVertexId,
    present_ite: IteVertexId,
    options: &MapOptions,
) -> IteMapResult<i32> {
    graph.map_ite(vertex, present_ite, options)
}

pub fn act_ite_map_node_with_matcher<F>(
    node: &mut MapNode,
    init_param: &ActInitParam,
    graph: &mut IteGraph,
    options: &MapOptions,
    mut is_act_function: F,
) -> IteMapResult<i32>
where
    F: FnMut(&MapNode, i32, bool) -> bool,
{
    if node.kind == NodeKind::PrimaryInput || node.kind == NodeKind::PrimaryOutput {
        return Ok(0);
    }

    if is_act_function(node, init_param.map_alg, options.use_or_patterns) {
        node.cost.has_match = true;
        node.cost.cost = match node.function {
            NodeFunction::Zero | NodeFunction::One | NodeFunction::Buffer => 0,
            NodeFunction::Other => 1,
        };
        return Ok(node.cost.cost);
    }

    match init_param.heuristic_num {
        0 | 1 => {
            let root = node.cost.ite_root.ok_or_else(|| IteMapError::MissingIte {
                node: node.name.clone(),
            })?;
            let cost = graph.make_tree_and_map(root, options)?;
            node.cost.cost = cost;
            node.cost.arrival_time = graph.vertex(root)?.arrival_time;
            Ok(cost)
        }
        2 => Err(missing_native_ports("act_bdd_make_tree_and_map")),
        3 => {
            let root = node.cost.ite_root.ok_or_else(|| IteMapError::MissingIte {
                node: node.name.clone(),
            })?;
            let cost = graph.make_tree_and_map(root, options)?;
            node.cost.cost = cost;
            node.cost.arrival_time = graph.vertex(root)?.arrival_time;
            if cost > 2 && node.fanin_count <= init_param.ite_fanin_limit_for_bdd {
                return Err(missing_native_ports(
                    "act_bdd_make_tree_and_map alternate path",
                ));
            }
            Ok(cost)
        }
        heuristic => Err(IteMapError::HeuristicOutOfRange(heuristic)),
    }
}

pub fn act_ite_preprocess_blocked<Network>(
    _network: &mut Network,
    _init_param: &ActInitParam,
) -> IteMapResult<()> {
    Err(missing_native_ports("act_ite_preprocess"))
}

pub fn act_ite_map_network_blocked<Network>(
    _network: &mut Network,
    init_param: &ActInitParam,
) -> IteMapResult<i32> {
    match init_param.map_method {
        MapMethod::Old | MapMethod::New | MapMethod::WithIter | MapMethod::WithJustDecomp => {
            Err(missing_native_ports("act_ite_map_network"))
        }
    }
}

pub fn act_ite_map_network_with_iter_blocked<Network>(
    _network: &mut Network,
    _init_param: &ActInitParam,
) -> IteMapResult<()> {
    Err(missing_native_ports("act_ite_map_network_with_iter"))
}

pub fn ite_free_cost_struct(cost_struct: &mut Option<ActIteCost>) -> bool {
    cost_struct.take().is_some()
}

pub fn act_bdd_make_tree_and_map_blocked<Node, Act>(
    _node: &mut Node,
    _act_of_node: &mut Act,
) -> IteMapResult<i32> {
    Err(missing_native_ports("act_bdd_make_tree_and_map"))
}

fn minimum_cost_index(cost: &[i32]) -> usize {
    let mut min_index = 0;
    for index in 1..cost.len() {
        if cost[index] < cost[min_index] {
            min_index = index;
        }
    }
    min_index
}

fn required_child(
    vertex: IteVertexId,
    child: &'static str,
    id: Option<IteVertexId>,
) -> IteMapResult<IteVertexId> {
    id.ok_or(IteMapError::MissingChild { vertex, child })
}

fn same_terminal_or_vertex(
    graph: &IteGraph,
    left: IteVertexId,
    right: IteVertexId,
) -> IteMapResult<bool> {
    if left == right {
        return Ok(true);
    }
    let left_value = graph.vertex(left)?.value;
    let right_value = graph.vertex(right)?.value;
    Ok(matches!(
        (left_value, right_value),
        (IteValue::Zero, IteValue::Zero) | (IteValue::One, IteValue::One)
    ))
}

fn saturated_sum(values: impl IntoIterator<Item = i32>) -> i32 {
    values
        .into_iter()
        .fold(0, |total, value| saturated_add(total, value))
}

fn saturated_add(left: i32, right: i32) -> i32 {
    left.saturating_add(right).min(MAX_COST)
}

fn missing_native_ports(operation: &'static str) -> IteMapError {
    IteMapError::MissingNativePorts {
        operation,
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(heuristic_num: i32) -> ActInitParam {
        ActInitParam {
            heuristic_num,
            map_method: MapMethod::Old,
            break_network: false,
            map_alg: 0,
            lit_bound: 20,
            ite_fanin_limit_for_bdd: 4,
        }
    }

    #[test]
    fn complemented_literal_costs_one_positive_literal_costs_zero() {
        let mut graph = IteGraph::new();
        let pos = graph.add_vertex(IteVertex::literal(true));
        let neg = graph.add_vertex(IteVertex::literal(false));

        assert_eq!(
            act_map_ite(&mut graph, pos, pos, &MapOptions::default()).unwrap(),
            0
        );
        assert_eq!(
            act_map_ite(&mut graph, neg, neg, &MapOptions::default()).unwrap(),
            1
        );
        assert_eq!(graph.vertex(neg).unwrap().pattern_num, Some(0));
    }

    #[test]
    fn positive_input_mux_is_zero_cost_variable() {
        let mut graph = IteGraph::new();
        let x = graph.add_vertex(IteVertex::literal(true));
        let one = graph.add_vertex(IteVertex::terminal(true));
        let zero = graph.add_vertex(IteVertex::terminal(false));
        let root = graph.add_vertex(IteVertex::ite(x, one, zero));

        let cost = graph
            .make_tree_and_map(root, &MapOptions::default())
            .unwrap();

        assert_eq!(cost, 0);
        assert_eq!(graph.vertex(root).unwrap().cost, 0);
        assert_eq!(graph.vertex(root).unwrap().pattern_num, None);
    }

    #[test]
    fn basic_mux_uses_first_pattern_when_or_patterns_are_disabled() {
        let mut graph = IteGraph::new();
        let x = graph.add_vertex(IteVertex::literal(true));
        let inv_y = graph.add_vertex(IteVertex::literal(false));
        let zero = graph.add_vertex(IteVertex::terminal(false));
        let root = graph.add_vertex(IteVertex::ite(x, inv_y, zero));

        let cost = graph
            .make_tree_and_map(
                root,
                &MapOptions {
                    use_or_patterns: false,
                },
            )
            .unwrap();

        assert_eq!(cost, 1);
        assert_eq!(graph.vertex(root).unwrap().pattern_num, Some(1));
    }

    #[test]
    fn or_pattern1_candidate_keeps_earliest_pattern_on_equal_cost() {
        let mut graph = IteGraph::new();
        let x = graph.add_vertex(IteVertex::literal(true));
        let y = graph.add_vertex(IteVertex::literal(true));
        let one = graph.add_vertex(IteVertex::terminal(true));
        let zero = graph.add_vertex(IteVertex::terminal(false));
        let if_or = graph.add_vertex(IteVertex::ite(x, one, zero));
        let root = graph.add_vertex(IteVertex::ite(if_or, y, zero));

        let cost = graph
            .make_tree_and_map(root, &MapOptions::default())
            .unwrap();

        assert_eq!(cost, 1);
        assert_eq!(graph.vertex(root).unwrap().pattern_num, Some(0));
    }

    #[test]
    fn initialization_detects_shared_vertices_and_maps_each_root_bottom_up() {
        let mut graph = IteGraph::new();
        let x = graph.add_vertex(IteVertex::literal(true));
        let inv = graph.add_vertex(IteVertex::literal(false));
        let zero = graph.add_vertex(IteVertex::terminal(false));
        let shared = graph.add_vertex(IteVertex::ite(x, inv, zero));
        let root = graph.add_vertex(IteVertex::ite(x, shared, shared));

        let roots = graph.initialize_ite_area(root).unwrap();

        assert_eq!(roots, vec![root, x, shared]);
        assert_eq!(graph.vertex(shared).unwrap().multiple_fo, 1);
        assert_eq!(graph.vertex(x).unwrap().multiple_fo, 1);

        let cost = graph
            .make_tree_and_map(root, &MapOptions::default())
            .unwrap();
        assert_eq!(cost, 2);
        assert!(!graph.vertex(root).unwrap().mark);
        assert!(!graph.vertex(shared).unwrap().mark);
    }

    #[test]
    fn map_node_sets_zero_cost_for_single_block_constants_and_buffers() {
        let mut graph = IteGraph::new();
        let mut node = MapNode::internal("buf", NodeFunction::Buffer);

        let cost = act_ite_map_node_with_matcher(
            &mut node,
            &params(0),
            &mut graph,
            &MapOptions::default(),
            |_node, _map_alg, _or_used| true,
        )
        .unwrap();

        assert_eq!(cost, 0);
        assert!(node.cost.has_match);
    }

    #[test]
    fn map_node_maps_existing_ite_for_heuristic_zero() {
        let mut graph = IteGraph::new();
        let inv = graph.add_vertex(IteVertex::literal(false));
        let mut node = MapNode::internal("n", NodeFunction::Other);
        node.cost.ite_root = Some(inv);

        let cost = act_ite_map_node_with_matcher(
            &mut node,
            &params(0),
            &mut graph,
            &MapOptions::default(),
            |_node, _map_alg, _or_used| false,
        )
        .unwrap();

        assert_eq!(cost, 1);
        assert_eq!(node.cost.cost, 1);
    }

    #[test]
    fn heuristic_three_reports_bdd_dependency_when_alternate_path_is_needed() {
        let mut graph = IteGraph::new();
        let x = graph.add_vertex(IteVertex::literal(true));
        let y = graph.add_vertex(IteVertex::literal(false));
        let z = graph.add_vertex(IteVertex::literal(false));
        let a = graph.add_vertex(IteVertex::ite(x, y, z));
        let root = graph.add_vertex(IteVertex::ite(x, a, y));
        let mut node = MapNode::internal("n", NodeFunction::Other);
        node.fanin_count = 3;
        node.cost.ite_root = Some(root);

        let Err(IteMapError::MissingNativePorts {
            operation,
            dependencies,
        }) = act_ite_map_node_with_matcher(
            &mut node,
            &params(3),
            &mut graph,
            &MapOptions {
                use_or_patterns: false,
            },
            |_node, _map_alg, _or_used| false,
        )
        else {
            panic!("expected BDD missing dependency");
        };

        assert_eq!(operation, "act_bdd_make_tree_and_map alternate path");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.353"
                && dependency.source_file == "LogicSynthesis/sis/pld/act_map.c"
        }));
    }

    #[test]
    fn blocked_entries_report_dependency_beads_and_sources() {
        let error = sis_bound_operation_unavailable("act_ite_preprocess").unwrap_err();
        let IteMapError::MissingNativePorts {
            operation,
            dependencies,
        } = error
        else {
            panic!("expected missing native ports");
        };

        assert_eq!(operation, "act_ite_preprocess");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.297"
                && dependency.source_file == "LogicSynthesis/sis/network/dfs.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.466"
                && dependency.source_file == "LogicSynthesis/sis/decomp/decomp.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.370"
                && dependency.source_file == "LogicSynthesis/sis/pld/ite_new_map.c"
        }));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("ite_map.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
