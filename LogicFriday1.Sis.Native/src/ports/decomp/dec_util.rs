use crate::ports::extract::gdivisor::{
    GeneralDivisorPort, KernelLevel, SubkernelSelectionMethod, find_divisor,
};
use crate::ports::node::node::{
    Cover, Cube, Node, NodeError, node_and, node_constant, node_literal,
};
use std::error::Error;
use std::fmt;

pub type DecUtilResult<T> = Result<T, DecUtilError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecUtilError
{
    Node(NodeError),
    MissingFunction { operation: &'static str },
    CubeIndexOutOfRange { index: usize, cube_count: usize },
}

impl fmt::Display for DecUtilError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::Node(error) => write!(formatter, "{error}"),
            Self::MissingFunction { operation } =>
            {
                write!(formatter, "{operation} requires a node with a Boolean function")
            }
            Self::CubeIndexOutOfRange { index, cube_count } =>
            {
                write!(
                    formatter,
                    "cube index {index} is outside the {cube_count} cubes in the node"
                )
            }
        }
    }
}

impl Error for DecUtilError {}

impl From<NodeError> for DecUtilError
{
    fn from(value: NodeError) -> Self
    {
        Self::Node(value)
    }
}

pub fn decomp_quick_kernel(node: &Node) -> DecUtilResult<Option<Node>>
{
    let mut quotient = node.clone();
    let mut loops = 0;

    loop
    {
        let Some((fanin_index, phase)) = most_frequent_literal(&quotient)? else
        {
            break;
        };

        quotient = divide_by_literal(&quotient, fanin_index, phase)?;
        loops += 1;
    }

    if loops == 0
    {
        Ok(None)
    }
    else
    {
        Ok(Some(quotient))
    }
}

pub fn decomp_good_kernel<P>(
    port: &mut P,
    node: &P::Node,
) -> Result<Option<P::Node>, P::Error>
where
    P: GeneralDivisorPort,
{
    find_divisor(
        port,
        node,
        KernelLevel::All,
        SubkernelSelectionMethod::BestSopValue,
    )
}

pub fn dec_node_cube(node: &Node, index: usize) -> DecUtilResult<Node>
{
    let function = node
        .function()
        .ok_or(DecUtilError::MissingFunction {
            operation: "dec_node_cube",
        })?;
    let cube = function
        .cubes()
        .get(index)
        .ok_or(DecUtilError::CubeIndexOutOfRange {
            index,
            cube_count: function.cube_count(),
        })?;

    let mut result = node_constant(1)?;
    for (fanin, literal) in node.fanins.iter().zip(cube.inputs())
    {
        let Some(phase) = literal else
        {
            continue;
        };

        let literal_node = node_literal(fanin.clone(), i32::from(*phase))?;
        result = node_and(&result, &literal_node)?;
    }

    Ok(result)
}

fn most_frequent_literal(node: &Node) -> DecUtilResult<Option<(usize, bool)>>
{
    let function = node
        .function()
        .ok_or(DecUtilError::MissingFunction {
            operation: "decomp_quick_kernel",
        })?;
    let counts = function.literal_counts();
    let mut best = 0;
    let mut literal = None;

    for index in (0..counts.len()).rev()
    {
        if counts[index] > best
        {
            best = counts[index];
            literal = Some((index / 2, index % 2 == 1));
        }
    }

    if best <= 1
    {
        Ok(None)
    }
    else
    {
        Ok(literal)
    }
}

fn divide_by_literal(node: &Node, fanin_index: usize, phase: bool) -> DecUtilResult<Node>
{
    let function = node
        .function()
        .ok_or(DecUtilError::MissingFunction {
            operation: "decomp_quick_kernel",
        })?;
    let mut cubes = Vec::new();

    for cube in function.cubes()
    {
        if cube.inputs()[fanin_index] != Some(phase)
        {
            continue;
        }

        let mut inputs = cube.inputs().to_vec();
        inputs[fanin_index] = None;
        cubes.push(Cube::new(inputs));
    }

    Ok(Node::new(
        Cover::new(function.input_count(), cubes)?,
        node.fanins.clone(),
    ))
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::ports::extract::gdivisor::{
        KernelCubeRow, KernelCubeTable, SubkernelRectangle,
    };
    use crate::ports::node::node::{node_equal, node_function, node_or, NodeFunction};

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNode(&'static str);

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestFunction(&'static str);

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestCoKernel
    {
        literal_count: usize,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestError;

    #[derive(Debug)]
    struct TestPort
    {
        selected_level: Option<KernelLevel>,
        selected_method: Option<SubkernelSelectionMethod>,
    }

    impl TestPort
    {
        fn new() -> Self
        {
            Self {
                selected_level: None,
                selected_method: None,
            }
        }
    }

    impl GeneralDivisorPort for TestPort
    {
        type Node = TestNode;
        type Function = TestFunction;
        type CoKernel = TestCoKernel;
        type Error = TestError;

        fn setup_globals_single(&mut self, _node: &Self::Node) -> Result<(), Self::Error>
        {
            Ok(())
        }

        fn function_from_node(&mut self, _node: &Self::Node) -> Result<Self::Function, Self::Error>
        {
            Ok(TestFunction("function"))
        }

        fn kernel_extract_init(&mut self) -> Result<(), Self::Error>
        {
            Ok(())
        }

        fn kernel_extract(
            &mut self,
            _function: &Self::Function,
            sis_index: usize,
            level: KernelLevel,
        ) -> Result<(), Self::Error>
        {
            assert_eq!(sis_index, 0);
            self.selected_level = Some(level);
            Ok(())
        }

        fn free_value_cells(&mut self, _function: &mut Self::Function) -> Result<(), Self::Error>
        {
            Ok(())
        }

        fn kernel_extract_end(&mut self) -> Result<KernelCubeTable<Self::CoKernel>, Self::Error>
        {
            Ok(KernelCubeTable::new([
                KernelCubeRow::new(TestCoKernel { literal_count: 1 }),
                KernelCubeRow::new(TestCoKernel { literal_count: 2 }),
            ]))
        }

        fn co_kernel_is_empty(&self, co_kernel: &Self::CoKernel) -> bool
        {
            co_kernel.literal_count == 0
        }

        fn choose_subkernel(
            &mut self,
            _table: &KernelCubeTable<Self::CoKernel>,
            method: SubkernelSelectionMethod,
        ) -> Result<SubkernelRectangle, Self::Error>
        {
            self.selected_method = Some(method);
            Ok(SubkernelRectangle::new(1, 1))
        }

        fn rectangle_to_kernel(
            &mut self,
            _table: &KernelCubeTable<Self::CoKernel>,
            _rectangle: &SubkernelRectangle,
        ) -> Result<Self::Function, Self::Error>
        {
            Ok(TestFunction("divisor"))
        }

        fn node_from_function(&mut self, function: Self::Function) -> Result<Self::Node, Self::Error>
        {
            assert_eq!(function, TestFunction("divisor"));
            Ok(TestNode("divisor"))
        }

        fn kernel_extract_free(&mut self) -> Result<(), Self::Error>
        {
            Ok(())
        }

        fn free_globals(&mut self) -> Result<(), Self::Error>
        {
            Ok(())
        }
    }

    fn lit(name: &str, phase: i32) -> Node
    {
        node_literal(name, phase).unwrap()
    }

    fn assert_equal(left: &Node, right: &Node)
    {
        assert!(node_equal(left, right).unwrap());
    }

    #[test]
    fn dec_node_cube_rebuilds_selected_cube_as_conjunction()
    {
        let ab = node_and(&lit("a", 1), &lit("b", 1)).unwrap();
        let acn = node_and(&lit("a", 1), &lit("c", 0)).unwrap();
        let node = node_or(&ab, &acn).unwrap();

        let cube = dec_node_cube(&node, 1).unwrap();

        assert_equal(&cube, &acn);
    }

    #[test]
    fn dec_node_cube_returns_one_for_tautology_cube()
    {
        let node = node_constant(1).unwrap();

        let cube = dec_node_cube(&node, 0).unwrap();

        assert_eq!(node_function(&cube).unwrap(), NodeFunction::One);
    }

    #[test]
    fn dec_node_cube_reports_out_of_range_index()
    {
        let node = lit("a", 1);

        let error = dec_node_cube(&node, 2).unwrap_err();

        assert_eq!(
            error,
            DecUtilError::CubeIndexOutOfRange {
                index: 2,
                cube_count: 1,
            }
        );
    }

    #[test]
    fn quick_kernel_divides_repeated_literals_until_kernel_free()
    {
        let ab = node_and(&lit("a", 1), &lit("b", 1)).unwrap();
        let ac = node_and(&lit("a", 1), &lit("c", 1)).unwrap();
        let node = node_or(&ab, &ac).unwrap();

        let kernel = decomp_quick_kernel(&node).unwrap().unwrap();

        assert_equal(&kernel, &node_or(&lit("b", 1), &lit("c", 1)).unwrap());
    }

    #[test]
    fn quick_kernel_returns_none_when_no_literal_repeats()
    {
        let node = node_or(&lit("a", 1), &lit("b", 1)).unwrap();

        let kernel = decomp_quick_kernel(&node).unwrap();

        assert_eq!(kernel, None);
    }

    #[test]
    fn good_kernel_uses_all_level_best_sop_general_divisor()
    {
        let mut port = TestPort::new();

        let divisor = decomp_good_kernel(&mut port, &TestNode("source"));

        assert_eq!(divisor, Ok(Some(TestNode("divisor"))));
        assert_eq!(port.selected_level, Some(KernelLevel::All));
        assert_eq!(
            port.selected_method,
            Some(SubkernelSelectionMethod::BestSopValue)
        );
    }

    #[test]
    fn no_legacy_abi_or_tracking_tokens_are_present()
    {
        let source = include_str!("dec_util.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-")));
    }
}
