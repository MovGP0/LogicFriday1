//! Native Rust scaffold for `LogicSynthesis/sis/simplify/simp_util.c`.
//!
//! The original C module mixes three kinds of behavior:
//! - building a temporary expression copy of the external don't-care network,
//! - finding the copied don't-care expression for a care-network output,
//! - sorting nodes for eliminate/simplify passes.
//!
//! The sorting and expression-copy behavior are represented here over owned
//! Rust data. Entry points that require mutating legacy `network_t`, `node_t`,
//! `array_t`, `st_table`, CSPF/ODC slots, or BDD state report explicit missing
//! native-port dependencies instead of exposing C ABI shims.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub note: &'static str,
}

pub const REQUIRED_PORT_BEADS: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        note: "array_alloc, array_fetch, array_insert_last, array_sort, and array_free",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.195",
        source_file: "LogicSynthesis/sis/factor/factor.c",
        note: "factor_num_literal used by simp_order",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.297",
        source_file: "LogicSynthesis/sis/network/dfs.c",
        note: "network_dfs and network_dfs_from_input traversal order",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        note: "network_dc_network, network_num_pi, and network_get_pi",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        note: "node_get_fanin traversal",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        note: "node constants, literals, AND/OR construction, functions, covers, and cube counts",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.321",
        source_file: "LogicSynthesis/sis/node/nodemisc.c",
        note: "node_dup for copied external don't-care expressions",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.326",
        source_file: "LogicSynthesis/sis/ntbdd/bdd_at_node.c",
        note: "ntbdd_free_at_node cleanup for temporary copied nodes",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.447",
        source_file: "LogicSynthesis/sis/simplify/compute_dc.c",
        note: "cspf_alloc/free, odc_alloc/free, odc_value, and find_odc_level",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        note: "st_lookup for care-output to external-DC-output lookup",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimplifyNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CubeLiteral {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverCube {
    pub inputs: Vec<CubeLiteral>,
}

impl CoverCube {
    pub fn new(inputs: Vec<CubeLiteral>) -> Self {
        Self { inputs }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplifyNode<N> {
    pub id: N,
    pub name: String,
    pub kind: SimplifyNodeKind,
    pub fanins: Vec<N>,
    pub cover: Vec<CoverCube>,
    pub odc_level: i32,
    pub odc_value: i32,
    pub factor_literal_count: usize,
}

impl<N> SimplifyNode<N> {
    pub fn primary_input(id: N, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            kind: SimplifyNodeKind::PrimaryInput,
            fanins: Vec::new(),
            cover: Vec::new(),
            odc_level: 0,
            odc_value: 0,
            factor_literal_count: 0,
        }
    }

    pub fn primary_output(id: N, name: impl Into<String>, fanin: N) -> Self {
        Self {
            id,
            name: name.into(),
            kind: SimplifyNodeKind::PrimaryOutput,
            fanins: vec![fanin],
            cover: Vec::new(),
            odc_level: 0,
            odc_value: 0,
            factor_literal_count: 0,
        }
    }

    pub fn internal(id: N, name: impl Into<String>, fanins: Vec<N>, cover: Vec<CoverCube>) -> Self {
        Self {
            id,
            name: name.into(),
            kind: SimplifyNodeKind::Internal,
            fanins,
            cover,
            odc_level: 0,
            odc_value: 0,
            factor_literal_count: 0,
        }
    }

    pub fn cube_count(&self) -> usize {
        self.cover.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplifyNetwork<N> {
    pub nodes: Vec<SimplifyNode<N>>,
}

impl<N> SimplifyNetwork<N> {
    pub fn new(nodes: Vec<SimplifyNode<N>>) -> Self {
        Self { nodes }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BooleanExpr<N> {
    Const(bool),
    Literal { node: N, phase: bool },
    Not(Box<BooleanExpr<N>>),
    And(Vec<BooleanExpr<N>>),
    Or(Vec<BooleanExpr<N>>),
}

impl<N> BooleanExpr<N> {
    fn and_with(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::Const(false), _) | (_, Self::Const(false)) => Self::Const(false),
            (Self::Const(true), expr) | (expr, Self::Const(true)) => expr,
            (Self::And(mut left), Self::And(right)) => {
                left.extend(right);
                Self::And(left)
            }
            (Self::And(mut left), right) => {
                left.push(right);
                Self::And(left)
            }
            (left, Self::And(mut right)) => {
                let mut terms = vec![left];
                terms.append(&mut right);
                Self::And(terms)
            }
            (left, right) => Self::And(vec![left, right]),
        }
    }

    fn or_with(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::Const(true), _) | (_, Self::Const(true)) => Self::Const(true),
            (Self::Const(false), expr) | (expr, Self::Const(false)) => expr,
            (Self::Or(mut left), Self::Or(right)) => {
                left.extend(right);
                Self::Or(left)
            }
            (Self::Or(mut left), right) => {
                left.push(right);
                Self::Or(left)
            }
            (left, Self::Or(mut right)) => {
                let mut terms = vec![left];
                terms.append(&mut right);
                Self::Or(terms)
            }
            (left, right) => Self::Or(vec![left, right]),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DcNetworkCopy<N: Eq + Hash> {
    expressions: HashMap<N, BooleanExpr<N>>,
}

impl<N> DcNetworkCopy<N>
where
    N: Clone + Eq + Hash,
{
    pub fn expression(&self, node: &N) -> Option<&BooleanExpr<N>> {
        self.expressions.get(node)
    }

    pub fn into_expressions(self) -> HashMap<N, BooleanExpr<N>> {
        self.expressions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimpUtilError {
    MissingSisPorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
    MissingDcFanin {
        node: String,
        fanin_index: usize,
    },
    MissingCarePrimaryInput {
        dc_input_name: String,
    },
    PrimaryOutputWithoutFanin {
        node: String,
    },
}

impl fmt::Display for SimpUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSisPorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS dependencies",
                dependencies.len()
            ),
            Self::MissingDcFanin { node, fanin_index } => {
                write!(
                    f,
                    "DC node {node} references missing copied fanin #{fanin_index}"
                )
            }
            Self::MissingCarePrimaryInput { dc_input_name } => write!(
                f,
                "DC primary input {dc_input_name} has no matching care-network primary input"
            ),
            Self::PrimaryOutputWithoutFanin { node } => {
                write!(f, "DC primary output {node} has no fanin")
            }
        }
    }
}

impl Error for SimpUtilError {}

pub fn required_port_beads() -> &'static [PortDependency] {
    REQUIRED_PORT_BEADS
}

pub fn copy_dcnetwork_model<N>(
    care_network: Option<&SimplifyNetwork<N>>,
    dc_network: Option<&SimplifyNetwork<N>>,
) -> Result<Option<DcNetworkCopy<N>>, SimpUtilError>
where
    N: Clone + Eq + Hash,
{
    let Some(care_network) = care_network else {
        return Ok(None);
    };
    let Some(dc_network) = dc_network else {
        return Ok(None);
    };

    let care_inputs = care_network
        .nodes
        .iter()
        .filter(|node| node.kind == SimplifyNodeKind::PrimaryInput)
        .map(|node| (node.name.as_str(), node.id.clone()))
        .collect::<HashMap<_, _>>();

    let mut expressions = HashMap::new();
    for dc_node in &dc_network.nodes {
        let expression = match dc_node.kind {
            SimplifyNodeKind::PrimaryInput => {
                let Some(care_input) = care_inputs.get(dc_node.name.as_str()) else {
                    return Err(SimpUtilError::MissingCarePrimaryInput {
                        dc_input_name: dc_node.name.clone(),
                    });
                };
                BooleanExpr::Literal {
                    node: care_input.clone(),
                    phase: true,
                }
            }
            SimplifyNodeKind::PrimaryOutput => {
                let Some(fanin) = dc_node.fanins.first() else {
                    return Err(SimpUtilError::PrimaryOutputWithoutFanin {
                        node: dc_node.name.clone(),
                    });
                };
                expressions
                    .get(fanin)
                    .cloned()
                    .ok_or_else(|| SimpUtilError::MissingDcFanin {
                        node: dc_node.name.clone(),
                        fanin_index: 0,
                    })?
            }
            SimplifyNodeKind::Internal => copy_dc_internal_node(dc_node, &expressions)?,
        };
        expressions.insert(dc_node.id.clone(), expression);
    }

    Ok(Some(DcNetworkCopy { expressions }))
}

fn copy_dc_internal_node<N>(
    dc_node: &SimplifyNode<N>,
    expressions: &HashMap<N, BooleanExpr<N>>,
) -> Result<BooleanExpr<N>, SimpUtilError>
where
    N: Clone + Eq + Hash,
{
    let mut node_expr = BooleanExpr::Const(false);
    for cube in &dc_node.cover {
        let mut cube_expr = BooleanExpr::Const(true);
        for (fanin_index, literal) in cube.inputs.iter().copied().enumerate() {
            let phase = match literal {
                CubeLiteral::Zero => false,
                CubeLiteral::One => true,
                CubeLiteral::DontCare => continue,
            };
            let fanin =
                dc_node
                    .fanins
                    .get(fanin_index)
                    .ok_or_else(|| SimpUtilError::MissingDcFanin {
                        node: dc_node.name.clone(),
                        fanin_index,
                    })?;
            let fanin_expr =
                expressions
                    .get(fanin)
                    .cloned()
                    .ok_or_else(|| SimpUtilError::MissingDcFanin {
                        node: dc_node.name.clone(),
                        fanin_index,
                    })?;
            cube_expr = BooleanExpr::and_with(cube_expr, apply_phase(fanin_expr, phase));
        }
        node_expr = BooleanExpr::or_with(cube_expr, node_expr);
    }
    Ok(node_expr)
}

fn apply_phase<N>(expr: BooleanExpr<N>, phase: bool) -> BooleanExpr<N> {
    match expr {
        BooleanExpr::Literal { node, .. } => BooleanExpr::Literal { node, phase },
        other => {
            if phase {
                other
            } else {
                BooleanExpr::Not(Box::new(other))
            }
        }
    }
}

pub fn find_node_exdc_model<N>(
    ponode: &N,
    node_exdc_table: Option<&HashMap<N, N>>,
    dc_copy: &DcNetworkCopy<N>,
) -> BooleanExpr<N>
where
    N: Clone + Eq + Hash,
{
    let Some(table) = node_exdc_table else {
        return BooleanExpr::Const(false);
    };
    let Some(dc_po) = table.get(ponode) else {
        return BooleanExpr::Const(false);
    };
    match dc_copy.expression(dc_po) {
        Some(BooleanExpr::Literal { node, .. }) => BooleanExpr::Literal {
            node: node.clone(),
            phase: true,
        },
        Some(expression) => expression.clone(),
        None => BooleanExpr::Const(false),
    }
}

pub fn free_dcnetwork_copy_model<N: Eq + Hash>(_copy: DcNetworkCopy<N>) {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElimOrderEntry<N> {
    pub node: N,
    pub odc_value: i32,
    pub odc_level: i32,
    pub original_order: usize,
}

pub fn order_nodes_elim_model<N>(nodes_from_input_dfs: &[SimplifyNode<N>]) -> Vec<ElimOrderEntry<N>>
where
    N: Clone,
{
    let mut entries = nodes_from_input_dfs
        .iter()
        .enumerate()
        .filter(|(_, node)| node.kind != SimplifyNodeKind::PrimaryOutput)
        .map(|(original_order, node)| ElimOrderEntry {
            node: node.id.clone(),
            odc_value: node.odc_value,
            odc_level: node.odc_level,
            original_order,
        })
        .collect::<Vec<_>>();

    entries.sort_by(level_elim_cmp);
    entries
}

pub fn level_elim_cmp<N>(left: &ElimOrderEntry<N>, right: &ElimOrderEntry<N>) -> Ordering {
    left.odc_level
        .cmp(&right.odc_level)
        .then_with(|| right.original_order.cmp(&left.original_order))
}

pub fn num_cube_cmp<N>(left: &SimplifyNode<N>, right: &SimplifyNode<N>) -> Ordering {
    left.cube_count().cmp(&right.cube_count())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorSizeEntry<N> {
    pub node: N,
    pub factor_literal_count: usize,
}

pub fn simp_order_model<N>(nodes: &[SimplifyNode<N>]) -> Vec<N>
where
    N: Clone,
{
    let mut entries = nodes
        .iter()
        .map(|node| FactorSizeEntry {
            node: node.id.clone(),
            factor_literal_count: node.factor_literal_count,
        })
        .collect::<Vec<_>>();
    entries.sort_by(fsize_cmp);
    entries.into_iter().map(|entry| entry.node).collect()
}

pub fn fsize_cmp<N>(left: &FactorSizeEntry<N>, right: &FactorSizeEntry<N>) -> Ordering {
    right.factor_literal_count.cmp(&left.factor_literal_count)
}

pub fn copy_dcnetwork_in_sis_network() -> Result<(), SimpUtilError> {
    Err(SimpUtilError::MissingSisPorts {
        operation: "copy_dcnetwork",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn find_node_exdc_in_sis_network() -> Result<(), SimpUtilError> {
    Err(SimpUtilError::MissingSisPorts {
        operation: "find_node_exdc",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn free_dcnetwork_copy_in_sis_network() -> Result<(), SimpUtilError> {
    Err(SimpUtilError::MissingSisPorts {
        operation: "free_dcnetwork_copy",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

pub fn order_nodes_elim_in_sis_network() -> Result<(), SimpUtilError> {
    Err(SimpUtilError::MissingSisPorts {
        operation: "order_nodes_elim",
        dependencies: REQUIRED_PORT_BEADS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(inputs: &[CubeLiteral]) -> CoverCube {
        CoverCube::new(inputs.to_vec())
    }

    fn node_with_metrics(
        id: &'static str,
        kind: SimplifyNodeKind,
        cubes: usize,
        odc_level: i32,
        odc_value: i32,
        factor_literal_count: usize,
    ) -> SimplifyNode<&'static str> {
        let mut node = SimplifyNode {
            id,
            name: id.to_string(),
            kind,
            fanins: Vec::new(),
            cover: vec![cube(&[]); cubes],
            odc_level,
            odc_value,
            factor_literal_count,
        };
        if kind == SimplifyNodeKind::PrimaryOutput {
            node.fanins.push("fanin");
        }
        node
    }

    #[test]
    fn copy_dcnetwork_returns_none_for_missing_networks_like_c_noops() {
        let care = SimplifyNetwork::new(vec![SimplifyNode::primary_input("a", "a")]);

        assert_eq!(copy_dcnetwork_model::<&str>(None, Some(&care)), Ok(None));
        assert_eq!(copy_dcnetwork_model(Some(&care), None), Ok(None));
    }

    #[test]
    fn copy_dcnetwork_maps_dc_inputs_and_builds_sum_of_products() {
        let care = SimplifyNetwork::new(vec![
            SimplifyNode::primary_input("care_a", "a"),
            SimplifyNode::primary_input("care_b", "b"),
        ]);
        let dc = SimplifyNetwork::new(vec![
            SimplifyNode::primary_input("dc_a", "a"),
            SimplifyNode::primary_input("dc_b", "b"),
            SimplifyNode::internal(
                "dc_n",
                "n",
                vec!["dc_a", "dc_b"],
                vec![
                    cube(&[CubeLiteral::One, CubeLiteral::Zero]),
                    cube(&[CubeLiteral::DontCare, CubeLiteral::One]),
                ],
            ),
            SimplifyNode::primary_output("dc_po", "po", "dc_n"),
        ]);

        let copy = copy_dcnetwork_model(Some(&care), Some(&dc))
            .unwrap()
            .expect("DC network should be copied");

        assert_eq!(
            copy.expression(&"dc_n"),
            Some(&BooleanExpr::Or(vec![
                BooleanExpr::Literal {
                    node: "care_b",
                    phase: true,
                },
                BooleanExpr::And(vec![
                    BooleanExpr::Literal {
                        node: "care_a",
                        phase: true,
                    },
                    BooleanExpr::Literal {
                        node: "care_b",
                        phase: false,
                    },
                ]),
            ]))
        );
        assert_eq!(copy.expression(&"dc_po"), copy.expression(&"dc_n"));
    }

    #[test]
    fn copy_dcnetwork_reports_missing_dc_primary_input_match() {
        let care = SimplifyNetwork::new(vec![SimplifyNode::primary_input("care_a", "a")]);
        let dc = SimplifyNetwork::new(vec![SimplifyNode::primary_input("dc_b", "b")]);

        assert_eq!(
            copy_dcnetwork_model(Some(&care), Some(&dc)),
            Err(SimpUtilError::MissingCarePrimaryInput {
                dc_input_name: "b".to_string(),
            })
        );
    }

    #[test]
    fn find_node_exdc_returns_zero_for_missing_table_or_mapping() {
        let care = SimplifyNetwork::new(vec![SimplifyNode::primary_input("care_a", "a")]);
        let dc = SimplifyNetwork::new(vec![SimplifyNode::primary_input("dc_a", "a")]);
        let copy = copy_dcnetwork_model(Some(&care), Some(&dc))
            .unwrap()
            .expect("DC network should be copied");

        assert_eq!(
            find_node_exdc_model(&"po", None, &copy),
            BooleanExpr::Const(false)
        );
        assert_eq!(
            find_node_exdc_model(&"po", Some(&HashMap::new()), &copy),
            BooleanExpr::Const(false)
        );
    }

    #[test]
    fn find_node_exdc_rebuilds_primary_input_literal_with_positive_phase() {
        let care = SimplifyNetwork::new(vec![SimplifyNode::primary_input("care_a", "a")]);
        let dc = SimplifyNetwork::new(vec![SimplifyNode::primary_input("dc_a", "a")]);
        let copy = copy_dcnetwork_model(Some(&care), Some(&dc))
            .unwrap()
            .expect("DC network should be copied");
        let table = HashMap::from([("care_po", "dc_a")]);

        assert_eq!(
            find_node_exdc_model(&"care_po", Some(&table), &copy),
            BooleanExpr::Literal {
                node: "care_a",
                phase: true,
            }
        );
    }

    #[test]
    fn order_nodes_elim_excludes_outputs_and_matches_c_comparator() {
        let nodes = vec![
            node_with_metrics("a", SimplifyNodeKind::PrimaryInput, 0, 2, 10, 0),
            node_with_metrics("b", SimplifyNodeKind::Internal, 0, 1, 20, 0),
            node_with_metrics("c", SimplifyNodeKind::Internal, 0, 1, 30, 0),
            node_with_metrics("po", SimplifyNodeKind::PrimaryOutput, 0, 0, 40, 0),
            node_with_metrics("d", SimplifyNodeKind::Internal, 0, 3, 50, 0),
        ];

        let ordered = order_nodes_elim_model(&nodes);

        assert_eq!(
            ordered,
            vec![
                ElimOrderEntry {
                    node: "c",
                    odc_value: 30,
                    odc_level: 1,
                    original_order: 2,
                },
                ElimOrderEntry {
                    node: "b",
                    odc_value: 20,
                    odc_level: 1,
                    original_order: 1,
                },
                ElimOrderEntry {
                    node: "a",
                    odc_value: 10,
                    odc_level: 2,
                    original_order: 0,
                },
                ElimOrderEntry {
                    node: "d",
                    odc_value: 50,
                    odc_level: 3,
                    original_order: 4,
                },
            ]
        );
    }

    #[test]
    fn num_cube_cmp_sorts_ascending_by_cover_cube_count() {
        let mut nodes = vec![
            node_with_metrics("three", SimplifyNodeKind::Internal, 3, 0, 0, 0),
            node_with_metrics("one", SimplifyNodeKind::Internal, 1, 0, 0, 0),
            node_with_metrics("two", SimplifyNodeKind::Internal, 2, 0, 0, 0),
        ];

        nodes.sort_by(num_cube_cmp);

        assert_eq!(
            nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
            vec!["one", "two", "three",]
        );
    }

    #[test]
    fn simp_order_sorts_descending_by_factor_literal_count() {
        let nodes = vec![
            node_with_metrics("small", SimplifyNodeKind::Internal, 0, 0, 0, 3),
            node_with_metrics("big", SimplifyNodeKind::Internal, 0, 0, 0, 9),
            node_with_metrics("middle", SimplifyNodeKind::Internal, 0, 0, 0, 5),
        ];

        assert_eq!(simp_order_model(&nodes), vec!["big", "middle", "small"]);
    }

    #[test]
    fn sis_bound_entry_points_report_dependency_beads_and_sources() {
        assert!(required_port_beads().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.447"
                && dependency.source_file == "LogicSynthesis/sis/simplify/compute_dc.c"
        }));
        assert!(required_port_beads().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.318"
                && dependency.source_file == "LogicSynthesis/sis/node/node.c"
        }));

        assert_eq!(
            copy_dcnetwork_in_sis_network(),
            Err(SimpUtilError::MissingSisPorts {
                operation: "copy_dcnetwork",
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
        assert_eq!(
            order_nodes_elim_in_sis_network(),
            Err(SimpUtilError::MissingSisPorts {
                operation: "order_nodes_elim",
                dependencies: REQUIRED_PORT_BEADS,
            })
        );
    }
}
