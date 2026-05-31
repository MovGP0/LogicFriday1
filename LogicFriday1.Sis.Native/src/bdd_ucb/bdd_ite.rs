use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_MANAGER_ID: AtomicUsize = AtomicUsize::new(1);

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
        (self.regular().0 >> 1).saturating_sub(2)
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalBdd {
    manager_id: usize,
    root: BddRef,
    external_index: usize,
    free: bool,
    origin: &'static str,
}

impl ExternalBdd {
    pub fn root(&self) -> BddRef {
        self.root
    }

    pub fn external_index(&self) -> usize {
        self.external_index
    }

    pub fn origin(&self) -> &'static str {
        self.origin
    }

    pub fn is_free(&self) -> bool {
        self.free
    }

    pub fn mark_free(&mut self) {
        self.free = true;
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteReturnStats {
    pub trivial: usize,
    pub computed: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddIteStats {
    pub calls: usize,
    pub unique_hits: usize,
    pub unique_misses: usize,
    pub returns: IteReturnStats,
}

#[derive(Clone, Debug)]
pub struct BddIteManager {
    id: usize,
    nodes: Vec<BddNode>,
    unique_table: HashMap<(usize, BddRef, BddRef), BddRef>,
    next_external_index: usize,
    stats: BddIteStats,
    flight_recorder: Vec<String>,
}

impl Default for BddIteManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BddIteManager {
    pub fn new() -> Self {
        Self {
            id: NEXT_MANAGER_ID.fetch_add(1, Ordering::Relaxed),
            nodes: Vec::new(),
            unique_table: HashMap::new(),
            next_external_index: 0,
            stats: BddIteStats::default(),
            flight_recorder: Vec::new(),
        }
    }

    pub fn stats(&self) -> BddIteStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn node(&self, value: BddRef) -> Result<Option<&BddNode>, BddIteError> {
        if self.is_constant(value) {
            return Ok(None);
        }

        self.nodes
            .get(value.node_index())
            .map(Some)
            .ok_or(BddIteError::InvalidNode)
    }

    pub fn flight_recorder(&self) -> &[String] {
        &self.flight_recorder
    }

    pub fn zero(&mut self) -> ExternalBdd {
        self.make_external_pointer(BddRef::ZERO, "bdd_zero")
    }

    pub fn one(&mut self) -> ExternalBdd {
        self.make_external_pointer(BddRef::ONE, "bdd_one")
    }

    pub fn variable(&mut self, variable: usize) -> ExternalBdd {
        let root = self.find_or_add(variable, BddRef::ONE, BddRef::ZERO);

        self.make_external_pointer(root, "bdd_variable")
    }

    pub fn bdd_ite(
        &mut self,
        f: &ExternalBdd,
        g: &ExternalBdd,
        h: &ExternalBdd,
        f_phase: bool,
        g_phase: bool,
        h_phase: bool,
    ) -> Result<ExternalBdd, BddIteError> {
        self.validate_operand(f)?;
        self.validate_operand(g)?;
        self.validate_operand(h)?;
        self.validate_ref(f.root)?;
        self.validate_ref(g.root)?;
        self.validate_ref(h.root)?;

        let real_f = apply_phase(f.root, f_phase);
        let real_g = apply_phase(g.root, g_phase);
        let real_h = apply_phase(h.root, h_phase);
        let root = self.ite_inner(real_f, real_g, real_h)?;
        let result = self.make_external_pointer(root, "bdd_ite");

        self.flight_recorder.push(format!(
            "{} <- bdd_ite({}, {}, {}, {}, {}, {})",
            result.external_index,
            f.external_index,
            g.external_index,
            h.external_index,
            f_phase,
            g_phase,
            h_phase
        ));

        Ok(result)
    }

    pub fn evaluate(
        &self,
        value: &ExternalBdd,
        assignment: &HashMap<usize, bool>,
    ) -> Result<bool, BddIteError> {
        self.validate_operand(value)?;
        let mut root = value.root;
        let mut complemented = false;

        loop {
            match root {
                BddRef::ZERO => return Ok(complemented),
                BddRef::ONE => return Ok(!complemented),
                _ => {
                    complemented ^= root.is_complemented();
                    let node = self.node(root.regular())?.ok_or(BddIteError::InvalidNode)?;
                    root = if assignment.get(&node.variable).copied().unwrap_or(false) {
                        node.high
                    } else {
                        node.low
                    };
                }
            }
        }
    }

    fn make_external_pointer(&mut self, root: BddRef, origin: &'static str) -> ExternalBdd {
        let external = ExternalBdd {
            manager_id: self.id,
            root,
            external_index: self.next_external_index,
            free: false,
            origin,
        };
        self.next_external_index += 1;

        external
    }

    fn validate_operand(&self, value: &ExternalBdd) -> Result<(), BddIteError> {
        if value.free {
            return Err(BddIteError::InvalidBdd);
        }

        if value.manager_id != self.id {
            return Err(BddIteError::DifferentManagers);
        }

        Ok(())
    }

    fn validate_ref(&self, value: BddRef) -> Result<(), BddIteError> {
        if self.is_constant(value) {
            return Ok(());
        }

        if value.node_index() < self.nodes.len() {
            Ok(())
        } else {
            Err(BddIteError::InvalidNode)
        }
    }

    fn ite_inner(
        &mut self,
        f: BddRef,
        mut g: BddRef,
        mut h: BddRef,
    ) -> Result<BddRef, BddIteError> {
        self.stats.calls += 1;

        if f == BddRef::ONE {
            self.stats.returns.trivial += 1;
            return Ok(g);
        }

        if f == BddRef::ZERO {
            self.stats.returns.trivial += 1;
            return Ok(h);
        }

        self.ite_var_to_const(f, &mut g, &mut h);

        if g == h {
            self.stats.returns.trivial += 1;
            return Ok(g);
        }

        if g == BddRef::ONE && h == BddRef::ZERO {
            self.stats.returns.trivial += 1;
            return Ok(f);
        }

        if g == BddRef::ZERO && h == BddRef::ONE {
            self.stats.returns.trivial += 1;
            return Ok(f.complement());
        }

        let top_variable = self
            .variable_id(f)?
            .min(self.variable_id(g)?)
            .min(self.variable_id(h)?);
        let (f_high, f_low) = self.quick_cofactor(f, top_variable)?;
        let (g_high, g_low) = self.quick_cofactor(g, top_variable)?;
        let (h_high, h_low) = self.quick_cofactor(h, top_variable)?;
        let high = self.ite_inner(f_high, g_high, h_high)?;
        let low = self.ite_inner(f_low, g_low, h_low)?;
        let result = self.find_or_add(top_variable, high, low);

        self.stats.returns.computed += 1;
        Ok(result)
    }

    fn ite_var_to_const(&self, f: BddRef, g: &mut BddRef, h: &mut BddRef) {
        if f == *g {
            *g = BddRef::ONE;
        } else if f == g.complement() {
            *g = BddRef::ZERO;
        }

        if f == *h {
            *h = BddRef::ZERO;
        } else if f == h.complement() {
            *h = BddRef::ONE;
        }
    }

    fn quick_cofactor(
        &self,
        value: BddRef,
        variable: usize,
    ) -> Result<(BddRef, BddRef), BddIteError> {
        if self.is_constant(value) {
            return Ok((value, value));
        }

        let node = self
            .node(value.regular())?
            .ok_or(BddIteError::InvalidNode)?;
        if node.variable != variable {
            return Ok((value, value));
        }

        if value.is_complemented() {
            Ok((node.high.complement(), node.low.complement()))
        } else {
            Ok((node.high, node.low))
        }
    }

    fn find_or_add(&mut self, variable: usize, high: BddRef, low: BddRef) -> BddRef {
        if high == low {
            return high;
        }

        let key = (variable, high, low);
        if let Some(existing) = self.unique_table.get(&key).copied() {
            self.stats.unique_hits += 1;
            return existing;
        }

        let root = BddRef((self.nodes.len() + 2) << 1);
        self.nodes.push(BddNode {
            variable,
            high,
            low,
        });
        self.unique_table.insert(key, root);
        self.stats.unique_misses += 1;

        root
    }

    fn variable_id(&self, value: BddRef) -> Result<usize, BddIteError> {
        if self.is_constant(value) {
            return Ok(usize::MAX);
        }

        let node = self
            .node(value.regular())?
            .ok_or(BddIteError::InvalidNode)?;

        Ok(node.variable)
    }

    fn is_constant(&self, value: BddRef) -> bool {
        matches!(value, BddRef::ZERO | BddRef::ONE)
    }
}

fn apply_phase(value: BddRef, phase: bool) -> BddRef {
    if phase { value } else { value.complement() }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddIteError {
    InvalidBdd,
    DifferentManagers,
    InvalidNode,
}

impl fmt::Display for BddIteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBdd => write!(formatter, "bdd_ite: invalid BDD"),
            Self::DifferentManagers => write!(formatter, "bdd_ite: different bdd managers"),
            Self::InvalidNode => write!(formatter, "bdd_ite: invalid BDD node"),
        }
    }
}

impl Error for BddIteError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn assignment(values: &[(usize, bool)]) -> HashMap<usize, bool> {
        values.iter().copied().collect()
    }

    #[test]
    fn ite_selects_then_branch_when_condition_is_true() {
        let mut manager = BddIteManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let z = manager.variable(2);

        let result = manager.bdd_ite(&x, &y, &z, true, true, true).unwrap();

        assert!(
            manager
                .evaluate(&result, &assignment(&[(0, true), (1, true), (2, false)]))
                .unwrap()
        );
        assert!(
            !manager
                .evaluate(&result, &assignment(&[(0, false), (1, true), (2, false)]))
                .unwrap()
        );
        assert_eq!(result.origin(), "bdd_ite");
    }

    #[test]
    fn phases_are_applied_before_ite_evaluation() {
        let mut manager = BddIteManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let z = manager.variable(2);

        let result = manager.bdd_ite(&x, &y, &z, false, false, true).unwrap();

        assert!(
            !manager
                .evaluate(&result, &assignment(&[(0, false), (1, true), (2, false)]))
                .unwrap()
        );
        assert!(
            manager
                .evaluate(&result, &assignment(&[(0, true), (1, true), (2, true)]))
                .unwrap()
        );
    }

    #[test]
    fn freed_external_pointer_is_rejected() {
        let mut manager = BddIteManager::new();
        let mut x = manager.variable(0);
        let one = manager.one();
        let zero = manager.zero();
        x.mark_free();

        let error = manager
            .bdd_ite(&x, &one, &zero, true, true, true)
            .unwrap_err();

        assert_eq!(error, BddIteError::InvalidBdd);
    }

    #[test]
    fn different_managers_are_rejected() {
        let mut first = BddIteManager::new();
        let mut second = BddIteManager::new();
        let x = first.variable(0);
        let one = second.one();
        let zero = first.zero();

        let error = first
            .bdd_ite(&x, &one, &zero, true, true, true)
            .unwrap_err();

        assert_eq!(error, BddIteError::DifferentManagers);
    }

    #[test]
    fn equivalent_branches_return_the_same_function() {
        let mut manager = BddIteManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result = manager.bdd_ite(&x, &y, &y, true, true, true).unwrap();

        assert_eq!(result.root(), y.root());
        assert_eq!(manager.stats().returns.trivial, 1);
    }

    #[test]
    fn ite_identity_reduces_to_condition() {
        let mut manager = BddIteManager::new();
        let x = manager.variable(0);
        let one = manager.one();
        let zero = manager.zero();

        let result = manager.bdd_ite(&x, &one, &zero, true, true, true).unwrap();

        assert_eq!(result.root(), x.root());
    }

    #[test]
    fn external_pointer_indices_and_recorder_follow_call_order() {
        let mut manager = BddIteManager::new();
        let x = manager.variable(0);
        let one = manager.one();
        let zero = manager.zero();
        let result = manager.bdd_ite(&x, &one, &zero, true, true, true).unwrap();

        assert_eq!(x.external_index(), 0);
        assert_eq!(one.external_index(), 1);
        assert_eq!(zero.external_index(), 2);
        assert_eq!(result.external_index(), 3);
        assert_eq!(
            manager.flight_recorder(),
            &["3 <- bdd_ite(0, 1, 2, true, true, true)".to_string()]
        );
    }
}
