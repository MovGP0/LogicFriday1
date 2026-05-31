//! Native Rust port of `LogicSynthesis/sis/retime/re_simplx.c`.
//!
//! The original C file is the Numerical Recipes simplex routine used by the
//! retiming LP builders. This module keeps the solver native and in-process:
//! the tableau is a zero-based Rust matrix with the C layout preserved
//! semantically: row 0 is the objective, rows `1..=m` are constraints, row
//! `m + 1` is the auxiliary objective, column 0 is the RHS, and columns
//! `1..=n` are variables.
//!
//! Direct graph-to-tableau and ASTG-to-tableau integrations remain explicit
//! dependency errors because those C units own problem construction around this
//! solver. No legacy C ABI entry points are exposed here.

use std::error::Error;
use std::fmt;

pub const SIMPLEX_EPSILON: f64 = 1.0e-6;

pub fn retime_min_register_simplex_blocked() -> Result<(), SimplexError> {
    Err(SimplexError::MissingIntegrationDependencies {
        operation: "retime minimum-register LP construction",
    })
}

pub fn astg_backward_lp_simplex_blocked() -> Result<(), SimplexError> {
    Err(SimplexError::MissingIntegrationDependencies {
        operation: "ASTG backward LP construction",
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimplexCase {
    Optimal,
    Unbounded,
    Infeasible,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimplexProblem {
    pub tableau: Vec<Vec<f64>>,
    pub variables: usize,
    pub less_equal_constraints: usize,
    pub greater_equal_constraints: usize,
    pub equality_constraints: usize,
}

impl SimplexProblem {
    pub fn new(
        tableau: Vec<Vec<f64>>,
        variables: usize,
        less_equal_constraints: usize,
        greater_equal_constraints: usize,
        equality_constraints: usize,
    ) -> Self {
        Self {
            tableau,
            variables,
            less_equal_constraints,
            greater_equal_constraints,
            equality_constraints,
        }
    }

    pub fn constraints(&self) -> usize {
        self.less_equal_constraints + self.greater_equal_constraints + self.equality_constraints
    }

    pub fn validate(&self) -> Result<(), SimplexError> {
        let constraints = self.constraints();
        if self.tableau.len() < constraints + 2 {
            return Err(SimplexError::TableauTooSmall {
                rows: self.tableau.len(),
                required_rows: constraints + 2,
            });
        }

        let required_cols = self.variables + 1;
        for (row, cols) in self.tableau.iter().map(Vec::len).enumerate() {
            if cols < required_cols {
                return Err(SimplexError::RowTooSmall {
                    row,
                    cols,
                    required_cols,
                });
            }
        }

        for row in 1..=constraints {
            if self.tableau[row][0] < 0.0 {
                return Err(SimplexError::NegativeConstraintRhs { row });
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimplexSolution {
    pub case: SimplexCase,
    pub objective: f64,
    pub zero_variables: Vec<usize>,
    pub positive_variables: Vec<usize>,
}

impl SimplexSolution {
    pub fn variable_values(&self, tableau: &[Vec<f64>], variables: usize) -> Vec<f64> {
        let mut values = vec![0.0; variables];
        for row in 1..self.positive_variables.len() {
            let variable = self.positive_variables[row];
            if (1..=variables).contains(&variable) {
                values[variable - 1] = tableau[row][0];
            }
        }
        values
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SimplexError {
    BadConstraintCounts {
        constraints: usize,
        counted_constraints: usize,
    },
    TableauTooSmall {
        rows: usize,
        required_rows: usize,
    },
    RowTooSmall {
        row: usize,
        cols: usize,
        required_cols: usize,
    },
    NegativeConstraintRhs {
        row: usize,
    },
    EmptyVariableSet,
    MissingIntegrationDependencies {
        operation: &'static str,
    },
}

impl fmt::Display for SimplexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadConstraintCounts {
                constraints,
                counted_constraints,
            } => write!(
                f,
                "bad simplex constraint counts: m={constraints}, m1+m2+m3={counted_constraints}"
            ),
            Self::TableauTooSmall {
                rows,
                required_rows,
            } => write!(
                f,
                "simplex tableau has {rows} rows, needs at least {required_rows}"
            ),
            Self::RowTooSmall {
                row,
                cols,
                required_cols,
            } => write!(
                f,
                "simplex tableau row {row} has {cols} columns, needs at least {required_cols}"
            ),
            Self::NegativeConstraintRhs { row } => {
                write!(f, "simplex constraint row {row} has a negative RHS")
            }
            Self::EmptyVariableSet => write!(f, "simplex problem has no variables"),
            Self::MissingIntegrationDependencies { operation } => {
                write!(f, "{operation} requires native prerequisite ports")
            }
        }
    }
}

impl Error for SimplexError {}

pub fn solve_simplex(problem: &mut SimplexProblem) -> Result<SimplexSolution, SimplexError> {
    problem.validate()?;
    let m = problem.constraints();
    let counted_constraints = problem.less_equal_constraints
        + problem.greater_equal_constraints
        + problem.equality_constraints;
    if m != counted_constraints {
        return Err(SimplexError::BadConstraintCounts {
            constraints: m,
            counted_constraints,
        });
    }
    if problem.variables == 0 {
        return Err(SimplexError::EmptyVariableSet);
    }

    let n = problem.variables;
    let m1 = problem.less_equal_constraints;
    let m2 = problem.greater_equal_constraints;
    let m3 = problem.equality_constraints;
    let tableau = &mut problem.tableau;

    let mut l1 = vec![0; n + 2];
    let mut l2 = vec![0; m + 1];
    let mut l3 = vec![0; m2 + 1];
    let mut izrov = vec![0; n + 1];
    let mut iposv = vec![0; m + 1];

    let mut nl1 = n;
    for k in 1..=n {
        l1[k] = k;
        izrov[k] = k;
    }

    let nl2 = m;
    for i in 1..=m {
        l2[i] = i;
        iposv[i] = n + i;
    }
    l3.iter_mut()
        .take(m2 + 1)
        .skip(1)
        .for_each(|value| *value = 1);

    if m2 + m3 > 0 {
        for k in 0..=n {
            let mut q1 = 0.0;
            for i in (m1 + 1)..=m {
                q1 += tableau[i][k];
            }
            tableau[m + 1][k] = -q1;
        }

        loop {
            let (kp, bmax) = simp1(tableau, m + 1, &l1, nl1, false);
            if bmax <= SIMPLEX_EPSILON && tableau[m + 1][0] < -SIMPLEX_EPSILON {
                return Ok(SimplexSolution {
                    case: SimplexCase::Infeasible,
                    objective: tableau[0][0],
                    zero_variables: izrov,
                    positive_variables: iposv,
                });
            } else if bmax <= SIMPLEX_EPSILON && tableau[m + 1][0] <= SIMPLEX_EPSILON {
                let mut pivot_from_artificial = None;
                let m12_start = m1 + m2 + 1;
                if m12_start <= m {
                    for ip in m12_start..=m {
                        if iposv[ip] == ip + n {
                            let (candidate_kp, candidate_bmax) = simp1(tableau, ip, &l1, nl1, true);
                            if candidate_bmax > 0.0 {
                                pivot_from_artificial = Some((ip, candidate_kp));
                                break;
                            }
                        }
                    }
                }

                if let Some((ip, kp)) = pivot_from_artificial {
                    simp3(tableau, m + 1, n, ip, kp);
                    update_artificial_basis(
                        tableau, &mut l1, &mut nl1, &mut izrov, &mut iposv, &mut l3, m, m1, n, ip,
                        kp,
                    );
                    continue;
                }

                let m12 = m1 + m2;
                if m1 < m12 {
                    for i in (m1 + 1)..=m12 {
                        if l3[i - m1] == 1 {
                            for k in 0..=n {
                                tableau[i][k] = -tableau[i][k];
                            }
                        }
                    }
                }
                break;
            }

            let (ip, _) = simp2(tableau, n, &l2, nl2, kp);
            if ip == 0 {
                return Ok(SimplexSolution {
                    case: SimplexCase::Infeasible,
                    objective: tableau[0][0],
                    zero_variables: izrov,
                    positive_variables: iposv,
                });
            }
            simp3(tableau, m + 1, n, ip, kp);
            update_artificial_basis(
                tableau, &mut l1, &mut nl1, &mut izrov, &mut iposv, &mut l3, m, m1, n, ip, kp,
            );
        }
    }

    loop {
        let (kp, bmax) = simp1(tableau, 0, &l1, nl1, false);
        if bmax <= 0.0 {
            return Ok(SimplexSolution {
                case: SimplexCase::Optimal,
                objective: tableau[0][0],
                zero_variables: izrov,
                positive_variables: iposv,
            });
        }

        let (ip, _) = simp2(tableau, n, &l2, nl2, kp);
        if ip == 0 {
            return Ok(SimplexSolution {
                case: SimplexCase::Unbounded,
                objective: tableau[0][0],
                zero_variables: izrov,
                positive_variables: iposv,
            });
        }
        simp3(tableau, m, n, ip, kp);
        let is = izrov[kp];
        izrov[kp] = iposv[ip];
        iposv[ip] = is;
    }
}

fn simp1(
    tableau: &[Vec<f64>],
    objective_row: usize,
    candidates: &[usize],
    candidate_count: usize,
    absolute: bool,
) -> (usize, f64) {
    let mut kp = candidates[1];
    let mut bmax = tableau[objective_row][kp];
    for &candidate in candidates.iter().take(candidate_count + 1).skip(2) {
        let test = if absolute {
            tableau[objective_row][candidate].abs() - bmax.abs()
        } else {
            tableau[objective_row][candidate] - bmax
        };
        if test > 0.0 {
            bmax = tableau[objective_row][candidate];
            kp = candidate;
        }
    }
    (kp, bmax)
}

fn simp2(
    tableau: &[Vec<f64>],
    variables: usize,
    candidates: &[usize],
    candidate_count: usize,
    pivot_col: usize,
) -> (usize, f64) {
    let mut ip = 0;
    let mut q1 = 0.0;

    for i in 1..=candidate_count {
        let row = candidates[i];
        if tableau[row][pivot_col] < -SIMPLEX_EPSILON {
            q1 = -tableau[row][0] / tableau[row][pivot_col];
            ip = row;
            for &candidate_row in candidates.iter().take(candidate_count + 1).skip(i + 1) {
                if tableau[candidate_row][pivot_col] < -SIMPLEX_EPSILON {
                    let q = -tableau[candidate_row][0] / tableau[candidate_row][pivot_col];
                    if q < q1 {
                        ip = candidate_row;
                        q1 = q;
                    } else if q == q1 {
                        let mut qp = 0.0;
                        let mut q0 = 0.0;
                        for k in 1..=variables {
                            qp = -tableau[ip][k] / tableau[ip][pivot_col];
                            q0 = -tableau[candidate_row][k] / tableau[candidate_row][pivot_col];
                            if q0 != qp {
                                break;
                            }
                        }
                        if q0 < qp {
                            ip = candidate_row;
                        }
                    }
                }
            }
            break;
        }
    }

    (ip, q1)
}

fn simp3(
    tableau: &mut [Vec<f64>],
    max_row: usize,
    variables: usize,
    pivot_row: usize,
    pivot_col: usize,
) {
    let piv = 1.0 / tableau[pivot_row][pivot_col];
    for row in 0..=max_row {
        if row != pivot_row {
            tableau[row][pivot_col] *= piv;
            for col in 0..=variables {
                if col != pivot_col {
                    tableau[row][col] -= tableau[pivot_row][col] * tableau[row][pivot_col];
                }
            }
        }
    }
    for col in 0..=variables {
        if col != pivot_col {
            tableau[pivot_row][col] *= -piv;
        }
    }
    tableau[pivot_row][pivot_col] = piv;
}

#[allow(clippy::too_many_arguments)]
fn update_artificial_basis(
    tableau: &mut [Vec<f64>],
    l1: &mut [usize],
    nl1: &mut usize,
    izrov: &mut [usize],
    iposv: &mut [usize],
    l3: &mut [usize],
    constraints: usize,
    less_equal_constraints: usize,
    variables: usize,
    pivot_row: usize,
    pivot_col: usize,
) {
    if iposv[pivot_row] >= variables + less_equal_constraints + l3.len() {
        if let Some(k) = (1..=*nl1).find(|&k| l1[k] == pivot_col) {
            *nl1 -= 1;
            for is in k..=*nl1 {
                l1[is] = l1[is + 1];
            }
        }
        tableau[constraints + 1][pivot_col] += 1.0;
        for row in 0..=(constraints + 1) {
            tableau[row][pivot_col] = -tableau[row][pivot_col];
        }
    } else if iposv[pivot_row] >= variables + less_equal_constraints + 1 {
        let kh = iposv[pivot_row] - less_equal_constraints - variables;
        if l3[kh] != 0 {
            l3[kh] = 0;
            tableau[constraints + 1][pivot_col] += 1.0;
            for row in 0..=(constraints + 1) {
                tableau[row][pivot_col] = -tableau[row][pivot_col];
            }
        }
    }

    let is = izrov[pivot_col];
    izrov[pivot_col] = iposv[pivot_row];
    iposv[pivot_row] = is;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_less_equal_maximization_problem() {
        let mut problem = SimplexProblem::new(
            vec![
                vec![0.0, 3.0, 2.0],
                vec![4.0, -1.0, -1.0],
                vec![2.0, -1.0, 0.0],
                vec![3.0, 0.0, -1.0],
                vec![0.0, 0.0, 0.0],
            ],
            2,
            3,
            0,
            0,
        );

        let solution = solve_simplex(&mut problem).unwrap();

        assert_eq!(solution.case, SimplexCase::Optimal);
        assert_eq!(
            solution.variable_values(&problem.tableau, 2),
            vec![2.0, 2.0]
        );
        assert_eq!(problem.tableau[0][0], 10.0);
    }

    #[test]
    fn reports_unbounded_objective() {
        let mut problem = SimplexProblem::new(
            vec![vec![0.0, 1.0], vec![1.0, 1.0], vec![0.0, 0.0]],
            1,
            1,
            0,
            0,
        );

        let solution = solve_simplex(&mut problem).unwrap();

        assert_eq!(solution.case, SimplexCase::Unbounded);
    }

    #[test]
    fn solves_problem_requiring_artificial_phase() {
        let mut problem = SimplexProblem::new(
            vec![vec![0.0, -1.0], vec![2.0, -1.0], vec![0.0, 0.0]],
            1,
            0,
            1,
            0,
        );

        let solution = solve_simplex(&mut problem).unwrap();

        assert_eq!(solution.case, SimplexCase::Optimal);
        assert_eq!(solution.variable_values(&problem.tableau, 1), vec![2.0]);
        assert_eq!(problem.tableau[0][0], -2.0);
    }

    #[test]
    fn detects_infeasible_artificial_problem() {
        let mut problem = SimplexProblem::new(
            vec![
                vec![0.0, 1.0],
                vec![2.0, -1.0],
                vec![1.0, 1.0],
                vec![0.0, 0.0],
            ],
            1,
            0,
            1,
            1,
        );

        let solution = solve_simplex(&mut problem).unwrap();

        assert_eq!(solution.case, SimplexCase::Infeasible);
    }

    #[test]
    fn validates_c_tableau_shape_and_rhs_rules() {
        let negative_rhs = SimplexProblem::new(
            vec![vec![0.0, 1.0], vec![-1.0, -1.0], vec![0.0, 0.0]],
            1,
            1,
            0,
            0,
        );
        assert_eq!(
            negative_rhs.validate(),
            Err(SimplexError::NegativeConstraintRhs { row: 1 })
        );

        let short_row = SimplexProblem::new(vec![vec![0.0], vec![1.0], vec![0.0]], 1, 1, 0, 0);
        assert_eq!(
            short_row.validate(),
            Err(SimplexError::RowTooSmall {
                row: 0,
                cols: 1,
                required_cols: 2,
            })
        );
    }
}
