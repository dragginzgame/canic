//! Module: persistence::json
//!
//! Responsibility: read and atomically write JSON persistence documents.
//! Does not own: document validation, layout paths, or integrity checks.
//! Boundary: provides filesystem JSON helpers for backup layout persistence.

use crate::persistence::PersistenceError;

use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};

pub(super) fn write_json_atomic<T>(path: &Path, value: &T) -> Result<(), PersistenceError>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = temp_path_for(path);
    let mut file = File::create(&tmp_path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.sync_all()?;
    drop(file);

    fs::rename(&tmp_path, path)?;

    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }

    Ok(())
}

pub(super) fn read_json<T>(path: &Path) -> Result<T, PersistenceError>
where
    T: DeserializeOwned,
{
    let file = File::open(path)?;
    Ok(serde_json::from_reader(file)?)
}

fn temp_path_for(path: &Path) -> PathBuf {
    let mut file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("canic-backup")
        .to_string();
    file_name.push_str(".tmp");
    path.with_file_name(file_name)
}
