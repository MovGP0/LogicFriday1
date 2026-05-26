//! Native Rust model for `LogicSynthesis/sis/sim/codegen.c`.
//!
//! The legacy C file generated a temporary C program for two SIS networks,
//! compiled it, and ran `driver.c` to compare random packed-word simulations.
//! This port keeps the deterministic parts native: PI/PO name mapping,
//! code-size metadata, node-index assignment, and C-like expression generation
//! from an owned Rust network model. Direct `network_t`/`node_t` integration and
//! the temporary C compiler/driver path are intentionally represented as
//! explicit dependency errors until those native SIS ports exist.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimNodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimNodeFunction {
    Zero,
    One,
    SumOfProducts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimLiteral {
    Zero,
    One,
    DontCare,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimCube {
    pub literals: Vec<SimLiteral>,
}

impl SimCube {
    pub fn new(literals: Vec<SimLiteral>) -> Self {
        Self { literals }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimNode<N> {
    pub id: N,
    pub name: String,
    pub kind: SimNodeKind,
    pub function: SimNodeFunction,
    pub fanins: Vec<N>,
    pub cubes: Vec<SimCube>,
}

impl<N> SimNode<N> {
    pub fn primary_input(id: N, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            kind: SimNodeKind::PrimaryInput,
            function: SimNodeFunction::SumOfProducts,
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }

    pub fn primary_output(id: N, name: impl Into<String>, fanin: N) -> Self {
        Self {
            id,
            name: name.into(),
            kind: SimNodeKind::PrimaryOutput,
            function: SimNodeFunction::SumOfProducts,
            fanins: vec![fanin],
            cubes: Vec::new(),
        }
    }

    pub fn internal(id: N, name: impl Into<String>, fanins: Vec<N>, cubes: Vec<SimCube>) -> Self {
        Self {
            id,
            name: name.into(),
            kind: SimNodeKind::Internal,
            function: SimNodeFunction::SumOfProducts,
            fanins,
            cubes,
        }
    }

    pub fn constant(id: N, name: impl Into<String>, value: bool) -> Self {
        Self {
            id,
            name: name.into(),
            kind: SimNodeKind::Internal,
            function: if value {
                SimNodeFunction::One
            } else {
                SimNodeFunction::Zero
            },
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimNetwork<N> {
    pub nodes: Vec<SimNode<N>>,
    pub dfs_order: Vec<N>,
}

impl<N> SimNetwork<N> {
    pub fn new(nodes: Vec<SimNode<N>>, dfs_order: Vec<N>) -> Self {
        Self { nodes, dfs_order }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodegenPlan {
    pub nin: usize,
    pub nout: usize,
    pub nodes1: usize,
    pub nodes2: usize,
    pub input_map: Vec<usize>,
    pub output_map: Vec<usize>,
    pub output_names: Vec<String>,
    pub func1: String,
    pub func2: String,
}

impl CodegenPlan {
    pub fn generated_source(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("#define nin {}\n", self.nin));
        output.push_str(&format!("#define nout {}\n", self.nout));
        output.push_str(&format!("#define nodes1 {}\n", self.nodes1));
        output.push_str(&format!("#define nodes2 {}\n", self.nodes2));
        output.push_str(&self.func1);
        output.push_str(&self.func2);
        output.push_str(&format!("char *output_names[{}] = {{\n", self.nout));
        for name in &self.output_names {
            output.push_str(&format!("    \"{}\",\n", escape_c_string(name)));
        }
        output.push_str("};\n");
        output.push_str(&format!("int input_map[{}] = {{\n", self.nin));
        for index in &self.input_map {
            output.push_str(&format!("    {index},\n"));
        }
        output.push_str("};\n");
        output.push_str(&format!("int output_map[{}] = {{\n", self.nout));
        for index in &self.output_map {
            output.push_str(&format!("    {index},\n"));
        }
        output.push_str("};\n");
        output
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodegenError {
    InputCountMismatch { left: usize, right: usize },
    OutputCountMismatch { left: usize, right: usize },
    MissingInputMatch { name: String },
    MissingOutputMatch { name: String },
    DuplicateNodeId,
    MissingNodeInDfsOrder,
    MissingFanin { node: String, fanin_index: usize },
    MissingOutputFanin { node: String },
    MissingSisPorts { operation: &'static str },
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputCountMismatch { left, right } => {
                write!(f, "number of inputs do not agree: {left} != {right}")
            }
            Self::OutputCountMismatch { left, right } => {
                write!(f, "number of outputs do not agree: {left} != {right}")
            }
            Self::MissingInputMatch { name } => {
                write!(f, "no match for input '{name}' in network2")
            }
            Self::MissingOutputMatch { name } => {
                write!(f, "no match for output '{name}' in network2")
            }
            Self::DuplicateNodeId => write!(f, "network contains duplicate node ids"),
            Self::MissingNodeInDfsOrder => write!(f, "DFS order references a missing node"),
            Self::MissingFanin { node, fanin_index } => {
                write!(f, "node {node} references missing fanin #{fanin_index}")
            }
            Self::MissingOutputFanin { node } => {
                write!(f, "primary output {node} has no fanin")
            }
            Self::MissingSisPorts { operation } => write!(
                f,
                "{operation} requires native SIS prerequisite ports that are not available yet"
            ),
        }
    }
}

impl Error for CodegenError {}

pub fn build_codegen_plan<N>(
    network1: &SimNetwork<N>,
    network2: &SimNetwork<N>,
) -> Result<CodegenPlan, CodegenError>
where
    N: Clone + Eq + Hash,
{
    let inputs1 = nodes_of_kind(network1, SimNodeKind::PrimaryInput)?;
    let inputs2 = nodes_of_kind(network2, SimNodeKind::PrimaryInput)?;
    if inputs1.len() != inputs2.len() {
        return Err(CodegenError::InputCountMismatch {
            left: inputs1.len(),
            right: inputs2.len(),
        });
    }

    let outputs1 = nodes_of_kind(network1, SimNodeKind::PrimaryOutput)?;
    let outputs2 = nodes_of_kind(network2, SimNodeKind::PrimaryOutput)?;
    if outputs1.len() != outputs2.len() {
        return Err(CodegenError::OutputCountMismatch {
            left: outputs1.len(),
            right: outputs2.len(),
        });
    }

    let input_map =
        map_by_names(&inputs1, &inputs2).map_err(|name| CodegenError::MissingInputMatch {
            name: name.to_string(),
        })?;
    let output_map =
        map_by_names(&outputs1, &outputs2).map_err(|name| CodegenError::MissingOutputMatch {
            name: name.to_string(),
        })?;
    let nin = inputs1.len();
    let nout = outputs2.len();

    Ok(CodegenPlan {
        nin,
        nout,
        nodes1: internal_count(network1)? + nin + nout + 10,
        nodes2: internal_count(network2)? + nin + nout + 10,
        input_map,
        output_map,
        output_names: outputs1.iter().map(|node| node.name.clone()).collect(),
        func1: generate_function(network1, "func1")?,
        func2: generate_function(network2, "func2")?,
    })
}

pub fn generate_function<N>(
    network: &SimNetwork<N>,
    function_name: &str,
) -> Result<String, CodegenError>
where
    N: Clone + Eq + Hash,
{
    let index = node_index(network)?;
    let nodes = nodes_by_id(network)?;
    let mut output = format!("void {function_name}(a)\nregister unsigned *a;\n{{\n");

    for node_id in &network.dfs_order {
        let Some(node) = nodes.get(node_id) else {
            return Err(CodegenError::MissingNodeInDfsOrder);
        };

        match node.kind {
            SimNodeKind::Internal => {
                output.push_str(&format!(
                    "    a[{}] = {};\n",
                    index[node_id],
                    node_expression(node, &index)?
                ));
            }
            SimNodeKind::PrimaryOutput => {
                let fanin =
                    node.fanins
                        .first()
                        .ok_or_else(|| CodegenError::MissingOutputFanin {
                            node: node.name.clone(),
                        })?;
                let Some(fanin_index) = index.get(fanin) else {
                    return Err(CodegenError::MissingFanin {
                        node: node.name.clone(),
                        fanin_index: 0,
                    });
                };
                output.push_str(&format!(
                    "    a[{}] = a[{}];\n",
                    index[node_id], fanin_index
                ));
            }
            SimNodeKind::PrimaryInput => {}
        }
    }

    output.push_str("}\n");
    Ok(output)
}

pub fn verify_codegen_in_sis_networks() -> Result<(), CodegenError> {
    Err(CodegenError::MissingSisPorts {
        operation: "sim_verify_codegen",
    })
}

fn nodes_of_kind<N>(
    network: &SimNetwork<N>,
    kind: SimNodeKind,
) -> Result<Vec<&SimNode<N>>, CodegenError>
where
    N: Eq + Hash,
{
    let _ = nodes_by_id(network)?;
    Ok(network
        .nodes
        .iter()
        .filter(|node| node.kind == kind)
        .collect())
}

fn internal_count<N>(network: &SimNetwork<N>) -> Result<usize, CodegenError>
where
    N: Eq + Hash,
{
    let _ = nodes_by_id(network)?;
    Ok(network
        .nodes
        .iter()
        .filter(|node| node.kind == SimNodeKind::Internal)
        .count())
}

fn map_by_names<'a, N>(
    source: &[&'a SimNode<N>],
    target: &[&'a SimNode<N>],
) -> Result<Vec<usize>, &'a str> {
    source
        .iter()
        .map(|left| {
            target
                .iter()
                .position(|right| left.name == right.name)
                .ok_or(left.name.as_str())
        })
        .collect()
}

fn node_index<N>(network: &SimNetwork<N>) -> Result<HashMap<N, usize>, CodegenError>
where
    N: Clone + Eq + Hash,
{
    let mut index = HashMap::new();
    for node in network
        .nodes
        .iter()
        .filter(|node| node.kind == SimNodeKind::PrimaryInput)
        .chain(
            network
                .nodes
                .iter()
                .filter(|node| node.kind == SimNodeKind::PrimaryOutput),
        )
        .chain(
            network
                .nodes
                .iter()
                .filter(|node| node.kind == SimNodeKind::Internal),
        )
    {
        if index.insert(node.id.clone(), index.len()).is_some() {
            return Err(CodegenError::DuplicateNodeId);
        }
    }
    Ok(index)
}

fn nodes_by_id<N>(network: &SimNetwork<N>) -> Result<HashMap<&N, &SimNode<N>>, CodegenError>
where
    N: Eq + Hash,
{
    let mut nodes = HashMap::new();
    for node in &network.nodes {
        if nodes.insert(&node.id, node).is_some() {
            return Err(CodegenError::DuplicateNodeId);
        }
    }
    Ok(nodes)
}

fn node_expression<N>(node: &SimNode<N>, index: &HashMap<N, usize>) -> Result<String, CodegenError>
where
    N: Eq + Hash,
{
    match node.function {
        SimNodeFunction::Zero => Ok("0".to_string()),
        SimNodeFunction::One => Ok("(unsigned) -1".to_string()),
        SimNodeFunction::SumOfProducts => sop_expression(node, index),
    }
}

fn sop_expression<N>(node: &SimNode<N>, index: &HashMap<N, usize>) -> Result<String, CodegenError>
where
    N: Eq + Hash,
{
    if node.cubes.is_empty() {
        return Ok(String::new());
    }

    let mut cube_terms = Vec::new();
    for cube in node.cubes.iter().rev() {
        let mut literal_terms = Vec::new();
        for (fanin_index, literal) in cube.literals.iter().copied().enumerate() {
            let Some(fanin) = node.fanins.get(fanin_index) else {
                return Err(CodegenError::MissingFanin {
                    node: node.name.clone(),
                    fanin_index,
                });
            };
            let Some(table_index) = index.get(fanin) else {
                return Err(CodegenError::MissingFanin {
                    node: node.name.clone(),
                    fanin_index,
                });
            };
            match literal {
                SimLiteral::One => literal_terms.push(format!("a[{table_index}]")),
                SimLiteral::Zero => literal_terms.push(format!("~a[{table_index}]")),
                SimLiteral::DontCare => {}
            }
        }
        cube_terms.push(literal_terms.join("&"));
    }

    Ok(cube_terms.join("|"))
}

fn escape_c_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(literals: &[SimLiteral]) -> SimCube {
        SimCube::new(literals.to_vec())
    }

    fn sample_network(order_ba: bool) -> SimNetwork<&'static str> {
        let mut inputs = vec![
            SimNode::primary_input("a", "a"),
            SimNode::primary_input("b", "b"),
        ];
        if order_ba {
            inputs.reverse();
        }

        let mut nodes = inputs;
        nodes.push(SimNode::internal(
            "n1",
            "n1",
            vec!["a", "b"],
            vec![
                cube(&[SimLiteral::One, SimLiteral::Zero]),
                cube(&[SimLiteral::DontCare, SimLiteral::One]),
            ],
        ));
        nodes.push(SimNode::primary_output("po", "out", "n1"));
        SimNetwork::new(nodes, vec!["n1", "po"])
    }

    #[test]
    fn build_plan_maps_inputs_outputs_and_emits_c_like_source() {
        let network1 = sample_network(false);
        let network2 = sample_network(true);

        let plan = build_codegen_plan(&network1, &network2).unwrap();

        assert_eq!(plan.nin, 2);
        assert_eq!(plan.nout, 1);
        assert_eq!(plan.nodes1, 14);
        assert_eq!(plan.nodes2, 14);
        assert_eq!(plan.input_map, vec![1, 0]);
        assert_eq!(plan.output_map, vec![0]);
        assert_eq!(plan.output_names, vec!["out"]);

        let source = plan.generated_source();
        assert!(source.contains("#define nin 2\n"));
        assert!(source.contains("char *output_names[1] = {\n    \"out\",\n};\n"));
        assert!(source.contains("int input_map[2] = {\n    1,\n    0,\n};\n"));
    }

    #[test]
    fn generate_function_matches_reverse_cube_order_and_table_indices() {
        let network = sample_network(false);

        assert_eq!(
            generate_function(&network, "func1").unwrap(),
            concat!(
                "void func1(a)\n",
                "register unsigned *a;\n",
                "{\n",
                "    a[3] = a[1]|a[0]&~a[1];\n",
                "    a[2] = a[3];\n",
                "}\n"
            )
        );
    }

    #[test]
    fn constant_nodes_emit_legacy_unsigned_masks() {
        let network = SimNetwork::new(
            vec![
                SimNode::primary_input("a", "a"),
                SimNode::primary_output("po0", "zero", "z"),
                SimNode::primary_output("po1", "one", "o"),
                SimNode::constant("z", "z", false),
                SimNode::constant("o", "o", true),
            ],
            vec!["z", "o", "po0", "po1"],
        );

        let code = generate_function(&network, "func").unwrap();

        assert!(code.contains("    a[3] = 0;\n"));
        assert!(code.contains("    a[4] = (unsigned) -1;\n"));
        assert!(code.contains("    a[1] = a[3];\n"));
        assert!(code.contains("    a[2] = a[4];\n"));
    }

    #[test]
    fn mismatched_names_and_counts_report_c_equivalent_errors() {
        let network1 = sample_network(false);
        let mut network2 = sample_network(false);
        network2.nodes[0].name = "renamed".to_string();

        assert_eq!(
            build_codegen_plan(&network1, &network2),
            Err(CodegenError::MissingInputMatch {
                name: "a".to_string(),
            })
        );

        network2.nodes.pop();
        assert_eq!(
            build_codegen_plan(&network1, &network2),
            Err(CodegenError::OutputCountMismatch { left: 1, right: 0 })
        );
    }

    #[test]
    fn invalid_fanin_and_dfs_references_are_explicit() {
        let missing_fanin = SimNetwork::new(
            vec![SimNode::internal(
                "n",
                "n",
                vec!["missing"],
                vec![cube(&[SimLiteral::One])],
            )],
            vec!["n"],
        );
        assert_eq!(
            generate_function(&missing_fanin, "func"),
            Err(CodegenError::MissingFanin {
                node: "n".to_string(),
                fanin_index: 0,
            })
        );

        let missing_dfs_node = SimNetwork::new(vec![SimNode::primary_input("a", "a")], vec!["n"]);
        assert_eq!(
            generate_function(&missing_dfs_node, "func"),
            Err(CodegenError::MissingNodeInDfsOrder)
        );
    }

    #[test]
    fn sis_entry_reports_missing_native_prerequisites_without_c_abi() {
        assert_eq!(
            verify_codegen_in_sis_networks(),
            Err(CodegenError::MissingSisPorts {
                operation: "sim_verify_codegen",
            })
        );
    }
}
