//! Native Rust PLA input routines for Espresso-style cube covers.
//!
//! The legacy reader mixed file IO, process-global cube state, and direct
//! mutation of Espresso covers. This module keeps the same PLA syntax handling
//! on owned Rust values and leaves interop boundaries to higher-level facade
//! code.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlaType
{
    bits: u16,
}

impl PlaType
{
    pub const ON: Self = Self { bits: 0b001 };
    pub const DONT_CARE: Self = Self { bits: 0b010 };
    pub const OFF: Self = Self { bits: 0b100 };
    pub const ON_DC: Self = Self { bits: Self::ON.bits | Self::DONT_CARE.bits };
    pub const ON_OFF: Self = Self { bits: Self::ON.bits | Self::OFF.bits };
    pub const DC_OFF: Self = Self { bits: Self::DONT_CARE.bits | Self::OFF.bits };
    pub const ON_DC_OFF: Self =
        Self { bits: Self::ON.bits | Self::DONT_CARE.bits | Self::OFF.bits };

    pub const fn bits(self) -> u16
    {
        self.bits
    }

    pub const fn contains(self, other: Self) -> bool
    {
        (self.bits & other.bits) == other.bits
    }

    fn parse(value: &str) -> Option<Self>
    {
        match value
        {
            "f" => Some(Self::ON),
            "d" => Some(Self::DONT_CARE),
            "r" => Some(Self::OFF),
            "fd" => Some(Self::ON_DC),
            "fr" => Some(Self::ON_OFF),
            "dr" => Some(Self::DC_OFF),
            "fdr" => Some(Self::ON_DC_OFF),
            _ => None,
        }
    }
}

impl Default for PlaType
{
    fn default() -> Self
    {
        Self::ON_DC
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure
{
    num_binary_vars: usize,
    part_size: Vec<usize>,
    first_part: Vec<usize>,
    last_part: Vec<usize>,
    size: usize,
}

impl CubeStructure
{
    pub fn binary(input_count: usize, output_count: usize) -> CvrinResult<Self>
    {
        let mut part_size = vec![2; input_count];
        part_size.push(output_count);
        Self::new(input_count, part_size)
    }

    pub fn new(num_binary_vars: usize, part_size: Vec<usize>) -> CvrinResult<Self>
    {
        if num_binary_vars > part_size.len()
        {
            return Err(CvrinError::InvalidShape(
                "binary variable count exceeds variable count".to_string(),
            ));
        }

        if part_size.is_empty()
        {
            return Err(CvrinError::InvalidShape(
                "PLA must declare at least one variable".to_string(),
            ));
        }

        if let Some((variable, _)) = part_size
            .iter()
            .enumerate()
            .find(|(_, size)| **size == 0)
        {
            return Err(CvrinError::InvalidPartSize { variable, size: 0 });
        }

        let mut first_part = Vec::with_capacity(part_size.len());
        let mut last_part = Vec::with_capacity(part_size.len());
        let mut next_part = 0usize;
        for size in &part_size
        {
            first_part.push(next_part);
            next_part += *size;
            last_part.push(next_part - 1);
        }

        Ok(Self
        {
            num_binary_vars,
            part_size,
            first_part,
            last_part,
            size: next_part,
        })
    }

    pub fn num_vars(&self) -> usize
    {
        self.part_size.len()
    }

    pub fn num_binary_vars(&self) -> usize
    {
        self.num_binary_vars
    }

    pub fn output_var(&self) -> usize
    {
        self.num_vars() - 1
    }

    pub fn part_size(&self, variable: usize) -> Option<usize>
    {
        self.part_size.get(variable).copied()
    }

    pub fn first_part(&self, variable: usize) -> Option<usize>
    {
        self.first_part.get(variable).copied()
    }

    pub fn last_part(&self, variable: usize) -> Option<usize>
    {
        self.last_part.get(variable).copied()
    }

    pub fn size(&self) -> usize
    {
        self.size
    }

    fn variable_parts(&self, variable: usize) -> std::ops::RangeInclusive<usize>
    {
        self.first_part[variable]..=self.last_part[variable]
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Cube
{
    parts: BTreeSet<usize>,
}

impl Cube
{
    pub fn empty() -> Self
    {
        Self
        {
            parts: BTreeSet::new(),
        }
    }

    pub fn from_parts(parts: impl IntoIterator<Item = usize>) -> Self
    {
        Self
        {
            parts: parts.into_iter().collect(),
        }
    }

    pub fn parts(&self) -> &BTreeSet<usize>
    {
        &self.parts
    }

    pub fn contains(&self, part: usize) -> bool
    {
        self.parts.contains(&part)
    }

    pub fn insert(&mut self, part: usize)
    {
        self.parts.insert(part);
    }

    fn xor_variable(&self, structure: &CubeStructure, variable: usize) -> Self
    {
        let mask = structure
            .variable_parts(variable)
            .filter(|part| !self.contains(*part));
        Self::from_parts(mask)
    }
}

pub type Cover = Vec<Cube>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Pairing
{
    pub var1: Vec<usize>,
    pub var2: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicItem
{
    pub variable: usize,
    pub position: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicDeclaration
{
    pub items: Vec<SymbolicItem>,
    pub labels: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pla
{
    pub on_set: Cover,
    pub dont_care_set: Cover,
    pub off_set: Cover,
    pub filename: Option<String>,
    pub pla_type: PlaType,
    pub phase: Option<Cube>,
    pub pair: Option<Pairing>,
    pub labels: Vec<Option<String>>,
    pub symbolic: Vec<SymbolicDeclaration>,
    pub symbolic_output: Vec<SymbolicDeclaration>,
    pub structure: Option<CubeStructure>,
    pub kiss: bool,
}

impl Pla
{
    pub fn new(pla_type: PlaType) -> Self
    {
        Self
        {
            on_set: Vec::new(),
            dont_care_set: Vec::new(),
            off_set: Vec::new(),
            filename: None,
            pla_type,
            phase: None,
            pair: None,
            labels: Vec::new(),
            symbolic: Vec::new(),
            symbolic_output: Vec::new(),
            structure: None,
            kiss: false,
        }
    }

    pub fn allocate_labels(&mut self)
    {
        let count = self.structure.as_ref().map(CubeStructure::size).unwrap_or(0);
        self.labels.resize(count, None);
    }

    pub fn label_index(&self, word: &str) -> Option<(usize, usize)>
    {
        let structure = self.structure.as_ref()?;
        if self.labels.first().and_then(Option::as_ref).is_none()
        {
            let index = word.parse::<usize>().ok()?;
            return Some((index, index));
        }

        for variable in 0..structure.num_vars()
        {
            for position in 0..structure.part_size[variable]
            {
                let part = structure.first_part[variable] + position;
                if self.labels.get(part).and_then(Option::as_ref).is_some_and(|label| label == word)
                {
                    return Some((variable, position));
                }
            }
        }

        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CvrinError
{
    MissingShape { line: usize },
    InvalidShape(String),
    InvalidDirective { line: usize, directive: String },
    InvalidCount { line: usize, directive: &'static str, value: String },
    InvalidPartSize { variable: usize, size: usize },
    InvalidType { line: usize, value: String },
    InvalidCube { line: usize, reason: String },
    InvalidLabel { line: usize, label: String },
    WrongLabelCount { line: usize, directive: &'static str, expected: usize, actual: usize },
    DuplicateDirective { line: usize, directive: &'static str },
}

impl fmt::Display for CvrinError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::MissingShape { line } => write!(formatter, "PLA size is not declared before line {line}"),
            Self::InvalidShape(reason) => write!(formatter, "invalid PLA shape: {reason}"),
            Self::InvalidDirective { line, directive } =>
            {
                write!(formatter, "unsupported PLA directive {directive} on line {line}")
            }
            Self::InvalidCount { line, directive, value } =>
            {
                write!(formatter, "invalid {directive} count {value:?} on line {line}")
            }
            Self::InvalidPartSize { variable, size } =>
            {
                write!(formatter, "invalid part size {size} for variable {variable}")
            }
            Self::InvalidType { line, value } =>
            {
                write!(formatter, "unknown PLA type {value:?} on line {line}")
            }
            Self::InvalidCube { line, reason } =>
            {
                write!(formatter, "invalid cube on line {line}: {reason}")
            }
            Self::InvalidLabel { line, label } =>
            {
                write!(formatter, "unknown label {label:?} on line {line}")
            }
            Self::WrongLabelCount { line, directive, expected, actual } =>
            {
                write!(
                    formatter,
                    "{directive} on line {line} has {actual} labels but {expected} were expected"
                )
            }
            Self::DuplicateDirective { line, directive } =>
            {
                write!(formatter, "duplicate {directive} directive on line {line}")
            }
        }
    }
}

impl Error for CvrinError {}

pub type CvrinResult<T> = Result<T, CvrinError>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReadPlaOptions
{
    pub needs_dcset: bool,
    pub needs_offset: bool,
    pub pla_type: PlaType,
    pub positive_phase: bool,
}

pub fn new_pla(pla_type: PlaType) -> Pla
{
    Pla::new(pla_type)
}

pub fn parse_pla(input: &str, pla: &mut Pla) -> CvrinResult<()>
{
    let mut parser = PlaParser::new(pla);
    parser.parse(input)
}

pub fn read_pla(input: &str, options: ReadPlaOptions) -> CvrinResult<Option<Pla>>
{
    let mut pla = new_pla(options.pla_type);
    parse_pla(input, &mut pla)?;

    if pla.structure.is_none() && pla.on_set.is_empty() && pla.dont_care_set.is_empty() && pla.off_set.is_empty()
    {
        return Ok(None);
    }

    if options.positive_phase
    {
        std::mem::swap(&mut pla.on_set, &mut pla.off_set);
        if let Some(structure) = &pla.structure
        {
            let output_var = structure.output_var();
            let phase_parts = (0..structure.size())
                .filter(|part| !structure.variable_parts(output_var).contains(part));
            pla.phase = Some(Cube::from_parts(phase_parts));
        }
    }

    Ok(Some(pla))
}

pub fn read_symbolic(pla: &Pla, words: &[&str]) -> CvrinResult<SymbolicDeclaration>
{
    parse_symbolic_words(0, pla, words)
}

struct PlaParser<'a>
{
    pla: &'a mut Pla,
    saw_product: bool,
}

impl<'a> PlaParser<'a>
{
    fn new(pla: &'a mut Pla) -> Self
    {
        Self
        {
            pla,
            saw_product: false,
        }
    }

    fn parse(&mut self, input: &str) -> CvrinResult<()>
    {
        for (index, raw_line) in input.lines().enumerate()
        {
            let line_number = index + 1;
            let line = raw_line.trim();
            if line.is_empty()
            {
                continue;
            }

            if line.starts_with('#')
            {
                continue;
            }

            if line.starts_with('.')
            {
                if self.parse_directive(line_number, line)?
                {
                    return Ok(());
                }
            }
            else
            {
                self.parse_cube(line_number, line)?;
            }
        }

        Ok(())
    }

    fn parse_directive(&mut self, line: usize, text: &str) -> CvrinResult<bool>
    {
        let words = text.split_whitespace().collect::<Vec<_>>();
        let directive = words.first().copied().unwrap_or_default();
        let values = &words[1..];

        match directive
        {
            ".e" | ".end" => Ok(true),
            ".i" => self.parse_binary_input_count(line, values),
            ".o" => self.parse_binary_output_count(line, values),
            ".mv" => self.parse_mv(line, values),
            ".p" => Ok(false),
            ".kiss" =>
            {
                self.pla.kiss = true;
                Ok(false)
            }
            ".type" =>
            {
                let value = values.first().copied().unwrap_or_default();
                self.pla.pla_type = PlaType::parse(value)
                    .ok_or_else(|| CvrinError::InvalidType { line, value: value.to_string() })?;
                Ok(false)
            }
            ".ilb" => self.parse_input_labels(line, values),
            ".ob" => self.parse_output_labels(line, values),
            ".label" => self.parse_variable_labels(line, values),
            ".phase" => self.parse_phase(line, values),
            ".pair" => self.parse_pair(line, values),
            ".symbolic" =>
            {
                let symbolic = parse_symbolic_words(line, self.pla, values)?;
                self.pla.symbolic.push(symbolic);
                Ok(false)
            }
            ".symbolic-output" =>
            {
                let symbolic = parse_symbolic_words(line, self.pla, values)?;
                self.pla.symbolic_output.push(symbolic);
                Ok(false)
            }
            _ => Err(CvrinError::InvalidDirective { line, directive: directive.to_string() }),
        }
    }

    fn parse_binary_input_count(&mut self, line: usize, values: &[&str]) -> CvrinResult<bool>
    {
        if self.pla.structure.is_some()
        {
            return Err(CvrinError::DuplicateDirective { line, directive: ".i" });
        }

        let input_count = parse_count(line, ".i", values.first().copied())?;
        self.pla.structure = Some(CubeStructure::binary(input_count, 1)?);
        self.pla.allocate_labels();
        Ok(false)
    }

    fn parse_binary_output_count(&mut self, line: usize, values: &[&str]) -> CvrinResult<bool>
    {
        let output_count = parse_count(line, ".o", values.first().copied())?;
        let Some(structure) = self.pla.structure.as_ref() else
        {
            return Err(CvrinError::MissingShape { line });
        };

        if structure.num_vars() != structure.num_binary_vars() + 1
        {
            return Err(CvrinError::DuplicateDirective { line, directive: ".o" });
        }

        self.pla.structure = Some(CubeStructure::binary(structure.num_binary_vars(), output_count)?);
        self.pla.allocate_labels();
        Ok(false)
    }

    fn parse_mv(&mut self, line: usize, values: &[&str]) -> CvrinResult<bool>
    {
        if self.pla.structure.is_some()
        {
            return Err(CvrinError::DuplicateDirective { line, directive: ".mv" });
        }

        let num_vars = parse_count(line, ".mv", values.first().copied())?;
        let num_binary_vars = parse_count(line, ".mv", values.get(1).copied())?;
        if num_binary_vars > num_vars
        {
            return Err(CvrinError::InvalidShape(
                "variable count must be at least the binary variable count".to_string(),
            ));
        }

        let expected_sizes = num_vars - num_binary_vars;
        if values.len().saturating_sub(2) != expected_sizes
        {
            return Err(CvrinError::WrongLabelCount
            {
                line,
                directive: ".mv",
                expected: expected_sizes,
                actual: values.len().saturating_sub(2),
            });
        }

        let mut part_size = vec![2; num_binary_vars];
        for (offset, value) in values[2..].iter().enumerate()
        {
            let variable = num_binary_vars + offset;
            let size = parse_signed_count(line, ".mv", value)?;
            if size == 0
            {
                return Err(CvrinError::InvalidPartSize { variable, size });
            }

            part_size.push(size);
        }

        self.pla.structure = Some(CubeStructure::new(num_binary_vars, part_size)?);
        self.pla.allocate_labels();
        Ok(false)
    }

    fn parse_input_labels(&mut self, line: usize, values: &[&str]) -> CvrinResult<bool>
    {
        let structure = self.require_structure(line)?;
        let expected = structure.num_binary_vars();
        if values.len() != expected
        {
            return Err(CvrinError::WrongLabelCount { line, directive: ".ilb", expected, actual: values.len() });
        }

        for (variable, label) in values.iter().enumerate()
        {
            let first = structure.first_part[variable];
            self.pla.labels[first] = Some(format!("{label}.bar"));
            self.pla.labels[first + 1] = Some((*label).to_string());
        }

        Ok(false)
    }

    fn parse_output_labels(&mut self, line: usize, values: &[&str]) -> CvrinResult<bool>
    {
        let structure = self.require_structure(line)?;
        let output_var = structure.output_var();
        let expected = structure.part_size[output_var];
        if values.len() != expected
        {
            return Err(CvrinError::WrongLabelCount { line, directive: ".ob", expected, actual: values.len() });
        }

        for (offset, label) in values.iter().enumerate()
        {
            self.pla.labels[structure.first_part[output_var] + offset] = Some((*label).to_string());
        }

        Ok(false)
    }

    fn parse_variable_labels(&mut self, line: usize, values: &[&str]) -> CvrinResult<bool>
    {
        let structure = self.require_structure(line)?;
        let Some(var_value) = values.first().and_then(|value| value.strip_prefix("var=")) else
        {
            return Err(CvrinError::InvalidDirective { line, directive: ".label".to_string() });
        };

        let variable = parse_usize(line, ".label", var_value)?;
        let expected = *structure.part_size.get(variable).ok_or_else(||
        {
            CvrinError::InvalidPartSize { variable, size: 0 }
        })?;
        let labels = &values[1..];
        if labels.len() != expected
        {
            return Err(CvrinError::WrongLabelCount { line, directive: ".label", expected, actual: labels.len() });
        }

        for (offset, label) in labels.iter().enumerate()
        {
            self.pla.labels[structure.first_part[variable] + offset] = Some((*label).to_string());
        }

        Ok(false)
    }

    fn parse_phase(&mut self, line: usize, values: &[&str]) -> CvrinResult<bool>
    {
        let structure = self.require_structure(line)?;
        if self.pla.phase.is_some()
        {
            return Err(CvrinError::DuplicateDirective { line, directive: ".phase" });
        }

        let output_var = structure.output_var();
        let phase = values.join("");
        let expected = structure.part_size[output_var];
        if phase.chars().count() != expected
        {
            return Err(CvrinError::WrongLabelCount { line, directive: ".phase", expected, actual: phase.chars().count() });
        }

        let mut cube = Cube::from_parts(0..structure.size());
        for (offset, ch) in phase.chars().enumerate()
        {
            match ch
            {
                '0' =>
                {
                    cube.parts.remove(&(structure.first_part[output_var] + offset));
                }
                '1' => {}
                _ =>
                {
                    return Err(CvrinError::InvalidCube
                    {
                        line,
                        reason: "only 0 or 1 is allowed in phase description".to_string(),
                    });
                }
            }
        }

        self.pla.phase = Some(cube);
        Ok(false)
    }

    fn parse_pair(&mut self, line: usize, values: &[&str]) -> CvrinResult<bool>
    {
        if self.pla.pair.is_some()
        {
            return Err(CvrinError::DuplicateDirective { line, directive: ".pair" });
        }

        let count = parse_count(line, ".pair", values.first().copied())?;
        let tokens = values[1..]
            .iter()
            .map(|value| value.trim_matches(['(', ')']))
            .collect::<Vec<_>>();
        if tokens.len() != count * 2
        {
            return Err(CvrinError::WrongLabelCount { line, directive: ".pair", expected: count * 2, actual: tokens.len() });
        }

        let mut pair = Pairing::default();
        for chunk in tokens.chunks_exact(2)
        {
            let (var1, _) = self.pla
                .label_index(chunk[0])
                .ok_or_else(|| CvrinError::InvalidLabel { line, label: chunk[0].to_string() })?;
            let (var2, _) = self.pla
                .label_index(chunk[1])
                .ok_or_else(|| CvrinError::InvalidLabel { line, label: chunk[1].to_string() })?;
            pair.var1.push(var1 + 1);
            pair.var2.push(var2 + 1);
        }

        self.pla.pair = Some(pair);
        Ok(false)
    }

    fn parse_cube(&mut self, line: usize, text: &str) -> CvrinResult<()>
    {
        let structure = self.require_structure(line)?;
        let mut scanner = ProductScanner::new(text);
        let mut cube = Cube::empty();

        for variable in 0..structure.num_binary_vars()
        {
            match scanner.next_symbol()?
            {
                '2' | '-' =>
                {
                    cube.insert(variable * 2 + 1);
                    cube.insert(variable * 2);
                }
                '0' => cube.insert(variable * 2),
                '1' => cube.insert(variable * 2 + 1),
                '?' => {}
                ch => return Err(invalid_symbol(line, ch)),
            }
        }

        for variable in structure.num_binary_vars()..structure.output_var()
        {
            for part in structure.variable_parts(variable)
            {
                match scanner.next_symbol()?
                {
                    '1' => cube.insert(part),
                    '0' => {}
                    ch => return Err(invalid_symbol(line, ch)),
                }
            }
        }

        let mut on_cube = cube.clone();
        let mut off_cube = if self.pla.kiss && structure.num_vars() >= 2
        {
            cube.xor_variable(&structure, structure.num_vars() - 2)
        }
        else
        {
            cube.clone()
        };
        let mut dc_cube = cube;
        let mut save_on = self.pla.kiss;
        let mut save_dc = false;
        let mut save_off = self.pla.kiss;

        for part in structure.variable_parts(structure.output_var())
        {
            match scanner.next_symbol()?
            {
                '4' | '1' =>
                {
                    if self.pla.pla_type.contains(PlaType::ON)
                    {
                        on_cube.insert(part);
                        save_on = true;
                    }
                }
                '3' | '0' =>
                {
                    if self.pla.pla_type.contains(PlaType::OFF)
                    {
                        off_cube.insert(part);
                        save_off = true;
                    }
                }
                '2' | '-' =>
                {
                    if self.pla.pla_type.contains(PlaType::DONT_CARE)
                    {
                        dc_cube.insert(part);
                        save_dc = true;
                    }
                }
                '~' => {}
                ch => return Err(invalid_symbol(line, ch)),
            }
        }

        if scanner.has_remaining()
        {
            return Err(CvrinError::InvalidCube
            {
                line,
                reason: "too many product-term symbols".to_string(),
            });
        }

        if save_on
        {
            self.pla.on_set.push(on_cube);
        }

        if save_dc
        {
            self.pla.dont_care_set.push(dc_cube);
        }

        if save_off
        {
            self.pla.off_set.push(off_cube);
        }

        self.saw_product = true;
        Ok(())
    }

    fn require_structure(&self, line: usize) -> CvrinResult<CubeStructure>
    {
        self.pla
            .structure
            .clone()
            .ok_or(CvrinError::MissingShape { line })
    }
}

struct ProductScanner<'a>
{
    chars: std::str::Chars<'a>,
}

impl<'a> ProductScanner<'a>
{
    fn new(text: &'a str) -> Self
    {
        Self
        {
            chars: text.chars(),
        }
    }

    fn next_symbol(&mut self) -> CvrinResult<char>
    {
        self.chars
            .by_ref()
            .find(|ch| !ch.is_whitespace() && *ch != '|')
            .ok_or_else(||
            {
                CvrinError::InvalidCube
                {
                    line: 0,
                    reason: "product term ended early".to_string(),
                }
            })
    }

    fn has_remaining(&mut self) -> bool
    {
        self.chars.any(|ch| !ch.is_whitespace() && ch != '|')
    }
}

fn parse_symbolic_words(line: usize, pla: &Pla, values: &[&str]) -> CvrinResult<SymbolicDeclaration>
{
    let mut sections = values.split(|value| *value == ";");
    let item_words = sections.next().unwrap_or_default();
    let label_words = sections.next().unwrap_or_default();
    if sections.any(|section| !section.is_empty())
    {
        return Err(CvrinError::InvalidDirective { line, directive: ".symbolic".to_string() });
    }

    let mut items = Vec::new();
    for word in item_words
    {
        let (variable, position) = pla
            .label_index(word)
            .ok_or_else(|| CvrinError::InvalidLabel { line, label: (*word).to_string() })?;
        items.push(SymbolicItem { variable, position });
    }

    Ok(SymbolicDeclaration
    {
        items,
        labels: label_words.iter().map(|label| (*label).to_string()).collect(),
    })
}

fn parse_count(line: usize, directive: &'static str, value: Option<&str>) -> CvrinResult<usize>
{
    let Some(value) = value else
    {
        return Err(CvrinError::InvalidCount { line, directive, value: String::new() });
    };

    parse_usize(line, directive, value)
}

fn parse_usize(line: usize, directive: &'static str, value: &str) -> CvrinResult<usize>
{
    value
        .parse::<usize>()
        .map_err(|_| CvrinError::InvalidCount { line, directive, value: value.to_string() })
}

fn parse_signed_count(line: usize, directive: &'static str, value: &str) -> CvrinResult<usize>
{
    let value = value.trim_start_matches('-');
    parse_usize(line, directive, value)
}

fn invalid_symbol(line: usize, symbol: char) -> CvrinError
{
    CvrinError::InvalidCube
    {
        line,
        reason: format!("unexpected symbol {symbol:?}"),
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn parts(cube: &Cube) -> BTreeSet<usize>
    {
        cube.parts().clone()
    }

    fn set(values: &[usize]) -> BTreeSet<usize>
    {
        values.iter().copied().collect()
    }

    #[test]
    fn parses_binary_pla_shape_labels_and_covers()
    {
        let input = "\
.i 2
.o 2
.ilb a b
.ob y z
.type fd
10 10
-1 1-
0- 02
.e
";

        let pla = read_pla(input, ReadPlaOptions::default()).unwrap().unwrap();

        let structure = pla.structure.as_ref().unwrap();
        assert_eq!(structure.num_binary_vars(), 2);
        assert_eq!(structure.part_size(structure.output_var()), Some(2));
        assert_eq!(pla.labels[0].as_deref(), Some("a.bar"));
        assert_eq!(pla.labels[1].as_deref(), Some("a"));
        assert_eq!(pla.labels[4].as_deref(), Some("y"));
        assert_eq!(pla.labels[5].as_deref(), Some("z"));
        assert_eq!(pla.on_set.len(), 2);
        assert_eq!(pla.dont_care_set.len(), 2);
        assert_eq!(parts(&pla.on_set[0]), set(&[1, 2, 4]));
        assert_eq!(parts(&pla.on_set[1]), set(&[0, 1, 3, 4]));
        assert_eq!(parts(&pla.dont_care_set[0]), set(&[0, 1, 3, 5]));
    }

    #[test]
    fn parses_mv_shape_variable_labels_phase_pair_and_symbolics()
    {
        let input = "\
.mv 4 2 3 2
.label var=2 idle run wait
.label var=3 ok fail
.ilb a b
.ob y z
.phase 10
.pair 1 (a b)
.symbolic a b ; s0 s1 s2 s3 ;
.symbolic-output y z ; good bad ;
10 100 10
.e
";

        let pla = read_pla(input, ReadPlaOptions::default()).unwrap().unwrap();
        let structure = pla.structure.as_ref().unwrap();

        assert_eq!(structure.num_vars(), 4);
        assert_eq!(structure.size(), 9);
        assert_eq!(pla.labels[4].as_deref(), Some("idle"));
        assert_eq!(pla.labels[8].as_deref(), Some("z"));
        assert_eq!(pla.phase.as_ref().unwrap().contains(7), true);
        assert_eq!(pla.phase.as_ref().unwrap().contains(8), false);
        assert_eq!(pla.pair.as_ref().unwrap().var1, vec![1]);
        assert_eq!(pla.pair.as_ref().unwrap().var2, vec![2]);
        assert_eq!(pla.symbolic[0].labels, ["s0", "s1", "s2", "s3"]);
        assert_eq!(pla.symbolic_output[0].labels, ["good", "bad"]);
        assert_eq!(parts(&pla.on_set[0]), set(&[1, 2, 4, 7]));
    }

    #[test]
    fn label_index_uses_numeric_fallback_until_labels_are_allocated()
    {
        let mut pla = new_pla(PlaType::ON_DC);
        pla.structure = Some(CubeStructure::binary(1, 1).unwrap());
        pla.allocate_labels();

        assert_eq!(pla.label_index("2"), Some((2, 2)));

        pla.labels[0] = Some("a.bar".to_string());
        pla.labels[1] = Some("a".to_string());
        pla.labels[2] = Some("y".to_string());

        assert_eq!(pla.label_index("a"), Some((0, 1)));
        assert_eq!(pla.label_index("y"), Some((1, 0)));
        assert_eq!(pla.label_index("2"), None);
    }

    #[test]
    fn positive_phase_swaps_on_and_off_sets()
    {
        let pla = read_pla(
            ".i 1\n.o 1\n0 1\n1 0\n.e\n",
            ReadPlaOptions
            {
                positive_phase: true,
                ..ReadPlaOptions::default()
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(pla.on_set.len(), 0);
        assert_eq!(pla.off_set.len(), 1);
        assert_eq!(parts(&pla.off_set[0]), set(&[0, 2]));
        assert!(pla.phase.is_some());
    }

    #[test]
    fn rejects_cubes_before_shape_and_bad_symbols()
    {
        assert_eq!(
            read_pla("1 1\n", ReadPlaOptions::default()).unwrap_err(),
            CvrinError::MissingShape { line: 1 }
        );

        assert!(matches!(
            read_pla(".i 1\n.o 1\nx 1\n.e\n", ReadPlaOptions::default()),
            Err(CvrinError::InvalidCube { line: 3, .. })
        ));
    }

    #[test]
    fn rejects_wrong_label_count_and_unknown_pair_labels()
    {
        assert_eq!(
            read_pla(".i 2\n.o 1\n.ilb a\n.e\n", ReadPlaOptions::default()).unwrap_err(),
            CvrinError::WrongLabelCount
            {
                line: 3,
                directive: ".ilb",
                expected: 2,
                actual: 1,
            }
        );

        assert!(matches!(
            read_pla(".i 2\n.o 1\n.ilb a b\n.pair 1 (a c)\n.e\n", ReadPlaOptions::default()),
            Err(CvrinError::InvalidLabel { line: 4, .. })
        ));
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present()
    {
        let source = include_str!("cvrin.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }
}
