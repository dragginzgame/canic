//! Project filesystem publication failures into Host's existing I/O error boundaries.
//!
//! Retain the typed phase and cleanup cause. A visible publication is never an
//! ordinary create conflict, even when its final synchronization cause has that kind.

use ic_host_fs::durable::NamedWriteError;
use std::io;

/// Retain publication phase and cleanup evidence at Canic I/O boundaries.
#[must_use]
pub fn io_error(error: NamedWriteError<io::Error>) -> io::Error {
    let kind = match &error {
        NamedWriteError::BeforePublication { source, .. } => source.kind(),
        NamedWriteError::Producer { source, .. }
            if source.kind() != io::ErrorKind::AlreadyExists =>
        {
            source.kind()
        }
        NamedWriteError::Producer { .. } | NamedWriteError::AfterPublication { .. } => {
            io::ErrorKind::Other
        }
    };
    io::Error::new(kind, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepublication_conflict_retains_native_kind_and_cleanup_evidence() {
        let error = io_error(NamedWriteError::BeforePublication {
            source: io::ErrorKind::AlreadyExists.into(),
            cleanup_error: Some(io::ErrorKind::PermissionDenied.into()),
        });
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(matches!(
            error.get_ref().unwrap().downcast_ref::<NamedWriteError<io::Error>>(),
            Some(NamedWriteError::BeforePublication { source, cleanup_error: Some(cleanup) })
                if source.kind() == io::ErrorKind::AlreadyExists
                    && cleanup.kind() == io::ErrorKind::PermissionDenied
        ));
    }

    #[test]
    fn producer_failure_cannot_be_treated_as_a_destination_conflict() {
        let error = io_error(NamedWriteError::Producer {
            source: io::ErrorKind::AlreadyExists.into(),
            cleanup_error: None,
        });
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(matches!(
            error.get_ref().unwrap().downcast_ref::<NamedWriteError<io::Error>>(),
            Some(NamedWriteError::Producer { source, cleanup_error: None })
                if source.kind() == io::ErrorKind::AlreadyExists
        ));
    }

    #[test]
    fn after_publication_failure_cannot_be_treated_as_a_create_conflict() {
        let error = io_error(NamedWriteError::AfterPublication {
            source: io::ErrorKind::AlreadyExists.into(),
        });
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(matches!(
            error.get_ref().unwrap().downcast_ref::<NamedWriteError<io::Error>>(),
            Some(NamedWriteError::AfterPublication { source })
                if source.kind() == io::ErrorKind::AlreadyExists
        ));
    }
}
