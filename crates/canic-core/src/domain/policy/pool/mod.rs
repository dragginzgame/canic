pub mod admissibility;
pub mod authority;

use crate::{Error, ThisError, cdk::candid::Principal, domain::policy::PolicyError};

///
/// PoolPolicyError
/// All semantic denials related to pool policy.
///
/// These errors:
/// - are side-effect free
/// - are safe to bubble through ops/workflows
/// - describe *why* an action is not permitted
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum PoolPolicyError {
    // Admissibility
    #[error("pool entry blocked for {0}: canister is still registered in subnet registry")]
    RegisteredInSubnet(Principal),

    #[error("pool entry blocked for {pid}: local non-importable: {details}")]
    NonImportableOnLocal { pid: Principal, details: String },

    // Recycling
    #[error("pool entry blocked for {0}: canister not registered in subnet registry")]
    NotRegisteredInSubnet(Principal),

    // Authority
    #[error("caller is not authorized to perform pool operation")]
    NotAuthorized,
}

impl From<PoolPolicyError> for Error {
    fn from(err: PoolPolicyError) -> Self {
        PolicyError::from(err).into()
    }
}
