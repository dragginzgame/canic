pub mod icrc;
pub mod policy;

use crate::{PublicError, ThisError};

///
/// DomainError
///

#[derive(Debug, ThisError)]
pub enum DomainError {
    #[error(transparent)]
    Policy(#[from] policy::PolicyError),
}

impl DomainError {
    #[allow(dead_code)]
    fn public(&self) -> PublicError {
        unreachable!("DomainError::public is not yet semantically diverse");
    }
}
