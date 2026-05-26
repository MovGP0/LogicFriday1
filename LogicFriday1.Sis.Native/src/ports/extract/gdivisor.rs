//! Native Rust port of `sis/extract/gdivisor.c`.
//!
//! The C routine is an orchestration layer: convert a node to the sparse
//! function form, build the kernel-cube matrix, reject kernel-free cases, pick a
//! subkernel rectangle, and convert that rectangle back to a divisor node. The
//! lower-level extraction, rectangle selection, and node conversion ports are
//! represented here as a Rust trait so this module can stay free of legacy C ABI
//! shims while those units are ported independently.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelLevel {
    LevelZero,
    All,
}

impl KernelLevel {
    pub fn use_all_kernels(self) -> bool {
        matches!(self, Self::All)
    }
}

impl From<bool> for KernelLevel {
    fn from(use_all_kernels: bool) -> Self {
        if use_all_kernels {
            Self::All
        } else {
            Self::LevelZero
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubkernelSelectionMethod {
    PingPong,
    BestSopValue,
    BestFactoredValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidSubkernelSelectionMethod {
    method: i32,
}

impl InvalidSubkernelSelectionMethod {
    pub fn method(&self) -> i32 {
        self.method
    }
}

impl fmt::Display for InvalidSubkernelSelectionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid subkernel selection method {}", self.method)
    }
}

impl Error for InvalidSubkernelSelectionMethod {}

impl TryFrom<i32> for SubkernelSelectionMethod {
    type Error = InvalidSubkernelSelectionMethod;

    fn try_from(method: i32) -> Result<Self, Self::Error> {
        match method {
            0 => Ok(Self::PingPong),
            1 => Ok(Self::BestSopValue),
            2 => Ok(Self::BestFactoredValue),
            _ => Err(InvalidSubkernelSelectionMethod { method }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelCubeRow<CoKernel> {
    co_kernel: CoKernel,
}

impl<CoKernel> KernelCubeRow<CoKernel> {
    pub fn new(co_kernel: CoKernel) -> Self {
        Self { co_kernel }
    }

    pub fn co_kernel(&self) -> &CoKernel {
        &self.co_kernel
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelCubeTable<CoKernel> {
    rows: Vec<KernelCubeRow<CoKernel>>,
}

impl<CoKernel> KernelCubeTable<CoKernel> {
    pub fn new(rows: impl IntoIterator<Item = KernelCubeRow<CoKernel>>) -> Self {
        Self {
            rows: rows.into_iter().collect(),
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn first_row(&self) -> Option<&KernelCubeRow<CoKernel>> {
        self.rows.first()
    }

    pub fn rows(&self) -> &[KernelCubeRow<CoKernel>] {
        &self.rows
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubkernelRectangle {
    row_count: usize,
    column_count: usize,
}

impl SubkernelRectangle {
    pub fn new(row_count: usize, column_count: usize) -> Self {
        Self {
            row_count,
            column_count,
        }
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn column_count(&self) -> usize {
        self.column_count
    }

    pub fn is_empty(&self) -> bool {
        self.row_count == 0 || self.column_count == 0
    }
}

pub trait GeneralDivisorPort {
    type Node;
    type Function;
    type CoKernel;
    type Error;

    fn setup_globals_single(&mut self, node: &Self::Node) -> Result<(), Self::Error>;

    fn function_from_node(&mut self, node: &Self::Node) -> Result<Self::Function, Self::Error>;

    fn kernel_extract_init(&mut self) -> Result<(), Self::Error>;

    fn kernel_extract(
        &mut self,
        function: &Self::Function,
        sis_index: usize,
        level: KernelLevel,
    ) -> Result<(), Self::Error>;

    fn free_value_cells(&mut self, function: &mut Self::Function) -> Result<(), Self::Error>;

    fn kernel_extract_end(&mut self) -> Result<KernelCubeTable<Self::CoKernel>, Self::Error>;

    fn co_kernel_is_empty(&self, co_kernel: &Self::CoKernel) -> bool;

    fn choose_subkernel(
        &mut self,
        table: &KernelCubeTable<Self::CoKernel>,
        method: SubkernelSelectionMethod,
    ) -> Result<SubkernelRectangle, Self::Error>;

    fn rectangle_to_kernel(
        &mut self,
        table: &KernelCubeTable<Self::CoKernel>,
        rectangle: &SubkernelRectangle,
    ) -> Result<Self::Function, Self::Error>;

    fn node_from_function(&mut self, function: Self::Function) -> Result<Self::Node, Self::Error>;

    fn kernel_extract_free(&mut self) -> Result<(), Self::Error>;

    fn free_globals(&mut self) -> Result<(), Self::Error>;
}

pub fn find_divisor<P>(
    port: &mut P,
    node: &P::Node,
    level: KernelLevel,
    method: SubkernelSelectionMethod,
) -> Result<Option<P::Node>, P::Error>
where
    P: GeneralDivisorPort,
{
    port.setup_globals_single(node)?;
    if let Err(error) = port.kernel_extract_init() {
        let _ = port.free_globals();
        return Err(error);
    }

    let result = find_divisor_after_init(port, node, level, method);
    let free_result = port.kernel_extract_free();
    let globals_result = port.free_globals();

    match (result, free_result, globals_result) {
        (Err(error), _, _) => Err(error),
        (_, Err(error), _) => Err(error),
        (_, _, Err(error)) => Err(error),
        (Ok(divisor), Ok(()), Ok(())) => Ok(divisor),
    }
}

fn find_divisor_after_init<P>(
    port: &mut P,
    node: &P::Node,
    level: KernelLevel,
    method: SubkernelSelectionMethod,
) -> Result<Option<P::Node>, P::Error>
where
    P: GeneralDivisorPort,
{
    let mut function = port.function_from_node(node)?;
    port.kernel_extract(&function, 0, level)?;
    port.free_value_cells(&mut function)?;

    let table = port.kernel_extract_end()?;
    if table.row_count() == 0 {
        return Ok(None);
    }

    if table.row_count() == 1 {
        let first_row = table
            .first_row()
            .expect("a kernel-cube table with one row has a first row");
        if port.co_kernel_is_empty(first_row.co_kernel()) {
            return Ok(None);
        }
    }

    let rectangle = port.choose_subkernel(&table, method)?;
    if rectangle.is_empty() {
        return Ok(None);
    }

    let kernel = port.rectangle_to_kernel(&table, &rectangle)?;
    port.node_from_function(kernel).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNode(&'static str);

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestFunction(&'static str);

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestCoKernel {
        literal_count: usize,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum TestError {
        Failed(&'static str),
    }

    #[derive(Debug)]
    struct TestPort {
        table: KernelCubeTable<TestCoKernel>,
        rectangle: SubkernelRectangle,
        fail_at: Option<&'static str>,
        calls: Vec<&'static str>,
        selected_method: Option<SubkernelSelectionMethod>,
        selected_level: Option<KernelLevel>,
    }

    impl TestPort {
        fn new(table: KernelCubeTable<TestCoKernel>, rectangle: SubkernelRectangle) -> Self {
            Self {
                table,
                rectangle,
                fail_at: None,
                calls: Vec::new(),
                selected_method: None,
                selected_level: None,
            }
        }

        fn with_failure(mut self, call: &'static str) -> Self {
            self.fail_at = Some(call);
            self
        }

        fn record(&mut self, call: &'static str) -> Result<(), TestError> {
            self.calls.push(call);
            if self.fail_at == Some(call) {
                Err(TestError::Failed(call))
            } else {
                Ok(())
            }
        }
    }

    impl GeneralDivisorPort for TestPort {
        type Node = TestNode;
        type Function = TestFunction;
        type CoKernel = TestCoKernel;
        type Error = TestError;

        fn setup_globals_single(&mut self, _node: &Self::Node) -> Result<(), Self::Error> {
            self.record("setup_globals_single")
        }

        fn function_from_node(
            &mut self,
            _node: &Self::Node,
        ) -> Result<Self::Function, Self::Error> {
            self.record("function_from_node")?;
            Ok(TestFunction("node_function"))
        }

        fn kernel_extract_init(&mut self) -> Result<(), Self::Error> {
            self.record("kernel_extract_init")
        }

        fn kernel_extract(
            &mut self,
            _function: &Self::Function,
            sis_index: usize,
            level: KernelLevel,
        ) -> Result<(), Self::Error> {
            assert_eq!(sis_index, 0);
            self.selected_level = Some(level);
            self.record("kernel_extract")
        }

        fn free_value_cells(&mut self, _function: &mut Self::Function) -> Result<(), Self::Error> {
            self.record("free_value_cells")
        }

        fn kernel_extract_end(&mut self) -> Result<KernelCubeTable<Self::CoKernel>, Self::Error> {
            self.record("kernel_extract_end")?;
            Ok(self.table.clone())
        }

        fn co_kernel_is_empty(&self, co_kernel: &Self::CoKernel) -> bool {
            co_kernel.literal_count == 0
        }

        fn choose_subkernel(
            &mut self,
            _table: &KernelCubeTable<Self::CoKernel>,
            method: SubkernelSelectionMethod,
        ) -> Result<SubkernelRectangle, Self::Error> {
            self.selected_method = Some(method);
            self.record("choose_subkernel")?;
            Ok(self.rectangle.clone())
        }

        fn rectangle_to_kernel(
            &mut self,
            _table: &KernelCubeTable<Self::CoKernel>,
            _rectangle: &SubkernelRectangle,
        ) -> Result<Self::Function, Self::Error> {
            self.record("rectangle_to_kernel")?;
            Ok(TestFunction("divisor_function"))
        }

        fn node_from_function(
            &mut self,
            function: Self::Function,
        ) -> Result<Self::Node, Self::Error> {
            self.record("node_from_function")?;
            assert_eq!(function, TestFunction("divisor_function"));
            Ok(TestNode("divisor"))
        }

        fn kernel_extract_free(&mut self) -> Result<(), Self::Error> {
            self.record("kernel_extract_free")
        }

        fn free_globals(&mut self) -> Result<(), Self::Error> {
            self.record("free_globals")
        }
    }

    #[test]
    fn converts_selected_subkernel_to_divisor_node() {
        let table = KernelCubeTable::new([
            KernelCubeRow::new(TestCoKernel { literal_count: 1 }),
            KernelCubeRow::new(TestCoKernel { literal_count: 2 }),
        ]);
        let mut port = TestPort::new(table, SubkernelRectangle::new(2, 3));

        let divisor = find_divisor(
            &mut port,
            &TestNode("source"),
            KernelLevel::All,
            SubkernelSelectionMethod::BestSopValue,
        );

        assert_eq!(divisor, Ok(Some(TestNode("divisor"))));
        assert_eq!(port.selected_level, Some(KernelLevel::All));
        assert_eq!(
            port.selected_method,
            Some(SubkernelSelectionMethod::BestSopValue)
        );
        assert_eq!(
            port.calls,
            vec![
                "setup_globals_single",
                "kernel_extract_init",
                "function_from_node",
                "kernel_extract",
                "free_value_cells",
                "kernel_extract_end",
                "choose_subkernel",
                "rectangle_to_kernel",
                "node_from_function",
                "kernel_extract_free",
                "free_globals"
            ]
        );
    }

    #[test]
    fn returns_none_when_kernel_cube_table_is_empty() {
        let mut port = TestPort::new(KernelCubeTable::new([]), SubkernelRectangle::new(2, 3));

        let divisor = find_divisor(
            &mut port,
            &TestNode("source"),
            KernelLevel::LevelZero,
            SubkernelSelectionMethod::PingPong,
        );

        assert_eq!(divisor, Ok(None));
        assert!(!port.calls.contains(&"choose_subkernel"));
        assert_eq!(port.selected_level, Some(KernelLevel::LevelZero));
    }

    #[test]
    fn returns_none_for_single_kernel_with_empty_co_kernel() {
        let table = KernelCubeTable::new([KernelCubeRow::new(TestCoKernel { literal_count: 0 })]);
        let mut port = TestPort::new(table, SubkernelRectangle::new(1, 2));

        let divisor = find_divisor(
            &mut port,
            &TestNode("source"),
            KernelLevel::All,
            SubkernelSelectionMethod::BestFactoredValue,
        );

        assert_eq!(divisor, Ok(None));
        assert!(!port.calls.contains(&"choose_subkernel"));
    }

    #[test]
    fn returns_none_when_chosen_rectangle_has_no_rows_or_columns() {
        let table = KernelCubeTable::new([KernelCubeRow::new(TestCoKernel { literal_count: 2 })]);
        let mut port = TestPort::new(table, SubkernelRectangle::new(0, 2));

        let divisor = find_divisor(
            &mut port,
            &TestNode("source"),
            KernelLevel::All,
            SubkernelSelectionMethod::BestFactoredValue,
        );

        assert_eq!(divisor, Ok(None));
        assert!(port.calls.contains(&"choose_subkernel"));
        assert!(!port.calls.contains(&"rectangle_to_kernel"));
    }

    #[test]
    fn cleanup_runs_when_late_step_fails() {
        let table = KernelCubeTable::new([KernelCubeRow::new(TestCoKernel { literal_count: 2 })]);
        let mut port =
            TestPort::new(table, SubkernelRectangle::new(1, 1)).with_failure("rectangle_to_kernel");

        let divisor = find_divisor(
            &mut port,
            &TestNode("source"),
            KernelLevel::All,
            SubkernelSelectionMethod::BestSopValue,
        );

        assert_eq!(divisor, Err(TestError::Failed("rectangle_to_kernel")));
        assert!(port.calls.contains(&"kernel_extract_free"));
        assert!(port.calls.contains(&"free_globals"));
    }

    #[test]
    fn globals_are_freed_when_kernel_initialization_fails() {
        let table = KernelCubeTable::new([KernelCubeRow::new(TestCoKernel { literal_count: 2 })]);
        let mut port =
            TestPort::new(table, SubkernelRectangle::new(1, 1)).with_failure("kernel_extract_init");

        let divisor = find_divisor(
            &mut port,
            &TestNode("source"),
            KernelLevel::All,
            SubkernelSelectionMethod::BestSopValue,
        );

        assert_eq!(divisor, Err(TestError::Failed("kernel_extract_init")));
        assert_eq!(
            port.calls,
            vec![
                "setup_globals_single",
                "kernel_extract_init",
                "free_globals"
            ]
        );
    }

    #[test]
    fn validates_legacy_method_numbers() {
        assert_eq!(
            SubkernelSelectionMethod::try_from(0),
            Ok(SubkernelSelectionMethod::PingPong)
        );
        assert_eq!(
            SubkernelSelectionMethod::try_from(1),
            Ok(SubkernelSelectionMethod::BestSopValue)
        );
        assert_eq!(
            SubkernelSelectionMethod::try_from(2),
            Ok(SubkernelSelectionMethod::BestFactoredValue)
        );
        assert_eq!(
            SubkernelSelectionMethod::try_from(9),
            Err(InvalidSubkernelSelectionMethod { method: 9 })
        );
    }
}
