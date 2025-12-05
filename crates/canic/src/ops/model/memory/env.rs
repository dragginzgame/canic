use crate::{
    Error, ThisError,
    core::types::Principal,
    model::memory::Env,
    ops::model::memory::MemoryOpsError,
    types::{CanisterType, SubnetType},
};

pub use crate::model::memory::env::EnvData;

///
/// EnvOpsError
///

#[derive(Debug, ThisError)]
pub enum EnvOpsError {
    #[error("failed to determine current canister type")]
    CanisterTypeUnavailable,

    #[error("failed to determine current parent principal")]
    ParentPidUnavailable,

    #[error("failed to determine current subnet principal")]
    SubnetPidUnavailable,

    #[error("failed to determine current subnet type")]
    SubnetTypeUnavailable,

    #[error("failed to determine current root principal")]
    RootPidUnavailable,
}

impl From<EnvOpsError> for Error {
    fn from(err: EnvOpsError) -> Self {
        MemoryOpsError::from(err).into()
    }
}

///
/// EnvOps
///

pub struct EnvOps;

impl EnvOps {
    pub fn import(env: EnvData) {
        Env::import(env);
    }

    pub fn set_canister_type(ty: CanisterType) {
        Env::set_canister_type(ty);
    }

    pub fn set_root_pid(pid: Principal) {
        Env::set_root_pid(pid);
    }

    pub fn set_prime_root_pid(pid: Principal) {
        Env::set_prime_root_pid(pid);
    }

    pub fn set_subnet_pid(pid: Principal) {
        Env::set_subnet_pid(pid);
    }

    pub fn set_subnet_type(ty: SubnetType) {
        Env::set_subnet_type(ty);
    }

    #[must_use]
    pub fn is_root() -> bool {
        Env::is_root()
    }

    #[must_use]
    pub fn is_prime_root() -> bool {
        Env::is_prime_root()
    }

    pub fn try_get_subnet_type() -> Result<SubnetType, Error> {
        let ty = Env::get_subnet_type().ok_or(EnvOpsError::SubnetTypeUnavailable)?;

        Ok(ty)
    }

    pub fn try_get_canister_type() -> Result<CanisterType, Error> {
        let ty = Env::get_canister_type().ok_or(EnvOpsError::CanisterTypeUnavailable)?;

        Ok(ty)
    }

    pub fn try_get_subnet_pid() -> Result<Principal, Error> {
        let pid = Env::get_subnet_pid().ok_or(EnvOpsError::SubnetPidUnavailable)?;

        Ok(pid)
    }

    pub fn try_get_root_pid() -> Result<Principal, Error> {
        let pid = Env::get_root_pid().ok_or(EnvOpsError::RootPidUnavailable)?;

        Ok(pid)
    }

    pub fn try_get_prime_root_pid() -> Result<Principal, Error> {
        let pid = Env::get_prime_root_pid().ok_or(EnvOpsError::RootPidUnavailable)?;

        Ok(pid)
    }

    pub fn try_get_parent_pid() -> Result<Principal, Error> {
        let pid = Env::get_parent_pid().ok_or(EnvOpsError::ParentPidUnavailable)?;

        Ok(pid)
    }

    /// Export a snapshot of the current environment metadata.
    #[must_use]
    pub fn export() -> EnvData {
        Env::export()
    }
}
