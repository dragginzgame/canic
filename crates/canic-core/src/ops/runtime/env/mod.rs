pub mod mapper;

use crate::{
    InternalError,
    cdk::api::canister_self,
    ids::SubnetRole,
    ops::{prelude::*, runtime::RuntimeOpsError},
    storage::stable::env::{Env, EnvRecord},
    view::env::ValidatedEnv,
};
use crate::{dto::env::EnvSnapshotResponse, ops::runtime::env::mapper::EnvRecordMapper};
use canic_memory::runtime::registry::MemoryRegistryRuntime;
use thiserror::Error as ThisError;

///
/// EnvOpsError
///

#[derive(Debug, ThisError)]
pub enum EnvOpsError {
    #[error("failed to determine current canister role")]
    CanisterRoleUnavailable,

    #[error("env missing required fields: {0}")]
    MissingFields(String),

    #[error("failed to determine current prime root principal")]
    PrimeRootPidUnavailable,

    #[error("failed to determine current root principal")]
    RootPidUnavailable,

    #[error("failed to determine current subnet principal")]
    SubnetPidUnavailable,

    #[error("failed to determine current subnet role")]
    SubnetRoleUnavailable,

    #[error("operation must be called from the root canister")]
    NotRoot,

    #[error("operation cannot be called from the root canister")]
    IsRoot,

    #[error("root_pid is immutable once initialized (existing {existing}, incoming {incoming})")]
    RootPidImmutable {
        existing: Principal,
        incoming: Principal,
    },

    #[error("memory registry must be initialized before env restore")]
    MemoryRegistryNotInitialized,
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

    pub fn require_root() -> Result<(), InternalError> {
        let root_pid = Env::get_root_pid().ok_or(EnvOpsError::RootPidUnavailable)?;

        if root_pid == canister_self() {
            Ok(())
        } else {
            Err(EnvOpsError::NotRoot.into())
        }
    }

    pub fn deny_root() -> Result<(), InternalError> {
        let root_pid = Env::get_root_pid().ok_or(EnvOpsError::RootPidUnavailable)?;

        if root_pid == canister_self() {
            Err(EnvOpsError::IsRoot.into())
        } else {
            Ok(())
        }
    }

    // ---------------------------------------------------------------------
    // Steady-state / required accessors
    // (env must be initialized; missing values are errors)
    // ---------------------------------------------------------------------

    /// SAFETY: Env must be initialized; do not call during init/post_upgrade.
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
    pub fn snapshot() -> EnvRecord {
        Env::export()
    }

    /// Export the current environment metadata as a DTO.
    #[must_use]
    pub fn snapshot_response() -> EnvSnapshotResponse {
        EnvRecordMapper::record_to_view(&Env::export())
    }

    /// Return any missing required fields for a complete environment snapshot.
    #[must_use]
    pub fn missing_required_fields() -> Vec<&'static str> {
        let data = Env::export();
        required_fields_missing(&data)
    }

    pub fn import(data: EnvRecord) -> Result<(), InternalError> {
        let missing = required_fields_missing(&data);
        if !missing.is_empty() {
            return Err(EnvOpsError::MissingFields(missing.join(", ")).into());
        }

        // `root_pid` is write-once: first initialization may set it, but any
        // subsequent import must preserve the same root authority.
        let incoming_root_pid = data
            .root_pid
            .ok_or_else(|| EnvOpsError::MissingFields("root_pid".to_string()))?;
        ensure_root_pid_immutable(Env::get_root_pid(), incoming_root_pid)?;

        Env::import(data);

        Ok(())
    }

    pub fn import_validated(validated: ValidatedEnv) -> Result<(), InternalError> {
        let record = EnvRecordMapper::validated_to_record(validated);
        Self::import(record)
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
        Self::assert_memory_registry_initialized()?;

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
        Self::assert_memory_registry_initialized()?;

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
        if Env::get_subnet_role().is_none() {
            missing.push("subnet_role");
        }
        if Env::get_parent_pid().is_none() {
            missing.push("parent_pid");
        }
        if Env::get_canister_role().is_none() {
            missing.push("canister_role");
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(EnvOpsError::MissingFields(missing.join(", ")).into())
        }
    }

    fn assert_memory_registry_initialized() -> Result<(), InternalError> {
        let initialized = MemoryRegistryRuntime::is_initialized();
        debug_assert!(
            initialized,
            "memory registry must be initialized before env restore"
        );

        if initialized {
            Ok(())
        } else {
            Err(EnvOpsError::MemoryRegistryNotInitialized.into())
        }
    }
}

fn required_fields_missing(data: &EnvRecord) -> Vec<&'static str> {
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

fn ensure_root_pid_immutable(
    existing: Option<Principal>,
    incoming: Principal,
) -> Result<(), EnvOpsError> {
    if let Some(existing) = existing
        && existing != incoming
    {
        return Err(EnvOpsError::RootPidImmutable { existing, incoming });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::{CanisterRole, SubnetRole},
        storage::stable::env::Env,
        test::seams,
    };

    struct EnvRestore(EnvRecord);

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            Env::import(self.0.clone());
        }
    }

    fn env_record(root_pid: Principal) -> EnvRecord {
        EnvRecord {
            prime_root_pid: Some(root_pid),
            subnet_role: Some(SubnetRole::PRIME),
            subnet_pid: Some(root_pid),
            root_pid: Some(root_pid),
            canister_role: Some(CanisterRole::ROOT),
            parent_pid: Some(root_pid),
        }
    }

    #[test]
    fn root_pid_immutable_allows_first_set() {
        assert!(ensure_root_pid_immutable(None, seams::p(1)).is_ok());
    }

    #[test]
    fn root_pid_immutable_rejects_change() {
        let existing = seams::p(1);
        let incoming = seams::p(2);
        let err = ensure_root_pid_immutable(Some(existing), incoming)
            .expect_err("root pid change must be rejected");

        match err {
            EnvOpsError::RootPidImmutable {
                existing: got_existing,
                incoming: got_incoming,
            } => {
                assert_eq!(got_existing, existing);
                assert_eq!(got_incoming, incoming);
            }
            other => panic!("unexpected env error: {other:?}"),
        }
    }

    #[test]
    fn import_rejects_root_pid_change_after_initialization() {
        let _guard = seams::lock();
        let original = Env::export();
        let _restore = EnvRestore(original);

        let initial_root = seams::p(11);
        EnvOps::import(env_record(initial_root)).expect("initial import should succeed");

        let changed_root = seams::p(12);
        let result = EnvOps::import(env_record(changed_root));
        assert!(result.is_err(), "changing root pid must fail");

        let snapshot = EnvOps::snapshot();
        assert_eq!(snapshot.root_pid, Some(initial_root));
    }
}
