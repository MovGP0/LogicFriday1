//! Legacy CMU BDD table sizes.
//!
//! The original C module exposes a global `long bdd_primes[]` array used to
//! choose hash-table growth sizes.  This Rust port keeps the same ordered
//! values and provides slice/query helpers instead of mutable global storage.

pub static BDD_PRIMES: &[usize] = &[
    1,
    2,
    3,
    7,
    13,
    23,
    59,
    113,
    241,
    503,
    1019,
    2039,
    4091,
    8179,
    11587,
    16369,
    23143,
    32749,
    46349,
    65521,
    92683,
    131063,
    185363,
    262139,
    330287,
    416147,
    524269,
    660557,
    832253,
    1048571,
    1321109,
    1664501,
    2097143,
    2642201,
    3328979,
    4194287,
    5284393,
    6657919,
    8388593,
    10568797,
    13315831,
    16777199,
    33554393,
    67108859,
    134217689,
    268435399,
    536870879,
    1073741789,
    2147483629,
];

pub fn bdd_primes() -> &'static [usize]
{
    BDD_PRIMES
}

pub fn bdd_prime_at(index: usize) -> Option<usize>
{
    BDD_PRIMES.get(index).copied()
}

pub fn is_bdd_prime_size(size: usize) -> bool
{
    BDD_PRIMES.binary_search(&size).is_ok()
}

pub fn next_bdd_prime_size(current_size: usize) -> Option<usize>
{
    BDD_PRIMES
        .iter()
        .copied()
        .find(|candidate| *candidate > current_size)
}

pub fn next_legacy_bdd_prime_size(current_size: usize) -> Option<usize>
{
    BDD_PRIMES
        .windows(2)
        .find_map(|pair| (pair[0] == current_size).then_some(pair[1]))
}

pub fn largest_bdd_prime_size() -> usize
{
    BDD_PRIMES[BDD_PRIMES.len() - 1]
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn table_matches_legacy_endpoints_and_length()
    {
        assert_eq!(BDD_PRIMES.len(), 49);
        assert_eq!(bdd_prime_at(0), Some(1));
        assert_eq!(bdd_prime_at(7), Some(113));
        assert_eq!(bdd_prime_at(48), Some(2147483629));
        assert_eq!(bdd_prime_at(49), None);
    }

    #[test]
    fn table_is_strictly_increasing()
    {
        for pair in BDD_PRIMES.windows(2)
        {
            assert!(pair[0] < pair[1]);
        }
    }

    #[test]
    fn membership_uses_legacy_table_values()
    {
        assert!(is_bdd_prime_size(1));
        assert!(is_bdd_prime_size(1048571));
        assert!(is_bdd_prime_size(2147483629));
        assert!(!is_bdd_prime_size(4));
        assert!(!is_bdd_prime_size(2147483630));
    }

    #[test]
    fn next_size_returns_first_larger_table_entry()
    {
        assert_eq!(next_bdd_prime_size(0), Some(1));
        assert_eq!(next_bdd_prime_size(1), Some(2));
        assert_eq!(next_bdd_prime_size(113), Some(241));
        assert_eq!(next_bdd_prime_size(114), Some(241));
        assert_eq!(next_bdd_prime_size(2147483629), None);
    }

    #[test]
    fn next_legacy_size_requires_exact_current_entry()
    {
        assert_eq!(next_legacy_bdd_prime_size(1), Some(2));
        assert_eq!(next_legacy_bdd_prime_size(113), Some(241));
        assert_eq!(next_legacy_bdd_prime_size(114), None);
        assert_eq!(next_legacy_bdd_prime_size(2147483629), None);
    }

    #[test]
    fn largest_size_matches_last_legacy_entry()
    {
        assert_eq!(largest_bdd_prime_size(), 2147483629);
    }

    #[test]
    fn accessor_returns_the_shared_table()
    {
        assert_eq!(bdd_primes().as_ptr(), BDD_PRIMES.as_ptr());
        assert_eq!(bdd_primes().len(), BDD_PRIMES.len());
    }
}
