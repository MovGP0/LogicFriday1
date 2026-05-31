use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type BddVariable = usize;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddEdge(usize);

impl BddEdge {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(2);
    const COMPLEMENT_MASK: usize = 1;

    pub fn is_complemented(self) -> bool {
        self.0 & Self::COMPLEMENT_MASK != 0
    }

    pub fn not(self) -> Self {
        match self {
            Self::ZERO => Self::ONE,
            Self::ONE => Self::ZERO,
            _ => Self(self.0 ^ Self::COMPLEMENT_MASK),
        }
    }

    fn regular(self) -> Self {
        match self {
            Self::ZERO | Self::ONE => self,
            _ => Self(self.0 & !Self::COMPLEMENT_MASK),
        }
    }

    fn node_index(self) -> Option<usize> {
        let regular = self.regular();

        if regular == Self::ZERO || regular == Self::ONE {
            return None;
        }

        Some((regular.0 >> 1) - 2)
    }

    fn from_node_index(index: usize) -> Self {
        Self((index + 2) << 1)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddMinMatchType {
    Tsm,
    Osm,
    Osdm,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddNode {
    variable: BddVariable,
    then_edge: BddEdge,
    else_edge: BddEdge,
}

impl BddNode {
    pub fn variable(&self) -> BddVariable {
        self.variable
    }

    pub fn then_edge(&self) -> BddEdge {
        self.then_edge
    }

    pub fn else_edge(&self) -> BddEdge {
        self.else_edge
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddMinError {
    InvalidEdge(BddEdge),
    ZeroIsNotCube,
}

impl fmt::Display for BddMinError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEdge(edge) => write!(formatter, "invalid BDD edge {edge:?}"),
            Self::ZeroIsNotCube => write!(formatter, "the zero BDD is not a cube"),
        }
    }
}

impl Error for BddMinError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddMinMatchResult {
    pub function: BddEdge,
    pub care: BddEdge,
}

#[derive(Clone, Debug, Default)]
pub struct BddManager {
    nodes: Vec<BddNode>,
    unique_table: HashMap<(BddVariable, BddEdge, BddEdge), BddEdge>,
}

impl BddManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn zero(&self) -> BddEdge {
        BddEdge::ZERO
    }

    pub fn one(&self) -> BddEdge {
        BddEdge::ONE
    }

    pub fn not(&self, edge: BddEdge) -> BddEdge {
        edge.not()
    }

    pub fn variable(&mut self, variable: BddVariable) -> BddEdge {
        self.find_or_add(variable, self.one(), self.zero())
    }

    pub fn find_or_add(
        &mut self,
        variable: BddVariable,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> BddEdge {
        if then_edge == else_edge {
            return then_edge;
        }

        let key = (variable, then_edge, else_edge);
        if let Some(existing) = self.unique_table.get(&key).copied() {
            return existing;
        }

        let edge = BddEdge::from_node_index(self.nodes.len());
        self.nodes.push(BddNode {
            variable,
            then_edge,
            else_edge,
        });
        self.unique_table.insert(key, edge);

        edge
    }

    pub fn node(&self, edge: BddEdge) -> Result<Option<&BddNode>, BddMinError> {
        match edge.regular().node_index() {
            Some(index) => self
                .nodes
                .get(index)
                .map(Some)
                .ok_or(BddMinError::InvalidEdge(edge)),
            None => Ok(None),
        }
    }

    pub fn and(&mut self, left: BddEdge, right: BddEdge) -> Result<BddEdge, BddMinError> {
        self.ite(left, right, self.zero())
    }

    pub fn or(&mut self, left: BddEdge, right: BddEdge) -> Result<BddEdge, BddMinError> {
        self.ite(left, self.one(), right)
    }

    pub fn xor(&mut self, left: BddEdge, right: BddEdge) -> Result<BddEdge, BddMinError> {
        self.ite(left, right.not(), right)
    }

    pub fn implies(&mut self, premise: BddEdge, consequence: BddEdge) -> Result<bool, BddMinError> {
        let counterexample = self.and(premise, consequence.not())?;

        Ok(counterexample == self.zero())
    }

    pub fn is_cube(&self, edge: BddEdge) -> Result<bool, BddMinError> {
        if edge == self.zero() {
            return Err(BddMinError::ZeroIsNotCube);
        }

        self.is_cube_inner(edge)
    }

    pub fn eval(
        &self,
        root: BddEdge,
        assignment: &HashMap<BddVariable, bool>,
    ) -> Result<bool, BddMinError> {
        let mut edge = root;
        let mut complemented = false;

        loop {
            if edge == self.zero() {
                return Ok(complemented);
            }

            if edge == self.one() {
                return Ok(!complemented);
            }

            complemented ^= edge.is_complemented();
            let node = self
                .node(edge.regular())?
                .expect("non-constant edge must have a node");
            edge = if assignment.get(&node.variable).copied().unwrap_or(false) {
                node.then_edge
            } else {
                node.else_edge
            };
        }
    }

    pub fn ite(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, BddMinError> {
        self.validate_edge(condition)?;
        self.validate_edge(then_edge)?;
        self.validate_edge(else_edge)?;
        self.ite_inner(condition, then_edge, else_edge)
    }

    fn ite_inner(
        &mut self,
        condition: BddEdge,
        then_edge: BddEdge,
        else_edge: BddEdge,
    ) -> Result<BddEdge, BddMinError> {
        if condition == self.one() {
            return Ok(then_edge);
        }

        if condition == self.zero() {
            return Ok(else_edge);
        }

        if then_edge == else_edge {
            return Ok(then_edge);
        }

        if then_edge == self.one() && else_edge == self.zero() {
            return Ok(condition);
        }

        if then_edge == self.zero() && else_edge == self.one() {
            return Ok(condition.not());
        }

        let variable = self.top_variable(condition, then_edge, else_edge)?;
        let (condition_then, condition_else) = self.cofactor(condition, variable)?;
        let (then_then, then_else) = self.cofactor(then_edge, variable)?;
        let (else_then, else_else) = self.cofactor(else_edge, variable)?;

        let positive = self.ite_inner(condition_then, then_then, else_then)?;
        let negative = self.ite_inner(condition_else, then_else, else_else)?;

        Ok(self.find_or_add(variable, positive, negative))
    }

    fn is_cube_inner(&self, edge: BddEdge) -> Result<bool, BddMinError> {
        if edge == self.one() {
            return Ok(true);
        }

        if edge == self.zero() {
            return Ok(false);
        }

        let (then_edge, else_edge) = self.branches(edge)?;

        if then_edge == self.zero() {
            self.is_cube_inner(else_edge)
        } else if else_edge == self.zero() {
            self.is_cube_inner(then_edge)
        } else {
            Ok(false)
        }
    }

    fn top_variable(
        &self,
        first: BddEdge,
        second: BddEdge,
        third: BddEdge,
    ) -> Result<BddVariable, BddMinError> {
        Ok(self
            .variable_of(first)?
            .min(self.variable_of(second)?)
            .min(self.variable_of(third)?))
    }

    fn variable_of(&self, edge: BddEdge) -> Result<BddVariable, BddMinError> {
        Ok(self
            .node(edge.regular())?
            .map(|node| node.variable)
            .unwrap_or(BddVariable::MAX))
    }

    fn cofactor(
        &self,
        edge: BddEdge,
        variable: BddVariable,
    ) -> Result<(BddEdge, BddEdge), BddMinError> {
        let Some(node) = self.node(edge.regular())? else {
            return Ok((edge, edge));
        };

        if node.variable != variable {
            return Ok((edge, edge));
        }

        self.branches(edge)
    }

    fn branches(&self, edge: BddEdge) -> Result<(BddEdge, BddEdge), BddMinError> {
        let node = self
            .node(edge.regular())?
            .expect("non-constant edge must have a node");

        if edge.is_complemented() {
            Ok((node.then_edge.not(), node.else_edge.not()))
        } else {
            Ok((node.then_edge, node.else_edge))
        }
    }

    fn validate_edge(&self, edge: BddEdge) -> Result<(), BddMinError> {
        self.node(edge).map(|_| ())
    }
}

pub fn bdd_min_is_match(
    manager: &mut BddManager,
    match_type: BddMinMatchType,
    complement: bool,
    f1: BddEdge,
    c1: BddEdge,
    f2: BddEdge,
    c2: BddEdge,
) -> Result<bool, BddMinError> {
    if match_type == BddMinMatchType::Osdm {
        return Ok(c1 == manager.zero());
    }

    let diff = if complement {
        manager.xor(f1, f2)?.not()
    } else {
        manager.xor(f1, f2)?
    };

    match match_type {
        BddMinMatchType::Osm => {
            let diff_allowed = manager.implies(diff, c1.not())?;
            let care_allowed = manager.implies(c1, c2)?;

            Ok(diff_allowed && care_allowed)
        }
        BddMinMatchType::Tsm => {
            let not_c1_or_not_c2 = manager.ite(c1, c2.not(), manager.one())?;

            manager.implies(diff, not_c1_or_not_c2)
        }
        BddMinMatchType::Osdm => Ok(c1 == manager.zero()),
    }
}

pub fn bdd_min_match_result(
    manager: &mut BddManager,
    match_type: BddMinMatchType,
    complement: bool,
    f1: BddEdge,
    c1: BddEdge,
    f2: BddEdge,
    c2: BddEdge,
) -> Result<BddMinMatchResult, BddMinError> {
    match match_type {
        BddMinMatchType::Osm | BddMinMatchType::Osdm => Ok(BddMinMatchResult {
            function: f2,
            care: c2,
        }),
        BddMinMatchType::Tsm => {
            let selected_f1 = if complement { f1.not() } else { f1 };
            let first_part = manager.and(selected_f1, c1)?;
            let second_part = manager.and(f2, c2)?;

            Ok(BddMinMatchResult {
                function: manager.or(first_part, second_part)?,
                care: manager.or(c1, c2)?,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(entries: &[(BddVariable, bool)]) -> HashMap<BddVariable, bool> {
        entries.iter().copied().collect()
    }

    #[test]
    fn osdm_matches_only_when_original_care_is_zero() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let zero = manager.zero();
        let one = manager.one();

        assert!(
            bdd_min_is_match(
                &mut manager,
                BddMinMatchType::Osdm,
                false,
                x,
                zero,
                x.not(),
                one
            )
            .unwrap()
        );
        assert!(
            !bdd_min_is_match(&mut manager, BddMinMatchType::Osdm, false, x, one, x, one).unwrap()
        );
    }

    #[test]
    fn osm_requires_difference_outside_first_care_and_care_inclusion() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let not_y = y.not();

        assert!(bdd_min_is_match(&mut manager, BddMinMatchType::Osm, false, x, x, x, x).unwrap());
        assert!(
            !bdd_min_is_match(&mut manager, BddMinMatchType::Osm, false, x, x, not_y, x).unwrap()
        );
        assert!(!bdd_min_is_match(&mut manager, BddMinMatchType::Osm, false, x, x, x, y).unwrap());
    }

    #[test]
    fn tsm_permits_differences_outside_shared_care() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let not_y = y.not();
        let one = manager.one();

        assert!(bdd_min_is_match(&mut manager, BddMinMatchType::Tsm, false, x, x, y, y).unwrap());
        assert!(
            !bdd_min_is_match(
                &mut manager,
                BddMinMatchType::Tsm,
                false,
                x,
                one,
                not_y,
                one
            )
            .unwrap()
        );
    }

    #[test]
    fn complement_match_uses_negated_second_function() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);

        assert!(
            bdd_min_is_match(&mut manager, BddMinMatchType::Tsm, true, x, x, x.not(), x).unwrap()
        );
    }

    #[test]
    fn osm_and_osdm_results_take_second_pair() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let osm =
            bdd_min_match_result(&mut manager, BddMinMatchType::Osm, false, x, x, y, y).unwrap();
        let osdm =
            bdd_min_match_result(&mut manager, BddMinMatchType::Osdm, true, x, x, y, y).unwrap();

        assert_eq!(
            osm,
            BddMinMatchResult {
                function: y,
                care: y
            }
        );
        assert_eq!(
            osdm,
            BddMinMatchResult {
                function: y,
                care: y
            }
        );
    }

    #[test]
    fn tsm_result_unions_selected_functions_and_cares() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);

        let result =
            bdd_min_match_result(&mut manager, BddMinMatchType::Tsm, false, x, x, y, y).unwrap();

        assert!(
            manager
                .eval(result.function, &values(&[(0, true), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .eval(result.function, &values(&[(0, false), (1, true)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result.function, &values(&[(0, false), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .eval(result.care, &values(&[(0, true), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .eval(result.care, &values(&[(0, false), (1, true)]))
                .unwrap()
        );
    }

    #[test]
    fn tsm_result_honors_complement_flag_for_first_function() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let one = manager.one();

        let result =
            bdd_min_match_result(&mut manager, BddMinMatchType::Tsm, true, x, one, y, y).unwrap();

        assert!(
            manager
                .eval(result.function, &values(&[(0, false), (1, false)]))
                .unwrap()
        );
        assert!(
            !manager
                .eval(result.function, &values(&[(0, true), (1, false)]))
                .unwrap()
        );
        assert!(
            manager
                .eval(result.function, &values(&[(0, true), (1, true)]))
                .unwrap()
        );
    }

    #[test]
    fn detects_single_path_cubes() {
        let mut manager = BddManager::new();
        let x = manager.variable(0);
        let y = manager.variable(1);
        let cube = manager.and(x, y.not()).unwrap();
        let non_cube = manager.or(x, y).unwrap();

        assert!(manager.is_cube(manager.one()).unwrap());
        assert!(manager.is_cube(cube).unwrap());
        assert!(!manager.is_cube(non_cube).unwrap());
        assert_eq!(
            manager.is_cube(manager.zero()),
            Err(BddMinError::ZeroIsNotCube)
        );
    }

    #[test]
    fn source_contains_no_legacy_exports_or_tracking_metadata() {
        let source = include_str!("bdd_min_util.rs");

        assert!(!source.contains(&["no", "_mangle"].concat()));
        assert!(!source.contains("extern \"C\""));
        assert!(!source.contains(&["REQUIRED", "_"].concat()));
        assert!(!source.contains(&["Port", "Dependency"].concat()));
        assert!(!source.contains(&["bead", "_id"].concat()));
        assert!(!source.contains(&["source", "_file"].concat()));
    }
}
