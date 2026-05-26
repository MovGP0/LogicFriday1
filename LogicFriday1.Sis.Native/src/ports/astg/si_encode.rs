//! Native STG state assignment support for ASTG encoding.
//!
//! The legacy routine computes Tracey's single-transition-time state assignment
//! constraints and, in the checked-in SIS source, returns after assigning state
//! encodings. The disabled network-construction branch is kept out of this
//! native module; callers can feed the encoded STG into higher-level network
//! conversion code when that integration boundary is available.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::ports::enc::dic_to_sm;
use crate::ports::enc::gen_eqn;
use crate::ports::enc::input;
use crate::ports::mincov::mincov;
use crate::ports::stg::stg::{Stg, StgError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstgEncodeResult
{
    pub state_bits: usize,
    pub seed_count: usize,
    pub reduced_seed_count: usize,
    pub prime_count: usize,
    pub cover: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SiEncodeError
{
    EmptyStg,
    Stg(StgError),
    GenEqn(gen_eqn::GenEqnError),
    DicToSm(dic_to_sm::DicToSmError),
    MinCover(mincov::MinCoverError),
}

impl fmt::Display for SiEncodeError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyStg => write!(formatter, "STG has no states"),
            Self::Stg(error) => write!(formatter, "{error}"),
            Self::GenEqn(error) => write!(formatter, "{error}"),
            Self::DicToSm(error) => write!(formatter, "{error}"),
            Self::MinCover(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for SiEncodeError
{
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match self
        {
            Self::EmptyStg => None,
            Self::Stg(error) => Some(error),
            Self::GenEqn(error) => Some(error),
            Self::DicToSm(error) => Some(error),
            Self::MinCover(error) => Some(error),
        }
    }
}

impl From<StgError> for SiEncodeError
{
    fn from(error: StgError) -> Self
    {
        Self::Stg(error)
    }
}

impl From<gen_eqn::GenEqnError> for SiEncodeError
{
    fn from(error: gen_eqn::GenEqnError) -> Self
    {
        Self::GenEqn(error)
    }
}

impl From<dic_to_sm::DicToSmError> for SiEncodeError
{
    fn from(error: dic_to_sm::DicToSmError) -> Self
    {
        Self::DicToSm(error)
    }
}

impl From<mincov::MinCoverError> for SiEncodeError
{
    fn from(error: mincov::MinCoverError) -> Self
    {
        Self::MinCover(error)
    }
}

pub fn astg_encode(stg: &mut Stg, heuristic: bool) -> Result<AstgEncodeResult, SiEncodeError>
{
    match stg.num_states()
    {
        0 => Err(SiEncodeError::EmptyStg),
        1 => Ok(AstgEncodeResult
        {
            state_bits: 0,
            seed_count: 0,
            reduced_seed_count: 0,
            prime_count: 0,
            cover: Vec::new(),
        }),
        2 =>
        {
            stg.set_state_encoding(crate::ports::stg::stg::StateId(0), "0")?;
            stg.set_state_encoding(crate::ports::stg::stg::StateId(1), "1")?;

            Ok(AstgEncodeResult
            {
                state_bits: 1,
                seed_count: 0,
                reduced_seed_count: 0,
                prime_count: 0,
                cover: Vec::new(),
            })
        }
        _ => extract_and_solve_constraints(stg, heuristic),
    }
}

pub fn extract_seed_constraints(stg: &Stg) -> input::DichotomyFamily
{
    let state_count = stg.num_states();
    let mut by_input = BTreeMap::<String, BTreeMap<usize, Vec<usize>>>::new();

    for transition in stg.transitions()
    {
        by_input
            .entry(transition.input.clone())
            .or_default()
            .entry(transition.to.0)
            .or_default()
            .push(transition.from.0);
    }

    let mut seeds = input::DichotomyFamily::new(state_count);
    for next_state_table in by_input.values()
    {
        if next_state_table.len() < 2
        {
            continue;
        }

        let entries = next_state_table
            .iter()
            .map(|(to, from_states)| (*to, from_states.as_slice()))
            .collect::<Vec<_>>();

        for left_index in 0..entries.len()
        {
            let (j, from_j) = entries[left_index];
            for (m, from_m) in &entries[left_index + 1..]
            {
                for i in from_j.iter().rev().copied()
                {
                    for k in from_m.iter().rev().copied()
                    {
                        if i != j || k != *m
                        {
                            add_seed_set(&mut seeds, [j, i], [*m]);
                            add_seed_set(&mut seeds, [j, i], [k]);
                            add_seed_set(&mut seeds, [j], [*m, k]);
                            add_seed_set(&mut seeds, [i], [*m, k]);
                            add_seed_set(&mut seeds, [*m, k], [j]);
                            add_seed_set(&mut seeds, [*m, k], [i]);
                            add_seed_set(&mut seeds, [*m], [j, i]);
                            add_seed_set(&mut seeds, [k], [j, i]);
                        }
                    }
                }
            }
        }
    }

    seeds
}

fn extract_and_solve_constraints(
    stg: &mut Stg,
    heuristic: bool,
) -> Result<AstgEncodeResult, SiEncodeError>
{
    let seed_list = extract_seed_constraints(stg);
    let unique_seeds = input::gen_uniq(&seed_list);
    let reduced_seeds = input::reduce_seeds(&unique_seeds);
    let prime_list = gen_eqn::gen_eqn(&to_gen_eqn_family(&reduced_seeds)?, usize::MAX)?;
    let cover_matrix = dic_to_sm::dic_to_sm(
        &to_dic_to_sm_family_from_gen_eqn(&prime_list)?,
        &to_dic_to_sm_family_from_input(&reduced_seeds)?,
    )?;
    let cover_result = mincov::sm_minimum_cover(cover_matrix.matrix(), None, heuristic, 0)?;

    encode_stg(stg, &cover_result.cover, &cover_matrix)?;

    Ok(AstgEncodeResult
    {
        state_bits: cover_result.cover.len(),
        seed_count: seed_list.len(),
        reduced_seed_count: reduced_seeds.len(),
        prime_count: prime_list.len(),
        cover: cover_result.cover,
    })
}

fn encode_stg(
    stg: &mut Stg,
    cover: &[usize],
    matrix: &dic_to_sm::DichotomyCoverMatrix,
) -> Result<(), SiEncodeError>
{
    let state_count = stg.num_states();
    let mut codes = vec![String::with_capacity(cover.len()); state_count];

    for column in cover
    {
        let prime = matrix
            .column_prime(*column)
            .ok_or(dic_to_sm::DicToSmError::MissingColumnPrime { column: *column })?;

        for (state_index, code) in codes.iter_mut().enumerate()
        {
            code.push(if prime.lhs().contains(&state_index)
            {
                '1'
            }
            else
            {
                '0'
            });
        }
    }

    for (state_index, code) in codes.into_iter().enumerate()
    {
        stg.set_state_encoding(crate::ports::stg::stg::StateId(state_index), code)?;
    }

    Ok(())
}

fn add_seed_set<const L: usize, const R: usize>(
    family: &mut input::DichotomyFamily,
    lhs: [usize; L],
    rhs: [usize; R],
)
{
    let lhs = lhs.into_iter().collect::<BTreeSet<_>>();
    let rhs = rhs.into_iter().collect::<BTreeSet<_>>();
    family.add_irredundant(input::Dichotomy::new(lhs, rhs));
}

fn to_gen_eqn_family(
    family: &input::DichotomyFamily,
) -> Result<gen_eqn::DichotomyFamily, SiEncodeError>
{
    gen_eqn::DichotomyFamily::from_dichotomies(
        family.element_count(),
        family.dichotomies().iter().map(|dichotomy|
        {
            gen_eqn::Dichotomy::new(
                dichotomy.lhs().iter().copied(),
                dichotomy.rhs().iter().copied(),
            )
        }),
    )
    .map_err(Into::into)
}

fn to_dic_to_sm_family_from_input(
    family: &input::DichotomyFamily,
) -> Result<dic_to_sm::DichotomyFamily, SiEncodeError>
{
    dic_to_sm::DichotomyFamily::from_dichotomies(
        family.element_count(),
        family
            .dichotomies()
            .iter()
            .map(|dichotomy|
            {
                dic_to_sm::Dichotomy::from_sides(
                    family.element_count(),
                    dichotomy.lhs().iter().copied(),
                    dichotomy.rhs().iter().copied(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(Into::into)
}

fn to_dic_to_sm_family_from_gen_eqn(
    family: &gen_eqn::DichotomyFamily,
) -> Result<dic_to_sm::DichotomyFamily, SiEncodeError>
{
    dic_to_sm::DichotomyFamily::from_dichotomies(
        family.element_count(),
        family
            .dichotomies()
            .iter()
            .map(|dichotomy|
            {
                dic_to_sm::Dichotomy::from_sides(
                    family.element_count(),
                    dichotomy.lhs().iter().copied(),
                    dichotomy.rhs().iter().copied(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(Into::into)
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::ports::stg::stg::StateId;

    fn stg_with_states(state_count: usize) -> Stg
    {
        let mut stg = Stg::with_dimensions(1, 1);
        for index in 0..state_count
        {
            stg.create_state(Some(format!("s{index}")), None::<String>);
        }

        stg
    }

    #[test]
    fn empty_stg_is_rejected()
    {
        let mut stg = Stg::new();

        assert_eq!(astg_encode(&mut stg, false), Err(SiEncodeError::EmptyStg));
    }

    #[test]
    fn two_state_machine_gets_legacy_zero_one_encoding()
    {
        let mut stg = stg_with_states(2);

        let result = astg_encode(&mut stg, false).unwrap();

        assert_eq!(result.state_bits, 1);
        assert_eq!(stg.state_encoding(StateId(0)), Some("0"));
        assert_eq!(stg.state_encoding(StateId(1)), Some("1"));
    }

    #[test]
    fn seed_extraction_matches_stt_eight_constraint_pattern()
    {
        let mut stg = stg_with_states(4);
        stg.create_transition(StateId(0), StateId(1), "0", "0")
            .unwrap();
        stg.create_transition(StateId(2), StateId(3), "0", "0")
            .unwrap();

        let seeds = extract_seed_constraints(&stg);

        assert_eq!(seeds.len(), 8);
        assert_eq!(
            seeds.dichotomies(),
            &[
                input::Dichotomy::new([1, 0], [3]),
                input::Dichotomy::new([1, 0], [2]),
                input::Dichotomy::new([1], [3, 2]),
                input::Dichotomy::new([0], [3, 2]),
                input::Dichotomy::new([3, 2], [1]),
                input::Dichotomy::new([3, 2], [0]),
                input::Dichotomy::new([3], [1, 0]),
                input::Dichotomy::new([2], [1, 0]),
            ]
        );
    }

    #[test]
    fn seed_extraction_skips_single_next_state_input_classes()
    {
        let mut stg = stg_with_states(3);
        stg.create_transition(StateId(0), StateId(1), "0", "0")
            .unwrap();
        stg.create_transition(StateId(2), StateId(1), "0", "0")
            .unwrap();

        let seeds = extract_seed_constraints(&stg);

        assert!(seeds.is_empty());
    }

    #[test]
    fn assigns_codes_for_multi_state_machine()
    {
        let mut stg = stg_with_states(4);
        stg.create_transition(StateId(0), StateId(1), "0", "0")
            .unwrap();
        stg.create_transition(StateId(2), StateId(3), "0", "0")
            .unwrap();

        let result = astg_encode(&mut stg, false).unwrap();

        assert_eq!(result.seed_count, 8);
        assert_eq!(result.state_bits, 2);
        assert_eq!(stg.state_encoding(StateId(0)), Some("00"));
        assert_eq!(stg.state_encoding(StateId(1)), Some("10"));
        assert_eq!(stg.state_encoding(StateId(2)), Some("11"));
        assert_eq!(stg.state_encoding(StateId(3)), Some("01"));
    }

    #[test]
    fn no_legacy_exports_or_dependency_metadata_are_present()
    {
        let source = include_str!("si_encode.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
    }
}
