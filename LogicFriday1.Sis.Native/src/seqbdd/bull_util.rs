//! Native Rust model for `LogicSynthesis/sis/seqbdd/bull_util.c`.
//!
//! The C file provides three helpers for the BULL sequential range method:
//! recursive input cofactoring, grouping functions by overlapping BDD support,
//! and a two-function range shortcut. The support grouping and two-function
//! shortcut are ported onto owned Rust data. The full recursive image path is
//! still tied to SIS `array_t`, `st_table`, `var_set_t`, and BDD-manager
//! lifetimes, so it reports generic missing-port diagnostics until those ports exist.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BullUtilDisposition {
    SupportAndRange2PortedSisBddIntegrationBlocked,
}

pub fn bull_util_disposition() -> BullUtilDisposition {
    BullUtilDisposition::SupportAndRange2PortedSisBddIntegrationBlocked
}

pub fn is_sis_bdd_integration_blocked() -> bool {
    bull_util_disposition() == BullUtilDisposition::SupportAndRange2PortedSisBddIntegrationBlocked
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BullUtilError {
    MissingSisDependencies {
        operation: &'static str,
    },
    EmptySupportList,
    SupportWidthMismatch {
        expected: usize,
        actual: usize,
        row: usize,
    },
    MissingRangeFunction {
        selected_count: usize,
    },
    PiListTooShort {
        needed_index: usize,
        actual_len: usize,
    },
    TruthTableVarMismatch {
        expected: usize,
        actual: usize,
    },
    TruthTableLengthMismatch {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for BullUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisDependencies { operation } => {
                write!(f, "{operation} is blocked by missing native SIS ports")
            }
            Self::EmptySupportList => write!(f, "support list is empty"),
            Self::SupportWidthMismatch {
                expected,
                actual,
                row,
            } => write!(
                f,
                "support row {row} has width {actual}; expected {expected}"
            ),
            Self::MissingRangeFunction { selected_count } => write!(
                f,
                "range_2_compute requires two selected live functions; found {selected_count}"
            ),
            Self::PiListTooShort {
                needed_index,
                actual_len,
            } => write!(
                f,
                "PI list length {actual_len} does not contain selected index {needed_index}"
            ),
            Self::TruthTableVarMismatch { expected, actual } => {
                write!(f, "truth table has {actual} variables; expected {expected}")
            }
            Self::TruthTableLengthMismatch { expected, actual } => {
                write!(f, "truth table has {actual} entries; expected {expected}")
            }
        }
    }
}

impl Error for BullUtilError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportSet {
    width: usize,
    variables: BTreeSet<usize>,
}

impl SupportSet {
    pub fn new(
        width: usize,
        variables: impl IntoIterator<Item = usize>,
    ) -> Result<Self, BullUtilError> {
        let variables = variables.into_iter().collect::<BTreeSet<_>>();
        if let Some(variable) = variables.iter().find(|variable| **variable >= width) {
            return Err(BullUtilError::SupportWidthMismatch {
                expected: width,
                actual: variable + 1,
                row: 0,
            });
        }
        Ok(Self { width, variables })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn contains(&self, variable: usize) -> bool {
        self.variables.contains(&variable)
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.variables
            .iter()
            .any(|variable| other.contains(*variable))
    }

    fn union_with(&mut self, other: &Self) {
        self.variables.extend(other.variables.iter().copied());
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionPartition {
    pub functions: BTreeSet<usize>,
}

pub fn disjoint_support_functions(
    supports: &[SupportSet],
) -> Result<Vec<FunctionPartition>, BullUtilError> {
    if supports.is_empty() {
        return Err(BullUtilError::EmptySupportList);
    }

    let width = supports[0].width();
    for (row, support) in supports.iter().enumerate() {
        if support.width() != width {
            return Err(BullUtilError::SupportWidthMismatch {
                expected: width,
                actual: support.width(),
                row,
            });
        }
    }

    let mut merged = supports.iter().cloned().map(Some).collect::<Vec<_>>();
    for variable in 0..width {
        let mut first: Option<usize> = None;
        for row in 0..merged.len() {
            if !merged[row]
                .as_ref()
                .is_some_and(|support| support.contains(variable))
            {
                continue;
            }

            if let Some(first_row) = first {
                let support = merged[row].take().expect("checked as present");
                merged[first_row]
                    .as_mut()
                    .expect("first support remains present")
                    .union_with(&support);
            } else {
                first = Some(row);
            }
        }
    }

    let mut partitions = Vec::new();
    for support in merged.into_iter().flatten() {
        let functions = supports
            .iter()
            .enumerate()
            .filter_map(|(index, original)| support.intersects(original).then_some(index))
            .collect::<BTreeSet<_>>();
        partitions.push(FunctionPartition { functions });
    }

    Ok(partitions)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TruthTable {
    n_vars: usize,
    values: Vec<bool>,
}

impl TruthTable {
    pub fn new(n_vars: usize, values: impl Into<Vec<bool>>) -> Result<Self, BullUtilError> {
        let values = values.into();
        let expected = 1usize << n_vars;
        if values.len() != expected {
            return Err(BullUtilError::TruthTableLengthMismatch {
                expected,
                actual: values.len(),
            });
        }
        Ok(Self { n_vars, values })
    }

    pub fn variable(n_vars: usize, variable: usize) -> Result<Self, BullUtilError> {
        if variable >= n_vars {
            return Err(BullUtilError::TruthTableVarMismatch {
                expected: n_vars,
                actual: variable + 1,
            });
        }

        let values = (0..(1usize << n_vars))
            .map(|minterm| ((minterm >> variable) & 1) == 1)
            .collect();
        Ok(Self { n_vars, values })
    }

    pub fn constant(n_vars: usize, value: bool) -> Self {
        Self {
            n_vars,
            values: vec![value; 1usize << n_vars],
        }
    }

    pub fn not(&self) -> Self {
        Self {
            n_vars: self.n_vars,
            values: self.values.iter().map(|value| !value).collect(),
        }
    }

    pub fn and(&self, other: &Self) -> Result<Self, BullUtilError> {
        self.zip(other, |left, right| left && right)
    }

    pub fn or(&self, other: &Self) -> Result<Self, BullUtilError> {
        self.zip(other, |left, right| left || right)
    }

    pub fn is_tautology(&self, value: bool) -> bool {
        self.values.iter().all(|entry| *entry == value)
    }

    pub fn cofactor_tautology(&self, condition: &Self) -> Result<Option<bool>, BullUtilError> {
        self.check_compatible(condition)?;
        let mut selected = self
            .values
            .iter()
            .zip(condition.values.iter())
            .filter_map(|(value, include)| include.then_some(*value));

        let Some(first) = selected.next() else {
            return Ok(None);
        };
        if selected.all(|value| value == first) {
            Ok(Some(first))
        } else {
            Ok(None)
        }
    }

    fn zip(
        &self,
        other: &Self,
        mut op: impl FnMut(bool, bool) -> bool,
    ) -> Result<Self, BullUtilError> {
        self.check_compatible(other)?;
        Ok(Self {
            n_vars: self.n_vars,
            values: self
                .values
                .iter()
                .zip(other.values.iter())
                .map(|(left, right)| op(*left, *right))
                .collect(),
        })
    }

    fn check_compatible(&self, other: &Self) -> Result<(), BullUtilError> {
        if self.n_vars != other.n_vars {
            return Err(BullUtilError::TruthTableVarMismatch {
                expected: self.n_vars,
                actual: other.n_vars,
            });
        }
        Ok(())
    }
}

pub fn range_2_compute(
    selected_live_functions: &FunctionPartition,
    bdd_list: &[Option<TruthTable>],
    pi_list: &[TruthTable],
) -> Result<TruthTable, BullUtilError> {
    let mut selected = Vec::new();
    let mut live_index = 0usize;

    for (bdd_index, bdd) in bdd_list.iter().enumerate() {
        let Some(function) = bdd else {
            continue;
        };

        if selected_live_functions.functions.contains(&live_index) {
            selected.push((bdd_index, function));
            if selected.len() == 2 {
                break;
            }
        }
        live_index += 1;
    }

    if selected.len() != 2 {
        return Err(BullUtilError::MissingRangeFunction {
            selected_count: selected.len(),
        });
    }

    let (k1, f1) = selected[0];
    let (k2, f2) = selected[1];
    let n1 = pi_list.get(k1).ok_or(BullUtilError::PiListTooShort {
        needed_index: k1,
        actual_len: pi_list.len(),
    })?;
    let n2 = pi_list.get(k2).ok_or(BullUtilError::PiListTooShort {
        needed_index: k2,
        actual_len: pi_list.len(),
    })?;

    let out1 = f2.cofactor_tautology(f1)?;
    let f1_bar = f1.not();
    let out2 = f2.cofactor_tautology(&f1_bar)?;

    let c1 = match out1 {
        Some(true) => n1.and(n2)?,
        Some(false) => n1.and(&n2.not())?,
        None => n1.clone(),
    };
    let c2 = match out2 {
        Some(true) => n1.not().and(n2)?,
        Some(false) => n1.not().and(&n2.not())?,
        None => n1.not(),
    };

    c1.or(&c2)
}

pub fn input_cofactor_is_blocked() -> bool {
    true
}

pub fn input_cofactor() -> Result<TruthTable, BullUtilError> {
    Err(BullUtilError::MissingSisDependencies {
        operation: "input_cofactor",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn support(width: usize, variables: &[usize]) -> SupportSet {
        SupportSet::new(width, variables.iter().copied()).unwrap()
    }

    #[test]
    fn disjoint_support_functions_merges_overlapping_supports() {
        let partitions = disjoint_support_functions(&[
            support(5, &[0, 2]),
            support(5, &[1]),
            support(5, &[2, 3]),
            support(5, &[4]),
        ])
        .unwrap();

        assert_eq!(
            partitions,
            vec![
                FunctionPartition {
                    functions: BTreeSet::from([0, 2])
                },
                FunctionPartition {
                    functions: BTreeSet::from([1])
                },
                FunctionPartition {
                    functions: BTreeSet::from([3])
                },
            ]
        );
    }

    #[test]
    fn disjoint_support_functions_reports_width_mismatch() {
        let err = disjoint_support_functions(&[support(2, &[0]), support(3, &[1])]).unwrap_err();
        assert_eq!(
            err,
            BullUtilError::SupportWidthMismatch {
                expected: 2,
                actual: 3,
                row: 1,
            }
        );
    }

    #[test]
    fn range_2_compute_ports_equal_case() {
        let x = TruthTable::variable(2, 0).unwrap();
        let y = TruthTable::variable(2, 1).unwrap();
        let result = range_2_compute(
            &FunctionPartition {
                functions: BTreeSet::from([0, 1]),
            },
            &[Some(x.clone()), Some(x.clone())],
            &[x.clone(), y.clone()],
        )
        .unwrap();

        assert_eq!(
            result,
            x.and(&y)
                .unwrap()
                .or(&x.not().and(&y.not()).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn range_2_compute_ports_complement_case() {
        let x = TruthTable::variable(2, 0).unwrap();
        let y = TruthTable::variable(2, 1).unwrap();
        let result = range_2_compute(
            &FunctionPartition {
                functions: BTreeSet::from([0, 1]),
            },
            &[Some(x.clone()), Some(x.not())],
            &[x.clone(), y.clone()],
        )
        .unwrap();

        assert_eq!(
            result,
            x.and(&y.not())
                .unwrap()
                .or(&x.not().and(&y).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn range_2_compute_uses_live_function_indices_like_c_code() {
        let x = TruthTable::variable(2, 0).unwrap();
        let y = TruthTable::variable(2, 1).unwrap();
        let result = range_2_compute(
            &FunctionPartition {
                functions: BTreeSet::from([0, 1]),
            },
            &[None, Some(x.clone()), Some(x.clone())],
            &[TruthTable::constant(2, false), x.clone(), y.clone()],
        )
        .unwrap();

        assert_eq!(
            result,
            x.and(&y)
                .unwrap()
                .or(&x.not().and(&y.not()).unwrap())
                .unwrap()
        );
    }
}
