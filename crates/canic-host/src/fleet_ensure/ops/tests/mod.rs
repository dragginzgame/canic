//! Consumer proof for complete, bounded Fleet document publication.

use super::*;
use crate::test_support::temp_dir;
use ic_host_fs::durable::NamedWriteError;
use serde::ser::{Error as _, SerializeSeq};
use std::fs;

struct InterruptedDocument;

impl Serialize for InterruptedDocument {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(2))?;
        sequence.serialize_element("partial document")?;
        Err(S::Error::custom("interrupted document producer"))
    }
}

#[test]
fn current_document_publication_preserves_complete_json_bytes() {
    let directory = temp_dir("canic-streamed-fleet-document");
    let path = directory.join("state.json");
    let value = (u64::MAX, "Unicode: 🌍, escaped: \"\\\n", vec![1, 2, 3]);

    write_current(&path, &value).unwrap();

    assert_eq!(
        fs::read(path).unwrap(),
        serde_json::to_vec_pretty(&value).unwrap()
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_document_producer_preserves_retained_authority_and_typed_cause() {
    let directory = temp_dir("canic-interrupted-fleet-document");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("journal.json");
    fs::write(&path, b"retained authority").unwrap();

    let error = write_current(&path, &InterruptedDocument).unwrap_err();

    let EnsureStateError::Io {
        path: failed,
        source,
    } = error
    else {
        panic!("expected typed publication failure");
    };
    assert_eq!(failed, path);
    assert_eq!(source.kind(), io::ErrorKind::InvalidData);
    assert!(matches!(
        source.get_ref().unwrap().downcast_ref::<NamedWriteError<serde_json::Error>>(),
        Some(NamedWriteError::Producer { source, cleanup_error: None }) if source.is_data()
    ));
    assert_eq!(fs::read(&path).unwrap(), b"retained authority");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn encoded_document_bound_refuses_before_replacing_retained_authority() {
    let directory = temp_dir("canic-oversized-fleet-document");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("journal.json");
    fs::write(&path, b"retained authority").unwrap();
    let value = "\\".repeat(MAX_DOCUMENT_READ_BYTES / 2);

    let error = write_current(&path, &value).unwrap_err();

    let EnsureStateError::Io { source, .. } = error else {
        panic!("expected bounded publication failure");
    };
    assert!(matches!(
        source.get_ref().unwrap().downcast_ref::<NamedWriteError<serde_json::Error>>(),
        Some(NamedWriteError::Producer { source, cleanup_error: None }) if source.is_io()
    ));
    assert_eq!(fs::read(&path).unwrap(), b"retained authority");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    fs::remove_dir_all(directory).unwrap();
}
