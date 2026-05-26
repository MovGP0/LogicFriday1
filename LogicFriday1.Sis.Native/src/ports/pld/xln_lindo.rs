//! Native Rust model for `LogicSynthesis/sis/pld/xln_lindo.c`.
//!
//! The C file writes a max-cardinality matching problem in LINDO syntax and
//! reads LINDO's solution report back into merge-pair arrays. This port keeps
//! that behavior in owned Rust data: sparse coefficient rows, deterministic
//! LINDO formatting, solution scanning, and match-pair extraction. Direct
//! operation on SIS `array_t`, `sm_matrix`, and `node_t` remains gated by the
//! explicit dependency list below.

use std::error::Error;
use std::fmt;

pub const TERMS_PER_LINE: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        reason: "get_Lindo_result reads candidate node arrays and appends match arrays",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.456",
        source_file: "LogicSynthesis/sis/sparse/cols.c",
        reason: "selected match columns use first_row and last_row from sm_col",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.457",
        source_file: "LogicSynthesis/sis/sparse/matrix.c",
        reason: "formulate_Lindo and get_Lindo_result traverse sm_matrix rows and columns",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.458",
        source_file: "LogicSynthesis/sis/sparse/rows.c",
        reason: "formulate_Lindo emits one <= 1 constraint per sm_row",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.390",
        source_file: "LogicSynthesis/sis/pld/xln_merge.c",
        reason: "xln_merge.c owns the full merge flow around LINDO invocation and node merging",
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseCoefficientMatrix {
    ncols: usize,
    rows: Vec<Vec<usize>>,
}

impl SparseCoefficientMatrix {
    pub fn new(ncols: usize) -> Self {
        Self {
            ncols,
            rows: Vec::new(),
        }
    }

    pub fn from_rows(ncols: usize, rows: impl IntoIterator<Item = Vec<usize>>) -> Self {
        let mut matrix = Self::new(ncols);
        for row in rows {
            matrix.add_row(row);
        }
        matrix
    }

    pub fn add_row(&mut self, mut columns: Vec<usize>) {
        columns.sort_unstable();
        columns.dedup();
        self.rows.push(columns);
    }

    pub fn ncols(&self) -> usize {
        self.ncols
    }

    pub fn rows(&self) -> &[Vec<usize>] {
        &self.rows
    }

    pub fn validate(&self) -> Result<(), XlnLindoError> {
        for (row, columns) in self.rows.iter().enumerate() {
            for &column in columns {
                if column >= self.ncols {
                    return Err(XlnLindoError::ColumnOutOfRange {
                        row,
                        column,
                        ncols: self.ncols,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn column_bounds(&self, column: usize) -> Result<Option<(usize, usize)>, XlnLindoError> {
        if column >= self.ncols {
            return Err(XlnLindoError::ColumnOutOfRange {
                row: usize::MAX,
                column,
                ncols: self.ncols,
            });
        }

        let mut first = None;
        let mut last = None;
        for (row_index, row) in self.rows.iter().enumerate() {
            if row.binary_search(&column).is_ok() {
                first.get_or_insert(row_index);
                last = Some(row_index);
            }
        }

        Ok(first.zip(last))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LindoMatchResult<Node> {
    pub num_match: usize,
    pub match1: Vec<Node>,
    pub match2: Vec<Node>,
    pub report: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum XlnLindoError {
    ColumnOutOfRange {
        row: usize,
        column: usize,
        ncols: usize,
    },
    VariableOutOfRange {
        variable: usize,
        ncols: usize,
    },
    SelectedColumnHasNoRows {
        column: usize,
    },
    CandidateNodeOutOfRange {
        row: usize,
        candidates: usize,
    },
    NoFeasibleSolution,
    MissingVariableValue {
        variable: usize,
    },
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for XlnLindoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColumnOutOfRange { row, column, ncols } => write!(
                f,
                "LINDO coefficient row {row} references column {column}, but ncols is {ncols}"
            ),
            Self::VariableOutOfRange { variable, ncols } => write!(
                f,
                "LINDO solution references X{variable}, but ncols is {ncols}"
            ),
            Self::SelectedColumnHasNoRows { column } => {
                write!(f, "selected LINDO column {column} has no incident rows")
            }
            Self::CandidateNodeOutOfRange { row, candidates } => write!(
                f,
                "LINDO match row {row} has no candidate node; candidate count is {candidates}"
            ),
            Self::NoFeasibleSolution => write!(f, "NO FEASIBLE SOLUTION OBTAINED"),
            Self::MissingVariableValue { variable } => {
                write!(f, "LINDO solution is missing a value after X{variable}")
            }
            Self::MissingSisPorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS C-file dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for XlnLindoError {}

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn sis_bound_operation_unavailable(operation: &'static str) -> Result<(), XlnLindoError> {
    Err(XlnLindoError::MissingSisPorts {
        operation,
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn formulate_lindo(matrix: &SparseCoefficientMatrix) -> Result<String, XlnLindoError> {
    matrix.validate()?;

    let mut output = String::new();
    output.push_str("MAX ");
    for i in 0..matrix.ncols() {
        if i == 0 {
            output.push_str("x0 ");
        } else if i % TERMS_PER_LINE == 0 {
            output.push_str(&format!("\n + x{i} "));
        } else {
            output.push_str(&format!("+ x{i} "));
        }
    }
    output.push('\n');

    output.push_str("ST\n");
    for row in matrix.rows() {
        for (term_index, column) in row.iter().enumerate() {
            if term_index == 0 {
                output.push_str(&format!("x{column} "));
            } else if term_index % TERMS_PER_LINE == 0 {
                output.push_str(&format!("\n + x{column} "));
            } else {
                output.push_str(&format!("+ x{column} "));
            }
        }
        output.push_str("<= 1\n");
    }

    output.push_str("END\n");
    for i in 0..matrix.ncols() {
        output.push_str(&format!("INTEGER x{i}\n"));
    }
    output.push_str("GO\n");
    output.push_str("QUIT\n");
    Ok(output)
}

pub fn parse_lindo_solution(output: &str, ncols: usize) -> Result<Vec<Option<i32>>, XlnLindoError> {
    let tokens: Vec<&str> = output.split_whitespace().collect();
    let mut results = vec![None; ncols];
    let mut index = 0;

    while index < tokens.len() {
        let word = tokens[index];
        if word == "NO" && tokens.get(index + 1) == Some(&"FEASIBLE") {
            return Err(XlnLindoError::NoFeasibleSolution);
        }

        if word != "REDUCED" {
            index += 1;
            continue;
        }

        index += 2;
        while index < tokens.len() {
            let word = tokens[index];
            if word == "ROW" {
                break;
            }

            if let Some(variable) = parse_uppercase_variable(word) {
                if variable >= ncols {
                    return Err(XlnLindoError::VariableOutOfRange { variable, ncols });
                }
                let value_token = tokens
                    .get(index + 1)
                    .ok_or(XlnLindoError::MissingVariableValue { variable })?;
                let value = value_token.parse::<f32>().unwrap_or(0.0);
                results[variable] = Some(round_lindo_value(value));
                index += 2;
            } else {
                index += 1;
            }
        }
    }

    Ok(results)
}

pub fn get_lindo_result<Node: Clone>(
    candidates: &[Node],
    matrix: &SparseCoefficientMatrix,
    lindo_output: &str,
) -> Result<LindoMatchResult<Node>, XlnLindoError> {
    matrix.validate()?;
    let solution = parse_lindo_solution(lindo_output, matrix.ncols())?;
    let mut match1 = Vec::new();
    let mut match2 = Vec::new();

    for (column, value) in solution.iter().enumerate() {
        if *value != Some(1) {
            continue;
        }

        let (first_row, last_row) = matrix
            .column_bounds(column)?
            .ok_or(XlnLindoError::SelectedColumnHasNoRows { column })?;
        let first = candidates
            .get(first_row)
            .ok_or(XlnLindoError::CandidateNodeOutOfRange {
                row: first_row,
                candidates: candidates.len(),
            })?;
        let last = candidates
            .get(last_row)
            .ok_or(XlnLindoError::CandidateNodeOutOfRange {
                row: last_row,
                candidates: candidates.len(),
            })?;
        match1.push(first.clone());
        match2.push(last.clone());
    }

    let num_match = match1.len();
    Ok(LindoMatchResult {
        num_match,
        match1,
        match2,
        report: format!("Total number of matching = {num_match}\n\n"),
    })
}

fn parse_uppercase_variable(word: &str) -> Option<usize> {
    word.strip_prefix('X')?.parse::<usize>().ok()
}

fn round_lindo_value(value: f32) -> i32 {
    value.round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulate_lindo_matches_c_shape_for_objective_constraints_and_integrality() {
        let matrix = SparseCoefficientMatrix::from_rows(
            6,
            vec![vec![0, 1, 2, 3, 4, 5], vec![1, 4], vec![2]],
        );

        let lindo = formulate_lindo(&matrix).unwrap();

        assert_eq!(
            lindo,
            concat!(
                "MAX x0 + x1 + x2 + x3 + x4 \n",
                " + x5 \n",
                "ST\n",
                "x0 + x1 + x2 + x3 + x4 \n",
                " + x5 <= 1\n",
                "x1 + x4 <= 1\n",
                "x2 <= 1\n",
                "END\n",
                "INTEGER x0\n",
                "INTEGER x1\n",
                "INTEGER x2\n",
                "INTEGER x3\n",
                "INTEGER x4\n",
                "INTEGER x5\n",
                "GO\n",
                "QUIT\n",
            )
        );
    }

    #[test]
    fn parse_lindo_solution_finds_last_reduced_cost_section_and_rounds_values() {
        let output = "\
            header REDUCED COST\n\
            X0 1.0 X1 0.0 ROW\n\
            other REDUCED COST\n\
            X0 0.0 ignored X1 0.51 X2 1.49 ROW tail";

        assert_eq!(
            parse_lindo_solution(output, 3).unwrap(),
            vec![Some(0), Some(1), Some(1)]
        );
    }

    #[test]
    fn parse_lindo_solution_reports_no_feasible_solution() {
        assert_eq!(
            parse_lindo_solution("banner NO FEASIBLE solution", 2),
            Err(XlnLindoError::NoFeasibleSolution)
        );
    }

    #[test]
    fn get_lindo_result_maps_selected_columns_to_first_and_last_rows() {
        let candidates = vec!["a", "b", "c", "d"];
        let matrix =
            SparseCoefficientMatrix::from_rows(3, vec![vec![0], vec![0, 2], vec![1], vec![1, 2]]);
        let output = "REDUCED COST X0 1 X1 0 X2 1 ROW";

        let result = get_lindo_result(&candidates, &matrix, output).unwrap();

        assert_eq!(result.num_match, 2);
        assert_eq!(result.match1, vec!["a", "b"]);
        assert_eq!(result.match2, vec!["b", "d"]);
        assert_eq!(result.report, "Total number of matching = 2\n\n");
    }

    #[test]
    fn selected_empty_column_is_reported_explicitly() {
        let candidates = vec!["a", "b"];
        let matrix = SparseCoefficientMatrix::from_rows(2, vec![vec![0], vec![0]]);

        assert_eq!(
            get_lindo_result(&candidates, &matrix, "REDUCED COST X1 1 ROW"),
            Err(XlnLindoError::SelectedColumnHasNoRows { column: 1 })
        );
    }

    #[test]
    fn matrix_validation_reports_bad_column_references() {
        let matrix = SparseCoefficientMatrix::from_rows(2, vec![vec![0, 2]]);

        assert_eq!(
            formulate_lindo(&matrix),
            Err(XlnLindoError::ColumnOutOfRange {
                row: 0,
                column: 2,
                ncols: 2,
            })
        );
    }

    #[test]
    fn sis_bound_entry_reports_dependency_beads_and_sources() {
        let Err(XlnLindoError::MissingSisPorts {
            operation,
            dependencies,
        }) = sis_bound_operation_unavailable("get_Lindo_result over SIS arrays")
        else {
            panic!("expected missing SIS ports");
        };

        assert_eq!(operation, "get_Lindo_result over SIS arrays");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.2"
                && dependency.source_file == "LogicSynthesis/sis/array/array.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.457"
                && dependency.source_file == "LogicSynthesis/sis/sparse/matrix.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.390"
                && dependency.source_file == "LogicSynthesis/sis/pld/xln_merge.c"
        }));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("xln_lindo.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
