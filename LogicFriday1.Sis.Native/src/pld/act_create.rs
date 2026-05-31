//! Native Rust model for `LogicSynthesis/sis/pld/act_create.c`.
//!
//! The original SIS code creates ACT decision graphs from `node_t` objects,
//! optionally searching fanin order permutations. This port keeps the
//! constructible behavior on owned Boolean expressions and reports direct
//! SIS/factor-tree integration as missing native prerequisite ports.

use std::error::Error;
use std::fmt;

pub const DEFAULT_MAX_OPTIMAL: usize = 8;
pub const HICOST: usize = usize::MAX / 4;

#[derive(Clone, Debug, PartialEq)]
pub enum ActCreateError {
    MissingNativePorts { operation: &'static str },
    MissingOrderVariable { node: String, variable: String },
    InvalidMode(f64),
    EmptyOptimalOrder { node: String },
}

impl fmt::Display for ActCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNativePorts { operation } => {
                write!(f, "{operation} requires native SIS prerequisite ports")
            }
            Self::MissingOrderVariable { node, variable } => {
                write!(
                    f,
                    "node {node} depends on {variable}, but the ACT order omits it"
                )
            }
            Self::InvalidMode(mode) => write!(f, "ACT ordering mode {mode} is outside 0.0..=1.0"),
            Self::EmptyOptimalOrder { node } => {
                write!(f, "optimal ACT ordering for node {node} produced no order")
            }
        }
    }
}

impl Error for ActCreateError {}

pub type ActCreateResult<T> = Result<T, ActCreateError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActOrderStyle {
    Fanin,
    Optimal,
    Random,
    Heuristic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActOrderMode {
    Area,
    AreaDelayBlend(f64),
}

impl ActOrderMode {
    fn score(self, cost: ActCost) -> ActCreateResult<f64> {
        match self {
            Self::Area => Ok(cost.area as f64),
            Self::AreaDelayBlend(mode) if (0.0..=1.0).contains(&mode) => {
                Ok((1.0 - mode) * cost.area as f64 + mode * cost.arrival_time)
            }
            Self::AreaDelayBlend(mode) => Err(ActCreateError::InvalidMode(mode)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActCost {
    pub area: usize,
    pub arrival_time: f64,
}

impl ActCost {
    pub fn area(area: usize) -> Self {
        Self {
            area,
            arrival_time: area as f64,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActBuildOptions {
    pub order_style: ActOrderStyle,
    pub mode: ActOrderMode,
    pub max_optimal: usize,
    pub random_seed: u64,
}

impl Default for ActBuildOptions {
    fn default() -> Self {
        Self {
            order_style: ActOrderStyle::Fanin,
            mode: ActOrderMode::Area,
            max_optimal: DEFAULT_MAX_OPTIMAL,
            random_seed: 0x9e37_79b9_7f4a_7c15,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActBuildDiagnostic {
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Act {
    pub node_name: String,
    pub root: ActVertex,
    pub node_list: Vec<String>,
}

impl Act {
    pub fn evaluate(&self, inputs: impl IntoIterator<Item = (impl AsRef<str>, bool)>) -> bool {
        let values = inputs
            .into_iter()
            .map(|(name, value)| (name.as_ref().to_owned(), value))
            .collect::<Vec<_>>();
        self.root.evaluate_with_pairs(&values)
    }

    pub fn internal_vertex_count(&self) -> usize {
        self.root.internal_vertex_count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActEntry {
    pub act: Act,
    pub order_style: ActOrderStyle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActBuildReport {
    pub entry: ActEntry,
    pub diagnostics: Vec<ActBuildDiagnostic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActValue {
    Zero,
    One,
    NoValue,
}

impl ActValue {
    fn from_bool(value: bool) -> Self {
        if value { Self::One } else { Self::Zero }
    }

    fn as_bool(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            Self::NoValue => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActVertex {
    pub id: usize,
    pub mark: bool,
    pub index: usize,
    pub index_size: usize,
    pub value: ActValue,
    pub node: Option<String>,
    pub name: Option<String>,
    pub multiple_fo: bool,
    pub cost: usize,
    pub mapped: bool,
    pub low: Option<Box<ActVertex>>,
    pub high: Option<Box<ActVertex>>,
}

impl ActVertex {
    pub fn terminal(value: bool, index_size: usize) -> Self {
        Self {
            id: 0,
            mark: false,
            index: index_size,
            index_size,
            value: ActValue::from_bool(value),
            node: None,
            name: None,
            multiple_fo: false,
            cost: 0,
            mapped: false,
            low: None,
            high: None,
        }
    }

    pub fn branch(
        index: usize,
        index_size: usize,
        variable: impl Into<String>,
        low: ActVertex,
        high: ActVertex,
    ) -> Self {
        Self {
            id: 0,
            mark: false,
            index,
            index_size,
            value: ActValue::NoValue,
            node: Some(variable.into()),
            name: None,
            multiple_fo: false,
            cost: 0,
            mapped: false,
            low: Some(Box::new(low)),
            high: Some(Box::new(high)),
        }
    }

    pub fn evaluate_with_pairs(&self, inputs: &[(String, bool)]) -> bool {
        if let Some(value) = self.value.as_bool() {
            return value;
        }

        let variable = self.node.as_deref().unwrap_or_default();
        let value = inputs
            .iter()
            .find_map(|(name, value)| (name == variable).then_some(*value))
            .unwrap_or(false);
        let child = if value { &self.high } else { &self.low };
        child
            .as_deref()
            .map(|vertex| vertex.evaluate_with_pairs(inputs))
            .unwrap_or(false)
    }

    pub fn internal_vertex_count(&self) -> usize {
        if self.value != ActValue::NoValue {
            return 0;
        }

        1 + self
            .low
            .as_deref()
            .map(Self::internal_vertex_count)
            .unwrap_or(0)
            + self
                .high
                .as_deref()
                .map(Self::internal_vertex_count)
                .unwrap_or(0)
    }

    fn structurally_matches(&self, other: &Self) -> bool {
        self.index == other.index
            && self.value == other.value
            && self.node == other.node
            && match (&self.low, &other.low) {
                (Some(left), Some(right)) => left.structurally_matches(right),
                (None, None) => true,
                _ => false,
            }
            && match (&self.high, &other.high) {
                (Some(left), Some(right)) => left.structurally_matches(right),
                (None, None) => true,
                _ => false,
            }
    }

    fn assign_ids(&mut self, next_id: &mut usize) {
        self.id = *next_id;
        *next_id += 1;
        if let Some(low) = self.low.as_deref_mut() {
            low.assign_ids(next_id);
        }
        if let Some(high) = self.high.as_deref_mut() {
            high.assign_ids(next_id);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BooleanNode {
    pub name: String,
    pub function: BooleanExpr,
}

impl BooleanNode {
    pub fn new(name: impl Into<String>, function: BooleanExpr) -> Self {
        Self {
            name: name.into(),
            function,
        }
    }

    pub fn fanin_order(&self) -> Vec<String> {
        let mut variables = Vec::new();
        self.function.collect_variables(&mut variables);
        variables
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BooleanExpr {
    Const(bool),
    Var(String),
    Not(Box<BooleanExpr>),
    And(Vec<BooleanExpr>),
    Or(Vec<BooleanExpr>),
}

impl BooleanExpr {
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var(name.into())
    }

    pub fn and(items: impl IntoIterator<Item = BooleanExpr>) -> Self {
        Self::And(items.into_iter().collect()).simplified()
    }

    pub fn or(items: impl IntoIterator<Item = BooleanExpr>) -> Self {
        Self::Or(items.into_iter().collect()).simplified()
    }

    pub fn inv(item: BooleanExpr) -> Self {
        Self::Not(Box::new(item)).simplified()
    }

    pub fn cofactor(&self, variable: &str, value: bool) -> Self {
        match self {
            Self::Const(value) => Self::Const(*value),
            Self::Var(name) if name == variable => Self::Const(value),
            Self::Var(name) => Self::Var(name.clone()),
            Self::Not(expr) => Self::inv(expr.cofactor(variable, value)),
            Self::And(items) => Self::and(items.iter().map(|item| item.cofactor(variable, value))),
            Self::Or(items) => Self::or(items.iter().map(|item| item.cofactor(variable, value))),
        }
    }

    pub fn evaluate(&self, inputs: impl IntoIterator<Item = (impl AsRef<str>, bool)>) -> bool {
        let values = inputs
            .into_iter()
            .map(|(name, value)| (name.as_ref().to_owned(), value))
            .collect::<Vec<_>>();
        self.evaluate_with_pairs(&values)
    }

    fn simplified(self) -> Self {
        match self {
            Self::Not(expr) => match expr.simplified() {
                Self::Const(value) => Self::Const(!value),
                Self::Not(inner) => *inner,
                item => Self::Not(Box::new(item)),
            },
            Self::And(items) => {
                let mut simplified = Vec::new();
                for item in items {
                    match item.simplified() {
                        Self::Const(false) => return Self::Const(false),
                        Self::Const(true) => {}
                        Self::And(nested) => simplified.extend(nested),
                        item => simplified.push(item),
                    }
                }
                match simplified.len() {
                    0 => Self::Const(true),
                    1 => simplified.pop().unwrap_or(Self::Const(true)),
                    _ => Self::And(simplified),
                }
            }
            Self::Or(items) => {
                let mut simplified = Vec::new();
                for item in items {
                    match item.simplified() {
                        Self::Const(true) => return Self::Const(true),
                        Self::Const(false) => {}
                        Self::Or(nested) => simplified.extend(nested),
                        item => simplified.push(item),
                    }
                }
                match simplified.len() {
                    0 => Self::Const(false),
                    1 => simplified.pop().unwrap_or(Self::Const(false)),
                    _ => Self::Or(simplified),
                }
            }
            item => item,
        }
    }

    fn evaluate_with_pairs(&self, inputs: &[(String, bool)]) -> bool {
        match self {
            Self::Const(value) => *value,
            Self::Var(name) => inputs
                .iter()
                .find_map(|(input, value)| (input == name).then_some(*value))
                .unwrap_or(false),
            Self::Not(expr) => !expr.evaluate_with_pairs(inputs),
            Self::And(items) => items.iter().all(|item| item.evaluate_with_pairs(inputs)),
            Self::Or(items) => items.iter().any(|item| item.evaluate_with_pairs(inputs)),
        }
    }

    fn collect_variables(&self, variables: &mut Vec<String>) {
        match self {
            Self::Const(_) => {}
            Self::Var(name) => {
                if !variables.contains(name) {
                    variables.push(name.clone());
                }
            }
            Self::Not(expr) => expr.collect_variables(variables),
            Self::And(items) | Self::Or(items) => {
                for item in items {
                    item.collect_variables(variables);
                }
            }
        }
    }

    fn contains_variable(&self, variable: &str) -> bool {
        match self {
            Self::Const(_) => false,
            Self::Var(name) => name == variable,
            Self::Not(expr) => expr.contains_variable(variable),
            Self::And(items) | Self::Or(items) => {
                items.iter().any(|item| item.contains_variable(variable))
            }
        }
    }
}

pub fn create_local_act(
    node: &BooleanNode,
    order: Option<&[String]>,
    options: &ActBuildOptions,
) -> ActCreateResult<ActBuildReport> {
    let mut diagnostics = Vec::new();
    let mut node_list = order
        .map(|order| order.to_vec())
        .unwrap_or_else(|| node.fanin_order());

    let requested_style = options.order_style;
    let effective_style = match requested_style {
        ActOrderStyle::Optimal => {
            if node_list.len() > options.max_optimal {
                diagnostics.push(ActBuildDiagnostic {
                    message: format!(
                        "optimal ordering too expensive for node {}, fanin ordering chosen",
                        node.name
                    ),
                });
                ActOrderStyle::Fanin
            } else {
                let best = optimal_order_by(node, &node_list, options.mode, |act| {
                    ActCost::area(act.internal_vertex_count())
                })?;
                node_list = best;
                ActOrderStyle::Optimal
            }
        }
        ActOrderStyle::Random => {
            shuffle_order(&mut node_list, options.random_seed);
            ActOrderStyle::Random
        }
        ActOrderStyle::Fanin | ActOrderStyle::Heuristic => ActOrderStyle::Fanin,
    };

    let mut root = act_create_step(&node.name, &node.function, 0, &node_list)?;
    root = reduce_vertex(root);
    let mut next_id = 0;
    root.assign_ids(&mut next_id);

    Ok(ActBuildReport {
        entry: ActEntry {
            act: Act {
                node_name: node.name.clone(),
                root,
                node_list,
            },
            order_style: effective_style,
        },
        diagnostics,
    })
}

pub fn optimal_order_by<F>(
    node: &BooleanNode,
    order: &[String],
    mode: ActOrderMode,
    mut cost_fn: F,
) -> ActCreateResult<Vec<String>>
where
    F: FnMut(&Act) -> ActCost,
{
    mode.score(ActCost::area(0))?;
    if order.is_empty() {
        return match node.function.clone().simplified() {
            BooleanExpr::Const(_) => Ok(Vec::new()),
            _ => Err(ActCreateError::EmptyOptimalOrder {
                node: node.name.clone(),
            }),
        };
    }

    let mut working = order.to_vec();
    let mut best_order = Vec::new();
    let mut best_score = f64::INFINITY;
    heap_permute(order.len(), &mut working, &mut |candidate| {
        let mut root = act_create_step(&node.name, &node.function, 0, candidate)?;
        root = reduce_vertex(root);
        let mut next_id = 0;
        root.assign_ids(&mut next_id);
        let act = Act {
            node_name: node.name.clone(),
            root,
            node_list: candidate.to_vec(),
        };
        let score = mode.score(cost_fn(&act))?;
        if score < best_score {
            best_score = score;
            best_order = candidate.to_vec();
        }
        Ok(())
    })?;

    if best_order.is_empty() {
        Err(ActCreateError::EmptyOptimalOrder {
            node: node.name.clone(),
        })
    } else {
        Ok(best_order)
    }
}

pub fn terminal_act(value: bool, index_size: usize) -> ActVertex {
    ActVertex::terminal(value, index_size)
}

pub fn create_global_act_from_sis_blocked<T>() -> ActCreateResult<T> {
    missing_native_ports("p_applyCreate SIS factor-tree/global ACT integration")
}

pub fn tree_node_dag_from_sis_blocked<T>() -> ActCreateResult<T> {
    missing_native_ports("p_treeNodeDag SIS factor-tree/apply integration")
}

fn missing_native_ports<T>(operation: &'static str) -> ActCreateResult<T> {
    Err(ActCreateError::MissingNativePorts { operation })
}

fn act_create_step(
    node_name: &str,
    expr: &BooleanExpr,
    level: usize,
    order: &[String],
) -> ActCreateResult<ActVertex> {
    match expr.clone().simplified() {
        BooleanExpr::Const(value) => return Ok(ActVertex::terminal(value, order.len())),
        expr => {
            let mut next_level = level;
            while next_level < order.len() {
                let variable = &order[next_level];
                if expr.contains_variable(variable) {
                    let low = expr.cofactor(variable, false).simplified();
                    let high = expr.cofactor(variable, true).simplified();
                    if low != high {
                        let low_vertex = act_create_step(node_name, &low, next_level + 1, order)?;
                        let high_vertex = act_create_step(node_name, &high, next_level + 1, order)?;
                        return Ok(ActVertex::branch(
                            next_level,
                            order.len(),
                            variable.clone(),
                            low_vertex,
                            high_vertex,
                        ));
                    }
                }
                next_level += 1;
            }

            let mut variables = Vec::new();
            expr.collect_variables(&mut variables);
            if let Some(variable) = variables
                .into_iter()
                .find(|variable| !order.iter().any(|ordered| ordered == variable))
            {
                return Err(ActCreateError::MissingOrderVariable {
                    node: node_name.to_owned(),
                    variable,
                });
            }

            Ok(ActVertex::terminal(true, order.len()))
        }
    }
}

fn reduce_vertex(mut vertex: ActVertex) -> ActVertex {
    let Some(low) = vertex.low.take() else {
        return vertex;
    };
    let Some(high) = vertex.high.take() else {
        vertex.low = Some(low);
        return vertex;
    };

    let reduced_low = reduce_vertex(*low);
    let reduced_high = reduce_vertex(*high);
    if reduced_low.structurally_matches(&reduced_high) {
        reduced_low
    } else {
        vertex.low = Some(Box::new(reduced_low));
        vertex.high = Some(Box::new(reduced_high));
        vertex
    }
}

fn heap_permute<F>(n: usize, list: &mut [String], visit: &mut F) -> ActCreateResult<()>
where
    F: FnMut(&[String]) -> ActCreateResult<()>,
{
    if n <= 1 {
        return visit(list);
    }

    heap_permute(n - 1, list, visit)?;
    for c in 0..(n - 1) {
        if n % 2 == 0 {
            list.swap(c, n - 1);
        } else {
            list.swap(0, n - 1);
        }
        heap_permute(n - 1, list, visit)?;
    }
    Ok(())
}

fn shuffle_order(order: &mut [String], mut state: u64) {
    if order.len() < 2 {
        return;
    }

    let original = order.to_vec();
    for i in (1..order.len()).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (state as usize) % (i + 1);
        order.swap(i, j);
    }
    if order == original {
        let amount = (state as usize % (order.len() - 1)) + 1;
        order.rotate_left(amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn create_step_builds_decision_graph_matching_expression() {
        let node = BooleanNode::new(
            "f",
            BooleanExpr::or([
                BooleanExpr::and([BooleanExpr::var("a"), BooleanExpr::var("b")]),
                BooleanExpr::and([
                    BooleanExpr::inv(BooleanExpr::var("a")),
                    BooleanExpr::var("c"),
                ]),
            ]),
        );

        let report = create_local_act(
            &node,
            Some(&names(&["a", "b", "c"])),
            &ActBuildOptions::default(),
        )
        .unwrap();

        let act = &report.entry.act;
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let inputs = [("a", a), ("b", b), ("c", c)];
                    assert_eq!(act.evaluate(inputs), node.function.evaluate(inputs));
                }
            }
        }
        assert_eq!(act.root.node.as_deref(), Some("a"));
    }

    #[test]
    fn create_step_skips_variables_with_equal_cofactors() {
        let node = BooleanNode::new("f", BooleanExpr::var("b"));

        let report = create_local_act(
            &node,
            Some(&names(&["a", "b"])),
            &ActBuildOptions::default(),
        )
        .unwrap();

        assert_eq!(report.entry.act.root.index, 1);
        assert_eq!(report.entry.act.root.node.as_deref(), Some("b"));
    }

    #[test]
    fn optimal_order_uses_supplied_cost_function() {
        let node = BooleanNode::new(
            "f",
            BooleanExpr::or([
                BooleanExpr::var("a"),
                BooleanExpr::and([BooleanExpr::var("b"), BooleanExpr::var("c")]),
            ]),
        );

        let order = optimal_order_by(&node, &names(&["a", "b", "c"]), ActOrderMode::Area, |act| {
            if act.node_list.first().map(String::as_str) == Some("b") {
                ActCost::area(0)
            } else {
                ActCost::area(HICOST)
            }
        })
        .unwrap();

        assert_eq!(order.first().map(String::as_str), Some("b"));
    }

    #[test]
    fn optimal_order_falls_back_when_order_exceeds_limit() {
        let node = BooleanNode::new(
            "f",
            BooleanExpr::and([
                BooleanExpr::var("a"),
                BooleanExpr::var("b"),
                BooleanExpr::var("c"),
            ]),
        );
        let options = ActBuildOptions {
            order_style: ActOrderStyle::Optimal,
            max_optimal: 2,
            ..ActBuildOptions::default()
        };

        let report = create_local_act(&node, Some(&names(&["a", "b", "c"])), &options).unwrap();

        assert_eq!(report.entry.order_style, ActOrderStyle::Fanin);
        assert_eq!(report.entry.act.node_list, names(&["a", "b", "c"]));
        assert_eq!(report.diagnostics.len(), 1);
    }

    #[test]
    fn random_order_style_shuffles_with_seed() {
        let node = BooleanNode::new(
            "f",
            BooleanExpr::and([
                BooleanExpr::var("a"),
                BooleanExpr::var("b"),
                BooleanExpr::var("c"),
                BooleanExpr::var("d"),
            ]),
        );
        let options = ActBuildOptions {
            order_style: ActOrderStyle::Random,
            random_seed: 2,
            ..ActBuildOptions::default()
        };

        let report =
            create_local_act(&node, Some(&names(&["a", "b", "c", "d"])), &options).unwrap();

        assert_eq!(report.entry.order_style, ActOrderStyle::Random);
        assert_ne!(report.entry.act.node_list, names(&["a", "b", "c", "d"]));
    }

    #[test]
    fn missing_global_integration_reports_runtime_diagnostic() {
        let error = create_global_act_from_sis_blocked::<()>().unwrap_err();

        assert_eq!(
            error.to_string(),
            "p_applyCreate SIS factor-tree/global ACT integration requires native SIS prerequisite ports"
        );
    }
}
