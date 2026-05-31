use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub const DELAY_NOT_SET: f64 = -1.0e30;
pub const DELAY_VALUE_NOT_GIVEN: f64 = DELAY_NOT_SET;
pub const DELAY_NOT_SET_STRING: &str = "----";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime {
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime {
    pub const fn new(rise: f64, fall: f64) -> Self {
        Self { rise, fall }
    }

    pub const fn scalar(value: f64) -> Self {
        Self::new(value, value)
    }

    pub const fn not_set() -> Self {
        Self::scalar(DELAY_NOT_SET)
    }

    pub fn max_edge(self) -> f64 {
        self.rise.max(self.fall)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayNode {
    pub name: String,
    pub kind: DelayNodeKind,
    pub arrival: DelayTime,
    pub required: DelayTime,
    pub slack: DelayTime,
    pub load: f64,
    pub drive: DelayTime,
    pub max_input_load: f64,
    pub fanin: Option<usize>,
    pub fanin_fanout_count: usize,
}

impl DelayNode {
    pub fn new(name: impl Into<String>, kind: DelayNodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            arrival: DelayTime::not_set(),
            required: DelayTime::not_set(),
            slack: DelayTime::not_set(),
            load: DELAY_NOT_SET,
            drive: DelayTime::not_set(),
            max_input_load: DELAY_NOT_SET,
            fanin: None,
            fanin_fanout_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DelayDefaultParameter {
    ArrivalRise,
    ArrivalFall,
    DriveRise,
    DriveFall,
    MaxInputLoad,
    OutputLoad,
    RequiredRise,
    RequiredFall,
    WireLoadSlope,
    AddWireLoad,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WireLoad {
    pub name: String,
    pub slope: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayNetwork {
    pub mapped: bool,
    pub nodes: Vec<DelayNode>,
    pub defaults: BTreeMap<DelayDefaultParameter, f64>,
    pub wire_loads: Vec<WireLoad>,
}

impl DelayNetwork {
    pub fn new() -> Self {
        Self {
            mapped: false,
            nodes: Vec::new(),
            defaults: BTreeMap::new(),
            wire_loads: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: DelayNode) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        index
    }

    pub fn node_index(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|node| node.name == name)
    }

    pub fn selected_node_indices(&self, names: &[String], _true_nodes_only: bool) -> Vec<usize> {
        if names.is_empty() {
            return (0..self.nodes.len()).collect();
        }

        names
            .iter()
            .filter_map(|name| self.node_index(name))
            .collect()
    }

    pub fn default_parameter(&self, parameter: DelayDefaultParameter) -> f64 {
        *self.defaults.get(&parameter).unwrap_or(&DELAY_NOT_SET)
    }

    pub fn set_default_parameter(&mut self, parameter: DelayDefaultParameter, value: f64) {
        self.defaults.insert(parameter, value);
    }
}

impl Default for DelayNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Library,
    Mapped,
    UnitFanout,
    Unit,
    Tdc,
}

impl DelayModel {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "library" => Some(Self::Library),
            "mapped" => Some(Self::Mapped),
            "unit-fanout" | "unit_fanout" => Some(Self::UnitFanout),
            "unit" => Some(Self::Unit),
            "tdc" => Some(Self::Tdc),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelaySort {
    Arrival,
    Required,
    Slack,
    Load,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintDelayOptions {
    pub print_arrival: bool,
    pub print_required: bool,
    pub print_slack: bool,
    pub print_load: bool,
    pub print_max: usize,
    pub sort: Option<DelaySort>,
    pub model: DelayModel,
    pub model_filename: Option<String>,
    pub node_names: Vec<String>,
}

impl Default for PrintDelayOptions {
    fn default() -> Self {
        Self {
            print_arrival: false,
            print_required: false,
            print_slack: false,
            print_load: false,
            print_max: usize::MAX,
            sort: None,
            model: DelayModel::Library,
            model_filename: None,
            node_names: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeDelayParameter {
    Arrival,
    Drive,
    MaxInputLoad,
    Load,
    Required,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayDefaultPair {
    Arrival,
    Drive,
    MaxInputLoad,
    OutputLoad,
    Required,
    WireLoadSlope,
    AddWireLoad,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetDelayOptions {
    pub node_parameter: Option<(NodeDelayParameter, f64)>,
    pub default_parameters: Vec<(DelayDefaultPair, f64)>,
    pub node_names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayCommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl DelayCommandOutput {
    fn ok(stdout: String) -> Self {
        Self {
            status: 0,
            stdout,
            stderr: String::new(),
        }
    }

    fn err(stderr: impl Into<String>) -> Self {
        Self {
            status: 1,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DelayCommandError {
    MissingOptionValue(String),
    UnknownOption(String),
    UnknownDelayModel(String),
    InvalidNumber(String),
    TooManyNodeParameters,
}

impl fmt::Display for DelayCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue(option) => write!(f, "option {option} requires a value"),
            Self::UnknownOption(option) => write!(f, "unknown option {option}"),
            Self::UnknownDelayModel(model) => write!(f, "unknown delay model {model}"),
            Self::InvalidNumber(value) => write!(f, "invalid number {value}"),
            Self::TooManyNodeParameters => write!(f, "specify at most one node delay parameter"),
        }
    }
}

impl Error for DelayCommandError {}

pub fn parse_print_delay_options<I, S>(args: I) -> Result<PrintDelayOptions, DelayCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut options = PrintDelayOptions::default();
    let mut args = args.into_iter().map(Into::into).peekable();

    let _ = args.next();
    while let Some(arg) = args.next() {
        if !arg.starts_with('-') || arg == "-" {
            options.node_names.push(arg);
            options.node_names.extend(args);
            break;
        }

        if arg == "--" {
            options.node_names.extend(args);
            break;
        }

        let mut chars = arg[1..].chars().peekable();
        while let Some(option) = chars.next() {
            match option {
                'a' => {
                    options.print_arrival = true;
                    options.sort.get_or_insert(DelaySort::Arrival);
                }
                'l' => {
                    options.print_load = true;
                    options.sort.get_or_insert(DelaySort::Load);
                }
                'r' => {
                    options.print_required = true;
                    options.sort.get_or_insert(DelaySort::Required);
                }
                's' => {
                    options.print_slack = true;
                    options.sort.get_or_insert(DelaySort::Slack);
                }
                'f' | 'm' | 'p' => {
                    let value = option_value(option, &mut chars, &mut args)?;
                    match option {
                        'f' => options.model_filename = Some(value),
                        'm' => {
                            options.model = DelayModel::parse(&value)
                                .ok_or(DelayCommandError::UnknownDelayModel(value))?;
                        }
                        'p' => options.print_max = parse_non_negative_count(&value)?,
                        _ => unreachable!("value option was filtered"),
                    }
                    break;
                }
                _ => return Err(DelayCommandError::UnknownOption(format!("-{option}"))),
            }
        }
    }

    Ok(options)
}

pub fn parse_set_delay_options<I, S>(args: I) -> Result<SetDelayOptions, DelayCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut node_parameter = None;
    let mut default_parameters = Vec::new();
    let mut node_names = Vec::new();
    let mut args = args.into_iter().map(Into::into).peekable();

    let _ = args.next();
    while let Some(arg) = args.next() {
        if !arg.starts_with('-') || arg == "-" {
            node_names.push(arg);
            node_names.extend(args);
            break;
        }

        if arg == "--" {
            node_names.extend(args);
            break;
        }

        let mut chars = arg[1..].chars().peekable();
        while let Some(option) = chars.next() {
            let value = option_value(option, &mut chars, &mut args)?;
            let value = parse_delay_value(&value)?;

            match option {
                'a' => set_node_parameter(&mut node_parameter, NodeDelayParameter::Arrival, value)?,
                'd' => set_node_parameter(&mut node_parameter, NodeDelayParameter::Drive, value)?,
                'i' => set_node_parameter(
                    &mut node_parameter,
                    NodeDelayParameter::MaxInputLoad,
                    value,
                )?,
                'l' => set_node_parameter(&mut node_parameter, NodeDelayParameter::Load, value)?,
                'r' => {
                    set_node_parameter(&mut node_parameter, NodeDelayParameter::Required, value)?
                }
                'A' => default_parameters.push((DelayDefaultPair::Arrival, value)),
                'D' => default_parameters.push((DelayDefaultPair::Drive, value)),
                'I' => default_parameters.push((DelayDefaultPair::MaxInputLoad, value)),
                'L' => default_parameters.push((DelayDefaultPair::OutputLoad, value)),
                'R' => default_parameters.push((DelayDefaultPair::Required, value)),
                'S' => default_parameters.push((DelayDefaultPair::WireLoadSlope, value)),
                'W' => default_parameters.push((DelayDefaultPair::AddWireLoad, value)),
                '?' => return Err(DelayCommandError::UnknownOption("-?".to_string())),
                _ => return Err(DelayCommandError::UnknownOption(format!("-{option}"))),
            }

            break;
        }
    }

    Ok(SetDelayOptions {
        node_parameter,
        default_parameters,
        node_names,
    })
}

pub fn print_delay(network: &DelayNetwork, options: &PrintDelayOptions) -> DelayCommandOutput {
    match model_trace_message(network, options.model) {
        Ok(message) => {
            let mut stdout = String::new();
            stdout.push_str(message);

            let mut indices = network.selected_node_indices(&options.node_names, true);
            if indices.is_empty() {
                return DelayCommandOutput::err(print_delay_usage());
            }

            indices.sort_unstable();
            indices.dedup();

            if let Some(sort) = options.sort {
                sort_delay_indices(network, &mut indices, sort);
            }

            let some_flag = options.print_arrival
                || options.print_required
                || options.print_slack
                || options.print_load;
            let mut printed = 0usize;

            for index in indices {
                if printed >= options.print_max {
                    break;
                }

                let node = &network.nodes[index];
                if options.node_names.is_empty()
                    && node.kind == DelayNodeKind::PrimaryOutput
                    && node
                        .fanin
                        .and_then(|fanin| network.nodes.get(fanin))
                        .is_some_and(|fanin| {
                            fanin.kind == DelayNodeKind::Internal && node.fanin_fanout_count == 1
                        })
                {
                    continue;
                }

                printed += 1;
                stdout.push_str(&format!("{:<10}: ", node.name));

                if !some_flag || options.print_arrival {
                    stdout.push_str(&format_delay_time("arrival", node.arrival, "-INF  -INF"));
                }
                if !some_flag || options.print_required {
                    stdout.push_str(&format_delay_time("required", node.required, "0 0"));
                }
                if !some_flag || options.print_slack {
                    stdout.push_str(&format_slack_time(node.slack));
                }
                if options.print_load {
                    stdout.push_str(&format!("load={:6.3}", node.load));
                }
                stdout.push('\n');
            }

            DelayCommandOutput::ok(stdout)
        }
        Err(message) => DelayCommandOutput::err(message),
    }
}

pub fn print_delay_constraints(
    network: &DelayNetwork,
    node_names: &[String],
) -> DelayCommandOutput {
    let mut stdout = String::new();

    stdout.push_str(&format!(
        "\t\tA setting of {DELAY_NOT_SET_STRING} means value not specified\n"
    ));
    stdout.push_str("Default settings:\n\t\tinput arrival=( ");
    stdout.push_str(&format_default(network, DelayDefaultParameter::ArrivalRise));
    stdout.push_str(&format_default(network, DelayDefaultParameter::ArrivalFall));
    stdout.push_str(")\n\t\tinput drive=( ");
    stdout.push_str(&format_default(network, DelayDefaultParameter::DriveRise));
    stdout.push_str(&format_default(network, DelayDefaultParameter::DriveFall));
    stdout.push_str(")\n\t\tmax input load=");
    stdout.push_str(&format_default(
        network,
        DelayDefaultParameter::MaxInputLoad,
    ));
    stdout.push_str("\n\t\toutput load=");
    stdout.push_str(&format_default(network, DelayDefaultParameter::OutputLoad));
    stdout.push_str("\n\t\toutput required=( ");
    stdout.push_str(&format_default(
        network,
        DelayDefaultParameter::RequiredRise,
    ));
    stdout.push_str(&format_default(
        network,
        DelayDefaultParameter::RequiredFall,
    ));
    stdout.push_str(")\n");

    for wire_load in &network.wire_loads {
        stdout.push_str(&format!(
            "\t\twire load {} slope={:.2}\n",
            wire_load.name, wire_load.slope
        ));
    }

    for index in network.selected_node_indices(node_names, false) {
        let node = &network.nodes[index];
        match node.kind {
            DelayNodeKind::PrimaryInput => {
                stdout.push_str(&format!(
                    "Settings for input {}:\tarrival=( {}{})\tdrive=( {}{})\tmax input load={}\n",
                    node.name,
                    format_value(node.arrival.rise),
                    format_value(node.arrival.fall),
                    format_value(node.drive.rise),
                    format_value(node.drive.fall),
                    format_value(node.max_input_load),
                ));
            }
            DelayNodeKind::PrimaryOutput => {
                stdout.push_str(&format!(
                    "Settings for output {}:\tload={}\trequired=( {}{})\n",
                    node.name,
                    format_value(node.load),
                    format_value(node.required.rise),
                    format_value(node.required.fall),
                ));
            }
            DelayNodeKind::Internal => {}
        }
    }

    DelayCommandOutput::ok(stdout)
}

pub fn set_delay(
    network: &mut DelayNetwork,
    options: &SetDelayOptions,
) -> Result<(), DelayCommandError> {
    for (parameter, value) in &options.default_parameters {
        set_default_pair(network, *parameter, *value);
    }

    let Some((parameter, value)) = options.node_parameter else {
        return Ok(());
    };

    let indices = network.selected_node_indices(&options.node_names, true);
    if indices.is_empty() {
        return Err(DelayCommandError::UnknownOption(set_delay_usage()));
    }

    for index in &indices {
        let node = &network.nodes[*index];
        if !node_parameter_accepts_kind(parameter, node.kind) {
            return Err(DelayCommandError::UnknownOption(set_delay_usage()));
        }
    }

    for index in indices {
        let node = &mut network.nodes[index];
        match parameter {
            NodeDelayParameter::Arrival => node.arrival = DelayTime::scalar(value),
            NodeDelayParameter::Drive => node.drive = DelayTime::scalar(value),
            NodeDelayParameter::MaxInputLoad => node.max_input_load = value,
            NodeDelayParameter::Load => node.load = value,
            NodeDelayParameter::Required => node.required = DelayTime::scalar(value),
        }
    }

    Ok(())
}

pub fn init_delay_plan() -> Vec<&'static str> {
    vec!["print_delay", "set_delay", "constraints"]
}

pub fn end_delay_plan() -> Vec<&'static str> {
    Vec::new()
}

fn sort_delay_indices(network: &DelayNetwork, indices: &mut [usize], sort: DelaySort) {
    indices.sort_by(|left, right| {
        let left_node = &network.nodes[*left];
        let right_node = &network.nodes[*right];
        let diff = match sort {
            DelaySort::Arrival => left_node.arrival.max_edge() - right_node.arrival.max_edge(),
            DelaySort::Required => right_node.required.max_edge() - left_node.required.max_edge(),
            DelaySort::Slack => right_node.slack.max_edge() - left_node.slack.max_edge(),
            DelaySort::Load => left_node.load - right_node.load,
        };

        if diff > 0.0 {
            Ordering::Less
        } else if diff < 0.0 {
            Ordering::Greater
        } else {
            left_node.name.cmp(&right_node.name)
        }
    });
}

fn model_trace_message(
    network: &DelayNetwork,
    model: DelayModel,
) -> Result<&'static str, &'static str> {
    match model {
        DelayModel::Library => {
            if network.mapped {
                Ok(" ... using library delay model\n")
            } else {
                Err("network not mapped, cannot use library delay model\n")
            }
        }
        DelayModel::Mapped => {
            if network.mapped {
                Ok("...network mapped, using library delay model\n")
            } else {
                Ok("...network not mapped, using mapped delay model\n")
            }
        }
        DelayModel::UnitFanout => Ok(" ... using unit delay model (with fanout)\n"),
        DelayModel::Unit => Ok(" ... using unit delay model (levels)\n"),
        DelayModel::Tdc => Ok(" ... using tdc delay model with fanout\n"),
    }
}

fn format_delay_time(label: &str, time: DelayTime, negative_infinite: &str) -> String {
    if time.rise < -(f64::INFINITY / 2.0) || time.fall < -(f64::INFINITY / 2.0) {
        format!("{label}=( {negative_infinite}) ")
    } else {
        format!("{label}=({:5.2} {:5.2}) ", time.rise, time.fall)
    }
}

fn format_slack_time(slack: DelayTime) -> String {
    if slack.rise > f64::INFINITY / 2.0 || slack.fall > f64::INFINITY / 2.0 {
        "slack=(  INF   INF) ".to_string()
    } else {
        format!("slack=({:5.2} {:5.2}) ", slack.rise, slack.fall)
    }
}

fn format_default(network: &DelayNetwork, parameter: DelayDefaultParameter) -> String {
    format_value(network.default_parameter(parameter))
}

fn format_value(value: f64) -> String {
    if value == DELAY_NOT_SET {
        DELAY_NOT_SET_STRING.to_string()
    } else {
        format!("{value:5.2} ")
    }
}

fn option_value<I>(
    option: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    args: &mut std::iter::Peekable<I>,
) -> Result<String, DelayCommandError>
where
    I: Iterator<Item = String>,
{
    let inline: String = chars.collect();
    if !inline.is_empty() {
        return Ok(inline);
    }

    args.next()
        .ok_or_else(|| DelayCommandError::MissingOptionValue(format!("-{option}")))
}

fn parse_non_negative_count(value: &str) -> Result<usize, DelayCommandError> {
    let count = value
        .parse::<isize>()
        .map_err(|_| DelayCommandError::InvalidNumber(value.to_string()))?;

    Ok(count.max(0) as usize)
}

fn parse_delay_value(value: &str) -> Result<f64, DelayCommandError> {
    let value = value
        .parse::<f64>()
        .map_err(|_| DelayCommandError::InvalidNumber(value.to_string()))?;

    if value < 0.0 {
        Ok(DELAY_VALUE_NOT_GIVEN)
    } else {
        Ok(value)
    }
}

fn set_node_parameter(
    target: &mut Option<(NodeDelayParameter, f64)>,
    parameter: NodeDelayParameter,
    value: f64,
) -> Result<(), DelayCommandError> {
    if target.is_some() {
        return Err(DelayCommandError::TooManyNodeParameters);
    }

    *target = Some((parameter, value));
    Ok(())
}

fn set_default_pair(network: &mut DelayNetwork, parameter: DelayDefaultPair, value: f64) {
    match parameter {
        DelayDefaultPair::Arrival => {
            network.set_default_parameter(DelayDefaultParameter::ArrivalRise, value);
            network.set_default_parameter(DelayDefaultParameter::ArrivalFall, value);
        }
        DelayDefaultPair::Drive => {
            network.set_default_parameter(DelayDefaultParameter::DriveRise, value);
            network.set_default_parameter(DelayDefaultParameter::DriveFall, value);
        }
        DelayDefaultPair::MaxInputLoad => {
            network.set_default_parameter(DelayDefaultParameter::MaxInputLoad, value);
        }
        DelayDefaultPair::OutputLoad => {
            network.set_default_parameter(DelayDefaultParameter::OutputLoad, value);
        }
        DelayDefaultPair::Required => {
            network.set_default_parameter(DelayDefaultParameter::RequiredRise, value);
            network.set_default_parameter(DelayDefaultParameter::RequiredFall, value);
        }
        DelayDefaultPair::WireLoadSlope => {
            network.set_default_parameter(DelayDefaultParameter::WireLoadSlope, value);
        }
        DelayDefaultPair::AddWireLoad => {
            network.set_default_parameter(DelayDefaultParameter::AddWireLoad, value);
        }
    }
}

fn node_parameter_accepts_kind(parameter: NodeDelayParameter, kind: DelayNodeKind) -> bool {
    match parameter {
        NodeDelayParameter::Arrival
        | NodeDelayParameter::Drive
        | NodeDelayParameter::MaxInputLoad => kind == DelayNodeKind::PrimaryInput,
        NodeDelayParameter::Load | NodeDelayParameter::Required => {
            kind == DelayNodeKind::PrimaryOutput
        }
    }
}

fn print_delay_usage() -> String {
    [
        "print_delay [-alrs] [-m model] [-f file] [-p n] n1 n2 ...",
        "    -a\t\tprint arrival times",
        "    -l\t\tprint output loads",
        "    -r\t\tprint required times",
        "    -s\t\tprint slack times",
        "    -m model\tchoose delay model (unit, unit-fanout, library, mapped, tdc)",
        "    -f file\t Force parameters to be read from file (for tdc model)",
        "    -p n\tonly print top 'n' values",
    ]
    .join("\n")
}

fn set_delay_usage() -> String {
    [
        "set_delay [-a|d|i|l|r f]  [-A f] [-D f] [-I f] [-L f] [-R f] [-S f] [-W f] [o1 o2 ... | i1 i2 ...]",
        "    -a f\t\tset arrival times to f",
        "    -d f\t\tset drives from primary inputs to f",
        "    -i f\t\tset max load limit on primary inputs to f",
        "    -l f\t\tset loads on primary outputs to f",
        "    -r f\t\tset required times to f",
        "    -A f\t\tset default arrival time to f",
        "    -D f\t\tset default input drive to f",
        "    -I f\t\tset default max input load to f",
        "    -L f\t\tset default output to f",
        "    -R f\t\tset default required time to f",
        "    -S f\t\tSet the wire load slope to f",
        "    -W f\t\tSet the next wire load to f",
        "specify at most one of a,r,l,i,d",
        "i1...in a vector of primary inputs",
        "o1...on a vector of primary outputs",
        "NOTE: a negative value means that the parameter is unspecified",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> DelayNetwork {
        let mut network = DelayNetwork::new();
        network.mapped = true;

        let mut input = DelayNode::new("a", DelayNodeKind::PrimaryInput);
        input.arrival = DelayTime::new(1.0, 2.0);
        input.drive = DelayTime::new(0.1, 0.2);
        input.max_input_load = 3.0;
        network.add_node(input);

        let mut internal = DelayNode::new("n1", DelayNodeKind::Internal);
        internal.arrival = DelayTime::new(5.0, 4.0);
        internal.required = DelayTime::new(8.0, 7.0);
        internal.slack = DelayTime::new(3.0, 3.0);
        internal.load = 2.0;
        let internal_index = network.add_node(internal);

        let mut output = DelayNode::new("z", DelayNodeKind::PrimaryOutput);
        output.required = DelayTime::new(10.0, 11.0);
        output.slack = DelayTime::new(5.0, 6.0);
        output.load = 4.0;
        output.fanin = Some(internal_index);
        output.fanin_fanout_count = 2;
        network.add_node(output);

        network.set_default_parameter(DelayDefaultParameter::ArrivalRise, 1.0);
        network.set_default_parameter(DelayDefaultParameter::ArrivalFall, 2.0);
        network.set_default_parameter(DelayDefaultParameter::OutputLoad, 4.5);
        network.wire_loads.push(WireLoad {
            name: "metal1".to_string(),
            slope: 0.25,
        });

        network
    }

    #[test]
    fn parse_print_delay_accepts_c_options_and_sets_first_sort_key() {
        let options =
            parse_print_delay_options(["print_delay", "-sa", "-m", "unit-fanout", "-p2", "n1"])
                .unwrap();

        assert!(options.print_slack);
        assert!(options.print_arrival);
        assert_eq!(options.sort, Some(DelaySort::Slack));
        assert_eq!(options.model, DelayModel::UnitFanout);
        assert_eq!(options.print_max, 2);
        assert_eq!(options.node_names, vec!["n1"]);
    }

    #[test]
    fn print_delay_sorts_descending_by_arrival_and_limits_rows() {
        let network = sample_network();
        let options =
            parse_print_delay_options(["print_delay", "-a", "-m", "unit", "-p", "1"]).unwrap();

        let output = print_delay(&network, &options);

        assert_eq!(output.status, 0);
        assert!(output
            .stdout
            .starts_with(" ... using unit delay model (levels)\n"));
        assert!(output.stdout.contains("n1        : arrival=( 5.00  4.00)"));
        assert!(!output.stdout.contains("z         :"));
    }

    #[test]
    fn print_delay_rejects_library_model_for_unmapped_network() {
        let mut network = sample_network();
        network.mapped = false;
        let options = parse_print_delay_options(["print_delay", "n1"]).unwrap();

        let output = print_delay(&network, &options);

        assert_eq!(output.status, 1);
        assert_eq!(
            output.stderr,
            "network not mapped, cannot use library delay model\n"
        );
    }

    #[test]
    fn constraints_report_defaults_inputs_outputs_and_wire_loads() {
        let network = sample_network();
        let output = print_delay_constraints(&network, &[]);

        assert_eq!(output.status, 0);
        assert!(output.stdout.contains("Default settings:"));
        assert!(output.stdout.contains("input arrival=(  1.00  2.00 "));
        assert!(output.stdout.contains("wire load metal1 slope=0.25"));
        assert!(output.stdout.contains("Settings for input a:\tarrival=("));
        assert!(output
            .stdout
            .contains("Settings for output z:\tload= 4.00 "));
    }

    #[test]
    fn set_delay_updates_one_node_parameter_for_matching_node_kinds() {
        let mut network = sample_network();
        let options = parse_set_delay_options(["set_delay", "-r", "9.5", "z"]).unwrap();

        set_delay(&mut network, &options).unwrap();

        assert_eq!(network.nodes[2].required, DelayTime::new(9.5, 9.5));
    }

    #[test]
    fn set_delay_negative_values_mark_parameters_unspecified() {
        let mut network = sample_network();
        let options = parse_set_delay_options(["set_delay", "-a", "-1", "a"]).unwrap();

        set_delay(&mut network, &options).unwrap();

        assert_eq!(network.nodes[0].arrival, DelayTime::not_set());
    }

    #[test]
    fn set_delay_rejects_multiple_node_parameters() {
        assert_eq!(
            parse_set_delay_options(["set_delay", "-a", "1", "-d", "2"]).unwrap_err(),
            DelayCommandError::TooManyNodeParameters
        );
    }

    #[test]
    fn set_delay_applies_default_pairs_without_selected_nodes() {
        let mut network = sample_network();
        let options = parse_set_delay_options(["set_delay", "-A", "3.5", "-W", "1.2"]).unwrap();

        set_delay(&mut network, &options).unwrap();

        assert_eq!(
            network.default_parameter(DelayDefaultParameter::ArrivalRise),
            3.5
        );
        assert_eq!(
            network.default_parameter(DelayDefaultParameter::ArrivalFall),
            3.5
        );
        assert_eq!(
            network.default_parameter(DelayDefaultParameter::AddWireLoad),
            1.2
        );
    }

    #[test]
    fn init_and_end_delay_are_native_registration_plans() {
        assert_eq!(
            init_delay_plan(),
            vec!["print_delay", "set_delay", "constraints"]
        );
        assert!(end_delay_plan().is_empty());
    }
}
