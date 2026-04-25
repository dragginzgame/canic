use crate::{
    InternalError, InternalErrorOrigin,
    config::ConfigModel,
    dto::auth::{
        DelegatedTokenClaims, DelegationProvisionResponse, DelegationProvisionStatus,
        DelegationProvisionTargetKind, DelegationRequest,
    },
    ids::{CanisterRole, cap},
    ops::{
        auth::DelegatedTokenOps,
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
        rpc::RpcOps,
        runtime::env::EnvOps,
        storage::{auth::DelegationStateOps, index::subnet::SubnetIndexOps},
    },
    protocol,
    workflow::prelude::*,
};

const DEFAULT_SIGNER_DELEGATION_PREWARM_TTL_SECS: u64 = 900;

///
/// RuntimeAuthWorkflow
///
/// Owns delegated-auth runtime startup checks and auth-specific runtime boot
/// logging for root and non-root canisters.
///

pub struct RuntimeAuthWorkflow;

impl RuntimeAuthWorkflow {
    /// Log the resolved verifier proof-cache policy once during runtime startup.
    pub fn log_delegation_proof_cache_policy() {
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

    /// Prewarm one local signer proof when the current canister is a delegated signer.
    pub async fn prewarm_signer_delegation_proof() -> Result<(), InternalError> {
        let delegated_tokens_cfg = ConfigOps::delegated_tokens_config()?;
        let canister_cfg = ConfigOps::current_canister()?;
        if !delegated_tokens_cfg.enabled || !canister_cfg.delegated_auth.signer {
            return Ok(());
        }

        let ttl_secs = delegated_tokens_cfg
            .max_ttl_secs
            .unwrap_or(DEFAULT_SIGNER_DELEGATION_PREWARM_TTL_SECS);
        let now = IcOps::now_secs();
        let claims = DelegatedTokenClaims {
            sub: IcOps::canister_self(),
            shard_pid: IcOps::canister_self(),
            scopes: vec![cap::VERIFY.to_string()],
            aud: Self::signer_prewarm_audience()?,
            iat: now,
            exp: now.saturating_add(ttl_secs),
        };

        if DelegationStateOps::latest_proof_dto()
            .is_some_and(|proof| DelegatedTokenOps::proof_reusable_for_claims(&proof, &claims, now))
        {
            return Ok(());
        }

        let verifier_targets = Self::signer_prewarm_verifier_targets(&claims.aud)?;
        let shard_public_key_sec1 =
            DelegatedTokenOps::local_shard_public_key_sec1(claims.shard_pid).await?;
        let request = DelegationRequest {
            shard_pid: claims.shard_pid,
            scopes: claims.scopes,
            aud: claims.aud,
            ttl_secs,
            verifier_targets: verifier_targets.clone(),
            include_root_verifier: true,
            shard_public_key_sec1: Some(shard_public_key_sec1),
            metadata: None,
        };

        let root_pid = EnvOps::root_pid()?;
        let response: DelegationProvisionResponse =
            RpcOps::call_rpc_result(root_pid, protocol::CANIC_REQUEST_DELEGATION, request).await?;
        Self::ensure_prewarm_verifier_targets_provisioned(&verifier_targets, &response)?;
        DelegatedTokenOps::cache_public_keys_for_cert(&response.proof.cert).await?;
        DelegatedTokenOps::verify_delegation_proof(&response.proof, root_pid)?;
        DelegationStateOps::upsert_proof_from_dto(response.proof.clone(), IcOps::now_secs())?;

        crate::log!(
            Topic::Auth,
            Info,
            "delegation signer proof prewarmed shard={} aud={} ttl_secs={}",
            response.proof.cert.shard_pid,
            response.proof.cert.aud.len(),
            ttl_secs
        );

        Ok(())
    }

    // Build a broad local proof audience from the current signer and indexed verifier canisters.
    fn signer_prewarm_audience() -> Result<Vec<Principal>, InternalError> {
        let local = IcOps::canister_self();
        let subnet_cfg = ConfigOps::current_subnet()?;
        let mut audience = vec![local];

        for (role, canister_cfg) in subnet_cfg.canisters {
            if !canister_cfg.delegated_auth.verifier {
                continue;
            }

            if let Some(pid) = SubnetIndexOps::get(&role)
                && !audience.contains(&pid)
            {
                audience.push(pid);
            }
        }

        Ok(audience)
    }

    // Select remote verifier targets that need root-pushed proof material during prewarm.
    fn signer_prewarm_verifier_targets(
        audience: &[Principal],
    ) -> Result<Vec<Principal>, InternalError> {
        let local = IcOps::canister_self();
        let root = EnvOps::root_pid()?;
        let verifier_targets = audience
            .iter()
            .copied()
            .filter(|pid| *pid != local && *pid != root)
            .collect();

        Ok(verifier_targets)
    }

    // Fail signer prewarm when root did not successfully install every required verifier target.
    fn ensure_prewarm_verifier_targets_provisioned(
        verifier_targets: &[Principal],
        response: &DelegationProvisionResponse,
    ) -> Result<(), InternalError> {
        for target in verifier_targets {
            let Some(result) = response.results.iter().find(|entry| {
                entry.kind == DelegationProvisionTargetKind::Verifier && entry.target == *target
            }) else {
                return Err(InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("delegation prewarm missing verifier target result for '{target}'"),
                ));
            };

            if result.status != DelegationProvisionStatus::Ok {
                return Err(InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!(
                        "delegation prewarm failed for verifier target '{}': {}",
                        target,
                        result
                            .error
                            .as_ref()
                            .map_or_else(|| "unknown error".to_string(), ToString::to_string)
                    ),
                ));
            }
        }

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
