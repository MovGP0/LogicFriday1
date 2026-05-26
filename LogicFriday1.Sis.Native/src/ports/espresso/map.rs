use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

const MAP_INDEX: [[usize; 16]; 16] = [
    [0, 1, 3, 2, 16, 17, 19, 18, 80, 81, 83, 82, 64, 65, 67, 66],
    [4, 5, 7, 6, 20, 21, 23, 22, 84, 85, 87, 86, 68, 69, 71, 70],
    [
        12, 13, 15, 14, 28, 29, 31, 30, 92, 93, 95, 94, 76, 77, 79, 78,
    ],
    [8, 9, 11, 10, 24, 25, 27, 26, 88, 89, 91, 90, 72, 73, 75, 74],
    [
        32, 33, 35, 34, 48, 49, 51, 50, 112, 113, 115, 114, 96, 97, 99, 98,
    ],
    [
        36, 37, 39, 38, 52, 53, 55, 54, 116, 117, 119, 118, 100, 101, 103, 102,
    ],
    [
        44, 45, 47, 46, 60, 61, 63, 62, 124, 125, 127, 126, 108, 109, 111, 110,
    ],
    [
        40, 41, 43, 42, 56, 57, 59, 58, 120, 121, 123, 122, 104, 105, 107, 106,
    ],
    [
        160, 161, 163, 162, 176, 177, 179, 178, 240, 241, 243, 242, 224, 225, 227, 226,
    ],
    [
        164, 165, 167, 166, 180, 181, 183, 182, 244, 245, 247, 246, 228, 229, 231, 230,
    ],
    [
        172, 173, 175, 174, 188, 189, 191, 190, 252, 253, 255, 254, 236, 237, 239, 238,
    ],
    [
        168, 169, 171, 170, 184, 185, 187, 186, 248, 249, 251, 250, 232, 233, 235, 234,
    ],
    [
        128, 129, 131, 130, 144, 145, 147, 146, 208, 209, 211, 210, 192, 193, 195, 194,
    ],
    [
        132, 133, 135, 134, 148, 149, 151, 150, 212, 213, 215, 214, 196, 197, 199, 198,
    ],
    [
        140, 141, 143, 142, 156, 157, 159, 158, 220, 221, 223, 222, 204, 205, 207, 206,
    ],
    [
        136, 137, 139, 138, 152, 153, 155, 154, 216, 217, 219, 218, 200, 201, 203, 202,
    ],
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapCube {
    parts: Vec<BTreeSet<usize>>,
}

impl MapCube {
    pub fn from_parts<I, P>(parts: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: IntoIterator<Item = usize>,
    {
        Self {
            parts: parts
                .into_iter()
                .map(|part| part.into_iter().collect())
                .collect(),
        }
    }

    pub fn parts(&self) -> &[BTreeSet<usize>] {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapCover {
    part_sizes: Vec<usize>,
    cubes: Vec<MapCube>,
}

impl MapCover {
    pub fn new(part_sizes: Vec<usize>, cubes: Vec<MapCube>) -> MapResult<Self> {
        if part_sizes.is_empty() {
            return Err(MapError::MissingOutputPart);
        }

        for (variable, part_size) in part_sizes.iter().enumerate() {
            if *part_size == 0 {
                return Err(MapError::EmptyPart { variable });
            }
        }

        for cube in &cubes {
            if cube.parts().len() != part_sizes.len() {
                return Err(MapError::CubeVariableCount {
                    expected: part_sizes.len(),
                    actual: cube.parts().len(),
                });
            }

            for (variable, selected) in cube.parts().iter().enumerate() {
                for value in selected {
                    if *value >= part_sizes[variable] {
                        return Err(MapError::PartValueOutOfRange {
                            variable,
                            value: *value,
                            part_size: part_sizes[variable],
                        });
                    }
                }
            }
        }

        Ok(Self { part_sizes, cubes })
    }

    pub fn from_rows<I, P>(part_sizes: Vec<usize>, cubes: I) -> MapResult<Self>
    where
        I: IntoIterator<Item = P>,
        P: IntoIterator<Item = Vec<usize>>,
    {
        Self::new(
            part_sizes,
            cubes.into_iter().map(MapCube::from_parts).collect(),
        )
    }

    pub fn part_sizes(&self) -> &[usize] {
        &self.part_sizes
    }

    pub fn cubes(&self) -> &[MapCube] {
        &self.cubes
    }

    pub fn input_count(&self) -> usize {
        self.part_sizes.len() - 1
    }

    pub fn output_count(&self) -> usize {
        self.part_sizes[self.part_sizes.len() - 1]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputSpace {
    output_index: usize,
    blocks: Vec<MapBlock>,
}

impl OutputSpace {
    pub fn output_index(&self) -> usize {
        self.output_index
    }

    pub fn blocks(&self) -> &[MapBlock] {
        &self.blocks
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapBlock {
    input_offset: usize,
    rows: Vec<MapRow>,
}

impl MapBlock {
    pub fn input_offset(&self) -> usize {
        self.input_offset
    }

    pub fn rows(&self) -> &[MapRow] {
        &self.rows
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapRow {
    row_index: usize,
    cells: Vec<bool>,
}

impl MapRow {
    pub fn row_index(&self) -> usize {
        self.row_index
    }

    pub fn cells(&self) -> &[bool] {
        &self.cells
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapError {
    MissingOutputPart,
    EmptyPart {
        variable: usize,
    },
    CubeVariableCount {
        expected: usize,
        actual: usize,
    },
    PartValueOutOfRange {
        variable: usize,
        value: usize,
        part_size: usize,
    },
    NonBinaryInput {
        variable: usize,
        part_size: usize,
    },
    TooManyInputs {
        input_count: usize,
        max_supported: usize,
    },
    MintermCountOverflow,
}

impl fmt::Display for MapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOutputPart => write!(formatter, "map cover must include an output part"),
            Self::EmptyPart { variable } => {
                write!(formatter, "map variable {variable} has an empty part")
            }
            Self::CubeVariableCount { expected, actual } => write!(
                formatter,
                "map cube has {actual} variables; expected {expected}"
            ),
            Self::PartValueOutOfRange {
                variable,
                value,
                part_size,
            } => write!(
                formatter,
                "map cube value {value} for variable {variable} is outside 0..{part_size}"
            ),
            Self::NonBinaryInput {
                variable,
                part_size,
            } => write!(
                formatter,
                "map input variable {variable} has part size {part_size}; expected 2"
            ),
            Self::TooManyInputs {
                input_count,
                max_supported,
            } => write!(
                formatter,
                "map rendering supports at most {max_supported} inputs, got {input_count}"
            ),
            Self::MintermCountOverflow => write!(formatter, "map minterm count overflowed usize"),
        }
    }
}

impl Error for MapError {}

pub type MapResult<T> = Result<T, MapError>;

pub fn minterms(cover: &MapCover) -> MapResult<BTreeSet<usize>> {
    let mut result = BTreeSet::new();
    let minterm_count = checked_minterm_count(cover.part_sizes())?;
    if minterm_count == 0 {
        return Ok(result);
    }

    for cube in cover.cubes() {
        explode_cube(
            cover.part_sizes().len() - 1,
            0,
            cover.part_sizes(),
            cube,
            &mut result,
        )?;
    }

    Ok(result)
}

pub fn karnaugh_map(cover: &MapCover) -> MapResult<Vec<OutputSpace>> {
    ensure_binary_inputs(cover)?;

    let minterms = minterms(cover)?;
    let largest_input_index = checked_power_of_two(cover.input_count())?;
    let mut spaces = Vec::with_capacity(cover.output_count());

    for output_index in 0..cover.output_count() {
        let output_offset = output_index * largest_input_index;
        let mut blocks = Vec::new();
        for input_offset in input_offsets(cover.input_count()) {
            let mut rows = Vec::new();
            for (row_index, map_row) in MAP_INDEX.iter().enumerate() {
                if let Some(row) = map_cells(
                    map_row,
                    input_offset,
                    largest_input_index,
                    output_offset,
                    &minterms,
                ) {
                    rows.push(MapRow {
                        row_index,
                        cells: row,
                    });
                }

                if row_index != 15 && MAP_INDEX[row_index + 1][0] >= largest_input_index {
                    break;
                }
            }

            blocks.push(MapBlock { input_offset, rows });
        }

        spaces.push(OutputSpace {
            output_index,
            blocks,
        });
    }

    Ok(spaces)
}

pub fn render_karnaugh_map(cover: &MapCover) -> MapResult<String> {
    let spaces = karnaugh_map(cover)?;
    let mut output = String::new();

    for space in spaces {
        output.push_str(&format!("\n\nOutput space # {}\n", space.output_index()));
        for block in space.blocks() {
            for row in block.rows() {
                for (index, value) in row.cells().iter().enumerate() {
                    output.push(if *value { '1' } else { '.' });
                    if (index + 1) % 4 == 0 {
                        output.push(' ');
                    }

                    if (index + 1) % 8 == 0 {
                        output.push_str("  ");
                    }
                }

                output.push('\n');
                if (row.row_index() + 1) % 4 == 0 {
                    output.push('\n');
                }

                if (row.row_index() + 1) % 8 == 0 {
                    output.push('\n');
                }
            }

            if block.input_offset() > 0 && !block.rows().is_empty() {
                output.push('\n');
            }
        }
    }

    Ok(output)
}

fn explode_cube(
    variable: usize,
    offset: usize,
    part_sizes: &[usize],
    cube: &MapCube,
    result: &mut BTreeSet<usize>,
) -> MapResult<()> {
    for value in &cube.parts()[variable] {
        let offset = offset
            .checked_mul(part_sizes[variable])
            .and_then(|offset| offset.checked_add(*value))
            .ok_or(MapError::MintermCountOverflow)?;

        if variable == 0 {
            result.insert(offset);
        } else {
            explode_cube(variable - 1, offset, part_sizes, cube, result)?;
        }
    }

    Ok(())
}

fn map_cells(
    map_row: &[usize; 16],
    input_offset: usize,
    largest_input_index: usize,
    output_offset: usize,
    minterms: &BTreeSet<usize>,
) -> Option<Vec<bool>> {
    let mut row = Vec::new();
    for index in map_row {
        let input_index = index + input_offset;
        if input_index < largest_input_index {
            row.push(minterms.contains(&(input_index + output_offset)));
        }
    }

    if row.is_empty() {
        None
    } else {
        Some(row)
    }
}

fn input_offsets(input_count: usize) -> Vec<usize> {
    if input_count <= 8 {
        return vec![0];
    }

    (0..=(input_count - 8)).map(|index| index * 256).collect()
}

fn ensure_binary_inputs(cover: &MapCover) -> MapResult<()> {
    for (variable, part_size) in cover
        .part_sizes()
        .iter()
        .take(cover.input_count())
        .enumerate()
    {
        if *part_size != 2 {
            return Err(MapError::NonBinaryInput {
                variable,
                part_size: *part_size,
            });
        }
    }

    checked_power_of_two(cover.input_count())?;
    Ok(())
}

fn checked_power_of_two(input_count: usize) -> MapResult<usize> {
    let max_supported = usize::BITS as usize - 1;
    if input_count > max_supported {
        return Err(MapError::TooManyInputs {
            input_count,
            max_supported,
        });
    }

    Ok(1usize << input_count)
}

fn checked_minterm_count(part_sizes: &[usize]) -> MapResult<usize> {
    part_sizes
        .iter()
        .try_fold(1usize, |accumulator, part_size| {
            accumulator
                .checked_mul(*part_size)
                .ok_or(MapError::MintermCountOverflow)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minterms_expand_selected_parts_in_mixed_radix_order() {
        let cover = MapCover::from_rows(
            vec![2, 2, 2],
            [
                vec![vec![1], vec![0], vec![1]],
                vec![vec![0, 1], vec![1], vec![0]],
            ],
        )
        .unwrap();

        assert_eq!(minterms(&cover).unwrap(), set(&[2, 3, 5]));
    }

    #[test]
    fn minterms_skip_cubes_with_empty_selected_parts() {
        let cover = MapCover::from_rows(vec![2, 2], [vec![vec![], vec![1]]]).unwrap();

        assert!(minterms(&cover).unwrap().is_empty());
    }

    #[test]
    fn karnaugh_map_uses_gray_order_for_binary_inputs() {
        let cover = MapCover::from_rows(
            vec![2, 2, 2, 1],
            [vec![vec![0, 1], vec![0], vec![1], vec![0]]],
        )
        .unwrap();

        let map = karnaugh_map(&cover).unwrap();
        let rows = map[0].blocks()[0].rows();

        assert_eq!(rows[0].cells(), &[false, false, false, false]);
        assert_eq!(rows[1].cells(), &[true, true, false, false]);
    }

    #[test]
    fn karnaugh_map_keeps_one_output_space_per_output_part() {
        let cover = MapCover::from_rows(
            vec![2, 2, 2],
            [
                vec![vec![1], vec![0], vec![0]],
                vec![vec![0], vec![1], vec![1]],
            ],
        )
        .unwrap();

        let map = karnaugh_map(&cover).unwrap();

        assert_eq!(map.len(), 2);
        assert_eq!(
            map[0].blocks()[0].rows()[0].cells(),
            &[false, true, false, false]
        );
        assert_eq!(
            map[1].blocks()[0].rows()[0].cells(),
            &[false, false, false, true]
        );
    }

    #[test]
    fn render_karnaugh_map_formats_output_spaces() {
        let cover = MapCover::from_rows(vec![2, 1], [vec![vec![1], vec![0]]]).unwrap();

        assert_eq!(
            render_karnaugh_map(&cover).unwrap(),
            "\n\nOutput space # 0\n.1\n"
        );
    }

    #[test]
    fn rejects_values_outside_declared_part_size() {
        assert_eq!(
            MapCover::from_rows(vec![2, 1], [vec![vec![2], vec![0]]]).unwrap_err(),
            MapError::PartValueOutOfRange {
                variable: 0,
                value: 2,
                part_size: 2,
            }
        );
    }

    #[test]
    fn rejects_non_binary_inputs_for_karnaugh_rendering() {
        let cover = MapCover::from_rows(vec![3, 1], [vec![vec![0], vec![0]]]).unwrap();

        assert_eq!(
            karnaugh_map(&cover).unwrap_err(),
            MapError::NonBinaryInput {
                variable: 0,
                part_size: 3,
            }
        );
    }

    #[test]
    fn no_legacy_c_abi_tokens_or_source_dependency_metadata_are_present() {
        let source = include_str!("map.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("LogicFriday1", "-")));
    }

    fn set(values: &[usize]) -> BTreeSet<usize> {
        values.iter().copied().collect()
    }
}
