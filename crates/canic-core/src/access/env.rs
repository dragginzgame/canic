use crate::{
    access::{AccessError, AccessRuleError, AccessRuleResult},
    cdk::{api::is_controller as caller_is_controller, types::Principal},
    config::Config,
    ops::runtime::env::EnvOps,
};
use thiserror::Error as ThisError;

///
/// EnvAccessError
///

#[derive(Debug, ThisError)]
pub enum EnvAccessError {
    #[error("operation cannot be called from the root canister")]
    IsRoot,

    #[error("operation must be called from the root canister")]
    NotRoot,

    #[error("this endpoint is only available on the prime subnet")]
    NotPrimeSubnet,

    #[error("this endpoint is only available on prime root")]
    NotPrimeRoot,
}

///
/// Env Checks
///

#[allow(clippy::unused_async)]
pub async fn self_is_root() -> Result<(), AccessError> {
    if EnvOps::is_root() {
        Ok(())
    } else {
        Err(EnvAccessError::NotRoot.into())
    }
}

#[allow(clippy::unused_async)]
pub async fn self_is_not_root() -> Result<(), AccessError> {
    if EnvOps::is_root() {
        Err(EnvAccessError::IsRoot.into())
    } else {
        Ok(())
    }
}

#[allow(clippy::unused_async)]
pub async fn is_prime_root() -> Result<(), AccessError> {
    if EnvOps::is_prime_root() {
        Ok(())
    } else {
        Err(EnvAccessError::NotPrimeRoot.into())
    }
}

#[allow(clippy::unused_async)]
pub async fn is_prime_subnet() -> Result<(), AccessError> {
    if EnvOps::is_prime_subnet() {
        Ok(())
    } else {
        Err(EnvAccessError::NotPrimeSubnet.into())
    }
}

// -----------------------------------------------------------------------------
// Caller rules
// -----------------------------------------------------------------------------

/// Require that the caller controls the current canister.
/// Allows controller-only maintenance calls.
#[must_use]
pub fn is_controller(caller: Principal) -> AccessRuleResult {
    Box::pin(async move {
        if caller_is_controller(&caller) {
            Ok(())
        } else {
            Err(AccessRuleError::NotController(caller).into())
        }
    })
}

/// Require that the caller appears in the active whitelist (IC deployments).
/// No-op on local builds; enforces whitelist on IC.
#[must_use]
pub fn is_whitelisted(caller: Principal) -> AccessRuleResult {
    Box::pin(async move {
        let cfg = Config::try_get().ok_or_else(|| {
            AccessRuleError::DependencyUnavailable("config not initialized".to_string())
        })?;

        if !cfg.is_whitelisted(&caller) {
            return Err(AccessRuleError::NotWhitelisted(caller).into());
        }

        Ok(())
    })
}
