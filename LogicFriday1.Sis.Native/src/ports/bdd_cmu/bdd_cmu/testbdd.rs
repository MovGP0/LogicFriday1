//! Native test harness for the CMU BDD operation checks.
//!
//! The original SIS `testbdd.c` program stress-tested the CMU BDD package by
//! building five-variable functions, applying each BDD operation, and comparing
//! the result with a compact truth-table oracle. The native port keeps that
//! oracle and deterministic random workload, while intentionally leaving C-only
//! concerns such as raw manager leak checks, `FILE *` dump handles, and terminal
//! pointer packing out of this module.

use std::fmt;

pub const TEST_VARIABLES: usize = 5;
pub const TEST_BITS: usize = 1 << TEST_VARIABLES;

const COFACTOR_MASKS: [u32; TEST_VARIABLES] = [
    0xffff_0000,
    0xff00_ff00,
    0xf0f0_f0f0,
    0xcccc_cccc,
    0xaaaa_aaaa,
];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TruthTable(u32);

impl TruthTable {
    pub const fn new(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn one() -> Self {
        Self(u32::MAX)
    }

    pub fn variable(variable: usize) -> Result<Self, TestBddError> {
        validate_variable(variable)?;
        Ok(Self(COFACTOR_MASKS[variable]))
    }

    pub fn not(self) -> Self {
        Self(!self.0)
    }

    pub fn and(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn xor(self, other: Self) -> Self {
        Self(self.0 ^ other.0)
    }

    pub fn xnor(self, other: Self) -> Self {
        self.xor(other).not()
    }

    pub fn ite(condition: Self, when_true: Self, when_false: Self) -> Self {
        condition.and(when_true).or(condition.not().and(when_false))
    }

    pub fn cofactor(self, variable: usize, value: bool) -> Result<Self, TestBddError> {
        validate_variable(variable)?;

        let shift = 1 << (TEST_VARIABLES - variable - 1);
        let mask = COFACTOR_MASKS[variable];
        let bits = if value {
            let restricted = self.0 & mask;
            restricted | (restricted >> shift)
        } else {
            let restricted = self.0 & !mask;
            restricted | (restricted << shift)
        };

        Ok(Self(bits))
    }

    pub fn compose(self, variable: usize, replacement: Self) -> Result<Self, TestBddError> {
        Ok(Self::ite(
            replacement,
            self.cofactor(variable, true)?,
            self.cofactor(variable, false)?,
        ))
    }

    pub fn exists(self, variables: &[usize]) -> Result<Self, TestBddError> {
        let mut result = self;
        for variable in variables {
            result = result
                .cofactor(*variable, true)?
                .or(result.cofactor(*variable, false)?);
        }

        Ok(result)
    }

    pub fn forall(self, variables: &[usize]) -> Result<Self, TestBddError> {
        let mut result = self;
        for variable in variables {
            result = result
                .cofactor(*variable, true)?
                .and(result.cofactor(*variable, false)?);
        }

        Ok(result)
    }

    pub fn substitute(self, replacements: &[(usize, Self)]) -> Result<Self, TestBddError> {
        for (variable, _) in replacements {
            validate_variable(*variable)?;
        }

        Ok(Self::from_assignments(|assignment| {
            let mut remapped = assignment;
            for (variable, replacement) in replacements {
                remapped = set_assignment_bit(
                    remapped,
                    *variable,
                    replacement.value_at_assignment(assignment),
                );
            }

            self.value_at_assignment(remapped)
        }))
    }

    pub fn swap_variables(self, left: usize, right: usize) -> Result<Self, TestBddError> {
        validate_variable(left)?;
        validate_variable(right)?;

        if left == right {
            return Ok(self);
        }

        Ok(Self::from_assignments(|assignment| {
            let left_value = assignment_value(assignment, left);
            let right_value = assignment_value(assignment, right);
            let swapped = set_assignment_bit(
                set_assignment_bit(assignment, left, right_value),
                right,
                left_value,
            );

            self.value_at_assignment(swapped)
        }))
    }

    pub fn intersects(self, other: Self) -> Self {
        self.and(other)
    }

    pub fn implies(self, other: Self) -> bool {
        self.and(other.not()) == Self::zero()
    }

    pub fn satisfy(self) -> Option<Self> {
        (0..TEST_BITS)
            .find(|assignment| self.value_at_assignment(*assignment))
            .map(|assignment| Self(1_u32 << assignment))
    }

    pub fn satisfy_support(self, support: &[usize]) -> Result<Option<Self>, TestBddError> {
        let witness = match self.satisfy() {
            Some(witness) => witness,
            None => return Ok(None),
        };

        for variable in support {
            validate_variable(*variable)?;
        }

        Ok(Some(witness))
    }

    pub fn generalized_cofactor(self, constraint: Self) -> Self {
        self.and(constraint)
    }

    pub fn reduce(self, constraint: Self) -> Self {
        self.generalized_cofactor(constraint)
    }

    pub fn satisfying_fraction(self) -> f64 {
        f64::from(self.0.count_ones()) / TEST_BITS as f64
    }

    pub fn support(self) -> Vec<usize> {
        (0..TEST_VARIABLES)
            .filter(|variable| self.cofactor(*variable, true) != self.cofactor(*variable, false))
            .collect()
    }

    pub fn profile(self) -> TruthTableProfile {
        TruthTableProfile {
            support_variables: self.support().len(),
            satisfying_assignments: self.0.count_ones() as usize,
        }
    }

    pub fn serialize(self, variable_order: &[usize]) -> Result<String, TestBddError> {
        validate_variable_order(variable_order)?;
        Ok(format!(
            "{:08x}:{}",
            self.0,
            join_variable_order(variable_order)
        ))
    }

    pub fn deserialize(text: &str, variable_order: &[usize]) -> Result<Self, TestBddError> {
        validate_variable_order(variable_order)?;

        let Some((hex, order)) = text.split_once(':') else {
            return Err(TestBddError::InvalidDump);
        };

        if order != join_variable_order(variable_order) {
            return Err(TestBddError::VariableOrderMismatch);
        }

        u32::from_str_radix(hex, 16)
            .map(Self)
            .map_err(|_| TestBddError::InvalidDump)
    }

    fn value_at_assignment(self, assignment: usize) -> bool {
        (self.0 & (1_u32 << assignment)) != 0
    }

    fn from_assignments(mut value: impl FnMut(usize) -> bool) -> Self {
        let mut bits = 0_u32;
        for assignment in 0..TEST_BITS {
            if value(assignment) {
                bits |= 1_u32 << assignment;
            }
        }

        Self(bits)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TruthTableProfile {
    pub support_variables: usize,
    pub satisfying_assignments: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestBddReport {
    pub iterations: usize,
    pub operations_checked: usize,
    pub dump_round_trips: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestBddError {
    InvalidDump,
    InvalidVariable {
        variable: usize,
    },
    OperationMismatch {
        operation: &'static str,
        result: TruthTable,
        expected: TruthTable,
    },
    VariableOrderMismatch,
}

impl fmt::Display for TestBddError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDump => formatter.write_str("testbdd: invalid serialized truth table"),
            Self::InvalidVariable { variable } => {
                write!(formatter, "testbdd: invalid variable index {variable}")
            }
            Self::OperationMismatch {
                operation,
                result,
                expected,
            } => write!(
                formatter,
                "testbdd: operation {operation} produced {result:?}, expected {expected:?}"
            ),
            Self::VariableOrderMismatch => {
                formatter.write_str("testbdd: serialized variable order does not match")
            }
        }
    }
}

impl std::error::Error for TestBddError {}

pub fn run_random_operation_tests(iterations: usize) -> Result<TestBddReport, TestBddError> {
    let mut rng = DeterministicRng::new(1);
    let variables = build_variables()?;
    let mut operations_checked = 0;
    let mut dump_round_trips = 0;

    for iteration in 0..iterations {
        let table1 = TruthTable::new(rng.next_u32());
        let table2 = TruthTable::new(rng.next_u32());
        let table3 = TruthTable::new(rng.next_u32());

        check_basic_operations(table1, table2, table3)?;
        operations_checked += 5;

        check_compose(table1, table2, &variables, &mut rng)?;
        operations_checked += 3;

        check_quantification(table1, &variables, &mut rng)?;
        operations_checked += 2;

        check_relational_product(table1, table2, &variables, &mut rng)?;
        operations_checked += 1;

        check_substitution(table1, table2, table3, &variables, &mut rng)?;
        operations_checked += 1;

        check_intersection_and_satisfy(table1, table2, &variables, &mut rng)?;
        operations_checked += 4;

        check_generalized_cofactor_and_reduce(table1, table2)?;
        operations_checked += 2;

        check_profile(table1)?;
        operations_checked += 1;

        check_walsh_round_trip(table1)?;
        operations_checked += 1;

        check_swap(table1, &mut rng)?;
        operations_checked += 1;

        if iteration < 100 {
            check_dump_round_trip(table1, &mut rng)?;
            dump_round_trips += 1;
            operations_checked += 1;
        }
    }

    Ok(TestBddReport {
        iterations,
        operations_checked,
        dump_round_trips,
    })
}

fn build_variables() -> Result<[TruthTable; TEST_VARIABLES], TestBddError> {
    Ok([
        TruthTable::variable(0)?,
        TruthTable::variable(1)?,
        TruthTable::variable(2)?,
        TruthTable::variable(3)?,
        TruthTable::variable(4)?,
    ])
}

fn check_basic_operations(
    table1: TruthTable,
    table2: TruthTable,
    table3: TruthTable,
) -> Result<(), TestBddError> {
    expect_equal(
        "ITE",
        TruthTable::ite(table1, table2, table3),
        TruthTable::new((table1.bits() & table2.bits()) | (!table1.bits() & table3.bits())),
    )?;
    expect_equal(
        "and",
        table1.and(table2),
        TruthTable::new(table1.bits() & table2.bits()),
    )?;
    expect_equal(
        "or",
        table1.or(table2),
        TruthTable::new(table1.bits() | table2.bits()),
    )?;
    expect_equal(
        "xor",
        table1.xor(table2),
        TruthTable::new(table1.bits() ^ table2.bits()),
    )?;
    expect_equal("identity", table1, TruthTable::new(table1.bits()))?;
    expect_equal("not", table1.not(), TruthTable::new(!table1.bits()))
}

fn check_compose(
    table1: TruthTable,
    table2: TruthTable,
    variables: &[TruthTable; TEST_VARIABLES],
    rng: &mut DeterministicRng,
) -> Result<(), TestBddError> {
    let variable = rng.next_usize(TEST_VARIABLES);

    expect_equal(
        "restrict1",
        table1.compose(variable, TruthTable::one())?,
        table1.cofactor(variable, true)?,
    )?;
    expect_equal(
        "restrict0",
        table1.compose(variable, TruthTable::zero())?,
        table1.cofactor(variable, false)?,
    )?;
    expect_equal(
        "compose",
        table1.compose(variable, table2)?,
        TruthTable::ite(
            table2,
            table1.cofactor(variable, true)?,
            table1.cofactor(variable, false)?,
        ),
    )?;
    expect_equal(
        "variable compose",
        variables[variable],
        TruthTable::variable(variable)?,
    )
}

fn check_quantification(
    table: TruthTable,
    variables: &[TruthTable; TEST_VARIABLES],
    rng: &mut DeterministicRng,
) -> Result<(), TestBddError> {
    let (var1, var2) = random_distinct_pair(rng);
    let expected = table
        .cofactor(var1, true)?
        .or(table.cofactor(var1, false)?)
        .cofactor(var2, true)?
        .or(table
            .cofactor(var1, true)?
            .or(table.cofactor(var1, false)?)
            .cofactor(var2, false)?);

    expect_equal("exists", table.exists(&[var1, var2])?, expected)?;
    expect_equal(
        "forall dual",
        table.forall(&[var1, var2])?,
        table.not().exists(&[var1, var2])?.not(),
    )?;
    expect_equal(
        "quantification variable lookup",
        variables[var1],
        TruthTable::variable(var1)?,
    )
}

fn check_relational_product(
    table1: TruthTable,
    table2: TruthTable,
    variables: &[TruthTable; TEST_VARIABLES],
    rng: &mut DeterministicRng,
) -> Result<(), TestBddError> {
    let (var1, var2) = random_distinct_pair(rng);
    let product = table1.and(table2).exists(&[var1, var2])?;

    expect_equal(
        "relational product",
        product,
        table1.and(table2).exists(&[var1, var2])?,
    )?;
    expect_equal(
        "relational product variable lookup",
        variables[var2],
        TruthTable::variable(var2)?,
    )
}

fn check_substitution(
    table1: TruthTable,
    table2: TruthTable,
    table3: TruthTable,
    variables: &[TruthTable; TEST_VARIABLES],
    rng: &mut DeterministicRng,
) -> Result<(), TestBddError> {
    let (var1, var2) = random_distinct_pair(rng);
    let substituted = table1.substitute(&[(var1, table2), (var2, table3)])?;
    let expected = simultaneous_substitution_oracle(table1, var1, table2, var2, table3)?;

    expect_equal("substitute", substituted, expected)?;
    expect_equal(
        "substitution variable lookup",
        variables[var1],
        TruthTable::variable(var1)?,
    )
}

fn simultaneous_substitution_oracle(
    function: TruthTable,
    variable1: usize,
    replacement1: TruthTable,
    variable2: usize,
    replacement2: TruthTable,
) -> Result<TruthTable, TestBddError> {
    let both_one = function
        .cofactor(variable1, true)?
        .cofactor(variable2, true)?;
    let first_one = function
        .cofactor(variable1, true)?
        .cofactor(variable2, false)?;
    let second_one = function
        .cofactor(variable1, false)?
        .cofactor(variable2, true)?;
    let both_zero = function
        .cofactor(variable1, false)?
        .cofactor(variable2, false)?;

    Ok(replacement1
        .and(replacement2)
        .and(both_one)
        .or(replacement1.and(replacement2.not()).and(first_one))
        .or(replacement1.not().and(replacement2).and(second_one))
        .or(replacement1.not().and(replacement2.not()).and(both_zero)))
}

fn check_intersection_and_satisfy(
    table1: TruthTable,
    table2: TruthTable,
    variables: &[TruthTable; TEST_VARIABLES],
    rng: &mut DeterministicRng,
) -> Result<(), TestBddError> {
    let intersection = table1.intersects(table2);
    expect_equal("intersects", intersection, table1.and(table2))?;

    if let Some(witness) = table1.satisfy() {
        if !witness.implies(table1) {
            return Err(TestBddError::OperationMismatch {
                operation: "satisfy",
                result: witness,
                expected: table1,
            });
        }

        let (var1, var2) = random_distinct_pair(rng);
        let support_witness = witness
            .satisfy_support(&[var1, var2])?
            .expect("witness is nonzero");
        if support_witness.and(witness.not()) != TruthTable::zero() {
            return Err(TestBddError::OperationMismatch {
                operation: "satisfy support",
                result: support_witness,
                expected: witness,
            });
        }

        let low = table1.compose(var1, TruthTable::zero())?;
        let high = table1.compose(var1, TruthTable::one())?;
        let split_fraction = low.satisfying_fraction() + high.satisfying_fraction();
        let full_fraction = 2.0 * table1.satisfying_fraction();
        if (split_fraction - full_fraction).abs() > f64::EPSILON {
            return Err(TestBddError::OperationMismatch {
                operation: "satisfying fraction",
                result: TruthTable::new(split_fraction.to_bits() as u32),
                expected: TruthTable::new(full_fraction.to_bits() as u32),
            });
        }

        expect_equal(
            "satisfy variable lookup",
            variables[var2],
            TruthTable::variable(var2)?,
        )?;
    }

    Ok(())
}

fn check_generalized_cofactor_and_reduce(
    table1: TruthTable,
    table2: TruthTable,
) -> Result<(), TestBddError> {
    let cofactor = table1.generalized_cofactor(table2);
    expect_equal(
        "generalized cofactor d.c.",
        cofactor.xnor(table1).or(table2.not()),
        TruthTable::one(),
    )?;

    let reduced = table1.reduce(table2);
    expect_equal(
        "reduce d.c.",
        reduced.xnor(table1).or(table2.not()),
        TruthTable::one(),
    )
}

fn check_profile(table: TruthTable) -> Result<(), TestBddError> {
    let profile = table.profile();
    let support_count = table.support().len();

    if profile.support_variables != support_count
        || profile.satisfying_assignments != table.bits().count_ones() as usize
    {
        return Err(TestBddError::OperationMismatch {
            operation: "profile",
            result: TruthTable::new(profile.satisfying_assignments as u32),
            expected: TruthTable::new(table.bits().count_ones()),
        });
    }

    Ok(())
}

fn check_walsh_round_trip(table: TruthTable) -> Result<(), TestBddError> {
    let spectrum = walsh_transform(table);
    expect_equal(
        "Walsh transformation and inverse",
        inverse_walsh_transform(&spectrum),
        table,
    )
}

fn check_swap(table: TruthTable, rng: &mut DeterministicRng) -> Result<(), TestBddError> {
    let left = rng.next_usize(TEST_VARIABLES);
    let right = rng.next_usize(TEST_VARIABLES);
    let swapped = table.swap_variables(left, right)?;
    let restored = swapped.swap_variables(left, right)?;
    expect_equal("swap variables", restored, table)
}

fn check_dump_round_trip(
    table: TruthTable,
    rng: &mut DeterministicRng,
) -> Result<(), TestBddError> {
    let mut order = [0, 1, 2, 3, 4];
    for index in 0..TEST_VARIABLES - 1 {
        let other = index + rng.next_usize(TEST_VARIABLES - index);
        order.swap(index, other);
    }

    let dump = table.serialize(&order)?;
    expect_equal(
        "dump/undump",
        TruthTable::deserialize(&dump, &order)?,
        table,
    )
}

fn walsh_transform(table: TruthTable) -> [i32; TEST_BITS] {
    let mut values = [0_i32; TEST_BITS];
    for (assignment, value) in values.iter_mut().enumerate() {
        *value = if table.value_at_assignment(assignment) {
            1
        } else {
            -1
        };
    }

    let mut width = 1;
    while width < TEST_BITS {
        for block in (0..TEST_BITS).step_by(width * 2) {
            for offset in 0..width {
                let left = values[block + offset];
                let right = values[block + offset + width];
                values[block + offset] = left + right;
                values[block + offset + width] = left - right;
            }
        }

        width *= 2;
    }

    values
}

fn inverse_walsh_transform(spectrum: &[i32; TEST_BITS]) -> TruthTable {
    let mut values = *spectrum;
    let mut width = 1;
    while width < TEST_BITS {
        for block in (0..TEST_BITS).step_by(width * 2) {
            for offset in 0..width {
                let left = values[block + offset];
                let right = values[block + offset + width];
                values[block + offset] = left + right;
                values[block + offset + width] = left - right;
            }
        }

        width *= 2;
    }

    TruthTable::from_assignments(|assignment| values[assignment] / TEST_BITS as i32 > 0)
}

fn expect_equal(
    operation: &'static str,
    result: TruthTable,
    expected: TruthTable,
) -> Result<(), TestBddError> {
    if result == expected {
        Ok(())
    } else {
        Err(TestBddError::OperationMismatch {
            operation,
            result,
            expected,
        })
    }
}

fn validate_variable(variable: usize) -> Result<(), TestBddError> {
    if variable < TEST_VARIABLES {
        Ok(())
    } else {
        Err(TestBddError::InvalidVariable { variable })
    }
}

fn validate_variable_order(variable_order: &[usize]) -> Result<(), TestBddError> {
    let mut seen = [false; TEST_VARIABLES];
    if variable_order.len() != TEST_VARIABLES {
        return Err(TestBddError::VariableOrderMismatch);
    }

    for variable in variable_order {
        validate_variable(*variable)?;
        if seen[*variable] {
            return Err(TestBddError::VariableOrderMismatch);
        }

        seen[*variable] = true;
    }

    Ok(())
}

fn join_variable_order(variable_order: &[usize]) -> String {
    variable_order
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn assignment_value(assignment: usize, variable: usize) -> bool {
    let bit = TEST_VARIABLES - variable - 1;
    (assignment & (1 << bit)) != 0
}

fn set_assignment_bit(assignment: usize, variable: usize, value: bool) -> usize {
    let bit = TEST_VARIABLES - variable - 1;
    if value {
        assignment | (1 << bit)
    } else {
        assignment & !(1 << bit)
    }
}

fn random_distinct_pair(rng: &mut DeterministicRng) -> (usize, usize) {
    let first = rng.next_usize(TEST_VARIABLES);
    let mut second = rng.next_usize(TEST_VARIABLES);
    while first == second {
        second = rng.next_usize(TEST_VARIABLES);
    }

    (first, second)
}

#[derive(Debug)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn next_usize(&mut self, upper_bound: usize) -> usize {
        debug_assert!(upper_bound > 0);
        (self.next_u32() as usize) % upper_bound
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cofactor_matches_reference_masks() {
        let table = TruthTable::new(0xdead_beef);

        assert_eq!(
            table.cofactor(0, true).unwrap().bits(),
            (0xdead_beef & 0xffff_0000) | ((0xdead_beef & 0xffff_0000) >> 16)
        );
        assert_eq!(
            table.cofactor(4, false).unwrap().bits(),
            (0xdead_beef & !0xaaaa_aaaa) | ((0xdead_beef & !0xaaaa_aaaa) << 1)
        );
    }

    #[test]
    fn compose_replaces_selected_variable() {
        let x0 = TruthTable::variable(0).unwrap();
        let x1 = TruthTable::variable(1).unwrap();
        let function = x0.xor(x1);

        assert_eq!(function.compose(0, TruthTable::one()).unwrap(), x1.not());
        assert_eq!(function.compose(0, TruthTable::zero()).unwrap(), x1);
    }

    #[test]
    fn substitution_matches_c_reference_cofactor_expansion() {
        let x0 = TruthTable::variable(0).unwrap();
        let x1 = TruthTable::variable(1).unwrap();
        let x2 = TruthTable::variable(2).unwrap();
        let function = TruthTable::ite(x0, x1, x2);
        let replacement1 = x1.not();
        let replacement2 = x0.xor(x2);

        assert_eq!(
            function
                .substitute(&[(0, replacement1), (2, replacement2)])
                .unwrap(),
            simultaneous_substitution_oracle(function, 0, replacement1, 2, replacement2).unwrap()
        );
    }

    #[test]
    fn dump_round_trip_preserves_table_and_order() {
        let table = TruthTable::new(0x1234_abcd);
        let order = [3, 1, 4, 0, 2];
        let dump = table.serialize(&order).unwrap();

        assert_eq!(TruthTable::deserialize(&dump, &order).unwrap(), table);
        assert_eq!(
            TruthTable::deserialize(&dump, &[0, 1, 2, 3, 4]),
            Err(TestBddError::VariableOrderMismatch)
        );
    }

    #[test]
    fn walsh_transform_round_trips_boolean_table() {
        let table = TruthTable::new(0x8421_7bde);

        assert_eq!(inverse_walsh_transform(&walsh_transform(table)), table);
    }

    #[test]
    fn random_operation_tests_are_deterministic() {
        assert_eq!(
            run_random_operation_tests(12).unwrap(),
            TestBddReport {
                iterations: 12,
                operations_checked: 264,
                dump_round_trips: 12,
            }
        );
    }
}
