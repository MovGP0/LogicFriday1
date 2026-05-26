//! Native Rust port of the feasible helper behavior in `sis/speed/buf_util.c`.
//!
//! The C file owns buffer-list construction, default buffering options,
//! implementation names, critical-fanin/slack tests, and a set of direct SIS
//! node/network/library mutations. The pure helper behavior is represented
//! here over owned Rust data. SIS-bound mutation entry points return explicit
//! dependency errors until the native node, network, delay, and mapped-library
//! ports are available.

use std::error::Error;
use std::fmt;

pub const POS_LARGE: f64 = 10_000.0;
pub const V_SMALL: f64 = 0.000001;

pub const REPOWER_MASK: u8 = 1 << 0;
pub const UNBALANCED_MASK: u8 = 1 << 1;
pub const BALANCED_MASK: u8 = 1 << 2;
pub const ALL_TRANSFORMS: u8 = REPOWER_MASK | UNBALANCED_MASK | BALANCED_MASK;

pub const UNIT_FANOUT_BLOCK: DelayTime = DelayTime::new(1.0, 1.0);
pub const UNIT_FANOUT_DRIVE: DelayTime = DelayTime::new(0.2, 0.2);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub fn min_edge(self) -> f64 {
        self.rise.min(self.fall)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinPhase {
    Inverting,
    NonInverting,
    Neither,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    UnitFanout,
    Mapped,
    Library,
    Unit,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferImplKind {
    None,
    Buffer,
    Gate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferOptions {
    pub trace: bool,
    pub threshold: f64,
    pub crit_slack: f64,
    pub single_pass: bool,
    pub do_decomp: bool,
    pub debug: i32,
    pub limit: usize,
    pub mode: u8,
    pub only_check_max_load: bool,
    pub interactive: bool,
}

impl Default for BufferOptions {
    fn default() -> Self {
        Self {
            trace: false,
            threshold: 2.0 * V_SMALL,
            crit_slack: 0.0,
            single_pass: false,
            do_decomp: false,
            debug: 0,
            limit: 2,
            mode: ALL_TRANSFORMS,
            only_check_max_load: false,
            interactive: false,
        }
    }
}

pub fn parse_buffer_options(args: &[&str]) -> Result<BufferOptions, BufUtilError> {
    let mut options = BufferOptions::default();
    let mut index = 0;
    while index < args.len() {
        let arg = args[index];
        if !arg.starts_with('-') || arg == "-" {
            break;
        }

        match arg {
            "-d" => options.do_decomp = true,
            "-c" => options.single_pass = true,
            "-L" => options.only_check_max_load = true,
            "-T" => options.trace = true,
            "-D" => options.debug = 1,
            "-f" => {
                index += 1;
                let mode = parse_next_u8(args, index, "-f")?;
                if !(1..=ALL_TRANSFORMS).contains(&mode) {
                    return Err(BufUtilError::InvalidTransformMode(mode));
                }
                options.mode = mode;
            }
            "-v" => {
                index += 1;
                options.debug = parse_next_i32(args, index, "-v")?;
            }
            "-l" => {
                index += 1;
                options.limit = parse_next_usize(args, index, "-l")?;
            }
            _ => return Err(BufUtilError::UnknownOption(arg.to_string())),
        }
        index += 1;
    }
    Ok(options)
}

fn parse_next_u8(args: &[&str], index: usize, option: &'static str) -> Result<u8, BufUtilError> {
    args.get(index)
        .ok_or(BufUtilError::MissingOptionValue(option))?
        .parse()
        .map_err(|_| BufUtilError::InvalidOptionValue {
            option,
            value: args[index].to_string(),
        })
}

fn parse_next_i32(args: &[&str], index: usize, option: &'static str) -> Result<i32, BufUtilError> {
    args.get(index)
        .ok_or(BufUtilError::MissingOptionValue(option))?
        .parse()
        .map_err(|_| BufUtilError::InvalidOptionValue {
            option,
            value: args[index].to_string(),
        })
}

fn parse_next_usize(
    args: &[&str],
    index: usize,
    option: &'static str,
) -> Result<usize, BufUtilError> {
    args.get(index)
        .ok_or(BufUtilError::MissingOptionValue(option))?
        .parse()
        .map_err(|_| BufUtilError::InvalidOptionValue {
            option,
            value: args[index].to_string(),
        })
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
    pub const fn new(
        block: DelayTime,
        drive: DelayTime,
        phase: PinPhase,
        load: f64,
        max_load: f64,
    ) -> Self {
        Self {
            block,
            drive,
            phase,
            load,
            max_load,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LibraryGate {
    pub name: String,
    pub area: f64,
    pub pin_delay: DelayPin,
}

impl LibraryGate {
    pub fn new(name: impl Into<String>, area: f64, pin_delay: DelayPin) -> Self {
        Self {
            name: name.into(),
            area,
            pin_delay,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BufferLibrary {
    pub inverters: Vec<LibraryGate>,
    pub buffers: Vec<LibraryGate>,
}

impl BufferLibrary {
    pub fn new(inverters: Vec<LibraryGate>, buffers: Vec<LibraryGate>) -> Self {
        Self { inverters, buffers }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedBuffer {
    pub gates: Vec<Option<LibraryGate>>,
    pub area: f64,
    pub ip_load: f64,
    pub max_load: f64,
    pub phase: PinPhase,
    pub block: DelayTime,
    pub drive: DelayTime,
}

impl SpeedBuffer {
    pub fn from_gate(gate: LibraryGate) -> Self {
        Self {
            area: gate.area,
            ip_load: gate.pin_delay.load,
            max_load: gate.pin_delay.max_load,
            phase: gate.pin_delay.phase,
            block: gate.pin_delay.block,
            drive: gate.pin_delay.drive,
            gates: vec![Some(gate)],
        }
    }

    pub fn unit_fanout(phase: PinPhase) -> Self {
        Self {
            gates: vec![None],
            area: 1.0,
            ip_load: 1.0,
            max_load: POS_LARGE,
            phase,
            block: UNIT_FANOUT_BLOCK,
            drive: UNIT_FANOUT_DRIVE,
        }
    }

    pub fn depth(&self) -> usize {
        self.gates.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferCatalog {
    pub options: BufferOptions,
    pub model: DelayModel,
    pub auto_route: f64,
    pub min_req_diff: f64,
    pub inv_load: f64,
    buffers: Vec<SpeedBuffer>,
    num_inv: usize,
}

impl BufferCatalog {
    pub fn buffers(&self) -> &[SpeedBuffer] {
        &self.buffers
    }

    pub fn num_inv(&self) -> usize {
        self.num_inv
    }

    pub fn num_total_buffers(&self) -> usize {
        self.buffers.len()
    }

    pub fn num_non_inverting(&self) -> usize {
        self.buffers.len() - self.num_inv
    }
}

pub fn unit_fanout_catalog(
    options: BufferOptions,
    auto_route: f64,
    model: DelayModel,
) -> BufferCatalog {
    BufferCatalog {
        options,
        model,
        auto_route,
        min_req_diff: 0.0,
        inv_load: 0.0,
        buffers: vec![
            SpeedBuffer::unit_fanout(PinPhase::Inverting),
            SpeedBuffer::unit_fanout(PinPhase::NonInverting),
        ],
        num_inv: 1,
    }
}

pub fn library_buffer_catalog(
    library: Option<&BufferLibrary>,
    options: BufferOptions,
    auto_route: f64,
    use_mapped: bool,
) -> BufferCatalog {
    let model = if use_mapped {
        DelayModel::Mapped
    } else {
        DelayModel::UnitFanout
    };

    let Some(library) = library else {
        return unit_fanout_catalog(options, auto_route, model);
    };
    let Some(first_inverter) = library.inverters.first() else {
        return unit_fanout_catalog(options, auto_route, model);
    };

    let inv_load = options.limit as f64 * first_inverter.pin_delay.load;
    let mut buffers = library
        .inverters
        .iter()
        .cloned()
        .chain(library.buffers.iter().cloned())
        .map(SpeedBuffer::from_gate)
        .collect::<Vec<_>>();

    sort_buffers_like_sis(&mut buffers);
    let num_inv = buffers
        .iter()
        .filter(|buffer| buffer.phase == PinPhase::Inverting)
        .count();
    let min_req_diff = buffers
        .iter()
        .filter(|buffer| buffer.phase == PinPhase::NonInverting)
        .map(|buffer| buffer.block.min_edge())
        .min_by(f64::total_cmp)
        .unwrap_or(0.0);

    BufferCatalog {
        options,
        model,
        auto_route,
        min_req_diff,
        inv_load,
        buffers,
        num_inv,
    }
}

pub fn sort_buffers_like_sis(buffers: &mut [SpeedBuffer]) {
    buffers.sort_by(|left, right| match (left.phase, right.phase) {
        (PinPhase::Inverting, PinPhase::NonInverting) => std::cmp::Ordering::Less,
        (PinPhase::NonInverting, PinPhase::Inverting) => std::cmp::Ordering::Greater,
        _ => right.area.total_cmp(&left.area),
    });
}

pub fn append_gate_to_buffer(
    previous: &SpeedBuffer,
    gate: LibraryGate,
    auto_route: f64,
) -> SpeedBuffer {
    let load = auto_route + gate.pin_delay.load;
    let mut gates = previous.gates.clone();
    gates.push(Some(gate.clone()));

    SpeedBuffer {
        gates,
        phase: PinPhase::NonInverting,
        area: previous.area + gate.area,
        block: DelayTime::new(
            previous.block.fall + previous.drive.fall * load + gate.pin_delay.block.rise,
            previous.block.rise + previous.drive.rise * load + gate.pin_delay.block.fall,
        ),
        drive: gate.pin_delay.drive,
        ip_load: previous.ip_load,
        max_load: gate.pin_delay.max_load,
    }
}

pub fn buffer_name(buffer: Option<&SpeedBuffer>) -> String {
    let Some(buffer) = buffer else {
        return "NONE".to_string();
    };
    if buffer.gates.first().is_some_and(Option::is_none) {
        return "UNIT_FAN".to_string();
    }

    buffer
        .gates
        .iter()
        .filter_map(|gate| gate.as_ref().map(|gate| gate.name.as_str()))
        .collect::<Vec<_>>()
        .join("-")
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeImplementation {
    MissingNode,
    PrimaryInput,
    None,
    Buffer(SpeedBuffer),
    Gate(LibraryGate),
}

pub fn implementation_name(implementation: &NodeImplementation) -> String {
    match implementation {
        NodeImplementation::MissingNode => "--NONE--".to_string(),
        NodeImplementation::PrimaryInput => "NODE_PI".to_string(),
        NodeImplementation::Buffer(buffer) => buffer_name(Some(buffer)),
        NodeImplementation::Gate(gate) => gate.name.clone(),
        NodeImplementation::None => "-NONE-".to_string(),
    }
}

pub fn dump_buffer_list(catalog: &BufferCatalog) -> String {
    let mut output =
        "type        name       area ipcap  blck_r blck_f drve_r drve_f rise   fall\n".to_string();
    output.push_str("--------------------------------------------------------------------------\n");
    for buffer in catalog.buffers() {
        let rise = buffer.block.rise + catalog.inv_load * buffer.drive.rise;
        let fall = buffer.block.fall + catalog.inv_load * buffer.drive.fall;
        let kind = if buffer.phase == PinPhase::Inverting {
            "INV"
        } else {
            "BUF"
        };
        output.push_str(&format!(
            "{kind}    {:<14} {:>5.1} {:<6.3} {:<6.3} {:<6.3} {:<6.3} {:<6.3} {:<6.3} {:<6.3}\n",
            buffer_name(Some(buffer)),
            buffer.area,
            buffer.ip_load,
            buffer.block.rise,
            buffer.block.fall,
            buffer.drive.rise,
            buffer.drive.fall,
            rise,
            fall
        ));
    }
    output
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaninTiming {
    pub arrival: DelayTime,
    pub required: DelayTime,
}

pub fn critical_fanin_index(fanins: &[FaninTiming]) -> Result<usize, BufUtilError> {
    if fanins.is_empty() {
        return Err(BufUtilError::NoFanins);
    }
    if fanins.len() == 1 {
        return Ok(0);
    }

    fanins
        .iter()
        .enumerate()
        .map(|(index, fanin)| {
            let slack = DelayTime::new(
                fanin.required.rise - fanin.arrival.rise,
                fanin.required.fall - fanin.arrival.fall,
            );
            (index, slack.min_edge())
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(index, _)| index)
        .ok_or(BufUtilError::NoFanins)
}

pub fn critical_slack_threshold(output_slacks: &[DelayTime], threshold: f64) -> f64 {
    let min_slack = output_slacks
        .iter()
        .map(|time| time.min_edge())
        .fold(POS_LARGE, f64::min);

    if min_slack > 0.0 {
        -1.0
    } else {
        min_slack + threshold
    }
}

pub fn is_critical(slack: DelayTime, crit_slack: f64) -> bool {
    slack.rise <= crit_slack || slack.fall <= crit_slack
}

pub fn gate_version(gates: &[LibraryGate], version: usize) -> Option<&LibraryGate> {
    gates.get(version)
}

pub fn subtract_delay(
    phase: PinPhase,
    block: DelayTime,
    drive: DelayTime,
    load: f64,
    req: DelayTime,
) -> DelayTime {
    let delay = DelayTime::new(
        block.rise + drive.rise * load,
        block.fall + drive.fall * load,
    );
    compute_required_at_input(phase, req, delay)
}

pub fn compute_required_at_input(phase: PinPhase, req: DelayTime, delay: DelayTime) -> DelayTime {
    let mut input = DelayTime::new(f64::INFINITY, f64::INFINITY);
    if matches!(phase, PinPhase::Inverting | PinPhase::Neither) {
        input.rise = input.rise.min(req.fall - delay.fall);
        input.fall = input.fall.min(req.rise - delay.rise);
    }
    if matches!(phase, PinPhase::NonInverting | PinPhase::Neither) {
        input.rise = input.rise.min(req.rise - delay.rise);
        input.fall = input.fall.min(req.fall - delay.fall);
    }
    input
}

#[derive(Clone, Debug, PartialEq)]
pub struct GateVersion {
    pub name: String,
    pub pins: Vec<DelayPin>,
}

pub fn failed_slack_test(
    original_fanins: &[FaninTiming],
    root_gate: &GateVersion,
    op_req: DelayTime,
    op_load: f64,
) -> Result<bool, BufUtilError> {
    if original_fanins.len() <= 1 {
        return Ok(false);
    }
    if root_gate.pins.len() < original_fanins.len() {
        return Err(BufUtilError::MissingGatePin {
            gate: root_gate.name.clone(),
            pin: root_gate.pins.len(),
        });
    }

    let mut fom_best = POS_LARGE;
    let mut new_fom_best = POS_LARGE;

    for (index, fanin) in original_fanins.iter().enumerate() {
        let slack = DelayTime::new(
            fanin.required.rise - fanin.arrival.rise,
            fanin.required.fall - fanin.arrival.fall,
        );
        fom_best = fom_best.min(slack.min_edge());

        let pin = root_gate.pins[index];
        let new_req = subtract_delay(pin.phase, pin.block, pin.drive, op_load, op_req);
        let new_slack = DelayTime::new(
            new_req.rise - fanin.arrival.rise,
            new_req.fall - fanin.arrival.fall,
        );
        new_fom_best = new_fom_best.min(new_slack.min_edge());
    }

    Ok(new_fom_best < fom_best)
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferNodeData {
    pub kind: BufferImplKind,
    pub cfi: isize,
    pub load: f64,
    pub req_time: DelayTime,
    pub prev_drive: DelayTime,
    pub prev_phase: PinPhase,
}

impl Default for BufferNodeData {
    fn default() -> Self {
        Self {
            kind: BufferImplKind::None,
            cfi: -1,
            load: 0.0,
            req_time: DelayTime::new(0.0, 0.0),
            prev_drive: DelayTime::new(0.0, 0.0),
            prev_phase: PinPhase::Unknown,
        }
    }
}

pub fn required_time_at_input(node: &BufferNodeData) -> DelayTime {
    node.req_time
}

pub fn set_required_time_at_input(node: &mut BufferNodeData, req: DelayTime) {
    node.req_time = req;
}

pub fn set_prev_drive(node: &mut BufferNodeData, drive: DelayTime) {
    node.prev_drive = drive;
}

pub fn set_prev_phase(node: &mut BufferNodeData, phase: PinPhase) {
    node.prev_phase = phase;
}

pub fn annotate_gate_from_sis_node() -> Result<(), BufUtilError> {
    missing_sis_ports("sp_buf_annotate_gate")
}

pub fn replace_lib_gate_in_sis_node() -> Result<(), BufUtilError> {
    missing_sis_ports("sp_replace_lib_gate")
}

pub fn implement_buffer_chain_in_sis_network() -> Result<(), BufUtilError> {
    missing_sis_ports("sp_implement_buffer_chain")
}

pub fn add_gate_implementation_to_sis_node() -> Result<(), BufUtilError> {
    missing_sis_ports("buf_add_implementation")
}

pub fn init_top_down_from_sis_network() -> Result<(), BufUtilError> {
    missing_sis_ports("buf_init_top_down")
}

pub fn map_interface_with_sis_network() -> Result<(), BufUtilError> {
    missing_sis_ports("buf_map_interface")
}

fn missing_sis_ports(operation: &'static str) -> Result<(), BufUtilError> {
    Err(BufUtilError::MissingSisPorts { operation })
}

#[derive(Clone, Debug, PartialEq)]
pub enum BufUtilError {
    UnknownOption(String),
    MissingOptionValue(&'static str),
    InvalidOptionValue { option: &'static str, value: String },
    InvalidTransformMode(u8),
    NoFanins,
    MissingGatePin { gate: String, pin: usize },
    MissingSisPorts { operation: &'static str },
}

impl fmt::Display for BufUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOption(option) => write!(f, "buffering: unknown option {option}"),
            Self::MissingOptionValue(option) => {
                write!(f, "buffering option {option} requires a value")
            }
            Self::InvalidOptionValue { option, value } => {
                write!(f, "buffering option {option} has invalid value {value}")
            }
            Self::InvalidTransformMode(mode) => {
                write!(f, "valid range of -f option is 1 to 7, got {mode}")
            }
            Self::NoFanins => write!(f, "critical fanin requires at least one fanin"),
            Self::MissingGatePin { gate, pin } => {
                write!(f, "gate {gate} has no delay pin at index {pin}")
            }
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} is blocked by unported SIS dependencies")
            }
        }
    }
}

impl Error for BufUtilError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(phase: PinPhase, block: (f64, f64), drive: (f64, f64), load: f64) -> DelayPin {
        DelayPin::new(
            DelayTime::new(block.0, block.1),
            DelayTime::new(drive.0, drive.1),
            phase,
            load,
            POS_LARGE,
        )
    }

    fn gate(name: &str, area: f64, phase: PinPhase, load: f64, block: (f64, f64)) -> LibraryGate {
        LibraryGate::new(name, area, pin(phase, block, (0.1, 0.2), load))
    }

    #[test]
    fn default_and_parsed_options_match_buffer_fill_options() {
        let defaults = BufferOptions::default();
        assert!(!defaults.trace);
        assert!(!defaults.single_pass);
        assert!(!defaults.do_decomp);
        assert_eq!(defaults.debug, 0);
        assert_eq!(defaults.limit, 2);
        assert_eq!(defaults.mode, 7);
        assert_eq!(defaults.threshold, 2.0 * V_SMALL);

        let parsed =
            parse_buffer_options(&["-d", "-c", "-L", "-T", "-f", "3", "-v", "12", "-l", "5"])
                .unwrap();
        assert!(parsed.do_decomp);
        assert!(parsed.single_pass);
        assert!(parsed.only_check_max_load);
        assert!(parsed.trace);
        assert_eq!(parsed.mode, 3);
        assert_eq!(parsed.debug, 12);
        assert_eq!(parsed.limit, 5);

        assert_eq!(
            parse_buffer_options(&["-f", "8"]),
            Err(BufUtilError::InvalidTransformMode(8))
        );
    }

    #[test]
    fn library_catalog_sorts_inverters_first_and_smallest_last() {
        let library = BufferLibrary::new(
            vec![
                gate("inv_small", 1.0, PinPhase::Inverting, 0.2, (3.0, 4.0)),
                gate("inv_big", 5.0, PinPhase::Inverting, 0.4, (1.0, 2.0)),
            ],
            vec![
                gate("buf_small", 2.0, PinPhase::NonInverting, 0.3, (0.5, 0.8)),
                gate("buf_big", 6.0, PinPhase::NonInverting, 0.6, (0.7, 0.9)),
            ],
        );

        let catalog = library_buffer_catalog(Some(&library), BufferOptions::default(), 0.0, true);
        let names = catalog
            .buffers()
            .iter()
            .map(|buffer| buffer_name(Some(buffer)))
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["inv_big", "inv_small", "buf_big", "buf_small"]);
        assert_eq!(catalog.model, DelayModel::Mapped);
        assert_eq!(catalog.num_inv(), 2);
        assert_eq!(catalog.num_total_buffers(), 4);
        assert_eq!(catalog.min_req_diff, 0.5);
        assert_eq!(catalog.inv_load, 0.4);
    }

    #[test]
    fn missing_library_or_inverter_uses_unit_fanout_pair() {
        let catalog = library_buffer_catalog(None, BufferOptions::default(), 0.0, false);
        assert_eq!(catalog.model, DelayModel::UnitFanout);
        assert_eq!(catalog.num_inv(), 1);
        assert_eq!(catalog.num_non_inverting(), 1);
        assert_eq!(buffer_name(catalog.buffers().first()), "UNIT_FAN");
        assert_eq!(catalog.buffers()[0].phase, PinPhase::Inverting);
        assert_eq!(catalog.buffers()[1].phase, PinPhase::NonInverting);
    }

    #[test]
    fn chained_inverter_buffer_uses_cross_edge_block_formula() {
        let previous =
            SpeedBuffer::from_gate(gate("inv1", 2.0, PinPhase::Inverting, 0.5, (1.0, 2.0)));
        let next = gate("inv2", 3.0, PinPhase::Inverting, 0.25, (4.0, 5.0));

        let chained = append_gate_to_buffer(&previous, next, 0.75);

        assert_eq!(buffer_name(Some(&chained)), "inv1-inv2");
        assert_eq!(chained.depth(), 2);
        assert_eq!(chained.phase, PinPhase::NonInverting);
        assert_eq!(chained.area, 5.0);
        assert_eq!(chained.block, DelayTime::new(6.2, 6.1));
        assert_eq!(chained.ip_load, 0.5);
    }

    #[test]
    fn implementation_names_match_c_debug_names() {
        let inv = SpeedBuffer::from_gate(gate("inv", 1.0, PinPhase::Inverting, 0.2, (1.0, 1.0)));

        assert_eq!(
            implementation_name(&NodeImplementation::MissingNode),
            "--NONE--"
        );
        assert_eq!(
            implementation_name(&NodeImplementation::PrimaryInput),
            "NODE_PI"
        );
        assert_eq!(implementation_name(&NodeImplementation::None), "-NONE-");
        assert_eq!(
            implementation_name(&NodeImplementation::Buffer(inv.clone())),
            "inv"
        );
        assert_eq!(
            implementation_name(&NodeImplementation::Gate(gate(
                "nand",
                3.0,
                PinPhase::NonInverting,
                0.4,
                (1.0, 1.0)
            ))),
            "nand"
        );
        assert_eq!(buffer_name(None), "NONE");
    }

    #[test]
    fn critical_fanin_and_threshold_helpers_match_c_rules() {
        let fanins = [
            FaninTiming {
                arrival: DelayTime::new(3.0, 1.0),
                required: DelayTime::new(5.0, 4.0),
            },
            FaninTiming {
                arrival: DelayTime::new(4.0, 3.0),
                required: DelayTime::new(4.5, 8.0),
            },
        ];

        assert_eq!(critical_fanin_index(&fanins), Ok(1));
        assert_eq!(
            critical_slack_threshold(&[DelayTime::new(0.2, 0.4), DelayTime::new(1.0, 2.0)], 0.5),
            -1.0
        );
        assert_eq!(
            critical_slack_threshold(&[DelayTime::new(-1.0, 0.4), DelayTime::new(1.0, 2.0)], 0.5),
            -0.5
        );
        assert!(is_critical(DelayTime::new(0.2, -0.6), -0.5));
        assert!(!is_critical(DelayTime::new(0.2, 0.6), -0.5));
    }

    #[test]
    fn required_time_subtraction_handles_inverting_and_neither_phase() {
        assert_eq!(
            subtract_delay(
                PinPhase::Inverting,
                DelayTime::new(1.0, 2.0),
                DelayTime::new(0.5, 0.25),
                4.0,
                DelayTime::new(20.0, 30.0)
            ),
            DelayTime::new(27.0, 17.0)
        );
        assert_eq!(
            compute_required_at_input(
                PinPhase::Neither,
                DelayTime::new(10.0, 12.0),
                DelayTime::new(3.0, 5.0)
            ),
            DelayTime::new(7.0, 7.0)
        );
    }

    #[test]
    fn failed_slack_test_detects_worsened_noncritical_input() {
        let fanins = [
            FaninTiming {
                arrival: DelayTime::new(1.0, 1.0),
                required: DelayTime::new(5.0, 6.0),
            },
            FaninTiming {
                arrival: DelayTime::new(2.0, 2.0),
                required: DelayTime::new(8.0, 8.0),
            },
        ];
        let root = GateVersion {
            name: "root_v2".to_string(),
            pins: vec![
                pin(PinPhase::NonInverting, (1.0, 1.0), (0.0, 0.0), 0.0),
                pin(PinPhase::NonInverting, (7.0, 7.0), (0.0, 0.0), 0.0),
            ],
        };

        assert_eq!(
            failed_slack_test(&fanins, &root, DelayTime::new(10.0, 10.0), 0.0),
            Ok(true)
        );
    }

    #[test]
    fn sis_bound_entry_points_report_dependency_beads() {
        assert_eq!(
            implement_buffer_chain_in_sis_network(),
            Err(BufUtilError::MissingSisPorts {
                operation: "sp_implement_buffer_chain",
            })
        );
        assert_eq!(
            map_interface_with_sis_network(),
            Err(BufUtilError::MissingSisPorts {
                operation: "buf_map_interface",
            })
        );
    }
}
