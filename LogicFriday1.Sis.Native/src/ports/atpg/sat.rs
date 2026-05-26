//! Native Rust port of `LogicSynthesis/sis/atpg/sat.c`.
//!
//! SIS represents a SAT variable as two adjacent literal IDs. The even literal
//! returned by [`SatSolver::new_variable`] is the positive phase and `id ^ 1`
//! is its complement. This module keeps that representation while replacing
//! sparse-matrix rows, columns, and AVL implication tables with owned Rust
//! collections.

use std::collections::BTreeSet;

const ASIZE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SatResult {
    Solved,
    Absurd,
    GaveUp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SatValue {
    False,
    True,
    Unbound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SatError {
    InvalidLiteral(usize),
    EmptyClause,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Strategy {
    var_order: usize,
    bktrack_lim: usize,
    n_static_pass: usize,
    add_unique: bool,
    add_nli: bool,
}

const STRATEGIES: [Strategy; 12] = [
    Strategy {
        var_order: 1,
        bktrack_lim: 15,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
    Strategy {
        var_order: 2,
        bktrack_lim: 15,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
    Strategy {
        var_order: 4,
        bktrack_lim: 15,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
    Strategy {
        var_order: 5,
        bktrack_lim: 15,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
    Strategy {
        var_order: 1,
        bktrack_lim: 100,
        n_static_pass: 10,
        add_unique: true,
        add_nli: true,
    },
    Strategy {
        var_order: 2,
        bktrack_lim: 100,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
    Strategy {
        var_order: 4,
        bktrack_lim: 100,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
    Strategy {
        var_order: 5,
        bktrack_lim: 100,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
    Strategy {
        var_order: 1,
        bktrack_lim: 500,
        n_static_pass: 10,
        add_unique: true,
        add_nli: true,
    },
    Strategy {
        var_order: 2,
        bktrack_lim: 500,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
    Strategy {
        var_order: 4,
        bktrack_lim: 500,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
    Strategy {
        var_order: 5,
        bktrack_lim: 500,
        n_static_pass: 0,
        add_unique: false,
        add_nli: false,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StackStatus {
    tos_var: usize,
    tos_inc: usize,
    tos_cla: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Clause {
    literals: Vec<usize>,
    unbound: isize,
}

impl Clause {
    fn new(literals: Vec<usize>) -> Self {
        let unbound = literals.len() as isize;

        Self { literals, unbound }
    }

    fn satisfies(&self) -> bool {
        self.unbound < 0
    }

    fn n_unbound(&self) -> isize {
        self.unbound
    }

    fn dec_unbound(&mut self) {
        self.unbound -= 1;
    }

    fn inc_unbound(&mut self) {
        self.unbound += 1;
    }

    fn inv_unbound(&mut self) {
        self.unbound = -self.unbound;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SatSolver {
    clauses: Vec<Clause>,
    occurrences: Vec<Vec<usize>>,
    implications: Vec<BTreeSet<usize>>,
    assigned: Vec<bool>,
    one_clauses: Vec<usize>,
    lit_index: usize,
    stk_var: Vec<usize>,
    stk_inc: Vec<usize>,
    stk_cla: Vec<usize>,
    bktrack: usize,
    gaveup: bool,
    n_impl: isize,
    strategy: Strategy,
}

impl Default for SatSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SatSolver {
    pub fn new() -> Self {
        Self {
            clauses: Vec::new(),
            occurrences: Vec::new(),
            implications: Vec::new(),
            assigned: Vec::new(),
            one_clauses: Vec::with_capacity(ASIZE),
            lit_index: 0,
            stk_var: Vec::with_capacity(ASIZE),
            stk_inc: Vec::with_capacity(ASIZE),
            stk_cla: Vec::with_capacity(ASIZE),
            bktrack: 0,
            gaveup: false,
            n_impl: -1,
            strategy: STRATEGIES[0],
        }
    }

    pub fn reset(&mut self) {
        self.clauses.clear();
        self.occurrences.clear();
        self.implications.clear();
        self.assigned.clear();
        self.one_clauses.clear();
        self.lit_index = 0;
        self.stk_var.clear();
        self.stk_inc.clear();
        self.stk_cla.clear();
        self.bktrack = 0;
        self.gaveup = false;
        self.n_impl = -1;
        self.strategy = STRATEGIES[0];
    }

    pub fn new_variable(&mut self) -> usize {
        let literal = self.lit_index;
        self.create_literal(literal);
        self.create_literal(literal + 1);
        self.lit_index += 2;
        literal
    }

    pub fn add_clause(&mut self, literals: &[usize]) -> Result<usize, SatError> {
        if literals.is_empty() {
            self.clauses.push(Clause::new(Vec::new()));
            return Err(SatError::EmptyClause);
        }

        for literal in literals {
            self.ensure_valid_literal(*literal)?;
        }

        let clause_index = self.clauses.len();
        let mut normalized = literals.to_vec();
        normalized.sort_unstable();
        normalized.dedup();

        for literal in &normalized {
            self.occurrences[*literal].push(clause_index);
        }

        self.clauses.push(Clause::new(normalized));
        Ok(clause_index)
    }

    pub fn add_implication(&mut self, var1: usize, var2: usize) -> Result<bool, SatError> {
        self.ensure_valid_literal(var1)?;
        self.ensure_valid_literal(var2)?;

        if var1 == var2 {
            return Ok(false);
        }

        Ok(self.implications[var1].insert(var2))
    }

    pub fn solve(&mut self, fast_sat: bool) -> SatResult {
        self.prepare_for_solve();

        for clause_index in 0..self.clauses.len() {
            let length = self.clauses[clause_index].literals.len();
            self.clauses[clause_index].unbound = length as isize;

            if length == 1 {
                self.clauses[clause_index].inv_unbound();
                self.one_clauses
                    .push(self.clauses[clause_index].literals[0]);
            } else if length == 0 {
                return SatResult::Absurd;
            }
        }

        let one_clauses = self.one_clauses.clone();
        for literal in one_clauses {
            if !self.fix_literal(literal) {
                return SatResult::Absurd;
            }
        }

        let strategy_count = if fast_sat { 4 } else { STRATEGIES.len() };
        let mut result = SatResult::GaveUp;

        for strategy in STRATEGIES.iter().take(strategy_count) {
            if result != SatResult::GaveUp {
                break;
            }

            self.strategy = *strategy;
            result = self.branch_n_bound();
        }

        result
    }

    pub fn value(&self, id: usize) -> SatValue {
        if self.is_literal_assigned(id) {
            SatValue::True
        } else if self.is_literal_assigned(neg(id)) {
            SatValue::False
        } else {
            SatValue::Unbound
        }
    }

    pub fn variable_count(&self) -> usize {
        self.lit_index / 2
    }

    pub fn clause_count(&self) -> usize {
        self.clauses.len()
    }

    fn create_literal(&mut self, literal: usize) {
        if literal >= self.assigned.len() {
            self.assigned.resize(literal + 1, false);
            self.implications.resize_with(literal + 1, BTreeSet::new);
            self.occurrences.resize_with(literal + 1, Vec::new);
        }
    }

    fn ensure_valid_literal(&self, literal: usize) -> Result<(), SatError> {
        if literal < self.lit_index {
            Ok(())
        } else {
            Err(SatError::InvalidLiteral(literal))
        }
    }

    fn prepare_for_solve(&mut self) {
        self.assigned.fill(false);
        self.one_clauses.clear();
        self.stk_var.clear();
        self.stk_inc.clear();
        self.stk_cla.clear();
        self.bktrack = 0;
        self.gaveup = false;
        self.n_impl = -1;
    }

    fn save_tos(&self) -> StackStatus {
        StackStatus {
            tos_var: self.stk_var.len(),
            tos_inc: self.stk_inc.len(),
            tos_cla: self.stk_cla.len(),
        }
    }

    fn find_next_lit(&self, next_clause: &mut Option<usize>) -> Option<usize> {
        let order = self.strategy.var_order;
        let mut clause_index = *next_clause;

        match order {
            1 | 2 => {
                while let Some(index) = clause_index {
                    if self.clauses[index].satisfies() {
                        clause_index = self.next_forward_clause(index);
                        continue;
                    }

                    let best = self.best_literal_in_clause(index, order == 1);
                    *next_clause = Some(index);
                    return best;
                }
            }
            4 | 5 => {
                while let Some(index) = clause_index {
                    if self.clauses[index].satisfies() {
                        clause_index = index.checked_sub(1);
                        continue;
                    }

                    let best = self.best_literal_in_clause(index, order == 4);
                    *next_clause = Some(index);
                    return best;
                }
            }
            _ => unreachable!("unknown SAT clause/variable ordering"),
        }

        *next_clause = clause_index;
        None
    }

    fn best_literal_in_clause(&self, clause_index: usize, first: bool) -> Option<usize> {
        let mut best = None;

        for literal in &self.clauses[clause_index].literals {
            if !self.is_literal_assigned(neg(*literal)) {
                best = Some(*literal);

                if first {
                    break;
                }
            }
        }

        best
    }

    fn bound(&mut self, literal: usize, add_impl: bool) -> bool {
        let bot = self.stk_var.len();

        if !self.push_assignment(literal) {
            return false;
        }

        let mut cursor = bot;
        while cursor < self.stk_var.len() {
            let lit = self.stk_var[cursor];
            cursor += 1;

            let implied_literals: Vec<usize> = self.implications[lit].iter().copied().collect();
            for imp_lit in implied_literals {
                if self.is_literal_assigned(neg(imp_lit)) {
                    return false;
                }

                if !self.push_assignment(imp_lit) {
                    return false;
                }
            }

            let complement_occurrences = self.occurrences[neg(lit)].clone();
            for clause_index in complement_occurrences {
                if self.clauses[clause_index].satisfies() {
                    continue;
                }

                if self.clauses[clause_index].n_unbound() > 2 {
                    self.stk_inc.push(clause_index);
                    self.clauses[clause_index].dec_unbound();
                    continue;
                }

                let mut implied_one = false;
                let clause_literals = self.clauses[clause_index].literals.clone();

                for candidate in clause_literals {
                    if self.is_literal_assigned(candidate) {
                        implied_one = true;
                        break;
                    }

                    if !self.is_literal_assigned(neg(candidate)) {
                        implied_one = true;

                        if !self.push_assignment(candidate) {
                            return false;
                        }

                        if add_impl && self.add_learned_implication(neg(candidate), neg(literal)) {
                            self.n_impl += 1;
                        }

                        break;
                    }
                }

                if !implied_one {
                    return false;
                }
            }

            let lit_occurrences = self.occurrences[lit].clone();
            for clause_index in lit_occurrences {
                if self.clauses[clause_index].n_unbound() > 0 {
                    self.stk_cla.push(clause_index);
                    self.clauses[clause_index].inv_unbound();
                }
            }
        }

        true
    }

    fn undo_assignment(&mut self, prev_status: StackStatus) {
        for index in (prev_status.tos_var..self.stk_var.len()).rev() {
            let literal = self.stk_var[index];
            self.assigned[literal] = false;
        }

        for index in (prev_status.tos_cla..self.stk_cla.len()).rev() {
            let clause_index = self.stk_cla[index];
            self.clauses[clause_index].inv_unbound();
        }

        for index in (prev_status.tos_inc..self.stk_inc.len()).rev() {
            let clause_index = self.stk_inc[index];
            self.clauses[clause_index].inc_unbound();
        }

        self.stk_var.truncate(prev_status.tos_var);
        self.stk_inc.truncate(prev_status.tos_inc);
        self.stk_cla.truncate(prev_status.tos_cla);
    }

    fn branch(&mut self, mut next_clause: Option<usize>) -> bool {
        let Some(literal) = self.find_next_lit(&mut next_clause) else {
            return true;
        };

        let prev_tos = self.save_tos();

        if self.bound(literal, false) && self.branch(next_clause) {
            return true;
        }

        self.bktrack += 1;
        if self.bktrack > self.strategy.bktrack_lim {
            self.gaveup = true;
            return true;
        }

        self.undo_assignment(prev_tos);

        self.bound(neg(literal), false) && self.branch(next_clause)
    }

    fn find_impl(&mut self) -> bool {
        self.n_impl = 0;

        for literal in 0..self.lit_index {
            if self.is_literal_assigned(literal)
                || self.is_literal_assigned(neg(literal))
                || self.implications[literal].is_empty()
            {
                continue;
            }

            let prev_tos = self.save_tos();
            let result = self.bound(literal, self.strategy.add_nli);
            self.undo_assignment(prev_tos);

            if !result && self.strategy.add_unique {
                self.n_impl += 1;
                if !self.fix_literal(neg(literal)) {
                    return false;
                }
            }
        }

        true
    }

    fn branch_n_bound(&mut self) -> SatResult {
        self.bktrack = 0;
        self.gaveup = false;
        let mut bnb_status = true;

        if self.n_impl != 0 && self.strategy.n_static_pass != 0 {
            let mut pass = 0;
            while pass < self.strategy.n_static_pass && bnb_status && self.n_impl != 0 {
                pass += 1;
                bnb_status = self.find_impl();
            }
        }

        let prev_tos = self.save_tos();
        let mut start_clause = if self.strategy.var_order < 4 {
            self.first_clause()
        } else {
            self.last_clause()
        };

        while bnb_status {
            let Some(literal) = self.find_next_lit(&mut start_clause) else {
                break;
            };

            if self.bound(literal, false) && self.branch(start_clause) {
                break;
            }

            self.undo_assignment(prev_tos);
            bnb_status = self.fix_literal(neg(literal));
        }

        if self.gaveup {
            self.undo_assignment(prev_tos);
            SatResult::GaveUp
        } else if bnb_status {
            SatResult::Solved
        } else {
            SatResult::Absurd
        }
    }

    fn fix_literal(&mut self, literal: usize) -> bool {
        if self.is_literal_assigned(literal) {
            return true;
        }

        if self.is_literal_assigned(neg(literal)) || !self.bound(literal, false) {
            return false;
        }

        self.stk_var.clear();
        self.stk_inc.clear();
        self.stk_cla.clear();
        true
    }

    fn push_assignment(&mut self, literal: usize) -> bool {
        if self.is_literal_assigned(neg(literal)) {
            return false;
        }

        if !self.assigned[literal] {
            self.stk_var.push(literal);
            self.assigned[literal] = true;
        }

        true
    }

    fn add_learned_implication(&mut self, from: usize, to: usize) -> bool {
        from != to && self.implications[from].insert(to)
    }

    fn is_literal_assigned(&self, literal: usize) -> bool {
        self.assigned.get(literal).copied().unwrap_or(false)
    }

    fn first_clause(&self) -> Option<usize> {
        (!self.clauses.is_empty()).then_some(0)
    }

    fn last_clause(&self) -> Option<usize> {
        self.clauses.len().checked_sub(1)
    }

    fn next_forward_clause(&self, index: usize) -> Option<usize> {
        let next = index + 1;
        (next < self.clauses.len()).then_some(next)
    }
}

pub fn neg(literal: usize) -> usize {
    literal ^ 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_adjacent_literal_pairs() {
        let mut sat = SatSolver::new();

        let a = sat.new_variable();
        let b = sat.new_variable();

        assert_eq!(a, 0);
        assert_eq!(neg(a), 1);
        assert_eq!(b, 2);
        assert_eq!(neg(b), 3);
        assert_eq!(sat.variable_count(), 2);
    }

    #[test]
    fn solves_formula_with_unit_propagation() {
        let mut sat = SatSolver::new();
        let a = sat.new_variable();
        let b = sat.new_variable();

        sat.add_clause(&[a]).unwrap();
        sat.add_clause(&[neg(a), b]).unwrap();

        assert_eq!(sat.solve(true), SatResult::Solved);
        assert_eq!(sat.value(a), SatValue::True);
        assert_eq!(sat.value(b), SatValue::True);
    }

    #[test]
    fn detects_conflicting_unit_clauses() {
        let mut sat = SatSolver::new();
        let a = sat.new_variable();

        sat.add_clause(&[a]).unwrap();
        sat.add_clause(&[neg(a)]).unwrap();

        assert_eq!(sat.solve(false), SatResult::Absurd);
    }

    #[test]
    fn detects_unsatisfiable_two_variable_formula() {
        let mut sat = SatSolver::new();
        let a = sat.new_variable();
        let b = sat.new_variable();

        sat.add_clause(&[a, b]).unwrap();
        sat.add_clause(&[a, neg(b)]).unwrap();
        sat.add_clause(&[neg(a), b]).unwrap();
        sat.add_clause(&[neg(a), neg(b)]).unwrap();

        assert_eq!(sat.solve(false), SatResult::Absurd);
    }

    #[test]
    fn honors_explicit_implications() {
        let mut sat = SatSolver::new();
        let a = sat.new_variable();
        let b = sat.new_variable();

        assert!(sat.add_implication(a, b).unwrap());
        sat.add_clause(&[a]).unwrap();

        assert_eq!(sat.solve(true), SatResult::Solved);
        assert_eq!(sat.value(a), SatValue::True);
        assert_eq!(sat.value(b), SatValue::True);
    }

    #[test]
    fn rejects_invalid_literals() {
        let mut sat = SatSolver::new();
        let a = sat.new_variable();

        assert_eq!(
            sat.add_clause(&[a + 2]),
            Err(SatError::InvalidLiteral(a + 2))
        );
        assert_eq!(
            sat.add_implication(a, a + 2),
            Err(SatError::InvalidLiteral(a + 2))
        );
    }

    #[test]
    fn empty_clause_is_absurd() {
        let mut sat = SatSolver::new();

        assert_eq!(sat.add_clause(&[]), Err(SatError::EmptyClause));
        assert_eq!(sat.clause_count(), 1);
        assert_eq!(sat.solve(true), SatResult::Absurd);
    }

    #[test]
    fn reset_discards_problem_state() {
        let mut sat = SatSolver::new();
        let a = sat.new_variable();
        sat.add_clause(&[a]).unwrap();

        sat.reset();

        assert_eq!(sat.variable_count(), 0);
        assert_eq!(sat.clause_count(), 0);
        assert_eq!(sat.solve(true), SatResult::Solved);
    }
}
