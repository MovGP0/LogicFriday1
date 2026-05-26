//! Native Rust random leaf ordering for SIS.
//!
//! The legacy routine assigns each leaf a unique variable index from a random
//! permutation and does not produce an ordered root list. This port keeps that
//! behavior as an owned Rust API and uses the same 48-bit generator family that
//! the SIS portability layer mapped to `random`.

use std::error::Error;
use std::fmt;

const MULTIPLIER: u64 = 0x5deece66d;
const ADDEND: u64 = 0xb;
const MASK: u64 = (1_u64 << 48) - 1;
const SEED_LOW_BITS: u64 = 0x330e;

pub const UNASSIGNED_ORDER: isize = -1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomOrderLeaf<N> {
    pub leaf: N,
    pub order: isize,
}

impl<N> RandomOrderLeaf<N> {
    pub fn new(leaf: N) -> Self {
        Self {
            leaf,
            order: UNASSIGNED_ORDER,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RandomOrderError {
    LeafAlreadyOrdered { index: usize, order: isize },
    LeafCountOutOfRange(usize),
}

impl fmt::Display for RandomOrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LeafAlreadyOrdered { index, order } => {
                write!(f, "leaf {index} already has assigned order {order}")
            }
            Self::LeafCountOutOfRange(count) => {
                write!(f, "leaf count {count} is outside the supported range")
            }
        }
    }
}

impl Error for RandomOrderError {}

pub type RandomOrderResult<T> = Result<T, RandomOrderError>;

pub fn order_random<N>(leaves: &mut [RandomOrderLeaf<N>], seed: i64) -> RandomOrderResult<()> {
    validate_unassigned(leaves)?;

    let permutation = random_permutation(seed, leaves.len())?;
    for (leaf, order) in leaves.iter_mut().zip(permutation) {
        leaf.order = order as isize;
    }

    Ok(())
}

pub fn random_permutation(seed: i64, n_elts: usize) -> RandomOrderResult<Vec<usize>> {
    if n_elts == 0 {
        return Ok(Vec::new());
    }
    if n_elts > i32::MAX as usize {
        return Err(RandomOrderError::LeafCountOutOfRange(n_elts));
    }

    let mut rng = Lrand48::new(seed);
    let mut remaining = (0..n_elts).collect::<Vec<_>>();
    let mut permutation = Vec::with_capacity(n_elts);

    while !remaining.is_empty() {
        let next_value = (rng.next() as usize) % remaining.len();
        permutation.push(remaining.remove(next_value));
    }

    Ok(permutation)
}

fn validate_unassigned<N>(leaves: &[RandomOrderLeaf<N>]) -> RandomOrderResult<()> {
    for (index, leaf) in leaves.iter().enumerate() {
        if leaf.order != UNASSIGNED_ORDER {
            return Err(RandomOrderError::LeafAlreadyOrdered {
                index,
                order: leaf.order,
            });
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Lrand48 {
    state: u64,
}

impl Lrand48 {
    fn new(seed: i64) -> Self {
        let seed = seed as u32 as u64;
        Self {
            state: ((seed << 16) | SEED_LOW_BITS) & MASK,
        }
    }

    fn next(&mut self) -> i64 {
        self.state = self.state.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND) & MASK;
        (self.state >> 17) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_leaf_set_is_a_noop() {
        let mut leaves = Vec::<RandomOrderLeaf<&str>>::new();

        assert_eq!(order_random(&mut leaves, 7), Ok(()));
        assert!(leaves.is_empty());
        assert_eq!(random_permutation(7, 0), Ok(Vec::new()));
    }

    #[test]
    fn random_permutation_matches_lrand48_selection_sequence() {
        assert_eq!(random_permutation(1, 6), Ok(vec![4, 3, 1, 2, 5, 0]));
        assert_eq!(random_permutation(42, 6), Ok(vec![1, 2, 5, 0, 4, 3]));
    }

    #[test]
    fn assigns_permutation_to_leaves_in_iteration_order() {
        let mut leaves = ["a", "b", "c", "d", "e", "f"]
            .into_iter()
            .map(RandomOrderLeaf::new)
            .collect::<Vec<_>>();

        order_random(&mut leaves, 1).unwrap();

        assert_eq!(
            leaves.iter().map(|leaf| leaf.order).collect::<Vec<_>>(),
            vec![4, 3, 1, 2, 5, 0]
        );
        assert_eq!(
            leaves.iter().map(|leaf| leaf.leaf).collect::<Vec<_>>(),
            vec!["a", "b", "c", "d", "e", "f"]
        );
    }

    #[test]
    fn rejects_preassigned_leaf_orders() {
        let mut leaves = vec![
            RandomOrderLeaf::new("a"),
            RandomOrderLeaf {
                leaf: "b",
                order: 0,
            },
        ];

        assert_eq!(
            order_random(&mut leaves, 1),
            Err(RandomOrderError::LeafAlreadyOrdered { index: 1, order: 0 })
        );
    }

    #[test]
    fn negative_seed_uses_the_legacy_low_32_bits() {
        assert_eq!(
            random_permutation(-1, 5),
            random_permutation(u32::MAX as i64, 5)
        );
    }
}
