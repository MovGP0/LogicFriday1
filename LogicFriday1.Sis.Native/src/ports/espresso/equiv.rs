//! Native Rust output-equivalence analysis for Espresso-style PLAs.
//!
//! The legacy routine compared every pair of output functions by deriving each
//! output's OFF cover, complementing it, and checking cover equivalence in both
//! polarities. This port keeps the behavior native by evaluating the output
//! functions over the input part space and returning structured matches instead
//! of printing from the algorithm.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Variable
{
    first_part: usize,
    last_part: usize,
}

impl Variable
{
    pub const fn new(first_part: usize, last_part: usize) -> Self
    {
        Self {
            first_part,
            last_part,
        }
    }

    pub const fn first_part(self) -> usize
    {
        self.first_part
    }

    pub const fn last_part(self) -> usize
    {
        self.last_part
    }

    pub fn part_count(self) -> usize
    {
        self.last_part - self.first_part + 1
    }

    fn parts(self) -> impl Iterator<Item = usize>
    {
        self.first_part..=self.last_part
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure
{
    variables: Vec<Variable>,
    output_variable_index: usize,
}

impl CubeStructure
{
    pub fn new(
        variables: impl IntoIterator<Item = Variable>,
        output_variable_index: usize,
    ) -> EquivalenceResult<Self>
    {
        let variables = variables.into_iter().collect::<Vec<_>>();
        if output_variable_index >= variables.len()
        {
            return Err(EquivalenceError::OutputVariableOutOfRange {
                output_variable_index,
                variable_count: variables.len(),
            });
        }

        for window in variables.windows(2)
        {
            if window[0].last_part() >= window[1].first_part()
            {
                return Err(EquivalenceError::OverlappingVariables {
                    left_last_part: window[0].last_part(),
                    right_first_part: window[1].first_part(),
                });
            }
        }

        Ok(Self {
            variables,
            output_variable_index,
        })
    }

    pub fn variables(&self) -> &[Variable]
    {
        &self.variables
    }

    pub fn output_variable(&self) -> Variable
    {
        self.variables[self.output_variable_index]
    }

    pub fn input_variables(&self) -> impl Iterator<Item = Variable> + '_
    {
        self.variables
            .iter()
            .enumerate()
            .filter_map(|(index, variable)| {
                if index == self.output_variable_index
                {
                    None
                }
                else
                {
                    Some(*variable)
                }
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cube
{
    parts: BTreeSet<usize>,
}

impl Cube
{
    pub fn from_parts(parts: impl IntoIterator<Item = usize>) -> Self
    {
        Self {
            parts: parts.into_iter().collect(),
        }
    }

    pub fn contains(&self, part: usize) -> bool
    {
        self.parts.contains(&part)
    }

    fn contains_assignment(&self, assignment: &[usize]) -> bool
    {
        assignment.iter().all(|part| self.contains(*part))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cover
{
    cubes: Vec<Cube>,
}

impl Cover
{
    pub fn new(cubes: impl IntoIterator<Item = Cube>) -> Self
    {
        Self {
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn empty() -> Self
    {
        Self { cubes: Vec::new() }
    }

    pub fn cubes(&self) -> &[Cube]
    {
        &self.cubes
    }

    fn evaluates(&self, assignment: &[usize]) -> bool
    {
        self.cubes
            .iter()
            .any(|cube| cube.contains_assignment(assignment))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pla
{
    off_set: Cover,
    structure: CubeStructure,
    output_labels: Vec<String>,
}

impl Pla
{
    pub fn new(
        off_set: Cover,
        structure: CubeStructure,
        output_labels: impl IntoIterator<Item = String>,
    ) -> EquivalenceResult<Self>
    {
        let output_labels = output_labels.into_iter().collect::<Vec<_>>();
        let output_count = structure.output_variable().part_count();
        if output_labels.len() != output_count
        {
            return Err(EquivalenceError::OutputLabelCount {
                labels: output_labels.len(),
                outputs: output_count,
            });
        }

        Ok(Self {
            off_set,
            structure,
            output_labels,
        })
    }

    pub fn off_set(&self) -> &Cover
    {
        &self.off_set
    }

    pub fn structure(&self) -> &CubeStructure
    {
        &self.structure
    }

    pub fn output_labels(&self) -> &[String]
    {
        &self.output_labels
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputPolarity
{
    Positive,
    Negative,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputEquivalence
{
    left_output: usize,
    left_polarity: OutputPolarity,
    right_output: usize,
    right_polarity: OutputPolarity,
    left_label: String,
    right_label: String,
}

impl OutputEquivalence
{
    pub fn new(
        left_output: usize,
        left_polarity: OutputPolarity,
        right_output: usize,
        right_polarity: OutputPolarity,
        left_label: String,
        right_label: String,
    ) -> Self
    {
        Self {
            left_output,
            left_polarity,
            right_output,
            right_polarity,
            left_label,
            right_label,
        }
    }

    pub fn left_output(&self) -> usize
    {
        self.left_output
    }

    pub fn left_polarity(&self) -> OutputPolarity
    {
        self.left_polarity
    }

    pub fn right_output(&self) -> usize
    {
        self.right_output
    }

    pub fn right_polarity(&self) -> OutputPolarity
    {
        self.right_polarity
    }

    pub fn left_label(&self) -> &str
    {
        &self.left_label
    }

    pub fn right_label(&self) -> &str
    {
        &self.right_label
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquivalenceReport
{
    equivalences: Vec<OutputEquivalence>,
}

impl EquivalenceReport
{
    pub fn new(equivalences: Vec<OutputEquivalence>) -> Self
    {
        Self { equivalences }
    }

    pub fn has_equivalent_outputs(&self) -> bool
    {
        !self.equivalences.is_empty()
    }

    pub fn equivalences(&self) -> &[OutputEquivalence]
    {
        &self.equivalences
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EquivalenceError
{
    OutputVariableOutOfRange {
        output_variable_index: usize,
        variable_count: usize,
    },
    OverlappingVariables {
        left_last_part: usize,
        right_first_part: usize,
    },
    OutputLabelCount {
        labels: usize,
        outputs: usize,
    },
}

impl fmt::Display for EquivalenceError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::OutputVariableOutOfRange {
                output_variable_index,
                variable_count,
            } => write!(
                formatter,
                "output variable index {output_variable_index} is outside 0..{variable_count}"
            ),
            Self::OverlappingVariables {
                left_last_part,
                right_first_part,
            } => write!(
                formatter,
                "variable part ranges overlap at {left_last_part} and {right_first_part}"
            ),
            Self::OutputLabelCount { labels, outputs } => {
                write!(formatter, "{labels} labels supplied for {outputs} outputs")
            }
        }
    }
}

impl Error for EquivalenceError {}

pub type EquivalenceResult<T> = Result<T, EquivalenceError>;

pub fn find_equivalent_outputs(pla: &Pla) -> EquivalenceReport
{
    let output_count = pla.structure().output_variable().part_count();
    let mut equivalences = Vec::new();

    for left in 0..output_count.saturating_sub(1)
    {
        for right in (left + 1)..output_count
        {
            if check_output_equivalence(pla, left, right, false)
            {
                equivalences.push(output_equivalence(
                    pla,
                    left,
                    OutputPolarity::Positive,
                    right,
                    OutputPolarity::Positive,
                ));
            }
            else if check_output_equivalence(pla, left, right, true)
            {
                equivalences.push(output_equivalence(
                    pla,
                    left,
                    OutputPolarity::Positive,
                    right,
                    OutputPolarity::Negative,
                ));
            }
        }
    }

    EquivalenceReport::new(equivalences)
}

pub fn check_equiv(left: &Cover, right: &Cover, structure: &CubeStructure) -> bool
{
    let mut assignment = Vec::new();
    every_input_assignment(structure, &mut assignment, |assignment| {
        left.evaluates(assignment) == right.evaluates(assignment)
    })
}

fn check_output_equivalence(
    pla: &Pla,
    left_output: usize,
    right_output: usize,
    inverted: bool,
) -> bool
{
    let mut assignment = Vec::new();
    every_input_assignment(pla.structure(), &mut assignment, |assignment| {
        let left = output_value(pla, left_output, assignment);
        let right = output_value(pla, right_output, assignment);

        if inverted
        {
            left != right
        }
        else
        {
            left == right
        }
    })
}

fn output_value(pla: &Pla, output_index: usize, input_assignment: &[usize]) -> bool
{
    let output_part = pla.structure().output_variable().first_part() + output_index;
    let mut assignment = Vec::with_capacity(input_assignment.len() + 1);
    assignment.extend_from_slice(input_assignment);
    assignment.push(output_part);

    !pla.off_set().evaluates(&assignment)
}

fn output_equivalence(
    pla: &Pla,
    left_output: usize,
    left_polarity: OutputPolarity,
    right_output: usize,
    right_polarity: OutputPolarity,
) -> OutputEquivalence
{
    OutputEquivalence::new(
        left_output,
        left_polarity,
        right_output,
        right_polarity,
        pla.output_labels()[left_output].clone(),
        pla.output_labels()[right_output].clone(),
    )
}

fn every_input_assignment<P>(
    structure: &CubeStructure,
    assignment: &mut Vec<usize>,
    mut predicate: P,
) -> bool
where
    P: FnMut(&[usize]) -> bool,
{
    every_input_assignment_inner(structure, 0, assignment, &mut predicate)
}

fn every_input_assignment_inner<P>(
    structure: &CubeStructure,
    variable_index: usize,
    assignment: &mut Vec<usize>,
    predicate: &mut P,
) -> bool
where
    P: FnMut(&[usize]) -> bool,
{
    if variable_index == structure.variables().len()
    {
        return predicate(assignment);
    }

    if variable_index == structure.output_variable_index
    {
        return every_input_assignment_inner(
            structure,
            variable_index + 1,
            assignment,
            predicate,
        );
    }

    for part in structure.variables()[variable_index].parts()
    {
        assignment.push(part);
        if !every_input_assignment_inner(structure, variable_index + 1, assignment, predicate)
        {
            assignment.pop();
            return false;
        }

        assignment.pop();
    }

    true
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn structure() -> CubeStructure
    {
        CubeStructure::new(
            [
                Variable::new(0, 1),
                Variable::new(2, 3),
                Variable::new(4, 6),
            ],
            2,
        )
        .unwrap()
    }

    fn cube(parts: &[usize]) -> Cube
    {
        Cube::from_parts(parts.iter().copied())
    }

    fn pla(off_cubes: &[&[usize]]) -> Pla
    {
        Pla::new(
            Cover::new(off_cubes.iter().map(|parts| cube(parts))),
            structure(),
            ["f".to_string(), "g".to_string(), "h".to_string()],
        )
        .unwrap()
    }

    #[test]
    fn finds_outputs_with_matching_positive_functions()
    {
        let pla = pla(&[
            &[0, 2, 4],
            &[1, 2, 4],
            &[0, 2, 5],
            &[1, 2, 5],
            &[0, 2, 6],
            &[1, 3, 6],
        ]);

        let report = find_equivalent_outputs(&pla);

        assert_eq!(
            report.equivalences(),
            &[OutputEquivalence::new(
                0,
                OutputPolarity::Positive,
                1,
                OutputPolarity::Positive,
                "f".to_string(),
                "g".to_string(),
            )]
        );
    }

    #[test]
    fn finds_outputs_with_inverted_functions()
    {
        let pla = pla(&[
            &[0, 2, 4],
            &[1, 2, 4],
            &[0, 3, 5],
            &[1, 3, 5],
            &[0, 2, 6],
            &[0, 3, 6],
        ]);

        let report = find_equivalent_outputs(&pla);

        assert_eq!(
            report.equivalences(),
            &[OutputEquivalence::new(
                0,
                OutputPolarity::Positive,
                1,
                OutputPolarity::Negative,
                "f".to_string(),
                "g".to_string(),
            )]
        );
    }

    #[test]
    fn reports_no_equivalence_when_all_output_functions_differ()
    {
        let pla = pla(&[
            &[0, 2, 4],
            &[1, 2, 4],
            &[0, 2, 5],
            &[0, 3, 5],
            &[0, 2, 6],
            &[1, 3, 5],
            &[1, 3, 6],
        ]);

        let report = find_equivalent_outputs(&pla);

        assert!(!report.has_equivalent_outputs());
        assert!(report.equivalences().is_empty());
    }

    #[test]
    fn check_equiv_compares_covers_over_input_assignments()
    {
        let structure = structure();
        let left = Cover::new([cube(&[0, 2]), cube(&[1, 3])]);
        let right = Cover::new([cube(&[0, 2]), cube(&[1, 3])]);

        assert!(check_equiv(&left, &right, &structure));
    }

    #[test]
    fn check_equiv_rejects_missing_input_assignment()
    {
        let structure = structure();
        let left = Cover::new([cube(&[0, 2]), cube(&[1, 3])]);
        let right = Cover::new([cube(&[0, 2])]);

        assert!(!check_equiv(&left, &right, &structure));
    }

    #[test]
    fn rejects_mismatched_output_labels()
    {
        let error = Pla::new(Cover::empty(), structure(), ["f".to_string()])
            .expect_err("label count should be validated");

        assert_eq!(
            error,
            EquivalenceError::OutputLabelCount {
                labels: 1,
                outputs: 3,
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("equiv.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }
}
