//! Source-aligned binary variable pairing helpers from Espresso `pair.c`.
//!
//! Pairing is an optional command-line transformation. The native port keeps
//! the pairing data structure, validation, greedy selection, and all-pair
//! enumeration as pure Rust helpers so callers can build pairing workflows
//! without depending on C allocation or process globals.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pairing {
    pairs: Vec<(usize, usize)>,
}

impl Pairing {
    pub fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    pub fn from_pairs(pairs: impl IntoIterator<Item = (usize, usize)>) -> Self {
        Self {
            pairs: pairs.into_iter().collect(),
        }
    }

    pub fn pairs(&self) -> &[(usize, usize)] {
        &self.pairs
    }

    pub fn push(&mut self, left: usize, right: usize) {
        self.pairs.push((left, right));
    }

    pub fn validate(&self, binary_variables: usize) -> Result<(), PairError> {
        let mut seen = BTreeSet::new();
        for &(left, right) in &self.pairs {
            if left == 0 || right == 0 || left > binary_variables || right > binary_variables {
                return Err(PairError::OutOfRange { left, right });
            }
            if left == right {
                return Err(PairError::SelfPair { variable: left });
            }
            if !seen.insert(left) || !seen.insert(right) {
                return Err(PairError::DuplicateVariable);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairError {
    OutOfRange { left: usize, right: usize },
    SelfPair { variable: usize },
    DuplicateVariable,
}

pub fn greedy_best_cost(cost: &[Vec<i32>]) -> Pairing {
    let mut used = BTreeSet::new();
    let mut result = Pairing::new();

    loop {
        let mut best = None;
        for left in 0..cost.len() {
            if used.contains(&(left + 1)) {
                continue;
            }
            for right in (left + 1)..cost.len() {
                if used.contains(&(right + 1)) {
                    continue;
                }
                let value = cost[left][right];
                if best
                    .map(|(_, _, best_value)| value > best_value)
                    .unwrap_or(true)
                {
                    best = Some((left + 1, right + 1, value));
                }
            }
        }

        let Some((left, right, _)) = best else {
            break;
        };
        used.insert(left);
        used.insert(right);
        result.push(left, right);
    }

    result
}

pub fn generate_all_pairs(binary_variables: usize) -> Vec<Pairing> {
    fn recur(candidate: Vec<usize>, current: Pairing, output: &mut Vec<Pairing>) {
        let Some((&first, rest)) = candidate.split_first() else {
            output.push(current);
            return;
        };

        if rest.is_empty() {
            output.push(current);
            return;
        }

        for (index, &second) in rest.iter().enumerate() {
            let mut next_candidate = rest.to_vec();
            next_candidate.remove(index);
            let mut next = current.clone();
            next.push(first, second);
            recur(next_candidate, next, output);
        }

        if candidate.len() % 2 == 1 {
            recur(rest.to_vec(), current, output);
        }
    }

    let mut output = Vec::new();
    recur(
        (1..=binary_variables).collect(),
        Pairing::new(),
        &mut output,
    );
    output
}

pub fn pair_part(a_is_one: bool, b_is_one: bool) -> usize {
    match (a_is_one, b_is_one) {
        (false, false) => 3,
        (false, true) => 2,
        (true, false) => 1,
        (true, true) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_validation_matches_binary_variable_constraints() {
        assert!(Pairing::from_pairs([(1, 2), (3, 4)]).validate(4).is_ok());
        assert_eq!(
            Pairing::from_pairs([(1, 5)]).validate(4),
            Err(PairError::OutOfRange { left: 1, right: 5 })
        );
        assert_eq!(
            Pairing::from_pairs([(1, 1)]).validate(4),
            Err(PairError::SelfPair { variable: 1 })
        );
        assert_eq!(
            Pairing::from_pairs([(1, 2), (2, 3)]).validate(4),
            Err(PairError::DuplicateVariable)
        );
    }

    #[test]
    fn greedy_best_cost_keeps_disjoint_highest_benefit_pairs() {
        let costs = vec![
            vec![0, 4, 1, 8],
            vec![4, 0, 3, 2],
            vec![1, 3, 0, 9],
            vec![8, 2, 9, 0],
        ];

        assert_eq!(greedy_best_cost(&costs).pairs(), &[(3, 4), (1, 2)]);
    }

    #[test]
    fn generate_all_pairs_matches_pair_c_odd_variable_branch() {
        let generated = generate_all_pairs(3);
        let pairs = generated
            .iter()
            .map(|pairing| pairing.pairs().to_vec())
            .collect::<Vec<_>>();

        assert_eq!(pairs, vec![vec![(1, 2)], vec![(1, 3)], vec![(2, 3)]]);
    }

    #[test]
    fn pair_part_matches_pairvar_column_order() {
        assert_eq!(pair_part(false, false), 3);
        assert_eq!(pair_part(false, true), 2);
        assert_eq!(pair_part(true, false), 1);
        assert_eq!(pair_part(true, true), 0);
    }
}
