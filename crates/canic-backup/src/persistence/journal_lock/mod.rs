//! Module: persistence::journal_lock
//!
//! Responsibility: serialize mutation of a persisted journal with a sidecar lock.
//! Does not own: journal validation, workflow policy, or domain error projection.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum JournalLockError {
    Locked { lock_path: String },
    Io(io::Error),
}

impl From<io::Error> for JournalLockError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub struct JournalLock {
    path: PathBuf,
}

impl JournalLock {
    pub(crate) fn acquire(journal_path: &Path) -> Result<Self, JournalLockError> {
        let path = journal_lock_path(journal_path);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                if let Err(error) = writeln!(file, "pid={}", std::process::id()) {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(error.into());
                }
                Ok(Self { path })
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(JournalLockError::Locked {
                    lock_path: path.to_string_lossy().to_string(),
                })
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for JournalLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn journal_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_path;

    #[test]
    fn lock_excludes_a_second_owner_and_is_removed_on_drop() {
        let journal_path = temp_path("canic-backup-journal-lock");
        let lock_path = journal_lock_path(&journal_path);
        let lock = JournalLock::acquire(&journal_path).expect("acquire lock");

        assert!(lock_path.is_file());
        std::assert_matches!(
            JournalLock::acquire(&journal_path),
            Err(JournalLockError::Locked { lock_path: actual })
                if actual == lock_path.to_string_lossy()
        );

        drop(lock);
        assert!(!lock_path.exists());
    }
}
