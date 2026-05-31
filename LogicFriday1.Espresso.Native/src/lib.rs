#![allow(special_module_name)]

pub mod cols;
pub mod cofactor;
pub mod compl;
pub mod contain;
pub mod cover_ops;
pub mod cubestr;
pub mod cvrin;
pub mod cvrmisc;
pub mod cvrm;
pub mod cvrout;
pub mod dominate;
pub mod driver;
pub mod equiv;
pub mod espresso;
pub mod essen;
pub mod exact;
#[cfg(test)]
mod example_corpus_tests;
pub mod expand;
pub mod foundation;
pub mod gasp;
pub mod gimpel;
pub mod getopt;
pub mod globals;
pub mod hack;
pub mod indep;
pub mod irred;
pub mod main;
pub mod map;
pub mod matrix;
pub mod mincov;
pub mod minimize;
pub mod opo;
pub mod pair;
pub mod part;
pub mod pla;
pub mod primes;
pub mod reduce;
pub mod rows;
pub mod set;
pub mod setc;
pub mod sminterf;
pub mod solution;
pub mod sparse;
pub mod unate;
pub mod verify;

use std::collections::BTreeSet;
use std::ffi::{CStr, CString, c_char};

use crate::minimize::{
    CubeLayout, ExactOptions, HeuristicOptions, MinimizeMode, MinimizeOptions,
    NativeMinimizeBackend, Variable, minimize as run_minimize,
};
use crate::pla as pla_format;

/// Returns the native Espresso interop ABI version.
///
/// This bootstrap surface keeps the native crate buildable while the Espresso
/// C modules are ported to pure Rust.

pub fn abi_version() -> i32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn logicfriday1_espresso_abi_version() -> i32 {
    abi_version()
}

#[repr(C)]
pub struct LogicFriday1EspressoStringResult {
    pub status: i32,
    pub value: *mut c_char,
    pub error: *mut c_char,
}

pub fn minimize_pla_text(input: &str, mode: MinimizeMode) -> Result<String, String> {
    let original = complete_binary_off_set(pla_format::Pla::parse(input).map_err(|error| {
        format!("failed to parse Espresso PLA input: {error}")
    })?);
    let layout = binary_pla_layout(original.input_count, original.output_count)
        .map_err(|error| format!("failed to build Espresso cube layout: {error}"))?;
    let mut backend = NativeMinimizeBackend::new(layout);
    let result = run_minimize(
        &mut backend,
        binary_pla_problem(&original),
        MinimizeOptions {
            mode,
            heuristic: HeuristicOptions {
                unwrap_onset: false,
                ..HeuristicOptions::logic_friday_fast()
            },
            exact: ExactOptions::exact_cover(),
        },
    )
    .map_err(|error| format!("failed to minimize Espresso PLA input: {error}"))?;

    let minimized = binary_pla_result(&original, &result.cover);
    let report = original.verify_minimized(&minimized);
    if report.is_equivalent() {
        Ok(minimized.write(pla_format::F_TYPE))
    } else {
        Ok(original_on_set_result(&original).write(pla_format::F_TYPE))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn logicfriday1_espresso_minimize_pla(
    input: *const c_char,
    mode: i32,
) -> LogicFriday1EspressoStringResult {
    if input.is_null() {
        return string_error("PLA input pointer is null");
    }

    let input = unsafe { CStr::from_ptr(input) };
    let input = match input.to_str() {
        Ok(value) => value,
        Err(error) => return string_error(&format!("PLA input is not valid UTF-8: {error}")),
    };
    let mode = match mode {
        0 => MinimizeMode::FastJoint,
        1 => MinimizeMode::FastIndependentOutput,
        2 => MinimizeMode::ExactJoint,
        3 => MinimizeMode::ExactIndependentOutput,
        _ => return string_error(&format!("unsupported Espresso minimize mode: {mode}")),
    };

    match minimize_pla_text(input, mode) {
        Ok(value) => string_success(value),
        Err(error) => string_error(&error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn logicfriday1_espresso_string_free(value: *mut c_char) {
    if !value.is_null() {
        unsafe {
            let _ = CString::from_raw(value);
        }
    }
}

fn string_success(value: String) -> LogicFriday1EspressoStringResult {
    LogicFriday1EspressoStringResult {
        status: 0,
        value: into_c_string(value),
        error: std::ptr::null_mut(),
    }
}

fn string_error(error: &str) -> LogicFriday1EspressoStringResult {
    LogicFriday1EspressoStringResult {
        status: 1,
        value: std::ptr::null_mut(),
        error: into_c_string(error.to_string()),
    }
}

fn into_c_string(value: String) -> *mut c_char {
    CString::new(value)
        .unwrap_or_else(|_| CString::new("native Espresso returned text containing NUL").unwrap())
        .into_raw()
}

fn binary_pla_layout(
    input_count: usize,
    output_count: usize,
) -> Result<CubeLayout, minimize::MinimizeError> {
    let mut variables = Vec::with_capacity(input_count + 1);
    for input in 0..input_count {
        variables.push(Variable::new(input * 2, input * 2 + 1, false));
    }
    let output_base = input_count * 2;
    variables.push(Variable::new(
        output_base,
        output_base + output_count.saturating_sub(1),
        true,
    ));
    CubeLayout::new(variables)
}

fn binary_pla_problem(parsed: &pla_format::Pla) -> minimize::Pla {
    let set_size = parsed.input_count * 2 + parsed.output_count;
    minimize::Pla::new(
        binary_pla_cover(&parsed.f, parsed, set_size),
        binary_pla_cover(&parsed.d, parsed, set_size),
        binary_pla_cover(&parsed.r, parsed, set_size),
    )
}

fn binary_pla_cover(
    cover: &pla_format::Cover,
    parsed: &pla_format::Pla,
    set_size: usize,
) -> minimize::Cover {
    minimize::Cover::from_cubes_with_output_part_size(
        set_size,
        parsed.output_count,
        cover.cubes.iter().map(|cube| binary_pla_cube(cube, parsed)),
    )
}

fn binary_pla_cube(cube: &pla_format::Cube, parsed: &pla_format::Pla) -> minimize::Cube {
    let mut columns = BTreeSet::new();
    for (index, part) in cube.inputs.iter().enumerate() {
        match part {
            pla_format::InputPart::Zero => {
                columns.insert(index * 2);
            }
            pla_format::InputPart::One => {
                columns.insert(index * 2 + 1);
            }
            pla_format::InputPart::Dash => {
                columns.insert(index * 2);
                columns.insert(index * 2 + 1);
            }
            pla_format::InputPart::Empty => {}
        }
    }

    let output_base = parsed.input_count * 2;
    for (index, selected) in cube.outputs.iter().enumerate() {
        if *selected {
            columns.insert(output_base + index);
        }
    }

    minimize::Cube::from_columns(columns)
}

fn binary_pla_result(original: &pla_format::Pla, cover: &minimize::Cover) -> pla_format::Pla {
    pla_format::Pla {
        input_count: original.input_count,
        output_count: original.output_count,
        pla_type: pla_format::F_TYPE,
        input_labels: original.input_labels.clone(),
        output_labels: original.output_labels.clone(),
        phase: original.phase.clone(),
        f: pla_format::Cover {
            cubes: cover
                .cubes()
                .iter()
                .map(|cube| binary_pla_result_cube(original, cube))
                .collect(),
        },
        d: pla_format::Cover::default(),
        r: pla_format::Cover::default(),
    }
}

fn original_on_set_result(original: &pla_format::Pla) -> pla_format::Pla {
    pla_format::Pla {
        input_count: original.input_count,
        output_count: original.output_count,
        pla_type: pla_format::F_TYPE,
        input_labels: original.input_labels.clone(),
        output_labels: original.output_labels.clone(),
        phase: original.phase.clone(),
        f: original.f.clone(),
        d: pla_format::Cover::default(),
        r: pla_format::Cover::default(),
    }
}

fn binary_pla_result_cube(original: &pla_format::Pla, cube: &minimize::Cube) -> pla_format::Cube {
    let inputs = (0..original.input_count)
        .map(|index| {
            let has_zero = cube.columns().any(|column| column == index * 2);
            let has_one = cube.columns().any(|column| column == index * 2 + 1);
            match (has_zero, has_one) {
                (true, true) => pla_format::InputPart::Dash,
                (true, false) => pla_format::InputPart::Zero,
                (false, true) => pla_format::InputPart::One,
                (false, false) => pla_format::InputPart::Empty,
            }
        })
        .collect();
    let output_base = original.input_count * 2;
    let outputs = (0..original.output_count)
        .map(|index| cube.columns().any(|column| column == output_base + index))
        .collect();

    pla_format::Cube { inputs, outputs }
}

fn complete_binary_off_set(mut pla: pla_format::Pla) -> pla_format::Pla {
    if pla.input_count >= usize::BITS as usize {
        return pla;
    }

    for input in 0..(1usize << pla.input_count) {
        let inputs = (0..pla.input_count)
            .map(|index| {
                if (input & (1 << (pla.input_count - index - 1))) == 0 {
                    pla_format::InputPart::Zero
                } else {
                    pla_format::InputPart::One
                }
            })
            .collect::<Vec<_>>();

        for output in 0..pla.output_count {
            if binary_pla_cover_has_point(&pla.f, &inputs, output)
                || binary_pla_cover_has_point(&pla.d, &inputs, output)
                || binary_pla_cover_has_point(&pla.r, &inputs, output)
            {
                continue;
            }

            let mut outputs = vec![false; pla.output_count];
            outputs[output] = true;
            pla.r.cubes.push(pla_format::Cube {
                inputs: inputs.clone(),
                outputs,
            });
        }
    }

    pla.pla_type |= pla_format::R_TYPE;
    pla
}

fn binary_pla_cover_has_point(
    cover: &pla_format::Cover,
    inputs: &[pla_format::InputPart],
    output: usize,
) -> bool {
    cover.cubes.iter().any(|cube| {
        cube.outputs.get(output).copied().unwrap_or(false)
            && cube
                .inputs
                .iter()
                .zip(inputs)
                .all(|(part, point)| *part == pla_format::InputPart::Dash || part == point)
    })
}

#[cfg(test)]
mod interop_tests {
    use super::*;

    #[test]
    fn minimize_pla_text_supports_all_logic_friday_modes() {
        let input = ".i 2\n.o 1\n.type fd\n00 1\n01 1\n11 -\n.e\n";

        for mode in [
            MinimizeMode::FastJoint,
            MinimizeMode::FastIndependentOutput,
            MinimizeMode::ExactJoint,
            MinimizeMode::ExactIndependentOutput,
        ] {
            let output = minimize_pla_text(input, mode).unwrap();
            let minimized = pla_format::Pla::parse(&output).unwrap();

            assert_eq!(minimized.input_count, 2);
            assert_eq!(minimized.output_count, 1);
            assert!(!minimized.f.cubes.is_empty());
        }
    }
}
