//! Native Rust model for `LogicSynthesis/sis/simplify/simp_image.c`.
//!
//! The original SIS file computes the image/range of a list of BDD-backed
//! Boolean functions and returns a `node_t` sum-of-products over the associated
//! output variables. This module keeps that behavior available over native Rust
//! truth tables. The direct SIS entry points remain blocked on the native BDD,
//! node, array, and `var_set` ports and report those dependencies explicitly.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead: &'static str,
    pub c_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.2",
        c_file: "LogicSynthesis/sis/array/array.c",
        reason: "array_t ownership and indexed BDD/node lists",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.71",
        c_file: "LogicSynthesis/sis/bdd_cmu/bdd_port/bddport.c",
        reason: "SIS bdd_t manager, constants, tautology tests, support, and lifetime API",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.78",
        c_file: "LogicSynthesis/sis/bdd_ucb/bdd_cofactor.c",
        reason: "bdd_cofactor used by recursive output cofactoring",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.90",
        c_file: "LogicSynthesis/sis/bdd_ucb/bdd_support.c",
        reason: "bdd_get_support partitioning substrate",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.318",
        c_file: "LogicSynthesis/sis/node/node.c",
        reason: "node_constant, node_literal, node_and, node_or, node_function, and node_free",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.447",
        c_file: "LogicSynthesis/sis/simplify/compute_dc.c",
        reason: "CSPF(node)->set metadata consumed by set_size_sort",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.518",
        c_file: "LogicSynthesis/sis/var_set/var_set.c",
        reason: "support sets and disjoint-support partitions",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SisEntryPoint {
    SimpBullCofactor,
    SetSizeSort,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimpImageError {
    EmptyOutputName {
        index: usize,
    },
    FunctionOutputCountMismatch {
        functions: usize,
        outputs: usize,
    },
    InvalidTruthTableLength {
        output: usize,
        support_len: usize,
        values_len: usize,
    },
    TooManyInputs {
        inputs: usize,
    },
    TooManyOutputs {
        outputs: usize,
    },
    MissingSisDependencies {
        entry_point: SisEntryPoint,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for SimpImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyOutputName { index } => write!(f, "output variable {index} has no name"),
            Self::FunctionOutputCountMismatch { functions, outputs } => write!(
                f,
                "function count ({functions}) does not match output variable count ({outputs})",
            ),
            Self::InvalidTruthTableLength {
                output,
                support_len,
                values_len,
            } => write!(
                f,
                "output {output} has {values_len} truth-table values for {support_len} support variables",
            ),
            Self::TooManyInputs { inputs } => write!(
                f,
                "image computation needs {inputs} input variables; this native scaffold supports at most {}",
                usize::BITS - 1,
            ),
            Self::TooManyOutputs { outputs } => write!(
                f,
                "image computation needs {outputs} output variables; this native scaffold supports at most {}",
                usize::BITS - 1,
            ),
            Self::MissingSisDependencies {
                entry_point,
                dependencies,
            } => write!(
                f,
                "{entry_point:?} requires {} unported SIS dependencies",
                dependencies.len(),
            ),
        }
    }
}

impl Error for SimpImageError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TruthTableFunction {
    support: Vec<usize>,
    values: Vec<bool>,
}

impl TruthTableFunction {
    pub fn new(support: impl Into<Vec<usize>>, values: impl Into<Vec<bool>>) -> Self {
        Self {
            support: support.into(),
            values: values.into(),
        }
    }

    pub fn constant(value: bool) -> Self {
        Self {
            support: Vec::new(),
            values: vec![value],
        }
    }

    pub fn projection(input: usize) -> Self {
        Self {
            support: vec![input],
            values: vec![false, true],
        }
    }

    pub fn support(&self) -> &[usize] {
        &self.support
    }

    pub fn values(&self) -> &[bool] {
        &self.values
    }

    fn validate(&self, output: usize) -> Result<(), SimpImageError> {
        let expected = 1usize.checked_shl(self.support.len() as u32).ok_or({
            SimpImageError::TooManyInputs {
                inputs: self.support.len(),
            }
        })?;

        if expected != self.values.len() {
            return Err(SimpImageError::InvalidTruthTableLength {
                output,
                support_len: self.support.len(),
                values_len: self.values.len(),
            });
        }

        Ok(())
    }

    fn eval_global_assignment(&self, assignment: usize) -> bool {
        let mut local_index = 0usize;
        for (local_bit, input) in self.support.iter().copied().enumerate() {
            let bit = (assignment >> input) & 1;
            local_index |= bit << local_bit;
        }

        self.values[local_index]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImageExpr {
    Const(bool),
    Literal { variable: String, phase: bool },
    And(Vec<ImageExpr>),
    Or(Vec<ImageExpr>),
}

impl ImageExpr {
    pub fn literal(variable: impl Into<String>, phase: bool) -> Self {
        Self::Literal {
            variable: variable.into(),
            phase,
        }
    }

    pub fn and(terms: impl IntoIterator<Item = ImageExpr>) -> Self {
        normalize_and(terms.into_iter().collect())
    }

    pub fn or(terms: impl IntoIterator<Item = ImageExpr>) -> Self {
        normalize_or(terms.into_iter().collect())
    }

    pub fn eval(&self, assignment: impl Fn(&str) -> bool + Copy) -> bool {
        match self {
            Self::Const(value) => *value,
            Self::Literal { variable, phase } => assignment(variable) == *phase,
            Self::And(terms) => terms.iter().all(|term| term.eval(assignment)),
            Self::Or(terms) => terms.iter().any(|term| term.eval(assignment)),
        }
    }
}

impl fmt::Display for ImageExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(true) => write!(f, "1"),
            Self::Const(false) => write!(f, "0"),
            Self::Literal { variable, phase } if *phase => write!(f, "{variable}"),
            Self::Literal { variable, .. } => write!(f, "!{variable}"),
            Self::And(terms) => join_expr(f, terms, " & "),
            Self::Or(terms) => join_expr(f, terms, " | "),
        }
    }
}

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn simp_bull_cofactor_sis_blocked() -> Result<ImageExpr, SimpImageError> {
    Err(SimpImageError::MissingSisDependencies {
        entry_point: SisEntryPoint::SimpBullCofactor,
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn set_size_sort_sis_blocked() -> Result<(), SimpImageError> {
    Err(SimpImageError::MissingSisDependencies {
        entry_point: SisEntryPoint::SetSizeSort,
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn image_from_truth_tables(
    functions: &[TruthTableFunction],
    output_variables: &[impl AsRef<str>],
) -> Result<ImageExpr, SimpImageError> {
    if functions.len() != output_variables.len() {
        return Err(SimpImageError::FunctionOutputCountMismatch {
            functions: functions.len(),
            outputs: output_variables.len(),
        });
    }
    if output_variables.len() >= usize::BITS as usize {
        return Err(SimpImageError::TooManyOutputs {
            outputs: output_variables.len(),
        });
    }

    let output_names = output_variables
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let name = name.as_ref();
            if name.is_empty() {
                Err(SimpImageError::EmptyOutputName { index })
            } else {
                Ok(name.to_owned())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    for (output, function) in functions.iter().enumerate() {
        function.validate(output)?;
    }

    let input_count = functions
        .iter()
        .flat_map(|function| function.support.iter().copied())
        .max()
        .map_or(0, |input| input + 1);
    let assignment_count =
        1usize
            .checked_shl(input_count as u32)
            .ok_or(SimpImageError::TooManyInputs {
                inputs: input_count,
            })?;

    let mut reachable_outputs = BTreeSet::new();
    for assignment in 0..assignment_count {
        let mut output_bits = 0usize;
        for (index, function) in functions.iter().enumerate() {
            if function.eval_global_assignment(assignment) {
                output_bits |= 1usize << index;
            }
        }
        reachable_outputs.insert(output_bits);
    }

    Ok(sop_from_reachable_outputs(
        &reachable_outputs,
        &output_names,
    ))
}

pub fn disjoint_support_partitions(functions: &[TruthTableFunction]) -> Vec<Vec<usize>> {
    let mut partitions: Vec<Vec<usize>> = Vec::new();

    for (function_index, function) in functions.iter().enumerate() {
        let mut overlapping = Vec::new();
        for (partition_index, partition) in partitions.iter().enumerate() {
            if partition
                .iter()
                .any(|member| supports_intersect(function, &functions[*member]))
            {
                overlapping.push(partition_index);
            }
        }

        if overlapping.is_empty() {
            partitions.push(vec![function_index]);
            continue;
        }

        let first = overlapping[0];
        partitions[first].push(function_index);
        for partition_index in overlapping.into_iter().skip(1).rev() {
            let merged = partitions.remove(partition_index);
            partitions[first].extend(merged);
        }
    }

    partitions
}

fn sop_from_reachable_outputs(
    reachable_outputs: &BTreeSet<usize>,
    output_names: &[String],
) -> ImageExpr {
    if output_names.is_empty() {
        return ImageExpr::Const(true);
    }

    let full_space_size = 1usize
        .checked_shl(output_names.len() as u32)
        .expect("output count was validated before SOP construction");
    if reachable_outputs.is_empty() {
        return ImageExpr::Const(false);
    }
    if reachable_outputs.len() == full_space_size {
        return ImageExpr::Const(true);
    }

    ImageExpr::or(reachable_outputs.iter().map(|output_bits| {
        ImageExpr::and(output_names.iter().enumerate().map(|(index, variable)| {
            ImageExpr::literal(variable.clone(), ((output_bits >> index) & 1) != 0)
        }))
    }))
}

fn supports_intersect(left: &TruthTableFunction, right: &TruthTableFunction) -> bool {
    left.support
        .iter()
        .any(|input| right.support.iter().any(|other| input == other))
}

fn normalize_and(terms: Vec<ImageExpr>) -> ImageExpr {
    let mut normalized = Vec::new();
    for term in terms {
        match term {
            ImageExpr::Const(false) => return ImageExpr::Const(false),
            ImageExpr::Const(true) => {}
            ImageExpr::And(nested) => normalized.extend(nested),
            other => normalized.push(other),
        }
    }

    match normalized.len() {
        0 => ImageExpr::Const(true),
        1 => normalized.remove(0),
        _ => ImageExpr::And(normalized),
    }
}

fn normalize_or(terms: Vec<ImageExpr>) -> ImageExpr {
    let mut normalized = Vec::new();
    for term in terms {
        match term {
            ImageExpr::Const(true) => return ImageExpr::Const(true),
            ImageExpr::Const(false) => {}
            ImageExpr::Or(nested) => normalized.extend(nested),
            other => normalized.push(other),
        }
    }

    match normalized.len() {
        0 => ImageExpr::Const(false),
        1 => normalized.remove(0),
        _ => ImageExpr::Or(normalized),
    }
}

fn join_expr(f: &mut fmt::Formatter<'_>, terms: &[ImageExpr], separator: &str) -> fmt::Result {
    let mut first = true;
    write!(f, "(")?;
    for term in terms {
        if first {
            first = false;
        } else {
            write!(f, "{separator}")?;
        }
        write!(f, "{term}")?;
    }
    write!(f, ")")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_constrain_output_image() {
        let expr = image_from_truth_tables(
            &[
                TruthTableFunction::constant(true),
                TruthTableFunction::constant(false),
            ],
            &["y0", "y1"],
        )
        .unwrap();

        assert_eq!(
            expr,
            ImageExpr::and([
                ImageExpr::literal("y0", true),
                ImageExpr::literal("y1", false),
            ]),
        );
        assert!(expr.eval(|name| name == "y0"));
        assert!(!expr.eval(|_| true));
    }

    #[test]
    fn one_nonconstant_output_has_full_range() {
        let expr = image_from_truth_tables(&[TruthTableFunction::projection(0)], &["y"]).unwrap();

        assert_eq!(expr, ImageExpr::Const(true));
    }

    #[test]
    fn correlated_two_output_range_matches_special_case_shape() {
        let x = TruthTableFunction::projection(0);
        let not_x = TruthTableFunction::new([0], [true, false]);

        let expr = image_from_truth_tables(&[x, not_x], &["a", "b"]).unwrap();

        assert_eq!(format!("{expr}"), "((a & !b) | (!a & b))");
        assert!(expr.eval(|name| name == "b"));
        assert!(expr.eval(|name| name == "a"));
        assert!(!expr.eval(|_| false));
        assert!(!expr.eval(|_| true));
    }

    #[test]
    fn disjoint_support_grouping_merges_transitive_overlaps() {
        let functions = [
            TruthTableFunction::projection(0),
            TruthTableFunction::projection(2),
            TruthTableFunction::new([0, 1], [false, true, true, false]),
            TruthTableFunction::projection(3),
            TruthTableFunction::new([2, 3], [false, true, true, false]),
        ];

        assert_eq!(
            disjoint_support_partitions(&functions),
            vec![vec![0, 2], vec![1, 4, 3]],
        );
    }

    #[test]
    fn invalid_tables_and_sis_blockers_are_explicit() {
        assert_eq!(
            image_from_truth_tables(&[TruthTableFunction::new([0, 1], [false, true])], &["y"]),
            Err(SimpImageError::InvalidTruthTableLength {
                output: 0,
                support_len: 2,
                values_len: 2,
            }),
        );

        let error = simp_bull_cofactor_sis_blocked().unwrap_err();
        let SimpImageError::MissingSisDependencies { dependencies, .. } = error else {
            panic!("expected missing dependency error");
        };
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.318"
                && dependency.c_file == "LogicSynthesis/sis/node/node.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.518"
                && dependency.c_file == "LogicSynthesis/sis/var_set/var_set.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead == "LogicFriday1-8j8.2.6.71"
                && dependency.c_file == "LogicSynthesis/sis/bdd_cmu/bdd_port/bddport.c"
        }));
    }
}
