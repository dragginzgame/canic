//! Archive completed operation files as opaque bytes before selecting a fresh reset.
//!
//! Append-only objects precede their manifest. Interrupted writes may leave unused
//! objects; repeating capture completes the same snapshot without changing its source.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{create_new_bytes_with_parents, read_optional_regular_bytes_bounded},
    fleet_ensure::{
        model::completed_operation::CompletedOperationArchiveRecord,
        ops::{EnsurePaths, EnsureStateError, is_sha256, operation_selection},
        policy::validate_path_labels,
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: usize = 256 * 1024 * 1024;
const MAX_FILES: usize = 8_192;

/// Preserve a complete local snapshot under the caller's Fleet lock. No file is removed.
pub(in crate::fleet_ensure) fn capture(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<Option<String>, EnsureStateError> {
    validate_path_labels(environment, fleet).map_err(|_| invalid())?;
    let Some(completed) = operation_selection::completed(paths, environment, fleet)? else {
        return Ok(None);
    };
    let directory = paths.plan.parent().ok_or_else(invalid)?;
    let mut snapshot = Snapshot {
        archive: paths
            .workspace
            .join(".canic/fleet-ensure/history")
            .join(environment)
            .join(fleet),
        files: BTreeMap::new(),
        sources: BTreeMap::new(),
        unavailable_objects: BTreeSet::new(),
        remaining: MAX_ARCHIVE_BYTES,
    };
    snapshot.directory(paths, directory, directory, 0)?;
    // Hashes bind bytes, not a reconstructed serialization or interpreted old contract.
    // Recheck every source before publishing the manifest; lock holders must not race.
    for (name, digest) in &snapshot.files {
        let source = snapshot.sources.get(name).ok_or_else(invalid)?;
        if sha256_hex(&read(source)?) != *digest {
            return Err(invalid());
        }
    }
    let record = CompletedOperationArchiveRecord {
        schema_version: 1,
        environment: environment.into(),
        fleet: fleet.into(),
        operation_id: completed.operation_id,
        plan_sha256: completed.plan_sha256,
        files: snapshot.files,
        unavailable_objects: snapshot.unavailable_objects,
    };
    let bytes = serde_json::to_vec_pretty(&record).map_err(|_| invalid())?;
    let digest = sha256_hex(&bytes);
    retain(
        &snapshot
            .archive
            .join("operations")
            .join(format!("{digest}.json")),
        &bytes,
    )?;
    Ok(Some(digest))
}

struct Snapshot {
    archive: PathBuf,
    files: BTreeMap<String, String>,
    sources: BTreeMap<String, PathBuf>,
    unavailable_objects: BTreeSet<String>,
    remaining: usize,
}

impl Snapshot {
    fn directory(
        &mut self,
        paths: &EnsurePaths,
        root: &Path,
        directory: &Path,
        depth: u8,
    ) -> Result<(), EnsureStateError> {
        if depth > 8 {
            return Err(invalid());
        }
        for entry in fs::read_dir(directory).map_err(|error| io_error(directory, error))? {
            let entry = entry.map_err(|error| io_error(directory, error))?;
            let path = entry.path();
            if path == paths.lock {
                continue;
            }
            let kind = entry.file_type().map_err(|error| io_error(&path, error))?;
            if kind.is_dir() {
                self.directory(paths, root, &path, depth + 1)?;
            } else if kind.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .ok()
                    .and_then(Path::to_str)
                    .ok_or_else(invalid)?;
                let bytes = read(&path)?;
                self.retain(format!("estate/{relative}"), &path, &bytes)?;
                // Shared publication blobs are part of evidence too. Recognize
                // storage references only, without hydrating executable phases.
                if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                    self.content(paths, &value)?;
                }
            } else {
                return Err(EnsureStateError::Unsafe { path });
            }
        }
        Ok(())
    }

    fn content(&mut self, paths: &EnsurePaths, value: &Value) -> Result<(), EnsureStateError> {
        match value {
            Value::Object(fields) => {
                if let (Some(Value::String(digest)), Some(_)) =
                    (fields.get("bytes_sha256"), fields.get("bytes_size"))
                    && is_sha256(digest)
                {
                    self.content_object(paths, digest)?;
                }
                for value in fields.values() {
                    self.content(paths, value)?;
                }
            }
            Value::Array(values) => {
                for value in values {
                    self.content(paths, value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn content_object(
        &mut self,
        paths: &EnsurePaths,
        digest: &str,
    ) -> Result<(), EnsureStateError> {
        let name = format!("objects/{digest}");
        if self.files.contains_key(&name) || self.unavailable_objects.contains(digest) {
            return Ok(());
        }
        let candidates = [
            paths.content.join(digest),
            paths
                .plan
                .with_file_name("activation-reset-evidence")
                .join(digest),
        ];
        for original in candidates {
            if let Some(bytes) = read_optional_regular_bytes_bounded(&original, MAX_FILE_BYTES)
                .map_err(|_| invalid())?
            {
                // Preserve what exists. Historical references are evidence, not
                // current artifact qualification or a reason to rebuild old Wasm.
                return self.retain(name, &original, &bytes);
            }
        }
        self.unavailable_objects.insert(digest.into());
        Ok(())
    }

    fn retain(
        &mut self,
        name: String,
        source: &Path,
        bytes: &[u8],
    ) -> Result<(), EnsureStateError> {
        self.remaining = self
            .remaining
            .checked_sub(bytes.len())
            .ok_or_else(invalid)?;
        if self.files.len() >= MAX_FILES {
            return Err(invalid());
        }
        let digest = sha256_hex(bytes);
        retain(&self.archive.join("objects").join(&digest), bytes)?;
        self.sources.insert(name.clone(), source.to_path_buf());
        if self.files.insert(name, digest).is_some() {
            return Err(invalid());
        }
        Ok(())
    }
}

fn read(path: &Path) -> Result<Vec<u8>, EnsureStateError> {
    read_optional_regular_bytes_bounded(path, MAX_FILE_BYTES)
        .map_err(|_| invalid())?
        .ok_or_else(invalid)
}

fn retain(path: &Path, bytes: &[u8]) -> Result<(), EnsureStateError> {
    match create_new_bytes_with_parents(path, bytes) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read(path)? == bytes {
                Ok(())
            } else {
                Err(invalid())
            }
        }
        Err(error) => Err(io_error(path, error)),
    }
}

const fn invalid() -> EnsureStateError {
    EnsureStateError::InvalidTerminalSource
}

fn io_error(path: &Path, source: std::io::Error) -> EnsureStateError {
    EnsureStateError::Io {
        path: path.to_path_buf(),
        source,
    }
}
