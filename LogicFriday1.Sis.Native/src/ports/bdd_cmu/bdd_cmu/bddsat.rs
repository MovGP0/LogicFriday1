use std::collections::HashMap;
use std::fmt;

pub type BddIndex = usize;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Bdd(usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct UniqueKey {
    index: BddIndex,
    then_branch: Bdd,
    else_branch: Bdd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BddNode {
    index: BddIndex,
    then_branch: Bdd,
    else_branch: Bdd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddSatError {
    Unsatisfiable,
    UnknownNode(Bdd),
    InvalidVariableIndex(BddIndex),
}

impl fmt::Display for BddSatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsatisfiable => formatter.write_str("BDD is false"),
            Self::UnknownNode(bdd) => write!(formatter, "unknown BDD node {bdd:?}"),
            Self::InvalidVariableIndex(index) => {
                write!(formatter, "invalid BDD variable index {index}")
            }
        }
    }
}

impl std::error::Error for BddSatError {}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<Option<BddNode>>,
    unique: HashMap<UniqueKey, Bdd>,
    fraction_cache: HashMap<Bdd, f64>,
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            nodes: vec![None, None],
            unique: HashMap::new(),
            fraction_cache: HashMap::new(),
        }
    }

    pub fn zero(&self) -> Bdd {
        Bdd(0)
    }

    pub fn one(&self) -> Bdd {
        Bdd(1)
    }

    pub fn is_zero(&self, bdd: Bdd) -> bool {
        bdd == self.zero()
    }

    pub fn is_one(&self, bdd: Bdd) -> bool {
        bdd == self.one()
    }

    pub fn variable(&mut self, index: BddIndex) -> Result<Bdd, BddSatError> {
        self.find(index, self.one(), self.zero())
    }

    pub fn find(
        &mut self,
        index: BddIndex,
        then_branch: Bdd,
        else_branch: Bdd,
    ) -> Result<Bdd, BddSatError> {
        self.validate_index(index)?;
        self.validate_bdd(then_branch)?;
        self.validate_bdd(else_branch)?;

        if then_branch == else_branch {
            return Ok(then_branch);
        }

        let key = UniqueKey {
            index,
            then_branch,
            else_branch,
        };

        if let Some(bdd) = self.unique.get(&key) {
            return Ok(*bdd);
        }

        let bdd = Bdd(self.nodes.len());
        self.nodes.push(Some(BddNode {
            index,
            then_branch,
            else_branch,
        }));
        self.unique.insert(key, bdd);
        self.fraction_cache.clear();

        Ok(bdd)
    }

    pub fn satisfy(&mut self, bdd: Bdd) -> Result<Bdd, BddSatError> {
        self.validate_bdd(bdd)?;

        if bdd == self.zero() {
            return Err(BddSatError::Unsatisfiable);
        }

        self.satisfy_step(bdd)
    }

    pub fn satisfy_support<I>(&mut self, bdd: Bdd, support: I) -> Result<Bdd, BddSatError>
    where
        I: IntoIterator<Item = BddIndex>,
    {
        self.validate_bdd(bdd)?;

        if bdd == self.zero() {
            return Err(BddSatError::Unsatisfiable);
        }

        let mut support = support.into_iter().collect::<Vec<_>>();
        support.sort_unstable();
        support.dedup();

        for index in &support {
            self.validate_index(*index)?;
        }

        self.satisfy_support_step(bdd, &support)
    }

    pub fn satisfying_fraction(&mut self, bdd: Bdd) -> Result<f64, BddSatError> {
        self.validate_bdd(bdd)?;
        self.satisfying_fraction_step(bdd)
    }

    pub fn evaluate<I>(&self, bdd: Bdd, true_indexes: I) -> Result<bool, BddSatError>
    where
        I: IntoIterator<Item = BddIndex>,
    {
        self.validate_bdd(bdd)?;
        let mut true_indexes = true_indexes.into_iter().collect::<Vec<_>>();
        true_indexes.sort_unstable();

        let mut cursor = bdd;
        loop {
            if cursor == self.zero() {
                return Ok(false);
            }

            if cursor == self.one() {
                return Ok(true);
            }

            let node = self.node(cursor)?;
            cursor = if true_indexes.binary_search(&node.index).is_ok() {
                node.then_branch
            } else {
                node.else_branch
            };
        }
    }

    fn satisfy_step(&mut self, bdd: Bdd) -> Result<Bdd, BddSatError> {
        if self.is_constant(bdd) {
            return Ok(bdd);
        }

        let node = self.node(bdd)?;
        if node.then_branch == self.zero() {
            let child = self.satisfy_step(node.else_branch)?;
            self.find(node.index, self.zero(), child)
        } else {
            let child = self.satisfy_step(node.then_branch)?;
            self.find(node.index, child, self.zero())
        }
    }

    fn satisfy_support_step(&mut self, bdd: Bdd, support: &[BddIndex]) -> Result<Bdd, BddSatError> {
        if support.is_empty() {
            return self.satisfy_step(bdd);
        }

        if self.is_constant(bdd) {
            let child = self.satisfy_support_step(bdd, &support[1..])?;
            return self.find(support[0], self.zero(), child);
        }

        let node = self.node(bdd)?;
        if node.index <= support[0] {
            let next_support = if node.index == support[0] {
                &support[1..]
            } else {
                support
            };

            if node.then_branch == self.zero() {
                let child = self.satisfy_support_step(node.else_branch, next_support)?;
                self.find(node.index, self.zero(), child)
            } else {
                let child = self.satisfy_support_step(node.then_branch, next_support)?;
                self.find(node.index, child, self.zero())
            }
        } else {
            let child = self.satisfy_support_step(bdd, &support[1..])?;
            self.find(support[0], self.zero(), child)
        }
    }

    fn satisfying_fraction_step(&mut self, bdd: Bdd) -> Result<f64, BddSatError> {
        if bdd == self.zero() {
            return Ok(0.0);
        }

        if bdd == self.one() {
            return Ok(1.0);
        }

        if let Some(result) = self.fraction_cache.get(&bdd) {
            return Ok(*result);
        }

        let node = self.node(bdd)?;
        let result = 0.5 * self.satisfying_fraction_step(node.then_branch)?
            + 0.5 * self.satisfying_fraction_step(node.else_branch)?;
        self.fraction_cache.insert(bdd, result);

        Ok(result)
    }

    fn validate_bdd(&self, bdd: Bdd) -> Result<(), BddSatError> {
        if self.is_constant(bdd) || self.nodes.get(bdd.0).and_then(|node| *node).is_some() {
            Ok(())
        } else {
            Err(BddSatError::UnknownNode(bdd))
        }
    }

    fn validate_index(&self, index: BddIndex) -> Result<(), BddSatError> {
        if index == 0 {
            Err(BddSatError::InvalidVariableIndex(index))
        } else {
            Ok(())
        }
    }

    fn is_constant(&self, bdd: Bdd) -> bool {
        bdd == self.zero() || bdd == self.one()
    }

    fn node(&self, bdd: Bdd) -> Result<BddNode, BddSatError> {
        self.nodes
            .get(bdd.0)
            .and_then(|node| *node)
            .ok_or(BddSatError::UnknownNode(bdd))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satisfy_selects_one_nonzero_path() {
        let mut manager = BddManager::new();
        let y = manager.variable(2).unwrap();
        let formula = manager.find(1, manager.zero(), y).unwrap();

        let cube = manager.satisfy(formula).unwrap();

        assert!(manager.evaluate(cube, [2]).unwrap());
        assert!(manager.evaluate(formula, [2]).unwrap());
        assert!(!manager.evaluate(cube, [1, 2]).unwrap());
        assert!(!manager.evaluate(cube, []).unwrap());
    }

    #[test]
    fn satisfy_rejects_false_function() {
        let mut manager = BddManager::new();

        assert_eq!(
            manager.satisfy(manager.zero()),
            Err(BddSatError::Unsatisfiable)
        );
    }

    #[test]
    fn satisfy_support_adds_missing_support_variables() {
        let mut manager = BddManager::new();
        let x = manager.variable(1).unwrap();

        let cube = manager.satisfy_support(x, [1, 2]).unwrap();

        assert!(manager.evaluate(cube, [1]).unwrap());
        assert!(!manager.evaluate(cube, [1, 2]).unwrap());
        assert!(!manager.evaluate(cube, []).unwrap());
    }

    #[test]
    fn satisfying_fraction_counts_branch_fraction_independent_of_external_support() {
        let mut manager = BddManager::new();
        let y = manager.variable(2).unwrap();
        let x_or_y = manager.find(1, manager.one(), y).unwrap();

        assert_eq!(manager.satisfying_fraction(manager.zero()).unwrap(), 0.0);
        assert_eq!(manager.satisfying_fraction(manager.one()).unwrap(), 1.0);
        assert_eq!(manager.satisfying_fraction(y).unwrap(), 0.5);
        assert_eq!(manager.satisfying_fraction(x_or_y).unwrap(), 0.75);
    }

    #[test]
    fn unique_table_reuses_existing_nodes_and_reduces_equal_branches() {
        let mut manager = BddManager::new();
        let y = manager.variable(2).unwrap();
        let first = manager.find(1, manager.zero(), y).unwrap();
        let second = manager.find(1, manager.zero(), y).unwrap();

        assert_eq!(first, second);
        assert_eq!(manager.find(1, y, y).unwrap(), y);
    }
}
