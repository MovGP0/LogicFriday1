//! Native Rust port of the feasible utility behavior in `sis/speed/speed_util.c`.
//!
//! The C file mixes small formulas and formatting helpers with direct SIS
//! `node_t`, `network_t`, delay, mapping, and library mutation. The pure
//! behavior is represented here over owned Rust data. Entry points that still
//! require native SIS graph ports report explicit dependency errors.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::hash::Hash;

pub const MIN_AREA_BUF_NAME: &str = "";

pub const ALONG_CRIT_PATH: i32 = 0;
pub const TRANSITIVE_FANIN: i32 = 1;
pub const COMPROMISE: i32 = 2;
pub const ONLY_TREE: i32 = 3;

pub const CLP: i32 = 0;
pub const FAN: i32 = 1;
pub const DUAL: i32 = 2;

pub const AREA_BASED: i32 = 0;
pub const TRANSFORM_BASED: i32 = 1;

pub const BEST_BENEFIT: i32 = 0;
pub const BEST_BANG_FOR_BUCK: i32 = 1;

pub const DEFAULT_SPEED_THRESH: f64 = 0.5;
pub const DEFAULT_SPEED_COEFF: f64 = 0.0;
pub const DEFAULT_SPEED_DIST: i32 = 3;

pub const NSP_EPSILON: f64 = 1.0e-6;
pub const NSP_INPUT_SEPARATOR: char = '#';
pub const NSP_OUTPUT_SEPARATOR: char = '%';

pub const POS_LARGE: f64 = 10_000.0;
pub const NEG_LARGE: f64 = -10_000.0;
pub const MAXWEIGHT: i32 = 1_000;

pub const SP_PI: f64 = std::f64::consts::PI;
pub const SP_PI_2: f64 = std::f64::consts::FRAC_PI_2;
pub const SP_PI_4: f64 = std::f64::consts::FRAC_PI_4;
pub const SP_1_PI: f64 = std::f64::consts::FRAC_1_PI;

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
        d_min(self.rise, self.fall)
    }

    pub fn max_edge(self) -> f64 {
        d_max(self.rise, self.fall)
    }
}

pub fn d_min(left: f64, right: f64) -> f64 {
    left.min(right)
}

pub fn d_max(left: f64, right: f64) -> f64 {
    left.max(right)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedRegion {
    AlongCriticalPath,
    TransitiveFanin,
    Compromise,
    OnlyTree,
    Unknown(i32),
}

impl SpeedRegion {
    pub fn from_c_flag(flag: i32) -> Self {
        match flag {
            ALONG_CRIT_PATH => Self::AlongCriticalPath,
            TRANSITIVE_FANIN => Self::TransitiveFanin,
            COMPROMISE => Self::Compromise,
            ONLY_TREE => Self::OnlyTree,
            other => Self::Unknown(other),
        }
    }

    pub fn method_name(self) -> &'static str {
        match self {
            Self::AlongCriticalPath => "CRITICAL",
            Self::Compromise => "COMPROMISE",
            Self::OnlyTree => "TREE",
            Self::TransitiveFanin => "TRANSITIVE",
            Self::Unknown(_) => "UNKNOWN",
        }
    }
}

pub fn speed_method_name(flag: i32) -> &'static str {
    SpeedRegion::from_c_flag(flag).method_name()
}

pub fn speed_improved(req_times_set: bool, best: f64, current: f64) -> bool {
    if req_times_set {
        best > current + NSP_EPSILON
    } else {
        best < current - NSP_EPSILON
    }
}

pub fn speed_performance_label(req_times_set: bool) -> &'static str {
    if req_times_set { "Slack" } else { "Delay" }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralValue {
    Zero,
    One,
    DontCare,
}

impl LiteralValue {
    pub fn phase(self) -> Option<bool> {
        match self {
            Self::Zero => Some(false),
            Self::One => Some(true),
            Self::DontCare => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedLiteral {
    pub fanin_index: usize,
    pub phase: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedCube {
    pub literals: Vec<DecodedLiteral>,
}

pub fn decode_node_cube(cube: &[LiteralValue]) -> DecodedCube {
    DecodedCube {
        literals: cube
            .iter()
            .enumerate()
            .filter_map(|(fanin_index, literal)| {
                literal
                    .phase()
                    .map(|phase| DecodedLiteral { fanin_index, phase })
            })
            .collect(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CubeTiming {
    pub cube_index: usize,
    pub fanin_arrivals: Vec<DelayTime>,
}

pub fn cube_is_critical(cube: &CubeTiming, threshold: f64) -> bool {
    cube.fanin_arrivals
        .iter()
        .any(|arrival| arrival.max_edge() > threshold)
}

pub fn noncritical_cube_indices(cubes: &[CubeTiming], threshold: f64) -> Vec<usize> {
    cubes
        .iter()
        .filter(|cube| !cube_is_critical(cube, threshold))
        .map(|cube| cube.cube_index)
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedFanin<'a, N> {
    pub name: &'a str,
    pub node: N,
}

pub fn name_to_fanin<'a, N: Copy>(fanins: &'a [NamedFanin<'a, N>], name: &str) -> Option<N> {
    fanins
        .iter()
        .find(|fanin| fanin.name == name)
        .map(|fanin| fanin.node)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LevelNode<N> {
    pub id: N,
    pub kind: NodeKind,
    pub fanins: Vec<N>,
    pub slack: DelayTime,
    pub critical: bool,
}

impl<N> LevelNode<N> {
    pub fn new(id: N, kind: NodeKind, fanins: Vec<N>, slack: DelayTime, critical: bool) -> Self {
        Self {
            id,
            kind,
            fanins,
            slack,
            critical,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeedUtilError {
    MissingSisPorts { operation: &'static str },
    MissingNode(String),
    Cycle(String),
    EmptyGateClass,
}

impl fmt::Display for SpeedUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts { operation } => {
                write!(f, "{operation} is blocked by unported SIS dependencies")
            }
            Self::MissingNode(node) => write!(f, "levelization references missing node {node}"),
            Self::Cycle(node) => write!(f, "levelization found a cycle at node {node}"),
            Self::EmptyGateClass => write!(f, "library class has no gates"),
        }
    }
}

impl Error for SpeedUtilError {}

pub fn compute_levels<N>(nodes: &[LevelNode<N>]) -> Result<HashMap<N, usize>, SpeedUtilError>
where
    N: Clone + Eq + Hash + ToString,
{
    let by_id: HashMap<N, &LevelNode<N>> =
        nodes.iter().map(|node| (node.id.clone(), node)).collect();
    let mut levels: HashMap<N, usize> = nodes.iter().map(|node| (node.id.clone(), 0)).collect();
    let mut visiting = HashSet::new();

    for output in nodes
        .iter()
        .filter(|node| node.kind == NodeKind::PrimaryOutput)
    {
        assign_level(&output.id, &by_id, &mut levels, &mut visiting)?;
    }

    Ok(levels)
}

fn assign_level<N>(
    node_id: &N,
    nodes: &HashMap<N, &LevelNode<N>>,
    levels: &mut HashMap<N, usize>,
    visiting: &mut HashSet<N>,
) -> Result<usize, SpeedUtilError>
where
    N: Clone + Eq + Hash + ToString,
{
    let node = nodes
        .get(node_id)
        .copied()
        .ok_or_else(|| SpeedUtilError::MissingNode(node_id.to_string()))?;
    let current = levels.get(node_id).copied().unwrap_or(0);

    if node.kind == NodeKind::PrimaryInput || current > 0 {
        return Ok(current);
    }

    if !visiting.insert(node_id.clone()) {
        return Err(SpeedUtilError::Cycle(node_id.to_string()));
    }

    let mut level = 0;
    for fanin in &node.fanins {
        level = level.max(assign_level(fanin, nodes, levels, visiting)?);
    }
    visiting.remove(node_id);

    let new_level = level + 1;
    levels.insert(node_id.clone(), new_level);
    Ok(new_level)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LevelBucket<N> {
    pub level: usize,
    pub nodes: Vec<N>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrintedLevelSummary<N> {
    pub buckets: Vec<LevelBucket<N>>,
    pub max_level: usize,
}

pub fn printable_levels<N>(
    nodes: &[LevelNode<N>],
    selected_nodes: Option<&HashSet<N>>,
    critical_only: bool,
    threshold: f64,
) -> Result<PrintedLevelSummary<N>, SpeedUtilError>
where
    N: Clone + Eq + Hash + ToString,
{
    let levels = compute_levels(nodes)?;
    let mut sorted_nodes = nodes
        .iter()
        .filter(|node| {
            node.kind != NodeKind::PrimaryOutput
                && selected_nodes.is_none_or(|selected| selected.contains(&node.id))
                && (!critical_only || node.slack.rise <= threshold || node.slack.fall <= threshold)
        })
        .collect::<Vec<_>>();

    sorted_nodes.sort_by(|left, right| {
        levels[&left.id]
            .cmp(&levels[&right.id])
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });

    let mut buckets: Vec<LevelBucket<N>> = Vec::new();
    for node in sorted_nodes {
        let level = levels[&node.id];
        if buckets.last().is_none_or(|bucket| bucket.level != level) {
            buckets.push(LevelBucket {
                level,
                nodes: Vec::new(),
            });
        }
        buckets
            .last_mut()
            .expect("bucket was just inserted when missing")
            .nodes
            .push(node.id.clone());
    }

    let max_level = buckets.last().map_or(0, |bucket| bucket.level);
    Ok(PrintedLevelSummary { buckets, max_level })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CriticalLevel<N> {
    pub node: N,
    pub level: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CriticalLevelization<N> {
    pub levels: Vec<CriticalLevel<N>>,
    pub max_level: isize,
}

pub fn levelize_critical<N>(
    nodes: &[LevelNode<N>],
) -> Result<CriticalLevelization<N>, SpeedUtilError>
where
    N: Clone + Eq + Hash + ToString,
{
    let levels = compute_levels(nodes)?;
    let mut sorted_nodes = nodes
        .iter()
        .filter(|node| node.kind != NodeKind::PrimaryOutput && node.critical)
        .collect::<Vec<_>>();

    sorted_nodes.sort_by(|left, right| {
        levels[&left.id]
            .cmp(&levels[&right.id])
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });

    let mut max_level = -1;
    let mut critical = Vec::new();
    for node in sorted_nodes {
        let level = levels[&node.id];
        max_level = max_level.max(level as isize);
        critical.push(CriticalLevel {
            node: node.id.clone(),
            level,
        });
    }

    Ok(CriticalLevelization {
        levels: critical,
        max_level,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AreaNode {
    pub kind: NodeKind,
    pub gate_area: Option<f64>,
    pub literal_count: usize,
}

impl AreaNode {
    pub const fn internal(gate_area: Option<f64>, literal_count: usize) -> Self {
        Self {
            kind: NodeKind::Internal,
            gate_area,
            literal_count,
        }
    }
}

pub fn net_area(nodes: &[AreaNode], mapped: bool) -> f64 {
    nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Internal)
        .map(|node| {
            if mapped {
                node.gate_area.unwrap_or(0.0)
            } else {
                node.literal_count as f64
            }
        })
        .sum()
}

pub fn compute_critical_slack(output_slacks: &[DelayTime], threshold: f64) -> f64 {
    let min_output_slack = output_slacks
        .iter()
        .map(|time| time.min_edge())
        .fold(POS_LARGE, d_min);

    if min_output_slack > NSP_EPSILON {
        -1.0
    } else {
        min_output_slack + threshold
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SingleFaninAction<N> {
    CollapseIntoFanout { node: N, fanout: N },
    DeleteNode { node: N },
}

pub fn single_fanin_cleanup_actions<N: Clone>(
    node: N,
    fanin_count: usize,
    fanouts: &[N],
) -> Vec<SingleFaninAction<N>> {
    if fanin_count > 1 {
        return Vec::new();
    }

    let mut actions = fanouts
        .iter()
        .cloned()
        .map(|fanout| SingleFaninAction::CollapseIntoFanout {
            node: node.clone(),
            fanout,
        })
        .collect::<Vec<_>>();
    actions.push(SingleFaninAction::DeleteNode { node });
    actions
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateChoice {
    Smallest,
    Biggest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LibraryGate<N> {
    pub id: N,
    pub area: f64,
}

pub fn select_gate<N: Clone>(
    gates: &[LibraryGate<N>],
    choice: GateChoice,
) -> Result<LibraryGate<N>, SpeedUtilError> {
    let first = gates.first().ok_or(SpeedUtilError::EmptyGateClass)?;
    let mut selected = first;

    for gate in &gates[1..] {
        match choice {
            GateChoice::Smallest if gate.area < selected.area => selected = gate,
            GateChoice::Biggest if gate.area > selected.area => selected = gate,
            _ => {}
        }
    }

    Ok(selected.clone())
}

pub fn speed_get_stats_from_sis_network() -> Result<(), SpeedUtilError> {
    Err(SpeedUtilError::MissingSisPorts {
        operation: "speed_get_stats",
    })
}

pub fn delete_single_fanin_node_in_sis_network() -> Result<(), SpeedUtilError> {
    Err(SpeedUtilError::MissingSisPorts {
        operation: "speed_delete_single_fanin_node",
    })
}

pub fn delete_node_in_sis_network() -> Result<(), SpeedUtilError> {
    Err(SpeedUtilError::MissingSisPorts {
        operation: "speed_network_delete_node",
    })
}

pub fn library_buffer_from_sis_library() -> Result<(), SpeedUtilError> {
    Err(SpeedUtilError::MissingSisPorts {
        operation: "sp_lib_get_buffer",
    })
}

pub fn library_inverter_from_sis_library() -> Result<(), SpeedUtilError> {
    Err(SpeedUtilError::MissingSisPorts {
        operation: "sp_lib_get_inv",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(
        id: &'static str,
        kind: NodeKind,
        fanins: Vec<&'static str>,
        slack: DelayTime,
        critical: bool,
    ) -> LevelNode<&'static str> {
        LevelNode::new(id, kind, fanins, slack, critical)
    }

    #[test]
    fn constants_and_method_strings_match_speed_int_header() {
        assert_eq!(MIN_AREA_BUF_NAME, "");
        assert_eq!(ALONG_CRIT_PATH, 0);
        assert_eq!(TRANSITIVE_FANIN, 1);
        assert_eq!(COMPROMISE, 2);
        assert_eq!(ONLY_TREE, 3);
        assert_eq!(CLP, 0);
        assert_eq!(FAN, 1);
        assert_eq!(DUAL, 2);
        assert_eq!(AREA_BASED, 0);
        assert_eq!(TRANSFORM_BASED, 1);
        assert_eq!(BEST_BENEFIT, 0);
        assert_eq!(BEST_BANG_FOR_BUCK, 1);
        assert_eq!(DEFAULT_SPEED_THRESH, 0.5);
        assert_eq!(DEFAULT_SPEED_COEFF, 0.0);
        assert_eq!(DEFAULT_SPEED_DIST, 3);
        assert_eq!(NSP_EPSILON, 1.0e-6);
        assert_eq!(NSP_INPUT_SEPARATOR, '#');
        assert_eq!(NSP_OUTPUT_SEPARATOR, '%');
        assert_eq!(POS_LARGE, 10_000.0);
        assert_eq!(NEG_LARGE, -10_000.0);
        assert_eq!(MAXWEIGHT, 1_000);
        assert_eq!(speed_method_name(ALONG_CRIT_PATH), "CRITICAL");
        assert_eq!(speed_method_name(TRANSITIVE_FANIN), "TRANSITIVE");
        assert_eq!(speed_method_name(COMPROMISE), "COMPROMISE");
        assert_eq!(speed_method_name(ONLY_TREE), "TREE");
        assert_eq!(speed_method_name(99), "UNKNOWN");
    }

    #[test]
    fn min_max_improvement_and_threshold_formulas_match_c_macros() {
        assert_eq!(d_min(3.0, -2.0), -2.0);
        assert_eq!(d_max(3.0, -2.0), 3.0);
        assert!(speed_improved(true, 2.0 + NSP_EPSILON * 2.0, 2.0));
        assert!(!speed_improved(true, 2.0 + NSP_EPSILON / 2.0, 2.0));
        assert!(speed_improved(false, 2.0 - NSP_EPSILON * 2.0, 2.0));
        assert!(!speed_improved(false, 2.0 - NSP_EPSILON / 2.0, 2.0));
        assert_eq!(speed_performance_label(true), "Slack");
        assert_eq!(speed_performance_label(false), "Delay");

        assert_eq!(
            compute_critical_slack(&[DelayTime::new(0.1, 0.3), DelayTime::new(0.2, 0.4)], 0.5),
            -1.0
        );
        assert_eq!(
            compute_critical_slack(&[DelayTime::new(-0.2, 0.3), DelayTime::new(0.0, 0.4)], 0.5),
            0.3
        );
    }

    #[test]
    fn decodes_cube_literals_and_filters_critical_cubes() {
        assert_eq!(
            decode_node_cube(&[
                LiteralValue::One,
                LiteralValue::DontCare,
                LiteralValue::Zero,
            ]),
            DecodedCube {
                literals: vec![
                    DecodedLiteral {
                        fanin_index: 0,
                        phase: true,
                    },
                    DecodedLiteral {
                        fanin_index: 2,
                        phase: false,
                    },
                ],
            }
        );

        let cubes = [
            CubeTiming {
                cube_index: 0,
                fanin_arrivals: vec![DelayTime::new(1.0, 2.0)],
            },
            CubeTiming {
                cube_index: 1,
                fanin_arrivals: vec![DelayTime::new(4.0, 2.0)],
            },
            CubeTiming {
                cube_index: 2,
                fanin_arrivals: Vec::new(),
            },
        ];

        assert_eq!(noncritical_cube_indices(&cubes, 3.0), vec![0, 2]);
    }

    #[test]
    fn name_lookup_matches_first_fanin_with_long_name() {
        let fanins = [
            NamedFanin {
                name: "a",
                node: 10,
            },
            NamedFanin {
                name: "b",
                node: 11,
            },
        ];

        assert_eq!(name_to_fanin(&fanins, "b"), Some(11));
        assert_eq!(name_to_fanin(&fanins, "missing"), None);
    }

    #[test]
    fn levelization_assigns_from_primary_outputs_like_recursive_c_helper() {
        let nodes = [
            node(
                "a",
                NodeKind::PrimaryInput,
                vec![],
                DelayTime::new(9.0, 9.0),
                false,
            ),
            node(
                "b",
                NodeKind::PrimaryInput,
                vec![],
                DelayTime::new(9.0, 9.0),
                false,
            ),
            node(
                "n1",
                NodeKind::Internal,
                vec!["a", "b"],
                DelayTime::new(0.1, 2.0),
                true,
            ),
            node(
                "n2",
                NodeKind::Internal,
                vec!["n1"],
                DelayTime::new(4.0, 4.0),
                false,
            ),
            node(
                "po",
                NodeKind::PrimaryOutput,
                vec!["n2"],
                DelayTime::new(0.0, 0.0),
                true,
            ),
            node(
                "dead",
                NodeKind::Internal,
                vec![],
                DelayTime::new(0.0, 0.0),
                true,
            ),
        ];

        let levels = compute_levels(&nodes).unwrap();
        assert_eq!(levels["a"], 0);
        assert_eq!(levels["n1"], 1);
        assert_eq!(levels["n2"], 2);
        assert_eq!(levels["po"], 3);
        assert_eq!(levels["dead"], 0);

        let printed = printable_levels(&nodes, None, true, 0.5).unwrap();
        assert_eq!(
            printed,
            PrintedLevelSummary {
                buckets: vec![
                    LevelBucket {
                        level: 0,
                        nodes: vec!["dead"],
                    },
                    LevelBucket {
                        level: 1,
                        nodes: vec!["n1"],
                    },
                ],
                max_level: 1,
            }
        );
    }

    #[test]
    fn critical_levelization_excludes_outputs_and_reports_minus_one_when_empty() {
        let nodes = [
            node(
                "a",
                NodeKind::PrimaryInput,
                vec![],
                DelayTime::new(0.0, 0.0),
                false,
            ),
            node(
                "n1",
                NodeKind::Internal,
                vec!["a"],
                DelayTime::new(0.0, 0.0),
                true,
            ),
            node(
                "n2",
                NodeKind::Internal,
                vec!["n1"],
                DelayTime::new(0.0, 0.0),
                true,
            ),
            node(
                "po",
                NodeKind::PrimaryOutput,
                vec!["n2"],
                DelayTime::new(0.0, 0.0),
                true,
            ),
        ];

        assert_eq!(
            levelize_critical(&nodes).unwrap(),
            CriticalLevelization {
                levels: vec![
                    CriticalLevel {
                        node: "n1",
                        level: 1,
                    },
                    CriticalLevel {
                        node: "n2",
                        level: 2,
                    },
                ],
                max_level: 2,
            }
        );

        let empty = [node(
            "po",
            NodeKind::PrimaryOutput,
            vec![],
            DelayTime::new(0.0, 0.0),
            true,
        )];
        assert_eq!(
            levelize_critical(&empty).unwrap(),
            CriticalLevelization::<&str> {
                levels: Vec::new(),
                max_level: -1,
            }
        );
    }

    #[test]
    fn area_gate_and_single_fanin_helpers_match_c_decisions() {
        let area_nodes = [
            AreaNode {
                kind: NodeKind::PrimaryInput,
                gate_area: Some(99.0),
                literal_count: 99,
            },
            AreaNode::internal(Some(2.5), 7),
            AreaNode::internal(Some(3.0), 4),
        ];
        assert_eq!(net_area(&area_nodes, true), 5.5);
        assert_eq!(net_area(&area_nodes, false), 11.0);

        let gates = [
            LibraryGate {
                id: "slow",
                area: 3.0,
            },
            LibraryGate {
                id: "small",
                area: 1.0,
            },
            LibraryGate {
                id: "big",
                area: 4.0,
            },
        ];
        assert_eq!(
            select_gate(&gates, GateChoice::Smallest).unwrap(),
            LibraryGate {
                id: "small",
                area: 1.0
            }
        );
        assert_eq!(
            select_gate(&gates, GateChoice::Biggest).unwrap(),
            LibraryGate {
                id: "big",
                area: 4.0
            }
        );
        assert_eq!(
            single_fanin_cleanup_actions("n", 1, &["fo1", "fo2"]),
            vec![
                SingleFaninAction::CollapseIntoFanout {
                    node: "n",
                    fanout: "fo1",
                },
                SingleFaninAction::CollapseIntoFanout {
                    node: "n",
                    fanout: "fo2",
                },
                SingleFaninAction::DeleteNode { node: "n" },
            ]
        );
        assert!(single_fanin_cleanup_actions("n", 2, &["fo"]).is_empty());
    }

    #[test]
    fn sis_bound_entry_points_report_explicit_missing_dependencies() {
        assert_eq!(
            speed_get_stats_from_sis_network(),
            Err(SpeedUtilError::MissingSisPorts {
                operation: "speed_get_stats",
            })
        );
        assert_eq!(
            delete_single_fanin_node_in_sis_network(),
            Err(SpeedUtilError::MissingSisPorts {
                operation: "speed_delete_single_fanin_node",
            })
        );
        assert_eq!(
            library_buffer_from_sis_library(),
            Err(SpeedUtilError::MissingSisPorts {
                operation: "sp_lib_get_buffer",
            })
        );
    }
}
