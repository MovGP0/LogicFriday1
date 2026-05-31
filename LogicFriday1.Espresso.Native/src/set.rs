//! Source-aligned Rust surface for Espresso `set.c`.
//!
//! The C implementation stores sets as packed `unsigned int` arrays with the
//! loop count in word zero.  The native port keeps the same 32-bit bit order in
//! [`BitSet`] and stores the metadata in Rust fields instead of embedding it in
//! the data words.

use crate::foundation::{ACTIVE, BitSet, SetFamily, bit_index as foundation_bit_index};

#[allow(unused_imports)]
pub use crate::foundation::{BPI, LOGBPI, loopinit, set_size, which_bit, which_word};

/// C `bit_index`: first set bit from the least significant bit.
pub fn bit_index(value: u32) -> isize {
    foundation_bit_index(value).map_or(-1, |index| index as isize)
}

/// C `set_new`/`set_clear`: create an empty set of `size` elements.
pub fn set_clear(size: usize) -> BitSet {
    BitSet::empty(size)
}

/// C `set_fill`: create the universal set of `size` elements.
pub fn set_fill(size: usize) -> BitSet {
    BitSet::full(size)
}

/// C `set_copy`.
pub fn set_copy(a: &BitSet) -> BitSet {
    a.clone()
}

/// C `set_ord`.
pub fn set_ord(a: &BitSet) -> usize {
    a.ord()
}

/// C `set_dist`: count common elements.
pub fn set_dist(a: &BitSet, b: &BitSet) -> usize {
    a.distance(b)
}

/// C `set_and`.
pub fn set_and(a: &BitSet, b: &BitSet) -> BitSet {
    a.and(b)
}

/// C `set_or`.
pub fn set_or(a: &BitSet, b: &BitSet) -> BitSet {
    a.or(b)
}

/// C `set_diff`.
pub fn set_diff(a: &BitSet, b: &BitSet) -> BitSet {
    a.diff(b)
}

/// C `set_xor`.
pub fn set_xor(a: &BitSet, b: &BitSet) -> BitSet {
    a.xor(b)
}

/// C `set_merge`: `(a & mask) | (b & !mask)`.
pub fn set_merge(a: &BitSet, b: &BitSet, mask: &BitSet) -> BitSet {
    a.merge(b, mask)
}

/// C `set_andp`: intersection plus the non-empty result predicate.
pub fn set_andp(a: &BitSet, b: &BitSet) -> (BitSet, bool) {
    let result = set_and(a, b);
    let nonempty = !result.is_empty();
    (result, nonempty)
}

/// C `set_orp`: union plus the non-empty result predicate.
pub fn set_orp(a: &BitSet, b: &BitSet) -> (BitSet, bool) {
    let result = set_or(a, b);
    let nonempty = !result.is_empty();
    (result, nonempty)
}

/// C `setp_empty`.
pub fn setp_empty(a: &BitSet) -> bool {
    a.is_empty()
}

/// C `setp_full`.
pub fn setp_full(a: &BitSet, size: usize) -> bool {
    a.size() == size && a.is_full()
}

/// C `setp_equal`.
pub fn setp_equal(a: &BitSet, b: &BitSet) -> bool {
    a == b
}

/// C `setp_disjoint`.
pub fn setp_disjoint(a: &BitSet, b: &BitSet) -> bool {
    a.is_disjoint(b)
}

/// C `setp_implies`: true when `b` contains every element in `a`.
pub fn setp_implies(a: &BitSet, b: &BitSet) -> bool {
    a.implies(b)
}

/// C `sf_new`.
pub fn sf_new(capacity: usize, size: usize) -> SetFamily {
    SetFamily::new(capacity, size)
}

/// C `sf_save`/`sf_copy`.
pub fn sf_copy(a: &SetFamily) -> SetFamily {
    a.clone()
}

/// C `sf_join`.
pub fn sf_join(a: &SetFamily, b: &SetFamily) -> SetFamily {
    assert_eq!(a.set_size(), b.set_size(), "sf_join: sf_size mismatch");
    let mut joined = a.clone();
    joined.append(b.clone());
    joined
}

/// C `sf_append`.
pub fn sf_append(a: &mut SetFamily, b: SetFamily) {
    assert_eq!(a.set_size(), b.set_size(), "sf_append: sf_size mismatch");
    a.append(b);
}

/// C `sf_addset`.
pub fn sf_addset(a: &mut SetFamily, set: BitSet) {
    a.add_set(set);
}

/// C `sf_delset`: replace the deleted row with the final row.
pub fn sf_delset(a: &mut SetFamily, index: usize) {
    a.delete_set(index);
}

/// C `sf_or`.
pub fn sf_or(a: &SetFamily) -> BitSet {
    a.union_all()
}

/// C `sf_and`.
pub fn sf_and(a: &SetFamily) -> BitSet {
    a.intersect_all()
}

/// C `sf_active`: mark every member active.
pub fn sf_active(a: &SetFamily) -> SetFamily {
    SetFamily::from_sets(
        a.set_size(),
        a.sets().iter().cloned().map(|mut set| {
            set.set_flag(ACTIVE);
            set
        }),
    )
}

/// C `sf_inactive`: compact away inactive members, keeping active rows.
pub fn sf_inactive(a: &SetFamily) -> SetFamily {
    SetFamily::from_sets(
        a.set_size(),
        a.sets().iter().filter(|set| set.test_flag(ACTIVE)).cloned(),
    )
}

/// C `sf_count`.
pub fn sf_count(a: &SetFamily) -> Vec<usize> {
    a.count_columns()
}

/// C `sf_count_restricted`.
pub fn sf_count_restricted(a: &SetFamily, restriction: &BitSet) -> Vec<usize> {
    a.count_columns_restricted(restriction)
}

/// C `set_adjcnt`.
pub fn set_adjcnt(a: &BitSet, counts: &mut [isize], weight: isize) {
    assert!(
        counts.len() >= a.size(),
        "set_adjcnt count vector shorter than set"
    );
    for element in a.elements() {
        counts[element] += weight;
    }
}

/// C `sf_delc`.
pub fn sf_delc(a: SetFamily, first: usize, last: usize) -> SetFamily {
    sf_delcol(a, first, (last - first + 1) as isize)
}

/// C `sf_addcol`.
pub fn sf_addcol(a: SetFamily, first_col: usize, count: usize) -> SetFamily {
    a.add_columns(first_col, count)
}

/// C `sf_delcol`: delete `count` columns when positive, insert blank columns
/// when negative.
pub fn sf_delcol(a: SetFamily, first_col: usize, count: isize) -> SetFamily {
    a.delete_columns(first_col, count)
}

/// C `sf_copy_col`.
pub fn sf_copy_col(dst: &SetFamily, dst_col: usize, src: &SetFamily, src_col: usize) -> SetFamily {
    assert_eq!(dst.len(), src.len(), "sf_copy_col row count mismatch");
    let mut result = dst.clone();
    for row in 0..src.len() {
        if src.get(row).contains(src_col) {
            result.get_mut(row).insert(dst_col);
        }
    }
    result
}

/// C `sf_compress`.
pub fn sf_compress(a: SetFamily, columns: &BitSet) -> SetFamily {
    a.compress(columns)
}

/// C `sf_transpose`.
pub fn sf_transpose(a: SetFamily) -> SetFamily {
    a.transpose()
}

/// C `sf_permute`.
pub fn sf_permute(a: SetFamily, permutation: &[usize]) -> SetFamily {
    a.permute(permutation)
}

/// C `ps1`.
pub fn ps1(a: &BitSet) -> String {
    a.to_element_string()
}

/// C `pbv1`.
pub fn pbv1(a: &BitSet, n: usize) -> String {
    assert!(n <= a.size(), "pbv1 length exceeds set size");
    (0..n)
        .map(|element| if a.contains(element) { '1' } else { '0' })
        .collect()
}

/// C `set_write`, returned as text instead of writing to a `FILE *`.
pub fn set_write_string(a: &BitSet) -> String {
    let mut fields = Vec::with_capacity(a.loop_index() + 1);
    fields.push(format!("{:x}", a.loop_index()));
    fields.extend(a.words().iter().map(|word| format!("{word:x}")));
    fields.join(" ") + "\n"
}

/// C `sf_write`, returned as text instead of writing to a `FILE *`.
pub fn sf_write_string(a: &SetFamily) -> String {
    let mut output = format!("{} {}\n", a.len(), a.set_size());
    for set in a.sets() {
        output.push_str(&set_write_string(set));
    }
    output
}

/// C `sf_read`, reading the text emitted by [`sf_write_string`].
pub fn sf_read_string(input: &str) -> Result<SetFamily, String> {
    let mut fields = input.split_whitespace();
    let rows = parse_decimal(&mut fields, "row count")?;
    let size = parse_decimal(&mut fields, "set size")?;
    let mut family = SetFamily::new(rows, size);
    for row in 0..rows {
        family.add_set(read_packed_set(&mut fields, size, row)?);
    }
    Ok(family)
}

/// C `sf_bm_print`, returned as bit-matrix lines.
pub fn sf_bm_print_lines(a: &SetFamily) -> Vec<String> {
    a.sets()
        .iter()
        .enumerate()
        .map(|(index, set)| format!("[{index:4}] {}", pbv1(set, a.set_size())))
        .collect()
}

/// C `sf_print`, returned as element-list lines.
pub fn sf_print_lines(a: &SetFamily) -> Vec<String> {
    a.sets()
        .iter()
        .enumerate()
        .map(|(index, set)| format!("A[{index}] = {}", ps1(set)))
        .collect()
}

/// C `sf_bm_read`, reading a header of `rows cols` followed by `0`/`1` rows.
pub fn sf_bm_read_string(input: &str) -> Result<SetFamily, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "missing bit-matrix header".to_string())?;
    let mut header_fields = header.split_whitespace();
    let rows = parse_decimal(&mut header_fields, "row count")?;
    let cols = parse_decimal(&mut header_fields, "column count")?;
    let mut family = SetFamily::new(rows, cols);
    for row in 0..rows {
        let line = lines
            .next()
            .ok_or_else(|| format!("missing bit-matrix row {row}"))?;
        if line.chars().count() != cols {
            return Err(format!("bit-matrix row {row} has wrong width"));
        }
        let mut set = BitSet::empty(cols);
        for (col, ch) in line.chars().enumerate() {
            match ch {
                '0' => {}
                '1' => set.insert(col),
                _ => return Err(format!("invalid bit-matrix character {ch:?}")),
            }
        }
        family.add_set(set);
    }
    Ok(family)
}

fn read_packed_set<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    size: usize,
    row: usize,
) -> Result<BitSet, String> {
    let stored_loop = parse_hex(fields, "set loop word")? as usize;
    let expected_loop = loopinit(size);
    if stored_loop != expected_loop {
        return Err(format!(
            "row {row} loop word {stored_loop} does not match expected {expected_loop}"
        ));
    }

    let mut set = BitSet::empty(size);
    for word in 1..=expected_loop {
        let value = parse_hex(fields, "set data word")?;
        set.set_word(word, value);
    }
    Ok(set)
}

fn parse_decimal<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    name: &str,
) -> Result<usize, String> {
    fields
        .next()
        .ok_or_else(|| format!("missing {name}"))?
        .parse::<usize>()
        .map_err(|err| format!("invalid {name}: {err}"))
}

fn parse_hex<'a>(fields: &mut impl Iterator<Item = &'a str>, name: &str) -> Result<u32, String> {
    let field = fields.next().ok_or_else(|| format!("missing {name}"))?;
    u32::from_str_radix(field, 16).map_err(|err| format!("invalid {name}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_bit_operations_match_set_c_semantics() {
        let a = BitSet::from_indices(35, [0, 2, 32, 34]);
        let b = BitSet::from_indices(35, [1, 2, 33, 34]);
        let mask = BitSet::from_indices(35, [0, 1, 2, 32]);

        assert_eq!(bit_index(0), -1);
        assert_eq!(bit_index(0b1000), 3);
        assert_eq!(set_ord(&a), 4);
        assert_eq!(set_dist(&a, &b), 2);
        assert_eq!(set_and(&a, &b).elements().collect::<Vec<_>>(), vec![2, 34]);
        assert_eq!(set_or(&a, &b).ord(), 6);
        assert_eq!(set_diff(&a, &b).elements().collect::<Vec<_>>(), vec![0, 32]);
        assert_eq!(set_xor(&a, &b).ord(), 4);
        assert_eq!(
            set_merge(&a, &b, &mask).elements().collect::<Vec<_>>(),
            vec![0, 2, 32, 33, 34]
        );
        assert!(set_andp(&a, &b).1);
        assert!(set_orp(&a, &b).1);
        assert!(setp_implies(&BitSet::from_indices(35, [2]), &a));
        assert!(setp_disjoint(&BitSet::from_indices(35, [4]), &a));
    }

    #[test]
    fn set_family_operations_match_set_c_shape() {
        let mut family = sf_new(2, 5);
        sf_addset(&mut family, BitSet::from_indices(5, [0, 2, 4]));
        sf_addset(&mut family, BitSet::from_indices(5, [1, 2]));
        sf_addset(&mut family, BitSet::from_indices(5, [2, 3]));

        assert_eq!(sf_count(&family), vec![1, 1, 3, 1, 1]);
        assert_eq!(sf_or(&family).to_bit_string(), "11111");
        assert_eq!(sf_and(&family).elements().collect::<Vec<_>>(), vec![2]);

        let compressed = sf_compress(family.clone(), &BitSet::from_indices(5, [2, 4]));
        assert_eq!(compressed.set_size(), 2);
        assert_eq!(pbv1(compressed.get(0), 2), "11");
        assert_eq!(pbv1(compressed.get(1), 2), "10");

        let transposed = sf_transpose(family.clone());
        assert_eq!(transposed.len(), 5);
        assert_eq!(
            transposed.get(2).elements().collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let active = sf_active(&family);
        assert_eq!(active.active_count(), family.len());
        assert_eq!(sf_inactive(&active).len(), family.len());
    }

    #[test]
    fn set_family_text_round_trips_packed_and_bit_matrix_forms() {
        let family = SetFamily::from_sets(
            6,
            [
                BitSet::from_indices(6, [0, 3, 5]),
                BitSet::from_indices(6, [1, 2]),
            ],
        );

        let packed = sf_write_string(&family);
        assert_eq!(sf_read_string(&packed).unwrap(), family);
        assert_eq!(sf_print_lines(&family)[0], "A[0] = [0,3,5]");
        assert_eq!(sf_bm_print_lines(&family)[1], "[   1] 011000");

        let matrix = "2 4\n1001\n0110\n";
        let parsed = sf_bm_read_string(matrix).unwrap();
        assert_eq!(parsed.get(0).elements().collect::<Vec<_>>(), vec![0, 3]);
        assert_eq!(parsed.get(1).elements().collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn column_insertion_deletion_and_copy_follow_set_c_contracts() {
        let family = SetFamily::from_sets(
            4,
            [
                BitSet::from_indices(4, [0, 2]),
                BitSet::from_indices(4, [1, 3]),
            ],
        );

        let inserted = sf_addcol(family.clone(), 2, 1);
        assert_eq!(inserted.set_size(), 5);
        assert_eq!(inserted.get(0).elements().collect::<Vec<_>>(), vec![0, 3]);

        let deleted = sf_delc(inserted, 1, 2);
        assert_eq!(deleted.set_size(), 3);
        assert_eq!(deleted.get(0).elements().collect::<Vec<_>>(), vec![0, 1]);

        let copied = sf_copy_col(&SetFamily::new(2, 4).join(family.clone()), 0, &family, 3);
        assert!(copied.get(1).contains(0));
    }
}
