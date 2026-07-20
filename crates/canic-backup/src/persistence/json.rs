//! Module: persistence::json
//!
//! Responsibility: read and durably create or replace JSON persistence documents.
//! Does not own: document validation, layout paths, or integrity checks.
//! Boundary: provides explicit create-only and replace filesystem primitives.

use crate::persistence::PersistenceError;

use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Serialize, de::DeserializeOwned};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn write_json_durable<T>(path: &Path, value: &T) -> Result<(), PersistenceError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    replace_bytes(path, &bytes).map_err(PersistenceError::from)
}

pub fn create_json_durable<T>(path: &Path, value: &T) -> Result<(), PersistenceError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    create_bytes_at_barriers(path, &bytes, || {}, || {}).map_err(PersistenceError::from)
}

pub fn read_json<T>(path: &Path) -> Result<T, PersistenceError>
where
    T: DeserializeOwned,
{
    let file = File::open(path)?;
    Ok(serde_json::from_reader(file)?)
}

fn replace_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    replace_bytes_at_barriers(path, bytes, |_| {})
}

fn create_bytes_at_barriers(
    path: &Path,
    bytes: &[u8],
    mut before_publication: impl FnMut(),
    mut after_directory_sync: impl FnMut(),
) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let (temp_path, mut temp_file) = create_sibling_temp(path, parent)?;
    if let Err(error) = temp_file
        .write_all(bytes)
        .and_then(|()| temp_file.sync_all())
    {
        drop(temp_file);
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    drop(temp_file);
    before_publication();

    if let Err(error) = fs::hard_link(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    fs::remove_file(&temp_path)?;
    File::open(parent)?.sync_all()?;
    after_directory_sync();
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableWriteBarrier {
    BeforeRename,
    AfterDirectorySync,
}

fn replace_bytes_at_barriers(
    path: &Path,
    bytes: &[u8],
    mut barrier: impl FnMut(DurableWriteBarrier),
) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let (temp_path, mut temp_file) = create_sibling_temp(path, parent)?;
    if let Err(error) = temp_file
        .write_all(bytes)
        .and_then(|()| temp_file.sync_all())
    {
        drop(temp_file);
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    drop(temp_file);
    barrier(DurableWriteBarrier::BeforeRename);

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    File::open(parent)?.sync_all()?;
    barrier(DurableWriteBarrier::AfterDirectorySync);
    Ok(())
}

#[cfg(test)]
pub fn write_json_durable_at_barriers<T>(
    path: &Path,
    value: &T,
    barrier: impl FnMut(DurableWriteBarrier),
) -> Result<(), PersistenceError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    replace_bytes_at_barriers(path, &bytes, barrier).map_err(PersistenceError::from)
}

#[cfg(test)]
pub fn create_json_durable_at_barriers<T>(
    path: &Path,
    value: &T,
    before_publication: impl FnMut(),
    after_directory_sync: impl FnMut(),
) -> Result<(), PersistenceError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    create_bytes_at_barriers(path, &bytes, before_publication, after_directory_sync)
        .map_err(PersistenceError::from)
}

fn create_sibling_temp(path: &Path, parent: &Path) -> io::Result<(PathBuf, File)> {
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("durable write target has no file name: {}", path.display()),
        )
    })?;

    for _ in 0..64 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temp_name = OsString::from(".");
        temp_name.push(file_name);
        temp_name.push(format!(".canic-tmp-{}-{sequence}", std::process::id()));
        let temp_path = parent.join(temp_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "could not allocate a unique sibling temporary file for {}",
            path.display()
        ),
    ))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operational_readiness::manifest::assert_case_defined;
    use crate::test_support::temp_dir;
    use serde::Serializer;

    struct FailingSerialize;

    impl Serialize for FailingSerialize {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(serde::ser::Error::custom(
                "intentional serialization failure",
            ))
        }
    }

    #[test]
    fn durable_json_replaces_the_complete_document() {
        let root = temp_dir("canic-backup-durable-json-replace");
        let path = root.join("journal.json");
        fs::create_dir_all(&root).expect("create temp root");
        fs::write(&path, b"previous-document-with-more-bytes").expect("write previous document");

        write_json_durable(&path, &serde_json::json!({"state": "ready"}))
            .expect("replace document");

        let written = fs::read_to_string(&path).expect("read replaced document");
        let decoded: serde_json::Value = serde_json::from_str(&written).expect("decode document");
        assert_eq!(decoded, serde_json::json!({"state": "ready"}));
        assert_no_staging_file(&root, "journal.json");
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn serialization_failure_preserves_the_previous_document() {
        assert_case_defined("CANIC-094-C10/persistence-failure/rejection");
        let root = temp_dir("canic-backup-durable-json-serialize");
        let path = root.join("journal.json");
        fs::create_dir_all(&root).expect("create temp root");
        fs::write(&path, b"previous-document").expect("write previous document");

        let error = write_json_durable(&path, &FailingSerialize)
            .expect_err("serialization failure should reject");

        std::assert_matches!(error, PersistenceError::Json(_));
        assert_eq!(
            fs::read(&path).expect("read previous document"),
            b"previous-document"
        );
        assert_no_staging_file(&root, "journal.json");
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn rename_failure_removes_the_staging_file() {
        assert_case_defined("CANIC-094-C10/persistence-failure/rejection");
        let root = temp_dir("canic-backup-durable-json-rename");
        let path = root.join("journal.json");
        fs::create_dir_all(&path).expect("create conflicting target directory");

        let error = write_json_durable(&path, &serde_json::json!({"state": "ready"}))
            .expect_err("rename over directory should reject");

        std::assert_matches!(error, PersistenceError::Io(_));
        assert!(path.is_dir());
        assert_no_staging_file(&root, "journal.json");
        fs::remove_dir_all(root).expect("remove temp root");
    }

    fn assert_no_staging_file(root: &Path, target_name: &str) {
        let prefix = format!(".{target_name}.canic-tmp-");
        let staging_files = fs::read_dir(root)
            .expect("read temp root")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
            .collect::<Vec<_>>();
        assert!(staging_files.is_empty(), "staging files remain");
    }
}
