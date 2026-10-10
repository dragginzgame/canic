//! Convert access refusals into the canonical public error contract.
//!
//! Endpoint authentication remains owned by access; no authority or state is changed.

use crate::access::AccessError;

impl From<AccessError> for canic_contracts::dto::error::Error {
    fn from(err: AccessError) -> Self {
        match err {
            AccessError::Internal(error) => error.into(),
            error => {
                let diagnostic = error
                    .diagnostic_codes()
                    .expect("non-internal access errors have registered reasons");
                Self::from_registered(diagnostic.public)
            }
        }
    }
}
