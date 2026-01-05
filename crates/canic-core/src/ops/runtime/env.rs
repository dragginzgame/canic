use crate::{
    Error, ThisError,
    cdk::api::canister_self,
    ids::SubnetRole,
    ops::{prelude::*, runtime::RuntimeOpsError},
    storage::stable::env::{Env, EnvData},
};

///
/// EnvOpsError
///

#[derive(Debug, ThisError)]
pub enum EnvOpsError {
    #[error("failed to determine current canister role")]
    CanisterRoleUnavailable,

    #[error("env import missing required fields: {0}")]
    MissingFields(String),

    #[error("failed to determine current parent principal")]
    ParentPidUnavailable,

    #[error("failed to determine current prime root principal")]
    PrimeRootPidUnavailable,

    #[error("failed to determine current root principal")]
    RootPidUnavailable,

    #[error("failed to determine current subnet principal")]
    SubnetPidUnavailable,

    #[error("failed to determine current subnet role")]
    SubnetRoleUnavailable,
}

impl From<EnvOpsError> for Error {
    fn from(err: EnvOpsError) -> Self {
        RuntimeOpsError::from(err).into()
    }
}

///
/// EnvSnapshot
/// Internal, operational snapshot of environment state.
///
/// - May be incomplete during initialization
/// - Not stable or serialized
/// - Used only by workflow and ops
///

pub struct EnvSnapshot {
    pub prime_root_pid: Option<Principal>,
    pub subnet_role: Option<SubnetRole>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}

impl From<EnvData> for EnvSnapshot {
    fn from(data: EnvData) -> Self {
        Self {
            prime_root_pid: data.prime_root_pid,
            subnet_role: data.subnet_role,
            subnet_pid: data.subnet_pid,
            root_pid: data.root_pid,
            canister_role: data.canister_role,
            parent_pid: data.parent_pid,
        }
    }
}

impl TryFrom<EnvSnapshot> for EnvData {
    type Error = EnvOpsError;

    fn try_from(snapshot: EnvSnapshot) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();

        if snapshot.prime_root_pid.is_none() {
            missing.push("prime_root_pid");
        }
        if snapshot.subnet_role.is_none() {
            missing.push("subnet_role");
        }
        if snapshot.subnet_pid.is_none() {
            missing.push("subnet_pid");
        }
        if snapshot.root_pid.is_none() {
            missing.push("root_pid");
        }
        if snapshot.canister_role.is_none() {
            missing.push("canister_role");
        }
        if snapshot.parent_pid.is_none() {
            missing.push("parent_pid");
        }

        if !missing.is_empty() {
            return Err(EnvOpsError::MissingFields(missing.join(", ")));
        }

        Ok(Self {
            prime_root_pid: snapshot.prime_root_pid,
            subnet_role: snapshot.subnet_role,
            subnet_pid: snapshot.subnet_pid,
            root_pid: snapshot.root_pid,
            canister_role: snapshot.canister_role,
            parent_pid: snapshot.parent_pid,
        })
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

    // ---------------------------------------------------------------------
    // Steady-state / required accessors
    // (env must be initialized; missing values are errors)
    // ---------------------------------------------------------------------

    pub fn subnet_role() -> Result<SubnetRole, Error> {
        Env::get_subnet_role().ok_or_else(|| EnvOpsError::SubnetRoleUnavailable.into())
    }

    pub fn canister_role() -> Result<CanisterRole, Error> {
        Env::get_canister_role().ok_or_else(|| EnvOpsError::CanisterRoleUnavailable.into())
    }

    pub fn subnet_pid() -> Result<Principal, Error> {
        Env::get_subnet_pid().ok_or_else(|| EnvOpsError::SubnetPidUnavailable.into())
    }

    pub fn root_pid() -> Result<Principal, Error> {
        Env::get_root_pid().ok_or_else(|| EnvOpsError::RootPidUnavailable.into())
    }

    pub fn prime_root_pid() -> Result<Principal, Error> {
        Env::get_prime_root_pid().ok_or_else(|| EnvOpsError::PrimeRootPidUnavailable.into())
    }

    pub fn parent_pid() -> Result<Principal, Error> {
        Env::get_parent_pid().ok_or_else(|| EnvOpsError::ParentPidUnavailable.into())
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
    // Snapshot / Import
    // ---------------------------------------------------------------------

    /// Export a snapshot of the current environment metadata.
    #[must_use]
    pub fn snapshot() -> EnvSnapshot {
        let data = Env::export(); // storage-level export

        data.into()
    }

    pub fn import(snapshot: EnvSnapshot) -> Result<(), Error> {
        let data: EnvData = snapshot.try_into()?;
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
    pub fn restore_root() -> Result<(), Error> {
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
    pub fn restore_role(role: CanisterRole) -> Result<(), Error> {
        // Ensure environment was initialized before upgrade
        Self::assert_initialized()?;

        // Restore the role context explicitly
        Env::set_canister_role(role);
        Ok(())
    }

    fn assert_initialized() -> Result<(), Error> {
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
