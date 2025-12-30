use crate::{
    Error, ThisError,
    cdk::{api::canister_self, types::Principal},
    dto::{env::EnvView, subnet::SubnetIdentity},
    ids::{CanisterRole, SubnetRole},
    infra::ic::{Network, build_network},
    model::memory::Env,
    ops::{
        adapter::env::{env_data_from_view, env_data_to_view},
        runtime::RuntimeOpsError,
    },
};

use crate::model::memory::env::EnvData;

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
        RuntimeOpsError::from(err).into()
    }
}

///
/// EnvOps
///
/// NOTE:
/// - `try_*` getters are test-only helpers for incomplete env setup.
/// - Non-`try_*` getters assume the environment has been fully initialized
///   during canister startup and will return errors if called earlier.
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
    pub fn init_root(identity: SubnetIdentity) -> Result<(), Error> {
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
            parent_pid: Some(prime_root_pid),
        };

        Self::import_data(env)
    }

    /// Initialize environment state for a non-root canister during init.
    ///
    /// This function must only be called from the IC `init` hook.
    pub fn init(env: EnvView, role: CanisterRole) -> Result<(), Error> {
        let mut env = env_data_from_view(env);
        // Override contextual role (do not trust payload blindly)
        env.canister_role = Some(role.clone());
        env = ensure_nonroot_env(role, env)?;

        // Import validates required fields and persists
        Self::import_data(env)
    }

    pub fn import(env: EnvView) -> Result<(), Error> {
        let env = env_data_from_view(env);
        Self::import_data(env)
    }

    fn import_data(env: EnvData) -> Result<(), Error> {
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
        Env::get_prime_root_pid().ok_or_else(|| EnvOpsError::RootPidUnavailable.into())
    }

    pub fn parent_pid() -> Result<Principal, Error> {
        Env::get_parent_pid().ok_or_else(|| EnvOpsError::ParentPidUnavailable.into())
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
        assert_initialized()?;

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
        assert_initialized()?;

        // Restore the role context explicitly
        Env::set_canister_role(role);
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Export
    // ---------------------------------------------------------------------

    /// Export a snapshot of the current environment metadata.
    #[must_use]
    pub fn export() -> EnvView {
        env_data_to_view(Env::export())
    }
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

fn ensure_nonroot_env(canister_role: CanisterRole, mut env: EnvData) -> Result<EnvData, Error> {
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

    if missing.is_empty() {
        return Ok(env);
    }

    if build_network() == Some(Network::Ic) {
        return Err(EnvOpsError::MissingFields(missing.join(", ")).into());
    }

    let root_pid = Principal::from_slice(&[0xBB; 29]);
    let subnet_pid = Principal::from_slice(&[0xAA; 29]);

    env.prime_root_pid.get_or_insert(root_pid);
    env.subnet_role.get_or_insert(SubnetRole::PRIME);
    env.subnet_pid.get_or_insert(subnet_pid);
    env.root_pid.get_or_insert(root_pid);
    env.canister_role.get_or_insert(canister_role);
    env.parent_pid.get_or_insert(root_pid);

    Ok(env)
}
