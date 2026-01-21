//! Environment (self) access checks.
//!
//! This bucket is strictly about the canister's own environment state
//! (prime subnet/root status).

use crate::{access::AccessError, ops::runtime::env::EnvOps};

///
/// Env Checks
///

#[allow(clippy::unused_async)]
pub async fn is_prime_root() -> Result<(), AccessError> {
    if EnvOps::is_prime_root() {
        Ok(())
    } else {
        Err(AccessError::Denied(
            "this endpoint is only available on prime root".to_string(),
        ))
    }
}

#[allow(clippy::unused_async)]
pub async fn is_prime_subnet() -> Result<(), AccessError> {
    if EnvOps::is_prime_subnet() {
        Ok(())
    } else {
        Err(AccessError::Denied(
            "this endpoint is only available on the prime subnet".to_string(),
        ))
    }
}
