use crate::{
    Error, ThisError, access::AccessError, cdk::api::canister_self, ops::runtime::env::EnvOps,
};

///
/// EnvAccessError
///

#[derive(Debug, ThisError)]
pub enum EnvAccessError {
    #[error("this endpoint is only available on the prime subnet")]
    NotPrimeSubnet,

    #[error("operation must be called from the root canister")]
    NotRoot,

    #[error("operation cannot be called from the root canister")]
    IsRoot,
}

impl From<EnvAccessError> for Error {
    fn from(err: EnvAccessError) -> Self {
        AccessError::Env(err).into()
    }
}

///
/// Guards
///

#[allow(clippy::unused_async)]
pub async fn is_prime_subnet() -> Result<(), AccessError> {
    if EnvOps::is_prime_subnet() {
        Ok(())
    } else {
        Err(EnvAccessError::NotPrimeSubnet.into())
    }
}

/// Ensure the caller is the root canister.
pub fn require_root() -> Result<(), Error> {
    let root_pid = EnvOps::root_pid()?;

    if root_pid == canister_self() {
        Ok(())
    } else {
        Err(EnvAccessError::NotRoot.into())
    }
}

/// Ensure the caller is not the root canister.
pub fn deny_root() -> Result<(), Error> {
    let root_pid = EnvOps::root_pid()?;

    if root_pid == canister_self() {
        Err(EnvAccessError::IsRoot.into())
    } else {
        Ok(())
    }
}
