use std::collections::BTreeMap;
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

    pub const fn part_count(self) -> usize
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
    binary_variable_count: usize,
    output_variable: Option<usize>,
}

impl CubeStructure
{
    pub fn new(
        variables: impl IntoIterator<Item = Variable>,
        binary_variable_count: usize,
        output_variable: Option<usize>,
    ) -> CvroutResult<Self>
    {
        let variables = variables.into_iter().collect::<Vec<_>>();
        if binary_variable_count > variables.len()
        {
            return Err(CvroutError::InvalidBinaryVariableCount {
                binary_variable_count,
                variable_count: variables.len(),
            });
        }

        for (index, variable) in variables.iter().take(binary_variable_count).enumerate()
        {
            if variable.part_count() != 2
            {
                return Err(CvroutError::InvalidBinaryVariable {
                    variable: index,
                    part_count: variable.part_count(),
                });
            }
        }

        for window in variables.windows(2)
        {
            if window[0].last_part >= window[1].first_part
            {
                return Err(CvroutError::OverlappingVariables);
            }
        }

        if let Some(output_variable) = output_variable
        {
            if output_variable >= variables.len()
            {
                return Err(CvroutError::InvalidOutputVariable {
                    output_variable,
                    variable_count: variables.len(),
                });
            }
        }

        Ok(Self {
            variables,
            binary_variable_count,
            output_variable,
        })
    }

    pub fn variables(&self) -> &[Variable]
    {
        &self.variables
    }

    pub fn variable(&self, index: usize) -> Option<Variable>
    {
        self.variables.get(index).copied()
    }

    pub fn variable_count(&self) -> usize
    {
        self.variables.len()
    }

    pub fn binary_variable_count(&self) -> usize
    {
        self.binary_variable_count
    }

    pub fn multiple_valued_variable_count(&self) -> usize
    {
        self.variables.len() - self.binary_variable_count
    }

    pub fn output_variable(&self) -> Option<usize>
    {
        self.output_variable
    }

    fn output_parts(&self) -> Option<Variable>
    {
        self.output_variable.and_then(|index| self.variable(index))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoCube
{
    parts: BTreeSet<usize>,
}

impl EspressoCube
{
    pub fn empty() -> Self
    {
        Self {
            parts: BTreeSet::new(),
        }
    }

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

    pub fn parts(&self) -> impl Iterator<Item = usize> + '_
    {
        self.parts.iter().copied()
    }

    fn contains_all_parts(&self, variable: Variable) -> bool
    {
        variable.parts().all(|part| self.contains(part))
    }

    fn binary_value(&self, variable: Variable) -> BinaryValue
    {
        let zero = self.contains(variable.first_part());
        let one = self.contains(variable.last_part());
        match (zero, one)
        {
            (false, false) => BinaryValue::Missing,
            (true, false) => BinaryValue::Zero,
            (false, true) => BinaryValue::One,
            (true, true) => BinaryValue::DontCare,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoCover
{
    cubes: Vec<EspressoCube>,
}

impl EspressoCover
{
    pub fn new(cubes: impl IntoIterator<Item = EspressoCube>) -> Self
    {
        Self {
            cubes: cubes.into_iter().collect(),
        }
    }

    pub fn empty() -> Self
    {
        Self {
            cubes: Vec::new(),
        }
    }

    pub fn len(&self) -> usize
    {
        self.cubes.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.cubes.is_empty()
    }

    pub fn cubes(&self) -> &[EspressoCube]
    {
        &self.cubes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspressoPla
{
    structure: CubeStructure,
    on_set: EspressoCover,
    dont_care_set: EspressoCover,
    off_set: EspressoCover,
    labels: Vec<Option<String>>,
    phase: Option<EspressoCube>,
}

impl EspressoPla
{
    pub fn new(
        structure: CubeStructure,
        on_set: EspressoCover,
        dont_care_set: EspressoCover,
        off_set: EspressoCover,
        labels: Vec<Option<String>>,
        phase: Option<EspressoCube>,
    ) -> CvroutResult<Self>
    {
        let required_labels = structure
            .variables()
            .last()
            .map(|variable| variable.last_part() + 1)
            .unwrap_or(0);
        if !labels.is_empty() && labels.len() < required_labels
        {
            return Err(CvroutError::LabelCount {
                labels: labels.len(),
                required: required_labels,
            });
        }

        Ok(Self {
            structure,
            on_set,
            dont_care_set,
            off_set,
            labels,
            phase,
        })
    }

    pub fn structure(&self) -> &CubeStructure
    {
        &self.structure
    }

    pub fn on_set(&self) -> &EspressoCover
    {
        &self.on_set
    }

    pub fn dont_care_set(&self) -> &EspressoCover
    {
        &self.dont_care_set
    }

    pub fn off_set(&self) -> &EspressoCover
    {
        &self.off_set
    }

    pub fn labels(&self) -> &[Option<String>]
    {
        &self.labels
    }

    pub fn phase(&self) -> Option<&EspressoCube>
    {
        self.phase.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputSelection
{
    pub on_set: bool,
    pub dont_care_set: bool,
    pub off_set: bool,
}

impl OutputSelection
{
    pub const fn on() -> Self
    {
        Self {
            on_set: true,
            dont_care_set: false,
            off_set: false,
        }
    }

    pub const fn on_dont_care_off() -> Self
    {
        Self {
            on_set: true,
            dont_care_set: true,
            off_set: true,
        }
    }

    fn is_on_only(self) -> bool
    {
        self.on_set && !self.dont_care_set && !self.off_set
    }

    fn type_letters(self) -> String
    {
        let mut result = String::new();
        if self.on_set
        {
            result.push('f');
        }
        if self.dont_care_set
        {
            result.push('d');
        }
        if self.off_set
        {
            result.push('r');
        }

        result
    }

    fn cube_count(self, pla: &EspressoPla) -> usize
    {
        let mut result = 0;
        if self.on_set
        {
            result += pla.on_set().len();
        }
        if self.dont_care_set
        {
            result += pla.dont_care_set().len();
        }
        if self.off_set
        {
            result += pla.off_set().len();
        }

        result
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat
{
    Pla(OutputSelection),
    Pleasure,
    Equations,
    Kiss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintFormat
{
    Numeric,
    Symbolic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CvroutError
{
    InvalidBinaryVariableCount {
        binary_variable_count: usize,
        variable_count: usize,
    },
    InvalidBinaryVariable {
        variable: usize,
        part_count: usize,
    },
    InvalidOutputVariable {
        output_variable: usize,
        variable_count: usize,
    },
    OverlappingVariables,
    LabelCount {
        labels: usize,
        required: usize,
    },
    MissingOutputVariable(&'static str),
    NonBinaryValuedEquations,
    MultipleSymbolicParts {
        variable: usize,
    },
}

impl fmt::Display for CvroutError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::InvalidBinaryVariableCount {
                binary_variable_count,
                variable_count,
            } => write!(
                formatter,
                "{binary_variable_count} binary variables exceeds {variable_count} variables"
            ),
            Self::InvalidBinaryVariable {
                variable,
                part_count,
            } => write!(
                formatter,
                "binary variable {variable} has {part_count} parts instead of 2"
            ),
            Self::InvalidOutputVariable {
                output_variable,
                variable_count,
            } => write!(
                formatter,
                "output variable {output_variable} is outside 0..{variable_count}"
            ),
            Self::OverlappingVariables => write!(formatter, "cube variables overlap"),
            Self::LabelCount {
                labels,
                required,
            } => write!(formatter, "{labels} labels supplied, {required} required"),
            Self::MissingOutputVariable(mode) => {
                write!(formatter, "{mode} output requires an output variable")
            }
            Self::NonBinaryValuedEquations => {
                write!(formatter, "equation output requires a binary-valued function")
            }
            Self::MultipleSymbolicParts {
                variable,
            } => write!(formatter, "symbolic variable {variable} contains multiple parts"),
        }
    }
}

impl Error for CvroutError {}

pub type CvroutResult<T> = Result<T, CvroutError>;

pub fn format_pla(
    pla: &EspressoPla,
    format: OutputFormat,
    constraints: impl IntoIterator<Item = ConstraintFormat>,
) -> CvroutResult<String>
{
    let mut result = String::new();
    for constraint in constraints
    {
        result.push_str(&format_symbolic_constraints(pla, constraint)?);
    }

    match format
    {
        OutputFormat::Pla(selection) =>
        {
            result.push_str(&format_header(pla, selection)?);
            result.push_str(&format!(".p {}\n", selection.cube_count(pla)));
            if selection.is_on_only()
            {
                append_cover(&mut result, pla.structure(), pla.on_set(), "01");
                result.push_str(".e\n");
            }
            else
            {
                if selection.on_set
                {
                    append_cover(&mut result, pla.structure(), pla.on_set(), "~1");
                }
                if selection.dont_care_set
                {
                    append_cover(&mut result, pla.structure(), pla.dont_care_set(), "~2");
                }
                if selection.off_set
                {
                    append_cover(&mut result, pla.structure(), pla.off_set(), "~0");
                }
                result.push_str(".end\n");
            }
        }
        OutputFormat::Pleasure =>
        {
            result.push_str(&format_pleasure(pla)?);
        }
        OutputFormat::Equations =>
        {
            result.push_str(&format_equations(pla)?);
        }
        OutputFormat::Kiss =>
        {
            result.push_str(&format_kiss(pla)?);
        }
    }

    Ok(result)
}

pub fn format_header(pla: &EspressoPla, selection: OutputSelection) -> CvroutResult<String>
{
    let structure = pla.structure();
    let mut result = String::new();
    if !selection.is_on_only()
    {
        result.push_str(".type ");
        result.push_str(&selection.type_letters());
        result.push('\n');
    }

    if structure.multiple_valued_variable_count() <= 1
    {
        result.push_str(&format!(".i {}\n", structure.binary_variable_count()));
        if let Some(output_variable) = structure.output_variable()
        {
            let output = structure.variable(output_variable).expect("valid output variable");
            result.push_str(&format!(".o {}\n", output.part_count()));
        }
    }
    else
    {
        result.push_str(&format!(
            ".mv {} {}",
            structure.variable_count(),
            structure.binary_variable_count()
        ));
        for variable in structure.variables().iter().skip(structure.binary_variable_count())
        {
            result.push_str(&format!(" {}", variable.part_count()));
        }
        result.push('\n');
    }

    append_binary_labels(&mut result, pla);
    append_output_labels(&mut result, pla);
    append_multiple_valued_labels(&mut result, pla);
    append_phase(&mut result, pla)?;

    Ok(result)
}

pub fn format_cube(structure: &CubeStructure, cube: &EspressoCube, output_map: &str) -> String
{
    let mut result = String::new();
    append_cube(&mut result, structure, cube, output_map);
    result
}

pub fn format_expanded_cube(
    structure: &CubeStructure,
    cube: &EspressoCube,
    phase: Option<&EspressoCube>,
) -> String
{
    let mut result = String::new();
    for variable in structure.variables().iter().take(structure.binary_variable_count())
    {
        for part in variable.parts()
        {
            result.push(if cube.contains(part) { '1' } else { '~' });
        }
    }

    for (index, variable) in structure.variables().iter().enumerate().skip(structure.binary_variable_count())
    {
        if Some(index) == structure.output_variable()
        {
            continue;
        }

        for part in variable.parts()
        {
            result.push(if cube.contains(part) { '~' } else { '1' });
        }
    }

    if let Some(output_variable) = structure.output_parts()
    {
        result.push(' ');
        for part in output_variable.parts()
        {
            let output_map = if phase.is_none_or(|phase| phase.contains(part))
            {
                "~1"
            }
            else
            {
                "~0"
            };
            result.push(mapped_char(output_map, cube.contains(part)));
        }
    }

    result
}

pub fn format_pleasure(pla: &EspressoPla) -> CvroutResult<String>
{
    let labels = make_labels(pla);
    let mut result = String::new();
    result.push_str(".option unmerged\n");
    append_pleasure_labels(&mut result, pla.structure(), &labels);
    result.push_str("\n.group");
    append_pleasure_groups(&mut result, pla.structure(), &labels);
    result.push_str(&format!("\n.p {}\n", pla.on_set().len()));
    for cube in pla.on_set().cubes()
    {
        result.push_str(&format_expanded_cube(pla.structure(), cube, pla.phase()));
        result.push('\n');
    }
    result.push_str(".end\n");

    Ok(result)
}

pub fn format_equations(pla: &EspressoPla) -> CvroutResult<String>
{
    let structure = pla.structure();
    let Some(output_variable_index) = structure.output_variable()
    else
    {
        return Err(CvroutError::MissingOutputVariable("equation"));
    };
    if structure.multiple_valued_variable_count() != 1
    {
        return Err(CvroutError::NonBinaryValuedEquations);
    }

    let output_variable = structure
        .variable(output_variable_index)
        .expect("valid output variable");
    let labels = make_labels(pla);
    let mut result = String::new();
    for output_part in output_variable.parts()
    {
        let output_label = &labels[output_part];
        result.push_str(output_label);
        result.push_str(" = ");
        let mut first_or = true;
        let mut column = output_label.len() + 3;

        for cube in pla.on_set().cubes()
        {
            if !cube.contains(output_part)
            {
                continue;
            }

            if first_or
            {
                result.push('(');
                column += 1;
            }
            else
            {
                result.push_str(" | (");
                column += 4;
            }
            first_or = false;

            let mut first_and = true;
            for variable_index in 0..structure.binary_variable_count()
            {
                let variable = structure.variable(variable_index).expect("valid variable");
                let value = cube.binary_value(variable);
                if value == BinaryValue::DontCare
                {
                    continue;
                }

                let input_label = &labels[variable.last_part()];
                if column + input_label.len() > 72
                {
                    result.push_str("\n    ");
                    column = 4;
                }
                if !first_and
                {
                    result.push('&');
                    column += 1;
                }
                first_and = false;
                if value == BinaryValue::Zero
                {
                    result.push('!');
                    column += 1;
                }
                result.push_str(input_label);
                column += input_label.len();
            }

            result.push(')');
            column += 1;
        }

        result.push_str(";\n\n");
    }

    Ok(result)
}

pub fn format_kiss(pla: &EspressoPla) -> CvroutResult<String>
{
    let mut result = String::new();
    for cube in pla.on_set().cubes()
    {
        result.push_str(&format_kiss_cube(pla, cube, "~1")?);
        result.push('\n');
    }
    for cube in pla.dont_care_set().cubes()
    {
        result.push_str(&format_kiss_cube(pla, cube, "~2")?);
        result.push('\n');
    }

    Ok(result)
}

pub fn format_kiss_cube(
    pla: &EspressoPla,
    cube: &EspressoCube,
    output_map: &str,
) -> CvroutResult<String>
{
    let structure = pla.structure();
    let labels = make_labels(pla);
    let mut result = String::new();
    for variable in structure.variables().iter().take(structure.binary_variable_count())
    {
        result.push(binary_char(cube.binary_value(*variable)));
    }

    for (variable_index, variable) in structure.variables().iter().enumerate().skip(structure.binary_variable_count())
    {
        if Some(variable_index) == structure.output_variable()
        {
            continue;
        }

        result.push(' ');
        if cube.contains_all_parts(*variable)
        {
            result.push('-');
            continue;
        }

        let mut selected_part = None;
        for part in variable.parts()
        {
            if cube.contains(part)
            {
                if selected_part.is_some()
                {
                    return Err(CvroutError::MultipleSymbolicParts {
                        variable: variable_index,
                    });
                }
                selected_part = Some(part);
            }
        }

        match selected_part
        {
            Some(part) => result.push_str(&labels[part]),
            None => result.push('~'),
        }
    }

    if let Some(output_variable) = structure.output_parts()
    {
        result.push(' ');
        for part in output_variable.parts()
        {
            result.push(mapped_char(output_map, cube.contains(part)));
        }
    }

    Ok(result)
}

pub fn format_symbolic_constraints(
    pla: &EspressoPla,
    format: ConstraintFormat,
) -> CvroutResult<String>
{
    let structure = pla.structure();
    if structure.multiple_valued_variable_count() <= 1
    {
        return Ok(String::new());
    }

    let labels = make_labels(pla);
    let mut result = String::new();
    for (variable_index, variable) in structure.variables().iter().enumerate().skip(structure.binary_variable_count())
    {
        if Some(variable_index) == structure.output_variable()
        {
            continue;
        }

        let mut unconstrained_weight = 0;
        let mut weights = BTreeMap::<Vec<usize>, usize>::new();
        for cube in pla.on_set().cubes()
        {
            let projected = variable
                .parts()
                .enumerate()
                .filter_map(|(offset, part)| cube.contains(part).then_some(offset))
                .collect::<Vec<_>>();
            if projected.len() == 1 || projected.len() == variable.part_count()
            {
                unconstrained_weight += 1;
                continue;
            }

            *weights.entry(projected).or_default() += 1;
        }

        match format
        {
            ConstraintFormat::Numeric =>
            {
                result.push_str(&format!(
                    "# Symbolic constraints for variable {variable_index} (Numeric form)\n"
                ));
                result.push_str(&format!("# unconstrained weight = {unconstrained_weight}\n"));
                result.push_str(&format!("num_codes={}\n", variable.part_count()));
                for (projection, weight) in weights
                {
                    result.push_str(&format!("weight={weight}:"));
                    for part in projection
                    {
                        result.push_str(&format!(" {part}"));
                    }
                    result.push('\n');
                }
            }
            ConstraintFormat::Symbolic =>
            {
                result.push_str(&format!(
                    "# Symbolic constraints for variable {variable_index} (Symbolic form)\n"
                ));
                for (projection, weight) in weights
                {
                    result.push_str(&format!("#   w={weight}: ("));
                    for offset in projection
                    {
                        result.push_str(&format!(" {}", labels[variable.first_part() + offset]));
                    }
                    result.push_str(" )\n");
                }
            }
        }
    }

    Ok(result)
}

fn append_cover(
    result: &mut String,
    structure: &CubeStructure,
    cover: &EspressoCover,
    output_map: &str,
)
{
    for cube in cover.cubes()
    {
        append_cube(result, structure, cube, output_map);
        result.push('\n');
    }
}

fn append_cube(
    result: &mut String,
    structure: &CubeStructure,
    cube: &EspressoCube,
    output_map: &str,
)
{
    for variable in structure.variables().iter().take(structure.binary_variable_count())
    {
        result.push(binary_char(cube.binary_value(*variable)));
    }

    for (index, variable) in structure.variables().iter().enumerate().skip(structure.binary_variable_count())
    {
        if Some(index) == structure.output_variable()
        {
            continue;
        }

        result.push(' ');
        for part in variable.parts()
        {
            result.push(if cube.contains(part) { '1' } else { '0' });
        }
    }

    if let Some(output_variable) = structure.output_parts()
    {
        result.push(' ');
        for part in output_variable.parts()
        {
            result.push(mapped_char(output_map, cube.contains(part)));
        }
    }
}

fn append_binary_labels(result: &mut String, pla: &EspressoPla)
{
    if pla.structure().binary_variable_count() == 0 || pla.labels().is_empty()
    {
        return;
    }

    let labels = make_labels(pla);
    result.push_str(".ilb");
    for variable in pla
        .structure()
        .variables()
        .iter()
        .take(pla.structure().binary_variable_count())
    {
        result.push(' ');
        result.push_str(&labels[variable.last_part()]);
    }
    result.push('\n');
}

fn append_output_labels(result: &mut String, pla: &EspressoPla)
{
    let Some(output_variable) = pla.structure().output_parts()
    else
    {
        return;
    };
    if pla.labels().is_empty()
    {
        return;
    }

    let labels = make_labels(pla);
    result.push_str(".ob");
    for part in output_variable.parts()
    {
        result.push(' ');
        result.push_str(&labels[part]);
    }
    result.push('\n');
}

fn append_multiple_valued_labels(result: &mut String, pla: &EspressoPla)
{
    if pla.labels().is_empty()
    {
        return;
    }

    let labels = make_labels(pla);
    for (variable_index, variable) in pla
        .structure()
        .variables()
        .iter()
        .enumerate()
        .skip(pla.structure().binary_variable_count())
    {
        if Some(variable_index) == pla.structure().output_variable()
        {
            continue;
        }

        result.push_str(&format!(".label var={variable_index}"));
        for part in variable.parts()
        {
            result.push(' ');
            result.push_str(&labels[part]);
        }
        result.push('\n');
    }
}

fn append_phase(result: &mut String, pla: &EspressoPla) -> CvroutResult<()>
{
    let Some(phase) = pla.phase()
    else
    {
        return Ok(());
    };
    let Some(output_variable) = pla.structure().output_parts()
    else
    {
        return Err(CvroutError::MissingOutputVariable("phase"));
    };

    result.push_str("#.phase ");
    for part in output_variable.parts()
    {
        result.push(if phase.contains(part) { '1' } else { '0' });
    }
    result.push('\n');

    Ok(())
}

fn append_pleasure_labels(result: &mut String, structure: &CubeStructure, labels: &[String])
{
    result.push_str(".label");
    let mut column = 6;
    for variable in structure.variables()
    {
        for part in variable.parts()
        {
            append_wrapped_label(result, &labels[part], &mut column);
        }
    }
}

fn append_pleasure_groups(result: &mut String, structure: &CubeStructure, labels: &[String])
{
    let output_variable = structure.output_variable();
    let mut column = 6;
    for (variable_index, variable) in structure.variables().iter().enumerate()
    {
        if Some(variable_index) == output_variable
        {
            continue;
        }

        result.push_str(" (");
        column += 2;
        for part in variable.parts()
        {
            append_wrapped_label(result, &labels[part], &mut column);
        }
        result.push(')');
        column += 1;
    }
}

fn append_wrapped_label(result: &mut String, label: &str, column: &mut usize)
{
    if *column + label.len() > 75
    {
        result.push_str(" \\\n");
        *column = 0;
    }
    else
    {
        result.push(' ');
        *column += 1;
    }
    result.push_str(label);
    *column += label.len();
}

fn make_labels(pla: &EspressoPla) -> Vec<String>
{
    let required_labels = pla
        .structure()
        .variables()
        .last()
        .map(|variable| variable.last_part() + 1)
        .unwrap_or(0);
    let mut labels = Vec::with_capacity(required_labels);
    for index in 0..required_labels
    {
        labels.push(
            pla.labels()
                .get(index)
                .and_then(|label| label.clone())
                .unwrap_or_else(|| default_label(pla.structure(), index)),
        );
    }

    labels
}

fn default_label(structure: &CubeStructure, part: usize) -> String
{
    for (variable_index, variable) in structure.variables().iter().enumerate()
    {
        if !variable.parts().any(|candidate| candidate == part)
        {
            continue;
        }

        let offset = part - variable.first_part();
        if variable_index < structure.binary_variable_count()
        {
            if offset == 0
            {
                return format!("v{variable_index}.bar");
            }

            return format!("v{variable_index}");
        }

        return format!("v{variable_index}.{offset}");
    }

    format!("p{part}")
}

fn binary_char(value: BinaryValue) -> char
{
    match value
    {
        BinaryValue::Missing => '?',
        BinaryValue::Zero => '0',
        BinaryValue::One => '1',
        BinaryValue::DontCare => '-',
    }
}

fn mapped_char(map: &str, present: bool) -> char
{
    let index = usize::from(present);
    map.chars().nth(index).unwrap_or('?')
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BinaryValue
{
    Missing,
    Zero,
    One,
    DontCare,
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
                Variable::new(7, 8),
            ],
            2,
            Some(3),
        )
        .unwrap()
    }

    fn binary_structure() -> CubeStructure
    {
        CubeStructure::new(
            [
                Variable::new(0, 1),
                Variable::new(2, 3),
                Variable::new(4, 5),
            ],
            2,
            Some(2),
        )
        .unwrap()
    }

    fn cube(parts: &[usize]) -> EspressoCube
    {
        EspressoCube::from_parts(parts.iter().copied())
    }

    fn cover(cubes: &[&[usize]]) -> EspressoCover
    {
        EspressoCover::new(cubes.iter().map(|parts| cube(parts)))
    }

    fn pla() -> EspressoPla
    {
        let labels = [
            "a_bar", "a", "b_bar", "b", "idle", "run", "stop", "z0", "z1",
        ]
        .into_iter()
        .map(|label| Some(label.to_string()))
        .collect();

        EspressoPla::new(
            structure(),
            cover(&[&[1, 2, 4, 8], &[0, 3, 5, 8]]),
            cover(&[&[0, 1, 2, 3, 6, 7]]),
            cover(&[&[0, 2, 4, 7]]),
            labels,
            Some(cube(&[8])),
        )
        .unwrap()
    }

    #[test]
    fn format_cube_matches_espresso_binary_multiple_valued_and_output_maps()
    {
        let result = format_cube(&structure(), &cube(&[0, 1, 3, 4, 6, 8]), "~1");

        assert_eq!(result, "-1 101 ~1");
    }

    #[test]
    fn pla_output_writes_header_counts_covers_and_legacy_end_marker()
    {
        let text = format_pla(
            &pla(),
            OutputFormat::Pla(OutputSelection::on_dont_care_off()),
            [],
        )
        .unwrap();

        assert!(text.starts_with(".type fdr\n.mv 4 2 3 2\n.ilb a b\n.ob z0 z1\n"));
        assert!(text.contains(".label var=2 idle run stop\n"));
        assert!(text.contains("#.phase 01\n"));
        assert!(text.contains(".p 4\n"));
        assert!(text.contains("10 100 ~1\n"));
        assert!(text.contains("-- 001 2~\n"));
        assert!(text.ends_with(".end\n"));
    }

    #[test]
    fn on_only_pla_output_uses_binary_output_map_and_dot_e()
    {
        let text = format_pla(&pla(), OutputFormat::Pla(OutputSelection::on()), []).unwrap();

        assert!(text.contains(".p 2\n"));
        assert!(text.contains("10 100 01\n"));
        assert!(text.ends_with(".e\n"));
        assert!(!text.contains(".type"));
    }

    #[test]
    fn pleasure_output_expands_binary_and_symbolic_columns()
    {
        let text = format_pleasure(&pla()).unwrap();

        assert!(text.starts_with(".option unmerged\n.label"));
        assert!(text.contains("\n.group ( a_bar a) ( b_bar b) ( idle run stop)\n.p 2\n"));
        assert!(text.contains("~11~~11 ~1\n"));
        assert!(text.ends_with(".end\n"));
    }

    #[test]
    fn equations_output_prints_one_expression_per_output()
    {
        let labels = vec![
            Some("a_bar".to_string()),
            Some("a".to_string()),
            Some("b_bar".to_string()),
            Some("b".to_string()),
            Some("z0".to_string()),
            Some("z1".to_string()),
        ];
        let pla = EspressoPla::new(
            binary_structure(),
            cover(&[&[1, 2, 4], &[0, 3, 5]]),
            EspressoCover::empty(),
            EspressoCover::empty(),
            labels,
            None,
        )
        .unwrap();

        let text = format_equations(&pla).unwrap();

        assert_eq!(text, "z0 = (a&!b);\n\nz1 = (!a&b);\n\n");
    }

    #[test]
    fn kiss_output_prints_symbolic_labels_and_dont_care_rows()
    {
        let text = format_kiss(&pla()).unwrap();

        assert_eq!(text, "10 idle ~1\n01 run ~1\n-- stop 2~\n");
    }

    #[test]
    fn symbolic_constraints_group_duplicate_projected_sets()
    {
        let pla = EspressoPla::new(
            structure(),
            cover(&[&[1, 2, 4, 5, 8], &[0, 3, 4, 5, 8], &[1, 2, 6, 8]]),
            EspressoCover::empty(),
            EspressoCover::empty(),
            Vec::new(),
            None,
        )
        .unwrap();

        let text = format_symbolic_constraints(&pla, ConstraintFormat::Numeric).unwrap();

        assert!(text.contains("# Symbolic constraints for variable 2 (Numeric form)\n"));
        assert!(text.contains("# unconstrained weight = 1\n"));
        assert!(text.contains("num_codes=3\n"));
        assert!(text.contains("weight=2: 0 1\n"));
    }

    #[test]
    fn symbolic_constraints_can_print_labels()
    {
        let text = format_symbolic_constraints(&pla(), ConstraintFormat::Symbolic).unwrap();

        assert!(text.contains("# Symbolic constraints for variable 2 (Symbolic form)\n"));
        assert!(!text.contains("#   w=1: ( idle )\n"));
    }

    #[test]
    fn kiss_output_rejects_multiple_symbolic_parts()
    {
        let error = format_kiss_cube(&pla(), &cube(&[1, 2, 4, 5, 8]), "~1").unwrap_err();

        assert_eq!(error, CvroutError::MultipleSymbolicParts { variable: 2 });
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("cvrout.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("LogicFriday", "1-")));
    }
}
