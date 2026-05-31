use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BddHandle(usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNode {
    variable_id: usize,
    then_branch: BddHandle,
    else_branch: BddHandle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddConstantStatus {
    Zero,
    One,
    NonConstant,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteConstantReturnStats {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IteConstantStats {
    pub calls: usize,
    pub returns: IteConstantReturnStats,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    const_cache: HashMap<(BddHandle, BddHandle, BddHandle), BddConstantStatus>,
    hash_cache: HashMap<(BddHandle, BddHandle, BddHandle), BddHandle>,
    stats: IteConstantStats,
}

impl BddHandle {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(2);
    const COMPLEMENT_MASK: usize = 1;

    pub fn is_complemented(self) -> bool {
        self.0 & Self::COMPLEMENT_MASK != 0
    }

    pub fn regular(self) -> Self {
        Self(self.0 & !Self::COMPLEMENT_MASK)
    }

    pub fn complement(self) -> Self {
        Self(self.0 ^ Self::COMPLEMENT_MASK)
    }

    fn from_node_index(index: usize) -> Self {
        Self((index + 2) << 1)
    }

    fn node_index(self) -> Option<usize> {
        let regular = self.regular();

        if regular == Self::ZERO || regular == Self::ONE {
            return None;
        }

        Some((regular.0 >> 1) - 2)
    }

    fn raw(self) -> usize {
        self.0
    }
}

impl BddNode {
    pub fn new(variable_id: usize, then_branch: BddHandle, else_branch: BddHandle) -> Self {
        Self {
            variable_id,
            then_branch,
            else_branch,
        }
    }
}

impl BddConstantStatus {
    pub fn complemented(self) -> Self {
        match self {
            Self::Zero => Self::One,
            Self::One => Self::Zero,
            Self::NonConstant => Self::NonConstant,
        }
    }

    pub fn with_complement(self, complement: bool) -> Self {
        if complement {
            self.complemented()
        } else {
            self
        }
    }
}

impl BddManager {
    pub fn zero(&self) -> BddHandle {
        BddHandle::ZERO
    }

    pub fn one(&self) -> BddHandle {
        BddHandle::ONE
    }

    pub fn not(&self, handle: BddHandle) -> BddHandle {
        handle.complement()
    }

    pub fn add_node(
        &mut self,
        variable_id: usize,
        then_branch: BddHandle,
        else_branch: BddHandle,
    ) -> BddHandle {
        let handle = BddHandle::from_node_index(self.nodes.len());
        self.nodes
            .push(BddNode::new(variable_id, then_branch, else_branch));
        handle
    }

    pub fn variable(&mut self, variable_id: usize) -> BddHandle {
        self.add_node(variable_id, self.one(), self.zero())
    }

    pub fn node(&self, handle: BddHandle) -> Option<&BddNode> {
        self.nodes.get(handle.node_index()?)
    }

    pub fn stats(&self) -> IteConstantStats {
        self.stats
    }

    pub fn cache_constant_result(
        &mut self,
        f: BddHandle,
        g: BddHandle,
        h: BddHandle,
        status: BddConstantStatus,
    ) {
        self.const_cache.insert((f, g, h), status);
    }

    pub fn cache_ite_result(
        &mut self,
        f: BddHandle,
        g: BddHandle,
        h: BddHandle,
        result: BddHandle,
    ) {
        self.hash_cache.insert((f, g, h), result);
    }

    pub fn ite_constant(
        &mut self,
        mut f: BddHandle,
        mut g: BddHandle,
        mut h: BddHandle,
    ) -> BddConstantStatus {
        self.stats.calls += 1;

        if f == self.one() {
            self.stats.returns.trivial += 1;
            return self.constantness(g, false);
        }

        if f == self.zero() {
            self.stats.returns.trivial += 1;
            return self.constantness(h, false);
        }

        (g, h) = self.ite_var_to_const(f, g, h);

        if g == h {
            self.stats.returns.trivial += 1;
            return self.constantness(g, false);
        }

        if self.is_constant(g) && self.is_constant(h) {
            self.stats.returns.trivial += 1;
            return BddConstantStatus::NonConstant;
        }

        let complement;
        (f, g, h, complement) = self.canonicalize_ite_inputs(f, g, h);

        if let Some(status) = self.const_cache.get(&(f, g, h)).copied() {
            self.stats.returns.cached += 1;
            return status.with_complement(complement);
        }

        if let Some(result) = self.hash_cache.get(&(f, g, h)).copied() {
            self.stats.returns.cached += 1;
            return self.constantness(result, complement);
        }

        let variable_id = self.top_variable_id(f, g, h);
        let (f_then, f_else) = self.quick_cofactor(f, variable_id);
        let (g_then, g_else) = self.quick_cofactor(g, variable_id);
        let (h_then, h_else) = self.quick_cofactor(h, variable_id);

        let then_status = self.ite_constant(f_then, g_then, h_then);

        if then_status == BddConstantStatus::NonConstant {
            self.stats.returns.full += 1;
            self.cache_constant_result(f, g, h, BddConstantStatus::NonConstant);
            return BddConstantStatus::NonConstant;
        }

        let else_status = self.ite_constant(f_else, g_else, h_else);

        if else_status == BddConstantStatus::NonConstant || then_status != else_status {
            self.stats.returns.full += 1;
            self.cache_constant_result(f, g, h, BddConstantStatus::NonConstant);
            return BddConstantStatus::NonConstant;
        }

        self.cache_constant_result(f, g, h, then_status);
        self.stats.returns.full += 1;

        then_status.with_complement(complement)
    }

    fn constantness(&self, handle: BddHandle, complement: bool) -> BddConstantStatus {
        let status = if handle == self.zero() {
            BddConstantStatus::Zero
        } else if handle == self.one() {
            BddConstantStatus::One
        } else {
            BddConstantStatus::NonConstant
        };

        status.with_complement(complement)
    }

    fn ite_var_to_const(
        &self,
        f: BddHandle,
        mut g: BddHandle,
        mut h: BddHandle,
    ) -> (BddHandle, BddHandle) {
        if f == g {
            g = self.one();
        } else if f == self.not(g) {
            g = self.zero();
        }

        if f == h {
            h = self.zero();
        } else if f == self.not(h) {
            h = self.one();
        }

        (g, h)
    }

    fn canonicalize_ite_inputs(
        &self,
        mut f: BddHandle,
        mut g: BddHandle,
        mut h: BddHandle,
    ) -> (BddHandle, BddHandle, BddHandle, bool) {
        if self.is_constant(g) {
            if self.less_than_ordered(h, f) {
                if g == self.one() {
                    std::mem::swap(&mut f, &mut h);
                } else {
                    std::mem::swap(&mut f, &mut h);
                    f = self.not(f);
                    h = self.not(h);
                }
            }
        } else if self.is_constant(h) {
            if self.less_than_ordered(g, f) {
                if h == self.one() {
                    std::mem::swap(&mut f, &mut g);
                    f = self.not(f);
                    g = self.not(g);
                } else {
                    std::mem::swap(&mut f, &mut g);
                }
            }
        } else if g == self.not(h) && self.less_than_ordered(g, f) {
            std::mem::swap(&mut f, &mut g);
            h = self.not(g);
        }

        if f.is_complemented() {
            f = self.not(f);
            std::mem::swap(&mut g, &mut h);
        }

        if !g.is_complemented() {
            (f, g, h, false)
        } else {
            g = self.not(g);
            h = self.not(h);
            (f, g, h, true)
        }
    }

    fn is_constant(&self, handle: BddHandle) -> bool {
        let regular = handle.regular();
        regular == self.zero() || regular == self.one()
    }

    fn variable_id(&self, handle: BddHandle) -> usize {
        self.node(handle.regular())
            .map(|node| node.variable_id)
            .unwrap_or(usize::MAX)
    }

    fn less_than_ordered(&self, left: BddHandle, right: BddHandle) -> bool {
        let left_id = self.variable_id(left);
        let right_id = self.variable_id(right);

        left_id < right_id || (left_id == right_id && left.raw() < right.raw())
    }

    fn top_variable_id(&self, f: BddHandle, g: BddHandle, h: BddHandle) -> usize {
        self.variable_id(f)
            .min(self.variable_id(g))
            .min(self.variable_id(h))
    }

    fn quick_cofactor(&self, handle: BddHandle, variable_id: usize) -> (BddHandle, BddHandle) {
        let Some(node) = self.node(handle.regular()) else {
            return (handle, handle);
        };

        if node.variable_id != variable_id {
            return (handle, handle);
        }

        if handle.is_complemented() {
            (self.not(node.then_branch), self.not(node.else_branch))
        } else {
            (node.then_branch, node.else_branch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_follow_selector_fast_paths() {
        let mut manager = BddManager::default();
        let x = manager.variable(0);

        assert_eq!(
            manager.ite_constant(manager.one(), x, manager.zero()),
            BddConstantStatus::NonConstant
        );
        assert_eq!(
            manager.ite_constant(manager.zero(), x, manager.one()),
            BddConstantStatus::One
        );
    }

    #[test]
    fn same_branches_return_branch_constantness() {
        let mut manager = BddManager::default();
        let x = manager.variable(0);

        assert_eq!(
            manager.ite_constant(x, manager.one(), manager.one()),
            BddConstantStatus::One
        );
        assert_eq!(
            manager.ite_constant(x, x, x),
            BddConstantStatus::NonConstant
        );
    }

    #[test]
    fn opposite_constant_branches_are_known_nonconstant() {
        let mut manager = BddManager::default();
        let x = manager.variable(0);

        assert_eq!(
            manager.ite_constant(x, manager.one(), manager.zero()),
            BddConstantStatus::NonConstant
        );
        assert_eq!(
            manager.ite_constant(x, manager.zero(), manager.one()),
            BddConstantStatus::NonConstant
        );
    }

    #[test]
    fn recursive_then_and_else_constant_statuses_must_match() {
        let mut manager = BddManager::default();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let y_or_not_x = manager.add_node(0, y, manager.one());

        assert_eq!(
            manager.ite_constant(x, y, y_or_not_x),
            BddConstantStatus::NonConstant
        );
    }

    #[test]
    fn constant_cache_result_is_reused_with_complement_status() {
        let mut manager = BddManager::default();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let not_y = manager.not(y);

        manager.cache_constant_result(x, y, not_y, BddConstantStatus::One);

        assert_eq!(
            manager.ite_constant(y, manager.not(x), x),
            BddConstantStatus::Zero
        );
        assert_eq!(manager.stats().returns.cached, 1);
    }

    #[test]
    fn hash_cache_result_is_reused_with_complement_status() {
        let mut manager = BddManager::default();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let not_y = manager.not(y);

        manager.cache_ite_result(x, y, not_y, manager.one());

        assert_eq!(
            manager.ite_constant(y, manager.not(x), x),
            BddConstantStatus::Zero
        );
        assert_eq!(manager.stats().returns.cached, 1);
    }

    #[test]
    fn hash_cache_result_reports_constantness() {
        let mut manager = BddManager::default();
        let x = manager.variable(0);
        let y = manager.variable(1);

        manager.cache_ite_result(x, y, manager.zero(), manager.one());

        assert_eq!(
            manager.ite_constant(x, y, manager.zero()),
            BddConstantStatus::One
        );
        assert_eq!(manager.stats().returns.cached, 1);
    }

    #[test]
    fn file_contains_no_legacy_abi_or_tracking_tokens() {
        let source = include_str!("ite_constant.rs");

        assert!(!source.contains(&["no", "_mangle"].concat()));
        assert!(!source.contains("extern \"C\""));
        assert!(!source.contains(&["REQUIRED", "_"].concat()));
        assert!(!source.contains(&["Port", "Dependency"].concat()));
        assert!(!source.contains(&["bead", "_id"].concat()));
        assert!(!source.contains(&["source", "_file"].concat()));
    }
}
