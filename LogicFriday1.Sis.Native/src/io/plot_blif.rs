//! Native Rust BLIF plot data generation.
//!
//! The legacy `plot_blif.c` sends a compact BLIF-like network description to
//! the graphics frontend. This port keeps that behavior as owned Rust data and
//! returns explicit graphics actions instead of opening process-global streams.

use std::error::Error;
use std::fmt;
use std::fmt::Write;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlotBlifNetwork {
    pub name: String,
    pub nodes: Vec<PlotBlifNode>,
    pub latches: Vec<PlotBlifLatch>,
    pub dc_network: Option<Box<PlotBlifNetwork>>,
}

impl PlotBlifNetwork {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            latches: Vec::new(),
            dc_network: None,
        }
    }

    pub fn add_node(&mut self, node: PlotBlifNode) {
        self.nodes.push(node);
    }

    pub fn add_latch(&mut self, latch: PlotBlifLatch) {
        self.latches.push(latch);
    }

    pub fn with_dc_network(mut self, dc_network: PlotBlifNetwork) -> Self {
        self.dc_network = Some(Box::new(dc_network));
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlotBlifNode {
    pub long_name: String,
    pub pretty_name: String,
    pub kind: PlotBlifNodeKind,
    pub fanins: Vec<String>,
    pub is_real_interface: bool,
}

impl PlotBlifNode {
    pub fn primary_input(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            long_name: name.clone(),
            pretty_name: name,
            kind: PlotBlifNodeKind::PrimaryInput,
            fanins: Vec::new(),
            is_real_interface: true,
        }
    }

    pub fn primary_output(name: impl Into<String>, fanin: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            long_name: name.clone(),
            pretty_name: name,
            kind: PlotBlifNodeKind::PrimaryOutput,
            fanins: vec![fanin.into()],
            is_real_interface: true,
        }
    }

    pub fn internal(
        name: impl Into<String>,
        fanins: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let name = name.into();
        Self {
            long_name: name.clone(),
            pretty_name: name,
            kind: PlotBlifNodeKind::Internal,
            fanins: fanins.into_iter().map(Into::into).collect(),
            is_real_interface: true,
        }
    }

    pub fn with_pretty_name(mut self, pretty_name: impl Into<String>) -> Self {
        self.pretty_name = pretty_name.into();
        self
    }

    pub fn with_real_interface(mut self, is_real_interface: bool) -> Self {
        self.is_real_interface = is_real_interface;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlotBlifNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlotBlifLatch {
    pub input: String,
    pub output: String,
}

impl PlotBlifLatch {
    pub fn new(input: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            output: output.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlotBlifOptions {
    pub plot_name: Option<String>,
    pub internal_names: bool,
    pub close: bool,
    pub replace: bool,
    pub geometry: Option<String>,
    pub dc_network: bool,
}

impl Default for PlotBlifOptions {
    fn default() -> Self {
        Self {
            plot_name: None,
            internal_names: false,
            close: false,
            replace: false,
            geometry: None,
            dc_network: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlotBlifAction {
    Close {
        plot_name: String,
    },
    Open {
        plot_name: String,
        mode: PlotBlifOpenMode,
        payload: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlotBlifOpenMode {
    New,
    Replace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlotBlifError {
    MissingNetwork,
    MissingDcNetwork,
    MissingOptionArgument { option: String },
    UnknownOption { option: String },
    UnexpectedArgument { argument: String },
}

impl fmt::Display for PlotBlifError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNetwork => write!(f, "plot_blif requires a network"),
            Self::MissingDcNetwork => {
                write!(f, "plot_blif requested a DC network, but none exists")
            }
            Self::MissingOptionArgument { option } => {
                write!(f, "plot_blif option {option} requires an argument")
            }
            Self::UnknownOption { option } => write!(f, "unknown plot_blif option {option}"),
            Self::UnexpectedArgument { argument } => {
                write!(f, "unexpected plot_blif argument {argument}")
            }
        }
    }
}

impl Error for PlotBlifError {}

pub type PlotBlifResult<T> = Result<T, PlotBlifError>;

pub fn io_plot_network(network: &PlotBlifNetwork, internal_names: bool) -> String {
    let mut output = String::new();
    write_plot_network(&mut output, network, internal_names);
    output
}

pub fn plot_blif(
    network: Option<&PlotBlifNetwork>,
    options: &PlotBlifOptions,
) -> PlotBlifResult<PlotBlifAction> {
    let network = network.ok_or(PlotBlifError::MissingNetwork)?;
    let plot_name = options
        .plot_name
        .clone()
        .unwrap_or_else(|| network.name.clone());

    if options.close {
        return Ok(PlotBlifAction::Close { plot_name });
    }

    let selected_network = if options.dc_network {
        network
            .dc_network
            .as_deref()
            .ok_or(PlotBlifError::MissingDcNetwork)?
    } else {
        network
    };

    let mut payload = String::new();
    if !options.replace {
        if let Some(geometry) = &options.geometry {
            writeln!(payload, ".geometry\t{geometry}").expect("writing to String cannot fail");
        }
    }
    write_plot_network(&mut payload, selected_network, options.internal_names);

    Ok(PlotBlifAction::Open {
        plot_name,
        mode: if options.replace {
            PlotBlifOpenMode::Replace
        } else {
            PlotBlifOpenMode::New
        },
        payload,
    })
}

pub fn parse_plot_blif_options(argv: &[&str]) -> PlotBlifResult<PlotBlifOptions> {
    let mut options = PlotBlifOptions::default();
    let mut index = 1;

    while index < argv.len() {
        let argument = argv[index];
        match argument {
            "-i" => options.internal_names = true,
            "-k" => options.close = true,
            "-r" => options.replace = true,
            "-d" => options.dc_network = true,
            "-g" | "-n" => {
                index += 1;
                let value =
                    argv.get(index)
                        .ok_or_else(|| PlotBlifError::MissingOptionArgument {
                            option: argument.to_string(),
                        })?;
                if argument == "-g" {
                    options.geometry = Some((*value).to_string());
                } else {
                    options.plot_name = Some((*value).to_string());
                }
            }
            _ if argument.starts_with('-') => {
                return Err(PlotBlifError::UnknownOption {
                    option: argument.to_string(),
                });
            }
            _ => {
                return Err(PlotBlifError::UnexpectedArgument {
                    argument: argument.to_string(),
                });
            }
        }
        index += 1;
    }

    Ok(options)
}

pub fn com_plot_blif(
    network: Option<&PlotBlifNetwork>,
    argv: &[&str],
) -> PlotBlifResult<PlotBlifAction> {
    let options = parse_plot_blif_options(argv)?;
    plot_blif(network, &options)
}

fn write_plot_network(output: &mut String, network: &PlotBlifNetwork, internal_names: bool) {
    writeln!(output, ".model\t{}", network.name).expect("writing to String cannot fail");

    output.push_str(".inputs");
    for node in &network.nodes {
        if node.kind == PlotBlifNodeKind::PrimaryInput && node.is_real_interface {
            write!(output, "\t{}", node.long_name).expect("writing to String cannot fail");
        }
    }
    output.push('\n');

    output.push_str(".outputs");
    for node in &network.nodes {
        if node.kind == PlotBlifNodeKind::PrimaryOutput && node.is_real_interface {
            write!(output, "\t{}", node.long_name).expect("writing to String cannot fail");
        }
    }
    output.push('\n');

    for node in &network.nodes {
        if node.fanins.is_empty() {
            continue;
        }

        write!(output, ".node\t{}", node.long_name).expect("writing to String cannot fail");
        for fanin in &node.fanins {
            write!(output, "\t{fanin}").expect("writing to String cannot fail");
        }
        output.push('\n');

        if !internal_names && node.long_name != node.pretty_name {
            writeln!(output, ".label\t{}\t{}", node.long_name, node.pretty_name)
                .expect("writing to String cannot fail");
        }
    }

    for latch in &network.latches {
        writeln!(output, ".latch\t{}\t{}", latch.input, latch.output)
            .expect("writing to String cannot fail");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network() -> PlotBlifNetwork {
        let mut network = PlotBlifNetwork::new("demo");
        network.add_node(PlotBlifNode::primary_input("a"));
        network.add_node(PlotBlifNode::primary_input("clk").with_real_interface(false));
        network
            .add_node(PlotBlifNode::internal("[3]", ["a", "clk"]).with_pretty_name("logic_node"));
        network.add_node(PlotBlifNode::primary_output("y", "[3]"));
        network
            .add_node(PlotBlifNode::primary_output("latch_in", "[3]").with_real_interface(false));
        network.add_latch(PlotBlifLatch::new("latch_in", "q"));
        network
    }

    #[test]
    fn io_plot_network_writes_real_interfaces_nodes_labels_and_latches() {
        let plot = io_plot_network(&sample_network(), false);

        assert_eq!(
            plot,
            concat!(
                ".model\tdemo\n",
                ".inputs\ta\n",
                ".outputs\ty\n",
                ".node\t[3]\ta\tclk\n",
                ".label\t[3]\tlogic_node\n",
                ".node\ty\t[3]\n",
                ".node\tlatch_in\t[3]\n",
                ".latch\tlatch_in\tq\n",
            )
        );
    }

    #[test]
    fn internal_name_mode_suppresses_labels() {
        let plot = io_plot_network(&sample_network(), true);

        assert!(plot.contains(".node\t[3]\ta\tclk\n"));
        assert!(!plot.contains(".label"));
    }

    #[test]
    fn new_plot_adds_geometry_before_payload() {
        let action = plot_blif(
            Some(&sample_network()),
            &PlotBlifOptions {
                geometry: Some("640x480+10+20".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            action,
            PlotBlifAction::Open {
                plot_name: "demo".to_string(),
                mode: PlotBlifOpenMode::New,
                payload: concat!(
                    ".geometry\t640x480+10+20\n",
                    ".model\tdemo\n",
                    ".inputs\ta\n",
                    ".outputs\ty\n",
                    ".node\t[3]\ta\tclk\n",
                    ".label\t[3]\tlogic_node\n",
                    ".node\ty\t[3]\n",
                    ".node\tlatch_in\t[3]\n",
                    ".latch\tlatch_in\tq\n",
                )
                .to_string(),
            }
        );
    }

    #[test]
    fn replace_plot_ignores_geometry_and_uses_requested_name() {
        let action = plot_blif(
            Some(&sample_network()),
            &PlotBlifOptions {
                plot_name: Some("window".to_string()),
                replace: true,
                geometry: Some("800x600".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            action,
            PlotBlifAction::Open {
                plot_name: "window".to_string(),
                mode: PlotBlifOpenMode::Replace,
                payload: io_plot_network(&sample_network(), false),
            }
        );
    }

    #[test]
    fn close_plot_returns_close_action_without_payload() {
        let action = plot_blif(
            Some(&sample_network()),
            &PlotBlifOptions {
                close: true,
                plot_name: Some("demo-window".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            action,
            PlotBlifAction::Close {
                plot_name: "demo-window".to_string()
            }
        );
    }

    #[test]
    fn dc_flag_selects_attached_dc_network() {
        let mut dc = PlotBlifNetwork::new("demo_dc");
        dc.add_node(PlotBlifNode::primary_input("a"));
        dc.add_node(PlotBlifNode::internal("dc_n", ["a"]));
        dc.add_node(PlotBlifNode::primary_output("y", "dc_n"));
        let network = sample_network().with_dc_network(dc);

        let action = plot_blif(
            Some(&network),
            &PlotBlifOptions {
                dc_network: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(matches!(action, PlotBlifAction::Open { .. }));
        if let PlotBlifAction::Open { payload, .. } = action {
            assert!(payload.starts_with(".model\tdemo_dc\n"));
            assert!(payload.contains(".node\tdc_n\ta\n"));
        }
    }

    #[test]
    fn missing_network_and_missing_dc_are_errors() {
        assert_eq!(
            plot_blif(None, &PlotBlifOptions::default()).unwrap_err(),
            PlotBlifError::MissingNetwork
        );

        assert_eq!(
            plot_blif(
                Some(&sample_network()),
                &PlotBlifOptions {
                    dc_network: true,
                    ..Default::default()
                },
            )
            .unwrap_err(),
            PlotBlifError::MissingDcNetwork
        );
    }

    #[test]
    fn parser_preserves_legacy_flags() {
        let options =
            parse_plot_blif_options(&["plot_blif", "-n", "view", "-i", "-k", "-g", "WxH", "-d"])
                .unwrap();

        assert_eq!(
            options,
            PlotBlifOptions {
                plot_name: Some("view".to_string()),
                internal_names: true,
                close: true,
                replace: false,
                geometry: Some("WxH".to_string()),
                dc_network: true,
            }
        );
    }

    #[test]
    fn parser_rejects_bad_arguments() {
        assert!(matches!(
            parse_plot_blif_options(&["plot_blif", "-g"]),
            Err(PlotBlifError::MissingOptionArgument { .. })
        ));
        assert!(matches!(
            parse_plot_blif_options(&["plot_blif", "-x"]),
            Err(PlotBlifError::UnknownOption { .. })
        ));
        assert!(matches!(
            parse_plot_blif_options(&["plot_blif", "extra"]),
            Err(PlotBlifError::UnexpectedArgument { .. })
        ));
    }
}
