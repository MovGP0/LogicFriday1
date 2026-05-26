//! Native Rust planning model for `sis/speed/buf_replace.c`.
//!
//! The C routine repowers a mapped node by selecting a stronger equivalent
//! root gate/buffer and, when an inverted fanout cone exists, a matching
//! inverter. Direct mutation of SIS `network_t`, `node_t`, `lib_gate_t`, and
//! decomposition networks is intentionally not reproduced here. This module
//! ports the feasible timing/load selection rules into owned Rust inputs and
//! returns an explicit plan; SIS-bound application remains blocked on the
//! native node/network/delay/library ports listed below.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;
pub const NEG_LARGE: f64 = -10_000.0;
pub const V_SMALL: f64 = 0.000001;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead: &'static str,
    pub c_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.133",
        c_file: "LogicSynthesis/sis/delay/delay.c",
        reason: "delay_generate_decomposition and get_pin_delay",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.257",
        c_file: "LogicSynthesis/sis/map/library.c",
        reason: "mapped lib_gate_t delay records",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.258",
        c_file: "LogicSynthesis/sis/map/libutil.c",
        reason: "lib_gate lookup and equivalent gate versions",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.262",
        c_file: "LogicSynthesis/sis/map/maputil.c",
        reason: "map_invalid during decomposition trial",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.305",
        c_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "network_add_node, network_free, and node array construction",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.313",
        c_file: "LogicSynthesis/sis/node/fan.c",
        reason: "fanin count and fanin/fanout rewiring",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.318",
        c_file: "LogicSynthesis/sis/node/node.c",
        reason: "node_replace and node_free",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.460",
        c_file: "LogicSynthesis/sis/speed/buf_delay.c",
        reason: "sp_subtract_delay and buffer required-time helpers",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.464",
        c_file: "LogicSynthesis/sis/speed/buf_util.c",
        reason: "buffer/gate implementation mutation helpers",
    },
    PortDependency {
        bead: "LogicFriday1-8j8.2.6.471",
        c_file: "LogicSynthesis/sis/speed/sp_network.c",
        reason: "network_and_node_to_array decomposition copy plan",
    },
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub const fn large_required() -> Self {
        Self {
            rise: POS_LARGE,
            fall: POS_LARGE,
        }
    }

    pub const fn negative_large() -> Self {
        Self {
            rise: NEG_LARGE,
            fall: NEG_LARGE,
        }
    }

    pub fn min_edgewise(self, rhs: Self) -> Self {
        Self {
            rise: self.rise.min(rhs.rise),
            fall: self.fall.min(rhs.fall),
        }
    }

    pub fn subtract_drive_load(self, drive: Self, load: f64) -> Self {
        Self {
            rise: self.rise - drive.rise * load,
            fall: self.fall - drive.fall * load,
        }
    }

    pub fn difference(self, rhs: Self) -> Self {
        Self {
            rise: self.rise - rhs.rise,
            fall: self.fall - rhs.fall,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinPhase {
    NotGiven,
    Inverting,
    NonInverting,
    Neither,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayPin {
    pub block: DelayTime,
    pub drive: DelayTime,
    pub phase: PinPhase,
    pub load: f64,
    pub max_load: f64,
}

impl DelayPin {
    pub const fn new(block: DelayTime, drive: DelayTime, phase: PinPhase, load: f64) -> Self {
        Self {
            block,
            drive,
            phase,
            load,
            max_load: POS_LARGE,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootImplementation {
    Buffer { buffer_index: usize },
    Gate { gate_id: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RootChoice {
    pub implementation: RootImplementation,
    pub pin_delay: DelayPin,
}

impl RootChoice {
    pub fn buffer(buffer_index: usize, pin_delay: DelayPin) -> Self {
        Self {
            implementation: RootImplementation::Buffer { buffer_index },
            pin_delay,
        }
    }

    pub fn gate(gate_id: usize, pin_delay: DelayPin) -> Self {
        Self {
            implementation: RootImplementation::Gate { gate_id },
            pin_delay,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InverterChoice {
    pub buffer_index: usize,
    pub input_load: f64,
    pub block: DelayTime,
    pub drive: DelayTime,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InverterState {
    pub current_buffer_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplaceCellStrengthInput {
    pub root_implementation: RootImplementation,
    pub critical_fanin_index: usize,
    pub current_required_time: DelayTime,
    pub previous_drive: DelayTime,
    pub original_input_load: f64,
    pub max_input_load: f64,
    pub critical_positive_required: DelayTime,
    pub critical_negative_required: DelayTime,
    pub total_positive_capacitance: f64,
    pub total_negative_capacitance: f64,
    pub auto_route: f64,
    pub do_decomposition: bool,
    pub root_fanin_count: usize,
    pub inverter: Option<InverterState>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReplacementAction {
    Keep(RootImplementation),
    Replace {
        from: RootImplementation,
        to: RootImplementation,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InverterAction {
    Keep { buffer_index: usize },
    Replace { from: Option<usize>, to: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellStrengthReplacementPlan {
    pub root_choice_index: usize,
    pub inverter_choice_index: Option<usize>,
    pub root_action: ReplacementAction,
    pub inverter_action: Option<InverterAction>,
    pub root_required_time: DelayTime,
    pub inverter_required_time: Option<DelayTime>,
    pub saving: DelayTime,
    pub total_root_load: f64,
    pub config_changed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecompositionBlockedPlan {
    pub inverter_choice_index: Option<usize>,
    pub projected_root_required_time: DelayTime,
    pub inverter_required_time: Option<DelayTime>,
    pub saving: DelayTime,
    pub total_root_load: f64,
    pub dependencies: &'static [PortDependency],
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReplaceCellStrengthOutcome {
    Replaced(CellStrengthReplacementPlan),
    DecompositionBlocked(DecompositionBlockedPlan),
    Unchanged { saving: DelayTime },
}

#[derive(Clone, Debug, PartialEq)]
pub enum BufReplaceError {
    MissingRootChoices,
    MissingInverterChoices,
    InvalidPhase(PinPhase),
    ImprovedWithoutConfigurationChange,
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for BufReplaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRootChoices => write!(f, "no root gate or buffer choices were supplied"),
            Self::MissingInverterChoices => {
                write!(
                    f,
                    "an inverter node is present but no inverter choices were supplied"
                )
            }
            Self::InvalidPhase(phase) => write!(f, "cannot subtract delay for phase {phase:?}"),
            Self::ImprovedWithoutConfigurationChange => write!(
                f,
                "timing improved even though the selected root/inverter configuration did not change"
            ),
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

impl Error for BufReplaceError {}

pub fn required_time_improved(newer: DelayTime, older: DelayTime) -> bool {
    newer.rise - older.rise > V_SMALL && newer.fall - older.fall > V_SMALL
}

pub fn subtract_delay(
    phase: PinPhase,
    block: DelayTime,
    drive: DelayTime,
    load: f64,
    required: DelayTime,
) -> Result<DelayTime, BufReplaceError> {
    if phase == PinPhase::NotGiven {
        return Err(BufReplaceError::InvalidPhase(phase));
    }

    let delay = DelayTime {
        rise: block.rise + drive.rise * load,
        fall: block.fall + drive.fall * load,
    };
    let mut input_required = DelayTime {
        rise: f64::INFINITY,
        fall: f64::INFINITY,
    };

    if phase == PinPhase::Inverting || phase == PinPhase::Neither {
        input_required.rise = input_required.rise.min(required.fall - delay.fall);
        input_required.fall = input_required.fall.min(required.rise - delay.rise);
    }
    if phase == PinPhase::NonInverting || phase == PinPhase::Neither {
        input_required.rise = input_required.rise.min(required.rise - delay.rise);
        input_required.fall = input_required.fall.min(required.fall - delay.fall);
    }

    Ok(input_required)
}

pub fn replacement_in_sis_network_bound() -> Result<(), BufReplaceError> {
    Err(BufReplaceError::MissingSisPorts {
        operation: "sp_replace_cell_strength network mutation",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn plan_cell_strength_replacement(
    input: &ReplaceCellStrengthInput,
    root_choices: &[RootChoice],
    inverter_choices: &[InverterChoice],
    decomposition_pin_delay: Option<DelayPin>,
) -> Result<ReplaceCellStrengthOutcome, BufReplaceError> {
    if root_choices.is_empty() {
        return Err(BufReplaceError::MissingRootChoices);
    }

    let original_required = input
        .current_required_time
        .subtract_drive_load(input.previous_drive, input.original_input_load);
    let inverter_fanouts = inverter_fanout_alternatives(input, inverter_choices, true)?;
    let best = best_root_replacement(input, root_choices, &inverter_fanouts)?;

    if required_time_improved(best.required_time, original_required) {
        let chosen_root = &root_choices[best.root_choice_index];
        let inverter = selected_inverter(input, inverter_choices, best.inverter_choice_index);
        let mut config_changed = chosen_root.implementation != input.root_implementation;

        let root_action = if config_changed {
            ReplacementAction::Replace {
                from: input.root_implementation,
                to: chosen_root.implementation,
            }
        } else {
            ReplacementAction::Keep(chosen_root.implementation)
        };

        let inverter_action = inverter.map(|selected| {
            let current = input.inverter.and_then(|state| state.current_buffer_index);
            if current == Some(selected.buffer_index) {
                InverterAction::Keep {
                    buffer_index: selected.buffer_index,
                }
            } else {
                config_changed = true;
                InverterAction::Replace {
                    from: current,
                    to: selected.buffer_index,
                }
            }
        });

        if !config_changed {
            return Err(BufReplaceError::ImprovedWithoutConfigurationChange);
        }

        return Ok(ReplaceCellStrengthOutcome::Replaced(
            CellStrengthReplacementPlan {
                root_choice_index: best.root_choice_index,
                inverter_choice_index: best.inverter_choice_index,
                root_action,
                inverter_action,
                root_required_time: best.required_time,
                inverter_required_time: best.inverter_required_time,
                saving: best.required_time.difference(original_required),
                total_root_load: best.total_root_load,
                config_changed,
            },
        ));
    }

    if input.do_decomposition && root_choices.len() <= 1 && input.root_fanin_count > 2 {
        let Some(pin_delay) = decomposition_pin_delay else {
            return Err(BufReplaceError::MissingSisPorts {
                operation: "delay_generate_decomposition trial",
                dependencies: REQUIRED_PORT_BEADS,
            });
        };
        return plan_decomposition(input, pin_delay, &inverter_fanouts, original_required);
    }

    Ok(ReplaceCellStrengthOutcome::Unchanged {
        saving: DelayTime::new(0.0, 0.0),
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct InverterFanoutAlternative {
    choice_index: Option<usize>,
    input_required_time: DelayTime,
    load: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Candidate {
    root_choice_index: usize,
    inverter_choice_index: Option<usize>,
    required_time: DelayTime,
    inverter_required_time: Option<DelayTime>,
    total_root_load: f64,
}

fn inverter_fanout_alternatives(
    input: &ReplaceCellStrengthInput,
    inverter_choices: &[InverterChoice],
    require_choices: bool,
) -> Result<Vec<InverterFanoutAlternative>, BufReplaceError> {
    if input.inverter.is_none() {
        return Ok(vec![InverterFanoutAlternative {
            choice_index: None,
            input_required_time: DelayTime::large_required(),
            load: 0.0,
        }]);
    }

    if inverter_choices.is_empty() && require_choices {
        return Err(BufReplaceError::MissingInverterChoices);
    }

    inverter_choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            let required = subtract_delay(
                PinPhase::Inverting,
                choice.block,
                choice.drive,
                input.total_negative_capacitance,
                input.critical_negative_required,
            )?;
            Ok(InverterFanoutAlternative {
                choice_index: Some(index),
                input_required_time: required,
                load: choice.input_load + input.auto_route,
            })
        })
        .collect()
}

fn best_root_replacement(
    input: &ReplaceCellStrengthInput,
    root_choices: &[RootChoice],
    inverter_fanouts: &[InverterFanoutAlternative],
) -> Result<Candidate, BufReplaceError> {
    let mut best = Candidate {
        root_choice_index: usize::MAX,
        inverter_choice_index: None,
        required_time: DelayTime::negative_large(),
        inverter_required_time: None,
        total_root_load: 0.0,
    };

    for (root_index, root) in root_choices.iter().enumerate() {
        if root.pin_delay.load > input.max_input_load {
            continue;
        }

        for inverter in inverter_fanouts {
            let candidate = root_required_time(input, root.pin_delay, inverter)?;
            if candidate.rise > best.required_time.rise && candidate.fall > best.required_time.fall
            {
                best = Candidate {
                    root_choice_index: root_index,
                    inverter_choice_index: inverter.choice_index,
                    required_time: candidate,
                    inverter_required_time: inverter
                        .choice_index
                        .map(|_| inverter.input_required_time),
                    total_root_load: input.total_positive_capacitance + inverter.load,
                };
            }
        }
    }

    Ok(best)
}

fn root_required_time(
    input: &ReplaceCellStrengthInput,
    pin_delay: DelayPin,
    inverter: &InverterFanoutAlternative,
) -> Result<DelayTime, BufReplaceError> {
    let required = inverter
        .input_required_time
        .min_edgewise(input.critical_positive_required);
    let required = subtract_delay(
        pin_delay.phase,
        pin_delay.block,
        pin_delay.drive,
        input.total_positive_capacitance + inverter.load,
        required,
    )?;
    Ok(required.subtract_drive_load(input.previous_drive, pin_delay.load))
}

fn selected_inverter<'a>(
    input: &ReplaceCellStrengthInput,
    inverter_choices: &'a [InverterChoice],
    choice_index: Option<usize>,
) -> Option<&'a InverterChoice> {
    if input.inverter.is_some() {
        choice_index.and_then(|index| inverter_choices.get(index))
    } else {
        None
    }
}

fn plan_decomposition(
    input: &ReplaceCellStrengthInput,
    pin_delay: DelayPin,
    inverter_fanouts: &[InverterFanoutAlternative],
    original_required: DelayTime,
) -> Result<ReplaceCellStrengthOutcome, BufReplaceError> {
    let mut best = Candidate {
        root_choice_index: 0,
        inverter_choice_index: None,
        required_time: DelayTime::negative_large(),
        inverter_required_time: None,
        total_root_load: 0.0,
    };

    for inverter in inverter_fanouts {
        let candidate = root_required_time(input, pin_delay, inverter)?;
        if candidate.rise > best.required_time.rise && candidate.fall > best.required_time.fall {
            best = Candidate {
                root_choice_index: 0,
                inverter_choice_index: inverter.choice_index,
                required_time: candidate,
                inverter_required_time: inverter.choice_index.map(|_| inverter.input_required_time),
                total_root_load: input.total_positive_capacitance + inverter.load,
            };
        }
    }

    if required_time_improved(best.required_time, original_required)
        && pin_delay.load < input.max_input_load
    {
        Ok(ReplaceCellStrengthOutcome::DecompositionBlocked(
            DecompositionBlockedPlan {
                inverter_choice_index: best.inverter_choice_index,
                projected_root_required_time: best.required_time,
                inverter_required_time: best.inverter_required_time,
                saving: best.required_time.difference(original_required),
                total_root_load: best.total_root_load,
                dependencies: REQUIRED_PORT_BEADS,
            },
        ))
    } else {
        Ok(ReplaceCellStrengthOutcome::Unchanged {
            saving: DelayTime::new(0.0, 0.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "actual {actual} != expected {expected}"
        );
    }

    fn input() -> ReplaceCellStrengthInput {
        ReplaceCellStrengthInput {
            root_implementation: RootImplementation::Gate { gate_id: 1 },
            critical_fanin_index: 0,
            current_required_time: DelayTime::new(10.0, 10.0),
            previous_drive: DelayTime::new(0.2, 0.3),
            original_input_load: 2.0,
            max_input_load: 4.0,
            critical_positive_required: DelayTime::new(20.0, 20.0),
            critical_negative_required: DelayTime::new(18.0, 18.0),
            total_positive_capacitance: 3.0,
            total_negative_capacitance: 2.0,
            auto_route: 0.5,
            do_decomposition: false,
            root_fanin_count: 2,
            inverter: None,
        }
    }

    fn pin(block: f64, drive: f64, phase: PinPhase, load: f64) -> DelayPin {
        DelayPin::new(
            DelayTime::new(block, block),
            DelayTime::new(drive, drive),
            phase,
            load,
        )
    }

    #[test]
    fn subtract_delay_applies_phase_rules() {
        let required = DelayTime::new(10.0, 20.0);
        let block = DelayTime::new(1.0, 2.0);
        let drive = DelayTime::new(0.5, 1.0);

        assert_eq!(
            subtract_delay(PinPhase::NonInverting, block, drive, 4.0, required).unwrap(),
            DelayTime::new(7.0, 14.0)
        );
        assert_eq!(
            subtract_delay(PinPhase::Inverting, block, drive, 4.0, required).unwrap(),
            DelayTime::new(14.0, 7.0)
        );
        assert_eq!(
            subtract_delay(PinPhase::NotGiven, block, drive, 4.0, required),
            Err(BufReplaceError::InvalidPhase(PinPhase::NotGiven))
        );
    }

    #[test]
    fn picks_stronger_root_gate_and_reports_saving() {
        let choices = vec![
            RootChoice::gate(1, pin(1.0, 1.0, PinPhase::NonInverting, 1.0)),
            RootChoice::gate(2, pin(1.0, 0.5, PinPhase::NonInverting, 1.5)),
        ];

        let outcome = plan_cell_strength_replacement(&input(), &choices, &[], None).unwrap();
        let ReplaceCellStrengthOutcome::Replaced(plan) = outcome else {
            panic!("expected replacement");
        };

        assert_eq!(plan.root_choice_index, 1);
        assert_eq!(plan.inverter_choice_index, None);
        assert_eq!(
            plan.root_action,
            ReplacementAction::Replace {
                from: RootImplementation::Gate { gate_id: 1 },
                to: RootImplementation::Gate { gate_id: 2 },
            }
        );
        assert_close(plan.root_required_time.rise, 17.2);
        assert_close(plan.root_required_time.fall, 17.05);
        assert_close(plan.saving.rise, 7.6);
        assert_close(plan.saving.fall, 7.65);
    }

    #[test]
    fn combines_root_and_inverter_choices_when_inverted_fanout_exists() {
        let mut with_inverter = input();
        with_inverter.inverter = Some(InverterState {
            current_buffer_index: Some(4),
        });
        with_inverter.root_implementation = RootImplementation::Buffer { buffer_index: 0 };
        let roots = vec![RootChoice::buffer(
            2,
            DelayPin::new(
                DelayTime::new(1.0, 1.2),
                DelayTime::new(0.5, 0.6),
                PinPhase::NonInverting,
                1.0,
            ),
        )];
        let inverters = vec![
            InverterChoice {
                buffer_index: 4,
                input_load: 4.0,
                block: DelayTime::new(2.0, 2.0),
                drive: DelayTime::new(2.0, 2.0),
            },
            InverterChoice {
                buffer_index: 6,
                input_load: 0.5,
                block: DelayTime::new(1.0, 1.0),
                drive: DelayTime::new(0.2, 0.2),
            },
        ];

        let outcome =
            plan_cell_strength_replacement(&with_inverter, &roots, &inverters, None).unwrap();
        let ReplaceCellStrengthOutcome::Replaced(plan) = outcome else {
            panic!("expected replacement");
        };

        assert_eq!(plan.inverter_choice_index, Some(1));
        assert_eq!(
            plan.inverter_action,
            Some(InverterAction::Replace {
                from: Some(4),
                to: 6,
            })
        );
        assert_eq!(
            plan.inverter_required_time,
            Some(DelayTime::new(16.6, 16.6))
        );
        assert_close(plan.root_required_time.rise, 13.4);
        assert_close(plan.root_required_time.fall, 12.7);
        assert_close(plan.total_root_load, 4.0);
    }

    #[test]
    fn skips_root_choices_that_exceed_max_input_load() {
        let choices = vec![RootChoice::gate(
            2,
            pin(0.1, 0.1, PinPhase::NonInverting, 6.0),
        )];

        assert_eq!(
            plan_cell_strength_replacement(&input(), &choices, &[], None).unwrap(),
            ReplaceCellStrengthOutcome::Unchanged {
                saving: DelayTime::new(0.0, 0.0),
            }
        );
    }

    #[test]
    fn reports_error_when_inverter_node_has_no_choices() {
        let mut with_inverter = input();
        with_inverter.inverter = Some(InverterState {
            current_buffer_index: None,
        });

        assert_eq!(
            plan_cell_strength_replacement(
                &with_inverter,
                &[RootChoice::gate(
                    2,
                    pin(1.0, 0.5, PinPhase::NonInverting, 1.0)
                )],
                &[],
                None,
            ),
            Err(BufReplaceError::MissingInverterChoices)
        );
    }

    #[test]
    fn rejects_improvement_that_does_not_change_configuration() {
        let choices = vec![RootChoice::gate(
            1,
            pin(1.0, 0.5, PinPhase::NonInverting, 1.0),
        )];

        assert_eq!(
            plan_cell_strength_replacement(&input(), &choices, &[], None),
            Err(BufReplaceError::ImprovedWithoutConfigurationChange)
        );
    }

    #[test]
    fn decomposition_path_is_explicitly_blocked_without_native_sis_ports() {
        let mut decomposable = input();
        decomposable.do_decomposition = true;
        decomposable.root_fanin_count = 3;
        let weak = vec![RootChoice::gate(
            1,
            pin(5.0, 4.0, PinPhase::NonInverting, 1.0),
        )];

        assert_eq!(
            plan_cell_strength_replacement(&decomposable, &weak, &[], None),
            Err(BufReplaceError::MissingSisPorts {
                operation: "delay_generate_decomposition trial",
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
    }

    #[test]
    fn decomposition_candidate_reports_blocked_rewrite_when_it_would_save_time() {
        let mut decomposable = input();
        decomposable.do_decomposition = true;
        decomposable.root_fanin_count = 3;
        let weak = vec![RootChoice::gate(
            1,
            pin(5.0, 4.0, PinPhase::NonInverting, 1.0),
        )];
        let decomp_pin = pin(1.0, 0.2, PinPhase::NonInverting, 1.0);

        let outcome =
            plan_cell_strength_replacement(&decomposable, &weak, &[], Some(decomp_pin)).unwrap();
        let ReplaceCellStrengthOutcome::DecompositionBlocked(plan) = outcome else {
            panic!("expected decomposition block");
        };

        assert_eq!(plan.inverter_choice_index, None);
        assert_close(plan.projected_root_required_time.rise, 18.2);
        assert_close(plan.saving.rise, 8.6);
        assert_eq!(plan.dependencies, REQUIRED_PORT_BEADS);
    }

    #[test]
    fn sis_bound_entry_point_reports_dependencies() {
        assert_eq!(
            replacement_in_sis_network_bound(),
            Err(BufReplaceError::MissingSisPorts {
                operation: "sp_replace_cell_strength network mutation",
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
    }
}
