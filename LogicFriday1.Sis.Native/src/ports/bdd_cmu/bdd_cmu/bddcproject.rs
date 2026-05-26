use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Bdd(usize);

impl Bdd {
    pub const FALSE: Self = Self(0);
    pub const TRUE: Self = Self(1);

    pub const fn is_false(self) -> bool {
        self.0 == Self::FALSE.0
    }

    pub const fn is_true(self) -> bool {
        self.0 == Self::TRUE.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BddNode {
    variable: usize,
    high: Bdd,
    low: Bdd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddProjectError {
    InvalidHandle(Bdd),
    TruthAssignmentLength {
        expected: usize,
        actual: usize,
    },
    VariableOutOfRange {
        variable: usize,
        variable_count: usize,
    },
}

impl fmt::Display for BddProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle(bdd) => write!(formatter, "invalid BDD handle {bdd:?}"),
            Self::TruthAssignmentLength { expected, actual } => write!(
                formatter,
                "truth assignment has {actual} values, expected {expected}"
            ),
            Self::VariableOutOfRange {
                variable,
                variable_count,
            } => write!(
                formatter,
                "variable {variable} is outside the manager range 0..{variable_count}"
            ),
        }
    }
}

impl std::error::Error for BddProjectError {}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum CacheKey {
    CompatibleProject(Bdd, Vec<usize>),
    GuardedSmooth {
        bdd: Bdd,
        guard_variable: usize,
        variables: Vec<usize>,
    },
    Ite(Bdd, Bdd, Bdd),
    Not(Bdd),
}

#[derive(Clone, Debug)]
pub struct BddManager {
    variable_count: usize,
    nodes: Vec<BddNode>,
    unique: HashMap<BddNode, Bdd>,
    cache: HashMap<CacheKey, Bdd>,
    current_projection_variables: Vec<bool>,
}

impl BddManager {
    pub fn new(variable_count: usize) -> Self {
        Self {
            variable_count,
            nodes: Vec::new(),
            unique: HashMap::new(),
            cache: HashMap::new(),
            current_projection_variables: vec![false; variable_count],
        }
    }

    pub const fn constant_false(&self) -> Bdd {
        Bdd::FALSE
    }

    pub const fn constant_true(&self) -> Bdd {
        Bdd::TRUE
    }

    pub fn variable(&mut self, variable: usize) -> Result<Bdd, BddProjectError> {
        self.check_variable(variable)?;

        Ok(self.find_or_add(variable, Bdd::TRUE, Bdd::FALSE))
    }

    pub fn set_current_projection_variables<I>(
        &mut self,
        variables: I,
    ) -> Result<(), BddProjectError>
    where
        I: IntoIterator<Item = usize>,
    {
        self.current_projection_variables.fill(false);

        for variable in variables {
            self.check_variable(variable)?;
            self.current_projection_variables[variable] = true;
        }

        self.cache.clear();
        Ok(())
    }

    pub fn not(&mut self, bdd: Bdd) -> Result<Bdd, BddProjectError> {
        self.check_handle(bdd)?;

        Ok(self.not_step(bdd))
    }

    pub fn ite(&mut self, condition: Bdd, high: Bdd, low: Bdd) -> Result<Bdd, BddProjectError> {
        self.check_handle(condition)?;
        self.check_handle(high)?;
        self.check_handle(low)?;

        Ok(self.ite_step(condition, high, low))
    }

    pub fn compatible_project(&mut self, bdd: Bdd) -> Result<Bdd, BddProjectError> {
        self.check_handle(bdd)?;

        let variables = self.projection_variables();
        Ok(self.compatible_project_step(bdd, &variables))
    }

    pub fn evaluate(&self, bdd: Bdd, assignment: &[bool]) -> Result<bool, BddProjectError> {
        self.check_handle(bdd)?;

        if assignment.len() != self.variable_count {
            return Err(BddProjectError::TruthAssignmentLength {
                expected: self.variable_count,
                actual: assignment.len(),
            });
        }

        let mut cursor = bdd;
        loop {
            if cursor.is_false() {
                return Ok(false);
            }

            if cursor.is_true() {
                return Ok(true);
            }

            let node = self.node(cursor)?;
            cursor = if assignment[node.variable] {
                node.high
            } else {
                node.low
            };
        }
    }

    fn compatible_project_step(&mut self, bdd: Bdd, variables: &[usize]) -> Bdd {
        if bdd.is_false() || bdd.is_true() {
            return bdd;
        }

        let Some(last_variable) = variables.last().copied() else {
            return bdd;
        };

        let node = self.node_unchecked(bdd);
        if node.variable > last_variable {
            return bdd;
        }

        let key = CacheKey::CompatibleProject(bdd, variables.to_vec());
        if let Some(cached) = self.cache.get(&key) {
            return *cached;
        }

        let result = if variables.binary_search(&node.variable).is_ok() {
            let smoothed_high = self.guarded_smooth_step(node.high, node.variable, variables);
            if smoothed_high.is_true() {
                let projected_high = self.compatible_project_step(node.high, variables);
                self.find_or_add(node.variable, projected_high, Bdd::FALSE)
            } else if smoothed_high.is_false() {
                let projected_low = self.compatible_project_step(node.low, variables);
                self.find_or_add(node.variable, Bdd::FALSE, projected_low)
            } else {
                let projected_high = self.compatible_project_step(node.high, variables);
                let projected_low = self.compatible_project_step(node.low, variables);
                let guarded_low = self.ite_step(smoothed_high, Bdd::FALSE, projected_low);

                self.find_or_add(node.variable, projected_high, guarded_low)
            }
        } else {
            let high = self.compatible_project_step(node.high, variables);
            let low = self.compatible_project_step(node.low, variables);

            self.find_or_add(node.variable, high, low)
        };

        self.cache.insert(key, result);
        result
    }

    fn guarded_smooth_step(&mut self, bdd: Bdd, guard_variable: usize, variables: &[usize]) -> Bdd {
        if bdd.is_false() || bdd.is_true() {
            return bdd;
        }

        let Some(last_variable) = variables.last().copied() else {
            return bdd;
        };

        let node = self.node_unchecked(bdd);
        if node.variable > last_variable {
            return bdd;
        }

        let key = CacheKey::GuardedSmooth {
            bdd,
            guard_variable,
            variables: variables.to_vec(),
        };
        if let Some(cached) = self.cache.get(&key) {
            return *cached;
        }

        let high = self.guarded_smooth_step(node.high, guard_variable, variables);
        let quantifying = variables.binary_search(&node.variable).is_ok();
        let result = if quantifying && high.is_true() && node.variable > guard_variable {
            high
        } else {
            let low = self.guarded_smooth_step(node.low, guard_variable, variables);
            if quantifying {
                self.ite_step(high, Bdd::TRUE, low)
            } else {
                self.find_or_add(node.variable, high, low)
            }
        };

        self.cache.insert(key, result);
        result
    }

    fn ite_step(&mut self, condition: Bdd, high: Bdd, low: Bdd) -> Bdd {
        if condition.is_true() {
            return high;
        }

        if condition.is_false() {
            return low;
        }

        if high == low {
            return high;
        }

        if high.is_true() && low.is_false() {
            return condition;
        }

        let key = CacheKey::Ite(condition, high, low);
        if let Some(cached) = self.cache.get(&key) {
            return *cached;
        }

        let variable = self
            .top_variable_of(condition)
            .min(self.top_variable_of(high))
            .min(self.top_variable_of(low));
        let (condition_high, condition_low) = self.cofactors(condition, variable);
        let (high_high, high_low) = self.cofactors(high, variable);
        let (low_high, low_low) = self.cofactors(low, variable);
        let result_high = self.ite_step(condition_high, high_high, low_high);
        let result_low = self.ite_step(condition_low, high_low, low_low);
        let result = self.find_or_add(variable, result_high, result_low);

        self.cache.insert(key, result);
        result
    }

    fn not_step(&mut self, bdd: Bdd) -> Bdd {
        if bdd.is_false() {
            return Bdd::TRUE;
        }

        if bdd.is_true() {
            return Bdd::FALSE;
        }

        let key = CacheKey::Not(bdd);
        if let Some(cached) = self.cache.get(&key) {
            return *cached;
        }

        let node = self.node_unchecked(bdd);
        let high = self.not_step(node.high);
        let low = self.not_step(node.low);
        let result = self.find_or_add(node.variable, high, low);

        self.cache.insert(key, result);
        result
    }

    fn find_or_add(&mut self, variable: usize, high: Bdd, low: Bdd) -> Bdd {
        if high == low {
            return high;
        }

        let node = BddNode {
            variable,
            high,
            low,
        };

        if let Some(existing) = self.unique.get(&node) {
            return *existing;
        }

        let bdd = Bdd(self.nodes.len() + 2);
        self.nodes.push(node);
        self.unique.insert(node, bdd);

        bdd
    }

    fn projection_variables(&self) -> Vec<usize> {
        self.current_projection_variables
            .iter()
            .enumerate()
            .filter_map(|(variable, selected)| selected.then_some(variable))
            .collect()
    }

    fn top_variable_of(&self, bdd: Bdd) -> usize {
        if bdd.is_false() || bdd.is_true() {
            usize::MAX
        } else {
            self.node_unchecked(bdd).variable
        }
    }

    fn cofactors(&self, bdd: Bdd, variable: usize) -> (Bdd, Bdd) {
        if bdd.is_false() || bdd.is_true() {
            return (bdd, bdd);
        }

        let node = self.node_unchecked(bdd);
        if node.variable == variable {
            (node.high, node.low)
        } else {
            (bdd, bdd)
        }
    }

    fn check_variable(&self, variable: usize) -> Result<(), BddProjectError> {
        if variable >= self.variable_count {
            return Err(BddProjectError::VariableOutOfRange {
                variable,
                variable_count: self.variable_count,
            });
        }

        Ok(())
    }

    fn check_handle(&self, bdd: Bdd) -> Result<(), BddProjectError> {
        if bdd.is_false() || bdd.is_true() || bdd.0.saturating_sub(2) < self.nodes.len() {
            return Ok(());
        }

        Err(BddProjectError::InvalidHandle(bdd))
    }

    fn node(&self, bdd: Bdd) -> Result<BddNode, BddProjectError> {
        self.nodes
            .get(bdd.0.saturating_sub(2))
            .copied()
            .ok_or(BddProjectError::InvalidHandle(bdd))
    }

    fn node_unchecked(&self, bdd: Bdd) -> BddNode {
        self.nodes[bdd.0 - 2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_assignments(variable_count: usize) -> Vec<Vec<bool>> {
        (0..(1_usize << variable_count))
            .map(|bits| {
                (0..variable_count)
                    .map(|variable| (bits & (1 << variable)) != 0)
                    .collect()
            })
            .collect()
    }

    fn assert_project_implies_original(manager: &BddManager, projected: Bdd, original: Bdd) {
        for assignment in all_assignments(manager.variable_count) {
            let projected_value = manager.evaluate(projected, &assignment).unwrap();
            let original_value = manager.evaluate(original, &assignment).unwrap();

            assert!(
                !projected_value || original_value,
                "compatible projection must be a cover subset at {assignment:?}"
            );
        }
    }

    #[test]
    fn projection_keeps_high_branch_when_it_covers_all_remaining_quantified_space() {
        let mut manager = BddManager::new(2);
        let x = manager.variable(0).unwrap();
        let y = manager.variable(1).unwrap();
        let function = manager.ite(x, manager.constant_true(), y).unwrap();

        manager.set_current_projection_variables([0]).unwrap();
        let projected = manager.compatible_project(function).unwrap();

        assert_eq!(projected, x);
        assert_project_implies_original(&manager, projected, function);
    }

    #[test]
    fn projection_keeps_low_branch_when_high_branch_is_unsatisfiable() {
        let mut manager = BddManager::new(2);
        let x = manager.variable(0).unwrap();
        let y = manager.variable(1).unwrap();
        let not_x = manager.not(x).unwrap();
        let function = manager.ite(x, manager.constant_false(), y).unwrap();

        manager.set_current_projection_variables([0]).unwrap();
        let projected = manager.compatible_project(function).unwrap();
        let expected = manager.ite(not_x, y, manager.constant_false()).unwrap();

        for assignment in all_assignments(2) {
            assert_eq!(
                manager.evaluate(projected, &assignment).unwrap(),
                manager.evaluate(expected, &assignment).unwrap()
            );
        }
    }

    #[test]
    fn projection_guards_low_branch_when_high_branch_is_partially_satisfiable() {
        let mut manager = BddManager::new(3);
        let x = manager.variable(0).unwrap();
        let y = manager.variable(1).unwrap();
        let z = manager.variable(2).unwrap();
        let function = manager.ite(x, y, z).unwrap();

        manager.set_current_projection_variables([0]).unwrap();
        let projected = manager.compatible_project(function).unwrap();

        assert_project_implies_original(&manager, projected, function);

        assert_eq!(
            manager.evaluate(projected, &[false, false, true]).unwrap(),
            true
        );
        assert_eq!(
            manager.evaluate(projected, &[false, true, true]).unwrap(),
            false
        );
        assert_eq!(
            manager.evaluate(projected, &[true, true, false]).unwrap(),
            true
        );
    }

    #[test]
    fn guarded_smoothing_short_circuits_on_deeper_quantified_true_branch() {
        let mut manager = BddManager::new(3);
        let x = manager.variable(0).unwrap();
        let y = manager.variable(1).unwrap();
        let z = manager.variable(2).unwrap();
        let function = manager.ite(x, y, z).unwrap();

        manager.set_current_projection_variables([0, 1]).unwrap();
        let projected = manager.compatible_project(function).unwrap();
        let expected = manager.ite(x, y, manager.constant_false()).unwrap();

        for assignment in all_assignments(3) {
            assert_eq!(
                manager.evaluate(projected, &assignment).unwrap(),
                manager.evaluate(expected, &assignment).unwrap()
            );
        }
    }

    #[test]
    fn variables_below_current_association_are_left_unchanged() {
        let mut manager = BddManager::new(3);
        let x = manager.variable(0).unwrap();
        let z = manager.variable(2).unwrap();
        let function = manager.ite(z, x, manager.constant_false()).unwrap();

        manager.set_current_projection_variables([0]).unwrap();
        let projected = manager.compatible_project(function).unwrap();

        assert_eq!(projected, function);
    }

    #[test]
    fn invalid_handles_are_rejected() {
        let mut manager = BddManager::new(1);

        assert_eq!(
            manager.compatible_project(Bdd(99)),
            Err(BddProjectError::InvalidHandle(Bdd(99)))
        );
    }
}
