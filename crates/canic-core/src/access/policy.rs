use crate::{Error, ThisError, access::AccessError, ops::model::memory::env::EnvOps};

///
/// PolicyError
///

#[derive(Debug, ThisError)]
pub enum PolicyError {
    #[error("this endpoint is only available on the prime subnet")]
    NotPrimeSubnet,
}

impl From<PolicyError> for Error {
    fn from(err: PolicyError) -> Self {
        AccessError::PolicyError(err).into()
    }
}

///
/// Policies
///

pub fn is_prime_subnet() -> Result<(), Error> {
    if EnvOps::is_prime_subnet() {
        Ok(())
    } else {
        Err(PolicyError::NotPrimeSubnet.into())
    }
}
