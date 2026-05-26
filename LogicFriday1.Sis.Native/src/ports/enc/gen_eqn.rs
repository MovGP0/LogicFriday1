use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dichotomy
{
    lhs: BTreeSet<usize>,
    rhs: BTreeSet<usize>,
}

impl Dichotomy
{
    pub fn new(
        lhs: impl IntoIterator<Item = usize>,
        rhs: impl IntoIterator<Item = usize>,
    ) -> Self
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

    pub fn from_dichotomies(
        element_count: usize,
        dichotomies: impl IntoIterator<Item = Dichotomy>,
    ) -> GenEqnResult<Self>
    {
        let mut family = Self::new(element_count);
        for dichotomy in dichotomies
        {
            family.push(dichotomy)?;
        }
        Ok(family)
    }

    pub fn push(&mut self, dichotomy: Dichotomy) -> GenEqnResult<()>
    {
        if let Some(index) = dichotomy
            .lhs
            .iter()
            .chain(dichotomy.rhs.iter())
            .copied()
            .find(|index| *index >= self.element_count)
        {
            return Err(GenEqnError::ElementOutOfRange
            {
                index,
                element_count: self.element_count,
            });
        }

        self.dichotomies.push(dichotomy);
        Ok(())
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenEqnError
{
    ElementOutOfRange
    {
        index: usize,
        element_count: usize,
    },
    CoverLimitExceeded
    {
        limit: usize,
        actual: usize,
    },
}

impl fmt::Display for GenEqnError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::ElementOutOfRange
            {
                index,
                element_count,
            } => write!(
                f,
                "dichotomy element {index} is outside element count {element_count}"
            ),
            Self::CoverLimitExceeded
            {
                limit,
                actual,
            } => write!(f, "cover size {actual} exceeded limit {limit}"),
        }
    }
}

impl Error for GenEqnError {}

pub type GenEqnResult<T> = Result<T, GenEqnError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CnfTerm
{
    var1: usize,
    var2: usize,
}

type Cube = BTreeSet<usize>;
type Cover = Vec<Cube>;

pub fn gen_eqn(dic_list: &DichotomyFamily, cover_limit: usize) -> GenEqnResult<DichotomyFamily>
{
    let variable_count = dic_list.len();
    let mut terms = Vec::new();
    let mut counts = vec![0usize; variable_count];

    for i in 0..variable_count
    {
        let p1 = &dic_list.dichotomies[i];
        for j in i + 1..variable_count
        {
            let p2 = &dic_list.dichotomies[j];
            let first_disjoint = p1.lhs.is_disjoint(&p2.rhs);
            let second_disjoint = p1.rhs.is_disjoint(&p2.lhs);

            if !first_disjoint || !second_disjoint
            {
                terms.push(CnfTerm
                {
                    var1: i,
                    var2: j,
                });
                counts[i] += 1;
                counts[j] += 1;
            }
        }
    }

    let cover = cnf_expand(terms, &mut counts, variable_count, cover_limit)?;
    Ok(cnf_to_prime(dic_list, cover, variable_count))
}

fn cnf_expand(
    terms: Vec<CnfTerm>,
    counts: &mut [usize],
    variable_count: usize,
    limit: usize,
) -> GenEqnResult<Cover>
{
    if terms.is_empty()
    {
        return Ok(vec![Cube::new()]);
    }

    let mut sop = if let Some(special) = special_cases(terms.len(), counts, variable_count)
    {
        special
    }
    else
    {
        let (split_var, lhs_terms, rhs_product) = cnf_split(&terms, counts, variable_count);
        let left_cover = cnf_expand(lhs_terms, counts, variable_count, limit)?;
        cnf_merge(split_var, rhs_product, &left_cover)
    };

    if sop.len() >= limit
    {
        return Err(GenEqnError::CoverLimitExceeded
        {
            limit,
            actual: sop.len(),
        });
    }

    sop.shrink_to_fit();
    Ok(sop)
}

fn cnf_split(
    terms: &[CnfTerm],
    counts: &mut [usize],
    variable_count: usize,
) -> (usize, Vec<CnfTerm>, Cube)
{
    let split_var = (0..variable_count)
        .max_by_key(|index| counts[*index])
        .unwrap_or(0);
    let mut lhs_terms = Vec::new();
    let mut rhs_product = Cube::new();

    for term in terms
    {
        if term.var1 == split_var
        {
            counts[split_var] -= 1;
            counts[term.var2] -= 1;
            rhs_product.insert(term.var2);
        }
        else if term.var2 == split_var
        {
            counts[split_var] -= 1;
            counts[term.var1] -= 1;
            rhs_product.insert(term.var1);
        }
        else
        {
            lhs_terms.push(*term);
        }
    }

    (split_var, lhs_terms, rhs_product)
}

fn cnf_merge(split_var: usize, rhs_product: Cube, left_cover: &[Cube]) -> Cover
{
    let mut sop = left_cover.to_vec();
    let mut rhs_absorbs = false;

    for cube in &mut sop
    {
        if cube.is_subset(&rhs_product)
        {
            rhs_absorbs = true;
            sop = vec![rhs_product.clone()];
            break;
        }

        cube.extend(rhs_product.iter().copied());
    }

    if !rhs_absorbs
    {
        sop = reverse_contain(sop);
    }

    for cube in left_cover
    {
        if rhs_product.is_subset(cube)
        {
            continue;
        }

        let mut lhs_cube = cube.clone();
        lhs_cube.insert(split_var);
        sop.push(lhs_cube);
    }

    sop
}

fn reverse_contain(mut cover: Cover) -> Cover
{
    cover.sort_by_key(Cube::len);
    let mut retained: Cover = Vec::new();

    for cube in cover
    {
        if retained
            .iter()
            .any(|existing| existing.is_subset(&cube))
        {
            continue;
        }

        retained.push(cube);
    }

    retained
}

fn special_cases(term_count: usize, counts: &[usize], variable_count: usize) -> Option<Cover>
{
    for i in 0..variable_count
    {
        if counts[i] == term_count
        {
            let mut second = Cube::new();
            for (j, count) in counts.iter().enumerate()
            {
                if *count != 0 && j != i
                {
                    second.insert(j);
                }
            }

            return Some(vec![Cube::from([i]), second]);
        }
    }

    None
}

fn cnf_to_prime(dic_list: &DichotomyFamily, cover: Cover, variable_count: usize) -> DichotomyFamily
{
    let mut result = DichotomyFamily::new(dic_list.element_count);

    for cube in cover
    {
        let mut lhs = BTreeSet::new();
        let mut rhs = BTreeSet::new();

        for index in 0..variable_count
        {
            if cube.contains(&index)
            {
                continue;
            }

            let dichotomy = &dic_list.dichotomies[index];
            lhs.extend(dichotomy.lhs.iter().copied());
            rhs.extend(dichotomy.rhs.iter().copied());
        }

        result.dichotomies.push(Dichotomy
        {
            lhs,
            rhs,
        });
    }

    result
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn d(lhs: &[usize], rhs: &[usize]) -> Dichotomy
    {
        Dichotomy::new(lhs.iter().copied(), rhs.iter().copied())
    }

    fn family(element_count: usize, dichotomies: &[Dichotomy]) -> DichotomyFamily
    {
        DichotomyFamily::from_dichotomies(element_count, dichotomies.iter().cloned()).unwrap()
    }

    #[test]
    fn returns_single_union_when_there_are_no_incompatibilities()
    {
        let input = family(4, &[d(&[0], &[1]), d(&[2], &[3])]);

        let result = gen_eqn(&input, 10).unwrap();

        assert_eq!(result, family(4, &[d(&[0, 2], &[1, 3])]));
    }

    #[test]
    fn expands_single_incompatibility_into_both_prime_choices()
    {
        let input = family(2, &[d(&[0], &[1]), d(&[1], &[0])]);

        let result = gen_eqn(&input, 10).unwrap();

        assert_eq!(result, family(2, &[d(&[1], &[0]), d(&[0], &[1])]));
    }

    #[test]
    fn applies_special_case_when_one_variable_appears_in_every_term()
    {
        let input = family(3, &[d(&[0], &[1]), d(&[1], &[0]), d(&[1], &[2])]);

        let result = gen_eqn(&input, 10).unwrap();

        assert_eq!(result, family(3, &[d(&[1], &[0, 2]), d(&[0], &[1])]));
    }

    #[test]
    fn recursively_splits_and_merges_non_unate_shape()
    {
        let terms = vec![
            CnfTerm
            {
                var1: 0,
                var2: 1,
            },
            CnfTerm
            {
                var1: 0,
                var2: 2,
            },
            CnfTerm
            {
                var1: 1,
                var2: 3,
            },
        ];
        let mut counts = vec![2, 2, 1, 1];

        let result = cnf_expand(terms, &mut counts, 4, 10).unwrap();

        assert_eq!(
            result,
            vec![
                Cube::from([0, 3]),
                Cube::from([0, 1]),
                Cube::from([1, 2]),
            ]
        );
    }

    #[test]
    fn rejects_dichotomy_elements_outside_family_width()
    {
        let error = DichotomyFamily::from_dichotomies(2, [d(&[0], &[2])]).unwrap_err();

        assert_eq!(
            error,
            GenEqnError::ElementOutOfRange
            {
                index: 2,
                element_count: 2,
            }
        );
    }

    #[test]
    fn reports_cover_limit_instead_of_exiting()
    {
        let input = family(2, &[d(&[0], &[1]), d(&[1], &[0])]);

        let error = gen_eqn(&input, 2).unwrap_err();

        assert_eq!(
            error,
            GenEqnError::CoverLimitExceeded
            {
                limit: 2,
                actual: 2,
            }
        );
    }
}
