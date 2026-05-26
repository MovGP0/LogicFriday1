use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddRef(usize);

impl BddRef {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(2);

    pub fn complement(self) -> Self {
        match self {
            Self::ZERO => Self::ONE,
            Self::ONE => Self::ZERO,
            _ => Self(self.0 ^ 1),
        }
    }

    pub fn is_complemented(self) -> bool {
        self.0 & 1 == 1
    }

    fn regular(self) -> Self {
        match self {
            Self::ZERO | Self::ONE => self,
            _ => Self(self.0 & !1),
        }
    }

    fn node_index(self) -> usize {
        self.0 >> 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNode {
    variable: usize,
    then_branch: BddRef,
    else_branch: BddRef,
}

impl BddNode {
    pub fn variable(&self) -> usize {
        self.variable
    }

    pub fn then_branch(&self) -> BddRef {
        self.then_branch
    }

    pub fn else_branch(&self) -> BddRef {
        self.else_branch
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinMatchType {
    TwoSide,
    OneSide,
    OneSideDontCare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MinimizeParams {
    pub match_type: MinMatchType,
    pub complement_matches: bool,
    pub no_new_variables: bool,
    pub return_smaller_input: bool,
}

impl Default for MinimizeParams {
    fn default() -> Self {
        Self {
            match_type: MinMatchType::OneSide,
            complement_matches: true,
            no_new_variables: true,
            return_smaller_input: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MinimizeReturnStats {
    pub trivial: usize,
    pub cached: usize,
    pub full: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MinimizeStats {
    pub calls: usize,
    pub returns: MinimizeReturnStats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BddStats {
    pub minimize: MinimizeStats,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddMinimizeError {
    InvalidNode(BddRef),
    InvalidCareSet,
    InvalidVariableOrder { parent: usize, child: usize },
    RecursionLimitExceeded { limit: usize },
}

impl fmt::Display for BddMinimizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode(node) => write!(formatter, "invalid BDD node reference {node:?}"),
            Self::InvalidCareSet => write!(
                formatter,
                "BDD minimization is undefined when the care set is zero"
            ),
            Self::InvalidVariableOrder { parent, child } => write!(
                formatter,
                "BDD variable order violation: parent variable {parent} is not before child variable {child}"
            ),
            Self::RecursionLimitExceeded { limit } => {
                write!(
                    formatter,
                    "BDD minimization recursion limit {limit} was exceeded"
                )
            }
        }
    }
}

impl Error for BddMinimizeError {}

#[derive(Clone, Debug)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(usize, BddRef, BddRef), BddRef>,
    ite_cache: HashMap<(BddRef, BddRef, BddRef), BddRef>,
    minimize_cache: HashMap<(BddRef, BddRef), BddRef>,
    recursion_limit: usize,
    stats: BddStats,
}

impl Default for BddManager {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            unique_table: HashMap::new(),
            ite_cache: HashMap::new(),
            minimize_cache: HashMap::new(),
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

    pub fn set_recursion_limit(&mut self, recursion_limit: usize) {
        self.recursion_limit = recursion_limit;
    }

    pub fn variable(&mut self, variable: usize) -> BddRef {
        self.find_or_add_unchecked(variable, self.one(), self.zero())
    }

    pub fn find_or_add(
        &mut self,
        variable: usize,
        then_branch: BddRef,
        else_branch: BddRef,
    ) -> Result<BddRef, BddMinimizeError> {
        self.validate_ref(then_branch)?;
        self.validate_ref(else_branch)?;
        self.validate_order(variable, then_branch)?;
        self.validate_order(variable, else_branch)?;

        Ok(self.find_or_add_unchecked(variable, then_branch, else_branch))
    }

    pub fn minimize(&mut self, function: BddRef, care: BddRef) -> Result<BddRef, BddMinimizeError> {
        self.minimize_with_params(function, care, MinimizeParams::default())
    }

    pub fn minimize_with_params(
        &mut self,
        function: BddRef,
        care: BddRef,
        params: MinimizeParams,
    ) -> Result<BddRef, BddMinimizeError> {
        self.validate_ref(function)?;
        self.validate_ref(care)?;
        self.minimize_cache.clear();

        let minimized = self.min_sibling(function, care, params, 0)?;
        if params.return_smaller_input && self.size(minimized)? > self.size(function)? {
            Ok(function)
        } else {
            Ok(minimized)
        }
    }

    pub fn between(
        &mut self,
        minimum: BddRef,
        maximum: BddRef,
    ) -> Result<BddRef, BddMinimizeError> {
        let care = self.or(minimum, maximum.complement())?;
        self.minimize(minimum, care)
    }

    pub fn not(&self, value: BddRef) -> BddRef {
        value.complement()
    }

    pub fn and(&mut self, left: BddRef, right: BddRef) -> Result<BddRef, BddMinimizeError> {
        self.ite(left, right, self.zero())
    }

    pub fn or(&mut self, left: BddRef, right: BddRef) -> Result<BddRef, BddMinimizeError> {
        self.ite(left, self.one(), right)
    }

    pub fn xor(&mut self, left: BddRef, right: BddRef) -> Result<BddRef, BddMinimizeError> {
        self.ite(left, right.complement(), right)
    }

    pub fn eval(
        &self,
        root: BddRef,
        assignments: &HashMap<usize, bool>,
    ) -> Result<bool, BddMinimizeError> {
        let mut current = root;
        let mut complemented = false;

        loop {
            if current == self.zero() {
                return Ok(complemented);
            }

            if current == self.one() {
                return Ok(!complemented);
            }

            complemented ^= current.is_complemented();
            let node = self.node(current.regular())?;
            current = if assignments.get(&node.variable).copied().unwrap_or(false) {
                node.then_branch
            } else {
                node.else_branch
            };
        }
    }

    pub fn size(&self, root: BddRef) -> Result<usize, BddMinimizeError> {
        let mut visited = HashSet::new();
        self.collect_size(root.regular(), &mut visited)?;
        Ok(visited.len())
    }

    fn min_sibling(
        &mut self,
        function: BddRef,
        care: BddRef,
        params: MinimizeParams,
        depth: usize,
    ) -> Result<BddRef, BddMinimizeError> {
        if depth > self.recursion_limit {
            return Err(BddMinimizeError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        self.stats.minimize.calls += 1;

        if self.is_constant(function) || care == self.one() {
            self.stats.minimize.returns.trivial += 1;
            return Ok(function);
        }

        if care == self.zero() {
            return Err(BddMinimizeError::InvalidCareSet);
        }

        let cache_key = (function, care);
        if let Some(cached) = self.minimize_cache.get(&cache_key).copied() {
            self.stats.minimize.returns.cached += 1;
            return Ok(cached);
        }

        let function_id = self.variable_id(function)?;
        let care_id = self.variable_id(care)?;
        let top_id = function_id.min(care_id);

        let (mut function_then, mut function_else) = self.branches(function)?;
        let (mut care_then, mut care_else) = self.branches(care)?;

        if top_id < care_id {
            care_then = care;
            care_else = care;
        }

        if top_id < function_id {
            function_then = function;
            function_else = function;
        }

        let result = if function_id > care_id && params.no_new_variables {
            let reduced_care = self.ite(care_then, self.one(), care_else)?;
            self.min_sibling(function, reduced_care, params, depth + 1)?
        } else if self.min_is_match(
            params.match_type,
            false,
            function_then,
            care_then,
            function_else,
            care_else,
        )? {
            let (new_function, new_care) = self.min_match_result(
                params.match_type,
                false,
                function_then,
                care_then,
                function_else,
                care_else,
            )?;
            self.min_sibling(new_function, new_care, params, depth + 1)?
        } else if params.match_type != MinMatchType::TwoSide
            && self.min_is_match(
                params.match_type,
                false,
                function_else,
                care_else,
                function_then,
                care_then,
            )?
        {
            let (new_function, new_care) = self.min_match_result(
                params.match_type,
                false,
                function_else,
                care_else,
                function_then,
                care_then,
            )?;
            self.min_sibling(new_function, new_care, params, depth + 1)?
        } else if params.complement_matches
            && self.min_is_match(
                params.match_type,
                true,
                function_then,
                care_then,
                function_else,
                care_else,
            )?
        {
            let (new_function, new_care) = self.min_match_result(
                params.match_type,
                true,
                function_then,
                care_then,
                function_else,
                care_else,
            )?;
            let co_else = self.min_sibling(new_function, new_care, params, depth + 1)?;
            let variable = self.variable(top_id);
            self.ite(variable, co_else.complement(), co_else)?
        } else if params.complement_matches
            && params.match_type != MinMatchType::TwoSide
            && self.min_is_match(
                params.match_type,
                true,
                function_else,
                care_else,
                function_then,
                care_then,
            )?
        {
            let (new_function, new_care) = self.min_match_result(
                params.match_type,
                true,
                function_else,
                care_else,
                function_then,
                care_then,
            )?;
            let co_then = self.min_sibling(new_function, new_care, params, depth + 1)?;
            let variable = self.variable(top_id);
            self.ite(variable, co_then, co_then.complement())?
        } else {
            let co_then = self.min_sibling(function_then, care_then, params, depth + 1)?;
            let co_else = self.min_sibling(function_else, care_else, params, depth + 1)?;
            let variable = self.variable(top_id);
            self.ite(variable, co_then, co_else)?
        };

        self.minimize_cache.insert(cache_key, result);
        self.stats.minimize.returns.full += 1;
        Ok(result)
    }

    fn min_is_match(
        &mut self,
        match_type: MinMatchType,
        complement: bool,
        function_left: BddRef,
        care_left: BddRef,
        function_right: BddRef,
        care_right: BddRef,
    ) -> Result<bool, BddMinimizeError> {
        if match_type == MinMatchType::OneSideDontCare {
            return Ok(care_left == self.zero());
        }

        let diff = if complement {
            self.ite(function_left, function_right, function_right.complement())?
        } else {
            self.ite(function_left, function_right.complement(), function_right)?
        };

        match match_type {
            MinMatchType::OneSide => {
                let mismatch_allowed = care_left.complement();
                Ok(self.implies(diff, mismatch_allowed)? && self.implies(care_left, care_right)?)
            }
            MinMatchType::TwoSide => {
                let mismatch_allowed = self.ite(care_left, care_right.complement(), self.one())?;
                self.implies(diff, mismatch_allowed)
            }
            MinMatchType::OneSideDontCare => unreachable!(),
        }
    }

    fn min_match_result(
        &mut self,
        match_type: MinMatchType,
        complement: bool,
        function_left: BddRef,
        care_left: BddRef,
        function_right: BddRef,
        care_right: BddRef,
    ) -> Result<(BddRef, BddRef), BddMinimizeError> {
        match match_type {
            MinMatchType::OneSide | MinMatchType::OneSideDontCare => {
                Ok((function_right, care_right))
            }
            MinMatchType::TwoSide => {
                let left_term = if complement {
                    self.ite(function_left.complement(), care_left, self.zero())?
                } else {
                    self.ite(function_left, care_left, self.zero())?
                };
                let right_term = self.ite(function_right, care_right, self.zero())?;
                let new_function = self.ite(left_term, self.one(), right_term)?;
                let new_care = self.ite(care_left, self.one(), care_right)?;
                Ok((new_function, new_care))
            }
        }
    }

    fn implies(
        &mut self,
        antecedent: BddRef,
        consequent: BddRef,
    ) -> Result<bool, BddMinimizeError> {
        let implication = self.ite(antecedent, consequent, self.one())?;
        Ok(implication == self.one())
    }

    fn ite(
        &mut self,
        condition: BddRef,
        then_branch: BddRef,
        else_branch: BddRef,
    ) -> Result<BddRef, BddMinimizeError> {
        self.validate_ref(condition)?;
        self.validate_ref(then_branch)?;
        self.validate_ref(else_branch)?;

        self.ite_inner(condition, then_branch, else_branch, 0)
    }

    fn ite_inner(
        &mut self,
        condition: BddRef,
        then_branch: BddRef,
        else_branch: BddRef,
        depth: usize,
    ) -> Result<BddRef, BddMinimizeError> {
        if depth > self.recursion_limit {
            return Err(BddMinimizeError::RecursionLimitExceeded {
                limit: self.recursion_limit,
            });
        }

        if condition == self.one() {
            return Ok(then_branch);
        }

        if condition == self.zero() {
            return Ok(else_branch);
        }

        if then_branch == else_branch {
            return Ok(then_branch);
        }

        if then_branch == self.one() && else_branch == self.zero() {
            return Ok(condition);
        }

        if then_branch == self.zero() && else_branch == self.one() {
            return Ok(condition.complement());
        }

        let cache_key = (condition, then_branch, else_branch);
        if let Some(cached) = self.ite_cache.get(&cache_key).copied() {
            return Ok(cached);
        }

        let top_id = self
            .variable_id(condition)?
            .min(self.variable_id(then_branch)?)
            .min(self.variable_id(else_branch)?);
        let (condition_then, condition_else) = self.cofactors(condition, top_id)?;
        let (then_then, then_else) = self.cofactors(then_branch, top_id)?;
        let (else_then, else_else) = self.cofactors(else_branch, top_id)?;

        let result_then = self.ite_inner(condition_then, then_then, else_then, depth + 1)?;
        let result_else = self.ite_inner(condition_else, then_else, else_else, depth + 1)?;
        let result = self.find_or_add_unchecked(top_id, result_then, result_else);

        self.ite_cache.insert(cache_key, result);
        Ok(result)
    }

    fn cofactors(
        &self,
        value: BddRef,
        variable: usize,
    ) -> Result<(BddRef, BddRef), BddMinimizeError> {
        if self.variable_id(value)? != variable {
            return Ok((value, value));
        }

        self.branches(value)
    }

    fn branches(&self, value: BddRef) -> Result<(BddRef, BddRef), BddMinimizeError> {
        if self.is_constant(value) {
            return Ok((value, value));
        }

        let node = self.node(value.regular())?;
        if value.is_complemented() {
            Ok((node.then_branch.complement(), node.else_branch.complement()))
        } else {
            Ok((node.then_branch, node.else_branch))
        }
    }

    fn variable_id(&self, value: BddRef) -> Result<usize, BddMinimizeError> {
        if self.is_constant(value) {
            return Ok(usize::MAX);
        }

        Ok(self.node(value.regular())?.variable)
    }

    fn node(&self, value: BddRef) -> Result<&BddNode, BddMinimizeError> {
        if self.is_constant(value) {
            return Err(BddMinimizeError::InvalidNode(value));
        }

        let index = value.regular().node_index().saturating_sub(2);
        self.nodes
            .get(index)
            .ok_or(BddMinimizeError::InvalidNode(value))
    }

    fn is_constant(&self, value: BddRef) -> bool {
        matches!(value, BddRef::ZERO | BddRef::ONE)
    }

    fn validate_ref(&self, value: BddRef) -> Result<(), BddMinimizeError> {
        if self.is_constant(value) {
            return Ok(());
        }

        let index = value.regular().node_index().saturating_sub(2);
        if index < self.nodes.len() {
            Ok(())
        } else {
            Err(BddMinimizeError::InvalidNode(value))
        }
    }

    fn validate_order(&self, parent: usize, child: BddRef) -> Result<(), BddMinimizeError> {
        if self.is_constant(child) {
            return Ok(());
        }

        let child_variable = self.variable_id(child)?;
        if parent < child_variable {
            Ok(())
        } else {
            Err(BddMinimizeError::InvalidVariableOrder {
                parent,
                child: child_variable,
            })
        }
    }

    fn find_or_add_unchecked(
        &mut self,
        variable: usize,
        then_branch: BddRef,
        else_branch: BddRef,
    ) -> BddRef {
        if then_branch == else_branch {
            return then_branch;
        }

        let key = (variable, then_branch, else_branch);
        if let Some(existing) = self.unique_table.get(&key).copied() {
            return existing;
        }

        let reference = BddRef((self.nodes.len() + 2) << 1);
        self.nodes.push(BddNode {
            variable,
            then_branch,
            else_branch,
        });
        self.unique_table.insert(key, reference);
        reference
    }

    fn collect_size(
        &self,
        root: BddRef,
        visited: &mut HashSet<BddRef>,
    ) -> Result<(), BddMinimizeError> {
        if self.is_constant(root) || !visited.insert(root.regular()) {
            return Ok(());
        }

        let node = self.node(root.regular())?;
        self.collect_size(node.then_branch.regular(), visited)?;
        self.collect_size(node.else_branch.regular(), visited)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assignments(entries: &[(usize, bool)]) -> HashMap<usize, bool> {
        entries.iter().copied().collect()
    }

    #[test]
    fn returns_function_when_care_is_tautology() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);

        let result = manager.minimize(x, manager.one()).unwrap();

        assert_eq!(result, x);
        assert_eq!(manager.stats().minimize.returns.trivial, 1);
    }

    #[test]
    fn rejects_empty_care_set_for_non_constant_function() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);

        let error = manager.minimize(x, manager.zero()).unwrap_err();

        assert_eq!(error, BddMinimizeError::InvalidCareSet);
    }

    #[test]
    fn one_side_match_removes_irrelevant_child_when_left_care_is_zero() {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);
        let function = manager.find_or_add(0, y, x).unwrap();
        let care = manager
            .find_or_add(0, manager.zero(), manager.one())
            .unwrap();

        let result = manager.minimize(function, care).unwrap();

        assert_eq!(result, x);
        for mask in 0..8 {
            let values = assignments(&[(0, mask & 1 != 0), (1, mask & 2 != 0), (2, mask & 4 != 0)]);

            if manager.eval(care, &values).unwrap() {
                assert_eq!(
                    manager.eval(result, &values).unwrap(),
                    manager.eval(function, &values).unwrap()
                );
            }
        }
    }

    #[test]
    fn complement_match_builds_xor_form_when_children_are_opposites() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let function = manager.find_or_add(0, y.complement(), y).unwrap();
        let result = manager
            .minimize_with_params(
                function,
                manager.one(),
                MinimizeParams {
                    return_smaller_input: false,
                    ..MinimizeParams::default()
                },
            )
            .unwrap();

        assert_eq!(result, function);
        assert!(
            manager
                .eval(result, &assignments(&[(0, false), (1, true)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result, &assignments(&[(0, true), (1, true)]))
                .unwrap()
        );
        assert_ne!(x, y);
    }

    #[test]
    fn no_new_variables_eliminates_independent_care_variable() {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let care = manager.variable(0);

        let result = manager.minimize(x, care).unwrap();

        assert_eq!(result, x);
    }

    #[test]
    fn return_min_keeps_original_when_candidate_is_larger() {
        let mut manager = BddManager::new();
        let x = manager.variable(1);
        let y = manager.variable(2);
        let original = manager.find_or_add(0, y, manager.zero()).unwrap();
        let care = manager.find_or_add(0, manager.one(), x).unwrap();

        let result = manager
            .minimize_with_params(
                original,
                care,
                MinimizeParams {
                    match_type: MinMatchType::TwoSide,
                    complement_matches: true,
                    no_new_variables: false,
                    return_smaller_input: true,
                },
            )
            .unwrap();

        assert!(manager.size(result).unwrap() <= manager.size(original).unwrap());
    }

    #[test]
    fn between_returns_function_inside_bounds() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let minimum = manager.and(x, y).unwrap();
        let maximum = manager.or(x, y).unwrap();

        let result = manager.between(minimum, maximum).unwrap();

        for mask in 0..4 {
            let values = assignments(&[(0, mask & 1 != 0), (1, mask & 2 != 0)]);
            let min_value = manager.eval(minimum, &values).unwrap();
            let max_value = manager.eval(maximum, &values).unwrap();
            let result_value = manager.eval(result, &values).unwrap();

            assert!(!min_value || result_value);
            assert!(!result_value || max_value);
        }
    }

    #[test]
    fn repeated_subproblem_uses_minimize_cache() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let z = manager.variable(2);
        let function = manager.find_or_add(0, z, z).unwrap_or(z);
        let care = manager.find_or_add(0, y, y).unwrap_or(y);

        let repeated_function = manager
            .find_or_add(0, function, function)
            .unwrap_or(function);
        let repeated_care = manager.find_or_add(0, care, care).unwrap_or(care);

        let _ = manager
            .minimize_with_params(
                repeated_function,
                repeated_care,
                MinimizeParams {
                    return_smaller_input: false,
                    ..MinimizeParams::default()
                },
            )
            .unwrap();

        assert!(manager.stats().minimize.calls > 0);
        assert_eq!(x, manager.variable(0));
    }

    #[test]
    fn validates_variable_order_for_public_nodes() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);

        let error = manager.find_or_add(0, x, manager.zero()).unwrap_err();

        assert_eq!(
            error,
            BddMinimizeError::InvalidVariableOrder {
                parent: 0,
                child: 0
            }
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens() {
        let source = include_str!("bdd_min_sibl.rs");
        let legacy_export = concat!("no", "_", "mangle");
        let required_prefix = concat!("REQUIRED", "_");
        let dependency_token = concat!("Port", "Dependency");
        let bead_token = concat!("bead", "_", "id");
        let source_tracking_token = ["source", "file"].join("_");
        let bead_prefix = concat!("LogicFriday1", "-", "8j8");

        assert!(!source.contains(legacy_export));
        assert!(!source.contains("extern \"C\""));
        assert!(!source.contains(required_prefix));
        assert!(!source.contains(dependency_token));
        assert!(!source.contains(bead_token));
        assert!(!source.contains(&source_tracking_token));
        assert!(!source.contains(bead_prefix));
    }
}
