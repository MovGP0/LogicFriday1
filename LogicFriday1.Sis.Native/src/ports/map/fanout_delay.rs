//! Native Rust fanout-delay support for `sis/map/fanout_delay.c`.
//!
//! The original SIS file keeps process-global buffer/source tables and dispatch
//! callbacks through C function pointers. This port uses owned model state and
//! typed Rust errors. It implements the bounded timing behavior that does not
//! require a live SIS `network_t`: buffer timing, primary-input source timing,
//! explicit internal-source timing, PWL source selection, load-limit penalties,
//! and the two-level inverter-count search used by fanout optimization.

use std::error::Error;
use std::fmt;

use super::library::{GenlibGate, PinPhase};
use super::pwl::{PiecewiseLinear, PiecewisePoint};
use super::virtual_net::{DelayTime, MINUS_INFINITY};

pub const ZERO_DELAY: DelayTime = DelayTime {
    rise: 0.0,
    fall: 0.0,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanoutPolarity {
    Positive,
    Negative,
}

impl FanoutPolarity {
    pub fn inverted(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferKind {
    NonInverting,
    Inverting,
}

impl BufferKind {
    pub fn polarity(self) -> FanoutPolarity {
        match self {
            Self::NonInverting => FanoutPolarity::Positive,
            Self::Inverting => FanoutPolarity::Negative,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingPhase {
    Inverting,
    NonInverting,
    Unknown,
}

impl From<PinPhase> for TimingPhase {
    fn from(value: PinPhase) -> Self {
        match value {
            PinPhase::Inv => Self::Inverting,
            PinPhase::NonInv => Self::NonInverting,
            PinPhase::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutDelayOptions {
    pub check_load_limit: bool,
    pub penalty_factor: f64,
    pub wire_load_per_fanout: f64,
}

impl Default for FanoutDelayOptions {
    fn default() -> Self {
        Self {
            check_load_limit: false,
            penalty_factor: 1.0,
            wire_load_per_fanout: 0.0,
        }
    }
}

impl FanoutDelayOptions {
    fn validate(self) -> Result<(), FanoutDelayError> {
        if !self.penalty_factor.is_finite() || self.penalty_factor < 0.0 {
            return Err(FanoutDelayError::InvalidOption {
                name: "penalty_factor",
                value: self.penalty_factor,
            });
        }
        if !self.wire_load_per_fanout.is_finite() || self.wire_load_per_fanout < 0.0 {
            return Err(FanoutDelayError::InvalidOption {
                name: "wire_load_per_fanout",
                value: self.wire_load_per_fanout,
            });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanoutPinTiming {
    pub phase: TimingPhase,
    pub input_load: f64,
    pub max_load: f64,
    pub rise_block_delay: f64,
    pub rise_fanout_delay: f64,
    pub fall_block_delay: f64,
    pub fall_fanout_delay: f64,
}

impl FanoutPinTiming {
    pub fn new(
        phase: TimingPhase,
        input_load: f64,
        max_load: f64,
        rise_block_delay: f64,
        rise_fanout_delay: f64,
        fall_block_delay: f64,
        fall_fanout_delay: f64,
    ) -> Self {
        Self {
            phase,
            input_load,
            max_load,
            rise_block_delay,
            rise_fanout_delay,
            fall_block_delay,
            fall_fanout_delay,
        }
    }

    pub fn from_genlib_pin(pin: &super::library::GenlibPin) -> Self {
        Self {
            phase: pin.phase.into(),
            input_load: pin.input_load,
            max_load: pin.max_load,
            rise_block_delay: pin.rise_block_delay,
            rise_fanout_delay: pin.rise_fanout_delay,
            fall_block_delay: pin.fall_block_delay,
            fall_fanout_delay: pin.fall_fanout_delay,
        }
    }

    fn validate(self, gate: &str, pin: usize) -> Result<(), FanoutDelayError> {
        let finite_values = [
            self.input_load,
            self.rise_block_delay,
            self.rise_fanout_delay,
            self.fall_block_delay,
            self.fall_fanout_delay,
        ];
        if finite_values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || self.max_load.is_nan()
            || self.max_load < 0.0
        {
            return Err(FanoutDelayError::InvalidPinTiming {
                gate: gate.to_string(),
                pin,
            });
        }

        Ok(())
    }

    fn block(self) -> DelayTime {
        DelayTime::new(self.rise_block_delay, self.fall_block_delay)
    }

    fn drive(self) -> DelayTime {
        DelayTime::new(self.rise_fanout_delay, self.fall_fanout_delay)
    }

    fn delay(self, load: f64) -> DelayTime {
        DelayTime::new(
            self.rise_block_delay + self.rise_fanout_delay * load,
            self.fall_block_delay + self.fall_fanout_delay * load,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutBuffer {
    pub name: String,
    pub kind: BufferKind,
    pub area: f64,
    pub input_load: f64,
    pub load_limit: f64,
    pub alpha: DelayTime,
    pub beta: DelayTime,
}

impl FanoutBuffer {
    pub fn new(
        name: impl Into<String>,
        kind: BufferKind,
        area: f64,
        input_load: f64,
        load_limit: f64,
        alpha: DelayTime,
        beta: DelayTime,
    ) -> Result<Self, FanoutDelayError> {
        let buffer = Self {
            name: name.into(),
            kind,
            area,
            input_load,
            load_limit,
            alpha,
            beta,
        };
        buffer.validate()?;
        Ok(buffer)
    }

    pub fn from_genlib_gate(gate: &GenlibGate) -> Result<Self, FanoutDelayError> {
        let Some(pin) = gate.pins.first() else {
            return Err(FanoutDelayError::MissingBufferPin {
                gate: gate.name.clone(),
            });
        };

        let timing = FanoutPinTiming::from_genlib_pin(pin);
        let kind = match timing.phase {
            TimingPhase::NonInverting => BufferKind::NonInverting,
            TimingPhase::Inverting => BufferKind::Inverting,
            TimingPhase::Unknown => {
                return Err(FanoutDelayError::InvalidBufferPhase {
                    gate: gate.name.clone(),
                });
            }
        };

        Self::new(
            gate.name.clone(),
            kind,
            gate.area,
            timing.input_load,
            timing.max_load,
            timing.block(),
            timing.drive(),
        )
    }

    fn validate(&self) -> Result<(), FanoutDelayError> {
        if self.name.is_empty() {
            return Err(FanoutDelayError::EmptyGateName);
        }

        let values = [
            self.area,
            self.input_load,
            self.alpha.rise,
            self.alpha.fall,
            self.beta.rise,
            self.beta.fall,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || self.load_limit.is_nan()
            || self.load_limit < 0.0
        {
            return Err(FanoutDelayError::InvalidBufferTiming {
                gate: self.name.clone(),
            });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimaryInputSource {
    pub name: String,
    pub arrival: DelayTime,
    pub drive: DelayTime,
    pub load_limit: f64,
}

impl PrimaryInputSource {
    pub fn new(
        name: impl Into<String>,
        arrival: DelayTime,
        drive: DelayTime,
        load_limit: f64,
    ) -> Result<Self, FanoutDelayError> {
        let source = Self {
            name: name.into(),
            arrival,
            drive,
            load_limit,
        };
        source.validate()?;
        Ok(source)
    }

    fn validate(&self) -> Result<(), FanoutDelayError> {
        if self.name.is_empty() {
            return Err(FanoutDelayError::EmptySourceName);
        }
        if !is_valid_delay(self.arrival)
            || !is_valid_delay(self.drive)
            || self.drive.rise < 0.0
            || self.drive.fall < 0.0
            || self.load_limit.is_nan()
            || self.load_limit < 0.0
        {
            return Err(FanoutDelayError::InvalidSourceTiming {
                source: self.name.clone(),
            });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InternalSource {
    pub name: String,
    pub input_arrivals: Vec<DelayTime>,
    pub pins: Vec<FanoutPinTiming>,
    pub load_limit: f64,
}

impl InternalSource {
    pub fn new(
        name: impl Into<String>,
        input_arrivals: Vec<DelayTime>,
        pins: Vec<FanoutPinTiming>,
        load_limit: f64,
    ) -> Result<Self, FanoutDelayError> {
        let source = Self {
            name: name.into(),
            input_arrivals,
            pins,
            load_limit,
        };
        source.validate()?;
        Ok(source)
    }

    fn validate(&self) -> Result<(), FanoutDelayError> {
        if self.name.is_empty() {
            return Err(FanoutDelayError::EmptySourceName);
        }
        if self.input_arrivals.len() != self.pins.len() {
            return Err(FanoutDelayError::PinTimingMismatch {
                source: self.name.clone(),
                expected: self.input_arrivals.len(),
                actual: self.pins.len(),
            });
        }
        if self
            .input_arrivals
            .iter()
            .any(|arrival| !is_valid_delay(*arrival))
            || self.load_limit.is_nan()
            || self.load_limit < 0.0
        {
            return Err(FanoutDelayError::InvalidSourceTiming {
                source: self.name.clone(),
            });
        }
        for (pin, timing) in self.pins.iter().copied().enumerate() {
            timing.validate(&self.name, pin)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutDelayPwl<T> {
    pub rise: PiecewiseLinear<T>,
    pub fall: PiecewiseLinear<T>,
}

impl<T> FanoutDelayPwl<T> {
    pub fn new(rise: PiecewiseLinear<T>, fall: PiecewiseLinear<T>) -> Self {
        Self { rise, fall }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PwlSource<T> {
    pub name: String,
    pub delay: FanoutDelayPwl<T>,
}

impl<T> PwlSource<T> {
    pub fn new(
        name: impl Into<String>,
        delay: FanoutDelayPwl<T>,
    ) -> Result<Self, FanoutDelayError> {
        let source = Self {
            name: name.into(),
            delay,
        };
        if source.name.is_empty() {
            return Err(FanoutDelayError::EmptySourceName);
        }

        Ok(source)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FanoutSource<T> {
    PrimaryInput(PrimaryInputSource),
    Internal(InternalSource),
    Pwl(PwlSource<T>),
    MissingNativeSource {
        name: String,
    },
}

impl<T> FanoutSource<T> {
    pub fn name(&self) -> &str {
        match self {
            Self::PrimaryInput(source) => &source.name,
            Self::Internal(source) => &source.name,
            Self::Pwl(source) => &source.name,
            Self::MissingNativeSource { name, .. } => name,
        }
    }

    fn load_limit(&self) -> f64 {
        match self {
            Self::PrimaryInput(source) => source.load_limit,
            Self::Internal(source) => source.load_limit,
            Self::Pwl(_) => f64::INFINITY,
            Self::MissingNativeSource { .. } => f64::INFINITY,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FanoutDelayGate<T> {
    Buffer(FanoutBuffer),
    Source {
        polarity: FanoutPolarity,
        source: FanoutSource<T>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FanoutGateCounts {
    pub positive_buffers: usize,
    pub negative_buffers: usize,
    pub buffers: usize,
    pub positive_sources: usize,
    pub negative_sources: usize,
    pub gates: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FanoutDelayModel<T> {
    options: FanoutDelayOptions,
    gates: Vec<FanoutDelayGate<T>>,
    positive_buffer_count: usize,
    positive_source_count: usize,
}

impl<T> FanoutDelayModel<T> {
    pub fn new(
        positive_buffers: Vec<FanoutBuffer>,
        negative_buffers: Vec<FanoutBuffer>,
        options: FanoutDelayOptions,
    ) -> Result<Self, FanoutDelayError> {
        options.validate()?;

        let positive_buffer_count = positive_buffers.len();
        let mut gates = Vec::with_capacity(positive_buffers.len() + negative_buffers.len());
        for buffer in positive_buffers {
            if buffer.kind != BufferKind::NonInverting {
                return Err(FanoutDelayError::InvalidBufferPartition {
                    gate: buffer.name,
                    expected: BufferKind::NonInverting,
                    actual: buffer.kind,
                });
            }
            gates.push(FanoutDelayGate::Buffer(buffer));
        }
        for buffer in negative_buffers {
            if buffer.kind != BufferKind::Inverting {
                return Err(FanoutDelayError::InvalidBufferPartition {
                    gate: buffer.name,
                    expected: BufferKind::Inverting,
                    actual: buffer.kind,
                });
            }
            gates.push(FanoutDelayGate::Buffer(buffer));
        }

        Ok(Self {
            options,
            gates,
            positive_buffer_count,
            positive_source_count: 0,
        })
    }

    pub fn from_buffers(
        buffers: impl IntoIterator<Item = FanoutBuffer>,
        options: FanoutDelayOptions,
    ) -> Result<Self, FanoutDelayError> {
        let mut positive = Vec::new();
        let mut negative = Vec::new();
        for buffer in buffers {
            match buffer.kind {
                BufferKind::NonInverting => positive.push(buffer),
                BufferKind::Inverting => negative.push(buffer),
            }
        }

        Self::new(positive, negative, options)
    }

    pub fn from_genlib_buffers(
        gates: impl IntoIterator<Item = GenlibGate>,
        options: FanoutDelayOptions,
    ) -> Result<Self, FanoutDelayError> {
        Self::from_buffers(
            gates
                .into_iter()
                .map(|gate| FanoutBuffer::from_genlib_gate(&gate))
                .collect::<Result<Vec<_>, _>>()?,
            options,
        )
    }

    pub fn counts(&self) -> FanoutGateCounts {
        let buffers = self.buffer_count();
        FanoutGateCounts {
            positive_buffers: self.positive_buffer_count,
            negative_buffers: buffers,
            buffers,
            positive_sources: buffers + self.positive_source_count,
            negative_sources: self.gates.len(),
            gates: self.gates.len(),
        }
    }

    pub fn gates(&self) -> &[FanoutDelayGate<T>] {
        &self.gates
    }

    pub fn add_source(
        &mut self,
        source: FanoutSource<T>,
        polarity: FanoutPolarity,
    ) -> Result<usize, FanoutDelayError> {
        let gate = FanoutDelayGate::Source { polarity, source };
        let index = match polarity {
            FanoutPolarity::Positive => {
                if self.negative_source_count() > 0 {
                    return Err(FanoutDelayError::PositiveSourceAfterNegative);
                }
                let index = self.buffer_count() + self.positive_source_count;
                self.gates.insert(index, gate);
                self.positive_source_count += 1;
                index
            }
            FanoutPolarity::Negative => {
                let index =
                    self.buffer_count() + self.positive_source_count + self.negative_source_count();
                self.gates.insert(index, gate);
                index
            }
        };

        Ok(index)
    }

    pub fn free_sources(&mut self) {
        self.gates.truncate(self.buffer_count());
        self.positive_source_count = 0;
    }

    pub fn buffer_load(&self, gate_index: usize) -> Result<f64, FanoutDelayError> {
        Ok(self.buffer(gate_index)?.input_load)
    }

    pub fn area(&self, gate_index: usize) -> Result<f64, FanoutDelayError> {
        match self.gate(gate_index)? {
            FanoutDelayGate::Buffer(buffer) => Ok(buffer.area),
            FanoutDelayGate::Source { .. } => Ok(0.0),
        }
    }

    pub fn source_name(&self, source_index: usize) -> Result<&str, FanoutDelayError> {
        let FanoutDelayGate::Source { source, .. } = self.gate(source_index)? else {
            return Err(FanoutDelayError::ExpectedSource {
                index: source_index,
            });
        };

        Ok(source.name())
    }

    pub fn source_polarity(&self, source_index: usize) -> Result<FanoutPolarity, FanoutDelayError> {
        let FanoutDelayGate::Source { polarity, .. } = self.gate(source_index)? else {
            return Err(FanoutDelayError::ExpectedSource {
                index: source_index,
            });
        };

        Ok(*polarity)
    }

    pub fn buffer_polarity(&self, buffer_index: usize) -> Result<FanoutPolarity, FanoutDelayError> {
        Ok(self.buffer(buffer_index)?.kind.polarity())
    }

    pub fn buffer_index(&self, name: &str) -> Option<usize> {
        self.gates.iter().enumerate().find_map(|(index, gate)| {
            let FanoutDelayGate::Buffer(buffer) = gate else {
                return None;
            };
            (buffer.name == name).then_some(index)
        })
    }

    pub fn source_index(&self, name: &str) -> Option<usize> {
        self.gates.iter().enumerate().find_map(|(index, gate)| {
            let FanoutDelayGate::Source { source, .. } = gate else {
                return None;
            };
            (source.name() == name).then_some(index)
        })
    }

    pub fn backward_intrinsic(
        &self,
        required: DelayTime,
        gate_index: usize,
    ) -> Result<DelayTime, FanoutDelayError> {
        let buffer = self.buffer(gate_index)?;
        match buffer.kind {
            BufferKind::NonInverting => Ok(sub_delay(required, buffer.alpha)),
            BufferKind::Inverting => Ok(DelayTime::new(
                required.fall - buffer.alpha.fall,
                required.rise - buffer.alpha.rise,
            )),
        }
    }

    pub fn forward_intrinsic(
        &self,
        arrival: DelayTime,
        gate_index: usize,
    ) -> Result<DelayTime, FanoutDelayError> {
        let buffer = self.buffer(gate_index)?;
        match buffer.kind {
            BufferKind::NonInverting => Ok(add_delay(arrival, buffer.alpha)),
            BufferKind::Inverting => Ok(DelayTime::new(
                arrival.fall + buffer.alpha.rise,
                arrival.rise + buffer.alpha.fall,
            )),
        }
    }

    pub fn backward_load_dependent(
        &self,
        required: DelayTime,
        gate_index: usize,
        load: f64,
    ) -> Result<DelayTime, FanoutDelayError>
    where
        T: Clone,
    {
        let load = self.effective_load(gate_index, load)?;
        match self.gate(gate_index)? {
            FanoutDelayGate::Buffer(buffer) => Ok(DelayTime::new(
                required.rise - buffer.beta.rise * load,
                required.fall - buffer.beta.fall * load,
            )),
            FanoutDelayGate::Source { source, .. } => {
                let arrival = self.source_arrival(source, load)?;
                Ok(sub_delay(required, arrival))
            }
        }
    }

    pub fn forward_load_dependent(
        &self,
        arrival: DelayTime,
        gate_index: usize,
        load: f64,
    ) -> Result<DelayTime, FanoutDelayError>
    where
        T: Clone,
    {
        let load = self.effective_load(gate_index, load)?;
        match self.gate(gate_index)? {
            FanoutDelayGate::Buffer(buffer) => Ok(DelayTime::new(
                arrival.rise + buffer.beta.rise * load,
                arrival.fall + buffer.beta.fall * load,
            )),
            FanoutDelayGate::Source { source, .. } => {
                if !delay_time_eq(arrival, ZERO_DELAY) {
                    return Err(FanoutDelayError::SourceForwardArrivalNotZero {
                        source: source.name().to_string(),
                        arrival,
                    });
                }
                self.source_arrival(source, load)
            }
        }
    }

    pub fn delay_pwl(
        &self,
        gate_index: usize,
        load: f64,
        arrival: DelayTime,
    ) -> Result<FanoutDelayPwl<T>, FanoutDelayError>
    where
        T: Clone,
    {
        match self.gate(gate_index)? {
            FanoutDelayGate::Buffer(buffer) => Ok(FanoutDelayPwl::new(
                PiecewiseLinear::extract(vec![PiecewisePoint::new(
                    0.0,
                    arrival.rise,
                    buffer.beta.rise,
                    None,
                )]),
                PiecewiseLinear::extract(vec![PiecewisePoint::new(
                    0.0,
                    arrival.fall,
                    buffer.beta.fall,
                    None,
                )]),
            )),
            FanoutDelayGate::Source { source, .. } => {
                if !delay_time_eq(arrival, ZERO_DELAY) {
                    return Err(FanoutDelayError::SourceForwardArrivalNotZero {
                        source: source.name().to_string(),
                        arrival,
                    });
                }
                match source {
                    FanoutSource::Pwl(source) => Ok(select_active_pwl(&source.delay, load)),
                    FanoutSource::MissingNativeSource { .. } => {
                        Err(FanoutDelayError::MissingSisPorts {
                            operation: "fanout-delay native source PWL extraction",
                        })
                    }
                    FanoutSource::PrimaryInput(_) | FanoutSource::Internal(_) => {
                        Err(FanoutDelayError::MissingSisPorts {
                            operation: "fanout-delay source PWL extraction",
                        })
                    }
                }
            }
        }
    }

    pub fn compute_best_number_of_inverters(
        &self,
        source_index: usize,
        buffer_index: usize,
        load: f64,
        max_n: usize,
    ) -> Result<usize, FanoutDelayError>
    where
        T: Clone,
    {
        if max_n == 0 {
            return Err(FanoutDelayError::InvalidMaxInverterCount(max_n));
        }
        validate_load(load)?;
        let mut from = 1;
        let mut to = max_n;

        if source_index < self.buffer_count() {
            let source = self.buffer(source_index)?;
            let buffer = self.buffer(buffer_index)?;
            let first_stage_load = buffer.input_load + self.wire_load(1);
            let a = DelayTime::new(
                source.beta.rise * first_stage_load,
                source.beta.fall * first_stage_load,
            );
            let b = match buffer.kind {
                BufferKind::NonInverting => {
                    DelayTime::new(buffer.beta.rise * load, buffer.beta.fall * load)
                }
                BufferKind::Inverting => {
                    DelayTime::new(buffer.beta.fall * load, buffer.beta.rise * load)
                }
            };
            let c = DelayTime::new(
                search_ratio(b.rise, a.rise, max_n),
                search_ratio(b.fall, a.fall, max_n),
            );
            from = c.rise.min(c.fall).floor() as usize;
            to = (c.rise.max(c.fall) + 1.0).floor() as usize;
            to = to.min(max_n);
            if from == 0 {
                from = 1;
            }
            if from > to {
                from = to;
            }
        } else {
            let _ = self.gate(source_index)?;
            let _ = self.buffer(buffer_index)?;
        }

        self.linear_search_best_number_of_inverters(
            source_index,
            buffer_index,
            load,
            max_n,
            from,
            to,
        )
    }

    fn linear_search_best_number_of_inverters(
        &self,
        source_index: usize,
        buffer_index: usize,
        load: f64,
        max_n: usize,
        from: usize,
        to: usize,
    ) -> Result<usize, FanoutDelayError>
    where
        T: Clone,
    {
        if from == 0 || from > to || to > max_n {
            return Err(FanoutDelayError::InvalidSearchWindow { from, to, max_n });
        }
        if from == to {
            return Ok(from);
        }

        let buffer_load = self.buffer(buffer_index)?.input_load;
        let mut best_required = MINUS_INFINITY;
        let mut best_index = None;
        for count in from..=to {
            let count_as_f64 = count as f64;
            let mut local_required =
                self.backward_load_dependent(ZERO_DELAY, buffer_index, load / count_as_f64)?;
            local_required = self.backward_intrinsic(local_required, buffer_index)?;
            let local_load = buffer_load * count_as_f64 + self.wire_load(count);
            local_required =
                self.backward_load_dependent(local_required, source_index, local_load)?;
            if min_component(best_required) < min_component(local_required) {
                best_required = local_required;
                best_index = Some(count);
            }
        }

        best_index.ok_or(FanoutDelayError::InvalidSearchWindow { from, to, max_n })
    }

    fn source_arrival(
        &self,
        source: &FanoutSource<T>,
        load: f64,
    ) -> Result<DelayTime, FanoutDelayError>
    where
        T: Clone,
    {
        match source {
            FanoutSource::PrimaryInput(source) => Ok(DelayTime::new(
                source.drive.rise * load + source.arrival.rise,
                source.drive.fall * load + source.arrival.fall,
            )),
            FanoutSource::Internal(source) => simulate_internal_source(source, load),
            FanoutSource::Pwl(source) => {
                Ok(DelayTime::new(
                    source.delay.rise.eval(load).ok_or_else(|| {
                        FanoutDelayError::EmptyPwlSource {
                            source: source.name.clone(),
                            phase: "rise",
                        }
                    })?,
                    source.delay.fall.eval(load).ok_or_else(|| {
                        FanoutDelayError::EmptyPwlSource {
                            source: source.name.clone(),
                            phase: "fall",
                        }
                    })?,
                ))
            }
            FanoutSource::MissingNativeSource { .. } => {
                Err(FanoutDelayError::MissingSisPorts {
                    operation: "fanout-delay native source load-dependent timing",
                })
            }
        }
    }

    fn effective_load(&self, gate_index: usize, load: f64) -> Result<f64, FanoutDelayError> {
        validate_load(load)?;
        let gate = self.gate(gate_index)?;
        let load_limit = match gate {
            FanoutDelayGate::Buffer(buffer) => buffer.load_limit,
            FanoutDelayGate::Source { source, .. } => source.load_limit(),
        };
        if self.options.check_load_limit && load > load_limit {
            Ok(load * self.options.penalty_factor)
        } else {
            Ok(load)
        }
    }

    fn gate(&self, index: usize) -> Result<&FanoutDelayGate<T>, FanoutDelayError> {
        self.gates
            .get(index)
            .ok_or(FanoutDelayError::GateIndexOutOfRange {
                index,
                len: self.gates.len(),
            })
    }

    fn buffer(&self, index: usize) -> Result<&FanoutBuffer, FanoutDelayError> {
        let FanoutDelayGate::Buffer(buffer) = self.gate(index)? else {
            return Err(FanoutDelayError::ExpectedBuffer { index });
        };

        Ok(buffer)
    }

    fn buffer_count(&self) -> usize {
        self.gates
            .iter()
            .take_while(|gate| matches!(gate, FanoutDelayGate::Buffer(_)))
            .count()
    }

    fn negative_source_count(&self) -> usize {
        self.gates.len() - self.buffer_count() - self.positive_source_count
    }

    fn wire_load(&self, fanout_count: usize) -> f64 {
        self.options.wire_load_per_fanout * fanout_count as f64
    }
}

#[derive(Debug, PartialEq)]
pub enum FanoutDelayError {
    EmptyGateName,
    EmptySourceName,
    InvalidOption {
        name: &'static str,
        value: f64,
    },
    InvalidBufferTiming {
        gate: String,
    },
    InvalidSourceTiming {
        source: String,
    },
    InvalidPinTiming {
        gate: String,
        pin: usize,
    },
    MissingBufferPin {
        gate: String,
    },
    InvalidBufferPhase {
        gate: String,
    },
    InvalidBufferPartition {
        gate: String,
        expected: BufferKind,
        actual: BufferKind,
    },
    PositiveSourceAfterNegative,
    GateIndexOutOfRange {
        index: usize,
        len: usize,
    },
    ExpectedBuffer {
        index: usize,
    },
    ExpectedSource {
        index: usize,
    },
    InvalidLoad(f64),
    PinTimingMismatch {
        source: String,
        expected: usize,
        actual: usize,
    },
    SourceForwardArrivalNotZero {
        source: String,
        arrival: DelayTime,
    },
    EmptyPwlSource {
        source: String,
        phase: &'static str,
    },
    InvalidMaxInverterCount(usize),
    InvalidSearchWindow {
        from: usize,
        to: usize,
        max_n: usize,
    },
    MissingSisPorts {
        operation: &'static str,
    },
}

impl fmt::Display for FanoutDelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGateName => write!(f, "fanout-delay gate name cannot be empty"),
            Self::EmptySourceName => write!(f, "fanout-delay source name cannot be empty"),
            Self::InvalidOption { name, value } => {
                write!(f, "fanout-delay option {name} has invalid value {value}")
            }
            Self::InvalidBufferTiming { gate } => {
                write!(f, "buffer '{gate}' has invalid fanout-delay timing")
            }
            Self::InvalidSourceTiming { source } => {
                write!(f, "source '{source}' has invalid fanout-delay timing")
            }
            Self::InvalidPinTiming { gate, pin } => {
                write!(f, "gate '{gate}' pin {pin} has invalid fanout-delay timing")
            }
            Self::MissingBufferPin { gate } => write!(f, "buffer '{gate}' has no delay pin"),
            Self::InvalidBufferPhase { gate } => {
                write!(f, "buffer '{gate}' must be inverting or non-inverting")
            }
            Self::InvalidBufferPartition {
                gate,
                expected,
                actual,
            } => write!(
                f,
                "buffer '{gate}' was added to the {expected:?} partition but is {actual:?}"
            ),
            Self::PositiveSourceAfterNegative => {
                write!(
                    f,
                    "positive fanout-delay sources must be added before negative sources"
                )
            }
            Self::GateIndexOutOfRange { index, len } => {
                write!(
                    f,
                    "fanout-delay gate index {index} is out of range for {len} gates"
                )
            }
            Self::ExpectedBuffer { index } => {
                write!(f, "fanout-delay gate index {index} is not a buffer")
            }
            Self::ExpectedSource { index } => {
                write!(f, "fanout-delay gate index {index} is not a source")
            }
            Self::InvalidLoad(load) => write!(f, "fanout-delay load {load} is invalid"),
            Self::PinTimingMismatch {
                source,
                expected,
                actual,
            } => write!(
                f,
                "source '{source}' expected {expected} input arrivals but has {actual} pin timings"
            ),
            Self::SourceForwardArrivalNotZero { source, arrival } => write!(
                f,
                "source '{source}' expected zero pre-load arrival but got ({}, {})",
                arrival.rise, arrival.fall
            ),
            Self::EmptyPwlSource { source, phase } => {
                write!(f, "source '{source}' has an empty {phase} PWL delay")
            }
            Self::InvalidMaxInverterCount(max_n) => {
                write!(f, "maximum inverter count {max_n} is invalid")
            }
            Self::InvalidSearchWindow { from, to, max_n } => write!(
                f,
                "inverter-count search window [{from}, {to}] is invalid for max {max_n}"
            ),
            Self::MissingSisPorts { operation } => write!(f, "{operation} requires unavailable native SIS integration"),
        }
    }
}

impl Error for FanoutDelayError {}

pub fn sis_bound_source_unavailable(operation: &'static str) -> Result<(), FanoutDelayError> {
    Err(FanoutDelayError::MissingSisPorts {
        operation,
    })
}

fn simulate_internal_source(
    source: &InternalSource,
    load: f64,
) -> Result<DelayTime, FanoutDelayError> {
    if source.input_arrivals.len() != source.pins.len() {
        return Err(FanoutDelayError::PinTimingMismatch {
            source: source.name.clone(),
            expected: source.input_arrivals.len(),
            actual: source.pins.len(),
        });
    }

    let mut arrival = MINUS_INFINITY;
    for (pin, input) in source
        .pins
        .iter()
        .copied()
        .zip(source.input_arrivals.iter().copied())
    {
        let delay = pin.delay(load);
        let pin_arrival = match pin.phase {
            TimingPhase::NonInverting => {
                DelayTime::new(input.rise + delay.rise, input.fall + delay.fall)
            }
            TimingPhase::Inverting => {
                DelayTime::new(input.fall + delay.rise, input.rise + delay.fall)
            }
            TimingPhase::Unknown => DelayTime::new(
                input.rise.max(input.fall) + delay.rise,
                input.rise.max(input.fall) + delay.fall,
            ),
        };
        arrival = max_delay_time(arrival, pin_arrival);
    }

    Ok(arrival)
}

fn select_active_pwl<T>(delay: &FanoutDelayPwl<T>, load: f64) -> FanoutDelayPwl<T>
where
    T: Clone,
{
    let rise = delay
        .rise
        .lookup(load)
        .map(|point| {
            PiecewiseLinear::extract(vec![PiecewisePoint::new(
                0.0,
                point.eval(load),
                point.slope,
                None,
            )])
        })
        .unwrap_or_else(PiecewiseLinear::empty);
    let fall = delay
        .fall
        .lookup(load)
        .map(|point| {
            PiecewiseLinear::extract(vec![PiecewisePoint::new(
                0.0,
                point.eval(load),
                point.slope,
                None,
            )])
        })
        .unwrap_or_else(PiecewiseLinear::empty);

    FanoutDelayPwl::new(rise, fall)
}

fn validate_load(load: f64) -> Result<(), FanoutDelayError> {
    if !load.is_finite() || load < 0.0 {
        return Err(FanoutDelayError::InvalidLoad(load));
    }

    Ok(())
}

fn is_valid_delay(delay: DelayTime) -> bool {
    delay.rise.is_finite() && delay.fall.is_finite()
}

fn add_delay(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise + right.rise, left.fall + right.fall)
}

fn sub_delay(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise - right.rise, left.fall - right.fall)
}

fn max_delay_time(left: DelayTime, right: DelayTime) -> DelayTime {
    DelayTime::new(left.rise.max(right.rise), left.fall.max(right.fall))
}

fn delay_time_eq(left: DelayTime, right: DelayTime) -> bool {
    left.rise == right.rise && left.fall == right.fall
}

fn min_component(delay: DelayTime) -> f64 {
    delay.rise.min(delay.fall)
}

fn search_ratio(numerator: f64, denominator: f64, max_n: usize) -> f64 {
    if denominator == 0.0 {
        max_n as f64
    } else {
        (numerator / denominator).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delay(rise: f64, fall: f64) -> DelayTime {
        DelayTime::new(rise, fall)
    }

    fn buffer(name: &str, kind: BufferKind, beta: DelayTime) -> FanoutBuffer {
        FanoutBuffer::new(name, kind, 2.0, 1.0, 4.0, delay(1.0, 2.0), beta).unwrap()
    }

    #[test]
    fn computes_buffer_intrinsic_and_load_delay() {
        let model = FanoutDelayModel::<()>::new(
            vec![buffer("buf", BufferKind::NonInverting, delay(0.2, 0.3))],
            vec![buffer("inv", BufferKind::Inverting, delay(0.4, 0.5))],
            FanoutDelayOptions::default(),
        )
        .unwrap();

        assert_eq!(
            model.backward_intrinsic(delay(5.0, 7.0), 0).unwrap(),
            delay(4.0, 5.0)
        );
        assert_eq!(
            model.forward_intrinsic(delay(3.0, 4.0), 1).unwrap(),
            delay(5.0, 5.0)
        );
        assert_eq!(
            model
                .backward_load_dependent(delay(10.0, 10.0), 0, 5.0)
                .unwrap(),
            delay(9.0, 8.5)
        );
        assert_eq!(
            model
                .forward_load_dependent(delay(1.0, 2.0), 1, 2.0)
                .unwrap(),
            delay(1.8, 3.0)
        );
    }

    #[test]
    fn applies_load_limit_penalty_before_timing() {
        let model = FanoutDelayModel::<()>::new(
            vec![buffer("buf", BufferKind::NonInverting, delay(1.0, 1.0))],
            Vec::new(),
            FanoutDelayOptions {
                check_load_limit: true,
                penalty_factor: 3.0,
                wire_load_per_fanout: 0.0,
            },
        )
        .unwrap();

        assert_eq!(
            model
                .backward_load_dependent(delay(20.0, 20.0), 0, 5.0)
                .unwrap(),
            delay(5.0, 5.0)
        );
    }

    #[test]
    fn computes_primary_input_source_timing() {
        let mut model = FanoutDelayModel::<()>::from_buffers(
            [buffer("buf", BufferKind::NonInverting, delay(1.0, 1.0))],
            FanoutDelayOptions::default(),
        )
        .unwrap();
        let source = FanoutSource::PrimaryInput(
            PrimaryInputSource::new("a", delay(2.0, 3.0), delay(0.5, 0.25), 10.0).unwrap(),
        );
        let source_index = model.add_source(source, FanoutPolarity::Positive).unwrap();

        assert_eq!(
            model
                .forward_load_dependent(ZERO_DELAY, source_index, 4.0)
                .unwrap(),
            delay(4.0, 4.0)
        );
        assert_eq!(
            model
                .backward_load_dependent(delay(10.0, 12.0), source_index, 4.0)
                .unwrap(),
            delay(6.0, 8.0)
        );
    }

    #[test]
    fn computes_internal_source_with_pin_phases() {
        let source = InternalSource::new(
            "n1",
            vec![delay(1.0, 4.0), delay(3.0, 2.0)],
            vec![
                FanoutPinTiming::new(TimingPhase::NonInverting, 1.0, 10.0, 1.0, 0.5, 2.0, 0.25),
                FanoutPinTiming::new(TimingPhase::Inverting, 1.0, 10.0, 2.0, 0.0, 3.0, 0.0),
            ],
            10.0,
        )
        .unwrap();

        assert_eq!(
            simulate_internal_source(&source, 2.0).unwrap(),
            delay(4.0, 6.5)
        );
    }

    #[test]
    fn keeps_positive_sources_before_negative_sources() {
        let mut model = FanoutDelayModel::<()>::from_buffers(
            [buffer("buf", BufferKind::NonInverting, delay(1.0, 1.0))],
            FanoutDelayOptions::default(),
        )
        .unwrap();
        model
            .add_source(
                FanoutSource::PrimaryInput(
                    PrimaryInputSource::new("neg", ZERO_DELAY, ZERO_DELAY, 1.0).unwrap(),
                ),
                FanoutPolarity::Negative,
            )
            .unwrap();

        let err = model
            .add_source(
                FanoutSource::PrimaryInput(
                    PrimaryInputSource::new("pos", ZERO_DELAY, ZERO_DELAY, 1.0).unwrap(),
                ),
                FanoutPolarity::Positive,
            )
            .unwrap_err();

        assert_eq!(err, FanoutDelayError::PositiveSourceAfterNegative);
    }

    #[test]
    fn computes_best_number_of_inverters_by_linear_search_window() {
        let mut model = FanoutDelayModel::<()>::from_buffers(
            [
                FanoutBuffer::new(
                    "source",
                    BufferKind::NonInverting,
                    1.0,
                    1.0,
                    100.0,
                    ZERO_DELAY,
                    delay(0.2, 0.2),
                )
                .unwrap(),
                FanoutBuffer::new(
                    "sink",
                    BufferKind::NonInverting,
                    1.0,
                    2.0,
                    100.0,
                    ZERO_DELAY,
                    delay(0.5, 0.5),
                )
                .unwrap(),
            ],
            FanoutDelayOptions {
                check_load_limit: false,
                penalty_factor: 1.0,
                wire_load_per_fanout: 0.0,
            },
        )
        .unwrap();
        let source = FanoutSource::PrimaryInput(
            PrimaryInputSource::new("a", ZERO_DELAY, delay(0.1, 0.1), 100.0).unwrap(),
        );
        model.add_source(source, FanoutPolarity::Positive).unwrap();

        assert_eq!(
            model
                .compute_best_number_of_inverters(0, 1, 18.0, 8)
                .unwrap(),
            5
        );
        assert_eq!(
            model
                .compute_best_number_of_inverters(2, 1, 18.0, 8)
                .unwrap(),
            7
        );
    }
}
