//! Native Rust model for `LogicSynthesis/sis/power/power_comp.c`.
//!
//! The C file scans a BDD to compute the probability that a function is one.
//! SIS builds the BDD elsewhere and supplies CMU BDD pointers, `st_table`
//! caches, state-probability tables, and Espresso `pset` values. This port
//! keeps the scanner behavior over an owned Rust BDD and reports explicit
//! dependency beads for SIS-bound adapters.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortDependency {
    pub bead_id: &'static str,
    pub source_file: &'static str,
    pub reason: &'static str,
}

pub const REQUIRED_PORT_DEPENDENCIES: &[PortDependency] = &[
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.329",
        source_file: "LogicSynthesis/sis/ntbdd/manager.c",
        reason: "native SIS BDD manager and CMU BDD node identity/index access",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.330",
        source_file: "LogicSynthesis/sis/ntbdd/node_to_bdd.c",
        reason: "conversion from SIS network nodes to BDD roots consumed by power_comp.c",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.403",
        source_file: "LogicSynthesis/sis/power/power_psExact.c",
        reason: "state probability table and present-state line encoding for exact mode",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.402",
        source_file: "LogicSynthesis/sis/power/power_psAppr.c",
        reason: "correlated present-state line set probabilities for approximate mode",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.407",
        source_file: "LogicSynthesis/sis/power/power_util.c",
        reason: "power_lines_in_set helper used by the grouped present-state probability scan",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.485",
        source_file: "LogicSynthesis/sis/st/st.c",
        reason: "legacy pointer-keyed visited and state-index tables are st_table instances",
    },
    PortDependency {
        bead_id: "LogicFriday1-8j8.2.6.165",
        source_file: "LogicSynthesis/sis/espresso/set.c",
        reason: "legacy grouped present-state scan represents line constraints with Espresso pset",
    },
];

pub fn required_port_dependencies() -> &'static [PortDependency] {
    REQUIRED_PORT_DEPENDENCIES
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BddNodeId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub enum BddNode {
    Zero,
    One,
    Branch {
        variable: usize,
        else_child: BddNodeId,
        then_child: BddNodeId,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProbabilityBdd {
    nodes: Vec<BddNode>,
    root: Option<BddNodeId>,
}

impl ProbabilityBdd {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_constants() -> Self {
        let mut bdd = Self::new();
        bdd.add_node(BddNode::Zero);
        bdd.add_node(BddNode::One);
        bdd
    }

    pub fn add_node(&mut self, node: BddNode) -> BddNodeId {
        let id = BddNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn set_root(&mut self, root: BddNodeId) -> Result<(), PowerCompError> {
        self.node(root)?;
        self.root = Some(root);
        Ok(())
    }

    pub fn root(&self) -> Result<BddNodeId, PowerCompError> {
        self.root.ok_or(PowerCompError::MissingRoot)
    }

    pub fn node(&self, id: BddNodeId) -> Result<&BddNode, PowerCompError> {
        self.nodes
            .get(id.0)
            .ok_or(PowerCompError::UnknownBddNode(id))
    }

    pub fn nodes(&self) -> &[BddNode] {
        &self.nodes
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerPiInfo {
    pub prob_one: f64,
    pub ps_line_index: Option<usize>,
}

impl PowerPiInfo {
    pub fn primary_input(prob_one: f64) -> Self {
        Self {
            prob_one,
            ps_line_index: None,
        }
    }

    pub fn present_state(prob_one: f64, ps_line_index: usize) -> Self {
        Self {
            prob_one,
            ps_line_index: Some(ps_line_index),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateProbabilityTable {
    probabilities: Vec<f64>,
    state_index: HashMap<String, usize>,
}

impl StateProbabilityTable {
    pub fn new(
        probabilities: Vec<f64>,
        state_index: impl IntoIterator<Item = (impl Into<String>, usize)>,
    ) -> Self {
        Self {
            probabilities,
            state_index: state_index
                .into_iter()
                .map(|(state, index)| (state.into(), index))
                .collect(),
        }
    }

    fn included_probability(&self, encoding: &[Option<bool>]) -> Result<f64, PowerCompError> {
        let mut sum = 0.0;
        for (state, index) in &self.state_index {
            if state.len() != encoding.len()
                || !state.bytes().all(|byte| byte == b'0' || byte == b'1')
            {
                return Err(PowerCompError::InvalidStateEncoding(state.clone()));
            }
            if state_matches_encoding(state, encoding) {
                let probability = self
                    .probabilities
                    .get(*index)
                    .copied()
                    .ok_or(PowerCompError::MissingStateProbability { index: *index })?;
                sum += probability;
            }
        }
        Ok(sum)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineSet {
    lines: Vec<bool>,
}

impl LineSet {
    pub fn full(line_count: usize) -> Self {
        Self {
            lines: vec![true; line_count],
        }
    }

    pub fn remove(&mut self, line: usize) -> Result<(), PowerCompError> {
        let Some(slot) = self.lines.get_mut(line) else {
            return Err(PowerCompError::LineSetIndexOutOfRange {
                line,
                len: self.lines.len(),
            });
        };
        *slot = false;
        Ok(())
    }

    pub fn contains(&self, line: usize) -> bool {
        self.lines.get(line).copied().unwrap_or(false)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PowerCompError {
    MissingRoot,
    UnknownBddNode(BddNodeId),
    MissingPiInfo {
        variable: usize,
    },
    MissingStateProbability {
        index: usize,
    },
    InvalidStateEncoding(String),
    InvalidPresentStateLine {
        variable: usize,
        ps_line_index: usize,
        state_line_count: usize,
    },
    InvalidPowerSetSize(usize),
    PresentStateProbabilityOutOfRange {
        index: usize,
        len: usize,
    },
    LineSetIndexOutOfRange {
        line: usize,
        len: usize,
    },
    MissingNativePorts {
        operation: &'static str,
        dependencies: &'static [PortDependency],
    },
}

impl fmt::Display for PowerCompError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRoot => write!(f, "probability BDD has no root node"),
            Self::UnknownBddNode(node) => write!(f, "unknown probability BDD node {:?}", node),
            Self::MissingPiInfo { variable } => {
                write!(
                    f,
                    "missing power_pi_t information for BDD variable {variable}"
                )
            }
            Self::MissingStateProbability { index } => {
                write!(f, "state probability index {index} is not present")
            }
            Self::InvalidStateEncoding(state) => {
                write!(
                    f,
                    "invalid state encoding {state:?}; expected only 0/1 bits"
                )
            }
            Self::InvalidPresentStateLine {
                variable,
                ps_line_index,
                state_line_count,
            } => write!(
                f,
                "BDD variable {variable} maps to present-state line {ps_line_index}, but only {state_line_count} state lines exist"
            ),
            Self::InvalidPowerSetSize(size) => {
                write!(f, "power_setSize must be at least 1, got {size}")
            }
            Self::PresentStateProbabilityOutOfRange { index, len } => write!(
                f,
                "present-state probability index {index} is outside psProb length {len}"
            ),
            Self::LineSetIndexOutOfRange { line, len } => {
                write!(f, "line-set index {line} is outside line-set length {len}")
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

impl Error for PowerCompError {}

pub fn power_calc_func_prob(
    bdd: &ProbabilityBdd,
    pi_info: &[PowerPiInfo],
) -> Result<f64, PowerCompError> {
    let mut visited = HashMap::new();
    count_prob(bdd, bdd.root()?, pi_info, &mut visited)
}

pub fn power_calc_func_prob_w_state_prob(
    bdd: &ProbabilityBdd,
    pi_info: &[PowerPiInfo],
    state_probabilities: &StateProbabilityTable,
    state_line_count: usize,
) -> Result<f64, PowerCompError> {
    let mut visited = HashMap::new();
    let mut encoding = vec![None; state_line_count];
    count_prob_w_state_prob(
        bdd,
        bdd.root()?,
        pi_info,
        state_probabilities,
        &mut encoding,
        &mut visited,
    )
}

pub fn power_calc_func_prob_w_sets(
    bdd: &ProbabilityBdd,
    pi_info: &[PowerPiInfo],
    present_state_probabilities: &[f64],
    present_state_line_count: usize,
    power_set_size: usize,
) -> Result<f64, PowerCompError> {
    if power_set_size == 0 {
        return Err(PowerCompError::InvalidPowerSetSize(power_set_size));
    }

    let mut base_cache = HashMap::new();
    let mut set_cache = HashMap::new();
    let sets = LineSet::full(2 * power_set_size);
    count_prob_w_sets(
        bdd,
        bdd.root()?,
        pi_info,
        present_state_probabilities,
        present_state_line_count,
        power_set_size,
        None,
        sets,
        &mut base_cache,
        &mut set_cache,
    )
}

pub fn calculate_sis_function_probability<Bdd, PiInfo>(
    _bdd: &Bdd,
    _pi_info: &PiInfo,
) -> Result<f64, PowerCompError> {
    Err(PowerCompError::MissingNativePorts {
        operation: "power_calc_func_prob",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn calculate_sis_function_probability_with_state_prob<Bdd, PiInfo, StateProb, StateIndex>(
    _bdd: &Bdd,
    _pi_info: &PiInfo,
    _state_probabilities: &StateProb,
    _state_index: &StateIndex,
) -> Result<f64, PowerCompError> {
    Err(PowerCompError::MissingNativePorts {
        operation: "power_calc_func_prob_w_stateProb",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

pub fn calculate_sis_function_probability_with_sets<Bdd, PiInfo, PsProb>(
    _bdd: &Bdd,
    _pi_info: &PiInfo,
    _present_state_probabilities: &PsProb,
) -> Result<f64, PowerCompError> {
    Err(PowerCompError::MissingNativePorts {
        operation: "power_calc_func_prob_w_sets",
        dependencies: REQUIRED_PORT_DEPENDENCIES,
    })
}

fn count_prob(
    bdd: &ProbabilityBdd,
    node_id: BddNodeId,
    pi_info: &[PowerPiInfo],
    visited: &mut HashMap<BddNodeId, f64>,
) -> Result<f64, PowerCompError> {
    if let Some(result) = visited.get(&node_id) {
        return Ok(*result);
    }

    let result = match *bdd.node(node_id)? {
        BddNode::Zero => 0.0,
        BddNode::One => 1.0,
        BddNode::Branch {
            variable,
            else_child,
            then_child,
        } => {
            let prob_one = pi_info
                .get(variable)
                .ok_or(PowerCompError::MissingPiInfo { variable })?
                .prob_one;
            (1.0 - prob_one) * count_prob(bdd, else_child, pi_info, visited)?
                + prob_one * count_prob(bdd, then_child, pi_info, visited)?
        }
    };

    if matches!(bdd.node(node_id)?, BddNode::Branch { .. }) {
        visited.insert(node_id, result);
    }
    Ok(result)
}

fn count_prob_w_state_prob(
    bdd: &ProbabilityBdd,
    node_id: BddNodeId,
    pi_info: &[PowerPiInfo],
    state_probabilities: &StateProbabilityTable,
    encoding: &mut [Option<bool>],
    base_cache: &mut HashMap<BddNodeId, f64>,
) -> Result<f64, PowerCompError> {
    match *bdd.node(node_id)? {
        BddNode::Zero => Ok(0.0),
        BddNode::One => state_probabilities.included_probability(encoding),
        BddNode::Branch {
            variable,
            else_child,
            then_child,
        } => {
            let pi = *pi_info
                .get(variable)
                .ok_or(PowerCompError::MissingPiInfo { variable })?;
            let Some(ps_line_index) = pi.ps_line_index else {
                return Ok(state_probabilities.included_probability(encoding)?
                    * count_prob(bdd, node_id, pi_info, base_cache)?);
            };
            if ps_line_index >= encoding.len() {
                return Err(PowerCompError::InvalidPresentStateLine {
                    variable,
                    ps_line_index,
                    state_line_count: encoding.len(),
                });
            }

            encoding[ps_line_index] = Some(false);
            let else_result = count_prob_w_state_prob(
                bdd,
                else_child,
                pi_info,
                state_probabilities,
                encoding,
                base_cache,
            )?;

            encoding[ps_line_index] = Some(true);
            let then_result = count_prob_w_state_prob(
                bdd,
                then_child,
                pi_info,
                state_probabilities,
                encoding,
                base_cache,
            )?;
            encoding[ps_line_index] = None;

            Ok(else_result + then_result)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn count_prob_w_sets(
    bdd: &ProbabilityBdd,
    node_id: BddNodeId,
    pi_info: &[PowerPiInfo],
    present_state_probabilities: &[f64],
    present_state_line_count: usize,
    power_set_size: usize,
    previous_index: Option<usize>,
    sets: LineSet,
    base_cache: &mut HashMap<BddNodeId, f64>,
    set_cache: &mut HashMap<BddNodeId, f64>,
) -> Result<f64, PowerCompError> {
    match *bdd.node(node_id)? {
        BddNode::Zero => Ok(0.0),
        BddNode::One => multiply_present_state_probability(
            1.0,
            previous_index,
            present_state_probabilities,
            power_set_size,
            &sets,
        ),
        BddNode::Branch {
            variable,
            else_child,
            then_child,
        } => {
            if let Some(stored) = set_cache.get(&node_id) {
                return multiply_present_state_probability(
                    *stored,
                    previous_index,
                    present_state_probabilities,
                    power_set_size,
                    &sets,
                );
            }

            if variable >= present_state_line_count {
                return multiply_present_state_probability(
                    count_prob(bdd, node_id, pi_info, base_cache)?,
                    previous_index,
                    present_state_probabilities,
                    power_set_size,
                    &sets,
                );
            }

            let new_set = previous_index
                .map(|previous| variable / power_set_size != previous / power_set_size)
                .unwrap_or(false);
            let active_sets = if new_set {
                LineSet::full(2 * power_set_size)
            } else {
                sets.clone()
            };
            let previous_sets = sets;

            let mut else_sets = active_sets.clone();
            else_sets.remove(2 * (variable % power_set_size) + 1)?;
            let else_result = count_prob_w_sets(
                bdd,
                else_child,
                pi_info,
                present_state_probabilities,
                present_state_line_count,
                power_set_size,
                Some(variable),
                else_sets,
                base_cache,
                set_cache,
            )?;

            let mut then_sets = active_sets;
            then_sets.remove(2 * (variable % power_set_size))?;
            let then_result = count_prob_w_sets(
                bdd,
                then_child,
                pi_info,
                present_state_probabilities,
                present_state_line_count,
                power_set_size,
                Some(variable),
                then_sets,
                base_cache,
                set_cache,
            )?;

            let mut result = else_result + then_result;
            if variable % power_set_size == 0 {
                set_cache.insert(node_id, result);
            }
            if new_set {
                result = multiply_present_state_probability(
                    result,
                    previous_index,
                    present_state_probabilities,
                    power_set_size,
                    &previous_sets,
                )?;
            }
            Ok(result)
        }
    }
}

fn multiply_present_state_probability(
    result: f64,
    index: Option<usize>,
    present_state_probabilities: &[f64],
    power_set_size: usize,
    sets: &LineSet,
) -> Result<f64, PowerCompError> {
    let Some(index) = index else {
        return Ok(result);
    };

    let set_number = index / power_set_size;
    let set_size = 1usize
        .checked_shl(power_set_size as u32)
        .ok_or(PowerCompError::InvalidPowerSetSize(power_set_size))?;
    let mut probability_sum = 0.0;
    for combination in 0..set_size {
        if combination_is_in_set(combination, power_set_size, sets) {
            let probability_index = set_number * set_size + combination;
            let probability = present_state_probabilities
                .get(probability_index)
                .copied()
                .ok_or(PowerCompError::PresentStateProbabilityOutOfRange {
                    index: probability_index,
                    len: present_state_probabilities.len(),
                })?;
            probability_sum += probability;
        }
    }

    Ok(result * probability_sum)
}

fn combination_is_in_set(combination: usize, power_set_size: usize, sets: &LineSet) -> bool {
    (0..power_set_size).all(|bit_index| {
        let required_line = if combination & (1usize << bit_index) == 0 {
            2 * bit_index
        } else {
            2 * bit_index + 1
        };
        sets.contains(required_line)
    })
}

fn state_matches_encoding(state: &str, encoding: &[Option<bool>]) -> bool {
    state
        .bytes()
        .zip(encoding)
        .all(|(bit, expected)| match expected {
            Some(false) => bit == b'0',
            Some(true) => bit == b'1',
            None => true,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branch(
        bdd: &mut ProbabilityBdd,
        variable: usize,
        else_child: BddNodeId,
        then_child: BddNodeId,
    ) -> BddNodeId {
        bdd.add_node(BddNode::Branch {
            variable,
            else_child,
            then_child,
        })
    }

    fn approx_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "actual {actual} expected {expected}"
        );
    }

    #[test]
    fn plain_function_probability_scans_else_and_then_paths() {
        let mut bdd = ProbabilityBdd::with_constants();
        let zero = BddNodeId(0);
        let one = BddNodeId(1);
        let b = branch(&mut bdd, 1, zero, one);
        let root = branch(&mut bdd, 0, zero, b);
        bdd.set_root(root).unwrap();

        let result = power_calc_func_prob(
            &bdd,
            &[
                PowerPiInfo::primary_input(0.25),
                PowerPiInfo::primary_input(0.5),
            ],
        )
        .unwrap();

        approx_eq(result, 0.125);
    }

    #[test]
    fn shared_subgraph_is_counted_once_but_semantics_match_probability_sum() {
        let mut bdd = ProbabilityBdd::with_constants();
        let zero = BddNodeId(0);
        let one = BddNodeId(1);
        let shared = branch(&mut bdd, 1, zero, one);
        let root = branch(&mut bdd, 0, shared, shared);
        bdd.set_root(root).unwrap();

        let result = power_calc_func_prob(
            &bdd,
            &[
                PowerPiInfo::primary_input(0.1),
                PowerPiInfo::primary_input(0.7),
            ],
        )
        .unwrap();

        approx_eq(result, 0.7);
    }

    #[test]
    fn state_probability_scan_sums_matching_present_state_encodings() {
        let mut bdd = ProbabilityBdd::with_constants();
        let zero = BddNodeId(0);
        let one = BddNodeId(1);
        let pi = branch(&mut bdd, 2, zero, one);
        let ps1 = branch(&mut bdd, 1, zero, pi);
        let root = branch(&mut bdd, 0, ps1, pi);
        bdd.set_root(root).unwrap();

        let state_probabilities = StateProbabilityTable::new(
            vec![0.1, 0.2, 0.3, 0.4],
            [("00", 0), ("01", 1), ("10", 2), ("11", 3)],
        );
        let result = power_calc_func_prob_w_state_prob(
            &bdd,
            &[
                PowerPiInfo::present_state(0.5, 0),
                PowerPiInfo::present_state(0.5, 1),
                PowerPiInfo::primary_input(0.5),
            ],
            &state_probabilities,
            2,
        )
        .unwrap();

        approx_eq(result, 0.45);
    }

    #[test]
    fn grouped_present_state_scan_multiplies_completed_set_probabilities() {
        let mut bdd = ProbabilityBdd::with_constants();
        let zero = BddNodeId(0);
        let one = BddNodeId(1);
        let pi = branch(&mut bdd, 2, zero, one);
        let ps1 = branch(&mut bdd, 1, zero, pi);
        let root = branch(&mut bdd, 0, ps1, pi);
        bdd.set_root(root).unwrap();

        let result = power_calc_func_prob_w_sets(
            &bdd,
            &[
                PowerPiInfo::present_state(0.5, 0),
                PowerPiInfo::present_state(0.5, 1),
                PowerPiInfo::primary_input(0.5),
            ],
            &[0.1, 0.2, 0.3, 0.4],
            2,
            2,
        )
        .unwrap();

        approx_eq(result, 0.45);
    }

    #[test]
    fn grouped_scan_handles_multiple_present_state_sets() {
        let mut bdd = ProbabilityBdd::with_constants();
        let zero = BddNodeId(0);
        let one = BddNodeId(1);
        let ps2 = branch(&mut bdd, 2, zero, one);
        let ps0 = branch(&mut bdd, 0, zero, ps2);
        bdd.set_root(ps0).unwrap();

        let result = power_calc_func_prob_w_sets(
            &bdd,
            &[
                PowerPiInfo::present_state(0.5, 0),
                PowerPiInfo::present_state(0.5, 1),
                PowerPiInfo::present_state(0.5, 2),
            ],
            &[0.25, 0.75, 0.5, 0.5, 0.2, 0.8],
            3,
            1,
        )
        .unwrap();

        approx_eq(result, 0.75 * 0.8);
    }

    #[test]
    fn sis_bound_entries_report_dependency_beads_and_sources() {
        let error = calculate_sis_function_probability(&(), &()).unwrap_err();

        assert_eq!(
            error,
            PowerCompError::MissingNativePorts {
                operation: "power_calc_func_prob",
                dependencies: REQUIRED_PORT_DEPENDENCIES,
            }
        );
        assert!(
            error
                .to_string()
                .contains("unported SIS C-file dependencies")
        );
        assert!(required_port_dependencies().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.329"
                && dependency.source_file == "LogicSynthesis/sis/ntbdd/manager.c"
        }));
        assert!(required_port_dependencies().iter().any(|dependency| {
            dependency.bead_id == "LogicFriday1-8j8.2.6.165"
                && dependency.source_file == "LogicSynthesis/sis/espresso/set.c"
        }));
    }

    #[test]
    fn no_legacy_c_abi_tokens_are_present_in_this_port() {
        let source = include_str!("power_comp.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
    }
}
