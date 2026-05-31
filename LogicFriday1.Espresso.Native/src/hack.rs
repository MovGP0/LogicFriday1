//! Source-aligned helpers for Espresso `hack.c`.
//!
//! The C file contains compatibility transformations for PLAs with symbolic
//! variables and DONT_CARE input labels. The native binary PLA model represents
//! unsupported symbolic directives at parse time, and this module ports the
//! DONT_CARE-label detection and separation behavior used by binary inputs.

use crate::pla::{Cover, Cube, InputPart, Pla};

pub fn find_dont_care_input(pla: &Pla) -> Option<usize> {
    pla.input_labels.iter().position(|label| {
        let normalized = label.to_ascii_lowercase().replace('_', "");
        normalized == "dontcare"
    })
}

pub fn map_dcset(pla: &Pla) -> Pla {
    let Some(dc_input) = find_dont_care_input(pla) else {
        return pla.clone();
    };

    let mut mapped = Pla {
        input_count: pla.input_count - 1,
        output_count: pla.output_count,
        pla_type: pla.pla_type,
        input_labels: pla
            .input_labels
            .iter()
            .enumerate()
            .filter_map(|(index, label)| (index != dc_input).then_some(label.clone()))
            .collect(),
        output_labels: pla.output_labels.clone(),
        phase: pla.phase.clone(),
        f: Cover::default(),
        d: Cover::default(),
        r: Cover::default(),
    };

    split_cover_by_dc_input(&pla.f, dc_input, &mut mapped.f, &mut mapped.d);
    split_cover_by_dc_input(&pla.d, dc_input, &mut mapped.d, &mut Cover::default());
    split_cover_by_dc_input(&pla.r, dc_input, &mut mapped.r, &mut Cover::default());
    mapped
}

pub fn symbolic_feature_name(directive: &str) -> Option<&'static str> {
    match directive {
        ".symbolic" => Some("symbolic input expansion"),
        ".symbolic-output" => Some("symbolic output expansion"),
        ".kiss" => Some("KISS state-table expansion"),
        ".pair" => Some("declared binary variable pairing"),
        _ => None,
    }
}

fn split_cover_by_dc_input(
    source: &Cover,
    dc_input: usize,
    normal: &mut Cover,
    dont_care: &mut Cover,
) {
    for cube in &source.cubes {
        let mut mapped_inputs = cube.inputs.clone();
        let dc_part = mapped_inputs.remove(dc_input);
        let mapped_cube = Cube {
            inputs: mapped_inputs,
            outputs: cube.outputs.clone(),
        };
        if matches!(dc_part, InputPart::One) {
            dont_care.cubes.push(mapped_cube);
        } else {
            normal.cubes.push(mapped_cube);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dont_care_label_variants_like_hack_c() {
        let pla = Pla::parse(".i 2\n.o 1\n.ilb a DONT_CARE\n.ob y\n00 1\n.e\n").unwrap();

        assert_eq!(find_dont_care_input(&pla), Some(1));
    }

    #[test]
    fn map_dcset_removes_dont_care_input_and_moves_one_rows_to_dc_cover() {
        let pla = Pla::parse(".i 2\n.o 1\n.ilb a dontcare\n.ob y\n00 1\n01 1\n10 1\n.e\n").unwrap();

        let mapped = map_dcset(&pla);

        assert_eq!(mapped.input_count, 1);
        assert_eq!(mapped.input_labels, ["a"]);
        assert_eq!(mapped.f.cubes.len(), 2);
        assert_eq!(mapped.d.cubes.len(), 1);
        assert_eq!(mapped.d.cubes[0].format("01"), "0 1");
    }

    #[test]
    fn symbolic_directives_are_named_for_native_parse_errors() {
        assert_eq!(
            symbolic_feature_name(".symbolic-output"),
            Some("symbolic output expansion")
        );
        assert_eq!(symbolic_feature_name(".type"), None);
    }
}
