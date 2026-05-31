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
pub enum BddError {
    VariableOutOfRange {
        variable: usize,
        variable_count: usize,
    },
    InvalidHandle(Bdd),
    TruthAssignmentLength {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for BddError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VariableOutOfRange {
                variable,
                variable_count,
            } => write!(
                formatter,
                "variable {variable} is outside the manager range 0..{variable_count}"
            ),
            Self::InvalidHandle(bdd) => write!(formatter, "invalid BDD handle {bdd:?}"),
            Self::TruthAssignmentLength { expected, actual } => write!(
                formatter,
                "truth assignment has {actual} values, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for BddError {}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum CacheKey {
    And(Bdd, Bdd),
    Or(Bdd, Bdd),
    Not(Bdd),
    Exists(Bdd, Vec<usize>),
    RelProduct(Bdd, Bdd, Vec<usize>),
}

#[derive(Clone, Debug)]
pub struct BddManager {
    variable_count: usize,
    nodes: Vec<BddNode>,
    unique: HashMap<BddNode, Bdd>,
    cache: HashMap<CacheKey, Bdd>,
    current_quantified_variables: Vec<bool>,
}

impl BddManager {
    pub fn new(variable_count: usize) -> Self {
        Self {
            variable_count,
            nodes: Vec::new(),
            unique: HashMap::new(),
            cache: HashMap::new(),
            current_quantified_variables: vec![false; variable_count],
        }
    }

    pub const fn constant_false(&self) -> Bdd {
        Bdd::FALSE
    }

    pub const fn constant_true(&self) -> Bdd {
        Bdd::TRUE
    }

    pub fn variable_count(&self) -> usize {
        self.variable_count
    }

    pub fn variable(&mut self, variable: usize) -> Result<Bdd, BddError> {
        self.check_variable(variable)?;

        Ok(self.find_or_add(variable, Bdd::TRUE, Bdd::FALSE))
    }

    pub fn set_current_quantified_variables<I>(&mut self, variables: I) -> Result<(), BddError>
    where
        I: IntoIterator<Item = usize>,
    {
        self.current_quantified_variables.fill(false);

        for variable in variables {
            self.check_variable(variable)?;
            self.current_quantified_variables[variable] = true;
        }

        self.cache.clear();
        Ok(())
    }

    pub fn and(&mut self, left: Bdd, right: Bdd) -> Result<Bdd, BddError> {
        self.check_handle(left)?;
        self.check_handle(right)?;
        Ok(self.and_step(left, right))
    }

    pub fn or(&mut self, left: Bdd, right: Bdd) -> Result<Bdd, BddError> {
        self.check_handle(left)?;
        self.check_handle(right)?;
        Ok(self.or_step(left, right))
    }

    pub fn not(&mut self, bdd: Bdd) -> Result<Bdd, BddError> {
        self.check_handle(bdd)?;
        Ok(self.not_step(bdd))
    }

    pub fn exists_current(&mut self, bdd: Bdd) -> Result<Bdd, BddError> {
        self.check_handle(bdd)?;

        let quantified = self.quantified_variables();
        Ok(self.exists_step(bdd, &quantified))
    }

    pub fn relational_product(&mut self, left: Bdd, right: Bdd) -> Result<Bdd, BddError> {
        self.check_handle(left)?;
        self.check_handle(right)?;

        let quantified = self.quantified_variables();
        Ok(self.rel_product_step(left, right, &quantified))
    }

    pub fn evaluate(&self, bdd: Bdd, assignment: &[bool]) -> Result<bool, BddError> {
        self.check_handle(bdd)?;

        if assignment.len() != self.variable_count {
            return Err(BddError::TruthAssignmentLength {
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

    fn and_step(&mut self, mut left: Bdd, mut right: Bdd) -> Bdd {
        if left.is_false() || right.is_false() {
            return Bdd::FALSE;
        }

        if left.is_true() {
            return right;
        }

        if right.is_true() || left == right {
            return left;
        }

        if right < left {
            std::mem::swap(&mut left, &mut right);
        }

        let key = CacheKey::And(left, right);
        if let Some(cached) = self.cache.get(&key) {
            return *cached;
        }

        let variable = self.top_variable(left, right);
        let (left_high, left_low) = self.cofactors(left, variable);
        let (right_high, right_low) = self.cofactors(right, variable);
        let high = self.and_step(left_high, right_high);
        let low = self.and_step(left_low, right_low);
        let result = self.find_or_add(variable, high, low);

        self.cache.insert(key, result);
        result
    }

    fn or_step(&mut self, mut left: Bdd, mut right: Bdd) -> Bdd {
        if left.is_true() || right.is_true() {
            return Bdd::TRUE;
        }

        if left.is_false() {
            return right;
        }

        if right.is_false() || left == right {
            return left;
        }

        if right < left {
            std::mem::swap(&mut left, &mut right);
        }

        let key = CacheKey::Or(left, right);
        if let Some(cached) = self.cache.get(&key) {
            return *cached;
        }

        let variable = self.top_variable(left, right);
        let (left_high, left_low) = self.cofactors(left, variable);
        let (right_high, right_low) = self.cofactors(right, variable);
        let high = self.or_step(left_high, right_high);
        let low = self.or_step(left_low, right_low);
        let result = self.find_or_add(variable, high, low);

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

    fn exists_step(&mut self, bdd: Bdd, quantified: &[usize]) -> Bdd {
        if bdd.is_false() || bdd.is_true() {
            return bdd;
        }

        let Some(last_quantified) = quantified.last().copied() else {
            return bdd;
        };

        let node = self.node_unchecked(bdd);
        if node.variable > last_quantified {
            return bdd;
        }

        let key = CacheKey::Exists(bdd, quantified.to_vec());
        if let Some(cached) = self.cache.get(&key) {
            return *cached;
        }

        let high = self.exists_step(node.high, quantified);
        let result = if quantified.binary_search(&node.variable).is_ok() && high.is_true() {
            high
        } else {
            let low = self.exists_step(node.low, quantified);
            if quantified.binary_search(&node.variable).is_ok() {
                self.or_step(high, low)
            } else {
                self.find_or_add(node.variable, high, low)
            }
        };

        self.cache.insert(key, result);
        result
    }

    fn rel_product_step(&mut self, mut left: Bdd, mut right: Bdd, quantified: &[usize]) -> Bdd {
        if left.is_false() || right.is_false() {
            return Bdd::FALSE;
        }

        if left.is_true() {
            return self.exists_step(right, quantified);
        }

        if right.is_true() {
            return self.exists_step(left, quantified);
        }

        if self.top_variable_of(left) > quantified.last().copied().unwrap_or(usize::MAX)
            && self.top_variable_of(right) > quantified.last().copied().unwrap_or(usize::MAX)
        {
            return self.and_step(left, right);
        }

        if right < left {
            std::mem::swap(&mut left, &mut right);
        }

        let key = CacheKey::RelProduct(left, right, quantified.to_vec());
        if let Some(cached) = self.cache.get(&key) {
            return *cached;
        }

        let variable = self.top_variable(left, right);
        let (left_high, left_low) = self.cofactors(left, variable);
        let (right_high, right_low) = self.cofactors(right, variable);
        let high = self.rel_product_step(left_high, right_high, quantified);
        let result = if quantified.binary_search(&variable).is_ok() && high.is_true() {
            high
        } else {
            let low = self.rel_product_step(left_low, right_low, quantified);
            if quantified.binary_search(&variable).is_ok() {
                self.or_step(high, low)
            } else {
                self.find_or_add(variable, high, low)
            }
        };

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

    fn quantified_variables(&self) -> Vec<usize> {
        self.current_quantified_variables
            .iter()
            .enumerate()
            .filter_map(|(variable, quantified)| quantified.then_some(variable))
            .collect()
    }

    fn top_variable(&self, left: Bdd, right: Bdd) -> usize {
        self.top_variable_of(left).min(self.top_variable_of(right))
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

    fn check_variable(&self, variable: usize) -> Result<(), BddError> {
        if variable >= self.variable_count {
            return Err(BddError::VariableOutOfRange {
                variable,
                variable_count: self.variable_count,
            });
        }

        Ok(())
    }

    fn check_handle(&self, bdd: Bdd) -> Result<(), BddError> {
        if bdd.is_false() || bdd.is_true() || bdd.0 - 2 < self.nodes.len() {
            return Ok(());
        }

        Err(BddError::InvalidHandle(bdd))
    }

    fn node(&self, bdd: Bdd) -> Result<BddNode, BddError> {
        self.nodes
            .get(bdd.0.saturating_sub(2))
            .copied()
            .ok_or(BddError::InvalidHandle(bdd))
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

    #[test]
    fn relational_product_conjoins_and_quantifies_current_variables() {
        let mut manager = BddManager::new(3);
        let x = manager.variable(0).unwrap();
        let y = manager.variable(1).unwrap();
        let z = manager.variable(2).unwrap();
        let x_and_y = manager.and(x, y).unwrap();
        let y_and_z = manager.and(y, z).unwrap();

        manager.set_current_quantified_variables([1]).unwrap();
        let result = manager.relational_product(x_and_y, y_and_z).unwrap();
        let expected = manager.and(x, z).unwrap();

        for assignment in all_assignments(3) {
            assert_eq!(
                manager.evaluate(result, &assignment).unwrap(),
                manager.evaluate(expected, &assignment).unwrap()
            );
        }
    }

    #[test]
    fn relational_product_short_circuits_when_one_side_is_true() {
        let mut manager = BddManager::new(2);
        let x = manager.variable(0).unwrap();
        let y = manager.variable(1).unwrap();
        let x_and_y = manager.and(x, y).unwrap();

        manager.set_current_quantified_variables([1]).unwrap();
        let result = manager
            .relational_product(manager.constant_true(), x_and_y)
            .unwrap();

        for assignment in all_assignments(2) {
            assert_eq!(
                manager.evaluate(result, &assignment).unwrap(),
                assignment[0]
            );
        }
    }

    #[test]
    fn relational_product_uses_plain_conjunction_when_no_quantified_variables_remain() {
        let mut manager = BddManager::new(3);
        let y = manager.variable(1).unwrap();
        let z = manager.variable(2).unwrap();
        let z_or_y = manager.or(z, y).unwrap();
        let y_or_z = manager.or(y, z).unwrap();

        manager.set_current_quantified_variables([0]).unwrap();
        let result = manager.relational_product(z_or_y, y_or_z).unwrap();
        let expected = manager.and(z_or_y, y_or_z).unwrap();

        for assignment in all_assignments(3) {
            assert_eq!(
                manager.evaluate(result, &assignment).unwrap(),
                manager.evaluate(expected, &assignment).unwrap()
            );
        }
    }

    #[test]
    fn relational_product_observes_multiple_quantified_variables() {
        let mut manager = BddManager::new(4);
        let w = manager.variable(0).unwrap();
        let x = manager.variable(1).unwrap();
        let y = manager.variable(2).unwrap();
        let z = manager.variable(3).unwrap();
        let left = manager.and(w, x).unwrap();
        let right = manager.and(y, z).unwrap();
        let conjunction = manager.and(left, right).unwrap();

        manager.set_current_quantified_variables([1, 2]).unwrap();
        let result = manager.relational_product(left, right).unwrap();
        let expected = manager.exists_current(conjunction).unwrap();

        for assignment in all_assignments(4) {
            assert_eq!(
                manager.evaluate(result, &assignment).unwrap(),
                manager.evaluate(expected, &assignment).unwrap()
            );
        }
    }

    #[test]
    fn invalid_handles_are_rejected() {
        let mut manager = BddManager::new(1);

        assert_eq!(
            manager.relational_product(Bdd(99), manager.constant_true()),
            Err(BddError::InvalidHandle(Bdd(99)))
        );
    }
}
