//! Native Rust fanout estimation helpers for `sis/map/fanout_est.c`.
//!
//! The original SIS file computes delay buckets for leaves of a multiple
//! fanout point and later compensates those buckets for the actual mapped pin
//! load. This port keeps that behavior over owned Rust data: callers provide a
//! bounded fanout problem, optional tree leaves, and candidate inverter delay
//! models. Full `node_t`, `MAP(node)`, `BIN(node)`, and PWL integration is left
//! to the native graph/timing ports and reported as typed dependency errors.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const ZERO: Self = Self {
        rise: 0.0,
        fall: 0.0,
    };

    pub const PLUS_INFINITY: Self = Self {
        rise: f64::INFINITY,
        fall: f64::INFINITY,
    };

    pub const MINUS_INFINITY: Self = Self {
        rise: f64::NEG_INFINITY,
        fall: f64::NEG_INFINITY,
    };

    pub fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub fn max_transition(self) -> f64 {
        self.rise.max(self.fall)
    }

    fn validate(self, field: &'static str) -> Result<(), FanoutEstimateError> {
        if !self.rise.is_finite() || !self.fall.is_finite() {
            return Err(FanoutEstimateError::InvalidDelay { field, delay: self });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Polarity {
    X,
    Y,
}

impl Polarity {
    pub fn inverted(self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::X,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutSource {
    FanoutPoint,
    InvertedFanin,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearDelayModel {
    pub base: DelayTime,
    pub slope: DelayTime,
}

impl LinearDelayModel {
    pub fn new(base: DelayTime, slope: DelayTime) -> Result<Self, FanoutEstimateError> {
        let model = Self { base, slope };
        model.validate()?;
        Ok(model)
    }

    pub fn delay_at_load(self, load: f64) -> Result<DelayTime, FanoutEstimateError> {
        validate_load(load, "load")?;
        Ok(DelayTime {
            rise: self.base.rise + self.slope.rise * load,
            fall: self.base.fall + self.slope.fall * load,
        })
    }

    fn validate(self) -> Result<(), FanoutEstimateError> {
        self.base.validate("base")?;
        self.slope.validate("slope")?;
        if self.slope.rise < 0.0 || self.slope.fall < 0.0 {
            return Err(FanoutEstimateError::NegativeDelaySlope { slope: self.slope });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InverterModel {
    pub name: &'static str,
    pub input_load: f64,
    pub delay: LinearDelayModel,
}

impl InverterModel {
    pub fn new(
        name: &'static str,
        input_load: f64,
        delay: LinearDelayModel,
    ) -> Result<Self, FanoutEstimateError> {
        validate_load(input_load, "input_load")?;
        Ok(Self {
            name,
            input_load,
            delay,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutBucket {
    pub load: f64,
    pub delay: LinearDelayModel,
}

impl FanoutBucket {
    pub fn new(load: f64, delay: LinearDelayModel) -> Result<Self, FanoutEstimateError> {
        validate_load(load, "bucket_load")?;
        Ok(Self { load, delay })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutLeaf {
    pub load: f64,
    pub bucket: FanoutBucket,
}

impl FanoutLeaf {
    pub fn new(load: f64, bucket: FanoutBucket) -> Result<Self, FanoutEstimateError> {
        validate_load(load, "leaf_load")?;
        Ok(Self { load, bucket })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutPinInfo {
    pub leaf: usize,
    pub fanout_index: usize,
    pub pin_polarity: Polarity,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutEstimateOptions {
    pub allow_duplication: bool,
    pub ignore_polarity: bool,
    pub load_estimation: LoadEstimationMode,
    pub default_sink_load: f64,
    pub max_fanouts: usize,
}

impl Default for FanoutEstimateOptions {
    fn default() -> Self {
        Self {
            allow_duplication: false,
            ignore_polarity: false,
            load_estimation: LoadEstimationMode::Balanced,
            default_sink_load: 1.0,
            max_fanouts: 65_536,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadEstimationMode {
    Direct,
    Balanced,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutProblem {
    pub n_fanouts: usize,
    pub fanout_polarity: Polarity,
    pub source: FanoutSource,
    pub leaves: [Vec<Option<FanoutLeaf>>; 2],
    pub loads: [Option<f64>; 2],
    pub best_is_inverter: bool,
}

impl FanoutProblem {
    pub fn new(n_fanouts: usize, fanout_polarity: Polarity) -> Result<Self, FanoutEstimateError> {
        if n_fanouts < 2 {
            return Err(FanoutEstimateError::NotMultipleFanout { n_fanouts });
        }

        Ok(Self {
            n_fanouts,
            fanout_polarity,
            source: FanoutSource::FanoutPoint,
            leaves: [vec![None; n_fanouts], vec![None; n_fanouts]],
            loads: [None, None],
            best_is_inverter: false,
        })
    }

    pub fn set_leaf(
        &mut self,
        polarity: Polarity,
        fanout_index: usize,
        leaf: FanoutLeaf,
    ) -> Result<(), FanoutEstimateError> {
        self.check_index(fanout_index)?;
        self.leaves[polarity.index()][fanout_index] = Some(leaf);
        Ok(())
    }

    pub fn leaf(
        &self,
        polarity: Polarity,
        fanout_index: usize,
    ) -> Result<&FanoutLeaf, FanoutEstimateError> {
        self.check_index(fanout_index)?;
        self.leaves[polarity.index()][fanout_index].as_ref().ok_or(
            FanoutEstimateError::MissingLeaf {
                polarity,
                fanout_index,
            },
        )
    }

    pub fn source_polarity(&self) -> Polarity {
        match self.source {
            FanoutSource::FanoutPoint => self.fanout_polarity,
            FanoutSource::InvertedFanin => self.fanout_polarity.inverted(),
        }
    }

    pub fn pin_arrival_time(
        &self,
        pin_info: &FanoutPinInfo,
        pin_load: f64,
    ) -> Result<DelayTime, FanoutEstimateError> {
        if pin_info.leaf != 0 {
            return Err(FanoutEstimateError::UnsupportedOwnedLeafReference {
                leaf: pin_info.leaf,
            });
        }
        let leaf = self.leaf(pin_info.pin_polarity, pin_info.fanout_index)?;
        compensated_pin_arrival_time(leaf, pin_load)
    }

    fn check_index(&self, fanout_index: usize) -> Result<(), FanoutEstimateError> {
        if fanout_index >= self.n_fanouts {
            return Err(FanoutEstimateError::FanoutIndexOutOfRange {
                fanout_index,
                n_fanouts: self.n_fanouts,
            });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutCost {
    pub slack: DelayTime,
    pub area: f64,
}

impl FanoutCost {
    pub fn is_better_than(self, other: Self) -> bool {
        let self_slack = self.slack.max_transition();
        let other_slack = other.slack.max_transition();
        self_slack > other_slack || (self_slack == other_slack && self.area < other.area)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutEstimate {
    pub problem: FanoutProblem,
    pub cost: FanoutCost,
}

pub fn compute_fanout_info(
    mut problem: FanoutProblem,
    options: FanoutEstimateOptions,
    inverters: &[InverterModel],
) -> Result<FanoutEstimate, FanoutEstimateError> {
    validate_options(options)?;
    if problem.n_fanouts > options.max_fanouts {
        return Err(FanoutEstimateError::FanoutLimitExceeded {
            n_fanouts: problem.n_fanouts,
            max_fanouts: options.max_fanouts,
        });
    }

    if no_leaf_is_present(&problem) {
        compute_dummy_fanout_info(&mut problem, options)?;
    }

    let source_polarity = problem.source_polarity();
    let source_load = source_load_for_polarity(&problem, source_polarity)?;
    problem.loads[source_polarity.index()] = Some(source_load);
    extract_remaining_fanout_leaves(&mut problem, options, inverters)?;

    if options.allow_duplication && problem.fanout_polarity == Polarity::Y {
        problem.best_is_inverter = true;
        let source_load = problem.loads[source_polarity.index()].ok_or(
            FanoutEstimateError::MissingSourceLoad {
                polarity: source_polarity,
            },
        )?;
        problem.loads[source_polarity.inverted().index()] = Some(source_load);
    } else if problem.source == FanoutSource::InvertedFanin {
        problem.best_is_inverter = true;
        problem.loads[Polarity::Y.index()] = Some(0.0);
    } else {
        problem.best_is_inverter = false;
    }

    let cost = estimate_cost(&problem)?;
    Ok(FanoutEstimate { problem, cost })
}

pub fn compensated_pin_arrival_time(
    leaf: &FanoutLeaf,
    pin_load: f64,
) -> Result<DelayTime, FanoutEstimateError> {
    validate_load(pin_load, "pin_load")?;
    let load = leaf.bucket.load + (pin_load - leaf.load);
    if load < 0.0 {
        return Err(FanoutEstimateError::NegativeCompensatedLoad {
            bucket_load: leaf.bucket.load,
            leaf_load: leaf.load,
            pin_load,
        });
    }

    leaf.bucket.delay.delay_at_load(load)
}

pub fn estimate_cost(problem: &FanoutProblem) -> Result<FanoutCost, FanoutEstimateError> {
    let mut worst_arrival = DelayTime::MINUS_INFINITY;
    let mut area = 0.0;
    let mut seen = 0usize;

    for polarity in [Polarity::X, Polarity::Y] {
        for leaf in problem.leaves[polarity.index()].iter().flatten() {
            let arrival = compensated_pin_arrival_time(leaf, leaf.load)?;
            worst_arrival.rise = worst_arrival.rise.max(arrival.rise);
            worst_arrival.fall = worst_arrival.fall.max(arrival.fall);
            area += leaf.bucket.load.max(leaf.load);
            seen += 1;
        }
    }

    if seen == 0 {
        return Err(FanoutEstimateError::NoFanoutLeaves);
    }

    Ok(FanoutCost {
        slack: DelayTime {
            rise: -worst_arrival.rise,
            fall: -worst_arrival.fall,
        },
        area,
    })
}

pub fn select_non_inverter_load(
    candidate_loads: &[f64],
    load: f64,
) -> Result<f64, FanoutEstimateError> {
    validate_load(load, "load")?;
    let mut best = None::<(f64, f64)>;
    for candidate in candidate_loads {
        validate_load(*candidate, "candidate_load")?;
        let distance = (*candidate - load).abs();
        if best.is_none_or(|(_, best_distance)| distance < best_distance) {
            best = Some((*candidate, distance));
        }
    }

    best.map(|(load, _)| load)
        .ok_or(FanoutEstimateError::MissingNonInverterCandidate)
}

pub fn require_full_graph_integration<T>() -> Result<T, FanoutEstimateError> {
    Err(FanoutEstimateError::MissingPortDependencies {
        operation: "full SIS graph fanout estimation",
    })
}

fn validate_options(options: FanoutEstimateOptions) -> Result<(), FanoutEstimateError> {
    validate_load(options.default_sink_load, "default_sink_load")?;
    if options.max_fanouts < 2 {
        return Err(FanoutEstimateError::InvalidFanoutLimit {
            max_fanouts: options.max_fanouts,
        });
    }

    Ok(())
}

fn compute_dummy_fanout_info(
    problem: &mut FanoutProblem,
    options: FanoutEstimateOptions,
) -> Result<(), FanoutEstimateError> {
    let source_polarity = problem.source_polarity();
    let slope = match options.load_estimation {
        LoadEstimationMode::Direct => DelayTime::new(0.0, 0.0),
        LoadEstimationMode::Balanced => DelayTime::new(0.05, 0.05),
    };
    let delay = LinearDelayModel::new(DelayTime::ZERO, slope)?;
    for fanout_index in 0..problem.n_fanouts {
        let leaf = FanoutLeaf::new(
            options.default_sink_load,
            FanoutBucket::new(options.default_sink_load, delay)?,
        )?;
        problem.set_leaf(source_polarity, fanout_index, leaf)?;
    }

    Ok(())
}

fn extract_remaining_fanout_leaves(
    problem: &mut FanoutProblem,
    options: FanoutEstimateOptions,
    inverters: &[InverterModel],
) -> Result<(), FanoutEstimateError> {
    for fanout_index in 0..problem.n_fanouts {
        let x_exists = problem.leaves[Polarity::X.index()][fanout_index].is_some();
        let y_exists = problem.leaves[Polarity::Y.index()][fanout_index].is_some();
        match (x_exists, y_exists) {
            (true, false) => {
                let old_leaf = problem.leaf(Polarity::X, fanout_index)?.clone();
                let new_leaf = extend_fanout_leaf(&old_leaf, options, inverters)?;
                problem.set_leaf(Polarity::Y, fanout_index, new_leaf)?;
            }
            (false, true) => {
                let old_leaf = problem.leaf(Polarity::Y, fanout_index)?.clone();
                let new_leaf = extend_fanout_leaf(&old_leaf, options, inverters)?;
                problem.set_leaf(Polarity::X, fanout_index, new_leaf)?;
            }
            (true, true) => {}
            (false, false) => {
                return Err(FanoutEstimateError::MissingBothPolarities { fanout_index });
            }
        }
    }

    Ok(())
}

fn extend_fanout_leaf(
    old_leaf: &FanoutLeaf,
    options: FanoutEstimateOptions,
    inverters: &[InverterModel],
) -> Result<FanoutLeaf, FanoutEstimateError> {
    if options.ignore_polarity {
        return FanoutLeaf::new(old_leaf.load, old_leaf.bucket);
    }

    let mut best = None::<(FanoutBucket, DelayTime)>;
    for inverter in inverters {
        let upstream_load = inverter.input_load + old_leaf.bucket.load - old_leaf.load;
        if upstream_load < 0.0 {
            continue;
        }

        let input_arrival = old_leaf.bucket.delay.delay_at_load(upstream_load)?;
        let combined_delay = LinearDelayModel::new(
            DelayTime {
                rise: input_arrival.rise + inverter.delay.base.rise,
                fall: input_arrival.fall + inverter.delay.base.fall,
            },
            inverter.delay.slope,
        )?;
        let arrival = combined_delay.delay_at_load(old_leaf.load)?;
        let bucket = FanoutBucket::new(old_leaf.load, combined_delay)?;

        if best.as_ref().is_none_or(|(_, best_arrival)| {
            arrival.max_transition() < best_arrival.max_transition()
        }) {
            best = Some((bucket, arrival));
        }
    }

    let (bucket, _) = best.ok_or(FanoutEstimateError::MissingInverterModel)?;
    FanoutLeaf::new(old_leaf.load, bucket)
}

fn source_load_for_polarity(
    problem: &FanoutProblem,
    polarity: Polarity,
) -> Result<f64, FanoutEstimateError> {
    let mut load = 0.0;
    let mut seen = false;
    for leaf in &problem.leaves[polarity.index()] {
        if let Some(leaf) = leaf {
            load += leaf.bucket.load;
            seen = true;
        }
    }

    if !seen {
        return Err(FanoutEstimateError::MissingSourceLoad { polarity });
    }

    Ok(load)
}

fn no_leaf_is_present(problem: &FanoutProblem) -> bool {
    problem
        .leaves
        .iter()
        .all(|leaves| leaves.iter().all(Option::is_none))
}

fn validate_load(load: f64, field: &'static str) -> Result<(), FanoutEstimateError> {
    if !load.is_finite() || load < 0.0 {
        return Err(FanoutEstimateError::InvalidLoad { field, load });
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub enum FanoutEstimateError {
    NotMultipleFanout {
        n_fanouts: usize,
    },
    FanoutLimitExceeded {
        n_fanouts: usize,
        max_fanouts: usize,
    },
    InvalidFanoutLimit {
        max_fanouts: usize,
    },
    FanoutIndexOutOfRange {
        fanout_index: usize,
        n_fanouts: usize,
    },
    InvalidLoad {
        field: &'static str,
        load: f64,
    },
    InvalidDelay {
        field: &'static str,
        delay: DelayTime,
    },
    NegativeDelaySlope {
        slope: DelayTime,
    },
    NegativeCompensatedLoad {
        bucket_load: f64,
        leaf_load: f64,
        pin_load: f64,
    },
    MissingLeaf {
        polarity: Polarity,
        fanout_index: usize,
    },
    MissingBothPolarities {
        fanout_index: usize,
    },
    MissingSourceLoad {
        polarity: Polarity,
    },
    MissingInverterModel,
    MissingNonInverterCandidate,
    NoFanoutLeaves,
    UnsupportedOwnedLeafReference {
        leaf: usize,
    },
    MissingPortDependencies {
        operation: &'static str,
    },
}

impl fmt::Display for FanoutEstimateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMultipleFanout { n_fanouts } => {
                write!(
                    f,
                    "fanout estimation requires at least two fanouts, got {n_fanouts}"
                )
            }
            Self::FanoutLimitExceeded {
                n_fanouts,
                max_fanouts,
            } => {
                write!(
                    f,
                    "fanout count {n_fanouts} exceeds configured limit {max_fanouts}"
                )
            }
            Self::InvalidFanoutLimit { max_fanouts } => {
                write!(f, "fanout limit must be at least two, got {max_fanouts}")
            }
            Self::FanoutIndexOutOfRange {
                fanout_index,
                n_fanouts,
            } => {
                write!(
                    f,
                    "fanout index {fanout_index} is outside fanout count {n_fanouts}"
                )
            }
            Self::InvalidLoad { field, load } => {
                write!(f, "{field} must be finite and non-negative, got {load}")
            }
            Self::InvalidDelay { field, delay } => {
                write!(
                    f,
                    "{field} delay must be finite, got rise {} fall {}",
                    delay.rise, delay.fall
                )
            }
            Self::NegativeDelaySlope { slope } => {
                write!(
                    f,
                    "delay slope must be non-negative, got rise {} fall {}",
                    slope.rise, slope.fall
                )
            }
            Self::NegativeCompensatedLoad {
                bucket_load,
                leaf_load,
                pin_load,
            } => {
                write!(
                    f,
                    "compensated load is negative for bucket load {bucket_load}, leaf load {leaf_load}, pin load {pin_load}"
                )
            }
            Self::MissingLeaf {
                polarity,
                fanout_index,
            } => {
                write!(
                    f,
                    "missing {polarity:?} fanout leaf at index {fanout_index}"
                )
            }
            Self::MissingBothPolarities { fanout_index } => {
                write!(
                    f,
                    "fanout index {fanout_index} has no leaf in either polarity"
                )
            }
            Self::MissingSourceLoad { polarity } => {
                write!(f, "missing source load for {polarity:?} polarity")
            }
            Self::MissingInverterModel => write!(f, "at least one inverter model is required"),
            Self::MissingNonInverterCandidate => {
                write!(f, "at least one non-inverter load candidate is required")
            }
            Self::NoFanoutLeaves => write!(f, "fanout estimate has no leaves"),
            Self::UnsupportedOwnedLeafReference { leaf } => {
                write!(
                    f,
                    "owned fanout estimate supports only local leaf 0, got {leaf}"
                )
            }
            Self::MissingPortDependencies { operation } => {
                write!(f, "{operation} requires unavailable native SIS integration")
            }
        }
    }
}

impl Error for FanoutEstimateError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(base: f64, slope: f64) -> LinearDelayModel {
        LinearDelayModel::new(DelayTime::new(base, base), DelayTime::new(slope, slope)).unwrap()
    }

    fn inverter(name: &'static str, input_load: f64, base: f64, slope: f64) -> InverterModel {
        InverterModel::new(name, input_load, model(base, slope)).unwrap()
    }

    fn leaf(load: f64, bucket_load: f64, base: f64, slope: f64) -> FanoutLeaf {
        FanoutLeaf::new(
            load,
            FanoutBucket::new(bucket_load, model(base, slope)).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn compensated_pin_arrival_uses_bucket_delta_load() {
        let leaf = leaf(3.0, 5.0, 1.0, 0.5);

        let arrival = compensated_pin_arrival_time(&leaf, 4.0).unwrap();

        assert_eq!(arrival, DelayTime::new(4.0, 4.0));
    }

    #[test]
    fn compensated_pin_arrival_rejects_negative_load() {
        let leaf = leaf(5.0, 1.0, 1.0, 0.5);

        let error = compensated_pin_arrival_time(&leaf, 2.0).unwrap_err();

        assert!(matches!(
            error,
            FanoutEstimateError::NegativeCompensatedLoad {
                bucket_load: 1.0,
                leaf_load: 5.0,
                pin_load: 2.0
            }
        ));
    }

    #[test]
    fn compute_fanout_info_extends_missing_polarity_with_best_inverter() {
        let mut problem = FanoutProblem::new(2, Polarity::Y).unwrap();
        problem
            .set_leaf(Polarity::Y, 0, leaf(2.0, 2.0, 0.5, 0.1))
            .unwrap();
        problem
            .set_leaf(Polarity::Y, 1, leaf(4.0, 4.0, 0.5, 0.1))
            .unwrap();

        let estimate = compute_fanout_info(
            problem,
            FanoutEstimateOptions::default(),
            &[
                inverter("slow", 2.0, 10.0, 1.0),
                inverter("fast", 1.0, 1.0, 0.0),
            ],
        )
        .unwrap();

        assert!(estimate.problem.leaf(Polarity::X, 0).is_ok());
        assert!(estimate.problem.leaf(Polarity::X, 1).is_ok());
        let arrival = estimate
            .problem
            .pin_arrival_time(
                &FanoutPinInfo {
                    leaf: 0,
                    fanout_index: 0,
                    pin_polarity: Polarity::X,
                },
                2.0,
            )
            .unwrap();
        assert_eq!(arrival, DelayTime::new(1.6, 1.6));
    }

    #[test]
    fn compute_fanout_info_creates_bounded_dummy_tree_when_no_leaves_exist() {
        let problem = FanoutProblem::new(3, Polarity::X).unwrap();
        let options = FanoutEstimateOptions {
            default_sink_load: 2.5,
            ..FanoutEstimateOptions::default()
        };

        let estimate =
            compute_fanout_info(problem, options, &[inverter("inv", 1.0, 0.25, 0.1)]).unwrap();

        assert_eq!(estimate.problem.loads[Polarity::X.index()], Some(7.5));
        assert!(estimate.problem.leaf(Polarity::Y, 2).is_ok());
    }

    #[test]
    fn compute_fanout_info_enforces_configured_bound() {
        let problem = FanoutProblem::new(4, Polarity::X).unwrap();
        let options = FanoutEstimateOptions {
            max_fanouts: 3,
            ..FanoutEstimateOptions::default()
        };

        let error = compute_fanout_info(problem, options, &[]).unwrap_err();

        assert!(matches!(
            error,
            FanoutEstimateError::FanoutLimitExceeded {
                n_fanouts: 4,
                max_fanouts: 3
            }
        ));
    }

    #[test]
    fn select_non_inverter_load_returns_nearest_candidate() {
        let selected = select_non_inverter_load(&[1.0, 2.5, 4.0], 3.0).unwrap();

        assert_eq!(selected, 2.5);
    }
}
