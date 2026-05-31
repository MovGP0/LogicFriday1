//! Source-aligned cube and cover output routines from Espresso `cvrout.c`.
//!
//! The C module writes PLA headers, PLA rows, cube text, equation text, and a
//! few debugging formats. This module exposes the same behavior as pure string
//! producing helpers over the native [`Pla`], [`Cover`], and [`Cube`] types.

use crate::pla::{Cover, Cube, InputPart, Pla};

/// Output selector understood by `fprint_pla`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Pla(u16),
    EqnTott,
    Pleasure,
    Kiss,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputError {
    Unsupported(&'static str),
    MissingOutputFunction,
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(mode) => write!(f, "unsupported output mode: {mode}"),
            Self::MissingOutputFunction => write!(f, "cannot output equations without outputs"),
        }
    }
}

impl std::error::Error for OutputError {}

/// `fprint_pla` equivalent. The C version writes to `FILE *`; Rust returns text.
pub fn fprint_pla(pla: &Pla, output: OutputFormat) -> Result<String, OutputError> {
    match output {
        OutputFormat::Pla(output_type) => Ok(pla.write(output_type)),
        OutputFormat::EqnTott => eqn_output(pla),
        OutputFormat::Pleasure => Ok(pls_output(pla)),
        OutputFormat::Kiss => Ok(kiss_output(pla)),
    }
}

/// `fpr_header` equivalent.
pub fn fpr_header(pla: &Pla, output_type: u16) -> String {
    let mut out = String::new();
    pla.write_header(output_type, &mut out);
    out
}

/// `fmt_cube` equivalent for binary PLA cubes.
pub fn fmt_cube(cube: &Cube, out_map: &str) -> String {
    cube.format(out_map)
}

/// `print_cube` equivalent, including the trailing newline.
pub fn print_cube(cube: &Cube, out_map: &str) -> String {
    format!("{}\n", fmt_cube(cube, out_map))
}

/// `print_expanded_cube` equivalent for the binary/native PLA representation.
pub fn print_expanded_cube(cube: &Cube, phase: Option<&[bool]>) -> String {
    let mut out = String::new();
    for input in &cube.inputs {
        out.push_str(match input {
            InputPart::Zero => "1~",
            InputPart::One => "~1",
            InputPart::Dash => "11",
            InputPart::Empty => "~~",
        });
    }
    out.push(' ');
    for (index, selected) in cube.outputs.iter().enumerate() {
        let positive_phase = phase
            .and_then(|bits| bits.get(index))
            .copied()
            .unwrap_or(true);
        out.push(match (*selected, positive_phase) {
            (false, _) => '~',
            (true, true) => '1',
            (true, false) => '0',
        });
    }
    out.push('\n');
    out
}

/// `eqn_output` equivalent for binary-valued PLAs.
pub fn eqn_output(pla: &Pla) -> Result<String, OutputError> {
    if pla.output_count == 0 {
        return Err(OutputError::MissingOutputFunction);
    }

    let input_labels = input_labels(pla);
    let output_labels = output_labels(pla);
    let mut out = String::new();

    for (output_index, output_label) in output_labels.iter().enumerate() {
        out.push_str(output_label);
        out.push_str(" = ");
        let mut first_or = true;

        for cube in &pla.f.cubes {
            if !cube.outputs.get(output_index).copied().unwrap_or(false) {
                continue;
            }
            if first_or {
                out.push('(');
            } else {
                out.push_str(" | (");
            }
            first_or = false;

            let mut first_and = true;
            for (part, label) in cube.inputs.iter().zip(&input_labels) {
                match part {
                    InputPart::Zero => {
                        if !first_and {
                            out.push('&');
                        }
                        first_and = false;
                        out.push('!');
                        out.push_str(label);
                    }
                    InputPart::One => {
                        if !first_and {
                            out.push('&');
                        }
                        first_and = false;
                        out.push_str(label);
                    }
                    InputPart::Dash | InputPart::Empty => {}
                }
            }
            out.push(')');
        }
        out.push_str(";\n\n");
    }

    Ok(out)
}

/// `pls_output` equivalent over the native binary PLA model.
pub fn pls_output(pla: &Pla) -> String {
    let mut out = String::new();
    out.push_str(".option unmerged\n");
    out.push_str(&pls_label(pla));
    out.push('\n');
    out.push_str(&pls_group(pla));
    out.push_str(&format!(".p {}\n", pla.f.cubes.len()));
    let phase = pla.phase.as_deref();
    for cube in &pla.f.cubes {
        out.push_str(&print_expanded_cube(cube, phase));
    }
    out.push_str(".end\n");
    out
}

/// `pls_group` equivalent.
pub fn pls_group(pla: &Pla) -> String {
    let labels = input_labels(pla);
    let mut out = String::from(".group");
    for label in labels {
        out.push_str(" (");
        out.push_str(&format!("{label}.bar {label}"));
        out.push(')');
    }
    out.push('\n');
    out
}

/// `pls_label` equivalent.
pub fn pls_label(pla: &Pla) -> String {
    let mut parts = Vec::new();
    for label in input_labels(pla) {
        parts.push(format!("{label}.bar"));
        parts.push(label);
    }
    parts.extend(output_labels(pla));
    format!(".label {}", parts.join(" "))
}

/// `kiss_output` equivalent for the native binary PLA model.
pub fn kiss_output(pla: &Pla) -> String {
    let mut out = String::new();
    for cube in &pla.f.cubes {
        out.push_str(&print_cube(cube, "~1"));
    }
    for cube in &pla.d.cubes {
        out.push_str(&print_cube(cube, "~2"));
    }
    out
}

/// `debug1_print` equivalent for a native cover.
pub fn debug1_print(cover: &Cover, name: &str, num: usize) -> String {
    let mut out = format!("{name}[{num}]: ord(T)={}\n", cover.cubes.len());
    for (index, cube) in cover.cubes.iter().enumerate() {
        out.push_str(&format!("{:4}. {}\n", index + 1, fmt_cube(cube, "01")));
    }
    out
}

/// `debug_print` equivalent for native cube slices.
pub fn debug_print(cubes: &[Cube], name: &str, level: usize, verbose: bool) -> String {
    let mut out = String::new();
    if verbose && level == 0 {
        out.push('\n');
    }
    out.push_str(&format!("{name}[{level}]: ord(T)={}\n", cubes.len()));
    if verbose {
        for (index, cube) in cubes.iter().enumerate() {
            out.push_str(&format!("{:4}. {}\n", index + 1, pc1(cube)));
        }
    }
    out
}

/// `cprint` equivalent.
pub fn cprint(cover: &Cover) -> String {
    cover
        .cubes
        .iter()
        .map(|cube| print_cube(cube, "01"))
        .collect()
}

/// `pc1` equivalent.
pub fn pc1(cube: &Cube) -> String {
    fmt_cube(cube, "01")
}

/// `pc2` equivalent.
pub fn pc2(cube: &Cube) -> String {
    fmt_cube(cube, "01")
}

/// `makeup_labels` equivalent for the compact native PLA labels.
pub fn makeup_labels(pla: &Pla) -> (Vec<String>, Vec<String>) {
    (input_labels(pla), output_labels(pla))
}

fn input_labels(pla: &Pla) -> Vec<String> {
    if pla.input_labels.len() == pla.input_count {
        return pla.input_labels.clone();
    }
    (0..pla.input_count)
        .map(|index| format!("v{index}"))
        .collect()
}

fn output_labels(pla: &Pla) -> Vec<String> {
    if pla.output_labels.len() == pla.output_count {
        return pla.output_labels.clone();
    }
    (0..pla.output_count)
        .map(|index| format!("v{}.{index}", pla.input_count))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pla::{F_TYPE, FD_TYPE, FDR_TYPE};

    #[test]
    fn fprint_pla_delegates_header_rows_and_end_marker() {
        let pla = Pla::parse(".i 2\n.o 1\n.type fd\n.ilb a b\n.ob y\n0- 1\n10 -\n.e\n").unwrap();

        assert_eq!(
            fprint_pla(&pla, OutputFormat::Pla(FD_TYPE)).unwrap(),
            ".type fd\n.i 2\n.o 1\n.ilb a b\n.ob y\n.p 2\n0- 1\n10 2\n.end\n"
        );
        assert_eq!(
            fpr_header(&pla, FDR_TYPE),
            ".type fdr\n.i 2\n.o 1\n.ilb a b\n.ob y\n"
        );
    }

    #[test]
    fn cube_formatters_match_espresso_maps() {
        let pla = Pla::parse(".i 2\n.o 2\n0- 10\n.e\n").unwrap();
        let cube = &pla.f.cubes[0];

        assert_eq!(fmt_cube(cube, "~1"), "0- 1~");
        assert_eq!(print_cube(cube, "01"), "0- 10\n");
        assert_eq!(print_expanded_cube(cube, None), "1~11 1~\n");
    }

    #[test]
    fn eqntott_uses_labels_and_binary_literals() {
        let pla = Pla::parse(".i 2\n.o 1\n.ilb a b\n.ob y\n00 1\n1- 1\n.e\n").unwrap();

        assert_eq!(eqn_output(&pla).unwrap(), "y = (!a&!b) | (a);\n\n");
    }

    #[test]
    fn pleasure_output_contains_labels_groups_and_expanded_rows() {
        let pla = Pla::parse(".i 1\n.o 1\n.ilb a\n.ob y\n.phase 0\n0 1\n.e\n").unwrap();

        assert_eq!(
            pls_output(&pla),
            ".option unmerged\n.label a.bar a y\n.group (a.bar a)\n.p 1\n1~ 0\n.end\n"
        );
    }

    #[test]
    fn debug_cover_prints_ord_and_cubes() {
        let pla = Pla::parse(".i 1\n.o 1\n0 1\n.e\n").unwrap();

        assert_eq!(debug1_print(&pla.f, "F", 0), "F[0]: ord(T)=1\n   1. 0 1\n");
        assert_eq!(cprint(&pla.f), "0 1\n");
        assert_eq!(pc1(&pla.f.cubes[0]), "0 1");
        assert_eq!(pc2(&pla.f.cubes[0]), "0 1");
        assert_eq!(
            debug_print(&pla.f.cubes, "T", 0, true),
            "\nT[0]: ord(T)=1\n   1. 0 1\n"
        );
    }

    #[test]
    fn f_type_output_uses_tpla_terminator() {
        let pla = Pla::parse(".i 1\n.o 1\n0 1\n.e\n").unwrap();

        assert_eq!(
            fprint_pla(&pla, OutputFormat::Pla(F_TYPE)).unwrap(),
            ".i 1\n.o 1\n.p 1\n0 1\n.e\n"
        );
    }
}
