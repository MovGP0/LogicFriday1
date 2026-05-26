//! Native Rust model for `LogicSynthesis/sis/power/power_psAppr.c`.
//!
//! The C file computes approximate present-state line probabilities directly
//! from next-state logic. This port keeps the reusable algorithmic pieces in
//! owned Rust types: present-state ordering, grouped next-state output
//! descriptors, BDD cofactor probability scans, and the Newton/LU update used
//! by the fixed-point iteration. Direct SIS `network_t`, `array_t`, `st_table`,
//! and CMU BDD integration is reported as an explicit missing dependency.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;

pub const DEFAULT_PS_MAX_ALLOWED_ERROR: f64 = 0.01;
const TINY: f64 = 1.0e-20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.2",
        source_file: "LogicSynthesis/sis/array/array.c",
        reason: "power_psAppr.c passes node orderings and generated NS outputs in array_t",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.299",
        source_file: "LogicSynthesis/sis/network/net_seq.c",
        reason: "network_latch_end maps NS primary outputs back to present-state lines",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.305",
        source_file: "LogicSynthesis/sis/network/network_util.c",
        reason: "network_dup, primary IO iteration, deletion, lookup, connection, and csweep",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.318",
        source_file: "LogicSynthesis/sis/node/node.c",
        reason: "node allocation, replacement, SCC minimization, names, and covers",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.313",
        source_file: "LogicSynthesis/sis/node/fan.c",
        reason: "nodevec_dup and fanin rewiring for generated grouped NS logic",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.329",
        source_file: "LogicSynthesis/sis/ntbdd/manager.c",
        reason: "ntbdd_start_manager and ntbdd_end_manager BDD manager lifetime",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.330",
        source_file: "LogicSynthesis/sis/ntbdd/node_to_bdd.c",
        reason: "ntbdd_node_to_bdd converts generated NS logic nodes to BDDs",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.407",
        source_file: "LogicSynthesis/sis/power/power_util.c",
        reason: "power_lines_in_set maps literal masks to grouped PS combination indexes",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.442",
        source_file: "LogicSynthesis/sis/seqbdd/verif_util.c",
        reason: "order_nodes supplies the PI order used before placing PS lines first",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "info_table, leaves, and visited caches are st_table instances in the C path",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.400",
        source_file: "LogicSynthesis/sis/power/power_main.c",
        reason: "node_info_t probability data is populated by power_get_node_info",
    },
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerPiInfo {
    pub probability_one: f64,
    pub ps_line_index: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedNsFunction {
    pub source_set: usize,
    pub combination: usize,
    pub input_lines: Vec<NodeId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PsApproximationConfig {
    pub set_size: usize,
    pub delta: f64,
    pub max_iterations: usize,
}

impl Default for PsApproximationConfig {
    fn default() -> Self {
        Self {
            set_size: 1,
            delta: DEFAULT_PS_MAX_ALLOWED_ERROR,
            max_iterations: 100,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PsIterationState {
    pub all_probabilities: Vec<f64>,
    pub line_probabilities: Vec<f64>,
}

impl PsIterationState {
    pub fn new(n_ns_lines: usize, n_ps_lines: usize, set_size: usize) -> Result<Self, PsApprError> {
        let layout = PsSetLayout::new(n_ns_lines, n_ps_lines, set_size)?;
        Ok(Self {
            all_probabilities: layout.initial_all_probabilities(),
            line_probabilities: layout.initial_line_probabilities(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PsApproximationReport {
    pub probabilities: Vec<f64>,
    pub iterations: usize,
    pub converged: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PsApprError {
    InvalidSetSize(usize),
    InvalidProbability(f64),
    InvalidBdd(BddId),
    MissingPiInfo(usize),
    SingularMatrix,
    DimensionMismatch {
        expected: usize,
        actual: usize,
    },
    DidNotConverge {
        iterations: usize,
        max_iterations: usize,
    },
    MissingDependency {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for PsApprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSetSize(set_size) => {
                write!(f, "power_setSize must be at least 1, got {set_size}")
            }
            Self::InvalidProbability(probability) => {
                write!(f, "probability {probability} is outside 0.0..=1.0")
            }
            Self::InvalidBdd(id) => write!(f, "BDD node {:?} is not present in the arena", id),
            Self::MissingPiInfo(index) => {
                write!(f, "BDD variable {index} has no primary-input probability")
            }
            Self::SingularMatrix => write!(f, "singular matrix in LU decomposition"),
            Self::DimensionMismatch { expected, actual } => {
                write!(f, "got dimension {actual}, expected {expected}")
            }
            Self::DidNotConverge {
                iterations,
                max_iterations,
            } => write!(
                f,
                "PS approximation did not converge after {iterations}/{max_iterations} iterations"
            ),
            Self::MissingDependency {
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

impl Error for PsApprError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddId(pub usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddNode {
    Zero,
    One,
    Branch { var: usize, low: BddId, high: BddId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddArena {
    nodes: Vec<BddNode>,
}

impl Default for BddArena {
    fn default() -> Self {
        Self {
            nodes: vec![BddNode::Zero, BddNode::One],
        }
    }
}

impl BddArena {
    pub fn zero() -> BddId {
        BddId(0)
    }

    pub fn one() -> BddId {
        BddId(1)
    }

    pub fn add_branch(&mut self, var: usize, low: BddId, high: BddId) -> BddId {
        let id = BddId(self.nodes.len());
        self.nodes.push(BddNode::Branch { var, low, high });
        id
    }

    pub fn node(&self, id: BddId) -> Result<&BddNode, PsApprError> {
        self.nodes.get(id.0).ok_or(PsApprError::InvalidBdd(id))
    }
}

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

pub fn power_direct_ps_lines_prob_from_sis<Network, InfoTable>(
    _network: &Network,
    _info_table: &InfoTable,
) -> Result<PsApproximationReport, PsApprError> {
    Err(PsApprError::MissingDependency {
        operation: "power_direct_PS_lines_prob",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn place_pis_last<T>(pi_order: &mut Vec<T>, ps_order: &[T])
where
    T: Copy + Ord,
{
    let ps_rank: BTreeMap<T, usize> = ps_order
        .iter()
        .copied()
        .enumerate()
        .map(|(rank, node)| (node, rank))
        .collect();
    pi_order.sort_by_key(|node| {
        (
            !ps_rank.contains_key(node),
            ps_rank.get(node).copied().unwrap_or(usize::MAX),
        )
    });
}

pub fn generated_ns_functions(
    ps_order: &[NodeId],
    set_size: usize,
) -> Result<Vec<GeneratedNsFunction>, PsApprError> {
    if set_size == 0 {
        return Err(PsApprError::InvalidSetSize(set_size));
    }

    let mut functions = Vec::new();
    for (set_index, chunk) in ps_order.chunks(set_size).enumerate() {
        let n_new_ns = (1usize << chunk.len()) - 1;
        for combination in 0..n_new_ns {
            functions.push(GeneratedNsFunction {
                source_set: set_index,
                combination,
                input_lines: chunk.to_vec(),
            });
        }
    }
    Ok(functions)
}

pub fn calc_func_cofact_prob(
    arena: &BddArena,
    root: BddId,
    ps_prob: &[f64],
    n_ns_lines: usize,
    n_ps_lines: usize,
    set_size: usize,
    pi_info: &[PowerPiInfo],
) -> Result<Vec<f64>, PsApprError> {
    if set_size == 0 {
        return Err(PsApprError::InvalidSetSize(set_size));
    }
    validate_probabilities(ps_prob)?;
    for info in pi_info {
        validate_probability(info.probability_one)?;
    }

    let mut cache = HashMap::new();
    let sets = LiteralSet::full(set_size);
    let result = count_f_cof_prob_ps(
        arena, root, ps_prob, n_ns_lines, n_ps_lines, set_size, pi_info, None, sets, &mut cache,
    )?;

    Ok(result.unwrap_or_else(|| vec![0.0; 2 * n_ns_lines + 1]))
}

pub fn approximate_ps_lines(
    arena: &BddArena,
    ns_functions: &[BddId],
    n_ps_lines: usize,
    pi_info: &[PowerPiInfo],
    config: PsApproximationConfig,
) -> Result<PsApproximationReport, PsApprError> {
    if config.set_size == 0 {
        return Err(PsApprError::InvalidSetSize(config.set_size));
    }

    let n_ns_lines = ns_functions.len();
    let mut state = PsIterationState::new(n_ns_lines, n_ps_lines, config.set_size)?;
    let mut y = vec![0.0; n_ns_lines];
    let mut j = vec![0.0; n_ns_lines * n_ns_lines];

    for iteration in 0..config.max_iterations {
        for (row, root) in ns_functions.iter().copied().enumerate() {
            let y_j_values = calc_func_cofact_prob(
                arena,
                root,
                &state.all_probabilities,
                n_ns_lines,
                n_ps_lines,
                config.set_size,
                pi_info,
            )?;
            for col in 0..n_ns_lines {
                j[row * n_ns_lines + col] =
                    y_j_values[2 * col] - y_j_values[2 * col + 1] + f64::from(row == col);
            }
            y[row] = state.line_probabilities[row] - y_j_values[2 * n_ns_lines];
        }

        let converged = y.iter().all(|value| value.abs() <= config.delta);
        calculate_next_iteration(
            &mut state.all_probabilities,
            &mut state.line_probabilities,
            &mut y,
            &mut j,
            config.set_size,
        )?;
        for probability in &mut state.line_probabilities {
            *probability = probability.clamp(0.0, 1.0);
        }

        if converged {
            return Ok(PsApproximationReport {
                probabilities: state.all_probabilities,
                iterations: iteration + 1,
                converged: true,
            });
        }
    }

    Err(PsApprError::DidNotConverge {
        iterations: config.max_iterations,
        max_iterations: config.max_iterations,
    })
}

pub fn calculate_next_iteration(
    all_ps_lines_prob: &mut Vec<f64>,
    ps_lines_prob: &mut Vec<f64>,
    y: &mut Vec<f64>,
    j: &mut [f64],
    set_size: usize,
) -> Result<(), PsApprError> {
    let n_ns_lines = ps_lines_prob.len();
    if y.len() != n_ns_lines {
        return Err(PsApprError::DimensionMismatch {
            expected: n_ns_lines,
            actual: y.len(),
        });
    }
    if j.len() != n_ns_lines * n_ns_lines {
        return Err(PsApprError::DimensionMismatch {
            expected: n_ns_lines * n_ns_lines,
            actual: j.len(),
        });
    }

    for row in 0..n_ns_lines {
        let mut sum = 0.0;
        for col in 0..n_ns_lines {
            sum += j[row * n_ns_lines + col] * ps_lines_prob[col];
        }
        y[row] = sum - y[row];
    }

    let index = ludcmp(j, n_ns_lines)?;
    lubksb(j, n_ns_lines, &index, y);
    std::mem::swap(y, ps_lines_prob);

    if set_size == 1 {
        all_ps_lines_prob.clone_from(ps_lines_prob);
    } else {
        let set_size_without_redundant = (1usize << set_size) - 1;
        let mut sum = 0.0;
        let mut all_index = 0;
        for (line_index, probability) in ps_lines_prob.iter().copied().enumerate() {
            all_ps_lines_prob[all_index] = probability;
            all_index += 1;
            sum += probability;
            if (line_index + 1) % set_size_without_redundant == 0 || line_index + 1 == n_ns_lines {
                all_ps_lines_prob[all_index] = 1.0 - sum;
                all_index += 1;
                sum = 0.0;
            }
        }
        if sum != 0.0 {
            all_ps_lines_prob[n_ns_lines] = 1.0 - sum;
        }
    }

    Ok(())
}

fn count_f_cof_prob_ps(
    arena: &BddArena,
    root: BddId,
    ps_prob: &[f64],
    n_ns_lines: usize,
    n_ps_lines: usize,
    set_size: usize,
    pi_info: &[PowerPiInfo],
    prev_index: Option<usize>,
    sets: LiteralSet,
    cache: &mut HashMap<BddId, Vec<f64>>,
) -> Result<Option<Vec<f64>>, PsApprError> {
    match arena.node(root)? {
        BddNode::Zero => Ok(None),
        BddNode::One => {
            let mut result = vec![1.0; 2 * n_ns_lines + 1];
            multiply_ps_prob(
                &mut result,
                prev_index,
                ps_prob,
                &sets,
                n_ns_lines,
                set_size,
            )?;
            Ok(Some(result))
        }
        BddNode::Branch { var, low, high } => {
            if let Some(cached) = cache.get(&root) {
                let mut result = cached.clone();
                multiply_ps_prob(
                    &mut result,
                    prev_index,
                    ps_prob,
                    &sets,
                    n_ns_lines,
                    set_size,
                )?;
                return Ok(Some(result));
            }

            if *var >= n_ps_lines {
                let mut result = count_f_cof_prob_pi(arena, root, n_ns_lines, pi_info, cache)?;
                if let Some(result) = result.as_mut() {
                    multiply_ps_prob(result, prev_index, ps_prob, &sets, n_ns_lines, set_size)?;
                }
                return Ok(result);
            }

            let new_set = prev_index
                .map(|previous| *var / set_size != previous / set_size)
                .unwrap_or(false);
            let mut active_sets = sets;
            let keep_sets = new_set.then(|| {
                let keep = active_sets.clone();
                active_sets = LiteralSet::full(set_size);
                keep
            });

            let mut other_set = active_sets.clone();
            other_set.remove(2 * (*var % set_size) + 1);
            let mut result = count_f_cof_prob_ps(
                arena,
                *low,
                ps_prob,
                n_ns_lines,
                n_ps_lines,
                set_size,
                pi_info,
                Some(*var),
                other_set,
                cache,
            )?;

            active_sets.remove(2 * (*var % set_size));
            let result1 = count_f_cof_prob_ps(
                arena,
                *high,
                ps_prob,
                n_ns_lines,
                n_ps_lines,
                set_size,
                pi_info,
                Some(*var),
                active_sets,
                cache,
            )?;
            merge_optional_vectors(&mut result, result1);

            if *var % set_size == 0 {
                if let Some(result) = result.as_ref() {
                    cache.insert(root, result.clone());
                }
            }

            if let (Some(result), Some(keep_sets)) = (result.as_mut(), keep_sets.as_ref()) {
                multiply_ps_prob(result, prev_index, ps_prob, keep_sets, n_ns_lines, set_size)?;
            }

            Ok(result)
        }
    }
}

fn count_f_cof_prob_pi(
    arena: &BddArena,
    root: BddId,
    n_ns_lines: usize,
    pi_info: &[PowerPiInfo],
    cache: &mut HashMap<BddId, Vec<f64>>,
) -> Result<Option<Vec<f64>>, PsApprError> {
    match arena.node(root)? {
        BddNode::Zero => Ok(None),
        BddNode::One => Ok(Some(vec![1.0; 2 * n_ns_lines + 1])),
        BddNode::Branch { var, low, high } => {
            if let Some(cached) = cache.get(&root) {
                return Ok(Some(cached.clone()));
            }

            let probability_one = pi_info
                .get(*var)
                .ok_or(PsApprError::MissingPiInfo(*var))?
                .probability_one;
            validate_probability(probability_one)?;

            let mut result = count_f_cof_prob_pi(arena, *low, n_ns_lines, pi_info, cache)?;
            if let Some(result) = result.as_mut() {
                for value in result {
                    *value *= 1.0 - probability_one;
                }
            }

            let mut result1 = count_f_cof_prob_pi(arena, *high, n_ns_lines, pi_info, cache)?;
            if let Some(result1) = result1.as_mut() {
                for value in result1.iter_mut() {
                    *value *= probability_one;
                }
            }
            merge_optional_vectors(&mut result, result1);

            if let Some(result) = result.as_ref() {
                cache.insert(root, result.clone());
            }

            Ok(result)
        }
    }
}

fn multiply_ps_prob(
    result: &mut [f64],
    index: Option<usize>,
    ps_prob: &[f64],
    sets: &LiteralSet,
    n_ns_lines: usize,
    set_size: usize,
) -> Result<(), PsApprError> {
    let Some(index) = index else {
        return Ok(());
    };

    if set_size == 1 {
        let sum_prob = if sets.contains(1) {
            ps_prob[index]
        } else {
            1.0 - ps_prob[index]
        };
        let mut i = 0;
        while i < 2 * n_ns_lines + 1 {
            if i == 2 * index {
                if sets.contains(1) {
                    result[i] = 0.0;
                    i += 2;
                } else {
                    i += 1;
                    result[i] = 0.0;
                    i += 1;
                }
            } else {
                result[i] *= sum_prob;
                i += 1;
            }
        }
        return Ok(());
    }

    let set_number = index / set_size;
    let set_without_redundant = (1usize << set_size) - 1;
    let full_set_size = set_without_redundant + 1;
    let remaining = n_ns_lines + 1 - set_number * set_without_redundant;
    let this_set = full_set_size.min(remaining);
    let included_sets = sets.included_combinations(this_set);

    let mut sum_prob = 0.0;
    for combination in 0..this_set {
        if included_sets[combination] {
            sum_prob += ps_prob[set_number * full_set_size + combination];
        }
    }

    for (i, value) in result.iter_mut().enumerate().take(2 * n_ns_lines) {
        if i / (2 * set_without_redundant) != set_number {
            *value *= sum_prob;
        }
    }
    result[2 * n_ns_lines] *= sum_prob;

    for combination in 0..this_set - 1 {
        if !included_sets[combination] {
            result[2 * (set_number * set_without_redundant + combination) + 1] = 0.0;
        }
    }
    if !included_sets[this_set - 1] {
        for combination in 0..this_set - 1 {
            result[2 * (set_number * set_without_redundant + combination)] = 0.0;
        }
    }

    Ok(())
}

fn merge_optional_vectors(left: &mut Option<Vec<f64>>, right: Option<Vec<f64>>) {
    if let Some(right) = right {
        if let Some(left) = left {
            for (left, right) in left.iter_mut().zip(right) {
                *left += right;
            }
        } else {
            *left = Some(right);
        }
    }
}

fn ludcmp(a: &mut [f64], n: usize) -> Result<Vec<usize>, PsApprError> {
    let mut index = vec![0; n];
    let mut vv = vec![0.0; n];

    for i in 0..n {
        let mut big = 0.0_f64;
        for j in 0..n {
            big = big.max(a[i * n + j].abs());
        }
        if big == 0.0 {
            return Err(PsApprError::SingularMatrix);
        }
        vv[i] = 1.0 / big;
    }

    for j in 0..n {
        for i in 0..j {
            let mut sum = a[i * n + j];
            for k in 0..i {
                sum -= a[i * n + k] * a[k * n + j];
            }
            a[i * n + j] = sum;
        }

        let mut big = 0.0;
        let mut imax = j;
        for i in j..n {
            let mut sum = a[i * n + j];
            for k in 0..j {
                sum -= a[i * n + k] * a[k * n + j];
            }
            a[i * n + j] = sum;
            let candidate = vv[i] * sum.abs();
            if candidate >= big {
                big = candidate;
                imax = i;
            }
        }

        if j != imax {
            for k in 0..n {
                a.swap(imax * n + k, j * n + k);
            }
            vv[imax] = vv[j];
        }
        index[j] = imax;
        if a[j * n + j] == 0.0 {
            a[j * n + j] = TINY;
        }
        if j != n - 1 {
            let denominator = 1.0 / a[j * n + j];
            for i in j + 1..n {
                a[i * n + j] *= denominator;
            }
        }
    }

    Ok(index)
}

fn lubksb(a: &[f64], n: usize, index: &[usize], b: &mut [f64]) {
    let mut ii = None;
    for i in 0..n {
        let ip = index[i];
        let mut sum = b[ip];
        b[ip] = b[i];
        if let Some(ii) = ii {
            for j in ii..i {
                sum -= a[i * n + j] * b[j];
            }
        } else if sum != 0.0 {
            ii = Some(i);
        }
        b[i] = sum;
    }

    for i in (0..n).rev() {
        let mut sum = b[i];
        for j in i + 1..n {
            sum -= a[i * n + j] * b[j];
        }
        b[i] = sum / a[i * n + i];
    }
}

fn validate_probability(probability: f64) -> Result<(), PsApprError> {
    if (0.0..=1.0).contains(&probability) {
        Ok(())
    } else {
        Err(PsApprError::InvalidProbability(probability))
    }
}

fn validate_probabilities(probabilities: &[f64]) -> Result<(), PsApprError> {
    for probability in probabilities {
        validate_probability(*probability)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PsSetLayout {
    n_ns_lines: usize,
    set_size: usize,
    n_sets: usize,
    ps_per_set: usize,
    remain: usize,
}

impl PsSetLayout {
    fn new(n_ns_lines: usize, n_ps_lines: usize, set_size: usize) -> Result<Self, PsApprError> {
        if set_size == 0 {
            return Err(PsApprError::InvalidSetSize(set_size));
        }
        let ps_per_set = 1usize << set_size;
        let n_sets = n_ps_lines / set_size;
        let mut remain = n_ns_lines.saturating_sub(n_sets * (ps_per_set - 1));
        if remain != 0 {
            remain += 1;
        }
        Ok(Self {
            n_ns_lines,
            set_size,
            n_sets,
            ps_per_set,
            remain,
        })
    }

    fn initial_all_probabilities(&self) -> Vec<f64> {
        let len = if self.remain == 0 {
            self.n_sets * self.ps_per_set
        } else {
            (self.n_sets + 1) * self.ps_per_set
        };
        let mut probabilities = vec![0.0; len];
        for value in probabilities.iter_mut().take(self.n_sets * self.ps_per_set) {
            *value = 1.0 / self.ps_per_set as f64;
        }
        for value in probabilities
            .iter_mut()
            .skip(self.n_sets * self.ps_per_set)
            .take(self.remain)
        {
            *value = 1.0 / self.remain as f64;
        }
        probabilities
    }

    fn initial_line_probabilities(&self) -> Vec<f64> {
        let mut probabilities = vec![0.0; self.n_ns_lines];
        for value in probabilities
            .iter_mut()
            .take(self.n_sets * (self.ps_per_set - 1))
        {
            *value = 1.0 / self.ps_per_set as f64;
        }
        for value in probabilities
            .iter_mut()
            .skip(self.n_sets * (self.ps_per_set - 1))
        {
            *value = 1.0 / self.remain as f64;
        }
        probabilities
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LiteralSet {
    allowed: Vec<bool>,
    set_size: usize,
}

impl LiteralSet {
    fn full(set_size: usize) -> Self {
        Self {
            allowed: vec![true; 2 * set_size],
            set_size,
        }
    }

    fn remove(&mut self, literal: usize) {
        if literal < self.allowed.len() {
            self.allowed[literal] = false;
        }
    }

    fn contains(&self, literal: usize) -> bool {
        self.allowed.get(literal).copied().unwrap_or(false)
    }

    fn included_combinations(&self, count: usize) -> Vec<bool> {
        (0..count)
            .map(|combination| {
                (0..self.set_size).all(|bit| {
                    let literal = if combination & (1usize << bit) == 0 {
                        2 * bit
                    } else {
                        2 * bit + 1
                    };
                    self.contains(literal)
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pi(probability_one: f64) -> PowerPiInfo {
        PowerPiInfo {
            probability_one,
            ps_line_index: None,
        }
    }

    #[test]
    fn place_pis_last_moves_ps_lines_to_front_in_ps_order() {
        let mut pi_order = vec![NodeId(4), NodeId(1), NodeId(3), NodeId(2)];

        place_pis_last(&mut pi_order, &[NodeId(3), NodeId(1)]);

        assert_eq!(pi_order, vec![NodeId(3), NodeId(1), NodeId(4), NodeId(2)]);
    }

    #[test]
    fn generated_ns_functions_match_grouped_output_logic_counts() {
        let generated = generated_ns_functions(&[NodeId(0), NodeId(1), NodeId(2)], 2).unwrap();

        assert_eq!(
            generated,
            vec![
                GeneratedNsFunction {
                    source_set: 0,
                    combination: 0,
                    input_lines: vec![NodeId(0), NodeId(1)],
                },
                GeneratedNsFunction {
                    source_set: 0,
                    combination: 1,
                    input_lines: vec![NodeId(0), NodeId(1)],
                },
                GeneratedNsFunction {
                    source_set: 0,
                    combination: 2,
                    input_lines: vec![NodeId(0), NodeId(1)],
                },
                GeneratedNsFunction {
                    source_set: 1,
                    combination: 0,
                    input_lines: vec![NodeId(2)],
                },
            ]
        );
    }

    #[test]
    fn initial_probability_layout_matches_c_all_and_reduced_vectors() {
        let state = PsIterationState::new(4, 3, 2).unwrap();

        assert_eq!(
            state.all_probabilities,
            vec![0.25, 0.25, 0.25, 0.25, 0.5, 0.5, 0.0, 0.0]
        );
        assert_eq!(state.line_probabilities, vec![0.25, 0.25, 0.25, 0.5]);
    }

    #[test]
    fn cofactor_probability_for_one_ps_line_matches_c_vector_shape() {
        let mut arena = BddArena::default();
        let root = arena.add_branch(0, BddArena::zero(), BddArena::one());

        let result = calc_func_cofact_prob(&arena, root, &[0.25], 1, 1, 1, &[pi(0.0)]).unwrap();

        assert_eq!(result, vec![0.0, 1.0, 0.25]);
    }

    #[test]
    fn cofactor_probability_multiplies_plain_pi_probabilities_after_ps_lines() {
        let mut arena = BddArena::default();
        let pi_node = arena.add_branch(1, BddArena::zero(), BddArena::one());
        let root = arena.add_branch(0, BddArena::zero(), pi_node);

        let result =
            calc_func_cofact_prob(&arena, root, &[0.5], 1, 1, 1, &[pi(0.0), pi(0.75)]).unwrap();

        assert_eq!(result, vec![0.0, 0.75, 0.375]);
    }

    #[test]
    fn newton_update_solves_linear_system_and_rebuilds_redundant_set_probability() {
        let mut all = vec![0.25, 0.25, 0.25, 0.25];
        let mut ps = vec![0.25, 0.25, 0.25];
        let mut y = vec![0.8, 0.1, 0.1];
        let mut j = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

        calculate_next_iteration(&mut all, &mut ps, &mut y, &mut j, 2).unwrap();

        assert_eq!(ps, vec![-0.55, 0.15, 0.15]);
        assert_eq!(all, vec![-0.55, 0.15, 0.15, 1.25]);
    }

    #[test]
    fn approximation_converges_for_constant_next_state_line() {
        let arena = BddArena::default();

        let report = approximate_ps_lines(
            &arena,
            &[BddArena::one()],
            1,
            &[pi(0.0)],
            PsApproximationConfig {
                set_size: 1,
                delta: DEFAULT_PS_MAX_ALLOWED_ERROR,
                max_iterations: 4,
            },
        )
        .unwrap();

        assert!(report.converged);
        assert_eq!(report.probabilities, vec![1.0]);
    }

    #[test]
    fn sis_bound_operation_reports_dependency_beads_and_sources() {
        let error = power_direct_ps_lines_prob_from_sis(&(), &()).unwrap_err();

        let PsApprError::MissingDependency {
            operation,
            dependencies,
        } = error
        else {
            panic!("expected missing dependency error");
        };
        assert_eq!(operation, "power_direct_PS_lines_prob");
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.330"
                && dependency.source_file == "LogicSynthesis/sis/ntbdd/node_to_bdd.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.407"
                && dependency.source_file == "LogicSynthesis/sis/power/power_util.c"
        }));
        assert!(dependencies.iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.400"
                && dependency.source_file == "LogicSynthesis/sis/power/power_main.c"
        }));
        assert!(
            error
                .to_string()
                .contains("unported SIS C-file dependencies")
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("power_ps_appr.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
