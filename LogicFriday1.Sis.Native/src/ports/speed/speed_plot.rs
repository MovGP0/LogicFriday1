//! Native Rust command/formatting port for `LogicSynthesis/sis/speed/speed_plot.c`.
//!
//! The legacy C command opens the SIS graphics backend, plots the current
//! network as BLIF, traces delay, computes critical nodes or cutsets, and then
//! writes graphics overlay commands. The SIS graphics, delay, network/node, and
//! speed-weight APIs are still C-only, so this module ports the independent
//! command option behavior and overlay formatting as native Rust APIs while the
//! full network execution reports explicit missing prerequisites.

use std::error::Error;
use std::fmt;

pub const DEFAULT_SPEED_THRESH: f64 = 0.5;
pub const DEFAULT_SPEED_COEFF: f64 = 0.0;
pub const DEFAULT_SPEED_DIST: i32 = 3;

pub const SPEED_PLOT_COMMAND: &str = "_speed_plot";

pub const REQUIRED_PORT_BEADS: &[&str] = &[
    "LogicFriday1-8j8.2.6.133", // delay/delay.c: delay_get_model_from_name, delay_trace, delay_latest_output
    "LogicFriday1-8j8.2.6.214", // graphics/com_graphics.c: com_graphics_enabled/open/close
    "LogicFriday1-8j8.2.6.216", // io/plot_blif.c: io_plot_network
    "LogicFriday1-8j8.2.6.257", // map/library.c: lib_gate_of, lib_gate_name
    "LogicFriday1-8j8.2.6.305", // network/network_util.c: network_name and network iteration helpers
    "LogicFriday1-8j8.2.6.317", // node/names.c: node_long_name
    "LogicFriday1-8j8.2.6.318", // node/node.c: node type/function data
    "LogicFriday1-8j8.2.6.465", // speed/com_speed.c: speed_fill_options
    "LogicFriday1-8j8.2.6.467", // speed/new_speed.c: new speed cutset flow
    "LogicFriday1-8j8.2.6.468", // speed/new_wght_util.c: new_speed_compute_weight/select/free
    "LogicFriday1-8j8.2.6.480", // speed/speed_util.c: set_speed_thresh
    "LogicFriday1-8j8.2.6.481", // speed/speedup.c: speed_critical
    "LogicFriday1-8j8.2.6.483", // speed/weight.c: speed_compute_weight and cutset weights
];

const USAGE_TAIL: &[&str] = &[
    "    -n name\tPlot name to use instead of network name.\n",
    "    -c\t\tHighlight minimum weight cutset.\n",
    "    -g\t\tUse gate names instead of node names\n",
    "    -f\t\tUse fast routine to compute weights\n",
    "    -H\t\tHighlight critical path nodes.\n",
    "    -t\tn.n\tCritical threshold (used with -f option).\n",
    "    -w\tn.n\tRelative weight of area (used with -f option).\n",
    "    -d\tn\tDistance for collapsing.\n",
    "    -m\tmodel\t Delay model\n",
    "    -s\tmethod\tMethod for selecting the region to transform.\n \t one of \"crit\"(default), \"transitive\", \"compromise\", \"tree\"\n",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel {
    Unit,
    Library,
    UnitFanout,
    Mapped,
    Tdc,
}

impl DelayModel {
    pub fn from_c_name(name: &str) -> Option<Self> {
        match name {
            "unit" | "DELAY_MODEL_UNIT" => Some(Self::Unit),
            "library" | "DELAY_MODEL_LIBRARY" => Some(Self::Library),
            "unit-fanout" | "unit_fanout" | "DELAY_MODEL_UNIT_FANOUT" => Some(Self::UnitFanout),
            "mapped" | "DELAY_MODEL_MAPPED" => Some(Self::Mapped),
            "tdc" | "DELAY_MODEL_TDC" => Some(Self::Tdc),
            _ => None,
        }
    }

    fn for_speed_plot_command(self) -> Self {
        if self == Self::Library {
            Self::Mapped
        } else {
            self
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedRegion {
    AlongCriticalPath,
    TransitiveFanin,
    Compromise,
    OnlyTree,
}

impl SpeedRegion {
    pub fn from_speed_plot_name(name: &str) -> Option<Self> {
        match name {
            "crit" => Some(Self::AlongCriticalPath),
            "transitive" => Some(Self::TransitiveFanin),
            "compromise" => Some(Self::Compromise),
            "tree" => Some(Self::OnlyTree),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedPlotOptions {
    pub plot_name: String,
    pub highlight_critical_path: bool,
    pub highlight_cutset: bool,
    pub print_gate_name: bool,
    pub delay_model: DelayModel,
    pub distance: i32,
    pub threshold: f64,
    pub area_weight: f64,
    pub use_new_weight_method: bool,
    pub region: SpeedRegion,
}

impl SpeedPlotOptions {
    pub fn with_network_name(network_name: impl Into<String>) -> Self {
        Self {
            plot_name: network_name.into(),
            highlight_critical_path: false,
            highlight_cutset: false,
            print_gate_name: false,
            delay_model: DelayModel::Unit,
            distance: DEFAULT_SPEED_DIST,
            threshold: DEFAULT_SPEED_THRESH,
            area_weight: DEFAULT_SPEED_COEFF,
            use_new_weight_method: true,
            region: SpeedRegion::AlongCriticalPath,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpeedPlotCommandError {
    MissingOptionValue(char),
    InvalidInteger { option: char, value: String },
    InvalidFloat { option: char, value: String },
    IllegalRegion(String),
    UnknownDelayModel(String),
    UnsupportedOption(String),
    UnexpectedOperand(String),
}

impl fmt::Display for SpeedPlotCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionValue(option) => write!(f, "-{option} requires an argument"),
            Self::InvalidInteger { option, value } => {
                write!(f, "invalid integer for -{option}: {value}")
            }
            Self::InvalidFloat { option, value } => {
                write!(f, "invalid float for -{option}: {value}")
            }
            Self::IllegalRegion(value) => write!(f, "illegal argument to the -s flag: {value}"),
            Self::UnknownDelayModel(value) => write!(f, "unknown delay model {value}"),
            Self::UnsupportedOption(option) => write!(f, "unsupported option {option}"),
            Self::UnexpectedOperand(operand) => write!(f, "unexpected operand {operand}"),
        }
    }
}

impl Error for SpeedPlotCommandError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedPlotDependency {
    GraphicsBackend,
    BlifPlotNetwork,
    DelayTrace,
    DelayModelLookup,
    NetworkAndNodeData,
    MappedLibraryGateData,
    SpeedOptionDefaults,
    SpeedThreshold,
    CriticalPathSelection,
    NewWeightCutsetSelection,
    LegacyWeightCutsetSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedPlotError {
    MissingDependency(SpeedPlotDependency),
}

impl fmt::Display for SpeedPlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDependency(dependency) => match dependency {
                SpeedPlotDependency::GraphicsBackend => {
                    write!(f, "SIS graphics backend is not ported to Rust yet")
                }
                SpeedPlotDependency::BlifPlotNetwork => {
                    write!(f, "SIS BLIF network plotting is not ported to Rust yet")
                }
                SpeedPlotDependency::DelayTrace => {
                    write!(f, "SIS delay tracing is not ported to Rust yet")
                }
                SpeedPlotDependency::DelayModelLookup => {
                    write!(f, "SIS delay model lookup is not ported to Rust yet")
                }
                SpeedPlotDependency::NetworkAndNodeData => {
                    write!(f, "SIS network/node data access is not ported to Rust yet")
                }
                SpeedPlotDependency::MappedLibraryGateData => {
                    write!(f, "SIS mapped library gate data is not ported to Rust yet")
                }
                SpeedPlotDependency::SpeedOptionDefaults => {
                    write!(f, "SIS speed option defaults are not ported to Rust yet")
                }
                SpeedPlotDependency::SpeedThreshold => {
                    write!(
                        f,
                        "SIS speed threshold calculation is not ported to Rust yet"
                    )
                }
                SpeedPlotDependency::CriticalPathSelection => {
                    write!(f, "SIS critical path selection is not ported to Rust yet")
                }
                SpeedPlotDependency::NewWeightCutsetSelection => {
                    write!(
                        f,
                        "SIS new weight cutset selection is not ported to Rust yet"
                    )
                }
                SpeedPlotDependency::LegacyWeightCutsetSelection => {
                    write!(
                        f,
                        "SIS legacy weight cutset selection is not ported to Rust yet"
                    )
                }
            },
        }
    }
}

impl Error for SpeedPlotError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateLabel<'a> {
    pub node_name: &'a str,
    pub gate_name: &'a str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HighlightOverlay<'a> {
    pub network_name: &'a str,
    pub plot_name: &'a str,
    pub latest_output_delay: f64,
    pub highlight_critical_path: bool,
    pub highlight_cutset: bool,
    pub critical_nodes: Vec<&'a str>,
    pub cutset_nodes: Vec<&'a str>,
}

pub fn required_port_beads() -> &'static [&'static str] {
    REQUIRED_PORT_BEADS
}

pub fn usage(command_name: &str) -> String {
    let mut result = format!(
        "usage: {command_name} [-n name] [-t thresh] [-cHgf] [-m model] [-t n.n] [-w n.n] [-d n] [-s method]\n"
    );
    for line in USAGE_TAIL {
        result.push_str(line);
    }
    result
}

pub fn speed_plot_usage() -> String {
    usage(SPEED_PLOT_COMMAND)
}

pub fn parse_speed_plot_args<I, S>(
    network_name: &str,
    args: I,
) -> Result<SpeedPlotOptions, SpeedPlotCommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = SpeedPlotOptions::with_network_name(network_name);
    let mut iter = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .peekable();
    let mut scanning_options = true;

    while let Some(arg) = iter.next() {
        if !scanning_options || !arg.starts_with('-') || arg == "-" {
            return Err(SpeedPlotCommandError::UnexpectedOperand(arg));
        }
        if arg == "--" {
            scanning_options = false;
            continue;
        }

        let mut chars = arg[1..].char_indices().peekable();
        while let Some((offset, option)) = chars.next() {
            match option {
                'c' => options.highlight_cutset = true,
                'g' => options.print_gate_name = true,
                'f' => options.use_new_weight_method = false,
                'H' => options.highlight_critical_path = true,
                'n' | 't' | 'w' | 'd' | 's' | 'm' => {
                    let value_start = offset + option.len_utf8();
                    let value = if value_start < arg[1..].len() {
                        arg[1 + value_start..].to_owned()
                    } else {
                        iter.next()
                            .ok_or(SpeedPlotCommandError::MissingOptionValue(option))?
                    };

                    apply_option_value(&mut options, option, value)?;
                    break;
                }
                _ => {
                    return Err(SpeedPlotCommandError::UnsupportedOption(format!(
                        "-{option}"
                    )));
                }
            }
        }
    }

    options.area_weight = options.area_weight.clamp(0.0, 1.0);
    Ok(options)
}

fn apply_option_value(
    options: &mut SpeedPlotOptions,
    option: char,
    value: String,
) -> Result<(), SpeedPlotCommandError> {
    match option {
        'n' => options.plot_name = value,
        't' => {
            options.threshold = value
                .parse()
                .map_err(|_| SpeedPlotCommandError::InvalidFloat {
                    option,
                    value: value.clone(),
                })?;
        }
        'w' => {
            options.area_weight =
                value
                    .parse()
                    .map_err(|_| SpeedPlotCommandError::InvalidFloat {
                        option,
                        value: value.clone(),
                    })?;
        }
        'd' => {
            options.distance =
                value
                    .parse()
                    .map_err(|_| SpeedPlotCommandError::InvalidInteger {
                        option,
                        value: value.clone(),
                    })?;
        }
        's' => {
            options.region = SpeedRegion::from_speed_plot_name(&value)
                .ok_or_else(|| SpeedPlotCommandError::IllegalRegion(value.clone()))?;
        }
        'm' => {
            options.delay_model = DelayModel::from_c_name(&value)
                .map(DelayModel::for_speed_plot_command)
                .ok_or_else(|| SpeedPlotCommandError::UnknownDelayModel(value.clone()))?;
        }
        _ => unreachable!("non-valued option passed to apply_option_value"),
    }

    Ok(())
}

pub fn render_gate_labels(labels: &[GateLabel<'_>]) -> String {
    let mut result = String::new();
    for label in labels {
        result.push_str(&format!(
            ".label\t{}\t{}\n",
            label.node_name, label.gate_name
        ));
    }
    result
}

pub fn render_highlight_overlay(overlay: &HighlightOverlay<'_>) -> String {
    let mut result = String::new();

    if overlay.highlight_critical_path {
        result.push_str(&format!(
            ".clear\tDelay = {:<5.2}\n",
            overlay.latest_output_delay
        ));
    } else if !overlay.highlight_cutset {
        result.push_str(&format!(".clear\t{}\n", overlay.network_name));
        result.push_str(&format!(
            ".command\t{SPEED_PLOT_COMMAND}\t{SPEED_PLOT_COMMAND} -H -n {}\tCritical\tHighlight critical path(s).\n",
            overlay.plot_name
        ));
        result.push_str(&format!(
            ".command\t{SPEED_PLOT_COMMAND}\t{SPEED_PLOT_COMMAND} -c -n {}\tCutset\tHighlight minimum weight cutset.\n",
            overlay.plot_name
        ));
    }

    if overlay.highlight_critical_path {
        render_node_line(&mut result, &overlay.critical_nodes);
    }

    if overlay.highlight_cutset && !overlay.cutset_nodes.is_empty() {
        if !overlay.highlight_critical_path {
            result.push_str(&format!(
                ".clear\tCutset of {} nodes\n",
                overlay.cutset_nodes.len()
            ));
        }

        let reversed = overlay
            .cutset_nodes
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>();
        render_node_line(&mut result, &reversed);
    }

    result
}

fn render_node_line(result: &mut String, nodes: &[&str]) {
    result.push_str(".nodes");
    for node in nodes {
        result.push('\t');
        result.push_str(node);
    }
    result.push('\n');
}

pub fn format_cutset_trace(distance: i32, threshold: f64) -> String {
    format!("Distance = {distance} ; Threshold = {threshold:5.2}\n")
}

pub fn plot_sis_network(_options: &SpeedPlotOptions) -> Result<(), SpeedPlotError> {
    Err(SpeedPlotError::MissingDependency(
        SpeedPlotDependency::GraphicsBackend,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_matches_c_text_for_command_name() {
        assert_eq!(
            speed_plot_usage(),
            concat!(
                "usage: _speed_plot [-n name] [-t thresh] [-cHgf] [-m model] [-t n.n] [-w n.n] [-d n] [-s method]\n",
                "    -n name\tPlot name to use instead of network name.\n",
                "    -c\t\tHighlight minimum weight cutset.\n",
                "    -g\t\tUse gate names instead of node names\n",
                "    -f\t\tUse fast routine to compute weights\n",
                "    -H\t\tHighlight critical path nodes.\n",
                "    -t\tn.n\tCritical threshold (used with -f option).\n",
                "    -w\tn.n\tRelative weight of area (used with -f option).\n",
                "    -d\tn\tDistance for collapsing.\n",
                "    -m\tmodel\t Delay model\n",
                "    -s\tmethod\tMethod for selecting the region to transform.\n \t one of \"crit\"(default), \"transitive\", \"compromise\", \"tree\"\n",
            )
        );
    }

    #[test]
    fn parses_defaults_and_clustered_flags() {
        let options = parse_speed_plot_args("net", ["-Hcg"]).unwrap();

        assert_eq!(
            options,
            SpeedPlotOptions {
                plot_name: "net".to_owned(),
                highlight_critical_path: true,
                highlight_cutset: true,
                print_gate_name: true,
                delay_model: DelayModel::Unit,
                distance: DEFAULT_SPEED_DIST,
                threshold: DEFAULT_SPEED_THRESH,
                area_weight: DEFAULT_SPEED_COEFF,
                use_new_weight_method: true,
                region: SpeedRegion::AlongCriticalPath,
            }
        );
    }

    #[test]
    fn parses_option_values_and_c_special_cases() {
        let options = parse_speed_plot_args(
            "net",
            [
                "-nplot", "-t", "1.25", "-w", "3.5", "-d7", "-m", "library", "-s", "tree", "-f",
            ],
        )
        .unwrap();

        assert_eq!(options.plot_name, "plot");
        assert_eq!(options.threshold, 1.25);
        assert_eq!(options.area_weight, 1.0);
        assert_eq!(options.distance, 7);
        assert_eq!(options.delay_model, DelayModel::Mapped);
        assert_eq!(options.region, SpeedRegion::OnlyTree);
        assert!(!options.use_new_weight_method);
    }

    #[test]
    fn rejects_invalid_arguments() {
        assert_eq!(
            parse_speed_plot_args("net", ["-s", "wide"]),
            Err(SpeedPlotCommandError::IllegalRegion("wide".to_owned()))
        );
        assert_eq!(
            parse_speed_plot_args("net", ["-m", "slow"]),
            Err(SpeedPlotCommandError::UnknownDelayModel("slow".to_owned()))
        );
        assert_eq!(
            parse_speed_plot_args("net", ["-t"]),
            Err(SpeedPlotCommandError::MissingOptionValue('t'))
        );
        assert_eq!(
            parse_speed_plot_args("net", ["operand"]),
            Err(SpeedPlotCommandError::UnexpectedOperand(
                "operand".to_owned()
            ))
        );
    }

    #[test]
    fn renders_graphics_overlays_like_speed_plot_c() {
        let base = HighlightOverlay {
            network_name: "net",
            plot_name: "plot",
            latest_output_delay: 12.345,
            highlight_critical_path: false,
            highlight_cutset: false,
            critical_nodes: Vec::new(),
            cutset_nodes: Vec::new(),
        };

        assert_eq!(
            render_highlight_overlay(&base),
            concat!(
                ".clear\tnet\n",
                ".command\t_speed_plot\t_speed_plot -H -n plot\tCritical\tHighlight critical path(s).\n",
                ".command\t_speed_plot\t_speed_plot -c -n plot\tCutset\tHighlight minimum weight cutset.\n",
            )
        );

        let highlighted = HighlightOverlay {
            highlight_critical_path: true,
            highlight_cutset: true,
            critical_nodes: vec!["n1", "n2"],
            cutset_nodes: vec!["c1", "c2", "c3"],
            ..base
        };

        assert_eq!(
            render_highlight_overlay(&highlighted),
            ".clear\tDelay = 12.35\n.nodes\tn1\tn2\n.nodes\tc3\tc2\tc1\n"
        );
    }

    #[test]
    fn renders_labels_trace_and_dependency_scaffold() {
        assert_eq!(
            render_gate_labels(&[
                GateLabel {
                    node_name: "n1",
                    gate_name: "NAND2"
                },
                GateLabel {
                    node_name: "n2",
                    gate_name: "INV"
                },
            ]),
            ".label\tn1\tNAND2\n.label\tn2\tINV\n"
        );
        assert_eq!(
            format_cutset_trace(3, 0.5),
            "Distance = 3 ; Threshold =  0.50\n"
        );
        assert!(required_port_beads().contains(&"LogicFriday1-8j8.2.6.214"));
        assert_eq!(
            plot_sis_network(&SpeedPlotOptions::with_network_name("net")),
            Err(SpeedPlotError::MissingDependency(
                SpeedPlotDependency::GraphicsBackend
            ))
        );
    }
}
