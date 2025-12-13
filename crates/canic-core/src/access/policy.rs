use crate::{Error, ThisError, access::AccessError};

///
/// PolicyError
///

#[derive(Debug, ThisError)]
pub enum PolicyError {}

impl From<PolicyError> for Error {
    fn from(err: PolicyError) -> Self {
        AccessError::PolicyError(err).into()
    }
}
