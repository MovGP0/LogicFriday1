//! Native Rust utilities corresponding to `sis/atpg/atpg_seq_util.c`.
//!
//! The original C file is mostly small glue around SIS arrays, BDD cubes,
//! latch ordering tables, and mutable BDD set updates. This module keeps the
//! deterministic state/key behavior in Rust data structures and models the BDD
//! operations as trait-backed values so callers can connect the concrete BDD
//! implementation when that port is available.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PiOrderingEntry
{
    pub node_id: usize,
    pub name: String,
    pub index: usize,
}

impl PiOrderingEntry
{
    pub fn new(node_id: usize, name: impl Into<String>, index: usize) -> Self
    {
        Self {
            node_id,
            name: name.into(),
            index,
        }
    }
}

pub trait PrimaryOutputCount
{
    fn primary_output_count(&self) -> usize;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddLiteral
{
    Zero,
    One,
    DontCare,
}

impl BddLiteral
{
    pub fn from_sis_value(value: i32) -> Result<Self, AtpgSeqUtilError>
    {
        match value
        {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::DontCare),
            other => Err(AtpgSeqUtilError::InvalidLiteral(other)),
        }
    }

    pub fn bit_value(self) -> Option<u8>
    {
        match self
        {
            Self::Zero => Some(0),
            Self::One => Some(1),
            Self::DontCare => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddCube
{
    literals: Vec<BddLiteral>,
}

impl BddCube
{
    pub fn new(literals: impl Into<Vec<BddLiteral>>) -> Self
    {
        Self {
            literals: literals.into(),
        }
    }

    pub fn from_sis_values(values: &[i32]) -> Result<Self, AtpgSeqUtilError>
    {
        values
            .iter()
            .copied()
            .map(BddLiteral::from_sis_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Self::new)
    }

    pub fn literals(&self) -> &[BddLiteral]
    {
        &self.literals
    }

    pub fn literal(&self, index: usize) -> Result<BddLiteral, AtpgSeqUtilError>
    {
        self.literals
            .get(index)
            .copied()
            .ok_or(AtpgSeqUtilError::MissingCubeLiteral {
                index,
                len: self.literals.len(),
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtpgSeqUtilError
{
    InvalidLiteral(i32),
    InvalidStateValue(i32),
    MissingCubeLiteral {
        index: usize,
        len: usize,
    },
    LatchOrderingLengthMismatch {
        latch_count: usize,
        product_ordering: usize,
        pi_ordering: usize,
    },
    StateLengthMismatch {
        good_state: usize,
        faulty_state: usize,
    },
    ShiftOutOfRange {
        shift: usize,
    },
    MissingVariable {
        variable: usize,
    },
    DuplicatePiOrderingIndex {
        index: usize,
    },
    MissingPiOrderingIndex {
        index: usize,
    },
    PrimaryOutputCountMismatch {
        actual: usize,
    },
    UnexpectedLiteral {
        variable: usize,
        literal: BddLiteral,
    },
}

impl fmt::Display for AtpgSeqUtilError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::InvalidLiteral(value) => {
                write!(formatter, "BDD literal {value} is not in the SIS set {{0,1,2}}")
            }
            Self::InvalidStateValue(value) => {
                write!(formatter, "state value {value} is not in the SIS set {{0,1,2}}")
            }
            Self::MissingCubeLiteral {
                index,
                len,
            } => {
                write!(formatter, "cube literal index {index} is outside cube length {len}")
            }
            Self::LatchOrderingLengthMismatch {
                latch_count,
                product_ordering,
                pi_ordering,
            } => write!(
                formatter,
                "latch count {latch_count} does not match ordering lengths {product_ordering} and {pi_ordering}"
            ),
            Self::StateLengthMismatch {
                good_state,
                faulty_state,
            } => {
                write!(formatter, "good state length {good_state} does not match faulty state length {faulty_state}")
            }
            Self::ShiftOutOfRange {
                shift,
            } => write!(formatter, "bit shift {shift} is outside the native key width"),
            Self::MissingVariable {
                variable,
            } => write!(formatter, "variable {variable} is not present in the requested set"),
            Self::DuplicatePiOrderingIndex {
                index,
            } => write!(formatter, "PI ordering contains duplicate index {index}"),
            Self::MissingPiOrderingIndex {
                index,
            } => write!(formatter, "PI ordering is missing index {index}"),
            Self::PrimaryOutputCountMismatch {
                actual,
            } => write!(formatter, "converted BDD network has {actual} primary outputs instead of 1"),
            Self::UnexpectedLiteral {
                variable,
                literal,
            } => write!(formatter, "variable {variable} has unexpected literal {literal:?}"),
        }
    }
}

impl Error for AtpgSeqUtilError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductKeyOrdering
{
    pub latch_count: usize,
    pub latch_to_product_pi_ordering: Vec<usize>,
    pub latch_to_pi_ordering: Vec<usize>,
}

impl ProductKeyOrdering
{
    pub fn new(
        latch_count: usize,
        latch_to_product_pi_ordering: impl Into<Vec<usize>>,
        latch_to_pi_ordering: impl Into<Vec<usize>>,
    ) -> Result<Self, AtpgSeqUtilError>
    {
        let latch_to_product_pi_ordering = latch_to_product_pi_ordering.into();
        let latch_to_pi_ordering = latch_to_pi_ordering.into();

        if latch_to_product_pi_ordering.len() != latch_count || latch_to_pi_ordering.len() != latch_count
        {
            return Err(AtpgSeqUtilError::LatchOrderingLengthMismatch {
                latch_count,
                product_ordering: latch_to_product_pi_ordering.len(),
                pi_ordering: latch_to_pi_ordering.len(),
            });
        }

        Ok(Self {
            latch_count,
            latch_to_product_pi_ordering,
            latch_to_pi_ordering,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateLiteral
{
    Zero(usize),
    One(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateConstraint
{
    literals: Vec<StateLiteral>,
}

impl StateConstraint
{
    pub fn one() -> Self
    {
        Self {
            literals: Vec::new(),
        }
    }

    pub fn literals(&self) -> &[StateLiteral]
    {
        &self.literals
    }

    pub fn is_one(&self) -> bool
    {
        self.literals.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductStateConstraint
{
    pub good_state_len: usize,
    pub constraint: StateConstraint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OneMinterm
{
    assignments: BTreeSet<(usize, bool)>,
}

impl OneMinterm
{
    pub fn assignments(&self) -> &BTreeSet<(usize, bool)>
    {
        &self.assignments
    }

    pub fn contains(&self, variable: usize, value: bool) -> bool
    {
        self.assignments.contains(&(variable, value))
    }
}

pub trait BddSet: Clone
{
    fn not(&self) -> Self;

    fn or(&self, other: &Self) -> Self;
}

pub fn free_bdds_in_array<Bdd>(bdd_array: Vec<Bdd>)
{
    drop(bdd_array);
}

pub fn ordered_variable_names(
    pi_ordering: &[PiOrderingEntry],
) -> Result<Vec<String>, AtpgSeqUtilError>
{
    let mut names = vec![None; pi_ordering.len()];

    for entry in pi_ordering
    {
        let slot = names
            .get_mut(entry.index)
            .ok_or(AtpgSeqUtilError::MissingPiOrderingIndex {
                index: entry.index,
            })?;
        if slot.is_some()
        {
            return Err(AtpgSeqUtilError::DuplicatePiOrderingIndex {
                index: entry.index,
            });
        }
        *slot = Some(entry.name.clone());
    }

    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            name.ok_or(AtpgSeqUtilError::MissingPiOrderingIndex {
                index,
            })
        })
        .collect()
}

pub fn convert_bdd_to_network<Bdd, Network, Build, Eliminate>(
    function: &Bdd,
    pi_ordering: &[PiOrderingEntry],
    build_network: Build,
    eliminate: Eliminate,
) -> Result<Network, AtpgSeqUtilError>
where
    Network: PrimaryOutputCount,
    Build: FnOnce(&Bdd, &str, &[String]) -> Network,
    Eliminate: FnOnce(&mut Network),
{
    let variable_names = ordered_variable_names(pi_ordering)?;
    let mut result = build_network(function, "bdd_out", &variable_names);
    eliminate(&mut result);

    let actual = result.primary_output_count();
    if actual != 1
    {
        return Err(AtpgSeqUtilError::PrimaryOutputCountMismatch {
            actual,
        });
    }

    Ok(result)
}

pub fn get_pi_to_var_table<Var, Resolve>(
    pi_ordering: &[PiOrderingEntry],
    mut resolve: Resolve,
) -> HashMap<usize, Var>
where
    Resolve: FnMut(&PiOrderingEntry) -> Var,
{
    pi_ordering
        .iter()
        .map(|entry| (entry.node_id, resolve(entry)))
        .collect()
}

pub fn convert_bdd_to_int(first_cube: &BddCube) -> Result<u64, AtpgSeqUtilError>
{
    let mut state_int = 0_u64;

    for (index, literal) in first_cube.literals().iter().copied().enumerate()
    {
        if literal == BddLiteral::One
        {
            state_int |= checked_bit(index)?;
        }
    }

    Ok(state_int)
}

pub fn convert_product_bdd_to_key(
    first_cube: &BddCube,
    ordering: &ProductKeyOrdering,
) -> Result<u64, AtpgSeqUtilError>
{
    let mut state_int = 0_u64;

    for latch_index in 0..ordering.latch_count
    {
        let position_in_cube = ordering.latch_to_product_pi_ordering[latch_index];
        let current_lit = first_cube.literal(position_in_cube)?;
        let position_in_key = ordering.latch_to_pi_ordering[latch_index];

        if current_lit == BddLiteral::One
        {
            state_int |= checked_bit(position_in_key)?;
        }
    }

    Ok(state_int)
}

pub fn bdd_add_varids_to_table(vars: &[usize], table: &mut HashSet<usize>)
{
    table.extend(vars.iter().copied());
}

pub fn seq_get_one_minterm(
    first_cube: &BddCube,
    vars: &HashSet<usize>,
) -> Result<OneMinterm, AtpgSeqUtilError>
{
    let mut assignments = BTreeSet::new();

    for (variable, literal) in first_cube.literals().iter().copied().enumerate()
    {
        if !vars.contains(&variable)
        {
            if literal != BddLiteral::DontCare
            {
                return Err(AtpgSeqUtilError::UnexpectedLiteral {
                    variable,
                    literal,
                });
            }
            continue;
        }

        let value = match literal
        {
            BddLiteral::Zero | BddLiteral::DontCare => false,
            BddLiteral::One => true,
        };
        assignments.insert((variable, value));
    }

    for variable in vars
    {
        if *variable >= first_cube.literals().len()
        {
            return Err(AtpgSeqUtilError::MissingVariable {
                variable: *variable,
            });
        }
    }

    Ok(OneMinterm {
        assignments,
    })
}

pub fn find_good_constraint<Bdd, CofactorFn>(
    current_set: &Bdd,
    total_set: &Bdd,
    cofactor_fn: CofactorFn,
) -> Bdd
where
    Bdd: BddSet,
    CofactorFn: FnOnce(&Bdd, &Bdd) -> Bdd,
{
    let care_set = total_set.not();
    cofactor_fn(current_set, &care_set)
}

pub fn use_cofactored_set<Bdd, CofactorFn>(
    current_set: &mut Bdd,
    total_set: &mut Bdd,
    cofactor_fn: CofactorFn,
)
where
    Bdd: BddSet,
    CofactorFn: FnOnce(&Bdd, &Bdd) -> Bdd,
{
    let new_current_set = find_good_constraint(current_set, total_set, cofactor_fn);
    let new_total_set = current_set.or(total_set);
    *total_set = new_total_set;
    *current_set = new_current_set;
}

pub fn convert_state_to_bdd(state: &[i32]) -> Result<StateConstraint, AtpgSeqUtilError>
{
    state_constraint_from_values(state.iter().copied().enumerate())
}

pub fn convert_states_to_product_bdd(
    good_state: &[i32],
    faulty_state: &[i32],
) -> Result<ProductStateConstraint, AtpgSeqUtilError>
{
    let mut values = Vec::with_capacity(good_state.len() + faulty_state.len());
    values.extend_from_slice(good_state);
    values.extend_from_slice(faulty_state);

    Ok(ProductStateConstraint {
        good_state_len: good_state.len(),
        constraint: convert_state_to_bdd(&values)?,
    })
}

pub fn derive_prop_key(good_state: &[i32], faulty_state: &[i32]) -> Result<u64, AtpgSeqUtilError>
{
    let state_length = checked_equal_state_lengths(good_state, faulty_state)?;
    let good_state_int = state_values_to_int(good_state)?;
    let faulty_state_int = state_values_to_int(faulty_state)?;

    Ok((good_state_int << state_length) + faulty_state_int)
}

pub fn derive_inverted_prop_key(
    good_state: &[i32],
    faulty_state: &[i32],
) -> Result<u64, AtpgSeqUtilError>
{
    let state_length = checked_equal_state_lengths(good_state, faulty_state)?;
    let good_state_int = state_values_to_int(good_state)?;
    let faulty_state_int = state_values_to_int(faulty_state)?;

    Ok((faulty_state_int << state_length) + good_state_int)
}

fn state_constraint_from_values(
    values: impl IntoIterator<Item = (usize, i32)>,
) -> Result<StateConstraint, AtpgSeqUtilError>
{
    let mut literals = Vec::new();

    for (index, value) in values
    {
        match value
        {
            0 => literals.push(StateLiteral::Zero(index)),
            1 => literals.push(StateLiteral::One(index)),
            2 => {}
            other => return Err(AtpgSeqUtilError::InvalidStateValue(other)),
        }
    }

    Ok(StateConstraint {
        literals,
    })
}

fn state_values_to_int(state: &[i32]) -> Result<u64, AtpgSeqUtilError>
{
    let mut result = 0_u64;

    for (index, value) in state.iter().copied().enumerate()
    {
        match value
        {
            0 => {}
            1 => result |= checked_bit(index)?,
            2 => {}
            other => return Err(AtpgSeqUtilError::InvalidStateValue(other)),
        }
    }

    Ok(result)
}

fn checked_equal_state_lengths(
    good_state: &[i32],
    faulty_state: &[i32],
) -> Result<usize, AtpgSeqUtilError>
{
    if good_state.len() != faulty_state.len()
    {
        return Err(AtpgSeqUtilError::StateLengthMismatch {
            good_state: good_state.len(),
            faulty_state: faulty_state.len(),
        });
    }

    if good_state.len() >= u64::BITS as usize
    {
        return Err(AtpgSeqUtilError::ShiftOutOfRange {
            shift: good_state.len(),
        });
    }

    Ok(good_state.len())
}

fn checked_bit(shift: usize) -> Result<u64, AtpgSeqUtilError>
{
    1_u64
        .checked_shl(shift as u32)
        .ok_or(AtpgSeqUtilError::ShiftOutOfRange {
            shift,
        })
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestBddSet
    {
        states: BTreeSet<u8>,
        universe: BTreeSet<u8>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNetwork
    {
        output_name: String,
        variable_names: Vec<String>,
        primary_outputs: usize,
        eliminated: bool,
    }

    impl PrimaryOutputCount for TestNetwork
    {
        fn primary_output_count(&self) -> usize
        {
            self.primary_outputs
        }
    }

    impl TestBddSet
    {
        fn new(states: &[u8]) -> Self
        {
            Self {
                states: states.iter().copied().collect(),
                universe: (0..=3).collect(),
            }
        }
    }

    impl BddSet for TestBddSet
    {
        fn not(&self) -> Self
        {
            Self {
                states: self.universe.difference(&self.states).copied().collect(),
                universe: self.universe.clone(),
            }
        }

        fn or(&self, other: &Self) -> Self
        {
            Self {
                states: self.states.union(&other.states).copied().collect(),
                universe: self.universe.clone(),
            }
        }
    }

    #[test]
    fn convert_bdd_to_int_treats_dont_cares_as_zero()
    {
        let cube = BddCube::from_sis_values(&[1, 2, 1, 0, 1]).unwrap();

        assert_eq!(convert_bdd_to_int(&cube), Ok(0b10101));
    }

    #[test]
    fn ordered_variable_names_follow_pi_ordering_indices()
    {
        let pi_ordering = [
            PiOrderingEntry::new(7, "c", 2),
            PiOrderingEntry::new(4, "a", 0),
            PiOrderingEntry::new(5, "b", 1),
        ];

        assert_eq!(
            ordered_variable_names(&pi_ordering).unwrap(),
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
        );
    }

    #[test]
    fn convert_bdd_to_network_builds_eliminates_and_checks_single_output()
    {
        let pi_ordering = [
            PiOrderingEntry::new(1, "x", 1),
            PiOrderingEntry::new(0, "w", 0),
        ];

        let network = convert_bdd_to_network(
            &42,
            &pi_ordering,
            |function, output_name, variable_names| TestNetwork {
                output_name: format!("{output_name}_{function}"),
                variable_names: variable_names.to_vec(),
                primary_outputs: 1,
                eliminated: false,
            },
            |network| {
                network.eliminated = true;
            },
        )
        .unwrap();

        assert_eq!(
            network,
            TestNetwork {
                output_name: "bdd_out_42".to_owned(),
                variable_names: vec!["w".to_owned(), "x".to_owned()],
                primary_outputs: 1,
                eliminated: true,
            }
        );
    }

    #[test]
    fn get_pi_to_var_table_maps_node_ids_to_resolved_variables()
    {
        let pi_ordering = [
            PiOrderingEntry::new(11, "a", 0),
            PiOrderingEntry::new(12, "b", 1),
        ];

        assert_eq!(
            get_pi_to_var_table(&pi_ordering, |entry| entry.index + 20),
            HashMap::from([(11, 20), (12, 21)])
        );
    }

    #[test]
    fn convert_product_bdd_to_key_uses_latch_ordering_tables()
    {
        let cube = BddCube::from_sis_values(&[0, 1, 2, 1]).unwrap();
        let ordering = ProductKeyOrdering::new(3, [1, 3, 2], [0, 2, 1]).unwrap();

        assert_eq!(convert_product_bdd_to_key(&cube, &ordering), Ok(0b101));
    }

    #[test]
    fn seq_get_one_minterm_assigns_selected_dont_cares_to_zero()
    {
        let cube = BddCube::from_sis_values(&[1, 2, 2]).unwrap();
        let vars = HashSet::from([0, 1]);

        let minterm = seq_get_one_minterm(&cube, &vars).unwrap();

        assert!(minterm.contains(0, true));
        assert!(minterm.contains(1, false));
        assert!(!minterm.contains(2, false));
    }

    #[test]
    fn seq_get_one_minterm_rejects_non_selected_care_literals()
    {
        let cube = BddCube::from_sis_values(&[2, 1]).unwrap();
        let vars = HashSet::from([0]);

        assert_eq!(
            seq_get_one_minterm(&cube, &vars),
            Err(AtpgSeqUtilError::UnexpectedLiteral {
                variable: 1,
                literal: BddLiteral::One,
            })
        );
    }

    #[test]
    fn cofactored_set_updates_current_and_total_sets_in_c_order()
    {
        let mut current_set = TestBddSet::new(&[1, 2]);
        let mut total_set = TestBddSet::new(&[2, 3]);

        use_cofactored_set(&mut current_set, &mut total_set, |current, care| TestBddSet {
            states: current.states.intersection(&care.states).copied().collect(),
            universe: current.universe.clone(),
        });

        assert_eq!(current_set.states, BTreeSet::from([1]));
        assert_eq!(total_set.states, BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn bdd_add_varids_to_table_inserts_every_variable_id()
    {
        let mut table = HashSet::from([4]);

        bdd_add_varids_to_table(&[2, 4, 7], &mut table);

        assert_eq!(table, HashSet::from([2, 4, 7]));
    }

    #[test]
    fn convert_state_to_bdd_builds_latch_literal_constraint()
    {
        assert_eq!(
            convert_state_to_bdd(&[1, 2, 0]).unwrap().literals(),
            &[StateLiteral::One(0), StateLiteral::Zero(2)]
        );
    }

    #[test]
    fn convert_states_to_product_bdd_concatenates_good_and_faulty_states()
    {
        let product = convert_states_to_product_bdd(&[1, 2], &[0, 1]).unwrap();

        assert_eq!(product.good_state_len, 2);
        assert_eq!(
            product.constraint.literals(),
            &[
                StateLiteral::One(0),
                StateLiteral::Zero(2),
                StateLiteral::One(3),
            ]
        );
    }

    #[test]
    fn derive_prop_key_places_good_state_above_faulty_state()
    {
        assert_eq!(derive_prop_key(&[1, 0, 1], &[0, 1, 1]), Ok(0b101110));
        assert_eq!(
            derive_inverted_prop_key(&[1, 0, 1], &[0, 1, 1]),
            Ok(0b110101)
        );
    }

    #[test]
    fn derive_prop_key_validates_state_lengths_and_values()
    {
        assert_eq!(
            derive_prop_key(&[1], &[1, 0]),
            Err(AtpgSeqUtilError::StateLengthMismatch {
                good_state: 1,
                faulty_state: 2,
            })
        );
        assert_eq!(
            derive_prop_key(&[3], &[0]),
            Err(AtpgSeqUtilError::InvalidStateValue(3))
        );
    }

    #[test]
    fn source_has_no_legacy_tokens()
    {
        let source = include_str!("atpg_seq_util.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("source", "_", "file")));
    }
}
