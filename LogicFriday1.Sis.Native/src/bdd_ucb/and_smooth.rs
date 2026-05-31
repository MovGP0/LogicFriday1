use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddRef(usize);

impl BddRef {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(2);

    pub const fn zero() -> Self {
        Self::ZERO
    }

    pub const fn one() -> Self {
        Self::ONE
    }

    pub fn is_complemented(self) -> bool {
        self.0 & 1 == 1
    }

    pub fn complement(self) -> Self {
        match self {
            Self::ZERO => Self::ONE,
            Self::ONE => Self::ZERO,
            _ => Self(self.0 ^ 1),
        }
    }

    fn node_index(self) -> usize {
        self.0 >> 1
    }

    fn regular(self) -> Self {
        match self {
            Self::ZERO | Self::ONE => self,
            _ => Self(self.0 & !1),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNode {
    variable: usize,
    high: BddRef,
    low: BddRef,
}

impl BddNode {
    pub const fn variable(&self) -> usize {
        self.variable
    }

    pub const fn high(&self) -> BddRef {
        self.high
    }

    pub const fn low(&self) -> BddRef {
        self.low
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AndSmoothReturnStats {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AndSmoothStats {
    pub calls: usize,
    pub returns: AndSmoothReturnStats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteStats {
    pub calls: usize,
    pub cached_returns: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddStats {
    pub and_smooth: AndSmoothStats,
    pub ite: IteStats,
    pub unique_hits: usize,
    pub unique_misses: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AndSmoothError {
    InvalidNode(BddRef),
    InvalidVariableOrder { parent: usize, child: usize },
    RecursionLimitExceeded { limit: usize },
}

impl fmt::Display for AndSmoothError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode(node) => write!(formatter, "invalid BDD node reference {node:?}"),
            Self::InvalidVariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
            Self::RecursionLimitExceeded { limit } => write!(
                formatter,
                "BDD and-smooth recursion limit {limit} was exceeded"
            ),
        }
    }
}

impl Error for AndSmoothError {}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(usize, BddRef, BddRef), BddRef>,
    ite_cache: HashMap<(BddRef, BddRef, BddRef), BddRef>,
    and_smooth_cache: HashMap<(BddRef, BddRef), BddRef>,
    recursion_limit: usize,
    stats: BddStats,
}

impl Default for BddManager {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            unique_table: HashMap::new(),
            ite_cache: HashMap::new(),
            and_smooth_cache: HashMap::new(),
            recursion_limit: 65_536,
            stats: BddStats::default(),
        }
    }
}

impl BddManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn zero(&self) -> BddRef {
        BddRef::ZERO
    }

    pub const fn one(&self) -> BddRef {
        BddRef::ONE
    }

    pub const fn stats(&self) -> BddStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn and_smooth_cache_len(&self) -> usize {
        self.and_smooth_cache.len()
    }

    pub fn set_recursion_limit(&mut self, recursion_limit: usize) {
        self.recursion_limit = recursion_limit;
    }

    pub fn node(&self, id: BddRef) -> Result<Option<&BddNode>, AndSmoothError> {
        if self.is_constant(id) {
            return Ok(None);
        }

        let index = id.regular().node_index().saturating_sub(2);
        self.nodes
            .get(index)
            .map(Some)
            .ok_or(AndSmoothError::InvalidNode(id))
    }

    pub fn variable(&mut self, variable: usize) -> BddRef {
        self.find_or_add_unchecked(variable, self.one(), self.zero())
    }

    pub fn find_or_add(
        &mut self,
        variable: usize,
        high: BddRef,
        low: BddRef,
    ) -> Result<BddRef, AndSmoothError> {
        self.validate_ref(high)?;
        self.validate_ref(low)?;
        self.validate_order(variable, high)?;
        self.validate_order(variable, low)?;

        Ok(self.find_or_add_unchecked(variable, high, low))
    }

    pub fn and(&mut self, left: BddRef, right: BddRef) -> Result<BddRef, AndSmoothError> {
        self.ite(left, right, self.zero())
    }

    pub fn or(&mut self, left: BddRef, right: BddRef) -> Result<BddRef, AndSmoothError> {
        self.ite(left, self.one(), right)
    }

    pub fn and_smooth(
        &mut self,
        left: BddRef,
        right: BddRef,
        smoothing_variables: &[BddRef],
    ) -> Result<BddRef, AndSmoothError> {
        self.validate_ref(left)?;
        self.validate_ref(right)?;

        let variable_ids = self.sorted_variable_ids(smoothing_variables)?;
        self.and_smooth_cache.clear();

        self.and_smooth_inner(left, right, 0, &variable_ids, 0)
    }

    pub fn eval(
        &self,
        root: BddRef,
        assignment: &HashMap<usize, bool>,
    ) -> Result<bool, AndSmoothError> {
        let mut current = root;
        let mut complement = false;

        loop {
            match current {
                BddRef::ZERO => return Ok(complement),
                BddRef::ONE => return Ok(!complement),
                _ => {
                    complement ^= current.is_complemented();
                    let node = self
                        .node(current.regular())?
                        .expect("regular non-constant references always have a node");
                    current = if assignment.get(&node.variable).copied().unwrap_or(false) {
                        node.high
                    } else {
                        node.low
                    };
                }
            }
        }
    }

    fn and_smooth_inner(
        &mut self,
        left: BddRef,
        right: BddRef,
        index: usize,
        variables: &[usize],
        depth: usize,
    ) -> Result<BddRef, AndSmoothError> {
        if depth > self.recursion_limit {
            return Err(AndSmoothError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        self.stats.and_smooth.calls += 1;

        if left == self.zero() || right == self.zero() {
            self.stats.and_smooth.returns.trivial += 1;
            return Ok(self.zero());
        }

        if left == self.one() && right == self.one() {
            self.stats.and_smooth.returns.trivial += 1;
            return Ok(self.one());
        }

        let cache_key = (left, right);
        if let Some(cached) = self.and_smooth_cache.get(&cache_key).copied() {
            self.stats.and_smooth.returns.cached += 1;
            return Ok(cached);
        }

        let left_id = self.variable_id(left.regular())?;
        let right_id = self.variable_id(right.regular())?;
        let (left_high, left_low) = self.branches(left)?;
        let (right_high, right_low) = self.branches(right)?;
        let (variable_id, next_index) = next_variable(left_id.min(right_id), index, variables);

        let result = if left_id > right_id {
            if variable_id == right_id {
                let smoothed_right = self.ite(right_high, self.one(), right_low)?;
                self.and_smooth_inner(left, smoothed_right, next_index, variables, depth + 1)?
            } else {
                let high =
                    self.and_smooth_inner(left, right_high, next_index, variables, depth + 1)?;
                let low =
                    self.and_smooth_inner(left, right_low, next_index, variables, depth + 1)?;
                let variable = self.find_or_add_unchecked(right_id, self.one(), self.zero());

                self.ite(variable, high, low)?
            }
        } else if left_id == right_id {
            let high =
                self.and_smooth_inner(left_high, right_high, next_index, variables, depth + 1)?;
            let low =
                self.and_smooth_inner(left_low, right_low, next_index, variables, depth + 1)?;

            if variable_id == left_id {
                self.ite(high, self.one(), low)?
            } else {
                let variable = self.find_or_add_unchecked(left_id, self.one(), self.zero());

                self.ite(variable, high, low)?
            }
        } else {
            let high = self.and_smooth_inner(left_high, right, next_index, variables, depth + 1)?;
            let low = self.and_smooth_inner(left_low, right, next_index, variables, depth + 1)?;

            if variable_id == left_id {
                self.ite(high, self.one(), low)?
            } else {
                let variable = self.find_or_add_unchecked(left_id, self.one(), self.zero());

                self.ite(variable, high, low)?
            }
        };

        self.and_smooth_cache.insert(cache_key, result);
        self.stats.and_smooth.returns.full += 1;

        Ok(result)
    }

    fn sorted_variable_ids(&self, variables: &[BddRef]) -> Result<Vec<usize>, AndSmoothError> {
        let mut ids = BTreeSet::new();
        for variable in variables {
            self.validate_ref(*variable)?;
            if self.is_constant(*variable) {
                continue;
            }

            ids.insert(self.variable_id(variable.regular())?);
        }

        Ok(ids.into_iter().collect())
    }

    fn branches(&self, value: BddRef) -> Result<(BddRef, BddRef), AndSmoothError> {
        if self.is_constant(value) {
            return Ok((value, value));
        }

        let node = self
            .node(value.regular())?
            .expect("regular non-constant references always have a node");
        if value.is_complemented() {
            Ok((node.high.complement(), node.low.complement()))
        } else {
            Ok((node.high, node.low))
        }
    }

    fn ite(
        &mut self,
        function: BddRef,
        high: BddRef,
        low: BddRef,
    ) -> Result<BddRef, AndSmoothError> {
        self.validate_ref(function)?;
        self.validate_ref(high)?;
        self.validate_ref(low)?;
        self.ite_inner(function, high, low, 0)
    }

    fn ite_inner(
        &mut self,
        function: BddRef,
        high: BddRef,
        low: BddRef,
        depth: usize,
    ) -> Result<BddRef, AndSmoothError> {
        if depth > self.recursion_limit {
            return Err(AndSmoothError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        self.stats.ite.calls += 1;

        if function == self.one() {
            return Ok(high);
        }

        if function == self.zero() {
            return Ok(low);
        }

        if high == low {
            return Ok(high);
        }

        if high == self.one() && low == self.zero() {
            return Ok(function);
        }

        if high == self.zero() && low == self.one() {
            return Ok(function.complement());
        }

        let cache_key = (function, high, low);
        if let Some(cached) = self.ite_cache.get(&cache_key).copied() {
            self.stats.ite.cached_returns += 1;
            return Ok(cached);
        }

        let top_variable = self
            .variable_id(function.regular())?
            .min(self.variable_id(high.regular())?)
            .min(self.variable_id(low.regular())?);
        let (function_high, function_low) = self.quick_cofactor(function, top_variable)?;
        let (high_high, high_low) = self.quick_cofactor(high, top_variable)?;
        let (low_high, low_low) = self.quick_cofactor(low, top_variable)?;

        let then_result = self.ite_inner(function_high, high_high, low_high, depth + 1)?;
        let else_result = self.ite_inner(function_low, high_low, low_low, depth + 1)?;
        let result = if then_result == else_result {
            then_result
        } else {
            self.find_or_add_unchecked(top_variable, then_result, else_result)
        };

        self.ite_cache.insert(cache_key, result);

        Ok(result)
    }

    fn quick_cofactor(
        &self,
        value: BddRef,
        variable: usize,
    ) -> Result<(BddRef, BddRef), AndSmoothError> {
        let Some(node) = self.node(value.regular())? else {
            return Ok((value, value));
        };

        if node.variable != variable {
            return Ok((value, value));
        }

        if value.is_complemented() {
            Ok((node.high.complement(), node.low.complement()))
        } else {
            Ok((node.high, node.low))
        }
    }

    fn find_or_add_unchecked(&mut self, variable: usize, high: BddRef, low: BddRef) -> BddRef {
        if high == low {
            return high;
        }

        let key = (variable, high, low);
        if let Some(existing) = self.unique_table.get(&key).copied() {
            self.stats.unique_hits += 1;
            return existing;
        }

        let node_ref = BddRef((self.nodes.len() + 2) << 1);
        self.nodes.push(BddNode {
            variable,
            high,
            low,
        });
        self.unique_table.insert(key, node_ref);
        self.stats.unique_misses += 1;

        node_ref
    }

    fn validate_ref(&self, value: BddRef) -> Result<(), AndSmoothError> {
        if self.is_constant(value) {
            return Ok(());
        }

        let index = value.regular().node_index().saturating_sub(2);
        if index < self.nodes.len() {
            Ok(())
        } else {
            Err(AndSmoothError::InvalidNode(value))
        }
    }

    fn validate_order(&self, parent: usize, child: BddRef) -> Result<(), AndSmoothError> {
        if self.is_constant(child) {
            return Ok(());
        }

        let child_variable = self.variable_id(child.regular())?;
        if parent < child_variable {
            Ok(())
        } else {
            Err(AndSmoothError::InvalidVariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn variable_id(&self, value: BddRef) -> Result<usize, AndSmoothError> {
        if self.is_constant(value) {
            return Ok(usize::MAX);
        }

        Ok(self
            .node(value.regular())?
            .expect("non-constant references always have a node")
            .variable)
    }

    fn is_constant(&self, value: BddRef) -> bool {
        matches!(value, BddRef::ZERO | BddRef::ONE)
    }
}

fn next_variable(top_variable: usize, index: usize, variables: &[usize]) -> (usize, usize) {
    for (position, value) in variables.iter().copied().enumerate().skip(index) {
        if value >= top_variable {
            if value == top_variable {
                return (value, position + 1);
            }

            return (value, position);
        }
    }

    (usize::MAX, variables.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(entries: &[(usize, bool)]) -> HashMap<usize, bool> {
        entries.iter().copied().collect()
    }

    #[test]
    fn and_smooth_exists_quantifies_shared_variable_after_and() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.and_smooth(x, y, &[x]).unwrap();

        assert_eq!(result, y);
        assert!(
            !manager
                .eval(result, &values(&[(0, false), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .eval(result, &values(&[(0, false), (1, true)]))
                .unwrap()
        );
    }

    #[test]
    fn smoothing_right_only_variable_matches_or_of_right_branches() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let z = manager.variable(2);
        let right = manager.or(y, z).unwrap();

        let result = manager.and_smooth(x, right, &[z]).unwrap();

        assert_eq!(result, x);
    }

    #[test]
    fn smoothing_left_variable_combines_high_and_low_results() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let function = manager.or(x, y).unwrap();

        let result = manager.and_smooth(function, y, &[x]).unwrap();

        assert_eq!(result, y);
    }

    #[test]
    fn smoothing_all_variables_in_conjunction_returns_one() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.and_smooth(x, y, &[y, x, x]).unwrap();

        assert_eq!(result, manager.one());
    }

    #[test]
    fn no_smoothing_variables_rebuilds_plain_and() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.and_smooth(x, y, &[]).unwrap();
        let expected = manager.and(x, y).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn complemented_operands_are_supported() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.and_smooth(x.complement(), y, &[x]).unwrap();

        assert_eq!(result, y);
    }

    #[test]
    fn cache_is_used_for_shared_subproblems() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let z = manager.variable(2);
        let shared = manager.or(y, z).unwrap();
        let function = manager.and(x, shared).unwrap();

        let result = manager.and_smooth(function, shared, &[z]).unwrap();
        let cached_before = manager.stats().and_smooth.returns.cached;
        let second = manager
            .and_smooth_inner(function, shared, 0, &[2], 0)
            .unwrap();

        assert!(manager.validate_ref(result).is_ok());
        assert_eq!(second, result);
        assert!(manager.and_smooth_cache_len() > 0);
        assert!(manager.stats().and_smooth.returns.cached > cached_before);
        assert_eq!(x, manager.variable(0));
    }

    #[test]
    fn invalid_references_are_rejected() {
        let mut manager = BddManager::new();

        let error = manager
            .and_smooth(BddRef(100), manager.one(), &[])
            .expect_err("invalid root must be rejected");

        assert_eq!(error, AndSmoothError::InvalidNode(BddRef(100)));
    }

    #[test]
    fn recursion_limit_is_enforced() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        manager.set_recursion_limit(0);

        let error = manager.and_smooth(x, y, &[x]).unwrap_err();

        assert_eq!(error, AndSmoothError::RecursionLimitExceeded { limit: 0 });
    }

    #[test]
    fn next_variable_matches_legacy_index_rules() {
        assert_eq!(next_variable(3, 0, &[1, 3, 5]), (3, 2));
        assert_eq!(next_variable(4, 0, &[1, 3, 5]), (5, 2));
        assert_eq!(next_variable(6, 0, &[1, 3, 5]), (usize::MAX, 3));
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("and_smooth.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
