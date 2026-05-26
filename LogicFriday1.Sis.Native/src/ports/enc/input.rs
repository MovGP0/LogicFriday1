use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

const LINE_LIMIT: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstraintError
{
    Io(String),
    LineTooLong,
    NoConstraints,
    VaryingConstraintLength
    {
        expected: usize,
        actual: usize,
    },
    InvalidConstraintSymbol
    {
        symbol: char,
        constraint: String,
    },
}

impl fmt::Display for ConstraintError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::Io(message) => write!(f, "{message}"),
            Self::LineTooLong => write!(f, "line is too long to handle"),
            Self::NoConstraints => write!(f, "no input constraints were found"),
            Self::VaryingConstraintLength
            {
                expected,
                actual,
            } => write!(
                f,
                "varying constraint length: expected {expected}, found {actual}"
            ),
            Self::InvalidConstraintSymbol
            {
                symbol,
                constraint,
            } => write!(
                f,
                "invalid constraint symbol '{symbol}' in {constraint}"
            ),
        }
    }
}

impl std::error::Error for ConstraintError
{
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dichotomy
{
    lhs: BTreeSet<usize>,
    rhs: BTreeSet<usize>,
}

impl Dichotomy
{
    pub fn new(lhs: impl IntoIterator<Item = usize>, rhs: impl IntoIterator<Item = usize>) -> Self
    {
        Self
        {
            lhs: lhs.into_iter().collect(),
            rhs: rhs.into_iter().collect(),
        }
    }

    pub fn lhs(&self) -> &BTreeSet<usize>
    {
        &self.lhs
    }

    pub fn rhs(&self) -> &BTreeSet<usize>
    {
        &self.rhs
    }

    pub fn implies(&self, other: &Self) -> bool
    {
        self.lhs.is_subset(&other.lhs) && self.rhs.is_subset(&other.rhs)
    }

    pub fn contains(&self, element: usize) -> bool
    {
        self.lhs.contains(&element) || self.rhs.contains(&element)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DichotomyFamily
{
    element_count: usize,
    dichotomies: Vec<Dichotomy>,
}

impl DichotomyFamily
{
    pub fn new(element_count: usize) -> Self
    {
        Self
        {
            element_count,
            dichotomies: Vec::new(),
        }
    }

    pub fn element_count(&self) -> usize
    {
        self.element_count
    }

    pub fn len(&self) -> usize
    {
        self.dichotomies.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.dichotomies.is_empty()
    }

    pub fn dichotomies(&self) -> &[Dichotomy]
    {
        &self.dichotomies
    }

    pub fn add(&mut self, dichotomy: Dichotomy)
    {
        self.dichotomies.push(dichotomy);
    }

    pub fn add_irredundant(&mut self, dichotomy: Dichotomy)
    {
        for existing in &mut self.dichotomies
        {
            if dichotomy.implies(existing)
            {
                return;
            }

            if existing.implies(&dichotomy)
            {
                *existing = dichotomy;
                return;
            }
        }

        self.add(dichotomy);
    }
}

pub fn filter_constraints_file(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<(), ConstraintError>
{
    let text = fs::read_to_string(input.as_ref()).map_err(|error| {
        ConstraintError::Io(format!(
            "unable to open constraint file {}: {error}",
            input.as_ref().display()
        ))
    })?;
    let filtered = filter_constraints(&text)?;
    fs::write(output.as_ref(), filtered).map_err(|error| {
        ConstraintError::Io(format!(
            "unable to write constraints file {}: {error}",
            output.as_ref().display()
        ))
    })
}

pub fn read_constraints_file(input: impl AsRef<Path>) -> Result<DichotomyFamily, ConstraintError>
{
    let text = fs::read_to_string(input.as_ref()).map_err(|error| {
        ConstraintError::Io(format!(
            "unable to open constraint file {}: {error}",
            input.as_ref().display()
        ))
    })?;
    read_constraints(&text)
}

pub fn filter_constraints(input: &str) -> Result<String, ConstraintError>
{
    let constraints = parse_constraints(input)?;
    let Some(width) = constraints.first().map(String::len) else
    {
        return Err(ConstraintError::NoConstraints);
    };

    let mut used = vec![false; width];
    for constraint in &constraints
    {
        for (index, symbol) in constraint.chars().enumerate()
        {
            validate_constraint_symbol(symbol, constraint)?;
            if symbol == '1'
            {
                used[index] = true;
            }
        }
    }

    let mut output = String::new();
    for constraint in constraints
    {
        output.push_str("input: ");
        for (index, symbol) in constraint.chars().enumerate()
        {
            if used[index]
            {
                output.push(symbol);
            }
        }
        output.push('\n');
    }

    Ok(output)
}

pub fn read_constraints(input: &str) -> Result<DichotomyFamily, ConstraintError>
{
    let constraints = parse_constraints(input)?;
    let Some(width) = constraints.first().map(String::len) else
    {
        return Err(ConstraintError::NoConstraints);
    };

    let mut family = DichotomyFamily::new(width);

    for constraint in constraints
    {
        let mut lhs = BTreeSet::new();
        let mut rhs_candidates = Vec::new();

        for (index, symbol) in constraint.chars().enumerate()
        {
            match symbol
            {
                '1' =>
                {
                    lhs.insert(index);
                }
                '-' =>
                {
                }
                '0' =>
                {
                    rhs_candidates.push(index);
                }
                _ =>
                {
                    return Err(ConstraintError::InvalidConstraintSymbol
                    {
                        symbol,
                        constraint,
                    });
                }
            }
        }

        for rhs_index in rhs_candidates
        {
            family.add_irredundant(Dichotomy::new(
                lhs.iter().copied(),
                std::iter::once(rhs_index),
            ));
            family.add_irredundant(Dichotomy::new(
                std::iter::once(rhs_index),
                lhs.iter().copied(),
            ));
        }
    }

    Ok(family)
}

pub fn gen_uniq(family: &DichotomyFamily) -> DichotomyFamily
{
    let mut output = family.clone();
    let mut implied = vec![vec![false; family.element_count]; family.element_count];

    for dichotomy in family.dichotomies()
    {
        for lhs in dichotomy.lhs()
        {
            for rhs in dichotomy.rhs()
            {
                let low = (*lhs).min(*rhs);
                let high = (*lhs).max(*rhs);
                implied[low][high] = true;
            }
        }
    }

    for lhs in 0..family.element_count
    {
        for rhs in (lhs + 1)..family.element_count
        {
            if !implied[lhs][rhs]
            {
                output.add(Dichotomy::new([lhs], [rhs]));
                output.add(Dichotomy::new([rhs], [lhs]));
            }
        }
    }

    output
}

pub fn reduce_seeds(family: &DichotomyFamily) -> DichotomyFamily
{
    let mut active = vec![true; family.len()];

    for index in 0..family.len()
    {
        if !active[index]
        {
            continue;
        }

        for compare_index in (index + 1)..family.len()
        {
            if family.dichotomies()[compare_index].implies(&family.dichotomies()[index])
            {
                active[compare_index] = false;
            }
        }
    }

    let mut reduced = DichotomyFamily::new(family.element_count);
    for (index, dichotomy) in family.dichotomies().iter().enumerate()
    {
        if active[index]
        {
            reduced.add(dichotomy.clone());
        }
    }

    let mut counts = vec![0usize; family.element_count];
    for dichotomy in reduced.dichotomies()
    {
        for element in 0..family.element_count
        {
            if dichotomy.contains(element)
            {
                counts[element] += 1;
            }
        }
    }

    let anchor = counts
        .iter()
        .enumerate()
        .max_by_key(|(index, count)| (**count, std::cmp::Reverse(*index)))
        .map(|(index, _)| index)
        .unwrap_or(0);

    let mut output = DichotomyFamily::new(family.element_count);
    for dichotomy in reduced.dichotomies()
    {
        if !dichotomy.lhs().contains(&anchor)
        {
            output.add(dichotomy.clone());
        }
    }

    output
}

fn parse_constraints(input: &str) -> Result<Vec<String>, ConstraintError>
{
    let mut constraints = Vec::new();
    let mut width = None;

    for line in input.lines()
    {
        if line.len() >= LINE_LIMIT
        {
            continue;
        }

        let Some(symbols) = parse_input_symbols(line) else
        {
            continue;
        };

        if let Some(expected) = width
        {
            if expected != symbols.len()
            {
                return Err(ConstraintError::VaryingConstraintLength
                {
                    expected,
                    actual: symbols.len(),
                });
            }
        }
        else
        {
            width = Some(symbols.len());
        }

        constraints.push(symbols.to_owned());
    }

    Ok(constraints)
}

fn parse_input_symbols(line: &str) -> Option<&str>
{
    let mut fields = line.split_whitespace();
    if fields.next()? != "input:"
    {
        return None;
    }

    fields.next()
}

fn validate_constraint_symbol(symbol: char, constraint: &str) -> Result<(), ConstraintError>
{
    match symbol
    {
        '0' | '1' | '-' => Ok(()),
        _ => Err(ConstraintError::InvalidConstraintSymbol
        {
            symbol,
            constraint: constraint.to_owned(),
        }),
    }
}

impl From<io::Error> for ConstraintError
{
    fn from(error: io::Error) -> Self
    {
        Self::Io(error.to_string())
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn filter_constraints_removes_columns_never_marked_one()
    {
        let output = filter_constraints("comment\ninput: 10-0\ninput: 0-10\n").unwrap();

        assert_eq!(output, "input: 1-\ninput: 01\n");
    }

    #[test]
    fn filter_constraints_rejects_varying_lengths()
    {
        let error = filter_constraints("input: 100\ninput: 10\n").unwrap_err();

        assert_eq!(
            error,
            ConstraintError::VaryingConstraintLength
            {
                expected: 3,
                actual: 2,
            }
        );
    }

    #[test]
    fn read_constraints_forms_seed_dichotomies()
    {
        let family = read_constraints("input: 10-0\n").unwrap();

        assert_eq!(family.element_count(), 4);
        assert_eq!(
            family.dichotomies(),
            &[
                Dichotomy::new([0], [1]),
                Dichotomy::new([1], [0]),
                Dichotomy::new([0], [3]),
                Dichotomy::new([3], [0]),
            ]
        );
    }

    #[test]
    fn read_constraints_preserves_non_implying_seed_pairs()
    {
        let family = read_constraints("input: 10-\ninput: 110\n").unwrap();

        assert_eq!(
            family.dichotomies(),
            &[
                Dichotomy::new([0], [1]),
                Dichotomy::new([1], [0]),
                Dichotomy::new([0, 1], [2]),
                Dichotomy::new([2], [0, 1]),
            ]
        );
    }

    #[test]
    fn gen_uniq_adds_missing_ordered_pairs()
    {
        let mut family = DichotomyFamily::new(3);
        family.add(Dichotomy::new([0], [1]));

        let unique = gen_uniq(&family);

        assert_eq!(
            unique.dichotomies(),
            &[
                Dichotomy::new([0], [1]),
                Dichotomy::new([0], [2]),
                Dichotomy::new([2], [0]),
                Dichotomy::new([1], [2]),
                Dichotomy::new([2], [1]),
            ]
        );
    }

    #[test]
    fn reduce_seeds_removes_implied_later_entries_and_anchors_most_frequent_element()
    {
        let mut family = DichotomyFamily::new(3);
        family.add(Dichotomy::new([1, 2], [0]));
        family.add(Dichotomy::new([1], [0]));

        let reduced = reduce_seeds(&family);

        assert_eq!(
            reduced.dichotomies(),
            &[
                Dichotomy::new([1, 2], [0]),
            ]
        );
    }

    #[test]
    fn rejects_invalid_constraint_symbols()
    {
        let error = read_constraints("input: 1x0\n").unwrap_err();

        assert_eq!(
            error,
            ConstraintError::InvalidConstraintSymbol
            {
                symbol: 'x',
                constraint: "1x0".to_owned(),
            }
        );
    }
}
