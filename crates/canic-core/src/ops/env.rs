use crate::{Error, ThisError, ops::OpsError};
use crate::{
    cdk::{api::canister_self, types::Principal},
    ids::{CanisterRole, SubnetRole},
    model::memory::Env,
};

pub use crate::model::memory::env::EnvData;

///
/// EnvOpsError
///

#[derive(Debug, ThisError)]
pub enum EnvOpsError {
    #[error("env import missing required fields: {0}")]
    MissingFields(String),

    #[error("failed to determine current canister role")]
    CanisterRoleUnavailable,

    #[error("failed to determine current parent principal")]
    ParentPidUnavailable,

    #[error("failed to determine current subnet principal")]
    SubnetPidUnavailable,

    #[error("failed to determine current subnet role")]
    SubnetRoleUnavailable,

    #[error("failed to determine current root principal")]
    RootPidUnavailable,
}

impl From<EnvOpsError> for Error {
    fn from(err: EnvOpsError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// EnvOps
///
/// NOTE:
/// - `try_*` getters are test-only helpers for incomplete env setup.
/// - Non-`try_*` getters assume the environment has been fully initialized
///   during canister startup and will panic if called earlier.
/// - After initialization, absence of environment fields is a programmer error.
///

pub struct EnvOps;

impl EnvOps {
    // ---------------------------------------------------------------------
    // Initialization / import
    // ---------------------------------------------------------------------

    pub fn import(env: EnvData) -> Result<(), Error> {
        let mut missing = Vec::new();
        if env.prime_root_pid.is_none() {
            missing.push("prime_root_pid");
        }
        if env.subnet_role.is_none() {
            missing.push("subnet_role");
        }
        if env.subnet_pid.is_none() {
            missing.push("subnet_pid");
        }
        if env.root_pid.is_none() {
            missing.push("root_pid");
        }
        if env.canister_role.is_none() {
            missing.push("canister_role");
        }
        if env.parent_pid.is_none() {
            missing.push("parent_pid");
        }

        if !missing.is_empty() {
            return Err(EnvOpsError::MissingFields(missing.join(", ")).into());
        }

        Env::import(env);
        Ok(())
    }

    pub fn set_prime_root_pid(pid: Principal) {
        Env::set_prime_root_pid(pid);
    }

    pub fn set_subnet_role(role: SubnetRole) {
        Env::set_subnet_role(role);
    }

    pub fn set_subnet_pid(pid: Principal) {
        Env::set_subnet_pid(pid);
    }

    pub fn set_root_pid(pid: Principal) {
        Env::set_root_pid(pid);
    }

    pub fn set_canister_role(role: CanisterRole) {
        Env::set_canister_role(role);
    }

    // ---------------------------------------------------------------------
    // Environment predicates
    // ---------------------------------------------------------------------

    #[must_use]
    pub fn is_prime_root() -> bool {
        Self::prime_root_pid() == Self::root_pid()
    }

    #[must_use]
    pub fn is_prime_subnet() -> bool {
        Self::subnet_role().is_prime()
    }

    #[must_use]
    pub fn is_root() -> bool {
        Self::root_pid() == canister_self()
    }

    // ---------------------------------------------------------------------
    // Bootstrap / fallible accessors
    // ---------------------------------------------------------------------

    #[cfg(test)]
    pub fn try_get_subnet_role() -> Result<SubnetRole, Error> {
        Env::get_subnet_role().ok_or_else(|| EnvOpsError::SubnetRoleUnavailable.into())
    }

    #[cfg(test)]
    pub fn try_get_canister_role() -> Result<CanisterRole, Error> {
        Env::get_canister_role().ok_or_else(|| EnvOpsError::CanisterRoleUnavailable.into())
    }

    #[cfg(test)]
    pub fn try_get_subnet_pid() -> Result<Principal, Error> {
        Env::get_subnet_pid().ok_or_else(|| EnvOpsError::SubnetPidUnavailable.into())
    }

    #[cfg(test)]
    pub fn try_get_root_pid() -> Result<Principal, Error> {
        Env::get_root_pid().ok_or_else(|| EnvOpsError::RootPidUnavailable.into())
    }

    #[cfg(test)]
    pub fn try_get_prime_root_pid() -> Result<Principal, Error> {
        Env::get_prime_root_pid().ok_or_else(|| EnvOpsError::RootPidUnavailable.into())
    }

    #[cfg(test)]
    pub fn try_get_parent_pid() -> Result<Principal, Error> {
        Env::get_parent_pid().ok_or_else(|| EnvOpsError::ParentPidUnavailable.into())
    }

    // ---------------------------------------------------------------------
    // Steady-state / infallible accessors
    // ---------------------------------------------------------------------

    #[must_use]
    pub fn subnet_role() -> SubnetRole {
        Env::get_subnet_role()
            .expect("EnvOps::subnet_role called before environment initialization")
    }

    #[must_use]
    pub fn canister_role() -> CanisterRole {
        Env::get_canister_role()
            .expect("EnvOps::canister_role called before environment initialization")
    }

    #[must_use]
    pub fn subnet_pid() -> Principal {
        Env::get_subnet_pid().expect("EnvOps::subnet_pid called before environment initialization")
    }

    #[must_use]
    pub fn root_pid() -> Principal {
        Env::get_root_pid().expect("EnvOps::root_pid called before environment initialization")
    }

    #[must_use]
    pub fn prime_root_pid() -> Principal {
        Env::get_prime_root_pid()
            .expect("EnvOps::prime_root_pid called before environment initialization")
    }

    #[must_use]
    pub fn parent_pid() -> Principal {
        Env::get_parent_pid().expect("EnvOps::parent_pid called before environment initialization")
    }

    // ---------------------------------------------------------------------
    // Export
    // ---------------------------------------------------------------------

    /// Export a snapshot of the current environment metadata.
    #[must_use]
    pub fn export() -> EnvData {
        Env::export()
    }
}
