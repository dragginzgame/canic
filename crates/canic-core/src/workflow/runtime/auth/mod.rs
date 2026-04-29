use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::ConfigModel,
    ids::CanisterRole,
    ops::{
        auth::DelegatedTokenOps,
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
    },
    workflow::prelude::*,
};

///
/// SignerAuthMaterialPrewarmPlan
///

struct SignerAuthMaterialPrewarmPlan {
    shard_pid: Principal,
}

impl SignerAuthMaterialPrewarmPlan {
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

    /// Prewarm local signer key material when the current canister is a delegated signer.
    pub async fn prewarm_signer_delegation_proof() -> Result<(), InternalError> {
        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        let canister_cfg = ConfigOps::current_canister()?;
        if !delegated_tokens_cfg.enabled || !canister_cfg.delegated_auth.signer {
            return Ok(());
        }

        let plan = SignerAuthMaterialPrewarmPlan::default();
        let shard_public_key_sec1 =
            DelegatedTokenOps::local_shard_public_key_sec1(plan.shard_pid).await?;

        crate::log!(
            Topic::Auth,
            Info,
            "delegation signer auth material prewarmed shard={} shard_public_key_bytes={}",
            plan.shard_pid,
            shard_public_key_sec1.len()
        );

        Ok(())
    }
}

// Decide whether the root runtime must carry threshold-ECDSA management support.
fn root_requires_auth_crypto(cfg: &ConfigModel) -> bool {
    cfg.auth.delegated_tokens.enabled
        && cfg.subnets.values().any(|subnet| {
            subnet
                .canisters
                .values()
                .any(|canister| canister.delegated_auth.signer || canister.delegated_auth.verifier)
        })
}

// Decide whether one non-root runtime must carry threshold-ECDSA management support.
const fn nonroot_requires_auth_crypto(
    canister_cfg: &crate::config::schema::CanisterConfig,
) -> bool {
    canister_cfg.delegated_auth.signer
}

#[cfg(test)]
mod tests {
    use super::{RuntimeAuthWorkflow, nonroot_requires_auth_crypto, root_requires_auth_crypto};
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

    #[test]
    fn runtime_auth_workflow_type_exists_for_runtime_ownership() {
        let _ = RuntimeAuthWorkflow;
    }
}
