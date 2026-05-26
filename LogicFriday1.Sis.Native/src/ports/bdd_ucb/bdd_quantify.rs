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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QuantifyType {
    Exists,
    ForAll,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QuantifyReturnStats {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QuantifyStats {
    pub calls: usize,
    pub returns: QuantifyReturnStats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteStats {
    pub calls: usize,
    pub cached_returns: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddStats {
    pub quantify: QuantifyStats,
    pub ite: IteStats,
    pub unique_hits: usize,
    pub unique_misses: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddQuantifyError {
    InvalidNode(BddRef),
    InvalidVariableOrder { parent: usize, child: usize },
    RecursionLimitExceeded { limit: usize },
}

impl fmt::Display for BddQuantifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode(node) => write!(formatter, "invalid BDD node reference {node:?}"),
            Self::InvalidVariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
            Self::RecursionLimitExceeded { limit } => write!(
                formatter,
                "BDD quantification recursion limit {limit} was exceeded"
            ),
        }
    }
}

impl Error for BddQuantifyError {}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(usize, BddRef, BddRef), BddRef>,
    ite_cache: HashMap<(BddRef, BddRef, BddRef), BddRef>,
    quantify_cache: HashMap<(BddRef, usize, QuantifyType), BddRef>,
    recursion_limit: usize,
    stats: BddStats,
}

impl Default for BddManager {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            unique_table: HashMap::new(),
            ite_cache: HashMap::new(),
            quantify_cache: HashMap::new(),
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

    pub fn quantify_cache_len(&self) -> usize {
        self.quantify_cache.len()
    }

    pub fn set_recursion_limit(&mut self, recursion_limit: usize) {
        self.recursion_limit = recursion_limit;
    }

    pub fn node(&self, id: BddRef) -> Result<Option<&BddNode>, BddQuantifyError> {
        if self.is_constant(id) {
            return Ok(None);
        }

        let index = id.regular().node_index().saturating_sub(2);
        self.nodes
            .get(index)
            .map(Some)
            .ok_or(BddQuantifyError::InvalidNode(id))
    }

    pub fn variable(&mut self, variable: usize) -> BddRef {
        self.find_or_add_unchecked(variable, self.one(), self.zero())
    }

    pub fn find_or_add(
        &mut self,
        variable: usize,
        high: BddRef,
        low: BddRef,
    ) -> Result<BddRef, BddQuantifyError> {
        self.validate_ref(high)?;
        self.validate_ref(low)?;
        self.validate_order(variable, high)?;
        self.validate_order(variable, low)?;

        Ok(self.find_or_add_unchecked(variable, high, low))
    }

    pub fn smooth(
        &mut self,
        function: BddRef,
        variables: &[BddRef],
    ) -> Result<BddRef, BddQuantifyError> {
        self.quantify(function, variables, QuantifyType::Exists)
    }

    pub fn consensus(
        &mut self,
        function: BddRef,
        variables: &[BddRef],
    ) -> Result<BddRef, BddQuantifyError> {
        self.quantify(function, variables, QuantifyType::ForAll)
    }

    pub fn quantify(
        &mut self,
        function: BddRef,
        variables: &[BddRef],
        quant_type: QuantifyType,
    ) -> Result<BddRef, BddQuantifyError> {
        self.validate_ref(function)?;

        let variable_ids = self.sorted_variable_ids(variables)?;
        self.quantify_cache.clear();
        self.internal_quantify(function, 0, &variable_ids, quant_type, 0)
    }

    pub fn eval(
        &self,
        root: BddRef,
        assignment: &HashMap<usize, bool>,
    ) -> Result<bool, BddQuantifyError> {
        let mut current = root;
        let mut complement = false;

        loop {
            match current {
                BddRef::ZERO => return Ok(complement),
                BddRef::ONE => return Ok(!complement),
                _ => {
                    complement ^= current.is_complemented();
                    let regular = current.regular();
                    let node = self
                        .node(regular)?
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

    fn internal_quantify(
        &mut self,
        function: BddRef,
        index: usize,
        variables: &[usize],
        quant_type: QuantifyType,
        depth: usize,
    ) -> Result<BddRef, BddQuantifyError> {
        if depth > self.recursion_limit {
            return Err(BddQuantifyError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        self.stats.quantify.calls += 1;

        if index >= variables.len() {
            self.stats.quantify.returns.trivial += 1;
            return Ok(function);
        }

        let function_id = self.variable_id(function.regular())?;
        let last_variable = variables[variables.len() - 1];
        if function_id > last_variable {
            self.stats.quantify.returns.trivial += 1;
            return Ok(function);
        }

        let cache_key = (function, index, quant_type);
        if let Some(cached) = self.quantify_cache.get(&cache_key).copied() {
            self.stats.quantify.returns.cached += 1;
            return Ok(cached);
        }

        let top_variable = variables[index];
        let (high, low) = self.branches(function)?;
        let result = if function_id > top_variable {
            self.internal_quantify(function, index + 1, variables, quant_type, depth + 1)?
        } else if function_id == top_variable {
            let quantified_high =
                self.internal_quantify(high, index + 1, variables, quant_type, depth + 1)?;
            let quantified_low =
                self.internal_quantify(low, index + 1, variables, quant_type, depth + 1)?;

            match quant_type {
                QuantifyType::Exists => self.or(quantified_high, quantified_low)?,
                QuantifyType::ForAll => self.and(quantified_high, quantified_low)?,
            }
        } else {
            let quantified_high =
                self.internal_quantify(high, index, variables, quant_type, depth + 1)?;
            let quantified_low =
                self.internal_quantify(low, index, variables, quant_type, depth + 1)?;

            if quantified_high == quantified_low {
                quantified_high
            } else {
                self.find_or_add_unchecked(function_id, quantified_high, quantified_low)
            }
        };

        self.quantify_cache.insert(cache_key, result);
        self.stats.quantify.returns.full += 1;
        Ok(result)
    }

    fn sorted_variable_ids(&self, variables: &[BddRef]) -> Result<Vec<usize>, BddQuantifyError> {
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

    fn branches(&self, value: BddRef) -> Result<(BddRef, BddRef), BddQuantifyError> {
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

    fn and(&mut self, left: BddRef, right: BddRef) -> Result<BddRef, BddQuantifyError> {
        self.ite(left, right, self.zero())
    }

    fn or(&mut self, left: BddRef, right: BddRef) -> Result<BddRef, BddQuantifyError> {
        self.ite(left, self.one(), right)
    }

    fn ite(
        &mut self,
        function: BddRef,
        high: BddRef,
        low: BddRef,
    ) -> Result<BddRef, BddQuantifyError> {
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
    ) -> Result<BddRef, BddQuantifyError> {
        if depth > self.recursion_limit {
            return Err(BddQuantifyError::RecursionLimitExceeded {
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
    ) -> Result<(BddRef, BddRef), BddQuantifyError> {
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

    fn validate_ref(&self, value: BddRef) -> Result<(), BddQuantifyError> {
        if self.is_constant(value) {
            return Ok(());
        }

        let index = value.regular().node_index().saturating_sub(2);
        if index < self.nodes.len() {
            Ok(())
        } else {
            Err(BddQuantifyError::InvalidNode(value))
        }
    }

    fn validate_order(&self, parent: usize, child: BddRef) -> Result<(), BddQuantifyError> {
        if self.is_constant(child) {
            return Ok(());
        }

        let child_variable = self.variable_id(child.regular())?;
        if parent < child_variable {
            Ok(())
        } else {
            Err(BddQuantifyError::InvalidVariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn variable_id(&self, value: BddRef) -> Result<usize, BddQuantifyError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn values(entries: &[(usize, bool)]) -> HashMap<usize, bool> {
        entries.iter().copied().collect()
    }

    fn sample_function(manager: &mut BddManager) -> (BddRef, BddRef, BddRef, BddRef) {
        let x = manager.variable(0);
        let y = manager.variable(1);
        let z = manager.variable(2);
        let y_and_z = manager.and(y, z).unwrap();
        let function = manager.or(x, y_and_z).unwrap();

        (function, x, y, z)
    }

    #[test]
    fn smooth_exists_quantifies_single_variable() {
        let mut manager = BddManager::new();
        let (function, x, y, z) = sample_function(&mut manager);

        let result = manager.smooth(function, &[x]).unwrap();

        for x_value in [false, true] {
            for y_value in [false, true] {
                for z_value in [false, true] {
                    assert_eq!(
                        manager
                            .eval(result, &values(&[(0, x_value), (1, y_value), (2, z_value)]))
                            .unwrap(),
                        true
                    );
                }
            }
        }
        assert_eq!(
            manager.smooth(function, &[z]).unwrap(),
            manager.or(x, y).unwrap()
        );
    }

    #[test]
    fn consensus_forall_quantifies_single_variable() {
        let mut manager = BddManager::new();
        let (function, x, y, z) = sample_function(&mut manager);

        let result = manager.consensus(function, &[x]).unwrap();

        assert_eq!(result, manager.and(y, z).unwrap());
        assert!(
            manager
                .eval(result, &values(&[(0, false), (1, true), (2, true)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result, &values(&[(0, true), (1, true), (2, false)]))
                .unwrap()
        );
    }

    #[test]
    fn quantify_sorts_and_deduplicates_variable_array() {
        let mut manager = BddManager::new();
        let (function, x, y, z) = sample_function(&mut manager);

        let result = manager.smooth(function, &[z, x, z]).unwrap();

        assert_eq!(result, manager.one());
        assert!(manager.consensus(function, &[z, y]).unwrap() == x);
    }

    #[test]
    fn unmentioned_variables_are_rebuilt() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let z = manager.variable(2);
        let xy = manager.and(x, y).unwrap();
        let function = manager.or(xy, z).unwrap();

        let result = manager.smooth(function, &[y]).unwrap();

        assert_eq!(result, manager.or(x, z).unwrap());
    }

    #[test]
    fn constants_and_late_variables_return_trivially() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);

        assert_eq!(manager.smooth(manager.one(), &[x]).unwrap(), manager.one());
        assert_eq!(
            manager.consensus(manager.zero(), &[x]).unwrap(),
            manager.zero()
        );
        assert_eq!(manager.smooth(x, &[]).unwrap(), x);
    }

    #[test]
    fn cache_is_used_for_shared_recursive_quantification() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let z = manager.variable(2);
        let shared = manager
            .find_or_add(2, manager.one(), manager.zero())
            .unwrap();
        let high_parent = manager.find_or_add(1, shared, manager.zero()).unwrap();
        let low_parent = manager.find_or_add(1, shared, manager.one()).unwrap();
        let function = manager.find_or_add(0, high_parent, low_parent).unwrap();

        let result = manager.smooth(function, &[z]).unwrap();

        assert!(manager.validate_ref(result).is_ok());
        assert!(manager.quantify_cache_len() > 0);
        assert!(manager.stats().quantify.returns.cached > 0);
        assert_eq!(x, manager.variable(0));
    }

    #[test]
    fn invalid_references_are_rejected() {
        let mut manager = BddManager::new();

        let error = manager
            .smooth(BddRef(100), &[manager.one()])
            .expect_err("invalid root must be rejected");

        assert_eq!(error, BddQuantifyError::InvalidNode(BddRef(100)));
    }

    #[test]
    fn recursion_limit_is_enforced() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let function = manager.and(x, y).unwrap();
        manager.set_recursion_limit(0);

        let error = manager.smooth(function, &[x, y]).unwrap_err();

        assert_eq!(error, BddQuantifyError::RecursionLimitExceeded { limit: 0 });
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bdd_quantify.rs");

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
