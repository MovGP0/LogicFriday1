//! Native Rust BDD variable association routines corresponding to the legacy
//! CMU `bddassoc.c` unit.
//!
//! The C implementation stores reusable association vectors, keeps a temporary
//! association, and switches the current association by integer ID. This module
//! preserves those semantics with typed Rust inputs instead of accepting raw
//! sentinel-terminated arrays.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type AssociationId = i32;
pub type BddVariableId = usize;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Bdd(usize);

impl Bdd
{
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);

    pub const fn from_index(index: usize) -> Self
    {
        Self(index)
    }

    pub const fn index(self) -> usize
    {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssociationError
{
    InvalidVariable
    {
        variable: BddVariableId,
        variable_count: usize,
    },
    UnknownAssociation(AssociationId),
}

impl fmt::Display for AssociationError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::InvalidVariable {
                variable,
                variable_count,
            } => write!(
                formatter,
                "variable {variable} is outside the manager range 0..{variable_count}"
            ),
            Self::UnknownAssociation(id) => {
                write!(formatter, "no variable association with ID {id}")
            }
        }
    }
}

impl Error for AssociationError {}

pub type AssociationResult<T> = Result<T, AssociationError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableAssociation
{
    replacements: Vec<Option<Bdd>>,
    last: Option<BddVariableId>,
}

impl VariableAssociation
{
    pub fn new(variable_count: usize) -> Self
    {
        Self {
            replacements: vec![None; variable_count],
            last: None,
        }
    }

    pub fn replacement(&self, variable: BddVariableId) -> Option<Bdd>
    {
        self.replacements.get(variable).copied().flatten()
    }

    pub fn contains(&self, variable: BddVariableId) -> bool
    {
        self.replacement(variable).is_some()
    }

    pub const fn last(&self) -> Option<BddVariableId>
    {
        self.last
    }

    pub fn replacements(&self) -> &[Option<Bdd>]
    {
        &self.replacements
    }

    fn clear(&mut self)
    {
        self.replacements.fill(None);
        self.last = None;
    }

    fn set(&mut self, variable: BddVariableId, replacement: Bdd)
    {
        self.replacements[variable] = Some(replacement);
        self.last = Some(self.last.map_or(variable, |last| last.max(variable)));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredAssociation
{
    id: AssociationId,
    association: VariableAssociation,
    references: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurrentAssociation
{
    Temporary,
    Stored(AssociationId),
}

#[derive(Clone, Debug)]
pub struct BddAssociationManager
{
    variable_count: usize,
    stored_associations: Vec<StoredAssociation>,
    temporary_association: VariableAssociation,
    current_association: CurrentAssociation,
    cache_epoch: u64,
    protected_replacements: HashMap<Bdd, usize>,
}

impl BddAssociationManager
{
    pub fn new(variable_count: usize) -> Self
    {
        Self {
            variable_count,
            stored_associations: Vec::new(),
            temporary_association: VariableAssociation::new(variable_count),
            current_association: CurrentAssociation::Temporary,
            cache_epoch: 0,
            protected_replacements: HashMap::new(),
        }
    }

    pub const fn variable_count(&self) -> usize
    {
        self.variable_count
    }

    pub const fn current_selection(&self) -> CurrentAssociation
    {
        self.current_association
    }

    pub const fn cache_epoch(&self) -> u64
    {
        self.cache_epoch
    }

    pub fn current_association(&self) -> &VariableAssociation
    {
        match self.current_association {
            CurrentAssociation::Temporary => &self.temporary_association,
            CurrentAssociation::Stored(id) => {
                &self
                    .stored_associations
                    .iter()
                    .find(|association| association.id == id)
                    .expect("current association ID is kept valid")
                    .association
            }
        }
    }

    pub fn temporary_association(&self) -> &VariableAssociation
    {
        &self.temporary_association
    }

    pub fn association(&self, id: AssociationId) -> AssociationResult<&VariableAssociation>
    {
        self.stored_associations
            .iter()
            .find(|association| association.id == id)
            .map(|association| &association.association)
            .ok_or(AssociationError::UnknownAssociation(id))
    }

    pub fn reference_count(&self, id: AssociationId) -> AssociationResult<usize>
    {
        self.stored_associations
            .iter()
            .find(|association| association.id == id)
            .map(|association| association.references)
            .ok_or(AssociationError::UnknownAssociation(id))
    }

    pub fn protected_replacement_count(&self, replacement: Bdd) -> usize
    {
        self.protected_replacements
            .get(&replacement)
            .copied()
            .unwrap_or(0)
    }

    pub fn new_replacement_association<I>(&mut self, replacements: I) -> AssociationResult<AssociationId>
    where
        I: IntoIterator<Item = (BddVariableId, Bdd)>,
    {
        let mut association = VariableAssociation::new(self.variable_count);
        let mut protected = Vec::new();

        for (variable, replacement) in replacements {
            self.check_variable(variable)?;
            association.set(variable, replacement);
            protected.push(replacement);
        }

        Ok(self.intern_association(association, &protected))
    }

    pub fn new_variable_association<I>(&mut self, variables: I) -> AssociationResult<AssociationId>
    where
        I: IntoIterator<Item = BddVariableId>,
    {
        let mut association = VariableAssociation::new(self.variable_count);

        for variable in variables {
            self.check_variable(variable)?;
            association.set(variable, Bdd::ONE);
        }

        Ok(self.intern_association(association, &[]))
    }

    pub fn free_association(&mut self, id: AssociationId) -> AssociationResult<()>
    {
        if self.current_association == CurrentAssociation::Stored(id) {
            self.current_association = CurrentAssociation::Temporary;
        }

        let Some(index) = self
            .stored_associations
            .iter()
            .position(|association| association.id == id)
        else {
            return Err(AssociationError::UnknownAssociation(id));
        };

        self.stored_associations[index].references -= 1;
        if self.stored_associations[index].references == 0 {
            let association = self.stored_associations.remove(index);
            self.unprotect_association(&association.association);
            self.cache_epoch = self.cache_epoch.wrapping_add(1);
        }

        Ok(())
    }

    pub fn augment_temporary_replacements<I>(&mut self, replacements: I) -> AssociationResult<()>
    where
        I: IntoIterator<Item = (BddVariableId, Bdd)>,
    {
        for (variable, replacement) in replacements {
            self.check_variable(variable)?;
            self.replace_temporary(variable, replacement);
        }

        Ok(())
    }

    pub fn augment_temporary_variables<I>(&mut self, variables: I) -> AssociationResult<()>
    where
        I: IntoIterator<Item = BddVariableId>,
    {
        for variable in variables {
            self.check_variable(variable)?;
            self.replace_temporary(variable, Bdd::ONE);
        }

        Ok(())
    }

    pub fn set_temporary_replacements<I>(&mut self, replacements: I) -> AssociationResult<()>
    where
        I: IntoIterator<Item = (BddVariableId, Bdd)>,
    {
        self.clear_temporary();
        self.augment_temporary_replacements(replacements)
    }

    pub fn set_temporary_variables<I>(&mut self, variables: I) -> AssociationResult<()>
    where
        I: IntoIterator<Item = BddVariableId>,
    {
        self.clear_temporary();
        self.augment_temporary_variables(variables)
    }

    pub fn select_association(
        &mut self,
        selection: CurrentAssociation,
    ) -> AssociationResult<CurrentAssociation>
    {
        let old_selection = self.current_association;

        match selection {
            CurrentAssociation::Temporary => {
                self.current_association = CurrentAssociation::Temporary;
                Ok(old_selection)
            }
            CurrentAssociation::Stored(id) => {
                if self
                    .stored_associations
                    .iter()
                    .any(|association| association.id == id)
                {
                    self.current_association = CurrentAssociation::Stored(id);
                    Ok(old_selection)
                }
                else {
                    self.current_association = CurrentAssociation::Temporary;
                    Err(AssociationError::UnknownAssociation(id))
                }
            }
        }
    }

    fn intern_association(
        &mut self,
        association: VariableAssociation,
        protected: &[Bdd],
    ) -> AssociationId
    {
        if let Some(existing) = self
            .stored_associations
            .iter_mut()
            .find(|stored| stored.association == association)
        {
            existing.references += 1;
            return existing.id;
        }

        let id = self.first_unused_id();
        let index = self
            .stored_associations
            .partition_point(|association| association.id < id);

        for replacement in protected {
            self.protect_replacement(*replacement);
        }

        self.stored_associations.insert(
            index,
            StoredAssociation {
                id,
                association,
                references: 1,
            },
        );

        id
    }

    fn first_unused_id(&self) -> AssociationId
    {
        let mut expected = 0;

        for association in &self.stored_associations {
            if association.id != expected {
                break;
            }

            expected += 1;
        }

        expected
    }

    fn replace_temporary(&mut self, variable: BddVariableId, replacement: Bdd)
    {
        if let Some(previous) = self.temporary_association.replacement(variable) {
            self.unprotect_replacement(previous);
        }

        self.temporary_association.set(variable, replacement);
        self.protect_replacement(replacement);
    }

    fn clear_temporary(&mut self)
    {
        let replacements: Vec<_> = self
            .temporary_association
            .replacements
            .iter()
            .copied()
            .flatten()
            .collect();

        for replacement in replacements {
            self.unprotect_replacement(replacement);
        }

        self.temporary_association.clear();
    }

    fn unprotect_association(&mut self, association: &VariableAssociation)
    {
        for replacement in association.replacements.iter().copied().flatten() {
            self.unprotect_replacement(replacement);
        }
    }

    fn protect_replacement(&mut self, replacement: Bdd)
    {
        *self.protected_replacements.entry(replacement).or_insert(0) += 1;
    }

    fn unprotect_replacement(&mut self, replacement: Bdd)
    {
        if let Some(count) = self.protected_replacements.get_mut(&replacement) {
            *count -= 1;
            if *count == 0 {
                self.protected_replacements.remove(&replacement);
            }
        }
    }

    fn check_variable(&self, variable: BddVariableId) -> AssociationResult<()>
    {
        if variable >= self.variable_count {
            return Err(AssociationError::InvalidVariable {
                variable,
                variable_count: self.variable_count,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn variable_association_maps_variables_to_one()
    {
        let mut manager = BddAssociationManager::new(4);

        let id = manager.new_variable_association([2, 0]).unwrap();
        let association = manager.association(id).unwrap();

        assert_eq!(association.replacement(0), Some(Bdd::ONE));
        assert_eq!(association.replacement(1), None);
        assert_eq!(association.replacement(2), Some(Bdd::ONE));
        assert_eq!(association.last(), Some(2));
    }

    #[test]
    fn replacement_association_records_replacements_and_last_variable()
    {
        let mut manager = BddAssociationManager::new(5);
        let replacement = Bdd::from_index(42);

        let id = manager
            .new_replacement_association([(3, replacement), (1, Bdd::ZERO)])
            .unwrap();
        let association = manager.association(id).unwrap();

        assert_eq!(association.replacement(1), Some(Bdd::ZERO));
        assert_eq!(association.replacement(3), Some(replacement));
        assert_eq!(association.last(), Some(3));
        assert_eq!(manager.protected_replacement_count(replacement), 1);
    }

    #[test]
    fn equal_associations_share_id_and_increment_reference_count()
    {
        let mut manager = BddAssociationManager::new(3);

        let first = manager.new_variable_association([0, 2]).unwrap();
        let second = manager.new_variable_association([2, 0]).unwrap();

        assert_eq!(first, second);
        assert_eq!(manager.reference_count(first).unwrap(), 2);
    }

    #[test]
    fn freed_ids_are_reused_after_final_reference()
    {
        let mut manager = BddAssociationManager::new(3);
        let first = manager.new_variable_association([0]).unwrap();
        let second = manager.new_variable_association([1]).unwrap();

        manager.free_association(first).unwrap();
        let reused = manager.new_variable_association([2]).unwrap();

        assert_eq!(second, 1);
        assert_eq!(reused, first);
    }

    #[test]
    fn freeing_selected_association_falls_back_to_temporary_and_flushes_cache_epoch()
    {
        let mut manager = BddAssociationManager::new(2);
        let id = manager.new_variable_association([1]).unwrap();
        manager
            .select_association(CurrentAssociation::Stored(id))
            .unwrap();

        manager.free_association(id).unwrap();

        assert_eq!(manager.current_selection(), CurrentAssociation::Temporary);
        assert_eq!(manager.cache_epoch(), 1);
    }

    #[test]
    fn temporary_association_can_be_set_and_augmented()
    {
        let mut manager = BddAssociationManager::new(4);
        let replacement = Bdd::from_index(11);

        manager.set_temporary_variables([1]).unwrap();
        manager
            .augment_temporary_replacements([(3, replacement)])
            .unwrap();

        let association = manager.temporary_association();
        assert_eq!(association.replacement(1), Some(Bdd::ONE));
        assert_eq!(association.replacement(3), Some(replacement));
        assert_eq!(association.last(), Some(3));
        assert_eq!(manager.protected_replacement_count(replacement), 1);
    }

    #[test]
    fn replacing_temporary_entry_releases_previous_replacement()
    {
        let mut manager = BddAssociationManager::new(2);
        let previous = Bdd::from_index(7);
        let replacement = Bdd::from_index(8);

        manager
            .set_temporary_replacements([(0, previous)])
            .unwrap();
        manager
            .augment_temporary_replacements([(0, replacement)])
            .unwrap();

        assert_eq!(manager.protected_replacement_count(previous), 0);
        assert_eq!(manager.protected_replacement_count(replacement), 1);
        assert_eq!(
            manager.temporary_association().replacement(0),
            Some(replacement)
        );
    }

    #[test]
    fn selecting_missing_association_restores_temporary_selection()
    {
        let mut manager = BddAssociationManager::new(1);
        let id = manager.new_variable_association([0]).unwrap();
        manager
            .select_association(CurrentAssociation::Stored(id))
            .unwrap();

        let result = manager.select_association(CurrentAssociation::Stored(99));

        assert_eq!(result, Err(AssociationError::UnknownAssociation(99)));
        assert_eq!(manager.current_selection(), CurrentAssociation::Temporary);
    }

    #[test]
    fn invalid_variable_is_rejected()
    {
        let mut manager = BddAssociationManager::new(2);

        assert_eq!(
            manager.new_variable_association([2]),
            Err(AssociationError::InvalidVariable {
                variable: 2,
                variable_count: 2,
            })
        );
    }

    #[test]
    fn source_contains_no_legacy_export_or_tracking_tokens()
    {
        let source = include_str!("bddassoc.rs");
        let legacy_export = concat!("no", "_", "mangle");
        let tracking_prefix = concat!("REQUIRED", "_");
        let dependency_type = concat!("Port", "Dependency");
        let bead_token = concat!("bead", "_id");
        let source_token = concat!("source", "_file");
        let bead_prefix = concat!("Logic", "Friday1", "-", "8j8");

        assert!(!source.contains(legacy_export));
        assert!(!source.contains("extern \"C\""));
        assert!(!source.contains(tracking_prefix));
        assert!(!source.contains(dependency_type));
        assert!(!source.contains(bead_token));
        assert!(!source.contains(source_token));
        assert!(!source.contains(bead_prefix));
    }
}
