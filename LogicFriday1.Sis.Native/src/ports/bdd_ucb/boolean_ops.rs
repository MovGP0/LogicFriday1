use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_MANAGER_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct BddRef(usize);

impl BddRef {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(2);

    fn complement(self) -> Self {
        match self {
            Self::ZERO => Self::ONE,
            Self::ONE => Self::ZERO,
            _ => Self(self.0 ^ 1),
        }
    }

    fn is_complemented(self) -> bool {
        self.0 & 1 == 1
    }

    fn regular(self) -> Self {
        match self {
            Self::ZERO | Self::ONE => self,
            _ => Self(self.0 & !1),
        }
    }

    fn node_index(self) -> usize {
        (self.regular().0 >> 1).saturating_sub(2)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BddNode {
    variable: usize,
    then_branch: BddRef,
    else_branch: BddRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bdd {
    manager_id: usize,
    root: BddRef,
    free: bool,
    origin: &'static str,
}

impl Bdd {
    pub fn is_free(&self) -> bool {
        self.free
    }

    pub fn origin(&self) -> &'static str {
        self.origin
    }

    pub fn mark_free(&mut self) {
        self.free = true;
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BooleanOpStats {
    pub ite_calls: usize,
    pub unique_hits: usize,
    pub unique_misses: usize,
}

#[derive(Clone, Debug)]
pub struct BddManager {
    id: usize,
    nodes: Vec<BddNode>,
    unique_table: HashMap<(usize, BddRef, BddRef), BddRef>,
    stats: BooleanOpStats,
}

impl Default for BddManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BddManager {
    pub fn new() -> Self {
        Self {
            id: NEXT_MANAGER_ID.fetch_add(1, Ordering::Relaxed),
            nodes: Vec::new(),
            unique_table: HashMap::new(),
            stats: BooleanOpStats::default(),
        }
    }

    pub fn stats(&self) -> BooleanOpStats {
        self.stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn variable(&mut self, variable: usize) -> Bdd {
        let root = self.find_or_add(variable, BddRef::ONE, BddRef::ZERO);
        self.make_external(root, "bdd_variable")
    }

    pub fn one(&self) -> Bdd {
        self.make_external(BddRef::ONE, "bdd_one")
    }

    pub fn zero(&self) -> Bdd {
        self.make_external(BddRef::ZERO, "bdd_zero")
    }

    pub fn not(&self, value: &Bdd) -> Result<Bdd, BooleanOpError> {
        self.validate_operand(value, "bdd_not")?;

        Ok(self.make_external(value.root.complement(), "bdd_not"))
    }

    pub fn and(
        &mut self,
        left: &Bdd,
        right: &Bdd,
        left_phase: bool,
        right_phase: bool,
    ) -> Result<Bdd, BooleanOpError> {
        let left = self.phased_operand(left, left_phase, "bdd_and")?;
        let right = self.phased_operand(right, right_phase, "bdd_and")?;
        let root = self.ite(left, right, BddRef::ZERO)?;

        Ok(self.make_external(root, "bdd_and"))
    }

    pub fn or(
        &mut self,
        left: &Bdd,
        right: &Bdd,
        left_phase: bool,
        right_phase: bool,
    ) -> Result<Bdd, BooleanOpError> {
        let left = self.phased_operand(left, left_phase, "bdd_or")?;
        let right = self.phased_operand(right, right_phase, "bdd_or")?;
        let root = self.ite(left, BddRef::ONE, right)?;

        Ok(self.make_external(root, "bdd_or"))
    }

    pub fn xor(&mut self, left: &Bdd, right: &Bdd) -> Result<Bdd, BooleanOpError> {
        let left = self.validated_root(left, "bdd_xor")?;
        let right = self.validated_root(right, "bdd_xor")?;
        let root = self.ite(left, right.complement(), right)?;

        Ok(self.make_external(root, "bdd_xor"))
    }

    pub fn xnor(&mut self, left: &Bdd, right: &Bdd) -> Result<Bdd, BooleanOpError> {
        let left = self.validated_root(left, "bdd_xnor")?;
        let right = self.validated_root(right, "bdd_xnor")?;
        let root = self.ite(left, right, right.complement())?;

        Ok(self.make_external(root, "bdd_xnor"))
    }

    pub fn is_tautology(&self, value: &Bdd, phase: bool) -> Result<bool, BooleanOpError> {
        let root = self.validated_root(value, "bdd_is_tautology")?;
        let constant = if phase { BddRef::ONE } else { BddRef::ZERO };

        Ok(root == constant)
    }

    pub fn equal(&self, left: &Bdd, right: &Bdd) -> Result<bool, BooleanOpError> {
        let left = self.validated_root(left, "bdd_equal")?;
        let right = self.validated_root(right, "bdd_equal")?;

        Ok(left == right)
    }

    pub fn leq(
        &mut self,
        left: &Bdd,
        right: &Bdd,
        left_phase: bool,
        right_phase: bool,
    ) -> Result<bool, BooleanOpError> {
        let left = self.phased_operand(left, left_phase, "bdd_leq")?;
        let right = self.phased_operand(right, right_phase, "bdd_leq")?;
        let implication = self.ite(left, right, BddRef::ONE)?;

        Ok(implication == BddRef::ONE)
    }

    pub fn evaluate(
        &self,
        value: &Bdd,
        assignment: &HashMap<usize, bool>,
    ) -> Result<bool, BooleanOpError> {
        let mut current = self.validated_root(value, "bdd_eval")?;
        let mut complemented = false;

        loop {
            match current {
                BddRef::ZERO => return Ok(complemented),
                BddRef::ONE => return Ok(!complemented),
                _ => {
                    complemented ^= current.is_complemented();
                    let node = self.node(current.regular())?;
                    current = if assignment.get(&node.variable).copied().unwrap_or(false) {
                        node.then_branch
                    } else {
                        node.else_branch
                    };
                }
            }
        }
    }

    fn make_external(&self, root: BddRef, origin: &'static str) -> Bdd {
        Bdd {
            manager_id: self.id,
            root,
            free: false,
            origin,
        }
    }

    fn phased_operand(
        &self,
        value: &Bdd,
        phase: bool,
        operation: &'static str,
    ) -> Result<BddRef, BooleanOpError> {
        let root = self.validated_root(value, operation)?;

        Ok(if phase { root } else { root.complement() })
    }

    fn validated_root(
        &self,
        value: &Bdd,
        operation: &'static str,
    ) -> Result<BddRef, BooleanOpError> {
        self.validate_operand(value, operation)?;
        self.validate_ref(value.root)?;

        Ok(value.root)
    }

    fn validate_operand(&self, value: &Bdd, operation: &'static str) -> Result<(), BooleanOpError> {
        if value.free {
            return Err(BooleanOpError::FreedBdd { operation });
        }

        if value.manager_id != self.id {
            return Err(BooleanOpError::DifferentManagers { operation });
        }

        Ok(())
    }

    fn validate_ref(&self, value: BddRef) -> Result<(), BooleanOpError> {
        if self.is_constant(value) {
            return Ok(());
        }

        if value.node_index() < self.nodes.len() {
            Ok(())
        } else {
            Err(BooleanOpError::InvalidNode)
        }
    }

    fn ite(&mut self, f: BddRef, g: BddRef, h: BddRef) -> Result<BddRef, BooleanOpError> {
        self.stats.ite_calls += 1;

        if f == BddRef::ONE {
            return Ok(g);
        }

        if f == BddRef::ZERO {
            return Ok(h);
        }

        let (g, h) = self.ite_var_to_const(f, g, h);

        if g == h {
            return Ok(g);
        }

        if g == BddRef::ONE && h == BddRef::ZERO {
            return Ok(f);
        }

        if g == BddRef::ZERO && h == BddRef::ONE {
            return Ok(f.complement());
        }

        let top_variable = self
            .variable_id(f)?
            .min(self.variable_id(g)?)
            .min(self.variable_id(h)?);
        let (f_then, f_else) = self.quick_cofactor(f, top_variable)?;
        let (g_then, g_else) = self.quick_cofactor(g, top_variable)?;
        let (h_then, h_else) = self.quick_cofactor(h, top_variable)?;
        let then_result = self.ite(f_then, g_then, h_then)?;
        let else_result = self.ite(f_else, g_else, h_else)?;

        Ok(self.find_or_add(top_variable, then_result, else_result))
    }

    fn ite_var_to_const(&self, f: BddRef, mut g: BddRef, mut h: BddRef) -> (BddRef, BddRef) {
        if f == g {
            g = BddRef::ONE;
        } else if f == g.complement() {
            g = BddRef::ZERO;
        }

        if f == h {
            h = BddRef::ZERO;
        } else if f == h.complement() {
            h = BddRef::ONE;
        }

        (g, h)
    }

    fn quick_cofactor(
        &self,
        value: BddRef,
        variable: usize,
    ) -> Result<(BddRef, BddRef), BooleanOpError> {
        if self.is_constant(value) {
            return Ok((value, value));
        }

        let node = self.node(value.regular())?;
        if node.variable != variable {
            return Ok((value, value));
        }

        if value.is_complemented() {
            Ok((node.then_branch.complement(), node.else_branch.complement()))
        } else {
            Ok((node.then_branch, node.else_branch))
        }
    }

    fn find_or_add(&mut self, variable: usize, then_branch: BddRef, else_branch: BddRef) -> BddRef {
        if then_branch == else_branch {
            return then_branch;
        }

        let key = (variable, then_branch, else_branch);
        if let Some(existing) = self.unique_table.get(&key).copied() {
            self.stats.unique_hits += 1;
            return existing;
        }

        let root = BddRef((self.nodes.len() + 2) << 1);
        self.nodes.push(BddNode {
            variable,
            then_branch,
            else_branch,
        });
        self.unique_table.insert(key, root);
        self.stats.unique_misses += 1;

        root
    }

    fn variable_id(&self, value: BddRef) -> Result<usize, BooleanOpError> {
        if self.is_constant(value) {
            return Ok(usize::MAX);
        }

        Ok(self.node(value.regular())?.variable)
    }

    fn node(&self, value: BddRef) -> Result<&BddNode, BooleanOpError> {
        self.nodes
            .get(value.node_index())
            .ok_or(BooleanOpError::InvalidNode)
    }

    fn is_constant(&self, value: BddRef) -> bool {
        matches!(value, BddRef::ZERO | BddRef::ONE)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanOpError {
    InvalidNode,
    FreedBdd { operation: &'static str },
    DifferentManagers { operation: &'static str },
}

impl fmt::Display for BooleanOpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode => write!(formatter, "invalid BDD"),
            Self::FreedBdd { operation } => write!(formatter, "{operation}: invalid BDD"),
            Self::DifferentManagers { operation } => {
                write!(formatter, "{operation}: different bdd managers")
            }
        }
    }
}

impl Error for BooleanOpError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn assignment(values: &[(usize, bool)]) -> HashMap<usize, bool> {
        values.iter().copied().collect()
    }

    #[test]
    fn constants_are_returned_as_external_bdds() {
        let manager = BddManager::new();
        let one = manager.one();
        let zero = manager.zero();

        assert_eq!(one.origin(), "bdd_one");
        assert_eq!(zero.origin(), "bdd_zero");
        assert!(manager.is_tautology(&one, true).unwrap());
        assert!(manager.is_tautology(&zero, false).unwrap());
    }

    #[test]
    fn not_returns_complemented_external_pointer() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let not_x = manager.not(&x).unwrap();

        assert!(!manager.evaluate(&not_x, &assignment(&[(0, true)])).unwrap());
        assert!(
            manager
                .evaluate(&not_x, &assignment(&[(0, false)]))
                .unwrap()
        );
    }

    #[test]
    fn and_and_or_honor_input_phases() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let x_and_y = manager.and(&x, &y, true, true).unwrap();
        let not_x_or_y = manager.or(&x, &y, false, true).unwrap();

        assert!(
            manager
                .evaluate(&x_and_y, &assignment(&[(0, true), (1, true)]))
                .unwrap()
        );
        assert!(
            !manager
                .evaluate(&x_and_y, &assignment(&[(0, true), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .evaluate(&not_x_or_y, &assignment(&[(0, false), (1, false)]))
                .unwrap()
        );
        assert!(
            !manager
                .evaluate(&not_x_or_y, &assignment(&[(0, true), (1, false)]))
                .unwrap()
        );
    }

    #[test]
    fn xor_and_xnor_are_complements() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let xor = manager.xor(&x, &y).unwrap();
        let xnor = manager.xnor(&x, &y).unwrap();

        for x_value in [false, true] {
            for y_value in [false, true] {
                let values = assignment(&[(0, x_value), (1, y_value)]);

                assert_eq!(manager.evaluate(&xor, &values).unwrap(), x_value ^ y_value);
                assert_eq!(
                    manager.evaluate(&xnor, &values).unwrap(),
                    x_value == y_value
                );
            }
        }
    }

    #[test]
    fn equality_uses_canonical_root_identity() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let first = manager.and(&x, &y, true, true).unwrap();
        let second = manager.and(&x, &y, true, true).unwrap();
        let different = manager.or(&x, &y, true, true).unwrap();

        assert!(manager.equal(&first, &second).unwrap());
        assert!(!manager.equal(&first, &different).unwrap());
        assert!(manager.stats().unique_hits > 0);
    }

    #[test]
    fn leq_checks_boolean_implication_with_phases() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let x_and_y = manager.and(&x, &y, true, true).unwrap();

        assert!(manager.leq(&x_and_y, &x, true, true).unwrap());
        assert!(!manager.leq(&x, &x_and_y, true, true).unwrap());
        assert!(manager.leq(&x, &x_and_y, false, false).unwrap());
    }

    #[test]
    fn rejects_freed_and_cross_manager_operands() {
        let mut first = BddManager::new();
        let mut second = BddManager::new();
        let mut x = first.variable(0);
        let y = first.variable(1);
        let other = second.variable(0);

        x.mark_free();

        assert_eq!(
            first.and(&x, &y, true, true),
            Err(BooleanOpError::FreedBdd {
                operation: "bdd_and"
            })
        );
        assert_eq!(
            first.equal(&y, &other),
            Err(BooleanOpError::DifferentManagers {
                operation: "bdd_equal"
            })
        );
    }

    #[test]
    fn file_contains_no_legacy_abi_or_tracking_tokens() {
        let source = include_str!("boolean_ops.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
