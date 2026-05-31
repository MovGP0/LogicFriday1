use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreeNodeType {
    And,
    Or,
    Nand,
    Nor,
    Zero,
    One,
    Leaf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeNode {
    pub name: Option<String>,
    pub node_type: TreeNodeType,
    pub phase: bool,
    pub level: i32,
    pub s: i32,
    pub p: i32,
    pub sons: Vec<TreeNode>,
}

impl TreeNode {
    pub fn leaf(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            node_type: TreeNodeType::Leaf,
            phase: true,
            level: 0,
            s: 1,
            p: 1,
            sons: Vec::new(),
        }
    }

    pub fn node(node_type: TreeNodeType, sons: Vec<TreeNode>) -> Self {
        let level = sons.iter().map(|son| son.level).max().unwrap_or(-1) + 1;

        Self {
            name: None,
            node_type,
            phase: true,
            level,
            s: 0,
            p: 0,
            sons,
        }
    }

    pub fn constant(name: impl Into<String>, value: bool) -> Self {
        Self {
            name: Some(name.into()),
            node_type: if value {
                TreeNodeType::One
            } else {
                TreeNodeType::Zero
            },
            phase: true,
            level: 0,
            s: 0,
            p: 0,
            sons: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_stats(mut self, level: i32, s: i32, p: i32) -> Self {
        self.level = level;
        self.s = s;
        self.p = p;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatchInfo {
    pub input: String,
    pub output: String,
    pub latch_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenlibIoError {
    Expression(String),
    MissingNodeName,
}

impl fmt::Display for GenlibIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenlibIoError::Expression(message) => {
                write!(formatter, "{message}")
            }
            GenlibIoError::MissingNodeName => {
                write!(formatter, "tree node is missing a BLIF name")
            }
        }
    }
}

impl std::error::Error for GenlibIoError {}

pub trait GateEnumerator {
    fn number_of_gates(&self, level: i32, s: i32, p: i32) -> i32;

    fn generate_complex_gates(&self, level: i32, s: i32, p: i32) -> Vec<TreeNode>;

    fn nand_gate_forms(&self, tree: &TreeNode, use_nor_gate: bool) -> Vec<TreeNode>;

    fn parse_expression(&self, expression: &str) -> Result<(TreeNode, String), GenlibIoError>;
}

pub fn table_by_level(
    s_max: i32,
    p_max: i32,
    number_of_gates: impl Fn(i32, i32, i32) -> i32,
) -> String {
    let mut output = String::new();
    output.push_str("                 ");

    for level in 0..=10 {
        push_fixed_int(&mut output, level, 8);
    }

    output.push('\n');

    for s in 1..=s_max {
        for p in 1..=p_max {
            let total = number_of_gates(s + p, s, p);
            output.push_str(&format!("{s} {p} ({total:9}):  "));

            for level in 0..=10 {
                let mut count = number_of_gates(level, s, p);

                if level > 0 {
                    count -= number_of_gates(level - 1, s, p);
                }

                if count > 0 {
                    push_fixed_int(&mut output, count, 8);
                } else {
                    output.push_str("        ");
                }
            }

            output.push('\n');
        }
    }

    output
}

pub fn table_of_gate_count(
    s_max: i32,
    p_max: i32,
    number_of_gates: impl Fn(i32, i32, i32) -> i32,
) -> String {
    let mut output = String::new();
    output.push_str("    ");

    for s in 1..=s_max {
        push_fixed_int(&mut output, s, 10);
    }

    output.push('\n');

    for s in 1..=s_max {
        output.push_str(&format!("{s} : "));

        for p in 1..=p_max {
            let level = s + p - 2;
            let count = number_of_gates(level, s, p) + number_of_gates(level, p, s) - 1;
            push_fixed_int(&mut output, count, 10);
        }

        output.push('\n');
    }

    output
}

pub fn table_of_nand_forms(
    s_max: i32,
    p_max: i32,
    number_of_nand_forms: impl Fn(i32, i32, i32) -> i32,
) -> String {
    let mut output = String::new();
    output.push_str("    ");

    for s in 1..=s_max {
        push_fixed_int(&mut output, s, 10);
    }

    output.push('\n');

    for s in 1..=s_max {
        output.push_str(&format!("{s} : "));

        for p in 1..=p_max {
            let level = s + p - 2;
            push_fixed_int(&mut output, number_of_nand_forms(level, s, p), 10);
        }

        output.push('\n');
    }

    output
}

pub fn num_leafs(tree: &TreeNode) -> usize {
    if tree.sons.is_empty() {
        return 1;
    }

    tree.sons.iter().map(num_leafs).sum()
}

pub fn print_all_gates_genlib(
    level: i32,
    s: i32,
    p: i32,
    invert_output: bool,
    enumerator: &impl GateEnumerator,
) -> String {
    let level = default_level(level, s, p);
    let mut output = String::new();

    for mut tree in enumerator.generate_complex_gates(level, s, p) {
        tree.phase = !invert_output;
        output.push_str("GATE \"");
        output.push_str(&format_tree(&tree));
        output.push_str(&format!("\" {} O=", num_leafs(&tree) + 1));
        output.push_str(&format_tree_algebraic(&tree));
        output.push_str(";\n");
        output.push_str("PIN * INV 1 999 1.0 0.2 1.0 0.2\n");
    }

    output
}

pub fn print_all_gates(level: i32, s: i32, p: i32, enumerator: &impl GateEnumerator) -> String {
    let level = default_level(level, s, p);
    let trees = enumerator.generate_complex_gates(level, s, p);
    let mut output = String::new();

    for tree in &trees {
        output.push_str(&format!(
            "level={} s={} p={}  {}\n",
            tree.level,
            tree.s,
            tree.p,
            format_tree(tree)
        ));
    }

    output.push_str(&format!("Total forms = {}\n", trees.len()));
    output
}

pub fn print_all_nand_forms(
    level: i32,
    s: i32,
    p: i32,
    use_nor_gate: bool,
    invert_output: bool,
    enumerator: &impl GateEnumerator,
) -> Result<String, GenlibIoError> {
    let level = default_level(level, s, p);
    let mut output = String::new();

    for mut tree in enumerator.generate_complex_gates(level, s, p) {
        tree.phase = !invert_output;
        assign_leaf_names(&mut tree);

        for mut nand_tree in enumerator.nand_gate_forms(&tree, use_nor_gate) {
            assign_node_names(&mut nand_tree);
            output.push_str(".model ");

            if invert_output {
                output.push_str("!(");
            }

            output.push_str(&format_tree(&tree));

            if invert_output {
                output.push(')');
            }

            output.push('\n');
            output.push_str(&write_blif(&nand_tree, use_nor_gate, None)?);
            output.push_str(".end\n");
        }
    }

    Ok(output)
}

pub fn number_of_nand_forms(level: i32, s: i32, p: i32, enumerator: &impl GateEnumerator) -> i32 {
    let level = default_level(level, s, p);

    enumerator
        .generate_complex_gates(level, s, p)
        .iter()
        .map(|tree| enumerator.nand_gate_forms(tree, false).len() as i32)
        .sum()
}

pub fn print_nand_forms(
    expression: &str,
    use_nor_gate: bool,
    enumerator: &impl GateEnumerator,
) -> Result<String, GenlibIoError> {
    let (tree, output_name) = enumerator.parse_expression(expression)?;
    let mut output = String::new();

    for mut nand_tree in enumerator.nand_gate_forms(&tree, use_nor_gate) {
        assign_node_names(&mut nand_tree);
        nand_tree.name = Some(output_name.clone());
        output.push_str(".model ");
        output.push_str(&format_tree(&tree));
        output.push('\n');
        output.push_str(&write_blif(&nand_tree, use_nor_gate, None)?);
        output.push_str(".end\n");
    }

    Ok(output)
}

pub fn write_blif(
    tree: &TreeNode,
    use_nor_gate: bool,
    latch: Option<&LatchInfo>,
) -> Result<String, GenlibIoError> {
    let name = node_name(tree)?;
    let mut output = String::new();

    match tree.node_type {
        TreeNodeType::Zero => {
            output.push_str(&format!(".outputs {name}\n"));
            output.push_str(&format!(".names {name}\n"));
        }
        TreeNodeType::One => {
            output.push_str(&format!(".outputs {name}\n"));
            output.push_str(&format!(".names {name}\n 1\n"));
        }
        _ => {
            output.push_str(".inputs");

            for leaf in sorted_unique_leaf_names(tree) {
                if latch.is_some_and(|latch| latch.output == leaf) {
                    continue;
                }

                output.push(' ');
                output.push_str(&leaf);
            }

            output.push('\n');

            if latch.is_none() {
                output.push_str(&format!(".outputs {name}\n"));
            }

            write_blif_tables(tree, use_nor_gate, &mut output)?;
        }
    }

    Ok(output)
}

pub fn format_tree(tree: &TreeNode) -> String {
    if tree.sons.is_empty() {
        return tree.name.clone().unwrap_or_else(|| "leaf".to_string());
    }

    let separator = match tree.node_type {
        TreeNodeType::Or | TreeNodeType::Nor => " + ",
        _ => " * ",
    };

    let expression = tree
        .sons
        .iter()
        .map(format_tree)
        .collect::<Vec<_>>()
        .join(separator);

    if tree.phase {
        format!("({expression})")
    } else {
        format!("!({expression})")
    }
}

pub fn format_tree_algebraic(tree: &TreeNode) -> String {
    match tree.node_type {
        TreeNodeType::Zero => "0".to_string(),
        TreeNodeType::One => "1".to_string(),
        _ => format_tree(tree),
    }
}

pub fn assign_leaf_names(tree: &mut TreeNode) {
    let mut next = 1;
    assign_leaf_names_recur(tree, &mut next);
}

pub fn assign_node_names(tree: &mut TreeNode) {
    let mut next = 1;
    assign_node_names_recur(tree, &mut next);
}

fn push_fixed_int(output: &mut String, value: i32, width: usize) {
    output.push_str(&format!("{value:>width$}"));
}

fn default_level(level: i32, s: i32, p: i32) -> i32 {
    if level < 0 { s + p } else { level }
}

fn node_name(tree: &TreeNode) -> Result<&str, GenlibIoError> {
    tree.name.as_deref().ok_or(GenlibIoError::MissingNodeName)
}

fn sorted_unique_leaf_names(tree: &TreeNode) -> Vec<String> {
    let mut names = BTreeSet::new();
    collect_leaf_names(tree, &mut names);
    names.into_iter().collect()
}

fn collect_leaf_names(tree: &TreeNode, names: &mut BTreeSet<String>) {
    if tree.sons.is_empty() {
        if let Some(name) = &tree.name {
            names.insert(name.clone());
        }

        return;
    }

    for son in &tree.sons {
        collect_leaf_names(son, names);
    }
}

fn write_blif_tables(
    tree: &TreeNode,
    use_nor_gate: bool,
    output: &mut String,
) -> Result<(), GenlibIoError> {
    for son in &tree.sons {
        write_blif_tables(son, use_nor_gate, output)?;
    }

    if tree.sons.is_empty() {
        return Ok(());
    }

    output.push_str(".names");

    for son in &tree.sons {
        output.push(' ');
        output.push_str(node_name(son)?);
    }

    output.push(' ');
    output.push_str(node_name(tree)?);
    output.push('\n');

    if use_nor_gate {
        for _ in &tree.sons {
            output.push('0');
        }

        output.push_str(" 1\n");
    } else {
        for index in 0..tree.sons.len() {
            for test in 0..tree.sons.len() {
                output.push(if index == test { '0' } else { '-' });
            }

            output.push_str(" 1\n");
        }
    }

    Ok(())
}

fn assign_leaf_names_recur(tree: &mut TreeNode, next: &mut usize) {
    if tree.sons.is_empty() {
        if tree.name.is_none() {
            tree.name = Some(format!("a{next}"));
            *next += 1;
        }

        return;
    }

    for son in &mut tree.sons {
        assign_leaf_names_recur(son, next);
    }
}

fn assign_node_names_recur(tree: &mut TreeNode, next: &mut usize) {
    for son in &mut tree.sons {
        assign_node_names_recur(son, next);
    }

    if !tree.sons.is_empty() && tree.name.is_none() {
        tree.name = Some(format!("n{next}"));
        *next += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeEnumerator;

    impl GateEnumerator for FakeEnumerator {
        fn number_of_gates(&self, level: i32, s: i32, p: i32) -> i32 {
            level + s + p
        }

        fn generate_complex_gates(&self, level: i32, s: i32, p: i32) -> Vec<TreeNode> {
            vec![
                TreeNode::node(
                    TreeNodeType::And,
                    vec![TreeNode::leaf("a"), TreeNode::leaf("b")],
                )
                .with_name("out")
                .with_stats(level, s, p),
                TreeNode::node(
                    TreeNodeType::Or,
                    vec![TreeNode::leaf("a"), TreeNode::leaf("c")],
                )
                .with_name("sum")
                .with_stats(level - 1, s, p),
            ]
        }

        fn nand_gate_forms(&self, tree: &TreeNode, use_nor_gate: bool) -> Vec<TreeNode> {
            let node_type = if use_nor_gate {
                TreeNodeType::Nor
            } else {
                TreeNodeType::Nand
            };

            vec![TreeNode::node(node_type, tree.sons.clone()).with_name("g0")]
        }

        fn parse_expression(&self, expression: &str) -> Result<(TreeNode, String), GenlibIoError> {
            if expression == "bad" {
                return Err(GenlibIoError::Expression("bad expression".to_string()));
            }

            Ok((
                TreeNode::node(
                    TreeNodeType::And,
                    vec![TreeNode::leaf("a"), TreeNode::leaf("b")],
                )
                .with_name("f"),
                "f".to_string(),
            ))
        }
    }

    #[test]
    fn formats_level_delta_table() {
        let table = table_by_level(1, 1, |level, s, p| level + s + p);

        assert!(table.starts_with("                        0       1"));
        assert!(table.contains("1 1 (        4):"));
        assert!(table.contains("       2       1       1"));
    }

    #[test]
    fn formats_gate_count_table() {
        let table = table_of_gate_count(2, 2, |level, s, p| level + s + p);

        assert!(table.contains("         1         2"));
        assert!(table.contains("1 :          3         7"));
        assert!(table.contains("2 :          7        11"));
    }

    #[test]
    fn formats_nand_form_count_table() {
        let table = table_of_nand_forms(1, 2, |level, s, p| level * 10 + s + p);

        assert!(table.contains("1 :          2        13"));
    }

    #[test]
    fn counts_leafs_recursively() {
        let tree = TreeNode::node(
            TreeNodeType::Or,
            vec![
                TreeNode::leaf("a"),
                TreeNode::node(
                    TreeNodeType::And,
                    vec![TreeNode::leaf("b"), TreeNode::leaf("c")],
                ),
            ],
        );

        assert_eq!(num_leafs(&tree), 3);
    }

    #[test]
    fn prints_genlib_gate_records() {
        let output = print_all_gates_genlib(-1, 1, 1, false, &FakeEnumerator);

        assert!(output.contains("GATE \"(a * b)\" 3 O=(a * b);"));
        assert!(output.contains("PIN * INV 1 999 1.0 0.2 1.0 0.2"));
    }

    #[test]
    fn prints_all_generated_gates() {
        let output = print_all_gates(3, 1, 2, &FakeEnumerator);

        assert!(output.contains("level=3 s=1 p=2  (a * b)"));
        assert!(output.contains("Total forms = 2"));
    }

    #[test]
    fn counts_nand_forms_from_generated_trees() {
        assert_eq!(number_of_nand_forms(-1, 1, 1, &FakeEnumerator), 2);
    }

    #[test]
    fn writes_nand_form_models() {
        let output = print_all_nand_forms(1, 1, 1, false, false, &FakeEnumerator).unwrap();

        assert!(output.contains(".model (a * b)"));
        assert!(output.contains(".inputs a b"));
        assert!(output.contains(".outputs g0"));
        assert!(output.contains(".end"));
    }

    #[test]
    fn writes_parsed_nand_forms_with_expression_model_name() {
        let output = print_nand_forms("a*b", true, &FakeEnumerator).unwrap();

        assert!(output.contains(".model (a * b)"));
        assert!(output.contains(".outputs f"));
        assert!(output.contains("00 1"));
    }

    #[test]
    fn writes_blif_constants() {
        let zero = TreeNode::constant("z", false);
        let one = TreeNode::constant("o", true);

        assert_eq!(
            write_blif(&zero, false, None).unwrap(),
            ".outputs z\n.names z\n"
        );
        assert_eq!(
            write_blif(&one, false, None).unwrap(),
            ".outputs o\n.names o\n 1\n"
        );
    }

    #[test]
    fn writes_blif_skipping_latch_output_as_input_and_output() {
        let tree = TreeNode::node(
            TreeNodeType::Nand,
            vec![TreeNode::leaf("a"), TreeNode::leaf("q")],
        )
        .with_name("next");
        let latch = LatchInfo {
            input: "next".to_string(),
            output: "q".to_string(),
            latch_type: "re".to_string(),
        };

        let output = write_blif(&tree, false, Some(&latch)).unwrap();

        assert!(output.starts_with(".inputs a\n.names a q next\n"));
        assert!(!output.contains(".outputs next"));
    }

    #[test]
    fn reports_missing_names_for_blif_internal_nodes() {
        let tree = TreeNode::node(
            TreeNodeType::Nand,
            vec![TreeNode::leaf("a"), TreeNode::leaf("b")],
        );

        assert_eq!(
            write_blif(&tree, false, None),
            Err(GenlibIoError::MissingNodeName)
        );
    }
}
