use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddRef(usize);

impl BddRef {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(2);

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
    pub fn variable(&self) -> usize {
        self.variable
    }

    pub fn high(&self) -> BddRef {
        self.high
    }

    pub fn low(&self) -> BddRef {
        self.low
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteReturnStats {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteStats {
    pub calls: usize,
    pub returns: IteReturnStats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UniqueTableStats {
    pub hits: usize,
    pub misses: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteCacheStats {
    pub hits: usize,
    pub misses: usize,
    pub inserts: usize,
    pub collisions: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddStats {
    pub ite: IteStats,
    pub unique_table: UniqueTableStats,
    pub ite_cache: IteCacheStats,
}

#[derive(Clone, Debug)]
pub struct IteCacheConfig {
    pub enabled: bool,
}

impl Default for IteCacheConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BddError {
    InvalidNode(BddRef),
    InvalidVariableOrder { parent: usize, child: usize },
    RecursionLimitExceeded { limit: usize },
}

impl fmt::Display for BddError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode(node) => write!(f, "invalid BDD node reference {node:?}"),
            Self::InvalidVariableOrder { parent, child } => write!(
                f,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
            Self::RecursionLimitExceeded { limit } => {
                write!(f, "BDD ITE recursion limit {limit} was exceeded")
            }
        }
    }
}

impl std::error::Error for BddError {}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(usize, BddRef, BddRef), BddRef>,
    ite_cache: HashMap<(BddRef, BddRef, BddRef), BddRef>,
    cache_config: IteCacheConfig,
    recursion_limit: usize,
    stats: BddStats,
}

impl Default for BddManager {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            unique_table: HashMap::new(),
            ite_cache: HashMap::new(),
            cache_config: IteCacheConfig::default(),
            recursion_limit: 65_536,
            stats: BddStats::default(),
        }
    }
}

impl BddManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn zero(&self) -> BddRef {
        BddRef::ZERO
    }

    pub fn one(&self) -> BddRef {
        BddRef::ONE
    }

    pub fn stats(&self) -> BddStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn cache_len(&self) -> usize {
        self.ite_cache.len()
    }

    pub fn set_cache_enabled(&mut self, enabled: bool) {
        self.cache_config.enabled = enabled;
        if !enabled {
            self.ite_cache.clear();
        }
    }

    pub fn set_recursion_limit(&mut self, recursion_limit: usize) {
        self.recursion_limit = recursion_limit;
    }

    pub fn node(&self, id: BddRef) -> Result<Option<&BddNode>, BddError> {
        if self.is_constant(id) {
            return Ok(None);
        }

        let index = id.regular().node_index().saturating_sub(2);
        self.nodes
            .get(index)
            .map(Some)
            .ok_or(BddError::InvalidNode(id))
    }

    pub fn variable(&mut self, variable: usize) -> BddRef {
        self.find_or_add_unchecked(variable, self.one(), self.zero())
    }

    pub fn find_or_add(
        &mut self,
        variable: usize,
        high: BddRef,
        low: BddRef,
    ) -> Result<BddRef, BddError> {
        self.validate_ref(high)?;
        self.validate_ref(low)?;
        self.validate_order(variable, high)?;
        self.validate_order(variable, low)?;
        Ok(self.find_or_add_unchecked(variable, high, low))
    }

    pub fn ite(&mut self, f: BddRef, g: BddRef, h: BddRef) -> Result<BddRef, BddError> {
        self.validate_ref(f)?;
        self.validate_ref(g)?;
        self.validate_ref(h)?;
        self.ite_inner(f, g, h, 0)
    }

    pub fn not(&self, value: BddRef) -> BddRef {
        value.complement()
    }

    pub fn and(&mut self, left: BddRef, right: BddRef) -> Result<BddRef, BddError> {
        self.ite(left, right, self.zero())
    }

    pub fn or(&mut self, left: BddRef, right: BddRef) -> Result<BddRef, BddError> {
        self.ite(left, self.one(), right)
    }

    pub fn eval(&self, root: BddRef, assignment: &HashMap<usize, bool>) -> Result<bool, BddError> {
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

    fn ite_inner(
        &mut self,
        mut f: BddRef,
        mut g: BddRef,
        mut h: BddRef,
        depth: usize,
    ) -> Result<BddRef, BddError> {
        if depth > self.recursion_limit {
            return Err(BddError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        self.stats.ite.calls += 1;

        if f == self.one() {
            self.stats.ite.returns.trivial += 1;
            return Ok(g);
        }

        if f == self.zero() {
            self.stats.ite.returns.trivial += 1;
            return Ok(h);
        }

        self.ite_var_to_const(f, &mut g, &mut h);

        if g == h {
            self.stats.ite.returns.trivial += 1;
            return Ok(g);
        }

        if g == self.one() && h == self.zero() {
            self.stats.ite.returns.trivial += 1;
            return Ok(f);
        }

        if g == self.zero() && h == self.one() {
            self.stats.ite.returns.trivial += 1;
            return Ok(f.complement());
        }

        let complement = self.canonicalize_ite_inputs(&mut f, &mut g, &mut h);
        let reg_f = f.regular();
        let reg_g = g.regular();
        let reg_h = h.regular();

        let g_id = self.variable_id(reg_g)?;
        let h_id = self.variable_id(reg_h)?;
        let var = g_id.min(h_id);

        if self.variable_id(reg_f)? < var {
            let f_node = self
                .node(reg_f)?
                .expect("non-constant f with a variable id has a node");

            if f_node.high == self.one() && f_node.low == self.zero() {
                let variable = f_node.variable;
                self.stats.ite.returns.trivial += 1;
                let ret = self.find_or_add_unchecked(variable, g, h);
                return Ok(if complement { ret.complement() } else { ret });
            }
        }

        let cache_key = (f, g, h);
        if self.cache_config.enabled {
            if let Some(cached) = self.ite_cache.get(&cache_key).copied() {
                self.stats.ite_cache.hits += 1;
                self.stats.ite.returns.cached += 1;
                return Ok(if complement {
                    cached.complement()
                } else {
                    cached
                });
            }

            self.stats.ite_cache.misses += 1;
        }

        let var = self
            .variable_id(reg_f)?
            .min(self.variable_id(reg_g)?)
            .min(self.variable_id(reg_h)?);
        let (f_high, f_low) = self.quick_cofactor(f, var)?;
        let (g_high, g_low) = self.quick_cofactor(g, var)?;
        let (h_high, h_low) = self.quick_cofactor(h, var)?;

        let then_result = self.ite_inner(f_high, g_high, h_high, depth + 1)?;
        let else_result = self.ite_inner(f_low, g_low, h_low, depth + 1)?;

        let ret = if then_result == else_result {
            then_result
        } else {
            self.find_or_add_unchecked(var, then_result, else_result)
        };

        if self.cache_config.enabled {
            if self.ite_cache.insert(cache_key, ret).is_some() {
                self.stats.ite_cache.collisions += 1;
            }

            self.stats.ite_cache.inserts += 1;
        }

        self.stats.ite.returns.full += 1;
        Ok(if complement { ret.complement() } else { ret })
    }

    fn ite_var_to_const(&self, f: BddRef, g: &mut BddRef, h: &mut BddRef) {
        if f == *g {
            *g = self.one();
        } else if f == g.complement() {
            *g = self.zero();
        }

        if f == *h {
            *h = self.zero();
        } else if f == h.complement() {
            *h = self.one();
        }
    }

    fn canonicalize_ite_inputs(&self, f: &mut BddRef, g: &mut BddRef, h: &mut BddRef) -> bool {
        if self.is_constant(*g) {
            if self.greater_or_equal(*f, *h) {
                if *g == self.one() {
                    std::mem::swap(h, f);
                } else {
                    std::mem::swap(h, f);
                    *f = f.complement();
                    *h = h.complement();
                }
            }
        } else if self.is_constant(*h) {
            if self.greater_or_equal(*f, *g) {
                if *h == self.one() {
                    std::mem::swap(g, f);
                    *f = f.complement();
                    *g = g.complement();
                } else {
                    std::mem::swap(g, f);
                }
            }
        } else if *g == h.complement() && self.greater_or_equal(*f, *g) {
            std::mem::swap(f, g);
            *h = g.complement();
        }

        if f.is_complemented() {
            *f = f.complement();
            std::mem::swap(g, h);
        }

        if !g.is_complemented() {
            false
        } else {
            *g = g.complement();
            *h = h.complement();
            true
        }
    }

    fn quick_cofactor(&self, value: BddRef, variable: usize) -> Result<(BddRef, BddRef), BddError> {
        let regular = value.regular();
        let Some(node) = self.node(regular)? else {
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
            self.stats.unique_table.hits += 1;
            return existing;
        }

        let node_ref = BddRef((self.nodes.len() + 2) << 1);
        self.nodes.push(BddNode {
            variable,
            high,
            low,
        });
        self.unique_table.insert(key, node_ref);
        self.stats.unique_table.misses += 1;
        node_ref
    }

    fn validate_ref(&self, value: BddRef) -> Result<(), BddError> {
        if self.is_constant(value) {
            return Ok(());
        }

        let index = value.regular().node_index().saturating_sub(2);
        if index < self.nodes.len() {
            Ok(())
        } else {
            Err(BddError::InvalidNode(value))
        }
    }

    fn validate_order(&self, parent: usize, child: BddRef) -> Result<(), BddError> {
        if self.is_constant(child) {
            return Ok(());
        }

        let child_var = self.variable_id(child.regular())?;
        if parent < child_var {
            Ok(())
        } else {
            Err(BddError::InvalidVariableOrder {
                parent,
                child: child_var,
            })
        }
    }

    fn variable_id(&self, value: BddRef) -> Result<usize, BddError> {
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

    fn greater_or_equal(&self, left: BddRef, right: BddRef) -> bool {
        let left_id = self.variable_id(left.regular()).unwrap_or(usize::MAX);
        let right_id = self.variable_id(right.regular()).unwrap_or(usize::MAX);

        left_id > right_id || left_id == right_id && left.0 >= right.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(entries: &[(usize, bool)]) -> HashMap<usize, bool> {
        entries.iter().copied().collect()
    }

    #[test]
    fn returns_then_branch_when_condition_is_one() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.ite(manager.one(), x, y).unwrap();

        assert_eq!(result, x);
        assert_eq!(manager.stats().ite.returns.trivial, 1);
    }

    #[test]
    fn returns_else_branch_when_condition_is_zero() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.ite(manager.zero(), x, y).unwrap();

        assert_eq!(result, y);
        assert_eq!(manager.stats().ite.returns.trivial, 1);
    }

    #[test]
    fn returns_condition_for_boolean_identity() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);

        let result = manager.ite(x, manager.one(), manager.zero()).unwrap();

        assert_eq!(result, x);
    }

    #[test]
    fn builds_canonical_and_expression() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.and(x, y).unwrap();

        assert!(
            !manager
                .eval(result, &values(&[(0, false), (1, false)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result, &values(&[(0, false), (1, true)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result, &values(&[(0, true), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .eval(result, &values(&[(0, true), (1, true)]))
                .unwrap()
        );
        assert_eq!(manager.node_count(), 3);
    }

    #[test]
    fn builds_canonical_or_expression_with_complemented_result() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.or(x, y).unwrap();

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
        assert!(
            manager
                .eval(result, &values(&[(0, true), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .eval(result, &values(&[(0, true), (1, true)]))
                .unwrap()
        );
    }

    #[test]
    fn canonicalizes_complemented_condition() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let direct = manager.ite(x.complement(), manager.one(), y).unwrap();
        let rewritten = manager.ite(x, y, manager.one()).unwrap();

        assert_eq!(direct, rewritten);
    }

    #[test]
    fn uses_cache_for_repeated_recursive_operation() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let z = manager.variable(2);

        let first = manager.ite(y, x, z).unwrap();
        let cached_before = manager.stats().ite.returns.cached;
        let second = manager.ite(y, x, z).unwrap();

        assert_eq!(first, second);
        assert!(manager.stats().ite.returns.cached > cached_before);
        assert!(!manager.ite_cache.is_empty());
    }

    #[test]
    fn validates_public_find_or_add_order() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);

        let error = manager.find_or_add(0, x, manager.zero()).unwrap_err();

        assert_eq!(
            error,
            BddError::InvalidVariableOrder {
                parent: 0,
                child: 0,
            }
        );
    }

    #[test]
    fn rejects_invalid_references() {
        let mut manager = BddManager::new();

        let error = manager
            .ite(BddRef(100), manager.one(), manager.zero())
            .unwrap_err();

        assert_eq!(error, BddError::InvalidNode(BddRef(100)));
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("ite.rs");
        let legacy_export = concat!("no", "_", "mangle");
        let bead_token = concat!("bead", "_", "id");
        let bead_prefix = concat!("LogicFriday1", "-", "8j8");
        let required_prefix = concat!("REQUIRED", "_");

        assert!(!source.contains(legacy_export));
        assert!(!source.contains("extern \"C\""));
        assert!(!source.contains(required_prefix));
        assert!(!source.contains(bead_token));
        assert!(!source.contains(bead_prefix));
    }
}
