//! Source-aligned output phase helpers from Espresso `opo.c`.
//!
//! The full C command searches for profitable output complement phases before
//! minimization. The native representation exposes the deterministic core that
//! Logic Friday needs: decode the strategy flags, choose a phase from ON/OFF
//! cover costs, and apply that phase by swapping ON/OFF output columns.

use crate::pla::{Cover, Cube, Pla};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseStrategy {
    pub no_make_sparse: bool,
    pub repeated: bool,
    pub exact: bool,
}

impl PhaseStrategy {
    pub fn from_espresso_strategy(value: i32) -> Self {
        Self {
            no_make_sparse: value % 2 != 0,
            repeated: (value / 2) % 2 != 0,
            exact: (value / 4) % 2 != 0,
        }
    }
}

pub fn choose_phase_by_cost(pla: &Pla) -> Vec<bool> {
    (0..pla.output_count)
        .map(|output| output_literal_cost(&pla.f, output) <= output_literal_cost(&pla.r, output))
        .collect()
}

pub fn set_phase(pla: &Pla, phase: &[bool]) -> Pla {
    let mut phased = Pla {
        input_count: pla.input_count,
        output_count: pla.output_count,
        pla_type: pla.pla_type,
        input_labels: pla.input_labels.clone(),
        output_labels: pla.output_labels.clone(),
        phase: Some(phase.to_vec()),
        f: Cover::default(),
        d: pla.d.clone(),
        r: Cover::default(),
    };

    for output in 0..pla.output_count {
        let positive = phase.get(output).copied().unwrap_or(true);
        let (on_source, off_source) = if positive {
            (&pla.f, &pla.r)
        } else {
            (&pla.r, &pla.f)
        };
        append_output_column(&mut phased.f, on_source, output);
        append_output_column(&mut phased.r, off_source, output);
    }

    phased
}

pub fn phase_assignment(pla: &Pla, strategy: PhaseStrategy) -> Pla {
    let mut phase = if strategy.repeated {
        vec![true; pla.output_count]
    } else {
        choose_phase_by_cost(pla)
    };

    if strategy.repeated {
        for output in 0..pla.output_count {
            phase[output] =
                output_literal_cost(&pla.f, output) <= output_literal_cost(&pla.r, output);
        }
    }

    set_phase(pla, &phase)
}

fn output_literal_cost(cover: &Cover, output: usize) -> usize {
    cover
        .cubes
        .iter()
        .filter(|cube| cube.outputs.get(output).copied().unwrap_or(false))
        .map(|cube| {
            cube.inputs
                .iter()
                .filter(|part| !matches!(part, crate::pla::InputPart::Dash))
                .count()
                + 1
        })
        .sum()
}

fn append_output_column(target: &mut Cover, source: &Cover, output: usize) {
    for cube in &source.cubes {
        if !cube.outputs.get(output).copied().unwrap_or(false) {
            continue;
        }
        let mut outputs = vec![false; cube.outputs.len()];
        outputs[output] = true;
        target.cubes.push(Cube {
            inputs: cube.inputs.clone(),
            outputs,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_bits_match_opo_c_flags() {
        assert_eq!(
            PhaseStrategy::from_espresso_strategy(7),
            PhaseStrategy {
                no_make_sparse: true,
                repeated: true,
                exact: true,
            }
        );
        assert_eq!(
            PhaseStrategy::from_espresso_strategy(2),
            PhaseStrategy {
                no_make_sparse: false,
                repeated: true,
                exact: false,
            }
        );
    }

    #[test]
    fn set_phase_swaps_negative_outputs_between_on_and_off() {
        let pla = Pla::parse(".i 1\n.o 1\n.type fr\n0 1\n1 0\n.e\n").unwrap();
        let phased = set_phase(&pla, &[false]);

        assert_eq!(phased.f.cubes[0].format("01"), "1 1");
        assert_eq!(phased.r.cubes[0].format("01"), "0 1");
        assert_eq!(phased.phase, Some(vec![false]));
    }

    #[test]
    fn phase_assignment_prefers_lower_on_cover_cost() {
        let pla = Pla::parse(".i 1\n.o 1\n.type fr\n- 1\n0 0\n1 0\n.e\n").unwrap();
        let phased = phase_assignment(&pla, PhaseStrategy::from_espresso_strategy(0));

        assert_eq!(phased.phase, Some(vec![true]));
    }
}
