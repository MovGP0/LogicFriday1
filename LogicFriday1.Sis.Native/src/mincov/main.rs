//! Native Rust model for `LogicSynthesis/sis/mincov/main.c`.
//!
//! The legacy file owns the standalone `mincov` command-line contract: parse
//! `-c`, `-h`, `-o`, and `-v`; read a sparse matrix from stdin or one file;
//! solve the covering problem; or convert Espresso compressed input to sparse
//! pair format. This port keeps that behavior on owned Rust structures. Wiring
//! to the repository sparse/mincov modules is intentionally left to bead
//! dependency tracking rather than source-level dependency metadata.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Write};

pub const USAGE: &str = "usage: mincov [-ch] [-v #]\n   -c\t\tread espresso 'compressed' pi table\n   -h\t\theuristic covering\n   -v n\t\tset verbose level to 'n' (e.g., 5)\n";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MincovOptions
{
    pub compressed: bool,
    pub heuristic: bool,
    pub opt: i32,
    pub verbose: i32,
    pub input: MincovInput,
}

impl Default for MincovOptions
{
    fn default() -> Self
    {
        Self
        {
            compressed: false,
            heuristic: false,
            opt: 0,
            verbose: 0,
            input: MincovInput::Stdin,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MincovInput
{
    Stdin,
    File(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MincovRun
{
    pub stdout: String,
    pub verbose: i32,
}

pub fn parse_mincov_args<I, S>(args: I) -> Result<MincovOptions, MincovError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();
    let mut options = MincovOptions::default();
    let mut operands = Vec::new();
    let mut index = 0;

    while index < args.len()
    {
        let arg = &args[index];
        if arg == "--"
        {
            operands.extend(args[index + 1..].iter().cloned());
            break;
        }
        if !arg.starts_with('-') || arg == "-"
        {
            operands.push(arg.clone());
            index += 1;
            continue;
        }

        let chars: Vec<char> = arg[1..].chars().collect();
        if chars.is_empty()
        {
            operands.push(arg.clone());
            index += 1;
            continue;
        }

        let mut char_index = 0;
        while char_index < chars.len()
        {
            match chars[char_index]
            {
                'c' =>
                {
                    options.compressed = true;
                    char_index += 1;
                }
                'h' =>
                {
                    options.heuristic = true;
                    char_index += 1;
                }
                'o' | 'v' =>
                {
                    let option = chars[char_index];
                    let value = if char_index + 1 < chars.len()
                    {
                        chars[char_index + 1..].iter().collect::<String>()
                    }
                    else
                    {
                        index += 1;
                        args.get(index)
                            .cloned()
                            .ok_or(MincovError::MissingOptionValue(option))?
                    };

                    let parsed = value.parse::<i32>().map_err(|_| {
                        MincovError::InvalidInteger
                        {
                            option,
                            value: value.clone(),
                        }
                    })?;
                    if option == 'o'
                    {
                        options.opt = parsed;
                    }
                    else
                    {
                        options.verbose = parsed;
                    }
                    char_index = chars.len();
                }
                option => return Err(MincovError::UnknownOption(option)),
            }
        }

        index += 1;
    }

    options.input = match operands.as_slice()
    {
        [] => MincovInput::Stdin,
        [filename] if filename == "-" => MincovInput::Stdin,
        [filename] => MincovInput::File(filename.clone()),
        _ => return Err(MincovError::TooManyInputFiles),
    };

    Ok(options)
}

pub fn run_mincov_text(input: &str, options: &MincovOptions) -> Result<MincovRun, MincovError>
{
    let matrix = if options.compressed
    {
        SparseMatrix::from_compressed(input)?
    }
    else
    {
        SparseMatrix::from_pairs(input)?
    };

    let stdout = match options.opt
    {
        0 =>
        {
            let cover = minimum_cover(&matrix, options.heuristic)?;
            let mut output = String::new();
            if matrix.row_count() < 25
            {
                output.push_str(&matrix.print_layout());
            }
            write!(&mut output, "Solution is{}", format_cover(&cover))
                .expect("writing to a String should not fail");
            output.push('\n');
            output
        }
        1 => matrix.write_pairs(),
        _ => return Err(MincovError::UsageRequested),
    };

    Ok(MincovRun
    {
        stdout,
        verbose: options.verbose,
    })
}

pub fn format_cover(cover: &[usize]) -> String
{
    let mut output = String::new();
    for col in cover
    {
        write!(&mut output, " {col}").expect("writing to a String should not fail");
    }
    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseMatrix
{
    rows: BTreeMap<usize, BTreeSet<usize>>,
    cols: BTreeMap<usize, BTreeSet<usize>>,
    rows_size: usize,
    cols_size: usize,
}

impl Default for SparseMatrix
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl SparseMatrix
{
    pub fn new() -> Self
    {
        Self
        {
            rows: BTreeMap::new(),
            cols: BTreeMap::new(),
            rows_size: 0,
            cols_size: 0,
        }
    }

    pub fn with_size(rows: usize, cols: usize) -> Self
    {
        let mut matrix = Self::new();
        matrix.resize(rows, cols);
        matrix
    }

    pub fn resize(&mut self, row: usize, col: usize)
    {
        if row >= self.rows_size
        {
            self.rows_size = (self.rows_size * 2).max(row + 1);
        }
        if col >= self.cols_size
        {
            self.cols_size = (self.cols_size * 2).max(col + 1);
        }
    }

    pub fn row_count(&self) -> usize
    {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize
    {
        self.cols.len()
    }

    pub fn contains(&self, row: usize, col: usize) -> bool
    {
        self.rows
            .get(&row)
            .is_some_and(|cols| cols.contains(&col))
    }

    pub fn insert(&mut self, row: usize, col: usize)
    {
        if row >= self.rows_size || col >= self.cols_size
        {
            self.resize(row, col);
        }
        if self.rows.entry(row).or_default().insert(col)
        {
            self.cols.entry(col).or_default().insert(row);
        }
    }

    pub fn rows(&self) -> impl Iterator<Item = (usize, &BTreeSet<usize>)>
    {
        self.rows.iter().map(|(row, cols)| (*row, cols))
    }

    pub fn cols(&self) -> impl Iterator<Item = (usize, &BTreeSet<usize>)>
    {
        self.cols.iter().map(|(col, rows)| (*col, rows))
    }

    pub fn from_pairs(input: &str) -> Result<Self, MincovError>
    {
        let mut matrix = Self::new();
        for (line_index, line) in input.lines().enumerate()
        {
            let line = line.trim();
            if line.is_empty()
            {
                continue;
            }

            let mut parts = line.split_whitespace();
            let row = parse_usize(parts.next(), line_index + 1)?;
            let col = parse_usize(parts.next(), line_index + 1)?;
            if parts.next().is_some()
            {
                return Err(MincovError::InvalidSparsePair
                {
                    line: line_index + 1,
                });
            }
            matrix.insert(row, col);
        }
        Ok(matrix)
    }

    pub fn from_compressed(input: &str) -> Result<Self, MincovError>
    {
        let mut tokens = input.split_whitespace();
        let nrows = tokens
            .next()
            .and_then(|token| token.parse::<usize>().ok())
            .ok_or(MincovError::InvalidCompressedSize)?;
        let ncols = tokens
            .next()
            .and_then(|token| token.parse::<usize>().ok())
            .ok_or(MincovError::InvalidCompressedSize)?;
        let mut matrix = Self::with_size(nrows, ncols);

        for row in 0..nrows
        {
            let _row_header = parse_hex_word(tokens.next(), row, None)?;
            for (block_index, col_base) in (0..ncols).step_by(32).enumerate()
            {
                let mut word = parse_hex_word(tokens.next(), row, Some(block_index))?;
                let mut col = col_base;
                while word != 0
                {
                    if word & 1 != 0
                    {
                        matrix.insert(row, col);
                    }
                    word >>= 1;
                    col += 1;
                }
            }
        }

        Ok(matrix)
    }

    pub fn write_pairs(&self) -> String
    {
        let mut output = String::new();
        for (row, cols) in &self.rows
        {
            for col in cols
            {
                writeln!(&mut output, "{row} {col}")
                    .expect("writing to a String should not fail");
            }
        }
        output
    }

    pub fn print_layout(&self) -> String
    {
        let Some(last_col) = self.cols.keys().next_back().copied() else
        {
            return String::new();
        };

        let mut output = String::new();
        if last_col >= 100
        {
            output.push_str("    ");
            for col in self.cols.keys()
            {
                write!(&mut output, "{}", (col / 100) % 10)
                    .expect("writing to a String should not fail");
            }
            output.push('\n');
        }
        if last_col >= 10
        {
            output.push_str("    ");
            for col in self.cols.keys()
            {
                write!(&mut output, "{}", (col / 10) % 10)
                    .expect("writing to a String should not fail");
            }
            output.push('\n');
        }

        output.push_str("    ");
        for col in self.cols.keys()
        {
            write!(&mut output, "{}", col % 10).expect("writing to a String should not fail");
        }
        output.push('\n');

        output.push_str("    ");
        for _ in self.cols.keys()
        {
            output.push('-');
        }
        output.push('\n');

        for (row, cols) in &self.rows
        {
            write!(&mut output, "{row:3}:").expect("writing to a String should not fail");
            for col in self.cols.keys()
            {
                output.push(if cols.contains(col) { '1' } else { '.' });
            }
            output.push('\n');
        }

        output
    }
}

fn parse_usize(value: Option<&str>, line: usize) -> Result<usize, MincovError>
{
    value
        .and_then(|token| token.parse::<usize>().ok())
        .ok_or(MincovError::InvalidSparsePair
        {
            line,
        })
}

fn parse_hex_word(
    value: Option<&str>,
    row: usize,
    block: Option<usize>,
) -> Result<u64, MincovError>
{
    value
        .and_then(|token| u64::from_str_radix(token, 16).ok())
        .ok_or(MincovError::InvalidCompressedWord
        {
            row,
            block,
        })
}

pub fn minimum_cover(matrix: &SparseMatrix, heuristic: bool) -> Result<Vec<usize>, MincovError>
{
    let mut uncovered: BTreeSet<usize> = matrix.rows.keys().copied().collect();
    let mut required = BTreeSet::new();

    for (row, cols) in matrix.rows()
    {
        if cols.is_empty()
        {
            return Err(MincovError::UncoverableRow(row));
        }
        if cols.len() == 1
        {
            required.insert(*cols.iter().next().expect("single-column row has one entry"));
        }
    }

    for col in &required
    {
        remove_covered_rows(matrix, *col, &mut uncovered);
    }

    let mut cover = required.into_iter().collect::<Vec<_>>();
    if heuristic
    {
        greedy_cover(matrix, &mut uncovered, &mut cover)?;
    }
    else
    {
        let candidates = candidate_columns(matrix, &uncovered, &cover);
        let suffix = exact_cover(matrix, &uncovered, &candidates)?;
        cover.extend(suffix);
    }

    cover.sort_unstable();
    Ok(cover)
}

fn candidate_columns(
    matrix: &SparseMatrix,
    uncovered: &BTreeSet<usize>,
    selected: &[usize],
) -> Vec<usize>
{
    let selected: BTreeSet<usize> = selected.iter().copied().collect();
    matrix
        .cols()
        .filter(|(col, rows)| {
            !selected.contains(col) && rows.iter().any(|row| uncovered.contains(row))
        })
        .map(|(col, _)| col)
        .collect()
}

fn greedy_cover(
    matrix: &SparseMatrix,
    uncovered: &mut BTreeSet<usize>,
    cover: &mut Vec<usize>,
) -> Result<(), MincovError>
{
    while !uncovered.is_empty()
    {
        let best = matrix
            .cols()
            .filter_map(|(col, rows)| {
                let score = rows.iter().filter(|row| uncovered.contains(row)).count();
                (score > 0).then_some((score, col))
            })
            .max_by(|(left_score, left_col), (right_score, right_col)| {
                left_score
                    .cmp(right_score)
                    .then_with(|| right_col.cmp(left_col))
            })
            .map(|(_, col)| col)
            .ok_or_else(|| first_uncovered_error(uncovered))?;
        cover.push(best);
        remove_covered_rows(matrix, best, uncovered);
    }
    Ok(())
}

fn exact_cover(
    matrix: &SparseMatrix,
    uncovered: &BTreeSet<usize>,
    candidates: &[usize],
) -> Result<Vec<usize>, MincovError>
{
    if uncovered.is_empty()
    {
        return Ok(Vec::new());
    }

    let mut best: Option<Vec<usize>> = None;
    let mut chosen = Vec::new();
    search_cover(matrix, uncovered, candidates, &mut chosen, &mut best);
    best.ok_or_else(|| first_uncovered_error(uncovered))
}

fn search_cover(
    matrix: &SparseMatrix,
    uncovered: &BTreeSet<usize>,
    candidates: &[usize],
    chosen: &mut Vec<usize>,
    best: &mut Option<Vec<usize>>,
)
{
    if uncovered.is_empty()
    {
        replace_best_if_better(chosen, best);
        return;
    }
    if best.as_ref().is_some_and(|current| chosen.len() >= current.len())
    {
        return;
    }

    let Some(row) = uncovered.iter().next().copied() else
    {
        return;
    };
    let Some(row_cols) = matrix.rows.get(&row) else
    {
        return;
    };

    for col in candidates
    {
        if chosen.contains(col) || !row_cols.contains(col)
        {
            continue;
        }

        let mut next_uncovered = uncovered.clone();
        remove_covered_rows(matrix, *col, &mut next_uncovered);
        chosen.push(*col);
        search_cover(
            matrix,
            &next_uncovered,
            candidates,
            chosen,
            best,
        );
        chosen.pop();
    }
}

fn replace_best_if_better(chosen: &[usize], best: &mut Option<Vec<usize>>)
{
    let mut candidate = chosen.to_vec();
    candidate.sort_unstable();
    match best
    {
        Some(current)
            if current.len() < candidate.len()
                || (current.len() == candidate.len() && *current <= candidate) => {}
        _ => *best = Some(candidate),
    }
}

fn remove_covered_rows(matrix: &SparseMatrix, col: usize, uncovered: &mut BTreeSet<usize>)
{
    if let Some(rows) = matrix.cols.get(&col)
    {
        for row in rows
        {
            uncovered.remove(row);
        }
    }
}

fn first_uncovered_error(uncovered: &BTreeSet<usize>) -> MincovError
{
    MincovError::UncoverableRow(*uncovered.iter().next().unwrap_or(&0))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MincovError
{
    MissingOptionValue(char),
    InvalidInteger
    {
        option: char,
        value: String,
    },
    UnknownOption(char),
    TooManyInputFiles,
    InvalidSparsePair
    {
        line: usize,
    },
    InvalidCompressedSize,
    InvalidCompressedWord
    {
        row: usize,
        block: Option<usize>,
    },
    UncoverableRow(usize),
    UsageRequested,
}

impl fmt::Display for MincovError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingOptionValue(option) => write!(f, "missing value for option -{option}"),
            Self::InvalidInteger
            {
                option,
                value,
            } => write!(f, "invalid integer {value:?} for option -{option}"),
            Self::UnknownOption(option) => write!(f, "unknown option -{option}\n{USAGE}"),
            Self::TooManyInputFiles => write!(f, "too many input files\n{USAGE}"),
            Self::InvalidSparsePair
            {
                line,
            } => write!(f, "invalid sparse row/column pair on line {line}"),
            Self::InvalidCompressedSize => write!(f, "invalid compressed matrix size"),
            Self::InvalidCompressedWord
            {
                row,
                block,
            } => match block
            {
                Some(block) => write!(f, "invalid compressed word for row {row}, block {block}"),
                None => write!(f, "invalid compressed row header for row {row}"),
            },
            Self::UncoverableRow(row) => write!(f, "row {row} has no covering column"),
            Self::UsageRequested => write!(f, "{USAGE}"),
        }
    }
}

impl Error for MincovError {}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn parses_defaults_flags_values_and_input_file()
    {
        let options = parse_mincov_args(["-ch", "-o1", "-v", "5", "matrix.pi"]).unwrap();

        assert_eq!(
            options,
            MincovOptions
            {
                compressed: true,
                heuristic: true,
                opt: 1,
                verbose: 5,
                input: MincovInput::File("matrix.pi".to_owned()),
            }
        );
    }

    #[test]
    fn defaults_to_standard_input_and_rejects_extra_operands()
    {
        assert_eq!(parse_mincov_args(std::iter::empty::<&str>()).unwrap().input, MincovInput::Stdin);
        assert_eq!(
            parse_mincov_args(["a", "b"]),
            Err(MincovError::TooManyInputFiles)
        );
    }

    #[test]
    fn reads_and_writes_sparse_pair_format_in_sorted_order()
    {
        let matrix = SparseMatrix::from_pairs("3 2\n1 5\n3 2\n").unwrap();

        assert_eq!(matrix.row_count(), 2);
        assert!(matrix.contains(1, 5));
        assert_eq!(matrix.write_pairs(), "1 5\n3 2\n");
    }

    #[test]
    fn reads_espresso_compressed_format()
    {
        let matrix = SparseMatrix::from_compressed("2 35 0 80000001 4 0 2 0").unwrap();

        assert!(matrix.contains(0, 0));
        assert!(matrix.contains(0, 31));
        assert!(matrix.contains(0, 34));
        assert!(matrix.contains(1, 1));
        assert_eq!(matrix.write_pairs(), "0 0\n0 31\n0 34\n1 1\n");
    }

    #[test]
    fn prints_matrix_layout_like_sparse_package()
    {
        let matrix = SparseMatrix::from_pairs("2 0\n2 12\n4 12\n").unwrap();

        assert_eq!(
            matrix.print_layout(),
            "    01\n    02\n    --\n  2:11\n  4:.1\n"
        );
    }

    #[test]
    fn exact_cover_finds_minimum_cardinality_columns()
    {
        let matrix = SparseMatrix::from_pairs("0 0\n0 1\n1 1\n1 2\n2 2\n").unwrap();

        assert_eq!(minimum_cover(&matrix, false).unwrap(), vec![0, 2]);
    }

    #[test]
    fn heuristic_cover_keeps_forced_columns_before_greedy_selection()
    {
        let matrix = SparseMatrix::from_pairs("0 0\n0 1\n1 1\n1 2\n2 2\n").unwrap();

        assert_eq!(minimum_cover(&matrix, true).unwrap(), vec![0, 2]);
    }

    #[test]
    fn exact_cover_allows_later_required_rows_to_use_earlier_columns()
    {
        let matrix = SparseMatrix::from_pairs("0 2\n0 3\n1 1\n").unwrap();

        assert_eq!(minimum_cover(&matrix, false).unwrap(), vec![1, 2]);
    }

    #[test]
    fn command_solve_prints_small_matrix_and_solution()
    {
        let options = parse_mincov_args(["-v3"]).unwrap();
        let run = run_mincov_text("0 0\n1 0\n1 2\n", &options).unwrap();

        assert_eq!(run.verbose, 3);
        assert_eq!(
            run.stdout,
            "    02\n    --\n  0:1.\n  1:11\nSolution is 0\n"
        );
    }

    #[test]
    fn option_one_converts_compressed_to_sparse_pairs()
    {
        let options = parse_mincov_args(["-c", "-o", "1"]).unwrap();
        let run = run_mincov_text("1 33 0 80000001 1", &options).unwrap();

        assert_eq!(run.stdout, "0 0\n0 31\n0 32\n");
    }
}
