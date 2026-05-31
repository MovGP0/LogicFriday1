//! Owned Rust model for ACT mapping.
//!
//! This module ports the deterministic ACT tree splitting and mux-pattern
//! costing behavior. Operations that still need direct SIS network mutation or
//! legacy ACT construction report generic missing-native-port diagnostics.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

pub const MAX_COST: i32 = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapMode {
    Area,
    Delay,
    Mixed,
}

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
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActInitParams {
    pub mode: MapMode,
    pub heuristic_num: i32,
    pub num_iter: usize,
    pub quick_phase: bool,
    pub disjoint_decomp: bool,
    pub last_gasp: bool,
    pub break_network: bool,
}

impl Default for ActInitParams {
    fn default() -> Self {
        Self {
            mode: MapMode::Area,
            heuristic_num: 1,
            num_iter: 0,
            quick_phase: false,
            disjoint_decomp: false,
            last_gasp: false,
            break_network: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActMapOptions {
    pub use_or_patterns: bool,
    pub delay_weight: f64,
    pub node_fanout_count: usize,
}

impl Default for ActMapOptions {
    fn default() -> Self {
        Self {
            use_or_patterns: true,
            delay_weight: 0.0,
            node_fanout_count: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CostStruct {
    pub node_name: String,
    pub cost: i32,
    pub arrival_time: f64,
    pub cost_and_arrival_time: f64,
    pub act_root: Option<ActVertexId>,
}

impl CostStruct {
    pub fn constant(node_name: impl Into<String>) -> Self {
        Self {
            node_name: node_name.into(),
            cost: 0,
            arrival_time: 0.0,
            cost_and_arrival_time: -1.0,
            act_root: None,
        }
    }

    pub fn weighted_cost(&self, delay_weight: f64) -> f64 {
        if self.cost_and_arrival_time >= 0.0 {
            return self.cost_and_arrival_time;
        }

        weighted_cost_delay(self.cost, self.arrival_time, delay_weight)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActMapNode {
    pub name: String,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub act_root: Option<ActVertexId>,
    pub fanout_count: usize,
}

impl ActMapNode {
    pub fn internal(name: impl Into<String>, function: NodeFunction) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Internal,
            function,
            act_root: None,
            fanout_count: 1,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActMapNetwork {
    nodes: Vec<ActMapNode>,
}

impl ActMapNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: ActMapNode) -> usize {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    pub fn nodes(&self) -> &[ActMapNode] {
        &self.nodes
    }

    pub fn total_internal_cost(&self, costs: &HashMap<String, CostStruct>) -> ActMapResult<i32> {
        let mut total = 0;
        for node in &self.nodes {
            if node.kind != NodeKind::Internal {
                continue;
            }

            let cost = costs
                .get(&node.name)
                .ok_or_else(|| ActMapError::MissingCost {
                    node: node.name.clone(),
                })?;
            total = saturated_add(total, cost.cost);
        }
        Ok(total)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActVertexId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActValue {
    Zero,
    One,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActVertex {
    pub value: ActValue,
    pub low: Option<ActVertexId>,
    pub high: Option<ActVertexId>,
    pub name: Option<String>,
    pub index: usize,
    pub mark: bool,
    pub multiple_fo: usize,
    pub multiple_fo_for_mapping: usize,
    pub mapped: bool,
    pub cost: i32,
    pub pattern_num: Option<usize>,
    pub arrival_time: f64,
    pub input_arrival_time: f64,
}

impl ActVertex {
    pub fn terminal(value: bool) -> Self {
        Self {
            value: if value { ActValue::One } else { ActValue::Zero },
            low: None,
            high: None,
            name: None,
            index: 0,
            mark: false,
            multiple_fo: 0,
            multiple_fo_for_mapping: 0,
            mapped: false,
            cost: 0,
            pattern_num: None,
            arrival_time: 0.0,
            input_arrival_time: 0.0,
        }
    }

    pub fn internal(low: ActVertexId, high: ActVertexId) -> Self {
        Self {
            value: ActValue::Internal,
            low: Some(low),
            high: Some(high),
            ..Self::terminal(false)
        }
    }

    pub fn named_input(name: impl Into<String>, low: ActVertexId, high: ActVertexId) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::internal(low, high)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActGraph {
    vertices: Vec<ActVertex>,
}

impl ActGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vertex(&mut self, vertex: ActVertex) -> ActVertexId {
        let id = ActVertexId(self.vertices.len());
        self.vertices.push(vertex);
        id
    }

    pub fn vertex(&self, id: ActVertexId) -> ActMapResult<&ActVertex> {
        self.vertices
            .get(id.0)
            .ok_or(ActMapError::UnknownVertex(id))
    }

    pub fn vertex_mut(&mut self, id: ActVertexId) -> ActMapResult<&mut ActVertex> {
        self.vertices
            .get_mut(id.0)
            .ok_or(ActMapError::UnknownVertex(id))
    }

    pub fn put_node_names_in_act(
        &mut self,
        root: ActVertexId,
        names: &[String],
    ) -> ActMapResult<()> {
        let mut seen = HashSet::new();
        self.put_node_names_from(root, names, &mut seen)
    }

    pub fn initialize_act_area(&mut self, root: ActVertexId) -> ActMapResult<Vec<ActVertexId>> {
        let mut multiple_fo_roots = vec![root];
        let mut seen = HashSet::new();
        self.initialize_area_from(root, &mut seen, &mut multiple_fo_roots)?;
        Ok(multiple_fo_roots)
    }

    pub fn initialize_act_delay(&mut self, root: ActVertexId) -> ActMapResult<Vec<ActVertexId>> {
        let mut seen = HashSet::new();
        self.initialize_delay_from(root, &mut seen)?;

        let mut multiple_fo_roots = vec![root];
        let mut collected = HashSet::from([root]);
        let mut traversal_seen = HashSet::new();
        self.collect_delay_multiple_fo(
            root,
            &mut traversal_seen,
            &mut collected,
            &mut multiple_fo_roots,
        )?;
        Ok(multiple_fo_roots)
    }

    pub fn clear_marks(&mut self, root: ActVertexId) -> ActMapResult<()> {
        let mut seen = HashSet::new();
        self.clear_marks_from(root, &mut seen)
    }

    pub fn make_tree_and_map(
        &mut self,
        node_name: impl Into<String>,
        root: ActVertexId,
        options: &ActMapOptions,
    ) -> ActMapResult<CostStruct> {
        let roots = self.initialize_act_area(root)?;
        let mut total = 0;
        for present in roots.into_iter().rev() {
            total = saturated_add(total, self.map_act(present, present, options)?);
        }

        let arrival_time = self.vertex(root)?.arrival_time;
        self.clear_marks(root)?;
        Ok(CostStruct {
            node_name: node_name.into(),
            cost: total,
            arrival_time,
            cost_and_arrival_time: -1.0,
            act_root: Some(root),
        })
    }

    pub fn make_tree_and_map_delay(
        &mut self,
        node_name: impl Into<String>,
        root: ActVertexId,
        delay_values: &DelayValues,
        options: &ActMapOptions,
    ) -> ActMapResult<CostStruct> {
        let roots = self.initialize_act_delay(root)?;
        let mut total = 0;
        for present in roots.into_iter().rev() {
            total = saturated_add(
                total,
                self.map_act_delay(present, present, delay_values, options)?,
            );
        }

        let correction = delay_values.node_fanout_correction(1, options.node_fanout_count.max(1));
        let arrival_time = self.vertex(root)?.arrival_time + correction;
        self.clear_marks(root)?;
        Ok(CostStruct {
            node_name: node_name.into(),
            cost: total,
            arrival_time,
            cost_and_arrival_time: weighted_cost_delay(total, arrival_time, options.delay_weight),
            act_root: Some(root),
        })
    }

    pub fn map_act(
        &mut self,
        vertex: ActVertexId,
        present_act: ActVertexId,
        options: &ActMapOptions,
    ) -> ActMapResult<i32> {
        if self.vertex(vertex)?.mapped {
            return Ok(self.vertex(vertex)?.cost);
        }

        if self.vertex(vertex)?.multiple_fo != 0 && vertex != present_act {
            self.vertex_mut(vertex)?.cost = 0;
            return Ok(0);
        }

        self.vertex_mut(vertex)?.mapped = true;
        if self.vertex(vertex)?.value != ActValue::Internal {
            self.set_cost(vertex, 0, None, 0.0)?;
            return Ok(0);
        }

        let (low, high) = self.children(vertex)?;
        if self.is_simple_input(low, high)? {
            self.set_cost(vertex, 0, None, 0.0)?;
            return Ok(0);
        }

        let total_patterns = if options.use_or_patterns { 12 } else { 4 };
        let mut costs = [MAX_COST; 12];
        let vlow_cost = self.child_cost_area(low, present_act, options)?;
        let vhigh_cost = self.child_cost_area(high, present_act, options)?;
        let (vlowlow_cost, vlowhigh_cost) =
            self.grandchild_costs_area(low, present_act, options)?;
        let (vhighlow_cost, vhighhigh_cost) =
            self.grandchild_costs_area(high, present_act, options)?;

        costs[0] = saturated_sum([vlow_cost, vhigh_cost, 1]);
        if self.vertex(low)?.value == ActValue::Internal {
            costs[1] = saturated_sum([vlowlow_cost, vlowhigh_cost, vhigh_cost, 1]);
        }
        if self.vertex(high)?.value == ActValue::Internal {
            costs[2] = saturated_sum([vlow_cost, vhighlow_cost, vhighhigh_cost, 1]);
        }
        if self.vertex(low)?.value == ActValue::Internal
            && self.vertex(high)?.value == ActValue::Internal
        {
            costs[3] = saturated_sum([
                vlowlow_cost,
                vlowhigh_cost,
                vhighlow_cost,
                vhighhigh_cost,
                1,
            ]);
        }

        if options.use_or_patterns {
            self.fill_or_pattern_area_costs(&mut costs, vertex, present_act, options)?;
        }

        let best = minimum_cost_index(&costs[..total_patterns]);
        self.set_cost(
            vertex,
            costs[best],
            Some(best),
            self.vertex(vertex)?.arrival_time,
        )?;
        Ok(costs[best])
    }

    pub fn map_act_delay(
        &mut self,
        vertex: ActVertexId,
        present_act: ActVertexId,
        delay_values: &DelayValues,
        options: &ActMapOptions,
    ) -> ActMapResult<i32> {
        if self.vertex(vertex)?.value != ActValue::Internal {
            self.set_cost(vertex, 0, None, 0.0)?;
            self.vertex_mut(vertex)?.mapped = true;
            return Ok(0);
        }

        if self.vertex(vertex)?.multiple_fo != 0 && vertex != present_act {
            return Ok(0);
        }

        if self.vertex(vertex)?.mapped {
            return Ok(self.vertex(vertex)?.cost);
        }

        self.vertex_mut(vertex)?.mapped = true;
        let (low, high) = self.children(vertex)?;
        if self.is_simple_input(low, high)? {
            let arrival = self.vertex(vertex)?.input_arrival_time;
            self.set_cost(vertex, 0, None, arrival)?;
            return Ok(0);
        }

        let mut costs = [MAX_COST; 12];
        let mut delays = [f64::from(MAX_COST); 12];
        let low_eval = self.child_eval_delay(low, present_act, delay_values, options)?;
        let high_eval = self.child_eval_delay(high, present_act, delay_values, options)?;
        let low_grand = self.grandchild_evals_delay(low, present_act, delay_values, options)?;
        let high_grand = self.grandchild_evals_delay(high, present_act, delay_values, options)?;
        let node_arrival = self.vertex(vertex)?.input_arrival_time;
        let prop_delay = delay_values.delay_for_fanout(self.vertex(vertex)?.multiple_fo + 1);

        costs[0] = saturated_sum([low_eval.cost, high_eval.cost, 1]);
        delays[0] = max3(low_eval.arrival_time, high_eval.arrival_time, node_arrival) + prop_delay;

        if self.vertex(low)?.value == ActValue::Internal {
            costs[1] = saturated_sum([low_grand.low.cost, low_grand.high.cost, high_eval.cost, 1]);
            delays[1] = max3(
                node_arrival,
                self.vertex(low)?.input_arrival_time,
                max3(
                    low_grand.low.arrival_time,
                    low_grand.high.arrival_time,
                    high_eval.arrival_time,
                ),
            ) + prop_delay;
        }

        if self.vertex(high)?.value == ActValue::Internal {
            costs[2] = saturated_sum([low_eval.cost, high_grand.low.cost, high_grand.high.cost, 1]);
            delays[2] = max3(
                node_arrival,
                self.vertex(high)?.input_arrival_time,
                max3(
                    high_grand.low.arrival_time,
                    high_grand.high.arrival_time,
                    low_eval.arrival_time,
                ),
            ) + prop_delay;
        }

        if self.vertex(low)?.value == ActValue::Internal
            && self.vertex(high)?.value == ActValue::Internal
        {
            costs[3] = saturated_sum([
                low_grand.low.cost,
                low_grand.high.cost,
                high_grand.low.cost,
                high_grand.high.cost,
                1,
            ]);
            delays[3] = max3(
                max3(
                    node_arrival,
                    self.vertex(low)?.input_arrival_time,
                    self.vertex(high)?.input_arrival_time,
                ),
                max3(
                    low_grand.low.arrival_time,
                    low_grand.high.arrival_time,
                    high_grand.low.arrival_time,
                ),
                high_grand.high.arrival_time,
            ) + prop_delay;
        }

        self.fill_or_pattern_delay_costs(
            &mut costs,
            &mut delays,
            vertex,
            present_act,
            delay_values,
            options,
        )?;

        let best = minimum_costdelay_index(&costs, &delays, 12, options.delay_weight);
        self.set_cost(vertex, costs[best], Some(best), delays[best])?;
        Ok(costs[best])
    }

    fn put_node_names_from(
        &mut self,
        vertex: ActVertexId,
        names: &[String],
        seen: &mut HashSet<ActVertexId>,
    ) -> ActMapResult<()> {
        if !seen.insert(vertex) {
            return Ok(());
        }

        if self.vertex(vertex)?.value != ActValue::Internal {
            return Ok(());
        }

        let index = self.vertex(vertex)?.index;
        let name = names
            .get(index)
            .ok_or(ActMapError::MissingNodeName { index })?
            .clone();
        self.vertex_mut(vertex)?.name = Some(name);
        let (low, high) = self.children(vertex)?;
        self.put_node_names_from(low, names, seen)?;
        self.put_node_names_from(high, names, seen)
    }

    fn initialize_area_from(
        &mut self,
        vertex: ActVertexId,
        seen: &mut HashSet<ActVertexId>,
        multiple_fo_roots: &mut Vec<ActVertexId>,
    ) -> ActMapResult<()> {
        if !seen.insert(vertex) {
            return Ok(());
        }

        self.reset_vertex(vertex)?;
        self.vertex_mut(vertex)?.mark = true;
        if self.vertex(vertex)?.value != ActValue::Internal {
            return Ok(());
        }

        let (low, high) = self.children(vertex)?;
        self.visit_or_mark_multiple_fo(low, seen, multiple_fo_roots, true)?;
        self.visit_or_mark_multiple_fo(high, seen, multiple_fo_roots, true)
    }

    fn initialize_delay_from(
        &mut self,
        vertex: ActVertexId,
        seen: &mut HashSet<ActVertexId>,
    ) -> ActMapResult<()> {
        if !seen.insert(vertex) {
            return Ok(());
        }

        self.reset_vertex(vertex)?;
        self.vertex_mut(vertex)?.mark = true;
        if self.vertex(vertex)?.value != ActValue::Internal {
            return Ok(());
        }

        let (low, high) = self.children(vertex)?;
        self.visit_or_mark_multiple_fo(low, seen, &mut Vec::new(), false)?;
        self.visit_or_mark_multiple_fo(high, seen, &mut Vec::new(), false)
    }

    fn visit_or_mark_multiple_fo(
        &mut self,
        child: ActVertexId,
        seen: &mut HashSet<ActVertexId>,
        multiple_fo_roots: &mut Vec<ActVertexId>,
        collect_immediately: bool,
    ) -> ActMapResult<()> {
        if seen.contains(&child) {
            if collect_immediately && self.vertex(child)?.multiple_fo == 0 {
                multiple_fo_roots.push(child);
            }
            let child_ref = self.vertex_mut(child)?;
            child_ref.multiple_fo += 1;
            child_ref.multiple_fo_for_mapping += 1;
            return Ok(());
        }

        if collect_immediately {
            self.initialize_area_from(child, seen, multiple_fo_roots)
        } else {
            self.initialize_delay_from(child, seen)
        }
    }

    fn collect_delay_multiple_fo(
        &mut self,
        vertex: ActVertexId,
        seen: &mut HashSet<ActVertexId>,
        collected: &mut HashSet<ActVertexId>,
        multiple_fo_roots: &mut Vec<ActVertexId>,
    ) -> ActMapResult<()> {
        if !seen.insert(vertex) {
            return Ok(());
        }

        if self.vertex(vertex)?.value != ActValue::Internal {
            return Ok(());
        }

        if self.vertex(vertex)?.multiple_fo != 0 && collected.insert(vertex) {
            multiple_fo_roots.push(vertex);
        }

        let (low, high) = self.children(vertex)?;
        self.collect_delay_multiple_fo(low, seen, collected, multiple_fo_roots)?;
        self.collect_delay_multiple_fo(high, seen, collected, multiple_fo_roots)
    }

    fn clear_marks_from(
        &mut self,
        vertex: ActVertexId,
        seen: &mut HashSet<ActVertexId>,
    ) -> ActMapResult<()> {
        if !seen.insert(vertex) {
            return Ok(());
        }

        self.vertex_mut(vertex)?.mark = false;
        if self.vertex(vertex)?.value == ActValue::Internal {
            let (low, high) = self.children(vertex)?;
            self.clear_marks_from(low, seen)?;
            self.clear_marks_from(high, seen)?;
        }
        Ok(())
    }

    fn reset_vertex(&mut self, vertex: ActVertexId) -> ActMapResult<()> {
        let vertex_ref = self.vertex_mut(vertex)?;
        vertex_ref.pattern_num = None;
        vertex_ref.cost = 0;
        vertex_ref.arrival_time = 0.0;
        vertex_ref.mapped = false;
        vertex_ref.multiple_fo = 0;
        vertex_ref.multiple_fo_for_mapping = 0;
        Ok(())
    }

    fn child_cost_area(
        &mut self,
        child: ActVertexId,
        present_act: ActVertexId,
        options: &ActMapOptions,
    ) -> ActMapResult<i32> {
        if self.vertex(child)?.multiple_fo != 0 {
            return Ok(0);
        }

        self.map_act(child, present_act, options)
    }

    fn grandchild_costs_area(
        &mut self,
        child: ActVertexId,
        present_act: ActVertexId,
        options: &ActMapOptions,
    ) -> ActMapResult<(i32, i32)> {
        if self.vertex(child)?.value != ActValue::Internal {
            return Ok((MAX_COST, MAX_COST));
        }

        if self.vertex(child)?.multiple_fo != 0 {
            return Ok((MAX_COST, MAX_COST));
        }

        let (low, high) = self.children(child)?;
        let low_cost = self.child_cost_area(low, present_act, options)?;
        let high_cost = self.child_cost_area(high, present_act, options)?;
        Ok((low_cost, high_cost))
    }

    fn child_eval_delay(
        &mut self,
        child: ActVertexId,
        present_act: ActVertexId,
        delay_values: &DelayValues,
        options: &ActMapOptions,
    ) -> ActMapResult<DelayEval> {
        if self.vertex(child)?.multiple_fo == 0 {
            self.map_act_delay(child, present_act, delay_values, options)?;
        }

        let vertex = self.vertex(child)?;
        Ok(DelayEval {
            cost: if vertex.multiple_fo == 0 {
                vertex.cost
            } else {
                0
            },
            arrival_time: vertex.arrival_time,
        })
    }

    fn grandchild_evals_delay(
        &mut self,
        child: ActVertexId,
        present_act: ActVertexId,
        delay_values: &DelayValues,
        options: &ActMapOptions,
    ) -> ActMapResult<DelayPair> {
        if self.vertex(child)?.value != ActValue::Internal || self.vertex(child)?.multiple_fo != 0 {
            return Ok(DelayPair::blocked());
        }

        let (low, high) = self.children(child)?;
        Ok(DelayPair {
            low: self.child_eval_delay(low, present_act, delay_values, options)?,
            high: self.child_eval_delay(high, present_act, delay_values, options)?,
        })
    }

    fn fill_or_pattern_area_costs(
        &mut self,
        costs: &mut [i32; 12],
        vertex: ActVertexId,
        present_act: ActVertexId,
        options: &ActMapOptions,
    ) -> ActMapResult<()> {
        let (low, high) = self.children(vertex)?;
        if self.is_or_pattern(vertex)? {
            let (low_low, _) = self.children(low)?;
            if self.vertex(low_low)?.multiple_fo == 0
                && self.vertex(low_low)?.value == ActValue::Internal
            {
                let (left, right) = self.children(low_low)?;
                costs[4] = saturated_sum([
                    self.map_act(left, present_act, options)?,
                    self.map_act(right, present_act, options)?,
                    1,
                ]);
            }
        }

        if self.vertex(high)?.value == ActValue::Internal
            && self.is_or_pattern(high)?
            && self.vertex(high)?.multiple_fo == 0
        {
            let (high_low, high_high) = self.children(high)?;
            let (high_low_low, _) = self.children(high_low)?;
            if self.vertex(low)?.value == ActValue::Zero {
                costs[5] = saturated_sum([
                    self.map_act(high_high, present_act, options)?,
                    self.map_act(high_low_low, present_act, options)?,
                    1,
                ]);
            } else if self.vertex(low)?.value == ActValue::One
                && self.vertex(high_high)?.value == ActValue::Zero
                && self.vertex(high_low_low)?.value == ActValue::One
            {
                costs[6] = 1;
            }
        }

        if self.vertex(low)?.value == ActValue::Internal
            && self.is_or_pattern(low)?
            && self.vertex(low)?.multiple_fo == 0
            && self.vertex(high)?.value == ActValue::Zero
        {
            let (low_low, low_high) = self.children(low)?;
            let (low_low_low, _) = self.children(low_low)?;
            costs[7] = saturated_sum([
                self.map_act(low_high, present_act, options)?,
                self.map_act(low_low_low, present_act, options)?,
                1,
            ]);
        }
        Ok(())
    }

    fn fill_or_pattern_delay_costs(
        &mut self,
        costs: &mut [i32; 12],
        delays: &mut [f64; 12],
        vertex: ActVertexId,
        present_act: ActVertexId,
        delay_values: &DelayValues,
        options: &ActMapOptions,
    ) -> ActMapResult<()> {
        let (low, high) = self.children(vertex)?;
        let prop_delay = delay_values.delay_for_fanout(self.vertex(vertex)?.multiple_fo + 1);
        let node_arrival = self.vertex(vertex)?.input_arrival_time;

        if self.is_or_pattern(vertex)? {
            let (low_low, _) = self.children(low)?;
            if self.vertex(low_low)?.multiple_fo == 0
                && self.vertex(low_low)?.value == ActValue::Internal
            {
                let (left, right) = self.children(low_low)?;
                let left_eval = self.child_eval_delay(left, present_act, delay_values, options)?;
                let right_eval =
                    self.child_eval_delay(right, present_act, delay_values, options)?;
                costs[4] = saturated_sum([left_eval.cost, right_eval.cost, 1]);
                delays[4] = max3(
                    max3(
                        node_arrival,
                        self.vertex(low)?.input_arrival_time,
                        self.vertex(low_low)?.input_arrival_time,
                    ),
                    left_eval.arrival_time,
                    right_eval.arrival_time,
                ) + prop_delay;
            }
        }

        if self.vertex(high)?.value == ActValue::Internal
            && self.is_or_pattern(high)?
            && self.vertex(high)?.multiple_fo == 0
        {
            let (high_low, high_high) = self.children(high)?;
            let (high_low_low, _) = self.children(high_low)?;
            if self.vertex(low)?.value == ActValue::Zero {
                let high_high_eval =
                    self.child_eval_delay(high_high, present_act, delay_values, options)?;
                let high_low_low_eval =
                    self.child_eval_delay(high_low_low, present_act, delay_values, options)?;
                costs[5] = saturated_sum([high_high_eval.cost, high_low_low_eval.cost, 1]);
                delays[5] = max3(
                    max3(
                        node_arrival,
                        self.vertex(high)?.input_arrival_time,
                        self.vertex(high_low)?.input_arrival_time,
                    ),
                    high_high_eval.arrival_time,
                    high_low_low_eval.arrival_time,
                ) + prop_delay;
            } else if self.vertex(low)?.value == ActValue::One
                && self.vertex(high_high)?.value == ActValue::Zero
                && self.vertex(high_low_low)?.value == ActValue::One
            {
                costs[6] = 1;
                delays[6] = max3(
                    node_arrival,
                    self.vertex(high)?.input_arrival_time,
                    self.vertex(high_low)?.input_arrival_time,
                ) + prop_delay;
            }
        }

        if self.vertex(low)?.value == ActValue::Internal
            && self.is_or_pattern(low)?
            && self.vertex(low)?.multiple_fo == 0
            && self.vertex(high)?.value == ActValue::Zero
        {
            let (low_low, low_high) = self.children(low)?;
            let (low_low_low, _) = self.children(low_low)?;
            let low_high_eval =
                self.child_eval_delay(low_high, present_act, delay_values, options)?;
            let low_low_low_eval =
                self.child_eval_delay(low_low_low, present_act, delay_values, options)?;
            costs[7] = saturated_sum([low_high_eval.cost, low_low_low_eval.cost, 1]);
            delays[7] = max3(
                max3(
                    node_arrival,
                    self.vertex(low)?.input_arrival_time,
                    self.vertex(low_low)?.input_arrival_time,
                ),
                low_high_eval.arrival_time,
                low_low_low_eval.arrival_time,
            ) + prop_delay;
        }
        Ok(())
    }

    fn is_or_pattern(&self, root: ActVertexId) -> ActMapResult<bool> {
        if self.vertex(root)?.value != ActValue::Internal {
            return Ok(false);
        }

        let (low, high) = self.children(root)?;
        if self.vertex(low)?.multiple_fo != 0 || self.vertex(low)?.value != ActValue::Internal {
            return Ok(false);
        }

        let (_, low_high) = self.children(low)?;
        Ok(high == low_high)
    }

    fn is_simple_input(&self, low: ActVertexId, high: ActVertexId) -> ActMapResult<bool> {
        Ok(self.vertex(low)?.value == ActValue::Zero && self.vertex(high)?.value == ActValue::One)
    }

    fn children(&self, vertex: ActVertexId) -> ActMapResult<(ActVertexId, ActVertexId)> {
        let vertex_ref = self.vertex(vertex)?;
        let low = vertex_ref.low.ok_or(ActMapError::MissingChild {
            vertex,
            child: "low",
        })?;
        let high = vertex_ref.high.ok_or(ActMapError::MissingChild {
            vertex,
            child: "high",
        })?;
        Ok((low, high))
    }

    fn set_cost(
        &mut self,
        vertex: ActVertexId,
        cost: i32,
        pattern_num: Option<usize>,
        arrival_time: f64,
    ) -> ActMapResult<()> {
        let vertex_ref = self.vertex_mut(vertex)?;
        vertex_ref.cost = cost;
        vertex_ref.pattern_num = pattern_num;
        vertex_ref.arrival_time = arrival_time;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayValues {
    values: Vec<f64>,
}

impl DelayValues {
    pub fn new(values: impl Into<Vec<f64>>) -> Self {
        Self {
            values: values.into(),
        }
    }

    pub fn delay_for_fanout(&self, fanout_count: usize) -> f64 {
        if self.values.is_empty() || fanout_count == 0 {
            return 0.0;
        }

        let index = fanout_count.min(self.values.len() - 1);
        if fanout_count <= self.values.len() - 1 || self.values.len() == 1 {
            return self.values[index];
        }

        let last_index = self.values.len() - 1;
        let previous = self.values[last_index - 1];
        let last = self.values[last_index];
        last + (fanout_count - last_index) as f64 * (last - previous)
    }

    pub fn node_fanout_correction(&self, assumed_fanout: usize, actual_fanout: usize) -> f64 {
        if assumed_fanout == 0 || actual_fanout == 0 || assumed_fanout == actual_fanout {
            return 0.0;
        }

        self.delay_for_fanout(actual_fanout) - self.delay_for_fanout(assumed_fanout)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DelayEval {
    cost: i32,
    arrival_time: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DelayPair {
    low: DelayEval,
    high: DelayEval,
}

impl DelayPair {
    fn blocked() -> Self {
        Self {
            low: DelayEval {
                cost: MAX_COST,
                arrival_time: f64::from(MAX_COST),
            },
            high: DelayEval {
                cost: MAX_COST,
                arrival_time: f64::from(MAX_COST),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActMapError {
    UnknownVertex(ActVertexId),
    MissingChild {
        vertex: ActVertexId,
        child: &'static str,
    },
    MissingNodeName {
        index: usize,
    },
    MissingAct {
        node: String,
    },
    MissingCost {
        node: String,
    },
    InvalidHeuristic(i32),
    MissingNativePorts {
        operation: &'static str,
    },
}

impl fmt::Display for ActMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVertex(vertex) => write!(f, "unknown ACT vertex {}", vertex.0),
            Self::MissingChild { vertex, child } => {
                write!(f, "ACT vertex {} is missing {child} child", vertex.0)
            }
            Self::MissingNodeName { index } => {
                write!(f, "missing node name for ACT vertex index {index}")
            }
            Self::MissingAct { node } => write!(f, "node {node} has no ACT root"),
            Self::MissingCost { node } => write!(f, "node {node} has no mapped cost"),
            Self::InvalidHeuristic(heuristic) => {
                write!(f, "ACT mapping heuristic {heuristic} is not supported")
            }
            Self::MissingNativePorts { operation } => write!(
                f,
                "{operation} requires native Rust ports for SIS dependencies"
            ),
        }
    }
}

impl Error for ActMapError {}

pub type ActMapResult<T> = Result<T, ActMapError>;

pub fn act_evaluate_map_cost(
    node: &ActMapNode,
    graph: &mut ActGraph,
    init_params: &ActInitParams,
    delay_values: Option<&DelayValues>,
    options: &ActMapOptions,
) -> ActMapResult<CostStruct> {
    if node.function == NodeFunction::Zero || node.function == NodeFunction::One {
        return Ok(CostStruct::constant(node.name.clone()));
    }

    match init_params.heuristic_num {
        1 | 3 | 4 => {
            let root = node.act_root.ok_or_else(|| ActMapError::MissingAct {
                node: node.name.clone(),
            })?;
            if init_params.mode == MapMode::Area {
                graph.make_tree_and_map(node.name.clone(), root, options)
            } else {
                let delay_values = delay_values.ok_or(ActMapError::MissingNativePorts {
                    operation: "delay table loading",
                })?;
                graph.make_tree_and_map_delay(node.name.clone(), root, delay_values, options)
            }
        }
        2 => Err(missing_native_ports("unordered ACT construction")),
        heuristic => Err(ActMapError::InvalidHeuristic(heuristic)),
    }
}

pub fn act_map_network_owned(
    network: &ActMapNetwork,
    graph: &mut ActGraph,
    init_params: &ActInitParams,
    delay_values: Option<&DelayValues>,
    options: &ActMapOptions,
) -> ActMapResult<HashMap<String, CostStruct>> {
    if init_params.disjoint_decomp {
        return Err(missing_native_ports("disjoint decomposition"));
    }

    let mut costs = HashMap::new();
    for node in network.nodes() {
        if node.kind == NodeKind::PrimaryInput || node.kind == NodeKind::PrimaryOutput {
            continue;
        }

        let cost = act_evaluate_map_cost(node, graph, init_params, delay_values, options)?;
        costs.insert(node.name.clone(), cost);
    }
    Ok(costs)
}

pub fn iterative_improvement_blocked<Network>(
    _network: &mut Network,
    _init_params: &ActInitParams,
) -> ActMapResult<()> {
    Err(missing_native_ports("iterative ACT network improvement"))
}

pub fn act_network_remap_blocked<Network>(
    _network: &mut Network,
    _init_params: &ActInitParams,
) -> ActMapResult<()> {
    Err(missing_native_ports("ACT network remapping"))
}

pub fn act_break_network_blocked<Network>(
    _network: &mut Network,
    _init_params: &ActInitParams,
) -> ActMapResult<()> {
    Err(missing_native_ports("ACT network breaking"))
}

pub fn act_delay_iterative_improvement_blocked<Network>(
    _network: &mut Network,
    _init_params: &ActInitParams,
) -> ActMapResult<()> {
    Err(missing_native_ports("ACT delay iterative improvement"))
}

pub fn minimum_cost_index(costs: &[i32]) -> usize {
    let mut min_index = 0;
    for index in 1..costs.len() {
        if costs[index] < costs[min_index] {
            min_index = index;
        }
    }
    min_index
}

pub fn minimum_costdelay_index(
    costs: &[i32],
    delays: &[f64],
    num_entries: usize,
    delay_weight: f64,
) -> usize {
    let entries = num_entries.min(costs.len()).min(delays.len());
    let mut min_index = 0;
    for index in 1..entries {
        let candidate = weighted_cost_delay(costs[index], delays[index], delay_weight);
        let best = weighted_cost_delay(costs[min_index], delays[min_index], delay_weight);
        if candidate < best {
            min_index = index;
        }
    }
    min_index
}

pub fn weighted_cost_delay(cost: i32, delay: f64, delay_weight: f64) -> f64 {
    let mode = delay_weight.clamp(0.0, 1.0);
    (1.0 - mode) * f64::from(cost) + mode * delay
}

fn saturated_sum(values: impl IntoIterator<Item = i32>) -> i32 {
    values.into_iter().fold(0, saturated_add)
}

fn saturated_add(left: i32, right: i32) -> i32 {
    left.saturating_add(right).min(MAX_COST)
}

fn max3(left: f64, middle: f64, right: f64) -> f64 {
    left.max(middle).max(right)
}

fn missing_native_ports(operation: &'static str) -> ActMapError {
    ActMapError::MissingNativePorts { operation }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input_vertex(graph: &mut ActGraph, name: &str) -> ActVertexId {
        let zero = graph.add_vertex(ActVertex::terminal(false));
        let one = graph.add_vertex(ActVertex::terminal(true));
        graph.add_vertex(ActVertex::named_input(name, zero, one))
    }

    #[test]
    fn simple_input_maps_to_zero_area_cost() {
        let mut graph = ActGraph::new();
        let x = input_vertex(&mut graph, "x");

        let cost = graph
            .make_tree_and_map("x", x, &ActMapOptions::default())
            .unwrap();

        assert_eq!(cost.cost, 0);
        assert_eq!(graph.vertex(x).unwrap().pattern_num, None);
    }

    #[test]
    fn basic_internal_vertex_uses_one_mux_pattern() {
        let mut graph = ActGraph::new();
        let x = input_vertex(&mut graph, "x");
        let y = input_vertex(&mut graph, "y");
        let root = graph.add_vertex(ActVertex::internal(x, y));

        let cost = graph
            .make_tree_and_map("f", root, &ActMapOptions::default())
            .unwrap();

        assert_eq!(cost.cost, 1);
        assert_eq!(graph.vertex(root).unwrap().pattern_num, Some(0));
    }

    #[test]
    fn multiple_fanout_roots_are_mapped_bottom_up_and_counted_once() {
        let mut graph = ActGraph::new();
        let x = input_vertex(&mut graph, "x");
        let y = input_vertex(&mut graph, "y");
        let shared = graph.add_vertex(ActVertex::internal(x, y));
        let root = graph.add_vertex(ActVertex::internal(shared, shared));

        let roots = graph.initialize_act_area(root).unwrap();
        assert_eq!(roots, vec![root, shared]);
        assert_eq!(graph.vertex(shared).unwrap().multiple_fo, 1);

        let cost = graph
            .make_tree_and_map("f", root, &ActMapOptions::default())
            .unwrap();

        assert_eq!(cost.cost, 2);
        assert!(!graph.vertex(root).unwrap().mark);
    }

    #[test]
    fn or_pattern_keeps_earliest_equal_cost_pattern() {
        let mut graph = ActGraph::new();
        let zero = graph.add_vertex(ActVertex::terminal(false));
        let one = graph.add_vertex(ActVertex::terminal(true));
        let x = graph.add_vertex(ActVertex::named_input("x", zero, one));
        let y = graph.add_vertex(ActVertex::named_input("y", zero, one));
        let low = graph.add_vertex(ActVertex::internal(x, y));
        let root = graph.add_vertex(ActVertex::internal(low, y));

        let cost = graph
            .make_tree_and_map("f", root, &ActMapOptions::default())
            .unwrap();

        assert_eq!(cost.cost, 1);
        assert_eq!(graph.vertex(root).unwrap().pattern_num, Some(1));
    }

    #[test]
    fn disabling_or_patterns_limits_selection_to_first_four_patterns() {
        let mut graph = ActGraph::new();
        let zero = graph.add_vertex(ActVertex::terminal(false));
        let one = graph.add_vertex(ActVertex::terminal(true));
        let x = graph.add_vertex(ActVertex::named_input("x", zero, one));
        let y = graph.add_vertex(ActVertex::named_input("y", zero, one));
        let low = graph.add_vertex(ActVertex::internal(x, y));
        let root = graph.add_vertex(ActVertex::internal(low, y));

        let cost = graph
            .make_tree_and_map(
                "f",
                root,
                &ActMapOptions {
                    use_or_patterns: false,
                    ..ActMapOptions::default()
                },
            )
            .unwrap();

        assert_eq!(cost.cost, 1);
        assert_eq!(graph.vertex(root).unwrap().pattern_num, Some(1));
    }

    #[test]
    fn delay_mapping_prefers_lower_weighted_delay_pattern() {
        let mut graph = ActGraph::new();
        let zero = graph.add_vertex(ActVertex::terminal(false));
        let one = graph.add_vertex(ActVertex::terminal(true));
        let mut x = ActVertex::named_input("x", zero, one);
        x.input_arrival_time = 4.0;
        let x = graph.add_vertex(x);
        let mut y = ActVertex::named_input("y", zero, one);
        y.input_arrival_time = 1.0;
        let y = graph.add_vertex(y);
        let low = graph.add_vertex(ActVertex::internal(x, y));
        let root = graph.add_vertex(ActVertex::internal(low, y));
        let delays = DelayValues::new(vec![0.0, 1.0, 2.0]);

        let cost = graph
            .make_tree_and_map_delay(
                "f",
                root,
                &delays,
                &ActMapOptions {
                    delay_weight: 1.0,
                    ..ActMapOptions::default()
                },
            )
            .unwrap();

        assert_eq!(cost.cost, 1);
        assert_eq!(graph.vertex(root).unwrap().pattern_num, Some(1));
        assert_eq!(cost.arrival_time, 5.0);
    }

    #[test]
    fn delay_values_use_linear_extrapolation_and_node_fanout_correction() {
        let delays = DelayValues::new(vec![0.0, 1.0, 1.5]);

        assert_eq!(delays.delay_for_fanout(1), 1.0);
        assert_eq!(delays.delay_for_fanout(4), 2.5);
        assert_eq!(delays.node_fanout_correction(1, 4), 1.5);
    }

    #[test]
    fn network_total_cost_ignores_primary_nodes() {
        let mut network = ActMapNetwork::new();
        network.add_node(ActMapNode {
            name: "pi".to_string(),
            kind: NodeKind::PrimaryInput,
            function: NodeFunction::Other,
            act_root: None,
            fanout_count: 1,
        });
        network.add_node(ActMapNode::internal("n1", NodeFunction::Other));
        network.add_node(ActMapNode::internal("n2", NodeFunction::One));
        let costs = HashMap::from([
            (
                "n1".to_string(),
                CostStruct {
                    node_name: "n1".to_string(),
                    cost: 3,
                    arrival_time: 0.0,
                    cost_and_arrival_time: -1.0,
                    act_root: None,
                },
            ),
            ("n2".to_string(), CostStruct::constant("n2")),
        ]);

        assert_eq!(network.total_internal_cost(&costs).unwrap(), 3);
    }

    #[test]
    fn no_legacy_c_abi_or_metadata_tokens_are_present_in_this_port() {
        let source = include_str!("act_map.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
