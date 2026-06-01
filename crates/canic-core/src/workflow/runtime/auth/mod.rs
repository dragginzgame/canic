use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::ConfigModel,
    dto::auth::SignedRoleAttestation,
    format::display_optional,
    ids::CanisterRole,
    log,
    ops::{
        auth::{AuthExpiryError, AuthOps, AuthOpsError, AuthValidationError},
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
        rpc::RpcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            record_attestation_epoch_rejected, record_attestation_refresh_failed,
            record_attestation_unknown_key_id, record_attestation_verify_failed,
        },
    },
    protocol,
    workflow::prelude::*,
};
use std::future::Future;

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

    /// Verify a role attestation, refreshing root keys once on unknown key.
    pub async fn verify_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), InternalError> {
        let configured_min_accepted_epoch = ConfigOps::role_attestation_config()?
            .min_accepted_epoch_by_role
            .get(attestation.payload.role.as_str())
            .copied();
        let min_accepted_epoch =
            resolve_min_accepted_epoch(min_accepted_epoch, configured_min_accepted_epoch);

        let caller = IcOps::msg_caller();
        let self_pid = IcOps::canister_self();
        let now_secs = IcOps::now_secs();
        let verifier_subnet = Some(EnvOps::subnet_pid()?);
        let root_pid = EnvOps::root_pid()?;

        let verify = || {
            AuthOps::verify_role_attestation_cached(
                attestation,
                caller,
                self_pid,
                verifier_subnet,
                now_secs,
                min_accepted_epoch,
            )
            .map(|_| ())
        };
        let refresh = || async {
            let key_set =
                RpcOps::call_rpc_result(root_pid, protocol::CANIC_ATTESTATION_KEY_SET, ()).await?;
            AuthOps::replace_attestation_key_set(key_set);
            Ok(())
        };

        match verify_role_attestation_with_single_refresh(verify, refresh).await {
            Ok(()) => Ok(()),
            Err(RoleAttestationVerifyFlowError::Initial(err)) => {
                record_attestation_verifier_rejection(&err);
                log_attestation_verifier_rejection(&err, attestation, caller, self_pid, "cached");
                Err(err.into())
            }
            Err(RoleAttestationVerifyFlowError::Refresh { trigger, source }) => {
                record_attestation_verifier_rejection(&trigger);
                log_attestation_verifier_rejection(
                    &trigger,
                    attestation,
                    caller,
                    self_pid,
                    "cache_miss_refresh",
                );
                record_attestation_refresh_failed();
                log!(
                    Topic::Auth,
                    Warn,
                    "role attestation refresh failed local={} caller={} key_id={} error={}",
                    self_pid,
                    caller,
                    attestation.key_id,
                    source
                );
                Err(source)
            }
            Err(RoleAttestationVerifyFlowError::PostRefresh(err)) => {
                record_attestation_verifier_rejection(&err);
                log_attestation_verifier_rejection(
                    &err,
                    attestation,
                    caller,
                    self_pid,
                    "post_refresh",
                );
                Err(err.into())
            }
        }
    }
}

#[derive(Debug)]
enum RoleAttestationVerifyFlowError {
    Initial(AuthOpsError),
    Refresh {
        trigger: AuthOpsError,
        source: InternalError,
    },
    PostRefresh(AuthOpsError),
}

async fn verify_role_attestation_with_single_refresh<Verify, Refresh, RefreshFuture>(
    mut verify: Verify,
    mut refresh: Refresh,
) -> Result<(), RoleAttestationVerifyFlowError>
where
    Verify: FnMut() -> Result<(), AuthOpsError>,
    Refresh: FnMut() -> RefreshFuture,
    RefreshFuture: Future<Output = Result<(), InternalError>>,
{
    match verify() {
        Ok(()) => Ok(()),
        Err(
            err @ AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId { .. }),
        ) => {
            refresh()
                .await
                .map_err(|source| RoleAttestationVerifyFlowError::Refresh {
                    trigger: err,
                    source,
                })?;
            verify().map_err(RoleAttestationVerifyFlowError::PostRefresh)
        }
        Err(err) => Err(RoleAttestationVerifyFlowError::Initial(err)),
    }
}

fn resolve_min_accepted_epoch(explicit: u64, configured: Option<u64>) -> u64 {
    if explicit > 0 {
        explicit
    } else {
        configured.unwrap_or(0)
    }
}

fn record_attestation_verifier_rejection(err: &AuthOpsError) {
    record_attestation_verify_failed();
    match err {
        AuthOpsError::Validation(AuthValidationError::AttestationUnknownKeyId { .. }) => {
            record_attestation_unknown_key_id();
        }
        AuthOpsError::Expiry(AuthExpiryError::AttestationEpochRejected { .. }) => {
            record_attestation_epoch_rejected();
        }
        _ => {}
    }
}

fn log_attestation_verifier_rejection(
    err: &AuthOpsError,
    attestation: &SignedRoleAttestation,
    caller: Principal,
    self_pid: Principal,
    phase: &str,
) {
    log!(
        Topic::Auth,
        Warn,
        "role attestation rejected phase={} local={} caller={} subject={} role={} key_id={} audience={} subnet={} issued_at={} expires_at={} epoch={} error={}",
        phase,
        self_pid,
        caller,
        attestation.payload.subject,
        attestation.payload.role,
        attestation.key_id,
        attestation.payload.audience,
        display_optional(attestation.payload.subnet_id),
        attestation.payload.issued_at,
        attestation.payload.expires_at,
        attestation.payload.epoch,
        err
    );
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
