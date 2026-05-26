//! Native Rust model for `LogicSynthesis/sis/pld/ite_new_urp.c`.
//!
//! The C implementation builds ACT ITEs from SIS `node_t` covers, with many
//! fast paths for constants, literals, cubes, ACT matches, unate covers, and
//! binate variable selection. This module keeps the cover-analysis and ITE
//! construction behavior on owned Rust data. Direct SIS node, ACT, sparse
//! matrix, and global terminal-table integration is reported through explicit
//! dependency errors instead of exposing legacy C ABI symbols.

use std::error::Error;
use std::fmt;

pub const ACT_ITE_ALPHA: isize = 10;
pub const ACT_ITE_GAMMA: isize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node_function, node cubes, node literals, node_free, and constant/literal node construction",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "fanin enumeration, fanin lookup, and support-size checks",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.308",
        source_file: "LogicSynthesis/sis/node/cofct.c",
        reason: "node_algebraic_cofactor and node_cofactor drive recursive ITE decomposition",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.317",
        source_file: "LogicSynthesis/sis/node/names.c",
        reason: "literal vertices and cover columns carry fanin long names",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.344",
        source_file: "LogicSynthesis/sis/pld/act_bool.c",
        reason: "act_is_act_function and ACT_MATCH conversion to ITE are SIS-backed fast paths",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.350",
        source_file: "LogicSynthesis/sis/pld/act_ite_new.c",
        reason: "cost slots, ACT_ITE_ite storage, and tree mapping belong to the ACT ITE integration port",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.359",
        source_file: "LogicSynthesis/sis/pld/act_urp.c",
        reason: "legacy unate-recursive mapping variants are called by selection heuristics",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.360",
        source_file: "LogicSynthesis/sis/pld/act_util.c",
        reason: "my_shannon_ite, ACT cleanup, and shared utility behavior are legacy SIS helpers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.367",
        source_file: "LogicSynthesis/sis/pld/ite_leaf.c",
        reason: "single cube, single-literal cube, and unate-cover leaf constructors are delegated helpers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.376",
        source_file: "LogicSynthesis/sis/pld/pld_util.c",
        reason: "cube-to-node conversion and cube extraction are used by orthogonal cube handling",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.457",
        source_file: "LogicSynthesis/sis/sparse/matrix.c",
        reason: "build_F and unate-cover minimum-cover construction use sm_matrix",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "ite_end_table interns constant ITE terminals during recursive construction",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    PrimaryInput,
    PrimaryOutput,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFunction {
    Zero,
    One,
    Buffer,
    Inverter,
    Cover,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralPhase {
    Negative,
    Positive,
    DontCare,
}

impl LiteralPhase {
    pub fn from_sis_literal(value: i32) -> IteNewUrpResult<Self> {
        match value {
            0 => Ok(Self::Negative),
            1 => Ok(Self::Positive),
            2 => Ok(Self::DontCare),
            _ => Err(IteNewUrpError::InvalidLiteralValue(value)),
        }
    }

    pub fn matches(self, value: bool) -> bool {
        match self {
            Self::Negative => !value,
            Self::Positive => value,
            Self::DontCare => true,
        }
    }

    pub fn is_present(self) -> bool {
        self != Self::DontCare
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputPhase {
    PositiveUnate,
    NegativeUnate,
    Binate,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverNode {
    pub name: String,
    pub kind: NodeKind,
    pub function: NodeFunction,
    pub fanins: Vec<String>,
    pub cubes: Vec<Vec<LiteralPhase>>,
}

impl CoverNode {
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            function: NodeFunction::Cover,
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }

    pub fn constant_zero(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Internal,
            function: NodeFunction::Zero,
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }

    pub fn constant_one(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Internal,
            function: NodeFunction::One,
            fanins: Vec::new(),
            cubes: Vec::new(),
        }
    }

    pub fn buffer(name: impl Into<String>, fanin: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Internal,
            function: NodeFunction::Buffer,
            fanins: vec![fanin.into()],
            cubes: vec![vec![LiteralPhase::Positive]],
        }
    }

    pub fn inverter(name: impl Into<String>, fanin: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Internal,
            function: NodeFunction::Inverter,
            fanins: vec![fanin.into()],
            cubes: vec![vec![LiteralPhase::Negative]],
        }
    }

    pub fn with_fanins(mut self, fanins: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.fanins = fanins.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_cubes(mut self, cubes: Vec<Vec<LiteralPhase>>) -> Self {
        self.cubes = cubes;
        self
    }

    pub fn cube_count(&self) -> usize {
        self.cubes.len()
    }

    pub fn literal_count(&self) -> usize {
        self.cubes
            .iter()
            .flatten()
            .filter(|phase| phase.is_present())
            .count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverMatrix {
    rows: Vec<Vec<LiteralPhase>>,
    column_names: Vec<String>,
}

impl CoverMatrix {
    pub fn new(column_names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            rows: Vec::new(),
            column_names: column_names.into_iter().map(Into::into).collect(),
        }
    }

    pub fn from_rows(
        column_names: impl IntoIterator<Item = impl Into<String>>,
        rows: Vec<Vec<LiteralPhase>>,
    ) -> IteNewUrpResult<Self> {
        let mut matrix = Self::new(column_names);
        for row in rows {
            matrix.push_row(row)?;
        }
        Ok(matrix)
    }

    pub fn push_row(&mut self, row: Vec<LiteralPhase>) -> IteNewUrpResult<()> {
        if row.len() != self.column_names.len() {
            return Err(IteNewUrpError::RowWidthMismatch {
                expected: self.column_names.len(),
                actual: row.len(),
            });
        }
        self.rows.push(row);
        Ok(())
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.column_names.len()
    }

    pub fn rows(&self) -> &[Vec<LiteralPhase>] {
        &self.rows
    }

    pub fn column_names(&self) -> &[String] {
        &self.column_names
    }

    pub fn evaluate(&self, inputs: &[bool]) -> IteNewUrpResult<bool> {
        if inputs.len() != self.column_count() {
            return Err(IteNewUrpError::InputWidthMismatch {
                expected: self.column_count(),
                actual: inputs.len(),
            });
        }

        Ok(self.rows.iter().any(|row| {
            row.iter()
                .zip(inputs)
                .all(|(phase, value)| phase.matches(*value))
        }))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ite {
    Terminal(bool),
    Literal {
        column: usize,
        name: String,
        phase: bool,
    },
    Shannon {
        condition: Box<Ite>,
        then_branch: Box<Ite>,
        else_branch: Box<Ite>,
    },
    LeafCube(CoverMatrix),
    LeafSingleLiteralCubes(CoverMatrix),
    LeafUnate(CoverMatrix),
    LeafActMatch(String),
}

impl Ite {
    pub fn evaluate(&self, inputs: &[bool]) -> IteNewUrpResult<bool> {
        match self {
            Self::Terminal(value) => Ok(*value),
            Self::Literal { column, phase, .. } => {
                let value = inputs
                    .get(*column)
                    .copied()
                    .ok_or(IteNewUrpError::UnknownColumn(*column))?;
                Ok(if *phase { value } else { !value })
            }
            Self::Shannon {
                condition,
                then_branch,
                else_branch,
            } => {
                if condition.evaluate(inputs)? {
                    then_branch.evaluate(inputs)
                } else {
                    else_branch.evaluate(inputs)
                }
            }
            Self::LeafCube(matrix)
            | Self::LeafSingleLiteralCubes(matrix)
            | Self::LeafUnate(matrix) => matrix.evaluate(inputs),
            Self::LeafActMatch(_) => Err(IteNewUrpError::ActMatchEvaluationUnavailable),
        }
    }

    pub fn node_count(&self) -> usize {
        match self {
            Self::Terminal(_) => 1,
            Self::Literal { .. } => 1,
            Self::Shannon {
                condition,
                then_branch,
                else_branch,
            } => 1 + condition.node_count() + then_branch.node_count() + else_branch.node_count(),
            Self::LeafCube(matrix)
            | Self::LeafSingleLiteralCubes(matrix)
            | Self::LeafUnate(matrix) => 1 + matrix.literal_count(),
            Self::LeafActMatch(_) => 1,
        }
    }

    pub fn is_inverter(&self) -> bool {
        match self {
            Self::Literal { phase, .. } => !*phase,
            Self::Shannon {
                condition,
                then_branch,
                else_branch,
            } => {
                matches!(condition.as_ref(), Self::Literal { phase: true, .. })
                    && matches!(then_branch.as_ref(), Self::Terminal(false))
                    && matches!(else_branch.as_ref(), Self::Terminal(true))
            }
            _ => false,
        }
    }
}

trait MatrixLiteralCount {
    fn literal_count(&self) -> usize;
}

impl MatrixLiteralCount for CoverMatrix {
    fn literal_count(&self) -> usize {
        self.rows
            .iter()
            .flatten()
            .filter(|phase| phase.is_present())
            .count()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitParams {
    pub var_selection_lit: isize,
    pub use_unate_leaf: bool,
    pub use_factored_form_when_unate: bool,
}

impl Default for InitParams {
    fn default() -> Self {
        Self {
            var_selection_lit: 0,
            use_unate_leaf: true,
            use_factored_form_when_unate: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VariableChoice {
    pub column: usize,
    pub occurrence_count: usize,
    pub difference_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeightedVariableChoice {
    pub column: usize,
    pub weight: isize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PosNegLiteralCount {
    pub positive: usize,
    pub negative: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MuxRemainder {
    Full,
    HasUnusedMux,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IteNewUrpError {
    PrimaryNode {
        kind: NodeKind,
    },
    RowWidthMismatch {
        expected: usize,
        actual: usize,
    },
    InputWidthMismatch {
        expected: usize,
        actual: usize,
    },
    UnknownColumn(usize),
    MissingFanin {
        column: usize,
    },
    InvalidLiteralValue(i32),
    InvalidCubeCount {
        actual: usize,
    },
    BinateLiteralInCube {
        column: usize,
    },
    ActMatchEvaluationUnavailable,
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for IteNewUrpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrimaryNode { kind } => write!(
                f,
                "act_ite_new_make_ite requires an internal node, got {kind:?}"
            ),
            Self::RowWidthMismatch { expected, actual } => {
                write!(f, "cover row has width {actual}, expected {expected}")
            }
            Self::InputWidthMismatch { expected, actual } => {
                write!(f, "input vector has width {actual}, expected {expected}")
            }
            Self::UnknownColumn(column) => write!(f, "unknown cover column {column}"),
            Self::MissingFanin { column } => write!(f, "cover column {column} has no fanin name"),
            Self::InvalidLiteralValue(value) => {
                write!(f, "invalid SIS literal value {value}; expected 0, 1, or 2")
            }
            Self::InvalidCubeCount { actual } => {
                write!(f, "expected exactly one cube, got {actual}")
            }
            Self::BinateLiteralInCube { column } => {
                write!(f, "cube column {column} is binate or unknown")
            }
            Self::ActMatchEvaluationUnavailable => {
                write!(f, "ACT match leaf evaluation requires the ACT matcher port")
            }
            Self::MissingNativePorts {
                operation,
                dependencies,
            } => write!(
                f,
                "{operation} is blocked by {} unported SIS C-file dependencies",
                dependencies.len()
            ),
        }
    }
}

impl Error for IteNewUrpError {}

pub type IteNewUrpResult<T> = Result<T, IteNewUrpError>;

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn act_ite_new_map_node_blocked<Node>(
    _node: &mut Node,
    _init_params: InitParams,
) -> IteNewUrpResult<usize> {
    Err(missing_native_ports(
        "act_ite_new_map_node SIS node/cost-slot integration",
    ))
}

pub fn act_ite_intermediate_new_make_ite_blocked<Node>(
    _node: &mut Node,
    _init_params: InitParams,
) -> IteNewUrpResult<Ite> {
    Err(missing_native_ports(
        "act_ite_intermediate_new_make_ite SIS node/global-terminal integration",
    ))
}

pub fn act_ite_new_make_ite_blocked<Node>(
    _node: &mut Node,
    _init_params: InitParams,
) -> IteNewUrpResult<Ite> {
    Err(missing_native_ports(
        "act_ite_new_make_ite SIS recursive ITE integration",
    ))
}

pub fn act_ite_new_make_ite(node: &CoverNode, init_params: InitParams) -> IteNewUrpResult<Ite> {
    match node.kind {
        NodeKind::Internal => {}
        kind => return Err(IteNewUrpError::PrimaryNode { kind }),
    }

    match node.function {
        NodeFunction::Zero => return Ok(Ite::Terminal(false)),
        NodeFunction::One => return Ok(Ite::Terminal(true)),
        NodeFunction::Buffer => return Ok(ite_buffer(0, fanin_name(node, 0)?.to_owned())),
        NodeFunction::Inverter => return Ok(ite_inv(0, fanin_name(node, 0)?.to_owned())),
        NodeFunction::Cover => {}
    }

    let cover = build_f(node)?;
    if node.cube_count() == 1 {
        return Ok(Ite::LeafCube(cover));
    }
    if node.literal_count() == node.cube_count() {
        return Ok(Ite::LeafSingleLiteralCubes(cover));
    }
    if node.literal_count() == node.fanins.len() {
        return ite_create_ite_for_orthogonal_cubes(&cover);
    }
    if init_params.use_unate_leaf && node_is_unate(&cover) {
        return Ok(ite_new_ite_for_unate_cover(&cover));
    }

    let variable = select_variable(&cover, init_params)?;
    let (then_matrix, else_matrix) = algebraic_cofactor_pair(&cover, variable.column)?;
    let variable_ite = ite_literal(variable.column, column_name(&cover, variable.column)?, true);
    let then_ite = urp_from_cover(&then_matrix, init_params)?;
    let else_ite = urp_from_cover(&else_matrix, init_params)?;
    Ok(my_shannon_ite(variable_ite, then_ite, else_ite))
}

pub fn build_f(node: &CoverNode) -> IteNewUrpResult<CoverMatrix> {
    CoverMatrix::from_rows(node.fanins.iter().cloned(), node.cubes.clone())
}

pub fn node_most_binate_variable(matrix: &CoverMatrix) -> Option<VariableChoice> {
    let counts = node_literal_count(matrix);
    choose_most_binate_from_counts(matrix.row_count(), &counts)
}

pub fn node_most_binate_variable_new(
    matrix: &CoverMatrix,
    init_params: InitParams,
) -> IteNewUrpResult<Option<WeightedVariableChoice>> {
    let counts = node_literal_count(matrix);
    if is_unate_counts(&counts) {
        if init_params.use_factored_form_when_unate {
            return Ok(None);
        }
        return Ok(
            choose_most_frequent_unate(&counts).map(|column| WeightedVariableChoice {
                column,
                weight: (counts[column].0 + counts[column].1) as isize,
            }),
        );
    }

    if matrix.literal_count() >= init_params.var_selection_lit.max(0) as usize {
        let mut best: Option<WeightedVariableChoice> = None;
        for column in 0..matrix.column_count() {
            let weight = ite_assign_var_weight(matrix, column)?;
            if best.is_none_or(|candidate| weight > candidate.weight) {
                best = Some(WeightedVariableChoice { column, weight });
            }
        }
        return Ok(best);
    }

    Ok(
        node_most_binate_variable(matrix).map(|choice| WeightedVariableChoice {
            column: choice.column,
            weight: choice.occurrence_count as isize,
        }),
    )
}

pub fn ite_get_minimum_cost_variable(matrix: &CoverMatrix) -> IteNewUrpResult<Option<usize>> {
    let mut best_column = None;
    let mut best_cost = usize::MAX;
    for column in 0..matrix.column_count() {
        let cost = ite_assign_var_cost(matrix, column)?;
        if cost < best_cost {
            best_cost = cost;
            best_column = Some(column);
        }
    }
    Ok(best_column)
}

pub fn ite_assign_var_weight(matrix: &CoverMatrix, column: usize) -> IteNewUrpResult<isize> {
    ensure_column(matrix, column)?;
    let counts = node_literal_count(matrix);
    let (positive, negative) = counts[column];
    let weight1 = (positive + negative) as isize;
    let weight2 = isize::from(positive > 0 && negative > 0);
    let (then_matrix, else_matrix) = algebraic_cofactor_pair(matrix, column)?;
    let total_binateness = node_compute_binateness(matrix)
        + node_compute_binateness(&then_matrix)
        + node_compute_binateness(&else_matrix);
    Ok(weight1 + ACT_ITE_ALPHA * weight2 + ACT_ITE_GAMMA * total_binateness as isize)
}

pub fn ite_assign_var_cost(matrix: &CoverMatrix, column: usize) -> IteNewUrpResult<usize> {
    ensure_column(matrix, column)?;
    let (then_matrix, else_matrix) = algebraic_cofactor_pair(matrix, column)?;
    Ok(cover_cost_estimate(&then_matrix) + cover_cost_estimate(&else_matrix))
}

pub fn node_input_phase(matrix: &CoverMatrix, column: usize) -> IteNewUrpResult<InputPhase> {
    ensure_column(matrix, column)?;
    let mut positive = false;
    let mut negative = false;
    for row in matrix.rows() {
        match row[column] {
            LiteralPhase::Positive => positive = true,
            LiteralPhase::Negative => negative = true,
            LiteralPhase::DontCare => {}
        }
    }
    Ok(match (positive, negative) {
        (true, true) => InputPhase::Binate,
        (true, false) => InputPhase::PositiveUnate,
        (false, true) => InputPhase::NegativeUnate,
        (false, false) => InputPhase::Unknown,
    })
}

pub fn node_is_unate(matrix: &CoverMatrix) -> bool {
    (0..matrix.column_count()).all(|column| {
        !matches!(
            node_input_phase(matrix, column),
            Ok(InputPhase::Binate) | Err(_)
        )
    })
}

pub fn node_num_binate_variables(matrix: &CoverMatrix) -> usize {
    (0..matrix.column_count())
        .filter(|column| matches!(node_input_phase(matrix, *column), Ok(InputPhase::Binate)))
        .count()
}

pub fn node_compute_binateness(matrix: &CoverMatrix) -> usize {
    if matrix.column_count() == 0 {
        0
    } else {
        node_num_binate_variables(matrix)
    }
}

pub fn pld_find_pos_neg_litcount_in_cube(
    cube_node: &CoverMatrix,
) -> IteNewUrpResult<PosNegLiteralCount> {
    if cube_node.row_count() != 1 {
        return Err(IteNewUrpError::InvalidCubeCount {
            actual: cube_node.row_count(),
        });
    }

    let mut count = PosNegLiteralCount {
        positive: 0,
        negative: 0,
    };
    for (column, phase) in cube_node.rows()[0].iter().enumerate() {
        match phase {
            LiteralPhase::Positive => count.positive += 1,
            LiteralPhase::Negative => count.negative += 1,
            LiteralPhase::DontCare => {}
        }
        if matches!(node_input_phase(cube_node, column)?, InputPhase::Binate) {
            return Err(IteNewUrpError::BinateLiteralInCube { column });
        }
    }
    Ok(count)
}

pub fn ite_find_mux_remaining(positive: usize, negative: usize, count: usize) -> MuxRemainder {
    if find_mux_remaining_bool(positive, negative, count) {
        MuxRemainder::HasUnusedMux
    } else {
        MuxRemainder::Full
    }
}

pub fn ite_create_ite_for_orthogonal_cubes(matrix: &CoverMatrix) -> IteNewUrpResult<Ite> {
    let mut muxfree = Vec::new();
    let mut full = Vec::new();

    for row in matrix.rows() {
        let cube =
            CoverMatrix::from_rows(matrix.column_names().iter().cloned(), vec![row.clone()])?;
        let count = pld_find_pos_neg_litcount_in_cube(&cube)?;
        let ite = Ite::LeafCube(cube);
        match ite_find_mux_remaining(count.positive, count.negative, 0) {
            MuxRemainder::HasUnusedMux => muxfree.push(ite),
            MuxRemainder::Full => full.push(ite),
        }
    }

    Ok(ite_interleave_muxfree_and_full(muxfree, full).unwrap_or(Ite::Terminal(false)))
}

pub fn ite_interleave_muxfree_and_full(mut muxfree: Vec<Ite>, mut full: Vec<Ite>) -> Option<Ite> {
    let mut out = Vec::new();
    while !muxfree.is_empty() || !full.is_empty() {
        if let Some(ite) = muxfree.pop() {
            out.push(ite);
        }
        for _ in 0..2 {
            if let Some(ite) = full.pop() {
                out.push(ite);
            }
        }
    }
    out.into_iter().reduce(ite_new_or)
}

pub fn ite_new_or(ite_a: Ite, ite_b: Ite) -> Ite {
    my_shannon_ite(ite_a, Ite::Terminal(true), ite_b)
}

pub fn ite_new_or_with_inv(ite_a: Ite, ite_b: Ite) -> Ite {
    if ite_a.is_inverter() {
        my_shannon_ite(invert_literal_root(ite_a), ite_b, Ite::Terminal(true))
    } else {
        ite_new_or(ite_a, ite_b)
    }
}

pub fn ite_is_inv(ite: &Ite) -> bool {
    ite.is_inverter()
}

pub fn ite_new_ite_for_unate_cover(matrix: &CoverMatrix) -> Ite {
    Ite::LeafUnate(matrix.clone())
}

fn urp_from_cover(matrix: &CoverMatrix, init_params: InitParams) -> IteNewUrpResult<Ite> {
    if matrix.row_count() == 0 {
        return Ok(Ite::Terminal(false));
    }
    if matrix.row_count() == 1 {
        return Ok(Ite::LeafCube(matrix.clone()));
    }
    if matrix.literal_count() == matrix.row_count() {
        return Ok(Ite::LeafSingleLiteralCubes(matrix.clone()));
    }
    if init_params.use_unate_leaf && node_is_unate(matrix) {
        return Ok(ite_new_ite_for_unate_cover(matrix));
    }
    let variable = select_variable(matrix, init_params)?;
    let (then_matrix, else_matrix) = algebraic_cofactor_pair(matrix, variable.column)?;
    let variable_ite = ite_literal(variable.column, column_name(matrix, variable.column)?, true);
    Ok(my_shannon_ite(
        variable_ite,
        urp_from_cover(&then_matrix, init_params)?,
        urp_from_cover(&else_matrix, init_params)?,
    ))
}

fn select_variable(
    matrix: &CoverMatrix,
    init_params: InitParams,
) -> IteNewUrpResult<WeightedVariableChoice> {
    if init_params.var_selection_lit == -1 {
        return Ok(WeightedVariableChoice {
            column: ite_get_minimum_cost_variable(matrix)?
                .ok_or(IteNewUrpError::UnknownColumn(0))?,
            weight: 0,
        });
    }
    if init_params.var_selection_lit > 0 {
        if let Some(choice) = node_most_binate_variable_new(matrix, init_params)? {
            return Ok(choice);
        }
    }
    let choice = node_most_binate_variable(matrix).ok_or(IteNewUrpError::UnknownColumn(0))?;
    Ok(WeightedVariableChoice {
        column: choice.column,
        weight: choice.occurrence_count as isize,
    })
}

fn algebraic_cofactor_pair(
    matrix: &CoverMatrix,
    column: usize,
) -> IteNewUrpResult<(CoverMatrix, CoverMatrix)> {
    ensure_column(matrix, column)?;
    let mut then_matrix = CoverMatrix::new(matrix.column_names().iter().cloned());
    let mut else_matrix = CoverMatrix::new(matrix.column_names().iter().cloned());

    for row in matrix.rows() {
        match row[column] {
            LiteralPhase::Positive => {
                let mut copied = row.clone();
                copied[column] = LiteralPhase::DontCare;
                then_matrix.push_row(copied)?;
            }
            LiteralPhase::Negative => {
                let mut copied = row.clone();
                copied[column] = LiteralPhase::DontCare;
                else_matrix.push_row(copied)?;
            }
            LiteralPhase::DontCare => {
                then_matrix.push_row(row.clone())?;
                else_matrix.push_row(row.clone())?;
            }
        }
    }

    Ok((then_matrix, else_matrix))
}

fn node_literal_count(matrix: &CoverMatrix) -> Vec<(usize, usize)> {
    let mut counts = vec![(0usize, 0usize); matrix.column_count()];
    for row in matrix.rows() {
        for (column, phase) in row.iter().enumerate() {
            match phase {
                LiteralPhase::Positive => counts[column].0 += 1,
                LiteralPhase::Negative => counts[column].1 += 1,
                LiteralPhase::DontCare => {}
            }
        }
    }
    counts
}

fn choose_most_binate_from_counts(
    cube_count: usize,
    counts: &[(usize, usize)],
) -> Option<VariableChoice> {
    let mut best: Option<VariableChoice> = None;
    for (column, (positive, negative)) in counts.iter().copied().enumerate() {
        let occurrence_count = positive + negative;
        let difference_count = positive.abs_diff(negative);
        let candidate = VariableChoice {
            column,
            occurrence_count,
            difference_count,
        };
        if best.is_none_or(|current| {
            occurrence_count > current.occurrence_count
                || (occurrence_count == current.occurrence_count
                    && (difference_count == cube_count
                        || (current.difference_count != cube_count
                            && difference_count < current.difference_count)))
        }) {
            best = Some(candidate);
        }
    }
    best.filter(|choice| choice.occurrence_count > 0)
}

fn is_unate_counts(counts: &[(usize, usize)]) -> bool {
    counts
        .iter()
        .all(|(positive, negative)| *positive == 0 || *negative == 0)
}

fn choose_most_frequent_unate(counts: &[(usize, usize)]) -> Option<usize> {
    let mut best = None;
    let mut best_count = 0usize;
    for (column, (positive, negative)) in counts.iter().copied().enumerate() {
        let count = positive + negative;
        if count > best_count || (count == best_count && positive == 0 && count > 0) {
            best = Some(column);
            best_count = count;
        }
    }
    best
}

fn cover_cost_estimate(matrix: &CoverMatrix) -> usize {
    if matrix.row_count() == 0 {
        0
    } else {
        matrix.literal_count().max(1)
    }
}

fn find_mux_remaining_bool(positive: usize, negative: usize, count: usize) -> bool {
    if positive == 0 && negative == 0 {
        return false;
    }
    if count == 0 {
        if positive == 1 && negative == 0 {
            return false;
        }
        if negative == 0 {
            if positive == 2 {
                return true;
            }
            return (positive - 3) % 2 != 0;
        }
        if positive == 0 {
            let q_3 = negative % 3;
            return q_3 != 0 && q_3 != 2;
        }
        if negative == 1 {
            if positive == 1 {
                return true;
            }
            return (positive - 2) % 2 != 0;
        }
        if positive == 1 {
            let q_3 = negative % 3;
            return q_3 != 0 && q_3 != 2;
        }
        return find_mux_remaining_bool(positive - 2, negative - 2, 1);
    }
    if negative == 0 {
        return positive % 2 != 0;
    }
    if positive == 0 {
        let q_3 = negative % 3;
        return q_3 != 0 && q_3 != 2;
    }
    if negative == 1 {
        return (positive - 1) % 2 != 0;
    }
    find_mux_remaining_bool(negative - 2, positive - 1, 1)
}

fn ite_buffer(column: usize, name: String) -> Ite {
    my_shannon_ite(
        ite_literal(column, &name, true),
        Ite::Terminal(true),
        Ite::Terminal(false),
    )
}

fn ite_inv(column: usize, name: String) -> Ite {
    my_shannon_ite(
        ite_literal(column, &name, true),
        Ite::Terminal(false),
        Ite::Terminal(true),
    )
}

fn ite_literal(column: usize, name: &str, phase: bool) -> Ite {
    Ite::Literal {
        column,
        name: name.to_owned(),
        phase,
    }
}

fn my_shannon_ite(condition: Ite, then_branch: Ite, else_branch: Ite) -> Ite {
    Ite::Shannon {
        condition: Box::new(condition),
        then_branch: Box::new(then_branch),
        else_branch: Box::new(else_branch),
    }
}

fn invert_literal_root(ite: Ite) -> Ite {
    match ite {
        Ite::Literal {
            column,
            name,
            phase,
        } => Ite::Literal {
            column,
            name,
            phase: !phase,
        },
        Ite::Shannon {
            condition,
            then_branch,
            else_branch,
        } => Ite::Shannon {
            condition,
            then_branch: else_branch,
            else_branch: then_branch,
        },
        other => other,
    }
}

fn fanin_name(node: &CoverNode, column: usize) -> IteNewUrpResult<&str> {
    node.fanins
        .get(column)
        .map(String::as_str)
        .filter(|name| !name.is_empty())
        .ok_or(IteNewUrpError::MissingFanin { column })
}

fn column_name(matrix: &CoverMatrix, column: usize) -> IteNewUrpResult<&str> {
    matrix
        .column_names()
        .get(column)
        .map(String::as_str)
        .filter(|name| !name.is_empty())
        .ok_or(IteNewUrpError::MissingFanin { column })
}

fn ensure_column(matrix: &CoverMatrix, column: usize) -> IteNewUrpResult<()> {
    if column >= matrix.column_count() {
        Err(IteNewUrpError::UnknownColumn(column))
    } else {
        Ok(())
    }
}

fn missing_native_ports(operation: &'static str) -> IteNewUrpError {
    IteNewUrpError::MissingNativePorts {
        operation,
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phases(values: &[i32]) -> Vec<LiteralPhase> {
        values
            .iter()
            .map(|value| LiteralPhase::from_sis_literal(*value).unwrap())
            .collect()
    }

    fn matrix(rows: &[&[i32]]) -> CoverMatrix {
        CoverMatrix::from_rows(
            ["a", "b", "c"],
            rows.iter().map(|row| phases(row)).collect(),
        )
        .unwrap()
    }

    #[test]
    fn constants_and_literal_nodes_follow_c_fast_paths() {
        assert_eq!(
            act_ite_new_make_ite(&CoverNode::constant_zero("z"), InitParams::default()).unwrap(),
            Ite::Terminal(false)
        );
        assert_eq!(
            act_ite_new_make_ite(&CoverNode::constant_one("o"), InitParams::default()).unwrap(),
            Ite::Terminal(true)
        );

        let buffer =
            act_ite_new_make_ite(&CoverNode::buffer("n", "a"), InitParams::default()).unwrap();
        let inverter =
            act_ite_new_make_ite(&CoverNode::inverter("n", "a"), InitParams::default()).unwrap();

        assert_eq!(buffer.evaluate(&[true]).unwrap(), true);
        assert_eq!(buffer.evaluate(&[false]).unwrap(), false);
        assert_eq!(inverter.evaluate(&[true]).unwrap(), false);
        assert_eq!(inverter.evaluate(&[false]).unwrap(), true);
    }

    #[test]
    fn build_f_preserves_cover_rows_and_fanin_names() {
        let node = CoverNode::new("n", NodeKind::Internal)
            .with_fanins(["a", "b", "c"])
            .with_cubes(vec![phases(&[1, 0, 2]), phases(&[2, 1, 0])]);
        let cover = build_f(&node).unwrap();

        assert_eq!(cover.column_names(), ["a", "b", "c"]);
        assert_eq!(cover.rows(), &[phases(&[1, 0, 2]), phases(&[2, 1, 0])]);
    }

    #[test]
    fn most_binate_variable_uses_occurrence_then_phase_balance() {
        let cover = matrix(&[&[1, 0, 2], &[0, 1, 2], &[1, 1, 0]]);

        assert_eq!(
            node_most_binate_variable(&cover),
            Some(VariableChoice {
                column: 0,
                occurrence_count: 3,
                difference_count: 1
            })
        );
    }

    #[test]
    fn new_variable_selection_prefers_weighted_binate_when_threshold_is_met() {
        let cover = matrix(&[&[1, 0, 2], &[0, 1, 2], &[1, 1, 0]]);
        let choice = node_most_binate_variable_new(
            &cover,
            InitParams {
                var_selection_lit: 2,
                ..InitParams::default()
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(choice.column, 0);
        assert!(choice.weight > 3);
    }

    #[test]
    fn unate_selection_breaks_ties_in_favor_of_negative_phase() {
        let cover = matrix(&[&[1, 0, 2], &[2, 0, 1]]);
        let choice = node_most_binate_variable_new(&cover, InitParams::default())
            .unwrap()
            .unwrap();

        assert_eq!(choice.column, 1);
    }

    #[test]
    fn variable_cost_uses_smaller_cofactor_cover_estimate() {
        let cover = matrix(&[&[1, 0, 2], &[0, 1, 2], &[1, 1, 0]]);

        assert_eq!(ite_get_minimum_cost_variable(&cover).unwrap(), Some(0));
        assert!(ite_assign_var_cost(&cover, 0).unwrap() <= ite_assign_var_cost(&cover, 2).unwrap());
    }

    #[test]
    fn phase_and_binateness_match_cover_literals() {
        let cover = matrix(&[&[1, 0, 2], &[0, 2, 2]]);

        assert_eq!(node_input_phase(&cover, 0).unwrap(), InputPhase::Binate);
        assert_eq!(
            node_input_phase(&cover, 1).unwrap(),
            InputPhase::NegativeUnate
        );
        assert_eq!(node_input_phase(&cover, 2).unwrap(), InputPhase::Unknown);
        assert_eq!(node_num_binate_variables(&cover), 1);
        assert!(!node_is_unate(&cover));
    }

    #[test]
    fn pos_neg_litcount_requires_single_cube_and_counts_phases() {
        let count = pld_find_pos_neg_litcount_in_cube(&matrix(&[&[1, 0, 2]])).unwrap();

        assert_eq!(
            count,
            PosNegLiteralCount {
                positive: 1,
                negative: 1
            }
        );
        assert!(matches!(
            pld_find_pos_neg_litcount_in_cube(&matrix(&[&[1, 0, 2], &[0, 1, 2]])),
            Err(IteNewUrpError::InvalidCubeCount { actual: 2 })
        ));
    }

    #[test]
    fn mux_remainder_matches_c_boundary_cases() {
        assert_eq!(ite_find_mux_remaining(0, 0, 0), MuxRemainder::Full);
        assert_eq!(ite_find_mux_remaining(1, 0, 0), MuxRemainder::Full);
        assert_eq!(ite_find_mux_remaining(2, 0, 0), MuxRemainder::HasUnusedMux);
        assert_eq!(ite_find_mux_remaining(1, 1, 0), MuxRemainder::HasUnusedMux);
        assert_eq!(ite_find_mux_remaining(0, 3, 0), MuxRemainder::Full);
    }

    #[test]
    fn orthogonal_cube_path_interleaves_muxfree_and_full_cubes() {
        let cover = matrix(&[&[1, 0, 2], &[2, 2, 1]]);
        let ite = ite_create_ite_for_orthogonal_cubes(&cover).unwrap();

        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let inputs = [a, b, c];
                    assert_eq!(
                        ite.evaluate(&inputs).unwrap(),
                        cover.evaluate(&inputs).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn recursive_ite_evaluates_same_function_as_cover_when_leaf_fast_paths_disabled() {
        let cover = matrix(&[&[1, 0, 2], &[0, 1, 2], &[1, 1, 0]]);
        let node = CoverNode::new("n", NodeKind::Internal)
            .with_fanins(["a", "b", "c"])
            .with_cubes(cover.rows().to_vec());
        let ite = act_ite_new_make_ite(
            &node,
            InitParams {
                use_unate_leaf: false,
                ..InitParams::default()
            },
        )
        .unwrap();

        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    let inputs = [a, b, c];
                    assert_eq!(
                        ite.evaluate(&inputs).unwrap(),
                        cover.evaluate(&inputs).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn inverter_detection_and_or_with_inv_match_c_helper_intent() {
        let inv = ite_inv(0, "a".to_owned());
        assert!(ite_is_inv(&inv));

        let or = ite_new_or_with_inv(inv, ite_literal(1, "b", true));
        assert_eq!(or.evaluate(&[false, false]).unwrap(), true);
        assert_eq!(or.evaluate(&[true, false]).unwrap(), false);
        assert_eq!(or.evaluate(&[true, true]).unwrap(), true);
    }

    #[test]
    fn blocked_sis_entries_report_dependency_beads_and_source_files() {
        let error = act_ite_new_make_ite_blocked(&mut (), InitParams::default()).unwrap_err();
        let IteNewUrpError::MissingNativePorts {
            operation,
            dependencies,
        } = error
        else {
            panic!("expected missing dependency error");
        };

        assert_eq!(
            operation,
            "act_ite_new_make_ite SIS recursive ITE integration"
        );
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.318"
                && dependency.source_file == "LogicSynthesis/sis/node/node.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.344"
                && dependency.source_file == "LogicSynthesis/sis/pld/act_bool.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.457"
                && dependency.source_file == "LogicSynthesis/sis/sparse/matrix.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.485"
                && dependency.source_file == "LogicSynthesis/sis/st/st.c"
        }));
    }
}
