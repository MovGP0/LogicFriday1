//! Native Rust helpers for Espresso symbolic PLA remapping.
//!
//! These routines model the legacy symbolic-input, symbolic-output, label
//! rewrite, and FSM disassembly transformations on owned set-family data. They
//! deliberately expose Rust data structures and results instead of preserving
//! per-file C ABI entry points.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CubeStructure {
    num_binary_vars: usize,
    part_size: Vec<usize>,
}

impl CubeStructure {
    pub fn new(num_binary_vars: usize, part_size: Vec<usize>) -> HackResult<Self> {
        if num_binary_vars > part_size.len() {
            return Err(HackError::InvalidBinaryVarCount {
                binary_vars: num_binary_vars,
                variables: part_size.len(),
            });
        }

        for (index, size) in part_size.iter().copied().enumerate() {
            if size == 0 {
                return Err(HackError::EmptyPart { variable: index });
            }
        }

        for (index, size) in part_size.iter().copied().take(num_binary_vars).enumerate() {
            if size != 2 {
                return Err(HackError::BinaryPartHasWrongSize {
                    variable: index,
                    size,
                });
            }
        }

        Ok(Self {
            num_binary_vars,
            part_size,
        })
    }

    pub fn binary(input_count: usize, output_count: usize) -> Self {
        let mut part_size = vec![2; input_count];
        part_size.push(output_count.max(1));

        Self {
            num_binary_vars: input_count,
            part_size,
        }
    }

    pub fn num_binary_vars(&self) -> usize {
        self.num_binary_vars
    }

    pub fn num_vars(&self) -> usize {
        self.part_size.len()
    }

    pub fn output(&self) -> usize {
        self.part_size.len() - 1
    }

    pub fn part_size(&self, variable: usize) -> HackResult<usize> {
        self.part_size
            .get(variable)
            .copied()
            .ok_or(HackError::VariableOutOfRange {
                variable,
                variables: self.part_size.len(),
            })
    }

    pub fn part_sizes(&self) -> &[usize] {
        &self.part_size
    }

    pub fn first_part(&self, variable: usize) -> HackResult<usize> {
        if variable >= self.part_size.len() {
            return Err(HackError::VariableOutOfRange {
                variable,
                variables: self.part_size.len(),
            });
        }

        Ok(self.part_size.iter().take(variable).sum())
    }

    pub fn size(&self) -> usize {
        self.part_size.iter().sum()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetRow {
    size: usize,
    bits: BTreeSet<usize>,
}

impl SetRow {
    pub fn full(size: usize) -> Self {
        Self {
            size,
            bits: (0..size).collect(),
        }
    }

    pub fn empty(size: usize) -> Self {
        Self {
            size,
            bits: BTreeSet::new(),
        }
    }

    pub fn from_bits(size: usize, bits: impl IntoIterator<Item = usize>) -> HackResult<Self> {
        let mut row = Self::empty(size);
        for bit in bits {
            row.insert(bit)?;
        }

        Ok(row)
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn bits(&self) -> &BTreeSet<usize> {
        &self.bits
    }

    pub fn contains(&self, bit: usize) -> bool {
        self.bits.contains(&bit)
    }

    pub fn insert(&mut self, bit: usize) -> HackResult<bool> {
        if bit >= self.size {
            return Err(HackError::BitOutOfRange {
                bit,
                size: self.size,
            });
        }

        Ok(self.bits.insert(bit))
    }

    pub fn remove(&mut self, bit: usize) {
        self.bits.remove(&bit);
    }

    pub fn retain_only(&self, keep: &BTreeSet<usize>) -> Self {
        Self {
            size: self.size,
            bits: self.bits.intersection(keep).copied().collect(),
        }
    }

    pub fn shifted_for_add_cols(&self, start: usize, count: usize) -> Self {
        Self {
            size: self.size + count,
            bits: self
                .bits
                .iter()
                .map(|bit| if *bit >= start { *bit + count } else { *bit })
                .collect(),
        }
    }

    pub fn compressed(&self, keep: &BTreeSet<usize>) -> Self {
        let index_by_old_bit = keep
            .iter()
            .copied()
            .enumerate()
            .map(|(new_bit, old_bit)| (old_bit, new_bit))
            .collect::<std::collections::BTreeMap<_, _>>();
        let bits = self
            .bits
            .iter()
            .filter_map(|bit| index_by_old_bit.get(bit).copied())
            .collect();

        Self {
            size: keep.len(),
            bits,
        }
    }

    fn without_range(&self, first: usize, last_inclusive: usize) -> Self {
        let width = last_inclusive - first + 1;
        let bits = self
            .bits
            .iter()
            .filter_map(|bit| {
                if *bit < first {
                    Some(*bit)
                } else if *bit > last_inclusive {
                    Some(*bit - width)
                } else {
                    None
                }
            })
            .collect();

        Self {
            size: self.size - width,
            bits,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetFamily {
    set_size: usize,
    rows: Vec<SetRow>,
}

impl SetFamily {
    pub fn new(set_size: usize) -> Self {
        Self {
            set_size,
            rows: Vec::new(),
        }
    }

    pub fn from_rows<I, R>(set_size: usize, rows: I) -> HackResult<Self>
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = usize>,
    {
        let mut family = Self::new(set_size);
        for row in rows {
            family.push(SetRow::from_bits(set_size, row)?)?;
        }

        Ok(family)
    }

    pub fn set_size(&self) -> usize {
        self.set_size
    }

    pub fn rows(&self) -> &[SetRow] {
        &self.rows
    }

    pub fn rows_mut(&mut self) -> &mut [SetRow] {
        &mut self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn push(&mut self, row: SetRow) -> HackResult<()> {
        if row.size() != self.set_size {
            return Err(HackError::SetSizeMismatch {
                expected: self.set_size,
                actual: row.size(),
            });
        }

        self.rows.push(row);
        Ok(())
    }

    pub fn add_columns(&mut self, start: usize, count: usize) -> HackResult<()> {
        if start > self.set_size {
            return Err(HackError::ColumnInsertOutOfRange {
                column: start,
                size: self.set_size,
            });
        }

        self.rows = self
            .rows
            .iter()
            .map(|row| row.shifted_for_add_cols(start, count))
            .collect();
        self.set_size += count;
        Ok(())
    }

    pub fn compress(&mut self, keep: &BTreeSet<usize>) -> HackResult<()> {
        if let Some(bit) = keep.iter().find(|bit| **bit >= self.set_size) {
            return Err(HackError::BitOutOfRange {
                bit: *bit,
                size: self.set_size,
            });
        }

        self.rows = self.rows.iter().map(|row| row.compressed(keep)).collect();
        self.set_size = keep.len();
        Ok(())
    }

    pub fn delete_column_range(&mut self, first: usize, last_inclusive: usize) -> HackResult<()> {
        if first > last_inclusive || last_inclusive >= self.set_size {
            return Err(HackError::InvalidColumnRange {
                first,
                last_inclusive,
                size: self.set_size,
            });
        }

        self.rows = self
            .rows
            .iter()
            .map(|row| row.without_range(first, last_inclusive))
            .collect();
        self.set_size -= last_inclusive - first + 1;
        Ok(())
    }

    pub fn retain_rows<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&SetRow) -> bool,
    {
        self.rows.retain(|row| predicate(row));
    }

    pub fn append(&mut self, mut other: Self) -> HackResult<()> {
        if other.set_size != self.set_size {
            return Err(HackError::SetSizeMismatch {
                expected: self.set_size,
                actual: other.set_size,
            });
        }

        self.rows.append(&mut other.rows);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicListEntry {
    pub variable: usize,
    pub pos: usize,
}

impl SymbolicListEntry {
    pub fn binary_variable(variable: usize) -> Self {
        Self { variable, pos: 0 }
    }

    pub fn output_position(pos: usize) -> Self {
        Self { variable: 0, pos }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicGroup {
    entries: Vec<SymbolicListEntry>,
    labels: Vec<String>,
}

impl SymbolicGroup {
    pub fn new(entries: Vec<SymbolicListEntry>, labels: Vec<String>) -> HackResult<Self> {
        let value_count = checked_value_count(entries.len())?;
        if !labels.is_empty() && labels.len() != value_count {
            return Err(HackError::SymbolicLabelCount {
                expected: value_count,
                actual: labels.len(),
            });
        }

        Ok(Self { entries, labels })
    }

    pub fn entries(&self) -> &[SymbolicListEntry] {
        &self.entries
    }

    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    pub fn value_count(&self) -> usize {
        1usize << self.entries.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pla {
    cube: CubeStructure,
    on_set: SetFamily,
    dont_care_set: SetFamily,
    off_set: SetFamily,
    labels: Option<Vec<String>>,
    symbolic: Vec<SymbolicGroup>,
    symbolic_output: Vec<SymbolicGroup>,
}

impl Pla {
    pub fn new(
        cube: CubeStructure,
        on_set: SetFamily,
        dont_care_set: SetFamily,
        off_set: SetFamily,
        labels: Option<Vec<String>>,
    ) -> HackResult<Self> {
        ensure_family_size(&cube, &on_set)?;
        ensure_family_size(&cube, &dont_care_set)?;
        ensure_family_size(&cube, &off_set)?;

        if let Some(labels) = &labels {
            if labels.len() != cube.size() {
                return Err(HackError::LabelCount {
                    expected: cube.size(),
                    actual: labels.len(),
                });
            }
        }

        Ok(Self {
            cube,
            on_set,
            dont_care_set,
            off_set,
            labels,
            symbolic: Vec::new(),
            symbolic_output: Vec::new(),
        })
    }

    pub fn cube(&self) -> &CubeStructure {
        &self.cube
    }

    pub fn on_set(&self) -> &SetFamily {
        &self.on_set
    }

    pub fn dont_care_set(&self) -> &SetFamily {
        &self.dont_care_set
    }

    pub fn off_set(&self) -> &SetFamily {
        &self.off_set
    }

    pub fn labels(&self) -> Option<&[String]> {
        self.labels.as_deref()
    }

    pub fn set_symbolic(&mut self, symbolic: Vec<SymbolicGroup>) {
        self.symbolic = symbolic;
    }

    pub fn set_symbolic_output(&mut self, symbolic_output: Vec<SymbolicGroup>) {
        self.symbolic_output = symbolic_output;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DcSetReport {
    pub removed_variable: Option<usize>,
    pub retained_on_rows: usize,
    pub dont_care_rows: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolicMapReport {
    pub removed_binary_vars: usize,
    pub added_symbolic_vars: usize,
    pub added_columns: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputSymbolicMapReport {
    pub added_output_columns: usize,
    pub removed_output_columns: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FsmArc {
    pub present_state: Option<usize>,
    pub next_state: Option<usize>,
    pub outputs: BTreeSet<usize>,
    pub cube: SetRow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HackError {
    InvalidBinaryVarCount {
        binary_vars: usize,
        variables: usize,
    },
    BinaryPartHasWrongSize {
        variable: usize,
        size: usize,
    },
    EmptyPart {
        variable: usize,
    },
    VariableOutOfRange {
        variable: usize,
        variables: usize,
    },
    BitOutOfRange {
        bit: usize,
        size: usize,
    },
    SetSizeMismatch {
        expected: usize,
        actual: usize,
    },
    LabelCount {
        expected: usize,
        actual: usize,
    },
    SymbolicLabelCount {
        expected: usize,
        actual: usize,
    },
    SymbolicVariableMustBeBinary {
        variable: usize,
    },
    SymbolicOutputOutOfRange {
        pos: usize,
        output_size: usize,
    },
    TooManySymbolicEntries {
        entries: usize,
    },
    ColumnInsertOutOfRange {
        column: usize,
        size: usize,
    },
    InvalidColumnRange {
        first: usize,
        last_inclusive: usize,
        size: usize,
    },
    RequiresPresentAndNextStateParts,
    OutputPartSmallerThanStateCount {
        outputs: usize,
        states: usize,
    },
}

impl fmt::Display for HackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBinaryVarCount {
                binary_vars,
                variables,
            } => write!(
                formatter,
                "{binary_vars} binary variables declared for {variables} total variables"
            ),
            Self::BinaryPartHasWrongSize { variable, size } => write!(
                formatter,
                "binary variable {variable} has part size {size}; expected 2"
            ),
            Self::EmptyPart { variable } => {
                write!(formatter, "variable {variable} has an empty part")
            }
            Self::VariableOutOfRange {
                variable,
                variables,
            } => write!(formatter, "variable {variable} is outside 0..{variables}"),
            Self::BitOutOfRange { bit, size } => {
                write!(formatter, "bit {bit} is outside set size {size}")
            }
            Self::SetSizeMismatch { expected, actual } => {
                write!(formatter, "set size {actual} does not match expected {expected}")
            }
            Self::LabelCount { expected, actual } => {
                write!(formatter, "{actual} labels supplied for set size {expected}")
            }
            Self::SymbolicLabelCount { expected, actual } => write!(
                formatter,
                "{actual} symbolic labels supplied for {expected} encoded values"
            ),
            Self::SymbolicVariableMustBeBinary { variable } => {
                write!(formatter, "symbolic variable {variable} is not binary")
            }
            Self::SymbolicOutputOutOfRange { pos, output_size } => write!(
                formatter,
                "symbolic output position {pos} is outside output part size {output_size}"
            ),
            Self::TooManySymbolicEntries { entries } => {
                write!(formatter, "{entries} symbolic entries exceed addressable width")
            }
            Self::ColumnInsertOutOfRange { column, size } => {
                write!(formatter, "cannot insert columns at {column} in set size {size}")
            }
            Self::InvalidColumnRange {
                first,
                last_inclusive,
                size,
            } => write!(
                formatter,
                "column range {first}..={last_inclusive} is invalid for set size {size}"
            ),
            Self::RequiresPresentAndNextStateParts => write!(
                formatter,
                "FSM disassembly requires exactly present-state, next-state, and output parts"
            ),
            Self::OutputPartSmallerThanStateCount { outputs, states } => write!(
                formatter,
                "output part has {outputs} bits but state count is {states}"
            ),
        }
    }
}

impl Error for HackError {}

pub type HackResult<T> = Result<T, HackError>;

pub fn map_dcset(pla: &mut Pla) -> HackResult<DcSetReport> {
    let Some(labels) = pla.labels.as_ref() else {
        return Ok(DcSetReport {
            removed_variable: None,
            retained_on_rows: pla.on_set.len(),
            dont_care_rows: pla.dont_care_set.len(),
        });
    };

    let Some(var) = labels
        .iter()
        .take(pla.cube.num_binary_vars() * 2)
        .position(|label| is_dont_care_label(label))
        .map(|index| index / 2)
    else {
        return Ok(DcSetReport {
            removed_variable: None,
            retained_on_rows: pla.on_set.len(),
            dont_care_rows: pla.dont_care_set.len(),
        });
    };

    let zero_bit = var * 2;
    let one_bit = zero_bit + 1;
    let mut derived_dont_care = SetFamily::new(pla.on_set.set_size());
    for row in pla.on_set.rows() {
        let has_zero = row.contains(zero_bit);
        let has_one = row.contains(one_bit);
        if has_zero ^ has_one {
            derived_dont_care.push(row.clone())?;
        }
    }

    pla.dont_care_set.append(derived_dont_care)?;
    pla.on_set
        .retain_rows(|row| row.contains(zero_bit) && row.contains(one_bit));

    pla.on_set.delete_column_range(zero_bit, one_bit)?;
    pla.dont_care_set.delete_column_range(zero_bit, one_bit)?;
    pla.off_set.delete_column_range(zero_bit, one_bit)?;

    if let Some(labels) = &mut pla.labels {
        labels.drain(zero_bit..=one_bit);
    }

    pla.cube.part_size.remove(var);
    pla.cube.num_binary_vars -= 1;

    Ok(DcSetReport {
        removed_variable: Some(var),
        retained_on_rows: pla.on_set.len(),
        dont_care_rows: pla.dont_care_set.len(),
    })
}

pub fn map_symbolic(pla: &mut Pla) -> HackResult<SymbolicMapReport> {
    for group in &pla.symbolic {
        for entry in group.entries() {
            if entry.variable >= pla.cube.num_binary_vars() {
                return Err(HackError::SymbolicVariableMustBeBinary {
                    variable: entry.variable,
                });
            }
        }
    }

    let size_added: usize = pla.symbolic.iter().map(SymbolicGroup::value_count).sum();
    let mut compress = full_set(pla.cube.size() + size_added);
    for group in &pla.symbolic {
        for entry in group.entries() {
            compress.remove(&(entry.variable * 2));
            compress.remove(&(entry.variable * 2 + 1));
        }
    }

    let num_deleted_vars = (pla.cube.size() + size_added - compress.len()) / 2;
    let num_added_vars = pla.symbolic.len();
    let output_base = pla.cube.first_part(pla.cube.output())?;

    add_columns_to_all_families(pla, output_base, size_added)?;

    let mut base = output_base;
    for group in &pla.symbolic {
        map_symbolic_cover(&mut pla.on_set, group.entries(), base)?;
        map_symbolic_cover(&mut pla.dont_care_set, group.entries(), base)?;
        map_symbolic_cover(&mut pla.off_set, group.entries(), base)?;
        base += group.value_count();
    }

    pla.on_set.compress(&compress)?;
    pla.dont_care_set.compress(&compress)?;
    pla.off_set.compress(&compress)?;

    let new_size = pla.cube.size() - num_deleted_vars * 2 + size_added;
    symbolic_hack_labels(
        &mut pla.labels,
        &pla.symbolic,
        &compress,
        new_size,
        pla.cube.size(),
        size_added,
        output_base,
    )?;

    let mut new_part_size = Vec::new();
    let removed_binary_vars = removed_binary_vars(&pla.symbolic);
    for variable in 0..pla.cube.num_binary_vars() {
        if !removed_binary_vars.contains(&variable) {
            new_part_size.push(2);
        }
    }

    for group in &pla.symbolic {
        new_part_size.push(group.value_count());
    }

    for variable in pla.cube.num_binary_vars()..pla.cube.num_vars() {
        new_part_size.push(pla.cube.part_size(variable)?);
    }

    pla.cube = CubeStructure::new(
        pla.cube.num_binary_vars() - num_deleted_vars,
        new_part_size,
    )?;

    Ok(SymbolicMapReport {
        removed_binary_vars: num_deleted_vars,
        added_symbolic_vars: num_added_vars,
        added_columns: size_added,
    })
}

pub fn map_symbolic_cover(
    cover: &mut SetFamily,
    list: &[SymbolicListEntry],
    base: usize,
) -> HackResult<()> {
    for row in cover.rows_mut() {
        form_bitvector(row, base, 0, list)?;
    }

    Ok(())
}

pub fn form_bitvector(
    row: &mut SetRow,
    base: usize,
    value: usize,
    list: &[SymbolicListEntry],
) -> HackResult<()> {
    let Some((entry, remaining)) = list.split_first() else {
        row.insert(base + value)?;
        return Ok(());
    };

    let zero = row.contains(entry.variable * 2);
    let one = row.contains(entry.variable * 2 + 1);
    match (zero, one) {
        (true, false) => form_bitvector(row, base, value * 2, remaining),
        (false, true) => form_bitvector(row, base, value * 2 + 1, remaining),
        (true, true) => {
            form_bitvector(row, base, value * 2, remaining)?;
            form_bitvector(row, base, value * 2 + 1, remaining)
        }
        (false, false) => Ok(()),
    }
}

pub fn map_output_symbolic(pla: &mut Pla) -> HackResult<OutputSymbolicMapReport> {
    let output_size = pla.cube.part_size(pla.cube.output())?;
    for group in &pla.symbolic_output {
        for entry in group.entries() {
            if entry.pos >= output_size {
                return Err(HackError::SymbolicOutputOutOfRange {
                    pos: entry.pos,
                    output_size,
                });
            }
        }
    }

    let size_added: usize = pla.symbolic_output.iter().map(SymbolicGroup::value_count).sum();
    let output_base = pla.cube.first_part(pla.cube.output())?;
    add_columns_to_all_families(pla, output_base, size_added)?;

    let mut base = output_base;
    for group in &pla.symbolic_output {
        map_output_symbolic_cover(
            &mut pla.on_set,
            group.entries(),
            output_base + size_added,
            base,
        )?;
        map_output_symbolic_cover(
            &mut pla.dont_care_set,
            group.entries(),
            output_base + size_added,
            base,
        )?;
        base += group.value_count();
    }

    let mut compress = full_set(pla.cube.size() + size_added);
    for group in &pla.symbolic_output {
        for entry in group.entries() {
            compress.remove(&(output_base + size_added + entry.pos));
        }
    }

    let old_size = pla.cube.size();
    let new_size = compress.len();
    pla.on_set.compress(&compress)?;
    pla.dont_care_set.compress(&compress)?;
    pla.off_set.compress(&compress)?;
    symbolic_hack_labels(
        &mut pla.labels,
        &pla.symbolic_output,
        &compress,
        new_size,
        old_size,
        size_added,
        output_base,
    )?;

    let removed_output_columns = old_size + size_added - new_size;
    let output_part = pla.cube.output();
    pla.cube.part_size[output_part] = pla.cube.part_size[output_part] + size_added - removed_output_columns;

    Ok(OutputSymbolicMapReport {
        added_output_columns: size_added,
        removed_output_columns,
    })
}

pub fn disassemble_fsm(pla: &Pla) -> HackResult<Vec<FsmArc>> {
    if pla.cube.num_vars() - pla.cube.num_binary_vars() != 3 {
        return Err(HackError::RequiresPresentAndNextStateParts);
    }

    let present_state_var = pla.cube.num_binary_vars();
    let next_state_var = present_state_var + 1;
    let output_var = next_state_var + 1;
    let states = pla.cube.part_size(present_state_var)?;
    let outputs = pla.cube.part_size(output_var)?;
    if outputs < states {
        return Err(HackError::OutputPartSmallerThanStateCount { outputs, states });
    }

    let present_base = pla.cube.first_part(present_state_var)?;
    let next_base = pla.cube.first_part(next_state_var)?;
    let output_base = pla.cube.first_part(output_var)?;
    let mut arcs = Vec::new();

    for row in pla.on_set.rows() {
        let present_states = selected_values(row, present_base, states);
        let next_states = selected_values(row, next_base, states);
        let output_bits = selected_values(row, output_base + states, outputs - states)
            .into_iter()
            .collect::<BTreeSet<_>>();

        match (present_states.as_slice(), next_states.as_slice()) {
            ([], []) => arcs.push(FsmArc {
                present_state: None,
                next_state: None,
                outputs: output_bits.clone(),
                cube: row.clone(),
            }),
            ([], next_values) => {
                for next in next_values {
                    arcs.push(FsmArc {
                        present_state: None,
                        next_state: Some(*next),
                        outputs: output_bits.clone(),
                        cube: row.clone(),
                    });
                }
            }
            (present_values, []) => {
                for present in present_values {
                    arcs.push(FsmArc {
                        present_state: Some(*present),
                        next_state: None,
                        outputs: output_bits.clone(),
                        cube: row.clone(),
                    });
                }
            }
            (present_values, next_values) => {
                for present in present_values {
                    for next in next_values {
                        arcs.push(FsmArc {
                            present_state: Some(*present),
                            next_state: Some(*next),
                            outputs: output_bits.clone(),
                            cube: row.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(arcs)
}

fn map_output_symbolic_cover(
    cover: &mut SetFamily,
    list: &[SymbolicListEntry],
    original_output_base: usize,
    encoded_base: usize,
) -> HackResult<()> {
    for row in cover.rows_mut() {
        let values = encoded_output_values(row, list, original_output_base);
        for value in values {
            row.insert(encoded_base + value)?;
        }
    }

    Ok(())
}

fn encoded_output_values(
    row: &SetRow,
    list: &[SymbolicListEntry],
    original_output_base: usize,
) -> Vec<usize> {
    encoded_output_values_inner(row, list, original_output_base, 0)
}

fn encoded_output_values_inner(
    row: &SetRow,
    list: &[SymbolicListEntry],
    original_output_base: usize,
    value: usize,
) -> Vec<usize> {
    let Some((entry, remaining)) = list.split_first() else {
        return vec![value];
    };

    let bit = original_output_base + entry.pos;
    let shift = remaining.len();
    let next_value = if row.contains(bit) {
        value | (1usize << shift)
    } else {
        value
    };

    encoded_output_values_inner(row, remaining, original_output_base, next_value)
}

fn symbolic_hack_labels(
    labels: &mut Option<Vec<String>>,
    groups: &[SymbolicGroup],
    compress: &BTreeSet<usize>,
    new_size: usize,
    old_size: usize,
    size_added: usize,
    output_base: usize,
) -> HackResult<()> {
    let Some(old_labels) = labels.take() else {
        return Ok(());
    };

    let mut new_labels = Vec::with_capacity(new_size);

    for old_bit in 0..output_base {
        if compress.contains(&old_bit) {
            new_labels.push(old_labels[old_bit].clone());
        }
    }

    for group in groups {
        if group.labels().is_empty() {
            for value in 0..group.value_count() {
                new_labels.push(format!("X{value}"));
            }
        } else {
            new_labels.extend(group.labels().iter().cloned());
        }
    }

    for old_bit in output_base..old_size {
        if compress.contains(&(old_bit + size_added)) {
            new_labels.push(old_labels[old_bit].clone());
        }
    }

    if new_labels.len() != new_size {
        return Err(HackError::LabelCount {
            expected: new_size,
            actual: new_labels.len(),
        });
    }

    *labels = Some(new_labels);
    Ok(())
}

fn add_columns_to_all_families(pla: &mut Pla, start: usize, count: usize) -> HackResult<()> {
    pla.on_set.add_columns(start, count)?;
    pla.dont_care_set.add_columns(start, count)?;
    pla.off_set.add_columns(start, count)?;
    Ok(())
}

fn ensure_family_size(cube: &CubeStructure, family: &SetFamily) -> HackResult<()> {
    if family.set_size() != cube.size() {
        return Err(HackError::SetSizeMismatch {
            expected: cube.size(),
            actual: family.set_size(),
        });
    }

    Ok(())
}

fn removed_binary_vars(groups: &[SymbolicGroup]) -> BTreeSet<usize> {
    groups
        .iter()
        .flat_map(SymbolicGroup::entries)
        .map(|entry| entry.variable)
        .collect()
}

fn checked_value_count(entries: usize) -> HackResult<usize> {
    if entries >= usize::BITS as usize {
        return Err(HackError::TooManySymbolicEntries { entries });
    }

    Ok(1usize << entries)
}

fn full_set(size: usize) -> BTreeSet<usize> {
    (0..size).collect()
}

fn is_dont_care_label(label: &str) -> bool {
    let normalized = label.replace('_', "").to_ascii_lowercase();
    normalized.starts_with("dontcare")
}

fn selected_values(row: &SetRow, base: usize, count: usize) -> Vec<usize> {
    (0..count)
        .filter(|value| row.contains(base + value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn row(size: usize, bits: &[usize]) -> SetRow {
        SetRow::from_bits(size, bits.iter().copied()).unwrap()
    }

    fn family(size: usize, rows: &[&[usize]]) -> SetFamily {
        SetFamily::from_rows(size, rows.iter().map(|row| row.iter().copied())).unwrap()
    }

    #[test]
    fn form_bitvector_inserts_all_encoded_values_for_dont_care_inputs() {
        let mut cube = row(8, &[0, 1, 2, 3]);
        let entries = vec![
            SymbolicListEntry::binary_variable(0),
            SymbolicListEntry::binary_variable(1),
        ];

        form_bitvector(&mut cube, 4, 0, &entries).unwrap();

        assert!(cube.contains(4));
        assert!(cube.contains(5));
        assert!(cube.contains(6));
        assert!(cube.contains(7));
    }

    #[test]
    fn map_symbolic_replaces_binary_inputs_with_multiple_valued_part() {
        let cube = CubeStructure::binary(2, 1);
        let mut pla = Pla::new(
            cube,
            family(5, &[&[0, 3, 4], &[1, 2, 4]]),
            SetFamily::new(5),
            SetFamily::new(5),
            Some(labels(&["a0", "a1", "b0", "b1", "f"])),
        )
        .unwrap();
        pla.set_symbolic(vec![
            SymbolicGroup::new(
                vec![
                    SymbolicListEntry::binary_variable(0),
                    SymbolicListEntry::binary_variable(1),
                ],
                labels(&["s0", "s1", "s2", "s3"]),
            )
            .unwrap(),
        ]);

        let report = map_symbolic(&mut pla).unwrap();

        assert_eq!(
            report,
            SymbolicMapReport {
                removed_binary_vars: 2,
                added_symbolic_vars: 1,
                added_columns: 4,
            }
        );
        assert_eq!(pla.cube.part_sizes(), &[4, 1]);
        assert_eq!(pla.labels(), Some(labels(&["s0", "s1", "s2", "s3", "f"]).as_slice()));
        assert_eq!(pla.on_set.rows(), &[row(5, &[1, 4]), row(5, &[2, 4])]);
    }

    #[test]
    fn map_output_symbolic_adds_one_hot_outputs_and_removes_source_bits() {
        let cube = CubeStructure::binary(1, 3);
        let mut pla = Pla::new(
            cube,
            family(5, &[&[1, 2, 4]]),
            SetFamily::new(5),
            SetFamily::new(5),
            Some(labels(&["a0", "a1", "o0", "o1", "keep"])),
        )
        .unwrap();
        pla.set_symbolic_output(vec![
            SymbolicGroup::new(
                vec![
                    SymbolicListEntry::output_position(0),
                    SymbolicListEntry::output_position(1),
                ],
                labels(&["n0", "n1", "n2", "n3"]),
            )
            .unwrap(),
        ]);

        let report = map_output_symbolic(&mut pla).unwrap();

        assert_eq!(
            report,
            OutputSymbolicMapReport {
                added_output_columns: 4,
                removed_output_columns: 2,
            }
        );
        assert_eq!(pla.cube.part_sizes(), &[2, 5]);
        assert_eq!(
            pla.labels(),
            Some(labels(&["a0", "a1", "n0", "n1", "n2", "n3", "keep"]).as_slice())
        );
        assert_eq!(pla.on_set.rows(), &[row(7, &[1, 4, 6])]);
    }

    #[test]
    fn map_output_symbolic_preserves_entry_order_for_three_bit_values() {
        let cube = CubeStructure::binary(0, 4);
        let mut pla = Pla::new(
            cube,
            family(4, &[&[0, 2, 3]]),
            SetFamily::new(4),
            SetFamily::new(4),
            Some(labels(&["o0", "o1", "o2", "keep"])),
        )
        .unwrap();
        pla.set_symbolic_output(vec![
            SymbolicGroup::new(
                vec![
                    SymbolicListEntry::output_position(0),
                    SymbolicListEntry::output_position(1),
                    SymbolicListEntry::output_position(2),
                ],
                Vec::new(),
            )
            .unwrap(),
        ]);

        map_output_symbolic(&mut pla).unwrap();

        assert_eq!(pla.on_set.rows(), &[row(9, &[5, 8])]);
    }

    #[test]
    fn map_dcset_removes_named_dont_care_binary_variable() {
        let cube = CubeStructure::binary(2, 1);
        let mut pla = Pla::new(
            cube,
            family(5, &[&[0, 1, 2, 4], &[0, 3, 4]]),
            SetFamily::new(5),
            SetFamily::new(5),
            Some(labels(&["DONT_CARE", "DONT_CARE", "b0", "b1", "f"])),
        )
        .unwrap();

        let report = map_dcset(&mut pla).unwrap();

        assert_eq!(report.removed_variable, Some(0));
        assert_eq!(pla.cube.part_sizes(), &[2, 1]);
        assert_eq!(pla.labels(), Some(labels(&["b0", "b1", "f"]).as_slice()));
        assert_eq!(pla.on_set.rows(), &[row(3, &[0, 2])]);
        assert_eq!(pla.dont_care_set.rows(), &[row(3, &[1, 2])]);
    }

    #[test]
    fn disassemble_fsm_expands_present_next_state_pairs() {
        let cube = CubeStructure::new(1, vec![2, 2, 2, 3]).unwrap();
        let pla = Pla::new(
            cube,
            family(9, &[&[0, 2, 3, 4, 8]]),
            SetFamily::new(9),
            SetFamily::new(9),
            None,
        )
        .unwrap();

        let arcs = disassemble_fsm(&pla).unwrap();

        assert_eq!(arcs.len(), 2);
        assert_eq!(arcs[0].present_state, Some(0));
        assert_eq!(arcs[0].next_state, Some(0));
        assert_eq!(arcs[1].present_state, Some(1));
        assert_eq!(arcs[1].next_state, Some(0));
        assert_eq!(arcs[0].outputs, BTreeSet::from([0]));
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("hack.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }
}
