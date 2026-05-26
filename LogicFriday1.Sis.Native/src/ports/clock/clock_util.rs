//! Native Rust transitive-clock lookup for `sis/clock/clock_util.c`.
//!
//! The SIS routine walks backward through a network node's fanins, returns the
//! first primary-input clock found by name, accumulates pin delays along that
//! path, and composes the input phases. This module exposes that behavior on an
//! owned Rust graph model so higher-level ports can use it without legacy C ABI
//! shims.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind
{
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayModel
{
    Unit,
    UnitFanout,
    Library,
    Mapped,
    Tdc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase
{
    PositiveUnate,
    NegativeUnate,
    Binate,
}

impl InputPhase
{
    pub const fn compose(self, arc_phase: Self) -> Self
    {
        match arc_phase {
            Self::Binate => Self::Binate,
            Self::PositiveUnate => self,
            Self::NegativeUnate => match self {
                Self::PositiveUnate => Self::NegativeUnate,
                Self::NegativeUnate => Self::PositiveUnate,
                Self::Binate => Self::Binate,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTime
{
    pub rise: f64,
    pub fall: f64,
}

impl DelayTime
{
    pub const fn new(rise: f64, fall: f64) -> Self
    {
        Self { rise, fall }
    }

    pub const fn zero() -> Self
    {
        Self {
            rise: 0.0,
            fall: 0.0,
        }
    }

    pub fn add_pin_delay(&mut self, pin_delay: Self)
    {
        self.rise += pin_delay.rise;
        self.fall += pin_delay.fall;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinDelay
{
    pub model: DelayModel,
    pub delay: DelayTime,
}

impl PinDelay
{
    pub const fn new(model: DelayModel, delay: DelayTime) -> Self
    {
        Self { model, delay }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockFanin
{
    pub node: NodeId,
    pub phase: InputPhase,
    pub delays: Vec<PinDelay>,
}

impl ClockFanin
{
    pub fn new(node: NodeId, phase: InputPhase, delays: Vec<PinDelay>) -> Self
    {
        Self {
            node,
            phase,
            delays,
        }
    }

    pub fn delay_for_model(&self, model: DelayModel) -> DelayTime
    {
        self.delays
            .iter()
            .find(|delay| delay.model == model)
            .map_or_else(DelayTime::zero, |delay| delay.delay)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockNode
{
    pub name: String,
    pub kind: NodeKind,
    pub fanins: Vec<ClockFanin>,
}

impl ClockNode
{
    pub fn new(name: impl Into<String>, kind: NodeKind, fanins: Vec<ClockFanin>) -> Self
    {
        Self {
            name: name.into(),
            kind,
            fanins,
        }
    }

    pub fn primary_input(name: impl Into<String>) -> Self
    {
        Self::new(name, NodeKind::PrimaryInput, Vec::new())
    }

    pub fn primary_output(name: impl Into<String>, fanin: NodeId) -> Self
    {
        Self::new(
            name,
            NodeKind::PrimaryOutput,
            vec![ClockFanin::new(
                fanin,
                InputPhase::PositiveUnate,
                Vec::new(),
            )],
        )
    }

    pub fn internal(name: impl Into<String>, fanins: Vec<ClockFanin>) -> Self
    {
        Self::new(name, NodeKind::Internal, fanins)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SisClock
{
    pub name: String,
}

impl SisClock
{
    pub fn new(name: impl Into<String>) -> Self
    {
        Self { name: name.into() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockNetwork
{
    pub nodes: Vec<ClockNode>,
    pub clocks: Vec<SisClock>,
}

impl ClockNetwork
{
    pub fn new(nodes: Vec<ClockNode>, clocks: Vec<SisClock>) -> Self
    {
        Self { nodes, clocks }
    }

    pub fn node(&self, id: NodeId) -> Option<&ClockNode>
    {
        self.nodes.get(id.0)
    }

    pub fn clock_by_name(&self, name: &str) -> Option<&SisClock>
    {
        self.clocks.iter().find(|clock| clock.name == name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitiveClock
{
    pub clock_name: String,
    pub offset: DelayTime,
    pub phase: InputPhase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockUtilError
{
    MissingNode(NodeId),
    PrimaryOutputWithoutFanin(NodeId),
    Cycle(NodeId),
}

impl fmt::Display for ClockUtilError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::MissingNode(node) => {
                write!(f, "node {} is not present in the clock network", node.0)
            }
            Self::PrimaryOutputWithoutFanin(node) => {
                write!(f, "primary output node {} has no fanin", node.0)
            }
            Self::Cycle(node) => write!(
                f,
                "cycle detected while tracing clock fanin at node {}",
                node.0
            ),
        }
    }
}

impl Error for ClockUtilError {}

pub fn clock_get_transitive_clock(
    network: &ClockNetwork,
    node: NodeId,
    model: DelayModel,
) -> Result<Option<TransitiveClock>, ClockUtilError>
{
    let mut visited = HashSet::new();
    let mut result = trace_transitive_clock(network, node, model, &mut visited)?;
    if let Some(result) = &mut result {
        result.offset = DelayTime::new(
            normalize_zero(result.offset.rise),
            normalize_zero(result.offset.fall),
        );
    }

    Ok(result)
}

fn trace_transitive_clock(
    network: &ClockNetwork,
    node_id: NodeId,
    model: DelayModel,
    visited: &mut HashSet<NodeId>,
) -> Result<Option<TransitiveClock>, ClockUtilError>
{
    if !visited.insert(node_id) {
        return Err(ClockUtilError::Cycle(node_id));
    }

    let node = network
        .node(node_id)
        .ok_or(ClockUtilError::MissingNode(node_id))?;
    let result = match node.kind {
        NodeKind::PrimaryInput => {
            Ok(network
                .clock_by_name(&node.name)
                .map(|clock| TransitiveClock {
                    clock_name: clock.name.clone(),
                    offset: DelayTime::zero(),
                    phase: InputPhase::PositiveUnate,
                }))
        }
        NodeKind::PrimaryOutput => {
            let fanin = node
                .fanins
                .first()
                .ok_or(ClockUtilError::PrimaryOutputWithoutFanin(node_id))?;
            trace_through_fanin(network, fanin, model, visited)
        }
        NodeKind::Internal => {
            let mut clock = None;
            for fanin in &node.fanins {
                if let Some(candidate) = trace_through_fanin(network, fanin, model, visited)? {
                    clock = Some(candidate);
                    break;
                }
            }
            Ok(clock)
        }
    };

    visited.remove(&node_id);
    result
}

fn trace_through_fanin(
    network: &ClockNetwork,
    fanin: &ClockFanin,
    model: DelayModel,
    visited: &mut HashSet<NodeId>,
) -> Result<Option<TransitiveClock>, ClockUtilError>
{
    let Some(mut clock) = trace_transitive_clock(network, fanin.node, model, visited)? else {
        return Ok(None);
    };

    clock.offset.add_pin_delay(fanin.delay_for_model(model));
    clock.phase = clock.phase.compose(fanin.phase);
    Ok(Some(clock))
}

fn normalize_zero(value: f64) -> f64
{
    if value == -0.0 { 0.0 } else { value }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn pin(node: usize, phase: InputPhase, rise: f64, fall: f64) -> ClockFanin
    {
        ClockFanin::new(
            NodeId(node),
            phase,
            vec![PinDelay::new(
                DelayModel::Mapped,
                DelayTime::new(rise, fall),
            )],
        )
    }

    #[test]
    fn primary_input_clock_is_found_by_name()
    {
        let network = ClockNetwork::new(
            vec![ClockNode::primary_input("clk")],
            vec![SisClock::new("clk")],
        );

        let clock = clock_get_transitive_clock(&network, NodeId(0), DelayModel::Mapped).unwrap();

        assert_eq!(
            clock,
            Some(TransitiveClock {
                clock_name: "clk".to_owned(),
                offset: DelayTime::zero(),
                phase: InputPhase::PositiveUnate,
            })
        );
    }

    #[test]
    fn primary_input_without_matching_clock_returns_none()
    {
        let network = ClockNetwork::new(
            vec![ClockNode::primary_input("data")],
            vec![SisClock::new("clk")],
        );

        assert_eq!(
            clock_get_transitive_clock(&network, NodeId(0), DelayModel::Mapped).unwrap(),
            None
        );
    }

    #[test]
    fn primary_output_traces_through_its_first_fanin()
    {
        let network = ClockNetwork::new(
            vec![
                ClockNode::primary_input("clk"),
                ClockNode::primary_output("out", NodeId(0)),
            ],
            vec![SisClock::new("clk")],
        );

        let clock = clock_get_transitive_clock(&network, NodeId(1), DelayModel::Mapped).unwrap();

        assert_eq!(clock.unwrap().clock_name, "clk");
    }

    #[test]
    fn offset_accumulates_pin_delay_on_path_to_queried_node()
    {
        let network = ClockNetwork::new(
            vec![
                ClockNode::primary_input("clk"),
                ClockNode::internal("mid", vec![pin(0, InputPhase::PositiveUnate, 1.25, 1.5)]),
                ClockNode::internal("top", vec![pin(1, InputPhase::PositiveUnate, 2.0, 3.0)]),
            ],
            vec![SisClock::new("clk")],
        );

        let clock = clock_get_transitive_clock(&network, NodeId(2), DelayModel::Mapped)
            .unwrap()
            .unwrap();

        assert_eq!(clock.offset, DelayTime::new(3.25, 4.5));
    }

    #[test]
    fn negative_unate_arcs_toggle_phase()
    {
        let network = ClockNetwork::new(
            vec![
                ClockNode::primary_input("clk"),
                ClockNode::internal("inv1", vec![pin(0, InputPhase::NegativeUnate, 0.0, 0.0)]),
                ClockNode::internal("inv2", vec![pin(1, InputPhase::NegativeUnate, 0.0, 0.0)]),
            ],
            vec![SisClock::new("clk")],
        );

        let first = clock_get_transitive_clock(&network, NodeId(1), DelayModel::Mapped)
            .unwrap()
            .unwrap();
        let second = clock_get_transitive_clock(&network, NodeId(2), DelayModel::Mapped)
            .unwrap()
            .unwrap();

        assert_eq!(first.phase, InputPhase::NegativeUnate);
        assert_eq!(second.phase, InputPhase::PositiveUnate);
    }

    #[test]
    fn binate_arc_dominates_later_phase_composition()
    {
        let network = ClockNetwork::new(
            vec![
                ClockNode::primary_input("clk"),
                ClockNode::internal("mix", vec![pin(0, InputPhase::Binate, 0.0, 0.0)]),
                ClockNode::internal("inv", vec![pin(1, InputPhase::NegativeUnate, 0.0, 0.0)]),
            ],
            vec![SisClock::new("clk")],
        );

        let clock = clock_get_transitive_clock(&network, NodeId(2), DelayModel::Mapped)
            .unwrap()
            .unwrap();

        assert_eq!(clock.phase, InputPhase::Binate);
    }

    #[test]
    fn first_fanin_with_transitive_clock_wins()
    {
        let network = ClockNetwork::new(
            vec![
                ClockNode::primary_input("data"),
                ClockNode::primary_input("clk_a"),
                ClockNode::primary_input("clk_b"),
                ClockNode::internal(
                    "top",
                    vec![
                        pin(0, InputPhase::PositiveUnate, 9.0, 9.0),
                        pin(1, InputPhase::PositiveUnate, 1.0, 1.0),
                        pin(2, InputPhase::PositiveUnate, 2.0, 2.0),
                    ],
                ),
            ],
            vec![SisClock::new("clk_a"), SisClock::new("clk_b")],
        );

        let clock = clock_get_transitive_clock(&network, NodeId(3), DelayModel::Mapped)
            .unwrap()
            .unwrap();

        assert_eq!(clock.clock_name, "clk_a");
        assert_eq!(clock.offset, DelayTime::new(1.0, 1.0));
    }

    #[test]
    fn missing_model_delay_defaults_to_zero_like_unset_delay_node_pin()
    {
        let network = ClockNetwork::new(
            vec![
                ClockNode::primary_input("clk"),
                ClockNode::internal(
                    "top",
                    vec![ClockFanin::new(
                        NodeId(0),
                        InputPhase::PositiveUnate,
                        vec![PinDelay::new(DelayModel::Unit, DelayTime::new(3.0, 4.0))],
                    )],
                ),
            ],
            vec![SisClock::new("clk")],
        );

        let clock = clock_get_transitive_clock(&network, NodeId(1), DelayModel::Mapped)
            .unwrap()
            .unwrap();

        assert_eq!(clock.offset, DelayTime::zero());
    }

    #[test]
    fn malformed_graph_reports_errors()
    {
        let network = ClockNetwork::new(
            vec![ClockNode::new("out", NodeKind::PrimaryOutput, Vec::new())],
            vec![SisClock::new("clk")],
        );

        assert_eq!(
            clock_get_transitive_clock(&network, NodeId(0), DelayModel::Mapped),
            Err(ClockUtilError::PrimaryOutputWithoutFanin(NodeId(0)))
        );
        assert_eq!(
            clock_get_transitive_clock(&network, NodeId(9), DelayModel::Mapped),
            Err(ClockUtilError::MissingNode(NodeId(9)))
        );
    }

    #[test]
    fn cycles_are_reported_instead_of_recursing_forever()
    {
        let network = ClockNetwork::new(
            vec![ClockNode::internal(
                "loop",
                vec![pin(0, InputPhase::PositiveUnate, 0.0, 0.0)],
            )],
            vec![SisClock::new("clk")],
        );

        assert_eq!(
            clock_get_transitive_clock(&network, NodeId(0), DelayModel::Mapped),
            Err(ClockUtilError::Cycle(NodeId(0)))
        );
    }
}
