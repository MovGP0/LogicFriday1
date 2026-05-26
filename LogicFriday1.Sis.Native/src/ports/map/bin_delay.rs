//! Native Rust delay-bucket helpers for `sis/map/bin_delay.c`.
//!
//! The original SIS implementation performs full tree matching against a live
//! `network_t` and stores selected gates through pointer payloads in PWL
//! segments. This port keeps the self-contained delay-bucket behavior as owned
//! Rust state: primary-input, constant, wire, and gate buckets; gate PWL
//! construction from genlib pin timing; node PWL updates through min/max or
//! min/sum cost functions; and active bucket delay selection. Full SIS network
//! traversal, latch mutation, and fanout-estimator callbacks remain higher
//! level integration work and are surfaced as typed runtime diagnostics.

use std::error::Error;
use std::fmt;

use super::library::{GenlibGate, PinPhase};
use super::libutil::PinDelay;
use super::pwl::{PiecewiseLinear, PiecewisePoint};
use super::virtual_net::DelayTime;

pub const ZERO_DELAY: DelayTime = DelayTime {
    rise: 0.0,
    fall: 0.0,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinDelayCostFunction {
    MaxRiseFall,
    SumRiseFall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayBucketType {
    Gate,
    Wire,
    PrimaryInput,
    Constant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelayBucketId(usize);

impl DelayBucketId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayPwl<T> {
    pub rise: PiecewiseLinear<T>,
    pub fall: PiecewiseLinear<T>,
}

impl<T> DelayPwl<T> {
    pub fn new(rise: PiecewiseLinear<T>, fall: PiecewiseLinear<T>) -> Self {
        Self { rise, fall }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayPinInfo<S> {
    pub input: S,
    pub arrival: DelayTime,
}

impl<S> DelayPinInfo<S> {
    pub fn new(input: S, arrival: DelayTime) -> Self {
        Self { input, arrival }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayBucket<S> {
    pub bucket_type: DelayBucketType,
    pub gate_name: Option<String>,
    pub gate_area: f64,
    pub save_binding: Vec<S>,
    pub pin_info: Vec<DelayPinInfo<S>>,
    pub pwl: DelayPwl<DelayBucketId>,
}

impl<S> DelayBucket<S> {
    pub fn input_count(&self) -> usize {
        self.save_binding.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaBucket<S> {
    pub area: f64,
    pub gate_name: Option<String>,
    pub save_binding: Vec<S>,
}

impl<S> Default for AreaBucket<S> {
    fn default() -> Self {
        Self {
            area: f64::INFINITY,
            gate_name: None,
            save_binding: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinDelayStore<S> {
    cost_function: BinDelayCostFunction,
    buckets: Vec<DelayBucket<S>>,
}

impl<S> BinDelayStore<S>
where
    S: Clone + PartialEq,
{
    pub fn new(cost_function: BinDelayCostFunction) -> Self {
        Self {
            cost_function,
            buckets: Vec::new(),
        }
    }

    pub fn buckets(&self) -> &[DelayBucket<S>] {
        &self.buckets
    }

    pub fn bucket(&self, id: DelayBucketId) -> Result<&DelayBucket<S>, BinDelayError> {
        self.buckets
            .get(id.index())
            .ok_or(BinDelayError::BucketIndexOutOfRange {
                index: id.index(),
                len: self.buckets.len(),
            })
    }

    pub fn add_primary_input_bucket(
        &mut self,
        arrival: DelayTime,
        drive: DelayTime,
    ) -> Result<DelayBucketId, BinDelayError> {
        validate_delay_time(arrival)?;
        validate_non_negative_delay_time(drive)?;

        let rise = PiecewiseLinear::linear_max(&[PiecewisePoint::new(
            0.0,
            arrival.rise,
            drive.rise,
            None,
        )]);
        let fall = PiecewiseLinear::linear_max(&[PiecewisePoint::new(
            0.0,
            arrival.fall,
            drive.fall,
            None,
        )]);

        self.push_bucket(DelayBucket {
            bucket_type: DelayBucketType::PrimaryInput,
            gate_name: None,
            gate_area: 0.0,
            save_binding: Vec::new(),
            pin_info: Vec::new(),
            pwl: DelayPwl::new(rise, fall),
        })
    }

    pub fn add_constant_bucket(
        &mut self,
        gate: &GenlibGate,
    ) -> Result<DelayBucketId, BinDelayError> {
        if !gate.area.is_finite() || gate.area < 0.0 {
            return Err(BinDelayError::InvalidGateArea {
                gate: gate.name.clone(),
                area: gate.area,
            });
        }

        let rise = gen_constant_pwl();
        let fall = gen_constant_pwl();
        self.push_bucket(DelayBucket {
            bucket_type: DelayBucketType::Constant,
            gate_name: Some(gate.name.clone()),
            gate_area: gate.area,
            save_binding: Vec::new(),
            pin_info: Vec::new(),
            pwl: DelayPwl::new(rise, fall),
        })
    }

    pub fn add_wire_bucket(
        &mut self,
        gate: &GenlibGate,
        input: S,
        input_pwl: &PiecewiseLinear<DelayBucketId>,
        prohibit_multiple_fanout: bool,
    ) -> Result<DelayBucketId, BinDelayError> {
        if gate.pins.len() > 1 {
            return Err(BinDelayError::PinCountMismatch {
                gate: gate.name.clone(),
                expected: 1,
                actual: gate.pins.len(),
            });
        }

        let rise = if prohibit_multiple_fanout {
            gen_infinitely_slow_pwl()
        } else {
            input_pwl.clone()
        };
        let fall = rise.clone();

        self.push_bucket(DelayBucket {
            bucket_type: DelayBucketType::Wire,
            gate_name: Some(gate.name.clone()),
            gate_area: gate.area,
            save_binding: vec![input.clone()],
            pin_info: vec![DelayPinInfo::new(input, ZERO_DELAY)],
            pwl: DelayPwl::new(rise, fall),
        })
    }

    pub fn add_gate_bucket(
        &mut self,
        gate: &GenlibGate,
        pin_info: &[DelayPinInfo<S>],
    ) -> Result<DelayBucketId, BinDelayError> {
        let pwl = compute_gate_pwl(gate, pin_info)?;
        self.push_bucket(DelayBucket {
            bucket_type: DelayBucketType::Gate,
            gate_name: Some(gate.name.clone()),
            gate_area: gate.area,
            save_binding: pin_info.iter().map(|pin| pin.input.clone()).collect(),
            pin_info: pin_info.to_vec(),
            pwl,
        })
    }

    pub fn update_node_pwl(
        &self,
        node_pwl: &PiecewiseLinear<DelayBucketId>,
        bucket_id: DelayBucketId,
    ) -> Result<PiecewiseLinear<DelayBucketId>, BinDelayError> {
        Ok(node_pwl.min(&self.bucket_pwl_max(bucket_id)?))
    }

    pub fn bucket_pwl_max(
        &self,
        bucket_id: DelayBucketId,
    ) -> Result<PiecewiseLinear<DelayBucketId>, BinDelayError> {
        let bucket = self.bucket(bucket_id)?;
        let mut result = if bucket.bucket_type == DelayBucketType::Wire {
            bucket.pwl.rise.clone()
        } else if self.cost_function == BinDelayCostFunction::MaxRiseFall {
            bucket.pwl.rise.max(&bucket.pwl.fall)
        } else {
            bucket.pwl.rise.sum(&bucket.pwl.fall)
        };
        result.set_data(Some(bucket_id));

        Ok(result)
    }

    pub fn compute_pwl_delay(
        &self,
        node_pwl: &PiecewiseLinear<DelayBucketId>,
        load: f64,
    ) -> Result<DelayTime, BinDelayError> {
        let active = self.select_active_pwl_delay(node_pwl, load)?;
        compute_delay_pwl_delay(&active, load)
    }

    pub fn select_active_pwl_delay(
        &self,
        node_pwl: &PiecewiseLinear<DelayBucketId>,
        load: f64,
    ) -> Result<DelayPwl<DelayBucketId>, BinDelayError> {
        validate_load(load)?;
        let bucket_id = *node_pwl
            .select(load)
            .ok_or(BinDelayError::EmptyPwl { phase: "node" })?;
        let bucket = self.bucket(bucket_id)?;

        match bucket.bucket_type {
            DelayBucketType::Gate | DelayBucketType::PrimaryInput | DelayBucketType::Constant => {
                Ok(bucket.pwl.clone())
            }
            DelayBucketType::Wire => self.select_active_pwl_delay(&bucket.pwl.rise, load),
        }
    }

    pub fn preserve_best_area(
        &self,
        bucket: &mut AreaBucket<S>,
        gate: &GenlibGate,
        pin_info: &[DelayPinInfo<S>],
        input_area: impl Fn(&S) -> f64,
    ) -> Result<(), BinDelayError> {
        if !gate.area.is_finite() || gate.area < 0.0 {
            return Err(BinDelayError::InvalidGateArea {
                gate: gate.name.clone(),
                area: gate.area,
            });
        }

        let mut area = gate.area;
        for pin in pin_info {
            let pin_area = input_area(&pin.input);
            if !pin_area.is_finite() || pin_area < 0.0 {
                return Err(BinDelayError::InvalidArea { area: pin_area });
            }
            area += pin_area;
        }

        if area < bucket.area {
            bucket.area = area;
            bucket.gate_name = Some(gate.name.clone());
            bucket.save_binding = pin_info.iter().map(|pin| pin.input.clone()).collect();
        }

        Ok(())
    }

    fn push_bucket(&mut self, mut bucket: DelayBucket<S>) -> Result<DelayBucketId, BinDelayError> {
        let id = DelayBucketId(self.buckets.len());
        if bucket.bucket_type != DelayBucketType::Wire {
            bucket.pwl.rise.set_data(Some(id));
            bucket.pwl.fall.set_data(Some(id));
        }
        self.buckets.push(bucket);

        Ok(id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinDelayError {
    InvalidLoad {
        load: f64,
    },
    InvalidDelayTime {
        delay: DelayTime,
    },
    InvalidGateArea {
        gate: String,
        area: f64,
    },
    InvalidArea {
        area: f64,
    },
    PinCountMismatch {
        gate: String,
        expected: usize,
        actual: usize,
    },
    LoadExceedsPinLimit {
        gate: String,
        pin: usize,
        load: f64,
        max_load: f64,
    },
    EmptyPwl {
        phase: &'static str,
    },
    MissingPwlData {
        phase: &'static str,
    },
    BucketIndexOutOfRange {
        index: usize,
        len: usize,
    },
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for BinDelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLoad { load } => write!(f, "invalid binary delay load {load}"),
            Self::InvalidDelayTime { delay } => write!(
                f,
                "invalid binary delay time ({}, {})",
                delay.rise, delay.fall
            ),
            Self::InvalidGateArea { gate, area } => {
                write!(f, "gate '{gate}' has invalid area {area}")
            }
            Self::InvalidArea { area } => write!(f, "invalid binary delay area {area}"),
            Self::PinCountMismatch {
                gate,
                expected,
                actual,
            } => write!(f, "gate '{gate}' expected {expected} pins but got {actual}"),
            Self::LoadExceedsPinLimit {
                gate,
                pin,
                load,
                max_load,
            } => write!(
                f,
                "gate '{gate}' pin {pin} load {load} exceeds max load {max_load}"
            ),
            Self::EmptyPwl { phase } => write!(f, "{phase} binary delay PWL is empty"),
            Self::MissingPwlData { phase } => {
                write!(f, "{phase} binary delay PWL segment has no bucket data")
            }
            Self::BucketIndexOutOfRange { index, len } => {
                write!(
                    f,
                    "binary delay bucket index {index} is out of range for {len} buckets"
                )
            }
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for BinDelayError {}

pub fn bin_delay_area_tree_match_unavailable() -> Result<(), BinDelayError> {
    Err(BinDelayError::MissingSisPorts {
        operation: "bin_delay_area_tree_match area/delay interpolation mode",
    })
}

pub fn bin_delay_tree_match_unavailable() -> Result<(), BinDelayError> {
    Err(BinDelayError::MissingSisPorts {
        operation: "bin_delay_tree_match full SIS network tree matching",
    })
}

pub fn compute_gate_pwl<S>(
    gate: &GenlibGate,
    pin_info: &[DelayPinInfo<S>],
) -> Result<DelayPwl<DelayBucketId>, BinDelayError> {
    if gate.pins.len() != pin_info.len() {
        return Err(BinDelayError::PinCountMismatch {
            gate: gate.name.clone(),
            expected: gate.pins.len(),
            actual: pin_info.len(),
        });
    }

    let mut rise_points = Vec::new();
    let mut fall_points = Vec::new();
    for (pin_index, (pin, info)) in gate.pins.iter().zip(pin_info.iter()).enumerate() {
        validate_delay_time(info.arrival)?;
        let pin_delay = PinDelay::from(pin);
        validate_pin_load(&gate.name, pin_index, 0.0, pin_delay.max_load)?;
        append_pin_points(pin_delay, info.arrival, &mut rise_points, &mut fall_points);
    }

    Ok(DelayPwl::new(
        PiecewiseLinear::linear_max(&rise_points),
        PiecewiseLinear::linear_max(&fall_points),
    ))
}

pub fn compute_delay_pwl_delay(
    pwl: &DelayPwl<DelayBucketId>,
    load: f64,
) -> Result<DelayTime, BinDelayError> {
    validate_load(load)?;
    let rise = pwl
        .rise
        .eval(load)
        .ok_or(BinDelayError::EmptyPwl { phase: "rise" })?;
    let fall = pwl
        .fall
        .eval(load)
        .ok_or(BinDelayError::EmptyPwl { phase: "fall" })?;

    Ok(DelayTime::new(rise, fall))
}

pub fn gen_constant_pwl() -> PiecewiseLinear<DelayBucketId> {
    PiecewiseLinear::linear_max(&[PiecewisePoint::new(0.0, 0.0, 0.0, None)])
}

pub fn gen_infinitely_slow_pwl() -> PiecewiseLinear<DelayBucketId> {
    PiecewiseLinear::linear_max(&[PiecewisePoint::new(0.0, f64::INFINITY, f64::INFINITY, None)])
}

fn append_pin_points(
    pin_delay: PinDelay,
    arrival: DelayTime,
    rise_points: &mut Vec<PiecewisePoint<DelayBucketId>>,
    fall_points: &mut Vec<PiecewisePoint<DelayBucketId>>,
) {
    match pin_delay.phase {
        PinPhase::Inv => {
            push_point_pair(
                arrival.fall,
                arrival.rise,
                pin_delay,
                rise_points,
                fall_points,
            );
        }
        PinPhase::NonInv => {
            push_point_pair(
                arrival.rise,
                arrival.fall,
                pin_delay,
                rise_points,
                fall_points,
            );
        }
        PinPhase::Unknown => {
            push_point_pair(
                arrival.rise,
                arrival.fall,
                pin_delay,
                rise_points,
                fall_points,
            );
            push_point_pair(
                arrival.fall,
                arrival.rise,
                pin_delay,
                rise_points,
                fall_points,
            );
        }
    }
}

fn push_point_pair(
    rise_input: f64,
    fall_input: f64,
    pin_delay: PinDelay,
    rise_points: &mut Vec<PiecewisePoint<DelayBucketId>>,
    fall_points: &mut Vec<PiecewisePoint<DelayBucketId>>,
) {
    rise_points.push(PiecewisePoint::new(
        0.0,
        rise_input + pin_delay.rise_block_delay,
        pin_delay.rise_fanout_delay,
        None,
    ));
    fall_points.push(PiecewisePoint::new(
        0.0,
        fall_input + pin_delay.fall_block_delay,
        pin_delay.fall_fanout_delay,
        None,
    ));
}

fn validate_load(load: f64) -> Result<(), BinDelayError> {
    if !load.is_finite() || load < 0.0 {
        return Err(BinDelayError::InvalidLoad { load });
    }

    Ok(())
}

fn validate_delay_time(delay: DelayTime) -> Result<(), BinDelayError> {
    if !delay.rise.is_finite() || !delay.fall.is_finite() {
        return Err(BinDelayError::InvalidDelayTime { delay });
    }

    Ok(())
}

fn validate_non_negative_delay_time(delay: DelayTime) -> Result<(), BinDelayError> {
    validate_delay_time(delay)?;
    if delay.rise < 0.0 || delay.fall < 0.0 {
        return Err(BinDelayError::InvalidDelayTime { delay });
    }

    Ok(())
}

fn validate_pin_load(
    gate: &str,
    pin: usize,
    load: f64,
    max_load: f64,
) -> Result<(), BinDelayError> {
    if max_load.is_finite() && load > max_load {
        return Err(BinDelayError::LoadExceedsPinLimit {
            gate: gate.to_string(),
            pin,
            load,
            max_load,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::map::library::parse_genlib;

    fn sample_gate(name: &str) -> GenlibGate {
        parse_genlib(concat!(
            "GATE and2 2 O=a*b;\n",
            "PIN a NONINV 1 20 1 .5 2 .25\n",
            "PIN b NONINV 1 20 3 .25 4 .5\n",
            "GATE inv 1 O=!a;\n",
            "PIN a INV 2 10 5 .5 7 .25\n",
            "GATE unk 1 O=a;\n",
            "PIN a UNKNOWN 1 20 2 1 3 2\n",
            "GATE zero 0 O=CONST0;\n",
        ))
        .unwrap()
        .gate(name)
        .unwrap()
        .clone()
    }

    #[test]
    fn computes_gate_pwl_for_pin_phases() {
        let and_gate = sample_gate("and2");
        let inv_gate = sample_gate("inv");
        let unk_gate = sample_gate("unk");

        let and_pwl = compute_gate_pwl(
            &and_gate,
            &[
                DelayPinInfo::new("a", DelayTime::new(1.0, 2.0)),
                DelayPinInfo::new("b", DelayTime::new(4.0, 5.0)),
            ],
        )
        .unwrap();
        assert_eq!(
            compute_delay_pwl_delay(&and_pwl, 4.0).unwrap(),
            DelayTime::new(8.0, 11.0)
        );

        let inv_pwl = compute_gate_pwl(
            &inv_gate,
            &[DelayPinInfo::new("a", DelayTime::new(1.0, 2.0))],
        )
        .unwrap();
        assert_eq!(
            compute_delay_pwl_delay(&inv_pwl, 4.0).unwrap(),
            DelayTime::new(9.0, 9.0)
        );

        let unk_pwl = compute_gate_pwl(
            &unk_gate,
            &[DelayPinInfo::new("a", DelayTime::new(1.0, 5.0))],
        )
        .unwrap();
        assert_eq!(
            compute_delay_pwl_delay(&unk_pwl, 2.0).unwrap(),
            DelayTime::new(9.0, 12.0)
        );
    }

    #[test]
    fn updates_node_pwl_with_minimum_of_bucket_maxes() {
        let slow_gate = sample_gate("and2");
        let fast_gate = sample_gate("inv");
        let mut store = BinDelayStore::new(BinDelayCostFunction::MaxRiseFall);
        let slow = store
            .add_gate_bucket(
                &slow_gate,
                &[
                    DelayPinInfo::new("a", DelayTime::new(5.0, 5.0)),
                    DelayPinInfo::new("b", DelayTime::new(5.0, 5.0)),
                ],
            )
            .unwrap();
        let fast = store
            .add_gate_bucket(
                &fast_gate,
                &[DelayPinInfo::new("a", DelayTime::new(0.0, 0.0))],
            )
            .unwrap();

        let mut node_pwl = PiecewiseLinear::empty();
        node_pwl = store.update_node_pwl(&node_pwl, slow).unwrap();
        node_pwl = store.update_node_pwl(&node_pwl, fast).unwrap();

        assert_eq!(node_pwl.select(0.0), Some(&fast));
        assert_eq!(
            store.compute_pwl_delay(&node_pwl, 4.0).unwrap(),
            DelayTime::new(7.0, 8.0)
        );
    }

    #[test]
    fn wire_bucket_recursively_selects_source_bucket() {
        let gate = sample_gate("inv");
        let wire_gate = GenlibGate {
            name: "**wire**".to_string(),
            area: 0.0,
            output: gate.output.clone(),
            pins: Vec::new(),
        };
        let mut store = BinDelayStore::new(BinDelayCostFunction::MaxRiseFall);
        let pi = store
            .add_primary_input_bucket(DelayTime::new(2.0, 3.0), DelayTime::new(0.5, 0.25))
            .unwrap();
        let mut node_pwl = PiecewiseLinear::empty();
        node_pwl = store.update_node_pwl(&node_pwl, pi).unwrap();
        let wire = store
            .add_wire_bucket(&wire_gate, "a", &node_pwl, false)
            .unwrap();
        let mut wire_node_pwl = PiecewiseLinear::empty();
        wire_node_pwl = store.update_node_pwl(&wire_node_pwl, wire).unwrap();

        assert_eq!(
            store.compute_pwl_delay(&wire_node_pwl, 4.0).unwrap(),
            DelayTime::new(4.0, 4.0)
        );
    }

    #[test]
    fn preserves_lowest_area_binding() {
        let store = BinDelayStore::<&str>::new(BinDelayCostFunction::MaxRiseFall);
        let high = sample_gate("and2");
        let low = sample_gate("inv");
        let mut area = AreaBucket::default();

        store
            .preserve_best_area(
                &mut area,
                &high,
                &[DelayPinInfo::new("a", ZERO_DELAY)],
                |_| 3.0,
            )
            .unwrap();
        store
            .preserve_best_area(
                &mut area,
                &low,
                &[DelayPinInfo::new("b", ZERO_DELAY)],
                |_| 1.0,
            )
            .unwrap();

        assert_eq!(area.area, 2.0);
        assert_eq!(area.gate_name.as_deref(), Some("inv"));
        assert_eq!(area.save_binding, vec!["b"]);
    }

    #[test]
    fn reports_full_tree_match_as_missing_native_integration() {
        assert!(matches!(
            bin_delay_tree_match_unavailable().unwrap_err(),
            BinDelayError::MissingSisPorts { .. }
        ));
    }

    #[test]
    fn no_legacy_abi_tokens_are_present_in_this_port() {
        let source = include_str!("bin_delay.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
