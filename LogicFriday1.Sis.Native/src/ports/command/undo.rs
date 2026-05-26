//! Native undo support for the SIS command package.
//!
//! The legacy command swaps the current network pointer with a single saved
//! backup network. This port keeps that behavior as owned Rust state so callers
//! can decide where command registration and facade bindings belong.

use std::error::Error;
use std::fmt;

pub type UndoResult<T> = Result<T, UndoError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UndoError
{
    Usage,
    NoSavedNetwork,
}

impl fmt::Display for UndoError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::Usage => write!(formatter, "usage: undo"),
            Self::NoSavedNetwork => write!(formatter, "undo: no network currently saved"),
        }
    }
}

impl Error for UndoError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndoManager<N>
{
    backup_network: Option<N>,
}

impl<N> Default for UndoManager<N>
{
    fn default() -> Self
    {
        Self::new()
    }
}

impl<N> UndoManager<N>
{
    pub fn new() -> Self
    {
        Self
        {
            backup_network: None,
        }
    }

    pub fn with_backup(backup_network: N) -> Self
    {
        Self
        {
            backup_network: Some(backup_network),
        }
    }

    pub fn has_backup(&self) -> bool
    {
        self.backup_network.is_some()
    }

    pub fn backup_network(&self) -> Option<&N>
    {
        self.backup_network.as_ref()
    }

    pub fn backup_network_mut(&mut self) -> Option<&mut N>
    {
        self.backup_network.as_mut()
    }

    pub fn replace_backup(&mut self, backup_network: Option<N>) -> Option<N>
    {
        std::mem::replace(&mut self.backup_network, backup_network)
    }

    pub fn take_backup(&mut self) -> Option<N>
    {
        self.backup_network.take()
    }

    pub fn undo<S>(&mut self, current_network: &mut Option<N>, argv: &[S]) -> UndoResult<()>
    where
        S: AsRef<str>,
    {
        if argv.len() != 1
        {
            return Err(UndoError::Usage);
        }

        if self.backup_network.is_none()
        {
            return Err(UndoError::NoSavedNetwork);
        }

        std::mem::swap(current_network, &mut self.backup_network);
        Ok(())
    }
}

pub fn undo_command<N, S>(
    manager: &mut UndoManager<N>,
    current_network: &mut Option<N>,
    argv: &[S],
) -> UndoResult<()>
where
    S: AsRef<str>,
{
    manager.undo(current_network, argv)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNetwork
    {
        name: String,
    }

    impl TestNetwork
    {
        fn new(name: impl Into<String>) -> Self
        {
            Self
            {
                name: name.into(),
            }
        }
    }

    #[test]
    fn undo_swaps_current_network_with_saved_backup()
    {
        let mut manager = UndoManager::with_backup(TestNetwork::new("previous"));
        let mut current_network = Some(TestNetwork::new("current"));

        undo_command(&mut manager, &mut current_network, &["undo"]).unwrap();

        assert_eq!(current_network, Some(TestNetwork::new("previous")));
        assert_eq!(manager.backup_network(), Some(&TestNetwork::new("current")));
    }

    #[test]
    fn undo_can_restore_saved_network_when_current_network_is_empty()
    {
        let mut manager = UndoManager::with_backup(TestNetwork::new("previous"));
        let mut current_network = None;

        manager.undo(&mut current_network, &["undo"]).unwrap();

        assert_eq!(current_network, Some(TestNetwork::new("previous")));
        assert_eq!(manager.backup_network(), None);
    }

    #[test]
    fn undo_requires_exactly_the_command_name_argument()
    {
        let mut manager = UndoManager::with_backup(TestNetwork::new("previous"));
        let mut current_network = Some(TestNetwork::new("current"));

        let error = manager
            .undo(&mut current_network, &["undo", "extra"])
            .unwrap_err();

        assert_eq!(error, UndoError::Usage);
        assert_eq!(error.to_string(), "usage: undo");
        assert_eq!(current_network, Some(TestNetwork::new("current")));
        assert_eq!(manager.backup_network(), Some(&TestNetwork::new("previous")));
    }

    #[test]
    fn undo_reports_missing_saved_network_without_changing_current_network()
    {
        let mut manager = UndoManager::<TestNetwork>::new();
        let mut current_network = Some(TestNetwork::new("current"));

        let error = manager.undo(&mut current_network, &["undo"]).unwrap_err();

        assert_eq!(error, UndoError::NoSavedNetwork);
        assert_eq!(error.to_string(), "undo: no network currently saved");
        assert_eq!(current_network, Some(TestNetwork::new("current")));
        assert_eq!(manager.backup_network(), None);
    }

    #[test]
    fn replace_backup_returns_previous_backup()
    {
        let mut manager = UndoManager::with_backup(TestNetwork::new("first"));

        let previous = manager.replace_backup(Some(TestNetwork::new("second")));

        assert_eq!(previous, Some(TestNetwork::new("first")));
        assert_eq!(manager.backup_network(), Some(&TestNetwork::new("second")));
    }
}
