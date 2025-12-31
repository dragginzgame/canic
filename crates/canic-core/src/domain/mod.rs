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
    #[expect(dead_code)]
    #[allow(clippy::unused_self)]
    fn public(&self) -> PublicError {
        unreachable!("DomainError::public is not yet semantically diverse");
    }
}
