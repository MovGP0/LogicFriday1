use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub const fn minus_infinity() -> Self {
        Self {
            rise: f64::NEG_INFINITY,
            fall: f64::NEG_INFINITY,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinPhase {
    Inverting,
    NonInverting,
    Neither,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayPin {
    pub block: DelayTime,
    pub drive: DelayTime,
    pub phase: PinPhase,
}

impl DelayPin {
    pub const fn new(block: DelayTime, drive: DelayTime, phase: PinPhase) -> Self {
        Self {
            block,
            drive,
            phase,
        }
    }

    pub fn delay_for_load(self, load: f64) -> DelayTime {
        DelayTime {
            rise: self.block.rise + self.drive.rise * load,
            fall: self.block.fall + self.drive.fall * load,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapDelayError {
    PinCountMismatch { arrivals: usize, model: usize },
}

impl fmt::Display for MapDelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PinCountMismatch { arrivals, model } => {
                write!(
                    f,
                    "pin arrival count {arrivals} does not match delay model pin count {model}"
                )
            }
        }
    }
}

impl Error for MapDelayError {}

pub fn delay_map_simulate(
    pin_arrivals: &[DelayTime],
    model: &[DelayPin],
    load: f64,
) -> Result<DelayTime, MapDelayError> {
    if pin_arrivals.len() != model.len() {
        return Err(MapDelayError::PinCountMismatch {
            arrivals: pin_arrivals.len(),
            model: model.len(),
        });
    }

    let mut arrival = DelayTime::minus_infinity();
    for (pin_arrival, pin_delay) in pin_arrivals.iter().zip(model.iter()).rev() {
        let delay = pin_delay.delay_for_load(load);
        merge_pin_arrival(&mut arrival, *pin_arrival, delay, pin_delay.phase);
    }

    Ok(arrival)
}

fn merge_pin_arrival(
    arrival: &mut DelayTime,
    pin_arrival: DelayTime,
    delay: DelayTime,
    phase: PinPhase,
) {
    match phase {
        PinPhase::Inverting => {
            arrival.rise = arrival.rise.max(pin_arrival.fall + delay.rise);
            arrival.fall = arrival.fall.max(pin_arrival.rise + delay.fall);
        }
        PinPhase::NonInverting => {
            arrival.rise = arrival.rise.max(pin_arrival.rise + delay.rise);
            arrival.fall = arrival.fall.max(pin_arrival.fall + delay.fall);
        }
        PinPhase::Neither => {
            arrival.rise = arrival.rise.max(pin_arrival.fall + delay.rise);
            arrival.fall = arrival.fall.max(pin_arrival.rise + delay.fall);
            arrival.rise = arrival.rise.max(pin_arrival.rise + delay.rise);
            arrival.fall = arrival.fall.max(pin_arrival.fall + delay.fall);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(
        block_rise: f64,
        block_fall: f64,
        drive_rise: f64,
        drive_fall: f64,
        phase: PinPhase,
    ) -> DelayPin {
        DelayPin::new(
            DelayTime::new(block_rise, block_fall),
            DelayTime::new(drive_rise, drive_fall),
            phase,
        )
    }

    #[test]
    fn empty_model_returns_negative_infinity_arrival() {
        assert_eq!(
            delay_map_simulate(&[], &[], 3.0).unwrap(),
            DelayTime::minus_infinity()
        );
    }

    #[test]
    fn non_inverting_pin_keeps_rise_and_fall_edges() {
        let arrivals = [DelayTime::new(4.0, 7.0)];
        let model = [pin(1.0, 2.0, 0.5, 0.25, PinPhase::NonInverting)];

        assert_eq!(
            delay_map_simulate(&arrivals, &model, 6.0).unwrap(),
            DelayTime::new(8.0, 10.5)
        );
    }

    #[test]
    fn inverting_pin_swaps_arrival_edges() {
        let arrivals = [DelayTime::new(4.0, 7.0)];
        let model = [pin(1.0, 2.0, 0.5, 0.25, PinPhase::Inverting)];

        assert_eq!(
            delay_map_simulate(&arrivals, &model, 6.0).unwrap(),
            DelayTime::new(11.0, 7.5)
        );
    }

    #[test]
    fn neither_phase_uses_worst_input_edge_for_each_output_edge() {
        let arrivals = [DelayTime::new(4.0, 9.0)];
        let model = [pin(1.0, 2.0, 0.5, 0.25, PinPhase::Neither)];

        assert_eq!(
            delay_map_simulate(&arrivals, &model, 6.0).unwrap(),
            DelayTime::new(13.0, 12.5)
        );
    }

    #[test]
    fn multiple_pins_return_worst_arrival_per_edge() {
        let arrivals = [
            DelayTime::new(1.0, 20.0),
            DelayTime::new(10.0, 2.0),
            DelayTime::new(3.0, 4.0),
        ];
        let model = [
            pin(1.0, 2.0, 1.0, 1.0, PinPhase::Inverting),
            pin(5.0, 7.0, 0.0, 0.0, PinPhase::NonInverting),
            pin(2.0, 3.0, 0.5, 0.25, PinPhase::Neither),
        ];

        assert_eq!(
            delay_map_simulate(&arrivals, &model, 2.0).unwrap(),
            DelayTime::new(23.0, 9.0)
        );
    }

    #[test]
    fn mismatched_pin_counts_are_reported() {
        let arrivals = [DelayTime::new(1.0, 2.0)];

        assert_eq!(
            delay_map_simulate(&arrivals, &[], 0.0),
            Err(MapDelayError::PinCountMismatch {
                arrivals: 1,
                model: 0,
            })
        );
    }
}
