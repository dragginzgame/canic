pub mod attestation;
pub mod cycles;
pub mod install;
pub mod intent;
pub mod log;
mod nonroot;
mod root;
pub mod timer;

use crate::{
    InternalError, InternalErrorOrigin,
    config::ConfigModel,
    ops::{
        config::ConfigOps,
        ic::ecdsa::EcdsaOps,
        runtime::{
            env::EnvOps,
            memory::{MemoryRegistryInitSummary, MemoryRegistryOps},
        },
    },
    workflow::{self, prelude::*},
};

pub use nonroot::{
    init_nonroot_canister, init_nonroot_canister_with_attestation_cache,
    post_upgrade_nonroot_canister_after_memory_init,
    post_upgrade_nonroot_canister_after_memory_init_with_attestation_cache,
};
pub use root::{init_root_canister, post_upgrade_root_canister_after_memory_init};

///
/// RuntimeWorkflow
/// Coordinates periodic background services (timers) for Canic canisters.
///

pub struct RuntimeWorkflow;

impl RuntimeWorkflow {
    /// Start timers that should run on all non-root canisters.
    pub fn start_all() {
        workflow::runtime::log::LogRetentionWorkflow::start();
        workflow::runtime::cycles::CycleTrackerWorkflow::start();
    }

    /// Start timers that should run on delegated-auth-aware non-root canisters.
    pub fn start_all_with_attestation_cache() {
        log_delegation_proof_cache_policy();
        workflow::runtime::attestation::AttestationKeyCacheWorkflow::start();
        Self::start_all();
    }

    /// Start timers that should run only on root canisters.
    pub fn start_all_root() -> Result<(), InternalError> {
        EnvOps::require_root().map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("root context required: {err}"),
            )
        })?;

        // start shared timers too
        Self::start_all();

        // root-only services
        workflow::pool::scheduler::PoolSchedulerWorkflow::start();
        Ok(())
    }
}

// Log the resolved verifier proof-cache policy once during runtime startup.
fn log_delegation_proof_cache_policy() {
    match ConfigOps::delegation_proof_cache_policy() {
        Ok(policy) => crate::log!(
            Topic::Auth,
            Info,
            "delegation proof cache policy profile={} capacity={} active_window_secs={}",
            policy.profile.as_str(),
            policy.capacity,
            policy.active_window_secs
        ),
        Err(err) => crate::log!(
            Topic::Auth,
            Warn,
            "delegation proof cache policy unavailable at runtime startup: {err}"
        ),
    }
}

pub(super) fn log_memory_summary(summary: &MemoryRegistryInitSummary) {
    for range in &summary.ranges {
        let used = summary
            .entries
            .iter()
            .filter(|entry| entry.id >= range.start && entry.id <= range.end)
            .count();

        crate::log!(
            Topic::Memory,
            Info,
            "💾 memory.range: {} [{}-{}] ({}/{} slots used)",
            range.crate_name,
            range.start,
            range.end,
            used,
            range.end - range.start + 1,
        );
    }
}

fn init_post_upgrade_memory_registry() -> Result<MemoryRegistryInitSummary, InternalError> {
    MemoryRegistryOps::bootstrap_registry().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("memory init failed: {err}"),
        )
    })
}

pub fn init_memory_registry_post_upgrade() -> Result<MemoryRegistryInitSummary, InternalError> {
    init_post_upgrade_memory_registry()
}

pub(super) fn ensure_root_delegated_auth_crypto_contract() -> Result<(), InternalError> {
    let cfg = ConfigOps::get()?;
    if root_requires_auth_crypto(&cfg) && !EcdsaOps::threshold_management_enabled() {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "delegated auth is configured in canic.toml, but this root build does not include threshold ECDSA management support; enable the `auth-crypto` feature for the root canister build".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn ensure_nonroot_delegated_auth_crypto_contract(
    canister_role: &CanisterRole,
    canister_cfg: &crate::config::schema::CanisterConfig,
) -> Result<(), InternalError> {
    if nonroot_requires_auth_crypto(canister_cfg) && !EcdsaOps::threshold_management_enabled() {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!(
                "canister '{canister_role}' is configured as a delegated auth signer, but this build does not include threshold ECDSA management support; enable the `auth-crypto` feature for that canister build",
            ),
        ));
    }

    Ok(())
}

fn root_requires_auth_crypto(cfg: &ConfigModel) -> bool {
    cfg.auth.delegated_tokens.enabled
        && cfg.subnets.values().any(|subnet| {
            subnet
                .canisters
                .values()
                .any(|canister| canister.delegated_auth.signer || canister.delegated_auth.verifier)
        })
}

const fn nonroot_requires_auth_crypto(
    canister_cfg: &crate::config::schema::CanisterConfig,
) -> bool {
    canister_cfg.delegated_auth.signer
}

#[cfg(test)]
mod tests {
    use super::{nonroot_requires_auth_crypto, root_requires_auth_crypto};
    use crate::{
        config::schema::{CanisterKind, DelegatedAuthCanisterConfig},
        ids::CanisterRole,
        test::config::ConfigTestBuilder,
    };

    #[test]
    fn root_requires_auth_crypto_when_any_delegated_auth_role_exists() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.delegated_auth = DelegatedAuthCanisterConfig {
            signer: false,
            verifier: true,
        };

        let cfg = ConfigTestBuilder::new()
            .with_prime_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_prime_canister("user_hub", verifier_cfg)
            .build();

        assert!(root_requires_auth_crypto(&cfg));
    }

    #[test]
    fn root_does_not_require_auth_crypto_without_delegated_auth_roles() {
        let cfg = ConfigTestBuilder::new().build();

        assert!(!root_requires_auth_crypto(&cfg));
    }

    #[test]
    fn verifier_only_nonroot_does_not_require_auth_crypto() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.delegated_auth = DelegatedAuthCanisterConfig {
            signer: false,
            verifier: true,
        };

        assert!(!nonroot_requires_auth_crypto(&verifier_cfg));
    }

    #[test]
    fn signer_nonroot_requires_auth_crypto() {
        let mut signer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        signer_cfg.delegated_auth = DelegatedAuthCanisterConfig {
            signer: true,
            verifier: true,
        };

        assert!(nonroot_requires_auth_crypto(&signer_cfg));
    }
}
