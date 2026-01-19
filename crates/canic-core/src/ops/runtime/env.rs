use crate::{
    InternalError,
    cdk::api::canister_self,
    ids::SubnetRole,
    ops::{prelude::*, runtime::RuntimeOpsError},
    storage::stable::env::{Env, EnvData},
};
use thiserror::Error as ThisError;

///
/// EnvOpsError
///

#[derive(Debug, ThisError)]
pub enum EnvOpsError {
    #[error("failed to determine current canister role")]
    CanisterRoleUnavailable,

    #[error("env import missing required fields: {0}")]
    MissingFields(String),

    #[error("failed to determine current prime root principal")]
    PrimeRootPidUnavailable,

    #[error("failed to determine current root principal")]
    RootPidUnavailable,

    #[error("failed to determine current subnet principal")]
    SubnetPidUnavailable,

    #[error("failed to determine current subnet role")]
    SubnetRoleUnavailable,
}

impl From<EnvOpsError> for InternalError {
    fn from(err: EnvOpsError) -> Self {
        RuntimeOpsError::from(err).into()
    }
}

///
/// EnvOps
/// NOTE:
/// - Non-`try_*` getters assume the environment has been fully initialized
///   during canister startup and will return errors if called earlier.
/// - After initialization, absence of environment fields is a programmer error.
///

pub struct EnvOps;

impl EnvOps {
    // ---------------------------------------------------------------------
    // Environment predicates
    // ---------------------------------------------------------------------

    #[must_use]
    pub fn is_prime_root() -> bool {
        let Some(prime_root) = Env::get_prime_root_pid() else {
            return false;
        };
        let Some(root_pid) = Env::get_root_pid() else {
            return false;
        };

        prime_root == root_pid
    }

    #[must_use]
    pub fn is_prime_subnet() -> bool {
        Env::get_subnet_role().is_some_and(|role| role.is_prime())
    }

    #[must_use]
    pub fn is_root() -> bool {
        Env::get_root_pid().is_some_and(|pid| pid == canister_self())
    }

    /// Returns true when the build is configured for uncertified-runtime testing.
    #[must_use]
    pub const fn is_uncertified_runtime() -> bool {
        cfg!(feature = "uncertified-testing")
    }

    // ---------------------------------------------------------------------
    // Steady-state / required accessors
    // (env must be initialized; missing values are errors)
    // ---------------------------------------------------------------------

    pub fn subnet_role() -> Result<SubnetRole, InternalError> {
        Env::get_subnet_role().ok_or_else(|| EnvOpsError::SubnetRoleUnavailable.into())
    }

    pub fn canister_role() -> Result<CanisterRole, InternalError> {
        Env::get_canister_role().ok_or_else(|| EnvOpsError::CanisterRoleUnavailable.into())
    }

    pub fn subnet_pid() -> Result<Principal, InternalError> {
        Env::get_subnet_pid().ok_or_else(|| EnvOpsError::SubnetPidUnavailable.into())
    }

    pub fn root_pid() -> Result<Principal, InternalError> {
        Env::get_root_pid().ok_or_else(|| EnvOpsError::RootPidUnavailable.into())
    }

    pub fn prime_root_pid() -> Result<Principal, InternalError> {
        Env::get_prime_root_pid().ok_or_else(|| EnvOpsError::PrimeRootPidUnavailable.into())
    }

    // ---------------------------------------------------------------------
    // Setters
    // ---------------------------------------------------------------------

    /// Update the subnet PID after init.
    ///
    /// This value is resolved asynchronously from the IC and may
    /// change after upgrade or during bootstrap.
    pub fn set_subnet_pid(pid: Principal) {
        Env::set_subnet_pid(pid);
    }

    // ---------------------------------------------------------------------
    // Data / Import
    // ---------------------------------------------------------------------

    /// Export the current environment metadata.
    #[must_use]
    pub fn snapshot() -> EnvData {
        Env::export()
    }

    pub fn import(data: EnvData) -> Result<(), InternalError> {
        let missing = required_fields_missing(&data);
        if !missing.is_empty() {
            return Err(EnvOpsError::MissingFields(missing.join(", ")).into());
        }

        Env::import(data);

        Ok(())
    }

    // ---------------------------------------------------------------------
    // Restore
    // ---------------------------------------------------------------------

    // NOTE:
    // Restore functions are intended to be called ONLY from lifecycle adapters.
    // Calling them during steady-state execution is a logic error.

    /// Restore root environment context after upgrade.
    ///
    /// Root identity and subnet metadata must already be present.
    pub fn restore_root() -> Result<(), InternalError> {
        // Ensure environment was initialized before upgrade
        Self::assert_initialized()?;

        // Root canister role is implicit
        Env::set_canister_role(CanisterRole::ROOT);
        Ok(())
    }

    /// Restore canister role context after upgrade.
    ///
    /// Environment data is expected to already exist in stable memory.
    /// Failure indicates a programmer error or corrupted state.
    pub fn restore_role(role: CanisterRole) -> Result<(), InternalError> {
        // Ensure environment was initialized before upgrade
        Self::assert_initialized()?;

        // Restore the role context explicitly
        Env::set_canister_role(role);
        Ok(())
    }

    fn assert_initialized() -> Result<(), InternalError> {
        let mut missing = Vec::new();
        if Env::get_root_pid().is_none() {
            missing.push("root_pid");
        }
        if Env::get_subnet_pid().is_none() {
            missing.push("subnet_pid");
        }
        if Env::get_prime_root_pid().is_none() {
            missing.push("prime_root_pid");
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(EnvOpsError::MissingFields(missing.join(", ")).into())
        }
    }
}

fn required_fields_missing(data: &EnvData) -> Vec<&'static str> {
    let mut missing = Vec::new();

    if data.prime_root_pid.is_none() {
        missing.push("prime_root_pid");
    }
    if data.subnet_role.is_none() {
        missing.push("subnet_role");
    }
    if data.subnet_pid.is_none() {
        missing.push("subnet_pid");
    }
    if data.root_pid.is_none() {
        missing.push("root_pid");
    }
    if data.canister_role.is_none() {
        missing.push("canister_role");
    }
    if data.parent_pid.is_none() {
        missing.push("parent_pid");
    }

    missing
}
