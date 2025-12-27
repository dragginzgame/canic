use crate::{
    Error, ThisError,
    cdk::{api::canister_self, types::Principal},
    dto::topology::SubnetIdentity,
    ids::{CanisterRole, SubnetRole},
    model::memory::Env,
    ops::OpsError,
};

pub use crate::model::memory::env::EnvData;

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

    #[error("failed to determine current root principal")]
    RootPidUnavailable,

    #[error("failed to determine current subnet principal")]
    SubnetPidUnavailable,

    #[error("failed to determine current subnet role")]
    SubnetRoleUnavailable,
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

    /// Initialize environment state for the root canister during init.
    ///
    /// This must only be called from the IC `init` hook.
    pub fn init_root(identity: SubnetIdentity) {
        let self_pid = canister_self();

        let (subnet_pid, subnet_role, prime_root_pid) = match identity {
            SubnetIdentity::Prime => {
                // Prime subnet: root == prime root == subnet
                (self_pid, SubnetRole::PRIME, self_pid)
            }

            SubnetIdentity::Standard(params) => {
                // Standard subnet syncing from prime
                (self_pid, params.subnet_type, params.prime_root_pid)
            }

            SubnetIdentity::Manual(pid) => {
                // Test/support only: explicit subnet override
                (pid, SubnetRole::MANUAL, pid)
            }
        };

        let env = EnvData {
            prime_root_pid: Some(prime_root_pid),
            root_pid: Some(self_pid),
            subnet_pid: Some(subnet_pid),
            subnet_role: Some(subnet_role),
            canister_role: Some(CanisterRole::ROOT),
            parent_pid: None,
        };

        if let Err(err) = Self::import(env) {
            panic!("EnvOps::init_root failed: {err}");
        }
    }

    /// Initialize environment state for a non-root canister during init.
    ///
    /// This function must only be called from the IC `init` hook.
    pub fn init(mut env: EnvData, role: CanisterRole) {
        // Override contextual role (do not trust payload blindly)
        env.canister_role = Some(role);

        // Import validates required fields and persists
        if let Err(err) = Self::import(env) {
            panic!("EnvOps::init failed: {err}");
        }
    }

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
    // Restore
    // ---------------------------------------------------------------------

    // NOTE:
    // Restore functions are intended to be called ONLY from lifecycle adapters.
    // Calling them during steady-state execution is a logic error.

    /// Restore root environment context after upgrade.
    ///
    /// Root identity and subnet metadata must already be present.
    pub fn restore_root() {
        // Ensure environment was initialized before upgrade
        assert_initialized();

        // Root canister role is implicit
        Env::set_canister_role(CanisterRole::ROOT);
    }

    /// Restore canister role context after upgrade.
    ///
    /// Environment data is expected to already exist in stable memory.
    /// Failure indicates a programmer error or corrupted state.
    pub fn restore_role(role: CanisterRole) {
        // Ensure environment was initialized before upgrade
        assert_initialized();

        // Restore the role context explicitly
        Env::set_canister_role(role);
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

fn assert_initialized() {
    assert!(
        Env::get_root_pid().is_some()
            && Env::get_subnet_pid().is_some()
            && Env::get_prime_root_pid().is_some(),
        "EnvOps called before environment initialization"
    );
}
