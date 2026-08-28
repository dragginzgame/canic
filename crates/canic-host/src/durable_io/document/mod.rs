//! Module: durable_io::document
//!
//! Responsibility: share bounded canonical encoding/read and exact replacement reconciliation.
//! Does not own: domain validation, document paths, locks, schemas, transitions or retry policy.
//! Boundary: callers validate typed state; this module preserves bytes and filesystem outcomes.

use super::{
    BoundedRegularFileReadError as DurableBoundedRegularFileReadError, RegularFileReadError,
    read_optional_regular_bytes, read_optional_regular_bytes_bounded, write_bytes,
};
use serde::Serialize;
use std::{io, path::Path};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Canonical JSON byte style frozen by one durable document owner.
pub enum CanonicalJsonStyle {
    Compact,
    PrettyNewline,
}

#[derive(Debug)]
/// Canonical encoding failure before publication.
pub enum CanonicalJsonEncodeError {
    Serialization(serde_json::Error),
    TooLarge,
}

#[derive(Debug)]
/// Bounded no-follow read failure before decoding.
pub enum BoundedRegularFileReadError {
    Read(RegularFileReadError),
    TooLarge,
}

#[derive(Debug)]
/// Exact replacement failure after response-loss reconciliation.
pub enum ExactReplaceError {
    Read(RegularFileReadError),
    Write(io::Error),
}

/// Encode one already validated document using one explicit canonical JSON style.
pub fn encode_canonical_json<T: Serialize>(
    document: &T,
    style: CanonicalJsonStyle,
    maximum_bytes: usize,
) -> Result<Vec<u8>, CanonicalJsonEncodeError> {
    let mut bytes = match style {
        CanonicalJsonStyle::Compact => serde_json::to_vec(document),
        CanonicalJsonStyle::PrettyNewline => serde_json::to_vec_pretty(document),
    }
    .map_err(CanonicalJsonEncodeError::Serialization)?;
    if style == CanonicalJsonStyle::PrettyNewline {
        bytes.push(b'\n');
    }
    if bytes.len() > maximum_bytes {
        return Err(CanonicalJsonEncodeError::TooLarge);
    }
    Ok(bytes)
}

/// Read one optional regular no-follow file and enforce its bound before decoding.
pub fn read_optional_bounded_regular_bytes(
    path: &Path,
    maximum_bytes: usize,
) -> Result<Option<Vec<u8>>, BoundedRegularFileReadError> {
    read_optional_regular_bytes_bounded(path, maximum_bytes).map_err(|error| match error {
        DurableBoundedRegularFileReadError::Read(source) => {
            BoundedRegularFileReadError::Read(source)
        }
        DurableBoundedRegularFileReadError::TooLarge => BoundedRegularFileReadError::TooLarge,
    })
}

/// Atomically replace one document and reconcile an uncertain result from exact durable bytes.
pub fn replace_bytes_exact(path: &Path, expected: &[u8]) -> Result<(), ExactReplaceError> {
    replace_bytes_exact_with(path, expected, write_bytes)
}

fn replace_bytes_exact_with(
    path: &Path,
    expected: &[u8],
    replace: impl FnOnce(&Path, &[u8]) -> io::Result<()>,
) -> Result<(), ExactReplaceError> {
    if let Err(source) = replace(path, expected) {
        return match read_optional_regular_bytes(path) {
            Ok(Some(observed)) if observed == expected => Ok(()),
            Ok(_) => Err(ExactReplaceError::Write(source)),
            Err(error) => Err(ExactReplaceError::Read(error)),
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use std::fs;

    #[derive(Serialize)]
    struct Example {
        value: u8,
    }

    #[test]
    fn canonical_styles_and_bounds_are_explicit() {
        let example = Example { value: 7 };
        assert_eq!(
            encode_canonical_json(&example, CanonicalJsonStyle::Compact, 11)
                .expect("compact document"),
            br#"{"value":7}"#
        );
        assert_eq!(
            encode_canonical_json(&example, CanonicalJsonStyle::PrettyNewline, 17)
                .expect("pretty document"),
            b"{\n  \"value\": 7\n}\n"
        );
        assert!(matches!(
            encode_canonical_json(&example, CanonicalJsonStyle::Compact, 10),
            Err(CanonicalJsonEncodeError::TooLarge)
        ));
    }

    #[test]
    fn exact_replacement_reconciles_a_lost_success_response() {
        let root = crate::test_support::temp_dir("durable-document-response-loss");
        let path = root.join("journal.json");
        let expected = b"exact replacement";

        replace_bytes_exact_with(&path, expected, |path, bytes| {
            write_bytes(path, bytes)?;
            Err(io::Error::other("simulated response loss"))
        })
        .expect("exact durable bytes reconcile the uncertain write");

        assert_eq!(fs::read(&path).expect("read reconciled document"), expected);
        fs::remove_dir_all(root).expect("remove document scratch");
    }
}
