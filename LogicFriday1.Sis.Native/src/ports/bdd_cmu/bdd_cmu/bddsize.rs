//! Native Rust node-size and profile routines for the CMU BDD package.
//!
//! The original routines use tagged BDD pointers and temporary node marks.
//! This port keeps the same observable counting rules while representing the
//! graph with typed handles and local traversal state.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddRef(usize);

impl BddRef {
    pub const ONE: Self = Self(0);
    pub const ZERO: Self = Self(1);

    pub const fn is_complemented(self) -> bool {
        self.0 & 1 == 1
    }

    pub const fn complement(self) -> Self {
        Self(self.0 ^ 1)
    }

    fn regular(self) -> Self {
        Self(self.0 & !1)
    }

    fn mark_key(self) -> usize {
        self.regular().0
    }

    fn phase_bit(self) -> u8 {
        1 << (self.0 & 1)
    }

    fn branch_index(self) -> Option<usize> {
        let regular = self.regular().0;
        if regular == 0 {
            None
        } else {
            Some((regular >> 1) - 1)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BddBranch {
    pub variable: usize,
    pub then_ref: BddRef,
    pub else_ref: BddRef,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BddManager {
    variable_count: usize,
    branches: Vec<BddBranch>,
}

impl BddManager {
    pub fn new(variable_count: usize) -> Self {
        Self {
            variable_count,
            branches: Vec::new(),
        }
    }

    pub fn variable_count(&self) -> usize {
        self.variable_count
    }

    pub fn one(&self) -> BddRef {
        BddRef::ONE
    }

    pub fn zero(&self) -> BddRef {
        BddRef::ZERO
    }

    pub fn add_branch(
        &mut self,
        variable: usize,
        then_ref: BddRef,
        else_ref: BddRef,
    ) -> Result<BddRef, BddSizeError> {
        if variable >= self.variable_count {
            return Err(BddSizeError::VariableOutOfRange {
                variable,
                variable_count: self.variable_count,
            });
        }

        self.validate_ref(then_ref)?;
        self.validate_ref(else_ref)?;

        let reference = BddRef((self.branches.len() + 1) << 1);
        self.branches.push(BddBranch {
            variable,
            then_ref,
            else_ref,
        });

        Ok(reference)
    }

    pub fn branch(&self, reference: BddRef) -> Result<Option<BddBranch>, BddSizeError> {
        let Some(index) = reference.branch_index() else {
            return Ok(None);
        };

        let branch = *self
            .branches
            .get(index)
            .ok_or(BddSizeError::MissingBranch(index))?;

        if reference.is_complemented() {
            Ok(Some(BddBranch {
                variable: branch.variable,
                then_ref: branch.then_ref.complement(),
                else_ref: branch.else_ref.complement(),
            }))
        } else {
            Ok(Some(branch))
        }
    }

    fn validate_ref(&self, reference: BddRef) -> Result<(), BddSizeError> {
        if let Some(index) = reference.branch_index() {
            if index >= self.branches.len() {
                return Err(BddSizeError::MissingBranch(index));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeCountingMode {
    CountLogicalNodes,
    CountOutputPhases,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddSizeError {
    MissingBranch(usize),
    VariableOutOfRange {
        variable: usize,
        variable_count: usize,
    },
    ProfileLength {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for BddSizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBranch(index) => {
                write!(formatter, "BDD branch {index} was not found")
            }
            Self::VariableOutOfRange {
                variable,
                variable_count,
            } => write!(
                formatter,
                "BDD variable {variable} is outside the manager range 0..{variable_count}",
            ),
            Self::ProfileLength { expected, actual } => write!(
                formatter,
                "profile length {actual} does not match expected length {expected}",
            ),
        }
    }
}

impl Error for BddSizeError {}

pub fn size(
    manager: &BddManager,
    root: BddRef,
    mode: NodeCountingMode,
) -> Result<i64, BddSizeError> {
    size_multiple(manager, &[root], mode)
}

pub fn size_multiple(
    manager: &BddManager,
    roots: &[BddRef],
    mode: NodeCountingMode,
) -> Result<i64, BddSizeError> {
    let marks = mark_roots(manager, roots)?;

    Ok(marks.values().map(|mark| count_mark(*mark, mode)).sum())
}

pub fn profile(
    manager: &BddManager,
    root: BddRef,
    mode: NodeCountingMode,
) -> Result<Vec<i64>, BddSizeError> {
    profile_multiple(manager, &[root], mode)
}

pub fn profile_into(
    manager: &BddManager,
    root: BddRef,
    level_counts: &mut [i64],
    mode: NodeCountingMode,
) -> Result<(), BddSizeError> {
    profile_multiple_into(manager, &[root], level_counts, mode)
}

pub fn profile_multiple(
    manager: &BddManager,
    roots: &[BddRef],
    mode: NodeCountingMode,
) -> Result<Vec<i64>, BddSizeError> {
    let mut level_counts = vec![0; manager.variable_count() + 1];
    profile_multiple_into(manager, roots, &mut level_counts, mode)?;

    Ok(level_counts)
}

pub fn profile_multiple_into(
    manager: &BddManager,
    roots: &[BddRef],
    level_counts: &mut [i64],
    mode: NodeCountingMode,
) -> Result<(), BddSizeError> {
    validate_profile_len(manager, level_counts)?;
    level_counts.fill(0);

    let marks = mark_roots(manager, roots)?;
    for (key, mark) in marks {
        let level = if key == BddRef::ONE.mark_key() {
            manager.variable_count()
        } else {
            let index = (key >> 1) - 1;
            manager
                .branches
                .get(index)
                .ok_or(BddSizeError::MissingBranch(index))?
                .variable
        };

        level_counts[level] += count_mark(mark, mode);
    }

    Ok(())
}

pub fn function_profile(manager: &BddManager, root: BddRef) -> Result<Vec<i64>, BddSizeError> {
    function_profile_multiple(manager, &[root])
}

pub fn function_profile_into(
    manager: &BddManager,
    root: BddRef,
    function_counts: &mut [i64],
) -> Result<(), BddSizeError> {
    function_profile_multiple_into(manager, &[root], function_counts)
}

pub fn function_profile_multiple(
    manager: &BddManager,
    roots: &[BddRef],
) -> Result<Vec<i64>, BddSizeError> {
    let mut function_counts = vec![0; manager.variable_count() + 1];
    function_profile_multiple_into(manager, roots, &mut function_counts)?;

    Ok(function_counts)
}

pub fn function_profile_multiple_into(
    manager: &BddManager,
    roots: &[BddRef],
    function_counts: &mut [i64],
) -> Result<(), BddSizeError> {
    validate_profile_len(manager, function_counts)?;
    profile_multiple_into(
        manager,
        roots,
        function_counts,
        NodeCountingMode::CountOutputPhases,
    )?;

    for count in &mut function_counts[..manager.variable_count()] {
        if *count == 0 {
            *count = 1;
        } else {
            *count <<= 1;
        }
    }

    let mut highest_refs = HashMap::new();
    for root in roots {
        highest_ref_step(manager, *root, &mut highest_refs)?;
    }

    for root in roots {
        highest_refs.insert(*root, -1);
    }

    let mut dominated = HashSet::new();
    for root in roots {
        dominated_step(
            manager,
            *root,
            function_counts,
            &mut highest_refs,
            &mut dominated,
        )?;
    }

    let mut carry_level = manager.variable_count();
    for level in (0..manager.variable_count()).rev() {
        if function_counts[level] == 1 {
            function_counts[level] = 0;
        } else {
            function_counts[level] = (function_counts[level] >> 1) + function_counts[carry_level];
            carry_level = level;
        }
    }

    Ok(())
}

fn mark_roots(manager: &BddManager, roots: &[BddRef]) -> Result<HashMap<usize, u8>, BddSizeError> {
    let mut marks = HashMap::new();
    for root in roots {
        manager.validate_ref(*root)?;
        mark_bdd(manager, *root, &mut marks)?;
    }

    Ok(marks)
}

fn mark_bdd(
    manager: &BddManager,
    reference: BddRef,
    marks: &mut HashMap<usize, u8>,
) -> Result<(), BddSizeError> {
    let entry = marks.entry(reference.mark_key()).or_insert(0);
    let phase = reference.phase_bit();
    if *entry & phase != 0 {
        return Ok(());
    }

    *entry |= phase;
    if let Some(branch) = manager.branch(reference)? {
        mark_bdd(manager, branch.then_ref, marks)?;
        mark_bdd(manager, branch.else_ref, marks)?;
    }

    Ok(())
}

fn highest_ref_step(
    manager: &BddManager,
    reference: BddRef,
    highest_refs: &mut HashMap<BddRef, i64>,
) -> Result<(), BddSizeError> {
    let Some(branch) = manager.branch(reference)? else {
        return Ok(());
    };

    let variable = branch.variable as i64;
    update_highest_ref(manager, branch.then_ref, variable, highest_refs)?;
    update_highest_ref(manager, branch.else_ref, variable, highest_refs)?;

    Ok(())
}

fn update_highest_ref(
    manager: &BddManager,
    child: BddRef,
    parent_variable: i64,
    highest_refs: &mut HashMap<BddRef, i64>,
) -> Result<(), BddSizeError> {
    if let Some(existing) = highest_refs.get_mut(&child) {
        if *existing > parent_variable {
            *existing = parent_variable;
        }
    } else {
        highest_refs.insert(child, parent_variable);
        highest_ref_step(manager, child, highest_refs)?;
    }

    Ok(())
}

fn dominated_step(
    manager: &BddManager,
    reference: BddRef,
    function_counts: &mut [i64],
    highest_refs: &mut HashMap<BddRef, i64>,
    dominated: &mut HashSet<BddRef>,
) -> Result<(), BddSizeError> {
    let highest_ref = *highest_refs.get(&reference).unwrap_or(&-1);
    if highest_ref >= 0 {
        function_counts[highest_ref as usize] -= 2;
    }

    if highest_ref <= -2 || !dominated.insert(reference) {
        return Ok(());
    }

    highest_refs.insert(reference, -2);
    if let Some(branch) = manager.branch(reference)? {
        dominated_step(
            manager,
            branch.then_ref,
            function_counts,
            highest_refs,
            dominated,
        )?;
        dominated_step(
            manager,
            branch.else_ref,
            function_counts,
            highest_refs,
            dominated,
        )?;
    }

    Ok(())
}

fn count_mark(mark: u8, mode: NodeCountingMode) -> i64 {
    match mode {
        NodeCountingMode::CountLogicalNodes => i64::from(mark != 0),
        NodeCountingMode::CountOutputPhases => i64::from(mark & 1 != 0) + i64::from(mark & 2 != 0),
    }
}

fn validate_profile_len(manager: &BddManager, counts: &[i64]) -> Result<(), BddSizeError> {
    let expected = manager.variable_count() + 1;
    if counts.len() != expected {
        return Err(BddSizeError::ProfileLength {
            expected,
            actual: counts.len(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xy_manager() -> (BddManager, BddRef, BddRef) {
        let mut manager = BddManager::new(3);
        let y = manager
            .add_branch(1, manager.one(), manager.zero())
            .unwrap();
        let x = manager.add_branch(0, y, manager.zero()).unwrap();

        (manager, x, y)
    }

    #[test]
    fn size_counts_each_reachable_logical_node_once() {
        let (manager, x, _) = xy_manager();

        assert_eq!(
            size(&manager, x, NodeCountingMode::CountLogicalNodes).unwrap(),
            3,
        );
    }

    #[test]
    fn size_can_expand_observed_output_phases() {
        let (manager, x, _) = xy_manager();

        assert_eq!(
            size(&manager, x, NodeCountingMode::CountOutputPhases).unwrap(),
            4,
        );
    }

    #[test]
    fn size_multiple_accounts_for_shared_nodes() {
        let mut manager = BddManager::new(2);
        let y = manager
            .add_branch(1, manager.one(), manager.zero())
            .unwrap();
        let x = manager.add_branch(0, y, y.complement()).unwrap();

        assert_eq!(
            size_multiple(&manager, &[x, y], NodeCountingMode::CountLogicalNodes,).unwrap(),
            3,
        );
        assert_eq!(
            size_multiple(&manager, &[x, y], NodeCountingMode::CountOutputPhases,).unwrap(),
            5,
        );
    }

    #[test]
    fn profile_groups_counts_by_variable_and_terminal_level() {
        let (manager, x, _) = xy_manager();

        assert_eq!(
            profile(&manager, x, NodeCountingMode::CountLogicalNodes).unwrap(),
            vec![1, 1, 0, 1],
        );
        assert_eq!(
            profile(&manager, x, NodeCountingMode::CountOutputPhases).unwrap(),
            vec![1, 1, 0, 2],
        );
    }

    #[test]
    fn profile_into_clears_previous_counts_and_validates_length() {
        let (manager, x, _) = xy_manager();
        let mut counts = vec![99; 4];

        profile_into(
            &manager,
            x,
            &mut counts,
            NodeCountingMode::CountLogicalNodes,
        )
        .unwrap();

        assert_eq!(counts, vec![1, 1, 0, 1]);
        assert_eq!(
            profile_into(
                &manager,
                x,
                &mut counts[..3],
                NodeCountingMode::CountLogicalNodes,
            )
            .unwrap_err(),
            BddSizeError::ProfileLength {
                expected: 4,
                actual: 3,
            },
        );
    }

    #[test]
    fn function_profile_reports_subfunctions_bottom_up() {
        let (manager, x, _) = xy_manager();

        assert_eq!(function_profile(&manager, x).unwrap(), vec![1, 2, 0, 2]);
    }

    #[test]
    fn function_profile_multiple_marks_roots_as_top_references() {
        let (manager, x, y) = xy_manager();

        assert_eq!(
            function_profile_multiple(&manager, &[x, y.complement()]).unwrap(),
            vec![2, 3, 0, 2],
        );
    }

    #[test]
    fn constant_profiles_keep_the_terminal_bucket() {
        let manager = BddManager::new(2);

        assert_eq!(
            profile(&manager, manager.one(), NodeCountingMode::CountOutputPhases,).unwrap(),
            vec![0, 0, 1],
        );
        assert_eq!(
            function_profile(&manager, manager.one()).unwrap(),
            vec![0, 0, 1]
        );
    }

    #[test]
    fn manager_rejects_invalid_variables_and_children() {
        let mut manager = BddManager::new(1);

        assert_eq!(
            manager
                .add_branch(3, manager.one(), manager.zero())
                .unwrap_err(),
            BddSizeError::VariableOutOfRange {
                variable: 3,
                variable_count: 1,
            },
        );
        assert_eq!(
            manager
                .add_branch(0, BddRef(20), manager.zero())
                .unwrap_err(),
            BddSizeError::MissingBranch(9),
        );
    }
}
