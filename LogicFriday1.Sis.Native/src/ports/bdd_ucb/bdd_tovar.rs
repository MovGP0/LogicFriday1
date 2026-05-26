use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

pub type BddVariableId = i32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BddTerminal {
    Zero,
    One,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BddFunction {
    id: BddVariableId,
    high: BddTerminal,
    low: BddTerminal,
    complemented: bool,
}

impl BddFunction {
    pub fn positive_variable(id: BddVariableId) -> Self {
        Self {
            id,
            high: BddTerminal::One,
            low: BddTerminal::Zero,
            complemented: false,
        }
    }

    pub fn complemented_variable(id: BddVariableId) -> Self {
        Self {
            id,
            high: BddTerminal::One,
            low: BddTerminal::Zero,
            complemented: true,
        }
    }

    pub fn branch(id: BddVariableId, high: BddTerminal, low: BddTerminal) -> Self {
        Self {
            id,
            high,
            low,
            complemented: false,
        }
    }

    pub fn id(&self) -> BddVariableId {
        self.id
    }

    pub fn high(&self) -> BddTerminal {
        self.high
    }

    pub fn low(&self) -> BddTerminal {
        self.low
    }

    pub fn is_complemented(&self) -> bool {
        self.complemented
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddToVarError {
    ComplementedVariable {
        id: BddVariableId,
    },
    ExpectedPositiveVariable {
        id: BddVariableId,
        high: BddTerminal,
        low: BddTerminal,
    },
}

impl fmt::Display for BddToVarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ComplementedVariable { id } => {
                write!(formatter, "BDD node {id} is complemented")
            }
            Self::ExpectedPositiveVariable { id, high, low } => write!(
                formatter,
                "BDD node {id} is not a positive variable: high={high:?}, low={low:?}"
            ),
        }
    }
}

impl Error for BddToVarError {}

pub fn bdd_get_varids(variables: &[BddFunction]) -> Result<Vec<BddVariableId>, BddToVarError> {
    variables.iter().map(get_var_id).collect()
}

pub fn bdd_get_sorted_varids(
    variables: &[BddFunction],
) -> Result<Vec<BddVariableId>, BddToVarError> {
    let mut result = bdd_get_varids(variables)?;
    result.sort();
    Ok(result)
}

pub fn bdd_varid_cmp(left: &BddFunction, right: &BddFunction) -> Result<Ordering, BddToVarError> {
    Ok(get_var_id(left)?.cmp(&get_var_id(right)?))
}

pub fn bdd_varid_difference(
    left: &BddFunction,
    right: &BddFunction,
) -> Result<BddVariableId, BddToVarError> {
    Ok(get_var_id(left)? - get_var_id(right)?)
}

fn get_var_id(variable: &BddFunction) -> Result<BddVariableId, BddToVarError> {
    if variable.is_complemented() {
        return Err(BddToVarError::ComplementedVariable { id: variable.id() });
    }

    if variable.high() != BddTerminal::One || variable.low() != BddTerminal::Zero {
        return Err(BddToVarError::ExpectedPositiveVariable {
            id: variable.id(),
            high: variable.high(),
            low: variable.low(),
        });
    }

    Ok(variable.id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_varids_preserves_input_order() {
        let variables = [
            BddFunction::positive_variable(9),
            BddFunction::positive_variable(2),
            BddFunction::positive_variable(6),
        ];

        assert_eq!(bdd_get_varids(&variables).unwrap(), vec![9, 2, 6]);
    }

    #[test]
    fn get_sorted_varids_returns_ascending_ids() {
        let variables = [
            BddFunction::positive_variable(9),
            BddFunction::positive_variable(2),
            BddFunction::positive_variable(6),
        ];

        assert_eq!(bdd_get_sorted_varids(&variables).unwrap(), vec![2, 6, 9]);
    }

    #[test]
    fn get_varids_accepts_empty_input() {
        assert_eq!(bdd_get_varids(&[]).unwrap(), Vec::<BddVariableId>::new());
    }

    #[test]
    fn comparator_orders_by_variable_id() {
        let left = BddFunction::positive_variable(3);
        let right = BddFunction::positive_variable(7);

        assert_eq!(bdd_varid_cmp(&left, &right).unwrap(), Ordering::Less);
        assert_eq!(bdd_varid_difference(&left, &right).unwrap(), -4);
    }

    #[test]
    fn complemented_variable_is_rejected() {
        let error = bdd_get_varids(&[BddFunction::complemented_variable(4)]).unwrap_err();

        assert_eq!(error, BddToVarError::ComplementedVariable { id: 4 });
    }

    #[test]
    fn non_variable_branch_is_rejected() {
        let error = bdd_get_varids(&[BddFunction::branch(4, BddTerminal::Zero, BddTerminal::Zero)])
            .unwrap_err();

        assert_eq!(
            error,
            BddToVarError::ExpectedPositiveVariable {
                id: 4,
                high: BddTerminal::Zero,
                low: BddTerminal::Zero,
            }
        );
    }

    #[test]
    fn sorting_stops_on_invalid_variable() {
        let variables = [
            BddFunction::positive_variable(3),
            BddFunction::branch(5, BddTerminal::One, BddTerminal::One),
        ];

        assert!(matches!(
            bdd_get_sorted_varids(&variables),
            Err(BddToVarError::ExpectedPositiveVariable { id: 5, .. })
        ));
    }

    #[test]
    fn source_contains_no_c_abi_or_dependency_metadata() {
        let source = include_str!("bdd_tovar.rs");

        for forbidden in [
            concat!("no", "_mangle"),
            concat!("extern ", "\"", "C", "\""),
            concat!("REQUIRED", "_"),
            concat!("Port", "Dependency"),
            concat!("bead", "_id"),
            concat!("source", "_file"),
            concat!("Logic", "Friday1", "-", "8j8"),
        ] {
            assert!(!source.contains(forbidden), "{forbidden}");
        }
    }
}
