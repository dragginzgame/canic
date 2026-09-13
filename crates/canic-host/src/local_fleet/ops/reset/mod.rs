//! Exact-session local state discard with interruption and terminal replay records.

use crate::{
    durable_io,
    local_fleet::{LocalFleetError, model::LocalResetRecord, ops},
};
use std::{fs, path::Path};

fn read(directory: &Path) -> Result<Option<LocalResetRecord>, LocalFleetError> {
    match durable_io::read_regular_bytes(&directory.join("reset.json"), 4096) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// An interrupted reset must complete before creating another local environment.
pub fn require_complete(directory: &Path) -> Result<(), LocalFleetError> {
    if read(directory)?.is_some_and(|record| record.schema_version != 1 || !record.complete) {
        return Err(LocalFleetError::Session);
    }
    Ok(())
}

/// Persist exact reset authority before removing any simulator file.
pub fn begin(directory: &Path, expected: &str) -> Result<bool, LocalFleetError> {
    let retained = ops::read_record(directory)?;
    if let Some(record) = retained {
        if record.session_id != expected
            || record.configuration.name
                != directory.file_name().unwrap_or_default().to_string_lossy()
        {
            return Err(LocalFleetError::Session);
        }
    } else {
        let prior = read(directory)?.ok_or(LocalFleetError::Session)?;
        if prior.schema_version != 1 || prior.session_id != expected {
            return Err(LocalFleetError::Session);
        }
        if prior.complete {
            return Ok(false);
        }
    }
    let intent = LocalResetRecord {
        schema_version: 1,
        session_id: expected.into(),
        complete: false,
    };
    durable_io::write_bytes(&directory.join("reset.json"), &serde_json::to_vec(&intent)?)?;
    Ok(true)
}

/// Remove only the selected owned instance tree after the durable intent.
pub fn remove_instance(directory: &Path) -> Result<(), LocalFleetError> {
    ops::validate_tree(directory)?;
    let reset = read(directory)?.ok_or(LocalFleetError::Session)?;
    let instance = ops::instance_directory(directory, &reset.session_id)?;
    match fs::remove_dir_all(&instance) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Publish terminal reset replay only after both instance and environment record are removed.
pub fn finish(directory: &Path, expected: &str) -> Result<(), LocalFleetError> {
    for name in ["preparation.json", "desired.toml", "root-key.der"] {
        match fs::remove_file(directory.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    match fs::remove_file(directory.join("environment.json")) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let record = LocalResetRecord {
        schema_version: 1,
        session_id: expected.into(),
        complete: true,
    };
    durable_io::write_bytes(&directory.join("reset.json"), &serde_json::to_vec(&record)?)?;
    Ok(())
}
