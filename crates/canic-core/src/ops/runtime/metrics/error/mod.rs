//! Shared error classification for runtime metric families.
//!
//! This ops helper maps diagnostics only; it owns no workflow decisions or public labels.

use crate::{InternalError, diagnostics::codes};

///
/// MetricErrorKind
///
/// Internal categories mapped to each family's maintained reason enum.
///

pub(super) enum MetricErrorKind {
    InvalidState,
    ManagementCall,
    PolicyDenied,
    Unknown,
}

impl MetricErrorKind {
    pub(super) fn classify(err: &InternalError) -> Self {
        let code = err.code();
        let public_code = err.public_error().code();
        if code == codes::PLATFORM_FAILED {
            Self::ManagementCall
        } else if code == codes::STATE_CONFLICT
            || public_code == codes::AUTHORITY_UNAUTHORIZED.raw_code()
            || public_code == codes::REQUEST_INVALID.raw_code()
        {
            Self::PolicyDenied
        } else if code == codes::STATE_INVALID
            || code == codes::STATE_FAILED
            || code == codes::LIFECYCLE_FAILED
        {
            Self::InvalidState
        } else {
            Self::Unknown
        }
    }
}
