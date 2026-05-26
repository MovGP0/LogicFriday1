//! Native Rust support for the ASTG backward linear-programming pass.
//!
//! This module ports the constraint and LP recursion mechanics from
//! `astg/bwd_lp.c`. The original entry point also depends on legacy SIS network,
//! ASTG traversal, delay, cube, and hazard data structures. Those integrations
//! are intentionally kept out of this file until the corresponding native Rust
//! ports expose stable APIs.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

const EPSILON: f64 = 1.0e-8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintKind
{
    Equal,
    GreaterEqual,
    LessEqual,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearConstraint
{
    pub variables: Vec<usize>,
    pub coefficients: Vec<i32>,
    pub value: f64,
    pub kind: ConstraintKind,
    pub level: Option<usize>,
}

impl LinearConstraint
{
    pub fn new(
        variables: Vec<usize>,
        coefficients: Vec<i32>,
        kind: ConstraintKind,
        level: Option<usize>,
        value: f64,
    ) -> Result<Self, BwdLpError>
    {
        if variables.len() != coefficients.len()
        {
            return Err(BwdLpError::ConstraintShapeMismatch
            {
                variables: variables.len(),
                coefficients: coefficients.len(),
            });
        }

        if variables.is_empty()
        {
            return Err(BwdLpError::EmptyConstraint);
        }

        Ok(Self
        {
            variables,
            coefficients,
            value,
            kind,
            level,
        })
    }

    pub fn one(
        coefficient: i32,
        variable: usize,
        kind: ConstraintKind,
        level: Option<usize>,
        value: f64,
    ) -> Self
    {
        Self
        {
            variables: vec![variable],
            coefficients: vec![coefficient],
            value,
            kind,
            level,
        }
    }

    pub fn three(
        terms: [(i32, usize); 3],
        kind: ConstraintKind,
        level: Option<usize>,
        value: f64,
    ) -> Self
    {
        Self
        {
            variables: terms.iter().map(|(_, variable)| *variable).collect(),
            coefficients: terms
                .iter()
                .map(|(coefficient, _)| *coefficient)
                .collect(),
            value,
            kind,
            level,
        }
    }

    pub fn format_lindo(&self) -> String
    {
        let mut text = String::new();

        for (index, (coefficient, variable)) in self
            .coefficients
            .iter()
            .zip(self.variables.iter())
            .enumerate()
        {
            if index != 0 && *coefficient >= 0
            {
                text.push_str("+ ");
            }

            text.push_str(&format!("{coefficient} x{variable} "));
        }

        text.push_str(match self.kind
        {
            ConstraintKind::Equal => "=",
            ConstraintKind::GreaterEqual => ">=",
            ConstraintKind::LessEqual => "<=",
        });
        text.push_str(&format!("{}", self.value));

        text
    }
}

impl Eq for LinearConstraint
{
}

impl Ord for LinearConstraint
{
    fn cmp(&self, other: &Self) -> Ordering
    {
        self.variables
            .len()
            .cmp(&other.variables.len())
            .then_with(|| constraint_kind_rank(self.kind).cmp(&constraint_kind_rank(other.kind)))
            .then_with(||
            {
                self.variables
                    .iter()
                    .zip(self.coefficients.iter())
                    .cmp(other.variables.iter().zip(other.coefficients.iter()))
            })
            .then_with(|| self.value.total_cmp(&other.value))
    }
}

impl PartialOrd for LinearConstraint
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TransitionPolarity
{
    Rising,
    Falling,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition
{
    pub name: String,
    pub signal: String,
    pub polarity: TransitionPolarity,
}

impl Transition
{
    pub fn new(
        name: impl Into<String>,
        signal: impl Into<String>,
        polarity: TransitionPolarity,
    ) -> Self
    {
        Self
        {
            name: name.into(),
            signal: signal.into(),
            polarity,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionArc
{
    pub from: usize,
    pub to: usize,
    pub delay: f64,
}

impl TransitionArc
{
    pub fn new(from: usize, to: usize, delay: f64) -> Self
    {
        Self
        {
            from,
            to,
            delay,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BackwardLpModel
{
    pub names: Vec<String>,
    pub minimized_variables: Vec<usize>,
    pub constraints: Vec<LinearConstraint>,
}

impl BackwardLpModel
{
    pub fn add_signal_padding_variable(&mut self, signal: impl Into<String>) -> usize
    {
        let variable = self.names.len();
        self.names.push(format!("X_{}", signal.into()));
        self.minimized_variables.push(variable);
        variable
    }

    pub fn add_delay_variable(&mut self, transition: &str, pair: usize) -> usize
    {
        let variable = self.names.len();
        self.names.push(format!("D_{transition}_{pair}"));
        variable
    }

    pub fn add_constraint(&mut self, constraint: LinearConstraint) -> bool
    {
        if self.constraints.iter().any(|existing| *existing == constraint)
        {
            return false;
        }

        self.constraints.push(constraint);
        true
    }

    pub fn write_constraints_for_order(
        &mut self,
        transitions: &[Transition],
        arcs: &[TransitionArc],
        order: &[usize],
        signal_variables: &HashMap<String, usize>,
        pair: usize,
        first_level: usize,
        delta: f64,
    ) -> Result<usize, BwdLpError>
    {
        let (&source, &sink) = order
            .first()
            .zip(order.last())
            .ok_or(BwdLpError::EmptyTransitionOrder)?;
        let mut transition_variables = HashMap::new();
        let mut level = first_level;

        for (order_index, transition_index) in order.iter().copied().enumerate()
        {
            let transition = transitions
                .get(transition_index)
                .ok_or(BwdLpError::UnknownTransition
                {
                    transition: transition_index,
                })?;
            let transition_variable = self.add_delay_variable(&transition.name, pair);
            transition_variables.insert(transition_index, transition_variable);

            if transition_index == source
            {
                self.add_constraint(LinearConstraint::one(
                    1,
                    transition_variable,
                    ConstraintKind::Equal,
                    None,
                    0.0,
                ));
            }

            if transition_index == sink && order_index != 0
            {
                self.add_constraint(LinearConstraint::one(
                    1,
                    transition_variable,
                    ConstraintKind::GreaterEqual,
                    None,
                    delta,
                ));
            }

            for arc in arcs.iter().filter(|arc| arc.to == transition_index)
            {
                let Some(from_variable) = transition_variables.get(&arc.from).copied() else
                {
                    continue;
                };
                let signal_variable = *signal_variables
                    .get(&transition.signal)
                    .ok_or_else(|| BwdLpError::MissingSignalVariable
                    {
                        signal: transition.signal.clone(),
                    })?;

                self.add_constraint(LinearConstraint::three(
                    [
                        (1, transition_variable),
                        (-1, from_variable),
                        (-1, signal_variable),
                    ],
                    ConstraintKind::GreaterEqual,
                    Some(level),
                    arc.delay,
                ));
            }

            if transition_index != source
            {
                level += 1;
            }
        }

        Ok(level)
    }

    pub fn solve_min_padding(&self, do_bound: bool) -> Result<BackwardLpSolution, BwdLpError>
    {
        let mut working = self.constraints.clone();
        let mut slow = vec![0.0; self.minimized_variables.len()];
        let mut best_slow = vec![0.0; self.minimized_variables.len()];
        let mut state = SearchState
        {
            best_cost: None,
            bound_count: 0,
            evaluation_count: 0,
        };

        solve_recur(
            &mut working,
            &self.minimized_variables,
            self.names.len(),
            &mut slow,
            &mut best_slow,
            0,
            do_bound,
            &mut state,
        )?;

        Ok(BackwardLpSolution
        {
            cost: state.best_cost.unwrap_or(0.0),
            slowed_variables: self
                .minimized_variables
                .iter()
                .copied()
                .zip(best_slow)
                .collect(),
            bound_count: state.bound_count,
            evaluation_count: state.evaluation_count,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackwardLpSolution
{
    pub cost: f64,
    pub slowed_variables: Vec<(usize, f64)>,
    pub bound_count: usize,
    pub evaluation_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BwdLpError
{
    ConstraintShapeMismatch
    {
        variables: usize,
        coefficients: usize,
    },
    EmptyConstraint,
    EmptyTransitionOrder,
    MissingSignalVariable
    {
        signal: String,
    },
    UnknownTransition
    {
        transition: usize,
    },
    VariableOutOfRange
    {
        variable: usize,
        variable_count: usize,
    },
    Infeasible,
    MissingIntegration
    {
        operation: &'static str,
    },
}

impl fmt::Display for BwdLpError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::ConstraintShapeMismatch
            {
                variables,
                coefficients,
            } => write!(
                f,
                "constraint has {variables} variables but {coefficients} coefficients"
            ),
            Self::EmptyConstraint => write!(f, "constraint must contain at least one variable"),
            Self::EmptyTransitionOrder => write!(f, "transition order must not be empty"),
            Self::MissingSignalVariable
            {
                signal,
            } => write!(f, "missing padding variable for signal {signal}"),
            Self::UnknownTransition
            {
                transition,
            } => write!(f, "unknown transition index {transition}"),
            Self::VariableOutOfRange
            {
                variable,
                variable_count,
            } => write!(
                f,
                "constraint references variable {variable}, but model has {variable_count} variables"
            ),
            Self::Infeasible => write!(f, "linear program is infeasible"),
            Self::MissingIntegration
            {
                operation,
            } => write!(f, "{operation} requires native ASTG/network integration"),
        }
    }
}

impl Error for BwdLpError
{
}

pub fn backward_lp_slowing_requires_native_integration() -> Result<(), BwdLpError>
{
    Err(BwdLpError::MissingIntegration
    {
        operation: "ASTG backward LP slowing",
    })
}

#[derive(Clone, Debug)]
struct SearchState
{
    best_cost: Option<f64>,
    bound_count: usize,
    evaluation_count: usize,
}

fn solve_recur(
    constraints: &mut [LinearConstraint],
    minimized_variables: &[usize],
    variable_count: usize,
    slow: &mut [f64],
    best_slow: &mut [f64],
    start: usize,
    do_bound: bool,
    state: &mut SearchState,
) -> Result<(), BwdLpError>
{
    let mut current = start;

    while current < constraints.len() && constraints[current].level.is_none()
    {
        current += 1;
    }

    if current >= constraints.len()
    {
        let cost = solve_lp(constraints, minimized_variables, variable_count, slow)?;
        state.evaluation_count += 1;

        if state.best_cost.is_none_or(|best_cost| cost < best_cost)
        {
            best_slow.copy_from_slice(slow);
            state.best_cost = Some(cost);
        }

        return Ok(());
    }

    let level = constraints[current].level;
    let mut count = 1;

    while current + count < constraints.len() && constraints[current + count].level == level
    {
        count += 1;
    }

    if do_bound && count > 1
    {
        match solve_lp(constraints, minimized_variables, variable_count, slow)
        {
            Ok(cost) =>
            {
                state.evaluation_count += 1;
                if state.best_cost.is_some_and(|best_cost| cost > best_cost)
                {
                    state.bound_count += 1;
                    return Ok(());
                }
            }
            Err(BwdLpError::Infeasible) =>
            {
                state.evaluation_count += 1;
                state.bound_count += 1;
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }

    for index in 0..count
    {
        let constraint_index = current + index;
        let original_kind = constraints[constraint_index].kind;
        constraints[constraint_index].kind = ConstraintKind::Equal;

        solve_recur(
            constraints,
            minimized_variables,
            variable_count,
            slow,
            best_slow,
            current + count,
            do_bound,
            state,
        )?;

        constraints[constraint_index].kind = original_kind;
    }

    Ok(())
}

fn solve_lp(
    constraints: &[LinearConstraint],
    minimized_variables: &[usize],
    variable_count: usize,
    slow: &mut [f64],
) -> Result<f64, BwdLpError>
{
    for constraint in constraints
    {
        for variable in &constraint.variables
        {
            if *variable >= variable_count
            {
                return Err(BwdLpError::VariableOutOfRange
                {
                    variable: *variable,
                    variable_count,
                });
            }
        }
    }

    let mut equalities = Vec::new();
    let mut inequalities = Vec::new();

    for constraint in constraints
    {
        match constraint.kind
        {
            ConstraintKind::Equal => equalities.push(row_from_constraint(constraint, variable_count)),
            ConstraintKind::LessEqual =>
            {
                inequalities.push(row_from_constraint(constraint, variable_count));
            }
            ConstraintKind::GreaterEqual =>
            {
                let mut row = row_from_constraint(constraint, variable_count);
                row.coefficients.iter_mut().for_each(|coefficient| *coefficient = -*coefficient);
                row.value = -row.value;
                inequalities.push(row);
            }
        }
    }

    for variable in 0..variable_count
    {
        let mut coefficients = vec![0.0; variable_count];
        coefficients[variable] = -1.0;
        inequalities.push(DenseRow
        {
            coefficients,
            value: 0.0,
        });
    }

    let mut best: Option<(f64, Vec<f64>)> = None;
    let required_active = variable_count.saturating_sub(equalities.len());

    if equalities.len() > variable_count
    {
        return Err(BwdLpError::Infeasible);
    }

    enumerate_active_sets(
        &inequalities,
        required_active,
        0,
        &mut Vec::new(),
        &mut |active|
        {
            let mut rows = equalities.clone();
            rows.extend(active.iter().map(|index| inequalities[*index].clone()));

            if let Some(candidate) = solve_dense_equalities(&rows, variable_count)
            {
                if feasible(&candidate, &equalities, &inequalities)
                {
                    let cost = minimized_variables
                        .iter()
                        .map(|variable| candidate[*variable])
                        .sum::<f64>();

                    if best.as_ref().is_none_or(|(best_cost, _)| cost < *best_cost)
                    {
                        best = Some((cost, candidate));
                    }
                }
            }
        },
    );

    let Some((cost, values)) = best else
    {
        return Err(BwdLpError::Infeasible);
    };

    for (index, variable) in minimized_variables.iter().copied().enumerate()
    {
        slow[index] = if values[variable].abs() < EPSILON
        {
            0.0
        }
        else
        {
            values[variable]
        };
    }

    Ok(cost)
}

#[derive(Clone, Debug)]
struct DenseRow
{
    coefficients: Vec<f64>,
    value: f64,
}

fn row_from_constraint(constraint: &LinearConstraint, variable_count: usize) -> DenseRow
{
    let mut coefficients = vec![0.0; variable_count];

    for (variable, coefficient) in constraint.variables.iter().zip(constraint.coefficients.iter())
    {
        coefficients[*variable] += f64::from(*coefficient);
    }

    DenseRow
    {
        coefficients,
        value: constraint.value,
    }
}

fn enumerate_active_sets<F>(
    rows: &[DenseRow],
    size: usize,
    start: usize,
    active: &mut Vec<usize>,
    callback: &mut F,
) where
    F: FnMut(&[usize]),
{
    if active.len() == size
    {
        callback(active);
        return;
    }

    let remaining_needed = size - active.len();
    if rows.len().saturating_sub(start) < remaining_needed
    {
        return;
    }

    for index in start..rows.len()
    {
        active.push(index);
        enumerate_active_sets(rows, size, index + 1, active, callback);
        active.pop();
    }
}

fn solve_dense_equalities(rows: &[DenseRow], variable_count: usize) -> Option<Vec<f64>>
{
    if rows.len() != variable_count
    {
        return None;
    }

    let mut matrix: Vec<Vec<f64>> = rows
        .iter()
        .map(|row|
        {
            let mut values = row.coefficients.clone();
            values.push(row.value);
            values
        })
        .collect();

    for pivot in 0..variable_count
    {
        let mut best_row = pivot;

        for row in (pivot + 1)..variable_count
        {
            if matrix[row][pivot].abs() > matrix[best_row][pivot].abs()
            {
                best_row = row;
            }
        }

        if matrix[best_row][pivot].abs() < EPSILON
        {
            return None;
        }

        matrix.swap(pivot, best_row);
        let divisor = matrix[pivot][pivot];

        for column in pivot..=variable_count
        {
            matrix[pivot][column] /= divisor;
        }

        for row in 0..variable_count
        {
            if row == pivot
            {
                continue;
            }

            let factor = matrix[row][pivot];
            for column in pivot..=variable_count
            {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
        }
    }

    Some(
        matrix
            .iter()
            .map(|row| row[variable_count])
            .collect::<Vec<_>>(),
    )
}

fn feasible(values: &[f64], equalities: &[DenseRow], inequalities: &[DenseRow]) -> bool
{
    equalities.iter().all(|row| (dot(&row.coefficients, values) - row.value).abs() <= EPSILON)
        && inequalities
            .iter()
            .all(|row| dot(&row.coefficients, values) - row.value <= EPSILON)
}

fn dot(left: &[f64], right: &[f64]) -> f64
{
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum()
}

fn constraint_kind_rank(kind: ConstraintKind) -> u8
{
    match kind
    {
        ConstraintKind::Equal => 0,
        ConstraintKind::GreaterEqual => 1,
        ConstraintKind::LessEqual => 2,
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn duplicate_constraints_are_ignored()
    {
        let mut model = BackwardLpModel::default();
        let constraint = LinearConstraint::one(1, 0, ConstraintKind::GreaterEqual, None, 3.0);

        assert!(model.add_constraint(constraint.clone()));
        assert!(!model.add_constraint(constraint));
        assert_eq!(model.constraints.len(), 1);
    }

    #[test]
    fn constraints_format_like_lindo_rows()
    {
        let constraint = LinearConstraint::three(
            [(1, 3), (-1, 1), (-1, 0)],
            ConstraintKind::GreaterEqual,
            Some(4),
            2.5,
        );

        assert_eq!(constraint.format_lindo(), "1 x3 -1 x1 -1 x0 >=2.5");
    }

    #[test]
    fn transition_order_writes_source_sink_and_arc_constraints()
    {
        let transitions = vec![
            Transition::new("t2", "b", TransitionPolarity::Rising),
            Transition::new("mid", "c", TransitionPolarity::Rising),
            Transition::new("t1", "a", TransitionPolarity::Falling),
        ];
        let arcs = vec![
            TransitionArc::new(0, 1, 1.25),
            TransitionArc::new(1, 2, 2.0),
        ];
        let mut model = BackwardLpModel::default();
        let x_a = model.add_signal_padding_variable("a");
        let x_c = model.add_signal_padding_variable("c");
        let signal_variables = HashMap::from([("a".to_string(), x_a), ("c".to_string(), x_c)]);

        let next_level = model
            .write_constraints_for_order(
                &transitions,
                &arcs,
                &[0, 1, 2],
                &signal_variables,
                7,
                10,
                4.0,
            )
            .unwrap();

        assert_eq!(next_level, 12);
        assert!(model.names.contains(&"D_t2_7".to_string()));
        assert!(model.names.contains(&"D_t1_7".to_string()));
        assert_eq!(
            model
                .constraints
                .iter()
                .filter(|constraint| constraint.level.is_some())
                .count(),
            2
        );
        assert!(model.constraints.iter().any(|constraint| {
            constraint.kind == ConstraintKind::GreaterEqual && constraint.value == 4.0
        }));
    }

    #[test]
    fn solves_minimum_padding_for_simple_difference_constraint()
    {
        let mut model = BackwardLpModel::default();
        let x_a = model.add_signal_padding_variable("a");
        let d_source = model.add_delay_variable("source", 1);
        let d_sink = model.add_delay_variable("sink", 1);

        model.add_constraint(LinearConstraint::one(
            1,
            d_source,
            ConstraintKind::Equal,
            None,
            0.0,
        ));
        model.add_constraint(LinearConstraint::one(
            1,
            d_sink,
            ConstraintKind::GreaterEqual,
            None,
            5.0,
        ));
        model.add_constraint(LinearConstraint::three(
            [(1, d_sink), (-1, d_source), (-1, x_a)],
            ConstraintKind::GreaterEqual,
            Some(0),
            2.0,
        ));

        let solution = model.solve_min_padding(false).unwrap();

        assert_eq!(solution.cost, 3.0);
        assert_eq!(solution.slowed_variables, vec![(x_a, 3.0)]);
        assert_eq!(solution.evaluation_count, 1);
    }

    #[test]
    fn max_level_recursion_selects_best_equal_constraint()
    {
        let mut model = BackwardLpModel::default();
        let x_a = model.add_signal_padding_variable("a");
        let x_b = model.add_signal_padding_variable("b");

        model.add_constraint(LinearConstraint::one(
            1,
            x_a,
            ConstraintKind::GreaterEqual,
            Some(0),
            3.0,
        ));
        model.add_constraint(LinearConstraint::one(
            1,
            x_b,
            ConstraintKind::GreaterEqual,
            Some(0),
            1.0,
        ));

        let solution = model.solve_min_padding(false).unwrap();

        assert_eq!(solution.cost, 4.0);
        assert_eq!(solution.slowed_variables, vec![(x_a, 3.0), (x_b, 1.0)]);
        assert_eq!(solution.evaluation_count, 2);
    }

    #[test]
    fn reports_missing_full_astg_network_integration()
    {
        assert_eq!(
            backward_lp_slowing_requires_native_integration(),
            Err(BwdLpError::MissingIntegration
            {
                operation: "ASTG backward LP slowing",
            })
        );
    }
}
