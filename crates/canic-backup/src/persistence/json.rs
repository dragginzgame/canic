//! Module: persistence::json
//!
//! Responsibility: read JSON documents and project shared durable publication.
//! Does not own: document validation, layout paths, or integrity checks.
//! Boundary: serializes before effects and retains current-operation crash barriers.

use crate::persistence::PersistenceError;

use std::{
    fs::File,
    io::{self, BufReader, Write},
    path::Path,
};

use ic_host_fs::durable::{NamedWriteError, PublicationMode, WriteOptions, write_typed_with};
use serde::{Serialize, de::DeserializeOwned};

pub fn write_json_durable<T>(path: &Path, value: &T) -> Result<(), PersistenceError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    publish_bytes_at_barriers(path, &bytes, PublicationMode::Replace, |_| {})
        .map_err(PersistenceError::from)
}

pub fn create_json_durable<T>(path: &Path, value: &T) -> Result<(), PersistenceError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    publish_bytes_at_barriers(path, &bytes, PublicationMode::CreateNew, |_| {})
        .map_err(PersistenceError::from)
}

pub fn read_json<T>(path: &Path) -> Result<T, PersistenceError>
where
    T: DeserializeOwned,
{
    let file = File::open(path)?;
    Ok(serde_json::from_reader(BufReader::new(file))?)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableWriteBarrier {
    BeforeRename,
    AfterDirectorySync,
}

fn publish_bytes_at_barriers(
    path: &Path,
    bytes: &[u8],
    mode: PublicationMode,
    mut barrier: impl FnMut(DurableWriteBarrier),
) -> io::Result<()> {
    // Preserve the existing umask-governed mode; layout custody remains local.
    write_typed_with(
        path,
        WriteOptions {
            mode,
            permissions: 0o666,
        },
        |file| {
            file.write_all(bytes)?;
            // Crash evidence needs synchronized staging before publication.
            file.sync_all()?;
            barrier(DurableWriteBarrier::BeforeRename);
            Ok::<(), io::Error>(())
        },
    )
    .map_err(publication_io_error)?;
    barrier(DurableWriteBarrier::AfterDirectorySync);
    Ok(())
}

fn publication_io_error(error: NamedWriteError<io::Error>) -> io::Error {
    let kind = match &error {
        NamedWriteError::Producer { source, .. }
        | NamedWriteError::BeforePublication { source, .. }
        | NamedWriteError::AfterPublication { source } => source.kind(),
    };
    // Retain the public Io variant/kind and the typed visibility/cleanup cause.
    io::Error::new(kind, error)
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
    publish_bytes_at_barriers(path, &bytes, PublicationMode::Replace, barrier)
        .map_err(PersistenceError::from)
}

#[cfg(test)]
pub fn create_json_durable_at_barriers<T>(
    path: &Path,
    value: &T,
    mut before_publication: impl FnMut(),
    mut after_directory_sync: impl FnMut(),
) -> Result<(), PersistenceError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    publish_bytes_at_barriers(
        path,
        &bytes,
        PublicationMode::CreateNew,
        |barrier| match barrier {
            DurableWriteBarrier::BeforeRename => before_publication(),
            DurableWriteBarrier::AfterDirectorySync => after_directory_sync(),
        },
    )
    .map_err(PersistenceError::from)
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
    use std::fs;

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

        let PersistenceError::Io(source) = error else {
            panic!("expected publication IO error");
        };
        std::assert_matches!(
            source
                .get_ref()
                .and_then(|cause| cause.downcast_ref::<NamedWriteError<io::Error>>()),
            Some(NamedWriteError::BeforePublication {
                cleanup_error: None,
                ..
            })
        );
        assert!(path.is_dir());
        assert_no_staging_file(&root, "journal.json");
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn create_only_conflict_preserves_the_existing_document() {
        let root = temp_dir("canic-backup-durable-json-create-conflict");
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("manifest.json");
        fs::write(&path, b"retained authority").expect("write existing document");

        let PersistenceError::Io(source) =
            create_json_durable(&path, &serde_json::json!({"state": "new"}))
                .expect_err("create-only conflict")
        else {
            panic!("expected publication IO error");
        };
        assert_eq!(source.kind(), io::ErrorKind::AlreadyExists);
        std::assert_matches!(
            source
                .get_ref()
                .and_then(|cause| cause.downcast_ref::<NamedWriteError<io::Error>>()),
            Some(NamedWriteError::BeforePublication { .. })
        );
        assert_eq!(
            fs::read(&path).expect("read retained document"),
            b"retained authority"
        );
        assert_no_staging_file(&root, "manifest.json");
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn failed_serialization_creates_no_parent_or_document() {
        let root = temp_dir("canic-backup-durable-json-create-serialize");
        let path = root.join("missing/manifest.json");
        let error = create_json_durable(&path, &FailingSerialize)
            .expect_err("serialization failure should reject");
        std::assert_matches!(error, PersistenceError::Json(_));
        assert!(!root.exists());
    }

    #[cfg(unix)]
    #[test]
    fn publication_projection_retains_visibility_cleanup_and_native_cause() {
        let native_error = rustix::io::Errno::IO.raw_os_error();
        let error = publication_io_error(NamedWriteError::AfterPublication {
            source: io::Error::from_raw_os_error(native_error),
        });
        let cause = error
            .get_ref()
            .and_then(|cause| cause.downcast_ref::<NamedWriteError<io::Error>>())
            .expect("typed publication error");
        let NamedWriteError::AfterPublication { source } = cause else {
            panic!("expected visible publication failure");
        };
        assert_eq!(error.kind(), source.kind());
        assert_eq!(source.raw_os_error(), Some(native_error));

        let error = publication_io_error(NamedWriteError::Producer {
            source: io::Error::from(io::ErrorKind::WriteZero),
            cleanup_error: Some(io::Error::from_raw_os_error(native_error)),
        });
        assert_eq!(error.kind(), io::ErrorKind::WriteZero);
        let cause = error
            .get_ref()
            .and_then(|cause| cause.downcast_ref::<NamedWriteError<io::Error>>())
            .expect("typed producer error");
        let NamedWriteError::Producer {
            source,
            cleanup_error: Some(cleanup_error),
        } = cause
        else {
            panic!("expected producer and independent cleanup failures");
        };
        assert_eq!(source.kind(), io::ErrorKind::WriteZero);
        assert_eq!(cleanup_error.raw_os_error(), Some(native_error));
    }

    fn assert_no_staging_file(root: &Path, target_name: &str) {
        let unexpected_files = fs::read_dir(root)
            .expect("read temp root")
            .map(|entry| entry.expect("read directory entry").file_name())
            .filter(|name| name != target_name)
            .collect::<Vec<_>>();
        assert!(unexpected_files.is_empty(), "unexpected files remain");
    }
}
