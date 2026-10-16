//! Module: persistence::json
//!
//! Responsibility: read JSON documents and project shared durable publication.
//! Does not own: document validation, layout paths, or integrity checks.
//! Boundary: serializes before effects and retains current-operation crash barriers.

use crate::persistence::PersistenceError;

use std::{
    io::{self, Write},
    path::Path,
};

use ic_host_fs::{
    durable::{NamedWriteError, PublicationMode, WriteOptions, write_with},
    path::canonicalize_allow_missing,
};
use serde::{Serialize, de::DeserializeOwned};

/// Maximum encoded size of a Canic backup manifest, plan, mapping or journal.
///
/// The 64 MiB document allowance is independent from streamed snapshot artifacts.
/// Publication applies the same bound so every written record remains readable.
pub const JSON_RECORD_MAX_BYTES: usize = 64 * 1024 * 1024;

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

/// Read one complete bounded backup record before decoding its JSON schema.
///
/// Uses Host's descriptor admission and independently bounded stream reader.
/// Explicit path aliases are followed; layout custody and schema validation
/// remain with callers. Non-regular descriptors refuse before any stream read.
///
/// # Errors
/// Returns native I/O, typed Host admission causes or JSON decoding failures.
pub fn read_json<T>(path: &Path) -> Result<T, PersistenceError>
where
    T: DeserializeOwned,
{
    let bytes =
        ic_host_fs::read::read_file(path, JSON_RECORD_MAX_BYTES).map_err(io::Error::from)?;
    Ok(serde_json::from_slice(&bytes)?)
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
    if bytes.len() > JSON_RECORD_MAX_BYTES {
        return Err(ic_host_artifacts::artifact::ArtifactError::LimitExceeded {
            limit: JSON_RECORD_MAX_BYTES as u64,
        }
        .into());
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    // Resolve selected directory aliases without following the publication leaf.
    let parent = canonicalize_allow_missing(parent, &std::env::current_dir()?)?;
    let destination = parent.join(name);
    // Preserve the existing umask-governed mode; layout custody remains local.
    write_with(
        &destination,
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
    fn bounded_record_reader_accepts_the_document_limit_and_refuses_larger_files() {
        use ic_host_artifacts::artifact::ArtifactError;
        let root = temp_dir("canic-backup-json-read-limit");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("record.json");
        let mut bytes = vec![b' '; JSON_RECORD_MAX_BYTES];
        bytes[..2].copy_from_slice(b"{}");
        fs::write(&path, &bytes).unwrap();
        assert_eq!(
            read_json::<serde_json::Value>(&path).unwrap(),
            serde_json::json!({})
        );
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(JSON_RECORD_MAX_BYTES as u64 + 1)
            .unwrap();
        let PersistenceError::Io(error) = read_json::<serde_json::Value>(&path).unwrap_err() else {
            panic!("expected a bounded file admission error");
        };
        assert!(
            matches!(error.get_ref().unwrap().downcast_ref::<ArtifactError>(),
            Some(ArtifactError::LimitExceeded { limit }) if *limit == JSON_RECORD_MAX_BYTES as u64)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oversized_publication_preserves_the_previous_record_before_any_barrier() {
        let root = temp_dir("canic-backup-json-write-limit");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("record.json");
        fs::write(&path, b"{}").unwrap();
        let bytes = vec![b' '; JSON_RECORD_MAX_BYTES + 1];
        let error = publish_bytes_at_barriers(&path, &bytes, PublicationMode::Replace, |_| {
            panic!("oversized publication must not cross a durable barrier")
        })
        .unwrap_err();
        assert!(matches!(
            error
                .get_ref()
                .unwrap()
                .downcast_ref::<ic_host_artifacts::artifact::ArtifactError>(),
            Some(ic_host_artifacts::artifact::ArtifactError::LimitExceeded { .. })
        ));
        assert_eq!(fs::read(&path).unwrap(), b"{}");
        assert_no_staging_file(&root, "record.json");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn record_reader_preserves_native_missing_and_json_failures() {
        let root = temp_dir("canic-backup-json-read-errors");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("record.json");
        assert!(matches!(read_json::<serde_json::Value>(&path),
            Err(PersistenceError::Io(error)) if error.kind() == io::ErrorKind::NotFound));
        fs::write(&path, b"{").unwrap();
        assert!(matches!(
            read_json::<serde_json::Value>(&path),
            Err(PersistenceError::Json(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn record_reader_rejects_a_writerless_fifo_before_reading() {
        let root = temp_dir("canic-backup-json-read-fifo");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("record.json");
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            &path,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .unwrap();
        let PersistenceError::Io(error) = read_json::<serde_json::Value>(&path).unwrap_err() else {
            panic!("expected descriptor admission failure");
        };
        assert!(matches!(
            error
                .get_ref()
                .unwrap()
                .downcast_ref::<ic_host_artifacts::artifact::ArtifactError>(),
            Some(ic_host_artifacts::artifact::ArtifactError::NotRegularFile)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn explicit_record_aliases_are_followed_without_masking_missing_targets() {
        let root = temp_dir("canic-backup-json-read-alias");
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.json");
        let alias = root.join("record.json");
        fs::write(&source, b"{}").unwrap();
        std::os::unix::fs::symlink(&source, &alias).unwrap();
        assert_eq!(
            read_json::<serde_json::Value>(&alias).unwrap(),
            serde_json::json!({})
        );
        fs::remove_file(source).unwrap();
        assert!(matches!(read_json::<serde_json::Value>(&alias),
            Err(PersistenceError::Io(error)) if error.kind() == io::ErrorKind::NotFound));
        fs::remove_dir_all(root).unwrap();
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
