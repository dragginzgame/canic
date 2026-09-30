//! Module: persistence::layout_lifetime
//!
//! Responsibility: exclude layout deletion and retain unfinished restore references.
//! Does not own: restore transitions, command execution, or retention selection.
//! Boundary: one stable parent-side lock protects backup contents and durable references.

#[cfg(test)]
mod tests;

use crate::persistence::{
    JournalLock, JournalLockError, PersistenceError, read_json, write_json_durable,
};

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const REFERENCES_FILE: &str = "restore-references.json";

///
/// BackupLayoutGuard
///
/// Persistence-owned exclusive layout access, shared by backup, restore and prune.
/// Its lock file lives outside the directory that prune removes.
///

#[derive(Debug)]
pub struct BackupLayoutGuard {
    root: PathBuf,
    _lock: JournalLock,
}

impl BackupLayoutGuard {
    pub(crate) fn acquire(root: &Path) -> Result<Self, JournalLockError> {
        let root = root.canonicalize()?;
        let parent = root
            .parent()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        let name = root
            .file_name()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        let key = format!("{:x}", Sha256::digest(name.as_encoded_bytes()));
        let lock = JournalLock::acquire(&parent.join(format!(".canic-backup-{key}")))?;
        // Prune may have won after canonicalization. Never recreate its removed root.
        if !fs::symlink_metadata(&root)?.is_dir() {
            return Err(io::Error::from(io::ErrorKind::NotADirectory).into());
        }
        Ok(Self { root, _lock: lock })
    }

    /// Return the resolved root protected by this guard.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Report durable restore dependencies, including external or missing journals.
    pub fn has_restore_references(&self) -> Result<bool, PersistenceError> {
        Ok(!self.read_references()?.restores.is_empty())
    }

    pub(crate) fn retain_restore(
        &self,
        journal: &Path,
        authority: &str,
    ) -> Result<(), PersistenceError> {
        let mut references = self.read_references()?;
        let reference = RestoreReferenceRecord {
            journal: journal.to_path_buf(),
            authority: authority.to_owned(),
        };
        if let Some(existing) = references
            .restores
            .iter()
            .find(|entry| entry.journal == journal)
        {
            if existing != &reference {
                return Err(PersistenceError::RestoreReferenceConflict {
                    path: journal.display().to_string(),
                });
            }
            // Adopt a reference whose rename may have outlived a failed directory sync.
            fs::File::open(self.root.join(REFERENCES_FILE))?.sync_all()?;
            fs::File::open(&self.root)?.sync_all()?;
            return Ok(());
        }
        references.restores.push(reference);
        write_json_durable(&self.root.join(REFERENCES_FILE), &references)
    }

    pub(crate) fn release_restore(
        &self,
        journal: &Path,
        authority: &str,
    ) -> Result<(), PersistenceError> {
        let mut references = self.read_references()?;
        let Some(index) = references
            .restores
            .iter()
            .position(|entry| entry.journal == journal)
        else {
            return Ok(());
        };
        if references.restores[index].authority != authority {
            return Err(PersistenceError::RestoreReferenceConflict {
                path: journal.display().to_string(),
            });
        }
        references.restores.remove(index);
        write_json_durable(&self.root.join(REFERENCES_FILE), &references)
    }

    fn read_references(&self) -> Result<RestoreReferencesRecord, PersistenceError> {
        let path = self.root.join(REFERENCES_FILE);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(RestoreReferencesRecord {
                    version: 1,
                    restores: Vec::new(),
                });
            }
            Err(error) => return Err(error.into()),
            Ok(metadata) if !metadata.is_file() => {
                return Err(PersistenceError::InvalidRestoreReferences {
                    path: path.display().to_string(),
                });
            }
            Ok(_) => {}
        }
        let references: RestoreReferencesRecord = read_json(&path)?;
        if references.version != 1 {
            return Err(PersistenceError::InvalidRestoreReferences {
                path: path.display().to_string(),
            });
        }
        Ok(references)
    }
}

///
/// BackupExecutionGuard
///
/// Holds both layout lifetime and execution-journal mutation authority.
///

#[derive(Debug)]
pub struct BackupExecutionGuard {
    pub(super) _lifetime: BackupLayoutGuard,
    pub(super) _journal: JournalLock,
}

///
/// RestoreReferencesRecord
///
/// Durable unfinished restore dependencies belonging to one backup layout.
///

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RestoreReferencesRecord {
    version: u16,
    restores: Vec<RestoreReferenceRecord>,
}

///
/// RestoreReferenceRecord
///
/// Binds one journal location to its immutable restore authority digest.
///

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RestoreReferenceRecord {
    journal: PathBuf,
    authority: String,
}
