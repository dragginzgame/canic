use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::ConfigModel,
    ids::CanisterRole,
    ops::{
        auth::AuthOps,
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
        runtime::env::EnvOps,
    },
    workflow::prelude::*,
};

///
/// DelegatedTokenSignerPrewarmPlan
///

struct DelegatedTokenSignerPrewarmPlan {
    shard_pid: Principal,
}

impl DelegatedTokenSignerPrewarmPlan {
    // Build the local signer material prewarm plan used by runtime lifecycle.
    fn default() -> Self {
        Self {
            shard_pid: IcOps::canister_self(),
        }
    }
}

///
/// RuntimeAuthWorkflow
///
/// Owns delegated-auth runtime startup checks and auth-specific runtime boot
/// logging for root and non-root canisters.
///

pub struct RuntimeAuthWorkflow;

impl RuntimeAuthWorkflow {
    /// Fail fast when root delegated-auth config requires threshold ECDSA support.
    pub fn ensure_root_crypto_contract() -> Result<(), InternalError> {
        let cfg = ConfigOps::get()?;
        if root_requires_auth_crypto(&cfg) && !EcdsaOps::threshold_management_enabled() {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "delegated auth is configured in canic.toml, but this root build does not include threshold ECDSA management support; enable the `auth-crypto` feature for the root canister build".to_string(),
            ));
        }

        Ok(())
    }

    /// Fail fast when one delegated signer canister lacks threshold ECDSA support.
    pub fn ensure_nonroot_crypto_contract(
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

    /// Check local signer key material when the current canister is a delegated signer.
    pub async fn check_signer_key_material() -> Result<(), InternalError> {
        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        let canister_cfg = ConfigOps::current_canister()?;
        if !delegated_tokens_cfg.enabled || !canister_cfg.auth.delegated_token_signer {
            return Ok(());
        }

        let plan = DelegatedTokenSignerPrewarmPlan::default();
        let shard_public_key_sec1 = AuthOps::local_shard_public_key_sec1(plan.shard_pid).await?;

        crate::log!(
            Topic::Auth,
            Info,
            "delegation signer auth material checked shard={} shard_public_key_bytes={}",
            plan.shard_pid,
            shard_public_key_sec1.len()
        );

        Ok(())
    }

    /// Ensure root delegated auth trust material is published in subnet state.
    pub async fn publish_root_delegated_key_to_subnet_state() -> Result<(), InternalError> {
        EnvOps::require_root()?;

        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        if !delegated_tokens_cfg.enabled {
            return Ok(());
        }

        AuthOps::publish_delegated_token_root_key_material().await
    }
}

// Decide whether the root runtime must carry threshold-ECDSA management support.
fn root_requires_auth_crypto(cfg: &ConfigModel) -> bool {
    cfg.subnets.values().any(|subnet| {
        subnet.canisters.values().any(|canister| {
            (cfg.auth.delegated_tokens.enabled && canister.auth.delegated_token_signer)
                || canister.auth.role_attestation_cache
        })
    })
}

// Decide whether one non-root runtime must carry threshold-ECDSA management support.
const fn nonroot_requires_auth_crypto(
    canister_cfg: &crate::config::schema::CanisterConfig,
) -> bool {
    canister_cfg.auth.delegated_token_signer
}

#[cfg(test)]
mod tests {
    use super::{RuntimeAuthWorkflow, nonroot_requires_auth_crypto, root_requires_auth_crypto};
    use crate::{
        config::schema::{CanisterAuthConfig, CanisterKind},
        ids::CanisterRole,
        test::config::ConfigTestBuilder,
    };

    #[test]
    fn root_requires_auth_crypto_for_delegated_signer_when_delegated_tokens_enabled() {
        let mut signer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        signer_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: true,
            role_attestation_cache: false,
        };

        let cfg = ConfigTestBuilder::new()
            .with_prime_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_prime_canister("user_shard", signer_cfg)
            .build();

        assert!(root_requires_auth_crypto(&cfg));
    }

    #[test]
    fn root_requires_auth_crypto_for_role_attestation_cache_when_delegated_tokens_disabled() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: false,
            role_attestation_cache: true,
        };

        let mut cfg = ConfigTestBuilder::new()
            .with_prime_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_prime_canister("project_hub", verifier_cfg)
            .build();
        cfg.auth.delegated_tokens.enabled = false;

        assert!(root_requires_auth_crypto(&cfg));
    }

    #[test]
    fn root_ignores_delegated_signer_when_delegated_tokens_disabled() {
        let mut signer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        signer_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: true,
            role_attestation_cache: false,
        };

        let mut cfg = ConfigTestBuilder::new()
            .with_prime_canister(
                CanisterRole::ROOT,
                ConfigTestBuilder::canister_config(CanisterKind::Root),
            )
            .with_prime_canister("user_shard", signer_cfg)
            .build();
        cfg.auth.delegated_tokens.enabled = false;

        assert!(!root_requires_auth_crypto(&cfg));
    }

    #[test]
    fn root_does_not_require_auth_crypto_without_auth_roles() {
        let cfg = ConfigTestBuilder::new().build();

        assert!(!root_requires_auth_crypto(&cfg));
    }

    #[test]
    fn verifier_only_nonroot_does_not_require_auth_crypto() {
        let mut verifier_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
        verifier_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: false,
            role_attestation_cache: true,
        };

        assert!(!nonroot_requires_auth_crypto(&verifier_cfg));
    }

    #[test]
    fn signer_nonroot_requires_auth_crypto() {
        let mut signer_cfg = ConfigTestBuilder::canister_config(CanisterKind::Shard);
        signer_cfg.auth = CanisterAuthConfig {
            delegated_token_signer: true,
            role_attestation_cache: true,
        };

        assert!(nonroot_requires_auth_crypto(&signer_cfg));
    }

    #[test]
    fn runtime_auth_workflow_type_exists_for_runtime_ownership() {
        let _ = RuntimeAuthWorkflow;
    }
}
