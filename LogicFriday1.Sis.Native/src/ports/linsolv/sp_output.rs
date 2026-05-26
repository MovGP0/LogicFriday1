//! Native Rust output helpers for the Sparse linsolv matrix package.
//!
//! The C implementation writes directly to `stdout` or to named files. This
//! port keeps the same row/column ordering and text formats, but returns
//! strings so callers decide where output is written.

use std::collections::BTreeMap;
use std::fmt::{self, Write};

const PRINTER_WIDTH: usize = 80;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatrixOrder {
    Original,
    Reordered,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatrixPrintData {
    Pattern,
    Values,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatrixHeader {
    Omit,
    Include,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SparseOutputElement {
    pub row: usize,
    pub col: usize,
    pub real: f64,
    pub imag: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SparseOutputMatrix {
    size: usize,
    complex: bool,
    factored: bool,
    reordered: bool,
    needs_ordering: bool,
    fillins: usize,
    rel_threshold: f64,
    abs_threshold: f64,
    int_to_ext_row: Vec<usize>,
    int_to_ext_col: Vec<usize>,
    elements: BTreeMap<(usize, usize), SparseOutputElement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SparseOutputError {
    CoordinateOutOfRange {
        row: usize,
        col: usize,
        size: usize,
    },
    ExternalMapLength {
        expected: usize,
        rows: usize,
        cols: usize,
    },
    VectorLength {
        expected: usize,
        actual: usize,
    },
    ImaginaryVectorLength {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for SparseOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoordinateOutOfRange { row, col, size } => {
                write!(f, "matrix coordinate ({row}, {col}) is outside 1..={size}")
            }
            Self::ExternalMapLength {
                expected,
                rows,
                cols,
            } => {
                write!(
                    f,
                    "external row/column maps must both contain {expected} entries, got {rows} and {cols}"
                )
            }
            Self::VectorLength { expected, actual } => {
                write!(f, "vector must contain {expected} entries, got {actual}")
            }
            Self::ImaginaryVectorLength { expected, actual } => {
                write!(
                    f,
                    "imaginary vector must contain {expected} entries, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for SparseOutputError {}

impl SparseOutputMatrix {
    pub fn new(size: usize) -> Self {
        let identity = (0..=size).collect::<Vec<_>>();

        Self {
            size,
            complex: false,
            factored: false,
            reordered: false,
            needs_ordering: true,
            fillins: 0,
            rel_threshold: 1.0e-3,
            abs_threshold: 1.0e-13,
            int_to_ext_row: identity.clone(),
            int_to_ext_col: identity,
            elements: BTreeMap::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_complex(&self) -> bool {
        self.complex
    }

    pub fn set_complex(&mut self, complex: bool) {
        self.complex = complex;
    }

    pub fn set_factored(&mut self, factored: bool) {
        self.factored = factored;
    }

    pub fn set_reordered(&mut self, reordered: bool) {
        self.reordered = reordered;
    }

    pub fn set_needs_ordering(&mut self, needs_ordering: bool) {
        self.needs_ordering = needs_ordering;
    }

    pub fn set_fillins(&mut self, fillins: usize) {
        self.fillins = fillins;
    }

    pub fn set_thresholds(&mut self, relative: f64, absolute: f64) {
        self.rel_threshold = relative;
        self.abs_threshold = absolute;
    }

    pub fn set_external_maps(
        &mut self,
        rows: Vec<usize>,
        cols: Vec<usize>,
    ) -> Result<(), SparseOutputError> {
        if rows.len() != self.size || cols.len() != self.size {
            return Err(SparseOutputError::ExternalMapLength {
                expected: self.size,
                rows: rows.len(),
                cols: cols.len(),
            });
        }

        self.int_to_ext_row = std::iter::once(0).chain(rows).collect();
        self.int_to_ext_col = std::iter::once(0).chain(cols).collect();

        Ok(())
    }

    pub fn add_real(&mut self, row: usize, col: usize, real: f64) -> Result<(), SparseOutputError> {
        self.add_complex(row, col, real, 0.0)
    }

    pub fn add_complex(
        &mut self,
        row: usize,
        col: usize,
        real: f64,
        imag: f64,
    ) -> Result<(), SparseOutputError> {
        self.validate_coordinate(row, col)?;
        self.elements.insert(
            (row, col),
            SparseOutputElement {
                row,
                col,
                real,
                imag,
            },
        );
        Ok(())
    }

    pub fn element(&self, row: usize, col: usize) -> Option<&SparseOutputElement> {
        self.elements.get(&(row, col))
    }

    pub fn elements(&self) -> impl Iterator<Item = &SparseOutputElement> {
        self.elements.values()
    }

    pub fn print(&self, order: MatrixOrder, data: MatrixPrintData, header: MatrixHeader) -> String {
        let mut output = String::new();
        self.write_print(&mut output, order, data, header)
            .expect("writing to String should not fail");
        output
    }

    pub fn file_matrix(
        &self,
        label: &str,
        order: MatrixOrder,
        data: MatrixPrintData,
        header: MatrixHeader,
    ) -> String {
        let mut output = String::new();
        self.write_file_matrix(&mut output, label, order, data, header)
            .expect("writing to String should not fail");
        output
    }

    pub fn file_vector(
        &self,
        rhs: &[f64],
        imaginary_rhs: Option<&[f64]>,
    ) -> Result<String, SparseOutputError> {
        let mut output = String::new();
        self.write_file_vector(&mut output, rhs, imaginary_rhs)?;
        Ok(output)
    }

    pub fn file_stats(&self, label: &str) -> String {
        let mut output = String::new();
        self.write_file_stats(&mut output, label)
            .expect("writing to String should not fail");
        output
    }

    pub fn write_print(
        &self,
        writer: &mut impl Write,
        order: MatrixOrder,
        data: MatrixPrintData,
        header: MatrixHeader,
    ) -> fmt::Result {
        if header == MatrixHeader::Include {
            writeln!(writer, "MATRIX SUMMARY\n")?;
            writeln!(writer, "Size of matrix = {} x {}.", self.size, self.size)?;
            if self.reordered && order == MatrixOrder::Reordered {
                writeln!(writer, "Matrix has been reordered.")?;
            }
            writeln!(writer)?;

            if self.factored {
                writeln!(writer, "Matrix after factorization:")?;
            } else {
                writeln!(writer, "Matrix before factorization:")?;
            }
        }

        let mut columns_per_group = PRINTER_WIDTH;
        if header == MatrixHeader::Include {
            columns_per_group -= 5;
        }
        if data == MatrixPrintData::Values {
            columns_per_group = (columns_per_group + 1) / 10;
        }

        let row_print_order = self.print_order_rows(order);
        let col_print_order = self.print_order_cols(order);
        let mut element_count = 0usize;
        let mut largest_element = 0.0f64;
        let mut smallest_element = f64::MAX;
        let mut start_col = 1usize;

        while start_col <= self.size {
            let stop_col = (start_col + columns_per_group - 1).min(self.size);

            if header == MatrixHeader::Include {
                self.write_group_header(
                    writer,
                    order,
                    data,
                    start_col,
                    stop_col,
                    &col_print_order,
                )?;
            }

            for printed_row in 1..=self.size {
                let row = row_print_order[printed_row];
                self.write_row_label(writer, order, data, header, printed_row, row)?;

                let mut imaginary_elements = Vec::new();
                for printed_col in start_col..=stop_col {
                    let col = col_print_order[printed_col];
                    let element = self.element(row, col);
                    if data == MatrixPrintData::Values {
                        imaginary_elements.push(element);
                    }

                    match element {
                        Some(element) => {
                            if data == MatrixPrintData::Values {
                                write!(writer, " {:>9.3}", element.real)?;
                            } else {
                                writer.write_char('x')?;
                            }

                            let magnitude = element.magnitude(self.complex);
                            if magnitude > largest_element {
                                largest_element = magnitude;
                            }
                            if magnitude < smallest_element && magnitude != 0.0 {
                                smallest_element = magnitude;
                            }
                            element_count += 1;
                        }
                        None => {
                            if data == MatrixPrintData::Values {
                                writer.write_str("       ...")?;
                            } else {
                                writer.write_char('.')?;
                            }
                        }
                    }
                }
                writer.write_char('\n')?;

                if self.complex && data == MatrixPrintData::Values {
                    writer.write_str("    ")?;
                    for element in imaginary_elements {
                        match element {
                            Some(element) => write!(writer, " {:>8.2}j", element.imag)?,
                            None => writer.write_str("          ")?,
                        }
                    }
                    writer.write_char('\n')?;
                }
            }

            start_col = stop_col + 1;
            writer.write_char('\n')?;
        }

        if header == MatrixHeader::Include {
            let stats = self.statistics(element_count, largest_element, smallest_element);
            writeln!(
                writer,
                "\nLargest element in matrix = {:.4}.",
                stats.largest_element
            )?;
            writeln!(
                writer,
                "Smallest element in matrix = {:.4}.",
                stats.smallest_element
            )?;

            if self.factored {
                writeln!(
                    writer,
                    "\nLargest diagonal element = {:.4}.",
                    stats.largest_diagonal
                )?;
                writeln!(
                    writer,
                    "Smallest diagonal element = {:.4}.",
                    stats.smallest_diagonal
                )?;
            } else {
                writeln!(
                    writer,
                    "\nLargest pivot element = {:.4}.",
                    stats.largest_diagonal
                )?;
                writeln!(
                    writer,
                    "Smallest pivot element = {:.4}.",
                    stats.smallest_diagonal
                )?;
            }

            writeln!(writer, "\nDensity = {:.2}%.", stats.density)?;
            if !self.needs_ordering {
                writeln!(writer, "Number of fill-ins = {}.", self.fillins)?;
            }
        }

        writer.write_char('\n')
    }

    pub fn write_file_matrix(
        &self,
        writer: &mut impl Write,
        label: &str,
        order: MatrixOrder,
        data: MatrixPrintData,
        header: MatrixHeader,
    ) -> fmt::Result {
        if header == MatrixHeader::Include {
            if self.factored && data == MatrixPrintData::Values {
                writeln!(
                    writer,
                    "Warning : The following matrix is factored in to LU form."
                )?;
            }
            writeln!(writer, "{label}")?;
            writeln!(
                writer,
                "{}\t{}",
                self.size,
                if self.complex { "complex" } else { "real" }
            )?;
        }

        for element in self.elements_by_column() {
            let (row, col) = match order {
                MatrixOrder::Reordered => (element.row, element.col),
                MatrixOrder::Original => (
                    self.int_to_ext_row[element.row],
                    self.int_to_ext_col[element.col],
                ),
            };

            match data {
                MatrixPrintData::Pattern => {
                    writeln!(writer, "{row}\t{col}")?;
                }
                MatrixPrintData::Values if self.complex => {
                    writeln!(
                        writer,
                        "{row}\t{col}\t{:.15}\t{:.15}",
                        element.real, element.imag
                    )?;
                }
                MatrixPrintData::Values => {
                    writeln!(writer, "{row}\t{col}\t{:.15}", element.real)?;
                }
            }
        }

        if header == MatrixHeader::Include {
            match data {
                MatrixPrintData::Pattern => writeln!(writer, "0\t0")?,
                MatrixPrintData::Values if self.complex => {
                    writeln!(writer, "0\t0\t0.0\t0.0")?;
                }
                MatrixPrintData::Values => {
                    writeln!(writer, "0\t0\t0.0")?;
                }
            }
        }

        Ok(())
    }

    pub fn write_file_vector(
        &self,
        writer: &mut impl Write,
        rhs: &[f64],
        imaginary_rhs: Option<&[f64]>,
    ) -> Result<(), SparseOutputError> {
        if self.complex {
            if let Some(imaginary_rhs) = imaginary_rhs {
                if rhs.len() != self.size {
                    return Err(SparseOutputError::VectorLength {
                        expected: self.size,
                        actual: rhs.len(),
                    });
                }
                if imaginary_rhs.len() != self.size {
                    return Err(SparseOutputError::ImaginaryVectorLength {
                        expected: self.size,
                        actual: imaginary_rhs.len(),
                    });
                }

                for (real, imag) in rhs.iter().zip(imaginary_rhs) {
                    writeln!(writer, "{real:.15}\t{imag:.15}")
                        .expect("format writer should not fail");
                }
            } else {
                if rhs.len() != self.size * 2 {
                    return Err(SparseOutputError::VectorLength {
                        expected: self.size * 2,
                        actual: rhs.len(),
                    });
                }

                for index in 0..self.size {
                    writeln!(writer, "{:.15}\t{:.15}", rhs[index * 2], rhs[index * 2 + 1])
                        .expect("format writer should not fail");
                }
            }
        } else {
            if rhs.len() != self.size {
                return Err(SparseOutputError::VectorLength {
                    expected: self.size,
                    actual: rhs.len(),
                });
            }
            for value in rhs {
                writeln!(writer, "{value:.15}").expect("format writer should not fail");
            }
        }

        Ok(())
    }

    pub fn write_file_stats(&self, writer: &mut impl Write, label: &str) -> fmt::Result {
        let stats = self.statistics_from_elements();

        if !self.factored {
            writeln!(writer, "Matrix has not been factored.")?;
        }
        writeln!(writer, "|||  Starting new matrix  |||")?;
        writeln!(writer, "{label}")?;
        writeln!(
            writer,
            "Matrix is {}.",
            if self.complex { "complex" } else { "real" }
        )?;
        writeln!(writer, "     Size = {}", self.size)?;
        writeln!(
            writer,
            "     Initial number of elements = {}",
            stats.element_count.saturating_sub(self.fillins)
        )?;
        writeln!(
            writer,
            "     Initial average number of elements per row = {:.6}",
            self.average_per_row(stats.element_count.saturating_sub(self.fillins))
        )?;
        writeln!(writer, "     Fill-ins = {}", self.fillins)?;
        writeln!(
            writer,
            "     Average number of fill-ins per row = {:.6}%",
            self.average_per_row(self.fillins)
        )?;
        writeln!(
            writer,
            "     Total number of elements = {}",
            stats.element_count
        )?;
        writeln!(
            writer,
            "     Average number of elements per row = {:.6}",
            self.average_per_row(stats.element_count)
        )?;
        writeln!(writer, "     Density = {:.6}%", stats.density)?;
        writeln!(writer, "     Relative Threshold = {:e}", self.rel_threshold)?;
        writeln!(writer, "     Absolute Threshold = {:e}", self.abs_threshold)?;
        writeln!(writer, "     Largest Element = {:e}", stats.largest_element)?;
        writeln!(
            writer,
            "     Smallest Element = {:e}\n",
            stats.smallest_element
        )
    }

    fn validate_coordinate(&self, row: usize, col: usize) -> Result<(), SparseOutputError> {
        if row == 0 || col == 0 || row > self.size || col > self.size {
            return Err(SparseOutputError::CoordinateOutOfRange {
                row,
                col,
                size: self.size,
            });
        }

        Ok(())
    }

    fn print_order_rows(&self, order: MatrixOrder) -> Vec<usize> {
        match order {
            MatrixOrder::Reordered => (0..=self.size).collect(),
            MatrixOrder::Original => packed_print_order(&self.int_to_ext_row),
        }
    }

    fn print_order_cols(&self, order: MatrixOrder) -> Vec<usize> {
        match order {
            MatrixOrder::Reordered => (0..=self.size).collect(),
            MatrixOrder::Original => packed_print_order(&self.int_to_ext_col),
        }
    }

    fn write_group_header(
        &self,
        writer: &mut impl Write,
        order: MatrixOrder,
        data: MatrixPrintData,
        start_col: usize,
        stop_col: usize,
        col_print_order: &[usize],
    ) -> fmt::Result {
        if data == MatrixPrintData::Values {
            writer.write_str("    ")?;
            for printed_col in start_col..=stop_col {
                let col = col_print_order[printed_col];
                write!(writer, " {:>9}", self.int_to_ext_col[col])?;
            }
            writeln!(writer, "\n")
        } else if order == MatrixOrder::Reordered {
            writeln!(writer, "Columns {start_col} to {stop_col}.")
        } else {
            let first = self.int_to_ext_col[col_print_order[start_col]];
            let last = self.int_to_ext_col[col_print_order[stop_col]];
            writeln!(writer, "Columns {first} to {last}.")
        }
    }

    fn write_row_label(
        &self,
        writer: &mut impl Write,
        order: MatrixOrder,
        data: MatrixPrintData,
        header: MatrixHeader,
        printed_row: usize,
        row: usize,
    ) -> fmt::Result {
        if header == MatrixHeader::Include {
            if order == MatrixOrder::Reordered && data == MatrixPrintData::Pattern {
                write!(writer, "{printed_row:>4}")?;
            } else {
                write!(writer, "{:>4}", self.int_to_ext_row[row])?;
            }
            if data == MatrixPrintData::Pattern {
                writer.write_char(' ')?;
            }
        }

        Ok(())
    }

    fn statistics_from_elements(&self) -> MatrixStatistics {
        self.statistics(self.elements.len(), 0.0, f64::MAX)
    }

    fn statistics(
        &self,
        element_count: usize,
        mut largest_element: f64,
        mut smallest_element: f64,
    ) -> MatrixStatistics {
        if largest_element == 0.0 && smallest_element == f64::MAX {
            for element in self.elements.values() {
                let magnitude = element.magnitude(self.complex);
                if magnitude > largest_element {
                    largest_element = magnitude;
                }
                if magnitude < smallest_element && magnitude != 0.0 {
                    smallest_element = magnitude;
                }
            }
        }

        if smallest_element == f64::MAX {
            smallest_element = largest_element;
        } else {
            smallest_element = smallest_element.min(largest_element);
        }

        let mut largest_diagonal = 0.0;
        let mut smallest_diagonal = f64::MAX;
        for index in 1..=self.size {
            if let Some(element) = self.element(index, index) {
                let magnitude = element.magnitude(self.complex);
                if magnitude > largest_diagonal {
                    largest_diagonal = magnitude;
                }
                if magnitude < smallest_diagonal {
                    smallest_diagonal = magnitude;
                }
            }
        }
        if smallest_diagonal == f64::MAX {
            smallest_diagonal = 0.0;
        }

        MatrixStatistics {
            element_count,
            largest_element,
            smallest_element,
            largest_diagonal,
            smallest_diagonal,
            density: self.density(element_count),
        }
    }

    fn elements_by_column(&self) -> Vec<&SparseOutputElement> {
        let mut elements = self.elements.values().collect::<Vec<_>>();
        elements.sort_by_key(|element| (element.col, element.row));
        elements
    }

    fn density(&self, element_count: usize) -> f64 {
        if self.size == 0 {
            0.0
        } else {
            (element_count as f64 * 100.0) / (self.size * self.size) as f64
        }
    }

    fn average_per_row(&self, count: usize) -> f64 {
        if self.size == 0 {
            0.0
        } else {
            count as f64 / self.size as f64
        }
    }
}

impl SparseOutputElement {
    fn magnitude(&self, complex: bool) -> f64 {
        if complex {
            self.real.abs() + self.imag.abs()
        } else {
            self.real.abs()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct MatrixStatistics {
    element_count: usize,
    largest_element: f64,
    smallest_element: f64,
    largest_diagonal: f64,
    smallest_diagonal: f64,
    density: f64,
}

fn packed_print_order(int_to_ext: &[usize]) -> Vec<usize> {
    let mut external_to_internal = vec![0; int_to_ext.iter().copied().max().unwrap_or(0) + 1];
    for (internal, external) in int_to_ext.iter().copied().enumerate().skip(1) {
        external_to_internal[external] = internal;
    }

    let mut result = Vec::with_capacity(int_to_ext.len());
    result.push(0);
    result.extend(
        external_to_internal
            .into_iter()
            .filter(|internal| *internal != 0),
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_matrix() -> SparseOutputMatrix {
        let mut matrix = SparseOutputMatrix::new(3);
        matrix.add_real(1, 1, 2.0).unwrap();
        matrix.add_real(2, 1, -3.5).unwrap();
        matrix.add_real(3, 2, 4.25).unwrap();
        matrix.add_real(2, 3, 5.0).unwrap();
        matrix
    }

    #[test]
    fn prints_pattern_matrix_in_column_groups() {
        let matrix = sample_matrix();

        let output = matrix.print(
            MatrixOrder::Reordered,
            MatrixPrintData::Pattern,
            MatrixHeader::Include,
        );

        assert!(output.contains("MATRIX SUMMARY"));
        assert!(output.contains("Size of matrix = 3 x 3."));
        assert!(output.contains("Columns 1 to 3."));
        assert!(output.contains("   1 x.."));
        assert!(output.contains("   2 x.x"));
        assert!(output.contains("   3 .x."));
        assert!(output.contains("Density = 44.44%."));
    }

    #[test]
    fn prints_original_order_using_external_maps() {
        let mut matrix = sample_matrix();
        matrix
            .set_external_maps(vec![30, 10, 20], vec![200, 300, 100])
            .unwrap();

        let output = matrix.print(
            MatrixOrder::Original,
            MatrixPrintData::Pattern,
            MatrixHeader::Include,
        );

        assert!(output.contains("Columns 100 to 300."));
        assert!(output.contains("  10 xx."));
        assert!(output.contains("  20 ..x"));
        assert!(output.contains("  30 .x."));
    }

    #[test]
    fn prints_value_matrix_with_complex_imaginary_rows() {
        let mut matrix = SparseOutputMatrix::new(2);
        matrix.set_complex(true);
        matrix.add_complex(1, 1, 1.25, -0.5).unwrap();
        matrix.add_complex(2, 2, -2.0, 3.0).unwrap();

        let output = matrix.print(
            MatrixOrder::Reordered,
            MatrixPrintData::Values,
            MatrixHeader::Include,
        );

        assert!(output.contains("         1         2"));
        assert!(output.contains("     1.250       ..."));
        assert!(output.contains("    -0.50j"));
        assert!(output.contains("    -2.000"));
        assert!(output.contains("     3.00j"));
        assert!(output.contains("Largest element in matrix = 5.0000."));
    }

    #[test]
    fn writes_pattern_file_format_with_terminator() {
        let matrix = sample_matrix();

        let output = matrix.file_matrix(
            "case-a",
            MatrixOrder::Reordered,
            MatrixPrintData::Pattern,
            MatrixHeader::Include,
        );

        assert_eq!(output, "case-a\n3\treal\n1\t1\n2\t1\n3\t2\n2\t3\n0\t0\n");
    }

    #[test]
    fn writes_real_data_file_format_in_original_coordinates() {
        let mut matrix = sample_matrix();
        matrix
            .set_external_maps(vec![30, 10, 20], vec![200, 300, 100])
            .unwrap();

        let output = matrix.file_matrix(
            "case-b",
            MatrixOrder::Original,
            MatrixPrintData::Values,
            MatrixHeader::Include,
        );

        assert!(output.starts_with("case-b\n3\treal\n"));
        assert!(output.contains("30\t200\t2.000000000000000"));
        assert!(output.contains("10\t200\t-3.500000000000000"));
        assert!(output.contains("20\t300\t4.250000000000000"));
        assert!(output.contains("10\t100\t5.000000000000000"));
        assert!(output.ends_with("0\t0\t0.0\n"));
    }

    #[test]
    fn writes_complex_data_file_format() {
        let mut matrix = SparseOutputMatrix::new(1);
        matrix.set_complex(true);
        matrix.set_factored(true);
        matrix.add_complex(1, 1, 2.5, -1.25).unwrap();

        let output = matrix.file_matrix(
            "complex-case",
            MatrixOrder::Reordered,
            MatrixPrintData::Values,
            MatrixHeader::Include,
        );

        assert_eq!(
            output,
            "Warning : The following matrix is factored in to LU form.\ncomplex-case\n1\tcomplex\n1\t1\t2.500000000000000\t-1.250000000000000\n0\t0\t0.0\t0.0\n"
        );
    }

    #[test]
    fn writes_real_and_separated_complex_vectors() {
        let real = sample_matrix().file_vector(&[1.0, 2.0, 3.0], None).unwrap();
        assert_eq!(
            real,
            "1.000000000000000\n2.000000000000000\n3.000000000000000\n"
        );

        let mut complex = SparseOutputMatrix::new(2);
        complex.set_complex(true);
        let output = complex
            .file_vector(&[1.0, 2.0], Some(&[-1.0, -2.0]))
            .unwrap();
        assert_eq!(
            output,
            "1.000000000000000\t-1.000000000000000\n2.000000000000000\t-2.000000000000000\n"
        );
    }

    #[test]
    fn rejects_invalid_vectors_and_coordinates() {
        let mut matrix = SparseOutputMatrix::new(2);

        assert_eq!(
            matrix.add_real(0, 1, 1.0),
            Err(SparseOutputError::CoordinateOutOfRange {
                row: 0,
                col: 1,
                size: 2,
            })
        );
        assert_eq!(
            matrix.file_vector(&[1.0], None),
            Err(SparseOutputError::VectorLength {
                expected: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn writes_stats_with_fillin_adjusted_counts() {
        let mut matrix = sample_matrix();
        matrix.set_factored(true);
        matrix.set_fillins(1);
        matrix.set_thresholds(0.25, 0.125);

        let output = matrix.file_stats("stats-case");

        assert!(output.contains("|||  Starting new matrix  |||"));
        assert!(output.contains("stats-case"));
        assert!(output.contains("Matrix is real."));
        assert!(output.contains("     Initial number of elements = 3"));
        assert!(output.contains("     Fill-ins = 1"));
        assert!(output.contains("     Total number of elements = 4"));
        assert!(output.contains("     Density = 44.444444%"));
        assert!(output.contains("     Relative Threshold = 2.5e-1"));
        assert!(output.contains("     Absolute Threshold = 1.25e-1"));
        assert!(!output.contains("Matrix has not been factored."));
    }

    #[test]
    fn validates_external_map_lengths() {
        let mut matrix = SparseOutputMatrix::new(2);

        assert_eq!(
            matrix.set_external_maps(vec![1], vec![1, 2]),
            Err(SparseOutputError::ExternalMapLength {
                expected: 2,
                rows: 1,
                cols: 2,
            })
        );
    }
}
