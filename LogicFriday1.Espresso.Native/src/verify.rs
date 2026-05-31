//! Verification routines ported from Espresso's `verify.c`.
//!
//! The C file checks that a minimized ON-set is contained by the original
//! ON-set plus DC-set, that the original ON-set remains covered by the
//! minimized ON-set plus DC-set, and that PLA ON/DC/OFF covers form a complete
//! partition. This module performs the same checks over the native Rust PLA
//! representation by expanding cubes to minterm/output points.

use std::collections::HashSet;

use crate::pla::{Cover, Cube, Pla};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VerifyReport {
    pub minimized_not_covered_by_original: Vec<(usize, usize)>,
    pub original_not_covered_by_minimized: Vec<(usize, usize)>,
}

impl VerifyReport {
    pub fn is_equivalent(&self) -> bool {
        self.minimized_not_covered_by_original.is_empty()
            && self.original_not_covered_by_minimized.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConsistencyReport {
    pub on_dc_overlap: Vec<(usize, usize)>,
    pub on_off_overlap: Vec<(usize, usize)>,
    pub dc_off_overlap: Vec<(usize, usize)>,
    pub unspecified: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaVerifyError {
    MissingLabels,
    MissingInputLabel(String),
    MissingOutputLabel(String),
    SizeMismatch {
        left_inputs: usize,
        left_outputs: usize,
        right_inputs: usize,
        right_outputs: usize,
    },
}

impl ConsistencyReport {
    pub fn is_consistent(&self) -> bool {
        self.on_dc_overlap.is_empty()
            && self.on_off_overlap.is_empty()
            && self.dc_off_overlap.is_empty()
            && self.unspecified.is_empty()
    }
}

impl Pla {
    pub fn verify_minimized(&self, minimized: &Pla) -> VerifyReport {
        verify_covers(
            &minimized.f,
            &self.f,
            &self.d,
            self.input_count,
            self.output_count,
        )
    }

    pub fn check_consistency(&self) -> ConsistencyReport {
        let universe = universe_points(self.input_count, self.output_count);
        let f = covered_points(&self.f, self.input_count, self.output_count);
        let d = covered_points(&self.d, self.input_count, self.output_count);
        let r = covered_points(&self.r, self.input_count, self.output_count);

        ConsistencyReport {
            on_dc_overlap: intersection_points(&f, &d),
            on_off_overlap: intersection_points(&f, &r),
            dc_off_overlap: intersection_points(&d, &r),
            unspecified: universe
                .difference(
                    &f.union(&d)
                        .copied()
                        .collect::<HashSet<_>>()
                        .union(&r)
                        .copied()
                        .collect(),
                )
                .copied()
                .collect(),
        }
    }

    pub fn verify_equivalent_to(&self, reference: &Pla) -> Result<VerifyReport, PlaVerifyError> {
        let permuted = self.permute_to_match(reference)?;
        if permuted.input_count != reference.input_count
            || permuted.output_count != reference.output_count
        {
            return Err(PlaVerifyError::SizeMismatch {
                left_inputs: permuted.input_count,
                left_outputs: permuted.output_count,
                right_inputs: reference.input_count,
                right_outputs: reference.output_count,
            });
        }

        Ok(verify_covers(
            &reference.f,
            &permuted.f,
            &permuted.d,
            reference.input_count,
            reference.output_count,
        ))
    }

    pub fn permute_to_match(&self, reference: &Pla) -> Result<Pla, PlaVerifyError> {
        if self.input_labels.is_empty()
            || reference.input_labels.is_empty()
            || self.output_labels.is_empty()
            || reference.output_labels.is_empty()
        {
            return Err(PlaVerifyError::MissingLabels);
        }

        let input_permutation = label_permutation(&self.input_labels, &reference.input_labels)
            .map_err(PlaVerifyError::MissingInputLabel)?;
        let output_permutation = label_permutation(&self.output_labels, &reference.output_labels)
            .map_err(PlaVerifyError::MissingOutputLabel)?;

        Ok(Pla {
            input_count: reference.input_count,
            output_count: reference.output_count,
            pla_type: self.pla_type,
            input_labels: reference.input_labels.clone(),
            output_labels: reference.output_labels.clone(),
            phase: self.phase.clone().map(|phase| {
                output_permutation
                    .iter()
                    .filter_map(|source| phase.get(*source).copied())
                    .collect()
            }),
            f: permute_cover(&self.f, &input_permutation, &output_permutation),
            d: permute_cover(&self.d, &input_permutation, &output_permutation),
            r: permute_cover(&self.r, &input_permutation, &output_permutation),
        })
    }
}

pub fn verify_covers(
    minimized: &Cover,
    original: &Cover,
    original_dc: &Cover,
    input_count: usize,
    output_count: usize,
) -> VerifyReport {
    let minimized_points = covered_points(minimized, input_count, output_count);
    let original_points = covered_points(original, input_count, output_count);
    let dc_points = covered_points(original_dc, input_count, output_count);
    let original_or_dc: HashSet<_> = original_points.union(&dc_points).copied().collect();
    let minimized_or_dc: HashSet<_> = minimized_points.union(&dc_points).copied().collect();

    VerifyReport {
        minimized_not_covered_by_original: minimized_points
            .difference(&original_or_dc)
            .copied()
            .collect(),
        original_not_covered_by_minimized: original_points
            .difference(&minimized_or_dc)
            .copied()
            .collect(),
    }
}

fn covered_points(
    cover: &Cover,
    input_count: usize,
    output_count: usize,
) -> HashSet<(usize, usize)> {
    let mut points = HashSet::new();
    for cube in &cover.cubes {
        for minterm in cube.covered_minterms(input_count) {
            for output in 0..output_count {
                if cube.outputs.get(output).copied().unwrap_or(false) {
                    points.insert((minterm, output));
                }
            }
        }
    }
    points
}

fn universe_points(input_count: usize, output_count: usize) -> HashSet<(usize, usize)> {
    let mut points = HashSet::new();
    for minterm in 0..(1usize << input_count) {
        for output in 0..output_count {
            points.insert((minterm, output));
        }
    }
    points
}

fn intersection_points(
    left: &HashSet<(usize, usize)>,
    right: &HashSet<(usize, usize)>,
) -> Vec<(usize, usize)> {
    left.intersection(right).copied().collect()
}

fn label_permutation(
    source_labels: &[String],
    target_labels: &[String],
) -> Result<Vec<usize>, String> {
    target_labels
        .iter()
        .map(|target| {
            source_labels
                .iter()
                .position(|source| source == target)
                .ok_or_else(|| target.clone())
        })
        .collect()
}

fn permute_cover(
    cover: &Cover,
    input_permutation: &[usize],
    output_permutation: &[usize],
) -> Cover {
    Cover {
        cubes: cover
            .cubes
            .iter()
            .map(|cube| permute_cube(cube, input_permutation, output_permutation))
            .collect(),
    }
}

fn permute_cube(cube: &Cube, input_permutation: &[usize], output_permutation: &[usize]) -> Cube {
    Cube {
        inputs: input_permutation
            .iter()
            .filter_map(|source| cube.inputs.get(*source).copied())
            .collect(),
        outputs: output_permutation
            .iter()
            .filter_map(|source| cube.outputs.get(*source).copied())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use crate::pla::Pla;

    #[test]
    fn verify_accepts_dc_preserving_minimized_cover() {
        let original = Pla::parse(
            r#"
.i 2
.o 1
.type fd
00 1
01 -
.e
"#,
        )
        .unwrap();
        let minimized = Pla::parse(
            r#"
.i 2
.o 1
0- 1
.e
"#,
        )
        .unwrap();

        assert!(original.verify_minimized(&minimized).is_equivalent());
    }

    #[test]
    fn verify_reports_both_growth_and_lost_original_minterms() {
        let original = Pla::parse(".i 2\n.o 1\n00 1\n.e\n").unwrap();
        let minimized = Pla::parse(".i 2\n.o 1\n01 1\n.e\n").unwrap();

        let report = original.verify_minimized(&minimized);

        assert_eq!(report.minimized_not_covered_by_original, [(1, 0)]);
        assert_eq!(report.original_not_covered_by_minimized, [(0, 0)]);
    }

    #[test]
    fn check_consistency_requires_on_dc_off_partition() {
        let pla = Pla::parse(
            r#"
.i 1
.o 1
.type fdr
0 1
1 0
.e
"#,
        )
        .unwrap();

        assert!(pla.check_consistency().is_consistent());
    }

    #[test]
    fn pla_verify_permutates_named_columns_before_comparison() {
        let reference = Pla::parse(
            r#"
.i 2
.o 2
.ilb a b
.ob y z
01 10
.e
"#,
        )
        .unwrap();
        let permuted = Pla::parse(
            r#"
.i 2
.o 2
.ilb b a
.ob z y
10 01
.e
"#,
        )
        .unwrap();

        assert!(
            permuted
                .verify_equivalent_to(&reference)
                .unwrap()
                .is_equivalent()
        );
    }
}
