//! Module: canister_build::reuse::snapshot
//!
//! Responsibility: compare source evidence independently of Cargo inventory replacement.
//! Does not own: compilation, dependency discovery or release publication.
//! Boundary: newly discovered bytes cannot acquire retrospective cache authority.

#[cfg(test)]
mod tests;

use super::{BuildReuseError, add_optional, collect_files, hash_field, source_entry_is_excluded};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
};

/// Pre-build source bytes and non-file authority retained until cache recording.
pub(super) struct BuildInputSnapshot {
    pub(super) identity: String,
    pub(super) files: BTreeMap<String, String>,
}

impl BuildInputSnapshot {
    pub(super) fn digest(&self) -> String {
        let mut digest = Sha256::new();
        hash_field(&mut digest, self.identity.as_bytes());
        for (path, hash) in &self.files {
            hash_field(&mut digest, path.as_bytes());
            hash_field(&mut digest, hash.as_bytes());
        }
        format!("{:x}", digest.finalize())
    }

    /// Admit the current inventory only when every input was verified before compilation.
    pub(super) fn validate_after(&self, after: &Self) -> Result<(), BuildReuseError> {
        if self.identity != after.identity {
            return Err(BuildReuseError::Changed);
        }
        // A replaced Cargo record may stop naming files or directories. Recheck them,
        // including additions beneath an external directory that disappeared from the inventory.
        let mut retained = BTreeMap::new();
        for path in self
            .files
            .keys()
            .filter(|path| !after.files.contains_key(*path))
        {
            if retained.contains_key(path) {
                continue;
            }
            let input = Path::new(path);
            if fs::symlink_metadata(input).is_ok_and(|metadata| metadata.is_dir()) {
                collect_files(input, input, &mut retained, true)?;
            } else {
                add_optional(input, &mut retained)?;
            }
        }
        for (path, before) in &self.files {
            if after.files.get(path).or_else(|| retained.get(path)) != Some(before) {
                return Err(BuildReuseError::ChangedInput(path.into()));
            }
        }
        for (path, value) in after
            .files
            .iter()
            .chain(&retained)
            .filter(|(path, _)| !self.files.contains_key(*path))
        {
            if self.observed_absence(Path::new(path)) {
                // Cargo can first name a missing input after compilation. A complete
                // earlier directory scan already proved absence beneath a missing entry.
                if value == "absent" {
                    continue;
                }
                return Err(BuildReuseError::ChangedInput(path.into()));
            }
            return Err(BuildReuseError::UnobservedInput(path.into()));
        }
        Ok(())
    }

    fn observed_absence(&self, path: &Path) -> bool {
        let Some(directory) = path.ancestors().skip(1).find(|ancestor| {
            ancestor
                .to_str()
                .and_then(|key| self.files.get(key))
                .is_some_and(|value| value == "directory")
        }) else {
            return false;
        };
        let suffix = path.strip_prefix(directory).expect("path ancestor");
        if !suffix
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        {
            return false;
        }
        let Some(Component::Normal(name)) = suffix.components().next() else {
            return false;
        };
        // Excluded entries were never scanned, even when their parent was. Do not
        // turn their first observation into retrospective source authority.
        !source_entry_is_excluded(name)
            && directory
                .join(name)
                .to_str()
                .is_some_and(|key| !self.files.contains_key(key))
    }
}
