//! Native Rust permutation helper for `sis/genlib/permute.c`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermuteError {
    TooManyItems { count: usize },
}

impl fmt::Display for PermuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyItems { count } => {
                write!(f, "permutation count for {count} items overflows usize")
            }
        }
    }
}

impl std::error::Error for PermuteError {}

pub fn gl_permute<T, S, F>(array: &mut [T], state: &mut S, mut visit: F)
where
    F: FnMut(&[T], &mut S),
{
    gl_permute_recur(array, 0, state, &mut visit);
}

pub fn gl_permute_result<T, S, F, E>(array: &mut [T], state: &mut S, mut visit: F) -> Result<(), E>
where
    F: FnMut(&[T], &mut S) -> Result<(), E>,
{
    gl_permute_recur_result(array, 0, state, &mut visit)
}

pub fn collect_permutations<T>(items: &[T]) -> Result<Vec<Vec<T>>, PermuteError>
where
    T: Clone,
{
    let count = permutation_count(items.len())?;
    let mut array = items.to_vec();
    let mut permutations = Vec::with_capacity(count);

    gl_permute(&mut array, &mut permutations, |permutation, output| {
        output.push(permutation.to_vec());
    });

    Ok(permutations)
}

pub fn permutation_count(count: usize) -> Result<usize, PermuteError> {
    let mut total = 1usize;

    for value in 2..=count {
        total = total
            .checked_mul(value)
            .ok_or(PermuteError::TooManyItems { count })?;
    }

    Ok(total)
}

fn gl_permute_recur<T, S, F>(array: &mut [T], start: usize, state: &mut S, visit: &mut F)
where
    F: FnMut(&[T], &mut S),
{
    if array.len().saturating_sub(start) <= 1 {
        visit(array, state);
        return;
    }

    for index in start..array.len() {
        array.swap(index, start);
        gl_permute_recur(array, start + 1, state, visit);
        array.swap(index, start);
    }
}

fn gl_permute_recur_result<T, S, F, E>(
    array: &mut [T],
    start: usize,
    state: &mut S,
    visit: &mut F,
) -> Result<(), E>
where
    F: FnMut(&[T], &mut S) -> Result<(), E>,
{
    if array.len().saturating_sub(start) <= 1 {
        return visit(array, state);
    }

    for index in start..array.len() {
        array.swap(index, start);
        let result = gl_permute_recur_result(array, start + 1, state, visit);
        array.swap(index, start);
        result?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visits_permutations_in_legacy_order() {
        let mut values = [1, 2, 3];
        let mut seen = Vec::new();

        gl_permute(&mut values, &mut seen, |permutation, output| {
            output.push(permutation.to_vec());
        });

        assert_eq!(
            seen,
            vec![
                vec![1, 2, 3],
                vec![1, 3, 2],
                vec![2, 1, 3],
                vec![2, 3, 1],
                vec![3, 2, 1],
                vec![3, 1, 2],
            ]
        );
    }

    #[test]
    fn restores_input_after_visiting() {
        let mut values = ["a", "b", "c", "d"];
        let original = values;
        let mut count = 0usize;

        gl_permute(&mut values, &mut count, |_, output| {
            *output += 1;
        });

        assert_eq!(values, original);
        assert_eq!(count, 24);
    }

    #[test]
    fn passes_state_through_callback() {
        let mut values = [1, 2, 3];
        let mut sums = Vec::new();

        gl_permute(&mut values, &mut sums, |permutation, output| {
            output.push(permutation[0] * 100 + permutation[1] * 10 + permutation[2]);
        });

        assert_eq!(sums, vec![123, 132, 213, 231, 321, 312]);
    }

    #[test]
    fn supports_duplicate_items_without_filtering() {
        let mut values = ["x", "x", "y"];
        let mut seen = Vec::new();

        gl_permute(&mut values, &mut seen, |permutation, output| {
            output.push(permutation.join(""));
        });

        assert_eq!(seen, vec!["xxy", "xyx", "xxy", "xyx", "yxx", "yxx"]);
    }

    #[test]
    fn single_item_is_visited_once() {
        let mut values = [42];
        let mut seen = Vec::new();

        gl_permute(&mut values, &mut seen, |permutation, output| {
            output.push(permutation.to_vec());
        });

        assert_eq!(seen, vec![vec![42]]);
    }

    #[test]
    fn empty_input_is_visited_once() {
        let mut values: [usize; 0] = [];
        let mut count = 0usize;

        gl_permute(&mut values, &mut count, |permutation, output| {
            assert!(permutation.is_empty());
            *output += 1;
        });

        assert_eq!(count, 1);
    }

    #[test]
    fn result_callback_stops_on_error_and_restores_input() {
        let mut values = [1, 2, 3];
        let mut seen = Vec::new();
        let result = gl_permute_result(&mut values, &mut seen, |permutation, output| {
            output.push(permutation.to_vec());
            if output.len() == 3 {
                Err("stop")
            } else {
                Ok(())
            }
        });

        assert_eq!(result, Err("stop"));
        assert_eq!(values, [1, 2, 3]);
        assert_eq!(seen, vec![vec![1, 2, 3], vec![1, 3, 2], vec![2, 1, 3]]);
    }

    #[test]
    fn collect_permutations_preallocates_and_collects() {
        let permutations = collect_permutations(&['a', 'b']).unwrap();

        assert_eq!(permutations, vec![vec!['a', 'b'], vec!['b', 'a']]);
    }

    #[test]
    fn permutation_count_detects_overflow() {
        let max_supported = (1usize..)
            .scan(1usize, |acc, value| {
                *acc = acc.checked_mul(value)?;
                Some(value)
            })
            .last()
            .unwrap();

        assert_eq!(
            permutation_count(max_supported + 1),
            Err(PermuteError::TooManyItems {
                count: max_supported + 1
            })
        );
    }
}
