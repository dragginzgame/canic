//! Original-command custody admission for Canic's maintained execution journals.
use ic_backup::{
    model::command_custody::CommandCustodyRecord,
    ops::persistence::{CommandLifetimeLock, CommandLifetimeLockError, CommandQuiescenceGuard},
};
use std::path::Path;

pub fn acquire_recorded_command(
    journal: &Path,
    sequence: usize,
    original: Option<&CommandCustodyRecord>,
) -> Result<CommandLifetimeLock, CommandLifetimeLockError> {
    if let Some(original) = original {
        drop(CommandQuiescenceGuard::acquire(original)?);
    }
    let lock = CommandLifetimeLock::acquire(journal, sequence as u64)?;
    if original.is_some_and(|original| original != lock.record()) {
        return Err(CommandLifetimeLockError::IdentityChanged {
            path: lock.path().to_path_buf(),
        });
    }
    Ok(lock)
}
